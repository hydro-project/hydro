# Hydroflow

Hydro's low-level dataflow runtime.

See the [Github Pages index](https://hydro-project.github.io/hydroflow/) for more documentation.

Read the [Hydroflow Book](https://hydro-project.github.io/hydroflow/book/).

## Start with a Template Program
We provide a `cargo-generate` template for you to get started from a simple working example.

To install `cargo-generate`, run the following:
```bash, ignore
cargo install cargo-generate
```

Then run 
```bash, ignore
cargo generate gh:hydro-project/hydroflow-template
```
and you will get a well-formed Hydroflow/Rust project to use as a starting point. It provides a simple Echo Server and Client, and advice
for adapting it to other uses.


## The Examples Container

The `hydroflow/examples` subdirectory of this repository includes a number of examples.
To make running these examples in the cloud easier, we've created a Docker image that contains compiled versions of those examples. The image is defined in the `Dockerfile` in the same directory as this README.

If you want to build the examples container locally, you can run
```
docker build -t hydroflow-examples .
```

This will build an image suitable for your architecture.

The `scripts/multiplatform-docker-build.sh <image name>` script will build both `arm64` and `amd64` versions of the image and push them to the image name specified. By default, this will push the image to DockerHub; if you want to push the image to another repository, you can pass an image URL as the argument to `multiplatform-docker-build.sh` instead.

Example binaries are located in `/usr/src/myapp`.

## Dev Setup

See the [setup section of the book](https://hydro-project.github.io/hydroflow/book/setup.html).

### mdBook Setup

[The Hydroflow Book](https://hydro-project.github.io/hydroflow/book/) is generated using [mdBook](https://rust-lang.github.io/mdBook/). To install `mdbook` and dependencies:
```bash, ignore
cargo install mdbook mdbook-mermaid mdbook-linkcheck mdbook-katex
```
The book can then be viewed locally with a web browser by running the following from the project root.
```bash, ignore
mdbook serve --open
```
