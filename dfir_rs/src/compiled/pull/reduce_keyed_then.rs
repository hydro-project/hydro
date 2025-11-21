use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::hash::{BuildHasher, Hash};
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use futures::Stream;
use pin_project_lite::pin_project;

pin_project! {
    #[project = ReduceKeyedThenStateProj]
    #[project_replace = ReduceKeyedThenStateProjOwn]
    enum ReduceKeyedThenState<'a, St, Key, Val, Hasher, AggFn, ThenFn, ThenIter> {
        Reducing {
            #[pin]
            stream: St,

            state: &'a mut HashMap<Key, Val, Hasher>,

            agg_fn: AggFn,
            then_fn: ThenFn,
        },
        Emitting {
            iter: ThenIter,
        },
        Empty,
    }
}

pin_project! {
    /// Special stream for all forms (lifetimes) of `reduce_keyed`.
    #[must_use = "streams do nothing unless polled"]
    pub struct ReduceKeyedThen<'a, St, Key, Val, Hasher, AggFn, ThenFn, ThenIter> {
        #[pin]
        state: ReduceKeyedThenState<'a, St, Key, Val, Hasher, AggFn, ThenFn, ThenIter>
    }
}

impl<'a, St, Key, Val, Hasher, AggFn, ThenFn, ThenIter>
    ReduceKeyedThen<'a, St, Key, Val, Hasher, AggFn, ThenFn, ThenIter>
where
    St: Stream<Item = (Key, Val)>,
    Key: Eq + Hash,
    Hasher: BuildHasher,
    AggFn: FnMut(&mut Val, Val),
    ThenFn: FnOnce(&'a mut HashMap<Key, Val, Hasher>) -> ThenIter,
    ThenIter: Iterator,
{
    pub fn new(
        stream: St,
        state: &'a mut HashMap<Key, Val, Hasher>,
        agg_fn: AggFn,
        then_fn: ThenFn,
    ) -> Self {
        Self {
            state: ReduceKeyedThenState::Reducing {
                stream,
                state,
                agg_fn,
                then_fn,
            },
        }
    }
}

impl<'a, St, Key, Val, Hasher, AggFn, ThenFn, ThenIter> Stream
    for ReduceKeyedThen<'a, St, Key, Val, Hasher, AggFn, ThenFn, ThenIter>
where
    St: Stream<Item = (Key, Val)>,
    Key: Eq + Hash,
    Hasher: BuildHasher,
    AggFn: FnMut(&mut Val, Val),
    ThenFn: FnOnce(&'a mut HashMap<Key, Val, Hasher>) -> ThenIter,
    ThenIter: Iterator,
{
    type Item = ThenIter::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.as_mut().project();

        match this.state.as_mut().project() {
            ReduceKeyedThenStateProj::Reducing {
                mut stream,
                state,
                agg_fn,
                then_fn: _,
            } => loop {
                if let Some((key, item)) = ready!(stream.as_mut().poll_next(cx)) {
                    match state.entry(key) {
                        Entry::Occupied(occupied_entry) => {
                            let val = occupied_entry.into_mut();
                            let () = (agg_fn)(val, item);
                        },
                        Entry::Vacant(vacant_entry) => {
                            vacant_entry.insert(item);
                        },
                    }
                } else {
                    let ReduceKeyedThenStateProjOwn::Reducing {
                        stream: _,
                        state,
                        agg_fn: _,
                        then_fn,
                    } = this
                        .state
                        .as_mut()
                        .project_replace(ReduceKeyedThenState::Empty)
                    else {
                        unreachable!();
                    };
                    let iter = (then_fn)(state);
                    this.state.set(ReduceKeyedThenState::Emitting { iter });
                    return self.poll_next(cx);
                }
            },
            ReduceKeyedThenStateProj::Emitting { iter } => {
                if let Some(item) = iter.next() {
                    Poll::Ready(Some(item))
                } else {
                    this.state.set(ReduceKeyedThenState::Empty);
                    Poll::Ready(None)
                }
            }
            ReduceKeyedThenStateProj::Empty => Poll::Ready(None),
        }
    }
}
