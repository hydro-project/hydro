//! Module for the [`Dfir`] struct and helper items.

use std::any::Any;
use std::borrow::Cow;
use std::cell::Cell;
use std::cmp::Ordering;
use std::future::Future;
use std::marker::PhantomData;
use std::rc::Rc;

#[cfg(feature = "meta")]
use dfir_lang::diagnostic::{Diagnostic, SerdeSpan};
#[cfg(feature = "meta")]
use dfir_lang::graph::DfirGraph;
use ref_cast::RefCast;
use smallvec::SmallVec;
use tracing::Instrument;
use web_time::SystemTime;

use super::context::Context;
use super::handoff::handoff_list::PortList;
use super::handoff::{Handoff, HandoffMeta, TeeingHandoff};
use super::metrics::{DfirMetrics, DfirMetricsIntervals, InstrumentSubgraph};
use super::port::{RECV, RecvCtx, RecvPort, SEND, SendCtx, SendPort};
use super::reactor::Reactor;
use super::state::StateHandle;
use super::subgraph::Subgraph;
use super::ticks::{TickDuration, TickInstant};
use super::{HandoffId, HandoffTag, LoopId, LoopTag, SubgraphId, SubgraphTag};
use crate::Never;
use crate::util::slot_vec::{SecondarySlotVec, SlotVec};

/// A DFIR graph. Owns, schedules, and runs the compiled subgraphs.
#[derive(Default)]
pub struct Dfir<'a> {
    pub(super) subgraphs: SlotVec<SubgraphTag, SubgraphData<'a>>,

    pub(super) loop_data: SecondarySlotVec<LoopTag, LoopData>,

    pub(super) context: Context,

    pub(super) handoffs: SlotVec<HandoffTag, HandoffData>,

    /// Live-updating DFIR runtime metrics via interior mutability.
    metrics: Rc<DfirMetrics>,

    #[cfg(feature = "meta")]
    /// See [`Self::meta_graph()`].
    meta_graph: Option<DfirGraph>,

    #[cfg(feature = "meta")]
    /// See [`Self::diagnostics()`].
    diagnostics: Option<Vec<Diagnostic<SerdeSpan>>>,
}

/// Methods for [`TeeingHandoff`] teeing and dropping.
impl Dfir<'_> {
    /// Tees a [`TeeingHandoff`].
    pub fn teeing_handoff_tee<T>(
        &mut self,
        tee_parent_port: &RecvPort<TeeingHandoff<T>>,
    ) -> RecvPort<TeeingHandoff<T>>
    where
        T: Clone,
    {
        // If we're teeing from a child make sure to find root.
        let tee_root = self.handoffs[tee_parent_port.handoff_id].pred_handoffs[0];

        // Set up teeing metadata.
        let tee_root_data = &mut self.handoffs[tee_root];
        let tee_root_data_name = tee_root_data.name.clone();

        // Insert new handoff output.
        let teeing_handoff =
            <dyn Any>::downcast_ref::<TeeingHandoff<T>>(&*tee_root_data.handoff).unwrap();
        let new_handoff = teeing_handoff.tee();

        // Handoff ID of new tee output.
        let new_hoff_id = self.handoffs.insert_with_key(|new_hoff_id| {
            let new_name = Cow::Owned(format!("{} tee {:?}", tee_root_data_name, new_hoff_id));
            let mut new_handoff_data = HandoffData::new(new_name, new_handoff, new_hoff_id);
            // Set self's predecessor as `tee_root`.
            new_handoff_data.pred_handoffs = vec![tee_root];
            new_handoff_data
        });

        // Go to `tee_root`'s successors and insert self (the new tee output).
        let tee_root_data = &mut self.handoffs[tee_root];
        tee_root_data.succ_handoffs.push(new_hoff_id);

        // Add our new handoff id into the subgraph data if the send `tee_root` has already been
        // used to add a subgraph.
        assert!(
            tee_root_data.preds.len() <= 1,
            "Tee send side should only have one sender (or none set yet)."
        );
        if let Some(&pred_sg_id) = tee_root_data.preds.first() {
            self.subgraphs[pred_sg_id].succs.push(new_hoff_id);
        }

        // Initialize handoff metrics struct.
        Rc::make_mut(&mut self.metrics)
            .handoffs
            .insert(new_hoff_id, Default::default());

        let output_port = RecvPort {
            handoff_id: new_hoff_id,
            _marker: PhantomData,
        };
        output_port
    }

    /// Marks an output of a [`TeeingHandoff`] as dropped so that no more data will be sent to it.
    ///
    /// It is recommended to not not use this method and instead simply avoid teeing a
    /// [`TeeingHandoff`] when it is not needed.
    pub fn teeing_handoff_drop<T>(&mut self, tee_port: RecvPort<TeeingHandoff<T>>)
    where
        T: Clone,
    {
        let data = &self.handoffs[tee_port.handoff_id];
        let teeing_handoff = <dyn Any>::downcast_ref::<TeeingHandoff<T>>(&*data.handoff).unwrap();
        teeing_handoff.drop();

        let tee_root = data.pred_handoffs[0];
        let tee_root_data = &mut self.handoffs[tee_root];
        // Remove this output from the send succ handoff list.
        tee_root_data
            .succ_handoffs
            .retain(|&succ_hoff| succ_hoff != tee_port.handoff_id);
        // Remove from subgraph successors if send port was already connected.
        assert!(
            tee_root_data.preds.len() <= 1,
            "Tee send side should only have one sender (or none set yet)."
        );
        if let Some(&pred_sg_id) = tee_root_data.preds.first() {
            self.subgraphs[pred_sg_id]
                .succs
                .retain(|&succ_hoff| succ_hoff != tee_port.handoff_id);
        }
    }
}

impl<'a> Dfir<'a> {
    /// Create a new empty graph.
    pub fn new() -> Self {
        Default::default()
    }

    /// Assign the meta graph via JSON string. Used internally by the [`crate::dfir_syntax`] and other macros.
    #[doc(hidden)]
    pub fn __assign_meta_graph(&mut self, _meta_graph_json: &str) {
        #[cfg(feature = "meta")]
        {
            let mut meta_graph: DfirGraph =
                serde_json::from_str(_meta_graph_json).expect("Failed to deserialize graph.");

            let mut op_inst_diagnostics = Vec::new();
            meta_graph.insert_node_op_insts_all(&mut op_inst_diagnostics);
            assert!(
                op_inst_diagnostics.is_empty(),
                "Expected no diagnostics, got: {:#?}",
                op_inst_diagnostics
            );

            assert!(self.meta_graph.replace(meta_graph).is_none());
        }
    }
    /// Assign the diagnostics via JSON string.
    #[doc(hidden)]
    pub fn __assign_diagnostics(&mut self, _diagnostics_json: &'static str) {
        #[cfg(feature = "meta")]
        {
            let diagnostics: Vec<Diagnostic<SerdeSpan>> = serde_json::from_str(_diagnostics_json)
                .expect("Failed to deserialize diagnostics.");

            assert!(self.diagnostics.replace(diagnostics).is_none());
        }
    }

    /// Return a handle to the meta graph, if set. The meta graph is a
    /// representation of all the operators, subgraphs, and handoffs in this instance.
    /// Will only be set if this graph was constructed using a surface syntax macro.
    #[cfg(feature = "meta")]
    #[cfg_attr(docsrs, doc(cfg(feature = "meta")))]
    pub fn meta_graph(&self) -> Option<&DfirGraph> {
        self.meta_graph.as_ref()
    }

    /// Returns any diagnostics generated by the surface syntax macro. Each diagnostic is a pair of
    /// (1) a `Diagnostic` with span info reset and (2) the `ToString` version of the diagnostic
    /// with original span info.
    /// Will only be set if this graph was constructed using a surface syntax macro.
    #[cfg(feature = "meta")]
    #[cfg_attr(docsrs, doc(cfg(feature = "meta")))]
    pub fn diagnostics(&self) -> Option<&[Diagnostic<SerdeSpan>]> {
        self.diagnostics.as_deref()
    }

    /// Returns a reactor for externally scheduling subgraphs, possibly from another thread.
    /// Reactor events are considered to be external events.
    pub fn reactor(&self) -> Reactor {
        Reactor::new(self.context.event_queue_send.clone())
    }

    /// Gets the current tick (local time) count.
    pub fn current_tick(&self) -> TickInstant {
        self.context.current_tick
    }

    /// Gets the current stratum nubmer.
    pub fn current_stratum(&self) -> usize {
        self.context.current_stratum
    }

    /// Runs the dataflow until the next tick begins.
    ///
    /// Returns `true` if any work was done.
    ///
    /// Will receive events if it is the start of a tick when called.
    #[tracing::instrument(level = "trace", skip(self), ret)]
    pub async fn run_tick(&mut self) -> bool {
        let mut work_done = false;
        // While work is immediately available *on the current tick*.
        while self.next_stratum(true) {
            work_done = true;
            // Do any work.
            self.run_stratum().await;
        }
        work_done
    }

    /// [`Self::run_tick`] but panics if a subgraph yields asynchronously.
    #[tracing::instrument(level = "trace", skip(self), ret)]
    pub fn run_tick_sync(&mut self) -> bool {
        let mut work_done = false;
        // While work is immediately available *on the current tick*.
        while self.next_stratum(true) {
            work_done = true;
            // Do any work.
            run_sync(self.run_stratum());
        }
        work_done
    }

    /// Runs the dataflow until no more (externally-triggered) work is immediately available.
    /// Runs at least one tick of dataflow, even if no external events have been received.
    /// If the dataflow contains loops this method may run forever.
    ///
    /// Returns `true` if any work was done.
    ///
    /// Yields repeatedly to allow external events to happen.
    #[tracing::instrument(level = "trace", skip(self), ret)]
    pub async fn run_available(&mut self) -> bool {
        let mut work_done = false;
        // While work is immediately available.
        while self.next_stratum(false) {
            work_done = true;
            // Do any work.
            self.run_stratum().await;

            // Yield between each stratum to receive more events.
            // TODO(mingwei): really only need to yield at start of ticks though.
            tokio::task::yield_now().await;
        }
        work_done
    }

    /// [`Self::run_available`] but panics if a subgraph yields asynchronously.
    #[tracing::instrument(level = "trace", skip(self), ret)]
    pub fn run_available_sync(&mut self) -> bool {
        let mut work_done = false;
        // While work is immediately available.
        while self.next_stratum(false) {
            work_done = true;
            // Do any work.
            run_sync(self.run_stratum());
        }
        work_done
    }

    /// Runs the current stratum of the dataflow until no more local work is available (does not receive events).
    ///
    /// Returns `true` if any work was done.
    #[tracing::instrument(level = "trace", skip(self), fields(tick = u64::from(self.context.current_tick), stratum = self.context.current_stratum), ret)]
    pub async fn run_stratum(&mut self) -> bool {
        // Make sure to spawn tasks once dfir is running!
        // This drains the task buffer, so becomes a no-op after first call.
        self.context.spawn_tasks();

        let mut work_done = false;

        'pop: while let Some(sg_id) =
            self.context.stratum_queues[self.context.current_stratum].pop_front()
        {
            let sg_data = &mut self.subgraphs[sg_id];

            // Update handoff metrics.
            // NOTE(mingwei):
            // We measure handoff metrics at `recv`, not `send`.
            // This is done because we (a) know that all `recv`
            // will be consumed(*) by the subgraph when it is run, and in contrast (b) do not know
            // that all `send` will be consumed after a run. I.e. this subgraph may run multiple
            // times before the next subgraph consumes the output; don't double count those.
            // (*) - usually... always true for Hydro-generated DFIR at least.
            for &handoff_id in sg_data.preds.iter() {
                let handoff_metrics = &self.metrics.handoffs[handoff_id];
                let handoff_data = &mut self.handoffs[handoff_id];
                let handoff_len = handoff_data.handoff.len();
                handoff_metrics
                    .total_items_count
                    .update(|x| x + handoff_len);
                handoff_metrics.curr_items_count.set(handoff_len);
            }

            // Run the subgraph (and do bookkeeping).
            {
                // This must be true for the subgraph to be enqueued.
                assert!(sg_data.is_scheduled.take());

                let run_subgraph_span_guard = tracing::info_span!(
                    "run-subgraph",
                    sg_id = sg_id.to_string(),
                    sg_name = &*sg_data.name,
                    sg_depth = sg_data.loop_depth,
                    sg_loop_nonce = sg_data.last_loop_nonce.0,
                    sg_iter_count = sg_data.last_loop_nonce.1,
                )
                .entered();

                match sg_data.loop_depth.cmp(&self.context.loop_nonce_stack.len()) {
                    Ordering::Greater => {
                        // We have entered a loop.
                        self.context.loop_nonce += 1;
                        self.context.loop_nonce_stack.push(self.context.loop_nonce);
                        tracing::trace!(loop_nonce = self.context.loop_nonce, "Entered loop.");
                    }
                    Ordering::Less => {
                        // We have exited a loop.
                        self.context.loop_nonce_stack.pop();
                        tracing::trace!("Exited loop.");
                    }
                    Ordering::Equal => {}
                }

                self.context.subgraph_id = sg_id;
                self.context.is_first_run_this_tick = sg_data
                    .last_tick_run_in
                    .is_none_or(|last_tick| last_tick < self.context.current_tick);

                if let Some(loop_id) = sg_data.loop_id {
                    // Loop execution - running loop block, from start to finish, containing
                    // multiple iterations.
                    // Loop iteration - a single iteration of a loop block, all subgraphs within
                    // the loop should run (at most) once.

                    // If the previous run of this subgraph had the same loop execution and
                    // iteration count, then we need to increment the iteration count.
                    let curr_loop_nonce = self.context.loop_nonce_stack.last().copied();

                    let LoopData {
                        iter_count: loop_iter_count,
                        allow_another_iteration,
                    } = &mut self.loop_data[loop_id];

                    let (prev_loop_nonce, prev_iter_count) = sg_data.last_loop_nonce;

                    // If the loop nonce is the same as the previous execution, then we are in
                    // the same loop execution.
                    // `curr_loop_nonce` is `None` for top-level loops, and top-level loops are
                    // always in the same (singular) loop execution.
                    let (curr_iter_count, new_loop_execution) =
                        if curr_loop_nonce.is_none_or(|nonce| nonce == prev_loop_nonce) {
                            // If the iteration count is the same as the previous execution, then
                            // we are on the next iteration.
                            if *loop_iter_count == prev_iter_count {
                                // If not true, then we shall not run the next iteration.
                                if !std::mem::take(allow_another_iteration) {
                                    tracing::debug!(
                                        "Loop will not continue to next iteration, skipping."
                                    );
                                    continue 'pop;
                                }
                                // Increment `loop_iter_count` or set it to 0.
                                loop_iter_count.map_or((0, true), |n| (n + 1, false))
                            } else {
                                // Otherwise update the local iteration count to match the loop.
                                debug_assert!(
                                    prev_iter_count < *loop_iter_count,
                                    "Expect loop iteration count to be increasing."
                                );
                                (loop_iter_count.unwrap(), false)
                            }
                        } else {
                            // The loop execution has already begun, but this is the first time this particular subgraph is running.
                            (0, false)
                        };

                    if new_loop_execution {
                        // Run state hooks.
                        self.context.run_state_hooks_loop(loop_id);
                    }
                    tracing::debug!("Loop iteration count {}", curr_iter_count);

                    *loop_iter_count = Some(curr_iter_count);
                    self.context.loop_iter_count = curr_iter_count;
                    sg_data.last_loop_nonce =
                        (curr_loop_nonce.unwrap_or_default(), Some(curr_iter_count));
                }

                // Run subgraph state hooks.
                self.context.run_state_hooks_subgraph(sg_id);

                tracing::info!("Running subgraph.");
                sg_data.last_tick_run_in = Some(self.context.current_tick);

                let sg_metrics = &self.metrics.subgraphs[sg_id];
                let sg_fut =
                    Box::into_pin(sg_data.subgraph.run(&mut self.context, &mut self.handoffs));
                // Update subgraph metrics.
                let sg_fut = InstrumentSubgraph::new(sg_fut, sg_metrics);
                // Pass along `run_subgraph_span` to the run subgraph future.
                let sg_fut = sg_fut.instrument(run_subgraph_span_guard.exit());
                let () = sg_fut.await;

                sg_metrics.total_run_count.update(|x| x + 1);
            };

            // Schedule the following subgraphs if data was pushed to the corresponding handoff.
            let sg_data = &self.subgraphs[sg_id];
            for &handoff_id in sg_data.succs.iter() {
                let handoff_data = &self.handoffs[handoff_id];
                let handoff_len = handoff_data.handoff.len();
                if 0 < handoff_len {
                    for &succ_id in handoff_data.succs.iter() {
                        let succ_sg_data = &self.subgraphs[succ_id];
                        // If we have sent data to the next tick, then we can start the next tick.
                        if succ_sg_data.stratum < self.context.current_stratum && !sg_data.is_lazy {
                            self.context.can_start_tick = true;
                        }
                        // Add subgraph to stratum queue if it is not already scheduled.
                        if !succ_sg_data.is_scheduled.replace(true) {
                            self.context.stratum_queues[succ_sg_data.stratum].push_back(succ_id);
                        }
                        // Add stratum to stratum stack if it is within a loop.
                        if 0 < succ_sg_data.loop_depth {
                            // TODO(mingwei): handle duplicates
                            self.context
                                .stratum_stack
                                .push(succ_sg_data.loop_depth, succ_sg_data.stratum);
                        }
                    }
                }
                let handoff_metrics = &self.metrics.handoffs[handoff_id];
                handoff_metrics.curr_items_count.set(handoff_len);
            }

            let reschedule = self.context.reschedule_loop_block.take();
            let allow_another = self.context.allow_another_iteration.take();

            if reschedule {
                // Re-enqueue the subgraph.
                self.context.schedule_deferred.push(sg_id);
                self.context
                    .stratum_stack
                    .push(sg_data.loop_depth, sg_data.stratum);
            }
            if (reschedule || allow_another)
                && let Some(loop_id) = sg_data.loop_id
            {
                self.loop_data
                    .get_mut(loop_id)
                    .unwrap()
                    .allow_another_iteration = true;
            }

            work_done = true;
        }
        work_done
    }

    /// Go to the next stratum which has work available, possibly the current stratum.
    /// Return true if more work is available, otherwise false if no work is immediately
    /// available on any strata.
    ///
    /// This will receive external events when at the start of a tick.
    ///
    /// If `current_tick_only` is set to `true`, will only return `true` if work is immediately
    /// available on the *current tick*.
    ///
    /// If this returns false then the graph will be at the start of a tick (at stratum 0, can
    /// receive more external events).
    #[tracing::instrument(level = "trace", skip(self), ret)]
    pub fn next_stratum(&mut self, current_tick_only: bool) -> bool {
        tracing::trace!(
            events_received_tick = self.context.events_received_tick,
            can_start_tick = self.context.can_start_tick,
            "Starting `next_stratum` call.",
        );

        // The stratum we will stop searching at, i.e. made a full loop around.
        let mut end_stratum = self.context.current_stratum;
        let mut new_tick_started = false;

        if 0 == self.context.current_stratum {
            new_tick_started = true;

            // Starting the tick, reset this to `false`.
            tracing::trace!("Starting tick, setting `can_start_tick = false`.");
            self.context.can_start_tick = false;
            self.context.current_tick_start = SystemTime::now();

            // Ensure external events are received before running the tick.
            if !self.context.events_received_tick {
                // Add any external jobs to ready queue.
                self.try_recv_events();
            }
        }

        loop {
            tracing::trace!(
                tick = u64::from(self.context.current_tick),
                stratum = self.context.current_stratum,
                "Looking for work on stratum."
            );
            // If current stratum has work, return true.
            if !self.context.stratum_queues[self.context.current_stratum].is_empty() {
                tracing::trace!(
                    tick = u64::from(self.context.current_tick),
                    stratum = self.context.current_stratum,
                    "Work found on stratum."
                );
                return true;
            }

            if let Some(next_stratum) = self.context.stratum_stack.pop() {
                self.context.current_stratum = next_stratum;

                // Now schedule deferred subgraphs.
                {
                    for sg_id in self.context.schedule_deferred.drain(..) {
                        let sg_data = &self.subgraphs[sg_id];
                        tracing::info!(
                            tick = u64::from(self.context.current_tick),
                            stratum = self.context.current_stratum,
                            sg_id = sg_id.to_string(),
                            sg_name = &*sg_data.name,
                            is_scheduled = sg_data.is_scheduled.get(),
                            "Rescheduling deferred subgraph."
                        );
                        if !sg_data.is_scheduled.replace(true) {
                            self.context.stratum_queues[sg_data.stratum].push_back(sg_id);
                        }
                    }
                }
            } else {
                // Increment stratum counter.
                self.context.current_stratum += 1;

                if self.context.current_stratum >= self.context.stratum_queues.len() {
                    new_tick_started = true;

                    tracing::trace!(
                        can_start_tick = self.context.can_start_tick,
                        "End of tick {}, starting tick {}.",
                        self.context.current_tick,
                        self.context.current_tick + TickDuration::SINGLE_TICK,
                    );
                    self.context.run_state_hooks_tick();

                    self.context.current_stratum = 0;
                    self.context.current_tick += TickDuration::SINGLE_TICK;
                    self.context.events_received_tick = false;

                    if current_tick_only {
                        tracing::trace!(
                            "`current_tick_only` is `true`, returning `false` before receiving events."
                        );
                        return false;
                    } else {
                        self.try_recv_events();
                        if std::mem::replace(&mut self.context.can_start_tick, false) {
                            tracing::trace!(
                                tick = u64::from(self.context.current_tick),
                                "`can_start_tick` is `true`, continuing."
                            );
                            // Do a full loop more to find where events have been added.
                            end_stratum = 0;
                            continue;
                        } else {
                            tracing::trace!(
                                "`can_start_tick` is `false`, re-setting `events_received_tick = false`, returning `false`."
                            );
                            self.context.events_received_tick = false;
                            return false;
                        }
                    }
                }
            }

            // After incrementing, exit if we made a full loop around the strata.
            if new_tick_started && end_stratum == self.context.current_stratum {
                tracing::trace!(
                    "Made full loop around stratum, re-setting `current_stratum = 0`, returning `false`."
                );
                // Note: if current stratum had work, the very first loop iteration would've
                // returned true. Therefore we can return false without checking.
                // Also means nothing was done so we can reset the stratum to zero and wait for
                // events.
                self.context.events_received_tick = false;
                self.context.current_stratum = 0;
                return false;
            }
        }
    }

    /// Runs the dataflow graph forever.
    ///
    /// TODO(mingwei): Currently blocks forever, no notion of "completion."
    #[tracing::instrument(level = "trace", skip(self), ret)]
    pub async fn run(&mut self) -> Option<Never> {
        loop {
            // Run any work which is immediately available.
            self.run_available().await;
            // When no work is available yield until more events occur.
            self.recv_events_async().await;
        }
    }

    /// [`Self::run`] but panics if a subgraph yields asynchronously.
    #[tracing::instrument(level = "trace", skip(self), ret)]
    pub fn run_sync(&mut self) -> Option<Never> {
        loop {
            // Run any work which is immediately available.
            self.run_available_sync();
            // When no work is available block until more events occur.
            self.recv_events();
        }
    }

    /// Enqueues subgraphs triggered by events without blocking.
    ///
    /// Returns the number of subgraphs enqueued, and if any were external.
    #[tracing::instrument(level = "trace", skip(self), fields(events_received_tick = self.context.events_received_tick), ret)]
    pub fn try_recv_events(&mut self) -> usize {
        let mut enqueued_count = 0;
        while let Ok((sg_id, is_external)) = self.context.event_queue_recv.try_recv() {
            let sg_data = &self.subgraphs[sg_id];
            tracing::trace!(
                sg_id = sg_id.to_string(),
                is_external = is_external,
                sg_stratum = sg_data.stratum,
                "Event received."
            );
            if !sg_data.is_scheduled.replace(true) {
                self.context.stratum_queues[sg_data.stratum].push_back(sg_id);
                enqueued_count += 1;
            }
            if is_external {
                // Next tick is triggered if we are at the start of the next tick (`!self.events_receved_tick`).
                // Or if the stratum is in the next tick.
                if !self.context.events_received_tick
                    || sg_data.stratum < self.context.current_stratum
                {
                    tracing::trace!(
                        current_stratum = self.context.current_stratum,
                        sg_stratum = sg_data.stratum,
                        "External event, setting `can_start_tick = true`."
                    );
                    self.context.can_start_tick = true;
                }
            }
        }
        self.context.events_received_tick = true;

        enqueued_count
    }

    /// Enqueues subgraphs triggered by external events, blocking until at
    /// least one subgraph is scheduled **from an external event**.
    #[tracing::instrument(level = "trace", skip(self), fields(events_received_tick = self.context.events_received_tick), ret)]
    pub fn recv_events(&mut self) -> Option<usize> {
        let mut count = 0;
        loop {
            let (sg_id, is_external) = self.context.event_queue_recv.blocking_recv()?;
            let sg_data = &self.subgraphs[sg_id];
            tracing::trace!(
                sg_id = sg_id.to_string(),
                is_external = is_external,
                sg_stratum = sg_data.stratum,
                "Event received."
            );
            if !sg_data.is_scheduled.replace(true) {
                self.context.stratum_queues[sg_data.stratum].push_back(sg_id);
                count += 1;
            }
            if is_external {
                // Next tick is triggered if we are at the start of the next tick (`!self.events_receved_tick`).
                // Or if the stratum is in the next tick.
                if !self.context.events_received_tick
                    || sg_data.stratum < self.context.current_stratum
                {
                    tracing::trace!(
                        current_stratum = self.context.current_stratum,
                        sg_stratum = sg_data.stratum,
                        "External event, setting `can_start_tick = true`."
                    );
                    self.context.can_start_tick = true;
                }
                break;
            }
        }
        self.context.events_received_tick = true;

        // Enqueue any other immediate events.
        let extra_count = self.try_recv_events();
        Some(count + extra_count)
    }

    /// Enqueues subgraphs triggered by external events asynchronously, waiting until at least one
    /// subgraph is scheduled **from an external event**. Returns the number of subgraphs enqueued,
    /// which may be zero if an external event scheduled an already-scheduled subgraph.
    ///
    /// Returns `None` if the event queue is closed, but that should not happen normally.
    #[tracing::instrument(level = "trace", skip(self), fields(events_received_tick = self.context.events_received_tick), ret)]
    pub async fn recv_events_async(&mut self) -> Option<usize> {
        let mut count = 0;
        loop {
            tracing::trace!("Awaiting events (`event_queue_recv`).");
            let (sg_id, is_external) = self.context.event_queue_recv.recv().await?;
            let sg_data = &self.subgraphs[sg_id];
            tracing::trace!(
                sg_id = sg_id.to_string(),
                is_external = is_external,
                sg_stratum = sg_data.stratum,
                "Event received."
            );
            if !sg_data.is_scheduled.replace(true) {
                self.context.stratum_queues[sg_data.stratum].push_back(sg_id);
                count += 1;
            }
            if is_external {
                // Next tick is triggered if we are at the start of the next tick (`!self.events_receved_tick`).
                // Or if the stratum is in the next tick.
                if !self.context.events_received_tick
                    || sg_data.stratum < self.context.current_stratum
                {
                    tracing::trace!(
                        current_stratum = self.context.current_stratum,
                        sg_stratum = sg_data.stratum,
                        "External event, setting `can_start_tick = true`."
                    );
                    self.context.can_start_tick = true;
                }
                break;
            }
        }
        self.context.events_received_tick = true;

        // Enqueue any other immediate events.
        let extra_count = self.try_recv_events();
        Some(count + extra_count)
    }

    /// Schedules a subgraph to be run. See also: [`Context::schedule_subgraph`].
    pub fn schedule_subgraph(&mut self, sg_id: SubgraphId) -> bool {
        let sg_data = &self.subgraphs[sg_id];
        let already_scheduled = sg_data.is_scheduled.replace(true);
        if !already_scheduled {
            self.context.stratum_queues[sg_data.stratum].push_back(sg_id);
            true
        } else {
            false
        }
    }

    /// Adds a new compiled subgraph with the specified inputs and outputs in stratum 0.
    pub fn add_subgraph<Name, R, W, Func>(
        &mut self,
        name: Name,
        recv_ports: R,
        send_ports: W,
        subgraph: Func,
    ) -> SubgraphId
    where
        Name: Into<Cow<'static, str>>,
        R: 'static + PortList<RECV>,
        W: 'static + PortList<SEND>,
        Func: 'a + for<'ctx> AsyncFnMut(&'ctx mut Context, R::Ctx<'ctx>, W::Ctx<'ctx>),
    {
        self.add_subgraph_stratified(name, 0, recv_ports, send_ports, false, subgraph)
    }

    /// Adds a new compiled subgraph with the specified inputs, outputs, and stratum number.
    ///
    /// TODO(mingwei): add example in doc.
    pub fn add_subgraph_stratified<Name, R, W, Func>(
        &mut self,
        name: Name,
        stratum: usize,
        recv_ports: R,
        send_ports: W,
        laziness: bool,
        subgraph: Func,
    ) -> SubgraphId
    where
        Name: Into<Cow<'static, str>>,
        R: 'static + PortList<RECV>,
        W: 'static + PortList<SEND>,
        Func: 'a + for<'ctx> AsyncFnMut(&'ctx mut Context, R::Ctx<'ctx>, W::Ctx<'ctx>),
    {
        self.add_subgraph_full(
            name, stratum, recv_ports, send_ports, laziness, None, subgraph,
        )
    }

    /// Adds a new compiled subgraph with all options.
    #[expect(clippy::too_many_arguments, reason = "Mainly for internal use.")]
    pub fn add_subgraph_full<Name, R, W, Func>(
        &mut self,
        name: Name,
        stratum: usize,
        recv_ports: R,
        send_ports: W,
        laziness: bool,
        loop_id: Option<LoopId>,
        mut subgraph: Func,
    ) -> SubgraphId
    where
        Name: Into<Cow<'static, str>>,
        R: 'static + PortList<RECV>,
        W: 'static + PortList<SEND>,
        Func: 'a + for<'ctx> AsyncFnMut(&'ctx mut Context, R::Ctx<'ctx>, W::Ctx<'ctx>),
    {
        // SAFETY: Check that the send and recv ports are from `self.handoffs`.
        recv_ports.assert_is_from(&self.handoffs);
        send_ports.assert_is_from(&self.handoffs);

        let loop_depth = loop_id
            .and_then(|loop_id| self.context.loop_depth.get(loop_id))
            .copied()
            .unwrap_or(0);

        let sg_id = self.subgraphs.insert_with_key(|sg_id| {
            let (mut subgraph_preds, mut subgraph_succs) = Default::default();
            recv_ports.set_graph_meta(&mut self.handoffs, &mut subgraph_preds, sg_id, true);
            send_ports.set_graph_meta(&mut self.handoffs, &mut subgraph_succs, sg_id, false);

            let subgraph =
                async move |context: &mut Context,
                            handoffs: &mut SlotVec<HandoffTag, HandoffData>| {
                    let (recv, send) = unsafe {
                        // SAFETY:
                        // 1. We checked `assert_is_from` at assembly time, above.
                        // 2. `SlotVec` is insert-only so no handoffs could have changed since then.
                        (
                            recv_ports.make_ctx(&*handoffs),
                            send_ports.make_ctx(&*handoffs),
                        )
                    };
                    (subgraph)(context, recv, send).await;
                };
            SubgraphData::new(
                name.into(),
                stratum,
                subgraph,
                subgraph_preds,
                subgraph_succs,
                true,
                laziness,
                loop_id,
                loop_depth,
            )
        });
        self.context.init_stratum(stratum);
        self.context.stratum_queues[stratum].push_back(sg_id);

        // Initialize subgraph metrics struct.
        Rc::make_mut(&mut self.metrics)
            .subgraphs
            .insert(sg_id, Default::default());

        sg_id
    }

    /// Adds a new compiled subgraph with a variable number of inputs and outputs of the same respective handoff types.
    pub fn add_subgraph_n_m<Name, R, W, Func>(
        &mut self,
        name: Name,
        recv_ports: Vec<RecvPort<R>>,
        send_ports: Vec<SendPort<W>>,
        subgraph: Func,
    ) -> SubgraphId
    where
        Name: Into<Cow<'static, str>>,
        R: 'static + Handoff,
        W: 'static + Handoff,
        Func: 'a
            + for<'ctx> AsyncFnMut(
                &'ctx mut Context,
                &'ctx [&'ctx RecvCtx<R>],
                &'ctx [&'ctx SendCtx<W>],
            ),
    {
        self.add_subgraph_stratified_n_m(name, 0, recv_ports, send_ports, subgraph)
    }

    /// Adds a new compiled subgraph with a variable number of inputs and outputs of the same respective handoff types.
    pub fn add_subgraph_stratified_n_m<Name, R, W, Func>(
        &mut self,
        name: Name,
        stratum: usize,
        recv_ports: Vec<RecvPort<R>>,
        send_ports: Vec<SendPort<W>>,
        mut subgraph: Func,
    ) -> SubgraphId
    where
        Name: Into<Cow<'static, str>>,
        R: 'static + Handoff,
        W: 'static + Handoff,
        Func: 'a
            + for<'ctx> AsyncFnMut(
                &'ctx mut Context,
                &'ctx [&'ctx RecvCtx<R>],
                &'ctx [&'ctx SendCtx<W>],
            ),
    {
        let sg_id = self.subgraphs.insert_with_key(|sg_id| {
            let subgraph_preds = recv_ports.iter().map(|port| port.handoff_id).collect();
            let subgraph_succs = send_ports.iter().map(|port| port.handoff_id).collect();

            for recv_port in recv_ports.iter() {
                self.handoffs[recv_port.handoff_id].succs.push(sg_id);
            }
            for send_port in send_ports.iter() {
                self.handoffs[send_port.handoff_id].preds.push(sg_id);
            }

            let subgraph =
                async move |context: &mut Context,
                            handoffs: &mut SlotVec<HandoffTag, HandoffData>| {
                    let recvs: Vec<&RecvCtx<R>> = recv_ports
                        .iter()
                        .map(|hid| hid.handoff_id)
                        .map(|hid| handoffs.get(hid).unwrap())
                        .map(|h_data| {
                            <dyn Any>::downcast_ref(&*h_data.handoff)
                                .expect("Attempted to cast handoff to wrong type.")
                        })
                        .map(RefCast::ref_cast)
                        .collect();

                    let sends: Vec<&SendCtx<W>> = send_ports
                        .iter()
                        .map(|hid| hid.handoff_id)
                        .map(|hid| handoffs.get(hid).unwrap())
                        .map(|h_data| {
                            <dyn Any>::downcast_ref(&*h_data.handoff)
                                .expect("Attempted to cast handoff to wrong type.")
                        })
                        .map(RefCast::ref_cast)
                        .collect();

                    (subgraph)(context, &recvs, &sends).await;
                };
            SubgraphData::new(
                name.into(),
                stratum,
                subgraph,
                subgraph_preds,
                subgraph_succs,
                true,
                false,
                None,
                0,
            )
        });
        self.context.init_stratum(stratum);
        self.context.stratum_queues[stratum].push_back(sg_id);

        // Initialize subgraph metrics struct.
        Rc::make_mut(&mut self.metrics)
            .subgraphs
            .insert(sg_id, Default::default());

        sg_id
    }

    /// Creates a handoff edge and returns the corresponding send and receive ports.
    pub fn make_edge<Name, H>(&mut self, name: Name) -> (SendPort<H>, RecvPort<H>)
    where
        Name: Into<Cow<'static, str>>,
        H: 'static + Handoff,
    {
        // Create and insert handoff.
        let handoff = H::default();
        let handoff_id = self
            .handoffs
            .insert_with_key(|hoff_id| HandoffData::new(name.into(), handoff, hoff_id));

        // Initialize handoff metrics struct.
        Rc::make_mut(&mut self.metrics)
            .handoffs
            .insert(handoff_id, Default::default());

        // Make ports.
        let input_port = SendPort {
            handoff_id,
            _marker: PhantomData,
        };
        let output_port = RecvPort {
            handoff_id,
            _marker: PhantomData,
        };
        (input_port, output_port)
    }

    /// Adds referenceable state into this instance. Returns a state handle which can be
    /// used externally or by operators to access the state.
    ///
    /// This is part of the "state API".
    pub fn add_state<T>(&mut self, state: T) -> StateHandle<T>
    where
        T: Any,
    {
        self.context.add_state(state)
    }

    /// Sets a hook to modify the state at the end of each tick, using the supplied closure.
    ///
    /// This is part of the "state API".
    pub fn set_state_lifespan_hook<T>(
        &mut self,
        handle: StateHandle<T>,
        lifespan: StateLifespan,
        hook_fn: impl 'static + FnMut(&mut T),
    ) where
        T: Any,
    {
        self.context
            .set_state_lifespan_hook(handle, lifespan, hook_fn)
    }

    /// Gets a exclusive (mut) ref to the internal context, setting the subgraph ID.
    pub fn context_mut(&mut self, sg_id: SubgraphId) -> &mut Context {
        self.context.subgraph_id = sg_id;
        &mut self.context
    }

    /// Adds a new loop with the given parent (or `None` for top-level). Returns a loop ID which
    /// is used in [`Self::add_subgraph_stratified`] or for nested loops.
    ///
    /// TODO(mingwei): add loop names to ensure traceability while debugging?
    pub fn add_loop(&mut self, parent: Option<LoopId>) -> LoopId {
        let depth = parent.map_or(0, |p| self.context.loop_depth[p] + 1);
        let loop_id = self.context.loop_depth.insert(depth);
        self.loop_data.insert(
            loop_id,
            LoopData {
                iter_count: None,
                allow_another_iteration: true,
            },
        );
        loop_id
    }

    /// Returns a referenced-counted handle to the continually-updated runtime metrics for this DFIR instance.
    pub fn metrics(&self) -> Rc<DfirMetrics> {
        Rc::clone(&self.metrics)
    }

    /// Returns an infinite iterator of [`DfirMetrics`], where each call to `next()` ends an interval.
    ///
    /// The first call to [`DfirMetricsIntervals::next`] returns metrics since this DFIR instance was created. Each
    /// subsequent call to [`DfirMetricsIntervals::next`] returns metrics since the previous call.
    ///
    /// Cloning the iterator "forks" it from the original, as afterwards each iterator will return different metrics
    /// based on when [`DfirMetricsIntervals::next`] is called.
    pub fn metrics_intervals(&self) -> DfirMetricsIntervals {
        DfirMetricsIntervals {
            curr: self.metrics(),
            prev: None,
        }
    }
}

impl Dfir<'_> {
    /// Alias for [`Context::request_task`].
    pub fn request_task<Fut>(&mut self, future: Fut)
    where
        Fut: Future<Output = ()> + 'static,
    {
        self.context.request_task(future);
    }

    /// Alias for [`Context::abort_tasks`].
    pub fn abort_tasks(&mut self) {
        self.context.abort_tasks()
    }

    /// Alias for [`Context::join_tasks`].
    pub fn join_tasks(&mut self) -> impl use<'_> + Future {
        self.context.join_tasks()
    }
}

fn run_sync<Fut>(fut: Fut) -> Fut::Output
where
    Fut: Future,
{
    let mut fut = std::pin::pin!(fut);
    let mut ctx = std::task::Context::from_waker(std::task::Waker::noop());
    match fut.as_mut().poll(&mut ctx) {
        std::task::Poll::Ready(out) => out,
        std::task::Poll::Pending => panic!("Future did not resolve immediately."),
    }
}

impl Drop for Dfir<'_> {
    fn drop(&mut self) {
        self.abort_tasks();
    }
}

/// A handoff and its input and output [SubgraphId]s.
///
/// Internal use: used to track the dfir graph structure.
///
/// TODO(mingwei): restructure `PortList` so this can be crate-private.
#[doc(hidden)]
pub struct HandoffData {
    /// A friendly name for diagnostics.
    pub(super) name: Cow<'static, str>,
    /// Crate-visible to crate for `handoff_list` internals.
    pub(super) handoff: Box<dyn HandoffMeta>,
    /// Preceeding subgraphs (including the send side of a teeing handoff).
    pub(super) preds: SmallVec<[SubgraphId; 1]>,
    /// Successor subgraphs (including recv sides of teeing handoffs).
    pub(super) succs: SmallVec<[SubgraphId; 1]>,

    /// Predecessor handoffs, used by teeing handoffs.
    /// Should be `self` on any teeing send sides (input).
    /// Should be the send `HandoffId` if this is teeing recv side (output).
    /// Should be just `self`'s `HandoffId` on other handoffs.
    /// This field is only used in initialization.
    pub(super) pred_handoffs: Vec<HandoffId>,
    /// Successor handoffs, used by teeing handoffs.
    /// Should be a list of outputs on the teeing send side (input).
    /// Should be `self` on any teeing recv sides (outputs).
    /// Should be just `self`'s `HandoffId` on other handoffs.
    /// This field is only used in initialization.
    pub(super) succ_handoffs: Vec<HandoffId>,
}

impl std::fmt::Debug for HandoffData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.debug_struct("HandoffData")
            .field("preds", &self.preds)
            .field("succs", &self.succs)
            .finish_non_exhaustive()
    }
}

impl HandoffData {
    /// New with `pred_handoffs` and `succ_handoffs` set to its own [`HandoffId`]: `vec![hoff_id]`.
    pub fn new(
        name: Cow<'static, str>,
        handoff: impl 'static + HandoffMeta,
        hoff_id: HandoffId,
    ) -> Self {
        let (preds, succs) = Default::default();
        Self {
            name,
            handoff: Box::new(handoff),
            preds,
            succs,
            pred_handoffs: vec![hoff_id],
            succ_handoffs: vec![hoff_id],
        }
    }
}

/// A subgraph along with its predecessor and successor [SubgraphId]s.
///
/// Used internally by the [Dfir] struct to represent the dataflow graph
/// structure and scheduled state.
pub(super) struct SubgraphData<'a> {
    /// A friendly name for diagnostics.
    pub(super) name: Cow<'static, str>,
    /// This subgraph's stratum number.
    ///
    /// Within loop blocks, corresponds to the topological sort of the DAG created when `next_loop()/next_tick()` are removed.
    pub(super) stratum: usize,
    /// The actual execution code of the subgraph.
    subgraph: Box<dyn 'a + Subgraph>,

    preds: Vec<HandoffId>,
    succs: Vec<HandoffId>,

    /// If this subgraph is scheduled in [`dfir_rs::stratum_queues`].
    /// [`Cell`] allows modifying this field when iterating `Self::preds` or
    /// `Self::succs`, as all `SubgraphData` are owned by the same vec
    /// `dfir_rs::subgraphs`.
    is_scheduled: Cell<bool>,

    /// Keep track of the last tick that this subgraph was run in
    last_tick_run_in: Option<TickInstant>,
    /// A meaningless ID to track the last loop execution this subgraph was run in.
    /// `(loop_nonce, iter_count)` pair.
    last_loop_nonce: (usize, Option<usize>),

    /// If this subgraph is marked as lazy, then sending data back to a lower stratum does not trigger a new tick to be run.
    is_lazy: bool,

    /// The subgraph's loop ID, or `None` for the top level.
    loop_id: Option<LoopId>,
    /// The loop depth of the subgraph.
    loop_depth: usize,
}

impl<'a> SubgraphData<'a> {
    #[expect(clippy::too_many_arguments, reason = "internal use")]
    pub(crate) fn new(
        name: Cow<'static, str>,
        stratum: usize,
        subgraph: impl 'a + Subgraph,
        preds: Vec<HandoffId>,
        succs: Vec<HandoffId>,
        is_scheduled: bool,
        is_lazy: bool,
        loop_id: Option<LoopId>,
        loop_depth: usize,
    ) -> Self {
        Self {
            name,
            stratum,
            subgraph: Box::new(subgraph),
            preds,
            succs,
            is_scheduled: Cell::new(is_scheduled),
            last_tick_run_in: None,
            last_loop_nonce: (0, None),
            is_lazy,
            loop_id,
            loop_depth,
        }
    }
}

pub(crate) struct LoopData {
    /// Count of iterations of this loop.
    iter_count: Option<usize>,
    /// If the loop has reason to do another iteration.
    allow_another_iteration: bool,
}

/// Defines when state should be reset.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StateLifespan {
    /// Always reset, associated with the subgraph.
    Subgraph(SubgraphId),
    /// Reset between loop executions.
    Loop(LoopId),
    /// Reset between ticks.
    Tick,
    /// Never reset.
    Static,
}
