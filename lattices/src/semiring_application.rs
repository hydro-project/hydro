use crate::{Addition, Multiplication, One, Zero};

/// Binary Trust semiring.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BinaryTrust(bool, bool);

/// Implementation of the Binary Trust semiring ({0,1}, or, and, false, true)
impl BinaryTrust {
    /// Create a new 'Binary Trust' semiring instance.
    pub fn new() -> Self {
        Self(true, false)
    }
}

/// Implementation of the addition trait for the Binary Trust semiring.
impl Addition<bool> for BinaryTrust {
    /// Adds two elements of the semiring (OR operation)
    fn add(&self, input_1: &bool, input_2: &bool) -> bool {
        if *input_1 == true {
            true
        } else {
            *input_2
        }
    }
}

/// Implementation of the multiplication trait for the Binary Trust semiring.
impl Multiplication<bool> for BinaryTrust {
    /// Multiplies two elements of the semiring (AND operation)
    fn mul(&self, input_1: &bool, input_2: &bool) -> bool {
        if *input_1 == true && *input_2 == true {
            true
        } else {
            false
        }
    }
}

/// Implementation of the Zero trait for the Binary Trust semiring.
impl Zero<bool> for BinaryTrust {
    /// Returns the zero element of the semiring.
    fn zero(&self) -> bool {
        false
    }
}

/// Implementation of the One trait for the Binary Trust semiring.
impl One<bool> for BinaryTrust {
    /// Returns the one element of the semiring.
    fn one(&self) -> bool {
        true
    }
}

/// Implementation of the Multiplicity semiring (N, +, *, 0, 1)
pub struct Multiplicity(u32);
/// Implementation of the Multiplicity semiring.
impl Multiplicity {
    /// Create a new instance of Multiplicity with the given value.
    pub fn new(value: u32) -> Self {
        Multiplicity(value)
    }
}

/// Implementation of the addition trait for Multiplicity semiring.
impl Addition<u32> for Multiplicity {
    /// Adds two elements of the semiring.
    fn add(&self, input_1: &u32, input_2: &u32) -> u32 {
        input_1.wrapping_add(*input_2)
    }
}

/// Implementation of the multiplication trait for Multiplicity semiring.
impl Multiplication<u32> for Multiplicity {
    /// Multiplies two elements of the semiring.
    fn mul(&self, input_1: &u32, input_2: &u32) -> u32 {
        input_1.wrapping_mul(*input_2)
    }
}

/// Implementation of the Zero trait for Multiplicity semiring.
impl Zero<u32> for Multiplicity {
    /// Returns the zero element of the semiring.
    fn zero(&self) -> u32 {
        0
    }
}

/// Implementation of the One trait for Multiplicity semiring.
impl One<u32> for Multiplicity {
    /// Returns the one element of the semiring.
    fn one(&self) -> u32 {
        1
    }
}

/// Implementation of the Cost/Tropical semiring ([0, inf], min, +, inf, 0)
pub struct Cost(u32);

impl Cost {
    /// Create a new instance of Cost with the given value.
    pub fn new(value: u32) -> Self {
        Cost(value)
    }
}

/// Implementation of the addition trait for Cost semiring.
impl Addition<u32> for Cost {
    /// Adds two elements of the semiring.
    fn add(&self, input_1: &u32, input_2: &u32) -> u32 {
        std::cmp::min(*input_1, *input_2)
    }
}

/// Implementation of the multiplication trait for Cost semiring.
impl Multiplication<u32> for Cost {
    /// Multiplies two elements of the semiring.
    fn mul(&self, input_1: &u32, input_2: &u32) -> u32 {
        input_1.wrapping_add(*input_2)
    }
}

/// Implementation of the Zero trait for Cost semiring.
impl Zero<u32> for Cost {
    /// Returns the zero element of the semiring.
    fn zero(&self) -> u32 {
        u32::MAX
    }
}

/// Implementation of the One trait for Cost semiring.
impl One<u32> for Cost {
    /// Returns the one element of the semiring.
    fn one(&self) -> u32 {
        0
    }
}

/// Implementation of the confidence Score semiring ([0, 1], max, *, 0, 1)
pub struct ConfidenceScore(f64);

impl ConfidenceScore {
    /// Create a new instance of ConfidenceScore with the given value.
    pub fn new(value: f64) -> Self {
        // Ensure the value is within the range [0, 1]
        let clamped_value = value.clamp(0.0, 1.0);
        ConfidenceScore(clamped_value)
    }
}

/// Implementation of the addition trait for ConfidenceScore semiring.
impl Addition<f64> for ConfidenceScore {
    /// Adds two elements of the semiring.
    fn add(&self, input_1: &f64, input_2: &f64) -> f64 {
        input_1.max(*input_2)
    }
}

/// Implementation of the multiplication trait for ConfidenceScore semiring.
impl Multiplication<f64> for ConfidenceScore {
    /// Multiplies two elements of the semiring.
    fn mul(&self, input_1: &f64, input_2: &f64) -> f64 {
        *input_1 * *input_2
    }
}

/// Implementation of the Zero trait for ConfidenceScore semiring.
impl Zero<f64> for ConfidenceScore {
    /// Returns the zero element of the semiring.
    fn zero(&self) -> f64 {
        0.0
    }
}

/// Implementation of the One trait for ConfidenceScore semiring.
impl One<f64> for ConfidenceScore {
    /// Returns the one element of the semiring.
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
        let clamped_value = value.clamp(0.0, 1.0);
        FuzzyLogic(clamped_value)
    }
}

/// Implementation of the addition trait for FuzzyLogic semiring.
impl Addition<f64> for FuzzyLogic {
    /// Adds two elements of the semiring.
    fn add(&self, input_1: &f64, input_2: &f64) -> f64 {
        input_1.max(*input_2)
    }
}

/// Implementation of the multiplication trait for FuzzyLogic semiring.
impl Multiplication<f64> for FuzzyLogic {
    /// Multiplies two elements of the semiring.
    fn mul(&self, input_1: &f64, input_2: &f64) -> f64 {
        input_1.min(*input_2)
    }
}

/// Implementation of the Zero trait for FuzzyLogic semiring.
impl Zero<f64> for FuzzyLogic {
    /// Returns the zero element of the semiring.
    fn zero(&self) -> f64 {
        0.0
    }
}

/// Implementation of the One trait for FuzzyLogic semiring.
impl One<f64> for FuzzyLogic {
    /// Returns the one element of the semiring.
    fn one(&self) -> f64 {
        1.0
    }
}
