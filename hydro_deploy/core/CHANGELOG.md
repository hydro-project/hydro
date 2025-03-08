# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.12.0 (2025-03-08)

### Chore

 - <csr-id-49a387d4a21f0763df8ec94de73fb953c9cd333a/> upgrade to Rust 2024 edition
   - Updates `Cargo.toml` to use new shared workspace keys
   - Updates lint settings (in workspace `Cargo.toml`)
   - `rustfmt` has changed slightly, resulting in a big diff - there are no
   actual code changes
   - Adds a script to `rustfmt` the template src files
 - <csr-id-473ee4ab8c3d77356b5d3199f1612e6f710eac51/> update buildstructor

### New Features

 - <csr-id-733494ea2655cdc1460da7f902d999f0e3797411/> Link DFIR operators to Hydro operators in perf
 - <csr-id-a4adb08700fdc5fdbc949fc656e6cb309e7159a5/> provide in-memory access to perf tracing results

### Bug Fixes

 - <csr-id-02858077604330299de18b10bb261e6d25bde6cd/> always write logs to stdout
 - <csr-id-070b6e0a300bfb4ccb47d231a008bd7ce2c93a7a/> improve error message when crates fail to build
 - <csr-id-75eb323a612fd5d2609e464fe7690bc2b6a8457a/> use correct `__staged` path when rewriting `crate::` imports
   Previously, a rewrite would first turn `crate` into `crate::__staged`,
   and another would rewrite `crate::__staged` into `hydro_test::__staged`.
   The latter global rewrite is unnecessary because the stageleft logic
   already will use the full crate name when handling public types, so we
   drop it.

### Refactor

 - <csr-id-6ac0c53fa02853be5e17998f19a36d1a30641201/> show a lot more error info on build failure
   From debugging today
   
   Main useful one is the STDERR output
   
   Uses more memory
 - <csr-id-a1572f4f6e665041012769f518be43e404383081/> fix lifetime issue with command forming for Rust 2024
 - <csr-id-c293cca6855695107e9cef5c5df99fb04a571934/> enable lints, cleanups for Rust 2024 #1732

### Style

 - <csr-id-8b3b60812d9f561cb7f59120993fbf2e23191e2b/> fix all unexpected cfgs
   Testing in https://github.com/MingweiSamuel/hydroflow

### Chore (BREAKING)

 - <csr-id-44fb2806cf2d165d86695910f4755e0944c11832/> use DFIR name instead of Hydroflow in some places, fix #1644
   Fix partially #1712
   
   * Renames `WriteContextArgs.hydroflow` to `WriteContextArgs.df_ident`
   for DFIR operator codegen
   * Removes some dead code/files

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 12 commits contributed to the release over the course of 52 calendar days.
 - 12 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 12 unique issues were worked on: [#1648](https://github.com/hydro-project/hydro/issues/1648), [#1657](https://github.com/hydro-project/hydro/issues/1657), [#1691](https://github.com/hydro-project/hydro/issues/1691), [#1700](https://github.com/hydro-project/hydro/issues/1700), [#1713](https://github.com/hydro-project/hydro/issues/1713), [#1719](https://github.com/hydro-project/hydro/issues/1719), [#1720](https://github.com/hydro-project/hydro/issues/1720), [#1723](https://github.com/hydro-project/hydro/issues/1723), [#1737](https://github.com/hydro-project/hydro/issues/1737), [#1744](https://github.com/hydro-project/hydro/issues/1744), [#1747](https://github.com/hydro-project/hydro/issues/1747), [#1752](https://github.com/hydro-project/hydro/issues/1752)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1648](https://github.com/hydro-project/hydro/issues/1648)**
    - Fix all unexpected cfgs ([`8b3b608`](https://github.com/hydro-project/hydro/commit/8b3b60812d9f561cb7f59120993fbf2e23191e2b))
 * **[#1657](https://github.com/hydro-project/hydro/issues/1657)**
    - Use correct `__staged` path when rewriting `crate::` imports ([`75eb323`](https://github.com/hydro-project/hydro/commit/75eb323a612fd5d2609e464fe7690bc2b6a8457a))
 * **[#1691](https://github.com/hydro-project/hydro/issues/1691)**
    - Improve error message when crates fail to build ([`070b6e0`](https://github.com/hydro-project/hydro/commit/070b6e0a300bfb4ccb47d231a008bd7ce2c93a7a))
 * **[#1700](https://github.com/hydro-project/hydro/issues/1700)**
    - Always write logs to stdout ([`0285807`](https://github.com/hydro-project/hydro/commit/02858077604330299de18b10bb261e6d25bde6cd))
 * **[#1713](https://github.com/hydro-project/hydro/issues/1713)**
    - Use DFIR name instead of Hydroflow in some places, fix #1644 ([`44fb280`](https://github.com/hydro-project/hydro/commit/44fb2806cf2d165d86695910f4755e0944c11832))
 * **[#1719](https://github.com/hydro-project/hydro/issues/1719)**
    - Provide in-memory access to perf tracing results ([`a4adb08`](https://github.com/hydro-project/hydro/commit/a4adb08700fdc5fdbc949fc656e6cb309e7159a5))
 * **[#1720](https://github.com/hydro-project/hydro/issues/1720)**
    - Update buildstructor ([`473ee4a`](https://github.com/hydro-project/hydro/commit/473ee4ab8c3d77356b5d3199f1612e6f710eac51))
 * **[#1723](https://github.com/hydro-project/hydro/issues/1723)**
    - Link DFIR operators to Hydro operators in perf ([`733494e`](https://github.com/hydro-project/hydro/commit/733494ea2655cdc1460da7f902d999f0e3797411))
 * **[#1737](https://github.com/hydro-project/hydro/issues/1737)**
    - Enable lints, cleanups for Rust 2024 #1732 ([`c293cca`](https://github.com/hydro-project/hydro/commit/c293cca6855695107e9cef5c5df99fb04a571934))
 * **[#1744](https://github.com/hydro-project/hydro/issues/1744)**
    - Fix lifetime issue with command forming for Rust 2024 ([`a1572f4`](https://github.com/hydro-project/hydro/commit/a1572f4f6e665041012769f518be43e404383081))
 * **[#1747](https://github.com/hydro-project/hydro/issues/1747)**
    - Upgrade to Rust 2024 edition ([`49a387d`](https://github.com/hydro-project/hydro/commit/49a387d4a21f0763df8ec94de73fb953c9cd333a))
 * **[#1752](https://github.com/hydro-project/hydro/issues/1752)**
    - Show a lot more error info on build failure ([`6ac0c53`](https://github.com/hydro-project/hydro/commit/6ac0c53fa02853be5e17998f19a36d1a30641201))
</details>

## 0.11.0 (2024-12-23)

### Documentation

 - <csr-id-28cd220c68e3660d9ebade113949a2346720cd04/> add `repository` field to `Cargo.toml`s, fix #1452
   #1452 
   
   Will trigger new releases of the following:
   `unchanged = 'hydroflow_deploy_integration', 'variadics',
   'variadics_macro', 'pusherator'`
   
   (All other crates already have changes, so would be released anyway)
 - <csr-id-e1a08e5d165fbc80da2ae695e507078a97a9031f/> update `CHANGELOG.md`s for big rename
   Generated before rename per `RELEASING.md` instructions.

### New Features

 - <csr-id-8d550b94ae2c08486e1c2222d37e3ca8b5f018b7/> use regular println when no tasks are active
   Significantly improves the appearance of Hydroflow+ logs when the
   terminal causes wrapping.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 45 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#1501](https://github.com/hydro-project/hydro/issues/1501), [#1577](https://github.com/hydro-project/hydro/issues/1577)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1501](https://github.com/hydro-project/hydro/issues/1501)**
    - Add `repository` field to `Cargo.toml`s, fix #1452 ([`28cd220`](https://github.com/hydro-project/hydro/commit/28cd220c68e3660d9ebade113949a2346720cd04))
 * **[#1577](https://github.com/hydro-project/hydro/issues/1577)**
    - Use regular println when no tasks are active ([`8d550b9`](https://github.com/hydro-project/hydro/commit/8d550b94ae2c08486e1c2222d37e3ca8b5f018b7))
 * **Uncategorized**
    - Release dfir_lang v0.11.0, dfir_datalog_core v0.11.0, dfir_datalog v0.11.0, dfir_macro v0.11.0, hydroflow_deploy_integration v0.11.0, lattices_macro v0.5.8, variadics v0.0.8, variadics_macro v0.5.6, lattices v0.5.9, multiplatform_test v0.4.0, pusherator v0.0.10, dfir_rs v0.11.0, hydro_deploy v0.11.0, stageleft_macro v0.5.0, stageleft v0.6.0, stageleft_tool v0.5.0, hydro_lang v0.11.0, hydro_std v0.11.0, hydro_cli v0.11.0, safety bump 6 crates ([`9a7e486`](https://github.com/hydro-project/hydro/commit/9a7e48693fce0face0f8ad16349258cdbe26395f))
    - Update `CHANGELOG.md`s for big rename ([`e1a08e5`](https://github.com/hydro-project/hydro/commit/e1a08e5d165fbc80da2ae695e507078a97a9031f))
</details>

## v0.10.0 (2024-11-08)

<csr-id-d5677604e93c07a5392f4229af94a0b736eca382/>
<csr-id-8442d1b524621a9f8b43372a9c25991efb33c25e/>

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
 - <csr-id-8a809315cd37929687fcabc34a12042db25d5767/> add API for external network inputs
   This is a key step towards being able to unit-test HF+ graphs, by being
   able to have controlled inputs. Outputs next.

### Style

 - <csr-id-8442d1b524621a9f8b43372a9c25991efb33c25e/> fixes for latest nightly clippy

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release.
 - 69 days passed between releases.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 4 unique issues were worked on: [#1444](https://github.com/hydro-project/hydro/issues/1444), [#1449](https://github.com/hydro-project/hydro/issues/1449), [#1450](https://github.com/hydro-project/hydro/issues/1450), [#1537](https://github.com/hydro-project/hydro/issues/1537)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1444](https://github.com/hydro-project/hydro/issues/1444)**
    - Update pinned rust version, clippy lints, remove some dead code ([`d567760`](https://github.com/hydro-project/hydro/commit/d5677604e93c07a5392f4229af94a0b736eca382))
 * **[#1449](https://github.com/hydro-project/hydro/issues/1449)**
    - Add API for external network inputs ([`8a80931`](https://github.com/hydro-project/hydro/commit/8a809315cd37929687fcabc34a12042db25d5767))
 * **[#1450](https://github.com/hydro-project/hydro/issues/1450)**
    - Add ability to have staged flows inside unit tests ([`afe78c3`](https://github.com/hydro-project/hydro/commit/afe78c343658472513b34d28658634b253148aee))
 * **[#1537](https://github.com/hydro-project/hydro/issues/1537)**
    - Fixes for latest nightly clippy ([`8442d1b`](https://github.com/hydro-project/hydro/commit/8442d1b524621a9f8b43372a9c25991efb33c25e))
 * **Uncategorized**
    - Release hydroflow_lang v0.10.0, hydroflow_datalog_core v0.10.0, hydroflow_datalog v0.10.0, hydroflow_deploy_integration v0.10.0, hydroflow_macro v0.10.0, lattices_macro v0.5.7, variadics v0.0.7, variadics_macro v0.5.5, lattices v0.5.8, multiplatform_test v0.3.0, pusherator v0.0.9, hydroflow v0.10.0, hydro_deploy v0.10.0, stageleft_macro v0.4.0, stageleft v0.5.0, stageleft_tool v0.4.0, hydroflow_plus v0.10.0, hydro_cli v0.10.0, safety bump 8 crates ([`dcd48fc`](https://github.com/hydro-project/hydro/commit/dcd48fc7ee805898d9b5ef0d082870e30615e95b))
</details>

## v0.9.0 (2024-08-30)

<csr-id-a2ec110ccadb97e293b19d83a155d98d94224bba/>
<csr-id-11af32828bab6e4a4264d2635ff71a12bb0bb778/>
<csr-id-a88a550cefde3a56790859127edc6a4e27e07090/>
<csr-id-77246e77df47a0006dcb3eaeeb76882efacfd25c/>
<csr-id-3fde68d0db0414017cfb771a218b14b8f57d1686/>
<csr-id-0a465e55dd39c76bc1aefb020460a639d792fe87/>
<csr-id-bb081d3b0af6dbce9630e23dfe8b7d1363751c2b/>
<csr-id-a2147864b24110c9ae2c1553e9e8b55bd5065f15/>
<csr-id-8856c8596d5ad9d5f24a46467690bfac1549fae2/>

### Chore

 - <csr-id-a2ec110ccadb97e293b19d83a155d98d94224bba/> manually set versions for crates renamed in #1413
 - <csr-id-11af32828bab6e4a4264d2635ff71a12bb0bb778/> lower min dependency versions where possible, update `Cargo.lock`
   Moved from #1418
   
   ---------

### Refactor (BREAKING)

 - <csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

### Refactor (BREAKING)

 - <csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

### Refactor (BREAKING)

 - <csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

### Refactor (BREAKING)

 - <csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

### Documentation

 - <csr-id-f5f1eb0c612f5c0c1752360d972ef6853c5e12f0/> cleanup doc comments for clippy latest

### New Features

 - <csr-id-6568263e03899d4e96837690e6e59284c194d7ff/> Add end-to-end flamegraph generation for macos and linux localhost, fix #1351
 - <csr-id-fedd3ef60fe16ab862244d816f7973269a7295e8/> improve progress UX by collapsing nested groups
   Now, when a group only has a single active task, we skip printing a line
   for the group itself and instead collapse its information into the line
   for the inner task (recursively as necessary). This allows us to show
   more fine grained progress without overflowing the console.
 - <csr-id-46a8a2cb08732bb21096e824bc4542d208c68fb2/> use trybuild to compile subgraph binaries

### Bug Fixes

 - <csr-id-c4683caca43f2927694c920b43ef35a6d1629eaa/> only record usermode events in perf
   When kernel stacks are included, the DWARF traces can become corrupted /
   overflown leading to flamegraphs with broken parents. We only are
   interested in usermode, anyways, and can measure I/O overhead through
   other methods.
 - <csr-id-63b528feeb2e6dac2ed12c02b2e39e0d42133a74/> only instantiate `Localhost` once
 - <csr-id-654b77d8f65ae6eb62c164a2d736168ff96cb168/> avoid Terraform crashing on empty provider block

### Refactor

 - <csr-id-a88a550cefde3a56790859127edc6a4e27e07090/> adjust `ProgressTracker::println`
   A small refactor pulled out of the perf tracing work, barely related to
   #1359
 - <csr-id-77246e77df47a0006dcb3eaeeb76882efacfd25c/> cleanup handling of arc `Weak` in `deployment.rs`

### Style

 - <csr-id-3fde68d0db0414017cfb771a218b14b8f57d1686/> use `name_of!` macro

### New Features (BREAKING)

 - <csr-id-749a10307f4eff2a46a1056735e84ed94d44b39e/> Perf works over SSH
   See documentation on how to use in
   [Notion](https://www.notion.so/hydro-project/perf-Measuring-CPU-usage-6135b6ce56a94af38eeeba0a55deef9c).

### Refactor (BREAKING)

 - <csr-id-0a465e55dd39c76bc1aefb020460a639d792fe87/> rename integration crates to drop CLI references
 - <csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-bb081d3b0af6dbce9630e23dfe8b7d1363751c2b/> end-to-end flamegraph generation, fix #1365
   Depends on #1370
 - <csr-id-a2147864b24110c9ae2c1553e9e8b55bd5065f15/> `Deployment.stop()` for graceful shutdown including updated `perf` profile downloading
   * `perf` profile downloading moved from the `drop()` impl to `async fn
   stop()`
   * download perf data via stdout
   * update async-ssh2-lite to 0.5 to cleanup tokio compat issues
   
   WIP for #1365
 - <csr-id-8856c8596d5ad9d5f24a46467690bfac1549fae2/> use `buildstructor` to handle excessive `Deployment` method arguments, fix #1364
   Adds new method `Deployment::AzureHost`

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-bb081d3b0af6dbce9630e23dfe8b7d1363751c2b/> end-to-end flamegraph generation, fix #1365
   Depends on #1370
 - <csr-id-a2147864b24110c9ae2c1553e9e8b55bd5065f15/> `Deployment.stop()` for graceful shutdown including updated `perf` profile downloading
   * `perf` profile downloading moved from the `drop()` impl to `async fn
   stop()`
   * download perf data via stdout
   * update async-ssh2-lite to 0.5 to cleanup tokio compat issues
   
   WIP for #1365
 - <csr-id-8856c8596d5ad9d5f24a46467690bfac1549fae2/> use `buildstructor` to handle excessive `Deployment` method arguments, fix #1364
   Adds new method `Deployment::AzureHost`

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-bb081d3b0af6dbce9630e23dfe8b7d1363751c2b/> end-to-end flamegraph generation, fix #1365
   Depends on #1370
 - <csr-id-a2147864b24110c9ae2c1553e9e8b55bd5065f15/> `Deployment.stop()` for graceful shutdown including updated `perf` profile downloading
   * `perf` profile downloading moved from the `drop()` impl to `async fn
   stop()`
   * download perf data via stdout
   * update async-ssh2-lite to 0.5 to cleanup tokio compat issues
   
   WIP for #1365
 - <csr-id-8856c8596d5ad9d5f24a46467690bfac1549fae2/> use `buildstructor` to handle excessive `Deployment` method arguments, fix #1364
   Adds new method `Deployment::AzureHost`

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-bb081d3b0af6dbce9630e23dfe8b7d1363751c2b/> end-to-end flamegraph generation, fix #1365
   Depends on #1370
 - <csr-id-a2147864b24110c9ae2c1553e9e8b55bd5065f15/> `Deployment.stop()` for graceful shutdown including updated `perf` profile downloading
   * `perf` profile downloading moved from the `drop()` impl to `async fn
   stop()`
   * download perf data via stdout
   * update async-ssh2-lite to 0.5 to cleanup tokio compat issues
   
   WIP for #1365
 - <csr-id-8856c8596d5ad9d5f24a46467690bfac1549fae2/> use `buildstructor` to handle excessive `Deployment` method arguments, fix #1364
   Adds new method `Deployment::AzureHost`

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-bb081d3b0af6dbce9630e23dfe8b7d1363751c2b/> end-to-end flamegraph generation, fix #1365
   Depends on #1370
 - <csr-id-a2147864b24110c9ae2c1553e9e8b55bd5065f15/> `Deployment.stop()` for graceful shutdown including updated `perf` profile downloading
   * `perf` profile downloading moved from the `drop()` impl to `async fn
   stop()`
   * download perf data via stdout
   * update async-ssh2-lite to 0.5 to cleanup tokio compat issues
   
   WIP for #1365
 - <csr-id-8856c8596d5ad9d5f24a46467690bfac1549fae2/> use `buildstructor` to handle excessive `Deployment` method arguments, fix #1364
   Adds new method `Deployment::AzureHost`

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 20 commits contributed to the release.
 - 38 days passed between releases.
 - 18 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 17 unique issues were worked on: [#1313](https://github.com/hydro-project/hydro/issues/1313), [#1360](https://github.com/hydro-project/hydro/issues/1360), [#1366](https://github.com/hydro-project/hydro/issues/1366), [#1369](https://github.com/hydro-project/hydro/issues/1369), [#1370](https://github.com/hydro-project/hydro/issues/1370), [#1372](https://github.com/hydro-project/hydro/issues/1372), [#1378](https://github.com/hydro-project/hydro/issues/1378), [#1394](https://github.com/hydro-project/hydro/issues/1394), [#1396](https://github.com/hydro-project/hydro/issues/1396), [#1398](https://github.com/hydro-project/hydro/issues/1398), [#1403](https://github.com/hydro-project/hydro/issues/1403), [#1411](https://github.com/hydro-project/hydro/issues/1411), [#1413](https://github.com/hydro-project/hydro/issues/1413), [#1423](https://github.com/hydro-project/hydro/issues/1423), [#1428](https://github.com/hydro-project/hydro/issues/1428), [#1429](https://github.com/hydro-project/hydro/issues/1429), [#1431](https://github.com/hydro-project/hydro/issues/1431)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1313](https://github.com/hydro-project/hydro/issues/1313)**
    - Fixup! feat(hydro_deploy)!: Perf works over SSH ([`220b5bc`](https://github.com/hydro-project/hydro/commit/220b5bce4fbf1af5e14ebe5aa73da7a7e668fea7))
    - Perf works over SSH ([`749a103`](https://github.com/hydro-project/hydro/commit/749a10307f4eff2a46a1056735e84ed94d44b39e))
 * **[#1360](https://github.com/hydro-project/hydro/issues/1360)**
    - Avoid Terraform crashing on empty provider block ([`654b77d`](https://github.com/hydro-project/hydro/commit/654b77d8f65ae6eb62c164a2d736168ff96cb168))
 * **[#1366](https://github.com/hydro-project/hydro/issues/1366)**
    - Use `buildstructor` to handle excessive `Deployment` method arguments, fix #1364 ([`8856c85`](https://github.com/hydro-project/hydro/commit/8856c8596d5ad9d5f24a46467690bfac1549fae2))
 * **[#1369](https://github.com/hydro-project/hydro/issues/1369)**
    - Cleanup handling of arc `Weak` in `deployment.rs` ([`77246e7`](https://github.com/hydro-project/hydro/commit/77246e77df47a0006dcb3eaeeb76882efacfd25c))
 * **[#1370](https://github.com/hydro-project/hydro/issues/1370)**
    - `Deployment.stop()` for graceful shutdown including updated `perf` profile downloading ([`a214786`](https://github.com/hydro-project/hydro/commit/a2147864b24110c9ae2c1553e9e8b55bd5065f15))
 * **[#1372](https://github.com/hydro-project/hydro/issues/1372)**
    - End-to-end flamegraph generation, fix #1365 ([`bb081d3`](https://github.com/hydro-project/hydro/commit/bb081d3b0af6dbce9630e23dfe8b7d1363751c2b))
 * **[#1378](https://github.com/hydro-project/hydro/issues/1378)**
    - Adjust `ProgressTracker::println` ([`a88a550`](https://github.com/hydro-project/hydro/commit/a88a550cefde3a56790859127edc6a4e27e07090))
 * **[#1394](https://github.com/hydro-project/hydro/issues/1394)**
    - Simplify process/cluster specs ([`128aaec`](https://github.com/hydro-project/hydro/commit/128aaecd40edce57dc254afdcd61ecd5b9948d71))
 * **[#1396](https://github.com/hydro-project/hydro/issues/1396)**
    - Add end-to-end flamegraph generation for macos and linux localhost, fix #1351 ([`6568263`](https://github.com/hydro-project/hydro/commit/6568263e03899d4e96837690e6e59284c194d7ff))
 * **[#1398](https://github.com/hydro-project/hydro/issues/1398)**
    - Use trybuild to compile subgraph binaries ([`46a8a2c`](https://github.com/hydro-project/hydro/commit/46a8a2cb08732bb21096e824bc4542d208c68fb2))
 * **[#1403](https://github.com/hydro-project/hydro/issues/1403)**
    - Only instantiate `Localhost` once ([`63b528f`](https://github.com/hydro-project/hydro/commit/63b528feeb2e6dac2ed12c02b2e39e0d42133a74))
 * **[#1411](https://github.com/hydro-project/hydro/issues/1411)**
    - Improve progress UX by collapsing nested groups ([`fedd3ef`](https://github.com/hydro-project/hydro/commit/fedd3ef60fe16ab862244d816f7973269a7295e8))
 * **[#1413](https://github.com/hydro-project/hydro/issues/1413)**
    - Rename integration crates to drop CLI references ([`0a465e5`](https://github.com/hydro-project/hydro/commit/0a465e55dd39c76bc1aefb020460a639d792fe87))
 * **[#1423](https://github.com/hydro-project/hydro/issues/1423)**
    - Lower min dependency versions where possible, update `Cargo.lock` ([`11af328`](https://github.com/hydro-project/hydro/commit/11af32828bab6e4a4264d2635ff71a12bb0bb778))
 * **[#1428](https://github.com/hydro-project/hydro/issues/1428)**
    - Cleanup doc comments for clippy latest ([`f5f1eb0`](https://github.com/hydro-project/hydro/commit/f5f1eb0c612f5c0c1752360d972ef6853c5e12f0))
 * **[#1429](https://github.com/hydro-project/hydro/issues/1429)**
    - Use `name_of!` macro ([`3fde68d`](https://github.com/hydro-project/hydro/commit/3fde68d0db0414017cfb771a218b14b8f57d1686))
 * **[#1431](https://github.com/hydro-project/hydro/issues/1431)**
    - Only record usermode events in perf ([`c4683ca`](https://github.com/hydro-project/hydro/commit/c4683caca43f2927694c920b43ef35a6d1629eaa))
 * **Uncategorized**
    - Release hydroflow_lang v0.9.0, hydroflow_datalog_core v0.9.0, hydroflow_datalog v0.9.0, hydroflow_deploy_integration v0.9.0, hydroflow_macro v0.9.0, lattices_macro v0.5.6, lattices v0.5.7, multiplatform_test v0.2.0, variadics v0.0.6, pusherator v0.0.8, hydroflow v0.9.0, stageleft_macro v0.3.0, stageleft v0.4.0, stageleft_tool v0.3.0, hydroflow_plus v0.9.0, hydro_deploy v0.9.0, hydro_cli v0.9.0, hydroflow_plus_deploy v0.9.0, safety bump 8 crates ([`0750117`](https://github.com/hydro-project/hydro/commit/0750117de7088c01a439b102adeb4c832889f171))
    - Manually set versions for crates renamed in #1413 ([`a2ec110`](https://github.com/hydro-project/hydro/commit/a2ec110ccadb97e293b19d83a155d98d94224bba))
</details>

## v0.8.0 (2024-07-23)

<csr-id-e3e69334fcba8488b6fad3975fb0ba88e82a4b02/>
<csr-id-0feae7454e4674eea1f3308b3d6d4e9d459cda67/>
<csr-id-947ebc1cb21a07fbfacae4ac956dbd0015a8a418/>
<csr-id-22865583a4260fe401c28aa39a74987478edc73d/>
<csr-id-c5a8de28e7844b3c29d58116d8340967f2e6bcc4/>
<csr-id-f536eccf7297be8185108b60897e92ad0efffe4a/>
<csr-id-057a0a510568cf81932368c8c65e056f91af7202/>
<csr-id-60390782dd7dcec18d193c800af716843a944dba/>
<csr-id-141eae1c3a1869fa42756250618a21ea2a2c7e34/>
<csr-id-12b8ba53f28eb9de1318b41cdf1e23282f6f0eb6/>

### Refactor

 - <csr-id-e3e69334fcba8488b6fad3975fb0ba88e82a4b02/> remove unneeded `Arc<RwLock<` wrapping of `launch_binary` return value (1/3)
   > Curious if there was any intention behind why it was `Arc<RwLock<`?
   
   > I think before some refactors we took the I/O handles instead of using broadcast channels.
 - <csr-id-0feae7454e4674eea1f3308b3d6d4e9d459cda67/> build cache cleanup
   * Replace mystery tuple with new `struct BuildOutput`
   * Replace `Mutex` and `Arc`-infested `HashMap` with `memo-map` crate,
   greatly simplifying build cache typing
   * Remove redundant build caching in `HydroflowCrateService`, expose and
   use cache parameters as `BuildParams`
   * Remove `once_cell` and `async-once-cell` dependencies, use `std`'s
   `OnceLock`
   * Add `Failed to execute command: {}` context to `perf` error message
   * Cleanup some repeated `format!` expressions

### Style

 - <csr-id-947ebc1cb21a07fbfacae4ac956dbd0015a8a418/> rename `SSH` -> `Ssh`

### Refactor (BREAKING)

 - <csr-id-22865583a4260fe401c28aa39a74987478edc73d/> make `Service::collect_resources` take `&self` instead of `&mut self`
   #430 but still has `RwLock` wrapping
   
   Depends on #1347
 - <csr-id-c5a8de28e7844b3c29d58116d8340967f2e6bcc4/> make `Host` trait use `&self` interior mutability to remove `RwLock` wrappings #430
   Depends on #1346
 - <csr-id-f536eccf7297be8185108b60897e92ad0efffe4a/> Make `Host::provision` not async anymore
   I noticed that none of the method impls have any `await`s
 - <csr-id-057a0a510568cf81932368c8c65e056f91af7202/> make `HydroflowSource`, `HydroflowSink` traits use `&self` interior mutability to remove `RwLock` wrappings #430
   Depends on #1339
 - <csr-id-60390782dd7dcec18d193c800af716843a944dba/> replace `async-channel` with `tokio::sync::mpsc::unbounded_channel`
   Depends on #1339
   
   We could make the publicly facing `stdout`, `stderr` APIs return `impl Stream<Output = String>` in the future, maybe
 - <csr-id-141eae1c3a1869fa42756250618a21ea2a2c7e34/> replace some uses of `tokio::sync::RwLock` with `std::sync::Mutex` #430 (3/3)

### Style (BREAKING)

 - <csr-id-12b8ba53f28eb9de1318b41cdf1e23282f6f0eb6/> enable clippy `upper-case-acronyms-aggressive`
   * rename `GCP` -> `Gcp`, `NodeID` -> `NodeId`
   * update CI `cargo-generate` template testing to use PR's branch instead
   of whatever `main` happens to be

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 11 commits contributed to the release.
 - 59 days passed between releases.
 - 10 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 10 unique issues were worked on: [#1334](https://github.com/hydro-project/hydro/issues/1334), [#1338](https://github.com/hydro-project/hydro/issues/1338), [#1339](https://github.com/hydro-project/hydro/issues/1339), [#1340](https://github.com/hydro-project/hydro/issues/1340), [#1343](https://github.com/hydro-project/hydro/issues/1343), [#1345](https://github.com/hydro-project/hydro/issues/1345), [#1346](https://github.com/hydro-project/hydro/issues/1346), [#1347](https://github.com/hydro-project/hydro/issues/1347), [#1348](https://github.com/hydro-project/hydro/issues/1348), [#1356](https://github.com/hydro-project/hydro/issues/1356)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1334](https://github.com/hydro-project/hydro/issues/1334)**
    - Build cache cleanup ([`0feae74`](https://github.com/hydro-project/hydro/commit/0feae7454e4674eea1f3308b3d6d4e9d459cda67))
 * **[#1338](https://github.com/hydro-project/hydro/issues/1338)**
    - Remove unneeded `Arc<RwLock<` wrapping of `launch_binary` return value (1/3) ([`e3e6933`](https://github.com/hydro-project/hydro/commit/e3e69334fcba8488b6fad3975fb0ba88e82a4b02))
 * **[#1339](https://github.com/hydro-project/hydro/issues/1339)**
    - Replace some uses of `tokio::sync::RwLock` with `std::sync::Mutex` #430 (3/3) ([`141eae1`](https://github.com/hydro-project/hydro/commit/141eae1c3a1869fa42756250618a21ea2a2c7e34))
 * **[#1340](https://github.com/hydro-project/hydro/issues/1340)**
    - Rename `SSH` -> `Ssh` ([`947ebc1`](https://github.com/hydro-project/hydro/commit/947ebc1cb21a07fbfacae4ac956dbd0015a8a418))
 * **[#1343](https://github.com/hydro-project/hydro/issues/1343)**
    - Make `Host::provision` not async anymore ([`f536ecc`](https://github.com/hydro-project/hydro/commit/f536eccf7297be8185108b60897e92ad0efffe4a))
 * **[#1345](https://github.com/hydro-project/hydro/issues/1345)**
    - Enable clippy `upper-case-acronyms-aggressive` ([`12b8ba5`](https://github.com/hydro-project/hydro/commit/12b8ba53f28eb9de1318b41cdf1e23282f6f0eb6))
 * **[#1346](https://github.com/hydro-project/hydro/issues/1346)**
    - Make `HydroflowSource`, `HydroflowSink` traits use `&self` interior mutability to remove `RwLock` wrappings #430 ([`057a0a5`](https://github.com/hydro-project/hydro/commit/057a0a510568cf81932368c8c65e056f91af7202))
 * **[#1347](https://github.com/hydro-project/hydro/issues/1347)**
    - Make `Host` trait use `&self` interior mutability to remove `RwLock` wrappings #430 ([`c5a8de2`](https://github.com/hydro-project/hydro/commit/c5a8de28e7844b3c29d58116d8340967f2e6bcc4))
 * **[#1348](https://github.com/hydro-project/hydro/issues/1348)**
    - Make `Service::collect_resources` take `&self` instead of `&mut self` ([`2286558`](https://github.com/hydro-project/hydro/commit/22865583a4260fe401c28aa39a74987478edc73d))
 * **[#1356](https://github.com/hydro-project/hydro/issues/1356)**
    - Replace `async-channel` with `tokio::sync::mpsc::unbounded_channel` ([`6039078`](https://github.com/hydro-project/hydro/commit/60390782dd7dcec18d193c800af716843a944dba))
 * **Uncategorized**
    - Release hydroflow_lang v0.8.0, hydroflow_datalog_core v0.8.0, hydroflow_datalog v0.8.0, hydroflow_macro v0.8.0, lattices_macro v0.5.5, lattices v0.5.6, variadics v0.0.5, pusherator v0.0.7, hydroflow v0.8.0, hydroflow_plus v0.8.0, hydro_deploy v0.8.0, hydro_cli v0.8.0, hydroflow_plus_cli_integration v0.8.0, safety bump 7 crates ([`ca6c16b`](https://github.com/hydro-project/hydro/commit/ca6c16b4a7ce35e155fe7fc6c7d1676c37c9e4de))
</details>

## v0.7.0 (2024-05-24)

### New Features

 - <csr-id-29a263fb564c5ce4bc495ea4e9d20b8b2621b645/> add support for collecting counts and running perf

### Bug Fixes

 - <csr-id-92c72ba9527241f88dfb23f64b999c8e4bd2b26c/> end processes with SIGTERM instead of SIGKILL
   fix(hydro_deploy): end processes with SIGTERM instead of SIGKILL

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 44 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#1129](https://github.com/hydro-project/hydro/issues/1129), [#1157](https://github.com/hydro-project/hydro/issues/1157)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1129](https://github.com/hydro-project/hydro/issues/1129)**
    - End processes with SIGTERM instead of SIGKILL ([`92c72ba`](https://github.com/hydro-project/hydro/commit/92c72ba9527241f88dfb23f64b999c8e4bd2b26c))
 * **[#1157](https://github.com/hydro-project/hydro/issues/1157)**
    - Add support for collecting counts and running perf ([`29a263f`](https://github.com/hydro-project/hydro/commit/29a263fb564c5ce4bc495ea4e9d20b8b2621b645))
 * **Uncategorized**
    - Release hydroflow_lang v0.7.0, hydroflow_datalog_core v0.7.0, hydroflow_datalog v0.7.0, hydroflow_macro v0.7.0, lattices v0.5.5, multiplatform_test v0.1.0, pusherator v0.0.6, hydroflow v0.7.0, stageleft_macro v0.2.0, stageleft v0.3.0, stageleft_tool v0.2.0, hydroflow_plus v0.7.0, hydro_deploy v0.7.0, hydro_cli v0.7.0, hydroflow_plus_cli_integration v0.7.0, safety bump 8 crates ([`2852147`](https://github.com/hydro-project/hydro/commit/285214740627685e911781793e05d234ab2ad2bd))
</details>

## v0.6.1 (2024-04-09)

<csr-id-7958fb0d900be8fe7359326abfa11dcb8fb35e8a/>

### Style

 - <csr-id-7958fb0d900be8fe7359326abfa11dcb8fb35e8a/> qualified path cleanups for clippy

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 38 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1090](https://github.com/hydro-project/hydro/issues/1090)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1090](https://github.com/hydro-project/hydro/issues/1090)**
    - Qualified path cleanups for clippy ([`7958fb0`](https://github.com/hydro-project/hydro/commit/7958fb0d900be8fe7359326abfa11dcb8fb35e8a))
 * **Uncategorized**
    - Release hydroflow_plus v0.6.1, hydro_deploy v0.6.1, hydro_cli v0.6.1, hydroflow_plus_cli_integration v0.6.1 ([`c385c13`](https://github.com/hydro-project/hydro/commit/c385c132c9733d1bace82156aa14216b8e7fef9f))
    - Release hydroflow_lang v0.6.2, hydroflow v0.6.2, hydroflow_plus v0.6.1, hydro_deploy v0.6.1, hydro_cli v0.6.1, hydroflow_plus_cli_integration v0.6.1, stageleft_tool v0.1.1 ([`23cfe08`](https://github.com/hydro-project/hydro/commit/23cfe0839079aa17d042bbd3976f6d188689d290))
    - Release hydroflow_cli_integration v0.5.2, hydroflow_lang v0.6.1, hydroflow_datalog_core v0.6.1, lattices v0.5.4, hydroflow v0.6.1, stageleft_macro v0.1.1, stageleft v0.2.1, hydroflow_plus v0.6.1, hydro_deploy v0.6.1, hydro_cli v0.6.1, hydroflow_plus_cli_integration v0.6.1, stageleft_tool v0.1.1 ([`cd63f22`](https://github.com/hydro-project/hydro/commit/cd63f2258c961a40f0e5dbef20ac329a2d570ad0))
</details>

## v0.6.0 (2024-03-02)

<csr-id-39ab8b0278e9e3fe96552ace0a4ae768a6bc10d8/>
<csr-id-e9639f608f8dafd3f384837067800a66951b25df/>

### Chore

 - <csr-id-39ab8b0278e9e3fe96552ace0a4ae768a6bc10d8/> appease various clippy lints

### New Features

 - <csr-id-fcf43bf86fe550247dffa4641a9ce3aff3b9afc3/> Add support for azure
   I accidentally committed some large files, so you won't see the commit
   history because I copied over the changes onto a fresh clone.

### Other

 - <csr-id-e9639f608f8dafd3f384837067800a66951b25df/> consolidate tasks and use sccache and nextest

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 3 unique issues were worked on: [#1015](https://github.com/hydro-project/hydro/issues/1015), [#1043](https://github.com/hydro-project/hydro/issues/1043), [#1084](https://github.com/hydro-project/hydro/issues/1084)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1015](https://github.com/hydro-project/hydro/issues/1015)**
    - Consolidate tasks and use sccache and nextest ([`e9639f6`](https://github.com/hydro-project/hydro/commit/e9639f608f8dafd3f384837067800a66951b25df))
 * **[#1043](https://github.com/hydro-project/hydro/issues/1043)**
    - Add support for azure ([`fcf43bf`](https://github.com/hydro-project/hydro/commit/fcf43bf86fe550247dffa4641a9ce3aff3b9afc3))
 * **[#1084](https://github.com/hydro-project/hydro/issues/1084)**
    - Appease various clippy lints ([`39ab8b0`](https://github.com/hydro-project/hydro/commit/39ab8b0278e9e3fe96552ace0a4ae768a6bc10d8))
 * **Uncategorized**
    - Release hydroflow_lang v0.6.0, hydroflow_datalog_core v0.6.0, hydroflow_datalog v0.6.0, hydroflow_macro v0.6.0, lattices v0.5.3, variadics v0.0.4, pusherator v0.0.5, hydroflow v0.6.0, stageleft v0.2.0, hydroflow_plus v0.6.0, hydro_deploy v0.6.0, hydro_cli v0.6.0, hydroflow_plus_cli_integration v0.6.0, safety bump 7 crates ([`09ea65f`](https://github.com/hydro-project/hydro/commit/09ea65fe9cd45c357c43bffca30e60243fa45cc8))
</details>

## v0.5.1 (2024-01-29)

<csr-id-1b555e57c8c812bed4d6495d2960cbf77fb0b3ef/>

### Chore

 - <csr-id-1b555e57c8c812bed4d6495d2960cbf77fb0b3ef/> manually set lockstep-versioned crates (and `lattices`) to version `0.5.1`
   Setting manually since
   https://github.com/frewsxcv/rust-crates-index/issues/159 is messing with
   smart-release

### Documentation

 - <csr-id-3b36020d16792f26da4df3c5b09652a4ab47ec4f/> actually committing empty CHANGELOG.md is required

### New Features

 - <csr-id-20fd1e5f876c5977e44a58757f41c66bdf6a3d15/> improve build error message debuggability
 - <csr-id-46d87fa364d3fe01422cf3c404fbc8a1d5e9fb88/> pass subgraph ID through deploy metadata
 - <csr-id-b7aafd3c97897db4bff62c4ab0b7480ef9a799e0/> improve API naming and eliminate wire API for builders
 - <csr-id-53d7aee8dcc574d47864ec89bfea30a82eab0ee7/> improve Rust API for defining services
 - <csr-id-c50ca121b6d5e30dc07843f82caa135b68626301/> split Rust core from Python bindings

### Bug Fixes

 - <csr-id-d23c2299098dd62058c0951c99a62bb9e0af5b25/> avoid inflexible `\\?\` canonical paths on windows to mitigate `/` separator errors
 - <csr-id-f8a0b95113e92e003061d2a3865c84d69851dd8e/> race conditions when handshake channels capture other outputs
   Timeouts in Hydroflow+ tests were being caused by a race condition in Hydro Deploy where stdout sent after a handshake message would sometimes be sent to the `cli_stdout` channel for handshakes.
   
   This PR adjusts the handshake channels to always be oneshot, so that the broadcaster immediately knows when to send data to the regular stdout channels.
   
   Also refactors Hydro Deploy sources to split up more modules.
 - <csr-id-1ae27de6aafb72cee5da0cce6cf52748161d0f33/> don't vendor openssl and fix docker build
 - <csr-id-1d8adc1df15bac74c6f4496589d615e361019f50/> fix docs and remove unnecessary async_trait

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 12 commits contributed to the release over the course of 39 calendar days.
 - 11 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 9 unique issues were worked on: [#1010](https://github.com/hydro-project/hydro/issues/1010), [#1014](https://github.com/hydro-project/hydro/issues/1014), [#986](https://github.com/hydro-project/hydro/issues/986), [#987](https://github.com/hydro-project/hydro/issues/987), [#992](https://github.com/hydro-project/hydro/issues/992), [#994](https://github.com/hydro-project/hydro/issues/994), [#995](https://github.com/hydro-project/hydro/issues/995), [#996](https://github.com/hydro-project/hydro/issues/996), [#999](https://github.com/hydro-project/hydro/issues/999)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1010](https://github.com/hydro-project/hydro/issues/1010)**
    - Improve build error message debuggability ([`20fd1e5`](https://github.com/hydro-project/hydro/commit/20fd1e5f876c5977e44a58757f41c66bdf6a3d15))
 * **[#1014](https://github.com/hydro-project/hydro/issues/1014)**
    - Avoid inflexible `\\?\` canonical paths on windows to mitigate `/` separator errors ([`d23c229`](https://github.com/hydro-project/hydro/commit/d23c2299098dd62058c0951c99a62bb9e0af5b25))
 * **[#986](https://github.com/hydro-project/hydro/issues/986)**
    - Split Rust core from Python bindings ([`c50ca12`](https://github.com/hydro-project/hydro/commit/c50ca121b6d5e30dc07843f82caa135b68626301))
 * **[#987](https://github.com/hydro-project/hydro/issues/987)**
    - Improve Rust API for defining services ([`53d7aee`](https://github.com/hydro-project/hydro/commit/53d7aee8dcc574d47864ec89bfea30a82eab0ee7))
 * **[#992](https://github.com/hydro-project/hydro/issues/992)**
    - Fix docs and remove unnecessary async_trait ([`1d8adc1`](https://github.com/hydro-project/hydro/commit/1d8adc1df15bac74c6f4496589d615e361019f50))
 * **[#994](https://github.com/hydro-project/hydro/issues/994)**
    - Don't vendor openssl and fix docker build ([`1ae27de`](https://github.com/hydro-project/hydro/commit/1ae27de6aafb72cee5da0cce6cf52748161d0f33))
 * **[#995](https://github.com/hydro-project/hydro/issues/995)**
    - Improve API naming and eliminate wire API for builders ([`b7aafd3`](https://github.com/hydro-project/hydro/commit/b7aafd3c97897db4bff62c4ab0b7480ef9a799e0))
 * **[#996](https://github.com/hydro-project/hydro/issues/996)**
    - Pass subgraph ID through deploy metadata ([`46d87fa`](https://github.com/hydro-project/hydro/commit/46d87fa364d3fe01422cf3c404fbc8a1d5e9fb88))
 * **[#999](https://github.com/hydro-project/hydro/issues/999)**
    - Race conditions when handshake channels capture other outputs ([`f8a0b95`](https://github.com/hydro-project/hydro/commit/f8a0b95113e92e003061d2a3865c84d69851dd8e))
 * **Uncategorized**
    - Release hydro_deploy v0.5.1 ([`f7a54c7`](https://github.com/hydro-project/hydro/commit/f7a54c7ae7c771b16ed2853b28a480fba5f06e5b))
    - Actually committing empty CHANGELOG.md is required ([`3b36020`](https://github.com/hydro-project/hydro/commit/3b36020d16792f26da4df3c5b09652a4ab47ec4f))
    - Manually set lockstep-versioned crates (and `lattices`) to version `0.5.1` ([`1b555e5`](https://github.com/hydro-project/hydro/commit/1b555e57c8c812bed4d6495d2960cbf77fb0b3ef))
</details>

