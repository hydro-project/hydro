use hydroflow_plus::deploy::SingleProcessGraph;
use hydroflow_plus::tokio::sync::mpsc::UnboundedSender;
use hydroflow_plus::tokio_stream::wrappers::UnboundedReceiverStream;
use hydroflow_plus::*;
use stageleft::{q, Quoted, RuntimeData};

#[stageleft::entry]
pub fn chat_app<'a>(
    flow: FlowBuilder<'a>,
    users_stream: RuntimeData<UnboundedReceiverStream<u32>>,
    messages: RuntimeData<UnboundedReceiverStream<String>>,
    output: RuntimeData<&'a UnboundedSender<(u32, String)>>,
    replay_messages: bool,
) -> impl Quoted<'a, Hydroflow<'a>> {
    let process = flow.process::<()>();

    let users = process.source_stream(users_stream).tick_batch().persist();
    let messages = process.source_stream(messages);
    let messages = if replay_messages {
        messages.tick_batch().persist()
    } else {
        messages.tick_batch()
    };

    // do this after the persist to test pullup
    let messages = messages.map(q!(|s| s.to_uppercase()));

    let mut joined = users.cross_product(messages);
    if replay_messages {
        joined = joined.delta();
    }

    joined.all_ticks().for_each(q!(|t| {
        output.send(t).unwrap();
    }));

    flow.with_default_optimize()
        .compile_no_network::<SingleProcessGraph>()
}

#[stageleft::runtime]
#[cfg(test)]
mod tests {
    use hydroflow_plus::assert_graphvis_snapshots;
    use hydroflow_plus::util::collect_ready;

    #[test]
    fn test_chat_app_no_replay() {
        let (users_send, users) = hydroflow_plus::util::unbounded_channel();
        let (messages_send, messages) = hydroflow_plus::util::unbounded_channel();
        let (out, mut out_recv) = hydroflow_plus::util::unbounded_channel();

        let mut chat_server = super::chat_app!(users, messages, &out, false);
        assert_graphvis_snapshots!(chat_server);

        users_send.send(1).unwrap();
        users_send.send(2).unwrap();

        messages_send.send("hello".to_string()).unwrap();
        messages_send.send("world".to_string()).unwrap();

        chat_server.run_tick();

        assert_eq!(
            &*collect_ready::<Vec<_>, _>(&mut out_recv),
            &[
                (1, "HELLO".to_string()),
                (2, "HELLO".to_string()),
                (1, "WORLD".to_string()),
                (2, "WORLD".to_string())
            ]
        );

        users_send.send(3).unwrap();

        messages_send.send("goodbye".to_string()).unwrap();

        chat_server.run_tick();

        assert_eq!(
            &*collect_ready::<Vec<_>, _>(&mut out_recv),
            &[
                (1, "GOODBYE".to_string()),
                (2, "GOODBYE".to_string()),
                (3, "GOODBYE".to_string())
            ]
        );
    }

    #[test]
    fn test_chat_app_replay() {
        let (users_send, users) = hydroflow_plus::util::unbounded_channel();
        let (messages_send, messages) = hydroflow_plus::util::unbounded_channel();
        let (out, mut out_recv) = hydroflow_plus::util::unbounded_channel();

        let mut chat_server = super::chat_app!(users, messages, &out, true);
        assert_graphvis_snapshots!(chat_server);

        users_send.send(1).unwrap();
        users_send.send(2).unwrap();

        messages_send.send("hello".to_string()).unwrap();
        messages_send.send("world".to_string()).unwrap();

        chat_server.run_tick();

        assert_eq!(
            &*collect_ready::<Vec<_>, _>(&mut out_recv),
            &[
                (1, "HELLO".to_string()),
                (2, "HELLO".to_string()),
                (1, "WORLD".to_string()),
                (2, "WORLD".to_string())
            ]
        );

        users_send.send(3).unwrap();

        messages_send.send("goodbye".to_string()).unwrap();

        chat_server.run_tick();

        assert_eq!(
            &*collect_ready::<Vec<_>, _>(&mut out_recv),
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
