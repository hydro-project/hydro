use hydro_lang::*;

pub fn graph_reachability<'a>(
    process: &Process<'a>,
    roots: Stream<u32, Process<'a>, Unbounded>,
    edges: Stream<(u32, u32), Process<'a>, Unbounded>,
) -> Stream<u32, Process<'a>, Unbounded, NoOrder> {
    let reachability_tick = process.tick();
    let (set_reached_cycle, reached_cycle) = reachability_tick.cycle::<Stream<_, _, _, NoOrder>>();

    let reached = unsafe {
        // SAFETY: roots can be inserted on any tick because we are fixpointing
        roots.tick_batch(&reachability_tick).chain(reached_cycle)
    };
    let reachable = reached
        .clone()
        .map(q!(|r| (r, ())))
        .join(unsafe {
            // SAFETY: edges can be inserted on any tick because we are fixpointing
            edges.tick_batch(&reachability_tick).persist()
        })
        .map(q!(|(_from, (_, to))| to));
    set_reached_cycle.complete_next_tick(reached.clone().chain(reachable));

    reached.all_ticks().unique()
}

#[cfg(test)]
mod tests {
    use futures::{SinkExt, StreamExt};
    use hydro_deploy::Deployment;

    #[tokio::test]
    #[ignore = "broken because ticks in Hydro are only triggered by external input"]
    async fn test_reachability() {
        let mut deployment = Deployment::new();

        let builder = hydro_lang::FlowBuilder::new();
        let external = builder.external_process::<()>();
        let p1 = builder.process();

        let (roots_send, roots) = external.source_external_bincode(&p1);
        let (edges_send, edges) = external.source_external_bincode(&p1);
        let out = super::graph_reachability(&p1, roots, edges);
        let out_recv = out.send_bincode_external(&external);

        let built = builder.with_default_optimize();

        println!(
            "{}",
            built
                .preview_compile()
                .dfir_for(&p1)
                .surface_syntax_string()
        );

        let nodes = built
            .with_process(&p1, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut roots_send = nodes.connect_sink_bincode(roots_send).await;
        let mut edges_send = nodes.connect_sink_bincode(edges_send).await;
        let out_recv = nodes.connect_source_bincode(out_recv).await;

        deployment.start().await.unwrap();

        roots_send.send(1).await.unwrap();
        roots_send.send(2).await.unwrap();

        edges_send.send((1, 2)).await.unwrap();
        edges_send.send((2, 3)).await.unwrap();
        edges_send.send((3, 4)).await.unwrap();
        edges_send.send((4, 5)).await.unwrap();

        assert_eq!(out_recv.take(5).collect::<Vec<_>>().await, &[1, 2, 3, 4, 5]);
    }
}
