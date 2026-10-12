//! A deterministic "prompt" schedule for the simulator: every nondeterminism point takes the
//! choice that moves the most data forward, and ready ticks are served round-robin.
//!
//! The simulator's hooks draw their decisions from a bolero [`Driver`]. Under fuzzing or
//! exhaustive search that driver explores the decision space; this one answers every question
//! with a fixed policy, which turns a simulation into a deterministic discrete-time execution.
//! That is what an experiment needs when it wants to vary the *program* or its *inputs* while
//! holding the schedule fixed.
//!
//! The policy is expressed in terms of how the hooks in [`super::runtime`] phrase their
//! questions:
//!
//! - An integer range with an *inclusive* upper bound is a "how many items to release" question
//!   (e.g. `(0..=len)`), answered with the maximum: release everything that is buffered.
//! - An integer range with an *exclusive* upper bound is a "which one" question (which ready
//!   tick to run, which buffered item to release next, which queued snapshot to observe),
//!   answered with the minimum: the first ready tick (the scheduler pushes a tick it has run to
//!   the back of the ready list, so this is round-robin) and the oldest item (FIFO).
//! - A boolean is "do something unusual" (stop releasing early, re-release a stale snapshot,
//!   crash a member), answered with `false`.
//!
//! The driver never runs out of entropy, so a run ends only when the test's future completes.
//!
//! # Why this exists, and when to delete it
//!
//! This driver stands in for the simulator's [`deterministic`] mode for now. `deterministic()`
//! refuses any hook with more than one legal decision, and requires at most one runnable tick at
//! a time, so a caller that scripts the hooks it cares about and wants a fixed default for the
//! rest cannot use it: the scripting facility cannot yet bind snapshots of `Optional`
//! singletons, snapshots of folds over unordered (`NoOrder`) input, or the ordering and retry
//! hooks on top-level streams, and a run of a multi-location program routinely has several ticks
//! ready at once. Until those hook kinds are scriptable, or `deterministic()` accepts a default
//! policy for unscripted hooks and for tick order, this driver supplies that default. Once either
//! holds, this module and [`CompiledSim::run_with_driver`] should be deleted and their callers
//! moved to `deterministic()`.
//!
//! [`deterministic`]: super::compiled::CompiledSim::deterministic
//! [`CompiledSim::run_with_driver`]: super::compiled::CompiledSim::run_with_driver

use core::ops::Bound;

use bolero::bolero_engine::driver::Driver;

/// See the [module documentation](self).
#[derive(Debug, Default)]
pub struct PromptScheduleDriver {
    depth: usize,
}

macro_rules! prompt_int {
    ($name:ident, $ty:ty) => {
        #[inline]
        fn $name(&mut self, min: Bound<&$ty>, max: Bound<&$ty>) -> Option<$ty> {
            Some(match max {
                Bound::Included(max) => *max,
                Bound::Excluded(_) | Bound::Unbounded => match min {
                    Bound::Included(min) => *min,
                    Bound::Excluded(min) => min.checked_add(1 as $ty)?,
                    Bound::Unbounded => 0 as $ty,
                },
            })
        }
    };
}

impl Driver for PromptScheduleDriver {
    fn depth(&self) -> usize {
        self.depth
    }

    fn set_depth(&mut self, depth: usize) {
        self.depth = depth;
    }

    fn max_depth(&self) -> usize {
        usize::MAX
    }

    fn gen_variant(&mut self, _variants: usize, base_case: usize) -> Option<usize> {
        Some(base_case)
    }

    prompt_int!(gen_u8, u8);
    prompt_int!(gen_i8, i8);
    prompt_int!(gen_u16, u16);
    prompt_int!(gen_i16, i16);
    prompt_int!(gen_u32, u32);
    prompt_int!(gen_i32, i32);
    prompt_int!(gen_u64, u64);
    prompt_int!(gen_i64, i64);
    prompt_int!(gen_u128, u128);
    prompt_int!(gen_i128, i128);
    prompt_int!(gen_usize, usize);
    prompt_int!(gen_isize, isize);

    fn gen_f32(&mut self, min: Bound<&f32>, _max: Bound<&f32>) -> Option<f32> {
        Some(match min {
            Bound::Included(m) | Bound::Excluded(m) => *m,
            Bound::Unbounded => 0.0,
        })
    }

    fn gen_f64(&mut self, min: Bound<&f64>, _max: Bound<&f64>) -> Option<f64> {
        Some(match min {
            Bound::Included(m) | Bound::Excluded(m) => *m,
            Bound::Unbounded => 0.0,
        })
    }

    fn gen_char(&mut self, min: Bound<&char>, _max: Bound<&char>) -> Option<char> {
        Some(match min {
            Bound::Included(m) | Bound::Excluded(m) => *m,
            Bound::Unbounded => '\0',
        })
    }

    fn gen_bool(&mut self, _probability: Option<f32>) -> Option<bool> {
        Some(false)
    }

    fn gen_from_bytes<Hint, Gen, T>(&mut self, hint: Hint, mut produce: Gen) -> Option<T>
    where
        Hint: FnOnce() -> (usize, Option<usize>),
        Gen: FnMut(&[u8]) -> Option<(usize, T)>,
    {
        let (min_len, max_len) = hint();
        let zeros = vec![0u8; max_len.unwrap_or(min_len).max(min_len)];
        produce(&zeros).map(|(_, value)| value)
    }
}
