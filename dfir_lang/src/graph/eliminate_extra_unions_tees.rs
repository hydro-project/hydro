#![warn(missing_docs)]

use super::ops::tee::TEE;
use super::ops::union::UNION;
use super::{DfirGraph, GraphNode, GraphNodeId};

fn find_unary_ops<'a>(
    graph: &'a DfirGraph,
    op_name: &'static str,
) -> impl 'a + Iterator<Item = GraphNodeId> {
    graph
        .node_ids()
        .filter(move |&node_id| {
            graph
                .node_op_inst(node_id)
                .is_some_and(|op_inst| op_name == op_inst.op_constraints.name)
        })
        .filter(|&node_id| {
            1 == graph.node_degree_in(node_id) && 1 == graph.node_degree_out(node_id)
        })
}

/// Find handoff nodes that have a handoff predecessor (i.e. adjacent handoffs) and have exactly
/// one input and one output, so they can be safely removed.
fn find_redundant_adjacent_handoffs(graph: &DfirGraph) -> Vec<GraphNodeId> {
    graph
        .node_ids()
        .filter(|&node_id| matches!(graph.node(node_id), GraphNode::Handoff { .. }))
        .filter(|&node_id| {
            1 == graph.node_degree_in(node_id) && 1 == graph.node_degree_out(node_id)
        })
        .filter(|&node_id| {
            graph
                .node_predecessor_nodes(node_id)
                .any(|pred_id| matches!(graph.node(pred_id), GraphNode::Handoff { .. }))
        })
        .collect()
}

/// Removes unary unions and tees, and collapses any adjacent handoffs that result.
/// Must be applied BEFORE subgraph partitioning, i.e. on a flat graph.
pub fn eliminate_extra_unions_tees(graph: &mut DfirGraph) {
    let extra_ops = find_unary_ops(graph, UNION.name)
        .chain(find_unary_ops(graph, TEE.name))
        .collect::<Vec<_>>();
    for extra_op in extra_ops {
        graph.remove_intermediate_node(extra_op);
    }

    // Removing unary tees/unions may create adjacent handoffs; collapse them.
    let redundant_handoffs = find_redundant_adjacent_handoffs(graph);
    for handoff in redundant_handoffs {
        graph.remove_intermediate_node(handoff);
    }
}
