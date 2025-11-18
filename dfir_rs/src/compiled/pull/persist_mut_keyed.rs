use std::hash::Hash;
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use futures::stream::Stream;
use pin_project_lite::pin_project;
use rustc_hash::FxHashMap;

use crate::util::PersistenceKeyed;
use crate::util::sparse_vec::{SparseVec, SparseVecIter};

pin_project! {
    #[project = PersistMutKeyedProj]
    #[project_replace = PersistMutKeyedProjOwn]
    /// Special stream for the `persist_mut_keyed` operator
    #[must_use = "streams do nothing unless polled"]
    pub enum PersistMutKeyed<'ctx, St, Key, Item> {
        Build {
            #[pin]
            stream: St,
            map: &'ctx mut FxHashMap<Key, SparseVec<Item>>,
        },
        Play {
            iter: Iter<'ctx, Key, Item>,
        },
        Empty
    }
}

impl<'ctx, St, Key, Item> PersistMutKeyed<'ctx, St, Key, Item>
where
    St: Stream<Item = PersistenceKeyed<Key, Item>>,
    Key: Clone + Eq + Hash,
    Item: Clone + Eq + Hash,
{
    /// Create with the preceding sink and given replay.
    pub fn new(
        stream: St,
        map: &'ctx mut FxHashMap<Key, SparseVec<Item>>,
        is_first_run_this_tick: bool,
    ) -> Self {
        if is_first_run_this_tick {
            Self::Build { stream, map }
        } else {
            Self::Empty
        }
    }
}

impl<'ctx, St, Key, Item> Stream for PersistMutKeyed<'ctx, St, Key, Item>
where
    St: Stream<Item = PersistenceKeyed<Key, Item>>,
    Key: Clone + Eq + Hash,
    Item: Clone + Eq + Hash,
{
    type Item = (Key, Item);

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.as_mut().project();
        match this {
            PersistMutKeyedProj::Build { mut stream, map } => {
                while let Some(delta) = ready!(stream.as_mut().poll_next(cx)) {
                    match delta {
                        PersistenceKeyed::Persist(k, v) => {
                            map.entry(k).or_default().push(v);
                        }
                        PersistenceKeyed::Delete(k) => {
                            map.remove(&k);
                        }
                    }
                }
                let PersistMutKeyedProjOwn::Build { stream: _, map } =
                    self.as_mut().project_replace(PersistMutKeyed::Empty)
                else {
                    unreachable!();
                };
                self.as_mut().set(PersistMutKeyed::Play {
                    iter: Iter::new(map),
                });
                self.poll_next(cx)
            }
            PersistMutKeyedProj::Play { iter } => {
                if let Some((k, v)) = iter.next() {
                    Poll::Ready(Some((k.clone(), v.clone())))
                } else {
                    self.set(PersistMutKeyed::Empty);
                    Poll::Ready(None)
                }
            }
            PersistMutKeyedProj::Empty => Poll::Ready(None),
        }
    }
}

struct Iter<'ctx, Key, Item> {
    iters: Option<(
        std::collections::hash_map::Iter<'ctx, Key, SparseVec<Item>>,
        &'ctx Key,
        SparseVecIter<'ctx, Item>,
    )>,
}

impl<'ctx, Key, Item> Iter<'ctx, Key, Item>
where
    Key: Clone + Eq + Hash,
    Item: Clone + Eq + Hash,
{
    fn new(map: &'ctx FxHashMap<Key, SparseVec<Item>>) -> Self {
        let mut map_iter = map.iter();
        let iters = map_iter.next().map(|(k, v)| (map_iter, k, v.iter()));
        Self { iters }
    }
}

impl<'ctx, Key, Item> Iterator for Iter<'ctx, Key, Item>
where
    Key: Clone + Eq + Hash,
    Item: Clone + Eq + Hash,
{
    type Item = (&'ctx Key, &'ctx Item);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (map_iter, key, val_iter) = self.iters.as_mut()?;
            if let Some(item) = val_iter.next() {
                return Some((key, item));
            } else if let Some((k, v)) = map_iter.next() {
                *key = k;
                *val_iter = v.iter();
            } else {
                self.iters = None;
            }
        }
    }
}
