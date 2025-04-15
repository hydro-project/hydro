# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.13.0 (2025-04-11)

### Bug Fixes (BREAKING)

 - <csr-id-52221ec4b68882a783e0d86e6e4ea80441b4a79b/> fix perf setup, remove GCP `startup_script`, use `TracingOptions::setup_command` instead

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 1 commit contributed to the release over the course of 19 calendar days.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1800](https://github.com/hydro-project/hydro/issues/1800)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1800](https://github.com/hydro-project/hydro/issues/1800)**
    - Fix perf setup, remove GCP `startup_script`, use `TracingOptions::setup_command` instead ([`52221ec`](https://github.com/hydro-project/hydro/commit/52221ec4b68882a783e0d86e6e4ea80441b4a79b))
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

 - 4 commits contributed to the release.
 - 7 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1773](https://github.com/hydro-project/hydro/issues/1773)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1773](https://github.com/hydro-project/hydro/issues/1773)**
    - Remove "hydroflow" for `hydro_deploy_integration`, `hydro_deploy::rust_crate`, fix #1712 ([`7dd71d6`](https://github.com/hydro-project/hydro/commit/7dd71d67da162d2e4f3043b271a52037a3c983c0))
 * **Uncategorized**
    - Release include_mdtests v0.0.0, dfir_rs v0.12.1, hydro_deploy v0.12.1, hydro_lang v0.12.1, hydro_std v0.12.1, hydro_cli v0.12.1 ([`faf0d3e`](https://github.com/hydro-project/hydro/commit/faf0d3ed9f172275f2e2f219c5ead1910c209a36))
    - Release dfir_lang v0.12.1, dfir_datalog_core v0.12.1, dfir_datalog v0.12.1, dfir_macro v0.12.1, hydro_deploy_integration v0.12.1, lattices v0.6.1, pusherator v0.0.12, dfir_rs v0.12.1, hydro_deploy v0.12.1, hydro_lang v0.12.1, hydro_std v0.12.1, hydro_cli v0.12.1 ([`23221b5`](https://github.com/hydro-project/hydro/commit/23221b53b30918707ddaa85529d04cd7919166b4))
    - Set `hydro_deploy_integration` to release as `0.12.1` ([`260902b`](https://github.com/hydro-project/hydro/commit/260902b210378af5291ec71a574256d7a5bcb463))
</details>

## 0.12.0 (2025-03-08)

<csr-id-49a387d4a21f0763df8ec94de73fb953c9cd333a/>
<csr-id-e5e6b75c79028990799a2c0def9fa3c5263788a8/>
<csr-id-2e4f6b828e3d54ee6b1753c1a814006a3d0b3e34/>
<csr-id-41ef00719671b087062628462a947c211d0d765c/>
<csr-id-a3202888433f5eae7d45b0814e090ff6a8600d02/>
<csr-id-ec3795a678d261a38085405b6e9bfea943dafefb/>

### Chore

 - <csr-id-49a387d4a21f0763df8ec94de73fb953c9cd333a/> upgrade to Rust 2024 edition
   - Updates `Cargo.toml` to use new shared workspace keys
   - Updates lint settings (in workspace `Cargo.toml`)
   - `rustfmt` has changed slightly, resulting in a big diff - there are no
   actual code changes
   - Adds a script to `rustfmt` the template src files

### Refactor

 - <csr-id-41ef00719671b087062628462a947c211d0d765c/> improve naming of types involved with networking
   I got confused looking at my own code for client-server handshakes (in
   preparation for external client support). Time to do some renaming!

### Other

 - <csr-id-a3202888433f5eae7d45b0814e090ff6a8600d02/> set dynamic=version in pyproject

### Chore

 - <csr-id-ec3795a678d261a38085405b6e9bfea943dafefb/> upgrade to Rust 2024 edition
   - Updates `Cargo.toml` to use new shared workspace keys
   - Updates lint settings (in workspace `Cargo.toml`)
   - `rustfmt` has changed slightly, resulting in a big diff - there are no
   actual code changes
   - Adds a script to `rustfmt` the template src files

### Other

 - <csr-id-e5e6b75c79028990799a2c0def9fa3c5263788a8/> set dynamic=version in pyproject

### Refactor

 - <csr-id-2e4f6b828e3d54ee6b1753c1a814006a3d0b3e34/> improve naming of types involved with networking
   I got confused looking at my own code for client-server handshakes (in
   preparation for external client support). Time to do some renaming!

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 74 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 3 unique issues were worked on: [#1631](https://github.com/hydro-project/hydro/issues/1631), [#1715](https://github.com/hydro-project/hydro/issues/1715), [#1747](https://github.com/hydro-project/hydro/issues/1747)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1631](https://github.com/hydro-project/hydro/issues/1631)**
    - Set dynamic=version in pyproject ([`a320288`](https://github.com/hydro-project/hydro/commit/a3202888433f5eae7d45b0814e090ff6a8600d02))
 * **[#1715](https://github.com/hydro-project/hydro/issues/1715)**
    - Improve naming of types involved with networking ([`41ef007`](https://github.com/hydro-project/hydro/commit/41ef00719671b087062628462a947c211d0d765c))
 * **[#1747](https://github.com/hydro-project/hydro/issues/1747)**
    - Upgrade to Rust 2024 edition ([`ec3795a`](https://github.com/hydro-project/hydro/commit/ec3795a678d261a38085405b6e9bfea943dafefb))
 * **Uncategorized**
    - Release dfir_lang v0.12.0, dfir_datalog_core v0.12.0, dfir_datalog v0.12.0, dfir_macro v0.12.0, hydroflow_deploy_integration v0.12.0, lattices_macro v0.5.9, variadics v0.0.9, variadics_macro v0.6.0, lattices v0.6.0, multiplatform_test v0.5.0, pusherator v0.0.11, dfir_rs v0.12.0, hydro_deploy v0.12.0, stageleft_macro v0.6.0, stageleft v0.7.0, stageleft_tool v0.6.0, hydro_lang v0.12.0, hydro_std v0.12.0, hydro_cli v0.12.0, safety bump 10 crates ([`973c925`](https://github.com/hydro-project/hydro/commit/973c925e87ed78344494581bd7ce1bbb4186a2f3))
</details>

## 0.11.0 (2024-12-23)

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
 - <csr-id-b6c9a5dfc0b1ce6684a5a55ea3c582776d5f0204/> mark non-deterministic operators as unsafe and introduce timestamped streams
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

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 45 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#1501](https://github.com/hydro-project/hydro/issues/1501), [#1584](https://github.com/hydro-project/hydro/issues/1584)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1501](https://github.com/hydro-project/hydro/issues/1501)**
    - Add `repository` field to `Cargo.toml`s, fix #1452 ([`204bd11`](https://github.com/hydro-project/hydro/commit/204bd117ca3a8845b4986539efb91a0c612dfa05))
 * **[#1584](https://github.com/hydro-project/hydro/issues/1584)**
    - Mark non-deterministic operators as unsafe and introduce timestamped streams ([`b6c9a5d`](https://github.com/hydro-project/hydro/commit/b6c9a5dfc0b1ce6684a5a55ea3c582776d5f0204))
 * **Uncategorized**
    - Release stageleft_macro v0.5.0, stageleft v0.6.0, stageleft_tool v0.5.0, hydro_lang v0.11.0, hydro_std v0.11.0, hydro_cli v0.11.0 ([`7633c38`](https://github.com/hydro-project/hydro/commit/7633c38c4a56acf7e5b3b6f2a72ccc1d6e6eeba1))
    - Release dfir_lang v0.11.0, dfir_datalog_core v0.11.0, dfir_datalog v0.11.0, dfir_macro v0.11.0, hydroflow_deploy_integration v0.11.0, lattices_macro v0.5.8, variadics v0.0.8, variadics_macro v0.5.6, lattices v0.5.9, multiplatform_test v0.4.0, pusherator v0.0.10, dfir_rs v0.11.0, hydro_deploy v0.11.0, stageleft_macro v0.5.0, stageleft v0.6.0, stageleft_tool v0.5.0, hydro_lang v0.11.0, hydro_std v0.11.0, hydro_cli v0.11.0, safety bump 6 crates ([`361b443`](https://github.com/hydro-project/hydro/commit/361b4439ef9c781860f18d511668ab463a8c5203))
</details>

## 0.10.0 (2024-11-08)

<csr-id-d5677604e93c07a5392f4229af94a0b736eca382/>
<csr-id-014ebb2628b5b80ea1b6426b58c4d62706edb9ef/>

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

## 0.9.0 (2024-08-30)

<csr-id-a2ec110ccadb97e293b19d83a155d98d94224bba/>
<csr-id-11af32828bab6e4a4264d2635ff71a12bb0bb778/>
<csr-id-0a465e55dd39c76bc1aefb020460a639d792fe87/>
<csr-id-a2147864b24110c9ae2c1553e9e8b55bd5065f15/>
<csr-id-8856c8596d5ad9d5f24a46467690bfac1549fae2/>
<csr-id-25989c7d938a0e93355a670f8d78a5aea900fce0/>
<csr-id-9a503cf85225ff1fcfe7a815fda3a4ac34a75c42/>
<csr-id-8bcd86c15bc4d9d2e3b564061be879bfe8820e25/>
<csr-id-3508f5aeda3e18a6857df4ceb77e5e1015c02a17/>
<csr-id-2c04f51f1ec44f7898307b6610371dcb490ea686/>

### Chore

 - <csr-id-a2ec110ccadb97e293b19d83a155d98d94224bba/> manually set versions for crates renamed in #1413
 - <csr-id-11af32828bab6e4a4264d2635ff71a12bb0bb778/> lower min dependency versions where possible, update `Cargo.lock`
   Moved from #1418
   
   ---------

### Refactor (BREAKING)

 - <csr-id-25989c7d938a0e93355a670f8d78a5aea900fce0/> rename integration crates to drop CLI references
 - <csr-id-9a503cf85225ff1fcfe7a815fda3a4ac34a75c42/> `Deployment.stop()` for graceful shutdown including updated `perf` profile downloading
   * `perf` profile downloading moved from the `drop()` impl to `async fn
   stop()`
   * download perf data via stdout
   * update async-ssh2-lite to 0.5 to cleanup tokio compat issues
   
   WIP for #1365
 - <csr-id-8bcd86c15bc4d9d2e3b564061be879bfe8820e25/> use `buildstructor` to handle excessive `Deployment` method arguments, fix #1364
   Adds new method `Deployment::AzureHost`

### Chore

 - <csr-id-3508f5aeda3e18a6857df4ceb77e5e1015c02a17/> manually set versions for crates renamed in #1413
 - <csr-id-2c04f51f1ec44f7898307b6610371dcb490ea686/> lower min dependency versions where possible, update `Cargo.lock`
   Moved from #1418
   
   ---------

### New Features

 - <csr-id-46a8a2cb08732bb21096e824bc4542d208c68fb2/> use trybuild to compile subgraph binaries
 - <csr-id-2a49c13f2f4e3b47d79c34167015d6ba98a89888/> use trybuild to compile subgraph binaries

### Bug Fixes

 - <csr-id-63b528feeb2e6dac2ed12c02b2e39e0d42133a74/> only instantiate `Localhost` once
 - <csr-id-628066bf8250b541493c8cf5efd6c7bf01900640/> only instantiate `Localhost` once

### New Features (BREAKING)

 - <csr-id-749a10307f4eff2a46a1056735e84ed94d44b39e/> Perf works over SSH
   See documentation on how to use in
   [Notion](https://www.notion.so/hydro-project/perf-Measuring-CPU-usage-6135b6ce56a94af38eeeba0a55deef9c).
 - <csr-id-43a411ea6ca0ad5110754fe788bb7593519cba51/> Perf works over SSH
   See documentation on how to use in
   [Notion](https://www.notion.so/hydro-project/perf-Measuring-CPU-usage-6135b6ce56a94af38eeeba0a55deef9c).

### Refactor (BREAKING)

 - <csr-id-0a465e55dd39c76bc1aefb020460a639d792fe87/> rename integration crates to drop CLI references
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

 - 9 commits contributed to the release.
 - 38 days passed between releases.
 - 8 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 7 unique issues were worked on: [#1313](https://github.com/hydro-project/hydro/issues/1313), [#1366](https://github.com/hydro-project/hydro/issues/1366), [#1370](https://github.com/hydro-project/hydro/issues/1370), [#1398](https://github.com/hydro-project/hydro/issues/1398), [#1403](https://github.com/hydro-project/hydro/issues/1403), [#1413](https://github.com/hydro-project/hydro/issues/1413), [#1423](https://github.com/hydro-project/hydro/issues/1423)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1313](https://github.com/hydro-project/hydro/issues/1313)**
    - Perf works over SSH ([`43a411e`](https://github.com/hydro-project/hydro/commit/43a411ea6ca0ad5110754fe788bb7593519cba51))
 * **[#1366](https://github.com/hydro-project/hydro/issues/1366)**
    - Use `buildstructor` to handle excessive `Deployment` method arguments, fix #1364 ([`8bcd86c`](https://github.com/hydro-project/hydro/commit/8bcd86c15bc4d9d2e3b564061be879bfe8820e25))
 * **[#1370](https://github.com/hydro-project/hydro/issues/1370)**
    - `Deployment.stop()` for graceful shutdown including updated `perf` profile downloading ([`9a503cf`](https://github.com/hydro-project/hydro/commit/9a503cf85225ff1fcfe7a815fda3a4ac34a75c42))
 * **[#1398](https://github.com/hydro-project/hydro/issues/1398)**
    - Use trybuild to compile subgraph binaries ([`2a49c13`](https://github.com/hydro-project/hydro/commit/2a49c13f2f4e3b47d79c34167015d6ba98a89888))
 * **[#1403](https://github.com/hydro-project/hydro/issues/1403)**
    - Only instantiate `Localhost` once ([`628066b`](https://github.com/hydro-project/hydro/commit/628066bf8250b541493c8cf5efd6c7bf01900640))
 * **[#1413](https://github.com/hydro-project/hydro/issues/1413)**
    - Rename integration crates to drop CLI references ([`25989c7`](https://github.com/hydro-project/hydro/commit/25989c7d938a0e93355a670f8d78a5aea900fce0))
 * **[#1423](https://github.com/hydro-project/hydro/issues/1423)**
    - Lower min dependency versions where possible, update `Cargo.lock` ([`2c04f51`](https://github.com/hydro-project/hydro/commit/2c04f51f1ec44f7898307b6610371dcb490ea686))
 * **Uncategorized**
    - Release hydroflow_lang v0.9.0, hydroflow_datalog_core v0.9.0, hydroflow_datalog v0.9.0, hydroflow_deploy_integration v0.9.0, hydroflow_macro v0.9.0, lattices_macro v0.5.6, lattices v0.5.7, multiplatform_test v0.2.0, variadics v0.0.6, pusherator v0.0.8, hydroflow v0.9.0, stageleft_macro v0.3.0, stageleft v0.4.0, stageleft_tool v0.3.0, hydroflow_plus v0.9.0, hydro_deploy v0.9.0, hydro_cli v0.9.0, hydroflow_plus_deploy v0.9.0, safety bump 8 crates ([`1d54331`](https://github.com/hydro-project/hydro/commit/1d54331976040c049e4c97a9fba0e66930efee52))
    - Manually set versions for crates renamed in #1413 ([`3508f5a`](https://github.com/hydro-project/hydro/commit/3508f5aeda3e18a6857df4ceb77e5e1015c02a17))
</details>

## 0.8.0 (2024-07-23)

<csr-id-3098f77fd99882aae23c4b31017aa4b761306197/>
<csr-id-0feae7454e4674eea1f3308b3d6d4e9d459cda67/>
<csr-id-947ebc1cb21a07fbfacae4ac956dbd0015a8a418/>
<csr-id-c5a8de28e7844b3c29d58116d8340967f2e6bcc4/>
<csr-id-057a0a510568cf81932368c8c65e056f91af7202/>
<csr-id-60390782dd7dcec18d193c800af716843a944dba/>
<csr-id-141eae1c3a1869fa42756250618a21ea2a2c7e34/>
<csr-id-12b8ba53f28eb9de1318b41cdf1e23282f6f0eb6/>
<csr-id-fbd7fb9bed9fd8d2afdfb5ad0edf076c3ad0f83f/>
<csr-id-a97480ab834293bcc81d81fcd10d8944eb312417/>
<csr-id-453fbce73423815752667c560318efe8b78014f8/>
<csr-id-0983248beab176debc602f92fa617f9beb02dad3/>
<csr-id-dd759aea1ac225654501e836b890dd8d144868b4/>
<csr-id-d56c731482e25f3ab397c4912df35a6375fcb23a/>
<csr-id-dfe7a0938c302353db05d9889eb8d88640887443/>
<csr-id-45091d413f6da32927b640df781ce671a6e17c15/>

### Chore

 - <csr-id-3098f77fd99882aae23c4b31017aa4b761306197/> update pinned rust version to 2024-06-17

### Style (BREAKING)

 - <csr-id-fbd7fb9bed9fd8d2afdfb5ad0edf076c3ad0f83f/> enable clippy `upper-case-acronyms-aggressive`
   * rename `GCP` -> `Gcp`, `NodeID` -> `NodeId`
   * update CI `cargo-generate` template testing to use PR's branch instead
   of whatever `main` happens to be

### Refactor (BREAKING)

 - <csr-id-a97480ab834293bcc81d81fcd10d8944eb312417/> make `Host` trait use `&self` interior mutability to remove `RwLock` wrappings #430
   Depends on #1346
 - <csr-id-453fbce73423815752667c560318efe8b78014f8/> make `HydroflowSource`, `HydroflowSink` traits use `&self` interior mutability to remove `RwLock` wrappings #430
   Depends on #1339
 - <csr-id-0983248beab176debc602f92fa617f9beb02dad3/> replace `async-channel` with `tokio::sync::mpsc::unbounded_channel`
   Depends on #1339
   
   We could make the publicly facing `stdout`, `stderr` APIs return `impl Stream<Output = String>` in the future, maybe
 - <csr-id-dd759aea1ac225654501e836b890dd8d144868b4/> replace some uses of `tokio::sync::RwLock` with `std::sync::Mutex` #430 (3/3)

### Style

 - <csr-id-d56c731482e25f3ab397c4912df35a6375fcb23a/> rename `SSH` -> `Ssh`

### Refactor

 - <csr-id-dfe7a0938c302353db05d9889eb8d88640887443/> build cache cleanup
   * Replace mystery tuple with new `struct BuildOutput`
   * Replace `Mutex` and `Arc`-infested `HashMap` with `memo-map` crate,
   greatly simplifying build cache typing
   * Remove redundant build caching in `HydroflowCrateService`, expose and
   use cache parameters as `BuildParams`
   * Remove `once_cell` and `async-once-cell` dependencies, use `std`'s
   `OnceLock`
   * Add `Failed to execute command: {}` context to `perf` error message
   * Cleanup some repeated `format!` expressions

### Chore

 - <csr-id-45091d413f6da32927b640df781ce671a6e17c15/> update pinned rust version to 2024-06-17

### Refactor

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

 - <csr-id-c5a8de28e7844b3c29d58116d8340967f2e6bcc4/> make `Host` trait use `&self` interior mutability to remove `RwLock` wrappings #430
   Depends on #1346
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

 - 9 commits contributed to the release.
 - 59 days passed between releases.
 - 8 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 8 unique issues were worked on: [#1309](https://github.com/hydro-project/hydro/issues/1309), [#1334](https://github.com/hydro-project/hydro/issues/1334), [#1339](https://github.com/hydro-project/hydro/issues/1339), [#1340](https://github.com/hydro-project/hydro/issues/1340), [#1345](https://github.com/hydro-project/hydro/issues/1345), [#1346](https://github.com/hydro-project/hydro/issues/1346), [#1347](https://github.com/hydro-project/hydro/issues/1347), [#1356](https://github.com/hydro-project/hydro/issues/1356)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1309](https://github.com/hydro-project/hydro/issues/1309)**
    - Update pinned rust version to 2024-06-17 ([`45091d4`](https://github.com/hydro-project/hydro/commit/45091d413f6da32927b640df781ce671a6e17c15))
 * **[#1334](https://github.com/hydro-project/hydro/issues/1334)**
    - Build cache cleanup ([`dfe7a09`](https://github.com/hydro-project/hydro/commit/dfe7a0938c302353db05d9889eb8d88640887443))
 * **[#1339](https://github.com/hydro-project/hydro/issues/1339)**
    - Replace some uses of `tokio::sync::RwLock` with `std::sync::Mutex` #430 (3/3) ([`dd759ae`](https://github.com/hydro-project/hydro/commit/dd759aea1ac225654501e836b890dd8d144868b4))
 * **[#1340](https://github.com/hydro-project/hydro/issues/1340)**
    - Rename `SSH` -> `Ssh` ([`d56c731`](https://github.com/hydro-project/hydro/commit/d56c731482e25f3ab397c4912df35a6375fcb23a))
 * **[#1345](https://github.com/hydro-project/hydro/issues/1345)**
    - Enable clippy `upper-case-acronyms-aggressive` ([`fbd7fb9`](https://github.com/hydro-project/hydro/commit/fbd7fb9bed9fd8d2afdfb5ad0edf076c3ad0f83f))
 * **[#1346](https://github.com/hydro-project/hydro/issues/1346)**
    - Make `HydroflowSource`, `HydroflowSink` traits use `&self` interior mutability to remove `RwLock` wrappings #430 ([`453fbce`](https://github.com/hydro-project/hydro/commit/453fbce73423815752667c560318efe8b78014f8))
 * **[#1347](https://github.com/hydro-project/hydro/issues/1347)**
    - Make `Host` trait use `&self` interior mutability to remove `RwLock` wrappings #430 ([`a97480a`](https://github.com/hydro-project/hydro/commit/a97480ab834293bcc81d81fcd10d8944eb312417))
 * **[#1356](https://github.com/hydro-project/hydro/issues/1356)**
    - Replace `async-channel` with `tokio::sync::mpsc::unbounded_channel` ([`0983248`](https://github.com/hydro-project/hydro/commit/0983248beab176debc602f92fa617f9beb02dad3))
 * **Uncategorized**
    - Release hydroflow_lang v0.8.0, hydroflow_datalog_core v0.8.0, hydroflow_datalog v0.8.0, hydroflow_macro v0.8.0, lattices_macro v0.5.5, lattices v0.5.6, variadics v0.0.5, pusherator v0.0.7, hydroflow v0.8.0, hydroflow_plus v0.8.0, hydro_deploy v0.8.0, hydro_cli v0.8.0, hydroflow_plus_cli_integration v0.8.0, safety bump 7 crates ([`7b9c367`](https://github.com/hydro-project/hydro/commit/7b9c3678930af8010f8e2ffd4069583ece528119))
</details>

## 0.7.0 (2024-05-24)

<csr-id-18015029a725b068696ed9edefd1097583c858a6/>
<csr-id-8c2a230afbc965d86339b7e9d71bc99c28d2fe41/>

### Chore

 - <csr-id-18015029a725b068696ed9edefd1097583c858a6/> update pyo3, silence warnings in generated code

### Chore

 - <csr-id-8c2a230afbc965d86339b7e9d71bc99c28d2fe41/> update pyo3, silence warnings in generated code

### New Features

 - <csr-id-29a263fb564c5ce4bc495ea4e9d20b8b2621b645/> add support for collecting counts and running perf
 - <csr-id-a33d9e29bcab427961dbfe2f03d80a9b87ecda6c/> add support for collecting counts and running perf

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 44 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#1152](https://github.com/hydro-project/hydro/issues/1152), [#1157](https://github.com/hydro-project/hydro/issues/1157)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1152](https://github.com/hydro-project/hydro/issues/1152)**
    - Update pyo3, silence warnings in generated code ([`8c2a230`](https://github.com/hydro-project/hydro/commit/8c2a230afbc965d86339b7e9d71bc99c28d2fe41))
 * **[#1157](https://github.com/hydro-project/hydro/issues/1157)**
    - Add support for collecting counts and running perf ([`a33d9e2`](https://github.com/hydro-project/hydro/commit/a33d9e29bcab427961dbfe2f03d80a9b87ecda6c))
 * **Uncategorized**
    - Release hydroflow_lang v0.7.0, hydroflow_datalog_core v0.7.0, hydroflow_datalog v0.7.0, hydroflow_macro v0.7.0, lattices v0.5.5, multiplatform_test v0.1.0, pusherator v0.0.6, hydroflow v0.7.0, stageleft_macro v0.2.0, stageleft v0.3.0, stageleft_tool v0.2.0, hydroflow_plus v0.7.0, hydro_deploy v0.7.0, hydro_cli v0.7.0, hydroflow_plus_cli_integration v0.7.0, safety bump 8 crates ([`855fda6`](https://github.com/hydro-project/hydro/commit/855fda65442ad7a9074a099ecc29e74322332418))
</details>

## 0.6.1 (2024-04-09)

<csr-id-7958fb0d900be8fe7359326abfa11dcb8fb35e8a/>
<csr-id-864ea856ecbabfe6786990924021a70fb4252765/>

### Style

 - <csr-id-7958fb0d900be8fe7359326abfa11dcb8fb35e8a/> qualified path cleanups for clippy

### Style

 - <csr-id-864ea856ecbabfe6786990924021a70fb4252765/> qualified path cleanups for clippy

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
    - Qualified path cleanups for clippy ([`864ea85`](https://github.com/hydro-project/hydro/commit/864ea856ecbabfe6786990924021a70fb4252765))
 * **Uncategorized**
    - Release hydroflow_plus v0.6.1, hydro_deploy v0.6.1, hydro_cli v0.6.1, hydroflow_plus_cli_integration v0.6.1 ([`499965b`](https://github.com/hydro-project/hydro/commit/499965b2bd27d3fca7e328b19960761bb64c0c0e))
    - Release hydroflow_lang v0.6.2, hydroflow v0.6.2, hydroflow_plus v0.6.1, hydro_deploy v0.6.1, hydro_cli v0.6.1, hydroflow_plus_cli_integration v0.6.1, stageleft_tool v0.1.1 ([`67e16d0`](https://github.com/hydro-project/hydro/commit/67e16d069a2d565039dcf17e6caf0a23e258f983))
    - Release hydroflow_cli_integration v0.5.2, hydroflow_lang v0.6.1, hydroflow_datalog_core v0.6.1, lattices v0.5.4, hydroflow v0.6.1, stageleft_macro v0.1.1, stageleft v0.2.1, hydroflow_plus v0.6.1, hydro_deploy v0.6.1, hydro_cli v0.6.1, hydroflow_plus_cli_integration v0.6.1, stageleft_tool v0.1.1 ([`fb82e52`](https://github.com/hydro-project/hydro/commit/fb82e523bb217658775989a276e18a1af68103c8))
</details>

## 0.6.0 (2024-03-02)

<csr-id-e9639f608f8dafd3f384837067800a66951b25df/>
<csr-id-d8203407a97c2ccbcb5ce0cc739d8ae5a89a40c7/>

### New Features

 - <csr-id-fcf43bf86fe550247dffa4641a9ce3aff3b9afc3/> Add support for azure
   I accidentally committed some large files, so you won't see the commit
   history because I copied over the changes onto a fresh clone.
 - <csr-id-8021da6e5fa5127dc67420157dff980d51c710ed/> Add support for azure
   I accidentally committed some large files, so you won't see the commit
   history because I copied over the changes onto a fresh clone.

### Other

 - <csr-id-e9639f608f8dafd3f384837067800a66951b25df/> consolidate tasks and use sccache and nextest

### Other

 - <csr-id-d8203407a97c2ccbcb5ce0cc739d8ae5a89a40c7/> consolidate tasks and use sccache and nextest

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 28 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#1015](https://github.com/hydro-project/hydro/issues/1015), [#1043](https://github.com/hydro-project/hydro/issues/1043)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1015](https://github.com/hydro-project/hydro/issues/1015)**
    - Consolidate tasks and use sccache and nextest ([`d820340`](https://github.com/hydro-project/hydro/commit/d8203407a97c2ccbcb5ce0cc739d8ae5a89a40c7))
 * **[#1043](https://github.com/hydro-project/hydro/issues/1043)**
    - Add support for azure ([`8021da6`](https://github.com/hydro-project/hydro/commit/8021da6e5fa5127dc67420157dff980d51c710ed))
 * **Uncategorized**
    - Release hydroflow_lang v0.6.0, hydroflow_datalog_core v0.6.0, hydroflow_datalog v0.6.0, hydroflow_macro v0.6.0, lattices v0.5.3, variadics v0.0.4, pusherator v0.0.5, hydroflow v0.6.0, stageleft v0.2.0, hydroflow_plus v0.6.0, hydro_deploy v0.6.0, hydro_cli v0.6.0, hydroflow_plus_cli_integration v0.6.0, safety bump 7 crates ([`0e94db4`](https://github.com/hydro-project/hydro/commit/0e94db41c842c1181574c5e69179027cfa7a19cf))
</details>

## 0.5.1 (2024-02-02)

<csr-id-ba6afab8416ad66eee4fdb9d0c73e62d45752617/>
<csr-id-1b555e57c8c812bed4d6495d2960cbf77fb0b3ef/>
<csr-id-69e04167f4774cf1ca3351e7ac34d15cfa83362b/>
<csr-id-7c48faf0d8301b498fa59e5eee5cddf5fa341229/>
<csr-id-5fa4168ea9d0ca07fb6fc46149aefbdc32ed7d61/>

### Chore

 - <csr-id-ba6afab8416ad66eee4fdb9d0c73e62d45752617/> fix clippy lints on latest nightly

### Chore

 - <csr-id-7c48faf0d8301b498fa59e5eee5cddf5fa341229/> manually set lockstep-versioned crates (and `lattices`) to version `0.5.1`
   Setting manually since
   https://github.com/frewsxcv/rust-crates-index/issues/159 is messing with
   smart-release
 - <csr-id-5fa4168ea9d0ca07fb6fc46149aefbdc32ed7d61/> generate pre-move changelogs for `hydro_cli` and `hydroflow_cli_integration`

### Bug Fixes

 - <csr-id-1ae27de6aafb72cee5da0cce6cf52748161d0f33/> don't vendor openssl and fix docker build
 - <csr-id-eef407e063aa0d9079dc800bd300c39185f4390a/> don't vendor openssl and fix docker build

### Chore

 - <csr-id-1b555e57c8c812bed4d6495d2960cbf77fb0b3ef/> manually set lockstep-versioned crates (and `lattices`) to version `0.5.1`
   Setting manually since
   https://github.com/frewsxcv/rust-crates-index/issues/159 is messing with
   smart-release
 - <csr-id-69e04167f4774cf1ca3351e7ac34d15cfa83362b/> generate pre-move changelogs for `hydro_cli` and `hydroflow_cli_integration`

### New Features

 - <csr-id-174607d12277d7544d0f42890c9a5da2ff184df4/> support building graphs for symmetric clusters in Hydroflow+
 - <csr-id-9e275824c88b24d060a7de5822e1359959b36b03/> auto-configure Hydro Deploy based on Hydroflow+ plans
 - <csr-id-7e46da04de306b42c454cd4c29d1cbc677827740/> perf improvements and better deploy logic
 - <csr-id-d8ca3d47c6ebd9268c61c6066eba23acfc3e1b26/> implement core fault tolerance protocol
 - <csr-id-6158a7aae2ef9b58245c23fc668715a3fb2ff7dc/> new implementation and Hydro Deploy setup
   --
 - <csr-id-53d7aee8dcc574d47864ec89bfea30a82eab0ee7/> improve Rust API for defining services
 - <csr-id-c50ca121b6d5e30dc07843f82caa135b68626301/> split Rust core from Python bindings
 - <csr-id-4133f52a40f7f77fb1d0bb44952815bc1fa4f1a5/> improve Rust API for defining services
 - <csr-id-04553830046ac51fcaa212c2565a742f56b3a3e5/> split Rust core from Python bindings

### Pre-Move Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release over the course of 42 calendar days.
 - 70 days passed between releases.
 - 6 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 6 unique issues were worked on: [#909](https://github.com/hydro-project/hydroflow/issues/909), [#910](https://github.com/hydro-project/hydroflow/issues/910), [#914](https://github.com/hydro-project/hydroflow/issues/914), [#960](https://github.com/hydro-project/hydroflow/issues/960), [#982](https://github.com/hydro-project/hydroflow/issues/982), [#984](https://github.com/hydro-project/hydroflow/issues/984)

### Pre-Move Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#909](https://github.com/hydro-project/hydroflow/issues/909)**
    - New implementation and Hydro Deploy setup ([`6158a7a`](https://github.com/hydro-project/hydroflow/commit/6158a7aae2ef9b58245c23fc668715a3fb2ff7dc))
 * **[#910](https://github.com/hydro-project/hydroflow/issues/910)**
    - Implement core fault tolerance protocol ([`d8ca3d4`](https://github.com/hydro-project/hydroflow/commit/d8ca3d47c6ebd9268c61c6066eba23acfc3e1b26))
 * **[#914](https://github.com/hydro-project/hydroflow/issues/914)**
    - Perf improvements and better deploy logic ([`7e46da0`](https://github.com/hydro-project/hydroflow/commit/7e46da04de306b42c454cd4c29d1cbc677827740))
 * **[#960](https://github.com/hydro-project/hydroflow/issues/960)**
    - Fix clippy lints on latest nightly ([`ba6afab`](https://github.com/hydro-project/hydroflow/commit/ba6afab8416ad66eee4fdb9d0c73e62d45752617))
 * **[#982](https://github.com/hydro-project/hydroflow/issues/982)**
    - Auto-configure Hydro Deploy based on Hydroflow+ plans ([`9e27582`](https://github.com/hydro-project/hydroflow/commit/9e275824c88b24d060a7de5822e1359959b36b03))
 * **[#984](https://github.com/hydro-project/hydroflow/issues/984)**
    - Support building graphs for symmetric clusters in Hydroflow+ ([`174607d`](https://github.com/hydro-project/hydroflow/commit/174607d12277d7544d0f42890c9a5da2ff184df4))
</details>

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 7 commits contributed to the release.
 - 114 days passed between releases.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 4 unique issues were worked on: [#1046](https://github.com/hydro-project/hydro/issues/1046), [#986](https://github.com/hydro-project/hydro/issues/986), [#987](https://github.com/hydro-project/hydro/issues/987), [#994](https://github.com/hydro-project/hydro/issues/994)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1046](https://github.com/hydro-project/hydro/issues/1046)**
    - Generate pre-move changelogs for `hydro_cli` and `hydroflow_cli_integration` ([`5fa4168`](https://github.com/hydro-project/hydro/commit/5fa4168ea9d0ca07fb6fc46149aefbdc32ed7d61))
 * **[#986](https://github.com/hydro-project/hydro/issues/986)**
    - Split Rust core from Python bindings ([`0455383`](https://github.com/hydro-project/hydro/commit/04553830046ac51fcaa212c2565a742f56b3a3e5))
 * **[#987](https://github.com/hydro-project/hydro/issues/987)**
    - Improve Rust API for defining services ([`4133f52`](https://github.com/hydro-project/hydro/commit/4133f52a40f7f77fb1d0bb44952815bc1fa4f1a5))
 * **[#994](https://github.com/hydro-project/hydro/issues/994)**
    - Don't vendor openssl and fix docker build ([`eef407e`](https://github.com/hydro-project/hydro/commit/eef407e063aa0d9079dc800bd300c39185f4390a))
 * **Uncategorized**
    - Release hydroflow_lang v0.5.2, hydroflow_datalog_core v0.5.2, hydroflow_macro v0.5.2, lattices v0.5.2, hydroflow v0.5.2, hydro_cli v0.5.1, hydroflow_plus_cli_integration v0.5.1 ([`c6af815`](https://github.com/hydro-project/hydro/commit/c6af815d0dc1133477cfd54e0159939f337bf94f))
    - Release hydroflow_cli_integration v0.5.1, hydroflow_lang v0.5.1, hydroflow_datalog_core v0.5.1, hydroflow_datalog v0.5.1, hydroflow_macro v0.5.1, lattices v0.5.1, variadics v0.0.3, pusherator v0.0.4, hydroflow v0.5.1, stageleft_macro v0.1.0, stageleft v0.1.0, hydroflow_plus v0.5.1, hydro_deploy v0.5.1, hydro_cli v0.5.1 ([`5a5e6d5`](https://github.com/hydro-project/hydro/commit/5a5e6d5933cf3c20ff23768d4592b0dde94e940b))
    - Manually set lockstep-versioned crates (and `lattices`) to version `0.5.1` ([`7c48faf`](https://github.com/hydro-project/hydro/commit/7c48faf0d8301b498fa59e5eee5cddf5fa341229))
</details>

## 0.5.0 (2023-10-11)

<csr-id-2b95a6d08c993760adaf79b945fdd0fbbdc8ecf2/>

### Chore

 - <csr-id-2b95a6d08c993760adaf79b945fdd0fbbdc8ecf2/> Add `clippy::needless_pass_by_ref_mut` false-positive workaround

### Bug Fixes

 - <csr-id-5d77694b6a3603381ae4217eb7aba8e00ee8d1e5/> better error message when using wrong port types
 - <csr-id-a927dc6afbe3178815b7c7c58ed2838d42d80334/> clippy warning on multiline string in hydro_cli, py_udf

### Pre-Move Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release over the course of 41 calendar days.
 - 56 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#931](https://github.com/hydro-project/hydroflow/issues/931)

### Pre-Move Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#931](https://github.com/hydro-project/hydroflow/issues/931)**
    - Better error message when using wrong port types ([`5d77694`](https://github.com/hydro-project/hydroflow/commit/5d77694b6a3603381ae4217eb7aba8e00ee8d1e5))
 * **Uncategorized**
    - Release hydroflow_macro v0.5.0, lattices v0.5.0, hydroflow v0.5.0, hydro_cli v0.5.0 ([`12697c2`](https://github.com/hydro-project/hydroflow/commit/12697c2f19bd96802591fa63a5b6b12104ecfe0d))
    - Release hydroflow_lang v0.5.0, hydroflow_datalog_core v0.5.0, hydroflow_datalog v0.5.0, hydroflow_macro v0.5.0, lattices v0.5.0, hydroflow v0.5.0, hydro_cli v0.5.0, safety bump 4 crates ([`2e2d8b3`](https://github.com/hydro-project/hydroflow/commit/2e2d8b386fb086c8276a2853d2a1f96ad4d7c221))
    - Clippy warning on multiline string in hydro_cli, py_udf ([`a927dc6`](https://github.com/hydro-project/hydroflow/commit/a927dc6afbe3178815b7c7c58ed2838d42d80334))
    - Add `clippy::needless_pass_by_ref_mut` false-positive workaround ([`2b95a6d`](https://github.com/hydro-project/hydroflow/commit/2b95a6d08c993760adaf79b945fdd0fbbdc8ecf2))
</details>

## 0.4.0 (2023-08-15)

<csr-id-949db02e9fa9878e1a7176c180d6f44c5cddf052/>

### Chore

 - <csr-id-949db02e9fa9878e1a7176c180d6f44c5cddf052/> fix lints for latest nightly

### Pre-Move Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release over the course of 27 calendar days.
 - 42 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#844](https://github.com/hydro-project/hydroflow/issues/844)

### Pre-Move Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#844](https://github.com/hydro-project/hydroflow/issues/844)**
    - Fix lints for latest nightly ([`949db02`](https://github.com/hydro-project/hydroflow/commit/949db02e9fa9878e1a7176c180d6f44c5cddf052))
 * **Uncategorized**
    - Release hydroflow_lang v0.4.0, hydroflow_datalog_core v0.4.0, hydroflow_datalog v0.4.0, hydroflow_macro v0.4.0, lattices v0.4.0, pusherator v0.0.3, hydroflow v0.4.0, hydro_cli v0.4.0, safety bump 4 crates ([`cb313f0`](https://github.com/hydro-project/hydroflow/commit/cb313f0635214460a8308d05cbef4bf7f4bfaa15))
</details>

## 0.3.0 (2023-07-04)

<csr-id-4c2cf81411835529b5d7daa35717834e46e28b9b/>

Unchanged from previous release.

### Chore

 - <csr-id-4c2cf81411835529b5d7daa35717834e46e28b9b/> mark hydro_cli as unchanged for 0.3 release

### Pre-Move Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 33 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Pre-Move Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release hydroflow_cli_integration v0.3.0, hydroflow_lang v0.3.0, hydroflow_datalog_core v0.3.0, hydroflow_datalog v0.3.0, hydroflow_macro v0.3.0, lattices v0.3.0, pusherator v0.0.2, hydroflow v0.3.0, hydro_cli v0.3.0, safety bump 5 crates ([`ec9633e`](https://github.com/hydro-project/hydroflow/commit/ec9633e2e393c2bf106223abeb0b680200fbdf84))
    - Mark hydro_cli as unchanged for 0.3 release ([`4c2cf81`](https://github.com/hydro-project/hydroflow/commit/4c2cf81411835529b5d7daa35717834e46e28b9b))
</details>

## v0.2.0 (2023-05-31)

<csr-id-fd896fbe925fbd8ef1d16be7206ac20ba585081a/>

### Chore

 - <csr-id-fd896fbe925fbd8ef1d16be7206ac20ba585081a/> manually bump versions for v0.2.0 release

### New Features

 - <csr-id-8b2c9f09b1f423ac6d562c29d4ea587578f1c98a/> Add more detailed Hydro Deploy docs and rename `ConnectedBidi` => `ConnectedDirect`

### Pre-Move Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 1 day passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#723](https://github.com/hydro-project/hydroflow/issues/723)

### Pre-Move Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#723](https://github.com/hydro-project/hydroflow/issues/723)**
    - Add more detailed Hydro Deploy docs and rename `ConnectedBidi` => `ConnectedDirect` ([`8b2c9f0`](https://github.com/hydro-project/hydroflow/commit/8b2c9f09b1f423ac6d562c29d4ea587578f1c98a))
 * **Uncategorized**
    - Release hydroflow_lang v0.2.0, hydroflow_datalog_core v0.2.0, hydroflow_datalog v0.2.0, hydroflow_macro v0.2.0, lattices v0.2.0, hydroflow v0.2.0, hydro_cli v0.2.0 ([`ca464c3`](https://github.com/hydro-project/hydroflow/commit/ca464c32322a7ad39eb53e1794777c849aa548a0))
    - Manually bump versions for v0.2.0 release ([`fd896fb`](https://github.com/hydro-project/hydroflow/commit/fd896fbe925fbd8ef1d16be7206ac20ba585081a))
</details>

## v0.1.0 (2023-05-29)

<csr-id-665ad20d996c7873117ff7cccfac22366117d71a/>
<csr-id-382a83c2304eda476d4ff8195a96efebd8dbbcb7/>
<csr-id-52ee8f8e443f0a8b5caf92d2c5f028c00302a79b/>
<csr-id-51a3a9e5f19594a21702d66730d5d1668009b550/>
<csr-id-2bd8517768ff3924b7af274d8d97f126143c4a2a/>
<csr-id-cd0a86d9271d0e3daab59c46f079925f863424e1/>
<csr-id-20a1b2c0cd04a8b495a02ce345db3d48a99ea0e9/>
<csr-id-1eda91a2ef8794711ef037240f15284e8085d863/>
<csr-id-61a1a0509b465ed57003bd0cdfedee8b847a48c8/>
<csr-id-e3ddfb8b47effd03a9bb346811ea360a14ab17b3/>

### Chore

 - <csr-id-665ad20d996c7873117ff7cccfac22366117d71a/> Cargo.toml documentation and description
 - <csr-id-382a83c2304eda476d4ff8195a96efebd8dbbcb7/> set hydroflow_cli_integration version
 - <csr-id-52ee8f8e443f0a8b5caf92d2c5f028c00302a79b/> bump versions to 0.1.0 for release
   For release on crates.io for v0.1

### Other

 - <csr-id-61a1a0509b465ed57003bd0cdfedee8b847a48c8/> initialize hydro_cli/CHANGELOG.md

### Chore

 - <csr-id-e3ddfb8b47effd03a9bb346811ea360a14ab17b3/> Cargo.toml documentation and description

### New Features

 - <csr-id-4536ac6bbcd14a621b5a039d7fe213bff72a8db1/> finish up WebSocket chat example and avoid deadlocks in network setup

### Bug Fixes

 - <csr-id-1c06b3b9ed253aea8c1d2cfd87a1ea77ce550f70/> don't create file copies on when deploying to localhost
   This causes issues on M1, likely due to some signing issue?
 - <csr-id-268f83794d77fbb95f7d3ce7e2439371ccbf8e0c/> mismatched package name in CLI build and attempt to really fix crashes
 - <csr-id-508b00e064427211d6ec6c884af1eb4a602d19b9/> Prepare action logic to publish CLI to PyPi and eliminate GIL acquires
   Hopefully this will work on the first try? Not really a good way to test it. It seems that acquiring the GIL in async/await code is asking for trouble, so this also eliminates those.

### Other

 - <csr-id-51a3a9e5f19594a21702d66730d5d1668009b550/> initialize hydro_cli/CHANGELOG.md
 - <csr-id-2bd8517768ff3924b7af274d8d97f126143c4a2a/> publish hydro_cli
   Will bump versions for python deploy.
   Update build-cli.yml to publish on hydro_cli release

### Style

 - <csr-id-cd0a86d9271d0e3daab59c46f079925f863424e1/> Warn lint `unused_qualifications`
 - <csr-id-20a1b2c0cd04a8b495a02ce345db3d48a99ea0e9/> rustfmt group imports
 - <csr-id-1eda91a2ef8794711ef037240f15284e8085d863/> rustfmt prescribe flat-module `use` format

### Pre-Move Commit Statistics

<csr-read-only-do-not-edit/>

 - 71 commits contributed to the release over the course of 101 calendar days.
 - 12 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 63 unique issues were worked on: [#390](https://github.com/hydro-project/hydroflow/issues/390), [#397](https://github.com/hydro-project/hydroflow/issues/397), [#410](https://github.com/hydro-project/hydroflow/issues/410), [#411](https://github.com/hydro-project/hydroflow/issues/411), [#417](https://github.com/hydro-project/hydroflow/issues/417), [#420](https://github.com/hydro-project/hydroflow/issues/420), [#433](https://github.com/hydro-project/hydroflow/issues/433), [#436](https://github.com/hydro-project/hydroflow/issues/436), [#437](https://github.com/hydro-project/hydroflow/issues/437), [#445](https://github.com/hydro-project/hydroflow/issues/445), [#446](https://github.com/hydro-project/hydroflow/issues/446), [#451](https://github.com/hydro-project/hydroflow/issues/451), [#452](https://github.com/hydro-project/hydroflow/issues/452), [#460](https://github.com/hydro-project/hydroflow/issues/460), [#461](https://github.com/hydro-project/hydroflow/issues/461), [#462](https://github.com/hydro-project/hydroflow/issues/462), [#466](https://github.com/hydro-project/hydroflow/issues/466), [#473](https://github.com/hydro-project/hydroflow/issues/473), [#474](https://github.com/hydro-project/hydroflow/issues/474), [#477](https://github.com/hydro-project/hydroflow/issues/477), [#479](https://github.com/hydro-project/hydroflow/issues/479), [#481](https://github.com/hydro-project/hydroflow/issues/481), [#484](https://github.com/hydro-project/hydroflow/issues/484), [#492](https://github.com/hydro-project/hydroflow/issues/492), [#494](https://github.com/hydro-project/hydroflow/issues/494), [#498](https://github.com/hydro-project/hydroflow/issues/498), [#503](https://github.com/hydro-project/hydroflow/issues/503), [#513](https://github.com/hydro-project/hydroflow/issues/513), [#515](https://github.com/hydro-project/hydroflow/issues/515), [#525](https://github.com/hydro-project/hydroflow/issues/525), [#527](https://github.com/hydro-project/hydroflow/issues/527), [#531](https://github.com/hydro-project/hydroflow/issues/531), [#532](https://github.com/hydro-project/hydroflow/issues/532), [#533](https://github.com/hydro-project/hydroflow/issues/533), [#534](https://github.com/hydro-project/hydroflow/issues/534), [#535](https://github.com/hydro-project/hydroflow/issues/535), [#537](https://github.com/hydro-project/hydroflow/issues/537), [#542](https://github.com/hydro-project/hydroflow/issues/542), [#557](https://github.com/hydro-project/hydroflow/issues/557), [#560](https://github.com/hydro-project/hydroflow/issues/560), [#576](https://github.com/hydro-project/hydroflow/issues/576), [#582](https://github.com/hydro-project/hydroflow/issues/582), [#586](https://github.com/hydro-project/hydroflow/issues/586), [#596](https://github.com/hydro-project/hydroflow/issues/596), [#600](https://github.com/hydro-project/hydroflow/issues/600), [#612](https://github.com/hydro-project/hydroflow/issues/612), [#617](https://github.com/hydro-project/hydroflow/issues/617), [#620](https://github.com/hydro-project/hydroflow/issues/620), [#626](https://github.com/hydro-project/hydroflow/issues/626), [#627](https://github.com/hydro-project/hydroflow/issues/627), [#628](https://github.com/hydro-project/hydroflow/issues/628), [#631](https://github.com/hydro-project/hydroflow/issues/631), [#647](https://github.com/hydro-project/hydroflow/issues/647), [#656](https://github.com/hydro-project/hydroflow/issues/656), [#660](https://github.com/hydro-project/hydroflow/issues/660), [#679](https://github.com/hydro-project/hydroflow/issues/679), [#681](https://github.com/hydro-project/hydroflow/issues/681), [#684](https://github.com/hydro-project/hydroflow/issues/684), [#694](https://github.com/hydro-project/hydroflow/issues/694), [#699](https://github.com/hydro-project/hydroflow/issues/699), [#708](https://github.com/hydro-project/hydroflow/issues/708), [#712](https://github.com/hydro-project/hydroflow/issues/712), [#715](https://github.com/hydro-project/hydroflow/issues/715)

### Pre-Move Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#390](https://github.com/hydro-project/hydroflow/issues/390)**
    - Introduce initial Hydro CLI architecture ([`52aa6e0`](https://github.com/hydro-project/hydroflow/commit/52aa6e0e5d5417bc185cf8f1f961c5494b5b5129))
 * **[#397](https://github.com/hydro-project/hydroflow/issues/397)**
    - Add basic support for connecting services with Unix/TCP sockets ([`dbdad61`](https://github.com/hydro-project/hydroflow/commit/dbdad61d43412a44449495b4204e37d5d128c12c))
 * **[#410](https://github.com/hydro-project/hydroflow/issues/410)**
    - Fixup! Initial support for GCP deployments ([`8695b5d`](https://github.com/hydro-project/hydroflow/commit/8695b5de22a03a4f5f06352c216183e9e10c5199))
    - Initial support for GCP deployments ([`f10a54f`](https://github.com/hydro-project/hydroflow/commit/f10a54ff1eee3e71e1c488d5948762171cca3f5b))
 * **[#411](https://github.com/hydro-project/hydroflow/issues/411)**
    - Fix non-unix (windows) build referencing unix sockets ([`5dac7e4`](https://github.com/hydro-project/hydroflow/commit/5dac7e4fcd2022c4fb9538d55f9a793139b98c6f))
 * **[#417](https://github.com/hydro-project/hydroflow/issues/417)**
    - Add API for defining custom services in deployment ([`2fb8871`](https://github.com/hydro-project/hydroflow/commit/2fb88710603948479580aea58f894ab3929280c8))
 * **[#420](https://github.com/hydro-project/hydroflow/issues/420)**
    - Update clap ([`4be709f`](https://github.com/hydro-project/hydroflow/commit/4be709f03acd854d27e551638e31af7ce5b26c0b))
 * **[#433](https://github.com/hydro-project/hydroflow/issues/433)**
    - Package CLI as a Python wheel to simplify distribution ([`b952257`](https://github.com/hydro-project/hydroflow/commit/b95225770b8ab43a414d5f3c41387d6941f45f26))
 * **[#436](https://github.com/hydro-project/hydroflow/issues/436)**
    - Support passing through extra arguments to deployment scripts ([`f40009c`](https://github.com/hydro-project/hydroflow/commit/f40009c2eab949c533ae5fb69fd9433a6b75c686))
 * **[#437](https://github.com/hydro-project/hydroflow/issues/437)**
    - Extract common logic for establishing CLI-configured connections ([`44cce72`](https://github.com/hydro-project/hydroflow/commit/44cce727b4363d1b6e7f73d72e0a3bec7b6ace53))
 * **[#445](https://github.com/hydro-project/hydroflow/issues/445)**
    - Add `demux` operator to Hydro CLI to map node IDs to connections ([`886d00f`](https://github.com/hydro-project/hydroflow/commit/886d00f6694ba926c9e1ff184acb31a5d60cee23))
 * **[#446](https://github.com/hydro-project/hydroflow/issues/446)**
    - Support running example deployment script without CLI ([`4b3233a`](https://github.com/hydro-project/hydroflow/commit/4b3233a3b791cfbde4a7721b6796436ef41233d0))
 * **[#451](https://github.com/hydro-project/hydroflow/issues/451)**
    - Enable local deployments on non-Linux hosts ([`74c8d3d`](https://github.com/hydro-project/hydroflow/commit/74c8d3d1f18c564808c930147e4d31463b80c735))
 * **[#452](https://github.com/hydro-project/hydroflow/issues/452)**
    - Build CLI wheels in CI and minimize CLI dependencies ([`3e33d0c`](https://github.com/hydro-project/hydroflow/commit/3e33d0cf6b068f0567e55462732598f8a4e2da6a))
 * **[#460](https://github.com/hydro-project/hydroflow/issues/460)**
    - Allow specifying args to launch `HydroflowCrate` with ([`3575fd3`](https://github.com/hydro-project/hydroflow/commit/3575fd3dd2b4aa98361cc4f723d590eff4794f5f))
 * **[#461](https://github.com/hydro-project/hydroflow/issues/461)**
    - Support networking topologies that mix local and cloud through SSH tunneling ([`0ec6d88`](https://github.com/hydro-project/hydroflow/commit/0ec6d889469331a212c04f9568136f770f0c973d))
 * **[#462](https://github.com/hydro-project/hydroflow/issues/462)**
    - Directly expose Rust bindings as Python APIs ([`b94413a`](https://github.com/hydro-project/hydroflow/commit/b94413a380007f5f4f710d2c849c412602a8f8c2))
 * **[#466](https://github.com/hydro-project/hydroflow/issues/466)**
    - Add APIs for sending data to a Hydroflow service from Python ([`c2203a1`](https://github.com/hydro-project/hydroflow/commit/c2203a15f0144308365af227f3ca044ae6a7954b))
 * **[#473](https://github.com/hydro-project/hydroflow/issues/473)**
    - Fixup! Add initial VPC configuration API and improve interrupt handling ([`7f21514`](https://github.com/hydro-project/hydroflow/commit/7f21514d2be2d9dd5e877ad5be534c81579367ce))
    - Add initial VPC configuration API and improve interrupt handling ([`c729fc0`](https://github.com/hydro-project/hydroflow/commit/c729fc0fe01ba75b0ba622e9bc68d891c5353e03))
 * **[#474](https://github.com/hydro-project/hydroflow/issues/474)**
    - Extract common SSH host logic into a separate module ([`5cc884e`](https://github.com/hydro-project/hydroflow/commit/5cc884e4063729216990c1793fb412edd60b0c63))
 * **[#477](https://github.com/hydro-project/hydroflow/issues/477)**
    - Properly handle interrupts and fix non-flushing demux ([`00ea017`](https://github.com/hydro-project/hydroflow/commit/00ea017e40b796e7561979efa0921658dfe072fd))
 * **[#479](https://github.com/hydro-project/hydroflow/issues/479)**
    - Allow custom ports to be used as sinks ([`8da15b7`](https://github.com/hydro-project/hydroflow/commit/8da15b7cbd8bdbf960d3ed58b69f98538ccacd2c))
 * **[#481](https://github.com/hydro-project/hydroflow/issues/481)**
    - Display Anyhow traces when using directly using CLI APIs ([`0f19fa4`](https://github.com/hydro-project/hydroflow/commit/0f19fa4ab1c821649e7f400b1842515e83fb4585))
 * **[#484](https://github.com/hydro-project/hydroflow/issues/484)**
    - Add merge API to CLI to have multiple sources for one sink ([`e09b567`](https://github.com/hydro-project/hydroflow/commit/e09b5670795292f66a004f41314c3c4aa7a24eeb))
 * **[#492](https://github.com/hydro-project/hydroflow/issues/492)**
    - Add API to gracefully shutdown services ([`eda517a`](https://github.com/hydro-project/hydroflow/commit/eda517a3435093830135a9f0384bfae1de5c853e))
 * **[#494](https://github.com/hydro-project/hydroflow/issues/494)**
    - Fixup! Add initial VPC configuration API and improve interrupt handling ([`7f21514`](https://github.com/hydro-project/hydroflow/commit/7f21514d2be2d9dd5e877ad5be534c81579367ce))
 * **[#498](https://github.com/hydro-project/hydroflow/issues/498)**
    - Add API to get CLI connection config as JSON ([`323e0f0`](https://github.com/hydro-project/hydroflow/commit/323e0f0afd73b66f321b2e88498627e76a186a4e))
 * **[#503](https://github.com/hydro-project/hydroflow/issues/503)**
    - Allow redeployment in CLI with updated services and hosts ([`967df05`](https://github.com/hydro-project/hydroflow/commit/967df05e7ec97201cdc602316bd99c03b541b5d4))
 * **[#513](https://github.com/hydro-project/hydroflow/issues/513)**
    - Add `hydro.null` API to connect no-op sources and sinks ([`9b2a4a6`](https://github.com/hydro-project/hydroflow/commit/9b2a4a690798d2a976221901fa25a908b7600f52))
 * **[#515](https://github.com/hydro-project/hydroflow/issues/515)**
    - Initial TopoloTree actor implementation for binary tree ([`e9fcc24`](https://github.com/hydro-project/hydroflow/commit/e9fcc24761b676f7f0796767d6f910eaad1ee9b4))
 * **[#525](https://github.com/hydro-project/hydroflow/issues/525)**
    - Add `existing` parameter to `GCPNetwork` to use existing VPCs ([`33249e4`](https://github.com/hydro-project/hydroflow/commit/33249e4517e8ca3735a0949957ef9b43c55ff947))
 * **[#527](https://github.com/hydro-project/hydroflow/issues/527)**
    - Actually return a `GCPComputeEngineHost` when creating one ([`0eef370`](https://github.com/hydro-project/hydroflow/commit/0eef370485b9904185f846a553c94accc0a91118))
 * **[#531](https://github.com/hydro-project/hydroflow/issues/531)**
    - Provision hosts even if they are not being used by a service ([`abdf61d`](https://github.com/hydro-project/hydroflow/commit/abdf61d8982e83262e8a452214936c0f9d90e456))
 * **[#532](https://github.com/hydro-project/hydroflow/issues/532)**
    - Generalize null source support into `SourcePath` abstraction ([`835ba3b`](https://github.com/hydro-project/hydroflow/commit/835ba3bdaf553dad8261b89087e0ab45f017325b))
 * **[#533](https://github.com/hydro-project/hydroflow/issues/533)**
    - Add `hydro.mux` operator and initial API tests ([`c25272b`](https://github.com/hydro-project/hydroflow/commit/c25272b90f8cc5ec7614caa29f0be889d2220510))
 * **[#534](https://github.com/hydro-project/hydroflow/issues/534)**
    - Allow specifying the user to sign in as on a GCP machine ([`ad1609d`](https://github.com/hydro-project/hydroflow/commit/ad1609d0c9a700ada5678a8df05694ff9606c54c))
 * **[#535](https://github.com/hydro-project/hydroflow/issues/535)**
    - Ignore GCP port requests for ports that have already been allocated ([`c948ab8`](https://github.com/hydro-project/hydroflow/commit/c948ab8aaad2204b277eb80752529283351536d6))
 * **[#537](https://github.com/hydro-project/hydroflow/issues/537)**
    - Use the correct user account ([`86135f4`](https://github.com/hydro-project/hydroflow/commit/86135f4efa3375e3ce527f40f05474d7011c1487))
 * **[#542](https://github.com/hydro-project/hydroflow/issues/542)**
    - Avoid deadlock in port loading when a service connects to itself ([`559f115`](https://github.com/hydro-project/hydroflow/commit/559f1154cb4b84b7b4cd3963c2d212e2bc05d524))
 * **[#557](https://github.com/hydro-project/hydroflow/issues/557)**
    - Have Python drive CLI cancellations to support interrupting loops ([`f3e57c9`](https://github.com/hydro-project/hydroflow/commit/f3e57c9ff7df36e24419aab9d6a957a11b5ab7cb))
 * **[#560](https://github.com/hydro-project/hydroflow/issues/560)**
    - Refactor `hydro.mux` to `source.tagged(id)` and support connections where the tagged source is the server ([`3f0ecc9`](https://github.com/hydro-project/hydroflow/commit/3f0ecc92abed7a0c95c04255adcc6d39c0767703))
 * **[#576](https://github.com/hydro-project/hydroflow/issues/576)**
    - Add classic counter CRDT benchmark to compare against ([`2f3bf04`](https://github.com/hydro-project/hydroflow/commit/2f3bf04ab33768b04d44f3f58907f958d4cd8dc8))
 * **[#582](https://github.com/hydro-project/hydroflow/issues/582)**
    - Add a global cache for Cargo builds initiated by the CLI ([`83c1df7`](https://github.com/hydro-project/hydroflow/commit/83c1df792d0dbb1d89fd9383ea284ca3ff167778))
 * **[#586](https://github.com/hydro-project/hydroflow/issues/586)**
    - Bump pinned nightly and fix build failures on latest nightly ([`84a831e`](https://github.com/hydro-project/hydroflow/commit/84a831efca6eddac20bac140c9c67bf4ab2d5cf8))
 * **[#596](https://github.com/hydro-project/hydroflow/issues/596)**
    - Improve CLI interrupt handling when subtasks are spawned ([`93fb340`](https://github.com/hydro-project/hydroflow/commit/93fb34040b12a74d246729e37bb6a3bd9924b807))
 * **[#600](https://github.com/hydro-project/hydroflow/issues/600)**
    - Display rich progress for deployment tasks in console ([`467e2fb`](https://github.com/hydro-project/hydroflow/commit/467e2fb719fb101e1c706814c07ebfc43f324eec))
 * **[#612](https://github.com/hydro-project/hydroflow/issues/612)**
    - Fix lints on windows ([`2f8d3e2`](https://github.com/hydro-project/hydroflow/commit/2f8d3e212f4d60d908e733d1b1f1348501596df8))
 * **[#617](https://github.com/hydro-project/hydroflow/issues/617)**
    - Update `Cargo.toml`s for publishing ([`a78ff9a`](https://github.com/hydro-project/hydroflow/commit/a78ff9aace6771787c2b72aad83be6ad8d49a828))
 * **[#620](https://github.com/hydro-project/hydroflow/issues/620)**
    - Replace using `cargo` as a library to shell out with `cargo-metadata` instead ([`5f2e8f3`](https://github.com/hydro-project/hydroflow/commit/5f2e8f3abffec38ba99afeb60969788e16e2f4ff))
 * **[#626](https://github.com/hydro-project/hydroflow/issues/626)**
    - Print logs from services with a prefix identifying the service ([`79dda6a`](https://github.com/hydro-project/hydroflow/commit/79dda6ab463f51c0c3e1c932cba0f45ef95a4f78))
 * **[#627](https://github.com/hydro-project/hydroflow/issues/627)**
    - Display cargo build status formatted next to a progress bar ([`5cbe43a`](https://github.com/hydro-project/hydroflow/commit/5cbe43a44e9e118eaf790886bef8409cd6b211ee))
 * **[#628](https://github.com/hydro-project/hydroflow/issues/628)**
    - Handle Terraform printing a log about reading existing resources ([`6bf7b71`](https://github.com/hydro-project/hydroflow/commit/6bf7b7182cfe137cfda3164898b461e5e5602ae7))
 * **[#631](https://github.com/hydro-project/hydroflow/issues/631)**
    - Avoid clobbering Rust errors with the progress bar ([`6f3cf4b`](https://github.com/hydro-project/hydroflow/commit/6f3cf4bcff4de658e9a4d80180748aefe393a0bb))
 * **[#647](https://github.com/hydro-project/hydroflow/issues/647)**
    - Fix Hydro CLI builds failing due to breaking Maturin change ([`ffee23f`](https://github.com/hydro-project/hydroflow/commit/ffee23f33a77e54a7ab6af3a678f95ed35f0b4eb))
 * **[#656](https://github.com/hydro-project/hydroflow/issues/656)**
    - Add WebSocket with CLI example and simplify init API ([`1015980`](https://github.com/hydro-project/hydroflow/commit/1015980ed995634ff8735e4daf33796e73bab563))
 * **[#660](https://github.com/hydro-project/hydroflow/issues/660)**
    - Warn lint `unused_qualifications` ([`cd0a86d`](https://github.com/hydro-project/hydroflow/commit/cd0a86d9271d0e3daab59c46f079925f863424e1))
    - Rustfmt group imports ([`20a1b2c`](https://github.com/hydro-project/hydroflow/commit/20a1b2c0cd04a8b495a02ce345db3d48a99ea0e9))
    - Rustfmt prescribe flat-module `use` format ([`1eda91a`](https://github.com/hydro-project/hydroflow/commit/1eda91a2ef8794711ef037240f15284e8085d863))
 * **[#679](https://github.com/hydro-project/hydroflow/issues/679)**
    - Only load converters helper module once in the CLI ([`860d74f`](https://github.com/hydro-project/hydroflow/commit/860d74fcab8525397eb630b14ca7c6619fcef1f4))
 * **[#681](https://github.com/hydro-project/hydroflow/issues/681)**
    - Migrate playground to new docs site ([`4d16bd2`](https://github.com/hydro-project/hydroflow/commit/4d16bd218104e1abcc1e1210942b0ec5b63301d0))
 * **[#684](https://github.com/hydro-project/hydroflow/issues/684)**
    - Bump versions to 0.1.0 for release ([`52ee8f8`](https://github.com/hydro-project/hydroflow/commit/52ee8f8e443f0a8b5caf92d2c5f028c00302a79b))
 * **[#694](https://github.com/hydro-project/hydroflow/issues/694)**
    - Prepare action logic to publish CLI to PyPi and eliminate GIL acquires ([`508b00e`](https://github.com/hydro-project/hydroflow/commit/508b00e064427211d6ec6c884af1eb4a602d19b9))
 * **[#699](https://github.com/hydro-project/hydroflow/issues/699)**
    - Mismatched package name in CLI build and attempt to really fix crashes ([`268f837`](https://github.com/hydro-project/hydroflow/commit/268f83794d77fbb95f7d3ce7e2439371ccbf8e0c))
 * **[#708](https://github.com/hydro-project/hydroflow/issues/708)**
    - Finish up WebSocket chat example and avoid deadlocks in network setup ([`4536ac6`](https://github.com/hydro-project/hydroflow/commit/4536ac6bbcd14a621b5a039d7fe213bff72a8db1))
 * **[#712](https://github.com/hydro-project/hydroflow/issues/712)**
    - Publish hydro_cli ([`2bd8517`](https://github.com/hydro-project/hydroflow/commit/2bd8517768ff3924b7af274d8d97f126143c4a2a))
 * **[#715](https://github.com/hydro-project/hydroflow/issues/715)**
    - Don't create file copies on when deploying to localhost ([`1c06b3b`](https://github.com/hydro-project/hydroflow/commit/1c06b3b9ed253aea8c1d2cfd87a1ea77ce550f70))
 * **Uncategorized**
    - Release hydro_cli v0.1.0 ([`5d48544`](https://github.com/hydro-project/hydroflow/commit/5d485442691f878ae6835f631ae13ff856fd941c))
    - Cargo.toml documentation and description ([`e3ddfb8`](https://github.com/hydro-project/hydroflow/commit/e3ddfb8b47effd03a9bb346811ea360a14ab17b3))
    - Initialize hydro_cli/CHANGELOG.md ([`61a1a05`](https://github.com/hydro-project/hydroflow/commit/61a1a0509b465ed57003bd0cdfedee8b847a48c8))
    - Set hydroflow_cli_integration version ([`382a83c`](https://github.com/hydro-project/hydroflow/commit/382a83c2304eda476d4ff8195a96efebd8dbbcb7))
    - Update pinned nightly rust version 2023-04-18 ([`6ced3c1`](https://github.com/hydro-project/hydroflow/commit/6ced3c177969dec3d3e3cf5938ab3973c1d1239b))
</details>

