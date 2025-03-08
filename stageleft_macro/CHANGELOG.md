# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.6.0 (2025-03-08)

### Chore

 - <csr-id-49a387d4a21f0763df8ec94de73fb953c9cd333a/> upgrade to Rust 2024 edition
   - Updates `Cargo.toml` to use new shared workspace keys
   - Updates lint settings (in workspace `Cargo.toml`)
   - `rustfmt` has changed slightly, resulting in a big diff - there are no
   actual code changes
   - Adds a script to `rustfmt` the template src files

### Refactor

 - <csr-id-c293cca6855695107e9cef5c5df99fb04a571934/> enable lints, cleanups for Rust 2024 #1732

### Bug Fixes (BREAKING)

 - <csr-id-c49a4913cfdae021404a86e5a4d0597aa4db9fbe/> reduce where `#[cfg(stageleft_runtime)]` needs to be used
   Simplifies the logic for generating the public clone of the code, which
   eliminates the need to sprinkle `#[cfg(stageleft_runtime)]` (renamed
   from `#[stageleft::runtime]`) everywhere. Also adds logic to pass
   through `cfg` attrs when re-exporting public types.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release over the course of 21 calendar days.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 3 unique issues were worked on: [#1721](https://github.com/hydro-project/hydro/issues/1721), [#1737](https://github.com/hydro-project/hydro/issues/1737), [#1747](https://github.com/hydro-project/hydro/issues/1747)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1721](https://github.com/hydro-project/hydro/issues/1721)**
    - Reduce where `#[cfg(stageleft_runtime)]` needs to be used ([`c49a491`](https://github.com/hydro-project/hydro/commit/c49a4913cfdae021404a86e5a4d0597aa4db9fbe))
 * **[#1737](https://github.com/hydro-project/hydro/issues/1737)**
    - Enable lints, cleanups for Rust 2024 #1732 ([`c293cca`](https://github.com/hydro-project/hydro/commit/c293cca6855695107e9cef5c5df99fb04a571934))
 * **[#1747](https://github.com/hydro-project/hydro/issues/1747)**
    - Upgrade to Rust 2024 edition ([`49a387d`](https://github.com/hydro-project/hydro/commit/49a387d4a21f0763df8ec94de73fb953c9cd333a))
</details>

## 0.5.0 (2024-12-23)

<csr-id-0dc709ed5a53c723f47fa1d10063e57bb50a63c8/>
<csr-id-162e49cf8a8cf944cded7f775d6f78afe4a89837/>

### Chore

 - <csr-id-0dc709ed5a53c723f47fa1d10063e57bb50a63c8/> use same hashing library everywhere

### Bug Fixes

 - <csr-id-b14d6522a29188b5306b47e32a3af24391ae8b2a/> syn missing features

### Documentation

 - <csr-id-28cd220c68e3660d9ebade113949a2346720cd04/> add `repository` field to `Cargo.toml`s, fix #1452
   #1452 
   
   Will trigger new releases of the following:
   `unchanged = 'hydroflow_deploy_integration', 'variadics',
   'variadics_macro', 'pusherator'`
   
   (All other crates already have changes, so would be released anyway)
 - <csr-id-e1a08e5d165fbc80da2ae695e507078a97a9031f/> update `CHANGELOG.md`s for big rename
   Generated before rename per `RELEASING.md` instructions.

### Chore

 - <csr-id-162e49cf8a8cf944cded7f775d6f78afe4a89837/> Rename HydroflowPlus to Hydro

### New Features (BREAKING)

 - <csr-id-a93a5e59e1681d325b3433193bb86254d23bdc77/> allow cluster self ID to be referenced as a global constant
   This eliminates the need to store `cluster.self_id()` in a local
   variable first, instead you can directly reference `CLUSTER_SELF_ID`.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 8 commits contributed to the release.
 - 45 days passed between releases.
 - 6 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 4 unique issues were worked on: [#1501](https://github.com/hydro-project/hydro/issues/1501), [#1574](https://github.com/hydro-project/hydro/issues/1574), [#1590](https://github.com/hydro-project/hydro/issues/1590), [#1617](https://github.com/hydro-project/hydro/issues/1617)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1501](https://github.com/hydro-project/hydro/issues/1501)**
    - Add `repository` field to `Cargo.toml`s, fix #1452 ([`28cd220`](https://github.com/hydro-project/hydro/commit/28cd220c68e3660d9ebade113949a2346720cd04))
 * **[#1574](https://github.com/hydro-project/hydro/issues/1574)**
    - Allow cluster self ID to be referenced as a global constant ([`a93a5e5`](https://github.com/hydro-project/hydro/commit/a93a5e59e1681d325b3433193bb86254d23bdc77))
 * **[#1590](https://github.com/hydro-project/hydro/issues/1590)**
    - Use same hashing library everywhere ([`0dc709e`](https://github.com/hydro-project/hydro/commit/0dc709ed5a53c723f47fa1d10063e57bb50a63c8))
 * **[#1617](https://github.com/hydro-project/hydro/issues/1617)**
    - Rename HydroflowPlus to Hydro ([`162e49c`](https://github.com/hydro-project/hydro/commit/162e49cf8a8cf944cded7f775d6f78afe4a89837))
 * **Uncategorized**
    - Release stageleft_macro v0.5.0, stageleft v0.6.0, stageleft_tool v0.5.0, hydro_lang v0.11.0, hydro_std v0.11.0, hydro_cli v0.11.0 ([`b58dccc`](https://github.com/hydro-project/hydro/commit/b58dccc7f85380951a0ae91d32548eff0784f3a7))
    - Syn missing features ([`b14d652`](https://github.com/hydro-project/hydro/commit/b14d6522a29188b5306b47e32a3af24391ae8b2a))
    - Release dfir_lang v0.11.0, dfir_datalog_core v0.11.0, dfir_datalog v0.11.0, dfir_macro v0.11.0, hydroflow_deploy_integration v0.11.0, lattices_macro v0.5.8, variadics v0.0.8, variadics_macro v0.5.6, lattices v0.5.9, multiplatform_test v0.4.0, pusherator v0.0.10, dfir_rs v0.11.0, hydro_deploy v0.11.0, stageleft_macro v0.5.0, stageleft v0.6.0, stageleft_tool v0.5.0, hydro_lang v0.11.0, hydro_std v0.11.0, hydro_cli v0.11.0, safety bump 6 crates ([`9a7e486`](https://github.com/hydro-project/hydro/commit/9a7e48693fce0face0f8ad16349258cdbe26395f))
    - Update `CHANGELOG.md`s for big rename ([`e1a08e5`](https://github.com/hydro-project/hydro/commit/e1a08e5d165fbc80da2ae695e507078a97a9031f))
</details>

## v0.4.0 (2024-11-08)

<csr-id-d5677604e93c07a5392f4229af94a0b736eca382/>

### Chore

 - <csr-id-d5677604e93c07a5392f4229af94a0b736eca382/> update pinned rust version, clippy lints, remove some dead code

### New Features

 - <csr-id-afe78c343658472513b34d28658634b253148aee/> add ability to have staged flows inside unit tests
   Whenever a Hydroflow+ program is compiled, it depends on a generated
   `__staged` module, which contains the entire contents of the crate but
   with every type / function made `pub` and exported, so that the compiled
   UDFs can resolve local references appropriately.
   
   Previously, we would not do this for `#[cfg(test)]` modules, since they
   may use `dev-dependencies` and therefore the generated module may fail
   to compile when not in test mode. To solve this, when running a unit
   test (marked with `hydroflow_plus::deploy::init_test()`) that uses
   trybuild, we emit a version of the `__staged` module with `#[cfg(test)]`
   modules included _into the generated trybuild sources_ because we can
   guarantee via trybuild that the appropriate `dev-dependencies` are
   available.
   
   This by itself allows crates depending on `hydroflow_plus` to have local
   unit tests with Hydroflow+ logic inside them. But we also want to use
   this support for unit tests inside `hydroflow_plus` itself. To enable
   that, we eliminate the `hydroflow_plus_deploy` crate and move its
   contents directly to `hydroflow_plus` itself so that we can access the
   trybuild machinery without incurring a circular dependency.
   
   Also fixes #1408
 - <csr-id-60d9becaf0b67f9819316ce6d76bd867f7d46505/> splice UDFs with type hints to avoid inference failures

### Bug Fixes

 - <csr-id-486dfbe1fab51f1b7c7aa03a51e6e9e4e427b912/> support tuple patterns

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release.
 - 69 days passed between releases.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 4 unique issues were worked on: [#1434](https://github.com/hydro-project/hydro/issues/1434), [#1444](https://github.com/hydro-project/hydro/issues/1444), [#1445](https://github.com/hydro-project/hydro/issues/1445), [#1450](https://github.com/hydro-project/hydro/issues/1450)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1434](https://github.com/hydro-project/hydro/issues/1434)**
    - Splice UDFs with type hints to avoid inference failures ([`60d9bec`](https://github.com/hydro-project/hydro/commit/60d9becaf0b67f9819316ce6d76bd867f7d46505))
 * **[#1444](https://github.com/hydro-project/hydro/issues/1444)**
    - Update pinned rust version, clippy lints, remove some dead code ([`d567760`](https://github.com/hydro-project/hydro/commit/d5677604e93c07a5392f4229af94a0b736eca382))
 * **[#1445](https://github.com/hydro-project/hydro/issues/1445)**
    - Support tuple patterns ([`486dfbe`](https://github.com/hydro-project/hydro/commit/486dfbe1fab51f1b7c7aa03a51e6e9e4e427b912))
 * **[#1450](https://github.com/hydro-project/hydro/issues/1450)**
    - Add ability to have staged flows inside unit tests ([`afe78c3`](https://github.com/hydro-project/hydro/commit/afe78c343658472513b34d28658634b253148aee))
 * **Uncategorized**
    - Release hydroflow_lang v0.10.0, hydroflow_datalog_core v0.10.0, hydroflow_datalog v0.10.0, hydroflow_deploy_integration v0.10.0, hydroflow_macro v0.10.0, lattices_macro v0.5.7, variadics v0.0.7, variadics_macro v0.5.5, lattices v0.5.8, multiplatform_test v0.3.0, pusherator v0.0.9, hydroflow v0.10.0, hydro_deploy v0.10.0, stageleft_macro v0.4.0, stageleft v0.5.0, stageleft_tool v0.4.0, hydroflow_plus v0.10.0, hydro_cli v0.10.0, safety bump 8 crates ([`dcd48fc`](https://github.com/hydro-project/hydro/commit/dcd48fc7ee805898d9b5ef0d082870e30615e95b))
</details>

## v0.3.0 (2024-08-30)

<csr-id-11af32828bab6e4a4264d2635ff71a12bb0bb778/>
<csr-id-461ae845c6c6506c733be6287eeefe6e3beca52c/>

### Chore

 - <csr-id-11af32828bab6e4a4264d2635ff71a12bb0bb778/> lower min dependency versions where possible, update `Cargo.lock`
   Moved from #1418
   
   ---------

### Documentation

 - <csr-id-f5f1eb0c612f5c0c1752360d972ef6853c5e12f0/> cleanup doc comments for clippy latest

### Refactor

 - <csr-id-461ae845c6c6506c733be6287eeefe6e3beca52c/> remove `lazy_static` dependency

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 97 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 3 unique issues were worked on: [#1423](https://github.com/hydro-project/hydro/issues/1423), [#1426](https://github.com/hydro-project/hydro/issues/1426), [#1428](https://github.com/hydro-project/hydro/issues/1428)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1423](https://github.com/hydro-project/hydro/issues/1423)**
    - Lower min dependency versions where possible, update `Cargo.lock` ([`11af328`](https://github.com/hydro-project/hydro/commit/11af32828bab6e4a4264d2635ff71a12bb0bb778))
 * **[#1426](https://github.com/hydro-project/hydro/issues/1426)**
    - Remove `lazy_static` dependency ([`461ae84`](https://github.com/hydro-project/hydro/commit/461ae845c6c6506c733be6287eeefe6e3beca52c))
 * **[#1428](https://github.com/hydro-project/hydro/issues/1428)**
    - Cleanup doc comments for clippy latest ([`f5f1eb0`](https://github.com/hydro-project/hydro/commit/f5f1eb0c612f5c0c1752360d972ef6853c5e12f0))
 * **Uncategorized**
    - Release hydroflow_lang v0.9.0, hydroflow_datalog_core v0.9.0, hydroflow_datalog v0.9.0, hydroflow_deploy_integration v0.9.0, hydroflow_macro v0.9.0, lattices_macro v0.5.6, lattices v0.5.7, multiplatform_test v0.2.0, variadics v0.0.6, pusherator v0.0.8, hydroflow v0.9.0, stageleft_macro v0.3.0, stageleft v0.4.0, stageleft_tool v0.3.0, hydroflow_plus v0.9.0, hydro_deploy v0.9.0, hydro_cli v0.9.0, hydroflow_plus_deploy v0.9.0, safety bump 8 crates ([`0750117`](https://github.com/hydro-project/hydro/commit/0750117de7088c01a439b102adeb4c832889f171))
</details>

## v0.2.0 (2024-05-24)

### New Features

 - <csr-id-93fd05e5ff256e2e0a3b513695ff869c32344447/> re-compile staged sources for the macro at the top level

### Bug Fixes

 - <csr-id-0cafbdb74a665412a83aa900b4eb10c00e2498dd/> handle send_bincode with local structs
   fix(hydroflow_plus): handle send_bincode with local structs

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 48 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#1104](https://github.com/hydro-project/hydro/issues/1104), [#1151](https://github.com/hydro-project/hydro/issues/1151)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1104](https://github.com/hydro-project/hydro/issues/1104)**
    - Re-compile staged sources for the macro at the top level ([`93fd05e`](https://github.com/hydro-project/hydro/commit/93fd05e5ff256e2e0a3b513695ff869c32344447))
 * **[#1151](https://github.com/hydro-project/hydro/issues/1151)**
    - Handle send_bincode with local structs ([`0cafbdb`](https://github.com/hydro-project/hydro/commit/0cafbdb74a665412a83aa900b4eb10c00e2498dd))
 * **Uncategorized**
    - Release hydroflow_lang v0.7.0, hydroflow_datalog_core v0.7.0, hydroflow_datalog v0.7.0, hydroflow_macro v0.7.0, lattices v0.5.5, multiplatform_test v0.1.0, pusherator v0.0.6, hydroflow v0.7.0, stageleft_macro v0.2.0, stageleft v0.3.0, stageleft_tool v0.2.0, hydroflow_plus v0.7.0, hydro_deploy v0.7.0, hydro_cli v0.7.0, hydroflow_plus_cli_integration v0.7.0, safety bump 8 crates ([`2852147`](https://github.com/hydro-project/hydro/commit/285214740627685e911781793e05d234ab2ad2bd))
</details>

## v0.1.1 (2024-04-05)

<csr-id-7958fb0d900be8fe7359326abfa11dcb8fb35e8a/>

### New Features

 - <csr-id-77f3e5afb9e276d1d6c643574ebac75ed0003939/> simplify lifetime bounds for processes and clusters
   feat(hydroflow_plus): simplify lifetime bounds for processes and
   clusters
   
   This allows `extract` to move the flow builder, which is a prerequisite
   for having developers run the optimizer during deployment as well in
   case it changes the network topology.

### Bug Fixes

 - <csr-id-6cc5a7f78fc8bd28f32dad58857ccd1c46bc93cd/> handle multiple references to copy-able free variables
 - <csr-id-54b4ee3fc847db14a0b825e04eae6e1288d44af3/> ignore unnecessary lifetime lint on entrypoints

### Style

 - <csr-id-7958fb0d900be8fe7359326abfa11dcb8fb35e8a/> qualified path cleanups for clippy

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 4 unique issues were worked on: [#1090](https://github.com/hydro-project/hydro/issues/1090), [#1100](https://github.com/hydro-project/hydro/issues/1100), [#1117](https://github.com/hydro-project/hydro/issues/1117), [#1124](https://github.com/hydro-project/hydro/issues/1124)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1090](https://github.com/hydro-project/hydro/issues/1090)**
    - Qualified path cleanups for clippy ([`7958fb0`](https://github.com/hydro-project/hydro/commit/7958fb0d900be8fe7359326abfa11dcb8fb35e8a))
 * **[#1100](https://github.com/hydro-project/hydro/issues/1100)**
    - Simplify lifetime bounds for processes and clusters ([`77f3e5a`](https://github.com/hydro-project/hydro/commit/77f3e5afb9e276d1d6c643574ebac75ed0003939))
 * **[#1117](https://github.com/hydro-project/hydro/issues/1117)**
    - Ignore unnecessary lifetime lint on entrypoints ([`54b4ee3`](https://github.com/hydro-project/hydro/commit/54b4ee3fc847db14a0b825e04eae6e1288d44af3))
 * **[#1124](https://github.com/hydro-project/hydro/issues/1124)**
    - Handle multiple references to copy-able free variables ([`6cc5a7f`](https://github.com/hydro-project/hydro/commit/6cc5a7f78fc8bd28f32dad58857ccd1c46bc93cd))
 * **Uncategorized**
    - Release hydroflow_cli_integration v0.5.2, hydroflow_lang v0.6.1, hydroflow_datalog_core v0.6.1, lattices v0.5.4, hydroflow v0.6.1, stageleft_macro v0.1.1, stageleft v0.2.1, hydroflow_plus v0.6.1, hydro_deploy v0.6.1, hydro_cli v0.6.1, hydroflow_plus_cli_integration v0.6.1, stageleft_tool v0.1.1 ([`cd63f22`](https://github.com/hydro-project/hydro/commit/cd63f2258c961a40f0e5dbef20ac329a2d570ad0))
</details>

## v0.1.0 (2024-01-29)

### Documentation

 - <csr-id-3b36020d16792f26da4df3c5b09652a4ab47ec4f/> actually committing empty CHANGELOG.md is required

### New Features

 - <csr-id-af6e3be60fdb69ceec1613347910f4dd49980d34/> push down persists and implement Pi example
   Also fixes type inference issues with reduce the same way as we did for fold.
 - <csr-id-71083233afc01e0132d7186f4af8c0b4a6323ec7/> support crates that have no entrypoints
   Also includes various bugfixes needed for Hydroflow+.
 - <csr-id-e5bdd12e32d6ea72fd91a55c12e09f07a0edaa5c/> add initial test using Hydro CLI from Hydroflow+
   This also required a change to Hydroflow core to make it possible to run the dataflow itself on a single thread (using a LocalSet), even if the surrounding runtime is not single-threaded (required to work around deadlocks because we can't use async APIs inside Hydroflow+). This requires us to spawn any Hydroflow tasks (only for `dest_sink` at the moment) right next to when we run the dataflow rather than when the Hydroflow graph is initialized. From a conceptual perspective, this seems _more right_, since now creating a Hydroflow program will not result in any actual tasks running.
   
   In the third PR of this series, I aim to add a new Hydroflow+ operator that will automate the setup of a `dest_sink`/`source_stream` pair that span nodes.
 - <csr-id-8b635683e5ac3c4ed2d896ae88e2953db1c6312c/> add a functional surface syntax using staging

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release over the course of 76 calendar days.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 4 unique issues were worked on: [#1021](https://github.com/hydro-project/hydro/issues/1021), [#899](https://github.com/hydro-project/hydro/issues/899), [#978](https://github.com/hydro-project/hydro/issues/978), [#983](https://github.com/hydro-project/hydro/issues/983)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1021](https://github.com/hydro-project/hydro/issues/1021)**
    - Push down persists and implement Pi example ([`af6e3be`](https://github.com/hydro-project/hydro/commit/af6e3be60fdb69ceec1613347910f4dd49980d34))
 * **[#899](https://github.com/hydro-project/hydro/issues/899)**
    - Add a functional surface syntax using staging ([`8b63568`](https://github.com/hydro-project/hydro/commit/8b635683e5ac3c4ed2d896ae88e2953db1c6312c))
 * **[#978](https://github.com/hydro-project/hydro/issues/978)**
    - Add initial test using Hydro CLI from Hydroflow+ ([`e5bdd12`](https://github.com/hydro-project/hydro/commit/e5bdd12e32d6ea72fd91a55c12e09f07a0edaa5c))
 * **[#983](https://github.com/hydro-project/hydro/issues/983)**
    - Support crates that have no entrypoints ([`7108323`](https://github.com/hydro-project/hydro/commit/71083233afc01e0132d7186f4af8c0b4a6323ec7))
 * **Uncategorized**
    - Release stageleft_macro v0.1.0, stageleft v0.1.0, hydroflow_plus v0.5.1 ([`1a48db5`](https://github.com/hydro-project/hydro/commit/1a48db5a1ba058a718ac777367bf6eba3a236b7c))
    - Actually committing empty CHANGELOG.md is required ([`3b36020`](https://github.com/hydro-project/hydro/commit/3b36020d16792f26da4df3c5b09652a4ab47ec4f))
</details>

