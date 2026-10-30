//! Module for the inline DFIR runtime context and execution engine.
//!
//! Provides [`Context`] (the lightweight operator context) and
//! [`Dfir`] (the dataflow execution wrapper).

use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::task::Wake;

#[cfg(feature = "meta")]
use dfir_lang::diagnostic::{Diagnostic, Diagnostics, SerdeSpan};
#[cfg(feature = "meta")]
use dfir_lang::graph::DfirGraph;

use super::metrics::{DfirMetrics, DfirMetricsIntervals, MetricsState};
use crate::scheduled::ticks::TickInstant;

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
/// scheduled graph (no tokio channels, no scheduler queues, no loop machinery).
///
/// Exposes methods that operator-generated code calls on both
/// `df` (for prologues: `request_task`) and
/// `context` (for iterators: `current_tick`, `schedule_subgraph`, etc.).
///
/// # Metrics typestate
///
/// The `M` type parameter is a [`MetricsState`] typestate which statically records whether this
/// context tracks runtime metrics:
///
/// - `Context<DfirMetrics>` (the default) is created via [`Context::new_with_metrics`] and holds
///   a shared `Arc<DfirMetrics>` handle, accessible via [`Context::metrics`].
/// - `Context<()>` is created via [`Context::new`] and holds no metrics state whatsoever. The
///   `metrics()` accessor does not exist on it, so it is a compile error to try to read metrics
///   from a context which never tracked them.
pub struct Context<M: MetricsState = DfirMetrics> {
    /// Counter for number of ticks run.
    current_tick: TickInstant,
    /// Coordinates waking between [`Context`] (inside the tick closure) and [`Dfir`]
    /// (the external runner). Shared via `Arc` between both. Implements [`Wake`].
    wake_state: Arc<WakeState>,
    /// Live-updating DFIR runtime metrics via atomics (`Arc<DfirMetrics>`), or `()`
    /// when metrics are not tracked.
    metrics: M::Handle,
    /// Tasks buffered via [`Self::request_task`], spawned via [`tokio::spawn`] by
    /// [`Dfir::spawn_tasks`] once the runtime is running inside a tokio runtime.
    ///
    /// Futures are required to be `Send` so that `Context` (and therefore [`Dfir`]) can be moved
    /// between threads before being run, and so that the tasks can be spawned onto any tokio
    /// runtime (no `LocalSet` required).
    #[cfg(feature = "tokio")]
    tasks_to_spawn: Vec<Pin<Box<dyn Future<Output = ()> + Send + 'static>>>,
}

impl Context<()> {
    /// Create a new inline context with a fresh wake state and **no** metrics tracking.
    ///
    /// The returned `Context<()>` statically contains no metrics state. Use
    /// [`Context::new_with_metrics`] to create a `Context<DfirMetrics>` which tracks metrics.
    pub fn new() -> Self {
        Self::with_metrics_handle(())
    }
}

impl Default for Context<()> {
    fn default() -> Self {
        Self::new()
    }
}

impl Context<DfirMetrics> {
    /// Create a new inline context with a fresh wake state and the given (pre-populated) metrics.
    pub fn new_with_metrics(metrics: DfirMetrics) -> Self {
        Self::with_metrics_handle(Arc::new(metrics))
    }

    /// Returns a reference to the runtime metrics.
    pub fn metrics(&self) -> &Arc<DfirMetrics> {
        &self.metrics
    }
}

impl<M: MetricsState> Context<M> {
    fn with_metrics_handle(metrics: M::Handle) -> Self {
        Self {
            current_tick: TickInstant::default(),
            wake_state: Arc::default(),
            metrics,
            #[cfg(feature = "tokio")]
            tasks_to_spawn: Vec::new(),
        }
    }

    // --- Methods called as `df.xxx()` in operator prologues ---

    /// Buffers an async task to be spawned later by `Dfir::spawn_tasks`.
    ///
    /// Tasks are deferred because `write_prologue` runs during graph construction,
    /// which may occur before a tokio runtime is entered. Buffered tasks are
    /// drained and spawned via [`tokio::spawn`] at the start of
    /// [`Dfir::run_tick`]. Tasks requested after that point remain buffered until
    /// the next call to [`Dfir::run_tick`].
    ///
    /// The future must be `Send` so that the `Context` remains `Send` and so the task can be
    /// spawned onto any tokio runtime.
    #[cfg(feature = "tokio")]
    pub fn request_task<Fut>(&mut self, future: Fut)
    where
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.tasks_to_spawn.push(Box::pin(future));
    }

    // --- Methods called as `context.xxx()` in operator iterators ---

    /// Gets the current tick count.
    pub fn current_tick(&self) -> TickInstant {
        self.current_tick
    }

    /// Signals that external data has arrived and a new tick should be started.
    pub fn schedule_subgraph(&self, is_external: bool) {
        if is_external {
            self.wake_state.wake_by_ref();
        }
    }

    /// Returns a waker that signals external data has arrived.
    pub fn waker(&self) -> std::task::Waker {
        std::task::Waker::from(self.wake_state.clone())
    }

    /// Increments the tick counter.
    /// Called by the generated tick closure at the end of each tick.
    #[doc(hidden)]
    pub fn __end_tick(&mut self) {
        self.current_tick += crate::scheduled::ticks::TickDuration::SINGLE_TICK;
    }
}

/// An executable DFIR dataflow, as created by
/// [`dfir_syntax!`](crate::dfir_syntax). Provides the [`Self::run`],
/// [`Self::run_available`], and [`Self::run_tick`] family of methods to
/// execute the dataflow.
///
/// # Design
///
/// The inline codegen generates an `async move |df: &mut Context<M>|` closure that captures
/// dataflow-specific state (handoff buffers, source iterators) and receives the [`Context`]
/// (tick counter, metrics) by reference each tick. `Dfir` owns both the
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
///
/// # Metrics
///
/// Whether this `Dfir` tracks runtime metrics is determined statically by
/// [`TickClosure::Metrics`] (see [`MetricsState`]). The [`Self::metrics`] and
/// [`Self::metrics_intervals`] accessors only exist when `Tick::Metrics = DfirMetrics`. When
/// returning a `Dfir` from a function via `impl TickClosure`, use
/// `Dfir<impl TickClosure<Metrics = DfirMetrics>>` if callers need to access the metrics.
pub struct Dfir<Tick: TickClosure> {
    /// Async closure which runs a single tick when called.
    tick_closure: Tick,
    /// Coordinates waking between [`Context`] (inside the tick closure) and [`Dfir`]
    /// (the external runner). Shared via `Arc` between both. Implements [`Wake`].
    wake_state: Arc<WakeState>,
    /// The inline context, owned by `Dfir` and passed to the tick closure by reference.
    context: Context<Tick::Metrics>,
    /// See [`Self::meta_graph()`].
    ///
    /// Stored as the serialized JSON string rather than a deserialized [`DfirGraph`], since
    /// `DfirGraph` contains `proc_macro2` types which are `!Send`. Deserialized on demand.
    #[cfg(feature = "meta")]
    meta_graph_json: Option<String>,
    /// See [`Self::diagnostics()`].
    #[cfg(feature = "meta")]
    diagnostics: Option<Vec<Diagnostic<SerdeSpan>>>,
}

/// Trait for tick closures — abstracts over both concrete async closures (wrapped in
/// [`TickClosureFn`]) and type-erased boxed versions ([`TickClosureErased`]).
///
/// The `&mut Context` parameter is owned by [`Dfir`] and lent to the
/// closure each tick, avoiding shared-ownership overhead for the context.
///
/// The [`Self::Metrics`] associated type carries the [`MetricsState`] typestate of the context the
/// closure expects, so that [`Dfir<Tick>`] can be parameterized by the closure alone.
pub trait TickClosure {
    /// Whether the context passed to this closure tracks metrics. See [`MetricsState`].
    type Metrics: MetricsState;

    /// Call the tick closure. Returns `true` if any subgraph received input data.
    #[doc(hidden)]
    fn call_tick<'a>(
        &'a mut self,
        ctx: &'a mut Context<Self::Metrics>,
    ) -> impl Future<Output = bool> + 'a;
}

/// Wraps an `AsyncFnMut(&mut Context<M>) -> bool` closure as a [`TickClosure`].
///
/// A blanket `impl<M, F: AsyncFnMut(&mut Context<M>)> TickClosure for F` is not allowed
/// since `M` would only be constrained via the `AsyncFnMut` bound (E0207), so the generated code
/// wraps the tick closure in this newtype which carries `M` explicitly.
#[doc(hidden)]
pub struct TickClosureFn<M, F> {
    func: F,
    _metrics: PhantomData<fn() -> M>,
}

impl<M, F> TickClosureFn<M, F>
where
    M: MetricsState,
    F: for<'a> AsyncFnMut(&'a mut Context<M>) -> bool,
{
    /// Wrap the given async tick closure.
    pub fn new(func: F) -> Self {
        Self {
            func,
            _metrics: PhantomData,
        }
    }
}

impl<M, F> TickClosure for TickClosureFn<M, F>
where
    M: MetricsState,
    F: for<'a> AsyncFnMut(&'a mut Context<M>) -> bool,
{
    type Metrics = M;

    fn call_tick<'a>(&'a mut self, ctx: &'a mut Context<M>) -> impl Future<Output = bool> + 'a {
        (self.func)(ctx)
    }
}

/// No-op `TickClosure`. Used by `dfir_syntax!` as a stand-in when codegen fails, so its
/// `Metrics` matches the macro's default (metrics tracking enabled).
#[doc(hidden)]
pub struct NullTickClosure;

impl TickClosure for NullTickClosure {
    type Metrics = DfirMetrics;

    fn call_tick<'a>(
        &'a mut self,
        _ctx: &'a mut Context<DfirMetrics>,
    ) -> impl Future<Output = bool> + 'a {
        std::future::ready(false)
    }
}

/// Type-erased tick function for use in heterogeneous collections (e.g., the sim runtime).
#[doc(hidden)]
pub struct TickClosureErased<M: MetricsState = DfirMetrics>(Box<dyn TickClosureErasedInner<M>>);

/// Object-safe inner trait for [`TickClosureErased`]. Needed because [`TickClosure::call_tick`]
/// returns `impl Future` (not object-safe), but a trait with `&mut self -> Pin<Box<dyn Future + '_>>`
/// is — the returned future borrows from the trait object which owns the closure.
trait TickClosureErasedInner<M: MetricsState> {
    fn call_tick<'a>(
        &'a mut self,
        ctx: &'a mut Context<M>,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>>;
}

impl<Tick: TickClosure> TickClosureErasedInner<Tick::Metrics> for Tick {
    fn call_tick<'a>(
        &'a mut self,
        ctx: &'a mut Context<Tick::Metrics>,
    ) -> Pin<Box<dyn Future<Output = bool> + 'a>> {
        Box::pin(TickClosure::call_tick(self, ctx))
    }
}

impl<M: MetricsState> TickClosure for TickClosureErased<M> {
    type Metrics = M;

    fn call_tick<'a>(&'a mut self, ctx: &'a mut Context<M>) -> impl Future<Output = bool> + 'a {
        self.0.call_tick(ctx)
    }
}

/// Type alias for a type-erased [`Dfir`] that can be stored in heterogeneous collections.
/// Created via [`Dfir::into_erased`].
///
/// `M` is the [`MetricsState`] of the erased dataflow: `DfirMetrics` (the default) if metrics
/// are tracked, or `()` if not.
pub type DfirErased<M = DfirMetrics> = Dfir<TickClosureErased<M>>;

impl<Tick: TickClosure> Dfir<Tick> {
    /// Create a new `Dfir` from a tick closure, inline context,
    /// and meta graph / diagnostics JSON strings.
    #[doc(hidden)]
    pub fn new(
        tick_closure: Tick,
        context: Context<Tick::Metrics>,
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
            meta_graph_json: meta_graph_json.map(str::to_owned),
            #[cfg(feature = "meta")]
            diagnostics: diagnostics_json.map(|json| {
                serde_json::from_str(json).expect("Failed to deserialize diagnostics.")
            }),
        }
    }

    /// Returns the meta graph, if set.
    ///
    /// The graph is deserialized from its JSON representation on each call, so callers which need
    /// it multiple times should store the result.
    #[cfg(feature = "meta")]
    #[cfg_attr(docsrs, doc(cfg(feature = "meta")))]
    pub fn meta_graph(&self) -> Option<DfirGraph> {
        self.meta_graph_json.as_deref().map(|json| {
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
        })
    }

    /// Returns any diagnostics generated by the surface syntax macro.
    #[cfg(feature = "meta")]
    #[cfg_attr(docsrs, doc(cfg(feature = "meta")))]
    pub fn diagnostics(&self) -> Option<&[Diagnostic<SerdeSpan>]> {
        self.diagnostics.as_deref()
    }

    /// Gets the current tick (local time) count.
    pub fn current_tick(&self) -> TickInstant {
        self.context.current_tick()
    }
}

/// Metrics accessors, only available when metrics tracking is enabled (`Tick::Metrics = DfirMetrics`).
impl<Tick: TickClosure<Metrics = DfirMetrics>> Dfir<Tick> {
    /// Returns a reference-counted handle to the continually-updated runtime metrics for this DFIR instance.
    ///
    /// Only available when metrics tracking is enabled, see [`MetricsState`].
    pub fn metrics(&self) -> Arc<DfirMetrics> {
        Arc::clone(self.context.metrics())
    }

    /// Returns a [`DfirMetricsIntervals`] handle where each call to
    /// [`DfirMetricsIntervals::take_interval`] ends the current interval and returns its metrics.
    ///
    /// The first call to `take_interval` returns metrics since this DFIR instance was created. Each subsequent call to
    /// `take_interval` returns metrics since the previous call.
    ///
    /// Cloning the handle "forks" it from the original, as afterwards each interval may return different metrics
    /// depending on when exactly `take_interval` is called.
    ///
    /// Only available when metrics tracking is enabled, see [`MetricsState`].
    pub fn metrics_intervals(&self) -> DfirMetricsIntervals {
        DfirMetricsIntervals {
            curr: self.metrics(),
            prev: None,
        }
    }
}

impl<Tick: TickClosure> Dfir<Tick> {
    /// Spawns all tasks buffered via [`Context::request_task`] onto the current tokio runtime.
    ///
    /// This drains the buffer, so subsequent calls are no-ops until new tasks are requested.
    ///
    /// # Panics
    ///
    /// Panics if there are buffered tasks and this is called outside of a tokio runtime.
    #[cfg(feature = "tokio")]
    fn spawn_tasks(&mut self) {
        for task in self.context.tasks_to_spawn.drain(..) {
            tokio::spawn(task);
        }
    }

    /// Run a single tick. Returns `true` if any subgraph received input data.
    ///
    /// Checks both handoff buffers (via `work_done` flag set in generated recv port code)
    /// and external events (via `can_start_tick` set by `waker`/`schedule_subgraph`).
    pub async fn run_tick(&mut self) -> bool {
        #[cfg(feature = "tokio")]
        self.spawn_tasks();
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
    #[cfg(feature = "tokio")]
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
    #[cfg(feature = "tokio")]
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

impl<Tick: TickClosure + 'static> Dfir<Tick> {
    /// Type-erase the tick closure for use in heterogeneous collections.
    ///
    /// Wraps the concrete tick closure in [`TickClosureErased`], which boxes the future
    /// returned by each tick call. This adds one heap allocation per tick, but enables
    /// storing multiple `Dfir`s with different closure types in a single `Vec`.
    ///
    /// The [`MetricsState`] typestate is preserved: the result is `DfirErased<Tick::Metrics>`.
    ///
    /// Only needed for the sim runtime path. The trybuild and embedded paths keep the
    /// concrete type and pay no erasure cost.
    pub fn into_erased(self) -> DfirErased<Tick::Metrics> {
        Dfir {
            tick_closure: TickClosureErased(Box::new(self.tick_closure)),
            wake_state: self.wake_state,
            context: self.context,
            #[cfg(feature = "meta")]
            meta_graph_json: self.meta_graph_json,
            #[cfg(feature = "meta")]
            diagnostics: self.diagnostics,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::future::ready;

    use static_assertions::assert_impl_all;

    use super::{Context, Dfir, NullTickClosure, TickClosure};
    use crate::scheduled::metrics::{
        AtomicCount, AtomicDuration, DfirMetrics, DfirMetricsIntervals,
    };

    struct SendTick;

    impl TickClosure for SendTick {
        type Metrics = ();

        fn call_tick<'a>(
            &'a mut self,
            _ctx: &'a mut Context<()>,
        ) -> impl Future<Output = bool> + 'a {
            ready(false)
        }
    }

    // Without metrics tracking.
    assert_impl_all!(Context<()>: Send);
    assert_impl_all!(Dfir<SendTick>: Send);

    // With metrics tracking (`NullTickClosure` has `Metrics = DfirMetrics`).
    assert_impl_all!(AtomicCount: Send, Sync);
    assert_impl_all!(AtomicDuration: Send, Sync);
    assert_impl_all!(DfirMetrics: Send, Sync);
    assert_impl_all!(DfirMetricsIntervals: Send);
    assert_impl_all!(Context<DfirMetrics>: Send);
    assert_impl_all!(Dfir<NullTickClosure>: Send);
}
