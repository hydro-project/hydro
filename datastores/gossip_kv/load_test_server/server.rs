use std::convert::Infallible;
use std::num::{NonZeroU32, ParseFloatError};
use std::thread::sleep;
use std::time::Duration;

use clap::Parser;
use dfir_rs::util::{unbounded_channel, unsync_channel};
use gossip_kv::membership::{MemberDataBuilder, Protocol};
use gossip_kv::{ClientRequest, GossipMessage, Key};
use governor::{Quota, RateLimiter};
use lazy_static::lazy_static;
use prometheus::{gather, register_int_counter, Encoder, IntCounter, TextEncoder};
use tokio::sync::mpsc::{Sender, UnboundedSender};
use tokio::sync::watch::Receiver;
use tokio::task;
use tracing::{error, info, trace};
use warp::Filter;

type LoadTestAddress = u64;

use dfir_rs::futures::sink::drain;
use dfir_rs::futures::stream;
use dfir_rs::tokio_stream::wrappers::UnboundedReceiverStream;
use dfir_rs::tokio_stream::StreamExt;
use dfir_rs::util::unsync::mpsc::bounded;
use gossip_kv::server::{server, SeedNode};
use lattices::cc_traits::Iter;

const UNKNOWN_ADDRESS: LoadTestAddress = 9999999999;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Parser)]
struct Opts {
    /// Number of threads to run. Each thread will run an instance of the gossip-kv server transducer.
    #[clap(short, long, default_value = "1")]
    thread_count: usize,

    /// Frequency (in seconds) at which to send gossip messages.
    #[clap(short, long, default_value = "10", value_parser = clap_duration_from_secs)]
    gossip_frequency: Duration,
}

lazy_static! {
    pub static ref SETS_SENT: IntCounter =
        register_int_counter!("sets_sent", "Counts the number of SET requests sent.").unwrap();
}

/// Parse duration from float string for clap args.
fn clap_duration_from_secs(arg: &str) -> Result<Duration, ParseFloatError> {
    arg.parse().map(Duration::from_secs_f32)
}

fn run_server(
    server_name: String,
    gossip_address: LoadTestAddress,
    gossip_input_rx: UnboundedReceiverStream<(GossipMessage, LoadTestAddress)>,
    switchboard: Switchboard,
    seed_nodes: Vec<SeedNode<LoadTestAddress>>,
    opts: Opts,
) {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        let (client_input_tx, client_input_rx) = bounded(1000);

        let (gossip_output_tx, mut gossip_output_rx) = unsync_channel(None);

        let (gossip_trigger_tx, gossip_trigger_rx) = unbounded_channel();

        let member_data = MemberDataBuilder::new(server_name.clone())
            .add_protocol(Protocol::new("gossip".into(), gossip_address))
            .build();

        rt.block_on(async {
            let local = task::LocalSet::new();
            // local.spawn_local(async move {
            //     // let key_master: Key = "/usr/table/key".parse().unwrap();
            //     loop {
            //         let request = ClientRequest::Set {
            //             key: 100,
            //             value: "FOOBAR".to_string(),
            //         };
            //         client_input_tx.send((request, UNKNOWN_ADDRESS)).await.unwrap();
            //         SETS_SENT.inc();
            //     }
            // });

            // Gossip trigger
            let gossip_frequency = opts.gossip_frequency;
            local.spawn_local(async move {
                loop {
                    tokio::time::sleep(gossip_frequency).await;
                    gossip_trigger_tx.send(()).unwrap();
                }
            });

            // Networking
            local.spawn_local(async move {
                while let Some((msg, addr)) = gossip_output_rx.next().await {
                    trace!("Sending gossip message: {:?} to {}", msg, addr);
                    let outbox = switchboard.gossip_outboxes.get(addr as usize).unwrap();
                    if let Err(e) = outbox.send((msg, gossip_address)) {
                        error!("Failed to send gossip message: {:?}", e);
                    }
                }
            });

            local.spawn_local(async {
                let mut server = server(
                    client_input_rx,
                    drain(), // Ignoring client responses for now.
                    gossip_input_rx,
                    gossip_output_tx,
                    gossip_trigger_rx,
                    member_data,
                    seed_nodes,
                    stream::empty(),
                );

                server.run_async().await
            });

            local.await
        });
    });
}

struct Switchboard {
    gossip_outboxes: Vec<UnboundedSender<(GossipMessage, LoadTestAddress)>>,
}

impl Clone for Switchboard {
    fn clone(&self) -> Self {
        Self {
            gossip_outboxes: self.gossip_outboxes.clone(),
        }
    }
}

impl Switchboard {
    fn new() -> Self {
        Self {
            gossip_outboxes: Vec::new(),
        }
    }
    fn new_outbox(
        &mut self,
    ) -> (
        LoadTestAddress,
        UnboundedReceiverStream<(GossipMessage, LoadTestAddress)>,
    ) {
        let addr: LoadTestAddress = self.gossip_outboxes.len() as LoadTestAddress;
        let (tx, rx) = unbounded_channel();
        self.gossip_outboxes.push(tx);
        (addr, rx)
    }
}

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

fn main() {
    tracing_subscriber::fmt::init();

    let opts: Opts = Opts::parse();

    std::thread::spawn(move || {
        let metrics_route = warp::path("metrics").and_then(metrics_handler);

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async move {
            info!("Starting metrics server on port 4003");
            warp::serve(metrics_route).run(([0, 0, 0, 0], 4003)).await;
        });
    });

    info!("Starting load test with with {} threads", opts.thread_count);

    let mut switchboard = Switchboard::new();

    let outboxes: Vec<_> = (0..opts.thread_count)
        .map(|_| {
            let (addr, rx) = switchboard.new_outbox();
            (format!("SERVER-{}", addr), addr, rx)
        })
        .collect();

    let seed_nodes: Vec<_> = outboxes
        .iter()
        .map(|(name, addr, _)| SeedNode {
            id: name.clone(),
            address: *addr,
        })
        .collect();

    outboxes.into_iter().for_each(|(name, addr, outbox)| {
        run_server(
            name,
            addr,
            outbox,
            switchboard.clone(),
            seed_nodes.clone(),
            opts,
        );
    });

    loop {
        sleep(Duration::from_secs(1));
    }
}
