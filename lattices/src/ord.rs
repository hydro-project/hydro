use std::cmp::Ordering;

use crate::{LatticeFrom, LatticeOrd, Merge};

/// A totally ordered max lattice. Merging returns the larger value.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default, PartialOrd, Ord, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Max<T>(T);
impl<T> Max<T> {
    /// Create a new `Max` lattice instance from a `T`.
    pub fn new(val: T) -> Self {
        Self(val)
    }

    /// Create a new `Max` lattice instance from an `Into<T>` value.
    pub fn from(val: impl Into<T>) -> Self {
        Self::new(val.into())
    }

    /// Reveal the inner value as a shared reference.
    pub fn as_reveal_ref(&self) -> &T {
        &self.0
    }

    /// Reveal the inner value as an exclusive reference.
    pub fn as_reveal_mut(&mut self) -> &mut T {
        &mut self.0
    }

    /// Gets the inner by value, consuming self.
    pub fn into_reveal(self) -> T {
        self.0
    }
}

impl<T> Merge<Max<T>> for Max<T>
where
    T: Ord,
{
    fn merge(&mut self, other: Max<T>) -> bool {
        if self.0 < other.0 {
            self.0 = other.0;
            true
        } else {
            false
        }
    }
}

impl<T> LatticeFrom<Max<T>> for Max<T> {
    fn lattice_from(other: Max<T>) -> Self {
        other
    }
}

impl<T> LatticeOrd<Self> for Max<T> where Self: PartialOrd<Self> {}

/// A totally ordered min lattice. Merging returns the smaller value.
///
/// This means the lattice order is the reverse of what you might naturally expect: 0 is greater
/// than 1.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Min<T>(T);
impl<T> Min<T> {
    /// Create a new `Min` lattice instance from a `T`.
    pub fn new(val: T) -> Self {
        Self(val)
    }

    /// Create a new `Min` lattice instance from an `Into<T>` value.
    pub fn new_from(val: impl Into<T>) -> Self {
        Self::new(val.into())
    }

    /// Reveal the inner value as a shared reference.
    pub fn as_reveal_ref(&self) -> &T {
        &self.0
    }

    /// Reveal the inner value as an exclusive reference.
    pub fn as_reveal_mut(&mut self) -> &mut T {
        &mut self.0
    }

    /// Gets the inner by value, consuming self.
    pub fn into_reveal(self) -> T {
        self.0
    }
}

impl<T> Merge<Min<T>> for Min<T>
where
    T: Ord,
{
    fn merge(&mut self, other: Min<T>) -> bool {
        if other.0 < self.0 {
            self.0 = other.0;
            true
        } else {
            false
        }
    }
}

impl<T> LatticeFrom<Min<T>> for Min<T> {
    fn lattice_from(other: Min<T>) -> Self {
        other
    }
}

impl<T> PartialOrd for Min<T>
where
    T: PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0).map(Ordering::reverse)
    }
}
impl<T> LatticeOrd<Self> for Min<T> where Self: PartialOrd<Self> {}

impl<T> Ord for Min<T>
where
    T: Ord,
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0).reverse()
    }
}

#[cfg(test)]
mod test {
    use std::cmp::Ordering::*;

    use super::*;
    use crate::test::{check_lattice_ord, check_lattice_properties, check_partial_ord_properties};

    #[test]
    fn ordering() {
        assert_eq!(Max::new(0).cmp(&Max::new(0)), Equal);
        assert_eq!(Max::new(0).cmp(&Max::new(1)), Less);
        assert_eq!(Max::new(1).cmp(&Max::new(0)), Greater);

        assert_eq!(Min::new(0).cmp(&Min::new(0)), Equal);
        assert_eq!(Min::new(0).cmp(&Min::new(1)), Greater);
        assert_eq!(Min::new(1).cmp(&Min::new(0)), Less);
    }

    #[test]
    fn eq() {
        assert!(Max::new(0).eq(&Max::new(0)));
        assert!(!Max::new(0).eq(&Max::new(1)));
        assert!(!Max::new(1).eq(&Max::new(0)));

        assert!(Min::new(0).eq(&Min::new(0)));
        assert!(!Min::new(0).eq(&Min::new(1)));
        assert!(!Min::new(1).eq(&Min::new(0)));
    }

    #[test]
    fn consistency() {
        let items_max = &[Max::new(0), Max::new(1)];
        check_lattice_ord(items_max);
        check_partial_ord_properties(items_max);
        check_lattice_properties(items_max);

        let items_min = &[Min::new(0), Min::new(1)];
        check_lattice_ord(items_min);
        check_partial_ord_properties(items_min);
        check_lattice_properties(items_min);
    }
}
