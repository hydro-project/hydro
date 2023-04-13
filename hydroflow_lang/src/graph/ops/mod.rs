use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::ops::{Bound, RangeBounds};

use once_cell::sync::OnceCell;
use proc_macro2::{Ident, Literal, Span, TokenStream};
use quote::quote_spanned;
use slotmap::Key;
use syn::punctuated::Punctuated;
use syn::{parse_quote_spanned, Token};

use crate::diagnostic::Diagnostic;
use crate::parse::{Operator, PortIndex};

use super::{GraphNodeId, GraphSubgraphId, Node, OpInstGenerics, OperatorInstance, PortIndexValue};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Debug)]
pub enum DelayType {
    Stratum,
    Tick,
}

pub enum PortListSpec {
    Variadic,
    Fixed(Punctuated<PortIndex, Token![,]>),
}

#[derive(Default, Debug, Eq, PartialEq, Clone, Copy)]
pub enum FlowPropertyVal {
    #[default]
    No,
    Yes,
    Preserve,
    DependsOnArgs,
}
#[derive(Default, Debug, Eq, PartialEq, Clone, Copy)]
pub struct FlowProperties {
    /// Is the flow deterministic.
    pub deterministic: FlowPropertyVal,
    /// Is the flow monotonic.
    pub monotonic: FlowPropertyVal,
    /// Has inconsistency been introduced.
    pub inconsistency_tainted: bool,
}

pub struct OperatorConstraints {
    /// Operator's name.
    pub name: &'static str,

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

    /// What named or numbered input ports to expect?
    pub ports_inn: Option<fn() -> PortListSpec>,
    /// What named or numbered output ports to expect?
    pub ports_out: Option<fn() -> PortListSpec>,

    /// Monotonicity preservation properties, for analysis.
    pub properties: FlowProperties,

    /// Determines if this input must be preceeded by a stratum barrier.
    pub input_delaytype_fn: fn(&PortIndexValue) -> Option<DelayType>,

    /// Emit code in multiple locations. See [`OperatorWriteOutput`].
    pub write_fn: WriteFn,
}
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
            // .field("write_fn", &self.write_fn)
            .finish()
    }
}

pub type WriteFn =
    fn(&WriteContextArgs<'_>, &mut Vec<Diagnostic>) -> Result<OperatorWriteOutput, ()>;

#[derive(Default)]
#[non_exhaustive]
pub struct OperatorWriteOutput {
    /// Code which runs once outside the subgraph to set up any external stuff
    /// like state API stuff, external chanels, network connections, etc.
    pub write_prologue: TokenStream,
    /// Iterator (or pusherator) code inside the subgraphs. The code for each
    /// operator is emitted in order.
    ///
    /// Emitted code should assign to [`WriteContextArgs.ident`] and use
    /// [`WriteIteratorArgs.inputs`] (pull iterators) or
    /// [`WriteIteratorArgs.outputs`] (pusherators).
    pub write_iterator: TokenStream,
    /// Code which runs after iterators have been run. Mainly for flushing IO.
    pub write_iterator_after: TokenStream,
}

pub const RANGE_ANY: &'static dyn RangeTrait<usize> = &(0..);
pub const RANGE_0: &'static dyn RangeTrait<usize> = &(0..=0);
pub const RANGE_1: &'static dyn RangeTrait<usize> = &(1..=1);

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
        .get(0)
        .map(quote::ToTokens::to_token_stream)
        .unwrap_or(quote_spanned!(op_span=> _));

    if is_pull {
        let input = &inputs[0];
        quote_spanned! {op_span=>
            let #ident = {
                fn check_input<Iter: ::std::iter::Iterator<Item = Item>, Item>(iter: Iter) -> impl ::std::iter::Iterator<Item = Item> { iter }
                check_input::<_, #generic_type>(#input)
            };
        }
    } else {
        let output = &outputs[0];
        quote_spanned! {op_span=>
            let #ident = {
                fn check_output<Push: #root::pusherator::Pusherator<Item = Item>, Item>(push: Push) -> impl #root::pusherator::Pusherator<Item = Item> { push }
                check_output::<_, #generic_type>(#output)
            }
        }
    }
}

pub const IDENTITY_WRITE_FN: WriteFn = |write_context_args, _| {
    let write_iterator = identity_write_iterator_fn(write_context_args);
    Ok(OperatorWriteOutput {
        write_iterator,
        ..Default::default()
    })
};

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
    let iter_type = type_args.get(0).unwrap_or(&default_type);
    if is_pull {
        quote_spanned! {op_span=>
            #(
                #inputs.for_each(std::mem::drop);
            )*
            let #ident = std::iter::empty::<#iter_type>();
        }
    } else {
        quote_spanned! {op_span=>
            #[allow(clippy::let_unit_value)]
            let _ = (#(#outputs),*);
            let #ident = #root::pusherator::for_each::ForEach::<_, #iter_type>::new(std::mem::drop);
        }
    }
}

pub const NULL_WRITE_FN: WriteFn = |write_context_args, _| {
    let write_iterator = null_write_iterator_fn(write_context_args);
    Ok(OperatorWriteOutput {
        write_iterator,
        ..Default::default()
    })
};

macro_rules! declare_ops {
    ( $( $mod:ident :: $op:ident, )* ) => {
        $( mod $mod; )*
        pub const OPERATORS: &[OperatorConstraints] = &[
            $( $mod :: $op, )*
        ];
    };
}
declare_ops![
    anti_join::ANTI_JOIN,
    batch::BATCH,
    cross_join::CROSS_JOIN,
    demux::DEMUX,
    dest_file::DEST_FILE,
    dest_sink::DEST_SINK,
    dest_sink_chunked::DEST_SINK_CHUNKED,
    dest_sink_serde::DEST_SINK_SERDE,
    difference::DIFFERENCE,
    filter::FILTER,
    filter_map::FILTER_MAP,
    flat_map::FLAT_MAP,
    flatten::FLATTEN,
    fold::FOLD,
    for_each::FOR_EACH,
    group_by::GROUP_BY,
    identity::IDENTITY,
    initialize::INITIALIZE,
    inspect::INSPECT,
    join::JOIN,
    lattice_join::LATTICE_JOIN,
    lattice_merge::LATTICE_MERGE,
    map::MAP,
    merge::MERGE,
    next_stratum::NEXT_STRATUM,
    next_tick::NEXT_TICK,
    null::NULL,
    persist::PERSIST,
    reduce::REDUCE,
    repeat_fn::REPEAT_FN,
    repeat_iter::REPEAT_ITER,
    repeat_iter_external::REPEAT_ITER_EXTERNAL,
    sort::SORT,
    sort_by::SORT_BY,
    source_file::SOURCE_FILE,
    source_interval::SOURCE_INTERVAL,
    source_iter::SOURCE_ITER,
    source_json::SOURCE_JSON,
    source_stdin::SOURCE_STDIN,
    source_stream::SOURCE_STREAM,
    source_stream_serde::SOURCE_STREAM_SERDE,
    tee::TEE,
    unique::UNIQUE,
    unzip::UNZIP,
];

pub fn operator_lookup() -> &'static HashMap<&'static str, &'static OperatorConstraints> {
    pub static OPERATOR_LOOKUP: OnceCell<HashMap<&'static str, &'static OperatorConstraints>> =
        OnceCell::new();
    OPERATOR_LOOKUP.get_or_init(|| OPERATORS.iter().map(|op| (op.name, op)).collect())
}
pub fn find_node_op_constraints(node: &Node) -> Option<&'static OperatorConstraints> {
    if let Node::Operator(operator) = node {
        find_op_op_constraints(operator)
    } else {
        None
    }
}
pub fn find_op_op_constraints(operator: &Operator) -> Option<&'static OperatorConstraints> {
    let name = &*operator.name_string();
    operator_lookup().get(name).copied()
}

#[derive(Clone)]
pub struct WriteContextArgs<'a> {
    /// `hydroflow` crate name for `use #root::something`.
    pub root: &'a TokenStream,
    /// `context` ident, the name of the provided
    /// [`hydroflow::scheduled::Context`](https://hydro-project.github.io/hydroflow/doc/hydroflow/scheduled/context/struct.Context.html).
    pub context: &'a Ident,
    /// `df` ident, the name of the
    /// [`hydroflow::scheduled::graph::Hydroflow`](https://hydro-project.github.io/hydroflow/doc/hydroflow/scheduled/graph/struct.Hydroflow.html)
    /// instance.
    pub hydroflow: &'a Ident,
    /// Subgraph ID in which this operator is contained.
    pub subgraph_id: GraphSubgraphId,
    /// Node ID identifying this operator in the flat or partitioned graph meta-datastructure.
    pub node_id: GraphNodeId,
    /// The source span of this operator.
    pub op_span: Span,

    /// Ident the iterator or pullerator should be assigned to.
    pub ident: &'a Ident,
    /// If a pull iterator (true) or pusherator (false) should be used.
    pub is_pull: bool,
    /// Input operator idents (used for pull).
    pub inputs: &'a [Ident],
    /// Output operator idents (used for push).
    pub outputs: &'a [Ident],

    /// Operator name.
    pub op_name: &'static str,
    /// Operator instance arguments object.
    pub op_inst: &'a OperatorInstance,
}
impl WriteContextArgs<'_> {
    pub fn make_ident(&self, suffix: impl AsRef<str>) -> Ident {
        Ident::new(
            &*format!(
                "sg_{:?}_node_{:?}_{}",
                self.subgraph_id.data(),
                self.node_id.data(),
                suffix.as_ref(),
            ),
            self.op_span,
        )
    }
}

pub trait RangeTrait<T>: Send + Sync + std::fmt::Debug
where
    T: ?Sized,
{
    fn start_bound(&self) -> Bound<&T>;
    fn end_bound(&self) -> Bound<&T>;
    fn contains(&self, item: &T) -> bool
    where
        T: PartialOrd<T>;

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
    R: RangeBounds<T> + Send + Sync + std::fmt::Debug,
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

#[derive(Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Persistence {
    Tick,
    Static,
}

fn make_missing_runtime_msg(op_name: &str) -> Literal {
    Literal::string(&*format!("`{}()` must be used within a Tokio runtime. For example, use `#[tokio::main]` on your main method.", op_name))
}
