use clap::{Parser, ValueEnum};
use client::run_client;
use hydroflow::lang::graph::{WriteConfig, WriteGraphType};
use hydroflow::tokio;
use hydroflow::util::ipv4_resolve;
use server::run_server;
use std::net::SocketAddr;

mod client;
mod helpers;
mod protocol;
mod server;

/// A simple echo server & client generated using the Hydroflow template. The lines starting with
/// `///` contain the message that appears when you run the compiled binary with the '--help'
/// arguments, so feel free to change it to whatever makes sense for your application.
///
/// See https://docs.rs/clap/latest/clap/ for more information.
#[derive(Parser, Debug)]
struct Opts {
    // The `Opts` structure contains the command line arguments accepted by the application and can
    // be modified to suit your requirements. Refer to the clap crate documentation for more
    // information.
    /// The role this application process should assume. The example in the template provides two
    /// roles: server and client. The server echoes whatever message the clients send to it.
    #[clap(value_enum, long)] // value_enum => parse as enum. long => "--role" instead of "-r".
    role: Role, // This is a mandatory argument.

    /// The server's network address. The server listens on this address. The client sends messages
    /// to this address.
    #[clap(long, value_parser = ipv4_resolve)]
    // value_parser => parse "ip:port" using ipv4_resolve
    address: Option<SocketAddr>, // Since this is an Option<T>, it is an optional argument.

    /// If specified, a graph representation of the Hydroflow flow used by the program will be
    /// printed to the console in the specified format. This parameter can be removed if your
    /// application doesn't need this functionality.
    #[clap(long)]
    graph: Option<WriteGraphType>,

    #[clap(flatten)]
    write_config: Option<WriteConfig>,
}

#[hydroflow::main]
/// This is the main entry-point for both `Client` and `Server`.
async fn main() {
    // Parse command line arguments
    let opts = Opts::parse();

    // Run the server or the client based on the role provided in the command-line arguments.
    match opts.role {
        Role::Server => {
            run_server(opts).await;
        }
        Role::Client => {
            run_client(opts).await;
        }
    }
}

/// A running application can assume one of these roles. The launched application process assumes
/// one of these roles, based on the `--role` parameter passed in as a command line argument.
#[derive(Clone, ValueEnum, Debug)]
enum Role {
    Client,
    Server,
}

/// The default server address & port on which the server listens for incoming messages. Clients
/// send message to this address & port.
pub const DEFAULT_SERVER_ADDRESS: &str = "localhost:54321";
