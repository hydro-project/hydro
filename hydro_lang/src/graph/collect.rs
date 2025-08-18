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

fn collect_leaf<'a>(leaf: &'a HydroLeaf, out: &mut CollectedGraph<'a>) {
    out.nodes.push(CollectedNode {
        op: leaf.print_root(),
        metadata: leaf.metadata(),
    });
    for m in leaf.input_metadata() {
        let _ = m; // ensure we exercise the API
    }
    match leaf {
        HydroLeaf::ForEach { input, .. }
        | HydroLeaf::DestSink { input, .. }
        | HydroLeaf::CycleSink { input, .. } => collect_node(input, out),
        HydroLeaf::SendExternal { .. } => {}
    }
}

fn collect_node<'a>(node: &'a HydroNode, out: &mut CollectedGraph<'a>) {
    out.nodes.push(CollectedNode {
        op: node.print_root(),
        metadata: node.metadata(),
    });
    for m in node.input_metadata() {
        let _ = m;
    }
    match node {
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
        | HydroNode::Counter { input: inner, .. } => collect_node(inner, out),

        HydroNode::ReduceKeyedWatermark {
            input, watermark, ..
        } => {
            collect_node(input, out);
            collect_node(watermark, out);
        }

        HydroNode::Map { input, .. }
        | HydroNode::FlatMap { input, .. }
        | HydroNode::Filter { input, .. }
        | HydroNode::FilterMap { input, .. } => collect_node(input, out),

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

        HydroNode::Network { input, .. } => collect_node(input, out),
        HydroNode::Tee { input, .. } => collect_node(input, out),
        HydroNode::Source { .. } => {}
    }
}
