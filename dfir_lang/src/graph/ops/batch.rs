use super::{
    identity_write_iterator_fn, FloType, OperatorCategory, OperatorConstraints,
    OperatorWriteOutput, WriteContextArgs, RANGE_0, RANGE_1,
};

/// Given an _unbounded_ input stream, emits values arbitrarily split into batches over multiple iterations in the same order.
///
/// Will cause additional loop iterations as long as new values arrive.
///
/// `batch()` is one of four loop-ingress ("windowing") operators, which differ in whether they
/// cause the surrounding `loop { ... }` to fire and whether pending data is retained:
/// - `batch()` triggers the loop only when its windowed input is non-empty.
/// - `batch_lazy()` never triggers the loop on its own; its data is only observed if the loop
///   fires for some other reason (otherwise dropped at tick end).
/// - `batch_eager()` always triggers the loop, even when the windowed input is empty.
/// - `snapshot()` never triggers the loop on its own, but retains pending data across ticks and
///   delivers it at the loop's next firing (nothing is dropped).
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
    flo_type: Some(FloType::Windowing),
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| None,
    // Scheduler automatically handles the batching of values as this is a `OperatorCategory::Windowing` operator.
    // In inline codegen, batch() is identity — the loop boundary handoff provides the buffering/gating.
    write_fn: |wc @ &WriteContextArgs { .. }, _diagnostics| {
        let write_iterator = identity_write_iterator_fn(wc);
        Ok(OperatorWriteOutput {
            write_iterator,
            ..Default::default()
        })
    },
};
