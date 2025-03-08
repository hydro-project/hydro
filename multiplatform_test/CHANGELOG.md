# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.5.0 (2025-03-08)

### Chore

 - <csr-id-49a387d4a21f0763df8ec94de73fb953c9cd333a/> upgrade to Rust 2024 edition
   - Updates `Cargo.toml` to use new shared workspace keys
   - Updates lint settings (in workspace `Cargo.toml`)
   - `rustfmt` has changed slightly, resulting in a big diff - there are no
   actual code changes
   - Adds a script to `rustfmt` the template src files

### Style

 - <csr-id-8b3b60812d9f561cb7f59120993fbf2e23191e2b/> fix all unexpected cfgs
   Testing in https://github.com/MingweiSamuel/hydroflow

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release over the course of 52 calendar days.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#1648](https://github.com/hydro-project/hydro/issues/1648), [#1747](https://github.com/hydro-project/hydro/issues/1747)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1648](https://github.com/hydro-project/hydro/issues/1648)**
    - Fix all unexpected cfgs ([`8b3b608`](https://github.com/hydro-project/hydro/commit/8b3b60812d9f561cb7f59120993fbf2e23191e2b))
 * **[#1747](https://github.com/hydro-project/hydro/issues/1747)**
    - Upgrade to Rust 2024 edition ([`49a387d`](https://github.com/hydro-project/hydro/commit/49a387d4a21f0763df8ec94de73fb953c9cd333a))
</details>

## 0.4.0 (2024-12-23)

<csr-id-3291c07b37c9f9031837a2a32953e8f8854ec298/>

### Chore

 - <csr-id-3291c07b37c9f9031837a2a32953e8f8854ec298/> Rename Hydroflow -> DFIR
   Work In Progress:
   - [x] hydroflow_macro
   - [x] hydroflow_datalog_core
   - [x] hydroflow_datalog
   - [x] hydroflow_lang
   - [x] hydroflow

### Documentation

 - <csr-id-28cd220c68e3660d9ebade113949a2346720cd04/> add `repository` field to `Cargo.toml`s, fix #1452
   #1452 
   
   Will trigger new releases of the following:
   `unchanged = 'hydroflow_deploy_integration', 'variadics',
   'variadics_macro', 'pusherator'`
   
   (All other crates already have changes, so would be released anyway)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 45 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#1501](https://github.com/hydro-project/hydro/issues/1501), [#1620](https://github.com/hydro-project/hydro/issues/1620)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1501](https://github.com/hydro-project/hydro/issues/1501)**
    - Add `repository` field to `Cargo.toml`s, fix #1452 ([`28cd220`](https://github.com/hydro-project/hydro/commit/28cd220c68e3660d9ebade113949a2346720cd04))
 * **[#1620](https://github.com/hydro-project/hydro/issues/1620)**
    - Rename Hydroflow -> DFIR ([`3291c07`](https://github.com/hydro-project/hydro/commit/3291c07b37c9f9031837a2a32953e8f8854ec298))
 * **Uncategorized**
    - Release dfir_lang v0.11.0, dfir_datalog_core v0.11.0, dfir_datalog v0.11.0, dfir_macro v0.11.0, hydroflow_deploy_integration v0.11.0, lattices_macro v0.5.8, variadics v0.0.8, variadics_macro v0.5.6, lattices v0.5.9, multiplatform_test v0.4.0, pusherator v0.0.10, dfir_rs v0.11.0, hydro_deploy v0.11.0, stageleft_macro v0.5.0, stageleft v0.6.0, stageleft_tool v0.5.0, hydro_lang v0.11.0, hydro_std v0.11.0, hydro_cli v0.11.0, safety bump 6 crates ([`9a7e486`](https://github.com/hydro-project/hydro/commit/9a7e48693fce0face0f8ad16349258cdbe26395f))
</details>

## 0.3.0 (2024-11-08)

<csr-id-d5677604e93c07a5392f4229af94a0b736eca382/>

### Chore

 - <csr-id-d5677604e93c07a5392f4229af94a0b736eca382/> update pinned rust version, clippy lints, remove some dead code

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 69 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1444](https://github.com/hydro-project/hydro/issues/1444)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1444](https://github.com/hydro-project/hydro/issues/1444)**
    - Update pinned rust version, clippy lints, remove some dead code ([`d567760`](https://github.com/hydro-project/hydro/commit/d5677604e93c07a5392f4229af94a0b736eca382))
 * **Uncategorized**
    - Release hydroflow_lang v0.10.0, hydroflow_datalog_core v0.10.0, hydroflow_datalog v0.10.0, hydroflow_deploy_integration v0.10.0, hydroflow_macro v0.10.0, lattices_macro v0.5.7, variadics v0.0.7, variadics_macro v0.5.5, lattices v0.5.8, multiplatform_test v0.3.0, pusherator v0.0.9, hydroflow v0.10.0, hydro_deploy v0.10.0, stageleft_macro v0.4.0, stageleft v0.5.0, stageleft_tool v0.4.0, hydroflow_plus v0.10.0, hydro_cli v0.10.0, safety bump 8 crates ([`dcd48fc`](https://github.com/hydro-project/hydro/commit/dcd48fc7ee805898d9b5ef0d082870e30615e95b))
</details>

## 0.2.0 (2024-08-30)

<csr-id-11af32828bab6e4a4264d2635ff71a12bb0bb778/>

### Chore

 - <csr-id-11af32828bab6e4a4264d2635ff71a12bb0bb778/> lower min dependency versions where possible, update `Cargo.lock`
   Moved from #1418
   
   ---------

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 97 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1423](https://github.com/hydro-project/hydro/issues/1423)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1423](https://github.com/hydro-project/hydro/issues/1423)**
    - Lower min dependency versions where possible, update `Cargo.lock` ([`11af328`](https://github.com/hydro-project/hydro/commit/11af32828bab6e4a4264d2635ff71a12bb0bb778))
 * **Uncategorized**
    - Release hydroflow_lang v0.9.0, hydroflow_datalog_core v0.9.0, hydroflow_datalog v0.9.0, hydroflow_deploy_integration v0.9.0, hydroflow_macro v0.9.0, lattices_macro v0.5.6, lattices v0.5.7, multiplatform_test v0.2.0, variadics v0.0.6, pusherator v0.0.8, hydroflow v0.9.0, stageleft_macro v0.3.0, stageleft v0.4.0, stageleft_tool v0.3.0, hydroflow_plus v0.9.0, hydro_deploy v0.9.0, hydro_cli v0.9.0, hydroflow_plus_deploy v0.9.0, safety bump 8 crates ([`0750117`](https://github.com/hydro-project/hydro/commit/0750117de7088c01a439b102adeb4c832889f171))
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
 - 6 unique issues were worked on: [#1161](https://github.com/hydro-project/hydro/issues/1161), [#609](https://github.com/hydro-project/hydro/issues/609), [#617](https://github.com/hydro-project/hydro/issues/617), [#755](https://github.com/hydro-project/hydro/issues/755), [#801](https://github.com/hydro-project/hydro/issues/801), [#822](https://github.com/hydro-project/hydro/issues/822)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1161](https://github.com/hydro-project/hydro/issues/1161)**
    - Fix clippy warning on latest nightly, fix docs ([`720c8a0`](https://github.com/hydro-project/hydro/commit/720c8a0095fc7593366b3f6c59365b4f6c245a9d))
 * **[#609](https://github.com/hydro-project/hydro/issues/609)**
    - Update syn to 2.0 ([`2e7d802`](https://github.com/hydro-project/hydro/commit/2e7d8024f35893ef0abcb6851e370b00615f9562))
 * **[#617](https://github.com/hydro-project/hydro/issues/617)**
    - Update `Cargo.toml`s for publishing ([`a78ff9a`](https://github.com/hydro-project/hydro/commit/a78ff9aace6771787c2b72aad83be6ad8d49a828))
 * **[#755](https://github.com/hydro-project/hydro/issues/755)**
    - `hydroflow`, `logging`/`tracing` features ([`c1b0280`](https://github.com/hydro-project/hydro/commit/c1b028089ea9d76ab71cd9cb4eaaaf16aa4b65a6))
 * **[#801](https://github.com/hydro-project/hydro/issues/801)**
    - Update proc-macro2, use new span location API where possible ([`8d3494b`](https://github.com/hydro-project/hydro/commit/8d3494b5afee858114a602a3e23077bb6d24dd77))
 * **[#822](https://github.com/hydro-project/hydro/issues/822)**
    - Fix lint, format errors for latest nightly version (without updated pinned) ([`f60053f`](https://github.com/hydro-project/hydro/commit/f60053f70da3071c54de4a0eabb059a143aa2ccc))
 * **Uncategorized**
    - Release hydroflow_lang v0.7.0, hydroflow_datalog_core v0.7.0, hydroflow_datalog v0.7.0, hydroflow_macro v0.7.0, lattices v0.5.5, multiplatform_test v0.1.0, pusherator v0.0.6, hydroflow v0.7.0, stageleft_macro v0.2.0, stageleft v0.3.0, stageleft_tool v0.2.0, hydroflow_plus v0.7.0, hydro_deploy v0.7.0, hydro_cli v0.7.0, hydroflow_plus_cli_integration v0.7.0, safety bump 8 crates ([`2852147`](https://github.com/hydro-project/hydro/commit/285214740627685e911781793e05d234ab2ad2bd))
    - Fix imports ([`b391447`](https://github.com/hydro-project/hydro/commit/b391447ec13f1f79c99142f296dc2fa8640034f4))
    - Bump proc-macro2 min version to 1.0.63 ([`f19eccc`](https://github.com/hydro-project/hydro/commit/f19eccc79d6d7c88de7ba1ef6a0abf1caaef377f))
    - Setup release workflow ([`108d0e9`](https://github.com/hydro-project/hydro/commit/108d0e933a08b183c4dadf8c3499e4946696e263))
    - Turn on WASM tests ([`d29bc63`](https://github.com/hydro-project/hydro/commit/d29bc6382d0d4d97931af10d90161552878903c7))
</details>

## 0.0.0 (2023-04-25)

