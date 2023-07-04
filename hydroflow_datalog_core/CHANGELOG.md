# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.3.0 (2023-07-04)

### New Features

 - <csr-id-22abcaff806c7de6e4a7725656bbcf201e7d9259/> allow stable build, refactors behind `nightly` feature flag

### Bug Fixes

 - <csr-id-8d3494b5afee858114a602a3e23077bb6d24dd77/> update proc-macro2, use new span location API where possible
   requires latest* rust nightly version
   
   *latest = 2023-06-28 or something

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release over the course of 13 calendar days.
 - 33 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#780](https://github.com/hydro-project/hydroflow/issues/780), [#801](https://github.com/hydro-project/hydroflow/issues/801)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#780](https://github.com/hydro-project/hydroflow/issues/780)**
    - Allow stable build, refactors behind `nightly` feature flag ([`22abcaf`](https://github.com/hydro-project/hydroflow/commit/22abcaff806c7de6e4a7725656bbcf201e7d9259))
 * **[#801](https://github.com/hydro-project/hydroflow/issues/801)**
    - Update proc-macro2, use new span location API where possible ([`8d3494b`](https://github.com/hydro-project/hydroflow/commit/8d3494b5afee858114a602a3e23077bb6d24dd77))
</details>

## 0.2.0 (2023-05-31)

<csr-id-fd896fbe925fbd8ef1d16be7206ac20ba585081a/>

### Chore

 - <csr-id-fd896fbe925fbd8ef1d16be7206ac20ba585081a/> manually bump versions for v0.2.0 release

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 1 day passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release hydroflow_lang v0.2.0, hydroflow_datalog_core v0.2.0, hydroflow_datalog v0.2.0, hydroflow_macro v0.2.0, lattices v0.2.0, hydroflow v0.2.0, hydro_cli v0.2.0 ([`ca464c3`](https://github.com/hydro-project/hydroflow/commit/ca464c32322a7ad39eb53e1794777c849aa548a0))
    - Manually bump versions for v0.2.0 release ([`fd896fb`](https://github.com/hydro-project/hydroflow/commit/fd896fbe925fbd8ef1d16be7206ac20ba585081a))
</details>

## 0.1.1 (2023-05-30)

<csr-id-d574cb2661ba086059ba8cd6904fd6b6b0a5a8cb/>
<csr-id-d13a01b3a3fa0c52381833f88bcadac7a4ebcda9/>
<csr-id-2843e7e114ac824a684a5400909819ccc5c88fe3/>

### Bug Fixes

 - <csr-id-075c99e7cdcf40ae5cab9efa787ba4447db8a479/> fix `persist` releasing multiple times during the same tick
   Add surface_double_handoff tests

### Other

 - <csr-id-d574cb2661ba086059ba8cd6904fd6b6b0a5a8cb/> merge() to union()

### Refactor

 - <csr-id-d13a01b3a3fa0c52381833f88bcadac7a4ebcda9/> add spin(), remove repeat_iter,repeat_iter_external
   * refactor: add spin(), remove repeat_iter,repeat_iter_external
   
   * fix: fix lints
 - <csr-id-2843e7e114ac824a684a5400909819ccc5c88fe3/> Suffixes and remove keyed fold
   * rename: keyed_fold/keyed_reduce -> fold_keyed/reduce_keyed
   
   * remove group_by

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release.
 - 6 days passed between releases.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 4 unique issues were worked on: [#697](https://github.com/hydro-project/hydroflow/issues/697), [#702](https://github.com/hydro-project/hydroflow/issues/702), [#714](https://github.com/hydro-project/hydroflow/issues/714), [#716](https://github.com/hydro-project/hydroflow/issues/716)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#697](https://github.com/hydro-project/hydroflow/issues/697)**
    - Merge() to union() ([`d574cb2`](https://github.com/hydro-project/hydroflow/commit/d574cb2661ba086059ba8cd6904fd6b6b0a5a8cb))
 * **[#702](https://github.com/hydro-project/hydroflow/issues/702)**
    - Suffixes and remove keyed fold ([`2843e7e`](https://github.com/hydro-project/hydroflow/commit/2843e7e114ac824a684a5400909819ccc5c88fe3))
 * **[#714](https://github.com/hydro-project/hydroflow/issues/714)**
    - Add spin(), remove repeat_iter,repeat_iter_external ([`d13a01b`](https://github.com/hydro-project/hydroflow/commit/d13a01b3a3fa0c52381833f88bcadac7a4ebcda9))
 * **[#716](https://github.com/hydro-project/hydroflow/issues/716)**
    - Fix `persist` releasing multiple times during the same tick ([`075c99e`](https://github.com/hydro-project/hydroflow/commit/075c99e7cdcf40ae5cab9efa787ba4447db8a479))
 * **Uncategorized**
    - Release hydroflow_cli_integration v0.1.1, hydroflow_lang v0.1.1, hydroflow_datalog_core v0.1.1, hydroflow_macro v0.1.1, lattices v0.1.2, hydroflow v0.1.1, hydro_cli v0.1.0 ([`d9fa8b3`](https://github.com/hydro-project/hydroflow/commit/d9fa8b387e303b33d9614dbde80abf1af08bd8eb))
</details>

## 0.1.0 (2023-05-23)

<csr-id-52ee8f8e443f0a8b5caf92d2c5f028c00302a79b/>
<csr-id-faab58f855e4d6f2ad885c6f39f57ebc5662ec20/>

### Chore

 - <csr-id-52ee8f8e443f0a8b5caf92d2c5f028c00302a79b/> bump versions to 0.1.0 for release
   For release on crates.io for v0.1

### Refactor

 - <csr-id-faab58f855e4d6f2ad885c6f39f57ebc5662ec20/> remove `hydroflow::lang` module, move `Clear`, `MonotonicMap` to `hydroflow::util` instead

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 2 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 3 unique issues were worked on: [#673](https://github.com/hydro-project/hydroflow/issues/673), [#677](https://github.com/hydro-project/hydroflow/issues/677), [#684](https://github.com/hydro-project/hydroflow/issues/684)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#673](https://github.com/hydro-project/hydroflow/issues/673)**
    - Don't box source_stream argument unnecessarily ([`dc37cba`](https://github.com/hydro-project/hydroflow/commit/dc37cba9512b47bbc98bbc84e3594817eca9bace))
 * **[#677](https://github.com/hydro-project/hydroflow/issues/677)**
    - Remove `hydroflow::lang` module, move `Clear`, `MonotonicMap` to `hydroflow::util` instead ([`faab58f`](https://github.com/hydro-project/hydroflow/commit/faab58f855e4d6f2ad885c6f39f57ebc5662ec20))
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

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release over the course of 17 calendar days.
 - 24 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 3 unique issues were worked on: [#639](https://github.com/hydro-project/hydroflow/issues/639), [#642](https://github.com/hydro-project/hydroflow/issues/642), [#660](https://github.com/hydro-project/hydroflow/issues/660)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#639](https://github.com/hydro-project/hydroflow/issues/639)**
    - Update pinned nightly to `nightly-2023-05-03` ([`f0afb56`](https://github.com/hydro-project/hydroflow/commit/f0afb56a069f6aa40c4f9eee131408b32a17d83c))
 * **[#642](https://github.com/hydro-project/hydroflow/issues/642)**
    - Remove zmq, use unsync channels locally, use sync mpsc cross-thread, use cross_join+enumerate instead of broadcast channel,remove Eq requirement from multisetjoin ([`b38f5cf`](https://github.com/hydro-project/hydroflow/commit/b38f5cf198e29a8de2f84eb4cd075818fbeffda6))
 * **[#660](https://github.com/hydro-project/hydroflow/issues/660)**
    - Warn lint `unused_qualifications` ([`cd0a86d`](https://github.com/hydro-project/hydroflow/commit/cd0a86d9271d0e3daab59c46f079925f863424e1))
    - Rustfmt group imports ([`20a1b2c`](https://github.com/hydro-project/hydroflow/commit/20a1b2c0cd04a8b495a02ce345db3d48a99ea0e9))
    - Rustfmt prescribe flat-module `use` format ([`1eda91a`](https://github.com/hydro-project/hydroflow/commit/1eda91a2ef8794711ef037240f15284e8085d863))
 * **Uncategorized**
    - Release hydroflow_cli_integration v0.0.1, hydroflow_lang v0.0.1, hydroflow_datalog_core v0.0.1, hydroflow_datalog v0.0.1, hydroflow_macro v0.0.1, lattices v0.1.0, variadics v0.0.2, pusherator v0.0.1, hydroflow v0.0.2 ([`809395a`](https://github.com/hydro-project/hydroflow/commit/809395acddb78949d7a2bf036e1a94972f23b1ad))
</details>

## 0.0.0 (2023-04-26)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 67 commits contributed to the release over the course of 83 calendar days.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 59 unique issues were worked on: [#209](https://github.com/hydro-project/hydroflow/issues/209), [#254](https://github.com/hydro-project/hydroflow/issues/254), [#329](https://github.com/hydro-project/hydroflow/issues/329), [#371](https://github.com/hydro-project/hydroflow/issues/371), [#374](https://github.com/hydro-project/hydroflow/issues/374), [#376](https://github.com/hydro-project/hydroflow/issues/376), [#383](https://github.com/hydro-project/hydroflow/issues/383), [#388](https://github.com/hydro-project/hydroflow/issues/388), [#403](https://github.com/hydro-project/hydroflow/issues/403), [#413](https://github.com/hydro-project/hydroflow/issues/413), [#419](https://github.com/hydro-project/hydroflow/issues/419), [#425](https://github.com/hydro-project/hydroflow/issues/425), [#431](https://github.com/hydro-project/hydroflow/issues/431), [#441 11/14](https://github.com/hydro-project/hydroflow/issues/441 11/14), [#441 12/14](https://github.com/hydro-project/hydroflow/issues/441 12/14), [#441 14/14](https://github.com/hydro-project/hydroflow/issues/441 14/14), [#442](https://github.com/hydro-project/hydroflow/issues/442), [#443](https://github.com/hydro-project/hydroflow/issues/443), [#444](https://github.com/hydro-project/hydroflow/issues/444), [#455](https://github.com/hydro-project/hydroflow/issues/455), [#459](https://github.com/hydro-project/hydroflow/issues/459), [#467](https://github.com/hydro-project/hydroflow/issues/467), [#469](https://github.com/hydro-project/hydroflow/issues/469), [#470](https://github.com/hydro-project/hydroflow/issues/470), [#475](https://github.com/hydro-project/hydroflow/issues/475), [#486](https://github.com/hydro-project/hydroflow/issues/486), [#499](https://github.com/hydro-project/hydroflow/issues/499), [#500](https://github.com/hydro-project/hydroflow/issues/500), [#501](https://github.com/hydro-project/hydroflow/issues/501), [#502](https://github.com/hydro-project/hydroflow/issues/502), [#505](https://github.com/hydro-project/hydroflow/issues/505), [#507](https://github.com/hydro-project/hydroflow/issues/507), [#509](https://github.com/hydro-project/hydroflow/issues/509), [#516](https://github.com/hydro-project/hydroflow/issues/516), [#518](https://github.com/hydro-project/hydroflow/issues/518), [#522](https://github.com/hydro-project/hydroflow/issues/522), [#540](https://github.com/hydro-project/hydroflow/issues/540), [#543](https://github.com/hydro-project/hydroflow/issues/543), [#547](https://github.com/hydro-project/hydroflow/issues/547), [#549](https://github.com/hydro-project/hydroflow/issues/549), [#551](https://github.com/hydro-project/hydroflow/issues/551), [#554](https://github.com/hydro-project/hydroflow/issues/554), [#555](https://github.com/hydro-project/hydroflow/issues/555), [#556](https://github.com/hydro-project/hydroflow/issues/556), [#558](https://github.com/hydro-project/hydroflow/issues/558), [#559](https://github.com/hydro-project/hydroflow/issues/559), [#565](https://github.com/hydro-project/hydroflow/issues/565), [#566](https://github.com/hydro-project/hydroflow/issues/566), [#567](https://github.com/hydro-project/hydroflow/issues/567), [#568](https://github.com/hydro-project/hydroflow/issues/568), [#571](https://github.com/hydro-project/hydroflow/issues/571), [#572](https://github.com/hydro-project/hydroflow/issues/572), [#573](https://github.com/hydro-project/hydroflow/issues/573), [#576](https://github.com/hydro-project/hydroflow/issues/576), [#598](https://github.com/hydro-project/hydroflow/issues/598), [#604](https://github.com/hydro-project/hydroflow/issues/604), [#609](https://github.com/hydro-project/hydroflow/issues/609), [#617](https://github.com/hydro-project/hydroflow/issues/617), [#618](https://github.com/hydro-project/hydroflow/issues/618)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#209](https://github.com/hydro-project/hydroflow/issues/209)**
    - Add filtering expressions to Dedalus rules. #178 ([`7462cc3`](https://github.com/hydro-project/hydroflow/commit/7462cc35e4953e7740a512285dc70ad8628eff6c))
 * **[#254](https://github.com/hydro-project/hydroflow/issues/254)**
    - Dedalus support for count, sum, choose, and comments. ([`e28c8d6`](https://github.com/hydro-project/hydroflow/commit/e28c8d694ced16212ece5bef0fe485853b210096))
 * **[#329](https://github.com/hydro-project/hydroflow/issues/329)**
    - Get hydroflow to compile to WASM ([`24354d2`](https://github.com/hydro-project/hydroflow/commit/24354d2e11c69e38e4e021aa4acf1525b376b2b1))
 * **[#371](https://github.com/hydro-project/hydroflow/issues/371)**
    - Get Datalog compiler to build on WASM ([`bef2435`](https://github.com/hydro-project/hydroflow/commit/bef24356a9696b494f89e014aec49063892b5b5e))
 * **[#374](https://github.com/hydro-project/hydroflow/issues/374)**
    - Support Dedalus rules that send results to the next tick ([`5f58f31`](https://github.com/hydro-project/hydroflow/commit/5f58f3168dbfe9d59e6b543407b6c0defd3a0b44))
 * **[#376](https://github.com/hydro-project/hydroflow/issues/376)**
    - Joins in Dedalus should only have the 'tick lifetime ([`962d903`](https://github.com/hydro-project/hydroflow/commit/962d9038490f32b1dccdd09066720c2d6ef86841))
 * **[#383](https://github.com/hydro-project/hydroflow/issues/383)**
    - Allow alias name assignment without any arrow in surface syntax, closes #266 ([`9d17b4d`](https://github.com/hydro-project/hydroflow/commit/9d17b4d5da37efcde633a87cf489541cb5371555))
 * **[#388](https://github.com/hydro-project/hydroflow/issues/388)**
    - Add support for negated relations to Dedalus ([`dc870e8`](https://github.com/hydro-project/hydroflow/commit/dc870e8c2775c352444d47ca063ff561fffda078))
 * **[#403](https://github.com/hydro-project/hydroflow/issues/403)**
    - Implement simple aggregations for Dedalus ([`f80754e`](https://github.com/hydro-project/hydroflow/commit/f80754e476d979e271ffa30cda1de7fc24c5ccde))
 * **[#413](https://github.com/hydro-project/hydroflow/issues/413)**
    - Implement async rules in Dedalus by deferring to external sender ([`3d46bde`](https://github.com/hydro-project/hydroflow/commit/3d46bde98d49f07cdff3ea81a7f4d23ffd41cc2e))
 * **[#419](https://github.com/hydro-project/hydroflow/issues/419)**
    - Encapsulate `FlatGraph`, separate `FlatGraphBuilder` ([`fceaea5`](https://github.com/hydro-project/hydroflow/commit/fceaea5659ac76c2275c1487582a17b646858602))
 * **[#425](https://github.com/hydro-project/hydroflow/issues/425)**
    - Fix `FlatGraph::write_surface_syntax` ([`6f0c29a`](https://github.com/hydro-project/hydroflow/commit/6f0c29abf38f4ed892308cc18d2edcd1b44596a6))
 * **[#431](https://github.com/hydro-project/hydroflow/issues/431)**
    - Make `unique()` streaming and dedup Dedalus facts ([`68f9bde`](https://github.com/hydro-project/hydroflow/commit/68f9bde464122c41fab3a75897137d46be3bee38))
 * **[#441 11/14](https://github.com/hydro-project/hydroflow/issues/441 11/14)**
    - Remove `FlatGraph`, unify under `PartitionedGraph` ([`b640b53`](https://github.com/hydro-project/hydroflow/commit/b640b532e34b29f44c768d523fbf780dba9785ff))
 * **[#441 12/14](https://github.com/hydro-project/hydroflow/issues/441 12/14)**
    - Rename `PartitionedGraph` -> `HydroflowGraph` ([`f95b325`](https://github.com/hydro-project/hydroflow/commit/f95b325dafcd5574050563f62a94d89a2fa811c8))
 * **[#441 14/14](https://github.com/hydro-project/hydroflow/issues/441 14/14)**
    - Cleanup graph docs, organize method names ([`09d3b57`](https://github.com/hydro-project/hydroflow/commit/09d3b57eb03f3920bd10f5c10277d3ef4f9cb0ec))
 * **[#442](https://github.com/hydro-project/hydroflow/issues/442)**
    - Fixup! Require users to specify Hydroflow pipelines at the edges of a Dedalus program ([`83c888f`](https://github.com/hydro-project/hydroflow/commit/83c888f20c59b47c4cb6fbc7bb8cd82ecb0f5c7c))
    - Require users to specify Hydroflow pipelines at the edges of a Dedalus program ([`b107c47`](https://github.com/hydro-project/hydroflow/commit/b107c476a5a817516a5a756e4c6ca6084a78e251))
 * **[#443](https://github.com/hydro-project/hydroflow/issues/443)**
    - Add `.async` Dedalus directive to specify pipeline for inter-Dedalus messaging ([`748242c`](https://github.com/hydro-project/hydroflow/commit/748242c950edde42944c8b6ab9ebca3409406150))
 * **[#444](https://github.com/hydro-project/hydroflow/issues/444)**
    - Add snapshot testing of graph visualizations (mermaid and dot) ([`58a2438`](https://github.com/hydro-project/hydroflow/commit/58a24387c001cbda78ad87c7c2d0c2e2502b3099))
 * **[#455](https://github.com/hydro-project/hydroflow/issues/455)**
    - Add `source_stream(...)` type guard ([`f09227b`](https://github.com/hydro-project/hydroflow/commit/f09227b1890f3548122ec1c35e91fd7f573c8eda))
 * **[#459](https://github.com/hydro-project/hydroflow/issues/459)**
    - Fix coloring (pull vs push) error in serdegraph, recompute colors rather than serializing ([`86d5623`](https://github.com/hydro-project/hydroflow/commit/86d562316a99b0095d32e9a8e5218432396febbb))
 * **[#467](https://github.com/hydro-project/hydroflow/issues/467)**
    - Parse error and return vector of diagnostics ([`1841f2c`](https://github.com/hydro-project/hydroflow/commit/1841f2c462a132272b1f0ffac51669fc1df2f593))
 * **[#469](https://github.com/hydro-project/hydroflow/issues/469)**
    - Automatically clone intermediate values when used multiple times in Dedalus ([`26a1e55`](https://github.com/hydro-project/hydroflow/commit/26a1e557ac175406d5ccd73aabd3c10a00712f96))
 * **[#470](https://github.com/hydro-project/hydroflow/issues/470)**
    - Dedalus support for count, sum, choose, and comments. ([`e28c8d6`](https://github.com/hydro-project/hydroflow/commit/e28c8d694ced16212ece5bef0fe485853b210096))
 * **[#475](https://github.com/hydro-project/hydroflow/issues/475)**
    - Use prettyplease to prettify hydroflow graph output ([`323279a`](https://github.com/hydro-project/hydroflow/commit/323279ad2597b75119b5cb7979702c41fd7e6477))
 * **[#486](https://github.com/hydro-project/hydroflow/issues/486)**
    - Dedalus voting, 2pc implementation ([`c078ced`](https://github.com/hydro-project/hydroflow/commit/c078ced09dc79002a229ae1033459e5a729d0553))
 * **[#499](https://github.com/hydro-project/hydroflow/issues/499)**
    - Dontdrophandoffs ([`b603581`](https://github.com/hydro-project/hydroflow/commit/b603581b83423e161ccac53607022d6e4857fa71))
 * **[#500](https://github.com/hydro-project/hydroflow/issues/500)**
    - Add support for arithmetic expressions on LHS of Dedalus rules ([`82db672`](https://github.com/hydro-project/hydroflow/commit/82db6726ebb3c35cc2e67f313f758cef0e980c53))
 * **[#501](https://github.com/hydro-project/hydroflow/issues/501)**
    - Fixup! Preserve serialize diagnostics for hydroflow graph, stop emitting expected warnings in tests ([`8ebd5f5`](https://github.com/hydro-project/hydroflow/commit/8ebd5f5c7923879f72bbbe81cdc2148f2057d75a))
    - Preserve serialize diagnostics for hydroflow graph, stop emitting expected warnings in tests ([`0c810e5`](https://github.com/hydro-project/hydroflow/commit/0c810e5fdd3445923c0c7afbe651f2b4a72c115e))
 * **[#502](https://github.com/hydro-project/hydroflow/issues/502)**
    - Implement `less_than` magic relation in Dedalus ([`197070d`](https://github.com/hydro-project/hydroflow/commit/197070d2badcd854a9603c642f347fda466d2211))
 * **[#505](https://github.com/hydro-project/hydroflow/issues/505)**
    - Let Rust infer the integer type of Dedalus literals and fix aggregation lifetimes ([`2146c05`](https://github.com/hydro-project/hydroflow/commit/2146c0597ebd4b7adb40170be8c0200ae3f93e99))
 * **[#507](https://github.com/hydro-project/hydroflow/issues/507)**
    - Deduplicate facts that are being sent over the network ([`cc328e6`](https://github.com/hydro-project/hydroflow/commit/cc328e6df7046d61c30cb162efb64647ff4ce961))
 * **[#509](https://github.com/hydro-project/hydroflow/issues/509)**
    - Even faster groupby ([`af304aa`](https://github.com/hydro-project/hydroflow/commit/af304aa7ed35e6d5d7ed0936e3827de2b40e1ddb))
 * **[#516](https://github.com/hydro-project/hydroflow/issues/516)**
    - Update Rust Sitter to fix Dedalus parser on WASM ([`5a4f408`](https://github.com/hydro-project/hydroflow/commit/5a4f4084a357d329b3bf228f1e4113898917d90c))
 * **[#518](https://github.com/hydro-project/hydroflow/issues/518)**
    - Attach spans to generated Hydroflow code in Dedalus ([`f00d865`](https://github.com/hydro-project/hydroflow/commit/f00d8655aa4404ddcc812e0decf8c1e48e62b0fd))
 * **[#522](https://github.com/hydro-project/hydroflow/issues/522)**
    - Don't copy all elements of row being filtered in Dedalus ([`4fa677d`](https://github.com/hydro-project/hydroflow/commit/4fa677d0316d441eedad7660a7f8490dbbecfa61))
 * **[#540](https://github.com/hydro-project/hydroflow/issues/540)**
    - Support arithmetic expressions and literals in Datalog predicates ([`fc2eac3`](https://github.com/hydro-project/hydroflow/commit/fc2eac3478c1f32c8fb22b2eec63316b84203fba))
 * **[#543](https://github.com/hydro-project/hydroflow/issues/543)**
    - Add `.persist` annotation to opt into more efficient Dedalus persistence ([`95c7190`](https://github.com/hydro-project/hydroflow/commit/95c7190b851cd82a88e7c7f062e617774238be1e))
 * **[#547](https://github.com/hydro-project/hydroflow/issues/547)**
    - Add transform to remove extra `merge()`s and `tee()`s ([`838ac2a`](https://github.com/hydro-project/hydroflow/commit/838ac2a4d9a2e3ea1a4cdb5f8702c8d2b1eb3e5e))
 * **[#549](https://github.com/hydro-project/hydroflow/issues/549)**
    - Support `_` as a wildcard variable in Dedalus rules that is not joined on ([`633cc4f`](https://github.com/hydro-project/hydroflow/commit/633cc4f8f9d7818b5c5d64d1d7d80a7d1a51d7bf))
 * **[#551](https://github.com/hydro-project/hydroflow/issues/551)**
    - Add remainder operator to Dedalus expressions ([`f24b746`](https://github.com/hydro-project/hydroflow/commit/f24b7469bdeaa023197eba42fc3e6e1e3343bd49))
 * **[#554](https://github.com/hydro-project/hydroflow/issues/554)**
    - Use grammar to be more careful about Dedalus wildcard variables ([`0aa26b4`](https://github.com/hydro-project/hydroflow/commit/0aa26b4ea233b21dcc17ed8cca33e489483aaf08))
 * **[#555](https://github.com/hydro-project/hydroflow/issues/555)**
    - Antijoin uses FxHash instead of SipHash ([`55fa0a2`](https://github.com/hydro-project/hydroflow/commit/55fa0a2a733a482400e01edd495ef429a54ac555))
 * **[#556](https://github.com/hydro-project/hydroflow/issues/556)**
    - Unique uses FxHash instead of SipHash ([`4323d47`](https://github.com/hydro-project/hydroflow/commit/4323d47efc495940cc4bf41f647e4e187bf1305b))
 * **[#558](https://github.com/hydro-project/hydroflow/issues/558)**
    - Be more careful about the semantics of `count` and wildcard columns ([`ad4a665`](https://github.com/hydro-project/hydroflow/commit/ad4a66536ef6f1ea29535fa7162a4bbf129db999))
 * **[#559](https://github.com/hydro-project/hydroflow/issues/559)**
    - Add optional multiset join operator ([`c70644d`](https://github.com/hydro-project/hydroflow/commit/c70644ddb784449b55a84278cb1bf8cc38557d82))
 * **[#565](https://github.com/hydro-project/hydroflow/issues/565)**
    - Use multiset joins in Dedalus because we handle uniqueness separately ([`e3900ed`](https://github.com/hydro-project/hydroflow/commit/e3900edf125af8049762b4079db472b5d240e2b2))
 * **[#566](https://github.com/hydro-project/hydroflow/issues/566)**
    - Only filter out duplicate elements in one place for persisted relations ([`a37a511`](https://github.com/hydro-project/hydroflow/commit/a37a511c37fd362044b563268e95fdf152700acf))
 * **[#567](https://github.com/hydro-project/hydroflow/issues/567)**
    - Make aggregations of persisted relations incremental ([`6670256`](https://github.com/hydro-project/hydroflow/commit/6670256a30ac656adf46ced3ea385558357122e9))
 * **[#568](https://github.com/hydro-project/hydroflow/issues/568)**
    - Don't reify persists if the target relation is already a persisted one ([`61abfc1`](https://github.com/hydro-project/hydroflow/commit/61abfc18b5e52fc5a8ab63cbb920f670c52b7c0c))
 * **[#571](https://github.com/hydro-project/hydroflow/issues/571)**
    - Add multiplication operator and test behavior of aggregations with grouping expressions ([`b3e790c`](https://github.com/hydro-project/hydroflow/commit/b3e790c5d6cb5e0d420f13fe9ead7e6c43e527d4))
 * **[#572](https://github.com/hydro-project/hydroflow/issues/572)**
    - Allow the result of `count(column)` to be any inferred integer type rather than `usize` ([`89fac77`](https://github.com/hydro-project/hydroflow/commit/89fac77e13c9a610401d0a24e6ac862d10a88fe7))
 * **[#573](https://github.com/hydro-project/hydroflow/issues/573)**
    - Make profiles easier to interpret ([`d0e5df1`](https://github.com/hydro-project/hydroflow/commit/d0e5df13d5bc3dd4a986e70f2125978bd2878b96))
 * **[#576](https://github.com/hydro-project/hydroflow/issues/576)**
    - Add classic counter CRDT benchmark to compare against ([`2f3bf04`](https://github.com/hydro-project/hydroflow/commit/2f3bf04ab33768b04d44f3f58907f958d4cd8dc8))
 * **[#598](https://github.com/hydro-project/hydroflow/issues/598)**
    - Add `index()` operator for getting the index of the current group ([`6f959b6`](https://github.com/hydro-project/hydroflow/commit/6f959b64f0cf494c23f9ec8bc107a23e006aeacf))
 * **[#604](https://github.com/hydro-project/hydroflow/issues/604)**
    - Don't drop groupby hash table for 'tick lifetimes ([`cc1b762`](https://github.com/hydro-project/hydroflow/commit/cc1b762364dd66e496cdc766f8694bea256dd0d1))
 * **[#609](https://github.com/hydro-project/hydroflow/issues/609)**
    - Update syn to 2.0 ([`2e7d802`](https://github.com/hydro-project/hydroflow/commit/2e7d8024f35893ef0abcb6851e370b00615f9562))
 * **[#617](https://github.com/hydro-project/hydroflow/issues/617)**
    - Update `Cargo.toml`s for publishing ([`a78ff9a`](https://github.com/hydro-project/hydroflow/commit/a78ff9aace6771787c2b72aad83be6ad8d49a828))
 * **[#618](https://github.com/hydro-project/hydroflow/issues/618)**
    - Add static declarations to Dedalus ([`3a77e62`](https://github.com/hydro-project/hydroflow/commit/3a77e62e6006b846db9055385bd76c81e08f2f15))
 * **Uncategorized**
    - Setup release workflow ([`108d0e9`](https://github.com/hydro-project/hydroflow/commit/108d0e933a08b183c4dadf8c3499e4946696e263))
    - Use clear rather than default for join state #562 ([`c4f3f97`](https://github.com/hydro-project/hydroflow/commit/c4f3f97bab8a1cb5d3453290f567798b4bc4b60d))
    - Update rust-sitter for spanned leafs ([`debc6a0`](https://github.com/hydro-project/hydroflow/commit/debc6a0ff0c52f0bcb378f9a5790bae7092091fd))
    - Improve datalog diagnostic robustness ([`0b3e085`](https://github.com/hydro-project/hydroflow/commit/0b3e08521131989dfaee821c060a931771936f80))
    - Serialize `HydroflowGraph` instead of `SerdeGraph` ([`ae205c6`](https://github.com/hydro-project/hydroflow/commit/ae205c69538fab9eeedd8fa460b8eef295d26bc2))
    - Refactor `FlatGraph` assembly into separate `FlatGraphBuilder` ([`9dd3bd9`](https://github.com/hydro-project/hydroflow/commit/9dd3bd91586966484abaf01c4330d831804b1983))
    - Implement forward name references in surface syntax, closes #158 ([`8cc479e`](https://github.com/hydro-project/hydroflow/commit/8cc479ea99fd2a58751fc24f8b46d60e8594d24a))
</details>

