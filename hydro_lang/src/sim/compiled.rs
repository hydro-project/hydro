//! Interfaces for compiled Hydro simulators and concrete simulation instances.
//!
//! # Quiescence and observation soundness
//!
//! The scheduler distinguishes two kinds of simulation work:
//! - **Deterministic work**: running the top-level async dataflows, which simply propagate
//!   whatever data is already in flight. This makes no `nondet!` decisions, so running it can
//!   never change which executions are explored.
//! - **Nondeterministic work**: running ticks and observations, whose behavior depends on
//!   decisions drawn from the bolero driver (batch boundaries, snapshot versions, message
//!   orderings). Each decision forks the space of possible executions.
//!
//! The simulation is **quiescent** when neither kind of work can make progress without new
//! external input. Test-side observations (the methods on [`SimReceiver`] /
//! [`SimClusterReceiver`]) interact with the scheduler while waiting, and the key soundness
//! question is: *when is it okay for an observation to let nondeterministic work run?*
//!
//! **Waiting for a message is always sound.** If the message eventually arrives, the work
//! that ran was necessary to produce it (schedules that run *extra* work are also valid
//! executions and are explored separately). If the simulation instead quiesces without
//! producing the message, the assertion fails and the instance ends, so nothing can observe
//! the overrun. This is why [`SimReceiver::next`], [`SimReceiver::collect_n`], and the
//! `assert_yields*` prefix checks are safe to use in the middle of a test.
//!
//! **Observing the *absence* of a message is dangerous.** Proving that "no more messages can
//! arrive" requires driving the simulation all the way to quiescence, running *all* pending
//! nondeterministic work. A later assertion may have needed to observe a state where that
//! work had not yet run — e.g., `assert_yields_only([1, 2])` followed by reading a counter
//! must be able to see the counter *before* the ticks that count `1` and `2` have fired.
//! Forcing quiescence at the first assertion would make some executions unobservable, and
//! extra messages produced by the forced work could surface at a *later* assertion,
//! misattributing the failure. Absence-observing APIs therefore proceed in phases:
//!
//! 1. **Settle** (see `SettlePauseGuard::poll_settle`): the scheduler runs only deterministic work, pausing
//!    just before nondeterministic work. If the simulation reaches quiescence this way, the
//!    end-of-stream check is *free* — no decision was forced, no execution was cut off — and
//!    the test simply continues.
//! 2. If nondeterministic work is pending, the check would overrun. What happens next depends
//!    on the API and engine:
//!    - The assertion APIs ([`SimReceiver::assert_no_more`], `assert_yields_only*`,
//!      `collect_n_only`) under [`CompiledSim::exhaustive`] **fork** the search on a bolero
//!      decision: one instance performs the check and then ends (via a discard panic, like
//!      `sim::continue_if!`), while sibling instances skip the check entirely and continue. The
//!      exhaustive driver enumerates the checking instance *first*, so a failing check is
//!      found before any instance runs past it — with a decision trace that leads exactly to
//!      the failing assertion. Since nothing after the check runs in the checking instance,
//!      the overrun it performs is unobservable, and the continuing instances never quiesce,
//!      so every downstream state remains reachable.
//!    - Otherwise (fuzz / RNG / replay engines, or the drain-everything APIs
//!      [`SimReceiver::try_next`], [`SimReceiver::collect`], and `collect_sorted` in every
//!      mode), the pending work runs and the instance is **tainted**
//!      (`QuiescenceState::tainted`). Reads of the now-quiescent state remain sound (they
//!      observe a fully-drained simulation that can no longer advance), so tests may drain
//!      multiple output ports at the end. But once new input is sent, the instance is
//!      **poisoned** (`QuiescenceState::poisoned`): any further receive panics (see
//!      `guard_not_poisoned`), because a failure observed after the forced overrun could
//!      have been caused by it and attributed to the wrong assertion.
//!
//! NOTE: This module runs inside bolero's `catch_unwind` scope, which silently
//! swallows panics. Internal invariant checks should use `abort_assert!`
//! rather than `panic!`/`assert!`.
//!
//! TODO(mingwei): Panics inside the tick DFIR (generated code in the dylib) are
//! also caught by bolero's `catch_unwind`. Consider a mechanism to detect and
//! propagate those as well.

/// Like `assert!`, but calls `std::process::abort()` instead of `panic!()`.
/// Use for internal invariants that must not be silently caught by bolero.
macro_rules! abort_assert {
    ($cond:expr, $($arg:tt)*) => {
        if !$cond {
            eprintln!("Simulator internal error: {}", format!($($arg)*));
            std::process::abort();
        }
    };
}

use core::{fmt, panic};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::fmt::Debug;
use std::panic::RefUnwindSafe;
use std::path::Path;
use std::pin::{Pin, pin};
use std::rc::Rc;
use std::task::{Poll, ready};

use bytes::Bytes;
use colored::Colorize;
use dfir_rs::scheduled::context::DfirErased;
use dfir_rs::util::unsync::mpsc::{Receiver as UnsyncReceiver, Sender as UnsyncSender};
use futures::StreamExt;
use libloading::Library;
use serde::Serialize;
use serde::de::DeserializeOwned;
use tempfile::TempPath;
use tokio::sync::{Mutex, Notify};

use super::runtime::{Hooks, InlineHooks};
use super::{SimClusterReceiver, SimClusterSender, SimReceiver, SimSender};
use crate::compile::builder::ExternalPortId;
use crate::live_collections::stream::{ExactlyOnce, NoOrder, Ordering, Retries, TotalOrder};
use crate::location::dynamic::LocationId;
use crate::sim::graph::{SimExternalPort, SimExternalPortRegistry};
use crate::sim::runtime::{SimHook, SimInlineHook};

struct QuiescenceState {
    /// Set to true when the scheduler reaches quiescence; reset to false when new input is sent.
    quiescent: Cell<bool>,
    /// Notified when the scheduler reaches quiescence (wakes receivers waiting for data).
    quiescence_notify: Notify,
    /// Notified when new input is sent, signaling the scheduler to resume.
    resume_notify: Notify,
    /// When nonzero, the scheduler must not start nondeterministic work (ticks /
    /// observations): once only such work remains, it sets `nondet_pending` and pauses until
    /// resumed. Used by receivers to query whether the simulation can quiesce
    /// deterministically. This is a count (not a bool) because multiple settling futures can
    /// be in flight at once (e.g. `select!`/`join!` between two receiver awaits): the
    /// scheduler must stay paused until *every* one of them has finished settling.
    pause_nondet: Cell<usize>,
    /// Set while the scheduler is paused because nondeterministic work is ready to run but
    /// `pause_nondet` is set.
    nondet_pending: Cell<bool>,
    /// Wakers for test-side tasks waiting for the scheduler to settle (either quiesce or set
    /// `nondet_pending`) while `pause_nondet` is set.
    settle_wakers: RefCell<Vec<std::task::Waker>>,
    /// Set when an observation *forced* the simulation to quiesce (running pending
    /// nondeterministic work) outside of exhaustive mode's forking. Further observations of
    /// the quiescent state remain sound, but once new input is sent (see `poisoned`), later
    /// observations could misattribute failures caused by the forced overrun.
    tainted: Cell<bool>,
    /// Set when new input is sent after `tainted`; all further receives panic.
    poisoned: Cell<bool>,
}

impl QuiescenceState {
    /// Signal that new input has been sent, waking the scheduler if it was quiescent.
    fn resume(&self) {
        if self.tainted.get() {
            self.poisoned.set(true);
        }
        self.quiescent.set(false);
        // `notify_one` (rather than `notify_waiters`) stores a permit if the scheduler driver
        // is not currently parked on [`Self::resumed`], so a resume that fires before the
        // driver parks (e.g. input sent while the driver is polling the thunk) is not lost.
        self.resume_notify.notify_one();
    }

    /// Whether the scheduler is currently quiescent (no more progress possible without input).
    fn is_quiescent(&self) -> bool {
        self.quiescent.get()
    }

    /// Returns a future that completes when the scheduler next reaches quiescence.
    fn notified(&self) -> tokio::sync::futures::Notified<'_> {
        self.quiescence_notify.notified()
    }

    /// Wakes test-side tasks waiting for the scheduler to settle.
    fn wake_settled(&self) {
        for waker in self.settle_wakers.borrow_mut().drain(..) {
            waker.wake();
        }
    }

    /// Enter quiescence, waking receivers waiting for data (their streams end). The scheduler
    /// driver is responsible for parking until [`Self::resume`] is called with new input.
    fn enter_quiescence(&self) {
        self.quiescent.set(true);
        self.quiescence_notify.notify_waiters();
        self.wake_settled();
    }

    /// Completes when new input arrives (via [`Self::resume`]).
    async fn resumed(&self) {
        self.resume_notify.notified().await;
    }
}

/// Tracks a pending "settle" pause request to the scheduler (see
/// [`QuiescenceState::pause_nondet`]), releasing it if the requesting future is dropped
/// mid-settle (e.g. by `select!`) so the scheduler is not left paused forever. Pause
/// requests are counted, so concurrent settling futures each hold their own request.
struct SettlePauseGuard {
    quiescence: Rc<QuiescenceState>,
    active: bool,
}

impl SettlePauseGuard {
    fn new(quiescence: Rc<QuiescenceState>) -> Self {
        SettlePauseGuard {
            quiescence,
            active: false,
        }
    }

    fn acquire(&mut self) {
        abort_assert!(!self.active, "settle pause acquired twice");
        self.quiescence
            .pause_nondet
            .set(self.quiescence.pause_nondet.get() + 1);
        self.active = true;
    }

    fn release(&mut self) {
        abort_assert!(self.active, "settle pause released without being acquired");
        self.active = false;
        self.quiescence
            .pause_nondet
            .set(self.quiescence.pause_nondet.get() - 1);
    }

    /// Polls the "settle" handshake with the scheduler: deterministic (non-tick) work is
    /// allowed to run, but the scheduler pauses instead of starting nondeterministic work
    /// (ticks / observations). Resolves to `true` if the simulation reached quiescence
    /// deterministically, or `false` if nondeterministic work is pending (in which case the
    /// scheduler is resumed).
    fn poll_settle(&mut self, cx: &mut std::task::Context<'_>) -> Poll<bool> {
        let quiescence = self.quiescence.clone();
        if !self.active {
            if quiescence.is_quiescent() {
                return Poll::Ready(true);
            }
            self.acquire();
        }

        if quiescence.is_quiescent() {
            self.release();
            Poll::Ready(true)
        } else if quiescence.nondet_pending.get() {
            self.release();
            // `notify_one` (permit-based): the driver only parks *between* thunk polls, so it
            // is not parked right now — the permit ensures this resume is not lost.
            quiescence.resume_notify.notify_one();
            Poll::Ready(false)
        } else {
            // This may push a duplicate waker if we are re-polled without an intervening
            // `wake_settled` (e.g. a `join!` sibling waking the shared task), but duplicates
            // are harmless (waking is idempotent) and are cleared at the next `wake_settled`,
            // so deduplicating here isn't worth the scan on every poll.
            quiescence
                .settle_wakers
                .borrow_mut()
                .push(cx.waker().clone());
            Poll::Pending
        }
    }
}

impl Drop for SettlePauseGuard {
    fn drop(&mut self) {
        if self.active {
            self.release();
            // Resume the scheduler in case this was the last pause request (otherwise it
            // would stay parked forever with nobody left to resume it). `notify_one`
            // (permit-based) so the resume is not lost if the driver has not parked yet. If
            // other settlers still hold requests, this wakeup is spurious but harmless: the
            // scheduler re-checks `pause_nondet > 0` before starting any nondeterministic
            // work, so it immediately re-parks without running anything.
            self.quiescence.resume_notify.notify_one();
        }
    }
}

/// Panics if the simulation has been poisoned: an earlier observation forced the simulation
/// to quiesce (running pending nondeterministic work), and new input has been sent since, so
/// further observations could misattribute failures caused by the forced overrun.
fn guard_not_poisoned(quiescence: &QuiescenceState) {
    if quiescence.poisoned.get() {
        panic!(
            "cannot receive more simulator output: an earlier observation (such as `try_next`, `collect`, or a quiescence assertion outside exhaustive mode) forced the simulation to quiesce by running pending nondeterministic work, and new input has been sent since. Failures observed now could be misattributed, so either restructure the test to make quiescence-forcing observations its last step, or insert an explicit `sim::quiesce().await` phase barrier before sending more input."
        );
    }
}

/// Runs the simulation to quiescence, as an explicit *phase barrier* between rounds of a
/// multi-phase test.
///
/// All pending nondeterministic work (ticks / observations) is forced to run until no more
/// progress is possible without new input. This deliberately narrows the explored executions:
/// inputs sent after the barrier will never interleave with work from before it, modeling
/// scenarios where new stimuli (such as timer ticks) arrive long after the system settles.
/// Pair such tests with a separate barrier-free test if interleaved executions should also be
/// explored.
///
/// Because the barrier is explicit, observations after it are *intended* to see the fully
/// settled state, so — unlike [`SimReceiver::try_next`] / [`SimReceiver::collect`] forcing
/// quiescence implicitly — it does not restrict what the test may do afterwards: receives
/// after the barrier observe only buffered output (plus whatever later input produces), and
/// failures cannot be misattributed across it.
pub async fn quiesce() {
    let quiescence =
        CURRENT_SIM_CONNECTIONS.with(|connections| connections.borrow().quiescence.clone());
    guard_not_poisoned(&quiescence);

    let mut notified_fut = pin!(None);
    std::future::poll_fn(|cx| {
        if quiescence.is_quiescent() {
            return Poll::Ready(());
        }
        // Registered before the scheduler can run (single-threaded), so the quiescence
        // notification cannot be missed.
        if notified_fut.is_none() {
            notified_fut.set(Some(quiescence.notified()));
        }
        let () = ready!(notified_fut.as_mut().as_pin_mut().unwrap().poll(cx));
        Poll::Ready(())
    })
    .await;

    // The barrier subsumes any quiescence forced by earlier observations in this phase:
    // everything before it has fully settled, and the test has explicitly opted into
    // observing only post-quiescence states from here on.
    quiescence.tainted.set(false);
}

/// Receives the next message from `receiver` while trying not to overrun the simulation:
/// first the simulation *settles* (deterministic work runs, but the scheduler pauses before
/// nondeterministic work). If a message arrives, it is returned; if the simulation settles to
/// quiescence, returns `None` without having run any nondeterministic work. Otherwise the
/// scheduler is resumed and pending nondeterministic work runs until a message arrives or the
/// simulation quiesces; quiescing this way *taints* the simulation (see
/// [`QuiescenceState::tainted`]).
async fn try_next_bytes(
    receiver: &Mutex<UnsyncReceiver<Bytes>>,
    quiescence: &Rc<QuiescenceState>,
) -> Option<Bytes> {
    guard_not_poisoned(quiescence);

    let mut receiver_stream = receiver.lock().await;
    let mut settle_guard = SettlePauseGuard::new(quiescence.clone());
    // `Some` once the settle phase has concluded that nondeterministic work is pending and
    // we have started forcing it to run.
    let mut notified_fut = pin!(None);

    std::future::poll_fn(|cx| {
        // A message may become available at any point (including from deterministic work
        // while settling), so always check the stream first.
        match receiver_stream.poll_next_unpin(cx) {
            Poll::Ready(Some(bytes)) => return Poll::Ready(Some(bytes)),
            Poll::Ready(None) => return Poll::Ready(None),
            Poll::Pending => {}
        }

        if notified_fut.is_none() {
            match settle_guard.poll_settle(cx) {
                // Deterministically quiescent: no more messages, and nothing was overrun.
                Poll::Ready(true) => return Poll::Ready(None),
                // Nondeterministic work is pending; start forcing it to run. The `Notified`
                // is created here and polled (registered) below in this same synchronous
                // poll — before the scheduler can run — and the simulation is not currently
                // quiescent, so the quiescence notification cannot be missed.
                Poll::Ready(false) => notified_fut.set(Some(quiescence.notified())),
                Poll::Pending => return Poll::Pending,
            }
        }

        // Let the scheduler run nondeterministic work until a message arrives or the
        // simulation quiesces. Note that merely entering this phase does not taint: if a
        // message arrives (the `Some` exit at the top), waiting was sound for the same
        // reason as `SimReceiver::next` — the work that ran was needed to produce it. Only
        // *observing quiescence* after forcing the pending work taints, since that is the
        // overrun a later observation could misattribute.
        let () = ready!(notified_fut.as_mut().as_pin_mut().unwrap().poll(cx));
        quiescence.tainted.set(true);
        Poll::Ready(None)
    })
    .await
}

struct SimConnections {
    input_senders: HashMap<SimExternalPort, UnsyncSender<Bytes>>,
    output_receivers: HashMap<SimExternalPort, Rc<Mutex<UnsyncReceiver<Bytes>>>>,
    cluster_input_senders: HashMap<SimExternalPort, HashMap<u32, UnsyncSender<Bytes>>>,
    cluster_output_receivers:
        HashMap<SimExternalPort, HashMap<u32, Rc<Mutex<UnsyncReceiver<Bytes>>>>>,
    external_registered: HashMap<ExternalPortId, SimExternalPort>,
    quiescence: Rc<QuiescenceState>,
    log: bool,
    /// Whether this instance is being executed by the exhaustive engine (see
    /// [`CompiledSim::exhaustive`]), which affects how `assert_yields_only` explores
    /// quiescence checks.
    exhaustive: bool,
}

/// Implementation detail of [`crate::sim::continue_if!`](crate::continue_if); do not call directly.
///
/// If `condition` is false, aborts the current simulation instance by panicking with a special
/// payload ([`bolero::generator::bolero_generator::any::Error`]) that bolero recognizes as an
/// "invalid input" marker: the instance is discarded (not treated as a test failure, and never
/// recorded as a reproducer) and exploration moves on to the next instance. If logging is
/// enabled for the current instance, the failed assumption is logged first.
#[doc(hidden)]
#[track_caller]
pub fn continue_if_impl(condition: bool, message: fmt::Arguments<'_>) {
    if condition {
        return;
    }

    let log = CURRENT_SIM_CONNECTIONS
        .try_with(|connections| connections.borrow().log)
        .unwrap_or(true);
    if log {
        eprintln!(
            "{}",
            render_continue_if_failure(std::panic::Location::caller(), message)
        );
    }

    // Panics with `bolero_generator::any::Error`, which bolero's engines treat as an invalid
    // input rather than a test failure. Both this function and bolero's `assume` are
    // `#[track_caller]`, so the recorded location is the user's `continue_if!` call site.
    bolero::generator::bolero_generator::any::assume(false, "simulation assumption failed");
}

/// Renders the log message for a failed assumption, echoing the source line with a caret
/// pointing at the `continue_if!` call site, in the same style as the other simulator logs.
fn render_continue_if_failure(
    location: &std::panic::Location<'_>,
    message: fmt::Arguments<'_>,
) -> String {
    use std::fmt::Write;

    // `Location::file()` is relative to the directory the crate was compiled from (e.g. the
    // workspace root), which may not match the current working directory (e.g. the crate
    // root when running `cargo test`), so walk up from the current directory to find it.
    let source_line = std::env::current_dir()
        .ok()
        .and_then(|cwd| {
            cwd.ancestors()
                .find_map(|base| std::fs::read_to_string(base.join(location.file())).ok())
        })
        .and_then(|content| {
            content
                .lines()
                .nth((location.line() as usize).saturating_sub(1))
                .map(|line| line.to_owned())
        })
        .unwrap_or_default();

    let caret_indent = " ".repeat((location.column() as usize).saturating_sub(1));

    let mut out = String::new();
    let _ = writeln!(
        out,
        "\n{}",
        "Condition failed (discarding simulation instance):"
            .color(colored::Color::Yellow)
            .bold()
    );
    let _ = writeln!(out, "{} {}", "-->".color(colored::Color::Blue), location);
    let _ = writeln!(out, " {}{}", "|".color(colored::Color::Blue), source_line);
    let _ = write!(
        out,
        " {}{}{}",
        "|".color(colored::Color::Blue),
        caret_indent,
        format!("^ {}", message).color(colored::Color::Yellow)
    );
    out
}

tokio::task_local! {
    static CURRENT_SIM_CONNECTIONS: RefCell<SimConnections>;
}

/// A handle to a compiled Hydro simulation, which can be instantiated and run.
pub struct CompiledSim {
    pub(super) _path: TempPath,
    pub(super) lib: Library,
    pub(super) externals_port_registry: SimExternalPortRegistry,
    pub(super) unit_test_fuzz_iterations: usize,
}

#[sealed::sealed]
/// A trait implemented by closures that can instantiate a compiled simulation.
///
/// This is needed to ensure [`RefUnwindSafe`] so instances can be created during fuzzing.
pub trait Instantiator<'a>: RefUnwindSafe + Fn() -> CompiledSimInstance<'a> {}
#[sealed::sealed]
impl<'a, T: RefUnwindSafe + Fn() -> CompiledSimInstance<'a>> Instantiator<'a> for T {}

fn null_handler(_args: fmt::Arguments<'_>) {}

fn println_handler(args: fmt::Arguments<'_>) {
    println!("{}", args);
}

fn eprintln_handler(args: fmt::Arguments<'_>) {
    eprintln!("{}", args);
}

/// Creates a simulation instance, returning:
/// - A list of async DFIRs to run (all process / cluster logic outside a tick)
/// - A list of tick DFIRs to run (where the &'static str is for the tick location id)
/// - A mapping of hooks for non-deterministic decisions at tick-input boundaries
/// - A mapping of inline hooks for non-deterministic decisions inside ticks
type SimLoaded<'a> = libloading::Symbol<
    'a,
    unsafe extern "Rust" fn(
        should_color: bool,
        external_out: &mut HashMap<usize, UnsyncReceiver<Bytes>>,
        external_in: &mut HashMap<usize, UnsyncSender<Bytes>>,
        cluster_external_out: &mut HashMap<usize, HashMap<u32, UnsyncReceiver<Bytes>>>,
        cluster_external_in: &mut HashMap<usize, HashMap<u32, UnsyncSender<Bytes>>>,
        println_handler: fn(fmt::Arguments<'_>),
        eprintln_handler: fn(fmt::Arguments<'_>),
    ) -> (
        Vec<(&'static str, Option<u32>, DfirErased)>,
        Vec<(&'static str, Option<u32>, DfirErased)>,
        Hooks<&'static str>,
        InlineHooks<&'static str>,
    ),
>;

impl CompiledSim {
    /// Executes the given closure with a single instance of the compiled simulation.
    pub fn with_instance<T>(&self, thunk: impl FnOnce(CompiledSimInstance<'_>) -> T) -> T {
        self.with_instantiator(|instantiator| thunk(instantiator()), true)
    }

    /// Executes the given closure with an [`Instantiator`], which can be called to create
    /// independent instances of the simulation. This is useful for fuzzing, where we need to
    /// re-execute the simulation several times with different decisions.
    ///
    /// The `always_log` parameter controls whether to log tick executions and stream releases. If
    /// it is `true`, logging will always be enabled. If it is `false`, logging will only be
    /// enabled if the `HYDRO_SIM_LOG` environment variable is set to `1`.
    pub fn with_instantiator<T>(
        &self,
        thunk: impl FnOnce(&dyn Instantiator<'_>) -> T,
        always_log: bool,
    ) -> T {
        let func: SimLoaded<'_> = unsafe { self.lib.get(b"__hydro_runtime").unwrap() };
        let log = always_log || std::env::var("HYDRO_SIM_LOG").is_ok_and(|v| v == "1");
        thunk(
            &(|| CompiledSimInstance {
                func: func.clone(),
                externals_port_registry: self.externals_port_registry.clone(),
                dylib_result: None,
                log,
                exhaustive: false,
            }),
        )
    }

    /// Uses a fuzzing strategy to explore possible executions of the simulation. The provided
    /// closure will be repeatedly executed with instances of the Hydro program where the
    /// batching boundaries, order of messages, and retries are varied.
    ///
    /// During development, you should run the test that invokes this function with the `cargo sim`
    /// command, which will use `libfuzzer` to intelligently explore the execution space. If a
    /// failure is found, a minimized test case will be produced in a `sim-failures` directory.
    /// When running the test with `cargo test` (such as in CI), if a reproducer is found it will
    /// be executed, and if no reproducer is found a small number of random executions will be
    /// performed.
    pub fn fuzz(&self, mut thunk: impl AsyncFnMut() + RefUnwindSafe) {
        let caller_fn = crate::compile::ir::backtrace::Backtrace::get_backtrace(0)
            .elements()
            .into_iter()
            .find(|e| {
                !e.fn_name.starts_with("hydro_lang::sim::compiled")
                    && !e.fn_name.starts_with("hydro_lang::sim::flow")
                    && !e.fn_name.starts_with("fuzz<")
                    && !e.fn_name.starts_with("<hydro_lang::sim")
            })
            .unwrap();

        let caller_path = Path::new(&caller_fn.filename.unwrap()).to_path_buf();
        let repro_folder = caller_path.parent().unwrap().join("sim-failures");

        let caller_fuzz_repro_path = repro_folder
            .join(caller_fn.fn_name.replace("::", "__"))
            .with_extension("bin");

        if std::env::var("BOLERO_FUZZER").is_ok() {
            let corpus_dir = std::env::current_dir().unwrap().join(".fuzz-corpus");
            std::fs::create_dir_all(&corpus_dir).unwrap();
            let libfuzzer_args = format!(
                "{} {} -artifact_prefix={}/ -handle_abrt=0",
                corpus_dir.to_str().unwrap(),
                corpus_dir.to_str().unwrap(),
                corpus_dir.to_str().unwrap(),
            );

            std::fs::create_dir_all(&repro_folder).unwrap();

            if !std::env::var("HYDRO_NO_FAILURE_OUTPUT").is_ok_and(|v| v == "1") {
                unsafe {
                    std::env::set_var(
                        "BOLERO_FAILURE_OUTPUT",
                        caller_fuzz_repro_path.to_str().unwrap(),
                    );
                }
            }

            unsafe {
                std::env::set_var("BOLERO_LIBFUZZER_ARGS", libfuzzer_args);
            }

            self.with_instantiator(
                |instantiator| {
                    bolero::test(bolero::TargetLocation {
                        package_name: "",
                        manifest_dir: "",
                        module_path: "",
                        file: "",
                        line: 0,
                        item_path: "<unknown>::__bolero_item_path__",
                        test_name: None,
                    })
                    .run_with_replay(move |is_replay| {
                        let mut instance = instantiator();

                        if instance.log {
                            eprintln!(
                                "{}",
                                "\n==== New Simulation Instance ===="
                                    .color(colored::Color::Cyan)
                                    .bold()
                            );
                        }

                        if is_replay {
                            instance.log = true;
                        }

                        tokio::runtime::Builder::new_current_thread()
                            .build()
                            .unwrap()
                            .block_on(async { instance.run(&mut thunk).await })
                    })
                },
                false,
            );
        } else if let Ok(existing_bytes) = std::fs::read(&caller_fuzz_repro_path) {
            self.fuzz_repro(existing_bytes, async |compiled| {
                compiled.run_with_scheduler(thunk()).await
            });
        } else {
            eprintln!(
                "Running a fuzz test without `cargo sim` and no reproducer found at {}, using {} iterations with random inputs.",
                caller_fuzz_repro_path.display(),
                self.unit_test_fuzz_iterations,
            );
            self.with_instantiator(
                |instantiator| {
                    bolero::test(bolero::TargetLocation {
                        package_name: "",
                        manifest_dir: "",
                        module_path: "",
                        file: ".",
                        line: 0,
                        item_path: "<unknown>::__bolero_item_path__",
                        test_name: None,
                    })
                    .with_iterations(self.unit_test_fuzz_iterations)
                    .run_with_replay(move |is_replay| {
                        let mut instance = instantiator();

                        if instance.log {
                            eprintln!(
                                "{}",
                                "\n==== New Simulation Instance ===="
                                    .color(colored::Color::Cyan)
                                    .bold()
                            );
                        }

                        if is_replay {
                            instance.log = true;
                        }

                        tokio::runtime::Builder::new_current_thread()
                            .build()
                            .unwrap()
                            .block_on(async { instance.run(&mut thunk).await })
                    })
                },
                false,
            );
        }
    }

    /// Executes the given closure with a single instance of the compiled simulation, using the
    /// provided bytes as the source of fuzzing decisions. This can be used to manually reproduce a
    /// failure found during fuzzing.
    pub fn fuzz_repro<'a>(
        &'a self,
        bytes: Vec<u8>,
        thunk: impl AsyncFnOnce(CompiledSimInstance<'_>) + RefUnwindSafe,
    ) {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.with_instance(|instance| {
                bolero::bolero_engine::any::scope::with(
                    Box::new(bolero::bolero_engine::driver::object::Object(
                        bolero::bolero_engine::driver::bytes::Driver::new(
                            bytes,
                            &Default::default(),
                        ),
                    )),
                    || {
                        tokio::runtime::Builder::new_current_thread()
                            .build()
                            .unwrap()
                            .block_on(async { instance.run_without_launching(thunk).await })
                    },
                )
            })
        }));

        if let Err(payload) = result {
            if payload
                .downcast_ref::<bolero::generator::bolero_generator::any::Error>()
                .is_some()
            {
                // A `continue_if!` failed (or the driver ran out of entropy) while replaying the
                // recorded bytes. Instances that fail an assumption are never recorded as
                // failures, so this means the reproducer is stale or does not correspond to
                // this program.
                panic!(
                    "simulation assumption failed while replaying recorded fuzz decisions; the reproducer may be stale or may not correspond to this program"
                );
            }
            std::panic::resume_unwind(payload);
        }
    }

    /// Exhaustively searches all possible executions of the simulation. The provided
    /// closure will be repeatedly executed with instances of the Hydro program where the
    /// batching boundaries, order of messages, and retries are varied.
    ///
    /// Exhaustive searching is feasible when the inputs to the Hydro program are finite and there
    /// are no dataflow loops that generate infinite messages. Exhaustive searching provides a
    /// stronger guarantee of correctness than fuzzing, but may take a long time to complete.
    /// Because no fuzzer is involved, you can run exhaustive tests with `cargo test`.
    ///
    /// Returns the number of distinct executions explored.
    pub fn exhaustive(&self, mut thunk: impl AsyncFnMut() + RefUnwindSafe) -> usize {
        if std::env::var("BOLERO_FUZZER").is_ok() {
            eprintln!(
                "Cannot run exhaustive tests with a fuzzer. Please use `cargo test` instead of `cargo sim`."
            );
            std::process::abort();
        }

        let mut count = 0;
        let count_mut = &mut count;

        let _span = tracing::debug_span!(target: "hydro_build", "sim_exhaustive").entered();

        self.with_instantiator(
            |instantiator| {
                bolero::test(bolero::TargetLocation {
                    package_name: "",
                    manifest_dir: "",
                    module_path: "",
                    file: "",
                    line: 0,
                    item_path: "<unknown>::__bolero_item_path__",
                    test_name: None,
                })
                .exhaustive()
                .run_with_replay(move |is_replay| {
                    *count_mut += 1;

                    let mut instance = instantiator();
                    instance.exhaustive = true;
                    if instance.log {
                        eprintln!(
                            "{}",
                            "\n==== New Simulation Instance ===="
                                .color(colored::Color::Cyan)
                                .bold()
                        );
                    }

                    if is_replay {
                        instance.log = true;
                    }

                    tokio::runtime::Builder::new_current_thread()
                        .build()
                        .unwrap()
                        .block_on(async { instance.run(&mut thunk).await })
                })
            },
            false,
        );

        count
    }
}

// This must be a tuple because it is referenced from generated code in `graph.rs`.
type DylibResult = (
    Vec<(&'static str, Option<u32>, DfirErased)>,
    Vec<(&'static str, Option<u32>, DfirErased)>,
    Hooks<&'static str>,
    InlineHooks<&'static str>,
);

/// A single instance of a compiled Hydro simulation, which provides methods to interactively
/// execute the simulation, feed inputs, and receive outputs.
pub struct CompiledSimInstance<'a> {
    func: SimLoaded<'a>,
    externals_port_registry: SimExternalPortRegistry,
    dylib_result: Option<DylibResult>,
    log: bool,
    exhaustive: bool,
}

impl<'a> CompiledSimInstance<'a> {
    async fn run(self, thunk: impl AsyncFnOnce() + RefUnwindSafe) {
        self.run_without_launching(async |instance| {
            instance.run_with_scheduler(thunk()).await;
        })
        .await;
    }

    async fn run_without_launching(
        mut self,
        thunk: impl AsyncFnOnce(CompiledSimInstance<'_>) + RefUnwindSafe,
    ) {
        let mut external_out: HashMap<usize, UnsyncReceiver<Bytes>> = HashMap::new();
        let mut external_in: HashMap<usize, UnsyncSender<Bytes>> = HashMap::new();
        let mut cluster_external_out: HashMap<usize, HashMap<u32, UnsyncReceiver<Bytes>>> =
            HashMap::new();
        let mut cluster_external_in: HashMap<usize, HashMap<u32, UnsyncSender<Bytes>>> =
            HashMap::new();

        let dylib_result = unsafe {
            (self.func)(
                colored::control::SHOULD_COLORIZE.should_colorize(),
                &mut external_out,
                &mut external_in,
                &mut cluster_external_out,
                &mut cluster_external_in,
                if self.log {
                    println_handler
                } else {
                    null_handler
                },
                if self.log {
                    eprintln_handler
                } else {
                    null_handler
                },
            )
        };

        let registered = &self.externals_port_registry.registered;

        let quiescence = Rc::new(QuiescenceState {
            quiescent: Cell::new(false),
            quiescence_notify: Notify::new(),
            resume_notify: Notify::new(),
            pause_nondet: Cell::new(0),
            nondet_pending: Cell::new(false),
            settle_wakers: RefCell::new(vec![]),
            tainted: Cell::new(false),
            poisoned: Cell::new(false),
        });

        let mut input_senders = HashMap::new();
        let mut output_receivers = HashMap::new();
        let mut cluster_input_senders = HashMap::new();
        let mut cluster_output_receivers = HashMap::new();

        #[expect(
            clippy::disallowed_methods,
            reason = "inserts into maps also unordered"
        )]
        for sim_port in registered.values() {
            let usize_key = sim_port.into_inner();
            if let Some(sender) = external_in.remove(&usize_key) {
                input_senders.insert(*sim_port, sender);
            }
            if let Some(receiver) = external_out.remove(&usize_key) {
                output_receivers.insert(*sim_port, Rc::new(Mutex::new(receiver)));
            }
            if let Some(senders) = cluster_external_in.remove(&usize_key) {
                cluster_input_senders.insert(*sim_port, senders);
            }
            if let Some(receivers) = cluster_external_out.remove(&usize_key) {
                cluster_output_receivers.insert(
                    *sim_port,
                    receivers
                        .into_iter()
                        .map(|(member, r)| (member, Rc::new(Mutex::new(r))))
                        .collect(),
                );
            }
        }

        self.dylib_result = Some(dylib_result);

        CURRENT_SIM_CONNECTIONS
            .scope(
                RefCell::new(SimConnections {
                    input_senders,
                    output_receivers,
                    cluster_input_senders,
                    cluster_output_receivers,
                    external_registered: self.externals_port_registry.registered.clone(),
                    quiescence: quiescence.clone(),
                    log: self.log,
                    exhaustive: self.exhaustive,
                }),
                async move {
                    thunk(self).await;
                },
            )
            .await;
    }

    /// Runs the simulation scheduler alongside the given future, until the future completes.
    ///
    /// The future always gets to run first; whenever it is blocked (e.g. waiting to receive
    /// simulation outputs), the scheduler runs a single step to completion. Steps are atomic
    /// with respect to the future: it is re-polled between every pair of scheduler steps, but
    /// never while a step is in flight. The [`LaunchedSim`] state struct lives across steps,
    /// in this function's frame.
    async fn run_with_scheduler(self, thunk: impl Future<Output = ()>) {
        self.run_with_scheduler_and_maybe_logger::<std::io::Empty>(None, thunk)
            .await;
    }

    /// Runs the simulation scheduler alongside the given future, until the future completes,
    /// reporting the simulation trace to the given logger.
    ///
    /// The future always gets to run first; whenever it is blocked (e.g. waiting to receive
    /// simulation outputs), the scheduler runs a single step to completion. Steps are atomic
    /// with respect to the future: it is re-polled between every pair of scheduler steps, but
    /// never while a step is in flight.
    pub async fn run_with_scheduler_and_logger<W: std::io::Write>(
        self,
        log_writer: W,
        thunk: impl Future<Output = ()>,
    ) {
        self.run_with_scheduler_and_maybe_logger(Some(log_writer), thunk)
            .await;
    }

    async fn run_with_scheduler_and_maybe_logger<W: std::io::Write>(
        self,
        log_override: Option<W>,
        thunk: impl Future<Output = ()>,
    ) {
        let mut sim = self.start(log_override);
        let mut thunk_fut = pin!(thunk);
        loop {
            // The thunk always gets to run first.
            if futures::poll!(thunk_fut.as_mut()).is_ready() {
                break;
            }

            if sim.quiescence.is_quiescent() || sim.quiescence.nondet_pending.get() {
                // The scheduler is parked: either no step can make progress until the thunk
                // sends new input (quiescent), or nondeterministic work is ready but a
                // settling test-side observation has paused the scheduler (nondet_pending).
                // Park until either the thunk is woken independently or the scheduler is
                // resumed. (`resumed()` is permit-based, so a resume that fired while polling
                // the thunk above is not lost.)
                tokio::select! {
                    biased;
                    () = &mut thunk_fut => break,
                    () = sim.quiescence.resumed() => {}
                }
                sim.quiescence.nondet_pending.set(false);
            } else {
                // Run a single scheduler step to completion. This is awaited directly (not
                // raced against the thunk), so a step is atomic: the thunk is never polled
                // while a step is in flight, and a step is never cancelled mid-execution.
                sim.step().await;
            }
        }
    }

    /// Consumes this instance and constructs the [`LaunchedSim`] state struct, which is
    /// advanced incrementally via [`LaunchedSim::step`].
    fn start<W: std::io::Write>(mut self, log_override: Option<W>) -> LaunchedSim<W> {
        let (async_dfirs, tick_dfirs, mut hooks, mut inline_hooks) =
            self.dylib_result.take().unwrap();

        // The generated code keys hooks and tick DFIRs by the same serialized location
        // strings, so we can move each tick's / observation's hooks out of the maps and
        // attach them directly. This lets the scheduler's hot paths avoid keyed lookups
        // (which would clone `LocationId`s) entirely.
        let not_ready_ticks = tick_dfirs
            .into_iter()
            .map(|(lid, cluster_id, dfir)| {
                let location: LocationId = serde_json::from_str(lid).unwrap();
                let LocationId::Tick(_, parent_location) = location else {
                    unreachable!("tick DFIRs are always keyed by a tick location")
                };
                SimTick {
                    parent_location: *parent_location,
                    cluster_id,
                    dfir,
                    hooks: hooks
                        .remove(&(lid, cluster_id))
                        .expect("every tick DFIR must have at least one hook"),
                    inline_hooks: inline_hooks.remove(&(lid, cluster_id)).unwrap_or_default(),
                }
            })
            .collect();

        let quiescence = CURRENT_SIM_CONNECTIONS.with(|connections| {
            let connections = connections.borrow();
            connections.quiescence.clone()
        });

        let not_ready_observations = async_dfirs
            .iter()
            .flat_map(|(lid, cluster_id, _)| {
                let location: LocationId = serde_json::from_str(lid).unwrap();
                hooks
                    .remove(&(*lid, *cluster_id))
                    .unwrap_or_default()
                    .into_iter()
                    .map(move |hook| SimObservation {
                        location: location.clone(),
                        cluster_id: *cluster_id,
                        hook,
                    })
            })
            .collect();

        debug_assert!(
            hooks.is_empty() && inline_hooks.is_empty(),
            "all hooks should belong to either a tick DFIR or a top-level location"
        );

        LaunchedSim {
            async_dfirs: async_dfirs
                .into_iter()
                .map(|(lid, c_id, dfir)| (serde_json::from_str(lid).unwrap(), c_id, dfir))
                .collect(),
            possibly_ready_ticks: vec![],
            not_ready_ticks,
            possibly_ready_observations: vec![],
            not_ready_observations,
            log: if self.log {
                if let Some(w) = log_override {
                    LogKind::Custom(w)
                } else {
                    LogKind::Stderr
                }
            } else {
                LogKind::Null
            },
            quiescence,
        }
    }
}

impl<T: Serialize + DeserializeOwned, O: Ordering, R: Retries> Clone for SimReceiver<T, O, R> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Serialize + DeserializeOwned, O: Ordering, R: Retries> Copy for SimReceiver<T, O, R> {}

/// How a [`QuiescenceCheckFuture`] resolves the "did the stream end?" check of
/// `assert_no_more`. Decided once the simulation has settled (run out of deterministic
/// work).
#[derive(Clone, Copy)]
enum QuiescenceBranch {
    /// Skip the check and continue the test. Only taken in exhaustive mode, where a
    /// sibling instance performs the check instead.
    Continue,
    /// Perform the check, then end this simulation instance (exhaustive mode), letting
    /// sibling instances continue past this point without forcing quiescence.
    CheckThenEnd,
    /// Perform the check and keep running. Taken when the simulation is already quiescent
    /// (the check is free) and in non-exhaustive modes.
    CheckAndKeepRunning,
}

/// Decides how to run the quiescence check when the simulation has pending nondeterministic
/// work (ticks / observations) that the check would force to run.
fn decide_quiescence_branch() -> QuiescenceBranch {
    let (exhaustive, log) = CURRENT_SIM_CONNECTIONS.with(|connections| {
        let connections = connections.borrow();
        (connections.exhaustive, connections.log)
    });

    if !exhaustive {
        return QuiescenceBranch::CheckAndKeepRunning;
    }

    // In exhaustive mode, fork the search on a bolero decision. The exhaustive driver
    // enumerates `false` first, so the instance that performs the quiescence check is
    // explored *before* any instance that continues past this assertion. This ensures that
    // if the stream has extra output, the failure is attributed to this assertion (with a
    // decision trace leading exactly to the check) rather than leaking the extra messages
    // into a later assertion.
    let continue_without_check: bool = bolero::any();
    if continue_without_check {
        if log {
            eprintln!(
                "\n{}",
                "Continuing past quiescence assertion without checking (checked by an earlier instance)"
                    .color(colored::Color::Cyan)
                    .bold()
            );
        }
        QuiescenceBranch::Continue
    } else {
        if log {
            eprintln!(
                "\n{}",
                "Checking that no more messages arrive (this instance will end after the check)"
                    .color(colored::Color::Cyan)
                    .bold()
            );
        }
        QuiescenceBranch::CheckThenEnd
    }
}

/// Ends the current simulation instance after a passing quiescence check, by panicking with
/// [`bolero::generator::bolero_generator::any::Error`], which bolero's engines treat as an
/// invalid input rather than a test failure. The instance has verified everything up to and
/// including the quiescence check; sibling instances continue past the check instead.
fn end_instance_after_quiescence_check() -> ! {
    bolero::generator::bolero_generator::any::assume(
        false,
        "simulation instance ended after quiescence check",
    );
    unreachable!()
}

pin_project_lite::pin_project! {
    // The "and then the stream ends" half of `assert_no_more` (and thus of
    // `assert_yields_only*` / `collect_n_only`). First lets the simulation *settle* (see
    // `poll_settle`): if it settles to quiescence, the check is free and the test simply
    // continues. Otherwise, in exhaustive mode the search forks into a checking instance and
    // continuing instances (see `SimReceiver::assert_no_more` and
    // `decide_quiescence_branch`); in non-exhaustive modes the check runs, forcing the
    // pending work (which taints the simulation, via `try_next_bytes`).
    //
    // See [`FutureTrackingCaller`] for why `poll` is `#[track_caller]`.
    struct QuiescenceCheckFuture<F: Future<Output = ()>> {
        #[pin]
        check: F,
        settle: SettlePauseGuard,
        branch: Option<QuiescenceBranch>,
    }
}

impl<F: Future<Output = ()>> QuiescenceCheckFuture<F> {
    fn new(check: F) -> Self {
        QuiescenceCheckFuture {
            check,
            settle: SettlePauseGuard::new(
                CURRENT_SIM_CONNECTIONS.with(|connections| connections.borrow().quiescence.clone()),
            ),
            branch: None,
        }
    }
}

impl<F: Future<Output = ()>> Future for QuiescenceCheckFuture<F> {
    type Output = ();

    #[track_caller]
    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let this = self.as_mut().project();

        if this.branch.is_none() {
            *this.branch = Some(if ready!(this.settle.poll_settle(cx)) {
                // Settled to quiescence deterministically, so the check is free.
                QuiescenceBranch::CheckAndKeepRunning
            } else {
                // The check would force nondeterministic work to run.
                decide_quiescence_branch()
            });
        }

        match this.branch.unwrap() {
            QuiescenceBranch::Continue => Poll::Ready(()),
            QuiescenceBranch::CheckAndKeepRunning => this.check.poll(cx),
            QuiescenceBranch::CheckThenEnd => {
                ready!(this.check.poll(cx));
                end_instance_after_quiescence_check()
            }
        }
    }
}

impl<T: Serialize + DeserializeOwned, O: Ordering, R: Retries> SimReceiver<T, O, R> {
    fn connections(&self) -> (Rc<Mutex<UnsyncReceiver<Bytes>>>, Rc<QuiescenceState>) {
        CURRENT_SIM_CONNECTIONS.with(|connections| {
            let connections = connections.borrow();
            let port = connections.external_registered.get(&self.0).unwrap();
            (
                connections.output_receivers.get(port).unwrap().clone(),
                connections.quiescence.clone(),
            )
        })
    }

    /// See [`try_next_bytes`].
    async fn try_next_impl(&self) -> Option<T> {
        let (receiver, quiescence) = self.connections();
        try_next_bytes(&receiver, &quiescence)
            .await
            .map(|bytes| bincode::deserialize(&bytes).unwrap())
    }

    /// Asserts that the stream has ended and no more messages can possibly arrive.
    ///
    /// If the check cannot be answered without running pending nondeterministic work (such
    /// as ticks with buffered inputs):
    /// - Under [`CompiledSim::exhaustive`], the search forks: one instance performs the
    ///   check and ends there, while sibling instances skip the check and continue.
    /// - In other modes, the pending work runs; afterwards, sending more input and then
    ///   attempting to receive output will panic.
    pub fn assert_no_more(self) -> impl Future<Output = ()>
    where
        T: Debug,
    {
        QuiescenceCheckFuture::new(FutureTrackingCaller {
            future: async move {
                if let Some(next) = self.try_next_impl().await {
                    return Err(format!(
                        "Stream yielded unexpected message: {:?}, expected termination",
                        next
                    ));
                }
                Ok(())
            },
        })
    }
}

impl<T: Serialize + DeserializeOwned> SimReceiver<T, TotalOrder, ExactlyOnce> {
    /// Receives the next message from the external bincode stream, waiting (and letting the
    /// scheduler run any pending simulation work) until one is available. If the simulation
    /// becomes quiescent without producing a message, the test fails.
    ///
    /// This is safe to use in the middle of a test; to observe the *absence* of a message,
    /// use [`Self::try_next`] or [`Self::assert_no_more`].
    pub fn next(&self) -> impl use<'_, T> + Future<Output = T> {
        // Waiting for a message never "overruns" the simulation, even though the scheduler
        // may run nondeterministic ticks while we wait: if a message arrives, some pending
        // work was necessary to produce it (schedules that run *extra* work are also valid
        // executions, explored separately), and if the simulation quiesces instead, the test
        // fails right here — so no later observation can be affected by the overrun (the
        // taint set by `try_next_impl` is unobservable). See the module docs for the full
        // soundness reasoning.
        FutureTrackingCaller {
            future: async move {
                self.try_next_impl().await.ok_or_else(|| {
                    "Stream ended (simulation quiescent), but another message was expected"
                        .to_owned()
                })
            },
        }
    }

    /// Receives the next message from the external bincode stream, or returns `None` if no
    /// more messages can possibly arrive.
    ///
    /// If answering requires forcing pending nondeterministic work to run, then afterwards,
    /// sending more input and then attempting to receive output will panic. Prefer
    /// [`Self::next`] (or [`Self::assert_no_more`]) when possible.
    pub async fn try_next(&self) -> Option<T> {
        self.try_next_impl().await
    }

    /// Receives the next `n` messages from the external bincode stream, waiting (and letting
    /// the scheduler run any pending simulation work) until they are available. If the
    /// simulation becomes quiescent before `n` messages arrive, the test fails.
    ///
    /// Like [`Self::next`], this is safe to use in the middle of a test. It does not check
    /// that the stream ends afterwards; use [`Self::collect_n_only`] for that.
    pub fn collect_n<C: Default + Extend<T>>(
        &self,
        n: usize,
    ) -> impl use<'_, T, C> + Future<Output = C> {
        FutureTrackingCaller {
            future: async move {
                let mut out = C::default();
                for i in 0..n {
                    // Like `next`, waiting for each message is safe mid-test; the taint on a
                    // forced `None` is unobservable because the test fails below.
                    if let Some(v) = self.try_next_impl().await {
                        out.extend([v]);
                    } else {
                        return Err(format!(
                            "Stream ended (simulation quiescent) after {} messages, but {} were expected",
                            i, n
                        ));
                    }
                }
                Ok(out)
            },
        }
    }

    /// Receives the next `n` messages (like [`Self::collect_n`]) and then asserts that the
    /// stream ends (like [`Self::assert_no_more`], forking the search in exhaustive mode).
    pub async fn collect_n_only<C: Default + Extend<T>>(self, n: usize) -> C
    where
        T: Debug,
    {
        let out = self.collect_n(n).await;
        self.assert_no_more().await;
        out
    }

    /// Collects all remaining messages from the external bincode stream into a collection,
    /// waiting until no more messages can possibly arrive.
    ///
    /// If this has to force pending nondeterministic work to run, it should be the last
    /// observation of the test: afterwards, sending more input and then attempting to
    /// receive output will panic. When the number of expected messages is known, prefer
    /// [`Self::collect_n`] / [`Self::collect_n_only`].
    pub async fn collect<C: Default + Extend<T>>(self) -> C {
        let mut out = C::default();
        while let Some(v) = self.try_next_impl().await {
            out.extend([v]);
        }
        out
    }

    /// Asserts that the stream yields exactly the expected sequence of messages, in order.
    /// This does not check that the stream ends, use [`Self::assert_yields_only`] for that.
    ///
    /// Like [`Self::next`], this is safe to use in the middle of a test.
    pub fn assert_yields<T2: Debug, I: IntoIterator<Item = T2>>(
        &self,
        expected: I,
    ) -> impl use<'_, T, T2, I> + Future<Output = ()>
    where
        T: Debug + PartialEq<T2>,
    {
        FutureTrackingCaller {
            future: async {
                let mut expected: VecDeque<T2> = expected.into_iter().collect();

                while !expected.is_empty() {
                    // Like `next`, waiting for each expected message is safe mid-test; the
                    // taint on a forced `None` is unobservable because the test fails below.
                    if let Some(next) = self.try_next_impl().await {
                        let next_expected = expected.pop_front().unwrap();
                        if next != next_expected {
                            return Err(format!(
                                "Stream yielded unexpected message: {:?}, expected: {:?}",
                                next, next_expected
                            ));
                        }
                    } else {
                        return Err(format!(
                            "Stream ended early, still expected: {:?}",
                            expected
                        ));
                    }
                }

                Ok(())
            },
        }
    }

    /// Asserts that the stream yields only the expected sequence of messages, in order,
    /// and then ends (like [`Self::assert_no_more`], forking the search in exhaustive mode).
    pub fn assert_yields_only<T2: Debug, I: IntoIterator<Item = T2>>(
        &self,
        expected: I,
    ) -> impl use<'_, T, T2, I> + Future<Output = ()>
    where
        T: Debug + PartialEq<T2>,
    {
        ChainedFuture {
            first: self.assert_yields(expected),
            second: self.assert_no_more(),
            first_done: false,
        }
    }
}

pin_project_lite::pin_project! {
    // A future that tracks the location of the `.await` call for better panic messages.
    //
    // `#[track_caller]` is important for us to create assertion methods because it makes
    // the panic backtrace show up at that method (instead of inside the call tree within
    // that method). This is e.g. what `Option::unwrap` uses. Unfortunately, `#[track_caller]`
    // does not work correctly for async methods (or `dyn Future` either), so we have to
    // create these concrete future types that (1) have `#[track_caller]` on their `poll()`
    // method and (2) have the `panic!` triggered in their `poll()` method (or in a directly
    // nested concrete future).
    struct FutureTrackingCaller<F> {
        #[pin]
        future: F,
    }
}

impl<T, F: Future<Output = Result<T, String>>> Future for FutureTrackingCaller<F> {
    type Output = T;

    #[track_caller]
    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        match ready!(self.as_mut().project().future.poll(cx)) {
            Ok(v) => Poll::Ready(v),
            Err(e) => panic!("{}", e),
        }
    }
}

pin_project_lite::pin_project! {
    // A future that first awaits the first future, then the second, propagating caller info.
    //
    // See [`FutureTrackingCaller`] for context.
    struct ChainedFuture<F1: Future<Output = ()>, F2: Future<Output = ()>> {
        #[pin]
        first: F1,
        #[pin]
        second: F2,
        first_done: bool,
    }
}

impl<F1: Future<Output = ()>, F2: Future<Output = ()>> Future for ChainedFuture<F1, F2> {
    type Output = ();

    #[track_caller]
    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        if !self.first_done {
            ready!(self.as_mut().project().first.poll(cx));
            *self.as_mut().project().first_done = true;
        }

        self.as_mut().project().second.poll(cx)
    }
}

impl<T: Serialize + DeserializeOwned> SimReceiver<T, NoOrder, ExactlyOnce> {
    /// Receives the next `n` messages, sorted, waiting (and letting the scheduler run any
    /// pending simulation work) until they are available. If the simulation becomes quiescent
    /// before `n` messages arrive, the test fails.
    ///
    /// Like [`SimReceiver::next`], this is safe to use in the middle of a test.
    pub fn collect_n_sorted<C: Default + Extend<T> + AsMut<[T]>>(
        &self,
        n: usize,
    ) -> impl use<'_, T, C> + Future<Output = C>
    where
        T: Ord,
    {
        FutureTrackingCaller {
            future: async move {
                let mut out = C::default();
                for i in 0..n {
                    // Like `next`, waiting for each message is safe mid-test; the taint on a
                    // forced `None` is unobservable because the test fails below.
                    if let Some(v) = self.try_next_impl().await {
                        out.extend([v]);
                    } else {
                        return Err(format!(
                            "Stream ended (simulation quiescent) after {} messages, but {} were expected",
                            i, n
                        ));
                    }
                }
                out.as_mut().sort();
                Ok(out)
            },
        }
    }

    /// Collects all remaining messages from the external bincode stream into a collection,
    /// sorting them. This will wait until no more messages can possibly arrive.
    ///
    /// If this has to force pending nondeterministic work to run, it should be the last
    /// observation of the test; see [`collect`](SimReceiver::collect).
    pub async fn collect_sorted<C: Default + Extend<T> + AsMut<[T]>>(self) -> C
    where
        T: Ord,
    {
        let mut collected = C::default();
        while let Some(v) = self.try_next_impl().await {
            collected.extend([v]);
        }
        collected.as_mut().sort();
        collected
    }

    /// Asserts that the stream yields exactly the expected sequence of messages, in some order.
    /// This does not check that the stream ends, use [`Self::assert_yields_only_unordered`] for that.
    ///
    /// Like [`SimReceiver::next`], this is safe to use in the middle of a test.
    pub fn assert_yields_unordered<T2: Debug, I: IntoIterator<Item = T2>>(
        &self,
        expected: I,
    ) -> impl use<'_, T, T2, I> + Future<Output = ()>
    where
        T: Debug + PartialEq<T2>,
    {
        FutureTrackingCaller {
            future: async {
                let mut expected: Vec<T2> = expected.into_iter().collect();

                while !expected.is_empty() {
                    // Like `next`, waiting for each expected message is safe mid-test; the
                    // taint on a forced `None` is unobservable because the test fails below.
                    if let Some(next) = self.try_next_impl().await {
                        let idx = expected.iter().enumerate().find(|(_, e)| &next == *e);
                        if let Some((i, _)) = idx {
                            expected.swap_remove(i);
                        } else {
                            return Err(format!("Stream yielded unexpected message: {:?}", next));
                        }
                    } else {
                        return Err(format!(
                            "Stream ended early, still expected: {:?}",
                            expected
                        ));
                    }
                }

                Ok(())
            },
        }
    }

    /// Asserts that the stream yields only the expected sequence of messages, in some order,
    /// and then ends (like [`Self::assert_no_more`], forking the search in exhaustive mode).
    pub fn assert_yields_only_unordered<T2: Debug, I: IntoIterator<Item = T2>>(
        &self,
        expected: I,
    ) -> impl use<'_, T, T2, I> + Future<Output = ()>
    where
        T: Debug + PartialEq<T2>,
    {
        ChainedFuture {
            first: self.assert_yields_unordered(expected),
            second: self.assert_no_more(),
            first_done: false,
        }
    }
}

impl<T: Serialize + DeserializeOwned, O: Ordering, R: Retries> SimSender<T, O, R> {
    fn with_sink<Out>(&self, thunk: impl FnOnce(&dyn Fn(T)) -> Out) -> Out {
        let (sender, quiescence) = CURRENT_SIM_CONNECTIONS.with(|connections| {
            let connections = connections.borrow();
            (
                connections
                    .input_senders
                    .get(connections.external_registered.get(&self.0).unwrap())
                    .unwrap()
                    .clone(),
                connections.quiescence.clone(),
            )
        });

        thunk(&move |t| {
            sender
                .try_send(bincode::serialize(&t).unwrap().into())
                .unwrap();
            quiescence.resume();
        })
    }
}

impl<T: Serialize + DeserializeOwned, O: Ordering> SimSender<T, O, ExactlyOnce> {
    /// Sends several messages to the external bincode sink. The messages will be asynchronously
    /// processed as part of the simulation, in non-deterministic order.
    pub fn send_many_unordered<I: IntoIterator<Item = T>>(&self, iter: I) {
        self.with_sink(|send| {
            for t in iter {
                send(t);
            }
        })
    }
}

impl<T: Serialize + DeserializeOwned> SimSender<T, TotalOrder, ExactlyOnce> {
    /// Sends a message to the external bincode sink. The message will be asynchronously processed
    /// as part of the simulation.
    pub fn send(&self, t: T) {
        self.with_sink(|send| send(t));
    }

    /// Sends several messages to the external bincode sink. The messages will be asynchronously
    /// processed as part of the simulation.
    pub fn send_many<I: IntoIterator<Item = T>>(&self, iter: I) {
        self.with_sink(|send| {
            for t in iter {
                send(t);
            }
        })
    }
}

impl<T: Serialize + DeserializeOwned, O: Ordering, R: Retries> Clone
    for SimClusterReceiver<T, O, R>
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Serialize + DeserializeOwned, O: Ordering, R: Retries> Copy
    for SimClusterReceiver<T, O, R>
{
}

impl<T: Serialize + DeserializeOwned, O: Ordering, R: Retries> SimClusterReceiver<T, O, R> {
    fn member_connections(
        &self,
        member_id: u32,
    ) -> (Rc<Mutex<UnsyncReceiver<Bytes>>>, Rc<QuiescenceState>) {
        CURRENT_SIM_CONNECTIONS.with(|connections| {
            let connections = connections.borrow();
            let port = connections.external_registered.get(&self.0).unwrap();
            let receivers = connections.cluster_output_receivers.get(port).unwrap();
            (
                receivers[&member_id].clone(),
                connections.quiescence.clone(),
            )
        })
    }

    /// See [`try_next_bytes`].
    async fn try_next_impl(&self, member_id: u32) -> Option<T> {
        let (receiver, quiescence) = self.member_connections(member_id);
        try_next_bytes(&receiver, &quiescence)
            .await
            .map(|bytes| bincode::deserialize(&bytes).unwrap())
    }
}

impl<T: Serialize + DeserializeOwned> SimClusterReceiver<T, TotalOrder, ExactlyOnce> {
    /// Receives the next value from a specific cluster member, waiting (and letting the
    /// scheduler run any pending simulation work) until one is available. If the simulation
    /// becomes quiescent without producing a value, the test fails.
    ///
    /// This is safe to use in the middle of a test; to observe the *absence* of a value,
    /// use [`Self::try_next`].
    pub fn next(&self, member_id: u32) -> impl use<'_, T> + Future<Output = T> {
        // See `SimReceiver::next` for why waiting for a value never "overruns" the
        // simulation.
        FutureTrackingCaller {
            future: async move {
                self.try_next_impl(member_id).await.ok_or_else(|| {
                    "Stream ended (simulation quiescent), but another message was expected"
                        .to_owned()
                })
            },
        }
    }

    /// Receives the next value from a specific cluster member, or returns `None` if no more
    /// values can possibly arrive.
    ///
    /// If answering requires forcing pending nondeterministic work to run, then afterwards,
    /// sending more input and then attempting to receive output will panic. Prefer
    /// [`Self::next`] when possible.
    pub async fn try_next(&self, member_id: u32) -> Option<T> {
        self.try_next_impl(member_id).await
    }

    /// Collects all remaining values from a specific cluster member into a collection,
    /// waiting until no more values can possibly arrive.
    ///
    /// If this has to force pending nondeterministic work to run, it should be the last
    /// observation of the test; see [`SimReceiver::collect`].
    pub async fn collect<C: Default + Extend<T>>(self, member_id: u32) -> C {
        let mut out = C::default();
        while let Some(v) = self.try_next_impl(member_id).await {
            out.extend([v]);
        }
        out
    }
}

impl<T: Serialize + DeserializeOwned> SimClusterReceiver<T, NoOrder, ExactlyOnce> {
    /// Receives the next `n` values from a specific cluster member, sorted, waiting (and
    /// letting the scheduler run any pending simulation work) until they are available. If
    /// the simulation becomes quiescent before `n` values arrive, the test fails.
    ///
    /// Like [`SimReceiver::next`], this is safe to use in the middle of a test.
    pub fn collect_n_sorted<C: Default + Extend<T> + AsMut<[T]>>(
        &self,
        member_id: u32,
        n: usize,
    ) -> impl use<'_, T, C> + Future<Output = C>
    where
        T: Ord,
    {
        FutureTrackingCaller {
            future: async move {
                let mut out = C::default();
                for i in 0..n {
                    // Like `SimReceiver::next`, waiting for each message is safe mid-test;
                    // the taint on a forced `None` is unobservable because the test fails
                    // below.
                    if let Some(v) = self.try_next_impl(member_id).await {
                        out.extend([v]);
                    } else {
                        return Err(format!(
                            "Stream ended (simulation quiescent) after {} messages, but {} were expected",
                            i, n
                        ));
                    }
                }
                out.as_mut().sort();
                Ok(out)
            },
        }
    }

    /// Collects all remaining values from a specific cluster member, sorted, waiting until no
    /// more values can possibly arrive.
    ///
    /// If this has to force pending nondeterministic work to run, it should be the last
    /// observation of the test; see [`SimReceiver::collect`].
    pub async fn collect_sorted<C: Default + Extend<T> + AsMut<[T]>>(self, member_id: u32) -> C
    where
        T: Ord,
    {
        let mut collected = C::default();
        while let Some(v) = self.try_next_impl(member_id).await {
            collected.extend([v]);
        }
        collected.as_mut().sort();
        collected
    }
}

impl<T: Serialize + DeserializeOwned, O: Ordering, R: Retries> SimClusterSender<T, O, R> {
    fn with_sink<Out>(&self, thunk: impl FnOnce(&dyn Fn(u32, T)) -> Out) -> Out {
        let (senders, quiescence) = CURRENT_SIM_CONNECTIONS.with(|connections| {
            let connections = connections.borrow();
            (
                connections
                    .cluster_input_senders
                    .get(connections.external_registered.get(&self.0).unwrap())
                    .unwrap()
                    .clone(),
                connections.quiescence.clone(),
            )
        });

        thunk(&move |member_id: u32, t: T| {
            let payload = bincode::serialize(&t).unwrap();
            senders[&member_id].try_send(Bytes::from(payload)).unwrap();
            quiescence.resume();
        })
    }
}

impl<T: Serialize + DeserializeOwned, O: Ordering> SimClusterSender<T, O, ExactlyOnce> {
    /// Sends multiple values to specific cluster members. The messages will be asynchronously
    /// processed as part of the simulation, in non-deterministic order.
    pub fn send_many_unordered<I: IntoIterator<Item = (u32, T)>>(&self, iter: I) {
        self.with_sink(|send| {
            for (member_id, t) in iter {
                send(member_id, t);
            }
        })
    }
}

impl<T: Serialize + DeserializeOwned> SimClusterSender<T, TotalOrder, ExactlyOnce> {
    /// Sends a value to a specific cluster member.
    pub fn send(&self, member_id: u32, t: T) {
        self.with_sink(|send| send(member_id, t));
    }

    /// Sends multiple values to specific cluster members.
    pub fn send_many<I: IntoIterator<Item = (u32, T)>>(&self, iter: I) {
        self.with_sink(|send| {
            for (member_id, t) in iter {
                send(member_id, t);
            }
        })
    }
}

enum LogKind<W: std::io::Write> {
    Null,
    Stderr,
    Custom(W),
}

// via https://www.reddit.com/r/rust/comments/t69sld/is_there_a_way_to_allow_either_stdfmtwrite_or/
impl<W: std::io::Write> std::fmt::Write for LogKind<W> {
    fn write_str(&mut self, s: &str) -> Result<(), std::fmt::Error> {
        match self {
            LogKind::Null => Ok(()),
            LogKind::Stderr => {
                eprint!("{}", s);
                Ok(())
            }
            LogKind::Custom(w) => w.write_all(s.as_bytes()).map_err(|_| std::fmt::Error),
        }
    }
}

/// A tick-scoped DFIR together with the hooks that feed it data.
struct SimTick {
    /// The location of the process/cluster the tick lives on, used to match this tick
    /// against the async DFIR that produces its input data.
    parent_location: LocationId,
    /// The cluster member ID, if the tick lives on a cluster.
    cluster_id: Option<u32>,
    /// The tick DFIR, executed once per tick.
    dfir: DfirErased,
    /// Hooks (e.g. from `batch`) resolved *before* the tick runs, deciding what data to
    /// release into it.
    hooks: Vec<Box<dyn SimHook>>,
    /// Hooks (e.g. from `assume_ordering` inside the tick) resolved *while* the tick DFIR
    /// is running, via a `tokio::select!` loop, for operators that block on ordering
    /// decisions mid-tick.
    inline_hooks: Vec<Box<dyn SimInlineHook>>,
}

impl SimTick {
    /// Whether the scheduler can execute this tick right now.
    fn can_run(&self) -> bool {
        // All hooks must be ready (have received input or have a last value)...
        self.hooks.iter().all(|hook| hook.is_ready())
            // ...and at least one hook must be able to release data into the tick.
            && self.hooks.iter().any(|hook| hook_can_release(&**hook))
    }
}

/// A single top-level hook (e.g. from `assume_ordering` on a non-tick stream) that needs
/// scheduling decisions, but has no tick DFIR to execute. The scheduler just resolves the
/// hook.
///
/// Each top-level hook is its own observation ("its own virtual tick"), even when several
/// hooks live at the same location: unlike a tick's hooks, which one atomic tick
/// execution consumes together, co-located top-level hooks are causally independent
/// operators, so resolving them jointly would only couple their decisions. Grouping them
/// would both add redundant schedules (releasing jointly is equivalent to releasing in
/// consecutive steps, which is explored anyway) and *lose* schedules for hook kinds whose
/// decisions always release when resolved (a fold could never stay silent while a
/// co-located sibling acts). With one hook per observation, "act" and "stay silent" are
/// expressed purely by the scheduler picking or not picking the observation, and a picked
/// observation always makes a nontrivial decision.
struct SimObservation {
    /// The top-level location, used to match this observation against the async DFIR that
    /// produces its input data.
    location: LocationId,
    /// The cluster member ID, if the location is a cluster.
    cluster_id: Option<u32>,
    /// The hook resolved when the scheduler selects this observation.
    hook: Box<dyn SimHook>,
}

impl SimObservation {
    /// Whether the scheduler can resolve this observation's hook right now.
    fn can_run(&self) -> bool {
        hook_can_release(&*self.hook)
    }
}

/// Whether the hook has already decided to release data, or has pending input that would
/// allow it to decide to do so.
fn hook_can_release(hook: &dyn SimHook) -> bool {
    hook.current_decision().unwrap_or(false) || hook.can_make_nontrivial_decision()
}

/// A running simulation, which manages the async DFIRs, tick DFIRs, and hook-based
/// scheduling decisions for non-deterministic operators like `batch` and `assume_ordering`.
///
/// This struct holds all simulator state across scheduler steps. Each [`Self::step`] performs
/// one of three kinds of work:
/// - **Async DFIRs**: long-running top-level dataflows (one per process/cluster member) that
///   produce data consumed by ticks and observations.
/// - **Ticks**: tick-scoped DFIRs that execute a single tick. Before running, their associated
///   hooks (e.g. from `batch`) are resolved to decide what data to release into the tick.
/// - **Observations**: top-level locations that have hooks (e.g. from `assume_ordering` on a
///   non-tick stream) needing decisions, but no tick DFIR to execute. The scheduler just
///   resolves their hooks.
struct LaunchedSim<W: std::io::Write> {
    /// Top-level async DFIRs, one per process/cluster member. These run continuously and
    /// produce data that feeds into ticks and observations.
    async_dfirs: Vec<(LocationId, Option<u32>, DfirErased)>,
    /// Ticks whose parent async DFIR has made progress, so they may be ready to run.
    /// The scheduler further filters these by checking whether their hooks have pending decisions.
    possibly_ready_ticks: Vec<SimTick>,
    /// Ticks whose parent async DFIR has not yet made progress since they were last checked.
    not_ready_ticks: Vec<SimTick>,
    /// Observations whose async DFIR has made progress, so their hooks may have decisions
    /// to resolve.
    possibly_ready_observations: Vec<SimObservation>,
    /// Observations whose async DFIR has not yet made progress since they were last checked.
    not_ready_observations: Vec<SimObservation>,
    log: LogKind<W>,
    /// Represents quiescence state of the simulation.
    quiescence: Rc<QuiescenceState>,
}

impl<W: std::io::Write> LaunchedSim<W> {
    /// Runs a single step of the simulation scheduler.
    ///
    /// A step first advances all async DFIRs; if none of them made progress, it instead runs
    /// one ready tick or resolves one ready observation. If nothing at all can make progress,
    /// the simulation is quiescent: this signals waiting receivers and returns; the driver is
    /// responsible for parking until new external input arrives (see
    /// [`QuiescenceState::resumed`]).
    ///
    /// This future is always awaited to completion by the driver, so a step is atomic: user
    /// code never runs (and never observes intermediate state) while a step is in flight.
    async fn step(&mut self) {
        let mut any_made_progress = false;
        for (loc, c_id, dfir) in &mut self.async_dfirs {
            if dfir.run_tick().await {
                any_made_progress = true;

                // This async DFIR may have produced new data, so the ticks and observations
                // it feeds may now be ready.
                self.possibly_ready_ticks
                    .extend(self.not_ready_ticks.extract_if(.., |tick| {
                        tick.parent_location == *loc && tick.cluster_id == *c_id
                    }));
                self.possibly_ready_observations.extend(
                    self.not_ready_observations
                        .extract_if(.., |obs| obs.location == *loc && obs.cluster_id == *c_id),
                );
            }
        }

        if any_made_progress {
            return;
        }

        use bolero::generator::*;

        // Send anything that can't make a scheduling decision back to the not-ready lists.
        self.not_ready_ticks.extend(
            self.possibly_ready_ticks
                .extract_if(.., |tick| !tick.can_run()),
        );
        self.not_ready_observations.extend(
            self.possibly_ready_observations
                .extract_if(.., |obs| !obs.can_run()),
        );

        if self.possibly_ready_ticks.is_empty() && self.possibly_ready_observations.is_empty() {
            // If any tick is blocked because a hook is not ready, that's a
            // simulator bug — it means a singleton never received a value.
            for tick in &self.not_ready_ticks {
                abort_assert!(
                    tick.hooks.iter().all(|hook| hook.is_ready()),
                    "tick has a hook that never became ready"
                );
            }

            // Signal quiescence, waking receivers waiting for data (their streams end). The
            // driver is responsible for parking until new input arrives.
            self.quiescence.enter_quiescence();
        } else if self.quiescence.pause_nondet.get() > 0 {
            // The test is querying whether the simulation can quiesce without
            // nondeterministic work (see `SettlePauseGuard::poll_settle`). Report that
            // ticks/observations are pending and pause; the driver parks until the test
            // decides how to proceed.
            self.quiescence.nondet_pending.set(true);
            self.quiescence.wake_settled();
        } else {
            let next_tick_or_obs = (0..(self.possibly_ready_ticks.len()
                + self.possibly_ready_observations.len()))
                .any();

            if next_tick_or_obs < self.possibly_ready_ticks.len() {
                let mut tick = self.possibly_ready_ticks.remove(next_tick_or_obs);

                match &mut self.log {
                    LogKind::Null => {}
                    LogKind::Stderr => {
                        if let Some(cid) = &tick.cluster_id {
                            eprintln!(
                                "\n{}",
                                format!("Running Tick (Cluster Member {})", cid)
                                    .color(colored::Color::Magenta)
                                    .bold()
                            )
                        } else {
                            eprintln!("\n{}", "Running Tick".color(colored::Color::Magenta).bold())
                        }
                    }
                    LogKind::Custom(writer) => {
                        writeln!(
                            writer,
                            "\n{}",
                            "Running Tick".color(colored::Color::Magenta).bold()
                        )
                        .unwrap();
                    }
                }

                let mut asterisk_indenter = |_line_no, write: &mut dyn std::fmt::Write| {
                    write.write_str(&"*".color(colored::Color::Magenta).bold())?;
                    write.write_str(" ")
                };

                let mut tick_decision_writer = (!matches!(self.log, LogKind::Null)).then(|| {
                    indenter::indented(&mut self.log).with_format(indenter::Format::Custom {
                        inserter: &mut asterisk_indenter,
                    })
                });

                run_hooks(tick_decision_writer.as_mut(), &mut tick.hooks);

                let run_tick_future = tick.dfir.run_tick();
                if !tick.inline_hooks.is_empty() {
                    let mut run_tick_future_pinned = pin!(run_tick_future);

                    loop {
                        tokio::select! {
                            biased;
                            r = &mut run_tick_future_pinned => {
                                abort_assert!(r, "tick DFIR run_tick() returned false");
                                break;
                            }
                            _ = async {} => {
                                bolero_generator::any::scope::borrow_with(|driver| {
                                    for hook in tick.inline_hooks.iter_mut() {
                                        if hook.pending_decision() {
                                            if !hook.has_decision() {
                                                hook.autonomous_decision(driver);
                                            }

                                            hook.release_decision(
                                                tick_decision_writer
                                                    .as_mut()
                                                    .map(|w| w as &mut dyn std::fmt::Write),
                                            );
                                        }
                                    }
                                });
                            }
                        }
                    }
                } else {
                    abort_assert!(run_tick_future.await, "tick DFIR run_tick() returned false");
                }

                self.possibly_ready_ticks.push(tick);
            } else {
                let next_obs = next_tick_or_obs - self.possibly_ready_ticks.len();
                let log_writer = (!matches!(self.log, LogKind::Null)).then_some(&mut self.log);
                run_hooks(
                    log_writer,
                    std::slice::from_mut(&mut self.possibly_ready_observations[next_obs].hook),
                );
            }
        }
    }
}

fn run_hooks<W: std::fmt::Write>(
    mut tick_decision_writer: Option<&mut W>,
    hooks: &mut [Box<dyn SimHook>],
) {
    let mut remaining_decision_count = hooks.len();
    let mut made_nontrivial_decision = false;

    bolero::generator::bolero_generator::any::scope::borrow_with(|driver| {
        // first, scan manual decisions
        hooks.iter_mut().for_each(|hook| {
            if let Some(is_nontrivial) = hook.current_decision() {
                made_nontrivial_decision |= is_nontrivial;
                remaining_decision_count -= 1;
            } else if !hook.can_make_nontrivial_decision() {
                // if no nontrivial decision is possible, make a trivial one
                // (we need to do this in the first pass to force nontrivial decisions
                // on the remaining hooks)
                hook.autonomous_decision(driver, false);
                remaining_decision_count -= 1;
            }
        });

        hooks.iter_mut().for_each(|hook| {
            if hook.current_decision().is_none() {
                made_nontrivial_decision |= hook.autonomous_decision(
                    driver,
                    !made_nontrivial_decision && remaining_decision_count == 1,
                );
                remaining_decision_count -= 1;
            }

            hook.release_decision(
                tick_decision_writer
                    .as_deref_mut()
                    .map(|w| w as &mut dyn std::fmt::Write),
            );
        });
    });
}
