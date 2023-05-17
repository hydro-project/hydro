use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, HashMap};
use std::ops::Deref;

use hydroflow_lang::diagnostic::{Diagnostic, Level};
use hydroflow_lang::graph::FlatGraphBuilder;
use hydroflow_lang::parse::Pipeline;
use proc_macro2::Span;
use rust_sitter::Spanned;
use syn::{self, parse_quote, parse_quote_spanned};

use crate::grammar::datalog::{BoolExpr, BoolOp, IdentOrUnderscore, InputRelationExpr, IntExpr};
use crate::util::{repeat_tuple, Counter};

/// Captures the tree of joins used to compute contributions from a single rule.
pub enum JoinPlan<'a> {
    /// A single relation without any joins, leaves of the tree.
    /// Second element is whether this is a persisted relation.
    Source(&'a Spanned<InputRelationExpr>, bool),
    /// A join between two subtrees.
    Join(Box<JoinPlan<'a>>, Box<JoinPlan<'a>>),
    AntiJoin(Box<JoinPlan<'a>>, Box<JoinPlan<'a>>),
    Predicate(Vec<&'a Spanned<BoolExpr>>, Box<JoinPlan<'a>>),
    /// A join between some relation and a magic relation that emits values between
    /// 0 and some value in the input relation (upper-exclusive).
    MagicNatLt(Box<JoinPlan<'a>>, IdentOrUnderscore, IdentOrUnderscore),
}

/// Tracks the Hydroflow node that corresponds to a subtree of a join plan.
pub struct IntermediateJoinNode {
    /// The name of the Hydroflow node that this join outputs to.
    pub name: syn::Ident,
    /// If true, the correct dataflow for this node ends in a `persist()` operator.
    pub persisted: bool,
    /// If this join node outputs data through a `tee()` operator, this is the index to consume the node with.
    /// (this is only used for cases where we are directly reading a relation)
    pub tee_idx: Option<isize>,
    /// A mapping from variables in the rule to the index of the corresponding element in the flattened tuples this node emits.
    pub variable_mapping: BTreeMap<String, usize>,
    /// Tuple indices that that correspond to wildcard, unused values.
    pub wildcard_indices: Vec<usize>,
    /// The type of the flattened tuples this node emits.
    pub tuple_type: syn::Type,
    /// The span corresponding to the original sources resulting in this node.
    pub span: Span,
}

enum JoinSide {
    Left,
    Right,
}

impl JoinSide {
    fn index(&self) -> usize {
        match self {
            JoinSide::Left => 0,
            JoinSide::Right => 1,
        }
    }
}

/// Generates a Hydroflow pipeline that transforms some input to a join
/// to emit key-value tuples that can be fed into a join operator.
fn emit_join_input_pipeline(
    // The identifiers of the input node that the key should be populated with.
    identifiers_to_join: &[String],
    identifiers_to_not_join: &[String],
    // The Hydroflow node that is one side of the join.
    source_expanded: &IntermediateJoinNode,
    // The Hydroflow node for the join operator.
    join_node: &syn::Ident,
    // Whether this node contributes to the left or right side of the join.
    join_side: JoinSide,
    // Whether the pipeline is for an anti-join.
    anti_join: bool,
    // The Hydroflow graph to emit the pipeline to.
    flat_graph_builder: &mut FlatGraphBuilder,
) {
    let hash_keys: Vec<syn::Expr> = identifiers_to_join
        .iter()
        .map(|ident| {
            if let Some(idx) = source_expanded.variable_mapping.get(ident) {
                let idx_ident = syn::Index::from(*idx);
                parse_quote!(_v.#idx_ident)
            } else {
                panic!("Could not find key that is being joined on: {:?}", ident);
            }
        })
        .collect();

    let not_hash_keys: Vec<syn::Expr> = identifiers_to_not_join
        .iter()
        .map(|ident| {
            if let Some(idx) = source_expanded.variable_mapping.get(ident) {
                let idx_ident = syn::Index::from(*idx);
                parse_quote!(_v.#idx_ident)
            } else {
                panic!("Could not find key that is being joined on: {:?}", ident);
            }
        })
        .chain(source_expanded.wildcard_indices.iter().map(|idx| {
            let idx_ident = syn::Index::from(*idx);
            parse_quote!(_v.#idx_ident)
        }))
        .collect();

    let out_index = syn::Index::from(join_side.index());

    let source_name = &source_expanded.name;
    let source_type = &source_expanded.tuple_type;

    let rhs: Pipeline = if anti_join {
        match join_side {
            JoinSide::Left => {
                parse_quote_spanned!(source_expanded.span=> map(|_v: #source_type| ((#(#hash_keys, )*), (#(#not_hash_keys, )*))) -> [pos] #join_node)
            }
            JoinSide::Right => {
                parse_quote_spanned!(source_expanded.span=> map(|_v: #source_type| (#(#hash_keys, )*)) -> [neg] #join_node)
            }
        }
    } else {
        parse_quote_spanned!(source_expanded.span=> map(|_v: #source_type| ((#(#hash_keys, )*), (#(#not_hash_keys, )*))) -> [#out_index] #join_node)
    };

    let rhs = if anti_join && source_expanded.persisted {
        parse_quote_spanned!(source_expanded.span=> persist() -> #rhs)
    } else {
        rhs
    };

    let statement = match source_expanded.tee_idx {
        Some(i) => {
            let in_index = syn::LitInt::new(&format!("{}", i), Span::call_site());
            parse_quote_spanned!(source_expanded.span=> #source_name [#in_index] -> #rhs)
        }
        None => {
            parse_quote_spanned!(source_expanded.span=> #source_name -> #rhs)
        }
    };

    flat_graph_builder.add_statement(statement);
}

/// Creates a mapping from variable names to the indices where that variable appears in `fields`.
///
/// Only return entries for variables that appear more than once. Those correspond to additional
/// constraints: the relation is only true when the values at those indices are equal.
///
/// For example, `rel(a, b, a) := ...` requires that the values in the 0th and 2nd slots be the
/// same, so we would return a map `{ "a" => [0, 2] }`. Note that since `b` is not repeated, it is
/// not in the map.
fn find_relation_local_constraints<'a>(
    fields: impl Iterator<Item = &'a Spanned<IdentOrUnderscore>>,
) -> BTreeMap<String, Vec<usize>> {
    let mut indices_grouped_by_var = BTreeMap::new();
    for (i, ident) in fields.enumerate() {
        if let IdentOrUnderscore::Ident(ident) = ident.deref() {
            let entry = indices_grouped_by_var
                // TODO(shadaj): Can we avoid cloning here?
                .entry(ident.name.clone())
                .or_insert_with(Vec::new);
            entry.push(i);
        }
    }

    indices_grouped_by_var.retain(|_, v| v.len() > 1);

    indices_grouped_by_var
}

/// Given a mapping from variable names to their repeated indices, builds a Rust expression that
/// tests whether the values at those indices are equal for each variable.
///
/// For example, `rel(a, b, a, a, b)` would give us the map `{ "a" => [0, 2, 3], "b" => [1, 4] }`.
/// Then we would want to generate the code `row.0 == row.2 && row.0 == row.3 && row.1 == row.4`.
fn build_local_constraint_conditions(constraints: &BTreeMap<String, Vec<usize>>) -> syn::Expr {
    constraints
        .values()
        .flat_map(|indices| {
            let equal_indices = indices
                .iter()
                .map(|i| syn::Index::from(*i))
                .collect::<Vec<_>>();

            let first_index = &equal_indices[0];

            equal_indices
                .iter()
                .skip(1)
                .map(|i| parse_quote!(row.#first_index == row.#i))
                .collect::<Vec<_>>()
        })
        .reduce(|a: syn::Expr, b| parse_quote!(#a && #b))
        .unwrap()
}

fn gen_predicate_value_expr(
    expr: &IntExpr,
    variable_mapping: &BTreeMap<String, usize>,
    diagnostics: &mut Vec<Diagnostic>,
    get_span: &dyn Fn((usize, usize)) -> Span,
) -> syn::Expr {
    crate::gen_value_expr(
        expr,
        &mut |ident| {
            if let Some(col) = variable_mapping.get(&ident.name) {
                let idx = syn::Index::from(*col);
                parse_quote_spanned!(get_span(ident.span)=> row.#idx)
            } else {
                diagnostics.push(Diagnostic::spanned(
                    get_span(ident.span),
                    Level::Error,
                    format!("Could not resolve reference to variable {}", &ident.name),
                ));
                parse_quote!(())
            }
        },
        get_span,
    )
}

/// Generates a Hydroflow pipeline that computes the output to a given [`JoinPlan`].
pub fn expand_join_plan(
    // The plan we are converting to a Hydroflow pipeline.
    plan: &JoinPlan,
    // The Hydroflow graph to emit the pipeline to.
    flat_graph_builder: &mut FlatGraphBuilder,
    tee_counter: &mut HashMap<String, Counter>,
    next_join_idx: &mut Counter,
    rule_span: (usize, usize),
    diagnostics: &mut Vec<Diagnostic>,
    get_span: &impl Fn((usize, usize)) -> Span,
) -> IntermediateJoinNode {
    match plan {
        JoinPlan::Source(target, persisted) => {
            let mut variable_mapping = BTreeMap::new();
            let mut wildcard_indices = vec![];
            let mut row_types: Vec<syn::Type> = vec![];

            let local_constraints = find_relation_local_constraints(target.fields.iter());

            for (i, ident) in target.fields.iter().enumerate() {
                row_types.push(parse_quote!(_));

                if let IdentOrUnderscore::Ident(ident) = ident.deref() {
                    if let Entry::Vacant(e) = variable_mapping.entry(ident.name.clone()) {
                        e.insert(i);
                    }
                } else {
                    wildcard_indices.push(i);
                }
            }

            // Because this is a node corresponding to some Datalog relation, we need to tee from it.
            let tee_index = tee_counter
                .entry(target.name.name.clone())
                .or_insert_with(|| 0..)
                .next()
                .expect("Out of tee indices");

            let row_type = parse_quote!((#(#row_types, )*));

            if local_constraints.is_empty() {
                return IntermediateJoinNode {
                    name: syn::Ident::new(&target.name.name, get_span(target.name.span)),
                    persisted: *persisted,
                    tee_idx: Some(tee_index),
                    variable_mapping,
                    wildcard_indices,
                    tuple_type: row_type,
                    span: get_span(target.span),
                };
            }

            let relation_node = syn::Ident::new(&target.name.name, get_span(target.name.span));
            let relation_idx = syn::LitInt::new(&tee_index.to_string(), Span::call_site());

            let filter_node = syn::Ident::new(
                &format!(
                    "join_{}_filter",
                    next_join_idx.next().expect("Out of join indices")
                ),
                Span::call_site(),
            );

            let conditions = build_local_constraint_conditions(&local_constraints);

            flat_graph_builder.add_statement(parse_quote_spanned! {get_span(rule_span)=>
                #filter_node = #relation_node [#relation_idx] -> filter(|row: &#row_type| #conditions)
            });

            IntermediateJoinNode {
                name: filter_node,
                persisted: *persisted,
                tee_idx: None,
                variable_mapping,
                wildcard_indices,
                tuple_type: row_type,
                span: get_span(target.span),
            }
        }
        JoinPlan::Join(lhs, rhs) | JoinPlan::AntiJoin(lhs, rhs) => {
            let is_anti = matches!(plan, JoinPlan::AntiJoin(_, _));

            let left_expanded = expand_join_plan(
                lhs,
                flat_graph_builder,
                tee_counter,
                next_join_idx,
                rule_span,
                diagnostics,
                get_span,
            );
            let right_expanded = expand_join_plan(
                rhs,
                flat_graph_builder,
                tee_counter,
                next_join_idx,
                rule_span,
                diagnostics,
                get_span,
            );

            let identifiers_to_join = right_expanded
                .variable_mapping
                .keys()
                .filter(|i| left_expanded.variable_mapping.contains_key(*i))
                .enumerate()
                .map(|t| (t.1, t.0))
                .collect::<BTreeMap<_, _>>();

            let left_non_joined_identifiers = left_expanded
                .variable_mapping
                .keys()
                .filter(|i| !right_expanded.variable_mapping.contains_key(*i))
                .enumerate()
                .map(|t| (t.1, t.0))
                .collect::<BTreeMap<_, _>>();

            let right_non_joined_identifiers = right_expanded
                .variable_mapping
                .keys()
                .filter(|i| !left_expanded.variable_mapping.contains_key(*i))
                .enumerate()
                .map(|t| (t.1, t.0))
                .collect::<BTreeMap<_, _>>();

            let key_type =
                repeat_tuple::<syn::Type, syn::Type>(|| parse_quote!(_), identifiers_to_join.len());

            let left_type = repeat_tuple::<syn::Type, syn::Type>(
                || parse_quote!(_),
                left_non_joined_identifiers.len() + left_expanded.wildcard_indices.len(),
            );
            let right_type = repeat_tuple::<syn::Type, syn::Type>(
                || parse_quote!(_),
                right_non_joined_identifiers.len() + right_expanded.wildcard_indices.len(),
            );

            let join_node = syn::Ident::new(
                &format!(
                    "join_{}",
                    next_join_idx.next().expect("Out of join indices")
                ),
                Span::call_site(),
            );

            // We start by defining the pipeline from the `join()` operator onwards. The main logic
            // here is to flatten the tuples from the left and right sides of the join into a
            // single tuple that is used by downstream joins or the final output.
            let mut flattened_tuple_elems: Vec<syn::Expr> = vec![];
            let mut flattened_mapping = BTreeMap::new();
            let mut flattened_wildcard_indices = vec![];

            for (ident, idx) in &identifiers_to_join {
                if !flattened_mapping.contains_key(*ident) {
                    let idx = syn::Index::from(*idx);
                    let value_expr: syn::Expr = parse_quote!(kv.0.#idx);

                    flattened_mapping.insert((*ident).clone(), flattened_tuple_elems.len());
                    flattened_tuple_elems.push(value_expr);
                }
            }

            if is_anti {
                for (ident, idx) in &left_non_joined_identifiers {
                    if !flattened_mapping.contains_key(*ident) {
                        let idx = syn::Index::from(*idx);
                        let value_expr: syn::Expr = parse_quote!(kv.1.#idx);

                        flattened_mapping.insert((*ident).clone(), flattened_tuple_elems.len());
                        flattened_tuple_elems.push(value_expr);
                    }
                }

                for idx in 0..left_expanded.wildcard_indices.len() {
                    let idx = syn::Index::from(left_non_joined_identifiers.len() + idx);
                    let value_expr: syn::Expr = parse_quote!(kv.1.#idx);

                    flattened_wildcard_indices.push(flattened_tuple_elems.len());
                    flattened_tuple_elems.push(value_expr);
                }
            } else {
                for (ident, source_idx) in left_non_joined_identifiers
                    .keys()
                    .map(|l| (l, 0))
                    .chain(right_non_joined_identifiers.keys().map(|l| (l, 1)))
                {
                    if !flattened_mapping.contains_key(*ident) {
                        let syn_source_index = syn::Index::from(source_idx);
                        let source_expr: syn::Expr = parse_quote!(kv.1.#syn_source_index);
                        let bindings = if source_idx == 0 {
                            &left_non_joined_identifiers
                        } else {
                            &right_non_joined_identifiers
                        };

                        let source_col_idx = syn::Index::from(*bindings.get(ident).unwrap());

                        flattened_mapping.insert((*ident).clone(), flattened_tuple_elems.len());
                        flattened_tuple_elems.push(parse_quote!(#source_expr.#source_col_idx));
                    }
                }

                for (idx, source_idx) in (0..left_expanded.wildcard_indices.len())
                    .map(|i| (left_non_joined_identifiers.len() + i, 0))
                    .chain(
                        (0..right_expanded.wildcard_indices.len())
                            .map(|i| (right_non_joined_identifiers.len() + i, 1)),
                    )
                {
                    let syn_source_index = syn::Index::from(source_idx);
                    let idx = syn::Index::from(idx);
                    let value_expr: syn::Expr = parse_quote!(kv.1.#syn_source_index.#idx);

                    flattened_wildcard_indices.push(flattened_tuple_elems.len());
                    flattened_tuple_elems.push(value_expr);
                }
            }

            let flatten_closure: syn::Expr = if is_anti {
                parse_quote!(|kv: (#key_type, #left_type)| (#(#flattened_tuple_elems, )*))
            } else {
                parse_quote!(|kv: (#key_type, (#left_type, #right_type))| (#(#flattened_tuple_elems, )*))
            };

            let (lt_left, lt_right, is_persist): (syn::Lifetime, syn::Lifetime, bool) =
                match (left_expanded.persisted, right_expanded.persisted, is_anti) {
                    (true, false, false) => (parse_quote!('static), parse_quote!('tick), false),
                    (false, true, false) => (parse_quote!('tick), parse_quote!('static), false),
                    (true, true, false) => (parse_quote!('static), parse_quote!('static), true),
                    _ => (parse_quote!('tick), parse_quote!('tick), false),
                };

            if is_anti {
                // this is always a 'tick join, so we place a persist operator in the join input pipeline
                flat_graph_builder
                    .add_statement(parse_quote_spanned!(get_span(rule_span)=> #join_node = anti_join() -> map(#flatten_closure)));
            } else {
                flat_graph_builder.add_statement(
                    parse_quote_spanned!(get_span(rule_span)=> #join_node = join::<#lt_left, #lt_right, hydroflow::compiled::pull::HalfMultisetJoinState>() -> map(#flatten_closure)),
                );
            }

            let output_type = repeat_tuple::<syn::Type, syn::Type>(
                || parse_quote!(_),
                flattened_tuple_elems.len(),
            );

            let intermediate = IntermediateJoinNode {
                name: join_node.clone(),
                persisted: is_persist,
                tee_idx: None,
                variable_mapping: flattened_mapping,
                wildcard_indices: flattened_wildcard_indices,
                tuple_type: output_type,
                span: left_expanded
                    .span
                    .join(right_expanded.span)
                    .unwrap_or(get_span(rule_span)),
            };

            emit_join_input_pipeline(
                &identifiers_to_join
                    .keys()
                    .cloned()
                    .cloned()
                    .collect::<Vec<_>>(),
                &left_non_joined_identifiers
                    .keys()
                    .cloned()
                    .cloned()
                    .collect::<Vec<_>>(),
                &left_expanded,
                &join_node,
                JoinSide::Left,
                is_anti,
                flat_graph_builder,
            );

            emit_join_input_pipeline(
                &identifiers_to_join
                    .keys()
                    .cloned()
                    .cloned()
                    .collect::<Vec<_>>(),
                &right_non_joined_identifiers
                    .keys()
                    .cloned()
                    .cloned()
                    .collect::<Vec<_>>(),
                &right_expanded,
                &join_node,
                JoinSide::Right,
                is_anti,
                flat_graph_builder,
            );

            intermediate
        }
        JoinPlan::Predicate(predicates, inner) => {
            let inner_expanded = expand_join_plan(
                inner,
                flat_graph_builder,
                tee_counter,
                next_join_idx,
                rule_span,
                diagnostics,
                get_span,
            );
            let inner_name = inner_expanded.name.clone();
            let row_type = inner_expanded.tuple_type;
            let variable_mapping = &inner_expanded.variable_mapping;

            let conditions = predicates
                .iter()
                .map(|p| {
                    let l =
                        gen_predicate_value_expr(&p.left, variable_mapping, diagnostics, get_span);
                    let r =
                        gen_predicate_value_expr(&p.right, variable_mapping, diagnostics, get_span);

                    match &p.op {
                        BoolOp::Lt(_) => parse_quote_spanned!(get_span(p.span)=> #l < #r),
                        BoolOp::LtEq(_) => parse_quote_spanned!(get_span(p.span)=> #l <= #r),
                        BoolOp::Gt(_) => parse_quote_spanned!(get_span(p.span)=> #l > #r),
                        BoolOp::GtEq(_) => parse_quote_spanned!(get_span(p.span)=> #l >= #r),
                        BoolOp::Eq(_) => parse_quote_spanned!(get_span(p.span)=> #l == #r),
                        BoolOp::Neq(_) => parse_quote_spanned!(get_span(p.span)=> #l != #r),
                    }
                })
                .reduce(|a: syn::Expr, b| parse_quote!(#a && #b))
                .unwrap();

            let predicate_filter_node = syn::Ident::new(
                &format!(
                    "predicate_{}_filter",
                    next_join_idx.next().expect("Out of join indices")
                ),
                Span::call_site(),
            );

            flat_graph_builder.add_statement(parse_quote_spanned! { get_span(rule_span)=>
                #predicate_filter_node = #inner_name -> filter(|row: &#row_type| #conditions )
            });

            IntermediateJoinNode {
                name: predicate_filter_node,
                persisted: inner_expanded.persisted,
                tee_idx: None,
                variable_mapping: inner_expanded.variable_mapping,
                wildcard_indices: inner_expanded.wildcard_indices,
                tuple_type: row_type,
                span: get_span(rule_span),
            }
        }
        JoinPlan::MagicNatLt(inner, less_than, threshold) => {
            let magic_node = syn::Ident::new(
                &format!(
                    "magic_nat_lt_{}",
                    next_join_idx.next().expect("Out of join indices")
                ),
                Span::call_site(),
            );

            let inner_expanded = expand_join_plan(
                inner,
                flat_graph_builder,
                tee_counter,
                next_join_idx,
                rule_span,
                diagnostics,
                get_span,
            );
            let inner_name = inner_expanded.name.clone();
            let row_type = inner_expanded.tuple_type;

            if let IdentOrUnderscore::Ident(less_than) = less_than.deref() {
                if inner_expanded
                    .variable_mapping
                    .contains_key(&less_than.name)
                {
                    todo!("The values generated by less_than cannot currently be used in other parts of the query");
                }
            }

            let threshold_name = if let IdentOrUnderscore::Ident(threshold) = threshold.deref() {
                threshold.name.clone()
            } else {
                panic!("The threshold must be a variable")
            };

            let threshold_index = inner_expanded
                .variable_mapping
                .get(&threshold_name)
                .expect("Threshold variable not found in inner plan");
            let threshold_index = syn::Index::from(*threshold_index);

            let mut flattened_elements: Vec<syn::Expr> = vec![];
            let mut flattened_mapping = BTreeMap::new();
            let mut flattened_wildcard_indices = Vec::new();

            for (ident, source_idx) in &inner_expanded.variable_mapping {
                let syn_source_index = syn::Index::from(*source_idx);
                flattened_mapping.insert(ident.clone(), flattened_elements.len());
                flattened_elements.push(parse_quote!(row.#syn_source_index.clone()));
            }

            for wildcard_idx in &inner_expanded.wildcard_indices {
                let syn_wildcard_idx = syn::Index::from(*wildcard_idx);
                flattened_wildcard_indices.push(flattened_elements.len());
                flattened_elements.push(parse_quote!(row.#syn_wildcard_idx.clone()));
            }

            if let IdentOrUnderscore::Ident(less_than) = less_than.deref() {
                if less_than.name == threshold_name {
                    panic!("The threshold and less_than variables must be different")
                }

                flattened_mapping.insert(less_than.name.clone(), flattened_elements.len());
            } else {
                flattened_wildcard_indices.push(flattened_elements.len());
            }

            flattened_elements.push(parse_quote!(v));

            flat_graph_builder.add_statement(parse_quote_spanned! {get_span(rule_span)=>
                #magic_node = #inner_name -> flat_map(|row: #row_type| (0..(row.#threshold_index)).map(move |v| (#(#flattened_elements, )*)) )
            });

            IntermediateJoinNode {
                name: magic_node,
                persisted: inner_expanded.persisted,
                tee_idx: None,
                variable_mapping: flattened_mapping,
                wildcard_indices: flattened_wildcard_indices,
                tuple_type: repeat_tuple::<syn::Type, syn::Type>(
                    || parse_quote!(_),
                    flattened_elements.len(),
                ),
                span: get_span(rule_span),
            }
        }
    }
}
