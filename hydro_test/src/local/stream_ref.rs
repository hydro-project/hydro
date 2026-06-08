#[cfg(test)]
mod tests {
    use futures::StreamExt;
    use hydro_deploy::Deployment;
    use hydro_lang::compile::builder::FlowBuilder;
    use hydro_lang::location::Location;
    use stageleft::q;

    #[tokio::test]
    async fn test_stream_ref() {
        let mut deployment = Deployment::new();

        let mut builder = FlowBuilder::new();
        let external = builder.external::<()>();
        let p1 = builder.process::<()>();

        // Create a bounded stream (source_iter is bounded within a tick)
        let my_stream = p1.source_iter(q!(1..=5i32));

        let stream_ref = my_stream.by_ref();

        // Use the stream ref to get the vec's length
        let out_port = p1
            .source_iter(q!([()]))
            .map(q!(|_| stream_ref.len() as i32))
            .send_bincode_external(&external);

        // Also consume the stream via pipe
        my_stream.for_each(q!(|_| {}));

        let nodes = builder
            .with_default_optimize()
            .with_process(&p1, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut out_recv = nodes.connect(out_port).await;

        deployment.start().await.unwrap();

        let result = out_recv.next().await.unwrap();
        // stream has 5 elements
        assert_eq!(result, 5);
    }

    #[tokio::test]
    async fn test_stream_ref_contents() {
        let mut deployment = Deployment::new();

        let mut builder = FlowBuilder::new();
        let external = builder.external::<()>();
        let p1 = builder.process::<()>();

        // Create a bounded stream
        let my_stream = p1.source_iter(q!(1..=3i32));

        let stream_ref = my_stream.by_ref();

        // Sum the referenced vec's contents
        let out_port = p1
            .source_iter(q!([()]))
            .map(q!(|_| stream_ref.iter().sum::<i32>()))
            .send_bincode_external(&external);

        my_stream.for_each(q!(|_| {}));

        let nodes = builder
            .with_default_optimize()
            .with_process(&p1, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut out_recv = nodes.connect(out_port).await;

        deployment.start().await.unwrap();

        let result = out_recv.next().await.unwrap();
        // sum of 1+2+3 = 6
        assert_eq!(result, 6);
    }

    #[tokio::test]
    async fn test_stream_ref_no_consumer() {
        let mut deployment = Deployment::new();

        let mut builder = FlowBuilder::new();
        let external = builder.external::<()>();
        let p1 = builder.process::<()>();

        // Create a bounded stream — no pipe consumer, only ref
        let my_stream = p1.source_iter(q!(1..=4i32));

        let stream_ref = my_stream.by_ref();

        let out_port = p1
            .source_iter(q!([()]))
            .map(q!(|_| stream_ref.len() as i32))
            .send_bincode_external(&external);

        let nodes = builder
            .with_default_optimize()
            .with_process(&p1, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut out_recv = nodes.connect(out_port).await;

        deployment.start().await.unwrap();

        let result = out_recv.next().await.unwrap();
        assert_eq!(result, 4);
    }

    #[tokio::test]
    async fn test_stream_mut() {
        let mut deployment = Deployment::new();

        let mut builder = FlowBuilder::new();
        let external = builder.external::<()>();
        let p1 = builder.process::<()>();

        // Create a bounded stream
        let my_stream = p1.source_iter(q!(1..=5i32));

        let stream_mut = my_stream.by_mut();

        // Mutably reference the buffer to retain only items > 3
        let out_port = p1
            .source_iter(q!([()]))
            .map(q!(|_| {
                stream_mut.retain(|x| *x > 3);
                stream_mut.len() as i32
            }))
            .send_bincode_external(&external);

        my_stream.for_each(q!(|_| {}));

        let nodes = builder
            .with_default_optimize()
            .with_process(&p1, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut out_recv = nodes.connect(out_port).await;

        deployment.start().await.unwrap();

        let result = out_recv.next().await.unwrap();
        // After retain(> 3): [4, 5] => len = 2
        assert_eq!(result, 2);
    }
}
