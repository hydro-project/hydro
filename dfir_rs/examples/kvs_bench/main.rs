mod buffer_pool;
mod protocol;
mod server;

use std::collections::HashMap;
use std::num::ParseFloatError;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use bytes::Bytes;
use clap::{Parser, Subcommand, command};
use dfir_lang::graph::{WriteConfig, WriteGraphType};
use futures::Stream;
use tokio::sync::mpsc::UnboundedSender;
use tokio_stream::StreamExt;
use web_time::{Duration, Instant};

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
        /// Number of threads to run on .
        #[clap(long, default_value_t = 1)]
        threads: usize,

        /// `s` value for the zipf sampling distribution for keys.
        #[clap(long, default_value_t = 4.0)]
        dist: f64,

        /// How long to warm up for, in seconds.
        #[clap(long, default_value = "2", value_parser = clap_duration_from_secs)]
        warmup: Duration,

        /// How long to run for, in seconds.
        #[clap(long, default_value = "10", value_parser = clap_duration_from_secs)]
        duration: Duration,

        /// Write the puts/s every second while running.
        #[clap(long, default_value_t = false)]
        report: bool,

        #[clap(long)]
        graph: Option<WriteGraphType>,
        #[clap(flatten)]
        write_config: Option<WriteConfig>,
    },
}

/// Parse duration from float string for clap args.
fn clap_duration_from_secs(arg: &str) -> Result<Duration, ParseFloatError> {
    arg.parse().map(Duration::from_secs_f32)
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
            mut graph,
            mut write_config,
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

                    let (tx, rx) = dfir_rs::util::unbounded_channel::<Bytes>();

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
                    // Only want one node to print the graph since it is the same for all of them.
                    graph.take(),
                    write_config.take(),
                );
            }

            let get_reset_throughputs = || {
                let mut sum = 0;
                for x in throughputs.iter() {
                    sum += x.swap(0, Ordering::SeqCst);
                }

                sum
            };

            let mut total_writes_so_far = 0;

            std::thread::sleep(warmup);

            get_reset_throughputs();
            let start_time = Instant::now();
            let mut time_last_interval = start_time;

            loop {
                if start_time.elapsed() >= duration {
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

#[test]
fn test() {
    use example_test::run_current_example;

    let mut server = run_current_example!("bench --threads 2");
    server.read_regex(r#"[0-9]+\.[0-9]+"#);
}
