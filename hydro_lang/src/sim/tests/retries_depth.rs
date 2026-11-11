//! Autonomous exploration of unhooked at-least-once operators (`SimFlow::retries_depth`).

use std::collections::BTreeSet;

use stageleft::q;

use crate::live_collections::sliced::sliced;
use crate::live_collections::stream::{AtLeastOnce, ExactlyOnce, NoOrder, TotalOrder};
use crate::location::Location;
use crate::nondet::nondet;
use crate::prelude::FlowBuilder;

fn covers(input: &[u32], output: &[u32]) -> bool {
    input.iter().all(|v| output.contains(v)) && output.iter().all(|v| input.contains(v))
}

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

#[test]
fn retries_depth_zero_generates_no_retries() {
    let observed = explore_top_level_unordered_retries(0);
    assert_eq!(observed, BTreeSet::from([vec![1, 2]]));
}

/// The budget is shared by all elements.
#[test]
fn retries_depth_top_level_unordered_retries_spends_run_budget() {
    let observed = explore_top_level_unordered_retries(1);
    assert_eq!(
        observed,
        BTreeSet::from([vec![1, 2], vec![1, 1, 2], vec![1, 2, 2]])
    );
}

#[test]
fn retries_depth_top_level_unordered_retries_depth_two() {
    let observed = explore_top_level_unordered_retries(2);
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
    flow.sim().retries_depth(1).exhaustive(async || {
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

/// `assume_ordering` only re-emits an element after another one; adjacent copies come
/// from the downstream `assume_retries`.
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
    flow.sim().retries_depth(1).exhaustive(async || {
        in_send.send_many_unordered([1, 2]);
        let all: Vec<u32> = output.collect().await;
        assert!(
            covers(&[1, 2], &all),
            "lost or invented an element: {all:?}"
        );
        observed.insert(all);
    });

    let mut expected = BTreeSet::new();
    for order in [vec![1, 2], vec![2, 1], vec![1, 2, 1], vec![2, 1, 2]] {
        expected.insert(order.clone());
        for repeated in 0..order.len() {
            let mut with_repeat = order.clone();
            with_repeat.insert(repeated, order[repeated]);
            expected.insert(with_repeat);
        }
    }
    assert_eq!(observed, expected);
}

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

/// Inside a tick, the budget is reset on every tick execution.
#[test]
fn retries_depth_in_tick_retries_budget_is_per_tick() {
    let observed = explore_in_tick_ordered_retries(1);
    let mut expected = BTreeSet::new();
    // a single batch
    for batch in [vec![1, 2], vec![1, 1, 2], vec![1, 2, 2]] {
        expected.insert(vec![batch]);
    }
    // one batch per element, each with its own budget
    for first in [vec![1], vec![1, 1]] {
        for second in [vec![2], vec![2, 2]] {
            expected.insert(vec![first.clone(), second]);
        }
    }
    assert_eq!(observed, expected);
}

#[test]
fn retries_depth_in_tick_alo_ordering_re_emits() {
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
    flow.sim().retries_depth(1).exhaustive(async || {
        in_send.send_many_unordered([1, 2]);
        let ticks: Vec<Vec<u32>> = output.collect().await;
        let all: Vec<u32> = ticks.iter().flatten().copied().collect();
        assert!(
            covers(&[1, 2], &all),
            "lost or invented an element: {all:?}"
        );
        observed_batches.extend(ticks);
    });

    // every order with at most one re-emission, then at most one adjacent copy
    let mut expected = BTreeSet::new();
    for order in [
        vec![1],
        vec![2],
        vec![1, 2],
        vec![2, 1],
        vec![1, 2, 1],
        vec![2, 1, 2],
    ] {
        expected.insert(order.clone());
        for repeated in 0..order.len() {
            let mut with_repeat = order.clone();
            with_repeat.insert(repeated, order[repeated]);
            expected.insert(with_repeat);
        }
    }
    assert_eq!(observed_batches, expected);
}

/// A kept element is redelivered at the start of the next batch. The fold collapses
/// adjacent repeats, so the in-tick retries it observes don't show up.
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
    flow.sim().retries_depth(1).exhaustive(async || {
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
    flow.sim().retries_depth(1).exhaustive(async || {
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

#[test]
#[should_panic(
    expected = "deterministic simulation requires every at-least-once operator to be bound to a sim hook"
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

    flow.sim().retries_depth(0).deterministic(async || {});
}

/// Without `retries_depth`, `fuzz` is unbounded. Reaching 32 copies takes 31 heads in a row
/// (probability `2^-31`), which random inputs essentially never produce, but coverage-guided
/// fuzzing finds quickly.
///
/// Run as `PATH="$PATH:." cargo sim -p hydro_lang --features sim -- fuzz_reaches_bug_requiring_many_retries`.
#[cfg(not(nightly))]
#[test]
#[should_panic(expected = "bug: an element was delivered 32 or more times")]
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
            all.len() < 32,
            "bug: an element was delivered 32 or more times ({} deliveries)",
            all.len()
        );
    });
}

/// The fold accumulates a set, so the in-tick retries it observes don't show up.
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
    flow.sim().retries_depth(1).exhaustive(async || {
        // values increase within each key, so sorting preserves per-key order
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
    flow.sim().retries_depth(1).exhaustive(async || {
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

/// An element can go around a feedback cycle and come back before the rest of the batch it
/// arrived with is observed.
#[test]
fn retries_depth_top_level_unordered_retries_cycle_back() {
    use crate::live_collections::stream::Stream;
    use crate::location::Process;
    use crate::networking::TCP;
    use crate::prelude::Unbounded;

    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let node2 = flow.process::<()>();
    let (in_send, input) = node.sim_input::<u32, NoOrder, ExactlyOnce>();

    let (complete_cycle_back, cycle_back) =
        node.forward_ref::<Stream<u32, Process<'_>, Unbounded, NoOrder, AtLeastOnce>>();
    let retried = input
        .weaken_retries::<AtLeastOnce>()
        .merge_unordered(cycle_back)
        .assume_retries::<ExactlyOnce>(nondet!(/** fuzzed */));
    complete_cycle_back.complete(
        retried
            .clone()
            .map(q!(|v| v + 1))
            .filter(q!(|v| v % 2 == 1))
            .send(&node2, TCP.fail_stop().bincode())
            .send(&node, TCP.fail_stop().bincode())
            .weaken_retries::<AtLeastOnce>(),
    );
    let output = retried
        .assume_ordering::<TotalOrder>(nondet!(/** fuzzed */))
        .sim_output();

    let mut saw = false;
    flow.sim().retries_depth(0).exhaustive(async || {
        in_send.send_many_unordered([0, 2]);
        let all: Vec<u32> = output.collect().await;
        if all.starts_with(&[0, 1, 2]) {
            saw = true;
        }
    });
    assert!(saw, "did not see an instance with 0, 1, 2 in order");
}
