#![cfg(not(target_arch = "wasm32"))]

use std::collections::HashMap;

use futures::{SinkExt, StreamExt};
use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::net::{TcpListener, TcpStream};
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

use crate::scheduled::graph::Hydroflow;
use crate::scheduled::graph_ext::GraphExt;
use crate::scheduled::handoff::VecHandoff;
use crate::scheduled::port::{RecvPort, SendPort};

pub type Address = String;

// These methods can't be wrapped up in a trait because async methods are not
// allowed in traits (yet).

impl Hydroflow {
    // TODO(justin): document these, but they're derivatives of inbound_tcp_vertex_internal.
    pub async fn inbound_tcp_vertex_port<T>(&mut self, port: u16) -> RecvPort<VecHandoff<T>>
    where
        T: 'static + DeserializeOwned + Send,
    {
        self.inbound_tcp_vertex_internal(Some(port)).await.1
    }

    pub async fn inbound_tcp_vertex<T>(&mut self) -> (u16, RecvPort<VecHandoff<T>>)
    where
        T: 'static + DeserializeOwned + Send,
    {
        self.inbound_tcp_vertex_internal(None).await
    }
    // TODO(justin): this needs to return a result/get rid of all the unwraps, I
    // guess we need a HydroflowError?
    /// Begins listening on some TCP port. Returns an [OutputPort] representing
    /// the stream of messages received. Currently there is no notion of
    /// identity to the connections received, if they are to be attached to some
    /// participant in the system, that needs to be included in the message
    /// directly.
    ///
    /// The messages will be interpreted to be bincode-encoded, length-delimited
    /// messages, as produced by [Self::outbound_tcp_vertex].
    async fn inbound_tcp_vertex_internal<T>(
        &mut self,
        port: Option<u16>,
    ) -> (u16, RecvPort<VecHandoff<T>>)
    where
        T: 'static + DeserializeOwned + Send,
    {
        let listener = TcpListener::bind(format!("localhost:{}", port.unwrap_or(0)))
            .await
            .unwrap();
        let port = listener.local_addr().unwrap().port();

        // TODO(justin): figure out an appropriate buffer here.
        let (incoming_send, incoming_messages) = futures::channel::mpsc::channel(1024);

        // Listen to incoming connections and spawn a tokio task for each one,
        // which feeds into the channel.
        // TODO(justin): give some way to get a handle into this thing.
        tokio::spawn(async move {
            loop {
                let (socket, _) = listener.accept().await.unwrap();
                let (reader, _) = socket.into_split();
                let mut reader = FramedRead::new(reader, LengthDelimitedCodec::new());
                let mut incoming_send = incoming_send.clone();
                tokio::spawn(async move {
                    while let Some(msg) = reader.next().await {
                        // TODO(justin): figure out error handling here.
                        let msg = msg.unwrap();
                        let out = bincode::deserialize(&msg).unwrap();
                        incoming_send.send(out).await.unwrap();
                    }
                    // TODO(justin): The connection is closed, so we should
                    // clean up its metadata.
                });
            }
        });

        let (send_port, recv_port) = self.make_edge("tcp ingress handoff");
        self.add_input_from_stream("tcp ingress stream", send_port, incoming_messages.map(Some));

        (port, recv_port)
    }

    pub async fn outbound_tcp_vertex<T>(&mut self) -> SendPort<VecHandoff<(Address, T)>>
    where
        T: 'static + Serialize + Send,
    {
        let (mut connection_reqs_send, mut connection_reqs_recv) =
            futures::channel::mpsc::channel(1024);
        let (mut connections_send, mut connections_recv) = futures::channel::mpsc::channel(1024);

        // TODO(justin): handle errors here.
        // Spawn an actor which establishes connections.
        tokio::spawn(async move {
            while let Some(addr) = connection_reqs_recv.next().await {
                let addr: Address = addr;
                connections_send
                    .send((addr.clone(), TcpStream::connect(addr.clone()).await))
                    .await
                    .unwrap();
            }
        });

        enum ConnStatus<T> {
            Pending(Vec<T>),
            Connected(FramedWrite<TcpStream, LengthDelimitedCodec>),
        }

        let (mut outbound_messages_send, mut outbound_messages_recv) =
            futures::channel::mpsc::channel(1024);
        tokio::spawn(async move {
            // TODO(justin): this cache should be global to the entire Hydroflow
            // instance so we can reuse connections from inbound connections.
            let mut connections = HashMap::<Address, ConnStatus<T>>::new();

            loop {
                tokio::select! {
                    Some((addr, msg)) = outbound_messages_recv.next() => {
                        let addr: Address = addr;
                        let msg: T = msg;
                        match connections.get_mut(&addr) {
                            None => {
                                // We have not seen this address before, open a
                                // connection to it and buffer the message to be
                                // sent once it's open.

                                // TODO(justin): what do we do if the buffer is full here?
                                connection_reqs_send.try_send(addr.clone()).unwrap();
                                connections.insert(addr, ConnStatus::Pending(vec![msg]));
                            }
                            Some(ConnStatus::Pending(msgs)) => {
                                // We have seen this address before but we're
                                // still trying to connect to it, so buffer this
                                // message so that when we _do_ connect we will
                                // send it.
                                msgs.push(msg);
                            }
                            Some(ConnStatus::Connected(conn)) => {
                                // TODO(justin): move the actual sending here
                                // into a different task so we don't have to
                                // wait for the send.
                                let msg = bincode::serialize(&msg).unwrap();
                                conn.send(msg.into()).await.unwrap();
                            }
                        }
                    },

                    Some((addr, conn)) = connections_recv.next() => {
                        match conn {
                            Ok(conn) => {
                                match connections.get_mut(&addr) {
                                    Some(ConnStatus::Pending(msgs)) => {
                                        let mut conn = FramedWrite::new(conn, LengthDelimitedCodec::new());
                                        for msg in msgs.drain(..) {
                                            // TODO(justin): move the actual sending here
                                            // into a different task so we don't have to
                                            // wait for the send.
                                            let msg = bincode::serialize(&msg).unwrap();
                                            conn.send(msg.into()).await.unwrap();
                                        }
                                        connections.insert(addr, ConnStatus::Connected(conn));
                                    }
                                    None => {
                                        // This means nobody ever requested this
                                        // connection, so we shouldn't have initiated it
                                        // in the first place.
                                        unreachable!()
                                    }
                                    Some(ConnStatus::Connected(_tcp)) => {
                                        // This means we were already connected, so we
                                        // shouldn't have connected again. If the
                                        // connection cache becomes shared this could
                                        // become reachable.
                                        unreachable!()
                                    }
                                }
                            }
                            Err(e) => {
                                // We couldn't connect to the address for some
                                // reason.
                                // TODO(justin): once we have a clearer picture
                                // of error handling, we could do something like
                                // send this error along a pipe to be handled by
                                // someone else. For now, just log it and drop
                                // any pending messages.
                                eprintln!("couldn't connect to {}: {}", addr, e);
                                connections.remove(&addr);
                            }
                        }
                    },
                    else => break,
                }
            }
        });

        let mut buffered_messages = Vec::new();
        let mut next_messages = Vec::new();
        let (input_port, output_port) = self.make_edge("tcp egress handoff");
        self.add_subgraph_sink("tcp egress stream", output_port, move |_ctx, recv| {
            buffered_messages.extend(recv.take_inner());
            for msg in buffered_messages.drain(..) {
                if let Err(e) = outbound_messages_send.try_send(msg) {
                    // If we weren't able to send a message (say, because the
                    // buffer is full), we get handed it back in the error. If
                    // this happens we hang onto the message to try sending it
                    // again next time.
                    next_messages.push(e.into_inner());
                }
            }

            // NB. we don't need to flush the channel here due to the use of
            // `try_send`.  It's guaranteed that there was space for the
            // messages and that they were sent.

            // TODO(justin): we do need to make sure we get rescheduled if
            // next_messages is empty here.

            std::mem::swap(&mut buffered_messages, &mut next_messages);
        });

        input_port
    }
}
