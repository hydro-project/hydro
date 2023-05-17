use std::collections::HashMap;
use std::sync::Arc;

use futures::channel::mpsc::UnboundedSender;
use futures::stream::SplitSink;
use futures::{SinkExt, StreamExt};
use hydroflow::bytes::Bytes;
use hydroflow::hydroflow_syntax;
use hydroflow::tokio_stream::wrappers::UnboundedReceiverStream;
use hydroflow::util::cli::ConnectedSink;
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;

async fn ws_server(
    ws_port: TcpListener,
) -> (
    UnboundedReceiverStream<usize>,
    UnboundedReceiverStream<usize>,
    UnboundedReceiverStream<(usize, String)>,
    UnboundedSender<(usize, String)>,
) {
    let (clients_send, clients_connect) = hydroflow::util::unbounded_channel();
    let (client_disconnect_send, clients_disconnect) = hydroflow::util::unbounded_channel();

    let (received_messages_send, from_client) =
        hydroflow::util::unbounded_channel::<(usize, String)>();
    let recipients = Arc::new(tokio::sync::Mutex::new(HashMap::<
        usize,
        SplitSink<_, Message>,
    >::new()));

    let send_recipients = recipients.clone();
    let (to_client, mut to_client_recv) = futures::channel::mpsc::unbounded::<(usize, String)>();
    tokio::spawn(async move {
        while let Some((recipient, msg)) = to_client_recv.next().await {
            let mut recipients_locked = send_recipients.lock().await;
            if let Some(r) = recipients_locked.get_mut(&recipient) {
                if r.send(Message::Text(msg)).await.is_err() {
                    let _ = client_disconnect_send.send(recipient);
                    recipients_locked.remove(&recipient);
                }
            } else {
                // dbg!("could not find recipient");
            }
        }
    });

    tokio::spawn(async move {
        let mut counter: usize = 0;
        loop {
            if let Ok((stream, _)) = ws_port.accept().await {
                if let Ok(ws_stream) = tokio_tungstenite::accept_async(stream).await {
                    let (send, mut recv) = ws_stream.split();
                    let my_counter = counter;
                    counter += 1;

                    let mut recipients_lock = recipients.lock().await;
                    recipients_lock.insert(my_counter, send);

                    let my_received_messages_send = received_messages_send.clone();
                    tokio::spawn(async move {
                        while let Some(msg) = recv.next().await {
                            if let Ok(Message::Text(msg)) = msg {
                                my_received_messages_send.send((my_counter, msg)).unwrap();
                            }
                        }
                    });

                    clients_send.send(my_counter).unwrap();
                }
            }
        }
    });

    (clients_connect, clients_disconnect, from_client, to_client)
}

#[tokio::main]
async fn main() {
    let mut ports = hydroflow::util::cli::init().await;

    let to_logger = ports
        .port("to_logger")
        .connect::<hydroflow::util::cli::ConnectedBidi>()
        .await
        .into_sink();

    let ws_port = TcpListener::bind("0.0.0.0:8080").await.unwrap();

    let (clients_connect, clients_disconnect, from_client, to_client) = ws_server(ws_port).await;

    let df = hydroflow_syntax! {
        source_stream(clients_connect) -> for_each(|_| println!("got connection!"));
        source_stream(clients_disconnect) -> for_each(|_| println!("lost connection!"));

        from_client = source_stream(from_client) -> tee();
        from_client -> dest_sink(to_client);

        names = from_client ->
            filter(|(_, msg): &(_, String)| msg.starts_with("name: ")) ->
            map(|(client, msg)| (client, msg[6..].to_string())) ->
            persist();

        from_client -> [0] messages_with_names;
        names -> [1] messages_with_names;
        messages_with_names = join::<'tick, 'tick>();

        messages_with_names -> map(|(_, (msg, name))| format!("{}: {}", name, msg)) -> map(Bytes::from) -> dest_sink(to_logger);
    };

    hydroflow::util::cli::launch_flow(df).await;
}
