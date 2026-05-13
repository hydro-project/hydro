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
        let out_port2 = my_vec
            .into_stream()
            .send_bincode_external(&external);

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
}
