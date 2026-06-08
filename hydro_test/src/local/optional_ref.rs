#[cfg(test)]
mod tests {
    use futures::StreamExt;
    use hydro_deploy::Deployment;
    use hydro_lang::compile::builder::FlowBuilder;
    use hydro_lang::location::Location;
    use stageleft::q;

    #[tokio::test]
    async fn test_optional_ref() {
        let mut deployment = Deployment::new();

        let mut builder = FlowBuilder::new();
        let external = builder.external::<()>();
        let p1 = builder.process::<()>();

        // Create an optional: reduce 0..5 => Some(10) (sum via reduce)
        let my_opt = p1.source_iter(q!(0..5i32)).reduce(q!(|a, b| *a += b));

        let opt_ref = my_opt.by_ref();

        // Use the optional ref in a map: unwrap_or(0) + x
        let out_port = p1
            .source_iter(q!(1..=3i32))
            .map(q!(|x| x + opt_ref.unwrap_or(0)))
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
        // reduce(0..5) = 10, so results should be 11, 12, 13
        assert_eq!(results, vec![11, 12, 13]);
    }

    #[tokio::test]
    async fn test_optional_ref_none() {
        let mut deployment = Deployment::new();

        let mut builder = FlowBuilder::new();
        let external = builder.external::<()>();
        let p1 = builder.process::<()>();

        // Create an optional from an empty source => None
        let my_opt = p1
            .source_iter(q!(std::iter::empty::<i32>()))
            .reduce(q!(|a, b| *a += b));

        let opt_ref = my_opt.by_ref();

        // Use the optional ref: should be None, so unwrap_or(99)
        let out_port = p1
            .source_iter(q!(1..=2i32))
            .map(q!(|x| x + opt_ref.unwrap_or(99)))
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
        // optional is None, so unwrap_or(99) => 100, 101
        assert_eq!(results, vec![100, 101]);
    }

    #[tokio::test]
    async fn test_optional_ref_and_consume() {
        let mut deployment = Deployment::new();

        let mut builder = FlowBuilder::new();
        let external = builder.external::<()>();
        let p1 = builder.process::<()>();

        // Use fold (produces Singleton) -> into Optional, to avoid cycle issues
        let my_opt = p1.source_iter(q!(0..5i32)).reduce(q!(|a, b| *a += b));

        let opt_ref = my_opt.by_ref();

        // Reference path
        let out_port_ref = p1
            .source_iter(q!(1..=2i32))
            .map(q!(|x| x + opt_ref.unwrap_or(0)))
            .send_bincode_external(&external);

        let nodes = builder
            .with_default_optimize()
            .with_process(&p1, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut out_recv_ref = nodes.connect(out_port_ref).await;

        deployment.start().await.unwrap();

        let mut ref_results = Vec::new();
        for _ in 0..2 {
            ref_results.push(out_recv_ref.next().await.unwrap());
        }
        ref_results.sort();
        // reduce(0..5) = 10, so 1+10=11, 2+10=12
        assert_eq!(ref_results, vec![11, 12]);
    }
}
