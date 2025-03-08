# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.7.0 (2025-03-08)

### Chore

 - <csr-id-49a387d4a21f0763df8ec94de73fb953c9cd333a/> upgrade to Rust 2024 edition
   - Updates `Cargo.toml` to use new shared workspace keys
   - Updates lint settings (in workspace `Cargo.toml`)
   - `rustfmt` has changed slightly, resulting in a big diff - there are no
   actual code changes
   - Adds a script to `rustfmt` the template src files

### Documentation

 - <csr-id-73444373dabeedd7a03a8231952684fb01bdf895/> add initial Rustdoc for some Stream APIs

### New Features

 - <csr-id-b968f5beccac2019b951cc5ab15891b48da01639/> allow developers to add their own re-export rewrites

### Bug Fixes

 - <csr-id-309221680dbb95bf7cbc609932dd36de731b985a/> don't emit imports when staged code is in a doctest

### Style

 - <csr-id-8b3b60812d9f561cb7f59120993fbf2e23191e2b/> fix all unexpected cfgs
   Testing in https://github.com/MingweiSamuel/hydroflow

### Bug Fixes (BREAKING)

 - <csr-id-c49a4913cfdae021404a86e5a4d0597aa4db9fbe/> reduce where `#[cfg(stageleft_runtime)]` needs to be used
   Simplifies the logic for generating the public clone of the code, which
   eliminates the need to sprinkle `#[cfg(stageleft_runtime)]` (renamed
   from `#[stageleft::runtime]`) everywhere. Also adds logic to pass
   through `cfg` attrs when re-exporting public types.

### Refactor (BREAKING)

 - <csr-id-80407a2f0fdaa8b8a81688d181166a0da8aa7b52/> rename timestamp to atomic and provide batching shortcuts

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 7 commits contributed to the release over the course of 57 calendar days.
 - 7 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 7 unique issues were worked on: [#1638](https://github.com/hydro-project/hydro/issues/1638), [#1648](https://github.com/hydro-project/hydro/issues/1648), [#1650](https://github.com/hydro-project/hydro/issues/1650), [#1681](https://github.com/hydro-project/hydro/issues/1681), [#1721](https://github.com/hydro-project/hydro/issues/1721), [#1743](https://github.com/hydro-project/hydro/issues/1743), [#1747](https://github.com/hydro-project/hydro/issues/1747)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1638](https://github.com/hydro-project/hydro/issues/1638)**
    - Allow developers to add their own re-export rewrites ([`b968f5b`](https://github.com/hydro-project/hydro/commit/b968f5beccac2019b951cc5ab15891b48da01639))
 * **[#1648](https://github.com/hydro-project/hydro/issues/1648)**
    - Fix all unexpected cfgs ([`8b3b608`](https://github.com/hydro-project/hydro/commit/8b3b60812d9f561cb7f59120993fbf2e23191e2b))
 * **[#1650](https://github.com/hydro-project/hydro/issues/1650)**
    - Add initial Rustdoc for some Stream APIs ([`7344437`](https://github.com/hydro-project/hydro/commit/73444373dabeedd7a03a8231952684fb01bdf895))
 * **[#1681](https://github.com/hydro-project/hydro/issues/1681)**
    - Rename timestamp to atomic and provide batching shortcuts ([`80407a2`](https://github.com/hydro-project/hydro/commit/80407a2f0fdaa8b8a81688d181166a0da8aa7b52))
 * **[#1721](https://github.com/hydro-project/hydro/issues/1721)**
    - Reduce where `#[cfg(stageleft_runtime)]` needs to be used ([`c49a491`](https://github.com/hydro-project/hydro/commit/c49a4913cfdae021404a86e5a4d0597aa4db9fbe))
 * **[#1743](https://github.com/hydro-project/hydro/issues/1743)**
    - Don't emit imports when staged code is in a doctest ([`3092216`](https://github.com/hydro-project/hydro/commit/309221680dbb95bf7cbc609932dd36de731b985a))
 * **[#1747](https://github.com/hydro-project/hydro/issues/1747)**
    - Upgrade to Rust 2024 edition ([`49a387d`](https://github.com/hydro-project/hydro/commit/49a387d4a21f0763df8ec94de73fb953c9cd333a))
</details>

## 0.6.0 (2024-12-23)

<csr-id-ec55910f5a41d4f08059b5feda4b96fbd058c959/>

### New Features

 - <csr-id-53da4c1c9b18562e7806adcaf3a3838f56b8ef1b/> extract initial Hydroflow+ utilities into a standard library

### Bug Fixes

 - <csr-id-f6989baf12631cf43a814123e274466740c2f159/> restrict lifetime parameters to be actually invariant
   Our lifetimes were accidentally made covariant when the lifetime `'a`
   was removed from the process/cluster tag type. This fixes that typing
   hole, and also loosens some restrictions on the lifetime of deploy
   environments.

### Refactor

 - <csr-id-ec55910f5a41d4f08059b5feda4b96fbd058c959/> generalize quorum logic

### Documentation

 - <csr-id-28cd220c68e3660d9ebade113949a2346720cd04/> add `repository` field to `Cargo.toml`s, fix #1452
   #1452 
   
   Will trigger new releases of the following:
   `unchanged = 'hydroflow_deploy_integration', 'variadics',
   'variadics_macro', 'pusherator'`
   
   (All other crates already have changes, so would be released anyway)
 - <csr-id-e1a08e5d165fbc80da2ae695e507078a97a9031f/> update `CHANGELOG.md`s for big rename
   Generated before rename per `RELEASING.md` instructions.

### New Features (BREAKING)

 - <csr-id-939389953875bf5f94ea84503a7a35efd7342282/> mark non-deterministic operators as unsafe and introduce timestamped streams
   Big PR.
   
   First big change is we introduce a `Timestamped` location. This is a bit
   of a hybrid between top-level locations and `Tick` locations. The idea
   is that you choose where timestamps are generated, and then have a
   guarantee that everything after that will be atomically computed (useful
   for making sure we add payloads to the log before ack-ing).
   
   The contract is that an operator or module that takes a `Timestamped`
   input must still be deterministic regardless of the stamps on messages
   (which are hidden unless you `tick_batch`). But unlike a top-level
   stream (which has the same constraints), you have the atomicity
   guarantee. Right now the guarantee is trivial since we have one global
   tick for everything. But in the future when we want to apply
   @davidchuyaya's optimizations this will be helpful to know when there
   are causal dependencies on when data can be sent to others.
   
   Second change is we mark every non-deterministic operator (modulo
   explicit annotations such as `NoOrder`) with Rust's `unsafe` keyword.
   This makes it super clear where non-determinism is taking place.
   
   I've used this to put `unsafe` blocks throughout our example code and
   add `SAFETY` annotations that argue why the non-determinism is safe (or
   point out that we've explicitly documented / expect non-determinism). I
   also added `#![warn(unsafe_op_in_unsafe_fn)]` to the examples and the
   template, since this forces good hygiene of annotating sources of
   non-determinism even inside a module that is intentionally
   non-deterministic.
   
   Paxos changes are mostly refactors, and I verified that the performance
   is the same as before.
 - <csr-id-a93a5e59e1681d325b3433193bb86254d23bdc77/> allow cluster self ID to be referenced as a global constant
   This eliminates the need to store `cluster.self_id()` in a local
   variable first, instead you can directly reference `CLUSTER_SELF_ID`.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 9 commits contributed to the release.
 - 45 days passed between releases.
 - 7 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 6 unique issues were worked on: [#1501](https://github.com/hydro-project/hydro/issues/1501), [#1559](https://github.com/hydro-project/hydro/issues/1559), [#1574](https://github.com/hydro-project/hydro/issues/1574), [#1583](https://github.com/hydro-project/hydro/issues/1583), [#1584](https://github.com/hydro-project/hydro/issues/1584), [#1591](https://github.com/hydro-project/hydro/issues/1591)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1501](https://github.com/hydro-project/hydro/issues/1501)**
    - Add `repository` field to `Cargo.toml`s, fix #1452 ([`28cd220`](https://github.com/hydro-project/hydro/commit/28cd220c68e3660d9ebade113949a2346720cd04))
 * **[#1559](https://github.com/hydro-project/hydro/issues/1559)**
    - Restrict lifetime parameters to be actually invariant ([`f6989ba`](https://github.com/hydro-project/hydro/commit/f6989baf12631cf43a814123e274466740c2f159))
 * **[#1574](https://github.com/hydro-project/hydro/issues/1574)**
    - Allow cluster self ID to be referenced as a global constant ([`a93a5e5`](https://github.com/hydro-project/hydro/commit/a93a5e59e1681d325b3433193bb86254d23bdc77))
 * **[#1583](https://github.com/hydro-project/hydro/issues/1583)**
    - Generalize quorum logic ([`ec55910`](https://github.com/hydro-project/hydro/commit/ec55910f5a41d4f08059b5feda4b96fbd058c959))
 * **[#1584](https://github.com/hydro-project/hydro/issues/1584)**
    - Mark non-deterministic operators as unsafe and introduce timestamped streams ([`9393899`](https://github.com/hydro-project/hydro/commit/939389953875bf5f94ea84503a7a35efd7342282))
 * **[#1591](https://github.com/hydro-project/hydro/issues/1591)**
    - Extract initial Hydroflow+ utilities into a standard library ([`53da4c1`](https://github.com/hydro-project/hydro/commit/53da4c1c9b18562e7806adcaf3a3838f56b8ef1b))
 * **Uncategorized**
    - Release stageleft_macro v0.5.0, stageleft v0.6.0, stageleft_tool v0.5.0, hydro_lang v0.11.0, hydro_std v0.11.0, hydro_cli v0.11.0 ([`b58dccc`](https://github.com/hydro-project/hydro/commit/b58dccc7f85380951a0ae91d32548eff0784f3a7))
    - Release dfir_lang v0.11.0, dfir_datalog_core v0.11.0, dfir_datalog v0.11.0, dfir_macro v0.11.0, hydroflow_deploy_integration v0.11.0, lattices_macro v0.5.8, variadics v0.0.8, variadics_macro v0.5.6, lattices v0.5.9, multiplatform_test v0.4.0, pusherator v0.0.10, dfir_rs v0.11.0, hydro_deploy v0.11.0, stageleft_macro v0.5.0, stageleft v0.6.0, stageleft_tool v0.5.0, hydro_lang v0.11.0, hydro_std v0.11.0, hydro_cli v0.11.0, safety bump 6 crates ([`9a7e486`](https://github.com/hydro-project/hydro/commit/9a7e48693fce0face0f8ad16349258cdbe26395f))
    - Update `CHANGELOG.md`s for big rename ([`e1a08e5`](https://github.com/hydro-project/hydro/commit/e1a08e5d165fbc80da2ae695e507078a97a9031f))
</details>

## v0.5.0 (2024-11-08)

<csr-id-d5677604e93c07a5392f4229af94a0b736eca382/>
<csr-id-47cb703e771f7d1c451ceb9d185ada96410949da/>

### Chore

 - <csr-id-d5677604e93c07a5392f4229af94a0b736eca382/> update pinned rust version, clippy lints, remove some dead code

### New Features

 - <csr-id-8a809315cd37929687fcabc34a12042db25d5767/> add API for external network inputs
   This is a key step towards being able to unit-test HF+ graphs, by being
   able to have controlled inputs. Outputs next.
 - <csr-id-60d9becaf0b67f9819316ce6d76bd867f7d46505/> splice UDFs with type hints to avoid inference failures

### Style

 - <csr-id-47cb703e771f7d1c451ceb9d185ada96410949da/> fixes for nightly clippy
   a couple few spurious `too_many_arguments` and a spurious
   `zombie_processes` still on current nightly (`clippy 0.1.84 (4392847410
   2024-10-21)`)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release.
 - 69 days passed between releases.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 4 unique issues were worked on: [#1434](https://github.com/hydro-project/hydro/issues/1434), [#1444](https://github.com/hydro-project/hydro/issues/1444), [#1449](https://github.com/hydro-project/hydro/issues/1449), [#1505](https://github.com/hydro-project/hydro/issues/1505)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1434](https://github.com/hydro-project/hydro/issues/1434)**
    - Splice UDFs with type hints to avoid inference failures ([`60d9bec`](https://github.com/hydro-project/hydro/commit/60d9becaf0b67f9819316ce6d76bd867f7d46505))
 * **[#1444](https://github.com/hydro-project/hydro/issues/1444)**
    - Update pinned rust version, clippy lints, remove some dead code ([`d567760`](https://github.com/hydro-project/hydro/commit/d5677604e93c07a5392f4229af94a0b736eca382))
 * **[#1449](https://github.com/hydro-project/hydro/issues/1449)**
    - Add API for external network inputs ([`8a80931`](https://github.com/hydro-project/hydro/commit/8a809315cd37929687fcabc34a12042db25d5767))
 * **[#1505](https://github.com/hydro-project/hydro/issues/1505)**
    - Fixes for nightly clippy ([`47cb703`](https://github.com/hydro-project/hydro/commit/47cb703e771f7d1c451ceb9d185ada96410949da))
 * **Uncategorized**
    - Release hydroflow_lang v0.10.0, hydroflow_datalog_core v0.10.0, hydroflow_datalog v0.10.0, hydroflow_deploy_integration v0.10.0, hydroflow_macro v0.10.0, lattices_macro v0.5.7, variadics v0.0.7, variadics_macro v0.5.5, lattices v0.5.8, multiplatform_test v0.3.0, pusherator v0.0.9, hydroflow v0.10.0, hydro_deploy v0.10.0, stageleft_macro v0.4.0, stageleft v0.5.0, stageleft_tool v0.4.0, hydroflow_plus v0.10.0, hydro_cli v0.10.0, safety bump 8 crates ([`dcd48fc`](https://github.com/hydro-project/hydro/commit/dcd48fc7ee805898d9b5ef0d082870e30615e95b))
</details>

## v0.4.0 (2024-08-30)

<csr-id-11af32828bab6e4a4264d2635ff71a12bb0bb778/>

### Chore

 - <csr-id-11af32828bab6e4a4264d2635ff71a12bb0bb778/> lower min dependency versions where possible, update `Cargo.lock`
   Moved from #1418
   
   ---------

### New Features

 - <csr-id-46a8a2cb08732bb21096e824bc4542d208c68fb2/> use trybuild to compile subgraph binaries

### Bug Fixes

 - <csr-id-06c514179bacafb52aceaed4c92367176c656822/> typing hole when splicing `RuntimeData`
   Previously, a `RuntimeData` could be spliced in any context even if its
   data type may not be valid under the required `'a` lifetime constraint.
   This fixed that typing bug.
   
   Soon, we should have regression testing for this via trybuild.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 97 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 3 unique issues were worked on: [#1397](https://github.com/hydro-project/hydro/issues/1397), [#1398](https://github.com/hydro-project/hydro/issues/1398), [#1423](https://github.com/hydro-project/hydro/issues/1423)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1397](https://github.com/hydro-project/hydro/issues/1397)**
    - Typing hole when splicing `RuntimeData` ([`06c5141`](https://github.com/hydro-project/hydro/commit/06c514179bacafb52aceaed4c92367176c656822))
 * **[#1398](https://github.com/hydro-project/hydro/issues/1398)**
    - Use trybuild to compile subgraph binaries ([`46a8a2c`](https://github.com/hydro-project/hydro/commit/46a8a2cb08732bb21096e824bc4542d208c68fb2))
 * **[#1423](https://github.com/hydro-project/hydro/issues/1423)**
    - Lower min dependency versions where possible, update `Cargo.lock` ([`11af328`](https://github.com/hydro-project/hydro/commit/11af32828bab6e4a4264d2635ff71a12bb0bb778))
 * **Uncategorized**
    - Release hydroflow_lang v0.9.0, hydroflow_datalog_core v0.9.0, hydroflow_datalog v0.9.0, hydroflow_deploy_integration v0.9.0, hydroflow_macro v0.9.0, lattices_macro v0.5.6, lattices v0.5.7, multiplatform_test v0.2.0, variadics v0.0.6, pusherator v0.0.8, hydroflow v0.9.0, stageleft_macro v0.3.0, stageleft v0.4.0, stageleft_tool v0.3.0, hydroflow_plus v0.9.0, hydro_deploy v0.9.0, hydro_cli v0.9.0, hydroflow_plus_deploy v0.9.0, safety bump 8 crates ([`0750117`](https://github.com/hydro-project/hydro/commit/0750117de7088c01a439b102adeb4c832889f171))
</details>

## v0.3.0 (2024-05-24)

### New Features

 - <csr-id-93fd05e5ff256e2e0a3b513695ff869c32344447/> re-compile staged sources for the macro at the top level

### Bug Fixes

 - <csr-id-658f6483587042d9c6df2936bc58749d30b72997/> fix missing `syn` `visit-mut` feature
 - <csr-id-0cafbdb74a665412a83aa900b4eb10c00e2498dd/> handle send_bincode with local structs
   fix(hydroflow_plus): handle send_bincode with local structs

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 48 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 3 unique issues were worked on: [#1104](https://github.com/hydro-project/hydro/issues/1104), [#1151](https://github.com/hydro-project/hydro/issues/1151), [#1225](https://github.com/hydro-project/hydro/issues/1225)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1104](https://github.com/hydro-project/hydro/issues/1104)**
    - Re-compile staged sources for the macro at the top level ([`93fd05e`](https://github.com/hydro-project/hydro/commit/93fd05e5ff256e2e0a3b513695ff869c32344447))
 * **[#1151](https://github.com/hydro-project/hydro/issues/1151)**
    - Handle send_bincode with local structs ([`0cafbdb`](https://github.com/hydro-project/hydro/commit/0cafbdb74a665412a83aa900b4eb10c00e2498dd))
 * **[#1225](https://github.com/hydro-project/hydro/issues/1225)**
    - Fix missing `syn` `visit-mut` feature ([`658f648`](https://github.com/hydro-project/hydro/commit/658f6483587042d9c6df2936bc58749d30b72997))
 * **Uncategorized**
    - Release hydroflow_lang v0.7.0, hydroflow_datalog_core v0.7.0, hydroflow_datalog v0.7.0, hydroflow_macro v0.7.0, lattices v0.5.5, multiplatform_test v0.1.0, pusherator v0.0.6, hydroflow v0.7.0, stageleft_macro v0.2.0, stageleft v0.3.0, stageleft_tool v0.2.0, hydroflow_plus v0.7.0, hydro_deploy v0.7.0, hydro_cli v0.7.0, hydroflow_plus_cli_integration v0.7.0, safety bump 8 crates ([`2852147`](https://github.com/hydro-project/hydro/commit/285214740627685e911781793e05d234ab2ad2bd))
</details>

## v0.2.1 (2024-04-05)

<csr-id-7958fb0d900be8fe7359326abfa11dcb8fb35e8a/>

### New Features

 - <csr-id-77f3e5afb9e276d1d6c643574ebac75ed0003939/> simplify lifetime bounds for processes and clusters
   feat(hydroflow_plus): simplify lifetime bounds for processes and
   clusters
   
   This allows `extract` to move the flow builder, which is a prerequisite
   for having developers run the optimizer during deployment as well in
   case it changes the network topology.

### Style

 - <csr-id-7958fb0d900be8fe7359326abfa11dcb8fb35e8a/> qualified path cleanups for clippy

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 34 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#1090](https://github.com/hydro-project/hydro/issues/1090), [#1100](https://github.com/hydro-project/hydro/issues/1100)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1090](https://github.com/hydro-project/hydro/issues/1090)**
    - Qualified path cleanups for clippy ([`7958fb0`](https://github.com/hydro-project/hydro/commit/7958fb0d900be8fe7359326abfa11dcb8fb35e8a))
 * **[#1100](https://github.com/hydro-project/hydro/issues/1100)**
    - Simplify lifetime bounds for processes and clusters ([`77f3e5a`](https://github.com/hydro-project/hydro/commit/77f3e5afb9e276d1d6c643574ebac75ed0003939))
 * **Uncategorized**
    - Release hydroflow_cli_integration v0.5.2, hydroflow_lang v0.6.1, hydroflow_datalog_core v0.6.1, lattices v0.5.4, hydroflow v0.6.1, stageleft_macro v0.1.1, stageleft v0.2.1, hydroflow_plus v0.6.1, hydro_deploy v0.6.1, hydro_cli v0.6.1, hydroflow_plus_cli_integration v0.6.1, stageleft_tool v0.1.1 ([`cd63f22`](https://github.com/hydro-project/hydro/commit/cd63f2258c961a40f0e5dbef20ac329a2d570ad0))
</details>

## v0.2.0 (2024-03-02)

### New Features

 - <csr-id-eb34ccd13f56e1d07cbae35ead79daeb3b9bad20/> use an IR before lowering to Hydroflow
   Makes it possible to write custom optimization passes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1070](https://github.com/hydro-project/hydro/issues/1070)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1070](https://github.com/hydro-project/hydro/issues/1070)**
    - Use an IR before lowering to Hydroflow ([`eb34ccd`](https://github.com/hydro-project/hydro/commit/eb34ccd13f56e1d07cbae35ead79daeb3b9bad20))
 * **Uncategorized**
    - Release hydroflow_lang v0.6.0, hydroflow_datalog_core v0.6.0, hydroflow_datalog v0.6.0, hydroflow_macro v0.6.0, lattices v0.5.3, variadics v0.0.4, pusherator v0.0.5, hydroflow v0.6.0, stageleft v0.2.0, hydroflow_plus v0.6.0, hydro_deploy v0.6.0, hydro_cli v0.6.0, hydroflow_plus_cli_integration v0.6.0, safety bump 7 crates ([`09ea65f`](https://github.com/hydro-project/hydro/commit/09ea65fe9cd45c357c43bffca30e60243fa45cc8))
</details>

## v0.1.0 (2024-01-29)

### Documentation

 - <csr-id-3b36020d16792f26da4df3c5b09652a4ab47ec4f/> actually committing empty CHANGELOG.md is required

### New Features

 - <csr-id-af6e3be60fdb69ceec1613347910f4dd49980d34/> push down persists and implement Pi example
   Also fixes type inference issues with reduce the same way as we did for fold.
 - <csr-id-174607d12277d7544d0f42890c9a5da2ff184df4/> support building graphs for symmetric clusters in Hydroflow+
 - <csr-id-71083233afc01e0132d7186f4af8c0b4a6323ec7/> support crates that have no entrypoints
   Also includes various bugfixes needed for Hydroflow+.
 - <csr-id-8b635683e5ac3c4ed2d896ae88e2953db1c6312c/> add a functional surface syntax using staging

### Bug Fixes

 - <csr-id-8df66f8c24127d8818d64d1534bb1ab4a616597f/> fix `include!` path separators on windows

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 7 commits contributed to the release over the course of 76 calendar days.
 - 6 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 5 unique issues were worked on: [#1010](https://github.com/hydro-project/hydro/issues/1010), [#1021](https://github.com/hydro-project/hydro/issues/1021), [#899](https://github.com/hydro-project/hydro/issues/899), [#983](https://github.com/hydro-project/hydro/issues/983), [#984](https://github.com/hydro-project/hydro/issues/984)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1010](https://github.com/hydro-project/hydro/issues/1010)**
    - Fix `include!` path separators on windows ([`8df66f8`](https://github.com/hydro-project/hydro/commit/8df66f8c24127d8818d64d1534bb1ab4a616597f))
 * **[#1021](https://github.com/hydro-project/hydro/issues/1021)**
    - Push down persists and implement Pi example ([`af6e3be`](https://github.com/hydro-project/hydro/commit/af6e3be60fdb69ceec1613347910f4dd49980d34))
 * **[#899](https://github.com/hydro-project/hydro/issues/899)**
    - Add a functional surface syntax using staging ([`8b63568`](https://github.com/hydro-project/hydro/commit/8b635683e5ac3c4ed2d896ae88e2953db1c6312c))
 * **[#983](https://github.com/hydro-project/hydro/issues/983)**
    - Support crates that have no entrypoints ([`7108323`](https://github.com/hydro-project/hydro/commit/71083233afc01e0132d7186f4af8c0b4a6323ec7))
 * **[#984](https://github.com/hydro-project/hydro/issues/984)**
    - Support building graphs for symmetric clusters in Hydroflow+ ([`174607d`](https://github.com/hydro-project/hydro/commit/174607d12277d7544d0f42890c9a5da2ff184df4))
 * **Uncategorized**
    - Release stageleft_macro v0.1.0, stageleft v0.1.0, hydroflow_plus v0.5.1 ([`1a48db5`](https://github.com/hydro-project/hydro/commit/1a48db5a1ba058a718ac777367bf6eba3a236b7c))
    - Actually committing empty CHANGELOG.md is required ([`3b36020`](https://github.com/hydro-project/hydro/commit/3b36020d16792f26da4df3c5b09652a4ab47ec4f))
</details>

