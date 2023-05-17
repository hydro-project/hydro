#![deny(missing_docs)]

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fmt::Debug;
use std::iter::FusedIterator;

use proc_macro2::{Ident, Literal, Span, TokenStream};
use quote::{format_ident, quote, quote_spanned, ToTokens};
use serde::{Deserialize, Serialize};
use slotmap::{Key, SecondaryMap, SlotMap, SparseSecondaryMap};
use syn::spanned::Spanned;

use super::graph_write::{Dot, GraphWrite, Mermaid};
use super::ops::{find_op_op_constraints, OperatorWriteOutput, WriteContextArgs, OPERATORS};
use super::{
    get_operator_generics, node_color, Color, DiMulGraph, GraphEdgeId, GraphNodeId,
    GraphSubgraphId, Node, OperatorInstance, PortIndexValue, Varname, CONTEXT, HANDOFF_NODE_STR,
    HYDROFLOW,
};
use crate::diagnostic::{Diagnostic, Level};
use crate::graph::ops::null_write_iterator_fn;
use crate::pretty_span::{PrettyRowCol, PrettySpan};

/// A graph representing a hydroflow dataflow graph (with or without subgraph partitioning,
/// stratification, and handoff insertion). This is a "meta" graph used for generating Rust source
/// code in macros from Hydroflow surface sytnax.
///
/// This struct has a lot of methods for manipulating the graph, vaguely grouped together in
/// separate `impl` blocks. You might notice a few particularly specific arbitray-seeming methods
/// in here--those are just what was needed for the compilation algorithms. If you need another
/// method then add it.
#[derive(Default, Debug, Serialize, Deserialize)]
pub struct HydroflowGraph {
    /// Each node (operator or handoff).
    nodes: SlotMap<GraphNodeId, Node>,
    /// Instance data corresponding to each operator node.
    /// This field will be empty after deserialization.
    #[serde(skip)]
    operator_instances: SecondaryMap<GraphNodeId, OperatorInstance>,
    /// Graph data structure (two-way adjacency list).
    graph: DiMulGraph<GraphNodeId, GraphEdgeId>,
    /// Input and output port for each edge.
    ports: SecondaryMap<GraphEdgeId, (PortIndexValue, PortIndexValue)>,
    /// Which subgraph each node belongs to.
    node_subgraph: SecondaryMap<GraphNodeId, GraphSubgraphId>,

    /// Which nodes belong to each subgraph.
    subgraph_nodes: SlotMap<GraphSubgraphId, Vec<GraphNodeId>>,
    /// Which stratum each subgraph belongs to.
    subgraph_stratum: SecondaryMap<GraphSubgraphId, usize>,

    /// What variable name each graph node belongs to (if any).
    node_varnames: SparseSecondaryMap<GraphNodeId, Varname>,
}
impl HydroflowGraph {
    /// Create a new empty `HydroflowGraph`.
    pub fn new() -> Self {
        Default::default()
    }
}
// Node methods.
impl HydroflowGraph {
    /// Get a node with its operator instance (if applicable).
    pub fn node(&self, node_id: GraphNodeId) -> &Node {
        self.nodes.get(node_id).expect("Node not found.")
    }

    /// Get the `OperatorInstance` for a given node. Node must be an operator and have an
    /// `OperatorInstance` present, otherwise will return `None`.
    ///
    /// Note that no operator instances will be persent after deserialization.
    pub fn node_op_inst(&self, node_id: GraphNodeId) -> Option<&OperatorInstance> {
        self.operator_instances.get(node_id)
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
           + Iterator<Item = (GraphEdgeId, GraphNodeId)>
           + DoubleEndedIterator
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
           + Iterator<Item = (GraphEdgeId, GraphNodeId)>
           + DoubleEndedIterator
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
           + Iterator<Item = GraphEdgeId>
           + DoubleEndedIterator
           + ExactSizeIterator
           + FusedIterator
           + Clone
           + Debug {
        self.graph.successor_edges(src)
    }

    /// Predecessor edges, iterator of `GraphEdgeId` of incoming edges.
    pub fn node_predecessor_edges(
        &self,
        src: GraphNodeId,
    ) -> impl '_
           + Iterator<Item = GraphEdgeId>
           + DoubleEndedIterator
           + ExactSizeIterator
           + FusedIterator
           + Clone
           + Debug {
        self.graph.predecessor_edges(src)
    }

    /// Successor nodes, iterator of `GraphNodeId`.
    pub fn node_successor_nodes(
        &self,
        src: GraphNodeId,
    ) -> impl '_
           + Iterator<Item = GraphNodeId>
           + DoubleEndedIterator
           + ExactSizeIterator
           + FusedIterator
           + Clone
           + Debug {
        self.graph.successor_vertices(src)
    }

    /// Predecessor edges, iterator of `GraphNodeId`.
    pub fn node_predecessor_nodes(
        &self,
        src: GraphNodeId,
    ) -> impl '_
           + Iterator<Item = GraphNodeId>
           + DoubleEndedIterator
           + ExactSizeIterator
           + FusedIterator
           + Clone
           + Debug {
        self.graph.predecessor_vertices(src)
    }

    /// Iterator of node IDs `GraphNodeId`.
    pub fn node_ids(&self) -> slotmap::basic::Keys<'_, GraphNodeId, Node> {
        self.nodes.keys()
    }

    /// Iterator over `(GraphNodeId, &Node)` pairs.
    pub fn nodes(&self) -> slotmap::basic::Iter<'_, GraphNodeId, Node> {
        self.nodes.iter()
    }

    /// Insert a node, assigning the given varname.
    pub fn insert_node(&mut self, node: Node, varname_opt: Option<Ident>) -> GraphNodeId {
        let node_id = self.nodes.insert(node);
        if let Some(varname) = varname_opt {
            self.node_varnames.insert(node_id, Varname(varname));
        }
        node_id
    }

    /// Insert an operator instance for the given node. Panics if already set.
    pub fn insert_node_op_inst(&mut self, node_id: GraphNodeId, op_inst: OperatorInstance) {
        assert!(matches!(self.nodes.get(node_id), Some(Node::Operator(_))));
        let old_inst = self.operator_instances.insert(node_id, op_inst);
        assert!(old_inst.is_none());
    }

    /// Assign all operator instances if not set. Write diagnostic messages/errors into `diagnostics`.
    pub fn insert_node_op_insts_all(&mut self, diagnostics: &mut Vec<Diagnostic>) {
        let mut op_insts = Vec::new();
        for (node_id, node) in self.nodes() {
            let Node::Operator(operator) = node else {
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
                if !op_constraints
                    .persistence_args
                    .contains(&generics.persistence_args.len())
                {
                    diagnostics.push(Diagnostic::spanned(
                        generics.generic_args.span(),
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
                        generics.generic_args.span(),
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
                    generics,
                    arguments: operator.args.clone(),
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
    pub fn insert_intermediate_node(
        &mut self,
        edge_id: GraphEdgeId,
        new_node: Node,
    ) -> (GraphNodeId, GraphEdgeId) {
        let span = Some(new_node.span());

        // Make corresponding operator instance (if `node` is an operator).
        let op_inst_opt = 'oc: {
            let Node::Operator(operator) = &new_node else { break 'oc None; };
            let Some(op_constraints) = find_op_op_constraints(operator) else { break 'oc None; };
            let (input_port, output_port) = self.ports.get(edge_id).cloned().unwrap();
            let generics = get_operator_generics(
                &mut Vec::new(), // TODO(mingwei) diagnostics
                operator,
            );
            Some(OperatorInstance {
                op_constraints,
                input_ports: vec![input_port],
                output_ports: vec![output_port],
                generics,
                arguments: operator.args.clone(),
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
}
// Edge methods.
impl HydroflowGraph {
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
    pub fn edge_ids(&self) -> slotmap::basic::Keys<GraphEdgeId, (GraphNodeId, GraphNodeId)> {
        self.graph.edge_ids()
    }

    /// Iterator over all edges: `(GraphEdgeId, (src GraphNodeId, dst GraphNodeId))`.
    pub fn edges(
        &self,
    ) -> impl '_
           + Iterator<Item = (GraphEdgeId, (GraphNodeId, GraphNodeId))>
           + ExactSizeIterator
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
}
// Subgraph methods.
impl HydroflowGraph {
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
        Ok(self.subgraph_nodes.insert_with_key(|sg_id| {
            for &node_id in node_ids.iter() {
                self.node_subgraph.insert(node_id, sg_id);
            }
            node_ids
        }))
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

    /// Gets the stratum nubmer of the subgraph.
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

    /// Returns the the stratum number of the largest (latest) stratum (inclusive).
    pub fn max_stratum(&self) -> Option<usize> {
        self.subgraph_stratum.values().copied().max()
    }
}
// Display/output stuff.
impl HydroflowGraph {
    /// Helper to generate a deterministic `Ident` for the given node.
    fn node_as_ident(&self, node_id: GraphNodeId, is_pred: bool) -> Ident {
        let name = match &self.nodes[node_id] {
            Node::Operator(_) => format!("op_{:?}", node_id.data()),
            Node::Handoff { .. } => format!(
                "hoff_{:?}_{}",
                node_id.data(),
                if is_pred { "recv" } else { "send" }
            ),
        };
        let span = match (is_pred, &self.nodes[node_id]) {
            (_, Node::Operator(operator)) => operator.span(),
            (true, &Node::Handoff { src_span, .. }) => src_span,
            (false, &Node::Handoff { dst_span, .. }) => dst_span,
        };
        Ident::new(&*name, span)
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
            if !matches!(node, Node::Handoff { .. }) {
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

    /// Emit this `HydroflowGraph` as runnable Rust source code tokens.
    pub fn as_code(
        &self,
        root: &TokenStream,
        include_type_guards: bool,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> TokenStream {
        let hf = Ident::new(HYDROFLOW, Span::call_site());
        let context = Ident::new(CONTEXT, Span::call_site());

        let handoffs = self
            .nodes
            .iter()
            .filter_map(|(node_id, node)| match node {
                Node::Operator(_) => None,
                &Node::Handoff { src_span, dst_span } => Some((node_id, (src_span, dst_span))),
            })
            .map(|(node_id, (src_span, dst_span))| {
                let ident_send = Ident::new(&*format!("hoff_{:?}_send", node_id.data()), dst_span);
                let ident_recv = Ident::new(&*format!("hoff_{:?}_recv", node_id.data()), src_span);
                let hoff_name = Literal::string(&*format!("handoff {:?}", node_id));
                quote! {
                    let (#ident_send, #ident_recv) =
                        #hf.make_edge::<_, #root::scheduled::handoff::VecHandoff<_>>(#hoff_name);
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

        let subgraphs = subgraphs_without_preds
            .iter()
            .chain(subgraphs_with_preds.iter())
            .map(|&(subgraph_id, subgraph_nodes)| {
                let (recv_hoffs, send_hoffs) = &subgraph_handoffs[subgraph_id];
                let recv_ports: Vec<Ident> = recv_hoffs
                    .iter()
                    .map(|&hoff_id| self.node_as_ident(hoff_id, true))
                    .collect();
                let send_ports: Vec<Ident> = send_hoffs
                    .iter()
                    .map(|&hoff_id| self.node_as_ident(hoff_id, false))
                    .collect();

                let recv_port_code = recv_ports
                    .iter()
                    .map(|ident| {
                        quote! {
                            let mut #ident = #ident.borrow_mut_swap();
                            let #ident = #ident.drain(..);
                        }
                    });
                let send_port_code = send_ports.iter().map(|ident| {
                    quote! {
                        let #ident = #root::pusherator::for_each::ForEach::new(|v| {
                            #ident.give(Some(v));
                        });
                    }
                });

                let mut op_prologue_code = Vec::new();
                let mut subgraph_op_iter_code = Vec::new();
                let mut subgraph_op_iter_after_code = Vec::new();
                {
                    let pull_to_push_idx = subgraph_nodes
                        .iter()
                        .position(|&node_id| {
                            node_color(
                                matches!(self.nodes[node_id], Node::Handoff { .. }),
                                self.graph.degree_in(node_id),
                                self.graph.degree_out(node_id),
                            )
                            .map(|color| Color::Pull != color)
                            .unwrap_or(false)
                        })
                        .unwrap_or(subgraph_nodes.len());

                    let (pull_half, push_half) = subgraph_nodes.split_at(pull_to_push_idx);
                    let nodes_iter = pull_half.iter().chain(push_half.iter().rev());

                    for (idx, &node_id) in nodes_iter.enumerate() {
                        let node = &self.nodes[node_id];
                        assert!(matches!(node, Node::Operator(_)), "Handoffs are not part of subgraphs.");
                        let op_inst = &self.operator_instances[node_id];

                        let op_span = node.span();
                        let op_name = op_inst.op_constraints.name;
                        let op_constraints = OPERATORS
                            .iter()
                            .find(|op| op_name == op.name)
                            .unwrap_or_else(|| panic!("Failed to find op: {}", op_name));

                        let ident = self.node_as_ident(node_id, false);

                        {
                            // TODO clean this up.
                            // Collect input arguments (predecessors).
                            let mut input_edges: Vec<(&PortIndexValue, GraphNodeId)> =
                                self.graph.predecessors(node_id)
                                    .map(|(edge_id, pred)| (&self.ports[edge_id].1, pred))
                                    .collect();
                            // Ensure sorted by port index.
                            input_edges.sort();

                            let inputs: Vec<Ident> = input_edges
                                .iter()
                                .map(|&(_port, pred)| self.node_as_ident(pred, true))
                                .collect();

                            // Collect output arguments (successors).
                            let mut output_edges: Vec<(&PortIndexValue, GraphNodeId)> =
                                self.graph.successors(node_id)
                                    .map(|(edge_id, succ)| (&self.ports[edge_id].0, succ))
                                    .collect();
                            // Ensure sorted by port index.
                            output_edges.sort();

                            let outputs: Vec<Ident> = output_edges
                                .iter()
                                .map(|&(_port, succ)| self.node_as_ident(succ, false))
                                .collect();

                            let is_pull = idx < pull_to_push_idx;

                            let context_args = WriteContextArgs {
                                root,
                                // There's a bit of dark magic hidden in `Span`s... you'd think it's just a `file:line:column`,
                                // but it has one extra bit of info for _name resolution_, used for `Ident`s. `Span::call_site()`
                                // has the (unhygienic) resolution we want, an ident is just solely determined by its string name,
                                // which is what you'd expect out of unhygienic proc macros like this. Meanwhile, declarative macros
                                // use `Span::mixed_site()` which is weird and I don't understand it. It turns out that if you call
                                // the hydroflow syntax proc macro from _within_ a declarative macro then `op_span` will have the
                                // bad `Span::mixed_site()` name resolution and cause "Cannot find value `df/context`" errors. So
                                // we call `.resolved_at()` to fix resolution back to `Span::call_site()`. -Mingwei
                                hydroflow: &Ident::new(HYDROFLOW, op_span.resolved_at(hf.span())),
                                context: &Ident::new(CONTEXT, op_span.resolved_at(context.span())),
                                subgraph_id,
                                node_id,
                                op_span,
                                ident: &ident,
                                is_pull,
                                inputs: &*inputs,
                                outputs: &*outputs,
                                op_name,
                                op_inst,
                            };

                            let write_result = (op_constraints.write_fn)(&context_args, diagnostics);
                            let OperatorWriteOutput {
                                write_prologue,
                                write_iterator,
                                write_iterator_after,
                            } = write_result.unwrap_or_else(|()| OperatorWriteOutput { write_iterator: null_write_iterator_fn(&context_args), ..Default::default() });

                            op_prologue_code.push(write_prologue);
                            subgraph_op_iter_code.push(write_iterator);

                            if include_type_guards {
                                let source_info = {
                                    // TODO: This crashes when running tests from certain directories because of diagnostics flag being turned on when it should not be on.
                                    // Not sure of the solution yet, but it is not too important because the file is usually obvious as there can only be one until module support is added.
                                    // #[cfg(feature = "diagnostics")]
                                    // let path = op_span.unwrap().source_file().path();
                                    #[cfg(feature = "diagnostics")]
                                    let location = "unknown"; // path.display();

                                    #[cfg(not(feature = "diagnostics"))]
                                    let location = "unknown";

                                    let location = location.to_string().replace(|x: char| !x.is_alphanumeric(), "_");

                                    format!(
                                        "loc_{}_start_{}_{}_end_{}_{}",
                                        location,
                                        op_span.start().line,
                                        op_span.start().column,
                                        op_span.end().line,
                                        op_span.end().column
                                    )
                                };
                                let fn_ident = format_ident!("{}__{}__{}", ident, op_name, source_info, span = op_span);
                                let type_guard = if is_pull {
                                    quote_spanned! {op_span=>
                                        let #ident = {
                                            #[allow(non_snake_case)]
                                            #[inline(always)]
                                            pub fn #fn_ident<Item, Input: ::std::iter::Iterator<Item = Item>>(input: Input) -> impl ::std::iter::Iterator<Item = Item> {
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
                                            #fn_ident( #ident )
                                        };
                                    }
                                } else {
                                    quote_spanned! {op_span=>
                                        let #ident = {
                                            #[allow(non_snake_case)]
                                            #[inline(always)]
                                            pub fn #fn_ident<Item, Input: #root::pusherator::Pusherator<Item = Item>>(input: Input) -> impl #root::pusherator::Pusherator<Item = Item> {
                                                struct Push<Item, Input: #root::pusherator::Pusherator<Item = Item>> {
                                                    inner: Input
                                                }

                                                impl<Item, Input: #root::pusherator::Pusherator<Item = Item>> #root::pusherator::Pusherator for Push<Item, Input> {
                                                    type Item = Item;

                                                    #[inline(always)]
                                                    fn give(&mut self, item: Self::Item) {
                                                        self.inner.give(item)
                                                    }
                                                }

                                                Push {
                                                    inner: input
                                                }
                                            }
                                            #fn_ident( #ident )
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
                        let pull_to_push_idx = pull_to_push_idx;
                        let pull_ident =
                            self.node_as_ident(subgraph_nodes[pull_to_push_idx - 1], false);

                        #[rustfmt::skip]
                        let push_ident = if let Some(&node_id) =
                            subgraph_nodes.get(pull_to_push_idx)
                        {
                            self.node_as_ident(node_id, false)
                        } else {
                            // Entire subgraph is pull (except for a single send/push handoff output).
                            assert_eq!(
                                1,
                                send_ports.len(),
                                "If entire subgraph is pull, should have only one handoff output. Do you have a loose `null()` or other degenerate pipeline somewhere?"
                            );
                            send_ports[0].clone()
                        };

                        // Pivot span is combination of pull and push spans (or if not possible, just take the push).
                        let pivot_span = pull_ident
                            .span()
                            .join(push_ident.span())
                            .unwrap_or_else(|| push_ident.span());
                        subgraph_op_iter_code.push(quote_spanned! {pivot_span=>
                            #[inline(always)]
                            fn check_pivot_run<Pull: ::std::iter::Iterator<Item = Item>, Push: #root::pusherator::Pusherator<Item = Item>, Item>(pull: Pull, push: Push) {
                                #root::pusherator::pivot::Pivot::new(pull, push).run();
                            }
                            check_pivot_run(#pull_ident, #push_ident);
                        });
                    }
                };

                let hoff_name = Literal::string(&*format!("Subgraph {:?}", subgraph_id));
                let stratum = Literal::usize_unsuffixed(
                    self.subgraph_stratum.get(subgraph_id).cloned().unwrap_or(0),
                );
                quote! {
                    #( #op_prologue_code )*

                    #hf.add_subgraph_stratified(
                        #hoff_name,
                        #stratum,
                        var_expr!( #( #recv_ports ),* ),
                        var_expr!( #( #send_ports ),* ),
                        move |#context, var_args!( #( #recv_ports ),* ), var_args!( #( #send_ports ),* )| {
                            #( #recv_port_code )*
                            #( #send_port_code )*
                            #( #subgraph_op_iter_code )*
                            #( #subgraph_op_iter_after_code )*
                        },
                    );
                }
            });

        // These two are quoted separately here because iterators are lazily evaluated, so this
        // forces them to do their work. This work includes populating some data, namely
        // `diagonstics`, which we need to determine if it compilation was actually successful.
        // -Mingwei
        let code = quote! {
            #( #handoffs )*
            #( #subgraphs )*
        };

        let meta_graph_json = serde_json::to_string(&self).unwrap();
        let meta_graph_json = Literal::string(&*meta_graph_json);

        let serde_diagnostics: Vec<_> = diagnostics.iter().map(Diagnostic::to_serde).collect();
        let diagnostics_json = serde_json::to_string(&*serde_diagnostics).unwrap();
        let diagnostics_json = Literal::string(&*diagnostics_json);

        quote! {
            {
                #[allow(unused_qualifications)]
                {
                    use #root::{var_expr, var_args};

                    let mut #hf = #root::scheduled::graph::Hydroflow::new();
                    #hf.__assign_meta_graph(#meta_graph_json);
                    #hf.__assign_diagnostics(#diagnostics_json);

                    #code

                    #hf
                }
            }
        }
    }

    /// Color mode (pull vs. push, handoff vs. comp) for nodes. Some nodes can be push *OR* pull;
    /// those nodes will not be set in the returned map.
    pub fn node_color_map(&self) -> SparseSecondaryMap<GraphNodeId, Color> {
        // TODO(mingwei): this repeated code will be unified when `SerdeGraph` is subsumed into `HydroflowGraph`.
        // TODO(mingwei): REPEATED CODE, COPIED FROM `flat_to_partitioned.rs`

        let mut node_color_map: SparseSecondaryMap<GraphNodeId, Color> = self
            .nodes
            .iter()
            .filter_map(|(node_id, node)| {
                let inn_degree = self.node_degree_in(node_id);
                let out_degree = self.node_degree_out(node_id);
                let is_handoff = matches!(node, Node::Handoff { .. });
                let op_color = node_color(is_handoff, inn_degree, out_degree);
                op_color.map(|op_color| (node_id, op_color))
            })
            .collect();

        // Fill in rest via subgraphs.
        for sg_nodes in self.subgraph_nodes.values() {
            // TODO(mingwei): REPEATED CODE, COPIED FROM `partitioned_graph.rs` codegen.
            let pull_to_push_idx = sg_nodes
                .iter()
                .position(|&node_id| {
                    let inn_degree = self.node_degree_in(node_id);
                    let out_degree = self.node_degree_out(node_id);
                    let node = &self.nodes[node_id];
                    let is_handoff = matches!(node, Node::Handoff { .. });
                    node_color(is_handoff, inn_degree, out_degree)
                        .map(|color| Color::Pull != color)
                        .unwrap_or(false)
                })
                .unwrap_or(sg_nodes.len());

            for (idx, node_id) in sg_nodes.iter().copied().enumerate() {
                let is_pull = idx < pull_to_push_idx;
                node_color_map.insert(node_id, if is_pull { Color::Pull } else { Color::Push });
            }
        }

        node_color_map
    }

    /// Writes this graph as mermaid into a string.
    pub fn to_mermaid(&self) -> String {
        let mut output = String::new();
        self.write_mermaid(&mut output).unwrap();
        output
    }

    /// Writes this graph as mermaid into the given `Write`.
    pub fn write_mermaid(&self, output: impl std::fmt::Write) -> std::fmt::Result {
        let mut graph_write = Mermaid::new(output);
        self.write_graph(&mut graph_write)
    }

    /// Writes this graph as DOT (graphviz) into a string.
    pub fn to_dot(&self) -> String {
        let mut output = String::new();
        let mut graph_write = Dot::new(&mut output);
        self.write_graph(&mut graph_write).unwrap();
        output
    }

    /// Writes this graph as DOT (graphviz) into the given `Write`.
    pub fn write_dot(&self, output: impl std::fmt::Write) -> std::fmt::Result {
        let mut graph_write = Dot::new(output);
        self.write_graph(&mut graph_write)
    }

    /// Write out this `HydroflowGraph` using the given `GraphWrite`. E.g. `Mermaid` or `Dot.
    pub fn write_graph<W>(&self, mut graph_write: W) -> Result<(), W::Err>
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
                (Some(l1), Some(l2)) => Some(format!("{} ~ {}", l1, l2)),
                (Some(l1), None) => Some(l1),
                (None, Some(l2)) => Some(l2),
                (None, None) => None,
            };
            label
        }

        let subgraph_handoffs = self.helper_collect_subgraph_handoffs();
        let node_color_map = self.node_color_map();
        let mut sg_varname_nodes =
            SparseSecondaryMap::<GraphSubgraphId, BTreeMap<Varname, BTreeSet<GraphNodeId>>>::new();
        for (node_id, varname) in self.node_varnames.iter() {
            let Some(sg_id) = self.node_subgraph(node_id) else { continue; };
            let varname_map = sg_varname_nodes.entry(sg_id).unwrap().or_default();
            varname_map
                .entry(varname.clone())
                .or_default()
                .insert(node_id);
        }
        let barrier_handoffs: HashSet<_> = self
            .node_ids()
            .filter(|&node_id| matches!(self.node(node_id), Node::Handoff { .. }))
            .filter(|&hoff_id| {
                assert_eq!(1, self.node_degree_in(hoff_id));
                assert_eq!(1, self.node_degree_out(hoff_id));
                let src_id = self.node_predecessor_nodes(hoff_id).next().unwrap();
                let dst_id = self.node_successor_nodes(hoff_id).next().unwrap();
                // Only include handoffs between different subgraphs.
                self.node_subgraph(src_id) != self.node_subgraph(dst_id)
            })
            .collect();

        graph_write.write_prologue()?;

        for (subgraph_id, subgraph_node_ids) in self.subgraph_nodes.iter() {
            let stratum = self.subgraph_stratum.get(subgraph_id);
            graph_write.write_subgraph_start(subgraph_id, *stratum.unwrap())?;

            // write out nodes
            for &node_id in subgraph_node_ids.iter() {
                graph_write.write_node(
                    node_id,
                    &*self.nodes.get(node_id).unwrap().to_pretty_string(),
                    node_color_map.get(node_id).copied().unwrap_or(Color::Comp),
                    Some(subgraph_id),
                )?;
            }
            // write out internal handoffs
            let internal_handoffs = {
                let (recvs, sends) = &subgraph_handoffs[subgraph_id];
                recvs.iter().filter(move |recv| sends.contains(recv))
            };
            for &hoff_id in internal_handoffs {
                graph_write.write_node(
                    hoff_id,
                    &*self.nodes.get(hoff_id).unwrap().to_pretty_string(),
                    Color::Hoff,
                    Some(subgraph_id),
                )?;
                // write out internal handoff edges. should only be one.
                assert_eq!(1, self.node_degree_out(hoff_id));
                let (edge_id, dst_id) = self.node_successors(hoff_id).next().unwrap();
                let (src_port, dst_port) = self.edge_ports(edge_id);
                let delay_type = self
                    .node_op_inst(dst_id)
                    .and_then(|op_inst| (op_inst.op_constraints.input_delaytype_fn)(dst_port));
                let label = helper_edge_label(src_port, dst_port);
                graph_write.write_edge(
                    hoff_id,
                    dst_id,
                    delay_type,
                    label.as_deref(),
                    Some(subgraph_id),
                )?;
            }

            // write out edges. Includes edges leaving the subgraph.
            for &src_id in subgraph_node_ids.iter() {
                for (edge_id, dst_id) in self.node_successors(src_id) {
                    if !barrier_handoffs.contains(&dst_id) {
                        let (src_port, dst_port) = self.edge_ports(edge_id);
                        let delay_type = self.node_op_inst(dst_id).and_then(|op_inst| {
                            (op_inst.op_constraints.input_delaytype_fn)(dst_port)
                        });
                        let label = helper_edge_label(src_port, dst_port);
                        graph_write.write_edge(
                            src_id,
                            dst_id,
                            delay_type,
                            label.as_deref(),
                            Some(subgraph_id),
                        )?;
                    }
                }
            }

            // write out any variable names
            for (varname, varname_node_ids) in
                sg_varname_nodes.get(subgraph_id).into_iter().flatten()
            {
                assert!(!varname_node_ids.is_empty());
                graph_write.write_subgraph_varname(
                    subgraph_id,
                    &*varname.0.to_string(),
                    varname_node_ids.iter().cloned(),
                )?;
            }

            graph_write.write_subgraph_end()?;
        }

        // Write out external handoffs (handoffs between different subgraphs).
        // This is set up in an awkward way in order to preserve the old write order.
        // write out handoffs outside the clusters and adjacent edges
        for (src_id, src_node) in self.nodes() {
            for (edge_id, dst_id) in self.node_successors(src_id) {
                if barrier_handoffs.contains(&src_id) {
                    // write out handoff
                    graph_write.write_node(
                        src_id,
                        &*src_node.to_pretty_string(),
                        Color::Hoff,
                        None,
                    )?;

                    // write out edge
                    let (src_port, dst_port) = self.edge_ports(edge_id);
                    let delay_type = self
                        .node_op_inst(dst_id)
                        .and_then(|op_inst| (op_inst.op_constraints.input_delaytype_fn)(dst_port));
                    let label = helper_edge_label(src_port, dst_port);
                    graph_write.write_edge(src_id, dst_id, delay_type, label.as_deref(), None)?;
                } else if barrier_handoffs.contains(&dst_id) {
                    // write out edge
                    let (src_port, dst_port) = self.edge_ports(edge_id);
                    let delay_type = self
                        .node_op_inst(dst_id)
                        .and_then(|op_inst| (op_inst.op_constraints.input_delaytype_fn)(dst_port));
                    let label = helper_edge_label(src_port, dst_port);
                    graph_write.write_edge(src_id, dst_id, delay_type, label.as_deref(), None)?;
                }
            }
        }

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
                Node::Operator(op) => {
                    writeln!(write, "{:?} = {};", key.data(), op.to_token_stream())?;
                }
                Node::Handoff { .. } => unimplemented!("HANDOFF IN FLAT GRAPH."),
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
                Node::Operator(operator) => writeln!(
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
                Node::Handoff { .. } => {
                    writeln!(write, r#"    {:?}{{"{}"}}"#, key.data(), HANDOFF_NODE_STR)
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
