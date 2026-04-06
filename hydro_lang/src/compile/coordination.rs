//! Static analysis for the Coordination Criterion (Hellerstein 2026).
//!
//! Determines whether each edge in a Hydro IR graph is **future-monotone**: whether
//! observable outcomes on that edge only refine (never contradict) as the execution
//! history grows. This corresponds to Gyatso's "monotone outputs" property from
//! Laddad's Flo/Gyatso semantics — a collection type whose concatenation operator
//! induces a natural partial order (sets under ⊆, lattices under ⊑, sequences under
//! prefix) is future-monotone, while operators that can retract previously observable
//! outputs (like set difference or unbounded fold) break this property.
//!
//! A program whose observable sinks are all future-monotone admits a coordination-free
//! implementation. Where future-monotonicity fails, coordination is intrinsically
//! required.
//!
//! # Limitations
//!
//! This analysis uses structural reasoning over the IR graph rather than inspecting
//! Rust trait implementations. Ideally, the natural partial order on collection types
//! (Definition 3.4.1 in Laddad's dissertation) would be reified as a trait (e.g.,
//! `LatticeOrd` or a new `NaturalOrder` trait) and the IR would carry that information
//! so the analysis could check "does this edge's type have a natural order?" rather
//! than hard-coding which operators produce ordered outputs. For now, the analysis is
//! conservative: it knows that bounded aggregations produce future-monotone output and
//! that `Difference`/`AntiJoin` do not, but it cannot distinguish a lattice-merge fold
//! from a general fold without boundedness information.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;

/// Format the first user-code span from a backtrace, e.g. "src/main.rs:42:5".
/// Returns None if no span info is available.
#[cfg(feature = "build")]
fn format_span(bt: &Backtrace) -> Option<String> {
    bt.elements().find_map(|elem| {
        let file = elem.filename.as_ref()?;
        // Skip internal frames (hydro_lang, stageleft, etc.)
        if file.contains("hydro_lang/src/") || file.contains("stageleft") || file.contains("hydro_std/src/") {
            return None;
        }
        let line = elem.lineno?;
        let col = elem.colno.unwrap_or(0);
        Some(format!("{file}:{line}:{col}"))
    })
}

#[cfg(not(feature = "build"))]
fn format_span(_bt: &Backtrace) -> Option<String> {
    None
}

use super::ir::backtrace::Backtrace;
use super::ir::{HydroNode, HydroRoot, SharedNode};
use crate::location::dynamic::LocationId;

/// Whether an edge in the dataflow graph is future-monotone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FutureMonotonicity {
    /// Output only refines under extension — coordination-free.
    Monotone,
    /// Output may contradict earlier observations — coordination required.
    NonMonotone {
        /// Why this edge is non-monotone.
        reason: &'static str,
        /// Short name of the operator that first broke monotonicity.
        origin: String,
    },
}

impl FutureMonotonicity {
    /// Returns `true` if this edge is future-monotone.
    pub fn is_monotone(&self) -> bool {
        matches!(self, FutureMonotonicity::Monotone)
    }

    pub fn combine(a: &FutureMonotonicity, b: &FutureMonotonicity) -> FutureMonotonicity {
        match (a, b) {
            (FutureMonotonicity::Monotone, FutureMonotonicity::Monotone) => {
                FutureMonotonicity::Monotone
            }
            (FutureMonotonicity::NonMonotone { reason, origin }, _) => {
                FutureMonotonicity::NonMonotone {
                    reason,
                    origin: origin.clone(),
                }
            }
            (_, FutureMonotonicity::NonMonotone { reason, origin }) => {
                FutureMonotonicity::NonMonotone {
                    reason,
                    origin: origin.clone(),
                }
            }
        }
    }
}

/// Extract a short operator name from `print_root()` output (e.g. "DeferTick()" → "defer_tick").
fn short_name(node: &HydroNode) -> String {
    let full = node.print_root();
    full.split('(')
        .next()
        .unwrap_or("unknown")
        .to_lowercase()
}

/// Annotation for a single edge (node output) in the IR graph.
pub struct EdgeAnnotation {
    /// Name of the operator producing this edge.
    pub operator: String,
    /// Whether this edge is future-monotone.
    pub monotonicity: FutureMonotonicity,
    /// Location where this operator executes.
    pub location: LocationId,
    /// Source location of the operator in user code.
    pub backtrace: Backtrace,
    /// Whether this is a sink (observable output) of the program.
    pub is_sink: bool,
    /// Path from this edge back to the operator that introduced non-monotonicity.
    /// Empty for monotone edges. First element is this node, last is the origin.
    pub blame_chain: Vec<String>,
}

/// Result of the Coordination Criterion analysis.
pub struct CoordinationReport {
    /// Monotonicity annotation for every edge in the graph.
    pub edges: Vec<EdgeAnnotation>,
}

impl CoordinationReport {
    /// Returns `true` if all observable sinks are future-monotone.
    pub fn is_coordination_free(&self) -> bool {
        self.edges
            .iter()
            .filter(|e| e.is_sink)
            .all(|e| e.monotonicity.is_monotone())
    }

    /// Iterator over sink (observable output) annotations.
    pub fn sinks(&self) -> impl Iterator<Item = &EdgeAnnotation> {
        self.edges.iter().filter(|e| e.is_sink)
    }

    /// Iterator over all non-monotone edges.
    pub fn non_monotone_edges(&self) -> impl Iterator<Item = &EdgeAnnotation> {
        self.edges
            .iter()
            .filter(|e| !e.monotonicity.is_monotone())
    }
}

impl fmt::Display for CoordinationReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let total = self.edges.len();
        let num_sinks = self.edges.iter().filter(|e| e.is_sink).count();
        let non_mono: Vec<_> = self.non_monotone_edges().collect();
        let sink_violations: Vec<_> = self
            .edges
            .iter()
            .filter(|e| e.is_sink && !e.monotonicity.is_monotone())
            .collect();

        if sink_violations.is_empty() {
            writeln!(
                f,
                "Coordination Criterion: PASS \u{2014} all {num_sinks} sinks are future-monotone ({total} edges analyzed)"
            )?;
        } else {
            writeln!(
                f,
                "Coordination Criterion: FAIL \u{2014} {}/{num_sinks} sinks require coordination",
                sink_violations.len(),
            )?;
            writeln!(f)?;
            for v in &sink_violations {
                if let FutureMonotonicity::NonMonotone { reason, .. } = &v.monotonicity {
                    let sink_short = v
                        .operator
                        .split('(')
                        .next()
                        .unwrap_or(&v.operator)
                        .to_lowercase();
                    let span = format_span(&v.backtrace)
                        .map(|s| format!(" at {s}"))
                        .unwrap_or_default();
                    writeln!(f, "  \u{2717} {sink_short}{span}: {reason}")?;
                    if !v.blame_chain.is_empty() {
                        let chain = v.blame_chain.join(" \u{2190} ");
                        writeln!(f, "    {chain}")?;
                    }
                }
            }
            for v in self.edges.iter().filter(|e| e.is_sink && e.monotonicity.is_monotone()) {
                let sink_short = v
                    .operator
                    .split('(')
                    .next()
                    .unwrap_or(&v.operator)
                    .to_lowercase();
                writeln!(f, "  \u{2713} {sink_short}")?;
            }
        }

        if !non_mono.is_empty() {
            writeln!(f)?;
            // Summarize origins
            let mut origins: Vec<&str> = non_mono
                .iter()
                .filter_map(|e| match &e.monotonicity {
                    FutureMonotonicity::NonMonotone { origin, .. } => Some(origin.as_str()),
                    _ => None,
                })
                .collect();
            origins.sort();
            origins.dedup();
            let origin_summary = if origins.len() == 1 {
                format!("all originating from {}", origins[0])
            } else {
                format!("originating from: {}", origins.join(", "))
            };
            writeln!(
                f,
                "  {}/{total} edges non-monotone ({origin_summary})",
                non_mono.len()
            )?;
        }

        Ok(())
    }
}

type SeenTees = HashMap<*const RefCell<HydroNode>, (FutureMonotonicity, Vec<String>)>;

/// Analyze a Hydro IR program for the Coordination Criterion.
pub fn analyze_coordination(ir: &[HydroRoot]) -> CoordinationReport {
    let mut edges = Vec::new();
    let mut seen_tees = SeenTees::new();

    for root in ir {
        let (input_mono, input_chain) = analyze_node(root.input(), &mut edges, &mut seen_tees);

        let meta = root.input_metadata();
        let sink_name = root
            .print_root()
            .split('(')
            .next()
            .unwrap_or("sink")
            .to_lowercase();
        let blame_chain = if input_mono.is_monotone() {
            vec![]
        } else {
            let mut chain = vec![sink_name];
            chain.extend(input_chain);
            chain
        };
        edges.push(EdgeAnnotation {
            operator: root.print_root(),
            monotonicity: input_mono,
            location: meta.location_id.clone(),
            backtrace: root.op_metadata().backtrace.clone(),
            is_sink: is_observable_sink(root),
            blame_chain,
        });
    }

    CoordinationReport { edges }
}

/// Determine whether a root is an externally observable sink.
///
/// Internal plumbing (cycle feedback, null discards, paired external ports)
/// is not observable — non-monotonicity there is expected and handled by
/// the protocol. Only truly external outputs count for the Coordination
/// Criterion verdict.
fn is_observable_sink(root: &HydroRoot) -> bool {
    match root {
        // Side effects are observable.
        HydroRoot::ForEach { .. } => true,
        // Embedded outputs are observable.
        HydroRoot::EmbeddedOutput { .. } => true,
        // Arbitrary sinks are observable.
        HydroRoot::DestSink { .. } => true,
        // Cycle feedback is internal plumbing.
        HydroRoot::CycleSink { .. } => false,
        // Null is explicitly discarded.
        HydroRoot::Null { .. } => false,
        // External sends: only observable if unpaired (one-way output).
        // Paired sends have a matching ExternalInput and are just internal edges.
        HydroRoot::SendExternal { unpaired, .. } => *unpaired,
    }
}

/// Returns (monotonicity, blame_chain). Blame chain is empty if monotone.
fn analyze_node(
    node: &HydroNode,
    edges: &mut Vec<EdgeAnnotation>,
    seen_tees: &mut SeenTees,
) -> (FutureMonotonicity, Vec<String>) {
    let (mono, meta, chain) = classify_node(node, edges, seen_tees);

    if let Some(meta) = meta {
        edges.push(EdgeAnnotation {
            operator: node.print_root(),
            monotonicity: mono.clone(),
            location: meta.location_id.clone(),
            backtrace: meta.op.backtrace.clone(),
            is_sink: false,
            blame_chain: chain.clone(),
        });
    }

    (mono, chain)
}

/// Prepend this node's short name to an input blame chain.
fn prepend_chain(node: &HydroNode, input_chain: Vec<String>) -> Vec<String> {
    let mut chain = vec![short_name(node)];
    chain.extend(input_chain);
    chain
}

/// Combine two blame chains, preferring the first non-empty one.
fn combine_chains(a: Vec<String>, b: Vec<String>) -> Vec<String> {
    if !a.is_empty() { a } else { b }
}

/// Returns (monotonicity, metadata_if_not_placeholder, blame_chain).
fn classify_node<'a>(
    node: &'a HydroNode,
    edges: &mut Vec<EdgeAnnotation>,
    seen_tees: &mut SeenTees,
) -> (
    FutureMonotonicity,
    Option<&'a super::ir::HydroIrMetadata>,
    Vec<String>,
) {
    match node {
        HydroNode::Placeholder => (FutureMonotonicity::Monotone, None, vec![]),

        // --- Sources: always monotone (data only arrives) ---
        HydroNode::Source { metadata, .. }
        | HydroNode::SingletonSource { metadata, .. }
        | HydroNode::CycleSource { metadata, .. }
        | HydroNode::ExternalInput { metadata, .. } => {
            (FutureMonotonicity::Monotone, Some(metadata), vec![])
        }

        // --- Tee / Partition: shared nodes ---
        HydroNode::Tee { inner, metadata } => {
            let (mono, chain) = analyze_shared(inner, edges, seen_tees);
            let chain = if mono.is_monotone() { vec![] } else { prepend_chain(node, chain) };
            (mono, Some(metadata), chain)
        }
        HydroNode::Partition { inner, metadata, .. } => {
            let (mono, chain) = analyze_shared(inner, edges, seen_tees);
            let chain = if mono.is_monotone() { vec![] } else { prepend_chain(node, chain) };
            (mono, Some(metadata), chain)
        }

        // --- Element-wise transforms: preserve input monotonicity ---
        HydroNode::Map { input, metadata, .. }
        | HydroNode::FlatMap { input, metadata, .. }
        | HydroNode::Filter { input, metadata, .. }
        | HydroNode::FilterMap { input, metadata, .. }
        | HydroNode::Inspect { input, metadata, .. }
        | HydroNode::Enumerate { input, metadata, .. }
        | HydroNode::Unique { input, metadata, .. }
        | HydroNode::Network { input, metadata, .. }
        | HydroNode::Counter { input, metadata, .. }
        | HydroNode::ResolveFutures { input, metadata, .. }
        | HydroNode::ResolveFuturesBlocking { input, metadata, .. }
        | HydroNode::ResolveFuturesOrdered { input, metadata, .. } => {
            let (mono, chain) = analyze_node(input, edges, seen_tees);
            let chain = if mono.is_monotone() { vec![] } else { prepend_chain(node, chain) };
            (mono, Some(metadata), chain)
        }

        // --- Structural pass-through: preserve input monotonicity ---
        HydroNode::Cast { inner, metadata }
        | HydroNode::ObserveNonDet { inner, metadata, .. }
        | HydroNode::BeginAtomic { inner, metadata }
        | HydroNode::EndAtomic { inner, metadata }
        | HydroNode::YieldConcat { inner, metadata } => {
            let (mono, chain) = analyze_node(inner, edges, seen_tees);
            let chain = if mono.is_monotone() { vec![] } else { prepend_chain(node, chain) };
            (mono, Some(metadata), chain)
        }

        // --- Batch: windowing creates bounded inner streams, preserves monotonicity ---
        HydroNode::Batch { inner, metadata } => {
            let (mono, chain) = analyze_node(inner, edges, seen_tees);
            let chain = if mono.is_monotone() { vec![] } else { prepend_chain(node, chain) };
            (mono, Some(metadata), chain)
        }

        // --- Chain/ChainFirst: union of two monotone streams is monotone ---
        HydroNode::Chain {
            first,
            second,
            metadata,
        }
        | HydroNode::ChainFirst {
            first,
            second,
            metadata,
        } => {
            let (a, ca) = analyze_node(first, edges, seen_tees);
            let (b, cb) = analyze_node(second, edges, seen_tees);
            let combined = FutureMonotonicity::combine(&a, &b);
            let chain = if combined.is_monotone() {
                vec![]
            } else {
                prepend_chain(node, combine_chains(ca, cb))
            };
            (combined, Some(metadata), chain)
        }

        // --- Joins: joining two growing collections yields a growing collection ---
        HydroNode::Join {
            left,
            right,
            metadata,
        }
        | HydroNode::CrossProduct {
            left,
            right,
            metadata,
        }
        | HydroNode::CrossSingleton {
            left,
            right,
            metadata,
        } => {
            let (a, ca) = analyze_node(left, edges, seen_tees);
            let (b, cb) = analyze_node(right, edges, seen_tees);
            let combined = FutureMonotonicity::combine(&a, &b);
            let chain = if combined.is_monotone() {
                vec![]
            } else {
                prepend_chain(node, combine_chains(ca, cb))
            };
            (combined, Some(metadata), chain)
        }

        // --- Difference / AntiJoin: always non-monotone ---
        HydroNode::Difference { pos, neg, metadata } => {
            let _ = analyze_node(pos, edges, seen_tees);
            let _ = analyze_node(neg, edges, seen_tees);
            let name = short_name(node);
            (
                FutureMonotonicity::NonMonotone {
                    reason: "set difference can retract elements when the negative input grows",
                    origin: name.clone(),
                },
                Some(metadata),
                vec![name],
            )
        }
        HydroNode::AntiJoin { pos, neg, metadata } => {
            let _ = analyze_node(pos, edges, seen_tees);
            let _ = analyze_node(neg, edges, seen_tees);
            let name = short_name(node);
            (
                FutureMonotonicity::NonMonotone {
                    reason: "anti-join can retract elements when the negative input grows",
                    origin: name.clone(),
                },
                Some(metadata),
                vec![name],
            )
        }

        // --- Aggregations: monotone only if input is bounded ---
        HydroNode::Fold {
            input, metadata, ..
        }
        | HydroNode::FoldKeyed {
            input, metadata, ..
        }
        | HydroNode::Reduce {
            input, metadata, ..
        }
        | HydroNode::ReduceKeyed {
            input, metadata, ..
        } => {
            let (input_mono, input_chain) = analyze_node(input, edges, seen_tees);
            if input.metadata().collection_kind.is_bounded() {
                (FutureMonotonicity::Monotone, Some(metadata), vec![])
            } else {
                let name = short_name(node);
                let self_mono = FutureMonotonicity::NonMonotone {
                    reason: "aggregation over unbounded input may produce intermediate results contradicted by later input",
                    origin: name.clone(),
                };
                let combined = FutureMonotonicity::combine(&self_mono, &input_mono);
                let chain = prepend_chain(node, if input_mono.is_monotone() { vec![name] } else { input_chain });
                (combined, Some(metadata), chain)
            }
        }

        HydroNode::ReduceKeyedWatermark {
            input,
            watermark,
            metadata,
            ..
        } => {
            let (input_mono, input_chain) = analyze_node(input, edges, seen_tees);
            let _ = analyze_node(watermark, edges, seen_tees);
            if input.metadata().collection_kind.is_bounded() {
                (FutureMonotonicity::Monotone, Some(metadata), vec![])
            } else {
                let name = short_name(node);
                let self_mono = FutureMonotonicity::NonMonotone {
                    reason: "watermark-based aggregation over unbounded input may retract",
                    origin: name.clone(),
                };
                let combined = FutureMonotonicity::combine(&self_mono, &input_mono);
                let chain = prepend_chain(node, if input_mono.is_monotone() { vec![name] } else { input_chain });
                (combined, Some(metadata), chain)
            }
        }

        // --- Sort: commits to a total order, monotone only with bounded input ---
        HydroNode::Sort { input, metadata } => {
            let (input_mono, input_chain) = analyze_node(input, edges, seen_tees);
            if input.metadata().collection_kind.is_bounded() {
                (FutureMonotonicity::Monotone, Some(metadata), vec![])
            } else {
                let name = short_name(node);
                let self_mono = FutureMonotonicity::NonMonotone {
                    reason: "sort commits to a total order that may be invalidated by later arrivals",
                    origin: name.clone(),
                };
                let combined = FutureMonotonicity::combine(&self_mono, &input_mono);
                let chain = prepend_chain(node, if input_mono.is_monotone() { vec![name] } else { input_chain });
                (combined, Some(metadata), chain)
            }
        }

        // --- Scan: stateful transform that can contradict earlier outputs ---
        HydroNode::Scan {
            input, metadata, ..
        } => {
            let (input_mono, input_chain) = analyze_node(input, edges, seen_tees);
            let name = short_name(node);
            let self_mono = FutureMonotonicity::NonMonotone {
                reason: "stateful scan can produce outputs that contradict earlier outputs under extension",
                origin: name.clone(),
            };
            let combined = FutureMonotonicity::combine(&self_mono, &input_mono);
            let chain = prepend_chain(node, if input_mono.is_monotone() { vec![name] } else { input_chain });
            (combined, Some(metadata), chain)
        }

        // --- DeferTick: temporal boundary that breaks monotonicity ---
        HydroNode::DeferTick { input, metadata } => {
            let (input_mono, input_chain) = analyze_node(input, edges, seen_tees);
            let name = short_name(node);
            let self_mono = FutureMonotonicity::NonMonotone {
                reason: "defer_tick creates a temporal boundary that can break monotonicity of the enclosing stream",
                origin: name.clone(),
            };
            let combined = FutureMonotonicity::combine(&self_mono, &input_mono);
            let chain = prepend_chain(node, if input_mono.is_monotone() { vec![name] } else { input_chain });
            (combined, Some(metadata), chain)
        }
    }
}

fn analyze_shared(
    inner: &SharedNode,
    edges: &mut Vec<EdgeAnnotation>,
    seen_tees: &mut SeenTees,
) -> (FutureMonotonicity, Vec<String>) {
    let ptr = inner.as_ptr();
    if let Some((mono, chain)) = seen_tees.get(&ptr) {
        return (mono.clone(), chain.clone());
    }
    // Insert a placeholder to handle cycles.
    seen_tees.insert(ptr, (FutureMonotonicity::Monotone, vec![]));
    let result = analyze_node(&inner.0.borrow(), edges, seen_tees);
    seen_tees.insert(ptr, result.clone());
    result
}
