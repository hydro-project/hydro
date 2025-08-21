mod serialization;

use std::cell::{Ref, RefCell, RefMut};
use std::rc::{Rc, Weak};

pub use serialization::AutoReturnBufferDeserializer;

/// It is slow to allocate and, in particular, free blocks of memory.
/// If we know ahead of time what size buffers we will need then we can pre-allocate a bunch of them and
/// reuse those allocations.
/// To comply with rust's type system, these buffers have shared owernship and interior mutability, so Rc<RefCell>
/// Eventually these buffers can be made more dynamic by using scatter-gather I/O and a buffer-of-buffers pattern.
/// We are not there yet and so this is a simple approximation of what it will eventually look like.
#[derive(Debug)]
pub struct BufferPool<const SIZE: usize> {
    buffers: Vec<Rc<RefCell<[u8; SIZE]>>>,
}

/// This buffer will be returned to it's owning buffer pool when it is dropped.
/// In order to return it, it has to know where it is being returned to, this is the [collector] member.
/// The collector pointer is weak because otherwise there would be a cycle between BufferPool and AutoReturnBufferInner
/// and an AutoReturnBufferInner does not logically have any kind of ownership over the BufferPool shared pool.
#[derive(Clone, Debug)]
pub struct AutoReturnBuffer<const SIZE: usize> {
    pub collector: Weak<RefCell<BufferPool<SIZE>>>,
    pub inner: Rc<RefCell<[u8; SIZE]>>,
}

impl<const SIZE: usize> BufferPool<SIZE> {
    pub fn create_buffer_pool() -> Rc<RefCell<BufferPool<SIZE>>> {
        Rc::new(RefCell::new(BufferPool {
            buffers: Vec::new(),
        }))
    }

    pub fn get_from_buffer_pool(pool: &Rc<RefCell<BufferPool<SIZE>>>) -> AutoReturnBuffer<SIZE> {
        let buffer = pool.borrow_mut().buffers.pop();

        if let Some(buffer) = buffer {
            AutoReturnBuffer {
                collector: Rc::downgrade(pool),
                inner: buffer,
            }
        } else {
            AutoReturnBuffer {
                collector: Rc::downgrade(pool),
                inner: Rc::new(RefCell::new([0; SIZE])),
            }
        }
    }
}

impl<const SIZE: usize> Drop for AutoReturnBuffer<SIZE> {
    fn drop(&mut self) {
        if Rc::strong_count(&self.inner) == 1 {
            // This is the last one, give the buffer back to the collection.
            if let Some(pool) = self.collector.upgrade() {
                pool.borrow_mut().buffers.push(self.inner.clone());
            }

            // If the upgrade fails then the buffer pool has been freed and so there
            // is nothing to return this buffer to, just drop it instead.
        }
    }
}

impl<const SIZE: usize> AutoReturnBuffer<SIZE> {
    pub fn borrow_mut(&self) -> RefMut<'_, [u8; SIZE]> {
        self.inner.borrow_mut()
    }

    pub fn borrow(&self) -> Ref<'_, [u8; SIZE]> {
        self.inner.borrow()
    }
}

impl<const SIZE: usize> PartialEq for AutoReturnBuffer<SIZE> {
    fn eq(&self, other: &Self) -> bool {
        *self.inner.borrow() == *other.inner.borrow()
    }
}
impl<const SIZE: usize> Eq for AutoReturnBuffer<SIZE> {}
