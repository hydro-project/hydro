# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.14.0 (2025-07-30)

### Chore

 - <csr-id-98baec71a6f1d01d55a3c983fdbb7824c45305cd/> update pinned nightly to 2025-04-27, update span API usage

### New Features

 - <csr-id-b58dfc899c67ee17a1818c484fa6cba7db3dd240/> add `scan` operator

### Bug Fixes

 - <csr-id-5b5bbe57b54a5d038bc28c0e674ed68ca34245d1/> Revert anti join allocation
   Added unit test for Paxos compilation and non-negative throughtput
 - <csr-id-0d841a536e1ab58838136e6c33b2115325ec1541/> add type arguments to `anti_join_multiset`, `difference_multiset` to mitigate #1857

### New Features (BREAKING)

 - <csr-id-d6ae619060339eb3dac5bec17d384430e3588093/> re-add loop lifetimes for anti_join_multiset, tests, remove MonotonicMap, fix #1830, fix #1823
   Redo of #1835
   
   Also updates path of trybuild errors to allow them to be clicked in the
   IDE
   
   ---
   
   Previous commit:
   
   Also implements loop lifetimes for `difference_multiset` which uses the
   `anti_join_multiset` codegen.
   
   Updates tests for `difference`, `difference_multiset`, `anti_join`, and
   `anti_join_multiset`
 - <csr-id-9bb9d1f3a0108a4789de6065af9e644c47601b9f/> display loops in graph visualizations, refactor, fix #1699
   Adds loops to display, new `GraphWrite.no_loops` option.
   
   Refactors how the heirarchy of `GraphWrite` items is handled to be
   simpler.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release over the course of 92 calendar days.
 - 6 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 6 unique issues were worked on: [#1843](https://github.com/hydro-project/hydro/issues/1843), [#1851](https://github.com/hydro-project/hydro/issues/1851), [#1858](https://github.com/hydro-project/hydro/issues/1858), [#1860](https://github.com/hydro-project/hydro/issues/1860), [#1911](https://github.com/hydro-project/hydro/issues/1911), [#1929](https://github.com/hydro-project/hydro/issues/1929)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1843](https://github.com/hydro-project/hydro/issues/1843)**
    - Update pinned nightly to 2025-04-27, update span API usage ([`98baec7`](https://github.com/hydro-project/hydro/commit/98baec71a6f1d01d55a3c983fdbb7824c45305cd))
 * **[#1851](https://github.com/hydro-project/hydro/issues/1851)**
    - Display loops in graph visualizations, refactor, fix #1699 ([`9bb9d1f`](https://github.com/hydro-project/hydro/commit/9bb9d1f3a0108a4789de6065af9e644c47601b9f))
 * **[#1858](https://github.com/hydro-project/hydro/issues/1858)**
    - Add type arguments to `anti_join_multiset`, `difference_multiset` to mitigate #1857 ([`0d841a5`](https://github.com/hydro-project/hydro/commit/0d841a536e1ab58838136e6c33b2115325ec1541))
 * **[#1860](https://github.com/hydro-project/hydro/issues/1860)**
    - Revert anti join allocation ([`5b5bbe5`](https://github.com/hydro-project/hydro/commit/5b5bbe57b54a5d038bc28c0e674ed68ca34245d1))
 * **[#1911](https://github.com/hydro-project/hydro/issues/1911)**
    - Re-add loop lifetimes for anti_join_multiset, tests, remove MonotonicMap, fix #1830, fix #1823 ([`d6ae619`](https://github.com/hydro-project/hydro/commit/d6ae619060339eb3dac5bec17d384430e3588093))
 * **[#1929](https://github.com/hydro-project/hydro/issues/1929)**
    - Add `scan` operator ([`b58dfc8`](https://github.com/hydro-project/hydro/commit/b58dfc899c67ee17a1818c484fa6cba7db3dd240))
</details>

## 0.13.0 (2025-04-11)

<csr-id-3aec2f739acd0a2305f99fcbde4c14bc1cd53e7a/>
<csr-id-7f3ec9dcce0ef9d52af03083970c8d26b9993fc0/>
<csr-id-2fdd5da1cf902a0c2f99cf8770bb48f9e046e38f/>

### New Features

 - <csr-id-127df13b7d7fc12d5265f1d7517f3b66a774dc07/> add resolve_futures and resolve_futures_ordered APIs

### Bug Fixes

 - <csr-id-40075b198b13cb6b4804633b76c96c520394fa71/> fix loop hooks triggered too often, implement lifetimes for `zip`
 - <csr-id-fbb5fab72c5a64a07653c9b6389186ad079703ec/> handle `-1` addresses from samply, fix `_counter()` rollover
   This fixes samply profiling on my "ancient" 2019 x86-64 macbook pro
   15.3.2 (24D81)
   
   This pull request aims to fix the handling of –1 address values from
   samply by updating tracing filenames and refactoring related error and
   type handling. Key changes include:
   - Better error messages when `dtrace` or `samply` are not instaled.
- Fix integer rollover in `_counter()` by using `u64` instead of
   inferred `i32`.
- Refactor samply profile conversion for asynchronous frame lookup.

### Other

 - <csr-id-3aec2f739acd0a2305f99fcbde4c14bc1cd53e7a/> adjust env vars, make DFIR printed span paths relative & re-enable most trybuild tests
   * Add `PYO3_USE_ABI3_FORWARD_COMPATIBILITY`, move
   `RUST_LOG`/`RUST_BACKTRACE` env vars to `.cargo/config.toml` (removed
   from `.vscode/settings.json`). Will now apply in general, so no need to
   specify everywhere.
   * Makes display of compiler spans' `source_file()` in `dfir_lang`
   relative to `DFIR_BASE_DIR` (new) or `CARGO_MANIFEST_DIR`, if possible.
   * Include `rust-src` `rustup` component to make trybuild error output
   more consistent:
   https://github.com/dtolnay/trybuild?tab=readme-ov-file#troubleshooting
   * Almost all `dfir_rs` trybuild test are re-enabled (except a couple
   obsolete ones are deleted, and two have inconsistent printing)
   * Updates docs build to remove old operators
 - <csr-id-7f3ec9dcce0ef9d52af03083970c8d26b9993fc0/> fix `docsrs` broken stable test, actually run `dfir_lang` op doc tests

### New Features (BREAKING)

 - <csr-id-fbaab5b12c0c661ee08d8ded6862a38834ba62ae/> loop lifetimes for `anti_join_multiset`, tests, remove `MonotonicMap`, fix #1830, fix #1823
   Also implements loop lifetimes for `difference_multiset` which uses the
   `anti_join_multiset` codegen.
   
   Updates tests for `difference`, `difference_multiset`, `anti_join`, and
   `anti_join_multiset`

### Refactor (BREAKING)

 - <csr-id-2fdd5da1cf902a0c2f99cf8770bb48f9e046e38f/> loop machinery, add `'loop` lifetimes, fix #1745

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 8 commits contributed to the release.
 - 27 days passed between releases.
 - 7 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 7 unique issues were worked on: [#1741](https://github.com/hydro-project/hydro/issues/1741), [#1790](https://github.com/hydro-project/hydro/issues/1790), [#1795](https://github.com/hydro-project/hydro/issues/1795), [#1814](https://github.com/hydro-project/hydro/issues/1814), [#1822](https://github.com/hydro-project/hydro/issues/1822), [#1833](https://github.com/hydro-project/hydro/issues/1833), [#1835](https://github.com/hydro-project/hydro/issues/1835)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1741](https://github.com/hydro-project/hydro/issues/1741)**
    - Add resolve_futures and resolve_futures_ordered APIs ([`127df13`](https://github.com/hydro-project/hydro/commit/127df13b7d7fc12d5265f1d7517f3b66a774dc07))
 * **[#1790](https://github.com/hydro-project/hydro/issues/1790)**
    - Fix `docsrs` broken stable test, actually run `dfir_lang` op doc tests ([`7f3ec9d`](https://github.com/hydro-project/hydro/commit/7f3ec9dcce0ef9d52af03083970c8d26b9993fc0))
 * **[#1795](https://github.com/hydro-project/hydro/issues/1795)**
    - Adjust env vars, make DFIR printed span paths relative & re-enable most trybuild tests ([`3aec2f7`](https://github.com/hydro-project/hydro/commit/3aec2f739acd0a2305f99fcbde4c14bc1cd53e7a))
 * **[#1814](https://github.com/hydro-project/hydro/issues/1814)**
    - Handle `-1` addresses from samply, fix `_counter()` rollover ([`fbb5fab`](https://github.com/hydro-project/hydro/commit/fbb5fab72c5a64a07653c9b6389186ad079703ec))
 * **[#1822](https://github.com/hydro-project/hydro/issues/1822)**
    - Loop machinery, add `'loop` lifetimes, fix #1745 ([`2fdd5da`](https://github.com/hydro-project/hydro/commit/2fdd5da1cf902a0c2f99cf8770bb48f9e046e38f))
 * **[#1833](https://github.com/hydro-project/hydro/issues/1833)**
    - Fix loop hooks triggered too often, implement lifetimes for `zip` ([`40075b1`](https://github.com/hydro-project/hydro/commit/40075b198b13cb6b4804633b76c96c520394fa71))
 * **[#1835](https://github.com/hydro-project/hydro/issues/1835)**
    - Loop lifetimes for `anti_join_multiset`, tests, remove `MonotonicMap`, fix #1830, fix #1823 ([`fbaab5b`](https://github.com/hydro-project/hydro/commit/fbaab5b12c0c661ee08d8ded6862a38834ba62ae))
 * **Uncategorized**
    - Release dfir_lang v0.13.0, dfir_datalog_core v0.13.0, dfir_datalog v0.13.0, dfir_macro v0.13.0, hydro_deploy_integration v0.13.0, dfir_rs v0.13.0, hydro_deploy v0.13.0, hydro_lang v0.13.0, hydro_std v0.13.0, hydro_cli v0.13.0, safety bump 8 crates ([`400fd8f`](https://github.com/hydro-project/hydro/commit/400fd8f2e8cada253f54980e7edce0631be70a82))
</details>

<csr-unknown>
<details>
<summary>Show a summary per file</summary>
FileDescriptionhydro_lang/src/rewrites/analyze_counter.rsAdds custom panic withmeasurement details if regex matching fails. (Used to diagnose_counter() i32 rollover)hydro_deploy/core/src/localhost/samply.rsUpdates type foraddresses/resources, refactors frame lookup to use asynchronousjoin_all, and adjusts string output for missing symbols.hydro_deploy/core/src/localhost/mod.rsImproves error handlingduring command spawning with conditional context messages for whensamply or dtrace executables are not found.hydro_deploy/core/src/localhost/launched_binary.rsUsesserde_path_to_error for improved deserialization error context.dfir_lang/src/graph/ops/dest_sink.rsStandardizes error messages byremoving extraneous punctuation.dfir_lang/src/graph/ops/_counter.rsAdds explicit type annotationfor a cell initialization to prevent i32 rollover.</details>
<csr-unknown/>

## 0.12.1 (2025-03-15)

<csr-id-f14174e92875b3264ef811effd954ed76cb3c948/>
<csr-id-c944371ba1c290f38c02f60979b381a53c39e680/>
<csr-id-056ac62611319b7bd10a751d7e231423a1b8dc4e/>

### Chore

 - <csr-id-f14174e92875b3264ef811effd954ed76cb3c948/> update pinned nightly to 2025-03-10, clippy cleanups

### Documentation

 - <csr-id-b235a42a3071e55da7b09bdc8bc710b18e0fe053/> demote python deploy docs, fix docsrs configs, fix #1392, fix #1629
   Running thru the quickstart in order to write more about Rust
   `hydro_deploy`, ran into some confusion due to feature-gated items not
   showing up in docs.
   
   `rustdocflags = [ '--cfg=docsrs', '--cfg=stageleft_runtime' ]` uses the
   standard `[cfg(docrs)]` as well as enabled our
   `[cfg(stageleft_runtime)]` so things `impl<H: Host + 'static>
   IntoProcessSpec<'_, HydroDeploy> for Arc<H>` show up.
   
   Also set `--all-features` for the docsrs build

### Other

 - <csr-id-c944371ba1c290f38c02f60979b381a53c39e680/> update actions, stop using actions-rs
   `actions-rs` actions are all unmaintained, trying to fix
   
   > The `set-output` command is deprecated and will be disabled soon.
   Please upgrade to using Environment Files. For more information see:
   https://github.blog/changelog/2022-10-11-github-actions-deprecating-save-state-and-set-output-commands/

### Style

 - <csr-id-056ac62611319b7bd10a751d7e231423a1b8dc4e/> cleanup old clippy lints, remove deprecated `relalg` crate

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release.
 - 6 days passed between releases.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 4 unique issues were worked on: [#1766](https://github.com/hydro-project/hydro/issues/1766), [#1768](https://github.com/hydro-project/hydro/issues/1768), [#1785](https://github.com/hydro-project/hydro/issues/1785), [#1787](https://github.com/hydro-project/hydro/issues/1787)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1766](https://github.com/hydro-project/hydro/issues/1766)**
    - Update actions, stop using actions-rs ([`c944371`](https://github.com/hydro-project/hydro/commit/c944371ba1c290f38c02f60979b381a53c39e680))
 * **[#1768](https://github.com/hydro-project/hydro/issues/1768)**
    - Update pinned nightly to 2025-03-10, clippy cleanups ([`f14174e`](https://github.com/hydro-project/hydro/commit/f14174e92875b3264ef811effd954ed76cb3c948))
 * **[#1785](https://github.com/hydro-project/hydro/issues/1785)**
    - Cleanup old clippy lints, remove deprecated `relalg` crate ([`056ac62`](https://github.com/hydro-project/hydro/commit/056ac62611319b7bd10a751d7e231423a1b8dc4e))
 * **[#1787](https://github.com/hydro-project/hydro/issues/1787)**
    - Demote python deploy docs, fix docsrs configs, fix #1392, fix #1629 ([`b235a42`](https://github.com/hydro-project/hydro/commit/b235a42a3071e55da7b09bdc8bc710b18e0fe053))
 * **Uncategorized**
    - Release dfir_lang v0.12.1, dfir_datalog_core v0.12.1, dfir_datalog v0.12.1, dfir_macro v0.12.1, hydro_deploy_integration v0.12.1, lattices v0.6.1, pusherator v0.0.12, dfir_rs v0.12.1, hydro_deploy v0.12.1, hydro_lang v0.12.1, hydro_std v0.12.1, hydro_cli v0.12.1 ([`23221b5`](https://github.com/hydro-project/hydro/commit/23221b53b30918707ddaa85529d04cd7919166b4))
</details>

## 0.12.0 (2025-03-08)

<csr-id-49a387d4a21f0763df8ec94de73fb953c9cd333a/>
<csr-id-2fd6aa7417dfa29f389c04c5b9674b80bfed6cf2/>
<csr-id-ddbc5990647d567c941bdfb16606842fc93a08be/>
<csr-id-c293cca6855695107e9cef5c5df99fb04a571934/>
<csr-id-c1983308743d912e5bf2583b7cccbb47d8a8b5d1/>
<csr-id-44fb2806cf2d165d86695910f4755e0944c11832/>
<csr-id-3c5bb05487a25345ea2f70b5a1ffbe74a216c684/>
<csr-id-e0d9b26709011a2d5d56cc3f86a3ee640983e75a/>
<csr-id-3966d9063dae52e65b077321e0bd1150f2b0c3f1/>
<csr-id-fd85262930c678601a80c080fb79778675124964/>
<csr-id-e5c2e23359055f41492344edf19efbe3f2afd7ce/>
<csr-id-5cd0a9625822620dcc99b99356edfecbf0549497/>
<csr-id-ec3795a678d261a38085405b6e9bfea943dafefb/>
<csr-id-8f4426089dcbbe5d1098f89e367c7be49a03e401/>

### Chore

 - <csr-id-49a387d4a21f0763df8ec94de73fb953c9cd333a/> upgrade to Rust 2024 edition
   - Updates `Cargo.toml` to use new shared workspace keys
   - Updates lint settings (in workspace `Cargo.toml`)
   - `rustfmt` has changed slightly, resulting in a big diff - there are no
   actual code changes
   - Adds a script to `rustfmt` the template src files
 - <csr-id-2fd6aa7417dfa29f389c04c5b9674b80bfed6cf2/> update pinned nightly to 2025-02-10, cleanups for clippy

### Refactor (BREAKING)

 - <csr-id-e0d9b26709011a2d5d56cc3f86a3ee640983e75a/> require semicolons after loop braces, fix #1726
   Is better for syntax highlighting.

### Chore (BREAKING)

 - <csr-id-3966d9063dae52e65b077321e0bd1150f2b0c3f1/> use DFIR name instead of Hydroflow in some places, fix #1644
   Fix partially #1712
   
   * Renames `WriteContextArgs.hydroflow` to `WriteContextArgs.df_ident`
   for DFIR operator codegen
   * Removes some dead code/files

### Style

 - <csr-id-fd85262930c678601a80c080fb79778675124964/> clippy cleanups for latest stable rust

### Refactor

 - <csr-id-e5c2e23359055f41492344edf19efbe3f2afd7ce/> unchecked casts for handoffs & state, pedantic version
 - <csr-id-5cd0a9625822620dcc99b99356edfecbf0549497/> enable lints, cleanups for Rust 2024 #1732

### Chore

 - <csr-id-ec3795a678d261a38085405b6e9bfea943dafefb/> upgrade to Rust 2024 edition
   - Updates `Cargo.toml` to use new shared workspace keys
   - Updates lint settings (in workspace `Cargo.toml`)
   - `rustfmt` has changed slightly, resulting in a big diff - there are no
   actual code changes
   - Adds a script to `rustfmt` the template src files
 - <csr-id-8f4426089dcbbe5d1098f89e367c7be49a03e401/> update pinned nightly to 2025-02-10, cleanups for clippy

### Documentation

 - <csr-id-19784f5bef45a823549bb9084d0f51a2b7ce0981/> fix extraneous `\<` escaping introduced in #1558, fix #1614
   Previous code also inserted `\<` into code blocks. This fixes the
   original issue of unescaped `<`s by ensuring all op docs have them in
   `code blocks`, removes the escaping.
 - <csr-id-f8313b018f6a1101935e4c06abbe5af3aafb400c/> fix broken links, fix #1613
 - <csr-id-8e612917f97edbb3739381ceb7f20daa1e4403b1/> fix extraneous `\<` escaping introduced in #1558, fix #1614
   Previous code also inserted `\<` into code blocks. This fixes the
   original issue of unescaped `<`s by ensuring all op docs have them in
   `code blocks`, removes the escaping.
 - <csr-id-d45273943b0ca087b05f0fe4331b12cbe2ff4e90/> fix broken links, fix #1613

### New Features

 - <csr-id-ce134fa0bae4085a9f81f9e556a553618e3652ab/> add APIs for getting DFIR without deploying
   Also modifies DFIR to elide Stageleft `type_hint`s when pretty printing
   an operator (e.g. for Mermaid). Also, because we stratify the graph
   before we can print it, adds basic support for printing stratified
   graphs as a surface string.
 - <csr-id-3a3382aade8b9e9500ae8fec70496ba7125e63ad/> add more perf tags to operators that do non-iterator work
   Also unifies all `dest_sink` related work under a common identifier now
   that we pass it to the operator codegen.
 - <csr-id-7009a042837e4d8bbde82b680ee6779d9dec062b/> `next_iteration()` operator, `all_iterations()` un-windowing op, k-means example, fix #1612, fix #1731
 - <csr-id-1ba45050f02a9f40860d7a52f144d35aebec031e/> add basic `_counter()` op for cardinality metrics, fix #1729
   This is the simplest possible implementation, there's a lot of room for
   improvement with a metrics system built into the runtime, but keeping it
   simple for now.
 - <csr-id-69706bab1b8a86a18dae2d984d0a1a22f1efa4a5/> Print dest_sink sink_feed_flush with tag
   So we can see CPU usage from sending messages in perf
 - <csr-id-a72f7d56a720d56f6e5fd08e9dc6f50a2f302c57/> loop iteration counter, fix #1622
 - <csr-id-ee280c972848170783d105ba047b26dfa7249f24/> `loop {` scheduler
   Fix https://github.com/hydro-project/hydro/issues/1580
 - <csr-id-f3c459036976d87b20356a761bdea9c010ae680b/> Add ability to customize operator tag for stack tracing/flamegraphs
   Actually inserting Hydro-level operator IDs/names is TODO
   
   #1479
 - <csr-id-a9762c54fa0733e4ac9d3d2dbdd517f637ed5f24/> Allow state_by to use a factory function.
   Currently, state_by uses Default::default to instantiate the backing
   storage. Accepting a factory function will allow the storage to be
   tweaked per instance of state. Example usage: pre-allocating memory for
   the data structures.
 - <csr-id-2ad0be3a843431afdff21ad0119d3661b77666b8/> add APIs for getting DFIR without deploying
   Also modifies DFIR to elide Stageleft `type_hint`s when pretty printing
   an operator (e.g. for Mermaid). Also, because we stratify the graph
   before we can print it, adds basic support for printing stratified
   graphs as a surface string.
 - <csr-id-3fa37f95a2deaaa83d36f177cbf0fdf245362021/> add more perf tags to operators that do non-iterator work
   Also unifies all `dest_sink` related work under a common identifier now
   that we pass it to the operator codegen.
 - <csr-id-9aee8b770116b1fc70263b9fab7176bfa9ffe73f/> `next_iteration()` operator, `all_iterations()` un-windowing op, k-means example, fix #1612, fix #1731
 - <csr-id-11e07ead7e7b3fb0b552745d3332e0f6804a30a5/> add basic `_counter()` op for cardinality metrics, fix #1729
   This is the simplest possible implementation, there's a lot of room for
   improvement with a metrics system built into the runtime, but keeping it
   simple for now.
 - <csr-id-59480c6d0d21450ec30c8320846ba33e653b654e/> Print dest_sink sink_feed_flush with tag
   So we can see CPU usage from sending messages in perf
 - <csr-id-17bb5fa5a2b660a1202d3fa1c64fa606e1605f3a/> loop iteration counter, fix #1622
 - <csr-id-206be0e0bb5d2ad8d19c502d6ac13e0ca1fed552/> `loop {` scheduler
   Fix https://github.com/hydro-project/hydro/issues/1580
 - <csr-id-53b26fcfb5ac942680e830078f4411922f917f91/> Add ability to customize operator tag for stack tracing/flamegraphs
   Actually inserting Hydro-level operator IDs/names is TODO
   
   #1479
 - <csr-id-2ba8d37452fa459f05c9736d752f9a0a940e1e00/> Allow state_by to use a factory function.
   Currently, state_by uses Default::default to instantiate the backing
   storage. Accepting a factory function will allow the storage to be
   tweaked per instance of state. Example usage: pre-allocating memory for
   the data structures.

### Bug Fixes

 - <csr-id-a7d4d473b49d67818290a46a822b4e30e83478ff/> update stateful operators to be 'stateless' in loops, fix #1618
   eventually, when the old 'tick/'static machinery is removed, we can unify under one set of lifetimes
 - <csr-id-10d8db58f9f2757c93f228f0a7fb45f7719068d0/> include operator tag for pre-iterator work
 - <csr-id-247a21d9617d44f13755c696224a6942afb9922a/> update stateful operators to be 'stateless' in loops, fix #1618
   eventually, when the old 'tick/'static machinery is removed, we can unify under one set of lifetimes
 - <csr-id-794384d153e3d15a29cf29ecbe82031edcf9054f/> include operator tag for pre-iterator work

### Refactor

 - <csr-id-ddbc5990647d567c941bdfb16606842fc93a08be/> unchecked casts for handoffs & state, pedantic version
 - <csr-id-c293cca6855695107e9cef5c5df99fb04a571934/> enable lints, cleanups for Rust 2024 #1732

### Style

 - <csr-id-c1983308743d912e5bf2583b7cccbb47d8a8b5d1/> clippy cleanups for latest stable rust

### Chore (BREAKING)

 - <csr-id-44fb2806cf2d165d86695910f4755e0944c11832/> use DFIR name instead of Hydroflow in some places, fix #1644
   Fix partially #1712
   
   * Renames `WriteContextArgs.hydroflow` to `WriteContextArgs.df_ident`
   for DFIR operator codegen
   * Removes some dead code/files

### Refactor (BREAKING)

 - <csr-id-3c5bb05487a25345ea2f70b5a1ffbe74a216c684/> require semicolons after loop braces, fix #1726
   Is better for syntax highlighting.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 21 commits contributed to the release.
 - 74 days passed between releases.
 - 20 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 20 unique issues were worked on: [#1640](https://github.com/hydro-project/hydro/issues/1640), [#1682](https://github.com/hydro-project/hydro/issues/1682), [#1686](https://github.com/hydro-project/hydro/issues/1686), [#1690](https://github.com/hydro-project/hydro/issues/1690), [#1692](https://github.com/hydro-project/hydro/issues/1692), [#1702](https://github.com/hydro-project/hydro/issues/1702), [#1704](https://github.com/hydro-project/hydro/issues/1704), [#1713](https://github.com/hydro-project/hydro/issues/1713), [#1714](https://github.com/hydro-project/hydro/issues/1714), [#1722](https://github.com/hydro-project/hydro/issues/1722), [#1725](https://github.com/hydro-project/hydro/issues/1725), [#1727](https://github.com/hydro-project/hydro/issues/1727), [#1733](https://github.com/hydro-project/hydro/issues/1733), [#1737](https://github.com/hydro-project/hydro/issues/1737), [#1747](https://github.com/hydro-project/hydro/issues/1747), [#1753](https://github.com/hydro-project/hydro/issues/1753), [#1754](https://github.com/hydro-project/hydro/issues/1754), [#1757](https://github.com/hydro-project/hydro/issues/1757), [#1758](https://github.com/hydro-project/hydro/issues/1758), [#1759](https://github.com/hydro-project/hydro/issues/1759)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1640](https://github.com/hydro-project/hydro/issues/1640)**
    - Clippy cleanups for latest stable rust ([`fd85262`](https://github.com/hydro-project/hydro/commit/fd85262930c678601a80c080fb79778675124964))
 * **[#1682](https://github.com/hydro-project/hydro/issues/1682)**
    - Allow state_by to use a factory function. ([`2ba8d37`](https://github.com/hydro-project/hydro/commit/2ba8d37452fa459f05c9736d752f9a0a940e1e00))
 * **[#1686](https://github.com/hydro-project/hydro/issues/1686)**
    - Fix broken links, fix #1613 ([`d452739`](https://github.com/hydro-project/hydro/commit/d45273943b0ca087b05f0fe4331b12cbe2ff4e90))
 * **[#1690](https://github.com/hydro-project/hydro/issues/1690)**
    - Fix extraneous `\<` escaping introduced in #1558, fix #1614 ([`8e61291`](https://github.com/hydro-project/hydro/commit/8e612917f97edbb3739381ceb7f20daa1e4403b1))
 * **[#1692](https://github.com/hydro-project/hydro/issues/1692)**
    - `loop {` scheduler ([`206be0e`](https://github.com/hydro-project/hydro/commit/206be0e0bb5d2ad8d19c502d6ac13e0ca1fed552))
 * **[#1702](https://github.com/hydro-project/hydro/issues/1702)**
    - Add ability to customize operator tag for stack tracing/flamegraphs ([`53b26fc`](https://github.com/hydro-project/hydro/commit/53b26fcfb5ac942680e830078f4411922f917f91))
 * **[#1704](https://github.com/hydro-project/hydro/issues/1704)**
    - Update pinned nightly to 2025-02-10, cleanups for clippy ([`8f44260`](https://github.com/hydro-project/hydro/commit/8f4426089dcbbe5d1098f89e367c7be49a03e401))
 * **[#1713](https://github.com/hydro-project/hydro/issues/1713)**
    - Use DFIR name instead of Hydroflow in some places, fix #1644 ([`3966d90`](https://github.com/hydro-project/hydro/commit/3966d9063dae52e65b077321e0bd1150f2b0c3f1))
 * **[#1714](https://github.com/hydro-project/hydro/issues/1714)**
    - Loop iteration counter, fix #1622 ([`17bb5fa`](https://github.com/hydro-project/hydro/commit/17bb5fa5a2b660a1202d3fa1c64fa606e1605f3a))
 * **[#1722](https://github.com/hydro-project/hydro/issues/1722)**
    - Print dest_sink sink_feed_flush with tag ([`59480c6`](https://github.com/hydro-project/hydro/commit/59480c6d0d21450ec30c8320846ba33e653b654e))
 * **[#1725](https://github.com/hydro-project/hydro/issues/1725)**
    - `next_iteration()` operator, `all_iterations()` un-windowing op, k-means example, fix #1612, fix #1731 ([`9aee8b7`](https://github.com/hydro-project/hydro/commit/9aee8b770116b1fc70263b9fab7176bfa9ffe73f))
 * **[#1727](https://github.com/hydro-project/hydro/issues/1727)**
    - Require semicolons after loop braces, fix #1726 ([`e0d9b26`](https://github.com/hydro-project/hydro/commit/e0d9b26709011a2d5d56cc3f86a3ee640983e75a))
 * **[#1733](https://github.com/hydro-project/hydro/issues/1733)**
    - Add basic `_counter()` op for cardinality metrics, fix #1729 ([`11e07ea`](https://github.com/hydro-project/hydro/commit/11e07ead7e7b3fb0b552745d3332e0f6804a30a5))
 * **[#1737](https://github.com/hydro-project/hydro/issues/1737)**
    - Enable lints, cleanups for Rust 2024 #1732 ([`5cd0a96`](https://github.com/hydro-project/hydro/commit/5cd0a9625822620dcc99b99356edfecbf0549497))
 * **[#1747](https://github.com/hydro-project/hydro/issues/1747)**
    - Upgrade to Rust 2024 edition ([`ec3795a`](https://github.com/hydro-project/hydro/commit/ec3795a678d261a38085405b6e9bfea943dafefb))
 * **[#1753](https://github.com/hydro-project/hydro/issues/1753)**
    - Include operator tag for pre-iterator work ([`794384d`](https://github.com/hydro-project/hydro/commit/794384d153e3d15a29cf29ecbe82031edcf9054f))
 * **[#1754](https://github.com/hydro-project/hydro/issues/1754)**
    - Add more perf tags to operators that do non-iterator work ([`3fa37f9`](https://github.com/hydro-project/hydro/commit/3fa37f95a2deaaa83d36f177cbf0fdf245362021))
 * **[#1757](https://github.com/hydro-project/hydro/issues/1757)**
    - Unchecked casts for handoffs & state, pedantic version ([`e5c2e23`](https://github.com/hydro-project/hydro/commit/e5c2e23359055f41492344edf19efbe3f2afd7ce))
 * **[#1758](https://github.com/hydro-project/hydro/issues/1758)**
    - Add APIs for getting DFIR without deploying ([`2ad0be3`](https://github.com/hydro-project/hydro/commit/2ad0be3a843431afdff21ad0119d3661b77666b8))
 * **[#1759](https://github.com/hydro-project/hydro/issues/1759)**
    - Update stateful operators to be 'stateless' in loops, fix #1618 ([`247a21d`](https://github.com/hydro-project/hydro/commit/247a21d9617d44f13755c696224a6942afb9922a))
 * **Uncategorized**
    - Release dfir_lang v0.12.0, dfir_datalog_core v0.12.0, dfir_datalog v0.12.0, dfir_macro v0.12.0, hydroflow_deploy_integration v0.12.0, lattices_macro v0.5.9, variadics v0.0.9, variadics_macro v0.6.0, lattices v0.6.0, multiplatform_test v0.5.0, pusherator v0.0.11, dfir_rs v0.12.0, hydro_deploy v0.12.0, stageleft_macro v0.6.0, stageleft v0.7.0, stageleft_tool v0.6.0, hydro_lang v0.12.0, hydro_std v0.12.0, hydro_cli v0.12.0, safety bump 10 crates ([`973c925`](https://github.com/hydro-project/hydro/commit/973c925e87ed78344494581bd7ce1bbb4186a2f3))
</details>

## 0.11.0 (2024-12-23)

<csr-id-7dea92b19e6b86566cc27babb457415896b6b608/>
<csr-id-251b1039c71d45d3f86123dba1926026ded80824/>
<csr-id-5196f247e0124a31567af940541044ce1906cdc1/>
<csr-id-03b3a349013a71b324276bca5329c33d400a73ff/>
<csr-id-3291c07b37c9f9031837a2a32953e8f8854ec298/>
<csr-id-84ee06755a0ed7cabf32b334f1696bb600797c92/>
<csr-id-a6f60c92ae7168eb86eb311ca7b7afb10025c7de/>
<csr-id-5e58e346612a094c7e637919c84ab1e78b59be27/>

### New Features

 - <csr-id-b8acd843bdbcfb445bf942e697447f6bf58a10da/> initial flo implementation
   Basic first pass implementation, without changing the current scheduler,
   no breaking behavior. Adds the flo syntax (`loop { ... }`), basic graph
   structure checks, nested loop blocks, and two basic windowing operators
   (`batch()` and `all_once()`).
   
   Next steps:
   - Needs scheduler changes:

### Other

 - <csr-id-7dea92b19e6b86566cc27babb457415896b6b608/> fix stable tests
   Sets `TRYBUILD=overwrite` on stable, as the output messages will often
   not match `pinned-nightly`
   
   Adds env `HYDROFLOW_EXPECT_WARNINGS=ignore` to allow warning tests to
   pass

### Chore

 - <csr-id-84ee06755a0ed7cabf32b334f1696bb600797c92/> update links for renamed repo (excluding `CHANGELOG.md`s), fix #1571
 - <csr-id-a6f60c92ae7168eb86eb311ca7b7afb10025c7de/> bump versions manually for renamed crates, per `RELEASING.md`
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
 - <csr-id-e1a08e5d165fbc80da2ae695e507078a97a9031f/> update `CHANGELOG.md`s for big rename
   Generated before rename per `RELEASING.md` instructions.
 - <csr-id-6ab625273d822812e83a333e928c3dea1c3c9ccb/> cleanups for the rename, fixing links
 - <csr-id-204bd117ca3a8845b4986539efb91a0c612dfa05/> add `repository` field to `Cargo.toml`s, fix #1452
   #1452 
   
   Will trigger new releases of the following:
   `unchanged = 'hydroflow_deploy_integration', 'variadics',
   'variadics_macro', 'pusherator'`
   
   (All other crates already have changes, so would be released anyway)
 - <csr-id-27c40e2ca5a822f6ebd31c7f01213aa6d407418a/> update `CHANGELOG.md`s for big rename
   Generated before rename per `RELEASING.md` instructions.
 - <csr-id-987f7ad8668d9740ceea577a595035228898d530/> cleanups for the rename, fixing links

### Chore

 - <csr-id-5196f247e0124a31567af940541044ce1906cdc1/> update links for renamed repo (excluding `CHANGELOG.md`s), fix #1571
 - <csr-id-03b3a349013a71b324276bca5329c33d400a73ff/> bump versions manually for renamed crates, per `RELEASING.md`
 - <csr-id-3291c07b37c9f9031837a2a32953e8f8854ec298/> Rename Hydroflow -> DFIR
   Work In Progress:
   - [x] hydroflow_macro
   - [x] hydroflow_datalog_core
   - [x] hydroflow_datalog
   - [x] hydroflow_lang
   - [x] hydroflow

### New Features (BREAKING)

 - <csr-id-f2a4bee8cd6945937bed5bc22fd85efd8d0aef0a/> remove `import!`, fix #1110
   in prep for rust stable #1587
   
   No good way to resolve the source file paths on stable
   
   No way to get good diagnostics on external files in general, at all

### Bug Fixes (BREAKING)

 - <csr-id-eb1ad3a54705efb06ee3f0647deaa9a52731ae6e/> rename `union` to `chain` and restrict LHS to be bounded
   Returning a `Stream` from `union` on unbounded streams was unsound,
   since the order of outputs is not deterministic.

### Refactor (BREAKING)

 - <csr-id-251b1039c71d45d3f86123dba1926026ded80824/> use `cfg(nightly)` instead of feature, remove `-Z` flag, use `Diagnostic::try_emit`
   Previous PR (#1587) website build did not work because `panic = "abort"`
   is set on wasm, leading to aborts for `proc_macro2::Span::unwrap()`
   calls.
   
   All tests except trybuild seem to pass on stable, WIP #1587 next

### `hydroflow_lang` Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release.
 - 38 days passed between releases.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 5 unique issues were worked on: [#1565](https://github.com/hydro-project/hydroflow/issues/1565), [#1585](https://github.com/hydro-project/hydroflow/issues/1585), [#1600](https://github.com/hydro-project/hydroflow/issues/1600), [#1606](https://github.com/hydro-project/hydroflow/issues/1606), [#1608](https://github.com/hydro-project/hydroflow/issues/1608)

### `hydroflow_lang` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1565](https://github.com/hydro-project/hydroflow/issues/1565)**
    - Rename `union` to `chain` and restrict LHS to be bounded ([`eb1ad3a`](https://github.com/hydro-project/hydroflow/commit/eb1ad3a54705efb06ee3f0647deaa9a52731ae6e))
 * **[#1585](https://github.com/hydro-project/hydroflow/issues/1585)**
    - Initial flo implementation ([`b8acd84`](https://github.com/hydro-project/hydroflow/commit/b8acd843bdbcfb445bf942e697447f6bf58a10da))
 * **[#1600](https://github.com/hydro-project/hydroflow/issues/1600)**
    - Remove `import!`, fix #1110 ([`f2a4bee`](https://github.com/hydro-project/hydroflow/commit/f2a4bee8cd6945937bed5bc22fd85efd8d0aef0a))
 * **[#1606](https://github.com/hydro-project/hydroflow/issues/1606)**
    - Use `cfg(nightly)` instead of feature, remove `-Z` flag, use `Diagnostic::try_emit` ([`251b103`](https://github.com/hydro-project/hydroflow/commit/251b1039c71d45d3f86123dba1926026ded80824))
 * **[#1608](https://github.com/hydro-project/hydroflow/issues/1608)**
    - Fix stable tests ([`7dea92b`](https://github.com/hydro-project/hydroflow/commit/7dea92b19e6b86566cc27babb457415896b6b608))
</details>

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 7 commits contributed to the release.
 - 6 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 5 unique issues were worked on: [#1501](https://github.com/hydro-project/hydro/issues/1501), [#1620](https://github.com/hydro-project/hydro/issues/1620), [#1624](https://github.com/hydro-project/hydro/issues/1624), [#1627](https://github.com/hydro-project/hydro/issues/1627), [#1628](https://github.com/hydro-project/hydro/issues/1628)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1501](https://github.com/hydro-project/hydro/issues/1501)**
    - Add `repository` field to `Cargo.toml`s, fix #1452 ([`204bd11`](https://github.com/hydro-project/hydro/commit/204bd117ca3a8845b4986539efb91a0c612dfa05))
 * **[#1620](https://github.com/hydro-project/hydro/issues/1620)**
    - Rename Hydroflow -> DFIR ([`5e58e34`](https://github.com/hydro-project/hydro/commit/5e58e346612a094c7e637919c84ab1e78b59be27))
 * **[#1624](https://github.com/hydro-project/hydro/issues/1624)**
    - Cleanups for the rename, fixing links ([`987f7ad`](https://github.com/hydro-project/hydro/commit/987f7ad8668d9740ceea577a595035228898d530))
 * **[#1627](https://github.com/hydro-project/hydro/issues/1627)**
    - Bump versions manually for renamed crates, per `RELEASING.md` ([`a6f60c9`](https://github.com/hydro-project/hydro/commit/a6f60c92ae7168eb86eb311ca7b7afb10025c7de))
 * **[#1628](https://github.com/hydro-project/hydro/issues/1628)**
    - Update links for renamed repo (excluding `CHANGELOG.md`s), fix #1571 ([`84ee067`](https://github.com/hydro-project/hydro/commit/84ee06755a0ed7cabf32b334f1696bb600797c92))
 * **Uncategorized**
    - Release dfir_lang v0.11.0, dfir_datalog_core v0.11.0, dfir_datalog v0.11.0, dfir_macro v0.11.0, hydroflow_deploy_integration v0.11.0, lattices_macro v0.5.8, variadics v0.0.8, variadics_macro v0.5.6, lattices v0.5.9, multiplatform_test v0.4.0, pusherator v0.0.10, dfir_rs v0.11.0, hydro_deploy v0.11.0, stageleft_macro v0.5.0, stageleft v0.6.0, stageleft_tool v0.5.0, hydro_lang v0.11.0, hydro_std v0.11.0, hydro_cli v0.11.0, safety bump 6 crates ([`361b443`](https://github.com/hydro-project/hydro/commit/361b4439ef9c781860f18d511668ab463a8c5203))
    - Update `CHANGELOG.md`s for big rename ([`27c40e2`](https://github.com/hydro-project/hydro/commit/27c40e2ca5a822f6ebd31c7f01213aa6d407418a))
</details>

## 0.10.0 (2024-11-08)

<csr-id-d5677604e93c07a5392f4229af94a0b736eca382/>
<csr-id-32e297094fed9908ca7bf77e7068bc0a6ea52eae/>
<csr-id-47cb703e771f7d1c451ceb9d185ada96410949da/>

### Chore

 - <csr-id-d5677604e93c07a5392f4229af94a0b736eca382/> update pinned rust version, clippy lints, remove some dead code

### New Features

 - <csr-id-f7e740fb2ba36d0fcf3fd196d60333552911e3a4/> generalized hash trie indexes for relational tuples
   Generalized Hash Tries are part of the SIGMOD '23 FreeJoin
   [paper](https://dl.acm.org/doi/abs/10.1145/3589295) by
   Wang/Willsey/Suciu. They provide a compressed ("factorized")
   representation of relations. By operating in the factorized domain, join
   algorithms can defer cross-products and achieve asymptotically optimal
   performance.
   
   ---------
 - <csr-id-d83cb83df59e647ba99bd896e7605ee18b9a84f6/> Added state_by operator.
   For https://github.com/hydro-project/hydroflow/issues/1467

### Bug Fixes

 - <csr-id-16b730c75cfca79ea5f869308b1e1e14b3e9c155/> `cross_singleton()` forgot value if multiple runs in a tick, fix #1518
   Adds the minimal reproducer test from @shadaj
   
   Note this may have negative performance implications, as the singleton value now is stored in the state API (heap) instead of locally. If we use singleton syntax this duplicate allocation could probably be avoided.
   
   > Confirmed that this fixed the bugs in our Paxos implementation, no noticeable performance impact. @shadj

### Refactor

 - <csr-id-32e297094fed9908ca7bf77e7068bc0a6ea52eae/> update topo-sort to detect cycles

### Style

 - <csr-id-47cb703e771f7d1c451ceb9d185ada96410949da/> fixes for nightly clippy
   a couple few spurious `too_many_arguments` and a spurious
   `zombie_processes` still on current nightly (`clippy 0.1.84 (4392847410
   2024-10-21)`)

### Bug Fixes (BREAKING)

 - <csr-id-e796200743f2cc2da5a0e91c492f016ca98008e8/> fix #1401 `lattice_bimorphism()` double-emit, add docs
   Fixes the issue by combining the all values generated per subgraph
   execution into one, which effectively de-duplicates the values.
   
   Adds basic docs.

### `hydroflow_lang` Commit Statistics

<csr-read-only-do-not-edit/>

 - 8 commits contributed to the release.
 - 69 days passed between releases.
 - 7 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 7 unique issues were worked on: [#1444](https://github.com/hydro-project/hydroflow/issues/1444), [#1469](https://github.com/hydro-project/hydroflow/issues/1469), [#1503](https://github.com/hydro-project/hydroflow/issues/1503), [#1505](https://github.com/hydro-project/hydroflow/issues/1505), [#1512](https://github.com/hydro-project/hydroflow/issues/1512), [#1520](https://github.com/hydro-project/hydroflow/issues/1520), [#1522](https://github.com/hydro-project/hydroflow/issues/1522)

### `hydroflow_lang` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1444](https://github.com/hydro-project/hydroflow/issues/1444)**
    - Update pinned rust version, clippy lints, remove some dead code ([`d567760`](https://github.com/hydro-project/hydroflow/commit/d5677604e93c07a5392f4229af94a0b736eca382))
 * **[#1469](https://github.com/hydro-project/hydroflow/issues/1469)**
    - Added state_by operator. ([`d83cb83`](https://github.com/hydro-project/hydroflow/commit/d83cb83df59e647ba99bd896e7605ee18b9a84f6))
 * **[#1503](https://github.com/hydro-project/hydroflow/issues/1503)**
    - Generalized hash trie indexes for relational tuples ([`f7e740f`](https://github.com/hydro-project/hydroflow/commit/f7e740fb2ba36d0fcf3fd196d60333552911e3a4))
 * **[#1505](https://github.com/hydro-project/hydroflow/issues/1505)**
    - Fixes for nightly clippy ([`47cb703`](https://github.com/hydro-project/hydroflow/commit/47cb703e771f7d1c451ceb9d185ada96410949da))
 * **[#1512](https://github.com/hydro-project/hydroflow/issues/1512)**
    - Update topo-sort to detect cycles ([`32e2970`](https://github.com/hydro-project/hydroflow/commit/32e297094fed9908ca7bf77e7068bc0a6ea52eae))
 * **[#1520](https://github.com/hydro-project/hydroflow/issues/1520)**
    - `cross_singleton()` forgot value if multiple runs in a tick, fix #1518 ([`16b730c`](https://github.com/hydro-project/hydroflow/commit/16b730c75cfca79ea5f869308b1e1e14b3e9c155))
 * **[#1522](https://github.com/hydro-project/hydroflow/issues/1522)**
    - Fix #1401 `lattice_bimorphism()` double-emit, add docs ([`e796200`](https://github.com/hydro-project/hydroflow/commit/e796200743f2cc2da5a0e91c492f016ca98008e8))
 * **Uncategorized**
    - Release hydroflow_lang v0.10.0, hydroflow_datalog_core v0.10.0, hydroflow_datalog v0.10.0, hydroflow_deploy_integration v0.10.0, hydroflow_macro v0.10.0, lattices_macro v0.5.7, variadics v0.0.7, variadics_macro v0.5.5, lattices v0.5.8, multiplatform_test v0.3.0, pusherator v0.0.9, hydroflow v0.10.0, hydro_deploy v0.10.0, stageleft_macro v0.4.0, stageleft v0.5.0, stageleft_tool v0.4.0, hydroflow_plus v0.10.0, hydro_cli v0.10.0, safety bump 8 crates ([`dcd48fc`](https://github.com/hydro-project/hydroflow/commit/dcd48fc7ee805898d9b5ef0d082870e30615e95b))
</details>

## 0.9.0 (2024-08-30)

<csr-id-11af32828bab6e4a4264d2635ff71a12bb0bb778/>
<csr-id-3c41e1e9b505bb84474ec10df30fb65c6d6f90cb/>
<csr-id-9c352f50a5a8d1b2187b34d5847a23b7397fe6ec/>

### Chore

 - <csr-id-11af32828bab6e4a4264d2635ff71a12bb0bb778/> lower min dependency versions where possible, update `Cargo.lock`
   Moved from #1418
   
   ---------
 - <csr-id-3c41e1e9b505bb84474ec10df30fb65c6d6f90cb/> drop unused dependencies

### Documentation

 - <csr-id-f5f1eb0c612f5c0c1752360d972ef6853c5e12f0/> cleanup doc comments for clippy latest

### New Features

 - <csr-id-9e5f58ef773f0aee39a9705d9845361a2488649b/> allow `demux_enum` to have any number of outputs, fix #1329
 - <csr-id-bd793e2fc1db735a6e623973028e99675a7933e0/> add `cross_singleton` operator
   This operator is necessary to eliminate performance bottlenecks in Paxos
   where cross-products result in unnecessary cloning and lack of
   short-circuit behavior results in values being cloned out of the
   internal state of `reduce_keyed`.

### Bug Fixes

 - <csr-id-22c72189bb76412955d29b03c5d99894c558a07c/> remove `FlowProps`
 - <csr-id-75dd4fbb772a3ae7e58989aacab2121a1a487557/> `fold<'tick>` should always emit a value
 - <csr-id-f215b18121fec131287115869b663899a7ea59ae/> Provide types for filter on antijoin

### Refactor

 - <csr-id-9c352f50a5a8d1b2187b34d5847a23b7397fe6ec/> cleanup handling of span locations #1268, workaround fix #729

### New Features (BREAKING)

 - <csr-id-0ee5d96e2c1ea2821084aad434abd6e06e157111/> change `state()` to output a stream of only de-duplicated items

### `hydroflow_lang` Commit Statistics

<csr-read-only-do-not-edit/>

 - 11 commits contributed to the release.
 - 38 days passed between releases.
 - 10 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 10 unique issues were worked on: [#1362](https://github.com/hydro-project/hydroflow/issues/1362), [#1373](https://github.com/hydro-project/hydroflow/issues/1373), [#1407](https://github.com/hydro-project/hydroflow/issues/1407), [#1409](https://github.com/hydro-project/hydroflow/issues/1409), [#1412](https://github.com/hydro-project/hydroflow/issues/1412), [#1417](https://github.com/hydro-project/hydroflow/issues/1417), [#1420](https://github.com/hydro-project/hydroflow/issues/1420), [#1423](https://github.com/hydro-project/hydroflow/issues/1423), [#1428](https://github.com/hydro-project/hydroflow/issues/1428), [#1432](https://github.com/hydro-project/hydroflow/issues/1432)

### `hydroflow_lang` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1362](https://github.com/hydro-project/hydroflow/issues/1362)**
    - Provide types for filter on antijoin ([`f215b18`](https://github.com/hydro-project/hydroflow/commit/f215b18121fec131287115869b663899a7ea59ae))
 * **[#1373](https://github.com/hydro-project/hydroflow/issues/1373)**
    - Add `cross_singleton` operator ([`bd793e2`](https://github.com/hydro-project/hydroflow/commit/bd793e2fc1db735a6e623973028e99675a7933e0))
 * **[#1407](https://github.com/hydro-project/hydroflow/issues/1407)**
    - `fold<'tick>` should always emit a value ([`75dd4fb`](https://github.com/hydro-project/hydroflow/commit/75dd4fbb772a3ae7e58989aacab2121a1a487557))
 * **[#1409](https://github.com/hydro-project/hydroflow/issues/1409)**
    - Allow `demux_enum` to have any number of outputs, fix #1329 ([`9e5f58e`](https://github.com/hydro-project/hydroflow/commit/9e5f58ef773f0aee39a9705d9845361a2488649b))
 * **[#1412](https://github.com/hydro-project/hydroflow/issues/1412)**
    - Change `state()` to output a stream of only de-duplicated items ([`0ee5d96`](https://github.com/hydro-project/hydroflow/commit/0ee5d96e2c1ea2821084aad434abd6e06e157111))
 * **[#1417](https://github.com/hydro-project/hydroflow/issues/1417)**
    - Drop unused dependencies ([`3c41e1e`](https://github.com/hydro-project/hydroflow/commit/3c41e1e9b505bb84474ec10df30fb65c6d6f90cb))
 * **[#1420](https://github.com/hydro-project/hydroflow/issues/1420)**
    - Remove `FlowProps` ([`22c7218`](https://github.com/hydro-project/hydroflow/commit/22c72189bb76412955d29b03c5d99894c558a07c))
 * **[#1423](https://github.com/hydro-project/hydroflow/issues/1423)**
    - Lower min dependency versions where possible, update `Cargo.lock` ([`11af328`](https://github.com/hydro-project/hydroflow/commit/11af32828bab6e4a4264d2635ff71a12bb0bb778))
 * **[#1428](https://github.com/hydro-project/hydroflow/issues/1428)**
    - Cleanup doc comments for clippy latest ([`f5f1eb0`](https://github.com/hydro-project/hydroflow/commit/f5f1eb0c612f5c0c1752360d972ef6853c5e12f0))
 * **[#1432](https://github.com/hydro-project/hydroflow/issues/1432)**
    - Cleanup handling of span locations #1268, workaround fix #729 ([`9c352f5`](https://github.com/hydro-project/hydroflow/commit/9c352f50a5a8d1b2187b34d5847a23b7397fe6ec))
 * **Uncategorized**
    - Release hydroflow_lang v0.9.0, hydroflow_datalog_core v0.9.0, hydroflow_datalog v0.9.0, hydroflow_deploy_integration v0.9.0, hydroflow_macro v0.9.0, lattices_macro v0.5.6, lattices v0.5.7, multiplatform_test v0.2.0, variadics v0.0.6, pusherator v0.0.8, hydroflow v0.9.0, stageleft_macro v0.3.0, stageleft v0.4.0, stageleft_tool v0.3.0, hydroflow_plus v0.9.0, hydro_deploy v0.9.0, hydro_cli v0.9.0, hydroflow_plus_deploy v0.9.0, safety bump 8 crates ([`0750117`](https://github.com/hydro-project/hydroflow/commit/0750117de7088c01a439b102adeb4c832889f171))
</details>

## 0.8.0 (2024-07-23)

<csr-id-f1442a0161fe7a1827a60cb96ff11646d711373d/>
<csr-id-70a3f9e19f70fae0967eb96c454ab922c0f5290b/>
<csr-id-b7275c2a770283cf8012a107ec422931e1f7338a/>
<csr-id-67c0e51fb25ea1a2e3aae197c1984920b46759fa/>

### Documentation

 - <csr-id-14067e920394f2ec268445e72db358aab324f2ad/> cleanup for `clippy::doc_lazy_continuation`
 - <csr-id-2a4881d7c981bdf8f4deae9902e7d305f36c4203/> test code snipets, generate output for `surface_flows`, fix #814
 - <csr-id-1bc6c61e9acbd965874ab97fbacbe70513693fe9/> fix `dest_sink` tokio links

### Bug Fixes

<csr-id-b79f1a4c8f30131c8ca2ab4900efed9ded819581/>

 - <csr-id-f45b9ddbfca84e11398f3dec774b713b5b071422/> allow `ensure_singleton_referencers_succeed_persist` to ignore `identity`/`tee`/`union` operators, helps #1290
   Adds tests for `ensure_singleton_referencers_succeed_persist` persist
   insertion behavior
   
   Fixes some cases of #1290, when just identity operators
   (union/tee/identity) are between the singleton referencer and its
   preceding flow state.
   
   TODO track per-operator instead: #1311
 - <csr-id-404f0accf08e643a1c5e815f06bb31a65379e8c8/> improve spanning of singleton substitution code, add compile-fail test, fix #1294
   Another small spanning improvement
 - <csr-id-f91c30045dfdf92cf3d383676875d9e749cb8d93/> add `add_state_tick` to state API, reset at end of each tick, fix #1298
   Option 2 of #1298
   
   * Main feature in the title is in `src/scheduled/{context,graph}.rs`
* Codegen for some stateful operators (those which can be used as singletons, and some others) is changed to use the new API.
* Add a test `test_cartesian_product_tick_state` for #1298
* Rest of the diff is snapshot changes

### Refactor

 - <csr-id-f1442a0161fe7a1827a60cb96ff11646d711373d/> improve diagnostics for missing generic params
   Improves diagnostic messages a bit for when no generic params are
   supplied but some are expected. Previously this would span to the entire
   macro invocation.
 - <csr-id-70a3f9e19f70fae0967eb96c454ab922c0f5290b/> improve diagnostics by re-spanning `#root`
   Inspired by fixing the spans in `demux_enum` in #1271
   * re-span `#root` to `op_span` for better diagnostics
   * use better span `func.inputs` in `demux` and `demux_enum`
   * clippy fixups in `source_json`, `source_stdin`
   * fix #1201 (for the most part)

### Reverted

 - <csr-id-256779abece03bee662b351430d27141d10bd5ef/> "feat(hydroflow): Added poll_futures and poll_futures_async operators.", fix #1183
   This reverts commit 997d90a76db9a4e05dbac35073a09548750ce342.
   
   We have been trying to figure out the semantics a bit, and want to give
   it more thought before we commit to maintaining it
   
   Can un-revert and adjust the semantics later when we use it

### Style

 - <csr-id-b7275c2a770283cf8012a107ec422931e1f7338a/> fix unnecessary `&` for clippy

### Bug Fixes (BREAKING)

 - <csr-id-755e8a6d2c2b30b5d28b60315bb099030d3f3964/> remove singleton referencer `persist::<'static>()` insertion
   Also enables singletons for `persist()` and ensures that only the
   `'static` lifetime is used
   
   Singletons are supposed to act like `cross_join()`. I.e. if we have this
   code:
   ```rust
   stream -> filter(|item| ... #y ...) -> ...
   ```
   It should behave equivalently to
   ```rust
   stream -> cj[0];
   y -> cj[1];
   cj = cross_join() -> filter(|(item, y)| ...) -> ...
   ```
   
   This has a very unintuitive replaying behavior, if `y` receives multiple
   updates:
   1. `y` receives an item `10`
2. `stream` receives an item `20`
3. `(10, 20)` is emitted
4. `y` receives an item `11`
5. `(11, 20)` is emitted
   In this case the item `20` gets emitted twice.

### Refactor (BREAKING)

 - <csr-id-67c0e51fb25ea1a2e3aae197c1984920b46759fa/> require lifetime on `perist*()` operators

### `hydroflow_lang` Commit Statistics

<csr-read-only-do-not-edit/>

 - 14 commits contributed to the release.
 - 59 days passed between releases.
 - 13 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 13 unique issues were worked on: [#1143](https://github.com/hydro-project/hydroflow/issues/1143), [#1216](https://github.com/hydro-project/hydroflow/issues/1216), [#1260](https://github.com/hydro-project/hydroflow/issues/1260), [#1266](https://github.com/hydro-project/hydroflow/issues/1266), [#1271](https://github.com/hydro-project/hydroflow/issues/1271), [#1280](https://github.com/hydro-project/hydroflow/issues/1280), [#1285](https://github.com/hydro-project/hydroflow/issues/1285), [#1295](https://github.com/hydro-project/hydroflow/issues/1295), [#1296](https://github.com/hydro-project/hydroflow/issues/1296), [#1297](https://github.com/hydro-project/hydroflow/issues/1297), [#1300](https://github.com/hydro-project/hydroflow/issues/1300), [#1312](https://github.com/hydro-project/hydroflow/issues/1312), [#1332](https://github.com/hydro-project/hydroflow/issues/1332)

### `hydroflow_lang` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1143](https://github.com/hydro-project/hydroflow/issues/1143)**
    - "feat(hydroflow): Added poll_futures and poll_futures_async operators.", fix #1183 ([`256779a`](https://github.com/hydro-project/hydroflow/commit/256779abece03bee662b351430d27141d10bd5ef))
 * **[#1216](https://github.com/hydro-project/hydroflow/issues/1216)**
    - "feat(hydroflow): Added poll_futures and poll_futures_async operators.", fix #1183 ([`256779a`](https://github.com/hydro-project/hydroflow/commit/256779abece03bee662b351430d27141d10bd5ef))
 * **[#1260](https://github.com/hydro-project/hydroflow/issues/1260)**
    - Test code snipets, generate output for `surface_flows`, fix #814 ([`2a4881d`](https://github.com/hydro-project/hydroflow/commit/2a4881d7c981bdf8f4deae9902e7d305f36c4203))
 * **[#1266](https://github.com/hydro-project/hydroflow/issues/1266)**
    - Fix `dest_sink` tokio links ([`1bc6c61`](https://github.com/hydro-project/hydroflow/commit/1bc6c61e9acbd965874ab97fbacbe70513693fe9))
 * **[#1271](https://github.com/hydro-project/hydroflow/issues/1271)**
    - Allow use of generics in `demux_enum::<...>()` op ([`b79f1a4`](https://github.com/hydro-project/hydroflow/commit/b79f1a4c8f30131c8ca2ab4900efed9ded819581))
 * **[#1280](https://github.com/hydro-project/hydroflow/issues/1280)**
    - Improve diagnostics by re-spanning `#root` ([`70a3f9e`](https://github.com/hydro-project/hydroflow/commit/70a3f9e19f70fae0967eb96c454ab922c0f5290b))
 * **[#1285](https://github.com/hydro-project/hydroflow/issues/1285)**
    - Cleanup for `clippy::doc_lazy_continuation` ([`14067e9`](https://github.com/hydro-project/hydroflow/commit/14067e920394f2ec268445e72db358aab324f2ad))
 * **[#1295](https://github.com/hydro-project/hydroflow/issues/1295)**
    - Require lifetime on `perist*()` operators ([`67c0e51`](https://github.com/hydro-project/hydroflow/commit/67c0e51fb25ea1a2e3aae197c1984920b46759fa))
 * **[#1296](https://github.com/hydro-project/hydroflow/issues/1296)**
    - Improve diagnostics for missing generic params ([`f1442a0`](https://github.com/hydro-project/hydroflow/commit/f1442a0161fe7a1827a60cb96ff11646d711373d))
 * **[#1297](https://github.com/hydro-project/hydroflow/issues/1297)**
    - Allow `ensure_singleton_referencers_succeed_persist` to ignore `identity`/`tee`/`union` operators, helps #1290 ([`f45b9dd`](https://github.com/hydro-project/hydroflow/commit/f45b9ddbfca84e11398f3dec774b713b5b071422))
 * **[#1300](https://github.com/hydro-project/hydroflow/issues/1300)**
    - Add `add_state_tick` to state API, reset at end of each tick, fix #1298 ([`f91c300`](https://github.com/hydro-project/hydroflow/commit/f91c30045dfdf92cf3d383676875d9e749cb8d93))
 * **[#1312](https://github.com/hydro-project/hydroflow/issues/1312)**
    - Improve spanning of singleton substitution code, add compile-fail test, fix #1294 ([`404f0ac`](https://github.com/hydro-project/hydroflow/commit/404f0accf08e643a1c5e815f06bb31a65379e8c8))
 * **[#1332](https://github.com/hydro-project/hydroflow/issues/1332)**
    - Remove singleton referencer `persist::<'static>()` insertion ([`755e8a6`](https://github.com/hydro-project/hydroflow/commit/755e8a6d2c2b30b5d28b60315bb099030d3f3964))
 * **Uncategorized**
    - Release hydroflow_lang v0.8.0, hydroflow_datalog_core v0.8.0, hydroflow_datalog v0.8.0, hydroflow_macro v0.8.0, lattices_macro v0.5.5, lattices v0.5.6, variadics v0.0.5, pusherator v0.0.7, hydroflow v0.8.0, hydroflow_plus v0.8.0, hydro_deploy v0.8.0, hydro_cli v0.8.0, hydroflow_plus_cli_integration v0.8.0, safety bump 7 crates ([`ca6c16b`](https://github.com/hydro-project/hydroflow/commit/ca6c16b4a7ce35e155fe7fc6c7d1676c37c9e4de))
    - Fix unnecessary `&` for clippy ([`b7275c2`](https://github.com/hydro-project/hydroflow/commit/b7275c2a770283cf8012a107ec422931e1f7338a))
</details>

## 0.7.0 (2024-05-24)

<csr-id-b86f11aad344fef6ad9cdd1db0b45bb738c48bd6/>
<csr-id-18015029a725b068696ed9edefd1097583c858a6/>
<csr-id-826dbd9a709de2f883992bdcefa8f2d566d74ecb/>
<csr-id-20471f11901e3fb15a2efea61752d836d4facba5/>
<csr-id-40f1a19ece2a8352e6fdc31f815d923e635f91b1/>
<csr-id-1057b273cbd941c1a12a287580c15e264d797f2c/>
<csr-id-271535091f6bd810ca8957c72dae357e3ddffa52/>
<csr-id-d9b2c0263d508e6f6855f49504896d4ea670c355/>
<csr-id-d2427e2cc901c4174830d41b4a1dfc52fd4f19ce/>
<csr-id-4386fac824d64f63eae7629292675ac6bc8df9f7/>
<csr-id-d7e579c39b370a0ea0b0385d1029e9f8a7351d68/>

### Chore

 - <csr-id-b86f11aad344fef6ad9cdd1db0b45bb738c48bd6/> expect custom config names to prevent warnings
   See
   https://doc.rust-lang.org/nightly/cargo/reference/build-scripts.html#rustc-check-cfg
 - <csr-id-18015029a725b068696ed9edefd1097583c858a6/> update pyo3, silence warnings in generated code

### New Features

 - <csr-id-b0692b0d697980eaf9893c07a443a257e04786c5/> add `'static`/`'tick` support for `state()` operator
 - <csr-id-f184ea145f0c7c3072d7d0f94d42fcda717ac8d9/> insert `persist()` before singleton referencers to enable replay
 - <csr-id-4ca8ce43c0998296e2d86bd74800585ebb24123a/> allow `defer_tick()`, `next_stratum()` to have a type hint arg
 - <csr-id-9df9c6251526903dbe7288e2fd9a532c63a9412c/> error on unused ports, fix #1108
 - <csr-id-997d90a76db9a4e05dbac35073a09548750ce342/> Added poll_futures and poll_futures_async operators.
 - <csr-id-29a263fb564c5ce4bc495ea4e9d20b8b2621b645/> add support for collecting counts and running perf

### Bug Fixes

 - <csr-id-e2fa6b0729cb92b29e1d293c2788458845ff306a/> fix #1050 for `reduce_keyed`, fix #1050
 - <csr-id-d57b91146ef44125f1dd87e040ef636797f90e76/> cleanup codegen for `fold_keyed`
 - <csr-id-d773f9a6938fe6d1521516f7a7c441c6c281a9fa/> fix scheduler spinning on replay, fix #961
   fixes the added tests failing in the previous commit

### Refactor

 - <csr-id-826dbd9a709de2f883992bdcefa8f2d566d74ecb/> simplify `demux_enum()`, somewhat improves error messages #1201
 - <csr-id-20471f11901e3fb15a2efea61752d836d4facba5/> reorder error message emission for conflicting varnames
 - <csr-id-40f1a19ece2a8352e6fdc31f815d923e635f91b1/> remove reference edges, reverts #1017 #1041
 - <csr-id-1057b273cbd941c1a12a287580c15e264d797f2c/> unify `process_singletons.rs` code

### Style

 - <csr-id-271535091f6bd810ca8957c72dae357e3ddffa52/> cleanup some ops formatting
 - <csr-id-d9b2c0263d508e6f6855f49504896d4ea670c355/> appease latest nightly clippy

### Test

 - <csr-id-d2427e2cc901c4174830d41b4a1dfc52fd4f19ce/> update some compile-fail outputs caused by rustc regression

### New Features (BREAKING)

 - <csr-id-c2f6c9578127a71c879752d52e115df75659e2b0/> Introduce newtypes for working with ticks

### Refactor (BREAKING)

 - <csr-id-4386fac824d64f63eae7629292675ac6bc8df9f7/> change `lattice_bimorphism` to take state via singleton arguments #969 #1058
 - <csr-id-d7e579c39b370a0ea0b0385d1029e9f8a7351d68/> replace `state()` with `state_ref()` implementation

### `hydroflow_lang` Commit Statistics

<csr-read-only-do-not-edit/>

 - 23 commits contributed to the release.
 - 44 days passed between releases.
 - 21 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 15 unique issues were worked on: [#1143](https://github.com/hydro-project/hydroflow/issues/1143), [#1152](https://github.com/hydro-project/hydroflow/issues/1152), [#1157](https://github.com/hydro-project/hydroflow/issues/1157), [#1159](https://github.com/hydro-project/hydroflow/issues/1159), [#1160](https://github.com/hydro-project/hydroflow/issues/1160), [#1167](https://github.com/hydro-project/hydroflow/issues/1167), [#1171](https://github.com/hydro-project/hydroflow/issues/1171), [#1176](https://github.com/hydro-project/hydroflow/issues/1176), [#1182](https://github.com/hydro-project/hydroflow/issues/1182), [#1190](https://github.com/hydro-project/hydroflow/issues/1190), [#1192](https://github.com/hydro-project/hydroflow/issues/1192), [#1193](https://github.com/hydro-project/hydroflow/issues/1193), [#1198](https://github.com/hydro-project/hydroflow/issues/1198), [#1204](https://github.com/hydro-project/hydroflow/issues/1204), [#1232](https://github.com/hydro-project/hydroflow/issues/1232)

### `hydroflow_lang` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1143](https://github.com/hydro-project/hydroflow/issues/1143)**
    - Added poll_futures and poll_futures_async operators. ([`997d90a`](https://github.com/hydro-project/hydroflow/commit/997d90a76db9a4e05dbac35073a09548750ce342))
 * **[#1152](https://github.com/hydro-project/hydroflow/issues/1152)**
    - Update some compile-fail outputs caused by rustc regression ([`d2427e2`](https://github.com/hydro-project/hydroflow/commit/d2427e2cc901c4174830d41b4a1dfc52fd4f19ce))
    - Update pyo3, silence warnings in generated code ([`1801502`](https://github.com/hydro-project/hydroflow/commit/18015029a725b068696ed9edefd1097583c858a6))
    - Appease latest nightly clippy ([`d9b2c02`](https://github.com/hydro-project/hydroflow/commit/d9b2c0263d508e6f6855f49504896d4ea670c355))
 * **[#1157](https://github.com/hydro-project/hydroflow/issues/1157)**
    - Add support for collecting counts and running perf ([`29a263f`](https://github.com/hydro-project/hydroflow/commit/29a263fb564c5ce4bc495ea4e9d20b8b2621b645))
 * **[#1159](https://github.com/hydro-project/hydroflow/issues/1159)**
    - Change `lattice_bimorphism` to take state via singleton arguments #969 #1058 ([`4386fac`](https://github.com/hydro-project/hydroflow/commit/4386fac824d64f63eae7629292675ac6bc8df9f7))
    - Unify `process_singletons.rs` code ([`1057b27`](https://github.com/hydro-project/hydroflow/commit/1057b273cbd941c1a12a287580c15e264d797f2c))
    - Replace `state()` with `state_ref()` implementation ([`d7e579c`](https://github.com/hydro-project/hydroflow/commit/d7e579c39b370a0ea0b0385d1029e9f8a7351d68))
 * **[#1160](https://github.com/hydro-project/hydroflow/issues/1160)**
    - Remove reference edges, reverts #1017 #1041 ([`40f1a19`](https://github.com/hydro-project/hydroflow/commit/40f1a19ece2a8352e6fdc31f815d923e635f91b1))
 * **[#1167](https://github.com/hydro-project/hydroflow/issues/1167)**
    - Error on unused ports, fix #1108 ([`9df9c62`](https://github.com/hydro-project/hydroflow/commit/9df9c6251526903dbe7288e2fd9a532c63a9412c))
    - Reorder error message emission for conflicting varnames ([`20471f1`](https://github.com/hydro-project/hydroflow/commit/20471f11901e3fb15a2efea61752d836d4facba5))
 * **[#1171](https://github.com/hydro-project/hydroflow/issues/1171)**
    - Fix scheduler spinning on replay, fix #961 ([`d773f9a`](https://github.com/hydro-project/hydroflow/commit/d773f9a6938fe6d1521516f7a7c441c6c281a9fa))
 * **[#1176](https://github.com/hydro-project/hydroflow/issues/1176)**
    - Fix #1050 for `reduce_keyed`, fix #1050 ([`e2fa6b0`](https://github.com/hydro-project/hydroflow/commit/e2fa6b0729cb92b29e1d293c2788458845ff306a))
    - Cleanup codegen for `fold_keyed` ([`d57b911`](https://github.com/hydro-project/hydroflow/commit/d57b91146ef44125f1dd87e040ef636797f90e76))
    - Cleanup some ops formatting ([`2715350`](https://github.com/hydro-project/hydroflow/commit/271535091f6bd810ca8957c72dae357e3ddffa52))
 * **[#1182](https://github.com/hydro-project/hydroflow/issues/1182)**
    - Allow `defer_tick()`, `next_stratum()` to have a type hint arg ([`4ca8ce4`](https://github.com/hydro-project/hydroflow/commit/4ca8ce43c0998296e2d86bd74800585ebb24123a))
 * **[#1190](https://github.com/hydro-project/hydroflow/issues/1190)**
    - `source_interval` no longer emits Instant instances on output. ([`b292f11`](https://github.com/hydro-project/hydroflow/commit/b292f111715fd9c397ffb35cf991bd0bfb01c1e1))
 * **[#1192](https://github.com/hydro-project/hydroflow/issues/1192)**
    - Expect custom config names to prevent warnings ([`b86f11a`](https://github.com/hydro-project/hydroflow/commit/b86f11aad344fef6ad9cdd1db0b45bb738c48bd6))
 * **[#1193](https://github.com/hydro-project/hydroflow/issues/1193)**
    - Introduce newtypes for working with ticks ([`c2f6c95`](https://github.com/hydro-project/hydroflow/commit/c2f6c9578127a71c879752d52e115df75659e2b0))
 * **[#1198](https://github.com/hydro-project/hydroflow/issues/1198)**
    - Insert `persist()` before singleton referencers to enable replay ([`f184ea1`](https://github.com/hydro-project/hydroflow/commit/f184ea145f0c7c3072d7d0f94d42fcda717ac8d9))
 * **[#1204](https://github.com/hydro-project/hydroflow/issues/1204)**
    - Simplify `demux_enum()`, somewhat improves error messages #1201 ([`826dbd9`](https://github.com/hydro-project/hydroflow/commit/826dbd9a709de2f883992bdcefa8f2d566d74ecb))
 * **[#1232](https://github.com/hydro-project/hydroflow/issues/1232)**
    - Add `'static`/`'tick` support for `state()` operator ([`b0692b0`](https://github.com/hydro-project/hydroflow/commit/b0692b0d697980eaf9893c07a443a257e04786c5))
 * **Uncategorized**
    - Release hydroflow_lang v0.7.0, hydroflow_datalog_core v0.7.0, hydroflow_datalog v0.7.0, hydroflow_macro v0.7.0, lattices v0.5.5, multiplatform_test v0.1.0, pusherator v0.0.6, hydroflow v0.7.0, stageleft_macro v0.2.0, stageleft v0.3.0, stageleft_tool v0.2.0, hydroflow_plus v0.7.0, hydro_deploy v0.7.0, hydro_cli v0.7.0, hydroflow_plus_cli_integration v0.7.0, safety bump 8 crates ([`2852147`](https://github.com/hydro-project/hydroflow/commit/285214740627685e911781793e05d234ab2ad2bd))
</details>

## 0.6.2 (2024-04-09)

### New Features

<csr-id-0f16d1f50cd64d9ca52ec811acc4a643a86f14fe/>
<csr-id-5c5b6523d96a22c97382a7c61ee0e36ad77c0a0f/>

 - <csr-id-5679bfb7d3b96089cc020308c7d88021a254e63c/> allow `reduce()` to be referenceable as a singleton, fix docs and bugs
   * fixed bug: accumulator closures could have return values, which would be ignored
* updated docs

### Bug Fixes

 - <csr-id-82b3030eefb759a97053057f717efb95491802b3/> fix singleton reference edges being backwards, fix #1147
 - <csr-id-994990be0403738b46f9736efc24618950d3f50c/> Fix handling of single-node push subgraphs

### `hydroflow_lang` Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release.
 - 3 days passed between releases.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 3 unique issues were worked on: [#1134](https://github.com/hydro-project/hydroflow/issues/1134), [#1148](https://github.com/hydro-project/hydroflow/issues/1148), [#1150](https://github.com/hydro-project/hydroflow/issues/1150)

### `hydroflow_lang` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1134](https://github.com/hydro-project/hydroflow/issues/1134)**
    - Make fold output optional, usable as just a singleton reference ([`0f16d1f`](https://github.com/hydro-project/hydroflow/commit/0f16d1f50cd64d9ca52ec811acc4a643a86f14fe))
    - Fix handling of single-node push subgraphs ([`994990b`](https://github.com/hydro-project/hydroflow/commit/994990be0403738b46f9736efc24618950d3f50c))
    - Allow `fold()` to be referenceable as a singleton ([`5c5b652`](https://github.com/hydro-project/hydroflow/commit/5c5b6523d96a22c97382a7c61ee0e36ad77c0a0f))
 * **[#1148](https://github.com/hydro-project/hydroflow/issues/1148)**
    - Fix singleton reference edges being backwards, fix #1147 ([`82b3030`](https://github.com/hydro-project/hydroflow/commit/82b3030eefb759a97053057f717efb95491802b3))
 * **[#1150](https://github.com/hydro-project/hydroflow/issues/1150)**
    - Allow `reduce()` to be referenceable as a singleton, fix docs and bugs ([`5679bfb`](https://github.com/hydro-project/hydroflow/commit/5679bfb7d3b96089cc020308c7d88021a254e63c))
 * **Uncategorized**
    - Release hydroflow_lang v0.6.2, hydroflow v0.6.2, hydroflow_plus v0.6.1, hydro_deploy v0.6.1, hydro_cli v0.6.1, hydroflow_plus_cli_integration v0.6.1, stageleft_tool v0.1.1 ([`23cfe08`](https://github.com/hydro-project/hydroflow/commit/23cfe0839079aa17d042bbd3976f6d188689d290))
</details>

## 0.6.1 (2024-04-05)

<csr-id-1b19361a87bafb7d7c12be04cc2ce3370d71439a/>
<csr-id-88e2ca44fed2d54b969e54a948c41d3bcd9a2ab5/>
<csr-id-7958fb0d900be8fe7359326abfa11dcb8fb35e8a/>

### Documentation

 - <csr-id-242784737abd1b2bb8e12a5b58a0771e4929954e/> add docs for `state()` operator
 - <csr-id-550b17c0e0edd233b0cceab55fed0de309051503/> more improvements to `persist_mut[_keyed]` docs

### New Features

 - <csr-id-1bde8a2443523fa8d3aafd7459c9a484de7724b7/> Render singleton references in graphvis
 - <csr-id-54c6874931ebd7ba13140dbcf3e75ad9b2852331/> enable inspect to have no ouputs
 - <csr-id-fa34dd317ab0cb59753bd884acae2605a7630b10/> track which ops have singleton state
   Add a nice error message for referencing a non-singleton op
 - <csr-id-601c484bf9afe9aada61c1a40478c0e093140a56/> improve singleton error messages/handling
 - <csr-id-8cb29fa809e255195b65fd23b589bf7c970c5599/> enable singleton reference usage in all operators
   also cleans up local imports
 - <csr-id-f9b26b6df7ff7d2a3530527579c60f41d93e9d45/> switch singletons references to being per-op intead of per-edge
   Only testing in `filter` op for now
 - <csr-id-0a5e785d4bf0729ba0fe95466c822e8304f96e24/> initial proof-of-concept for singletons
 - <csr-id-602e21fc04da5a0d65ac7128688760c92ffba3c1/> include subgraph ID in pivot_run codegen
 - <csr-id-fd90d4130e86b16139f6d3a386fbbfb49fae5a9b/> add `kvs_mut` example, fix #785

### Bug Fixes

 - <csr-id-a0e829e835efcf643bf26ce472d9de6dce05bb15/> fix `state()` op push codegen
 - <csr-id-dfe5a1bc66564fbd6de297cd695385b627091658/> allow `null()` to receive reference input
 - <csr-id-f8311db2cd9628607887fc04f2ea5933c8b7c11e/> #1050 for `fold_keyed`

### Refactor

 - <csr-id-1b19361a87bafb7d7c12be04cc2ce3370d71439a/> consider singleton references as `DelayType::Stratum` barriers
 - <csr-id-88e2ca44fed2d54b969e54a948c41d3bcd9a2ab5/> add `#[repr(transparent)]` to type guard codegen
   theoretically could help performance but is probably negligible

### Style

 - <csr-id-7958fb0d900be8fe7359326abfa11dcb8fb35e8a/> qualified path cleanups for clippy

### `hydroflow_lang` Commit Statistics

<csr-read-only-do-not-edit/>

 - 18 commits contributed to the release.
 - 34 days passed between releases.
 - 17 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 13 unique issues were worked on: [#1086](https://github.com/hydro-project/hydroflow/issues/1086), [#1087](https://github.com/hydro-project/hydroflow/issues/1087), [#1089](https://github.com/hydro-project/hydroflow/issues/1089), [#1090](https://github.com/hydro-project/hydroflow/issues/1090), [#1091](https://github.com/hydro-project/hydroflow/issues/1091), [#1109](https://github.com/hydro-project/hydroflow/issues/1109), [#1128](https://github.com/hydro-project/hydroflow/issues/1128), [#1130](https://github.com/hydro-project/hydroflow/issues/1130), [#1133](https://github.com/hydro-project/hydroflow/issues/1133), [#1136](https://github.com/hydro-project/hydroflow/issues/1136), [#1137](https://github.com/hydro-project/hydroflow/issues/1137), [#1145](https://github.com/hydro-project/hydroflow/issues/1145), [#1146](https://github.com/hydro-project/hydroflow/issues/1146)

### `hydroflow_lang` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1086](https://github.com/hydro-project/hydroflow/issues/1086)**
    - #1050 for `fold_keyed` ([`f8311db`](https://github.com/hydro-project/hydroflow/commit/f8311db2cd9628607887fc04f2ea5933c8b7c11e))
 * **[#1087](https://github.com/hydro-project/hydroflow/issues/1087)**
    - Add `kvs_mut` example, fix #785 ([`fd90d41`](https://github.com/hydro-project/hydroflow/commit/fd90d4130e86b16139f6d3a386fbbfb49fae5a9b))
 * **[#1089](https://github.com/hydro-project/hydroflow/issues/1089)**
    - More improvements to `persist_mut[_keyed]` docs ([`550b17c`](https://github.com/hydro-project/hydroflow/commit/550b17c0e0edd233b0cceab55fed0de309051503))
 * **[#1090](https://github.com/hydro-project/hydroflow/issues/1090)**
    - Qualified path cleanups for clippy ([`7958fb0`](https://github.com/hydro-project/hydroflow/commit/7958fb0d900be8fe7359326abfa11dcb8fb35e8a))
 * **[#1091](https://github.com/hydro-project/hydroflow/issues/1091)**
    - Switch singletons references to being per-op intead of per-edge ([`f9b26b6`](https://github.com/hydro-project/hydroflow/commit/f9b26b6df7ff7d2a3530527579c60f41d93e9d45))
    - Initial proof-of-concept for singletons ([`0a5e785`](https://github.com/hydro-project/hydroflow/commit/0a5e785d4bf0729ba0fe95466c822e8304f96e24))
 * **[#1109](https://github.com/hydro-project/hydroflow/issues/1109)**
    - Add docs for `state()` operator ([`2427847`](https://github.com/hydro-project/hydroflow/commit/242784737abd1b2bb8e12a5b58a0771e4929954e))
 * **[#1128](https://github.com/hydro-project/hydroflow/issues/1128)**
    - Enable inspect to have no ouputs ([`54c6874`](https://github.com/hydro-project/hydroflow/commit/54c6874931ebd7ba13140dbcf3e75ad9b2852331))
 * **[#1130](https://github.com/hydro-project/hydroflow/issues/1130)**
    - Fix `state()` op push codegen ([`a0e829e`](https://github.com/hydro-project/hydroflow/commit/a0e829e835efcf643bf26ce472d9de6dce05bb15))
 * **[#1133](https://github.com/hydro-project/hydroflow/issues/1133)**
    - Track which ops have singleton state ([`fa34dd3`](https://github.com/hydro-project/hydroflow/commit/fa34dd317ab0cb59753bd884acae2605a7630b10))
    - Improve singleton error messages/handling ([`601c484`](https://github.com/hydro-project/hydroflow/commit/601c484bf9afe9aada61c1a40478c0e093140a56))
    - Enable singleton reference usage in all operators ([`8cb29fa`](https://github.com/hydro-project/hydroflow/commit/8cb29fa809e255195b65fd23b589bf7c970c5599))
 * **[#1136](https://github.com/hydro-project/hydroflow/issues/1136)**
    - Add `#[repr(transparent)]` to type guard codegen ([`88e2ca4`](https://github.com/hydro-project/hydroflow/commit/88e2ca44fed2d54b969e54a948c41d3bcd9a2ab5))
 * **[#1137](https://github.com/hydro-project/hydroflow/issues/1137)**
    - Include subgraph ID in pivot_run codegen ([`602e21f`](https://github.com/hydro-project/hydroflow/commit/602e21fc04da5a0d65ac7128688760c92ffba3c1))
 * **[#1145](https://github.com/hydro-project/hydroflow/issues/1145)**
    - Consider singleton references as `DelayType::Stratum` barriers ([`1b19361`](https://github.com/hydro-project/hydroflow/commit/1b19361a87bafb7d7c12be04cc2ce3370d71439a))
 * **[#1146](https://github.com/hydro-project/hydroflow/issues/1146)**
    - Render singleton references in graphvis ([`1bde8a2`](https://github.com/hydro-project/hydroflow/commit/1bde8a2443523fa8d3aafd7459c9a484de7724b7))
 * **Uncategorized**
    - Release hydroflow_cli_integration v0.5.2, hydroflow_lang v0.6.1, hydroflow_datalog_core v0.6.1, lattices v0.5.4, hydroflow v0.6.1, stageleft_macro v0.1.1, stageleft v0.2.1, hydroflow_plus v0.6.1, hydro_deploy v0.6.1, hydro_cli v0.6.1, hydroflow_plus_cli_integration v0.6.1, stageleft_tool v0.1.1 ([`cd63f22`](https://github.com/hydro-project/hydroflow/commit/cd63f2258c961a40f0e5dbef20ac329a2d570ad0))
    - Allow `null()` to receive reference input ([`dfe5a1b`](https://github.com/hydro-project/hydroflow/commit/dfe5a1bc66564fbd6de297cd695385b627091658))
</details>

## 0.6.0 (2024-03-02)

### New Features

 - <csr-id-09f0c5711c2d038d1d5764d400fe6206da995ce3/> add (inefficient) `lattice_bimorphism` operator, fix #1073
 - <csr-id-eb34ccd13f56e1d07cbae35ead79daeb3b9bad20/> use an IR before lowering to Hydroflow
   Makes it possible to write custom optimization passes.

### `hydroflow_lang` Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 28 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#1061](https://github.com/hydro-project/hydroflow/issues/1061), [#1070](https://github.com/hydro-project/hydroflow/issues/1070)

### `hydroflow_lang` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1061](https://github.com/hydro-project/hydroflow/issues/1061)**
    - Add (inefficient) `lattice_bimorphism` operator, fix #1073 ([`09f0c57`](https://github.com/hydro-project/hydroflow/commit/09f0c5711c2d038d1d5764d400fe6206da995ce3))
 * **[#1070](https://github.com/hydro-project/hydroflow/issues/1070)**
    - Use an IR before lowering to Hydroflow ([`eb34ccd`](https://github.com/hydro-project/hydroflow/commit/eb34ccd13f56e1d07cbae35ead79daeb3b9bad20))
 * **Uncategorized**
    - Release hydroflow_lang v0.6.0, hydroflow_datalog_core v0.6.0, hydroflow_datalog v0.6.0, hydroflow_macro v0.6.0, lattices v0.5.3, variadics v0.0.4, pusherator v0.0.5, hydroflow v0.6.0, stageleft v0.2.0, hydroflow_plus v0.6.0, hydro_deploy v0.6.0, hydro_cli v0.6.0, hydroflow_plus_cli_integration v0.6.0, safety bump 7 crates ([`09ea65f`](https://github.com/hydro-project/hydroflow/commit/09ea65fe9cd45c357c43bffca30e60243fa45cc8))
</details>

## 0.5.2 (2024-02-02)

### Documentation

 - <csr-id-56fbc34bec94bc87b2c5d4a6fc68ced7f6fc43a3/> `source_iter_delta` docs, fix #962

### New Features

 - <csr-id-7a791b8ccc489050ef10ddb186409cc046bd30f0/> implement state operator
   (#929)
 - <csr-id-420580d5bd94aa9caa531690926181ae5144753f/> generate different idents for ref edges state
   (#929)
 - <csr-id-fa8cfdb41e0c1dded8bd973fffa4398e34d4d536/> Add `input/output_edgtypes` field to `WriteContextArgs`
   (#929)
 - <csr-id-5969bc968bb2bd033ee6330e8a172459425a1b5d/> subgraph partitioning handling (ignoring) reference edges
   (#929)
   
   (currently panics)
 - <csr-id-f661f01016790bb15a241fecb32ab7e9af82e665/> Add `state`, `state_join` ops for testing ref edges
   (#929)

### Bug Fixes

 - <csr-id-a572dee118fb9d014954a18d560413a6c6fc5deb/> fix coloring of `_lattice_join_fused_join`, fix #943
 - <csr-id-175cc20e15b90627ae86d488e31ec91278c8beeb/> ensure other graph methods handle edge types
   (#929)

### `hydroflow_lang` Commit Statistics

<csr-read-only-do-not-edit/>

 - 9 commits contributed to the release.
 - 4 days passed between releases.
 - 8 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 3 unique issues were worked on: [#1041](https://github.com/hydro-project/hydroflow/issues/1041), [#1053](https://github.com/hydro-project/hydroflow/issues/1053), [#1055](https://github.com/hydro-project/hydroflow/issues/1055)

### `hydroflow_lang` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1041](https://github.com/hydro-project/hydroflow/issues/1041)**
    - Ensure other graph methods handle edge types ([`175cc20`](https://github.com/hydro-project/hydroflow/commit/175cc20e15b90627ae86d488e31ec91278c8beeb))
    - Implement state operator ([`7a791b8`](https://github.com/hydro-project/hydroflow/commit/7a791b8ccc489050ef10ddb186409cc046bd30f0))
    - Generate different idents for ref edges state ([`420580d`](https://github.com/hydro-project/hydroflow/commit/420580d5bd94aa9caa531690926181ae5144753f))
    - Add `input/output_edgtypes` field to `WriteContextArgs` ([`fa8cfdb`](https://github.com/hydro-project/hydroflow/commit/fa8cfdb41e0c1dded8bd973fffa4398e34d4d536))
    - Subgraph partitioning handling (ignoring) reference edges ([`5969bc9`](https://github.com/hydro-project/hydroflow/commit/5969bc968bb2bd033ee6330e8a172459425a1b5d))
    - Add `state`, `state_join` ops for testing ref edges ([`f661f01`](https://github.com/hydro-project/hydroflow/commit/f661f01016790bb15a241fecb32ab7e9af82e665))
 * **[#1053](https://github.com/hydro-project/hydroflow/issues/1053)**
    - `source_iter_delta` docs, fix #962 ([`56fbc34`](https://github.com/hydro-project/hydroflow/commit/56fbc34bec94bc87b2c5d4a6fc68ced7f6fc43a3))
 * **[#1055](https://github.com/hydro-project/hydroflow/issues/1055)**
    - Fix coloring of `_lattice_join_fused_join`, fix #943 ([`a572dee`](https://github.com/hydro-project/hydroflow/commit/a572dee118fb9d014954a18d560413a6c6fc5deb))
 * **Uncategorized**
    - Release hydroflow_lang v0.5.2, hydroflow_datalog_core v0.5.2, hydroflow_macro v0.5.2, lattices v0.5.2, hydroflow v0.5.2, hydro_cli v0.5.1, hydroflow_plus_cli_integration v0.5.1 ([`6ac8720`](https://github.com/hydro-project/hydroflow/commit/6ac872081753548ebb8ec95549b4d820dc050d3e))
</details>

## 0.5.1 (2024-01-29)

<csr-id-1b555e57c8c812bed4d6495d2960cbf77fb0b3ef/>
<csr-id-ba6afab8416ad66eee4fdb9d0c73e62d45752617/>
<csr-id-2b0a6672b06eb1d71d4602eec296b5ce55ea293e/>
<csr-id-1a80f1cd57e6f3a5ee806e1bf3b8ad59dcecfff7/>
<csr-id-ff4bddd844969a9e8da5e8a1948712567a6e39bb/>

### Chore

 - <csr-id-1b555e57c8c812bed4d6495d2960cbf77fb0b3ef/> manually set lockstep-versioned crates (and `lattices`) to version `0.5.1`
   Setting manually since
   https://github.com/frewsxcv/rust-crates-index/issues/159 is messing with
   smart-release
 - <csr-id-ba6afab8416ad66eee4fdb9d0c73e62d45752617/> fix clippy lints on latest nightly

### New Features

 - <csr-id-5a03ed41548b5766b945efbd1eedb0dfceb714d9/> add core negation operators
 - <csr-id-355bcd1fb013124dd2991fabf0fff0e4c451ef62/> add checking of input edge types via `OperatorConstraints::input_edgetype_fn`
 - <csr-id-e61d22dd0cc3f88e76969fec2ae5c13bf8c234cf/> add `OperatorConstraints::input_edgetype_fn` to validate input ref/val edges
 - <csr-id-67c4195d538dbdef9a6ce48058d7647127eb65c6/> add operator edge type tracking into meta graph
 - <csr-id-cdbc43336e53891658b6d34cc2e45be94f5d8320/> add `OperatorConstraints::output_edgetype_fn` to enable reference edges
 - <csr-id-73e9b68ec2f5b2627784addcce9fba684848bb55/> implement keyed fold and reduce
 - <csr-id-af6e3be60fdb69ceec1613347910f4dd49980d34/> push down persists and implement Pi example
   Also fixes type inference issues with reduce the same way as we did for fold.
 - <csr-id-a0af314a032096fc94b9f4aabb21aadc8184fb30/> Add initial structure for by-reference edge types
 - <csr-id-e5bdd12e32d6ea72fd91a55c12e09f07a0edaa5c/> add initial test using Hydro CLI from Hydroflow+
   This also required a change to Hydroflow core to make it possible to run the dataflow itself on a single thread (using a LocalSet), even if the surrounding runtime is not single-threaded (required to work around deadlocks because we can't use async APIs inside Hydroflow+). This requires us to spawn any Hydroflow tasks (only for `dest_sink` at the moment) right next to when we run the dataflow rather than when the Hydroflow graph is initialized. From a conceptual perspective, this seems _more right_, since now creating a Hydroflow program will not result in any actual tasks running.
   
   In the third PR of this series, I aim to add a new Hydroflow+ operator that will automate the setup of a `dest_sink`/`source_stream` pair that span nodes.
 - <csr-id-8b635683e5ac3c4ed2d896ae88e2953db1c6312c/> add a functional surface syntax using staging
 - <csr-id-7df0a0df61597764eed763b68138929fed1413ac/> add defer() which is the same as defer_tick() except that it is lazy

### Bug Fixes

<csr-id-f0a03786b47d590477f8169bb0a40fd4981fef9e/>
<csr-id-d0b0a35fa5ed1fdbfd2c2dc5034a3ec52a078779/>
<csr-id-38411ea007d4feb30dd16bdd1505802a111a67d1/>
<csr-id-43280cb698cf6bc070483365ee272106c271dca4/>
<csr-id-f89d11a9c8c6712183c76a193674aba21349675e/>
<csr-id-35b1e9e83f2a0cfa171b4994a2cffb0d22706abf/>
<csr-id-8ef14a396c5c56789e2993284b96234ad5032be1/>

 - <csr-id-a67f43f35ec4eada3aab69781234c9d3d82648e8/> typo
 - <csr-id-5ed9be478daf4fef91c6d35893f68944da8eac94/> lattice ops are monotone and return stream of lattices
   * fix: lattice ops are monotone and return stream of lattices
* fix: remove trailing whitespace
* fix: remove warnings from unused tees in topolotree
* fix: avoid error message in test
* fix: set `flow_prop_fn` properly in `_lattice_fold_batch.rs`
* fix: test_lattice_join_fused_join_map_union now checks assertions

### Refactor

 - <csr-id-2b0a6672b06eb1d71d4602eec296b5ce55ea293e/> unify node coloring code
 - <csr-id-1a80f1cd57e6f3a5ee806e1bf3b8ad59dcecfff7/> emit prologue code before all subgraph code
   Before, prologue code would be emitted before its subgraph, resulting in
   interleaving between subgraphs.
 - <csr-id-ff4bddd844969a9e8da5e8a1948712567a6e39bb/> remove old unused structured `FlowProperties`

### Bug Fixes (BREAKING)

 - <csr-id-3136e0f286f87e944e7f718d926fd7670b44194b/> fold takes initial value by closure rather than by value

### `hydroflow_lang` Commit Statistics

<csr-read-only-do-not-edit/>

 - 28 commits contributed to the release.
 - 110 days passed between releases.
 - 26 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 20 unique issues were worked on: [#1005](https://github.com/hydro-project/hydroflow/issues/1005), [#1009](https://github.com/hydro-project/hydroflow/issues/1009), [#1016](https://github.com/hydro-project/hydroflow/issues/1016), [#1017](https://github.com/hydro-project/hydroflow/issues/1017), [#1021](https://github.com/hydro-project/hydroflow/issues/1021), [#1023](https://github.com/hydro-project/hydroflow/issues/1023), [#1026](https://github.com/hydro-project/hydroflow/issues/1026), [#1033](https://github.com/hydro-project/hydroflow/issues/1033), [#1036](https://github.com/hydro-project/hydroflow/issues/1036), [#1040](https://github.com/hydro-project/hydroflow/issues/1040), [#899](https://github.com/hydro-project/hydroflow/issues/899), [#945](https://github.com/hydro-project/hydroflow/issues/945), [#947](https://github.com/hydro-project/hydroflow/issues/947), [#948](https://github.com/hydro-project/hydroflow/issues/948), [#949](https://github.com/hydro-project/hydroflow/issues/949), [#950](https://github.com/hydro-project/hydroflow/issues/950), [#959](https://github.com/hydro-project/hydroflow/issues/959), [#960](https://github.com/hydro-project/hydroflow/issues/960), [#978](https://github.com/hydro-project/hydroflow/issues/978), [#989](https://github.com/hydro-project/hydroflow/issues/989)

### `hydroflow_lang` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1005](https://github.com/hydro-project/hydroflow/issues/1005)**
    - Improve type inference for fold accumulators ([`f0a0378`](https://github.com/hydro-project/hydroflow/commit/f0a03786b47d590477f8169bb0a40fd4981fef9e))
 * **[#1009](https://github.com/hydro-project/hydroflow/issues/1009)**
    - Clippy lints on latest nightly ([`d0b0a35`](https://github.com/hydro-project/hydroflow/commit/d0b0a35fa5ed1fdbfd2c2dc5034a3ec52a078779))
 * **[#1016](https://github.com/hydro-project/hydroflow/issues/1016)**
    - Add initial structure for by-reference edge types ([`a0af314`](https://github.com/hydro-project/hydroflow/commit/a0af314a032096fc94b9f4aabb21aadc8184fb30))
 * **[#1017](https://github.com/hydro-project/hydroflow/issues/1017)**
    - Fixup! feat(hydroflow_lang): add `OperatorConstraints::input_edgetype_fn` to validate input ref/val edges ([`f079b85`](https://github.com/hydro-project/hydroflow/commit/f079b85aab57364c070bfb35aa28419e3876b1de))
    - Add checking of input edge types via `OperatorConstraints::input_edgetype_fn` ([`355bcd1`](https://github.com/hydro-project/hydroflow/commit/355bcd1fb013124dd2991fabf0fff0e4c451ef62))
    - Typo ([`a67f43f`](https://github.com/hydro-project/hydroflow/commit/a67f43f35ec4eada3aab69781234c9d3d82648e8))
    - Add `OperatorConstraints::input_edgetype_fn` to validate input ref/val edges ([`e61d22d`](https://github.com/hydro-project/hydroflow/commit/e61d22dd0cc3f88e76969fec2ae5c13bf8c234cf))
    - Add operator edge type tracking into meta graph ([`67c4195`](https://github.com/hydro-project/hydroflow/commit/67c4195d538dbdef9a6ce48058d7647127eb65c6))
    - Add `OperatorConstraints::output_edgetype_fn` to enable reference edges ([`cdbc433`](https://github.com/hydro-project/hydroflow/commit/cdbc43336e53891658b6d34cc2e45be94f5d8320))
 * **[#1021](https://github.com/hydro-project/hydroflow/issues/1021)**
    - Push down persists and implement Pi example ([`af6e3be`](https://github.com/hydro-project/hydroflow/commit/af6e3be60fdb69ceec1613347910f4dd49980d34))
 * **[#1023](https://github.com/hydro-project/hydroflow/issues/1023)**
    - Implement keyed fold and reduce ([`73e9b68`](https://github.com/hydro-project/hydroflow/commit/73e9b68ec2f5b2627784addcce9fba684848bb55))
 * **[#1026](https://github.com/hydro-project/hydroflow/issues/1026)**
    - Lattice ops are monotone and return stream of lattices ([`5ed9be4`](https://github.com/hydro-project/hydroflow/commit/5ed9be478daf4fef91c6d35893f68944da8eac94))
 * **[#1033](https://github.com/hydro-project/hydroflow/issues/1033)**
    - Emit prologue code before all subgraph code ([`1a80f1c`](https://github.com/hydro-project/hydroflow/commit/1a80f1cd57e6f3a5ee806e1bf3b8ad59dcecfff7))
 * **[#1036](https://github.com/hydro-project/hydroflow/issues/1036)**
    - Add core negation operators ([`5a03ed4`](https://github.com/hydro-project/hydroflow/commit/5a03ed41548b5766b945efbd1eedb0dfceb714d9))
 * **[#1040](https://github.com/hydro-project/hydroflow/issues/1040)**
    - Unify node coloring code ([`2b0a667`](https://github.com/hydro-project/hydroflow/commit/2b0a6672b06eb1d71d4602eec296b5ce55ea293e))
 * **[#899](https://github.com/hydro-project/hydroflow/issues/899)**
    - Add a functional surface syntax using staging ([`8b63568`](https://github.com/hydro-project/hydroflow/commit/8b635683e5ac3c4ed2d896ae88e2953db1c6312c))
 * **[#945](https://github.com/hydro-project/hydroflow/issues/945)**
    - Add defer() which is the same as defer_tick() except that it is lazy ([`7df0a0d`](https://github.com/hydro-project/hydroflow/commit/7df0a0df61597764eed763b68138929fed1413ac))
 * **[#947](https://github.com/hydro-project/hydroflow/issues/947)**
    - Remove old unused structured `FlowProperties` ([`ff4bddd`](https://github.com/hydro-project/hydroflow/commit/ff4bddd844969a9e8da5e8a1948712567a6e39bb))
 * **[#948](https://github.com/hydro-project/hydroflow/issues/948)**
    - Fold takes initial value by closure rather than by value ([`3136e0f`](https://github.com/hydro-project/hydroflow/commit/3136e0f286f87e944e7f718d926fd7670b44194b))
 * **[#949](https://github.com/hydro-project/hydroflow/issues/949)**
    - Fix/improve rendering with `--no-handoffs` and double-labelled edges ([`8ef14a3`](https://github.com/hydro-project/hydroflow/commit/8ef14a396c5c56789e2993284b96234ad5032be1))
 * **[#950](https://github.com/hydro-project/hydroflow/issues/950)**
    - Avoid panic-ing on degen `null()` ([`35b1e9e`](https://github.com/hydro-project/hydroflow/commit/35b1e9e83f2a0cfa171b4994a2cffb0d22706abf))
 * **[#959](https://github.com/hydro-project/hydroflow/issues/959)**
    - `multiset_delta` incorrect `is_first_run_this_tick` check, fixes #958 ([`43280cb`](https://github.com/hydro-project/hydroflow/commit/43280cb698cf6bc070483365ee272106c271dca4))
 * **[#960](https://github.com/hydro-project/hydroflow/issues/960)**
    - Fix clippy lints on latest nightly ([`ba6afab`](https://github.com/hydro-project/hydroflow/commit/ba6afab8416ad66eee4fdb9d0c73e62d45752617))
 * **[#978](https://github.com/hydro-project/hydroflow/issues/978)**
    - Add initial test using Hydro CLI from Hydroflow+ ([`e5bdd12`](https://github.com/hydro-project/hydroflow/commit/e5bdd12e32d6ea72fd91a55c12e09f07a0edaa5c))
 * **[#989](https://github.com/hydro-project/hydroflow/issues/989)**
    - Fix spelling of "propagate" ([`38411ea`](https://github.com/hydro-project/hydroflow/commit/38411ea007d4feb30dd16bdd1505802a111a67d1))
 * **Uncategorized**
    - Release hydroflow_cli_integration v0.5.1, hydroflow_lang v0.5.1, hydroflow_datalog_core v0.5.1, hydroflow_datalog v0.5.1, hydroflow_macro v0.5.1, lattices v0.5.1, variadics v0.0.3, pusherator v0.0.4, hydroflow v0.5.1, stageleft_macro v0.1.0, stageleft v0.1.0, hydroflow_plus v0.5.1, hydro_deploy v0.5.1, hydro_cli v0.5.1 ([`478aebc`](https://github.com/hydro-project/hydroflow/commit/478aebc8fee2aa78eab86bd386322db1c70bde6a))
    - Manually set lockstep-versioned crates (and `lattices`) to version `0.5.1` ([`1b555e5`](https://github.com/hydro-project/hydroflow/commit/1b555e57c8c812bed4d6495d2960cbf77fb0b3ef))
    - 2 nested module imports bugs ([`f89d11a`](https://github.com/hydro-project/hydroflow/commit/f89d11a9c8c6712183c76a193674aba21349675e))
</details>

## 0.5.0 (2023-10-11)

<csr-id-594375803750056ac03b27e160a67bbd4ed9b71a/>
<csr-id-f19eccc79d6d7c88de7ba1ef6a0abf1caaef377f/>
<csr-id-1fb753ea85511ade1a834ec2536f56358ade9858/>
<csr-id-e788989737fbd501173bc99c6f9f5f5ba514ec9c/>
<csr-id-cb90ae184151ab9085ecb6d58f11d668619af9df/>
<csr-id-1126266e69c2c4364bc8de558f11859e5bad1c69/>
<csr-id-2e61c62cd866e37793a161b2f517296b93e8078d/>

### Chore

 - <csr-id-594375803750056ac03b27e160a67bbd4ed9b71a/> cleanup cli quotes, loose TODO comment
 - <csr-id-f19eccc79d6d7c88de7ba1ef6a0abf1caaef377f/> bump proc-macro2 min version to 1.0.63
 - <csr-id-1fb753ea85511ade1a834ec2536f56358ade9858/> ignore `clippy::unwrap_or_default` in `fold_keyed` codegen
 - <csr-id-e788989737fbd501173bc99c6f9f5f5ba514ec9c/> Fix `clippy::implied_bounds_in_impls` from latest nightlies

### Documentation

 - <csr-id-a6f3c646c7204509eec40e7e3b259886e15fec75/> add docs for the hydroflow surface syntax compilation process

### New Features

<csr-id-e7ea6d804ae162c0d7ecbd6e4cbc1084766ce506/>
<csr-id-9646ca06e61af8c827e2d2fb9826ce62b70b6799/>
<csr-id-02fddd2c0d99956d89f36395b283b198046b8766/>
<csr-id-b3d114827256f2b82a3c357f3419c6853a97f5c0/>
<csr-id-fc2543359ba11c0947fdc26f5360b2ac43a5a0c4/>
<csr-id-d254e2deb883f9633f8b325a595fb7c61bad42d7/>
<csr-id-1ce5f01cde288930cb1281468966dfb66d2e3e53/>
<csr-id-f013c3ca15f2cc9413fcfb92898f71d5fc00073a/>
<csr-id-1bdbf73b630e4f2eff009b00b0e66d71be53bb4a/>
<csr-id-63c435c32d170dcb6f1ee2a8da74b528d68e8e50/>
<csr-id-9baf80ccc38c4e41c8a1a2ae048036cec2b723c6/>
<csr-id-fd89cb46c5983d277e16bb7b19f7d3ca83dd60cc/>
<csr-id-38346cf01aec0afa2b491095043aa31587613e24/>
<csr-id-9ab7cf8199ddfa8a6a83b7e5f5bc5e6dc05a3110/>
<csr-id-7714403e130969b96c8f405444d4daf451450fdf/>
<csr-id-008b980a70561aa45c24d9a00d0908121d2a5ac6/>
<csr-id-fd5cdb583cb5b63dca790825d70836ea547d3d81/>
<csr-id-b2ca4b723c4a78020202d6eb06969a8c85ff5c01/>
<csr-id-686c2752e5c82a7f61a7a2aa4e6f6db52741e509/>

 - <csr-id-13fab158818b3e75dccd2a3dfbead7f79801dd32/> Add `--no-handoffs` option to graphwrite args
 - <csr-id-6dbbf35b6e5ae7f0225ac05c85598d4962ec66d8/> Add `--op-short-text` and `--no-pull-push` graphwrite args
 - <csr-id-d38ec080ba195acf52997d4a0f7296e43270ad8b/> add kvs with replication example
   have both kvs_replicated and kvs, separate examples
   
   add `flow_props_fn`s to `cross_join` (same as `join`), and `filter`
 - <csr-id-21140f09156e1dad195162854955522f138ae781/> update snapshot tests for previous two commits
 - <csr-id-9686ae8e7d26bb9cf6879a52d2324aa655588ec8/> update propegate_flow_props fn to reach fixed-point
 - <csr-id-cff7e48d611e4eb8e7e020bb3def5cf22744567a/> Add `flow_prop_fn`s to many operators
   * `_upcast` (new)
* `demux_enum`
* `for_each`
* `inspect`
* `join`
* `HydroflowGraph::open_mermaid()` opens https://mermaid.live/
* `HydroflowGraph::open_dot()` opens https://dreampuf.github.io/GraphvizOnline/

### Bug Fixes

 - <csr-id-2edf77961ca0218265b35f179c2d86c810795266/> restore in-subgraph rendering of self-handoffs
 - <csr-id-f67b9f0f9414977c24eace1e95ce840094be67a4/> fix handling of whitespace in `cast` expressions
 - <csr-id-a927dc6afbe3178815b7c7c58ed2838d42d80334/> clippy warning on multiline string in hydro_cli, py_udf
 - <csr-id-5a7e1b157362b0d655a28d6f3e5cd139ab8799f3/> fix demux error messages and add tests
 - <csr-id-51a200a444a42f21e6557f3b20d822aea8ccc670/> clippy redundant `to_string()` in `print!` lints
 - <csr-id-159a262ba056ec6ffad5590c4f3e57422022901e/> Clean up degenerate subgraph error message for consistency
   Makes the pinned and latest nightly version have the same stderr output
   for consistent testing.
 - <csr-id-5ac9ddebedf615f87684d1092382ba64826c1c1c/> separate internal compiler operators in docs name/category/sort order

### Refactor

 - <csr-id-cb90ae184151ab9085ecb6d58f11d668619af9df/> cleanup kvs example more
   Add `persist` `flow_prop_fn`
 - <csr-id-1126266e69c2c4364bc8de558f11859e5bad1c69/> `demux_enum` requires enum type name, add better error handling
 - <csr-id-2e61c62cd866e37793a161b2f517296b93e8078d/> combine `topo_sort` and `scc_kosaraju` into `topo_sort_scc`

### New Features (BREAKING)

 - <csr-id-9ed0ce02128a0eeaf0b603efcbe896427e47ef62/> Simplify graph printing code, add delta/cumul green edges, allow hiding of vars/subgraphs

### `hydroflow_lang` Commit Statistics

<csr-read-only-do-not-edit/>

 - 42 commits contributed to the release.
 - 56 days passed between releases.
 - 41 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 14 unique issues were worked on: [#882](https://github.com/hydro-project/hydroflow/issues/882), [#883](https://github.com/hydro-project/hydroflow/issues/883), [#884](https://github.com/hydro-project/hydroflow/issues/884), [#892](https://github.com/hydro-project/hydroflow/issues/892), [#896](https://github.com/hydro-project/hydroflow/issues/896), [#898](https://github.com/hydro-project/hydroflow/issues/898), [#902](https://github.com/hydro-project/hydroflow/issues/902), [#906](https://github.com/hydro-project/hydroflow/issues/906), [#923](https://github.com/hydro-project/hydroflow/issues/923), [#924](https://github.com/hydro-project/hydroflow/issues/924), [#926](https://github.com/hydro-project/hydroflow/issues/926), [#932](https://github.com/hydro-project/hydroflow/issues/932), [#933](https://github.com/hydro-project/hydroflow/issues/933), [#935](https://github.com/hydro-project/hydroflow/issues/935)

### `hydroflow_lang` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#882](https://github.com/hydro-project/hydroflow/issues/882)**
    - Add `Cumul` `flow_prop_fn`s for `lattice_fold()` and `lattice_reduce()` ([`63c435c`](https://github.com/hydro-project/hydroflow/commit/63c435c32d170dcb6f1ee2a8da74b528d68e8e50))
    - Update dot/graphviz rendering of delta/cumul and `defer_tick` edges ([`9baf80c`](https://github.com/hydro-project/hydroflow/commit/9baf80ccc38c4e41c8a1a2ae048036cec2b723c6))
    - Make `propegate_flow_props` fallible, cleanup `flow_prop_fn` definition. ([`fd89cb4`](https://github.com/hydro-project/hydroflow/commit/fd89cb46c5983d277e16bb7b19f7d3ca83dd60cc))
    - Add `cast` operator ([`38346cf`](https://github.com/hydro-project/hydroflow/commit/38346cf01aec0afa2b491095043aa31587613e24))
    - Update mermaid rendering of hydroflow graph to show flow properties ([`9ab7cf8`](https://github.com/hydro-project/hydroflow/commit/9ab7cf8199ddfa8a6a83b7e5f5bc5e6dc05a3110))
    - Add `monotonic_fn` and `morphism` macros, update snapshots for flow props. ([`7714403`](https://github.com/hydro-project/hydroflow/commit/7714403e130969b96c8f405444d4daf451450fdf))
    - Move structs into separate `flow_props` module, make `flow_prop_fn` return `Option`s, impl for `map` ([`008b980`](https://github.com/hydro-project/hydroflow/commit/008b980a70561aa45c24d9a00d0908121d2a5ac6))
    - Add `source_iter_delta` op for testing, basic flow props test, cleanups. ([`fd5cdb5`](https://github.com/hydro-project/hydroflow/commit/fd5cdb583cb5b63dca790825d70836ea547d3d81))
    - Implement basic flow prop traversal (untested) ([`b2ca4b7`](https://github.com/hydro-project/hydroflow/commit/b2ca4b723c4a78020202d6eb06969a8c85ff5c01))
    - Setup structure for tracking flow properties ([`686c275`](https://github.com/hydro-project/hydroflow/commit/686c2752e5c82a7f61a7a2aa4e6f6db52741e509))
 * **[#883](https://github.com/hydro-project/hydroflow/issues/883)**
    - Combine `topo_sort` and `scc_kosaraju` into `topo_sort_scc` ([`2e61c62`](https://github.com/hydro-project/hydroflow/commit/2e61c62cd866e37793a161b2f517296b93e8078d))
    - Add docs for the hydroflow surface syntax compilation process ([`a6f3c64`](https://github.com/hydro-project/hydroflow/commit/a6f3c646c7204509eec40e7e3b259886e15fec75))
 * **[#884](https://github.com/hydro-project/hydroflow/issues/884)**
    - Separate internal compiler operators in docs name/category/sort order ([`5ac9dde`](https://github.com/hydro-project/hydroflow/commit/5ac9ddebedf615f87684d1092382ba64826c1c1c))
 * **[#892](https://github.com/hydro-project/hydroflow/issues/892)**
    - Clean up degenerate subgraph error message for consistency ([`159a262`](https://github.com/hydro-project/hydroflow/commit/159a262ba056ec6ffad5590c4f3e57422022901e))
 * **[#896](https://github.com/hydro-project/hydroflow/issues/896)**
    - Ignore `clippy::unwrap_or_default` in `fold_keyed` codegen ([`1fb753e`](https://github.com/hydro-project/hydroflow/commit/1fb753ea85511ade1a834ec2536f56358ade9858))
 * **[#898](https://github.com/hydro-project/hydroflow/issues/898)**
    - Add import!() expression ([`f013c3c`](https://github.com/hydro-project/hydroflow/commit/f013c3ca15f2cc9413fcfb92898f71d5fc00073a))
 * **[#902](https://github.com/hydro-project/hydroflow/issues/902)**
    - Make lattice_fold and lattice_reduce consistent with fold/reduce ([`1ce5f01`](https://github.com/hydro-project/hydroflow/commit/1ce5f01cde288930cb1281468966dfb66d2e3e53))
 * **[#906](https://github.com/hydro-project/hydroflow/issues/906)**
    - Add context.is_first_time_subgraph_is_scheduled to simplify replaying operators ([`d254e2d`](https://github.com/hydro-project/hydroflow/commit/d254e2deb883f9633f8b325a595fb7c61bad42d7))
 * **[#923](https://github.com/hydro-project/hydroflow/issues/923)**
    - Open mermaid/dot graph in browser ([`e7ea6d8`](https://github.com/hydro-project/hydroflow/commit/e7ea6d804ae162c0d7ecbd6e4cbc1084766ce506))
 * **[#924](https://github.com/hydro-project/hydroflow/issues/924)**
    - Update snapshot tests for previous two commits ([`21140f0`](https://github.com/hydro-project/hydroflow/commit/21140f09156e1dad195162854955522f138ae781))
    - Update propegate_flow_props fn to reach fixed-point ([`9686ae8`](https://github.com/hydro-project/hydroflow/commit/9686ae8e7d26bb9cf6879a52d2324aa655588ec8))
    - Add `flow_prop_fn`s to many operators ([`cff7e48`](https://github.com/hydro-project/hydroflow/commit/cff7e48d611e4eb8e7e020bb3def5cf22744567a))
 * **[#926](https://github.com/hydro-project/hydroflow/issues/926)**
    - Cleanup cli quotes, loose TODO comment ([`5943758`](https://github.com/hydro-project/hydroflow/commit/594375803750056ac03b27e160a67bbd4ed9b71a))
    - Add kvs with replication example ([`d38ec08`](https://github.com/hydro-project/hydroflow/commit/d38ec080ba195acf52997d4a0f7296e43270ad8b))
    - Cleanup kvs example more ([`cb90ae1`](https://github.com/hydro-project/hydroflow/commit/cb90ae184151ab9085ecb6d58f11d668619af9df))
 * **[#932](https://github.com/hydro-project/hydroflow/issues/932)**
    - Add `--no-handoffs` option to graphwrite args ([`13fab15`](https://github.com/hydro-project/hydroflow/commit/13fab158818b3e75dccd2a3dfbead7f79801dd32))
    - Add `--op-short-text` and `--no-pull-push` graphwrite args ([`6dbbf35`](https://github.com/hydro-project/hydroflow/commit/6dbbf35b6e5ae7f0225ac05c85598d4962ec66d8))
    - Simplify graph printing code, add delta/cumul green edges, allow hiding of vars/subgraphs ([`9ed0ce0`](https://github.com/hydro-project/hydroflow/commit/9ed0ce02128a0eeaf0b603efcbe896427e47ef62))
 * **[#933](https://github.com/hydro-project/hydroflow/issues/933)**
    - Fix handling of whitespace in `cast` expressions ([`f67b9f0`](https://github.com/hydro-project/hydroflow/commit/f67b9f0f9414977c24eace1e95ce840094be67a4))
 * **[#935](https://github.com/hydro-project/hydroflow/issues/935)**
    - Restore in-subgraph rendering of self-handoffs ([`2edf779`](https://github.com/hydro-project/hydroflow/commit/2edf77961ca0218265b35f179c2d86c810795266))
 * **Uncategorized**
    - Release hydroflow_lang v0.5.0, hydroflow_datalog_core v0.5.0, hydroflow_datalog v0.5.0, hydroflow_macro v0.5.0, lattices v0.5.0, hydroflow v0.5.0, hydro_cli v0.5.0, safety bump 4 crates ([`2e2d8b3`](https://github.com/hydro-project/hydroflow/commit/2e2d8b386fb086c8276a2853d2a1f96ad4d7c221))
    - Bump proc-macro2 min version to 1.0.63 ([`f19eccc`](https://github.com/hydro-project/hydroflow/commit/f19eccc79d6d7c88de7ba1ef6a0abf1caaef377f))
    - Clippy warning on multiline string in hydro_cli, py_udf ([`a927dc6`](https://github.com/hydro-project/hydroflow/commit/a927dc6afbe3178815b7c7c58ed2838d42d80334))
    - Update documentation and improve error messages for `demux_enum` operator ([`9646ca0`](https://github.com/hydro-project/hydroflow/commit/9646ca06e61af8c827e2d2fb9826ce62b70b6799))
    - `demux_enum` requires enum type name, add better error handling ([`1126266`](https://github.com/hydro-project/hydroflow/commit/1126266e69c2c4364bc8de558f11859e5bad1c69))
    - Add type guard to `demux_enum` codegen ([`02fddd2`](https://github.com/hydro-project/hydroflow/commit/02fddd2c0d99956d89f36395b283b198046b8766))
    - Initial technically working version of `demux_enum` with very bad error messages ([`b3d1148`](https://github.com/hydro-project/hydroflow/commit/b3d114827256f2b82a3c357f3419c6853a97f5c0))
    - Implement `partition` operator ([`fc25433`](https://github.com/hydro-project/hydroflow/commit/fc2543359ba11c0947fdc26f5360b2ac43a5a0c4))
    - Fix demux error messages and add tests ([`5a7e1b1`](https://github.com/hydro-project/hydroflow/commit/5a7e1b157362b0d655a28d6f3e5cd139ab8799f3))
    - Clippy redundant `to_string()` in `print!` lints ([`51a200a`](https://github.com/hydro-project/hydroflow/commit/51a200a444a42f21e6557f3b20d822aea8ccc670))
    - Implement `flow_prop_fn` for `union()` ([`1bdbf73`](https://github.com/hydro-project/hydroflow/commit/1bdbf73b630e4f2eff009b00b0e66d71be53bb4a))
    - Fix `clippy::implied_bounds_in_impls` from latest nightlies ([`e788989`](https://github.com/hydro-project/hydroflow/commit/e788989737fbd501173bc99c6f9f5f5ba514ec9c))
</details>

## 0.4.0 (2023-08-15)

<csr-id-d6db9cd22a3d63bcc65dafd5bc0ca663ecc553d7/>
<csr-id-949db02e9fa9878e1a7176c180d6f44c5cddf052/>
<csr-id-f60053f70da3071c54de4a0eabb059a143aa2ccc/>

### Chore

 - <csr-id-d6db9cd22a3d63bcc65dafd5bc0ca663ecc553d7/> Allow `clippy::redundant_locals`, for latest nightlies
 - <csr-id-949db02e9fa9878e1a7176c180d6f44c5cddf052/> fix lints for latest nightly
 - <csr-id-f60053f70da3071c54de4a0eabb059a143aa2ccc/> fix lint, format errors for latest nightly version (without updated pinned)
   For nightly version (d9c13cd45 2023-07-05)

### New Features

<csr-id-8f306e2a36582e168417808099eedf8a9de3b419/>
<csr-id-871002267e3c03da83729ecc2d028f3c7b5c18d2/>

 - <csr-id-b4b9644a19e8e7e7725c9c5b88e3a6b8c2be7364/> Add `use` statements to hydroflow syntax
   And use in doc tests.
 - <csr-id-fe02f23649312bb64c5d0c8870edf578e516f397/> add `iter_batches_stream` util to break up iterator into per-tick batches
   * Also tightens up a bit of `assert_eq`'s code

### Bug Fixes

<csr-id-cc959c762c3a0e036e672801c615028cbfb95168/>
<csr-id-ebba38230df134b04dd38c1df7c6de8712e3122e/>
<csr-id-a55fc74dc1ebbe26b49359a104beb48d7f6cd449/>
<csr-id-6c98bbc2bd3443fe6f77e0b8689b461edde1b316/>
<csr-id-2d53110336b2da5a16887c3d72101da72b2362bb/>

 - <csr-id-d378e5eada3d2bae90f98c5a33b2d055940a8c7f/> unify antijoin and difference with set and multiset semantics
   * fix: unify antijoin and difference with set and multiset semantics
* fix: replay semantics for antijoin and difference now work
   also added cross_join_multiset
* fix: enforce sort for tests of anti_join and difference using assert_eq
* fix: advance __borrow_ident beyond the current tick to prevent replay loops
* fix: add modified snapshots
* fix: temp
* fix: spelling typo in comment
* fix: make anti_join replay more efficient
* fix: ignore test that depends on order of antijoin
* fix: really ignore test_index
* fix: fix specific test ordering in wasm

### New Features (BREAKING)

 - <csr-id-7a3b4c04779ea38bfa06c246882fa8dfb52bc8f1/> add fused joins, make lattice_join replay correctly
   * feat!: add fused joins, make lattice_join replay correctly
* address comments
* fix clippy

### `hydroflow_lang` Commit Statistics

<csr-read-only-do-not-edit/>

 - 17 commits contributed to the release.
 - 42 days passed between releases.
 - 14 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 16 unique issues were worked on: [#820](https://github.com/hydro-project/hydroflow/issues/820), [#821](https://github.com/hydro-project/hydroflow/issues/821), [#822](https://github.com/hydro-project/hydroflow/issues/822), [#823](https://github.com/hydro-project/hydroflow/issues/823), [#833](https://github.com/hydro-project/hydroflow/issues/833), [#835](https://github.com/hydro-project/hydroflow/issues/835), [#840](https://github.com/hydro-project/hydroflow/issues/840), [#843](https://github.com/hydro-project/hydroflow/issues/843), [#844](https://github.com/hydro-project/hydroflow/issues/844), [#845](https://github.com/hydro-project/hydroflow/issues/845), [#851](https://github.com/hydro-project/hydroflow/issues/851), [#853](https://github.com/hydro-project/hydroflow/issues/853), [#861](https://github.com/hydro-project/hydroflow/issues/861), [#870](https://github.com/hydro-project/hydroflow/issues/870), [#872](https://github.com/hydro-project/hydroflow/issues/872), [#873](https://github.com/hydro-project/hydroflow/issues/873)

### `hydroflow_lang` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#820](https://github.com/hydro-project/hydroflow/issues/820)**
    - Make batch take two inputs [input] and [signal] ([`8710022`](https://github.com/hydro-project/hydroflow/commit/871002267e3c03da83729ecc2d028f3c7b5c18d2))
 * **[#821](https://github.com/hydro-project/hydroflow/issues/821)**
    - `py_udf` operator feature gating ([`2d53110`](https://github.com/hydro-project/hydroflow/commit/2d53110336b2da5a16887c3d72101da72b2362bb))
 * **[#822](https://github.com/hydro-project/hydroflow/issues/822)**
    - Fix lint, format errors for latest nightly version (without updated pinned) ([`f60053f`](https://github.com/hydro-project/hydroflow/commit/f60053f70da3071c54de4a0eabb059a143aa2ccc))
 * **[#823](https://github.com/hydro-project/hydroflow/issues/823)**
    - Book/doc edits ([`4bdd556`](https://github.com/hydro-project/hydroflow/commit/4bdd5568fa0a6674f650f91a029fab302cbf14f4))
 * **[#833](https://github.com/hydro-project/hydroflow/issues/833)**
    - Rename next_tick -> defer, batch -> defer_signal ([`6c98bbc`](https://github.com/hydro-project/hydroflow/commit/6c98bbc2bd3443fe6f77e0b8689b461edde1b316))
 * **[#835](https://github.com/hydro-project/hydroflow/issues/835)**
    - Rename assert => assert_eq, add assert, change underlying implementation to work across ticks ([`8f306e2`](https://github.com/hydro-project/hydroflow/commit/8f306e2a36582e168417808099eedf8a9de3b419))
 * **[#840](https://github.com/hydro-project/hydroflow/issues/840)**
    - Make all operators 'tick by default ([`a55fc74`](https://github.com/hydro-project/hydroflow/commit/a55fc74dc1ebbe26b49359a104beb48d7f6cd449))
 * **[#843](https://github.com/hydro-project/hydroflow/issues/843)**
    - Add `iter_batches_stream` util to break up iterator into per-tick batches ([`fe02f23`](https://github.com/hydro-project/hydroflow/commit/fe02f23649312bb64c5d0c8870edf578e516f397))
 * **[#844](https://github.com/hydro-project/hydroflow/issues/844)**
    - Fix lints for latest nightly ([`949db02`](https://github.com/hydro-project/hydroflow/commit/949db02e9fa9878e1a7176c180d6f44c5cddf052))
 * **[#845](https://github.com/hydro-project/hydroflow/issues/845)**
    - Add `use` statements to hydroflow syntax ([`b4b9644`](https://github.com/hydro-project/hydroflow/commit/b4b9644a19e8e7e7725c9c5b88e3a6b8c2be7364))
 * **[#851](https://github.com/hydro-project/hydroflow/issues/851)**
    - Lattice_batch now takes [input] and [signal] ([`ebba382`](https://github.com/hydro-project/hydroflow/commit/ebba38230df134b04dd38c1df7c6de8712e3122e))
 * **[#853](https://github.com/hydro-project/hydroflow/issues/853)**
    - Book updates ([`2e57445`](https://github.com/hydro-project/hydroflow/commit/2e574457246ac5bd231745a8ad068558859698ef))
 * **[#861](https://github.com/hydro-project/hydroflow/issues/861)**
    - Add fused joins, make lattice_join replay correctly ([`7a3b4c0`](https://github.com/hydro-project/hydroflow/commit/7a3b4c04779ea38bfa06c246882fa8dfb52bc8f1))
 * **[#870](https://github.com/hydro-project/hydroflow/issues/870)**
    - Joins now replay correctly ([`cc959c7`](https://github.com/hydro-project/hydroflow/commit/cc959c762c3a0e036e672801c615028cbfb95168))
 * **[#872](https://github.com/hydro-project/hydroflow/issues/872)**
    - Unify antijoin and difference with set and multiset semantics ([`d378e5e`](https://github.com/hydro-project/hydroflow/commit/d378e5eada3d2bae90f98c5a33b2d055940a8c7f))
 * **[#873](https://github.com/hydro-project/hydroflow/issues/873)**
    - Allow `clippy::redundant_locals`, for latest nightlies ([`d6db9cd`](https://github.com/hydro-project/hydroflow/commit/d6db9cd22a3d63bcc65dafd5bc0ca663ecc553d7))
 * **Uncategorized**
    - Release hydroflow_lang v0.4.0, hydroflow_datalog_core v0.4.0, hydroflow_datalog v0.4.0, hydroflow_macro v0.4.0, lattices v0.4.0, pusherator v0.0.3, hydroflow v0.4.0, hydro_cli v0.4.0, safety bump 4 crates ([`cb313f0`](https://github.com/hydro-project/hydroflow/commit/cb313f0635214460a8308d05cbef4bf7f4bfaa15))
</details>

## 0.3.0 (2023-07-04)

<csr-id-70c88a51c4c83a4dc2fc67a0cd344786a4ff26f7/>
<csr-id-4a727ecf1232e0f03f5300547282bfbe73342cfa/>

### Documentation

 - <csr-id-fa5b180d96498d144f3617bba7722e8f4ac9dd0e/> remove pattern deref from inspect, filter examples
   `*` derefs are easier for Rust beginners to comprehend.
 - <csr-id-f55d540532ba0a0970cab2bb5aef81b6a76b317a/> change mermaid colors
   Use a lighter shade of blue and yellow, and dark text.

### New Features

<csr-id-6323980e83bee27a8233a69a35734b5970336701/>
<csr-id-010524615bb78288e339e03880c4dd3b432b6d7f/>
<csr-id-d83b049e4d643617a2b15b3dbf1698aa79846aeb/>
<csr-id-ea65349d241873f8460d7a8b024d64c63180246f/>
<csr-id-22abcaff806c7de6e4a7725656bbcf201e7d9259/>

 - <csr-id-b435bbb1d64d60f1248fdcd636635b15954e7325/> fold and reduce take accumulated value by mutable reference
   * feat: fold and reduce take accumulated value by mutable reference
* address comments
* feat: add lattice_reduce and lattice_fold
* address comments
* simplify lattice fold a bit
* address comments
* feat: add join_multiset()
* address comments
* fix assert
* feat: add assert() operator
* update: change for_each -> assert, make doctest use run_avaialble()
* don't run tests that panic in wasm
* update comments
* address comments

### Bug Fixes

 - <csr-id-8d3494b5afee858114a602a3e23077bb6d24dd77/> update proc-macro2, use new span location API where possible
   requires latest* rust nightly version
   
   *latest = 2023-06-28 or something
 - <csr-id-20cb3811fc0da3ce1b36003c8823b4b242d64196/> fix nightly removing array_zip feature, bump pinned nightly to 06-01

### Style

 - <csr-id-70c88a51c4c83a4dc2fc67a0cd344786a4ff26f7/> `warn` missing docs (instead of `deny`) to allow code before docs

### New Features (BREAKING)

 - <csr-id-931d93887c238025596cb22226e16d43e16a7425/> Add `reveal` methods, make fields private

### Bug Fixes (BREAKING)

 - <csr-id-6f3c536fcd4d1305d478ec3db62416aad9cf3c68/> make join default to multiset join

### Refactor (BREAKING)

 - <csr-id-4a727ecf1232e0f03f5300547282bfbe73342cfa/> Rename `ConvertFrom::from` -> `LatticeFrom::lattice_from`

### `hydroflow_lang` Commit Statistics

<csr-read-only-do-not-edit/>

 - 17 commits contributed to the release.
 - 33 days passed between releases.
 - 14 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 15 unique issues were worked on: [#741](https://github.com/hydro-project/hydroflow/issues/741), [#765](https://github.com/hydro-project/hydroflow/issues/765), [#773](https://github.com/hydro-project/hydroflow/issues/773), [#774](https://github.com/hydro-project/hydroflow/issues/774), [#775](https://github.com/hydro-project/hydroflow/issues/775), [#778](https://github.com/hydro-project/hydroflow/issues/778), [#780](https://github.com/hydro-project/hydroflow/issues/780), [#784](https://github.com/hydro-project/hydroflow/issues/784), [#789](https://github.com/hydro-project/hydroflow/issues/789), [#792](https://github.com/hydro-project/hydroflow/issues/792), [#799](https://github.com/hydro-project/hydroflow/issues/799), [#801](https://github.com/hydro-project/hydroflow/issues/801), [#803](https://github.com/hydro-project/hydroflow/issues/803), [#804](https://github.com/hydro-project/hydroflow/issues/804), [#809](https://github.com/hydro-project/hydroflow/issues/809)

### `hydroflow_lang` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#741](https://github.com/hydro-project/hydroflow/issues/741)**
    - Fix nightly removing array_zip feature, bump pinned nightly to 06-01 ([`20cb381`](https://github.com/hydro-project/hydroflow/commit/20cb3811fc0da3ce1b36003c8823b4b242d64196))
 * **[#765](https://github.com/hydro-project/hydroflow/issues/765)**
    - Rename `ConvertFrom::from` -> `LatticeFrom::lattice_from` ([`4a727ec`](https://github.com/hydro-project/hydroflow/commit/4a727ecf1232e0f03f5300547282bfbe73342cfa))
 * **[#773](https://github.com/hydro-project/hydroflow/issues/773)**
    - `warn` missing docs (instead of `deny`) to allow code before docs ([`70c88a5`](https://github.com/hydro-project/hydroflow/commit/70c88a51c4c83a4dc2fc67a0cd344786a4ff26f7))
 * **[#774](https://github.com/hydro-project/hydroflow/issues/774)**
    - Make join default to multiset join ([`6f3c536`](https://github.com/hydro-project/hydroflow/commit/6f3c536fcd4d1305d478ec3db62416aad9cf3c68))
 * **[#775](https://github.com/hydro-project/hydroflow/issues/775)**
    - Add persist_mut and persist_mut_keyed for non-monitone deletions ([`8d8247f`](https://github.com/hydro-project/hydroflow/commit/8d8247f0b37d53415f5738099c0c8a021415b158))
 * **[#778](https://github.com/hydro-project/hydroflow/issues/778)**
    - Change mermaid colors ([`f55d540`](https://github.com/hydro-project/hydroflow/commit/f55d540532ba0a0970cab2bb5aef81b6a76b317a))
 * **[#780](https://github.com/hydro-project/hydroflow/issues/780)**
    - Emit `compile_error!` diagnostics for stable ([`ea65349`](https://github.com/hydro-project/hydroflow/commit/ea65349d241873f8460d7a8b024d64c63180246f))
    - Allow stable build, refactors behind `nightly` feature flag ([`22abcaf`](https://github.com/hydro-project/hydroflow/commit/22abcaff806c7de6e4a7725656bbcf201e7d9259))
 * **[#784](https://github.com/hydro-project/hydroflow/issues/784)**
    - Add assert() operator ([`d83b049`](https://github.com/hydro-project/hydroflow/commit/d83b049e4d643617a2b15b3dbf1698aa79846aeb))
 * **[#789](https://github.com/hydro-project/hydroflow/issues/789)**
    - Add `reveal` methods, make fields private ([`931d938`](https://github.com/hydro-project/hydroflow/commit/931d93887c238025596cb22226e16d43e16a7425))
 * **[#792](https://github.com/hydro-project/hydroflow/issues/792)**
    - Add `py_udf` operator [wip] ([`7dbd5e2`](https://github.com/hydro-project/hydroflow/commit/7dbd5e24d6e71cf8fab7c3ce09d5937c0f301456))
 * **[#799](https://github.com/hydro-project/hydroflow/issues/799)**
    - Remove pattern deref from inspect, filter examples ([`fa5b180`](https://github.com/hydro-project/hydroflow/commit/fa5b180d96498d144f3617bba7722e8f4ac9dd0e))
 * **[#801](https://github.com/hydro-project/hydroflow/issues/801)**
    - Update proc-macro2, use new span location API where possible ([`8d3494b`](https://github.com/hydro-project/hydroflow/commit/8d3494b5afee858114a602a3e23077bb6d24dd77))
 * **[#803](https://github.com/hydro-project/hydroflow/issues/803)**
    - Add lattice_reduce and lattice_fold ([`6323980`](https://github.com/hydro-project/hydroflow/commit/6323980e83bee27a8233a69a35734b5970336701))
 * **[#804](https://github.com/hydro-project/hydroflow/issues/804)**
    - Add join_multiset() ([`0105246`](https://github.com/hydro-project/hydroflow/commit/010524615bb78288e339e03880c4dd3b432b6d7f))
 * **[#809](https://github.com/hydro-project/hydroflow/issues/809)**
    - Fold and reduce take accumulated value by mutable reference ([`b435bbb`](https://github.com/hydro-project/hydroflow/commit/b435bbb1d64d60f1248fdcd636635b15954e7325))
 * **Uncategorized**
    - Release hydroflow_cli_integration v0.3.0, hydroflow_lang v0.3.0, hydroflow_datalog_core v0.3.0, hydroflow_datalog v0.3.0, hydroflow_macro v0.3.0, lattices v0.3.0, pusherator v0.0.2, hydroflow v0.3.0, hydro_cli v0.3.0, safety bump 5 crates ([`ec9633e`](https://github.com/hydro-project/hydroflow/commit/ec9633e2e393c2bf106223abeb0b680200fbdf84))
</details>

## 0.2.0 (2023-05-31)

<csr-id-fd896fbe925fbd8ef1d16be7206ac20ba585081a/>
<csr-id-c9e8603c6ede61d5098869d3d0b5e24c7254f7a4/>

### Chore

 - <csr-id-fd896fbe925fbd8ef1d16be7206ac20ba585081a/> manually bump versions for v0.2.0 release

### Documentation

 - <csr-id-989adcbcd304ad0890d71351d56a22977bdcf73f/> categorize operators, organize op docs, fix #727

### Bug Fixes

 - <csr-id-554d563fe53a1303c5a5c9352197365235c607e3/> make `build.rs`s infallible, log to stderr, to fix release

### Refactor

 - <csr-id-c9e8603c6ede61d5098869d3d0b5e24c7254f7a4/> remove `hydroflow_internalmacro`, use `hydroflow_lang/build.rs` instead

### `hydroflow_lang` Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release.
 - 1 day passed between releases.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#728](https://github.com/hydro-project/hydroflow/issues/728), [#730](https://github.com/hydro-project/hydroflow/issues/730)

### `hydroflow_lang` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#728](https://github.com/hydro-project/hydroflow/issues/728)**
    - Remove `hydroflow_internalmacro`, use `hydroflow_lang/build.rs` instead ([`c9e8603`](https://github.com/hydro-project/hydroflow/commit/c9e8603c6ede61d5098869d3d0b5e24c7254f7a4))
 * **[#730](https://github.com/hydro-project/hydroflow/issues/730)**
    - Categorize operators, organize op docs, fix #727 ([`989adcb`](https://github.com/hydro-project/hydroflow/commit/989adcbcd304ad0890d71351d56a22977bdcf73f))
 * **Uncategorized**
    - Release hydroflow_lang v0.2.0, hydroflow_datalog_core v0.2.0, hydroflow_datalog v0.2.0, hydroflow_macro v0.2.0, lattices v0.2.0, hydroflow v0.2.0, hydro_cli v0.2.0 ([`ca464c3`](https://github.com/hydro-project/hydroflow/commit/ca464c32322a7ad39eb53e1794777c849aa548a0))
    - Make `build.rs`s infallible, log to stderr, to fix release ([`554d563`](https://github.com/hydro-project/hydroflow/commit/554d563fe53a1303c5a5c9352197365235c607e3))
    - Manually bump versions for v0.2.0 release ([`fd896fb`](https://github.com/hydro-project/hydroflow/commit/fd896fbe925fbd8ef1d16be7206ac20ba585081a))
</details>

## 0.1.1 (2023-05-30)

<csr-id-d574cb2661ba086059ba8cd6904fd6b6b0a5a8cb/>
<csr-id-d13a01b3a3fa0c52381833f88bcadac7a4ebcda9/>
<csr-id-ea21462cac6d14ad744d8f0c39d5bcddc33d82ce/>
<csr-id-3608de2e8d0c8bbd67b6ecb9aa4261e5cfc955da/>
<csr-id-5d99ef7801517fa2ec6efe038ae07ab21233167f/>
<csr-id-9ecda698486d8472a2f3688ba24c76c1bc3328e1/>
<csr-id-2843e7e114ac824a684a5400909819ccc5c88fe3/>

### New Features

 - <csr-id-977b9c4e8accd2ae4ae8e8798d7b72a637874b77/> add `zip_longest` operator, fix #707
   With a test.
 - <csr-id-78bc06eb09090acd46495b8e0147e3434378c9f6/> add per-tick truncating `zip` operator, fix #707
   With tests.
 - <csr-id-8d88e8e01a985db8ebd8dbc6768163452cedc3ab/> Add `multiset_delta` operator

### Bug Fixes

 - <csr-id-c771879f2fb81658f59d286ee0899065b2f2ab90/> multiset_delta not correctly tracking counts beyond two ticks
   We were swapping the `RefCell`s, but we need to swap what's _behind_ them.
 - <csr-id-075c99e7cdcf40ae5cab9efa787ba4447db8a479/> fix `persist` releasing multiple times during the same tick
   Add surface_double_handoff tests

### Other

 - <csr-id-d574cb2661ba086059ba8cd6904fd6b6b0a5a8cb/> merge() to union()

### Refactor

 - <csr-id-d13a01b3a3fa0c52381833f88bcadac7a4ebcda9/> add spin(), remove repeat_iter,repeat_iter_external
   * refactor: add spin(), remove repeat_iter,repeat_iter_external
   
   * fix: fix lints
 - <csr-id-ea21462cac6d14ad744d8f0c39d5bcddc33d82ce/> change `lattice_merge` to use `reduce` instead of `fold`, fix #710
   `Default` no longer needed
 - <csr-id-3608de2e8d0c8bbd67b6ecb9aa4261e5cfc955da/> rename `sort_by` -> `sort_by_key`, fix #705
 - <csr-id-5d99ef7801517fa2ec6efe038ae07ab21233167f/> rename `keyed_reduce` -> `reduce_keyed`, fix #705
 - <csr-id-9ecda698486d8472a2f3688ba24c76c1bc3328e1/> remove `'static` from `sort()`, fix #703
 - <csr-id-2843e7e114ac824a684a5400909819ccc5c88fe3/> Suffixes and remove keyed fold
   * rename: keyed_fold/keyed_reduce -> fold_keyed/reduce_keyed
   
   * remove group_by

### `hydroflow_lang` Commit Statistics

<csr-read-only-do-not-edit/>

 - 13 commits contributed to the release.
 - 6 days passed between releases.
 - 12 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 8 unique issues were worked on: [#696](https://github.com/hydro-project/hydroflow/issues/696), [#697](https://github.com/hydro-project/hydroflow/issues/697), [#702](https://github.com/hydro-project/hydroflow/issues/702), [#704](https://github.com/hydro-project/hydroflow/issues/704), [#706](https://github.com/hydro-project/hydroflow/issues/706), [#714](https://github.com/hydro-project/hydroflow/issues/714), [#716](https://github.com/hydro-project/hydroflow/issues/716), [#719](https://github.com/hydro-project/hydroflow/issues/719)

### `hydroflow_lang` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#696](https://github.com/hydro-project/hydroflow/issues/696)**
    - Add `multiset_delta` operator ([`8d88e8e`](https://github.com/hydro-project/hydroflow/commit/8d88e8e01a985db8ebd8dbc6768163452cedc3ab))
 * **[#697](https://github.com/hydro-project/hydroflow/issues/697)**
    - Merge() to union() ([`d574cb2`](https://github.com/hydro-project/hydroflow/commit/d574cb2661ba086059ba8cd6904fd6b6b0a5a8cb))
 * **[#702](https://github.com/hydro-project/hydroflow/issues/702)**
    - Suffixes and remove keyed fold ([`2843e7e`](https://github.com/hydro-project/hydroflow/commit/2843e7e114ac824a684a5400909819ccc5c88fe3))
 * **[#704](https://github.com/hydro-project/hydroflow/issues/704)**
    - Remove `'static` from `sort()`, fix #703 ([`9ecda69`](https://github.com/hydro-project/hydroflow/commit/9ecda698486d8472a2f3688ba24c76c1bc3328e1))
 * **[#706](https://github.com/hydro-project/hydroflow/issues/706)**
    - Rename `sort_by` -> `sort_by_key`, fix #705 ([`3608de2`](https://github.com/hydro-project/hydroflow/commit/3608de2e8d0c8bbd67b6ecb9aa4261e5cfc955da))
    - Rename `keyed_reduce` -> `reduce_keyed`, fix #705 ([`5d99ef7`](https://github.com/hydro-project/hydroflow/commit/5d99ef7801517fa2ec6efe038ae07ab21233167f))
 * **[#714](https://github.com/hydro-project/hydroflow/issues/714)**
    - Add spin(), remove repeat_iter,repeat_iter_external ([`d13a01b`](https://github.com/hydro-project/hydroflow/commit/d13a01b3a3fa0c52381833f88bcadac7a4ebcda9))
 * **[#716](https://github.com/hydro-project/hydroflow/issues/716)**
    - Fix `persist` releasing multiple times during the same tick ([`075c99e`](https://github.com/hydro-project/hydroflow/commit/075c99e7cdcf40ae5cab9efa787ba4447db8a479))
 * **[#719](https://github.com/hydro-project/hydroflow/issues/719)**
    - Multiset_delta not correctly tracking counts beyond two ticks ([`c771879`](https://github.com/hydro-project/hydroflow/commit/c771879f2fb81658f59d286ee0899065b2f2ab90))
 * **Uncategorized**
    - Release hydroflow_cli_integration v0.1.1, hydroflow_lang v0.1.1, hydroflow_datalog_core v0.1.1, hydroflow_macro v0.1.1, lattices v0.1.2, hydroflow v0.1.1, hydro_cli v0.1.0 ([`d9fa8b3`](https://github.com/hydro-project/hydroflow/commit/d9fa8b387e303b33d9614dbde80abf1af08bd8eb))
    - Change `lattice_merge` to use `reduce` instead of `fold`, fix #710 ([`ea21462`](https://github.com/hydro-project/hydroflow/commit/ea21462cac6d14ad744d8f0c39d5bcddc33d82ce))
    - Add `zip_longest` operator, fix #707 ([`977b9c4`](https://github.com/hydro-project/hydroflow/commit/977b9c4e8accd2ae4ae8e8798d7b72a637874b77))
    - Add per-tick truncating `zip` operator, fix #707 ([`78bc06e`](https://github.com/hydro-project/hydroflow/commit/78bc06eb09090acd46495b8e0147e3434378c9f6))
</details>

## 0.1.0 (2023-05-23)

<csr-id-52ee8f8e443f0a8b5caf92d2c5f028c00302a79b/>
<csr-id-faab58f855e4d6f2ad885c6f39f57ebc5662ec20/>

### Chore

 - <csr-id-52ee8f8e443f0a8b5caf92d2c5f028c00302a79b/> bump versions to 0.1.0 for release
   For release on crates.io for v0.1

### Refactor

 - <csr-id-faab58f855e4d6f2ad885c6f39f57ebc5662ec20/> remove `hydroflow::lang` module, move `Clear`, `MonotonicMap` to `hydroflow::util` instead

### `hydroflow_lang` Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release.
 - 2 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 5 unique issues were worked on: [#661](https://github.com/hydro-project/hydroflow/issues/661), [#673](https://github.com/hydro-project/hydroflow/issues/673), [#676](https://github.com/hydro-project/hydroflow/issues/676), [#677](https://github.com/hydro-project/hydroflow/issues/677), [#684](https://github.com/hydro-project/hydroflow/issues/684)

### `hydroflow_lang` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#661](https://github.com/hydro-project/hydroflow/issues/661)**
    - Add hydroflow_{test, main} so that hydroflow is actually singlethreaded ([`f61054e`](https://github.com/hydro-project/hydroflow/commit/f61054eaeca6fab1ab0cb588b7ed546b87772e91))
 * **[#673](https://github.com/hydro-project/hydroflow/issues/673)**
    - Don't box source_stream argument unnecessarily ([`dc37cba`](https://github.com/hydro-project/hydroflow/commit/dc37cba9512b47bbc98bbc84e3594817eca9bace))
 * **[#676](https://github.com/hydro-project/hydroflow/issues/676)**
    - Remove last instances of tokio::main ([`367073b`](https://github.com/hydro-project/hydroflow/commit/367073bf01b54057a4f6c2c9f9e89079f11542de))
 * **[#677](https://github.com/hydro-project/hydroflow/issues/677)**
    - Remove `hydroflow::lang` module, move `Clear`, `MonotonicMap` to `hydroflow::util` instead ([`faab58f`](https://github.com/hydro-project/hydroflow/commit/faab58f855e4d6f2ad885c6f39f57ebc5662ec20))
 * **[#684](https://github.com/hydro-project/hydroflow/issues/684)**
    - Bump versions to 0.1.0 for release ([`52ee8f8`](https://github.com/hydro-project/hydroflow/commit/52ee8f8e443f0a8b5caf92d2c5f028c00302a79b))
 * **Uncategorized**
    - Release hydroflow_cli_integration v0.1.0, hydroflow_internalmacro v0.1.0, hydroflow_lang v0.1.0, hydroflow_datalog_core v0.1.0, hydroflow_datalog v0.1.0, hydroflow_macro v0.1.0, lattices v0.1.1, hydroflow v0.1.0 ([`7324974`](https://github.com/hydro-project/hydroflow/commit/73249744293c9b89cbaa2d84b23ca3f25b00ae4e))
</details>

## 0.0.1 (2023-05-21)

<csr-id-4d4446c0988ee7c2a991d2845b66a281934d6100/>
<csr-id-cd0a86d9271d0e3daab59c46f079925f863424e1/>
<csr-id-20a1b2c0cd04a8b495a02ce345db3d48a99ea0e9/>
<csr-id-1eda91a2ef8794711ef037240f15284e8085d863/>

### Documentation

 - <csr-id-95d23eaf8218002ad0a6a8c4c6e6c76e6b8f785b/> Update docs, add book chapter for `lattices` crate
   - Adds `mdbook-katex` to the book build for latex support.

### Style

 - <csr-id-4d4446c0988ee7c2a991d2845b66a281934d6100/> rustfmt normalize comments
 - <csr-id-cd0a86d9271d0e3daab59c46f079925f863424e1/> Warn lint `unused_qualifications`
 - <csr-id-20a1b2c0cd04a8b495a02ce345db3d48a99ea0e9/> rustfmt group imports
 - <csr-id-1eda91a2ef8794711ef037240f15284e8085d863/> rustfmt prescribe flat-module `use` format

### `hydroflow_lang` Commit Statistics

<csr-read-only-do-not-edit/>

 - 11 commits contributed to the release.
 - 25 days passed between releases.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 7 unique issues were worked on: [#638](https://github.com/hydro-project/hydroflow/issues/638), [#639](https://github.com/hydro-project/hydroflow/issues/639), [#642](https://github.com/hydro-project/hydroflow/issues/642), [#649](https://github.com/hydro-project/hydroflow/issues/649), [#654](https://github.com/hydro-project/hydroflow/issues/654), [#660](https://github.com/hydro-project/hydroflow/issues/660), [#667](https://github.com/hydro-project/hydroflow/issues/667)

### `hydroflow_lang` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#638](https://github.com/hydro-project/hydroflow/issues/638)**
    - Remove old lattice code ([`f4915fa`](https://github.com/hydro-project/hydroflow/commit/f4915fab98c57652e5345d39076d95ebb0a43fd8))
 * **[#639](https://github.com/hydro-project/hydroflow/issues/639)**
    - Update pinned nightly to `nightly-2023-05-03` ([`f0afb56`](https://github.com/hydro-project/hydroflow/commit/f0afb56a069f6aa40c4f9eee131408b32a17d83c))
 * **[#642](https://github.com/hydro-project/hydroflow/issues/642)**
    - Remove zmq, use unsync channels locally, use sync mpsc cross-thread, use cross_join+enumerate instead of broadcast channel,remove Eq requirement from multisetjoin ([`b38f5cf`](https://github.com/hydro-project/hydroflow/commit/b38f5cf198e29a8de2f84eb4cd075818fbeffda6))
 * **[#649](https://github.com/hydro-project/hydroflow/issues/649)**
    - Add lattice_batch ([`af26532`](https://github.com/hydro-project/hydroflow/commit/af265328179f1cb1f77663cbd3e414a618583bf1))
 * **[#654](https://github.com/hydro-project/hydroflow/issues/654)**
    - Deduplicate `dest_sink_serde` code by using `dest_sink`'s `write_fn` ([`3b8d2f5`](https://github.com/hydro-project/hydroflow/commit/3b8d2f5e1e3a16c825171adf610d4dd6fa47c6e3))
 * **[#660](https://github.com/hydro-project/hydroflow/issues/660)**
    - Rustfmt normalize comments ([`4d4446c`](https://github.com/hydro-project/hydroflow/commit/4d4446c0988ee7c2a991d2845b66a281934d6100))
    - Warn lint `unused_qualifications` ([`cd0a86d`](https://github.com/hydro-project/hydroflow/commit/cd0a86d9271d0e3daab59c46f079925f863424e1))
    - Rustfmt group imports ([`20a1b2c`](https://github.com/hydro-project/hydroflow/commit/20a1b2c0cd04a8b495a02ce345db3d48a99ea0e9))
    - Rustfmt prescribe flat-module `use` format ([`1eda91a`](https://github.com/hydro-project/hydroflow/commit/1eda91a2ef8794711ef037240f15284e8085d863))
 * **[#667](https://github.com/hydro-project/hydroflow/issues/667)**
    - Update docs, add book chapter for `lattices` crate ([`95d23ea`](https://github.com/hydro-project/hydroflow/commit/95d23eaf8218002ad0a6a8c4c6e6c76e6b8f785b))
 * **Uncategorized**
    - Release hydroflow_cli_integration v0.0.1, hydroflow_lang v0.0.1, hydroflow_datalog_core v0.0.1, hydroflow_datalog v0.0.1, hydroflow_macro v0.0.1, lattices v0.1.0, variadics v0.0.2, pusherator v0.0.1, hydroflow v0.0.2 ([`809395a`](https://github.com/hydro-project/hydroflow/commit/809395acddb78949d7a2bf036e1a94972f23b1ad))
</details>

## 0.0.0 (2023-04-25)

<csr-id-62fcfb157eaaaabedfeb5c77b2a6df89ee1a6852/>
<csr-id-bc3d12f563dab96f4751ec21cd20b193eea95456/>
<csr-id-a2078f7056a54d20f91e2e0f9a7617dc6ef1f627/>

### Other

 - <csr-id-62fcfb157eaaaabedfeb5c77b2a6df89ee1a6852/> :<'static> now replays #143 #364
 - <csr-id-bc3d12f563dab96f4751ec21cd20b193eea95456/> :<'static> now replays #143 #364
 - <csr-id-a2078f7056a54d20f91e2e0f9a7617dc6ef1f627/> :<'static> now replays #143 #364

### `hydroflow_lang` Commit Statistics

<csr-read-only-do-not-edit/>

 - 216 commits contributed to the release.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 102 unique issues were worked on: [#155](https://github.com/hydro-project/hydroflow/issues/155), [#162](https://github.com/hydro-project/hydroflow/issues/162), [#211](https://github.com/hydro-project/hydroflow/issues/211), [#213](https://github.com/hydro-project/hydroflow/issues/213), [#230](https://github.com/hydro-project/hydroflow/issues/230), [#236](https://github.com/hydro-project/hydroflow/issues/236), [#239](https://github.com/hydro-project/hydroflow/issues/239), [#245](https://github.com/hydro-project/hydroflow/issues/245), [#250](https://github.com/hydro-project/hydroflow/issues/250), [#259](https://github.com/hydro-project/hydroflow/issues/259), [#261](https://github.com/hydro-project/hydroflow/issues/261), [#277](https://github.com/hydro-project/hydroflow/issues/277), [#278](https://github.com/hydro-project/hydroflow/issues/278), [#282](https://github.com/hydro-project/hydroflow/issues/282), [#284](https://github.com/hydro-project/hydroflow/issues/284), [#285](https://github.com/hydro-project/hydroflow/issues/285), [#295](https://github.com/hydro-project/hydroflow/issues/295), [#296](https://github.com/hydro-project/hydroflow/issues/296), [#298](https://github.com/hydro-project/hydroflow/issues/298), [#301](https://github.com/hydro-project/hydroflow/issues/301), [#309](https://github.com/hydro-project/hydroflow/issues/309), [#311](https://github.com/hydro-project/hydroflow/issues/311), [#320](https://github.com/hydro-project/hydroflow/issues/320), [#321](https://github.com/hydro-project/hydroflow/issues/321), [#323](https://github.com/hydro-project/hydroflow/issues/323), [#329](https://github.com/hydro-project/hydroflow/issues/329), [#331](https://github.com/hydro-project/hydroflow/issues/331), [#334](https://github.com/hydro-project/hydroflow/issues/334), [#337](https://github.com/hydro-project/hydroflow/issues/337), [#338](https://github.com/hydro-project/hydroflow/issues/338), [#350](https://github.com/hydro-project/hydroflow/issues/350), [#360](https://github.com/hydro-project/hydroflow/issues/360), [#363](https://github.com/hydro-project/hydroflow/issues/363), [#381](https://github.com/hydro-project/hydroflow/issues/381), [#382](https://github.com/hydro-project/hydroflow/issues/382), [#383](https://github.com/hydro-project/hydroflow/issues/383), [#399](https://github.com/hydro-project/hydroflow/issues/399), [#404](https://github.com/hydro-project/hydroflow/issues/404), [#405](https://github.com/hydro-project/hydroflow/issues/405), [#412](https://github.com/hydro-project/hydroflow/issues/412), [#419](https://github.com/hydro-project/hydroflow/issues/419), [#425](https://github.com/hydro-project/hydroflow/issues/425), [#431](https://github.com/hydro-project/hydroflow/issues/431), [#434](https://github.com/hydro-project/hydroflow/issues/434), [#441 1/14](https://github.com/hydro-project/hydroflow/issues/441 1/14), [#441 10/14](https://github.com/hydro-project/hydroflow/issues/441 10/14), [#441 11/14](https://github.com/hydro-project/hydroflow/issues/441 11/14), [#441 12/14](https://github.com/hydro-project/hydroflow/issues/441 12/14), [#441 13/14](https://github.com/hydro-project/hydroflow/issues/441 13/14), [#441 14/14](https://github.com/hydro-project/hydroflow/issues/441 14/14), [#441 2/14](https://github.com/hydro-project/hydroflow/issues/441 2/14), [#441 3/14](https://github.com/hydro-project/hydroflow/issues/441 3/14), [#441 4/14](https://github.com/hydro-project/hydroflow/issues/441 4/14), [#441 5/14](https://github.com/hydro-project/hydroflow/issues/441 5/14), [#441 6/14](https://github.com/hydro-project/hydroflow/issues/441 6/14), [#441 7/14](https://github.com/hydro-project/hydroflow/issues/441 7/14), [#441 8/14](https://github.com/hydro-project/hydroflow/issues/441 8/14), [#441 9/14](https://github.com/hydro-project/hydroflow/issues/441 9/14), [#444](https://github.com/hydro-project/hydroflow/issues/444), [#445](https://github.com/hydro-project/hydroflow/issues/445), [#448 1/2](https://github.com/hydro-project/hydroflow/issues/448 1/2), [#455](https://github.com/hydro-project/hydroflow/issues/455), [#459](https://github.com/hydro-project/hydroflow/issues/459), [#465](https://github.com/hydro-project/hydroflow/issues/465), [#468](https://github.com/hydro-project/hydroflow/issues/468), [#471](https://github.com/hydro-project/hydroflow/issues/471), [#475](https://github.com/hydro-project/hydroflow/issues/475), [#488](https://github.com/hydro-project/hydroflow/issues/488), [#490](https://github.com/hydro-project/hydroflow/issues/490), [#491](https://github.com/hydro-project/hydroflow/issues/491), [#493](https://github.com/hydro-project/hydroflow/issues/493), [#497](https://github.com/hydro-project/hydroflow/issues/497), [#499](https://github.com/hydro-project/hydroflow/issues/499), [#501](https://github.com/hydro-project/hydroflow/issues/501), [#508](https://github.com/hydro-project/hydroflow/issues/508), [#509](https://github.com/hydro-project/hydroflow/issues/509), [#511](https://github.com/hydro-project/hydroflow/issues/511), [#512](https://github.com/hydro-project/hydroflow/issues/512), [#518](https://github.com/hydro-project/hydroflow/issues/518), [#523](https://github.com/hydro-project/hydroflow/issues/523), [#524](https://github.com/hydro-project/hydroflow/issues/524), [#526](https://github.com/hydro-project/hydroflow/issues/526), [#530](https://github.com/hydro-project/hydroflow/issues/530), [#538](https://github.com/hydro-project/hydroflow/issues/538), [#547](https://github.com/hydro-project/hydroflow/issues/547), [#550](https://github.com/hydro-project/hydroflow/issues/550), [#555](https://github.com/hydro-project/hydroflow/issues/555), [#556](https://github.com/hydro-project/hydroflow/issues/556), [#559](https://github.com/hydro-project/hydroflow/issues/559), [#566](https://github.com/hydro-project/hydroflow/issues/566), [#573](https://github.com/hydro-project/hydroflow/issues/573), [#579](https://github.com/hydro-project/hydroflow/issues/579), [#591](https://github.com/hydro-project/hydroflow/issues/591), [#598](https://github.com/hydro-project/hydroflow/issues/598), [#599](https://github.com/hydro-project/hydroflow/issues/599), [#602](https://github.com/hydro-project/hydroflow/issues/602), [#604](https://github.com/hydro-project/hydroflow/issues/604), [#605](https://github.com/hydro-project/hydroflow/issues/605), [#607](https://github.com/hydro-project/hydroflow/issues/607), [#609](https://github.com/hydro-project/hydroflow/issues/609), [#610](https://github.com/hydro-project/hydroflow/issues/610), [#617](https://github.com/hydro-project/hydroflow/issues/617)

### `hydroflow_lang` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#155](https://github.com/hydro-project/hydroflow/issues/155)**
    - Add datalog frontend via a proc macro ([`fd3867f`](https://github.com/hydro-project/hydroflow/commit/fd3867fde4302aabd747ca81564dfba6016a6395))
 * **[#162](https://github.com/hydro-project/hydroflow/issues/162)**
    - SerdeGraph from parser to be callable at runtime ([`17dd150`](https://github.com/hydro-project/hydroflow/commit/17dd1500be1dab5f7abbd498d8f96b6ed00dba59))
 * **[#211](https://github.com/hydro-project/hydroflow/issues/211)**
    - Add cross join surface syntax operator, update tests, fix #200 ([`c526f9a`](https://github.com/hydro-project/hydroflow/commit/c526f9a70de0d9a5d15655ad99412f3b425b4cab))
 * **[#213](https://github.com/hydro-project/hydroflow/issues/213)**
    - Add flatten op to surface syntax ([`f802b95`](https://github.com/hydro-project/hydroflow/commit/f802b9536cf9d07846e2ace54b09786c919aea11))
 * **[#230](https://github.com/hydro-project/hydroflow/issues/230)**
    - Add testing of surface syntax errors (and warnings) ([`b8394d8`](https://github.com/hydro-project/hydroflow/commit/b8394d8da3479be55a19fe5743285d8480f78c61))
 * **[#236](https://github.com/hydro-project/hydroflow/issues/236)**
    - Add unique operator to remove duplicates ([`e3e8db2`](https://github.com/hydro-project/hydroflow/commit/e3e8db208606bd354426332ca128a894f0e9f76e))
 * **[#239](https://github.com/hydro-project/hydroflow/issues/239)**
    - First version of groupby with test and example ([`c85a19d`](https://github.com/hydro-project/hydroflow/commit/c85a19d081e2c53da21700163ad3e6178b59fc33))
 * **[#245](https://github.com/hydro-project/hydroflow/issues/245)**
    - Book docs for ops ([`26e4cfe`](https://github.com/hydro-project/hydroflow/commit/26e4cfe7354230907d9dc32737d3ceb877f9195c))
 * **[#250](https://github.com/hydro-project/hydroflow/issues/250)**
    - Limit `null()` to up to one input and/or output. ([`05a05bb`](https://github.com/hydro-project/hydroflow/commit/05a05bb81f780141e727a47cbe4cdcef31e4a311))
 * **[#259](https://github.com/hydro-project/hydroflow/issues/259)**
    - Rename split->unzip, implement surface op ([`293c37c`](https://github.com/hydro-project/hydroflow/commit/293c37cd477c88af4ff0a3aaeb15a2da30ea391b))
 * **[#261](https://github.com/hydro-project/hydroflow/issues/261)**
    - Add demux operator ([`d07e5c1`](https://github.com/hydro-project/hydroflow/commit/d07e5c16be1bf3de627cd0f45225146129a6ab41))
 * **[#277](https://github.com/hydro-project/hydroflow/issues/277)**
    - Improvements to book ([`a98c745`](https://github.com/hydro-project/hydroflow/commit/a98c7453df1ff1733000f5281f4ac9f9f5403537))
 * **[#278](https://github.com/hydro-project/hydroflow/issues/278)**
    - Add operator-specific diagnostics, use in `demux(..)`, fix #265 ([`7341f87`](https://github.com/hydro-project/hydroflow/commit/7341f87c821bcb534d232ce02fd113853c2ef17a))
 * **[#282](https://github.com/hydro-project/hydroflow/issues/282)**
    - Simplify boilerplate with new helpers, ops ([`57403cc`](https://github.com/hydro-project/hydroflow/commit/57403ccc3d66c07b4e1631a504904286a9cf28c3))
 * **[#284](https://github.com/hydro-project/hydroflow/issues/284)**
    - Rename source and dest surface syntax operators, fix #216 #276 ([`b7074eb`](https://github.com/hydro-project/hydroflow/commit/b7074ebb5d376493b52efe471b65f6e2c06fce7c))
 * **[#285](https://github.com/hydro-project/hydroflow/issues/285)**
    - `demux` use `Pusherator` automatically, fix #267 ([`36708ab`](https://github.com/hydro-project/hydroflow/commit/36708abaa599a0da4966c1265e97fcc9e5f08224))
 * **[#295](https://github.com/hydro-project/hydroflow/issues/295)**
    - Explicit serde example, resolve #214 ([`96f1481`](https://github.com/hydro-project/hydroflow/commit/96f1481fb73b2411f4afa161142ebd64b901ec60))
 * **[#296](https://github.com/hydro-project/hydroflow/issues/296)**
    - Make ipv4_resolve return a Result for use in clap ([`a2316df`](https://github.com/hydro-project/hydroflow/commit/a2316df30aecee9a083b345702d6f948fd2889a0))
 * **[#298](https://github.com/hydro-project/hydroflow/issues/298)**
    - Better names/structure for serde helper functions, get UdpSocket back from bind_udp_xxx calls ([`e6b1ec5`](https://github.com/hydro-project/hydroflow/commit/e6b1ec569afaba424ad8c7d18fdeef0d5344ca23))
 * **[#301](https://github.com/hydro-project/hydroflow/issues/301)**
    - Add sort_by, rename groupby to group_by ([`b5d6f60`](https://github.com/hydro-project/hydroflow/commit/b5d6f6086b37df15a73b199a4ad638596af82a34))
 * **[#309](https://github.com/hydro-project/hydroflow/issues/309)**
    - `epoch` --> `tick` replace ([`f4ad527`](https://github.com/hydro-project/hydroflow/commit/f4ad527151f9cb9d04616fe252ed1d54ea13d19d))
 * **[#311](https://github.com/hydro-project/hydroflow/issues/311)**
    - Better autogen of input/output specs for ops docs ([`2cbd3e7`](https://github.com/hydro-project/hydroflow/commit/2cbd3e7757da427a47fdde74278de3ec8cbbf9fb))
 * **[#320](https://github.com/hydro-project/hydroflow/issues/320)**
    - Better mermaid graphs ([`f2ee139`](https://github.com/hydro-project/hydroflow/commit/f2ee139666da9ab72093dde80812df6bc7bc0193))
 * **[#321](https://github.com/hydro-project/hydroflow/issues/321)**
    - Better graphs for both mermaid and dot ([`876fb31`](https://github.com/hydro-project/hydroflow/commit/876fb3140374588c55b4a7ec7a51e7cf6317eb67))
 * **[#323](https://github.com/hydro-project/hydroflow/issues/323)**
    - Cleanup and reorg book ([`501aeba`](https://github.com/hydro-project/hydroflow/commit/501aebac270b288ce49d8c9d4a28cef64424c37f))
 * **[#329](https://github.com/hydro-project/hydroflow/issues/329)**
    - Get hydroflow to compile to WASM ([`24354d2`](https://github.com/hydro-project/hydroflow/commit/24354d2e11c69e38e4e021aa4acf1525b376b2b1))
 * **[#331](https://github.com/hydro-project/hydroflow/issues/331)**
    - Get `hydroflow_lang` to compile on WASM ([`946d1a2`](https://github.com/hydro-project/hydroflow/commit/946d1a29bd1dcc2fa557c54e0da1edf74d77cf26))
 * **[#334](https://github.com/hydro-project/hydroflow/issues/334)**
    - Implement `'tick`/`'static` lifetimes for `cross_join` #272 ([`8b5faa7`](https://github.com/hydro-project/hydroflow/commit/8b5faa70dabadc13d0dd48c53c9d47d20a2bf36b))
 * **[#337](https://github.com/hydro-project/hydroflow/issues/337)**
    - Implement `'tick`/`'static` for `sort()` #272 ([`ffacc15`](https://github.com/hydro-project/hydroflow/commit/ffacc15f2d3905bfea0912408ba44ea2e712e620))
 * **[#338](https://github.com/hydro-project/hydroflow/issues/338)**
    - Implement `'tick`/`'static` for `unique()` #272 ([`fe79c6d`](https://github.com/hydro-project/hydroflow/commit/fe79c6dd9207384b36e59365bba6f3b36bbad783))
 * **[#350](https://github.com/hydro-project/hydroflow/issues/350)**
    - Fix `run_tick()` semantics, fix `unique`'s `'static` ([`d8d833c`](https://github.com/hydro-project/hydroflow/commit/d8d833c3b98c7e5f1c664e0731a670cfc5669b32))
 * **[#360](https://github.com/hydro-project/hydroflow/issues/360)**
    - Preserve varnames info, display in mermaid, fix #327 ([`e7acecc`](https://github.com/hydro-project/hydroflow/commit/e7acecc480fbc2031e83777f58e7eb16603b8f26))
 * **[#363](https://github.com/hydro-project/hydroflow/issues/363)**
    - Document surface syntax `context` object, cleanup internal usage ([`c259bea`](https://github.com/hydro-project/hydroflow/commit/c259beabb69f22e8e0cc9cd89ceffd0f416a11d2))
 * **[#381](https://github.com/hydro-project/hydroflow/issues/381)**
    - Fix `run_async()` not yielding with replay (stateful) operators ([`546b9e0`](https://github.com/hydro-project/hydroflow/commit/546b9e06f499d7f38bd91eb45b9031a8d7ea08de))
 * **[#382](https://github.com/hydro-project/hydroflow/issues/382)**
    - Add `anti_join` operator ([`54bcbaa`](https://github.com/hydro-project/hydroflow/commit/54bcbaa85ccf943ae11002f092cb7659fdc7fe59))
 * **[#383](https://github.com/hydro-project/hydroflow/issues/383)**
    - Allow alias name assignment without any arrow in surface syntax, closes #266 ([`9d17b4d`](https://github.com/hydro-project/hydroflow/commit/9d17b4d5da37efcde633a87cf489541cb5371555))
 * **[#399](https://github.com/hydro-project/hydroflow/issues/399)**
    - Refactor `OpConstraints` fns, use lookup helper ([`6570401`](https://github.com/hydro-project/hydroflow/commit/6570401af7038e881baecd3bc5a337f081b6f9fc))
 * **[#404](https://github.com/hydro-project/hydroflow/issues/404)**
    - Fix op docs "blocking" to check elided port names, fix #400 ([`608e65b`](https://github.com/hydro-project/hydroflow/commit/608e65b61788376a06ab56b7f92dfd45820b4c0e))
 * **[#405](https://github.com/hydro-project/hydroflow/issues/405)**
    - Make `DiMulGraph` fields private for encapsulation ([`89cab62`](https://github.com/hydro-project/hydroflow/commit/89cab6289180f8e046ff590825cc2b192cc8e1fb))
 * **[#412](https://github.com/hydro-project/hydroflow/issues/412)**
    - Add monotonicity properties to operators (currently unused) ([`9ead3f7`](https://github.com/hydro-project/hydroflow/commit/9ead3f7c654f8fb9fce7d8f53e56b0825c3b07b5))
 * **[#419](https://github.com/hydro-project/hydroflow/issues/419)**
    - Encapsulate `FlatGraph`, separate `FlatGraphBuilder` ([`fceaea5`](https://github.com/hydro-project/hydroflow/commit/fceaea5659ac76c2275c1487582a17b646858602))
 * **[#425](https://github.com/hydro-project/hydroflow/issues/425)**
    - Fix `FlatGraph::write_surface_syntax` ([`6f0c29a`](https://github.com/hydro-project/hydroflow/commit/6f0c29abf38f4ed892308cc18d2edcd1b44596a6))
 * **[#431](https://github.com/hydro-project/hydroflow/issues/431)**
    - Make `unique()` streaming and dedup Dedalus facts ([`68f9bde`](https://github.com/hydro-project/hydroflow/commit/68f9bde464122c41fab3a75897137d46be3bee38))
 * **[#434](https://github.com/hydro-project/hydroflow/issues/434)**
    - Add `try_build` to flat graph to expose diagnostics ([`2a6ddd5`](https://github.com/hydro-project/hydroflow/commit/2a6ddd58d3803392be0461ec49271d27da2dd38d))
 * **[#441 1/14](https://github.com/hydro-project/hydroflow/issues/441 1/14)**
    - Move `find_barrier_crossers`, coloring, subgraph-making into builder ([`b977e95`](https://github.com/hydro-project/hydroflow/commit/b977e95276ea7461cbb786c93715146a5b2bb820))
 * **[#441 10/14](https://github.com/hydro-project/hydroflow/issues/441 10/14)**
    - Remove `subgraph_send/recv_handoffs` from `PartitionedGraph`, compute on the fly ([`a1efedc`](https://github.com/hydro-project/hydroflow/commit/a1efedc10fd9754ab9ff47d1b5b0eb4a3c2e4f9f))
 * **[#441 11/14](https://github.com/hydro-project/hydroflow/issues/441 11/14)**
    - Remove `FlatGraph`, unify under `PartitionedGraph` ([`b640b53`](https://github.com/hydro-project/hydroflow/commit/b640b532e34b29f44c768d523fbf780dba9785ff))
 * **[#441 12/14](https://github.com/hydro-project/hydroflow/issues/441 12/14)**
    - Rename `PartitionedGraph` -> `HydroflowGraph` ([`f95b325`](https://github.com/hydro-project/hydroflow/commit/f95b325dafcd5574050563f62a94d89a2fa811c8))
 * **[#441 13/14](https://github.com/hydro-project/hydroflow/issues/441 13/14)**
    - Make `HydroflowGraph` fields private ([`3ddb10a`](https://github.com/hydro-project/hydroflow/commit/3ddb10a3802804c006087a1629654e88ad4992bc))
 * **[#441 14/14](https://github.com/hydro-project/hydroflow/issues/441 14/14)**
    - Cleanup graph docs, organize method names ([`09d3b57`](https://github.com/hydro-project/hydroflow/commit/09d3b57eb03f3920bd10f5c10277d3ef4f9cb0ec))
 * **[#441 2/14](https://github.com/hydro-project/hydroflow/issues/441 2/14)**
    - Move `find_subgraph_strata()` into builder ([`9dcaea8`](https://github.com/hydro-project/hydroflow/commit/9dcaea8506ba94610b0575a65bbd48334bb4631d))
 * **[#441 3/14](https://github.com/hydro-project/hydroflow/issues/441 3/14)**
    - Move `separate_external_inputs()` into builder ([`dcceaf1`](https://github.com/hydro-project/hydroflow/commit/dcceaf1a26d928dfe1ed6c6b55b0a252fcdf1415))
 * **[#441 4/14](https://github.com/hydro-project/hydroflow/issues/441 4/14)**
    - `helper_find_subgraph_handoffs()` (does not compile) ([`7e90818`](https://github.com/hydro-project/hydroflow/commit/7e90818c13a8c174f693196e5991b0e0ce77d960))
 * **[#441 5/14](https://github.com/hydro-project/hydroflow/issues/441 5/14)**
    - Working, moved internal handoffs ([`733b00c`](https://github.com/hydro-project/hydroflow/commit/733b00c3836dc75b0d3afb25d0d6f3ed01839c8b))
 * **[#441 6/14](https://github.com/hydro-project/hydroflow/issues/441 6/14)**
    - Remove builder (didn't do much really) ([`c0c00b3`](https://github.com/hydro-project/hydroflow/commit/c0c00b305a8a698b8cb14fbdb64a64006daa096a))
 * **[#441 7/14](https://github.com/hydro-project/hydroflow/issues/441 7/14)**
    - Regenerate colors in `SerdeGraph`, remove from `PartitionedGraph` ([`f37c025`](https://github.com/hydro-project/hydroflow/commit/f37c025cbb70db597b1585370ee1c35819c68236))
 * **[#441 8/14](https://github.com/hydro-project/hydroflow/issues/441 8/14)**
    - Encapsulate subgraph insertion ([`eb8f0e4`](https://github.com/hydro-project/hydroflow/commit/eb8f0e49a78deadb2888068e5a23ed45bcada05c))
 * **[#441 9/14](https://github.com/hydro-project/hydroflow/issues/441 9/14)**
    - Update subgraph handoff algorithm ([`577071a`](https://github.com/hydro-project/hydroflow/commit/577071a9898a3a2490d059ad5cc3d9b80b7c7e79))
 * **[#444](https://github.com/hydro-project/hydroflow/issues/444)**
    - Add snapshot testing of graph visualizations (mermaid and dot) ([`58a2438`](https://github.com/hydro-project/hydroflow/commit/58a24387c001cbda78ad87c7c2d0c2e2502b3099))
 * **[#445](https://github.com/hydro-project/hydroflow/issues/445)**
    - Add `demux` operator to Hydro CLI to map node IDs to connections ([`886d00f`](https://github.com/hydro-project/hydroflow/commit/886d00f6694ba926c9e1ff184acb31a5d60cee23))
 * **[#448 1/2](https://github.com/hydro-project/hydroflow/issues/448 1/2)**
    - Avoid spinning on internal state replay, fix #380 ([`742ca19`](https://github.com/hydro-project/hydroflow/commit/742ca1962a46db015ef83a2bb18565862626b2a5))
 * **[#455](https://github.com/hydro-project/hydroflow/issues/455)**
    - Add `source_stream(...)` type guard ([`f09227b`](https://github.com/hydro-project/hydroflow/commit/f09227b1890f3548122ec1c35e91fd7f573c8eda))
 * **[#459](https://github.com/hydro-project/hydroflow/issues/459)**
    - Fix coloring (pull vs push) error in serdegraph, recompute colors rather than serializing ([`86d5623`](https://github.com/hydro-project/hydroflow/commit/86d562316a99b0095d32e9a8e5218432396febbb))
 * **[#465](https://github.com/hydro-project/hydroflow/issues/465)**
    - Add generic arg to `identity()`, add tests, close #392 ([`09dd190`](https://github.com/hydro-project/hydroflow/commit/09dd19042cf8d1c9c3d6456cfb0ce33e7117e9af))
 * **[#468](https://github.com/hydro-project/hydroflow/issues/468)**
    - Add scalar `persist()` operator, #438 ([`688026b`](https://github.com/hydro-project/hydroflow/commit/688026b29490936906eb77314466eb85f95dbab3))
 * **[#471](https://github.com/hydro-project/hydroflow/issues/471)**
    - Add buffer operator ([`119ba93`](https://github.com/hydro-project/hydroflow/commit/119ba9365c775b3d2a3d89d00460a4af5f9d2225))
 * **[#475](https://github.com/hydro-project/hydroflow/issues/475)**
    - Use prettyplease to prettify hydroflow graph output ([`323279a`](https://github.com/hydro-project/hydroflow/commit/323279ad2597b75119b5cb7979702c41fd7e6477))
 * **[#488](https://github.com/hydro-project/hydroflow/issues/488)**
    - Remove extra clone in groupby ([`5f1f6b4`](https://github.com/hydro-project/hydroflow/commit/5f1f6b4759dc8bbdce417bb05af994fde7b40664))
 * **[#490](https://github.com/hydro-project/hydroflow/issues/490)**
    - Resolve #354 to document repeat_iter ([`7322ab3`](https://github.com/hydro-project/hydroflow/commit/7322ab3fefdd9e3cf47bc55b5e01413cc53ca05a))
 * **[#491](https://github.com/hydro-project/hydroflow/issues/491)**
    - Add `initialize()` operator equivalent to `source_iter([()])`, close #110 ([`a613632`](https://github.com/hydro-project/hydroflow/commit/a6136324d2f152aef8a040775e3ea188e217e5ee))
 * **[#493](https://github.com/hydro-project/hydroflow/issues/493)**
    - Add `source_interval` op based on `tokio::time::Interval`, close #361 ([`488c001`](https://github.com/hydro-project/hydroflow/commit/488c001bb7a042d1eda4df24d93ca3fc3741d359))
 * **[#497](https://github.com/hydro-project/hydroflow/issues/497)**
    - Add `source_json` operator, use in `two_pc` ([`c5933a5`](https://github.com/hydro-project/hydroflow/commit/c5933a549703b1d7f88d4f5801523864c263069e))
 * **[#499](https://github.com/hydro-project/hydroflow/issues/499)**
    - Dontdrophandoffs ([`b603581`](https://github.com/hydro-project/hydroflow/commit/b603581b83423e161ccac53607022d6e4857fa71))
 * **[#501](https://github.com/hydro-project/hydroflow/issues/501)**
    - Preserve serialize diagnostics for hydroflow graph, stop emitting expected warnings in tests ([`0c810e5`](https://github.com/hydro-project/hydroflow/commit/0c810e5fdd3445923c0c7afbe651f2b4a72c115e))
 * **[#508](https://github.com/hydro-project/hydroflow/issues/508)**
    - Use `null` write fn when operator codegen errors ([`1227446`](https://github.com/hydro-project/hydroflow/commit/1227446ab97edc3d298fb7ef2692450efa2cabda))
 * **[#509](https://github.com/hydro-project/hydroflow/issues/509)**
    - Even faster groupby ([`af304aa`](https://github.com/hydro-project/hydroflow/commit/af304aa7ed35e6d5d7ed0936e3827de2b40e1ddb))
 * **[#511](https://github.com/hydro-project/hydroflow/issues/511)**
    - Fix multi-line code blocks, mermaid styling ([`2e0b4dc`](https://github.com/hydro-project/hydroflow/commit/2e0b4dc17820bf08772022f2b8b45c1aa6971949))
 * **[#512](https://github.com/hydro-project/hydroflow/issues/512)**
    - Display varnames in dot output, fix #385 ([`8746c3c`](https://github.com/hydro-project/hydroflow/commit/8746c3c9bd32ba163fadc6789e95d5a3c69b9eb9))
 * **[#518](https://github.com/hydro-project/hydroflow/issues/518)**
    - Attach spans to generated Hydroflow code in Dedalus ([`f00d865`](https://github.com/hydro-project/hydroflow/commit/f00d8655aa4404ddcc812e0decf8c1e48e62b0fd))
 * **[#523](https://github.com/hydro-project/hydroflow/issues/523)**
    - Lattice join ([`f6af455`](https://github.com/hydro-project/hydroflow/commit/f6af455a2a8e49046d70546fbc6f8c69f8c8e3b2))
 * **[#524](https://github.com/hydro-project/hydroflow/issues/524)**
    - Fix lattice join cases ([`90c456e`](https://github.com/hydro-project/hydroflow/commit/90c456ec00bae11bfe0cd71c64e2c0a065bb70a8))
 * **[#526](https://github.com/hydro-project/hydroflow/issues/526)**
    - Add repeat_fn op ([`9620b91`](https://github.com/hydro-project/hydroflow/commit/9620b912fd09bcf92ee29944314083de1a0e6c62))
 * **[#530](https://github.com/hydro-project/hydroflow/issues/530)**
    - Add specialized `lattice_merge::<MyLatRepr>()` operator ([`1a9b652`](https://github.com/hydro-project/hydroflow/commit/1a9b65286e41013178adfba11bcdde4e3b5c44d8))
 * **[#538](https://github.com/hydro-project/hydroflow/issues/538)**
    - Source_stream_serde returns Result<T> instead of T ([`7c38361`](https://github.com/hydro-project/hydroflow/commit/7c383611eca4bd80a0d4ee40ae60dcf903939ef5))
 * **[#547](https://github.com/hydro-project/hydroflow/issues/547)**
    - Add transform to remove extra `merge()`s and `tee()`s ([`838ac2a`](https://github.com/hydro-project/hydroflow/commit/838ac2a4d9a2e3ea1a4cdb5f8702c8d2b1eb3e5e))
 * **[#550](https://github.com/hydro-project/hydroflow/issues/550)**
    - Fix `persist()` operator not self-scheduling for replay ([`6831a65`](https://github.com/hydro-project/hydroflow/commit/6831a6529d842e3123c145bbf20e8635d1e9a85a))
 * **[#555](https://github.com/hydro-project/hydroflow/issues/555)**
    - Antijoin uses FxHash instead of SipHash ([`55fa0a2`](https://github.com/hydro-project/hydroflow/commit/55fa0a2a733a482400e01edd495ef429a54ac555))
 * **[#556](https://github.com/hydro-project/hydroflow/issues/556)**
    - Unique uses FxHash instead of SipHash ([`4323d47`](https://github.com/hydro-project/hydroflow/commit/4323d47efc495940cc4bf41f647e4e187bf1305b))
 * **[#559](https://github.com/hydro-project/hydroflow/issues/559)**
    - Add optional multiset join operator ([`c70644d`](https://github.com/hydro-project/hydroflow/commit/c70644ddb784449b55a84278cb1bf8cc38557d82))
 * **[#566](https://github.com/hydro-project/hydroflow/issues/566)**
    - Only filter out duplicate elements in one place for persisted relations ([`a37a511`](https://github.com/hydro-project/hydroflow/commit/a37a511c37fd362044b563268e95fdf152700acf))
 * **[#573](https://github.com/hydro-project/hydroflow/issues/573)**
    - Make profiles easier to interpret ([`d0e5df1`](https://github.com/hydro-project/hydroflow/commit/d0e5df13d5bc3dd4a986e70f2125978bd2878b96))
 * **[#579](https://github.com/hydro-project/hydroflow/issues/579)**
    - Add `repeat_iter_external()` operator for spinning ([`e2e204d`](https://github.com/hydro-project/hydroflow/commit/e2e204d486d70d41aa1f5d9b6e5e9424a0280dc4))
 * **[#591](https://github.com/hydro-project/hydroflow/issues/591)**
    - Add `keyed_reduce()` operator, make `group_by()` an alias of renamed `keyed_fold()` operator ([`71c72ff`](https://github.com/hydro-project/hydroflow/commit/71c72ffa6d669a098e634a7c6c0fc153c0e596fa))
 * **[#598](https://github.com/hydro-project/hydroflow/issues/598)**
    - Add `index()` operator for getting the index of the current group ([`6f959b6`](https://github.com/hydro-project/hydroflow/commit/6f959b64f0cf494c23f9ec8bc107a23e006aeacf))
 * **[#599](https://github.com/hydro-project/hydroflow/issues/599)**
    - Add `enumerate` operator ([`73da148`](https://github.com/hydro-project/hydroflow/commit/73da148c80f9834b6d2ea582ef4a020b7f7eb38e))
 * **[#602](https://github.com/hydro-project/hydroflow/issues/602)**
    - Remove `std`-ified `once_cell` crate, remove dead bespoke `Once` channel code ([`753f38c`](https://github.com/hydro-project/hydroflow/commit/753f38c9c4ee46cf315d68ed4d4978275f6a6b3a))
 * **[#604](https://github.com/hydro-project/hydroflow/issues/604)**
    - Don't drop groupby hash table for 'tick lifetimes ([`cc1b762`](https://github.com/hydro-project/hydroflow/commit/cc1b762364dd66e496cdc766f8694bea256dd0d1))
 * **[#605](https://github.com/hydro-project/hydroflow/issues/605)**
    - Add batch limit to batch and fix scheduling poor behavior ([`f831f9d`](https://github.com/hydro-project/hydroflow/commit/f831f9d8518bbc55f1c5e7b78e9b3ca189b2adfb))
 * **[#607](https://github.com/hydro-project/hydroflow/issues/607)**
    - Don't drop updated_keys in lattice join, drain it and reuse it ([`b06ef93`](https://github.com/hydro-project/hydroflow/commit/b06ef93a35ac7591bd2314bf8ca6b2e1bb22ff20))
 * **[#609](https://github.com/hydro-project/hydroflow/issues/609)**
    - Update syn to 2.0 ([`2e7d802`](https://github.com/hydro-project/hydroflow/commit/2e7d8024f35893ef0abcb6851e370b00615f9562))
 * **[#610](https://github.com/hydro-project/hydroflow/issues/610)**
    - Don't dump payload to terminal when dest_sink/dest_sink_serde fails ([`1756f1a`](https://github.com/hydro-project/hydroflow/commit/1756f1a200ee84786794ef9b93f33478459cda73))
 * **[#617](https://github.com/hydro-project/hydroflow/issues/617)**
    - Update `Cargo.toml`s for publishing ([`a78ff9a`](https://github.com/hydro-project/hydroflow/commit/a78ff9aace6771787c2b72aad83be6ad8d49a828))
 * **Uncategorized**
    - Setup release workflow ([`108d0e9`](https://github.com/hydro-project/hydroflow/commit/108d0e933a08b183c4dadf8c3499e4946696e263))
    - Use clear rather than default for join state #562 ([`c4f3f97`](https://github.com/hydro-project/hydroflow/commit/c4f3f97bab8a1cb5d3453290f567798b4bc4b60d))
    - Add `dest_file(filename, append)` operator ([`7807687`](https://github.com/hydro-project/hydroflow/commit/7807687fa9ba52c67fb5eb286aece37fab82a67b))
    - Add `source_file(filename)` operator ([`f3e1f98`](https://github.com/hydro-project/hydroflow/commit/f3e1f983c5622f8297f807c6afc0d8f441ccb33e))
    - Update surface syntax missing runtime messages ([`e967c02`](https://github.com/hydro-project/hydroflow/commit/e967c026f4a237b6bedf7a155bc0e53ece71919f))
    - Use macro to declare & import operators ([`ca826f7`](https://github.com/hydro-project/hydroflow/commit/ca826f738820d9efe7101a5a04b0fbf850d50423))
    - Use `HydroflowGraph` for graph writing, delete `SerdeGraph` ([`d1ef14e`](https://github.com/hydro-project/hydroflow/commit/d1ef14ee459c51d5a2dd9e7ea03050772e14178c))
    - Serialize `HydroflowGraph` instead of `SerdeGraph` ([`ae205c6`](https://github.com/hydro-project/hydroflow/commit/ae205c69538fab9eeedd8fa460b8eef295d26bc2))
    - Abstract mermaid/dot writing into `GraphWrite` trait ([`fc0826d`](https://github.com/hydro-project/hydroflow/commit/fc0826d75e38a3b233085c5aa23117635b308395))
    - Additional cleanups for PR #407 ([`fff4d0a`](https://github.com/hydro-project/hydroflow/commit/fff4d0a708c15f2609c0db9122e0b19abcaaa779))
    - Build `OperatorInstance` data in `FlatGraph` ([`c883fd4`](https://github.com/hydro-project/hydroflow/commit/c883fd4ccd50638bdab0dbbc00f75cc74f001e16))
    - Fixup! Update examples to use forward name references ([`8406905`](https://github.com/hydro-project/hydroflow/commit/8406905f9a2ace1622b7fc85122a710c56877c67))
    - Detect name cycles sooner, memoize resolution, better error messages ([`00d5f63`](https://github.com/hydro-project/hydroflow/commit/00d5f63a2b672648831d98d65eae4d4e09bf9ed3))
    - Refactor `FlatGraph` assembly into separate `FlatGraphBuilder` ([`9dd3bd9`](https://github.com/hydro-project/hydroflow/commit/9dd3bd91586966484abaf01c4330d831804b1983))
    - Update examples to use forward name references ([`398cff6`](https://github.com/hydro-project/hydroflow/commit/398cff6b9b27ec8091d90f8f3e844d2574d9429f))
    - Implement forward name references in surface syntax, closes #158 ([`8cc479e`](https://github.com/hydro-project/hydroflow/commit/8cc479ea99fd2a58751fc24f8b46d60e8594d24a))
    - Improve parsing handling/error messages ([`bfe9a90`](https://github.com/hydro-project/hydroflow/commit/bfe9a906d37f9f91ccea3fe7e6414ec62c695c78))
    - Fixup! Add `DiMulGraph`, use in `FlatGraph` (not compiling) ([`da1047c`](https://github.com/hydro-project/hydroflow/commit/da1047c7adb32acb8a048cc640dda3d891fcd896))
    - :<'static>` now replays #143 #364 ([`62fcfb1`](https://github.com/hydro-project/hydroflow/commit/62fcfb157eaaaabedfeb5c77b2a6df89ee1a6852))
    - :<'static>` now replays #143 #364 ([`bc3d12f`](https://github.com/hydro-project/hydroflow/commit/bc3d12f563dab96f4751ec21cd20b193eea95456))
    - :<'static>` now replays #143 #364 ([`a2078f7`](https://github.com/hydro-project/hydroflow/commit/a2078f7056a54d20f91e2e0f9a7617dc6ef1f627))
    - `repeat_iter` now repeats via self-scheduling #143 #364 ([`e5f46df`](https://github.com/hydro-project/hydroflow/commit/e5f46df99299771cb52127ff07bfbc26a46cb569))
    - Remove unnecessary `mut` from `repeat_iter` ([`13a51e5`](https://github.com/hydro-project/hydroflow/commit/13a51e514ed50e6924a26702a240e891946bc085))
    - Add persistence lifetimes to `reduce` ([`050cadf`](https://github.com/hydro-project/hydroflow/commit/050cadffaf6c1287e374c83e81ad57cd6ef67ec3))
    - Add persistence lifetimes to `fold` ([`1283da5`](https://github.com/hydro-project/hydroflow/commit/1283da5f1534d6bf0d2e85ab96e4ec514d1bb845))
    - Replace old references to `'epoch` with `'static` ([`8431060`](https://github.com/hydro-project/hydroflow/commit/84310607b6f07fe5c8fdd4877bf288cad1e0b003))
    - Ops specify persistence/type arg counts, handle separately in `partitioned_graph` ([`cdc83b6`](https://github.com/hydro-project/hydroflow/commit/cdc83b68d989d60732c01fb99957762781d161cb))
    - Add post-partitioning step to break source operators into stratum 0, fix #348 ([`9a746a0`](https://github.com/hydro-project/hydroflow/commit/9a746a0dbe6fbeb268d0e4144bd1ce8cc83da36f))
    - Add `is_external_input` field to `OperatorConstraints` ([`861fd94`](https://github.com/hydro-project/hydroflow/commit/861fd94a1cea26a7843084eddac205b487db24a1))
    - Add generic type arguments for `group_by` when inference fails #272 ([`75f152e`](https://github.com/hydro-project/hydroflow/commit/75f152ef9170982336da0a19dd334b8065975036))
    - Add persistence spec to `group_by` #272 ([`df13190`](https://github.com/hydro-project/hydroflow/commit/df131909a1725ca941d76a19168d22c12bfa775d))
    - Add persistence lifetimes to join #272 ([`47b2941`](https://github.com/hydro-project/hydroflow/commit/47b2941d74704792e5e2a7f30fa088c81c3ab506))
    - Fix rare bug in `dest_sink` doctest ([`d4be35b`](https://github.com/hydro-project/hydroflow/commit/d4be35b36381b21e5c8955ecfecc9332f15a167c))
    - Type guard for `source_iter`, `repeat_iter` #263 ([`496a7a1`](https://github.com/hydro-project/hydroflow/commit/496a7a11629533944064e2e86fd7b0e2026be8cf))
    - Add type guard to `group_by` #263 ([`3fcfb46`](https://github.com/hydro-project/hydroflow/commit/3fcfb464f7b527a7ddc43926a10827c125c2e8e4))
    - Simplify `dest_sink`, add type guards #263 ([`6aa4d41`](https://github.com/hydro-project/hydroflow/commit/6aa4d41cc75825e5ea1c4c8bfe590f02387fcc5e))
    - Add type guard before `Pivot` #263 ([`c215e8c`](https://github.com/hydro-project/hydroflow/commit/c215e8c4523a1e465eafa3320daa34d6cb35aa11))
    - Add type guard to `merge` #263 ([`6db3f60`](https://github.com/hydro-project/hydroflow/commit/6db3f6013a934b3087c8d116e61fbfc293e1baa0))
    - Emit type guards inline, configurable #263 ([`c6510da`](https://github.com/hydro-project/hydroflow/commit/c6510da4b4cb46ec026e3c1c69b5ce29b17c473c))
    - Add very good type guard to `join` op #263 ([`3ee9d33`](https://github.com/hydro-project/hydroflow/commit/3ee9d338c27859b31a057be53ee9251248ca235c))
    - Improve spanning of write context `make_ident(..)` #263 ([`58668bd`](https://github.com/hydro-project/hydroflow/commit/58668bd6ec758ed091b754f8769ed8c243cbde78))
    - Improve spanning of handoffs #263 ([`53e62cd`](https://github.com/hydro-project/hydroflow/commit/53e62cd36bba66bbddeaba845d39d56a1124f157))
    - Improve `Iterator`/`Pusherator` typeguards by erasing types, using local fns #263 ([`6413fa4`](https://github.com/hydro-project/hydroflow/commit/6413fa417cab0481e3db1adbcaf71525eb866cc9))
    - Rename `recv_into` -> `collect_ready` ([`32fddfe`](https://github.com/hydro-project/hydroflow/commit/32fddfec46d2d136b4fc399fc0c438f922012487))
    - Remove `dest_asyncwrite`, consolidate using codecs, now in `hydroflow::util::udp/tcp`, fix #216 ([`5418ea4`](https://github.com/hydro-project/hydroflow/commit/5418ea47c7cbe0cf9be755346b0054faeb98d5c1))
    - Add example usage code to `dest_sink`, `dest_asyncwrite`, #216 ([`05c990f`](https://github.com/hydro-project/hydroflow/commit/05c990fcad2bc7ee64b7d58fce11bb126655a359))
    - Rename variadics/tuple_list macros ([`91d37b0`](https://github.com/hydro-project/hydroflow/commit/91d37b022b1cd0ed590765c40ef43244027c8035))
    - Disallow overwriting names in surface syntax (preps for #158) ([`7db1357`](https://github.com/hydro-project/hydroflow/commit/7db13575f97deedc2730f7f43bebc1282d9deec9))
    - More `indices` -> `ports` renames ([`696eb32`](https://github.com/hydro-project/hydroflow/commit/696eb321eee9a704df67ff7555bfefaf21e6f793))
    - Clarify handling of ports/`Ends` naming in `FlatGraph` ([`1534e1a`](https://github.com/hydro-project/hydroflow/commit/1534e1acf70bef1e14b0fab89f7062c1d8a5ad36))
    - Allow `clippy::uninlined-format-args` in `.cargo/config.toml` ([`17be5dd`](https://github.com/hydro-project/hydroflow/commit/17be5dd3993ee3239a3fbdb81572923479b0cc3e))
    - Add/update more operator docs ([`43e32ee`](https://github.com/hydro-project/hydroflow/commit/43e32eefa1ae2c6db7389ac023d16fae21b05e34))
    - Move operators into individual files, use `#[hydroflow_internalmacro::operator_docgen]` macro ([`694571b`](https://github.com/hydro-project/hydroflow/commit/694571b9b10393e7027a35a35a43b13d9035fb8b))
    - Implement `hydroflow_internalmacro::operator_docgen` for surface op docgen ([`5d56aaf`](https://github.com/hydro-project/hydroflow/commit/5d56aaf59a38ddb686862f8456e50d1b4025480a))
    - Refactor out surface syntax diagnostics (error messages) ([`008425b`](https://github.com/hydro-project/hydroflow/commit/008425bb436042524f540fc05a855f5fa5535c76))
    - Implement named ports in operators ([`879e977`](https://github.com/hydro-project/hydroflow/commit/879e977205f055e9712c2887a275dcdbee49f540))
    - Add parsing of named ports (WIP, compiling) ([`bd8313c`](https://github.com/hydro-project/hydroflow/commit/bd8313cf59a30bb121c07d754099d92c13daa734))
    - Remove surface API, fix #224 ([`7b75f5e`](https://github.com/hydro-project/hydroflow/commit/7b75f5eb73046c3fe9f50970e05b4665bc0bf7fc))
    - Implement `inspect()` surface syntax operator, fix #208 ([`7797c6c`](https://github.com/hydro-project/hydroflow/commit/7797c6c4aff07f780069bb9af2b12b8999b33725))
    - Add type guards, better spans to surface syntax codegen ([`09953f7`](https://github.com/hydro-project/hydroflow/commit/09953f73e96fdfd985daf555e01e46f5d54320b0))
    - Fix surface syntax port ordering bug ([`c241c05`](https://github.com/hydro-project/hydroflow/commit/c241c0580616d81e725e60afeeb7d60b3a47dab8))
    - Implement and add test for `sink_async` ([`19424cf`](https://github.com/hydro-project/hydroflow/commit/19424cfa02443a44ea022c1558e4a010545df9d6))
    - Emit better, more local, more useful error messages in surface syntax ([`bba512f`](https://github.com/hydro-project/hydroflow/commit/bba512f3c5d3a05633f3b1c90a11189dba73b938))
    - Restructor operator constraints into single `write_fn` ([`4a36e1b`](https://github.com/hydro-project/hydroflow/commit/4a36e1b7057ff17cdadfe85a64726c3324c27b25))
    - Rename `send_async` -> `write_async` to match trait names ([`666d14e`](https://github.com/hydro-project/hydroflow/commit/666d14e63ba870f7d1bb9bb7486ff45720c079e6))
    - Remove internal runtime, use tokio::spawn mechanism (requires tokio context) ([`302b213`](https://github.com/hydro-project/hydroflow/commit/302b213c6432c5d16cf517557eec8a876f46085d))
    - Fix handling of empty `merge()`/`tee()`, add tests ([`3a0ab8a`](https://github.com/hydro-project/hydroflow/commit/3a0ab8a51c31f57145fe52c362fb6ab49f8a6370))
    - Surface syntax fix handling of wildcard linear chains which might cause later pull-push conflicts ([`3559fbf`](https://github.com/hydro-project/hydroflow/commit/3559fbfa19711447fc53dfc597ad18b9a2f81a50))
    - Update `recv_stream` to handle all `Stream`s instead of just `tokio::mpsc::unbounded_channel` ([`8b68c64`](https://github.com/hydro-project/hydroflow/commit/8b68c643b55e9a04f373bded939b512be4ee0d7f))
    - Add more useful `DiMulGraph` methods ([`ffc5dc9`](https://github.com/hydro-project/hydroflow/commit/ffc5dc929573922b6d0228a6958caaaae9c19d32))
    - Cleanup code using `DiMulGraph` ([`ca5a540`](https://github.com/hydro-project/hydroflow/commit/ca5a54089e1c6a699f23d1a0af99e14713231510))
    - Re-enable detection of conflicting surface syntax ports ([`b76d334`](https://github.com/hydro-project/hydroflow/commit/b76d334cf996da1593bc47d797a64d4267013a0a))
    - Use `DiMulGraph` in `flat_to_partitioned.rs` and `PartitionedGraph`, working ([`cdd45fe`](https://github.com/hydro-project/hydroflow/commit/cdd45fe8eeefaa997bc2d38386fb9d33daf47b50))
    - Add `DiMulGraph`, use in `FlatGraph` (not compiling) ([`5e3dbaa`](https://github.com/hydro-project/hydroflow/commit/5e3dbaa214b9e33ad5bcd07e2fa70626105f9358))
    - Fix handing of "complex" expressions in recv_stream ([`7c67e2d`](https://github.com/hydro-project/hydroflow/commit/7c67e2ddc435effd7120bcc8ff8a1ab7e034d457))
    - Add comments to flat_graph ([`09e5cfd`](https://github.com/hydro-project/hydroflow/commit/09e5cfdc2c0e02d1c84251008814f1f569048b18))
    - Add `null` (nothing) operator ([`309163b`](https://github.com/hydro-project/hydroflow/commit/309163b9afc2f310a26e89733bcce9b82e0a9f83))
    - Refactor for foundation of properties iterators ([`a14c439`](https://github.com/hydro-project/hydroflow/commit/a14c439f82f5811299c352c1eb7508f6c18839ce))
    - Use `BTreeMap` instead of `HashMap` in surface syntax codegen for determinism ([`cca822a`](https://github.com/hydro-project/hydroflow/commit/cca822a0f34b7ffe272ad50dde87d873743233c7))
    - Fix subtle partial write bug in `send_async()` ([`f075150`](https://github.com/hydro-project/hydroflow/commit/f075150d08bc65fde4dc90d5e9a4bf125946c11c))
    - Fix handling of warnings, degenerate merge and tee ([`13c15d7`](https://github.com/hydro-project/hydroflow/commit/13c15d798a5b2f51c58f9812f2e59b47b760153a))
    - Add stratum consolidation as an optimization ([`7f76dba`](https://github.com/hydro-project/hydroflow/commit/7f76dba1512e2e1c33e94c73e223fd30fb94f059))
    - Add note about entire subgraph being pull ([`22d8432`](https://github.com/hydro-project/hydroflow/commit/22d8432fe2668a108965568f64b5d712bf63d957))
    - Add `send_async(impl AsyncWrite)` surface syntax operator, Hydroflow tokio runtime ([`e5abe91`](https://github.com/hydro-project/hydroflow/commit/e5abe911a428015bf3d4699812530dd8d4e226ab))
    - Move flat->partitioned conversion code into separate module ([`75547fa`](https://github.com/hydro-project/hydroflow/commit/75547fa110ab31939d16cc560197d36816e53077))
    - Move `scc_kosaraju()` into separate helper fn ([`647fa20`](https://github.com/hydro-project/hydroflow/commit/647fa20c5b93e6b2e63ed476aa4a1912176263ae))
    - Break up `find_subgraph()` with helper fns ([`a71c228`](https://github.com/hydro-project/hydroflow/commit/a71c228d41d532dfa39c9fbd67f744d94616ff7b))
    - Get rid of cloned preds/succs, use helper to insert handoffs ([`46e7fd8`](https://github.com/hydro-project/hydroflow/commit/46e7fd8da53e4e24c8d56970c024da81329127d5))
    - Implement and use `insert_intermediate_node()` helper fn for graph manipulation ([`79f0154`](https://github.com/hydro-project/hydroflow/commit/79f0154aaccafc4c0eac8783dcf0eb9e3f6067c8))
    - Fix potential bug when making `condensed_preds` ([`c2ff6ea`](https://github.com/hydro-project/hydroflow/commit/c2ff6ea70262574747b08d595e1996e8297f08ba))
    - Move code into `can_connect_colorize()` helper ([`090f87c`](https://github.com/hydro-project/hydroflow/commit/090f87c7669b1fb6128807ace0e5af3e7ad44c5a))
    - Move code into `find_barrier_crossers()` helper ([`485928e`](https://github.com/hydro-project/hydroflow/commit/485928e04bcca452876530b89f3c078e95ec38a2))
    - Fix `InputBarrier` to not have silly `None` variant ([`6b6136e`](https://github.com/hydro-project/hydroflow/commit/6b6136eb7646b8f84d489405f780c8429cb173ba))
    - Add comments, cleanup for PR ([`03531dd`](https://github.com/hydro-project/hydroflow/commit/03531ddcaf173be7b0361dafcdd13936751e69ce))
    - Fix lint errors ([`5b59c79`](https://github.com/hydro-project/hydroflow/commit/5b59c79041400c45b3f1a1b8efe193ce2d3d99d0))
    - Add topo_sort test ([`dd82d44`](https://github.com/hydro-project/hydroflow/commit/dd82d440286764b522668344eb721b13020a6b34))
    - Use separate topo_sort function ([`557b665`](https://github.com/hydro-project/hydroflow/commit/557b665e62ebba2c494615f562da9190e5442cae))
    - Add sort surface syntax operator, test ([`bb7d334`](https://github.com/hydro-project/hydroflow/commit/bb7d3346762d93b0feb5186f85b4f371b8e773b8))
    - Add more tests, fix surface syntax bugs ([`eb62ef1`](https://github.com/hydro-project/hydroflow/commit/eb62ef1a47ec58abcf6a11745667e00d69df6d93))
    - Add stratification tests ([`dbbce89`](https://github.com/hydro-project/hydroflow/commit/dbbce8921b405240b9254d5ce06eef665603bf86))
    - Reorganization, epoch-crossing in subgraph compilation ([`a9595de`](https://github.com/hydro-project/hydroflow/commit/a9595de3050617eed83730611d00f1e60b366404))
    - Fold issue notes ([`440b251`](https://github.com/hydro-project/hydroflow/commit/440b2513835b12125a67f8b15e31229257e43841))
    - Fix difference forgetfulness ([`093eb45`](https://github.com/hydro-project/hydroflow/commit/093eb45b262d17a26ce2f331bf571305c7cc83d7))
    - Add fold() and reduce() surface syntax operators ([`80d4385`](https://github.com/hydro-project/hydroflow/commit/80d4385386dd0818730820f92b77777dee9e85fa))
    - Stratification WIP 4/4 ([`ee0c4ed`](https://github.com/hydro-project/hydroflow/commit/ee0c4ed46fced698dd7135a14e3e1945c2fed55a))
    - Stratification WIP 3/4 ([`7557f2d`](https://github.com/hydro-project/hydroflow/commit/7557f2d78737d3b2bba7742bfd4d42c2a8476776))
    - Stratification WIP 2/4 ([`2c39fe2`](https://github.com/hydro-project/hydroflow/commit/2c39fe2053a2c7ae2ea267d9843f9e6db11183d8))
    - Stratification WIP 1/4 ([`553740f`](https://github.com/hydro-project/hydroflow/commit/553740fe87a47e6858c84064c0fcdc0b99e66d43))
    - Check operator number of expression arguments ([`20c3eeb`](https://github.com/hydro-project/hydroflow/commit/20c3eeb6e6b653e92277c35a759c320166693404))
    - Rename `seed` -> `recv_iter`, `input` -> `recv_stream` ([`bc27dcf`](https://github.com/hydro-project/hydroflow/commit/bc27dcf82b29fd0cb477e7eb4fc34aa99e0ba9c6))
    - Make parenthesis optional in surface syntax ([`e528c5f`](https://github.com/hydro-project/hydroflow/commit/e528c5f88bddfe7616d1dd62f0a3de8116cf7b45))
    - Remove automatic index incrementing ([`5f5242f`](https://github.com/hydro-project/hydroflow/commit/5f5242f7c2fb2f5b482856b32d99e33dbfd9dc58))
    - Output source code row/col in mermaid instead of slotmap ID ([`7797342`](https://github.com/hydro-project/hydroflow/commit/7797342cfaed6c98ab02f6c9e51a8a6e21f8beba))
    - Cleanups, rename `hydroflow_core` to `hydroflow_lang` ([`c8f2b56`](https://github.com/hydro-project/hydroflow/commit/c8f2b56295555c04e8240432ff686d89fccef01c))
</details>

