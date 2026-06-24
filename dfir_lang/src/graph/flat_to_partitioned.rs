//! Subgraph partioning algorithm

use std::collections::{BTreeSet, HashMap, HashSet};

use itertools::Itertools;
use slotmap::{SecondaryMap, SparseSecondaryMap};

use super::meta_graph::DfirGraph;
use super::ops::{DelayType, FloType};
use super::{
    Color, GraphEdgeId, GraphLoopId, GraphNode, GraphNodeId, GraphSubgraphId, HandoffKind,
};
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

/// Find handoff reference access group ordering constraints: for the same handoff target, operators in
/// lower access groups must run before operators in higher access groups.
fn find_access_group_ordering(partitioned_graph: &DfirGraph) -> Vec<(GraphNodeId, GraphNodeId)> {
    let mut pairs = Vec::new();
    let refs_by_target = partitioned_graph.node_handoff_reference_groups();
    for (_handoff, groups) in refs_by_target {
        for (group_a, group_b) in groups.values().tuple_windows() {
            for &(node_a, _, _) in group_a {
                for &(node_b, _, _) in group_b {
                    // TODO(mingwei): handle with diagnostics.
                    assert_ne!(
                        node_a, node_b,
                        "encounted conflicted or cyclical handoff references\n{:?}\n{:?}",
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

    // Handoff references: producer must run before consumer.
    for node_id in partitioned_graph.node_ids() {
        for handoff_ref in partitioned_graph.node_handoff_references(node_id).iter() {
            if let Some(src) = handoff_ref.node_id {
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
                .node_handoff_references(dst)
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
        partitioned_graph.insert_subgraph(nodes.to_vec()).unwrap();
    }

    // Build a hierarchical topological sort that guarantees subgraphs within a loop
    // are contiguous in the final order.
    let subgraph_toposort = build_hierarchical_toposort(partitioned_graph)?;
    partitioned_graph.set_subgraph_toposort(subgraph_toposort);
    Ok(())
}

/// An item in a particular level of the loop hierarchy, used for hierarchical topo sort.
/// Either a bare subgraph or an entire child loop (treated as a single unit).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum HierItem {
    Subgraph(GraphSubgraphId),
    Loop(GraphLoopId),
}

/// Builds a hierarchical topological sort of subgraphs, guaranteeing that all subgraphs
/// within a loop are contiguous in the final order.
///
/// At each level of the loop hierarchy, the sortable items are bare subgraphs at that level
/// plus child loops (treated as opaque units). Edges between items are derived from handoff
/// edges crossing between them. The sort recurses into child loops and flattens the result.
fn build_hierarchical_toposort(
    graph: &DfirGraph,
) -> Result<Vec<GraphSubgraphId>, Diagnostic> {
    // Build a map: for each subgraph, what is its immediate loop context.
    // Also build a map: for each loop, which subgraphs are directly in it (not in child loops).
    let mut loop_direct_subgraphs: HashMap<Option<GraphLoopId>, Vec<GraphSubgraphId>> =
        HashMap::new();
    for sg_id in graph.subgraph_ids() {
        let loop_ctx = graph.subgraph_loop(sg_id);
        loop_direct_subgraphs
            .entry(loop_ctx)
            .or_default()
            .push(sg_id);
    }

    // Recursively sort a level. `parent_loop` is None for top-level.
    fn sort_level(
        graph: &DfirGraph,
        parent_loop: Option<GraphLoopId>,
        loop_direct_subgraphs: &HashMap<Option<GraphLoopId>, Vec<GraphSubgraphId>>,
    ) -> Result<Vec<GraphSubgraphId>, Diagnostic> {
        // Collect items at this level: direct subgraphs + child loops.
        let direct_sgs = loop_direct_subgraphs
            .get(&parent_loop)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        let child_loops: &[GraphLoopId] = match parent_loop {
            Some(parent) => graph.loop_children(parent),
            None => graph.root_loops(),
        };

        // If nothing at this level, return empty.
        if direct_sgs.is_empty() && child_loops.is_empty() {
            return Ok(Vec::new());
        }

        // Assign each item an index for the topo sort.
        let items: Vec<HierItem> = direct_sgs
            .iter()
            .map(|&sg| HierItem::Subgraph(sg))
            .chain(child_loops.iter().map(|&l| HierItem::Loop(l)))
            .collect();

        // Build a lookup: subgraph_id -> which HierItem it belongs to at this level.
        // For subgraphs directly at this level, it maps to themselves.
        // For subgraphs inside a child loop, it maps to that child loop's HierItem.
        let mut sg_to_item: HashMap<GraphSubgraphId, HierItem> = HashMap::new();
        for &sg in direct_sgs {
            sg_to_item.insert(sg, HierItem::Subgraph(sg));
        }
        for &child_loop in child_loops {
            // All subgraphs transitively inside this child loop map to the loop item.
            collect_subgraphs_in_loop(graph, child_loop, loop_direct_subgraphs, &mut |sg| {
                sg_to_item.insert(sg, HierItem::Loop(child_loop));
            });
        }

        // Build predecessor edges between items by examining handoff edges.
        // A handoff connects a sender subgraph to a receiver subgraph. If they map to
        // different items at this level, that's a dependency edge.
        let mut item_preds: HashMap<HierItem, HashSet<HierItem>> = HashMap::new();
        for &item in &items {
            item_preds.entry(item).or_default();
        }

        for (hoff_id, hoff) in graph.nodes() {
            if !matches!(hoff, GraphNode::Handoff { .. }) {
                continue;
            }
            // Each handoff has predecessors (senders) and successors (receivers).
            for (_edge, pred_id) in graph.node_predecessors(hoff_id) {
                let Some(pred_sg) = graph.node_subgraph(pred_id) else {
                    continue;
                };
                for (_edge, succ_id) in graph.node_successors(hoff_id) {
                    let Some(succ_sg) = graph.node_subgraph(succ_id) else {
                        continue;
                    };
                    // Look up what item each belongs to at this level.
                    let Some(&pred_item) = sg_to_item.get(&pred_sg) else {
                        continue;
                    };
                    let Some(&succ_item) = sg_to_item.get(&succ_sg) else {
                        continue;
                    };
                    if pred_item != succ_item {
                        item_preds.entry(succ_item).or_default().insert(pred_item);
                    }
                }
            }
        }

        // Topo sort items at this level.
        let sorted_items = super::graph_algorithms::topo_sort(items.iter().copied(), |item| {
            item_preds
                .get(&item)
                .into_iter()
                .flatten()
                .copied()
                .collect::<Vec<_>>()
                .into_iter()
        })
        .map_err(|_cycle| {
            Diagnostic::spanned(
                proc_macro2::Span::call_site(),
                Level::Error,
                "Cyclical dataflow between loop regions is not supported.",
            )
        })?;

        // Flatten: for each item in sorted order, emit its subgraphs.
        let mut result = Vec::new();
        for item in sorted_items {
            match item {
                HierItem::Subgraph(sg) => result.push(sg),
                HierItem::Loop(loop_id) => {
                    let inner = sort_level(graph, Some(loop_id), loop_direct_subgraphs)?;
                    result.extend(inner);
                }
            }
        }
        Ok(result)
    }

    sort_level(graph, None, &loop_direct_subgraphs)
}

/// Recursively collects all subgraph IDs transitively inside a loop (including nested child loops).
fn collect_subgraphs_in_loop(
    graph: &DfirGraph,
    loop_id: GraphLoopId,
    loop_direct_subgraphs: &HashMap<Option<GraphLoopId>, Vec<GraphSubgraphId>>,
    collector: &mut impl FnMut(GraphSubgraphId),
) {
    if let Some(direct) = loop_direct_subgraphs.get(&Some(loop_id)) {
        for &sg in direct {
            collector(sg);
        }
    }
    for &child in graph.loop_children(loop_id) {
        collect_subgraphs_in_loop(graph, child, loop_direct_subgraphs, collector);
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

#[cfg(test)]
mod tests {
    use proc_macro2::Span;

    use super::*;
    use crate::graph::{GraphNode, HandoffKind, PortIndexValue};

    fn elided() -> PortIndexValue {
        PortIndexValue::Elided(None)
    }

    fn make_op_node(graph: &mut DfirGraph, loop_ctx: Option<GraphLoopId>) -> GraphNodeId {
        let operator: crate::parse::Operator = syn::parse_quote! { identity() };
        graph.insert_node(GraphNode::Operator(operator), None, loop_ctx)
    }

    fn make_handoff_node(graph: &mut DfirGraph) -> GraphNodeId {
        graph.insert_node(
            GraphNode::Handoff {
                kind: HandoffKind::Vec,
                src_span: Span::call_site(),
                dst_span: Span::call_site(),
            },
            None,
            None,
        )
    }

    /// Test that subgraphs within a loop are contiguous in the toposort,
    /// even when there are unrelated subgraphs outside the loop.
    #[test]
    fn test_hierarchical_toposort_contiguity() {
        let mut graph = DfirGraph::default();

        // Create a loop.
        let loop_id = graph.insert_loop(None);

        // Create operator nodes: two outside the loop, two inside.
        let op_a = make_op_node(&mut graph, None);
        let op_b = make_op_node(&mut graph, None);
        let op_c = make_op_node(&mut graph, Some(loop_id));
        let op_d = make_op_node(&mut graph, Some(loop_id));

        // Create handoffs to connect: A -> [h1] -> C -> [h2] -> D -> [h3] -> B
        let h1 = make_handoff_node(&mut graph);
        let h2 = make_handoff_node(&mut graph);
        let h3 = make_handoff_node(&mut graph);

        graph.insert_edge(op_a, elided(), h1, elided());
        graph.insert_edge(h1, elided(), op_c, elided());
        graph.insert_edge(op_c, elided(), h2, elided());
        graph.insert_edge(h2, elided(), op_d, elided());
        graph.insert_edge(op_d, elided(), h3, elided());
        graph.insert_edge(h3, elided(), op_b, elided());

        // Manually create subgraphs (one per operator node).
        let sg_a = graph.insert_subgraph(vec![op_a]).unwrap();
        let sg_b = graph.insert_subgraph(vec![op_b]).unwrap();
        let sg_c = graph.insert_subgraph(vec![op_c]).unwrap();
        let sg_d = graph.insert_subgraph(vec![op_d]).unwrap();

        let toposort = build_hierarchical_toposort(&graph).unwrap();

        // Verify: A comes first, then C and D (contiguous, in order), then B.
        assert_eq!(toposort, vec![sg_a, sg_c, sg_d, sg_b]);
    }

    /// Test that two independent sibling loops each have their subgraphs contiguous.
    #[test]
    fn test_hierarchical_toposort_independent_loops() {
        let mut graph = DfirGraph::default();

        let loop1 = graph.insert_loop(None);
        let loop2 = graph.insert_loop(None);

        // Loop1: C, D. Loop2: E, F.
        let op_c = make_op_node(&mut graph, Some(loop1));
        let op_d = make_op_node(&mut graph, Some(loop1));
        let op_e = make_op_node(&mut graph, Some(loop2));
        let op_f = make_op_node(&mut graph, Some(loop2));

        // Internal handoffs within each loop.
        let h1 = make_handoff_node(&mut graph);
        let h2 = make_handoff_node(&mut graph);

        graph.insert_edge(op_c, elided(), h1, elided());
        graph.insert_edge(h1, elided(), op_d, elided());
        graph.insert_edge(op_e, elided(), h2, elided());
        graph.insert_edge(h2, elided(), op_f, elided());

        let sg_c = graph.insert_subgraph(vec![op_c]).unwrap();
        let sg_d = graph.insert_subgraph(vec![op_d]).unwrap();
        let sg_e = graph.insert_subgraph(vec![op_e]).unwrap();
        let sg_f = graph.insert_subgraph(vec![op_f]).unwrap();

        let toposort = build_hierarchical_toposort(&graph).unwrap();

        // Both loops should be contiguous. Order between loops is unspecified,
        // but within each loop the order must be respected.
        let pos_c = toposort.iter().position(|&s| s == sg_c).unwrap();
        let pos_d = toposort.iter().position(|&s| s == sg_d).unwrap();
        let pos_e = toposort.iter().position(|&s| s == sg_e).unwrap();
        let pos_f = toposort.iter().position(|&s| s == sg_f).unwrap();

        // C before D, E before F.
        assert!(pos_c < pos_d);
        assert!(pos_e < pos_f);

        // Contiguity: C and D are adjacent, E and F are adjacent.
        assert_eq!(pos_d - pos_c, 1, "Loop1 subgraphs must be contiguous");
        assert_eq!(pos_f - pos_e, 1, "Loop2 subgraphs must be contiguous");
    }

    /// Test nested loops: inner loop subgraphs are contiguous within the outer loop block.
    #[test]
    fn test_hierarchical_toposort_nested_loops() {
        let mut graph = DfirGraph::default();

        let outer = graph.insert_loop(None);
        let inner = graph.insert_loop(Some(outer));

        // Outer loop has: op_a, then inner loop (op_b, op_c), then op_d.
        let op_a = make_op_node(&mut graph, Some(outer));
        let op_b = make_op_node(&mut graph, Some(inner));
        let op_c = make_op_node(&mut graph, Some(inner));
        let op_d = make_op_node(&mut graph, Some(outer));

        // A -> [h1] -> B -> [h2] -> C -> [h3] -> D
        let h1 = make_handoff_node(&mut graph);
        let h2 = make_handoff_node(&mut graph);
        let h3 = make_handoff_node(&mut graph);

        graph.insert_edge(op_a, elided(), h1, elided());
        graph.insert_edge(h1, elided(), op_b, elided());
        graph.insert_edge(op_b, elided(), h2, elided());
        graph.insert_edge(h2, elided(), op_c, elided());
        graph.insert_edge(op_c, elided(), h3, elided());
        graph.insert_edge(h3, elided(), op_d, elided());

        let sg_a = graph.insert_subgraph(vec![op_a]).unwrap();
        let sg_b = graph.insert_subgraph(vec![op_b]).unwrap();
        let sg_c = graph.insert_subgraph(vec![op_c]).unwrap();
        let sg_d = graph.insert_subgraph(vec![op_d]).unwrap();

        let toposort = build_hierarchical_toposort(&graph).unwrap();

        // Expected: A, B, C, D (all contiguous within outer, B/C contiguous within inner).
        assert_eq!(toposort, vec![sg_a, sg_b, sg_c, sg_d]);
    }
}
