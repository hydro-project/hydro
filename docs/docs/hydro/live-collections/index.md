# Live Collections
Traditional programs (like those in Rust) typically manipulate **collections** of data elements, such as those stored in a `Vec` or `HashMap`. These collections are **fixed** in the sense that any transformations applied to them such as `map` are immediately executed on a snapshot of the collection. This means that the output will not be updated when the input collection is modified.

In Hydro, programs instead work with **live collections** which are expected to dynamically change over time as new elements are added or removed (in response to API requests, streaming ingestion, etc). Applying a transformation like `map` to a live collection results in another live collection that will dynamically change over time. All network inputs and outputs in Hydro are handled via live collections, so the majority of application logic written with Hydro will involve manipulating live collections.

Hydro offers several types of live collections that capture various asynchronous semantics:
- **[Streams](./streams.md)** capture the asynchronous arrival of messages over time, and are the main collection type in Hydro
- **[Singletons / Optionals](./singletons-optionals.md)** represent a blob of asynchronously mutated data, and can be used to create local state
- **[Keyed Streams](./keyed-streams.mdx)** capture grouped streams with independent ordering for each group, such as for messages received from a cluster
