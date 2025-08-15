---
sidebar_position: 1
---

# Setup

This section explains how to get DFIR running, either for development or
usage, even if you are not familiar with Rust development.

## Installing Rust

First you will need to install Rust. We recommend the conventional installation
method, `rustup`, which allows you to easily manage and update Rust versions.

[**Install Rust**](https://www.rust-lang.org/tools/install)

The link in the previous line will take you to the Rust website that shows you how to
install `rustup` and the Rust package manager `cargo` (and the
internally-used `rustc` compiler). `cargo` is Rust's main development tool,
used for building, running, and testing Rust code.

The following `cargo` commands will come in handy:
* `cargo check --all-targets` - Checks the workspace for any compile-time
  errors.
* `cargo build --all-targets` - Builds all projects/tests/benchmarks/examples
  in the workspace.
* `cargo clean` - Cleans the build cache, sometimes needed if the build is
  acting up.
* `cargo test` - Runs tests in the workspace.
* `cargo run -p dfir_rs --example <example name>` - Run an example program in
  `dfir_rs/examples`.

To learn Rust see the official [Learn Rust page](https://www.rust-lang.org/learn).
Here are some good resources:
* [_The Rust Programming Language_, AKA "The Book"](https://doc.rust-lang.org/book/)
* [Learn Rust With Entirely Too Many Linked Lists](https://rust-unofficial.github.io/too-many-lists/)
  is a good way to learn Rust's ownership system and its
  implications.

In this book we will be using the DFIR template generator, which we recommend
as a starting point for your DFIR projects. For this purpose you
will need to install the `cargo-generate` tool:
```bash
#shell-command-next-line
cargo install cargo-generate
```

## VS Code Setup

We recommend using VS Code with the `rust-analyzer` extension (and NOT the
`Rust` extension).

## Setting up a DFIR Project
The easiest way to get started with DFIR is to begin with a template project.
Create a directory where you'd like to put that project, direct your terminal there and run:
```bash
#shell-command-next-line
cargo generate gh:hydro-project/dfir-template
```
You will be prompted to name your project. The `cargo generate` command will create a subdirectory
with the relevant files and folders.

`cd` into the generated folder, ensure the correct nightly version of rust is installed:
```bash
#shell-command-next-line
cd <my-project>
#shell-command-next-line
rustup update
```

As part of generating the project, the `dfir_rs` library will be downloaded as a dependency.
You can then open the project in VS Code or IDE of your choice, or
you can simply build the template project with `cargo build`.
```bash
#shell-command-next-line
cargo build
```
This should return successfully.

The template provides a simple working example of a DFIR program.
As a sort of "hello, world" of distributed systems, it implements an "echo server" that
simply echoes back the messages you sent it; it also implements a client to test the server.
We will replace the code in that example with our own, but it's a good idea to run it first to make sure everything is working.

:::note
We call a running DFIR binary a *process*.
:::

Start by running a process for the server:
```console
#shell-command-next-line
cargo run -- --role server
Listening on 127.0.0.1:<port>
Server live!
```

Take note of the server's port number, and in a separate terminal, start a client process:
```console
#shell-command-next-line
cd <project name>
#shell-command-next-line
cargo run -- --role client --address 127.0.0.1:<port>
Listening on 127.0.0.1:<client_port>
Connecting to server at 127.0.0.1:<port>
Client live!
```
Now you can type strings in the client, which are sent to the server, echo'ed back, and printed at the client. E.g.:
```console
Hello!
2023-06-01 00:19:53.906635 UTC: Got Echo { payload: "Hello!", ts: 2023-06-01T00:19:53.906123Z } from 127.0.0.1:61019
```

## Alternative: Checking out the Hydro Repository

This book will assume you are using the template project, but some
Rust experts may want to get started with DFIR by cloning and working in the
repository directly.
You should fork the repository if you want to push your
changes.

To clone the repo, run:
```bash
git clone git@github.com:hydro-project/hydro.git
```
DFIR requires nightly Rust, but the repo is already configured for it via
`rust-toolchain.toml`.

You can then open the repo in VS Code or IDE of your choice. In VS Code, `rust-analyzer`
will provide inline type and error messages, code completion, etc.

To work with the repository, it's best to start with an "example", found in the
[`dfir/examples` folder](https://github.com/hydro-project/hydro/tree/main/dfir_rs/examples).
The simplest example is the
['hello world'](https://github.com/hydro-project/hydro/blob/main/dfir_rs/examples/hello_world/main.rs) example;
the simplest example with networking is the
[`echo server`](https://github.com/hydro-project/hydro/blob/main/dfir_rs/examples/echo_server/main.rs).

The DFIR repository is set up as a [workspace](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html),
i.e. a repo containing a bunch of separate packages, `dfir_rs` is just the
main one. So if you want to work in a proper separate cargo package, you can
create one and add it into the [root `Cargo.toml`](https://github.com/hydro-project/hydro/blob/main/Cargo.toml),
much like the [provided template](https://github.com/hydro-project/hydro/tree/main/template/dfir#readme).
