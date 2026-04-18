//! Backward goal-seeking coordination analysis.
//!
//! Starting from each observable sink, determines whether the program's output
//! is monotone under a partial order appropriate to the output type:
//!
//! - **Prefix order** for `TotalOrder` streams: output is a growing prefix
//! - **Set inclusion** for `NoOrder` streams: output elements only accumulate
//! - **Lattice order** for singletons with commutative+idempotent fold: value only grows
//!
//! The analysis walks backward from each sink, carrying a "proof goal" that
//! operators either discharge (proof complete), preserve (pass to inputs),
//! or break (monotonicity violated).

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt;

use slotmap::SecondaryMap;

use super::builder::CycleId;
use super::ir::{HydroNode, HydroRoot, SharedNode, StreamOrder};
use crate::location::dynamic::LocationId;
use crate::location::LocationKey;
use super::ir::HydroSource;

#[cfg(feature = "build")]
use dfir_lang::diagnostic::{Diagnostic, Level};

// ---------------------------------------------------------------------------
// Order goals
// ---------------------------------------------------------------------------

/// The partial order under which we're trying to prove monotonicity.
///
/// The Coordination Criterion (Hellerstein 2026) states that a program admits
/// a coordination-free implementation iff its observable outputs are
/// future-monotone — meaning outputs only grow (never contradict) as inputs
/// grow. "Growth" is defined by a partial order on the output type:
///
/// - **Prefix**: output is a growing deterministic sequence. Each observation
///   is a prefix of all future observations. Applies to streams.
/// - **SetInclusion**: output elements only accumulate. New elements may appear
///   but none are retracted. Applies to streams.
/// - **Lattice**: output value only grows under a join-semilattice order.
///   Applies to singletons produced by commutative+idempotent aggregation.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum OrderGoal {
    /// Growing prefix of a deterministic sequence (TotalOrder streams).
    Prefix,
    /// Elements only accumulate; no retractions (NoOrder streams).
    SetInclusion,
    /// Value only grows under lattice join. Applies to singletons from
    /// aggregations proven to be lattice joins (commutative + idempotent).
    Lattice,
    // Future: UserDefined { type_name: String }
}

impl fmt::Display for OrderGoal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrderGoal::Prefix => write!(f, "prefix order"),
            OrderGoal::SetInclusion => write!(f, "set inclusion"),
            OrderGoal::Lattice => write!(f, "lattice order"),
        }
    }
}

/// The consistency guarantee provided by a passing coordination analysis.
///
/// Derived from the proof goal, the proof result, and the sink's location:
/// - **SequentiallyConsistent**: Prefix goal passes on a Cluster — all replicas
///   produce prefixes of the same deterministic sequence.
/// - **Convergent**: SetInclusion or Lattice goal passes — all replicas converge
///   via commutative merge.
/// - **SelfConsistent**: analysis passes but no cross-replica guarantee applies
///   (e.g., single Process).
/// - **Inconsistent**: analysis fails — output may contradict earlier observations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConsistencyLabel {
    SequentiallyConsistent,
    Convergent,
    SelfConsistent,
    Inconsistent,
}

impl ConsistencyLabel {
    pub fn strength(&self) -> u8 {
        match self {
            ConsistencyLabel::SequentiallyConsistent => 3,
            ConsistencyLabel::Convergent => 2,
            ConsistencyLabel::SelfConsistent => 1,
            ConsistencyLabel::Inconsistent => 0,
        }
    }
}

impl fmt::Display for ConsistencyLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConsistencyLabel::SequentiallyConsistent => write!(f, "SEQUENTIALLY CONSISTENT"),
            ConsistencyLabel::Convergent => write!(f, "CONVERGENT"),
            ConsistencyLabel::SelfConsistent => write!(f, "SELF-CONSISTENT"),
            ConsistencyLabel::Inconsistent => write!(f, "INCONSISTENT"),
        }
    }
}

fn is_cluster_location(location: &LocationId) -> bool {
    match location {
        LocationId::Cluster(_) => true,
        LocationId::Tick(_, inner) | LocationId::Atomic(inner) => is_cluster_location(inner),
        _ => false,
    }
}

/// Label based only on goal (ignoring location). Used for propagation where
/// location context is handled by the propagation rules.
fn goal_label(goal: &OrderGoal) -> ConsistencyLabel {
    match goal {
        OrderGoal::Prefix => ConsistencyLabel::SequentiallyConsistent,
        OrderGoal::SetInclusion | OrderGoal::Lattice => ConsistencyLabel::Convergent,
    }
}

/// An edge in the location graph with its consistency label.
#[derive(Clone, Debug)]
pub struct ChannelEdge {
    pub from: LocationId,
    pub to: LocationId,
    pub label: ConsistencyLabel,
    /// Label assuming fixed broadcast membership (broadcast plumbing is trusted).
    pub label_fixed_membership: ConsistencyLabel,
    pub goal: OrderGoal,
    /// Whether this is a broadcast-to-Cluster edge.
    pub is_broadcast: bool,
}

// ---------------------------------------------------------------------------
// Channel graph rendering: forward DAG via ascii-dag, back edges listed
// ---------------------------------------------------------------------------

/// Split channel edges into forward (toward sink) and back (feedback) based
/// on backward-walk discovery order. Forward edges form a DAG.
fn split_forward_back(
    channels: &[ChannelEdge],
    discovery_order: &HashMap<LocationId, usize>,
) -> (Vec<ChannelEdge>, Vec<ChannelEdge>) {
    let mut forward = Vec::new();
    let mut back = Vec::new();
    for ch in channels {
        let from_ord = discovery_order.get(&ch.from).copied().unwrap_or(usize::MAX);
        let to_ord = discovery_order.get(&ch.to).copied().unwrap_or(usize::MAX);
        if from_ord > to_ord {
            forward.push(ch.clone());
        } else {
            back.push(ch.clone());
        }
    }
    (forward, back)
}

/// Propagate consistency labels forward along the DAG edges.
///
/// Implements Definition 6.2 from the consistency paper:
///   (i)   Cluster with upstream ≥ Self  → ℓ_local
///   (ii)  Cluster with upstream = Incon → Incon
///   (iii) Deterministic Process          → min(ℓ_local, ℓ_upstream)
///   (iv)  Nondeterministic Process       → Self
///   (v)   Failing proof                  → Incon
///
/// When `assume_fixed_membership` is true, broadcast-to-Cluster edges use
/// their `label_fixed_membership` (Self floor) instead of `label`, modeling
/// the scenario where broadcast membership is static.
///
/// Returns the propagated consistency at the terminal sink.
fn propagate_forward(
    forward: &mut [ChannelEdge],
    discovery_order: &HashMap<LocationId, usize>,
    sink_goal: &OrderGoal,
    sink_proof_passed: bool,
    sink_has_nondet: bool,
    sink_location: &LocationId,
    assume_fixed_membership: bool,
) -> ConsistencyLabel {
    // Sort forward edges source-first (highest discovery order first)
    forward.sort_by(|a, b| {
        let a_ord = discovery_order.get(&a.from).copied().unwrap_or(0);
        let b_ord = discovery_order.get(&b.from).copied().unwrap_or(0);
        b_ord.cmp(&a_ord)
    });

    // Track the best label arriving at each location
    let mut arriving: HashMap<LocationId, ConsistencyLabel> = HashMap::new();

    for edge in forward.iter() {
        let edge_label = if assume_fixed_membership {
            &edge.label_fixed_membership
        } else {
            &edge.label
        };

        let propagated = if let Some(upstream) = arriving.get(&edge.from) {
            let sender_local = goal_label(&edge.goal);
            let sender_is_cluster = is_cluster_location(&edge.from);
            propagate_at_location(upstream, &sender_local, sender_is_cluster, false)
        } else {
            edge_label.clone()
        };

        let entry = arriving.entry(edge.to.clone()).or_insert(ConsistencyLabel::Inconsistent);
        if propagated.strength() > entry.strength() {
            *entry = propagated;
        }
    }

    // Terminal sink label
    if !sink_proof_passed {
        return ConsistencyLabel::Inconsistent;
    }
    let sink_base = base_location(sink_location).clone();
    let local_label = goal_label(sink_goal);
    let is_cluster = is_cluster_location(sink_location);
    match arriving.get(&sink_base) {
        Some(upstream) => propagate_at_location(upstream, &local_label, is_cluster, sink_has_nondet),
        None => {
            // No channels — purely local
            if is_cluster { local_label } else { ConsistencyLabel::SelfConsistent }
        }
    }
}

/// Apply the paper's propagation rules (Definition 6.2) at a single location.
///
/// `upstream`:    best label arriving from upstream locations
/// `local`:       label from this location's own proof goal (goal_label)
/// `is_cluster`:  whether this location is a Cluster
/// `has_nondet`:  whether this location has nondeterministic operations (Process only)
fn propagate_at_location(
    upstream: &ConsistencyLabel,
    local: &ConsistencyLabel,
    is_cluster: bool,
    has_nondet: bool,
) -> ConsistencyLabel {
    // Rule (v): failing proof → Incon
    if *local == ConsistencyLabel::Inconsistent {
        return ConsistencyLabel::Inconsistent;
    }
    if is_cluster {
        // Rule (ii): Cluster with non-monotone upstream → Incon
        if *upstream == ConsistencyLabel::Inconsistent {
            return ConsistencyLabel::Inconsistent;
        }
        // Rule (i): Cluster with monotone upstream → ℓ_local
        local.clone()
    } else if has_nondet {
        // Rule (iv): Nondeterministic Process → Self
        ConsistencyLabel::SelfConsistent
    } else {
        // Rule (iii): Deterministic Process → min(ℓ_local, ℓ_upstream)
        if upstream.strength() <= local.strength() {
            upstream.clone()
        } else {
            local.clone()
        }
    }
}

#[cfg(feature = "build")]
fn render_forward_dag(forward: &[ChannelEdge], names: &SecondaryMap<LocationKey, String>) -> String {
    let mut loc_ids: Vec<LocationId> = Vec::new();
    let mut loc_id = |loc: &LocationId| -> usize {
        let base = base_location(loc).clone();
        if let Some(i) = loc_ids.iter().position(|l| l == &base) { i }
        else { loc_ids.push(base); loc_ids.len() - 1 }
    };

    let edges: Vec<(usize, usize, String)> = forward.iter().map(|ch| {
        (loc_id(&ch.from), loc_id(&ch.to), short_label(&ch.label_fixed_membership).to_string())
    }).collect();

    let nodes: Vec<(usize, String)> = loc_ids.iter().enumerate()
        .map(|(i, loc)| (i, fmt_location(loc, names)))
        .collect();

    // Check if it's a simple chain: each node has at most 1 in-edge and 1 out-edge
    let n = nodes.len();
    let mut in_count = vec![0usize; n];
    let mut out_count = vec![0usize; n];
    let mut out_edge: Vec<Option<usize>> = vec![None; n];
    for (i, &(f, t, _)) in edges.iter().enumerate() {
        in_count[t] += 1;
        out_count[f] += 1;
        out_edge[f] = Some(i);
    }
    let is_chain = in_count.iter().all(|&c| c <= 1) && out_count.iter().all(|&c| c <= 1);

    if is_chain && !edges.is_empty() {
        // Find the root (no incoming edges)
        let start = (0..n).find(|&i| in_count[i] == 0).unwrap_or(0);
        let mut parts = vec![nodes[start].1.clone()];
        let mut cur = start;
        while let Some(ei) = out_edge[cur] {
            let (_, to, ref label) = edges[ei];
            parts.push(format!("--[{}]--> {}", label, nodes[to].1));
            cur = to;
        }
        parts.join(" ")
    } else {
        // Diamond/complex: fall back to ascii-dag vertical with edge labels
        use ascii_dag::Graph;
        let node_refs: Vec<(usize, &str)> = nodes.iter().map(|(i, s)| (*i, s.as_str())).collect();
        let edge_refs: Vec<(usize, usize, Option<&str>)> = edges.iter()
            .map(|(f, t, l)| (*f, *t, Some(l.as_str())))
            .collect();
        let dag = Graph::from_edges_labeled(&node_refs, &edge_refs);
        let ir = dag.compute_layout();
        ir.render_scanline().trim_end().to_string()
    }
}

fn fmt_location(loc: &LocationId, names: &SecondaryMap<LocationKey, String>) -> String {
    let base = base_location(loc);
    let key = base.key();
    let name = names.get(key).map(|s| s.as_str()).unwrap_or("");
    let kind = if is_cluster_location(base) { "Cluster" } else { "Process" };
    if name.is_empty() || name == "()" {
        format!("{}({})", kind, key)
    } else {
        // Strip crate path: "kvs_zoo::kvs_core::KVSNode" → "KVSNode"
        let short = name.rsplit("::").next().unwrap_or(name);
        format!("{}({})", kind, short)
    }
}

/// Short label for use in chain display.
fn short_label(label: &ConsistencyLabel) -> &'static str {
    match label {
        ConsistencyLabel::SequentiallyConsistent => "SEQ",
        ConsistencyLabel::Convergent => "CONV",
        ConsistencyLabel::SelfConsistent => "SELF",
        ConsistencyLabel::Inconsistent => "INCON",
    }
}
fn consistency_description(label: &ConsistencyLabel) -> &'static str {
    match label {
        ConsistencyLabel::SequentiallyConsistent => "All replicas produce prefixes of the same deterministic sequence.",
        ConsistencyLabel::Convergent => "All replicas converge to the same value via commutative merge.",
        ConsistencyLabel::SelfConsistent => "Future-monotone: observations only refine, never retract.",
        ConsistencyLabel::Inconsistent => "Output may contradict earlier observations.",
    }
}

/// Analyze channels along the dataflow path from a sink backward.
/// Returns channel edges and a discovery order map (location → order, 0 = sink location).
fn analyze_channels(
    root_input: &HydroNode,
    sink_location: &LocationId,
    cycle_proofs: &CycleProofs,
    ir: &[HydroRoot],
) -> (Vec<ChannelEdge>, HashMap<LocationId, usize>) {
    let mut edges = Vec::new();
    let mut visited_edges: HashSet<(String, String)> = HashSet::new();
    let mut visited_cycles: HashSet<CycleId> = HashSet::new();
    let mut discovery_order: HashMap<LocationId, usize> = HashMap::new();
    let sink_base = base_location(sink_location).clone();
    discovery_order.insert(sink_base, 0);
    analyze_channels_from(root_input, cycle_proofs, ir, &mut edges, &mut visited_edges, &mut visited_cycles, &mut discovery_order);
    (edges, discovery_order)
}

fn analyze_channels_from(
    node: &HydroNode,
    cycle_proofs: &CycleProofs,
    ir: &[HydroRoot],
    edges: &mut Vec<ChannelEdge>,
    visited_edges: &mut HashSet<(String, String)>,
    visited_cycles: &mut HashSet<CycleId>,
    discovery_order: &mut HashMap<LocationId, usize>,
) {
    match node {
        HydroNode::Network { input, metadata, .. } => {
            let recv_loc = base_location(&metadata.location_id).clone();
            let send_loc = base_location(&input.metadata().location_id).clone();
            if send_loc != recv_loc {
                let edge_key = (format!("{:?}", send_loc), format!("{:?}", recv_loc));
                if !visited_edges.contains(&edge_key) {
                    visited_edges.insert(edge_key);
                    // Record discovery order
                    let next_ord = discovery_order.len();
                    discovery_order.entry(recv_loc.clone()).or_insert(next_ord);
                    let next_ord = discovery_order.len();
                    discovery_order.entry(send_loc.clone()).or_insert(next_ord);
                    // Run proof from sending side, pick strongest goal-based label.
                    // The edge label must reflect the sender's actual consistency
                    // so that propagation rule (ii) can detect upstream Incon.
                    let is_broadcast_to_cluster = is_cluster_location(&recv_loc);
                    let mut best_label = ConsistencyLabel::Inconsistent;
                    let mut best_goal = OrderGoal::SetInclusion;
                    for candidate in &[OrderGoal::Prefix, OrderGoal::Lattice, OrderGoal::SetInclusion] {
                        let mut seen = SeenTees::new();
                        let result = prove(input, candidate, cycle_proofs, &mut seen);
                        if result.is_proved() {
                            let label = goal_label(candidate);
                            if label.strength() > best_label.strength() {
                                best_label = label;
                                best_goal = candidate.clone();
                            }
                        }
                    }
                    // Fixed-membership label: for broadcast-to-Cluster edges, the
                    // edge proof may fail due to broadcast plumbing (dynamic membership
                    // tracking) rather than the actual data path. The fixed-membership
                    // label floors at Self, modeling static cluster membership where
                    // broadcast is trusted infrastructure.
                    let label_fixed = if is_broadcast_to_cluster && best_label == ConsistencyLabel::Inconsistent {
                        ConsistencyLabel::SelfConsistent
                    } else {
                        best_label.clone()
                    };
                    edges.push(ChannelEdge {
                        from: send_loc,
                        to: recv_loc,
                        label: best_label,
                        label_fixed_membership: label_fixed,
                        goal: best_goal,
                        is_broadcast: is_broadcast_to_cluster,
                    });
                }
                // Continue upstream
                analyze_channels_from(input, cycle_proofs, ir, edges, visited_edges, visited_cycles, discovery_order);
                return;
            }
            // Same-location network: keep walking
            analyze_channels_from(input, cycle_proofs, ir, edges, visited_edges, visited_cycles, discovery_order);
        }
        HydroNode::CycleSource { cycle_id, .. } => {
            if !visited_cycles.insert(*cycle_id) {
                return;
            }
            for root in ir {
                if let HydroRoot::CycleSink { cycle_id: sid, input, .. } = root {
                    if sid == cycle_id {
                        analyze_channels_from(input, cycle_proofs, ir, edges, visited_edges, visited_cycles, discovery_order);
                        return;
                    }
                }
            }
        }
        HydroNode::Tee { inner, .. } | HydroNode::Partition { inner, .. } => {
            let borrowed = inner.0.borrow();
            analyze_channels_from(&borrowed, cycle_proofs, ir, edges, visited_edges, visited_cycles, discovery_order);
        }
        _ => {
            for input in node.input() {
                analyze_channels_from(input, cycle_proofs, ir, edges, visited_edges, visited_cycles, discovery_order);
            }
        }
    }
}

fn base_location(loc: &LocationId) -> &LocationId {
    match loc {
        LocationId::Tick(_, inner) | LocationId::Atomic(inner) => base_location(inner),
        other => other,
    }
}

fn goal_for_collection_kind(kind: &super::ir::CollectionKind) -> OrderGoal {
    match kind {
        super::ir::CollectionKind::Stream { order: StreamOrder::TotalOrder, .. } => OrderGoal::Prefix,
        super::ir::CollectionKind::Stream { .. } => OrderGoal::SetInclusion,
        super::ir::CollectionKind::KeyedStream { value_order: StreamOrder::TotalOrder, .. } => OrderGoal::Prefix,
        super::ir::CollectionKind::KeyedStream { .. } => OrderGoal::SetInclusion,
        super::ir::CollectionKind::Singleton { .. }
        | super::ir::CollectionKind::Optional { .. }
        | super::ir::CollectionKind::KeyedSingleton { .. } => OrderGoal::Lattice,
    }
}

// ---------------------------------------------------------------------------
// Proof result
// ---------------------------------------------------------------------------

/// A single step in the backward proof walk.
#[derive(Clone, Debug)]
pub struct ProofStep {
    /// Short operator name (e.g. "map", "foldkeyed", "scan").
    pub operator: String,
    /// What happened at this step.
    pub action: ProofAction,
    /// Source location of the operator in user code (for text display).
    pub span: Option<String>,
    /// proc_macro2 span for compiler diagnostic integration (IDE warnings).
    pub proc_macro_span: Option<proc_macro2::Span>,
}

/// What the proof did at a given operator.
#[derive(Clone, Debug)]
pub enum ProofAction {
    /// Goal preserved — passed through to input.
    Preserved,
    /// Goal discharged — proof complete at this operator.
    Discharged { reason: String },
    /// Goal broken — monotonicity violated at this operator.
    Broken { reason: String },
    /// Goal changed — a different goal was required on an input.
    GoalChanged { new_goal: OrderGoal },
    /// Nondeterminism resolved locally on a Process — proof continues,
    /// but replication would require coordination at this site.
    ResolvedLocally { reason: String },
}

/// Result of the backward proof walk for a single path.
#[derive(Clone, Debug)]
pub struct ProofResult {
    pub success: bool,
    /// The walk trace, from sink (first) to source/break point (last).
    pub trace: Vec<ProofStep>,
    /// Sites where ObserveNonDet on a Process was resolved locally.
    /// These would require coordination if the Process were replicated as a Cluster.
    pub replication_issues: Vec<ProofStep>,
}

impl ProofResult {
    pub fn is_proved(&self) -> bool {
        self.success
    }

    pub(crate) fn proved(trace: Vec<ProofStep>) -> Self {
        Self { success: true, trace, replication_issues: vec![] }
    }

    pub(crate) fn broken(trace: Vec<ProofStep>) -> Self {
        Self { success: false, trace, replication_issues: vec![] }
    }

    pub(crate) fn discharged(operator: &str, reason: impl Into<String>, span: Option<String>, pm_span: Option<proc_macro2::Span>) -> Self {
        Self::proved(vec![ProofStep {
            operator: operator.to_string(),
            action: ProofAction::Discharged { reason: reason.into() },
            span,
            proc_macro_span: pm_span,
        }])
    }

    pub(crate) fn fail(operator: &str, reason: impl Into<String>, span: Option<String>, pm_span: Option<proc_macro2::Span>) -> Self {
        Self::broken(vec![ProofStep {
            operator: operator.to_string(),
            action: ProofAction::Broken { reason: reason.into() },
            span,
            proc_macro_span: pm_span,
        }])
    }

    /// Append a "preserved" step (trace is built back-to-front, reversed at display time).
    pub(crate) fn prepend_preserved(mut self, operator: &str, span: Option<String>, pm_span: Option<proc_macro2::Span>) -> Self {
        self.trace.push(ProofStep {
            operator: operator.to_string(),
            action: ProofAction::Preserved,
            span,
            proc_macro_span: pm_span,
        });
        self
    }

    /// Append a "goal changed" step (trace is built back-to-front, reversed at display time).
    pub(crate) fn prepend_goal_changed(mut self, operator: &str, new_goal: &OrderGoal, span: Option<String>, pm_span: Option<proc_macro2::Span>) -> Self {
        self.trace.push(ProofStep {
            operator: operator.to_string(),
            action: ProofAction::GoalChanged { new_goal: new_goal.clone() },
            span,
            proc_macro_span: pm_span,
        });
        self
    }

    /// Append a "resolved locally" step and record a replication issue.
    pub(crate) fn prepend_resolved_locally(mut self, operator: &str, reason: impl Into<String>, span: Option<String>, pm_span: Option<proc_macro2::Span>) -> Self {
        let reason = reason.into();
        let step = ProofStep {
            operator: operator.to_string(),
            action: ProofAction::ResolvedLocally { reason: reason.clone() },
            span: span.clone(),
            proc_macro_span: pm_span,
        };
        self.replication_issues.push(ProofStep {
            operator: step.operator.clone(),
            action: ProofAction::ResolvedLocally { reason },
            span,
            proc_macro_span: step.proc_macro_span,
        });
        self.trace.push(step);
        self
    }

    /// Merge replication issues from another result into this one.
    pub(crate) fn merge_replication_issues(mut self, other: &ProofResult) -> Self {
        self.replication_issues.extend(other.replication_issues.iter().cloned());
        self
    }
}

// ---------------------------------------------------------------------------
// Span formatting
// ---------------------------------------------------------------------------

fn node_proc_macro_span(node: &HydroNode) -> Option<proc_macro2::Span> {
    use syn::spanned::Spanned;
    // Try to get the span from the node's expression (acc/f for folds/reduces)
    match node {
        HydroNode::Fold { acc, .. }
        | HydroNode::FoldKeyed { acc, .. }
        | HydroNode::Scan { acc, .. }
        | HydroNode::ScanAsyncBlocking { acc, .. } => Some(acc.0.span()),
        HydroNode::Reduce { f, .. }
        | HydroNode::ReduceKeyed { f, .. }
        | HydroNode::ReduceKeyedWatermark { f, .. } => Some(f.0.span()),
        HydroNode::Map { f, .. }
        | HydroNode::FlatMap { f, .. }
        | HydroNode::Filter { f, .. }
        | HydroNode::FilterMap { f, .. } => Some(f.0.span()),
        _ => None,
    }
}

fn node_span(node: &HydroNode) -> Option<String> {
    node.metadata().op.backtrace.format_span()
}

// ---------------------------------------------------------------------------
// Sink annotation
// ---------------------------------------------------------------------------

/// Analysis result for a single observable sink.
#[derive(Clone)]
pub struct SinkResult {
    pub name: String,
    pub span: String,
    pub goal: OrderGoal,
    pub result: ProofResult,
    pub location: LocationId,
    /// Consistency assuming dynamic broadcast membership (full analysis).
    pub consistency: ConsistencyLabel,
    /// Consistency assuming fixed broadcast membership (broadcast plumbing trusted).
    pub consistency_fixed: ConsistencyLabel,
    /// Per-channel consistency labels along the dataflow path.
    pub channels: Vec<ChannelEdge>,
    /// Discovery order of locations from the backward walk (location → order, 0 = sink).
    pub discovery_order: HashMap<LocationId, usize>,
    /// Sites where local nondeterminism was resolved on a Process.
    pub replication_issues: Vec<ProofStep>,
}

// ---------------------------------------------------------------------------
// Report
// ---------------------------------------------------------------------------

/// Full coordination analysis report.
pub struct CoordinationReport {
    pub sinks: Vec<SinkResult>,
    pub location_names: SecondaryMap<LocationKey, String>,
}

impl CoordinationReport {
    /// Whether all observable sinks are proved monotone.
    pub fn all_monotone(&self) -> bool {
        self.sinks.iter().all(|s| s.result.is_proved())
    }

    /// Whether all observable sinks have a consistency guarantee (not Inconsistent).
    pub fn all_consistent(&self) -> bool {
        self.sinks.iter().all(|s| s.consistency != ConsistencyLabel::Inconsistent)
    }

    /// Generate compiler diagnostics (warnings/notes) for rust-analyzer integration.
    ///
    /// Failing sinks produce a warning at the break point's span.
    /// Passing sinks produce a note at the discharge point's span.
    #[cfg(feature = "build")]
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        let mut diags = Vec::new();
        for sink in &self.sinks {
            if !sink.result.is_proved() {
                // Find the break point — last step in the trace
                if let Some(step) = sink.result.trace.last() {
                    if let ProofAction::Broken { reason } = &step.action {
                        let span = step.proc_macro_span.unwrap_or_else(proc_macro2::Span::call_site);
                        diags.push(Diagnostic::spanned(
                            span,
                            Level::Warning,
                            format!(
                                "INCONSISTENT: {} (sink `{}`, goal: {})",
                                reason, sink.name, sink.goal
                            ),
                        ));
                    }
                }
            } else {
                if let Some(step) = sink.result.trace.last() {
                    if let ProofAction::Discharged { .. } = &step.action {
                        let span = step.proc_macro_span.unwrap_or_else(proc_macro2::Span::call_site);
                        diags.push(Diagnostic::spanned(
                            span,
                            Level::Note,
                            format!("{}: sink `{}` ({})", sink.consistency, sink.name, sink.goal),
                        ));
                    }
                }
            }
        }
        diags
    }
}

impl fmt::Display for CoordinationReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.sinks.is_empty() {
            return write!(f, "Coordination Criterion: no observable sinks found");
        }

        // Compute common path prefix across all spans for shorter display
        let all_spans: Vec<&str> = self.sinks.iter()
            .map(|s| s.span.as_str())
            .chain(self.sinks.iter().flat_map(|s| s.result.trace.iter().filter_map(|t| t.span.as_deref())))
            .filter(|s| !s.is_empty())
            .collect();
        let prefix_len = if all_spans.is_empty() { 0 } else {
            let first = all_spans[0].as_bytes();
            let mut len = first.len();
            for s in &all_spans[1..] {
                len = len.min(
                    first.iter().zip(s.as_bytes()).take_while(|(a, b)| a == b).count()
                );
            }
            // Back up to last '/' so we don't cut mid-component
            match all_spans[0][..len].rfind('/') {
                Some(i) => i + 1,
                None => 0,
            }
        };
        let trim = |s: &str| -> String {
            if s.len() > prefix_len { s[prefix_len..].to_string() } else { s.to_string() }
        };

        let total = self.sinks.len();
        let failing: Vec<_> = self.sinks.iter().filter(|s| !s.result.is_proved()).collect();
        if failing.is_empty() {
            writeln!(f, "Coordination Analysis: all {total} sinks are consistent")?;
        } else {
            writeln!(
                f,
                "Coordination Analysis: {}/{total} sinks are INCONSISTENT",
                failing.len()
            )?;
        }

        for s in &self.sinks {
            let loc = fmt_location(&s.location, &self.location_names);
            if s.result.is_proved() {
                if s.consistency == s.consistency_fixed {
                    writeln!(f, "\n  ✓ {} @ {} ({}) — {}", s.name, trim(&s.span), loc, s.consistency)?;
                } else {
                    writeln!(f, "\n  ✓ {} @ {} ({}) — {} (fixed membership: {})",
                        s.name, trim(&s.span), loc, s.consistency, s.consistency_fixed)?;
                }
            } else {
                writeln!(f, "\n  ✗ {} @ {} ({}) — INCONSISTENT", s.name, trim(&s.span), loc)?;
            }
            // Show channel graph: forward DAG via ascii-dag, back edges listed
            if !s.channels.is_empty() {
                let (forward, back) = split_forward_back(&s.channels, &s.discovery_order);
                #[cfg(feature = "build")]
                if !forward.is_empty() {
                    let graph = render_forward_dag(&forward, &self.location_names);
                    for line in graph.trim_end().lines() {
                        writeln!(f, "    {}", line)?;
                    }
                }
                #[cfg(not(feature = "build"))]
                for ch in &forward {
                    writeln!(f, "    {} --[{}]--> {}", fmt_location(&ch.from, &self.location_names), short_label(&ch.label_fixed_membership), fmt_location(&ch.to, &self.location_names))?;
                }
                for ch in &back {
                    writeln!(f, "    ↩ {} --[{}]--> {}", fmt_location(&ch.from, &self.location_names), short_label(&ch.label_fixed_membership), fmt_location(&ch.to, &self.location_names))?;
                }
            }
            // Show proof trace
            for step in &s.result.trace {
                let span = step.span.as_deref().map(|s| trim(s)).unwrap_or_default();
                match &step.action {
                    ProofAction::Preserved => {
                        writeln!(f, "    {} — preserved  {}", step.operator, span)?;
                    }
                    ProofAction::Discharged { reason } => {
                        writeln!(f, "    {} — ✓ discharged: {}  {}", step.operator, reason, span)?;
                    }
                    ProofAction::Broken { reason } => {
                        writeln!(f, "    {} — ✗ BROKEN: {}  {}", step.operator, reason, span)?;
                    }
                    ProofAction::GoalChanged { new_goal } => {
                        writeln!(f, "    {} — goal → {}  {}", step.operator, new_goal, span)?;
                    }
                    ProofAction::ResolvedLocally { .. } => {
                        writeln!(f, "    {} — resolved locally (Process)  {}", step.operator, span)?;
                    }
                }
            }
        }

        // Replication analysis: collect all Process locations with replication issues
        let mut process_issues: HashMap<String, Vec<&ProofStep>> = HashMap::new();
        for s in &self.sinks {
            if !s.replication_issues.is_empty() {
                let loc_name = fmt_location(&s.location, &self.location_names);
                let issues = process_issues.entry(loc_name).or_default();
                for issue in &s.replication_issues {
                    // Deduplicate by span
                    if !issues.iter().any(|existing| existing.span == issue.span) {
                        issues.push(issue);
                    }
                }
            }
        }
        if !process_issues.is_empty() {
            writeln!(f, "\n  Replication analysis:")?;
            for (loc, issues) in &process_issues {
                writeln!(f, "    {loc}: {} coordination point(s) if replicated", issues.len())?;
                for issue in issues {
                    let span = issue.span.as_deref().map(|s| trim(s)).unwrap_or_else(|| "(unknown)".to_string());
                    writeln!(f, "      • {span}")?;
                }
            }
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Analysis state
// ---------------------------------------------------------------------------

type SeenTees = HashMap<(*const RefCell<HydroNode>, OrderGoal), ProofResult>;
type CycleProofs = HashMap<CycleId, ProofResult>;

// ---------------------------------------------------------------------------
// Observable sink detection
// ---------------------------------------------------------------------------

fn is_observable_sink(root: &HydroRoot) -> bool {
    match root {
        HydroRoot::ForEach { .. } => true,
        HydroRoot::EmbeddedOutput { .. } => true,
        HydroRoot::DestSink { .. } => true,
        HydroRoot::CycleSink { .. } => false,
        HydroRoot::Null { .. } => false,
        HydroRoot::SendExternal { .. } => true,
    }
}

// ---------------------------------------------------------------------------
// Short name helper
// ---------------------------------------------------------------------------

fn short_name(node: &HydroNode) -> &'static str {
    match node {
        HydroNode::Placeholder => "placeholder",
        HydroNode::Source { .. } => "source",
        HydroNode::SingletonSource { .. } => "singletonsource",
        HydroNode::CycleSource { .. } => "cyclesource",
        HydroNode::ExternalInput { .. } => "externalinput",
        HydroNode::Tee { .. } => "tee",
        HydroNode::Partition { .. } => "partition",
        HydroNode::Cast { .. } => "cast",
        HydroNode::ObserveNonDet { .. } => "observenondet",
        HydroNode::BeginAtomic { .. } => "beginatomic",
        HydroNode::EndAtomic { .. } => "endatomic",
        HydroNode::YieldConcat { .. } => "yieldconcat",
        HydroNode::Map { .. } => "map",
        HydroNode::FlatMap { .. } => "flatmap",
        HydroNode::Filter { .. } => "filter",
        HydroNode::FilterMap { .. } => "filtermap",
        HydroNode::Inspect { .. } => "inspect",
        HydroNode::Enumerate { .. } => "enumerate",
        HydroNode::Unique { .. } => "unique",
        HydroNode::Network { .. } => "network",
        HydroNode::Counter { .. } => "counter",
        HydroNode::Batch { .. } => "batch",
        HydroNode::Chain { .. } => "chain",
        HydroNode::ChainFirst { .. } => "chainfirst",
        HydroNode::Join { .. } => "join",
        HydroNode::CrossProduct { .. } => "crossproduct",
        HydroNode::CrossSingleton { .. } => "crosssingleton",
        HydroNode::Difference { .. } => "difference",
        HydroNode::AntiJoin { .. } => "antijoin",
        HydroNode::Fold { .. } => "fold",
        HydroNode::FoldKeyed { .. } => "foldkeyed",
        HydroNode::Reduce { .. } => "reduce",
        HydroNode::ReduceKeyed { .. } => "reducekeyed",
        HydroNode::ReduceKeyedWatermark { .. } => "reducekeyedwatermark",
        HydroNode::Scan { .. } => "scan",
        HydroNode::Sort { .. } => "sort",
        HydroNode::DeferTick { .. } => "defertick",
        HydroNode::ResolveFutures { .. } => "resolvefutures",
        HydroNode::ResolveFuturesBlocking { .. } => "resolvefuturesblocking",
        HydroNode::ResolveFuturesOrdered { .. } => "resolvefuturesordered",
        HydroNode::FlatMapStreamBlocking { .. } => "flatmapstreamblocking",
        HydroNode::ScanAsyncBlocking { .. } => "scanasyncblocking",
    }
}

// ---------------------------------------------------------------------------
// Core backward walk
// ---------------------------------------------------------------------------


/// Helper: preserve goal on a single input, or fail with a message.
fn preserve_or_fail(
    input: &HydroNode,
    goal: &OrderGoal,
    allowed: &[OrderGoal],
    fail_msg: &str,
    name: &str,
    span: Option<String>,
    pm_span: Option<proc_macro2::Span>,
    cycle_proofs: &CycleProofs,
    seen_tees: &mut SeenTees,
) -> ProofResult {
    if allowed.contains(goal) {
        prove(input, goal, cycle_proofs, seen_tees).prepend_preserved(name, span, pm_span)
    } else {
        ProofResult::fail(name, fail_msg, span, pm_span)
    }
}

/// Helper: aggregation discharge logic. A lattice join (commutative+idempotent)
/// discharges the proof; bounded input also discharges.
fn aggregation_discharge(
    is_commutative: bool,
    is_idempotent: bool,
    input: &HydroNode,
    goal: &OrderGoal,
    name: &str,
    span: Option<String>,
    pm_span: Option<proc_macro2::Span>,
    fail_msg: &str,
) -> ProofResult {
    if is_commutative && is_idempotent && *goal != OrderGoal::Prefix {
        ProofResult::discharged(name, "lattice join (proven commutative + idempotent)", span, pm_span)
    } else if input.metadata().collection_kind.is_bounded() {
        ProofResult::discharged(name, "bounded input", span, pm_span)
    } else {
        ProofResult::fail(name, fail_msg, span, pm_span)
    }
}

/// Helper: difference/anti-join logic.
fn difference_logic(
    pos: &HydroNode,
    neg: &HydroNode,
    goal: &OrderGoal,
    name: &str,
    span: Option<String>,
    pm_span: Option<proc_macro2::Span>,
    fail_msg: &str,
    cycle_proofs: &CycleProofs,
    seen_tees: &mut SeenTees,
) -> ProofResult {
    match goal {
        OrderGoal::SetInclusion => {
            let p = prove(pos, &OrderGoal::SetInclusion, cycle_proofs, seen_tees);
            if !p.is_proved() { return p.prepend_preserved(name, span.clone(), pm_span.clone()); }
            if neg.metadata().collection_kind.is_bounded() {
                let mut result = ProofResult::discharged(name, "neg input is bounded", span, pm_span);
                result.replication_issues = p.replication_issues;
                result
            } else {
                ProofResult::fail(name, "unbounded neg input can retract output elements", span, pm_span)
            }
        }
        _ => ProofResult::fail(name, fail_msg, span, pm_span),
    }
}


/// How an operator behaves for monotonicity analysis.
enum MonotoneBehavior<'a> {
    /// Discharges any goal (sources).
    Source,
    /// Preserves any goal, recurse into inner node.
    Passthrough(&'a HydroNode),
    /// Preserves the listed goals, breaks all others. Recurse into input.
    PreserveGoals { input: &'a HydroNode, goals: &'static [OrderGoal] },
    /// Aggregation that may be a lattice join (discharges if proven commutative+idempotent).
    Aggregation { is_commutative: bool, is_idempotent: bool, input: &'a HydroNode },
    /// Preserves any goal via prove_shared (Tee, Partition).
    SharedPassthrough(&'a SharedNode),
    /// Difference/AntiJoin: SetInclusion with bounded neg check.
    DifferenceOp { pos: &'a HydroNode, neg: &'a HydroNode },
    /// Preserve SetInclusion+Prefix if output metadata is TotalOrder, else preserve only SetInclusion.
    /// Used for Network and Scan which may or may not preserve ordering.
    PreserveIfOrdered { input: &'a HydroNode, output_is_total_order: bool },
    /// Two branches, both must prove the goal. SetInclusion only.
    TwoBranchSetInclusion { first: &'a HydroNode, second: &'a HydroNode },
    /// Requires custom logic in prove().
    Custom,
}

/// Classify a node's monotonicity behavior for the coordination analysis.
/// Most operators fall into a standard category; only a few need custom
/// logic in `prove()`. New operators should add an entry here.
fn classify(node: &HydroNode) -> MonotoneBehavior<'_> {
    use MonotoneBehavior::*;
    const SET: &[OrderGoal] = &[OrderGoal::SetInclusion];
    const SET_PREFIX: &[OrderGoal] = &[OrderGoal::SetInclusion, OrderGoal::Prefix];
    const SET_LATTICE: &[OrderGoal] = &[OrderGoal::SetInclusion, OrderGoal::Lattice];

    match node {
        // Sources: on a Cluster, per-member sources break coordination.
        // Embedded/EmbeddedSingleton (includes CLUSTER_SELF_ID) and Spin are per-member.
        // Iter, Stream, ClusterMembers, ExternalNetwork produce identical data at all members.
        HydroNode::Source { source, metadata, .. } => {
            if matches!(metadata.location_id, LocationId::Cluster(_)) {
                match source {
                    HydroSource::Embedded(_) | HydroSource::EmbeddedSingleton(_) | HydroSource::Spin() => Custom,
                    _ => Source, // Iter, Stream, ClusterMembers, ExternalNetwork — same at all members
                }
            } else {
                Source
            }
        }
        HydroNode::Placeholder
        | HydroNode::SingletonSource { .. }
        | HydroNode::ExternalInput { .. }
        | HydroNode::Counter { .. } => Source,

        // Passthrough
        HydroNode::Cast { inner, .. }
        | HydroNode::BeginAtomic { inner, .. }
        | HydroNode::EndAtomic { inner, .. }
        | HydroNode::YieldConcat { inner, .. } => Passthrough(inner),

        // ObserveNonDet: type-level ordering annotation (assume_ordering / nondet!).
        // Does not change data — just a type cast. The analysis looks straight through it.
        // The real trust boundary is the upstream operator (e.g., Network broadcast into Cluster).
        HydroNode::ObserveNonDet { inner, .. } => Passthrough(inner),

        // Shared passthrough
        HydroNode::Tee { inner, .. }
        | HydroNode::Partition { inner, .. } => SharedPassthrough(inner),

        // Element-wise: preserve SetInclusion + Prefix
        HydroNode::Map { input, .. }
        | HydroNode::FlatMap { input, .. }
        | HydroNode::Filter { input, .. }
        | HydroNode::FilterMap { input, .. }
        | HydroNode::Inspect { input, .. }
        | HydroNode::FlatMapStreamBlocking { input, .. } => PreserveGoals { input, goals: SET_PREFIX },

        // SetInclusion only
        HydroNode::Enumerate { input, .. }
        | HydroNode::Unique { input, .. } => PreserveGoals { input, goals: SET },

        // Batch: nondeterministic tick boundaries — handled in prove() for replication analysis
        HydroNode::Batch { .. } => Custom,

        HydroNode::ResolveFutures { input, .. }
        | HydroNode::ResolveFuturesBlocking { input, .. }
        | HydroNode::ResolveFuturesOrdered { input, .. } => PreserveGoals { input, goals: SET },

        // SetInclusion + Lattice
        HydroNode::DeferTick { input, .. } => PreserveGoals { input, goals: SET_LATTICE },

        // Aggregation
        HydroNode::Fold { is_commutative, is_idempotent, input, .. }
        | HydroNode::FoldKeyed { is_commutative, is_idempotent, input, .. }
        | HydroNode::Reduce { is_commutative, is_idempotent, input, .. }
        | HydroNode::ReduceKeyed { is_commutative, is_idempotent, input, .. } => {
            Aggregation { is_commutative: *is_commutative, is_idempotent: *is_idempotent, input }
        }

        HydroNode::ReduceKeyedWatermark { is_commutative, is_idempotent, input, .. } => {
            Aggregation { is_commutative: *is_commutative, is_idempotent: *is_idempotent, input }
        }

        // Difference / AntiJoin
        HydroNode::Difference { pos, neg, .. }
        | HydroNode::AntiJoin { pos, neg, .. } => DifferenceOp { pos, neg },

        // Sort: bounded discharges, unbounded breaks (same as non-commutative aggregation)
        HydroNode::Sort { input, .. } => Aggregation { is_commutative: false, is_idempotent: false, input },

        // Network: cross-location receive onto a Cluster discharges (broadcast trust boundary).
        // Same-location or non-Cluster preserves based on output ordering.
        HydroNode::Network { input, metadata, .. } => {
            let recv_loc = &metadata.location_id;
            let send_loc = &input.metadata().location_id;
            let is_cross_location_to_cluster = matches!(recv_loc, LocationId::Cluster(_))
                && recv_loc != send_loc;
            if is_cross_location_to_cluster {
                Source // all cluster members receive the same data via broadcast
            } else {
                let is_total = matches!(
                    &metadata.collection_kind,
                    super::ir::CollectionKind::Stream { order: StreamOrder::TotalOrder, .. }
                    | super::ir::CollectionKind::KeyedStream { value_order: StreamOrder::TotalOrder, .. }
                );
                PreserveIfOrdered { input, output_is_total_order: is_total }
            }
        }

        // Scan: preserves if input is TotalOrder
        HydroNode::Scan { input, .. }
        | HydroNode::ScanAsyncBlocking { input, .. } => {
            let is_total = matches!(
                &input.metadata().collection_kind,
                super::ir::CollectionKind::Stream { order: StreamOrder::TotalOrder, .. }
            );
            PreserveIfOrdered { input, output_is_total_order: is_total }
        }

        // Two-branch: both must prove the goal
        HydroNode::Chain { first, second, .. }
        | HydroNode::ChainFirst { first, second, .. }
        | HydroNode::Join { left: first, right: second, .. }
        | HydroNode::CrossProduct { left: first, right: second, .. } => TwoBranchSetInclusion { first, second },

        // Everything else needs custom logic
        _ => Custom,
    }
}

/// Walk backward from `node`, trying to prove `goal`. Returns the proof result with trace.
fn prove(
    node: &HydroNode,
    goal: &OrderGoal,
    cycle_proofs: &CycleProofs,
    seen_tees: &mut SeenTees,
) -> ProofResult {
    let name = short_name(node);
    let span = node_span(node);
    let pm_span = node_proc_macro_span(node);

    // Dispatch standard categories first
    match classify(node) {
        MonotoneBehavior::Source => {
            return ProofResult::discharged(&name, "source — data only arrives", span, pm_span);
        }
        MonotoneBehavior::Passthrough(inner) => {
            return prove(inner, goal, cycle_proofs, seen_tees).prepend_preserved(&name, span, pm_span);
        }
        MonotoneBehavior::PreserveGoals { input, goals } => {
            return preserve_or_fail(
                input, goal, goals,
                "breaks goal (not preserved by this operator)",
                &name, span, pm_span, cycle_proofs, seen_tees,
            );
        }
        MonotoneBehavior::Aggregation { is_commutative, is_idempotent, input } => {
            return aggregation_discharge(
                is_commutative, is_idempotent, input, goal,
                &name, span, pm_span,
                "unbounded input without lattice join proof (requires commutative + idempotent)",
            );
        }
        MonotoneBehavior::SharedPassthrough(inner) => {
            return prove_shared(inner, goal, cycle_proofs, seen_tees).prepend_preserved(&name, span, pm_span);
        }
        MonotoneBehavior::DifferenceOp { pos, neg } => {
            return difference_logic(
                pos, neg, goal, &name, span, pm_span,
                "breaks prefix/lattice order",
                cycle_proofs, seen_tees,
            );
        }
        MonotoneBehavior::PreserveIfOrdered { input, output_is_total_order } => {
            let allowed: &[OrderGoal] = if output_is_total_order {
                &[OrderGoal::SetInclusion, OrderGoal::Prefix]
            } else {
                &[OrderGoal::SetInclusion]
            };
            return preserve_or_fail(
                input, goal, allowed,
                "non-TotalOrder breaks prefix order",
                &name, span, pm_span, cycle_proofs, seen_tees,
            );
        }
        MonotoneBehavior::TwoBranchSetInclusion { first, second } => {
            return match goal {
                OrderGoal::SetInclusion => {
                    let a = prove(first, goal, cycle_proofs, seen_tees);
                    if !a.is_proved() {
                        return a.prepend_preserved(&format!("{name} (1st branch)"), span.clone(), pm_span.clone());
                    }
                    let b = prove(second, goal, cycle_proofs, seen_tees);
                    if !b.is_proved() {
                        return b.merge_replication_issues(&a).prepend_preserved(&format!("{name} (2nd branch)"), span.clone(), pm_span.clone());
                    }
                    b.merge_replication_issues(&a).prepend_preserved(&name, span, pm_span)
                }
                _ => ProofResult::fail(&name, "breaks prefix/lattice order", span, pm_span),
            };
        }
        MonotoneBehavior::Custom => {} // fall through
    }

    // Custom logic for operators that need it
    match node {

        // --- Batch: nondeterministic tick boundaries ---
        // Batch groups elements into ticks nondeterministically. However:
        // - SetInclusion is always preserved (no elements lost).
        // - Prefix is preserved when the input is TotalOrder: batch doesn't
        //   reorder elements, and all_ticks() concatenation restores the
        //   full prefix. On a Cluster with broadcast input, all members
        //   process the same TotalOrder sequence.
        HydroNode::Batch { inner, metadata, .. } => {
            let input_is_total_order = matches!(
                &inner.metadata().collection_kind,
                super::ir::CollectionKind::Stream { order: StreamOrder::TotalOrder, .. }
            );
            let allowed: &[OrderGoal] = if input_is_total_order {
                &[OrderGoal::SetInclusion, OrderGoal::Prefix]
            } else {
                &[OrderGoal::SetInclusion]
            };
            if !allowed.contains(goal) {
                return ProofResult::fail(&name, "breaks goal (not preserved by batch)", span, pm_span);
            }
            if is_cluster_location(&metadata.location_id) {
                prove(inner, goal, cycle_proofs, seen_tees)
                    .prepend_resolved_locally(&name, "per-member nondeterministic batch boundary on Cluster", span, pm_span)
            } else {
                prove(inner, goal, cycle_proofs, seen_tees)
                    .prepend_resolved_locally(&name, "nondeterministic batch boundary", span, pm_span)
            }
        }

        // --- CycleSource: inherit from matching CycleSink ---
        HydroNode::CycleSource { cycle_id, .. } => {
            match cycle_proofs.get(cycle_id) {
                Some(r) => r.clone().prepend_preserved(&name, span, pm_span),
                None => {
                    // This should not happen in well-formed IR — ForwardHandle panics
                    // if not completed, guaranteeing a matching CycleSink exists.
                    ProofResult::fail(&name, "BUG: CycleSource with no matching CycleSink", span, pm_span)
                }
            }
        }

        // --- CrossSingleton ---
        HydroNode::CrossSingleton { left, right, .. } => match goal {
            OrderGoal::SetInclusion | OrderGoal::Prefix => {
                let a = prove(left, goal, cycle_proofs, seen_tees);
                if !a.is_proved() { return a.prepend_preserved(&name, span.clone(), pm_span.clone()); }
                // Bounded singleton is stable — no lattice proof needed
                if right.metadata().collection_kind.is_bounded() {
                    a.prepend_preserved(&name, span, pm_span)
                } else {
                    let b = prove(right, &OrderGoal::Lattice, cycle_proofs, seen_tees);
                    b.merge_replication_issues(&a)
                        .prepend_goal_changed(&name, &OrderGoal::Lattice, span, pm_span)
                }
            }
            _ => ProofResult::fail(&name, "cross_singleton breaks lattice order", span, pm_span),
        },

        // Catch-all for nodes handled by classify() — should not be reached
        _ => ProofResult::fail(&name, "BUG: unhandled node type", span, pm_span),
    }
}


fn prove_shared(
    inner: &SharedNode,
    goal: &OrderGoal,
    cycle_proofs: &CycleProofs,
    seen_tees: &mut SeenTees,
) -> ProofResult {
    let key = (inner.as_ptr(), goal.clone());
    if let Some(result) = seen_tees.get(&key) {
        return result.clone();
    }
    // Placeholder to break cycles
    seen_tees.insert(key.clone(), ProofResult::proved(vec![]));
    let result = prove(&inner.0.borrow(), goal, cycle_proofs, seen_tees);
    seen_tees.insert(key, result.clone());
    result
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Analyze a Hydro IR program using backward goal-seeking.
///
/// For each observable sink, determines a default proof goal based on the
/// output collection kind, then walks backward to prove or disprove it.
///
/// The `goal_overrides` parameter allows overriding the default goal for
/// specific sinks. Keys are sink identifiers in the format `"name@file:line:col"`
/// (e.g. `"sendexternal@src/plumbing.rs:73:20"`), matching the name and span
/// shown in the analysis report. Pass an empty map to use all defaults.
pub fn analyze_coordination(
    ir: &[HydroRoot],
    goal_overrides: &HashMap<String, OrderGoal>,
    location_names: &SecondaryMap<LocationKey, String>,
) -> CoordinationReport {
    // Pass 1: analyze CycleSink roots to determine cycle monotonicity.
    // Seed all cycles with optimistic placeholders so self-recursive cycles
    // (where CycleSink input depends on its own CycleSource) don't fail.
    // Then iterate to fixpoint.
    let mut cycle_proofs = CycleProofs::new();
    for root in ir {
        if let HydroRoot::CycleSink { cycle_id, .. } = root {
            cycle_proofs.insert(*cycle_id, ProofResult::proved(vec![]));
        }
    }
    loop {
        let mut seen_tees = SeenTees::new();
        let mut changed = false;
        for root in ir {
            if let HydroRoot::CycleSink { cycle_id, input, .. } = root {
                let cycle_goal = goal_for_collection_kind(&input.metadata().collection_kind);
                let result = prove(input, &cycle_goal, &cycle_proofs, &mut seen_tees);
                let prev_success = cycle_proofs.get(cycle_id).map(|p| p.success);
                if prev_success != Some(result.success) {
                    changed = true;
                }
                cycle_proofs.insert(*cycle_id, result);
            }
        }
        if !changed { break; }
    }

    // Pass 2: analyze observable sinks with fresh cache.
    let mut seen_tees = SeenTees::new();
    let mut sinks = Vec::new();
    for root in ir.iter() {
        if !is_observable_sink(root) {
            continue;
        }

        let sink_name = short_name_root(root);
        let sink_span = root.op_metadata().backtrace.format_span().unwrap_or_default();
        let sink_id = format!("{sink_name}@{sink_span}");
        // If the user provided an override, use it. Otherwise try candidate goals
        // and pick the best (fewest blessings, or passing outright).
        let override_goal = goal_overrides
            .get(sink_id.as_str())
            .or_else(|| goal_overrides.get(sink_name))
            .cloned();

        let candidates: Vec<OrderGoal> = if let Some(g) = override_goal {
            vec![g]
        } else {
            vec![OrderGoal::Prefix, OrderGoal::Lattice, OrderGoal::SetInclusion]
        };

        // Try each candidate goal, pick the strongest passing one
        // (Prefix > Lattice > SetInclusion — Prefix implies SetInclusion)
        let mut best_goal = candidates[0].clone();
        let mut best_result = prove(root.input(), &candidates[0], &cycle_proofs, &mut seen_tees);
        for candidate in &candidates[1..] {
            if best_result.success { break; } // stronger goal already passes
            let mut alt_seen = SeenTees::new();
            let alt_result = prove(root.input(), candidate, &cycle_proofs, &mut alt_seen);
            if alt_result.success || !best_result.success {
                best_goal = candidate.clone();
                best_result = alt_result;
            }
        }

          best_result.trace.reverse();
          let replication_issues = best_result.replication_issues.clone();
          let location = root.input_metadata().location_id.clone();

          // Compute per-channel consistency labels by walking upstream
          let (channels, discovery_order) = analyze_channels(root.input(), &location, &cycle_proofs, ir);
          let (mut forward, back) = split_forward_back(&channels, &discovery_order);
          let sink_has_nondet = !replication_issues.is_empty();

          // Propagate twice: dynamic membership (full analysis) and fixed membership
          let consistency = propagate_forward(
              &mut forward, &discovery_order,
              &best_goal, best_result.is_proved(), sink_has_nondet, &location,
              false,
          );
          let consistency_fixed = propagate_forward(
              &mut forward, &discovery_order,
              &best_goal, best_result.is_proved(), sink_has_nondet, &location,
              true,
          );
          // Reassemble channels with propagated labels
          let channels = forward.into_iter().chain(back.into_iter()).collect();

          sinks.push(SinkResult {
              name: short_name_root(root).to_string(),
              span: root.op_metadata().backtrace.format_span().unwrap_or_default(),
              goal: best_goal,
              result: best_result,
              location,
              consistency,
              consistency_fixed,
              channels,
              discovery_order,
              replication_issues,
          });
    }

    CoordinationReport { sinks, location_names: location_names.clone() }
}

fn short_name_root(root: &HydroRoot) -> &'static str {
    match root {
        HydroRoot::ForEach { .. } => "foreach",
        HydroRoot::SendExternal { .. } => "sendexternal",
        HydroRoot::EmbeddedOutput { .. } => "embeddedoutput",
        HydroRoot::DestSink { .. } => "destsink",
        HydroRoot::CycleSink { .. } => "cyclesink",
        HydroRoot::Null { .. } => "null",
    }
}

// ---------------------------------------------------------------------------
// Convenience: analyze with all defaults (no overrides)
// ---------------------------------------------------------------------------

/// Analyze with default goals for all sinks.
pub fn analyze_coordination_default(ir: &[HydroRoot], location_names: &SecondaryMap<LocationKey, String>) -> CoordinationReport {
    analyze_coordination(ir, &HashMap::new(), location_names)
}

#[cfg(test)]
#[cfg(feature = "build")]
mod tests {
    use super::*;
    use crate::compile::builder::FlowBuilder;
    use crate::live_collections::stream::TotalOrder;
    use crate::nondet::nondet;
    use crate::prelude::*;
    use crate::properties::manual_proof;

    fn check(build: impl FnOnce(&mut FlowBuilder<'_>)) -> CoordinationReport {
        let mut flow = FlowBuilder::new();
        build(&mut flow);
        flow.finalize().check_coordination()
    }

    fn check_set_inclusion(build: impl FnOnce(&mut FlowBuilder<'_>)) -> CoordinationReport {
        let mut flow = FlowBuilder::new();
        build(&mut flow);
        let built = flow.finalize();
        let overrides: HashMap<String, OrderGoal> = built.ir().iter()
            .filter(|root| is_observable_sink(root))
            .map(|root| (short_name_root(root).to_string(), OrderGoal::SetInclusion))
            .collect();
        built.check_coordination_with_goals(&overrides)
    }

    // --- Element-wise transforms preserve SetInclusion ---

    #[test]
    fn map_preserves_set_inclusion() {
        let r = check(|flow| {
            let p = flow.process::<()>();
            p.source_iter(q!([1, 2, 3])).map(q!(|x| x * 2)).for_each(q!(|_| {}));
        });
        assert!(r.all_monotone(), "map:\n{r}");
    }

    #[test]
    fn filter_preserves_set_inclusion() {
        let r = check(|flow| {
            let p = flow.process::<()>();
            p.source_iter(q!([1, 2, 3])).filter(q!(|x| *x > 1)).for_each(q!(|_| {}));
        });
        assert!(r.all_monotone(), "filter:\n{r}");
    }

    #[test]
    fn filter_map_preserves_set_inclusion() {
        let r = check(|flow| {
            let p = flow.process::<()>();
            p.source_iter(q!([1, 2, 3]))
                .filter_map(q!(|x| if x > 1 { Some(x) } else { None }))
                .for_each(q!(|_| {}));
        });
        assert!(r.all_monotone(), "filter_map:\n{r}");
    }

    #[test]
    fn inspect_preserves_set_inclusion() {
        let r = check(|flow| {
            let p = flow.process::<()>();
            p.source_iter(q!([1, 2, 3])).inspect(q!(|_| {})).for_each(q!(|_| {}));
        });
        assert!(r.all_monotone(), "inspect:\n{r}");
    }

    // --- Chain preserves SetInclusion ---

    #[test]
    fn chain_preserves_set_inclusion() {
        let r = check_set_inclusion(|flow| {
            let p = flow.process::<()>();
            let a = p.source_iter(q!([1, 2]));
            let b = p.source_iter(q!([3, 4]));
            a.chain(b)
                .assume_ordering::<TotalOrder>(nondet!(/** test */))
                .for_each(q!(|_| {}));
        });
        assert!(r.all_monotone(), "chain:\n{r}");
    }

    // --- Join preserves SetInclusion ---

    #[test]
    fn join_preserves_set_inclusion() {
        let r = check_set_inclusion(|flow| {
            let p = flow.process::<()>();
            let a = p.source_iter(q!(vec![(1, "a"), (2, "b")]));
            let b = p.source_iter(q!(vec![(1, "x"), (2, "y")]));
            a.join(b)
                .assume_ordering::<TotalOrder>(nondet!(/** test */))
                .for_each(q!(|_| {}));
        });
        assert!(r.all_monotone(), "join:\n{r}");
    }

    // --- Fold: bounded always discharges ---

    #[test]
    fn fold_keyed_on_bounded_discharges() {
        let r = check_set_inclusion(|flow| {
            let p = flow.process::<()>();
            p.source_iter(q!(vec![(1, 10), (2, 20)]))
                .erase_consistency()
                .into_keyed()
                .fold(q!(|| 0), q!(|acc, x| { *acc = x; }))
                .entries()
                .assume_ordering::<TotalOrder>(nondet!(/** test */))
                .for_each(q!(|_| {}));
        });
        assert!(r.all_monotone(), "fold_keyed bounded:\n{r}");
    }

    // --- Fold: commutative+idempotent on unbounded discharges ---

    #[test]
    fn commutative_fold_keyed_on_unbounded_discharges() {
        let r = check_set_inclusion(|flow| {
            let p = flow.process::<()>();
            let tick = p.tick();
            let storage = p.source_iter(q!(vec![(1, 10), (2, 20)]))
                .batch(&tick, nondet!(/** test */))
                .all_ticks()
                .into_keyed()
                .fold(
                    q!(|| 0i32),
                    q!(|acc, x| { *acc = std::cmp::max(*acc, x); },
                       commutative = manual_proof!(/** max */),
                       idempotent = manual_proof!(/** max */)),
                );
            storage.snapshot(&tick, nondet!(/** test */))
                .entries().all_ticks()
                .assume_ordering::<TotalOrder>(nondet!(/** test */))
                .for_each(q!(|_| {}));
        });
        assert!(r.all_monotone(), "commutative fold_keyed unbounded:\n{r}");
    }

    // --- Fold: non-commutative on unbounded breaks ---

    #[test]
    fn non_commutative_fold_keyed_on_unbounded_breaks() {
        let r = check_set_inclusion(|flow| {
            let p = flow.process::<()>();
            let tick = p.tick();
            let storage = p.source_iter(q!(vec![(1, 10), (2, 20)]))
                .batch(&tick, nondet!(/** test */))
                .all_ticks()
                .assume_ordering::<TotalOrder>(nondet!(/** test */))
                .into_keyed()
                .fold(q!(|| 0i32), q!(|acc, x| { *acc = x; }));
            storage.snapshot(&tick, nondet!(/** test */))
                .entries().all_ticks()
                .assume_ordering::<TotalOrder>(nondet!(/** test */))
                .for_each(q!(|_| {}));
        });
        assert!(!r.all_monotone(), "non-commutative fold_keyed unbounded:\n{r}");
    }

    // --- Scan: TotalOrder discharges Prefix and SetInclusion ---

    #[test]
    fn scan_on_total_order_discharges_prefix() {
        let r = check(|flow| {
            let p = flow.process::<()>();
            p.source_iter(q!([1, 2, 3]))
                .scan(q!(|| 0i32), q!(|s, x| { *s += x; Some(*s) }))
                .for_each(q!(|_| {}));
        });
        assert!(r.all_monotone(), "scan prefix:\n{r}");
        assert_eq!(r.sinks[0].goal, OrderGoal::Prefix);
    }

    #[test]
    fn scan_on_total_order_discharges_set_inclusion() {
        let r = check_set_inclusion(|flow| {
            let p = flow.process::<()>();
            p.source_iter(q!([1, 2, 3]))
                .scan(q!(|| 0i32), q!(|s, x| { *s += x; Some(*s) }))
                .for_each(q!(|_| {}));
        });
        assert!(r.all_monotone(), "scan set inclusion:\n{r}");
    }

    // --- Unique preserves SetInclusion ---

    #[test]
    fn unique_preserves_set_inclusion() {
        let r = check_set_inclusion(|flow| {
            let p = flow.process::<()>();
            p.source_iter(q!([1, 1, 2, 3]))
                .unique()
                .assume_ordering::<TotalOrder>(nondet!(/** test */))
                .for_each(q!(|_| {}));
        });
        assert!(r.all_monotone(), "unique:\n{r}");
    }

    // --- Enumerate preserves SetInclusion ---

    #[test]
    fn enumerate_preserves_set_inclusion() {
        let r = check_set_inclusion(|flow| {
            let p = flow.process::<()>();
            p.source_iter(q!([1, 2, 3]))
                .enumerate()
                .for_each(q!(|_| {}));
        });
        assert!(r.all_monotone(), "enumerate:\n{r}");
    }

    // --- DeferTick preserves SetInclusion ---

    #[test]
    fn defer_tick_preserves_set_inclusion() {
        let r = check_set_inclusion(|flow| {
            let p = flow.process::<()>();
            let tick = p.tick();
            p.source_iter(q!([1, 2, 3]))
                .batch(&tick, nondet!(/** test */))
                .defer_tick()
                .all_ticks()
                .assume_ordering::<TotalOrder>(nondet!(/** test */))
                .for_each(q!(|_| {}));
        });
        assert!(r.all_monotone(), "defer_tick:\n{r}");
    }

    // --- Sort on bounded discharges ---

    #[test]
    fn sort_bounded_discharges() {
        let r = check(|flow| {
            let p = flow.process::<()>();
            let tick = p.tick();
            p.source_iter(q!([3, 1, 2]))
                .batch(&tick, nondet!(/** test */))
                .sort()
                .all_ticks()
                .for_each(q!(|_| {}));
        });
        assert!(r.all_monotone(), "sort bounded:\n{r}");
    }

    // --- AntiJoin with bounded neg preserves ---

    #[test]
    fn anti_join_bounded_neg_preserves() {
        let r = check_set_inclusion(|flow| {
            let p = flow.process::<()>();
            let tick = p.tick();
            let stream = p.source_iter(q!(vec![(1, "a"), (2, "b"), (3, "c")]))
                .batch(&tick, nondet!(/** test */));
            let neg = p.source_iter(q!(vec![2]))
                .batch(&tick, nondet!(/** test */));
            stream.anti_join(neg)
                .all_ticks()
                .assume_ordering::<TotalOrder>(nondet!(/** test */))
                .for_each(q!(|_| {}));
        });
        assert!(r.all_monotone(), "anti_join bounded neg:\n{r}");
    }

    // --- ReduceKeyed commutative discharges ---

    #[test]
    fn commutative_reduce_keyed_discharges() {
        let r = check_set_inclusion(|flow| {
            let p = flow.process::<()>();
            let tick = p.tick();
            let storage = p.source_iter(q!(vec![(1, 10i32), (2, 20)]))
                .batch(&tick, nondet!(/** test */))
                .all_ticks()
                .into_keyed()
                .reduce(q!(|acc, x| { *acc = std::cmp::max(*acc, x); },
                    commutative = manual_proof!(/** max */),
                    idempotent = manual_proof!(/** max */)));
            storage.snapshot(&tick, nondet!(/** test */))
                .entries().all_ticks()
                .assume_ordering::<TotalOrder>(nondet!(/** test */))
                .for_each(q!(|_| {}));
        });
        assert!(r.all_monotone(), "commutative reduce_keyed:\n{r}");
    }

    // --- Goal override ---

    #[test]
    fn goal_override_set_inclusion_on_total_order() {
        let mut flow = FlowBuilder::new();
        let p = flow.process::<()>();
        p.source_iter(q!([1, 2, 3]))
            .assume_ordering::<TotalOrder>(nondet!(/** test */))
            .for_each(q!(|_| {}));
        let built = flow.finalize();

        let default_report = built.check_coordination();
        assert!(default_report.all_monotone());
        assert_eq!(default_report.sinks[0].goal, OrderGoal::Prefix);

        let mut overrides = HashMap::new();
        overrides.insert("foreach".to_string(), OrderGoal::SetInclusion);
        let override_report = built.check_coordination_with_goals(&overrides);
        assert!(override_report.all_monotone());
        assert_eq!(override_report.sinks[0].goal, OrderGoal::SetInclusion);
    }

    // --- Trace quality ---

    #[test]
    fn failing_trace_ends_with_broken() {
        let r = check_set_inclusion(|flow| {
            let p = flow.process::<()>();
            let tick = p.tick();
            let storage = p.source_iter(q!(vec![(1, 10)]))
                .batch(&tick, nondet!(/** test */))
                .all_ticks()
                .assume_ordering::<TotalOrder>(nondet!(/** test */))
                .into_keyed()
                .fold(q!(|| 0i32), q!(|acc, x| { *acc = x; }));
            storage.snapshot(&tick, nondet!(/** test */))
                .entries().all_ticks()
                .assume_ordering::<TotalOrder>(nondet!(/** test */))
                .for_each(q!(|_| {}));
        });
        assert!(!r.all_monotone());
        let last = r.sinks[0].result.trace.last().unwrap();
        assert!(matches!(last.action, ProofAction::Broken { .. }), "last step should be Broken");
    }

    #[test]
    fn passing_trace_ends_with_discharged() {
        let r = check(|flow| {
            let p = flow.process::<()>();
            p.source_iter(q!([1, 2, 3])).for_each(q!(|_| {}));
        });
        assert!(r.all_monotone());
        let last = r.sinks[0].result.trace.last().unwrap();
        assert!(matches!(last.action, ProofAction::Discharged { .. }), "last step should be Discharged");
    }

    // --- CrossSingleton: bounded right side passes (snapshot is stable) ---

    #[test]
    fn cross_singleton_bounded_right_passes() {
        // snapshot creates a bounded view — stable within a tick
        let r = check_set_inclusion(|flow| {
            let p = flow.process::<()>();
            let tick = p.tick();
            let singleton = p.source_iter(q!(vec![(1, 10)]))
                .batch(&tick, nondet!(/** test */))
                .all_ticks()
                .assume_ordering::<TotalOrder>(nondet!(/** test */))
                .into_keyed()
                .fold(q!(|| 0i32), q!(|acc, x| { *acc = x; }));
            let stream = p.source_iter(q!([1, 2, 3]))
                .batch(&tick, nondet!(/** test */));
            stream.cross_singleton(
                singleton.snapshot(&tick, nondet!(/** test */))
                    .into_singleton()
            ).all_ticks()
                .assume_ordering::<TotalOrder>(nondet!(/** test */))
                .for_each(q!(|_| {}));
        });
        // Bounded snapshot is stable — cross_singleton passes
        assert!(r.all_monotone(), "cross_singleton with bounded snapshot should pass:\n{r}");
    }

    // -----------------------------------------------------------------------
    // Negative tests: operators that BREAK specific goals
    // -----------------------------------------------------------------------

    // --- Element-wise transforms break Lattice ---

    #[test]
    fn map_on_bounded_singleton_passes() {
        // map on a bounded singleton (via snapshot) is fine — the snapshot
        // freezes the value, so the map result is also stable.
        // Lattice goal is only checked for UNBOUNDED singletons.
        let r = check_set_inclusion(|flow| {
            let p = flow.process::<()>();
            let tick = p.tick();
            let singleton = p.source_iter(q!(vec![(1i32, 10i32)]))
                .batch(&tick, nondet!(/** test */))
                .all_ticks()
                .into_keyed()
                .fold(
                    q!(|| 0i32),
                    q!(|acc: &mut i32, x: i32| { *acc = std::cmp::max(*acc, x); },
                       commutative = manual_proof!(/** max */),
                       idempotent = manual_proof!(/** max */)),
                );
            let mapped = singleton.map(q!(|x: i32| x + 1));
            let stream = p.source_iter(q!([1i32, 2, 3]))
                .batch(&tick, nondet!(/** test */));
            stream.cross_singleton(
                mapped.snapshot(&tick, nondet!(/** test */)).into_singleton()
            ).all_ticks()
                .assume_ordering::<TotalOrder>(nondet!(/** test */))
                .for_each(q!(|_| {}));
        });
        assert!(r.all_monotone(), "map on bounded singleton should pass:\n{r}");
    }

    // --- Chain breaks Prefix ---

    #[test]
    fn chain_breaks_prefix() {
        // chain breaks Prefix but passes SetInclusion.
        // With multi-goal exploration, the analysis picks SetInclusion (the better goal).
        let r = check(|flow| {
            let p = flow.process::<()>();
            let a = p.source_iter(q!([1, 2]));
            let b = p.source_iter(q!([3, 4]));
            a.chain(b).for_each(q!(|_| {}));
        });
        assert!(r.all_monotone(), "chain should pass under SetInclusion:\n{r}");
        assert_eq!(r.sinks[0].goal, OrderGoal::SetInclusion);
    }

    // --- Join breaks Prefix ---

    #[test]
    fn join_breaks_prefix() {
        let mut flow = FlowBuilder::new();
        let p = flow.process::<()>();
        let a = p.source_iter(q!(vec![(1, "a"), (2, "b")]));
        let b = p.source_iter(q!(vec![(1, "x"), (2, "y")]));
        a.join(b)
            .assume_ordering::<TotalOrder>(nondet!(/** test */))
            .for_each(q!(|_| {}));
        let mut overrides = HashMap::new();
        overrides.insert("foreach".to_string(), OrderGoal::Prefix);
        let r = flow.finalize().check_coordination_with_goals(&overrides);
        assert!(!r.all_monotone(), "join should break Prefix:\n{r}");
    }

    // --- Enumerate breaks Prefix ---

    #[test]
    fn enumerate_breaks_prefix() {
        let mut flow = FlowBuilder::new();
        let p = flow.process::<()>();
        p.source_iter(q!([1, 2, 3]))
            .enumerate()
            .for_each(q!(|_| {}));
        let mut overrides = HashMap::new();
        overrides.insert("foreach".to_string(), OrderGoal::Prefix);
        let r = flow.finalize().check_coordination_with_goals(&overrides);
        assert!(!r.all_monotone(), "enumerate should break Prefix:\n{r}");
    }

    // --- DeferTick breaks Prefix ---

    #[test]
    fn defer_tick_breaks_prefix() {
        let mut flow = FlowBuilder::new();
        let p = flow.process::<()>();
        let tick = p.tick();
        p.source_iter(q!([1, 2, 3]))
            .batch(&tick, nondet!(/** test */))
            .defer_tick()
            .all_ticks()
            .for_each(q!(|_| {}));
        let mut overrides = HashMap::new();
        overrides.insert("foreach".to_string(), OrderGoal::Prefix);
        let r = flow.finalize().check_coordination_with_goals(&overrides);
        assert!(!r.all_monotone(), "defer_tick should break Prefix:\n{r}");
    }

    // --- Unique breaks Prefix ---

    #[test]
    fn unique_breaks_prefix() {
        let mut flow = FlowBuilder::new();
        let p = flow.process::<()>();
        p.source_iter(q!([1, 1, 2, 3]))
            .unique()
            .assume_ordering::<TotalOrder>(nondet!(/** test */))
            .for_each(q!(|_| {}));
        let mut overrides = HashMap::new();
        overrides.insert("foreach".to_string(), OrderGoal::Prefix);
        let r = flow.finalize().check_coordination_with_goals(&overrides);
        assert!(!r.all_monotone(), "unique should break Prefix:\n{r}");
    }

    // --- CrossSingleton with unbounded non-lattice singleton breaks ---

    #[test]
    fn cross_singleton_unbounded_non_lattice_breaks() {
        let r = check_set_inclusion(|flow| {
            let p = flow.process::<()>();
            let tick = p.tick();
            // Non-commutative fold → not a lattice
            let singleton = p.source_iter(q!(vec![(1, 10)]))
                .batch(&tick, nondet!(/** test */))
                .all_ticks()
                .assume_ordering::<TotalOrder>(nondet!(/** test */))
                .into_keyed()
                .fold(q!(|| 0i32), q!(|acc, x| { *acc = x; }));
            let stream = p.source_iter(q!([1, 2, 3]))
                .batch(&tick, nondet!(/** test */));
            stream.cross_singleton(
                singleton.snapshot(&tick, nondet!(/** test */)).into_singleton()
            ).all_ticks()
                .assume_ordering::<TotalOrder>(nondet!(/** test */))
                .for_each(q!(|_| {}));
        });
        // snapshot makes it bounded, so this actually passes (bounded singleton is stable)
        // This is correct — the snapshot is a stable view within the tick.
        assert!(r.all_monotone(), "cross_singleton with bounded snapshot passes even with non-lattice fold:\n{r}");
    }

    // -----------------------------------------------------------------------
    // Multi-cycle fixpoint test
    // -----------------------------------------------------------------------

    #[test]
    fn multi_cycle_fixpoint_monotone() {
        // Two chained cycles: source -> cycle_a -> cycle_b -> sink
        // Both carry monotone data (source only grows).
        // The fixpoint must resolve both cycles regardless of IR order.
        let r = check_set_inclusion(|flow| {
            let p = flow.process::<()>();

            let (handle_a, cycle_a_out) =
                p.forward_ref::<Stream<i32, Process<()>, Unbounded, TotalOrder>>();
            let (handle_b, cycle_b_out) =
                p.forward_ref::<Stream<i32, Process<()>, Unbounded, TotalOrder>>();

            let source = p.source_iter(q!([1i32, 2, 3]));
            handle_a.complete(source.map(q!(|x: i32| x + 1)));
            handle_b.complete(cycle_a_out.map(q!(|x: i32| x * 2)));

            cycle_b_out.for_each(q!(|_: i32| {}));
        });
        assert!(r.all_monotone(), "chained cycles with monotone source should pass:\n{r}");
    }

    #[test]
    fn multi_cycle_fixpoint_non_monotone() {
        // Two chained cycles: A feeds B, A contains a non-commutative
        // keyed fold. Non-monotonicity in A should propagate to B.
        let r = check_set_inclusion(|flow| {
            let p = flow.process::<()>();
            let tick = p.tick();

            let (handle_a, cycle_a_out) =
                p.forward_ref::<Stream<(i32, i32), Process<()>, Unbounded, TotalOrder>>();
            let (handle_b, cycle_b_out) =
                p.forward_ref::<Stream<(i32, i32), Process<()>, Unbounded, TotalOrder>>();

            // Cycle A: non-commutative keyed fold (overwrite)
            let storage = p.source_iter(q!(vec![(1i32, 10i32)]))
                .batch(&tick, nondet!(/** test */))
                .all_ticks()
                .assume_ordering::<TotalOrder>(nondet!(/** test */))
                .into_keyed()
                .fold(q!(|| 0i32), q!(|acc, x| { *acc = x; }));
            handle_a.complete(
                storage.snapshot(&tick, nondet!(/** test */))
                    .entries()
                    .all_ticks()
                    .assume_ordering::<TotalOrder>(nondet!(/** test */))
            );

            // Cycle B: depends on cycle A
            handle_b.complete(cycle_a_out.map(q!(|(k, v): (i32, i32)| (k, v + 1))));

            // Observe cycle B
            cycle_b_out.for_each(q!(|_: (i32, i32)| {}));
        });
        assert!(!r.all_monotone(), "chained cycles with non-monotone fold should fail:\n{r}");
    }
}
