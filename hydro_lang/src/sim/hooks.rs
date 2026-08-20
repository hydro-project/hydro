//! Test-side API for **simulator hooks**: scripting the decisions of unsafe operators.
//!
//! A hook handle (see [`crate::sim_hooks`]) is created from the
//! [`FlowBuilder`](crate::compile::builder::FlowBuilder) via
//! [`FlowBuilder::sim_hook`](crate::compile::builder::FlowBuilder::sim_hook) and attached
//! to one specific unsafe operator with `nondet!(/** reason */ hook = handle)`. Inside a
//! simulation test body (under [`SimFlow::deterministic`](crate::sim::flow::SimFlow::deterministic),
//! [`fuzz`](crate::sim::flow::SimFlow::fuzz), or
//! [`exhaustive`](crate::sim::flow::SimFlow::exhaustive)), the handle scripts the
//! operator's decisions.
//!
//! # The script is a schedule
//!
//! Decision calls are `async`, and the sequence of calls in the test body is a
//! **schedule**, read in program order:
//!
//! - Consecutive decisions that target *different hooks of the same tick* form a
//!   **group**: one execution of that tick will consume all of them together.
//! - A decision that targets a different tick — or the *next execution* of the same tick
//!   (scripting a hook that already has a decision in the current group) — starts a new
//!   group. The `.await` on the first decision of a new group suspends the test until the
//!   previous group's tick execution has actually happened, so the test body advances in
//!   lockstep with the execution it describes. Decisions whose turn has already come
//!   return immediately without suspending.
//! - Output awaits are group barriers: an output await completes only after every
//!   decision scripted so far has been consumed.
//!
//! A decision may be scripted before its data exists (`release(3)` immediately after
//! `send_many([1, 2, 3])`): the tick simply fires at the first moment the decision can be
//! honored in full. A decision that can *never* be honored is reported when the
//! simulation runs out of other work, attributed to the test line that is suspended
//! waiting on it.
//!
//! # Holding data on purpose
//!
//! A hook with buffered data and no decision is an error the simulator reports at every
//! scheduling boundary. When buffering *is* the scenario, declare it with the `pause`
//! family ([`BatchHook::pause`], [`BatchHook::pause_while`],
//! [`BatchHook::pause_until_count`], [`BatchHook::auto_pause`], and the snapshot
//! equivalents).

use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::live_collections::boundedness::{Bounded, Unbounded};
use crate::live_collections::stream::{NoOrder, Ordering, Retries, TotalOrder};
use crate::sim::compiled::{
    ScheduleDecision, script_ctx, script_stuck_error, script_unconsumed_description,
};
use crate::sim::runtime::{
    BatchDecision, InlineOrderingDecision, KeyedBatchDecision, KeyedSnapshotDecision,
    MergeDecision, ScriptDecision, SnapshotDecision, TopLevelOrderingDecision,
    UnorderedBatchDecision, UnorderedKeyedBatchDecision,
};
pub use crate::sim::runtime::{
    BatchStatus, KeyedSnapshotStatus, MergeStatus, OrderingStatus, SnapshotStatus,
};
pub use crate::sim_hooks::{
    BatchHook, KeyedBatchHook, KeyedMergeOrderedHook, KeyedOrderingHook, KeyedSnapshotHook,
    MergeOrderedHook, OrderingHook, PartialOrderingHook, SimHook, SnapshotHook,
};

/// A scripted decision that has been issued but not yet installed into the schedule.
///
/// Awaiting it suspends the test until every previously scripted tick execution the
/// decision must come after has actually happened (see the module docs); it resolves once
/// the decision is installed for its tick's next execution. Panics (at the `.await`'s
/// location) if the decision can never take its place in the schedule.
#[must_use = "a scripted decision does nothing until awaited"]
pub struct DecisionFuture {
    hook_id: usize,
    /// The decision, bincode-serialized (the handle and the hook it is bound to
    /// statically know the same decision type). `None` once installed.
    blob: Option<Vec<u8>>,
}

impl DecisionFuture {
    fn new(hook_id: usize, decision: &impl ScriptDecision) -> Self {
        DecisionFuture {
            hook_id,
            blob: Some(bincode::serialize(decision).unwrap()),
        }
    }
}

/// Panics (at the scripting call site) when a per-key decision names the same key more
/// than once, establishing the no-duplicate-keys invariant of the keyed decisions before
/// they are installed.
#[track_caller]
fn assert_distinct_keys<'a, K: std::hash::Hash + Eq + 'a>(
    keys: impl Iterator<Item = &'a K>,
    method: &str,
) {
    let mut seen: dfir_rs::rustc_hash::FxHashSet<&K> = Default::default();
    for (position, key) in keys.enumerate() {
        assert!(
            seen.insert(key),
            "{}: the same key appears more than once in a single decision (duplicate at entry {}); a key takes exactly one decision per tick",
            method,
            position
        );
    }
}

impl Future for DecisionFuture {
    type Output = ();

    #[track_caller]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let Some(blob) = this.blob.take() else {
            return Poll::Ready(());
        };

        let ctx = script_ctx();
        match ctx.try_schedule_decision(this.hook_id, blob) {
            Ok(ScheduleDecision::Installed) => Poll::Ready(()),
            Ok(ScheduleDecision::Wait(blob)) => {
                this.blob = Some(blob);
                // While the script waits, the rest of the simulation does not: the
                // scheduler keeps freely choosing which *other* ticks run. The test body
                // is re-polled after every scheduler step; the waker is only needed for
                // the parked (quiescent) case.
                ctx.push_park_waker(cx.waker());
                Poll::Pending
            }
            Err(message) => panic!("{}", message),
        }
    }
}

/// A `pause_until` wait: resolves once the hook's pending-input status satisfies the
/// predicate, un-pausing the hook. The status is read from the hook **on demand** at every
/// poll (the test body is polled between every pair of scheduler steps), so the wait
/// resolves at the first scheduling point where the predicate holds. Panics (at the
/// `.await`'s location) if the simulation can no longer satisfy it.
#[must_use = "the pause is only released once this future is awaited"]
pub struct PauseUntilFuture<S, F> {
    hook_id: usize,
    /// What the wait is called in error messages (e.g. `pause_until_count(3)`).
    label: String,
    predicate: F,
    _status: PhantomData<fn(S)>,
}

impl<S: DeserializeOwned, F: Fn(&S) -> bool + Unpin> Future for PauseUntilFuture<S, F> {
    type Output = ();

    #[track_caller]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        let ctx = script_ctx();

        // A `pause_until` wait is a script barrier, like an output await: it must not
        // resolve (dropping the hold) while an earlier decision group is unconsumed,
        // or the next decision would spuriously overlap the outstanding group and the
        // boundary scan would see the exposed hook mid-group. And if that group is
        // stuck, it is the root cause — report it instead of blaming the predicate.
        if let Some(stuck) = script_unconsumed_description() {
            if ctx.is_quiescent() {
                panic!("{}", script_stuck_error(&stuck));
            }
            ctx.push_park_waker(cx.waker());
            return Poll::Pending;
        }

        let hook = ctx.control(this.hook_id);

        let status: S = bincode::deserialize(&hook.borrow().status_blob())
            .expect("internal error: hook status blob did not match the handle's status type");

        if (this.predicate)(&status) {
            // The wait is satisfied; the hook is unpaused (the ordinary missing-decision
            // error applies from here on).
            hook.borrow_mut().release_hold();
            Poll::Ready(())
        } else if ctx.is_quiescent() {
            let hook = hook.borrow();
            let loc = hook.location_meta().location;
            panic!(
                "{} can never be satisfied: the hook at {} has {} and the simulation has no more work it can do",
                this.label,
                loc,
                hook.describe_pending()
                    .unwrap_or_else(|| "no pending input".to_owned()),
            );
        } else {
            ctx.push_park_waker(cx.waker());
            Poll::Pending
        }
    }
}

/// RAII guard for [`BatchHook::pause_while`] / [`SnapshotHook::pause_while`]: ends the
/// hold when dropped (even on panic), leaving a standing `auto_pause` hold in place.
struct PauseGuard {
    hook_id: usize,
}

impl Drop for PauseGuard {
    fn drop(&mut self) {
        let ctx = script_ctx();
        let hook = ctx.control(self.hook_id);
        hook.borrow_mut().release_hold();
    }
}

macro_rules! pause_family {
    ($status:ty) => {
        /// Declares that buffering at this operator is intended: while paused, the hook is
        /// exempt from the missing-decision error, never causes its tick to run, and — if
        /// its tick runs anyway because *other* hooks feed it — contributes its "nothing
        /// new" behavior each time. Scripting any decision implicitly resumes the hook.
        ///
        /// A pause takes its place in the script like everything else: requested while a
        /// decision is still pending, the hold begins once that decision has been
        /// consumed.
        pub fn pause(&self) {
            let ctx = script_ctx();
            ctx.control(self.id).borrow_mut().set_hold(true);
        }

        /// Ends a [`Self::pause`] (and clears [`Self::auto_pause`] mode).
        pub fn resume(&self) {
            let ctx = script_ctx();
            let hook = ctx.control(self.id);
            let mut hook = hook.borrow_mut();
            hook.set_auto_pause(false);
            hook.set_hold(false);
        }

        /// Sets a standing mode where this hook only ever acts when scripted: it holds
        /// immediately, and every scripted decision leaves a fresh hold in place behind
        /// it.
        ///
        /// This deliberately opts out of the forgotten-hook protection: if the test
        /// forgets a step, the operator silently holds its data instead of failing. The
        /// one `auto_pause()` line at the top of a test is the reviewer-visible marker
        /// that this hook's timing is entirely script-driven, missed steps and all.
        pub fn auto_pause(&self) {
            let ctx = script_ctx();
            let hook = ctx.control(self.id);
            let mut hook = hook.borrow_mut();
            hook.set_auto_pause(true);
            hook.set_hold(true);
        }

        /// Pauses the hook exactly for the duration of `body` (resuming even on panic), so
        /// a bracketed buffering phase cannot leak a paused hook.
        pub async fn pause_while<Fut: Future>(&self, body: Fut) -> Fut::Output {
            self.pause();
            let _guard = PauseGuard { hook_id: self.id };
            body.await
        }

        /// Pauses the hook and returns a future that resolves once the hook's
        /// pending-input status satisfies `predicate` — a synchronization point for
        /// scripts where the right decision is not knowable upfront. The status is read
        /// on demand at every scheduling point. After the future resolves, the hook is
        /// unpaused; the ordinary missing-decision error applies from there on.
        pub fn pause_until(
            &self,
            predicate: impl Fn(&$status) -> bool + Unpin,
        ) -> PauseUntilFuture<$status, impl Fn(&$status) -> bool + Unpin> {
            self.pause_until_labeled("pause_until(..)".to_owned(), predicate)
        }

        fn pause_until_labeled<F: Fn(&$status) -> bool + Unpin>(
            &self,
            label: String,
            predicate: F,
        ) -> PauseUntilFuture<$status, F> {
            let ctx = script_ctx();
            ctx.control(self.id).borrow_mut().set_hold(true);
            PauseUntilFuture {
                hook_id: self.id,
                label,
                predicate,
                _status: PhantomData,
            }
        }
    };
}

impl<T, O: Ordering, R: Retries> BatchHook<T, O, R> {
    pause_family!(BatchStatus);

    /// Pauses the hook and returns a future that resolves once at least `n` elements are
    /// buffered; see [`Self::pause_until`].
    pub fn pause_until_count(
        &self,
        n: usize,
    ) -> PauseUntilFuture<BatchStatus, impl Fn(&BatchStatus) -> bool + Unpin> {
        self.pause_until_labeled(format!("pause_until_count({})", n), move |status| {
            status.buffered >= n
        })
    }
}

impl<T, R: Retries> BatchHook<T, TotalOrder, R>
where
    T: Serialize + DeserializeOwned + PartialEq,
{
    /// Scripts the next batch to be exactly the next `n` buffered elements. The tick
    /// fires at the first moment the decision can be honored in full.
    pub fn release(&self, n: usize) -> DecisionFuture {
        DecisionFuture::new(self.id, &BatchDecision::<T>::Prefix(n))
    }

    /// Scripts the next batch to be exactly this sequence of values. Values must match the
    /// buffered prefix in order: a mismatching available value panics immediately, while a
    /// matching but incomplete prefix waits for the remaining values to arrive.
    pub fn release_values(&self, values: impl IntoIterator<Item = T>) -> DecisionFuture {
        DecisionFuture::new(
            self.id,
            &BatchDecision::Values(values.into_iter().collect()),
        )
    }

    /// Scripts the next batch to be everything that has arrived by the time the tick
    /// fires. Under fuzzing, the released contents co-vary with the schedule being
    /// explored; use [`Self::release`] to name them exactly.
    pub fn release_all(&self) -> DecisionFuture {
        DecisionFuture::new(self.id, &BatchDecision::<T>::All)
    }

    /// Scripts the next batch to be empty, holding everything buffered. Shorthand for
    /// [`Self::release`]`(0)`.
    pub fn release_empty(&self) -> DecisionFuture {
        self.release(0)
    }
}

impl<T, R: Retries> BatchHook<T, NoOrder, R>
where
    T: Serialize + DeserializeOwned + PartialEq,
{
    /// Scripts the next batch to contain exactly this multiset of buffered values. Values
    /// are matched independently of arrival order; duplicates request the corresponding
    /// number of equal buffered items. The tick fires once every requested value exists.
    pub fn release_values(&self, values: impl IntoIterator<Item = T>) -> DecisionFuture {
        DecisionFuture::new(
            self.id,
            &UnorderedBatchDecision::Values(values.into_iter().collect()),
        )
    }

    /// Scripts the next batch to be everything that has arrived by the time the tick
    /// fires. Under fuzzing, the released contents co-vary with the schedule being
    /// explored; use [`Self::release_values`] to name them exactly.
    pub fn release_all(&self) -> DecisionFuture {
        DecisionFuture::new(self.id, &UnorderedBatchDecision::<T>::All)
    }

    /// Scripts the next batch to be empty, holding everything buffered. Shorthand for
    /// [`Self::release_values`] with no values.
    pub fn release_empty(&self) -> DecisionFuture {
        self.release_values([])
    }
}

impl<T> OrderingHook<T, Unbounded>
where
    T: Serialize + DeserializeOwned,
{
    /// Scripts a top-level `assume_ordering` action to release the buffered element equal
    /// to `value`. Exactly one element is released, preserving opportunities for ticks and
    /// feedback to interleave with the remaining buffered input.
    pub fn next(&self, value: T) -> DecisionFuture {
        DecisionFuture::new(self.id, &TopLevelOrderingDecision::Next(value))
    }

    pause_family!(OrderingStatus);

    /// Pauses a top-level ordering hook until at least `n` elements are buffered; see
    /// [`Self::pause_until`].
    pub fn pause_until_count(
        &self,
        n: usize,
    ) -> PauseUntilFuture<OrderingStatus, impl Fn(&OrderingStatus) -> bool + Unpin> {
        self.pause_until_labeled(format!("pause_until_count({})", n), move |status| {
            status.buffered >= n
        })
    }
}

impl<T> OrderingHook<T, Bounded>
where
    T: Serialize + DeserializeOwned,
{
    /// Scripts an in-tick `assume_ordering` observation to consume its complete input in
    /// exactly this order. The supplied values must be a permutation of all values received
    /// by the operator during that tick.
    pub fn order(&self, values: impl IntoIterator<Item = T>) -> DecisionFuture {
        DecisionFuture::new(
            self.id,
            &InlineOrderingDecision::Order(values.into_iter().collect()),
        )
    }
}

impl<T> SnapshotHook<T> {
    /// Scripts the next tick execution to observe the buffered version equal to `value`:
    /// scans forward from the currently-revealed version through the buffered ones and
    /// releases the first equal version, skipping over earlier versions.
    ///
    /// This is a combined assertion and release, and the recommended way to script
    /// snapshots: a script written with positional decisions breaks silently when the
    /// program changes how often the state updates, while `reveal(value)` names the state
    /// it means and any mis-synchronization fails loudly at the reveal.
    pub fn reveal(&self, value: T) -> DecisionFuture
    where
        T: Serialize + DeserializeOwned,
    {
        DecisionFuture::new(self.id, &SnapshotDecision::Reveal(value))
    }

    /// Scripts the next tick execution to observe the next buffered version.
    pub fn reveal_next(&self) -> DecisionFuture
    where
        T: Serialize + DeserializeOwned,
    {
        DecisionFuture::new(self.id, &SnapshotDecision::<T>::RevealNext)
    }

    /// Scripts the next tick execution to observe the newest version that has arrived by
    /// the time the tick fires. Under fuzzing, which version is newest co-varies with the
    /// schedule being explored; use [`Self::reveal`] to name it exactly.
    pub fn reveal_latest(&self) -> DecisionFuture
    where
        T: Serialize + DeserializeOwned,
    {
        DecisionFuture::new(self.id, &SnapshotDecision::<T>::RevealLatest)
    }

    /// Scripts the next tick execution to observe the previously revealed version again.
    pub fn keep(&self) -> DecisionFuture
    where
        T: Serialize + DeserializeOwned,
    {
        DecisionFuture::new(self.id, &SnapshotDecision::<T>::Keep)
    }

    pause_family!(SnapshotStatus);

    /// Pauses the hook and returns a future that resolves once at least `n` newer
    /// versions are buffered; see [`Self::pause_until`].
    pub fn pause_until_versions(
        &self,
        n: usize,
    ) -> PauseUntilFuture<SnapshotStatus, impl Fn(&SnapshotStatus) -> bool + Unpin> {
        self.pause_until_labeled(format!("pause_until_versions({})", n), move |status| {
            status.newer_versions >= n
        })
    }
}

impl<K, V, O: Ordering, R: Retries> KeyedBatchHook<K, V, O, R> {
    pause_family!(BatchStatus);

    /// Pauses the hook and returns a future that resolves once at least `n` entries are
    /// buffered (in total, across all keys); see [`Self::pause_until`].
    pub fn pause_until_count(
        &self,
        n: usize,
    ) -> PauseUntilFuture<BatchStatus, impl Fn(&BatchStatus) -> bool + Unpin> {
        self.pause_until_labeled(format!("pause_until_count({})", n), move |status| {
            status.buffered >= n
        })
    }
}

impl<K, V, R: Retries> KeyedBatchHook<K, V, TotalOrder, R>
where
    K: Serialize + DeserializeOwned + PartialEq,
    V: Serialize + DeserializeOwned + PartialEq,
{
    /// Scripts the next batch to be exactly the next `count` buffered values of each
    /// named key. The tick fires at the first moment the decision can be honored in
    /// full.
    ///
    /// # Panics
    /// Panics immediately if `counts` names the same key more than once.
    #[track_caller]
    pub fn release(&self, counts: impl IntoIterator<Item = (K, usize)>) -> DecisionFuture
    where
        K: std::hash::Hash + Eq,
    {
        let counts: Vec<(K, usize)> = counts.into_iter().collect();
        assert_distinct_keys(counts.iter().map(|(key, _)| key), "release");
        DecisionFuture::new(self.id, &KeyedBatchDecision::<K, V>::Prefixes(counts))
    }

    /// Scripts the next batch to be exactly these `(key, value)` entries. Each key's
    /// values must match that key's buffered prefix in order (the interleaving of
    /// *different* keys in the scripted sequence is irrelevant): a mismatching available
    /// value panics immediately, while a matching but incomplete prefix waits for the
    /// remaining values to arrive.
    pub fn release_values(&self, entries: impl IntoIterator<Item = (K, V)>) -> DecisionFuture {
        DecisionFuture::new(
            self.id,
            &KeyedBatchDecision::Values(entries.into_iter().collect()),
        )
    }

    /// Scripts the next batch to be everything that has arrived by the time the tick
    /// fires. Under fuzzing, the released contents co-vary with the schedule being
    /// explored; use [`Self::release_values`] to name them exactly.
    pub fn release_all(&self) -> DecisionFuture {
        DecisionFuture::new(self.id, &KeyedBatchDecision::<K, V>::All)
    }

    /// Scripts the next batch to be empty, holding everything buffered. Shorthand for
    /// [`Self::release_values`] with no entries.
    pub fn release_empty(&self) -> DecisionFuture {
        self.release_values([])
    }
}

impl<K, V, R: Retries> KeyedBatchHook<K, V, NoOrder, R>
where
    K: Serialize + DeserializeOwned + PartialEq,
    V: Serialize + DeserializeOwned + PartialEq,
{
    /// Scripts the next batch to contain exactly these `(key, value)` entries. Values are
    /// matched per key as multisets (independently of arrival order); duplicates request
    /// the corresponding number of equal buffered items. The tick fires once every
    /// requested entry exists.
    pub fn release_values(&self, entries: impl IntoIterator<Item = (K, V)>) -> DecisionFuture {
        DecisionFuture::new(
            self.id,
            &UnorderedKeyedBatchDecision::Values(entries.into_iter().collect()),
        )
    }

    /// Scripts the next batch to be everything that has arrived by the time the tick
    /// fires. Under fuzzing, the released contents co-vary with the schedule being
    /// explored; use [`Self::release_values`] to name them exactly.
    pub fn release_all(&self) -> DecisionFuture {
        DecisionFuture::new(self.id, &UnorderedKeyedBatchDecision::<K, V>::All)
    }

    /// Scripts the next batch to be empty, holding everything buffered. Shorthand for
    /// [`Self::release_values`] with no entries.
    pub fn release_empty(&self) -> DecisionFuture {
        self.release_values([])
    }
}

impl<K, V> KeyedSnapshotHook<K, V> {
    /// Scripts the next tick execution to observe, for each named key, the buffered
    /// version equal to the named value: scans forward from that key's currently-revealed
    /// version through the buffered ones and releases the first equal version, skipping
    /// over earlier versions. Keys that are not named observe their previously revealed
    /// version again (or stay absent if they have never been revealed).
    ///
    /// This is a combined assertion and release, and the recommended way to script keyed
    /// snapshots: naming the state each key means makes any mis-synchronization fail
    /// loudly at the reveal.
    ///
    /// # Panics
    /// Panics immediately if `entries` names the same key more than once: a key observes
    /// exactly one version per tick execution.
    #[track_caller]
    pub fn reveal(&self, entries: impl IntoIterator<Item = (K, V)>) -> DecisionFuture
    where
        K: Serialize + DeserializeOwned + std::hash::Hash + Eq,
        V: Serialize + DeserializeOwned,
    {
        let entries: Vec<(K, V)> = entries.into_iter().collect();
        assert_distinct_keys(entries.iter().map(|(key, _)| key), "reveal");
        DecisionFuture::new(self.id, &KeyedSnapshotDecision::Reveal(entries))
    }

    /// Scripts the next tick execution to observe, for every key, the newest version that
    /// has arrived by the time the tick fires (keys with nothing newer observe their
    /// previously revealed version again). Under fuzzing, which versions are newest
    /// co-varies with the schedule being explored; use [`Self::reveal`] to name them
    /// exactly.
    pub fn reveal_latest(&self) -> DecisionFuture
    where
        K: Serialize + DeserializeOwned,
        V: Serialize + DeserializeOwned,
    {
        DecisionFuture::new(self.id, &KeyedSnapshotDecision::<K, V>::RevealLatest)
    }

    /// Scripts the next tick execution to observe every key's previously revealed version
    /// again.
    pub fn keep(&self) -> DecisionFuture
    where
        K: Serialize + DeserializeOwned,
        V: Serialize + DeserializeOwned,
    {
        DecisionFuture::new(self.id, &KeyedSnapshotDecision::<K, V>::Keep)
    }

    pause_family!(KeyedSnapshotStatus);

    /// Pauses the hook and returns a future that resolves once at least `n` newer
    /// versions are buffered (in total, across all keys); see [`Self::pause_until`].
    pub fn pause_until_versions(
        &self,
        n: usize,
    ) -> PauseUntilFuture<KeyedSnapshotStatus, impl Fn(&KeyedSnapshotStatus) -> bool + Unpin> {
        self.pause_until_labeled(format!("pause_until_versions({})", n), move |status| {
            status.newer_versions >= n
        })
    }
}

impl<K, V> KeyedOrderingHook<K, V, Unbounded>
where
    K: Serialize + DeserializeOwned,
    V: Serialize + DeserializeOwned,
{
    /// Scripts a top-level keyed `assume_ordering` action to release the buffered entry
    /// under `key` equal to `value`. Exactly one entry is released, preserving
    /// opportunities for ticks and feedback to interleave with the remaining buffered
    /// input.
    pub fn next(&self, key: K, value: V) -> DecisionFuture {
        DecisionFuture::new(self.id, &TopLevelOrderingDecision::Next((key, value)))
    }

    pause_family!(OrderingStatus);

    /// Pauses a top-level keyed ordering hook until at least `n` entries are buffered (in
    /// total, across all keys); see [`Self::pause_until`].
    pub fn pause_until_count(
        &self,
        n: usize,
    ) -> PauseUntilFuture<OrderingStatus, impl Fn(&OrderingStatus) -> bool + Unpin> {
        self.pause_until_labeled(format!("pause_until_count({})", n), move |status| {
            status.buffered >= n
        })
    }
}

impl<K, V> KeyedOrderingHook<K, V, Bounded>
where
    K: Serialize + DeserializeOwned,
    V: Serialize + DeserializeOwned,
{
    /// Scripts an in-tick keyed `assume_ordering` observation to consume its complete
    /// input with each key's values in exactly the scripted per-key order. The supplied
    /// entries must contain exactly all `(key, value)` entries received by the operator
    /// during that tick; the relative order of *different* keys in the scripted sequence
    /// is irrelevant (a keyed stream carries no cross-key ordering).
    pub fn order(&self, entries: impl IntoIterator<Item = (K, V)>) -> DecisionFuture {
        DecisionFuture::new(
            self.id,
            &InlineOrderingDecision::Order(entries.into_iter().collect()),
        )
    }
}

impl<K, V> PartialOrderingHook<K, V, Unbounded>
where
    K: Serialize + DeserializeOwned,
    V: Serialize + DeserializeOwned,
{
    /// Scripts a top-level `entries_partially_ordered` action to release the front entry
    /// of `key`'s buffer, which must equal `value` (within-key order is preserved, so a
    /// mismatch panics). Exactly one entry is released, preserving opportunities for
    /// ticks and feedback to interleave with the remaining buffered input.
    pub fn next(&self, key: K, value: V) -> DecisionFuture {
        DecisionFuture::new(self.id, &TopLevelOrderingDecision::Next((key, value)))
    }

    pause_family!(OrderingStatus);

    /// Pauses a top-level partially-ordered hook until at least `n` entries are buffered
    /// (in total, across all keys); see [`Self::pause_until`].
    pub fn pause_until_count(
        &self,
        n: usize,
    ) -> PauseUntilFuture<OrderingStatus, impl Fn(&OrderingStatus) -> bool + Unpin> {
        self.pause_until_labeled(format!("pause_until_count({})", n), move |status| {
            status.buffered >= n
        })
    }
}

impl<K, V> PartialOrderingHook<K, V, Bounded>
where
    K: Serialize + DeserializeOwned,
    V: Serialize + DeserializeOwned,
{
    /// Scripts an in-tick `entries_partially_ordered` observation to consume its complete
    /// input in exactly this interleaving. The supplied entries must be a permutation of
    /// all `(key, value)` entries received by the operator during that tick that preserves
    /// each key's within-key order.
    pub fn order(&self, entries: impl IntoIterator<Item = (K, V)>) -> DecisionFuture {
        DecisionFuture::new(
            self.id,
            &InlineOrderingDecision::Order(entries.into_iter().collect()),
        )
    }
}

impl<T> MergeOrderedHook<T, Unbounded>
where
    T: Serialize + DeserializeOwned,
{
    /// Scripts a top-level `merge_ordered` action to release the front element of the
    /// *first* input's buffer, which must equal `value` (per-input order is preserved, so
    /// a mismatch panics). Exactly one element is released, preserving opportunities for
    /// ticks and feedback to interleave with the remaining buffered input.
    pub fn next_first(&self, value: T) -> DecisionFuture {
        DecisionFuture::new(self.id, &MergeDecision::<T>::First(value))
    }

    /// Scripts a top-level `merge_ordered` action to release the front element of the
    /// *second* input's buffer, which must equal `value`; see [`Self::next_first`].
    pub fn next_second(&self, value: T) -> DecisionFuture {
        DecisionFuture::new(self.id, &MergeDecision::<T>::Second(value))
    }

    /// Scripts a top-level `merge_ordered` action to release the front element of the
    /// *first* input's buffer, whatever it is (waiting for one to arrive if that input is
    /// empty). Unlike [`Self::next_first`], this does not assert the released value; use
    /// `next_first(value)` to name it exactly and fail loudly on mis-synchronization.
    pub fn advance_first(&self) -> DecisionFuture {
        DecisionFuture::new(self.id, &MergeDecision::<T>::FirstNext(()))
    }

    /// Scripts a top-level `merge_ordered` action to release the front element of the
    /// *second* input's buffer, whatever it is; see [`Self::advance_first`].
    pub fn advance_second(&self) -> DecisionFuture {
        DecisionFuture::new(self.id, &MergeDecision::<T>::SecondNext(()))
    }

    pause_family!(MergeStatus);

    /// Pauses a top-level merge hook until at least `n` elements are buffered (in total,
    /// across both inputs); see [`Self::pause_until`].
    pub fn pause_until_count(
        &self,
        n: usize,
    ) -> PauseUntilFuture<MergeStatus, impl Fn(&MergeStatus) -> bool + Unpin> {
        self.pause_until_labeled(format!("pause_until_count({})", n), move |status| {
            status.first_buffered + status.second_buffered >= n
        })
    }
}

impl<T> MergeOrderedHook<T, Bounded>
where
    T: Serialize + DeserializeOwned,
{
    /// Scripts an in-tick `merge_ordered` observation to consume its complete input in
    /// exactly this interleaving, with each value labeled by the input it is drawn from
    /// (`false` = first/left, `true` = second/right). Each input's labeled values must be
    /// exactly that input's tick-local batch, in order.
    pub fn order(&self, values: impl IntoIterator<Item = (bool, T)>) -> DecisionFuture {
        DecisionFuture::new(
            self.id,
            &InlineOrderingDecision::Order(values.into_iter().collect()),
        )
    }
}

impl<K, V> KeyedMergeOrderedHook<K, V, Unbounded>
where
    K: Serialize + DeserializeOwned,
    V: Serialize + DeserializeOwned,
{
    /// Scripts a top-level keyed `merge_ordered` action to release the front entry of
    /// `key`'s buffer in the *first* input, which must equal `value` (per-input
    /// within-key order is preserved, so a mismatch panics). Exactly one entry is
    /// released, preserving opportunities for ticks and feedback to interleave with the
    /// remaining buffered input.
    pub fn next_first(&self, key: K, value: V) -> DecisionFuture {
        DecisionFuture::new(self.id, &MergeDecision::<(K, V), K>::First((key, value)))
    }

    /// Scripts a top-level keyed `merge_ordered` action to release the front entry of
    /// `key`'s buffer in the *second* input, which must equal `value`; see
    /// [`Self::next_first`].
    pub fn next_second(&self, key: K, value: V) -> DecisionFuture {
        DecisionFuture::new(self.id, &MergeDecision::<(K, V), K>::Second((key, value)))
    }

    /// Scripts a top-level keyed `merge_ordered` action to release the front entry of
    /// `key`'s buffer in the *first* input, whatever its value (waiting for one to arrive
    /// if that key's buffer is empty). Unlike [`Self::next_first`], this does not assert
    /// the released value; use `next_first(key, value)` to name it exactly and fail
    /// loudly on mis-synchronization.
    pub fn advance_first(&self, key: K) -> DecisionFuture {
        DecisionFuture::new(self.id, &MergeDecision::<(K, V), K>::FirstNext(key))
    }

    /// Scripts a top-level keyed `merge_ordered` action to release the front entry of
    /// `key`'s buffer in the *second* input, whatever its value; see
    /// [`Self::advance_first`].
    pub fn advance_second(&self, key: K) -> DecisionFuture {
        DecisionFuture::new(self.id, &MergeDecision::<(K, V), K>::SecondNext(key))
    }

    pause_family!(MergeStatus);

    /// Pauses a top-level keyed merge hook until at least `n` entries are buffered (in
    /// total, across both inputs and all keys); see [`Self::pause_until`].
    pub fn pause_until_count(
        &self,
        n: usize,
    ) -> PauseUntilFuture<MergeStatus, impl Fn(&MergeStatus) -> bool + Unpin> {
        self.pause_until_labeled(format!("pause_until_count({})", n), move |status| {
            status.first_buffered + status.second_buffered >= n
        })
    }
}

impl<K, V> KeyedMergeOrderedHook<K, V, Bounded>
where
    K: Serialize + DeserializeOwned,
    V: Serialize + DeserializeOwned,
{
    /// Scripts an in-tick keyed `merge_ordered` observation to consume its complete input
    /// in exactly this interleaving, with each `(key, value)` entry labeled by the input
    /// it is drawn from (`false` = first/left, `true` = second/right). Each input's
    /// labeled entries must be exactly that input's tick-local batch, with every key's
    /// values in order; the relative order of *different* keys is irrelevant (a keyed
    /// stream carries no cross-key ordering).
    pub fn order(&self, entries: impl IntoIterator<Item = (bool, K, V)>) -> DecisionFuture {
        DecisionFuture::new(
            self.id,
            &InlineOrderingDecision::Order(
                entries
                    .into_iter()
                    .map(|(from_second, key, value)| (from_second, (key, value)))
                    .collect(),
            ),
        )
    }
}
