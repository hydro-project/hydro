//! Utilities for transforming live collections via slicing.

use super::boundedness::{Bounded, Unbounded};
use crate::live_collections::keyed_singleton::BoundedValue;
use crate::live_collections::stream::{Ordering, Retries};
use crate::location::{Location, NoTick, Tick};
use crate::nondet::NonDet;

/// Transforms a live collection with a computation relying on a slice of another live collection.
/// This is useful for reading a snapshot of an asynchronously updated collection while processing another
/// collection, such as joining a stream with the latest values from a singleton.
///
/// # Syntax
/// The `sliced!` macro takes in a closure-like syntax specifying the live collections to be sliced
/// and the body of the transformation. Each `use` statement indicates a live collection to be sliced,
/// along with a non-determinism explanation. Optionally, a style can be specified to control how the
/// live collection is sliced (e.g., atomically).
///
/// ```rust,ignore
/// let stream = sliced!(|
///     use(collection1, nondet!(/** explanation */)) as name1,
///     use::atomic(collection2, nondet!(/** explanation */)) as name2,
///     ...
/// | {
///     // body using name1, name2, etc.
/// });
/// ```
///
/// # Example with two collections
/// ```rust
/// # use hydro_lang::prelude::*;
/// # use futures::StreamExt;
/// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
/// let singleton = process.singleton(q!(5));
/// let stream = process.source_iter(q!(vec![1, 2, 3]));
/// let out: Stream<(i32, i32), _> = sliced!(|
///     use(stream, nondet!(/** test */)) as batch_of_req,
///     use(singleton, nondet!(/** test */)) as latest_singleton
/// | {
///     batch_of_req.cross_singleton(latest_singleton)
/// });
/// # out
/// # }, |mut stream| async move {
/// # assert_eq!(stream.next().await.unwrap(), (1, 5));
/// # assert_eq!(stream.next().await.unwrap(), (2, 5));
/// # assert_eq!(stream.next().await.unwrap(), (3, 5));
/// # }));
/// ```
#[macro_export]
macro_rules! __sliced__ {
    (|use$(::$first_style:ident)?($first:expr, $nondet_first:expr) as $first_name:ident$(, use$(::$rest_style:ident)?($rest:expr, $nondet_expl:expr) as $rest_name:ident),*| $body:expr) => {
        {
            let _ = $nondet_first;
            $(let _ = $nondet_expl;)*
            $crate::live_collections::sliced::transform_sliced(
                $($crate::live_collections::sliced::style::$first_style)?($first),
                ($($($crate::live_collections::sliced::style::$rest_style)?($rest),)*),
                $crate::nondet::NonDet,
                |$first_name, ($($rest_name,)*)| $body
            )
        }
    };
}

pub use crate::__sliced__ as sliced;

/// Styles for use with the `sliced!` macro.
pub mod style {
    use super::Slicable;
    use crate::live_collections::boundedness::{Bounded, Unbounded};
    use crate::live_collections::keyed_singleton::BoundedValue;
    use crate::live_collections::stream::{Ordering, Retries, Stream};
    use crate::location::{Location, NoTick, Tick};
    use crate::nondet::NonDet;

    /// Marks a live collection to be treated atomically during slicing.
    pub struct Atomic<T>(pub T);

    /// Wraps a live collection to be treated atomically during slicing.
    pub fn atomic<T>(t: T) -> Atomic<T> {
        Atomic(t)
    }

    impl<'a, T, L: Location<'a> + NoTick, O: Ordering, R: Retries> Slicable<'a, L>
        for Atomic<Stream<T, crate::location::Atomic<L>, Unbounded, O, R>>
    {
        type Slice = Stream<T, Tick<L>, Bounded, O, R>;

        fn preferred_tick(&self) -> Option<Tick<L>> {
            Some(self.0.location().tick().as_regular_tick())
        }

        fn get_location(&self) -> &L {
            panic!("Atomic location has no accessible inner location")
        }

        fn slice(self, tick: &Tick<L>, nondet: NonDet) -> Self::Slice {
            assert_eq!(
                self.0.location().tick().as_regular_tick().id(),
                tick.id(),
                "Mismatched tick for atomic slicing"
            );
            self.0.batch_atomic(nondet)
        }
    }

    impl<'a, T, L: Location<'a> + NoTick> Slicable<'a, L>
        for Atomic<crate::live_collections::Singleton<T, crate::location::Atomic<L>, Unbounded>>
    {
        type Slice = crate::live_collections::Singleton<T, Tick<L>, Bounded>;

        fn preferred_tick(&self) -> Option<Tick<L>> {
            Some(self.0.location().tick().as_regular_tick())
        }

        fn get_location(&self) -> &L {
            panic!("Atomic location has no accessible inner location")
        }

        fn slice(self, tick: &Tick<L>, nondet: NonDet) -> Self::Slice {
            assert_eq!(
                self.0.location().tick().as_regular_tick().id(),
                tick.id(),
                "Mismatched tick for atomic slicing"
            );
            self.0.snapshot_atomic(nondet)
        }
    }

    impl<'a, T, L: Location<'a> + NoTick> Slicable<'a, L>
        for Atomic<crate::live_collections::Optional<T, crate::location::Atomic<L>, Unbounded>>
    {
        type Slice = crate::live_collections::Optional<T, Tick<L>, Bounded>;

        fn preferred_tick(&self) -> Option<Tick<L>> {
            Some(self.0.location().tick().as_regular_tick())
        }

        fn get_location(&self) -> &L {
            panic!("Atomic location has no accessible inner location")
        }

        fn slice(self, tick: &Tick<L>, nondet: NonDet) -> Self::Slice {
            assert_eq!(
                self.0.location().tick().as_regular_tick().id(),
                tick.id(),
                "Mismatched tick for atomic slicing"
            );
            self.0.snapshot_atomic(nondet)
        }
    }

    impl<'a, K, V, L: Location<'a> + NoTick> Slicable<'a, L>
        for Atomic<
            crate::live_collections::KeyedSingleton<K, V, crate::location::Atomic<L>, Unbounded>,
        >
    {
        type Slice = crate::live_collections::KeyedSingleton<K, V, Tick<L>, Bounded>;

        fn preferred_tick(&self) -> Option<Tick<L>> {
            Some(self.0.location().tick().as_regular_tick())
        }

        fn get_location(&self) -> &L {
            panic!("Atomic location has no accessible inner location")
        }

        fn slice(self, tick: &Tick<L>, nondet: NonDet) -> Self::Slice {
            assert_eq!(
                self.0.location().tick().as_regular_tick().id(),
                tick.id(),
                "Mismatched tick for atomic slicing"
            );
            self.0.snapshot_atomic(nondet)
        }
    }

    impl<'a, K, V, L: Location<'a> + NoTick> Slicable<'a, L>
        for Atomic<
            crate::live_collections::KeyedSingleton<K, V, crate::location::Atomic<L>, BoundedValue>,
        >
    {
        type Slice = crate::live_collections::KeyedSingleton<K, V, Tick<L>, Bounded>;

        fn preferred_tick(&self) -> Option<Tick<L>> {
            Some(self.0.location().tick().as_regular_tick())
        }

        fn get_location(&self) -> &L {
            panic!("Atomic location has no accessible inner location")
        }

        fn slice(self, tick: &Tick<L>, nondet: NonDet) -> Self::Slice {
            assert_eq!(
                self.0.location().tick().as_regular_tick().id(),
                tick.id(),
                "Mismatched tick for atomic slicing"
            );
            self.0.batch_atomic(nondet)
        }
    }
}

/// A trait for live collections which can be sliced into bounded versions at a tick.
pub trait Slicable<'a, L: Location<'a>> {
    /// The sliced version of this live collection.
    type Slice;

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
    fn slice(self, tick: &Tick<L>, nondet: NonDet) -> Self::Slice;
}

/// A trait for live collections which can be yielded out of a slice back into their original form.
pub trait Unslicable {
    /// The unsliced version of this live collection.
    type Unsliced;

    /// Unslices a sliced live collection back into its original form.
    fn unslice(self) -> Self::Unsliced;
}

/// Transforms a live collection with a computation relying on a slice of another live collection.
/// This is useful for reading a snapshot of an asynchronously updated collection while processing another
/// collection, such as joining a stream with the latest values from a singleton.
///
/// For a cleaner syntax, see the [`sliced!`] macro.
///
/// # Example
/// ```rust
/// # use hydro_lang::prelude::*;
/// # use hydro_lang::live_collections::sliced::transform_sliced;
/// # use futures::StreamExt;
/// # tokio_test::block_on(hydro_lang::test_util::stream_transform_test(|process| {
/// let singleton = // some unbounded singleton that has value 5 for the duration of the test
/// # process.singleton(q!(5));
/// let stream = process.source_iter(q!(vec![1, 2, 3]));
/// transform_sliced(stream, singleton, nondet!(/** test */), |batch_of_req, latest_singleton| {
///     batch_of_req.cross_singleton(latest_singleton)
/// })
/// # }, |mut stream| async move {
/// // (1, 5), (2, 5), (3, 5)
/// # assert_eq!(stream.next().await.unwrap(), (1, 5));
/// # assert_eq!(stream.next().await.unwrap(), (2, 5));
/// # assert_eq!(stream.next().await.unwrap(), (3, 5));
/// # }));
/// ```
pub fn transform_sliced<
    'a,
    L: Location<'a> + NoTick,
    C: Slicable<'a, L>,
    S: Slicable<'a, L>,
    O: Unslicable,
>(
    c: C,
    with: S,
    nondet: NonDet,
    thunk: impl FnOnce(C::Slice, S::Slice) -> O,
) -> O::Unsliced {
    let mut preferred = c.preferred_tick();
    if let Some(tick) = with.preferred_tick() {
        preferred = Some(match preferred {
            Some(current) => {
                if Location::id(&current) == Location::id(&tick) {
                    current
                } else {
                    panic!("Mismatched preferred ticks for sliced collections")
                }
            }
            None => tick,
        });
    }

    let tick = preferred.unwrap_or_else(|| c.get_location().tick());
    let c_slice = c.slice(&tick, nondet);
    let s_slice = with.slice(&tick, nondet);
    let o_slice = thunk(c_slice, s_slice);
    o_slice.unslice()
}

impl<'a, L: Location<'a>> Slicable<'a, L> for () {
    type Slice = ();

    fn get_location(&self) -> &L {
        unreachable!()
    }

    fn preferred_tick(&self) -> Option<Tick<L>> {
        None
    }

    fn slice(self, _tick: &Tick<L>, _nondet: NonDet) -> Self::Slice {}
}

macro_rules! impl_slicable_for_tuple {
    ($($T:ident, $idx:tt),*) => {
        impl<'a, L: Location<'a>, $($T: Slicable<'a, L>),*> Slicable<'a, L> for ($($T,)*) {
            type Slice = ($($T::Slice,)*);

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
                )*
                preferred
            }

            #[expect(non_snake_case, reason = "macro codegen")]
            fn slice(self, tick: &Tick<L>, nondet: NonDet) -> Self::Slice {
                let ($($T,)*) = self;
                ($($T.slice(tick, nondet),)*)
            }
        }
    };
}

#[cfg(stageleft_runtime)]
impl_slicable_for_tuple!(S1, 0);
#[cfg(stageleft_runtime)]
impl_slicable_for_tuple!(S1, 0, S2, 1);
#[cfg(stageleft_runtime)]
impl_slicable_for_tuple!(S1, 0, S2, 1, S3, 2);
#[cfg(stageleft_runtime)]
impl_slicable_for_tuple!(S1, 0, S2, 1, S3, 2, S4, 3);
#[cfg(stageleft_runtime)]
impl_slicable_for_tuple!(S1, 0, S2, 1, S3, 2, S4, 3, S5, 4); // 5 slices ought to be enough for anyone

impl<'a, T, L: Location<'a>, O: Ordering, R: Retries> Slicable<'a, L>
    for super::Stream<T, L, Unbounded, O, R>
{
    type Slice = super::Stream<T, Tick<L>, Bounded, O, R>;

    fn get_location(&self) -> &L {
        self.location()
    }

    fn preferred_tick(&self) -> Option<Tick<L>> {
        None
    }

    fn slice(self, tick: &Tick<L>, nondet: NonDet) -> Self::Slice {
        self.batch(tick, nondet)
    }
}

impl<'a, T, L: Location<'a>> Unslicable for super::Stream<T, Tick<L>, Bounded> {
    type Unsliced = super::Stream<T, L, Unbounded>;

    fn unslice(self) -> Self::Unsliced {
        self.all_ticks()
    }
}

impl<'a, T, L: Location<'a>> Slicable<'a, L> for super::Singleton<T, L, Unbounded> {
    type Slice = super::Singleton<T, Tick<L>, Bounded>;

    fn get_location(&self) -> &L {
        self.location()
    }

    fn preferred_tick(&self) -> Option<Tick<L>> {
        None
    }

    fn slice(self, tick: &Tick<L>, nondet: NonDet) -> Self::Slice {
        self.snapshot(tick, nondet)
    }
}

impl<'a, T, L: Location<'a>> Unslicable for super::Singleton<T, Tick<L>, Bounded> {
    type Unsliced = super::Singleton<T, L, Unbounded>;

    fn unslice(self) -> Self::Unsliced {
        self.latest()
    }
}

impl<'a, T, L: Location<'a>> Slicable<'a, L> for super::Optional<T, L, Unbounded> {
    type Slice = super::Optional<T, Tick<L>, Bounded>;

    fn get_location(&self) -> &L {
        self.location()
    }

    fn preferred_tick(&self) -> Option<Tick<L>> {
        None
    }

    fn slice(self, tick: &Tick<L>, nondet: NonDet) -> Self::Slice {
        self.snapshot(tick, nondet)
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

    fn get_location(&self) -> &L {
        self.location()
    }

    fn preferred_tick(&self) -> Option<Tick<L>> {
        None
    }

    fn slice(self, tick: &Tick<L>, nondet: NonDet) -> Self::Slice {
        self.batch(tick, nondet)
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

    fn get_location(&self) -> &L {
        self.location()
    }

    fn preferred_tick(&self) -> Option<Tick<L>> {
        None
    }

    fn slice(self, tick: &Tick<L>, nondet: NonDet) -> Self::Slice {
        self.snapshot(tick, nondet)
    }
}

impl<'a, K, V, L: Location<'a> + NoTick> Slicable<'a, L>
    for super::KeyedSingleton<K, V, L, BoundedValue>
{
    type Slice = super::KeyedSingleton<K, V, Tick<L>, Bounded>;

    fn get_location(&self) -> &L {
        self.location()
    }

    fn preferred_tick(&self) -> Option<Tick<L>> {
        None
    }

    fn slice(self, tick: &Tick<L>, nondet: NonDet) -> Self::Slice {
        self.batch(tick, nondet)
    }
}
