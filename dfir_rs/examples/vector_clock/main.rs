use std::net::SocketAddr;

use clap::{Parser, ValueEnum};
use client::run_client;
use dfir_lang::graph::{WriteConfig, WriteGraphType};
use dfir_rs::util::ipv4_resolve;
use server::run_server;

mod client;
mod helpers;
mod protocol;
mod server;

#[dfir_rs::main]
async fn main() {
    let opts = Opts::parse();
    match opts.role {
        Role::Server => {
            run_server(opts).await;
        }
        Role::Client => {
            run_client(opts).await;
        }
    }
}

// The `Opts` structure contains the command line arguments accepted by the application and can
// be modified to suit your requirements. Refer to the clap crate documentation for more
// information.  The lines starting with
// `///` contain the message that appears when you run the compiled binary with the '--help'
// arguments, so feel free to change it to whatever makes sense for your application.
// See https://docs.rs/clap/latest/clap/ for more information.
/// Echo server with vector clock stamped messages.
#[derive(Parser, Debug)]
struct Opts {
    /// The role this application process should assume. The example in the template provides two
    /// roles: server and client.
    #[clap(value_enum, long)] // value_enum => parse as enum. long => "--role" instead of "-r".
    role: Role,

    /// The server's network address. The server listens on this address. The client sends messages
    /// to this address. Format is `"ip:port"`.
    // `value_parser`: parse using ipv4_resolve
    #[clap(long, value_parser = ipv4_resolve, default_value = DEFAULT_SERVER_ADDRESS)]
    address: SocketAddr,

    /// If specified, a graph representation of the flow used by the program will be
    /// printed to the console in the specified format. This parameter can be removed if your
    /// application doesn't need this functionality.
    #[clap(long)]
    graph: Option<WriteGraphType>,

    #[clap(flatten)]
    write_config: Option<WriteConfig>,
}

/// The default server address & port on which the server listens for incoming messages. Clients
/// send message to this address & port.
pub const DEFAULT_SERVER_ADDRESS: &str = "localhost:12053";

/// A running application can assume one of these roles. The launched application process assumes
/// one of these roles, based on the `--role` parameter passed in as a command line argument.
#[derive(Clone, ValueEnum, Debug)]
enum Role {
    Client,
    Server,
}

#[test]
fn test_vector_clock() {
    use example_test::run_current_example;

    let mut server = run_current_example!("--role server --address 127.0.0.1:12053");
    server.read_string("Server is live! Listening on 127.0.0.1:12053");

    let mut client1 = run_current_example!("--role client --address 127.0.0.1:12053");
    let mut client2 = run_current_example!("--role client --address 127.0.0.1:12053");

    client1.read_string("Client is live! Listening on 127.0.0.1:");
    client2.read_string("Client is live! Listening on 127.0.0.1:");

    client1.write_line("Hello1");
    client1.read_regex(
        r#"payload: "Hello1", vc: .*"127.0.0.1:12053": Max\(1\).*from 127.0.0.1:12053"#,
    );

    client2.write_line("Hello2");
    client2.read_regex(
        r#"payload: "Hello2", vc: .*"127.0.0.1:12053": Max\(2\).*from 127.0.0.1:12053"#,
    );
}
