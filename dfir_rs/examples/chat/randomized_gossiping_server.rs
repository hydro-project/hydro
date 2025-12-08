use std::collections::HashSet;
use std::net::SocketAddr;

use chrono::{DateTime, Utc};
use dfir_macro::dfir_syntax;
use dfir_rs::scheduled::graph::Dfir;
use dfir_rs::util::{bind_udp_bytes, ipv4_resolve};
use rand::seq::SliceRandom;
use rand::thread_rng;
use serde::{Deserialize, Serialize};
use web_time::Duration;

use crate::Role::{
    Client, GossipingServer1, GossipingServer2, GossipingServer3, GossipingServer4,
    GossipingServer5, Server,
};
use crate::protocol::{Message, MessageWithAddr};
use crate::{Opts, Role, default_server_address};

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize, Debug, Hash)]
pub struct ChatMessage {
    nickname: String,
    message: String,
    ts: DateTime<Utc>,
}

/// Used to model add and remove operations on a set of infecting messages.
enum InfectionOperation {
    /// Add an infecting message to the current set
    InfectWithMessage { msg: ChatMessage },

    /// Remove an infecting message from the current set
    RemoveForMessage { msg: ChatMessage },
}

pub const REMOVAL_PROBABILITY: f32 = 1.0 / 4.0;

/// Runs an instance of a server that gossips new chat messages with other instances of the server.
///
/// The servers are protocol-compatible with the broadcast-based server run by
/// [crate::server::run_server], so can be used with the existing client
/// ([crate::client::run_client]).
///
/// The implementation is based on "Epidemic algorithms for replicated database maintenance"
/// (https://dl.acm.org/doi/epdf/10.1145/41840.41841). Specifically, it implements push-based
/// "rumor-mongering" with a blind-coin removal process described below.
///
/// At every "cycle" a server chooses, randomly, one peer from a group of five servers. It then
/// "pushes" all the "rumors" (messages) that it has heard so far to that peer. This is how new
/// messages propagate through the system. Without a removal process, the messages would bounce
/// around forever.
///
/// A "blind-coin" removal process is used. After a server gossips the known rumors with randomly
/// selected peers, each message is dropped with a 1/K probability. K can be configured by changing
/// [REMOVAL_PROBABILITY]. The removal is "blind" because the server doesn't check if the receiving
/// peer already knew the message, i.e. it doesn't rely on a feedback mechanism from the peer to
/// drive the process. The removal is "coin" based because it relies on pure chance (instead of
/// keeping track using a counter).
///
/// To keep things simple, the peer-group of servers is based on static membership - it contains
/// 5 members that communicate with each other on fixed ports.
pub(crate) async fn run_gossiping_server(opts: Opts) {
    // If a server address & port are provided as command-line inputs, use those, else use the
    // default.
    let server_address = opts.address.unwrap_or_else(default_server_address);

    let all_members = [
        GossipingServer1,
        GossipingServer2,
        GossipingServer3,
        GossipingServer4,
        GossipingServer5,
    ];

    let other_members: Vec<Role> = all_members
        .into_iter()
        .filter(|role| *role != opts.role)
        .collect();

    let gossip_listening_addr = gossip_address(&opts.role);

    println!("Starting server on {:?}", server_address);

    // Separate sinks and streams for client-server protocol & gossip protocol.
    let (client_outbound, client_inbound, actual_server_addr) =
        bind_udp_bytes(server_address).await;
    let (gossip_outbound, gossip_inbound, _) = bind_udp_bytes(gossip_listening_addr).await;

    println!(
        "Server is live! Listening on {:?}. Gossiping On: {:?}",
        actual_server_addr, gossip_listening_addr
    );
    let mut hf: Dfir = dfir_syntax! {
        // Define shared inbound and outbound channels
        client_out = union() -> dest_sink_serde(client_outbound);
        client_in = source_stream_serde(client_inbound)
            -> map(Result::unwrap)
            -> map(|(msg, addr)| MessageWithAddr::from_message(msg, addr))
            -> demux_enum::<MessageWithAddr>();
        clients = client_in[ConnectRequest] -> map(|(addr,)| addr) -> tee();
        client_in[ConnectResponse] -> for_each(|(addr,)| println!("Received unexpected `ConnectResponse` as server from addr {}.", addr));

        // Pipeline 1: Acknowledge client connections
        clients[0] -> map(|addr| (Message::ConnectResponse, addr)) -> [0]client_out;

        // Pipeline 2: When a message arrives from a client, it is the first time the message is
        // seen. Still, send it to the "maybe_new_messages" flow for simplicity.
        messages_from_connected_client = client_in[ChatMsg]
            -> map(|(_addr, nickname, message, ts)| ChatMessage { nickname, message, ts })
            -> maybe_new_messages;

        // Pipeline 3: When you want to send a message to all the connected clients, send it to
        // "broadcast"
        clients[1] -> [1]broadcast;
        broadcast = cross_join::<'tick, 'static>() -> [1]client_out;

        // Pipeline 3: Gossip-based broadcast to other servers.
        gossip_out = dest_sink_serde(gossip_outbound);
        gossip_in = source_stream_serde(gossip_inbound)
            -> map(Result::unwrap)
            -> map(|(message, _)| message)
            -> maybe_new_messages;

        // If you think there may be a new message, send it here.
        maybe_new_messages = union();

        // actually_new_messages are a stream of messages that the server is definitely seeing
        // for the first time.
        actually_new_messages = difference() -> tee();
        maybe_new_messages -> [pos]actually_new_messages;
        all_messages -> [neg]actually_new_messages;

        // When we have a new message, we should do 3 things
        // 1. Add it to the set of known messages.
        // 2. Broadcast it to the clients connected locally.
        // 3. Add it to the set of messages currently infecting this server.
        actually_new_messages -> defer_tick() -> all_messages; // Add to known messages
        actually_new_messages
            -> map(|chat_msg: ChatMessage| Message::ChatMsg {
                    nickname: chat_msg.nickname,
                    message: chat_msg.message,
                    ts: chat_msg.ts})
            -> [0]broadcast; // Broadcast to locally connected clients.
        actually_new_messages
            -> map(|msg: ChatMessage| InfectionOperation::InfectWithMessage { msg })
            -> infecting_messages;

        // Holds all the known messages.
        all_messages = fold::<'static>(HashSet::<ChatMessage>::new, |accum, message| {
            accum.insert(message);
        }) -> flatten();

        // Holds a set of messages that are currently infecting this server
        infecting_messages = union() -> fold::<'static>(HashSet::<ChatMessage>::new, |accum, op| {
            match op {
                InfectionOperation::InfectWithMessage{ msg } => {accum.insert(msg);},
                InfectionOperation::RemoveForMessage{ msg } => { accum.remove(&msg);}
            }
        });

        // Infection process.
        // Every 1 second, the infecting messages are dispatched to a randomly selected peer. They
        // are blindly removed with a 1/K probability after this.
        source_interval(Duration::from_secs(1)) -> [0]triggered_messages; // The time trigger to perform a round of gossip
        triggered_messages = cross_join()
            -> map(|(_, message)| {
                    // Choose a random peer
                    let random_peer = other_members.choose(&mut thread_rng()).unwrap();
                    (message, gossip_address(random_peer))
               })
            -> tee();

        infecting_messages -> flatten() -> [1]triggered_messages;

        triggered_messages
            -> inspect(|(msg, addr)| println!("Gossiping {:?} to {:?}", msg, addr))
            -> gossip_out;

        triggered_messages
            -> filter_map(|(msg, _addr)| {
                if rand::random::<f32>() < REMOVAL_PROBABILITY{
                    println!("Dropping Message {:?}", msg);
                    Some(InfectionOperation::RemoveForMessage{ msg })
                } else {
                    None
                }
            })
            -> defer_tick()
            -> infecting_messages;

    };

    #[cfg(feature = "debugging")]
    if let Some(graph) = opts.graph {
        let serde_graph = hf
            .meta_graph()
            .expect("No graph found, maybe failed to parse.");
        serde_graph.open_graph(graph, opts.write_config).unwrap();
    }

    hf.run().await.unwrap();
}

/// The address on which the gossip protocol runs. Servers communicate with each other using these
/// addresses. This is different from the ports on which clients connect to servers.
fn gossip_address(role: &Role) -> SocketAddr {
    match role {
        Client | Server => {
            panic!("Incorrect role {:?} for gossip server.", role)
        }
        GossipingServer1 => ipv4_resolve("localhost:54322"),
        GossipingServer2 => ipv4_resolve("localhost:54323"),
        GossipingServer3 => ipv4_resolve("localhost:54324"),
        GossipingServer4 => ipv4_resolve("localhost:54325"),
        GossipingServer5 => ipv4_resolve("localhost:54326"),
    }
    .unwrap()
}
