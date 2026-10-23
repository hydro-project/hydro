//! Tick-input hooks ([`TickInputHook`]): the per-kind hook types that buffer input for
//! a tick across scheduling boundaries and decide, when the tick runs, what to release
//! into that execution: batch hooks ([`StreamHook`], [`KeyedStreamHook`]) and snapshot
//! hooks ([`SingletonHook`], [`KeyedSingletonHook`], and the choice-free
//! [`PassthroughSingletonHook`]). Their scripted decision/status types and
//! [`ScriptableHook`] impls live alongside them.
//!
//! The batch hooks are generic over the stream's retries guarantee: for an `AtLeastOnce`
//! stream a release may *keep* released elements buffered, so they are delivered again in
//! a later batch (a cross-tick redelivery), spending the hook's [`RetryBudget`] when
//! autonomous. See [`StreamHook`].

use std::cell::RefCell;
use std::collections::VecDeque;
use std::hash::Hash;
use std::marker::PhantomData;
use std::rc::Rc;

use bolero::generator::bolero_generator::driver::object::Borrowed;
use bolero::{ValueGenerator, produce};
use dfir_rs::rustc_hash::{FxHashMap, FxHashSet};
use dfir_rs::util::unsync::mpsc::Sender;

use super::{
    HookLocationMeta, ManualDebug, RetryBudget, RuntimeHook, ScriptDecision, ScriptableHook,
    ScriptableTickInputHook, TickInputHook, TruncatedVecDebug, describe_keyed_pending,
    keyed_buffer_len, log_release,
};
use crate::compile::ir::{StreamOrder, StreamRetry};
use crate::live_collections::stream::{
    AtLeastOnce, ExactlyOnce, NoOrder, Ordering, Retries, TotalOrder,
};

/// The per-retries-guarantee behavior of the batch hooks: whether, and how, a released
/// element may be *kept* buffered for redelivery in a later batch. Exactly-once streams
/// never keep (and need no `Clone`); at-least-once streams keep by cloning, spending retry
/// budget when autonomous.
trait BatchRetries<T>: Retries {
    /// Autonomously decides whether to keep `item`, returning the copy to release if so
    /// (and spending one unit of `budget`).
    fn autonomous_keep(budget: &mut RetryBudget, item: &T, driver: &mut Borrowed<'_>) -> Option<T>;

    /// The copy released for a scripted keep decision.
    fn scripted_keep(item: &T) -> T;
}

impl<T> BatchRetries<T> for ExactlyOnce {
    fn autonomous_keep(_: &mut RetryBudget, _: &T, _: &mut Borrowed<'_>) -> Option<T> {
        None
    }

    fn scripted_keep(_: &T) -> T {
        // The exactly-once handle types offer no keep decisions, and `is_honorable`
        // rejects one that arrives anyway, so this is unreachable.
        abort!("a keep decision was applied to an exactly-once batch hook")
    }
}

impl<T: Clone> BatchRetries<T> for AtLeastOnce {
    fn autonomous_keep(budget: &mut RetryBudget, item: &T, driver: &mut Borrowed<'_>) -> Option<T> {
        if budget.allows_retry() && produce::<bool>().generate(driver).unwrap() {
            budget.spend(1);
            Some(item.clone())
        } else {
            None
        }
    }

    fn scripted_keep(item: &T) -> T {
        item.clone()
    }
}

/// Rejects a scripted keep decision on an exactly-once hook (the handle types make this
/// unreachable; kept as a defensive permanent error).
fn check_keep_allowed<R: Retries>(method: &str) -> Result<(), String> {
    if R::RETRIES_KIND == StreamRetry::ExactlyOnce {
        Err(format!(
            "{method}: only an at-least-once stream may keep released elements for redelivery"
        ))
    } else {
        Ok(())
    }
}

/// The adjective the release log uses for a batch of the given order.
fn order_label<O: Ordering>() -> &'static str {
    match O::ORDERING_KIND {
        StreamOrder::TotalOrder => "",
        StreamOrder::NoOrder => "unordered ",
    }
}

/// Logs and sends a batch hook's staged release: the shared body of the batch hooks'
/// [`RuntimeHook::release_decision`]. `kept` is how many released items stay buffered for
/// redelivery.
fn release_batch<T>(
    to_release: Option<Vec<T>>,
    kept: usize,
    label: &str,
    batch_location: HookLocationMeta,
    format_item_debug: fn(&T) -> Option<String>,
    output: &Sender<T>,
    log_writer: Option<&mut dyn std::fmt::Write>,
) {
    let Some(to_release) = to_release else {
        panic!("No decision to release");
    };
    if let Some(log_writer) = log_writer {
        let HookLocationMeta {
            location,
            line,
            caret_indent,
        } = batch_location;
        let note_str = if to_release.is_empty() {
            "^ releasing no items".to_owned()
        } else {
            format!(
                "^ releasing {}items: {:?}{}",
                label,
                TruncatedVecDebug(RefCell::new(Some(to_release.iter())), 8, format_item_debug),
                if kept > 0 {
                    format!(" ({} kept for redelivery)", kept)
                } else {
                    String::new()
                }
            )
        };

        log_release(
            log_writer,
            location,
            line,
            caret_indent,
            &note_str,
            colored::Color::Green,
        );
    }

    for item in to_release {
        output.try_send(item).unwrap();
    }
}

/// Batch hook for a stream: buffers the stream's elements and decides what each tick
/// execution receives (a prefix for a `TotalOrder` stream, any subset for `NoOrder`).
///
/// For an **`AtLeastOnce`** stream an element may be delivered in **several batches**: a
/// release may *keep* released elements buffered, so they are delivered again in a later
/// batch. This is the point where the simulator injects cross-tick redeliveries of an
/// at-least-once stream (in-batch duplicates and redelivery order are the in-tick
/// `assume_retries` / `assume_ordering` observations' dimensions).
///
/// - **`TotalOrder`**: only the **last** element of the released prefix may be kept, since a
///   redelivery must stay in place (adjacent to its previous delivery); the kept element
///   is the front of the buffer and necessarily the first element of the next non-empty
///   batch.
/// - **`NoOrder`**: any released element may be kept for some later batch.
///
/// Autonomously, each kept element spends one unit of the hook's (bounded) run-wide
/// [`RetryBudget`]; scripted, the keep decision forms are explicit. A kept element is
/// ordinary pending input: a scripted hook must release it again (or pause) like anything
/// else buffered. Exactly-once hooks never keep.
pub struct StreamHook<T, Order: Ordering, R: Retries> {
    input: Rc<RefCell<VecDeque<T>>>,
    to_release: Option<Vec<T>>,
    /// How many elements of the staged release stay buffered for redelivery (noted in the
    /// release log); always `0` for an exactly-once stream.
    kept_in_release: usize,
    /// How redeliveries are decided (see [`RetryBudget`]); never consulted for an
    /// exactly-once stream.
    budget: RetryBudget,
    output: Sender<T>,
    batch_location: HookLocationMeta,
    format_item_debug: fn(&T) -> Option<String>,
    _marker: PhantomData<(Order, R)>,
}

impl<T, Order: Ordering> StreamHook<T, Order, ExactlyOnce> {
    pub fn new(
        input: Rc<RefCell<VecDeque<T>>>,
        output: Sender<T>,
        batch_location: HookLocationMeta,
        format_item_debug: fn(&T) -> Option<String>,
    ) -> Self {
        Self {
            input,
            to_release: None,
            kept_in_release: 0,
            budget: RetryBudget::Bounded(0),
            output,
            batch_location,
            format_item_debug,
            _marker: PhantomData,
        }
    }
}

impl<T, Order: Ordering> StreamHook<T, Order, AtLeastOnce> {
    pub fn new(
        input: Rc<RefCell<VecDeque<T>>>,
        output: Sender<T>,
        batch_location: HookLocationMeta,
        format_item_debug: fn(&T) -> Option<String>,
        budget: RetryBudget,
    ) -> Self {
        Self {
            input,
            to_release: None,
            kept_in_release: 0,
            budget,
            output,
            batch_location,
            format_item_debug,
            _marker: PhantomData,
        }
    }
}

impl<T, Order: Ordering, R: Retries> RuntimeHook for StreamHook<T, Order, R> {
    fn has_pending_input(&self) -> bool {
        !self.input.borrow().is_empty()
    }

    fn only_one_possible_decision(&self) -> bool {
        // One buffered element is still a real choice: release it or not.
        self.input.borrow().is_empty()
    }

    fn release_decision(&mut self, log_writer: Option<&mut dyn std::fmt::Write>) {
        release_batch(
            self.to_release.take(),
            std::mem::take(&mut self.kept_in_release),
            order_label::<Order>(),
            self.batch_location,
            self.format_item_debug,
            &self.output,
            log_writer,
        );
    }

    fn location_meta(&self) -> HookLocationMeta {
        self.batch_location
    }
}

impl<T, R: BatchRetries<T>> TickInputHook for StreamHook<T, TotalOrder, R> {
    fn can_trigger_tick(&self) -> bool {
        !self.input.borrow().is_empty()
    }

    fn autonomous_decision<'a>(&mut self, driver: &mut Borrowed<'a>, force_trigger: bool) -> bool {
        let mut input = self.input.borrow_mut();
        let count = (usize::from(force_trigger)..=input.len())
            .generate(driver)
            .unwrap();

        let mut out: Vec<T> = input.drain(0..count.saturating_sub(1)).collect();
        if count > 0 {
            // The trailing element may be kept for redelivery at the start of a later
            // batch (at-least-once only).
            match R::autonomous_keep(&mut self.budget, input.front().unwrap(), driver) {
                Some(copy) => {
                    out.push(copy);
                    self.kept_in_release = 1;
                }
                None => out.push(input.pop_front().unwrap()),
            }
        }
        self.to_release = Some(out);
        count > 0
    }
}

impl<T, R: BatchRetries<T>> TickInputHook for StreamHook<T, NoOrder, R> {
    fn can_trigger_tick(&self) -> bool {
        !self.input.borrow().is_empty()
    }

    fn autonomous_decision<'a>(&mut self, driver: &mut Borrowed<'a>, force_trigger: bool) -> bool {
        let mut input = self.input.borrow_mut();
        let mut out = vec![];
        let mut kept = 0;
        let mut min_index = 0;
        while min_index < input.len() {
            let must_release = force_trigger && out.is_empty();
            if !must_release && produce().generate(driver).unwrap() {
                break;
            }

            // Only items at or after the previously chosen one are considered next:
            // batching a `NoOrder` stream yields `NoOrder` batches, so different orders
            // *within* a batch are redundant.
            let idx = (min_index..input.len()).generate(driver).unwrap();
            // A released element may be kept buffered for a later batch (at-least-once
            // only); it then stays in place and is skipped for the rest of this batch.
            if let Some(copy) = R::autonomous_keep(&mut self.budget, &input[idx], driver) {
                out.push(copy);
                kept += 1;
                min_index = idx + 1;
            } else {
                out.push(input.remove(idx).unwrap());
                min_index = idx;
            }
        }

        self.kept_in_release = kept;
        let triggered = !out.is_empty();
        self.to_release = Some(out);
        triggered
    }
}

/// A scripted decision for a totally ordered batch hook. The keep forms apply to
/// `AtLeastOnce` streams only (see [`StreamHook`]).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum BatchDecision<T> {
    /// Release the next `n` buffered elements.
    Prefix(usize),
    /// Release this exact sequence of values from the front of the buffer.
    Values(Vec<T>),
    /// Release everything that has arrived by the time the tick fires.
    All,
    /// Release the next `n >= 1` buffered elements, keeping the last one buffered: it is
    /// redelivered as the first element of a later batch.
    PrefixKeepLast(usize),
    /// Release this exact (non-empty) sequence of values from the front of the buffer,
    /// keeping the last one buffered for redelivery.
    ValuesKeepLast(Vec<T>),
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
            BatchDecision::PrefixKeepLast(n) => format!("release_keep_last({})", n),
            BatchDecision::ValuesKeepLast(values) => {
                format!("release_values_keep_last({} value(s))", values.len())
            }
        }
    }
}

/// A scripted decision for an unordered batch hook. The keep form applies to
/// `AtLeastOnce` streams only (see [`StreamHook`]).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum UnorderedBatchDecision<T> {
    /// Release this multiset of values. Duplicate values name duplicate buffered items.
    Values(Vec<T>),
    /// Release everything that has arrived by the time the tick fires.
    All,
    /// Release this multiset of values, keeping the second multiset (a sub-multiset of
    /// the first) buffered for redelivery in later batches.
    ValuesKeep(Vec<T>, Vec<T>),
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
            UnorderedBatchDecision::ValuesKeep(values, keep) => format!(
                "release_values_keep({} value(s), {} kept)",
                values.len(),
                keep.len()
            ),
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

/// Checks that `values` matches the buffered prefix (`Err` on a contradiction, `Ok(false)`
/// while the prefix is still incomplete).
fn ordered_prefix_honorable<T: PartialEq>(
    input: &VecDeque<T>,
    values: &[T],
    method: &str,
) -> Result<bool, String> {
    if let Some(idx) = input
        .iter()
        .zip(values)
        .position(|(buffered, expected)| buffered != expected)
    {
        Err(format!(
            "{}: buffered item at prefix position {} did not match the expected value",
            method, idx
        ))
    } else {
        Ok(input.len() >= values.len())
    }
}

/// Whether every value of `requested` has a distinct match in `available` (a multiset
/// inclusion check).
fn multiset_included<'a, T: PartialEq + 'a>(
    requested: &[T],
    available: impl Iterator<Item = &'a T>,
) -> bool {
    let mut unmatched: Vec<&T> = requested.iter().collect();
    for item in available {
        if let Some(idx) = unmatched.iter().position(|r| *r == item) {
            unmatched.swap_remove(idx);
        }
    }
    unmatched.is_empty()
}

impl<T, R: BatchRetries<T>> ScriptableHook for StreamHook<T, TotalOrder, R>
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
                ordered_prefix_honorable(&input, values, "release_values")
            }
            BatchDecision::All => Ok(true),
            BatchDecision::PrefixKeepLast(n) => {
                check_keep_allowed::<R>("release_keep_last")?;
                if *n == 0 {
                    // The handle asserts this at the scripting call site; kept as a
                    // defensive permanent error.
                    return Err(
                        "release_keep_last: at least one element must be released to keep its last"
                            .to_owned(),
                    );
                }
                Ok(input.len() >= *n)
            }
            BatchDecision::ValuesKeepLast(values) => {
                check_keep_allowed::<R>("release_values_keep_last")?;
                if values.is_empty() {
                    return Err(
                        "release_values_keep_last: at least one value must be released to keep its last"
                            .to_owned(),
                    );
                }
                ordered_prefix_honorable(&input, values, "release_values_keep_last")
            }
        }
    }

    fn apply(&mut self, decision: BatchDecision<T>) {
        let mut input = self.input.borrow_mut();
        let keep_last = matches!(
            decision,
            BatchDecision::PrefixKeepLast(_) | BatchDecision::ValuesKeepLast(_)
        );
        let count = match decision {
            BatchDecision::Prefix(n) | BatchDecision::PrefixKeepLast(n) => n,
            BatchDecision::Values(values) | BatchDecision::ValuesKeepLast(values) => values.len(),
            BatchDecision::All => input.len(),
        };
        let mut out: Vec<T> = input.drain(0..count.saturating_sub(1)).collect();
        if count > 0 {
            if keep_last {
                out.push(R::scripted_keep(input.front().unwrap()));
                self.kept_in_release = 1;
            } else {
                out.push(input.pop_front().unwrap());
            }
        }
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

impl<T, R: BatchRetries<T>> ScriptableTickInputHook for StreamHook<T, TotalOrder, R>
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq,
{
    fn decision_triggers_tick(&self, decision: &BatchDecision<T>) -> bool {
        match decision {
            BatchDecision::Prefix(n) | BatchDecision::PrefixKeepLast(n) => *n > 0,
            BatchDecision::Values(values) | BatchDecision::ValuesKeepLast(values) => {
                !values.is_empty()
            }
            BatchDecision::All => !self.input.borrow().is_empty(),
        }
    }
}

impl<T, R: BatchRetries<T>> ScriptableHook for StreamHook<T, NoOrder, R>
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq,
{
    type Decision = UnorderedBatchDecision<T>;
    type Status = BatchStatus;

    fn is_honorable(&self, decision: &UnorderedBatchDecision<T>) -> Result<bool, String> {
        match decision {
            UnorderedBatchDecision::Values(values) => {
                Ok(multiset_included(values, self.input.borrow().iter()))
            }
            UnorderedBatchDecision::All => Ok(true),
            UnorderedBatchDecision::ValuesKeep(values, keep) => {
                check_keep_allowed::<R>("release_values_keep")?;
                if !multiset_included(keep, values.iter()) {
                    return Err(
                        "release_values_keep: every kept value must be among the released values"
                            .to_owned(),
                    );
                }
                Ok(multiset_included(values, self.input.borrow().iter()))
            }
        }
    }

    fn apply(&mut self, decision: UnorderedBatchDecision<T>) {
        let mut input = self.input.borrow_mut();
        let (mut values, mut keep) = match decision {
            UnorderedBatchDecision::All => {
                self.to_release = Some(input.drain(..).collect());
                return;
            }
            UnorderedBatchDecision::Values(values) => (values, vec![]),
            UnorderedBatchDecision::ValuesKeep(values, keep) => (values, keep),
        };

        // Match requested values against the buffer in buffer order; a matched item is
        // kept (copied, left in place) if it also matches a kept value, else removed.
        let mut out = vec![];
        let mut remaining = VecDeque::with_capacity(input.len());
        let mut kept = 0;
        for buffered in input.drain(..) {
            if let Some(idx) = values.iter().position(|requested| *requested == buffered) {
                values.swap_remove(idx);
                if let Some(keep_idx) = keep.iter().position(|k| *k == buffered) {
                    keep.swap_remove(keep_idx);
                    kept += 1;
                    out.push(R::scripted_keep(&buffered));
                    remaining.push_back(buffered);
                } else {
                    out.push(buffered);
                }
            } else {
                remaining.push_back(buffered);
            }
        }
        if !(values.is_empty() && keep.is_empty()) {
            abort!("scripted unordered batch decision was not honorable");
        }
        *input = remaining;
        self.kept_in_release = kept;
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

impl<T, R: BatchRetries<T>> ScriptableTickInputHook for StreamHook<T, NoOrder, R>
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq,
{
    fn decision_triggers_tick(&self, decision: &UnorderedBatchDecision<T>) -> bool {
        match decision {
            UnorderedBatchDecision::Values(values)
            | UnorderedBatchDecision::ValuesKeep(values, _) => !values.is_empty(),
            UnorderedBatchDecision::All => !self.input.borrow().is_empty(),
        }
    }
}

/// Batch hook for a **`KeyedStream`**: the keyed counterpart of [`StreamHook`]. Buffers
/// values per key and decides, per key, what each tick execution receives (a prefix of
/// the key's queue for `TotalOrder` values, any subset for `NoOrder`). For `AtLeastOnce`
/// values a release may keep released values for redelivery exactly as for
/// [`StreamHook`], per key: with `TotalOrder` values only the last value of each key's
/// released prefix may be kept.
pub struct KeyedStreamHook<K: Hash + Eq + Clone, V, Order: Ordering, R: Retries> {
    input: Rc<RefCell<FxHashMap<K, VecDeque<V>>>>, // FxHasher is deterministic
    to_release: Option<Vec<(K, V)>>,
    /// How many values of the staged release stay buffered for redelivery (noted in the
    /// release log); always `0` for exactly-once values.
    kept_in_release: usize,
    /// How redeliveries are decided (see [`RetryBudget`]); never consulted for
    /// exactly-once values.
    budget: RetryBudget,
    output: Sender<(K, V)>,
    batch_location: HookLocationMeta,
    format_item_debug: fn(&(K, V)) -> Option<String>,
    _marker: PhantomData<(Order, R)>,
}

impl<K: Hash + Eq + Clone, V, Order: Ordering> KeyedStreamHook<K, V, Order, ExactlyOnce> {
    pub fn new(
        input: Rc<RefCell<FxHashMap<K, VecDeque<V>>>>,
        output: Sender<(K, V)>,
        batch_location: HookLocationMeta,
        format_item_debug: fn(&(K, V)) -> Option<String>,
    ) -> Self {
        Self {
            input,
            to_release: None,
            kept_in_release: 0,
            budget: RetryBudget::Bounded(0),
            output,
            batch_location,
            format_item_debug,
            _marker: PhantomData,
        }
    }
}

impl<K: Hash + Eq + Clone, V, Order: Ordering> KeyedStreamHook<K, V, Order, AtLeastOnce> {
    pub fn new(
        input: Rc<RefCell<FxHashMap<K, VecDeque<V>>>>,
        output: Sender<(K, V)>,
        batch_location: HookLocationMeta,
        format_item_debug: fn(&(K, V)) -> Option<String>,
        budget: RetryBudget,
    ) -> Self {
        Self {
            input,
            to_release: None,
            kept_in_release: 0,
            budget,
            output,
            batch_location,
            format_item_debug,
            _marker: PhantomData,
        }
    }
}

impl<K: Hash + Eq + Clone, V, Order: Ordering, R: Retries> RuntimeHook
    for KeyedStreamHook<K, V, Order, R>
{
    fn has_pending_input(&self) -> bool {
        keyed_buffer_len(&self.input.borrow()) > 0
    }

    fn only_one_possible_decision(&self) -> bool {
        // One buffered element is still a real choice: release it or not.
        keyed_buffer_len(&self.input.borrow()) == 0
    }

    fn release_decision(&mut self, log_writer: Option<&mut dyn std::fmt::Write>) {
        release_batch(
            self.to_release.take(),
            std::mem::take(&mut self.kept_in_release),
            order_label::<Order>(),
            self.batch_location,
            self.format_item_debug,
            &self.output,
            log_writer,
        );
    }

    fn location_meta(&self) -> HookLocationMeta {
        self.batch_location
    }
}

impl<K: Hash + Eq + Clone, V, R: BatchRetries<V>> TickInputHook
    for KeyedStreamHook<K, V, TotalOrder, R>
{
    fn can_trigger_tick(&self) -> bool {
        keyed_buffer_len(&self.input.borrow()) > 0
    }

    fn autonomous_decision<'a>(
        &mut self,
        driver: &mut Borrowed<'a>,
        mut force_trigger: bool,
    ) -> bool {
        let mut current_input = self.input.borrow_mut();
        let mut out = vec![];
        let mut kept = 0;
        #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
        let nonempty_key_count = current_input.values().filter(|q| !q.is_empty()).count();

        let mut remaining_nonempty_keys = nonempty_key_count;
        #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
        for (key, queue) in current_input.iter_mut() {
            if queue.is_empty() {
                continue;
            }

            remaining_nonempty_keys -= 1;

            let must_reveal = force_trigger && remaining_nonempty_keys == 0;
            let count = (usize::from(must_reveal)..=queue.len())
                .generate(driver)
                .unwrap();

            out.extend(
                queue
                    .drain(0..count.saturating_sub(1))
                    .map(|v| (key.clone(), v)),
            );
            if count > 0 {
                // The key's trailing value may be kept for redelivery at the start of the
                // key's contribution to a later batch (at-least-once only).
                match R::autonomous_keep(&mut self.budget, queue.front().unwrap(), driver) {
                    Some(copy) => {
                        out.push((key.clone(), copy));
                        kept += 1;
                    }
                    None => out.push((key.clone(), queue.pop_front().unwrap())),
                }
                force_trigger = false;
            }
        }

        self.kept_in_release = kept;
        let triggered = !out.is_empty();
        self.to_release = Some(out);
        triggered
    }
}

impl<K: Hash + Eq + Clone, V, R: BatchRetries<V>> TickInputHook
    for KeyedStreamHook<K, V, NoOrder, R>
{
    fn can_trigger_tick(&self) -> bool {
        keyed_buffer_len(&self.input.borrow()) > 0
    }

    fn autonomous_decision<'a>(
        &mut self,
        driver: &mut Borrowed<'a>,
        mut force_trigger: bool,
    ) -> bool {
        let mut current_input = self.input.borrow_mut();
        let mut out = vec![];
        let mut kept = 0;
        #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
        let nonempty_key_count = current_input.values().filter(|q| !q.is_empty()).count();

        let mut remaining_nonempty_keys = nonempty_key_count;
        #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
        for (key, queue) in current_input.iter_mut() {
            if queue.is_empty() {
                continue;
            }

            remaining_nonempty_keys -= 1;

            // Same per-key subset enumeration as `StreamHook<T, NoOrder, R>` (see there).
            let mut min_index = 0;
            while min_index < queue.len() {
                let must_release = force_trigger && remaining_nonempty_keys == 0;
                if !must_release && produce().generate(driver).unwrap() {
                    break;
                }

                let idx = (min_index..queue.len()).generate(driver).unwrap();
                if let Some(copy) = R::autonomous_keep(&mut self.budget, &queue[idx], driver) {
                    out.push((key.clone(), copy));
                    kept += 1;
                    min_index = idx + 1;
                } else {
                    out.push((key.clone(), queue.remove(idx).unwrap()));
                    min_index = idx;
                }
                force_trigger = false;
            }
        }

        self.kept_in_release = kept;
        let triggered = !out.is_empty();
        self.to_release = Some(out);
        triggered
    }
}

/// A scripted decision for a keyed batch hook over totally ordered values. Named values
/// are matched against each key's buffered prefix in order. The keep forms apply to
/// `AtLeastOnce` values only (see [`KeyedStreamHook`]).
///
/// Per-key payloads are `Vec`s rather than maps: the scripted entry order is preserved
/// (map iteration would make the release and log order nondeterministic), and the
/// handle rejects duplicate keys in [`Self::Prefixes`] when the decision is created, so
/// key distinctness is an invariant rather than a map-enforced property.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum KeyedBatchDecision<K, V> {
    /// Release the next `n` buffered values of each named key. Keys are distinct
    /// (enforced by the handle when the decision is created).
    Prefixes(Vec<(K, usize)>),
    /// Release exactly these `(key, value)` entries.
    Values(Vec<(K, V)>),
    /// Release everything that has arrived by the time the tick fires.
    All,
    /// Release the next `n >= 1` buffered values of each named key, keeping each key's
    /// last one buffered: it is redelivered as that key's first value in a later batch.
    /// Keys are distinct (enforced by the handle).
    PrefixesKeepLast(Vec<(K, usize)>),
    /// Release exactly these (non-empty) `(key, value)` entries, keeping the last entry
    /// of each named key buffered for redelivery.
    ValuesKeepLast(Vec<(K, V)>),
}

impl<K, V> ScriptDecision for KeyedBatchDecision<K, V>
where
    K: serde::Serialize + serde::de::DeserializeOwned,
    V: serde::Serialize + serde::de::DeserializeOwned,
{
    fn describe(&self) -> String {
        match self {
            KeyedBatchDecision::Prefixes(counts) => {
                format!("release({} per-key count(s))", counts.len())
            }
            KeyedBatchDecision::Values(values) => {
                format!("release_values({} entr(ies))", values.len())
            }
            KeyedBatchDecision::All => "release_all()".to_owned(),
            KeyedBatchDecision::PrefixesKeepLast(counts) => {
                format!("release_keep_last({} per-key count(s))", counts.len())
            }
            KeyedBatchDecision::ValuesKeepLast(values) => {
                format!("release_values_keep_last({} entr(ies))", values.len())
            }
        }
    }
}

/// A scripted decision for a keyed batch hook over unordered values. Named values are
/// matched per key as multisets. The keep form applies to `AtLeastOnce` values only (see
/// [`KeyedStreamHook`]).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum UnorderedKeyedBatchDecision<K, V> {
    /// Release exactly these `(key, value)` entries.
    Values(Vec<(K, V)>),
    /// Release everything that has arrived by the time the tick fires.
    All,
    /// Release exactly these `(key, value)` entries, keeping the second list (a
    /// sub-multiset of the first) buffered for redelivery in later batches.
    ValuesKeep(Vec<(K, V)>, Vec<(K, V)>),
}

impl<K, V> ScriptDecision for UnorderedKeyedBatchDecision<K, V>
where
    K: serde::Serialize + serde::de::DeserializeOwned,
    V: serde::Serialize + serde::de::DeserializeOwned,
{
    fn describe(&self) -> String {
        match self {
            UnorderedKeyedBatchDecision::Values(values) => {
                format!("release_values({} entr(ies))", values.len())
            }
            UnorderedKeyedBatchDecision::All => "release_all()".to_owned(),
            UnorderedKeyedBatchDecision::ValuesKeep(values, keep) => format!(
                "release_values_keep({} entr(ies), {} kept)",
                values.len(),
                keep.len()
            ),
        }
    }
}

/// Checks that every named key has buffered at least the requested number of values
/// (keys are distinct by the decision's invariant).
fn keyed_prefixes_honorable<K: Hash + Eq, V>(
    input: &FxHashMap<K, VecDeque<V>>,
    counts: &[(K, usize)],
) -> bool {
    counts
        .iter()
        .all(|(key, count)| input.get(key).is_some_and(|queue| queue.len() >= *count))
}

/// Checks that each key's scripted values match that key's buffered prefix in order
/// (cross-key interleaving in the script is irrelevant): `Err` on a contradiction,
/// `Ok(false)` while some key's prefix is still incomplete.
fn keyed_values_honorable<K: Hash + Eq, V: PartialEq>(
    input: &FxHashMap<K, VecDeque<V>>,
    values: &[(K, V)],
    method: &str,
) -> Result<bool, String> {
    let mut consumed: FxHashMap<&K, usize> = FxHashMap::default();
    for (position, (key, expected)) in values.iter().enumerate() {
        let offset = consumed.entry(key).or_insert(0);
        match input.get(key).and_then(|queue| queue.get(*offset)) {
            Some(buffered) if buffered == expected => {}
            Some(_) => {
                return Err(format!(
                    "{}: buffered item at per-key prefix position {} did not match the expected value (scripted entry {})",
                    method, *offset, position
                ));
            }
            None => return Ok(false),
        }
        *offset += 1;
    }
    Ok(true)
}

/// Whether every entry of `requested` has a distinct match among the buffered entries (a
/// per-key multiset inclusion check).
fn keyed_multiset_included<'a, K: Hash + Eq + 'a, V: PartialEq + 'a>(
    requested: &[(K, V)],
    available: impl Iterator<Item = (&'a K, &'a V)>,
) -> bool {
    let mut unmatched: Vec<(&K, &V)> = requested.iter().map(|(k, v)| (k, v)).collect();
    for (key, value) in available {
        if let Some(idx) = unmatched.iter().position(|(k, v)| *k == key && *v == value) {
            unmatched.swap_remove(idx);
        }
    }
    unmatched.is_empty()
}

/// Drains every key's queue into `(key, value)` entries.
fn drain_all_keyed<K: Clone, V>(input: &mut FxHashMap<K, VecDeque<V>>) -> Vec<(K, V)> {
    let mut out = vec![];
    #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
    for (key, queue) in input.iter_mut() {
        out.extend(queue.drain(..).map(|v| (key.clone(), v)));
    }
    out
}

impl<K, V, R: BatchRetries<V>> ScriptableHook for KeyedStreamHook<K, V, TotalOrder, R>
where
    K: Hash + Eq + Clone + serde::Serialize + serde::de::DeserializeOwned,
    V: serde::Serialize + serde::de::DeserializeOwned + PartialEq,
{
    type Decision = KeyedBatchDecision<K, V>;
    type Status = BatchStatus;

    fn is_honorable(&self, decision: &KeyedBatchDecision<K, V>) -> Result<bool, String> {
        let input = self.input.borrow();
        match decision {
            KeyedBatchDecision::Prefixes(counts) => Ok(keyed_prefixes_honorable(&input, counts)),
            KeyedBatchDecision::Values(values) => {
                keyed_values_honorable(&input, values, "release_values")
            }
            KeyedBatchDecision::All => Ok(true),
            KeyedBatchDecision::PrefixesKeepLast(counts) => {
                check_keep_allowed::<R>("release_keep_last")?;
                if counts.iter().any(|(_, count)| *count == 0) {
                    // The handle asserts this at the scripting call site; kept as a
                    // defensive permanent error.
                    return Err(
                        "release_keep_last: every named key must release at least one value to keep its last"
                            .to_owned(),
                    );
                }
                Ok(keyed_prefixes_honorable(&input, counts))
            }
            KeyedBatchDecision::ValuesKeepLast(values) => {
                check_keep_allowed::<R>("release_values_keep_last")?;
                if values.is_empty() {
                    return Err(
                        "release_values_keep_last: at least one entry must be released to keep its last"
                            .to_owned(),
                    );
                }
                keyed_values_honorable(&input, values, "release_values_keep_last")
            }
        }
    }

    fn apply(&mut self, decision: KeyedBatchDecision<K, V>) {
        let mut input = self.input.borrow_mut();
        let keep_last = matches!(
            decision,
            KeyedBatchDecision::PrefixesKeepLast(_) | KeyedBatchDecision::ValuesKeepLast(_)
        );
        let mut kept = 0;
        let out: Vec<(K, V)> = match decision {
            KeyedBatchDecision::Prefixes(counts) | KeyedBatchDecision::PrefixesKeepLast(counts) => {
                let mut out = vec![];
                for (key, count) in counts {
                    let queue = input.get_mut(&key).unwrap();
                    out.extend(
                        queue
                            .drain(0..count.saturating_sub(1))
                            .map(|v| (key.clone(), v)),
                    );
                    if count > 0 {
                        if keep_last {
                            // The key's trailing value stays at its front for redelivery.
                            out.push((key.clone(), R::scripted_keep(queue.front().unwrap())));
                            kept += 1;
                        } else {
                            out.push((key.clone(), queue.pop_front().unwrap()));
                        }
                    }
                }
                out
            }
            KeyedBatchDecision::Values(values) | KeyedBatchDecision::ValuesKeepLast(values) => {
                // `is_honorable` verified each key's scripted values match that key's
                // buffered prefix, so popping fronts in script order releases exactly the
                // named items; when keeping, each key's last scripted entry is instead
                // copied and left at the key's front for redelivery.
                let mut remaining_per_key: FxHashMap<&K, usize> = FxHashMap::default();
                if keep_last {
                    for (key, _) in &values {
                        *remaining_per_key.entry(key).or_insert(0) += 1;
                    }
                }
                let mut out = Vec::with_capacity(values.len());
                for (key, _expected) in &values {
                    let queue = input.get_mut(key).unwrap();
                    let is_kept_last = keep_last && {
                        let remaining = remaining_per_key.get_mut(key).unwrap();
                        *remaining -= 1;
                        *remaining == 0
                    };
                    if is_kept_last {
                        out.push((key.clone(), R::scripted_keep(queue.front().unwrap())));
                        kept += 1;
                    } else {
                        out.push((key.clone(), queue.pop_front().unwrap()));
                    }
                }
                out
            }
            KeyedBatchDecision::All => drain_all_keyed(&mut input),
        };

        self.kept_in_release = kept;
        self.to_release = Some(out);
    }

    fn implicit(&mut self) {
        self.to_release = Some(vec![]);
    }

    fn status(&self) -> BatchStatus {
        BatchStatus {
            buffered: keyed_buffer_len(&self.input.borrow()),
        }
    }

    fn describe_pending(&self) -> Option<String> {
        describe_keyed_pending(&self.input.borrow())
    }
}

impl<K, V, R: BatchRetries<V>> ScriptableTickInputHook for KeyedStreamHook<K, V, TotalOrder, R>
where
    K: Hash + Eq + Clone + serde::Serialize + serde::de::DeserializeOwned,
    V: serde::Serialize + serde::de::DeserializeOwned + PartialEq,
{
    fn decision_triggers_tick(&self, decision: &KeyedBatchDecision<K, V>) -> bool {
        match decision {
            KeyedBatchDecision::Prefixes(counts) | KeyedBatchDecision::PrefixesKeepLast(counts) => {
                counts.iter().any(|(_, count)| *count > 0)
            }
            KeyedBatchDecision::Values(values) | KeyedBatchDecision::ValuesKeepLast(values) => {
                !values.is_empty()
            }
            KeyedBatchDecision::All => keyed_buffer_len(&self.input.borrow()) > 0,
        }
    }
}

impl<K, V, R: BatchRetries<V>> ScriptableHook for KeyedStreamHook<K, V, NoOrder, R>
where
    K: Hash + Eq + Clone + serde::Serialize + serde::de::DeserializeOwned,
    V: serde::Serialize + serde::de::DeserializeOwned + PartialEq,
{
    type Decision = UnorderedKeyedBatchDecision<K, V>;
    type Status = BatchStatus;

    fn is_honorable(&self, decision: &UnorderedKeyedBatchDecision<K, V>) -> Result<bool, String> {
        let input = self.input.borrow();
        // Values are matched per key as multisets: duplicates request the corresponding
        // number of equal buffered items under that key.
        #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
        let buffered = || {
            input
                .iter()
                .flat_map(|(key, queue)| queue.iter().map(move |value| (key, value)))
        };
        match decision {
            UnorderedKeyedBatchDecision::Values(values) => {
                Ok(keyed_multiset_included(values, buffered()))
            }
            UnorderedKeyedBatchDecision::All => Ok(true),
            UnorderedKeyedBatchDecision::ValuesKeep(values, keep) => {
                check_keep_allowed::<R>("release_values_keep")?;
                if !keyed_multiset_included(keep, values.iter().map(|(k, v)| (k, v))) {
                    return Err(
                        "release_values_keep: every kept entry must be among the released entries"
                            .to_owned(),
                    );
                }
                Ok(keyed_multiset_included(values, buffered()))
            }
        }
    }

    fn apply(&mut self, decision: UnorderedKeyedBatchDecision<K, V>) {
        let mut input = self.input.borrow_mut();
        let (values, mut keep) = match decision {
            UnorderedKeyedBatchDecision::All => {
                self.to_release = Some(drain_all_keyed(&mut input));
                return;
            }
            UnorderedKeyedBatchDecision::Values(values) => (values, vec![]),
            UnorderedKeyedBatchDecision::ValuesKeep(values, keep) => (values, keep),
        };

        // A released entry that also matches a kept entry is copied and left in place;
        // equal entries under a key are indistinguishable, so which of them is kept is
        // unobservable.
        let mut kept = 0;
        let mut out = Vec::with_capacity(values.len());
        for (key, expected) in values {
            let queue = input.get_mut(&key).unwrap();
            let idx = queue.iter().position(|item| *item == expected).unwrap();
            if let Some(keep_idx) = keep.iter().position(|(k, v)| *k == key && *v == expected) {
                keep.swap_remove(keep_idx);
                kept += 1;
                out.push((key, R::scripted_keep(&queue[idx])));
            } else {
                out.push((key, queue.remove(idx).unwrap()));
            }
        }
        if !keep.is_empty() {
            abort!("scripted unordered keyed batch decision was not honorable");
        }

        self.kept_in_release = kept;
        self.to_release = Some(out);
    }

    fn implicit(&mut self) {
        self.to_release = Some(vec![]);
    }

    fn status(&self) -> BatchStatus {
        BatchStatus {
            buffered: keyed_buffer_len(&self.input.borrow()),
        }
    }

    fn describe_pending(&self) -> Option<String> {
        describe_keyed_pending(&self.input.borrow())
    }
}

impl<K, V, R: BatchRetries<V>> ScriptableTickInputHook for KeyedStreamHook<K, V, NoOrder, R>
where
    K: Hash + Eq + Clone + serde::Serialize + serde::de::DeserializeOwned,
    V: serde::Serialize + serde::de::DeserializeOwned + PartialEq,
{
    fn decision_triggers_tick(&self, decision: &UnorderedKeyedBatchDecision<K, V>) -> bool {
        match decision {
            UnorderedKeyedBatchDecision::Values(values)
            | UnorderedKeyedBatchDecision::ValuesKeep(values, _) => !values.is_empty(),
            UnorderedKeyedBatchDecision::All => keyed_buffer_len(&self.input.borrow()) > 0,
        }
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

    fn location_meta(&self) -> HookLocationMeta {
        self.batch_location
    }
}

impl<T: Clone> TickInputHook for SingletonHook<T> {
    fn can_trigger_tick(&self) -> bool {
        // TODO(mingwei): Singletons/Optionals will soon not trigger a tick, so then this will always return false.
        !self.input.borrow().is_empty()
    }

    fn autonomous_decision<'a>(&mut self, driver: &mut Borrowed<'a>, force_trigger: bool) -> bool {
        let mut current_input = self.input.borrow_mut();
        if current_input.is_empty() {
            assert!(
                !force_trigger,
                "Cannot make a triggering decision when there is no input"
            );

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

/// A hook for batching / snapshotting an [`Optional`](crate::live_collections::Optional) with
/// `InitNone` boundedness into a tick.
///
/// This is the [`SingletonHook`] analog for optionals whose *presence* is monotone (the
/// `InitNone` bound): the optional starts null and, once it becomes non-null, stays non-null.
/// It differs from [`SingletonHook`] in that its released value is optional. Before the first
/// non-null value it releases *null* (sending nothing into the tick's `source_stream`, so the
/// downstream optional is empty). Once it has released a value, presence is monotone, so it only
/// ever re-releases or advances to a newer value — never back to null.
pub struct OptionalInitNoneHook<T> {
    input: Rc<RefCell<VecDeque<T>>>,
    to_release: Option<(Option<T>, bool)>, // (value or null, is new)
    last_released: Option<T>,              // last non-null value released (None until the first)
    skipped_states: Vec<T>,
    output: Sender<T>,
    batch_location: HookLocationMeta,
    format_item_debug: fn(&T) -> Option<String>,
}

impl<T: Clone> OptionalInitNoneHook<T> {
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

impl<T: Clone> RuntimeHook for OptionalInitNoneHook<T> {
    fn has_pending_input(&self) -> bool {
        !self.input.borrow().is_empty()
    }

    fn only_one_possible_decision(&self) -> bool {
        // If there is no input, the decision is trivial.
        // Even if the input is a single item, we can always either take it or leave it, and
        // remain at the existing value (which may be `None`).
        self.input.borrow().is_empty()
    }

    fn release_decision(&mut self, log_writer: Option<&mut dyn std::fmt::Write>) {
        let Some((to_release, is_new)) = self.to_release.take() else {
            panic!("No decision to release");
        };

        if let Some(value) = &to_release {
            self.last_released = Some(value.clone());
        }

        if let Some(log_writer) = log_writer {
            let HookLocationMeta {
                location: batch_location,
                line,
                caret_indent,
            } = self.batch_location;
            let note_str = match (&to_release, is_new) {
                (None, _) => "^ releasing null snapshot".to_owned(),
                (Some(value), true) => {
                    if self.skipped_states.is_empty() {
                        format!(
                            "^ releasing snapshot: {:?}",
                            ManualDebug(value, self.format_item_debug)
                        )
                    } else {
                        format!(
                            "^ releasing snapshot: {:?} (skipping earlier states: {:?})",
                            ManualDebug(value, self.format_item_debug),
                            self.skipped_states
                                .iter()
                                .map(|s| ManualDebug(s, self.format_item_debug))
                                .collect::<Vec<_>>()
                        )
                    }
                }
                (Some(value), false) => format!(
                    "^ releasing unchanged snapshot: {:?}",
                    ManualDebug(value, self.format_item_debug)
                ),
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

        if let Some(value) = to_release {
            self.output.try_send(value).unwrap();
        }
    }

    fn location_meta(&self) -> HookLocationMeta {
        self.batch_location
    }
}

impl<T: Clone> TickInputHook for OptionalInitNoneHook<T> {
    fn can_trigger_tick(&self) -> bool {
        // TODO(mingwei): Singletons/Optionals will soon not trigger a tick, so then this will always return false.
        // Only advancing to a new value is nontrivial; releasing null (or re-releasing the
        // latest value) does not, by itself, drive a tick.
        !self.input.borrow().is_empty()
    }

    fn autonomous_decision<'a>(
        &mut self,
        driver: &mut Borrowed<'a>,
        force_nontrivial: bool,
    ) -> bool {
        let mut current_input = self.input.borrow_mut();
        if current_input.is_empty() {
            // Case 1 (trivial): No input.
            assert!(
                !force_nontrivial,
                "Cannot make nontrivial decision when there is no input"
            );

            if let Some(last) = &self.last_released {
                // Presence is monotone: once non-null, re-release the latest value.
                self.to_release = Some((Some(last.clone()), false));
            } else {
                // Still in the initial-null prefix.
                self.to_release = Some((None, false));
            }
            false
        } else if !force_nontrivial && produce().generate(driver).unwrap() {
            // Case 2 (trivial): Keep latest value (may be `Some` or `None`)
            if let Some(last) = &self.last_released {
                // Already non-null; re-release the latest value (models snapshot lag).
                self.to_release = Some((Some(last.clone()), false));
            } else {
                // Still in the initial-null prefix even though a value is buffered: models a
                // snapshot that does not yet include the first value.
                self.to_release = Some((None, false));
            }
            false
        } else {
            // Case 3 (non-trivial): Advance to new value.
            let idx_to_release = (0..current_input.len()).generate(driver).unwrap();
            self.skipped_states = current_input.drain(0..idx_to_release).collect(); // Drop earlier items
            let item = current_input.pop_front().unwrap();
            self.to_release = Some((Some(item), true));
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
                }

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

        any_triggered
    }
}

/// A scripted decision for a keyed snapshot hook: which buffered version each key's next
/// tick execution observes.
///
/// The [`Self::Reveal`] payload is a `Vec` rather than a map: the scripted entry order is
/// preserved for the release log, and the handle rejects duplicate keys when the decision
/// is created, so key distinctness is an invariant rather than a map-enforced property.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum KeyedSnapshotDecision<K, V> {
    /// For each named key, reveal the first buffered version equal to the named value,
    /// skipping over earlier versions. Keys are distinct (enforced by the handle when
    /// the decision is created; a key observes exactly one version per tick). Keys that
    /// are not named observe their previously revealed version again (or stay absent if
    /// they have never been revealed).
    Reveal(Vec<(K, V)>),
    /// For every key, reveal the newest version that has arrived by the time the tick
    /// fires (keys with nothing newer observe their previously revealed version again).
    RevealLatest,
    /// Every key observes its previously revealed version again.
    Keep,
}

impl<K, V> ScriptDecision for KeyedSnapshotDecision<K, V>
where
    K: serde::Serialize + serde::de::DeserializeOwned,
    V: serde::Serialize + serde::de::DeserializeOwned,
{
    fn describe(&self) -> String {
        match self {
            KeyedSnapshotDecision::Reveal(_) => "reveal(..)".to_owned(),
            KeyedSnapshotDecision::RevealLatest => "reveal_latest()".to_owned(),
            KeyedSnapshotDecision::Keep => "keep()".to_owned(),
        }
    }
}

/// The pending-input view a keyed snapshot hook reports to its test-side handle (see
/// [`ScriptableHook::status`]), used by `pause_until_*` predicates.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct KeyedSnapshotStatus {
    /// The total number of buffered versions (across all keys) newer than the last
    /// revealed ones.
    pub newer_versions: usize,
    /// The number of keys with at least one newer buffered version.
    pub keys_with_newer_versions: usize,
}

impl<K, V> ScriptableHook for KeyedSingletonHook<K, V>
where
    K: Hash + Eq + Clone + serde::Serialize + serde::de::DeserializeOwned,
    V: Clone + PartialEq + serde::Serialize + serde::de::DeserializeOwned,
{
    type Decision = KeyedSnapshotDecision<K, V>;
    type Status = KeyedSnapshotStatus;

    fn is_honorable(&self, decision: &KeyedSnapshotDecision<K, V>) -> Result<bool, String> {
        let input = self.input.borrow();
        Ok(match decision {
            KeyedSnapshotDecision::Reveal(entries) => entries.iter().all(|(key, target)| {
                input
                    .get(key)
                    .is_some_and(|queue| queue.iter().any(|version| version == target))
            }),
            KeyedSnapshotDecision::RevealLatest => {
                keyed_buffer_len(&input) > 0 || !self.last_released.is_empty()
            }
            KeyedSnapshotDecision::Keep => true,
        })
    }

    fn apply(&mut self, decision: KeyedSnapshotDecision<K, V>) {
        let mut input = self.input.borrow_mut();
        match decision {
            KeyedSnapshotDecision::Reveal(entries) => {
                let mut to_release = vec![];
                let mut named: FxHashSet<K> = FxHashSet::default();
                for (key, target) in entries {
                    let queue = input.get_mut(&key).unwrap();
                    let idx = queue.iter().position(|version| *version == target).unwrap();
                    let skipped: Vec<V> = queue.drain(0..idx).collect();
                    let item = queue.pop_front().unwrap();
                    self.skipped_states.insert(key.clone(), skipped);
                    self.last_released.insert(key.clone(), item.clone());
                    to_release.push((key.clone(), item, true));
                    named.insert(key);
                }
                // Unnamed keys observe their previously revealed version again.
                #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
                for (key, last) in self.last_released.iter() {
                    if !named.contains(key) {
                        to_release.push((key.clone(), last.clone(), false));
                    }
                }
                self.to_release = Some(to_release);
            }
            KeyedSnapshotDecision::RevealLatest => {
                let mut to_release = vec![];
                #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
                for (key, queue) in input.iter_mut() {
                    if queue.is_empty() {
                        if let Some(last) = self.last_released.get(key) {
                            to_release.push((key.clone(), last.clone(), false));
                        }
                    } else {
                        let skip_count = queue.len() - 1;
                        let skipped: Vec<V> = queue.drain(0..skip_count).collect();
                        let item = queue.pop_front().unwrap();
                        self.skipped_states.insert(key.clone(), skipped);
                        self.last_released.insert(key.clone(), item.clone());
                        to_release.push((key.clone(), item, true));
                    }
                }
                self.to_release = Some(to_release);
            }
            KeyedSnapshotDecision::Keep => {
                #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
                let to_release = self
                    .last_released
                    .iter()
                    .map(|(key, last)| (key.clone(), last.clone(), false))
                    .collect();
                self.to_release = Some(to_release);
            }
        }
    }

    fn implicit(&mut self) {
        // "Nothing new": every previously revealed key observes its value again.
        #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
        let to_release = self
            .last_released
            .iter()
            .map(|(key, last)| (key.clone(), last.clone(), false))
            .collect();
        self.to_release = Some(to_release);
    }

    fn status(&self) -> KeyedSnapshotStatus {
        let input = self.input.borrow();
        #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
        let keys_with_newer_versions = input.values().filter(|q| !q.is_empty()).count();
        KeyedSnapshotStatus {
            newer_versions: keyed_buffer_len(&input),
            keys_with_newer_versions,
        }
    }

    fn describe_pending(&self) -> Option<String> {
        let input = self.input.borrow();
        let total = keyed_buffer_len(&input);
        #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
        let keys = input.values().filter(|q| !q.is_empty()).count();
        (total > 0).then(|| format!("{} buffered version(s) across {} key(s)", total, keys))
    }
}

impl<K, V> ScriptableTickInputHook for KeyedSingletonHook<K, V>
where
    K: Hash + Eq + Clone + serde::Serialize + serde::de::DeserializeOwned,
    V: Clone + PartialEq + serde::Serialize + serde::de::DeserializeOwned,
{
    fn decision_triggers_tick(&self, decision: &KeyedSnapshotDecision<K, V>) -> bool {
        match decision {
            KeyedSnapshotDecision::Reveal(entries) => !entries.is_empty(),
            KeyedSnapshotDecision::RevealLatest => keyed_buffer_len(&self.input.borrow()) > 0,
            KeyedSnapshotDecision::Keep => false,
        }
    }
}
