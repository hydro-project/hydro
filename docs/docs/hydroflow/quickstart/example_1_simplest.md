---
sidebar_position: 2
---

# Simplest Example

> In this example we will cover:
> - Modifying the Hydroflow template project
> - How Hydroflow program specs are embedded inside Rust 
> - How to execute a simple Hydroflow program
> - Two Hydroflow operators: `source_iter` and `for_each`

Lets start out with the simplest possible Hydroflow program, which prints out
the numbers in `0..10`.

Create a clean template project:
```console
#shell-command-next-line
cargo generate hydro-project/hydroflow-template
⚠️   Favorite `hydro-project/hydroflow-template` not found in config, using it as a git repository: https://github.com/hydro-project/hydroflow-template.git
🤷   Project Name: simple
🔧   Destination: /Users/jmh/code/sussudio/simple ...
🔧   project-name: simple ...
🔧   Generating template ...
[ 1/11]   Done: .gitignore                                                      [ 2/11]   Done: Cargo.lock                                                      [ 3/11]   Done: Cargo.toml                                                      [ 4/11]   Done: README.md                                                       [ 5/11]   Done: rust-toolchain.toml                                             [ 6/11]   Done: src/client.rs                                                   [ 7/11]   Done: src/helpers.rs                                                  [ 8/11]   Done: src/main.rs                                                     [ 9/11]   Done: src/protocol.rs                                                 [10/11]   Done: src/server.rs                                                   [11/11]   Done: src                                                             🔧   Moving generated files into: `<dir>/simple`...
💡   Initializing a fresh Git repository
✨   Done! New project created <dir>/simple
```

Change directory into the resulting `simple` folder or open it in your IDE. Then edit the `src/main.rs` file, replacing 
*all* of its contents with the following code:

```rust
use hydroflow::hydroflow_syntax;

pub fn main() {
    let mut flow = hydroflow_syntax! {
        source_iter(0..10) -> for_each(|n| println!("Hello {}", n));
    };

    flow.run_available();
}
```

And then run the program:
```console
#shell-command-next-line
cargo run
Hello 0
Hello 1
Hello 2
Hello 3
Hello 4
Hello 5
Hello 6
Hello 7
Hello 8
Hello 9
```

## Understanding the Code
Although this is a trivial program, it's useful to go through it line by line.
```rust,ignore
use hydroflow::hydroflow_syntax;
```
This import gives you everything you need from Hydroflow to write code with Hydroflow's 
[_surface syntax_](../syntax/index.md).

Next, inside the main method we specify a flow by calling the 
`hydroflow_syntax!` macro. We assign the resulting `Hydroflow` instance to
a mutable variable `flow`––mutable because we will be changing its status when we run it.
```rust,ignore
# use hydroflow::hydroflow_syntax;
pub fn main() {
    let mut flow = hydroflow_syntax! {
        source_iter(0..10) -> for_each(|n| println!("Hello {}", n));
    };
```

Hydroflow surface syntax defines a "flow" consisting of *operators* connected via `->` arrows.
This simplest example uses a simple two-step linear flow.
It starts with a [`source_iter`](../syntax/surface_ops.gen.md#source_iter) operator that takes the Rust
iterator `0..10` and iterates it to emit the 
numbers 0 through 9. That operator then passes those numbers along the `->` arrow downstream to a 
[`for_each`](../syntax/surface_ops.gen.md#for_each) operator that invokes its closure argument to print each
item passed in.

The Hydroflow surface syntax is merely a *specification*; it does not actually do anything
until we run it.
We run the flow from within Rust via the [`run_available()` method](https://hydro-project.github.io/hydroflow/doc/hydroflow/scheduled/graph/struct.Hydroflow.html#method.run_available).
```rust,ignore
    flow.run_available();
```
Note that `run_available()` runs the Hydroflow graph until no more work is immediately
available. In this example flow, running the graph drains the iterator completely, so no
more work will *ever* be available. In future examples we will use external inputs such as
network ingress, in which case more work might appear at any time. 

### A Note on Project Structure
The template project is intended to be a starting point for your own Hydroflow project, and you can add files and directories as you see fit. The only requirement is that the `src/main.rs` file exists and contains a `main()` function.

In this simplest example we did not use a number of the files in the template: notably everything in the `src/` subdirectory other than `src/main.rs`. If you'd like to delete those extraneous files you can do so, but it's not necessary, and we'll use them in subsequent examples. 