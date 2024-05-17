//! Module containing the [`BinaryTrust`] applications.

use crate::{Addition, Multiplication, One, Zero};
#[derive(PartialEq, Debug)]
// #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]

/// Implementation of the Binary Trust semiring ({0,1}, OR, AND, False, True)
pub struct BinaryTrust(bool);

impl BinaryTrust {
    /// Create a new 'Binary Trust' semiring instance.
    pub fn new() -> Self {
        Self(true)
    }
}

impl Default for BinaryTrust {
    fn default() -> Self {
        Self::new()
    }
}

/// Implementation of the addition trait for the Binary Trust semiring.
impl Addition<BinaryTrust> for BinaryTrust {
    /// OR operation
    fn add(&mut self, other: BinaryTrust) {
        self.0 = self.0 || other.0;
    }
}

/// Implementation of the multiplication trait for the Binary Trust semiring.
impl Multiplication<BinaryTrust> for BinaryTrust {
    /// AND operation
    fn mul(&mut self, other: BinaryTrust) {
        self.0 = self.0 && other.0;
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

#[allow(dead_code)]
/// Implementation of the Multiplicity semiring (N, +, *, 0, 1)
pub struct Multiplicity(u32);

impl Multiplicity {
    /// Create a new instance of Multiplicity.
    pub fn new(value: u32) -> Self {
        Multiplicity(value)
    }
}

/// Implementation of the addition trait for Multiplicity semiring.
impl Addition<Multiplicity> for Multiplicity {
    /// + operation
    fn add(&mut self, other: Multiplicity) {
        self.0 = self.0.checked_add(other.0).unwrap();
    }
}

/// Implementation of the multiplication trait for Multiplicity semiring.
impl Multiplication<Multiplicity> for Multiplicity {
    /// * operation
    fn mul(&mut self, other: Multiplicity) {
        self.0 = self.0.checked_mul(other.0).unwrap();
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
/// Implementation for N U Inf
pub enum U32WithInfinity {
    /// Infinity
    Infinity,
    /// Natural numbers
    Finite(u32),
}

#[allow(dead_code)]
/// Implementation of the Cost/Tropical semiring (N U Inf, min, +, inf, 0)
pub struct Cost(U32WithInfinity);

impl Cost {
    /// Create a new instance of Cost.
    pub fn new(value: U32WithInfinity) -> Self {
        Cost(value)
    }
}

/// Implementation of the addition trait for Cost semiring.
impl Addition<Cost> for Cost {
    /// Min operation
    fn add(&mut self, other: Cost) {
        self.0 = match (self.0, other.0) {
            (U32WithInfinity::Infinity, x) | (x, U32WithInfinity::Infinity) => x,
            (U32WithInfinity::Finite(a), U32WithInfinity::Finite(b)) => {
                U32WithInfinity::Finite(a.min(b))
            }
        };
    }
}

/// Implementation of the multiplication trait for Cost semiring.
impl Multiplication<Cost> for Cost {
    /// + operation
    fn mul(&mut self, other: Cost) {
        self.0 = match (self.0, other.0) {
            (U32WithInfinity::Infinity, _) | (_, U32WithInfinity::Infinity) => {
                U32WithInfinity::Infinity
            }
            (U32WithInfinity::Finite(a), U32WithInfinity::Finite(b)) => {
                U32WithInfinity::Finite(a + b)
            }
        };
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

#[allow(dead_code)]
/// Implementation of the confidence Score semiring ([0, 1], max, *, 0, 1)
pub struct ConfidenceScore(f64);

impl ConfidenceScore {
    /// Create a new instance of ConfidenceScore with the given value.
    pub fn new(value: f64) -> Self {
        // Ensure the value is within the range [0, 1]
        assert!((0.0..=1.0).contains(&value));
        ConfidenceScore(value)
    }
}

/// Implementation of the addition trait for ConfidenceScore semiring.
impl Addition<ConfidenceScore> for ConfidenceScore {
    /// Maximum is the addition operation for ConfidenceScore semiring.
    fn add(&mut self, other: ConfidenceScore) {
        self.0 = f64::max(self.0, other.0);
    }
}

/// Implementation of the multiplication trait for ConfidenceScore semiring.
impl Multiplication<ConfidenceScore> for ConfidenceScore {
    /// Multiplication is the multiplication operation for ConfidenceScore semiring.
    fn mul(&mut self, other: ConfidenceScore) {
        self.0 *= other.0;
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

#[allow(dead_code)]
/// Implementation of Fuzzy Logic semiring ([0, 1], max, min, 0, 1).
pub struct FuzzyLogic(f64);

impl FuzzyLogic {
    /// Create a new instance of FuzzyLogic with the given value.
    pub fn new(value: f64) -> Self {
        // Ensure the value is within the range [0, 1]
        assert!((0.0..=1.0).contains(&value));
        FuzzyLogic(value)
    }
}

/// Implementation of the addition trait for FuzzyLogic semiring.
impl Addition<FuzzyLogic> for FuzzyLogic {
    /// Maximum is the addition operation for FuzzyLogic semiring.
    fn add(&mut self, other: FuzzyLogic) {
        self.0 = f64::max(self.0, other.0);
    }
}

/// Implementation of the multiplication trait for FuzzyLogic semiring.
impl Multiplication<FuzzyLogic> for FuzzyLogic {
    /// Minimum is the multiplication operation for FuzzyLogic semiring.
    fn mul(&mut self, other: FuzzyLogic) {
        self.0 = f64::min(self.0, other.0);
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

    use super::*;

    // Test for BinaryTrust ({0,1}, OR, AND, False, True)
    #[test]
    fn test_binary_trust() {
        let mut binary_trust = BinaryTrust::new();

        // Test addition (OR)
        binary_trust.add(BinaryTrust(true));
        assert_eq!(binary_trust.0, true);
        binary_trust = BinaryTrust::new();
        binary_trust.add(BinaryTrust(false));
        assert_eq!(binary_trust.0, true);

        // Test multiplication (AND)
        binary_trust = BinaryTrust::new();
        binary_trust.mul(BinaryTrust(true));
        assert_eq!(binary_trust.0, true);
        binary_trust = BinaryTrust::new();
        binary_trust.mul(BinaryTrust(false));
        assert_eq!(binary_trust.0, false);

        // Test identity element for addition (False)
        assert_eq!(BinaryTrust(false).zero(), false);

        // Test identity element for multiplication (True)
        assert_eq!(BinaryTrust(true).one(), true);
    }

    // Test for Multiplicity (N, +, *, 0, 1)
    #[test]
    fn test_multiplicity() {
        let mut multiplicity = Multiplicity::new(5);

        // Test addition (+)
        multiplicity.add(Multiplicity(10));
        assert_eq!(multiplicity.0, 15);
        multiplicity = Multiplicity::new(5);
        multiplicity.add(Multiplicity(0));
        assert_eq!(multiplicity.0, 5);

        // Test multiplication (*)
        multiplicity = Multiplicity::new(5);
        multiplicity.mul(Multiplicity(10));
        assert_eq!(multiplicity.0, 50);
        multiplicity = Multiplicity::new(10);
        multiplicity.mul(Multiplicity(5));
        assert_eq!(multiplicity.0, 50);
        multiplicity = Multiplicity::new(5);
        multiplicity.mul(Multiplicity(0));
        assert_eq!(multiplicity.0, 0);

        // Test identity element for addition (0)
        assert_eq!(multiplicity.zero(), 0);

        // Test identity element for multiplication (1)
        assert_eq!(multiplicity.one(), 1);
    }

    // Test for Cost/Tropical semiring (N U Inf, min, +, inf, 0)
    #[test]
    fn test_cost() {
        let mut cost = Cost::new(U32WithInfinity::Finite(5));

        // Test addition (min)
        cost.add(Cost::new(U32WithInfinity::Finite(10)));
        assert_eq!(cost.0, U32WithInfinity::Finite(5));
        cost = Cost::new(U32WithInfinity::Finite(5));
        cost.add(Cost::new(U32WithInfinity::Infinity));
        assert_eq!(cost.0, U32WithInfinity::Finite(5));
        cost = Cost::new(U32WithInfinity::Infinity);
        cost.add(Cost::new(U32WithInfinity::Finite(5)));
        assert_eq!(cost.0, U32WithInfinity::Finite(5));
        cost = Cost::new(U32WithInfinity::Infinity);
        cost.add(Cost::new(U32WithInfinity::Infinity));
        assert_eq!(cost.0, U32WithInfinity::Infinity);

        // Test multiplication (+)
        cost = Cost::new(U32WithInfinity::Finite(5));
        cost.mul(Cost::new(U32WithInfinity::Finite(10)));
        assert_eq!(cost.0, U32WithInfinity::Finite(15));
        cost = Cost::new(U32WithInfinity::Finite(5));
        cost.mul(Cost::new(U32WithInfinity::Infinity));
        assert_eq!(cost.0, U32WithInfinity::Infinity);
        cost = Cost::new(U32WithInfinity::Infinity);
        cost.mul(Cost::new(U32WithInfinity::Finite(5)));
        assert_eq!(cost.0, U32WithInfinity::Infinity);

        // Test identity element for addition (Infinity)
        assert_eq!(cost.zero(), U32WithInfinity::Infinity);

        // Test identity element for multiplication (0)
        assert_eq!(cost.one(), U32WithInfinity::Finite(0));
    }

    // Test for Confidence Score ([0, 1], max, *, 0, 1)
    #[test]
    fn test_confidence_score() {
        let mut confidence_score = ConfidenceScore::new(0.5);

        // Test addition (max)
        confidence_score.add(ConfidenceScore::new(0.6));
        assert_eq!(confidence_score.0, 0.6);
        confidence_score = ConfidenceScore::new(0.5);
        confidence_score.add(ConfidenceScore::new(0.4));
        assert_eq!(confidence_score.0, 0.5);

        // Test multiplication (*)
        confidence_score = ConfidenceScore::new(0.5);
        confidence_score.mul(ConfidenceScore::new(0.6));
        assert_eq!(confidence_score.0, 0.3);
        confidence_score = ConfidenceScore::new(0.5);
        confidence_score.mul(ConfidenceScore::new(0.4));
        assert_eq!(confidence_score.0, 0.2);

        // Test identity element for addition (0)
        assert_eq!(confidence_score.zero(), 0.0);

        // Test identity element for multiplication (1)
        assert_eq!(confidence_score.one(), 1.0);
    }

    // Test for Fuzzy Logic ([0, 1], max, min, 0, 1).
    #[test]
    fn test_fuzzy_logic() {
        let mut fuzzy_logic = FuzzyLogic::new(0.5);

        // Test addition (max)
        fuzzy_logic.add(FuzzyLogic::new(0.6));
        assert_eq!(fuzzy_logic.0, 0.6);
        fuzzy_logic = FuzzyLogic::new(0.5);
        fuzzy_logic.add(FuzzyLogic::new(0.4));
        assert_eq!(fuzzy_logic.0, 0.5);

        // Test multiplication (min)
        fuzzy_logic = FuzzyLogic::new(0.5);
        fuzzy_logic.mul(FuzzyLogic::new(0.6));
        assert_eq!(fuzzy_logic.0, 0.5);
        fuzzy_logic = FuzzyLogic::new(0.5);
        fuzzy_logic.mul(FuzzyLogic::new(0.4));
        assert_eq!(fuzzy_logic.0, 0.4);

        // Test identity element for addition (0)
        assert_eq!(fuzzy_logic.zero(), 0.0);

        // Test identity element for multiplication (1)
        assert_eq!(fuzzy_logic.one(), 1.0);
    }
}
