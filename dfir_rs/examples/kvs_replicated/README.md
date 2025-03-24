A less simple version of the `kvs` example, with multi-node replication. Each server can connect to
an existing peer server, and updates will be gossiped between them.

Current semantics are:
 - PUTs are appended: we remember them all forever
 - GETs are only remembered for the current tick, which may not be monotone depending on how they
   are consumed.
 - GETs for empty keys get no acknowledgement.

 Clients accept commands on stdin. Command syntax is as follows:
 - `PUT <key>, <value>`
 - `GET <key>`
 Commands are case-insensitive. All keys and values are treated as `String`s.

## Running the example

We will need four (or more) terminals:

First, start a server:
```
cargo run -p hydroflow --example kvs_replicated -- --address localhost:12346 --role server
```

In another terminal connect a client to the first server:
```
cargo run -p hydroflow --example kvs_replicated -- --address localhost:12346 --role client
```

Then, run a second server to connect to to the first server:
```
cargo run -p hydroflow --example kvs_replicated -- --peer-address localhost:12346 --address localhost:12347 --role server
```

And finally we can run a second client to connect to the second server:
```
cargo run -p hydroflow --example kvs_replicated -- --address localhost:12347 --role client
```

Adding the `--graph <graph_type>` flag to the end of the command lines above will print out a node-and-edge diagram of the program. Supported values for `<graph_type>` include [mermaid](https://mermaid-js.github.io/) and [dot](https://graphviz.org/doc/info/lang.html).
