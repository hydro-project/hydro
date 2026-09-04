//! In-tick (inline) hooks ([`InlineHook`]): hooks resolved *while* a tick DFIR is
//! executing, for operators that block on ordering decisions mid-tick. Their input
//! exists only during one tick execution, so unlike the other kinds they have no
//! cross-boundary buffering (and no pause semantics when scripted). The
//! [`ScriptableInlineHook`] impls live alongside them.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::hash::Hash;
use std::rc::Rc;

use bolero::generator::bolero_generator::driver::object::Borrowed;
use bolero::{ValueGenerator, produce};
use dfir_rs::rustc_hash::{FxHashMap, FxHashSet};
use dfir_rs::util::unsync::mpsc::Sender;

use super::{
    HookLocationMeta, InlineHook, ManualDebug, MergeStatus, OrderingStatus, RuntimeHook,
    ScriptDecision, ScriptableInlineHook, TruncatedLabeledVecDebug, TruncatedVecDebug, log_release,
};

pub struct StreamOrderHook<T> {
    input: Rc<RefCell<Option<Vec<T>>>>,
    to_release: Option<Vec<T>>,
    output: Sender<Vec<T>>,
    batch_location: HookLocationMeta,
    format_debug: fn(&T) -> Option<String>,
}

impl<T> StreamOrderHook<T> {
    pub fn new(
        input: Rc<RefCell<Option<Vec<T>>>>,
        output: Sender<Vec<T>>,
        batch_location: HookLocationMeta,
        format_debug: fn(&T) -> Option<String>,
    ) -> Self {
        Self {
            input,
            to_release: None,
            output,
            batch_location,
            format_debug,
        }
    }
}

impl<T> RuntimeHook for StreamOrderHook<T> {
    fn has_pending_input(&self) -> bool {
        self.input.borrow().is_some()
    }

    fn only_one_possible_decision(&self) -> bool {
        self.input
            .borrow()
            .as_ref()
            .is_none_or(|inputs| inputs.len() <= 1)
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
                } = self.batch_location;
                let note_str = format!(
                    "^ observed non-deterministic order: {:?}",
                    TruncatedVecDebug(RefCell::new(Some(to_release.iter())), 8, self.format_debug)
                );

                log_release(
                    log_writer,
                    batch_location,
                    line,
                    caret_indent,
                    &note_str,
                    colored::Color::Cyan,
                );
            }

            self.output.try_send(to_release).unwrap();
        } else {
            panic!("No decision to release");
        }
    }

    fn location_meta(&self) -> HookLocationMeta {
        self.batch_location
    }
}

impl<T> InlineHook for StreamOrderHook<T> {
    fn autonomous_decision<'a>(&mut self, driver: &mut Borrowed<'a>) {
        let mut inputs = self.input.borrow_mut().take().unwrap();

        // from Bolero
        let max_dst = inputs.len().saturating_sub(1);
        for src in 0..max_dst {
            let dst = (src..=max_dst).generate(driver).unwrap();
            inputs.swap(src, dst);
        }

        self.to_release = Some(inputs);
    }
}

pub struct MergeOrderedHook<T> {
    first: Rc<RefCell<Option<Vec<T>>>>,
    second: Rc<RefCell<Option<Vec<T>>>>,
    to_release: Option<Vec<T>>,
    release_sources: Option<Vec<bool>>,
    output: Sender<Vec<T>>,
    batch_location: HookLocationMeta,
    format_debug: fn(&T) -> Option<String>,
}

impl<T> MergeOrderedHook<T> {
    pub fn new(
        first: Rc<RefCell<Option<Vec<T>>>>,
        second: Rc<RefCell<Option<Vec<T>>>>,
        output: Sender<Vec<T>>,
        batch_location: HookLocationMeta,
        format_debug: fn(&T) -> Option<String>,
    ) -> Self {
        Self {
            first,
            second,
            to_release: None,
            release_sources: None,
            output,
            batch_location,
            format_debug,
        }
    }
}

impl<T> RuntimeHook for MergeOrderedHook<T> {
    fn has_pending_input(&self) -> bool {
        self.first.borrow().is_some() && self.second.borrow().is_some()
    }

    fn only_one_possible_decision(&self) -> bool {
        // The interleaving is only a choice when both inputs have elements.
        self.first.borrow().as_ref().is_none_or(|f| f.is_empty())
            || self.second.borrow().as_ref().is_none_or(|s| s.is_empty())
    }

    fn release_decision(&mut self, log_writer: Option<&mut dyn std::fmt::Write>) {
        if let Some(to_release) = self.to_release.take() {
            let sources = self.release_sources.take().unwrap();
            if !to_release.is_empty()
                && let Some(log_writer) = log_writer
            {
                let HookLocationMeta {
                    location: batch_location,
                    line,
                    caret_indent,
                } = self.batch_location;

                let labeled_iter =
                    sources
                        .iter()
                        .zip(to_release.iter())
                        .map(|(is_second, item)| {
                            let label: &'static str = if *is_second { "r" } else { "l" };
                            (label, item)
                        });

                let note_str = format!(
                    "^ observed non-deterministic merge order: {:?}",
                    TruncatedLabeledVecDebug(
                        RefCell::new(Some(labeled_iter)),
                        8,
                        self.format_debug
                    )
                );

                log_release(
                    log_writer,
                    batch_location,
                    line,
                    caret_indent,
                    &note_str,
                    colored::Color::Cyan,
                );
            }

            self.output.try_send(to_release).unwrap();
        } else {
            panic!("No decision to release");
        }
    }

    fn location_meta(&self) -> HookLocationMeta {
        self.batch_location
    }
}

impl<T> InlineHook for MergeOrderedHook<T> {
    fn autonomous_decision<'a>(&mut self, driver: &mut Borrowed<'a>) {
        let first_input = self.first.borrow_mut().take().unwrap();
        let second_input = self.second.borrow_mut().take().unwrap();

        let first_len = first_input.len();
        let second_len = second_input.len();

        // Generate a valid interleaving preserving per-input order.
        let mut result = Vec::with_capacity(first_len + second_len);
        let mut sources = Vec::with_capacity(first_len + second_len);
        let mut first_iter = first_input.into_iter();
        let mut second_iter = second_input.into_iter();
        let mut first_remaining = first_len;
        let mut second_remaining = second_len;

        while first_remaining > 0 && second_remaining > 0 {
            let take_second: bool = produce().generate(driver).unwrap();
            if take_second {
                result.push(second_iter.next().unwrap());
                sources.push(true);
                second_remaining -= 1;
            } else {
                result.push(first_iter.next().unwrap());
                sources.push(false);
                first_remaining -= 1;
            }
        }

        for item in first_iter {
            result.push(item);
            sources.push(false);
        }
        for item in second_iter {
            result.push(item);
            sources.push(true);
        }

        self.to_release = Some(result);
        self.release_sources = Some(sources);
    }
}

type KeyedStreamOrderHookInput<K, V> = Rc<RefCell<Option<Vec<(K, V)>>>>;

pub struct KeyedStreamOrderHook<K: Hash + Eq + Clone, V> {
    input: KeyedStreamOrderHookInput<K, V>,
    to_release: Option<FxHashMap<K, Vec<V>>>,
    output: Sender<Vec<(K, V)>>,
    batch_location: HookLocationMeta,
    format_key_debug: fn(&K) -> Option<String>,
    format_value_debug: fn(&V) -> Option<String>,
}

impl<K: Hash + Eq + Clone, V> KeyedStreamOrderHook<K, V> {
    pub fn new(
        input: KeyedStreamOrderHookInput<K, V>,
        output: Sender<Vec<(K, V)>>,
        batch_location: HookLocationMeta,
        format_key_debug: fn(&K) -> Option<String>,
        format_value_debug: fn(&V) -> Option<String>,
    ) -> Self {
        Self {
            input,
            to_release: None,
            output,
            batch_location,
            format_key_debug,
            format_value_debug,
        }
    }
}

impl<K: Hash + Eq + Clone, V> RuntimeHook for KeyedStreamOrderHook<K, V> {
    fn has_pending_input(&self) -> bool {
        self.input.borrow().is_some()
    }

    fn only_one_possible_decision(&self) -> bool {
        // Ordering is only unobservable when no key has more than one element.
        self.input.borrow().as_ref().is_none_or(|inputs| {
            let mut seen_keys: Vec<&K> = vec![];
            for (k, _) in inputs {
                if seen_keys.contains(&k) {
                    return false;
                }
                seen_keys.push(k);
            }
            true
        })
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
                } = self.batch_location;
                let mut note_str = String::new();
                for (key, values) in &to_release {
                    let entry_text = format!(
                        "{:?}: {:?}",
                        ManualDebug(key, self.format_key_debug),
                        TruncatedVecDebug(
                            RefCell::new(Some(values.iter())),
                            8,
                            self.format_value_debug
                        )
                    );
                    if !note_str.is_empty() {
                        note_str.push_str(", ");
                    }
                    note_str.push_str(&entry_text);
                }
                note_str = format!("^ observed non-deterministic order: {{ {} }}", note_str);

                log_release(
                    log_writer,
                    batch_location,
                    line,
                    caret_indent,
                    &note_str,
                    colored::Color::Cyan,
                );
            }

            let mut flat_out = vec![];
            for (k, vs) in to_release {
                for v in vs {
                    flat_out.push((k.clone(), v));
                }
            }
            self.output.try_send(flat_out).unwrap();
        } else {
            panic!("No decision to release");
        }
    }

    fn location_meta(&self) -> HookLocationMeta {
        self.batch_location
    }
}

impl<K: Hash + Eq + Clone, V> InlineHook for KeyedStreamOrderHook<K, V> {
    fn autonomous_decision<'a>(&mut self, driver: &mut Borrowed<'a>) {
        let mut inputs = self.input.borrow_mut().take().unwrap();
        let mut grouped = FxHashMap::default();
        for (k, v) in inputs.drain(..) {
            grouped.entry(k).or_insert_with(Vec::new).push(v);
        }

        let mut out = FxHashMap::default();
        for (key, mut values) in grouped {
            // from Bolero
            let max_dst = values.len().saturating_sub(1);
            for src in 0..max_dst {
                let dst = (src..=max_dst).generate(driver).unwrap();
                values.swap(src, dst);
            }

            out.insert(key, values);
        }

        self.to_release = Some(out);
    }
}

/// Inline hook for `entries_partially_ordered` on bounded/not-root keyed streams.
/// Produces a random interleaving of key-value pairs that preserves within-key order.
pub struct PartiallyOrderedStreamHook<K: Hash + Eq + Clone, V> {
    input: KeyedStreamOrderHookInput<K, V>,
    to_release: Option<Vec<(K, V)>>,
    output: Sender<Vec<(K, V)>>,
    batch_location: HookLocationMeta,
    format_key_debug: fn(&K) -> Option<String>,
    format_value_debug: fn(&V) -> Option<String>,
}

impl<K: Hash + Eq + Clone, V> PartiallyOrderedStreamHook<K, V> {
    pub fn new(
        input: KeyedStreamOrderHookInput<K, V>,
        output: Sender<Vec<(K, V)>>,
        batch_location: HookLocationMeta,
        format_key_debug: fn(&K) -> Option<String>,
        format_value_debug: fn(&V) -> Option<String>,
    ) -> Self {
        Self {
            input,
            to_release: None,
            output,
            batch_location,
            format_key_debug,
            format_value_debug,
        }
    }
}

impl<K: Hash + Eq + Clone, V> RuntimeHook for PartiallyOrderedStreamHook<K, V> {
    fn has_pending_input(&self) -> bool {
        self.input.borrow().is_some()
    }

    fn only_one_possible_decision(&self) -> bool {
        // The interleaving across keys is the only choice; a single distinct key (or no
        // input) leaves nothing to decide.
        self.input.borrow().as_ref().is_none_or(|inputs| {
            let mut distinct_keys: Vec<&K> = vec![];
            for (k, _) in inputs {
                if !distinct_keys.contains(&k) {
                    distinct_keys.push(k);
                }
            }
            distinct_keys.len() <= 1
        })
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
                } = self.batch_location;
                let mut note_str = String::new();
                for (key, value) in &to_release {
                    let entry_text = format!(
                        "({:?}, {:?})",
                        ManualDebug(key, self.format_key_debug),
                        ManualDebug(value, self.format_value_debug)
                    );
                    if !note_str.is_empty() {
                        note_str.push_str(", ");
                    }
                    note_str.push_str(&entry_text);
                }
                note_str = format!("^ observed partially-ordered interleaving: [{}]", note_str);

                log_release(
                    log_writer,
                    batch_location,
                    line,
                    caret_indent,
                    &note_str,
                    colored::Color::Cyan,
                );
            }

            self.output.try_send(to_release).unwrap();
        } else {
            panic!("No decision to release");
        }
    }

    fn location_meta(&self) -> HookLocationMeta {
        self.batch_location
    }
}

impl<K: Hash + Eq + Clone, V> InlineHook for PartiallyOrderedStreamHook<K, V> {
    fn autonomous_decision<'a>(&mut self, driver: &mut Borrowed<'a>) {
        let inputs = self.input.borrow_mut().take().unwrap();
        // Group by key, preserving within-key order
        let mut grouped: Vec<(K, VecDeque<V>)> = Vec::new();
        let mut key_indices: FxHashMap<K, usize> = FxHashMap::default();
        for (k, v) in inputs {
            if let Some(&idx) = key_indices.get(&k) {
                grouped[idx].1.push_back(v);
            } else {
                let idx = grouped.len();
                key_indices.insert(k.clone(), idx);
                grouped.push((k, VecDeque::from([v])));
            }
        }

        // Interleave: repeatedly pick a random non-empty key and take its front element
        let mut out = Vec::new();
        loop {
            let nonempty: Vec<usize> = grouped
                .iter()
                .enumerate()
                .filter(|(_, (_, q))| !q.is_empty())
                .map(|(i, _)| i)
                .collect();
            if nonempty.is_empty() {
                break;
            }
            let pick = nonempty[(0..nonempty.len()).generate(driver).unwrap()];
            let (k, q) = &mut grouped[pick];
            out.push((k.clone(), q.pop_front().unwrap()));
        }

        self.to_release = Some(out);
    }
}

type KeyedMergeOrderedInput<K, V> = Rc<RefCell<Option<Vec<(K, V)>>>>;

/// Keyed variant of [`MergeOrderedHook`], used for bounded / non-root keyed
/// streams. Both inputs are fully materialized batches. Interleaves the two
/// batches *independently within each key*, preserving per-input order within
/// each key. The interleaving across different keys is irrelevant because a
/// [`crate::live_collections::keyed_stream::KeyedStream`] carries no ordering
/// guarantee across keys.
pub struct KeyedMergeOrderedHook<K: Hash + Eq + Clone, V> {
    first: KeyedMergeOrderedInput<K, V>,
    second: KeyedMergeOrderedInput<K, V>,
    to_release: Option<Vec<(K, V)>>,
    release_sources: Option<Vec<bool>>,
    output: Sender<Vec<(K, V)>>,
    batch_location: HookLocationMeta,
    format_item_debug: fn(&(K, V)) -> Option<String>,
}

impl<K: Hash + Eq + Clone, V> KeyedMergeOrderedHook<K, V> {
    pub fn new(
        first: KeyedMergeOrderedInput<K, V>,
        second: KeyedMergeOrderedInput<K, V>,
        output: Sender<Vec<(K, V)>>,
        batch_location: HookLocationMeta,
        format_item_debug: fn(&(K, V)) -> Option<String>,
    ) -> Self {
        Self {
            first,
            second,
            to_release: None,
            release_sources: None,
            output,
            batch_location,
            format_item_debug,
        }
    }
}

impl<K: Hash + Eq + Clone, V> RuntimeHook for KeyedMergeOrderedHook<K, V> {
    fn has_pending_input(&self) -> bool {
        self.first.borrow().is_some() && self.second.borrow().is_some()
    }

    fn only_one_possible_decision(&self) -> bool {
        // The interleaving within a key is the only observable ordering choice, so there
        // is only one possible decision when no key appears in both inputs.
        let first = self.first.borrow();
        let second = self.second.borrow();
        if let (Some(first), Some(second)) = (first.as_ref(), second.as_ref()) {
            !first
                .iter()
                .any(|(k, _)| second.iter().any(|(k2, _)| k2 == k))
        } else {
            true
        }
    }

    fn release_decision(&mut self, log_writer: Option<&mut dyn std::fmt::Write>) {
        if let Some(to_release) = self.to_release.take() {
            let sources = self.release_sources.take().unwrap();
            if !to_release.is_empty()
                && let Some(log_writer) = log_writer
            {
                let HookLocationMeta {
                    location: batch_location,
                    line,
                    caret_indent,
                } = self.batch_location;

                let labeled_iter =
                    sources
                        .iter()
                        .zip(to_release.iter())
                        .map(|(is_second, item)| {
                            let label: &'static str = if *is_second { "r" } else { "l" };
                            (label, item)
                        });

                let note_str = format!(
                    "^ observed non-deterministic merge order: {:?}",
                    TruncatedLabeledVecDebug(
                        RefCell::new(Some(labeled_iter)),
                        8,
                        self.format_item_debug
                    )
                );

                log_release(
                    log_writer,
                    batch_location,
                    line,
                    caret_indent,
                    &note_str,
                    colored::Color::Cyan,
                );
            }

            self.output.try_send(to_release).unwrap();
        } else {
            panic!("No decision to release");
        }
    }

    fn location_meta(&self) -> HookLocationMeta {
        self.batch_location
    }
}

impl<K: Hash + Eq + Clone, V> InlineHook for KeyedMergeOrderedHook<K, V> {
    fn autonomous_decision<'a>(&mut self, driver: &mut Borrowed<'a>) {
        let first_input = self.first.borrow_mut().take().unwrap();
        let second_input = self.second.borrow_mut().take().unwrap();

        // Group each input by key, preserving per-input order within each key
        // and recording the order in which keys are first seen so that output
        // is deterministic given the same random choices.
        let mut key_order: Vec<K> = Vec::new();
        let mut first_grouped: FxHashMap<K, VecDeque<V>> = FxHashMap::default();
        let mut second_grouped: FxHashMap<K, VecDeque<V>> = FxHashMap::default();

        for (k, v) in first_input {
            if !first_grouped.contains_key(&k) && !second_grouped.contains_key(&k) {
                key_order.push(k.clone());
            }
            first_grouped.entry(k).or_default().push_back(v);
        }
        for (k, v) in second_input {
            if !first_grouped.contains_key(&k) && !second_grouped.contains_key(&k) {
                key_order.push(k.clone());
            }
            second_grouped.entry(k).or_default().push_back(v);
        }

        let mut result = Vec::new();
        let mut sources = Vec::new();

        for key in key_order {
            let mut first_q = first_grouped.remove(&key).unwrap_or_default();
            let mut second_q = second_grouped.remove(&key).unwrap_or_default();

            // Generate a valid interleaving of this key's two sub-sequences,
            // preserving per-input order.
            while !first_q.is_empty() && !second_q.is_empty() {
                let take_second: bool = produce().generate(driver).unwrap();
                if take_second {
                    result.push((key.clone(), second_q.pop_front().unwrap()));
                    sources.push(true);
                } else {
                    result.push((key.clone(), first_q.pop_front().unwrap()));
                    sources.push(false);
                }
            }
            for v in first_q {
                result.push((key.clone(), v));
                sources.push(false);
            }
            for v in second_q {
                result.push((key.clone(), v));
                sources.push(true);
            }
        }

        self.to_release = Some(result);
        self.release_sources = Some(sources);
    }
}

/// Checks that `scripted` is a permutation of `pending`, returning the mismatch (values
/// pending but not scripted, and scripted but not pending) otherwise. Only used by
/// [`ScriptedInline::run_decision`], which must order a tick's complete input.
fn multiset_mismatch<'a, T: PartialEq>(
    pending: &'a [T],
    scripted: &'a [T],
) -> Result<(), (Vec<&'a T>, Vec<&'a T>)> {
    let mut missing: Vec<&T> = vec![];
    let mut extra: Vec<&T> = scripted.iter().collect();
    for item in pending {
        if let Some(index) = extra.iter().position(|other| *other == item) {
            extra.swap_remove(index);
        } else {
            missing.push(item);
        }
    }
    if missing.is_empty() && extra.is_empty() {
        Ok(())
    } else {
        Err((missing, extra))
    }
}

/// A scripted decision for an `assume_ordering` reached inside a tick.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum InlineOrderingDecision<T> {
    Order(Vec<T>),
}

impl<T> ScriptDecision for InlineOrderingDecision<T>
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    fn describe(&self) -> String {
        let InlineOrderingDecision::Order(values) = self;
        format!("order({} value(s))", values.len())
    }
}

impl<T> ScriptableInlineHook for StreamOrderHook<T>
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq,
{
    type Decision = InlineOrderingDecision<T>;
    type Status = OrderingStatus;

    fn apply_scripted(
        &mut self,
        decision: Option<InlineOrderingDecision<T>>,
        log_writer: Option<&mut dyn std::fmt::Write>,
    ) -> Result<(), String> {
        let input = self.input.borrow_mut().take().unwrap();
        let output = match decision {
            Some(InlineOrderingDecision::Order(values)) => {
                if let Err((missing, extra)) = multiset_mismatch(&input, &values) {
                    let fmt = |items: Vec<&T>| {
                        let rendered: Vec<String> = items
                            .iter()
                            .map(|v| (self.format_debug)(v).unwrap_or_else(|| "<value>".to_owned()))
                            .collect();
                        rendered.join(", ")
                    };
                    return Err(format!(
                        "scripted in-tick ordering decision must contain exactly all pending input values ({} pending, {} scripted; missing: [{}]; extra: [{}])",
                        input.len(),
                        values.len(),
                        fmt(missing),
                        fmt(extra),
                    ));
                }
                values
            }
            // No choice: zero or one element has exactly one possible order.
            None => input,
        };
        self.to_release = Some(output);
        self.release_decision(log_writer);
        Ok(())
    }

    fn status(&self) -> OrderingStatus {
        OrderingStatus {
            buffered: self.input.borrow().as_ref().map_or(0, Vec::len),
        }
    }

    fn describe_pending(&self) -> Option<String> {
        let input = self.input.borrow();
        input.as_ref().map(|values| {
            format!(
                "{} in-tick ordering item(s): {:?}",
                values.len(),
                TruncatedVecDebug(RefCell::new(Some(values.iter())), 8, self.format_debug)
            )
        })
    }
}

impl<K, V> ScriptableInlineHook for KeyedStreamOrderHook<K, V>
where
    K: Hash + Eq + Clone + serde::Serialize + serde::de::DeserializeOwned + PartialEq,
    V: serde::Serialize + serde::de::DeserializeOwned + PartialEq,
{
    type Decision = InlineOrderingDecision<(K, V)>;
    type Status = OrderingStatus;

    fn apply_scripted(
        &mut self,
        decision: Option<InlineOrderingDecision<(K, V)>>,
        log_writer: Option<&mut dyn std::fmt::Write>,
    ) -> Result<(), String> {
        let input = self.input.borrow_mut().take().unwrap();
        let ordered: Vec<(K, V)> = match decision {
            Some(InlineOrderingDecision::Order(values)) => {
                if multiset_mismatch(&input, &values).is_err() {
                    return Err(format!(
                        "scripted in-tick keyed ordering decision must contain exactly all pending input entries ({} pending, {} scripted)",
                        input.len(),
                        values.len(),
                    ));
                }
                values
            }
            // No choice: no key has more than one element, so per-key order is not a
            // choice and the input order stands.
            None => input,
        };

        let mut grouped: FxHashMap<K, Vec<V>> = FxHashMap::default();
        for (k, v) in ordered {
            grouped.entry(k).or_default().push(v);
        }
        self.to_release = Some(grouped);
        self.release_decision(log_writer);
        Ok(())
    }

    fn status(&self) -> OrderingStatus {
        OrderingStatus {
            buffered: self.input.borrow().as_ref().map_or(0, Vec::len),
        }
    }

    fn describe_pending(&self) -> Option<String> {
        let input = self.input.borrow();
        input
            .as_ref()
            .map(|values| format!("{} in-tick keyed ordering entr(ies)", values.len()))
    }
}

impl<K, V> ScriptableInlineHook for PartiallyOrderedStreamHook<K, V>
where
    K: Hash + Eq + Clone + serde::Serialize + serde::de::DeserializeOwned + PartialEq,
    V: serde::Serialize + serde::de::DeserializeOwned + PartialEq,
{
    type Decision = InlineOrderingDecision<(K, V)>;
    type Status = OrderingStatus;

    fn apply_scripted(
        &mut self,
        decision: Option<InlineOrderingDecision<(K, V)>>,
        log_writer: Option<&mut dyn std::fmt::Write>,
    ) -> Result<(), String> {
        let input = self.input.borrow_mut().take().unwrap();
        let output = match decision {
            Some(InlineOrderingDecision::Order(values)) => {
                // The scripted sequence must be a valid interleaving: same entries, and
                // each key's subsequence must preserve that key's input order.
                let valid = values.len() == input.len() && {
                    let mut seen: FxHashSet<&K> = FxHashSet::default();
                    input
                        .iter()
                        .map(|(k, _)| k)
                        .filter(|key| seen.insert(key))
                        .all(|key| {
                            let input_seq = input.iter().filter(|(k, _)| k == key).map(|(_, v)| v);
                            let scripted_seq =
                                values.iter().filter(|(k, _)| k == key).map(|(_, v)| v);
                            input_seq.eq(scripted_seq)
                        })
                };
                if !valid {
                    return Err(format!(
                        "scripted in-tick partially-ordered decision must be an interleaving of all pending entries that preserves each key's order ({} pending, {} scripted)",
                        input.len(),
                        values.len(),
                    ));
                }
                values
            }
            // No choice: at most one distinct key is present, so the interleaving is not
            // a choice and the input order stands.
            None => input,
        };
        self.to_release = Some(output);
        self.release_decision(log_writer);
        Ok(())
    }

    fn status(&self) -> OrderingStatus {
        OrderingStatus {
            buffered: self.input.borrow().as_ref().map_or(0, Vec::len),
        }
    }

    fn describe_pending(&self) -> Option<String> {
        let input = self.input.borrow();
        input
            .as_ref()
            .map(|values| format!("{} in-tick partially-ordered entr(ies)", values.len()))
    }
}

/// Checks that the scripted merge (each entry labeled with the input it is drawn from:
/// `false` = first, `true` = second) consumes exactly the two pending inputs, preserving
/// each input's order: each side's labeled subsequence must equal that input's batch.
fn labeled_merge_consumes_inputs<T: PartialEq>(
    first: &[T],
    second: &[T],
    scripted: &[(bool, T)],
) -> bool {
    let scripted_side = |side: bool| {
        scripted
            .iter()
            .filter(move |(from_second, _)| *from_second == side)
            .map(|(_, value)| value)
    };
    scripted_side(false).eq(first.iter()) && scripted_side(true).eq(second.iter())
}

impl<T> ScriptableInlineHook for MergeOrderedHook<T>
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq,
{
    type Decision = InlineOrderingDecision<(bool, T)>;
    type Status = MergeStatus;

    fn apply_scripted(
        &mut self,
        decision: Option<InlineOrderingDecision<(bool, T)>>,
        log_writer: Option<&mut dyn std::fmt::Write>,
    ) -> Result<(), String> {
        let first = self.first.borrow_mut().take().unwrap();
        let second = self.second.borrow_mut().take().unwrap();
        match decision {
            Some(InlineOrderingDecision::Order(labeled)) => {
                if !labeled_merge_consumes_inputs(&first, &second, &labeled) {
                    return Err(format!(
                        "scripted in-tick merge decision must consume exactly the two inputs' values, each in its original order ({} + {} pending, {} scripted)",
                        first.len(),
                        second.len(),
                        labeled.len(),
                    ));
                }
                let (sources, values) = labeled.into_iter().unzip();
                self.to_release = Some(values);
                self.release_sources = Some(sources);
            }
            // No choice: at most one input has elements, so the interleaving is not a
            // choice and concatenation is the unique result.
            None => {
                let mut sources = vec![false; first.len()];
                sources.extend(std::iter::repeat_n(true, second.len()));
                let mut result = first;
                result.extend(second);
                self.to_release = Some(result);
                self.release_sources = Some(sources);
            }
        }
        self.release_decision(log_writer);
        Ok(())
    }

    fn status(&self) -> MergeStatus {
        MergeStatus {
            first_buffered: self.first.borrow().as_ref().map_or(0, Vec::len),
            second_buffered: self.second.borrow().as_ref().map_or(0, Vec::len),
        }
    }

    fn describe_pending(&self) -> Option<String> {
        let first = self.first.borrow();
        let second = self.second.borrow();
        match (first.as_ref(), second.as_ref()) {
            (Some(first), Some(second)) => Some(format!(
                "{} + {} in-tick merge value(s) (first + second input)",
                first.len(),
                second.len()
            )),
            _ => None,
        }
    }
}

impl<K, V> ScriptableInlineHook for KeyedMergeOrderedHook<K, V>
where
    K: Hash + Eq + Clone + serde::Serialize + serde::de::DeserializeOwned + PartialEq,
    V: serde::Serialize + serde::de::DeserializeOwned + PartialEq,
{
    type Decision = InlineOrderingDecision<(bool, (K, V))>;
    type Status = MergeStatus;

    fn apply_scripted(
        &mut self,
        decision: Option<InlineOrderingDecision<(bool, (K, V))>>,
        log_writer: Option<&mut dyn std::fmt::Write>,
    ) -> Result<(), String> {
        let first = self.first.borrow_mut().take().unwrap();
        let second = self.second.borrow_mut().take().unwrap();
        match decision {
            Some(InlineOrderingDecision::Order(labeled)) => {
                // Each side's labeled subsequence must, for every key, equal that key's
                // entries from that input in order (cross-key order within an input is
                // unobservable for keyed streams, so keys are validated independently).
                let mut seen: FxHashSet<&K> = FxHashSet::default();
                let valid = first
                    .iter()
                    .chain(&second)
                    .map(|(k, _)| k)
                    .chain(labeled.iter().map(|(_, (k, _))| k))
                    .filter(|key| seen.insert(key))
                    .all(|key| {
                        let side_matches = |side: bool, input: &[(K, V)]| {
                            labeled
                                .iter()
                                .filter(|(from_second, (k, _))| *from_second == side && k == key)
                                .map(|(_, (_, v))| v)
                                .eq(input.iter().filter(|(k, _)| k == key).map(|(_, v)| v))
                        };
                        side_matches(false, &first) && side_matches(true, &second)
                    });
                if !valid {
                    return Err(format!(
                        "scripted in-tick keyed merge decision must consume exactly the two inputs' entries, each in its original per-key order ({} + {} pending, {} scripted)",
                        first.len(),
                        second.len(),
                        labeled.len(),
                    ));
                }

                let (sources, values) = labeled.into_iter().unzip();
                self.to_release = Some(values);
                self.release_sources = Some(sources);
            }
            // No choice: no key appears in both inputs, so no per-key interleaving is a
            // choice and concatenation is the unique result.
            None => {
                let mut sources = vec![false; first.len()];
                sources.extend(std::iter::repeat_n(true, second.len()));
                let mut result = first;
                result.extend(second);
                self.to_release = Some(result);
                self.release_sources = Some(sources);
            }
        }
        self.release_decision(log_writer);
        Ok(())
    }

    fn status(&self) -> MergeStatus {
        MergeStatus {
            first_buffered: self.first.borrow().as_ref().map_or(0, Vec::len),
            second_buffered: self.second.borrow().as_ref().map_or(0, Vec::len),
        }
    }

    fn describe_pending(&self) -> Option<String> {
        let first = self.first.borrow();
        let second = self.second.borrow();
        match (first.as_ref(), second.as_ref()) {
            (Some(first), Some(second)) => Some(format!(
                "{} + {} in-tick keyed merge entr(ies) (first + second input)",
                first.len(),
                second.len()
            )),
            _ => None,
        }
    }
}
