Extra [`Sink`] adaptors and functions.

## Forward building with [`SinkBuild`]

For an intuitive API that matches the data flow direction, use [`SinkBuilder`] and the [`SinkBuild`] trait to chain
adaptors in forward order:

```rust
use sinktools::{SinkBuilder, SinkBuild};
use sinktools::sink::SinkExt; // `futures::sink::SinkExt` for `.send(_).await`

# #[tokio::main(flavor = "current_thread")]
# async fn main() {
// Forward chain: flatten -> filter_map -> map -> filter -> for_each
let mut pipeline = SinkBuilder::<Vec<i32>>::new()
    .flatten::<Vec<i32>>()          // Flatten input vectors
    .filter_map(|x: i32| {          // Double evens, filter odds
        if x % 2 == 0 {
            Some(x * 2)
        } else {
            None
        }
    })
    .map(|x| x + 1)                 // Add 1
    .filter(|x| *x < 100)           // Only values < 100
    .for_each(|x: i32| {            // Terminal operation
        println!("Received: {}", x);
    });

// Send nested data
pipeline.send(vec![1, 2, 3, 4]).await.unwrap();
pipeline.send(vec![5, 6]).await.unwrap();
pipeline.send(vec![]).await.unwrap();
pipeline.send(vec![7, 8, 9]).await.unwrap();
# }
```

## Direct construction

Alternatively, you can construct sink adaptors directly using their `new` methods:

```rust
use sinktools::{for_each, filter, map, filter_map, flatten};
use sinktools::sink::SinkExt; // for `.send(_).await`

# #[tokio::main(flavor = "current_thread")]
# async fn main() {
// Build the same chain from inside out: sink <- filter <- map <- filter_map <- flatten
let sink = for_each(|x: i32| {
    println!("Received: {}", x);
});
let filter_sink = filter(|x: &i32| *x < 100, sink);
let map_sink = map(|x: i32| x + 1, filter_sink);
let filter_map_sink = filter_map(|x: i32| {
    if x % 2 == 0 {
        Some(x * 2)
    } else {
        None
    }
}, map_sink);
let mut complex_sink = flatten::<Vec<i32>, _>(filter_map_sink);

// Send nested data
complex_sink.send(vec![1, 2, 3, 4]).await.unwrap();
complex_sink.send(vec![5, 6]).await.unwrap();
complex_sink.send(vec![]).await.unwrap();
complex_sink.send(vec![7, 8, 9]).await.unwrap();
# }
```

Each adaptor also provides a `new_sink` method which ensures the construction is correct for `Sink` to be implemented,
but may require additional type arguments to aid inference.

The forward `SinkBuilder` API is more intuitive for direct users, as it matches the data flow direction. Direct construction is
better for generated code as it aids compiler type inference.
