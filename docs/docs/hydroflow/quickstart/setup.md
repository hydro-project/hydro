---
sidebar_position: 1
---

# Setup

This section explains how to get Hydroflow running, either for development or
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
* `cargo run -p hydroflow --example <example name>` - Run an example program in
  `hydroflow/examples`.

To learn Rust see the official [Learn Rust page](https://www.rust-lang.org/learn).
Here are some good resources:
* [_The Rust Programming Language_, AKA "The Book"](https://doc.rust-lang.org/book/)
* [Learn Rust With Entirely Too Many Linked Lists](https://rust-unofficial.github.io/too-many-lists/)
  is a good way to learn Rust's ownership system and its
  implications.

In this book we will be using the Hydroflow template generator, which we recommend
as a starting point for your Hydroflow projects. For this purpose you
will need to install the `cargo-generate` tool:
```bash
cargo install cargo-generate
```

## VS Code Setup

We recommend using VS Code with the `rust-analyzer` extension (and NOT the
`Rust` extension). To enable the pre-release version of `rust-analyzer`
(required by some new nightly syntax we use, at least until 2022-03-14), click
the "Switch to Pre-Release Version" button next to the uninstall button.

## Setting up a Hydroflow Project
The easiest way to get started with Hydroflow is to begin with a template project. 
Create a directory where you'd like to put that project, direct your terminal there and run:
```bash
cargo generate hydro-project/hydroflow-template
```
You will be prompted to name your project. The `cargo generate` command will create a subdirectory 
with the relevant files and folders. 

As part of generating the project, the `hydroflow` library will be downloaded as a dependency.
You can then open the project in VS Code or IDE of your choice, or
you can simply build the template project with `cargo build`.
```bash
cd <project name>
cargo build
```
This should return successfully.

The template provides a simple working example of a Hydroflow program.
As a sort of "hello, world" of distributed systems, it implements an "echo server" that
simply echoes back the messages you sent it; it also implements a client to test the server. 
We will replace the code in that example with our own, but it's a good idea to run it first to make sure everything is working.

:::note
We call a running Hydroflow binary a *transducer*.
:::

Start by running a transducer for the server:
```console
#shell-command-next-line
cargo run -- --role server
Listening on 127.0.0.1:<port>
Server live!
```

Take note of the server's port number, and in a separate terminal, start a client transducer:
```console
#shell-command-next-line
cd <project name>
#shell-command-next-line
cargo run -- --role client --server-addr 127.0.0.1:<port>
Listening on 127.0.0.1:<client_port>
Connecting to server at 127.0.0.1:<port>
Client live!
```
Now you can type strings in the client, which are sent to the server, echo'ed back, and printed at the client. E.g.:
```console
Hello!
2022-12-20 18:51:50.181647 UTC: Got Echo { payload: "Hello!", ts: 2022-12-20T18:51:50.180874Z } from 127.0.0.1:61065
```

## Alternative: Checking out the Hydroflow Repository

This book will assume you are using the template project, but some
Rust experts may want to get started with Hydroflow by cloning and working in the
repository directly. 
You should fork the repository if you want to push your
changes.

To clone the repo, run:
```bash
git clone git@github.com:hydro-project/hydroflow.git
```
Hydroflow requires nightly Rust, but the repo is already configured for it via
`rust-toolchain.toml`.

You can then open the repo in VS Code or IDE of your choice. In VS Code, `rust-analyzer`
will provide inline type and error messages, code completion, etc.

To work with the repository, it's best to start with an "example", found in the
[`hydroflow/examples` folder](https://github.com/hydro-project/hydroflow/tree/main/hydroflow/examples).
These examples are included via the [`hydroflow/Cargo.toml` file](https://github.com/hydro-project/hydroflow/blob/main/hydroflow/Cargo.toml),
so make sure to add your example there if you create a new one. The simplest
example is the [`echo server`](https://github.com/hydro-project/hydroflow/blob/main/hydroflow/examples/echoserver/main.rs).

The Hydroflow repository is set up as a [workspace](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html),
i.e. a repo containing a bunch of separate packages, `hydroflow` is just the
main one. So if you want to work in a proper separate cargo package, you can
create one and add it into the [root `Cargo.toml`](https://github.com/hydro-project/hydroflow/blob/main/Cargo.toml),
much like the [provided template](https://github.com/hydro-project/hydroflow-template/blob/main/Cargo.toml).
