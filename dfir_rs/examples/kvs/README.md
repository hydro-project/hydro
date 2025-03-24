Simple single-node key-value store example based on a join of PUTs and GETs.

Current semantics are:
- PUTs are appended: we remember them all forever
- GETs are only remembered for the current tick, which may not be monotone depending on how they
  are consumed.
- GETs for empty keys get no acknowledgement.

Clients accept commands on stdin. Command syntax is as follows:
- `PUT <key>, <value>`
- `GET <key>`
Commands are case-insensitive. All keys and values are treated as `String`s.

## Overwriting values?

This KVS actually stores all values written to a key because deleting old values, in the general case,
is not monotonic. So therefore a read on a particular key will receive all previous writes to that key.
For a more traditional, and non-monotonic KVS, see the `kvs_mut` example.

The implementation difference can be found in `server.rs`. This implementation uses a `join()`
with `'static` persistence on the write side. Every written value is persisted.
```rust
// Join PUTs and GETs by key, persisting the PUTs.
puts -> map(|(key, value, _addr)| (key, value)) -> [0]lookup;
gets -> [1]lookup;
lookup = join::<'static, 'tick>();
```

## Running the example

To run the example, open 2 terminals.

In one terminal run the server like so:
```
cargo run -p hydroflow --example kvs -- --address localhost:12346 --role server
```

In another terminal run a client:
```
cargo run -p hydroflow --example kvs -- --address localhost:12346 --role client
```

Adding the `--graph <graph_type>` flag to the end of the command lines above will print out a node-and-edge diagram of the program. Supported values for `<graph_type>` include [mermaid](https://mermaid-js.github.io/) and [dot](https://graphviz.org/doc/info/lang.html).
