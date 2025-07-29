use hydro_lang::*;

pub fn count_elems<'a, T: 'a>(
    process: &Process<'a>,
    input_stream: Stream<T, Process<'a>, Unbounded>,
) -> Stream<u32, Process<'a>, Unbounded> {
    let tick = process.tick();

    let count = unsafe {
        // SAFETY: intentionally using ticks
        input_stream.map(q!(|_| 1)).tick_batch(&tick)
    }
    .fold(q!(|| 0), q!(|a, b| *a += b))
    .all_ticks();

    count
}

#[cfg(test)]
mod tests {
    use futures::{SinkExt, StreamExt};
    use hydro_deploy::Deployment;
    use hydro_lang::Location;

    #[tokio::test]
    async fn test_count() {
        let mut deployment = Deployment::new();

        let builder = hydro_lang::FlowBuilder::new();
        let external = builder.external::<()>();
        let p1 = builder.process();

        let (input_send, input) = p1.source_external_bincode(&external);
        let out = super::count_elems(&p1, input);
        let out_recv = out.send_bincode_external(&external);

        let built = builder.with_default_optimize();
        let nodes = built
            .with_process(&p1, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut input_send = nodes.connect_sink_bincode(input_send).await;
        let mut out_recv = nodes.connect_source_bincode(out_recv).await;

        // send before starting so everything shows up in single tick
        input_send.send(1).await.unwrap();
        input_send.send(1).await.unwrap();
        input_send.send(1).await.unwrap();

        deployment.start().await.unwrap();

        assert_eq!(out_recv.next().await.unwrap(), 0); // first tick (no data yet)
        assert_eq!(out_recv.next().await.unwrap(), 3); // second tick
    }
}
