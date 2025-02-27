use std::net::SocketAddr;

use clap::{CommandFactory, Parser, Subcommand};
use dfir_rs::util::{bind_udp_bytes, ipv4_resolve};
use dfir_rs::{DemuxEnum, dfir_syntax, tokio};
use gossip_kv::{ClientRequest, ClientResponse, Key};
use tracing::error;

/// CLI program to interact with Layer 0 gossip store.
#[derive(Debug, Parser)]
struct Opts {
    #[clap(short, long, help = "Server address to connect to.")]
    server_address: Option<String>,
}

/// Dummy app for using clap to process commands for interactive CLI.
#[derive(Debug, Parser)]
#[command(multicall = true)]
struct InteractiveApp {
    #[clap(subcommand)]
    commands: InteractiveCommands,
}

#[derive(Debug, Subcommand, DemuxEnum)]
enum InteractiveCommands {
    /// Get a value from the store.
    Get {
        #[arg(value_parser = parse_key, required = true, help = "Key to get")]
        key: Key,
    },
    /// Upsert a value in the store.
    Set {
        #[arg(value_parser = parse_key, required = true, help = "Key to set")]
        key: Key,
        value: String,
    },
    /// Delete a value from the store.
    Delete {
        #[arg(value_parser = parse_key, required = true, help = "Key to delete")]
        key: Key,
    },
    /// Exit the application.
    Exit,
}

/// Allows clap to parse Keys from user input.
fn parse_key(s: &str) -> Result<Key, String> {
    s.parse::<Key>().map_err(|e| e.to_string())
}

/// Parse a command from a line of input.
fn parse_command(line: String) -> Option<InteractiveCommands> {
    // Override how help is handled.
    if line.trim() == "help" {
        InteractiveApp::command()
            .help_template("\nAvailable Commands: \n{subcommands}")
            .print_help()
            .unwrap();
        return None;
    }

    // Split quoted string into parts.
    let line_parts = shlex::split(&line);

    if line_parts.is_none() {
        error!("\nUnable to parse command.");
        return None;
    }

    // Provide split parts to clap to process.
    let maybe_parsed = InteractiveApp::try_parse_from(line_parts.unwrap());

    match maybe_parsed {
        Err(e) => {
            // Problem with the parsed result. This displays some help.
            error!("\n{}", e);
            None
        }
        Ok(cli) => Some(cli.commands),
    }
}

#[dfir_rs::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let opts = Opts::parse();

    // Bind to OS-assigned port on localhost.
    let address = ipv4_resolve("0.0.0.0:0").unwrap();

    // Default to localhost:3000 if not provided.
    let server_address = opts.server_address.map_or_else(
        || ipv4_resolve("localhost:3001").unwrap(),
        |s| ipv4_resolve(&s).unwrap(),
    );

    // Setup UDP sockets for communication.
    let (outbound, inbound, _) = bind_udp_bytes(address).await;

    let mut cli = dfir_syntax! {
        inbound_messages = source_stream_serde(inbound) -> map(Result::unwrap) -> for_each(|(response, _addr): (ClientResponse, SocketAddr)| println!("{:?}", response));

        outbound_messages = union() -> dest_sink_serde(outbound);

        // Parse commands from stdin.
        commands = source_stdin()
            -> filter_map(|line| parse_command(line.unwrap()))
            -> demux_enum::<InteractiveCommands>();

        commands[Get] -> map(|(key,)| (ClientRequest::Get {key}, server_address)) -> outbound_messages;
        commands[Set] -> map(|(key, value)| (ClientRequest::Set {key, value}, server_address)) -> outbound_messages;
        commands[Delete] -> map(|(key,)| (ClientRequest::Delete {key}, server_address)) -> outbound_messages;
        commands[Exit] -> for_each(|()| std::process::exit(0)); // TODO: Graceful shutdown https://github.com/hydro-project/hydro/issues/1253

    };

    cli.run_async().await;
}
