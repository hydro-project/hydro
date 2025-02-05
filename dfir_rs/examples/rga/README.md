# RGA 
This example illustrates a lattice-based tree for collaborative text editing, called a [Timestamped Insertion Tree](https://software.imdea.org/papers/2021-attiya-tcs.pdf) (Attiya et all 2016). It is not currently interactive; the `main` routine simply passes in the stream of "keystrokes" for the example. A keystroke is represented by a `(child, parent, value)` triple, where the child is a (monotonically increasing) timestamp, the parent is an older timestamp, and the value is a character from the keystroke.

It then outputs a graph in the DOT format, which can be rendered with Graphviz -- this graph shows the tree structure, the total ordering among the tree nodes, and the string that comes from that ordering.

There are multiple implementations to choose from, via the `--impl` flag:
- A `minimal` implementation is nothing more than a set of (child, parent) edges. The Hydroflow code here does essentially nothing beyond collecting edges and outputting them. No ordering is produced.
- A `datalog` implementation based on a [talk by Martin Kleppman](https://speakerdeck.com/ept/data-structures-as-queries-expressing-crdts-using-datalog). Kleppman's dataflow has been hand-compiled to Hydroflow, rule-by-rule.
- A `datalog-agg` implementation that modifies Kleppman's code for Datalog-with-aggregation, again hand-compiled to Hydroflow
- An `adjacency` implementation that builds an adjacency list in Hydroflow to reduce the number of aggregation passes. *This is the default if you do not specify the `--impl` flag.*

To run:
```
cargo run -p hydroflow --example rga -- --impl datalog
```
Optionally append `--impl <choice>` to choose an implementation among {`minimal`, `datalog`, `datalog-agg`, `adjacency`}
and append `--graph <type>` to also print a graph of the hydroflow code to stdout in one of the formats {`dot`, `mermaid`, `json`}.
