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

This KVS overwrites the old value when a new value is written to a key. In the general case this is not monotonic
because we are deleting old information. For a more monotonic KVS, see the `kvs` example.

The implementation difference can be found in `server.rs`. This implementation uses a `persist_mut_keyed()`
to enable deletion on the `PUT` side of the `join()`.
```rust
// Store puts mutably (supporting deletion)
puts
    -> flat_map(|(key, value, _addr): (String, Option<String>, _)| {
        match value {
            Some(val) => vec![
                // Clear key then put new value
                PersistenceKeyed::Delete(key.clone()),
                PersistenceKeyed::Persist(key, val),
            ],
            None => vec![
                PersistenceKeyed::Delete(key),
            ],
        }
    })
    -> persist_mut_keyed()
    -> [0]lookup;
gets -> [1]lookup;
// Join PUTs and GETs by key, persisting the PUTs.
lookup = join::<'tick, 'tick>();
```

## Running the example

To run the example, open 2 terminals.

In one terminal run the server like so:
```
cargo run -p hydroflow --example kvs_mut -- --address localhost:12346 --role server
```

In another terminal run a client:
```
cargo run -p hydroflow --example kvs_mut -- --address localhost:12346 --role client
```

Adding the `--graph <graph_type>` flag to the end of the command lines above will print out a node-and-edge diagram of the program. Supported values for `<graph_type>` include [mermaid](https://mermaid-js.github.io/) and [dot](https://graphviz.org/doc/info/lang.html).
