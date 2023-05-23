---
sidebar_position: 8
---

# Networked Services 1: EchoServer
> In this example we cover:
> * The standard project template for networked Hydroflow services.
> * Rust's `clap` crate for command-line options
> * Defining message types
> * Destination operators (e.g. for sending data to a network)
> * Network sources and dests with built-in serde (`source_stream_serde`, `dest_sink_serde`)
> * The `source_stdin` source
> * Long-running services via `run_async`

Our examples up to now have been simple single-node programs, to get us comfortable with Hydroflow's
surface syntax. But the whole point of Hydroflow is to help us write distributed programs or services that run on a cluster of machines!

In this example we'll study the "hello, world" of distributed systems -- a simple echo server. It will listen on a UDP port,
and send back a copy of any message it receives, with a timestamp. We will also look at a client to 
accept strings from the command line, send them to the echo server, and print responses.

We will use a fresh `hydroflow-template` project template to get started. Change to the directory where you'd like to put your project, and once again run:
```bash
cargo generate hydro-project/hydroflow-template
```
Then change directory into the resulting project.

The `README.md` for the template project is a good place to start. It contains a brief overview of the project structure, and how to build and run the example. Here we'll spend more time learning from the code.

## Hydroflow Project Structure
The Hydroflow template project auto-generates this example for us. If you prefer, you can find the source in the `examples/echo_server` directory of the Hydroflow repository.

The directory structure encouraged by the template is as follows:
```txt
project/README.md           # documentation
project/Cargo.toml          # package and dependency info
project/src/main.rs         # main function
project/src/protocol.rs     # message types exchanged between roles
project/src/helpers.rs      # helper functions used by all roles
project/src/<roleA>.rs      # service definition for role A (e.g. server)
project/src/<roleB>.rs      # service definition for role B (e.g. client)
```
In the default example, the roles we use are `Client` and `Server`, but you can imagine different roles depending on the structure of your service or application.

## main.rs
We start with a `main` function that parses command-line options, and invokes the appropriate
role-specific service.
After a prelude of imports, we start by defining a Rust `enum` for the `Role`s that the service supports. 

```rust,ignore
use clap::{Parser, ValueEnum};
use client::run_client;
use hydroflow::tokio;
use hydroflow::util::{bind_udp_bytes, ipv4_resolve};
use server::run_server;
use std::net::SocketAddr;

mod client;
mod helpers;
mod protocol;
mod server;

#[derive(Clone, ValueEnum, Debug)]
enum Role {
    Client,
    Server,
}
```

Following that, we use Rust's [`clap`](https://docs.rs/clap/latest/clap/) (Command Line Argument Parser) crate to parse command-line options:

```rust,ignore
#[derive(Parser, Debug)]
struct Opts {
    #[clap(value_enum, long)]
    role: Role,
    #[clap(long, value_parser = ipv4_resolve)]
    addr: Option<SocketAddr>,
    #[clap(long, value_parser = ipv4_resolve)]
    server_addr: Option<SocketAddr>,
}
```
This sets up 3 command-line flags: `role`, `addr`, and `server_addr`. Note how the `addr` and `server_addr` flags are made optional via wrapping in a Rust `Option`; by contrast, the `role` option is required. The `clap` crate will parse the command-line options and populate the `Opts` struct with the values. `clap` handles parsing the command line strings into the associated Rust types --  the `value_parser` attribute tells `clap` to use Hydroflow's `ipv4_resolve` helper function to parse a string like "127.0.0.1:6552" into a `SocketAddr`.

This brings us to the `main` function itself. It is prefaced by a `#[tokio::main]` attribute, which is a macro that sets up the tokio runtime. This is necessary because Hydroflow uses the tokio runtime for asynchronous execution as a service.  

```rust,ignore
#[tokio::main]
async fn main() {
    // parse command line arguments
    let opts = Opts::parse();
    // if no addr was provided, we ask the OS to assign a local port by passing in "localhost:0"
    let addr = opts
        .addr
        .unwrap_or_else(|| ipv4_resolve("localhost:0").unwrap());

    // allocate `outbound` sink and `inbound` stream
    let (outbound, inbound, addr) = bind_udp_bytes(addr).await;
    println!("Listening on {:?}", addr);
```

After parsing the command line arguments we set up some Rust-based networking. Specifically, for either client or server roles we will need to allocate a UDP socket that is used for both sending and receiving messages. We do this by calling the async `bind_udp_bytes` function, which is defined in the `hydroflow/src/util` module. As an async function it returns a `Future`, so requires appending `.await`; the function returns a triple of type `(UdpSink, UdpSource, SocketAddr)`. The first two are the types that we'll use in Hydroflow to send and receive messages, respectively. (Note: your IDE might expand out the `UdpSink` and `UdpSource` traits to their more verbose definitions. That is fine; you can ignore those.) The SocketAddr is there in case you specified port 0 in your `addr` argument, in which case this return value tells you what port the OS has assigned for you.

All that's left is to fire up the code for the appropriate role!
```rust,ignore
    match opts.role {
        Role::Server => {
            run_server(outbound, inbound, opts).await;
        }
        Role::Client => {
            run_client(outbound, inbound, opts).await;
        }
    }
}
```

## protocol.rs
As a design pattern, it is natural in distributed Hydroflow programs to define various message types in a `protocol.rs` file with structures shared for use by all the Hydroflow logic across roles. In this simple example, we define only one message type: `EchoMsg`, and a simple struct with two fields: `payload` and `ts` (timestamp). The `payload` field is a string, and the `ts` field is a `DateTime<Utc>`, which is a type from the [`chrono`](https://docs.rs/chrono/latest/chrono/) crate. Note the various derived traits on `EchoMsg`—specifically `Serialize` and `Deserialize`—these are required for structs that we send over the network.

```rust,ignore
use chrono::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Clone, Serialize, Deserialize, Debug)]
pub struct EchoMsg {
    pub payload: String,
    pub ts: DateTime<Utc>,
}
```

# server.rs
Things get interesting when we look at the `run_server` function. This function is the main entry point for the server. It takes as arguments the `outbound` and `inbound` sockets from `main`, and the `Opts` (which are ignored). 

After printing a cheery message, we get into the Hydroflow code for the server, consisting of three short pipelines.

```rust,ignore
use crate::protocol::EchoMsg;
use chrono::prelude::*;
use hydroflow::hydroflow_syntax;
use hydroflow::scheduled::graph::Hydroflow;
use hydroflow::util::{UdpSink, UdpStream};
use std::net::SocketAddr;

pub(crate) async fn run_server(outbound: UdpSink, inbound: UdpStream, _opts: crate::Opts) {
    println!("Server live!");

    let mut flow: Hydroflow = hydroflow_syntax! {
        // Define a shared inbound channel
        inbound_chan = source_stream_serde(inbound) -> tee();

        // Print all messages for debugging purposes
        inbound_chan[0]
            -> for_each(|(msg, addr): (EchoMsg, SocketAddr)| println!("{}: Got {:?} from {:?}", Utc::now(), msg, addr));

        // Echo back the Echo messages with updated timestamp
        inbound_chan[1]
            -> map(|(EchoMsg {payload, ..}, addr)| (EchoMsg { payload, ts: Utc::now() }, addr) ) -> dest_sink_serde(outbound);
    };

    // run the server
    flow.run_async().await;
}
```
Lets take the Hydroflow code one statement at a time. 

The first pipeline, `inbound_chan` uses a source operator we have not seen before, [`source_stream_serde()`](../syntax/surface_ops.gen.md#source_stream_serde). This is a streaming source like `source_stream`, but for network streams. It takes a `UdpSource` as an argument, and has a particular output type: a stream of `(T, SocketAddr)` pairs where `T` is some type that implements the `Serialize` and `Deserialize` traits (together known as "serde"), and `SocketAddr` is the network address of the sender of the item. In this case, `T` is `EchoMsg`, which we defined in `protocol.rs`, and the `SocketAddr` is the address of the client that sent the message. We pipe the result into a `tee()` for reuse.

The second pipeline is a simple `for_each` to print the messages received at the server.

The third and final pipeline constructs a response `EchoMsg` with the local timestamp copied in. It then pipes the result into a `dest_XXX` operator—the first that we've seen!  A dest is the opposite of a `source_XXX` operator: it can go at the end of a pipeline and sends data out on a tokio channel. The specific operator used here is [`dest_sink_serde()`](../syntax/surface_ops.gen.md#dest_sink_serde). This is a dest operator like `dest_sink`, but for network streams. It takes a `UdpSink` as an argument, and requires a particular input type: a stream of `(T, SocketAddr)` pairs where `T` is some type that implements the `Serialize` and `Deserialize` traits, and `SocketAddr` is the network address of the destination. In this case, `T` is once again `EchoMsg`, and the `SocketAddr` is the address of the client that sent the original message.

The remaining line of code runs the server. The `run_async()` function is a method on the `Hydroflow` type. It is an async function, so we append `.await` to the call. The program will block on this call until the server is terminated.
## client.rs
The client begins by making sure the user specified a server address at the command line. After printing a message to the terminal, it constructs a hydroflow graph.

Again, we start the hydroflow code defining shared inbound and outbound channels. The code here is simplified compared
to the server because the `inbound_chan` and `outbound_chan` are each referenced only once, so they do not require `tee` or `merge` operators, respectively (they have been commented out).

The `inbound_chan` drives a pipeline that prints messages to the screen. 

Only the last pipeline is novel for us by now. It uses another new source operator [`source_stdin()`](../syntax/surface_ops.gen.md#source_stdin), which does what you might expect: it streams lines of text as they arrive from `stdin` (i.e. as they are typed into a terminal). It then uses a `map` to construct an `EchoMsg` with each line of text and the current timestamp. The result is piped into a sink operator [`dest_sink_serde()`](../syntax/surface_ops.gen.md#dest_sink_serde), which sends the message to the server.

The client logic ends by launching the flow graph with `flow.run_async().await.unwrap()`.

```rust,ignore
use crate::protocol::EchoMsg;
use crate::Opts;
use chrono::prelude::*;
use hydroflow::hydroflow_syntax;
use hydroflow::util::{UdpSink, UdpStream};
use std::net::SocketAddr;

pub(crate) async fn run_client(outbound: UdpSink, inbound: UdpStream, opts: Opts) {
    // server_addr is required for client
    let server_addr = opts.server_addr.expect("Client requires a server address");
    println!("Client live!");

    let mut flow = hydroflow_syntax! {
        // Define shared inbound and outbound channels
        inbound_chan = source_stream_serde(inbound)
            // -> tee() // commented out since we only use this once in the client template
        ;
        outbound_chan = // merge() ->  // commented out since we only use this once in the client template
            dest_sink_serde(outbound);

        // Print all messages for debugging purposes
        inbound_chan
            -> for_each(|(m, a): (EchoMsg, SocketAddr)| println!("{}: Got {:?} from {:?}", Utc::now(), m, a));

        // take stdin and send to server as an Message::Echo
        source_stdin() -> map(|l| (EchoMsg{ payload: l.unwrap(), ts: Utc::now(), }, server_addr) )
            -> outbound_chan;
    };

    flow.run_async().await.unwrap();
}
```

## Running the example
As described in the `README.md` file, we can run the server in one terminal, and the client in another. The server will print the messages it receives, and the client will print the messages it receives back from the server. The client and servers' `--server-addr' arguments need to match or this won't work!

Fire up the server in terminal 1:
```console
#shell-command-next-line
cargo run -p hydroflow --example echoserver -- --role server --addr localhost:12347
```

Then start the client in terminal 2 and type some messages!
```console
#shell-command-next-line
cargo run -p hydroflow --example echoserver -- --role client --server-addr localhost:12347
Listening on 127.0.0.1:54532
Connecting to server at 127.0.0.1:12347
Client live!
This is a test
2022-12-15 05:40:11.258052 UTC: Got Echo { payload: "This is a test", ts: 2022-12-15T05:40:11.257145Z } from 127.0.0.1:12347
This is the rest
2022-12-15 05:40:14.025216 UTC: Got Echo { payload: "This is the rest", ts: 2022-12-15T05:40:14.023577Z } from 127.0.0.1:12347
```

And have a look back at the server console!
```console
Listening on 127.0.0.1:12347
Server live!
2022-12-15 05:40:11.256640 UTC: Got Echo { payload: "This is a test", ts: 2022-12-15T05:40:11.254207Z } from 127.0.0.1:54532
2022-12-15 05:40:14.023363 UTC: Got Echo { payload: "This is the rest", ts: 2022-12-15T05:40:14.020897Z } from 127.0.0.1:54532
```
