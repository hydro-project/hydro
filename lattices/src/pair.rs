use crate::{DeepReveal, Lattice, LatticeBimorphism};

/// Pair compound lattice.
///
/// `LatA` and `LatB` specify the nested lattice types.
///
/// When merging, both sub-lattices are always merged.
#[derive(Copy, Clone, Debug, Default, Eq, Lattice)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Pair<LatA, LatB> {
    /// The "left" Lattice of the Pair lattice.
    pub a: LatA,

    /// The "right" Lattice of the Pair lattice.
    pub b: LatB,
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

    /// Reveal the inner value as a shared reference.
    pub fn as_reveal_ref(&self) -> (&LatA, &LatB) {
        (&self.a, &self.b)
    }

    /// Reveal the inner value as an exclusive reference.
    pub fn as_reveal_mut(&mut self) -> (&mut LatA, &mut LatB) {
        (&mut self.a, &mut self.b)
    }

    /// Gets the inner by value, consuming self.
    pub fn into_reveal(self) -> (LatA, LatB) {
        (self.a, self.b)
    }
}

impl<LatA, LatB> DeepReveal for Pair<LatA, LatB>
where
    LatA: DeepReveal,
    LatB: DeepReveal,
{
    type Revealed = (LatA::Revealed, LatB::Revealed);

    fn deep_reveal(self) -> Self::Revealed {
        (self.a.deep_reveal(), self.b.deep_reveal())
    }
}

/// Bimorphism which pairs up the two input lattices.
#[derive(Default)]
pub struct PairBimorphism;
impl<LatA, LatB> LatticeBimorphism<LatA, LatB> for PairBimorphism {
    type Output = Pair<LatA, LatB>;

    fn call(&mut self, lat_a: LatA, lat_b: LatB) -> Self::Output {
        Pair::new(lat_a, lat_b)
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashSet;

    use super::*;
    use crate::set_union::{SetUnionBTreeSet, SetUnionHashSet, SetUnionSingletonSet};
    use crate::test::{check_all, check_lattice_bimorphism};
    use crate::{Merge, WithTop};

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

        check_all(&test_vec);
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
                test_vec.push(Pair::new(
                    WithTop::new(
                        a.map(|x| SetUnionHashSet::new_from(HashSet::from_iter(x.iter().cloned()))),
                    ),
                    WithTop::new(
                        b.map(|x| SetUnionHashSet::new_from(HashSet::from_iter(x.iter().cloned()))),
                    ),
                ));
            }
        }

        check_all(&test_vec);
    }

    #[test]
    fn test_merge_direction() {
        let src = Pair::new(
            SetUnionSingletonSet::new_from(5),
            SetUnionSingletonSet::new_from("hello"),
        );
        let mut dst = Pair::new(
            SetUnionHashSet::new_from([1, 2]),
            SetUnionBTreeSet::new_from(["world"]),
        );
        dst.merge(src);
    }

    #[test]
    fn test_pair_bimorphism() {
        let items_a = &[
            SetUnionHashSet::new_from([]),
            SetUnionHashSet::new_from([0]),
            SetUnionHashSet::new_from([1]),
            SetUnionHashSet::new_from([0, 1]),
        ];
        let items_b = &[
            SetUnionBTreeSet::new("hello".chars().collect()),
            SetUnionBTreeSet::new("world".chars().collect()),
        ];

        check_lattice_bimorphism(PairBimorphism, items_a, items_a);
        check_lattice_bimorphism(PairBimorphism, items_a, items_b);
        check_lattice_bimorphism(PairBimorphism, items_b, items_a);
        check_lattice_bimorphism(PairBimorphism, items_b, items_b);
    }
}
