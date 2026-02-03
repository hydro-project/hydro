use std::fmt::Debug;

use crate::test::cartesian_power;

/// Defines a monoid structure.
///
/// A monoid is a set of items along with an associative binary operation `f` and an identity element `zero`.
/// The `f` operation combines two items and the `zero` element acts as the identity for `f`.
pub fn monoid<S: Debug + PartialEq + Clone, const N: usize>(
    items: &[S; N],
    f: &impl Fn(S, S) -> S,
    zero: S, // zero is the identity element of f
) -> Result<(), &'static str> {
    semigroup(items, f)?;
    identity(items, f, zero)?;
    Ok(())
}

/// Defines a semigroup structure.
///
/// A semigroup is a set of items along with an associative binary operation `f`.
pub fn semigroup<S: Debug + PartialEq + Clone, const N: usize>(
    items: &[S; N],
    f: &impl Fn(S, S) -> S,
) -> Result<(), &'static str> {
    associativity(items, f)?;
    Ok(())
}

/// Defines a semiring structure.
///
/// A semiring is a set of items along with two associative binary operations `f` and `g`,
/// and two identity elements `zero` and `one`.
/// f must be commutative and g must distribute over f.
/// the zero of f must also be absorbing with respect to g.
pub fn semiring<S: Debug + PartialEq + Clone, const N: usize>(
    items: &[S; N],
    f: &impl Fn(S, S) -> S,
    g: &impl Fn(S, S) -> S,
    zero: S, // zero is the identity element of f
    one: S,  // one is the identity element of g
) -> Result<(), &'static str> {
    commutative_monoid(items, f, zero.clone())?;
    monoid(items, g, one)?;

    absorbing_element(items, g, zero)?;

    distributive(items, f, g)?;

    Ok(())
}

/// Defines a ring structure.
///
/// A ring is a semiring with an inverse operation on f.
pub fn ring<S: Debug + PartialEq + Clone, const N: usize>(
    items: &[S; N],
    f: &impl Fn(S, S) -> S,
    g: &impl Fn(S, S) -> S,
    zero: S, // zero is the identity element of f
    one: S,  // one is the identity element of g
    b: &impl Fn(S) -> S,
) -> Result<(), &'static str> {
    semiring(items, f, g, zero.clone(), one)?;
    inverse(items, f, zero, b)?;
    Ok(())
}

/// Defines an integral domain structure.
///
/// An integral domain is a nonzero commutative ring with no nonzero zero divisors.
pub fn integral_domain<S: Debug + PartialEq + Clone, const N: usize>(
    items: &[S; N],
    f: &impl Fn(S, S) -> S,
    g: &impl Fn(S, S) -> S,
    zero: S,                     // zero is the identity element of f
    one: S,                      // one is the identity element of g
    inverse_f: &impl Fn(S) -> S, /* the function to compute the inverse element of an element with respect to f */
) -> Result<(), &'static str> {
    commutative_ring(items, f, g, zero.clone(), one, inverse_f)?;
    no_nonzero_zero_divisors(items, g, zero)?;
    Ok(())
}

/// Defines a no-nonzero-zero-divisors property.
///
/// x is a nonzero divisor if xy = 0 and y is also a nonzero element.
pub fn no_nonzero_zero_divisors<S: Debug + PartialEq + Clone, const N: usize>(
    items: &[S; N],
    f: &impl Fn(S, S) -> S,
    zero: S,
) -> Result<(), &'static str> {
    for a in items {
        for b in items {
            if *a != zero && *b != zero {
                if f(a.clone(), b.clone()) == zero {
                    return Err("No nonzero zero divisors check failed.");
                };
                if f(b.clone(), a.clone()) == zero {
                    return Err("No nonzero zero divisors check failed.");
                };
            }
        }
    }
    Ok(())
}

/// Defines a commutative ring structure.
///
/// A commutative ring is a ring where the multiplication operation g is commutative.
pub fn commutative_ring<S: Debug + PartialEq + Clone, const N: usize>(
    items: &[S; N],
    f: &impl Fn(S, S) -> S, // addition operation
    g: &impl Fn(S, S) -> S, // multiplication operation
    zero: S,                // zero is the identity element of f
    one: S,                 // one is the identity element of g
    inverse_f: &impl Fn(S) -> S,
) -> Result<(), &'static str> {
    semiring(items, f, g, zero.clone(), one)?;
    inverse(items, f, zero, inverse_f)?;
    commutativity(items, g)?;
    Ok(())
}

/// Defines a field structure.
///
/// A field is a commutative ring where every element has a multiplicative inverse.
pub fn field<S: Debug + PartialEq + Clone, const N: usize>(
    items: &[S; N],
    f: &impl Fn(S, S) -> S,
    g: &impl Fn(S, S) -> S,
    zero: S,                     // zero is the identity element of f
    one: S,                      // one is the identity element of g
    inverse_f: &impl Fn(S) -> S, /* inverse_f is the function that given x computes x' such that f(x,x') = zero. */
    inverse_g: &impl Fn(S) -> S, /* //inverse_g is the function that given x computes x' such that g(x,x') = one. */
) -> Result<(), &'static str> {
    commutative_ring(items, f, g, zero.clone(), one.clone(), inverse_f)?;
    nonzero_inverse(items, g, one, zero, inverse_g)?;
    Ok(())
}

/// Defines a commutative monoid structure.
///
/// A commutative monoid is a monoid where the operation f is commutative.
pub fn commutative_monoid<S: Debug + PartialEq + Clone, const N: usize>(
    items: &[S; N],
    f: &impl Fn(S, S) -> S,
    zero: S,
) -> Result<(), &'static str> {
    monoid(items, f, zero)?;
    commutativity(items, f)?;
    Ok(())
}

/// Defines a group structure.
///
/// A group is a set of items along with an associative binary operation `f`
/// an identity element `zero`
/// and every element has an inverse element with respect to `f`
pub fn group<S: Debug + PartialEq + Clone, const N: usize>(
    items: &[S; N],
    f: &impl Fn(S, S) -> S,
    zero: S,             // zero is the identity element of f
    b: &impl Fn(S) -> S, /* b is the function to compute the inverse element of an element with respect to f */
) -> Result<(), &'static str> {
    monoid(items, f, zero.clone())?;
    inverse(items, f, zero, b)?;
    Ok(())
}

/// Defines an abelian group structure.
///
/// An abelian group is a group where the operation f is commutative.
pub fn abelian_group<S: Debug + PartialEq + Clone, const N: usize>(
    items: &[S; N],
    f: &impl Fn(S, S) -> S,
    zero: S,
    b: &impl Fn(S) -> S, /* b is the function to compute the inverse element of an element with respect to f */
) -> Result<(), &'static str> {
    group(items, f, zero, b)?;
    commutativity(items, f)?;
    Ok(())
}

// Algebraic Properties
/// Defines the distributive property
///
/// a(b+c) = ab + ac
/// and (b+c)a = ba + ca
pub fn distributive<S: Debug + PartialEq + Clone, const N: usize>(
    items: &[S; N],
    f: &impl Fn(S, S) -> S,
    g: &impl Fn(S, S) -> S,
) -> Result<(), &'static str> {
    left_distributes(items, f, g)?;
    right_distributes(items, f, g)?;
    Ok(())
}

/// Defines the left distributive property
///
/// a(b+c) = ab + ac
pub fn left_distributes<S: Debug + PartialEq + Clone, const N: usize>(
    items: &[S; N],
    f: impl Fn(S, S) -> S,
    g: impl Fn(S, S) -> S,
) -> Result<(), &'static str> {
    for [a, b, c] in cartesian_power(items) {
        if g(a.clone(), f(b.clone(), c.clone()))
            != f(g(a.clone(), b.clone()), g(a.clone(), c.clone()))
        {
            return Err("Left distributive property check failed.");
        }
    }
    Ok(())
}

/// Defines the right distributive property.
///
/// (b+c)a = ba + ca
pub fn right_distributes<S: Debug + PartialEq + Clone, const N: usize>(
    items: &[S; N],
    f: impl Fn(S, S) -> S,
    g: impl Fn(S, S) -> S,
) -> Result<(), &'static str> {
    for [a, b, c] in cartesian_power(items) {
        if g(f(b.clone(), c.clone()), a.clone())
            != f(g(b.clone(), a.clone()), g(c.clone(), a.clone()))
        {
            return Err("Right distributive property check failed.");
        }
    }
    Ok(())
}

/// Defines the absorbing_element property.
///
/// An element z is absorbing if az = z and za = z for all a.
pub fn absorbing_element<S: Debug + PartialEq + Clone, const N: usize>(
    items: &[S; N],
    f: impl Fn(S, S) -> S,
    z: S, // absorbing element (anything multiplied by z is z e.g. 0 in integers)
) -> Result<(), &'static str> {
    for a in items {
        // az = z
        if f(a.clone(), z.clone()) != z.clone() {
            return Err("Absorbing element property check failed.");
        }

        // za = z
        if f(z.clone(), a.clone()) != z.clone() {
            return Err("Absorbing element property check failed.");
        }
    }
    Ok(())
}

/// Defines the inverse property.
///
/// An element b is the inverse of a if ab = e and ba = e for some identity element e.
pub fn inverse<S: Debug + PartialEq + Clone, const N: usize>(
    items: &[S; N],
    f: impl Fn(S, S) -> S,
    e: S,               // e is the identity element of f
    b: impl Fn(S) -> S, /* b is the function to compute the inverse element of an element with respect to f */
) -> Result<(), &'static str> {
    // ∃b: ab = e, ba = e
    for a in items {
        if f(a.clone(), b(a.clone())) != e {
            return Err("Inverse check failed.");
        }
        if f(b(a.clone()), a.clone()) != e {
            return Err("Inverse check failed.");
        }
    }
    Ok(())
}

/// Defines the non_zero inverse property.
///
/// Every element except zero must have an inverse.
pub fn nonzero_inverse<S: Debug + PartialEq + Clone, const N: usize>(
    items: &[S; N],
    f: impl Fn(S, S) -> S,
    e: S,
    zero: S,
    b: impl Fn(S) -> S,
) -> Result<(), &'static str> {
    // ∃b: ab = e, ba = e
    for a in items {
        if *a != zero {
            if f(a.clone(), b(a.clone())) != e {
                return Err("Nonzero inverse check failed.");
            }
            if f(b(a.clone()), a.clone()) != e {
                return Err("Nonzero inverse check failed.");
            }
        }
    }
    Ok(())
}

/// Defines the identity property.
///
/// An element e is the identity of f if ae = a and ea = a for all a.
pub fn identity<S: Debug + PartialEq + Clone, const N: usize>(
    items: &[S; N],
    f: impl Fn(S, S) -> S,
    e: S,
) -> Result<(), &'static str> {
    // ea = a, ae = a
    for a in items {
        if f(e.clone(), a.clone()) != a.clone() {
            return Err("Left Identity check failed.");
        }
        if f(a.clone(), e.clone()) != a.clone() {
            return Err("Right Identity check failed.");
        }
    }
    Ok(())
}

/// Defines the associativity property.
///
/// a(bc) = (ab)c
pub fn associativity<S: Debug + PartialEq + Clone, const N: usize>(
    items: &[S; N],
    f: impl Fn(S, S) -> S,
) -> Result<(), &'static str> {
    for [a, b, c] in cartesian_power(items) {
        if f(a.clone(), f(b.clone(), c.clone())) != // f(a, f(b,c)) ie a + (b + c)
            f(f(a.clone(), b.clone()), c.clone())
        // f(f(a,b),c) ie (a + b) + c
        {
            return Err("Associativity check failed.");
        }
    }
    Ok(())
}

/// Defines the commutativity property.
///
/// xy = yx
pub fn commutativity<S: Debug + PartialEq + Clone, const N: usize>(
    items: &[S; N],
    f: impl Fn(S, S) -> S,
) -> Result<(), &'static str> {
    for [x, y] in cartesian_power(items) {
        if f(x.clone(), y.clone()) != f(y.clone(), x.clone()) {
            // a + b = b + a
            return Err("Commutativity check failed.");
        }
    }
    Ok(())
}

/// Defines the idempotency property.
///
/// xx = x
pub fn idempotency<S: Debug + PartialEq + Clone, const N: usize>(
    items: &[S; N],
    f: impl Fn(S, S) -> S,
) -> Result<(), &'static str> {
    for x in items {
        if f(x.clone(), x.clone()) != x.clone() {
            return Err("Idempotency check failed.");
        }
    }
    Ok(())
}

/// Defines the linearity property
///
/// q is linear with respect to some group operation + if q(a+b) = q(a) + q(b)
/// This is the same as q being a group homomorphism
/// As defined in the paper "DBSP: Automatic Incremental View Maintenance for Rich Query Languages"
/// Input parameters f, g, and q represent (f) the base operation of the algebraic structure for state,
/// (g) the base operation of the algebraic structure the query q outputs to
/// and (q) the query over f that we want to check for linearity (to incrementalize) respectively
pub fn linearity<S: Debug + PartialEq + Clone, R: Debug + PartialEq + Clone>(
    items: &[S],
    f: impl Fn(S, S) -> S,
    g: impl Fn(R, R) -> R,
    q: impl Fn(S) -> R,
) -> Result<(), &'static str> {
    for [a, b] in cartesian_power(items) {
        if q(f(a.clone(), b.clone())) != g(q(b.clone()), q(a.clone())) {
            // q(f(a,b)) = f(q(a), q(b))
            return Err("Linearity check failed.");
        }
    }
    Ok(())
}

/// Defines the bilinearity property
///
/// q is bilinear with respect to + if q(a + b, c) = q(a,c) + q(b,c) and q(a,c + d) = q(a,c) + q(a,d)
/// This is the same as q being distributive over the addition operation of the three groups S, T, and R in q:S x T --> R
/// As defined in the paper "DBSP: Automatic Incremental View Maintenance for Rich Query Languages
/// Input parameters f, h, g, and q represent (f) the base operation of the algebraic structure on the left input to the query q,
/// (h) the base operation of the algebraic structure on the right input to the query q,
/// (g) the base operation of the algebraic structure the query q outputs to,
/// and (q) The query over (f,g) that we want to check for bilinearity (to incrementalize)
pub fn bilinearity<
    S: Debug + PartialEq + Clone,
    R: Debug + PartialEq + Clone,
    T: Debug + PartialEq + Clone,
>(
    items_f: &[S],
    items_h: &[T],
    f: impl Fn(S, S) -> S,
    h: impl Fn(T, T) -> T,
    g: impl Fn(R, R) -> R,
    q: impl Fn(S, T) -> R,
) -> Result<(), &'static str> {
    for [a, b] in cartesian_power(items_f) {
        for [c, d] in cartesian_power(items_h) {
            if q(f(a.clone(), b.clone()), c.clone())
                != g(q(a.clone(), c.clone()), q(b.clone(), c.clone()))
                || q(a.clone(), h(c.clone(), d.clone()))
                    != g(q(a.clone(), c.clone()), q(a.clone(), d.clone()))
            {
                // q(a + b, c) = q(a,c) + q(b,c) AND
                // q(a,c + d) = q(a,c + q(c,d)
                return Err("Bilinearity check failed.");
            }
        }
    }
    Ok(())
}

// Functions for testing out whether user defined code satisfies different properties

// A list of algebraic properties of a single function that we support
// static SINGLE_FUNCTION_PROPERTIES: [(&str, fn(&[S; N], impl Fn(S, S) -> S)); 6] = [
//     ("associativity", associativity),
//     ("commutativity", commutativity),
//     ("idempotency", idempotency),
//     ("identity", identity),
//     ("inverse", inverse),
//     ("absorbing_element", absorbing_element)];

/// Loop through each algebraic property in SINGLE_FUNCTION_PROPERTIES and test for them.
pub fn get_single_function_properties<S: Debug + PartialEq + Clone, const N: usize>(
    items: &[S; N],
    f: impl Fn(S, S) -> S,
    // identity element (TODO make optional parameter)
    e: S,
    // inverse function (TODO make optional parameter)
    b: impl Fn(S) -> S,
    // absorbing element (TODO make optional parameter)
    z: S,
) -> Vec<String> {
    // store the list of properties (strings) that are satisfied to be returned
    let mut properties_satisfied: Vec<String> = Vec::new();

    // TODO make this a loop through the SINGLE_FUNCTION_PROPERTIES array
    if associativity(items, &f).is_ok() {
        properties_satisfied.push("associativity".to_owned());
    }
    if commutativity(items, &f).is_ok() {
        properties_satisfied.push("commutativity".to_owned());
    }
    if idempotency(items, &f).is_ok() {
        properties_satisfied.push("idempotency".to_owned());
    }
    if identity(items, &f, e.clone()).is_ok() {
        properties_satisfied.push("identity".to_owned());
    }
    if inverse(items, &f, e, b).is_ok() {
        properties_satisfied.push("inverse".to_owned());
    }
    if absorbing_element(items, &f, z).is_ok() {
        properties_satisfied.push("absorbing_element".to_owned());
    }

    properties_satisfied
}

// TODO write a function to take in a set of functions and check which pairs satisfy different pairwise properties (e.g. distributivity

// Tests

#[cfg(test)]
mod test {
    use std::collections::HashSet;

    use crate::algebra::*;

    static TEST_ITEMS: &[u32; 14] = &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13];
    static TEST_ITEMS_NONZERO: &[u32; 13] = &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13];
    static TEST_MOD_PRIME_7: &[u32; 7] = &[0, 1, 2, 3, 4, 5, 6];
    static TEST_BOOLS: &[bool; 2] = &[false, true];

    #[test]
    fn test_associativity() {
        // Test that max() is associative and exponentiation isn't
        assert!(associativity(TEST_ITEMS, u32::max).is_ok());
        assert!(associativity(TEST_ITEMS, u32::wrapping_pow).is_err());
    }

    #[test]
    fn test_left_distributes() {
        // Test that multiplication and subtraction are left distributive  a(b-c) = ab - ac.
        // but exponentiation and subtraction isn't since a^(b-c) != a^b - a^c.
        assert!(left_distributes(TEST_ITEMS, u32::wrapping_sub, u32::wrapping_mul).is_ok());
        assert!(left_distributes(TEST_ITEMS, u32::wrapping_sub, u32::wrapping_pow).is_err());
    }

    #[test]
    fn test_right_distributes() {
        // Test that multiplication and subtraction are right distributive (b-c)a = ba - ca.
        // but exponentiation and subtraction isn't since (b-c)^a != b^a - c^a.
        assert!(right_distributes(TEST_ITEMS, u32::wrapping_sub, u32::wrapping_mul).is_ok());
        assert!(right_distributes(TEST_ITEMS, u32::wrapping_sub, u32::wrapping_pow).is_err());
    }

    #[test]
    fn test_nonzero_inverse() {
        // Test that addition and subtraction has a nonzero inverse and that multiplication does not.
        assert!(
            nonzero_inverse(TEST_ITEMS, u32::wrapping_add, 0, 0, |x| {
                0u32.wrapping_sub(x)
            })
            .is_ok()
        );
        assert!(
            nonzero_inverse(TEST_ITEMS, u32::wrapping_sub, 0, 0, |x| {
                0u32.wrapping_add(x)
            })
            .is_ok()
        );
        assert!(
            right_distributes(TEST_ITEMS_NONZERO, u32::wrapping_div, u32::wrapping_mul).is_err()
        );
    }

    #[test]
    fn test_idempotency() {
        // Test that max() is idempotent and addition is non-idempotent
        assert!(idempotency(TEST_ITEMS, u32::max).is_ok());

        assert!(idempotency(TEST_ITEMS, u32::wrapping_add).is_err());
    }

    #[test]
    fn test_commutativity() {
        // Test that max() is commutative and division is non-commutative
        assert!(commutativity(TEST_ITEMS, u32::max).is_ok());
        assert!(commutativity(TEST_ITEMS_NONZERO, u32::wrapping_div).is_err());
        // Test items non-zero to avoid a divide by zero exception
    }

    #[test]
    fn test_commutative_ring() {
        // Test that (Z, +, *) is a commutative ring.
        assert!(
            commutative_ring(
                TEST_ITEMS,
                &u32::wrapping_add,
                &u32::wrapping_mul,
                0,
                1,
                &|x| 0u32.wrapping_sub(x),
            )
            .is_ok()
        );

        // Test that (Z, +, ^) is not a commutative ring.
        assert!(
            commutative_ring(
                TEST_ITEMS,
                &u32::wrapping_add,
                &u32::wrapping_pow,
                0,
                1,
                &|x| 0u32.wrapping_sub(x),
            )
            .is_err()
        );

        // Test that matrix multiplication is not a commutative ring.
        assert!(
            commutative_ring(
                &[[[1, 2], [3, 4]], [[5, 6], [7, 8]], [[9, 10], [11, 12]]],
                &|a, b| {
                    [
                        [a[0][0] + b[0][0], a[0][1] + b[0][1]],
                        [a[1][0] + b[1][0], a[1][0] + b[1][1]],
                    ]
                },
                &|a, b| {
                    [
                        [
                            a[0][0] * b[0][0] + a[0][1] * b[1][0],
                            a[0][0] * b[0][1] + a[0][1] * b[1][1],
                        ],
                        [
                            a[1][0] * b[0][0] + a[1][1] * b[1][0],
                            a[1][0] * b[0][1] + a[1][1] * b[1][1],
                        ],
                    ]
                },
                [[0, 0], [0, 0]],
                [[1, 0], [0, 1]],
                &|a| {
                    [
                        [
                            -a[0][0] / (a[0][0] * a[1][1] - a[0][1] * a[1][1]),
                            -a[0][1] / (a[0][0] * a[1][1] - a[0][1] * a[1][1]),
                        ],
                        [
                            -a[1][0] / (a[0][0] * a[1][1] - a[0][1] * a[1][1]),
                            -a[1][1] / (a[0][0] * a[1][1] - a[0][1] * a[1][1]),
                        ],
                    ]
                },
            )
            .is_err()
        );
    }

    #[test]
    fn test_commutative_monoid() {
        // Test that (Z, +) is commutative monoid since every abelian group is commutative monoid.
        assert!(commutative_monoid(TEST_ITEMS, &u32::wrapping_add, 0).is_ok());

        // Test that  set of natural numbers N = {0, 1, 2, ...} is a commutative monoid under addition (identity element 0) or multiplication (identity element 1).
        assert!(commutative_monoid(TEST_ITEMS, &u32::wrapping_mul, 1).is_ok());
        assert!(commutative_monoid(TEST_ITEMS, &u32::wrapping_add, 0).is_ok());

        // Test that ({true, false}, ∧) is a commutative monoid with identity element true.
        assert!(commutative_monoid(TEST_BOOLS, &|a, b| a & b, true).is_ok()); // logical AND

        // Test that (Z, -) is not a commutative monoid.
        assert!(commutative_monoid(TEST_ITEMS, &u32::wrapping_sub, 0).is_err());

        // Test that (N, +) is not a commutative monoid since it doesn't have an identity element (0 is missing).
        assert!(commutative_monoid(TEST_ITEMS_NONZERO, &u32::wrapping_add, 1).is_err()); // Note that 1 is an arbitrary identity element in TEST_ITEMS_NONZERO since it doesn't have an identity element 0.

        // Test that (Z, ^) is not a commutative monoid.
        assert!(commutative_monoid(TEST_ITEMS, &u32::wrapping_pow, 3).is_err());
    }

    #[test]
    fn test_semigroup() {
        // Test that N := {1, 2, . . .} together with addition is a semigroup.
        assert!(semigroup(TEST_ITEMS_NONZERO, &u32::wrapping_add).is_ok());
        // Test that set of all natural numbers N = {0, 1, 2, ...} is a semigroup under addition.
        assert!(semigroup(TEST_ITEMS, &u32::wrapping_add).is_ok());
        // Test that set of all natural numbers N = {0, 1, 2, ...} is a semigroup under multiplication.
        assert!(semigroup(TEST_ITEMS, &u32::wrapping_mul).is_ok());
        // Test that ({true, false}, ∧) is a semigroup.
        assert!(semigroup(TEST_BOOLS, &|a, b| a & b).is_ok()); // logical AND
        // Test that matrix multiplication is a semigroup.
        assert!(
            semigroup(
                &[[[1, 2], [3, 4]], [[5, 6], [7, 8]], [[9, 10], [11, 12]]],
                &|a, b| {
                    [
                        [
                            a[0][0] * b[0][0] + a[0][1] * b[1][0],
                            a[0][0] * b[0][1] + a[0][1] * b[1][1],
                        ],
                        [
                            a[1][0] * b[0][0] + a[1][1] * b[1][0],
                            a[1][0] * b[0][1] + a[1][1] * b[1][1],
                        ],
                    ]
                },
            )
            .is_ok()
        );
        // Test that set of all natural numbers N = {0, 1, 2, ...} is not a semigroup under exponentiation.
        assert!(semigroup(TEST_ITEMS, &u32::wrapping_pow).is_err());
    }

    #[test]
    fn test_identity() {
        // Test that 0 is the identity for addition and 5 is not
        assert!(identity(TEST_ITEMS, u32::wrapping_add, 0).is_ok());

        assert!(identity(TEST_ITEMS, u32::wrapping_add, 5).is_err());
    }

    #[test]
    fn test_inverse() {
        // Test that subtraction is the inverse of addition and that addition is not the inverse of addition
        assert!(inverse(TEST_ITEMS, u32::wrapping_add, 0, |x| 0u32.wrapping_sub(x)).is_ok());

        assert!(inverse(TEST_ITEMS, u32::wrapping_add, 0, |x| 0u32.wrapping_add(x)).is_err());
    }

    #[test]
    fn test_distributive() {
        // Test that addition and multiplication are distributive and that addition and max() are not
        assert!(distributive(TEST_ITEMS, &u32::wrapping_add, &u32::wrapping_mul).is_ok());
        assert!(distributive(TEST_ITEMS, &u32::wrapping_add, &u32::max).is_err());
    }

    #[test]
    fn test_linearity() {
        // Test that multiplication over the (Z,+) group is linear
        // but exponentiation over the (Z,+) group is not linear
        assert!(
            linearity(TEST_ITEMS, u32::wrapping_add, u32::wrapping_add, |x| {
                u32::wrapping_mul(x, 5)
            })
            .is_ok()
        );
        assert!(
            linearity(TEST_ITEMS, u32::wrapping_add, u32::wrapping_add, |x| {
                u32::pow(x, 5)
            })
            .is_err()
        );
    }

    #[test]
    fn test_bilinearity() {
        // Test that multiplication over the (Z,+) group is bilinear
        // but exponentiation over the (Z,+) group is not bilinear
        assert!(
            bilinearity(
                TEST_ITEMS,
                TEST_ITEMS,
                u32::wrapping_add,
                u32::wrapping_add,
                u32::wrapping_add,
                u32::wrapping_mul
            )
            .is_ok()
        );
        assert!(
            bilinearity(
                TEST_ITEMS,
                TEST_ITEMS,
                u32::wrapping_add,
                u32::wrapping_add,
                u32::wrapping_add,
                u32::pow
            )
            .is_err()
        );
    }

    #[test]
    fn test_group() {
        // Test that (Z, +) form a group.
        assert!(group(TEST_ITEMS, &u32::wrapping_add, 0, &|x| 0u32.wrapping_sub(x)).is_ok());
        // Test that (Z/7Z, +) form a group.
        assert!(group(TEST_MOD_PRIME_7, &modulo_add_7, 0, &modulo_sub_7).is_ok());
        // Test that (Z/14Z, +) form a group.
        assert!(group(TEST_ITEMS, &modulo_add_14, 0, &modulo_sub_14).is_ok());
        // Test that (Z, *) do not form a group since it has no inverse.
        assert!(
            group(TEST_ITEMS_NONZERO, &u32::wrapping_mul, 1, &|x| 1u32
                .wrapping_div(x))
            .is_err()
        );
    }

    #[test]
    fn test_abelian_group() {
        // Test that (Z, +) form an abelian group.
        assert!(
            abelian_group(TEST_ITEMS, &u32::wrapping_add, 0, &|x| 0u32.wrapping_sub(x)).is_ok()
        );
        // Test that (Z/7Z, +) form an abelian group.
        assert!(abelian_group(TEST_MOD_PRIME_7, &modulo_add_7, 0, &modulo_sub_7).is_ok());
        // Test that (Z, *) do not form an abelian group.
        assert!(
            abelian_group(TEST_ITEMS_NONZERO, &u32::wrapping_mul, 1, &|x| 1u32
                .wrapping_div(x))
            .is_err()
        );
        // Test that matrix multiplication is not an abelian group.
        assert!(
            abelian_group(
                &[[[1, 2], [3, 4]], [[5, 6], [7, 8]], [[9, 10], [11, 12]]],
                &|a, b| {
                    [
                        [
                            a[0][0] * b[0][0] + a[0][1] * b[1][0],
                            a[0][0] * b[0][1] + a[0][1] * b[1][1],
                        ],
                        [
                            a[1][0] * b[0][0] + a[1][1] * b[1][0],
                            a[1][0] * b[0][1] + a[1][1] * b[1][1],
                        ],
                    ]
                },
                [[1, 0], [0, 1]],
                &|a| {
                    [
                        [
                            -a[0][0] / (a[0][0] * a[1][1] - a[0][1] * a[1][1]),
                            -a[0][1] / (a[0][0] * a[1][1] - a[0][1] * a[1][1]),
                        ],
                        [
                            -a[1][0] / (a[0][0] * a[1][1] - a[0][1] * a[1][1]),
                            -a[1][1] / (a[0][0] * a[1][1] - a[0][1] * a[1][1]),
                        ],
                    ]
                },
            )
            .is_err()
        );
    }

    #[test]
    fn test_monoid() {
        // Test that N = {0, 1, 2, . . .} is a monoid with respect to addition
        assert!(monoid(TEST_ITEMS, &u32::wrapping_add, 0).is_ok());
        // Test that N+ = N − {0} and N are both monoids with respect to multiplication
        assert!(monoid(TEST_ITEMS_NONZERO, &u32::wrapping_mul, 1).is_ok());
        assert!(monoid(TEST_ITEMS, &u32::wrapping_mul, 1).is_ok());
        // Test that the set of nxn matrix with matrix multiplication is a monoid.
        assert!(
            monoid(
                &[[[1, 2], [3, 4]], [[5, 6], [7, 8]], [[9, 10], [11, 12]]],
                &|a, b| {
                    [
                        [
                            a[0][0] * b[0][0] + a[0][1] * b[1][0],
                            a[0][0] * b[0][1] + a[0][1] * b[1][1],
                        ],
                        [
                            a[1][0] * b[0][0] + a[1][1] * b[1][0],
                            a[1][0] * b[0][1] + a[1][1] * b[1][1],
                        ],
                    ]
                },
                [[1, 0], [0, 1]],
            )
            .is_ok()
        );
        // Test that N+ = N − {0} is not a monoid with respect to addition since it doesn't have an identity element (0 is missing).
        assert!(monoid(TEST_ITEMS_NONZERO, &u32::wrapping_add, 1).is_err());
    }

    #[test]
    fn test_absorbing() {
        // Test that 0 is absorbing for multiplication and 5 is not
        assert!(absorbing_element(TEST_ITEMS, u32::wrapping_mul, 0).is_ok());
        assert!(absorbing_element(TEST_ITEMS, u32::wrapping_mul, 5).is_err());
    }

    // Performs addition modulo 7, ensuring the result remains within the range of 0 to 6.
    // This function is used to compute addition modulo 7 within the context of testing integral domains.
    fn modulo_add_7(a: u32, b: u32) -> u32 {
        u32::wrapping_add(a, b) % 7
    }

    // Performs addition modulo 14, ensuring the result remains within the range of 0 to 13.
    // This function is used to compute addition modulo 14 within the context of testing integral domains.
    fn modulo_add_14(a: u32, b: u32) -> u32 {
        u32::wrapping_add(a, b) % 14
    }

    // Performs subtraction modulo 7, ensuring the result remains within the range of 0 to 6.
    // This function is used to compute subtraction modulo 7 within the context of testing integral domains.
    fn modulo_sub_7(a: u32) -> u32 {
        u32::wrapping_sub(7, a) % 7
    }

    // Performs subtraction modulo 14, ensuring the result remains within the range of 0 to 13.
    // This function is used to compute subtraction modulo 14 within the context of testing integral domains.
    fn modulo_sub_14(a: u32) -> u32 {
        u32::wrapping_sub(14, a) % 14
    }

    // Performs multiplication modulo 7, ensuring the result remains within the range of 0 to 6.
    // This function is used to compute multiplication modulo 7 within the context of testing integral domains.
    fn modulo_mult_7(a: u32, b: u32) -> u32 {
        u32::wrapping_mul(a, b) % 7
    }

    // Performs multiplication modulo 14, ensuring the result remains within the range of 0 to 13.
    // This function is used to compute multiplication modulo 14 within the context of testing integral domains.
    fn modulo_mult_14(a: u32, b: u32) -> u32 {
        u32::wrapping_mul(a, b) % 14
    }

    #[test]
    fn test_additive_inverse_7() {
        // Tests that the additive inverse of each element in the ring of integers modulo 7 is correct.
        assert_eq!(0, modulo_sub_7(0));
        assert_eq!(1, modulo_sub_7(6));
        assert_eq!(2, modulo_sub_7(5));
        assert_eq!(3, modulo_sub_7(4));
        assert_eq!(4, modulo_sub_7(3));
        assert_eq!(6, modulo_sub_7(1));
    }

    #[test]
    fn test_modulo_mu14() {
        // Tests that the multiplication modulo 14 is correct.
        assert_eq!(0, modulo_mult_14(2, 7));
        assert_eq!(3, modulo_mult_14(1, 3));
        assert_eq!(2, modulo_mult_14(2, 1));
        assert_eq!(3, modulo_mult_14(3, 1));
        assert_eq!(4, modulo_mult_14(2, 2));
        assert_eq!(6, modulo_mult_14(2, 3));
        assert_eq!(9, modulo_mult_14(3, 3));
    }
    #[test]
    fn test_modulo_mu7() {
        // Tests that the multiplication modulo 7 is correct.
        assert_eq!(0, modulo_mult_7(0, 0));
        assert_eq!(3, modulo_mult_7(1, 3));
        assert_eq!(2, modulo_mult_7(2, 1));
        assert_eq!(2, modulo_mult_7(3, 3));
        assert_eq!(2, modulo_mult_7(3, 3));
        assert_eq!(5, modulo_mult_7(3, 4));
        assert_eq!(1, modulo_mult_7(3, 5));
    }

    #[test]
    fn test_no_nonzero_zero_divisors() {
        // The ring of integer mod prime number has no nonzero zero divisors.
        assert!(no_nonzero_zero_divisors(TEST_MOD_PRIME_7, &modulo_mult_7, 0).is_ok());
        // The ring of integers with multiplication mod prime number has nonzero zero divisors. (e.g. 1 * 7 = 0 mod 7)
        assert!(no_nonzero_zero_divisors(TEST_ITEMS, &modulo_mult_7, 0).is_err());
    }

    #[test]
    fn test_integral_domain() {
        // The ring of integers modulo a prime number is an integral domain.
        assert!(
            integral_domain(
                TEST_MOD_PRIME_7,
                &modulo_add_7,
                &modulo_mult_7,
                0,
                1,
                &modulo_sub_7,
            )
            .is_ok()
        );
        // The ring of integers modulo a composite number is not an integral domain.
        assert!(
            integral_domain(
                TEST_ITEMS,
                &modulo_add_14,
                &modulo_mult_14,
                0,
                1,
                &modulo_sub_14,
            )
            .is_err()
        );
    }

    #[test]
    fn test_field() {
        // Test that GF2 (0, 1, XOR, AND) is a field and  +, x, 0, 1, - is not a field (no multiplicative inverses)
        // Note GF2 is the Galois Field with 2 elements.

        assert!(
            field(
                TEST_BOOLS,
                &|a, b| a ^ b, // logical XOR
                &|a, b| a & b, // a & b, // logical AND
                false,
                true,
                &|x| x, // XOR(x,x) = false, the identity for XOR
                &|_x| true /* AND(x,true) = true, the identity for AND. Note that the inverse doesn't need to work for the additive identity (false)
                            */
            )
            .is_ok()
        );

        assert!(
            field(
                TEST_ITEMS,
                &u32::wrapping_add,
                &u32::wrapping_mul,
                0,
                1,
                &|x| 0u32.wrapping_sub(x),
                &|x| 0u32.wrapping_sub(x) //Note there is no valid inverse function for multiplication over the integers so we just pick some function
            )
            .is_err()
        );
    }

    #[test]
    fn test_ring() {
        // Test that +, x, 0, 1, - are a ring and +, x, 0, 5 are not (5 isn't a multiplicative identity)
        assert!(
            ring(
                TEST_ITEMS,
                &u32::wrapping_add,
                &u32::wrapping_mul,
                0,
                1,
                &|x| 0u32.wrapping_sub(x),
            )
            .is_ok()
        );
        assert!(
            ring(
                TEST_ITEMS,
                &u32::wrapping_add,
                &u32::wrapping_mul,
                0,
                5,
                &|x| 0u32.wrapping_sub(x),
            )
            .is_err()
        );
    }

    #[test]
    fn test_semiring() {
        // Test +, x is a semiring
        assert!(semiring(TEST_ITEMS, &u32::wrapping_add, &u32::wrapping_mul, 0, 1).is_ok());

        // Test boolean semiring with AND as + and OR as x
        assert!(semiring(&[false, true], &|x, y| x | y, &|x, y| x & y, false, true).is_ok());

        // Test min plus semiring. + is min and x is plus. Also known as the "tropical semiring"
        assert!(
            semiring(
                &[0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, f64::INFINITY],
                &f64::min,
                &|x, y| x + y,
                f64::INFINITY,
                0.0,
            )
            .is_ok()
        );

        // Test max plus semiring. + is max and x is plus.
        assert!(
            semiring(
                &[0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, f64::NEG_INFINITY],
                &f64::max,
                &|x, y| x + y,
                f64::NEG_INFINITY,
                0.0,
            )
            .is_ok()
        );

        // Test sets of strings semiring with union as + and concatenation as x
        assert!(
            semiring(
                &[
                    HashSet::from([]),
                    HashSet::from(["".to_owned()]),
                    HashSet::from(["a".to_owned()]),
                    HashSet::from(["aa".to_owned(), "bb".to_owned()]),
                    HashSet::from(["ab".to_owned(), "bb".to_owned(), "cc".to_owned()]),
                    HashSet::from(["ba".to_owned()]),
                    HashSet::from(["bb".to_owned()]),
                ],
                &|x, y| x.union(&y).cloned().collect(),
                &|x, y| {
                    let mut new_set = HashSet::new();

                    #[expect(
                        clippy::disallowed_methods,
                        reason = "nondeterministic iteration order, fine to collect into set"
                    )]
                    for a in x.iter() {
                        for b in y.iter() {
                            new_set.insert(format!("{a}{b}"));
                        }
                    }
                    new_set
                },
                HashSet::from([]),
                HashSet::from(["".to_owned()]),
            )
            .is_ok()
        );
    }

    #[test]
    fn test_get_single_function_properties() {
        // Test that get single function properties on addition returns associative, commutative, identity, and inverses.
        let test_properties_satisfied = get_single_function_properties(
            TEST_ITEMS,
            u32::wrapping_add,
            0,
            |x| 0u32.wrapping_sub(x),
            0,
        );
        let correct_properties = vec![
            "associativity".to_owned(),
            "commutativity".to_owned(),
            "identity".to_owned(),
            "inverse".to_owned(),
        ];
        assert_eq!(test_properties_satisfied, correct_properties);

        // Test that get single function properties on max returns associative, commutative, idempotent, identity, and absorbing element.
        let test_properties_satisfied = get_single_function_properties(
            &[0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, f64::INFINITY],
            f64::max,
            0.0,
            |x| x,
            f64::INFINITY,
        );
        let correct_properties = vec![
            "associativity".to_owned(),
            "commutativity".to_owned(),
            "idempotency".to_owned(),
            "identity".to_owned(),
            "absorbing_element".to_owned(),
        ];
        assert_eq!(test_properties_satisfied, correct_properties);

        // Define a function that takes in two u32s and returns the first one
        let f = |x: u32, _y: u32| x;
        let test_properties_satisfied =
            get_single_function_properties(TEST_ITEMS, f, 0, |x| 0u32.wrapping_sub(x), 0);
        let correct_properties = vec!["associativity".to_owned(), "idempotency".to_owned()];
        assert_eq!(test_properties_satisfied, correct_properties);
    }
}
