use std::collections::{HashMap, HashSet};

use hydroflow_lang::{
    diagnostic::{Diagnostic, Level},
    graph::flat_graph::{FlatGraph, FlatGraphBuilder},
    parse::{IndexInt, Indexing, Pipeline, PipelineLink},
};
use proc_macro2::{Span, TokenStream};
use syn::parse_quote;

mod grammar;
mod join_plan;
mod util;

use grammar::datalog::*;
use join_plan::*;
use util::{repeat_tuple, Counter};

pub fn gen_hydroflow_graph(
    literal: proc_macro2::Literal,
) -> Result<FlatGraph, (Vec<rust_sitter::errors::ParseError>, Vec<Diagnostic>)> {
    let str_node: syn::LitStr = parse_quote!(#literal);
    let actual_str = str_node.value();
    let program: Program = grammar::datalog::parse(&actual_str).map_err(|e| (e, vec![]))?;

    let mut inputs = Vec::new();
    let mut outputs = Vec::new();
    let mut rules = Vec::new();

    for stmt in &program.rules {
        match stmt {
            Declaration::Input(_, ident) => inputs.push(ident),
            Declaration::Output(_, ident) => outputs.push(ident),
            Declaration::Rule(rule) => rules.push(rule),
        }
    }

    let mut flat_graph_builder = FlatGraphBuilder::new();
    let mut tee_counter = HashMap::new();
    let mut merge_counter = HashMap::new();

    let mut created_rules = HashSet::new();
    for decl in &program.rules {
        let target_ident = match decl {
            Declaration::Input(_, ident) => ident.clone(),
            Declaration::Output(_, ident) => ident.clone(),
            Declaration::Rule(rule) => rule.target.name.clone(),
        };

        if !created_rules.contains(&target_ident) {
            created_rules.insert(target_ident.clone());
            let name = syn::Ident::new(&target_ident.name, Span::call_site());
            flat_graph_builder.add_statement(parse_quote!(#name = merge() -> tee()));
        }
    }

    for target in inputs {
        let target_ident = syn::Ident::new(&target.name, Span::call_site());

        let my_merge_index = merge_counter
            .entry(target.name.clone())
            .or_insert_with(|| 0..)
            .next()
            .expect("Out of merge indices");

        let my_merge_index_lit =
            syn::LitInt::new(&format!("{}", my_merge_index), Span::call_site());
        let name = syn::Ident::new(&target.name, Span::call_site());

        flat_graph_builder.add_statement(parse_quote! {
            source_stream(#target_ident) -> [#my_merge_index_lit] #name
        });
    }

    for target in outputs {
        let my_tee_index = tee_counter
            .entry(target.name.clone())
            .or_insert_with(|| 0..)
            .next()
            .expect("Out of tee indices");

        let out_send_ident = syn::Ident::new(&target.name, Span::call_site());

        let my_tee_index_lit = syn::LitInt::new(&format!("{}", my_tee_index), Span::call_site());
        let target_ident = syn::Ident::new(&target.name, Span::call_site());

        flat_graph_builder.add_statement(parse_quote! {
            #target_ident [#my_tee_index_lit] -> for_each(|v| #out_send_ident.send(v).unwrap())
        });
    }

    let mut next_join_idx = 0..;
    let mut diagnostics = Vec::new();
    for rule in rules {
        generate_rule(
            rule,
            &mut flat_graph_builder,
            &mut tee_counter,
            &mut merge_counter,
            &mut next_join_idx,
            &mut diagnostics,
        );
    }

    if !diagnostics.is_empty() {
        Err((vec![], diagnostics))
    } else {
        let flat_graph = flat_graph_builder
            .build(Level::Error)
            .unwrap_or_else(std::convert::identity);
        Ok(flat_graph)
    }
}

pub fn hydroflow_graph_to_program(flat_graph: FlatGraph, root: TokenStream) -> syn::Stmt {
    let code_tokens = flat_graph
        .into_partitioned_graph()
        .expect("failed to partition")
        .as_code(root, true);

    syn::parse_quote!({
        #code_tokens
    })
}

fn generate_rule(
    rule: &Rule,
    flat_graph_builder: &mut FlatGraphBuilder,
    tee_counter: &mut HashMap<String, Counter>,
    merge_counter: &mut HashMap<String, Counter>,
    next_join_idx: &mut Counter,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let target = &rule.target.name;
    let target_ident = syn::Ident::new(&target.name, Span::call_site());

    let sources: Vec<Atom> = rule.sources.to_vec();

    // TODO(shadaj): smarter plans
    let mut plan: JoinPlan = sources
        .iter()
        .filter_map(|x| match x {
            Atom::Relation(negated, e) => {
                if negated.is_none() {
                    Some(JoinPlan::Source(e))
                } else {
                    None
                }
            }
            _ => None,
        })
        .reduce(|a, b| JoinPlan::Join(Box::new(a), Box::new(b)))
        .unwrap();

    plan = sources
        .iter()
        .filter_map(|x| match x {
            Atom::Relation(negated, e) => {
                if negated.is_some() {
                    Some(JoinPlan::Source(e))
                } else {
                    None
                }
            }
            _ => None,
        })
        .fold(plan, |a, b| JoinPlan::AntiJoin(Box::new(a), Box::new(b)));

    let predicates = sources
        .iter()
        .filter_map(|x| match x {
            Atom::Predicate(e) => Some(e),
            _ => None,
        })
        .collect::<Vec<_>>();

    if !predicates.is_empty() {
        plan = JoinPlan::Predicate(predicates, Box::new(plan))
    }

    let out_expanded = expand_join_plan(&plan, flat_graph_builder, tee_counter, next_join_idx);

    let after_join = apply_aggregations(rule, &out_expanded, diagnostics);

    let my_merge_index = merge_counter
        .entry(target.name.clone())
        .or_insert_with(|| 0..)
        .next()
        .expect("Out of merge indices");

    let my_merge_index_lit = syn::LitInt::new(&format!("{}", my_merge_index), Span::call_site());

    let after_join_and_send: Pipeline = match rule.rule_type {
        RuleType::Sync(_) => {
            if rule.target.at_node.is_some() {
                panic!("Rule must be async to send data to other nodes")
            }

            parse_quote!(#after_join -> [#my_merge_index_lit] #target_ident)
        }
        RuleType::NextTick(_) => {
            if rule.target.at_node.is_some() {
                panic!("Rule must be async to send data to other nodes")
            }

            parse_quote!(#after_join -> next_tick() -> [#my_merge_index_lit] #target_ident)
        }
        RuleType::Async(_) => {
            if rule.target.at_node.is_none() {
                panic!("Async rules are only for sending data to other nodes")
            }

            let exprs_get_data = rule
                .target
                .fields
                .iter()
                .enumerate()
                .map(|(i, _)| -> syn::Expr {
                    let syn_index = syn::Index::from(i);
                    parse_quote!(v.#syn_index)
                });

            let syn_target_index = syn::Index::from(rule.target.fields.len());

            let v_type = repeat_tuple::<syn::Type, syn::Type>(
                || parse_quote!(_),
                rule.target.fields.len() + 1,
            );

            let send_ident = syn::Ident::new(
                &format!("async_send_{}", &rule.target.name.name),
                Span::call_site(),
            );

            let async_receiver_ident = syn::Ident::new(
                &format!("async_receive_{}", &rule.target.name.name),
                Span::call_site(),
            );
            flat_graph_builder.add_statement(parse_quote!(source_stream(#async_receiver_ident) -> [#my_merge_index_lit] #target_ident));

            parse_quote!(#after_join -> for_each(|v: #v_type| #send_ident(v.#syn_target_index, (#(#exprs_get_data, )*)).unwrap()))
        }
    };

    let out_name = out_expanded.name;
    // If the output comes with a tee index, we must read with that. This only happens when we are
    // directly outputting a transformation of a single relation on the RHS.
    let out_indexing = out_expanded.tee_idx.map(|i| Indexing {
        bracket_token: syn::token::Bracket::default(),
        index: hydroflow_lang::parse::PortIndex::Int(IndexInt {
            value: i,
            span: Span::call_site(),
        }),
    });
    flat_graph_builder.add_statement(hydroflow_lang::parse::HfStatement::Pipeline(
        Pipeline::Link(PipelineLink {
            lhs: Box::new(parse_quote!(#out_name #out_indexing)), // out_name[idx]
            arrow: parse_quote!(->),
            rhs: Box::new(after_join_and_send),
        }),
    ));
}

fn apply_aggregations(
    rule: &Rule,
    out_expanded: &IntermediateJoinNode,
    diagnostics: &mut Vec<Diagnostic>,
) -> Pipeline {
    let mut aggregations = vec![];
    let mut group_by_exprs = vec![];
    let mut agg_exprs = vec![];

    for field in rule
        .target
        .fields
        .iter()
        .chain(rule.target.at_node.iter().map(|n| &n.node))
    {
        let expr: syn::Expr = if let Some(col) = out_expanded
            .variable_mapping
            .get(&syn::Ident::new(&field.ident().name, Span::call_site()))
        {
            let source_col_idx = syn::Index::from(*col);
            parse_quote!(row.#source_col_idx)
        } else {
            diagnostics.push(Diagnostic::spanned(
                Span::call_site(),
                Level::Error,
                format!(
                    "Could not find column {} in RHS of rule",
                    &field.ident().name
                ),
            ));
            parse_quote!(())
        };

        match field {
            TargetExpr::Ident(_) => {
                group_by_exprs.push(expr);
            }
            TargetExpr::Aggregation(_) => {
                aggregations.push(field);
                agg_exprs.push(expr);
            }
        }
    }

    let flattened_tuple_type = &out_expanded.tuple_type;

    if agg_exprs.is_empty() {
        parse_quote!(map(|row: #flattened_tuple_type| (#(#group_by_exprs, )*)))
    } else {
        let agg_initial =
            repeat_tuple::<syn::Expr, syn::Expr>(|| parse_quote!(None), agg_exprs.len());

        let group_by_input_type =
            repeat_tuple::<syn::Type, syn::Type>(|| parse_quote!(_), group_by_exprs.len());

        let agg_input_type =
            repeat_tuple::<syn::Type, syn::Type>(|| parse_quote!(_), agg_exprs.len());
        let agg_type: syn::Type =
            repeat_tuple::<syn::Type, syn::Type>(|| parse_quote!(Option<_>), agg_exprs.len());

        let group_by_stmts: Vec<syn::Stmt> = aggregations
            .iter()
            .enumerate()
            .map(|(i, agg)| {
                let idx = syn::Index::from(i);
                let old_at_index: syn::Expr = parse_quote!(old.#idx);
                let val_at_index: syn::Expr = parse_quote!(val.#idx);

                let agg_expr: syn::Expr = match agg {
                    TargetExpr::Aggregation(Aggregation { tpe, .. }) => match tpe {
                        AggregationType::Min(_) => {
                            parse_quote!(std::cmp::min(prev, #val_at_index))
                        },
                        AggregationType::Max(_) => {
                            parse_quote!(std::cmp::max(prev, #val_at_index))
                        },
                        AggregationType::Sum(_) => {
                            parse_quote!(prev + #val_at_index)
                        },
                        AggregationType::Count(_) => {
                            parse_quote!(prev+1)
                        },
                        AggregationType::Choose(_) => {
                            parse_quote!(prev) // choose = select any 1 element from the relation. By default we select the 1st.
                        },
                    },
                    _ => panic!(),
                };

                let agg_initial: syn::Expr = match agg {
                    TargetExpr::Aggregation(Aggregation { tpe, .. }) => match tpe {
                        AggregationType::Min(_) | AggregationType::Max(_) | AggregationType::Sum(_) | AggregationType::Choose(_) => {
                            parse_quote!(#val_at_index)
                        },
                        AggregationType::Count(_) => {
                            parse_quote!(1)
                        },
                    },
                    _ => panic!(),
                };

                parse_quote! {
                    #old_at_index = if let Some(prev) = #old_at_index {
                        Some(#agg_expr)
                    } else {
                        Some(#agg_initial)
                    };
                }
            })
            .collect();

        let group_by_fn: syn::Expr = parse_quote!(|old: &mut #agg_type, val: #agg_input_type| {
            #(#group_by_stmts)*
        });

        let mut after_group_lookups: Vec<syn::Expr> = vec![];
        let mut group_key_idx = 0;
        let mut agg_idx = 0;
        for field in rule
            .target
            .fields
            .iter()
            .chain(rule.target.at_node.iter().map(|n| &n.node))
        {
            match field {
                TargetExpr::Ident(_) => {
                    let idx = syn::Index::from(group_key_idx);
                    after_group_lookups.push(parse_quote!(g.#idx));
                    group_key_idx += 1;
                }
                TargetExpr::Aggregation(_) => {
                    let idx = syn::Index::from(agg_idx);
                    after_group_lookups.push(parse_quote!(a.#idx.unwrap()));
                    agg_idx += 1;
                }
            }
        }

        let pre_group_by_map: syn::Expr = parse_quote!(|row: #flattened_tuple_type| ((#(#group_by_exprs, )*), (#(#agg_exprs, )*)));
        let after_group_map: syn::Expr = parse_quote!(|(g, a)| (#(#after_group_lookups, )*));

        parse_quote! {
            map(#pre_group_by_map) -> group_by::<'tick, #group_by_input_type, #agg_type>(|| #agg_initial, #group_by_fn) -> map(#after_group_map)
        }
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    use super::gen_hydroflow_graph;
    use super::hydroflow_graph_to_program;

    macro_rules! test_snapshots {
        ($program:literal) => {
            let graph = gen_hydroflow_graph(parse_quote!($program)).unwrap();

            insta::with_settings!({snapshot_suffix => "surface_graph"}, {
                insta::assert_display_snapshot!(graph.surface_syntax_string());
            });

            // Have to make a new graph as the above closure borrows.
            let graph2 = gen_hydroflow_graph(parse_quote!($program)).unwrap();
            let out = &hydroflow_graph_to_program(graph2, quote::quote! { hydroflow });
            let wrapped: syn::File = parse_quote! {
                fn main() {
                    #out
                }
            };

            insta::with_settings!({snapshot_suffix => "datalog_program"}, {
                insta::assert_display_snapshot!(
                    prettyplease::unparse(&wrapped)
                );
            });
        };
    }

    #[test]
    fn minimal_program() {
        test_snapshots!(
            r#"
            .input input
            .output out

            out(y, x) :- input(x, y).
            "#
        );
    }

    #[test]
    fn join_with_self() {
        test_snapshots!(
            r#"
            .input input
            .output out

            out(x, y) :- input(x, y), input(y, x).
            "#
        );
    }

    #[test]
    fn join_with_other() {
        test_snapshots!(
            r#"
            .input in1
            .input in2
            .output out

            out(x, y) :- in1(x, y), in2(y, x).
            "#
        );
    }

    #[test]
    fn multiple_contributors() {
        test_snapshots!(
            r#"
            .input in1
            .input in2
            .output out

            out(x, y) :- in1(x, y).
            out(x, y) :- in2(y, x).
            "#
        );
    }

    #[test]
    fn transitive_closure() {
        test_snapshots!(
            r#"
            .input edges
            .input seed_reachable
            .output reachable

            reachable(x) :- seed_reachable(x).
            reachable(y) :- reachable(x), edges(x, y).
            "#
        );
    }

    #[test]
    fn single_column_program() {
        test_snapshots!(
            r#"
            .input in1
            .input in2
            .output out

            out(x) :- in1(x), in2(x).
            "#
        );
    }

    #[test]
    fn triple_relation_join() {
        test_snapshots!(
            r#"
            .input in1
            .input in2
            .input in3
            .output out

            out(d, c, b, a) :- in1(a, b), in2(b, c), in3(c, d).
            "#
        );
    }

    #[test]
    fn local_constraints() {
        test_snapshots!(
            r#"
            .input input
            .output out

            out(x, x) :- input(x, x).
            "#
        );

        test_snapshots!(
            r#"
            .input input
            .output out

            out(x, x, y, y) :- input(x, x, y, y).
            "#
        );
    }

    #[test]
    fn test_simple_filter() {
        test_snapshots!(
            r#"
            .input input
            .output out

            out(x, y) :- input(x, y), ( x > y ), ( y == x ).
            "#
        );
    }

    #[test]
    fn test_anti_join() {
        test_snapshots!(
            r#"
            .input ints_1
            .input ints_2
            .input ints_3
            .output result

            result(x, z) :- ints_1(x, y), ints_2(y, z), !ints_3(y)
            "#
        );
    }

    #[test]
    fn test_max() {
        test_snapshots!(
            r#"
            .input ints
            .output result

            result(max(a), b) :- ints(a, b)
            "#
        );
    }

    #[test]
    fn test_max_all() {
        test_snapshots!(
            r#"
            .input ints
            .output result

            result(max(a), max(b)) :- ints(a, b)
            "#
        );
    }

    #[test]
    fn test_send_to_node() {
        test_snapshots!(
            r#"
            .input ints
            .output result

            result@b(a) :~ ints(a, b)
            "#
        );
    }

    #[test]
    fn test_choose() {
        test_snapshots!(
            r#"
            .input clientIn
            .output clientOut
            .input clientAddr
            
            clientOut@addr(choose(i)) :~ clientIn(i), clientAddr(addr)
            "#
        );
    }

    #[test]
    fn test_voting_replica() {
        test_snapshots!(
            r#"
            .input voteToReplica
            .input leaderAddr
            .input local
            .output voteFromReplica
            
            # reply to leader
            voteFromReplica@addr(s, v) :~ voteToReplica(v), leaderAddr(addr), local(s)
            "#
        );
    }

    #[test]
    fn test_voting_leader() {
        test_snapshots!(
            r#"
            .input clientIn
            .output clientOut
            
            .output voteToReplica
            .input voteFromReplica
            .input voteCounts

            .input replicaAddr
            .input numReplicas
            .input clientAddr
            
            voteToReplica@addr(v) :~ clientIn(v), replicaAddr(addr)
            voteFromReplica(s, v) :+ voteFromReplica(s, v)
            voteCounts(count(l), v) :- voteFromReplica(l, v)
            clientOut@addr(v) :~ voteCounts(n, v), numReplicas(n), clientAddr(addr)
            "#
        );
    }
}
