//! Module containing the [`BinaryTrust`] applications.

use std::cmp;

use crate::{Addition, Multiplication, One, Zero};
#[derive(PartialEq, Debug)]
// #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]

/// Implementation of the Binary Trust semiring ({0,1}, OR, AND, False, True)
pub struct BinaryTrust(bool, bool);

impl BinaryTrust {
    /// Create a new 'Binary Trust' semiring instance.
    pub fn new() -> Self {
        Self(true, false)
    }
}

/// Implementation of the addition trait for the Binary Trust semiring.
impl Addition<bool> for BinaryTrust {
    /// OR operation
    fn add(&self, input_1: &bool, input_2: &bool) -> bool {
        *input_1 || *input_2
    }
}

/// Implementation of the multiplication trait for the Binary Trust semiring.
impl Multiplication<bool> for BinaryTrust {
    /// AND operation
    fn mul(&self, input_1: &bool, input_2: &bool) -> bool {
        *input_1 && *input_2
    }
}

/// Implementation of Identity for addition.
impl Zero<bool> for BinaryTrust {
    /// False is the identity element for addition operation.
    fn zero(&self) -> bool {
        false
    }
}

/// Implementation of Identity for multiplication.
impl One<bool> for BinaryTrust {
    /// True is the identity element for multiplication operation.
    fn one(&self) -> bool {
        true
    }
}

/// Implementation of the Multiplicity semiring (N, +, *, 0, 1)
pub struct Multiplicity(u32);

impl Multiplicity {
    /// Create a new instance of Multiplicity.
    pub fn new(value: u32) -> Self {
        Multiplicity(value)
    }
}

/// Implementation of the addition trait for Multiplicity semiring.
impl Addition<u32> for Multiplicity {
    /// + operation
    fn add(&self, input_1: &u32, input_2: &u32) -> u32 {
        input_1.checked_add(*input_2).unwrap()
    }
}

/// Implementation of the multiplication trait for Multiplicity semiring.
impl Multiplication<u32> for Multiplicity {
    /// * operation
    fn mul(&self, input_1: &u32, input_2: &u32) -> u32 {
        input_1.checked_mul(*input_2).unwrap()
    }
}

/// Implementation of Identity for addition.
impl Zero<u32> for Multiplicity {
    /// 0 is the zero element of the semiring.  
    fn zero(&self) -> u32 {
        0
    }
}

/// Implementation of Identity for multiplication.
impl One<u32> for Multiplicity {
    /// 1 is the one element of the semiring.
    fn one(&self) -> u32 {
        1
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]

enum U32WithInfinity {
    Infinity,
    Finite(u32),
}

/// Implementation of the Cost/Tropical semiring (N U Inf, min, +, inf, 0)
pub struct Cost(U32WithInfinity);

impl Cost {
    /// Create a new instance of Cost.
    pub fn new(value: U32WithInfinity) -> Self {
        Cost(value)
    }
}

/// Implementation of the addition trait for Cost semiring.
impl Addition<U32WithInfinity> for Cost {
    /// Min operation
    fn add(&self, input_1: &U32WithInfinity, input_2: &U32WithInfinity) -> U32WithInfinity {
        match (input_1, input_2) {
            (U32WithInfinity::Finite(a), U32WithInfinity::Finite(b)) => {
                U32WithInfinity::Finite(cmp::min(*a, *b))
            }
            (U32WithInfinity::Infinity, U32WithInfinity::Finite(b)) => U32WithInfinity::Finite(*b),
            (U32WithInfinity::Finite(a), U32WithInfinity::Infinity) => U32WithInfinity::Finite(*a),
            (U32WithInfinity::Infinity, U32WithInfinity::Infinity) => U32WithInfinity::Infinity,
        }
    }
}

/// Implementation of the multiplication trait for Cost semiring.
impl Multiplication<U32WithInfinity> for Cost {
    /// + operation
    fn mul(&self, input_1: &U32WithInfinity, input_2: &U32WithInfinity) -> U32WithInfinity {
        match (input_1, input_2) {
            (U32WithInfinity::Infinity, _) => U32WithInfinity::Infinity,
            (_, U32WithInfinity::Infinity) => U32WithInfinity::Infinity,
            (U32WithInfinity::Finite(a), U32WithInfinity::Finite(b)) => {
                U32WithInfinity::Finite(a.checked_add(*b).unwrap())
            }
        }
    }
}

/// Implementation of Identity for addition.
impl Zero<U32WithInfinity> for Cost {
    /// Infinity is the identity element for addition operation.
    fn zero(&self) -> U32WithInfinity {
        U32WithInfinity::Infinity
    }
}

/// Implementation of Identity for multiplication.
impl One<U32WithInfinity> for Cost {
    /// 0 is the one element of the semiring.
    fn one(&self) -> U32WithInfinity {
        U32WithInfinity::Finite(0)
    }
}

/// Implementation of the confidence Score semiring ([0, 1], max, *, 0, 1)
pub struct ConfidenceScore(f64);

impl ConfidenceScore {
    /// Create a new instance of ConfidenceScore with the given value.
    pub fn new(value: f64) -> Self {
        // Ensure the value is within the range [0, 1]
        (0.0..=1.0).contains(&value);
        ConfidenceScore(value)
    }
}

/// Implementation of the addition trait for ConfidenceScore semiring.
impl Addition<f64> for ConfidenceScore {
    /// Maximum is the addition operation for ConfidenceScore semiring.
    fn add(&self, input_1: &f64, input_2: &f64) -> f64 {
        f64::max(*input_1, *input_2)
    }
}

/// Implementation of the multiplication trait for ConfidenceScore semiring.
impl Multiplication<f64> for ConfidenceScore {
    /// Multiplication is the multiplication operation for ConfidenceScore semiring.
    fn mul(&self, input_1: &f64, input_2: &f64) -> f64 {
        *input_1 * *input_2
    }
}

/// Implementation of Identity for addition.
impl Zero<f64> for ConfidenceScore {
    /// 0 is the zero element of the semiring.
    fn zero(&self) -> f64 {
        0.0
    }
}

/// Implementation of Identity for multiplication.
impl One<f64> for ConfidenceScore {
    /// 1 is the one element of the semiring.
    fn one(&self) -> f64 {
        1.0
    }
}

/// Implementation of Fuzzy Logic semiring ([0, 1], max, min, 0, 1).
pub struct FuzzyLogic(f64);

impl FuzzyLogic {
    /// Create a new instance of FuzzyLogic with the given value.
    pub fn new(value: f64) -> Self {
        // Ensure the value is within the range [0, 1]
        (0.0..=1.0).contains(&value);
        FuzzyLogic(value)
    }
}

/// Implementation of the addition trait for FuzzyLogic semiring.
impl Addition<f64> for FuzzyLogic {
    /// Maximum is the addition operation for FuzzyLogic semiring.
    fn add(&self, input_1: &f64, input_2: &f64) -> f64 {
        f64::max(*input_1, *input_2)
    }
}

/// Implementation of the multiplication trait for FuzzyLogic semiring.
impl Multiplication<f64> for FuzzyLogic {
    /// Minimum is the multiplication operation for FuzzyLogic semiring.
    fn mul(&self, input_1: &f64, input_2: &f64) -> f64 {
        f64::min(*input_1, *input_2)
    }
}

/// Implementation of Identity for addition.
impl Zero<f64> for FuzzyLogic {
    /// 0 is the zero element of the semiring.
    fn zero(&self) -> f64 {
        0.0
    }
}

/// Implementation of Identity for multiplication.
impl One<f64> for FuzzyLogic {
    /// 1 is the one element of the semiring.
    fn one(&self) -> f64 {
        1.0
    }
}

#[cfg(test)]
mod test {

    // use crate::semiring_application::{BinaryTrust, Multiplicity, Cost, ConfidenceScore, FuzzyLogic};
    use super::*;

    // write test for binary trust ({0,1}, OR, AND, False, True)
    #[test]
    fn test_binary_trust() {
        let binary_trust: BinaryTrust = BinaryTrust::new();

        // test addition (OR)
        assert_eq!(binary_trust.add(&true, &false), true);
        assert_eq!(binary_trust.add(&true, &true), true);
        assert_eq!(binary_trust.add(&false, &false), false);

        // test multiplication (AND)
        assert_eq!(binary_trust.mul(&true, &false), false);
        assert_eq!(binary_trust.mul(&true, &true), true);
        assert_eq!(binary_trust.mul(&false, &false), false);

        // test identity element for addition (False)
        assert_eq!(binary_trust.zero(), false);
        assert_eq!(binary_trust.add(&binary_trust.zero(), &false), false);
        assert_eq!(binary_trust.add(&binary_trust.zero(), &true), true);

        // test identity element for multiplication (TRUE)
        assert_eq!(binary_trust.one(), true);
        assert_eq!(binary_trust.mul(&binary_trust.one(), &true), true);
        assert_eq!(binary_trust.mul(&binary_trust.one(), &false), false);
    }

    // write test for multiplicity (N, +, *, 0, 1)
    #[test]
    fn test_multiplicity() {
        let multiplicity: Multiplicity = Multiplicity::new(5);

        // test addition (+)
        assert_eq!(multiplicity.add(&5, &10), 15);
        assert_eq!(multiplicity.add(&5, &0), 5);
        assert_eq!(multiplicity.add(&0, &0), 0);

        // test multiplication (*)
        assert_eq!(multiplicity.mul(&5, &10), 50);
        assert_eq!(multiplicity.mul(&10, &5), 50);
        assert_eq!(multiplicity.mul(&5, &0), 0);
        assert_eq!(multiplicity.mul(&0, &0), 0);

        // test identity element for addition (0)
        assert_eq!(multiplicity.zero(), 0);
        assert_eq!(multiplicity.add(&multiplicity.zero(), &5), 5);
        assert_eq!(multiplicity.add(&multiplicity.zero(), &0), 0);

        // test identity element for multiplication (1)
        assert_eq!(multiplicity.one(), 1);
        assert_eq!(multiplicity.mul(&multiplicity.one(), &5), 5);
        assert_eq!(multiplicity.mul(&multiplicity.one(), &0), 0);
    }

    // write test for cost/tropical semiring (N U Inf, min, +, inf, 0)
    #[test]
    fn test_cost() {
        let cost: Cost = Cost::new(U32WithInfinity::Finite(5));

        // test addition (min)
        assert_eq!(
            cost.add(&U32WithInfinity::Finite(5), &U32WithInfinity::Finite(10)),
            U32WithInfinity::Finite(5)
        );
        assert_eq!(
            cost.add(&U32WithInfinity::Finite(5), &U32WithInfinity::Infinity),
            U32WithInfinity::Finite(5)
        );
        assert_eq!(
            cost.add(&U32WithInfinity::Infinity, &U32WithInfinity::Finite(5)),
            U32WithInfinity::Finite(5)
        );
        assert_eq!(
            cost.add(&U32WithInfinity::Infinity, &U32WithInfinity::Infinity),
            U32WithInfinity::Infinity
        );

        // test multiplication (+)
        assert_eq!(
            cost.mul(&U32WithInfinity::Finite(5), &U32WithInfinity::Finite(10)),
            U32WithInfinity::Finite(15)
        );
        assert_eq!(
            cost.mul(&U32WithInfinity::Finite(5), &U32WithInfinity::Infinity),
            U32WithInfinity::Infinity
        );
        assert_eq!(
            cost.mul(&U32WithInfinity::Infinity, &U32WithInfinity::Finite(5)),
            U32WithInfinity::Infinity
        );
        assert_eq!(
            cost.mul(&U32WithInfinity::Infinity, &U32WithInfinity::Infinity),
            U32WithInfinity::Infinity
        );

        // test identity element for addition (Infinity)
        assert_eq!(cost.zero(), U32WithInfinity::Infinity);
        assert_eq!(
            cost.add(&cost.zero(), &U32WithInfinity::Finite(5)),
            U32WithInfinity::Finite(5)
        );
        assert_eq!(
            cost.add(&cost.zero(), &U32WithInfinity::Infinity),
            U32WithInfinity::Infinity
        );

        // test identity element for multiplication (0)
        assert_eq!(cost.one(), U32WithInfinity::Finite(0));
        assert_eq!(
            cost.mul(&cost.one(), &U32WithInfinity::Finite(5)),
            U32WithInfinity::Finite(5)
        );
        assert_eq!(
            cost.mul(&cost.one(), &U32WithInfinity::Infinity),
            U32WithInfinity::Infinity
        );
    }

    // write test for confidence score ([0, 1], max, *, 0, 1)
    #[test]
    fn test_confidence_score() {
        let confidence_score: ConfidenceScore = ConfidenceScore::new(0.5);

        // test addition (max)
        assert_eq!(confidence_score.add(&0.5, &0.6), 0.6);
        assert_eq!(confidence_score.add(&0.5, &0.4), 0.5);
        assert_eq!(confidence_score.add(&0.5, &0.5), 0.5);

        // test multiplication (*)
        assert_eq!(confidence_score.mul(&0.5, &0.6), 0.3);
        assert_eq!(confidence_score.mul(&0.5, &0.4), 0.2);
        assert_eq!(confidence_score.mul(&0.5, &0.5), 0.25);

        // test identity element for addition (0)
        assert_eq!(confidence_score.zero(), 0.0);
        assert_eq!(confidence_score.add(&confidence_score.zero(), &0.5), 0.5);
        assert_eq!(confidence_score.add(&confidence_score.zero(), &0.0), 0.0);

        // test identity element for multiplication (1)
        assert_eq!(confidence_score.one(), 1.0);
        assert_eq!(confidence_score.mul(&confidence_score.one(), &0.5), 0.5);
        assert_eq!(confidence_score.mul(&confidence_score.one(), &0.0), 0.0);
    }
}
