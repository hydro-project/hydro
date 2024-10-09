//! Helper test utils to test lattice implementation correctness.

use std::fmt::Debug;

use crate::{
    Atomize, IsBot, IsTop, Lattice, LatticeBimorphism, LatticeMorphism, LatticeOrd, Merge,
    NaiveLatticeOrd,
};

/// Helper which calls many other `check_*` functions in this module. See source code for which
/// functions are called.
pub fn check_all<T: Lattice + Clone + PartialEq + Debug + Default>(items: &[T]) {
    check_lattice_ord(items);
    check_partial_ord_properties(items);
    check_lattice_properties(items);
    check_lattice_is_bot(items);
    check_lattice_is_top(items);
    check_lattice_default_is_bot::<T>();
}

/// Check that the lattice's `PartialOrd` implementation agrees with the `NaiveLatticeOrd` partial
/// order derived from `Merge`.
pub fn check_lattice_ord<T: LatticeOrd + NaiveLatticeOrd + Debug>(items: &[T]) {
    // `NaiveLatticeOrd` is a better source of truth, as it is based on the `Merge` impl. But it
    // is inefficient. It also could be wrong if `Merge` doesn't properly return true/false
    // iff the merge changed things.
    for [a, b] in cartesian_power(items) {
        assert_eq!(a.naive_cmp(b), a.partial_cmp(b), "`{:?}`, `{:?}`", a, b);
    }
}

/// Checks `PartialOrd` and `PartialEq`'s reflexivity, symmetry, transitivity, and duality.
#[expect(
    clippy::eq_op,
    clippy::double_comparisons,
    reason = "testing comparison properties"
)]
pub fn check_partial_ord_properties<T: PartialOrd + PartialEq + Debug>(items: &[T]) {
    use std::cmp::Ordering::*;

    // PartialEq:
    // reflexive: a == a;
    for a in items {
        assert!(a == a, "Reflexivity: `{:?}` should equal itself.", a);
    }
    // symmetric: a == b implies b == a; and
    for [a, b] in cartesian_power(items) {
        assert_eq!(a == b, b == a, "`{:?}`, `{:?}`", a, b);
    }
    // transitive: a == b and b == c implies a == c.
    for [a, b, c] in cartesian_power(items) {
        if a == b && b == c {
            assert_eq!(a == b && b == c, a == c, "`{:?}`, `{:?}`, `{:?}`", a, b, c);
        }
    }

    // PartialOrd
    for [a, b] in cartesian_power(items) {
        // a == b if and only if partial_cmp(a, b) == Some(Equal).
        assert_eq!(
            a == b,
            a.partial_cmp(b) == Some(Equal),
            "`{:?}`, `{:?}`",
            a,
            b,
        );
        // a < b if and only if partial_cmp(a, b) == Some(Less)
        assert_eq!(
            a < b,
            a.partial_cmp(b) == Some(Less),
            "`{:?}`, `{:?}`",
            a,
            b,
        );
        // a > b if and only if partial_cmp(a, b) == Some(Greater)
        assert_eq!(
            a > b,
            a.partial_cmp(b) == Some(Greater),
            "`{:?}`, `{:?}`",
            a,
            b,
        );
        // a <= b if and only if a < b || a == b
        assert_eq!(a <= b, a < b || a == b, "`{:?}`, `{:?}`", a, b);
        // a >= b if and only if a > b || a == b
        assert_eq!(a >= b, a > b || a == b, "`{:?}`, `{:?}`", a, b);
        // PartialEq:
        // a != b if and only if !(a == b).
        // #[expect(clippy::nonminimal_bool, reason = "testing comparison properties")]
        {
            assert_eq!(a != b, !(a == b), "`{:?}`, `{:?}`", a, b);
        }
    }
    // transitivity: a < b and b < c implies a < c. The same must hold for both == and >.
    for [a, b, c] in cartesian_power(items) {
        if a < b && b < c {
            assert!(a < c, "`{:?}`, `{:?}`, `{:?}`", a, b, c);
        }
        if a == b && b == c {
            assert!(a == c, "`{:?}`, `{:?}`, `{:?}`", a, b, c);
        }
        if a > b && b > c {
            assert!(a > c, "`{:?}`, `{:?}`, `{:?}`", a, b, c);
        }
    }
    // duality: a < b if and only if b > a.
    for [a, b] in cartesian_power(items) {
        assert_eq!(a < b, b > a, "`{:?}`, `{:?}`", a, b);
    }
}

/// Check lattice associativity, commutativity, and idempotence.
pub fn check_lattice_properties<T: Merge<T> + Clone + PartialEq + Debug>(items: &[T]) {
    // Idempotency
    // x ∧ x = x
    for x in items {
        assert_eq!(
            Merge::merge_owned(x.clone(), x.clone()),
            x.clone(),
            "`{:?}`",
            x,
        );
    }

    // Commutativity
    // x ∧ y = y ∧ x
    for [x, y] in cartesian_power(items) {
        assert_eq!(
            Merge::merge_owned(x.clone(), y.clone()),
            Merge::merge_owned(y.clone(), x.clone()),
            "`{:?}`, `{:?}`",
            x,
            y,
        );
    }

    // Associativity
    // x ∧ (y ∧ z) = (x ∧ y) ∧ z
    for [x, y, z] in cartesian_power(items) {
        assert_eq!(
            Merge::merge_owned(x.clone(), Merge::merge_owned(y.clone(), z.clone())),
            Merge::merge_owned(Merge::merge_owned(x.clone(), y.clone()), z.clone()),
            "`{:?}`, `{:?}`, `{:?}`",
            x,
            y,
            z,
        );
    }
}

/// Checks that the item which is bot is less than (or equal to) all other items.
pub fn check_lattice_is_bot<T: IsBot + LatticeOrd + Debug>(items: &[T]) {
    let Some(bot) = items.iter().find(|&x| IsBot::is_bot(x)) else {
        return;
    };
    for x in items {
        assert!(bot <= x);
        assert_eq!(bot == x, x.is_bot(), "{:?}", x);
    }
}

/// Checks that the item which is top is greater than (or equal to) all other items.
pub fn check_lattice_is_top<T: IsTop + LatticeOrd + Debug>(items: &[T]) {
    let Some(top) = items.iter().find(|&x| IsTop::is_top(x)) else {
        return;
    };
    for x in items {
        assert!(x <= top);
        assert_eq!(top == x, x.is_top(), "{:?}", x);
    }
}

/// Asserts that [`IsBot`] is true for [`Default::default()`].
pub fn check_lattice_default_is_bot<T: IsBot + Default>() {
    assert!(T::is_bot(&T::default()));
}

/// Check that the atomized lattice points re-merge to form the same original lattice point, for each item in `items`.
pub fn check_atomize_each<
    T: Atomize + Merge<T::Atom> + LatticeOrd + IsBot + Default + Clone + Debug,
>(
    items: &[T],
) where
    T::Atom: Debug,
{
    for item in items {
        let mut reformed = T::default();
        let mut atoms = item.clone().atomize().peekable();
        assert_eq!(
            atoms.peek().is_none(),
            item.is_bot(),
            "`{:?}` atomize should return empty iterator ({}) if and only if item is bot ({}).",
            item,
            atoms.peek().is_none(),
            item.is_bot()
        );
        for atom in atoms {
            assert!(
                !atom.is_bot(),
                "`{:?}` atomize illegally returned a bottom atom `{:?}`.",
                item,
                atom,
            );
            reformed.merge(atom);
        }
        assert_eq!(item, &reformed, "`{:?}` atomize failed to reform", item);
    }
}

/// Checks that the [`LatticeMorphism`] is valid, i.e. that merge distributes over it.
pub fn check_lattice_morphism<LatIn, Func>(mut func: Func, items: &[LatIn])
where
    Func: LatticeMorphism<LatIn>,
    LatIn: Merge<LatIn> + Clone + PartialEq + Debug,
    Func::Output: Merge<Func::Output> + Clone + PartialEq + Debug,
{
    for [a, b] in cartesian_power(items) {
        assert_eq!(
            func.call(Merge::merge_owned(a.clone(), b.clone())),
            Merge::merge_owned(func.call(a.clone()), func.call(b.clone())),
            "Func not a morphism: `f(a ⊔ b) != f(a) ⊔ f(b)`
            \n`a = {:?}`, `b = {:?}`",
            a,
            b
        )
    }
}

/// Checks that the [`LatticeBimorphism`] is valid, i.e. that merge distributes over both arguments of it.
pub fn check_lattice_bimorphism<LatA, LatB, Func>(
    mut func: Func,
    items_a: &[LatA],
    items_b: &[LatB],
) where
    Func: LatticeBimorphism<LatA, LatB>,
    LatA: Merge<LatA> + Clone + PartialEq + Debug,
    LatB: Merge<LatB> + Clone + PartialEq + Debug,
    Func::Output: Merge<Func::Output> + Clone + PartialEq + Debug,
{
    // Morphism LHS, fixed RHS:
    for b in items_b {
        for [a, da] in cartesian_power(items_a) {
            assert_eq!(
                func.call(Merge::merge_owned(a.clone(), da.clone()), b.clone()),
                Merge::merge_owned(
                    func.call(a.clone(), b.clone()),
                    func.call(da.clone(), b.clone())
                ),
                "Left arg not a morphism: `f(a ⊔ da, b) != f(a, b) ⊔ f(da, b)`
                \n`a = {:?}`, `da = {:?}`, `b = {:?}`",
                a,
                da,
                b,
            );
        }
    }
    // Fixed LHS, morphism RHS:
    for a in items_a {
        for [b, db] in cartesian_power(items_b) {
            assert_eq!(
                func.call(a.clone(), Merge::merge_owned(b.clone(), db.clone())),
                Merge::merge_owned(
                    func.call(a.clone(), b.clone()),
                    func.call(a.clone(), db.clone())
                ),
                "Right arg not a morphism: `f(a, b ⊔ db) != f(a, b) ⊔ f(a, db)`
                \n`a = {:?}`, `b = {:?}`, `db = {:?}`",
                a,
                b,
                db,
            );
        }
    }
}

/// Returns an iterator of `N`-length arrays containing all permutations of `items` (with
/// replacement).
///
/// I.e. the `N`th cartesian power of `items`. I.e. the cartesian
/// product of `items` with itself `N` times.
pub fn cartesian_power<T, const N: usize>(
    items: &[T],
) -> impl ExactSizeIterator<Item = [&T; N]> + Clone {
    struct CartesianPower<'a, T, const N: usize> {
        items: &'a [T],
        iters: [std::iter::Peekable<std::slice::Iter<'a, T>>; N],
    }
    impl<'a, T, const N: usize> Iterator for CartesianPower<'a, T, N> {
        type Item = [&'a T; N];

        fn next(&mut self) -> Option<Self::Item> {
            if self.items.is_empty() {
                return None;
            }

            let mut go_next = true;
            let out = std::array::from_fn::<_, N, _>(|i| {
                let iter = &mut self.iters[i];
                let &item = iter.peek().unwrap();
                if go_next {
                    iter.next();
                    if iter.peek().is_none() {
                        // "Carry" the `go_next` to the next entry.
                        *iter = self.items.iter().peekable();
                    } else {
                        go_next = false;
                    }
                }
                item
            });
            if go_next {
                // This is the last element, after this return `None`.
                self.items = &[];
            };
            Some(out)
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            if self.items.is_empty() {
                return (0, Some(0));
            }
            let mut pow = 1;
            let mut passed = 0;
            for iter in self.iters.iter() {
                passed += pow * (self.items.len() - iter.len());
                pow *= self.items.len();
            }
            let size = pow - passed;
            (size, Some(size))
        }
    }
    impl<'a, T, const N: usize> ExactSizeIterator for CartesianPower<'a, T, N> {}
    impl<'a, T, const N: usize> Clone for CartesianPower<'a, T, N> {
        fn clone(&self) -> Self {
            Self {
                items: self.items,
                iters: self.iters.clone(),
            }
        }
    }
    let iters = std::array::from_fn::<_, N, _>(|_| items.iter().peekable());
    CartesianPower { items, iters }
}

#[test]
fn test_cartesian_power() {
    let items = &[1, 2, 3];
    let mut iter = cartesian_power(items);
    let mut len = 27;
    assert_eq!(len, iter.len());
    for x in items {
        for y in items {
            for z in items {
                len -= 1;
                assert_eq!(Some([z, y, x]), iter.next());
                assert_eq!(len, iter.len());
            }
        }
    }
}

#[test]
fn test_cartesian_power_zero() {
    let mut iter = cartesian_power::<_, 0>(&[0, 1, 2]);
    assert_eq!(1, iter.len());
    assert_eq!(Some([]), iter.next());
    assert_eq!(None, iter.next());
}

#[test]
fn test_cartesian_power_empty() {
    let mut iter = cartesian_power::<_, 4>(&[] as &[usize]);
    assert_eq!(0, iter.len());
    assert_eq!(None, iter.next());
}
