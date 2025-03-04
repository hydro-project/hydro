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

use super::state::StateHandle;
use super::{LoopTag, StateId, SubgraphId};
use crate::scheduled::ticks::TickInstant;
use crate::util::priority_stack::PriorityStack;
use crate::util::slot_vec::SlotVec;

/// The main state and scheduler of the runtime instance. Provided as the `context` API to each
/// subgraph/operator as it is run.
///
/// Each instance stores eactly one Context inline. Before the `Context` is provided to
/// a running operator, the `subgraph_id` field must be updated.
pub struct Context {
    /// User-facing State API.
    states: Vec<StateData>,

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

    // Depth of loop (zero for top-level).
    pub(super) loop_depth: SlotVec<LoopTag, usize>,

    pub(super) loop_nonce: usize,

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

        use futures::task::ArcWake;

        struct ContextWaker {
            subgraph_id: SubgraphId,
            event_queue_send: UnboundedSender<(SubgraphId, bool)>,
        }
        impl ArcWake for ContextWaker {
            fn wake_by_ref(arc_self: &Arc<Self>) {
                let _recv_closed_error =
                    arc_self.event_queue_send.send((arc_self.subgraph_id, true));
            }
        }

        let context_waker = ContextWaker {
            subgraph_id: self.subgraph_id,
            event_queue_send: self.event_queue_send.clone(),
        };
        futures::task::waker(Arc::new(context_waker))
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
            .get(handle.state_id.0)
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
            .get(handle.state_id.0)
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
            .get_mut(handle.state_id.0)
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
        let state_id = StateId(self.states.len());

        let state_data = StateData {
            state: Box::new(state),
            tick_reset: None,
        };
        self.states.push(state_data);

        StateHandle {
            state_id,
            _phantom: PhantomData,
        }
    }

    /// Sets a hook to modify the state at the end of each tick, using the supplied closure.
    pub fn set_state_tick_hook<T>(
        &mut self,
        handle: StateHandle<T>,
        mut tick_hook_fn: impl 'static + FnMut(&mut T),
    ) where
        T: Any,
    {
        self.states
            .get_mut(handle.state_id.0)
            .expect("Failed to find state with given handle.")
            .tick_reset = Some(Box::new(move |state| {
            (tick_hook_fn)(state.downcast_mut::<T>().unwrap());
        }));
    }

    /// Removes state from the context returns it as an owned heap value.
    pub fn remove_state<T>(&mut self, handle: StateHandle<T>) -> Box<T>
    where
        T: Any,
    {
        self.states
            .remove(handle.state_id.0)
            .state
            .downcast()
            .expect("StateHandle wrong type T for casting.")
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
            states: Vec::new(),

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
            loop_nonce: 0,

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
    pub(super) fn reset_state_at_end_of_tick(&mut self) {
        for StateData { state, tick_reset } in self.states.iter_mut() {
            if let Some(tick_reset) = tick_reset {
                (tick_reset)(Box::deref_mut(state));
            }
        }
    }
}

/// Internal struct containing a pointer to instance-owned state.
struct StateData {
    state: Box<dyn Any>,
    tick_reset: Option<TickResetFn>,
}
type TickResetFn = Box<dyn FnMut(&mut dyn Any)>;
