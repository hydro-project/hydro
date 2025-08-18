#[cfg(feature = "build")]
#[test]
fn test_other_multi_input_ops_capture() {
    use hydro_lang::{FlowBuilder, Location, ir, q};

    fn find_two_inputs(n: &ir::HydroNode) -> Option<&ir::HydroIrMetadata> {
        match n {
            ir::HydroNode::Join { metadata, .. }
            | ir::HydroNode::CrossProduct { metadata, .. }
            | ir::HydroNode::CrossSingleton { metadata, .. }
            | ir::HydroNode::Difference { metadata, .. }
            | ir::HydroNode::AntiJoin { metadata, .. }
            | ir::HydroNode::Chain { metadata, .. } => Some(metadata),
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
            | ir::HydroNode::Counter { input: inner, .. } => find_two_inputs(inner),
            ir::HydroNode::ReduceKeyedWatermark {
                input, watermark, ..
            } => find_two_inputs(input).or_else(|| find_two_inputs(watermark)),
            ir::HydroNode::Map { input, .. }
            | ir::HydroNode::FlatMap { input, .. }
            | ir::HydroNode::Filter { input, .. }
            | ir::HydroNode::FilterMap { input, .. } => find_two_inputs(input),
            ir::HydroNode::Network { input, .. } => find_two_inputs(input),
            ir::HydroNode::Tee { .. }
            | ir::HydroNode::Source { .. }
            | ir::HydroNode::CycleSource { .. }
            | ir::HydroNode::ExternalInput { .. }
            | ir::HydroNode::Placeholder => None,
        }
    }

    // Build and assert for each operator
    // 1) chain (Bounded)
    let builder = FlowBuilder::new();
    let process = builder.process::<()>();
    let s1 = process.source_iter(q!(vec![1, 2, 3]));
    let s2 = process.source_iter(q!(vec![4, 5, 6]));
    let tick = process.tick();
    let b1 = s1.batch(&tick, hydro_lang::nondet!(/** test */));
    let b2 = s2.batch(&tick, hydro_lang::nondet!(/** test */));
    let chained = b1.chain(b2);
    chained.all_ticks().for_each(q!(|_x| {}));
    let built = builder.finalize();
    let leaf_input = match &built.ir()[0] {
        ir::HydroLeaf::ForEach { input, .. } => input.as_ref(),
        _ => panic!("unexpected"),
    };
    let m = find_two_inputs(leaf_input).expect("multi-input node not found (chain)");
    assert_eq!(
        m.input_collection_types.len(),
        2,
        "chain should record two inputs"
    );

    // 2) cross_product
    let builder = FlowBuilder::new();
    let process = builder.process::<()>();
    let s1 = process.source_iter(q!(vec!['a', 'b']));
    let s2 = process.source_iter(q!(vec![1, 2]));
    let cross = s1.cross_product(s2);
    cross.for_each(q!(|_x| {}));
    let built = builder.finalize();
    let leaf_input = match &built.ir()[0] {
        ir::HydroLeaf::ForEach { input, .. } => input.as_ref(),
        _ => panic!("unexpected"),
    };
    let m = find_two_inputs(leaf_input).expect("multi-input node not found (cross_product)");
    assert_eq!(
        m.input_collection_types.len(),
        2,
        "cross_product should record two inputs"
    );

    // 3) cross_singleton
    let builder = FlowBuilder::new();
    let process = builder.process::<()>();
    let s = process.source_iter(q!(vec![1, 2]));
    let tick = process.tick();
    let batch = s.batch(&tick, hydro_lang::nondet!(/** test */));
    let count = batch.clone().count();
    let crosss = batch.cross_singleton(count);
    crosss.all_ticks().for_each(q!(|_x| {}));
    let built = builder.finalize();
    let leaf_input = match &built.ir()[0] {
        ir::HydroLeaf::ForEach { input, .. } => input.as_ref(),
        _ => panic!("unexpected"),
    };
    let m = find_two_inputs(leaf_input).expect("multi-input node not found (cross_singleton)");
    assert_eq!(
        m.input_collection_types.len(),
        2,
        "cross_singleton should record two inputs"
    );

    // 4) difference (filter_not_in)
    let builder = FlowBuilder::new();
    let process = builder.process::<()>();
    let s = process.source_iter(q!(vec![1, 2, 3, 4]));
    let tick = process.tick();
    let batch = s.clone().batch(&tick, hydro_lang::nondet!(/** test */));
    let neg = process
        .source_iter(q!(vec![1, 2]))
        .batch(&tick, hydro_lang::nondet!(/** test */));
    let diff = batch.filter_not_in(neg);
    diff.all_ticks().for_each(q!(|_x| {}));
    let built = builder.finalize();
    let leaf_input = match &built.ir()[0] {
        ir::HydroLeaf::ForEach { input, .. } => input.as_ref(),
        _ => panic!("unexpected"),
    };
    let m = find_two_inputs(leaf_input).expect("multi-input node not found (difference)");
    assert_eq!(
        m.input_collection_types.len(),
        2,
        "difference should record two inputs"
    );

    // 5) anti_join
    let builder = FlowBuilder::new();
    let process = builder.process::<()>();
    let kv = process.source_iter(q!(vec![(1, 'a'), (2, 'b')]));
    let tick = process.tick();
    let batch = kv.batch(&tick, hydro_lang::nondet!(/** test */));
    let neg = process
        .source_iter(q!(vec![1]))
        .batch(&tick, hydro_lang::nondet!(/** test */));
    let anti = batch.anti_join(neg);
    anti.all_ticks().for_each(q!(|_x| {}));
    let built = builder.finalize();
    let leaf_input = match &built.ir()[0] {
        ir::HydroLeaf::ForEach { input, .. } => input.as_ref(),
        _ => panic!("unexpected"),
    };
    let m = find_two_inputs(leaf_input).expect("multi-input node not found (anti_join)");
    assert_eq!(
        m.input_collection_types.len(),
        2,
        "anti_join should record two inputs"
    );

    // 6) optional union
    let builder = FlowBuilder::new();
    let process = builder.process::<()>();
    let tick = process.tick();
    let batch = process
        .source_iter(q!(vec![1, 2, 3]))
        .batch(&tick, hydro_lang::nondet!(/** test */));
    let o1 = batch.clone().first();
    let o2 = batch.last();
    let u = o1.union(o2);
    let s = u.into_stream();
    s.all_ticks().for_each(q!(|_x| {}));
    let built = builder.finalize();
    let leaf_input = match &built.ir()[0] {
        ir::HydroLeaf::ForEach { input, .. } => input.as_ref(),
        _ => panic!("unexpected"),
    };
    let m = find_two_inputs(leaf_input).expect("multi-input node not found (optional union)");
    assert_eq!(
        m.input_collection_types.len(),
        2,
        "optional union should record two inputs"
    );

    // 7) optional zip
    let builder = FlowBuilder::new();
    let process = builder.process::<()>();
    let tick = process.tick();
    let batch = process
        .source_iter(q!(vec![1, 2, 3]))
        .batch(&tick, hydro_lang::nondet!(/** test */));
    let o1 = batch.clone().first();
    let o2 = batch.last();
    let z = o1.zip(o2);
    let s = z.into_stream();
    s.all_ticks().for_each(q!(|_x| {}));
    let built = builder.finalize();
    let leaf_input = match &built.ir()[0] {
        ir::HydroLeaf::ForEach { input, .. } => input.as_ref(),
        _ => panic!("unexpected"),
    };
    let m = find_two_inputs(leaf_input).expect("multi-input node not found (optional zip)");
    assert_eq!(
        m.input_collection_types.len(),
        2,
        "optional zip should record two inputs"
    );
}
