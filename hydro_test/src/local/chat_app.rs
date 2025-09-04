use hydro_lang::live_collections::stream::NoOrder;
use hydro_lang::nondet::NonDet;
use hydro_lang::prelude::*;

pub fn chat_app<'a>(
    process: &Process<'a>,
    users_stream: Stream<u32, Process<'a>, Unbounded>,
    messages: Stream<String, Process<'a>, Unbounded>,
    replay_messages: bool,
    // intentionally non-deterministic to not send messages to users that joined after the message was sent
    nondet_user_arrival_broadcast: NonDet,
) -> Stream<(u32, String), Process<'a>, Unbounded, NoOrder> {
    let tick = process.tick();

    let users = users_stream
        .batch(&tick, nondet_user_arrival_broadcast)
        .persist();

    let messages = if replay_messages {
        messages
            .batch(&tick, nondet_user_arrival_broadcast)
            .persist()
    } else {
        messages.batch(&tick, nondet_user_arrival_broadcast)
    };

    // do this after the persist to test pullup
    let messages = messages.map(q!(|s| s.to_uppercase()));

    let mut joined = users.cross_product(messages);
    if replay_messages {
        joined = joined.delta();
    }

    joined.all_ticks()
}

#[cfg(test)]
mod tests {
    use futures::{SinkExt, Stream, StreamExt};
    use hydro_deploy::Deployment;
    use hydro_lang::location::Location;
    use hydro_lang::nondet::nondet;

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
    async fn test_chat_app_no_replay() {
        let mut deployment = Deployment::new();

        let builder = hydro_lang::builder::FlowBuilder::new();
        let external = builder.external::<()>();
        let p1 = builder.process();

        let (users_send, users) = p1.source_external_bincode(&external);
        let (messages_send, messages) = p1.source_external_bincode(&external);
        let out = super::chat_app(&p1, users, messages, false, nondet!(/** test */));
        let out_recv = out.send_bincode_external(&external);

        let built = builder.with_default_optimize();

        hydro_build_utils::assert_snapshot!(
            built
                .preview_compile()
                .dfir_for(&p1)
                .to_mermaid(&Default::default())
        );

        let nodes = built
            .with_process(&p1, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut users_send = nodes.connect_sink_bincode(users_send).await;
        let mut messages_send = nodes.connect_sink_bincode(messages_send).await;
        let mut out_recv = nodes.connect_source_bincode(out_recv).await;

        deployment.start().await.unwrap();

        users_send.send(1).await.unwrap();
        users_send.send(2).await.unwrap();

        messages_send.send("hello".to_string()).await.unwrap();
        messages_send.send("world".to_string()).await.unwrap();

        assert_eq!(
            take_next_n(&mut out_recv, 4).await,
            &[
                (1, "HELLO".to_string()),
                (2, "HELLO".to_string()),
                (1, "WORLD".to_string()),
                (2, "WORLD".to_string())
            ]
        );

        users_send.send(3).await.unwrap();

        messages_send.send("goodbye".to_string()).await.unwrap();

        assert_eq!(
            take_next_n(&mut out_recv, 3).await,
            &[
                (1, "GOODBYE".to_string()),
                (2, "GOODBYE".to_string()),
                (3, "GOODBYE".to_string())
            ]
        );
    }

    #[tokio::test]
    async fn test_chat_app_replay() {
        let mut deployment = Deployment::new();

        let builder = hydro_lang::builder::FlowBuilder::new();
        let external = builder.external::<()>();
        let p1 = builder.process();

        let (users_send, users) = p1.source_external_bincode(&external);
        let (messages_send, messages) = p1.source_external_bincode(&external);
        let out = super::chat_app(&p1, users, messages, true, nondet!(/** test */));
        let out_recv = out.send_bincode_external(&external);

        let built = builder.with_default_optimize();

        hydro_build_utils::assert_snapshot!(
            built
                .preview_compile()
                .dfir_for(&p1)
                .to_mermaid(&Default::default())
        );

        let nodes = built
            .with_process(&p1, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut users_send = nodes.connect_sink_bincode(users_send).await;
        let mut messages_send = nodes.connect_sink_bincode(messages_send).await;
        let mut out_recv = nodes.connect_source_bincode(out_recv).await;

        deployment.start().await.unwrap();

        users_send.send(1).await.unwrap();
        users_send.send(2).await.unwrap();

        messages_send.send("hello".to_string()).await.unwrap();
        messages_send.send("world".to_string()).await.unwrap();

        assert_eq!(
            take_next_n(&mut out_recv, 4).await,
            &[
                (1, "HELLO".to_string()),
                (2, "HELLO".to_string()),
                (1, "WORLD".to_string()),
                (2, "WORLD".to_string())
            ]
        );

        users_send.send(3).await.unwrap();

        messages_send.send("goodbye".to_string()).await.unwrap();

        assert_eq!(
            take_next_n(&mut out_recv, 5).await,
            &[
                (3, "HELLO".to_string()),
                (3, "WORLD".to_string()),
                (1, "GOODBYE".to_string()),
                (2, "GOODBYE".to_string()),
                (3, "GOODBYE".to_string())
            ]
        );
    }
}
