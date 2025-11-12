#![warn(missing_docs)]

extern crate proc_macro;

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt::Debug;
use std::iter::FusedIterator;

use itertools::Itertools;
use proc_macro2::{Ident, Literal, Span, TokenStream};
use quote::{ToTokens, TokenStreamExt, format_ident, quote, quote_spanned};
use serde::{Deserialize, Serialize};
use slotmap::{Key, SecondaryMap, SlotMap, SparseSecondaryMap};
use syn::spanned::Spanned;

use super::graph_write::{Dot, GraphWrite, Mermaid};
use super::ops::{
    DelayType, OPERATORS, OperatorWriteOutput, WriteContextArgs, find_op_op_constraints,
    null_write_iterator_fn,
};
use super::{
    CONTEXT, Color, DiMulGraph, GRAPH, GraphEdgeId, GraphLoopId, GraphNode, GraphNodeId,
    GraphSubgraphId, HANDOFF_NODE_STR, MODULE_BOUNDARY_NODE_STR, OperatorInstance, PortIndexValue,
    Varname, change_spans, get_operator_generics,
};
use crate::diagnostic::{Diagnostic, Level};
use crate::pretty_span::{PrettyRowCol, PrettySpan};
use crate::process_singletons;

/// An abstract "meta graph" representation of a DFIR graph.
///
/// Can be with or without subgraph partitioning, stratification, and handoff insertion. This is
/// the meta graph used for generating Rust source code in macros from DFIR sytnax.
///
/// This struct has a lot of methods for manipulating the graph, vaguely grouped together in
/// separate `impl` blocks. You might notice a few particularly specific arbitray-seeming methods
/// in here--those are just what was needed for the compilation algorithms. If you need another
/// method then add it.
#[derive(Default, Debug, Serialize, Deserialize)]
pub struct DfirGraph {
    /// Each node type (operator or handoff).
    nodes: SlotMap<GraphNodeId, GraphNode>,

    /// Instance data corresponding to each operator node.
    /// This field will be empty after deserialization.
    #[serde(skip)]
    operator_instances: SecondaryMap<GraphNodeId, OperatorInstance>,
    /// Debugging/tracing tag for each operator node.
    operator_tag: SecondaryMap<GraphNodeId, String>,
    /// Graph data structure (two-way adjacency list).
    graph: DiMulGraph<GraphNodeId, GraphEdgeId>,
    /// Input and output port for each edge.
    ports: SecondaryMap<GraphEdgeId, (PortIndexValue, PortIndexValue)>,

    /// Which loop a node belongs to (or none for top-level).
    node_loops: SecondaryMap<GraphNodeId, GraphLoopId>,
    /// Which nodes belong to each loop.
    loop_nodes: SlotMap<GraphLoopId, Vec<GraphNodeId>>,
    /// For the loop, what is its parent (`None` for top-level).
    loop_parent: SparseSecondaryMap<GraphLoopId, GraphLoopId>,
    /// What loops are at the root.
    root_loops: Vec<GraphLoopId>,
    /// For the loop, what are its child loops.
    loop_children: SecondaryMap<GraphLoopId, Vec<GraphLoopId>>,

    /// Which subgraph each node belongs to.
    node_subgraph: SecondaryMap<GraphNodeId, GraphSubgraphId>,

    /// Which nodes belong to each subgraph.
    subgraph_nodes: SlotMap<GraphSubgraphId, Vec<GraphNodeId>>,
    /// Which stratum each subgraph belongs to.
    subgraph_stratum: SecondaryMap<GraphSubgraphId, usize>,

    /// Resolved singletons varnames references, per node.
    node_singleton_references: SparseSecondaryMap<GraphNodeId, Vec<Option<GraphNodeId>>>,
    /// What variable name each graph node belongs to (if any). For debugging (graph writing) purposes only.
    node_varnames: SparseSecondaryMap<GraphNodeId, Varname>,

    /// If this subgraph is 'lazy' then when it sends data to a lower stratum it does not cause a new tick to start
    /// This is to support lazy defers
    /// If the value does not exist for a given subgraph id then the subgraph is not lazy.
    subgraph_laziness: SecondaryMap<GraphSubgraphId, bool>,
}

/// Basic methods.
impl DfirGraph {
    /// Create a new empty graph.
    pub fn new() -> Self {
        Default::default()
    }
}

/// Node methods.
impl DfirGraph {
    /// Get a node with its operator instance (if applicable).
    pub fn node(&self, node_id: GraphNodeId) -> &GraphNode {
        self.nodes.get(node_id).expect("Node not found.")
    }

    /// Get the `OperatorInstance` for a given node. Node must be an operator and have an
    /// `OperatorInstance` present, otherwise will return `None`.
    ///
    /// Note that no operator instances will be persent after deserialization.
    pub fn node_op_inst(&self, node_id: GraphNodeId) -> Option<&OperatorInstance> {
        self.operator_instances.get(node_id)
    }

    /// Get the debug variable name attached to a graph node.
    pub fn node_varname(&self, node_id: GraphNodeId) -> Option<&Varname> {
        self.node_varnames.get(node_id)
    }

    /// Get subgraph for node.
    pub fn node_subgraph(&self, node_id: GraphNodeId) -> Option<GraphSubgraphId> {
        self.node_subgraph.get(node_id).copied()
    }

    /// Degree into a node, i.e. the number of predecessors.
    pub fn node_degree_in(&self, node_id: GraphNodeId) -> usize {
        self.graph.degree_in(node_id)
    }

    /// Degree out of a node, i.e. the number of successors.
    pub fn node_degree_out(&self, node_id: GraphNodeId) -> usize {
        self.graph.degree_out(node_id)
    }

    /// Successors, iterator of `(GraphEdgeId, GraphNodeId)` of outgoing edges.
    pub fn node_successors(
        &self,
        src: GraphNodeId,
    ) -> impl '_
    + DoubleEndedIterator<Item = (GraphEdgeId, GraphNodeId)>
    + ExactSizeIterator
    + FusedIterator
    + Clone
    + Debug {
        self.graph.successors(src)
    }

    /// Predecessors, iterator of `(GraphEdgeId, GraphNodeId)` of incoming edges.
    pub fn node_predecessors(
        &self,
        dst: GraphNodeId,
    ) -> impl '_
    + DoubleEndedIterator<Item = (GraphEdgeId, GraphNodeId)>
    + ExactSizeIterator
    + FusedIterator
    + Clone
    + Debug {
        self.graph.predecessors(dst)
    }

    /// Successor edges, iterator of `GraphEdgeId` of outgoing edges.
    pub fn node_successor_edges(
        &self,
        src: GraphNodeId,
    ) -> impl '_
    + DoubleEndedIterator<Item = GraphEdgeId>
    + ExactSizeIterator
    + FusedIterator
    + Clone
    + Debug {
        self.graph.successor_edges(src)
    }

    /// Predecessor edges, iterator of `GraphEdgeId` of incoming edges.
    pub fn node_predecessor_edges(
        &self,
        dst: GraphNodeId,
    ) -> impl '_
    + DoubleEndedIterator<Item = GraphEdgeId>
    + ExactSizeIterator
    + FusedIterator
    + Clone
    + Debug {
        self.graph.predecessor_edges(dst)
    }

    /// Successor nodes, iterator of `GraphNodeId`.
    pub fn node_successor_nodes(
        &self,
        src: GraphNodeId,
    ) -> impl '_
    + DoubleEndedIterator<Item = GraphNodeId>
    + ExactSizeIterator
    + FusedIterator
    + Clone
    + Debug {
        self.graph.successor_vertices(src)
    }

    /// Predecessor nodes, iterator of `GraphNodeId`.
    pub fn node_predecessor_nodes(
        &self,
        dst: GraphNodeId,
    ) -> impl '_
    + DoubleEndedIterator<Item = GraphNodeId>
    + ExactSizeIterator
    + FusedIterator
    + Clone
    + Debug {
        self.graph.predecessor_vertices(dst)
    }

    /// Iterator of node IDs `GraphNodeId`.
    pub fn node_ids(&self) -> slotmap::basic::Keys<'_, GraphNodeId, GraphNode> {
        self.nodes.keys()
    }

    /// Iterator over `(GraphNodeId, &Node)` pairs.
    pub fn nodes(&self) -> slotmap::basic::Iter<'_, GraphNodeId, GraphNode> {
        self.nodes.iter()
    }

    /// Insert a node, assigning the given varname.
    pub fn insert_node(
        &mut self,
        node: GraphNode,
        varname_opt: Option<Ident>,
        loop_opt: Option<GraphLoopId>,
    ) -> GraphNodeId {
        let node_id = self.nodes.insert(node);
        if let Some(varname) = varname_opt {
            self.node_varnames.insert(node_id, Varname(varname));
        }
        if let Some(loop_id) = loop_opt {
            self.node_loops.insert(node_id, loop_id);
            self.loop_nodes[loop_id].push(node_id);
        }
        node_id
    }

    /// Insert an operator instance for the given node. Panics if already set.
    pub fn insert_node_op_inst(&mut self, node_id: GraphNodeId, op_inst: OperatorInstance) {
        assert!(matches!(
            self.nodes.get(node_id),
            Some(GraphNode::Operator(_))
        ));
        let old_inst = self.operator_instances.insert(node_id, op_inst);
        assert!(old_inst.is_none());
    }

    /// Assign all operator instances if not set. Write diagnostic messages/errors into `diagnostics`.
    pub fn insert_node_op_insts_all(&mut self, diagnostics: &mut Vec<Diagnostic>) {
        let mut op_insts = Vec::new();
        for (node_id, node) in self.nodes() {
            let GraphNode::Operator(operator) = node else {
                continue;
            };
            if self.node_op_inst(node_id).is_some() {
                continue;
            };

            // Op constraints.
            let Some(op_constraints) = find_op_op_constraints(operator) else {
                diagnostics.push(Diagnostic::spanned(
                    operator.path.span(),
                    Level::Error,
                    format!("Unknown operator `{}`", operator.name_string()),
                ));
                continue;
            };

            // Input and output ports.
            let (input_ports, output_ports) = {
                let mut input_edges: Vec<(&PortIndexValue, GraphNodeId)> = self
                    .node_predecessors(node_id)
                    .map(|(edge_id, pred_id)| (self.edge_ports(edge_id).1, pred_id))
                    .collect();
                // Ensure sorted by port index.
                input_edges.sort();
                let input_ports: Vec<PortIndexValue> = input_edges
                    .into_iter()
                    .map(|(port, _pred)| port)
                    .cloned()
                    .collect();

                // Collect output arguments (successors).
                let mut output_edges: Vec<(&PortIndexValue, GraphNodeId)> = self
                    .node_successors(node_id)
                    .map(|(edge_id, succ)| (self.edge_ports(edge_id).0, succ))
                    .collect();
                // Ensure sorted by port index.
                output_edges.sort();
                let output_ports: Vec<PortIndexValue> = output_edges
                    .into_iter()
                    .map(|(port, _succ)| port)
                    .cloned()
                    .collect();

                (input_ports, output_ports)
            };

            // Generic arguments.
            let generics = get_operator_generics(diagnostics, operator);
            // Generic argument errors.
            {
                // Span of `generic_args` (if it exists), otherwise span of the operator name.
                let generics_span = generics
                    .generic_args
                    .as_ref()
                    .map(Spanned::span)
                    .unwrap_or_else(|| operator.path.span());

                if !op_constraints
                    .persistence_args
                    .contains(&generics.persistence_args.len())
                {
                    diagnostics.push(Diagnostic::spanned(
                        generics_span,
                        Level::Error,
                        format!(
                            "`{}` should have {} persistence lifetime arguments, actually has {}.",
                            op_constraints.name,
                            op_constraints.persistence_args.human_string(),
                            generics.persistence_args.len()
                        ),
                    ));
                }
                if !op_constraints.type_args.contains(&generics.type_args.len()) {
                    diagnostics.push(Diagnostic::spanned(
                        generics_span,
                        Level::Error,
                        format!(
                            "`{}` should have {} generic type arguments, actually has {}.",
                            op_constraints.name,
                            op_constraints.type_args.human_string(),
                            generics.type_args.len()
                        ),
                    ));
                }
            }

            op_insts.push((
                node_id,
                OperatorInstance {
                    op_constraints,
                    input_ports,
                    output_ports,
                    singletons_referenced: operator.singletons_referenced.clone(),
                    generics,
                    arguments_pre: operator.args.clone(),
                    arguments_raw: operator.args_raw.clone(),
                },
            ));
        }

        for (node_id, op_inst) in op_insts {
            self.insert_node_op_inst(node_id, op_inst);
        }
    }

    /// Inserts a node between two existing nodes connected by the given `edge_id`.
    ///
    /// `edge`: (src, dst, dst_idx)
    ///
    /// Before: A (src) ------------> B (dst)
    /// After:  A (src) -> X (new) -> B (dst)
    ///
    /// Returns the ID of X & ID of edge OUT of X.
    ///
    /// Note that both the edges will be new and `edge_id` will be removed. Both new edges will
    /// get the edge type of the original edge.
    pub fn insert_intermediate_node(
        &mut self,
        edge_id: GraphEdgeId,
        new_node: GraphNode,
    ) -> (GraphNodeId, GraphEdgeId) {
        let span = Some(new_node.span());

        // Make corresponding operator instance (if `node` is an operator).
        let op_inst_opt = 'oc: {
            let GraphNode::Operator(operator) = &new_node else {
                break 'oc None;
            };
            let Some(op_constraints) = find_op_op_constraints(operator) else {
                break 'oc None;
            };
            let (input_port, output_port) = self.ports.get(edge_id).cloned().unwrap();
            let generics = get_operator_generics(
                &mut Vec::new(), // TODO(mingwei) diagnostics
                operator,
            );
            Some(OperatorInstance {
                op_constraints,
                input_ports: vec![input_port],
                output_ports: vec![output_port],
                singletons_referenced: operator.singletons_referenced.clone(),
                generics,
                arguments_pre: operator.args.clone(),
                arguments_raw: operator.args_raw.clone(),
            })
        };

        // Insert new `node`.
        let node_id = self.nodes.insert(new_node);
        // Insert corresponding `OperatorInstance` if applicable.
        if let Some(op_inst) = op_inst_opt {
            self.operator_instances.insert(node_id, op_inst);
        }
        // Update edges to insert node within `edge_id`.
        let (e0, e1) = self
            .graph
            .insert_intermediate_vertex(node_id, edge_id)
            .unwrap();

        // Update corresponding ports.
        let (src_idx, dst_idx) = self.ports.remove(edge_id).unwrap();
        self.ports
            .insert(e0, (src_idx, PortIndexValue::Elided(span)));
        self.ports
            .insert(e1, (PortIndexValue::Elided(span), dst_idx));

        (node_id, e1)
    }

    /// Remove the node `node_id` but preserves and connects the single predecessor and single successor.
    /// Panics if the node does not have exactly one predecessor and one successor, or is not in the graph.
    pub fn remove_intermediate_node(&mut self, node_id: GraphNodeId) {
        assert_eq!(
            1,
            self.node_degree_in(node_id),
            "Removed intermediate node must have one predecessor"
        );
        assert_eq!(
            1,
            self.node_degree_out(node_id),
            "Removed intermediate node must have one successor"
        );
        assert!(
            self.node_subgraph.is_empty() && self.subgraph_nodes.is_empty(),
            "Should not remove intermediate node after subgraph partitioning"
        );

        assert!(self.nodes.remove(node_id).is_some());
        let (new_edge_id, (pred_edge_id, succ_edge_id)) =
            self.graph.remove_intermediate_vertex(node_id).unwrap();
        self.operator_instances.remove(node_id);
        self.node_varnames.remove(node_id);

        let (src_port, _) = self.ports.remove(pred_edge_id).unwrap();
        let (_, dst_port) = self.ports.remove(succ_edge_id).unwrap();
        self.ports.insert(new_edge_id, (src_port, dst_port));
    }

    /// Helper method: determine the "color" (pull vs push) of a node based on its in and out degree,
    /// excluding reference edges. If linear (1 in, 1 out), color is `None`, indicating it can be
    /// either push or pull.
    ///
    /// Note that this does NOT consider `DelayType` barriers (which generally implies `Pull`).
    pub(crate) fn node_color(&self, node_id: GraphNodeId) -> Option<Color> {
        if matches!(self.node(node_id), GraphNode::Handoff { .. }) {
            return Some(Color::Hoff);
        }

        // TODO(shadaj): this is a horrible hack
        if let GraphNode::Operator(op) = self.node(node_id)
            && (op.name_string() == "resolve_futures_blocking"
                || op.name_string() == "resolve_futures_blocking_ordered")
        {
            return Some(Color::Push);
        }

        // In-degree, excluding ref-edges.
        let inn_degree = self.node_predecessor_nodes(node_id).count();
        // Out-degree excluding ref-edges.
        let out_degree = self.node_successor_nodes(node_id).count();

        match (inn_degree, out_degree) {
            (0, 0) => None, // Generally should not happen, "Degenerate subgraph detected".
            (0, 1) => Some(Color::Pull),
            (1, 0) => Some(Color::Push),
            (1, 1) => None, // Linear, can be either push or pull.
            (_many, 0 | 1) => Some(Color::Pull),
            (0 | 1, _many) => Some(Color::Push),
            (_many, _to_many) => Some(Color::Comp),
        }
    }

    /// Set the operator tag (for debugging/tracing).
    pub fn set_operator_tag(&mut self, node_id: GraphNodeId, tag: String) {
        self.operator_tag.insert(node_id, tag.to_owned());
    }
}

/// Singleton references.
impl DfirGraph {
    /// Set the singletons referenced for the `node_id` operator. Each reference corresponds to the
    /// same index in the [`crate::parse::Operator::singletons_referenced`] vec.
    pub fn set_node_singleton_references(
        &mut self,
        node_id: GraphNodeId,
        singletons_referenced: Vec<Option<GraphNodeId>>,
    ) -> Option<Vec<Option<GraphNodeId>>> {
        self.node_singleton_references
            .insert(node_id, singletons_referenced)
    }

    /// Gets the singletons referenced by a node. Returns an empty iterator for non-operators and
    /// operators that do not reference singletons.
    pub fn node_singleton_references(&self, node_id: GraphNodeId) -> &[Option<GraphNodeId>] {
        self.node_singleton_references
            .get(node_id)
            .map(std::ops::Deref::deref)
            .unwrap_or_default()
    }
}

/// Module methods.
impl DfirGraph {
    /// When modules are imported into a flat graph, they come with an input and output ModuleBoundary node.
    /// The partitioner doesn't understand these nodes and will panic if it encounters them.
    /// merge_modules removes them from the graph, stitching the input and ouput sides of the ModuleBondaries based on their ports
    /// For example:
    ///     source_iter([]) -> \[myport\]ModuleBoundary(input)\[my_port\] -> map(|x| x) -> ModuleBoundary(output) -> null();
    /// in the above eaxmple, the \[myport\] port will be used to connect the source_iter with the map that is inside of the module.
    /// The output module boundary has elided ports, this is also used to match up the input/output across the module boundary.
    pub fn merge_modules(&mut self) -> Result<(), Diagnostic> {
        let mod_bound_nodes = self
            .nodes()
            .filter(|(_nid, node)| matches!(node, GraphNode::ModuleBoundary { .. }))
            .map(|(nid, _node)| nid)
            .collect::<Vec<_>>();

        for mod_bound_node in mod_bound_nodes {
            self.remove_module_boundary(mod_bound_node)?;
        }

        Ok(())
    }

    /// see `merge_modules`
    /// This function removes a singular module boundary from the graph and performs the necessary stitching to fix the graph afterward.
    /// `merge_modules` calls this function for each module boundary in the graph.
    fn remove_module_boundary(&mut self, mod_bound_node: GraphNodeId) -> Result<(), Diagnostic> {
        assert!(
            self.node_subgraph.is_empty() && self.subgraph_nodes.is_empty(),
            "Should not remove intermediate node after subgraph partitioning"
        );

        let mut mod_pred_ports = BTreeMap::new();
        let mut mod_succ_ports = BTreeMap::new();

        for mod_out_edge in self.node_predecessor_edges(mod_bound_node) {
            let (pred_port, succ_port) = self.edge_ports(mod_out_edge);
            mod_pred_ports.insert(succ_port.clone(), (mod_out_edge, pred_port.clone()));
        }

        for mod_inn_edge in self.node_successor_edges(mod_bound_node) {
            let (pred_port, succ_port) = self.edge_ports(mod_inn_edge);
            mod_succ_ports.insert(pred_port.clone(), (mod_inn_edge, succ_port.clone()));
        }

        if mod_pred_ports.keys().collect::<BTreeSet<_>>()
            != mod_succ_ports.keys().collect::<BTreeSet<_>>()
        {
            // get module boundary node
            let GraphNode::ModuleBoundary { input, import_expr } = self.node(mod_bound_node) else {
                panic!();
            };

            if *input {
                return Err(Diagnostic {
                    span: *import_expr,
                    level: Level::Error,
                    message: format!(
                        "The ports into the module did not match. input: {:?}, expected: {:?}",
                        mod_pred_ports.keys().map(|x| x.to_string()).join(", "),
                        mod_succ_ports.keys().map(|x| x.to_string()).join(", ")
                    ),
                });
            } else {
                return Err(Diagnostic {
                    span: *import_expr,
                    level: Level::Error,
                    message: format!(
                        "The ports out of the module did not match. output: {:?}, expected: {:?}",
                        mod_succ_ports.keys().map(|x| x.to_string()).join(", "),
                        mod_pred_ports.keys().map(|x| x.to_string()).join(", "),
                    ),
                });
            }
        }

        for (port, (pred_edge, pred_port)) in mod_pred_ports {
            let (succ_edge, succ_port) = mod_succ_ports.remove(&port).unwrap();

            let (src, _) = self.edge(pred_edge);
            let (_, dst) = self.edge(succ_edge);
            self.remove_edge(pred_edge);
            self.remove_edge(succ_edge);

            let new_edge_id = self.graph.insert_edge(src, dst);
            self.ports.insert(new_edge_id, (pred_port, succ_port));
        }

        self.graph.remove_vertex(mod_bound_node);
        self.nodes.remove(mod_bound_node);

        Ok(())
    }
}

/// Edge methods.
impl DfirGraph {
    /// Get the `src` and `dst` for an edge: `(src GraphNodeId, dst GraphNodeId)`.
    pub fn edge(&self, edge_id: GraphEdgeId) -> (GraphNodeId, GraphNodeId) {
        let (src, dst) = self.graph.edge(edge_id).expect("Edge not found.");
        (src, dst)
    }

    /// Get the source and destination ports for an edge: `(src &PortIndexValue, dst &PortIndexValue)`.
    pub fn edge_ports(&self, edge_id: GraphEdgeId) -> (&PortIndexValue, &PortIndexValue) {
        let (src_port, dst_port) = self.ports.get(edge_id).expect("Edge not found.");
        (src_port, dst_port)
    }

    /// Iterator of all edge IDs `GraphEdgeId`.
    pub fn edge_ids(&self) -> slotmap::basic::Keys<'_, GraphEdgeId, (GraphNodeId, GraphNodeId)> {
        self.graph.edge_ids()
    }

    /// Iterator over all edges: `(GraphEdgeId, (src GraphNodeId, dst GraphNodeId))`.
    pub fn edges(
        &self,
    ) -> impl '_
    + ExactSizeIterator<Item = (GraphEdgeId, (GraphNodeId, GraphNodeId))>
    + FusedIterator
    + Clone
    + Debug {
        self.graph.edges()
    }

    /// Insert an edge between nodes thru the given ports.
    pub fn insert_edge(
        &mut self,
        src: GraphNodeId,
        src_port: PortIndexValue,
        dst: GraphNodeId,
        dst_port: PortIndexValue,
    ) -> GraphEdgeId {
        let edge_id = self.graph.insert_edge(src, dst);
        self.ports.insert(edge_id, (src_port, dst_port));
        edge_id
    }

    /// Removes an edge and its corresponding ports and edge type info.
    pub fn remove_edge(&mut self, edge: GraphEdgeId) {
        let (_src, _dst) = self.graph.remove_edge(edge).unwrap();
        let (_src_port, _dst_port) = self.ports.remove(edge).unwrap();
    }
}

/// Subgraph methods.
impl DfirGraph {
    /// Nodes belonging to the given subgraph.
    pub fn subgraph(&self, subgraph_id: GraphSubgraphId) -> &Vec<GraphNodeId> {
        self.subgraph_nodes
            .get(subgraph_id)
            .expect("Subgraph not found.")
    }

    /// Iterator over all subgraph IDs.
    pub fn subgraph_ids(&self) -> slotmap::basic::Keys<'_, GraphSubgraphId, Vec<GraphNodeId>> {
        self.subgraph_nodes.keys()
    }

    /// Iterator over all subgraphs, ID and members: `(GraphSubgraphId, Vec<GraphNodeId>)`.
    pub fn subgraphs(&self) -> slotmap::basic::Iter<'_, GraphSubgraphId, Vec<GraphNodeId>> {
        self.subgraph_nodes.iter()
    }

    /// Create a subgraph consisting of `node_ids`. Returns an error if any of the nodes are already in a subgraph.
    pub fn insert_subgraph(
        &mut self,
        node_ids: Vec<GraphNodeId>,
    ) -> Result<GraphSubgraphId, (GraphNodeId, GraphSubgraphId)> {
        // Check none are already in subgraphs
        for &node_id in node_ids.iter() {
            if let Some(&old_sg_id) = self.node_subgraph.get(node_id) {
                return Err((node_id, old_sg_id));
            }
        }
        let subgraph_id = self.subgraph_nodes.insert_with_key(|sg_id| {
            for &node_id in node_ids.iter() {
                self.node_subgraph.insert(node_id, sg_id);
            }
            node_ids
        });

        Ok(subgraph_id)
    }

    /// Removes a node from its subgraph. Returns true if the node was in a subgraph.
    pub fn remove_from_subgraph(&mut self, node_id: GraphNodeId) -> bool {
        if let Some(old_sg_id) = self.node_subgraph.remove(node_id) {
            self.subgraph_nodes[old_sg_id].retain(|&other_node_id| other_node_id != node_id);
            true
        } else {
            false
        }
    }

    /// Gets the stratum number of the subgraph.
    pub fn subgraph_stratum(&self, sg_id: GraphSubgraphId) -> Option<usize> {
        self.subgraph_stratum.get(sg_id).copied()
    }

    /// Set subgraph's stratum number, returning the old value if exists.
    pub fn set_subgraph_stratum(
        &mut self,
        sg_id: GraphSubgraphId,
        stratum: usize,
    ) -> Option<usize> {
        self.subgraph_stratum.insert(sg_id, stratum)
    }

    /// Gets whether the subgraph is lazy or not
    fn subgraph_laziness(&self, sg_id: GraphSubgraphId) -> bool {
        self.subgraph_laziness.get(sg_id).copied().unwrap_or(false)
    }

    /// Set subgraph's laziness, returning the old value.
    pub fn set_subgraph_laziness(&mut self, sg_id: GraphSubgraphId, lazy: bool) -> bool {
        self.subgraph_laziness.insert(sg_id, lazy).unwrap_or(false)
    }

    /// Returns the the stratum number of the largest (latest) stratum (inclusive).
    pub fn max_stratum(&self) -> Option<usize> {
        self.subgraph_stratum.values().copied().max()
    }

    /// Helper: finds the first index in `subgraph_nodes` where it transitions from pull to push.
    fn find_pull_to_push_idx(&self, subgraph_nodes: &[GraphNodeId]) -> usize {
        subgraph_nodes
            .iter()
            .position(|&node_id| {
                self.node_color(node_id)
                    .is_some_and(|color| Color::Pull != color)
            })
            .unwrap_or(subgraph_nodes.len())
    }
}

/// Display/output methods.
impl DfirGraph {
    /// Helper to generate a deterministic `Ident` for the given node.
    fn node_as_ident(&self, node_id: GraphNodeId, is_pred: bool) -> Ident {
        let name = match &self.nodes[node_id] {
            GraphNode::Operator(_) => format!("op_{:?}", node_id.data()),
            GraphNode::Handoff { .. } => format!(
                "hoff_{:?}_{}",
                node_id.data(),
                if is_pred { "recv" } else { "send" }
            ),
            GraphNode::ModuleBoundary { .. } => panic!(),
        };
        let span = match (is_pred, &self.nodes[node_id]) {
            (_, GraphNode::Operator(operator)) => operator.span(),
            (true, &GraphNode::Handoff { src_span, .. }) => src_span,
            (false, &GraphNode::Handoff { dst_span, .. }) => dst_span,
            (_, GraphNode::ModuleBoundary { .. }) => panic!(),
        };
        Ident::new(&name, span)
    }

    /// For per-node singleton references. Helper to generate a deterministic `Ident` for the given node.
    fn node_as_singleton_ident(&self, node_id: GraphNodeId, span: Span) -> Ident {
        Ident::new(&format!("singleton_op_{:?}", node_id.data()), span)
    }

    /// Resolve the singletons via [`Self::node_singleton_references`] for the given `node_id`.
    fn helper_resolve_singletons(&self, node_id: GraphNodeId, span: Span) -> Vec<Ident> {
        self.node_singleton_references(node_id)
            .iter()
            .map(|singleton_node_id| {
                // TODO(mingwei): this `expect` should be caught in error checking
                self.node_as_singleton_ident(
                    singleton_node_id
                        .expect("Expected singleton to be resolved but was not, this is a bug."),
                    span,
                )
            })
            .collect::<Vec<_>>()
    }

    /// Returns each subgraph's receive and send handoffs.
    /// `Map<GraphSubgraphId, (recv handoffs, send handoffs)>`
    fn helper_collect_subgraph_handoffs(
        &self,
    ) -> SecondaryMap<GraphSubgraphId, (Vec<GraphNodeId>, Vec<GraphNodeId>)> {
        // Get data on handoff src and dst subgraphs.
        let mut subgraph_handoffs: SecondaryMap<
            GraphSubgraphId,
            (Vec<GraphNodeId>, Vec<GraphNodeId>),
        > = self
            .subgraph_nodes
            .keys()
            .map(|k| (k, Default::default()))
            .collect();

        // For each handoff node, add it to the `send`/`recv` lists for the corresponding subgraphs.
        for (hoff_id, node) in self.nodes() {
            if !matches!(node, GraphNode::Handoff { .. }) {
                continue;
            }
            // Receivers from the handoff. (Should really only be one).
            for (_edge, succ_id) in self.node_successors(hoff_id) {
                let succ_sg = self.node_subgraph(succ_id).unwrap();
                subgraph_handoffs[succ_sg].0.push(hoff_id);
            }
            // Senders into the handoff. (Should really only be one).
            for (_edge, pred_id) in self.node_predecessors(hoff_id) {
                let pred_sg = self.node_subgraph(pred_id).unwrap();
                subgraph_handoffs[pred_sg].1.push(hoff_id);
            }
        }

        subgraph_handoffs
    }

    /// Code for adding all nested loops.
    fn codegen_nested_loops(&self, df: &Ident) -> TokenStream {
        // Breadth-first iteration from outermost (root) loops to deepest nested loops.
        let mut out = TokenStream::new();
        let mut queue = VecDeque::from_iter(self.root_loops.iter().copied());
        while let Some(loop_id) = queue.pop_front() {
            let parent_opt = self
                .loop_parent(loop_id)
                .map(|loop_id| loop_id.as_ident(Span::call_site()))
                .map(|ident| quote! { Some(#ident) })
                .unwrap_or_else(|| quote! { None });
            let loop_name = loop_id.as_ident(Span::call_site());
            out.append_all(quote! {
                let #loop_name = #df.add_loop(#parent_opt);
            });
            queue.extend(self.loop_children.get(loop_id).into_iter().flatten());
        }
        out
    }

    /// Emit this graph as runnable Rust source code tokens.
    pub fn as_code(
        &self,
        root: &TokenStream,
        include_type_guards: bool,
        prefix: TokenStream,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> TokenStream {
        let df = Ident::new(GRAPH, Span::call_site());
        let context = Ident::new(CONTEXT, Span::call_site());

        // Code for adding handoffs.
        let handoff_code = self
            .nodes
            .iter()
            .filter_map(|(node_id, node)| match node {
                GraphNode::Operator(_) => None,
                &GraphNode::Handoff { src_span, dst_span } => Some((node_id, (src_span, dst_span))),
                GraphNode::ModuleBoundary { .. } => panic!(),
            })
            .map(|(node_id, (src_span, dst_span))| {
                let ident_send = Ident::new(&format!("hoff_{:?}_send", node_id.data()), dst_span);
                let ident_recv = Ident::new(&format!("hoff_{:?}_recv", node_id.data()), src_span);
                let span = src_span.join(dst_span).unwrap_or(src_span);
                let mut hoff_name = Literal::string(&format!("handoff {:?}", node_id));
                hoff_name.set_span(span);
                let hoff_type = quote_spanned! (span=> #root::scheduled::handoff::VecHandoff<_>);
                quote_spanned! {span=>
                    let (#ident_send, #ident_recv) =
                        #df.make_edge::<_, #hoff_type>(#hoff_name);
                }
            });

        let subgraph_handoffs = self.helper_collect_subgraph_handoffs();

        // we first generate the subgraphs that have no inputs to guide type inference
        let (subgraphs_without_preds, subgraphs_with_preds) = self
            .subgraph_nodes
            .iter()
            .partition::<Vec<_>, _>(|(_, nodes)| {
                nodes
                    .iter()
                    .any(|&node_id| self.node_degree_in(node_id) == 0)
            });

        let mut op_prologue_code = Vec::new();
        let mut op_prologue_after_code = Vec::new();
        let mut subgraphs = Vec::new();
        {
            for &(subgraph_id, subgraph_nodes) in subgraphs_without_preds
                .iter()
                .chain(subgraphs_with_preds.iter())
            {
                let (recv_hoffs, send_hoffs) = &subgraph_handoffs[subgraph_id];
                let recv_ports: Vec<Ident> = recv_hoffs
                    .iter()
                    .map(|&hoff_id| self.node_as_ident(hoff_id, true))
                    .collect();
                let send_ports: Vec<Ident> = send_hoffs
                    .iter()
                    .map(|&hoff_id| self.node_as_ident(hoff_id, false))
                    .collect();

                let recv_port_code = recv_ports.iter().map(|ident| {
                    quote_spanned! {ident.span()=>
                        let mut #ident = #ident.borrow_mut_swap();
                        let #ident = #ident.drain(..);
                    }
                });
                let send_port_code = send_ports.iter().map(|ident| {
                    quote_spanned! {ident.span()=>
                        let #ident = #root::sinktools::for_each(|v| {
                            #ident.give(Some(v));
                        });
                    }
                });

                let loop_id = self
                    // All nodes in a subgraph should be in the same loop.
                    .node_loop(subgraph_nodes[0]);

                let mut subgraph_op_iter_code = Vec::new();
                let mut subgraph_op_iter_after_code = Vec::new();
                {
                    let pull_to_push_idx = self.find_pull_to_push_idx(subgraph_nodes);

                    let (pull_half, push_half) = subgraph_nodes.split_at(pull_to_push_idx);
                    let nodes_iter = pull_half.iter().chain(push_half.iter().rev());

                    for (idx, &node_id) in nodes_iter.enumerate() {
                        let node = &self.nodes[node_id];
                        assert!(
                            matches!(node, GraphNode::Operator(_)),
                            "Handoffs are not part of subgraphs."
                        );
                        let op_inst = &self.operator_instances[node_id];

                        let op_span = node.span();
                        let op_name = op_inst.op_constraints.name;
                        // Use op's span for root. #root is expected to be correct, any errors should span back to the op gen.
                        let root = change_spans(root.clone(), op_span);
                        // TODO(mingwei): Just use `op_inst.op_constraints`?
                        let op_constraints = OPERATORS
                            .iter()
                            .find(|op| op_name == op.name)
                            .unwrap_or_else(|| panic!("Failed to find op: {}", op_name));

                        let ident = self.node_as_ident(node_id, false);

                        {
                            // TODO clean this up.
                            // Collect input arguments (predecessors).
                            let mut input_edges = self
                                .graph
                                .predecessor_edges(node_id)
                                .map(|edge_id| (self.edge_ports(edge_id).1, edge_id))
                                .collect::<Vec<_>>();
                            // Ensure sorted by port index.
                            input_edges.sort();

                            let inputs = input_edges
                                .iter()
                                .map(|&(_port, edge_id)| {
                                    let (pred, _) = self.edge(edge_id);
                                    self.node_as_ident(pred, true)
                                })
                                .collect::<Vec<_>>();

                            // Collect output arguments (successors).
                            let mut output_edges = self
                                .graph
                                .successor_edges(node_id)
                                .map(|edge_id| (&self.ports[edge_id].0, edge_id))
                                .collect::<Vec<_>>();
                            // Ensure sorted by port index.
                            output_edges.sort();

                            let outputs = output_edges
                                .iter()
                                .map(|&(_port, edge_id)| {
                                    let (_, succ) = self.edge(edge_id);
                                    self.node_as_ident(succ, false)
                                })
                                .collect::<Vec<_>>();

                            let is_pull = idx < pull_to_push_idx;

                            let singleton_output_ident = &if op_constraints.has_singleton_output {
                                self.node_as_singleton_ident(node_id, op_span)
                            } else {
                                // This ident *should* go unused.
                                Ident::new(&format!("{}_has_no_singleton_output", op_name), op_span)
                            };

                            // There's a bit of dark magic hidden in `Span`s... you'd think it's just a `file:line:column`,
                            // but it has one extra bit of info for _name resolution_, used for `Ident`s. `Span::call_site()`
                            // has the (unhygienic) resolution we want, an ident is just solely determined by its string name,
                            // which is what you'd expect out of unhygienic proc macros like this. Meanwhile, declarative macros
                            // use `Span::mixed_site()` which is weird and I don't understand it. It turns out that if you call
                            // the dfir syntax proc macro from _within_ a declarative macro then `op_span` will have the
                            // bad `Span::mixed_site()` name resolution and cause "Cannot find value `df/context`" errors. So
                            // we call `.resolved_at()` to fix resolution back to `Span::call_site()`. -Mingwei
                            let df_local = &Ident::new(GRAPH, op_span.resolved_at(df.span()));
                            let context = &Ident::new(CONTEXT, op_span.resolved_at(context.span()));

                            let singletons_resolved =
                                self.helper_resolve_singletons(node_id, op_span);
                            let arguments = &process_singletons::postprocess_singletons(
                                op_inst.arguments_raw.clone(),
                                singletons_resolved.clone(),
                                context,
                            );
                            let arguments_handles =
                                &process_singletons::postprocess_singletons_handles(
                                    op_inst.arguments_raw.clone(),
                                    singletons_resolved.clone(),
                                );

                            let source_tag = 'a: {
                                if let Some(tag) = self.operator_tag.get(node_id).cloned() {
                                    break 'a tag;
                                }

                                #[cfg(nightly)]
                                if proc_macro::is_available() {
                                    let op_span = op_span.unwrap();
                                    break 'a format!(
                                        "loc_{}_{}_{}_{}_{}",
                                        crate::pretty_span::make_source_path_relative(
                                            &op_span.file()
                                        )
                                        .display()
                                        .to_string()
                                        .replace(|x: char| !x.is_ascii_alphanumeric(), "_"),
                                        op_span.start().line(),
                                        op_span.start().column(),
                                        op_span.end().line(),
                                        op_span.end().column(),
                                    );
                                }

                                format!(
                                    "loc_nopath_{}_{}_{}_{}",
                                    op_span.start().line,
                                    op_span.start().column,
                                    op_span.end().line,
                                    op_span.end().column
                                )
                            };

                            let work_fn = format_ident!(
                                "{}__{}__{}",
                                ident,
                                op_name,
                                source_tag,
                                span = op_span
                            );

                            let context_args = WriteContextArgs {
                                root: &root,
                                df_ident: df_local,
                                context,
                                subgraph_id,
                                node_id,
                                loop_id,
                                op_span,
                                op_tag: self.operator_tag.get(node_id).cloned(),
                                work_fn: &work_fn,
                                ident: &ident,
                                is_pull,
                                inputs: &inputs,
                                outputs: &outputs,
                                singleton_output_ident,
                                op_name,
                                op_inst,
                                arguments,
                                arguments_handles,
                            };

                            let write_result =
                                (op_constraints.write_fn)(&context_args, diagnostics);
                            let OperatorWriteOutput {
                                write_prologue,
                                write_prologue_after,
                                write_iterator,
                                write_iterator_after,
                            } = write_result.unwrap_or_else(|()| {
                                assert!(
                                    diagnostics.iter().any(Diagnostic::is_error),
                                    "Operator `{}` returned `Err` but emitted no diagnostics, this is a bug.",
                                    op_name,
                                );
                                OperatorWriteOutput { write_iterator: null_write_iterator_fn(&context_args), ..Default::default() }
                            });

                            op_prologue_code.push(syn::parse_quote! {
                                #[allow(non_snake_case)]
                                #[inline(always)]
                                fn #work_fn<T>(thunk: impl FnOnce() -> T) -> T {
                                    thunk()
                                }
                            });
                            op_prologue_code.push(write_prologue);
                            op_prologue_after_code.push(write_prologue_after);
                            subgraph_op_iter_code.push(write_iterator);

                            if include_type_guards {
                                let type_guard = if is_pull {
                                    quote_spanned! {op_span=>
                                        let #ident = {
                                            #[allow(non_snake_case)]
                                            #[inline(always)]
                                            pub fn #work_fn<Item, Input: ::std::iter::Iterator<Item = Item>>(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
                                                #[repr(transparent)]
                                                struct Pull<Item, Input: ::std::iter::Iterator<Item = Item>> {
                                                    inner: Input
                                                }

                                                impl<Item, Input: ::std::iter::Iterator<Item = Item>> Iterator for Pull<Item, Input> {
                                                    type Item = Item;

                                                    #[inline(always)]
                                                    fn next(&mut self) -> Option<Self::Item> {
                                                        self.inner.next()
                                                    }

                                                    #[inline(always)]
                                                    fn size_hint(&self) -> (usize, Option<usize>) {
                                                        self.inner.size_hint()
                                                    }
                                                }

                                                Pull {
                                                    inner: input
                                                }
                                            }
                                            #work_fn( #ident )
                                        };
                                    }
                                } else {
                                    quote_spanned! {op_span=>
                                        let #ident = {
                                            #[allow(non_snake_case)]
                                            #[inline(always)]
                                            pub fn #work_fn<Item, Si>(si: Si) -> impl #root::futures::sink::Sink<Item, Error = #root::Never>
                                            where
                                                Si: #root::futures::sink::Sink<Item, Error = #root::Never>
                                            {
                                                #root::pin_project_lite::pin_project! {
                                                    #[repr(transparent)]
                                                    struct Push<Si> {
                                                        #[pin]
                                                        si: Si,
                                                    }
                                                }
                                                impl<Item, Si> #root::futures::sink::Sink<Item> for Push<Si>
                                                where
                                                    Si: #root::futures::sink::Sink<Item>,
                                                {
                                                    type Error = Si::Error;

                                                    fn poll_ready(
                                                        self: ::std::pin::Pin<&mut Self>,
                                                        cx: &mut ::std::task::Context<'_>,
                                                    ) -> ::std::task::Poll<::std::result::Result<(), Self::Error>> {
                                                        self.project().si.poll_ready(cx)
                                                    }

                                                    fn start_send(
                                                        self: ::std::pin::Pin<&mut Self>,
                                                        item: Item,
                                                    ) -> ::std::result::Result<(), Self::Error> {
                                                        self.project().si.start_send(item)
                                                    }

                                                    fn poll_flush(
                                                        self: ::std::pin::Pin<&mut Self>,
                                                        cx: &mut ::std::task::Context<'_>,
                                                    ) -> ::std::task::Poll<::std::result::Result<(), Self::Error>> {
                                                        self.project().si.poll_flush(cx)
                                                    }

                                                    fn poll_close(
                                                        self: ::std::pin::Pin<&mut Self>,
                                                        cx: &mut ::std::task::Context<'_>,
                                                    ) -> ::std::task::Poll<::std::result::Result<(), Self::Error>> {
                                                        self.project().si.poll_close(cx)
                                                    }
                                                }

                                                Push {
                                                    si
                                                }
                                            }
                                            #work_fn( #ident )
                                        };
                                    }
                                };
                                subgraph_op_iter_code.push(type_guard);
                            }
                            subgraph_op_iter_after_code.push(write_iterator_after);
                        }
                    }

                    {
                        // Determine pull and push halves of the `Pivot`.
                        let pull_ident = if 0 < pull_to_push_idx {
                            self.node_as_ident(subgraph_nodes[pull_to_push_idx - 1], false)
                        } else {
                            // Entire subgraph is push (with a single recv/pull handoff input).
                            recv_ports[0].clone()
                        };

                        #[rustfmt::skip]
                        let push_ident = if let Some(&node_id) =
                            subgraph_nodes.get(pull_to_push_idx)
                        {
                            self.node_as_ident(node_id, false)
                        } else if 1 == send_ports.len() {
                            // Entire subgraph is pull (with a single send/push handoff output).
                            send_ports[0].clone()
                        } else {
                            diagnostics.push(Diagnostic::spanned(
                                pull_ident.span(),
                                Level::Error,
                                "Degenerate subgraph detected, is there a disconnected `null()` or other degenerate pipeline somewhere?",
                            ));
                            continue;
                        };

                        // Pivot span is combination of pull and push spans (or if not possible, just take the push).
                        let pivot_span = pull_ident
                            .span()
                            .join(push_ident.span())
                            .unwrap_or_else(|| push_ident.span());
                        let pivot_fn_ident =
                            Ident::new(&format!("pivot_run_sg_{:?}", subgraph_id.0), pivot_span);
                        let root = change_spans(root.clone(), pivot_span);
                        subgraph_op_iter_code.push(quote_spanned! {pivot_span=>
                            #[inline(always)]
                            fn #pivot_fn_ident<Pull, Push, Item>(pull: Pull, push: Push)
                                -> impl ::std::future::Future<Output = ::std::result::Result<(), #root::Never>>
                            where
                                Pull: ::std::iter::Iterator<Item = Item>,
                                Push: #root::futures::sink::Sink<Item, Error = #root::Never>,
                            {
                                #root::sinktools::send_iter(pull, push)
                            }
                            (#pivot_fn_ident)(#pull_ident, #push_ident).await.unwrap(/* Never */);
                        });
                    }
                };

                let subgraph_name = Literal::string(&format!("Subgraph {:?}", subgraph_id));
                let stratum = Literal::usize_unsuffixed(
                    self.subgraph_stratum.get(subgraph_id).cloned().unwrap_or(0),
                );
                let laziness = self.subgraph_laziness(subgraph_id);

                // Codegen: the loop that this subgraph is in `Some(<loop_id>)`, or `None` if not in a loop.
                let loop_id_opt = loop_id
                    .map(|loop_id| loop_id.as_ident(Span::call_site()))
                    .map(|ident| quote! { Some(#ident) })
                    .unwrap_or_else(|| quote! { None });

                let sg_ident = subgraph_id.as_ident(Span::call_site());

                subgraphs.push(quote! {
                    let #sg_ident = #df.add_subgraph_full(
                        #subgraph_name,
                        #stratum,
                        var_expr!( #( #recv_ports ),* ),
                        var_expr!( #( #send_ports ),* ),
                        #laziness,
                        #loop_id_opt,
                        async move |#context, var_args!( #( #recv_ports ),* ), var_args!( #( #send_ports ),* )| {
                            #( #recv_port_code )*
                            #( #send_port_code )*
                            #( #subgraph_op_iter_code )*
                            #( #subgraph_op_iter_after_code )*
                        },
                    );
                });
            }
        }

        let loop_code = self.codegen_nested_loops(&df);

        // These two are quoted separately here because iterators are lazily evaluated, so this
        // forces them to do their work. This work includes populating some data, namely
        // `diagonstics`, which we need to determine if it compilation was actually successful.
        // -Mingwei
        let code = quote! {
            #( #handoff_code )*
            #loop_code
            #( #op_prologue_code )*
            #( #subgraphs )*
            #( #op_prologue_after_code )*
        };

        let meta_graph_json = serde_json::to_string(&self).unwrap();
        let meta_graph_json = Literal::string(&meta_graph_json);

        let serde_diagnostics: Vec<_> = diagnostics.iter().map(Diagnostic::to_serde).collect();
        let diagnostics_json = serde_json::to_string(&*serde_diagnostics).unwrap();
        let diagnostics_json = Literal::string(&diagnostics_json);

        quote! {
            {
                #[allow(unused_qualifications, clippy::await_holding_refcell_ref)]
                {
                    #prefix

                    use #root::{var_expr, var_args};

                    let mut #df = #root::scheduled::graph::Dfir::new();
                    #df.__assign_meta_graph(#meta_graph_json);
                    #df.__assign_diagnostics(#diagnostics_json);

                    #code

                    #df
                }
            }
        }
    }

    /// Color mode (pull vs. push, handoff vs. comp) for nodes. Some nodes can be push *OR* pull;
    /// those nodes will not be set in the returned map.
    pub fn node_color_map(&self) -> SparseSecondaryMap<GraphNodeId, Color> {
        let mut node_color_map: SparseSecondaryMap<GraphNodeId, Color> = self
            .node_ids()
            .filter_map(|node_id| {
                let op_color = self.node_color(node_id)?;
                Some((node_id, op_color))
            })
            .collect();

        // Fill in rest via subgraphs.
        for sg_nodes in self.subgraph_nodes.values() {
            let pull_to_push_idx = self.find_pull_to_push_idx(sg_nodes);

            for (idx, node_id) in sg_nodes.iter().copied().enumerate() {
                let is_pull = idx < pull_to_push_idx;
                node_color_map.insert(node_id, if is_pull { Color::Pull } else { Color::Push });
            }
        }

        node_color_map
    }

    /// Writes this graph as mermaid into a string.
    pub fn to_mermaid(&self, write_config: &WriteConfig) -> String {
        let mut output = String::new();
        self.write_mermaid(&mut output, write_config).unwrap();
        output
    }

    /// Writes this graph as mermaid into the given `Write`.
    pub fn write_mermaid(
        &self,
        output: impl std::fmt::Write,
        write_config: &WriteConfig,
    ) -> std::fmt::Result {
        let mut graph_write = Mermaid::new(output);
        self.write_graph(&mut graph_write, write_config)
    }

    /// Writes this graph as DOT (graphviz) into a string.
    pub fn to_dot(&self, write_config: &WriteConfig) -> String {
        let mut output = String::new();
        let mut graph_write = Dot::new(&mut output);
        self.write_graph(&mut graph_write, write_config).unwrap();
        output
    }

    /// Writes this graph as DOT (graphviz) into the given `Write`.
    pub fn write_dot(
        &self,
        output: impl std::fmt::Write,
        write_config: &WriteConfig,
    ) -> std::fmt::Result {
        let mut graph_write = Dot::new(output);
        self.write_graph(&mut graph_write, write_config)
    }

    /// Write out this graph using the given `GraphWrite`. E.g. `Mermaid` or `Dot.
    pub(crate) fn write_graph<W>(
        &self,
        mut graph_write: W,
        write_config: &WriteConfig,
    ) -> Result<(), W::Err>
    where
        W: GraphWrite,
    {
        fn helper_edge_label(
            src_port: &PortIndexValue,
            dst_port: &PortIndexValue,
        ) -> Option<String> {
            let src_label = match src_port {
                PortIndexValue::Path(path) => Some(path.to_token_stream().to_string()),
                PortIndexValue::Int(index) => Some(index.value.to_string()),
                _ => None,
            };
            let dst_label = match dst_port {
                PortIndexValue::Path(path) => Some(path.to_token_stream().to_string()),
                PortIndexValue::Int(index) => Some(index.value.to_string()),
                _ => None,
            };
            let label = match (src_label, dst_label) {
                (Some(l1), Some(l2)) => Some(format!("{}\n{}", l1, l2)),
                (Some(l1), None) => Some(l1),
                (None, Some(l2)) => Some(l2),
                (None, None) => None,
            };
            label
        }

        // Make node color map one time.
        let node_color_map = self.node_color_map();

        // Write prologue.
        graph_write.write_prologue()?;

        // Define nodes.
        let mut skipped_handoffs = BTreeSet::new();
        let mut subgraph_handoffs = <BTreeMap<GraphSubgraphId, Vec<GraphNodeId>>>::new();
        for (node_id, node) in self.nodes() {
            if matches!(node, GraphNode::Handoff { .. }) {
                if write_config.no_handoffs {
                    skipped_handoffs.insert(node_id);
                    continue;
                } else {
                    let pred_node = self.node_predecessor_nodes(node_id).next().unwrap();
                    let pred_sg = self.node_subgraph(pred_node);
                    let succ_node = self.node_successor_nodes(node_id).next().unwrap();
                    let succ_sg = self.node_subgraph(succ_node);
                    if let Some((pred_sg, succ_sg)) = pred_sg.zip(succ_sg)
                        && pred_sg == succ_sg
                    {
                        subgraph_handoffs.entry(pred_sg).or_default().push(node_id);
                    }
                }
            }
            graph_write.write_node_definition(
                node_id,
                &if write_config.op_short_text {
                    node.to_name_string()
                } else if write_config.op_text_no_imports {
                    // Remove any lines that start with "use" (imports)
                    let full_text = node.to_pretty_string();
                    let mut output = String::new();
                    for sentence in full_text.split('\n') {
                        if sentence.trim().starts_with("use") {
                            continue;
                        }
                        output.push('\n');
                        output.push_str(sentence);
                    }
                    output.into()
                } else {
                    node.to_pretty_string()
                },
                if write_config.no_pull_push {
                    None
                } else {
                    node_color_map.get(node_id).copied()
                },
            )?;
        }

        // Write edges.
        for (edge_id, (src_id, mut dst_id)) in self.edges() {
            // Handling for if `write_config.no_handoffs` true.
            if skipped_handoffs.contains(&src_id) {
                continue;
            }

            let (src_port, mut dst_port) = self.edge_ports(edge_id);
            if skipped_handoffs.contains(&dst_id) {
                let mut handoff_succs = self.node_successors(dst_id);
                assert_eq!(1, handoff_succs.len());
                let (succ_edge, succ_node) = handoff_succs.next().unwrap();
                dst_id = succ_node;
                dst_port = self.edge_ports(succ_edge).1;
            }

            let label = helper_edge_label(src_port, dst_port);
            let delay_type = self
                .node_op_inst(dst_id)
                .and_then(|op_inst| (op_inst.op_constraints.input_delaytype_fn)(dst_port));
            graph_write.write_edge(src_id, dst_id, delay_type, label.as_deref(), false)?;
        }

        // Write reference edges.
        if !write_config.no_references {
            for dst_id in self.node_ids() {
                for src_ref_id in self
                    .node_singleton_references(dst_id)
                    .iter()
                    .copied()
                    .flatten()
                {
                    let delay_type = Some(DelayType::Stratum);
                    let label = None;
                    graph_write.write_edge(src_ref_id, dst_id, delay_type, label, true)?;
                }
            }
        }

        // The following code is a little bit tricky. Generally, the graph has the hierarchy:
        // `loop -> subgraph -> varname -> node`. However, each of these can be disabled via the `write_config`. To
        // handle both the enabled and disabled case, this code is structured as a series of nested loops. If the layer
        // is disabled, then the HashMap<Option<KEY>, Vec<VALUE>> will only have a single key (`None`) with a
        // corresponding `Vec` value containing everything. This way no special handling is needed for the next layer.
        //
        // (Note: `stratum` could also be included in this hierarchy, but it is being phased-out/deprecated in favor of
        // Flo loops).

        // Loop -> Subgraphs
        let loop_subgraphs = self.subgraph_ids().map(|sg_id| {
            let loop_id = if write_config.no_loops {
                None
            } else {
                self.subgraph_loop(sg_id)
            };
            (loop_id, sg_id)
        });
        let loop_subgraphs = into_group_map(loop_subgraphs);
        for (loop_id, subgraph_ids) in loop_subgraphs {
            if let Some(loop_id) = loop_id {
                graph_write.write_loop_start(loop_id)?;
            }

            // Subgraph -> Varnames.
            let subgraph_varnames_nodes = subgraph_ids.into_iter().flat_map(|sg_id| {
                self.subgraph(sg_id).iter().copied().map(move |node_id| {
                    let opt_sg_id = if write_config.no_subgraphs {
                        None
                    } else {
                        Some(sg_id)
                    };
                    (opt_sg_id, (self.node_varname(node_id), node_id))
                })
            });
            let subgraph_varnames_nodes = into_group_map(subgraph_varnames_nodes);
            for (sg_id, varnames) in subgraph_varnames_nodes {
                if let Some(sg_id) = sg_id {
                    let stratum = self.subgraph_stratum(sg_id).unwrap();
                    graph_write.write_subgraph_start(sg_id, stratum)?;
                }

                // Varnames -> Nodes.
                let varname_nodes = varnames.into_iter().map(|(varname, node)| {
                    let varname = if write_config.no_varnames {
                        None
                    } else {
                        varname
                    };
                    (varname, node)
                });
                let varname_nodes = into_group_map(varname_nodes);
                for (varname, node_ids) in varname_nodes {
                    if let Some(varname) = varname {
                        graph_write.write_varname_start(&varname.0.to_string(), sg_id)?;
                    }

                    // Write all nodes.
                    for node_id in node_ids {
                        graph_write.write_node(node_id)?;
                    }

                    if varname.is_some() {
                        graph_write.write_varname_end()?;
                    }
                }

                if sg_id.is_some() {
                    graph_write.write_subgraph_end()?;
                }
            }

            if loop_id.is_some() {
                graph_write.write_loop_end()?;
            }
        }

        // Write epilogue.
        graph_write.write_epilogue()?;

        Ok(())
    }

    /// Convert back into surface syntax.
    pub fn surface_syntax_string(&self) -> String {
        let mut string = String::new();
        self.write_surface_syntax(&mut string).unwrap();
        string
    }

    /// Convert back into surface syntax.
    pub fn write_surface_syntax(&self, write: &mut impl std::fmt::Write) -> std::fmt::Result {
        for (key, node) in self.nodes.iter() {
            match node {
                GraphNode::Operator(op) => {
                    writeln!(write, "{:?} = {};", key.data(), op.to_token_stream())?;
                }
                GraphNode::Handoff { .. } => {
                    writeln!(write, "// {:?} = <handoff>;", key.data())?;
                }
                GraphNode::ModuleBoundary { .. } => panic!(),
            }
        }
        writeln!(write)?;
        for (_e, (src_key, dst_key)) in self.graph.edges() {
            writeln!(write, "{:?} -> {:?};", src_key.data(), dst_key.data())?;
        }
        Ok(())
    }

    /// Convert into a [mermaid](https://mermaid-js.github.io/) graph. Ignores subgraphs.
    pub fn mermaid_string_flat(&self) -> String {
        let mut string = String::new();
        self.write_mermaid_flat(&mut string).unwrap();
        string
    }

    /// Convert into a [mermaid](https://mermaid-js.github.io/) graph. Ignores subgraphs.
    pub fn write_mermaid_flat(&self, write: &mut impl std::fmt::Write) -> std::fmt::Result {
        writeln!(write, "flowchart TB")?;
        for (key, node) in self.nodes.iter() {
            match node {
                GraphNode::Operator(operator) => writeln!(
                    write,
                    "    %% {span}\n    {id:?}[\"{row_col} <tt>{code}</tt>\"]",
                    span = PrettySpan(node.span()),
                    id = key.data(),
                    row_col = PrettyRowCol(node.span()),
                    code = operator
                        .to_token_stream()
                        .to_string()
                        .replace('&', "&amp;")
                        .replace('<', "&lt;")
                        .replace('>', "&gt;")
                        .replace('"', "&quot;")
                        .replace('\n', "<br>"),
                ),
                GraphNode::Handoff { .. } => {
                    writeln!(write, r#"    {:?}{{"{}"}}"#, key.data(), HANDOFF_NODE_STR)
                }
                GraphNode::ModuleBoundary { .. } => {
                    writeln!(
                        write,
                        r#"    {:?}{{"{}"}}"#,
                        key.data(),
                        MODULE_BOUNDARY_NODE_STR
                    )
                }
            }?;
        }
        writeln!(write)?;
        for (_e, (src_key, dst_key)) in self.graph.edges() {
            writeln!(write, "    {:?}-->{:?}", src_key.data(), dst_key.data())?;
        }
        Ok(())
    }
}

/// Loops
impl DfirGraph {
    /// Iterator over all loop IDs.
    pub fn loop_ids(&self) -> slotmap::basic::Keys<'_, GraphLoopId, Vec<GraphNodeId>> {
        self.loop_nodes.keys()
    }

    /// Iterator over all loops, ID and members: `(GraphLoopId, Vec<GraphNodeId>)`.
    pub fn loops(&self) -> slotmap::basic::Iter<'_, GraphLoopId, Vec<GraphNodeId>> {
        self.loop_nodes.iter()
    }

    /// Create a new loop context, with the given parent loop (or `None`).
    pub fn insert_loop(&mut self, parent_loop: Option<GraphLoopId>) -> GraphLoopId {
        let loop_id = self.loop_nodes.insert(Vec::new());
        self.loop_children.insert(loop_id, Vec::new());
        if let Some(parent_loop) = parent_loop {
            self.loop_parent.insert(loop_id, parent_loop);
            self.loop_children
                .get_mut(parent_loop)
                .unwrap()
                .push(loop_id);
        } else {
            self.root_loops.push(loop_id);
        }
        loop_id
    }

    /// Get a node's loop context (or `None` for root).
    pub fn node_loop(&self, node_id: GraphNodeId) -> Option<GraphLoopId> {
        self.node_loops.get(node_id).copied()
    }

    /// Get a subgraph's loop context (or `None` for root).
    pub fn subgraph_loop(&self, subgraph_id: GraphSubgraphId) -> Option<GraphLoopId> {
        let &node_id = self.subgraph(subgraph_id).first().unwrap();
        let out = self.node_loop(node_id);
        debug_assert!(
            self.subgraph(subgraph_id)
                .iter()
                .all(|&node_id| self.node_loop(node_id) == out),
            "Subgraph nodes should all have the same loop context."
        );
        out
    }

    /// Get a loop context's parent loop context (or `None` for root).
    pub fn loop_parent(&self, loop_id: GraphLoopId) -> Option<GraphLoopId> {
        self.loop_parent.get(loop_id).copied()
    }

    /// Get a loop context's child loops.
    pub fn loop_children(&self, loop_id: GraphLoopId) -> &Vec<GraphLoopId> {
        self.loop_children.get(loop_id).unwrap()
    }
}

/// Configuration for writing graphs.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "clap-derive", derive(clap::Args))]
pub struct WriteConfig {
    /// Subgraphs will not be rendered if set.
    #[cfg_attr(feature = "clap-derive", arg(long))]
    pub no_subgraphs: bool,
    /// Variable names will not be rendered if set.
    #[cfg_attr(feature = "clap-derive", arg(long))]
    pub no_varnames: bool,
    /// Will not render pull/push shapes if set.
    #[cfg_attr(feature = "clap-derive", arg(long))]
    pub no_pull_push: bool,
    /// Will not render handoffs if set.
    #[cfg_attr(feature = "clap-derive", arg(long))]
    pub no_handoffs: bool,
    /// Will not render singleton references if set.
    #[cfg_attr(feature = "clap-derive", arg(long))]
    pub no_references: bool,
    /// Will not render loops if set.
    #[cfg_attr(feature = "clap-derive", arg(long))]
    pub no_loops: bool,

    /// Op text will only be their name instead of the whole source.
    #[cfg_attr(feature = "clap-derive", arg(long))]
    pub op_short_text: bool,
    /// Op text will exclude any line that starts with "use".
    #[cfg_attr(feature = "clap-derive", arg(long))]
    pub op_text_no_imports: bool,
}

/// Enum for choosing between mermaid and dot graph writing.
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "clap-derive", derive(clap::Parser, clap::ValueEnum))]
pub enum WriteGraphType {
    /// Mermaid graphs.
    Mermaid,
    /// Dot (Graphviz) graphs.
    Dot,
}

/// [`itertools::Itertools::into_group_map`], but for `BTreeMap`.
fn into_group_map<K, V>(iter: impl IntoIterator<Item = (K, V)>) -> BTreeMap<K, Vec<V>>
where
    K: Ord,
{
    let mut out: BTreeMap<_, Vec<_>> = BTreeMap::new();
    for (k, v) in iter {
        out.entry(k).or_default().push(v);
    }
    out
}
