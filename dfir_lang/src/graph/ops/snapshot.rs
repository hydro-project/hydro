use super::{
    FloType, OperatorCategory, OperatorConstraints, OperatorWriteOutput, RANGE_0, RANGE_1,
    WriteContextArgs, identity_write_iterator_fn,
};

/// Given an _unbounded_ input stream, retains the **latest** value across ticks (last-write-wins)
/// and emits it (as a clone) every time the loop fires, without causing the loop to fire.
///
/// Like `batch_lazy()`, this operator does NOT cause the loop to fire. But unlike `batch_lazy()`
/// — which drops its pending data at the end of any tick where the loop does not fire —
/// `snapshot()` holds the most recent value in a single cell that persists across ticks. Each
/// newly arriving value *replaces* (and drops) the previous one, so at most one value is retained
/// no matter how many ticks pass before the loop fires. Older values superseded by a newer one are
/// never observed.
///
/// The retained value is **not consumed** when the loop fires: `snapshot()` is a persistent view
/// of the latest value, so if the loop fires again (e.g. triggered by a `batch()` input) without
/// any new input to `snapshot()`, the same value is observed again. The item type must therefore
/// implement `Clone`. Before any value has arrived, `snapshot()` emits nothing.
///
/// This makes `snapshot()` the right ingress for *state observation* feeds (e.g. a stream of
/// the current value of some state being snapshotted by the loop): the observer only ever cares
/// about the latest value, and must never be left permanently stale by a dropped update.
///
/// `snapshot()` is one of the loop-ingress ("windowing") operators, which differ in whether they
/// cause the surrounding `loop { ... }` to fire and whether pending data is retained:
/// - `batch()` triggers the loop only when its windowed input is non-empty.
/// - `batch_lazy()` never triggers the loop on its own; its data is only observed if the loop
///   fires for some other reason (otherwise dropped at tick end).
/// - `batch_eager()` always triggers the loop, even when the windowed input is empty.
/// - `snapshot()` never triggers the loop on its own, but retains the latest value
///   (last-write-wins) across ticks and emits it every time the loop fires.
/// - `snapshot_prefix()` never triggers the loop on its own, but retains every element received
///   so far across ticks and replays the whole prefix every time the loop fires.
///
/// `snapshot()` holds a single item. To snapshot keyed state (one entry per item), use
/// `snapshot_keyed()`, which retains the latest value *per key*. To replay every element received
/// so far (e.g. a bounded input), use `snapshot_prefix()`.
///
/// Memory note: because only the single latest value is retained, memory use is bounded
/// regardless of how often the producer emits or how long the loop goes without firing.
///
/// `snapshot()` is only valid at the entry of a root-level loop: nested loops iterate to fixpoint
/// within a single tick, so cross-tick retention does not apply to them.
///
/// ```dfir
/// trigger = source_iter([(), ()]);
/// updates = source_iter([1, 2, 3]);
/// loop {
///     trigger -> batch() -> for_each(std::mem::drop);
///     // Only the latest value, `3`, is observed.
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
    // handoff feeding this operator being declared outside the tick closure as a single
    // last-write-wins `Option<T>` cell (rather than a tick-local bump-allocated vec), so the
    // latest value survives ticks where the loop does not fire, newer values replace (drop)
    // older ones, and the loop reads a clone (not consuming the cell) each time it fires.
    // See `MetaGraph::as_code_with_options`.
    write_fn: |wc @ &WriteContextArgs { .. }, _diagnostics| {
        let write_iterator = identity_write_iterator_fn(wc);
        Ok(OperatorWriteOutput {
            write_iterator,
            ..Default::default()
        })
    },
};
