// Test harness for the various implementations of shopping carts.

use clap::{Parser, ValueEnum};
use driver::run_driver;
use hydroflow::tokio;

mod driver;
mod flows;
mod lattices;
mod structs;
mod test_data;
mod wrappers;

#[derive(Clone, ValueEnum, Debug)]
enum Role {
    Client,
    Server,
}
#[derive(Clone, ValueEnum, Debug)]
enum GraphType {
    Mermaid,
    Dot,
    Json,
}

#[derive(Parser, Debug)]
struct Opts {
    #[clap(value_enum, long)]
    graph: Option<GraphType>,
    #[clap(long)]
    opt: usize,
}

#[hydroflow::main]
async fn main() {
    let opts = Opts::parse();

    // all the interesting logic is in the driver
    run_driver(opts).await;
}
