# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v0.9.0 (2024-08-30)

### Chore

 - <csr-id-11af32828bab6e4a4264d2635ff71a12bb0bb778/> lower min dependency versions where possible, update `Cargo.lock`
   Moved from #1418
   
   ---------

### Documentation

 - <csr-id-f5f1eb0c612f5c0c1752360d972ef6853c5e12f0/> cleanup doc comments for clippy latest

### New Features

 - <csr-id-71f69aa5e9f2ba187f07c44c0a9f2becfe72aab1/> add API for cycle with initial value
 - <csr-id-82de6f5fc89fd44fd2ac18fddd94d121b4b10c8a/> add unbounded top-level singletons
 - <csr-id-7bf9ee2f707ddd5d8f51853ab7babe035fd8d964/> add paxos
 - <csr-id-46a8a2cb08732bb21096e824bc4542d208c68fb2/> use trybuild to compile subgraph binaries
 - <csr-id-eaf497b601928be37530bc8d81717d200fd5987a/> add operators necessary for Paxos / PBFT

### Bug Fixes

 - <csr-id-22c72189bb76412955d29b03c5d99894c558a07c/> remove `FlowProps`
 - <csr-id-1aeacb212227f654e8f0cdc8a59816a68f059177/> rewrite IR in place to avoid stack overflow and disable cloning
   Cloning was unsafe because values behind a `Rc<RefCell<...>>` in the
   case of tee would be entangled with the old IR.
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1404).
   * #1405
   * #1398
   * __->__ #1404
 - <csr-id-b518e674560971ebd1b32c737151214b8d3310b0/> wrong stream type for `source_interval`
 - <csr-id-c12b2495c70f170eba655e458f4591ef7d0941a4/> add `Clone` bounds to `cross_join` and simplify broadcast logic
 - <csr-id-ab12e5b66718f06adc3c34bf879c9581d79ee0d2/> overly restrictive input types for `send_bincode_interleaved`
   The original types prevented usage in cluster-to-cluster communication.

### New Features (BREAKING)

 - <csr-id-44c6b149bea102e8598460ba0286e370b36fd25a/> separate singletons into their own types
 - <csr-id-536e6442d68b0947da5bfef9991825003e6867fc/> refactor API to have no-tick semantics by default
   Now, by default streams exist at a "top-level" where there are no ticks
   and operators run over the entire collection. To perform iterative
   computations, developers must explicitly entire a tick domain (using
   `tick_batch`), and return to the outer domain (using `all_ticks`).

### Refactor (BREAKING)

 - <csr-id-0a465e55dd39c76bc1aefb020460a639d792fe87/> rename integration crates to drop CLI references
 - <csr-id-5f2789a13d1602f170e678fe9bbc59caf69db4b5/> disentangle instantiated nodes from locations
 - <csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377
 - <csr-id-09d6d44eafc866881e73719813fe9edeb49ca2a6/> start rearranging stages of flow compilation to prepare for trybuild approach

### Style (BREAKING)

 - <csr-id-fa417205569d8c49c85b0c2324118e0f9b1c8407/> rename some `CLI`->`Deploy`, decapitalize acronym names

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 20 commits contributed to the release.
 - 20 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 20 unique issues were worked on: [#1358](https://github.com/hydro-project/hydroflow/issues/1358), [#1368](https://github.com/hydro-project/hydroflow/issues/1368), [#1375](https://github.com/hydro-project/hydroflow/issues/1375), [#1376](https://github.com/hydro-project/hydroflow/issues/1376), [#1377](https://github.com/hydro-project/hydroflow/issues/1377), [#1394](https://github.com/hydro-project/hydroflow/issues/1394), [#1395](https://github.com/hydro-project/hydroflow/issues/1395), [#1398](https://github.com/hydro-project/hydroflow/issues/1398), [#1399](https://github.com/hydro-project/hydroflow/issues/1399), [#1404](https://github.com/hydro-project/hydroflow/issues/1404), [#1405](https://github.com/hydro-project/hydroflow/issues/1405), [#1410](https://github.com/hydro-project/hydroflow/issues/1410), [#1413](https://github.com/hydro-project/hydroflow/issues/1413), [#1420](https://github.com/hydro-project/hydroflow/issues/1420), [#1421](https://github.com/hydro-project/hydroflow/issues/1421), [#1423](https://github.com/hydro-project/hydroflow/issues/1423), [#1425](https://github.com/hydro-project/hydroflow/issues/1425), [#1427](https://github.com/hydro-project/hydroflow/issues/1427), [#1428](https://github.com/hydro-project/hydroflow/issues/1428), [#1430](https://github.com/hydro-project/hydroflow/issues/1430)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1358](https://github.com/hydro-project/hydroflow/issues/1358)**
    - Start rearranging stages of flow compilation to prepare for trybuild approach ([`09d6d44`](https://github.com/hydro-project/hydroflow/commit/09d6d44eafc866881e73719813fe9edeb49ca2a6))
 * **[#1368](https://github.com/hydro-project/hydroflow/issues/1368)**
    - Overly restrictive input types for `send_bincode_interleaved` ([`ab12e5b`](https://github.com/hydro-project/hydroflow/commit/ab12e5b66718f06adc3c34bf879c9581d79ee0d2))
 * **[#1375](https://github.com/hydro-project/hydroflow/issues/1375)**
    - Add `Clone` bounds to `cross_join` and simplify broadcast logic ([`c12b249`](https://github.com/hydro-project/hydroflow/commit/c12b2495c70f170eba655e458f4591ef7d0941a4))
 * **[#1376](https://github.com/hydro-project/hydroflow/issues/1376)**
    - Add operators necessary for Paxos / PBFT ([`eaf497b`](https://github.com/hydro-project/hydroflow/commit/eaf497b601928be37530bc8d81717d200fd5987a))
 * **[#1377](https://github.com/hydro-project/hydroflow/issues/1377)**
    - Defer network instantiation until after finalizing IR ([`0eba702`](https://github.com/hydro-project/hydroflow/commit/0eba702f62e7a6816cf931b01a2ea5643bd7321d))
 * **[#1394](https://github.com/hydro-project/hydroflow/issues/1394)**
    - Simplify process/cluster specs ([`128aaec`](https://github.com/hydro-project/hydroflow/commit/128aaecd40edce57dc254afdcd61ecd5b9948d71))
 * **[#1395](https://github.com/hydro-project/hydroflow/issues/1395)**
    - Disentangle instantiated nodes from locations ([`5f2789a`](https://github.com/hydro-project/hydroflow/commit/5f2789a13d1602f170e678fe9bbc59caf69db4b5))
 * **[#1398](https://github.com/hydro-project/hydroflow/issues/1398)**
    - Use trybuild to compile subgraph binaries ([`46a8a2c`](https://github.com/hydro-project/hydroflow/commit/46a8a2cb08732bb21096e824bc4542d208c68fb2))
 * **[#1399](https://github.com/hydro-project/hydroflow/issues/1399)**
    - Rename some `CLI`->`Deploy`, decapitalize acronym names ([`fa41720`](https://github.com/hydro-project/hydroflow/commit/fa417205569d8c49c85b0c2324118e0f9b1c8407))
 * **[#1404](https://github.com/hydro-project/hydroflow/issues/1404)**
    - Rewrite IR in place to avoid stack overflow and disable cloning ([`1aeacb2`](https://github.com/hydro-project/hydroflow/commit/1aeacb212227f654e8f0cdc8a59816a68f059177))
 * **[#1405](https://github.com/hydro-project/hydroflow/issues/1405)**
    - Wrong stream type for `source_interval` ([`b518e67`](https://github.com/hydro-project/hydroflow/commit/b518e674560971ebd1b32c737151214b8d3310b0))
 * **[#1410](https://github.com/hydro-project/hydroflow/issues/1410)**
    - Add paxos ([`7bf9ee2`](https://github.com/hydro-project/hydroflow/commit/7bf9ee2f707ddd5d8f51853ab7babe035fd8d964))
 * **[#1413](https://github.com/hydro-project/hydroflow/issues/1413)**
    - Rename integration crates to drop CLI references ([`0a465e5`](https://github.com/hydro-project/hydroflow/commit/0a465e55dd39c76bc1aefb020460a639d792fe87))
 * **[#1420](https://github.com/hydro-project/hydroflow/issues/1420)**
    - Remove `FlowProps` ([`22c7218`](https://github.com/hydro-project/hydroflow/commit/22c72189bb76412955d29b03c5d99894c558a07c))
 * **[#1421](https://github.com/hydro-project/hydroflow/issues/1421)**
    - Refactor API to have no-tick semantics by default ([`536e644`](https://github.com/hydro-project/hydroflow/commit/536e6442d68b0947da5bfef9991825003e6867fc))
 * **[#1423](https://github.com/hydro-project/hydroflow/issues/1423)**
    - Lower min dependency versions where possible, update `Cargo.lock` ([`11af328`](https://github.com/hydro-project/hydroflow/commit/11af32828bab6e4a4264d2635ff71a12bb0bb778))
 * **[#1425](https://github.com/hydro-project/hydroflow/issues/1425)**
    - Separate singletons into their own types ([`44c6b14`](https://github.com/hydro-project/hydroflow/commit/44c6b149bea102e8598460ba0286e370b36fd25a))
 * **[#1427](https://github.com/hydro-project/hydroflow/issues/1427)**
    - Add unbounded top-level singletons ([`82de6f5`](https://github.com/hydro-project/hydroflow/commit/82de6f5fc89fd44fd2ac18fddd94d121b4b10c8a))
 * **[#1428](https://github.com/hydro-project/hydroflow/issues/1428)**
    - Cleanup doc comments for clippy latest ([`f5f1eb0`](https://github.com/hydro-project/hydroflow/commit/f5f1eb0c612f5c0c1752360d972ef6853c5e12f0))
 * **[#1430](https://github.com/hydro-project/hydroflow/issues/1430)**
    - Add API for cycle with initial value ([`71f69aa`](https://github.com/hydro-project/hydroflow/commit/71f69aa5e9f2ba187f07c44c0a9f2becfe72aab1))
</details>

## v0.8.0 (2024-07-23)

<csr-id-67c0e51fb25ea1a2e3aae197c1984920b46759fa/>

### Reverted

 - <csr-id-256779abece03bee662b351430d27141d10bd5ef/> "feat(hydroflow): Added poll_futures and poll_futures_async operators.", fix #1183
   This reverts commit 997d90a76db9a4e05dbac35073a09548750ce342.
   
   We have been trying to figure out the semantics a bit, and want to give
   it more thought before we commit to maintaining it
   
   Can un-revert and adjust the semantics later when we use it

### Refactor (BREAKING)

 - <csr-id-67c0e51fb25ea1a2e3aae197c1984920b46759fa/> require lifetime on `perist*()` operators

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 3 unique issues were worked on: [#1143](https://github.com/hydro-project/hydroflow/issues/1143), [#1216](https://github.com/hydro-project/hydroflow/issues/1216), [#1295](https://github.com/hydro-project/hydroflow/issues/1295)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1143](https://github.com/hydro-project/hydroflow/issues/1143)**
    - "feat(hydroflow): Added poll_futures and poll_futures_async operators.", fix #1183 ([`256779a`](https://github.com/hydro-project/hydroflow/commit/256779abece03bee662b351430d27141d10bd5ef))
 * **[#1216](https://github.com/hydro-project/hydroflow/issues/1216)**
    - "feat(hydroflow): Added poll_futures and poll_futures_async operators.", fix #1183 ([`256779a`](https://github.com/hydro-project/hydroflow/commit/256779abece03bee662b351430d27141d10bd5ef))
 * **[#1295](https://github.com/hydro-project/hydroflow/issues/1295)**
    - Require lifetime on `perist*()` operators ([`67c0e51`](https://github.com/hydro-project/hydroflow/commit/67c0e51fb25ea1a2e3aae197c1984920b46759fa))
 * **Uncategorized**
    - Release hydroflow_lang v0.8.0, hydroflow_datalog_core v0.8.0, hydroflow_datalog v0.8.0, hydroflow_macro v0.8.0, lattices_macro v0.5.5, lattices v0.5.6, variadics v0.0.5, pusherator v0.0.7, hydroflow v0.8.0, hydroflow_plus v0.8.0, hydro_deploy v0.8.0, hydro_cli v0.8.0, hydroflow_plus_cli_integration v0.8.0, safety bump 7 crates ([`ca6c16b`](https://github.com/hydro-project/hydroflow/commit/ca6c16b4a7ce35e155fe7fc6c7d1676c37c9e4de))
</details>

## v0.7.0 (2024-05-24)

<csr-id-c9dfddc680e0ce5415539d7b77bc5beb97ab59d9/>

### Chore

 - <csr-id-c9dfddc680e0ce5415539d7b77bc5beb97ab59d9/> use workaround for `cargo smart-release` not properly ordering `dev-`/`build-dependencies`

### New Features

 - <csr-id-6e571726ff40818fbe9bbe9923511877c20fb243/> add API to get the cluster ID of the current node
   feat(hydroflow_plus): add API to get the cluster ID of the current node
 - <csr-id-997d90a76db9a4e05dbac35073a09548750ce342/> Added poll_futures and poll_futures_async operators.
 - <csr-id-c3f5a37ff746401a2383a900f9004e33072d5b1a/> add prototype of tagging algebraic properties
 - <csr-id-29a263fb564c5ce4bc495ea4e9d20b8b2621b645/> add support for collecting counts and running perf

### Bug Fixes

 - <csr-id-0cafbdb74a665412a83aa900b4eb10c00e2498dd/> handle send_bincode with local structs
   fix(hydroflow_plus): handle send_bincode with local structs

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 7 commits contributed to the release.
 - 6 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 6 unique issues were worked on: [#1143](https://github.com/hydro-project/hydroflow/issues/1143), [#1151](https://github.com/hydro-project/hydroflow/issues/1151), [#1156](https://github.com/hydro-project/hydroflow/issues/1156), [#1157](https://github.com/hydro-project/hydroflow/issues/1157), [#1194](https://github.com/hydro-project/hydroflow/issues/1194), [#1238](https://github.com/hydro-project/hydroflow/issues/1238)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1143](https://github.com/hydro-project/hydroflow/issues/1143)**
    - Added poll_futures and poll_futures_async operators. ([`997d90a`](https://github.com/hydro-project/hydroflow/commit/997d90a76db9a4e05dbac35073a09548750ce342))
 * **[#1151](https://github.com/hydro-project/hydroflow/issues/1151)**
    - Handle send_bincode with local structs ([`0cafbdb`](https://github.com/hydro-project/hydroflow/commit/0cafbdb74a665412a83aa900b4eb10c00e2498dd))
 * **[#1156](https://github.com/hydro-project/hydroflow/issues/1156)**
    - Add prototype of tagging algebraic properties ([`c3f5a37`](https://github.com/hydro-project/hydroflow/commit/c3f5a37ff746401a2383a900f9004e33072d5b1a))
 * **[#1157](https://github.com/hydro-project/hydroflow/issues/1157)**
    - Add support for collecting counts and running perf ([`29a263f`](https://github.com/hydro-project/hydroflow/commit/29a263fb564c5ce4bc495ea4e9d20b8b2621b645))
 * **[#1194](https://github.com/hydro-project/hydroflow/issues/1194)**
    - Add API to get the cluster ID of the current node ([`6e57172`](https://github.com/hydro-project/hydroflow/commit/6e571726ff40818fbe9bbe9923511877c20fb243))
 * **[#1238](https://github.com/hydro-project/hydroflow/issues/1238)**
    - Use workaround for `cargo smart-release` not properly ordering `dev-`/`build-dependencies` ([`c9dfddc`](https://github.com/hydro-project/hydroflow/commit/c9dfddc680e0ce5415539d7b77bc5beb97ab59d9))
 * **Uncategorized**
    - Release hydroflow_lang v0.7.0, hydroflow_datalog_core v0.7.0, hydroflow_datalog v0.7.0, hydroflow_macro v0.7.0, lattices v0.5.5, multiplatform_test v0.1.0, pusherator v0.0.6, hydroflow v0.7.0, stageleft_macro v0.2.0, stageleft v0.3.0, stageleft_tool v0.2.0, hydroflow_plus v0.7.0, hydro_deploy v0.7.0, hydro_cli v0.7.0, hydroflow_plus_cli_integration v0.7.0, safety bump 8 crates ([`2852147`](https://github.com/hydro-project/hydroflow/commit/285214740627685e911781793e05d234ab2ad2bd))
</details>

## v0.6.1 (2024-04-09)

<csr-id-fc447ffdf8fd1b2189545a991f08588238182f00/>

### Chore

 - <csr-id-fc447ffdf8fd1b2189545a991f08588238182f00/> appease latest nightly clippy
   Also updates `surface_keyed_fold.rs` `test_fold_keyed_infer_basic` test.

### New Features

 - <csr-id-7f68ebf2a23e8e73719229a6f0408bffc7fbe7af/> simplify Location trait to remove lifetimes
 - <csr-id-77f3e5afb9e276d1d6c643574ebac75ed0003939/> simplify lifetime bounds for processes and clusters
   feat(hydroflow_plus): simplify lifetime bounds for processes and
   clusters
   
   This allows `extract` to move the flow builder, which is a prerequisite
   for having developers run the optimizer during deployment as well in
   case it changes the network topology.
 - <csr-id-5b6562662ce3a0dd172ddc1103a591c1c6037e95/> move persist manipulation into a proper optimization
   feat(hydroflow_plus): move persist manipulation into a proper
   optimization
 - <csr-id-cfb3029a6fb0836789db04a7d0d4a1e8b812b629/> add APIs for running optimization passes
   feat(hydroflow_plus): add APIs for running optimization passes

### Bug Fixes

 - <csr-id-2d2c43dc001dbea17d46d73de464c95066b18fa2/> allow BuiltFlow to be cloned even if the deploy flavor can't

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 9 commits contributed to the release.
 - 6 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 6 unique issues were worked on: [#1083](https://github.com/hydro-project/hydroflow/issues/1083), [#1098](https://github.com/hydro-project/hydroflow/issues/1098), [#1100](https://github.com/hydro-project/hydroflow/issues/1100), [#1101](https://github.com/hydro-project/hydroflow/issues/1101), [#1107](https://github.com/hydro-project/hydroflow/issues/1107), [#1140](https://github.com/hydro-project/hydroflow/issues/1140)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1083](https://github.com/hydro-project/hydroflow/issues/1083)**
    - Add APIs for running optimization passes ([`cfb3029`](https://github.com/hydro-project/hydroflow/commit/cfb3029a6fb0836789db04a7d0d4a1e8b812b629))
 * **[#1098](https://github.com/hydro-project/hydroflow/issues/1098)**
    - Move persist manipulation into a proper optimization ([`5b65626`](https://github.com/hydro-project/hydroflow/commit/5b6562662ce3a0dd172ddc1103a591c1c6037e95))
 * **[#1100](https://github.com/hydro-project/hydroflow/issues/1100)**
    - Simplify lifetime bounds for processes and clusters ([`77f3e5a`](https://github.com/hydro-project/hydroflow/commit/77f3e5afb9e276d1d6c643574ebac75ed0003939))
 * **[#1101](https://github.com/hydro-project/hydroflow/issues/1101)**
    - Simplify Location trait to remove lifetimes ([`7f68ebf`](https://github.com/hydro-project/hydroflow/commit/7f68ebf2a23e8e73719229a6f0408bffc7fbe7af))
 * **[#1107](https://github.com/hydro-project/hydroflow/issues/1107)**
    - Allow BuiltFlow to be cloned even if the deploy flavor can't ([`2d2c43d`](https://github.com/hydro-project/hydroflow/commit/2d2c43dc001dbea17d46d73de464c95066b18fa2))
 * **[#1140](https://github.com/hydro-project/hydroflow/issues/1140)**
    - Appease latest nightly clippy ([`fc447ff`](https://github.com/hydro-project/hydroflow/commit/fc447ffdf8fd1b2189545a991f08588238182f00))
 * **Uncategorized**
    - Release hydroflow_plus v0.6.1, hydro_deploy v0.6.1, hydro_cli v0.6.1, hydroflow_plus_cli_integration v0.6.1 ([`c385c13`](https://github.com/hydro-project/hydroflow/commit/c385c132c9733d1bace82156aa14216b8e7fef9f))
    - Release hydroflow_lang v0.6.2, hydroflow v0.6.2, hydroflow_plus v0.6.1, hydro_deploy v0.6.1, hydro_cli v0.6.1, hydroflow_plus_cli_integration v0.6.1, stageleft_tool v0.1.1 ([`23cfe08`](https://github.com/hydro-project/hydroflow/commit/23cfe0839079aa17d042bbd3976f6d188689d290))
    - Release hydroflow_cli_integration v0.5.2, hydroflow_lang v0.6.1, hydroflow_datalog_core v0.6.1, lattices v0.5.4, hydroflow v0.6.1, stageleft_macro v0.1.1, stageleft v0.2.1, hydroflow_plus v0.6.1, hydro_deploy v0.6.1, hydro_cli v0.6.1, hydroflow_plus_cli_integration v0.6.1, stageleft_tool v0.1.1 ([`cd63f22`](https://github.com/hydro-project/hydroflow/commit/cd63f2258c961a40f0e5dbef20ac329a2d570ad0))
</details>

## v0.6.0 (2024-03-02)

<csr-id-39ab8b0278e9e3fe96552ace0a4ae768a6bc10d8/>

### Chore

 - <csr-id-39ab8b0278e9e3fe96552ace0a4ae768a6bc10d8/> appease various clippy lints

### New Features

 - <csr-id-c1d1b51ee26cc9946af59ac02c040e0a33d15fde/> unify send/demux/tagged APIs
   feat(hydroflow_plus): unify send/demux/tagged APIs
 - <csr-id-eb34ccd13f56e1d07cbae35ead79daeb3b9bad20/> use an IR before lowering to Hydroflow
   Makes it possible to write custom optimization passes.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 3 unique issues were worked on: [#1070](https://github.com/hydro-project/hydroflow/issues/1070), [#1080](https://github.com/hydro-project/hydroflow/issues/1080), [#1084](https://github.com/hydro-project/hydroflow/issues/1084)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1070](https://github.com/hydro-project/hydroflow/issues/1070)**
    - Use an IR before lowering to Hydroflow ([`eb34ccd`](https://github.com/hydro-project/hydroflow/commit/eb34ccd13f56e1d07cbae35ead79daeb3b9bad20))
 * **[#1080](https://github.com/hydro-project/hydroflow/issues/1080)**
    - Unify send/demux/tagged APIs ([`c1d1b51`](https://github.com/hydro-project/hydroflow/commit/c1d1b51ee26cc9946af59ac02c040e0a33d15fde))
 * **[#1084](https://github.com/hydro-project/hydroflow/issues/1084)**
    - Appease various clippy lints ([`39ab8b0`](https://github.com/hydro-project/hydroflow/commit/39ab8b0278e9e3fe96552ace0a4ae768a6bc10d8))
 * **Uncategorized**
    - Release hydroflow_lang v0.6.0, hydroflow_datalog_core v0.6.0, hydroflow_datalog v0.6.0, hydroflow_macro v0.6.0, lattices v0.5.3, variadics v0.0.4, pusherator v0.0.5, hydroflow v0.6.0, stageleft v0.2.0, hydroflow_plus v0.6.0, hydro_deploy v0.6.0, hydro_cli v0.6.0, hydroflow_plus_cli_integration v0.6.0, safety bump 7 crates ([`09ea65f`](https://github.com/hydro-project/hydroflow/commit/09ea65fe9cd45c357c43bffca30e60243fa45cc8))
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

 - <csr-id-5a03ed41548b5766b945efbd1eedb0dfceb714d9/> add core negation operators
 - <csr-id-7d930a2ccf656d3d6bc5db3e22eb63c5fd6d37d1/> add APIs for declaring external ports on clusters
 - <csr-id-73e9b68ec2f5b2627784addcce9fba684848bb55/> implement keyed fold and reduce
 - <csr-id-5e6ebac1a7f128227ae92a8c195235b27532e17a/> add interleaved shortcut when sending from a cluster
 - <csr-id-af6e3be60fdb69ceec1613347910f4dd49980d34/> push down persists and implement Pi example
   Also fixes type inference issues with reduce the same way as we did for fold.
 - <csr-id-6eeb9be9bc4136041a2855f650ae640c478b7fc9/> improve API naming and polish docs
 - <csr-id-44a308f77bddd67b5c51723ac39f3bc10af52553/> tweak naming of windowing operators
 - <csr-id-1edc5ae5b5f70e1390183e8c8eb27eb0ab32196d/> provide simpler API for launching and minimize dependencies
 - <csr-id-b7aafd3c97897db4bff62c4ab0b7480ef9a799e0/> improve API naming and eliminate wire API for builders
 - <csr-id-d288e51f980577510bb2ed45c04554102c4f1e14/> split API for building single-node graphs
 - <csr-id-26f4d6f610b78a75c41b1ae63366d089ad08b322/> require explicit batching for aggregation operators
 - <csr-id-174607d12277d7544d0f42890c9a5da2ff184df4/> support building graphs for symmetric clusters in Hydroflow+
 - <csr-id-9e275824c88b24d060a7de5822e1359959b36b03/> auto-configure Hydro Deploy based on Hydroflow+ plans
 - <csr-id-27dabcf6878576dc3675788ce3381cb25116033a/> add preliminary `send_to` operator for multi-node graphs
 - <csr-id-e5bdd12e32d6ea72fd91a55c12e09f07a0edaa5c/> add initial test using Hydro CLI from Hydroflow+
   This also required a change to Hydroflow core to make it possible to run the dataflow itself on a single thread (using a LocalSet), even if the surrounding runtime is not single-threaded (required to work around deadlocks because we can't use async APIs inside Hydroflow+). This requires us to spawn any Hydroflow tasks (only for `dest_sink` at the moment) right next to when we run the dataflow rather than when the Hydroflow graph is initialized. From a conceptual perspective, this seems _more right_, since now creating a Hydroflow program will not result in any actual tasks running.
   
   In the third PR of this series, I aim to add a new Hydroflow+ operator that will automate the setup of a `dest_sink`/`source_stream` pair that span nodes.
 - <csr-id-05fb1353cf3e0e8c5da9522365150bd78bd3c5f8/> allow Hydroflow+ programs to emit multiple graphs
   This PR adds support for tagging elements of Hydroflow+ graphs with a node ID, an integer which specifies which Hydroflow graph the computation should be emitted to. The generated code includes the Hydroflow graph for each node ID, so that the appropriate graph can be selected at runtime.
   
   At a larger scale, this is a precursor to adding network operators to Hydroflow+, which will allow distributed logic to be described in a single Hydroflow+ program by specifying points at which data is transferred between different graphs.
 - <csr-id-8b635683e5ac3c4ed2d896ae88e2953db1c6312c/> add a functional surface syntax using staging

### Bug Fixes

 - <csr-id-88a17967d0c9e681a04de4b5796f532f4833272c/> persist cluster IDs for broadcast
   I'll follow this up with a unit test for this, but want to get this fixed ASAP first.
 - <csr-id-bd2bf233302e3638c8f4bc9c0460e1a47edc00aa/> rewrite uses of alloc crate in bincode operators
 - <csr-id-2addaed8a8a441bff7acf9a0a265cc09483fd487/> disallow joining streams on different nodes
 - <csr-id-38411ea007d4feb30dd16bdd1505802a111a67d1/> fix spelling of "propagate"

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 25 commits contributed to the release.
 - 23 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 20 unique issues were worked on: [#1001](https://github.com/hydro-project/hydroflow/issues/1001), [#1003](https://github.com/hydro-project/hydroflow/issues/1003), [#1004](https://github.com/hydro-project/hydroflow/issues/1004), [#1006](https://github.com/hydro-project/hydroflow/issues/1006), [#1013](https://github.com/hydro-project/hydroflow/issues/1013), [#1021](https://github.com/hydro-project/hydroflow/issues/1021), [#1022](https://github.com/hydro-project/hydroflow/issues/1022), [#1023](https://github.com/hydro-project/hydroflow/issues/1023), [#1035](https://github.com/hydro-project/hydroflow/issues/1035), [#1036](https://github.com/hydro-project/hydroflow/issues/1036), [#899](https://github.com/hydro-project/hydroflow/issues/899), [#976](https://github.com/hydro-project/hydroflow/issues/976), [#978](https://github.com/hydro-project/hydroflow/issues/978), [#981](https://github.com/hydro-project/hydroflow/issues/981), [#982](https://github.com/hydro-project/hydroflow/issues/982), [#984](https://github.com/hydro-project/hydroflow/issues/984), [#989](https://github.com/hydro-project/hydroflow/issues/989), [#991](https://github.com/hydro-project/hydroflow/issues/991), [#993](https://github.com/hydro-project/hydroflow/issues/993), [#995](https://github.com/hydro-project/hydroflow/issues/995)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1001](https://github.com/hydro-project/hydroflow/issues/1001)**
    - Disallow joining streams on different nodes ([`2addaed`](https://github.com/hydro-project/hydroflow/commit/2addaed8a8a441bff7acf9a0a265cc09483fd487))
 * **[#1003](https://github.com/hydro-project/hydroflow/issues/1003)**
    - Provide simpler API for launching and minimize dependencies ([`1edc5ae`](https://github.com/hydro-project/hydroflow/commit/1edc5ae5b5f70e1390183e8c8eb27eb0ab32196d))
 * **[#1004](https://github.com/hydro-project/hydroflow/issues/1004)**
    - Rewrite uses of alloc crate in bincode operators ([`bd2bf23`](https://github.com/hydro-project/hydroflow/commit/bd2bf233302e3638c8f4bc9c0460e1a47edc00aa))
 * **[#1006](https://github.com/hydro-project/hydroflow/issues/1006)**
    - Tweak naming of windowing operators ([`44a308f`](https://github.com/hydro-project/hydroflow/commit/44a308f77bddd67b5c51723ac39f3bc10af52553))
 * **[#1013](https://github.com/hydro-project/hydroflow/issues/1013)**
    - Improve API naming and polish docs ([`6eeb9be`](https://github.com/hydro-project/hydroflow/commit/6eeb9be9bc4136041a2855f650ae640c478b7fc9))
 * **[#1021](https://github.com/hydro-project/hydroflow/issues/1021)**
    - Push down persists and implement Pi example ([`af6e3be`](https://github.com/hydro-project/hydroflow/commit/af6e3be60fdb69ceec1613347910f4dd49980d34))
 * **[#1022](https://github.com/hydro-project/hydroflow/issues/1022)**
    - Add interleaved shortcut when sending from a cluster ([`5e6ebac`](https://github.com/hydro-project/hydroflow/commit/5e6ebac1a7f128227ae92a8c195235b27532e17a))
 * **[#1023](https://github.com/hydro-project/hydroflow/issues/1023)**
    - Implement keyed fold and reduce ([`73e9b68`](https://github.com/hydro-project/hydroflow/commit/73e9b68ec2f5b2627784addcce9fba684848bb55))
 * **[#1035](https://github.com/hydro-project/hydroflow/issues/1035)**
    - Persist cluster IDs for broadcast ([`88a1796`](https://github.com/hydro-project/hydroflow/commit/88a17967d0c9e681a04de4b5796f532f4833272c))
 * **[#1036](https://github.com/hydro-project/hydroflow/issues/1036)**
    - Add core negation operators ([`5a03ed4`](https://github.com/hydro-project/hydroflow/commit/5a03ed41548b5766b945efbd1eedb0dfceb714d9))
 * **[#899](https://github.com/hydro-project/hydroflow/issues/899)**
    - Add a functional surface syntax using staging ([`8b63568`](https://github.com/hydro-project/hydroflow/commit/8b635683e5ac3c4ed2d896ae88e2953db1c6312c))
 * **[#976](https://github.com/hydro-project/hydroflow/issues/976)**
    - Allow Hydroflow+ programs to emit multiple graphs ([`05fb135`](https://github.com/hydro-project/hydroflow/commit/05fb1353cf3e0e8c5da9522365150bd78bd3c5f8))
 * **[#978](https://github.com/hydro-project/hydroflow/issues/978)**
    - Add initial test using Hydro CLI from Hydroflow+ ([`e5bdd12`](https://github.com/hydro-project/hydroflow/commit/e5bdd12e32d6ea72fd91a55c12e09f07a0edaa5c))
 * **[#981](https://github.com/hydro-project/hydroflow/issues/981)**
    - Add preliminary `send_to` operator for multi-node graphs ([`27dabcf`](https://github.com/hydro-project/hydroflow/commit/27dabcf6878576dc3675788ce3381cb25116033a))
 * **[#982](https://github.com/hydro-project/hydroflow/issues/982)**
    - Auto-configure Hydro Deploy based on Hydroflow+ plans ([`9e27582`](https://github.com/hydro-project/hydroflow/commit/9e275824c88b24d060a7de5822e1359959b36b03))
 * **[#984](https://github.com/hydro-project/hydroflow/issues/984)**
    - Support building graphs for symmetric clusters in Hydroflow+ ([`174607d`](https://github.com/hydro-project/hydroflow/commit/174607d12277d7544d0f42890c9a5da2ff184df4))
 * **[#989](https://github.com/hydro-project/hydroflow/issues/989)**
    - Fix spelling of "propagate" ([`38411ea`](https://github.com/hydro-project/hydroflow/commit/38411ea007d4feb30dd16bdd1505802a111a67d1))
 * **[#991](https://github.com/hydro-project/hydroflow/issues/991)**
    - Require explicit batching for aggregation operators ([`26f4d6f`](https://github.com/hydro-project/hydroflow/commit/26f4d6f610b78a75c41b1ae63366d089ad08b322))
 * **[#993](https://github.com/hydro-project/hydroflow/issues/993)**
    - Split API for building single-node graphs ([`d288e51`](https://github.com/hydro-project/hydroflow/commit/d288e51f980577510bb2ed45c04554102c4f1e14))
 * **[#995](https://github.com/hydro-project/hydroflow/issues/995)**
    - Improve API naming and eliminate wire API for builders ([`b7aafd3`](https://github.com/hydro-project/hydroflow/commit/b7aafd3c97897db4bff62c4ab0b7480ef9a799e0))
 * **Uncategorized**
    - Release hydroflow_plus v0.5.1 ([`58d1d71`](https://github.com/hydro-project/hydroflow/commit/58d1d7166f026a8c7a08a23bc1d77045d7e5f2a9))
    - Release stageleft_macro v0.1.0, stageleft v0.1.0, hydroflow_plus v0.5.1 ([`1a48db5`](https://github.com/hydro-project/hydroflow/commit/1a48db5a1ba058a718ac777367bf6eba3a236b7c))
    - Actually committing empty CHANGELOG.md is required ([`3b36020`](https://github.com/hydro-project/hydroflow/commit/3b36020d16792f26da4df3c5b09652a4ab47ec4f))
    - Manually set lockstep-versioned crates (and `lattices`) to version `0.5.1` ([`1b555e5`](https://github.com/hydro-project/hydroflow/commit/1b555e57c8c812bed4d6495d2960cbf77fb0b3ef))
    - Add APIs for declaring external ports on clusters ([`7d930a2`](https://github.com/hydro-project/hydroflow/commit/7d930a2ccf656d3d6bc5db3e22eb63c5fd6d37d1))
</details>

