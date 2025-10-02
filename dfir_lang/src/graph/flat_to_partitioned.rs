//! Subgraph partioning algorithm

use std::collections::{BTreeMap, BTreeSet};

use slotmap::{SecondaryMap, SparseSecondaryMap};
use syn::parse_quote;

use super::meta_graph::DfirGraph;
use super::ops::{DelayType, FloType, find_node_op_constraints};
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

/// Stratification is surprisingly tricky. Basically it is topological sort, but with some nuance.
///
/// Returns an error if there is a cycle thru negation.
fn find_subgraph_strata(
    partitioned_graph: &mut DfirGraph,
    barrier_crossers: &BarrierCrossers,
) -> Result<(), Diagnostic> {
    // Determine subgraphs's stratum number.
    // Find SCCs ignoring `defer_tick()` (`DelayType::Tick`) edges, then do TopoSort on the
    // resulting DAG.
    // Cycles thru cross-stratum negative edges (both `DelayType::Tick` and `DelayType::Stratum`)
    // are an error.

    // Generate a subgraph graph. I.e. each node is a subgraph.
    // Edges are connections between subgraphs, ignoring tick-crossers.
    // TODO: use DiMulGraph here?
    #[derive(Default)]
    struct SubgraphGraph {
        preds: BTreeMap<GraphSubgraphId, Vec<GraphSubgraphId>>,
        succs: BTreeMap<GraphSubgraphId, Vec<GraphSubgraphId>>,
    }
    impl SubgraphGraph {
        fn insert_edge(&mut self, src: GraphSubgraphId, dst: GraphSubgraphId) {
            self.preds.entry(dst).or_default().push(src);
            self.succs.entry(src).or_default().push(dst);
        }
    }
    let mut subgraph_graph = SubgraphGraph::default();

    // Negative (next stratum) connections between subgraphs. (Ignore `defer_tick()` connections).
    let mut subgraph_stratum_barriers: BTreeSet<(GraphSubgraphId, GraphSubgraphId)> =
        Default::default();

    // Iterate handoffs between subgraphs, to build a subgraph meta-graph.
    for (node_id, node) in partitioned_graph.nodes() {
        if matches!(node, GraphNode::Handoff { .. }) {
            assert_eq!(1, partitioned_graph.node_successors(node_id).count());
            let (succ_edge, succ) = partitioned_graph.node_successors(node_id).next().unwrap();

            // TODO(mingwei): Should we look at the singleton references too?
            let succ_edge_delaytype = barrier_crossers
                .edge_barrier_crossers
                .get(succ_edge)
                .copied();
            // Ignore tick edges.
            if let Some(DelayType::Tick | DelayType::TickLazy) = succ_edge_delaytype {
                continue;
            }

            assert_eq!(1, partitioned_graph.node_predecessors(node_id).count());
            let (_edge_id, pred) = partitioned_graph.node_predecessors(node_id).next().unwrap();

            let pred_sg = partitioned_graph.node_subgraph(pred).unwrap();
            let succ_sg = partitioned_graph.node_subgraph(succ).unwrap();

            subgraph_graph.insert_edge(pred_sg, succ_sg);

            if Some(DelayType::Stratum) == succ_edge_delaytype {
                subgraph_stratum_barriers.insert((pred_sg, succ_sg));
            }
        }
    }
    // Include reference edges as well.
    // TODO(mingwei): deduplicate graph building code.
    for &(pred, succ) in barrier_crossers.singleton_barrier_crossers.iter() {
        assert_ne!(pred, succ, "TODO(mingwei)");
        let pred_sg = partitioned_graph.node_subgraph(pred).unwrap();
        let succ_sg = partitioned_graph.node_subgraph(succ).unwrap();
        assert_ne!(pred_sg, succ_sg);
        subgraph_graph.insert_edge(pred_sg, succ_sg);
        subgraph_stratum_barriers.insert((pred_sg, succ_sg));
    }

    // Topological sort (of strongly connected components) is how we find the (nondecreasing)
    // order of strata.
    let topo_sort_order = graph_algorithms::topo_sort_scc(
        || partitioned_graph.subgraph_ids(),
        |v| subgraph_graph.preds.get(&v).into_iter().flatten().cloned(),
        |u| subgraph_graph.succs.get(&u).into_iter().flatten().cloned(),
    );

    // Each subgraph's stratum number is the same as it's predecessors.
    //
    // Unless:
    // - At the top level: there is a negative edge (e.g. `fold()`), then we increment.
    // - Entering or exiting a loop.
    for sg_id in topo_sort_order {
        let curr_loop = partitioned_graph.subgraph_loop(sg_id);

        let stratum = subgraph_graph
            .preds
            .get(&sg_id)
            .into_iter()
            .flatten()
            .filter_map(|&pred_sg_id| {
                partitioned_graph
                    .subgraph_stratum(pred_sg_id)
                    .map(|stratum| {
                        let pred_loop = partitioned_graph.subgraph_loop(pred_sg_id);
                        if curr_loop != pred_loop {
                            // Entering or exiting a loop.
                            stratum + 1
                        } else if curr_loop.is_none()
                            && subgraph_stratum_barriers.contains(&(pred_sg_id, sg_id))
                        {
                            // Top level && negative edge.
                            stratum + 1
                        } else {
                            stratum
                        }
                    })
            })
            .max()
            .unwrap_or(0);
        partitioned_graph.set_subgraph_stratum(sg_id, stratum);
    }

    // Re-introduce the `defer_tick()` edges, ensuring they actually go to the next tick.
    let extra_stratum = partitioned_graph.max_stratum().unwrap_or(0) + 1; // Used for `defer_tick()` delayer subgraphs.
    for (edge_id, &delay_type) in barrier_crossers.edge_barrier_crossers.iter() {
        let (hoff, dst) = partitioned_graph.edge(edge_id);
        // Ignore barriers within `loop {` blocks.
        if partitioned_graph.node_loop(dst).is_some() {
            continue;
        }
        let (_hoff_port, dst_port) = partitioned_graph.edge_ports(edge_id);

        assert_eq!(1, partitioned_graph.node_predecessors(hoff).count());
        let src = partitioned_graph
            .node_predecessor_nodes(hoff)
            .next()
            .unwrap();

        let src_sg = partitioned_graph.node_subgraph(src).unwrap();
        let dst_sg = partitioned_graph.node_subgraph(dst).unwrap();
        let src_stratum = partitioned_graph.subgraph_stratum(src_sg);
        let dst_stratum = partitioned_graph.subgraph_stratum(dst_sg);
        let dst_span = partitioned_graph.node(dst).span();
        match delay_type {
            DelayType::Tick | DelayType::TickLazy => {
                let is_lazy = matches!(delay_type, DelayType::TickLazy);
                // If tick edge goes foreward in stratum, need to buffer.
                // (TODO(mingwei): could use a different kind of handoff.)
                // Or if lazy, need to create extra subgraph to mark as lazy.
                if src_stratum <= dst_stratum || is_lazy {
                    // We inject a new subgraph between the src/dst which runs as the last stratum
                    // of the tick and therefore delays the data until the next tick.

                    // Before: A (src) -> H -> B (dst)
                    // Then add intermediate identity:
                    let (new_node_id, new_edge_id) = partitioned_graph.insert_intermediate_node(
                        edge_id,
                        // TODO(mingwei): Proper span w/ `parse_quote_spanned!`?
                        GraphNode::Operator(parse_quote! { identity() }),
                    );
                    // Intermediate: A (src) -> H -> ID -> B (dst)
                    let hoff = GraphNode::Handoff {
                        // Span to the node that has the input stratum barrier.
                        src_span: dst_span,
                        dst_span,
                    };
                    let (_hoff_node_id, _hoff_edge_id) =
                        partitioned_graph.insert_intermediate_node(new_edge_id, hoff);
                    // After: A (src) -> H -> ID -> H' -> B (dst)

                    // Set stratum number for new intermediate:
                    // Create subgraph.
                    let new_subgraph_id = partitioned_graph
                        .insert_subgraph(vec![new_node_id])
                        .unwrap();

                    // Assign stratum.
                    partitioned_graph.set_subgraph_stratum(new_subgraph_id, extra_stratum);

                    // Assign laziness.
                    partitioned_graph.set_subgraph_laziness(new_subgraph_id, is_lazy);
                }
            }
            DelayType::Stratum => {
                // Any negative edges which go onto the same or previous stratum are bad.
                // Indicates an unbroken negative cycle.
                // TODO(mingwei): This check is insufficient: https://github.com/hydro-project/hydro/issues/1115#issuecomment-2018385033
                if dst_stratum <= src_stratum {
                    return Err(Diagnostic::spanned(
                        dst_port.span(),
                        Level::Error,
                        "Negative edge creates a negative cycle which must be broken with a `defer_tick()` operator.",
                    ));
                }
            }
            DelayType::MonotoneAccum => {
                // cycles are actually fine
                continue;
            }
        }
    }
    Ok(())
}

/// Put `is_external_input: true` operators in separate stratum 0 subgraphs if they are not in stratum 0.
/// By ripping them out of their subgraph/stratum if they're not already in statum 0.
fn separate_external_inputs(partitioned_graph: &mut DfirGraph) {
    let external_input_nodes: Vec<_> = partitioned_graph
        .nodes()
        // Ensure node is an operator (not a handoff), get constraints spec.
        .filter_map(|(node_id, node)| {
            find_node_op_constraints(node).map(|op_constraints| (node_id, op_constraints))
        })
        // Ensure current `node_id` is an external input.
        .filter(|(_node_id, op_constraints)| op_constraints.is_external_input)
        // Collect just `node_id`s.
        .map(|(node_id, _op_constraints)| node_id)
        // Ignore if operator node is already stratum 0.
        .filter(|&node_id| {
            0 != partitioned_graph
                .subgraph_stratum(partitioned_graph.node_subgraph(node_id).unwrap())
                .unwrap()
        })
        .collect();

    for node_id in external_input_nodes {
        // Remove node from old subgraph.
        assert!(
            partitioned_graph.remove_from_subgraph(node_id),
            "Cannot move input node that is not in a subgraph, this is a bug."
        );
        // Create new subgraph in stratum 0 for this source.
        let new_sg_id = partitioned_graph.insert_subgraph(vec![node_id]).unwrap();
        partitioned_graph.set_subgraph_stratum(new_sg_id, 0);

        // Insert handoff.
        for edge_id in partitioned_graph
            .node_successor_edges(node_id)
            .collect::<Vec<_>>()
        {
            let span = partitioned_graph.node(node_id).span();
            let hoff = GraphNode::Handoff {
                src_span: span,
                dst_span: span,
            };
            partitioned_graph.insert_intermediate_node(edge_id, hoff);
        }
    }
}

/// Main method for this module. Partions a flat [`DfirGraph`] into one with subgraphs.
///
/// Returns an error if a negative cycle exists in the graph. Negative cycles prevent partioning.
pub fn partition_graph(flat_graph: DfirGraph) -> Result<DfirGraph, Diagnostic> {
    // Pre-find barrier crossers (input edges with a `DelayType`).
    let mut barrier_crossers = find_barrier_crossers(&flat_graph);
    let mut partitioned_graph = flat_graph;

    // Partition into subgraphs.
    make_subgraphs(&mut partitioned_graph, &mut barrier_crossers);

    // Find strata for subgraphs (early returns with error if negative cycle found).
    find_subgraph_strata(&mut partitioned_graph, &barrier_crossers)?;

    // Ensure all external inputs are in stratum 0.
    separate_external_inputs(&mut partitioned_graph);

    Ok(partitioned_graph)
}
