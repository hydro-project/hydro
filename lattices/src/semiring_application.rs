//! Module containing the [`BinaryTrust`] applications of semirings.

use std::collections::HashSet;
use std::hash::Hash;

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

/// Implementation for N U Inf
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
enum U32WithInfinity {
    Finite(u32),
    Infinity,
}

#[allow(dead_code)]
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
/// Implementation of the RealNumber semiring (R, +, *, 0, 1)
pub struct RealNumber(f64);

impl RealNumber {
    /// Create a new instance of RealNumber.
    pub fn new(value: f64) -> Self {
        RealNumber(value)
    }
}

/// Implementation of the addition trait for RealNumber semiring.
impl Addition<RealNumber> for RealNumber {
    /// + operation
    fn add(&mut self, other: RealNumber) {
        self.0 += other.0
    }
}

/// Implementation of the multiplication trait for RealNumber semiring.
impl Multiplication<RealNumber> for RealNumber {
    /// * operation
    fn mul(&mut self, other: RealNumber) {
        self.0 *= other.0
    }
}

/// Implementation of Identity for addition.
impl Zero<f64> for RealNumber {
    /// 0 is the zero element of the semiring.  
    fn zero(&self) -> RealNumber {
        RealNumber(0.0)
    }
}

/// Implementation of Identity for multiplication.
impl One<f64> for RealNumber {
    /// 1 is the one element of the semiring.
    fn one(&self) -> RealNumber {
        RealNumber(1.0)
    }
}

/// Ridge Regression using RealNumber semiring
#[allow(dead_code)]
fn ridge_regression(
    a: Vec<Vec<f64>>,
    b: Vec<f64>,
    lambda: f64,
    alpha: f64,
    iterations: usize,
    tolerance: f64,
) -> Vec<f64> {
    // Convert input data to RealNumber semiring
    let a: Vec<Vec<RealNumber>> = a
        .iter()
        .map(|row| row.iter().map(|&x| RealNumber::new(x)).collect())
        .collect();
    let b: Vec<RealNumber> = b.iter().map(|&x| RealNumber::new(x)).collect();
    let lambda = RealNumber::new(lambda);
    let alpha = RealNumber::new(alpha);

    // Initialize x with zeros
    let mut x: Vec<RealNumber> = vec![RealNumber::new(0.0); a[0].len()];

    for _ in 0..iterations {
        // Reset gradient
        let mut gradient: Vec<RealNumber> = vec![RealNumber::new(0.0); a[0].len()];

        // Compute the gradient
        for i in 0..a[0].len() {
            for j in 0..a.len() {
                let mut sum = RealNumber::new(0.0);
                for (k, &item) in x.iter().enumerate().take(a[0].len()) {
                    let mut temp = a[j][k];
                    temp.mul(item);
                    sum.add(temp);
                }
                let mut negative = RealNumber::new(-1.0);
                negative.mul(b[j]);
                sum.add(negative);
                let mut a_j_i = a[j][i];
                a_j_i.mul(sum);
                gradient[i].add(a_j_i);
            }
            let mut regularization = lambda;
            regularization.mul(x[i]);
            gradient[i].add(regularization);
        }

        // Update x using the gradient and check for convergence
        let mut max_change = RealNumber::new(0.0);
        for i in 0..x.len() {
            let mut change = alpha;
            change.mul(gradient[i]);
            change.mul(RealNumber::new(-1.0));
            x[i].add(change);

            if change.0.abs() > max_change.0 {
                max_change = change;
            }
        }

        // Check if the maximum change is less than the tolerance
        if max_change.0 < tolerance {
            break;
        }
    }

    // Convert x back to Vec<f64>
    x.iter().map(|&RealNumber(value)| value).collect()
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
/// Helper function for converting adjacency graph with infinity distances
/// to set of triples with the infinity paths removed.
/// The triple is (start, destination, distance).
fn remove_infinity_from_adjacency_graph(graph: Vec<Vec<Cost>>) -> HashSet<(u32, u32, u32)> {
    let n = graph.len();
    let mut result: HashSet<(u32, u32, u32)> = HashSet::new();

    for start in 0..n {
        for dest in 0..n {
            match graph[start][dest].0 {
                U32WithInfinity::Finite(distance) => {
                    result.insert((start as u32, dest as u32, distance));
                }
                U32WithInfinity::Infinity => {
                    // Do nothing for infinity paths
                }
            }
        }
    }
    result
}

/// Helper function for converting the set of triples (start, destination, distance)
/// to an adjacency graph.
fn convert_triples_to_adjacency_graph(
    triples: Vec<(usize, usize, u32)>,
    n: usize,
) -> Vec<Vec<Cost>> {
    // Initialize the adjacency graph with infinity distances
    let mut dist: Vec<Vec<Cost>> = vec![vec![Cost(U32WithInfinity::Infinity); n]; n];

    // Initialize distances with edge weights from the triples
    for (start, end, distance) in triples {
        dist[start][end] = Cost(U32WithInfinity::Finite(distance));
    }

    // Initialize distance from each node to itself as 0
    for i in 0..n {
        dist[i][i] = Cost(U32WithInfinity::Finite(0));
    }

    dist
}

/// Use Tropical Semiring implementation for calculating All Pair Shortest Path in a graph with positive edge weights.
/// Floyd-Warshall implementaion and is identical to the APSP implementation in paper Datalog in Wonderland.
/// P(x,y) = shortest path from x to y
/// E(x,y) = edge weight from x to y
/// P(x,y) = min(E(x,y), min z (P(x,z) + E(z,y)) where (min,+) are the “addition” and “multiplication” operators in the tropical semirings
///  Input:
///     - a list of (start, end, distance) if there exists a path between start and end nodes.
/// Output
///     - a list of (start, end, shortest_distance) if there exists a path between start and end nodes.
fn all_pairs_shortest_path(edges: Vec<(usize, usize, u32)>) -> HashSet<(u32, u32, u32)> {
    // Find the maximum node index by scanning through the triples
    let max_node = edges
        .iter()
        .map(|(start, end, _)| start.max(end))
        .max()
        .unwrap_or(&0);

    // The size of the graph is max_node + 1
    let n = (max_node + 1) as usize;

    let mut graph = convert_triples_to_adjacency_graph(edges, n);

    // Apply Floyd-Warshall algorithm using the tropical semiring operations
    for k in 0..n {
        for i in 0..n {
            for j in 0..n {
                let mut new_cost = graph[i][k];
                new_cost.mul(graph[k][j]);
                graph[i][j].add(new_cost);
            }
        }
    }
    let result = remove_infinity_from_adjacency_graph(graph);
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
        // Graph in matrix form:
        //     A(0)  B(1) C(2) D(3)  E (4)
        // A [0, 4, ∞, 5, ∞]
        // B [∞, 0, 1, ∞, 6]
        // C [2, ∞, 0, 3, ∞]
        // D [∞, ∞, 1, 0, 2]
        // E [1, ∞, ∞, 4, 0]

        // This graph contains a cycle (A -> D -> E -> A) and is directed, sparse graph
        // with all pairs of vertices having path.

        let edges = vec![
            (0, 1, 4),
            (0, 3, 5),
            (1, 2, 1),
            (1, 4, 6),
            (2, 0, 2),
            (2, 3, 3),
            (3, 2, 1),
            (3, 4, 2),
            (4, 0, 1),
            (4, 3, 4),
        ];

        let result: HashSet<(u32, u32, u32)> = all_pairs_shortest_path(edges).into_iter().collect();

        let expected: HashSet<(u32, u32, u32)> = vec![
            (0, 0, 0),
            (0, 1, 4),
            (0, 2, 5),
            (0, 3, 5),
            (0, 4, 7),
            (1, 0, 3),
            (1, 1, 0),
            (1, 2, 1),
            (1, 3, 4),
            (1, 4, 6),
            (2, 0, 2),
            (2, 1, 6),
            (2, 2, 0),
            (2, 3, 3),
            (2, 4, 5),
            (3, 0, 3),
            (3, 1, 7),
            (3, 2, 1),
            (3, 3, 0),
            (3, 4, 2),
            (4, 0, 1),
            (4, 1, 5),
            (4, 2, 5),
            (4, 3, 4),
            (4, 4, 0),
        ]
        .into_iter()
        .collect();

        assert_eq!(result, expected);
    }

    #[test]
    fn test_all_pairs_shortest_path_2() {
        // Graph in matrix form:
        //    A(0) B(1) C(2)  D(3)
        // A [0, 8, ∞, 1]
        // B [∞, 0, 1, ∞]
        // C [4, ∞, 0, ∞]
        // D [∞, 2, 9, 0]

        // This graph contains a cycle (A -> B -> C -> A) and is directed, sparse graph
        // with all pairs of vertices having path.

        let edges = vec![
            (0, 1, 8),
            (0, 3, 1),
            (1, 2, 1),
            (2, 0, 4),
            (3, 1, 2),
            (3, 2, 9),
        ];

        let result = all_pairs_shortest_path(edges);

        let expected: HashSet<(u32, u32, u32)> = vec![
            (0, 0, 0),
            (0, 1, 3),
            (0, 2, 4),
            (0, 3, 1),
            (1, 0, 5),
            (1, 1, 0),
            (1, 2, 1),
            (1, 3, 6),
            (2, 0, 4),
            (2, 1, 7),
            (2, 2, 0),
            (2, 3, 5),
            (3, 0, 7),
            (3, 1, 2),
            (3, 2, 3),
            (3, 3, 0),
        ]
        .into_iter()
        .collect();

        assert_eq!(result, expected);
    }

    #[test]
    fn test_all_pairs_shortest_path_3() {
        // Graph in matrix form:
        // A → B → C
        // ↓    ↓
        // D → E
        //
        // Graph in adjacency matrix:
        //       A  B  C  D  E
        //    A [ 0, 1, ∞, 1, ∞ ]
        //    B [ ∞, 0, 1, ∞, 1 ]
        //    C [ ∞, ∞, 0, ∞, ∞ ]
        //    D [ ∞, ∞, ∞, 0, 1 ]
        //    E [ ∞, ∞, ∞, ∞, 0 ]

        // This graph doesn't have a cycle and some pairs of vercies don't have paths between them.

        let edges = vec![(0, 1, 1), (0, 3, 1), (1, 2, 1), (1, 4, 1), (3, 4, 1)];

        let result = all_pairs_shortest_path(edges);

        let expected: HashSet<(u32, u32, u32)> = vec![
            (0, 0, 0),
            (0, 1, 1),
            (0, 2, 2),
            (0, 3, 1),
            (0, 4, 2),
            (1, 1, 0),
            (1, 2, 1),
            (1, 4, 1),
            (2, 2, 0),
            (3, 3, 0),
            (3, 4, 1),
            (4, 4, 0),
        ]
        .into_iter()
        .collect();

        assert_eq!(result, expected);
    }

    #[test]
    fn test_all_pairs_shortest_path_4() {
        // Graph in matrix form:
        //     A  B  C  D  E
        // A [ 0,  1, ∞, ∞, ∞ ]
        // B [ ∞,  0, 1, ∞, ∞ ]
        // C [ ∞,  ∞, 0, 1, ∞ ]
        // D [ ∞,  ∞, ∞, 0, 1 ]
        // E [ ∞,  ∞, 1, ∞, 0 ]

        // This graph has a cycle but not all vertices have path between them.

        let edges = vec![(0, 1, 1), (1, 2, 1), (2, 3, 1), (3, 4, 1), (4, 2, 1)];

        let result = all_pairs_shortest_path(edges);

        let expected: HashSet<(u32, u32, u32)> = vec![
            (0, 0, 0),
            (0, 1, 1),
            (0, 2, 2),
            (0, 3, 3),
            (0, 4, 4),
            (1, 1, 0),
            (1, 2, 1),
            (1, 3, 2),
            (1, 4, 3),
            (2, 2, 0),
            (2, 3, 1),
            (2, 4, 2),
            (3, 2, 2),
            (3, 3, 0),
            (3, 4, 1),
            (4, 2, 1),
            (4, 3, 2),
            (4, 4, 0),
        ]
        .into_iter()
        .collect();

        assert_eq!(result, expected);
    }

    // #[cfg(test)]
    // // Ridge Regression tests will pass with high probability if we run,
    // // but are not ran automatically since they are non-deterministic.
    // mod test_for_ridge_regression {
    //     use super::*;

    //     fn assert_approx_eq(a: &[f64], b: &[f64], epsilon: f64) {
    //         assert_eq!(a.len(), b.len());
    //         for (x, y) in a.iter().zip(b.iter()) {
    //             assert!(
    //                 (x - y).abs() < epsilon,
    //                 "Expected: {:?}, but got: {:?}",
    //                 a,
    //                 b
    //             );
    //         }
    //     }

    //     #[test]
    //     fn test_zero_matrix_closed_form() {
    //         let a = vec![vec![0.0, 0.0], vec![0.0, 0.0]];
    //         let b = vec![0.0, 0.0];
    //         let lambda = 0.1;
    //         let alpha = 0.01;
    //         let iterations = 1000;

    //         let result = ridge_regression(a, b, lambda, alpha, iterations, 1e-8);
    //         // expected result is computed using ridge regression's closed form solution X = (A^T * A + λI)^-1 * A^T * B.
    //         let expected_result = vec![0.0, 0.0];
    //         assert_approx_eq(&result, &expected_result, 1e-8);
    //     }

    //     #[test]
    //     fn test_basic_ridge_regression() {
    //         let a = vec![vec![1.0, 2.0], vec![3.0, 4.0], vec![5.0, 6.0]];
    //         let b = vec![1.0, 2.0, 3.0];
    //         let lambda = 0.1;
    //         let alpha = 0.01;
    //         let iterations = 1000;

    //         let result = ridge_regression(a, b, lambda, alpha, iterations, 1e-8);

    //         // expected result is computed using ridge regression's closed form solution X = (A^T * A + λI)^-1 * A^T * B.
    //         let expected_result = vec![0.06644518, 0.4469948656];
    //         assert_approx_eq(&expected_result, &result, 1e-1);
    //     }

    //     #[test]
    //     fn test_singular_matrix() {
    //         let a = vec![vec![1.0, 2.0], vec![2.0, 4.0]];
    //         let b = vec![1.0, 2.0];
    //         let lambda = 0.1;
    //         let alpha = 0.01;
    //         let iterations = 1000;

    //         let result = ridge_regression(a, b, lambda, alpha, iterations, 1e-8);
    //         // expected result is computed using ridge regression's closed form solution X = (A^T * A + λI)^-1 * A^T * B.
    //         let expected_result = vec![0.1992031, 0.39840637];
    //         assert_approx_eq(&result, &expected_result, 1e-2);
    //     }

    //     #[test]
    //     fn test_varying_lambda() {
    //         let a = vec![vec![1.0, 2.0], vec![3.0, 4.0], vec![5.0, 6.0]];
    //         let b = vec![1.0, 2.0, 3.0];
    //         let alpha = 0.01;
    //         let iterations = 1000;

    //         // Testing different lambda values 0, 0.5, 1
    //         let result1 = ridge_regression(a.clone(), b.clone(), 0.0, alpha, iterations, 1e-8);
    //         let result2 = ridge_regression(a.clone(), b.clone(), 1.0, alpha, iterations, 1e-8);
    //         let result3 = ridge_regression(a.clone(), b.clone(), 0.5, alpha, iterations, 1e-8);

    //         // expected result is computed using ridge regression's closed form solution X = (A^T * A + λI)^-1 * A^T * B.
    //         let expected_result1 = vec![0.0, 0.5];
    //         let expected_result2 = vec![0.1577060932, 0.3727598566];
    //         let expected_result3 = vec![0.1896551724, 0.3448275862];

    //         assert_approx_eq(&result1, &expected_result1, 1e-1);
    //         assert_approx_eq(&result2, &expected_result2, 1e-1);
    //         assert_approx_eq(&result3, &expected_result3, 1e-1);
    //     }
    //     #[test]
    //     fn test_varying_alpha() {
    //         let a = vec![vec![1.0, 2.0], vec![3.0, 4.0], vec![5.0, 6.0]];
    //         let b = vec![1.0, 2.0, 3.0];
    //         let lambda = 0.1;
    //         let iterations = 10000;

    //         let result1 = ridge_regression(a.clone(), b.clone(), lambda, 0.01, iterations, 1e-8);
    //         let result2 = ridge_regression(a.clone(), b.clone(), lambda, 0.005, iterations, 1e-8);

    //         // expected result is computed using ridge regression's closed form solution X = (A^T * A + λI)^-1 * A^T * B.
    //         let expected_result = vec![0.06644518272, 0.4469948656];

    //         assert_approx_eq(&expected_result, &result1, 1e-1);
    //         assert_approx_eq(&expected_result, &result2, 1e-1);
    //     }

    // }
}
