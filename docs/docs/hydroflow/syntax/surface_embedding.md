---
sidebar_position: 1
---

# Embedding a Flow in Rust
Hydroflow's surface syntax is typically used within a Rust program. (An interactive client and/or external language bindings are TBD.)

The surface syntax is embedded into Rust via a macro as follows
```rust
use hydroflow::hydroflow_syntax;

pub fn example() {
    let mut flow = hydroflow_syntax! {
        // Hydroflow Surface Syntax goes here
    };
}
```
The resulting `flow` object is of type [`Hydroflow`](https://hydro-project.github.io/hydroflow/doc/hydroflow/scheduled/graph/struct.Hydroflow.html).
<!-- TODO(mingwei): see the documentation on the
Hydroflow Object for details on how to use the result. -->
