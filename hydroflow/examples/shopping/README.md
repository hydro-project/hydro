# The Shopping Cart Example from ApPLIED '23
This directory contains the code for the shopping example from the paper 
"Initial Steps Toward a Compiler for Distributed Programs", Hellerstein, et al., ApPLIED, 2023.

The main program includes a driver program in `driver.rs` that generates data from 3 client "sessions" defined in `test_data.rs`. The driver process shuts itself down after a short time since this is just an example.

Code for the BP and SSIV lattices is in `lattices.rs`. Basic types for the shopping scenario are defined in `structs.rs`. The code for the various Hydroflow examples is in `flows/`.

To run the driver on an example from the paper, choose one of the following numbered options from the paper:

1. the original flow (`flows/orig_flow.rs`)
2. the bounded prefix (bp) lattice (`flows/bp_flow.rs`)
3. the sealed set of indexed values (ssiv) lattice (`flows/ssiv_flow.rs`)
4. the sealed set of indexed values (ssiv) lattice with group_by pushed through join (`flows/ssiv_flow_groupby.rs`)
5. decoupled across a network with state at the server (`flows/server_state_flow.rs`)
6. decoupled across a network with state at the client (`flows/client_state_flow.rs`)
7. decoupled across a network with state at a triply-replicated server (`flows/rep_server_flow.rs`)

Then, with your current directory set to the top of the `hydroflow` project, run the driver program, passing the number of your option to the `--opt` flag. E.g:
```
cargo run -p hydroflow --example shopping -- --opt 5
```

Adding the `--graph <graph_type>` flag to the end of the command lines above will print out a node-and-edge diagram of the program. Supported values for `<graph_type>` include [mermaid](https://mermaid-js.github.io/) and [dot](https://graphviz.org/doc/info/lang.html).

For options 1-4, the driver runs a single Hydro transducer (thread) that handles client requests.

For options 5-6, the driver runs two Hydro transducers, one for each side of the network communication.

For option 7, the driver runs four Hydro transducers: one client proxy and 3 server replicas.

Under all options, the driver runs an additional independent Hydro transducer (thread) to receive the output of the flow and print it to the console. The code for this transducer is in `flows/listener_flow.rs`.
