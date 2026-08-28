//! Top-level observation hooks ([`ObservationHook`]): hooks with no tick DFIR that are
//! their own scheduling unit — running one *is* releasing data (e.g. `assume_ordering`
//! on a non-tick stream, or a hooked top-level `fold`). Each is its own
//! `SimObservation` in the scheduler. Their scripted decision/status types and
//! [`ScriptableHook`] impls live alongside them.

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
    ScriptableObservationHook, TruncatedLabeledVecDebug, TruncatedVecDebug, abort, log_release,
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
