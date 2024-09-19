use std::cell::RefCell;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::pin::pin;
use std::rc::Rc;

use futures::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio::task::spawn_local;
use tokio_tungstenite::tungstenite::{Error, Message};

use crate::util::unsync::mpsc::{Receiver, Sender};
use crate::util::unsync_channel;

pub async fn bind_websocket(
    endpoint: SocketAddr,
) -> Result<
    (
        Sender<(Message, SocketAddr)>,
        Receiver<Result<(Message, SocketAddr), Error>>,
        SocketAddr,
    ),
    std::io::Error,
> {
    let listener = TcpListener::bind(endpoint).await.unwrap();

    let bound_endpoint = listener.local_addr()?;

    let (tx_egress, mut rx_egress) = unsync_channel(None);
    let (tx_ingress, rx_ingress) = unsync_channel(None);

    let clients = Rc::new(RefCell::new(HashMap::new()));

    spawn_local({
        let clients = clients.clone();

        async move {
            while let Some((payload, addr)) = rx_egress.next().await {
                let client = clients.borrow_mut().remove(&addr);

                if let Some(mut sender) = client {
                    let _ = SinkExt::send(&mut sender, payload).await;
                    clients.borrow_mut().insert(addr, sender);
                }
            }
        }
    });

    // Spawn the listener
    spawn_local(async move {
        loop {
            let (stream, peer_addr) = if let Ok((stream, _)) = listener.accept().await {
                if let Ok(peer_addr) = stream.peer_addr() {
                    (stream, peer_addr)
                } else {
                    continue;
                }
            } else {
                continue;
            };

            // Perform the websocket handshake
            let ws_stream = tokio_tungstenite::accept_async(stream)
                .await
                .expect("Error during the websocket handshake occurred");

            // Split the stream into incoming and outgoing
            let (outgoing, incoming) = ws_stream.split();
            let mut tx_ingress = tx_ingress.clone();

            clients.borrow_mut().insert(peer_addr, outgoing);

            spawn_local({
                let clients = clients.clone();
                async move {
                    let mapped = incoming.map(|x| Ok(x.map(|x| (x, peer_addr))));
                    let _ = tx_ingress.send_all(&mut pin!(mapped)).await;

                    clients.borrow_mut().remove(&peer_addr);
                }
            });
        }
    });

    Ok((tx_egress, rx_ingress, bound_endpoint))
}
