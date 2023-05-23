# Hydroflow Surface Syntax
The natural way to write a Hydroflow program is using the _Surface Syntax_ documented here. 
It is a chained `Iterator`-style syntax of operators built into Hydroflow that should be sufficient
for most uses. If you want lower-level access you can work with the `Core API` documented in the [Architecture](../architecture/index.md) section.

In this chapter we go over the syntax piece by piece: how to [embed surface syntax in Rust](./surface_embedding.md) and how to specify [_flows_](./surface_flows.md), which consist of [_data sources_](./surface_data.md) flowing through [_operators_](./surface_ops.gen.md).
<!-- TODO(mingwei): In the [Hydroflow Types](surface_types.md) chapter we dive into the details of the data types that pass through flows. -->

As a teaser, here is a Rust/Hydroflow "HELLO WORLD" program:
```rust
use hydroflow::hydroflow_syntax;

pub fn test_hello_world() {
    let mut df = hydroflow_syntax! {
        source_iter(vec!["hello", "world"])
            -> map(|x| x.to_uppercase()) -> for_each(|x| println!("{}", x));
    };
    df.run_available();
}
```
