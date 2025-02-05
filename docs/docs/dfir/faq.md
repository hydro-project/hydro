---
sidebar_position: 9
---

# FAQ
#### Q: How is DFIR different from dataflow systems like Spark, Flink, or MapReduce?
**A:** DFIR is designed as a substrate for building a wide range of distributed systems; traditional software dataflow systems
like the ones mentioned are targeted more narrowly at large-scale data processing. As such, DFIR differs from these systems in several ways:

First, DFIR is a lower-level abstraction than the systems mentioned. DFIR adopts a dataflow abstraction for specifying a process
running on a *single core*; one implementes a distributed system out of multiple DFIR processes.  By contrast, Spark, Flink and MapReduce are 
distributed systems, which make specific choices for implementing issues like scheduling, fault tolerance, and so on.

Second, the systems mentioned above were designed specifically for distributed data processing tasks. 
By contrast, DFIR is designed as a compiler target for many possible language models. Distributed
dataflow systems like those mentioned above are one possible higher-level model one could build with DFIR, 
but there are many others.

Third, DFIR is a *reactive* dataflow engine, optimized for low-latency handling of 
asynchronous events and messages in a long-lived service. For example, one can build 
low-level protocol handlers in DFIR for implementing distributed protocols
like Paxos or Two-Phase Commit with low latency.

#### Q: What model of parallelism does DFIR use? SPMD? MPMD? Actors? MPI?
**A:** As a substrate for building individual nodes in a distributed systems, DFIR does not make any 
assumptions about the model of parallelism used. One can construct a distributed system out of DFIR 
processes that use any model of parallelism, including 
[SPMD](https://en.wikipedia.org/wiki/Flynn%27s_taxonomy#Single_program,_multiple_data_streams_(SPMD)), 
[MPMD](https://en.wikipedia.org/wiki/Flynn%27s_taxonomy#Multiple_programs,_multiple_data_streams_(MPMD)), 
[Actors](https://en.wikipedia.org/wiki/Actor_model) or 
[MPI Collective Operations](https://en.wikipedia.org/wiki/Collective_operation). 
DFIR provides abstractions for implementing individual processes to handle inputs, computing, and emitting outputs in
a general-purpose manner.

That said, it is common practice to deploy many instance of the same DFIR process; most distributed systems built in DFIR
therefore have some form of SPMD parallelism. (This is true of most distributed systems in general.)