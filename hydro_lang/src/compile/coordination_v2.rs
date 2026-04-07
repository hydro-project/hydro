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
use std::collections::HashMap;
use std::fmt;

use super::builder::CycleId;
use super::ir::backtrace::Backtrace;
use super::ir::{HydroNode, HydroRoot, SharedNode, StreamOrder};
use crate::location::dynamic::LocationId;

// ---------------------------------------------------------------------------
// Order goals
// ---------------------------------------------------------------------------

/// The partial order under which we're trying to prove monotonicity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OrderGoal {
    /// Growing prefix of a deterministic sequence (TotalOrder streams).
    Prefix,
    /// Elements only accumulate; no retractions (NoOrder streams).
    SetInclusion,
    /// Value only grows under lattice join (singletons from commutative+idempotent fold).
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

/// Determine the default proof goal for a sink based on its input's collection kind.
/// This can be overridden by the user (future API).
fn default_goal_for_sink(root: &HydroRoot) -> OrderGoal {
    let meta = root.input_metadata();
    match &meta.collection_kind {
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
    /// Source location of the operator in user code, if available.
    pub span: Option<String>,
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
}

/// Result of the backward proof walk for a single path.
#[derive(Clone, Debug)]
pub struct ProofResult {
    pub success: bool,
    /// The walk trace, from sink (first) to source/break point (last).
    pub trace: Vec<ProofStep>,
}

impl ProofResult {
    pub fn is_proved(&self) -> bool {
        self.success
    }

    fn proved(trace: Vec<ProofStep>) -> Self {
        Self { success: true, trace }
    }

    fn broken(trace: Vec<ProofStep>) -> Self {
        Self { success: false, trace }
    }

    fn discharged(operator: &str, reason: impl Into<String>, span: Option<String>) -> Self {
        Self::proved(vec![ProofStep {
            operator: operator.to_string(),
            action: ProofAction::Discharged { reason: reason.into() },
            span,
        }])
    }

    fn fail(operator: &str, reason: impl Into<String>, span: Option<String>) -> Self {
        Self::broken(vec![ProofStep {
            operator: operator.to_string(),
            action: ProofAction::Broken { reason: reason.into() },
            span,
        }])
    }

    /// Prepend a "preserved" step from the current operator.
    fn prepend_preserved(mut self, operator: &str, span: Option<String>) -> Self {
        self.trace.insert(0, ProofStep {
            operator: operator.to_string(),
            action: ProofAction::Preserved,
            span,
        });
        self
    }

    /// Prepend a "goal changed" step.
    fn prepend_goal_changed(mut self, operator: &str, new_goal: &OrderGoal, span: Option<String>) -> Self {
        self.trace.insert(0, ProofStep {
            operator: operator.to_string(),
            action: ProofAction::GoalChanged { new_goal: new_goal.clone() },
            span,
        });
        self
    }
}

// ---------------------------------------------------------------------------
// Span formatting
// ---------------------------------------------------------------------------

#[cfg(feature = "build")]
fn format_span(bt: &Backtrace) -> Option<String> {
    let elem = bt.elements().next()?;
    let file = elem.filename.as_ref()?;
    let line = elem.lineno?;
    let col = elem.colno.unwrap_or(0);
    Some(format!("{file}:{line}:{col}"))
}

#[cfg(not(feature = "build"))]
fn format_span(_bt: &Backtrace) -> Option<String> {
    None
}

fn node_span(node: &HydroNode) -> Option<String> {
    format_span(&node.metadata().op.backtrace)
}

// ---------------------------------------------------------------------------
// Sink annotation
// ---------------------------------------------------------------------------

/// Analysis result for a single observable sink.
#[derive(Clone)]
pub struct SinkResult {
    pub name: String,
    pub goal: OrderGoal,
    pub result: ProofResult,
    pub backtrace: Backtrace,
    pub location: LocationId,
}

// ---------------------------------------------------------------------------
// Report
// ---------------------------------------------------------------------------

/// Full coordination analysis report.
pub struct CoordinationReport {
    pub sinks: Vec<SinkResult>,
}

impl CoordinationReport {
    /// Whether all observable sinks are proved monotone.
    pub fn all_monotone(&self) -> bool {
        self.sinks.iter().all(|s| s.result.is_proved())
    }
}

impl fmt::Display for CoordinationReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.sinks.is_empty() {
            return write!(f, "Coordination Criterion: no observable sinks found");
        }

        let total = self.sinks.len();
        let failing: Vec<_> = self.sinks.iter().filter(|s| !s.result.is_proved()).collect();

        if failing.is_empty() {
            writeln!(f, "Coordination Criterion: PASS — all {total} sinks are future-monotone")?;
        } else {
            writeln!(
                f,
                "Coordination Criterion: FAIL — {}/{total} sinks require coordination",
                failing.len()
            )?;
        }

        for s in &self.sinks {
            if s.result.is_proved() {
                writeln!(f, "\n  ✓ {} ({})", s.name, s.goal)?;
                for step in &s.result.trace {
                    let span = step.span.as_deref().unwrap_or("");
                    match &step.action {
                        ProofAction::Preserved => {
                            writeln!(f, "    {} — preserved  {}", step.operator, span)?;
                        }
                        ProofAction::Discharged { reason } => {
                            writeln!(f, "    {} — ✓ discharged: {}  {}", step.operator, reason, span)?;
                        }
                        ProofAction::GoalChanged { new_goal } => {
                            writeln!(f, "    {} — goal → {}  {}", step.operator, new_goal, span)?;
                        }
                        ProofAction::Broken { .. } => {} // shouldn't appear in proved
                    }
                }
            } else {
                writeln!(f, "\n  ✗ {} (goal: {})", s.name, s.goal)?;
                for step in &s.result.trace {
                    let span = step.span.as_deref().unwrap_or("");
                    match &step.action {
                        ProofAction::Preserved => {
                            writeln!(f, "    {} — preserved  {}", step.operator, span)?;
                        }
                        ProofAction::Broken { reason } => {
                            writeln!(f, "    {} — ✗ BROKEN: {}  {}", step.operator, reason, span)?;
                        }
                        ProofAction::GoalChanged { new_goal } => {
                            writeln!(f, "    {} — goal → {}  {}", step.operator, new_goal, span)?;
                        }
                        ProofAction::Discharged { .. } => {} // shouldn't appear in broken
                    }
                }
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Analysis state
// ---------------------------------------------------------------------------

type SeenTees = HashMap<*const RefCell<HydroNode>, ProofResult>;
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

fn short_name(node: &HydroNode) -> String {
    let full = node.print_root();
    full.split('(').next().unwrap_or(&full).to_lowercase()
}

// ---------------------------------------------------------------------------
// Core backward walk
// ---------------------------------------------------------------------------

/// Walk backward from `node`, trying to prove `goal`. Returns the proof result with trace.
fn prove(
    node: &HydroNode,
    goal: &OrderGoal,
    cycle_proofs: &CycleProofs,
    seen_tees: &mut SeenTees,
) -> ProofResult {
    let name = short_name(node);
    let span = node_span(node);

    match node {
        HydroNode::Placeholder => ProofResult::proved(vec![]),

        // --- Sources: discharge any goal ---
        HydroNode::Source { .. }
        | HydroNode::SingletonSource { .. }
        | HydroNode::ExternalInput { .. } => {
            ProofResult::discharged(&name, "source — data only arrives", span)
        }

        // --- CycleSource: inherit from matching CycleSink ---
        HydroNode::CycleSource { cycle_id, .. } => {
            match cycle_proofs.get(cycle_id) {
                Some(r) => r.clone().prepend_preserved(&name, span),
                None => ProofResult::discharged(&name, "cycle source (no matching sink)", span),
            }
        }

        // --- Tee: shared node ---
        HydroNode::Tee { inner, .. } => {
            prove_shared(inner, goal, cycle_proofs, seen_tees).prepend_preserved(&name, span)
        }
        HydroNode::Partition { inner, .. } => {
            prove_shared(inner, goal, cycle_proofs, seen_tees).prepend_preserved(&name, span)
        }

        // --- Structural pass-through ---
        HydroNode::Cast { inner, .. }
        | HydroNode::ObserveNonDet { inner, .. }
        | HydroNode::BeginAtomic { inner, .. }
        | HydroNode::EndAtomic { inner, .. }
        | HydroNode::YieldConcat { inner, .. } => {
            prove(inner, goal, cycle_proofs, seen_tees).prepend_preserved(&name, span)
        }

        // --- Element-wise transforms ---
        HydroNode::Map { input, .. }
        | HydroNode::FlatMap { input, .. }
        | HydroNode::Filter { input, .. }
        | HydroNode::FilterMap { input, .. }
        | HydroNode::Inspect { input, .. } => match goal {
            OrderGoal::SetInclusion | OrderGoal::Prefix => {
                prove(input, goal, cycle_proofs, seen_tees).prepend_preserved(&name, span)
            }
            OrderGoal::Lattice => ProofResult::fail(&name, "may not preserve lattice order (closure not annotated)", span),
        },

        HydroNode::Enumerate { input, .. } => match goal {
            OrderGoal::SetInclusion => prove(input, goal, cycle_proofs, seen_tees).prepend_preserved(&name, span),
            _ => ProofResult::fail(&name, "enumerate breaks prefix/lattice order", span),
        },

        HydroNode::Unique { input, .. } => match goal {
            OrderGoal::SetInclusion => prove(input, goal, cycle_proofs, seen_tees).prepend_preserved(&name, span),
            _ => ProofResult::fail(&name, "unique breaks prefix/lattice order", span),
        },

        HydroNode::Network { input, .. } => match goal {
            OrderGoal::SetInclusion | OrderGoal::Lattice => {
                prove(input, goal, cycle_proofs, seen_tees).prepend_preserved(&name, span)
            }
            OrderGoal::Prefix => ProofResult::fail(&name, "network may reorder elements", span),
        },

        HydroNode::Counter { .. } => ProofResult::discharged(&name, "count only grows", span),

        HydroNode::Batch { inner, .. } => match goal {
            OrderGoal::SetInclusion => prove(inner, goal, cycle_proofs, seen_tees).prepend_preserved(&name, span),
            _ => ProofResult::fail(&name, "batch windowing breaks prefix/lattice order", span),
        },

        // --- Chain ---
        HydroNode::Chain { first, second, .. }
        | HydroNode::ChainFirst { first, second, .. } => match goal {
            OrderGoal::SetInclusion => {
                let a = prove(first, goal, cycle_proofs, seen_tees);
                if !a.is_proved() { return a.prepend_preserved(&name, span.clone()); }
                prove(second, goal, cycle_proofs, seen_tees).prepend_preserved(&name, span)
            }
            _ => ProofResult::fail(&name, "union/chain breaks prefix/lattice order", span),
        },

        // --- Join / CrossProduct ---
        HydroNode::Join { left, right, .. }
        | HydroNode::CrossProduct { left, right, .. } => match goal {
            OrderGoal::SetInclusion => {
                let a = prove(left, &OrderGoal::SetInclusion, cycle_proofs, seen_tees);
                if !a.is_proved() { return a.prepend_preserved(&name, span.clone()); }
                prove(right, &OrderGoal::SetInclusion, cycle_proofs, seen_tees).prepend_preserved(&name, span)
            }
            _ => ProofResult::fail(&name, "join breaks prefix/lattice order", span),
        },

        // --- CrossSingleton ---
        HydroNode::CrossSingleton { left, right, .. } => match goal {
            OrderGoal::SetInclusion => {
                let a = prove(left, &OrderGoal::SetInclusion, cycle_proofs, seen_tees);
                if !a.is_proved() { return a.prepend_preserved(&name, span.clone()); }
                prove(right, &OrderGoal::Lattice, cycle_proofs, seen_tees)
                    .prepend_goal_changed(&name, &OrderGoal::Lattice, span)
            }
            _ => ProofResult::fail(&name, "cross_singleton breaks prefix/lattice order", span),
        },

        // --- Difference / AntiJoin ---
        HydroNode::Difference { pos, neg, .. } => match goal {
            OrderGoal::SetInclusion => {
                let p = prove(pos, &OrderGoal::SetInclusion, cycle_proofs, seen_tees);
                if !p.is_proved() { return p.prepend_preserved(&name, span.clone()); }
                if neg.metadata().collection_kind.is_bounded() {
                    ProofResult::discharged(&name, "neg input is bounded", span)
                } else {
                    ProofResult::fail(&name, "unbounded neg input can retract output elements", span)
                }
            }
            _ => ProofResult::fail(&name, "difference breaks prefix/lattice order", span),
        },
        HydroNode::AntiJoin { pos, neg, .. } => match goal {
            OrderGoal::SetInclusion => {
                let p = prove(pos, &OrderGoal::SetInclusion, cycle_proofs, seen_tees);
                if !p.is_proved() { return p.prepend_preserved(&name, span.clone()); }
                if neg.metadata().collection_kind.is_bounded() {
                    ProofResult::discharged(&name, "neg input is bounded", span)
                } else {
                    ProofResult::fail(&name, "unbounded neg input can retract output elements", span)
                }
            }
            _ => ProofResult::fail(&name, "anti_join breaks prefix/lattice order", span),
        },

        // --- Fold / FoldKeyed ---
        HydroNode::Fold { is_commutative, is_idempotent, input, .. } => {
            if *is_commutative && *is_idempotent {
                ProofResult::discharged(&name, "commutative + idempotent fold (lattice join)", span)
            } else if input.metadata().collection_kind.is_bounded() {
                ProofResult::discharged(&name, "fold over bounded input", span)
            } else {
                ProofResult::fail(&name, "fold over unbounded input without commutativity+idempotency proof", span)
            }
        }
        HydroNode::FoldKeyed { is_commutative, is_idempotent, input, .. } => {
            if *is_commutative && *is_idempotent {
                ProofResult::discharged(&name, "commutative + idempotent fold (lattice join)", span)
            } else if input.metadata().collection_kind.is_bounded() {
                ProofResult::discharged(&name, "fold over bounded input", span)
            } else {
                ProofResult::fail(&name, "fold_keyed over unbounded input without commutativity+idempotency proof", span)
            }
        }

        // --- Reduce / ReduceKeyed ---
        HydroNode::Reduce { is_commutative, input, .. } => {
            if *is_commutative {
                ProofResult::discharged(&name, "commutative reduce (lattice join)", span)
            } else if input.metadata().collection_kind.is_bounded() {
                ProofResult::discharged(&name, "reduce over bounded input", span)
            } else {
                ProofResult::fail(&name, "reduce over unbounded input without commutativity proof", span)
            }
        }
        HydroNode::ReduceKeyed { is_commutative, input, .. } => {
            if *is_commutative {
                ProofResult::discharged(&name, "commutative reduce (lattice join)", span)
            } else if input.metadata().collection_kind.is_bounded() {
                ProofResult::discharged(&name, "reduce over bounded input", span)
            } else {
                ProofResult::fail(&name, "reduce_keyed over unbounded input without commutativity proof", span)
            }
        }

        HydroNode::ReduceKeyedWatermark { input, .. } => {
            if input.metadata().collection_kind.is_bounded() {
                ProofResult::discharged(&name, "watermark reduce over bounded input", span)
            } else {
                ProofResult::fail(&name, "watermark-based reduce may retract above watermark", span)
            }
        }

        // --- Scan ---
        HydroNode::Scan { input, .. } => match &input.metadata().collection_kind {
            super::ir::CollectionKind::Stream { order: StreamOrder::TotalOrder, .. } => match goal {
                OrderGoal::Prefix | OrderGoal::SetInclusion => {
                    ProofResult::discharged(&name, "scan on TotalOrder input (deterministic prefix)", span)
                }
                OrderGoal::Lattice => ProofResult::fail(&name, "scan produces a stream, not a lattice singleton", span),
            },
            _ => {
                if input.metadata().collection_kind.is_bounded() {
                    ProofResult::discharged(&name, "scan over bounded input", span)
                } else {
                    ProofResult::fail(&name, "scan on non-TotalOrder unbounded input is non-deterministic", span)
                }
            }
        },

        // --- Sort ---
        HydroNode::Sort { input, .. } => {
            if input.metadata().collection_kind.is_bounded() {
                ProofResult::discharged(&name, "sort over bounded input", span)
            } else {
                ProofResult::fail(&name, "sort on unbounded input commits to order that may change", span)
            }
        }

        // --- DeferTick ---
        HydroNode::DeferTick { input, .. } => match goal {
            OrderGoal::SetInclusion | OrderGoal::Lattice => {
                prove(input, goal, cycle_proofs, seen_tees).prepend_preserved(&name, span)
            }
            OrderGoal::Prefix => ProofResult::fail(&name, "defer_tick breaks prefix order", span),
        },

        // --- Futures ---
        HydroNode::ResolveFutures { input, .. }
        | HydroNode::ResolveFuturesBlocking { input, .. }
        | HydroNode::ResolveFuturesOrdered { input, .. } => match goal {
            OrderGoal::SetInclusion => prove(input, goal, cycle_proofs, seen_tees).prepend_preserved(&name, span),
            _ => ProofResult::fail(&name, "future resolution may reorder", span),
        },
    }
}


fn prove_shared(
    inner: &SharedNode,
    goal: &OrderGoal,
    cycle_proofs: &CycleProofs,
    seen_tees: &mut SeenTees,
) -> ProofResult {
    let ptr = inner.as_ptr();
    if let Some(result) = seen_tees.get(&ptr) {
        return result.clone();
    }
    // Placeholder to break cycles
    seen_tees.insert(ptr, ProofResult::proved(vec![]));
    let result = prove(&inner.0.borrow(), goal, cycle_proofs, seen_tees);
    seen_tees.insert(ptr, result.clone());
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
/// specific sinks (identified by index in the IR root list). Pass an empty
/// map to use all defaults.
pub fn analyze_coordination(
    ir: &[HydroRoot],
    goal_overrides: &HashMap<usize, OrderGoal>,
) -> CoordinationReport {
    // Pass 1: analyze CycleSink roots to determine cycle monotonicity.
    let mut cycle_proofs = CycleProofs::new();
    let mut seen_tees = SeenTees::new();
    for root in ir {
        if let HydroRoot::CycleSink { cycle_id, input, .. } = root {
            // For cycle sinks, we try SetInclusion as the default goal
            // since cycles typically carry streams.
            let result = prove(input, &OrderGoal::SetInclusion, &cycle_proofs, &mut seen_tees);
            cycle_proofs.insert(*cycle_id, result);
        }
    }

    // Pass 2: analyze observable sinks.
    let mut sinks = Vec::new();
    for (i, root) in ir.iter().enumerate() {
        if !is_observable_sink(root) {
            continue;
        }

        let goal = goal_overrides
            .get(&i)
            .cloned()
            .unwrap_or_else(|| default_goal_for_sink(root));

        let result = prove(root.input(), &goal, &cycle_proofs, &mut seen_tees);

        sinks.push(SinkResult {
            name: short_name_root(root),
            goal,
            result,
            backtrace: root.op_metadata().backtrace.clone(),
            location: root.input_metadata().location_id.clone(),
        });
    }

    CoordinationReport { sinks }
}

fn short_name_root(root: &HydroRoot) -> String {
    let full = root.print_root();
    full.split('(').next().unwrap_or(&full).to_lowercase()
}

// ---------------------------------------------------------------------------
// Convenience: analyze with all defaults (no overrides)
// ---------------------------------------------------------------------------

/// Analyze with default goals for all sinks.
pub fn analyze_coordination_default(ir: &[HydroRoot]) -> CoordinationReport {
    analyze_coordination(ir, &HashMap::new())
}
