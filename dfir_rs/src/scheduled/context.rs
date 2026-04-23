//! Module for the inline DFIR runtime context and execution engine.
//!
//! Provides [`Context`] (the lightweight operator context) and
//! [`Dfir`] (the dataflow execution wrapper).

use std::any::Any;
use std::future::Future;
use std::marker::PhantomData;
use std::ops::DerefMut;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::task::Wake;

#[cfg(feature = "meta")]
use dfir_lang::diagnostic::{Diagnostic, Diagnostics, SerdeSpan};
#[cfg(feature = "meta")]
use dfir_lang::graph::DfirGraph;

use super::metrics::{DfirMetrics, DfirMetricsIntervals};
use super::state::StateHandle;
use super::{StateLifespan, StateTag, SubgraphId};
use crate::scheduled::ticks::TickInstant;
use crate::util::slot_vec::SlotVec;

/// Internal state storage for operator accumulators.
struct StateData {
    state: Box<dyn Any>,
    lifespan_hook_fn: Option<LifespanResetFn>,
    /// `None` for static.
    lifespan: Option<StateLifespan>,
}
type LifespanResetFn = Box<dyn FnMut(&mut dyn Any)>;

/// Coordinates waking between [`Context`] (inside the tick closure) and [`Dfir`]
/// (the external runner). Shared via `Arc` between both.
///
/// When external data arrives (e.g., a tokio stream receives a message), the [`Context::waker`]
/// fires, which sets `can_start_tick` and wakes the [`Dfir::run`](Dfir::run) task so it starts a new tick.
/// Implements [`Wake`] directly so it can be used as a `Waker` without an extra wrapper.
#[doc(hidden)]
pub struct WakeState {
    /// Set to `true` when external data arrives, signaling that a new tick should run.
    /// Checked by [`Dfir::run_tick`](Dfir::run_tick) and [`Dfir::run_available`](Dfir::run_available).
    can_start_tick: std::sync::atomic::AtomicBool,
    /// Wakes the [`Dfir::run`](Dfir::run) task from its idle `poll_fn` sleep.
    task_waker: futures::task::AtomicWaker,
}

impl Default for WakeState {
    fn default() -> Self {
        Self {
            can_start_tick: std::sync::atomic::AtomicBool::new(false),
            task_waker: futures::task::AtomicWaker::new(),
        }
    }
}

impl Wake for WakeState {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.can_start_tick.store(true, Ordering::Relaxed);
        self.task_waker.wake();
    }
}

/// A lightweight context for inline codegen that avoids the overhead of the full
/// [`Context`] (no tokio channels, no scheduler queues, no loop machinery).
///
/// Exposes the same method names that operator-generated code calls on both
/// `df` (for prologues: `add_state`, `set_state_lifespan_hook`) and
/// `context` (for iterators: `state_ref_unchecked`, `is_first_run_this_tick`, etc.).
#[doc(hidden)]
#[derive(Default)]
pub struct Context {
    /// Storage for the operator-facing State API.
    states: SlotVec<StateTag, StateData>,
    /// Counter for number of ticks run.
    current_tick: TickInstant,
    /// Coordinates waking between [`Context`] (inside the tick closure) and [`Dfir`]
    /// (the external runner). Shared via `Arc` between both. Implements [`Wake`].
    wake_state: Arc<WakeState>,
    /// Live-updating DFIR runtime metrics via interior mutability.
    metrics: Rc<DfirMetrics>,
}

impl Context {
    /// Create a new inline context with shared wake state and metrics.
    pub fn new(wake_state: Arc<WakeState>, metrics: Rc<DfirMetrics>) -> Self {
        Self {
            states: SlotVec::new(),
            current_tick: TickInstant::default(),
            wake_state,
            metrics,
        }
    }

    // --- Methods called as `df.xxx()` in operator prologues ---

    /// Adds state and returns a handle.
    pub fn add_state<T>(&mut self, state: T) -> StateHandle<T>
    where
        T: Any,
    {
        let state_data = StateData {
            state: Box::new(state),
            lifespan_hook_fn: None,
            lifespan: None,
        };
        let state_id = self.states.insert(state_data);
        StateHandle {
            state_id,
            _phantom: PhantomData,
        }
    }

    /// Sets a hook to modify state at the end of each tick.
    pub fn set_state_lifespan_hook<T>(
        &mut self,
        handle: StateHandle<T>,
        _lifespan: StateLifespan,
        mut hook_fn: impl 'static + FnMut(&mut T),
    ) where
        T: Any,
    {
        let state_data = self
            .states
            .get_mut(handle.state_id)
            .expect("Failed to find state with given handle.");
        state_data.lifespan_hook_fn = Some(Box::new(move |state| {
            (hook_fn)(state.downcast_mut::<T>().unwrap());
        }));
        state_data.lifespan = Some(_lifespan);
    }

    // --- Methods called as `context.xxx()` in operator iterators ---

    /// Returns a shared reference to the state.
    ///
    /// # Safety
    /// `StateHandle<T>` must be from _this_ instance.
    pub unsafe fn state_ref_unchecked<T>(&self, handle: StateHandle<T>) -> &'_ T
    where
        T: Any,
    {
        let state = self
            .states
            .get(handle.state_id)
            .expect("Failed to find state with given handle.")
            .state
            .as_ref();
        debug_assert!(state.is::<T>());
        unsafe { &*(state as *const dyn Any as *const T) }
    }

    /// Always returns `true` in inline mode. The inline codegen runs the entire DAG
    /// once per tick with no re-execution, so every subgraph is always on its first
    /// (and only) run within each tick.
    pub fn is_first_run_this_tick(&self) -> bool {
        true
    }

    /// Gets the current tick count.
    pub fn current_tick(&self) -> TickInstant {
        self.current_tick
    }

    /// Returns a reference to the runtime metrics.
    pub fn metrics(&self) -> &Rc<DfirMetrics> {
        &self.metrics
    }

    /// No-op: inline mode has no subgraph scheduling.
    pub fn current_subgraph(&self) -> SubgraphId {
        SubgraphId::from_raw(0)
    }

    /// In inline mode, every subgraph runs unconditionally each tick, so the `sg_id`
    /// parameter is ignored. Only `is_external` matters: when `true`, it signals that
    /// external data has arrived and a new tick should be started.
    pub fn schedule_subgraph(&self, _sg_id: SubgraphId, is_external: bool) {
        if is_external {
            self.wake_state.wake_by_ref();
        }
    }

    /// Returns a waker that signals external data has arrived.
    pub fn waker(&self) -> std::task::Waker {
        std::task::Waker::from(self.wake_state.clone())
    }

    /// Runs end-of-tick state hooks and increments the tick counter.
    /// Called by the generated tick closure at the end of each tick.
    #[doc(hidden)]
    pub fn __end_tick(&mut self) {
        for state_data in self.states.values_mut() {
            let StateData {
                state,
                lifespan_hook_fn: Some(lifespan_hook_fn),
                lifespan: Some(StateLifespan::Tick),
            } = state_data
            else {
                continue;
            };
            (lifespan_hook_fn)(Box::deref_mut(state));
        }
        self.current_tick += crate::scheduled::ticks::TickDuration::SINGLE_TICK;
    }
}

/// A wrapper around an inline-codegen tick closure that provides [`Self::run`],
/// [`Self::run_available`], and [`Self::run_tick`] methods — mirroring the [`Dfir`](super::context::Dfir)
/// API.
///
/// # Design
///
/// The inline codegen generates an `async move |df: &mut Context|` closure that captures
/// dataflow-specific state (handoff buffers, source iterators) and receives the [`Context`]
/// (operator accumulators, tick counter) by reference each tick. `Dfir` owns both the
/// closure and the context, and coordinates tick lifecycle and idle/wake behavior.
///
/// We use a single opaque closure rather than generating a bespoke struct per dataflow because:
/// - The closure naturally captures exactly the state it needs with correct lifetimes
/// - No codegen needed for struct definitions, field accessors, or initialization
/// - Rust's async closure machinery handles the complex state machine (suspend/resume across
///   `.await` points) that would be very difficult to replicate in a generated struct
///
/// The `Tick` type parameter is bounded by [`TickClosure`] (not `AsyncFnMut` directly) to
/// support type erasure via [`TickClosureErased`] / [`DfirErased`] for heterogeneous
/// collections (e.g., the sim runtime storing multiple locations in a `Vec`). The concrete
/// (non-erased) path used by trybuild and embedded has zero overhead.
#[doc(hidden)]
pub struct Dfir<Tick> {
    /// Async closure which runs a single tick when called.
    tick_closure: Tick,
    /// Coordinates waking between [`Context`] (inside the tick closure) and [`Dfir`]
    /// (the external runner). Shared via `Arc` between both. Implements [`Wake`].
    wake_state: Arc<WakeState>,
    /// The inline context, owned by `Dfir` and passed to the tick closure by reference.
    context: Context,
    /// See [`Self::meta_graph()`].
    #[cfg(feature = "meta")]
    meta_graph: Option<DfirGraph>,
    /// See [`Self::diagnostics()`].
    #[cfg(feature = "meta")]
    diagnostics: Option<Vec<Diagnostic<SerdeSpan>>>,
}

/// Trait for tick closures — abstracts over both concrete async closures
/// and type-erased boxed versions ([`TickClosureErased`]).
///
/// The `&mut Context` parameter is owned by [`Dfir`] and lent to the
/// closure each tick, avoiding shared-ownership overhead for the context.
#[doc(hidden)]
pub trait TickClosure {
    /// Call the tick closure. Returns `true` if any subgraph received input data.
    fn call_tick<'a>(&'a mut self, ctx: &'a mut Context) -> impl Future<Output = bool> + 'a;
}

impl<F: for<'a> AsyncFnMut(&'a mut Context) -> bool> TickClosure for F {
    fn call_tick<'a>(&'a mut self, ctx: &'a mut Context) -> impl Future<Output = bool> + 'a {
        self(ctx)
    }
}

/// No-op `TickClosure`.
#[doc(hidden)]
pub struct NullTickClosure;

impl TickClosure for NullTickClosure {
    fn call_tick<'a>(&'a mut self, _ctx: &'a mut Context) -> impl Future<Output = bool> + 'a {
        std::future::ready(false)
    }
}

/// Type-erased tick function for use in heterogeneous collections (e.g., the sim runtime).
#[doc(hidden)]
pub struct TickClosureErased(Box<dyn TickClosureErasedInner>);

/// Object-safe inner trait for [`TickClosureErased`]. Needed because `AsyncFnMut` is not
/// object-safe (GAT return type), but a trait with `&mut self -> Pin<Box<dyn Future + '_>>`
/// is — the returned future borrows from the trait object which owns the closure.
trait TickClosureErasedInner {
    fn call_tick<'a>(
        &'a mut self,
        ctx: &'a mut Context,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>>;
}

impl<F: for<'a> AsyncFnMut(&'a mut Context) -> bool> TickClosureErasedInner for F {
    fn call_tick<'a>(
        &'a mut self,
        ctx: &'a mut Context,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        Box::pin(self(ctx))
    }
}

impl TickClosure for TickClosureErased {
    fn call_tick<'a>(&'a mut self, ctx: &'a mut Context) -> impl Future<Output = bool> + 'a {
        self.0.call_tick(ctx)
    }
}

/// Type alias for a type-erased [`Dfir`] that can be stored in heterogeneous collections.
/// Created via [`Dfir::into_erased`].
pub type DfirErased = Dfir<TickClosureErased>;

impl<Tick: TickClosure> Dfir<Tick> {
    /// Create a new `Dfir` from a tick closure, inline context,
    /// and meta graph / diagnostics JSON strings.
    #[doc(hidden)]
    pub fn new(
        tick_closure: Tick,
        context: Context,
        meta_graph_json: Option<&str>,
        diagnostics_json: Option<&str>,
    ) -> Self {
        #[cfg(not(feature = "meta"))]
        let _ = (meta_graph_json, diagnostics_json);
        Self {
            tick_closure,
            wake_state: context.wake_state.clone(),
            context,
            #[cfg(feature = "meta")]
            meta_graph: meta_graph_json.map(|json| {
                let mut meta_graph: DfirGraph =
                    serde_json::from_str(json).expect("Failed to deserialize graph.");
                let mut op_inst_diagnostics = Diagnostics::new();
                meta_graph.insert_node_op_insts_all(&mut op_inst_diagnostics);
                assert!(
                    op_inst_diagnostics.is_empty(),
                    "Expected no diagnostics, got: {:#?}",
                    op_inst_diagnostics
                );
                meta_graph
            }),
            #[cfg(feature = "meta")]
            diagnostics: diagnostics_json.map(|json| {
                serde_json::from_str(json).expect("Failed to deserialize diagnostics.")
            }),
        }
    }

    /// Return a handle to the meta graph, if set.
    #[cfg(feature = "meta")]
    #[cfg_attr(docsrs, doc(cfg(feature = "meta")))]
    pub fn meta_graph(&self) -> Option<&DfirGraph> {
        self.meta_graph.as_ref()
    }

    /// Returns any diagnostics generated by the surface syntax macro.
    #[cfg(feature = "meta")]
    #[cfg_attr(docsrs, doc(cfg(feature = "meta")))]
    pub fn diagnostics(&self) -> Option<&[Diagnostic<SerdeSpan>]> {
        self.diagnostics.as_deref()
    }

    /// Returns a reference-counted handle to the continually-updated runtime metrics for this DFIR instance.
    pub fn metrics(&self) -> Rc<DfirMetrics> {
        Rc::clone(self.context.metrics())
    }

    /// Gets the current tick (local time) count.
    pub fn current_tick(&self) -> TickInstant {
        self.context.current_tick()
    }

    /// Returns a [`DfirMetricsIntervals`] handle where each call to
    /// [`DfirMetricsIntervals::take_interval`] ends the current interval and returns its metrics.
    ///
    /// The first call to `take_interval` returns metrics since this DFIR instance was created. Each subsequent call to
    /// `take_interval` returns metrics since the previous call.
    ///
    /// Cloning the handle "forks" it from the original, as afterwards each interval may return different metrics
    /// depending on when exactly `take_interval` is called.
    pub fn metrics_intervals(&self) -> DfirMetricsIntervals {
        DfirMetricsIntervals {
            curr: self.metrics(),
            prev: None,
        }
    }
}

impl<Tick: TickClosure> Dfir<Tick> {
    /// Run a single tick. Returns `true` if any subgraph received input data.
    ///
    /// Checks both handoff buffers (via `work_done` flag set in generated recv port code)
    /// and external events (via `can_start_tick` set by wakers/schedule_subgraph).
    pub async fn run_tick(&mut self) -> bool {
        let had_external = self
            .wake_state
            .can_start_tick
            .swap(false, Ordering::Relaxed);
        let tick_had_work = self.tick_closure.call_tick(&mut self.context).await;
        had_external || tick_had_work || self.wake_state.can_start_tick.load(Ordering::Relaxed)
    }

    /// Run a single tick synchronously. Panics if the tick yields (async suspension).
    /// Returns `true` if work was done (see [`Self::run_tick`]).
    pub fn run_tick_sync(&mut self) -> bool {
        let mut fut = std::pin::pin!(self.run_tick());
        let mut ctx = std::task::Context::from_waker(std::task::Waker::noop());
        match fut.as_mut().poll(&mut ctx) {
            std::task::Poll::Ready(result) => result,
            std::task::Poll::Pending => {
                panic!("Dfir::run_tick_sync: tick yielded asynchronously.")
            }
        }
    }

    /// Run ticks as long as work is available, then return.
    pub async fn run_available(&mut self) {
        // Always run at least one tick.
        self.wake_state
            .can_start_tick
            .store(false, Ordering::Relaxed);
        loop {
            self.run_tick().await;
            let can_start_tick = self
                .wake_state
                .can_start_tick
                .swap(false, Ordering::Relaxed);
            if !can_start_tick {
                break;
            }
            // Yield between each tick to receive more events.
            tokio::task::yield_now().await;
        }
    }

    /// [`Self::run_available`] but panics if any tick yields asynchronously.
    pub fn run_available_sync(&mut self) {
        self.wake_state
            .can_start_tick
            .store(false, Ordering::Relaxed);
        loop {
            self.run_tick_sync();
            let can_start_tick = self
                .wake_state
                .can_start_tick
                .swap(false, Ordering::Relaxed);
            if !can_start_tick {
                break;
            }
        }
    }

    /// Run forever, processing ticks when work is available and yielding when idle.
    pub async fn run(&mut self) -> crate::Never {
        loop {
            self.run_available().await;
            // Wait for an external event to wake us.
            std::future::poll_fn(|cx| {
                // Register waker first to avoid race: if an event fires between
                // the check and the register, the waker is already in place.
                self.wake_state.task_waker.register(cx.waker());
                if self.wake_state.can_start_tick.load(Ordering::Relaxed) {
                    std::task::Poll::Ready(())
                } else {
                    std::task::Poll::Pending
                }
            })
            .await;
        }
    }
}

impl<Tick: 'static + for<'a> AsyncFnMut(&'a mut Context) -> bool> Dfir<Tick> {
    /// Type-erase the tick closure for use in heterogeneous collections.
    ///
    /// Wraps the concrete async closure in [`TickClosureErased`], which boxes the future
    /// returned by each tick call. This adds one heap allocation per tick, but enables
    /// storing multiple `Dfir`s with different closure types in a single `Vec`.
    ///
    /// Only needed for the sim runtime path. The trybuild and embedded paths keep the
    /// concrete type and pay no erasure cost.
    pub fn into_erased(self) -> DfirErased {
        Dfir {
            tick_closure: TickClosureErased(Box::new(self.tick_closure)),
            wake_state: self.wake_state,
            context: self.context,
            #[cfg(feature = "meta")]
            meta_graph: self.meta_graph,
            #[cfg(feature = "meta")]
            diagnostics: self.diagnostics,
        }
    }
}
