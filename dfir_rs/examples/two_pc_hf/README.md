## Two Phase Commit
This is a remedial 2PC implementation.

Design limitations:
- No database logging (just log statements via println)
- No distinction between forced and non-forced logs, no presumed commit/abort optimizations
- No recovery manager implementation (yet)
- Subordinates make random decisions whether to commit or abort

### To Run the code:
Look in the file `members.json` to find the addresses of the coordinator and subordinates.
For the coordinator, launch a process on the node with a matching IP address as follows.
Here we assume the coordinator's IP address is `localhost` and port `12346` is free:
```
cargo run --example two_pc_hf -- --path hydroflow/examples/two_pc_hf/members.json --role coordinator --addr localhost:12346
```

Now for each subordinate, launch a process on the node with the matching IP address as follows.
Here we assume the subordinate's IP address is `127.0.0.1` and port `12349` is free:
```
cargo run --example two_pc_hf -- --path hydroflow/examples/two_pc_hf/members.json --role subordinate --addr localhost:12349
```

Now, in the coordinator process you can type an integer at `stdin`. Each integer you type is considered a transaction ID,
and a two-phase commit process is run for that transaction. Votes to commit or abort are randomized.

You should see logging information on screen at both the coordinator and the subordinates.

Adding the `--graph <graph_type>` flag to the end of the command lines above will print out a node-and-edge diagram of the program. Supported values for `<graph_type>` include [mermaid](https://mermaid-js.github.io/) and [dot](https://graphviz.org/doc/info/lang.html).
