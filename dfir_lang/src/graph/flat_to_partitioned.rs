//! Subgraph partioning algorithm

use std::collections::BTreeSet;

use itertools::Itertools;
use slotmap::{SecondaryMap, SparseSecondaryMap};

use super::meta_graph::DfirGraph;
use super::ops::{DelayType, FloType};
use super::{Color, GraphEdgeId, GraphNode, GraphNodeId, HandoffKind};
use crate::diagnostic::{Diagnostic, Level};
use crate::graph::graph_algorithms::SubgraphMerge;

/// Find edge barriers: edges whose destination operator declares an input delay type.
/// Excludes edges within `loop {}` blocks.
///
/// Returns:
/// - Tick/TickLazy edges keyed by edge ID (for topo-sort exclusion and handoff marking).
/// - All barrier (src, dst) node pairs (for the enemies set).
fn find_edge_barriers(
    partitioned_graph: &DfirGraph,
) -> (
    SecondaryMap<GraphEdgeId, DelayType>,
    Vec<(GraphNodeId, GraphNodeId)>,
) {
    let mut tick_edges = SecondaryMap::new();
    let mut barrier_pairs = Vec::new();

    for (edge_id, (src, dst)) in partitioned_graph.edges() {
        // Ignore barriers within `loop {` blocks.
        if partitioned_graph.node_loop(dst).is_some() {
            continue;
        }
        let Some(op_inst) = partitioned_graph.node_op_inst(dst) else {
            continue;
        };
        let (_src_port, dst_port) = partitioned_graph.edge_ports(edge_id);
        let Some(delay_type) = (op_inst.op_constraints.input_delaytype_fn)(dst_port) else {
            continue;
        };

        barrier_pairs.push((src, dst));
        if matches!(delay_type, DelayType::Tick | DelayType::TickLazy) {
            tick_edges.insert(edge_id, delay_type);
        }
    }

    (tick_edges, barrier_pairs)
}

/// Find access group ordering constraints: for the same singleton target, operators in
/// lower access groups must run before operators in higher access groups.
fn find_access_group_ordering(partitioned_graph: &DfirGraph) -> Vec<(GraphNodeId, GraphNodeId)> {
    let mut pairs = Vec::new();
    let refs_by_target = partitioned_graph.node_singleton_reference_groups();
    for (_singleton, groups) in refs_by_target {
        for (group_a, group_b) in groups.values().tuple_windows() {
            for &(node_a, _, _) in group_a {
                for &(node_b, _, _) in group_b {
                    // TODO(mingwei): handle with diagnostics.
                    assert_ne!(
                        node_a, node_b,
                        "encounted conflicted or cyclical singleton references\n{:?}\n{:?}",
                        group_a, group_b,
                    );
                    pairs.push((node_a, node_b));
                }
            }
        }
    }
    pairs
}

fn find_subgraph_unionfind(
    partitioned_graph: &DfirGraph,
    tick_edges: &SecondaryMap<GraphEdgeId, DelayType>,
    edge_barrier_pairs: &[(GraphNodeId, GraphNodeId)],
    access_group_pairs: &[(GraphNodeId, GraphNodeId)],
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

    // Pre-compute all predecessor edges for the topological sort.
    let mut all_preds: SecondaryMap<GraphNodeId, Vec<GraphNodeId>> = SecondaryMap::new();

    // Pipe predecessors (excluding tick edges which are cross-tick).
    for (edge_id, (src, dst)) in partitioned_graph.edges() {
        if !tick_edges.contains_key(edge_id) {
            all_preds.entry(dst).unwrap().or_default().push(src);
        }
    }

    // Singleton references: producer must run before consumer.
    for node_id in partitioned_graph.node_ids() {
        for singleton_ref in partitioned_graph.node_singleton_references(node_id).iter() {
            if let Some(src) = singleton_ref.node_id {
                all_preds.entry(node_id).unwrap().or_default().push(src);
                // Extra ordering: if the ref target is a handoff, its pipe consumers
                // depend on the borrower (borrower runs before consumer).
                if let GraphNode::Handoff { .. } = partitioned_graph.node(src) {
                    for (_edge, consumer) in partitioned_graph.node_successors(src) {
                        all_preds
                            .entry(consumer)
                            .unwrap()
                            .or_default()
                            .push(node_id);
                    }
                }
            }
        }
    }

    // Access group ordering.
    for &(src, dst) in access_group_pairs {
        all_preds.entry(dst).unwrap().or_default().push(src);
    }

    // Build enemies: all node pairs that must not be in the same subgraph.
    let enemies = edge_barrier_pairs
        .iter()
        .copied()
        .chain(access_group_pairs.iter().copied())
        .chain(partitioned_graph.node_ids().flat_map(|dst| {
            partitioned_graph
                .node_singleton_references(dst)
                .iter()
                .filter_map(|r| r.node_id)
                .map(move |src| (src, dst))
        }));

    let mut subgraph_unionfind = SubgraphMerge::<GraphNodeId>::new(
        partitioned_graph.node_ids(),
        |node_id| all_preds.get(node_id).into_iter().flatten().copied(),
        enemies,
    )
    .map_err(|cycle| {
        let span = cycle
            .first()
            .map(|&node_id| partitioned_graph.node(node_id).span())
            .unwrap_or_else(proc_macro2::Span::call_site);
        let node_cycle = cycle
            .iter()
            .map(|&node_id| partitioned_graph.node(node_id).to_pretty_string())
            .collect::<Vec<_>>();
        Diagnostic::spanned(
            span,
            Level::Error,
            format!(
                "Cyclical dataflow within a tick is not supported. Use `defer_tick()` or `defer_tick_lazy()` to break the cycle across ticks. \
                Cycle: {:?}",
                node_cycle,
            ),
        )
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

/// Find subgraphs and insert handoffs.
fn make_subgraphs(
    partitioned_graph: &mut DfirGraph,
    tick_edges: &mut SecondaryMap<GraphEdgeId, DelayType>,
    edge_barrier_pairs: &[(GraphNodeId, GraphNodeId)],
    access_group_pairs: &[(GraphNodeId, GraphNodeId)],
) -> Result<(), Diagnostic> {
    // Algorithm:
    // 1. Each node begins as its own subgraph.
    // 2. Collect edges. (Future optimization: sort so edges which should not be split across a handoff come first).
    // 3. For each edge, try to join `(to, from)` into the same subgraph.

    // TODO(mingwei):
    // self.partitioned_graph.assert_valid();

    let (subgraph_merge, handoff_edges) = find_subgraph_unionfind(
        partitioned_graph,
        tick_edges,
        edge_barrier_pairs,
        access_group_pairs,
    )?;

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

        // Update tick_edges for inserted node.
        if let Some(delay_type) = tick_edges.remove(edge_id) {
            tick_edges.insert(out_edge_id, delay_type);
        }
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
    tick_edges: &SecondaryMap<GraphEdgeId, DelayType>,
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
            let &delay_type = tick_edges.get(succ_edge)?;
            Some((hoff_id, delay_type))
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
    let (mut tick_edges, edge_barrier_pairs) = find_edge_barriers(&flat_graph);
    let access_group_pairs = find_access_group_ordering(&flat_graph);
    let mut partitioned_graph = flat_graph;

    // Partition into subgraphs and insert handoffs.
    make_subgraphs(
        &mut partitioned_graph,
        &mut tick_edges,
        &edge_barrier_pairs,
        &access_group_pairs,
    )?;

    // Mark tick-boundary handoffs for double-buffering.
    mark_tick_boundary_handoffs(&mut partitioned_graph, &tick_edges);

    Ok(partitioned_graph)
}
