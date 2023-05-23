---
sidebar_position: 3
---

# Data Sources and Sinks in Rust
Any useful flow requires us to define sources of data, either generated computationally or received from 
and outside environment via I/O.

## `source_iter`
A flow can receive data from a Rust collection object via the `source_iter` operator, which takes the 
iterable collection as an argument and passes the items down the flow. 
For example, here we iterate through a vector of `usize` items and push them down the flow:
```rust,ignore
    source_iter(vec![0, 1]) -> ...
```
The Hello, World example above uses this construct.

## `source_stream`
More commonly, a flow should handle external data coming in asynchronously from a [_Tokio_ runtime](https://tokio.rs/tokio/tutorial).
One way to do this is with _channels_ that allow Rust code to send data into the Hydroflow inputs.
The code below creates a channel for data of (Rust) type `(usize, usize)`:
```rust,ignore
    let (input_send, input_recv) = hydroflow::util::unbounded_channel::<(usize, usize)>();
```
Under the hood this uses [Tokio unbounded channels](https://docs.rs/tokio/latest/tokio/sync/mpsc/fn.unbounded_channel.html).
Now in Rust we can now push data into the channel. E.g. for testing we can do
it explicitly as follows:
```rust,ignore
    input_send.send((0, 1)).unwrap()
```
And in our Hydroflow syntax we can receive the data from the channel using the `source_stream` syntax and
pass it along a flow:
```rust,ignore
    source_stream(input_recv) -> ...
```

To put this together, let's revisit our Hello, World example from above with data sent 
in from outside the flow:
```rust
# use hydroflow::hydroflow_syntax;
let (input_send, input_recv) = hydroflow::util::unbounded_channel::<&str>();
let mut flow = hydroflow_syntax! {
    source_stream(input_recv) -> map(|x| x.to_uppercase())
        -> for_each(|x| println!("{}", x));
};
input_send.send("Hello").unwrap();
input_send.send("World").unwrap();
flow.run_available();
```

TODO: add source_stream_serde