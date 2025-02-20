use quote::quote_spanned;

use super::{
    identity_write_iterator_fn, FloType, OperatorCategory, OperatorConstraints,
    OperatorWriteOutput, WriteContextArgs, RANGE_0, RANGE_1,
};

/// Given an _unbounded_ input stream, emits values arbitrarily split into batches over multiple iterations in the same order.
///
/// Will cause additional loop iterations as long as new values arrive.
pub const BATCH: OperatorConstraints = OperatorConstraints {
    name: "batch",
    categories: &[OperatorCategory::Windowing],
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 0,
    persistence_args: RANGE_0,
    type_args: RANGE_0,
    is_external_input: false,
    has_singleton_output: true,
    flo_type: Some(FloType::Windowing),
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| None,
    // Scheduler automatically handles the batching of values as this is a `OperatorCategory::Windowing` operator.
    write_fn: |wc @ &WriteContextArgs {
                   context, op_span, ..
               },
               _diagnostics| {
        let write_iterator = identity_write_iterator_fn(wc);
        let write_iterator_after = quote_spanned! {op_span=>
            #context.allow_another_iteration();
        };
        Ok(OperatorWriteOutput {
            write_iterator,
            write_iterator_after,
            ..Default::default()
        })
    },
};
