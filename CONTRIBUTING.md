# Contributing to Hydro

Thanks for your interest in contributing to Hydro! This is an experimental, research-driven
project which can make getting started a bit tricky. This guide will explain the project structure,
code style, commit messages, testing setups, and more to help you get started.

## Repository Structure

The Hydro repo is set up as a monorepo and [Cargo workspace](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html).
Relative to the repository root:

* `dfir_rs` is the main DFIR package, containing the DFIR runtime. It re-exports the
  flow syntax macros in `dfir_macro` and `dfir_lang`. The runtime is the "scheduled
  layer" while the flow syntax compiler is the "compiled layer".
* `hydro_lang` and related (`hydro_*`) packages contain Hydro, which is a functional syntax built on
  top of `DFIR`.
* `docs` is the [Hydro.run](https://hydro.run/) website. `website_playground` contains the
  playground portion of the website, used for compiling DFIR in-browser via WASM.
* `benches` contains some microbenchmarks for DFIR and other frameworks.
* `design_docs` contains old point-in-time design docs for DFIR's architecture.

There are several subpackages included that are used by Hydro but are more general-purpose:

* `stageleft` is a framework for staged programming in Rust, used by `hydro_lang`.
* `lattices` is a abstract algebra library, originally for lattice types.
* `variadics` is a crate for emulating variadic generics using tuple lists.
* `pusherator` is a rudimentary library providing push-based iterators.
* `multiplatform_test` provides a convenience macro for specifying and initializing tests on
  various platforms.

## Rust

Hydro should build on latest stable releases of Rust. However we develop on a pinned nightly
version, which is updated every month or two. The version is in `rust-toolchain.toml` which is
automatically detected by `cargo`, so no special setup should be needed.

## Python

Some parts of the Hydro repo require a relatively recent version of Python 3, maybe 3.10 or
later. On Mac, installing directly from python.org may work if `brew install` doesn't.

## Wasm

[Node.js](https://nodejs.org/),
[`wasm-bindgen`](https://github.com/rustwasm/wasm-bindgen?tab=readme-ov-file#install-wasm-bindgen-cli),
and [`wasm-pack`](https://rustwasm.github.io/wasm-pack/installer/) are required to run Wasm tests.
```shell
cargo install wasm-bindgen-cli
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
Insta provides general snapshot testing in Rust, and we mainly use it to test the DFIR graphs
generated from the flow syntax. These snapshots are of the [Mermaid](https://mermaid.js.org/) or
[DOT](https://graphviz.org/) graph visualizations rather than the graph datastructures themselves;
see `dfir_rs/tests/snapshots`. The snapshots can be useful not just to track changes but also as
a quick reference to view the visualizations (i.e. by pasting into [mermaid.live](https://mermaid.live/)).
`trybuild` is used to test the error messages in DFIR's flow syntax; see `dfir_rs/tests/compile-fail`.

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
nightly. Sometimes these tests fail when the PR tests pass. Most often this is due to new lints
in the latest version of `clippy`. See [Setup#Rust](#rust) above.

## Releasing

See [`RELEASING.md`](https://github.com/hydro-project/hydro/blob/main/RELEASING.md).
