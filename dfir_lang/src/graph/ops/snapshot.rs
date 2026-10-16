use super::{
    FloType, OperatorCategory, OperatorConstraints, OperatorWriteOutput, RANGE_0, RANGE_1,
    WriteContextArgs, identity_write_iterator_fn,
};

/// Given an _unbounded_ input stream, emits values arbitrarily split into batches over multiple
/// iterations in the same order, **retaining** pending values across ticks until the loop fires.
///
/// Like `batch_lazy()`, this operator does NOT cause the loop to fire. But unlike `batch_lazy()`
/// — which drops its pending data at the end of any tick where the loop does not fire —
/// `snapshot()` holds pending values in a buffer that persists across ticks, and delivers all of
/// them at the loop's next firing. Nothing is ever dropped.
///
/// This makes `snapshot()` the right ingress for *state observation* feeds (e.g. a stream of
/// updates to a value being snapshotted by the loop), where the producer emits only when the
/// state *changes*: a dropped update would leave the observer permanently stale, so updates must
/// be retained until the loop next fires.
///
/// `snapshot()` is one of four loop-ingress ("windowing") operators, which differ in whether they
/// cause the surrounding `loop { ... }` to fire and whether pending data is retained:
/// - `batch()` triggers the loop only when its windowed input is non-empty.
/// - `batch_lazy()` never triggers the loop on its own; its data is only observed if the loop
///   fires for some other reason (otherwise dropped at tick end).
/// - `batch_eager()` always triggers the loop, even when the windowed input is empty.
/// - `snapshot()` never triggers the loop on its own, but retains pending data across ticks and
///   delivers it at the loop's next firing (nothing is dropped).
///
/// Memory note: because pending data is retained until the loop fires, a producer that re-emits
/// every tick (rather than only on change) will grow the retained buffer unboundedly while the
/// loop does not fire. `snapshot()` is intended for emit-on-change feeds.
///
/// `snapshot()` is only valid at the entry of a root-level loop: nested loops iterate to fixpoint
/// within a single tick, so cross-tick retention does not apply to them.
///
/// ```dfir
/// trigger = source_iter([(), ()]);
/// updates = source_iter([1, 2, 3]);
/// loop {
///     trigger -> batch() -> for_each(std::mem::drop);
///     updates -> snapshot() -> for_each(|x| println!("{}", x));
/// };
/// ```
pub const SNAPSHOT: OperatorConstraints = OperatorConstraints {
    name: "snapshot",
    categories: &[OperatorCategory::Windowing],
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 0,
    persistence_args: RANGE_0,
    type_args: RANGE_0,
    is_external_input: false,
    flo_type: Some(FloType::WindowingRetain),
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| None,
    // Same as batch() — identity in inline codegen. The retention behavior comes from the
    // handoff buffer feeding this operator being declared outside the tick closure (a persistent
    // `std::vec::Vec` rather than a tick-local bump-allocated vec), so un-consumed data survives
    // ticks where the loop does not fire. See `MetaGraph::as_code_with_options`.
    write_fn: |wc @ &WriteContextArgs { .. }, _diagnostics| {
        let write_iterator = identity_write_iterator_fn(wc);
        Ok(OperatorWriteOutput {
            write_iterator,
            ..Default::default()
        })
    },
};
