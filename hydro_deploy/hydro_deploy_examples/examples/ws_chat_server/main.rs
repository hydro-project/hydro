use dfir_rs::compiled::pull::HalfMultisetJoinState;
use dfir_rs::dfir_syntax;
use dfir_rs::util::deploy::{ConnectedSink, ConnectedSource};
use dfir_rs::util::{deserialize_from_bytes, serialize_to_bytes};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;

mod util;

#[derive(Clone, Serialize, Deserialize)]
struct PeerMessage {
    msg: ChatMessage,
    node_id: u32,
    client_id: usize,
    msg_id: usize,
}

#[derive(Clone, Serialize, Deserialize)]
enum FromClient {
    Name(String),
    Message { id: usize, text: String },
}

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct ChatMessage {
    name: String,
    text: String,
}

#[dfir_rs::main]
async fn main() {
    let ports = dfir_rs::util::deploy::init::<()>().await;

    let from_peer = ports
        .port("from_peer")
        .connect::<dfir_rs::util::deploy::ConnectedDirect>()
        .into_source();

    let to_peer = ports
        .port("to_peer")
        .connect::<dfir_rs::util::deploy::ConnectedDemux<u32, dfir_rs::util::deploy::ConnectedDirect>>()
        .into_sink();

    let number_of_nodes: u32 = std::env::args().nth(1).unwrap().parse().unwrap();
    let self_node_id: u32 = std::env::args().nth(2).unwrap().parse().unwrap();
    let port: u16 = std::env::args().nth(3).unwrap().parse().unwrap();

    let ws_port = TcpListener::bind(("0.0.0.0", port)).await.unwrap();
    println!("listening!");

    let (clients_connect, clients_disconnect, from_client, to_client) =
        util::ws_server(ws_port).await;

    let df = dfir_syntax! {
        all_peers = source_iter((0..number_of_nodes).filter(move |&i| i != self_node_id)) -> persist::<'static>();

        // networking
        from_peer = source_stream(from_peer) -> map(|b| deserialize_from_bytes::<PeerMessage>(b.unwrap()).unwrap());
        to_peer = map(|(peer, v): (u32, PeerMessage)| (peer, serialize_to_bytes(v))) -> dest_sink(to_peer);

        from_client = source_stream(from_client) -> map(|(id, json)| (id, serde_json::from_str::<FromClient>(&json).unwrap())) -> tee();
        to_client = map(|(id, data)| (id, serde_json::to_string(&data).unwrap())) -> dest_sink(to_client);

        clients_connect = source_stream(clients_connect) -> tee();
        clients_disconnect = source_stream(clients_disconnect);

        // helpers
        peer_broadcast = cross_join::<'tick, 'tick, HalfMultisetJoinState>() -> to_peer;
        all_peers -> [0]peer_broadcast;
        to_peers =  [1]peer_broadcast;

        names = from_client ->
            filter_map(|(client, msg)| if let FromClient::Name(name) = msg { Some((client, name)) } else { None });
        messages = from_client ->
            filter_map(|(client, msg)| if let FromClient::Message { id, text } = msg { Some((client, (id, text))) } else { None });

        clients_connect -> persist::<'static>() -> [pos]active_clients;
        clients_disconnect -> persist::<'static>() -> [neg]active_clients;
        active_clients = difference() -> null();

        // logic
        // echo server
        // messages -> map(|(client, (_, text))| (client, ChatMessage {
        //     name: "server".to_string(),
        //     text,
        // })) -> to_client;

        // replicated chat
        messages -> [0]local_messages;
        names -> persist::<'static>() -> [1]local_messages;
        local_messages = join::<'tick, 'tick, HalfMultisetJoinState>() -> tee();

        local_messages -> map(|(client_id, ((msg_id, text), name))| (ChatMessage {
            name,
            text
        }, self_node_id, client_id, msg_id)) -> all_messages;
        local_messages -> map(|(client_id, ((msg_id, text), name))| PeerMessage {
            msg: ChatMessage {
                name,
                text
            },
            node_id: self_node_id,
            client_id,
            msg_id,
        }) -> to_peers;

        from_peer -> map(|p| (p.msg, p.node_id, p.client_id, p.msg_id)) -> all_messages;

        all_messages = union() /* -> persist::<'static>() -> (PATCH 2) */ -> unique::<'tick>() -> map(|t| t.0);

        broadcast_clients = cross_join::<'static /*'tick (PATCH 1) */, 'static /*'tick, HalfMultisetJoinState (PATCH 2) */>() -> multiset_delta() -> to_client;
        // active_clients -> [0]broadcast_clients; (PATCH 1)
        clients_connect -> [0]broadcast_clients;
        all_messages -> [1]broadcast_clients;
    };

    dfir_rs::util::deploy::launch_flow(df).await;
}
