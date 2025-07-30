use hydro_lang::*;

pub fn test_difference<'a>(
    process: &Process<'a>,
    persist1: bool,
    persist2: bool,
    tick_trigger: Stream<(), Process<'a>, Unbounded>,
) -> Stream<u32, Process<'a>, Unbounded> {
    let tick = process.tick();

    let mut source = unsafe {
        // SAFETY: intentionally using ticks
        process
            .source_iter(q!(0..5))
            .tick_batch(&tick)
            .continue_if(tick_trigger.clone().tick_batch(&tick).first())
    };
    if persist1 {
        source = source.persist();
    }

    let mut source2 = unsafe {
        // SAFETY: intentionally using ticks
        process
            .source_iter(q!(3..6))
            .tick_batch(&tick)
            .continue_if(tick_trigger.clone().tick_batch(&tick).first())
    };
    if persist2 {
        source2 = source2.persist();
    }

    source
        .filter_not_in(source2)
        .continue_if(unsafe { tick_trigger.tick_batch(&tick).first() })
        .all_ticks()
}

pub fn test_anti_join<'a>(
    process: &Process<'a>,
    persist1: bool,
    persist2: bool,
    tick_trigger: Stream<(), Process<'a>, Unbounded>,
) -> Stream<u32, Process<'a>, Unbounded> {
    let tick = process.tick();

    let mut source = unsafe {
        // SAFETY: intentionally using ticks
        process
            .source_iter(q!(0..5))
            .map(q!(|v| (v, v)))
            .tick_batch(&tick)
            .continue_if(tick_trigger.clone().tick_batch(&tick).first())
    };
    if persist1 {
        source = source.persist();
    }

    let mut source2 = unsafe {
        // SAFETY: intentionally using ticks
        process
            .source_iter(q!(3..6))
            .tick_batch(&tick)
            .continue_if(tick_trigger.clone().tick_batch(&tick).first())
    };
    if persist2 {
        source2 = source2.persist();
    }

    source
        .anti_join(source2)
        .continue_if(unsafe { tick_trigger.tick_batch(&tick).first() })
        .all_ticks()
        .map(q!(|v| v.0))
}

#[cfg(test)]
mod tests {
    use futures::{SinkExt, Stream, StreamExt};
    use hydro_deploy::Deployment;
    use hydro_lang::Location;

    async fn take_next_n<T>(stream: &mut (impl Stream<Item = T> + Unpin), n: usize) -> Vec<T> {
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            if let Some(item) = stream.next().await {
                out.push(item);
            } else {
                panic!();
            }
        }
        out
    }

    #[tokio::test]
    async fn test_difference_tick_tick() {
        let mut deployment = Deployment::new();

        let builder = hydro_lang::FlowBuilder::new();
        let external = builder.external::<()>();
        let p1 = builder.process();

        let (tick_send, tick_trigger) = p1.source_external_bincode(&external);

        let out = super::test_difference(&p1, false, false, tick_trigger);
        let out_recv = out.send_bincode_external(&external);

        let built = builder.with_default_optimize();
        let nodes = built
            .with_process(&p1, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut tick_send = nodes.connect_sink_bincode(tick_send).await;
        let out_recv = nodes.connect_source_bincode(out_recv).await;

        tick_send.send(()).await.unwrap();

        deployment.start().await.unwrap();

        assert_eq!(out_recv.take(3).collect::<Vec<_>>().await, &[0, 1, 2]);
    }

    #[tokio::test]
    async fn test_difference_tick_static() {
        let mut deployment = Deployment::new();

        let builder = hydro_lang::FlowBuilder::new();
        let external = builder.external::<()>();
        let p1 = builder.process();

        let (tick_send, tick_trigger) = p1.source_external_bincode(&external);

        let out = super::test_difference(&p1, false, true, tick_trigger);
        let out_recv = out.send_bincode_external(&external);

        let built = builder.with_default_optimize();
        let nodes = built
            .with_process(&p1, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut tick_send = nodes.connect_sink_bincode(tick_send).await;
        let out_recv = nodes.connect_source_bincode(out_recv).await;

        tick_send.send(()).await.unwrap();

        deployment.start().await.unwrap();

        assert_eq!(out_recv.take(3).collect::<Vec<_>>().await, &[0, 1, 2]);
    }

    #[tokio::test]
    async fn test_difference_static_tick() {
        let mut deployment = Deployment::new();

        let builder = hydro_lang::FlowBuilder::new();
        let external = builder.external::<()>();
        let p1 = builder.process();

        let (tick_send, tick_trigger) = p1.source_external_bincode(&external);

        let out = super::test_difference(&p1, true, false, tick_trigger);
        let out_recv = out.send_bincode_external(&external);

        let built = builder.with_default_optimize();
        let nodes = built
            .with_process(&p1, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut tick_send = nodes.connect_sink_bincode(tick_send).await;
        let mut out_recv = nodes.connect_source_bincode(out_recv).await;

        tick_send.send(()).await.unwrap();

        deployment.start().await.unwrap();

        assert_eq!(take_next_n(&mut out_recv, 3).await, &[0, 1, 2]);

        tick_send.send(()).await.unwrap();

        assert_eq!(take_next_n(&mut out_recv, 5).await, &[0, 1, 2, 3, 4]);
    }

    #[tokio::test]
    async fn test_difference_static_static() {
        let mut deployment = Deployment::new();

        let builder = hydro_lang::FlowBuilder::new();
        let external = builder.external::<()>();
        let p1 = builder.process();

        let (tick_send, tick_trigger) = p1.source_external_bincode(&external);

        let out = super::test_difference(&p1, true, true, tick_trigger);
        let out_recv = out.send_bincode_external(&external);

        let built = builder.with_default_optimize();
        let nodes = built
            .with_process(&p1, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut tick_send = nodes.connect_sink_bincode(tick_send).await;
        let mut out_recv = nodes.connect_source_bincode(out_recv).await;

        tick_send.send(()).await.unwrap();

        deployment.start().await.unwrap();

        assert_eq!(take_next_n(&mut out_recv, 3).await, &[0, 1, 2]);

        tick_send.send(()).await.unwrap();

        assert_eq!(take_next_n(&mut out_recv, 3).await, &[0, 1, 2]);
    }

    #[tokio::test]
    async fn test_anti_join_tick_tick() {
        let mut deployment = Deployment::new();

        let builder = hydro_lang::FlowBuilder::new();
        let external = builder.external::<()>();
        let p1 = builder.process();

        let (tick_send, tick_trigger) = p1.source_external_bincode(&external);

        let out = super::test_anti_join(&p1, false, false, tick_trigger);
        let out_recv = out.send_bincode_external(&external);

        let built = builder.with_default_optimize();
        let nodes = built
            .with_process(&p1, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut tick_send = nodes.connect_sink_bincode(tick_send).await;
        let out_recv = nodes.connect_source_bincode(out_recv).await;

        tick_send.send(()).await.unwrap();

        deployment.start().await.unwrap();

        assert_eq!(out_recv.take(3).collect::<Vec<_>>().await, &[0, 1, 2]);
    }

    #[tokio::test]
    async fn test_anti_join_tick_static() {
        let mut deployment = Deployment::new();

        let builder = hydro_lang::FlowBuilder::new();
        let external = builder.external::<()>();
        let p1 = builder.process();

        let (tick_send, tick_trigger) = p1.source_external_bincode(&external);

        let out = super::test_anti_join(&p1, false, true, tick_trigger);
        let out_recv = out.send_bincode_external(&external);

        let built = builder.with_default_optimize();
        let nodes = built
            .with_process(&p1, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut tick_send = nodes.connect_sink_bincode(tick_send).await;
        let out_recv = nodes.connect_source_bincode(out_recv).await;

        tick_send.send(()).await.unwrap();

        deployment.start().await.unwrap();

        assert_eq!(out_recv.take(3).collect::<Vec<_>>().await, &[0, 1, 2]);
    }

    #[tokio::test]
    async fn test_anti_join_static_tick() {
        let mut deployment = Deployment::new();

        let builder = hydro_lang::FlowBuilder::new();
        let external = builder.external::<()>();
        let p1 = builder.process();

        let (tick_send, tick_trigger) = p1.source_external_bincode(&external);

        let out = super::test_anti_join(&p1, true, false, tick_trigger);
        let out_recv = out.send_bincode_external(&external);

        let built = builder.with_default_optimize();
        let nodes = built
            .with_process(&p1, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut tick_send = nodes.connect_sink_bincode(tick_send).await;
        let mut out_recv = nodes.connect_source_bincode(out_recv).await;

        tick_send.send(()).await.unwrap();

        deployment.start().await.unwrap();

        assert_eq!(take_next_n(&mut out_recv, 3).await, &[0, 1, 2]);

        tick_send.send(()).await.unwrap();

        assert_eq!(take_next_n(&mut out_recv, 5).await, &[0, 1, 2, 3, 4]);
    }

    #[tokio::test]
    async fn test_anti_join_static_static() {
        let mut deployment = Deployment::new();

        let builder = hydro_lang::FlowBuilder::new();
        let external = builder.external::<()>();
        let p1 = builder.process();

        let (tick_send, tick_trigger) = p1.source_external_bincode(&external);

        let out = super::test_anti_join(&p1, true, true, tick_trigger);
        let out_recv = out.send_bincode_external(&external);

        let built = builder.with_default_optimize();
        let nodes = built
            .with_process(&p1, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut tick_send = nodes.connect_sink_bincode(tick_send).await;
        let mut out_recv = nodes.connect_source_bincode(out_recv).await;

        tick_send.send(()).await.unwrap();

        deployment.start().await.unwrap();

        assert_eq!(take_next_n(&mut out_recv, 3).await, &[0, 1, 2]);

        tick_send.send(()).await.unwrap();

        assert_eq!(take_next_n(&mut out_recv, 3).await, &[0, 1, 2]);
    }
}
