use super::{
    FloType, OperatorCategory, OperatorConstraints, OperatorWriteOutput, RANGE_0, RANGE_1,
    WriteContextArgs, identity_write_iterator_fn,
};

/// Given an _unbounded_ input stream, retains **every** element received so far in a persistent
/// buffer across ticks, and emits the whole prefix (as clones, in arrival order) every time the
/// loop fires, without causing the loop to fire.
///
/// Like `batch_lazy()` and `snapshot()`, this operator does NOT cause the loop to fire. Unlike
/// `batch_lazy()` — which drops its pending data at the end of any tick where the loop does not
/// fire — nothing is ever dropped: each arriving element is appended to a buffer that persists
/// across ticks. And unlike `snapshot()` — which holds only the single latest value —
/// `snapshot_prefix()` keeps all of them.
///
/// The buffer is **not consumed** when the loop fires: `snapshot_prefix()` is a persistent view
/// of the stream's prefix up to now, so if the loop fires again (e.g. triggered by a `batch()`
/// input) without any new input, the same prefix is replayed. The item type must therefore
/// implement `Clone`. Before any element has arrived, `snapshot_prefix()` emits nothing.
///
/// This is the right ingress for replaying a *bounded* input (e.g. a `source_iter(..)` or a
/// bounded collection) into a loop that is driven by some other input: it is equivalent to
/// `persist::<'static>() -> batch_lazy()`, except that it does not need a `persist` at the top
/// level and does not depend on the replay arriving in the same tick that the loop fires.
///
/// Memory note: the retained buffer grows with the total number of elements received, and is
/// never truncated. `snapshot_prefix()` is intended for bounded inputs; for an unbounded
/// emit-on-change feed where only the latest value matters, use `snapshot()` (or
/// `snapshot_keyed()` for per-key state).
///
/// `snapshot_prefix()` is one of the loop-ingress ("windowing") operators, which differ in whether
/// they cause the surrounding `loop { ... }` to fire and whether pending data is retained:
/// - `batch()` triggers the loop only when its windowed input is non-empty.
/// - `batch_lazy()` never triggers the loop on its own; its data is only observed if the loop
///   fires for some other reason (otherwise dropped at tick end).
/// - `batch_eager()` always triggers the loop, even when the windowed input is empty.
/// - `snapshot()` never triggers the loop on its own, but retains the latest value
///   (last-write-wins) across ticks and emits it every time the loop fires.
/// - `snapshot_prefix()` never triggers the loop on its own, but retains every element received
///   so far across ticks and replays the whole prefix every time the loop fires.
///
/// `snapshot_prefix()` is only valid at the entry of a root-level loop: nested loops iterate to
/// fixpoint within a single tick, so cross-tick retention does not apply to them.
///
/// ```dfir
/// trigger = source_iter([(), ()]);
/// items = source_iter([1, 2, 3]);
/// loop {
///     trigger -> batch() -> for_each(std::mem::drop);
///     // Observes `1`, `2`, `3` (every element received so far, in order).
///     items -> snapshot_prefix() -> for_each(|x| println!("{}", x));
/// };
/// ```
pub const SNAPSHOT_PREFIX: OperatorConstraints = OperatorConstraints {
    name: "snapshot_prefix",
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
    // Identity in inline codegen, like `snapshot()`. The retention behavior comes from the
    // handoff feeding this operator being declared outside the tick closure as a persistent
    // `std::vec::Vec<T>` (rather than a tick-local bump-allocated vec) that producers append to,
    // and that the loop reads (cloning every element, not consuming the buffer) each time it
    // fires. See `MetaGraph::as_code_with_options`.
    write_fn: |wc @ &WriteContextArgs { .. }, _diagnostics| {
        let write_iterator = identity_write_iterator_fn(wc);
        Ok(OperatorWriteOutput {
            write_iterator,
            ..Default::default()
        })
    },
};
