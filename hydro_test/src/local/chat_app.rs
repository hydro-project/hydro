use hydro_lang::live_collections::stream::NoOrder;
use hydro_lang::nondet::NonDet;
use hydro_lang::prelude::*;

pub fn chat_app<'a>(
    users_stream: Stream<u32, Process<'a>, Unbounded>,
    messages: Stream<String, Process<'a>, Unbounded>,
    replay_messages: bool,
    // intentionally non-deterministic to not send messages to users that joined after the message was sent
    nondet_user_arrival_broadcast: NonDet,
) -> Stream<(u32, String), Process<'a>, Unbounded, NoOrder> {
    let messages = messages.map(q!(|s| s.to_uppercase()));
    if replay_messages {
        users_stream.cross_product(messages)
    } else {
        let current_users = users_stream.collect_vec();

        sliced! {
            let users = use(current_users, nondet_user_arrival_broadcast);
            let messages = use(messages, nondet_user_arrival_broadcast);

            users.flatten_ordered().cross_product(messages)
        }
    }
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

        let mut builder = hydro_lang::compile::builder::FlowBuilder::new();
        let external = builder.external::<()>();
        let p1 = builder.process();

        let (users_send, users) = p1.source_external_bincode(&external);
        let (messages_send, messages) = p1.source_external_bincode(&external);
        let out = super::chat_app(users, messages, false, nondet!(/** test */));
        let out_recv = out.send_bincode_external(&external);

        let mut built = builder.with_default_optimize();

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

        let mut users_send = nodes.connect(users_send).await;
        let mut messages_send = nodes.connect(messages_send).await;
        let mut out_recv = nodes.connect(out_recv).await;

        deployment.start().await.unwrap();

        users_send.send(1).await.unwrap();
        users_send.send(2).await.unwrap();

        messages_send.send("hello".to_owned()).await.unwrap();
        messages_send.send("world".to_owned()).await.unwrap();

        {
            let out = take_next_n(&mut out_recv, 4).await;
            let h1 = out.iter().position(|x| *x == (1, "HELLO".to_owned()));
            let h2 = out.iter().position(|x| *x == (2, "HELLO".to_owned()));
            let w1 = out.iter().position(|x| *x == (1, "WORLD".to_owned()));
            let w2 = out.iter().position(|x| *x == (2, "WORLD".to_owned()));
            // Assert all items exist.
            assert!(h1.is_some());
            assert!(h2.is_some());
            assert!(w1.is_some());
            assert!(w2.is_some());
            // Assert partial order is preserved
            assert!(h1.unwrap() < h2.unwrap());
            assert!(h1.unwrap() < w1.unwrap());
            assert!(h2.unwrap() < w2.unwrap());
            assert!(w1.unwrap() < w2.unwrap());
        }

        users_send.send(3).await.unwrap();

        messages_send.send("goodbye".to_owned()).await.unwrap();

        assert_eq!(
            take_next_n(&mut out_recv, 3).await,
            &[
                (1, "GOODBYE".to_owned()),
                (2, "GOODBYE".to_owned()),
                (3, "GOODBYE".to_owned())
            ]
        );
    }

    #[tokio::test]
    async fn test_chat_app_replay() {
        let mut deployment = Deployment::new();

        let mut builder = hydro_lang::compile::builder::FlowBuilder::new();
        let external = builder.external::<()>();
        let p1 = builder.process();

        let (users_send, users) = p1.source_external_bincode(&external);
        let (messages_send, messages) = p1.source_external_bincode(&external);
        let out = super::chat_app(users, messages, true, nondet!(/** test */));
        let out_recv = out.send_bincode_external(&external);

        let mut built = builder.with_default_optimize();

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

        let mut users_send = nodes.connect(users_send).await;
        let mut messages_send = nodes.connect(messages_send).await;
        let mut out_recv = nodes.connect(out_recv).await;

        deployment.start().await.unwrap();

        users_send.send(1).await.unwrap();
        users_send.send(2).await.unwrap();

        messages_send.send("hello".to_owned()).await.unwrap();
        messages_send.send("world".to_owned()).await.unwrap();

        {
            let out = take_next_n(&mut out_recv, 4).await;
            let h1 = out.iter().position(|x| *x == (1, "HELLO".to_owned()));
            let h2 = out.iter().position(|x| *x == (2, "HELLO".to_owned()));
            let w1 = out.iter().position(|x| *x == (1, "WORLD".to_owned()));
            let w2 = out.iter().position(|x| *x == (2, "WORLD".to_owned()));
            // Assert all items exist.
            assert!(h1.is_some());
            assert!(h2.is_some());
            assert!(w1.is_some());
            assert!(w2.is_some());
            // Assert partial order is preserved
            assert!(h1.unwrap() < h2.unwrap());
            assert!(h1.unwrap() < w1.unwrap());
            assert!(h2.unwrap() < w2.unwrap());
            assert!(w1.unwrap() < w2.unwrap());
        }

        users_send.send(3).await.unwrap();

        messages_send.send("goodbye".to_owned()).await.unwrap();

        {
            let out = take_next_n(&mut out_recv, 5).await;
            let h3 = out.iter().position(|x| *x == (3, "HELLO".to_owned()));
            let w3 = out.iter().position(|x| *x == (3, "WORLD".to_owned()));
            let g1 = out.iter().position(|x| *x == (1, "GOODBYE".to_owned()));
            let g2 = out.iter().position(|x| *x == (2, "GOODBYE".to_owned()));
            let g3 = out.iter().position(|x| *x == (3, "GOODBYE".to_owned()));
            // Assert all items exist.
            assert!(h3.is_some());
            assert!(w3.is_some());
            assert!(g1.is_some());
            assert!(g2.is_some());
            assert!(g3.is_some());
            // Assert partial order is preserved
            assert!(g1.unwrap() < g2.unwrap());
            assert!(g2.unwrap() < g3.unwrap());
            assert!(h3.unwrap() < w3.unwrap());
            assert!(w3.unwrap() < g3.unwrap());
        }
    }
}
