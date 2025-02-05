use std::convert::Infallible;
use std::fmt::Debug;
use std::future::ready;
use std::hash::Hash;
use std::io::Error;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::num::ParseFloatError;
use std::time::Duration;

use clap::Parser;
use dfir_rs::futures::{Sink, SinkExt, StreamExt};
use dfir_rs::tokio_stream::wrappers::IntervalStream;
use dfir_rs::util::{bind_udp_bytes, ipv4_resolve};
use dfir_rs::{bincode, bytes, tokio};
use gossip_kv::membership::{MemberDataBuilder, Protocol};
use gossip_kv::server::{server, SeedNode};
use prometheus::{gather, Encoder, TextEncoder};
use serde::Serialize;
use tracing::{error, info, trace};
use warp::Filter;

use crate::config::{setup_settings_watch, SeedNodeSettings};
use crate::membership::member_name;

mod config;

mod membership;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Parser)]
struct Opts {
    /// Port to listen for gossip messages.
    #[clap(short, long, default_value = "3000")]
    gossip_port: u16,

    /// Port to listen for client requests.
    #[clap(short, long, default_value = "3001")]
    client_port: u16,

    /// The duration (in seconds) between gossip rounds.
    #[clap(short, long, default_value = "5", value_parser = clap_duration_from_secs)]
    gossip_frequency: Duration,

    #[clap(env = "GOSSIP_MEMBER_SUFFIX_LEN", default_value = "4")]
    member_suffix_len: usize,
}

/// Parse duration from float string for clap args.
fn clap_duration_from_secs(arg: &str) -> Result<Duration, ParseFloatError> {
    arg.parse().map(Duration::from_secs_f32)
}

/// Create a SeedNode from a SeedNodeSettings.
/// Performs a DNS lookup on the address.
fn make_seed_node(settings: &SeedNodeSettings) -> SeedNode<SocketAddr> {
    SeedNode {
        id: settings.id.clone(),
        address: ipv4_resolve(&settings.address).unwrap(),
    }
}

/// Handler for the /metrics route. Used to expose prometheus metrics for the server.
async fn metrics_handler() -> Result<impl warp::Reply, Infallible> {
    let encoder = TextEncoder::new();
    let metric_families = gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();

    Ok(warp::reply::with_header(
        buffer,
        "Content-Type",
        encoder.format_type(),
    ))
}

/// Setup serialization for outbound networking messages.
fn setup_outbound_serialization<Outbound, Message>(
    outbound: Outbound,
) -> impl Sink<(Message, SocketAddr), Error = Error>
where
    Outbound: Sink<(bytes::Bytes, SocketAddr), Error = Error>,
    Message: Serialize + Debug + Send + 'static,
{
    outbound.with(|(msg, addr): (Message, SocketAddr)| {
        ready(Ok::<(bytes::Bytes, SocketAddr), Error>((
            dfir_rs::util::serialize_to_bytes(msg),
            addr,
        )))
    })
}

/// Setup deserialization for inbound networking messages.
fn setup_inbound_deserialization<Inbound, Message>(
    inbound: Inbound,
) -> impl dfir_rs::futures::Stream<Item = (Message, SocketAddr)>
where
    Inbound: dfir_rs::futures::Stream<Item = Result<(bytes::BytesMut, SocketAddr), Error>>,
    Message: for<'de> serde::Deserialize<'de> + Debug + Send + 'static,
{
    inbound.filter_map(|input| {
        let mapped = match input {
            Ok((bytes, addr)) => {
                let msg: bincode::Result<Message> = dfir_rs::util::deserialize_from_bytes(&bytes);
                match msg {
                    Ok(msg) => Some((msg, addr)),
                    Err(e) => {
                        error!("Error deserializing message: {:?}", e);
                        None
                    }
                }
            }
            Err(e) => {
                error!("Error receiving message: {:?}", e);
                None
            }
        };
        ready(mapped)
    })
}

#[dfir_rs::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let opts: Opts = Opts::parse();

    // Setup metrics server
    let metrics_route = warp::path("metrics").and_then(metrics_handler);
    tokio::spawn(async move {
        info!("Starting metrics server on port 4003");
        warp::serve(metrics_route).run(([0, 0, 0, 0], 4003)).await;
    });

    // Setup protocol information for this member
    let client_protocol_address =
        SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), opts.client_port);
    let gossip_protocol_address =
        SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), opts.gossip_port);

    let member_data = MemberDataBuilder::new(member_name(opts.member_suffix_len).clone())
        .add_protocol(Protocol::new("gossip".into(), gossip_protocol_address))
        .add_protocol(Protocol::new("client".into(), client_protocol_address))
        .build();

    // Bind to the UDP ports
    let (client_outbound, client_inbound, _) = bind_udp_bytes(client_protocol_address).await;
    let (gossip_outbound, gossip_inbound, _) = bind_udp_bytes(gossip_protocol_address).await;

    info!(
        "Server {:?} listening for client requests on: {:?}",
        member_data.id, client_protocol_address
    );

    // Setup serde for client requests
    let client_ob = setup_outbound_serialization(client_outbound);
    let client_ib = setup_inbound_deserialization(client_inbound);

    // Setup serde for gossip messages
    let gossip_ob = setup_outbound_serialization(gossip_outbound);
    let gossip_ib = setup_inbound_deserialization(gossip_inbound);

    // Setup regular gossip triggers
    let gossip_rx = IntervalStream::new(tokio::time::interval(opts.gossip_frequency)).map(|_| ());

    // Setup watcher for setting changes
    let (_watcher, server_settings, settings_stream) = setup_settings_watch();

    let seed_nodes = server_settings
        .seed_nodes
        .iter()
        .map(make_seed_node)
        .collect::<Vec<_>>();

    let seed_node_stream = settings_stream.map(|settings| {
        trace!("Settings updated. Reloading seed nodes");
        settings.seed_nodes.iter().map(make_seed_node).collect()
    });

    // Create and run the server
    let mut server = server(
        client_ib,
        client_ob,
        gossip_ib,
        gossip_ob,
        gossip_rx,
        member_data,
        seed_nodes,
        seed_node_stream,
    );

    server.run_async().await;
}
