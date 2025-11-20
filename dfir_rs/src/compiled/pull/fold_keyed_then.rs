use std::{
    collections::HashMap,
    hash::Hash,
    pin::Pin,
    task::{Context, Poll, ready},
};

use futures::Stream;
use pin_project_lite::pin_project;

pin_project! {
    #[project = FoldKeyedThenStateProj]
    enum FoldKeyedThenState<'a, St, Key, Val, InitFn, AggFn, ThenFn, ThenIter> {
        Folding {
            #[pin]
            stream: St,

            state: &'a mut HashMap<Key, Val>,

            init_fn: InitFn,
            agg_fn: AggFn,
            then_fn: ThenFn,
        },
        Emitting {
            iter: ThenIter,
        },
    }
}

pin_project! {
    pub struct FoldKeyedThen<'a, St, Key, Val, InitFn, AggFn, ThenFn, ThenIter> {
        #[pin]
        state: FoldKeyedThenState<'a, St, Key, Val, InitFn, AggFn, ThenFn, ThenIter>
    }
}

impl<'a, St, Item, Key, Val, InitFn, AggFn, ThenFn, ThenIter>
    FoldKeyedThen<'a, St, Key, Val, InitFn, AggFn, ThenFn, ThenIter>
where
    St: Stream<Item = (Key, Item)>,
    Key: Eq + Hash,
    InitFn: FnMut() -> Val,
    AggFn: FnMut(&mut Val, Item),
    ThenFn: FnMut(&mut HashMap<Key, Val>) -> ThenIter,
    ThenIter: Iterator,
{
    pub fn new(
        stream: St,
        state: &'a mut HashMap<Key, Val>,
        init_fn: InitFn,
        agg_fn: AggFn,
        then_fn: ThenFn,
    ) -> Self {
        Self {
            state: FoldKeyedThenState::Folding {
                stream,
                state,
                init_fn,
                agg_fn,
                then_fn,
            },
        }
    }
}

impl<'a, St, Item, Key, Val, InitFn, AggFn, ThenFn, ThenIter> Stream
    for FoldKeyedThen<'a, St, Key, Val, InitFn, AggFn, ThenFn, ThenIter>
where
    St: Stream<Item = (Key, Item)>,
    Key: Eq + Hash,
    InitFn: FnMut() -> Val,
    AggFn: FnMut(&mut Val, Item),
    ThenFn: FnMut(&mut HashMap<Key, Val>) -> ThenIter,
    ThenIter: Iterator,
{
    type Item = ThenIter::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.as_mut().project();

        match this.state.as_mut().project() {
            FoldKeyedThenStateProj::Folding {
                mut stream,
                state,
                init_fn,
                agg_fn,
                then_fn,
            } => loop {
                if let Some((key, item)) = ready!(stream.as_mut().poll_next(cx)) {
                    let val = state.entry(key).or_insert_with(&mut *init_fn);
                    (agg_fn)(val, item);
                } else {
                    let iter = (then_fn)(state);
                    this.state.set(FoldKeyedThenState::Emitting { iter });
                    return self.poll_next(cx);
                }
            },
            FoldKeyedThenStateProj::Emitting { iter } => Poll::Ready(iter.next()),
        }
    }
}
