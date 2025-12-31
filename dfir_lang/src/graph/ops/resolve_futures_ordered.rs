use syn::Ident;

use super::{
    OperatorCategory, OperatorConstraints, RANGE_0, RANGE_1,
    resolve_futures::resolve_futures_writer,
};

/// Given an incoming stream of `F: Future`, sends those futures to the executor being used
/// by the DFIR runtime. Yields the results of each future in the same order as the futures are
/// received, so the output will always be blocked on the first remaining unresolved future.
/// However, multiple futures may be spawned concurrently so there is no guarantee on when they are
/// executed.
pub const RESOLVE_FUTURES_ORDERED: OperatorConstraints = OperatorConstraints {
    name: "resolve_futures_ordered",
    categories: &[OperatorCategory::Map],
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 0,
    persistence_args: RANGE_0,
    type_args: RANGE_0,
    is_external_input: false,
    has_singleton_output: false,
    flo_type: None,
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| None,
    write_fn: move |wc, _| {
        resolve_futures_writer(Ident::new("FuturesOrdered", wc.op_span), false, wc)
    },
};
