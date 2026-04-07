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
///   is a prefix of all future observations. Applies to  streams.
/// - **SetInclusion**: output elements only accumulate. New elements may appear
///   but none are retracted. Applies to  streams.
/// - **Lattice**: output value only grows under a join-semilattice order.
///   Applies to singletons produced by commutative+idempotent aggregation.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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
/// Infer the default proof goal from a collection kind.
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

fn default_goal_for_sink(root: &HydroRoot) -> OrderGoal {
    goal_for_collection_kind(&root.input_metadata().collection_kind)
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

    pub(crate) fn proved(trace: Vec<ProofStep>) -> Self {
        Self { success: true, trace }
    }

    pub(crate) fn broken(trace: Vec<ProofStep>) -> Self {
        Self { success: false, trace }
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

fn node_proc_macro_span(node: &HydroNode) -> Option<proc_macro2::Span> {
    use syn::spanned::Spanned;
    // Try to get the span from the node's expression (acc/f for folds/reduces)
    match node {
        HydroNode::Fold { acc, .. }
        | HydroNode::FoldKeyed { acc, .. }
        | HydroNode::Scan { acc, .. } => Some(acc.0.span()),
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
                                "coordination required: {} (sink `{}`, goal: {})",
                                reason, sink.name, sink.goal
                            ),
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
    }
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
    let pm_span = node_proc_macro_span(node);

    match node {
        HydroNode::Placeholder => ProofResult::proved(vec![]),

        // --- Sources: discharge any goal ---
        HydroNode::Source { .. }
        | HydroNode::SingletonSource { .. }
        | HydroNode::ExternalInput { .. } => {
            ProofResult::discharged(&name, "source — data only arrives", span, pm_span)
        }

        // --- CycleSource: inherit from matching CycleSink ---
        HydroNode::CycleSource { cycle_id, .. } => {
            match cycle_proofs.get(cycle_id) {
                Some(r) => r.clone().prepend_preserved(&name, span, pm_span),
                None => ProofResult::discharged(&name, "cycle source (no matching sink)", span, pm_span),
            }
        }

        // --- Tee: shared node ---
        HydroNode::Tee { inner, .. } => {
            prove_shared(inner, goal, cycle_proofs, seen_tees).prepend_preserved(&name, span, pm_span)
        }
        HydroNode::Partition { inner, .. } => {
            prove_shared(inner, goal, cycle_proofs, seen_tees).prepend_preserved(&name, span, pm_span)
        }

        // --- Structural pass-through ---
        HydroNode::Cast { inner, .. }
        | HydroNode::ObserveNonDet { inner, .. }
        | HydroNode::BeginAtomic { inner, .. }
        | HydroNode::EndAtomic { inner, .. }
        | HydroNode::YieldConcat { inner, .. } => {
            prove(inner, goal, cycle_proofs, seen_tees).prepend_preserved(&name, span, pm_span)
        }

        // --- Element-wise transforms ---
        HydroNode::Map { input, .. }
        | HydroNode::FlatMap { input, .. }
        | HydroNode::Filter { input, .. }
        | HydroNode::FilterMap { input, .. }
        | HydroNode::Inspect { input, .. } => match goal {
            OrderGoal::SetInclusion | OrderGoal::Prefix => {
                prove(input, goal, cycle_proofs, seen_tees).prepend_preserved(&name, span, pm_span)
            }
            OrderGoal::Lattice => ProofResult::fail(&name, "may not preserve lattice order (closure not annotated)", span, pm_span),
        },

        HydroNode::Enumerate { input, .. } => match goal {
            OrderGoal::SetInclusion => prove(input, goal, cycle_proofs, seen_tees).prepend_preserved(&name, span, pm_span),
            _ => ProofResult::fail(&name, "enumerate breaks prefix/lattice order", span, pm_span),
        },

        HydroNode::Unique { input, .. } => match goal {
            OrderGoal::SetInclusion => prove(input, goal, cycle_proofs, seen_tees).prepend_preserved(&name, span, pm_span),
            _ => ProofResult::fail(&name, "unique breaks prefix/lattice order", span, pm_span),
        },

        HydroNode::Network { input, .. } => match goal {
            OrderGoal::SetInclusion | OrderGoal::Lattice => {
                prove(input, goal, cycle_proofs, seen_tees).prepend_preserved(&name, span, pm_span)
            }
            OrderGoal::Prefix => ProofResult::fail(&name, "network may reorder elements", span, pm_span),
        },

        HydroNode::Counter { .. } => ProofResult::discharged(&name, "count only grows", span, pm_span),

        HydroNode::Batch { inner, .. } => match goal {
            OrderGoal::SetInclusion => prove(inner, goal, cycle_proofs, seen_tees).prepend_preserved(&name, span, pm_span),
            _ => ProofResult::fail(&name, "batch windowing breaks prefix/lattice order", span, pm_span),
        },

        // --- Chain ---
        HydroNode::Chain { first, second, .. }
        | HydroNode::ChainFirst { first, second, .. } => match goal {
            OrderGoal::SetInclusion => {
                let a = prove(first, goal, cycle_proofs, seen_tees);
                if !a.is_proved() {
                    return a.prepend_preserved(&format!("{name} (1st branch)"), span.clone(), pm_span.clone());
                }
                let b = prove(second, goal, cycle_proofs, seen_tees);
                if !b.is_proved() {
                    return b.prepend_preserved(&format!("{name} (2nd branch)"), span.clone(), pm_span.clone());
                }
                b.prepend_preserved(&name, span, pm_span)
            }
            _ => ProofResult::fail(&name, "union/chain breaks prefix/lattice order", span, pm_span),
        },

        // --- Join / CrossProduct ---
        HydroNode::Join { left, right, .. }
        | HydroNode::CrossProduct { left, right, .. } => match goal {
            OrderGoal::SetInclusion => {
                let a = prove(left, &OrderGoal::SetInclusion, cycle_proofs, seen_tees);
                if !a.is_proved() { return a.prepend_preserved(&name, span.clone(), pm_span.clone()); }
                prove(right, &OrderGoal::SetInclusion, cycle_proofs, seen_tees).prepend_preserved(&name, span, pm_span)
            }
            _ => ProofResult::fail(&name, "join breaks prefix/lattice order", span, pm_span),
        },

        // --- CrossSingleton ---
        HydroNode::CrossSingleton { left, right, .. } => match goal {
            OrderGoal::SetInclusion => {
                let a = prove(left, &OrderGoal::SetInclusion, cycle_proofs, seen_tees);
                if !a.is_proved() { return a.prepend_preserved(&name, span.clone(), pm_span.clone()); }
                // Bounded singleton is stable — no lattice proof needed
                if right.metadata().collection_kind.is_bounded() {
                    a.prepend_preserved(&name, span, pm_span)
                } else {
                    prove(right, &OrderGoal::Lattice, cycle_proofs, seen_tees)
                        .prepend_goal_changed(&name, &OrderGoal::Lattice, span, pm_span)
                }
            }
            _ => ProofResult::fail(&name, "cross_singleton breaks prefix/lattice order", span, pm_span),
        },

        // --- Difference / AntiJoin ---
        HydroNode::Difference { pos, neg, .. } => match goal {
            OrderGoal::SetInclusion => {
                let p = prove(pos, &OrderGoal::SetInclusion, cycle_proofs, seen_tees);
                if !p.is_proved() { return p.prepend_preserved(&name, span.clone(), pm_span.clone()); }
                if neg.metadata().collection_kind.is_bounded() {
                    ProofResult::discharged(&name, "neg input is bounded", span, pm_span)
                } else {
                    ProofResult::fail(&name, "unbounded neg input can retract output elements", span, pm_span)
                }
            }
            _ => ProofResult::fail(&name, "difference breaks prefix/lattice order", span, pm_span),
        },
        HydroNode::AntiJoin { pos, neg, .. } => match goal {
            OrderGoal::SetInclusion => {
                let p = prove(pos, &OrderGoal::SetInclusion, cycle_proofs, seen_tees);
                if !p.is_proved() { return p.prepend_preserved(&name, span.clone(), pm_span.clone()); }
                if neg.metadata().collection_kind.is_bounded() {
                    ProofResult::discharged(&name, "neg input is bounded", span, pm_span)
                } else {
                    ProofResult::fail(&name, "unbounded neg input can retract output elements", span, pm_span)
                }
            }
            _ => ProofResult::fail(&name, "anti_join breaks prefix/lattice order", span, pm_span),
        },

        // --- Fold / FoldKeyed ---
        HydroNode::Fold { is_commutative, is_idempotent, input, .. } => {
            if *is_commutative && *is_idempotent && *goal != OrderGoal::Prefix {
                ProofResult::discharged(&name, "commutative + idempotent fold (lattice join)", span, pm_span)
            } else if input.metadata().collection_kind.is_bounded() {
                ProofResult::discharged(&name, "fold over bounded input", span, pm_span)
            } else {
                ProofResult::fail(&name, "fold over unbounded input without commutativity+idempotency proof", span, pm_span)
            }
        }
        HydroNode::FoldKeyed { is_commutative, is_idempotent, input, .. } => {
            if *is_commutative && *is_idempotent && *goal != OrderGoal::Prefix {
                ProofResult::discharged(&name, "commutative + idempotent fold (lattice join)", span, pm_span)
            } else if input.metadata().collection_kind.is_bounded() {
                ProofResult::discharged(&name, "fold over bounded input", span, pm_span)
            } else {
                ProofResult::fail(&name, "fold_keyed over unbounded input without commutativity+idempotency proof", span, pm_span)
            }
        }

        // --- Reduce / ReduceKeyed ---
        HydroNode::Reduce { is_commutative, is_idempotent, input, .. } => {
            if *is_commutative && *is_idempotent && *goal != OrderGoal::Prefix {
                ProofResult::discharged(&name, "commutative + idempotent reduce (lattice join)", span, pm_span)
            } else if input.metadata().collection_kind.is_bounded() {
                ProofResult::discharged(&name, "reduce over bounded input", span, pm_span)
            } else {
                ProofResult::fail(&name, "reduce over unbounded input without commutativity proof", span, pm_span)
            }
        }
        HydroNode::ReduceKeyed { is_commutative, is_idempotent, input, .. } => {
            if *is_commutative && *is_idempotent && *goal != OrderGoal::Prefix {
                ProofResult::discharged(&name, "commutative + idempotent reduce (lattice join)", span, pm_span)
            } else if input.metadata().collection_kind.is_bounded() {
                ProofResult::discharged(&name, "reduce over bounded input", span, pm_span)
            } else {
                ProofResult::fail(&name, "reduce_keyed over unbounded input without commutativity proof", span, pm_span)
            }
        }

        HydroNode::ReduceKeyedWatermark { is_commutative, input, .. } => {
            if *is_commutative && *goal != OrderGoal::Prefix {
                ProofResult::discharged(&name, "commutative watermark reduce (lattice join)", span, pm_span)
            } else if input.metadata().collection_kind.is_bounded() {
                ProofResult::discharged(&name, "watermark reduce over bounded input", span, pm_span)
            } else {
                ProofResult::fail(&name, "watermark-based reduce may retract above watermark", span, pm_span)
            }
        }

        // --- Scan ---
        HydroNode::Scan { input, .. } => match &input.metadata().collection_kind {
            super::ir::CollectionKind::Stream { order: StreamOrder::TotalOrder, .. } => match goal {
                OrderGoal::Prefix | OrderGoal::SetInclusion => {
                    ProofResult::discharged(&name, "scan on TotalOrder input (deterministic prefix)", span, pm_span)
                }
                OrderGoal::Lattice => ProofResult::fail(&name, "scan produces a stream, not a lattice singleton", span, pm_span),
            },
            _ => {
                if input.metadata().collection_kind.is_bounded() {
                    ProofResult::discharged(&name, "scan over bounded input", span, pm_span)
                } else {
                    ProofResult::fail(&name, "scan on non-TotalOrder unbounded input is non-deterministic", span, pm_span)
                }
            }
        },

        // --- Sort ---
        HydroNode::Sort { input, .. } => {
            if input.metadata().collection_kind.is_bounded() {
                ProofResult::discharged(&name, "sort over bounded input", span, pm_span)
            } else {
                ProofResult::fail(&name, "sort on unbounded input commits to order that may change", span, pm_span)
            }
        }

        // --- DeferTick ---
        HydroNode::DeferTick { input, .. } => match goal {
            OrderGoal::SetInclusion | OrderGoal::Lattice => {
                prove(input, goal, cycle_proofs, seen_tees).prepend_preserved(&name, span, pm_span)
            }
            OrderGoal::Prefix => ProofResult::fail(&name, "defer_tick breaks prefix order", span, pm_span),
        },

        // --- Futures ---
        HydroNode::ResolveFutures { input, .. }
        | HydroNode::ResolveFuturesBlocking { input, .. }
        | HydroNode::ResolveFuturesOrdered { input, .. } => match goal {
            OrderGoal::SetInclusion => prove(input, goal, cycle_proofs, seen_tees).prepend_preserved(&name, span, pm_span),
            _ => ProofResult::fail(&name, "future resolution may reorder", span, pm_span),
        },
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
/// specific sinks, keyed by their index in the IR root list. Note that
/// indices include non-observable roots (CycleSink, Null); only indices
/// corresponding to observable sinks (ForEach, SendExternal, etc.) are
/// used. Pass an empty map to use all defaults.
///
/// Future: a more robust sink identifier (name or span-based) would be
/// preferable to raw indices.
pub fn analyze_coordination(
    ir: &[HydroRoot],
    goal_overrides: &HashMap<usize, OrderGoal>,
) -> CoordinationReport {
    // Pass 1: analyze CycleSink roots to determine cycle monotonicity.
    // Run twice to handle inter-cycle dependencies (cycle A depends on cycle B).
    let mut cycle_proofs = CycleProofs::new();
    let mut seen_tees = SeenTees::new();
    for _pass in 0..2 {
        for root in ir {
            if let HydroRoot::CycleSink { cycle_id, input, .. } = root {
                let cycle_goal = goal_for_collection_kind(&input.metadata().collection_kind);
                let result = prove(input, &cycle_goal, &cycle_proofs, &mut seen_tees);
                cycle_proofs.insert(*cycle_id, result);
            }
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

        let mut result = result;
        result.trace.reverse();
        sinks.push(SinkResult {
            name: short_name_root(root),
            goal,
            result,
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
        // Only override observable sinks, not CycleSink/Null
        let overrides: HashMap<usize, OrderGoal> = built.ir().iter().enumerate()
            .filter(|(_, root)| is_observable_sink(root))
            .map(|(i, _)| (i, OrderGoal::SetInclusion))
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
        overrides.insert(0, OrderGoal::SetInclusion);
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
}
