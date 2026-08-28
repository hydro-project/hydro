//! The hook objects that give the simulator scheduler control over unsafe
//! (nondeterministic) operators. Generated simulation code constructs one hook per
//! unsafe operator instance and registers it in the location-keyed maps returned to the
//! host ([`Hooks`], [`ObservationHooks`], [`InlineHooks`], and the scripted variants);
//! the scheduler in `sim::compiled` then drives every decision through the type-erased
//! traits defined here.
//!
//! This module holds the shared foundation: the location key ([`SimLocation`]), the hook
//! traits ([`RuntimeHook`] and its two refinements [`TickInputHook`] and
//! [`ObservationHook`], plus the in-tick [`InlineHook`]), and the error/log rendering
//! helpers. The hook kinds live in submodules, re-exported here so generated code can
//! refer to everything as `sim::runtime::X`:
//!
//! - `tick_input`: hooks that buffer input for a tick and decide what to release into
//!   each execution (batches, snapshots).
//! - `observation`: top-level hooks that are their own scheduling unit (e.g.
//!   `assume_ordering` or a hooked `fold` on a non-tick stream).
//! - `inline`: hooks resolved *during* a tick's execution (in-tick ordering).
//! - `scripted`: the script layer — decision traits, the [`Scripted`] /
//!   [`ScriptedInline`] shells that bind hooks to test-side handles, and the registry
//!   types.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Debug;

use bolero::generator::bolero_generator::driver::object::Borrowed;
use colored::Colorize;

use crate::location::dynamic::LocationId;

mod inline;
mod observation;
mod scripted;
mod tick_input;

pub use inline::*;
pub use observation::*;
pub use scripted::*;
pub use tick_input::*;

/// A location plus cluster member, keying the hook maps, DFIR lists, and script targets
/// that generated code hands to the host. Generated code constructs it by parsing an
/// inline JSON string ([`parse_location`]), so the serialized form never outlives the
/// dylib's initialization; everything downstream operates on the typed value.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SimLocation {
    pub location: LocationId,
    /// The cluster member, when the location is on a cluster.
    pub cluster_id: Option<u32>,
}

/// Parses a [`LocationId`](crate::location::dynamic::LocationId) from its JSON
/// serialization. Called by generated code with inline string literals.
#[doc(hidden)]
pub fn parse_location(serialized: &str) -> LocationId {
    serde_json::from_str(serialized).unwrap()
}

pub type Hooks = HashMap<SimLocation, Vec<Box<dyn TickInputHook>>>;
pub type ObservationHooks = HashMap<SimLocation, Vec<Box<dyn ObservationHook>>>;
pub type InlineHooks = HashMap<SimLocation, Vec<Box<dyn InlineHook>>>;

#[doc(hidden)]
#[macro_export]
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

/// The shared, type-erased surface of every hook — refined by [`TickInputHook`],
/// [`ObservationHook`], and [`InlineHook`]: querying and releasing decisions, and
/// attributing errors. Implemented once per collection kind ([`StreamHook`],
/// [`SingletonHook`], the keyed, top-level, and in-tick variants); each unsafe operator
/// instance gets one hook object of the matching kind. A hook bound to a test-side
/// handle is wrapped in [`Scripted<H>`] (or [`ScriptedInline<H>`]), which adds the
/// script state and implements this same trait so the scheduler needs no separate path.
///
/// The docs here state each method's *contract*; who consults it (scheduling candidacy,
/// the boundary scan, the in-tick resolve loop, deterministic mode) varies by hook kind.
pub trait RuntimeHook {
    /// Whether the hook holds buffered input it has not yet decided on.
    fn has_pending_input(&self) -> bool;

    /// Whether the current buffered state admits exactly one possible decision, i.e.
    /// resolving the hook right now would consume no fuzzer entropy. Examples of unique
    /// states: an empty batch buffer (the implicit empty batch), a single element at an
    /// ordering point (top-level or in-tick), a snapshot's forced first reveal, a merge
    /// with only one non-empty side, and always for [`PassthroughSingletonHook`]. Note
    /// that a batch hook with one buffered element still has two choices (release it or
    /// not), unlike an ordering hook, whose release of a sole element is forced.
    ///
    /// Deterministic mode permits a hook to resolve autonomously only when this holds
    /// (and panics, naming the operator, otherwise). The scheduling freedom of *when*
    /// the resolution happens relative to other candidates is not part of this
    /// question: deterministic mode guards it separately by requiring at most one
    /// runnable scheduler action.
    fn only_one_possible_decision(&self) -> bool;

    /// Release the decision that was made, logging to `log_writer`. A `None`
    /// writer means logging is disabled, allowing the hook to skip formatting
    /// entirely.
    fn release_decision(&mut self, log_writer: Option<&mut dyn std::fmt::Write>);

    /// Whether this hook is ready to participate in an execution. Returns false if the
    /// hook has never received any input and cannot produce a value (e.g. a
    /// singleton whose producing tick hasn't run yet).
    fn is_ready(&self) -> bool {
        true
    }

    /// The source location of the operator this hook simulates, used to attribute errors
    /// (e.g. an unhooked non-deterministic operator in deterministic mode).
    fn location_meta(&self) -> HookLocationMeta;
}

/// A hook that feeds a tick: it buffers input across scheduling boundaries and decides,
/// when its tick runs, what to release into that execution.
pub trait TickInputHook: RuntimeHook {
    /// Whether this hook can make a decision that would trigger its tick, making the
    /// tick runnable on this hook's account. Deliberately separate from
    /// [`RuntimeHook::only_one_possible_decision`]: [`PassthroughSingletonHook`] can
    /// trigger without facing any choice, and snapshot hooks are planned to face a
    /// choice (reveal or keep) without ever offering to trigger — revealing only when
    /// the tick runs for some other reason.
    fn can_trigger_tick(&self) -> bool;

    /// Make an autonomous decision from fuzzer entropy. When `force_trigger` is true,
    /// the decision must trigger the tick: the scheduler only runs a tick to do
    /// meaningful work, so when no other hook has triggered and this is the last
    /// undecided hook, it is forced to make the run count. Returns whether the decision
    /// triggers the tick.
    fn autonomous_decision<'a>(&mut self, driver: &mut Borrowed<'a>, force_trigger: bool) -> bool;
}

/// A top-level observation hook (e.g. from `assume_ordering` or a hooked `fold` on a
/// non-tick stream): it has no tick DFIR and is its own scheduling unit, so running it
/// *is* releasing data. There is consequently no `force` parameter here: an observation
/// is only ever scheduled when it can release, and its autonomous decision must release.
pub trait ObservationHook: RuntimeHook {
    /// Make an autonomous decision from fuzzer entropy. Must stage a releasing decision:
    /// the scheduler only runs an observation that reported pending input
    /// ([`RuntimeHook::has_pending_input`]).
    fn autonomous_decision<'a>(&mut self, driver: &mut Borrowed<'a>);
}

/// Renders the error for an unsafe operator that needs a non-deterministic decision in
/// deterministic mode.
pub(crate) fn render_unhooked_nondet_error(location: HookLocationMeta) -> String {
    let HookLocationMeta {
        location: loc,
        line,
        caret_indent: caret,
    } = location;
    format!(
        "deterministic simulation encountered an unsafe operator with pending input that is not bound to a sim hook:\n--> {loc}\n |{line}\n |{caret}^ this operator must make a non-deterministic decision\nhelp: bind a sim hook to this operator (`nondet!(... hook = handle)`) and script its decisions, or run under `fuzz` / `exhaustive` instead"
    )
}

/// A hook resolved *while* a tick DFIR is executing (primarily `ObserveNonDet` IR nodes,
/// e.g. in-tick `assume_ordering`), for operators that block mid-tick on a decision.
/// Unlike the other kinds, its input exists only during one tick execution: nothing
/// buffers across scheduling boundaries, and the tick cannot proceed until the decision
/// is released.
pub trait InlineHook: RuntimeHook {
    /// Make an autonomous decision from fuzzer entropy, staging a release of the
    /// *entire* pending input: the tick is blocked on it, so nothing may be withheld —
    /// the decision space is only the arrangement of what is buffered.
    fn autonomous_decision<'a>(&mut self, driver: &mut Borrowed<'a>);
}

pub(crate) struct ManualDebug<'a, T>(pub(crate) &'a T, pub(crate) fn(&T) -> Option<String>);
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

pub(crate) struct LabeledDebug<'a, T>(
    pub(crate) &'static str,
    pub(crate) &'a T,
    pub(crate) fn(&T) -> Option<String>,
);
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

pub(crate) struct TruncatedVecDebug<'a, T: 'a, I: Iterator<Item = &'a T>>(
    pub(crate) RefCell<Option<I>>,
    pub(crate) usize,
    pub(crate) fn(&T) -> Option<String>,
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

pub(crate) struct TruncatedLabeledVecDebug<'a, T: 'a, I: Iterator<Item = (&'static str, &'a T)>>(
    pub(crate) RefCell<Option<I>>,
    pub(crate) usize,
    pub(crate) fn(&T) -> Option<String>,
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

/// The source attribution of a hooked operator, used to render release logs and hook
/// error messages. Constructed by generated code from the operator's `nondet!` guard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HookLocationMeta {
    /// The `file:line:col` of the operator.
    pub location: &'static str,
    /// The text of the operator's source line.
    pub line: &'static str,
    /// The whitespace that positions a `^` marker under the operator within `line`.
    pub caret_indent: &'static str,
}

/// Aborts the process with an internal-error message. Used instead of `panic!` for
/// invariant violations inside code that may be monomorphized into the generated dylib:
/// a panic must not unwind across the dylib boundary (Rust aborts on foreign exceptions,
/// with a far less useful message).
macro_rules! abort {
    ($($arg:tt)*) => {{
        eprintln!("Simulator internal error: {}", format_args!($($arg)*));
        std::process::abort();
    }};
}
pub(crate) use abort;

/// Writes the standard release-log source block: the `--> location` header, the source
/// line, and a caret note (colored `note_color`) under the operator.
pub(crate) fn log_release(
    log_writer: &mut dyn std::fmt::Write,
    location: &str,
    line: &str,
    caret_indent: &str,
    note: &str,
    note_color: colored::Color,
) {
    writeln!(
        log_writer,
        "{} {}",
        "-->".color(colored::Color::Blue),
        location
    )
    .unwrap();

    writeln!(log_writer, " {}{}", "|".color(colored::Color::Blue), line).unwrap();

    writeln!(
        log_writer,
        " {}{}{}",
        "|".color(colored::Color::Blue),
        caret_indent,
        note.color(note_color)
    )
    .unwrap();
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
