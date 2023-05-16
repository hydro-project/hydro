use clap::{Parser, ValueEnum};
use client::run_client;
use hydroflow::tokio;

mod client;
mod lattices;

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

#[tokio::main]
async fn main() {
    let opts = Opts::parse();
    run_client(opts).await;
}
