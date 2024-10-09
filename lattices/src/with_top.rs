use std::cmp::Ordering::{self, *};

use crate::{Atomize, DeepReveal, IsBot, IsTop, LatticeFrom, LatticeOrd, Merge};

/// Adds a new "top" value to the nested lattice type.
///
/// Given an existing lattice, wrap it into a new lattice with a new top element. The new top
/// element compares as less than all the values of the wrapped lattice.  This can be used for
/// giving a sensible default/bottom element to lattices that don't necessarily have one.
///
/// The implementation wraps an [`Option`], with [`None`] representing the top element.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WithTop<Inner>(Option<Inner>);
impl<Inner> WithTop<Inner> {
    /// Create a new `WithTop` lattice instance from a value.
    pub fn new(val: Option<Inner>) -> Self {
        Self(val)
    }

    /// Create a new `WithTop` lattice instance from a value using `Into`.
    pub fn new_from(val: impl Into<Option<Inner>>) -> Self {
        Self::new(val.into())
    }

    /// Reveal the inner value as a shared reference.
    pub fn as_reveal_ref(&self) -> Option<&Inner> {
        self.0.as_ref()
    }

    /// Reveal the inner value as an exclusive reference.
    pub fn as_reveal_mut(&mut self) -> Option<&mut Inner> {
        self.0.as_mut()
    }

    /// Gets the inner by value, consuming self.
    pub fn into_reveal(self) -> Option<Inner> {
        self.0
    }
}

// Use inner's default rather than `None` (which is top, not bot).
impl<Inner> Default for WithTop<Inner>
where
    Inner: Default,
{
    fn default() -> Self {
        Self(Some(Inner::default()))
    }
}

impl<Inner> DeepReveal for WithTop<Inner>
where
    Inner: DeepReveal,
{
    type Revealed = Option<Inner::Revealed>;

    fn deep_reveal(self) -> Self::Revealed {
        self.0.map(DeepReveal::deep_reveal)
    }
}

impl<Inner, Other> Merge<WithTop<Other>> for WithTop<Inner>
where
    Inner: Merge<Other> + LatticeFrom<Other>,
{
    fn merge(&mut self, other: WithTop<Other>) -> bool {
        match (&mut self.0, other.0) {
            (None, None) => false,
            (this @ Some(_), None) => {
                *this = None;
                true
            }
            (None, Some(_)) => false,
            (Some(self_inner), Some(other_inner)) => self_inner.merge(other_inner),
        }
    }
}

impl<Inner, Other> LatticeFrom<WithTop<Other>> for WithTop<Inner>
where
    Inner: LatticeFrom<Other>,
{
    fn lattice_from(other: WithTop<Other>) -> Self {
        Self(other.0.map(Inner::lattice_from))
    }
}

impl<Inner, Other> PartialOrd<WithTop<Other>> for WithTop<Inner>
where
    Inner: PartialOrd<Other>,
{
    fn partial_cmp(&self, other: &WithTop<Other>) -> Option<Ordering> {
        match (&self.0, &other.0) {
            (None, None) => Some(Equal),
            (None, Some(_)) => Some(Greater),
            (Some(_), None) => Some(Less),
            (Some(this_inner), Some(other_inner)) => this_inner.partial_cmp(other_inner),
        }
    }
}
impl<Inner, Other> LatticeOrd<WithTop<Other>> for WithTop<Inner> where
    Self: PartialOrd<WithTop<Other>>
{
}

impl<Inner, Other> PartialEq<WithTop<Other>> for WithTop<Inner>
where
    Inner: PartialEq<Other>,
{
    fn eq(&self, other: &WithTop<Other>) -> bool {
        match (&self.0, &other.0) {
            (None, None) => true,
            (None, Some(_)) => false,
            (Some(_), None) => false,
            (Some(this_inner), Some(other_inner)) => this_inner == other_inner,
        }
    }
}

impl<Inner> IsBot for WithTop<Inner>
where
    Inner: IsBot,
{
    fn is_bot(&self) -> bool {
        self.0.as_ref().map_or(false, IsBot::is_bot)
    }
}

impl<Inner> IsTop for WithTop<Inner>
where
    Inner: IsTop,
{
    fn is_top(&self) -> bool {
        self.0.as_ref().map_or(true, IsTop::is_top)
    }
}

impl<Inner> Atomize for WithTop<Inner>
where
    Inner: Atomize + LatticeFrom<<Inner as Atomize>::Atom>,
{
    type Atom = WithTop<Inner::Atom>;

    // TODO: use impl trait, then remove 'static.
    type AtomIter = Box<dyn Iterator<Item = Self::Atom>>;

    fn atomize(self) -> Self::AtomIter {
        match self.0 {
            Some(inner) => Box::new(inner.atomize().map(WithTop::new_from)),
            None => Box::new(std::iter::once(WithTop::new(None))),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::set_union::{SetUnionHashSet, SetUnionSingletonSet};
    use crate::test::{check_all, check_atomize_each, check_lattice_is_top};

    #[test]
    fn test_singly_nested_singleton_example() {
        let mut my_hash_set = WithTop::new_from(SetUnionHashSet::<&str>::default());
        let my_delta_set = WithTop::new_from(SetUnionSingletonSet::new_from("hello world"));

        assert!(my_hash_set.merge(my_delta_set)); // Changes
        assert!(!my_hash_set.merge(my_delta_set)); // No changes
    }

    #[test]
    fn test_doubly_nested_singleton_example() {
        let mut my_hash_set =
            WithTop::new_from(WithTop::new_from(SetUnionHashSet::<&str>::default()));
        let my_delta_set = WithTop::new_from(WithTop::new_from(SetUnionSingletonSet::new_from(
            "hello world",
        )));

        assert!(my_hash_set.merge(my_delta_set)); // Changes
        assert!(!my_hash_set.merge(my_delta_set)); // No changes
    }

    #[test]
    #[rustfmt::skip]
    fn auto_derives() {
        type B = WithTop<SetUnionHashSet<usize>>;

        assert_eq!(B::new(None).partial_cmp(&B::new(None)), Some(Equal));
        assert_eq!(B::new_from(SetUnionHashSet::new_from([])).partial_cmp(&B::new(None)), Some(Less));
        assert_eq!(B::new(None).partial_cmp(&B::new_from(SetUnionHashSet::new_from([]))), Some(Greater));
        assert_eq!(B::new_from(SetUnionHashSet::new_from([])).partial_cmp(&B::new_from(SetUnionHashSet::new_from([]))), Some(Equal));
        assert_eq!(B::new_from(SetUnionHashSet::new_from([0])).partial_cmp(&B::new_from(SetUnionHashSet::new_from([]))), Some(Greater));
        assert_eq!(B::new_from(SetUnionHashSet::new_from([])).partial_cmp(&B::new_from(SetUnionHashSet::new_from([0]))), Some(Less));
        assert_eq!(B::new_from(SetUnionHashSet::new_from([0])).partial_cmp(&B::new_from(SetUnionHashSet::new_from([1]))), None);

        assert!(B::new(None).eq(&B::new(None)));
        assert!(!B::new_from(SetUnionHashSet::new_from([])).eq(&B::new(None)));
        assert!(!B::new(None).eq(&B::new_from(SetUnionHashSet::new_from([]))));
        assert!(B::new_from(SetUnionHashSet::new_from([])).eq(&B::new_from(SetUnionHashSet::new_from([]))));
        assert!(!B::new_from(SetUnionHashSet::new_from([0])).eq(&B::new_from(SetUnionHashSet::new_from([]))));
        assert!(!B::new_from(SetUnionHashSet::new_from([])).eq(&B::new_from(SetUnionHashSet::new_from([0]))));
        assert!(!B::new_from(SetUnionHashSet::new_from([0])).eq(&B::new_from(SetUnionHashSet::new_from([1]))));
    }

    #[test]
    fn consistency() {
        let items = &[
            WithTop::new(None),
            WithTop::new_from(SetUnionHashSet::new_from([])),
            WithTop::new_from(SetUnionHashSet::new_from([0])),
            WithTop::new_from(SetUnionHashSet::new_from([1])),
            WithTop::new_from(SetUnionHashSet::new_from([0, 1])),
        ];
        check_all(items);
        check_lattice_is_top(items);
    }

    #[test]
    fn atomize() {
        check_atomize_each(&[
            WithTop::new(None),
            WithTop::new_from(SetUnionHashSet::new_from([])),
            WithTop::new_from(SetUnionHashSet::new_from([0])),
            WithTop::new_from(SetUnionHashSet::new_from([1])),
            WithTop::new_from(SetUnionHashSet::new_from([0, 1])),
            WithTop::new_from(SetUnionHashSet::new((0..10).collect())),
        ]);
    }
}
