//! Tick-input hooks ([`TickInputHook`]): the per-kind hook types that buffer input for
//! a tick across scheduling boundaries and decide, when the tick runs, what to release
//! into that execution — batch hooks ([`StreamHook`], [`KeyedStreamHook`]) and snapshot
//! hooks ([`SingletonHook`], [`KeyedSingletonHook`], and the choice-free
//! [`PassthroughSingletonHook`]). Their scripted decision/status types and
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
    HookLocationMeta, ManualDebug, RuntimeHook, ScriptDecision, ScriptableHook,
    ScriptableTickInputHook, TickInputHook, TruncatedVecDebug, abort, log_release,
};
use crate::live_collections::stream::{NoOrder, Ordering, TotalOrder};

pub struct StreamHook<T, Order: Ordering> {
    pub input: Rc<RefCell<VecDeque<T>>>,
    pub to_release: Option<Vec<T>>,
    pub output: Sender<T>,
    pub batch_location: HookLocationMeta,
    pub format_item_debug: fn(&T) -> Option<String>,
    pub _order: std::marker::PhantomData<Order>,
}

impl<T> RuntimeHook for StreamHook<T, TotalOrder> {
    fn has_pending_input(&self) -> bool {
        !self.input.borrow().is_empty()
    }

    fn only_one_possible_decision(&self) -> bool {
        // One buffered element is still a real choice: release it or not.
        self.input.borrow().is_empty()
    }

    fn release_decision(&mut self, log_writer: Option<&mut dyn std::fmt::Write>) {
        if let Some(to_release) = self.to_release.take() {
            if let Some(log_writer) = log_writer {
                let HookLocationMeta {
                    location: batch_location,
                    line,
                    caret_indent,
                } = self.batch_location;
                let note_str = if to_release.is_empty() {
                    "^ releasing no items".to_owned()
                } else {
                    format!(
                        "^ releasing items: {:?}",
                        TruncatedVecDebug(
                            RefCell::new(Some(to_release.iter())),
                            8,
                            self.format_item_debug
                        )
                    )
                };

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
        self.batch_location
    }
}

impl<T> TickInputHook for StreamHook<T, TotalOrder> {
    fn can_trigger_tick(&self) -> bool {
        !self.input.borrow().is_empty()
    }

    fn autonomous_decision<'a>(&mut self, driver: &mut Borrowed<'a>, force_trigger: bool) -> bool {
        let mut current_input = self.input.borrow_mut();
        let count = ((if force_trigger { 1 } else { 0 })..=current_input.len())
            .generate(driver)
            .unwrap();

        self.to_release = Some(current_input.drain(0..count).collect());
        count > 0
    }
}

impl<T> RuntimeHook for StreamHook<T, NoOrder> {
    fn has_pending_input(&self) -> bool {
        !self.input.borrow().is_empty()
    }

    fn only_one_possible_decision(&self) -> bool {
        // One buffered element is still a real choice: release it or not.
        self.input.borrow().is_empty()
    }

    fn release_decision(&mut self, log_writer: Option<&mut dyn std::fmt::Write>) {
        if let Some(to_release) = self.to_release.take() {
            if let Some(log_writer) = log_writer {
                let HookLocationMeta {
                    location: batch_location,
                    line,
                    caret_indent,
                } = self.batch_location;
                let note_str = if to_release.is_empty() {
                    "^ releasing no items".to_owned()
                } else {
                    format!(
                        "^ releasing unordered items: {:?}",
                        TruncatedVecDebug(
                            RefCell::new(Some(to_release.iter())),
                            8,
                            self.format_item_debug
                        )
                    )
                };

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
        self.batch_location
    }
}

impl<T> TickInputHook for StreamHook<T, NoOrder> {
    fn can_trigger_tick(&self) -> bool {
        !self.input.borrow().is_empty()
    }

    fn autonomous_decision<'a>(&mut self, driver: &mut Borrowed<'a>, force_trigger: bool) -> bool {
        let mut current_input = self.input.borrow_mut();
        let mut out = vec![];
        let mut min_index = 0;
        while !current_input.is_empty() {
            let must_release = force_trigger && out.is_empty();
            if !must_release && produce().generate(driver).unwrap() {
                break;
            }

            let idx = (min_index..current_input.len()).generate(driver).unwrap();
            let item = current_input.remove(idx).unwrap();
            out.push(item);

            min_index = idx;
            // Next time, only consider items at or after this index. The reason this is safe is
            // because batching a `NoOrder` streams results in batches with a `NoOrder` guarantee.
            // Therefore, simulating different order of elements _within_ a batch is redundant.

            if min_index == current_input.len() {
                break;
            }
        }

        let triggered = !out.is_empty();
        self.to_release = Some(out);
        triggered
    }
}

/// A scripted decision for a totally ordered batch hook.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum BatchDecision<T> {
    /// Release the next `n` buffered elements.
    Prefix(usize),
    /// Release this exact sequence of values from the front of the buffer.
    Values(Vec<T>),
    /// Release everything that has arrived by the time the tick fires.
    All,
}

impl<T> ScriptDecision for BatchDecision<T>
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    fn describe(&self) -> String {
        match self {
            BatchDecision::Prefix(n) => format!("release({})", n),
            BatchDecision::Values(values) => {
                format!("release_values({} value(s))", values.len())
            }
            BatchDecision::All => "release_all()".to_owned(),
        }
    }
}

/// A scripted decision for an unordered batch hook.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum UnorderedBatchDecision<T> {
    /// Release this multiset of values. Duplicate values name duplicate buffered items.
    Values(Vec<T>),
    /// Release everything that has arrived by the time the tick fires.
    All,
}

impl<T> ScriptDecision for UnorderedBatchDecision<T>
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    fn describe(&self) -> String {
        match self {
            UnorderedBatchDecision::Values(values) => {
                format!("release_values({} value(s))", values.len())
            }
            UnorderedBatchDecision::All => "release_all()".to_owned(),
        }
    }
}

/// The pending-input view a batch hook reports to its test-side handle (see
/// [`ScriptableHook::status`]), used by `pause_until_*` predicates.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct BatchStatus {
    /// The number of buffered elements.
    pub buffered: usize,
}

impl<T> ScriptableHook for StreamHook<T, TotalOrder>
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq,
{
    type Decision = BatchDecision<T>;
    type Status = BatchStatus;

    fn is_honorable(&self, decision: &BatchDecision<T>) -> Result<bool, String> {
        let input = self.input.borrow();
        match decision {
            BatchDecision::Prefix(n) => Ok(input.len() >= *n),
            BatchDecision::Values(values) => {
                if let Some(idx) = input
                    .iter()
                    .zip(values)
                    .position(|(buffered, expected)| buffered != expected)
                {
                    Err(format!(
                        "release_values: buffered item at prefix position {} did not match the expected value",
                        idx
                    ))
                } else {
                    Ok(input.len() >= values.len())
                }
            }
            BatchDecision::All => Ok(true),
        }
    }

    fn apply(&mut self, decision: BatchDecision<T>) {
        let mut input = self.input.borrow_mut();
        let out: Vec<T> = match decision {
            BatchDecision::Prefix(n) => input.drain(0..n).collect(),
            BatchDecision::Values(values) => input.drain(0..values.len()).collect(),
            BatchDecision::All => input.drain(..).collect(),
        };

        self.to_release = Some(out);
    }

    fn implicit(&mut self) {
        self.to_release = Some(vec![]);
    }

    fn status(&self) -> BatchStatus {
        BatchStatus {
            buffered: self.input.borrow().len(),
        }
    }

    fn describe_pending(&self) -> Option<String> {
        let input = self.input.borrow();
        (!input.is_empty()).then(|| {
            format!(
                "{} buffered item(s): {:?}",
                input.len(),
                TruncatedVecDebug(RefCell::new(Some(input.iter())), 8, self.format_item_debug)
            )
        })
    }
}

impl<T> ScriptableTickInputHook for StreamHook<T, TotalOrder>
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq,
{
    fn decision_triggers_tick(&self, decision: &BatchDecision<T>) -> bool {
        match decision {
            BatchDecision::Prefix(n) => *n > 0,
            BatchDecision::Values(values) => !values.is_empty(),
            BatchDecision::All => !self.input.borrow().is_empty(),
        }
    }
}

impl<T> ScriptableHook for StreamHook<T, NoOrder>
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq,
{
    type Decision = UnorderedBatchDecision<T>;
    type Status = BatchStatus;

    fn is_honorable(&self, decision: &UnorderedBatchDecision<T>) -> Result<bool, String> {
        Ok(match decision {
            UnorderedBatchDecision::Values(values) => {
                let mut unmatched: Vec<&T> = values.iter().collect();
                for buffered in self.input.borrow().iter() {
                    if let Some(idx) = unmatched
                        .iter()
                        .position(|requested| *requested == buffered)
                    {
                        unmatched.swap_remove(idx);
                    }
                }
                unmatched.is_empty()
            }
            UnorderedBatchDecision::All => true,
        })
    }

    fn apply(&mut self, decision: UnorderedBatchDecision<T>) {
        let mut input = self.input.borrow_mut();
        let out: Vec<T> = match decision {
            UnorderedBatchDecision::Values(mut values) => {
                let (selected, remaining): (Vec<_>, Vec<_>) =
                    input.drain(..).partition(|buffered| {
                        values
                            .iter()
                            .position(|requested| requested == buffered)
                            .is_some_and(|idx| {
                                values.swap_remove(idx);
                                true
                            })
                    });
                assert!(
                    values.is_empty(),
                    "scripted unordered batch decision was not honorable"
                );
                *input = remaining.into();
                selected
            }
            UnorderedBatchDecision::All => input.drain(..).collect(),
        };

        self.to_release = Some(out);
    }

    fn implicit(&mut self) {
        self.to_release = Some(vec![]);
    }

    fn status(&self) -> BatchStatus {
        BatchStatus {
            buffered: self.input.borrow().len(),
        }
    }

    fn describe_pending(&self) -> Option<String> {
        let input = self.input.borrow();
        (!input.is_empty()).then(|| {
            format!(
                "{} buffered item(s): {:?}",
                input.len(),
                TruncatedVecDebug(RefCell::new(Some(input.iter())), 8, self.format_item_debug)
            )
        })
    }
}

impl<T> ScriptableTickInputHook for StreamHook<T, NoOrder>
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq,
{
    fn decision_triggers_tick(&self, decision: &UnorderedBatchDecision<T>) -> bool {
        match decision {
            UnorderedBatchDecision::Values(values) => !values.is_empty(),
            UnorderedBatchDecision::All => !self.input.borrow().is_empty(),
        }
    }
}

pub struct KeyedStreamHook<K: Hash + Eq + Clone, V, Order: Ordering> {
    pub input: Rc<RefCell<FxHashMap<K, VecDeque<V>>>>, // FxHasher is deterministic
    pub to_release: Option<Vec<(K, V)>>,
    pub output: Sender<(K, V)>,
    pub batch_location: HookLocationMeta,
    pub format_item_debug: fn(&(K, V)) -> Option<String>,
    pub _order: std::marker::PhantomData<Order>,
}

impl<K: Hash + Eq + Clone, V> RuntimeHook for KeyedStreamHook<K, V, TotalOrder> {
    fn has_pending_input(&self) -> bool {
        #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
        !self.input.borrow().values().all(|q| q.is_empty())
    }

    fn only_one_possible_decision(&self) -> bool {
        // One buffered element is still a real choice: release it or not.
        #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
        self.input.borrow().values().all(|q| q.is_empty())
    }

    fn release_decision(&mut self, log_writer: Option<&mut dyn std::fmt::Write>) {
        if let Some(to_release) = self.to_release.take() {
            if let Some(log_writer) = log_writer {
                let HookLocationMeta {
                    location: batch_location,
                    line,
                    caret_indent,
                } = self.batch_location;
                let note_str = if to_release.is_empty() {
                    "^ releasing no items".to_owned()
                } else {
                    format!(
                        "^ releasing items: {:?}",
                        TruncatedVecDebug(
                            RefCell::new(Some(to_release.iter())),
                            8,
                            self.format_item_debug
                        )
                    )
                };

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
        self.batch_location
    }
}

impl<K: Hash + Eq + Clone, V> TickInputHook for KeyedStreamHook<K, V, TotalOrder> {
    fn can_trigger_tick(&self) -> bool {
        #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
        !self.input.borrow().values().all(|q| q.is_empty())
    }

    fn autonomous_decision<'a>(
        &mut self,
        driver: &mut Borrowed<'a>,
        mut force_trigger: bool,
    ) -> bool {
        let mut current_input = self.input.borrow_mut();
        self.to_release = Some(vec![]);
        #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
        let nonempty_key_count = current_input.values().filter(|q| !q.is_empty()).count();

        let mut remaining_nonempty_keys = nonempty_key_count;
        #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
        for (key, queue) in current_input.iter_mut() {
            if queue.is_empty() {
                continue;
            }

            remaining_nonempty_keys -= 1;

            let count = ((if force_trigger && remaining_nonempty_keys == 0 {
                1
            } else {
                0
            })..=queue.len())
                .generate(driver)
                .unwrap();

            let items: Vec<(K, V)> = queue.drain(0..count).map(|v| (key.clone(), v)).collect();
            self.to_release.as_mut().unwrap().extend(items);

            if count > 0 {
                force_trigger = false;
            }
        }

        !self.to_release.as_ref().unwrap().is_empty()
    }
}

impl<K: Hash + Eq + Clone, V> RuntimeHook for KeyedStreamHook<K, V, NoOrder> {
    fn has_pending_input(&self) -> bool {
        #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
        !self.input.borrow().values().all(|q| q.is_empty())
    }

    fn only_one_possible_decision(&self) -> bool {
        // One buffered element is still a real choice: release it or not.
        #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
        self.input.borrow().values().all(|q| q.is_empty())
    }

    fn release_decision(&mut self, log_writer: Option<&mut dyn std::fmt::Write>) {
        if let Some(to_release) = self.to_release.take() {
            if let Some(log_writer) = log_writer {
                let HookLocationMeta {
                    location: batch_location,
                    line,
                    caret_indent,
                } = self.batch_location;
                let note_str = if to_release.is_empty() {
                    "^ releasing no items".to_owned()
                } else {
                    format!(
                        "^ releasing unordered items: {:?}",
                        TruncatedVecDebug(
                            RefCell::new(Some(to_release.iter())),
                            8,
                            self.format_item_debug
                        )
                    )
                };

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
        self.batch_location
    }
}

impl<K: Hash + Eq + Clone, V> TickInputHook for KeyedStreamHook<K, V, NoOrder> {
    fn can_trigger_tick(&self) -> bool {
        #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
        !self.input.borrow().values().all(|q| q.is_empty())
    }

    fn autonomous_decision<'a>(
        &mut self,
        driver: &mut Borrowed<'a>,
        mut force_trigger: bool,
    ) -> bool {
        let mut current_input = self.input.borrow_mut();
        self.to_release = Some(vec![]);
        #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
        let nonempty_key_count = current_input.values().filter(|q| !q.is_empty()).count();

        let mut remaining_nonempty_keys = nonempty_key_count;
        #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
        for (key, queue) in current_input.iter_mut() {
            if queue.is_empty() {
                continue;
            }

            remaining_nonempty_keys -= 1;

            let mut min_index = 0;
            while !queue.is_empty() {
                let must_release = force_trigger && remaining_nonempty_keys == 0;
                if !must_release && produce().generate(driver).unwrap() {
                    break;
                }

                let idx = (min_index..queue.len()).generate(driver).unwrap();
                let item = queue.remove(idx).unwrap();
                self.to_release.as_mut().unwrap().push((key.clone(), item));
                force_trigger = false;

                min_index = idx;
                // Next time, only consider items at or after this index. The reason this is safe is
                // because batching a `NoOrder` stream results in batches with a `NoOrder` guarantee.
                // Therefore, simulating different order of elements _within_ a batch is redundant.

                if min_index == queue.len() {
                    break;
                }
            }
        }

        !self.to_release.as_ref().unwrap().is_empty()
    }
}

pub struct SingletonHook<T> {
    input: Rc<RefCell<VecDeque<T>>>,
    to_release: Option<(T, bool)>, // (data, is new)
    last_released: Option<T>,
    skipped_states: Vec<T>,
    output: Sender<T>,
    batch_location: HookLocationMeta,
    format_item_debug: fn(&T) -> Option<String>,
}

impl<T: Clone> SingletonHook<T> {
    pub fn new(
        input: Rc<RefCell<VecDeque<T>>>,
        output: Sender<T>,
        batch_location: HookLocationMeta,
        format_item_debug: fn(&T) -> Option<String>,
    ) -> Self {
        Self {
            input,
            to_release: None,
            last_released: None,
            skipped_states: vec![],
            output,
            batch_location,
            format_item_debug,
        }
    }
}

impl<T: Clone> RuntimeHook for SingletonHook<T> {
    fn has_pending_input(&self) -> bool {
        !self.input.borrow().is_empty()
    }

    fn only_one_possible_decision(&self) -> bool {
        // With no previously revealed value, a sole buffered version has exactly one
        // resolution: reveal it (there is nothing to keep). Once a value has been
        // revealed, any buffered version is a real choice (keep vs reveal); with two or
        // more buffered versions the choice of which to reveal is real either way.
        let input = self.input.borrow();
        input.is_empty() || (input.len() == 1 && self.last_released.is_none())
    }

    fn release_decision(&mut self, log_writer: Option<&mut dyn std::fmt::Write>) {
        if let Some((to_release, is_new)) = self.to_release.take() {
            self.last_released = Some(to_release.clone());

            if let Some(log_writer) = log_writer {
                let HookLocationMeta {
                    location: batch_location,
                    line,
                    caret_indent,
                } = self.batch_location;
                let note_str = if self.skipped_states.is_empty() {
                    if is_new {
                        format!(
                            "^ releasing snapshot: {:?}",
                            ManualDebug(&to_release, self.format_item_debug)
                        )
                    } else {
                        format!(
                            "^ releasing unchanged snapshot: {:?}",
                            ManualDebug(&to_release, self.format_item_debug)
                        )
                    }
                } else {
                    format!(
                        "^ releasing snapshot: {:?} (skipping earlier states: {:?})",
                        ManualDebug(&to_release, self.format_item_debug),
                        self.skipped_states
                            .iter()
                            .map(|s| ManualDebug(s, self.format_item_debug))
                            .collect::<Vec<_>>()
                    )
                };

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

    fn is_ready(&self) -> bool {
        !self.input.borrow().is_empty() || self.last_released.is_some()
    }

    fn location_meta(&self) -> HookLocationMeta {
        self.batch_location
    }
}

impl<T: Clone> TickInputHook for SingletonHook<T> {
    fn can_trigger_tick(&self) -> bool {
        !self.input.borrow().is_empty()
    }

    fn autonomous_decision<'a>(&mut self, driver: &mut Borrowed<'a>, force_trigger: bool) -> bool {
        let mut current_input = self.input.borrow_mut();
        if current_input.is_empty() {
            if force_trigger {
                panic!("Cannot make a triggering decision when there is no input");
            }

            if let Some(last) = &self.last_released {
                // Re-release the last item
                self.to_release = Some((last.clone(), false));
                false
            } else {
                panic!("No input and no last released item to re-release");
            }
        } else if !force_trigger
            && let Some(last) = &self.last_released
            && produce().generate(driver).unwrap()
        {
            // Re-release the last item
            self.to_release = Some((last.clone(), false));
            false
        } else {
            // Release a new item
            let idx_to_release = (0..current_input.len()).generate(driver).unwrap();
            self.skipped_states = current_input.drain(0..idx_to_release).collect(); // Drop earlier items
            let item = current_input.pop_front().unwrap();
            self.to_release = Some((item, true));
            true
        }
    }
}

/// A scripted decision for a snapshot hook: which buffered version of the state the next
/// tick execution observes.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum SnapshotDecision<T> {
    /// Reveal the first buffered version equal to this value, skipping over earlier
    /// versions.
    Reveal(T),
    /// Advance to the next buffered version.
    RevealNext,
    /// Reveal the newest version that has arrived by the time the tick fires.
    RevealLatest,
    /// Observe the previously revealed version again.
    Keep,
}

impl<T> ScriptDecision for SnapshotDecision<T>
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    fn describe(&self) -> String {
        match self {
            SnapshotDecision::Reveal(_) => "reveal(..)".to_owned(),
            SnapshotDecision::RevealNext => "reveal_next()".to_owned(),
            SnapshotDecision::RevealLatest => "reveal_latest()".to_owned(),
            SnapshotDecision::Keep => "keep()".to_owned(),
        }
    }
}

/// The pending-input view a snapshot hook reports to its test-side handle (see
/// [`ScriptableHook::status`]), used by `pause_until_*` predicates.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SnapshotStatus {
    /// The number of buffered versions newer than the last revealed one.
    pub newer_versions: usize,
}

impl<T: Clone + PartialEq + serde::Serialize + serde::de::DeserializeOwned> ScriptableHook
    for SingletonHook<T>
{
    type Decision = SnapshotDecision<T>;
    type Status = SnapshotStatus;

    fn is_honorable(&self, decision: &SnapshotDecision<T>) -> Result<bool, String> {
        let input = self.input.borrow();
        Ok(match decision {
            SnapshotDecision::Reveal(target) => input.iter().any(|version| version == target),
            SnapshotDecision::RevealNext => !input.is_empty(),
            SnapshotDecision::RevealLatest => !input.is_empty() || self.last_released.is_some(),
            SnapshotDecision::Keep => self.last_released.is_some(),
        })
    }

    fn apply(&mut self, decision: SnapshotDecision<T>) {
        match decision {
            SnapshotDecision::Reveal(target) => {
                let mut input = self.input.borrow_mut();
                let idx = input.iter().position(|version| *version == target).unwrap();
                self.skipped_states = input.drain(0..idx).collect();
                let item = input.pop_front().unwrap();
                self.to_release = Some((item, true));
            }
            SnapshotDecision::RevealNext => {
                let mut input = self.input.borrow_mut();
                self.skipped_states = vec![];
                let item = input.pop_front().unwrap();
                self.to_release = Some((item, true));
            }
            SnapshotDecision::RevealLatest => {
                let mut input = self.input.borrow_mut();
                if input.is_empty() {
                    self.skipped_states = vec![];
                    self.to_release = Some((self.last_released.clone().unwrap(), false));
                } else {
                    let skip_count = input.len() - 1;
                    self.skipped_states = input.drain(0..skip_count).collect();
                    let item = input.pop_front().unwrap();
                    self.to_release = Some((item, true));
                }
            }
            SnapshotDecision::Keep => {
                self.skipped_states = vec![];
                self.to_release = Some((self.last_released.clone().unwrap(), false));
            }
        }
    }

    fn implicit(&mut self) {
        if let Some(last) = &self.last_released {
            self.skipped_states = vec![];
            self.to_release = Some((last.clone(), false));
        } else {
            // `is_ready()` prevents the tick from running before the singleton has a
            // value, so this is unreachable.
            abort!("scripted snapshot hook asked for implicit behavior with no revealed value");
        }
    }

    fn status(&self) -> SnapshotStatus {
        SnapshotStatus {
            newer_versions: self.input.borrow().len(),
        }
    }

    fn describe_pending(&self) -> Option<String> {
        let input = self.input.borrow();
        (!input.is_empty()).then(|| {
            format!(
                "{} buffered version(s): {:?}",
                input.len(),
                TruncatedVecDebug(RefCell::new(Some(input.iter())), 8, self.format_item_debug)
            )
        })
    }
}

impl<T: Clone + PartialEq + serde::Serialize + serde::de::DeserializeOwned> ScriptableTickInputHook
    for SingletonHook<T>
{
    fn decision_triggers_tick(&self, decision: &SnapshotDecision<T>) -> bool {
        match decision {
            SnapshotDecision::Reveal(_) | SnapshotDecision::RevealNext => true,
            SnapshotDecision::RevealLatest => !self.input.borrow().is_empty(),
            SnapshotDecision::Keep => false,
        }
    }
}
/// A passthrough singleton hook for fold outputs that are already controlled by a
/// `TopLevelFoldHook`. Always releases the latest value without any non-deterministic
/// decisions, since the fold hook already made the only meaningful choice (which subset
/// of inputs to process).
pub struct PassthroughSingletonHook<T> {
    input: Rc<RefCell<VecDeque<T>>>,
    to_release: Option<T>,
    output: Sender<T>,
    batch_location: HookLocationMeta,
    format_item_debug: fn(&T) -> Option<String>,
}

impl<T> PassthroughSingletonHook<T> {
    pub fn new(
        input: Rc<RefCell<VecDeque<T>>>,
        output: Sender<T>,
        batch_location: HookLocationMeta,
        format_item_debug: fn(&T) -> Option<String>,
    ) -> Self {
        Self {
            input,
            to_release: None,
            output,
            batch_location,
            format_item_debug,
        }
    }
}

impl<T> RuntimeHook for PassthroughSingletonHook<T> {
    fn has_pending_input(&self) -> bool {
        !self.input.borrow().is_empty()
    }

    fn only_one_possible_decision(&self) -> bool {
        // Releasing the latest value is the only behavior this hook ever has: the
        // controlling `TopLevelFoldHook` already made every meaningful choice, so even
        // with buffered input there is nothing non-deterministic left to decide. (This
        // hook is why the choice question is separate from `can_trigger_tick`.)
        true
    }

    fn release_decision(&mut self, log_writer: Option<&mut dyn std::fmt::Write>) {
        if let Some(to_release) = self.to_release.take() {
            if let Some(log_writer) = log_writer {
                let HookLocationMeta {
                    location: batch_location,
                    line,
                    caret_indent,
                } = self.batch_location;
                let note_str = format!(
                    "^ releasing snapshot: {:?}",
                    ManualDebug(&to_release, self.format_item_debug)
                );

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
        self.batch_location
    }
}

impl<T> TickInputHook for PassthroughSingletonHook<T> {
    fn can_trigger_tick(&self) -> bool {
        !self.input.borrow().is_empty()
    }

    fn autonomous_decision<'a>(
        &mut self,
        _driver: &mut Borrowed<'a>,
        _force_trigger: bool,
    ) -> bool {
        let mut current_input = self.input.borrow_mut();
        // Always take the last (most recent) value, discard intermediates.
        if let Some(item) = current_input.pop_back() {
            current_input.clear();
            self.to_release = Some(item);
            true
        } else {
            false
        }
    }
}

pub struct KeyedSingletonHook<K: Hash + Eq + Clone, V: Clone> {
    input: Rc<RefCell<FxHashMap<K, VecDeque<V>>>>, // FxHasher is deterministic
    to_release: Option<Vec<(K, V, bool)>>,         // (key, data, is new)
    last_released: FxHashMap<K, V>,
    skipped_states: FxHashMap<K, Vec<V>>,
    output: Sender<(K, V)>,
    batch_location: HookLocationMeta,
    format_key_debug: fn(&K) -> Option<String>,
    format_value_debug: fn(&V) -> Option<String>,
}

impl<K: Hash + Eq + Clone, V: Clone> KeyedSingletonHook<K, V> {
    pub fn new(
        input: Rc<RefCell<FxHashMap<K, VecDeque<V>>>>,
        output: Sender<(K, V)>,
        batch_location: HookLocationMeta,
        format_key_debug: fn(&K) -> Option<String>,
        format_value_debug: fn(&V) -> Option<String>,
    ) -> Self {
        Self {
            input,
            to_release: None,
            last_released: FxHashMap::default(),
            skipped_states: FxHashMap::default(),
            output,
            batch_location,
            format_key_debug,
            format_value_debug,
        }
    }
}

impl<K: Hash + Eq + Clone, V: Clone> RuntimeHook for KeyedSingletonHook<K, V> {
    fn has_pending_input(&self) -> bool {
        #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
        !self.input.borrow().values().all(|q| q.is_empty())
    }

    fn only_one_possible_decision(&self) -> bool {
        // Even a sole buffered version for a key admits two resolutions (a key not yet
        // in the snapshot may stay withheld; a key with a previous value may keep it).
        #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
        self.input.borrow().values().all(|q| q.is_empty())
    }

    fn release_decision(&mut self, log_writer: Option<&mut dyn std::fmt::Write>) {
        if let Some(to_release) = self.to_release.take() {
            if let Some(log_writer) = log_writer {
                let HookLocationMeta {
                    location: batch_location,
                    line,
                    caret_indent,
                } = self.batch_location;
                let note_str = if to_release.is_empty() {
                    "^ releasing no items".to_owned()
                } else {
                    let mut mapping_text = String::new();
                    for (key, value, is_new) in &to_release {
                        let entry_text = if *is_new {
                            format!(
                                "{:?}: {:?}",
                                ManualDebug(key, self.format_key_debug),
                                ManualDebug(value, self.format_value_debug)
                            )
                        } else {
                            format!(
                                "{:?}: {:?} (unchanged)",
                                ManualDebug(key, self.format_key_debug),
                                ManualDebug(value, self.format_value_debug)
                            )
                        };
                        if !mapping_text.is_empty() {
                            mapping_text.push_str(", ");
                        }
                        mapping_text.push_str(&entry_text);
                    }
                    format!("^ releasing items: {{ {} }}", mapping_text)
                };

                log_release(
                    log_writer,
                    batch_location,
                    line,
                    caret_indent,
                    &note_str,
                    colored::Color::Green,
                );
            }

            for (key, value, _) in to_release {
                self.output.try_send((key, value)).unwrap();
            }
        } else {
            panic!("No decision to release");
        }
    }

    fn location_meta(&self) -> HookLocationMeta {
        self.batch_location
    }
}

impl<K: Hash + Eq + Clone, V: Clone> TickInputHook for KeyedSingletonHook<K, V> {
    fn can_trigger_tick(&self) -> bool {
        #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
        !self.input.borrow().values().all(|q| q.is_empty())
    }

    fn autonomous_decision<'a>(
        &mut self,
        driver: &mut Borrowed<'a>,
        mut force_trigger: bool,
    ) -> bool {
        let mut current_input = self.input.borrow_mut();
        self.to_release = Some(vec![]);
        #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
        let nonempty_key_count = current_input.values().filter(|q| !q.is_empty()).count();

        let mut remaining_nonempty_keys = nonempty_key_count;
        let mut any_triggered = false;
        #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
        for (key, queue) in current_input.iter_mut() {
            if queue.is_empty() {
                self.to_release.as_mut().unwrap().push((
                    key.clone(),
                    self.last_released.get(key).unwrap().clone(),
                    false,
                ));

                continue;
            }

            remaining_nonempty_keys -= 1;

            let must_reveal = force_trigger && remaining_nonempty_keys == 0;

            if !must_reveal
                && self.last_released.contains_key(key)
                && produce().generate(driver).unwrap()
            {
                // Re-release the last item for this key
                let last = self.last_released.get(key).unwrap().clone();
                self.to_release
                    .as_mut()
                    .unwrap()
                    .push((key.clone(), last, false));
            } else {
                let allow_null_release = !must_reveal && !self.last_released.contains_key(key);
                if allow_null_release && produce().generate(driver).unwrap() {
                    // Don't emit anything, this key is not yet added to the snapshot
                    continue;
                } else {
                    // Release a new item for this key
                    let idx_to_release = (0..queue.len()).generate(driver).unwrap();
                    let skipped: Vec<V> = queue.drain(0..idx_to_release).collect();
                    let item = queue.pop_front().unwrap();
                    self.skipped_states.insert(key.clone(), skipped);
                    self.to_release
                        .as_mut()
                        .unwrap()
                        .push((key.clone(), item.clone(), true));
                    self.last_released.insert(key.clone(), item);

                    any_triggered |= true;
                    force_trigger = false;
                }
            }
        }

        any_triggered
    }
}
