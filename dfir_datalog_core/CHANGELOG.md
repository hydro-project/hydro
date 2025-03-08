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

### New Features

 - <csr-id-3a3382aade8b9e9500ae8fec70496ba7125e63ad/> add more perf tags to operators that do non-iterator work
   Also unifies all `dest_sink` related work under a common identifier now
   that we pass it to the operator codegen.
 - <csr-id-ee280c972848170783d105ba047b26dfa7249f24/> `loop {` scheduler
   Fix https://github.com/hydro-project/hydro/issues/1580
 - <csr-id-f3c459036976d87b20356a761bdea9c010ae680b/> Add ability to customize operator tag for stack tracing/flamegraphs
   Actually inserting Hydro-level operator IDs/names is TODO
   
   #1479

### Bug Fixes

 - <csr-id-10d8db58f9f2757c93f228f0a7fb45f7719068d0/> include operator tag for pre-iterator work

### Chore (BREAKING)

 - <csr-id-44fb2806cf2d165d86695910f4755e0944c11832/> use DFIR name instead of Hydroflow in some places, fix #1644
   Fix partially #1712
   
   * Renames `WriteContextArgs.hydroflow` to `WriteContextArgs.df_ident`
   for DFIR operator codegen
   * Removes some dead code/files

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release over the course of 28 calendar days.
 - 6 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 6 unique issues were worked on: [#1692](https://github.com/hydro-project/hydro/issues/1692), [#1702](https://github.com/hydro-project/hydro/issues/1702), [#1713](https://github.com/hydro-project/hydro/issues/1713), [#1747](https://github.com/hydro-project/hydro/issues/1747), [#1753](https://github.com/hydro-project/hydro/issues/1753), [#1754](https://github.com/hydro-project/hydro/issues/1754)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1692](https://github.com/hydro-project/hydro/issues/1692)**
    - `loop {` scheduler ([`ee280c9`](https://github.com/hydro-project/hydro/commit/ee280c972848170783d105ba047b26dfa7249f24))
 * **[#1702](https://github.com/hydro-project/hydro/issues/1702)**
    - Add ability to customize operator tag for stack tracing/flamegraphs ([`f3c4590`](https://github.com/hydro-project/hydro/commit/f3c459036976d87b20356a761bdea9c010ae680b))
 * **[#1713](https://github.com/hydro-project/hydro/issues/1713)**
    - Use DFIR name instead of Hydroflow in some places, fix #1644 ([`44fb280`](https://github.com/hydro-project/hydro/commit/44fb2806cf2d165d86695910f4755e0944c11832))
 * **[#1747](https://github.com/hydro-project/hydro/issues/1747)**
    - Upgrade to Rust 2024 edition ([`49a387d`](https://github.com/hydro-project/hydro/commit/49a387d4a21f0763df8ec94de73fb953c9cd333a))
 * **[#1753](https://github.com/hydro-project/hydro/issues/1753)**
    - Include operator tag for pre-iterator work ([`10d8db5`](https://github.com/hydro-project/hydro/commit/10d8db58f9f2757c93f228f0a7fb45f7719068d0))
 * **[#1754](https://github.com/hydro-project/hydro/issues/1754)**
    - Add more perf tags to operators that do non-iterator work ([`3a3382a`](https://github.com/hydro-project/hydro/commit/3a3382aade8b9e9500ae8fec70496ba7125e63ad))
</details>

## 0.11.0 (2024-12-23)

<csr-id-251b1039c71d45d3f86123dba1926026ded80824/>
<csr-id-03b3a349013a71b324276bca5329c33d400a73ff/>
<csr-id-accb13cad718c99d350e4bafe82e0ca38bf94c62/>
<csr-id-3291c07b37c9f9031837a2a32953e8f8854ec298/>

### New Features

 - <csr-id-b8acd843bdbcfb445bf942e697447f6bf58a10da/> initial flo implementation
   Basic first pass implementation, without changing the current scheduler,
   no breaking behavior. Adds the flo syntax (`loop { ... }`), basic graph
   structure checks, nested loop blocks, and two basic windowing operators
   (`batch()` and `all_once()`).
   
   Next steps:
   - Needs scheduler changes:
- Implement un-windowing operators

### Refactor (BREAKING)

 - <csr-id-251b1039c71d45d3f86123dba1926026ded80824/> use `cfg(nightly)` instead of feature, remove `-Z` flag, use `Diagnostic::try_emit`
   Previous PR (#1587) website build did not work because `panic = "abort"`
   is set on wasm, leading to aborts for `proc_macro2::Span::unwrap()`
   calls.
   
   All tests except trybuild seem to pass on stable, WIP #1587 next

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

 - <csr-id-03b3a349013a71b324276bca5329c33d400a73ff/> bump versions manually for renamed crates, per `RELEASING.md`
 - <csr-id-accb13cad718c99d350e4bafe82e0ca38bf94c62/> cleanup snapshots
 - <csr-id-3291c07b37c9f9031837a2a32953e8f8854ec298/> Rename Hydroflow -> DFIR
   Work In Progress:
   - [x] hydroflow_macro
   - [x] hydroflow_datalog_core
   - [x] hydroflow_datalog
   - [x] hydroflow_lang
   - [x] hydroflow

### `hydroflow_datalog_core` Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 38 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#1585](https://github.com/hydro-project/hydroflow/issues/1585), [#1606](https://github.com/hydro-project/hydroflow/issues/1606)

### `hydroflow_datalog_core` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1585](https://github.com/hydro-project/hydroflow/issues/1585)**
    - Initial flo implementation ([`b8acd84`](https://github.com/hydro-project/hydroflow/commit/b8acd843bdbcfb445bf942e697447f6bf58a10da))
 * **[#1606](https://github.com/hydro-project/hydroflow/issues/1606)**
    - Use `cfg(nightly)` instead of feature, remove `-Z` flag, use `Diagnostic::try_emit` ([`251b103`](https://github.com/hydro-project/hydroflow/commit/251b1039c71d45d3f86123dba1926026ded80824))
</details>

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 4 unique issues were worked on: [#1501](https://github.com/hydro-project/hydro/issues/1501), [#1620](https://github.com/hydro-project/hydro/issues/1620), [#1623](https://github.com/hydro-project/hydro/issues/1623), [#1627](https://github.com/hydro-project/hydro/issues/1627)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1501](https://github.com/hydro-project/hydro/issues/1501)**
    - Add `repository` field to `Cargo.toml`s, fix #1452 ([`28cd220`](https://github.com/hydro-project/hydro/commit/28cd220c68e3660d9ebade113949a2346720cd04))
 * **[#1620](https://github.com/hydro-project/hydro/issues/1620)**
    - Rename Hydroflow -> DFIR ([`3291c07`](https://github.com/hydro-project/hydro/commit/3291c07b37c9f9031837a2a32953e8f8854ec298))
 * **[#1623](https://github.com/hydro-project/hydro/issues/1623)**
    - Cleanup snapshots ([`accb13c`](https://github.com/hydro-project/hydro/commit/accb13cad718c99d350e4bafe82e0ca38bf94c62))
 * **[#1627](https://github.com/hydro-project/hydro/issues/1627)**
    - Bump versions manually for renamed crates, per `RELEASING.md` ([`03b3a34`](https://github.com/hydro-project/hydro/commit/03b3a349013a71b324276bca5329c33d400a73ff))
 * **Uncategorized**
    - Release dfir_lang v0.11.0, dfir_datalog_core v0.11.0, dfir_datalog v0.11.0, dfir_macro v0.11.0, hydroflow_deploy_integration v0.11.0, lattices_macro v0.5.8, variadics v0.0.8, variadics_macro v0.5.6, lattices v0.5.9, multiplatform_test v0.4.0, pusherator v0.0.10, dfir_rs v0.11.0, hydro_deploy v0.11.0, stageleft_macro v0.5.0, stageleft v0.6.0, stageleft_tool v0.5.0, hydro_lang v0.11.0, hydro_std v0.11.0, hydro_cli v0.11.0, safety bump 6 crates ([`9a7e486`](https://github.com/hydro-project/hydro/commit/9a7e48693fce0face0f8ad16349258cdbe26395f))
    - Update `CHANGELOG.md`s for big rename ([`e1a08e5`](https://github.com/hydro-project/hydro/commit/e1a08e5d165fbc80da2ae695e507078a97a9031f))
</details>

<csr-unknown>
Implement windowing operator repeat_n() -<csr-unknown/>

## 0.10.0 (2024-11-08)

<csr-id-4f6d400d0992594f8f12992fb6939378b12fadd6/>
<csr-id-d5677604e93c07a5392f4229af94a0b736eca382/>
<csr-id-8442d1b524621a9f8b43372a9c25991efb33c25e/>

### Chore

 - <csr-id-4f6d400d0992594f8f12992fb6939378b12fadd6/> update rust-sitter
   The latest Rust Sitter drops the dependency on `tree-sitter-cli`, which
   eliminates many transitive dependencies.
 - <csr-id-d5677604e93c07a5392f4229af94a0b736eca382/> update pinned rust version, clippy lints, remove some dead code

### Style

 - <csr-id-8442d1b524621a9f8b43372a9c25991efb33c25e/> fixes for latest nightly clippy

### `hydroflow_datalog_core` Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 69 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 3 unique issues were worked on: [#1444](https://github.com/hydro-project/hydroflow/issues/1444), [#1495](https://github.com/hydro-project/hydroflow/issues/1495), [#1537](https://github.com/hydro-project/hydroflow/issues/1537)

### `hydroflow_datalog_core` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1444](https://github.com/hydro-project/hydroflow/issues/1444)**
    - Update pinned rust version, clippy lints, remove some dead code ([`d567760`](https://github.com/hydro-project/hydroflow/commit/d5677604e93c07a5392f4229af94a0b736eca382))
 * **[#1495](https://github.com/hydro-project/hydroflow/issues/1495)**
    - Update rust-sitter ([`4f6d400`](https://github.com/hydro-project/hydroflow/commit/4f6d400d0992594f8f12992fb6939378b12fadd6))
 * **[#1537](https://github.com/hydro-project/hydroflow/issues/1537)**
    - Fixes for latest nightly clippy ([`8442d1b`](https://github.com/hydro-project/hydroflow/commit/8442d1b524621a9f8b43372a9c25991efb33c25e))
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

### Bug Fixes

 - <csr-id-22c72189bb76412955d29b03c5d99894c558a07c/> remove `FlowProps`

### Refactor

 - <csr-id-9c352f50a5a8d1b2187b34d5847a23b7397fe6ec/> cleanup handling of span locations #1268, workaround fix #729

### `hydroflow_datalog_core` Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release.
 - 38 days passed between releases.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 4 unique issues were worked on: [#1417](https://github.com/hydro-project/hydroflow/issues/1417), [#1420](https://github.com/hydro-project/hydroflow/issues/1420), [#1423](https://github.com/hydro-project/hydroflow/issues/1423), [#1432](https://github.com/hydro-project/hydroflow/issues/1432)

### `hydroflow_datalog_core` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1417](https://github.com/hydro-project/hydroflow/issues/1417)**
    - Drop unused dependencies ([`3c41e1e`](https://github.com/hydro-project/hydroflow/commit/3c41e1e9b505bb84474ec10df30fb65c6d6f90cb))
 * **[#1420](https://github.com/hydro-project/hydroflow/issues/1420)**
    - Remove `FlowProps` ([`22c7218`](https://github.com/hydro-project/hydroflow/commit/22c72189bb76412955d29b03c5d99894c558a07c))
 * **[#1423](https://github.com/hydro-project/hydroflow/issues/1423)**
    - Lower min dependency versions where possible, update `Cargo.lock` ([`11af328`](https://github.com/hydro-project/hydroflow/commit/11af32828bab6e4a4264d2635ff71a12bb0bb778))
 * **[#1432](https://github.com/hydro-project/hydroflow/issues/1432)**
    - Cleanup handling of span locations #1268, workaround fix #729 ([`9c352f5`](https://github.com/hydro-project/hydroflow/commit/9c352f50a5a8d1b2187b34d5847a23b7397fe6ec))
 * **Uncategorized**
    - Release hydroflow_lang v0.9.0, hydroflow_datalog_core v0.9.0, hydroflow_datalog v0.9.0, hydroflow_deploy_integration v0.9.0, hydroflow_macro v0.9.0, lattices_macro v0.5.6, lattices v0.5.7, multiplatform_test v0.2.0, variadics v0.0.6, pusherator v0.0.8, hydroflow v0.9.0, stageleft_macro v0.3.0, stageleft v0.4.0, stageleft_tool v0.3.0, hydroflow_plus v0.9.0, hydro_deploy v0.9.0, hydro_cli v0.9.0, hydroflow_plus_deploy v0.9.0, safety bump 8 crates ([`0750117`](https://github.com/hydro-project/hydroflow/commit/0750117de7088c01a439b102adeb4c832889f171))
</details>

## 0.8.0 (2024-07-23)

<csr-id-67c0e51fb25ea1a2e3aae197c1984920b46759fa/>

### New Features

 - <csr-id-b3d01c20cae2335a3da2c02343debe677f17786b/> add `#[derive(Lattice)]` derive macros, fix #1247
   This adds derive macros to allow user-created macros. Each field must be
   a lattice.
   
   Example usage:
   ```rust
   struct MyLattice<KeySet, Epoch>
   where
   KeySet: Collection,
   Epoch: Ord,
   {
   keys: SetUnion<KeySet>,
   epoch: Max<Epoch>,
   }
   ```
   
   Uses `#[derive(Lattice)]` for the `lattices` library `Pair` lattice.
   Also contains some cleanup in the `lattices` crate.

### Bug Fixes

 - <csr-id-f91c30045dfdf92cf3d383676875d9e749cb8d93/> add `add_state_tick` to state API, reset at end of each tick, fix #1298
   Option 2 of #1298
   
   * Main feature in the title is in `src/scheduled/{context,graph}.rs`
* Codegen for some stateful operators (those which can be used as singletons, and some others) is changed to use the new API.
* Add a test `test_cartesian_product_tick_state` for #1298
* Rest of the diff is snapshot changes

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

### `hydroflow_datalog_core` Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release.
 - 59 days passed between releases.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 4 unique issues were worked on: [#1250](https://github.com/hydro-project/hydroflow/issues/1250), [#1295](https://github.com/hydro-project/hydroflow/issues/1295), [#1300](https://github.com/hydro-project/hydroflow/issues/1300), [#1332](https://github.com/hydro-project/hydroflow/issues/1332)

### `hydroflow_datalog_core` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1250](https://github.com/hydro-project/hydroflow/issues/1250)**
    - Add `#[derive(Lattice)]` derive macros, fix #1247 ([`b3d01c2`](https://github.com/hydro-project/hydroflow/commit/b3d01c20cae2335a3da2c02343debe677f17786b))
 * **[#1295](https://github.com/hydro-project/hydroflow/issues/1295)**
    - Require lifetime on `perist*()` operators ([`67c0e51`](https://github.com/hydro-project/hydroflow/commit/67c0e51fb25ea1a2e3aae197c1984920b46759fa))
 * **[#1300](https://github.com/hydro-project/hydroflow/issues/1300)**
    - Add `add_state_tick` to state API, reset at end of each tick, fix #1298 ([`f91c300`](https://github.com/hydro-project/hydroflow/commit/f91c30045dfdf92cf3d383676875d9e749cb8d93))
 * **[#1332](https://github.com/hydro-project/hydroflow/issues/1332)**
    - Remove singleton referencer `persist::<'static>()` insertion ([`755e8a6`](https://github.com/hydro-project/hydroflow/commit/755e8a6d2c2b30b5d28b60315bb099030d3f3964))
 * **Uncategorized**
    - Release hydroflow_lang v0.8.0, hydroflow_datalog_core v0.8.0, hydroflow_datalog v0.8.0, hydroflow_macro v0.8.0, lattices_macro v0.5.5, lattices v0.5.6, variadics v0.0.5, pusherator v0.0.7, hydroflow v0.8.0, hydroflow_plus v0.8.0, hydro_deploy v0.8.0, hydro_cli v0.8.0, hydroflow_plus_cli_integration v0.8.0, safety bump 7 crates ([`ca6c16b`](https://github.com/hydro-project/hydroflow/commit/ca6c16b4a7ce35e155fe7fc6c7d1676c37c9e4de))
</details>

## 0.7.0 (2024-05-24)

<csr-id-b86f11aad344fef6ad9cdd1db0b45bb738c48bd6/>
<csr-id-cbb3e4b3601211baa84d8d0976c5176561f940b8/>
<csr-id-40f1a19ece2a8352e6fdc31f815d923e635f91b1/>

### Chore

 - <csr-id-b86f11aad344fef6ad9cdd1db0b45bb738c48bd6/> expect custom config names to prevent warnings
   See
   https://doc.rust-lang.org/nightly/cargo/reference/build-scripts.html#rustc-check-cfg
 - <csr-id-cbb3e4b3601211baa84d8d0976c5176561f940b8/> update Rust Sitter
   chore(hydroflow_datalog): update Rust Sitter

### Bug Fixes

 - <csr-id-d57b91146ef44125f1dd87e040ef636797f90e76/> cleanup codegen for `fold_keyed`

### Refactor

 - <csr-id-40f1a19ece2a8352e6fdc31f815d923e635f91b1/> remove reference edges, reverts #1017 #1041

### New Features (BREAKING)

 - <csr-id-c9dc66db2bde34aa1385c8847d0ba7c16472f75d/> add detupling syntax and allow interleaving with flattening

### `hydroflow_datalog_core` Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release.
 - 48 days passed between releases.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 5 unique issues were worked on: [#1160](https://github.com/hydro-project/hydroflow/issues/1160), [#1166](https://github.com/hydro-project/hydroflow/issues/1166), [#1168](https://github.com/hydro-project/hydroflow/issues/1168), [#1176](https://github.com/hydro-project/hydroflow/issues/1176), [#1192](https://github.com/hydro-project/hydroflow/issues/1192)

### `hydroflow_datalog_core` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1160](https://github.com/hydro-project/hydroflow/issues/1160)**
    - Remove reference edges, reverts #1017 #1041 ([`40f1a19`](https://github.com/hydro-project/hydroflow/commit/40f1a19ece2a8352e6fdc31f815d923e635f91b1))
 * **[#1166](https://github.com/hydro-project/hydroflow/issues/1166)**
    - Add detupling syntax and allow interleaving with flattening ([`c9dc66d`](https://github.com/hydro-project/hydroflow/commit/c9dc66db2bde34aa1385c8847d0ba7c16472f75d))
 * **[#1168](https://github.com/hydro-project/hydroflow/issues/1168)**
    - Update Rust Sitter ([`cbb3e4b`](https://github.com/hydro-project/hydroflow/commit/cbb3e4b3601211baa84d8d0976c5176561f940b8))
 * **[#1176](https://github.com/hydro-project/hydroflow/issues/1176)**
    - Cleanup codegen for `fold_keyed` ([`d57b911`](https://github.com/hydro-project/hydroflow/commit/d57b91146ef44125f1dd87e040ef636797f90e76))
 * **[#1192](https://github.com/hydro-project/hydroflow/issues/1192)**
    - Expect custom config names to prevent warnings ([`b86f11a`](https://github.com/hydro-project/hydroflow/commit/b86f11aad344fef6ad9cdd1db0b45bb738c48bd6))
 * **Uncategorized**
    - Release hydroflow_lang v0.7.0, hydroflow_datalog_core v0.7.0, hydroflow_datalog v0.7.0, hydroflow_macro v0.7.0, lattices v0.5.5, multiplatform_test v0.1.0, pusherator v0.0.6, hydroflow v0.7.0, stageleft_macro v0.2.0, stageleft v0.3.0, stageleft_tool v0.2.0, hydroflow_plus v0.7.0, hydro_deploy v0.7.0, hydro_cli v0.7.0, hydroflow_plus_cli_integration v0.7.0, safety bump 8 crates ([`2852147`](https://github.com/hydro-project/hydroflow/commit/285214740627685e911781793e05d234ab2ad2bd))
</details>

## 0.6.1 (2024-04-05)

<csr-id-88e2ca44fed2d54b969e54a948c41d3bcd9a2ab5/>

### New Features

 - <csr-id-f9b26b6df7ff7d2a3530527579c60f41d93e9d45/> switch singletons references to being per-op intead of per-edge
   Only testing in `filter` op for now
 - <csr-id-0a5e785d4bf0729ba0fe95466c822e8304f96e24/> initial proof-of-concept for singletons
 - <csr-id-602e21fc04da5a0d65ac7128688760c92ffba3c1/> include subgraph ID in pivot_run codegen
 - <csr-id-916a366cba94c739a6e170d281663f19f706d6be/> add syntax for "splatting" columns
   feat(hydroflow_datalog): add syntax for "splatting" columns
   
   The * operator can be used on the left-hand side of a rule to indicate
   that each row should be duplicated into many rows with that column being
   replaced by every value in iterable corresponding to the original
   expression.
 - <csr-id-61cb55a04e518560beff1ea5d446927b6d7096b3/> add aggregation for collecting values into a vector
   feat(hydroflow_datalog): add aggregation for collecting values into a
   vector

### Bug Fixes

 - <csr-id-e395ad5bc22bc94cc9b85363367a306f1589efde/> leftover comments
   fix(hydroflow_datalog): leftover comments
   
   Merged too soon.
 - <csr-id-f8311db2cd9628607887fc04f2ea5933c8b7c11e/> #1050 for `fold_keyed`

### Refactor

 - <csr-id-88e2ca44fed2d54b969e54a948c41d3bcd9a2ab5/> add `#[repr(transparent)]` to type guard codegen
   theoretically could help performance but is probably negligible

### `hydroflow_datalog_core` Commit Statistics

<csr-read-only-do-not-edit/>

 - 9 commits contributed to the release.
 - 34 days passed between releases.
 - 8 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 7 unique issues were worked on: [#1086](https://github.com/hydro-project/hydroflow/issues/1086), [#1091](https://github.com/hydro-project/hydroflow/issues/1091), [#1094](https://github.com/hydro-project/hydroflow/issues/1094), [#1132](https://github.com/hydro-project/hydroflow/issues/1132), [#1135](https://github.com/hydro-project/hydroflow/issues/1135), [#1136](https://github.com/hydro-project/hydroflow/issues/1136), [#1137](https://github.com/hydro-project/hydroflow/issues/1137)

### `hydroflow_datalog_core` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1086](https://github.com/hydro-project/hydroflow/issues/1086)**
    - #1050 for `fold_keyed` ([`f8311db`](https://github.com/hydro-project/hydroflow/commit/f8311db2cd9628607887fc04f2ea5933c8b7c11e))
 * **[#1091](https://github.com/hydro-project/hydroflow/issues/1091)**
    - Switch singletons references to being per-op intead of per-edge ([`f9b26b6`](https://github.com/hydro-project/hydroflow/commit/f9b26b6df7ff7d2a3530527579c60f41d93e9d45))
    - Initial proof-of-concept for singletons ([`0a5e785`](https://github.com/hydro-project/hydroflow/commit/0a5e785d4bf0729ba0fe95466c822e8304f96e24))
 * **[#1094](https://github.com/hydro-project/hydroflow/issues/1094)**
    - Add aggregation for collecting values into a vector ([`61cb55a`](https://github.com/hydro-project/hydroflow/commit/61cb55a04e518560beff1ea5d446927b6d7096b3))
 * **[#1132](https://github.com/hydro-project/hydroflow/issues/1132)**
    - Add syntax for "splatting" columns ([`916a366`](https://github.com/hydro-project/hydroflow/commit/916a366cba94c739a6e170d281663f19f706d6be))
 * **[#1135](https://github.com/hydro-project/hydroflow/issues/1135)**
    - Leftover comments ([`e395ad5`](https://github.com/hydro-project/hydroflow/commit/e395ad5bc22bc94cc9b85363367a306f1589efde))
 * **[#1136](https://github.com/hydro-project/hydroflow/issues/1136)**
    - Add `#[repr(transparent)]` to type guard codegen ([`88e2ca4`](https://github.com/hydro-project/hydroflow/commit/88e2ca44fed2d54b969e54a948c41d3bcd9a2ab5))
 * **[#1137](https://github.com/hydro-project/hydroflow/issues/1137)**
    - Include subgraph ID in pivot_run codegen ([`602e21f`](https://github.com/hydro-project/hydroflow/commit/602e21fc04da5a0d65ac7128688760c92ffba3c1))
 * **Uncategorized**
    - Release hydroflow_cli_integration v0.5.2, hydroflow_lang v0.6.1, hydroflow_datalog_core v0.6.1, lattices v0.5.4, hydroflow v0.6.1, stageleft_macro v0.1.1, stageleft v0.2.1, hydroflow_plus v0.6.1, hydro_deploy v0.6.1, hydro_cli v0.6.1, hydroflow_plus_cli_integration v0.6.1, stageleft_tool v0.1.1 ([`cd63f22`](https://github.com/hydro-project/hydroflow/commit/cd63f2258c961a40f0e5dbef20ac329a2d570ad0))
</details>

## 0.6.0 (2024-03-02)

<csr-id-b391447ec13f1f79c99142f296dc2fa8640034f4/>

### Style

 - <csr-id-b391447ec13f1f79c99142f296dc2fa8640034f4/> fix imports

### `hydroflow_datalog_core` Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 28 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### `hydroflow_datalog_core` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release hydroflow_lang v0.6.0, hydroflow_datalog_core v0.6.0, hydroflow_datalog v0.6.0, hydroflow_macro v0.6.0, lattices v0.5.3, variadics v0.0.4, pusherator v0.0.5, hydroflow v0.6.0, stageleft v0.2.0, hydroflow_plus v0.6.0, hydro_deploy v0.6.0, hydro_cli v0.6.0, hydroflow_plus_cli_integration v0.6.0, safety bump 7 crates ([`09ea65f`](https://github.com/hydro-project/hydroflow/commit/09ea65fe9cd45c357c43bffca30e60243fa45cc8))
    - Fix imports ([`b391447`](https://github.com/hydro-project/hydroflow/commit/b391447ec13f1f79c99142f296dc2fa8640034f4))
</details>

## 0.5.2 (2024-02-02)

<csr-id-d4d786269e44cdf938580b8fb7abf888952eca0e/>

### Chore

 - <csr-id-d4d786269e44cdf938580b8fb7abf888952eca0e/> update snapshots for
   (#929)

### Bug Fixes

 - <csr-id-175cc20e15b90627ae86d488e31ec91278c8beeb/> ensure other graph methods handle edge types
   (#929)

### `hydroflow_datalog_core` Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 4 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1041](https://github.com/hydro-project/hydroflow/issues/1041)

### `hydroflow_datalog_core` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1041](https://github.com/hydro-project/hydroflow/issues/1041)**
    - Update snapshots for ([`d4d7862`](https://github.com/hydro-project/hydroflow/commit/d4d786269e44cdf938580b8fb7abf888952eca0e))
    - Ensure other graph methods handle edge types ([`175cc20`](https://github.com/hydro-project/hydroflow/commit/175cc20e15b90627ae86d488e31ec91278c8beeb))
 * **Uncategorized**
    - Release hydroflow_lang v0.5.2, hydroflow_datalog_core v0.5.2, hydroflow_macro v0.5.2, lattices v0.5.2, hydroflow v0.5.2, hydro_cli v0.5.1, hydroflow_plus_cli_integration v0.5.1 ([`6ac8720`](https://github.com/hydro-project/hydroflow/commit/6ac872081753548ebb8ec95549b4d820dc050d3e))
</details>

## 0.5.1 (2024-01-29)

<csr-id-1b555e57c8c812bed4d6495d2960cbf77fb0b3ef/>
<csr-id-1a80f1cd57e6f3a5ee806e1bf3b8ad59dcecfff7/>

### Chore

 - <csr-id-1b555e57c8c812bed4d6495d2960cbf77fb0b3ef/> manually set lockstep-versioned crates (and `lattices`) to version `0.5.1`
   Setting manually since
   https://github.com/frewsxcv/rust-crates-index/issues/159 is messing with
   smart-release

### New Features

 - <csr-id-73e9b68ec2f5b2627784addcce9fba684848bb55/> implement keyed fold and reduce
 - <csr-id-a0af314a032096fc94b9f4aabb21aadc8184fb30/> Add initial structure for by-reference edge types
 - <csr-id-7df0a0df61597764eed763b68138929fed1413ac/> add defer() which is the same as defer_tick() except that it is lazy

### Bug Fixes

 - <csr-id-38411ea007d4feb30dd16bdd1505802a111a67d1/> fix spelling of "propagate"

### Refactor

 - <csr-id-1a80f1cd57e6f3a5ee806e1bf3b8ad59dcecfff7/> emit prologue code before all subgraph code
   Before, prologue code would be emitted before its subgraph, resulting in
   interleaving between subgraphs.

### `hydroflow_datalog_core` Commit Statistics

<csr-read-only-do-not-edit/>

 - 7 commits contributed to the release.
 - 110 days passed between releases.
 - 6 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 5 unique issues were worked on: [#1016](https://github.com/hydro-project/hydroflow/issues/1016), [#1023](https://github.com/hydro-project/hydroflow/issues/1023), [#1033](https://github.com/hydro-project/hydroflow/issues/1033), [#945](https://github.com/hydro-project/hydroflow/issues/945), [#989](https://github.com/hydro-project/hydroflow/issues/989)

### `hydroflow_datalog_core` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1016](https://github.com/hydro-project/hydroflow/issues/1016)**
    - Add initial structure for by-reference edge types ([`a0af314`](https://github.com/hydro-project/hydroflow/commit/a0af314a032096fc94b9f4aabb21aadc8184fb30))
 * **[#1023](https://github.com/hydro-project/hydroflow/issues/1023)**
    - Implement keyed fold and reduce ([`73e9b68`](https://github.com/hydro-project/hydroflow/commit/73e9b68ec2f5b2627784addcce9fba684848bb55))
 * **[#1033](https://github.com/hydro-project/hydroflow/issues/1033)**
    - Emit prologue code before all subgraph code ([`1a80f1c`](https://github.com/hydro-project/hydroflow/commit/1a80f1cd57e6f3a5ee806e1bf3b8ad59dcecfff7))
 * **[#945](https://github.com/hydro-project/hydroflow/issues/945)**
    - Add defer() which is the same as defer_tick() except that it is lazy ([`7df0a0d`](https://github.com/hydro-project/hydroflow/commit/7df0a0df61597764eed763b68138929fed1413ac))
 * **[#989](https://github.com/hydro-project/hydroflow/issues/989)**
    - Fix spelling of "propagate" ([`38411ea`](https://github.com/hydro-project/hydroflow/commit/38411ea007d4feb30dd16bdd1505802a111a67d1))
 * **Uncategorized**
    - Release hydroflow_cli_integration v0.5.1, hydroflow_lang v0.5.1, hydroflow_datalog_core v0.5.1, hydroflow_datalog v0.5.1, hydroflow_macro v0.5.1, lattices v0.5.1, variadics v0.0.3, pusherator v0.0.4, hydroflow v0.5.1, stageleft_macro v0.1.0, stageleft v0.1.0, hydroflow_plus v0.5.1, hydro_deploy v0.5.1, hydro_cli v0.5.1 ([`478aebc`](https://github.com/hydro-project/hydroflow/commit/478aebc8fee2aa78eab86bd386322db1c70bde6a))
    - Manually set lockstep-versioned crates (and `lattices`) to version `0.5.1` ([`1b555e5`](https://github.com/hydro-project/hydroflow/commit/1b555e57c8c812bed4d6495d2960cbf77fb0b3ef))
</details>

## 0.5.0 (2023-10-11)

<csr-id-f19eccc79d6d7c88de7ba1ef6a0abf1caaef377f/>
<csr-id-1fb753ea85511ade1a834ec2536f56358ade9858/>
<csr-id-7c7eea7fddda7ea9526c5af4191520e821c979dc/>
<csr-id-9144dd96b915e1b807ef14f40d963cdbd47e9078/>

### Chore

 - <csr-id-f19eccc79d6d7c88de7ba1ef6a0abf1caaef377f/> bump proc-macro2 min version to 1.0.63
 - <csr-id-1fb753ea85511ade1a834ec2536f56358ade9858/> ignore `clippy::unwrap_or_default` in `fold_keyed` codegen
 - <csr-id-7c7eea7fddda7ea9526c5af4191520e821c979dc/> Replace `or_insert_with(Vec::new)` with `or_default()`
   Clippy lint `unwrap_or_default` complaining on latest nightly

### New Features

 - <csr-id-21140f09156e1dad195162854955522f138ae781/> update snapshot tests for previous two commits
 - <csr-id-d254e2deb883f9633f8b325a595fb7c61bad42d7/> add context.is_first_time_subgraph_is_scheduled to simplify replaying operators
 - <csr-id-f013c3ca15f2cc9413fcfb92898f71d5fc00073a/> add import!() expression
 - <csr-id-1bdbf73b630e4f2eff009b00b0e66d71be53bb4a/> Implement `flow_prop_fn` for `union()`
 - <csr-id-fd89cb46c5983d277e16bb7b19f7d3ca83dd60cc/> Make `propegate_flow_props` fallible, cleanup `flow_prop_fn` definition.
 - <csr-id-7714403e130969b96c8f405444d4daf451450fdf/> Add `monotonic_fn` and `morphism` macros, update snapshots for flow props.

### Test

 - <csr-id-9144dd96b915e1b807ef14f40d963cdbd47e9078/> update snapshots

### `hydroflow_datalog_core` Commit Statistics

<csr-read-only-do-not-edit/>

 - 11 commits contributed to the release.
 - 56 days passed between releases.
 - 10 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 7 unique issues were worked on: [#882](https://github.com/hydro-project/hydroflow/issues/882), [#893](https://github.com/hydro-project/hydroflow/issues/893), [#896](https://github.com/hydro-project/hydroflow/issues/896), [#898](https://github.com/hydro-project/hydroflow/issues/898), [#906](https://github.com/hydro-project/hydroflow/issues/906), [#924](https://github.com/hydro-project/hydroflow/issues/924), [#926](https://github.com/hydro-project/hydroflow/issues/926)

### `hydroflow_datalog_core` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#882](https://github.com/hydro-project/hydroflow/issues/882)**
    - Make `propegate_flow_props` fallible, cleanup `flow_prop_fn` definition. ([`fd89cb4`](https://github.com/hydro-project/hydroflow/commit/fd89cb46c5983d277e16bb7b19f7d3ca83dd60cc))
    - Add `monotonic_fn` and `morphism` macros, update snapshots for flow props. ([`7714403`](https://github.com/hydro-project/hydroflow/commit/7714403e130969b96c8f405444d4daf451450fdf))
 * **[#893](https://github.com/hydro-project/hydroflow/issues/893)**
    - Replace `or_insert_with(Vec::new)` with `or_default()` ([`7c7eea7`](https://github.com/hydro-project/hydroflow/commit/7c7eea7fddda7ea9526c5af4191520e821c979dc))
 * **[#896](https://github.com/hydro-project/hydroflow/issues/896)**
    - Ignore `clippy::unwrap_or_default` in `fold_keyed` codegen ([`1fb753e`](https://github.com/hydro-project/hydroflow/commit/1fb753ea85511ade1a834ec2536f56358ade9858))
 * **[#898](https://github.com/hydro-project/hydroflow/issues/898)**
    - Add import!() expression ([`f013c3c`](https://github.com/hydro-project/hydroflow/commit/f013c3ca15f2cc9413fcfb92898f71d5fc00073a))
 * **[#906](https://github.com/hydro-project/hydroflow/issues/906)**
    - Add context.is_first_time_subgraph_is_scheduled to simplify replaying operators ([`d254e2d`](https://github.com/hydro-project/hydroflow/commit/d254e2deb883f9633f8b325a595fb7c61bad42d7))
 * **[#924](https://github.com/hydro-project/hydroflow/issues/924)**
    - Update snapshot tests for previous two commits ([`21140f0`](https://github.com/hydro-project/hydroflow/commit/21140f09156e1dad195162854955522f138ae781))
 * **[#926](https://github.com/hydro-project/hydroflow/issues/926)**
    - Update snapshots ([`9144dd9`](https://github.com/hydro-project/hydroflow/commit/9144dd96b915e1b807ef14f40d963cdbd47e9078))
 * **Uncategorized**
    - Release hydroflow_lang v0.5.0, hydroflow_datalog_core v0.5.0, hydroflow_datalog v0.5.0, hydroflow_macro v0.5.0, lattices v0.5.0, hydroflow v0.5.0, hydro_cli v0.5.0, safety bump 4 crates ([`2e2d8b3`](https://github.com/hydro-project/hydroflow/commit/2e2d8b386fb086c8276a2853d2a1f96ad4d7c221))
    - Bump proc-macro2 min version to 1.0.63 ([`f19eccc`](https://github.com/hydro-project/hydroflow/commit/f19eccc79d6d7c88de7ba1ef6a0abf1caaef377f))
    - Implement `flow_prop_fn` for `union()` ([`1bdbf73`](https://github.com/hydro-project/hydroflow/commit/1bdbf73b630e4f2eff009b00b0e66d71be53bb4a))
</details>

## 0.4.0 (2023-08-15)

<csr-id-6a2ad6b770c2ccf470548320d8753025b3a66c0a/>

### New Features

 - <csr-id-b4b9644a19e8e7e7725c9c5b88e3a6b8c2be7364/> Add `use` statements to hydroflow syntax
   And use in doc tests.

### Bug Fixes

<csr-id-cc959c762c3a0e036e672801c615028cbfb95168/>
<csr-id-6c98bbc2bd3443fe6f77e0b8689b461edde1b316/>

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

### Refactor

 - <csr-id-6a2ad6b770c2ccf470548320d8753025b3a66c0a/> fix new clippy lints on latest nightly 1.73.0-nightly (db7ff98a7 2023-07-31)

### `hydroflow_datalog_core` Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release.
 - 42 days passed between releases.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 4 unique issues were worked on: [#833](https://github.com/hydro-project/hydroflow/issues/833), [#845](https://github.com/hydro-project/hydroflow/issues/845), [#870](https://github.com/hydro-project/hydroflow/issues/870), [#872](https://github.com/hydro-project/hydroflow/issues/872)

### `hydroflow_datalog_core` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#833](https://github.com/hydro-project/hydroflow/issues/833)**
    - Rename next_tick -> defer, batch -> defer_signal ([`6c98bbc`](https://github.com/hydro-project/hydroflow/commit/6c98bbc2bd3443fe6f77e0b8689b461edde1b316))
 * **[#845](https://github.com/hydro-project/hydroflow/issues/845)**
    - Add `use` statements to hydroflow syntax ([`b4b9644`](https://github.com/hydro-project/hydroflow/commit/b4b9644a19e8e7e7725c9c5b88e3a6b8c2be7364))
 * **[#870](https://github.com/hydro-project/hydroflow/issues/870)**
    - Joins now replay correctly ([`cc959c7`](https://github.com/hydro-project/hydroflow/commit/cc959c762c3a0e036e672801c615028cbfb95168))
 * **[#872](https://github.com/hydro-project/hydroflow/issues/872)**
    - Unify antijoin and difference with set and multiset semantics ([`d378e5e`](https://github.com/hydro-project/hydroflow/commit/d378e5eada3d2bae90f98c5a33b2d055940a8c7f))
 * **Uncategorized**
    - Release hydroflow_lang v0.4.0, hydroflow_datalog_core v0.4.0, hydroflow_datalog v0.4.0, hydroflow_macro v0.4.0, lattices v0.4.0, pusherator v0.0.3, hydroflow v0.4.0, hydro_cli v0.4.0, safety bump 4 crates ([`cb313f0`](https://github.com/hydro-project/hydroflow/commit/cb313f0635214460a8308d05cbef4bf7f4bfaa15))
    - Fix new clippy lints on latest nightly 1.73.0-nightly (db7ff98a7 2023-07-31) ([`6a2ad6b`](https://github.com/hydro-project/hydroflow/commit/6a2ad6b770c2ccf470548320d8753025b3a66c0a))
</details>

## 0.3.0 (2023-07-04)

### New Features

 - <csr-id-22abcaff806c7de6e4a7725656bbcf201e7d9259/> allow stable build, refactors behind `nightly` feature flag

### Bug Fixes

 - <csr-id-8d3494b5afee858114a602a3e23077bb6d24dd77/> update proc-macro2, use new span location API where possible
   requires latest* rust nightly version
   
   *latest = 2023-06-28 or something

### `hydroflow_datalog_core` Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 33 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#780](https://github.com/hydro-project/hydroflow/issues/780), [#801](https://github.com/hydro-project/hydroflow/issues/801)

### `hydroflow_datalog_core` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#780](https://github.com/hydro-project/hydroflow/issues/780)**
    - Allow stable build, refactors behind `nightly` feature flag ([`22abcaf`](https://github.com/hydro-project/hydroflow/commit/22abcaff806c7de6e4a7725656bbcf201e7d9259))
 * **[#801](https://github.com/hydro-project/hydroflow/issues/801)**
    - Update proc-macro2, use new span location API where possible ([`8d3494b`](https://github.com/hydro-project/hydroflow/commit/8d3494b5afee858114a602a3e23077bb6d24dd77))
 * **Uncategorized**
    - Release hydroflow_cli_integration v0.3.0, hydroflow_lang v0.3.0, hydroflow_datalog_core v0.3.0, hydroflow_datalog v0.3.0, hydroflow_macro v0.3.0, lattices v0.3.0, pusherator v0.0.2, hydroflow v0.3.0, hydro_cli v0.3.0, safety bump 5 crates ([`ec9633e`](https://github.com/hydro-project/hydroflow/commit/ec9633e2e393c2bf106223abeb0b680200fbdf84))
</details>

## 0.2.0 (2023-05-31)

<csr-id-fd896fbe925fbd8ef1d16be7206ac20ba585081a/>

### Chore

 - <csr-id-fd896fbe925fbd8ef1d16be7206ac20ba585081a/> manually bump versions for v0.2.0 release

### `hydroflow_datalog_core` Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 1 day passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### `hydroflow_datalog_core` Commit Details

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

### `hydroflow_datalog_core` Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release.
 - 6 days passed between releases.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 4 unique issues were worked on: [#697](https://github.com/hydro-project/hydroflow/issues/697), [#702](https://github.com/hydro-project/hydroflow/issues/702), [#714](https://github.com/hydro-project/hydroflow/issues/714), [#716](https://github.com/hydro-project/hydroflow/issues/716)

### `hydroflow_datalog_core` Commit Details

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

### `hydroflow_datalog_core` Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 2 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 3 unique issues were worked on: [#673](https://github.com/hydro-project/hydroflow/issues/673), [#677](https://github.com/hydro-project/hydroflow/issues/677), [#684](https://github.com/hydro-project/hydroflow/issues/684)

### `hydroflow_datalog_core` Commit Details

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

### `hydroflow_datalog_core` Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release.
 - 25 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 3 unique issues were worked on: [#639](https://github.com/hydro-project/hydroflow/issues/639), [#642](https://github.com/hydro-project/hydroflow/issues/642), [#660](https://github.com/hydro-project/hydroflow/issues/660)

### `hydroflow_datalog_core` Commit Details

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

## 0.0.0 (2023-04-25)

### `hydroflow_datalog_core` Commit Statistics

<csr-read-only-do-not-edit/>

 - 67 commits contributed to the release.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 59 unique issues were worked on: [#209](https://github.com/hydro-project/hydroflow/issues/209), [#254](https://github.com/hydro-project/hydroflow/issues/254), [#329](https://github.com/hydro-project/hydroflow/issues/329), [#371](https://github.com/hydro-project/hydroflow/issues/371), [#374](https://github.com/hydro-project/hydroflow/issues/374), [#376](https://github.com/hydro-project/hydroflow/issues/376), [#383](https://github.com/hydro-project/hydroflow/issues/383), [#388](https://github.com/hydro-project/hydroflow/issues/388), [#403](https://github.com/hydro-project/hydroflow/issues/403), [#413](https://github.com/hydro-project/hydroflow/issues/413), [#419](https://github.com/hydro-project/hydroflow/issues/419), [#425](https://github.com/hydro-project/hydroflow/issues/425), [#431](https://github.com/hydro-project/hydroflow/issues/431), [#441 11/14](https://github.com/hydro-project/hydroflow/issues/441 11/14), [#441 12/14](https://github.com/hydro-project/hydroflow/issues/441 12/14), [#441 14/14](https://github.com/hydro-project/hydroflow/issues/441 14/14), [#442](https://github.com/hydro-project/hydroflow/issues/442), [#443](https://github.com/hydro-project/hydroflow/issues/443), [#444](https://github.com/hydro-project/hydroflow/issues/444), [#455](https://github.com/hydro-project/hydroflow/issues/455), [#459](https://github.com/hydro-project/hydroflow/issues/459), [#467](https://github.com/hydro-project/hydroflow/issues/467), [#469](https://github.com/hydro-project/hydroflow/issues/469), [#470](https://github.com/hydro-project/hydroflow/issues/470), [#475](https://github.com/hydro-project/hydroflow/issues/475), [#486](https://github.com/hydro-project/hydroflow/issues/486), [#499](https://github.com/hydro-project/hydroflow/issues/499), [#500](https://github.com/hydro-project/hydroflow/issues/500), [#501](https://github.com/hydro-project/hydroflow/issues/501), [#502](https://github.com/hydro-project/hydroflow/issues/502), [#505](https://github.com/hydro-project/hydroflow/issues/505), [#507](https://github.com/hydro-project/hydroflow/issues/507), [#509](https://github.com/hydro-project/hydroflow/issues/509), [#516](https://github.com/hydro-project/hydroflow/issues/516), [#518](https://github.com/hydro-project/hydroflow/issues/518), [#522](https://github.com/hydro-project/hydroflow/issues/522), [#540](https://github.com/hydro-project/hydroflow/issues/540), [#543](https://github.com/hydro-project/hydroflow/issues/543), [#547](https://github.com/hydro-project/hydroflow/issues/547), [#549](https://github.com/hydro-project/hydroflow/issues/549), [#551](https://github.com/hydro-project/hydroflow/issues/551), [#554](https://github.com/hydro-project/hydroflow/issues/554), [#555](https://github.com/hydro-project/hydroflow/issues/555), [#556](https://github.com/hydro-project/hydroflow/issues/556), [#558](https://github.com/hydro-project/hydroflow/issues/558), [#559](https://github.com/hydro-project/hydroflow/issues/559), [#565](https://github.com/hydro-project/hydroflow/issues/565), [#566](https://github.com/hydro-project/hydroflow/issues/566), [#567](https://github.com/hydro-project/hydroflow/issues/567), [#568](https://github.com/hydro-project/hydroflow/issues/568), [#571](https://github.com/hydro-project/hydroflow/issues/571), [#572](https://github.com/hydro-project/hydroflow/issues/572), [#573](https://github.com/hydro-project/hydroflow/issues/573), [#576](https://github.com/hydro-project/hydroflow/issues/576), [#598](https://github.com/hydro-project/hydroflow/issues/598), [#604](https://github.com/hydro-project/hydroflow/issues/604), [#609](https://github.com/hydro-project/hydroflow/issues/609), [#617](https://github.com/hydro-project/hydroflow/issues/617), [#618](https://github.com/hydro-project/hydroflow/issues/618)

### `hydroflow_datalog_core` Commit Details

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

