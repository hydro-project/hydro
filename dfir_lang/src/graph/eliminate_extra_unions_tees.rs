#![warn(missing_docs)]

use super::ops::tee::TEE;
use super::ops::union::UNION;
use super::{DfirGraph, GraphNodeId};

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

/// Removes missing unions and tees. Must be applied BEFORE subgraph partitioning, i.e. on a flat
/// graph.
pub fn eliminate_extra_unions_tees(graph: &mut DfirGraph) {
    let extra_ops = find_unary_ops(graph, UNION.name)
        .chain(find_unary_ops(graph, TEE.name))
        .collect::<Vec<_>>();
    for extra_op in extra_ops {
        graph.remove_intermediate_node(extra_op);
    }
}
