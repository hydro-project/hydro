# Agent Guidelines

## Commit Messages

Use [Conventional Commits](https://www.conventionalcommits.org/) format:

```
<type>[optional scope]: <short summary>

<markdown body>
```

Types: `feat`, `fix`, `refactor`, `docs`, `test`, `chore`, `ci`, `perf`.

For breaking changes, append `!` after the type (e.g. `refactor(hydro_lang)!:`), and add `BREAKING CHANGE: ...` footers
describing each breaking change (or summarizing if many).

## Descriptions

Write the description in **Markdown**. Use bullet lists, code spans, and headings to organize multi-topic changes. Example:

```
feat(hydro_lang): ...

... detailed explanation of key design decisions
```

## Building and Testing

Set `__CARGO_DEFAULT_LIB_METADATA=1` in the environment for every `cargo` invocation
(`check`, `clippy`, `test`, …). It makes cargo reuse compiled dependency artifacts across
invocations that differ only in metadata, which turns multi-minute rebuilds of the
workspace's dependencies into seconds:

```
__CARGO_DEFAULT_LIB_METADATA=1 cargo test -p hydro_test --lib
```

Format with the **nightly** toolchain — `rustfmt.toml` uses unstable options
(`imports_granularity`, `group_imports`, `hex_literal_case`) that stable `cargo fmt` silently
skips, so a stable run leaves the tree mis-formatted (and reflows unrelated lines):

```
cargo +nightly fmt --all
cargo +nightly fmt --all -- --check
```
