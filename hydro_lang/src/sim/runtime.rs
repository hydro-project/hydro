use std::cell::RefCell;
use std::collections::VecDeque;
use std::fmt::Debug;
use std::rc::Rc;

use bolero::generator::bolero_generator::driver::object::Borrowed;
use bolero::{ValueGenerator, produce};
use colored::Colorize;
use tokio::sync::mpsc::UnboundedSender;

use crate::live_collections::stream::{NoOrder, Ordering, TotalOrder};

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

struct TruncatedVecDebug<'a, T>(&'a Vec<T>, usize, fn(&T) -> Option<String>);
impl<'a, T> Debug for TruncatedVecDebug<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self(vec, max, elem_debug) = self;
        if vec.len() > *max {
            f.debug_list()
                .entries(vec[..*max].iter().map(|v| ManualDebug(v, *elem_debug)))
                .finish_non_exhaustive()?;
            write!(f, " ({} total)", vec.len())
        } else {
            f.debug_list()
                .entries(vec[..].iter().map(|v| ManualDebug(v, *elem_debug)))
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
                    TruncatedVecDebug(&to_release, 8, self.format_item_debug)
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
                    TruncatedVecDebug(&to_release, 8, self.format_item_debug)
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
        if let Some((to_release, _)) = self.to_release.take() {
            self.last_released = Some(to_release.clone());

            let (batch_location, line, caret_indent) = self.batch_location;
            let note_str = if self.skipped_states.is_empty() {
                format!(
                    "^ releasing snapshot: {:?}",
                    ManualDebug(&to_release, self.format_item_debug)
                )
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
