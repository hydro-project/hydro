use syn::Ident;

use super::{
    resolve_futures_blocking::resolve_futures_writer, OperatorCategory, OperatorConstraints, RANGE_0, RANGE_1
};

/// Given an incoming stream of `F: Future`, resolves each future, blocking the subgraph execution.
/// Yields the results of each future in the same order as the futures are received, so the output
/// will always be blocked on the first remaining unresolved future. The next future will not be
/// started until the prior future is resolved.
pub const RESOLVE_FUTURES_BLOCKING_ORDERED: OperatorConstraints = OperatorConstraints {
    name: "resolve_futures_blocking_ordered",
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
        resolve_futures_writer(
            Ident::new("FuturesOrdered", wc.op_span),
            wc
        )
    }
};
