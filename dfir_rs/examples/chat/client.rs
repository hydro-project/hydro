use chrono::prelude::*;
use colored::Colorize;
use dfir_rs::dfir_syntax;
use dfir_rs::util::{bind_udp_bytes, ipv4_resolve};

use crate::protocol::Message;
use crate::{Opts, default_server_address};

fn pretty_print_msg(nickname: String, message: String, ts: DateTime<Utc>) {
    println!(
        "{} {}: {}",
        ts.with_timezone(&Local)
            .format("%b %-d, %-I:%M:%S")
            .to_string()
            .truecolor(126, 126, 126)
            .italic(),
        nickname.green().italic(),
        message,
    );
}

pub(crate) async fn run_client(opts: Opts) {
    // Client listens on a port picked by the OS.
    let client_addr = ipv4_resolve("localhost:0").unwrap();

    // Use the server address that was provided in the command-line arguments, or use the default
    // if one was not provided.
    let server_addr = opts.address.unwrap_or_else(default_server_address);

    let (outbound, inbound, allocated_client_addr) = bind_udp_bytes(client_addr).await;

    println!(
        "Client is live! Listening on {:?} and talking to server on {:?}",
        allocated_client_addr, server_addr
    );

    let mut hf = dfir_syntax! {
        // set up channels
        outbound_chan = union() -> dest_sink_serde(outbound);
        inbound_chan = source_stream_serde(inbound)
            -> map(Result::unwrap)
            -> map(|(msg, _addr)| msg)
            -> demux_enum::<Message>();
        inbound_chan[ConnectRequest] -> for_each(|()| println!("Received unexpected connect request from server."));

        // send a single connection request on startup
        initialize() -> map(|_m| (Message::ConnectRequest, server_addr)) -> [0]outbound_chan;

        // take stdin and send to server as a msg
        // the batch serves to buffer msgs until the connection request is acked
        lines = source_stdin()
          -> map(|l| Message::ChatMsg {
                    nickname: opts.name.clone(),
                    message: l.unwrap(),
                    ts: Utc::now()})
          -> [input]msg_send;
        inbound_chan[ConnectResponse] -> persist::<'static>() -> [signal]msg_send;
        msg_send = defer_signal() -> map(|msg| (msg, server_addr)) -> [1]outbound_chan;

        // receive and print messages
        inbound_chan[ChatMsg] -> for_each(|(nick, msg, ts)| pretty_print_msg(nick, msg, ts));
    };

    // optionally print the dataflow graph
    #[cfg(feature = "debugging")]
    if let Some(graph) = opts.graph {
        let serde_graph = hf
            .meta_graph()
            .expect("No graph found, maybe failed to parse.");
        serde_graph.open_graph(graph, opts.write_config).unwrap();
    }

    let None = hf.run().await;
}
