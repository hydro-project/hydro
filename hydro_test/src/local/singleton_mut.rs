#[cfg(test)]
mod tests {
    use futures::StreamExt;
    use hydro_deploy::Deployment;
    use hydro_lang::compile::builder::FlowBuilder;
    use hydro_lang::location::Location;
    use stageleft::q;

    #[tokio::test]
    async fn test_singleton_mut() {
        let mut deployment = Deployment::new();

        let mut builder = FlowBuilder::new();
        let external = builder.external::<()>();
        let p1 = builder.process::<()>();

        // Create a singleton: fold 0..5 => 10
        let my_count = p1
            .source_iter(q!(0..5i32))
            .fold(q!(|| 0i32), q!(|acc: &mut i32, x| *acc += x));

        let count_mut = my_count.by_mut();

        // Use the singleton mut in a map on another stream
        let out_port = p1
            .source_iter(q!(1..=3i32))
            .weaken_ordering::<hydro_lang::live_collections::stream::NoOrder>()
            .map(q!(
                |x| {
                    *count_mut += x;
                    *count_mut
                },
                commutative = hydro_lang::__manual_proof__!(/** addition is commutative */)
            ))
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
        assert_eq!(results, vec![11, 13, 16]);
    }
}
