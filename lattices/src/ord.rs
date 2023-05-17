//! Totally-ordered lattices, [`Max`] and [`Min`].
//!
//! Uses [std::cmp::Ord`].

use std::cmp::Ordering;

use super::{ConvertFrom, Merge};
use crate::LatticeOrd;

/// A totally ordered max lattice. Merging takes the larger value.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default, PartialOrd, Ord, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Max<T>(pub T);
impl<T> Max<T> {
    /// Create a new `Max` lattice instance from a `T`.
    pub fn new(val: T) -> Self {
        Self(val)
    }

    /// Create a new `Max` lattice instance from an `Into<T>` value.
    pub fn from(val: impl Into<T>) -> Self {
        Self::new(val.into())
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

impl<T> ConvertFrom<Max<T>> for Max<T> {
    fn from(other: Max<T>) -> Self {
        other
    }
}

impl<T> LatticeOrd<Self> for Max<T> where Self: PartialOrd<Self> {}

/// A totally ordered min lattice. Merging takes the smaller value.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Min<T>(pub T);
impl<T> Min<T> {
    /// Create a new `Min` lattice instance from a `T`.
    pub fn new(val: T) -> Self {
        Self(val)
    }

    /// Create a new `Min` lattice instance from an `Into<T>` value.
    pub fn new_from(val: impl Into<T>) -> Self {
        Self::new(val.into())
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

impl<T> ConvertFrom<Min<T>> for Min<T> {
    fn from(other: Min<T>) -> Self {
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
    use crate::test::{assert_lattice_identities, assert_partial_ord_identities};

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
        let test_vec = vec![Max::new(0), Max::new(1)];

        assert_partial_ord_identities(&test_vec);
        assert_lattice_identities(&test_vec);

        let test_vec = vec![Min::new(0), Min::new(1)];

        assert_partial_ord_identities(&test_vec);
        assert_lattice_identities(&test_vec);
    }
}
