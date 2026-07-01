use super::{
    identity_write_iterator_fn, FloType, OperatorCategory, OperatorConstraints,
    OperatorWriteOutput, WriteContextArgs, RANGE_0, RANGE_1,
};

/// Given an _unbounded_ input stream, emits values arbitrarily split into batches over multiple iterations in the same order.
///
/// Unlike `batch()`, this operator does NOT cause the loop to fire. If the loop
/// does not fire for another reason (e.g., a non-lazy `batch` or `defer_loop`
/// has data), the lazy-batched data is available. If the loop never fires that
/// tick, the data is simply dropped (reclaimed by the bump allocator at tick end).
pub const BATCH_LAZY: OperatorConstraints = OperatorConstraints {
    name: "batch_lazy",
    categories: &[OperatorCategory::Windowing],
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 0,
    persistence_args: RANGE_0,
    type_args: RANGE_0,
    is_external_input: false,
    flo_type: Some(FloType::WindowingLazy),
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| None,
    // Same as batch() — identity in inline codegen.
    write_fn: |wc @ &WriteContextArgs { .. }, _diagnostics| {
        let write_iterator = identity_write_iterator_fn(wc);
        Ok(OperatorWriteOutput {
            write_iterator,
            ..Default::default()
        })
    },
};
