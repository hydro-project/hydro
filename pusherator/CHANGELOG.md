# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.0.11 (2025-03-08)

### Chore

 - <csr-id-49a387d4a21f0763df8ec94de73fb953c9cd333a/> upgrade to Rust 2024 edition
   - Updates `Cargo.toml` to use new shared workspace keys
   - Updates lint settings (in workspace `Cargo.toml`)
   - `rustfmt` has changed slightly, resulting in a big diff - there are no
   actual code changes
   - Adds a script to `rustfmt` the template src files

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 1 commit contributed to the release over the course of 8 calendar days.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1747](https://github.com/hydro-project/hydro/issues/1747)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1747](https://github.com/hydro-project/hydro/issues/1747)**
    - Upgrade to Rust 2024 edition ([`49a387d`](https://github.com/hydro-project/hydro/commit/49a387d4a21f0763df8ec94de73fb953c9cd333a))
</details>

## 0.0.10 (2024-12-23)

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

## 0.0.9 (2024-11-08)

<csr-id-d5677604e93c07a5392f4229af94a0b736eca382/>

### Chore

 - <csr-id-d5677604e93c07a5392f4229af94a0b736eca382/> update pinned rust version, clippy lints, remove some dead code

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
    - Update pinned rust version, clippy lints, remove some dead code ([`d567760`](https://github.com/hydro-project/hydro/commit/d5677604e93c07a5392f4229af94a0b736eca382))
 * **Uncategorized**
    - Release hydroflow_lang v0.10.0, hydroflow_datalog_core v0.10.0, hydroflow_datalog v0.10.0, hydroflow_deploy_integration v0.10.0, hydroflow_macro v0.10.0, lattices_macro v0.5.7, variadics v0.0.7, variadics_macro v0.5.5, lattices v0.5.8, multiplatform_test v0.3.0, pusherator v0.0.9, hydroflow v0.10.0, hydro_deploy v0.10.0, stageleft_macro v0.4.0, stageleft v0.5.0, stageleft_tool v0.4.0, hydroflow_plus v0.10.0, hydro_cli v0.10.0, safety bump 8 crates ([`dcd48fc`](https://github.com/hydro-project/hydro/commit/dcd48fc7ee805898d9b5ef0d082870e30615e95b))
</details>

## 0.0.8 (2024-08-30)

<csr-id-11af32828bab6e4a4264d2635ff71a12bb0bb778/>

### Chore

 - <csr-id-11af32828bab6e4a4264d2635ff71a12bb0bb778/> lower min dependency versions where possible, update `Cargo.lock`
   Moved from #1418
   
   ---------

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 38 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1423](https://github.com/hydro-project/hydro/issues/1423)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1423](https://github.com/hydro-project/hydro/issues/1423)**
    - Lower min dependency versions where possible, update `Cargo.lock` ([`11af328`](https://github.com/hydro-project/hydro/commit/11af32828bab6e4a4264d2635ff71a12bb0bb778))
 * **Uncategorized**
    - Release hydroflow_lang v0.9.0, hydroflow_datalog_core v0.9.0, hydroflow_datalog v0.9.0, hydroflow_deploy_integration v0.9.0, hydroflow_macro v0.9.0, lattices_macro v0.5.6, lattices v0.5.7, multiplatform_test v0.2.0, variadics v0.0.6, pusherator v0.0.8, hydroflow v0.9.0, stageleft_macro v0.3.0, stageleft v0.4.0, stageleft_tool v0.3.0, hydroflow_plus v0.9.0, hydro_deploy v0.9.0, hydro_cli v0.9.0, hydroflow_plus_deploy v0.9.0, safety bump 8 crates ([`0750117`](https://github.com/hydro-project/hydro/commit/0750117de7088c01a439b102adeb4c832889f171))
</details>

## 0.0.7 (2024-07-23)

<csr-id-669beead61802bfc9db8ef628c690cba3aa93791/>

Unchanged from previous release.

### Chore

 - <csr-id-669beead61802bfc9db8ef628c690cba3aa93791/> mark `pusherators` as unchanged for release, to ensure version is updated

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 59 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release hydroflow_lang v0.8.0, hydroflow_datalog_core v0.8.0, hydroflow_datalog v0.8.0, hydroflow_macro v0.8.0, lattices_macro v0.5.5, lattices v0.5.6, variadics v0.0.5, pusherator v0.0.7, hydroflow v0.8.0, hydroflow_plus v0.8.0, hydro_deploy v0.8.0, hydro_cli v0.8.0, hydroflow_plus_cli_integration v0.8.0, safety bump 7 crates ([`ca6c16b`](https://github.com/hydro-project/hydro/commit/ca6c16b4a7ce35e155fe7fc6c7d1676c37c9e4de))
    - Mark `pusherators` as unchanged for release, to ensure version is updated ([`669beea`](https://github.com/hydro-project/hydro/commit/669beead61802bfc9db8ef628c690cba3aa93791))
</details>

## 0.0.6 (2024-05-24)

<csr-id-826dbd9a709de2f883992bdcefa8f2d566d74ecb/>

### Refactor

 - <csr-id-826dbd9a709de2f883992bdcefa8f2d566d74ecb/> simplify `demux_enum()`, somewhat improves error messages #1201

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 83 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1204](https://github.com/hydro-project/hydro/issues/1204)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1204](https://github.com/hydro-project/hydro/issues/1204)**
    - Simplify `demux_enum()`, somewhat improves error messages #1201 ([`826dbd9`](https://github.com/hydro-project/hydro/commit/826dbd9a709de2f883992bdcefa8f2d566d74ecb))
 * **Uncategorized**
    - Release hydroflow_lang v0.7.0, hydroflow_datalog_core v0.7.0, hydroflow_datalog v0.7.0, hydroflow_macro v0.7.0, lattices v0.5.5, multiplatform_test v0.1.0, pusherator v0.0.6, hydroflow v0.7.0, stageleft_macro v0.2.0, stageleft v0.3.0, stageleft_tool v0.2.0, hydroflow_plus v0.7.0, hydro_deploy v0.7.0, hydro_cli v0.7.0, hydroflow_plus_cli_integration v0.7.0, safety bump 8 crates ([`2852147`](https://github.com/hydro-project/hydro/commit/285214740627685e911781793e05d234ab2ad2bd))
</details>

## 0.0.5 (2024-03-02)

<csr-id-5a451ac4ae75024153a06416fc81d834d1fdae6f/>

### Chore

 - <csr-id-5a451ac4ae75024153a06416fc81d834d1fdae6f/> prep for 0.0.4 release

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release hydroflow_lang v0.6.0, hydroflow_datalog_core v0.6.0, hydroflow_datalog v0.6.0, hydroflow_macro v0.6.0, lattices v0.5.3, variadics v0.0.4, pusherator v0.0.5, hydroflow v0.6.0, stageleft v0.2.0, hydroflow_plus v0.6.0, hydro_deploy v0.6.0, hydro_cli v0.6.0, hydroflow_plus_cli_integration v0.6.0, safety bump 7 crates ([`09ea65f`](https://github.com/hydro-project/hydro/commit/09ea65fe9cd45c357c43bffca30e60243fa45cc8))
    - Prep for 0.0.4 release ([`5a451ac`](https://github.com/hydro-project/hydro/commit/5a451ac4ae75024153a06416fc81d834d1fdae6f))
</details>

## 0.0.4 (2024-01-29)

<csr-id-1b555e57c8c812bed4d6495d2960cbf77fb0b3ef/>

### Chore

 - <csr-id-1b555e57c8c812bed4d6495d2960cbf77fb0b3ef/> manually set lockstep-versioned crates (and `lattices`) to version `0.5.1`
   Setting manually since
   https://github.com/frewsxcv/rust-crates-index/issues/159 is messing with
   smart-release

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release hydroflow_cli_integration v0.5.1, hydroflow_lang v0.5.1, hydroflow_datalog_core v0.5.1, hydroflow_datalog v0.5.1, hydroflow_macro v0.5.1, lattices v0.5.1, variadics v0.0.3, pusherator v0.0.4, hydroflow v0.5.1, stageleft_macro v0.1.0, stageleft v0.1.0, hydroflow_plus v0.5.1, hydro_deploy v0.5.1, hydro_cli v0.5.1 ([`478aebc`](https://github.com/hydro-project/hydro/commit/478aebc8fee2aa78eab86bd386322db1c70bde6a))
    - Manually set lockstep-versioned crates (and `lattices`) to version `0.5.1` ([`1b555e5`](https://github.com/hydro-project/hydro/commit/1b555e57c8c812bed4d6495d2960cbf77fb0b3ef))
</details>

## 0.0.3 (2023-08-15)

<csr-id-f60053f70da3071c54de4a0eabb059a143aa2ccc/>

### Chore

 - <csr-id-f60053f70da3071c54de4a0eabb059a143aa2ccc/> fix lint, format errors for latest nightly version (without updated pinned)
   For nightly version (d9c13cd45 2023-07-05)

### New Features

 - <csr-id-8f306e2a36582e168417808099eedf8a9de3b419/> rename assert => assert_eq, add assert, change underlying implementation to work across ticks

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 42 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#822](https://github.com/hydro-project/hydro/issues/822), [#835](https://github.com/hydro-project/hydro/issues/835)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#822](https://github.com/hydro-project/hydro/issues/822)**
    - Fix lint, format errors for latest nightly version (without updated pinned) ([`f60053f`](https://github.com/hydro-project/hydro/commit/f60053f70da3071c54de4a0eabb059a143aa2ccc))
 * **[#835](https://github.com/hydro-project/hydro/issues/835)**
    - Rename assert => assert_eq, add assert, change underlying implementation to work across ticks ([`8f306e2`](https://github.com/hydro-project/hydro/commit/8f306e2a36582e168417808099eedf8a9de3b419))
 * **Uncategorized**
    - Release hydroflow_lang v0.4.0, hydroflow_datalog_core v0.4.0, hydroflow_datalog v0.4.0, hydroflow_macro v0.4.0, lattices v0.4.0, pusherator v0.0.3, hydroflow v0.4.0, hydro_cli v0.4.0, safety bump 4 crates ([`cb313f0`](https://github.com/hydro-project/hydro/commit/cb313f0635214460a8308d05cbef4bf7f4bfaa15))
</details>

## 0.0.2 (2023-07-04)

### Bug Fixes

 - <csr-id-a3c1fbbd1e3fa7a7299878f61b4bfd12dce0052c/> remove nightly feature `never_type` where unused
 - <csr-id-9bb5528d99e83fdae5aeca9456802379131c2f90/> removed unused nightly features `impl_trait_in_assoc_type`, `type_alias_impl_trait`

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 44 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#780](https://github.com/hydro-project/hydro/issues/780)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#780](https://github.com/hydro-project/hydro/issues/780)**
    - Remove nightly feature `never_type` where unused ([`a3c1fbb`](https://github.com/hydro-project/hydro/commit/a3c1fbbd1e3fa7a7299878f61b4bfd12dce0052c))
    - Removed unused nightly features `impl_trait_in_assoc_type`, `type_alias_impl_trait` ([`9bb5528`](https://github.com/hydro-project/hydro/commit/9bb5528d99e83fdae5aeca9456802379131c2f90))
 * **Uncategorized**
    - Release hydroflow_cli_integration v0.3.0, hydroflow_lang v0.3.0, hydroflow_datalog_core v0.3.0, hydroflow_datalog v0.3.0, hydroflow_macro v0.3.0, lattices v0.3.0, pusherator v0.0.2, hydroflow v0.3.0, hydro_cli v0.3.0, safety bump 5 crates ([`ec9633e`](https://github.com/hydro-project/hydro/commit/ec9633e2e393c2bf106223abeb0b680200fbdf84))
</details>

## 0.0.1 (2023-05-21)

<csr-id-20a1b2c0cd04a8b495a02ce345db3d48a99ea0e9/>
<csr-id-1eda91a2ef8794711ef037240f15284e8085d863/>

### Style

 - <csr-id-20a1b2c0cd04a8b495a02ce345db3d48a99ea0e9/> rustfmt group imports
 - <csr-id-1eda91a2ef8794711ef037240f15284e8085d863/> rustfmt prescribe flat-module `use` format

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#660](https://github.com/hydro-project/hydro/issues/660)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#660](https://github.com/hydro-project/hydro/issues/660)**
    - Rustfmt group imports ([`20a1b2c`](https://github.com/hydro-project/hydro/commit/20a1b2c0cd04a8b495a02ce345db3d48a99ea0e9))
    - Rustfmt prescribe flat-module `use` format ([`1eda91a`](https://github.com/hydro-project/hydro/commit/1eda91a2ef8794711ef037240f15284e8085d863))
 * **Uncategorized**
    - Release hydroflow_cli_integration v0.0.1, hydroflow_lang v0.0.1, hydroflow_datalog_core v0.0.1, hydroflow_datalog v0.0.1, hydroflow_macro v0.0.1, lattices v0.1.0, variadics v0.0.2, pusherator v0.0.1, hydroflow v0.0.2 ([`809395a`](https://github.com/hydro-project/hydro/commit/809395acddb78949d7a2bf036e1a94972f23b1ad))
</details>

## 0.0.0 (2023-04-25)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 11 commits contributed to the release over the course of 239 calendar days.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 4 unique issues were worked on: [#233](https://github.com/hydro-project/hydro/issues/233), [#259](https://github.com/hydro-project/hydro/issues/259), [#261](https://github.com/hydro-project/hydro/issues/261), [#617](https://github.com/hydro-project/hydro/issues/617)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#233](https://github.com/hydro-project/hydro/issues/233)**
    - Add split and switch pusherators ([`016b971`](https://github.com/hydro-project/hydro/commit/016b97112e3f417a81b333f988367d9689d0ce55))
 * **[#259](https://github.com/hydro-project/hydro/issues/259)**
    - Rename split->unzip, implement surface op ([`293c37c`](https://github.com/hydro-project/hydro/commit/293c37cd477c88af4ff0a3aaeb15a2da30ea391b))
 * **[#261](https://github.com/hydro-project/hydro/issues/261)**
    - Add demux operator ([`d07e5c1`](https://github.com/hydro-project/hydro/commit/d07e5c16be1bf3de627cd0f45225146129a6ab41))
 * **[#617](https://github.com/hydro-project/hydro/issues/617)**
    - Update `Cargo.toml`s for publishing ([`a78ff9a`](https://github.com/hydro-project/hydro/commit/a78ff9aace6771787c2b72aad83be6ad8d49a828))
 * **Uncategorized**
    - Setup release workflow ([`108d0e9`](https://github.com/hydro-project/hydro/commit/108d0e933a08b183c4dadf8c3499e4946696e263))
    - Rename variadics/tuple_list macros ([`91d37b0`](https://github.com/hydro-project/hydro/commit/91d37b022b1cd0ed590765c40ef43244027c8035))
    - Rename pkg `type_list` -> `variadics` ([`50e7361`](https://github.com/hydro-project/hydro/commit/50e7361709cd34fd0e1cbf0c9a9f79343ee9c2e2))
    - Implement `inspect()` surface syntax operator, fix #208 ([`7797c6c`](https://github.com/hydro-project/hydro/commit/7797c6c4aff07f780069bb9af2b12b8999b33725))
    - Remove `#![feature(generic_associated_types)]` b/c stabilization! ([`4786baf`](https://github.com/hydro-project/hydro/commit/4786baf69197a7254ffe1aea964238cbed9d755e))
    - Standardizing pusherators, implement wrap specs ([`f4081f8`](https://github.com/hydro-project/hydro/commit/f4081f8ee7cc6f308136c68f8015309106c43a5a))
    - Refactor for foundation of properties iterators ([`a14c439`](https://github.com/hydro-project/hydro/commit/a14c439f82f5811299c352c1eb7508f6c18839ce))
</details>

