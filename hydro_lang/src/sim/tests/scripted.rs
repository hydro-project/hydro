//! Tests for simulator hooks: scripting the decisions of unsafe operators.
//!
//! These tests cover forgotten and *temporarily* forgotten hooks, boundary-scan
//! placement, the safety of implicit decisions (the T1/T2 scenario), schedule-dependent
//! decisions like `release_all`, and the script-as-schedule group model.

use stageleft::q;

#[expect(unused_imports, reason = "used by some tests")]
use crate::live_collections::Optional;
use crate::live_collections::sliced::sliced;
use crate::live_collections::stream::{ExactlyOnce, NoOrder, TotalOrder};
use crate::live_collections::{Singleton, Stream};
use crate::location::{Location, Process};
use crate::nondet::{NonDet, nondet};
use crate::prelude::{Bounded, FlowBuilder, Unbounded};
use crate::sim::{SimReceiver, SimSender};
use crate::sim_hooks::{
    BatchHook, KeyedBatchHook, KeyedMergeOrderedHook, KeyedOrderingHook, KeyedSnapshotHook,
    MergeOrderedHook, OrderingHook, PartialOrderingHook, SimHook, SnapshotHook,
};

/// A commutativity proof does not exempt a fold from simulation: the ordering hook on the
/// proof scripts the exploration, releasing one named element per decision so intermediate
/// fold states are observable at exactly the script's release points (the subset-and-permute
/// optimization is bypassed).
#[test]
fn scripted_fold_commutative_hook_releases_one_at_a_time() {
    use crate::properties::manual_proof;

    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let ordering: OrderingHook<i32, Unbounded> = flow.sim_hook();
    let state_hook: SnapshotHook<i32> = flow.sim_hook();

    let (in_send, input) = node.sim_input::<i32, NoOrder, ExactlyOnce>();
    let folded = input.fold(
        q!(|| 0),
        q!(
            |acc, v| *acc += v,
            commutative = manual_proof!(
                /// integer addition is commutative
                hook = ordering
            )
        ),
    );
    let out_recv = sliced! {
        let state = use::snapshot(folded, nondet!(/** scripted */ hook = state_hook));
        state.into_stream()
    }
    .sim_output();

    flow.sim().deterministic(async || {
        in_send.send_many_unordered([1, 2, 4]);

        // The ordering hook's remaining input stays buffered across the interleaved tick
        // executions below; `auto_pause` declares that standing buffering once.
        ordering.auto_pause();

        state_hook.reveal(0).await; // the initial fold state is a buffered version too

        ordering.next(2).await;
        state_hook.reveal(2).await;

        ordering.next(4).await;
        state_hook.reveal(6).await;

        ordering.next(1).await;
        state_hook.reveal(7).await;

        out_recv.assert_yields_only([0, 2, 6, 7]).await;
    });
}

/// Under `exhaustive`, ticks are interleaved *around* a scripted top-level ordering
/// observation: `next(1)` names a value that only exists after the (fuzzer-scheduled,
/// unhooked) tick processes 0 and cycles it back through the network, so the observation
/// waits for that tick; conversely the tick can fire either between the scripted releases
/// of 1 and 2 or only after both, and exhaustive mode must explore both interleavings.
#[test]
fn scripted_exhaustive_ticks_interleave_around_observation_cycle() {
    use crate::networking::TCP;

    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let node2 = flow.process::<()>();
    let ordering: OrderingHook<u32, Unbounded> = flow.sim_hook();

    let (in_send, input) = node.sim_input::<u32, NoOrder, _>();

    let (complete_cycle_back, cycle_back) = node.forward_ref::<Stream<_, _, _, NoOrder>>();
    let ordered = input
        .merge_unordered(cycle_back)
        .assume_ordering::<TotalOrder>(nondet!(/** scripted */ hook = ordering));

    let tick_outputs = sliced! {
        let b = use::batch(
            ordered.clone(),
            nondet!(/** fuzzed: the tick schedule is explored exhaustively */)
        );
        (b.clone(), b.collect_vec().into_stream())
    };
    let (cycle_src, batches) = tick_outputs;
    complete_cycle_back.complete(
        cycle_src
            .map(q!(|v| v + 1))
            .filter(q!(|v| *v == 1))
            .send(&node2, TCP.fail_stop().bincode())
            .send(&node, TCP.fail_stop().bincode()),
    );
    let batches_recv = batches.sim_output();
    let out_recv = ordered.sim_output();

    let mut saw_tick_between_releases = false;
    let mut saw_tick_after_both_releases = false;
    flow.sim().exhaustive(async || {
        in_send.send_many_unordered([0, 2]);

        // 1 does not exist yet: it is 0 cycled back through the tick, so this decision
        // suspends until the fuzzer-scheduled tick has fired — and it must beat the
        // already-buffered 2.
        ordering.next(0).await;
        ordering.next(1).await;
        ordering.next(2).await;

        let out: Vec<u32> = out_recv.collect().await;
        assert_eq!(out, [0, 1, 2]);

        let batches: Vec<Vec<u32>> = batches_recv.collect().await;
        assert_eq!(batches.first().unwrap(), &[0]);
        if batches.contains(&vec![1]) {
            saw_tick_between_releases = true;
        }
        if batches.contains(&vec![1, 2]) {
            saw_tick_after_both_releases = true;
        }
    });

    assert!(
        saw_tick_between_releases,
        "no instance ran the tick between the scripted releases of 1 and 2"
    );
    assert!(
        saw_tick_after_both_releases,
        "no instance deferred the tick until after both scripted releases"
    );
}

/// A top-level `assume_ordering` (outside any tick) is scripted as a standalone scheduler
/// action: each `next(value)` decision releases exactly the one named buffered element,
/// and the scripted order is what flows downstream.
#[test]
fn scripted_top_level_ordering_is_a_scheduler_action() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let ordering: OrderingHook<u32, Unbounded> = flow.sim_hook();
    let (input_send, input) = node.sim_input::<_, NoOrder, _>();
    let output = input
        .assume_ordering::<TotalOrder>(nondet!(/** scripted */ hook = ordering))
        .sim_output();

    flow.sim().deterministic(async || {
        input_send.send_many_unordered([1, 2, 3]);
        ordering.next(2).await;
        ordering.next(1).await;
        ordering.next(3).await;
        output.assert_yields_only([2, 1, 3]).await;
    });
}

/// Two scripted `assume_ordering` hooks chained at the same top-level location (the
/// second reached via `weaken_ordering`) are independent scheduler actions: running one
/// hook's scripted decision must not touch the other hook, which has no queued decision
/// (its input has not even been produced yet when the first decision runs).
///
/// Each top-level hook is its own observation, so running one hook's decision never
/// touches a co-located sibling (which would otherwise hit the `implicit()` invariant
/// abort, since a top-level hook has no forced behavior). The remaining input buffered at
/// `first` while `second`'s group is outstanding is declared with `pause()`, per the
/// ordinary waiting-group rules.
#[test]
fn scripted_chained_top_level_orderings_run_independently() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let first: OrderingHook<u32> = flow.sim_hook();
    let second: OrderingHook<u32> = flow.sim_hook();
    let (in_send, input) = node.sim_input::<_, NoOrder, _>();
    let output = input
        .assume_ordering::<TotalOrder>(nondet!(/** scripted */ hook = first))
        .weaken_ordering::<NoOrder>()
        .assume_ordering::<TotalOrder>(nondet!(/** scripted */ hook = second))
        .sim_output();

    flow.sim().deterministic(async || {
        in_send.send_many_unordered([1, 2]);
        first.next(2).await; // the second hook has no input and no decision here
        // While `second`'s group waits for 2 to reach it, `first` still holds 1 with no
        // decision; buffering it across those boundaries is declared like any other.
        first.pause();
        second.next(2).await;
        first.next(1).await; // installing implicitly resumes `first`
        second.next(1).await;
        output.assert_yields_only([2, 1]).await;
    });
}

/// A paused scripted hook co-located with an unscripted (fuzzed) hook is left untouched
/// when the scheduler resolves the sibling: the pause declares the scripted hook's
/// buffered input, and only the sibling releases.
///
/// With per-hook observations, the fuzzer resolving the unscripted hook is an action on
/// that hook alone; the co-located scripted hook (which has no decision, only a declared
/// pause) is a separate observation that simply is not runnable.
#[test]
fn scripted_paused_hook_untouched_when_colocated_fuzzed_hook_releases() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let scripted: OrderingHook<u32> = flow.sim_hook();
    let (s_send, s_in) = node.sim_input::<_, NoOrder, _>();
    let (u_send, u_in) = node.sim_input::<u32, NoOrder, ExactlyOnce>();
    let s_out = s_in
        .assume_ordering::<TotalOrder>(nondet!(/** scripted */ hook = scripted))
        .sim_output();
    let u_out = u_in
        .assume_ordering::<TotalOrder>(nondet!(/** fuzzed */))
        .sim_output();

    flow.sim().exhaustive(async || {
        s_send.send_many_unordered([1u32]);
        u_send.send_many_unordered([10u32]);

        scripted.pause(); // the scripted stream's input is deliberately withheld for now
        u_out.assert_yields([10]).await; // resolving the sibling must not touch the pause

        scripted.next(1).await;
        s_out.assert_yields_only([1]).await;
    });
}

/// An in-tick `assume_ordering` is scripted with a single `order` decision supplying a
/// complete permutation of the tick's input, and that decision joins the same script
/// group as the batch decision feeding the tick (one execution consumes both together).
#[test]
fn scripted_inline_ordering_joins_tick_group() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let batch_hook: BatchHook<u32, NoOrder> = flow.sim_hook();
    let ordering: OrderingHook<u32, Bounded> = flow.sim_hook();
    let (input_send, input) = node.sim_input::<_, NoOrder, _>();
    let output = sliced! {
        let b = use::batch(input, nondet!(/** scripted */ hook = batch_hook));
        b.assume_ordering::<TotalOrder>(nondet!(/** scripted */ hook = ordering))
    }
    .sim_output();

    flow.sim().deterministic(async || {
        input_send.send_many_unordered([1, 2, 3]);
        batch_hook.release_values([1, 2, 3]).await;
        ordering.order([3, 1, 2]).await;
        output.assert_yields_only([3, 1, 2]).await;
    });
}

/// An in-tick ordering over two or more elements is a genuine choice: releasing a batch
/// into the tick without scripting the ordering fails the tick, rather than falling back
/// to some arbitrary order the test never chose.
#[test]
#[should_panic(expected = "require an explicit ordering")]
fn scripted_inline_ordering_missing_decision_panics_during_tick() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let batch_hook: BatchHook<u32, NoOrder> = flow.sim_hook();
    let ordering: OrderingHook<u32, Bounded> = flow.sim_hook();
    let (input_send, input) = node.sim_input::<_, NoOrder, _>();
    let _output = sliced! {
        let b = use::batch(input, nondet!(/** scripted */ hook = batch_hook));
        b.assume_ordering::<TotalOrder>(nondet!(/** scripted */ hook = ordering))
    }
    .sim_output();

    flow.sim().deterministic(async || {
        input_send.send_many_unordered([1, 2]);
        batch_hook.release_values([1, 2]).await;
    });
}

/// An in-tick commutative fold is simulated through an inline shuffle of its batch, and
/// the ordering hook on the `manual_proof!` scripts that shuffle: the decision joins the
/// same tick group as the batch decision, and must order the complete tick-local input.
#[test]
fn scripted_in_tick_fold_commutative_hook_orders_batch() {
    use crate::properties::manual_proof;

    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let batch_hook: BatchHook<u32, NoOrder> = flow.sim_hook();
    let ordering: OrderingHook<u32, Bounded> = flow.sim_hook();
    let (input_send, input) = node.sim_input::<_, NoOrder, _>();
    let output = sliced! {
        let b = use::batch(input, nondet!(/** scripted */ hook = batch_hook));
        b.fold(
            q!(|| Vec::new()),
            q!(
                |acc, v| acc.push(v),
                // Deliberately not commutative: the scripted order is observable in the
                // accumulated Vec, demonstrating that the proof is not trusted.
                commutative = manual_proof!(
                    /// scripted by the test
                    hook = ordering
                )
            ),
        )
        .into_stream()
    }
    .sim_output();

    flow.sim().deterministic(async || {
        input_send.send_many_unordered([1, 2, 3]);
        batch_hook.release_values([1, 2, 3]).await;
        ordering.order([2, 3, 1]).await;
        output.assert_yields_only([vec![2, 3, 1]]).await;
    });
}

/// A counter service with a `batch` that groups incoming
/// read requests, and a `snapshot` that picks which version of the count each batch of
/// reads observes.
///
/// # Non-Determinism
/// - `nondet_batch`: how read requests are batched is observable in responses
/// - `nondet_snapshot`: reads may observe any version of the count
fn counter_service<'a>(
    increments: Stream<u64, Process<'a>>,
    get_requests: Stream<u32, Process<'a>>,
    nondet_batch: NonDet<Option<BatchHook<u32>>>,
    nondet_snapshot: NonDet<Option<SnapshotHook<u64>>>,
) -> Stream<(u32, u64), Process<'a>, Unbounded, TotalOrder> {
    let current_count: Singleton<u64, _, _> = increments.fold(q!(|| 0), q!(|acc, v| *acc += v));

    sliced! {
        let request_batch = use::batch(get_requests, nondet_batch);
        let count_snapshot = use::snapshot(current_count, nondet_snapshot);

        request_batch.cross_singleton(count_snapshot)
    }
}

struct CounterSim {
    inc_send: SimSender<u64, TotalOrder, ExactlyOnce>,
    get_send: SimSender<u32, TotalOrder, ExactlyOnce>,
    out: SimReceiver<(u32, u64), TotalOrder, ExactlyOnce>,
    batch_hook: BatchHook<u32>,
    snapshot_hook: SnapshotHook<u64>,
    flow: FlowBuilder<'static>,
}

fn counter_sim() -> CounterSim {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let batch_hook: BatchHook<u32> = flow.sim_hook();
    let snapshot_hook: SnapshotHook<u64> = flow.sim_hook();

    let (inc_send, increments) = node.sim_input();
    let (get_send, get_requests) = node.sim_input();

    let out = counter_service(
        increments,
        get_requests,
        nondet!(/** scripted by the test */ hook = batch_hook),
        nondet!(/** scripted by the test */ hook = snapshot_hook),
    )
    .sim_output();

    CounterSim {
        inc_send,
        get_send,
        out,
        batch_hook,
        snapshot_hook,
        flow,
    }
}

/// A read request arrives between
/// two increments, and the service reveals the state after the first increment.
#[test]
fn scripted_counter_reveals_intermediate_state() {
    let CounterSim {
        inc_send,
        get_send,
        out,
        batch_hook,
        snapshot_hook,
        flow,
    } = counter_sim();

    flow.sim().deterministic(async || {
        inc_send.send_many([1, 1]);
        get_send.send(0);

        snapshot_hook.reveal(1u64).await; // the batch observes count = 1 (not the latest, 2!)
        batch_hook.release(1).await; // ...and contains exactly the one read request

        // The unrevealed version 2 stays buffered at the snapshot; holding data past a
        // point where the operator could have fired requires an explicit declaration
        // (otherwise the boundary scan reports the forgotten version).
        snapshot_hook.pause();

        out.assert_yields_only([(0, 1u64)]).await;
    });
}

/// A two-group script with two successive executions of the
/// same tick, each pairing a revealed count with one released request. `reveal(3)` skips
/// over the unobserved version 2.
#[test]
fn scripted_counter_two_tick_executions() {
    let CounterSim {
        inc_send,
        get_send,
        out,
        batch_hook,
        snapshot_hook,
        flow,
    } = counter_sim();

    flow.sim().deterministic(async || {
        inc_send.send_many([1, 1, 1]);
        get_send.send_many([7, 8]);

        snapshot_hook.reveal(1u64).await; // 1st execution: reveal count = 1
        batch_hook.release(1).await; // 1st execution: get 7 → (7, 1)   (same group: no waiting)

        snapshot_hook.reveal(3u64).await; // 2nd execution: reveal count = 3 — suspends until
        batch_hook.release(1).await; // the 1st execution has actually run

        out.assert_yields_only([(7, 1u64), (8, 3u64)]).await;
    });
}

/// A struct of handles created in one `flow.sim_hook()` call. `Option` fields make the
/// struct usable as a composite hook payload too (its `Default` is "no hooks").
#[derive(Clone, Copy, Default)]
struct CounterHooks {
    batch: Option<BatchHook<u32>>,
    snapshot: Option<SnapshotHook<u64>>,
}

impl SimHook for CounterHooks {
    fn create(next_id: &mut dyn FnMut() -> usize) -> Self {
        CounterHooks {
            batch: SimHook::create(next_id),
            snapshot: SimHook::create(next_id),
        }
    }
}

/// A struct of handles implementing `SimHook` is created in one `flow.sim_hook()`
/// call, and its fields bind and script exactly like individually created handles.
#[test]
fn scripted_hook_bundle() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let hooks: CounterHooks = flow.sim_hook();

    let (inc_send, increments) = node.sim_input();
    let (get_send, get_requests) = node.sim_input();

    let out = counter_service(
        increments,
        get_requests,
        nondet!(/** scripted */ hook = hooks.batch),
        nondet!(/** scripted */ hook = hooks.snapshot),
    )
    .sim_output();

    flow.sim().deterministic(async || {
        inc_send.send(5);
        get_send.send(0);

        hooks.snapshot.unwrap().reveal(5u64).await;
        hooks.batch.unwrap().release(1).await;

        out.assert_yields_only([(0, 5u64)]).await;
    });
}

/// A single scripted batch feeding a per-tick fold, used by many tests below.
///
/// # Non-Determinism
/// - `nondet_batch`: how the input is batched determines the per-tick sums
fn scripted_batch_sum<'a>(
    input: Stream<i32, Process<'a>>,
    nondet_batch: NonDet<Option<BatchHook<i32>>>,
) -> Stream<i32, Process<'a>> {
    sliced! {
        let batch = use::batch(input, nondet_batch);
        batch.fold(q!(|| 0), q!(|acc, v| *acc += v)).into_stream()
    }
}

/// A *forgotten* hook — buffered input, no decision, no hold — is
/// confronted at the first scheduling boundary, not silently hidden or left to chance.
/// Without this, "no more output arrives" below would pass for exactly the wrong reason.
#[test]
#[should_panic(expected = "scripted hook has buffered input but no decision")]
fn scripted_forgotten_hook_panics() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let batch_hook: BatchHook<i32> = flow.sim_hook();

    let (in_send, input) = node.sim_input();
    let out = scripted_batch_sum(input, nondet!(/** scripted */ hook = batch_hook)).sim_output();

    flow.sim().deterministic(async || {
        in_send.send_many([1, 2]);
        // The test "moves on" without ever scripting the batch: the boundary scan panics
        // instead of letting this assertion succeed vacuously.
        out.assert_no_more().await;
    });
}

/// The *partial* version of the forgotten-hook failure — a hook scripted *too
/// late*, with data sitting buffered through other steps — requires an explicit `pause()`
/// declaration: increments flow while reads stay buffered.
#[test]
fn scripted_pause_declares_buffering() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let batch_hook: BatchHook<u32> = flow.sim_hook();

    let (get_send, get_requests) = node.sim_input();
    let (inc_send, increments) = node.sim_input::<u64, TotalOrder, ExactlyOnce>();

    let reads_out = sliced! {
        let request_batch = use::batch(get_requests, nondet!(/** scripted */ hook = batch_hook));
        request_batch.count().into_stream()
    }
    .sim_output();
    let incs_out = increments.sim_output();

    flow.sim().deterministic(async || {
        batch_hook.pause();
        get_send.send_many([7, 8]); // reads pile up at the paused batch
        inc_send.send_many([1, 1, 1]);
        incs_out.assert_yields([1u64, 1, 1]).await; // increments flow while reads stay buffered
        batch_hook.release(2).await; // resuming implicitly; both reads in one batch
        reads_out.assert_yields_only([2usize]).await;
    });
}

/// `pause_while` brackets a buffering phase, resuming even if the body panics.
#[test]
fn scripted_pause_while_brackets_buffering() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let batch_hook: BatchHook<u32> = flow.sim_hook();

    let (get_send, get_requests) = node.sim_input();
    let (inc_send, increments) = node.sim_input::<u64, TotalOrder, ExactlyOnce>();

    let reads_out = sliced! {
        let request_batch = use::batch(get_requests, nondet!(/** scripted */ hook = batch_hook));
        request_batch.count().into_stream()
    }
    .sim_output();
    let incs_out = increments.sim_output();

    flow.sim().deterministic(async || {
        batch_hook
            .pause_while(async {
                get_send.send_many([7, 8]);
                inc_send.send_many([1, 1]);
                incs_out.assert_yields([1u64, 1]).await;
            })
            .await;
        batch_hook.release(2).await;
        reads_out.assert_yields_only([2usize]).await;
    });
}

/// `auto_pause` makes buffering the standing rule — missed steps silently hold
/// (deliberately opting out of the forgotten-hook protection), and each scripted decision
/// acts once before the hold is re-established.
#[test]
fn scripted_auto_pause_holds_after_each_decision() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let batch_hook: BatchHook<i32> = flow.sim_hook();

    let (in_send, input) = node.sim_input();
    let out = scripted_batch_sum(input, nondet!(/** scripted */ hook = batch_hook)).sim_output();

    flow.sim().deterministic(async || {
        batch_hook.auto_pause(); // this operator acts only when the script says so
        in_send.send_many([1, 2, 4]);
        batch_hook.release(2).await; // acts once, then is held again automatically
        // The remaining element (4) stays buffered with no declaration needed here.
        out.assert_yields_only([3]).await;
    });
}

/// `pause_until_count` is a declared synchronization point — the hook itself
/// completes the wait once the watermark is reached.
#[test]
fn scripted_pause_until_count() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let batch_hook: BatchHook<i32> = flow.sim_hook();

    let (in_send, input) = node.sim_input();
    let out = scripted_batch_sum(input, nondet!(/** scripted */ hook = batch_hook)).sim_output();

    flow.sim().deterministic(async || {
        in_send.send_many([1, 2, 4]);
        batch_hook.pause_until_count(3).await; // wait until all 3 requests reach the batch
        batch_hook.release(3).await;
        out.assert_yields_only([7]).await;
    });
}

/// A `pause_until_count` watermark that can never be reached is reported at the
/// suspended line rather than hanging.
#[test]
#[should_panic(expected = "pause_until_count(3) can never be satisfied")]
fn scripted_pause_until_count_never_satisfied() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let batch_hook: BatchHook<i32> = flow.sim_hook();

    let (in_send, input) = node.sim_input();
    let _out = scripted_batch_sum(input, nondet!(/** scripted */ hook = batch_hook)).sim_output();

    flow.sim().deterministic(async || {
        in_send.send_many([1, 2]);
        batch_hook.pause_until_count(3).await;
    });
}

/// A decision may be scripted before its data has propagated (or even been
/// sent) — the tick fires at the first moment the decision can be honored in full. The
/// test never needs to synchronize with internal propagation before scripting.
#[test]
fn scripted_decision_before_data_exists() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let batch_hook: BatchHook<i32> = flow.sim_hook();

    let (in_send, input) = node.sim_input();
    let out = scripted_batch_sum(input, nondet!(/** scripted */ hook = batch_hook)).sim_output();

    flow.sim().deterministic(async || {
        batch_hook.release(2).await; // scripted before any data exists
        in_send.send_many([1, 2]);
        out.assert_yields_only([3]).await;
    });
}

/// An output await is a group barrier: it cannot complete while an installed decision can
/// never be honored, and reports that stuck decision once the simulation is quiescent.
#[test]
#[should_panic(expected = "can never be satisfied")]
fn scripted_output_wait_panics_when_decision_is_impossible() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let batch_hook: BatchHook<i32> = flow.sim_hook();

    let (in_send, input) = node.sim_input();
    let out = scripted_batch_sum(input, nondet!(/** scripted */ hook = batch_hook)).sim_output();

    flow.sim().deterministic(async || {
        in_send.send_many([1, 2]);
        batch_hook.release(3).await; // only two elements will ever arrive
        out.next().await; // cannot pass the unconsumed scripted group
    });
}

/// Test-body completion is a final script barrier: a last decision is driven through its
/// tick even when the test does not await output afterward.
#[test]
fn scripted_final_decision_is_consumed() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let batch_hook: BatchHook<i32> = flow.sim_hook();

    let (in_send, input) = node.sim_input();
    let _out = scripted_batch_sum(input, nondet!(/** scripted */ hook = batch_hook)).sim_output();

    flow.sim().deterministic(async || {
        in_send.send_many([1, 2]);
        batch_hook.release_values([1, 2]).await;
        // No output await: returning from the body still waits for this decision to run.
    });
}

/// If a final decision cannot be consumed, test-body completion reports it instead of
/// silently abandoning the pending script.
#[test]
#[should_panic(expected = "a scripted decision can never be satisfied")]
fn scripted_impossible_final_decision_panics() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let batch_hook: BatchHook<i32> = flow.sim_hook();

    let (in_send, input) = node.sim_input();
    let _out = scripted_batch_sum(input, nondet!(/** scripted */ hook = batch_hook)).sim_output();

    flow.sim().deterministic(async || {
        in_send.send_many([1, 2]);
        batch_hook.release(3).await;
        // No later await: completion itself must confront the impossible decision.
    });
}

/// A later-group decision cannot skip past an earlier decision that can never be honored.
/// The first decision installs immediately; the second decision targets the same hook again,
/// so its `.await` is a group barrier and reports the stuck first group at quiescence.
#[test]
#[should_panic(expected = "a previously scripted decision can never be satisfied")]
fn scripted_later_group_panics_when_prior_decision_is_impossible() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let batch_hook: BatchHook<i32> = flow.sim_hook();

    let (in_send, input) = node.sim_input();
    let _out = scripted_batch_sum(input, nondet!(/** scripted */ hook = batch_hook)).sim_output();

    flow.sim().deterministic(async || {
        in_send.send_many([1, 2]);
        batch_hook.release(3).await; // first group: only two elements will ever arrive
        batch_hook.release_empty().await; // second group: cannot pass the stuck first group
    });
}

/// `reveal(value)` is a combined assertion and release. If the versions the state
/// passes through never match, the mis-synchronization fails loudly at the script (with
/// the buffered versions in the message), not three assertions later.
#[test]
#[should_panic(expected = "can never be satisfied")]
fn scripted_reveal_mismatch_panics() {
    let CounterSim {
        inc_send,
        get_send,
        out,
        batch_hook,
        snapshot_hook,
        flow,
    } = counter_sim();

    flow.sim().deterministic(async || {
        inc_send.send_many([1, 1]);
        get_send.send(0);

        snapshot_hook.reveal(5u64).await; // the count only ever passes through 1 and 2
        batch_hook.release(1).await;
        out.next().await;
    });
}

/// Scripting a decision on a handle that was never bound to an operator panics at
/// that call (creating a handle without binding it is allowed, for partially-used bundles).
#[test]
#[should_panic(expected = "not bound to any operator")]
fn scripted_unbound_handle_panics() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let bound: BatchHook<i32> = flow.sim_hook();
    let unbound: BatchHook<i32> = flow.sim_hook(); // created but never attached

    let (in_send, input) = node.sim_input();
    let out = scripted_batch_sum(input, nondet!(/** scripted */ hook = bound)).sim_output();

    flow.sim().deterministic(async || {
        in_send.send(1);
        unbound.release(1).await;
        out.next().await;
    });
}

/// Binding the same handle to two different operators is an error at flow build
/// time, reported with both operator locations.
#[test]
#[should_panic(expected = "bound to two different operators")]
fn scripted_double_bind_panics_at_build() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let batch_hook: BatchHook<i32> = flow.sim_hook();

    let (_in_send, input) = node.sim_input::<i32, TotalOrder, ExactlyOnce>();
    let (_in_send2, input2) = node.sim_input::<i32, TotalOrder, ExactlyOnce>();

    let _out = scripted_batch_sum(input, nondet!(/** scripted */ hook = batch_hook)).sim_output();
    let _out2 = scripted_batch_sum(input2, nondet!(/** scripted */ hook = batch_hook)).sim_output();

    flow.sim().deterministic(async || {});
}

/// In deterministic mode, an unhooked unsafe operator that receives meaningful
/// input panics, naming the operator — silently substituting some fixed behavior would
/// fake coverage. This is also what happens when a program change introduces a *new*
/// unsafe operator under an existing deterministic test.
#[test]
#[should_panic(expected = "not bound to a sim hook")]
fn deterministic_unhooked_operator_panics() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();

    let (in_send, input) = node.sim_input();
    let out = sliced! {
        let batch = use::batch(input, nondet!(/** unhooked! */));
        batch.fold(q!(|| 0), q!(|acc, v: i32| *acc += v)).into_stream()
    }
    .sim_output();

    flow.sim().deterministic(async || {
        in_send.send_many([1, 2]);
        out.next().await;
    });
}

/// `release_all` is well-defined in deterministic mode — ticks
/// fire only after async propagation has fully quiesced, so "everything that has arrived"
/// is exactly the set causally available at this point in the script.
#[test]
fn scripted_release_all_deterministic() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let batch_hook: BatchHook<i32> = flow.sim_hook();

    let (in_send, input) = node.sim_input();
    let out = scripted_batch_sum(input, nondet!(/** scripted */ hook = batch_hook)).sim_output();

    flow.sim().deterministic(async || {
        in_send.send_many([1, 2, 4]);
        batch_hook.release_all().await;
        out.assert_yields_only([7]).await;
    });
}

/// On ordered streams, `release_values` both asserts and releases the exact next prefix.
/// A matching but incomplete prefix waits until all expected values have arrived.
#[test]
fn scripted_release_values_ordered() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let batch_hook: BatchHook<i32> = flow.sim_hook();

    let (in_send, input) = node.sim_input();
    let out = sliced! {
        let batch = use::batch(input, nondet!(/** scripted */ hook = batch_hook));
        batch
    }
    .sim_output();

    flow.sim().deterministic(async || {
        batch_hook.release_values([10, 20]).await;
        in_send.send_many([10, 20, 30]);
        batch_hook.pause(); // 30 deliberately stays buffered
        out.assert_yields_only([10, 20]).await;
    });
}

/// An ordered value decision fails as soon as an available prefix value differs from the
/// script rather than waiting forever for an impossible sequence.
#[test]
#[should_panic(expected = "did not match the expected value")]
fn scripted_release_values_ordered_mismatch_panics() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let batch_hook: BatchHook<i32> = flow.sim_hook();

    let (in_send, input) = node.sim_input();
    let out = sliced! {
        let batch = use::batch(input, nondet!(/** scripted */ hook = batch_hook));
        batch
    }
    .sim_output();

    flow.sim().deterministic(async || {
        in_send.send_many([10, 99]);
        batch_hook.release_values([10, 20]).await;
        out.next().await;
    });
}

/// On unordered streams, batch *contents* (not just counts) are a choice —
/// `release_values` selects a value multiset independently of incidental buffer order.
#[test]
fn scripted_release_values_unordered() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let batch_hook: BatchHook<i32, NoOrder> = flow.sim_hook();

    let (in_send, input) = node.sim_input::<i32, NoOrder, ExactlyOnce>();
    let out = sliced! {
        let batch = use::batch(input, nondet!(/** scripted */ hook = batch_hook));
        batch
    }
    .sim_output();

    flow.sim().deterministic(async || {
        in_send.send_many_unordered([10, 20, 40, 10]);
        batch_hook.release_values([40, 10, 10]).await; // leave only 20 buffered
        batch_hook.pause(); // 20 deliberately stays buffered
        out.assert_yields_only_unordered([40, 10, 10]).await;
    });
}

/// `keep()` re-observes the previously revealed version even while newer versions
/// are buffered, and `reveal_latest()` catches up to the newest version.
#[test]
fn scripted_snapshot_keep_and_reveal_latest() {
    let CounterSim {
        inc_send,
        get_send,
        out,
        batch_hook,
        snapshot_hook,
        flow,
    } = counter_sim();

    flow.sim().deterministic(async || {
        inc_send.send_many([1, 1, 1]);
        get_send.send_many([7, 8, 9]);

        snapshot_hook.reveal(1u64).await; // 1st execution observes count = 1
        batch_hook.release(1).await;

        snapshot_hook.keep().await; // 2nd execution re-observes count = 1
        batch_hook.release(1).await;

        snapshot_hook.reveal_latest().await; // 3rd execution catches up to count = 3
        batch_hook.release(1).await;

        out.assert_yields_only([(7, 1u64), (8, 1u64), (9, 3u64)])
            .await;
    });
}

/// The only implicit behaviors are the *forced* ones. A scripted hook with an empty
/// buffer contributes an empty batch when its tick runs for another hook — requiring the
/// test to write `release_empty()` for every such execution would be pure noise.
#[test]
fn scripted_implicit_empty_batch() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let first_hook: BatchHook<i32> = flow.sim_hook();
    let second_hook: BatchHook<i32> = flow.sim_hook();

    let (in1_send, in1) = node.sim_input();
    let (_in2_send, in2) = node.sim_input::<i32, TotalOrder, ExactlyOnce>();

    let out = sliced! {
        let batch1 = use::batch(in1, nondet!(/** scripted */ hook = first_hook));
        let batch2 = use::batch(in2, nondet!(/** scripted */ hook = second_hook));
        batch1.chain(batch2).fold(q!(|| 0), q!(|acc, v| *acc += v)).into_stream()
    }
    .sim_output();

    flow.sim().deterministic(async || {
        in1_send.send_many([1, 2]);
        // Only batch1 is scripted; batch2 has an empty buffer, so contributing an empty
        // batch is the only thing it could possibly do (no `release_empty()` needed).
        first_hook.release(2).await;
        out.assert_yields_only([3]).await;
    });
}

/// A group containing only trivial decisions cannot make its tick runnable. The simulator
/// reports the unconsumable group instead of executing an empty tick or silently finishing.
#[test]
#[should_panic(expected = "scripted decision group contains only trivial decisions")]
fn scripted_all_trivial_decisions_panic() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let first_hook: BatchHook<i32> = flow.sim_hook();
    let second_hook: BatchHook<i32> = flow.sim_hook();

    let (_first_send, first_input) = node.sim_input();
    let (_second_send, second_input) = node.sim_input();
    let _out = sliced! {
        let first_batch = use::batch(first_input, nondet!(/** scripted */ hook = first_hook));
        let second_batch = use::batch(second_input, nondet!(/** scripted */ hook = second_hook));
        first_batch.chain(second_batch)
    }
    .sim_output();

    flow.sim().deterministic(async || {
        first_hook.release_empty().await;
        second_hook.release_empty().await;
    });
}

/// Groups spanning two different ticks are executed in script order, with the
/// second tick's decision waiting for data that only exists once the first tick has run.
#[test]
fn scripted_groups_across_ticks() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let first_hook: BatchHook<i32> = flow.sim_hook();
    let second_hook: BatchHook<i32> = flow.sim_hook();

    let (in_send, input) = node.sim_input();
    let stage1 = scripted_batch_sum(input, nondet!(/** scripted */ hook = first_hook));
    let out = scripted_batch_sum(stage1, nondet!(/** scripted */ hook = second_hook)).sim_output();

    flow.sim().deterministic(async || {
        in_send.send_many([1, 2]);
        first_hook.release(2).await; // 1st tick: sum = 3
        second_hook.release(1).await; // 2nd tick: waits for the 3 produced by the 1st
        out.assert_yields_only([3]).await;
    });
}

/// Scripting tick B's decision while tick A's group is still outstanding must not shield
/// B's hook from the boundary scan: if B already has buffered input *before* A runs, that
/// input is buffered across a scheduling boundary with neither a decision nor an explicit
/// pause, and the forgotten-hook error must fire.
#[test]
#[should_panic(expected = "scripted hook has buffered input but no decision")]
fn scripted_waiting_group_does_not_shield_buffered_input() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let first_hook: BatchHook<i32> = flow.sim_hook();
    let second_hook: BatchHook<i32> = flow.sim_hook();

    let (first_send, first_input) = node.sim_input();
    let _first_out =
        scripted_batch_sum(first_input, nondet!(/** scripted */ hook = first_hook)).sim_output();
    let (second_send, second_input) = node.sim_input();
    let _second_out =
        scripted_batch_sum(second_input, nondet!(/** scripted */ hook = second_hook)).sim_output();

    flow.sim().deterministic(async || {
        first_send.send(1);
        second_send.send(10); // buffered at the second tick before the first tick runs
        first_hook.release(1).await;
        // Waits behind the first group while the second tick's input is already buffered:
        // that buffering must be declared with a pause, not silently exempted.
        second_hook.release(1).await;
    });
}

/// The consequential variant of
/// [`scripted_waiting_group_does_not_shield_buffered_input`]: tick A is *partially
/// unscripted* (a fuzzed batch fed by a network cycle from tick B), and tick B is pre-fed
/// before A's group is scripted. Script order is the schedule, so scripting A before B
/// forces A to execute before B — which means A's fuzzed hook can never observe B's
/// cycled-back value. If the waiting decision exempted B's pre-fed input from the
/// boundary scan (the old hold), that pruning of the search space would be *silent*.
/// Instead the developer is confronted and must decide explicitly: `pause()` B to accept
/// the exclusion, or script B first (see the companion test below) so the fuzzer can
/// explore A observing the cycled value.
#[test]
#[should_panic(expected = "scripted hook has buffered input but no decision")]
fn scripted_prefed_input_behind_waiting_group_confronts_schedule_pruning() {
    use crate::networking::TCP;

    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let node2 = flow.process::<()>();
    let a_hook: BatchHook<u32> = flow.sim_hook();
    let b_hook: BatchHook<u32> = flow.sim_hook();

    let (a_send, a_input) = node.sim_input::<u32, TotalOrder, _>();
    let (b_send, b_input) = node.sim_input::<u32, TotalOrder, _>();

    let (complete_cycle, cycled) = node.forward_ref::<Stream<_, _, _, NoOrder>>();
    let b_out = sliced! {
        let released = use::batch(b_input, nondet!(/** scripted */ hook = b_hook));
        released
    };
    complete_cycle.complete(
        b_out
            .map(q!(|v| v + 100))
            .send(&node2, TCP.fail_stop().bincode())
            .send(&node, TCP.fail_stop().bincode()),
    );

    let _a_out = sliced! {
        let scripted = use::batch(a_input, nondet!(/** scripted */ hook = a_hook));
        let fuzzed = use::batch(cycled, nondet!(/** fuzzed: whether A observes the cycle */));
        scripted.cross_singleton(fuzzed.count())
    }
    .sim_output();

    flow.sim().exhaustive(async || {
        b_send.send(10); // pre-fed at B before A's group is scripted
        a_send.send(1);
        a_hook.release(1).await;
        // Waits behind A's group; B's pre-fed 10 is buffered across A's execution without
        // a pause, silently excluding every schedule where A's fuzzed hook sees B's 110.
        b_hook.release(1).await;
    });
}

/// The companion to
/// [`scripted_prefed_input_behind_waiting_group_confronts_schedule_pruning`], using the
/// explicit resolution: `pause()` on B declares that its pre-fed input intentionally
/// waits behind A's group, accepting that A's fuzzed hook never observes the cycled value
/// (every instance yields exactly `(1, 0)`).
///
/// This also pins a load-bearing pause semantic: the pause is released only when the
/// queued decision *installs* — which happens strictly after the previous group has
/// executed — not when the decision is first scripted. If the wait released the pause
/// early, the boundary scan would confront B's buffered input mid-wait and this test
/// would panic.
#[test]
fn scripted_pause_spans_waiting_group_and_releases_on_install() {
    use crate::networking::TCP;

    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let node2 = flow.process::<()>();
    let a_hook: BatchHook<u32> = flow.sim_hook();
    let b_hook: BatchHook<u32> = flow.sim_hook();

    let (a_send, a_input) = node.sim_input::<u32, TotalOrder, _>();
    let (b_send, b_input) = node.sim_input::<u32, TotalOrder, _>();

    let (complete_cycle, cycled) = node.forward_ref::<Stream<_, _, _, NoOrder>>();
    let b_out = sliced! {
        let released = use::batch(b_input, nondet!(/** scripted */ hook = b_hook));
        released
    };
    complete_cycle.complete(
        b_out
            .map(q!(|v| v + 100))
            .send(&node2, TCP.fail_stop().bincode())
            .send(&node, TCP.fail_stop().bincode()),
    );

    let a_out = sliced! {
        let scripted = use::batch(a_input, nondet!(/** scripted */ hook = a_hook));
        let fuzzed = use::batch(cycled, nondet!(/** fuzzed: whether A observes the cycle */));
        scripted.cross_singleton(fuzzed.count())
    }
    .sim_output();

    flow.sim().exhaustive(async || {
        b_send.send(10);
        a_send.send(1);
        // Declares the exclusion the panic variant confronts: B's pre-fed input waits
        // behind A's group on purpose. The pause must hold through the entire wait below.
        b_hook.pause();
        a_hook.release(1).await;
        // Waits behind A's group; installing (after A has run) releases the pause.
        b_hook.release(1).await;

        // A always executed before B, so its fuzzed hook never saw the cycled 110: the
        // pruning is total, and *declared*.
        let outs: Vec<(u32, usize)> = a_out.collect().await;
        assert_eq!(outs, [(1, 0)]);
    });
}

/// A decision for a different tick also starts a new group and cannot pass an earlier
/// impossible group. The later tick could otherwise make progress independently, so this
/// specifically verifies the global script-order barrier across ticks.
#[test]
#[should_panic(expected = "a previously scripted decision can never be satisfied")]
fn scripted_different_tick_panics_when_prior_decision_is_impossible() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let stuck_hook: BatchHook<i32> = flow.sim_hook();
    let independent_hook: BatchHook<i32> = flow.sim_hook();

    let (stuck_send, stuck_input) = node.sim_input();
    let _stuck_out =
        scripted_batch_sum(stuck_input, nondet!(/** scripted */ hook = stuck_hook)).sim_output();
    let (_independent_send, independent_input) = node.sim_input();
    let _independent_out = scripted_batch_sum(
        independent_input,
        nondet!(/** scripted */ hook = independent_hook),
    )
    .sim_output();

    flow.sim().deterministic(async || {
        stuck_send.send_many([1, 2]);
        stuck_hook.release(3).await; // first group can never be honored
        // The independent tick has no buffered input, so the boundary scan stays quiet;
        // the script-order barrier itself reports the stuck first group here.
        independent_hook.release(1).await; // different tick, but cannot pass the first group
    });
}

/// A component with several unsafe operators can expose them through a single
/// guard with a composite (tuple) hook payload, letting the caller hook some, all, or
/// none of them. The component splits the payload with `NonDet::take_hook` and attaches
/// each part to the operator it controls.
#[test]
fn scripted_composite_hook_payload() {
    fn two_stage_pipeline<'a>(
        input: Stream<i32, Process<'a>>,
        mut nondet_stages: NonDet<(Option<BatchHook<i32>>, Option<BatchHook<i32>>)>,
    ) -> Stream<i32, Process<'a>> {
        let (first_hook, second_hook) = nondet_stages.take_hook();
        let stage1 = sliced! {
            let batch = use::batch(input, nondet!(
                /// stage 1 batching
                hook = first_hook
            ));
            batch.fold(q!(|| 0), q!(|acc, v| *acc += v)).into_stream()
        };
        sliced! {
            let batch = use::batch(stage1, nondet!(
                /// stage 2 batching
                hook = second_hook
            ));
            batch.fold(q!(|| 0), q!(|acc, v| *acc += v)).into_stream()
        }
    }

    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let first_hook: BatchHook<i32> = flow.sim_hook();
    let second_hook: BatchHook<i32> = flow.sim_hook();

    let (in_send, input) = node.sim_input();
    // Both stages hooked through one guard; `(first_hook.into(), None)` would hook only
    // the first.
    let out = two_stage_pipeline(
        input,
        nondet!(/** scripted */ hook = (Some(first_hook), Some(second_hook))),
    )
    .sim_output();

    flow.sim().deterministic(async || {
        in_send.send_many([1, 2]);
        first_hook.release(2).await; // 1st stage: sum = 3
        second_hook.release(1).await; // 2nd stage: consumes the 3
        out.assert_yields_only([3]).await;
    });
}

/// A fully scripted program with exact decisions removes every dimension from the
/// search space — `exhaustive` explores exactly one execution.
#[test]
fn scripted_exact_script_single_execution() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let batch_hook: BatchHook<i32> = flow.sim_hook();

    let (in_send, input) = node.sim_input();
    let out = scripted_batch_sum(input, nondet!(/** scripted */ hook = batch_hook)).sim_output();

    let instances = flow.sim().exhaustive(async || {
        in_send.send_many([1, 2]);
        batch_hook.release(2).await;
        out.assert_yields([3]).await;
    });

    assert_eq!(instances, 1);
}

/// Scripting composes with fuzzing — the scripted hook is pinned (its batch is
/// identical in every instance) while the unhooked hook's dimension remains fully
/// explored by the exhaustive engine.
#[test]
fn scripted_hook_composes_with_fuzzing() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let batch_hook: BatchHook<i32> = flow.sim_hook();

    let (in1_send, in1) = node.sim_input();
    let (in2_send, in2) = node.sim_input::<i32, TotalOrder, ExactlyOnce>();

    let scripted_out =
        scripted_batch_sum(in1, nondet!(/** scripted */ hook = batch_hook)).sim_output();
    let fuzzed_out = sliced! {
        let batch = use::batch(in2, nondet!(/** fuzzed */));
        batch.fold(q!(|| 0), q!(|acc, v| *acc += v)).into_stream()
    }
    .sim_output();

    let mut saw_one_fuzzed_batch = false;
    let mut saw_split_fuzzed_batches = false;

    flow.sim().exhaustive(async || {
        in1_send.send_many([1, 2]);
        in2_send.send_many([10, 20]);

        batch_hook.release(2).await;
        // The scripted batch releases identical contents in every fuzz instance.
        scripted_out.assert_yields([3]).await;

        // The unhooked batch remains fully explored: drain whatever batching the engine
        // chose for it.
        let fuzzed: Vec<i32> = fuzzed_out.collect().await;
        match fuzzed.as_slice() {
            [30] => saw_one_fuzzed_batch = true,
            [10, 20] => saw_split_fuzzed_batches = true,
            other => panic!("unexpected fuzzed batching: {:?}", other),
        }
    });

    // The unhooked hook's dimension was genuinely explored around the pinned script.
    assert!(saw_one_fuzzed_batch, "never explored a single fuzzed batch");
    assert!(
        saw_split_fuzzed_batches,
        "never explored split fuzzed batches"
    );
}

/// With a `release_all` decision in the script, *when* the scripted tick fires
/// relative to the fuzzed ticks around it changes *what* it releases — so the wait for a
/// scripted execution is a genuinely free-running wait, and every legal placement of the
/// scripted execution among the fuzzed ones is explored (never forced to fire at the
/// earliest possible moment).
#[test]
fn scripted_release_all_placement_explored() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let batch_hook: BatchHook<i32> = flow.sim_hook();

    let (in_send, input) = node.sim_input();

    // A fuzzed tick produces per-batch sums that feed the scripted batch.
    let fuzzed_stage = sliced! {
        let batch = use::batch(input, nondet!(/** fuzzed */));
        batch.fold(q!(|| 0), q!(|acc, v| *acc += v)).into_stream()
    };
    let out =
        scripted_batch_sum(fuzzed_stage, nondet!(/** scripted */ hook = batch_hook)).sim_output();

    let mut saw_early_firing = false; // released before the fuzzed tick's second batch
    let mut saw_late_firing = false; // released everything the fuzzed tick produced

    flow.sim().exhaustive(async || {
        in_send.send_many([1, 2]);

        batch_hook.pause_until_count(1).await;
        batch_hook.release_all().await; // "whatever has arrived when the tick fires"
        let released_sum = out.next().await;
        batch_hook.pause(); // anything still in flight is deliberately left buffered

        match released_sum {
            1 => saw_early_firing = true,
            3 => saw_late_firing = true,
            other => panic!("unexpected release_all() contents summing to {}", other),
        }
    });

    assert!(
        saw_early_firing,
        "never explored the placement before the fuzzed tick's second batch"
    );
    assert!(
        saw_late_firing,
        "never explored the placement after all fuzzed batches"
    );
}

/// Which version `reveal_latest()` observes co-varies with the fuzzed schedule around
/// the scripted tick, so under exhaustive exploration every causally-possible observed
/// version is explored (while the paired exact `release(1)` stays pinned).
#[test]
fn scripted_reveal_latest_contents_explored() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let batch_hook: BatchHook<u32> = flow.sim_hook();
    let snapshot_hook: SnapshotHook<i32> = flow.sim_hook();

    let (in2_send, in2) = node.sim_input::<i32, TotalOrder, ExactlyOnce>();
    let (get_send, gets) = node.sim_input::<u32, TotalOrder, ExactlyOnce>();

    // A fuzzed tick whose firings update the state the scripted snapshot observes.
    let t2_out = sliced! {
        let batch = use::batch(in2, nondet!(/** fuzzed */));
        batch.fold(q!(|| 0), q!(|acc, v| *acc += v)).into_stream()
    };
    let count_state = t2_out.fold(q!(|| 0), q!(|acc, v| *acc += v));

    let out = sliced! {
        let request_batch = use::batch(gets, nondet!(/** scripted */ hook = batch_hook));
        let snapshot = use::snapshot(count_state, nondet!(/** scripted */ hook = snapshot_hook));
        request_batch.cross_singleton(snapshot)
    }
    .sim_output();

    let mut saw_initial_version = false;
    let mut saw_final_version = false;

    flow.sim().exhaustive(async || {
        in2_send.send_many([1, 2]);
        get_send.send(7);

        snapshot_hook.reveal_latest().await; // "the newest version that has arrived"
        batch_hook.release(1).await; // exact: pinned in every instance

        let (get, observed) = out.next().await;
        assert_eq!(get, 7);
        // Versions produced by fuzzed firings after the observation stay deliberately
        // unobserved.
        snapshot_hook.pause();

        match observed {
            0 => saw_initial_version = true, // fired before any fuzzed batch
            1 => {}                          // between the fuzzed tick's two batches
            3 => saw_final_version = true,   // after everything
            other => panic!("unexpected observed version {}", other),
        }
    });

    assert!(
        saw_initial_version,
        "never explored the placement before the fuzzed tick fired"
    );
    assert!(
        saw_final_version,
        "never explored the placement after all fuzzed firings"
    );
}

/// An unscripted (fuzzed) tick T2 that feeds a scripted
/// hook H cannot slip data past the boundary scan — the exploration itself surfaces the
/// missed pairing as a loud panic, instead of T1's implicit behavior silently standing.
#[test]
#[should_panic(expected = "scripted hook has buffered input but no decision")]
fn scripted_t1_t2_fuzzed_feeder_is_confronted() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let batch_hook: BatchHook<u32> = flow.sim_hook();
    let snapshot_hook: SnapshotHook<i32> = flow.sim_hook();

    let (in2_send, in2) = node.sim_input::<i32, TotalOrder, ExactlyOnce>();
    let (get_send, gets) = node.sim_input::<u32, TotalOrder, ExactlyOnce>();

    // T2: an unhooked (fuzzed) batch whose output updates the state H observes.
    let t2_out = sliced! {
        let batch = use::batch(in2, nondet!(/** fuzzed */));
        batch.fold(q!(|| 0), q!(|acc, v| *acc += v)).into_stream()
    };
    let count_state = t2_out.fold(q!(|| 0), q!(|acc, v| *acc += v));

    // T1: a scripted tick observing that state.
    let out = sliced! {
        let request_batch = use::batch(gets, nondet!(/** scripted */ hook = batch_hook));
        let snapshot = use::snapshot(count_state, nondet!(/** scripted */ hook = snapshot_hook));
        request_batch.cross_singleton(snapshot)
    }
    .sim_output();

    flow.sim().exhaustive(async || {
        in2_send.send(10);
        get_send.send(7);

        // Script T1's first execution against the initial state...
        snapshot_hook.reveal(0i32).await;
        batch_hook.release(1).await;
        out.assert_yields([(7u32, 0i32)]).await;

        // ...then move on without saying what H should do with the version T2's firing
        // produces. Instances where T2 fires leave H holding an unaccounted version, and
        // the boundary scan confronts it.
        out.assert_no_more().await;
    });
}

/// The dual of [`scripted_t1_t2_fuzzed_feeder_is_confronted`]: acknowledging the fuzzed
/// feeder's data (here with a `pause()`) makes the same schedule pass in every instance.
#[test]
fn scripted_t1_t2_fuzzed_feeder_acknowledged() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let batch_hook: BatchHook<u32> = flow.sim_hook();
    let snapshot_hook: SnapshotHook<i32> = flow.sim_hook();

    let (in2_send, in2) = node.sim_input::<i32, TotalOrder, ExactlyOnce>();
    let (get_send, gets) = node.sim_input::<u32, TotalOrder, ExactlyOnce>();

    let t2_out = sliced! {
        let batch = use::batch(in2, nondet!(/** fuzzed */));
        batch.fold(q!(|| 0), q!(|acc, v| *acc += v)).into_stream()
    };
    let count_state = t2_out.fold(q!(|| 0), q!(|acc, v| *acc += v));

    let out = sliced! {
        let request_batch = use::batch(gets, nondet!(/** scripted */ hook = batch_hook));
        let snapshot = use::snapshot(count_state, nondet!(/** scripted */ hook = snapshot_hook));
        request_batch.cross_singleton(snapshot)
    }
    .sim_output();

    flow.sim().exhaustive(async || {
        in2_send.send(10);
        get_send.send(7);

        snapshot_hook.reveal(0i32).await;
        batch_hook.release(1).await;
        out.assert_yields([(7u32, 0i32)]).await;

        // Declare that any later versions produced by T2's (fuzzed) firing are
        // deliberately left unobserved.
        snapshot_hook.pause();
        out.assert_no_more().await;
    });
}

// ============================================================================
// Keyed and merge hooks: scripting the keyed batch / snapshot / ordering /
// partially-ordered / merge-ordered operator kinds.
// ============================================================================

/// A keyed batch over totally ordered values is scripted with `release_values`, naming
/// `(key, value)` entries: each key's values must match that key's buffered prefix in
/// order, while the cross-key interleaving in the script is irrelevant. Ticks consume
/// one decision each.
#[test]
fn scripted_keyed_batch_release_values_ordered() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let batch_hook: KeyedBatchHook<u32, char> = flow.sim_hook();
    let (in_send, input) = node.sim_input::<(u32, char), TotalOrder, ExactlyOnce>();
    let output = sliced! {
        let b = use::batch(input.into_keyed(), nondet!(/** scripted */ hook = batch_hook));
        b.entries().sort().collect_vec().into_stream()
    }
    .sim_output();

    flow.sim().deterministic(async || {
        in_send.send_many([(1, 'a'), (2, 'x'), (1, 'b')]);
        batch_hook.release_values([(1, 'a'), (2, 'x')]).await;
        batch_hook.release_values([(1, 'b')]).await;
        output
            .assert_yields_only([vec![(1, 'a'), (2, 'x')], vec![(1, 'b')]])
            .await;
    });
}

/// A keyed `release_values` that names a value out of its key's buffered order fails
/// loudly: within a key, values must match the buffered prefix in order.
#[test]
#[should_panic(expected = "did not match the expected value")]
fn scripted_keyed_batch_release_values_ordered_mismatch_panics() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let batch_hook: KeyedBatchHook<u32, char> = flow.sim_hook();
    let (in_send, input) = node.sim_input::<(u32, char), TotalOrder, ExactlyOnce>();
    let _output = sliced! {
        let b = use::batch(input.into_keyed(), nondet!(/** scripted */ hook = batch_hook));
        b.entries().sort().collect_vec().into_stream()
    }
    .sim_output();

    flow.sim().deterministic(async || {
        in_send.send_many([(1, 'a'), (1, 'b')]);
        batch_hook.release_values([(1, 'b')]).await;
    });
}

/// A keyed batch over unordered values matches `release_values` per key as multisets:
/// entries can be named independently of their arrival order within each key.
#[test]
fn scripted_keyed_batch_release_values_unordered() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let batch_hook: KeyedBatchHook<u32, char, NoOrder> = flow.sim_hook();
    let (in_send, input) = node.sim_input::<(u32, char), NoOrder, ExactlyOnce>();
    let output = sliced! {
        let b = use::batch(input.into_keyed(), nondet!(/** scripted */ hook = batch_hook));
        b.entries().sort().collect_vec().into_stream()
    }
    .sim_output();

    flow.sim().deterministic(async || {
        in_send.send_many_unordered([(1, 'a'), (1, 'b'), (2, 'x')]);
        // 'b' is named before 'a' even though it arrived after: unordered values are
        // matched as per-key multisets.
        batch_hook.release_values([(1, 'b'), (2, 'x')]).await;
        batch_hook.release_values([(1, 'a')]).await;
        output
            .assert_yields_only([vec![(1, 'b'), (2, 'x')], vec![(1, 'a')]])
            .await;
    });
}

/// A keyed snapshot is scripted with `reveal`, naming the version each key observes;
/// unnamed keys observe their previously revealed version again.
#[test]
fn scripted_keyed_snapshot_reveal_advances_named_keys() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let snapshot_hook: KeyedSnapshotHook<u32, u32> = flow.sim_hook();
    let (in_send, input) = node.sim_input::<(u32, u32), TotalOrder, ExactlyOnce>();
    let folded = input.into_keyed().fold(q!(|| 0u32), q!(|acc, v| *acc += v));
    let output = sliced! {
        let snap = use::snapshot(folded, nondet!(/** scripted */ hook = snapshot_hook));
        snap.entries().sort().collect_vec().into_stream()
    }
    .sim_output();

    flow.sim().deterministic(async || {
        in_send.send((1, 1));
        snapshot_hook.reveal([(1, 1)]).await;

        in_send.send((2, 5));
        // Key 1 is not named: it observes its previously revealed version again.
        snapshot_hook.reveal([(2, 5)]).await;

        in_send.send((1, 2));
        snapshot_hook.reveal([(1, 3)]).await;

        output
            .assert_yields_only([vec![(1, 1)], vec![(1, 1), (2, 5)], vec![(1, 3), (2, 5)]])
            .await;
    });
}

/// A top-level keyed `assume_ordering` releases one named `(key, value)` entry per
/// decision, and a top-level `entries_partially_ordered` names the front entry of one
/// key's buffer per decision; scripting both pins down the exact output sequence.
#[test]
fn scripted_top_level_keyed_ordering_and_partial_ordering() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let ordering: KeyedOrderingHook<u32, u32> = flow.sim_hook();
    let partial: PartialOrderingHook<u32, u32> = flow.sim_hook();
    let (in_send, input) = node.sim_input::<(u32, u32), NoOrder, ExactlyOnce>();
    let output = input
        .into_keyed()
        .assume_ordering::<TotalOrder>(nondet!(/** scripted */ hook = ordering))
        .entries_partially_ordered(nondet!(/** scripted */ hook = partial))
        .sim_output();

    flow.sim().deterministic(async || {
        in_send.send_many_unordered([(1, 10), (1, 20), (2, 30)]);

        // Entries released by the ordering hook buffer at the downstream
        // partially-ordered hook until its decisions run; declare that standing
        // buffering once.
        partial.auto_pause();

        // The keyed ordering hook may release a key's values in any order (the input is
        // unordered within each key); each decision names exactly one entry.
        ordering.next(1, 20).await;
        ordering.next(2, 30).await;
        ordering.next(1, 10).await;

        // The partially-ordered hook must preserve the (now total) within-key order and
        // scripts the cross-key interleaving.
        partial.next(1, 20).await;
        partial.next(2, 30).await;
        partial.next(1, 10).await;

        output.assert_yields_only([(1, 20), (2, 30), (1, 10)]).await;
    });
}

/// A top-level `entries_partially_ordered` decision that names a value other than the
/// front of its key's buffer can never be honored (within-key order is preserved), and
/// fails loudly.
#[test]
#[should_panic(expected = "did not match the front")]
fn scripted_top_level_partial_ordering_front_mismatch_panics() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let partial: PartialOrderingHook<u32, u32> = flow.sim_hook();
    let (in_send, input) = node.sim_input::<(u32, u32), TotalOrder, ExactlyOnce>();
    let _output = input
        .into_keyed()
        .entries_partially_ordered(nondet!(/** scripted */ hook = partial))
        .sim_output();

    flow.sim().deterministic(async || {
        in_send.send_many([(1, 10), (1, 20)]);
        partial.next(1, 20).await;
    });
}

/// An in-tick keyed `assume_ordering` is scripted with a single `order` decision naming
/// all of the tick's entries; each key's values are released in the scripted per-key
/// order.
#[test]
fn scripted_inline_keyed_ordering_orders_within_keys() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let batch_hook: KeyedBatchHook<u32, u32, NoOrder> = flow.sim_hook();
    let ordering: KeyedOrderingHook<u32, u32, Bounded> = flow.sim_hook();
    let (in_send, input) = node.sim_input::<(u32, u32), NoOrder, ExactlyOnce>();
    let output = sliced! {
        let b = use::batch(input.into_keyed(), nondet!(/** scripted */ hook = batch_hook));
        b.assume_ordering::<TotalOrder>(nondet!(/** scripted */ hook = ordering))
            .fold(q!(|| vec![]), q!(|acc, v| acc.push(v)))
            .entries()
    }
    .sim_output();

    flow.sim().deterministic(async || {
        in_send.send_many_unordered([(1, 10), (1, 20), (2, 30)]);
        batch_hook.release_values([(1, 10), (1, 20), (2, 30)]).await;
        ordering.order([(1, 20), (2, 30), (1, 10)]).await;
        output
            .assert_yields_only_unordered([(1, vec![20, 10]), (2, vec![30])])
            .await;
    });
}

/// An in-tick `entries_partially_ordered` is scripted with a single `order` decision
/// supplying the complete interleaving, which must preserve each key's within-key order.
#[test]
fn scripted_inline_partial_ordering_interleaves_keys() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let batch_hook: KeyedBatchHook<u32, u32> = flow.sim_hook();
    let partial: PartialOrderingHook<u32, u32, Bounded> = flow.sim_hook();
    let (in_send, input) = node.sim_input::<(u32, u32), TotalOrder, ExactlyOnce>();
    let output = sliced! {
        let b = use::batch(input.into_keyed(), nondet!(/** scripted */ hook = batch_hook));
        b.entries_partially_ordered(nondet!(/** scripted */ hook = partial))
            .collect_vec()
            .into_stream()
    }
    .sim_output();

    flow.sim().deterministic(async || {
        in_send.send_many([(1, 10), (1, 20), (2, 30)]);
        batch_hook.release_values([(1, 10), (1, 20), (2, 30)]).await;
        partial.order([(2, 30), (1, 10), (1, 20)]).await;
        output
            .assert_yields_only([vec![(2, 30), (1, 10), (1, 20)]])
            .await;
    });
}

/// An in-tick `entries_partially_ordered` decision that breaks a key's within-key order
/// is rejected.
#[test]
#[should_panic(expected = "preserves each key's order")]
fn scripted_inline_partial_ordering_invalid_interleaving_panics() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let batch_hook: KeyedBatchHook<u32, u32> = flow.sim_hook();
    let partial: PartialOrderingHook<u32, u32, Bounded> = flow.sim_hook();
    let (in_send, input) = node.sim_input::<(u32, u32), TotalOrder, ExactlyOnce>();
    let _output = sliced! {
        let b = use::batch(input.into_keyed(), nondet!(/** scripted */ hook = batch_hook));
        b.entries_partially_ordered(nondet!(/** scripted */ hook = partial))
            .collect_vec()
            .into_stream()
    }
    .sim_output();

    flow.sim().deterministic(async || {
        in_send.send_many([(1, 10), (1, 20), (2, 30)]);
        batch_hook.release_values([(1, 10), (1, 20), (2, 30)]).await;
        partial.order([(1, 20), (2, 30), (1, 10)]).await;
    });
}

/// A top-level `merge_ordered` is scripted one element at a time with `next_first` /
/// `next_second` (naming the front of the chosen input's buffer) or `advance_first` /
/// `advance_second` (releasing it without asserting its value).
#[test]
fn scripted_top_level_merge_ordered_interleaves_inputs() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let merge: MergeOrderedHook<u32> = flow.sim_hook();
    let (first_send, first) = node.sim_input::<u32, TotalOrder, ExactlyOnce>();
    let (second_send, second) = node.sim_input::<u32, TotalOrder, ExactlyOnce>();
    let output = first
        .merge_ordered(second, nondet!(/** scripted */ hook = merge))
        .sim_output();

    flow.sim().deterministic(async || {
        first_send.send_many([1, 3]);
        second_send.send_many([2]);
        merge.next_first(1).await;
        merge.advance_second().await;
        merge.next_first(3).await;
        output.assert_yields_only([1, 2, 3]).await;
    });
}

/// A top-level `merge_ordered` decision that names a value other than the front of its
/// input's buffer can never be honored (per-input order is preserved), and fails loudly.
#[test]
#[should_panic(expected = "did not match the front")]
fn scripted_top_level_merge_ordered_front_mismatch_panics() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let merge: MergeOrderedHook<u32> = flow.sim_hook();
    let (first_send, first) = node.sim_input::<u32, TotalOrder, ExactlyOnce>();
    let (_second_send, second) = node.sim_input::<u32, TotalOrder, ExactlyOnce>();
    let _output = first
        .merge_ordered(second, nondet!(/** scripted */ hook = merge))
        .sim_output();

    flow.sim().deterministic(async || {
        first_send.send_many([1, 3]);
        merge.next_first(3).await;
    });
}

/// An in-tick `merge_ordered` is scripted with a single `order` decision supplying the
/// complete merged sequence, which must be an interleaving that preserves each input's
/// order; the decision joins the same script group as the batch decisions feeding the
/// tick.
#[test]
fn scripted_inline_merge_ordered_joins_tick_group() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let first_batch: BatchHook<u32> = flow.sim_hook();
    let second_batch: BatchHook<u32> = flow.sim_hook();
    let merge: MergeOrderedHook<u32, Bounded> = flow.sim_hook();
    let (first_send, first) = node.sim_input::<u32, TotalOrder, ExactlyOnce>();
    let (second_send, second) = node.sim_input::<u32, TotalOrder, ExactlyOnce>();
    let output = sliced! {
        let a = use::batch(first, nondet!(/** scripted */ hook = first_batch));
        let b = use::batch(second, nondet!(/** scripted */ hook = second_batch));
        a.merge_ordered(b, nondet!(/** scripted */ hook = merge))
            .collect_vec()
            .into_stream()
    }
    .sim_output();

    flow.sim().deterministic(async || {
        first_send.send_many([1, 3]);
        second_send.send_many([2]);
        first_batch.release_values([1, 3]).await;
        second_batch.release_values([2]).await;
        merge.order([1, 2, 3]).await;
        output.assert_yields_only([vec![1, 2, 3]]).await;
    });
}

/// An in-tick `merge_ordered` with elements on both sides requires an explicit scripted
/// interleaving rather than falling back to an order the test never chose.
#[test]
#[should_panic(expected = "require an explicit merge order")]
fn scripted_inline_merge_ordered_missing_decision_panics_during_tick() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let first_batch: BatchHook<u32> = flow.sim_hook();
    let second_batch: BatchHook<u32> = flow.sim_hook();
    let merge: MergeOrderedHook<u32, Bounded> = flow.sim_hook();
    let (first_send, first) = node.sim_input::<u32, TotalOrder, ExactlyOnce>();
    let (second_send, second) = node.sim_input::<u32, TotalOrder, ExactlyOnce>();
    let _output = sliced! {
        let a = use::batch(first, nondet!(/** scripted */ hook = first_batch));
        let b = use::batch(second, nondet!(/** scripted */ hook = second_batch));
        a.merge_ordered(b, nondet!(/** scripted */ hook = merge))
            .collect_vec()
            .into_stream()
    }
    .sim_output();

    flow.sim().deterministic(async || {
        first_send.send_many([1]);
        second_send.send_many([2]);
        first_batch.release_values([1]).await;
        second_batch.release_values([2]).await;
    });
}

/// A top-level keyed `merge_ordered` is scripted one entry at a time with `next_first` /
/// `next_second` (naming the front of that key's buffer in the chosen input) or
/// `advance_first` / `advance_second` (releasing a key's front without asserting its
/// value).
#[test]
fn scripted_top_level_keyed_merge_ordered_interleaves_within_key() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let merge: KeyedMergeOrderedHook<u32, u32> = flow.sim_hook();
    let partial: PartialOrderingHook<u32, u32> = flow.sim_hook();
    let (first_send, first) = node.sim_input::<(u32, u32), TotalOrder, ExactlyOnce>();
    let (second_send, second) = node.sim_input::<(u32, u32), TotalOrder, ExactlyOnce>();
    let output = first
        .into_keyed()
        .merge_ordered(second.into_keyed(), nondet!(/** scripted */ hook = merge))
        .entries_partially_ordered(nondet!(/** scripted */ hook = partial))
        .sim_output();

    flow.sim().deterministic(async || {
        first_send.send_many([(1, 10), (1, 20)]);
        second_send.send_many([(1, 30)]);

        // Entries released by the merge hook buffer at the downstream partially-ordered
        // hook until its decisions run; declare that standing buffering once.
        partial.auto_pause();

        merge.next_first(1, 10).await;
        merge.advance_second(1).await;
        merge.next_first(1, 20).await;

        partial.next(1, 10).await;
        partial.next(1, 30).await;
        partial.next(1, 20).await;

        output.assert_yields_only([(1, 10), (1, 30), (1, 20)]).await;
    });
}

/// An in-tick keyed `merge_ordered` is scripted with a single `order` decision; for every
/// key, the scripted subsequence must interleave that key's entries from the two inputs
/// preserving each input's order.
#[test]
fn scripted_inline_keyed_merge_ordered_orders_within_keys() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<()>();
    let first_batch: KeyedBatchHook<u32, u32> = flow.sim_hook();
    let second_batch: KeyedBatchHook<u32, u32> = flow.sim_hook();
    let merge: KeyedMergeOrderedHook<u32, u32, Bounded> = flow.sim_hook();
    let (first_send, first) = node.sim_input::<(u32, u32), TotalOrder, ExactlyOnce>();
    let (second_send, second) = node.sim_input::<(u32, u32), TotalOrder, ExactlyOnce>();
    let output = sliced! {
        let a = use::batch(first.into_keyed(), nondet!(/** scripted */ hook = first_batch));
        let b = use::batch(second.into_keyed(), nondet!(/** scripted */ hook = second_batch));
        a.merge_ordered(b, nondet!(/** scripted */ hook = merge))
            .fold(q!(|| vec![]), q!(|acc, v| acc.push(v)))
            .entries()
    }
    .sim_output();

    flow.sim().deterministic(async || {
        first_send.send_many([(1, 10), (1, 20)]);
        second_send.send_many([(1, 30)]);
        first_batch.release_values([(1, 10), (1, 20)]).await;
        second_batch.release_values([(1, 30)]).await;
        merge.order([(1, 30), (1, 10), (1, 20)]).await;
        output
            .assert_yields_only_unordered([(1, vec![30, 10, 20])])
            .await;
    });
}
