#[cfg(feature = "build")]
#[test]
fn test_multi_input_metadata_capture() {
    use hydro_lang::{FlowBuilder, Location, ir, q};
    let builder = FlowBuilder::new();
    let process = builder.process::<()>();

    // Build in process root (NoTick) to simplify leaf creation
    let s1 = process.source_iter(q!(vec![('a', 1usize), ('b', 2usize)]));
    let s2 = process.source_iter(q!(vec![('a', 10usize), ('b', 20usize)]));

    let joined = s1.clone().join(s2.clone());
    joined.for_each(q!(|_x| {}));

    // Finalize and get IR leaves
    let built = builder.finalize();
    let leaves = built.ir();
    assert!(!leaves.is_empty());

    // Walk down to find the Join node and check its metadata
    fn find_join(n: &ir::HydroNode) -> Option<&ir::HydroNode> {
        match n {
            ir::HydroNode::Placeholder => None,
            ir::HydroNode::CycleSource { .. } => None,
            ir::HydroNode::ExternalInput { .. } => None,
            ir::HydroNode::Join { .. } => Some(n),
            ir::HydroNode::Persist { inner, .. }
            | ir::HydroNode::Unpersist { inner, .. }
            | ir::HydroNode::Delta { inner, .. }
            | ir::HydroNode::DeferTick { input: inner, .. }
            | ir::HydroNode::Enumerate { input: inner, .. }
            | ir::HydroNode::Inspect { input: inner, .. }
            | ir::HydroNode::Unique { input: inner, .. }
            | ir::HydroNode::Sort { input: inner, .. }
            | ir::HydroNode::Scan { input: inner, .. }
            | ir::HydroNode::Fold { input: inner, .. }
            | ir::HydroNode::FoldKeyed { input: inner, .. }
            | ir::HydroNode::Reduce { input: inner, .. }
            | ir::HydroNode::ResolveFutures { input: inner, .. }
            | ir::HydroNode::ResolveFuturesOrdered { input: inner, .. }
            | ir::HydroNode::ReduceKeyed { input: inner, .. }
            | ir::HydroNode::Counter { input: inner, .. } => find_join(inner),

            ir::HydroNode::ReduceKeyedWatermark {
                input, watermark, ..
            } => find_join(input).or_else(|| find_join(watermark)),

            ir::HydroNode::Map { input, .. }
            | ir::HydroNode::FlatMap { input, .. }
            | ir::HydroNode::Filter { input, .. }
            | ir::HydroNode::FilterMap { input, .. } => find_join(input),

            ir::HydroNode::CrossProduct { left, right, .. }
            | ir::HydroNode::CrossSingleton { left, right, .. }
            | ir::HydroNode::Difference {
                pos: left,
                neg: right,
                ..
            }
            | ir::HydroNode::AntiJoin {
                pos: left,
                neg: right,
                ..
            }
            | ir::HydroNode::Chain {
                first: left,
                second: right,
                ..
            } => find_join(left).or_else(|| find_join(right)),

            ir::HydroNode::Network { input, .. } => find_join(input),
            ir::HydroNode::Tee { .. } => None,
            ir::HydroNode::Source { .. } => None,
        }
    }

    let for_each_input = match &leaves[0] {
        ir::HydroLeaf::ForEach { input, .. } => input.as_ref(),
        _ => panic!("unexpected leaf kind"),
    };
    let join_node = find_join(for_each_input).expect("join node not found");
    let m = join_node.metadata();
    assert_eq!(
        m.input_collection_types.len(),
        2,
        "expected two inputs captured for join"
    );
}
