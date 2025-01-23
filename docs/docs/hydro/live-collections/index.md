# Live Collections
Traditional programs (like those in Rust) typically manipulate **collections** of data elements, such as those stored in a `Vec` or `HashMap`. These collections are **fixed** in the sense that any transformations applied to them such as `map` are immediately executed on a snapshot of the collection. This means that the output will not be updated when the input collection is modified.

In Hydro, programs instead work with **live collections** which are expected to dynamically change over time as new elements are added or removed (in response to API requests, streaming ingestion, etc). Applying a transformation like `map` to a live collection results in another live collection that will dynamically change over time.
