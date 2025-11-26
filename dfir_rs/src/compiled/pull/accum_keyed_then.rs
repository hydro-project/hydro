use std::collections::HashMap;
use std::hash::{BuildHasher, Hash};
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use futures::Stream;
use pin_project_lite::pin_project;

use crate::util::accumulator::Accumulator;

pin_project! {
    #[project = AccumKeyedThenStateProj]
    #[project_replace = AccumKeyedThenStateProjOwn]
    enum AccumKeyedThenState<'a, St, Key, ValAccum, Hasher, Accum, ThenFn, ThenIter> {
        Accumulating {
            #[pin]
            stream: St,

            state: &'a mut HashMap<Key, ValAccum, Hasher>,

            accum: Accum,
            then_fn: ThenFn,
        },
        Emitting {
            iter: ThenIter,
        },
        Empty,
    }
}

pin_project! {
    /// Special stream for all forms (lifetimes) of `fold_keyed`.
    #[must_use = "streams do nothing unless polled"]
    pub struct AccumKeyedThen<'a, St, Key, ValAccum, Hasher, Accum, ThenFn, ThenIter> {
        #[pin]
        state: AccumKeyedThenState<'a, St, Key, ValAccum, Hasher, Accum, ThenFn, ThenIter>
    }
}

impl<'a, St, Key, ValAccum, Hasher, Accum, ThenFn, ThenIter, ValIn>
    AccumKeyedThen<'a, St, Key, ValAccum, Hasher, Accum, ThenFn, ThenIter>
where
    St: Stream<Item = (Key, ValIn)>,
    Key: Eq + Hash,
    Hasher: BuildHasher,
    Accum: Accumulator<ValAccum, ValIn>,
    ThenFn: FnOnce(&'a mut HashMap<Key, ValAccum, Hasher>) -> ThenIter,
    ThenIter: Iterator,
{
    /// Creates a new `AccumKeyedThen` stream.
    pub fn new(
        stream: St,
        state: &'a mut HashMap<Key, ValAccum, Hasher>,
        accum: Accum,
        then_fn: ThenFn,
    ) -> Self {
        Self {
            state: AccumKeyedThenState::Accumulating {
                stream,
                state,
                accum,
                then_fn,
            },
        }
    }
}

impl<'a, St, Key, ValAccum, Hasher, Accum, ThenFn, ThenIter, ValIn> Stream
    for AccumKeyedThen<'a, St, Key, ValAccum, Hasher, Accum, ThenFn, ThenIter>
where
    St: Stream<Item = (Key, ValIn)>,
    Key: Eq + Hash,
    Hasher: BuildHasher,
    Accum: Accumulator<ValAccum, ValIn>,
    ThenFn: FnOnce(&'a mut HashMap<Key, ValAccum, Hasher>) -> ThenIter,
    ThenIter: Iterator,
{
    type Item = ThenIter::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.as_mut().project();

        match this.state.as_mut().project() {
            AccumKeyedThenStateProj::Accumulating {
                mut stream,
                state,
                accum,
                then_fn: _,
            } => loop {
                if let Some((key, val_in)) = ready!(stream.as_mut().poll_next(cx)) {
                    accum.accumulate(state.entry(key), val_in);
                } else {
                    let AccumKeyedThenStateProjOwn::Accumulating {
                        stream: _,
                        state,
                        accum: _,
                        then_fn,
                    } = this
                        .state
                        .as_mut()
                        .project_replace(AccumKeyedThenState::Empty)
                    else {
                        unreachable!();
                    };
                    let iter = (then_fn)(state);
                    this.state.set(AccumKeyedThenState::Emitting { iter });
                    return self.poll_next(cx);
                }
            },
            AccumKeyedThenStateProj::Emitting { iter } => {
                if let Some(item) = iter.next() {
                    Poll::Ready(Some(item))
                } else {
                    this.state.set(AccumKeyedThenState::Empty);
                    Poll::Ready(None)
                }
            }
            AccumKeyedThenStateProj::Empty => Poll::Ready(None),
        }
    }
}
