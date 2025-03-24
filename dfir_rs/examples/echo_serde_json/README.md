An example of explicit serialization and deserialization of data, in this case using a JSON encoder.

Ordinarily it's simple and efficient to use `source_stream_serde` and `dest_sink_serde` for network I/O in Hydroflow programs. They automatically serialize and deserialize data on its way in and out read/write stream, such as a UDP port. This example shows how to do the serialization and deserialization explicitly using a JSON encoder. This can be useful for debugging, or as a template for cases where you want to use a different serialization format.

Note that in `main.rs` we use `udp_lines` in place of the usual `udp_bytes` that we use with `source_stream_serde` and `dest_sink_serde`. This is because in this example we delimit messages using newlines. (`udp_lines` uses the `use tokio_util::codec::LinesCodec`; more details can be seen in `hydroflow/src/util/mod.rs`).

To run the example, open 2 terminals.

In one terminal run the server like so:
```
cargo run -p hydroflow --example echo_serde_json -- --address localhost:12347 --role server
```

In another terminal run a client:
```
cargo run -p hydroflow --example echo_serde_json -- --address localhost:12347 --role client
```

If you type in the client terminal the message will be sent to the server, echo'd back to the client and printed with a checksum and server timestamp.

Adding the `--graph <graph_type>` flag to the end of the command lines above will print out a node-and-edge diagram of the program. Supported values for `<graph_type>` include [mermaid](https://mermaid-js.github.io/) and [dot](https://graphviz.org/doc/info/lang.html).
