use super::{
    identity_write_iterator_fn, FloType, OperatorCategory, OperatorConstraints,
    OperatorWriteOutput, WriteContextArgs, RANGE_0, RANGE_1,
};

/// Given an _unbounded_ input stream, emits values arbitrarily split into batches over multiple iterations in the same order.
///
/// `batch_eager()` is one of three loop-ingress ("windowing") operators, which differ only in
/// whether they cause the surrounding `loop { ... }` to fire:
/// - `batch()` triggers the loop only when its windowed input is non-empty.
/// - `batch_lazy()` never triggers the loop on its own; its data is only observed if the loop
///   fires for some other reason (otherwise dropped at tick end).
/// - `batch_eager()` **always** triggers the loop, even when the windowed input is empty.
///
/// Because it forces the loop body to run, `batch_eager()` is only valid at the entry of a
/// root-level loop. It is disallowed inside nested loops, where forcing the loop to always fire
/// would prevent the fixpoint iteration from terminating.
///
/// Note that `batch_eager()` only forces the loop body to run when a tick executes; it does not
/// schedule additional ticks on its own (unlike `spin()`).
pub const BATCH_EAGER: OperatorConstraints = OperatorConstraints {
    name: "batch_eager",
    categories: &[OperatorCategory::Windowing],
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 0,
    persistence_args: RANGE_0,
    type_args: RANGE_0,
    is_external_input: false,
    flo_type: Some(FloType::WindowingEager),
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| None,
    // Same as batch() — identity in inline codegen. The loop-gate logic (see `emit_loop_gate`)
    // uses the `WindowingEager` flo type to force the loop to fire unconditionally.
    write_fn: |wc @ &WriteContextArgs { .. }, _diagnostics| {
        let write_iterator = identity_write_iterator_fn(wc);
        Ok(OperatorWriteOutput {
            write_iterator,
            ..Default::default()
        })
    },
};
