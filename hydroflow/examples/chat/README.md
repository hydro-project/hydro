## Chat Examples
There are two flavors of the chat example. The first is a broadcast-based example in which multiple clients connect to a
single server. Whenever a client wishes to send a message, it sends it to the server and the server broadcasts it to
the other clients. The second example is a gossip-based, multi-server example. The example is compatible with the same
client used in the broadcast-based example. Clients can connect to any one of the  running servers. As before, when a 
client wishes to send a message, it sends it to the server. The server gossips the message to other servers using a 
gossip algorithm. Whenever a server learns of a new message, it is broadcast to all the clients connected to it.

### Broadcast Example

To run the example, open 3 terminals.

#### Running the Server
In one terminal run the server like so:
```shell
cargo run -p hydroflow --example chat -- --name "_" --role server
```

#### Running the Clients
In another terminal run the first client:
```shell
cargo run -p hydroflow --example chat -- --name "alice" --role client
```

In the third terminal run the second client:
```shell
cargo run -p hydroflow --example chat -- --name "bob" --role client
```

If you type in the client terminals the messages should appear everywhere.

### Gossip Example
#### Running the Servers
The gossip-based servers rely on static membership for discovery. The servers run a gossip protocol (parallel to the 
client-server protocol). The roles `gossiping-server1`, ..., `gossiping-server5` determine which pre-configured port
will be used by the server to send/receive gossip protocol messages.

In (up to) five separate tabs, run the following servers. 

##### First
```shell
cargo run -p hydroflow --example chat -- --name "_" --address 127.0.0.1:12345  --role gossiping-server1 
```
##### Second
```shell
cargo run -p hydroflow --example chat -- --name "_" --address 127.0.0.1:12346  --role gossiping-server2 
```
##### Third
```shell
cargo run -p hydroflow --example chat -- --name "_" --address 127.0.0.1:12347  --role gossiping-server3 
```

##### Fourth
```shell
cargo run -p hydroflow --example chat -- --name "_" --address 127.0.0.1:12348  --role gossiping-server4 
```

##### Fifth
```shell
cargo run -p hydroflow --example chat -- --name "_" --address 127.0.0.1:12349  --role gossiping-server5 
```
#### Running the Clients
In another terminal run the first client:
```shell
cargo run -p hydroflow --example chat -- --name "alice" --address 127.0.0.1:12345 --role client
```

In another terminal run the second client:
```shell
cargo run -p hydroflow --example chat -- --name "bob" --address 127.0.0.1:12349 --role client
```

If you type in the client terminals the messages should appear everywhere. Give it a few seconds though - unlike the
broadcast example, the message delivery isn't instantaneous. The gossip protocol runs in cycles and it could take a few
cycles for the message to be delivered everywhere.

### Dump Graphs of the Flows
#### Client
```shell
cargo run -p hydroflow --example chat -- --name "alice" --role client --graph mermaid
```
#### Broadcast Server
```shell
cargo run -p hydroflow --example chat -- --name "_" --role server --graph mermaid
```

#### Gossip Server
```shell
cargo run -p hydroflow --example chat -- --name "_" --role gossiping-server1 --graph mermaid
```

### Display Help
```shell
cargo run -p hydroflow --example chat -- --help
```