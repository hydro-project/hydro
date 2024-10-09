<h1 align="center">
    <img src="https://raw.githubusercontent.com/hydro-project/hydroflow/main/docs/static/img/hydroflow_100.png" width="50" height="50" alt='"hf"'>
    Hydroflow<br>
</h1>
<p align="center">
    <a href="https://crates.io/crates/hydroflow"><img src="https://img.shields.io/crates/v/hydroflow?style=flat-square&logo=rust" alt="Crates.io"></a>
    <a href="https://docs.rs/hydroflow/"><img src="https://img.shields.io/badge/docs.rs-Hydroflow-blue?style=flat-square&logo=read-the-docs&logoColor=white" alt="Docs.rs"></a>
</p>

Hydroflow is a low-latency dataflow runtime written in Rust. The goal of the [Hydro Project](https://hydro.run)
is to empower developers to harness the full potential of the cloud by making distributed programs easy to specify and automatic to scale. Hydroflow is the lowest level in the [Hydro stack](https://hydro.run/docs/hydroflow/ecosystem/),
serving as a single-node low-latency runtime with explicit networking. This allows us to support
not just data processing pipelines, but distributed protocols (e.g. Paxos) and real-world
long-running applications as well.

Take a look at the [Hydroflow Book](https://hydro.run/docs/hydroflow/).

## The Hydroflow Surface Syntax

Hydroflow comes with a custom "surface syntax"—a domain-specific language which serves as a very
simple, readable IR for specifying single-node Hydroflow programs. These programs are intended to be stitched together
by the Hydro stack to create larger autoscaling distributed systems.

Here's a simple example of the surface syntax. Check out the [Hydroflow Playground](https://hydro.run/playground)
for an interactive demo.
```rust
source_iter(0..10)
  -> map(|n| n * n)
  -> filter(|n| *n > 10)
  -> foo;

foo = map(|n| (n..=n+1))
  -> flatten()
  -> for_each(|n| println!("Howdy {}", n));
```

For more, check out the [surface syntax section of the Hydroflow book](https://hydro.run/docs/hydroflow/syntax/).

## Start with a Template Program
We provide a `cargo-generate` template for you to get started from a simple working example.

To install `cargo-generate`, run the following:
```bash,ignore
cargo install cargo-generate
```

Then run:
```bash,ignore
cargo generate gh:hydro-project/hydroflow template/hydroflow
```

`cd` into the generated folder, ensure the correct nightly version of rust is installed, and test the generated project:
```bash
#shell-command-next-line
cd <my-project>
#shell-command-next-line
rustup update
#shell-command-next-line
cargo test
```

And you will get a well-formed Hydroflow/Rust project to use as a starting point. It provides a simple Echo Server and Client, and advice
for adapting it to other uses.

### Enable IDE Support for Ligatures
Since flow edges `->` appear frequently in flows described using the Hydroflow surface syntax, enabling ligature support
in your IDE may improve your code reading experience. This has no impact on code functionality or performance.

Instructions to enable this for the `Fira Code` font:
- [VSCode](https://github.com/tonsky/FiraCode/wiki/VS-Code-Instructions)
- [IntelliJ](https://github.com/tonsky/FiraCode/wiki/IntelliJ-products-instructions)

More font options are available [here](https://github.com/tonsky/FiraCode?tab=readme-ov-file#alternatives).

## Dev Setup

See the [setup section of the book](https://hydro.run/docs/hydroflow/quickstart/setup).

### The Examples Container

The `hydroflow/examples` subdirectory of this repository includes a number of examples.
To make running these examples in the cloud easier, we've created a Docker image that contains compiled versions of those examples. The image is defined in the `Dockerfile` in the same directory as this README.

If you want to build the examples container locally, you can run
```
docker build -t hydroflow-examples .
```

This will build an image suitable for your architecture.

The `scripts/multiplatform-docker-build.sh <image name>` script will build both `arm64` and `amd64` versions of the image and push them to the image name specified. By default, this will push the image to DockerHub; if you want to push the image to another repository, you can pass an image URL as the argument to `multiplatform-docker-build.sh` instead.

Example binaries are located in `/usr/src/myapp`.
