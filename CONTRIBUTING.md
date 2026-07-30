# Contributing to Hydro

Thanks for your interest in contributing to Hydro! This is an experimental, research-driven
project which can make getting started a bit tricky. This guide will explain the project structure,
code style, commit messages, testing setups, and more to help you get started.

## Repository Structure

The Hydro repo is set up as a monorepo and [Cargo workspace](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html).
Relative to the repository root:

* `hydro_lang` and related (`hydro_*`) packages contain Hydro, which is a functional syntax built on
  top of `DFIR`. Hydro tracks runtime properties and handles distribution across multiple locations.
* `dfir_rs` is the main DFIR package, containing the DFIR runtime APIs and utilities. It re-exports the DFIR surface
  syntax macros from `dfir_macro` and `dfir_lang`. Runtime APIs are the outer "runtime layer" while the surface syntax
  is compiled into the "compiled layer". The compiled layer uses the `dfir_pipes` iterator/pusherator framework.
* `docs` is the [Hydro.run](https://hydro.run/) website. `website_playground` contains the
  playground portion of the website, used for compiling DFIR in-browser via WASM.
* `benches` contains microbenchmarks for DFIR and other frameworks.

There are several related packages/folders included that are used by Hydro but are more general-purpose:

* `design_docs` contains old point-in-time design docs for DFIR's architecture.
* [`stageleft`](https://github.com/hydro-project/stageleft/) is a framework for staged programming in Rust, used by `hydro_lang`.
* `variadics` is a crate for emulating variadic generics using tuple lists.
* `lattices` is an abstract algebra library, originally for lattice types.
* `multiplatform_test` provides a convenience macro for specifying and initializing tests on
  various platforms.

## Rust version

Hydro should build on latest stable releases of Rust. However we develop on a pinned stable
version, which is usually updated regularly. The version is in `rust-toolchain.toml` which is
automatically detected by `cargo`, so no special setup should be needed.

## `cargo-nextest`

We use [cargo-nextest](https://nexte.st) to run tests. To install:
```shell
cargo install cargo-nextest --locked
```
Or [install a prebuilt binary](https://nexte.st/docs/installation/pre-built-binaries/).


## Wasm

[Node.js](https://nodejs.org/),
[`wasm-bindgen`](https://wasm-bindgen.github.io/wasm-bindgen/wasm-bindgen-test/usage.html#install-the-test-runner),
and [`wasm-pack`](https://rustwasm.github.io/wasm-pack/installer/) are required to run Wasm tests.
```shell
cargo install wasm-bindgen-cli --vers "X.Y.Z" # Should match version of "wasm-bindgen" in Cargo.lock
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
```

## Submitting Changes

### Feature Branches

Prototypes should be committed to feature branches, rather than main. To create a feature branch:

```shell
git fetch origin
git checkout -b feature/$FEATURE_NAME origin/main
git push origin HEAD
```

To add changes on top of feature branches:
```shell
git checkout -b $BRANCH_NAME `feature/$FEATURE_NAME`
.. make changes ..
git add ... # Add all changes
git commit # Commit changes
git push origin HEAD
```

### Commit Messages

Pull request title and body should follow [Conventional Commits specification](https://www.conventionalcommits.org/).
The repository defaults to Squash+Merge commits, so individual commits are only useful for showing code evolution
during code-reviews.

Pull request title and body are used to generate changelogs. See [Releasing](#releasing) for more.

### Pull Requests and `precheck.bash`

CI runs a comprehensive set of tests on PRs before they are merged. This includes format and lint
checks. To run some checks locally, you can run `./precheck.bash` with various flags for different
parts of the code (see `./precheck.bash --help` for info). Note that this will overwrite any
changed snapshot tests instead of failing-- you should double-check that the snapshot diff matches
what you expect.

## Snapshot Testing

Hydro uses two types of snapshot testing: [`insta`](https://insta.rs/) and [`trybuild`](https://github.com/dtolnay/trybuild).
Insta provides general snapshot testing in Rust, and we mainly use it to test the generated graph structures in both
Hydro and DFIR. In Hydro, these are of the graph AST. In DFIR, these are of the [Mermaid](https://mermaid.js.org/) or
[DOT](https://graphviz.org/) graph visualizations rather than the graph datastructure itself. The snapshots can be
useful not just to track changes but also as a quick reference to view the visualizations (i.e. by pasting into
[mermaid.live](https://mermaid.live/)). `trybuild` is used to test the error messages in both Hydro and DFIR's surface
syntax.

`insta` provides a CLI, `cargo insta` to run tests and review changes:
```shell
cargo install cargo-insta
cargo insta test # or cargo test --all-targets --no-fail-fast
cargo insta review
```
Environmental variables [`INSTA_FORCE_PASS=1` and `INSTA_UPDATE=always`](https://insta.rs/docs/advanced/#disabling-assertion-failure)
can be used instead, to update `insta` snapshots. `TRYBUILD=overwrite` can be used to update
`trybuild` snapshots. `precheck.bash` uses these, and they are also set when running code with
`rust-analyzer`(see `.vscode/settings.json`).

## CI Testing

The CI runs the same the tests that are done on PRs, but also runs some tests on the latest
nightly. Sometimes these tests fail when the PR tests pass. This is often due to new lints
in the latest version of `clippy`. This is also often due to snapshot/trybuild changes, but
the CI should automatically create a PR to update the snapshots. See [Rust version](#rust-version) above.

## Releasing

See [`RELEASING.md`](https://github.com/hydro-project/hydro/blob/main/RELEASING.md).
