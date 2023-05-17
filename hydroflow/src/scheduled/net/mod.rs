#![cfg(not(target_arch = "wasm32"))]

//! This module contiains networking code.
//!
//! ## How Tokio interacts with Hydroflow (Mingwei 2021-12-07)
//!
//! [Tokio](https://tokio.rs/) is a Rust async runtime. In Rust's async/await
//! system, `Future`s must be spawned or sent to an async runtime in order to
//! run. Tokio is the most popular provider of one of these runtimes, with
//! [async-std](https://async.rs/) (mirrors std lib) and [smol](https://github.com/smol-rs/smol)
//! (minimal runtime) as commonly used alternatives.
//!
//! Fundamentally, an async runtime's job is to poll futures (read: run tasks)
//! when they are ready to make progress. However async runtimes also provide a
//! `Future`s abstraction for async events such as timers, network IO, and
//! filesystem IO. To do this, [Tokio](https://tokio.rs/) uses [Mio](https://github.com/tokio-rs/mio)
//! which is a low-level non-blocking API for IO event notification/polling.
//! A user of Mio can write an event loop, i.e. something like: wait for
//! events, run computations responding to those events, repeat. Tokio provides
//! the higher-level async/await slash `Future` abstraction on top of Mio, as
//! well as the runtime to execute those `Future`s. Essentially, the Tokio
//! async runtime essentially replaces the low-level event loop a user might
//! handwrite when using Mio.
//!
//! For context, both Mio and Tokio provide socket/UDP/TCP-level network
//! abstractions, which is probably the right layer for us. There are also
//! libraries built on top of Tokio providing nice server/client HTTP APIs
//! like [Hyper](https://hyper.rs/).
//!
//! The Hydroflow scheduled layer scheduler is essentially the same as a simple
//! event loop: it runs subgraphs when they have data. We have also let it
//! respond to external asynchonous events by providing a threadsafe channel
//! through which subgraphs can be externally scheduled.
//!
//! In order to add networking to Hydroflow, in our current implementation we
//! use Tokio and have a compatibility mechanism for working with `Future`s.
//! A `Future` provides a `Waker` mechanism to notify when it had work to do,
//! so we have hooked these Wakers up with Hydroflow's threadsafe external
//! scheduling channel. This essentially turns Hydroflow into a simple async
//! runtime.
//!
//! However in some situations, we still need to run futures outside of
//! Hydroflow's basic runtime. It's not a goal for Hydroflow to provide all
//! the features of a full runtime like Tokio. Currently for this situation we
//! run Hydroflow as a task (`Future`) within the Tokio runtime. In Hydroflow's
//! event loop we do all available work, then rather than block and wait for
//! external events to schedule more tasks, we temporarily yield back to the
//! Tokio runtime. Tokio will then respond to any outstanding events it has
//! before once again running the Hydroflow scheduler task.
//!
//! This works perfectly well but maybe isn't the best solution long-term.
//! In the future we may want to remove the extra Tokio runtime layer and
//! interface with Mio directly. In this case we would have to do our own
//! socket-style polling within the Hydroflow scheduler's event loop, which
//! would require some extra work and thought. But for now interfacing with
//! Tokio works and I don't think the overhead of the extra runtime loop is
//! significant.

use std::collections::VecDeque;
use std::pin::Pin;

use byteorder::{NetworkEndian, WriteBytesExt};
use futures::{Sink, StreamExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

use super::graph::Hydroflow;
use super::graph_ext::GraphExt;
use super::handoff::VecHandoff;
use super::port::{RecvPort, SendPort};

pub mod network_vertex;

const ADDRESS_LEN: usize = 4;

// TODO(justin): I don't think we should include the address here, that should
// just be a part of the bytes being sent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    pub address: u32,
    pub batch: bytes::Bytes,
}

impl Message {
    fn encode(&self, v: &mut Vec<u8>) {
        v.write_u32::<NetworkEndian>(self.address).unwrap();
        v.extend(self.batch.iter());
    }

    pub fn decode(v: bytes::Bytes) -> Self {
        let address = u32::from_be_bytes(v[0..ADDRESS_LEN].try_into().unwrap());
        let batch = v.slice(ADDRESS_LEN..);
        Message { address, batch }
    }
}

impl Hydroflow {
    fn register_read_tcp_stream(&mut self, reader: OwnedReadHalf) -> RecvPort<VecHandoff<Message>> {
        let reader = FramedRead::new(reader, LengthDelimitedCodec::new());
        let (send_port, recv_port) = self.make_edge("tcp ingress handoff");
        self.add_input_from_stream(
            "tcp ingress",
            send_port,
            reader.map(|buf| Some(<Message>::decode(buf.unwrap().into()))),
        );
        recv_port
    }

    fn register_write_tcp_stream(
        &mut self,
        writer: OwnedWriteHalf,
    ) -> SendPort<VecHandoff<Message>> {
        let mut writer = FramedWrite::new(writer, LengthDelimitedCodec::new());
        let mut message_queue = VecDeque::new();

        let (input_port, output_port) =
            self.make_edge::<_, VecHandoff<Message>>("tcp egress handoff");
        self.add_subgraph_sink("tcp egress", output_port, move |ctx, recv| {
            let waker = ctx.waker();
            let mut cx = std::task::Context::from_waker(&waker);

            // TODO(mingwei): queue may grow unbounded? Subtle rate matching concern.
            // TODO(mingwei): put into state system.
            message_queue.extend(recv.take_inner().into_iter());
            while !message_queue.is_empty() {
                if let std::task::Poll::Ready(Ok(())) = Pin::new(&mut writer).poll_ready(&mut cx) {
                    let v = message_queue.pop_front().unwrap();
                    let mut buf = Vec::new();
                    v.encode(&mut buf);

                    Pin::new(&mut writer).start_send(buf.into()).unwrap();
                }
            }
            let _ = Pin::new(&mut writer).poll_flush(&mut cx);
        });

        input_port
    }

    pub fn add_write_tcp_stream(&mut self, stream: TcpStream) -> SendPort<VecHandoff<Message>> {
        let (_, writer) = stream.into_split();

        self.register_write_tcp_stream(writer)
    }

    pub fn add_read_tcp_stream(&mut self, stream: TcpStream) -> RecvPort<VecHandoff<Message>> {
        let (reader, _) = stream.into_split();

        self.register_read_tcp_stream(reader)
    }

    pub fn add_tcp_stream(
        &mut self,
        stream: TcpStream,
    ) -> (SendPort<VecHandoff<Message>>, RecvPort<VecHandoff<Message>>) {
        let (reader, writer) = stream.into_split();

        (
            self.register_write_tcp_stream(writer),
            self.register_read_tcp_stream(reader),
        )
    }
}
