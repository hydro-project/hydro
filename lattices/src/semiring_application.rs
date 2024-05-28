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
    // Cast a, b, lambda, alpha to RealNumber
    let a: Vec<Vec<RealNumber>> = a
        .iter()
        .map(|row| row.iter().map(|&x| RealNumber(x)).collect())
        .collect();

    let b: Vec<RealNumber> = b.iter().map(|&x| RealNumber(x)).collect();
    let lambda = RealNumber(lambda);
    let alpha = RealNumber(alpha);

    // Initialize x with zeros
    let mut x: Vec<RealNumber> = vec![RealNumber(0.0); a[0].len()];
    // let mut gradient: Vec<RealNumber> = vec![RealNumber(0.0); a[0].len()];

    for _ in 0..iterations {
        // Reset gradient
        let mut gradient: Vec<RealNumber> = vec![RealNumber(0.0); a[0].len()];

        // Compute the gradient
        for i in 0..x.len() {
            let mut a_k_i_a_k_j_x_j = RealNumber(0.0);
            let mut a_j_i_b_j = RealNumber(0.0);

            for j in 0..x.len() {
                let mut a_k_i_a_k_j_x_j_temp = RealNumber(0.0);
                for a_k in &a {
                    let mut temp1 = a_k[j];
                    temp1.mul(x[j]);
                    let mut temp2 = a_k[i];
                    temp2.mul(temp1);
                    a_k_i_a_k_j_x_j_temp.add(temp2);
                }
                a_k_i_a_k_j_x_j.add(a_k_i_a_k_j_x_j_temp);

                let mut temp3 = a[j][i];
                temp3.mul(RealNumber(-1.0));
                temp3.mul(b[j]);
                a_j_i_b_j.add(temp3);

                let mut lambda_x_i = lambda;
                lambda_x_i.mul(x[i]);
                a_j_i_b_j.add(lambda_x_i);
            }
            gradient[i].add(a_k_i_a_k_j_x_j);
            gradient[i].add(a_j_i_b_j);
        }

        // Update x using the gradient and check for convergence
        let mut max_change = 0.0;
        for i in 0..x.len() {
            let mut alpha_gradient_i = alpha;
            alpha_gradient_i.mul(RealNumber(-1.0));
            alpha_gradient_i.mul(gradient[i]);

            let new_x_i = {
                let mut temp = x[i];
                temp.add(alpha_gradient_i);
                temp
            };

            let change = (new_x_i.0 - x[i].0).abs();
            if change > max_change {
                max_change = change;
            }

            x[i] = new_x_i;
        }

        // Check if the maximum change is less than the tolerance
        if max_change < tolerance {
            break;
        }
    }

    // Convert x back to Vec<f64>
    x.iter().map(|&RealNumber(value)| value).collect()
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

/// Floyd-Warshall Algorithm
#[allow(dead_code)]
fn all_pairs_shortest_path(edges: Vec<(usize, usize, u32)>, n: usize) -> Vec<(usize, usize, u32)> {
    let inf = U32WithInfinity::Infinity;
    let mut graph: Vec<Vec<U32WithInfinity>> = vec![vec![inf; n]; n];

    // Initialize the graph with given edges
    for &(start, destination, path_length) in &edges {
        graph[start][destination] = U32WithInfinity::Finite(path_length);
    }

    for (i, graph_i) in graph.iter_mut().enumerate().take(n) {
        graph_i[i] = U32WithInfinity::Finite(0);
    }

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

    // Convert the result back into the edge list format
    let mut result: Vec<(usize, usize, u32)> = Vec::new();
    for (i, dist_i) in dist.iter().enumerate().take(n) {
        for (j, dist_ij) in dist_i.iter().enumerate().take(n) {
            if i != j {
                if let U32WithInfinity::Finite(cost) = dist_ij.0 {
                    result.push((i, j, cost));
                }
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
    fn assert_approx_eq(a: &[f64], b: &[f64], epsilon: f64) {
        assert_eq!(a.len(), b.len());
        for (x, y) in a.iter().zip(b.iter()) {
            assert!(
                (x - y).abs() < epsilon,
                "Expected: {:?}, but got: {:?}",
                a,
                b
            );
        }
    }

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
    fn test_graph_with_multiple_directed_cycle() {
        // Testing a graph with more than one cycle between nodes.

        // Graph in matrix form:
        //     A  B  C  D  E
        // A [0, 4, ∞, 5, ∞]
        // B [∞, 0, 1, ∞, 6]
        // C [2, ∞, 0, 3, ∞]
        // D [∞, ∞, 1, 0, 2]
        // E [1, ∞, ∞, 4, 0]

        // Graph :https://www.geeksforgeeks.org/floyd-warshall-algorithm-dp-16/

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
        let n = 5;

        let result = all_pairs_shortest_path(edges, n);

        let expected = vec![
            (0, 1, 4),
            (0, 2, 5),
            (0, 3, 5),
            (0, 4, 7),
            (1, 0, 3),
            (1, 2, 1),
            (1, 3, 4),
            (1, 4, 6),
            (2, 0, 2),
            (2, 1, 6),
            (2, 3, 3),
            (2, 4, 5),
            (3, 0, 3),
            (3, 1, 7),
            (3, 2, 1),
            (3, 4, 2),
            (4, 0, 1),
            (4, 1, 5),
            (4, 2, 5),
            (4, 3, 4),
        ];

        assert_eq!(result, expected);
    }

    #[test]
    fn test_graph_with_one_directed_cycle() {
        // Testing a graph with a directed cycle.

        // Graph in matrix form:
        //     A  B  C  D
        // A [0, 8, ∞, 1]
        // B [∞, 0, 1, ∞]
        // C [4, ∞, 0, ∞]
        // D [∞, 2, 9, 0]
        // Has a cycle between A -> D -> C -> A

        // Graph: https://www.tutorialspoint.com/all-pairs-shortest-paths

        let edges = vec![
            (0, 1, 8),
            (0, 3, 1),
            (1, 2, 1),
            (2, 0, 4),
            (3, 1, 2),
            (3, 2, 9),
        ];
        let n = 4;

        let result = all_pairs_shortest_path(edges, n);

        let expected = vec![
            (0, 1, 3),
            (0, 2, 4),
            (0, 3, 1),
            (1, 0, 5),
            (1, 2, 1),
            (1, 3, 6),
            (2, 0, 4),
            (2, 1, 7),
            (2, 3, 5),
            (3, 0, 7),
            (3, 1, 2),
            (3, 2, 3),
        ];

        assert_eq!(result, expected);
    }

    #[test]
    fn test_graph_with_undirected_edges() {
        // Testing a graph with undirected edges between nodes.
        // A --(5)-- B
        // |         |
        // (8)       (2)
        // |         |
        // C --(3)-- D

        // Graph in matrix form:
        //     A  B  C  D
        // A [0, 5, 8, ∞]
        // B [5, 0, ∞, 2]
        // C [8, ∞, 0, 3]
        // D [∞, 2, 3, 0]

        // Graph: https://www.altcademy.com/blog/calculate-the-all-pairs-shortest-paths-in-a-graph/

        let edges = vec![
            (0, 1, 5),
            (0, 2, 8),
            (1, 0, 5),
            (1, 3, 2),
            (2, 0, 8),
            (2, 3, 3),
            (3, 1, 2),
            (3, 2, 3),
        ];
        let n = 4;

        let result = all_pairs_shortest_path(edges, n);

        // Expected Output in matrix form:
        //    A  B  C  D
        // A [0, 5, 8, 7]
        // B [5, 0, 5, 2]
        // C [8, 5, 0, 3]
        // D [7, 2, 3, 0]

        let expected = vec![
            (0, 1, 5),
            (0, 2, 8),
            (0, 3, 7),
            (1, 0, 5),
            (1, 2, 5),
            (1, 3, 2),
            (2, 0, 8),
            (2, 1, 5),
            (2, 3, 3),
            (3, 0, 7),
            (3, 1, 2),
            (3, 2, 3),
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_graph_with_zero_weight_edges() {
        // Testing a graph with zero weight edges.
        let edges = vec![(0, 1, 0), (1, 2, 0), (2, 3, 0), (3, 0, 0)];
        let n = 4;

        let result = all_pairs_shortest_path(edges, n);

        let expected = vec![
            (0, 1, 0),
            (0, 2, 0),
            (0, 3, 0),
            (1, 2, 0),
            (1, 3, 0),
            (2, 0, 0),
            (2, 1, 0),
            (2, 3, 0),
            (3, 0, 0),
            (3, 1, 0),
            (3, 2, 0),
        ];

        assert_eq!(result, expected);
    }

    #[test]
    fn test_disconnected_graph() {
        // Testing a disconnected graph.
        let edges = vec![(0, 1, 2), (2, 3, 3)];
        let n = 4;

        let result = all_pairs_shortest_path(edges, n);

        let expected = vec![(0, 1, 2), (2, 3, 3)];

        assert_eq!(result, expected);
    }

    #[test]
    fn test_basic_ridge_regression() {
        let a = vec![vec![1.0, 2.0], vec![3.0, 4.0], vec![5.0, 6.0]];
        let b = vec![1.0, 2.0, 3.0];
        let lambda = 0.1;
        let alpha = 0.01;
        let iterations = 1000;

        let result = ridge_regression(a, b, lambda, alpha, iterations, 1e-8);

        // The expected result is found by running a known-good implementation or manually computing it.
        let expected_result = vec![0.090909, 0.136364];
        assert_approx_eq(&result, &expected_result, 1e-4);
    }

    #[test]
    fn test_zero_matrix() {
        let a = vec![vec![0.0, 0.0], vec![0.0, 0.0]];
        let b = vec![0.0, 0.0];
        let lambda = 1.0;
        let alpha = 0.01;
        let iterations = 10;

        let result = ridge_regression(a, b, lambda, alpha, iterations, 1e-8);
        let expected_result = vec![0.0, 0.0];
        assert_approx_eq(&result, &expected_result, 1e-8);
    }

    #[test]
    fn test_identity_matrix() {
        let a = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        let b = vec![1.0, 1.0];
        let lambda = 0.0;
        let alpha = 0.01;
        let iterations = 100;

        let result = ridge_regression(a, b, lambda, alpha, iterations, 1e-8);
        let expected_result = vec![0.0, 1.0];
        assert_approx_eq(&result, &expected_result, 1e-8);
    }

    #[test]
    fn test_singular_matrix() {
        let a = vec![vec![1.0, 2.0], vec![2.0, 4.0]];
        let b = vec![1.0, 2.0];
        let lambda = 0.1;
        let alpha = 0.01;
        let iterations = 1000;

        let result = ridge_regression(a, b, lambda, alpha, iterations, 1e-8);
        let expected_result = vec![0.1992031, 0.39840637];
        assert_approx_eq(&result, &expected_result, 1e-2);
    }

    #[test]
    fn test_varying_lambda() {
        let a = vec![vec![1.0, 2.0], vec![3.0, 4.0], vec![5.0, 6.0]];
        let b = vec![1.0, 2.0, 3.0];
        let alpha = 0.01;
        let iterations = 1000;

        // Testing different lambda values 0, 0.5, 1
        let result1 = ridge_regression(a.clone(), b.clone(), 0.0, alpha, iterations, 1e-8);
        let result2: Vec<f64> =
            ridge_regression(a.clone(), b.clone(), 1.0, alpha, iterations, 1e-8);
        let result3: Vec<f64> =
            ridge_regression(a.clone(), b.clone(), 0.5, alpha, iterations, 1e-8);

        let expected_result1: Vec<f64> = vec![0.0, 0.5];
        let expected_result2 = vec![0.18965517, 0.3448276862];
        let expected_result3 = vec![0.1577060932, 0.3727598566];

        assert_approx_eq(&result1, &expected_result1, 1e-4);
        assert_approx_eq(&result2, &expected_result2, 1e-4);
        assert_approx_eq(&result3, &expected_result3, 1e-4);
    }

    #[test]
    fn test_varying_alpha() {
        let a = vec![vec![1.0, 2.0], vec![3.0, 4.0], vec![5.0, 6.0]];
        let b = vec![1.0, 2.0, 3.0];
        let lambda = 0.1;
        let iterations = 1000;

        let result1 = ridge_regression(a.clone(), b.clone(), lambda, 0.01, iterations, 1e-8);
        let result2 = ridge_regression(a.clone(), b.clone(), lambda, 0.001, iterations, 1e-8);

        let expected_result = vec![0.06644518272, 0.4469948656];

        assert_approx_eq(&result1, &expected_result, 1e-4);
        assert_approx_eq(&result2, &expected_result, 1e-4);
    }
}
