//! Pair compound lattice.
//!
//! Merge both nested lattices.

use std::cmp::Ordering::{self, *};

use super::{ConvertFrom, Merge};
use crate::LatticeOrd;

/// Pair lattice.
///
/// `LatA` and `LatB` specify the nested lattice types.
#[derive(Copy, Clone, Debug, Default, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Pair<LatA, LatB> {
    a: LatA,
    b: LatB,
}

impl<LatA, LatB> Pair<LatA, LatB> {
    /// Create a `Pair` from the given values.
    pub fn new(a: LatA, b: LatB) -> Self {
        Self { a, b }
    }

    /// Create a `Pair` from the given values, using `Into`.
    pub fn new_from(a: impl Into<LatA>, b: impl Into<LatB>) -> Self {
        Self::new(a.into(), b.into())
    }
}

impl<LatASelf, LatAOther, LatBSelf, LatBOther> Merge<Pair<LatAOther, LatBOther>>
    for Pair<LatASelf, LatBSelf>
where
    LatASelf: Merge<LatAOther>,
    LatBSelf: Merge<LatBOther>,
{
    fn merge(&mut self, other: Pair<LatAOther, LatBOther>) -> bool {
        // Do NOT use short-circuiting `||`.
        self.a.merge(other.a) | self.b.merge(other.b)
    }
}

impl<LatASelf, LatAOther, LatBSelf, LatBOther> ConvertFrom<Pair<LatAOther, LatBOther>>
    for Pair<LatASelf, LatBSelf>
where
    LatASelf: ConvertFrom<LatAOther>,
    LatBSelf: ConvertFrom<LatBOther>,
{
    fn from(other: Pair<LatAOther, LatBOther>) -> Self {
        Self {
            a: ConvertFrom::from(other.a),
            b: ConvertFrom::from(other.b),
        }
    }
}

impl<LatASelf, LatAOther, LatBSelf, LatBOther> PartialOrd<Pair<LatAOther, LatBOther>>
    for Pair<LatASelf, LatBSelf>
where
    LatASelf: PartialOrd<LatAOther>,
    LatBSelf: PartialOrd<LatBOther>,
{
    fn partial_cmp(&self, other: &Pair<LatAOther, LatBOther>) -> Option<Ordering> {
        let ord_a = self.a.partial_cmp(&other.a);
        let ord_b = self.b.partial_cmp(&other.b);
        if ord_a == ord_b {
            ord_a
        } else {
            match (ord_a, ord_b) {
                (ord_a, Some(Equal)) => ord_a,
                (Some(Equal), ord_b) => ord_b,
                _conflicting => None,
            }
        }
    }
}
impl<LatASelf, LatAOther, LatBSelf, LatBOther> LatticeOrd<Pair<LatAOther, LatBOther>>
    for Pair<LatASelf, LatBSelf>
where
    Self: PartialOrd<Pair<LatAOther, LatBOther>>,
{
}

impl<LatASelf, LatAOther, LatBSelf, LatBOther> PartialEq<Pair<LatAOther, LatBOther>>
    for Pair<LatASelf, LatBSelf>
where
    LatASelf: PartialEq<LatAOther>,
    LatBSelf: PartialEq<LatBOther>,
{
    fn eq(&self, other: &Pair<LatAOther, LatBOther>) -> bool {
        self.a == other.a && self.b == other.b
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashSet;

    use super::*;
    use crate::set_union::SetUnionHashSet;
    use crate::test::{assert_lattice_identities, assert_partial_ord_identities};

    #[test]
    fn consistency() {
        let mut test_vec = Vec::new();

        for a in [vec![], vec![0], vec![1], vec![0, 1]] {
            for b in [vec![], vec![0], vec![1], vec![0, 1]] {
                test_vec.push(Pair::new(
                    SetUnionHashSet::new_from(HashSet::from_iter(a.clone())),
                    SetUnionHashSet::new_from(HashSet::from_iter(b.clone())),
                ));
            }
        }

        assert_partial_ord_identities(&test_vec);
        assert_lattice_identities(&test_vec);
    }
}
