use std::cmp::Ordering::{self, *};

use super::{ConvertFrom, Merge};
use crate::LatticeOrd;

/// Dominating pair compound lattice.
///
/// When merging if one `Key` (usually a timestamp) fully dominates (is greater than) the other,
/// then both that `Key` and corresponding `Val` are selected. If the `Key`s are equal or
/// incomparable, then both the `Key`s and `Val`s are merged.
///
/// `Key` specifies the key lattice (usually a timestamp), and `Val` specifies the value lattice.
///
/// Note that this is not a proper lattice, it fails associativity. However it will behave like a
/// proper lattice if `Key` is a totally ordered lattice or a properly formed vector clock lattice.
/// The exact meaning of "properly formed" is still TBD, but each node always incrementing its
/// entry for each operation sent should be sufficient.
#[derive(Copy, Clone, Debug, Default, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DomPair<Key, Val> {
    /// The `Key` of the  dominating pair lattice, usually a timestamp
    pub key: Key,
    /// The `Val` of the dominating pair lattice.
    pub val: Val,
}

impl<Key, Val> DomPair<Key, Val> {
    /// Create a `DomPair` from the given `Key` and `Val`.
    pub fn new(key: Key, val: Val) -> Self {
        Self { key, val }
    }

    /// Create a `DomPair` from the given `Into<Key>` and `Into<Val>`.
    pub fn new_from(key: impl Into<Key>, val: impl Into<Val>) -> Self {
        Self::new(key.into(), val.into())
    }
}

impl<KeySelf, KeyOther, ValSelf, ValOther> Merge<DomPair<KeyOther, ValOther>>
    for DomPair<KeySelf, ValSelf>
where
    KeySelf: Merge<KeyOther> + ConvertFrom<KeyOther> + PartialOrd<KeyOther>,
    ValSelf: Merge<ValOther> + ConvertFrom<ValOther>,
{
    fn merge(&mut self, other: DomPair<KeyOther, ValOther>) -> bool {
        match self.key.partial_cmp(&other.key) {
            None => {
                assert!(self.key.merge(other.key));
                self.val.merge(other.val);
                true
            }
            Some(Equal) => self.val.merge(other.val),
            Some(Less) => {
                *self = ConvertFrom::from(other);
                true
            }
            Some(Greater) => false,
        }
    }
}

impl<KeySelf, KeyOther, ValSelf, ValOther> ConvertFrom<DomPair<KeyOther, ValOther>>
    for DomPair<KeySelf, ValSelf>
where
    KeySelf: ConvertFrom<KeyOther>,
    ValSelf: ConvertFrom<ValOther>,
{
    fn from(other: DomPair<KeyOther, ValOther>) -> Self {
        Self {
            key: ConvertFrom::from(other.key),
            val: ConvertFrom::from(other.val),
        }
    }
}

impl<KeySelf, KeyOther, ValSelf, ValOther> PartialOrd<DomPair<KeyOther, ValOther>>
    for DomPair<KeySelf, ValSelf>
where
    KeySelf: PartialOrd<KeyOther>,
    ValSelf: PartialOrd<ValOther>,
{
    fn partial_cmp(&self, other: &DomPair<KeyOther, ValOther>) -> Option<Ordering> {
        match self.key.partial_cmp(&other.key) {
            Some(Equal) => self.val.partial_cmp(&other.val),
            otherwise => otherwise,
        }
    }
}
impl<KeySelf, KeyOther, ValSelf, ValOther> LatticeOrd<DomPair<KeyOther, ValOther>>
    for DomPair<KeySelf, ValSelf>
where
    Self: PartialOrd<DomPair<KeyOther, ValOther>>,
{
}

impl<KeySelf, KeyOther, ValSelf, ValOther> PartialEq<DomPair<KeyOther, ValOther>>
    for DomPair<KeySelf, ValSelf>
where
    KeySelf: PartialEq<KeyOther>,
    ValSelf: PartialEq<ValOther>,
{
    fn eq(&self, other: &DomPair<KeyOther, ValOther>) -> bool {
        if self.key != other.key {
            return false;
        }

        if self.val != other.val {
            return false;
        }

        true
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashSet;

    use super::*;
    use crate::ord::Max;
    use crate::set_union::SetUnionHashSet;
    use crate::test::{
        check_all, check_lattice_ord, check_lattice_properties, check_partial_ord_properties,
    };

    #[test]
    fn consistency() {
        let mut test_vec = Vec::new();

        for a in [vec![], vec![0], vec![1], vec![0, 1]] {
            for b in [vec![], vec![0], vec![1], vec![0, 1]] {
                test_vec.push(DomPair::new(
                    SetUnionHashSet::new_from(HashSet::from_iter(a.clone())),
                    SetUnionHashSet::new_from(HashSet::from_iter(b.clone())),
                ));
            }
        }

        check_lattice_ord(&test_vec);
        check_partial_ord_properties(&test_vec);
        // DomPair is not actually a lattice.
        assert!(std::panic::catch_unwind(|| check_lattice_properties(&test_vec)).is_err());
    }

    #[test]
    fn consistency_with_ord_lhs() {
        let mut test_vec = Vec::new();

        for a in [0, 1, 2] {
            for b in [vec![], vec![0], vec![1], vec![0, 1]] {
                test_vec.push(DomPair::new(
                    Max::new(a),
                    SetUnionHashSet::new_from(HashSet::from_iter(b.clone())),
                ));
            }
        }

        check_all(&test_vec);
    }
}
