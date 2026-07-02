use serde::{Deserialize, Serialize};
use stageleft::q;

use crate::live_collections::sliced::sliced;
use crate::live_collections::stream::{ExactlyOnce, TotalOrder};
use crate::location::{Location, Process};
use crate::nondet::nondet;
use crate::prelude::FlowBuilder;
use crate::sim::{SimReceiver, SimSender};

mod trophies;

// Test is currently broken in nightly.
#[cfg(not(nightly))]
#[test]
#[should_panic]
#[cfg_attr(not(target_os = "linux"), ignore)] // sim reproducer not yet reproducible on non-linux OSes
fn sim_crash_in_output() {
    use bytes::Bytes;

    // run as PATH="$PATH:." cargo sim -p hydro_lang --features sim -- sim_crash_in_output
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();

    let (in_send, input) = node.sim_input();
    let out_recv: SimReceiver<Bytes, TotalOrder, ExactlyOnce> = input.sim_output();

    flow.sim().fuzz(async || {
        in_send.send(bolero::any::<Vec<u8>>().into());

        let x = out_recv.next().await.unwrap();
        if !x.is_empty() && x[0] == 42 && x.len() > 1 && x[1] == 43 && x.len() > 2 && x[2] == 44 {
            panic!("boom");
        }
    });
}

// Test is currently broken in nightly.
#[cfg(not(nightly))]
#[test]
#[should_panic]
#[cfg_attr(not(target_os = "linux"), ignore)] // sim reproducer not yet reproducible on non-linux OSes
fn sim_crash_in_output_with_filter() {
    use bytes::Bytes;

    // run as PATH="$PATH:." cargo sim -p hydro_lang --features sim -- sim_crash_in_output_with_filter
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();

    let (in_send, input) = node.sim_input::<Bytes, _, _>();

    let out_recv = input
        .filter(q!(|x| x.len() > 1 && x[0] == 42 && x[1] == 43))
        .sim_output();

    flow.sim().fuzz(async || {
        in_send.send(bolero::any::<Vec<u8>>().into());

        if let Some(x) = out_recv.next().await
            && x.len() > 2
            && x[2] == 44
        {
            panic!("boom");
        }
    });
}

#[test]
fn sim_batch_preserves_order_fuzzed() {
    // uses RNG fuzzing in CI
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();

    let (in_send, input) = node.sim_input();

    let tick = node.tick();
    let out_recv = input
        .batch(&tick, nondet!(/** test */))
        .all_ticks()
        .sim_output();

    flow.sim().fuzz(async || {
        in_send.send(1);
        in_send.send(2);
        in_send.send(3);

        assert_eq!(out_recv.next().await.unwrap(), 1);
        assert_eq!(out_recv.next().await.unwrap(), 2);
        assert_eq!(out_recv.next().await.unwrap(), 3);
        assert!(out_recv.next().await.is_none());
    });
}

fn fuzzed_batching_program<'a>(
    node: Process<'a>,
) -> (
    SimSender<i32, TotalOrder, ExactlyOnce>,
    SimReceiver<i32, TotalOrder, ExactlyOnce>,
) {
    let tick = node.tick();

    let (in_send, input) = node.sim_input();

    let out_recv = input
        .batch(&tick, nondet!(/** test */))
        .fold(q!(|| 0), q!(|acc, v| *acc += v))
        .all_ticks()
        .sim_output();
    (in_send, out_recv)
}

fn fuzzed_batching_program_sliced<'a>(
    node: Process<'a>,
) -> (
    SimSender<i32, TotalOrder, ExactlyOnce>,
    SimReceiver<i32, TotalOrder, ExactlyOnce>,
) {
    let (in_send, input) = node.sim_input();

    let out_recv = sliced! {
        let batch = use(input, nondet!(/** test */));
        batch.fold(q!(|| 0), q!(|acc, v| *acc += v)).into_stream()
    }
    .sim_output();
    (in_send, out_recv)
}

#[test]
#[should_panic]
fn sim_crash_with_fuzzed_batching() {
    // run as PATH="$PATH:." cargo sim -p hydro_lang --features sim -- sim_crash_with_fuzzed_batching
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let (in_send, out_recv) = fuzzed_batching_program(node);

    // takes forever with exhaustive, but should complete quickly with fuzz
    flow.sim().fuzz(async || {
        for _ in 0..1000 {
            in_send.send(456); // the fuzzer should put these some batches
        }

        in_send.send(100);
        in_send.send(23); // the fuzzer must put these in one batch

        in_send.send(99); // the fuzzer must put this in a later batch

        while let Some(out) = out_recv.next().await {
            if out == 456 {
                // make sure exhaustive can't catch the bug by using trivial (size 1) batches
                return;
            } else if out == 123 {
                panic!("boom");
            }
        }
    });
}

#[test]
#[cfg_attr(target_os = "windows", ignore)] // trace locations don't work on Windows right now
fn trace_for_fuzzed_batching() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();

    let (in_send, out_recv) = fuzzed_batching_program(node);

    let repro_bytes = std::fs::read(
        "./src/sim/tests/sim-failures/hydro_lang__sim__tests__sim_crash_with_fuzzed_batching.bin",
    )
    .unwrap();

    let mut log_out = Vec::new();
    colored::control::set_override(false);

    flow.sim()
        .compiled()
        .fuzz_repro(repro_bytes, async |compiled| {
            let schedule = compiled.schedule_with_logger(&mut log_out);
            let rest = async move {
                for _ in 0..1000 {
                    in_send.send(456); // the fuzzer should put these some batches
                }

                in_send.send(100);
                in_send.send(23); // the fuzzer must put these in one batch

                in_send.send(99); // the fuzzer must put this in a later batch

                while let Some(out) = out_recv.next().await {
                    if out == 456 {
                        // make sure exhaustive can't catch the bug by using trivial (size 1) batches
                        return;
                    } else if out == 123 {
                        // don't actually panic so that we can get the trace
                        return;
                    }
                }
            };

            tokio::select! {
                biased;
                _ = rest => {},
                _ = schedule => {},
            };
        });

    let log_str = String::from_utf8(log_out).unwrap();
    hydro_build_utils::assert_snapshot!(log_str);
}

#[test]
#[cfg_attr(target_os = "windows", ignore)] // trace locations don't work on Windows right now
fn trace_for_fuzzed_batching_sliced() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();

    let (in_send, out_recv) = fuzzed_batching_program_sliced(node);

    let repro_bytes = std::fs::read(
        "./src/sim/tests/sim-failures/hydro_lang__sim__tests__sim_crash_with_fuzzed_batching.bin",
    )
    .unwrap();

    let mut log_out = Vec::new();
    colored::control::set_override(false);

    flow.sim()
        .compiled()
        .fuzz_repro(repro_bytes, async |compiled| {
            let schedule = compiled.schedule_with_logger(&mut log_out);
            let rest = async move {
                for _ in 0..1000 {
                    in_send.send(456); // the fuzzer should put these some batches
                }

                in_send.send(100);
                in_send.send(23); // the fuzzer must put these in one batch

                in_send.send(99); // the fuzzer must put this in a later batch

                while let Some(out) = out_recv.next().await {
                    if out == 456 {
                        // make sure exhaustive can't catch the bug by using trivial (size 1) batches
                        return;
                    } else if out == 123 {
                        // don't actually panic so that we can get the trace
                        return;
                    }
                }
            };

            tokio::select! {
                biased;
                _ = rest => {},
                _ = schedule => {},
            };
        });

    let log_str = String::from_utf8(log_out).unwrap();
    hydro_build_utils::assert_snapshot!(log_str);
}

#[derive(Serialize, Deserialize)]
struct Test {}

#[test]
fn sim_batch_nondebuggable_type() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();

    let (in_send, input) = node.sim_input::<_, TotalOrder, _>();

    let tick = node.tick();
    let _out_recv = input
        .batch(&tick, nondet!(/** test */))
        .count()
        .all_ticks()
        .sim_output();

    flow.sim().exhaustive(async || {
        in_send.send(Test {});
        let _: Vec<_> = _out_recv.collect().await;
    });
}

#[test]
fn sim_embedded_send() {
    use crate::networking::TCP;

    let mut flow = FlowBuilder::new();
    let p1 = flow.process::<()>();
    let p2 = flow.process::<()>();

    let (in_send, input) = p1.sim_input::<i32, _, _>();
    // `.embedded()` leaves serialization to external code; in the simulator the raw value is
    // carried directly through the in-memory channel.
    let out_recv = input
        .send(&p2, TCP.fail_stop().embedded().name("ch"))
        .map(q!(|x| x * 10))
        .sim_output();

    flow.sim().exhaustive(async || {
        in_send.send(1);
        in_send.send(2);
        let all: Vec<i32> = out_recv.collect().await;
        assert_eq!(all, vec![10, 20]);
    });
}

#[test]
fn sim_cluster_e2m_m2e() {
    let mut flow = FlowBuilder::new();
    let cluster = flow.cluster::<()>();

    let (in_send, input) = cluster.sim_input::<i32>();
    let out_recv = input.map(q!(|x| x * 10)).sim_cluster_output();

    flow.sim()
        .with_cluster_size(&cluster, 3)
        .exhaustive(async || {
            // Send values to specific cluster members
            in_send.send(0, 1); // member 0 gets 1
            in_send.send(1, 2); // member 1 gets 2
            in_send.send(2, 3); // member 2 gets 3

            // Each member multiplies by 10
            assert_eq!(out_recv.next(0).await, Some(10));
            assert_eq!(out_recv.next(1).await, Some(20));
            assert_eq!(out_recv.next(2).await, Some(30));
        });
}
#[test]
fn sim_send_after_assert_yields_only() {
    let mut flow = FlowBuilder::new();
    let process = flow.process::<()>();

    let (send_port, input) = process.sim_input();
    let output = input.atomic().end_atomic();
    let out_port = output.sim_output();

    flow.sim().exhaustive(async || {
        send_port.send(1u32);
        out_port.assert_yields_only([1u32]).await;

        // This previously panicked with SendError because the scheduler terminated on quiescence.
        send_port.send(2u32);
        out_port.assert_yields_only([2u32]).await;
    });
}

#[test]
#[should_panic(expected = "unexpected message")]
fn assert_yields_only_catches_extra_value() {
    let mut flow = FlowBuilder::new();
    let process = flow.process::<()>();

    let (send_port, input) = process.sim_input();
    let out_port = input.atomic().end_atomic().sim_output();

    flow.sim().exhaustive(async || {
        send_port.send(1u32);
        send_port.send(2u32);
        // Expects only [1], but stream also produces 2 → should panic
        out_port.assert_yields_only([1u32]).await;
    });
}

#[test]
fn sim_collect_waits_for_all_ticks() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let tick = node.tick();
    let (in_send, input) = node.sim_input();
    let out_recv = input
        .batch(&tick, nondet!(/** test */))
        .all_ticks()
        .sim_output();

    flow.sim().exhaustive(async || {
        in_send.send(1);
        in_send.send(2);
        in_send.send(3);
        let all: Vec<i32> = out_recv.collect().await;
        assert_eq!(all, vec![1, 2, 3]);
    });
}

/// Regression test for https://github.com/hydro-project/hydro/issues/2602
/// Verifies that `resolve_futures_blocking` preserves `Bounded`, allowing
/// its output to be used with APIs that require boundedness (e.g. `cross_singleton`).
/// If `resolve_futures_blocking` ever regresses to return `Unbounded`, this test
/// will fail to compile.
#[test]
fn resolve_futures_blocking_preserves_bounded() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let tick = node.tick();

    let resolved = node
        .source_iter(q!(vec![1, 2, 3]))
        .batch(&tick, nondet!(/** test */))
        .map(q!(|x| async move { x }))
        .resolve_futures_blocking();

    // cross_singleton requires Bounded — this is the compile-time regression check
    let crossed = resolved.cross_singleton(node.singleton(q!(10)).clone_into_tick(&tick));

    let out_recv = crossed.all_ticks().sim_output();

    flow.sim().exhaustive(async || {
        let results: Vec<(i32, i32)> = out_recv.collect_sorted().await;
        assert_eq!(results, vec![(1, 10), (2, 10), (3, 10)]);
    });
}

#[test]
fn sim_fold_sample_eager_state_count() {
    use crate::live_collections::stream::NoOrder;
    use crate::properties::manual_proof;

    // Assert the exact exhaustive state count to detect regressions.
    // 108 states with batch-fold optimization + passthrough singleton hook + always permute.
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();

    let (in_send, input) = node.sim_input::<i32, NoOrder, ExactlyOnce>();

    let folded = input.fold(
        q!(|| 0),
        q!(
            |acc, v| *acc += v,
            commutative = manual_proof!(/** integer addition is commutative */)
        ),
    );
    let out_recv = sliced! {
        let snapshot = use(folded, nondet!(/** test */));
        snapshot.into_stream()
    }
    .sim_output();

    let count = flow.sim().exhaustive(async || {
        in_send.send_many_unordered([1, 2, 3]);

        let all: Vec<i32> = out_recv.collect().await;
        // The final value must always be 6 (1+2+3)
        assert_eq!(*all.last().unwrap(), 6);
    });

    assert_eq!(count, 108, "Exhaustive states explored");
}

#[test]
fn sim_fold_commutative_explores_all_subset_sums() {
    use std::collections::HashSet;

    use crate::live_collections::stream::NoOrder;
    use crate::properties::manual_proof;

    // With inputs [1, 2, 4], the possible subset sums are:
    // {1}, {2}, {4}, {1,2}, {1,4}, {2,4}, {1,2,4} → sums: 1, 2, 4, 3, 5, 6, 7
    // The fold can be snapshotted after processing any prefix of subsets.
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();

    let (in_send, input) = node.sim_input::<i32, NoOrder, ExactlyOnce>();
    let folded = input.fold(
        q!(|| 0),
        q!(
            |acc, v| *acc += v,
            commutative = manual_proof!(/** addition is commutative */)
        ),
    );
    let out_recv = sliced! {
        let snapshot = use(folded, nondet!(/** test */));
        snapshot.into_stream()
    }
    .sim_output();

    let mut observed_values = HashSet::new();

    flow.sim().exhaustive(async || {
        in_send.send_many_unordered([1, 2, 4]);
        let all: Vec<i32> = out_recv.collect().await;
        assert_eq!(*all.last().unwrap(), 7);
        for &v in &all {
            observed_values.insert(v);
        }
    });

    // The exhaustive exploration must observe every possible subset sum.
    // With inputs [1, 2, 4], the fold can be snapshotted after processing any
    // non-empty subset, so all values 1..=7 must appear, plus 0 (initial state).
    let expected: HashSet<i32> = (0..=7).collect();
    assert_eq!(
        observed_values, expected,
        "Should observe all subset sums across all executions"
    );
}

#[test]
fn sim_fold_total_order_no_permutation() {
    // Non-commutative fold on TotalOrder: no hook emitted, order is fixed.
    // Every intermediate must be a prefix-concatenation of "a","b","c".
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();

    let source = node.source_stream(q!(tokio_stream::iter(vec!["a", "b", "c"])));
    let folded = source.fold(q!(|| String::new()), q!(|acc, v| acc.push_str(v)));
    let out_recv = sliced! {
        let snapshot = use(folded, nondet!(/** test */));
        snapshot.into_stream()
    }
    .sim_output();

    let mut all_observed = std::collections::HashSet::new();

    flow.sim().exhaustive(async || {
        let all: Vec<String> = out_recv.collect().await;
        assert_eq!(all.last().unwrap(), "abc");
        for v in all {
            all_observed.insert(v);
        }
    });

    // Only valid prefixes should be observed (no permutations like "ba", "cab", etc.)
    for v in &all_observed {
        assert!(
            ["", "a", "ab", "abc"].contains(&v.as_str()),
            "Unexpected intermediate: {:?}",
            v
        );
    }
}

#[test]
fn sim_fold_keyed_no_order() {
    use crate::live_collections::stream::NoOrder;
    use crate::properties::manual_proof;

    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let (in_send, input) = node.sim_input::<(u32, i32), NoOrder, ExactlyOnce>();

    let folded = input.into_keyed().fold(
        q!(|| 0),
        q!(
            |acc, v| *acc += v,
            commutative = manual_proof!(/** addition is commutative */)
        ),
    );
    let out_recv = sliced! {
        let snapshot = use(folded, nondet!(/** test */));
        snapshot.entries()
    }
    .sim_output();

    flow.sim().exhaustive(async || {
        in_send.send_many_unordered([(1, 10), (2, 20), (1, 30)]);
        let all: Vec<(u32, i32)> = out_recv.collect_sorted().await;
        let mut last_by_key = std::collections::HashMap::new();
        for (k, v) in all {
            last_by_key.insert(k, v);
        }
        assert_eq!(last_by_key.get(&1), Some(&40));
        assert_eq!(last_by_key.get(&2), Some(&20));
    });
}

#[test]
fn sim_fold_tee_downstream_sees_different_subsets() {
    use std::collections::HashSet;

    // Two downstream consumers of the same fold([1, 2, 3]) accumulator can
    // independently snapshot at different times. One might see {3, 6} while
    // the other sees {1, 3, 6} — they are not forced to observe the same
    // intermediate states.
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();

    let source = node.source_stream(q!(tokio_stream::iter(vec![1, 2, 3])));
    let folded = source.fold(q!(|| 0), q!(|acc, v| *acc += v));

    let out_a = sliced! {
        let snapshot = use(folded.clone(), nondet!(/** test */));
        snapshot.into_stream()
    }
    .sim_output();

    let out_b = sliced! {
        let snapshot = use(folded, nondet!(/** test */));
        snapshot.into_stream()
    }
    .sim_output();

    let mut observed_pairs: HashSet<(Vec<i32>, Vec<i32>)> = HashSet::new();

    flow.sim().exhaustive(async || {
        let a_values: Vec<i32> = out_a.collect().await;
        let b_values: Vec<i32> = out_b.collect().await;

        // Both must end at 6 (1+2+3)
        assert_eq!(*a_values.last().unwrap(), 6);
        assert_eq!(*b_values.last().unwrap(), 6);

        observed_pairs.insert((a_values, b_values));
    });

    // There must exist at least one execution where the two downstreams
    // observed different sequences of intermediate states.
    #[expect(clippy::disallowed_methods, reason = "order is not used in test")]
    let has_divergent = observed_pairs.iter().any(|(a, b)| a != b);
    assert!(
        has_divergent,
        "Expected at least one execution where downstream consumers see different intermediate states, \
         but all observed pairs were identical: {:?}",
        observed_pairs
    );
}

/// Demonstrates that the simulator catches a bug in a fold that falsely claims commutativity.
/// The exhaustive run should observe different final values (e.g. "ab" vs "ba"),
/// which would violate the invariant that a commutative fold's result is order-independent.
#[test]
fn sim_fold_catches_false_commutativity() {
    use std::collections::HashSet;

    use crate::live_collections::stream::NoOrder;
    use crate::properties::manual_proof;

    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();

    let (in_send, input) = node.sim_input::<String, NoOrder, ExactlyOnce>();
    // string concatenation is not commutative, but lets claim it is, what
    // could go wrong
    let folded = input.fold(
        q!(|| String::new()),
        q!(
            |acc, v| acc.push_str(&v),
            commutative = manual_proof!(/** WRONG */)
        ),
    );
    let out_recv = sliced! {
        let snapshot = use(folded, nondet!(/** test */));
        snapshot.into_stream()
    }
    .sim_output();

    let mut final_values = HashSet::new();

    flow.sim().exhaustive(async || {
        in_send.send_many_unordered(["a".to_owned(), "b".to_owned()]);
        let all: Vec<String> = out_recv.collect().await;
        // Collect the first values we see to verify we're fully exploring
        // the state space. If we're _not_ then we wouldn't see a "ba"
        // permutation as the first result
        final_values.insert(all.first().unwrap().clone());
    });

    // If commutativity held, we wouldn't see "ba"
    assert!(
        final_values.contains("ab") && final_values.contains("ba"),
        "Expected both 'ab' and 'ba' to be observed, got: {:?}",
        final_values
    );
}

/// Verifies that the simulator catches false commutativity for in-tick folds on
/// NoOrder streams by permuting the batch before it reaches the fold.
///
/// Top-level folds ARE tested via cross-batch subset selection + permutation
/// (see `sim_fold_catches_false_commutativity`).
#[test]
fn sim_fold_in_tick_catches_false_commutativity() {
    use std::collections::HashSet;

    use crate::live_collections::stream::NoOrder;
    use crate::properties::manual_proof;

    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();

    let (in_send, input) = node.sim_input::<String, NoOrder, ExactlyOnce>();

    let tick = node.tick();
    let out_recv = input
        .batch(&tick, nondet!(/** test */))
        .fold(
            q!(|| String::new()),
            q!(
                |acc, v| acc.push_str(&v),
                commutative = manual_proof!(/** WRONG */)
            ),
        )
        .into_stream()
        .all_ticks()
        .sim_output();

    let mut final_values = HashSet::new();

    flow.sim().exhaustive(async || {
        in_send.send_many_unordered(["a".to_owned(), "b".to_owned()]);
        let all: Vec<String> = out_recv.collect().await;
        for v in all {
            final_values.insert(v);
        }
    });

    assert!(
        final_values.contains("ab") && final_values.contains("ba"),
        "Expected both \"ab\" and \"ba\" to be observed, got: {:?}",
        final_values
    );
}

/// Minimal repro for the singleton empty-on-first-tick bug.
///
/// The bug: when one `sliced!` block emits a singleton that is consumed by
/// another `sliced!` block, the second tick may be scheduled before the first
/// has run. At that point the singleton has no value yet, but the IR marks it
/// as `Singleton` (which must always have a value). The SingletonHook panics
/// with "No input and no last released item to re-release".
#[test]
fn sim_singleton_not_ready_until_producer_runs() {
    use crate::live_collections::stream::NoOrder;

    let mut flow = FlowBuilder::new();
    let p = flow.process::<()>();

    let (in_port, in_stream) = p.sim_input::<u32, TotalOrder, _>();
    let in_no_order = in_stream.weaken_ordering::<NoOrder>();

    // First sliced block: produces an Unbounded Singleton
    let produced_singleton = sliced! {
        let batch = use(in_no_order.clone(), nondet!(/** batch */));
        batch.assume_ordering::<TotalOrder>(nondet!(/** order */))
            .fold(q!(|| 0u32), q!(|acc, v| *acc += v))
    };

    // Second sliced block: consumes the singleton via use(singleton, nondet).
    // If the simulator schedules this tick before the first one has run,
    // the SingletonHook has no value → panic.
    let out = sliced! {
        let trigger = use(in_no_order, nondet!(/** batch */));
        let snapshot = use(produced_singleton, nondet!(/** snapshot */));
        trigger.cross_singleton(snapshot)
    }
    .assume_ordering::<TotalOrder>(nondet!(/** test */));

    let out_port = out.sim_output();

    flow.sim().exhaustive(async || {
        in_port.send(42);
        let _ = out_port.next().await;
    });
}

/// The simulator does not yet support `Unbounded` keyed singletons (where keys can be removed).
/// This snapshot test verifies the panic message when attempting to simulate one.
#[test]
#[cfg_attr(target_os = "windows", ignore)]
fn sim_unbounded_keyed_singleton_rejected_snapshot() {
    use crate::compile::ir::KeyedSingletonBoundKind;

    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();

    let (input_port, input) = node.sim_input::<(u32, u32), TotalOrder, ExactlyOnce>();

    let monotone_keys_singleton = input
        .into_keyed()
        .fold(q!(|| 0u32), q!(|acc, v| *acc = (*acc).max(v)));

    // Patch the IR node's collection_kind to Unbounded.
    // There's currently no public API that produces an Unbounded keyed singleton.
    {
        let mut ir = monotone_keys_singleton.ir_node.borrow_mut();
        if let crate::compile::ir::CollectionKind::KeyedSingleton { ref mut bound, .. } =
            ir.metadata_mut().collection_kind
        {
            *bound = KeyedSingletonBoundKind::Unbounded;
        }
    }

    let output = monotone_keys_singleton
        .snapshot(&node.tick(), nondet!(/** test */))
        .entries()
        .all_ticks()
        .sim_output();

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        flow.sim().exhaustive(async || {
            input_port.send((1, 100));
            let _ = output.collect_sorted::<Vec<_>>().await;
        });
    }));

    let err = result.unwrap_err();
    let panic_msg = err
        .downcast_ref::<String>()
        .map(|s| s.as_str())
        .or_else(|| err.downcast_ref::<&str>().copied())
        .unwrap_or("(non-string panic)");

    hydro_build_utils::assert_snapshot!(panic_msg);
}

/// The simulator does not yet support non-atomic yield of an Optional (i.e., `latest()`
/// on a tick-level Optional that produces a top-level Unbounded Optional).
/// This snapshot test verifies the panic message.
#[test]
#[cfg_attr(target_os = "windows", ignore)]
fn sim_unbounded_optional_rejected_snapshot() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();

    let (input_port, input) = node.sim_input::<u32, TotalOrder, ExactlyOnce>();

    let tick = node.tick();
    let optional = input
        .batch(&tick, nondet!(/** test */))
        .sort()
        .first()
        .latest();

    let output = sliced! {
        let snapshot = use(optional, nondet!(/** test */));
        snapshot.into_stream()
    }
    .sim_output();

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        flow.sim().exhaustive(async || {
            input_port.send(42);
            let _ = output.collect::<Vec<_>>().await;
        });
    }));

    let err = result.unwrap_err();
    let panic_msg = err
        .downcast_ref::<String>()
        .map(|s| s.as_str())
        .or_else(|| err.downcast_ref::<&str>().copied())
        .unwrap_or("(non-string panic)");

    hydro_build_utils::assert_snapshot!(panic_msg);
}
