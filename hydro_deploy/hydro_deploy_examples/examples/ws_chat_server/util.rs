use std::collections::HashMap;
use std::sync::Arc;

use dfir_rs::tokio_stream::wrappers::UnboundedReceiverStream;
use futures::channel::mpsc::UnboundedSender;
use futures::stream::SplitSink;
use futures::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;

pub(crate) async fn ws_server(
    ws_port: TcpListener,
) -> (
    UnboundedReceiverStream<usize>,
    UnboundedReceiverStream<usize>,
    UnboundedReceiverStream<(usize, String)>,
    UnboundedSender<(usize, String)>,
) {
    let (clients_send, clients_connect) = dfir_rs::util::unbounded_channel();
    let (client_disconnect_send, clients_disconnect) = dfir_rs::util::unbounded_channel();

    let (received_messages_send, from_client) =
        dfir_rs::util::unbounded_channel::<(usize, String)>();
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

                    recipients.lock().await.insert(my_counter, send);

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
