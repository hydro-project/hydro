# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.6.0 (2025-11-25)

### Chore

 - <csr-id-e5d90803ae16993ac3db24a7795d0864abc4ac52/> update wasm

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 1 commit contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#2087](https://github.com/hydro-project/hydro/issues/2087)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#2087](https://github.com/hydro-project/hydro/issues/2087)**
    - Update wasm ([`e5d9080`](https://github.com/hydro-project/hydro/commit/e5d90803ae16993ac3db24a7795d0864abc4ac52))
</details>

## 0.5.0 (2025-03-08)

<csr-id-49a387d4a21f0763df8ec94de73fb953c9cd333a/>
<csr-id-8b3b60812d9f561cb7f59120993fbf2e23191e2b/>

### Chore

 - <csr-id-49a387d4a21f0763df8ec94de73fb953c9cd333a/> upgrade to Rust 2024 edition
   - Updates `Cargo.toml` to use new shared workspace keys
   - Updates lint settings (in workspace `Cargo.toml`)
   - `rustfmt` has changed slightly, resulting in a big diff - there are no
   actual code changes
   - Adds a script to `rustfmt` the template src files

### Style

 - <csr-id-3f76e91766a0bd9e61f11f9013d76f688467fb5e/> fix all unexpected cfgs
   Testing in https://github.com/MingweiSamuel/hydroflow

### Chore

 - <csr-id-ec3795a678d261a38085405b6e9bfea943dafefb/> upgrade to Rust 2024 edition
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

 - 3 commits contributed to the release.
 - 74 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#1648](https://github.com/hydro-project/hydro/issues/1648), [#1747](https://github.com/hydro-project/hydro/issues/1747)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1648](https://github.com/hydro-project/hydro/issues/1648)**
    - Fix all unexpected cfgs ([`3f76e91`](https://github.com/hydro-project/hydro/commit/3f76e91766a0bd9e61f11f9013d76f688467fb5e))
 * **[#1747](https://github.com/hydro-project/hydro/issues/1747)**
    - Upgrade to Rust 2024 edition ([`ec3795a`](https://github.com/hydro-project/hydro/commit/ec3795a678d261a38085405b6e9bfea943dafefb))
 * **Uncategorized**
    - Release dfir_lang v0.12.0, dfir_datalog_core v0.12.0, dfir_datalog v0.12.0, dfir_macro v0.12.0, hydroflow_deploy_integration v0.12.0, lattices_macro v0.5.9, variadics v0.0.9, variadics_macro v0.6.0, lattices v0.6.0, multiplatform_test v0.5.0, pusherator v0.0.11, dfir_rs v0.12.0, hydro_deploy v0.12.0, stageleft_macro v0.6.0, stageleft v0.7.0, stageleft_tool v0.6.0, hydro_lang v0.12.0, hydro_std v0.12.0, hydro_cli v0.12.0, safety bump 10 crates ([`973c925`](https://github.com/hydro-project/hydro/commit/973c925e87ed78344494581bd7ce1bbb4186a2f3))
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

### Chore

 - <csr-id-5e58e346612a094c7e637919c84ab1e78b59be27/> Rename Hydroflow -> DFIR
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
 - <csr-id-204bd117ca3a8845b4986539efb91a0c612dfa05/> add `repository` field to `Cargo.toml`s, fix #1452
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
    - Add `repository` field to `Cargo.toml`s, fix #1452 ([`204bd11`](https://github.com/hydro-project/hydro/commit/204bd117ca3a8845b4986539efb91a0c612dfa05))
 * **[#1620](https://github.com/hydro-project/hydro/issues/1620)**
    - Rename Hydroflow -> DFIR ([`5e58e34`](https://github.com/hydro-project/hydro/commit/5e58e346612a094c7e637919c84ab1e78b59be27))
 * **Uncategorized**
    - Release dfir_lang v0.11.0, dfir_datalog_core v0.11.0, dfir_datalog v0.11.0, dfir_macro v0.11.0, hydroflow_deploy_integration v0.11.0, lattices_macro v0.5.8, variadics v0.0.8, variadics_macro v0.5.6, lattices v0.5.9, multiplatform_test v0.4.0, pusherator v0.0.10, dfir_rs v0.11.0, hydro_deploy v0.11.0, stageleft_macro v0.5.0, stageleft v0.6.0, stageleft_tool v0.5.0, hydro_lang v0.11.0, hydro_std v0.11.0, hydro_cli v0.11.0, safety bump 6 crates ([`361b443`](https://github.com/hydro-project/hydro/commit/361b4439ef9c781860f18d511668ab463a8c5203))
</details>

## 0.3.0 (2024-11-08)

<csr-id-d5677604e93c07a5392f4229af94a0b736eca382/>

### Chore

 - <csr-id-d5677604e93c07a5392f4229af94a0b736eca382/> update pinned rust version, clippy lints, remove some dead code

### Chore

 - <csr-id-014ebb2628b5b80ea1b6426b58c4d62706edb9ef/> update pinned rust version, clippy lints, remove some dead code

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
    - Update pinned rust version, clippy lints, remove some dead code ([`014ebb2`](https://github.com/hydro-project/hydro/commit/014ebb2628b5b80ea1b6426b58c4d62706edb9ef))
 * **Uncategorized**
    - Release hydroflow_lang v0.10.0, hydroflow_datalog_core v0.10.0, hydroflow_datalog v0.10.0, hydroflow_deploy_integration v0.10.0, hydroflow_macro v0.10.0, lattices_macro v0.5.7, variadics v0.0.7, variadics_macro v0.5.5, lattices v0.5.8, multiplatform_test v0.3.0, pusherator v0.0.9, hydroflow v0.10.0, hydro_deploy v0.10.0, stageleft_macro v0.4.0, stageleft v0.5.0, stageleft_tool v0.4.0, hydroflow_plus v0.10.0, hydro_cli v0.10.0, safety bump 8 crates ([`258f480`](https://github.com/hydro-project/hydro/commit/258f4805dbcca36750cbfaaf36db00d3a007d817))
</details>

## 0.2.0 (2024-08-30)

<csr-id-11af32828bab6e4a4264d2635ff71a12bb0bb778/>

### Chore

 - <csr-id-11af32828bab6e4a4264d2635ff71a12bb0bb778/> lower min dependency versions where possible, update `Cargo.lock`
   Moved from #1418
   
   ---------

### Chore

 - <csr-id-2c04f51f1ec44f7898307b6610371dcb490ea686/> lower min dependency versions where possible, update `Cargo.lock`
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
    - Lower min dependency versions where possible, update `Cargo.lock` ([`2c04f51`](https://github.com/hydro-project/hydro/commit/2c04f51f1ec44f7898307b6610371dcb490ea686))
 * **Uncategorized**
    - Release hydroflow_lang v0.9.0, hydroflow_datalog_core v0.9.0, hydroflow_datalog v0.9.0, hydroflow_deploy_integration v0.9.0, hydroflow_macro v0.9.0, lattices_macro v0.5.6, lattices v0.5.7, multiplatform_test v0.2.0, variadics v0.0.6, pusherator v0.0.8, hydroflow v0.9.0, stageleft_macro v0.3.0, stageleft v0.4.0, stageleft_tool v0.3.0, hydroflow_plus v0.9.0, hydro_deploy v0.9.0, hydro_cli v0.9.0, hydroflow_plus_deploy v0.9.0, safety bump 8 crates ([`1d54331`](https://github.com/hydro-project/hydro/commit/1d54331976040c049e4c97a9fba0e66930efee52))
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

### Style

 - <csr-id-f28237376a2479fb042d68bd27aad71f357bdbb1/> fix imports

### Chore

 - <csr-id-a83b39c47e2acdb8909fb864454d90ab82581d6e/> fix clippy warning on latest nightly, fix docs
 - <csr-id-e5c5fcb25616ba00be955b318299c1cdf02bc241/> bump proc-macro2 min version to 1.0.63
 - <csr-id-dd270adee8ed4d29a20628c4082b0f29cfd6ebac/> fix lint, format errors for latest nightly version (without updated pinned)
   For nightly version (d9c13cd45 2023-07-05)

### Bug Fixes

 - <csr-id-8d3494b5afee858114a602a3e23077bb6d24dd77/> update proc-macro2, use new span location API where possible
   requires latest* rust nightly version
   
   *latest = 2023-06-28 or something
 - <csr-id-1ce417c4b2c9a3855cd2f51dfa3cf318c054f32b/> update proc-macro2, use new span location API where possible
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
 - <csr-id-3a67daeaea2c168ddc4c3d4a10615ecd96fddff3/> `hydroflow`, `logging`/`tracing` features
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
    - Fix clippy warning on latest nightly, fix docs ([`a83b39c`](https://github.com/hydro-project/hydro/commit/a83b39c47e2acdb8909fb864454d90ab82581d6e))
 * **[#609](https://github.com/hydro-project/hydro/issues/609)**
    - Update syn to 2.0 ([`2cd9295`](https://github.com/hydro-project/hydro/commit/2cd9295b978058bae26a565d2d0a5f0513cb0e82))
 * **[#617](https://github.com/hydro-project/hydro/issues/617)**
    - Update `Cargo.toml`s for publishing ([`3a08bb2`](https://github.com/hydro-project/hydro/commit/3a08bb2501482323e069c6c1f808d611ac679f1f))
 * **[#755](https://github.com/hydro-project/hydro/issues/755)**
    - `hydroflow`, `logging`/`tracing` features ([`3a67dae`](https://github.com/hydro-project/hydro/commit/3a67daeaea2c168ddc4c3d4a10615ecd96fddff3))
 * **[#801](https://github.com/hydro-project/hydro/issues/801)**
    - Update proc-macro2, use new span location API where possible ([`1ce417c`](https://github.com/hydro-project/hydro/commit/1ce417c4b2c9a3855cd2f51dfa3cf318c054f32b))
 * **[#822](https://github.com/hydro-project/hydro/issues/822)**
    - Fix lint, format errors for latest nightly version (without updated pinned) ([`dd270ad`](https://github.com/hydro-project/hydro/commit/dd270adee8ed4d29a20628c4082b0f29cfd6ebac))
 * **Uncategorized**
    - Release hydroflow_lang v0.7.0, hydroflow_datalog_core v0.7.0, hydroflow_datalog v0.7.0, hydroflow_macro v0.7.0, lattices v0.5.5, multiplatform_test v0.1.0, pusherator v0.0.6, hydroflow v0.7.0, stageleft_macro v0.2.0, stageleft v0.3.0, stageleft_tool v0.2.0, hydroflow_plus v0.7.0, hydro_deploy v0.7.0, hydro_cli v0.7.0, hydroflow_plus_cli_integration v0.7.0, safety bump 8 crates ([`855fda6`](https://github.com/hydro-project/hydro/commit/855fda65442ad7a9074a099ecc29e74322332418))
    - Fix imports ([`f282373`](https://github.com/hydro-project/hydro/commit/f28237376a2479fb042d68bd27aad71f357bdbb1))
    - Bump proc-macro2 min version to 1.0.63 ([`e5c5fcb`](https://github.com/hydro-project/hydro/commit/e5c5fcb25616ba00be955b318299c1cdf02bc241))
    - Setup release workflow ([`f4eb56d`](https://github.com/hydro-project/hydro/commit/f4eb56dacebe96a92cb7448bcce14b8b5093c9d5))
    - Turn on WASM tests ([`6918e2b`](https://github.com/hydro-project/hydro/commit/6918e2b8d61166e13b814d0abf078c62d8b69084))
</details>

## 0.0.0 (2023-04-25)

