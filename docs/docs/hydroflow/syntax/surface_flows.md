---
sidebar_position: 2
---

# Flow Syntax
Flows consist of named _operators_ that are connected via flow _edges_ denoted by `->`. The example below
uses the [`source_iter`](./surface_ops.gen.md#source_iter) operator to generate two strings from a Rust `vec`, the
[`map`](./surface_ops.gen.md#map) operator to apply some Rust code to uppercase each string, and the [`for_each`](./surface_ops.gen.md#for_each)
operator to print each string to stdout.
```rust,ignore
source_iter(vec!["Hello", "world"])
    -> map(|x| x.to_uppercase()) -> for_each(|x| println!("{}", x));
```

Flows can be assigned to variable names for convenience. E.g, the above can be rewritten as follows:
```rust,ignore
source_iter(vec!["Hello", "world"]) -> upper_print;
upper_print = map(|x| x.to_uppercase()) -> for_each(|x| println!("{}", x));
```
Note that the order of the statements (lines) doesn't matter. In this example, `upper_print` is
referenced before it is assigned, and that is completely OK and better matches the flow of
data, making the program more understandable.

## Operators with Multiple Ports
Some operators have more than one input _port_ that can be referenced by `->`. For example [`merge`](./surface_ops.gen.md#merge)
merges the contents of many flows, so it can have an abitrary number of input ports. Some operators have multiple outputs, notably [`tee`](./surface_ops.gen.md#tee),
which has an arbitrary number of outputs.

In the syntax, we optionally distinguish input ports via an _indexing prefix_ number
in square brackets before the name (e.g. `[0]my_join` and `[1]my_join`). We
can distinguish output ports by an _indexing suffix_ (e.g. `my_tee[0]`).

Here is an example that tees one flow into two, handles each separately, and then merges them to print out the contents in both lowercase and uppercase:
```rust,ignore
my_tee = source_iter(vec!["Hello", "world"]) -> tee();
my_tee -> map(|x| x.to_uppercase()) -> my_merge;
my_tee -> map(|x| x.to_lowercase()) -> my_merge;
my_merge = merge() -> for_each(|x| println!("{}", x));
```
`merge()` and `tee()` treat all their input/outputs the same, so we omit the indexing.

Here is a visualization of the flow that was generated:
```mermaid
%%{init:{'theme':'base','themeVariables':{'clusterBkg':'#ddd','clusterBorder':'#888'}}}%%
flowchart TD
classDef pullClass fill:#02f,color:#fff,stroke:#000
classDef pushClass fill:#ff0,stroke:#000
linkStyle default stroke:#aaa,stroke-width:4px,color:red,font-size:1.5em;
subgraph sg_1v1 ["sg_1v1 stratum 0"]
    1v1[\"(1v1) <tt>source_iter(vec! [&quot;Hello&quot;, &quot;world&quot;])</tt>"/]:::pullClass
    2v1[/"(2v1) <tt>tee()</tt>"\]:::pushClass
    1v1--->2v1
    subgraph sg_1v1_var_my_tee ["var <tt>my_tee</tt>"]
        1v1
        2v1
    end
end
subgraph sg_2v1 ["sg_2v1 stratum 0"]
    3v1[\"(3v1) <tt>map(| x : &amp; str | x.to_uppercase())</tt>"/]:::pullClass
    4v1[\"(4v1) <tt>map(| x : &amp; str | x.to_lowercase())</tt>"/]:::pullClass
    5v1[\"(5v1) <tt>merge()</tt>"/]:::pullClass
    6v1[/"(6v1) <tt>for_each(| x | println! (&quot;{}&quot;, x))</tt>"\]:::pushClass
    3v1--0--->5v1
    4v1--1--->5v1
    5v1--->6v1
    subgraph sg_2v1_var_my_merge ["var <tt>my_merge</tt>"]
        5v1
        6v1
    end
end
2v1--0--->7v1
2v1--1--->8v1
7v1["(7v1) <tt>handoff</tt>"]:::otherClass
7v1--->3v1
8v1["(8v1) <tt>handoff</tt>"]:::otherClass
8v1--->4v1
```
Hydroflow compiled this flow into two subgraphs called _compiled components_, connected by _handoffs_. You can ignore
these details unless you are interested in low-level performance tuning; they are explained in the discussion
of [in-out trees](../architecture/in-out_trees.md).

### A note on assigning flows with multiple ports
> *TODO*: _Need to document the port numbers for variables assigned to tree- or dag-shaped flows_

## The `context` object

Closures inside surface syntax operators have access to a special `context` object which provides
access to scheduling, timing, and state APIs. The object is accessible as a shared reference
(`&Context`) via the special name `context`.
[Here is the full API documentation for `Context`](https://hydro-project.github.io/hydroflow/doc/hydroflow/scheduled/context/struct.Context.html).

```rust,ignore
source_iter([()])
    -> for_each(|()| println!("Current tick: {}, stratum: {}", context.current_tick(), context.current_stratum()));
// Current tick: 0, stratum: 0
```
