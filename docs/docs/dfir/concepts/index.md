# Concepts
DFIR is different from other distributed systems infrastructure, so you probably have questions, like:
What is DFIR? How does it work? What is special about it, and what are the key concepts to understand?

This chapter covers those basic questions. We start simple describing DFIR, and build up to an understanding of
what makes DFIR uniquely powerful.

But in case you want a preview of the DFIR goodies, here are the main themes:
1. **Distributed Correctness**: DFIR's type system can prevent distributed system bugs at compile time. (One example: will your code
produce the same results if you deploy on a single machine or replicate on a distributed cluster of machines?)
2. **Local Efficiency**: DFIR compiles your dataflow programs into efficient, low-latency, single-threaded executables.

Taken together, DFIR provides a high-efficiency substrate on which to build powerful languages for distributed computing.

> DFIR's type system checks are not yet implemented, but they are coming soon!

## DFIR: A Runtime Library and a DSL in Rust
Setting fanfare aside, what *is* DFIR?

DFIR is a library that can be used in any Rust program. It includes two main components:

1. A runtime library that executes low-latency, reactive dataflow programs written in Rust. (The *core API*.)
2. A domain-specific language (DSL) for specifying dataflow programs. (The DFIR *surface syntax*.)

DFIR's surface syntax must be embedded in a Rust program; the Rust compiler takes that DFIR syntax and
compiles it into an efficient binary executable.

> We call a running DFIR binary a *process*.

In typical usage, a developer writes a process as a single-threaded Rust program that is mostly composed of
DFIR surface syntax. Each process is typically responsible for a single
"node" (a machine, or a core) in a distributed system composed of many such processes,
which send and receive flows of data to each other.

> DFIR itself does not generate distributed code. It is a library for specifying the processes (individual nodes) that
> participate in a distributed system.
>
> In the [Hydro Project](https://hydro.run), higher-level framework are being built on top of DFIR to generate
> distributed code in the form of multiple processes.
> Meanwhile, you can use DFIR to write your own distributed code, by writing individual processes that work together,
> and deploying them manually or with a tool like [Hydroplane](https://github.com/hydro-project/hydroplane). See the [Hydro Ecosystem](../ecosystem) for more on this.

### So how might a human write distributed systems with DFIR?
As an illustration of how you can work at the DFIR layer, consider the
[Chat Server example](https://github.com/hydro-project/hydro/tree/main/dfir_rs/examples/chat). If you run that binary
with the command-line argument `--role server` it will start a single process that is responsible for a chat server: receiving
membership requests and messages from clients, and forwarding messages from individual clients to all other clients.
If you run that binary with the argument `--role client` it will start a process that is responsible for a chat client, which
forwards chat messages from stdin to the server, and prints out messages sent by the server. As a distributed system, the chat
service would typically consist of many client processes and a single server process.

Note that this is an example of an extremely simple distributed system in a "star" or "hub-and spokes" topology: the multiple client processes are completely independent of each other, and each talks only with the central server process.

<div align="center">

```mermaid
%%{init: {'theme':'neutral'}}%%
graph TD;
    Client1---S((Server));
    Client2---S;
    Client3---S;
    S---Client4;
    S---Client5;
    S---Client6;
```
</div>

 If we wanted something more interesting, we could consider deploying a cluster of multiple server processes, say for fault tolerance or geo-distribution, perhaps like this:

 <div align="center">

```mermaid
%%{init: {'theme':'neutral'}}%%
graph LR;
    subgraph servers[Server Cluster]
        S1((Server1))---S2((Server2))---S3((Server3))---S1;
    end
    Client1---S1;
    Client2---S1;
    Client3---S2;
    S2---Client4;
    S3---Client5;
    S3---Client6;
```
</div>

 We'd need to change the server code to get servers sharing their state in a correct and reliable manner. But that's a topic for another section **TODO**; for now, let's stay focused on the basics.

