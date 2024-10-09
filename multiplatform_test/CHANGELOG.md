# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.2.0 (2024-08-30)

### Chore

 - <csr-id-11af32828bab6e4a4264d2635ff71a12bb0bb778/> lower min dependency versions where possible, update `Cargo.lock`
   Moved from #1418
   
   ---------

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 1 commit contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1423](https://github.com/hydro-project/hydroflow/issues/1423)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1423](https://github.com/hydro-project/hydroflow/issues/1423)**
    - Lower min dependency versions where possible, update `Cargo.lock` ([`11af328`](https://github.com/hydro-project/hydroflow/commit/11af32828bab6e4a4264d2635ff71a12bb0bb778))
</details>

## 0.1.0 (2024-05-24)

<csr-id-720c8a0095fc7593366b3f6c59365b4f6c245a9d/>
<csr-id-f19eccc79d6d7c88de7ba1ef6a0abf1caaef377f/>
<csr-id-f60053f70da3071c54de4a0eabb059a143aa2ccc/>
<csr-id-b391447ec13f1f79c99142f296dc2fa8640034f4/>

### Chore

 - <csr-id-720c8a0095fc7593366b3f6c59365b4f6c245a9d/> fix clippy warning on latest nightly, fix docs
 - <csr-id-f19eccc79d6d7c88de7ba1ef6a0abf1caaef377f/> bump proc-macro2 min version to 1.0.63
 - <csr-id-f60053f70da3071c54de4a0eabb059a143aa2ccc/> fix lint, format errors for latest nightly version (without updated pinned)
   For nightly version (d9c13cd45 2023-07-05)

### Bug Fixes

 - <csr-id-8d3494b5afee858114a602a3e23077bb6d24dd77/> update proc-macro2, use new span location API where possible
   requires latest* rust nightly version
   
   *latest = 2023-06-28 or something

### Style

 - <csr-id-b391447ec13f1f79c99142f296dc2fa8640034f4/> fix imports

### New Features (BREAKING)

 - <csr-id-c1b028089ea9d76ab71cd9cb4eaaaf16aa4b65a6/> `hydroflow`, `logging`/`tracing` features
   * Adds `tokio` for `#[tokio::test]`.
* Adds `async_std` for `#[async_std::test]`.
* Adds `hydroflow` for `#[hydroflow::test]`.
* Adds `env_logging` for `env_logger` registering.
* Adds `env_tracing` for `EnvFilter` `FmtSubscriber` `tracing`.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 11 commits contributed to the release.
 - 6 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 6 unique issues were worked on: [#1161](https://github.com/hydro-project/hydroflow/issues/1161), [#609](https://github.com/hydro-project/hydroflow/issues/609), [#617](https://github.com/hydro-project/hydroflow/issues/617), [#755](https://github.com/hydro-project/hydroflow/issues/755), [#801](https://github.com/hydro-project/hydroflow/issues/801), [#822](https://github.com/hydro-project/hydroflow/issues/822)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1161](https://github.com/hydro-project/hydroflow/issues/1161)**
    - Fix clippy warning on latest nightly, fix docs ([`720c8a0`](https://github.com/hydro-project/hydroflow/commit/720c8a0095fc7593366b3f6c59365b4f6c245a9d))
 * **[#609](https://github.com/hydro-project/hydroflow/issues/609)**
    - Update syn to 2.0 ([`2e7d802`](https://github.com/hydro-project/hydroflow/commit/2e7d8024f35893ef0abcb6851e370b00615f9562))
 * **[#617](https://github.com/hydro-project/hydroflow/issues/617)**
    - Update `Cargo.toml`s for publishing ([`a78ff9a`](https://github.com/hydro-project/hydroflow/commit/a78ff9aace6771787c2b72aad83be6ad8d49a828))
 * **[#755](https://github.com/hydro-project/hydroflow/issues/755)**
    - `hydroflow`, `logging`/`tracing` features ([`c1b0280`](https://github.com/hydro-project/hydroflow/commit/c1b028089ea9d76ab71cd9cb4eaaaf16aa4b65a6))
 * **[#801](https://github.com/hydro-project/hydroflow/issues/801)**
    - Update proc-macro2, use new span location API where possible ([`8d3494b`](https://github.com/hydro-project/hydroflow/commit/8d3494b5afee858114a602a3e23077bb6d24dd77))
 * **[#822](https://github.com/hydro-project/hydroflow/issues/822)**
    - Fix lint, format errors for latest nightly version (without updated pinned) ([`f60053f`](https://github.com/hydro-project/hydroflow/commit/f60053f70da3071c54de4a0eabb059a143aa2ccc))
 * **Uncategorized**
    - Release hydroflow_lang v0.7.0, hydroflow_datalog_core v0.7.0, hydroflow_datalog v0.7.0, hydroflow_macro v0.7.0, lattices v0.5.5, multiplatform_test v0.1.0, pusherator v0.0.6, hydroflow v0.7.0, stageleft_macro v0.2.0, stageleft v0.3.0, stageleft_tool v0.2.0, hydroflow_plus v0.7.0, hydro_deploy v0.7.0, hydro_cli v0.7.0, hydroflow_plus_cli_integration v0.7.0, safety bump 8 crates ([`2852147`](https://github.com/hydro-project/hydroflow/commit/285214740627685e911781793e05d234ab2ad2bd))
    - Fix imports ([`b391447`](https://github.com/hydro-project/hydroflow/commit/b391447ec13f1f79c99142f296dc2fa8640034f4))
    - Bump proc-macro2 min version to 1.0.63 ([`f19eccc`](https://github.com/hydro-project/hydroflow/commit/f19eccc79d6d7c88de7ba1ef6a0abf1caaef377f))
    - Setup release workflow ([`108d0e9`](https://github.com/hydro-project/hydroflow/commit/108d0e933a08b183c4dadf8c3499e4946696e263))
    - Turn on WASM tests ([`d29bc63`](https://github.com/hydro-project/hydroflow/commit/d29bc6382d0d4d97931af10d90161552878903c7))
</details>

## 0.0.0 (2023-04-25)

