use std::cmp::Ordering::{self, *};

use crate::{DeepReveal, IsBot, IsTop, LatticeFrom, LatticeOrd, Merge};

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
    /// The `Key` of the  dominating pair lattice, usually a timestamp.
    ///
    /// This field is public as it is always monotonically increasing in its lattice.
    pub key: Key,
    /// The `Val` of the dominating pair lattice.
    val: Val,
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

    /// Reveal the inner value as a shared reference.
    pub fn as_reveal_ref(&self) -> (&Key, &Val) {
        (&self.key, &self.val)
    }

    /// Reveal the inner value as an exclusive reference.
    pub fn as_reveal_mut(&mut self) -> (&mut Key, &mut Val) {
        (&mut self.key, &mut self.val)
    }

    /// Gets the inner by value, consuming self.
    pub fn into_reveal(self) -> (Key, Val) {
        (self.key, self.val)
    }
}

impl<Key, Val> DeepReveal for DomPair<Key, Val>
where
    Key: DeepReveal,
    Val: DeepReveal,
{
    type Revealed = (Key::Revealed, Val::Revealed);

    fn deep_reveal(self) -> Self::Revealed {
        (self.key.deep_reveal(), self.val.deep_reveal())
    }
}

impl<KeySelf, KeyOther, ValSelf, ValOther> Merge<DomPair<KeyOther, ValOther>>
    for DomPair<KeySelf, ValSelf>
where
    KeySelf: Merge<KeyOther> + LatticeFrom<KeyOther> + PartialOrd<KeyOther>,
    ValSelf: Merge<ValOther> + LatticeFrom<ValOther>,
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
                *self = LatticeFrom::lattice_from(other);
                true
            }
            Some(Greater) => false,
        }
    }
}

impl<KeySelf, KeyOther, ValSelf, ValOther> LatticeFrom<DomPair<KeyOther, ValOther>>
    for DomPair<KeySelf, ValSelf>
where
    KeySelf: LatticeFrom<KeyOther>,
    ValSelf: LatticeFrom<ValOther>,
{
    fn lattice_from(other: DomPair<KeyOther, ValOther>) -> Self {
        Self {
            key: LatticeFrom::lattice_from(other.key),
            val: LatticeFrom::lattice_from(other.val),
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

impl<Key, Val> IsBot for DomPair<Key, Val>
where
    Key: IsBot,
    Val: IsBot,
{
    fn is_bot(&self) -> bool {
        self.key.is_bot() && self.val.is_bot()
    }
}

impl<Key, Val> IsTop for DomPair<Key, Val>
where
    Key: IsTop,
    Val: IsTop,
{
    fn is_top(&self) -> bool {
        self.key.is_top() && self.val.is_top()
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashSet;

    use super::*;
    use crate::WithTop;
    use crate::ord::Max;
    use crate::set_union::SetUnionHashSet;
    use crate::test::{
        check_lattice_is_bot, check_lattice_is_top, check_lattice_ord, check_lattice_properties,
        check_partial_ord_properties,
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
        check_lattice_is_bot(&test_vec);
        // DomPair is not actually a lattice.
        assert!(std::panic::catch_unwind(|| check_lattice_properties(&test_vec)).is_err());
    }

    #[test]
    fn consistency_withtop() {
        let mut test_vec = vec![];

        let sub_items = &[
            Some(&[] as &[usize]),
            Some(&[0]),
            Some(&[1]),
            Some(&[0, 1]),
            None,
        ];

        for a in sub_items {
            for b in sub_items {
                test_vec.push(DomPair::new(
                    WithTop::new(
                        a.map(|x| SetUnionHashSet::new_from(HashSet::from_iter(x.iter().cloned()))),
                    ),
                    WithTop::new(
                        b.map(|x| SetUnionHashSet::new_from(HashSet::from_iter(x.iter().cloned()))),
                    ),
                ));
            }
        }

        check_lattice_ord(&test_vec);
        check_partial_ord_properties(&test_vec);
        check_lattice_is_bot(&test_vec);
        check_lattice_is_top(&test_vec);
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

        check_lattice_ord(&test_vec);
        check_lattice_properties(&test_vec);
        check_partial_ord_properties(&test_vec);
    }
}
