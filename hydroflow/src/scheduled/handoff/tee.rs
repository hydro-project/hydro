use std::any::Any;
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
    readers: Vec<ReaderHandoff<T>>,
}

// A [Handoff] which is part of a "family" of handoffs. Writing to this handoff
// will write to every reader. New readers can be created by calling `tee`.
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
                readers: vec![Default::default()],
            })),
        }
    }
}

impl<T> TeeingHandoff<T>
where
    T: Clone,
{
    #[must_use]
    pub fn tee(&self) -> Self {
        let id = (*self.internal).borrow().readers.len();
        (*self.internal)
            .borrow_mut()
            .readers
            .push(ReaderHandoff::default());
        Self {
            read_from: id,
            internal: self.internal.clone(),
        }
    }
}

impl<T> HandoffMeta for TeeingHandoff<T> {
    fn any_ref(&self) -> &dyn Any {
        self
    }

    fn is_bottom(&self) -> bool {
        true
    }
}

impl<T> Handoff for TeeingHandoff<T> {
    type Inner = VecDeque<Vec<T>>;

    fn take_inner(&self) -> Self::Inner {
        std::mem::take(&mut (*self.internal).borrow_mut().readers[self.read_from].contents)
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
        for i in 0..(readers.len() - 1) {
            readers[i].contents.push_back(vec.clone());
        }
        let last = readers.len() - 1;
        readers[last].contents.push_back(vec);
        Vec::new()
    }
}
