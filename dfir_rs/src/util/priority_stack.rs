//! A priority queue in which elements of the same priority are popped in a LIFO order.

/// A priority stack in which elements of the same priority are popped in a LIFO order.
#[derive(Debug, Clone)]
pub struct PriorityStack<T> {
    /// Note: inner stack `Vec`s may be empty.
    stacks: Vec<Vec<T>>,
}

impl<T> PriorityStack<T> {
    /// Creates a new, empty `PriorityStack`.
    pub fn new() -> Self {
        Self {
            stacks: Vec::default(),
        }
    }

    /// Pushes an element onto the stack with the given priority.
    pub fn push(&mut self, priority: usize, item: T) {
        if priority >= self.stacks.len() {
            self.stacks.resize_with(priority + 1, Default::default);
        }
        self.stacks[priority].push(item);
    }

    /// Pops an element from the stack with the highest priority.
    pub fn pop(&mut self) -> Option<T> {
        self.stacks.iter_mut().rev().filter_map(Vec::pop).next()
    }

    /// Returns the number of elements in the `PriorityStack`.
    pub fn len(&self) -> usize {
        self.stacks.iter().map(Vec::len).sum()
    }

    /// Returns true if the `PriorityStack` is empty.
    pub fn is_empty(&self) -> bool {
        self.stacks.is_empty()
    }
}

impl<T> Default for PriorityStack<T> {
    fn default() -> Self {
        Self::new()
    }
}
