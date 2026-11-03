//! DFIR's operators

use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::ops::{Bound, RangeBounds};
use std::sync::OnceLock;

use documented::DocumentedVariants;
use proc_macro2::{Ident, Literal, Span, TokenStream, TokenTree};
use quote::quote_spanned;
use serde::{Deserialize, Serialize};
use slotmap::Key;
use syn::punctuated::Punctuated;
use syn::{Block, Expr, Pat, Path, Stmt, Token, parse_quote_spanned};

use super::{
    AsCodeOptions, GraphLoopId, GraphNode, GraphNodeId, GraphSubgraphId, OpInstGenerics,
    OperatorInstance, PortIndexValue,
};
use crate::diagnostic::{Diagnostic, Diagnostics, Level};
use crate::parse::{Operator, PortIndex};

/// The delay (soft barrier) type, for each input to an operator if needed.
#[derive(Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum DelayType {
    /// Input must be collected over the previous tick.
    Tick,
    /// Input must be collected over the previous tick but also not cause a new tick to occur.
    TickLazy,
    /// Input must be collected over the previous loop iteration. Causes the loop to re-fire.
    Loop,
    /// Input must be collected over the previous loop iteration. Does NOT cause the loop to re-fire.
    LoopLazy,
}

/// Specification of the named (or unnamed) ports for an operator's inputs or outputs.
pub enum PortListSpec {
    /// Any number of unnamed (or optionally named) ports.
    Variadic,
    /// A specific number of named ports.
    Fixed(Punctuated<PortIndex, Token![,]>),
}

/// An instance of this struct represents a single dfir operator.
pub struct OperatorConstraints {
    /// Operator's name.
    pub name: &'static str,
    /// Operator categories, for docs.
    pub categories: &'static [OperatorCategory],

    // TODO: generic argument ranges.
    /// Input argument range required to not show an error.
    pub hard_range_inn: &'static dyn RangeTrait<usize>,
    /// Input argument range required to not show a warning.
    pub soft_range_inn: &'static dyn RangeTrait<usize>,
    /// Output argument range required to not show an error.
    pub hard_range_out: &'static dyn RangeTrait<usize>,
    /// Output argument range required to not show an warning.
    pub soft_range_out: &'static dyn RangeTrait<usize>,
    /// Number of arguments i.e. `operator(a, b, c)` has `num_args = 3`.
    pub num_args: usize,
    /// How many persistence lifetime arguments can be provided.
    pub persistence_args: &'static dyn RangeTrait<usize>,
    // /// How many (non-persistence) lifetime arguments can be provided.
    // pub lifetime_args: &'static dyn RangeTrait<usize>,
    /// How many generic type arguments can be provided.
    pub type_args: &'static dyn RangeTrait<usize>,
    /// If this operator receives external inputs and therefore must be in
    /// stratum 0.
    pub is_external_input: bool,
    /// Flo semantics type.
    pub flo_type: Option<FloType>,

    /// What named or numbered input ports to expect?
    pub ports_inn: Option<fn() -> PortListSpec>,
    /// What named or numbered output ports to expect?
    pub ports_out: Option<fn() -> PortListSpec>,

    /// Determines if this input must be preceeded by a stratum barrier.
    pub input_delaytype_fn: fn(&PortIndexValue) -> Option<DelayType>,
    /// The operator's codegen. Returns code that is emited is several different locations. See [`OperatorWriteOutput`].
    pub write_fn: WriteFn,
}

/// Type alias for [`OperatorConstraints::write_fn`]'s type.
pub type WriteFn = fn(&WriteContextArgs<'_>, &mut Diagnostics) -> Result<OperatorWriteOutput, ()>;

impl Debug for OperatorConstraints {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OperatorConstraints")
            .field("name", &self.name)
            .field("hard_range_inn", &self.hard_range_inn)
            .field("soft_range_inn", &self.soft_range_inn)
            .field("hard_range_out", &self.hard_range_out)
            .field("soft_range_out", &self.soft_range_out)
            .field("num_args", &self.num_args)
            .field("persistence_args", &self.persistence_args)
            .field("type_args", &self.type_args)
            .field("is_external_input", &self.is_external_input)
            .field("ports_inn", &self.ports_inn)
            .field("ports_out", &self.ports_out)
            // .field("input_delaytype_fn", &self.input_delaytype_fn)
            // .field("flow_prop_fn", &self.flow_prop_fn)
            // .field("write_fn", &self.write_fn)
            .finish()
    }
}

/// The code generated and returned by a [`OperatorConstraints::write_fn`].
/// **Important**: When destructuring this struct in delegating operators, list all fields
/// explicitly rather than using `..` to ensure new fields are not silently dropped.
#[derive(Default)]
pub struct OperatorWriteOutput {
    /// Code which runs once outside any subgraphs to set up external state
    /// (channels, network connections, etc.) to be used by the subgraph.
    pub write_prologue: TokenStream,
    /// Iterator (or pusherator) code inside the subgraphs. The code for each
    /// operator is emitted in order.
    ///
    /// Emitted code should assign to [`WriteContextArgs::ident`] and use
    /// [`WriteContextArgs::inputs`] (pull `Stream`s) or
    /// [`WriteContextArgs::outputs`] (push `Sink`s).
    pub write_iterator: TokenStream,
    /// Code which runs after `Stream`s/`Sink`s have been run. Mainly for flushing IO.
    pub write_iterator_after: TokenStream,
    /// Code which runs at the end of each tick, after all subgraphs have run.
    /// Used for resetting state with `'tick` persistence.
    pub write_tick_end: TokenStream,
}

/// Convenience range: zero or more (any number).
pub const RANGE_ANY: &'static dyn RangeTrait<usize> = &(0..);
/// Convenience range: exactly zero.
pub const RANGE_0: &'static dyn RangeTrait<usize> = &(0..=0);
/// Convenience range: exactly one.
pub const RANGE_1: &'static dyn RangeTrait<usize> = &(1..=1);

/// Helper to write the `write_iterator` portion of [`OperatorConstraints::write_fn`] output for
/// unary identity operators.
///
/// When no explicit type argument is given and
/// [`AsCodeOptions::exclude_type_guards`](super::AsCodeOptions::exclude_type_guards) is set, the
/// identity `check_input`/`check_output` type guard is omitted and the input/output is simply
/// rebound.
pub fn identity_write_iterator_fn(
    &WriteContextArgs {
        root,
        op_span,
        ident,
        inputs,
        outputs,
        is_pull,
        options,
        op_inst:
            OperatorInstance {
                generics: OpInstGenerics { type_args, .. },
                ..
            },
        ..
    }: &WriteContextArgs<'_>,
) -> TokenStream {
    if type_args.is_empty() && options.exclude_type_guards {
        // No user-provided type to pin down, so the guard would be a pure (verbose) no-op.
        let src = if is_pull { &inputs[0] } else { &outputs[0] };
        return quote_spanned! {op_span=>
            let #ident = #src;
        };
    }

    let generic_type = type_args
        .first()
        .map(quote::ToTokens::to_token_stream)
        .unwrap_or_else(|| quote_spanned!(op_span=> _));

    if is_pull {
        let input = &inputs[0];
        quote_spanned! {op_span=>
            let #ident = {
                fn check_input<Pull, Item>(pull: Pull) -> impl #root::dfir_pipes::pull::Pull<Item = Item, Meta = Pull::Meta, CanPend = Pull::CanPend, CanEnd = Pull::CanEnd>
                where
                    Pull: #root::dfir_pipes::pull::Pull<Item = Item>,
                {
                    pull
                }
                check_input::<_, #generic_type>(#input)
            };
        }
    } else {
        let output = &outputs[0];
        quote_spanned! {op_span=>
            let #ident = {
                fn check_output<Psh, Item>(push: Psh) -> impl #root::dfir_pipes::push::Push<Item, (), CanPend = Psh::CanPend>
                where
                    Psh: #root::dfir_pipes::push::Push<Item, ()>,
                {
                    push
                }
                check_output::<_, #generic_type>(#output)
            };
        }
    }
}

/// [`OperatorConstraints::write_fn`] for unary identity operators.
pub const IDENTITY_WRITE_FN: WriteFn = |write_context_args, _| {
    let write_iterator = identity_write_iterator_fn(write_context_args);
    Ok(OperatorWriteOutput {
        write_iterator,
        ..Default::default()
    })
};

/// Helper to write the `write_iterator` portion of [`OperatorConstraints::write_fn`] output for
/// the null operator - an operator that ignores all inputs and produces no output.
pub fn null_write_iterator_fn(
    &WriteContextArgs {
        root,
        op_span,
        ident,
        inputs,
        outputs,
        is_pull,
        op_inst:
            OperatorInstance {
                generics: OpInstGenerics { type_args, .. },
                ..
            },
        ..
    }: &WriteContextArgs<'_>,
) -> TokenStream {
    let default_type = parse_quote_spanned! {op_span=> _};
    let iter_type = type_args.first().unwrap_or(&default_type);

    if is_pull {
        quote_spanned! {op_span=>
            let #ident = #root::dfir_pipes::pull::poll_fn({
                #(
                    let mut #inputs = ::std::boxed::Box::pin(#inputs);
                )*
                move |_cx| {
                    // Make sure to poll all `#inputs` to completion.
                    // NOTE(mingwei): `null()` can only have 0 or 1 inputs, though.
                    // TODO(mingwei): Do we actually need to poll to completion or can we short-circuit?
                    #(
                        let #inputs = #root::dfir_pipes::pull::Pull::pull(
                            ::std::pin::Pin::as_mut(&mut #inputs),
                            <_ as #root::dfir_pipes::Context>::from_task(_cx),
                        );
                    )*
                    #(
                        if let #root::dfir_pipes::pull::PullStep::Pending(_) = #inputs {
                            return #root::dfir_pipes::pull::PullStep::Pending(#root::dfir_pipes::Yes);
                        }
                    )*
                    #root::dfir_pipes::pull::PullStep::<_, _, #root::dfir_pipes::Yes, _>::Ended(#root::dfir_pipes::Yes)
                }
            });
        }
    } else {
        quote_spanned! {op_span=>
            #[allow(clippy::let_unit_value, clippy::ignored_unit_patterns)]
            let _ = (#(#outputs),*);
            let #ident = #root::dfir_pipes::push::for_each::<_, #iter_type>(::std::mem::drop::<#iter_type>);
        }
    }
}

/// [`OperatorConstraints::write_fn`] for the null operator - an operator that ignores all inputs
/// and produces no output.
pub const NULL_WRITE_FN: WriteFn = |write_context_args, _| {
    let write_iterator = null_write_iterator_fn(write_context_args);
    Ok(OperatorWriteOutput {
        write_iterator,
        ..Default::default()
    })
};

macro_rules! declare_ops {
    ( $( $mod:ident :: $op:ident, )* ) => {
        $( pub(crate) mod $mod; )*
        /// All DFIR operators.
        pub const OPERATORS: &[OperatorConstraints] = &[
            $( $mod :: $op, )*
        ];
    };
}
declare_ops![
    all_iterations::ALL_ITERATIONS,
    anti_join::ANTI_JOIN,
    assert::ASSERT,
    assert_eq::ASSERT_EQ,
    batch::BATCH,
    batch_eager::BATCH_EAGER,
    batch_lazy::BATCH_LAZY,
    chain::CHAIN,
    chain_first_n::CHAIN_FIRST_N,
    _counter::_COUNTER,
    cross_join::CROSS_JOIN,
    cross_join_multiset::CROSS_JOIN_MULTISET,
    cross_singleton::CROSS_SINGLETON,
    demux_enum::DEMUX_ENUM,
    dest_file::DEST_FILE,
    dest_sink::DEST_SINK,
    dest_sink_serde::DEST_SINK_SERDE,
    difference::DIFFERENCE,
    enumerate::ENUMERATE,
    filter::FILTER,
    filter_map::FILTER_MAP,
    flat_map::FLAT_MAP,
    flat_map_stream_blocking::FLAT_MAP_STREAM_BLOCKING,
    flatten::FLATTEN,
    flatten_stream_blocking::FLATTEN_STREAM_BLOCKING,
    fold::FOLD,
    fold_no_replay::FOLD_NO_REPLAY,
    for_each::FOR_EACH,
    identity::IDENTITY,
    initialize::INITIALIZE,
    inspect::INSPECT,
    iter_ref::ITER_REF,
    join::JOIN,
    join_fused::JOIN_FUSED,
    join_fused_lhs::JOIN_FUSED_LHS,
    join_fused_rhs::JOIN_FUSED_RHS,
    join_multiset::JOIN_MULTISET,
    join_multiset_half::JOIN_MULTISET_HALF,
    fold_keyed::FOLD_KEYED,
    reduce_keyed::REDUCE_KEYED,
    lattice_bimorphism::LATTICE_BIMORPHISM,
    _lattice_fold_batch::_LATTICE_FOLD_BATCH,
    lattice_fold::LATTICE_FOLD,
    _lattice_join_fused_join::_LATTICE_JOIN_FUSED_JOIN,
    lattice_reduce::LATTICE_REDUCE,
    map::MAP,
    union::UNION,
    multiset_delta::MULTISET_DELTA,
    defer_signal::DEFER_SIGNAL,
    defer_tick::DEFER_TICK,
    defer_tick_lazy::DEFER_TICK_LAZY,
    null::NULL,
    partition::PARTITION,
    persist::PERSIST,
    resolve_futures::RESOLVE_FUTURES,
    resolve_futures_blocking::RESOLVE_FUTURES_BLOCKING,
    resolve_futures_blocking_ordered::RESOLVE_FUTURES_BLOCKING_ORDERED,
    resolve_futures_ordered::RESOLVE_FUTURES_ORDERED,
    reduce::REDUCE,
    reduce_no_replay::REDUCE_NO_REPLAY,
    scan::SCAN,
    scan_async_blocking::SCAN_ASYNC_BLOCKING,
    spin::SPIN,
    sort::SORT,
    sort_by_key::SORT_BY_KEY,
    source_file::SOURCE_FILE,
    source_interval::SOURCE_INTERVAL,
    source_iter::SOURCE_ITER,
    source_json::SOURCE_JSON,
    source_stdin::SOURCE_STDIN,
    source_stream::SOURCE_STREAM,
    source_stream_serde::SOURCE_STREAM_SERDE,
    state::STATE,
    state_by::STATE_BY,
    tee::TEE,
    unique::UNIQUE,
    unzip::UNZIP,
    zip::ZIP,
    zip_longest::ZIP_LONGEST,
];

/// Get the operator lookup table, generating it if needed.
pub fn operator_lookup() -> &'static HashMap<&'static str, &'static OperatorConstraints> {
    pub static OPERATOR_LOOKUP: OnceLock<HashMap<&'static str, &'static OperatorConstraints>> =
        OnceLock::new();
    OPERATOR_LOOKUP.get_or_init(|| OPERATORS.iter().map(|op| (op.name, op)).collect())
}
/// Find an operator by [`GraphNode`].
pub fn find_node_op_constraints(node: &GraphNode) -> Option<&'static OperatorConstraints> {
    if let GraphNode::Operator(operator) = node {
        find_op_op_constraints(operator)
    } else {
        None
    }
}
/// Find an operator by an AST [`Operator`].
pub fn find_op_op_constraints(operator: &Operator) -> Option<&'static OperatorConstraints> {
    let name = &*operator.name_string();
    operator_lookup().get(name).copied()
}

/// Context arguments provided to [`OperatorConstraints::write_fn`].
#[derive(Clone)]
pub struct WriteContextArgs<'a> {
    /// `dfir` crate name for `use #root::something`.
    pub root: &'a TokenStream,
    /// `context` ident, the name of the provided
    /// [`dfir_rs::scheduled::Context`](https://hydro.run/rustdoc/dfir_rs/scheduled/context/struct.Context.html).
    pub context: &'a Ident,
    /// `df` ident, the name of the
    /// [`dfir_rs::scheduled::graph::Dfir`](https://hydro.run/rustdoc/dfir_rs/scheduled/graph/struct.Dfir.html)
    /// instance.
    pub df_ident: &'a Ident,
    /// Subgraph ID in which this operator is contained.
    pub subgraph_id: GraphSubgraphId,
    /// Node ID identifying this operator in the flat or partitioned graph meta-datastructure.
    pub node_id: GraphNodeId,
    /// Loop ID in which this operator is contained, or `None` if not in a loop.
    pub loop_id: Option<GraphLoopId>,
    /// The source span of this operator.
    pub op_span: Span,
    /// Tag for this operator appended to the generated identifier.
    pub op_tag: Option<String>,
    /// Identifier for a function to call when doing work outside the stream.
    pub work_fn: &'a Ident,
    /// Identifier for a function to wrap futures when doing work outside the stream.
    pub work_fn_async: &'a Ident,

    /// Ident the `Stream` or `Sink` should be assigned to.
    pub ident: &'a Ident,
    /// If a pull `Stream` (true) or push `Sink` (false) should be used.
    pub is_pull: bool,
    /// Input `Stream` operator idents (or ref idents; used for pull).
    pub inputs: &'a [Ident],
    /// Output `Sink` operator idents (or ref idents; used for push).
    pub outputs: &'a [Ident],

    /// Operator name.
    pub op_name: &'static str,
    /// Operator instance arguments object.
    pub op_inst: &'a OperatorInstance,
    /// Arguments provided by the user into the operator as arguments.
    /// I.e. the `a, b, c` in `-> my_op(a, b, c) -> `.
    ///
    /// These arguments include singleton postprocessing codegen, with
    /// [`std::cell::RefCell::borrow_mut`] code pre-generated.
    pub arguments: &'a Punctuated<Expr, Token![,]>,

    /// The codegen options passed to
    /// [`DfirGraph::as_code_with_options`](super::DfirGraph::as_code_with_options).
    ///
    /// Operators may consult these, e.g. to omit their own type-guard helper functions when
    /// [`AsCodeOptions::exclude_type_guards`] is set.
    pub options: &'a AsCodeOptions,
}
impl WriteContextArgs<'_> {
    /// Generate a (almost certainly) unique identifier with the given suffix.
    ///
    /// Includes the subgraph and node IDs in the generated identifier.
    ///
    /// This will always return the same identifier for a given `suffix`.
    pub fn make_ident(&self, suffix: impl AsRef<str>) -> Ident {
        Ident::new(
            &format!(
                "sg_{:?}_node_{:?}_{}",
                self.subgraph_id.data(),
                self.node_id.data(),
                suffix.as_ref(),
            ),
            self.op_span,
        )
    }

    /// Returns the given number of persistence arguments, defaulting to `'tick` when unspecified.
    pub fn persistence_args<const N: usize>(
        &self,
        diagnostics: &mut Diagnostics,
    ) -> [Persistence; N] {
        let len = self.op_inst.generics.persistence_args.len();
        if 0 != len && 1 != len && N != len {
            diagnostics.push(Diagnostic::spanned(
                self.op_span,
                Level::Error,
                format!(
                    "The operator `{}` only accepts 0, 1, or {} persistence arguments",
                    self.op_name, N
                ),
            ));
        }

        let mut out = [Persistence::Tick; N];
        self.op_inst
            .generics
            .persistence_args
            .iter()
            .copied()
            .cycle() // Re-use the first element for both persistences.
            .take(N)
            .enumerate()
            .for_each(|(i, p)| {
                out[i] = p;
            });
        out
    }
}

/// Argument indices of owned-item closures that are safe to build once per run.
///
/// Only `for_each` today. It consumes owned items and is how a program holds a run-long sink
/// (an open file, a channel sender). Closures that borrow their argument (`map`, `filter`,
/// `inspect`, `flat_map`, `partition`, `sort_by_key`) or that unify integer literals with both
/// neighbors (`fold(|old, s| ...)`) stay in the tick: moving them breaks inference or
/// higher-ranked borrow checking. `join_fused*` hoists its accumulator value separately.
fn top_level_closure_arg_indices(op_name: &str) -> &'static [usize] {
    match op_name {
        "for_each" => &[0],
        _ => &[],
    }
}

/// `true` when `expr` names something that exists only inside a tick (`context`, `df`, or a
/// handoff buffer). Those expressions cannot be built in the prologue.
pub(crate) fn expr_references_tick_local(expr: &Expr) -> bool {
    fn walk(tokens: TokenStream) -> bool {
        tokens.into_iter().any(|tt| match tt {
            TokenTree::Ident(ident) => {
                let name = ident.to_string();
                // `context` / `df` match [`super::CONTEXT`] / [`super::GRAPH`].
                name == super::CONTEXT || name == super::GRAPH || name.starts_with("hoff_")
            }
            TokenTree::Group(group) => walk(group.stream()),
            TokenTree::Punct(_) | TokenTree::Literal(_) => false,
        })
    }
    walk(quote::ToTokens::to_token_stream(expr))
}

/// `true` when `expr` names a local from the surrounding function.
///
/// Those captures borrow the caller's stack. Building the closure in the prologue would put that
/// borrow on the tick closure and break `Dfir::into_erased`, which
/// needs a `'static` tick. Statics (no lowercase letters, e.g. `COUNT`) and locals bound inside
/// `expr` are not captures. Unknown syntax is treated as capturing so we skip the hoist.
fn expr_has_free_captures(expr: &Expr) -> bool {
    fn is_local_name(name: &str) -> bool {
        name.chars().any(|c| c.is_ascii_lowercase())
    }
    fn path_is_capture(path: &Path, bound: &[String]) -> bool {
        if path.leading_colon.is_some() || path.segments.len() != 1 {
            return false;
        }
        let name = path.segments[0].ident.to_string();
        if !is_local_name(&name) || bound.iter().any(|b| b == &name) {
            return false;
        }
        true
    }
    fn bind_pat(pat: &Pat, bound: &mut Vec<String>) {
        match pat {
            Pat::Ident(pat) => bound.push(pat.ident.to_string()),
            Pat::Reference(pat) => bind_pat(&pat.pat, bound),
            Pat::Type(pat) => bind_pat(&pat.pat, bound),
            Pat::Tuple(pat) => pat.elems.iter().for_each(|p| bind_pat(p, bound)),
            Pat::TupleStruct(pat) => pat.elems.iter().for_each(|p| bind_pat(p, bound)),
            Pat::Slice(pat) => pat.elems.iter().for_each(|p| bind_pat(p, bound)),
            Pat::Struct(pat) => pat
                .fields
                .iter()
                .for_each(|field| bind_pat(&field.pat, bound)),
            Pat::Or(pat) => pat.cases.iter().for_each(|p| bind_pat(p, bound)),
            Pat::Paren(pat) => bind_pat(&pat.pat, bound),
            _ => {}
        }
    }
    fn walk_block(block: &Block, bound: &mut Vec<String>) -> bool {
        let start = bound.len();
        let mut free = false;
        for stmt in &block.stmts {
            free |= match stmt {
                Stmt::Local(local) => {
                    let init_free = local
                        .init
                        .as_ref()
                        .is_some_and(|init| walk_expr(&init.expr, bound));
                    bind_pat(&local.pat, bound);
                    init_free
                }
                Stmt::Expr(expr, _) => walk_expr(expr, bound),
                // `static` / items / macros: don't try to understand them.
                Stmt::Item(_) | Stmt::Macro(_) => true,
            };
        }
        bound.truncate(start);
        free
    }
    fn walk_expr(expr: &Expr, bound: &mut Vec<String>) -> bool {
        match expr {
            Expr::Path(path) => path_is_capture(&path.path, bound),
            Expr::Lit(_) | Expr::Continue(_) | Expr::Infer(_) | Expr::Verbatim(_) => false,
            Expr::Paren(expr) => walk_expr(&expr.expr, bound),
            Expr::Group(expr) => walk_expr(&expr.expr, bound),
            Expr::Reference(expr) => walk_expr(&expr.expr, bound),
            Expr::Unary(expr) => walk_expr(&expr.expr, bound),
            Expr::Try(expr) => walk_expr(&expr.expr, bound),
            Expr::Await(expr) => walk_expr(&expr.base, bound),
            Expr::Field(expr) => walk_expr(&expr.base, bound),
            Expr::Cast(expr) => walk_expr(&expr.expr, bound),
            Expr::Binary(expr) => walk_expr(&expr.left, bound) || walk_expr(&expr.right, bound),
            Expr::Assign(expr) => walk_expr(&expr.left, bound) || walk_expr(&expr.right, bound),
            Expr::Index(expr) => walk_expr(&expr.expr, bound) || walk_expr(&expr.index, bound),
            Expr::Range(expr) => {
                expr.start.as_ref().is_some_and(|e| walk_expr(e, bound))
                    || expr.end.as_ref().is_some_and(|e| walk_expr(e, bound))
            }
            Expr::Call(expr) => {
                walk_expr(&expr.func, bound) || expr.args.iter().any(|arg| walk_expr(arg, bound))
            }
            Expr::MethodCall(expr) => {
                walk_expr(&expr.receiver, bound)
                    || expr.args.iter().any(|arg| walk_expr(arg, bound))
            }
            Expr::Array(expr) => expr.elems.iter().any(|elem| walk_expr(elem, bound)),
            Expr::Tuple(expr) => expr.elems.iter().any(|elem| walk_expr(elem, bound)),
            Expr::Repeat(expr) => walk_expr(&expr.expr, bound) || walk_expr(&expr.len, bound),
            Expr::Struct(expr) => {
                path_is_capture(&expr.path, bound)
                    || expr
                        .fields
                        .iter()
                        .any(|field| walk_expr(&field.expr, bound))
                    || expr
                        .rest
                        .as_ref()
                        .is_some_and(|rest| walk_expr(rest, bound))
            }
            Expr::Closure(expr) => {
                let start = bound.len();
                expr.inputs.iter().for_each(|pat| bind_pat(pat, bound));
                let free = walk_expr(&expr.body, bound);
                bound.truncate(start);
                free
            }
            Expr::Block(expr) => walk_block(&expr.block, bound),
            Expr::Unsafe(expr) => walk_block(&expr.block, bound),
            Expr::Const(expr) => walk_block(&expr.block, bound),
            Expr::If(expr) => {
                walk_expr(&expr.cond, bound)
                    || walk_block(&expr.then_branch, bound)
                    || expr
                        .else_branch
                        .as_ref()
                        .is_some_and(|(_, else_expr)| walk_expr(else_expr, bound))
            }
            Expr::Loop(expr) => walk_block(&expr.body, bound),
            Expr::While(expr) => walk_expr(&expr.cond, bound) || walk_block(&expr.body, bound),
            Expr::ForLoop(expr) => {
                let iter_free = walk_expr(&expr.expr, bound);
                let start = bound.len();
                bind_pat(&expr.pat, bound);
                let body_free = walk_block(&expr.body, bound);
                bound.truncate(start);
                iter_free || body_free
            }
            Expr::Break(expr) => expr
                .expr
                .as_ref()
                .is_some_and(|expr| walk_expr(expr, bound)),
            Expr::Return(expr) => expr
                .expr
                .as_ref()
                .is_some_and(|expr| walk_expr(expr, bound)),
            Expr::Let(expr) => walk_expr(&expr.expr, bound),
            // Macros can hide captures (`println!("{}", local)`).
            Expr::Macro(_) => true,
            _ => true,
        }
    }
    let mut bound = Vec::new();
    walk_expr(expr, &mut bound)
}

/// One-argument wrapper around the closure stored in `slot`.
///
/// Rebuilt every tick; the stored closure is not. `&mut` of the stored closure is not
/// higher-ranked, so the operator must see this fresh wrapper instead.
fn forwarding_closure(slot: &Ident, span: Span) -> Expr {
    parse_quote_spanned! {span=>
        |__dfir_once_item| (#slot)(__dfir_once_item)
    }
}

/// Like the closure hoist, but the use-site tokens are the place itself.
///
/// Callers that already write `&mut #argument` (fused join accumulators) should use this so the
/// borrow is not doubled.
pub(crate) fn hoist_top_level_place(
    wc: &WriteContextArgs<'_>,
    expr: &Expr,
    suffix: &str,
) -> (TokenStream, TokenStream) {
    if wc.loop_id.is_some() || expr_references_tick_local(expr) || expr_has_free_captures(expr) {
        return (TokenStream::new(), quote_spanned!(wc.op_span=> #expr));
    }
    let ident = wc.make_ident(suffix);
    let prologue = quote_spanned! {wc.op_span=>
        #[allow(unused_mut, reason = "accumulator may be a Fn or a struct")]
        let mut #ident = #expr;
    };
    let use_tokens = quote_spanned! {wc.op_span=> #ident };
    (prologue, use_tokens)
}

/// Rewrite closure arguments of a top-level operator so each is constructed once per run.
///
/// Returns prologue statements declaring the bindings. `arguments` is updated to a forwarding
/// closure for each hoisted argument.
pub(crate) fn hoist_top_level_closure_args(
    loop_id: Option<GraphLoopId>,
    op_name: &str,
    subgraph_id: GraphSubgraphId,
    node_id: GraphNodeId,
    op_span: Span,
    arguments: &mut Punctuated<Expr, Token![,]>,
) -> TokenStream {
    if loop_id.is_some() {
        return TokenStream::new();
    }
    let indices = top_level_closure_arg_indices(op_name);
    if indices.is_empty() {
        return TokenStream::new();
    }

    let mut prologue = TokenStream::new();
    for (index, expr) in arguments.iter_mut().enumerate() {
        if !indices.contains(&index)
            || expr_references_tick_local(expr)
            || expr_has_free_captures(expr)
        {
            continue;
        }
        let slot = Ident::new(
            &format!(
                "sg_{:?}_node_{:?}_fn_once_{}",
                subgraph_id.data(),
                node_id.data(),
                index
            ),
            op_span,
        );
        let original = expr.clone();
        prologue.extend(quote_spanned! {op_span=>
            #[allow(unused_mut, reason = "Fn closures need not be mutated")]
            let mut #slot = #original;
        });
        *expr = forwarding_closure(&slot, op_span);
    }
    prologue
}

/// An object-safe version of [`RangeBounds`].
pub trait RangeTrait<T>: Send + Sync + Debug
where
    T: ?Sized,
{
    /// Start (lower) bound.
    fn start_bound(&self) -> Bound<&T>;
    /// End (upper) bound.
    fn end_bound(&self) -> Bound<&T>;
    /// Returns if `item` is contained in this range.
    fn contains(&self, item: &T) -> bool
    where
        T: PartialOrd<T>;

    /// Turn this range into a human-readable string.
    fn human_string(&self) -> String
    where
        T: Display + PartialEq,
    {
        match (self.start_bound(), self.end_bound()) {
            (Bound::Unbounded, Bound::Unbounded) => "any number of".to_owned(),

            (Bound::Included(n), Bound::Included(x)) if n == x => {
                format!("exactly {}", n)
            }
            (Bound::Included(n), Bound::Included(x)) => {
                format!("at least {} and at most {}", n, x)
            }
            (Bound::Included(n), Bound::Excluded(x)) => {
                format!("at least {} and less than {}", n, x)
            }
            (Bound::Included(n), Bound::Unbounded) => format!("at least {}", n),
            (Bound::Excluded(n), Bound::Included(x)) => {
                format!("more than {} and at most {}", n, x)
            }
            (Bound::Excluded(n), Bound::Excluded(x)) => {
                format!("more than {} and less than {}", n, x)
            }
            (Bound::Excluded(n), Bound::Unbounded) => format!("more than {}", n),
            (Bound::Unbounded, Bound::Included(x)) => format!("at most {}", x),
            (Bound::Unbounded, Bound::Excluded(x)) => format!("less than {}", x),
        }
    }
}

impl<R, T> RangeTrait<T> for R
where
    R: RangeBounds<T> + Send + Sync + Debug,
{
    fn start_bound(&self) -> Bound<&T> {
        self.start_bound()
    }

    fn end_bound(&self) -> Bound<&T> {
        self.end_bound()
    }

    fn contains(&self, item: &T) -> bool
    where
        T: PartialOrd<T>,
    {
        self.contains(item)
    }
}

/// Persistence lifetimes: `'none`, `'loop`, `'tick`, or `'static`.
#[derive(Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Persistence {
    /// Persistence for one "tick" (or loop iteration), i.e. no persistance at all
    Tick,
    /// Persistence across the entire program execution.
    Static,
}
impl Persistence {
    /// Returns a lowercase string for the persistence type.
    pub fn to_str_lowercase(self) -> &'static str {
        match self {
            Persistence::Tick => "tick",
            Persistence::Static => "static",
        }
    }
}

/// Helper which creates a error message string literal for when the Tokio runtime is not found.
fn make_missing_runtime_msg(op_name: &str) -> Literal {
    Literal::string(&format!(
        "`{}()` must be used within a Tokio runtime. For example, use `#[dfir_rs::main]` on your main method.",
        op_name
    ))
}

/// Operator categories, for docs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, DocumentedVariants)]
pub enum OperatorCategory {
    /// Maps: Simple one-in-one-out operators.
    Map,
    /// Filters: One-in zero-or-one-out operators.
    Filter,
    /// Flattens: One-in multiple-out operators.
    Flatten,
    /// Folds: Operators which accumulate elements together.
    Fold,
    /// Keyed Folds: Operators which accumulate elements together by key.
    KeyedFold,
    /// Lattice Folds: Folds based on lattice-merge.
    LatticeFold,
    /// Persistent Operators: Persistent (stateful) operators.
    Persistence,
    /// Multi-Input Operators: Operators with multiple inputs.
    MultiIn,
    /// Multi-Output Operators: Operators with multiple outputs.
    MultiOut,
    /// Sources: Operators which produce output elements (and consume no inputs).
    Source,
    /// Sinks: Operators which consume input elements (and produce no outputs).
    Sink,
    /// Control Flow Operators: Operators which affect control flow/scheduling.
    Control,
    /// Compiler Fusion Operators: Operators which are necessary to implement certain optimizations and rewrite rules.
    CompilerFusionOperator,
    /// Windowing Operators: Operators for windowing `loop` inputs.
    Windowing,
    /// Un-Windowing Operators: Operators for collecting `loop` outputs.
    Unwindowing,
}
impl OperatorCategory {
    /// Human-readible heading name, for docs.
    pub fn name(self) -> &'static str {
        self.get_variant_docs().split_once(':').unwrap().0
    }
    /// Human description, for docs.
    pub fn description(self) -> &'static str {
        self.get_variant_docs().split_once(':').unwrap().1
    }
}

/// Operator type for Flo semantics.
#[derive(Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Debug)]
pub enum FloType {
    /// A source operator, which must be at the top level.
    Source,
    /// A windowing operator, for moving data into a loop context.
    Windowing,
    /// A lazy windowing operator — moves data into a loop context but does not trigger the loop.
    /// Data is dropped if the loop does not fire that tick.
    WindowingLazy,
    /// An eager windowing operator — moves data into a loop context and always triggers the loop,
    /// even when the windowed input is empty. Only valid at the entry of a root-level loop (it is
    /// disallowed in nested loops, where it would prevent the fixpoint iteration from terminating).
    WindowingEager,
    /// An un-windowing operator, for moving data out of a loop context.
    Unwindowing,
}
