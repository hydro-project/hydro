//! Tests of the host-side schedule on a small program with one tick that both admits a batch and
//! reads a snapshot: what a hold does to each kind of hook, what ending a hold does, and that a
//! hold from the first round does not trip the simulator.
//!
//! The program: a process receives `items` (a stream of `u64`) and `clock` (a stream of `()`).
//! A top-level fold over `clock` counts the elements seen so far; that count is the singleton the
//! tick snapshots. Each tick execution pairs every admitted item with the snapshot it saw and
//! sends the pairs to an output. The output therefore shows, for each item, which version of the
//! counter the tick acted on when it admitted the item.
//!
//! Everything but the tests themselves is gated on `stageleft_runtime`, so that the staged copy of
//! this crate (which the simulator compiles into the program's dylib) does not refer to the
//! checker's modules, which are absent from that copy.

use hydro_lang::live_collections::sliced::sliced;
use hydro_lang::live_collections::stream::{ExactlyOnce, TotalOrder};
use hydro_lang::nondet::nondet;
use hydro_lang::prelude::*;
#[cfg(stageleft_runtime)]
use hydro_lang::sim::prompt_schedule::PromptScheduleDriver;
#[cfg(stageleft_runtime)]
use hydro_lang::sim::{SimReceiver, SimSender, quiesce};
use stageleft::q;

#[cfg(stageleft_runtime)]
use crate::schedule::{HookKind, Schedule, instrument};

struct Node;

#[cfg(stageleft_runtime)]
struct Program {
    items: SimSender<u64, TotalOrder, ExactlyOnce>,
    clock: SimSender<(), TotalOrder, ExactlyOnce>,
    pairs: SimReceiver<(u64, usize), TotalOrder, ExactlyOnce>,
}

/// Builds the program and instruments it. Returns the compiled simulation, the instrumentation,
/// and the test-side ends.
#[cfg(stageleft_runtime)]
fn program() -> (
    hydro_lang::sim::compiled::CompiledSim,
    crate::schedule::Instrumented,
    Program,
) {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<Node>();
    let (items_send, items) = node.sim_input::<u64, TotalOrder, ExactlyOnce>();
    let (clock_send, clock) = node.sim_input::<(), TotalOrder, ExactlyOnce>();
    let seen = clock.fold(q!(|| 0usize), q!(|n, ()| *n += 1));
    let pairs = sliced! {
        let items = use::batch(items, nondet!(/** the test drives this */));
        let seen = use::snapshot(seen, nondet!(/** the test drives this */));
        items.cross_singleton(seen)
    };
    let pairs_recv = pairs.sim_output();
    let mut sim = flow.sim();
    let instrumented = instrument(&mut sim);
    (
        sim.compiled(),
        instrumented,
        Program {
            items: items_send,
            clock: clock_send,
            pairs: pairs_recv,
        },
    )
}

/// The id of the one bound hook of `kind`.
#[cfg(stageleft_runtime)]
fn hook_id(instrumented: &crate::schedule::Instrumented, kind: HookKind) -> usize {
    let mut ids = instrumented.hooks.iter().filter(|h| h.kind == kind);
    let id = ids.next().expect("the program has one hook of each kind").id;
    assert!(ids.next().is_none());
    id
}

/// Runs `rounds` rounds. Each round sends one clock element and one item (numbered by the
/// round), then settles. `hold` says which hook to hold and over which rounds (`start..end`).
/// Returns the pairs the program emitted, in order.
#[cfg(stageleft_runtime)]
fn run(
    rounds: usize,
    hold: Option<(HookKind, std::ops::Range<usize>)>,
) -> (Vec<(u64, usize)>, u64) {
    let (compiled, instrumented, program) = program();
    let out = std::sync::Mutex::new(None);
    compiled.run_with_driver(PromptScheduleDriver::default(), async |instance| {
        instance
            .run_with_scheduler(async {
                let mut schedule = Schedule::new(&instrumented);
                let held = hold
                    .as_ref()
                    .map(|(kind, range)| (hook_id(&instrumented, *kind), range.clone()));
                for round in 0..rounds {
                    if let Some((id, range)) = &held {
                        if round == range.start {
                            schedule.hold(*id);
                        }
                        if round == range.end {
                            schedule.release();
                        }
                    }
                    program.clock.send(());
                    program.items.send(round as u64);
                    schedule.settle().await;
                }
                quiesce().await;
                let mut pairs = Vec::new();
                while let Some(pair) = program.pairs.try_next().await {
                    pairs.push(pair);
                }
                *out.lock().unwrap() = Some((pairs, schedule.holds_applied));
            })
            .await
    });
    out.into_inner().unwrap().unwrap()
}

/// With nothing held, every item is admitted in the round it arrives and sees the counter of
/// that round: the fold has counted the round's clock element before the tick runs (the fold is
/// upstream of the tick and the schedule runs the tick only once the simulation has quiesced).
#[test]
fn unheld_items_see_the_current_count() {
    let (pairs, applied) = run(5, None);
    assert_eq!(pairs, vec![(0, 1), (1, 2), (2, 3), (3, 4), (4, 5)]);
    assert_eq!(applied, 0);
}

/// Holding the batch admits nothing while the hold lasts and everything at once when it ends,
/// in one tick execution, which sees the count of the release round.
#[test]
fn held_batch_releases_everything_on_release() {
    let (pairs, applied) = run(6, Some((HookKind::Batch, 1..4)));
    // Round 0 unheld; rounds 1, 2, 3 held; released at round 4 together with round 4's item.
    assert_eq!(
        pairs,
        vec![(0, 1), (1, 5), (2, 5), (3, 5), (4, 5), (5, 6)]
    );
    // In each held round the counter changed, so the tick ran for the snapshot's sake while the
    // held batch had items waiting and contributed nothing; that is what `holds_applied` counts.
    assert_eq!(applied, 3);
}

/// Holding the snapshot pins the version the tick saw last: items admitted during the hold are
/// paired with the stale count. Ending the hold jumps to the newest version in one step rather
/// than replaying the versions that queued up.
#[test]
fn held_snapshot_pins_then_jumps_to_newest() {
    let (pairs, _) = run(6, Some((HookKind::Snapshot, 1..4)));
    // Round 0 revealed count 1. Rounds 1, 2, 3 held: items see 1. Round 4: release; the first
    // decision reveals the newest version (5), not 2, so item 4 sees 5 and no tick ran on 2, 3
    // or 4 (versions 2, 3, 4 are skipped). Round 5 sees 6.
    assert_eq!(
        pairs,
        vec![(0, 1), (1, 1), (2, 1), (3, 1), (4, 5), (5, 6)]
    );
}

/// A snapshot held before it has ever revealed anything has nothing to pin: its first version is
/// revealed as the ordinary schedule would reveal it, and the hold takes effect from the next
/// version on. Without this rule the simulator aborts the process when the tick runs.
#[test]
fn hold_from_the_first_round_reveals_the_first_version() {
    let (pairs, _) = run(4, Some((HookKind::Snapshot, 0..3)));
    // Round 0: nothing to pin, so count 1 is revealed. Rounds 1, 2: pinned at 1. Round 3:
    // release, jump to 4.
    assert_eq!(pairs, vec![(0, 1), (1, 1), (2, 1), (3, 4)]);
}

/// A hold that lasts to the end of the run on the batch leaves every held item unadmitted.
#[test]
fn hold_to_the_end_never_admits() {
    let (pairs, _) = run(4, Some((HookKind::Batch, 1..usize::MAX)));
    assert_eq!(pairs, vec![(0, 1)]);
}

/// `instrument` refuses a flow whose author already bound a scripted hook: the checker must be
/// the only party installing decisions.
#[test]
#[should_panic(expected = "the flow already has 1 scripted hook(s)")]
fn refuses_flows_with_scripted_hooks() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<Node>();
    let (_items_send, items) = node.sim_input::<u64, TotalOrder, ExactlyOnce>();
    let items_hook: hydro_lang::sim_hooks::BatchHook<u64, TotalOrder, ExactlyOnce, hydro_lang::sim_hooks::OnProcess<Node>> =
        flow.sim_hook();
    let out = sliced! {
        let items = use::batch(items, nondet!(/** scripted by the test */ hook = items_hook));
        items
    };
    let _out_recv = out.sim_output();
    let mut sim = flow.sim();
    instrument(&mut sim);
}

/// The verdict of a check whose program has nothing to hold is "not explored", not benign.
#[test]
fn nothing_to_hold_is_not_explored() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<Node>();
    let (items_send, items) = node.sim_input::<u64, TotalOrder, ExactlyOnce>();
    let _out: SimReceiver<u64, TotalOrder, ExactlyOnce> = items.sim_output();
    let config = crate::CheckConfig::new(4).quiet();
    let report = crate::check(flow.sim(), &config, async |round| {
        items_send.send(round as u64);
    });
    assert!(
        matches!(report.verdict, crate::Verdict::NotExplored { .. }),
        "{}",
        report
    );
    let text = report.to_string();
    assert!(text.contains("verdict: not explored"), "{text}");
    assert!(!text.contains("verdict: benign"), "{text}");
}

/// A program that enters an atomic context with `.atomic()` and reads inside it with
/// `use::atomic` (the shape of the tutorial counters). The simulator places the scheduling
/// decision at the `.atomic()` entry and refuses a hook on the read, so the checker must bind the
/// entry and leave the read alone. Before this was handled, `instrument` bound the read and the
/// program's dylib failed to compile.
#[test]
fn atomic_entry_is_the_decision_point_and_the_read_is_not() {
    let mut flow = FlowBuilder::new();
    let node = flow.process::<Node>();
    let (increments_send, increments) = node.sim_input::<(), TotalOrder, ExactlyOnce>();
    let (gets_send, gets) = node.sim_input::<(), TotalOrder, ExactlyOnce>();
    let processing = increments.atomic();
    let count = processing.clone().count();
    let acks = processing.end_atomic();
    let responses = sliced! {
        let gets = use::batch(gets, nondet!(/** the checker drives this */));
        let count = use::atomic(count, nondet!(/** atomic with the increments */));
        gets.cross_singleton(count).map(q!(|(_, n)| n))
    };
    let _acks: SimReceiver<(), TotalOrder, ExactlyOnce> = acks.sim_output();
    let _responses: SimReceiver<usize, TotalOrder, ExactlyOnce> = responses.sim_output();

    let config = crate::CheckConfig::new(8).quiet();
    let report = crate::check(flow.sim(), &config, async |_round| {
        increments_send.send(());
        gets_send.send(());
    });
    assert!(
        matches!(report.verdict, crate::Verdict::Benign),
        "{}",
        report
    );
    // Two decision points: the `.atomic()` entry and the `use::batch` of gets. The `use::atomic`
    // read is not one, and nothing is reported as unholdable.
    assert_eq!(report.curves.len(), 2, "{}", report);
    assert!(report.not_holdable.is_empty(), "{}", report);
    let text = report.to_string();
    assert!(text.contains("verdict: benign"), "{text}");
}
