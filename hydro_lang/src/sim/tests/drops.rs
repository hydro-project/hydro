//! Simulator examples for `MayDrop` streams, which may be missing elements, following the
//! claims of the denotational liveness design.
//!
//! - An ordered `MayDrop` stream is observed with `observe_drops(nondet!, nonlive!)`, where the
//!   simulator explores the drops: each element is either kept or dropped, so (unlike
//!   duplication) exhaustive simulation can enumerate every subsequence that survives.
//! - An unordered `MayDrop` stream, singleton, or optional is observed with
//!   `observe_drops(nonlive!)`. There is no order to observe, so a missing element cannot be told
//!   from one delayed until the end of the execution; the simulator delivers every element and
//!   requires `test_safety_only`, since it exercises safety but not liveness.

use std::collections::BTreeSet;
use std::sync::Mutex;

use stageleft::q;

use crate::live_collections::boundedness::Unbounded;
use crate::live_collections::stream::{AtLeastOnce, ExactlyOnce, MayDrop, Stream, TotalOrder};
use crate::liveness::nonlive;
use crate::location::{Location, Process};
use crate::nondet::nondet;
use crate::prelude::{FlowBuilder, manual_proof};

/// Exhaustive simulation observes every in-order subsequence of an ordered `MayDrop` stream:
/// each element is independently kept or dropped, and drops never reorder.
#[test]
fn sim_drops_explores_every_subsequence() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();

    let (in_send, input) = node.sim_input::<i32, TotalOrder, ExactlyOnce>();
    // Standing in for a stream received over a channel that may drop messages.
    let input = input.weaken_drops::<MayDrop>();
    let out_recv = input
        .observe_drops(
            nondet!(/** the test observes which elements arrive */),
            nonlive!(/** the test does not need every element to arrive */),
        )
        .sim_output();

    let observed = Mutex::new(BTreeSet::new());
    flow.sim().exhaustive(async || {
        in_send.send(1);
        in_send.send(2);
        in_send.send(3);
        let received: Vec<i32> = out_recv.collect().await;
        observed.lock().unwrap().insert(received);
    });

    let all_subsequences: BTreeSet<Vec<i32>> = [
        vec![],
        vec![1],
        vec![2],
        vec![3],
        vec![1, 2],
        vec![1, 3],
        vec![2, 3],
        vec![1, 2, 3],
    ]
    .into_iter()
    .collect();
    assert_eq!(observed.into_inner().unwrap(), all_subsequences);
}

/// The `nondet!` on the ordered `observe_drops` is needed: downstream of it the survivors are
/// taken as the whole stream, and a check that the sequence has no gap (which can never fail on
/// a `NoDrop` stream) finds the run where a middle element is missing.
#[test]
#[should_panic(expected = "gap in the sequence")]
fn sim_drops_sequence_check_finds_gap() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let tick = node.tick();

    let (in_send, input) = node.sim_input::<u32, TotalOrder, ExactlyOnce>();
    // Standing in for a stream received over a channel that may drop messages.
    let input = input.weaken_drops::<MayDrop>();
    // The sequence check: the next sequence number expected, and whether one was ever skipped.
    let check = input
        .observe_drops(
            nondet!(/** BUG: the sequence check assumes that no element is missing */),
            nonlive!(/** the test does not need every element to arrive */),
        )
        .fold(
            q!(|| (0u32, false)),
            q!(|(next, gap), seq| {
                *gap |= seq != *next;
                *next = seq + 1;
            }),
        );
    let out_recv = check
        .snapshot(
            &tick,
            nondet!(/** the test checks every intermediate state */),
        )
        .all_ticks()
        .sim_output();

    flow.sim().exhaustive(async || {
        in_send.send(0);
        in_send.send(1);
        in_send.send(2);
        for (_, gap) in out_recv.collect::<Vec<(u32, bool)>>().await {
            assert!(!gap, "gap in the sequence");
        }
    });
}

/// Element-wise operators apply to each element that arrives, before the drops are observed:
/// mapping a `MayDrop` stream and then observing its drops gives every mapped subsequence.
#[test]
fn sim_drops_element_wise_operators() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();

    let (in_send, input) = node.sim_input::<i32, TotalOrder, ExactlyOnce>();
    // Standing in for a stream received over a channel that may drop messages.
    let input = input.weaken_drops::<MayDrop>();
    let out_recv = input
        .map(q!(|x| x * 10))
        .filter(q!(|x| *x != 20))
        .observe_drops(
            nondet!(/** the test observes which elements arrive */),
            nonlive!(/** the test does not need every element to arrive */),
        )
        .sim_output();

    let observed = Mutex::new(BTreeSet::new());
    flow.sim().exhaustive(async || {
        in_send.send(1);
        in_send.send(2);
        in_send.send(3);
        let received: Vec<i32> = out_recv.collect().await;
        observed.lock().unwrap().insert(received);
    });

    let expected: BTreeSet<Vec<i32>> = [vec![], vec![10], vec![30], vec![10, 30]]
        .into_iter()
        .collect();
    assert_eq!(observed.into_inner().unwrap(), expected);
}

/// A join with a `MayDrop` input is `MayDrop`, and (against a bounded right-hand side) keeps the
/// order of its left input: the simulator observes every subsequence of the joined stream.
#[test]
fn sim_drops_through_join() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();

    let (in_send, input) = node.sim_input::<(i32, char), TotalOrder, ExactlyOnce>();
    // Standing in for a stream received over a channel that may drop messages.
    let input = input.weaken_drops::<MayDrop>();
    let names = node.source_iter(q!(vec![(1, "one"), (2, "two")]));
    let out_recv = input
        .join(names)
        .observe_drops(
            nondet!(/** the test observes which elements arrive */),
            nonlive!(/** the test does not need every element to arrive */),
        )
        .map(q!(|(k, (c, name))| (k, (c, name.to_owned()))))
        .sim_output();

    let observed = Mutex::new(BTreeSet::new());
    flow.sim().exhaustive(async || {
        in_send.send((1, 'a'));
        in_send.send((2, 'b'));
        let received: Vec<(i32, (char, String))> = out_recv.collect().await;
        observed.lock().unwrap().insert(received);
    });

    let one = (1, ('a', "one".to_owned()));
    let two = (2, ('b', "two".to_owned()));
    let expected: BTreeSet<Vec<(i32, (char, String))>> =
        [vec![], vec![one.clone()], vec![two.clone()], vec![one, two]]
            .into_iter()
            .collect();
    assert_eq!(observed.into_inner().unwrap(), expected);
}

/// The trajectory argument for commutative folds: whichever elements are missing, every value a
/// sum takes is on some trajectory of the sum over the whole stream (the one where the missing
/// elements arrive last), that is a prefix sum of some ordering of the ideal. With the elements
/// `1, 2, 4` the prefix sums of every ordering are exactly the subset sums `0..=7`.
#[test]
fn sim_drops_commutative_fold_stays_on_a_trajectory() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let tick = node.tick();

    let (in_send, input) = node.sim_input::<u32, TotalOrder, ExactlyOnce>();
    // Standing in for a stream received over a channel that may drop messages.
    let input = input.weaken_drops::<MayDrop>();
    let sum = input
        .observe_drops(
            nondet!(/** the test observes which elements arrive */),
            nonlive!(/** the test does not need every element to arrive */),
        )
        .fold(q!(|| 0u32), q!(|acc, x| *acc += x));
    let out_recv = sum
        .snapshot(
            &tick,
            nondet!(/** the test observes every intermediate sum */),
        )
        .all_ticks()
        .sim_output();

    let ideal_trajectories: BTreeSet<u32> = (0..=7).collect();
    let observed = Mutex::new(BTreeSet::new());
    flow.sim().exhaustive(async || {
        in_send.send(1);
        in_send.send(2);
        in_send.send(4);
        for value in out_recv.collect::<Vec<u32>>().await {
            assert!(ideal_trajectories.contains(&value));
            observed.lock().unwrap().insert(value);
        }
    });
    assert_eq!(observed.into_inner().unwrap(), ideal_trajectories);
}

/// A non-commutative fold has a single trajectory, and its value after a missing element is on
/// none of it: concatenating `a, b, c` only ever takes the values `"", "a", "ab", "abc"`, but
/// with `b` missing it reaches `"ac"`. This is why the type system refuses a non-commutative
/// fold over an ordered `MayDrop` stream until its drops are observed.
#[test]
fn sim_drops_non_commutative_fold_leaves_the_trajectory() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let tick = node.tick();

    let (in_send, input) = node.sim_input::<char, TotalOrder, ExactlyOnce>();
    // Standing in for a stream received over a channel that may drop messages.
    let input = input.weaken_drops::<MayDrop>();
    let concatenated = input
        .observe_drops(
            nondet!(/** the test observes which elements arrive */),
            nonlive!(/** the test does not need every element to arrive */),
        )
        .fold(q!(|| String::new()), q!(|acc, c| acc.push(c)));
    let out_recv = concatenated
        .snapshot(
            &tick,
            nondet!(/** the test observes every intermediate value */),
        )
        .all_ticks()
        .sim_output();

    let trajectory: BTreeSet<String> = ["", "a", "ab", "abc"]
        .into_iter()
        .map(String::from)
        .collect();
    let off_trajectory = Mutex::new(BTreeSet::new());
    flow.sim().exhaustive(async || {
        in_send.send('a');
        in_send.send('b');
        in_send.send('c');
        for value in out_recv.collect::<Vec<String>>().await {
            if !trajectory.contains(&value) {
                off_trajectory.lock().unwrap().insert(value);
            }
        }
    });
    assert!(off_trajectory.into_inner().unwrap().contains("ac"));
}

/// A commutative fold or `count` is available on an ordered `MayDrop` stream, and produces a
/// `MayDrop` singleton, which is observed with only a `nonlive!`. With no order to observe, the
/// simulator cannot tell a missing element from a late one there, so it delivers all of them.
#[test]
fn sim_drops_aggregations_produce_may_drop_values() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let tick = node.tick();

    let (in_send, input) = node.sim_input::<u32, TotalOrder, ExactlyOnce>();
    // Standing in for a stream received over a channel that may drop messages.
    let input = input.weaken_drops::<MayDrop>();

    let count = input
        .clone()
        .count()
        .observe_drops(nonlive!(/** the test delivers every element */));
    let sum = input
        .fold(
            q!(|| 0u32),
            q!(
                |acc, x| *acc += x,
                commutative = manual_proof!(/** addition is commutative */)
            ),
        )
        .map(q!(|sum| sum * 10))
        .observe_drops(nonlive!(/** the test delivers every element */));

    let count_recv = count
        .snapshot(&tick, nondet!(/** the test reads the final count */))
        .all_ticks()
        .sim_output();
    let sum_recv = sum
        .snapshot(&tick, nondet!(/** the test reads the final sum */))
        .all_ticks()
        .sim_output();

    flow.sim().test_safety_only().exhaustive(async || {
        in_send.send(3);
        in_send.send(1);
        in_send.send(2);
        assert_eq!(count_recv.collect::<Vec<_>>().await.last(), Some(&3));
        assert_eq!(sum_recv.collect::<Vec<_>>().await.last(), Some(&60));
    });
}

/// `max` and `last` are available on an ordered `MayDrop` stream too, and produce `MayDrop`
/// optionals. The simulator does not yet support an unbounded top-level `reduce`, so they are only
/// type-checked here.
#[expect(dead_code, reason = "only type-checked")]
fn may_drop_max_and_last<'a>(
    input: Stream<u32, Process<'a>, Unbounded, TotalOrder, ExactlyOnce, MayDrop>,
) {
    let _max = input
        .clone()
        .max()
        .observe_drops(nonlive!(/** the max of the elements that arrive is enough */));
    let _last = input
        .last()
        .observe_drops(nonlive!(/** the last element that arrives is enough */));
}

/// `observe_drops(nonlive!)` only answers for liveness, which the simulator does not test, so it
/// requires the simulation to opt in with `test_safety_only`.
#[test]
#[should_panic(expected = "requires `.test_safety_only()`")]
fn sim_drops_unordered_observe_requires_test_safety_only() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let tick = node.tick();

    let (in_send, input) = node.sim_input::<u32, TotalOrder, ExactlyOnce>();
    // Standing in for a stream received over a channel that may drop messages.
    let count = input
        .weaken_drops::<MayDrop>()
        .count()
        .observe_drops(nonlive!(/** the test delivers every element */));
    let out_recv = count
        .snapshot(&tick, nondet!(/** the test reads the count */))
        .all_ticks()
        .sim_output();

    flow.sim().exhaustive(async || {
        in_send.send(1);
        let _ = out_recv.collect::<Vec<_>>().await;
    });
}

/// `delayed_forever` forgets the order of an ordered `MayDrop` stream with no guard. Once the
/// order is gone, observing the drops needs only a `nonlive!`, and the simulator delivers every
/// element, held to the obligations of an unordered stream (here, any arrival order).
#[test]
fn sim_drops_delayed_forever_delivers_every_element() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();

    let (in_send, input) = node.sim_input::<i32, TotalOrder, ExactlyOnce>();
    let (other_send, other) = node.sim_input::<i32, TotalOrder, ExactlyOnce>();
    // Standing in for a stream received over a channel that may drop messages.
    let out_recv = input
        .weaken_drops::<MayDrop>()
        .delayed_forever()
        .merge_unordered(other)
        .observe_drops(nonlive!(/** the test delivers every element */))
        .assume_ordering::<TotalOrder>(nondet!(/** the test compares sets */))
        .sim_output();

    let orders = Mutex::new(BTreeSet::new());
    flow.sim().test_safety_only().exhaustive(async || {
        in_send.send(1);
        in_send.send(2);
        other_send.send(10);
        let received: Vec<i32> = out_recv.collect().await;
        assert_eq!(
            received.iter().copied().collect::<BTreeSet<_>>(),
            [1, 2, 10].into_iter().collect()
        );
        orders.lock().unwrap().insert(received);
    });
    // Every arrival order of the unordered stream is explored.
    assert_eq!(orders.into_inner().unwrap().len(), 6);
}

/// A sum over a `MayDrop` stream is only a lower bound: the simulator finds the run where a
/// deposit is missing and the balance is wrong.
#[test]
#[should_panic(expected = "a deposit was lost")]
fn sim_drops_finds_missing_deposit() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let tick = node.tick();

    let (in_send, deposits) = node.sim_input::<i32, TotalOrder, ExactlyOnce>();
    // Standing in for deposits received over a channel that may drop messages.
    let deposits = deposits.weaken_drops::<MayDrop>();
    let batch_totals = deposits
        .observe_drops(
            nondet!(/** BUG: the balance assumes no deposit is ever lost */),
            nonlive!(/** BUG: nothing retransmits a lost deposit */),
        )
        .batch(
            &tick,
            nondet!(/** the test sums the totals of every batch */),
        )
        .fold(q!(|| 0), q!(|acc, amount| *acc += amount))
        .all_ticks()
        .sim_output();

    flow.sim().exhaustive(async || {
        in_send.send(10);
        in_send.send(20);
        let totals: Vec<i32> = batch_totals.collect().await;
        assert_eq!(totals.into_iter().sum::<i32>(), 30, "a deposit was lost");
    });
}

/// Drops and retries are independent: a stream that may both repeat and miss elements is
/// `AtLeastOnce` and `MayDrop`. `unique` makes it exactly-once, and observing its drops explores
/// every set of elements that survive.
#[test]
fn sim_drops_with_retries() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();

    let (in_send, input) = node.sim_input::<i32, TotalOrder, ExactlyOnce>();
    // Standing in for a channel that may both resend and drop messages.
    let input = input
        .weaken_retries::<AtLeastOnce>()
        .weaken_drops::<MayDrop>();
    let seen = input
        .unique()
        .observe_drops(
            nondet!(/** the test observes which elements arrive */),
            nonlive!(/** the test does not need every element to arrive */),
        )
        .sim_output();

    let observed = Mutex::new(BTreeSet::new());
    flow.sim().exhaustive(async || {
        in_send.send(1);
        in_send.send(2);
        let received: Vec<i32> = seen.collect().await;
        observed.lock().unwrap().insert(received);
    });

    let all_subsequences: BTreeSet<Vec<i32>> =
        [vec![], vec![1], vec![2], vec![1, 2]].into_iter().collect();
    assert_eq!(observed.into_inner().unwrap(), all_subsequences);
}

/// The values of a `MayDrop` keyed stream with ordered values are kept or dropped
/// independently, so exhaustive simulation observes every subset of the entries.
#[test]
fn sim_keyed_drops_explores_every_subset() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();

    let (in_send, input) = node.sim_input::<(i32, char), TotalOrder, ExactlyOnce>();
    // Standing in for entries received over a channel that may drop messages.
    let keyed = input.weaken_drops::<MayDrop>().into_keyed();
    let out_recv = keyed
        .map(q!(|c| c.to_ascii_uppercase()))
        .observe_drops(
            nondet!(/** the test observes which values arrive */),
            nonlive!(/** the test does not need every value to arrive */),
        )
        .entries()
        .assume_ordering::<TotalOrder>(nondet!(/** the test compares sets */))
        .sim_output();

    let observed = Mutex::new(BTreeSet::new());
    flow.sim().exhaustive(async || {
        in_send.send((1, 'a'));
        in_send.send((1, 'b'));
        in_send.send((2, 'c'));
        let received: Vec<(i32, char)> = out_recv.collect().await;
        observed
            .lock()
            .unwrap()
            .insert(received.into_iter().collect::<BTreeSet<_>>());
    });

    let entries = [(1, 'A'), (1, 'B'), (2, 'C')];
    let all_subsets: BTreeSet<BTreeSet<(i32, char)>> = (0..8)
        .map(|mask: u32| {
            entries
                .iter()
                .enumerate()
                .filter(|(i, _)| mask & (1 << i) != 0)
                .map(|(_, e)| *e)
                .collect()
        })
        .collect();
    assert_eq!(observed.into_inner().unwrap(), all_subsets);
}

/// A key whose values are all missing is missing too: after observing the drops of a keyed
/// stream, a key appears only if one of its values survived.
#[test]
fn sim_keyed_drops_keys_may_be_missing() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();

    let (in_send, input) = node.sim_input::<(i32, char), TotalOrder, ExactlyOnce>();
    // Standing in for entries received over a channel that may drop messages.
    let keys = input
        .weaken_drops::<MayDrop>()
        .into_keyed()
        .observe_drops(
            nondet!(/** the test observes which keys arrive */),
            nonlive!(/** the test does not need every value to arrive */),
        )
        .keys()
        .assume_ordering::<TotalOrder>(nondet!(/** the test compares sets */))
        .sim_output();

    let observed = Mutex::new(BTreeSet::new());
    flow.sim().exhaustive(async || {
        in_send.send((1, 'a'));
        in_send.send((2, 'b'));
        let received: Vec<i32> = keys.collect().await;
        observed
            .lock()
            .unwrap()
            .insert(received.into_iter().collect::<BTreeSet<_>>());
    });

    let expected: BTreeSet<BTreeSet<i32>> = [vec![], vec![1], vec![2], vec![1, 2]]
        .into_iter()
        .map(|keys| keys.into_iter().collect())
        .collect();
    assert_eq!(observed.into_inner().unwrap(), expected);
}

/// The keys of a `MayDrop` keyed stream are an unordered `MayDrop` stream, observed with only a
/// `nonlive!`: the simulator delivers every key.
#[test]
fn sim_keyed_drops_unordered_keys_delivered() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();

    let (in_send, input) = node.sim_input::<(i32, char), TotalOrder, ExactlyOnce>();
    // Standing in for entries received over a channel that may drop messages.
    let keys = input
        .weaken_drops::<MayDrop>()
        .into_keyed()
        .keys()
        .observe_drops(nonlive!(/** the test delivers every value */))
        .assume_ordering::<TotalOrder>(nondet!(/** the test compares sets */))
        .sim_output();

    flow.sim().test_safety_only().exhaustive(async || {
        in_send.send((1, 'a'));
        in_send.send((2, 'b'));
        let received: Vec<i32> = keys.collect().await;
        assert_eq!(
            received.into_iter().collect::<BTreeSet<_>>(),
            [1, 2].into_iter().collect()
        );
    });
}

/// The trajectory argument holds per key: whichever values of a key are missing, every value its
/// sum takes is on some trajectory of the sum over all of its values, and a key appears only once
/// one of its values survives.
#[test]
fn sim_keyed_drops_commutative_fold_stays_on_a_trajectory() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let tick = node.tick();

    let (in_send, input) = node.sim_input::<(char, u32), TotalOrder, ExactlyOnce>();
    // Standing in for entries received over a channel that may drop messages.
    let sums = input
        .weaken_drops::<MayDrop>()
        .into_keyed()
        .observe_drops(
            nondet!(/** the test observes which values arrive */),
            nonlive!(/** the test does not need every value to arrive */),
        )
        .fold(q!(|| 0u32), q!(|acc, x| *acc += x));
    let out_recv = sums
        .snapshot(
            &tick,
            nondet!(/** the test observes every intermediate sum */),
        )
        .entries()
        .all_ticks()
        .sim_output();

    let observed = Mutex::new(BTreeSet::new());
    flow.sim().exhaustive(async || {
        in_send.send(('a', 1));
        in_send.send(('a', 2));
        in_send.send(('b', 4));
        for (key, sum) in out_recv.collect_sorted::<Vec<(char, u32)>>().await {
            match key {
                // The prefix sums of every ordering of `1, 2`, without the initial `0` (a key
                // appears with its first value).
                'a' => assert!([1, 2, 3].contains(&sum)),
                'b' => assert_eq!(sum, 4),
                _ => unreachable!(),
            }
            observed.lock().unwrap().insert((key, sum));
        }
    });
    assert_eq!(
        observed.into_inner().unwrap(),
        [('a', 1), ('a', 2), ('a', 3), ('b', 4)]
            .into_iter()
            .collect()
    );
}

/// A commutative keyed fold and `value_counts` are available on a `MayDrop` keyed stream, and
/// produce a `MayDrop` keyed singleton, which is observed with only a `nonlive!`.
#[test]
fn sim_keyed_drops_aggregations_produce_may_drop_values() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let tick = node.tick();

    let (in_send, input) = node.sim_input::<(char, u32), TotalOrder, ExactlyOnce>();
    // Standing in for entries received over a channel that may drop messages.
    let keyed = input.weaken_drops::<MayDrop>().into_keyed();

    let sums = keyed
        .clone()
        .fold(
            q!(|| 0u32),
            q!(
                |acc, x| *acc += x,
                commutative = manual_proof!(/** addition is commutative */)
            ),
        )
        .map(q!(|sum| sum * 10))
        .observe_drops(nonlive!(/** the test delivers every value */));
    let counts = keyed
        .value_counts()
        .observe_drops(nonlive!(/** the test delivers every value */));

    let sums_recv = sums
        .snapshot(&tick, nondet!(/** the test reads the final sums */))
        .entries()
        .all_ticks()
        .sim_output();
    let counts_recv = counts
        .snapshot(&tick, nondet!(/** the test reads the final counts */))
        .entries()
        .all_ticks()
        .sim_output();

    flow.sim().test_safety_only().exhaustive(async || {
        in_send.send(('a', 1));
        in_send.send(('b', 4));
        in_send.send(('a', 2));
        let sums = sums_recv.collect_sorted::<Vec<_>>().await;
        assert_eq!(sums.iter().rfind(|(k, _)| *k == 'a'), Some(&('a', 30)));
        assert_eq!(sums.iter().rfind(|(k, _)| *k == 'b'), Some(&('b', 40)));
        let counts = counts_recv.collect_sorted::<Vec<_>>().await;
        assert_eq!(counts.iter().rfind(|(k, _)| *k == 'a'), Some(&('a', 2)));
        assert_eq!(counts.iter().rfind(|(k, _)| *k == 'b'), Some(&('b', 1)));
    });
}

/// `reduce` is available on a `MayDrop` keyed stream too, with a commutative closure. The
/// simulator does not yet support an unbounded top-level keyed `reduce`, so it is only
/// type-checked here.
#[expect(dead_code, reason = "only type-checked")]
fn may_drop_keyed_reduce<'a>(
    input: Stream<(char, u32), Process<'a>, Unbounded, TotalOrder, ExactlyOnce, MayDrop>,
) {
    let _max = input
        .into_keyed()
        .reduce(q!(
            |curr, x| *curr = std::cmp::max(*curr, x),
            commutative = manual_proof!(/** max is commutative */)
        ))
        .observe_drops(nonlive!(/** the max of the values that arrive is enough */));
}

/// The keyed `observe_drops(nonlive!)` also requires `test_safety_only`.
#[test]
#[should_panic(expected = "requires `.test_safety_only()`")]
fn sim_keyed_drops_observe_requires_test_safety_only() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let tick = node.tick();

    let (in_send, input) = node.sim_input::<(char, u32), TotalOrder, ExactlyOnce>();
    // Standing in for entries received over a channel that may drop messages.
    let counts = input
        .weaken_drops::<MayDrop>()
        .into_keyed()
        .value_counts()
        .observe_drops(nonlive!(/** the test delivers every value */));
    let out_recv = counts
        .snapshot(&tick, nondet!(/** the test reads the counts */))
        .entries()
        .all_ticks()
        .sim_output();

    flow.sim().exhaustive(async || {
        in_send.send(('a', 1));
        let _ = out_recv.collect_sorted::<Vec<_>>().await;
    });
}
