Simple echo server with Lamport-clock stamped messages.

To run the example, open 3 terminals.

In one terminal run the server like so:
```
cargo run -p hydroflow --example lamport_clock -- --address localhost:12347 --role server
```

In another terminal run a client:
```
cargo run -p hydroflow --example lamport_clock -- --address localhost:12347 --role client
```

In a third terminal run a client:
```
cargo run -p hydroflow --example lamport_clock -- --address localhost:12347 --role client
```

If you type in a client terminal the message will be sent to the server, echo'd back to the client and printed with a Lamport clock.

Adding the `--graph <graph_type>` flag to the end of the command lines above will print out a node-and-edge diagram of the program. Supported values for `<graph_type>` include [mermaid](https://mermaid-js.github.io/) and [dot](https://graphviz.org/doc/info/lang.html).
