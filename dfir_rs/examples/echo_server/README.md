Simple echo server example.

To run the example, open 2 terminals.

In one terminal run the server like so:
```
cargo run -p hydroflow --example echo_server -- --address localhost:12347 --role server
```

In another terminal run a client:
```
cargo run -p hydroflow --example echo_server -- --address localhost:12347 --role client
```

If you type in the client terminal the message will be sent to the server, echo'd back to the client and printed with a checksum and server timestamp.

Adding the `--graph <graph_type>` flag to the end of the command lines above will print out a node-and-edge diagram of the program. Supported values for `<graph_type>` include [mermaid](https://mermaid-js.github.io/) and [dot](https://graphviz.org/doc/info/lang.html).
