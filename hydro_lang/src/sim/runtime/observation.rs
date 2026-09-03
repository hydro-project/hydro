//! Top-level observation hooks ([`ObservationHook`]): hooks with no tick DFIR that are
//! their own scheduling unit — running one *is* releasing data (e.g. `assume_ordering`
//! on a non-tick stream, or a hooked top-level `fold`). Each is its own
//! `SimObservation` in the scheduler. Their scripted decision/status types and
//! [`ScriptableHook`] impls live alongside them.
//!
//! The retry kinds ([`TopLevelStreamRetriesHook`], [`TopLevelOrderedStreamRetriesHook`],
//! [`TopLevelAtLeastOnceOrderHook`]) implement only the scriptable surface, never
//! [`ObservationHook`]: their decision spaces are infinite (every element admits
//! arbitrarily many retries), so no autonomous exploration is possible and the builder
//! requires them to be bound to a sim hook.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::hash::Hash;
use std::rc::Rc;

use bolero::generator::bolero_generator::driver::object::Borrowed;
use bolero::{ValueGenerator, produce};
use dfir_rs::rustc_hash::FxHashMap;
use dfir_rs::util::unsync::mpsc::Sender;

use super::{
    HookLocationMeta, ObservationHook, RuntimeHook, ScriptDecision, ScriptableHook,
    ScriptableObservationHook, TruncatedLabeledVecDebug, TruncatedVecDebug, abort,
    describe_keyed_pending, keyed_buffer_len, log_release,
};

/// Top-level (outside-tick) `assume_ordering` hooks release elements **one at a
/// time** rather than shuffling the entire batch. This is the key mechanism for
/// simulating causality in feedback cycles: when data flows through a network hop
/// and cycles back (e.g. via `forward_ref`), the cycled-back result can arrive
/// and interleave with elements that are still pending in the input queue.
///
/// For example, given input `[1, 2, 3]` where each element is mapped and sent
/// through a network cycle, the simulator can explore orderings like
/// `[1, map(1), 2, map(2), 3, ...]` -- the cycled-back `map(1)` arrives before `2`.
///
/// This is the only place where such causality is observable, because top-level
/// unbounded streams are "maximally async" -- any non-atomic stream can be
/// arbitrarily decoupled. The in-tick variants (`StreamOrderHook`,
/// `KeyedStreamOrderHook`) shuffle the full batch instead, since within a tick
/// all data is available simultaneously.
///
/// The `sim_top_level_assume_ordering_*` tests are regression tests ensuring
/// this one-at-a-time release behavior correctly explores causal interleavings.
pub struct TopLevelStreamOrderHook<T> {
    pub input: Rc<RefCell<VecDeque<T>>>,
    pub to_release: Option<Vec<T>>,
    pub output: Sender<T>,
    pub location: HookLocationMeta,
    pub format_item_debug: fn(&T) -> Option<String>,
}

impl<T> RuntimeHook for TopLevelStreamOrderHook<T> {
    fn has_pending_input(&self) -> bool {
        !self.input.borrow().is_empty()
    }

    fn only_one_possible_decision(&self) -> bool {
        // A sole buffered element has exactly one possible release; ordering only
        // becomes a choice with two or more.
        self.input.borrow().len() <= 1
    }

    fn release_decision(&mut self, log_writer: Option<&mut dyn std::fmt::Write>) {
        if let Some(to_release) = self.to_release.take() {
            if !to_release.is_empty()
                && let Some(log_writer) = log_writer
            {
                let HookLocationMeta {
                    location: batch_location,
                    line,
                    caret_indent,
                } = self.location;
                let note_str = format!(
                    "^ observed non-deterministic order: {:?}",
                    TruncatedVecDebug(
                        RefCell::new(Some(to_release.iter())),
                        8,
                        self.format_item_debug
                    )
                );

                let _ = writeln!(log_writer);
                log_release(
                    log_writer,
                    batch_location,
                    line,
                    caret_indent,
                    &note_str,
                    colored::Color::Green,
                );
            }

            for item in to_release {
                self.output.try_send(item).unwrap();
            }
        } else {
            panic!("No decision to release");
        }
    }

    fn location_meta(&self) -> HookLocationMeta {
        self.location
    }
}

impl<T> ObservationHook for TopLevelStreamOrderHook<T> {
    fn autonomous_decision<'a>(&mut self, driver: &mut Borrowed<'a>) {
        let mut current_input = self.input.borrow_mut();
        // Instead of a full shuffle, we only release one element at a time
        // in order to handle possible feedback cycles.
        let idx = (0..current_input.len()).generate(driver).unwrap();
        let item = current_input.remove(idx).unwrap();
        self.to_release = Some(vec![item]);
    }
}

/// A scripted decision for a top-level `assume_ordering` observation.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum TopLevelOrderingDecision<T> {
    Next(T),
}

impl<T> ScriptDecision for TopLevelOrderingDecision<T>
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    fn describe(&self) -> String {
        "next(value)".to_owned()
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct OrderingStatus {
    pub buffered: usize,
}

impl<T> ScriptableHook for TopLevelStreamOrderHook<T>
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq,
{
    type Decision = TopLevelOrderingDecision<T>;
    type Status = OrderingStatus;

    fn is_honorable(&self, decision: &Self::Decision) -> Result<bool, String> {
        let TopLevelOrderingDecision::Next(expected) = decision;
        Ok(self.input.borrow().iter().any(|item| item == expected))
    }

    fn apply(&mut self, decision: Self::Decision) {
        let mut input = self.input.borrow_mut();
        let TopLevelOrderingDecision::Next(expected) = decision;
        let index = input.iter().position(|item| item == &expected).unwrap();
        self.to_release = Some(vec![input.remove(index).unwrap()]);
    }

    fn implicit(&mut self) {
        // Implicit behavior exists for tick hooks whose tick is forced to run by *other*
        // hooks; a top-level observation consists of exactly this hook, so it can never
        // be forced to run without a scripted decision.
        abort!("implicit decision invoked on a top-level ordering hook");
    }

    fn status(&self) -> Self::Status {
        OrderingStatus {
            buffered: self.input.borrow().len(),
        }
    }

    fn describe_pending(&self) -> Option<String> {
        let input = self.input.borrow();
        (!input.is_empty()).then(|| {
            format!(
                "{} buffered ordering item(s): {:?}",
                input.len(),
                TruncatedVecDebug(RefCell::new(Some(input.iter())), 8, self.format_item_debug)
            )
        })
    }
}

impl<T> ScriptableObservationHook for TopLevelStreamOrderHook<T> where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq
{
}

/// Hook for top-level folds. Selects a non-empty subset of buffered inputs to release,
/// always permuting them to explore all orderings. Unselected elements remain
/// in the buffer for future releases (modeling delayed/lossy inputs).
pub struct TopLevelFoldHook<T> {
    pub input: Rc<RefCell<VecDeque<T>>>,
    pub to_release: Option<Vec<T>>,
    pub output: Sender<Vec<T>>,
    pub location: HookLocationMeta,
    pub format_item_debug: fn(&T) -> Option<String>,
}

impl<T> RuntimeHook for TopLevelFoldHook<T> {
    fn has_pending_input(&self) -> bool {
        !self.input.borrow().is_empty()
    }

    fn only_one_possible_decision(&self) -> bool {
        // The subset must be non-empty, so a sole buffered element is forced; subset
        // choice and permutation only appear with two or more.
        self.input.borrow().len() <= 1
    }

    fn release_decision(&mut self, log_writer: Option<&mut dyn std::fmt::Write>) {
        if let Some(to_release) = self.to_release.take() {
            if !to_release.is_empty()
                && let Some(log_writer) = log_writer
            {
                let HookLocationMeta {
                    location: batch_location,
                    line,
                    caret_indent,
                } = self.location;
                let note_str = format!(
                    "^ fold input batch (permuted): {:?}",
                    TruncatedVecDebug(
                        RefCell::new(Some(to_release.iter())),
                        8,
                        self.format_item_debug
                    )
                );

                let _ = writeln!(log_writer);
                log_release(
                    log_writer,
                    batch_location,
                    line,
                    caret_indent,
                    &note_str,
                    colored::Color::Green,
                );
            }

            self.output.try_send(to_release).unwrap();
        } else {
            panic!("No decision to release");
        }
    }

    fn location_meta(&self) -> HookLocationMeta {
        self.location
    }
}

impl<T> ObservationHook for TopLevelFoldHook<T> {
    fn autonomous_decision<'a>(&mut self, driver: &mut Borrowed<'a>) {
        let mut current_input = self.input.borrow_mut();

        // Select a non-empty subset: for each element, decide include/exclude.
        // Only force inclusion on the last element if nothing was selected yet.
        let mut selected = Vec::new();
        let mut remaining = VecDeque::new();

        let len = current_input.len();
        for (i, item) in current_input.drain(..).enumerate() {
            let is_last = i == len - 1;
            let must_include = is_last && selected.is_empty();
            if must_include || produce().generate(driver).unwrap() {
                selected.push(item);
            } else {
                remaining.push_back(item);
            }
        }

        // Put unselected elements back
        *current_input = remaining;

        // Always permute selected elements (Fisher-Yates) to explore all orderings.
        // Even if commutativity is claimed via manual_proof!, the simulator is
        // conservative and does not trust it — it still explores permutations.
        {
            let slen = selected.len();
            for i in (1..slen).rev() {
                let j = (0..=i).generate(driver).unwrap();
                selected.swap(i, j);
            }
        }

        self.to_release = Some(selected);
    }
}

/// Scripting a top-level fold releases exactly **one named element per decision**
/// (like a top-level `assume_ordering`), so intermediate fold states become observable
/// exactly at the script's release points. The autonomous subset-and-permute path is
/// never used: it is only sound when the fuzzer explores every subset split.
impl<T> ScriptableHook for TopLevelFoldHook<T>
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq,
{
    type Decision = TopLevelOrderingDecision<T>;
    type Status = OrderingStatus;

    fn is_honorable(&self, decision: &Self::Decision) -> Result<bool, String> {
        let TopLevelOrderingDecision::Next(expected) = decision;
        Ok(self.input.borrow().iter().any(|item| item == expected))
    }

    fn apply(&mut self, decision: Self::Decision) {
        let TopLevelOrderingDecision::Next(expected) = decision;
        let mut input = self.input.borrow_mut();
        let index = input.iter().position(|item| item == &expected).unwrap();
        self.to_release = Some(vec![input.remove(index).unwrap()]);
    }

    fn implicit(&mut self) {
        // Implicit behavior exists for tick hooks whose tick is forced to run by *other*
        // hooks; a top-level observation consists of exactly this hook, so it can never
        // be forced to run without a scripted decision.
        abort!("implicit decision invoked on a top-level fold hook");
    }

    fn status(&self) -> Self::Status {
        OrderingStatus {
            buffered: self.input.borrow().len(),
        }
    }

    fn describe_pending(&self) -> Option<String> {
        let input = self.input.borrow();
        (!input.is_empty()).then(|| {
            format!(
                "{} buffered fold input(s): {:?}",
                input.len(),
                TruncatedVecDebug(RefCell::new(Some(input.iter())), 8, self.format_item_debug)
            )
        })
    }
}

impl<T> ScriptableObservationHook for TopLevelFoldHook<T> where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq
{
}

/// Keyed variant of [`TopLevelStreamOrderHook`]. Same one-at-a-time release
/// strategy to simulate causal interleavings -- see the comment on
/// [`TopLevelStreamOrderHook`] for the full explanation.
pub struct TopLevelKeyedStreamOrderHook<K: Hash + Eq + Clone, V> {
    pub input: Rc<RefCell<FxHashMap<K, VecDeque<V>>>>,
    pub to_release: Option<Vec<(K, V)>>,
    pub output: Sender<(K, V)>,
    pub location: HookLocationMeta,
    pub format_item_debug: fn(&(K, V)) -> Option<String>,
}

impl<K: Hash + Eq + Clone, V> RuntimeHook for TopLevelKeyedStreamOrderHook<K, V> {
    fn has_pending_input(&self) -> bool {
        #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
        !self.input.borrow().values().all(|q| q.is_empty())
    }

    fn only_one_possible_decision(&self) -> bool {
        // A sole buffered element (across all keys) has exactly one possible release.
        #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
        let total: usize = self.input.borrow().values().map(|q| q.len()).sum();
        total <= 1
    }

    fn release_decision(&mut self, log_writer: Option<&mut dyn std::fmt::Write>) {
        if let Some(to_release) = self.to_release.take() {
            if !to_release.is_empty()
                && let Some(log_writer) = log_writer
            {
                let HookLocationMeta {
                    location: batch_location,
                    line,
                    caret_indent,
                } = self.location;
                let note_str = format!(
                    "^ observed non-deterministic order: {:?}",
                    TruncatedVecDebug(
                        RefCell::new(Some(to_release.iter())),
                        8,
                        self.format_item_debug
                    )
                );

                let _ = writeln!(log_writer);
                log_release(
                    log_writer,
                    batch_location,
                    line,
                    caret_indent,
                    &note_str,
                    colored::Color::Green,
                );
            }

            for item in to_release {
                self.output.try_send(item).unwrap();
            }
        } else {
            panic!("No decision to release");
        }
    }

    fn location_meta(&self) -> HookLocationMeta {
        self.location
    }
}

impl<K: Hash + Eq + Clone, V> ObservationHook for TopLevelKeyedStreamOrderHook<K, V> {
    fn autonomous_decision<'a>(&mut self, driver: &mut Borrowed<'a>) {
        let mut current_input = self.input.borrow_mut();

        // Collect non-empty keys with their queue lengths
        #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
        let nonempty_keys: Vec<(K, usize)> = current_input
            .iter()
            .filter(|(_, q)| !q.is_empty())
            .map(|(k, q)| (k.clone(), q.len()))
            .collect();

        // Pick which key to release from
        let key_idx = (0..nonempty_keys.len()).generate(driver).unwrap();
        let (key, queue_len) = &nonempty_keys[key_idx];

        // Pick which item from that key's queue
        let item_idx = (0..*queue_len).generate(driver).unwrap();
        let item = current_input
            .get_mut(key)
            .unwrap()
            .remove(item_idx)
            .unwrap();

        self.to_release = Some(vec![(key.clone(), item)]);
    }
}

/// Top-level variant of [`PartiallyOrderedStreamHook`](super::PartiallyOrderedStreamHook). Same one-at-a-time release
/// strategy as [`TopLevelKeyedStreamOrderHook`], but always takes from the FRONT
/// of the chosen key's queue to preserve within-key order.
pub struct TopLevelPartiallyOrderedStreamHook<K: Hash + Eq + Clone, V> {
    pub input: Rc<RefCell<FxHashMap<K, VecDeque<V>>>>,
    pub to_release: Option<Vec<(K, V)>>,
    pub output: Sender<(K, V)>,
    pub location: HookLocationMeta,
    pub format_item_debug: fn(&(K, V)) -> Option<String>,
}

impl<K: Hash + Eq + Clone, V> RuntimeHook for TopLevelPartiallyOrderedStreamHook<K, V> {
    fn has_pending_input(&self) -> bool {
        #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
        !self.input.borrow().values().all(|q| q.is_empty())
    }

    fn only_one_possible_decision(&self) -> bool {
        // Within a key the front element is forced, so the only choice is which key
        // releases next: a single non-empty key is fully forced regardless of depth.
        #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
        let nonempty_keys = self
            .input
            .borrow()
            .values()
            .filter(|q| !q.is_empty())
            .count();
        nonempty_keys <= 1
    }

    fn release_decision(&mut self, log_writer: Option<&mut dyn std::fmt::Write>) {
        if let Some(to_release) = self.to_release.take() {
            if !to_release.is_empty()
                && let Some(log_writer) = log_writer
            {
                let HookLocationMeta {
                    location: batch_location,
                    line,
                    caret_indent,
                } = self.location;
                let note_str = format!(
                    "^ observed partially-ordered interleaving: {:?}",
                    TruncatedVecDebug(
                        RefCell::new(Some(to_release.iter())),
                        8,
                        self.format_item_debug
                    )
                );

                let _ = writeln!(log_writer);
                log_release(
                    log_writer,
                    batch_location,
                    line,
                    caret_indent,
                    &note_str,
                    colored::Color::Green,
                );
            }

            for item in to_release {
                self.output.try_send(item).unwrap();
            }
        } else {
            panic!("No decision to release");
        }
    }

    fn location_meta(&self) -> HookLocationMeta {
        self.location
    }
}

impl<K: Hash + Eq + Clone, V> ObservationHook for TopLevelPartiallyOrderedStreamHook<K, V> {
    fn autonomous_decision<'a>(&mut self, driver: &mut Borrowed<'a>) {
        let mut current_input = self.input.borrow_mut();

        #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
        let nonempty_keys: Vec<K> = current_input
            .iter()
            .filter(|(_, q)| !q.is_empty())
            .map(|(k, _)| k.clone())
            .collect();

        // Pick which key to release from
        let key_idx = (0..nonempty_keys.len()).generate(driver).unwrap();
        let key = &nonempty_keys[key_idx];

        // Always take from the front to preserve within-key order
        let item = current_input.get_mut(key).unwrap().pop_front().unwrap();

        self.to_release = Some(vec![(key.clone(), item)]);
    }
}

/// Top-level merge-ordered hook. Releases one element at a time, picking from
/// the front of either the first or second input queue. This preserves per-input
/// order while allowing feedback cycles to deliver elements between releases.
pub struct TopLevelMergeOrderedHook<T> {
    pub first: Rc<RefCell<VecDeque<T>>>,
    pub second: Rc<RefCell<VecDeque<T>>>,
    pub to_release: Option<Vec<T>>,
    pub release_source: Option<&'static str>,
    pub output: Sender<T>,
    pub location: HookLocationMeta,
    pub format_item_debug: fn(&T) -> Option<String>,
}

impl<T> RuntimeHook for TopLevelMergeOrderedHook<T> {
    fn has_pending_input(&self) -> bool {
        !self.first.borrow().is_empty() || !self.second.borrow().is_empty()
    }

    fn only_one_possible_decision(&self) -> bool {
        // Each side's front element is forced, so the only choice is which side
        // releases next: it exists only when both sides are non-empty.
        self.first.borrow().is_empty() || self.second.borrow().is_empty()
    }

    fn release_decision(&mut self, log_writer: Option<&mut dyn std::fmt::Write>) {
        if let Some(to_release) = self.to_release.take() {
            let source = self.release_source.take();
            if !to_release.is_empty()
                && let Some(log_writer) = log_writer
            {
                let HookLocationMeta {
                    location: batch_location,
                    line,
                    caret_indent,
                } = self.location;
                let source_label = source.unwrap_or("?");

                let labeled_iter = to_release.iter().map(|item| (source_label, item));

                let note_str = format!(
                    "^ observed non-deterministic merge order: {:?}",
                    TruncatedLabeledVecDebug(
                        RefCell::new(Some(labeled_iter)),
                        8,
                        self.format_item_debug
                    )
                );

                let _ = writeln!(log_writer);
                log_release(
                    log_writer,
                    batch_location,
                    line,
                    caret_indent,
                    &note_str,
                    colored::Color::Green,
                );
            }

            for item in to_release {
                self.output.try_send(item).unwrap();
            }
        } else {
            panic!("No decision to release");
        }
    }

    fn location_meta(&self) -> HookLocationMeta {
        self.location
    }
}

impl<T> ObservationHook for TopLevelMergeOrderedHook<T> {
    fn autonomous_decision<'a>(&mut self, driver: &mut Borrowed<'a>) {
        let first_empty = self.first.borrow().is_empty();
        let second_empty = self.second.borrow().is_empty();

        let (item, source) = if first_empty {
            (self.second.borrow_mut().pop_front().unwrap(), "r")
        } else if second_empty {
            (self.first.borrow_mut().pop_front().unwrap(), "l")
        } else {
            let take_second: bool = produce().generate(driver).unwrap();
            if take_second {
                (self.second.borrow_mut().pop_front().unwrap(), "r")
            } else {
                (self.first.borrow_mut().pop_front().unwrap(), "l")
            }
        };

        self.to_release = Some(vec![item]);
        self.release_source = Some(source);
    }
}

/// Keyed variant of [`TopLevelMergeOrderedHook`]. Releases one element at a
/// time, picking from the front of some key's queue in either the first or the
/// second input. This preserves per-input order within each key while allowing
/// arbitrary interleaving both across the two inputs and across keys (which is
/// unconstrained for keyed streams), and lets feedback cycles deliver elements
/// between releases.
pub struct TopLevelKeyedMergeOrderedHook<K: Hash + Eq + Clone, V> {
    pub first: Rc<RefCell<FxHashMap<K, VecDeque<V>>>>,
    pub second: Rc<RefCell<FxHashMap<K, VecDeque<V>>>>,
    pub to_release: Option<Vec<(K, V)>>,
    pub release_source: Option<&'static str>,
    pub output: Sender<(K, V)>,
    pub location: HookLocationMeta,
    pub format_item_debug: fn(&(K, V)) -> Option<String>,
}

impl<K: Hash + Eq + Clone, V> RuntimeHook for TopLevelKeyedMergeOrderedHook<K, V> {
    fn has_pending_input(&self) -> bool {
        #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
        let first_nonempty = !self.first.borrow().values().all(|q| q.is_empty());
        #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
        let second_nonempty = !self.second.borrow().values().all(|q| q.is_empty());
        first_nonempty || second_nonempty
    }

    fn only_one_possible_decision(&self) -> bool {
        // Each (side, key) queue's front element is forced, so the only choice is which
        // queue releases next.
        #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
        let first_count = self
            .first
            .borrow()
            .values()
            .filter(|q| !q.is_empty())
            .count();
        #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
        let second_count = self
            .second
            .borrow()
            .values()
            .filter(|q| !q.is_empty())
            .count();
        first_count + second_count <= 1
    }

    fn release_decision(&mut self, log_writer: Option<&mut dyn std::fmt::Write>) {
        if let Some(to_release) = self.to_release.take() {
            let source = self.release_source.take();
            if !to_release.is_empty()
                && let Some(log_writer) = log_writer
            {
                let HookLocationMeta {
                    location: batch_location,
                    line,
                    caret_indent,
                } = self.location;
                let source_label = source.unwrap_or("?");

                let labeled_iter = to_release.iter().map(|item| (source_label, item));

                let note_str = format!(
                    "^ observed non-deterministic merge order: {:?}",
                    TruncatedLabeledVecDebug(
                        RefCell::new(Some(labeled_iter)),
                        8,
                        self.format_item_debug
                    )
                );

                let _ = writeln!(log_writer);
                log_release(
                    log_writer,
                    batch_location,
                    line,
                    caret_indent,
                    &note_str,
                    colored::Color::Green,
                );
            }

            for item in to_release {
                self.output.try_send(item).unwrap();
            }
        } else {
            panic!("No decision to release");
        }
    }

    fn location_meta(&self) -> HookLocationMeta {
        self.location
    }
}

impl<K: Hash + Eq + Clone, V> ObservationHook for TopLevelKeyedMergeOrderedHook<K, V> {
    fn autonomous_decision<'a>(&mut self, driver: &mut Borrowed<'a>) {
        // Collect candidates: for each non-empty key queue in either input, we
        // can release its front element. `false` = first input, `true` = second.
        let mut candidates: Vec<(bool, K)> = Vec::new();
        {
            #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
            for (k, q) in self.first.borrow().iter() {
                if !q.is_empty() {
                    candidates.push((false, k.clone()));
                }
            }
            #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
            for (k, q) in self.second.borrow().iter() {
                if !q.is_empty() {
                    candidates.push((true, k.clone()));
                }
            }
        }

        let idx = (0..candidates.len()).generate(driver).unwrap();
        let (take_second, key) = &candidates[idx];
        let take_second = *take_second;

        let item = if take_second {
            self.second
                .borrow_mut()
                .get_mut(key)
                .unwrap()
                .pop_front()
                .unwrap()
        } else {
            self.first
                .borrow_mut()
                .get_mut(key)
                .unwrap()
                .pop_front()
                .unwrap()
        };

        self.to_release = Some(vec![(key.clone(), item)]);
        self.release_source = Some(if take_second { "r" } else { "l" });
    }
}

/// Keyed variant of [`TopLevelStreamOrderHook`]'s scripting: a top-level keyed
/// `assume_ordering` decision names one `(key, value)` entry to release; values within a
/// key may be matched at any buffered position (the input is unordered within each key).
impl<K, V> ScriptableHook for TopLevelKeyedStreamOrderHook<K, V>
where
    K: Hash + Eq + Clone + serde::Serialize + serde::de::DeserializeOwned,
    V: serde::Serialize + serde::de::DeserializeOwned + PartialEq,
{
    type Decision = TopLevelOrderingDecision<(K, V)>;
    type Status = OrderingStatus;

    fn is_honorable(&self, decision: &Self::Decision) -> Result<bool, String> {
        let TopLevelOrderingDecision::Next((key, expected)) = decision;
        Ok(self
            .input
            .borrow()
            .get(key)
            .is_some_and(|queue| queue.iter().any(|item| item == expected)))
    }

    fn apply(&mut self, decision: Self::Decision) {
        let TopLevelOrderingDecision::Next((key, expected)) = decision;
        let mut input = self.input.borrow_mut();
        let queue = input.get_mut(&key).unwrap();
        let index = queue.iter().position(|item| item == &expected).unwrap();
        let item = queue.remove(index).unwrap();
        self.to_release = Some(vec![(key, item)]);
    }

    fn implicit(&mut self) {
        // Implicit behavior exists for tick hooks whose tick is forced to run by *other*
        // hooks; a top-level observation consists of exactly this hook, so it can never
        // be forced to run without a scripted decision.
        abort!("implicit decision invoked on a top-level keyed ordering hook");
    }

    fn status(&self) -> Self::Status {
        OrderingStatus {
            buffered: keyed_buffer_len(&self.input.borrow()),
        }
    }

    fn describe_pending(&self) -> Option<String> {
        describe_keyed_pending(&self.input.borrow())
    }
}

impl<K, V> ScriptableObservationHook for TopLevelKeyedStreamOrderHook<K, V>
where
    K: Hash + Eq + Clone + serde::Serialize + serde::de::DeserializeOwned,
    V: serde::Serialize + serde::de::DeserializeOwned + PartialEq,
{
}

/// Scripting for a top-level `entries_partially_ordered`: each decision names one
/// `(key, value)` entry to release, and the value must be the *front* of that key's
/// buffered queue since within-key order is preserved.
impl<K, V> ScriptableHook for TopLevelPartiallyOrderedStreamHook<K, V>
where
    K: Hash + Eq + Clone + serde::Serialize + serde::de::DeserializeOwned,
    V: serde::Serialize + serde::de::DeserializeOwned + PartialEq,
{
    type Decision = TopLevelOrderingDecision<(K, V)>;
    type Status = OrderingStatus;

    fn is_honorable(&self, decision: &Self::Decision) -> Result<bool, String> {
        let TopLevelOrderingDecision::Next((key, expected)) = decision;
        match self.input.borrow().get(key).and_then(VecDeque::front) {
            Some(front) if front == expected => Ok(true),
            // Within-key order is preserved, so a mismatching front can never be
            // released ahead of the named value: the decision is permanently stuck.
            Some(_) => Err(
                "next: the named value did not match the front of its key's buffered queue \
                 (within-key order is preserved by this operator)"
                    .to_owned(),
            ),
            None => Ok(false),
        }
    }

    fn apply(&mut self, decision: Self::Decision) {
        let TopLevelOrderingDecision::Next((key, _expected)) = decision;
        let mut input = self.input.borrow_mut();
        let item = input.get_mut(&key).unwrap().pop_front().unwrap();
        self.to_release = Some(vec![(key, item)]);
    }

    fn implicit(&mut self) {
        // Implicit behavior exists for tick hooks whose tick is forced to run by *other*
        // hooks; a top-level observation consists of exactly this hook, so it can never
        // be forced to run without a scripted decision.
        abort!("implicit decision invoked on a top-level partially-ordered hook");
    }

    fn status(&self) -> Self::Status {
        OrderingStatus {
            buffered: keyed_buffer_len(&self.input.borrow()),
        }
    }

    fn describe_pending(&self) -> Option<String> {
        describe_keyed_pending(&self.input.borrow())
    }
}

impl<K, V> ScriptableObservationHook for TopLevelPartiallyOrderedStreamHook<K, V>
where
    K: Hash + Eq + Clone + serde::Serialize + serde::de::DeserializeOwned,
    V: serde::Serialize + serde::de::DeserializeOwned + PartialEq,
{
}

/// A scripted decision for a top-level `merge_ordered` observation: which input the next
/// released element comes from. The element may be named ([`Self::First`] /
/// [`Self::Second`], asserting it equals the front of that input's buffer) or released
/// positionally ([`Self::FirstNext`] / [`Self::SecondNext`], releasing the front without
/// asserting its value). `S` selects *which* front for keyed merges (the key), and is `()`
/// for plain streams.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum MergeDecision<T, S = ()> {
    /// Release the front of the first (left) input's buffer, which must equal this value.
    First(T),
    /// Release the front of the second (right) input's buffer, which must equal this value.
    Second(T),
    /// Release the front of the first (left) input's buffer, whatever it is.
    FirstNext(S),
    /// Release the front of the second (right) input's buffer, whatever it is.
    SecondNext(S),
}

impl<T, S> ScriptDecision for MergeDecision<T, S>
where
    T: serde::Serialize + serde::de::DeserializeOwned,
    S: serde::Serialize + serde::de::DeserializeOwned,
{
    fn describe(&self) -> String {
        match self {
            MergeDecision::First(_) => "next_first(value)".to_owned(),
            MergeDecision::Second(_) => "next_second(value)".to_owned(),
            MergeDecision::FirstNext(_) => "advance_first(..)".to_owned(),
            MergeDecision::SecondNext(_) => "advance_second(..)".to_owned(),
        }
    }
}

/// The pending-input view a top-level merge hook reports to its test-side handle (see
/// [`ScriptableHook::status`]), used by `pause_until_*` predicates.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct MergeStatus {
    /// The number of elements buffered from the first (left) input.
    pub first_buffered: usize,
    /// The number of elements buffered from the second (right) input.
    pub second_buffered: usize,
}

impl<T> ScriptableHook for TopLevelMergeOrderedHook<T>
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq,
{
    type Decision = MergeDecision<T>;
    type Status = MergeStatus;

    fn is_honorable(&self, decision: &MergeDecision<T>) -> Result<bool, String> {
        let (buffer, expected) = match decision {
            MergeDecision::First(expected) => (&self.first, Some(expected)),
            MergeDecision::Second(expected) => (&self.second, Some(expected)),
            MergeDecision::FirstNext(()) => (&self.first, None),
            MergeDecision::SecondNext(()) => (&self.second, None),
        };
        match (buffer.borrow().front(), expected) {
            (Some(_), None) => Ok(true),
            (Some(front), Some(expected)) if front == expected => Ok(true),
            // Per-input order is preserved, so a mismatching front can never be released
            // ahead of the named value: the decision is permanently stuck.
            (Some(_), Some(_)) => Err(
                "next_first/next_second: the named value did not match the front of that \
                 input's buffer (per-input order is preserved by merge_ordered)"
                    .to_owned(),
            ),
            (None, _) => Ok(false),
        }
    }

    fn apply(&mut self, decision: MergeDecision<T>) {
        let (item, source) = match decision {
            MergeDecision::First(_) | MergeDecision::FirstNext(()) => {
                (self.first.borrow_mut().pop_front().unwrap(), "l")
            }
            MergeDecision::Second(_) | MergeDecision::SecondNext(()) => {
                (self.second.borrow_mut().pop_front().unwrap(), "r")
            }
        };
        self.to_release = Some(vec![item]);
        self.release_source = Some(source);
    }

    fn implicit(&mut self) {
        // Implicit behavior exists for tick hooks whose tick is forced to run by *other*
        // hooks; a top-level observation consists of exactly this hook, so it can never
        // be forced to run without a scripted decision.
        abort!("implicit decision invoked on a top-level merge-ordered hook");
    }

    fn status(&self) -> MergeStatus {
        MergeStatus {
            first_buffered: self.first.borrow().len(),
            second_buffered: self.second.borrow().len(),
        }
    }

    fn describe_pending(&self) -> Option<String> {
        let first = self.first.borrow();
        let second = self.second.borrow();
        (!first.is_empty() || !second.is_empty()).then(|| {
            format!(
                "{} + {} buffered item(s) (first + second input)",
                first.len(),
                second.len()
            )
        })
    }
}

impl<T> ScriptableObservationHook for TopLevelMergeOrderedHook<T> where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq
{
}

impl<K, V> ScriptableHook for TopLevelKeyedMergeOrderedHook<K, V>
where
    K: Hash + Eq + Clone + serde::Serialize + serde::de::DeserializeOwned,
    V: serde::Serialize + serde::de::DeserializeOwned + PartialEq,
{
    type Decision = MergeDecision<(K, V), K>;
    type Status = MergeStatus;

    fn is_honorable(&self, decision: &MergeDecision<(K, V), K>) -> Result<bool, String> {
        let (buffer, key, expected) = match decision {
            MergeDecision::First((key, expected)) => (&self.first, key, Some(expected)),
            MergeDecision::Second((key, expected)) => (&self.second, key, Some(expected)),
            MergeDecision::FirstNext(key) => (&self.first, key, None),
            MergeDecision::SecondNext(key) => (&self.second, key, None),
        };
        match (buffer.borrow().get(key).and_then(VecDeque::front), expected) {
            (Some(_), None) => Ok(true),
            (Some(front), Some(expected)) if front == expected => Ok(true),
            // Per-input order within a key is preserved, so a mismatching front can
            // never be released ahead of the named value: the decision is permanently
            // stuck.
            (Some(_), Some(_)) => Err(
                "next_first/next_second: the named value did not match the front of its \
                 key's buffer in that input (per-input within-key order is preserved by \
                 merge_ordered)"
                    .to_owned(),
            ),
            (None, _) => Ok(false),
        }
    }

    fn apply(&mut self, decision: MergeDecision<(K, V), K>) {
        let (buffer, key, source) = match decision {
            MergeDecision::First((key, _)) => (&self.first, key, "l"),
            MergeDecision::Second((key, _)) => (&self.second, key, "r"),
            MergeDecision::FirstNext(key) => (&self.first, key, "l"),
            MergeDecision::SecondNext(key) => (&self.second, key, "r"),
        };
        let item = buffer
            .borrow_mut()
            .get_mut(&key)
            .unwrap()
            .pop_front()
            .unwrap();
        self.to_release = Some(vec![(key, item)]);
        self.release_source = Some(source);
    }

    fn implicit(&mut self) {
        // Implicit behavior exists for tick hooks whose tick is forced to run by *other*
        // hooks; a top-level observation consists of exactly this hook, so it can never
        // be forced to run without a scripted decision.
        abort!("implicit decision invoked on a top-level keyed merge-ordered hook");
    }

    fn status(&self) -> MergeStatus {
        MergeStatus {
            first_buffered: keyed_buffer_len(&self.first.borrow()),
            second_buffered: keyed_buffer_len(&self.second.borrow()),
        }
    }

    fn describe_pending(&self) -> Option<String> {
        let first = keyed_buffer_len(&self.first.borrow());
        let second = keyed_buffer_len(&self.second.borrow());
        (first + second > 0).then(|| {
            format!(
                "{} + {} buffered item(s) (first + second input)",
                first, second
            )
        })
    }
}

impl<K, V> ScriptableObservationHook for TopLevelKeyedMergeOrderedHook<K, V>
where
    K: Hash + Eq + Clone + serde::Serialize + serde::de::DeserializeOwned,
    V: serde::Serialize + serde::de::DeserializeOwned + PartialEq,
{
}

/// Top-level hook for `assume_retries` (`AtLeastOnce` → `ExactlyOnce`) on a **`NoOrder`**
/// stream: the point where the simulator injects the retries the type says downstream
/// must tolerate. Each decision picks one buffered element and a cardinality `n >= 1`,
/// releasing `n` copies and consuming the element.
///
/// Cardinality is the *entire* decision: the output stays `NoOrder`, so the placement of
/// the copies is unobservable — downstream ordering points shuffle them like any other
/// elements. This is what makes `assume_retries().assume_ordering()` explore the same
/// state space as `assume_ordering().assume_retries()` (see
/// [`TopLevelAtLeastOnceOrderHook`] for the other factoring): here each instance's
/// copies become distinct elements of a `NoOrder + ExactlyOnce` multiset, and the
/// downstream ordering observation arranges that multiset arbitrarily.
///
/// There is no autonomous ([`ObservationHook`]) implementation: cardinalities make the
/// decision space infinite, so this hook exists only bound to a sim hook handle.
pub struct TopLevelStreamRetriesHook<T> {
    pub input: Rc<RefCell<VecDeque<T>>>,
    pub to_release: Option<Vec<T>>,
    pub output: Sender<T>,
    pub location: HookLocationMeta,
    pub format_item_debug: fn(&T) -> Option<String>,
}

/// Top-level hook for `assume_retries` (`AtLeastOnce` → `ExactlyOnce`) on a
/// **`TotalOrder`** stream. Each decision names the **front** of the queue and a
/// cardinality `n >= 1`, releasing `n` copies **adjacently in place** and consuming the
/// slot.
///
/// Adjacent-only expansion is deliberate. An `AtLeastOnce` collection `[A, B]` denotes
/// the physical realizations `A⁺B⁺` (each element instance stutters *in place*); a
/// delayed redelivery like `ABAB` is a *different* collection (`[A, B, A, B]`), mintable
/// only where the *order* dimension is observed — `assume_ordering` on an `AtLeastOnce`
/// stream ([`TopLevelAtLeastOnceOrderHook`]) can emit an instance into several
/// non-adjacent slots, which then arrive here as separate slots to expand. This
/// factoring makes `assume_ordering().assume_retries()` and
/// `assume_retries().assume_ordering()` explore the same state space: any sequence in
/// which every input instance appears at least once.
///
/// There is no autonomous ([`ObservationHook`]) implementation: cardinalities make the
/// decision space infinite, so this hook exists only bound to a sim hook handle.
pub struct TopLevelOrderedStreamRetriesHook<T> {
    pub input: Rc<RefCell<VecDeque<T>>>,
    pub to_release: Option<Vec<T>>,
    pub output: Sender<T>,
    pub location: HookLocationMeta,
    pub format_item_debug: fn(&T) -> Option<String>,
}

/// The shared [`RuntimeHook`] surface of the two top-level retries kinds (they differ
/// only in which buffered element a decision may name).
macro_rules! top_level_retries_runtime_hook {
    ($hook:ident) => {
        impl<T> RuntimeHook for $hook<T> {
            fn has_pending_input(&self) -> bool {
                !self.input.borrow().is_empty()
            }

            fn only_one_possible_decision(&self) -> bool {
                // Even a sole buffered element admits infinitely many decisions (its
                // cardinality), so only an empty buffer is unique.
                self.input.borrow().is_empty()
            }

            fn release_decision(&mut self, log_writer: Option<&mut dyn std::fmt::Write>) {
                if let Some(to_release) = self.to_release.take() {
                    if !to_release.is_empty()
                        && let Some(log_writer) = log_writer
                    {
                        let HookLocationMeta {
                            location: batch_location,
                            line,
                            caret_indent,
                        } = self.location;
                        let note_str = format!(
                            "^ observed non-deterministic retries: {:?}",
                            TruncatedVecDebug(
                                RefCell::new(Some(to_release.iter())),
                                8,
                                self.format_item_debug
                            )
                        );

                        let _ = writeln!(log_writer);
                        log_release(
                            log_writer,
                            batch_location,
                            line,
                            caret_indent,
                            &note_str,
                            colored::Color::Green,
                        );
                    }

                    for item in to_release {
                        self.output.try_send(item).unwrap();
                    }
                } else {
                    panic!("No decision to release");
                }
            }

            fn location_meta(&self) -> HookLocationMeta {
                self.location
            }
        }
    };
}

top_level_retries_runtime_hook!(TopLevelStreamRetriesHook);
top_level_retries_runtime_hook!(TopLevelOrderedStreamRetriesHook);

/// A scripted decision for a top-level `assume_retries` observation on a **`NoOrder`**
/// stream: release `copies >= 1` copies of the named buffered element, consuming it.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum RetriesDecision<T> {
    Retry(T, usize),
}

impl<T> ScriptDecision for RetriesDecision<T>
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    fn describe(&self) -> String {
        let RetriesDecision::Retry(_, copies) = self;
        format!("retry(value, {})", copies)
    }
}

/// A scripted decision for a top-level `assume_retries` observation on a
/// **`TotalOrder`** stream: expand the front slot into `copies >= 1` adjacent copies,
/// consuming it. The front may be named ([`Self::Retry`], asserting its value) or
/// released positionally ([`Self::RetryNext`], whatever it is).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum OrderedRetriesDecision<T> {
    /// Expand the front of the queue, which must equal this value, into the given
    /// number of copies.
    Retry(T, usize),
    /// Expand the front of the queue, whatever it is, into the given number of copies.
    RetryNext(usize),
}

impl<T> ScriptDecision for OrderedRetriesDecision<T>
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    fn describe(&self) -> String {
        match self {
            OrderedRetriesDecision::Retry(_, copies) => format!("retry(value, {})", copies),
            OrderedRetriesDecision::RetryNext(copies) => format!("retry_next({})", copies),
        }
    }
}

/// Builds the staged release for an honored `retry`: `copies - 1` clones plus the
/// consumed element itself.
fn expand_retries<T: Clone>(item: T, copies: usize) -> Vec<T> {
    let mut to_release = Vec::with_capacity(copies);
    for _ in 1..copies {
        to_release.push(item.clone());
    }
    to_release.push(item);
    to_release
}

/// Defensive check mirrored from the handle's call-site assertion: an element observed
/// zero times would be a *loss*, which `AtLeastOnce` does not permit.
fn check_copies_at_least_one(copies: usize) -> Result<(), String> {
    if copies == 0 {
        Err("retry: the number of copies must be at least 1".to_owned())
    } else {
        Ok(())
    }
}

impl<T> ScriptableHook for TopLevelStreamRetriesHook<T>
where
    T: Clone + serde::Serialize + serde::de::DeserializeOwned + PartialEq,
{
    type Decision = RetriesDecision<T>;
    type Status = OrderingStatus;

    fn is_honorable(&self, decision: &Self::Decision) -> Result<bool, String> {
        let RetriesDecision::Retry(expected, copies) = decision;
        check_copies_at_least_one(*copies)?;
        Ok(self.input.borrow().iter().any(|item| item == expected))
    }

    fn apply(&mut self, decision: Self::Decision) {
        let RetriesDecision::Retry(expected, copies) = decision;
        let mut input = self.input.borrow_mut();
        let index = input.iter().position(|item| item == &expected).unwrap();
        let item = input.remove(index).unwrap();
        self.to_release = Some(expand_retries(item, copies));
    }

    fn implicit(&mut self) {
        // Implicit behavior exists for tick hooks whose tick is forced to run by *other*
        // hooks; a top-level observation consists of exactly this hook, so it can never
        // be forced to run without a scripted decision.
        abort!("implicit decision invoked on a top-level retries hook");
    }

    fn status(&self) -> Self::Status {
        OrderingStatus {
            buffered: self.input.borrow().len(),
        }
    }

    fn describe_pending(&self) -> Option<String> {
        let input = self.input.borrow();
        (!input.is_empty()).then(|| {
            format!(
                "{} buffered item(s) awaiting a retries decision: {:?}",
                input.len(),
                TruncatedVecDebug(RefCell::new(Some(input.iter())), 8, self.format_item_debug)
            )
        })
    }
}

impl<T> ScriptableObservationHook for TopLevelStreamRetriesHook<T> where
    T: Clone + serde::Serialize + serde::de::DeserializeOwned + PartialEq
{
}

impl<T> ScriptableHook for TopLevelOrderedStreamRetriesHook<T>
where
    T: Clone + serde::Serialize + serde::de::DeserializeOwned + PartialEq,
{
    type Decision = OrderedRetriesDecision<T>;
    type Status = OrderingStatus;

    fn is_honorable(&self, decision: &Self::Decision) -> Result<bool, String> {
        let (expected, copies) = match decision {
            OrderedRetriesDecision::Retry(expected, copies) => (Some(expected), *copies),
            OrderedRetriesDecision::RetryNext(copies) => (None, *copies),
        };
        check_copies_at_least_one(copies)?;
        match (self.input.borrow().front(), expected) {
            (Some(_), None) => Ok(true),
            (Some(front), Some(expected)) if front == expected => Ok(true),
            // Input order is preserved, so a mismatching front can never be released
            // ahead of the named value: the decision is permanently stuck.
            (Some(_), Some(_)) => Err(
                "retry: the named value did not match the front of the buffered queue \
                 (the input is ordered, so retries may only expand the front element in \
                 place)"
                    .to_owned(),
            ),
            (None, _) => Ok(false),
        }
    }

    fn apply(&mut self, decision: Self::Decision) {
        let copies = match decision {
            OrderedRetriesDecision::Retry(_, copies) => copies,
            OrderedRetriesDecision::RetryNext(copies) => copies,
        };
        let item = self.input.borrow_mut().pop_front().unwrap();
        self.to_release = Some(expand_retries(item, copies));
    }

    fn implicit(&mut self) {
        // Implicit behavior exists for tick hooks whose tick is forced to run by *other*
        // hooks; a top-level observation consists of exactly this hook, so it can never
        // be forced to run without a scripted decision.
        abort!("implicit decision invoked on a top-level retries hook");
    }

    fn status(&self) -> Self::Status {
        OrderingStatus {
            buffered: self.input.borrow().len(),
        }
    }

    fn describe_pending(&self) -> Option<String> {
        let input = self.input.borrow();
        (!input.is_empty()).then(|| {
            format!(
                "{} buffered item(s) awaiting a retries decision: {:?}",
                input.len(),
                TruncatedVecDebug(RefCell::new(Some(input.iter())), 8, self.format_item_debug)
            )
        })
    }
}

impl<T> ScriptableObservationHook for TopLevelOrderedStreamRetriesHook<T> where
    T: Clone + serde::Serialize + serde::de::DeserializeOwned + PartialEq
{
}

/// Top-level hook for `assume_ordering` on an `AtLeastOnce` stream. Ordering a stream
/// that may carry retries means choosing the sequence of **slots** each element instance
/// occupies, and an instance may occupy several non-adjacent slots: from a queue of
/// `{A, B}`, `ABAB…` is a legitimate slot sequence (each extra slot is a delayed
/// redelivery, still `AtLeastOnce`). So each decision releases one slot:
///
/// - `emit(value)` releases a copy and *keeps* the value buffered for future slots;
/// - `emit_final(value)` releases the value's last slot and consumes every buffered
///   element equal to it, so the queue eventually drains and the forgotten-hook check
///   stays meaningful.
///
/// Adjacent duplicate slots are rejected as redundant when only one buffered element
/// matches the value: as collections under `TotalOrder + AtLeastOnce` they are
/// equivalent to the single slot (`AA` and `A` both denote `A⁺`), and the *cardinality*
/// dimension belongs to the downstream `assume_retries` observation
/// ([`TopLevelOrderedStreamRetriesHook`]). Decisions name **values**, not element
/// instances (equal instances are indistinguishable to the script), so the hook works
/// at the value-class level: `emit(v)` releases one slot for the class, and
/// `emit_final(v)` releases its last slot while consuming *every* buffered element
/// equal to `v` — checking, via the per-class emission counts, that the class received
/// at least as many slots as it has buffered elements (each element must be observed at
/// least once; at-least-once permits duplicates, not losses). Two adjacent slots of the
/// same value are legitimate exactly when two or more buffered elements share it.
///
/// There is no autonomous ([`ObservationHook`]) implementation: re-emission makes the
/// decision space infinite, so this hook exists only bound to a sim hook handle.
pub struct TopLevelAtLeastOnceOrderHook<T> {
    pub input: Rc<RefCell<VecDeque<T>>>,
    pub to_release: Option<Vec<T>>,
    /// Whether the staged release is a non-final slot (its value stays buffered),
    /// noted in the release log.
    pub release_provisional: bool,
    /// The value released by the immediately-preceding decision, if that decision was a
    /// provisional `emit` (elements equal to it are still buffered). The next decision
    /// may name the same value only if two or more buffered elements share it —
    /// otherwise it would place the same sole element in adjacent slots, a redundant
    /// duplicate. `None` after an `emit_final` (everything equal to the released value
    /// was consumed, so a later equal arrival is a genuinely new element).
    pub last_emitted: Option<T>,
    /// Per value-class count of provisional slots released so far, consulted by
    /// `emit_final` to check that every buffered element of the class got a slot.
    /// An entry lives from a class's first `emit` until its `emit_final` consumes it.
    pub emitted_counts: Vec<(T, usize)>,
    pub output: Sender<T>,
    pub location: HookLocationMeta,
    pub format_item_debug: fn(&T) -> Option<String>,
}

impl<T> RuntimeHook for TopLevelAtLeastOnceOrderHook<T> {
    fn has_pending_input(&self) -> bool {
        !self.input.borrow().is_empty()
    }

    fn only_one_possible_decision(&self) -> bool {
        // Even a sole buffered element admits more than one decision (emit it
        // provisionally or finally), so only an empty buffer is unique.
        self.input.borrow().is_empty()
    }

    fn release_decision(&mut self, log_writer: Option<&mut dyn std::fmt::Write>) {
        if let Some(to_release) = self.to_release.take() {
            let provisional = std::mem::take(&mut self.release_provisional);
            if !to_release.is_empty()
                && let Some(log_writer) = log_writer
            {
                let HookLocationMeta {
                    location: batch_location,
                    line,
                    caret_indent,
                } = self.location;
                let note_str = format!(
                    "^ observed non-deterministic order: {:?}{}",
                    TruncatedVecDebug(
                        RefCell::new(Some(to_release.iter())),
                        8,
                        self.format_item_debug
                    ),
                    if provisional { " (may re-emit)" } else { "" }
                );

                let _ = writeln!(log_writer);
                log_release(
                    log_writer,
                    batch_location,
                    line,
                    caret_indent,
                    &note_str,
                    colored::Color::Green,
                );
            }

            for item in to_release {
                self.output.try_send(item).unwrap();
            }
        } else {
            panic!("No decision to release");
        }
    }

    fn location_meta(&self) -> HookLocationMeta {
        self.location
    }
}

/// A scripted decision for a top-level `assume_ordering` on an `AtLeastOnce` stream:
/// release one slot for a buffered element instance, either keeping the instance for
/// future re-emission ([`Self::Emit`]) or consuming it ([`Self::EmitFinal`]).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum AtLeastOnceOrderingDecision<T> {
    Emit(T),
    EmitFinal(T),
}

impl<T> ScriptDecision for AtLeastOnceOrderingDecision<T>
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    fn describe(&self) -> String {
        match self {
            AtLeastOnceOrderingDecision::Emit(_) => "emit(value)".to_owned(),
            AtLeastOnceOrderingDecision::EmitFinal(_) => "emit_final(value)".to_owned(),
        }
    }
}

impl<T> TopLevelAtLeastOnceOrderHook<T>
where
    T: PartialEq,
{
    /// The number of buffered elements equal to `value`.
    fn buffered_count(&self, value: &T) -> usize {
        self.input
            .borrow()
            .iter()
            .filter(|item| *item == value)
            .count()
    }

    /// The number of provisional slots already released for `value`'s class.
    fn emitted_count(&self, value: &T) -> usize {
        self.emitted_counts
            .iter()
            .find(|(v, _)| v == value)
            .map_or(0, |(_, count)| *count)
    }

    /// Validates a decision naming `value` against the value-class rules. Returns
    /// `Ok(true)` when honorable now, `Ok(false)` when it should wait for a matching
    /// element to arrive, and `Err` for a mis-stated script.
    fn check_decision(&self, value: &T, is_final: bool) -> Result<bool, String> {
        let buffered = self.buffered_count(value);
        if buffered == 0 {
            return Ok(false);
        }
        if self.last_emitted.as_ref() == Some(value) && buffered < 2 {
            return Err(
                "emit/emit_final: this would place the just-emitted element in adjacent \
                 slots, but only one buffered element matches the value; adjacent duplicate \
                 slots of the same element are redundant (they denote the same \
                 at-least-once collection) — interleave another element's slot first, or \
                 script the copy count at the downstream `assume_retries` instead"
                    .to_owned(),
            );
        }
        if is_final && self.emitted_count(value) + 1 < buffered {
            return Err(format!(
                "emit_final: {} buffered element(s) equal the named value, but only {} \
                 slot(s) would have been released for it; every buffered element must \
                 occupy at least one slot (at-least-once permits duplicates, not losses) — \
                 emit more slots before retiring the value",
                buffered,
                self.emitted_count(value) + 1,
            ));
        }
        Ok(true)
    }
}

impl<T> ScriptableHook for TopLevelAtLeastOnceOrderHook<T>
where
    T: Clone + serde::Serialize + serde::de::DeserializeOwned + PartialEq,
{
    type Decision = AtLeastOnceOrderingDecision<T>;
    type Status = OrderingStatus;

    fn is_honorable(&self, decision: &Self::Decision) -> Result<bool, String> {
        match decision {
            AtLeastOnceOrderingDecision::Emit(expected) => self.check_decision(expected, false),
            AtLeastOnceOrderingDecision::EmitFinal(expected) => self.check_decision(expected, true),
        }
    }

    fn apply(&mut self, decision: Self::Decision) {
        match decision {
            AtLeastOnceOrderingDecision::Emit(expected) => {
                if let Some((_, count)) =
                    self.emitted_counts.iter_mut().find(|(v, _)| *v == expected)
                {
                    *count += 1;
                } else {
                    self.emitted_counts.push((expected.clone(), 1));
                }
                self.to_release = Some(vec![expected.clone()]);
                self.release_provisional = true;
                self.last_emitted = Some(expected);
            }
            AtLeastOnceOrderingDecision::EmitFinal(expected) => {
                // The last slot of the value's class: release one copy and consume
                // every buffered element equal to it (the script has no way to name
                // individual instances, so retirement is class-level).
                self.input.borrow_mut().retain(|item| item != &expected);
                self.emitted_counts.retain(|(v, _)| v != &expected);
                self.to_release = Some(vec![expected]);
                self.release_provisional = false;
                self.last_emitted = None;
            }
        }
    }

    fn implicit(&mut self) {
        // Implicit behavior exists for tick hooks whose tick is forced to run by *other*
        // hooks; a top-level observation consists of exactly this hook, so it can never
        // be forced to run without a scripted decision.
        abort!("implicit decision invoked on a top-level at-least-once ordering hook");
    }

    fn status(&self) -> Self::Status {
        OrderingStatus {
            buffered: self.input.borrow().len(),
        }
    }

    fn describe_pending(&self) -> Option<String> {
        let input = self.input.borrow();
        (!input.is_empty()).then(|| {
            format!(
                "{} buffered ordering item(s) (each may be emitted into several slots): {:?}",
                input.len(),
                TruncatedVecDebug(RefCell::new(Some(input.iter())), 8, self.format_item_debug)
            )
        })
    }
}

impl<T> ScriptableObservationHook for TopLevelAtLeastOnceOrderHook<T> where
    T: Clone + serde::Serialize + serde::de::DeserializeOwned + PartialEq
{
}
