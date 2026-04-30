//! Subgraph partioning algorithm

use std::collections::{BTreeMap, BTreeSet};

use proc_macro2::Span;
use slotmap::{SecondaryMap, SparseSecondaryMap};

use super::meta_graph::DfirGraph;
use super::ops::{DelayType, FloType};
use super::{Color, GraphEdgeId, GraphNode, GraphNodeId, GraphSubgraphId, graph_algorithms};
use crate::diagnostic::{Diagnostic, Level};
use crate::union_find::UnionFind;

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
    let singleton_barrier_crossers = partitioned_graph
        .node_ids()
        .flat_map(|dst| {
            partitioned_graph
                .node_singleton_references(dst)
                .iter()
                .flatten()
                .map(move |&src_ref| (src_ref, dst))
        })
        .collect();
    BarrierCrossers {
        edge_barrier_crossers,
        singleton_barrier_crossers,
    }
}

fn find_subgraph_unionfind(
    partitioned_graph: &DfirGraph,
    barrier_crossers: &BarrierCrossers,
) -> (UnionFind<GraphNodeId>, BTreeSet<GraphEdgeId>) {
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

    let mut subgraph_unionfind: UnionFind<GraphNodeId> =
        UnionFind::with_capacity(partitioned_graph.nodes().len());

    // Will contain all edges which are handoffs. Starts out with all edges and
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
                subgraph_unionfind.union(src, dst);
                assert!(handoff_edges.remove(&edge_id));
                progress = true;
            }
        }
    }

    (subgraph_unionfind, handoff_edges)
}

/// Builds the datastructures for checking which subgraph each node belongs to
/// after handoffs have already been inserted to partition subgraphs.
/// This list of nodes in each subgraph are returned in topological sort order.
fn make_subgraph_collect(
    partitioned_graph: &DfirGraph,
    mut subgraph_unionfind: UnionFind<GraphNodeId>,
) -> SecondaryMap<GraphNodeId, Vec<GraphNodeId>> {
    // We want the nodes of each subgraph to be listed in topo-sort order.
    // We could do this on each subgraph, or we could do it all at once on the
    // whole node graph by ignoring handoffs, which is what we do here:
    let topo_sort = graph_algorithms::topo_sort(
        partitioned_graph
            .nodes()
            .filter(|&(_, node)| !matches!(node, GraphNode::Handoff { .. }))
            .map(|(node_id, _)| node_id),
        |v| {
            partitioned_graph
                .node_predecessor_nodes(v)
                .filter(|&pred_id| {
                    let pred = partitioned_graph.node(pred_id);
                    !matches!(pred, GraphNode::Handoff { .. })
                })
        },
    )
    .expect("Subgraphs are in-out trees.");

    let mut grouped_nodes: SecondaryMap<GraphNodeId, Vec<GraphNodeId>> = Default::default();
    for node_id in topo_sort {
        let repr_node = subgraph_unionfind.find(node_id);
        if !grouped_nodes.contains_key(repr_node) {
            grouped_nodes.insert(repr_node, Default::default());
        }
        grouped_nodes[repr_node].push(node_id);
    }
    grouped_nodes
}

/// Find subgraph and insert handoffs.
/// Modifies barrier_crossers so that the edge OUT of an inserted handoff has
/// the DelayType data.
fn make_subgraphs(partitioned_graph: &mut DfirGraph, barrier_crossers: &mut BarrierCrossers) {
    // Algorithm:
    // 1. Each node begins as its own subgraph.
    // 2. Collect edges. (Future optimization: sort so edges which should not be split across a handoff come first).
    // 3. For each edge, try to join `(to, from)` into the same subgraph.

    // TODO(mingwei):
    // self.partitioned_graph.assert_valid();

    let (subgraph_unionfind, handoff_edges) =
        find_subgraph_unionfind(partitioned_graph, barrier_crossers);

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
            src_span: src_node.span(),
            dst_span: dst_node.span(),
        };
        let (_node_id, out_edge_id) = partitioned_graph.insert_intermediate_node(edge_id, hoff);

        // Update barrier_crossers for inserted node.
        barrier_crossers.replace_edge(edge_id, out_edge_id);
    }

    // Determine node's subgraph and subgraph's nodes.
    // This list of nodes in each subgraph are to be in topological sort order.
    // Eventually returned directly in the [`DfirGraph`].
    let grouped_nodes = make_subgraph_collect(partitioned_graph, subgraph_unionfind);
    for (_repr_node, member_nodes) in grouped_nodes {
        partitioned_graph.insert_subgraph(member_nodes).unwrap();
    }
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

/// Topologically sorts subgraphs and marks tick-boundary (`defer_tick` / `defer_tick_lazy`)
/// handoffs with their delay type for double-buffered codegen in `as_code`.
///
/// Returns an error if there is an intra-tick cycle (i.e. the subgraph DAG has a cycle when
/// tick-boundary edges are excluded).
fn order_subgraphs(
    partitioned_graph: &mut DfirGraph,
    barrier_crossers: &BarrierCrossers,
) -> Result<(), Diagnostic> {
    // Build a subgraph-level directed graph, excluding tick-boundary edges.
    let mut sg_preds: BTreeMap<GraphSubgraphId, Vec<GraphSubgraphId>> = Default::default();

    // Track which handoff edges are tick-boundary, keyed by (src_sg, dst_sg).
    let mut tick_edges: Vec<(GraphEdgeId, DelayType)> = Vec::new();

    // Iterate handoffs between subgraphs.
    for (node_id, node) in partitioned_graph.nodes() {
        if !matches!(node, GraphNode::Handoff { .. }) {
            continue;
        }
        assert_eq!(1, partitioned_graph.node_successors(node_id).len());
        let (succ_edge, succ) = partitioned_graph.node_successors(node_id).next().unwrap();

        let succ_edge_delaytype = barrier_crossers
            .edge_barrier_crossers
            .get(succ_edge)
            .copied();
        // Tick edges are excluded from the topo sort — they are cross-tick by design.
        if let Some(delay_type @ (DelayType::Tick | DelayType::TickLazy)) = succ_edge_delaytype {
            tick_edges.push((succ_edge, delay_type));
            continue;
        }

        assert_eq!(1, partitioned_graph.node_predecessors(node_id).len());
        let (_edge_id, pred) = partitioned_graph.node_predecessors(node_id).next().unwrap();

        let pred_sg = partitioned_graph.node_subgraph(pred).unwrap();
        let succ_sg = partitioned_graph.node_subgraph(succ).unwrap();

        sg_preds.entry(succ_sg).or_default().push(pred_sg);
    }
    // Include singleton reference edges.
    for &(pred, succ) in barrier_crossers.singleton_barrier_crossers.iter() {
        assert_ne!(pred, succ, "TODO(mingwei)");
        let pred_sg = partitioned_graph.node_subgraph(pred).unwrap();
        let succ_sg = partitioned_graph.node_subgraph(succ).unwrap();
        assert_ne!(pred_sg, succ_sg);
        sg_preds.entry(succ_sg).or_default().push(pred_sg);
    }

    // Topological sort — rejects intra-tick cycles.
    if let Err(cycle) = graph_algorithms::topo_sort(partitioned_graph.subgraph_ids(), |v| {
        sg_preds.get(&v).into_iter().flatten().copied()
    }) {
        let span = cycle
            .first()
            .and_then(|&sg_id| partitioned_graph.subgraph(sg_id).first().copied())
            .map(|n| partitioned_graph.node(n).span())
            .unwrap_or_else(Span::call_site);
        return Err(Diagnostic::spanned(
            span,
            Level::Error,
            "Cyclical dataflow within a tick is not supported. Use `defer_tick()` or `defer_tick_lazy()` to break the cycle across ticks.",
        ));
    }

    // Mark tick-boundary handoffs with their delay type.
    // These handoffs are excluded from the intra-tick topo ordering in
    // `as_code`; instead, their double-buffered handoff semantics defer data
    // across the tick boundary to the next tick.
    for (edge_id, delay_type) in tick_edges {
        let (hoff, _dst) = partitioned_graph.edge(edge_id);
        assert!(matches!(
            partitioned_graph.node(hoff),
            GraphNode::Handoff { .. }
        ));
        partitioned_graph.set_handoff_delay_type(hoff, delay_type);
    }
    Ok(())
}

/// Main method for this module. Partitions a flat [`DfirGraph`] into one with subgraphs.
///
/// Returns an error if an intra-tick cycle exists in the graph.
pub fn partition_graph(flat_graph: DfirGraph) -> Result<DfirGraph, Diagnostic> {
    // Pre-find barrier crossers (input edges with a `DelayType`).
    let mut barrier_crossers = find_barrier_crossers(&flat_graph);
    let mut partitioned_graph = flat_graph;

    // Partition into subgraphs.
    make_subgraphs(&mut partitioned_graph, &mut barrier_crossers);

    // Topologically order subgraphs and mark tick-boundary handoffs for double-buffering.
    order_subgraphs(&mut partitioned_graph, &barrier_crossers)?;

    Ok(partitioned_graph)
}
