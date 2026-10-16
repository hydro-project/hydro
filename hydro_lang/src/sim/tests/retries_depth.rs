//! Tests for autonomous exploration of at-least-once non-determinism under a retry budget
//! (`SimFlow::retries_depth`): unhooked `assume_retries`, and `assume_ordering` / `batch`
//! on `AtLeastOnce` streams, explored by `exhaustive` with at most `depth - 1` generated
//! retries per operator (over the run for top-level operators, per tick execution for
//! in-tick operators).

use std::collections::BTreeSet;

use stageleft::q;

use crate::live_collections::sliced::sliced;
use crate::live_collections::stream::{AtLeastOnce, ExactlyOnce, NoOrder, TotalOrder};
use crate::location::Location;
use crate::nondet::nondet;
use crate::prelude::FlowBuilder;

/// Whether `output` delivers every value of `input` at least once and nothing else.
fn covers(input: &[u32], output: &[u32]) -> bool {
    input.iter().all(|v| output.contains(v)) && output.iter().all(|v| input.contains(v))
}

/// The set of multisets (as sorted vectors) explored by an unordered top-level
/// `assume_retries` under the given depth.
fn explore_top_level_unordered_retries(depth: usize) -> BTreeSet<Vec<u32>> {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let (in_send, input) = node.sim_input::<u32, NoOrder, ExactlyOnce>();
    let output = input
        .weaken_retries::<AtLeastOnce>()
        .assume_retries::<ExactlyOnce>(nondet!(/** fuzzed */))
        .sim_output();

    let mut observed = BTreeSet::new();
    flow.sim().retries_depth(depth).exhaustive(async || {
        in_send.send_many_unordered([1, 2]);
        let all: Vec<u32> = output.collect_sorted().await;
        assert!(
            covers(&[1, 2], &all),
            "lost or invented an element: {all:?}"
        );
        observed.insert(all);
    });
    observed
}

/// `retries_depth(1)` generates no retries: an unhooked `assume_retries` is a pass-through.
#[test]
fn retries_depth_one_generates_no_retries() {
    let observed = explore_top_level_unordered_retries(1);
    assert_eq!(observed, BTreeSet::from([vec![1, 2]]));
}

/// A top-level unordered `assume_retries` with one unit of budget duplicates at most one
/// element, once: the budget is spent over the whole run, not per element.
#[test]
fn retries_depth_top_level_unordered_retries_spends_run_budget() {
    let observed = explore_top_level_unordered_retries(2);
    assert_eq!(
        observed,
        BTreeSet::from([vec![1, 2], vec![1, 1, 2], vec![1, 2, 2]])
    );
}

/// With two units of budget, the extra copies may go to one element (as two retries) or
/// be split across both.
#[test]
fn retries_depth_top_level_unordered_retries_depth_three() {
    let observed = explore_top_level_unordered_retries(3);
    assert_eq!(
        observed,
        BTreeSet::from([
            vec![1, 2],
            vec![1, 1, 2],
            vec![1, 2, 2],
            vec![1, 1, 1, 2],
            vec![1, 2, 2, 2],
            vec![1, 1, 2, 2],
        ])
    );
}

/// A top-level ordered `assume_retries` repeats an element only back-to-back (a delayed
/// redelivery would be a different collection), and spends its budget over the run.
#[test]
fn retries_depth_top_level_ordered_retries_repeats_in_place() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let (in_send, input) = node.sim_input::<u32, TotalOrder, ExactlyOnce>();
    let output = input
        .weaken_retries::<AtLeastOnce>()
        .assume_retries::<ExactlyOnce>(nondet!(/** fuzzed */))
        .sim_output();

    let mut observed = BTreeSet::new();
    flow.sim().retries_depth(2).exhaustive(async || {
        in_send.send_many([1, 2, 3]);
        let all: Vec<u32> = output.collect().await;
        observed.insert(all);
    });
    assert_eq!(
        observed,
        BTreeSet::from([
            vec![1, 2, 3],
            vec![1, 1, 2, 3],
            vec![1, 2, 2, 3],
            vec![1, 2, 3, 3],
        ])
    );
}

/// A top-level `assume_ordering` on an at-least-once stream may emit an element into a
/// second, non-adjacent slot (a delayed redelivery) while budget remains, never into
/// adjacent slots (that is the downstream `assume_retries`'s dimension). With one unit of
/// budget at each of the two points, the slot sequences are `12`, `21`, `121`, `212`, and
/// the retries point then repeats at most one slot in place.
#[test]
fn retries_depth_top_level_alo_ordering_emits_delayed_redelivery() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let (in_send, input) = node.sim_input::<u32, NoOrder, ExactlyOnce>();
    let output = input
        .weaken_retries::<AtLeastOnce>()
        .assume_ordering::<TotalOrder>(nondet!(/** fuzzed */))
        .assume_retries::<ExactlyOnce>(nondet!(/** fuzzed */))
        .sim_output();

    let mut observed = BTreeSet::new();
    flow.sim().retries_depth(2).exhaustive(async || {
        in_send.send_many_unordered([1, 2]);
        let all: Vec<u32> = output.collect().await;
        assert!(
            covers(&[1, 2], &all),
            "lost or invented an element: {all:?}"
        );
        observed.insert(all);
    });

    let mut expected = BTreeSet::new();
    for slots in [vec![1, 2], vec![2, 1], vec![1, 2, 1], vec![2, 1, 2]] {
        expected.insert(slots.clone());
        for repeated in 0..slots.len() {
            let mut with_repeat = slots.clone();
            with_repeat.insert(repeated, slots[repeated]);
            expected.insert(with_repeat);
        }
    }
    assert_eq!(observed, expected);
}

/// Explores an in-tick `assume_retries` over fuzzed batches of `[1, 2]`, returning the
/// per-tick outputs of every execution.
fn explore_in_tick_ordered_retries(depth: usize) -> BTreeSet<Vec<Vec<u32>>> {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let (in_send, input) = node.sim_input::<u32, TotalOrder, ExactlyOnce>();
    let output = sliced! {
        let b = use::batch(input, nondet!(/** fuzzed */));
        b.weaken_retries::<AtLeastOnce>()
            .assume_retries::<ExactlyOnce>(nondet!(/** fuzzed */))
            .fold(q!(|| Vec::new()), q!(|acc, v| acc.push(v)))
            .into_stream()
    }
    .sim_output();

    let mut observed = BTreeSet::new();
    flow.sim().retries_depth(depth).exhaustive(async || {
        in_send.send_many([1, 2]);
        let ticks: Vec<Vec<u32>> = output.collect().await;
        observed.insert(ticks);
    });
    observed
}

/// An in-tick `assume_retries` gets a fresh budget on every tick execution: with one unit,
/// each batch may repeat one of its elements in place, independently of other batches.
#[test]
fn retries_depth_in_tick_retries_budget_is_per_tick() {
    let observed = explore_in_tick_ordered_retries(2);
    let mut expected = BTreeSet::new();
    // One batch holding both elements, with at most one in-place repeat...
    for batch in [vec![1, 2], vec![1, 1, 2], vec![1, 2, 2]] {
        expected.insert(vec![batch]);
    }
    // ...or one batch per element, each independently repeated or not.
    for first in [vec![1], vec![1, 1]] {
        for second in [vec![2], vec![2, 2]] {
            expected.insert(vec![first.clone(), second]);
        }
    }
    assert_eq!(observed, expected);
}

/// An in-tick `assume_ordering` on an at-least-once stream may give an element a second,
/// non-adjacent slot within the tick. Adjacent repeats are the downstream in-tick
/// `assume_retries`'s dimension (here also fuzzed, with its own per-tick budget).
#[test]
fn retries_depth_in_tick_alo_ordering_emits_extra_slots() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let (in_send, input) = node.sim_input::<u32, NoOrder, ExactlyOnce>();
    let output = sliced! {
        let b = use::batch(input, nondet!(/** fuzzed */));
        b.weaken_retries::<AtLeastOnce>()
            .assume_ordering::<TotalOrder>(nondet!(/** fuzzed */))
            .assume_retries::<ExactlyOnce>(nondet!(/** fuzzed */))
            .fold(q!(|| Vec::new()), q!(|acc, v| acc.push(v)))
            .into_stream()
    }
    .sim_output();

    let mut observed_batches = BTreeSet::new();
    flow.sim().retries_depth(2).exhaustive(async || {
        in_send.send_many_unordered([1, 2]);
        let ticks: Vec<Vec<u32>> = output.collect().await;
        let all: Vec<u32> = ticks.iter().flatten().copied().collect();
        assert!(
            covers(&[1, 2], &all),
            "lost or invented an element: {all:?}"
        );
        observed_batches.extend(ticks);
    });

    // A batch holding both elements: the slot sequences `12`, `21`, `121`, `212` (one
    // extra slot), each with at most one adjacent repeat; a batch holding one element can
    // only repeat it in place.
    let mut expected = BTreeSet::new();
    for slots in [
        vec![1],
        vec![2],
        vec![1, 2],
        vec![2, 1],
        vec![1, 2, 1],
        vec![2, 1, 2],
    ] {
        expected.insert(slots.clone());
        for repeated in 0..slots.len() {
            let mut with_repeat = slots.clone();
            with_repeat.insert(repeated, slots[repeated]);
            expected.insert(with_repeat);
        }
    }
    assert_eq!(observed_batches, expected);
}

/// Batching an ordered at-least-once stream may keep the last element of a batch for
/// redelivery: it is then the first element of the next non-empty batch. The budget is
/// spent over the run, so at most one such redelivery occurs. (The in-tick fold over the
/// still-at-least-once batch observes retries too, with its own budget; its accumulator
/// collapses adjacent repeats so only the batch boundaries are observed.)
#[test]
fn retries_depth_alo_batch_ordered_redelivers_last_in_next_batch() {
    use crate::properties::manual_proof;

    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let (in_send, input) = node.sim_input::<u32, TotalOrder, ExactlyOnce>();
    let output = sliced! {
        let b = use::batch(input.weaken_retries::<AtLeastOnce>(), nondet!(/** fuzzed */));
        b.fold(
            q!(|| Vec::new()),
            q!(
                |acc: &mut Vec<u32>, v| {
                    if acc.last() != Some(&v) {
                        acc.push(v);
                    }
                },
                idempotent = manual_proof!(/** adjacent repeats are collapsed */)
            ),
        )
        .into_stream()
    }
    .sim_output();

    let mut saw_redelivery = false;
    let mut saw_none = false;
    flow.sim().retries_depth(2).exhaustive(async || {
        in_send.send_many([1, 2, 3]);
        let ticks: Vec<Vec<u32>> = output.collect().await;
        let mut redeliveries = 0;
        let mut delivered = vec![];
        for batch in &ticks {
            assert!(
                !batch.is_empty(),
                "a tick ran with an empty batch: {ticks:?}"
            );
            let mut batch = batch.iter().copied();
            let first = batch.next().unwrap();
            if delivered.last() == Some(&first) {
                redeliveries += 1;
            } else {
                delivered.push(first);
            }
            delivered.extend(batch);
        }
        assert_eq!(delivered, [1, 2, 3], "redelivery out of place: {ticks:?}");
        assert!(redeliveries <= 1, "budget exceeded: {ticks:?}");
        if redeliveries == 1 {
            saw_redelivery = true;
        } else {
            saw_none = true;
        }
    });
    assert!(saw_redelivery, "never explored a cross-batch redelivery");
    assert!(saw_none, "never explored the retry-free execution");
}

/// Batching an unordered at-least-once stream may keep any released element for
/// redelivery in some later batch; the run-wide budget allows at most one.
#[test]
fn retries_depth_alo_batch_unordered_redelivers_in_later_batch() {
    use crate::properties::manual_proof;

    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let (in_send, input) = node.sim_input::<u32, NoOrder, ExactlyOnce>();
    let output = sliced! {
        let b = use::batch(input.weaken_retries::<AtLeastOnce>(), nondet!(/** fuzzed */));
        b.fold(
            q!(|| Vec::new()),
            q!(
                |acc: &mut Vec<u32>, v| {
                    if !acc.contains(&v) {
                        acc.push(v);
                        acc.sort();
                    }
                },
                commutative = manual_proof!(/** a sorted set */),
                idempotent = manual_proof!(/** a set */)
            ),
        )
        .into_stream()
    }
    .sim_output();

    let mut saw_redelivery = false;
    let mut saw_none = false;
    flow.sim().retries_depth(2).exhaustive(async || {
        in_send.send_many_unordered([1, 2, 3]);
        let ticks: Vec<Vec<u32>> = output.collect().await;
        let mut all: Vec<u32> = ticks.iter().flatten().copied().collect();
        all.sort();
        assert!(
            covers(&[1, 2, 3], &all),
            "lost or invented an element: {ticks:?}"
        );
        match all.len() {
            3 => saw_none = true,
            4 => saw_redelivery = true,
            _ => panic!("budget exceeded: {ticks:?}"),
        }
    });
    assert!(saw_redelivery, "never explored a cross-batch redelivery");
    assert!(saw_none, "never explored the retry-free execution");
}

/// A zero depth would permit losses, which at-least-once does not.
#[test]
#[should_panic(expected = "retries_depth must be at least 1")]
fn retries_depth_zero_panics() {
    let mut flow = FlowBuilder::new();
    let _node = flow.process::<()>();
    let _ = flow.sim().retries_depth(0);
}

/// Without a retries depth, `exhaustive` cannot enumerate the infinite decision space of
/// an unhooked at-least-once batch; the error points at both ways out.
#[test]
#[should_panic(expected = "call `.retries_depth(n)`")]
fn unhooked_alo_batch_panics_in_exhaustive() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let (_in_send, input) = node.sim_input::<u32, TotalOrder, AtLeastOnce>();
    let _output = sliced! {
        let b = use::batch(input, nondet!(/** deliberately unhooked */));
        b.first().into_stream()
    }
    .sim_output();

    flow.sim().exhaustive(async || {});
}

/// `deterministic` has no fuzzer to draw from, so a retries depth is meaningless there:
/// every at-least-once point must be scripted, and an unhooked one is rejected even with a
/// depth configured.
#[test]
#[should_panic(
    expected = "deterministic simulation requires every at-least-once operator (`assume_retries`, or `assume_ordering` / `batch` on an `AtLeastOnce` stream) to be bound to a sim hook"
)]
fn unhooked_alo_batch_panics_in_deterministic_even_with_depth() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let (_in_send, input) = node.sim_input::<u32, TotalOrder, AtLeastOnce>();
    let _output = sliced! {
        let b = use::batch(input, nondet!(/** deliberately unhooked */));
        b.first().into_stream()
    }
    .sim_output();

    flow.sim().retries_depth(1).deterministic(async || {});
}

/// Under `fuzz`, unhooked at-least-once points need no bound: retry counts are drawn from
/// the fuzzer without limit, so a bug that only manifests after *many* redeliveries of one
/// message is within reach. Here the "bug" needs a single element delivered 1000 or more
/// times. That is pathological for random inputs (the unbounded draw builds counts from
/// `u8` chunks where `255` means "and more", so 1000 needs four maxed chunks in a row, a
/// 1-in-16-million event per decision), but coverage-guided fuzzing is rewarded for every
/// extra chunk (each is a new hit-count bucket on the draw loop) and climbs there quickly.
/// Replayed under `cargo test` from the reproducer `cargo sim` found.
///
/// Run as `PATH="$PATH:." cargo sim -p hydro_lang --features sim -- fuzz_reaches_bug_requiring_many_retries`.
#[cfg(not(nightly))]
#[test]
#[should_panic(expected = "bug: an element was delivered 1000 or more times")]
#[cfg_attr(not(target_os = "linux"), ignore = "sim reproducer is Linux-only")]
fn fuzz_reaches_bug_requiring_many_retries() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let (in_send, input) = node.sim_input::<u32, TotalOrder, ExactlyOnce>();
    let output = input
        .weaken_retries::<AtLeastOnce>()
        .assume_retries::<ExactlyOnce>(nondet!(/** fuzzed without a bound */))
        .sim_output();

    flow.sim().fuzz(async || {
        in_send.send_many([1]);
        let all: Vec<u32> = output.collect().await;
        assert!(
            all.len() < 1000,
            "bug: an element was delivered 1000 or more times ({} deliveries)",
            all.len()
        );
    });
}

/// Batching a keyed stream with ordered at-least-once values may keep a key's trailing
/// value for redelivery: it is then that key's first value in the next batch the key
/// contributes to. The run-wide budget allows at most one such redelivery. (The in-tick
/// fold accumulates a set, so the in-tick retries it observes are invisible.)
#[test]
fn retries_depth_alo_keyed_batch_ordered_redelivers_last_in_next_batch() {
    use std::collections::BTreeMap;

    use crate::properties::manual_proof;

    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let (in_send, input) = node.sim_input::<(u32, u32), TotalOrder, ExactlyOnce>();
    let output = sliced! {
        let b = use::batch(
            input.into_keyed().weaken_retries::<AtLeastOnce>(),
            nondet!(/** fuzzed */)
        );
        b.entries()
            .fold(
                q!(|| Vec::new()),
                q!(
                    |acc: &mut Vec<(u32, u32)>, v| {
                        if !acc.contains(&v) {
                            acc.push(v);
                            acc.sort();
                        }
                    },
                    commutative = manual_proof!(/** a sorted set */),
                    idempotent = manual_proof!(/** a set */)
                ),
            )
            .into_stream()
    }
    .sim_output();

    let mut saw_redelivery = false;
    let mut saw_none = false;
    flow.sim().retries_depth(2).exhaustive(async || {
        // Values increase within each key, so a batch's sorted entries list each key's
        // values in arrival order.
        in_send.send_many([(1, 10), (1, 11), (2, 20)]);
        let ticks: Vec<Vec<(u32, u32)>> = output.collect().await;
        let mut redeliveries = 0;
        let mut delivered: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
        for batch in &ticks {
            assert!(
                !batch.is_empty(),
                "a tick ran with an empty batch: {ticks:?}"
            );
            let mut per_key: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
            for (key, value) in batch {
                per_key.entry(*key).or_default().push(*value);
            }
            for (key, values) in per_key {
                let delivered_key = delivered.entry(key).or_default();
                let mut values = values.into_iter();
                let first = values.next().unwrap();
                if delivered_key.last() == Some(&first) {
                    redeliveries += 1;
                } else {
                    delivered_key.push(first);
                }
                delivered_key.extend(values);
            }
        }
        assert_eq!(
            delivered,
            BTreeMap::from([(1, vec![10, 11]), (2, vec![20])]),
            "redelivery out of place: {ticks:?}"
        );
        assert!(redeliveries <= 1, "budget exceeded: {ticks:?}");
        if redeliveries == 1 {
            saw_redelivery = true;
        } else {
            saw_none = true;
        }
    });
    assert!(saw_redelivery, "never explored a cross-batch redelivery");
    assert!(saw_none, "never explored the retry-free execution");
}

/// Batching a keyed stream with unordered at-least-once values may keep any released
/// entry for redelivery in some later batch; the run-wide budget allows at most one.
#[test]
fn retries_depth_alo_keyed_batch_unordered_redelivers_in_later_batch() {
    use crate::properties::manual_proof;

    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let (in_send, input) = node.sim_input::<(u32, u32), NoOrder, ExactlyOnce>();
    let output = sliced! {
        let b = use::batch(
            input.into_keyed().weaken_retries::<AtLeastOnce>(),
            nondet!(/** fuzzed */)
        );
        b.entries()
            .fold(
                q!(|| Vec::new()),
                q!(
                    |acc: &mut Vec<(u32, u32)>, v| {
                        if !acc.contains(&v) {
                            acc.push(v);
                            acc.sort();
                        }
                    },
                    commutative = manual_proof!(/** a sorted set */),
                    idempotent = manual_proof!(/** a set */)
                ),
            )
            .into_stream()
    }
    .sim_output();

    let mut saw_redelivery = false;
    let mut saw_none = false;
    flow.sim().retries_depth(2).exhaustive(async || {
        in_send.send_many_unordered([(1, 10), (1, 11), (2, 20)]);
        let ticks: Vec<Vec<(u32, u32)>> = output.collect().await;
        let mut all: Vec<(u32, u32)> = ticks.iter().flatten().copied().collect();
        all.sort();
        for entry in [(1, 10), (1, 11), (2, 20)] {
            assert!(all.contains(&entry), "lost an entry: {ticks:?}");
        }
        match all.len() {
            3 => saw_none = true,
            4 => saw_redelivery = true,
            _ => panic!("budget exceeded: {ticks:?}"),
        }
    });
    assert!(saw_redelivery, "never explored a cross-batch redelivery");
    assert!(saw_none, "never explored the retry-free execution");
}
