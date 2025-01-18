//! A priority queue where the priorities are indices (small non-negative integers).

use std::collections::VecDeque;

/// A priority queue where the priorities are indices (small non-negative integers).
///
/// Larger indices are higher priority.
pub struct IndexedQueue<T> {
    /// Invariant: the last `VecDeque` is non-empty.
    queues: Vec<VecDeque<T>>,
}
impl<T> Default for IndexedQueue<T> {
    fn default() -> Self {
        Self { queues: Vec::new() }
    }
}
impl<T> IndexedQueue<T> {
    /// Creates a new `IndexedQueue`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Pushes an element to the queue with the given index.
    pub fn push(&mut self, index: usize, element: T) {
        if index >= self.queues.len() {
            self.queues.resize_with(index + 1, Default::default);
        }
        self.queues[index].push_back(element);
    }

    /// Pops an element from the queue with the largest index.
    pub fn pop(&mut self) -> Option<(usize, T)> {
        let item = self.queues.last_mut()?.pop_front().unwrap();
        let index = self.queues.len() - 1;
        // Remove trailing empty queues to maintain the invariant.
        while self.queues.last().is_some_and(VecDeque::is_empty) {
            self.queues.pop();
        }
        Some((index, item))
    }

    /// Returns the number of elements in the `IndexedQueue`.
    pub fn len(&self) -> usize {
        self.queues.iter().map(VecDeque::len).sum()
    }

    /// Returns `true` if the `IndexedQueue` is empty.
    pub fn is_empty(&self) -> bool {
        self.queues.is_empty()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_indexed_queue() {
        let mut queue = IndexedQueue::new();
        assert_eq!(queue.len(), 0);
        assert!(queue.is_empty());

        queue.push(0, 0);
        queue.push(1, 1);
        queue.push(0, 2);
        queue.push(5, 3);
        queue.push(1, 4);

        assert_eq!(queue.len(), 5);
        assert!(!queue.is_empty());

        assert_eq!(queue.pop(), Some((5, 3)));

        assert_eq!(queue.len(), 4);
        assert!(!queue.is_empty());

        assert_eq!(queue.pop(), Some((1, 1)));
        assert_eq!(queue.pop(), Some((1, 4)));
        assert_eq!(queue.pop(), Some((0, 0)));

        assert_eq!(queue.len(), 1);
        assert!(!queue.is_empty());

        assert_eq!(queue.pop(), Some((0, 2)));
        assert_eq!(queue.pop(), None);

        assert_eq!(queue.len(), 0);
        assert!(queue.is_empty());
    }
}
