# HydroProjectHydro Agent Notes

## Repository Origin

This is an internal Amazon fork of the open-source [Hydro project](https://github.com/hydro-project/hydro) (`upstream` remote). The fork contains Amazon-specific changes to support Brazil builds, internal deployment targets (ECS/Docker), and `amzn-` crate prefixing. These changes are proprietary and must never be pushed to the upstream remote.

- `origin` — Amazon internal (git.amazon.com). All work goes here.
- `upstream` — Public GitHub (hydro-project/hydro). Read-only for pulling upstream changes. **Never push to upstream.**

### Syncing with Upstream

GitFarm cannot handle rebases on existing branches. To sync with upstream, we rebase our Brazil compatibility patches onto the latest upstream and create a new branch:

1. Fetch upstream: `git fetch upstream`
2. Create a new branch off upstream: `git checkout -b main-YYYY-MM-DD upstream/main`
3. Rebase our patches on top: `git rebase --onto main-YYYY-MM-DD upstream/main origin/<previous-branch>`
4. During rebase, drop any commits that have already been upstreamed to the public project
5. Keep only the Brazil compatibility patches (amzn- prefixing, Config files, cargo-brazil support, etc.)
6. Push the new branch to origin

### Upstreaming Changes

Before a rebase sync, identify commits that are not Amazon-specific and could be contributed to the public Hydro project. These should be manually upstreamed by a person via a GitHub PR to `hydro-project/hydro`. Once merged upstream, they can be dropped during the next rebase, keeping our fork minimal.

**Never upstream:** Brazil build configs, `amzn-` crate prefixes, internal deployment code, Amazon-specific infrastructure.

## Clippy

Run all three variants to match CI:

```sh
cargo clippy --all-targets -- -D warnings
cargo clippy --all-targets --no-default-features -- -D warnings
cargo clippy --all-targets --all-features -- -D warnings
```

The `amzn-dfir_rs` test `scheduled_test` has a pre-existing deprecation warning (`try_next` → `try_recv`) that is not our code.


## Git

- Never amend commits that have been pushed to a remote branch. Always check with `git fetch` first — CRUX auto-merge can push commits without explicit user action.
- Prefer creating new commits over amending.

## Snapshot Tests

The backtrace snapshot tests (`backtrace.snap`) capture exact line numbers. Any code change in `backtrace.rs` that adds or removes lines will shift these numbers. After modifying `backtrace.rs`, run `cargo insta test --review` to update the snapshots.
