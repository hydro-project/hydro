use std::cmp::Ordering::{self, *};

use crate::{Atomize, DeepReveal, IsBot, IsTop, LatticeFrom, LatticeOrd, Merge};

/// Adds a new "bot" value to the nested lattice type.
///
/// Given an existing lattice, wrap it into a new lattice with a new bottom element. The new bottom
/// element compares as less than all the values of the wrapped lattice.  This can be used for
/// giving a sensible default/bottom element to lattices that don't necessarily have one.
///
/// The implementation wraps an [`Option`], with [`None`] representing the bottom element.
#[repr(transparent)]
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WithBot<Inner>(Option<Inner>);
impl<Inner> WithBot<Inner> {
    /// Create a new `WithBot` lattice instance from a value.
    pub fn new(val: Option<Inner>) -> Self {
        Self(val)
    }

    /// Create a new `WithBot` lattice instance from a value using `Into`.
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

// Cannot auto derive because the generated implementation has the wrong trait bounds.
// https://github.com/rust-lang/rust/issues/26925
impl<Inner> Default for WithBot<Inner> {
    fn default() -> Self {
        Self(None)
    }
}

impl<Inner> DeepReveal for WithBot<Inner>
where
    Inner: DeepReveal,
{
    type Revealed = Option<Inner::Revealed>;

    fn deep_reveal(self) -> Self::Revealed {
        self.0.map(DeepReveal::deep_reveal)
    }
}

impl<Inner, Other> Merge<WithBot<Other>> for WithBot<Inner>
where
    Inner: Merge<Other> + LatticeFrom<Other>,
    Other: IsBot,
{
    fn merge(&mut self, other: WithBot<Other>) -> bool {
        match (&mut self.0, other.0) {
            (this @ None, Some(other_inner)) if !other_inner.is_bot() => {
                *this = Some(LatticeFrom::lattice_from(other_inner));
                true
            }
            (Some(self_inner), Some(other_inner)) => self_inner.merge(other_inner),
            (_self, _none_or_bot) => false,
        }
    }
}

impl<Inner, Other> LatticeFrom<WithBot<Other>> for WithBot<Inner>
where
    Inner: LatticeFrom<Other>,
{
    fn lattice_from(other: WithBot<Other>) -> Self {
        Self(other.0.map(Inner::lattice_from))
    }
}

impl<Inner, Other> PartialOrd<WithBot<Other>> for WithBot<Inner>
where
    Inner: PartialOrd<Other> + IsBot,
    Other: IsBot,
{
    fn partial_cmp(&self, other: &WithBot<Other>) -> Option<Ordering> {
        match (&self.0, &other.0) {
            (None, None) => Some(Equal),
            (None, Some(bot)) if bot.is_bot() => Some(Equal),
            (Some(bot), None) if bot.is_bot() => Some(Equal),
            (None, Some(_)) => Some(Less),
            (Some(_), None) => Some(Greater),
            (Some(this_inner), Some(other_inner)) => this_inner.partial_cmp(other_inner),
        }
    }
}
impl<Inner, Other> LatticeOrd<WithBot<Other>> for WithBot<Inner> where
    Self: PartialOrd<WithBot<Other>>
{
}

impl<Inner, Other> PartialEq<WithBot<Other>> for WithBot<Inner>
where
    Inner: PartialEq<Other> + IsBot,
    Other: IsBot,
{
    fn eq(&self, other: &WithBot<Other>) -> bool {
        match (&self.0, &other.0) {
            (None, None) => true,
            (None, Some(bot)) if bot.is_bot() => true,
            (Some(bot), None) if bot.is_bot() => true,
            (None, Some(_)) => false,
            (Some(_), None) => false,
            (Some(this_inner), Some(other_inner)) => this_inner == other_inner,
        }
    }
}
impl<Inner> Eq for WithBot<Inner> where Self: PartialEq {}

impl<Inner> IsBot for WithBot<Inner>
where
    Inner: IsBot,
{
    fn is_bot(&self) -> bool {
        self.0.as_ref().map_or(true, IsBot::is_bot)
    }
}

impl<Inner> IsTop for WithBot<Inner>
where
    Inner: IsTop,
{
    fn is_top(&self) -> bool {
        self.0.as_ref().map_or(false, IsTop::is_top)
    }
}

impl<Inner> Atomize for WithBot<Inner>
where
    Inner: 'static + Atomize + LatticeFrom<<Inner as Atomize>::Atom>,
{
    type Atom = WithBot<Inner::Atom>;

    // TODO: use impl trait, then remove 'static.
    type AtomIter = Box<dyn Iterator<Item = Self::Atom>>;

    fn atomize(self) -> Self::AtomIter {
        Box::new(
            self.0
                .into_iter()
                .flat_map(Atomize::atomize)
                .map(WithBot::new_from),
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::set_union::{SetUnionHashSet, SetUnionSingletonSet};
    use crate::test::{check_all, check_atomize_each};

    #[test]
    fn test_singly_nested_singleton_example() {
        let mut my_hash_set = WithBot::new_from(SetUnionHashSet::<&str>::default());
        let my_delta_set = WithBot::new_from(SetUnionSingletonSet::new_from("hello world"));

        assert!(my_hash_set.merge(my_delta_set)); // Changes
        assert!(!my_hash_set.merge(my_delta_set)); // No changes
    }

    #[test]
    fn test_doubly_nested_singleton_example() {
        let mut my_hash_set =
            WithBot::new_from(WithBot::new_from(SetUnionHashSet::<&str>::default()));
        let my_delta_set = WithBot::new_from(WithBot::new_from(SetUnionSingletonSet::new_from(
            "hello world",
        )));

        assert!(my_hash_set.merge(my_delta_set)); // Changes
        assert!(!my_hash_set.merge(my_delta_set)); // No changes
    }

    #[test]
    #[rustfmt::skip]
    fn auto_derives() {
        type B = WithBot<SetUnionHashSet<usize>>;

        assert_eq!(B::default().partial_cmp(&B::default()), Some(Equal));

        // Test bot collapsing - `WithBot(Some(Bot))` equals `WithBot(None)`.
        assert_eq!(B::new_from(SetUnionHashSet::new_from([])).partial_cmp(&B::default()), Some(Equal));
        assert_eq!(B::default().partial_cmp(&B::new_from(SetUnionHashSet::new_from([]))), Some(Equal));
        assert!(B::new_from(SetUnionHashSet::new_from([])).eq(&B::default()));
        assert!(B::default().eq(&B::new_from(SetUnionHashSet::new_from([]))));

        // PartialOrd
        assert_eq!(B::new_from(SetUnionHashSet::new_from([])).partial_cmp(&B::new_from(SetUnionHashSet::new_from([]))), Some(Equal));
        assert_eq!(B::new_from(SetUnionHashSet::new_from([0])).partial_cmp(&B::new_from(SetUnionHashSet::new_from([]))), Some(Greater));
        assert_eq!(B::new_from(SetUnionHashSet::new_from([])).partial_cmp(&B::new_from(SetUnionHashSet::new_from([0]))), Some(Less));
        assert_eq!(B::new_from(SetUnionHashSet::new_from([0])).partial_cmp(&B::new_from(SetUnionHashSet::new_from([1]))), None);

        // PartialEq
        assert!(B::default().eq(&B::default()));
        assert!(B::new_from(SetUnionHashSet::new_from([])).eq(&B::new_from(SetUnionHashSet::new_from([]))));
        assert!(!B::new_from(SetUnionHashSet::new_from([0])).eq(&B::new_from(SetUnionHashSet::new_from([]))));
        assert!(!B::new_from(SetUnionHashSet::new_from([])).eq(&B::new_from(SetUnionHashSet::new_from([0]))));
        assert!(!B::new_from(SetUnionHashSet::new_from([0])).eq(&B::new_from(SetUnionHashSet::new_from([1]))));
    }

    #[test]
    fn consistency() {
        check_all(&[
            WithBot::default(),
            WithBot::new_from(SetUnionHashSet::new_from([])),
            WithBot::new_from(SetUnionHashSet::new_from([0])),
            WithBot::new_from(SetUnionHashSet::new_from([1])),
            WithBot::new_from(SetUnionHashSet::new_from([0, 1])),
        ])
    }

    #[test]
    fn atomize() {
        check_atomize_each(&[
            WithBot::default(),
            WithBot::new_from(SetUnionHashSet::new_from([])),
            WithBot::new_from(SetUnionHashSet::new_from([0])),
            WithBot::new_from(SetUnionHashSet::new_from([1])),
            WithBot::new_from(SetUnionHashSet::new_from([0, 1])),
            WithBot::new_from(SetUnionHashSet::new((0..10).collect())),
        ]);
    }
}
