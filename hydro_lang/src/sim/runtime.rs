use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::fmt::Debug;
use std::hash::Hash;
use std::rc::Rc;

use bolero::generator::bolero_generator::driver::object::Borrowed;
use bolero::{ValueGenerator, produce};
use colored::Colorize;
use dfir_rs::rustc_hash::FxHashMap;
use dfir_rs::util::unsync::mpsc::Sender;

use crate::live_collections::stream::{NoOrder, Ordering, TotalOrder};

pub type Hooks<Key> = HashMap<(Key, Option<u32>), Vec<Box<dyn RuntimeHook>>>;
pub type InlineHooks<Key> = HashMap<(Key, Option<u32>), Vec<Box<dyn SimInlineHook>>>;

#[doc(hidden)]
#[macro_export]
#[doc(hidden)]
macro_rules! __maybe_debug__ {
    ($type:ty) => {{
        // Inherent-shadows-trait trick (same as `impls` crate), but for
        // a function pointer. The const is stored as a raw `*const ()` to
        // avoid mentioning the type in the const type (which would force
        // bound resolution and break the trick). We transmute back at the end.

        #[expect(clippy::allow_attributes, reason = "macro codegen")]
        #[allow(dead_code, reason = "shadowing trick")]
        fn no_debug<T>(_: &T) -> Option<String> {
            None
        }
        fn yes_debug<T: std::fmt::Debug>(v: &T) -> Option<String> {
            Some(format!("{:?}", v))
        }

        trait __Fallback {
            const MAYBE_DEBUG_FN: *const () = no_debug::<()> as *const ();
        }
        impl<T: ?Sized> __Fallback for T {}

        struct __Wrap<T>(std::marker::PhantomData<T>);

        #[expect(clippy::allow_attributes, reason = "macro codegen")]
        #[allow(dead_code, reason = "shadowing trick")]
        impl<T: std::fmt::Debug> __Wrap<T> {
            const MAYBE_DEBUG_FN: *const () = yes_debug::<T> as *const ();
        }

        // SAFETY: The pointer is either `no_debug::<()>` or `yes_debug::<$type>`.
        // `no_debug` ignores its argument entirely, so the ABI is compatible
        // regardless of the concrete type.
        unsafe {
            std::mem::transmute::<*const (), fn(&$type) -> Option<String>>(
                <__Wrap<$type>>::MAYBE_DEBUG_FN,
            )
        }
    }};
}

pub trait RuntimeHook {
    fn current_decision(&self) -> Option<bool>;
    fn can_make_nontrivial_decision(&self) -> bool;
    fn autonomous_decision<'a>(
        &mut self,
        driver: &mut Borrowed<'a>,
        force_nontrivial: bool,
    ) -> bool;

    /// Release the decision that was made, logging to `log_writer`. A `None`
    /// writer means logging is disabled, allowing the hook to skip formatting
    /// entirely.
    fn release_decision(&mut self, log_writer: Option<&mut dyn std::fmt::Write>);

    /// Whether this hook is ready to participate in a tick. Returns false if the
    /// hook has never received any input and cannot produce a value (e.g. a
    /// singleton whose producing tick hasn't run yet).
    fn is_ready(&self) -> bool {
        true
    }

    /// The source location of the operator this hook simulates, used to attribute errors
    /// (e.g. an unhooked non-deterministic operator in deterministic mode).
    fn location_meta(&self) -> Option<HookLocationMeta> {
        None
    }
}

/// Renders the error for an unsafe operator that needs a non-deterministic decision in
/// deterministic mode.
pub(crate) fn render_unhooked_nondet_error(location: HookLocationMeta) -> String {
    let (loc, line, caret) = location;
    format!(
        "deterministic simulation encountered an unsafe operator with pending input that is not bound to a sim hook:\n--> {loc}\n |{line}\n |{caret}^ this operator must make a non-deterministic decision\nhelp: bind a sim hook to this operator (`nondet!(... hook = handle)`) and script its decisions, or run under `fuzz` / `exhaustive` instead"
    )
}

/// A hook that can make inline decisions during the execution of a tick.
///
/// Primarily used for `ObserveNonDet` IR nodes.
pub trait SimInlineHook {
    /// Whether there are pending inputs that require a decision to be made.
    fn pending_decision(&self) -> bool;

    /// Whether a decision has already been made and is ready to be released.
    fn has_decision(&self) -> bool;

    /// Make an autonomous decision.
    fn autonomous_decision<'a>(&mut self, driver: &mut Borrowed<'a>);

    /// Whether the pending decision is *forced* (e.g. ordering zero or one elements):
    /// deterministic mode allows autonomous decisions only when there is nothing to
    /// decide, and panics (naming the operator) otherwise.
    fn decision_is_forced(&self) -> bool;

    /// The source location of the operator this hook simulates.
    fn location_meta(&self) -> HookLocationMeta;

    /// Release the decision that was made, logging to `log_writer`. A `None`
    /// writer means logging is disabled, allowing the hook to skip formatting
    /// entirely.
    fn release_decision(&mut self, log_writer: Option<&mut dyn std::fmt::Write>);
}

struct ManualDebug<'a, T>(&'a T, fn(&T) -> Option<String>);
impl<'a, T> Debug for ManualDebug<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self(v, debug_fn) = self;
        if let Some(s) = debug_fn(v) {
            write!(f, "{}", s)
        } else {
            write!(f, "?")
        }
    }
}

struct LabeledDebug<'a, T>(&'static str, &'a T, fn(&T) -> Option<String>);
impl<'a, T> Debug for LabeledDebug<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self(label, v, debug_fn) = self;
        let value = debug_fn(v).unwrap_or_else(|| "?".to_owned());
        let color = if *label == "l" {
            colored::Color::Magenta
        } else {
            colored::Color::Yellow
        };
        write!(f, "{}", format!("{}: {}", label, value).color(color))
    }
}

struct TruncatedVecDebug<'a, T: 'a, I: Iterator<Item = &'a T>>(
    RefCell<Option<I>>,
    usize,
    fn(&T) -> Option<String>,
);
impl<'a, T, I: Iterator<Item = &'a T>> Debug for TruncatedVecDebug<'a, T, I> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self(iter, max, elem_debug) = self;
        let iter = iter.take().unwrap();
        if let Some(length) = iter.size_hint().1
            && length > *max
        {
            f.debug_list()
                .entries(iter.take(*max).map(|v| ManualDebug(v, *elem_debug)))
                .finish_non_exhaustive()?;
            write!(f, " ({} total)", length)
        } else {
            f.debug_list()
                .entries(iter.map(|v| ManualDebug(v, *elem_debug)))
                .finish()
        }
    }
}

struct TruncatedLabeledVecDebug<'a, T: 'a, I: Iterator<Item = (&'static str, &'a T)>>(
    RefCell<Option<I>>,
    usize,
    fn(&T) -> Option<String>,
);
impl<'a, T, I: Iterator<Item = (&'static str, &'a T)>> Debug
    for TruncatedLabeledVecDebug<'a, T, I>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self(iter, max, elem_debug) = self;
        let iter = iter.take().unwrap();
        if let Some(length) = iter.size_hint().1
            && length > *max
        {
            f.debug_list()
                .entries(
                    iter.take(*max)
                        .map(|(l, v)| LabeledDebug(l, v, *elem_debug)),
                )
                .finish_non_exhaustive()?;
            write!(f, " ({} total)", length)
        } else {
            f.debug_list()
                .entries(iter.map(|(l, v)| LabeledDebug(l, v, *elem_debug)))
                .finish()
        }
    }
}

type HookLocationMeta = (&'static str, &'static str, &'static str);

/// Writes the standard release-log source block: the `--> location` header, the source
/// line, and a caret note (colored `note_color`) under the operator.
fn log_release(
    log_writer: &mut dyn std::fmt::Write,
    location: &str,
    line: &str,
    caret_indent: &str,
    note: &str,
    note_color: colored::Color,
) {
    let _ = writeln!(
        log_writer,
        "{} {}",
        "-->".color(colored::Color::Blue),
        location
    );

    let _ = writeln!(log_writer, " {}{}", "|".color(colored::Color::Blue), line);

    let _ = writeln!(
        log_writer,
        " {}{}{}",
        "|".color(colored::Color::Blue),
        caret_indent,
        note.color(note_color)
    );
}

pub struct StreamHook<T, Order: Ordering> {
    pub input: Rc<RefCell<VecDeque<T>>>,
    pub to_release: Option<Vec<T>>,
    pub output: Sender<T>,
    pub batch_location: HookLocationMeta,
    pub format_item_debug: fn(&T) -> Option<String>,
    pub _order: std::marker::PhantomData<Order>,
}

impl<T> RuntimeHook for StreamHook<T, TotalOrder> {
    fn location_meta(&self) -> Option<HookLocationMeta> {
        Some(self.batch_location)
    }

    fn current_decision(&self) -> Option<bool> {
        self.to_release.as_ref().map(|v| !v.is_empty())
    }

    fn can_make_nontrivial_decision(&self) -> bool {
        !self.input.borrow().is_empty()
    }

    fn autonomous_decision<'a>(
        &mut self,
        driver: &mut Borrowed<'a>,
        force_nontrivial: bool,
    ) -> bool {
        let mut current_input = self.input.borrow_mut();
        let count = ((if force_nontrivial { 1 } else { 0 })..=current_input.len())
            .generate(driver)
            .unwrap();

        self.to_release = Some(current_input.drain(0..count).collect());
        count > 0
    }

    fn release_decision(&mut self, log_writer: Option<&mut dyn std::fmt::Write>) {
        if let Some(to_release) = self.to_release.take() {
            if let Some(log_writer) = log_writer {
                let (batch_location, line, caret_indent) = self.batch_location;
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
}

impl<T> RuntimeHook for StreamHook<T, NoOrder> {
    fn location_meta(&self) -> Option<HookLocationMeta> {
        Some(self.batch_location)
    }

    fn current_decision(&self) -> Option<bool> {
        self.to_release.as_ref().map(|v| !v.is_empty())
    }

    fn can_make_nontrivial_decision(&self) -> bool {
        !self.input.borrow().is_empty()
    }

    fn autonomous_decision<'a>(
        &mut self,
        driver: &mut Borrowed<'a>,
        force_nontrivial: bool,
    ) -> bool {
        let mut current_input = self.input.borrow_mut();
        let mut out = vec![];
        let mut min_index = 0;
        while !current_input.is_empty() {
            let must_release = force_nontrivial && out.is_empty();
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

        let was_nontrivial = !out.is_empty();
        self.to_release = Some(out);
        was_nontrivial
    }

    fn release_decision(&mut self, log_writer: Option<&mut dyn std::fmt::Write>) {
        if let Some(to_release) = self.to_release.take() {
            if let Some(log_writer) = log_writer {
                let (batch_location, line, caret_indent) = self.batch_location;
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
    /// Release an empty batch, holding everything buffered.
    Empty,
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
            BatchDecision::Empty => "release_empty()".to_owned(),
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
    /// Release an empty batch, holding everything buffered.
    Empty,
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
            UnorderedBatchDecision::Empty => "release_empty()".to_owned(),
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
            BatchDecision::All | BatchDecision::Empty => Ok(true),
        }
    }

    fn is_nontrivial(&self, decision: &BatchDecision<T>) -> bool {
        match decision {
            BatchDecision::Prefix(n) => *n > 0,
            BatchDecision::Values(values) => !values.is_empty(),
            BatchDecision::All => !self.input.borrow().is_empty(),
            BatchDecision::Empty => false,
        }
    }

    fn apply(&mut self, decision: BatchDecision<T>) -> bool {
        let mut input = self.input.borrow_mut();
        let out: Vec<T> = match decision {
            BatchDecision::Prefix(n) => input.drain(0..n).collect(),
            BatchDecision::Values(values) => input.drain(0..values.len()).collect(),
            BatchDecision::All => input.drain(..).collect(),
            BatchDecision::Empty => vec![],
        };

        let nontrivial = !out.is_empty();
        self.to_release = Some(out);
        nontrivial
    }

    fn implicit(&mut self) -> bool {
        self.to_release = Some(vec![]);
        false
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
            UnorderedBatchDecision::All | UnorderedBatchDecision::Empty => true,
        })
    }

    fn is_nontrivial(&self, decision: &UnorderedBatchDecision<T>) -> bool {
        match decision {
            UnorderedBatchDecision::Values(values) => !values.is_empty(),
            UnorderedBatchDecision::All => !self.input.borrow().is_empty(),
            UnorderedBatchDecision::Empty => false,
        }
    }

    fn apply(&mut self, decision: UnorderedBatchDecision<T>) -> bool {
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
            UnorderedBatchDecision::Empty => vec![],
        };

        let nontrivial = !out.is_empty();
        self.to_release = Some(out);
        nontrivial
    }

    fn implicit(&mut self) -> bool {
        self.to_release = Some(vec![]);
        false
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

pub struct KeyedStreamHook<K: Hash + Eq + Clone, V, Order: Ordering> {
    pub input: Rc<RefCell<FxHashMap<K, VecDeque<V>>>>, // FxHasher is deterministic
    pub to_release: Option<Vec<(K, V)>>,
    pub output: Sender<(K, V)>,
    pub batch_location: HookLocationMeta,
    pub format_item_debug: fn(&(K, V)) -> Option<String>,
    pub _order: std::marker::PhantomData<Order>,
}

impl<K: Hash + Eq + Clone, V> RuntimeHook for KeyedStreamHook<K, V, TotalOrder> {
    fn location_meta(&self) -> Option<HookLocationMeta> {
        Some(self.batch_location)
    }

    fn current_decision(&self) -> Option<bool> {
        self.to_release.as_ref().map(|v| !v.is_empty())
    }

    fn can_make_nontrivial_decision(&self) -> bool {
        #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
        !self.input.borrow().values().all(|q| q.is_empty())
    }

    fn autonomous_decision<'a>(
        &mut self,
        driver: &mut Borrowed<'a>,
        mut force_nontrivial: bool,
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

            let count = ((if force_nontrivial && remaining_nonempty_keys == 0 {
                1
            } else {
                0
            })..=queue.len())
                .generate(driver)
                .unwrap();

            let items: Vec<(K, V)> = queue.drain(0..count).map(|v| (key.clone(), v)).collect();
            self.to_release.as_mut().unwrap().extend(items);

            if count > 0 {
                force_nontrivial = false;
            }
        }

        !self.to_release.as_ref().unwrap().is_empty()
    }

    fn release_decision(&mut self, log_writer: Option<&mut dyn std::fmt::Write>) {
        if let Some(to_release) = self.to_release.take() {
            if let Some(log_writer) = log_writer {
                let (batch_location, line, caret_indent) = self.batch_location;
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
}

impl<K: Hash + Eq + Clone, V> RuntimeHook for KeyedStreamHook<K, V, NoOrder> {
    fn location_meta(&self) -> Option<HookLocationMeta> {
        Some(self.batch_location)
    }

    fn current_decision(&self) -> Option<bool> {
        self.to_release.as_ref().map(|v| !v.is_empty())
    }

    fn can_make_nontrivial_decision(&self) -> bool {
        #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
        !self.input.borrow().values().all(|q| q.is_empty())
    }

    fn autonomous_decision<'a>(
        &mut self,
        driver: &mut Borrowed<'a>,
        mut force_nontrivial: bool,
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
                let must_release = force_nontrivial && remaining_nonempty_keys == 0;
                if !must_release && produce().generate(driver).unwrap() {
                    break;
                }

                let idx = (min_index..queue.len()).generate(driver).unwrap();
                let item = queue.remove(idx).unwrap();
                self.to_release.as_mut().unwrap().push((key.clone(), item));
                force_nontrivial = false;

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

    fn release_decision(&mut self, log_writer: Option<&mut dyn std::fmt::Write>) {
        if let Some(to_release) = self.to_release.take() {
            if let Some(log_writer) = log_writer {
                let (batch_location, line, caret_indent) = self.batch_location;
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
}

/// A scripted decision for a keyed batch hook. Values are named as `(key, value)` entries;
/// how they are matched against the buffer depends on the ordering guarantee of the keyed
/// stream being batched (in-order prefixes per key for `TotalOrder` values, multisets per
/// key for `NoOrder` values).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum KeyedBatchDecision<K, V> {
    /// Release exactly these `(key, value)` entries.
    Values(Vec<(K, V)>),
    /// Release everything that has arrived by the time the tick fires.
    All,
    /// Release an empty batch, holding everything buffered.
    Empty,
}

impl<K, V> ScriptDecision for KeyedBatchDecision<K, V>
where
    K: serde::Serialize + serde::de::DeserializeOwned,
    V: serde::Serialize + serde::de::DeserializeOwned,
{
    fn describe(&self) -> String {
        match self {
            KeyedBatchDecision::Values(values) => {
                format!("release_values({} entr(ies))", values.len())
            }
            KeyedBatchDecision::All => "release_all()".to_owned(),
            KeyedBatchDecision::Empty => "release_empty()".to_owned(),
        }
    }
}

fn keyed_buffer_len<K: Hash + Eq, V>(input: &FxHashMap<K, VecDeque<V>>) -> usize {
    #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
    input.values().map(VecDeque::len).sum()
}

fn describe_keyed_pending<K: Hash + Eq, V>(input: &FxHashMap<K, VecDeque<V>>) -> Option<String> {
    let total = keyed_buffer_len(input);
    #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
    let keys = input.values().filter(|q| !q.is_empty()).count();
    (total > 0).then(|| format!("{} buffered item(s) across {} key(s)", total, keys))
}

impl<K, V> ScriptableHook for KeyedStreamHook<K, V, TotalOrder>
where
    K: Hash + Eq + Clone + serde::Serialize + serde::de::DeserializeOwned,
    V: serde::Serialize + serde::de::DeserializeOwned + PartialEq,
{
    type Decision = KeyedBatchDecision<K, V>;
    type Status = BatchStatus;

    fn is_honorable(&self, decision: &KeyedBatchDecision<K, V>) -> Result<bool, String> {
        let input = self.input.borrow();
        match decision {
            KeyedBatchDecision::Values(values) => {
                // Each key's scripted values must match that key's buffered prefix in
                // order (cross-key interleaving in the script is irrelevant).
                let mut consumed: FxHashMap<&K, usize> = FxHashMap::default();
                for (position, (key, expected)) in values.iter().enumerate() {
                    let offset = consumed.entry(key).or_insert(0);
                    match input.get(key).and_then(|queue| queue.get(*offset)) {
                        Some(buffered) if buffered == expected => {}
                        Some(_) => {
                            return Err(format!(
                                "release_values: buffered item at per-key prefix position {} did not match the expected value (scripted entry {})",
                                *offset, position
                            ));
                        }
                        None => return Ok(false),
                    }
                    *offset += 1;
                }
                Ok(true)
            }
            KeyedBatchDecision::All | KeyedBatchDecision::Empty => Ok(true),
        }
    }

    fn is_nontrivial(&self, decision: &KeyedBatchDecision<K, V>) -> bool {
        match decision {
            KeyedBatchDecision::Values(values) => !values.is_empty(),
            KeyedBatchDecision::All => self.can_make_nontrivial_decision(),
            KeyedBatchDecision::Empty => false,
        }
    }

    fn apply(&mut self, decision: KeyedBatchDecision<K, V>) -> bool {
        let mut input = self.input.borrow_mut();
        let out: Vec<(K, V)> = match decision {
            KeyedBatchDecision::Values(values) => values
                .into_iter()
                .map(|(key, _expected)| {
                    // `is_honorable` verified each key's scripted values match that
                    // key's buffered prefix, so popping fronts in script order releases
                    // exactly the named items.
                    let item = input.get_mut(&key).unwrap().pop_front().unwrap();
                    (key, item)
                })
                .collect(),
            KeyedBatchDecision::All => {
                let mut out = vec![];
                #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
                for (key, queue) in input.iter_mut() {
                    out.extend(queue.drain(..).map(|v| (key.clone(), v)));
                }
                out
            }
            KeyedBatchDecision::Empty => vec![],
        };

        let nontrivial = !out.is_empty();
        self.to_release = Some(out);
        nontrivial
    }

    fn implicit(&mut self) -> bool {
        self.to_release = Some(vec![]);
        false
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

impl<K, V> ScriptableHook for KeyedStreamHook<K, V, NoOrder>
where
    K: Hash + Eq + Clone + serde::Serialize + serde::de::DeserializeOwned,
    V: serde::Serialize + serde::de::DeserializeOwned + PartialEq,
{
    type Decision = KeyedBatchDecision<K, V>;
    type Status = BatchStatus;

    fn is_honorable(&self, decision: &KeyedBatchDecision<K, V>) -> Result<bool, String> {
        match decision {
            KeyedBatchDecision::Values(values) => {
                // Values are matched per key as multisets: duplicates request the
                // corresponding number of equal buffered items under that key.
                let input = self.input.borrow();
                let mut unmatched: Vec<(&K, &V)> = values.iter().map(|(k, v)| (k, v)).collect();
                #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
                for (key, queue) in input.iter() {
                    for buffered in queue {
                        if let Some(idx) = unmatched
                            .iter()
                            .position(|(k, v)| *k == key && *v == buffered)
                        {
                            unmatched.swap_remove(idx);
                        }
                    }
                }
                Ok(unmatched.is_empty())
            }
            KeyedBatchDecision::All | KeyedBatchDecision::Empty => Ok(true),
        }
    }

    fn is_nontrivial(&self, decision: &KeyedBatchDecision<K, V>) -> bool {
        match decision {
            KeyedBatchDecision::Values(values) => !values.is_empty(),
            KeyedBatchDecision::All => self.can_make_nontrivial_decision(),
            KeyedBatchDecision::Empty => false,
        }
    }

    fn apply(&mut self, decision: KeyedBatchDecision<K, V>) -> bool {
        let mut input = self.input.borrow_mut();
        let out: Vec<(K, V)> = match decision {
            KeyedBatchDecision::Values(values) => values
                .into_iter()
                .map(|(key, expected)| {
                    let queue = input.get_mut(&key).unwrap();
                    let idx = queue.iter().position(|item| *item == expected).unwrap();
                    let item = queue.remove(idx).unwrap();
                    (key, item)
                })
                .collect(),
            KeyedBatchDecision::All => {
                let mut out = vec![];
                #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
                for (key, queue) in input.iter_mut() {
                    out.extend(queue.drain(..).map(|v| (key.clone(), v)));
                }
                out
            }
            KeyedBatchDecision::Empty => vec![],
        };

        let nontrivial = !out.is_empty();
        self.to_release = Some(out);
        nontrivial
    }

    fn implicit(&mut self) -> bool {
        self.to_release = Some(vec![]);
        false
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
    fn location_meta(&self) -> Option<HookLocationMeta> {
        Some(self.batch_location)
    }

    fn current_decision(&self) -> Option<bool> {
        self.to_release.as_ref().map(|t| t.1)
    }

    fn can_make_nontrivial_decision(&self) -> bool {
        !self.input.borrow().is_empty()
    }

    fn is_ready(&self) -> bool {
        !self.input.borrow().is_empty() || self.last_released.is_some()
    }

    fn autonomous_decision<'a>(
        &mut self,
        driver: &mut Borrowed<'a>,
        force_nontrivial: bool,
    ) -> bool {
        let mut current_input = self.input.borrow_mut();
        if current_input.is_empty() {
            if force_nontrivial {
                panic!("Cannot make nontrivial decision when there is no input");
            }

            if let Some(last) = &self.last_released {
                // Re-release the last item
                self.to_release = Some((last.clone(), false));
                false
            } else {
                panic!("No input and no last released item to re-release");
            }
        } else if !force_nontrivial
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

    fn release_decision(&mut self, log_writer: Option<&mut dyn std::fmt::Write>) {
        if let Some((to_release, is_new)) = self.to_release.take() {
            self.last_released = Some(to_release.clone());

            if let Some(log_writer) = log_writer {
                let (batch_location, line, caret_indent) = self.batch_location;
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
}

/// A scripted decision for a snapshot hook: which buffered version of the state the next
/// tick execution observes.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum SnapshotDecision {
    /// Reveal the first buffered version equal to this value (bincode-serialized, since it
    /// crosses the handle/hook boundary), skipping over earlier versions.
    Reveal(Vec<u8>),
    /// Advance to the next buffered version.
    RevealNext,
    /// Reveal the newest version that has arrived by the time the tick fires.
    RevealLatest,
    /// Observe the previously revealed version again.
    Keep,
}

impl ScriptDecision for SnapshotDecision {
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

impl<T: Clone + PartialEq + serde::de::DeserializeOwned> ScriptableHook for SingletonHook<T> {
    type Decision = SnapshotDecision;
    type Status = SnapshotStatus;

    fn is_honorable(&self, decision: &SnapshotDecision) -> Result<bool, String> {
        let input = self.input.borrow();
        Ok(match decision {
            SnapshotDecision::Reveal(bytes) => {
                let target: T = bincode::deserialize(bytes)
                    .expect("failed to deserialize the reveal() target value");
                input.iter().any(|version| *version == target)
            }
            SnapshotDecision::RevealNext => !input.is_empty(),
            SnapshotDecision::RevealLatest => !input.is_empty() || self.last_released.is_some(),
            SnapshotDecision::Keep => self.last_released.is_some(),
        })
    }

    fn is_nontrivial(&self, decision: &SnapshotDecision) -> bool {
        match decision {
            SnapshotDecision::Reveal(_) | SnapshotDecision::RevealNext => true,
            SnapshotDecision::RevealLatest => !self.input.borrow().is_empty(),
            SnapshotDecision::Keep => false,
        }
    }

    fn apply(&mut self, decision: SnapshotDecision) -> bool {
        match decision {
            SnapshotDecision::Reveal(bytes) => {
                let target: T = bincode::deserialize(&bytes)
                    .expect("failed to deserialize the reveal() target value");
                let mut input = self.input.borrow_mut();
                let idx = input.iter().position(|version| *version == target).unwrap();
                self.skipped_states = input.drain(0..idx).collect();
                let item = input.pop_front().unwrap();
                self.to_release = Some((item, true));
                true
            }
            SnapshotDecision::RevealNext => {
                let mut input = self.input.borrow_mut();
                self.skipped_states = vec![];
                let item = input.pop_front().unwrap();
                self.to_release = Some((item, true));
                true
            }
            SnapshotDecision::RevealLatest => {
                let mut input = self.input.borrow_mut();
                if input.is_empty() {
                    self.skipped_states = vec![];
                    self.to_release = Some((self.last_released.clone().unwrap(), false));
                    false
                } else {
                    let skip_count = input.len() - 1;
                    self.skipped_states = input.drain(0..skip_count).collect();
                    let item = input.pop_front().unwrap();
                    self.to_release = Some((item, true));
                    true
                }
            }
            SnapshotDecision::Keep => {
                self.skipped_states = vec![];
                self.to_release = Some((self.last_released.clone().unwrap(), false));
                false
            }
        }
    }

    fn implicit(&mut self) -> bool {
        if let Some(last) = &self.last_released {
            self.skipped_states = vec![];
            self.to_release = Some((last.clone(), false));
        } else {
            // `is_ready()` prevents the tick from running before the singleton has a
            // value, so this is unreachable.
            eprintln!(
                "Simulator internal error: scripted snapshot hook asked for implicit behavior with no revealed value"
            );
            std::process::abort();
        }
        false
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
    fn location_meta(&self) -> Option<HookLocationMeta> {
        Some(self.batch_location)
    }

    fn current_decision(&self) -> Option<bool> {
        self.to_release.as_ref().map(|_| true)
    }

    fn can_make_nontrivial_decision(&self) -> bool {
        !self.input.borrow().is_empty()
    }

    fn autonomous_decision<'a>(
        &mut self,
        _driver: &mut Borrowed<'a>,
        _force_nontrivial: bool,
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

    fn release_decision(&mut self, log_writer: Option<&mut dyn std::fmt::Write>) {
        if let Some(to_release) = self.to_release.take() {
            if let Some(log_writer) = log_writer {
                let (batch_location, line, caret_indent) = self.batch_location;
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
    fn location_meta(&self) -> Option<HookLocationMeta> {
        Some(self.batch_location)
    }

    fn current_decision(&self) -> Option<bool> {
        self.to_release
            .as_ref()
            .map(|v| v.iter().any(|(_, _, is_new)| *is_new))
    }

    fn can_make_nontrivial_decision(&self) -> bool {
        #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
        !self.input.borrow().values().all(|q| q.is_empty())
    }

    fn autonomous_decision<'a>(
        &mut self,
        driver: &mut Borrowed<'a>,
        mut force_nontrivial: bool,
    ) -> bool {
        let mut current_input = self.input.borrow_mut();
        self.to_release = Some(vec![]);
        #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
        let nonempty_key_count = current_input.values().filter(|q| !q.is_empty()).count();

        let mut remaining_nonempty_keys = nonempty_key_count;
        let mut any_nontrivial = false;
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

            let do_nontrivial = force_nontrivial && remaining_nonempty_keys == 0;

            if !do_nontrivial
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
                let allow_null_release = !do_nontrivial && !self.last_released.contains_key(key);
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

                    any_nontrivial |= true;
                    force_nontrivial = false;
                }
            }
        }

        any_nontrivial
    }

    fn release_decision(&mut self, log_writer: Option<&mut dyn std::fmt::Write>) {
        if let Some(to_release) = self.to_release.take() {
            if let Some(log_writer) = log_writer {
                let (batch_location, line, caret_indent) = self.batch_location;
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
}

/// A scripted decision for a keyed snapshot hook: which buffered version each key's next
/// tick execution observes.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum KeyedSnapshotDecision {
    /// For each named key, reveal the first buffered version equal to the named value
    /// (bincode-serialized `Vec<(K, V)>`, since it crosses the handle/hook boundary),
    /// skipping over earlier versions. Keys that are not named observe their previously
    /// revealed version again (or stay absent if they have never been revealed).
    Reveal(Vec<u8>),
    /// For every key, reveal the newest version that has arrived by the time the tick
    /// fires (keys with nothing newer observe their previously revealed version again).
    RevealLatest,
    /// Every key observes its previously revealed version again.
    Keep,
}

impl ScriptDecision for KeyedSnapshotDecision {
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
    type Decision = KeyedSnapshotDecision;
    type Status = KeyedSnapshotStatus;

    fn is_honorable(&self, decision: &KeyedSnapshotDecision) -> Result<bool, String> {
        let input = self.input.borrow();
        Ok(match decision {
            KeyedSnapshotDecision::Reveal(bytes) => {
                let entries: Vec<(K, V)> = bincode::deserialize(bytes)
                    .expect("failed to deserialize the reveal() target entries");
                entries.iter().all(|(key, target)| {
                    input
                        .get(key)
                        .is_some_and(|queue| queue.iter().any(|version| version == target))
                })
            }
            KeyedSnapshotDecision::RevealLatest => {
                #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
                let any_newer = input.values().any(|q| !q.is_empty());
                any_newer || !self.last_released.is_empty()
            }
            KeyedSnapshotDecision::Keep => true,
        })
    }

    fn is_nontrivial(&self, decision: &KeyedSnapshotDecision) -> bool {
        match decision {
            KeyedSnapshotDecision::Reveal(bytes) => {
                let entries: Vec<(K, V)> = bincode::deserialize(bytes)
                    .expect("failed to deserialize the reveal() target entries");
                !entries.is_empty()
            }
            KeyedSnapshotDecision::RevealLatest => {
                #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
                let any_newer = self.input.borrow().values().any(|q| !q.is_empty());
                any_newer
            }
            KeyedSnapshotDecision::Keep => false,
        }
    }

    fn apply(&mut self, decision: KeyedSnapshotDecision) -> bool {
        let mut input = self.input.borrow_mut();
        match decision {
            KeyedSnapshotDecision::Reveal(bytes) => {
                let entries: Vec<(K, V)> = bincode::deserialize(&bytes)
                    .expect("failed to deserialize the reveal() target entries");
                let mut to_release = vec![];
                let mut named: Vec<K> = vec![];
                for (key, target) in entries {
                    let queue = input.get_mut(&key).unwrap();
                    let idx = queue.iter().position(|version| *version == target).unwrap();
                    let skipped: Vec<V> = queue.drain(0..idx).collect();
                    let item = queue.pop_front().unwrap();
                    self.skipped_states.insert(key.clone(), skipped);
                    self.last_released.insert(key.clone(), item.clone());
                    to_release.push((key.clone(), item, true));
                    named.push(key);
                }
                // Unnamed keys observe their previously revealed version again.
                #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
                for (key, last) in self.last_released.iter() {
                    if !named.contains(key) {
                        to_release.push((key.clone(), last.clone(), false));
                    }
                }
                let nontrivial = !named.is_empty();
                self.to_release = Some(to_release);
                nontrivial
            }
            KeyedSnapshotDecision::RevealLatest => {
                let mut to_release = vec![];
                let mut any_nontrivial = false;
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
                        any_nontrivial = true;
                    }
                }
                self.to_release = Some(to_release);
                any_nontrivial
            }
            KeyedSnapshotDecision::Keep => {
                #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
                let to_release = self
                    .last_released
                    .iter()
                    .map(|(key, last)| (key.clone(), last.clone(), false))
                    .collect();
                self.to_release = Some(to_release);
                false
            }
        }
    }

    fn implicit(&mut self) -> bool {
        // "Nothing new": every previously revealed key observes its value again.
        #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
        let to_release = self
            .last_released
            .iter()
            .map(|(key, last)| (key.clone(), last.clone(), false))
            .collect();
        self.to_release = Some(to_release);
        false
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

impl<T> SimInlineHook for StreamOrderHook<T> {
    fn decision_is_forced(&self) -> bool {
        self.input
            .borrow()
            .as_ref()
            .is_none_or(|inputs| inputs.len() <= 1)
    }

    fn location_meta(&self) -> HookLocationMeta {
        self.batch_location
    }

    fn pending_decision(&self) -> bool {
        self.input.borrow().is_some()
    }

    fn has_decision(&self) -> bool {
        self.to_release.is_some()
    }

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

    fn release_decision(&mut self, log_writer: Option<&mut dyn std::fmt::Write>) {
        if let Some(to_release) = self.to_release.take() {
            if !to_release.is_empty()
                && let Some(log_writer) = log_writer
            {
                let (batch_location, line, caret_indent) = self.batch_location;
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

impl<T> SimInlineHook for MergeOrderedHook<T> {
    fn decision_is_forced(&self) -> bool {
        // The interleaving is only a choice when both inputs have elements.
        self.first.borrow().as_ref().is_none_or(|f| f.is_empty())
            || self.second.borrow().as_ref().is_none_or(|s| s.is_empty())
    }

    fn location_meta(&self) -> HookLocationMeta {
        self.batch_location
    }

    fn pending_decision(&self) -> bool {
        self.first.borrow().is_some() && self.second.borrow().is_some()
    }

    fn has_decision(&self) -> bool {
        self.to_release.is_some()
    }

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

    fn release_decision(&mut self, log_writer: Option<&mut dyn std::fmt::Write>) {
        if let Some(to_release) = self.to_release.take() {
            let sources = self.release_sources.take().unwrap();
            if !to_release.is_empty()
                && let Some(log_writer) = log_writer
            {
                let (batch_location, line, caret_indent) = self.batch_location;

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

impl<K: Hash + Eq + Clone, V> SimInlineHook for KeyedStreamOrderHook<K, V> {
    fn decision_is_forced(&self) -> bool {
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

    fn location_meta(&self) -> HookLocationMeta {
        self.batch_location
    }

    fn pending_decision(&self) -> bool {
        self.input.borrow().is_some()
    }

    fn has_decision(&self) -> bool {
        self.to_release.is_some()
    }

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

    fn release_decision(&mut self, log_writer: Option<&mut dyn std::fmt::Write>) {
        if let Some(to_release) = self.to_release.take() {
            if !to_release.is_empty()
                && let Some(log_writer) = log_writer
            {
                let (batch_location, line, caret_indent) = self.batch_location;
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

impl<K: Hash + Eq + Clone, V> SimInlineHook for PartiallyOrderedStreamHook<K, V> {
    fn decision_is_forced(&self) -> bool {
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

    fn location_meta(&self) -> HookLocationMeta {
        self.batch_location
    }

    fn pending_decision(&self) -> bool {
        self.input.borrow().is_some()
    }

    fn has_decision(&self) -> bool {
        self.to_release.is_some()
    }

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

    fn release_decision(&mut self, log_writer: Option<&mut dyn std::fmt::Write>) {
        if let Some(to_release) = self.to_release.take() {
            if !to_release.is_empty()
                && let Some(log_writer) = log_writer
            {
                let (batch_location, line, caret_indent) = self.batch_location;
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
}

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
    fn location_meta(&self) -> Option<HookLocationMeta> {
        Some(self.location)
    }

    fn current_decision(&self) -> Option<bool> {
        self.to_release.as_ref().map(|v| !v.is_empty())
    }

    fn can_make_nontrivial_decision(&self) -> bool {
        !self.input.borrow().is_empty()
    }

    fn autonomous_decision<'a>(
        &mut self,
        driver: &mut Borrowed<'a>,
        force_nontrivial: bool,
    ) -> bool {
        let mut current_input = self.input.borrow_mut();
        let mut out = vec![];

        // instead of a full shuffle, we only release one element at a time
        // in order to handle possible feedback cycles
        if !current_input.is_empty() {
            let must_release = force_nontrivial && out.is_empty();
            if !must_release && produce().generate(driver).unwrap() {
                // don't release anything
            } else {
                let idx = (0..current_input.len()).generate(driver).unwrap();
                let item = current_input.remove(idx).unwrap();
                out.push(item);
            }
        }

        let was_nontrivial = !out.is_empty();
        self.to_release = Some(out);
        was_nontrivial
    }

    fn release_decision(&mut self, log_writer: Option<&mut dyn std::fmt::Write>) {
        if let Some(to_release) = self.to_release.take() {
            if !to_release.is_empty()
                && let Some(log_writer) = log_writer
            {
                let (batch_location, line, caret_indent) = self.location;
                let note_str = format!(
                    "^ observered non-deterministic order: {:?}",
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

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct OrderingStatus {
    pub buffered: usize,
}

fn same_multiset<T: PartialEq>(left: &[T], right: &[T]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut unmatched: Vec<&T> = right.iter().collect();
    for item in left {
        let Some(index) = unmatched.iter().position(|other| *other == item) else {
            return false;
        };
        unmatched.swap_remove(index);
    }
    true
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

    fn is_nontrivial(&self, _decision: &Self::Decision) -> bool {
        true
    }

    fn apply(&mut self, decision: Self::Decision) -> bool {
        let mut input = self.input.borrow_mut();
        let TopLevelOrderingDecision::Next(expected) = decision;
        let index = input.iter().position(|item| item == &expected).unwrap();
        self.to_release = Some(vec![input.remove(index).unwrap()]);
        true
    }

    fn implicit(&mut self) -> bool {
        // Implicit behavior exists for tick hooks whose tick is forced to run by *other*
        // hooks; a top-level observation consists of exactly this hook, so it can never
        // be forced to run without a scripted decision.
        eprintln!(
            "Simulator internal error: implicit decision invoked on a top-level ordering hook"
        );
        std::process::abort();
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
    fn location_meta(&self) -> Option<HookLocationMeta> {
        Some(self.location)
    }

    fn current_decision(&self) -> Option<bool> {
        self.to_release.as_ref().map(|v| !v.is_empty())
    }

    fn can_make_nontrivial_decision(&self) -> bool {
        !self.input.borrow().is_empty()
    }

    fn autonomous_decision<'a>(
        &mut self,
        driver: &mut Borrowed<'a>,
        force_nontrivial: bool,
    ) -> bool {
        let mut current_input = self.input.borrow_mut();

        if current_input.is_empty() {
            if force_nontrivial {
                panic!("Cannot make nontrivial decision when there is no input");
            }
            self.to_release = Some(vec![]);
            return false;
        }

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
        true
    }

    fn release_decision(&mut self, log_writer: Option<&mut dyn std::fmt::Write>) {
        if let Some(to_release) = self.to_release.take() {
            if !to_release.is_empty()
                && let Some(log_writer) = log_writer
            {
                let (batch_location, line, caret_indent) = self.location;
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

    fn is_nontrivial(&self, _decision: &Self::Decision) -> bool {
        true
    }

    fn apply(&mut self, decision: Self::Decision) -> bool {
        let TopLevelOrderingDecision::Next(expected) = decision;
        let mut input = self.input.borrow_mut();
        let index = input.iter().position(|item| item == &expected).unwrap();
        self.to_release = Some(vec![input.remove(index).unwrap()]);
        true
    }

    fn implicit(&mut self) -> bool {
        // Implicit behavior exists for tick hooks whose tick is forced to run by *other*
        // hooks; a top-level observation consists of exactly this hook, so it can never
        // be forced to run without a scripted decision.
        eprintln!("Simulator internal error: implicit decision invoked on a top-level fold hook");
        std::process::abort();
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
    fn location_meta(&self) -> Option<HookLocationMeta> {
        Some(self.location)
    }

    fn current_decision(&self) -> Option<bool> {
        self.to_release.as_ref().map(|v| !v.is_empty())
    }

    fn can_make_nontrivial_decision(&self) -> bool {
        #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
        !self.input.borrow().values().all(|q| q.is_empty())
    }

    fn autonomous_decision<'a>(
        &mut self,
        driver: &mut Borrowed<'a>,
        force_nontrivial: bool,
    ) -> bool {
        let mut current_input = self.input.borrow_mut();

        // Collect non-empty keys with their queue lengths
        #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
        let nonempty_keys: Vec<(K, usize)> = current_input
            .iter()
            .filter(|(_, q)| !q.is_empty())
            .map(|(k, q)| (k.clone(), q.len()))
            .collect();

        if nonempty_keys.is_empty() {
            self.to_release = Some(vec![]);
            return false;
        }

        // Decide whether to release anything
        if !force_nontrivial && produce().generate(driver).unwrap() {
            self.to_release = Some(vec![]);
            return false;
        }

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
        true
    }

    fn release_decision(&mut self, log_writer: Option<&mut dyn std::fmt::Write>) {
        if let Some(to_release) = self.to_release.take() {
            if !to_release.is_empty()
                && let Some(log_writer) = log_writer
            {
                let (batch_location, line, caret_indent) = self.location;
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
}

/// Top-level variant of [`PartiallyOrderedStreamHook`]. Same one-at-a-time release
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
    fn location_meta(&self) -> Option<HookLocationMeta> {
        Some(self.location)
    }

    fn current_decision(&self) -> Option<bool> {
        self.to_release.as_ref().map(|v| !v.is_empty())
    }

    fn can_make_nontrivial_decision(&self) -> bool {
        #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
        !self.input.borrow().values().all(|q| q.is_empty())
    }

    fn autonomous_decision<'a>(
        &mut self,
        driver: &mut Borrowed<'a>,
        force_nontrivial: bool,
    ) -> bool {
        let mut current_input = self.input.borrow_mut();

        #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
        let nonempty_keys: Vec<K> = current_input
            .iter()
            .filter(|(_, q)| !q.is_empty())
            .map(|(k, _)| k.clone())
            .collect();

        if nonempty_keys.is_empty() {
            self.to_release = Some(vec![]);
            return false;
        }

        if !force_nontrivial && produce().generate(driver).unwrap() {
            self.to_release = Some(vec![]);
            return false;
        }

        // Pick which key to release from
        let key_idx = (0..nonempty_keys.len()).generate(driver).unwrap();
        let key = &nonempty_keys[key_idx];

        // Always take from the front to preserve within-key order
        let item = current_input.get_mut(key).unwrap().pop_front().unwrap();

        self.to_release = Some(vec![(key.clone(), item)]);
        true
    }

    fn release_decision(&mut self, log_writer: Option<&mut dyn std::fmt::Write>) {
        if let Some(to_release) = self.to_release.take() {
            if !to_release.is_empty()
                && let Some(log_writer) = log_writer
            {
                let (batch_location, line, caret_indent) = self.location;
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

    fn is_nontrivial(&self, _decision: &Self::Decision) -> bool {
        true
    }

    fn apply(&mut self, decision: Self::Decision) -> bool {
        let TopLevelOrderingDecision::Next((key, expected)) = decision;
        let mut input = self.input.borrow_mut();
        let queue = input.get_mut(&key).unwrap();
        let index = queue.iter().position(|item| item == &expected).unwrap();
        let item = queue.remove(index).unwrap();
        self.to_release = Some(vec![(key, item)]);
        true
    }

    fn implicit(&mut self) -> bool {
        // Implicit behavior exists for tick hooks whose tick is forced to run by *other*
        // hooks; a top-level observation consists of exactly this hook, so it can never
        // be forced to run without a scripted decision.
        eprintln!(
            "Simulator internal error: implicit decision invoked on a top-level keyed ordering hook"
        );
        std::process::abort();
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

    fn is_nontrivial(&self, _decision: &Self::Decision) -> bool {
        true
    }

    fn apply(&mut self, decision: Self::Decision) -> bool {
        let TopLevelOrderingDecision::Next((key, _expected)) = decision;
        let mut input = self.input.borrow_mut();
        let item = input.get_mut(&key).unwrap().pop_front().unwrap();
        self.to_release = Some(vec![(key, item)]);
        true
    }

    fn implicit(&mut self) -> bool {
        // Implicit behavior exists for tick hooks whose tick is forced to run by *other*
        // hooks; a top-level observation consists of exactly this hook, so it can never
        // be forced to run without a scripted decision.
        eprintln!(
            "Simulator internal error: implicit decision invoked on a top-level partially-ordered hook"
        );
        std::process::abort();
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
    fn location_meta(&self) -> Option<HookLocationMeta> {
        Some(self.location)
    }

    fn current_decision(&self) -> Option<bool> {
        self.to_release.as_ref().map(|v| !v.is_empty())
    }

    fn can_make_nontrivial_decision(&self) -> bool {
        !self.first.borrow().is_empty() || !self.second.borrow().is_empty()
    }

    fn autonomous_decision<'a>(
        &mut self,
        driver: &mut Borrowed<'a>,
        force_nontrivial: bool,
    ) -> bool {
        let first_empty = self.first.borrow().is_empty();
        let second_empty = self.second.borrow().is_empty();

        if first_empty && second_empty {
            self.to_release = Some(vec![]);
            self.release_source = None;
            return false;
        }

        if !force_nontrivial && produce().generate(driver).unwrap() {
            // don't release anything
            self.to_release = Some(vec![]);
            self.release_source = None;
            return false;
        }

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
        true
    }

    fn release_decision(&mut self, log_writer: Option<&mut dyn std::fmt::Write>) {
        if let Some(to_release) = self.to_release.take() {
            let source = self.release_source.take();
            if !to_release.is_empty()
                && let Some(log_writer) = log_writer
            {
                let (batch_location, line, caret_indent) = self.location;
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

    fn is_nontrivial(&self, _decision: &MergeDecision<T>) -> bool {
        true
    }

    fn apply(&mut self, decision: MergeDecision<T>) -> bool {
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
        true
    }

    fn implicit(&mut self) -> bool {
        // Implicit behavior exists for tick hooks whose tick is forced to run by *other*
        // hooks; a top-level observation consists of exactly this hook, so it can never
        // be forced to run without a scripted decision.
        eprintln!(
            "Simulator internal error: implicit decision invoked on a top-level merge-ordered hook"
        );
        std::process::abort();
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

impl<K: Hash + Eq + Clone, V> SimInlineHook for KeyedMergeOrderedHook<K, V> {
    fn decision_is_forced(&self) -> bool {
        // The interleaving within a key is the only observable ordering choice, so the
        // decision is forced when no key appears in both inputs.
        let first = self.first.borrow();
        let second = self.second.borrow();
        match (first.as_ref(), second.as_ref()) {
            (Some(first), Some(second)) => !first
                .iter()
                .any(|(k, _)| second.iter().any(|(k2, _)| k2 == k)),
            _ => true,
        }
    }

    fn location_meta(&self) -> HookLocationMeta {
        self.batch_location
    }

    fn pending_decision(&self) -> bool {
        self.first.borrow().is_some() && self.second.borrow().is_some()
    }

    fn has_decision(&self) -> bool {
        self.to_release.is_some()
    }

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

    fn release_decision(&mut self, log_writer: Option<&mut dyn std::fmt::Write>) {
        if let Some(to_release) = self.to_release.take() {
            let sources = self.release_sources.take().unwrap();
            if !to_release.is_empty()
                && let Some(log_writer) = log_writer
            {
                let (batch_location, line, caret_indent) = self.batch_location;

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
    fn location_meta(&self) -> Option<HookLocationMeta> {
        Some(self.location)
    }

    fn current_decision(&self) -> Option<bool> {
        self.to_release.as_ref().map(|v| !v.is_empty())
    }

    fn can_make_nontrivial_decision(&self) -> bool {
        #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
        let first_nonempty = !self.first.borrow().values().all(|q| q.is_empty());
        #[expect(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
        let second_nonempty = !self.second.borrow().values().all(|q| q.is_empty());
        first_nonempty || second_nonempty
    }

    fn autonomous_decision<'a>(
        &mut self,
        driver: &mut Borrowed<'a>,
        force_nontrivial: bool,
    ) -> bool {
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

        if candidates.is_empty() {
            self.to_release = Some(vec![]);
            self.release_source = None;
            return false;
        }

        if !force_nontrivial && produce().generate(driver).unwrap() {
            // don't release anything
            self.to_release = Some(vec![]);
            self.release_source = None;
            return false;
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
        true
    }

    fn release_decision(&mut self, log_writer: Option<&mut dyn std::fmt::Write>) {
        if let Some(to_release) = self.to_release.take() {
            let source = self.release_source.take();
            if !to_release.is_empty()
                && let Some(log_writer) = log_writer
            {
                let (batch_location, line, caret_indent) = self.location;
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

    fn is_nontrivial(&self, _decision: &MergeDecision<(K, V), K>) -> bool {
        true
    }

    fn apply(&mut self, decision: MergeDecision<(K, V), K>) -> bool {
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
        true
    }

    fn implicit(&mut self) -> bool {
        // Implicit behavior exists for tick hooks whose tick is forced to run by *other*
        // hooks; a top-level observation consists of exactly this hook, so it can never
        // be forced to run without a scripted decision.
        eprintln!(
            "Simulator internal error: implicit decision invoked on a top-level keyed merge-ordered hook"
        );
        std::process::abort();
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

// ============================================================================
// Scripted hooks: manual control of unsafe operators from simulation tests.
//
// A hook that is bound to a test-side handle (`nondet!(... hook = handle)`) is emitted as
// [`Scripted<H>`] wrapping the ordinary hook type, which implements [`ScriptableHook`]
// (the per-kind decision semantics). The scripted hook is shared, via `Rc<RefCell<dyn
// ScriptedRuntimeHook>>`, between the scheduler's tick lists and a per-instance registry
// keyed by handle ID, so test-side handles can install decisions and read status
// on demand. Kind-specific payloads (decisions in, status out) cross the registry's
// type-erased surface bincode-serialized: the handle and the hook statically know the
// same types (the `NonDet` typing guarantees a handle can only bind to a matching
// operator), so no runtime dispatch is needed.
//
// A scripted hook never makes a decision on its own — no fuzzer entropy is ever spent on
// it; everything it releases was scripted, with one narrow exception: when there is only
// one thing the hook could possibly do (empty buffer → empty batch; no newer versions →
// re-reveal), it does that implicitly.
// ============================================================================

/// The decisions a test-side handle can script for one kind of hook. Serialized when
/// crossing the type-erased registry surface (see [`ScriptedHookControl::install_decision`]).
pub trait ScriptDecision: serde::Serialize + serde::de::DeserializeOwned {
    /// A short human-readable rendering for error messages.
    fn describe(&self) -> String;
}

/// The per-kind decision semantics of a hook that can be driven by a test-side handle,
/// implemented directly on the ordinary hook types ([`StreamHook`], [`SingletonHook`]).
/// The generic [`Scripted<H>`] shell turns any implementor into a scripted hook.
pub trait ScriptableHook: RuntimeHook {
    /// The decisions a handle can script for this kind of hook.
    type Decision: ScriptDecision;
    /// This kind's view of its pending, undecided input, read on demand by the test-side
    /// handle (for `pause_until_*` predicates).
    type Status: serde::Serialize;

    /// Whether the decision could be honored right now, against the current buffer.
    /// Returns an error when the decision is already known to be invalid rather than
    /// merely waiting for more input. The host scheduler reports that error without
    /// unwinding through the generated dylib boundary.
    fn is_honorable(&self, decision: &Self::Decision) -> Result<bool, String>;
    /// Whether applying this currently honorable decision would release new data. Used to
    /// decide whether this hook can make its tick runnable; empty/keep decisions are only
    /// consumed when another hook makes the same tick run.
    fn is_nontrivial(&self, decision: &Self::Decision) -> bool;
    /// Applies the decision, staging the release. Returns whether it was nontrivial.
    fn apply(&mut self, decision: Self::Decision) -> bool;
    /// Stages the only-possibility implicit behavior (empty batch / re-reveal). Called
    /// only when the hook's tick was forced to run by *other* hooks while this hook had
    /// no queued decision (nothing to decide, or held). Top-level observation hooks abort
    /// here: an observation consists of exactly one hook, so nothing can force it to run
    /// without a scripted decision.
    fn implicit(&mut self) -> bool;
    /// The current pending-input view.
    fn status(&self) -> Self::Status;
    /// A human-readable rendering of the pending, undecided input (e.g. `2 buffered
    /// item(s): [1, 2]`), or `None` when there is nothing pending. Used by the
    /// forgotten-hook and stuck-decision error messages.
    fn describe_pending(&self) -> Option<String>;
}

/// The scheduler action selected by a scripted decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptTarget {
    /// A real tick. Multiple decisions for different hooks may join one tick group.
    Tick((&'static str, Option<u32>)),
    /// One top-level observation hook, treated as its own virtual tick.
    Observation {
        location: (&'static str, Option<u32>),
        hook_id: usize,
    },
}

impl ScriptTarget {
    pub fn location(self) -> (&'static str, Option<u32>) {
        match self {
            ScriptTarget::Tick(location) | ScriptTarget::Observation { location, .. } => location,
        }
    }
}

/// Handle-facing state shared by pre-tick, top-level observation, and inline scripted hooks.
pub trait ScriptedHookControl {
    fn target(&self) -> ScriptTarget;
    fn location_meta(&self) -> Option<HookLocationMeta>;
    fn has_decision(&self) -> bool;
    fn describe_decision(&self) -> Option<String>;
    fn describe_pending(&self) -> Option<String>;
    fn install_decision(&mut self, decision_blob: &[u8]);
    fn status_blob(&self) -> Vec<u8>;
    fn set_hold(&mut self, hold: bool);
    fn release_hold(&mut self);
    fn set_auto_pause(&mut self, auto_pause: bool);
}

/// The scheduler-facing interface of a pre-tick or top-level observation scripted hook.
pub trait ScriptedRuntimeHook: RuntimeHook + ScriptedHookControl {
    /// Executes and releases this hook's queued (or implicit forced) scripted decision,
    /// without consulting the entropy driver. Returns whether it was nontrivial.
    fn run_decision(&mut self, log_writer: Option<&mut dyn std::fmt::Write>) -> bool;

    /// Whether this hook currently *blocks* its tick from running: a queued decision that
    /// is not yet honorable prevents the tick from executing at all, so the decision can
    /// never be skipped over or paired with a different execution.
    fn blocks_tick(&self) -> bool;

    /// Called at every scheduling boundary (async dataflows exhausted, ticks about to be
    /// considered). Reports an error if the hook is *forgotten*: it faces a meaningful
    /// choice (pending input) with no queued decision and no hold.
    fn boundary_check(&self) -> Result<(), String>;
}

/// A scripted observation reached while a tick DFIR is executing.
pub trait ScriptedInlineHook: ScriptedHookControl {
    fn pending_decision(&self) -> bool;
    /// Consumes the queued decision (or the forced implicit behavior) and releases it.
    /// Returns an error message instead of panicking: this code is monomorphized into the
    /// generated dylib, and a panic must not unwind across that boundary — the host
    /// scheduler reports the error.
    fn run_decision(&mut self, log_writer: Option<&mut dyn std::fmt::Write>) -> Result<(), String>;
    fn location_meta(&self) -> HookLocationMeta;
}

/// Scripted hooks keyed by the scheduler action they feed. Top-level locations become
/// observations; tick locations become ordinary tick inputs.
pub type ScriptedTickHooks<Key> =
    HashMap<(Key, Option<u32>), Vec<Rc<RefCell<dyn ScriptedRuntimeHook>>>>;

pub type ScriptedInlineHooks<Key> =
    HashMap<(Key, Option<u32>), Vec<Rc<RefCell<dyn ScriptedInlineHook>>>>;

/// The per-instance registry of every scripted hook, keyed by handle ID.
pub type ScriptedHookRegistry =
    std::collections::BTreeMap<usize, Rc<RefCell<dyn ScriptedHookControl>>>;

fn render_forgotten_error(location: HookLocationMeta, pending: &str) -> String {
    let (loc, line, caret) = location;
    format!(
        "scripted hook has buffered input but no decision:\n--> {loc}\n |{line}\n |{caret}^ {pending}\nhelp: script a decision (e.g. `.release(..)` / `.reveal(..)`) or call `.pause()` if buffering is intended"
    )
}

/// The scripted variation of a hook: wraps the ordinary hook type (which owns the buffers
/// and implements the decision semantics via [`ScriptableHook`]) together with the script
/// state: at most one queued decision (decisions are installed group by group, and a
/// hook's earlier decision is always consumed before its next one arrives), plus a `hold`
/// flag for the `pause()` family. The fields are consulted in order — decision present →
/// normal decision semantics; otherwise hold set → held; otherwise pending input → the
/// *forgotten* state, which the scheduler's boundary scan reports as an error.
pub struct Scripted<H: ScriptableHook> {
    core: H,
    target: ScriptTarget,
    next_decision: Option<H::Decision>,
    hold: bool,
    auto_pause: bool,
}

impl<H: ScriptableHook> Scripted<H> {
    /// Wraps `core` for scripting. `tick` is the execution unit the hook feeds. Called
    /// from generated code.
    pub fn new(core: H, target: ScriptTarget) -> Self {
        Scripted {
            core,
            target,
            next_decision: None,
            hold: false,
            auto_pause: false,
        }
    }

    /// Consumes the queued decision if honorable (or the implicit only-possibility
    /// behavior if none is queued), staging the release on the core hook.
    fn scripted_step(&mut self) -> bool {
        match self.next_decision.take() {
            Some(decision) if matches!(self.core.is_honorable(&decision), Ok(true)) => {
                self.core.apply(decision)
            }
            Some(decision) => {
                // `blocks_tick()` prevents the tick from running while a queued decision
                // is not honorable, so this is unreachable.
                eprintln!(
                    "Simulator internal error: tick ran while scripted decision {} was not honorable",
                    decision.describe()
                );
                std::process::abort();
            }
            None => {
                // The boundary scan guarantees this hook either has nothing pending or is
                // held, so the implicit "nothing new" behavior is the only possibility.
                self.core.implicit()
            }
        }
    }
}

impl<H: ScriptableHook> RuntimeHook for Scripted<H> {
    fn current_decision(&self) -> Option<bool> {
        self.core.current_decision()
    }

    fn can_make_nontrivial_decision(&self) -> bool {
        // Pending input alone never makes a scripted hook offer work (the boundary scan is
        // the mechanism that reacts to it). Only an honorable queued decision that releases
        // new data makes the tick runnable; empty/keep decisions are consumed when another
        // hook on the same tick makes it run.
        self.next_decision.as_ref().is_some_and(|decision| {
            matches!(self.core.is_honorable(decision), Ok(true))
                && self.core.is_nontrivial(decision)
        })
    }

    fn is_ready(&self) -> bool {
        self.core.is_ready()
    }

    fn location_meta(&self) -> Option<HookLocationMeta> {
        self.core.location_meta()
    }

    fn autonomous_decision<'a>(
        &mut self,
        _driver: &mut Borrowed<'a>,
        _force_nontrivial: bool,
    ) -> bool {
        eprintln!("Simulator internal error: autonomous_decision called on a scripted hook");
        std::process::abort();
    }

    fn release_decision(&mut self, mut log_writer: Option<&mut dyn std::fmt::Write>) {
        if let Some(w) = log_writer.as_mut() {
            let _ = writeln!(w, "{}", "(scripted)".color(colored::Color::Cyan));
        }
        self.core.release_decision(log_writer);
    }
}

impl<H: ScriptableHook> ScriptedHookControl for Scripted<H> {
    fn target(&self) -> ScriptTarget {
        self.target
    }

    fn location_meta(&self) -> Option<HookLocationMeta> {
        self.core.location_meta()
    }

    fn has_decision(&self) -> bool {
        self.next_decision.is_some()
    }

    fn describe_decision(&self) -> Option<String> {
        self.next_decision.as_ref().map(|d| d.describe())
    }

    fn describe_pending(&self) -> Option<String> {
        self.core.describe_pending()
    }

    fn install_decision(&mut self, decision_blob: &[u8]) {
        assert!(
            self.next_decision.is_none(),
            "internal error: installed a scripted decision while one was still queued"
        );
        self.next_decision = Some(bincode::deserialize(decision_blob).expect(
            "internal error: scripted decision blob did not match the hook's decision type",
        ));
        self.hold = self.auto_pause;
    }

    fn status_blob(&self) -> Vec<u8> {
        bincode::serialize(&self.core.status()).unwrap()
    }

    fn set_hold(&mut self, hold: bool) {
        self.hold = hold;
    }

    fn release_hold(&mut self) {
        self.hold = self.auto_pause;
    }

    fn set_auto_pause(&mut self, auto_pause: bool) {
        self.auto_pause = auto_pause;
    }
}

impl<H: ScriptableHook> ScriptedRuntimeHook for Scripted<H> {
    fn run_decision(&mut self, log_writer: Option<&mut dyn std::fmt::Write>) -> bool {
        let made_nontrivial_decision = self.scripted_step();
        self.release_decision(log_writer);
        made_nontrivial_decision
    }

    fn blocks_tick(&self) -> bool {
        self.next_decision
            .as_ref()
            .is_some_and(|d| matches!(self.core.is_honorable(d), Ok(false)))
    }

    fn boundary_check(&self) -> Result<(), String> {
        if let Some(Err(message)) = self
            .next_decision
            .as_ref()
            .map(|decision| self.core.is_honorable(decision))
        {
            return Err(message);
        }

        // "Faces a meaningful choice" is exactly the autonomous question
        // `can_make_nontrivial_decision` — for any hook kind.
        if self.next_decision.is_none() && !self.hold && self.core.can_make_nontrivial_decision() {
            Err(render_forgotten_error(
                self.core
                    .location_meta()
                    .unwrap_or(("unknown location", "", "")),
                &self
                    .core
                    .describe_pending()
                    .unwrap_or_else(|| "pending input".to_owned()),
            ))
        } else {
            Ok(())
        }
    }
}

/// The per-kind scripted-decision semantics of an inline (in-tick) hook, implemented
/// directly on the ordinary inline hook types ([`StreamOrderHook`],
/// [`MergeOrderedHook`], ...). The generic [`ScriptedInline<H>`] shell turns any
/// implementor into a scripted inline hook.
pub trait ScriptableInlineHook: SimInlineHook {
    /// The decisions a handle can script for this kind of inline hook.
    type Decision: ScriptDecision;

    /// Consumes the pending in-tick input together with the queued decision (`None` when
    /// the test scripted nothing, which is only valid when the decision is forced),
    /// staging the release. Returns an error message instead of panicking: this code is
    /// monomorphized into the generated dylib, and a panic must not unwind across that
    /// boundary — the host scheduler reports the error.
    fn apply_scripted(&mut self, decision: Option<Self::Decision>) -> Result<(), String>;

    /// A human-readable rendering of the pending, undecided input, or `None` when there
    /// is nothing pending.
    fn describe_pending(&self) -> Option<String>;

    /// The current pending-input view, bincode-serialized for the type-erased registry
    /// surface (the handle statically knows the matching status type).
    fn status_blob(&self) -> Vec<u8>;
}

impl<T> ScriptableInlineHook for StreamOrderHook<T>
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq,
{
    type Decision = InlineOrderingDecision<T>;

    fn apply_scripted(&mut self, decision: Option<Self::Decision>) -> Result<(), String> {
        let input = self.input.borrow_mut().take().unwrap();
        let output = match decision {
            Some(InlineOrderingDecision::Order(values)) => {
                if !same_multiset(&input, &values) {
                    return Err(format!(
                        "scripted in-tick ordering decision must contain exactly all pending input values ({} pending, {} scripted)",
                        input.len(),
                        values.len(),
                    ));
                }
                values
            }
            None if input.len() <= 1 => input,
            None => {
                return Err(render_forgotten_error(
                    self.batch_location,
                    &format!(
                        "{} in-tick values require an explicit ordering",
                        input.len()
                    ),
                ));
            }
        };
        self.to_release = Some(output);
        Ok(())
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

    fn status_blob(&self) -> Vec<u8> {
        bincode::serialize(&OrderingStatus {
            buffered: self.input.borrow().as_ref().map_or(0, Vec::len),
        })
        .unwrap()
    }
}

impl<K, V> ScriptableInlineHook for KeyedStreamOrderHook<K, V>
where
    K: Hash + Eq + Clone + serde::Serialize + serde::de::DeserializeOwned + PartialEq,
    V: serde::Serialize + serde::de::DeserializeOwned + PartialEq,
{
    type Decision = InlineOrderingDecision<(K, V)>;

    fn apply_scripted(&mut self, decision: Option<Self::Decision>) -> Result<(), String> {
        let input = self.input.borrow_mut().take().unwrap();
        let ordered: Vec<(K, V)> = match decision {
            Some(InlineOrderingDecision::Order(values)) => {
                if !same_multiset(&input, &values) {
                    return Err(format!(
                        "scripted in-tick keyed ordering decision must contain exactly all pending input entries ({} pending, {} scripted)",
                        input.len(),
                        values.len(),
                    ));
                }
                values
            }
            None => {
                // Only forced when no key has more than one element (then per-key order
                // is not a choice).
                let mut seen_keys: Vec<&K> = vec![];
                for (k, _) in &input {
                    if seen_keys.contains(&k) {
                        return Err(render_forgotten_error(
                            self.batch_location,
                            &format!(
                                "{} in-tick entries require an explicit ordering",
                                input.len()
                            ),
                        ));
                    }
                    seen_keys.push(k);
                }
                input
            }
        };

        let mut grouped: FxHashMap<K, Vec<V>> = FxHashMap::default();
        for (k, v) in ordered {
            grouped.entry(k).or_default().push(v);
        }
        self.to_release = Some(grouped);
        Ok(())
    }

    fn describe_pending(&self) -> Option<String> {
        let input = self.input.borrow();
        input
            .as_ref()
            .map(|values| format!("{} in-tick keyed ordering entr(ies)", values.len()))
    }

    fn status_blob(&self) -> Vec<u8> {
        bincode::serialize(&OrderingStatus {
            buffered: self.input.borrow().as_ref().map_or(0, Vec::len),
        })
        .unwrap()
    }
}

impl<K, V> ScriptableInlineHook for PartiallyOrderedStreamHook<K, V>
where
    K: Hash + Eq + Clone + serde::Serialize + serde::de::DeserializeOwned + PartialEq,
    V: serde::Serialize + serde::de::DeserializeOwned + PartialEq,
{
    type Decision = InlineOrderingDecision<(K, V)>;

    fn apply_scripted(&mut self, decision: Option<Self::Decision>) -> Result<(), String> {
        let input = self.input.borrow_mut().take().unwrap();
        let output = match decision {
            Some(InlineOrderingDecision::Order(values)) => {
                // The scripted sequence must be a valid interleaving: same entries, and
                // each key's subsequence must preserve that key's input order.
                let valid = values.len() == input.len() && {
                    let mut distinct_keys: Vec<&K> = vec![];
                    for (k, _) in &input {
                        if !distinct_keys.contains(&k) {
                            distinct_keys.push(k);
                        }
                    }
                    distinct_keys.iter().all(|key| {
                        let input_seq: Vec<&V> = input
                            .iter()
                            .filter(|(k, _)| k == *key)
                            .map(|(_, v)| v)
                            .collect();
                        let scripted_seq: Vec<&V> = values
                            .iter()
                            .filter(|(k, _)| k == *key)
                            .map(|(_, v)| v)
                            .collect();
                        input_seq == scripted_seq
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
            None => {
                // Only forced when at most one distinct key is present (then the
                // interleaving is not a choice).
                let mut distinct_keys: Vec<&K> = vec![];
                for (k, _) in &input {
                    if !distinct_keys.contains(&k) {
                        distinct_keys.push(k);
                    }
                }
                if distinct_keys.len() > 1 {
                    return Err(render_forgotten_error(
                        self.batch_location,
                        &format!(
                            "{} in-tick entries require an explicit interleaving",
                            input.len()
                        ),
                    ));
                }
                input
            }
        };
        self.to_release = Some(output);
        Ok(())
    }

    fn describe_pending(&self) -> Option<String> {
        let input = self.input.borrow();
        input
            .as_ref()
            .map(|values| format!("{} in-tick partially-ordered entr(ies)", values.len()))
    }

    fn status_blob(&self) -> Vec<u8> {
        bincode::serialize(&OrderingStatus {
            buffered: self.input.borrow().as_ref().map_or(0, Vec::len),
        })
        .unwrap()
    }
}

/// Computes, for a scripted `merged` sequence, which source each element is drawn from
/// (`false` = first, `true` = second) such that `merged` is a valid interleaving of
/// `first` and `second` preserving each input's order. Returns `None` when no such
/// assignment exists. Equal values on both fronts are resolved by dynamic programming
/// over reachable `(i, j)` prefixes, so any valid interleaving is accepted.
fn interleaving_sources<T: PartialEq>(
    first: &[T],
    second: &[T],
    merged: &[T],
) -> Option<Vec<bool>> {
    if merged.len() != first.len() + second.len() {
        return None;
    }

    let n = first.len();
    let m = second.len();
    let mut reachable = vec![false; (n + 1) * (m + 1)];
    let at = |i: usize, j: usize| i * (m + 1) + j;
    reachable[at(0, 0)] = true;
    for i in 0..=n {
        for j in 0..=m {
            if !reachable[at(i, j)] {
                continue;
            }
            let k = i + j;
            if k == merged.len() {
                continue;
            }
            if i < n && first[i] == merged[k] {
                reachable[at(i + 1, j)] = true;
            }
            if j < m && second[j] == merged[k] {
                reachable[at(i, j + 1)] = true;
            }
        }
    }

    if !reachable[at(n, m)] {
        return None;
    }

    // Walk backwards from the full state; any reachable predecessor with a matching
    // transition lies on a complete path.
    let mut sources = vec![false; merged.len()];
    let (mut i, mut j) = (n, m);
    while i + j > 0 {
        let k = i + j - 1;
        if i > 0 && reachable[at(i - 1, j)] && first[i - 1] == merged[k] {
            sources[k] = false;
            i -= 1;
        } else {
            sources[k] = true;
            j -= 1;
        }
    }
    Some(sources)
}

impl<T> ScriptableInlineHook for MergeOrderedHook<T>
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq,
{
    type Decision = InlineOrderingDecision<T>;

    fn apply_scripted(&mut self, decision: Option<Self::Decision>) -> Result<(), String> {
        let first = self.first.borrow_mut().take().unwrap();
        let second = self.second.borrow_mut().take().unwrap();
        match decision {
            Some(InlineOrderingDecision::Order(values)) => {
                let Some(sources) = interleaving_sources(&first, &second, &values) else {
                    return Err(format!(
                        "scripted in-tick merge decision must be an interleaving of the two inputs that preserves each input's order ({} + {} pending, {} scripted)",
                        first.len(),
                        second.len(),
                        values.len(),
                    ));
                };
                self.to_release = Some(values);
                self.release_sources = Some(sources);
            }
            None => {
                // Only forced when at most one input has elements (then the
                // interleaving is not a choice).
                if !first.is_empty() && !second.is_empty() {
                    return Err(render_forgotten_error(
                        self.batch_location,
                        &format!(
                            "{} + {} in-tick values require an explicit merge order",
                            first.len(),
                            second.len()
                        ),
                    ));
                }
                let mut sources = vec![false; first.len()];
                sources.extend(std::iter::repeat_n(true, second.len()));
                let mut result = first;
                result.extend(second);
                self.to_release = Some(result);
                self.release_sources = Some(sources);
            }
        }
        Ok(())
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

    fn status_blob(&self) -> Vec<u8> {
        bincode::serialize(&MergeStatus {
            first_buffered: self.first.borrow().as_ref().map_or(0, Vec::len),
            second_buffered: self.second.borrow().as_ref().map_or(0, Vec::len),
        })
        .unwrap()
    }
}

impl<K, V> ScriptableInlineHook for KeyedMergeOrderedHook<K, V>
where
    K: Hash + Eq + Clone + serde::Serialize + serde::de::DeserializeOwned + PartialEq,
    V: serde::Serialize + serde::de::DeserializeOwned + PartialEq,
{
    type Decision = InlineOrderingDecision<(K, V)>;

    fn apply_scripted(&mut self, decision: Option<Self::Decision>) -> Result<(), String> {
        let first = self.first.borrow_mut().take().unwrap();
        let second = self.second.borrow_mut().take().unwrap();
        match decision {
            Some(InlineOrderingDecision::Order(values)) => {
                // Validate per key: each key's scripted subsequence must be a valid
                // interleaving of that key's subsequences from the two inputs
                // (cross-key order is unobservable for keyed streams).
                let error = || {
                    format!(
                        "scripted in-tick keyed merge decision must, for every key, be an interleaving of that key's entries from the two inputs that preserves each input's order ({} + {} pending, {} scripted)",
                        first.len(),
                        second.len(),
                        values.len(),
                    )
                };
                if values.len() != first.len() + second.len() {
                    return Err(error());
                }

                let mut distinct_keys: Vec<&K> = vec![];
                for (k, _) in first.iter().chain(&second).chain(&values) {
                    if !distinct_keys.contains(&k) {
                        distinct_keys.push(k);
                    }
                }

                // Per-key source assignments, consumed in scripted order below.
                let mut per_key_sources: Vec<(&K, VecDeque<bool>)> = vec![];
                for key in distinct_keys {
                    let first_seq: Vec<&V> = first
                        .iter()
                        .filter(|(k, _)| k == key)
                        .map(|(_, v)| v)
                        .collect();
                    let second_seq: Vec<&V> = second
                        .iter()
                        .filter(|(k, _)| k == key)
                        .map(|(_, v)| v)
                        .collect();
                    let scripted_seq: Vec<&V> = values
                        .iter()
                        .filter(|(k, _)| k == key)
                        .map(|(_, v)| v)
                        .collect();
                    let Some(sources) =
                        interleaving_sources(&first_seq, &second_seq, &scripted_seq)
                    else {
                        return Err(error());
                    };
                    per_key_sources.push((key, sources.into()));
                }

                let sources: Vec<bool> = values
                    .iter()
                    .map(|(k, _)| {
                        per_key_sources
                            .iter_mut()
                            .find(|(key, _)| *key == k)
                            .unwrap()
                            .1
                            .pop_front()
                            .unwrap()
                    })
                    .collect();

                self.to_release = Some(values);
                self.release_sources = Some(sources);
            }
            None => {
                // Only forced when no key appears in both inputs (then no per-key
                // interleaving is a choice).
                if first
                    .iter()
                    .any(|(k, _)| second.iter().any(|(k2, _)| k2 == k))
                {
                    return Err(render_forgotten_error(
                        self.batch_location,
                        &format!(
                            "{} + {} in-tick entries require an explicit merge order",
                            first.len(),
                            second.len()
                        ),
                    ));
                }
                let mut sources = vec![false; first.len()];
                sources.extend(std::iter::repeat_n(true, second.len()));
                let mut result = first;
                result.extend(second);
                self.to_release = Some(result);
                self.release_sources = Some(sources);
            }
        }
        Ok(())
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

    fn status_blob(&self) -> Vec<u8> {
        bincode::serialize(&MergeStatus {
            first_buffered: self.first.borrow().as_ref().map_or(0, Vec::len),
            second_buffered: self.second.borrow().as_ref().map_or(0, Vec::len),
        })
        .unwrap()
    }
}

/// The scripted variation of an inline (in-tick) hook: wraps the ordinary inline hook
/// type (which owns the buffers and implements the decision semantics via
/// [`ScriptableInlineHook`]) together with at most one queued decision.
pub struct ScriptedInline<H: ScriptableInlineHook> {
    core: H,
    target: ScriptTarget,
    next_decision: Option<H::Decision>,
}

impl<H: ScriptableInlineHook> ScriptedInline<H> {
    pub fn new(core: H, target: ScriptTarget) -> Self {
        Self {
            core,
            target,
            next_decision: None,
        }
    }
}

impl<H: ScriptableInlineHook> ScriptedHookControl for ScriptedInline<H> {
    fn target(&self) -> ScriptTarget {
        self.target
    }

    fn location_meta(&self) -> Option<HookLocationMeta> {
        Some(SimInlineHook::location_meta(&self.core))
    }

    fn has_decision(&self) -> bool {
        self.next_decision.is_some()
    }

    fn describe_decision(&self) -> Option<String> {
        self.next_decision.as_ref().map(ScriptDecision::describe)
    }

    fn describe_pending(&self) -> Option<String> {
        self.core.describe_pending()
    }

    fn install_decision(&mut self, decision_blob: &[u8]) {
        assert!(self.next_decision.is_none());
        self.next_decision = Some(
            bincode::deserialize(decision_blob)
                .expect("internal error: scripted inline decision had the wrong type"),
        );
    }

    fn status_blob(&self) -> Vec<u8> {
        self.core.status_blob()
    }

    fn set_hold(&mut self, _hold: bool) {}
    fn release_hold(&mut self) {}
    fn set_auto_pause(&mut self, _auto_pause: bool) {}
}

impl<H: ScriptableInlineHook> ScriptedInlineHook for ScriptedInline<H> {
    fn pending_decision(&self) -> bool {
        self.core.pending_decision()
    }

    fn run_decision(&mut self, log_writer: Option<&mut dyn std::fmt::Write>) -> Result<(), String> {
        self.core.apply_scripted(self.next_decision.take())?;
        self.core.release_decision(log_writer);
        Ok(())
    }

    fn location_meta(&self) -> HookLocationMeta {
        SimInlineHook::location_meta(&self.core)
    }
}

#[cfg(test)]
mod maybe_debug_tests {
    struct NotDebuggable;

    #[derive(Debug)]
    struct Debuggable;

    #[test]
    fn test_non_debug_type_returns_none() {
        let fmt_fn: fn(&NotDebuggable) -> Option<String> = crate::__maybe_debug__!(NotDebuggable);
        assert_eq!(fmt_fn(&NotDebuggable), None);
    }

    #[test]
    fn test_debug_type_returns_some() {
        let fmt_fn: fn(&Debuggable) -> Option<String> = crate::__maybe_debug__!(Debuggable);
        assert_eq!(fmt_fn(&Debuggable), Some("Debuggable".to_owned()));
    }

    #[test]
    fn test_primitive_debug() {
        let fmt_fn: fn(&i32) -> Option<String> = crate::__maybe_debug__!(i32);
        assert_eq!(fmt_fn(&42), Some("42".to_owned()));
    }
}
