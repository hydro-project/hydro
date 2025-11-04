//! Utilities for transforming live collections via slicing.

use super::boundedness::{Bounded, Unbounded};
use crate::live_collections::stream::{Ordering, Retries};
use crate::location::{Location, NoTick, Tick};
use crate::nondet::NonDet;

/// Helper macro to build tuples for sliced macro
#[doc(hidden)]
#[macro_export]
macro_rules! __sliced_tuple__ {
    () => {
        ()
    };
    ($single:ident) => {
        $single
    };
    ($first:ident, $($rest:ident),+) => {
        ($first, $($rest),+)
    };
}

/// Transforms a live collection with a computation relying on a slice of another live collection.
/// This is useful for reading a snapshot of an asynchronously updated collection while processing another
/// collection, such as joining a stream with the latest values from a singleton.
///
/// # Syntax
/// ```ignore
/// let stream = sliced!(|
///     use(nondet!(/** explanation */)) collection1 as name1,
///     use(nondet!(/** explanation */)) collection2 as name2,
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
///     use stream as batch_of_req,
///     use(nondet!(/** test */)) singleton as latest_singleton
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
    (|use($nondet_first:expr) $first:ident as $first_name:ident$(, use($nondet_expl:expr) $rest:ident as $rest_name:ident),*| $body:expr) => {
        {
            let _ = $nondet_first;
            $(let _ = $nondet_expl;)*
            $crate::live_collections::sliced::transform_sliced(
                $first,
                $crate::__sliced_tuple__!($($rest),*),
                $crate::nondet::NonDet,
                |$first_name, $crate::__sliced_tuple__!($($rest_name),*)| $body
            )
        }
    };
}

pub use crate::__sliced__ as sliced;

/// A trait for live collections which can be sliced into bounded versions at a tick.
pub trait Slicable<'a, L: Location<'a>> {
    /// The sliced version of this live collection.
    type Slice;

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
    let tick = c.get_location().tick();
    let c_slice = c.slice(&tick, nondet);
    let s_slice = with.slice(&tick, nondet);
    let o_slice = thunk(c_slice, s_slice);
    o_slice.unslice()
}

impl <'a, L: Location<'a>> Slicable<'a, L> for () {
    type Slice = ();

    fn get_location(&self) -> &L {
        unreachable!()
    }

    fn slice(self, _tick: &Tick<L>, _nondet: NonDet) -> Self::Slice {}
}

macro_rules! impl_slicable_for_tuple {
    ($($T:ident),*) => {
        impl<'a, L: Location<'a>, $($T: Slicable<'a, L>),*> Slicable<'a, L> for ($($T,)*) {
            type Slice = ($($T::Slice,)*);

            fn get_location(&self) -> &L {
                self.0.get_location()
            }

            #[allow(non_snake_case, unused_variables)]
            fn slice(self, tick: &Tick<L>, nondet: NonDet) -> Self::Slice {
                let ($($T,)*) = self;
                ($($T.slice(tick, nondet),)*)
            }
        }
    };
}

impl_slicable_for_tuple!(S1);
impl_slicable_for_tuple!(S1, S2);
impl_slicable_for_tuple!(S1, S2, S3);
impl_slicable_for_tuple!(S1, S2, S3, S4);
impl_slicable_for_tuple!(S1, S2, S3, S4, S5); // 5 slices ought to be enough for anyone

impl<'a, T, L: Location<'a>, O: Ordering, R: Retries> Slicable<'a, L>
    for super::Stream<T, L, Unbounded, O, R>
{
    type Slice = super::Stream<T, Tick<L>, Bounded, O, R>;

    fn get_location(&self) -> &L {
        self.location()
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

    fn slice(self, tick: &Tick<L>, nondet: NonDet) -> Self::Slice {
        self.snapshot(tick, nondet)
    }
}
