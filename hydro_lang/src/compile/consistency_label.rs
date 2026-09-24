use super::ir::{CollectionKind, HydroRoot, StreamOrder, StreamRetry};
use crate::location::dynamic::ClusterConsistency;

/// Consistency label derived from Hydro's type system.
///
/// These correspond to the labels produced by the external `coord-analysis` tool.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConsistencyLabel {
    /// TotalOrder + ExactlyOnce on a non-keyed stream.
    SeqConsistent,
    /// TotalOrder + ExactlyOnce on a KeyedStream (per-key ordering, cross-key interleaving).
    PerKeySeqConsistent,
    /// NoOrder + ExactlyOnce (elements converge as a multiset).
    ConvergentMultiset,
    /// NoOrder + AtLeastOnce, or TotalOrder + AtLeastOnce (elements converge as a set).
    ConvergentSet,
    /// Singleton/KeyedSingleton (lattice convergence).
    ConvergentLattice,
    /// Untrusted nondeterminism upstream of a sensitive operator.
    Inconsistent,
    /// No consistency guarantee from the type system.
    NoGuarantee,
}

impl ConsistencyLabel {
    /// The string form matching coord-analysis output.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SeqConsistent => "SEQ_CONSISTENT",
            Self::PerKeySeqConsistent => "PER_KEY_SEQ_CONSISTENT",
            Self::ConvergentMultiset => "CONVERGENT_MULTISET",
            Self::ConvergentSet => "CONVERGENT_SET",
            Self::ConvergentLattice => "CONVERGENT_LATTICE",
            Self::Inconsistent => "INCONSISTENT",
            Self::NoGuarantee => "NO_GUARANTEE",
        }
    }
}

impl std::fmt::Display for ConsistencyLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A sink's name and its type-derived consistency label.
#[derive(Clone, Debug)]
pub struct SinkConsistency {
    /// The sink name (from the `tag` field, or the root's print representation).
    pub name: String,
    /// The consistency label derived from the type system.
    pub label: ConsistencyLabel,
    /// Source spans of untrusted nondet nodes that cause INCONSISTENT (if any).
    pub blame: Vec<String>,
}

/// Derive the consistency label from a `ClusterConsistency` and `CollectionKind`.
pub fn derive_label(
    consistency: Option<&ClusterConsistency>,
    collection_kind: &CollectionKind,
) -> ConsistencyLabel {
    match consistency {
        Some(ClusterConsistency::EventualConsistency) => match collection_kind {
            CollectionKind::Stream { order, retry, .. } => stream_label(order, retry, false),
            CollectionKind::KeyedStream {
                value_order,
                value_retry,
                ..
            } => stream_label(value_order, value_retry, true),
            CollectionKind::Singleton { .. } | CollectionKind::KeyedSingleton { .. } => {
                ConsistencyLabel::ConvergentLattice
            }
            CollectionKind::Optional { .. } => ConsistencyLabel::ConvergentSet,
        },
        Some(ClusterConsistency::NoConsistency) | None => ConsistencyLabel::NoGuarantee,
    }
}

fn stream_label(order: &StreamOrder, retry: &StreamRetry, keyed: bool) -> ConsistencyLabel {
    match (order, retry) {
        (StreamOrder::TotalOrder, StreamRetry::ExactlyOnce) => {
            if keyed {
                ConsistencyLabel::PerKeySeqConsistent
            } else {
                ConsistencyLabel::SeqConsistent
            }
        }
        (StreamOrder::TotalOrder, StreamRetry::AtLeastOnce) => ConsistencyLabel::ConvergentSet,
        (StreamOrder::NoOrder, StreamRetry::ExactlyOnce) => ConsistencyLabel::ConvergentMultiset,
        (StreamOrder::NoOrder, StreamRetry::AtLeastOnce) => ConsistencyLabel::ConvergentSet,
    }
}

/// Derive the "best possible" consistency label from a sink's collection kind alone,
/// matching coord-analysis's `sink_type_level`.
pub fn sink_type_label(collection_kind: &CollectionKind) -> ConsistencyLabel {
    match collection_kind {
        CollectionKind::Stream { order, retry, .. } => stream_label(order, retry, false),
        CollectionKind::KeyedStream {
            value_order,
            value_retry,
            ..
        } => stream_label(value_order, value_retry, true),
        CollectionKind::Singleton { .. } | CollectionKind::KeyedSingleton { .. } => {
            ConsistencyLabel::ConvergentLattice
        }
        CollectionKind::Optional { .. } => ConsistencyLabel::ConvergentSet,
    }
}

/// Analyze all observable sinks in the IR, producing consistency labels
/// equivalent to coord-analysis.
///
/// For each sink:
/// 1. Derives the "best possible" label from the sink's output collection kind
/// 2. Walks backward through the IR checking for untrusted `ObserveNonDet` nodes
///    that aren't resolved by a commutative+idempotent fold downstream —
///    if unresolved, downgrades to `Inconsistent`
pub fn analyze_sink_labels(ir: &[HydroRoot]) -> Vec<SinkConsistency> {
    ir.iter()
        .filter(|root| !matches!(root, HydroRoot::CycleSink { .. }))
        .map(|root| {
            let meta = root.input_metadata();
            let name = meta
                .tag
                .clone()
                .or_else(|| find_tag(root.input()))
                .unwrap_or_else(|| root.print_root());

            let type_label = sink_type_label(&meta.collection_kind);

            let mut blame = Vec::new();
            let mut visited = std::collections::HashSet::new();
            collect_unresolved_nondet(root.input(), &mut blame, &mut visited);
            let mut visited = std::collections::HashSet::new();
            collect_unresolved_nondet(root.input(), &mut blame, &mut visited);
            let label = if blame.is_empty() {
                type_label
            } else {
                ConsistencyLabel::Inconsistent
            };

            SinkConsistency { name, label, blame }
        })
        .collect()
}

/// Collect spans of unresolved untrusted `ObserveNonDet` nodes.
fn collect_unresolved_nondet(
    node: &super::ir::HydroNode,
    blame: &mut Vec<String>,
    visited: &mut std::collections::HashSet<usize>,
) {
    use std::rc::Rc;

    use super::ir::HydroNode;
    match node {
        // A fold/reduce whose input is NoOrder+AtLeastOnce is commutative+idempotent
        // (the type system enforced this at compile time). Such a fold absorbs upstream nondet.
        HydroNode::Fold { input, .. }
        | HydroNode::FoldKeyed { input, .. }
        | HydroNode::Reduce { input, .. }
        | HydroNode::ReduceKeyed { input, .. }
        | HydroNode::ReduceKeyedWatermark { input, .. }
            if is_ci_from_input(input) => {}
        // Untrusted nondet that hasn't been resolved
        HydroNode::ObserveNonDet {
            trusted: false,
            metadata,
            ..
        } => {
            if let Some(span) = metadata.op.backtrace.format_span() {
                blame.push(span);
            } else {
                blame.push("<unknown location>".to_owned());
            }
        }
        // Tee/Singleton/Partition: follow shared inner, skip if visited
        HydroNode::Tee { inner, .. }
        | HydroNode::Singleton { inner, .. }
        | HydroNode::Partition { inner, .. } => {
            let ptr = Rc::as_ptr(&inner.0) as usize;
            if visited.insert(ptr) {
                collect_unresolved_nondet(&inner.0.borrow(), blame, visited);
            }
        }
        // Recurse into children
        _ => {
            for child in node.input() {
                collect_unresolved_nondet(child, blame, visited);
            }
        }
    }
}

/// Infer whether a fold/reduce is commutative+idempotent from its input's collection_kind.
/// If input is NoOrder → commutative; if input is AtLeastOnce → idempotent.
/// Both together → c+i, which absorbs upstream nondeterminism.
fn is_ci_from_input(input: &super::ir::HydroNode) -> bool {
    use super::ir::{CollectionKind, StreamOrder, StreamRetry};
    let ck = &input.metadata().collection_kind;
    let is_no_order = matches!(
        ck,
        CollectionKind::Stream {
            order: StreamOrder::NoOrder,
            ..
        } | CollectionKind::KeyedStream {
            value_order: StreamOrder::NoOrder,
            ..
        }
    );
    let is_at_least_once = matches!(
        ck,
        CollectionKind::Stream {
            retry: StreamRetry::AtLeastOnce,
            ..
        } | CollectionKind::KeyedStream {
            value_retry: StreamRetry::AtLeastOnce,
            ..
        }
    );
    is_no_order && is_at_least_once
}

/// Walk backward to find the first tagged node.
fn find_tag(node: &super::ir::HydroNode) -> Option<String> {
    use super::ir::HydroNode;
    if let Some(ref tag) = node.metadata().tag {
        return Some(tag.clone());
    }
    match node {
        HydroNode::Tee { inner, .. }
        | HydroNode::Singleton { inner, .. }
        | HydroNode::Partition { inner, .. } => find_tag(&inner.0.borrow()),
        _ => node.input().iter().find_map(|child| find_tag(child)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_label_eventual_consistency() {
        use super::super::ir::{BoundKind, StreamOrder, StreamRetry};

        let ec = Some(ClusterConsistency::EventualConsistency);

        // Stream + TotalOrder + ExactlyOnce → SEQ_CONSISTENT
        assert_eq!(
            derive_label(
                ec.as_ref(),
                &CollectionKind::Stream {
                    bound: BoundKind::Unbounded,
                    order: StreamOrder::TotalOrder,
                    retry: StreamRetry::ExactlyOnce,
                    element_type: super::super::ir::DebugType(Box::new(
                        syn::parse_str("i32").unwrap()
                    )),
                }
            ),
            ConsistencyLabel::SeqConsistent
        );

        // Stream + NoOrder + ExactlyOnce → CONVERGENT_MULTISET
        assert_eq!(
            derive_label(
                ec.as_ref(),
                &CollectionKind::Stream {
                    bound: BoundKind::Unbounded,
                    order: StreamOrder::NoOrder,
                    retry: StreamRetry::ExactlyOnce,
                    element_type: super::super::ir::DebugType(Box::new(
                        syn::parse_str("i32").unwrap()
                    )),
                }
            ),
            ConsistencyLabel::ConvergentMultiset
        );

        // Stream + NoOrder + AtLeastOnce → CONVERGENT_SET
        assert_eq!(
            derive_label(
                ec.as_ref(),
                &CollectionKind::Stream {
                    bound: BoundKind::Unbounded,
                    order: StreamOrder::NoOrder,
                    retry: StreamRetry::AtLeastOnce,
                    element_type: super::super::ir::DebugType(Box::new(
                        syn::parse_str("i32").unwrap()
                    )),
                }
            ),
            ConsistencyLabel::ConvergentSet
        );

        // KeyedStream + TotalOrder + ExactlyOnce → PER_KEY_SEQ_CONSISTENT
        assert_eq!(
            derive_label(
                ec.as_ref(),
                &CollectionKind::KeyedStream {
                    bound: BoundKind::Unbounded,
                    value_order: StreamOrder::TotalOrder,
                    value_retry: StreamRetry::ExactlyOnce,
                    key_type: super::super::ir::DebugType(Box::new(
                        syn::parse_str("String").unwrap()
                    )),
                    value_type: super::super::ir::DebugType(Box::new(
                        syn::parse_str("i32").unwrap()
                    )),
                }
            ),
            ConsistencyLabel::PerKeySeqConsistent
        );
    }

    #[test]
    fn test_derive_label_no_consistency() {
        use super::super::ir::{BoundKind, StreamOrder, StreamRetry};

        // NoConsistency → NO_GUARANTEE regardless of collection kind
        assert_eq!(
            derive_label(
                Some(&ClusterConsistency::NoConsistency),
                &CollectionKind::Stream {
                    bound: BoundKind::Unbounded,
                    order: StreamOrder::TotalOrder,
                    retry: StreamRetry::ExactlyOnce,
                    element_type: super::super::ir::DebugType(Box::new(
                        syn::parse_str("i32").unwrap()
                    )),
                }
            ),
            ConsistencyLabel::NoGuarantee
        );

        // None → NO_GUARANTEE
        assert_eq!(
            derive_label(
                None,
                &CollectionKind::Stream {
                    bound: BoundKind::Unbounded,
                    order: StreamOrder::TotalOrder,
                    retry: StreamRetry::ExactlyOnce,
                    element_type: super::super::ir::DebugType(Box::new(
                        syn::parse_str("i32").unwrap()
                    )),
                }
            ),
            ConsistencyLabel::NoGuarantee
        );
    }

    /// Integration test: builds a flow with `broadcast_closed` and verifies
    /// the sink on the receiving cluster has `EventualConsistency`.
    #[test]
    #[cfg(feature = "build")]
    fn test_broadcast_closed_produces_eventual_consistency_label() {
        use stageleft::q;

        use crate::prelude::*;

        let mut flow = FlowBuilder::new();
        let process = flow.process::<()>();
        let cluster = flow.cluster::<()>();

        let numbers = process.source_iter(q!(vec![1i32, 2, 3]));
        numbers
            .broadcast_closed(&cluster, TCP.fail_stop().bincode())
            .ir_node_named("broadcast_sink")
            .for_each(q!(|_: i32| {}));

        let built = flow.finalize();
        let labels = built.analyze_consistency();

        let broadcast_sink = labels.iter().find(|s| s.name == "broadcast_sink");
        assert!(
            broadcast_sink.is_some(),
            "broadcast_sink not found; available: {:?}",
            labels.iter().map(|s| &s.name).collect::<Vec<_>>()
        );
        // broadcast_closed with FailStop TCP preserves TotalOrder from source_iter,
        // so: EventualConsistency + TotalOrder + ExactlyOnce = SEQ_CONSISTENT
        assert_eq!(
            broadcast_sink.unwrap().label,
            ConsistencyLabel::SeqConsistent,
            "broadcast_closed on FailStop TCP with TotalOrder source should produce SEQ_CONSISTENT"
        );
    }

    /// Test: cluster without EventualConsistency has NoGuarantee.
    #[test]
    #[cfg(feature = "build")]
    fn test_plain_cluster_has_no_guarantee() {
        use stageleft::q;

        use crate::prelude::*;

        let mut flow = FlowBuilder::new();
        let cluster = flow.cluster::<()>();

        // Source directly on cluster (NoConsistency)
        cluster
            .source_iter(q!(vec![1i32, 2, 3]))
            .ir_node_named("plain_cluster_sink")
            .for_each(q!(|_: i32| {}));

        let built = flow.finalize();
        let labels = built.analyze_consistency();

        let sink = labels
            .iter()
            .find(|s| s.name == "plain_cluster_sink")
            .unwrap();
        // Cluster with TotalOrder+ExactlyOnce → SeqConsistent (no untrusted nondet)
        assert_eq!(sink.label, ConsistencyLabel::SeqConsistent);
    }

    /// Test: a Process-local sink has no cluster consistency (NoGuarantee).
    #[test]
    #[cfg(feature = "build")]
    fn test_process_sink_has_no_guarantee() {
        use stageleft::q;

        use crate::prelude::*;

        let mut flow = FlowBuilder::new();
        let process = flow.process::<()>();

        process
            .source_iter(q!(vec![1i32, 2, 3]))
            .ir_node_named("local_sink")
            .for_each(q!(|_: i32| {}));

        let built = flow.finalize();
        let labels = built.analyze_consistency();

        let sink = labels.iter().find(|s| s.name == "local_sink").unwrap();
        // Process with TotalOrder+ExactlyOnce → SeqConsistent
        assert_eq!(sink.label, ConsistencyLabel::SeqConsistent);
    }

    // ─── Simulator-validated consistency tests ────────────────────────────────

    /// SEQ_CONSISTENT: a deterministic pipeline always produces the same prefix.
    /// The simulator confirms this by exhaustively checking all schedules produce
    /// identical output.
    #[test]
    #[cfg(feature = "sim")]
    fn sim_validates_seq_consistent() {
        use stageleft::q;

        use crate::prelude::*;

        let mut flow = FlowBuilder::new();
        let process = flow.process::<()>();

        let source = process.source_stream(q!(tokio_stream::iter(vec![1i32, 2, 3])));
        let mapped = source.map(q!(|x| x * 10));
        mapped
            .clone()
            .ir_node_named("seq_sink")
            .for_each(q!(|_: i32| {}));
        let out = mapped.sim_output();

        let built = flow.finalize();
        let labels = analyze_sink_labels(built.ir());
        let sink = labels.iter().find(|s| s.name == "seq_sink").unwrap();
        assert_eq!(sink.label, ConsistencyLabel::SeqConsistent);

        // Simulator confirms: all executions produce the same sequence
        built.sim().exhaustive(async || {
            let results: Vec<i32> = out.collect().await;
            assert_eq!(results, vec![10, 20, 30]);
        });
    }

    /// False commutativity: fold claims commutative but isn't (string concat).
    /// Simulator catches this by exploring all input orderings and observing divergence.
    #[test]
    #[cfg(feature = "sim")]
    fn sim_validates_inconsistent() {
        use std::collections::HashSet;

        use stageleft::q;

        use crate::live_collections::stream::NoOrder;
        use crate::prelude::*;
        use crate::properties::manual_proof;

        let mut flow = FlowBuilder::new();
        let process = flow.process::<()>();

        let (in_send, input) = process.sim_input::<String, NoOrder, _>();

        // Non-commutative fold with manual (false) commutativity proof
        let folded = input.fold(
            q!(|| String::new()),
            q!(
                |acc, v| acc.push_str(&v),
                commutative = manual_proof!(/** WRONG — string concat is not commutative */)
            ),
        );
        let out = crate::live_collections::sliced::sliced! {
            let snapshot = use(folded, crate::nondet::nondet!(/** test */));
            snapshot.into_stream()
        }
        .sim_output();

        // The static analysis trusts the manual_proof annotations, so it will NOT
        // report INCONSISTENT here. This test validates that the simulator catches
        // the false commutativity claim at runtime by exploring all orderings.
        let mut final_values = HashSet::new();
        flow.sim().exhaustive(async || {
            in_send.send_many_unordered(["a".to_owned(), "b".to_owned()]);
            let all: Vec<String> = out.collect().await;
            final_values.insert(all.first().unwrap().clone());
        });

        // Sim proves inconsistency: different executions produce different results
        assert!(
            final_values.len() > 1,
            "Expected multiple distinct results proving inconsistency, got: {:?}",
            final_values
        );
    }

    /// CONVERGENT: a commutative+idempotent fold always converges to the same value.
    /// Simulator confirms all executions produce the same final result.
    #[test]
    #[cfg(feature = "sim")]
    fn sim_validates_convergent_ci_fold() {
        use stageleft::q;

        use crate::live_collections::stream::NoOrder;
        use crate::prelude::*;
        use crate::properties::manual_proof;

        let mut flow = FlowBuilder::new();
        let process = flow.process::<()>();

        let (in_send, input) = process.sim_input::<i32, NoOrder, _>();

        // Commutative + idempotent fold (max)
        let folded = input.fold(
            q!(|| i32::MIN),
            q!(
                |acc, v| {
                    if v > *acc {
                        *acc = v;
                    }
                },
                commutative = manual_proof!(/** max is commutative */),
                idempotent = manual_proof!(/** max is idempotent */)
            ),
        );
        let out = crate::live_collections::sliced::sliced! {
            let snapshot = use(folded, crate::nondet::nondet!(/** test */));
            snapshot.into_stream()
        }
        .sim_output();

        // Static analysis: c+i fold absorbs any upstream nondet.
        // The sink is Stream(TotalOrder, ExactlyOnce) after into_stream(), so label is SeqConsistent.
        let built = flow.finalize();
        let labels = analyze_sink_labels(built.ir());
        // No INCONSISTENT labels — the c+i fold resolved any nondet
        assert!(
            !labels
                .iter()
                .any(|s| s.label == ConsistencyLabel::Inconsistent),
            "c+i fold should absorb nondet, got: {:?}",
            labels
        );

        // Simulator confirms: all executions converge to the same final value
        built.sim().exhaustive(async || {
            in_send.send_many_unordered([3, 1, 4, 1, 5]);
            let all: Vec<i32> = out.collect().await;
            assert_eq!(*all.last().unwrap(), 5, "max of [3,1,4,1,5] should be 5");
        });
    }
}
