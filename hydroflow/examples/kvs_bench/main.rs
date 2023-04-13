#[cfg(not(target_family = "windows"))]
#[path = "."]
mod src {
    mod buffer_pool;
    mod protocol;
    mod server;
    mod util;
}
#[cfg(not(target_family = "windows"))]
use src::*;

#[cfg(not(target_family = "windows"))]
fn main() {

    use server::run_server;

    use std::sync::atomic::AtomicUsize;
    use std::sync::atomic::Ordering;
    use std::sync::Arc;
    use std::time::Duration;

    use clap::command;
    use clap::Parser;
    use clap::Subcommand;

    #[derive(Debug, Parser)]
    struct Cli {
        #[command(subcommand)]
        command: Commands,
    }

    #[derive(Debug, Subcommand)]
    enum Commands {
        #[command(arg_required_else_help = true)]
        Bench {
            #[clap(long)]
            threads: usize,

            #[clap(long)]
            dist: f64,
        },
    }
    let ctx = tmq::Context::new();

    let throughput = Arc::new(AtomicUsize::new(0));

    match Cli::parse().command {
        Commands::Bench { threads, dist } => {
            let topology: Vec<_> = (0..threads).collect();

            for addr in topology.iter() {
                run_server(
                    *addr,
                    topology.clone(),
                    dist,
                    ctx.clone(),
                    throughput.clone(),
                );
            }
        }
    }

    std::thread::sleep(Duration::from_millis(2000));

    throughput.store(0, Ordering::SeqCst);
    let start_time = std::time::Instant::now();

    std::thread::sleep(Duration::from_millis(12000));
    let puts = throughput.load(Ordering::SeqCst) as f64 / start_time.elapsed().as_secs_f64();
    println!("{puts}");
}

#[cfg(target_family = "windows")]
fn main() {
    unimplemented!();
}
