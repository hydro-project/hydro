---
sidebar_position: 9
---

# FAQ
#### Q: How is Hydroflow different from dataflow systems like Spark, Flink, or MapReduce?
**A:** Hydroflow is designed as a substrate for building a wide range of distributed systems; traditional software dataflow systems
like the ones mentioned are targeted more narrowly at large-scale data processing. As such, Hydroflow differs from these systems in several ways:

First, Hydroflow is a lower-level abstraction than the systems mentioned. Hydroflow adopts a dataflow abstraction for specifying a transducer
running on a *single core*; one implementes a distributed system out of multiple Hydroflow transducers.  By contrast, Spark, Flink and MapReduce are 
distributed systems, which make specific choices for implementing issues like scheduling, fault tolerance, and so on.

Second, the systems mentioned above were designed specifically for distributed data processing tasks. 
By contrast, Hydroflow is designed as a compiler target for many possible language models. Distributed
dataflow systems like those mentioned above are one possible higher-level model one could build with Hydroflow, 
but there are many others.

Third, Hydroflow is a *reactive* dataflow engine, optimized for low-latency handling of 
asynchronous events and messages in a long-lived service. For example, one can build 
low-level protocol handlers in Hydroflow for implementing distributed protocols
like Paxos or Two-Phase Commit with low latency.

#### Q: What model of parallelism does Hydroflow use? SPMD? MPMD? Actors? MPI?
**A:** As a substrate for building individual nodes in a distributed systems, Hydroflow does not make any 
assumptions about the model of parallelism used. One can construct a distributed system out of Hydroflow 
transducers that use any model of parallelism, including 
[SPMD](https://en.wikipedia.org/wiki/Flynn%27s_taxonomy#Single_program,_multiple_data_streams_(SPMD)), 
[MPMD](https://en.wikipedia.org/wiki/Flynn%27s_taxonomy#Multiple_programs,_multiple_data_streams_(MPMD)), 
[Actors](https://en.wikipedia.org/wiki/Actor_model) or 
[MPI Collective Operations](https://en.wikipedia.org/wiki/Collective_operation). 
Hydroflow provides abstractions for implementing individual transducers to handle inputs, computing, and emitting outputs in
a general-purpose manner.

That said, it is common practice to deploy many instance of the same Hydroflow transducer; most distributed systems built in Hydroflow
therefore have some form of SPMD parallelism. (This is true of most distributed systems in general.)