#[cfg(test)]
mod tests {
    use futures::StreamExt;
    use hydro_deploy::Deployment;
    use hydro_lang::compile::builder::FlowBuilder;
    use hydro_lang::location::Location;
    use stageleft::q;

    #[tokio::test]
    async fn test_singleton_ref() {
        let mut deployment = Deployment::new();

        let mut builder = FlowBuilder::new();
        let external = builder.external::<()>();
        let p1 = builder.process::<()>();

        // Create a singleton: fold 0..5 => 10
        let my_count = p1
            .source_iter(q!(0..5i32))
            .fold(q!(|| 0i32), q!(|acc: &mut i32, x| *acc += x));

        let count_ref = my_count.by_ref();

        // Use the singleton ref in a map on another stream
        let out_port = p1
            .source_iter(q!(1..=3i32))
            .map(q!(|x| x + *count_ref))
            .send_bincode_external(&external);

        let nodes = builder
            .with_default_optimize()
            .with_process(&p1, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut out_recv = nodes.connect(out_port).await;

        deployment.start().await.unwrap();

        let mut results = Vec::new();
        for _ in 0..3 {
            results.push(out_recv.next().await.unwrap());
        }
        results.sort();
        // fold(0..5) = 10, so results should be 11, 12, 13
        assert_eq!(results, vec![11, 12, 13]);
    }

    /// Test: same singleton ref used in multiple map operators.
    #[tokio::test]
    async fn test_singleton_ref_multiple_uses() {
        let mut deployment = Deployment::new();

        let mut builder = FlowBuilder::new();
        let external = builder.external::<()>();
        let p1 = builder.process::<()>();

        let my_count = p1
            .source_iter(q!(0..5i32))
            .fold(q!(|| 0i32), q!(|acc: &mut i32, x| *acc += x));

        let count_ref = my_count.by_ref();

        // Use the same singleton ref in two different maps
        let out_port1 = p1
            .source_iter(q!(1..=3i32))
            .map(q!(|x| x + *count_ref))
            .send_bincode_external(&external);

        let out_port2 = p1
            .source_iter(q!(10..=12i32))
            .map(q!(|x| x * *count_ref))
            .send_bincode_external(&external);

        let nodes = builder
            .with_default_optimize()
            .with_process(&p1, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut out_recv1 = nodes.connect(out_port1).await;
        let mut out_recv2 = nodes.connect(out_port2).await;

        deployment.start().await.unwrap();

        let mut results1 = Vec::new();
        for _ in 0..3 {
            results1.push(out_recv1.next().await.unwrap());
        }
        results1.sort();
        // fold(0..5) = 10, so results should be 11, 12, 13
        assert_eq!(results1, vec![11, 12, 13]);

        let mut results2 = Vec::new();
        for _ in 0..3 {
            results2.push(out_recv2.next().await.unwrap());
        }
        results2.sort();
        // fold(0..5) = 10, so results should be 100, 110, 120
        assert_eq!(results2, vec![100, 110, 120]);
    }

    #[tokio::test]
    async fn test_singleton_ref_non_copy() {
        let mut deployment = Deployment::new();

        let mut builder = FlowBuilder::new();
        let external = builder.external::<()>();
        let p1 = builder.process::<()>();

        // Create a singleton: fold into a Vec
        let my_vec = p1.source_iter(q!(0..5i32)).fold(
            q!(|| Vec::<i32>::new()),
            q!(|acc: &mut Vec<i32>, x| acc.push(x)),
        );

        let vec_ref = my_vec.by_ref();

        // Use the singleton ref to get the vec's length
        let out_port = p1
            .source_iter(q!(1..=3i32))
            .map(q!(|x| x + vec_ref.len() as i32))
            .send_bincode_external(&external);

        let nodes = builder
            .with_default_optimize()
            .with_process(&p1, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut out_recv = nodes.connect(out_port).await;

        deployment.start().await.unwrap();

        let mut results = Vec::new();
        for _ in 0..3 {
            results.push(out_recv.next().await.unwrap());
        }
        results.sort();
        // vec has 5 elements, so results should be 1+5, 2+5, 3+5
        assert_eq!(results, vec![6, 7, 8]);
    }

    /// Test: by_ref() + consume via into_stream() on the same singleton.
    #[tokio::test]
    async fn test_singleton_ref_and_consume() {
        let mut deployment = Deployment::new();

        let mut builder = FlowBuilder::new();
        let external = builder.external::<()>();
        let p1 = builder.process::<()>();

        let my_vec = p1.source_iter(q!(0..5i32)).fold(
            q!(|| Vec::<i32>::new()),
            q!(|acc: &mut Vec<i32>, x| acc.push(x)),
        );

        let vec_ref = my_vec.by_ref();

        // Reference path: use vec_ref.len()
        let out_port = p1
            .source_iter(q!(1..=3i32))
            .map(q!(|x| x + vec_ref.len() as i32))
            .send_bincode_external(&external);

        // Consume path: pipe the singleton value
        let out_port2 = my_vec.into_stream().send_bincode_external(&external);

        let nodes = builder
            .with_default_optimize()
            .with_process(&p1, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut out_recv = nodes.connect(out_port).await;
        let mut out_recv2 = nodes.connect(out_port2).await;

        deployment.start().await.unwrap();

        let mut results = Vec::new();
        for _ in 0..3 {
            results.push(out_recv.next().await.unwrap());
        }
        results.sort();
        assert_eq!(results, vec![6, 7, 8]);

        let mut consumed: Vec<i32> = out_recv2.next().await.unwrap();
        consumed.sort();
        assert_eq!(consumed, vec![0, 1, 2, 3, 4]);
    }

    /// Test: two different singleton refs captured in the same closure.
    #[tokio::test]
    async fn test_singleton_ref_two_refs_one_closure() {
        let mut deployment = Deployment::new();

        let mut builder = FlowBuilder::new();
        let external = builder.external::<()>();
        let p1 = builder.process::<()>();

        // Singleton A: fold 0..5 => 10
        let sum = p1
            .source_iter(q!(0..5i32))
            .fold(q!(|| 0i32), q!(|acc: &mut i32, x| *acc += x));

        // Singleton B: fold 10..13 => 33
        let sum2 = p1
            .source_iter(q!(10..13i32))
            .fold(q!(|| 0i32), q!(|acc: &mut i32, x| *acc += x));

        let ref_a = sum.by_ref();
        let ref_b = sum2.by_ref();

        // Capture both refs in one closure
        let out_port = p1
            .source_iter(q!(1..=2i32))
            .map(q!(|x| x + *ref_a + *ref_b))
            .send_bincode_external(&external);

        let nodes = builder
            .with_default_optimize()
            .with_process(&p1, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut out_recv = nodes.connect(out_port).await;

        deployment.start().await.unwrap();

        let mut results = Vec::new();
        for _ in 0..2 {
            results.push(out_recv.next().await.unwrap());
        }
        results.sort();
        // ref_a = 10, ref_b = 33, so results = 1+10+33=44, 2+10+33=45
        assert_eq!(results, vec![44, 45]);
    }

    /// Test: singleton ref inside a filter closure.
    #[tokio::test]
    async fn test_singleton_ref_filter() {
        let mut deployment = Deployment::new();

        let mut builder = FlowBuilder::new();
        let external = builder.external::<()>();
        let p1 = builder.process::<()>();

        // Singleton: fold 0..5 => 10 (threshold)
        let threshold = p1
            .source_iter(q!(0..5i32))
            .fold(q!(|| 0i32), q!(|acc: &mut i32, x| *acc += x));
        let threshold_ref = threshold.by_ref();

        // Filter: keep only elements > threshold (10)
        let out_port = p1
            .source_iter(q!(vec![5i32, 8, 11, 15, 3]))
            .filter(q!(|x| *x > *threshold_ref))
            .send_bincode_external(&external);

        threshold.into_stream().for_each(q!(|_| {}));

        let nodes = builder
            .with_default_optimize()
            .with_process(&p1, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut out_recv = nodes.connect(out_port).await;

        deployment.start().await.unwrap();

        let mut results = Vec::new();
        for _ in 0..2 {
            results.push(out_recv.next().await.unwrap());
        }
        results.sort();
        // threshold = 10, so only 11 and 15 pass
        assert_eq!(results, vec![11, 15]);
    }

    /// Test: singleton ref inside a partition closure.
    /// partition is unique because it has a closure but produces two consumer streams,
    /// so the singleton ref must be correctly shared across both output branches.
    #[tokio::test]
    async fn test_singleton_ref_partition() {
        let mut deployment = Deployment::new();

        let mut builder = FlowBuilder::new();
        let external = builder.external::<()>();
        let p1 = builder.process::<()>();

        // Singleton: fold 0..5 => 10 (threshold)
        let threshold = p1
            .source_iter(q!(0..5i32))
            .fold(q!(|| 0i32), q!(|acc: &mut i32, x| *acc += x));
        let threshold_ref = threshold.by_ref();

        // Partition: elements > threshold go to true branch, others to false branch
        let (above, below) = p1
            .source_iter(q!(vec![5i32, 8, 10, 11, 15, 3]))
            .partition(q!(|x| *x > *threshold_ref));

        let out_above = above.send_bincode_external(&external);
        let out_below = below.send_bincode_external(&external);

        threshold.into_stream().for_each(q!(|_| {}));

        let nodes = builder
            .with_default_optimize()
            .with_process(&p1, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut recv_above = nodes.connect(out_above).await;
        let mut recv_below = nodes.connect(out_below).await;

        deployment.start().await.unwrap();

        let mut results_above = Vec::new();
        for _ in 0..2 {
            results_above.push(recv_above.next().await.unwrap());
        }
        results_above.sort();
        // threshold = 10, elements strictly > 10 are 11 and 15
        assert_eq!(results_above, vec![11, 15]);

        let mut results_below = Vec::new();
        for _ in 0..4 {
            results_below.push(recv_below.next().await.unwrap());
        }
        results_below.sort();
        // elements <= 10 are 3, 5, 8, 10
        assert_eq!(results_below, vec![3, 5, 8, 10]);
    }

    /// Test: singleton ref in partition with downstream map operators on both branches.
    ///
    /// This specifically exercises the ident_stack pop logic in the "already built" path
    /// of Partition code generation (lines 3462-3466 in ir/mod.rs). When the second branch
    /// of a partition is processed, transform_children pushes singleton ref idents onto the
    /// stack, but since the partition was already built by the first branch, those idents
    /// must be popped to keep the stack consistent for downstream operators.
    ///
    /// Without the pop, the ident_stack would be corrupted and downstream operators (the
    /// maps on each branch) would read wrong idents, causing a compile/runtime failure.
    #[tokio::test]
    async fn test_singleton_ref_partition_with_downstream_ops() {
        let mut deployment = Deployment::new();

        let mut builder = FlowBuilder::new();
        let external = builder.external::<()>();
        let p1 = builder.process::<()>();

        // Singleton: fold 0..5 => 10
        let threshold = p1
            .source_iter(q!(0..5i32))
            .fold(q!(|| 0i32), q!(|acc: &mut i32, x| *acc += x));
        let threshold_ref = threshold.by_ref();

        // Partition using the singleton ref
        let (above, below) = p1
            .source_iter(q!(vec![5i32, 8, 11, 15]))
            .partition(q!(|x| *x > *threshold_ref));

        // Apply downstream operators on BOTH branches — this is the key part.
        // If the ident_stack pop is missing, these maps will get wrong idents.
        let out_above = above.map(q!(|x| x * 2)).send_bincode_external(&external);
        let out_below = below.map(q!(|x| x + 100)).send_bincode_external(&external);

        threshold.into_stream().for_each(q!(|_| {}));

        let nodes = builder
            .with_default_optimize()
            .with_process(&p1, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut recv_above = nodes.connect(out_above).await;
        let mut recv_below = nodes.connect(out_below).await;

        deployment.start().await.unwrap();

        let mut results_above = Vec::new();
        for _ in 0..2 {
            results_above.push(recv_above.next().await.unwrap());
        }
        results_above.sort();
        // threshold = 10, above = [11, 15], mapped: [22, 30]
        assert_eq!(results_above, vec![22, 30]);

        let mut results_below = Vec::new();
        for _ in 0..2 {
            results_below.push(recv_below.next().await.unwrap());
        }
        results_below.sort();
        // below = [5, 8], mapped: [105, 108]
        assert_eq!(results_below, vec![105, 108]);
    }

    /// Test: singleton ref in partition where the false branch is chained with another stream.
    ///
    /// This creates a scenario where the stale singleton ref ident left on the ident_stack
    /// (if the pop is missing) would be incorrectly consumed by the chain operator,
    /// causing a compilation or runtime failure.
    #[tokio::test]
    async fn test_singleton_ref_partition_chain_false_branch() {
        let mut deployment = Deployment::new();

        let mut builder = FlowBuilder::new();
        let external = builder.external::<()>();
        let p1 = builder.process::<()>();

        // Singleton: fold 0..5 => 10
        let threshold = p1
            .source_iter(q!(0..5i32))
            .fold(q!(|| 0i32), q!(|acc: &mut i32, x| *acc += x));
        let threshold_ref = threshold.by_ref();

        // Partition using the singleton ref
        let (above, below) = p1
            .source_iter(q!(vec![5i32, 8, 11, 15]))
            .partition(q!(|x| *x > *threshold_ref));

        // Chain the false branch with another stream — this forces the chain operator
        // to pop two idents from the stack. If the singleton ref ident wasn't popped,
        // the chain would get the wrong ident.
        let extra_stream = p1.source_iter(q!(vec![99i32]));
        let out_above = above.send_bincode_external(&external);
        let out_below = below.chain(extra_stream).send_bincode_external(&external);

        threshold.into_stream().for_each(q!(|_| {}));

        let nodes = builder
            .with_default_optimize()
            .with_process(&p1, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut recv_above = nodes.connect(out_above).await;
        let mut recv_below = nodes.connect(out_below).await;

        deployment.start().await.unwrap();

        let mut results_above = Vec::new();
        for _ in 0..2 {
            results_above.push(recv_above.next().await.unwrap());
        }
        results_above.sort();
        // threshold = 10, above = [11, 15]
        assert_eq!(results_above, vec![11, 15]);

        let mut results_below = Vec::new();
        for _ in 0..3 {
            results_below.push(recv_below.next().await.unwrap());
        }
        results_below.sort();
        // below = [5, 8] chained with [99]
        assert_eq!(results_below, vec![5, 8, 99]);
    }

    /// Test: singleton ref inside a flat_map closure.
    #[tokio::test]
    async fn test_singleton_ref_flat_map() {
        let mut deployment = Deployment::new();

        let mut builder = FlowBuilder::new();
        let external = builder.external::<()>();
        let p1 = builder.process::<()>();

        // Singleton: fold 0..3 => 3 (repeat count)
        let count = p1
            .source_iter(q!(0..3i32))
            .fold(q!(|| 0i32), q!(|acc: &mut i32, _| *acc += 1));
        let count_ref = count.by_ref();

        // flat_map: for each element, produce a vec of `count` copies
        let out_port = p1
            .source_iter(q!(vec![10i32, 20]))
            .flat_map_ordered(q!(|x| {
                let n = *count_ref as usize;
                let mut v = Vec::new();
                for _ in 0..n {
                    v.push(x);
                }
                v
            }))
            .send_bincode_external(&external);

        count.into_stream().for_each(q!(|_| {}));

        let nodes = builder
            .with_default_optimize()
            .with_process(&p1, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut out_recv = nodes.connect(out_port).await;

        deployment.start().await.unwrap();

        let mut results = Vec::new();
        for _ in 0..6 {
            results.push(out_recv.next().await.unwrap());
        }
        results.sort();
        // count = 3, so [10, 10, 10, 20, 20, 20]
        assert_eq!(results, vec![10, 10, 10, 20, 20, 20]);
    }
}
