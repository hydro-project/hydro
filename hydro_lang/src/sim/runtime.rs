use std::cell::RefCell;
use std::collections::VecDeque;
use std::fmt::Debug;
use std::hash::Hash;
use std::rc::Rc;

use bolero::generator::bolero_generator::driver::object::Borrowed;
use bolero::{ValueGenerator, produce};
use colored::Colorize;
use dfir_rs::rustc_hash::FxHashMap;
use tokio::sync::mpsc::UnboundedSender;

use crate::live_collections::stream::{NoOrder, Ordering, TotalOrder};

#[macro_export]
#[doc(hidden)]
macro_rules! __maybe_debug__ {
    () => {{
        trait NotDebug {
            fn format_debug(&self) -> Option<String> {
                None
            }
        }

        impl<T> NotDebug for T {}
        struct IsDebug<T>(std::marker::PhantomData<T>);
        impl<T: std::fmt::Debug> IsDebug<T> {
            fn format_debug(v: &T) -> Option<String> {
                Some(format!("{:?}", v))
            }
        }
        IsDebug::format_debug
    }};
}

pub trait SimHook {
    fn current_decision(&self) -> Option<bool>;
    fn can_make_nontrivial_decision(&self) -> bool;
    fn autonomous_decision<'a>(
        &mut self,
        driver: &mut Borrowed<'a>,
        force_nontrivial: bool,
    ) -> bool;
    fn release_decision(&mut self, log_writer: &mut dyn std::fmt::Write);
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

pub struct StreamHook<T, Order: Ordering> {
    pub input: Rc<RefCell<VecDeque<T>>>,
    pub to_release: Option<Vec<T>>,
    pub output: UnboundedSender<T>,
    pub batch_location: (&'static str, &'static str, &'static str),
    pub format_item_debug: fn(&T) -> Option<String>,
    pub _order: std::marker::PhantomData<Order>,
}

impl<T> SimHook for StreamHook<T, TotalOrder> {
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

    fn release_decision(&mut self, log_writer: &mut dyn std::fmt::Write) {
        if let Some(to_release) = self.to_release.take() {
            let (batch_location, line, caret_indent) = self.batch_location;
            let note_str = if to_release.is_empty() {
                "^ releasing no items".to_string()
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

            let _ = writeln!(
                log_writer,
                "{} {}",
                "-->".color(colored::Color::Blue),
                batch_location
            );

            let _ = writeln!(log_writer, " {}{}", "|".color(colored::Color::Blue), line);

            let _ = writeln!(
                log_writer,
                " {}{}{}",
                "|".color(colored::Color::Blue),
                caret_indent,
                note_str.color(colored::Color::Green)
            );

            for item in to_release {
                self.output.send(item).unwrap();
            }
        } else {
            panic!("No decision to release");
        }
    }
}

impl<T> SimHook for StreamHook<T, NoOrder> {
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

    fn release_decision(&mut self, log_writer: &mut dyn std::fmt::Write) {
        if let Some(to_release) = self.to_release.take() {
            let (batch_location, line, caret_indent) = self.batch_location;
            let note_str = if to_release.is_empty() {
                "^ releasing no items".to_string()
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

            let _ = writeln!(
                log_writer,
                "{} {}",
                "-->".color(colored::Color::Blue),
                batch_location
            );

            let _ = writeln!(log_writer, " {}{}", "|".color(colored::Color::Blue), line);

            let _ = writeln!(
                log_writer,
                " {}{}{}",
                "|".color(colored::Color::Blue),
                caret_indent,
                note_str.color(colored::Color::Green)
            );

            for item in to_release {
                self.output.send(item).unwrap();
            }
        } else {
            panic!("No decision to release");
        }
    }
}

pub struct KeyedStreamHook<K: Hash + Eq + Clone, V, Order: Ordering> {
    pub input: Rc<RefCell<FxHashMap<K, VecDeque<V>>>>, // FxHasher is deterministic
    pub to_release: Option<Vec<(K, V)>>,
    pub output: UnboundedSender<(K, V)>,
    pub batch_location: (&'static str, &'static str, &'static str),
    pub format_item_debug: fn(&(K, V)) -> Option<String>,
    pub _order: std::marker::PhantomData<Order>,
}

impl<K: Hash + Eq + Clone, V> SimHook for KeyedStreamHook<K, V, TotalOrder> {
    fn current_decision(&self) -> Option<bool> {
        self.to_release.as_ref().map(|v| !v.is_empty())
    }

    fn can_make_nontrivial_decision(&self) -> bool {
        !self.input.borrow().values().all(|q| q.is_empty())
    }

    fn autonomous_decision<'a>(
        &mut self,
        driver: &mut Borrowed<'a>,
        mut force_nontrivial: bool,
    ) -> bool {
        let mut current_input = self.input.borrow_mut();
        self.to_release = Some(vec![]);
        let nonempty_key_count = current_input.values().filter(|q| !q.is_empty()).count();

        let mut remaining_nonempty_keys = nonempty_key_count;
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

    fn release_decision(&mut self, log_writer: &mut dyn std::fmt::Write) {
        if let Some(to_release) = self.to_release.take() {
            let (batch_location, line, caret_indent) = self.batch_location;
            let note_str = if to_release.is_empty() {
                "^ releasing no items".to_string()
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

            let _ = writeln!(
                log_writer,
                "{} {}",
                "-->".color(colored::Color::Blue),
                batch_location
            );

            let _ = writeln!(log_writer, " {}{}", "|".color(colored::Color::Blue), line);

            let _ = writeln!(
                log_writer,
                " {}{}{}",
                "|".color(colored::Color::Blue),
                caret_indent,
                note_str.color(colored::Color::Green)
            );

            for item in to_release {
                self.output.send(item).unwrap();
            }
        } else {
            panic!("No decision to release");
        }
    }
}

pub struct SingletonHook<T> {
    input: Rc<RefCell<VecDeque<T>>>,
    to_release: Option<(T, bool)>, // (data, is new)
    last_released: Option<T>,
    skipped_states: Vec<T>,
    output: UnboundedSender<T>,
    batch_location: (&'static str, &'static str, &'static str),
    format_item_debug: fn(&T) -> Option<String>,
}

impl<T: Clone> SingletonHook<T> {
    pub fn new(
        input: Rc<RefCell<VecDeque<T>>>,
        output: UnboundedSender<T>,
        batch_location: (&'static str, &'static str, &'static str),
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

impl<T: Clone> SimHook for SingletonHook<T> {
    fn current_decision(&self) -> Option<bool> {
        self.to_release.as_ref().map(|t| t.1)
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

    fn release_decision(&mut self, log_writer: &mut dyn std::fmt::Write) {
        if let Some((to_release, is_new)) = self.to_release.take() {
            self.last_released = Some(to_release.clone());

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

            let _ = writeln!(
                log_writer,
                "{} {}",
                "-->".color(colored::Color::Blue),
                batch_location
            );

            let _ = writeln!(log_writer, " {}{}", "|".color(colored::Color::Blue), line);

            let _ = writeln!(
                log_writer,
                " {}{}{}",
                "|".color(colored::Color::Blue),
                caret_indent,
                note_str.color(colored::Color::Green)
            );

            self.output.send(to_release).unwrap();
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
    output: UnboundedSender<(K, V)>,
    batch_location: (&'static str, &'static str, &'static str),
    format_key_debug: fn(&K) -> Option<String>,
    format_value_debug: fn(&V) -> Option<String>,
}

impl<K: Hash + Eq + Clone, V: Clone> KeyedSingletonHook<K, V> {
    pub fn new(
        input: Rc<RefCell<FxHashMap<K, VecDeque<V>>>>,
        output: UnboundedSender<(K, V)>,
        batch_location: (&'static str, &'static str, &'static str),
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

impl<K: Hash + Eq + Clone, V: Clone> SimHook for KeyedSingletonHook<K, V> {
    fn current_decision(&self) -> Option<bool> {
        self.to_release
            .as_ref()
            .map(|v| v.iter().any(|(_, _, is_new)| *is_new))
    }

    fn can_make_nontrivial_decision(&self) -> bool {
        !self.input.borrow().values().all(|q| q.is_empty())
    }

    fn autonomous_decision<'a>(
        &mut self,
        driver: &mut Borrowed<'a>,
        mut force_nontrivial: bool,
    ) -> bool {
        let mut current_input = self.input.borrow_mut();
        self.to_release = Some(vec![]);
        let nonempty_key_count = current_input.values().filter(|q| !q.is_empty()).count();

        let mut remaining_nonempty_keys = nonempty_key_count;
        let mut any_nontrivial = false;
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

    fn release_decision(&mut self, log_writer: &mut dyn std::fmt::Write) {
        if let Some(to_release) = self.to_release.take() {
            let (batch_location, line, caret_indent) = self.batch_location;
            let note_str = if to_release.is_empty() {
                "^ releasing no items".to_string()
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

            let _ = writeln!(
                log_writer,
                "{} {}",
                "-->".color(colored::Color::Blue),
                batch_location
            );

            let _ = writeln!(log_writer, " {}{}", "|".color(colored::Color::Blue), line);

            let _ = writeln!(
                log_writer,
                " {}{}{}",
                "|".color(colored::Color::Blue),
                caret_indent,
                note_str.color(colored::Color::Green)
            );

            for (key, value, _) in to_release {
                self.output.send((key, value)).unwrap();
            }
        } else {
            panic!("No decision to release");
        }
    }
}
