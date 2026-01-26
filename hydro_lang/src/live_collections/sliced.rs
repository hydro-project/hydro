//! Utilities for transforming live collections via slicing.

use super::boundedness::{Bounded, Unbounded};
use crate::live_collections::boundedness::Boundedness;
use crate::live_collections::keyed_singleton::BoundedValue;
use crate::live_collections::stream::{Ordering, Retries};
use crate::location::{Location, NoTick, Tick};
use crate::nondet::NonDet;

#[doc(hidden)]
pub fn __sliced_wrap_invoke<A, B, O: Unslicable>(
    a: A,
    b: B,
    f: impl FnOnce(A, B) -> O,
) -> O::Unsliced {
    let o_slice = f(a, b);
    o_slice.unslice()
}

#[doc(hidden)]
#[macro_export]
macro_rules! __sliced_parse_uses__ {
    // Parse immutable use statements: let name = use(expr, nondet);
    (
        @uses [$($uses:tt)*]
        @states [$($states:tt)*]
        let $name:ident = use $(::$style:ident)?($expr:expr, $nondet:expr); $($rest:tt)*
    ) => {
        $crate::__sliced_parse_uses__!(
            @uses [$($uses)* { $name, ($($style)?), $expr, $nondet }]
            @states [$($states)*]
            $($rest)*
        )
    };

    // Parse mutable state statements: let mut name = use::style::<Type>(args);
    (
        @uses [$($uses:tt)*]
        @states [$($states:tt)*]
        let mut $name:ident = use ::$style:ident $(::<$ty:ty>)? ($($args:expr)?); $($rest:tt)*
    ) => {
        $crate::__sliced_parse_uses__!(
            @uses [$($uses)*]
            @states [$($states)* { $name, $style, (($($ty)?), ($($args)?)) }]
            $($rest)*
        )
    };

    // Terminal case: no uses, only states
    (
        @uses []
        @states [$({ $state_name:ident, $state_style:ident, $state_arg:tt })+]
        $($body:tt)*
    ) => {
        {
            // We need at least one use to get a tick, so panic if there are none
            compile_error!("sliced! requires at least one `let name = use(...)` statement to determine the tick")
        }
    };

    // Terminal case: uses with optional states
    (
        @uses [{ $first_name:ident, ($($first_style:ident)?), $first:expr, $nondet_first:expr } $({ $rest_name:ident, ($($rest_style:ident)?), $rest:expr, $nondet_expl:expr })*]
        @states [$({ $state_name:ident, $state_style:ident, (($($state_ty:ty)?), ($($state_arg:expr)?)) })*]
        $($body:tt)*
    ) => {
        {
            let _ = $nondet_first;
            $(let _ = $nondet_expl;)*

            let __styled = (
                $($crate::live_collections::sliced::style::$first_style)?($first),
                $($($crate::live_collections::sliced::style::$rest_style)?($rest),)*
            );

            let __tick = $crate::live_collections::sliced::Slicable::preferred_tick(&__styled).unwrap_or_else(|| $crate::live_collections::sliced::Slicable::get_location(&__styled.0).tick());
            let __backtraces = {
                use $crate::compile::ir::backtrace::__macro_get_backtrace;
                (
                    $crate::macro_support::copy_span::copy_span!($first, {
                        __macro_get_backtrace(1)
                    }),
                    $($crate::macro_support::copy_span::copy_span!($rest, {
                        __macro_get_backtrace(1)
                    }),)*
                )
            };
            let __sliced = $crate::live_collections::sliced::Slicable::slice(__styled, &__tick, __backtraces, $nondet_first);
            let (
                $first_name,
                $($rest_name,)*
            ) = __sliced;

            // Create all cycles and pack handles/values into tuples
            let (__handles, __states) = $crate::live_collections::sliced::unzip_cycles((
                $($crate::live_collections::sliced::style::$state_style$(::<$state_ty, _>)?(& __tick, $($state_arg)?),)*
            ));

            // Unpack mutable state values
            let (
                $(mut $state_name,)*
            ) = __states;

            // Execute the body
            let __body_result = {
                $($body)*
            };

            // Re-pack the final state values and complete cycles
            let __final_states = (
                $($state_name,)*
            );
            $crate::live_collections::sliced::complete_cycles(__handles, __final_states);

            // Unslice the result
            $crate::live_collections::sliced::Unslicable::unslice(__body_result)
        }
    };
}

#[macro_export]
/// Transforms a live collection with a computation relying on a slice of another live collection.
/// This is useful for reading a snapshot of an asynchronously updated collection while processing another
/// collection, such as joining a stream with the latest values from a singleton.
///
/// # Syntax
/// The `sliced!` macro takes in a closure-like syntax specifying the live collections to be sliced
/// and the body of the transformation. Each `use` statement indicates a live collection to be sliced,
/// along with a non-determinism explanation. Optionally, a style can be specified to control how the
/// live collection is sliced (e.g., atomically). All `use` statements must appear before the body.
///
/// ```rust,ignore
/// let stream = sliced! {
///     let name1 = use(collection1, nondet!(/** explanation */));
///     let name2 = use::atomic(collection2, nondet!(/** explanation */));
///
///     // arbitrary statements can follow
///     let intermediate = name1.map(...);
///     intermediate.cross_singleton(name2)
/// };
/// ```
///
/// # Stateful Computations
/// The `sliced!` macro also supports stateful computations across iterations using `let mut` bindings
/// with `use::state` or `use::state_null`. These create cycles that persist values between iterations.
///
/// - `use::state(|l| initial)`: Creates a cycle with an initial value. The closure receives
///   the slice location and returns the initial state for the first iteration.
/// - `use::state_null::<Type>()`: Creates a cycle that starts as null/empty on the first iteration.
///
/// The mutable binding can be reassigned in the body, and the final value will be passed to the
/// next iteration.
///
/// ```rust,ignore
/// let counter_stream = sliced! {
///     let batch = use(input_stream, nondet!(/** explanation */));
///     let mut counter = use::state(|l| l.singleton(q!(0)));
///
///     // Increment counter by the number of items in this batch
///     let new_count = counter.clone().zip(batch.count())
///         .map(q!(|(old, add)| old + add));
///     counter = new_count.clone();
///     new_count.into_stream()
/// };
/// ```
macro_rules! __sliced__ {
    ($($tt:tt)*) => {
        $crate::__sliced_parse_uses__!(
            @uses []
            @states []
            $($tt)*
        )
    };
}

pub use crate::__sliced__ as sliced;

/// Marks this live collection as atomically-yielded, which means that the output outside
/// `sliced` will be at an atomic location that is synchronous with respect to the body
/// of the slice.
pub fn yield_atomic<T>(t: T) -> style::Atomic<T> {
    style::Atomic(t)
}

/// Styles for use with the `sliced!` macro.
pub mod style {
    use super::Slicable;
    #[cfg(stageleft_runtime)]
    use crate::forward_handle::{CycleCollection, CycleCollectionWithInitial};
    use crate::forward_handle::{TickCycle, TickCycleHandle};
    use crate::live_collections::boundedness::{Bounded, Unbounded};
    use crate::live_collections::keyed_singleton::BoundedValue;
    use crate::live_collections::sliced::Unslicable;
    use crate::live_collections::stream::{Ordering, Retries, Stream};
    use crate::location::tick::DeferTick;
    use crate::location::{Location, NoTick, Tick};
    use crate::nondet::NonDet;

    /// Marks a live collection to be treated atomically during slicing.
    pub struct Atomic<T>(pub T);

    /// Wraps a live collection to be treated atomically during slicing.
    pub fn atomic<T>(t: T) -> Atomic<T> {
        Atomic(t)
    }

    /// Creates a stateful cycle with an initial value for use in `sliced!`.
    ///
    /// The initial value is computed from a closure that receives the location
    /// for the body of the slice.
    ///
    /// The initial value is used on the first iteration, and subsequent iterations receive
    /// the value assigned to the mutable binding at the end of the previous iteration.
    #[cfg(stageleft_runtime)]
    #[expect(
        private_bounds,
        reason = "only Hydro collections can implement CycleCollectionWithInitial"
    )]
    pub fn state<
        'a,
        S: CycleCollectionWithInitial<'a, TickCycle, Location = Tick<L>>,
        L: Location<'a> + NoTick,
    >(
        tick: &Tick<L>,
        initial_fn: impl FnOnce(&Tick<L>) -> S,
    ) -> (TickCycleHandle<'a, S>, S) {
        let initial = initial_fn(tick);
        tick.cycle_with_initial(initial)
    }

    /// Creates a stateful cycle without an initial value for use in `sliced!`.
    ///
    /// On the first iteration, the state will be null/empty. Subsequent iterations receive
    /// the value assigned to the mutable binding at the end of the previous iteration.
    #[cfg(stageleft_runtime)]
    #[expect(
        private_bounds,
        reason = "only Hydro collections can implement CycleCollection"
    )]
    pub fn state_null<
        'a,
        S: CycleCollection<'a, TickCycle, Location = Tick<L>> + DeferTick,
        L: Location<'a> + NoTick,
    >(
        tick: &Tick<L>,
    ) -> (TickCycleHandle<'a, S>, S) {
        tick.cycle::<S>()
    }

    impl<'a, T, L: Location<'a> + NoTick, O: Ordering, R: Retries> Slicable<'a, L>
        for Atomic<Stream<T, crate::location::Atomic<L>, Unbounded, O, R>>
    {
        type Slice = Stream<T, Tick<L>, Bounded, O, R>;
        type Backtrace = crate::compile::ir::backtrace::Backtrace;

        fn preferred_tick(&self) -> Option<Tick<L>> {
            Some(self.0.location().tick.clone())
        }

        fn get_location(&self) -> &L {
            panic!("Atomic location has no accessible inner location")
        }

        fn slice(self, tick: &Tick<L>, backtrace: Self::Backtrace, nondet: NonDet) -> Self::Slice {
            assert_eq!(
                self.0.location().tick.id(),
                tick.id(),
                "Mismatched tick for atomic slicing"
            );

            let out = self.0.batch_atomic(nondet);
            out.ir_node.borrow_mut().op_metadata_mut().backtrace = backtrace;
            out
        }
    }

    impl<'a, T, L: Location<'a> + NoTick, O: Ordering, R: Retries> Unslicable
        for Atomic<Stream<T, Tick<L>, Bounded, O, R>>
    {
        type Unsliced = Stream<T, crate::location::Atomic<L>, Unbounded, O, R>;

        fn unslice(self) -> Self::Unsliced {
            self.0.all_ticks_atomic()
        }
    }

    impl<'a, T, L: Location<'a> + NoTick> Slicable<'a, L>
        for Atomic<crate::live_collections::Singleton<T, crate::location::Atomic<L>, Unbounded>>
    {
        type Slice = crate::live_collections::Singleton<T, Tick<L>, Bounded>;
        type Backtrace = crate::compile::ir::backtrace::Backtrace;

        fn preferred_tick(&self) -> Option<Tick<L>> {
            Some(self.0.location().tick.clone())
        }

        fn get_location(&self) -> &L {
            panic!("Atomic location has no accessible inner location")
        }

        fn slice(self, tick: &Tick<L>, backtrace: Self::Backtrace, nondet: NonDet) -> Self::Slice {
            assert_eq!(
                self.0.location().tick.id(),
                tick.id(),
                "Mismatched tick for atomic slicing"
            );

            let out = self.0.snapshot_atomic(nondet);
            out.ir_node.borrow_mut().op_metadata_mut().backtrace = backtrace;
            out
        }
    }

    impl<'a, T, L: Location<'a> + NoTick> Unslicable
        for Atomic<crate::live_collections::Singleton<T, Tick<L>, Bounded>>
    {
        type Unsliced =
            crate::live_collections::Singleton<T, crate::location::Atomic<L>, Unbounded>;

        fn unslice(self) -> Self::Unsliced {
            self.0.latest_atomic()
        }
    }

    impl<'a, T, L: Location<'a> + NoTick> Slicable<'a, L>
        for Atomic<crate::live_collections::Optional<T, crate::location::Atomic<L>, Unbounded>>
    {
        type Slice = crate::live_collections::Optional<T, Tick<L>, Bounded>;
        type Backtrace = crate::compile::ir::backtrace::Backtrace;

        fn preferred_tick(&self) -> Option<Tick<L>> {
            Some(self.0.location().tick.clone())
        }

        fn get_location(&self) -> &L {
            panic!("Atomic location has no accessible inner location")
        }

        fn slice(self, tick: &Tick<L>, backtrace: Self::Backtrace, nondet: NonDet) -> Self::Slice {
            assert_eq!(
                self.0.location().tick.id(),
                tick.id(),
                "Mismatched tick for atomic slicing"
            );

            let out = self.0.snapshot_atomic(nondet);
            out.ir_node.borrow_mut().op_metadata_mut().backtrace = backtrace;
            out
        }
    }

    impl<'a, T, L: Location<'a> + NoTick> Unslicable
        for Atomic<crate::live_collections::Optional<T, Tick<L>, Bounded>>
    {
        type Unsliced = crate::live_collections::Optional<T, crate::location::Atomic<L>, Unbounded>;

        fn unslice(self) -> Self::Unsliced {
            self.0.latest_atomic()
        }
    }

    impl<'a, K, V, L: Location<'a> + NoTick, O: Ordering, R: Retries> Slicable<'a, L>
        for Atomic<
            crate::live_collections::KeyedStream<K, V, crate::location::Atomic<L>, Unbounded, O, R>,
        >
    {
        type Slice = crate::live_collections::KeyedStream<K, V, Tick<L>, Bounded, O, R>;
        type Backtrace = crate::compile::ir::backtrace::Backtrace;

        fn preferred_tick(&self) -> Option<Tick<L>> {
            Some(self.0.location().tick.clone())
        }

        fn get_location(&self) -> &L {
            panic!("Atomic location has no accessible inner location")
        }

        fn slice(self, tick: &Tick<L>, backtrace: Self::Backtrace, nondet: NonDet) -> Self::Slice {
            assert_eq!(
                self.0.location().tick.id(),
                tick.id(),
                "Mismatched tick for atomic slicing"
            );

            let out = self.0.batch_atomic(nondet);
            out.ir_node.borrow_mut().op_metadata_mut().backtrace = backtrace;
            out
        }
    }

    impl<'a, K, V, L: Location<'a> + NoTick, O: Ordering, R: Retries> Unslicable
        for Atomic<crate::live_collections::KeyedStream<K, V, Tick<L>, Bounded, O, R>>
    {
        type Unsliced =
            crate::live_collections::KeyedStream<K, V, crate::location::Atomic<L>, Unbounded, O, R>;

        fn unslice(self) -> Self::Unsliced {
            self.0.all_ticks_atomic()
        }
    }

    impl<'a, K, V, L: Location<'a> + NoTick> Slicable<'a, L>
        for Atomic<
            crate::live_collections::KeyedSingleton<K, V, crate::location::Atomic<L>, Unbounded>,
        >
    {
        type Slice = crate::live_collections::KeyedSingleton<K, V, Tick<L>, Bounded>;
        type Backtrace = crate::compile::ir::backtrace::Backtrace;

        fn preferred_tick(&self) -> Option<Tick<L>> {
            Some(self.0.location().tick.clone())
        }

        fn get_location(&self) -> &L {
            panic!("Atomic location has no accessible inner location")
        }

        fn slice(self, tick: &Tick<L>, backtrace: Self::Backtrace, nondet: NonDet) -> Self::Slice {
            assert_eq!(
                self.0.location().tick.id(),
                tick.id(),
                "Mismatched tick for atomic slicing"
            );

            let out = self.0.snapshot_atomic(nondet);
            out.ir_node.borrow_mut().op_metadata_mut().backtrace = backtrace;
            out
        }
    }

    impl<'a, K, V, L: Location<'a> + NoTick> Slicable<'a, L>
        for Atomic<
            crate::live_collections::KeyedSingleton<K, V, crate::location::Atomic<L>, BoundedValue>,
        >
    {
        type Slice = crate::live_collections::KeyedSingleton<K, V, Tick<L>, Bounded>;
        type Backtrace = crate::compile::ir::backtrace::Backtrace;

        fn preferred_tick(&self) -> Option<Tick<L>> {
            Some(self.0.location().tick.clone())
        }

        fn get_location(&self) -> &L {
            panic!("Atomic location has no accessible inner location")
        }

        fn slice(self, tick: &Tick<L>, backtrace: Self::Backtrace, nondet: NonDet) -> Self::Slice {
            assert_eq!(
                self.0.location().tick.id(),
                tick.id(),
                "Mismatched tick for atomic slicing"
            );

            let out = self.0.batch_atomic(nondet);
            out.ir_node.borrow_mut().op_metadata_mut().backtrace = backtrace;
            out
        }
    }
}

/// A trait for live collections which can be sliced into bounded versions at a tick.
pub trait Slicable<'a, L: Location<'a>> {
    /// The sliced version of this live collection.
    type Slice;

    /// The type of backtrace associated with this slice.
    type Backtrace;

    /// Gets the preferred tick to slice at. Used for atomic slicing.
    fn preferred_tick(&self) -> Option<Tick<L>>;

    /// Gets the location associated with this live collection.
    fn get_location(&self) -> &L;

    /// Slices this live collection at the given tick.
    ///
    /// # Non-Determinism
    /// Slicing a live collection may involve non-determinism, such as choosing which messages
    /// to include in a batch. The provided `nondet` parameter should be used to explain the impact
    /// of this non-determinism on the program's behavior.
    fn slice(self, tick: &Tick<L>, backtrace: Self::Backtrace, nondet: NonDet) -> Self::Slice;
}

/// A trait for live collections which can be yielded out of a slice back into their original form.
pub trait Unslicable {
    /// The unsliced version of this live collection.
    type Unsliced;

    /// Unslices a sliced live collection back into its original form.
    fn unslice(self) -> Self::Unsliced;
}

/// A trait for unzipping a tuple of (handle, state) pairs into separate tuples.
#[doc(hidden)]
pub trait UnzipCycles {
    /// The tuple of cycle handles.
    type Handles;
    /// The tuple of state values.
    type States;

    /// Unzips the cycles into handles and states.
    fn unzip(self) -> (Self::Handles, Self::States);
}

/// Unzips a tuple of cycles into handles and states.
#[doc(hidden)]
pub fn unzip_cycles<T: UnzipCycles>(cycles: T) -> (T::Handles, T::States) {
    cycles.unzip()
}

/// A trait for completing a tuple of cycle handles with their final state values.
#[doc(hidden)]
pub trait CompleteCycles<States> {
    /// Completes all cycles with the provided state values.
    fn complete(self, states: States);
}

/// Completes a tuple of cycle handles with their final state values.
#[doc(hidden)]
pub fn complete_cycles<H: CompleteCycles<S>, S>(handles: H, states: S) {
    handles.complete(states);
}

impl<'a, L: Location<'a>> Slicable<'a, L> for () {
    type Slice = ();
    type Backtrace = ();

    fn get_location(&self) -> &L {
        unreachable!()
    }

    fn preferred_tick(&self) -> Option<Tick<L>> {
        None
    }

    fn slice(self, _tick: &Tick<L>, __backtrace: Self::Backtrace, _nondet: NonDet) -> Self::Slice {}
}

impl Unslicable for () {
    type Unsliced = ();

    fn unslice(self) -> Self::Unsliced {}
}

macro_rules! impl_slicable_for_tuple {
    ($($T:ident, $T_bt:ident, $idx:tt),+) => {
        impl<'a, L: Location<'a>, $($T: Slicable<'a, L>),+> Slicable<'a, L> for ($($T,)+) {
            type Slice = ($($T::Slice,)+);
            type Backtrace = ($($T::Backtrace,)+);

            fn get_location(&self) -> &L {
                self.0.get_location()
            }

            fn preferred_tick(&self) -> Option<Tick<L>> {
                let mut preferred: Option<Tick<L>> = None;
                $(
                    if let Some(tick) = self.$idx.preferred_tick() {
                        preferred = Some(match preferred {
                            Some(current) => {
                                if $crate::location::Location::id(&current) == $crate::location::Location::id(&tick) {
                                    current
                                } else {
                                    panic!("Mismatched preferred ticks for sliced collections")
                                }
                            },
                            None => tick,
                        });
                    }
                )+
                preferred
            }

            #[expect(non_snake_case, reason = "macro codegen")]
            fn slice(self, tick: &Tick<L>, backtrace: Self::Backtrace, nondet: NonDet) -> Self::Slice {
                let ($($T,)+) = self;
                let ($($T_bt,)+) = backtrace;
                ($($T.slice(tick, $T_bt, nondet),)+)
            }
        }

        impl<$($T: Unslicable),+> Unslicable for ($($T,)+) {
            type Unsliced = ($($T::Unsliced,)+);

            #[expect(non_snake_case, reason = "macro codegen")]
            fn unslice(self) -> Self::Unsliced {
                let ($($T,)+) = self;
                ($($T.unslice(),)+)
            }
        }
    };
}

#[cfg(stageleft_runtime)]
impl_slicable_for_tuple!(S1, S1_bt, 0);
#[cfg(stageleft_runtime)]
impl_slicable_for_tuple!(S1, S1_bt, 0, S2, S2_bt, 1);
#[cfg(stageleft_runtime)]
impl_slicable_for_tuple!(S1, S1_bt, 0, S2, S2_bt, 1, S3, S3_bt, 2);
#[cfg(stageleft_runtime)]
impl_slicable_for_tuple!(S1, S1_bt, 0, S2, S2_bt, 1, S3, S3_bt, 2, S4, S4_bt, 3);
#[cfg(stageleft_runtime)]
impl_slicable_for_tuple!(
    S1, S1_bt, 0, S2, S2_bt, 1, S3, S3_bt, 2, S4, S4_bt, 3, S5, S5_bt, 4
); // 5 slices ought to be enough for anyone

macro_rules! impl_cycles_for_tuple {
    ($($H:ident, $S:ident, $idx:tt),*) => {
        impl<$($H, $S),*> UnzipCycles for ($(($H, $S),)*) {
            type Handles = ($($H,)*);
            type States = ($($S,)*);

            #[expect(clippy::allow_attributes, reason = "macro codegen")]
            #[allow(non_snake_case, reason = "macro codegen")]
            fn unzip(self) -> (Self::Handles, Self::States) {
                let ($($H,)*) = self;
                (
                    ($($H.0,)*),
                    ($($H.1,)*),
                )
            }
        }

        impl<$($H: crate::forward_handle::CompleteCycle<$S>, $S),*> CompleteCycles<($($S,)*)> for ($($H,)*) {
            #[expect(clippy::allow_attributes, reason = "macro codegen")]
            #[allow(non_snake_case, reason = "macro codegen")]
            fn complete(self, states: ($($S,)*)) {
                let ($($H,)*) = self;
                let ($($S,)*) = states;
                $($H.complete_next_tick($S);)*
            }
        }
    };
}

#[cfg(stageleft_runtime)]
impl_cycles_for_tuple!();
#[cfg(stageleft_runtime)]
impl_cycles_for_tuple!(H1, S1, 0);
#[cfg(stageleft_runtime)]
impl_cycles_for_tuple!(H1, S1, 0, H2, S2, 1);
#[cfg(stageleft_runtime)]
impl_cycles_for_tuple!(H1, S1, 0, H2, S2, 1, H3, S3, 2);
#[cfg(stageleft_runtime)]
impl_cycles_for_tuple!(H1, S1, 0, H2, S2, 1, H3, S3, 2, H4, S4, 3);
#[cfg(stageleft_runtime)]
impl_cycles_for_tuple!(H1, S1, 0, H2, S2, 1, H3, S3, 2, H4, S4, 3, H5, S5, 4);

impl<'a, T, L: Location<'a>, B: Boundedness, O: Ordering, R: Retries> Slicable<'a, L>
    for super::Stream<T, L, B, O, R>
{
    type Slice = super::Stream<T, Tick<L>, Bounded, O, R>;
    type Backtrace = crate::compile::ir::backtrace::Backtrace;

    fn get_location(&self) -> &L {
        self.location()
    }

    fn preferred_tick(&self) -> Option<Tick<L>> {
        None
    }

    fn slice(self, tick: &Tick<L>, backtrace: Self::Backtrace, nondet: NonDet) -> Self::Slice {
        let out = self.batch(tick, nondet);
        out.ir_node.borrow_mut().op_metadata_mut().backtrace = backtrace;
        out
    }
}

impl<'a, T, L: Location<'a>, O: Ordering, R: Retries> Unslicable
    for super::Stream<T, Tick<L>, Bounded, O, R>
{
    type Unsliced = super::Stream<T, L, Unbounded, O, R>;

    fn unslice(self) -> Self::Unsliced {
        self.all_ticks()
    }
}

impl<'a, T, L: Location<'a>, B: Boundedness> Slicable<'a, L> for super::Singleton<T, L, B> {
    type Slice = super::Singleton<T, Tick<L>, Bounded>;
    type Backtrace = crate::compile::ir::backtrace::Backtrace;

    fn get_location(&self) -> &L {
        self.location()
    }

    fn preferred_tick(&self) -> Option<Tick<L>> {
        None
    }

    fn slice(self, tick: &Tick<L>, backtrace: Self::Backtrace, nondet: NonDet) -> Self::Slice {
        let out = self.snapshot(tick, nondet);
        out.ir_node.borrow_mut().op_metadata_mut().backtrace = backtrace;
        out
    }
}

impl<'a, T, L: Location<'a>> Unslicable for super::Singleton<T, Tick<L>, Bounded> {
    type Unsliced = super::Singleton<T, L, Unbounded>;

    fn unslice(self) -> Self::Unsliced {
        self.latest()
    }
}

impl<'a, T, L: Location<'a>, B: Boundedness> Slicable<'a, L> for super::Optional<T, L, B> {
    type Slice = super::Optional<T, Tick<L>, Bounded>;
    type Backtrace = crate::compile::ir::backtrace::Backtrace;

    fn get_location(&self) -> &L {
        self.location()
    }

    fn preferred_tick(&self) -> Option<Tick<L>> {
        None
    }

    fn slice(self, tick: &Tick<L>, backtrace: Self::Backtrace, nondet: NonDet) -> Self::Slice {
        let out = self.snapshot(tick, nondet);
        out.ir_node.borrow_mut().op_metadata_mut().backtrace = backtrace;
        out
    }
}

impl<'a, T, L: Location<'a>> Unslicable for super::Optional<T, Tick<L>, Bounded> {
    type Unsliced = super::Optional<T, L, Unbounded>;

    fn unslice(self) -> Self::Unsliced {
        self.latest()
    }
}

impl<'a, K, V, L: Location<'a>, O: Ordering, R: Retries> Slicable<'a, L>
    for super::KeyedStream<K, V, L, Unbounded, O, R>
{
    type Slice = super::KeyedStream<K, V, Tick<L>, Bounded, O, R>;
    type Backtrace = crate::compile::ir::backtrace::Backtrace;

    fn get_location(&self) -> &L {
        self.location()
    }

    fn preferred_tick(&self) -> Option<Tick<L>> {
        None
    }

    fn slice(self, tick: &Tick<L>, backtrace: Self::Backtrace, nondet: NonDet) -> Self::Slice {
        let out = self.batch(tick, nondet);
        out.ir_node.borrow_mut().op_metadata_mut().backtrace = backtrace;
        out
    }
}

impl<'a, K, V, L: Location<'a>, O: Ordering, R: Retries> Unslicable
    for super::KeyedStream<K, V, Tick<L>, Bounded, O, R>
{
    type Unsliced = super::KeyedStream<K, V, L, Unbounded, O, R>;

    fn unslice(self) -> Self::Unsliced {
        self.all_ticks()
    }
}

impl<'a, K, V, L: Location<'a>> Slicable<'a, L> for super::KeyedSingleton<K, V, L, Unbounded> {
    type Slice = super::KeyedSingleton<K, V, Tick<L>, Bounded>;
    type Backtrace = crate::compile::ir::backtrace::Backtrace;

    fn get_location(&self) -> &L {
        self.location()
    }

    fn preferred_tick(&self) -> Option<Tick<L>> {
        None
    }

    fn slice(self, tick: &Tick<L>, backtrace: Self::Backtrace, nondet: NonDet) -> Self::Slice {
        let out = self.snapshot(tick, nondet);
        out.ir_node.borrow_mut().op_metadata_mut().backtrace = backtrace;
        out
    }
}

impl<'a, K, V, L: Location<'a> + NoTick> Slicable<'a, L>
    for super::KeyedSingleton<K, V, L, BoundedValue>
{
    type Slice = super::KeyedSingleton<K, V, Tick<L>, Bounded>;
    type Backtrace = crate::compile::ir::backtrace::Backtrace;

    fn get_location(&self) -> &L {
        self.location()
    }

    fn preferred_tick(&self) -> Option<Tick<L>> {
        None
    }

    fn slice(self, tick: &Tick<L>, backtrace: Self::Backtrace, nondet: NonDet) -> Self::Slice {
        let out = self.batch(tick, nondet);
        out.ir_node.borrow_mut().op_metadata_mut().backtrace = backtrace;
        out
    }
}

#[cfg(feature = "sim")]
#[cfg(test)]
mod tests {
    use stageleft::q;

    use super::sliced;
    use crate::location::Location;
    use crate::nondet::nondet;
    use crate::prelude::FlowBuilder;

    /// Test a counter using `use::state` with an initial singleton value.
    /// Each input increments the counter, and we verify the output after each tick.

    #[test]
    fn sim_state_counter() {
        let flow = FlowBuilder::new();
        let node = flow.process::<()>();

        let (input_send, input) = node.sim_input::<i32, _, _>();

        let out_recv = sliced! {
            let batch = use(input, nondet!(/** test */));
            let mut counter = use::state(|l| l.singleton(q!(0)));

            let new_count = counter.clone().zip(batch.count())
                .map(q!(|(old, add)| old + add));
            counter = new_count.clone();
            new_count.into_stream()
        }
        .sim_output();

        flow.sim().exhaustive(async || {
            input_send.send(1);
            assert_eq!(out_recv.next().await.unwrap(), 1);

            input_send.send(1);
            assert_eq!(out_recv.next().await.unwrap(), 2);

            input_send.send(1);
            assert_eq!(out_recv.next().await.unwrap(), 3);
        });
    }

    /// Test `use::state_null` with an Optional that starts as None.
    #[cfg(feature = "sim")]
    #[test]
    fn sim_state_null_optional() {
        use crate::live_collections::Optional;
        use crate::live_collections::boundedness::Bounded;
        use crate::location::{Location, Tick};

        let flow = FlowBuilder::new();
        let node = flow.process::<()>();

        let (input_send, input) = node.sim_input::<i32, _, _>();

        let out_recv = sliced! {
            let batch = use(input, nondet!(/** test */));
            let mut prev = use::state_null::<Optional<i32, Tick<_>, Bounded>>();

            // Output the previous value (or -1 if none)
            let output = prev.clone().unwrap_or(prev.location().singleton(q!(-1)));
            // Store the current batch's first value for next tick
            prev = batch.first();
            output.into_stream()
        }
        .sim_output();

        flow.sim().exhaustive(async || {
            input_send.send(10);
            // First tick: prev is None, so output is -1
            assert_eq!(out_recv.next().await.unwrap(), -1);

            input_send.send(20);
            // Second tick: prev is Some(10), so output is 10
            assert_eq!(out_recv.next().await.unwrap(), 10);

            input_send.send(30);
            // Third tick: prev is Some(20), so output is 20
            assert_eq!(out_recv.next().await.unwrap(), 20);
        });
    }
}
