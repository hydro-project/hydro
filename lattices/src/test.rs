//! Helper test utils to test lattice implementation correctness.

use std::fmt::Debug;

use crate::{LatticeOrd, Merge, NaiveOrd};

/// Helper which calls [`check_lattice_ord`], [`check_partial_ord_properties`], and
/// [`check_lattice_properties`].
pub fn check_all<T: LatticeOrd + NaiveOrd + Merge<T> + Clone + Eq + Debug>(items: &[T]) {
    check_lattice_ord(items);
    check_partial_ord_properties(items);
    check_lattice_properties(items);
}

/// Check that the lattice's `PartialOrd` implementation agrees with the `NaiveOrd` partial orde
/// derived from `Merge.
pub fn check_lattice_ord<T: LatticeOrd + NaiveOrd>(items: &[T]) {
    // `NaiveOrd` is a better source of truth, as it is based on the `Merge` impl. But it
    // is inefficient. It also could be wrong if `Merge` doesn't properly return true/false
    // iff the merge changed things.
    for [a, b] in cartesian_power(items) {
        assert_eq!(a.naive_cmp(b), a.partial_cmp(b));
        assert_eq!(b.naive_cmp(a), b.partial_cmp(a));
    }
}

/// Checks `PartialOrd`, `PartialEq`, and `Eq`'s reflexivity, symmetry, transitivity, and duality.
#[allow(clippy::eq_op)]
#[allow(clippy::double_comparisons)]
pub fn check_partial_ord_properties<T: PartialOrd + Eq>(items: &[T]) {
    use std::cmp::Ordering::*;

    // PartialEq:
    // a != b if and only if !(a == b).
    for [a, b] in cartesian_power(items) {
        assert_eq!(a != b, !(a == b))
    }

    // Eq:
    // reflexive: a == a;
    for a in items {
        assert!(a == a)
    }
    // symmetric: a == b implies b == a; and
    for [a, b] in cartesian_power(items) {
        assert_eq!(a == b, b == a)
    }
    // transitive: a == b and b == c implies a == c.
    for [a, b, c] in cartesian_power(items) {
        if a == b && b == c {
            assert_eq!(a == b && b == c, a == c);
        }
    }

    // PartialOrd
    for [a, b] in cartesian_power(items) {
        // a == b if and only if partial_cmp(a, b) == Some(Equal).
        assert_eq!(a == b, a.partial_cmp(b) == Some(Equal));
        // a < b if and only if partial_cmp(a, b) == Some(Less)
        assert_eq!(a < b, a.partial_cmp(b) == Some(Less));
        // a > b if and only if partial_cmp(a, b) == Some(Greater)
        assert_eq!(a > b, a.partial_cmp(b) == Some(Greater));
        // a <= b if and only if a < b || a == b
        assert_eq!(a <= b, a < b || a == b);
        // a >= b if and only if a > b || a == b
        assert_eq!(a >= b, a > b || a == b);
        // a != b if and only if !(a == b).
        assert_eq!(a != b, !(a == b));
    }
    // transitivity: a < b and b < c implies a < c. The same must hold for both == and >.
    for [a, b, c] in cartesian_power(items) {
        if a < b && b < c {
            assert!(a < c);
        }
        if a == b && b == c {
            assert!(a == c);
        }
        if a > b && b > c {
            assert!(a > c);
        }
    }
    // duality: a < b if and only if b > a.
    for [a, b] in cartesian_power(items) {
        assert_eq!(a < b, b > a)
    }
}

/// Check lattice associativity, commutativity, and idempotence.
pub fn check_lattice_properties<T: Merge<T> + Clone + Eq + Debug>(items: &[T]) {
    // Idempotency
    // x ∧ x = x
    for x in items {
        assert_eq!(Merge::merge_owned(x.to_owned(), x.to_owned()), x.to_owned())
    }

    // Commutativity
    // x ∧ y = y ∧ x
    for [x, y] in cartesian_power(items) {
        assert_eq!(
            Merge::merge_owned(x.to_owned(), y.to_owned()),
            Merge::merge_owned(y.to_owned(), x.to_owned())
        )
    }

    // Associativity
    // x ∧ (y ∧ z) = (x ∧ y) ∧ z
    for [x, y, z] in cartesian_power(items) {
        assert_eq!(
            Merge::merge_owned(x.to_owned(), Merge::merge_owned(y.to_owned(), z.to_owned())),
            Merge::merge_owned(Merge::merge_owned(x.to_owned(), y.to_owned()), z.to_owned())
        )
    }
}

/// Returns an iterator of `N`-length arrays containing all possible permutations (with
/// replacement) of items in `items`. I.e. the `N`th cartesian power of `items`. I.e. the cartesian
/// product of `items` with itself `N` times.
pub fn cartesian_power<T, const N: usize>(
    items: &[T],
) -> impl Iterator<Item = [&T; N]> + ExactSizeIterator + Clone {
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
                items: self.items.clone(),
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
fn test_cartesian_power_empty() {
    let mut iter = cartesian_power::<_, 4>(&[] as &[usize]);
    assert_eq!(0, iter.len());
    assert_eq!(None, iter.next());
}
