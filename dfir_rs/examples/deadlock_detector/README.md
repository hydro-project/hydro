# Deadlock Detector

This is a distributed deadlock detector. I.e. it finds 
cycles in a graph that has been partitioned across peers.
In the demo, new waits-for edges can be added at each node via the command line, and
cycles are printed to stdio in canonical order (starting with the lowest node id in the cycle).

Some comments:
1. This is a purely monotonic, streaming implementation that will report cycles as it finds them.
2. Each local node runs transitive closure from scratch every tick; there is currently no materialization of paths.
3. You can add waits-for edges, but you cannot (currently) delete them.
4. Many networking topologies would work. We chose to implement a peer-to-peer architecture with random gossip for this example. Each message is sent to each node with a fixed probability. Because it's peer-to-peer, every node will eventually learn about and emit each cycle. This was an arbitrary design choice. A more natural coordinator-subordinate architecture would likely make sense, but would be quite similar to the 2PC example.
5. We have not implemented termination detection (which is non-monotone) so this will run until you interrupt it.

### To Run the code:
Look in the file `members.json` to find the addresses of the peers. 
For each, launch a process on the node with a matching IP address as follows.
Here we assume the peer's IP address is `localhost` and ports `12346`-`12348` are free:
```
cargo run --example deadlock_detector -- --path hydroflow/examples/deadlock_detector/peers.json --addr 127.0.0.1 --port 12346 
cargo run --example deadlock_detector -- --path hydroflow/examples/deadlock_detector/peers.json --addr 127.0.0.1 --port 12347 
cargo run --example deadlock_detector -- --path hydroflow/examples/deadlock_detector/peers.json --addr 127.0.0.1 --port 12348
```

Now, you will be prompted to type an integer pair at `stdin`. Each pair `(x, y)` you type is a *waits-for* edge. E.g.,
if you type `(2, 4)` you are indicating that transaction `2` is waiting for transaction `4`. (Ordinarily this information 
would come from another module like a lock manager). The edges across all peers represents the current *waits-for* relation

Adding the `--graph <graph_type>` flag to the end of the command lines above will print out a node-and-edge diagram of the program. Supported values for `<graph_type>` include [`mermaid`](https://mermaid-js.github.io/), [`dot`](https://graphviz.org/doc/info/lang.html) and `json`.

