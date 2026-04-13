//! Module for the user-facing [`Context`] object.
//!
//! Provides APIs for state and scheduling.

use std::any::Any;
use std::cell::Cell;
use std::collections::VecDeque;
use std::future::Future;
use std::marker::PhantomData;
use std::ops::DerefMut;
use std::pin::Pin;

use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;
use web_time::SystemTime;

use super::graph::StateLifespan;
use super::state::StateHandle;
use super::{LoopId, LoopTag, StateId, StateTag, SubgraphId, SubgraphTag};
use crate::scheduled::ticks::TickInstant;
use crate::util::priority_stack::PriorityStack;
use crate::util::slot_vec::{SecondarySlotVec, SlotVec};

/// The main state and scheduler of the runtime instance. Provided as the `context` API to each
/// subgraph/operator as it is run.
///
/// Each instance stores eactly one Context inline. Before the `Context` is provided to
/// a running operator, the `subgraph_id` field must be updated.
pub struct Context {
    /// Storage for the user-facing State API.
    states: SlotVec<StateTag, StateData>,

    /// Priority stack for handling strata within loops. Prioritized by loop depth.
    pub(super) stratum_stack: PriorityStack<usize>,

    /// Stack of loop nonces. Used to identify when a new loop iteration begins.
    pub(super) loop_nonce_stack: Vec<usize>,

    /// TODO(mingwei):
    /// used for loop iteration scheduling.
    pub(super) schedule_deferred: Vec<SubgraphId>,

    /// TODO(mingwei): separate scheduler into its own struct/trait?
    /// Index is stratum, value is FIFO queue for that stratum.
    pub(super) stratum_queues: Vec<VecDeque<SubgraphId>>,

    /// Receive events, if second arg indicates if it is an external "important" event (true).
    pub(super) event_queue_recv: UnboundedReceiver<(SubgraphId, bool)>,
    /// If external events or data can justify starting the next tick.
    pub(super) can_start_tick: bool,
    /// If the events have been received for this tick.
    pub(super) events_received_tick: bool,

    // TODO(mingwei): as long as this is here, it's impossible to know when all work is done.
    // Second field (bool) is for if the event is an external "important" event (true).
    pub(super) event_queue_send: UnboundedSender<(SubgraphId, bool)>,

    /// If the current subgraph wants to reschedule the current loop block (in the current tick).
    pub(super) reschedule_loop_block: Cell<bool>,
    pub(super) allow_another_iteration: Cell<bool>,

    pub(super) current_tick: TickInstant,
    pub(super) current_stratum: usize,

    pub(super) current_tick_start: SystemTime,
    pub(super) is_first_run_this_tick: bool,
    pub(super) loop_iter_count: usize,

    /// Depth of loop (zero for top-level).
    pub(super) loop_depth: SlotVec<LoopTag, usize>,
    /// For each loop, state which needs to be reset between loop executions.
    loop_states: SecondarySlotVec<LoopTag, Vec<StateId>>,
    /// Used to differentiate between loop executions. Incremented at the start of each loop execution.
    pub(super) loop_nonce: usize,

    /// For each subgraph, state which needs to be reset between executions.
    subgraph_states: SecondarySlotVec<SubgraphTag, Vec<StateId>>,

    /// The SubgraphId of the currently running operator. When this context is
    /// not being forwarded to a running operator, this field is meaningless.
    pub(super) subgraph_id: SubgraphId,

    tasks_to_spawn: Vec<Pin<Box<dyn Future<Output = ()> + 'static>>>,
    /// Join handles for spawned tasks.
    task_join_handles: Vec<JoinHandle<()>>,
}
/// Public APIs.
impl Context {
    /// Gets the current tick (local time) count.
    pub fn current_tick(&self) -> TickInstant {
        self.current_tick
    }

    /// Gets the timestamp of the beginning of the current tick.
    pub fn current_tick_start(&self) -> SystemTime {
        self.current_tick_start
    }

    /// Gets whether this is the first time this subgraph is being scheduled for this tick
    pub fn is_first_run_this_tick(&self) -> bool {
        self.is_first_run_this_tick
    }

    /// Gets the current loop iteration count.
    pub fn loop_iter_count(&self) -> usize {
        self.loop_iter_count
    }

    /// Gets the current stratum nubmer.
    pub fn current_stratum(&self) -> usize {
        self.current_stratum
    }

    /// Gets the ID of the current subgraph.
    pub fn current_subgraph(&self) -> SubgraphId {
        self.subgraph_id
    }

    /// Schedules a subgraph for the next tick.
    ///
    /// If `is_external` is `true`, the scheduling will trigger the next tick to begin. If it is
    /// `false` then scheduling will be lazy and the next tick will not begin unless there is other
    /// reason to.
    pub fn schedule_subgraph(&self, sg_id: SubgraphId, is_external: bool) {
        self.event_queue_send.send((sg_id, is_external)).unwrap()
    }

    /// Schedules the current loop block to be run again (_in this tick_).
    pub fn reschedule_loop_block(&self) {
        self.reschedule_loop_block.set(true);
    }

    /// Allow another iteration of the loop, if more data comes.
    pub fn allow_another_iteration(&self) {
        self.allow_another_iteration.set(true);
    }

    /// Returns a `Waker` for interacting with async Rust.
    /// Waker events are considered to be extenral.
    pub fn waker(&self) -> std::task::Waker {
        use std::sync::Arc;
        use std::task::Wake;

        struct ContextWaker {
            subgraph_id: SubgraphId,
            event_queue_send: UnboundedSender<(SubgraphId, bool)>,
        }
        impl Wake for ContextWaker {
            fn wake(self: Arc<Self>) {
                self.wake_by_ref();
            }

            fn wake_by_ref(self: &Arc<Self>) {
                let _recv_closed_error = self.event_queue_send.send((self.subgraph_id, true));
            }
        }

        let context_waker = ContextWaker {
            subgraph_id: self.subgraph_id,
            event_queue_send: self.event_queue_send.clone(),
        };
        std::task::Waker::from(Arc::new(context_waker))
    }

    /// Returns a shared reference to the state.
    ///
    /// # Safety
    /// `StateHandle<T>` must be from _this_ instance, created via [`Self::add_state`].
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

        unsafe {
            // SAFETY: `handle` is from this instance.
            // TODO(shadaj): replace with `downcast_ref_unchecked` when it's stabilized
            &*(state as *const dyn Any as *const T)
        }
    }

    /// Returns a shared reference to the state.
    pub fn state_ref<T>(&self, handle: StateHandle<T>) -> &'_ T
    where
        T: Any,
    {
        self.states
            .get(handle.state_id)
            .expect("Failed to find state with given handle.")
            .state
            .downcast_ref()
            .expect("StateHandle wrong type T for casting.")
    }

    /// Returns an exclusive reference to the state.
    pub fn state_mut<T>(&mut self, handle: StateHandle<T>) -> &'_ mut T
    where
        T: Any,
    {
        self.states
            .get_mut(handle.state_id)
            .expect("Failed to find state with given handle.")
            .state
            .downcast_mut()
            .expect("StateHandle wrong type T for casting.")
    }

    /// Adds state to the context and returns the handle.
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

    /// Sets a hook to modify the state at the end of each tick, using the supplied closure.
    pub fn set_state_lifespan_hook<T>(
        &mut self,
        handle: StateHandle<T>,
        lifespan: StateLifespan,
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
        state_data.lifespan = Some(lifespan);

        match lifespan {
            StateLifespan::Subgraph(key) => {
                self.subgraph_states
                    .get_or_insert_with(key, Vec::new)
                    .push(handle.state_id);
            }
            StateLifespan::Loop(loop_id) => {
                self.loop_states
                    .get_or_insert_with(loop_id, Vec::new)
                    .push(handle.state_id);
            }
            StateLifespan::Tick => {
                // Already included in `run_state_hooks_tick`.
            }
            StateLifespan::Static => {
                // Never resets.
            }
        }
    }

    /// Prepares an async task to be launched by [`Self::spawn_tasks`].
    pub fn request_task<Fut>(&mut self, future: Fut)
    where
        Fut: Future<Output = ()> + 'static,
    {
        self.tasks_to_spawn.push(Box::pin(future));
    }

    /// Launches all tasks requested with [`Self::request_task`] on the internal Tokio executor.
    pub fn spawn_tasks(&mut self) {
        for task in self.tasks_to_spawn.drain(..) {
            self.task_join_handles.push(tokio::task::spawn_local(task));
        }
    }

    /// Aborts all tasks spawned with [`Self::spawn_tasks`].
    pub fn abort_tasks(&mut self) {
        for task in self.task_join_handles.drain(..) {
            task.abort();
        }
    }

    /// Waits for all tasks spawned with [`Self::spawn_tasks`] to complete.
    ///
    /// Will probably just hang.
    pub async fn join_tasks(&mut self) {
        futures::future::join_all(self.task_join_handles.drain(..)).await;
    }
}

impl Default for Context {
    fn default() -> Self {
        let stratum_queues = vec![Default::default()]; // Always initialize stratum #0.
        let (event_queue_send, event_queue_recv) = mpsc::unbounded_channel();
        let (stratum_stack, loop_depth) = Default::default();
        Self {
            states: SlotVec::new(),

            stratum_stack,

            loop_nonce_stack: Vec::new(),

            schedule_deferred: Vec::new(),

            stratum_queues,
            event_queue_recv,
            can_start_tick: false,
            events_received_tick: false,

            event_queue_send,
            reschedule_loop_block: Cell::new(false),
            allow_another_iteration: Cell::new(false),

            current_stratum: 0,
            current_tick: TickInstant::default(),

            current_tick_start: SystemTime::now(),
            is_first_run_this_tick: false,
            loop_iter_count: 0,

            loop_depth,
            loop_states: SecondarySlotVec::new(),
            loop_nonce: 0,

            subgraph_states: SecondarySlotVec::new(),

            // Will be re-set before use.
            subgraph_id: SubgraphId::from_raw(0),

            tasks_to_spawn: Vec::new(),
            task_join_handles: Vec::new(),
        }
    }
}
/// Internal APIs.
impl Context {
    /// Makes sure stratum STRATUM is initialized.
    pub(super) fn init_stratum(&mut self, stratum: usize) {
        if self.stratum_queues.len() <= stratum {
            self.stratum_queues
                .resize_with(stratum + 1, Default::default);
        }
    }

    /// Call this at the end of a tick,
    pub(super) fn run_state_hooks_tick(&mut self) {
        tracing::trace!("Running state hooks for tick.");
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
    }

    pub(super) fn run_state_hooks_subgraph(&mut self, subgraph_id: SubgraphId) {
        tracing::trace!("Running state hooks for subgraph.");
        for state_id in self.subgraph_states.get(subgraph_id).into_iter().flatten() {
            let StateData {
                state,
                lifespan_hook_fn,
                lifespan: _,
            } = self
                .states
                .get_mut(*state_id)
                .expect("Failed to find state with given ID.");

            if let Some(lifespan_hook_fn) = lifespan_hook_fn {
                (lifespan_hook_fn)(Box::deref_mut(state));
            }
        }
    }

    // Run the state hooks for each state in the loop.
    // Call at the end of each loop execution.
    pub(super) fn run_state_hooks_loop(&mut self, loop_id: LoopId) {
        tracing::trace!(
            loop_id = loop_id.to_string(),
            "Running state hooks for loop."
        );
        for state_id in self.loop_states.get(loop_id).into_iter().flatten() {
            let StateData {
                state,
                lifespan_hook_fn,
                lifespan: _,
            } = self
                .states
                .get_mut(*state_id)
                .expect("Failed to find state with given ID.");

            if let Some(lifespan_hook_fn) = lifespan_hook_fn {
                (lifespan_hook_fn)(Box::deref_mut(state));
            }
        }
    }
}

/// Internal struct containing a pointer to instance-owned state.
struct StateData {
    state: Box<dyn Any>,
    lifespan_hook_fn: Option<LifespanResetFn>, // TODO(mingwei): replace with trait?
    /// `None` for static.
    lifespan: Option<StateLifespan>,
}
type LifespanResetFn = Box<dyn FnMut(&mut dyn Any)>;

/// Shared state between [`InlineContext`] and [`InlineFlow`].
#[doc(hidden)]
pub struct InlineFlowState {
    /// Whether the next tick should run.
    can_start_tick: std::sync::atomic::AtomicBool,
    /// Whether any work was done during the current tick.
    work_done: std::sync::atomic::AtomicBool,
    /// Waker to wake the [`InlineFlow::run`] task when external events arrive.
    task_waker: futures::task::AtomicWaker,
}

impl Default for InlineFlowState {
    fn default() -> Self {
        Self {
            can_start_tick: std::sync::atomic::AtomicBool::new(false),
            // Pre-set to true so the first run_tick always returns true, matching
            // Dfir behavior where all subgraphs are pre-scheduled on creation.
            work_done: std::sync::atomic::AtomicBool::new(true),
            task_waker: futures::task::AtomicWaker::new(),
        }
    }
}

impl std::task::Wake for InlineFlowState {
    fn wake(self: std::sync::Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &std::sync::Arc<Self>) {
        self.can_start_tick
            .store(true, std::sync::atomic::Ordering::Relaxed);
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
pub struct InlineContext {
    states: SlotVec<StateTag, StateData>,
    current_tick: TickInstant,
    is_first_run_this_tick: bool,
    flow_state: std::sync::Arc<InlineFlowState>,
}

impl InlineContext {
    /// Create a new inline context with shared flow state.
    pub fn new(flow_state: std::sync::Arc<InlineFlowState>) -> Self {
        Self {
            states: SlotVec::new(),
            current_tick: TickInstant::default(),
            is_first_run_this_tick: true,
            flow_state,
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

    /// Gets whether this is the first time running this tick.
    pub fn is_first_run_this_tick(&self) -> bool {
        self.is_first_run_this_tick
    }

    /// Gets the current tick count.
    pub fn current_tick(&self) -> TickInstant {
        self.current_tick
    }

    /// No-op: inline mode has no subgraph scheduling.
    pub fn current_subgraph(&self) -> SubgraphId {
        SubgraphId::from_raw(0)
    }

    /// Schedules a subgraph. In inline mode, only external events trigger a new tick.
    pub fn schedule_subgraph(&self, _sg_id: SubgraphId, is_external: bool) {
        if is_external {
            self.flow_state
                .can_start_tick
                .store(true, std::sync::atomic::Ordering::Relaxed);
            self.flow_state.task_waker.wake();
        }
    }

    /// Returns a waker that signals external data has arrived.
    pub fn waker(&self) -> std::task::Waker {
        std::task::Waker::from(self.flow_state.clone())
    }

    /// No-op for inline mode.
    pub fn request_task<Fut>(&mut self, _future: Fut)
    where
        Fut: Future<Output = ()> + 'static,
    {
    }

    // --- Methods called by __end_tick / __context ---

    /// Returns `&Self` (for binding `context` in generated code).
    pub fn __context(&self) -> &Self {
        self
    }

    /// Marks that work was done this tick (a handoff buffer had data).
    pub fn __mark_work_done(&self) {
        self.flow_state
            .work_done
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }

    /// Runs end-of-tick state hooks and increments the tick counter.
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
        self.is_first_run_this_tick = true;
    }
}

/// A wrapper around an inline-codegen tick closure that provides [`Self::run`],
/// [`Self::run_available`], and [`Self::run_tick`] methods — mirroring the [`super::graph::Dfir`]
/// API.
///
/// # Design
///
/// The inline codegen generates an `async move ||` closure that captures all dataflow state
/// (operator accumulators, handoff buffers, source iterators) and runs one tick per call.
/// `InlineFlow` wraps this closure and adds tick lifecycle and idle/wake coordination.
///
/// We use a single opaque closure rather than generating a bespoke struct per dataflow because:
/// - The closure naturally captures exactly the state it needs with correct lifetimes
/// - No codegen needed for struct definitions, field accessors, or initialization
/// - Rust's async closure machinery handles the complex state machine (suspend/resume across
///   `.await` points) that would be very difficult to replicate in a generated struct
///
/// The `Tick` type parameter is bounded by [`TickClosure`] (not `AsyncFnMut` directly) to
/// support type erasure via [`ErasedTickFn`] / [`ErasedInlineFlow`] for heterogeneous
/// collections (e.g., the sim runtime storing multiple locations in a `Vec`). The concrete
/// (non-erased) path used by trybuild and embedded has zero overhead.
#[doc(hidden)]
pub struct InlineFlow<Tick> {
    tick: Tick,
    flow_state: std::sync::Arc<InlineFlowState>,
}

/// Trait for tick closures — abstracts over both concrete async closures
/// and type-erased boxed versions ([`ErasedTickFn`]).
///
/// This trait exists because `AsyncFnMut()` is not object-safe (due to its GAT return type),
/// so we cannot use `dyn AsyncFnMut()`. Instead, concrete async closures implement `TickClosure`
/// via a blanket impl, and [`ErasedTickFn`] implements it by delegating to an object-safe inner
/// trait.
#[doc(hidden)]
pub trait TickClosure {
    /// Call the tick closure, returning a future that completes when the tick is done.
    fn call_tick(&mut self) -> impl std::future::Future<Output = ()>;
}

impl<F: AsyncFnMut()> TickClosure for F {
    fn call_tick(&mut self) -> impl std::future::Future<Output = ()> {
        self()
    }
}

/// Type-erased tick function for use in heterogeneous collections (e.g., the sim runtime).
///
/// Wraps a `Box<dyn ErasedTickFnInner>` — an object-safe trait whose `call_tick` method
/// returns `Pin<Box<dyn Future + '_>>`, borrowing from the trait object's own state.
/// This avoids the `FnMut` borrow-escaping problem (see [`InlineFlow`] docs).
#[doc(hidden)]
pub struct ErasedTickFn(Box<dyn ErasedTickFnInner>);

/// Object-safe inner trait for type-erased tick functions.
/// The key insight: `call_tick(&mut self) -> Pin<Box<dyn Future + '_>>` borrows from
/// `&mut self` (the trait object), which is allowed because the trait object owns the
/// async closure. This sidesteps the `FnMut` borrow-escaping limitation.
trait ErasedTickFnInner {
    fn call_tick(&mut self) -> Pin<Box<dyn Future<Output = ()> + '_>>;
}

impl<F: AsyncFnMut()> ErasedTickFnInner for F {
    fn call_tick(&mut self) -> Pin<Box<dyn Future<Output = ()> + '_>> {
        Box::pin(self())
    }
}

impl TickClosure for ErasedTickFn {
    fn call_tick(&mut self) -> impl std::future::Future<Output = ()> {
        self.0.call_tick()
    }
}

/// Type alias for a type-erased [`InlineFlow`] that can be stored in heterogeneous collections.
#[doc(hidden)]
pub type ErasedInlineFlow = InlineFlow<ErasedTickFn>;

impl<Tick: TickClosure> InlineFlow<Tick> {
    /// Create a new `InlineFlow` from a tick closure and shared flow state.
    pub fn new(tick: Tick, flow_state: std::sync::Arc<InlineFlowState>) -> Self {
        Self { tick, flow_state }
    }

    /// Run a single tick. Returns `true` if any subgraph received input data.
    ///
    /// Checks both handoff buffers (via `work_done` flag set in generated recv port code)
    /// and external events (via `can_start_tick` set by wakers/schedule_subgraph),
    /// Run a single tick. Returns `true` if any subgraph received input data.
    ///
    /// Checks handoff buffers (via `work_done` flag, pre-set to true on creation so the first
    /// tick always returns true) and external events (via `can_start_tick`), both before and
    /// after the tick runs.
    pub async fn run_tick(&mut self) -> bool {
        use std::sync::atomic::Ordering::Relaxed;
        let had_external = self.flow_state.can_start_tick.swap(false, Relaxed);
        let had_work = self.flow_state.work_done.swap(false, Relaxed);
        self.tick.call_tick().await;
        had_external
            || had_work
            || self.flow_state.work_done.load(Relaxed)
            || self.flow_state.can_start_tick.load(Relaxed)
    }

    /// Run a single tick synchronously. Panics if the tick yields (async suspension).
    /// Returns `true` if work was done (see [`Self::run_tick`]).
    pub fn run_tick_sync(&mut self) -> bool {
        let mut fut = std::pin::pin!(self.run_tick());
        let mut ctx = std::task::Context::from_waker(std::task::Waker::noop());
        match fut.as_mut().poll(&mut ctx) {
            std::task::Poll::Ready(result) => result,
            std::task::Poll::Pending => {
                panic!("InlineFlow::run_tick_sync: tick yielded asynchronously.")
            }
        }
    }

    /// Run ticks as long as work is available, then return.
    pub async fn run_available(&mut self) {
        use std::sync::atomic::Ordering::Relaxed;
        // Always run at least one tick.
        self.flow_state.can_start_tick.store(false, Relaxed);
        self.run_tick().await;

        // Keep running while there's more work.
        while self.flow_state.can_start_tick.swap(false, Relaxed) {
            self.run_tick().await;
        }
    }

    /// Run forever, processing ticks when work is available and yielding when idle.
    pub async fn run(&mut self) -> crate::Never {
        use std::sync::atomic::Ordering::Relaxed;
        loop {
            self.run_available().await;
            // Wait for an external event to wake us.
            std::future::poll_fn(|cx| {
                if self.flow_state.can_start_tick.load(Relaxed) {
                    std::task::Poll::Ready(())
                } else {
                    self.flow_state.task_waker.register(cx.waker());
                    std::task::Poll::Pending
                }
            })
            .await;
        }
    }
}

impl<Tick: AsyncFnMut() + 'static> InlineFlow<Tick> {
    /// Type-erase the tick closure for use in heterogeneous collections.
    ///
    /// Wraps the concrete async closure in [`ErasedTickFn`], which boxes the future
    /// returned by each tick call. This adds one heap allocation per tick, but enables
    /// storing multiple `InlineFlow`s with different closure types in a single `Vec`.
    ///
    /// Only needed for the sim runtime path. The trybuild and embedded paths keep the
    /// concrete type and pay no erasure cost.
    pub fn into_erased(self) -> ErasedInlineFlow {
        InlineFlow {
            tick: ErasedTickFn(Box::new(self.tick)),
            flow_state: self.flow_state,
        }
    }
}
