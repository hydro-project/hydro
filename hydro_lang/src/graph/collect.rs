use crate::ir::{HydroIrMetadata, HydroLeaf, HydroNode};

#[derive(Debug, Clone)]
pub struct CollectedNode<'a> {
    pub op: String,
    pub metadata: &'a HydroIrMetadata,
}

#[derive(Debug, Default, Clone)]
pub struct CollectedGraph<'a> {
    pub nodes: Vec<CollectedNode<'a>>,
}

impl<'a> CollectedGraph<'a> {
    pub fn collect_from_leaves(leaves: &'a [HydroLeaf]) -> Self {
        let mut g = CollectedGraph { nodes: Vec::new() };
        for leaf in leaves {
            collect_leaf(leaf, &mut g);
        }
        g
    }
}

/// Helper to add a node to the collection and exercise its metadata API
fn add_node_to_collection<'a>(
    op: String,
    metadata: &'a HydroIrMetadata,
    input_metadata: impl Iterator<Item = &'a HydroIrMetadata>,
    out: &mut CollectedGraph<'a>,
) {
    out.nodes.push(CollectedNode { op, metadata });
    // Exercise the input metadata API to ensure it works
    for m in input_metadata {
        let _ = m;
    }
}

fn collect_leaf<'a>(leaf: &'a HydroLeaf, out: &mut CollectedGraph<'a>) {
    add_node_to_collection(
        leaf.print_root(),
        leaf.metadata(),
        leaf.input_metadata(),
        out,
    );

    match leaf {
        HydroLeaf::ForEach { input, .. }
        | HydroLeaf::DestSink { input, .. }
        | HydroLeaf::CycleSink { input, .. } => collect_node(input, out),
        HydroLeaf::SendExternal { .. } => {}
    }
}

fn collect_node<'a>(node: &'a HydroNode, out: &mut CollectedGraph<'a>) {
    add_node_to_collection(
        node.print_root(),
        node.metadata(),
        node.input_metadata(),
        out,
    );

    match node {
        // Single-input nodes (grouped by pattern)
        HydroNode::Persist { inner, .. }
        | HydroNode::Unpersist { inner, .. }
        | HydroNode::Delta { inner, .. }
        | HydroNode::DeferTick { input: inner, .. }
        | HydroNode::Enumerate { input: inner, .. }
        | HydroNode::Inspect { input: inner, .. }
        | HydroNode::Unique { input: inner, .. }
        | HydroNode::Sort { input: inner, .. }
        | HydroNode::Scan { input: inner, .. }
        | HydroNode::Fold { input: inner, .. }
        | HydroNode::FoldKeyed { input: inner, .. }
        | HydroNode::Reduce { input: inner, .. }
        | HydroNode::ResolveFutures { input: inner, .. }
        | HydroNode::ResolveFuturesOrdered { input: inner, .. }
        | HydroNode::ReduceKeyed { input: inner, .. }
        | HydroNode::Counter { input: inner, .. }
        | HydroNode::Map { input: inner, .. }
        | HydroNode::FlatMap { input: inner, .. }
        | HydroNode::Filter { input: inner, .. }
        | HydroNode::FilterMap { input: inner, .. }
        | HydroNode::Network { input: inner, .. }
        | HydroNode::Tee { input: inner, .. } => collect_node(inner, out),

        // Two-input nodes
        HydroNode::ReduceKeyedWatermark {
            input, watermark, ..
        } => {
            collect_node(input, out);
            collect_node(watermark, out);
        }

        HydroNode::Join { left, right, .. }
        | HydroNode::CrossProduct { left, right, .. }
        | HydroNode::CrossSingleton { left, right, .. }
        | HydroNode::Difference {
            pos: left,
            neg: right,
            ..
        }
        | HydroNode::AntiJoin {
            pos: left,
            neg: right,
            ..
        }
        | HydroNode::Chain {
            first: left,
            second: right,
            ..
        } => {
            collect_node(left, out);
            collect_node(right, out);
        }

        // Leaf nodes (no inputs)
        HydroNode::Source { .. }
        | HydroNode::CycleSource { .. }
        | HydroNode::ExternalInput { .. }
        | HydroNode::Placeholder => {}
    }
}
