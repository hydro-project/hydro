//! Slotmap key types for graph nodes, edges, subgraphs, and loops.
//!
//! These are separated so they can be used without pulling in heavy codegen
//! dependencies (syn, proc-macro2, quote, etc.).

use slotmap::new_key_type;

new_key_type! {
    /// ID to identify a node (operator or handoff) in `DfirGraph`.
    pub struct GraphNodeId;

    /// ID to identify an edge.
    pub struct GraphEdgeId;

    /// ID to identify a subgraph in `DfirGraph`.
    pub struct GraphSubgraphId;

    /// ID to identify a loop block in `DfirGraph`.
    pub struct GraphLoopId;
}
