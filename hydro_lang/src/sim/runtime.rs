use std::cell::RefCell;
use std::collections::VecDeque;
use std::fmt::Debug;
use std::rc::Rc;

use bolero::ValueGenerator;
use bolero::generator::bolero_generator::driver::object::Borrowed;
use colored::Colorize;
use tokio::sync::mpsc::UnboundedSender;

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

struct TruncatedVecDebug<'a, T>(&'a Vec<T>, usize, fn(&T) -> Option<String>);
impl<'a, T> Debug for TruncatedVecDebug<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self(vec, max, elem_debug) = self;
        if vec.len() > *max {
            f.debug_list()
                .entries(
                    vec[..*max]
                        .iter()
                        .map(|v| elem_debug(v).unwrap_or("?".to_string())),
                )
                .finish_non_exhaustive()?;
            write!(f, " ({} total)", vec.len())
        } else {
            f.debug_list()
                .entries(
                    vec[..]
                        .iter()
                        .map(|v| elem_debug(v).unwrap_or("?".to_string())),
                )
                .finish()
        }
    }
}

pub struct StreamHook<T> {
    pub input: Rc<RefCell<VecDeque<T>>>,
    pub to_release: Option<Vec<T>>,
    pub output: UnboundedSender<T>,
    pub batch_location: (&'static str, &'static str, &'static str),
    pub format_item_debug: fn(&T) -> Option<String>,
}

impl<T> SimHook for StreamHook<T> {
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
