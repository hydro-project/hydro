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
 - <csr-id-2fd6aa7417dfa29f389c04c5b9674b80bfed6cf2/> update pinned nightly to 2025-02-10, cleanups for clippy

### Style

 - <csr-id-39a2963518a9cc63c7e60a5c542cfa2509064a0c/> fix small format issue
   after upgrading to edition 2024
 - <csr-id-c1983308743d912e5bf2583b7cccbb47d8a8b5d1/> clippy cleanups for latest stable rust

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release over the course of 52 calendar days.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 4 unique issues were worked on: [#1640](https://github.com/hydro-project/hydro/issues/1640), [#1704](https://github.com/hydro-project/hydro/issues/1704), [#1747](https://github.com/hydro-project/hydro/issues/1747), [#1749](https://github.com/hydro-project/hydro/issues/1749)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1640](https://github.com/hydro-project/hydro/issues/1640)**
    - Clippy cleanups for latest stable rust ([`c198330`](https://github.com/hydro-project/hydro/commit/c1983308743d912e5bf2583b7cccbb47d8a8b5d1))
 * **[#1704](https://github.com/hydro-project/hydro/issues/1704)**
    - Update pinned nightly to 2025-02-10, cleanups for clippy ([`2fd6aa7`](https://github.com/hydro-project/hydro/commit/2fd6aa7417dfa29f389c04c5b9674b80bfed6cf2))
 * **[#1747](https://github.com/hydro-project/hydro/issues/1747)**
    - Upgrade to Rust 2024 edition ([`49a387d`](https://github.com/hydro-project/hydro/commit/49a387d4a21f0763df8ec94de73fb953c9cd333a))
 * **[#1749](https://github.com/hydro-project/hydro/issues/1749)**
    - Fix small format issue ([`39a2963`](https://github.com/hydro-project/hydro/commit/39a2963518a9cc63c7e60a5c542cfa2509064a0c))
</details>

## 0.5.9 (2024-12-23)

<csr-id-3291c07b37c9f9031837a2a32953e8f8854ec298/>

### Chore

 - <csr-id-3291c07b37c9f9031837a2a32953e8f8854ec298/> Rename Hydroflow -> DFIR
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
 - <csr-id-6ab625273d822812e83a333e928c3dea1c3c9ccb/> cleanups for the rename, fixing links

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 45 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 3 unique issues were worked on: [#1501](https://github.com/hydro-project/hydro/issues/1501), [#1620](https://github.com/hydro-project/hydro/issues/1620), [#1624](https://github.com/hydro-project/hydro/issues/1624)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1501](https://github.com/hydro-project/hydro/issues/1501)**
    - Add `repository` field to `Cargo.toml`s, fix #1452 ([`28cd220`](https://github.com/hydro-project/hydro/commit/28cd220c68e3660d9ebade113949a2346720cd04))
 * **[#1620](https://github.com/hydro-project/hydro/issues/1620)**
    - Rename Hydroflow -> DFIR ([`3291c07`](https://github.com/hydro-project/hydro/commit/3291c07b37c9f9031837a2a32953e8f8854ec298))
 * **[#1624](https://github.com/hydro-project/hydro/issues/1624)**
    - Cleanups for the rename, fixing links ([`6ab6252`](https://github.com/hydro-project/hydro/commit/6ab625273d822812e83a333e928c3dea1c3c9ccb))
 * **Uncategorized**
    - Release dfir_lang v0.11.0, dfir_datalog_core v0.11.0, dfir_datalog v0.11.0, dfir_macro v0.11.0, hydroflow_deploy_integration v0.11.0, lattices_macro v0.5.8, variadics v0.0.8, variadics_macro v0.5.6, lattices v0.5.9, multiplatform_test v0.4.0, pusherator v0.0.10, dfir_rs v0.11.0, hydro_deploy v0.11.0, stageleft_macro v0.5.0, stageleft v0.6.0, stageleft_tool v0.5.0, hydro_lang v0.11.0, hydro_std v0.11.0, hydro_cli v0.11.0, safety bump 6 crates ([`9a7e486`](https://github.com/hydro-project/hydro/commit/9a7e48693fce0face0f8ad16349258cdbe26395f))
</details>

## 0.5.8 (2024-11-08)

<csr-id-d5677604e93c07a5392f4229af94a0b736eca382/>
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

### Style

 - <csr-id-47cb703e771f7d1c451ceb9d185ada96410949da/> fixes for nightly clippy
   a couple few spurious `too_many_arguments` and a spurious
   `zombie_processes` still on current nightly (`clippy 0.1.84 (4392847410
   2024-10-21)`)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 69 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 3 unique issues were worked on: [#1444](https://github.com/hydro-project/hydro/issues/1444), [#1503](https://github.com/hydro-project/hydro/issues/1503), [#1505](https://github.com/hydro-project/hydro/issues/1505)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1444](https://github.com/hydro-project/hydro/issues/1444)**
    - Update pinned rust version, clippy lints, remove some dead code ([`d567760`](https://github.com/hydro-project/hydro/commit/d5677604e93c07a5392f4229af94a0b736eca382))
 * **[#1503](https://github.com/hydro-project/hydro/issues/1503)**
    - Generalized hash trie indexes for relational tuples ([`f7e740f`](https://github.com/hydro-project/hydro/commit/f7e740fb2ba36d0fcf3fd196d60333552911e3a4))
 * **[#1505](https://github.com/hydro-project/hydro/issues/1505)**
    - Fixes for nightly clippy ([`47cb703`](https://github.com/hydro-project/hydro/commit/47cb703e771f7d1c451ceb9d185ada96410949da))
 * **Uncategorized**
    - Release hydroflow_lang v0.10.0, hydroflow_datalog_core v0.10.0, hydroflow_datalog v0.10.0, hydroflow_deploy_integration v0.10.0, hydroflow_macro v0.10.0, lattices_macro v0.5.7, variadics v0.0.7, variadics_macro v0.5.5, lattices v0.5.8, multiplatform_test v0.3.0, pusherator v0.0.9, hydroflow v0.10.0, hydro_deploy v0.10.0, stageleft_macro v0.4.0, stageleft v0.5.0, stageleft_tool v0.4.0, hydroflow_plus v0.10.0, hydro_cli v0.10.0, safety bump 8 crates ([`dcd48fc`](https://github.com/hydro-project/hydro/commit/dcd48fc7ee805898d9b5ef0d082870e30615e95b))
</details>

## 0.5.7 (2024-08-30)

<csr-id-11af32828bab6e4a4264d2635ff71a12bb0bb778/>

### Chore

 - <csr-id-11af32828bab6e4a4264d2635ff71a12bb0bb778/> lower min dependency versions where possible, update `Cargo.lock`
   Moved from #1418
   
   ---------

### Documentation

 - <csr-id-f5f1eb0c612f5c0c1752360d972ef6853c5e12f0/> cleanup doc comments for clippy latest

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 38 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#1423](https://github.com/hydro-project/hydro/issues/1423), [#1428](https://github.com/hydro-project/hydro/issues/1428)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1423](https://github.com/hydro-project/hydro/issues/1423)**
    - Lower min dependency versions where possible, update `Cargo.lock` ([`11af328`](https://github.com/hydro-project/hydro/commit/11af32828bab6e4a4264d2635ff71a12bb0bb778))
 * **[#1428](https://github.com/hydro-project/hydro/issues/1428)**
    - Cleanup doc comments for clippy latest ([`f5f1eb0`](https://github.com/hydro-project/hydro/commit/f5f1eb0c612f5c0c1752360d972ef6853c5e12f0))
 * **Uncategorized**
    - Release hydroflow_lang v0.9.0, hydroflow_datalog_core v0.9.0, hydroflow_datalog v0.9.0, hydroflow_deploy_integration v0.9.0, hydroflow_macro v0.9.0, lattices_macro v0.5.6, lattices v0.5.7, multiplatform_test v0.2.0, variadics v0.0.6, pusherator v0.0.8, hydroflow v0.9.0, stageleft_macro v0.3.0, stageleft v0.4.0, stageleft_tool v0.3.0, hydroflow_plus v0.9.0, hydro_deploy v0.9.0, hydro_cli v0.9.0, hydroflow_plus_deploy v0.9.0, safety bump 8 crates ([`0750117`](https://github.com/hydro-project/hydro/commit/0750117de7088c01a439b102adeb4c832889f171))
</details>

## 0.5.6 (2024-07-23)

<csr-id-3098f77fd99882aae23c4b31017aa4b761306197/>

### Chore

 - <csr-id-3098f77fd99882aae23c4b31017aa4b761306197/> update pinned rust version to 2024-06-17

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

 - <csr-id-9c834406efcc3839a2a0d48b514146d06bb6e35d/> change fuzz test bounds to require `PartialEq` instead of `Eq`, fix #1302
 - <csr-id-1ad690b993f38ac6a03667fdce56e6603076b1d2/> Make inner for `WithTop` & `WithBot` private
   `Option<T>` is not a lattice, so it is unsafe to expose as public.
   
   I also updated documentation to lead with intention before
   implementation (minor cleanup).

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release.
 - 59 days passed between releases.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 4 unique issues were worked on: [#1244](https://github.com/hydro-project/hydro/issues/1244), [#1250](https://github.com/hydro-project/hydro/issues/1250), [#1309](https://github.com/hydro-project/hydro/issues/1309), [#1326](https://github.com/hydro-project/hydro/issues/1326)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1244](https://github.com/hydro-project/hydro/issues/1244)**
    - Make inner for `WithTop` & `WithBot` private ([`1ad690b`](https://github.com/hydro-project/hydro/commit/1ad690b993f38ac6a03667fdce56e6603076b1d2))
 * **[#1250](https://github.com/hydro-project/hydro/issues/1250)**
    - Add `#[derive(Lattice)]` derive macros, fix #1247 ([`b3d01c2`](https://github.com/hydro-project/hydro/commit/b3d01c20cae2335a3da2c02343debe677f17786b))
 * **[#1309](https://github.com/hydro-project/hydro/issues/1309)**
    - Update pinned rust version to 2024-06-17 ([`3098f77`](https://github.com/hydro-project/hydro/commit/3098f77fd99882aae23c4b31017aa4b761306197))
 * **[#1326](https://github.com/hydro-project/hydro/issues/1326)**
    - Change fuzz test bounds to require `PartialEq` instead of `Eq`, fix #1302 ([`9c83440`](https://github.com/hydro-project/hydro/commit/9c834406efcc3839a2a0d48b514146d06bb6e35d))
 * **Uncategorized**
    - Release hydroflow_lang v0.8.0, hydroflow_datalog_core v0.8.0, hydroflow_datalog v0.8.0, hydroflow_macro v0.8.0, lattices_macro v0.5.5, lattices v0.5.6, variadics v0.0.5, pusherator v0.0.7, hydroflow v0.8.0, hydroflow_plus v0.8.0, hydro_deploy v0.8.0, hydro_cli v0.8.0, hydroflow_plus_cli_integration v0.8.0, safety bump 7 crates ([`ca6c16b`](https://github.com/hydro-project/hydro/commit/ca6c16b4a7ce35e155fe7fc6c7d1676c37c9e4de))
</details>

## 0.5.5 (2024-05-24)

### Documentation

 - <csr-id-0d2f14b9237c0eaa8131d1d1118768357ac8133b/> Updating CONTRIBUTING.md with some info about feature branches
   Also updating GitHub workflows to run on feature branches as well.

### New Features

<csr-id-c2577bd0ad1969f4badf23874a9e7a6c1622c5c3/>
<csr-id-d8e4d9dc784ae28fcefe5f32a0561698c1196d31/>
<csr-id-c3f5a37ff746401a2383a900f9004e33072d5b1a/>

 - <csr-id-0ed1f26b485894d3f24bd4d3251f6d3134fd1947/> Make Pair<> members public
   Summary of types examined:
   
   - `Min<T>`: T is not a lattice

### Bug Fixes

 - <csr-id-c0a06bbd20e1621de46ab835dd27df162f689411/> typos in lattice docs

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 9 commits contributed to the release.
 - 48 days passed between releases.
 - 6 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 7 unique issues were worked on: [#1155](https://github.com/hydro-project/hydro/issues/1155), [#1156](https://github.com/hydro-project/hydro/issues/1156), [#1174](https://github.com/hydro-project/hydro/issues/1174), [#1181](https://github.com/hydro-project/hydro/issues/1181), [#1230](https://github.com/hydro-project/hydro/issues/1230), [#1233](https://github.com/hydro-project/hydro/issues/1233), [#1236](https://github.com/hydro-project/hydro/issues/1236)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1155](https://github.com/hydro-project/hydro/issues/1155)**
    - Add Integral domain to algebra library ([`eeef3ac`](https://github.com/hydro-project/hydro/commit/eeef3acd7f5f06d6d8cb7719ee2bebf7f980e537))
 * **[#1156](https://github.com/hydro-project/hydro/issues/1156)**
    - Add prototype of tagging algebraic properties ([`c3f5a37`](https://github.com/hydro-project/hydro/commit/c3f5a37ff746401a2383a900f9004e33072d5b1a))
 * **[#1174](https://github.com/hydro-project/hydro/issues/1174)**
    - Typos in lattice docs ([`c0a06bb`](https://github.com/hydro-project/hydro/commit/c0a06bbd20e1621de46ab835dd27df162f689411))
 * **[#1181](https://github.com/hydro-project/hydro/issues/1181)**
    - Part 1 ([`041bcf9`](https://github.com/hydro-project/hydro/commit/041bcf92a48f4fb0557796d541e27213d78235eb))
 * **[#1230](https://github.com/hydro-project/hydro/issues/1230)**
    - Expose PairBimorphism public. ([`c2577bd`](https://github.com/hydro-project/hydro/commit/c2577bd0ad1969f4badf23874a9e7a6c1622c5c3))
 * **[#1233](https://github.com/hydro-project/hydro/issues/1233)**
    - Make Pair<> members public ([`0ed1f26`](https://github.com/hydro-project/hydro/commit/0ed1f26b485894d3f24bd4d3251f6d3134fd1947))
 * **[#1236](https://github.com/hydro-project/hydro/issues/1236)**
    - Updating CONTRIBUTING.md with some info about feature branches ([`0d2f14b`](https://github.com/hydro-project/hydro/commit/0d2f14b9237c0eaa8131d1d1118768357ac8133b))
 * **Uncategorized**
    - Release hydroflow_lang v0.7.0, hydroflow_datalog_core v0.7.0, hydroflow_datalog v0.7.0, hydroflow_macro v0.7.0, lattices v0.5.5, multiplatform_test v0.1.0, pusherator v0.0.6, hydroflow v0.7.0, stageleft_macro v0.2.0, stageleft v0.3.0, stageleft_tool v0.2.0, hydroflow_plus v0.7.0, hydro_deploy v0.7.0, hydro_cli v0.7.0, hydroflow_plus_cli_integration v0.7.0, safety bump 8 crates ([`2852147`](https://github.com/hydro-project/hydro/commit/285214740627685e911781793e05d234ab2ad2bd))
    - Definitions of linearity and bilinearity in algebra lib ([`d8e4d9d`](https://github.com/hydro-project/hydro/commit/d8e4d9dc784ae28fcefe5f32a0561698c1196d31))
</details>

## 0.5.4 (2024-04-05)

<csr-id-2a10c4f395bbf3a320bdde6ec24c3c6abd5d6ed0/>

Unchanged from previous release.

### Chore

 - <csr-id-2a10c4f395bbf3a320bdde6ec24c3c6abd5d6ed0/> mark `lattices` as unchanged for `0.6.1` release

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 34 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1127](https://github.com/hydro-project/hydro/issues/1127)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1127](https://github.com/hydro-project/hydro/issues/1127)**
    - Initial Algebra Library ([`b6e3bec`](https://github.com/hydro-project/hydro/commit/b6e3bec0bff31f3b7e8166cf1b545c39a5b8d617))
 * **Uncategorized**
    - Release hydroflow_cli_integration v0.5.2, hydroflow_lang v0.6.1, hydroflow_datalog_core v0.6.1, lattices v0.5.4, hydroflow v0.6.1, stageleft_macro v0.1.1, stageleft v0.2.1, hydroflow_plus v0.6.1, hydro_deploy v0.6.1, hydro_cli v0.6.1, hydroflow_plus_cli_integration v0.6.1, stageleft_tool v0.1.1 ([`cd63f22`](https://github.com/hydro-project/hydro/commit/cd63f2258c961a40f0e5dbef20ac329a2d570ad0))
    - Mark `lattices` as unchanged for `0.6.1` release ([`2a10c4f`](https://github.com/hydro-project/hydro/commit/2a10c4f395bbf3a320bdde6ec24c3c6abd5d6ed0))
</details>

## 0.5.3 (2024-03-02)

<csr-id-39ab8b0278e9e3fe96552ace0a4ae768a6bc10d8/>
<csr-id-71353f0d4dfd9766dfdc715c4a91a028081f910f/>

### Chore

 - <csr-id-39ab8b0278e9e3fe96552ace0a4ae768a6bc10d8/> appease various clippy lints

### New Features

 - <csr-id-ff158dbb57ef3a754ed1cc834a19e30bb2895488/> impl missing `SimpleCollectionRef` for various collections types
 - <csr-id-c8d6985cc99e623432d609e1e1bc4cfd4c31feb7/> add `Lattice[Bi]Morphism` traits, impls for cartesian product, pair, and keyed

### Style

 - <csr-id-71353f0d4dfd9766dfdc715c4a91a028081f910f/> fix imports for clippy

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release.
 - 28 days passed between releases.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 3 unique issues were worked on: [#1061](https://github.com/hydro-project/hydro/issues/1061), [#1062](https://github.com/hydro-project/hydro/issues/1062), [#1084](https://github.com/hydro-project/hydro/issues/1084)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1061](https://github.com/hydro-project/hydro/issues/1061)**
    - Impl missing `SimpleCollectionRef` for various collections types ([`ff158db`](https://github.com/hydro-project/hydro/commit/ff158dbb57ef3a754ed1cc834a19e30bb2895488))
 * **[#1062](https://github.com/hydro-project/hydro/issues/1062)**
    - Add `Lattice[Bi]Morphism` traits, impls for cartesian product, pair, and keyed ([`c8d6985`](https://github.com/hydro-project/hydro/commit/c8d6985cc99e623432d609e1e1bc4cfd4c31feb7))
 * **[#1084](https://github.com/hydro-project/hydro/issues/1084)**
    - Appease various clippy lints ([`39ab8b0`](https://github.com/hydro-project/hydro/commit/39ab8b0278e9e3fe96552ace0a4ae768a6bc10d8))
 * **Uncategorized**
    - Release hydroflow_lang v0.6.0, hydroflow_datalog_core v0.6.0, hydroflow_datalog v0.6.0, hydroflow_macro v0.6.0, lattices v0.5.3, variadics v0.0.4, pusherator v0.0.5, hydroflow v0.6.0, stageleft v0.2.0, hydroflow_plus v0.6.0, hydro_deploy v0.6.0, hydro_cli v0.6.0, hydroflow_plus_cli_integration v0.6.0, safety bump 7 crates ([`09ea65f`](https://github.com/hydro-project/hydro/commit/09ea65fe9cd45c357c43bffca30e60243fa45cc8))
    - Fix imports for clippy ([`71353f0`](https://github.com/hydro-project/hydro/commit/71353f0d4dfd9766dfdc715c4a91a028081f910f))
</details>

## 0.5.2 (2024-02-02)

### New Features

 - <csr-id-87e86a2ab9e068634ebed17616b7482b3e69d539/> add map_union_with_tombstones, fix #336

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1052](https://github.com/hydro-project/hydro/issues/1052)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1052](https://github.com/hydro-project/hydro/issues/1052)**
    - Add map_union_with_tombstones, fix #336 ([`87e86a2`](https://github.com/hydro-project/hydro/commit/87e86a2ab9e068634ebed17616b7482b3e69d539))
 * **Uncategorized**
    - Release hydroflow_lang v0.5.2, hydroflow_datalog_core v0.5.2, hydroflow_macro v0.5.2, lattices v0.5.2, hydroflow v0.5.2, hydro_cli v0.5.1, hydroflow_plus_cli_integration v0.5.1 ([`6ac8720`](https://github.com/hydro-project/hydro/commit/6ac872081753548ebb8ec95549b4d820dc050d3e))
</details>

## 0.5.1 (2024-01-29)

<csr-id-1b555e57c8c812bed4d6495d2960cbf77fb0b3ef/>
<csr-id-ba6afab8416ad66eee4fdb9d0c73e62d45752617/>
<csr-id-f6a729925ddeb6063fa8c4b03d6621c1c35f0cc8/>

### Chore

 - <csr-id-1b555e57c8c812bed4d6495d2960cbf77fb0b3ef/> manually set lockstep-versioned crates (and `lattices`) to version `0.5.1`
   Setting manually since
   https://github.com/frewsxcv/rust-crates-index/issues/159 is messing with
   smart-release
 - <csr-id-ba6afab8416ad66eee4fdb9d0c73e62d45752617/> fix clippy lints on latest nightly
 - <csr-id-f6a729925ddeb6063fa8c4b03d6621c1c35f0cc8/> fix `clippy::items_after_test_module`, simplify rustdoc links

### New Features

 - <csr-id-e30602e6a3210a4ea4fe8a65aedb9469e79e3c37/> Add `DeepReveal` trait
 - <csr-id-3f701997ec1e6ca2a364537fbd2ef39cf96ce0f1/> add set_union_with_tombstones

### Bug Fixes

 - <csr-id-0539e2a91eb3ba71ed1c9fbe8d0c74b6344ad1bf/> chat and two_pc no longer replay

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 8 commits contributed to the release.
 - 6 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 4 unique issues were worked on: [#1032](https://github.com/hydro-project/hydro/issues/1032), [#942](https://github.com/hydro-project/hydro/issues/942), [#960](https://github.com/hydro-project/hydro/issues/960), [#967](https://github.com/hydro-project/hydro/issues/967)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1032](https://github.com/hydro-project/hydro/issues/1032)**
    - Fixup! feat(lattices): Add `DeepReveal` trait ([`0bed8ca`](https://github.com/hydro-project/hydro/commit/0bed8cab4a5e7a7be88de4d2e9c3c2d8ee0e7b7f))
    - Add `DeepReveal` trait ([`e30602e`](https://github.com/hydro-project/hydro/commit/e30602e6a3210a4ea4fe8a65aedb9469e79e3c37))
 * **[#942](https://github.com/hydro-project/hydro/issues/942)**
    - Fix `clippy::items_after_test_module`, simplify rustdoc links ([`f6a7299`](https://github.com/hydro-project/hydro/commit/f6a729925ddeb6063fa8c4b03d6621c1c35f0cc8))
 * **[#960](https://github.com/hydro-project/hydro/issues/960)**
    - Fix clippy lints on latest nightly ([`ba6afab`](https://github.com/hydro-project/hydro/commit/ba6afab8416ad66eee4fdb9d0c73e62d45752617))
 * **[#967](https://github.com/hydro-project/hydro/issues/967)**
    - Chat and two_pc no longer replay ([`0539e2a`](https://github.com/hydro-project/hydro/commit/0539e2a91eb3ba71ed1c9fbe8d0c74b6344ad1bf))
 * **Uncategorized**
    - Release hydroflow_cli_integration v0.5.1, hydroflow_lang v0.5.1, hydroflow_datalog_core v0.5.1, hydroflow_datalog v0.5.1, hydroflow_macro v0.5.1, lattices v0.5.1, variadics v0.0.3, pusherator v0.0.4, hydroflow v0.5.1, stageleft_macro v0.1.0, stageleft v0.1.0, hydroflow_plus v0.5.1, hydro_deploy v0.5.1, hydro_cli v0.5.1 ([`478aebc`](https://github.com/hydro-project/hydro/commit/478aebc8fee2aa78eab86bd386322db1c70bde6a))
    - Manually set lockstep-versioned crates (and `lattices`) to version `0.5.1` ([`1b555e5`](https://github.com/hydro-project/hydro/commit/1b555e57c8c812bed4d6495d2960cbf77fb0b3ef))
    - Add set_union_with_tombstones ([`3f70199`](https://github.com/hydro-project/hydro/commit/3f701997ec1e6ca2a364537fbd2ef39cf96ce0f1))
</details>

## 0.5.0 (2023-10-11)

<csr-id-e788989737fbd501173bc99c6f9f5f5ba514ec9c/>

### Chore

 - <csr-id-e788989737fbd501173bc99c6f9f5f5ba514ec9c/> Fix `clippy::implied_bounds_in_impls` from latest nightlies

### Documentation

 - <csr-id-6b82126347e2ae3c11cc10fea4f3fbcb463734e6/> fix lattice math link

### New Features

 - <csr-id-488d6dd448e10e2bf217693dd2a29973488c838a/> Add serde derives to collections
 - <csr-id-35c2606f2df16a428a5c163d5582923ecd5998c4/> Add `UnionFind` lattice

### Bug Fixes (BREAKING)

 - <csr-id-18e9cfaa8b1415d72d67a69d7b0fecc997b5670a/> fix some types and semantics for atomization

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 7 commits contributed to the release.
 - 56 days passed between releases.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#915](https://github.com/hydro-project/hydro/issues/915), [#922](https://github.com/hydro-project/hydro/issues/922)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#915](https://github.com/hydro-project/hydro/issues/915)**
    - Add `UnionFind` lattice ([`35c2606`](https://github.com/hydro-project/hydro/commit/35c2606f2df16a428a5c163d5582923ecd5998c4))
    - Fix some types and semantics for atomization ([`18e9cfa`](https://github.com/hydro-project/hydro/commit/18e9cfaa8b1415d72d67a69d7b0fecc997b5670a))
 * **[#922](https://github.com/hydro-project/hydro/issues/922)**
    - Add serde derives to collections ([`488d6dd`](https://github.com/hydro-project/hydro/commit/488d6dd448e10e2bf217693dd2a29973488c838a))
 * **Uncategorized**
    - Release hydroflow_macro v0.5.0, lattices v0.5.0, hydroflow v0.5.0, hydro_cli v0.5.0 ([`12697c2`](https://github.com/hydro-project/hydro/commit/12697c2f19bd96802591fa63a5b6b12104ecfe0d))
    - Release hydroflow_lang v0.5.0, hydroflow_datalog_core v0.5.0, hydroflow_datalog v0.5.0, hydroflow_macro v0.5.0, lattices v0.5.0, hydroflow v0.5.0, hydro_cli v0.5.0, safety bump 4 crates ([`2e2d8b3`](https://github.com/hydro-project/hydro/commit/2e2d8b386fb086c8276a2853d2a1f96ad4d7c221))
    - Fix lattice math link ([`6b82126`](https://github.com/hydro-project/hydro/commit/6b82126347e2ae3c11cc10fea4f3fbcb463734e6))
    - Fix `clippy::implied_bounds_in_impls` from latest nightlies ([`e788989`](https://github.com/hydro-project/hydro/commit/e788989737fbd501173bc99c6f9f5f5ba514ec9c))
</details>

## 0.4.0 (2023-08-15)

<csr-id-f60053f70da3071c54de4a0eabb059a143aa2ccc/>
<csr-id-6a2ad6b770c2ccf470548320d8753025b3a66c0a/>
<csr-id-262166e7cecf8ffb5a2c7bc989e8cf66c4524a68/>
<csr-id-7b0485b20939ec86ed8e74ecc9c75ac1b5d01072/>

### Chore

 - <csr-id-f60053f70da3071c54de4a0eabb059a143aa2ccc/> fix lint, format errors for latest nightly version (without updated pinned)
   For nightly version (d9c13cd45 2023-07-05)

### Documentation

 - <csr-id-a8b0d2d10eef3e45669f77a1f2460cd31a95d15b/> Improve `Atomize` docs

### New Features

 - <csr-id-7282457e383407eabbeb1f931c130edb095c33ca/> formalize `Default::default()` as returning bottom for lattice types
   Not a breaking change since changed names were introduced only since last release
 - <csr-id-b2406994a703f028724cc30065fec60f7f8a7247/> Implement `SimpleKeyedRef` for map types
 - <csr-id-8ec75c6d8998b7d7e5a0ae24ee53b0cdb6932683/> Add atomize trait, impls, tests

### Refactor

 - <csr-id-6a2ad6b770c2ccf470548320d8753025b3a66c0a/> fix new clippy lints on latest nightly 1.73.0-nightly (db7ff98a7 2023-07-31)
 - <csr-id-262166e7cecf8ffb5a2c7bc989e8cf66c4524a68/> Change `Atomize` to require returning empty iff lattice is bottom
   Previously was the opposite, `Atomize` always had to return non-empty.
   
   Not breaking since `Atomize` has not yet been published.

### New Features (BREAKING)

 - <csr-id-7b752f743cbedc632b127dddf3f9a84e839eb47a/> Add bottom (+top) collapsing, implement `IsBot`/`IsTop` for all lattice types
   * `WithBot(Some(BOTTOM))` and `WithBot(None)` are now considered to both be bottom, equal. Also, `MapUnion({})` and `MapUnion({key: BOTTOM})` are considered to both be bottom, equal.
* `WithTop(Some(TOP))` and `WithTop(None)` are now considered to both be top, equal.
* `check_lattice_bot/top` now check that `is_bot` and `is_top` must be consistent among all equal elements

### Refactor (BREAKING)

 - <csr-id-7b0485b20939ec86ed8e74ecc9c75ac1b5d01072/> Rename `Seq` -> `VecUnion`

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 10 commits contributed to the release.
 - 42 days passed between releases.
 - 9 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 8 unique issues were worked on: [#822](https://github.com/hydro-project/hydro/issues/822), [#849](https://github.com/hydro-project/hydro/issues/849), [#854](https://github.com/hydro-project/hydro/issues/854), [#860](https://github.com/hydro-project/hydro/issues/860), [#865](https://github.com/hydro-project/hydro/issues/865), [#866](https://github.com/hydro-project/hydro/issues/866), [#867](https://github.com/hydro-project/hydro/issues/867), [#879](https://github.com/hydro-project/hydro/issues/879)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#822](https://github.com/hydro-project/hydro/issues/822)**
    - Fix lint, format errors for latest nightly version (without updated pinned) ([`f60053f`](https://github.com/hydro-project/hydro/commit/f60053f70da3071c54de4a0eabb059a143aa2ccc))
 * **[#849](https://github.com/hydro-project/hydro/issues/849)**
    - Rename `Seq` -> `VecUnion` ([`7b0485b`](https://github.com/hydro-project/hydro/commit/7b0485b20939ec86ed8e74ecc9c75ac1b5d01072))
 * **[#854](https://github.com/hydro-project/hydro/issues/854)**
    - Add atomize trait, impls, tests ([`8ec75c6`](https://github.com/hydro-project/hydro/commit/8ec75c6d8998b7d7e5a0ae24ee53b0cdb6932683))
 * **[#860](https://github.com/hydro-project/hydro/issues/860)**
    - Improve `Atomize` docs ([`a8b0d2d`](https://github.com/hydro-project/hydro/commit/a8b0d2d10eef3e45669f77a1f2460cd31a95d15b))
 * **[#865](https://github.com/hydro-project/hydro/issues/865)**
    - Add bottom (+top) collapsing, implement `IsBot`/`IsTop` for all lattice types ([`7b752f7`](https://github.com/hydro-project/hydro/commit/7b752f743cbedc632b127dddf3f9a84e839eb47a))
 * **[#866](https://github.com/hydro-project/hydro/issues/866)**
    - Implement `SimpleKeyedRef` for map types ([`b240699`](https://github.com/hydro-project/hydro/commit/b2406994a703f028724cc30065fec60f7f8a7247))
 * **[#867](https://github.com/hydro-project/hydro/issues/867)**
    - Change `Atomize` to require returning empty iff lattice is bottom ([`262166e`](https://github.com/hydro-project/hydro/commit/262166e7cecf8ffb5a2c7bc989e8cf66c4524a68))
 * **[#879](https://github.com/hydro-project/hydro/issues/879)**
    - Formalize `Default::default()` as returning bottom for lattice types ([`7282457`](https://github.com/hydro-project/hydro/commit/7282457e383407eabbeb1f931c130edb095c33ca))
 * **Uncategorized**
    - Release hydroflow_lang v0.4.0, hydroflow_datalog_core v0.4.0, hydroflow_datalog v0.4.0, hydroflow_macro v0.4.0, lattices v0.4.0, pusherator v0.0.3, hydroflow v0.4.0, hydro_cli v0.4.0, safety bump 4 crates ([`cb313f0`](https://github.com/hydro-project/hydro/commit/cb313f0635214460a8308d05cbef4bf7f4bfaa15))
    - Fix new clippy lints on latest nightly 1.73.0-nightly (db7ff98a7 2023-07-31) ([`6a2ad6b`](https://github.com/hydro-project/hydro/commit/6a2ad6b770c2ccf470548320d8753025b3a66c0a))
</details>

## 0.3.0 (2023-07-04)

<csr-id-0cbbaeaec5e192e2539771bb247926271c2dc4a3/>
<csr-id-70c88a51c4c83a4dc2fc67a0cd344786a4ff26f7/>
<csr-id-4a727ecf1232e0f03f5300547282bfbe73342cfa/>
<csr-id-5c7e4d3aea1dfb61d51bcb0291740281824e3090/>
<csr-id-1bdadb82b25941d11f3fa24eaac35109927c852f/>

### Documentation

 - <csr-id-ac4fd827ccede0ad53dfc59079cdb7df5928e491/> List `WithTop` in README 4/4

### New Features

 - <csr-id-016abeea3ecd390a976dd8dbec371b08fe744655/> make unit `()` a point lattice
 - <csr-id-dc99c021640a47b704905d087eadcbc477f033f0/> impl `IsTop`, `IsBot` for `Min`, `Max` over numeric types
 - <csr-id-f5e0d19e8531c250bc4492b61b9731c947916daf/> Add `Conflict<T>` lattice
 - <csr-id-fc4dcbdfa703d79a0c183a2eb3f5dbb42260b67a/> add top lattice, opposite of bottom
 - <csr-id-153cbabd462d776eae395e371470abb4662642cd/> Add `Seq` lattice.

### Bug Fixes

 - <csr-id-9bb5528d99e83fdae5aeca9456802379131c2f90/> removed unused nightly features `impl_trait_in_assoc_type`, `type_alias_impl_trait`
 - <csr-id-3c4eb16833160f8813b812487a1297c023400138/> fix ConvertFrom for bottom to actually convert the type
   * fix: fix type inference with doubly-nested bottom types
* fix: address comments

### Refactor

 - <csr-id-0cbbaeaec5e192e2539771bb247926271c2dc4a3/> Rename `bottom.rs` -> `with_bot.rs`, `top.rs` -> `with_top.rs` 1/4

### Style

 - <csr-id-70c88a51c4c83a4dc2fc67a0cd344786a4ff26f7/> `warn` missing docs (instead of `deny`) to allow code before docs

### New Features (BREAKING)

<csr-id-deb26af6bcd547f91bf339367387d36e5e59565a/>

 - <csr-id-931d93887c238025596cb22226e16d43e16a7425/> Add `reveal` methods, make fields private
 - <csr-id-7aec1ac884e01a560770dfab7e0ba64d520415f6/> Add `Provenance` generic param token to `Point`.
   - Use `()` provenance for `kvs_bench` example.

### Bug Fixes (BREAKING)

 - <csr-id-5cfd2a0f48f11f6185070cab932f50b630e1f800/> Remove `Default` impl for `WithTop` 3/4
   Is confusing, probably not what users want.

### Refactor (BREAKING)

 - <csr-id-4a727ecf1232e0f03f5300547282bfbe73342cfa/> Rename `ConvertFrom::from` -> `LatticeFrom::lattice_from`
 - <csr-id-5c7e4d3aea1dfb61d51bcb0291740281824e3090/> Rename `Bottom` -> `WithBot`, `Top` -> `WithTop`, constructors now take `Option`s 2/4
 - <csr-id-1bdadb82b25941d11f3fa24eaac35109927c852f/> Rename `Immut` -> `Point` lattice.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 18 commits contributed to the release.
 - 33 days passed between releases.
 - 17 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 12 unique issues were worked on: [#742](https://github.com/hydro-project/hydro/issues/742), [#744](https://github.com/hydro-project/hydro/issues/744), [#761](https://github.com/hydro-project/hydro/issues/761), [#763](https://github.com/hydro-project/hydro/issues/763), [#765](https://github.com/hydro-project/hydro/issues/765), [#766](https://github.com/hydro-project/hydro/issues/766), [#767](https://github.com/hydro-project/hydro/issues/767), [#772](https://github.com/hydro-project/hydro/issues/772), [#773](https://github.com/hydro-project/hydro/issues/773), [#780](https://github.com/hydro-project/hydro/issues/780), [#789](https://github.com/hydro-project/hydro/issues/789), [#793](https://github.com/hydro-project/hydro/issues/793)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#742](https://github.com/hydro-project/hydro/issues/742)**
    - Fix ConvertFrom for bottom to actually convert the type ([`3c4eb16`](https://github.com/hydro-project/hydro/commit/3c4eb16833160f8813b812487a1297c023400138))
 * **[#744](https://github.com/hydro-project/hydro/issues/744)**
    - Add top lattice, opposite of bottom ([`fc4dcbd`](https://github.com/hydro-project/hydro/commit/fc4dcbdfa703d79a0c183a2eb3f5dbb42260b67a))
 * **[#761](https://github.com/hydro-project/hydro/issues/761)**
    - Rename `Immut` -> `Point` lattice. ([`1bdadb8`](https://github.com/hydro-project/hydro/commit/1bdadb82b25941d11f3fa24eaac35109927c852f))
 * **[#763](https://github.com/hydro-project/hydro/issues/763)**
    - List `WithTop` in README 4/4 ([`ac4fd82`](https://github.com/hydro-project/hydro/commit/ac4fd827ccede0ad53dfc59079cdb7df5928e491))
    - Remove `Default` impl for `WithTop` 3/4 ([`5cfd2a0`](https://github.com/hydro-project/hydro/commit/5cfd2a0f48f11f6185070cab932f50b630e1f800))
    - Rename `Bottom` -> `WithBot`, `Top` -> `WithTop`, constructors now take `Option`s 2/4 ([`5c7e4d3`](https://github.com/hydro-project/hydro/commit/5c7e4d3aea1dfb61d51bcb0291740281824e3090))
    - Rename `bottom.rs` -> `with_bot.rs`, `top.rs` -> `with_top.rs` 1/4 ([`0cbbaea`](https://github.com/hydro-project/hydro/commit/0cbbaeaec5e192e2539771bb247926271c2dc4a3))
 * **[#765](https://github.com/hydro-project/hydro/issues/765)**
    - Rename `ConvertFrom::from` -> `LatticeFrom::lattice_from` ([`4a727ec`](https://github.com/hydro-project/hydro/commit/4a727ecf1232e0f03f5300547282bfbe73342cfa))
 * **[#766](https://github.com/hydro-project/hydro/issues/766)**
    - Add `IsBot::is_bot` and `IsTop::is_top` traits ([`deb26af`](https://github.com/hydro-project/hydro/commit/deb26af6bcd547f91bf339367387d36e5e59565a))
 * **[#767](https://github.com/hydro-project/hydro/issues/767)**
    - Add `Conflict<T>` lattice ([`f5e0d19`](https://github.com/hydro-project/hydro/commit/f5e0d19e8531c250bc4492b61b9731c947916daf))
 * **[#772](https://github.com/hydro-project/hydro/issues/772)**
    - Add `Provenance` generic param token to `Point`. ([`7aec1ac`](https://github.com/hydro-project/hydro/commit/7aec1ac884e01a560770dfab7e0ba64d520415f6))
 * **[#773](https://github.com/hydro-project/hydro/issues/773)**
    - `warn` missing docs (instead of `deny`) to allow code before docs ([`70c88a5`](https://github.com/hydro-project/hydro/commit/70c88a51c4c83a4dc2fc67a0cd344786a4ff26f7))
 * **[#780](https://github.com/hydro-project/hydro/issues/780)**
    - Removed unused nightly features `impl_trait_in_assoc_type`, `type_alias_impl_trait` ([`9bb5528`](https://github.com/hydro-project/hydro/commit/9bb5528d99e83fdae5aeca9456802379131c2f90))
 * **[#789](https://github.com/hydro-project/hydro/issues/789)**
    - Add `reveal` methods, make fields private ([`931d938`](https://github.com/hydro-project/hydro/commit/931d93887c238025596cb22226e16d43e16a7425))
 * **[#793](https://github.com/hydro-project/hydro/issues/793)**
    - Make unit `()` a point lattice ([`016abee`](https://github.com/hydro-project/hydro/commit/016abeea3ecd390a976dd8dbec371b08fe744655))
    - Impl `IsTop`, `IsBot` for `Min`, `Max` over numeric types ([`dc99c02`](https://github.com/hydro-project/hydro/commit/dc99c021640a47b704905d087eadcbc477f033f0))
 * **Uncategorized**
    - Release hydroflow_cli_integration v0.3.0, hydroflow_lang v0.3.0, hydroflow_datalog_core v0.3.0, hydroflow_datalog v0.3.0, hydroflow_macro v0.3.0, lattices v0.3.0, pusherator v0.0.2, hydroflow v0.3.0, hydro_cli v0.3.0, safety bump 5 crates ([`ec9633e`](https://github.com/hydro-project/hydro/commit/ec9633e2e393c2bf106223abeb0b680200fbdf84))
    - Add `Seq` lattice. ([`153cbab`](https://github.com/hydro-project/hydro/commit/153cbabd462d776eae395e371470abb4662642cd))
</details>

## 0.2.0 (2023-05-31)

<csr-id-fd896fbe925fbd8ef1d16be7206ac20ba585081a/>
<csr-id-10b308532245db8f4480ce53b67aea050ae1918d/>

### Chore

 - <csr-id-fd896fbe925fbd8ef1d16be7206ac20ba585081a/> manually bump versions for v0.2.0 release

### Refactor (BREAKING)

 - <csr-id-10b308532245db8f4480ce53b67aea050ae1918d/> rename `Fake` -> `Immut`

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 1 day passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release hydroflow_lang v0.2.0, hydroflow_datalog_core v0.2.0, hydroflow_datalog v0.2.0, hydroflow_macro v0.2.0, lattices v0.2.0, hydroflow v0.2.0, hydro_cli v0.2.0 ([`ca464c3`](https://github.com/hydro-project/hydro/commit/ca464c32322a7ad39eb53e1794777c849aa548a0))
    - Manually bump versions for v0.2.0 release ([`fd896fb`](https://github.com/hydro-project/hydro/commit/fd896fbe925fbd8ef1d16be7206ac20ba585081a))
    - Rename `Fake` -> `Immut` ([`10b3085`](https://github.com/hydro-project/hydro/commit/10b308532245db8f4480ce53b67aea050ae1918d))
</details>

## 0.1.2 (2023-05-30)

### New Features

 - <csr-id-ecff609a0153446efc1809230ae100964bb9f89b/> print out items when lattice identity tests fail

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 6 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#691](https://github.com/hydro-project/hydro/issues/691)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#691](https://github.com/hydro-project/hydro/issues/691)**
    - Print out items when lattice identity tests fail ([`ecff609`](https://github.com/hydro-project/hydro/commit/ecff609a0153446efc1809230ae100964bb9f89b))
 * **Uncategorized**
    - Release hydroflow_cli_integration v0.1.1, hydroflow_lang v0.1.1, hydroflow_datalog_core v0.1.1, hydroflow_macro v0.1.1, lattices v0.1.2, hydroflow v0.1.1, hydro_cli v0.1.0 ([`d9fa8b3`](https://github.com/hydro-project/hydro/commit/d9fa8b387e303b33d9614dbde80abf1af08bd8eb))
</details>

## 0.1.1 (2023-05-23)

<csr-id-3bee6f858a78d82b7431e124ef9792002c8d77ce/>

### Documentation

 - <csr-id-720744fc90fa05a11e0b79c96baba2eb6fd1c7f3/> simplified explanations, fixed typos, removed dead named links
 - <csr-id-4bc1ac1ea2fa6257219ec7fae94a2b039ec7eb7b/> update links from old to new book

### Refactor

 - <csr-id-3bee6f858a78d82b7431e124ef9792002c8d77ce/> update cc-traits to v2, remove `SimpleKeyedRef` shim

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release.
 - 2 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 3 unique issues were worked on: [#671](https://github.com/hydro-project/hydro/issues/671), [#674](https://github.com/hydro-project/hydro/issues/674), [#687](https://github.com/hydro-project/hydro/issues/687)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#671](https://github.com/hydro-project/hydro/issues/671)**
    - Migrate docs to a unified Docusuarus site ([`feed326`](https://github.com/hydro-project/hydro/commit/feed3268c0aabeb027b19abd9ed06c565a0462f4))
 * **[#674](https://github.com/hydro-project/hydro/issues/674)**
    - Update cc-traits to v2, remove `SimpleKeyedRef` shim ([`3bee6f8`](https://github.com/hydro-project/hydro/commit/3bee6f858a78d82b7431e124ef9792002c8d77ce))
 * **[#687](https://github.com/hydro-project/hydro/issues/687)**
    - Simplified explanations, fixed typos, removed dead named links ([`720744f`](https://github.com/hydro-project/hydro/commit/720744fc90fa05a11e0b79c96baba2eb6fd1c7f3))
    - Update links from old to new book ([`4bc1ac1`](https://github.com/hydro-project/hydro/commit/4bc1ac1ea2fa6257219ec7fae94a2b039ec7eb7b))
 * **Uncategorized**
    - Release hydroflow_cli_integration v0.1.0, hydroflow_internalmacro v0.1.0, hydroflow_lang v0.1.0, hydroflow_datalog_core v0.1.0, hydroflow_datalog v0.1.0, hydroflow_macro v0.1.0, lattices v0.1.1, hydroflow v0.1.0 ([`7324974`](https://github.com/hydro-project/hydro/commit/73249744293c9b89cbaa2d84b23ca3f25b00ae4e))
</details>

## 0.1.0 (2023-05-21)

<csr-id-cd0a86d9271d0e3daab59c46f079925f863424e1/>
<csr-id-20a1b2c0cd04a8b495a02ce345db3d48a99ea0e9/>
<csr-id-1eda91a2ef8794711ef037240f15284e8085d863/>

### Documentation

 - <csr-id-95d23eaf8218002ad0a6a8c4c6e6c76e6b8f785b/> Update docs, add book chapter for `lattices` crate
   - Adds `mdbook-katex` to the book build for latex support.

### New Features

 - <csr-id-15f9688ff4dc816a374ed9068d98bee0a4d51b2c/> Make lattice test helpers public, restructure
   Also impl `LatticeOrd` for `SetUnion`

### Style

 - <csr-id-cd0a86d9271d0e3daab59c46f079925f863424e1/> Warn lint `unused_qualifications`
 - <csr-id-20a1b2c0cd04a8b495a02ce345db3d48a99ea0e9/> rustfmt group imports
 - <csr-id-1eda91a2ef8794711ef037240f15284e8085d863/> rustfmt prescribe flat-module `use` format

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 14 commits contributed to the release.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 10 unique issues were worked on: [#625](https://github.com/hydro-project/hydro/issues/625), [#637](https://github.com/hydro-project/hydro/issues/637), [#638](https://github.com/hydro-project/hydro/issues/638), [#642](https://github.com/hydro-project/hydro/issues/642), [#644](https://github.com/hydro-project/hydro/issues/644), [#645](https://github.com/hydro-project/hydro/issues/645), [#658](https://github.com/hydro-project/hydro/issues/658), [#660](https://github.com/hydro-project/hydro/issues/660), [#664](https://github.com/hydro-project/hydro/issues/664), [#667](https://github.com/hydro-project/hydro/issues/667)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#625](https://github.com/hydro-project/hydro/issues/625)**
    - Use `cc-traits` instead of own `Collection`, remove `tag` indirection ([`10ed00d`](https://github.com/hydro-project/hydro/commit/10ed00df8e6f2e86d7db737dd2049f2c5dbfeba0))
 * **[#637](https://github.com/hydro-project/hydro/issues/637)**
    - Add bottom and fake lattice types ([`95ce1ed`](https://github.com/hydro-project/hydro/commit/95ce1edbecebdc505893b1f527bef905b4247ed8))
 * **[#638](https://github.com/hydro-project/hydro/issues/638)**
    - Remove old lattice code ([`f4915fa`](https://github.com/hydro-project/hydro/commit/f4915fab98c57652e5345d39076d95ebb0a43fd8))
 * **[#642](https://github.com/hydro-project/hydro/issues/642)**
    - Remove zmq, use unsync channels locally, use sync mpsc cross-thread, use cross_join+enumerate instead of broadcast channel,remove Eq requirement from multisetjoin ([`b38f5cf`](https://github.com/hydro-project/hydro/commit/b38f5cf198e29a8de2f84eb4cd075818fbeffda6))
 * **[#644](https://github.com/hydro-project/hydro/issues/644)**
    - Remove Compare trait, add tests, make all lattice types PartialOrd, Eq, PartialEq ([`698b72f`](https://github.com/hydro-project/hydro/commit/698b72f8f013288f211a655bf93f2a3cd6d386e7))
 * **[#645](https://github.com/hydro-project/hydro/issues/645)**
    - Fix `Pair` `PartialOrd` implementation, add consistency tests with `NaiveOrd` ([`76e19a7`](https://github.com/hydro-project/hydro/commit/76e19a7346cd6005a04c39974bbcf4ed2bd37217))
 * **[#658](https://github.com/hydro-project/hydro/issues/658)**
    - Allow fake to merge, compare equal values ([`1a159dc`](https://github.com/hydro-project/hydro/commit/1a159dc8a16e40fa40ae3e8433d53c08d82e833c))
 * **[#660](https://github.com/hydro-project/hydro/issues/660)**
    - Warn lint `unused_qualifications` ([`cd0a86d`](https://github.com/hydro-project/hydro/commit/cd0a86d9271d0e3daab59c46f079925f863424e1))
    - Rustfmt group imports ([`20a1b2c`](https://github.com/hydro-project/hydro/commit/20a1b2c0cd04a8b495a02ce345db3d48a99ea0e9))
    - Rustfmt prescribe flat-module `use` format ([`1eda91a`](https://github.com/hydro-project/hydro/commit/1eda91a2ef8794711ef037240f15284e8085d863))
 * **[#664](https://github.com/hydro-project/hydro/issues/664)**
    - Make lattice test helpers public, restructure ([`15f9688`](https://github.com/hydro-project/hydro/commit/15f9688ff4dc816a374ed9068d98bee0a4d51b2c))
 * **[#667](https://github.com/hydro-project/hydro/issues/667)**
    - Bump lattices version to `0.1.0` ([`a46ce4a`](https://github.com/hydro-project/hydro/commit/a46ce4a522b70661e5acf644f893bfdf56294578))
    - Update docs, add book chapter for `lattices` crate ([`95d23ea`](https://github.com/hydro-project/hydro/commit/95d23eaf8218002ad0a6a8c4c6e6c76e6b8f785b))
 * **Uncategorized**
    - Release hydroflow_cli_integration v0.0.1, hydroflow_lang v0.0.1, hydroflow_datalog_core v0.0.1, hydroflow_datalog v0.0.1, hydroflow_macro v0.0.1, lattices v0.1.0, variadics v0.0.2, pusherator v0.0.1, hydroflow v0.0.2 ([`809395a`](https://github.com/hydro-project/hydro/commit/809395acddb78949d7a2bf036e1a94972f23b1ad))
</details>

## 0.0.0 (2023-05-02)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#634](https://github.com/hydro-project/hydro/issues/634), [#636](https://github.com/hydro-project/hydro/issues/636)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#634](https://github.com/hydro-project/hydro/issues/634)**
    - Fixup! Move lattice2 into new separate `lattices` crate ([`b08e793`](https://github.com/hydro-project/hydro/commit/b08e793edf30e40f21c916ae630a09337034150e))
    - Move lattice2 into new separate `lattices` crate ([`c0006c4`](https://github.com/hydro-project/hydro/commit/c0006c4c73e0f3f5c65274e3ad76537ea9fe2643))
 * **[#636](https://github.com/hydro-project/hydro/issues/636)**
    - Fixup! Move lattice2 into new separate `lattices` crate ([`b08e793`](https://github.com/hydro-project/hydro/commit/b08e793edf30e40f21c916ae630a09337034150e))
</details>

