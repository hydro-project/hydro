use chrono::prelude::*;
use colored::Colorize;

use crate::helpers::{deserialize_msg, resolve_ipv4_connection_addr, serialize_msg};
use crate::protocol::Message;
use crate::{GraphType, Opts};
use chrono::Utc;
use hydroflow::hydroflow_syntax;
use hydroflow::pusherator::Pusherator;
use tokio::io::AsyncBufReadExt;
use tokio::net::UdpSocket;
use tokio_stream::wrappers::LinesStream;

pub(crate) async fn run_client(opts: Opts) {
    // set up network and I/O channels
    let server_ip = opts
        .server_addr
        .expect("Clients must specify --server-addr");
    let server_port = opts
        .server_port
        .expect("Clients must specify --server-port");

    let server_addr = resolve_ipv4_connection_addr(server_ip, server_port)
        .expect("Unable to resolve server address");
    println!("Attempting to connect to server at {}", server_addr);

    let client_addr = resolve_ipv4_connection_addr(opts.addr, opts.port)
        .expect("Unable to resolve client address");

    let client_socket = UdpSocket::bind(client_addr).await.unwrap();

    println!("Client is bound to {}", client_addr);

    let (outbound, inbound) = hydroflow::util::udp_lines(client_socket);

    let reader = tokio::io::BufReader::new(tokio::io::stdin());
    let stdin_lines = LinesStream::new(reader.lines());
    println!("Client live!");

    let mut hf = hydroflow_syntax! {
        // set up channels
        outbound_chan = merge() -> map(|(m,a)| (serialize_msg(m), a)) -> sink_async(outbound);
        inbound_chan = recv_stream(inbound) -> map(deserialize_msg)
            ->  demux(|m, tl!(acks, msgs, errs)|
                    match m {
                        Message::ConnectResponse => acks.give(m),
                        Message::ChatMsg {..} => msgs.give(m),
                        _ => errs.give(m),
                    }
                );
        connect_acks = inbound_chan[acks] -> tee();
        inbound_chan[errs] -> for_each(|m| println!("Received unexpected message type: {:?}", m));

        // send a single connection request on startup
        recv_iter([()]) -> map(|_m| (Message::ConnectRequest {
            nickname: opts.name.clone(),
            addr: client_addr,
        }, server_addr)) -> [0]outbound_chan;

        // take stdin and send to server as a msg
        // the join serves to postpone msgs until the connection request is acked
        msg_send = cross_join() -> map(|(msg, _)| (msg, server_addr)) -> [1]outbound_chan;
        lines = recv_stream(stdin_lines)
          -> map(|l| Message::ChatMsg {
                    nickname: opts.name.clone(),
                    message: l.unwrap(),
                    ts: Utc::now()})
          -> [0]msg_send;

        // receive and print messages
        inbound_chan[msgs] -> for_each(|m: Message| if let Message::ChatMsg{ nickname, message, ts } = m {
                println!(
                    "{} {}: {}",
                    ts
                        .with_timezone(&Local)
                        .format("%b %-d, %-I:%M:%S")
                        .to_string()
                        .truecolor(126, 126, 126)
                        .italic(),
                    nickname.green().italic(),
                    message,
                );
        });

        // handle connect ack
        connect_acks[0] -> for_each(|m: Message| println!("connected: {:?}", m));
        connect_acks[1] -> [1]msg_send;

    };

    if let Some(graph) = opts.graph {
        let serde_graph = hf
            .serde_graph()
            .expect("No graph found, maybe failed to parse.");
        match graph {
            GraphType::Mermaid => {
                println!("{}", serde_graph.to_mermaid());
            }
            GraphType::Dot => {
                println!("{}", serde_graph.to_dot())
            }
            GraphType::Json => {
                unimplemented!();
                // println!("{}", serde_graph.to_json())
            }
        }
    }
    hf.run_async().await.unwrap();
}
