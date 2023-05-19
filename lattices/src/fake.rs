use super::{ConvertFrom, Merge};
use crate::LatticeOrd;

/// A fake lattice that will runtime panic if a merge between inequal values is attempted.
///
/// This is used to wrap non lattice data into a lattice in a way that typechecks
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Fake<T>(pub T);
impl<T> Fake<T> {
    /// Create a new `Fake` lattice instance from a value.
    pub fn new(val: T) -> Self {
        Self(val)
    }

    /// Create a new `Fake` lattice instance from a value using `Into`.
    pub fn new_from(val: impl Into<T>) -> Self {
        Self::new(val.into())
    }
}

impl<T, O> Merge<Fake<O>> for Fake<T>
where
    T: PartialEq<O>,
{
    fn merge(&mut self, other: Fake<O>) -> bool {
        if self.0 != other.0 {
            panic!("The fake lattice cannot merge inequal elements.")
        }
        false
    }
}

impl<T> ConvertFrom<Fake<T>> for Fake<T> {
    fn from(other: Fake<T>) -> Self {
        other
    }
}

impl<T, O> PartialOrd<Fake<O>> for Fake<T>
where
    T: PartialEq<O>,
{
    fn partial_cmp(&self, other: &Fake<O>) -> Option<std::cmp::Ordering> {
        if self.0 != other.0 {
            panic!("The fake lattice does not have a partial order between inequal elements.");
        }
        Some(std::cmp::Ordering::Equal)
    }
}
impl<T, O> LatticeOrd<Fake<O>> for Fake<T> where Self: PartialOrd<Fake<O>> {}

impl<T, O> PartialEq<Fake<O>> for Fake<T>
where
    T: PartialEq<O>,
{
    fn eq(&self, other: &Fake<O>) -> bool {
        self.0 == other.0
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::{
        check_all, check_lattice_ord, check_lattice_properties, check_partial_ord_properties,
    };

    #[test]
    fn consistency_equal() {
        check_all(&[Fake::new("hello world")])
    }

    #[test]
    fn consistency_inequal() {
        use std::collections::BTreeSet;

        let items = [
            Fake::new(BTreeSet::from_iter([])),
            Fake::new(BTreeSet::from_iter([0])),
            Fake::new(BTreeSet::from_iter([1])),
            Fake::new(BTreeSet::from_iter([0, 1])),
        ];

        // Merged inequal elements panic, therefore `NaiveMerge` panics.
        assert!(std::panic::catch_unwind(|| check_lattice_ord(&items)).is_err());
        // Fake does not have a partial order.
        assert!(std::panic::catch_unwind(|| check_partial_ord_properties(&items)).is_err());
        // Fake is not actually a lattice.
        assert!(std::panic::catch_unwind(|| check_lattice_properties(&items)).is_err());
    }
}
