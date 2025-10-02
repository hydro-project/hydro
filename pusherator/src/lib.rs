//! [`Pusherator`], the _push_ equivalent of _pull_ [`Iterator]`s.
//!
//! Pusherator generics and argument order conventions:
//! - `Next` (being the next owned pusherator) should come first in generic
//!   arguments.
//! - However `next: Next` in `new(...)` arguments should come last. This is so
//!   the rest of the arguments appear in the order data flows in.
//! - Any closures `Func` should come before their arguments, so:
//!   `<Func: Fn(A) -> B, A, B>`

#[cfg(feature = "demux")]
pub mod demux;
pub mod filter;
pub mod filter_map;
pub mod flatten;
pub mod for_each;
pub mod inspect;
pub mod map;
pub mod null;
pub mod partition;
pub mod pivot;
pub mod switch;
pub mod tee;
pub mod unzip;

use std::marker::PhantomData;

use either::Either;

pub trait Pusherator: Sized {
    type Item;
    fn give(&mut self, item: Self::Item);
}

pub trait IteratorToPusherator: Iterator {
    fn pull_to_push(self) -> pivot::PivotBuild<Self>
    where
        Self: Sized,
    {
        pivot::PivotBuild::new(self)
    }
}
impl<I> IteratorToPusherator for I where I: Sized + Iterator {}

pub trait PusheratorBuild {
    type ItemOut;

    type Output<Next: Pusherator<Item = Self::ItemOut>>;
    fn push_to<Next>(self, input: Next) -> Self::Output<Next>
    where
        Next: Pusherator<Item = Self::ItemOut>;

    fn map<Func, Out>(self, func: Func) -> map::MapBuild<Self, Func>
    where
        Self: Sized,
        Func: FnMut(Self::ItemOut) -> Out,
    {
        map::MapBuild::new(self, func)
    }

    fn inspect<Func>(self, func: Func) -> inspect::InspectBuild<Self, Func>
    where
        Self: Sized,
        Func: FnMut(&Self::ItemOut),
    {
        inspect::InspectBuild::new(self, func)
    }

    fn filter<Func>(self, func: Func) -> filter::FilterBuild<Self, Func>
    where
        Self: Sized,
        Func: FnMut(&Self::ItemOut) -> bool,
    {
        filter::FilterBuild::new(self, func)
    }

    fn tee<Next1>(self, next1: Next1) -> tee::TeeBuild<Self, Next1>
    where
        Self: Sized,
        Self::ItemOut: Clone,
        Next1: Pusherator<Item = Self::ItemOut>,
    {
        tee::TeeBuild::new(self, next1)
    }

    fn unzip<Next1, Item2>(self, next1: Next1) -> unzip::UnzipBuild<Self, Next1>
    where
        Self: Sized,
        Self: PusheratorBuild<ItemOut = (Next1::Item, Item2)>,
        Next1: Pusherator,
    {
        unzip::UnzipBuild::new(self, next1)
    }

    fn switch<Next1, Item2>(self, next1: Next1) -> switch::SwitchBuild<Self, Next1>
    where
        Self: Sized,
        Self: PusheratorBuild<ItemOut = Either<Next1::Item, Item2>>,
        Next1: Pusherator,
    {
        switch::SwitchBuild::new(self, next1)
    }

    fn for_each<Func>(self, func: Func) -> Self::Output<for_each::ForEach<Func, Self::ItemOut>>
    where
        Self: Sized,
        Func: FnMut(Self::ItemOut),
    {
        self.push_to(for_each::ForEach::new(func))
    }

    #[cfg(feature = "demux")]
    fn demux<Func, Nexts>(
        self,
        func: Func,
        nexts: Nexts,
    ) -> Self::Output<demux::Demux<Func, Nexts, Self::ItemOut>>
    where
        Self: Sized,
        Func: FnMut(Self::ItemOut, &mut Nexts),
    {
        self.push_to(demux::Demux::new(func, nexts))
    }
}

pub struct InputBuild<T>(PhantomData<T>);
impl<T> Default for InputBuild<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}
impl<T> InputBuild<T> {
    pub fn new() -> Self {
        Default::default()
    }
}
impl<T> PusheratorBuild for InputBuild<T> {
    type ItemOut = T;

    type Output<O: Pusherator<Item = Self::ItemOut>> = O;
    fn push_to<O>(self, input: O) -> Self::Output<O>
    where
        O: Pusherator<Item = Self::ItemOut>,
    {
        input
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::Pusherator;
    use super::filter::Filter;
    use super::for_each::ForEach;
    use super::map::Map;
    use super::partition::Partition;
    use super::pivot::Pivot;
    use super::tee::Tee;

    #[test]
    fn linear_chains() {
        let mut v = Vec::new();
        let mut pusher = Map::new(
            |x| x * 2,
            Filter::new(|x| *x > 5, ForEach::new(|x| v.push(x))),
        );

        for i in 0..5 {
            pusher.give(i);
        }

        assert_eq!(v, vec![6, 8]);
    }

    #[test]
    fn partition() {
        let mut evens = Vec::new();
        let mut odds = Vec::new();
        let mut pusher = Partition::new(
            |x| x % 2 == 0,
            ForEach::new(|x| evens.push(x)),
            ForEach::new(|x| odds.push(x)),
        );

        for i in 0..5 {
            pusher.give(i);
        }

        assert_eq!(evens, vec![0, 2, 4]);
        assert_eq!(odds, vec![1, 3]);
    }

    #[test]
    fn tee() {
        let mut left = Vec::new();
        let mut right = Vec::new();
        let mut pusher = Tee::new(
            ForEach::new(|x| left.push(x)),
            ForEach::new(|x| right.push(x)),
        );

        for i in 0..5 {
            pusher.give(i);
        }

        assert_eq!(left, vec![0, 1, 2, 3, 4]);
        assert_eq!(right, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn tee_rcs() {
        let mut left = Vec::new();
        let mut right = Vec::new();
        let mut pusher = Map::new(
            Rc::new,
            Tee::new(
                ForEach::new(|x: Rc<i32>| left.push(*x)),
                ForEach::new(|x: Rc<i32>| right.push(*x)),
            ),
        );

        for i in 0..5 {
            pusher.give(i);
        }

        assert_eq!(left, vec![0, 1, 2, 3, 4]);
        assert_eq!(right, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn pivot() {
        let a = 0..10;
        let b = 10..20;

        let mut left = Vec::new();
        let mut right = Vec::new();

        let pivot = Pivot::new(
            a.into_iter().chain(b),
            Partition::new(
                |x| x % 2 == 0,
                ForEach::new(|x| left.push(x)),
                ForEach::new(|x| right.push(x)),
            ),
        );

        pivot.run();

        assert_eq!(left, vec![0, 2, 4, 6, 8, 10, 12, 14, 16, 18]);
        assert_eq!(right, vec![1, 3, 5, 7, 9, 11, 13, 15, 17, 19]);
    }
}

#[cfg(test)]
mod test_builder {
    use super::*;

    #[test]
    fn test_builder_constructed() {
        let pb = InputBuild::<usize>(PhantomData);
        let pb = filter::FilterBuild::new(pb, |&x| 0 == x % 2);
        let pb = map::MapBuild::new(pb, |x| x * x);

        let mut output = Vec::new();
        let mut pusherator = pb.push_to(for_each::ForEach::new(|x| output.push(x)));

        for x in 0..10 {
            pusherator.give(x);
        }

        assert_eq!(&[0, 4, 16, 36, 64], &*output);
    }

    #[test]
    fn test_builder() {
        let mut output = Vec::new();

        let mut pusherator = <InputBuild<usize>>::new()
            .filter(|&x| 0 == x % 2)
            .map(|x| x * x)
            .for_each(|x| output.push(x));

        for x in 0..10 {
            pusherator.give(x);
        }

        assert_eq!(&[0, 4, 16, 36, 64], &*output);
    }

    #[test]
    fn test_builder_tee() {
        let mut output_evn = Vec::new();
        let mut output_odd = Vec::new();

        let mut pusherator = <InputBuild<usize>>::new()
            .tee(
                <InputBuild<usize>>::new()
                    .filter(|&x| 0 == x % 2)
                    .for_each(|x| output_evn.push(x)),
            )
            .filter(|&x| 1 == x % 2)
            .for_each(|x| output_odd.push(x));

        for x in 0..10 {
            pusherator.give(x);
        }

        assert_eq!(&[0, 2, 4, 6, 8], &*output_evn);
        assert_eq!(&[1, 3, 5, 7, 9], &*output_odd);
    }

    #[test]
    fn test_built_subgraph() {
        let mut output_evn = Vec::new();
        let mut output_odd = Vec::new();

        let pivot = [1, 2, 3, 4, 5]
            .into_iter()
            .chain([3, 4, 5, 6, 7])
            .map(|x| x * 9)
            .pull_to_push()
            .map(|x| if 0 == x % 2 { x / 2 } else { 3 * x + 1 })
            .tee(
                <InputBuild<usize>>::new()
                    .filter(|&x| 0 == x % 2)
                    .for_each(|x| output_evn.push(x)),
            )
            .filter(|&x| 1 == x % 2)
            .for_each(|x| output_odd.push(x));

        pivot.run();

        assert_eq!(&[28, 82, 18, 136, 82, 18, 136, 190], &*output_evn);
        assert_eq!(&[9, 27], &*output_odd);
    }
}
