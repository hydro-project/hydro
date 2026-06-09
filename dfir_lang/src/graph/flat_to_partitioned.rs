//! Subgraph partioning algorithm

use std::collections::BTreeSet;

use itertools::Itertools;
use slotmap::{SecondaryMap, SparseSecondaryMap};

use super::meta_graph::DfirGraph;
use super::ops::{DelayType, FloType};
use super::{Color, GraphEdgeId, GraphNode, GraphNodeId, HandoffKind};
use crate::diagnostic::{Diagnostic, Level};
use crate::graph::graph_algorithms::SubgraphMerge;

/// Helper struct for tracking barrier crossers, see [`find_barrier_crossers`].
struct BarrierCrossers {
    /// Edge barrier crossers, including what type.
    pub edge_barrier_crossers: SecondaryMap<GraphEdgeId, DelayType>,
    /// Singleton reference barrier crossers, considered to be [`DelayType::Stratum`].
    pub singleton_barrier_crossers: Vec<(GraphNodeId, GraphNodeId)>,
}
impl BarrierCrossers {
    /// Iterate pairs of nodes that are across a barrier. Excludes `DelayType::NextIteration` pairs.
    fn iter_node_pairs<'a>(
        &'a self,
        partitioned_graph: &'a DfirGraph,
    ) -> impl 'a + Iterator<Item = ((GraphNodeId, GraphNodeId), DelayType)> {
        let edge_pairs_iter = self
            .edge_barrier_crossers
            .iter()
            .map(|(edge_id, &delay_type)| {
                let src_dst = partitioned_graph.edge(edge_id);
                (src_dst, delay_type)
            });
        let singleton_pairs_iter = self
            .singleton_barrier_crossers
            .iter()
            .map(|&src_dst| (src_dst, DelayType::Stratum));
        edge_pairs_iter.chain(singleton_pairs_iter)
    }

    /// Insert/replace edge.
    fn replace_edge(&mut self, old_edge_id: GraphEdgeId, new_edge_id: GraphEdgeId) {
        if let Some(delay_type) = self.edge_barrier_crossers.remove(old_edge_id) {
            self.edge_barrier_crossers.insert(new_edge_id, delay_type);
        }
    }
}

/// Find all the barrier crossers.
fn find_barrier_crossers(partitioned_graph: &DfirGraph) -> BarrierCrossers {
    let edge_barrier_crossers = partitioned_graph
        .edges()
        .filter(|&(_, (_src, dst))| {
            // Ignore barriers within `loop {` blocks.
            partitioned_graph.node_loop(dst).is_none()
        })
        .filter_map(|(edge_id, (_src, dst))| {
            let (_src_port, dst_port) = partitioned_graph.edge_ports(edge_id);
            let op_constraints = partitioned_graph.node_op_inst(dst)?.op_constraints;
            let input_barrier = (op_constraints.input_delaytype_fn)(dst_port)?;
            Some((edge_id, input_barrier))
        })
        .collect();

    // Basic singleton barriers: producer → consumer.
    let mut singleton_barrier_crossers: Vec<(GraphNodeId, GraphNodeId)> = partitioned_graph
        .node_ids()
        .flat_map(|dst| {
            partitioned_graph
                .node_singleton_references(dst)
                .iter()
                .filter_map(|r| r.node_id)
                .map(move |src_ref| (src_ref, dst))
        })
        .collect();

    // Access group ordering barriers: for the same singleton target, operators in
    // lower access groups must run before operators in higher access groups.
    // Also: ungrouped mutable refs must run after ungrouped shared refs.
    let refs_by_target = partitioned_graph.node_singleton_reference_groups();
    // For each singleton target...
    for (_singleton, groups) in refs_by_target {
        // For sequential access groups...
        for (group_a, group_b) in groups.values().tuple_windows() {
            // Add ordering barriers so every node in the lower group must run before every node in the higher group.
            for &(node_a, _, _) in group_a {
                for &(node_b, _, _) in group_b {
                    // TODO(mingwei): handle with diagnostics.
                    assert_ne!(
                        node_a, node_b,
                        "encounted conflicted or cyclical singleton references\n{:?}\n{:?}",
                        group_a, group_b,
                    );
                    singleton_barrier_crossers.push((node_a, node_b));
                }
            }
        }
    }

    BarrierCrossers {
        edge_barrier_crossers,
        singleton_barrier_crossers,
    }
}

fn find_subgraph_unionfind(
    partitioned_graph: &DfirGraph,
    barrier_crossers: &BarrierCrossers,
) -> Result<(SubgraphMerge<GraphNodeId>, BTreeSet<GraphEdgeId>), Diagnostic> {
    // Modality (color) of nodes, push or pull.
    // TODO(mingwei)? This does NOT consider `DelayType` barriers (which generally imply `Pull`),
    // which makes it inconsistant with the final output in `as_code()`. But this doesn't create
    // any bugs since we exclude `DelayType` edges from joining subgraphs anyway.
    let mut node_color = partitioned_graph
        .node_ids()
        .filter_map(|node_id| {
            let op_color = partitioned_graph.node_color(node_id)?;
            Some((node_id, op_color))
        })
        .collect::<SparseSecondaryMap<_, _>>();

    // Pre-compute extra ordering edges: if node A references handoff H via singleton ref,
    // and H has a pipe successor C, then C depends on A (borrower runs before consumer).
    let mut extra_preds: SecondaryMap<GraphNodeId, Vec<GraphNodeId>> = SecondaryMap::new();
    for node_id in partitioned_graph.node_ids() {
        for singleton_ref in partitioned_graph.node_singleton_references(node_id).iter() {
            if let Some(ref_target) = singleton_ref.node_id
                && let GraphNode::Handoff { .. } = partitioned_graph.node(ref_target)
            {
                // ref_target is a handoff; find its pipe consumer(s).
                for (_edge, consumer) in partitioned_graph.node_successors(ref_target) {
                    // consumer depends on node_id (the borrower).
                    extra_preds
                        .entry(consumer)
                        .unwrap()
                        .or_default()
                        .push(node_id);
                }
            }
        }
    }

    let mut subgraph_unionfind =
        SubgraphMerge::<GraphNodeId>::new(partitioned_graph.node_ids(), |node_id| {
            partitioned_graph
                .node_predecessors(node_id)
                .filter_map(|(succ_edge, pred_id)| {
                    let succ_edge_delaytype = barrier_crossers
                        .edge_barrier_crossers
                        .get(succ_edge)
                        .copied();
                    // Tick edges are excluded from the topo sort — they are cross-tick by design.
                    if let Some(_delay_type @ (DelayType::Tick | DelayType::TickLazy)) =
                        succ_edge_delaytype
                    {
                        None
                    } else {
                        Some(pred_id)
                    }
                })
                .chain(
                    partitioned_graph
                        .node_singleton_references(node_id)
                        .iter()
                        .filter_map(|r| r.node_id),
                )
                .chain(
                    extra_preds
                        .get(node_id)
                        .map(|v| v.as_slice())
                        .unwrap_or(&[])
                        .iter()
                        .copied(),
                )
        })
        .map_err(|cycle| {
            let span = cycle
                .first()
                .map(|&node_id| partitioned_graph.node(node_id).span())
                .unwrap_or_else(proc_macro2::Span::call_site);
            Diagnostic::spanned(span, Level::Error, format!("Not a DAG: {:?}", cycle))
        })?;

    // Will contain all edges which need handoffs added. Starts out with all edges and
    // we remove from this set as we combine nodes into subgraphs.
    let mut handoff_edges: BTreeSet<GraphEdgeId> = partitioned_graph.edge_ids().collect();
    // Would sort edges here for priority (for now, no sort/priority).

    // Each edge gets looked at in order. However we may not know if a linear
    // chain of operators is PUSH vs PULL until we look at the ends. A fancier
    // algorithm would know to handle linear chains from the outside inward.
    // But instead we just run through the edges in a loop until no more
    // progress is made. Could have some sort of O(N^2) pathological worst
    // case.
    let mut progress = true;
    while progress {
        progress = false;
        // TODO(mingwei): Could this iterate `handoff_edges` instead? (Modulo ownership). Then no case (1) below.
        for (edge_id, (src, dst)) in partitioned_graph.edges().collect::<Vec<_>>() {
            // Ignore existing handoffs, remove `edge_id` from `handoff_edges`.
            if matches!(partitioned_graph.node(src), GraphNode::Handoff { .. })
                || matches!(partitioned_graph.node(dst), GraphNode::Handoff { .. })
            {
                handoff_edges.remove(&edge_id);
                continue;
            }

            // Ignore (1) already added edges as well as (2) new self-cycles. (Unless reference edge).
            if subgraph_unionfind.same_set(src, dst) {
                // Note that the _edge_ `edge_id` might not be in the subgraph even when both `src` and `dst` are. This prevents case 2.
                // Handoffs will be inserted later for this self-loop.
                continue;
            }

            // Do not connect stratum crossers (next edges).
            if barrier_crossers
                .iter_node_pairs(partitioned_graph)
                .any(|((x_src, x_dst), _)| {
                    (subgraph_unionfind.same_set(x_src, src)
                        && subgraph_unionfind.same_set(x_dst, dst))
                        || (subgraph_unionfind.same_set(x_src, dst)
                            && subgraph_unionfind.same_set(x_dst, src))
                })
            {
                continue;
            }

            // Do not connect across loop contexts.
            if partitioned_graph.node_loop(src) != partitioned_graph.node_loop(dst) {
                continue;
            }
            // Do not connect `next_iteration()`.
            if partitioned_graph.node_op_inst(dst).is_some_and(|op_inst| {
                Some(FloType::NextIteration) == op_inst.op_constraints.flo_type
            }) {
                continue;
            }

            if can_connect_colorize(&mut node_color, src, dst) {
                // At this point we have selected this edge and its src & dst to be
                // within a single subgraph.
                let ok = subgraph_unionfind.try_merge(src, dst);
                if ok {
                    assert!(handoff_edges.remove(&edge_id));
                    progress = true;
                }
            }
        }
    }

    Ok((subgraph_unionfind, handoff_edges))
}

/// Find subgraph and insert handoffs.
/// Modifies barrier_crossers so that the edge OUT of an inserted handoff has
/// the DelayType data.
fn make_subgraphs(
    partitioned_graph: &mut DfirGraph,
    barrier_crossers: &mut BarrierCrossers,
) -> Result<(), Diagnostic> {
    // Algorithm:
    // 1. Each node begins as its own subgraph.
    // 2. Collect edges. (Future optimization: sort so edges which should not be split across a handoff come first).
    // 3. For each edge, try to join `(to, from)` into the same subgraph.

    // TODO(mingwei):
    // self.partitioned_graph.assert_valid();

    let (subgraph_merge, handoff_edges) =
        find_subgraph_unionfind(partitioned_graph, barrier_crossers)?;

    // Insert handoffs between subgraphs (or on subgraph self-loop edges)
    for edge_id in handoff_edges {
        let (src_id, dst_id) = partitioned_graph.edge(edge_id);

        // Already has a handoff, no need to insert one.
        let src_node = partitioned_graph.node(src_id);
        let dst_node = partitioned_graph.node(dst_id);
        if matches!(src_node, GraphNode::Handoff { .. })
            || matches!(dst_node, GraphNode::Handoff { .. })
        {
            continue;
        }

        let hoff = GraphNode::Handoff {
            kind: HandoffKind::Vec,
            src_span: src_node.span(),
            dst_span: dst_node.span(),
        };
        let (_node_id, out_edge_id) = partitioned_graph.insert_intermediate_node(edge_id, hoff);

        // Update barrier_crossers for inserted node.
        barrier_crossers.replace_edge(edge_id, out_edge_id);
    }

    // Register subgraphs. SubgraphMerge maintains operators in topo-sorted order per subgraph.
    // Filter out handoff nodes — they are not part of any subgraph.
    let mut subgraph_toposort = Vec::new();
    for nodes in subgraph_merge.subgraphs() {
        if nodes.is_empty() {
            continue;
        }
        // Skip single-node "subgraphs" that are handoff nodes.
        if nodes
            .iter()
            .any(|&n| matches!(partitioned_graph.node(n), GraphNode::Handoff { .. }))
        {
            continue;
        }
        let sg_id = partitioned_graph.insert_subgraph(nodes.to_vec()).unwrap();
        subgraph_toposort.push(sg_id);
    }
    partitioned_graph.set_subgraph_toposort(subgraph_toposort);
    Ok(())
}

/// Set `src` or `dst` color if `None` based on the other (if possible):
/// `None` indicates an op could be pull or push i.e. unary-in & unary-out.
/// So in that case we color `src` or `dst` based on its newfound neighbor (the other one).
///
/// Returns if `src` and `dst` can be in the same subgraph.
fn can_connect_colorize(
    node_color: &mut SparseSecondaryMap<GraphNodeId, Color>,
    src: GraphNodeId,
    dst: GraphNodeId,
) -> bool {
    // Pull -> Pull
    // Push -> Push
    // Pull -> [Computation] -> Push
    // Push -> [Handoff] -> Pull
    let can_connect = match (node_color.get(src), node_color.get(dst)) {
        // Linear chain, can't connect because it may cause future conflicts.
        // But if it doesn't in the _future_ we can connect it (once either/both ends are determined).
        (None, None) => false,

        // Infer left side.
        (None, Some(Color::Pull | Color::Comp)) => {
            node_color.insert(src, Color::Pull);
            true
        }
        (None, Some(Color::Push | Color::Hoff)) => {
            node_color.insert(src, Color::Push);
            true
        }

        // Infer right side.
        (Some(Color::Pull | Color::Hoff), None) => {
            node_color.insert(dst, Color::Pull);
            true
        }
        (Some(Color::Comp | Color::Push), None) => {
            node_color.insert(dst, Color::Push);
            true
        }

        // Both sides already specified.
        (Some(Color::Pull), Some(Color::Pull)) => true,
        (Some(Color::Pull), Some(Color::Comp)) => true,
        (Some(Color::Pull), Some(Color::Push)) => true,

        (Some(Color::Comp), Some(Color::Pull)) => false,
        (Some(Color::Comp), Some(Color::Comp)) => false,
        (Some(Color::Comp), Some(Color::Push)) => true,

        (Some(Color::Push), Some(Color::Pull)) => false,
        (Some(Color::Push), Some(Color::Comp)) => false,
        (Some(Color::Push), Some(Color::Push)) => true,

        // Handoffs are not part of subgraphs.
        (Some(Color::Hoff), Some(_)) => false,
        (Some(_), Some(Color::Hoff)) => false,
    };
    can_connect
}

/// Marks tick-boundary (`defer_tick` / `defer_tick_lazy`) handoffs with their delay type
/// for double-buffered codegen in `as_code`.
fn mark_tick_boundary_handoffs(
    partitioned_graph: &mut DfirGraph,
    barrier_crossers: &BarrierCrossers,
) {
    let tick_handoffs: Vec<_> = partitioned_graph
        .nodes()
        .filter_map(|(hoff_id, hoff)| {
            if !matches!(hoff, GraphNode::Handoff { .. }) {
                return None;
            }
            if partitioned_graph.node_degree_out(hoff_id) == 0 {
                return None;
            }
            let (succ_edge, _) = partitioned_graph.node_successors(hoff_id).next().unwrap();
            let delay_type = barrier_crossers
                .edge_barrier_crossers
                .get(succ_edge)
                .copied()?;
            match delay_type {
                DelayType::Tick | DelayType::TickLazy => Some((hoff_id, delay_type)),
                _ => None,
            }
        })
        .collect();

    for (hoff_id, delay_type) in tick_handoffs {
        partitioned_graph.set_handoff_delay_type(hoff_id, delay_type);
    }
}

/// Main method for this module. Partitions a flat [`DfirGraph`] into one with subgraphs.
///
/// Returns an error if an intra-tick cycle exists in the graph.
pub fn partition_graph(flat_graph: DfirGraph) -> Result<DfirGraph, Diagnostic> {
    // Pre-find barrier crossers (input edges with a `DelayType`).
    let mut barrier_crossers = find_barrier_crossers(&flat_graph);
    let mut partitioned_graph = flat_graph;

    // Partition into subgraphs and insert handoffs.
    make_subgraphs(&mut partitioned_graph, &mut barrier_crossers)?;

    // Mark tick-boundary handoffs for double-buffering.
    mark_tick_boundary_handoffs(&mut partitioned_graph, &barrier_crossers);

    Ok(partitioned_graph)
}
