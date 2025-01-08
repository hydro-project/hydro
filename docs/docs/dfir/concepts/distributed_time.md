---
sidebar_position: 4
---

# Distributed Time
DFIR does not embed a particular notion of distributed time, but instead provides primitives for 
you to implement one of many possible distributed time models. If you're a distributed systems aficionado, you might
be interested to read this chapter to learn about how DFIR's time model compares to classical distributed time models.

## Lamport Time
Lamport's paper on [Time, Clocks, and the Ordering of Events in a Distributed System](https://lamport.azurewebsites.net/pubs/time-clocks.pdf) provides a classical definition of time in a distributed system. In that paper, each single-threaded *process* has a sequential clock (a so-called [Lamport Clock](https://en.wikipedia.org/wiki/Lamport_timestamp)). The clock in a process advances by one unit of time for each *event* that the process observes. Between events, the clock is frozen and the process does local computation.

In addition, to maintain Lamport Clocks, each process stamps its outbound messages with the current clock value, and begins each "tick" by *advancing* its local clock to the larger of 

- its current clock value and 
- the largest clock value of any message it received. 

The way that Lamport clocks jump ahead provides a desirable property: the timestamps provide a reasonable notion of what events *happen-before* other events in a distributed system.
Lamport timestamps track not only the order of events on a single node, they also ensure that the timestamps on events reflect distributed ordering. Suppose that node `source` wants to send a message to node `dest`, and node source has current clock value *T_source*. The events that precede that message on node `source` have smaller timestamps. In addition, consider an event at node `dest` that follows the receipt of that message. That event must have a timestamp greater than *T_source* by Lamport's advancing rule above. Hence all the events on node `source` that preceded the sending of the message have lower timestamps than the events on node `dest` following the receipt of the message. This is Lamport's distributed "happens-before" relation, and the Lamport clock capture that relation.

## DFIR Time
As a built-in primitive, DFIR defines time only for a single transducer, as a sequence of consecutive ticks without any gaps. 

Thus the main difference between DFIR events and Lamport events are:

1. **Batched Events**: DFIR treats the ingestion of a batch of multiple inbound events as a single tick. *TODO: is it possible to limit batch size to 1?* 
2. **Fixpoints between Events**: DFIR *requires* a fixpoint computation to complete between ticks.
3. **Consecutive Ticks**: the built-in clock primitive in DFIR always advances sequentially and cannot skip a tick like the Lamport clock does.

## Implementing Distributed Time in DFIR

Although Lamport clocks are not built into DFIR, it is straightforward to implement them in DFIR. Alternatively, one can implement more sophisticated distributed clocks like [vector clocks](https://en.wikipedia.org/wiki/Vector_clock) in DFIR instead. By leaving this decision to the user, DFIR can be used in a variety of distributed time models.

*TODO: add example of implementing Lamport clocks?*.