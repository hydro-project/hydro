use std::hash::Hash;
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use futures::stream::Stream;
use pin_project_lite::pin_project;

use crate::util::Persistence;
use crate::util::sparse_vec::{SparseVec, SparseVecIter};

pin_project! {
    #[project = PersistMutProj]
    #[project_replace = PersistMutProjOwn]
    /// Special stream for the `persist_mut` operator
    #[must_use = "streams do nothing unless polled"]
    pub enum PersistMut<'ctx, St, Item> {
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

impl<'ctx, St, Item> PersistMut<'ctx, St, Item>
where
    St: Stream<Item = Persistence<Item>>,
    Item: Clone + Eq + Hash,
{
    /// Create with the preceding sink and given replay.
    pub fn new(stream: St, vec: &'ctx mut SparseVec<Item>, is_first_run_this_tick: bool) -> Self {
        if is_first_run_this_tick {
            Self::Build { stream, vec }
        } else {
            Self::Empty
        }
    }
}

impl<'ctx, St, Item> Stream for PersistMut<'ctx, St, Item>
where
    St: Stream<Item = Persistence<Item>>,
    Item: Clone + Eq + Hash,
{
    type Item = Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.as_mut().project();
        match this {
            PersistMutProj::Build { mut stream, vec } => {
                while let Some(delta) = ready!(stream.as_mut().poll_next(cx)) {
                    match delta {
                        Persistence::Persist(v) => vec.push(v),
                        Persistence::Delete(v) => vec.delete(&v),
                    }
                }
                let PersistMutProjOwn::Build { stream: _, vec } =
                    self.as_mut().project_replace(PersistMut::Empty)
                else {
                    unreachable!();
                };
                self.as_mut().set(PersistMut::Play { iter: vec.iter() });
                self.poll_next(cx)
            }
            PersistMutProj::Play { iter } => {
                if let Some(item) = iter.next() {
                    Poll::Ready(Some(item.clone()))
                } else {
                    self.set(PersistMut::Empty);
                    Poll::Ready(None)
                }
            }
            PersistMutProj::Empty => Poll::Ready(None),
        }
    }
}
