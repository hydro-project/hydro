#![feature(core_intrinsics)]

mod buffer_pool;
mod protocol;
mod server;

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use bytes::Bytes;
use clap::{command, Parser, Subcommand};
use futures::Stream;
use tokio::sync::mpsc::UnboundedSender;
use tokio_stream::StreamExt;

use crate::protocol::NodeId;
use crate::server::run_server;

#[derive(Debug, Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Bench {
        #[clap(long, default_value_t = 1)]
        threads: usize,

        #[clap(long, default_value_t = 4.0)]
        dist: f64,

        #[clap(long, default_value_t = 2)]
        warmup: u64,

        #[clap(long, default_value_t = 10)]
        duration: u64,

        #[clap(long, default_value_t = false)]
        report: bool,

        #[clap(long, default_value_t = false)]
        print_mermaid: bool,
    },
}

pub struct Topology<RX>
where
    RX: Stream<Item = (usize, Bytes)>,
{
    pub lookup: Vec<usize>,
    pub tx: Vec<UnboundedSender<Bytes>>,
    pub rx: Vec<RX>,
}

impl<RX> Default for Topology<RX>
where
    RX: Stream<Item = (usize, Bytes)> + StreamExt + Unpin,
{
    fn default() -> Self {
        Self {
            lookup: Default::default(),
            tx: Default::default(),
            rx: Default::default(),
        }
    }
}

fn main() {
    match Cli::parse().command {
        Commands::Bench {
            threads,
            dist,
            warmup,
            duration,
            report,
            mut print_mermaid,
        } => {
            let mut throughputs = Vec::new();
            let mut nodes: HashMap<NodeId, Topology<_>> = HashMap::default();
            // let mut receivers_for: HashMap<NodeId, Vec<_>> = HashMap::default();

            for n1 in 0..threads {
                throughputs.push(Arc::new(AtomicUsize::new(0)));

                nodes.entry(n1).or_default();

                for n2 in 0..threads {
                    if n2 == n1 {
                        continue;
                    }

                    let (tx, rx) = hydroflow::util::unbounded_channel::<Bytes>();

                    {
                        let entry = nodes.entry(n1).or_default();

                        entry.lookup.push(n2);
                        entry.tx.push(tx);
                    }

                    {
                        nodes
                            .entry(n2)
                            .or_default()
                            .rx
                            .push(rx.map(move |x| (n2, x)));
                    }
                }
            }

            for (node_id, topology) in nodes {
                run_server(
                    node_id,
                    topology,
                    dist,
                    throughputs[node_id].clone(),
                    print_mermaid,
                );

                print_mermaid = false; // Only want one node to print the mermaid since it is the same for all of them.
            }

            let get_reset_throughputs = || {
                let mut sum = 0;
                for x in throughputs.iter() {
                    sum += x.swap(0, Ordering::SeqCst);
                }

                sum
            };

            let mut total_writes_so_far = 0;

            std::thread::sleep(Duration::from_secs(warmup));

            get_reset_throughputs();
            let start_time = Instant::now();
            let mut time_last_interval = start_time;

            loop {
                if start_time.elapsed().as_secs_f64() >= duration as f64 {
                    break;
                }

                std::thread::sleep(Duration::from_secs(1));

                if report {
                    let writes_this_interval = get_reset_throughputs();
                    let puts =
                        writes_this_interval as f64 / time_last_interval.elapsed().as_secs_f64();
                    time_last_interval = Instant::now();
                    println!("{puts}");

                    total_writes_so_far += writes_this_interval;
                }
            }

            total_writes_so_far += get_reset_throughputs();
            let puts = total_writes_so_far as f64 / start_time.elapsed().as_secs_f64();

            println!("{puts}");
        }
    }
}
