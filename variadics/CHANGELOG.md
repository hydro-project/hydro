# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.0.9 (2025-03-08)

### Chore

 - <csr-id-49a387d4a21f0763df8ec94de73fb953c9cd333a/> upgrade to Rust 2024 edition
   - Updates `Cargo.toml` to use new shared workspace keys
   - Updates lint settings (in workspace `Cargo.toml`)
   - `rustfmt` has changed slightly, resulting in a big diff - there are no
   actual code changes
   - Adds a script to `rustfmt` the template src files
 - <csr-id-2fd6aa7417dfa29f389c04c5b9674b80bfed6cf2/> update pinned nightly to 2025-02-10, cleanups for clippy

### Refactor

 - <csr-id-c293cca6855695107e9cef5c5df99fb04a571934/> enable lints, cleanups for Rust 2024 #1732

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release over the course of 24 calendar days.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 3 unique issues were worked on: [#1704](https://github.com/hydro-project/hydro/issues/1704), [#1737](https://github.com/hydro-project/hydro/issues/1737), [#1747](https://github.com/hydro-project/hydro/issues/1747)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1704](https://github.com/hydro-project/hydro/issues/1704)**
    - Update pinned nightly to 2025-02-10, cleanups for clippy ([`2fd6aa7`](https://github.com/hydro-project/hydro/commit/2fd6aa7417dfa29f389c04c5b9674b80bfed6cf2))
 * **[#1737](https://github.com/hydro-project/hydro/issues/1737)**
    - Enable lints, cleanups for Rust 2024 #1732 ([`c293cca`](https://github.com/hydro-project/hydro/commit/c293cca6855695107e9cef5c5df99fb04a571934))
 * **[#1747](https://github.com/hydro-project/hydro/issues/1747)**
    - Upgrade to Rust 2024 edition ([`49a387d`](https://github.com/hydro-project/hydro/commit/49a387d4a21f0763df8ec94de73fb953c9cd333a))
</details>

## 0.0.8 (2024-12-23)

### Documentation

 - <csr-id-28cd220c68e3660d9ebade113949a2346720cd04/> add `repository` field to `Cargo.toml`s, fix #1452
   #1452 
   
   Will trigger new releases of the following:
   `unchanged = 'hydroflow_deploy_integration', 'variadics',
   'variadics_macro', 'pusherator'`
   
   (All other crates already have changes, so would be released anyway)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 45 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1501](https://github.com/hydro-project/hydro/issues/1501)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1501](https://github.com/hydro-project/hydro/issues/1501)**
    - Add `repository` field to `Cargo.toml`s, fix #1452 ([`28cd220`](https://github.com/hydro-project/hydro/commit/28cd220c68e3660d9ebade113949a2346720cd04))
 * **Uncategorized**
    - Release dfir_lang v0.11.0, dfir_datalog_core v0.11.0, dfir_datalog v0.11.0, dfir_macro v0.11.0, hydroflow_deploy_integration v0.11.0, lattices_macro v0.5.8, variadics v0.0.8, variadics_macro v0.5.6, lattices v0.5.9, multiplatform_test v0.4.0, pusherator v0.0.10, dfir_rs v0.11.0, hydro_deploy v0.11.0, stageleft_macro v0.5.0, stageleft v0.6.0, stageleft_tool v0.5.0, hydro_lang v0.11.0, hydro_std v0.11.0, hydro_cli v0.11.0, safety bump 6 crates ([`9a7e486`](https://github.com/hydro-project/hydro/commit/9a7e48693fce0face0f8ad16349258cdbe26395f))
</details>

## 0.0.7 (2024-11-08)

<csr-id-d5677604e93c07a5392f4229af94a0b736eca382/>
<csr-id-47cb703e771f7d1c451ceb9d185ada96410949da/>
<csr-id-656ee328c8710bce7370c851437a80ca3db46a5a/>

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
 - <csr-id-1c2825942f8a326699a7fb68b5372b49918851b5/> additions to variadics including collection types
   adds a number of features:
   
   collection types for variadics (sets, multisets) that allow search via
   RefVars (variadic of refs)
   into_option (convert a variadic to a variadic of options)
   into_vec (convert a variadic to a variadic of vecs)
 - <csr-id-8afd3266dac43c04c3fc29065a13c9c9a6a55afe/> additions to variadics including collection types
   adds a number of features:
   - collection types for variadics (sets, multisets) that allow search via
   RefVars (variadic of refs)

### Style

 - <csr-id-47cb703e771f7d1c451ceb9d185ada96410949da/> fixes for nightly clippy
   a couple few spurious `too_many_arguments` and a spurious
   `zombie_processes` still on current nightly (`clippy 0.1.84 (4392847410
   2024-10-21)`)

### Test

 - <csr-id-656ee328c8710bce7370c851437a80ca3db46a5a/> ignore trybuild tests inconsistent on latest nightly

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 8 commits contributed to the release.
 - 69 days passed between releases.
 - 6 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 6 unique issues were worked on: [#1444](https://github.com/hydro-project/hydro/issues/1444), [#1473](https://github.com/hydro-project/hydro/issues/1473), [#1474](https://github.com/hydro-project/hydro/issues/1474), [#1475](https://github.com/hydro-project/hydro/issues/1475), [#1503](https://github.com/hydro-project/hydro/issues/1503), [#1505](https://github.com/hydro-project/hydro/issues/1505)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1444](https://github.com/hydro-project/hydro/issues/1444)**
    - Update pinned rust version, clippy lints, remove some dead code ([`d567760`](https://github.com/hydro-project/hydro/commit/d5677604e93c07a5392f4229af94a0b736eca382))
 * **[#1473](https://github.com/hydro-project/hydro/issues/1473)**
    - Additions to variadics including collection types ([`8afd326`](https://github.com/hydro-project/hydro/commit/8afd3266dac43c04c3fc29065a13c9c9a6a55afe))
 * **[#1474](https://github.com/hydro-project/hydro/issues/1474)**
    - Revert "feat: additions to variadics including collection types" ([`08c2af5`](https://github.com/hydro-project/hydro/commit/08c2af538821bbf460d2a52b4f0474082b5de7da))
 * **[#1475](https://github.com/hydro-project/hydro/issues/1475)**
    - Additions to variadics including collection types ([`1c28259`](https://github.com/hydro-project/hydro/commit/1c2825942f8a326699a7fb68b5372b49918851b5))
 * **[#1503](https://github.com/hydro-project/hydro/issues/1503)**
    - Generalized hash trie indexes for relational tuples ([`f7e740f`](https://github.com/hydro-project/hydro/commit/f7e740fb2ba36d0fcf3fd196d60333552911e3a4))
 * **[#1505](https://github.com/hydro-project/hydro/issues/1505)**
    - Fixes for nightly clippy ([`47cb703`](https://github.com/hydro-project/hydro/commit/47cb703e771f7d1c451ceb9d185ada96410949da))
 * **Uncategorized**
    - Release hydroflow_lang v0.10.0, hydroflow_datalog_core v0.10.0, hydroflow_datalog v0.10.0, hydroflow_deploy_integration v0.10.0, hydroflow_macro v0.10.0, lattices_macro v0.5.7, variadics v0.0.7, variadics_macro v0.5.5, lattices v0.5.8, multiplatform_test v0.3.0, pusherator v0.0.9, hydroflow v0.10.0, hydro_deploy v0.10.0, stageleft_macro v0.4.0, stageleft v0.5.0, stageleft_tool v0.4.0, hydroflow_plus v0.10.0, hydro_cli v0.10.0, safety bump 8 crates ([`dcd48fc`](https://github.com/hydro-project/hydro/commit/dcd48fc7ee805898d9b5ef0d082870e30615e95b))
    - Ignore trybuild tests inconsistent on latest nightly ([`656ee32`](https://github.com/hydro-project/hydro/commit/656ee328c8710bce7370c851437a80ca3db46a5a))
</details>

<csr-unknown>
into_option (convert a variadic to a variadic of options)into_vec (convert a variadic to a variadic of vecs)<csr-unknown/>

## 0.0.6 (2024-08-30)

<csr-id-11af32828bab6e4a4264d2635ff71a12bb0bb778/>

### Chore

 - <csr-id-11af32828bab6e4a4264d2635ff71a12bb0bb778/> lower min dependency versions where possible, update `Cargo.lock`
   Moved from #1418
   
   ---------

### Bug Fixes

 - <csr-id-43ff49d72789d78535717d2db04cf595cc511274/> allow `PartialEqVariadic::eq_ref` to take `AsRefVar`s with different lifetimes
   Bug found while working on GHTs

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 38 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#1367](https://github.com/hydro-project/hydro/issues/1367), [#1423](https://github.com/hydro-project/hydro/issues/1423)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1367](https://github.com/hydro-project/hydro/issues/1367)**
    - Allow `PartialEqVariadic::eq_ref` to take `AsRefVar`s with different lifetimes ([`43ff49d`](https://github.com/hydro-project/hydro/commit/43ff49d72789d78535717d2db04cf595cc511274))
 * **[#1423](https://github.com/hydro-project/hydro/issues/1423)**
    - Lower min dependency versions where possible, update `Cargo.lock` ([`11af328`](https://github.com/hydro-project/hydro/commit/11af32828bab6e4a4264d2635ff71a12bb0bb778))
 * **Uncategorized**
    - Release hydroflow_lang v0.9.0, hydroflow_datalog_core v0.9.0, hydroflow_datalog v0.9.0, hydroflow_deploy_integration v0.9.0, hydroflow_macro v0.9.0, lattices_macro v0.5.6, lattices v0.5.7, multiplatform_test v0.2.0, variadics v0.0.6, pusherator v0.0.8, hydroflow v0.9.0, stageleft_macro v0.3.0, stageleft v0.4.0, stageleft_tool v0.3.0, hydroflow_plus v0.9.0, hydro_deploy v0.9.0, hydro_cli v0.9.0, hydroflow_plus_deploy v0.9.0, safety bump 8 crates ([`0750117`](https://github.com/hydro-project/hydro/commit/0750117de7088c01a439b102adeb4c832889f171))
</details>

## 0.0.5 (2024-07-23)

### New Features

 - <csr-id-20080cb7ceb5b5d3ba349dfd822a37288e40add6/> add traits for dealing with variadics of references
   Renames some traits, but not a breaking change since there hasn't been a
   release that includes those traits.
 - <csr-id-b92dfc7460c985db6935e79d612f42b9b87e746f/> add `iter_any_ref` and `iter_any_mut` to `VariadicsExt`
   Depends on #1241
   
   This isn't needed for the current GHT implementation, but is useful in
   general
 - <csr-id-1a6228f2db081af68890e2e64b3a91f15dd9214f/> add traits for referencing variadics
   This adds a way to convert a reference to a variadic into a variadic of
   references. I.e. `&var_expr!(a, b, c) -> var_expr!(&a, &b, &c)`

### Bug Fixes

 - <csr-id-bbef0705d509831415d3bb5ce003116af06b6ffb/> `EitherRefVariadic` is `Variadic`
 - <csr-id-c70114d836e5bc36e2104188867e548e90ab38f4/> fix `HomogenousVariadic` `get` and `get_mut` only returning `None`

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release.
 - 143 days passed between releases.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 5 unique issues were worked on: [#1241](https://github.com/hydro-project/hydro/issues/1241), [#1245](https://github.com/hydro-project/hydro/issues/1245), [#1324](https://github.com/hydro-project/hydro/issues/1324), [#1325](https://github.com/hydro-project/hydro/issues/1325), [#1352](https://github.com/hydro-project/hydro/issues/1352)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1241](https://github.com/hydro-project/hydro/issues/1241)**
    - Add traits for referencing variadics ([`1a6228f`](https://github.com/hydro-project/hydro/commit/1a6228f2db081af68890e2e64b3a91f15dd9214f))
 * **[#1245](https://github.com/hydro-project/hydro/issues/1245)**
    - Add `iter_any_ref` and `iter_any_mut` to `VariadicsExt` ([`b92dfc7`](https://github.com/hydro-project/hydro/commit/b92dfc7460c985db6935e79d612f42b9b87e746f))
 * **[#1324](https://github.com/hydro-project/hydro/issues/1324)**
    - Add traits for dealing with variadics of references ([`20080cb`](https://github.com/hydro-project/hydro/commit/20080cb7ceb5b5d3ba349dfd822a37288e40add6))
 * **[#1325](https://github.com/hydro-project/hydro/issues/1325)**
    - Fix `HomogenousVariadic` `get` and `get_mut` only returning `None` ([`c70114d`](https://github.com/hydro-project/hydro/commit/c70114d836e5bc36e2104188867e548e90ab38f4))
 * **[#1352](https://github.com/hydro-project/hydro/issues/1352)**
    - `EitherRefVariadic` is `Variadic` ([`bbef070`](https://github.com/hydro-project/hydro/commit/bbef0705d509831415d3bb5ce003116af06b6ffb))
 * **Uncategorized**
    - Release hydroflow_lang v0.8.0, hydroflow_datalog_core v0.8.0, hydroflow_datalog v0.8.0, hydroflow_macro v0.8.0, lattices_macro v0.5.5, lattices v0.5.6, variadics v0.0.5, pusherator v0.0.7, hydroflow v0.8.0, hydroflow_plus v0.8.0, hydro_deploy v0.8.0, hydro_cli v0.8.0, hydroflow_plus_cli_integration v0.8.0, safety bump 7 crates ([`ca6c16b`](https://github.com/hydro-project/hydro/commit/ca6c16b4a7ce35e155fe7fc6c7d1676c37c9e4de))
</details>

## 0.0.4 (2024-03-02)

<csr-id-5a451ac4ae75024153a06416fc81d834d1fdae6f/>
<csr-id-7103e77d0da1d73f1c93fcdb260b6a4c9a18ff66/>
<csr-id-b4683450a273d510a11338f07920a5558033b31f/>

### Chore

 - <csr-id-5a451ac4ae75024153a06416fc81d834d1fdae6f/> prep for 0.0.4 release
 - <csr-id-7103e77d0da1d73f1c93fcdb260b6a4c9a18ff66/> update pinned rust to 2024-04-24

### Style

 - <csr-id-b4683450a273d510a11338f07920a5558033b31f/> fix dead code lint

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release hydroflow_lang v0.6.0, hydroflow_datalog_core v0.6.0, hydroflow_datalog v0.6.0, hydroflow_macro v0.6.0, lattices v0.5.3, variadics v0.0.4, pusherator v0.0.5, hydroflow v0.6.0, stageleft v0.2.0, hydroflow_plus v0.6.0, hydro_deploy v0.6.0, hydro_cli v0.6.0, hydroflow_plus_cli_integration v0.6.0, safety bump 7 crates ([`09ea65f`](https://github.com/hydro-project/hydro/commit/09ea65fe9cd45c357c43bffca30e60243fa45cc8))
    - Prep for 0.0.4 release ([`5a451ac`](https://github.com/hydro-project/hydro/commit/5a451ac4ae75024153a06416fc81d834d1fdae6f))
    - Fix dead code lint ([`b468345`](https://github.com/hydro-project/hydro/commit/b4683450a273d510a11338f07920a5558033b31f))
    - Update pinned rust to 2024-04-24 ([`7103e77`](https://github.com/hydro-project/hydro/commit/7103e77d0da1d73f1c93fcdb260b6a4c9a18ff66))
</details>

## 0.0.3 (2024-01-29)

<csr-id-1b555e57c8c812bed4d6495d2960cbf77fb0b3ef/>
<csr-id-7e65a08711775656e435e854777c5f089dd31a05/>

### Chore

 - <csr-id-1b555e57c8c812bed4d6495d2960cbf77fb0b3ef/> manually set lockstep-versioned crates (and `lattices`) to version `0.5.1`
   Setting manually since
   https://github.com/frewsxcv/rust-crates-index/issues/159 is messing with
   smart-release

### Refactor

 - <csr-id-7e65a08711775656e435e854777c5f089dd31a05/> Improvements prepping for release
   - Adds the "spread"/"splat" `...` syntax to the three variadics macros.
   - Adds `#[sealed]` traits.
   - Adds testing of error messages.
   - Improves docs: `README.md` and Rust docs.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release over the course of 37 calendar days.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#974](https://github.com/hydro-project/hydro/issues/974)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#974](https://github.com/hydro-project/hydro/issues/974)**
    - Improvements prepping for release ([`7e65a08`](https://github.com/hydro-project/hydro/commit/7e65a08711775656e435e854777c5f089dd31a05))
 * **Uncategorized**
    - Release hydroflow_cli_integration v0.5.1, hydroflow_lang v0.5.1, hydroflow_datalog_core v0.5.1, hydroflow_datalog v0.5.1, hydroflow_macro v0.5.1, lattices v0.5.1, variadics v0.0.3, pusherator v0.0.4, hydroflow v0.5.1, stageleft_macro v0.1.0, stageleft v0.1.0, hydroflow_plus v0.5.1, hydro_deploy v0.5.1, hydro_cli v0.5.1 ([`478aebc`](https://github.com/hydro-project/hydro/commit/478aebc8fee2aa78eab86bd386322db1c70bde6a))
    - Manually set lockstep-versioned crates (and `lattices`) to version `0.5.1` ([`1b555e5`](https://github.com/hydro-project/hydro/commit/1b555e57c8c812bed4d6495d2960cbf77fb0b3ef))
</details>

## 0.0.2 (2023-05-21)

<csr-id-5a3c2949653685de1e33cf7412057a70880283df/>

### Style

 - <csr-id-5a3c2949653685de1e33cf7412057a70880283df/> rustfmt format code comments

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#660](https://github.com/hydro-project/hydro/issues/660)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#660](https://github.com/hydro-project/hydro/issues/660)**
    - Rustfmt format code comments ([`5a3c294`](https://github.com/hydro-project/hydro/commit/5a3c2949653685de1e33cf7412057a70880283df))
 * **Uncategorized**
    - Release hydroflow_cli_integration v0.0.1, hydroflow_lang v0.0.1, hydroflow_datalog_core v0.0.1, hydroflow_datalog v0.0.1, hydroflow_macro v0.0.1, lattices v0.1.0, variadics v0.0.2, pusherator v0.0.1, hydroflow v0.0.2 ([`809395a`](https://github.com/hydro-project/hydro/commit/809395acddb78949d7a2bf036e1a94972f23b1ad))
</details>

## 0.0.1 (2023-04-25)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#617](https://github.com/hydro-project/hydro/issues/617)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#617](https://github.com/hydro-project/hydro/issues/617)**
    - Update `Cargo.toml`s for publishing ([`a78ff9a`](https://github.com/hydro-project/hydro/commit/a78ff9aace6771787c2b72aad83be6ad8d49a828))
 * **Uncategorized**
    - Setup release workflow ([`108d0e9`](https://github.com/hydro-project/hydro/commit/108d0e933a08b183c4dadf8c3499e4946696e263))
    - Rename variadics/tuple_list macros ([`91d37b0`](https://github.com/hydro-project/hydro/commit/91d37b022b1cd0ed590765c40ef43244027c8035))
    - Rename pkg `type_list` -> `variadics` ([`50e7361`](https://github.com/hydro-project/hydro/commit/50e7361709cd34fd0e1cbf0c9a9f79343ee9c2e2))
</details>

