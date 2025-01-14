# Core Concepts
Hydro is distinguished from many other approaches to distributed programming by two key features:

1. A *global* programming model, in which you can write straight-line code that spans multiple processes or machines, even within a single function. There are multiple benefits to this approach, but perhaps the most important is this:
    - In Hydro, code can be *logically modular*, rather than *physically modular*. That is, Hydro code designed to fulfill a logical purpose can be in a single file or even a single function, regardless of how many processes or messages it spans. By contrast, most older approaches (e.g. actors, coroutines, RPCs) encourage programmers to factor their programs so that the code for two separate processes is in two separate functions or files.
        > Example: Consider a simple "heartbeat" protocol for establishing availability of processes. Each *participant* process sends a message (a "heartbeat") to some *coordinator* process at regular intervals. The coordinator invokes a failure handler if it does not receive a heartbeat from a participant in a timely fashion. In Hydro this can be written in a single function of a few lines of code, even though it involves multiple processes. In an older, physically modular approach, code for the coordinator process would typically be in one file, and code for the participants would be in another file.
        
    In research circles this is sometimes called *choreographic* programming.

2. A *stream*-centric dataflow model, in which the arrival of messages over time is treated holistically as a stream of data, rather than a series of individual "events", as is traditional in many older approaches. Being stream-centric encourages developers to think about a service's behavior over time, rather than one message at a time. 

    Dataflow programming also allows both developers and the Hydro compiler to reason naturally and in detail about *dataflow dependencies*: how multiple different streams of events and data are interrelated in the program logic. Rich dataflow dependencies help ensure correctness, enable optimizations, and allow data lineage (a.k.a. provenance) to be tracked across processes. Many of Hydro's correctness guarantees derive from its stream type system, as we discuss next.

## Locations (in Space and Time)
Distributed systems are, by definition, decoupled across "space", in the sense of being split across separate sequential processes. But distributed systems are also decoupled across "time", in the sense that one process can never track the exact progress of another process, they can only send messages to each other with some non-zero delay. These concepts of Space and Time are fundamental to any distributed programming model.

Sometimes we need to track both space and time.
In the physical world, two items meet (or collide) if they are in the same place at the same time. If you have ever caught a ball, you know this requires some finesse. Similarly, in distributed systems, we can only compute on two items (events, data structures) if they are in the same process (place) at the same time. Correct, predictable distributed systems code needs to handle these issues with finesse as well.

To capture these issues, Hydro's type system has a notion of a `Location`, which is a unique identifier for an item in space and time. That sounds a bit cosmic, so let's take it one piece at a time.

- **Space**: In Hydro, a `Location` can either be associated with a `Process` or a `Cluster`. A Hydro `Process` is very much what you would expect: a single uniquely-identified process (a thread of control with private state). A Hydro `Cluster` is a uniquely-identified set of processes, each running the same code, but each with its own thread of control, private state and identity within the cluster. We will often use a `Cluster` for replicating a process, or for partitioning ("sharding") data across many identical processes.

- **Time**: In distributed systems, the basic local, logical unit of "time" is captured via a sequentially incrementing "clock" value in a single sequential process. Hydro implements this in a small-batch fashion.
In Hydro, each process runs an event loop in which it dequeues a prefix ("batch") of data from the heads of its various input streams, runs that data through the local Hydro code, generates whatever output data the program specifies, and then starts over. Each iteration of that loop is called a `Tick`, and each Hydro process increases the local value of its `Tick` "clock" on each iteration. A stream value that has an associated `Tick` carries the value of the local "clock" when it was generated. A stream value with `NoTick` carries no information about the time of processing. 

> Example: Consider a Hydro program `times_two` that takes an input stream of integers, and uses a `map` operator to produce an output stream with each input integer value doubled. This "stateless" program never needs to combine two items, and hence never needs to reason about time.  The `times_two` program is well-defined even with `NoTick`-based locations. By contrast, consider a program `rendezvous`


## Streams and their Variants
Streams are the fundamental data structure in Hydro, and they come in three core variants:
1. A Hydro `Stream<T, L>` is an ordered series of data items of type `T` at a location `L`. Note that we make no distinction between "events" and "data": `T` could be a network message type, or it could be a type of local data that is being processed in order.
2. A Hydro `Singleton<T>` is a single value of type `T` that may mutate over time. The values of a `Singleton` over time can be treated as a stream, but at any given time a `Singleton` has only one value.
3. A Hydro `Optional<T>` is 