use std::collections::HashSet;
use std::fmt::{Debug, Display};
use std::hash::Hash;

use serde::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize, Debug)]
pub struct Message {
    pub edges: HashSet<(u32, u32)>,
}
impl Message {
    pub fn new() -> Self {
        Message {
            edges: HashSet::new(),
        }
    }
}

// SimplePath is a path that is either acyclic, or ends in a loop.
// Once it has a loop, it stops growing (and is detected as a cycle).
#[derive(PartialEq, Eq, Clone, Serialize, Deserialize, Debug, Default)]
pub struct SimplePath<T>
where
    T: Eq + Hash + Clone + Ord + Display + Debug + Copy,
{
    nodes: HashSet<T>,
    visited: Vec<T>,
    loop_end: Option<T>,
}

impl<T> SimplePath<T>
where
    T: Eq + Hash + Clone + Ord + Display + Debug + Copy,
{
    pub fn new(items: Vec<T>) -> SimplePath<T> {
        let mut s = Self {
            nodes: HashSet::new(),
            visited: vec![],
            loop_end: None,
        };
        for i in items {
            s.push(i)
        }
        s
    }
    pub fn cycle(&self) -> bool {
        self.loop_end.is_some()
    }

    pub fn push(&mut self, item: T) {
        if self.cycle() {
            // NOOP
        } else if self.nodes.contains(&item) {
            self.loop_end = Some(item);
        } else {
            self.visited.push(item);
            self.nodes.insert(item);
        }
    }

    pub fn ordered_from(&self, from: T) -> Vec<T> {
        let mut slices = self.visited.split(|i| *i == from);
        let tail = slices.next().unwrap();
        let head = slices.next().unwrap();

        let mut retval = vec![from];
        retval.extend_from_slice(head);
        retval.extend_from_slice(tail);
        retval
    }

    pub fn canonical(&self) -> Vec<T> {
        if self.cycle() {
            self.ordered_from(*self.visited.iter().min().unwrap())
        } else {
            self.visited.clone()
        }
    }
}
