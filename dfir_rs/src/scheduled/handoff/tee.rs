//! Module for teeing handoffs, not currently used much.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use super::{CanReceive, Handoff, HandoffMeta};

struct ReaderHandoff<T> {
    contents: VecDeque<Vec<T>>,
}

impl<T> Default for ReaderHandoff<T> {
    fn default() -> Self {
        Self {
            contents: Default::default(),
        }
    }
}

struct TeeingHandoffInternal<T> {
    /// (is alive, reader)
    readers: Vec<(bool, ReaderHandoff<T>)>,
}

/// A [Handoff] which is part of a "family" of handoffs. Writing to this handoff
/// will write to every reader. New readers can be created by calling `tee`.
#[derive(Clone)]
pub struct TeeingHandoff<T>
where
    T: 'static,
{
    read_from: usize,
    internal: Rc<RefCell<TeeingHandoffInternal<T>>>,
}

impl<T> Default for TeeingHandoff<T> {
    fn default() -> Self {
        TeeingHandoff {
            read_from: 0,
            internal: Rc::new(RefCell::new(TeeingHandoffInternal {
                readers: vec![(true, ReaderHandoff::<T>::default())],
            })),
        }
    }
}

impl<T> TeeingHandoff<T>
where
    T: Clone,
{
    /// Tee the internal shared datastructure to create a new tee output.
    #[must_use]
    pub(crate) fn tee(&self) -> Self {
        let id = (*self.internal).borrow().readers.len();
        (*self.internal)
            .borrow_mut()
            .readers
            .push((true, ReaderHandoff::default()));
        Self {
            read_from: id,
            internal: self.internal.clone(),
        }
    }

    /// Mark this particular teeing handoff output as dead, so no more data will be written to it.
    pub(crate) fn drop(&self) {
        self.internal.borrow_mut().readers[self.read_from].0 = false;
    }
}

impl<T> HandoffMeta for TeeingHandoff<T> {
    /// If this output's buffer is empty, return true.
    fn is_bottom(&self) -> bool {
        self.internal.borrow().readers[self.read_from]
            .1
            .contents
            .iter()
            .all(Vec::is_empty)
    }
}

impl<T> Handoff for TeeingHandoff<T> {
    type Inner = VecDeque<Vec<T>>;

    fn take_inner(&self) -> Self::Inner {
        std::mem::take(
            &mut (*self.internal).borrow_mut().readers[self.read_from]
                .1
                .contents,
        )
    }

    fn borrow_mut_swap(&self) -> std::cell::RefMut<Self::Inner> {
        todo!()
    }
}

impl<T> CanReceive<Vec<T>> for TeeingHandoff<T>
where
    T: Clone,
{
    fn give(&self, vec: Vec<T>) -> Vec<T> {
        let readers = &mut (*self.internal).borrow_mut().readers;
        if let Some((last, rest)) = readers.split_last_mut() {
            for reader in rest {
                if reader.0 {
                    reader.1.contents.push_back(vec.clone());
                }
            }
            if last.0 {
                last.1.contents.push_back(vec);
            }
        }
        Vec::new()
    }
}
