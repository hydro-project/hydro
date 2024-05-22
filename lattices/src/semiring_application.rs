//! Module containing the [`BinaryTrust`] applications of semirings.

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
    fn zero(&self) -> BinaryTrust {
        BinaryTrust(false)
    }
}

/// Implementation of Identity for multiplication.
impl One<bool> for BinaryTrust {
    /// True is the identity element for multiplication operation.
    fn one(&self) -> BinaryTrust {
        BinaryTrust(true)
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
    fn zero(&self) -> Multiplicity {
        Multiplicity(0)
    }
}

/// Implementation of Identity for multiplication.
impl One<u32> for Multiplicity {
    /// 1 is the one element of the semiring.
    fn one(&self) -> Multiplicity {
        Multiplicity(1)
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
#[derive(Clone, Copy, PartialEq, Debug)]
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
    fn add(&mut self, other: Self) {
        match (self.0, other.0) {
            (U32WithInfinity::Infinity, _) => self.0 = other.0,
            (_, U32WithInfinity::Infinity) => (),
            (U32WithInfinity::Finite(a), U32WithInfinity::Finite(b)) => {
                if a < b {
                    self.0 = U32WithInfinity::Finite(a);
                } else {
                    self.0 = U32WithInfinity::Finite(b);
                }
            }
        }
    }
}

/// Implementation of the multiplication trait for Cost semiring.
impl Multiplication<Cost> for Cost {
    /// + operation
    fn mul(&mut self, other: Cost) {
        match (self.0, other.0) {
            (U32WithInfinity::Infinity, _) | (_, U32WithInfinity::Infinity) => {
                self.0 = U32WithInfinity::Infinity;
            }
            (U32WithInfinity::Finite(a), U32WithInfinity::Finite(b)) => {
                self.0 = U32WithInfinity::Finite(a + b);
            }
        }
    }
}

/// Implementation of Identity for addition.
impl Zero<U32WithInfinity> for Cost {
    /// Infinity is the identity element for addition operation.
    fn zero(&self) -> Cost {
        Cost(U32WithInfinity::Infinity)
    }
}

/// Implementation of Identity for multiplication.
impl One<U32WithInfinity> for Cost {
    /// 0 is the one element of the semiring.
    fn one(&self) -> Cost {
        Cost(U32WithInfinity::Finite(0))
    }
}

#[allow(dead_code)]
/// use Tropical Semiring implementation for calculating All Pair Shortest Path in a graph with positive edge weights.
/// P(x,y) = shortest path from x to y
/// E(x,y) = edge weight from x to y
/// P(x,y) = min(E(x,y), min z (P(x,z) + E(z,y)) where (min,+) are the “addition” and “multiplication” operators in the tropical semirings
fn all_pairs_shortest_path(graph: Vec<Vec<U32WithInfinity>>) -> Vec<Vec<u32>> {
    let n = graph.len();
    let mut dist: Vec<Vec<Cost>> = vec![vec![Cost::new(U32WithInfinity::Infinity); n]; n];
    for i in 0..n {
        for j in 0..n {
            dist[i][j] = Cost::new(graph[i][j]);
        }
    }
    for i in 0..n {
        for j in 0..n {
            for k in 0..n {
                // distance[i][j] = min(distance[i][j], distance[i][k] + distance[k][j])
                let mut new_cost = dist[i][k];
                new_cost.mul(dist[k][j]);
                dist[i][j].add(new_cost);
            }
        }
    }

    // Convert the Cost elements into U32WithInfinity
    let mut result: Vec<Vec<u32>> = vec![vec![0; n]; n];
    for i in 0..n {
        for j in 0..n {
            match dist[i][j].0 {
                // if infinity, don't change the value
                U32WithInfinity::Infinity => result[i][j] = u32::MAX,
                U32WithInfinity::Finite(x) => result[i][j] = x,
            }
        }
    }
    result
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
        self.0 *= other.0
    }
}

/// Implementation of Identity for addition.
impl Zero<f64> for ConfidenceScore {
    /// 0 is the zero element of the semiring.
    fn zero(&self) -> ConfidenceScore {
        ConfidenceScore(0.0)
    }
}

/// Implementation of Identity for multiplication.
impl One<f64> for ConfidenceScore {
    /// 1 is the one element of the semiring.
    fn one(&self) -> ConfidenceScore {
        ConfidenceScore(1.0)
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
    fn zero(&self) -> FuzzyLogic {
        FuzzyLogic(0.0)
    }
}

/// Implementation of Identity for multiplication.
impl One<f64> for FuzzyLogic {
    /// 1 is the one element of the semiring.
    fn one(&self) -> FuzzyLogic {
        FuzzyLogic(1.0)
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
        assert!(binary_trust.0);
        binary_trust = BinaryTrust::new();
        binary_trust.add(BinaryTrust(false));
        assert!(binary_trust.0);

        // Test multiplication (AND)
        binary_trust = BinaryTrust::new();
        binary_trust.mul(BinaryTrust(true));
        assert!(binary_trust.0);
        binary_trust = BinaryTrust::new();
        binary_trust.mul(BinaryTrust(false));
        assert!(!binary_trust.0);
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
    }

    // Test for Cost/Tropical semiring
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

        cost = Cost::new(U32WithInfinity::Finite(7));
        cost.mul(Cost::new(U32WithInfinity::Finite(3)));
        assert_eq!(cost.0, U32WithInfinity::Finite(10));
        cost.add(Cost::new(U32WithInfinity::Finite(5)));
        assert_eq!(cost.0, U32WithInfinity::Finite(5));
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
    }

    #[test]
    fn test_all_pairs_shortest_path_1() {
        let inf = U32WithInfinity::Infinity;
        let c0 = U32WithInfinity::Finite(0);
        let c1 = U32WithInfinity::Finite(1);
        let c2 = U32WithInfinity::Finite(2);
        let c3 = U32WithInfinity::Finite(3);
        let c4 = U32WithInfinity::Finite(4);
        let c5 = U32WithInfinity::Finite(5);
        let c6 = U32WithInfinity::Finite(6);

        // Graph in matrix form:
        //     A  B  C  D  E
        // A [0, 4, ∞, 5, ∞]
        // B [∞, 0, 1, ∞, 6]
        // C [2, ∞, 0, 3, ∞]
        // D [∞, ∞, 1, 0, 2]
        // E [1, ∞, ∞, 4, 0]

        // Graph :https://www.geeksforgeeks.org/floyd-warshall-algorithm-dp-16/
        let graph: Vec<Vec<U32WithInfinity>> = vec![
            vec![c0, c4, inf, c5, inf],
            vec![inf, c0, c1, inf, c6],
            vec![c2, inf, c0, c3, inf],
            vec![inf, inf, c1, c0, c2],
            vec![c1, inf, inf, c4, c0],
        ];

        let result = all_pairs_shortest_path(graph);

        let expected = vec![
            vec![0, 4, 5, 5, 7],
            vec![3, 0, 1, 4, 6],
            vec![2, 6, 0, 3, 5],
            vec![3, 7, 1, 0, 2],
            vec![1, 5, 5, 4, 0],
        ];

        assert_eq!(result, expected);
    }

    #[test]
    fn test_all_pairs_shortest_path_2() {
        let inf = U32WithInfinity::Infinity;
        let c0 = U32WithInfinity::Finite(0);
        let c1 = U32WithInfinity::Finite(1);
        let c2 = U32WithInfinity::Finite(2);
        let c4 = U32WithInfinity::Finite(4);
        let c8 = U32WithInfinity::Finite(8);
        let c9 = U32WithInfinity::Finite(9);

        // Graph in matrix form:
        //     A  B  C  D
        // A [0, 8, ∞, 1]
        // B [∞, 0, 1, ∞]
        // C [4, ∞, 0, ∞]
        // D [∞, 2, 9, 0]

        // Graph: https://www.tutorialspoint.com/all-pairs-shortest-paths

        let graph: Vec<Vec<U32WithInfinity>> = vec![
            vec![c0, c8, inf, c1],
            vec![inf, c0, c1, inf],
            vec![c4, inf, c0, inf],
            vec![inf, c2, c9, c0],
        ];

        let result = all_pairs_shortest_path(graph);

        let expected = vec![
            vec![0, 3, 4, 1],
            vec![5, 0, 1, 6],
            vec![4, 7, 0, 5],
            vec![7, 2, 3, 0],
        ];

        assert_eq!(result, expected);
    }

    #[test]
    fn test_all_pairs_shortest_path_3() {
        let inf = U32WithInfinity::Infinity;
        let c0 = U32WithInfinity::Finite(0);
        let c2 = U32WithInfinity::Finite(2);
        let c3 = U32WithInfinity::Finite(3);
        let c5 = U32WithInfinity::Finite(5);
        let c8 = U32WithInfinity::Finite(8);

        // Graph in matrix form:
        //     A  B  C  D
        // A [0, 5, 8, ∞]
        // B [5, 0, ∞, 2]
        // C [8, ∞, 0, 3]
        // D [∞, 2, 3, 0]

        // Graph: https://www.altcademy.com/blog/calculate-the-all-pairs-shortest-paths-in-a-graph/
        let graph: Vec<Vec<U32WithInfinity>> = vec![
            vec![c0, c5, c8, inf], // A
            vec![c5, c0, inf, c2], // B
            vec![c8, inf, c0, c3], // C
            vec![inf, c2, c3, c0], // D
        ];

        let result = all_pairs_shortest_path(graph);

        let expected = vec![
            vec![0, 5, 8, 7],
            vec![5, 0, 5, 2],
            vec![8, 5, 0, 3],
            vec![7, 2, 3, 0],
        ];
        assert_eq!(result, expected);
    }
}
