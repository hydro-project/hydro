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

/// Result of the backward proof walk for a single path.
#[derive(Clone, Debug)]
pub enum ProofResult {
    /// Monotonicity proved under the given goal.
    Proved,
    /// Monotonicity broken at the named operator.
    Broken {
        reason: String,
        blame: Vec<String>,
    },
}

impl ProofResult {
    pub fn is_proved(&self) -> bool {
        matches!(self, ProofResult::Proved)
    }
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
            for s in &self.sinks {
                writeln!(f, "  ✓ {} ({})", s.name, s.goal)?;
            }
        } else {
            writeln!(
                f,
                "Coordination Criterion: FAIL — {}/{total} sinks require coordination",
                failing.len()
            )?;
            for s in &failing {
                if let ProofResult::Broken { reason, blame } = &s.result {
                    writeln!(f, "\n  ✗ {} (goal: {}): {}", s.name, s.goal, reason)?;
                    if !blame.is_empty() {
                        writeln!(f, "    {}", blame.join(" ← "))?;
                    }
                }
            }
        }
        writeln!(f)?;
        let passing: Vec<_> = self.sinks.iter().filter(|s| s.result.is_proved()).collect();
        if !passing.is_empty() && !failing.is_empty() {
            writeln!(f, "  Monotone sinks:")?;
            for s in &passing {
                writeln!(f, "    ✓ {} ({})", s.name, s.goal)?;
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

/// Walk backward from `node`, trying to prove `goal`. Returns the proof result.
fn prove(
    node: &HydroNode,
    goal: &OrderGoal,
    cycle_proofs: &CycleProofs,
    seen_tees: &mut SeenTees,
) -> ProofResult {
    match node {
        HydroNode::Placeholder => ProofResult::Proved,

        // --- Sources: discharge any goal ---
        HydroNode::Source { .. }
        | HydroNode::SingletonSource { .. }
        | HydroNode::ExternalInput { .. } => ProofResult::Proved,

        // --- CycleSource: inherit from matching CycleSink ---
        HydroNode::CycleSource { cycle_id, .. } => {
            cycle_proofs.get(cycle_id).cloned().unwrap_or(ProofResult::Proved)
        }

        // --- Tee: shared node ---
        HydroNode::Tee { inner, .. } => prove_shared(inner, goal, cycle_proofs, seen_tees),
        HydroNode::Partition { inner, .. } => prove_shared(inner, goal, cycle_proofs, seen_tees),

        // --- Structural pass-through: preserve any goal ---
        HydroNode::Cast { inner, .. }
        | HydroNode::ObserveNonDet { inner, .. }
        | HydroNode::BeginAtomic { inner, .. }
        | HydroNode::EndAtomic { inner, .. }
        | HydroNode::YieldConcat { inner, .. } => prove(inner, goal, cycle_proofs, seen_tees),

        // --- Element-wise transforms: preserve SetInclusion and Prefix ---
        HydroNode::Map { input, .. }
        | HydroNode::FlatMap { input, .. }
        | HydroNode::Filter { input, .. }
        | HydroNode::FilterMap { input, .. }
        | HydroNode::Inspect { input, .. } => {
            match goal {
                OrderGoal::SetInclusion | OrderGoal::Prefix => {
                    prove(input, goal, cycle_proofs, seen_tees)
                }
                OrderGoal::Lattice => {
                    // Closure may not preserve lattice order — conservatively break.
                    // Future: allow `monotone = manual_proof!(...)` annotation.
                    ProofResult::Broken {
                        reason: format!("{} may not preserve lattice order (closure not annotated)", short_name(node)),
                        blame: vec![short_name(node)],
                    }
                }
            }
        }

        // --- Enumerate: preserves SetInclusion, breaks Prefix ---
        HydroNode::Enumerate { input, .. } => {
            match goal {
                OrderGoal::SetInclusion => prove(input, goal, cycle_proofs, seen_tees),
                OrderGoal::Prefix => ProofResult::Broken {
                    reason: "enumerate assigns indices that depend on arrival order".into(),
                    blame: vec![short_name(node)],
                },
                OrderGoal::Lattice => ProofResult::Broken {
                    reason: "enumerate not applicable to lattice singletons".into(),
                    blame: vec![short_name(node)],
                },
            }
        }

        // --- Unique: preserves SetInclusion, breaks Prefix ---
        HydroNode::Unique { input, .. } => {
            match goal {
                OrderGoal::SetInclusion => prove(input, goal, cycle_proofs, seen_tees),
                _ => ProofResult::Broken {
                    reason: "unique deduplication breaks prefix/lattice order".into(),
                    blame: vec![short_name(node)],
                },
            }
        }

        // --- Network: preserves SetInclusion, breaks Prefix ---
        HydroNode::Network { input, .. } => {
            match goal {
                OrderGoal::SetInclusion => prove(input, goal, cycle_proofs, seen_tees),
                OrderGoal::Prefix => ProofResult::Broken {
                    reason: "network may reorder elements, breaking prefix order".into(),
                    blame: vec![short_name(node)],
                },
                OrderGoal::Lattice => {
                    // Network forwards a singleton value unchanged
                    prove(input, goal, cycle_proofs, seen_tees)
                }
            }
        }

        // --- Counter: always monotone (count only grows) ---
        HydroNode::Counter { .. } => ProofResult::Proved,

        // --- Batch: preserves SetInclusion ---
        HydroNode::Batch { inner, .. } => {
            match goal {
                OrderGoal::SetInclusion => prove(inner, goal, cycle_proofs, seen_tees),
                _ => ProofResult::Broken {
                    reason: "batch windowing breaks prefix/lattice order".into(),
                    blame: vec![short_name(node)],
                },
            }
        }

        // --- Chain: preserves SetInclusion (both inputs), breaks Prefix ---
        HydroNode::Chain { first, second, .. }
        | HydroNode::ChainFirst { first, second, .. } => {
            match goal {
                OrderGoal::SetInclusion => {
                    let a = prove(first, goal, cycle_proofs, seen_tees);
                    if !a.is_proved() { return a; }
                    prove(second, goal, cycle_proofs, seen_tees)
                }
                _ => ProofResult::Broken {
                    reason: "union/chain breaks prefix/lattice order".into(),
                    blame: vec![short_name(node)],
                },
            }
        }

        // --- Join / CrossProduct: preserves SetInclusion (both inputs) ---
        HydroNode::Join { left, right, .. }
        | HydroNode::CrossProduct { left, right, .. } => {
            match goal {
                OrderGoal::SetInclusion => {
                    let a = prove(left, &OrderGoal::SetInclusion, cycle_proofs, seen_tees);
                    if !a.is_proved() { return a; }
                    prove(right, &OrderGoal::SetInclusion, cycle_proofs, seen_tees)
                }
                _ => ProofResult::Broken {
                    reason: format!("{} breaks prefix/lattice order", short_name(node)),
                    blame: vec![short_name(node)],
                },
            }
        }

        // --- CrossSingleton: stream side needs SetInclusion, singleton side needs Lattice ---
        HydroNode::CrossSingleton { left, right, .. } => {
            match goal {
                OrderGoal::SetInclusion => {
                    let a = prove(left, &OrderGoal::SetInclusion, cycle_proofs, seen_tees);
                    if !a.is_proved() { return a; }
                    // The singleton side must be stable (lattice order) for the
                    // cross product's output set to only grow.
                    prove(right, &OrderGoal::Lattice, cycle_proofs, seen_tees)
                }
                _ => ProofResult::Broken {
                    reason: "cross_singleton breaks prefix/lattice order".into(),
                    blame: vec![short_name(node)],
                },
            }
        }

        // --- Difference / AntiJoin: pos preserves SetInclusion, neg breaks it ---
        HydroNode::Difference { pos, neg, .. } => {
            match goal {
                OrderGoal::SetInclusion => {
                    // pos side: adding elements to pos adds to output
                    let p = prove(pos, &OrderGoal::SetInclusion, cycle_proofs, seen_tees);
                    if !p.is_proved() { return p; }
                    // neg side: adding elements to neg REMOVES from output — anti-monotone
                    // Only safe if neg is bounded (complete, won't grow further)
                    if neg.metadata().collection_kind.is_bounded() {
                        ProofResult::Proved
                    } else {
                        ProofResult::Broken {
                            reason: "difference: unbounded neg input can retract output elements".into(),
                            blame: vec!["difference".into()],
                        }
                    }
                }
                _ => ProofResult::Broken {
                    reason: "difference breaks prefix/lattice order".into(),
                    blame: vec![short_name(node)],
                },
            }
        }
        HydroNode::AntiJoin { pos, neg, .. } => {
            match goal {
                OrderGoal::SetInclusion => {
                    let p = prove(pos, &OrderGoal::SetInclusion, cycle_proofs, seen_tees);
                    if !p.is_proved() { return p; }
                    if neg.metadata().collection_kind.is_bounded() {
                        ProofResult::Proved
                    } else {
                        ProofResult::Broken {
                            reason: "anti_join: unbounded neg input can retract output elements".into(),
                            blame: vec!["antijoin".into()],
                        }
                    }
                }
                _ => ProofResult::Broken {
                    reason: "anti_join breaks prefix/lattice order".into(),
                    blame: vec![short_name(node)],
                },
            }
        }

        // --- Fold / FoldKeyed: can DISCHARGE lattice goal if commutative+idempotent ---
        HydroNode::Fold { is_commutative, is_idempotent, input, .. } => {
            if *is_commutative && *is_idempotent {
                // Commutative+idempotent fold is a lattice join.
                // Discharges both Lattice (value grows) and SetInclusion
                // (downstream conversions to stream produce growing sets).
                ProofResult::Proved
            } else if input.metadata().collection_kind.is_bounded() {
                ProofResult::Proved
            } else {
                ProofResult::Broken {
                    reason: "fold over unbounded input without commutativity+idempotency proof".into(),
                    blame: vec![short_name(node)],
                }
            }
        }
        HydroNode::FoldKeyed { is_commutative, is_idempotent, input, .. } => {
            if *is_commutative && *is_idempotent {
                ProofResult::Proved
            } else if input.metadata().collection_kind.is_bounded() {
                ProofResult::Proved
            } else {
                ProofResult::Broken {
                    reason: "fold_keyed over unbounded input without commutativity+idempotency proof".into(),
                    blame: vec![short_name(node)],
                }
            }
        }

        // --- Reduce / ReduceKeyed: similar to Fold ---
        HydroNode::Reduce { is_commutative, input, .. } => {
            match goal {
                OrderGoal::Lattice if *is_commutative => ProofResult::Proved,
                _ => {
                    if input.metadata().collection_kind.is_bounded() {
                        ProofResult::Proved
                    } else {
                        ProofResult::Broken {
                            reason: "reduce over unbounded input without commutativity proof".into(),
                            blame: vec![short_name(node)],
                        }
                    }
                }
            }
        }
        HydroNode::ReduceKeyed { is_commutative, input, .. } => {
            match goal {
                OrderGoal::Lattice if *is_commutative => ProofResult::Proved,
                OrderGoal::SetInclusion if *is_commutative => ProofResult::Proved,
                _ => {
                    if input.metadata().collection_kind.is_bounded() {
                        ProofResult::Proved
                    } else {
                        ProofResult::Broken {
                            reason: "reduce_keyed over unbounded input without commutativity proof".into(),
                            blame: vec![short_name(node)],
                        }
                    }
                }
            }
        }

        HydroNode::ReduceKeyedWatermark { input, .. } => {
            if input.metadata().collection_kind.is_bounded() {
                ProofResult::Proved
            } else {
                ProofResult::Broken {
                    reason: "watermark-based reduce may retract above watermark".into(),
                    blame: vec![short_name(node)],
                }
            }
        }

        // --- Scan: discharges Prefix on TotalOrder input ---
        HydroNode::Scan { input, .. } => {
            match goal {
                OrderGoal::Prefix => {
                    // Scan on TotalOrder input produces a deterministic prefix.
                    // Check if input is TotalOrder.
                    match &input.metadata().collection_kind {
                        super::ir::CollectionKind::Stream { order: StreamOrder::TotalOrder, .. } => {
                            ProofResult::Proved
                        }
                        _ => ProofResult::Broken {
                            reason: "scan on non-TotalOrder input cannot prove prefix order".into(),
                            blame: vec![short_name(node)],
                        },
                    }
                }
                _ => {
                    if input.metadata().collection_kind.is_bounded() {
                        ProofResult::Proved
                    } else {
                        ProofResult::Broken {
                            reason: "scan is stateful; cannot prove set inclusion or lattice order".into(),
                            blame: vec![short_name(node)],
                        }
                    }
                }
            }
        }

        // --- Sort: breaks unless bounded ---
        HydroNode::Sort { input, .. } => {
            if input.metadata().collection_kind.is_bounded() {
                ProofResult::Proved
            } else {
                ProofResult::Broken {
                    reason: "sort on unbounded input commits to order that may change".into(),
                    blame: vec![short_name(node)],
                }
            }
        }

        // --- DeferTick: preserves SetInclusion, breaks Prefix ---
        HydroNode::DeferTick { input, .. } => {
            match goal {
                OrderGoal::SetInclusion => prove(input, goal, cycle_proofs, seen_tees),
                OrderGoal::Lattice => prove(input, goal, cycle_proofs, seen_tees),
                OrderGoal::Prefix => ProofResult::Broken {
                    reason: "defer_tick introduces temporal boundary breaking prefix order".into(),
                    blame: vec![short_name(node)],
                },
            }
        }

        // --- Futures resolution: preserves SetInclusion ---
        HydroNode::ResolveFutures { input, .. }
        | HydroNode::ResolveFuturesBlocking { input, .. }
        | HydroNode::ResolveFuturesOrdered { input, .. } => {
            match goal {
                OrderGoal::SetInclusion => prove(input, goal, cycle_proofs, seen_tees),
                _ => ProofResult::Broken {
                    reason: "future resolution may reorder, breaking prefix/lattice order".into(),
                    blame: vec![short_name(node)],
                },
            }
        }
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
    seen_tees.insert(ptr, ProofResult::Proved);
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
