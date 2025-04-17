# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.13.0 (2025-04-11)

### New Features (BREAKING)

 - <csr-id-dfb7a1b5ad47f03822e9b7cae7dae81914b305e2/> don't pull in dfir_rs during the compilation stage
   Because `hydro_lang` is responsible for _generating_ DFIR code, it
   doesn't actually need to depend on the runtime (`dfir_rs`), other than
   when it is used in the (legacy) macro mode or when we want to include
   utilities for runtime logic (`resource_measurement`). This sticks those
   pieces under feature flags and makes `dfir_rs` an optional dependency,
   which reduces the compile tree for crates like `hydro_test`.

 - <csr-id-xxxxxx/> refactor SSH handling to use `russh` instead of `libssh2`, introducing `async_ssh2_russh` crate
   This change replaces the `libssh2` dependency with `russh`, providing a more efficient and flexible SSH handling
   mechanism. The new `async_ssh2_russh` crate wraps `russh` to offer asynchronous SSH operations, including SFTP support.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 1 commit contributed to the release over the course of 23 calendar days.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1797](https://github.com/hydro-project/hydro/issues/1797)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1797](https://github.com/hydro-project/hydro/issues/1797)**
    - Don't pull in dfir_rs during the compilation stage ([`dfb7a1b`](https://github.com/hydro-project/hydro/commit/dfb7a1b5ad47f03822e9b7cae7dae81914b305e2))
</details>

## 0.12.1 (2025-03-15)

<csr-id-260902b210378af5291ec71a574256d7a5bcb463/>
<csr-id-7dd71d67da162d2e4f3043b271a52037a3c983c0/>

### Chore

 - <csr-id-260902b210378af5291ec71a574256d7a5bcb463/> set `hydro_deploy_integration` to release as `0.12.1`

### Refactor (BREAKING)

 - <csr-id-7dd71d67da162d2e4f3043b271a52037a3c983c0/> remove "hydroflow" for `hydro_deploy_integration`, `hydro_deploy::rust_crate`, fix #1712
   Opted to use `RustCrate` as the replacement prefix with the expectation
   that @shadaj may have a more coincise name in mind?

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1773](https://github.com/hydro-project/hydro/issues/1773)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1773](https://github.com/hydro-project/hydro/issues/1773)**
    - Remove "hydroflow" for `hydro_deploy_integration`, `hydro_deploy::rust_crate`, fix #1712 ([`7dd71d6`](https://github.com/hydro-project/hydro/commit/7dd71d67da162d2e4f3043b271a52037a3c983c0))
 * **Uncategorized**
    - Release dfir_lang v0.12.1, dfir_datalog_core v0.12.1, dfir_datalog v0.12.1, dfir_macro v0.12.1, hydro_deploy_integration v0.12.1, lattices v0.6.1, pusherator v0.0.12, dfir_rs v0.12.1, hydro_deploy v0.12.1, hydro_lang v0.12.1, hydro_std v0.12.1, hydro_cli v0.12.1 ([`23221b5`](https://github.com/hydro-project/hydro/commit/23221b53b30918707ddaa85529d04cd7919166b4))
    - Set `hydro_deploy_integration` to release as `0.12.1` ([`260902b`](https://github.com/hydro-project/hydro/commit/260902b210378af5291ec71a574256d7a5bcb463))
</details>

## 0.12.0 (2025-03-08)

<csr-id-49a387d4a21f0763df8ec94de73fb953c9cd333a/>
<csr-id-2e4f6b828e3d54ee6b1753c1a814006a3d0b3e34/>

### Chore

 - <csr-id-49a387d4a21f0763df8ec94de73fb953c9cd333a/> upgrade to Rust 2024 edition
   - Updates `Cargo.toml` to use new shared workspace keys
   - Updates lint settings (in workspace `Cargo.toml`)
   - `rustfmt` has changed slightly, resulting in a big diff - there are no
   actual code changes
   - Adds a script to `rustfmt` the template src files

### Refactor

 - <csr-id-2e4f6b828e3d54ee6b1753c1a814006a3d0b3e34/> improve naming of types involved with networking
   I got confused looking at my own code for client-server handshakes (in
   preparation for external client support). Time to do some renaming!

## 0.11.0 (2024-12-23)

### Documentation

 - <csr-id-28cd220c68e3660d9ebade113949a2346720cd04/> add `repository` field to `Cargo.toml`s, fix #1452
   #1452 
   
   Will trigger new releases of the following:
   `unchanged = 'hydroflow_deploy_integration', 'variadics',
   'variadics_macro', 'pusherator'`
   
   (All other crates already have changes, so would be released anyway)

## 0.10.0 (2024-11-08)

<csr-id-d5677604e93c07a5392f4229af94a0b736eca382/>

### Chore

 - <csr-id-d5677604e93c07a5392f4229af94a0b736eca382/> update pinned rust version, clippy lints, remove some dead code

## 0.9.0 (2024-08-30)

<csr-id-a2ec110ccadb97e293b19d83a155d98d94224bba/>
<csr-id-11af32828bab6e4a4264d2635ff71a12bb0bb778/>
<csr-id-c41787f527859cb9d704736ecdea5ca7bc641460/>
<csr-id-0a465e55dd39c76bc1aefb020460a639d792fe87/>

### Chore

 - <csr-id-a2ec110ccadb97e293b19d83a155d98d94224bba/> manually set versions for crates renamed in #1413
 - <csr-id-11af32828bab6e4a4264d2635ff71a12bb0bb778/> lower min dependency versions where possible, update `Cargo.lock`
   Moved from #1418
   
   ---------

### Other

 - <csr-id-c41787f527859cb9d704736ecdea5ca7bc641460/> update `RELEASING.md` notes, prep for release, wip

### Refactor (BREAKING)

 - <csr-id-0a465e55dd39c76bc1aefb020460a639d792fe87/> rename integration crates to drop CLI references

## 0.5.2 (2024-04-05)

<csr-id-ba2df44efd42b7c4d37ebefbf82e77c6f1d4cb94/>

### Refactor

 - <csr-id-ba2df44efd42b7c4d37ebefbf82e77c6f1d4cb94/> use `TcpListenerStream` instead of spawning task, fix #659

### Pre-Move Commit Statistics

<csr-read-only-do-not-edit/>

 - 1 commit contributed to the release over the course of 9 calendar days.
 - 67 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1121](https://github.com/hydro-project/hydroflow/issues/1121)

### Pre-Move Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1121](https://github.com/hydro-project/hydroflow/issues/1121)**
    - Use `TcpListenerStream` instead of spawning task, fix #659 ([`ba2df44`](https://github.com/hydro-project/hydroflow/commit/ba2df44efd42b7c4d37ebefbf82e77c6f1d4cb94))
</details>

## 0.5.1 (2024-01-29)

<csr-id-1b555e57c8c812bed4d6495d2960cbf77fb0b3ef/>
<csr-id-69e04167f4774cf1ca3351e7ac34d15cfa83362b/>

### New Features

 - <csr-id-9e275824c88b24d060a7de5822e1359959b36b03/> auto-configure Hydro Deploy based on Hydroflow+ plans
 - <csr-id-e5bdd12e32d6ea72fd91a55c12e09f07a0edaa5c/> add initial test using Hydro CLI from Hydroflow+
   This also required a change to Hydroflow core to make it possible to run the dataflow itself on a single thread (using a LocalSet), even if the surrounding runtime is not single-threaded (required to work around deadlocks because we can't use async APIs inside Hydroflow+). This requires us to spawn any Hydroflow tasks (only for `dest_sink` at the moment) right next to when we run the dataflow rather than when the Hydroflow graph is initialized. From a conceptual perspective, this seems _more right_, since now creating a Hydroflow program will not result in any actual tasks running.
   
   In the third PR of this series, I aim to add a new Hydroflow+ operator that will automate the setup of a `dest_sink`/`source_stream` pair that span nodes.
 - <csr-id-c50ca121b6d5e30dc07843f82caa135b68626301/> split Rust core from Python bindings

### Pre-Move Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release over the course of 3 calendar days.
 - 169 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#978](https://github.com/hydro-project/hydroflow/issues/978), [#982](https://github.com/hydro-project/hydroflow/issues/982)

### Chore

 - <csr-id-1b555e57c8c812bed4d6495d2960cbf77fb0b3ef/> manually set lockstep-versioned crates (and `lattices`) to version `0.5.1`
   Setting manually since
   https://github.com/frewsxcv/rust-crates-index/issues/159 is messing with
   smart-release
 - <csr-id-69e04167f4774cf1ca3351e7ac34d15cfa83362b/> generate pre-move changelogs for `hydro_cli` and `hydroflow_cli_integration`

### Pre-Move Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#978](https://github.com/hydro-project/hydroflow/issues/978)**
    - Add initial test using Hydro CLI from Hydroflow+ ([`e5bdd12`](https://github.com/hydro-project/hydroflow/commit/e5bdd12e32d6ea72fd91a55c12e09f07a0edaa5c))
 * **[#982](https://github.com/hydro-project/hydroflow/issues/982)**
    - Auto-configure Hydro Deploy based on Hydroflow+ plans ([`9e27582`](https://github.com/hydro-project/hydroflow/commit/9e275824c88b24d060a7de5822e1359959b36b03))
</details>

### Pre-Move Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release over the course of 39 calendar days.
 - 209 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#1046](https://github.com/hydro-project/hydroflow/issues/1046), [#986](https://github.com/hydro-project/hydroflow/issues/986)

### Pre-Move Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1046](https://github.com/hydro-project/hydroflow/issues/1046)**
    - Generate pre-move changelogs for `hydro_cli` and `hydroflow_cli_integration` ([`69e0416`](https://github.com/hydro-project/hydroflow/commit/69e04167f4774cf1ca3351e7ac34d15cfa83362b))
 * **[#986](https://github.com/hydro-project/hydroflow/issues/986)**
    - Split Rust core from Python bindings ([`c50ca12`](https://github.com/hydro-project/hydroflow/commit/c50ca121b6d5e30dc07843f82caa135b68626301))
 * **Uncategorized**
    - Release hydroflow_cli_integration v0.5.1, hydroflow_lang v0.5.1, hydroflow_datalog_core v0.5.1, hydroflow_datalog v0.5.1, hydroflow_macro v0.5.1, lattices v0.5.1, variadics v0.0.3, pusherator v0.0.4, hydroflow v0.5.1, stageleft_macro v0.1.0, stageleft v0.1.0, hydroflow_plus v0.5.1, hydro_deploy v0.5.1, hydro_cli v0.5.1 ([`478aebc`](https://github.com/hydro-project/hydroflow/commit/478aebc8fee2aa78eab86bd386322db1c70bde6a))
    - Manually set lockstep-versioned crates (and `lattices`) to version `0.5.1` ([`1b555e5`](https://github.com/hydro-project/hydroflow/commit/1b555e57c8c812bed4d6495d2960cbf77fb0b3ef))
</details>

## 0.3.0 (2023-07-04)

### Bug Fixes

 - <csr-id-a3c1fbbd1e3fa7a7299878f61b4bfd12dce0052c/> remove nightly feature `never_type` where unused

### Pre-Move Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release over the course of 12 calendar days.
 - 33 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#780](https://github.com/hydro-project/hydroflow/issues/780)

### Pre-Move Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#780](https://github.com/hydro-project/hydroflow/issues/780)**
    - Remove nightly feature `never_type` where unused ([`a3c1fbb`](https://github.com/hydro-project/hydroflow/commit/a3c1fbbd1e3fa7a7299878f61b4bfd12dce0052c))
 * **Uncategorized**
    - Release hydroflow_cli_integration v0.3.0, hydroflow_lang v0.3.0, hydroflow_datalog_core v0.3.0, hydroflow_datalog v0.3.0, hydroflow_macro v0.3.0, lattices v0.3.0, pusherator v0.0.2, hydroflow v0.3.0, hydro_cli v0.3.0, safety bump 5 crates ([`ec9633e`](https://github.com/hydro-project/hydroflow/commit/ec9633e2e393c2bf106223abeb0b680200fbdf84))
</details>

## 0.2.0 (2023-05-31)

<csr-id-fd896fbe925fbd8ef1d16be7206ac20ba585081a/>

### Chore

 - <csr-id-fd896fbe925fbd8ef1d16be7206ac20ba585081a/> manually bump versions for v0.2.0 release

### New Features

 - <csr-id-8b2c9f09b1f423ac6d562c29d4ea587578f1c98a/> Add more detailed Hydro Deploy docs and rename `ConnectedBidi` => `ConnectedDirect`

### Pre-Move Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 2 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#723](https://github.com/hydro-project/hydroflow/issues/723)

### Pre-Move Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#723](https://github.com/hydro-project/hydroflow/issues/723)**
    - Add more detailed Hydro Deploy docs and rename `ConnectedBidi` => `ConnectedDirect` ([`8b2c9f0`](https://github.com/hydro-project/hydroflow/commit/8b2c9f09b1f423ac6d562c29d4ea587578f1c98a))
 * **Uncategorized**
    - Release hydroflow_cli_integration v0.2.0 ([`71d940b`](https://github.com/hydro-project/hydroflow/commit/71d940b1b2ca379543f28c4e827f895fc5efa4bd))
    - Manually bump versions for v0.2.0 release ([`fd896fb`](https://github.com/hydro-project/hydroflow/commit/fd896fbe925fbd8ef1d16be7206ac20ba585081a))
</details>

## 0.1.1 (2023-05-30)

### New Features

 - <csr-id-4536ac6bbcd14a621b5a039d7fe213bff72a8db1/> finish up WebSocket chat example and avoid deadlocks in network setup

### Bug Fixes

 - <csr-id-2adfdd2867092352121e4f232b63928a810948d3/> fix CLI build on windows

### Pre-Move Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 6 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#708](https://github.com/hydro-project/hydroflow/issues/708), [#717](https://github.com/hydro-project/hydroflow/issues/717)

### Pre-Move Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#708](https://github.com/hydro-project/hydroflow/issues/708)**
    - Finish up WebSocket chat example and avoid deadlocks in network setup ([`4536ac6`](https://github.com/hydro-project/hydroflow/commit/4536ac6bbcd14a621b5a039d7fe213bff72a8db1))
 * **[#717](https://github.com/hydro-project/hydroflow/issues/717)**
    - Fix CLI build on windows ([`2adfdd2`](https://github.com/hydro-project/hydroflow/commit/2adfdd2867092352121e4f232b63928a810948d3))
 * **Uncategorized**
    - Release hydroflow_cli_integration v0.1.1, hydroflow_lang v0.1.1, hydroflow_datalog_core v0.1.1, hydroflow_macro v0.1.1, lattices v0.1.2, hydroflow v0.1.1, hydro_cli v0.1.0 ([`d9fa8b3`](https://github.com/hydro-project/hydroflow/commit/d9fa8b387e303b33d9614dbde80abf1af08bd8eb))
</details>

## 0.1.0 (2023-05-23)

<csr-id-52ee8f8e443f0a8b5caf92d2c5f028c00302a79b/>

### Chore

 - <csr-id-52ee8f8e443f0a8b5caf92d2c5f028c00302a79b/> bump versions to 0.1.0 for release
   For release on crates.io for v0.1

### Pre-Move Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 2 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#684](https://github.com/hydro-project/hydroflow/issues/684)

### Pre-Move Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#684](https://github.com/hydro-project/hydroflow/issues/684)**
    - Bump versions to 0.1.0 for release ([`52ee8f8`](https://github.com/hydro-project/hydroflow/commit/52ee8f8e443f0a8b5caf92d2c5f028c00302a79b))
 * **Uncategorized**
    - Release hydroflow_cli_integration v0.1.0, hydroflow_internalmacro v0.1.0, hydroflow_lang v0.1.0, hydroflow_datalog_core v0.1.0, hydroflow_datalog v0.1.0, hydroflow_macro v0.1.0, lattices v0.1.1, hydroflow v0.1.0 ([`7324974`](https://github.com/hydro-project/hydroflow/commit/73249744293c9b89cbaa2d84b23ca3f25b00ae4e))
</details>

## 0.0.1 (2023-05-21)

<csr-id-cd0a86d9271d0e3daab59c46f079925f863424e1/>
<csr-id-20a1b2c0cd04a8b495a02ce345db3d48a99ea0e9/>
<csr-id-1eda91a2ef8794711ef037240f15284e8085d863/>

### Style

 - <csr-id-cd0a86d9271d0e3daab59c46f079925f863424e1/> Warn lint `unused_qualifications`
 - <csr-id-20a1b2c0cd04a8b495a02ce345db3d48a99ea0e9/> rustfmt group imports
 - <csr-id-1eda91a2ef8794711ef037240f15284e8085d863/> rustfmt prescribe flat-module `use` format

### Pre-Move Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release over the course of 13 calendar days.
 - 24 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 3 unique issues were worked on: [#646](https://github.com/hydro-project/hydroflow/issues/646), [#656](https://github.com/hydro-project/hydroflow/issues/656), [#660](https://github.com/hydro-project/hydroflow/issues/660)

### Pre-Move Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#646](https://github.com/hydro-project/hydroflow/issues/646)**
    - Update pinned `nightly-2023-05-07`, fix lints ([`19d5d96`](https://github.com/hydro-project/hydroflow/commit/19d5d963ac11bef951f00b93d0509679082cedef))
 * **[#656](https://github.com/hydro-project/hydroflow/issues/656)**
    - Add WebSocket with CLI example and simplify init API ([`1015980`](https://github.com/hydro-project/hydroflow/commit/1015980ed995634ff8735e4daf33796e73bab563))
 * **[#660](https://github.com/hydro-project/hydroflow/issues/660)**
    - Warn lint `unused_qualifications` ([`cd0a86d`](https://github.com/hydro-project/hydroflow/commit/cd0a86d9271d0e3daab59c46f079925f863424e1))
    - Rustfmt group imports ([`20a1b2c`](https://github.com/hydro-project/hydroflow/commit/20a1b2c0cd04a8b495a02ce345db3d48a99ea0e9))
    - Rustfmt prescribe flat-module `use` format ([`1eda91a`](https://github.com/hydro-project/hydroflow/commit/1eda91a2ef8794711ef037240f15284e8085d863))
 * **Uncategorized**
    - Release hydroflow_cli_integration v0.0.1, hydroflow_lang v0.0.1, hydroflow_datalog_core v0.0.1, hydroflow_datalog v0.0.1, hydroflow_macro v0.0.1, lattices v0.1.0, variadics v0.0.2, pusherator v0.0.1, hydroflow v0.0.2 ([`809395a`](https://github.com/hydro-project/hydroflow/commit/809395acddb78949d7a2bf036e1a94972f23b1ad))
</details>

## 0.0.0 (2023-04-26)

### Pre-Move Commit Statistics

<csr-read-only-do-not-edit/>

 - 20 commits contributed to the release over the course of 46 calendar days.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 19 unique issues were worked on: [#452](https://github.com/hydro-project/hydroflow/issues/452), [#461](https://github.com/hydro-project/hydroflow/issues/461), [#466](https://github.com/hydro-project/hydroflow/issues/466), [#472](https://github.com/hydro-project/hydroflow/issues/472), [#477](https://github.com/hydro-project/hydroflow/issues/477), [#479](https://github.com/hydro-project/hydroflow/issues/479), [#484](https://github.com/hydro-project/hydroflow/issues/484), [#498](https://github.com/hydro-project/hydroflow/issues/498), [#513](https://github.com/hydro-project/hydroflow/issues/513), [#533](https://github.com/hydro-project/hydroflow/issues/533), [#541](https://github.com/hydro-project/hydroflow/issues/541), [#545](https://github.com/hydro-project/hydroflow/issues/545), [#560](https://github.com/hydro-project/hydroflow/issues/560), [#563](https://github.com/hydro-project/hydroflow/issues/563), [#575](https://github.com/hydro-project/hydroflow/issues/575), [#576](https://github.com/hydro-project/hydroflow/issues/576), [#584](https://github.com/hydro-project/hydroflow/issues/584), [#613](https://github.com/hydro-project/hydroflow/issues/613), [#617](https://github.com/hydro-project/hydroflow/issues/617)

### Pre-Move Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#452](https://github.com/hydro-project/hydroflow/issues/452)**
    - Build CLI wheels in CI and minimize CLI dependencies ([`3e33d0c`](https://github.com/hydro-project/hydroflow/commit/3e33d0cf6b068f0567e55462732598f8a4e2da6a))
 * **[#461](https://github.com/hydro-project/hydroflow/issues/461)**
    - Support networking topologies that mix local and cloud through SSH tunneling ([`0ec6d88`](https://github.com/hydro-project/hydroflow/commit/0ec6d889469331a212c04f9568136f770f0c973d))
 * **[#466](https://github.com/hydro-project/hydroflow/issues/466)**
    - Add APIs for sending data to a Hydroflow service from Python ([`c2203a1`](https://github.com/hydro-project/hydroflow/commit/c2203a15f0144308365af227f3ca044ae6a7954b))
 * **[#472](https://github.com/hydro-project/hydroflow/issues/472)**
    - Some clippy/windows fixes for hydro cli code ([`bbf7b50`](https://github.com/hydro-project/hydroflow/commit/bbf7b506463d7fceb5d87005c7cb270a2b0521df))
 * **[#477](https://github.com/hydro-project/hydroflow/issues/477)**
    - Properly handle interrupts and fix non-flushing demux ([`00ea017`](https://github.com/hydro-project/hydroflow/commit/00ea017e40b796e7561979efa0921658dfe072fd))
 * **[#479](https://github.com/hydro-project/hydroflow/issues/479)**
    - Allow custom ports to be used as sinks ([`8da15b7`](https://github.com/hydro-project/hydroflow/commit/8da15b7cbd8bdbf960d3ed58b69f98538ccacd2c))
 * **[#484](https://github.com/hydro-project/hydroflow/issues/484)**
    - Add merge API to CLI to have multiple sources for one sink ([`e09b567`](https://github.com/hydro-project/hydroflow/commit/e09b5670795292f66a004f41314c3c4aa7a24eeb))
 * **[#498](https://github.com/hydro-project/hydroflow/issues/498)**
    - Add API to get CLI connection config as JSON ([`323e0f0`](https://github.com/hydro-project/hydroflow/commit/323e0f0afd73b66f321b2e88498627e76a186a4e))
 * **[#513](https://github.com/hydro-project/hydroflow/issues/513)**
    - Add `hydro.null` API to connect no-op sources and sinks ([`9b2a4a6`](https://github.com/hydro-project/hydroflow/commit/9b2a4a690798d2a976221901fa25a908b7600f52))
 * **[#533](https://github.com/hydro-project/hydroflow/issues/533)**
    - Add `hydro.mux` operator and initial API tests ([`c25272b`](https://github.com/hydro-project/hydroflow/commit/c25272b90f8cc5ec7614caa29f0be889d2220510))
 * **[#541](https://github.com/hydro-project/hydroflow/issues/541)**
    - Fixup! Start accepting connections in the background of CLI initialization to avoid deadlocks ([`d2480f7`](https://github.com/hydro-project/hydroflow/commit/d2480f7b92a9067f63d736c6ba72f1dbc0614d0f))
    - Start accepting connections in the background of CLI initialization to avoid deadlocks ([`681a6ba`](https://github.com/hydro-project/hydroflow/commit/681a6baef73de8a67e140526ede4b36e239976f0))
 * **[#545](https://github.com/hydro-project/hydroflow/issues/545)**
    - Fixup! Start accepting connections in the background of CLI initialization to avoid deadlocks ([`d2480f7`](https://github.com/hydro-project/hydroflow/commit/d2480f7b92a9067f63d736c6ba72f1dbc0614d0f))
 * **[#560](https://github.com/hydro-project/hydroflow/issues/560)**
    - Refactor `hydro.mux` to `source.tagged(id)` and support connections where the tagged source is the server ([`3f0ecc9`](https://github.com/hydro-project/hydroflow/commit/3f0ecc92abed7a0c95c04255adcc6d39c0767703))
 * **[#563](https://github.com/hydro-project/hydroflow/issues/563)**
    - Don't drop the write half of a stream even if we are only reading in the CLI ([`aef43be`](https://github.com/hydro-project/hydroflow/commit/aef43be9c0a3cc39229c90cde9c8f5ed8b8198c8))
 * **[#575](https://github.com/hydro-project/hydroflow/issues/575)**
    - Place a buffer over each sink of a demux to avoid serial message sending ([`a26f759`](https://github.com/hydro-project/hydroflow/commit/a26f759d717fd1685fc53181b4c2fd9b7b93a544))
 * **[#576](https://github.com/hydro-project/hydroflow/issues/576)**
    - Add classic counter CRDT benchmark to compare against ([`2f3bf04`](https://github.com/hydro-project/hydroflow/commit/2f3bf04ab33768b04d44f3f58907f958d4cd8dc8))
 * **[#584](https://github.com/hydro-project/hydroflow/issues/584)**
    - Fix windows build ([`5aa96c4`](https://github.com/hydro-project/hydroflow/commit/5aa96c451ba69ff2beed41528b8c847b75c45ea7))
 * **[#613](https://github.com/hydro-project/hydroflow/issues/613)**
    - Improve error message when key is not found in a demux ([`f9f9e72`](https://github.com/hydro-project/hydroflow/commit/f9f9e729affe41d37b2414e7c5dfc5e54caf82a7))
 * **[#617](https://github.com/hydro-project/hydroflow/issues/617)**
    - Update `Cargo.toml`s for publishing ([`a78ff9a`](https://github.com/hydro-project/hydroflow/commit/a78ff9aace6771787c2b72aad83be6ad8d49a828))
 * **Uncategorized**
    - Setup release workflow ([`108d0e9`](https://github.com/hydro-project/hydroflow/commit/108d0e933a08b183c4dadf8c3499e4946696e263))
</details>

