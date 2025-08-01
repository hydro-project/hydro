## Getting Started
This is a template for a Rust project that uses [Hydroflow](https://github.com/hydro-project/hydro) for
distributed services. It implements a simple echo server and client over UDP.

## Using the Template
```bash
cargo generate gh:hydro-project/dfir-template
```

You will be prompted to name your project. Once the command completes, you can `cd` into the project.

Ensure the correct nightly version of rust is installed:
```bash
rustup update
```

Then test the project:
```bash
cargo test
```

## Running the Template
The server can be run in one terminal and one or more clients can be run in separate terminals.
### Server
```console
% cargo run -- --role server
```

### Client
You can run multiple instances of the client by running the following command:
```console
% cargo run -- --role client
```

## Viewing Help
```console
cargo run -- --help
```

## Template Project Structure
The `src` directory contains the following files:

| File          | Description                                                                                                                          |
|---------------|--------------------------------------------------------------------------------------------------------------------------------------|
| `main.rs`     | Contains `main` entry-point function for both client and server. Performs command-line argument parsing.                             |
| `protocol.rs` | Contains the `Message` enum that defines the messages that can be sent between instances.                                            |
| `<role>.rs`   | Contains the service for the given role. Example implementations and skeletal hydroflow spec are provided for `server` and `client`. |
| `helpers.rs`  | Contains helper functions that are invoked from Hydroflow code in multiple services.                                                 |

## Communication Patterns
No particular communication pattern is assumed by Hydroflow. The unmodified template application is designed to be used in a "star topology":
multiple independent clients talking to a single server. However, the template can be easily modified to support other topologies.
Additional examples are provided in the [hydroflow](https://github.com/hydro-project/hydro) repository in the `hydroflow/examples` directory.

## Where do you go from here?
This template is intended to be a starting point for your own project. You'll undoubtedly want to change it.

In our experience, when starting a Hydroflow project we recommend a four-step approach:

1. **Roles**: Identify the roles that your services will play (in the `Opts` struct in `src/main.rs`)
2. **Messages**: Define the basic message types that services will send to each other (in the `Message` enum in `src/protocol.rs`).
3. **Print Received Messages**: Utilize the template logic at each service that prints out messages received.
4. **Exercise Sending Patterns**:  Make sure the right messages get to the right recipients! Write simple logic to send out messages in all the message patterns you expect to see (in the `src/<role>.rs` files).
5. **Service Programming**: Begin writing the actual logic for each service, with plenty of `inspect(|m| println!("{:?}", m))` operators
peppered throughout!

Have fun!

## Print a Dataflow Graph
The client and server can optionally print out a dataflow graph of their hydroflow code.

### Mermaid
#### Server
Run the following command and view the messages received by the server on stdout.
```console
% cargo run -- --role server --graph mermaid
```

#### Client
Run the following command and type in the messages to send to the server. When the server responds, the echoed message
will be printed on stdout.
```console
% cargo run -- --role client --graph mermaid
```

### Dot
#### Server
```console
% cargo run -- --role server --graph dot
```

#### Client
```console
% cargo run -- --role client --graph dot
```