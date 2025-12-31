use syn::Ident;

use super::{
    OperatorCategory, OperatorConstraints, RANGE_0, RANGE_1,
    resolve_futures::resolve_futures_writer,
};

/// Given an incoming stream of `F: Future`, resolves each future, blocking the subgraph execution.
/// Until the results are resolved. The output order is based on when futures complete, and may be
/// different than the input order.
pub const RESOLVE_FUTURES_BLOCKING: OperatorConstraints = OperatorConstraints {
    name: "resolve_futures_blocking",
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
        resolve_futures_writer(Ident::new("FuturesUnordered", wc.op_span), true, wc)
    },
};
