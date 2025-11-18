//! DFIR's operators

use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::ops::{Bound, RangeBounds};
use std::sync::OnceLock;

use documented::DocumentedVariants;
use proc_macro2::{Ident, Literal, Span, TokenStream};
use quote::quote_spanned;
use serde::{Deserialize, Serialize};
use slotmap::Key;
use syn::punctuated::Punctuated;
use syn::{Expr, Token, parse_quote_spanned};

use super::{
    GraphLoopId, GraphNode, GraphNodeId, GraphSubgraphId, OpInstGenerics, OperatorInstance,
    PortIndexValue,
};
use crate::diagnostic::{Diagnostic, Level};
use crate::parse::{Operator, PortIndex};

/// The delay (soft barrier) type, for each input to an operator if needed.
#[derive(Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Debug)]
pub enum DelayType {
    /// Input must be collected over the preceding stratum.
    Stratum,
    /// Monotone accumulation: can delay to reduce flow rate, but also correct to emit "early"
    MonotoneAccum,
    /// Input must be collected over the previous tick.
    Tick,
    /// Input must be collected over the previous tick but also not cause a new tick to occur.
    TickLazy,
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
    /// If this operator has a singleton reference output. For stateful operators.
    /// If true, [`WriteContextArgs::singleton_output_ident`] will be set to a meaningful value in
    /// the [`Self::write_fn`] invocation.
    pub has_singleton_output: bool,
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
pub type WriteFn =
    fn(&WriteContextArgs<'_>, &mut Vec<Diagnostic>) -> Result<OperatorWriteOutput, ()>;

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
#[derive(Default)]
#[non_exhaustive]
pub struct OperatorWriteOutput {
    /// Code which runs once outside any subgraphs, BEFORE subgraphs are initialized,
    /// to set up any external state (state API, chanels, network connections, etc.)
    /// to be used by the subgraph.
    pub write_prologue: TokenStream,
    /// Code which runs once outside the subgraph, AFTER subgraphs are initialized,
    /// to set up state hooks which may need the subgraph ID.
    pub write_prologue_after: TokenStream,
    /// Iterator (or pusherator) code inside the subgraphs. The code for each
    /// operator is emitted in order.
    ///
    /// Emitted code should assign to [`WriteContextArgs::ident`] and use
    /// [`WriteContextArgs::inputs`] (pull iterators) or
    /// [`WriteContextArgs::outputs`] (pusherators).
    pub write_iterator: TokenStream,
    /// Code which runs after iterators have been run. Mainly for flushing IO.
    pub write_iterator_after: TokenStream,
}

/// Convenience range: zero or more (any number).
pub const RANGE_ANY: &'static dyn RangeTrait<usize> = &(0..);
/// Convenience range: exactly zero.
pub const RANGE_0: &'static dyn RangeTrait<usize> = &(0..=0);
/// Convenience range: exactly one.
pub const RANGE_1: &'static dyn RangeTrait<usize> = &(1..=1);

/// Helper to write the `write_iterator` portion of [`OperatorConstraints::write_fn`] output for
/// unary identity operators.
pub fn identity_write_iterator_fn(
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
    }: &WriteContextArgs,
) -> TokenStream {
    let generic_type = type_args
        .first()
        .map(quote::ToTokens::to_token_stream)
        .unwrap_or(quote_spanned!(op_span=> _));

    if is_pull {
        let input = &inputs[0];
        quote_spanned! {op_span=>
            let #ident = {
                fn check_input<St, Item>(stream: St) -> impl #root::futures::stream::Stream<Item = Item>
                where
                    St: #root::futures::stream::Stream<Item = Item>,
                {
                    stream
                }
                check_input::<_, #generic_type>(#input)
            };
        }
    } else {
        let output = &outputs[0];
        quote_spanned! {op_span=>
            let #ident = {
                fn check_output<Si, Item>(sink: Si) -> impl #root::futures::sink::Sink<Item>
                where
                    Si: #root::futures::sink::Sink<Item>,
                {
                    sink
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
    }: &WriteContextArgs,
) -> TokenStream {
    let default_type = parse_quote_spanned! {op_span=> _};
    let iter_type = type_args.first().unwrap_or(&default_type);

    if is_pull {
        quote_spanned! {op_span=>
            let #ident = #root::futures::stream::poll_fn(move |cx| {
                // Make sure to poll all #inputs to completion.
                #(
                    let #inputs = #root::futures::stream::Stream::poll_next(::std::pin::pin!(#inputs), cx);
                )*
                #(
                    let _ = ::std::task::ready!(#inputs);
                )*
                ::std::task::Poll::Ready(::std::option::Option::None)
            });
        }
    } else {
        quote_spanned! {op_span=>
            #[allow(clippy::let_unit_value)]
            let _ = (#(#outputs),*);
            let #ident = #root::sinktools::for_each::ForEach::new::<#iter_type>(::std::mem::drop::<#iter_type>);
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
    all_once::ALL_ONCE,
    anti_join::ANTI_JOIN,
    anti_join_multiset::ANTI_JOIN_MULTISET,
    assert::ASSERT,
    assert_eq::ASSERT_EQ,
    batch::BATCH,
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
    difference_multiset::DIFFERENCE_MULTISET,
    enumerate::ENUMERATE,
    filter::FILTER,
    filter_map::FILTER_MAP,
    flat_map::FLAT_MAP,
    flatten::FLATTEN,
    fold::FOLD,
    for_each::FOR_EACH,
    identity::IDENTITY,
    initialize::INITIALIZE,
    inspect::INSPECT,
    join::JOIN,
    join_fused::JOIN_FUSED,
    join_fused_lhs::JOIN_FUSED_LHS,
    join_fused_rhs::JOIN_FUSED_RHS,
    join_multiset::JOIN_MULTISET,
    fold_keyed::FOLD_KEYED,
    reduce_keyed::REDUCE_KEYED,
    repeat_n::REPEAT_N,
    // last_iteration::LAST_ITERATION,
    lattice_bimorphism::LATTICE_BIMORPHISM,
    _lattice_fold_batch::_LATTICE_FOLD_BATCH,
    lattice_fold::LATTICE_FOLD,
    _lattice_join_fused_join::_LATTICE_JOIN_FUSED_JOIN,
    lattice_reduce::LATTICE_REDUCE,
    map::MAP,
    union::UNION,
    multiset_delta::MULTISET_DELTA,
    next_iteration::NEXT_ITERATION,
    next_stratum::NEXT_STRATUM,
    defer_signal::DEFER_SIGNAL,
    defer_tick::DEFER_TICK,
    defer_tick_lazy::DEFER_TICK_LAZY,
    null::NULL,
    partition::PARTITION,
    persist::PERSIST,
    persist_mut::PERSIST_MUT,
    persist_mut_keyed::PERSIST_MUT_KEYED,
    prefix::PREFIX,
    resolve_futures::RESOLVE_FUTURES,
    resolve_futures_blocking::RESOLVE_FUTURES_BLOCKING,
    resolve_futures_blocking_ordered::RESOLVE_FUTURES_BLOCKING_ORDERED,
    resolve_futures_ordered::RESOLVE_FUTURES_ORDERED,
    reduce::REDUCE,
    scan::SCAN,
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
    /// Identifier for a function to call when doing work outside the iterator.
    pub work_fn: &'a Ident,

    /// Ident the iterator or pullerator should be assigned to.
    pub ident: &'a Ident,
    /// If a pull iterator (true) or pusherator (false) should be used.
    pub is_pull: bool,
    /// Input operator idents (or ref idents; used for pull).
    pub inputs: &'a [Ident],
    /// Output operator idents (or ref idents; used for push).
    pub outputs: &'a [Ident],
    /// Ident for the singleton output of this operator, if any.
    pub singleton_output_ident: &'a Ident,

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
    /// Same as [`Self::arguments`] but with only `StateHandle`s, no borrowing code.
    pub arguments_handles: &'a Punctuated<Expr, Token![,]>,
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

    /// Returns `#root::scheduled::graph::StateLifespan::#variant` corresponding to the given
    /// peristence.
    pub fn persistence_as_state_lifespan(&self, persistence: Persistence) -> Option<TokenStream> {
        let root = self.root;
        let variant =
            persistence.as_state_lifespan_variant(self.subgraph_id, self.loop_id, self.op_span)?;
        Some(quote_spanned! {self.op_span=>
            #root::scheduled::graph::StateLifespan::#variant
        })
    }

    /// Returns the given number of persistence arguments, disallowing mutable lifetimes.
    pub fn persistence_args_disallow_mutable<const N: usize>(
        &self,
        diagnostics: &mut Vec<Diagnostic>,
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

        let default_persistence = if self.loop_id.is_some() {
            Persistence::None
        } else {
            Persistence::Tick
        };
        let mut out = [default_persistence; N];
        self.op_inst
            .generics
            .persistence_args
            .iter()
            .copied()
            .cycle() // Re-use the first element for both persistences.
            .take(N)
            .enumerate()
            .filter(|&(_i, p)| {
                if p == Persistence::Mutable {
                    diagnostics.push(Diagnostic::spanned(
                        self.op_span,
                        Level::Error,
                        format!(
                            "An implementation of `'{}` does not exist",
                            p.to_str_lowercase()
                        ),
                    ));
                    false
                } else {
                    true
                }
            })
            .for_each(|(i, p)| {
                out[i] = p;
            });
        out
    }
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

/// Persistence lifetimes: `'none`, `'tick`, `'static`, or `'mutable`.
#[derive(Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Persistence {
    /// No persistence, for within a loop iteration.
    None,
    /// Persistence throughout a single loop execution, across iterations.
    Loop,
    /// Persistence for one tick, at the top-level only (outside any loops).
    Tick,
    /// Persistence across all ticks.
    Static,
    /// The static lifetime but allowing non-monotonic mutability.
    Mutable,
}
impl Persistence {
    /// Returns just the variant of `#root::scheduled::graph::StateLifespan::VARIANT` for use in macros.
    pub fn as_state_lifespan_variant(
        self,
        subgraph_id: GraphSubgraphId,
        loop_id: Option<GraphLoopId>,
        span: Span,
    ) -> Option<TokenStream> {
        match self {
            Persistence::None => {
                let sg_ident = subgraph_id.as_ident(span);
                Some(quote_spanned!(span=> Subgraph(#sg_ident)))
            }
            Persistence::Loop => {
                let loop_ident = loop_id
                    .expect("`Persistence::Loop` outside of a loop context.")
                    .as_ident(span);
                Some(quote_spanned!(span=> Loop(#loop_ident)))
            }
            Persistence::Tick => Some(quote_spanned!(span=> Tick)),
            Persistence::Static => None,
            Persistence::Mutable => None,
        }
    }

    /// Returns a lowercase string for the persistence type.
    pub fn to_str_lowercase(self) -> &'static str {
        match self {
            Persistence::None => "none",
            Persistence::Tick => "tick",
            Persistence::Loop => "loop",
            Persistence::Static => "static",
            Persistence::Mutable => "mutable",
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
        self.get_variant_docs().split_once(":").unwrap().0
    }
    /// Human description, for docs.
    pub fn description(self) -> &'static str {
        self.get_variant_docs().split_once(":").unwrap().1
    }
}

/// Operator type for Flo semantics.
#[derive(Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Debug)]
pub enum FloType {
    /// A source operator, which must be at the top level.
    Source,
    /// A windowing operator, for moving data into a loop context.
    Windowing,
    /// An un-windowing operator, for moving data out of a loop context.
    Unwindowing,
    /// Moves data into the next loop iteration within a loop context.
    NextIteration,
}
