use super::{
    FloType, OperatorCategory, OperatorConstraints, OperatorWriteOutput, RANGE_0, RANGE_1,
    WriteContextArgs, identity_write_iterator_fn,
};

/// > Input: `(K, Option<V>)` updates. Output: `(K, V)` entries.
///
/// The keyed counterpart of `snapshot()`: retains the **latest** value *per key* across ticks
/// and emits every current entry (as clones) each time the loop fires, without causing the loop
/// to fire.
///
/// Each input item is an update to one key: `(k, Some(v))` sets `k`'s current value to `v`
/// (replacing any previous value), and `(k, None)` removes `k`. The resulting map is a persistent
/// view held across ticks: it is **not consumed** when the loop fires, so if the loop fires again
/// without any new updates, the same entries are observed again. Keys and values must therefore
/// implement `Clone` (and keys `Eq + Hash`). Before any update has arrived, `snapshot_keyed()`
/// emits nothing.
///
/// Use `snapshot_keyed()` to observe keyed state (e.g. a keyed aggregation) from inside a loop.
/// A plain `snapshot()` holds only a single item, so it would retain just the most recent entry
/// of a keyed feed. For feeds without removals, map each entry to an upsert first:
/// `-> map(|(k, v)| (k, Some(v))) -> snapshot_keyed()`.
///
/// Like `snapshot()`, `snapshot_keyed()` is only valid at the entry of a root-level loop: nested
/// loops iterate to fixpoint within a single tick, so cross-tick retention does not apply to them.
///
/// Memory note: the retained map holds at most one value per (non-removed) key.
///
/// ```dfir
/// trigger = source_iter([()]);
/// updates = source_iter([("a", Some(1)), ("b", Some(2)), ("a", Some(3)), ("b", None), ("c", Some(4))]);
/// loop {
///     trigger -> batch() -> for_each(std::mem::drop);
///     // Observes the latest value per key, with `"b"` removed: `("a", 3)` and `("c", 4)`.
///     updates -> snapshot_keyed() -> for_each(|(k, v)| println!("{}: {}", k, v));
/// };
/// ```
pub const SNAPSHOT_KEYED: OperatorConstraints = OperatorConstraints {
    name: "snapshot_keyed",
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
    // keyed map (rather than a tick-local bump-allocated vec) that applies `(K, Option<V>)`
    // updates, and that the loop reads (cloning every entry, not consuming the map) each time it
    // fires. See `MetaGraph::as_code_with_options`.
    write_fn: |wc @ &WriteContextArgs { .. }, _diagnostics| {
        let write_iterator = identity_write_iterator_fn(wc);
        Ok(OperatorWriteOutput {
            write_iterator,
            ..Default::default()
        })
    },
};
