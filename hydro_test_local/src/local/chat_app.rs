use dfir_rs::scheduled::graph::Dfir;
use dfir_rs::tokio_stream::wrappers::UnboundedReceiverStream;
use hydro_lang::deploy::SingleProcessGraph;
use hydro_lang::*;
use stageleft::{Quoted, RuntimeData};
use tokio::sync::mpsc::UnboundedSender;

#[stageleft::entry]
pub fn chat_app<'a>(
    flow: FlowBuilder<'a>,
    users_stream: RuntimeData<UnboundedReceiverStream<u32>>,
    messages: RuntimeData<UnboundedReceiverStream<String>>,
    output: RuntimeData<&'a UnboundedSender<(u32, String)>>,
    replay_messages: bool,
) -> impl Quoted<'a, Dfir<'a>> {
    let process = flow.process::<()>();
    let tick = process.tick();

    let users = unsafe {
        // SAFETY: intentionally non-deterministic to not send messaged
        // to users that joined after the message was sent
        process.source_stream(users_stream).tick_batch(&tick)
    }
    .persist();
    let messages = process.source_stream(messages);
    let messages = if replay_messages {
        unsafe {
            // SAFETY: see above
            messages.tick_batch(&tick)
        }
        .persist()
    } else {
        unsafe {
            // SAFETY: see above
            messages.tick_batch(&tick)
        }
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

    flow.compile_no_network::<SingleProcessGraph>()
}

#[cfg(stageleft_runtime)]
#[cfg(test)]
mod tests {
    use dfir_rs::assert_graphvis_snapshots;
    use dfir_rs::util::collect_ready;

    #[test]
    fn test_chat_app_no_replay() {
        let (users_send, users) = dfir_rs::util::unbounded_channel();
        let (messages_send, messages) = dfir_rs::util::unbounded_channel();
        let (out, mut out_recv) = dfir_rs::util::unbounded_channel();

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
        let (users_send, users) = dfir_rs::util::unbounded_channel();
        let (messages_send, messages) = dfir_rs::util::unbounded_channel();
        let (out, mut out_recv) = dfir_rs::util::unbounded_channel();

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
