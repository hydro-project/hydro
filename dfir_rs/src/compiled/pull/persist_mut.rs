use std::hash::Hash;
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use futures::stream::Stream;
use pin_project_lite::pin_project;

use crate::util::Persistence;
use crate::util::sparse_vec::{SparseVec, SparseVecIter};

pin_project! {
    #[project = PersistMutStateProj]
    #[project_replace = PersistMutStateProjOwn]
    enum PersistMutState<'ctx, St, Item> {
        Build {
            #[pin]
            stream: St,
            vec: &'ctx mut SparseVec<Item>,
        },
        Play {
            iter: SparseVecIter<'ctx, Item>,
        },
        Empty
    }
}

pin_project! {
    /// Special stream for the `persist_mut` operator
    #[must_use = "streams do nothing unless polled"]
    pub struct PersistMut<'ctx, St, Item> {
        #[pin]
        state: PersistMutState<'ctx, St, Item>,
    }
}

impl<'ctx, St, Item> PersistMut<'ctx, St, Item>
where
    St: Stream<Item = Persistence<Item>>,
    Item: Clone + Eq + Hash,
{
    /// Create with the preceding sink and given replay.
    pub fn new(stream: St, vec: &'ctx mut SparseVec<Item>, is_first_run_this_tick: bool) -> Self {
        let state = if is_first_run_this_tick {
            PersistMutState::Build { stream, vec }
        } else {
            PersistMutState::Empty
        };
        Self { state }
    }
}

impl<'ctx, St, Item> Stream for PersistMut<'ctx, St, Item>
where
    St: Stream<Item = Persistence<Item>>,
    Item: Clone + Eq + Hash,
{
    type Item = Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.as_mut().project();

        match this.state.as_mut().project() {
            PersistMutStateProj::Build { mut stream, vec } => {
                while let Some(delta) = ready!(stream.as_mut().poll_next(cx)) {
                    match delta {
                        Persistence::Persist(v) => vec.push(v),
                        Persistence::Delete(v) => vec.delete(&v),
                    }
                }
                let PersistMutStateProjOwn::Build { stream: _, vec } =
                    this.state.as_mut().project_replace(PersistMutState::Empty)
                else {
                    unreachable!();
                };
                this.state.set(PersistMutState::Play { iter: vec.iter() });
                self.poll_next(cx)
            }
            PersistMutStateProj::Play { iter } => {
                if let Some(item) = iter.next() {
                    Poll::Ready(Some(item.clone()))
                } else {
                    this.state.set(PersistMutState::Empty);
                    Poll::Ready(None)
                }
            }
            PersistMutStateProj::Empty => Poll::Ready(None),
        }
    }
}
