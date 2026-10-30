//! The hook for `observe_drops(nondet!, nonlive!)` on an ordered
//! [`MayDrop`](crate::live_collections::stream::MayDrop) stream: the point where the simulator
//! decides which elements are missing. Each element is either kept or dropped, so unlike
//! duplication (where every element admits arbitrarily many retries) the decision space is
//! finite and is explored autonomously: exhaustive simulation enumerates every subsequence of the
//! elements that reach the observation.
//!
//! Drops never reorder: kept elements are released in the order they were buffered. A
//! `MayDrop` stream never enters a tick (`batch` requires `NoDrop`), so there is no in-tick
//! variant, and the unordered `observe_drops(nonlive!)` needs no hook (see `SimFlow::test_safety_only`).

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use bolero::generator::bolero_generator::driver::object::Borrowed;
use bolero::{ValueGenerator, produce};
use dfir_rs::util::unsync::mpsc::Sender;

use super::{HookLocationMeta, ManualDebug, ObservationHook, RuntimeHook, log_release};

/// Top-level (outside-tick) `observe_drops` on an ordered `MayDrop` stream. Like the top-level
/// ordering hook, it decides **one element at a time** (the front of its buffer), so that
/// elements produced later (for example through a feedback cycle) can interleave with the
/// decisions; each decision keeps or drops that element.
pub struct TopLevelStreamDropsHook<T> {
    pub input: Rc<RefCell<VecDeque<T>>>,
    /// The decided element, and whether it is kept.
    pub to_release: Option<(T, bool)>,
    pub output: Sender<T>,
    pub location: HookLocationMeta,
    pub format_item_debug: fn(&T) -> Option<String>,
}

impl<T> RuntimeHook for TopLevelStreamDropsHook<T> {
    fn has_pending_input(&self) -> bool {
        !self.input.borrow().is_empty()
    }

    fn only_one_possible_decision(&self) -> bool {
        // Every buffered element may be kept or dropped.
        self.input.borrow().is_empty()
    }

    fn release_decision(&mut self, log_writer: Option<&mut dyn std::fmt::Write>) {
        let Some((item, keep)) = self.to_release.take() else {
            panic!("No decision to release");
        };
        if let Some(log_writer) = log_writer {
            let HookLocationMeta {
                location,
                line,
                caret_indent,
            } = self.location;
            let note_str = format!(
                "^ {} {:?}",
                if keep { "kept" } else { "dropped" },
                ManualDebug(&item, self.format_item_debug)
            );
            let _ = writeln!(log_writer);
            log_release(
                log_writer,
                location,
                line,
                caret_indent,
                &note_str,
                if keep {
                    colored::Color::Green
                } else {
                    colored::Color::Red
                },
            );
        }
        if keep {
            self.output.try_send(item).unwrap();
        }
    }

    fn location_meta(&self) -> HookLocationMeta {
        self.location
    }
}

impl<T> ObservationHook for TopLevelStreamDropsHook<T> {
    fn autonomous_decision<'a>(&mut self, driver: &mut Borrowed<'a>) {
        let item = self.input.borrow_mut().pop_front().unwrap();
        let keep: bool = produce().generate(driver).unwrap();
        self.to_release = Some((item, keep));
    }
}
