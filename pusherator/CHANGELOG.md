# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.0.12 (2025-03-15)

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

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 1 commit contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1787](https://github.com/hydro-project/hydro/issues/1787)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1787](https://github.com/hydro-project/hydro/issues/1787)**
    - Demote python deploy docs, fix docsrs configs, fix #1392, fix #1629 ([`b235a42`](https://github.com/hydro-project/hydro/commit/b235a42a3071e55da7b09bdc8bc710b18e0fe053))
</details>

## 0.0.11 (2025-03-08)

<csr-id-49a387d4a21f0763df8ec94de73fb953c9cd333a/>

### Chore

 - <csr-id-49a387d4a21f0763df8ec94de73fb953c9cd333a/> upgrade to Rust 2024 edition
   - Updates `Cargo.toml` to use new shared workspace keys
   - Updates lint settings (in workspace `Cargo.toml`)
   - `rustfmt` has changed slightly, resulting in a big diff - there are no
   actual code changes
   - Adds a script to `rustfmt` the template src files

### Chore

 - <csr-id-ec3795a678d261a38085405b6e9bfea943dafefb/> upgrade to Rust 2024 edition
   - Updates `Cargo.toml` to use new shared workspace keys
   - Updates lint settings (in workspace `Cargo.toml`)
   - `rustfmt` has changed slightly, resulting in a big diff - there are no
   actual code changes
   - Adds a script to `rustfmt` the template src files

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 74 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1747](https://github.com/hydro-project/hydro/issues/1747)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1747](https://github.com/hydro-project/hydro/issues/1747)**
    - Upgrade to Rust 2024 edition ([`ec3795a`](https://github.com/hydro-project/hydro/commit/ec3795a678d261a38085405b6e9bfea943dafefb))
 * **Uncategorized**
    - Release dfir_lang v0.12.0, dfir_datalog_core v0.12.0, dfir_datalog v0.12.0, dfir_macro v0.12.0, hydroflow_deploy_integration v0.12.0, lattices_macro v0.5.9, variadics v0.0.9, variadics_macro v0.6.0, lattices v0.6.0, multiplatform_test v0.5.0, pusherator v0.0.11, dfir_rs v0.12.0, hydro_deploy v0.12.0, stageleft_macro v0.6.0, stageleft v0.7.0, stageleft_tool v0.6.0, hydro_lang v0.12.0, hydro_std v0.12.0, hydro_cli v0.12.0, safety bump 10 crates ([`973c925`](https://github.com/hydro-project/hydro/commit/973c925e87ed78344494581bd7ce1bbb4186a2f3))
</details>

## 0.0.10 (2024-12-23)

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
    - Add `repository` field to `Cargo.toml`s, fix #1452 ([`204bd11`](https://github.com/hydro-project/hydro/commit/204bd117ca3a8845b4986539efb91a0c612dfa05))
 * **Uncategorized**
    - Release dfir_lang v0.11.0, dfir_datalog_core v0.11.0, dfir_datalog v0.11.0, dfir_macro v0.11.0, hydroflow_deploy_integration v0.11.0, lattices_macro v0.5.8, variadics v0.0.8, variadics_macro v0.5.6, lattices v0.5.9, multiplatform_test v0.4.0, pusherator v0.0.10, dfir_rs v0.11.0, hydro_deploy v0.11.0, stageleft_macro v0.5.0, stageleft v0.6.0, stageleft_tool v0.5.0, hydro_lang v0.11.0, hydro_std v0.11.0, hydro_cli v0.11.0, safety bump 6 crates ([`361b443`](https://github.com/hydro-project/hydro/commit/361b4439ef9c781860f18d511668ab463a8c5203))
</details>

## 0.0.9 (2024-11-08)

<csr-id-d5677604e93c07a5392f4229af94a0b736eca382/>

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

## 0.0.8 (2024-08-30)

<csr-id-11af32828bab6e4a4264d2635ff71a12bb0bb778/>

### Chore

 - <csr-id-11af32828bab6e4a4264d2635ff71a12bb0bb778/> lower min dependency versions where possible, update `Cargo.lock`
   Moved from #1418
   
   ---------

### Chore

 - <csr-id-2c04f51f1ec44f7898307b6610371dcb490ea686/> lower min dependency versions where possible, update `Cargo.lock`
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
    - Lower min dependency versions where possible, update `Cargo.lock` ([`2c04f51`](https://github.com/hydro-project/hydro/commit/2c04f51f1ec44f7898307b6610371dcb490ea686))
 * **Uncategorized**
    - Release hydroflow_lang v0.9.0, hydroflow_datalog_core v0.9.0, hydroflow_datalog v0.9.0, hydroflow_deploy_integration v0.9.0, hydroflow_macro v0.9.0, lattices_macro v0.5.6, lattices v0.5.7, multiplatform_test v0.2.0, variadics v0.0.6, pusherator v0.0.8, hydroflow v0.9.0, stageleft_macro v0.3.0, stageleft v0.4.0, stageleft_tool v0.3.0, hydroflow_plus v0.9.0, hydro_deploy v0.9.0, hydro_cli v0.9.0, hydroflow_plus_deploy v0.9.0, safety bump 8 crates ([`1d54331`](https://github.com/hydro-project/hydro/commit/1d54331976040c049e4c97a9fba0e66930efee52))
</details>

## 0.0.7 (2024-07-23)

<csr-id-669beead61802bfc9db8ef628c690cba3aa93791/>

Unchanged from previous release.

### Chore

 - <csr-id-f5745d34164fa3c412753ffb82c0bf48180d719e/> mark `pusherators` as unchanged for release, to ensure version is updated

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
    - Release hydroflow_lang v0.8.0, hydroflow_datalog_core v0.8.0, hydroflow_datalog v0.8.0, hydroflow_macro v0.8.0, lattices_macro v0.5.5, lattices v0.5.6, variadics v0.0.5, pusherator v0.0.7, hydroflow v0.8.0, hydroflow_plus v0.8.0, hydro_deploy v0.8.0, hydro_cli v0.8.0, hydroflow_plus_cli_integration v0.8.0, safety bump 7 crates ([`7b9c367`](https://github.com/hydro-project/hydro/commit/7b9c3678930af8010f8e2ffd4069583ece528119))
    - Mark `pusherators` as unchanged for release, to ensure version is updated ([`f5745d3`](https://github.com/hydro-project/hydro/commit/f5745d34164fa3c412753ffb82c0bf48180d719e))
</details>

## 0.0.6 (2024-05-24)

<csr-id-826dbd9a709de2f883992bdcefa8f2d566d74ecb/>

### Refactor

 - <csr-id-826dbd9a709de2f883992bdcefa8f2d566d74ecb/> simplify `demux_enum()`, somewhat improves error messages #1201

### Refactor

 - <csr-id-ee904bad68f740c1e9c890f91ad82a4a408ff636/> simplify `demux_enum()`, somewhat improves error messages #1201

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
    - Simplify `demux_enum()`, somewhat improves error messages #1201 ([`ee904ba`](https://github.com/hydro-project/hydro/commit/ee904bad68f740c1e9c890f91ad82a4a408ff636))
 * **Uncategorized**
    - Release hydroflow_lang v0.7.0, hydroflow_datalog_core v0.7.0, hydroflow_datalog v0.7.0, hydroflow_macro v0.7.0, lattices v0.5.5, multiplatform_test v0.1.0, pusherator v0.0.6, hydroflow v0.7.0, stageleft_macro v0.2.0, stageleft v0.3.0, stageleft_tool v0.2.0, hydroflow_plus v0.7.0, hydro_deploy v0.7.0, hydro_cli v0.7.0, hydroflow_plus_cli_integration v0.7.0, safety bump 8 crates ([`855fda6`](https://github.com/hydro-project/hydro/commit/855fda65442ad7a9074a099ecc29e74322332418))
</details>

## 0.0.5 (2024-03-02)

<csr-id-5a451ac4ae75024153a06416fc81d834d1fdae6f/>

### Chore

 - <csr-id-5a451ac4ae75024153a06416fc81d834d1fdae6f/> prep for 0.0.4 release

### Chore

 - <csr-id-ae69ce53657104745764fd278153e965182223c4/> prep for 0.0.4 release

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release hydroflow_lang v0.6.0, hydroflow_datalog_core v0.6.0, hydroflow_datalog v0.6.0, hydroflow_macro v0.6.0, lattices v0.5.3, variadics v0.0.4, pusherator v0.0.5, hydroflow v0.6.0, stageleft v0.2.0, hydroflow_plus v0.6.0, hydro_deploy v0.6.0, hydro_cli v0.6.0, hydroflow_plus_cli_integration v0.6.0, safety bump 7 crates ([`0e94db4`](https://github.com/hydro-project/hydro/commit/0e94db41c842c1181574c5e69179027cfa7a19cf))
    - Prep for 0.0.4 release ([`ae69ce5`](https://github.com/hydro-project/hydro/commit/ae69ce53657104745764fd278153e965182223c4))
</details>

## 0.0.4 (2024-01-29)

<csr-id-1b555e57c8c812bed4d6495d2960cbf77fb0b3ef/>

### Chore

 - <csr-id-1b555e57c8c812bed4d6495d2960cbf77fb0b3ef/> manually set lockstep-versioned crates (and `lattices`) to version `0.5.1`
   Setting manually since
   https://github.com/frewsxcv/rust-crates-index/issues/159 is messing with
   smart-release

### Chore

 - <csr-id-7c48faf0d8301b498fa59e5eee5cddf5fa341229/> manually set lockstep-versioned crates (and `lattices`) to version `0.5.1`
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
    - Release hydroflow_cli_integration v0.5.1, hydroflow_lang v0.5.1, hydroflow_datalog_core v0.5.1, hydroflow_datalog v0.5.1, hydroflow_macro v0.5.1, lattices v0.5.1, variadics v0.0.3, pusherator v0.0.4, hydroflow v0.5.1, stageleft_macro v0.1.0, stageleft v0.1.0, hydroflow_plus v0.5.1, hydro_deploy v0.5.1, hydro_cli v0.5.1 ([`5a5e6d5`](https://github.com/hydro-project/hydro/commit/5a5e6d5933cf3c20ff23768d4592b0dde94e940b))
    - Manually set lockstep-versioned crates (and `lattices`) to version `0.5.1` ([`7c48faf`](https://github.com/hydro-project/hydro/commit/7c48faf0d8301b498fa59e5eee5cddf5fa341229))
</details>

## 0.0.3 (2023-08-15)

<csr-id-f60053f70da3071c54de4a0eabb059a143aa2ccc/>

### Chore

 - <csr-id-f60053f70da3071c54de4a0eabb059a143aa2ccc/> fix lint, format errors for latest nightly version (without updated pinned)
   For nightly version (d9c13cd45 2023-07-05)

### Chore

 - <csr-id-dd270adee8ed4d29a20628c4082b0f29cfd6ebac/> fix lint, format errors for latest nightly version (without updated pinned)
   For nightly version (d9c13cd45 2023-07-05)

### New Features

 - <csr-id-8f306e2a36582e168417808099eedf8a9de3b419/> rename assert => assert_eq, add assert, change underlying implementation to work across ticks
 - <csr-id-e32adff511c157548e606e2497c0f5a849fb429c/> rename assert => assert_eq, add assert, change underlying implementation to work across ticks

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
    - Fix lint, format errors for latest nightly version (without updated pinned) ([`dd270ad`](https://github.com/hydro-project/hydro/commit/dd270adee8ed4d29a20628c4082b0f29cfd6ebac))
 * **[#835](https://github.com/hydro-project/hydro/issues/835)**
    - Rename assert => assert_eq, add assert, change underlying implementation to work across ticks ([`e32adff`](https://github.com/hydro-project/hydro/commit/e32adff511c157548e606e2497c0f5a849fb429c))
 * **Uncategorized**
    - Release hydroflow_lang v0.4.0, hydroflow_datalog_core v0.4.0, hydroflow_datalog v0.4.0, hydroflow_macro v0.4.0, lattices v0.4.0, pusherator v0.0.3, hydroflow v0.4.0, hydro_cli v0.4.0, safety bump 4 crates ([`8d53ee5`](https://github.com/hydro-project/hydro/commit/8d53ee51686b41e403c2e91de23dfa7b8f9d1583))
</details>

## 0.0.2 (2023-07-04)

### Bug Fixes

 - <csr-id-a3c1fbbd1e3fa7a7299878f61b4bfd12dce0052c/> remove nightly feature `never_type` where unused
 - <csr-id-9bb5528d99e83fdae5aeca9456802379131c2f90/> removed unused nightly features `impl_trait_in_assoc_type`, `type_alias_impl_trait`
 - <csr-id-be328356a36e7c2b52b57fc4ef8e8f3ebc2bdff7/> remove nightly feature `never_type` where unused
 - <csr-id-902d426dfec7754cbe949d80c669e3d3f1a1d262/> removed unused nightly features `impl_trait_in_assoc_type`, `type_alias_impl_trait`

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
    - Remove nightly feature `never_type` where unused ([`be32835`](https://github.com/hydro-project/hydro/commit/be328356a36e7c2b52b57fc4ef8e8f3ebc2bdff7))
    - Removed unused nightly features `impl_trait_in_assoc_type`, `type_alias_impl_trait` ([`902d426`](https://github.com/hydro-project/hydro/commit/902d426dfec7754cbe949d80c669e3d3f1a1d262))
 * **Uncategorized**
    - Release hydroflow_cli_integration v0.3.0, hydroflow_lang v0.3.0, hydroflow_datalog_core v0.3.0, hydroflow_datalog v0.3.0, hydroflow_macro v0.3.0, lattices v0.3.0, pusherator v0.0.2, hydroflow v0.3.0, hydro_cli v0.3.0, safety bump 5 crates ([`c1ac8a0`](https://github.com/hydro-project/hydro/commit/c1ac8a0c95d4fee82fa55c0c4273091d168f8b86))
</details>

## 0.0.1 (2023-05-21)

<csr-id-20a1b2c0cd04a8b495a02ce345db3d48a99ea0e9/>
<csr-id-1eda91a2ef8794711ef037240f15284e8085d863/>

### Style

 - <csr-id-20a1b2c0cd04a8b495a02ce345db3d48a99ea0e9/> rustfmt group imports
 - <csr-id-1eda91a2ef8794711ef037240f15284e8085d863/> rustfmt prescribe flat-module `use` format

### Style

 - <csr-id-21a503e795593173b1fd114d70a7cfad3e79ecfe/> rustfmt group imports
 - <csr-id-2a144a622682a958d44377df71a71b59cf1b39c4/> rustfmt prescribe flat-module `use` format

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#660](https://github.com/hydro-project/hydro/issues/660)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#660](https://github.com/hydro-project/hydro/issues/660)**
    - Rustfmt group imports ([`21a503e`](https://github.com/hydro-project/hydro/commit/21a503e795593173b1fd114d70a7cfad3e79ecfe))
    - Rustfmt prescribe flat-module `use` format ([`2a144a6`](https://github.com/hydro-project/hydro/commit/2a144a622682a958d44377df71a71b59cf1b39c4))
 * **Uncategorized**
    - Release hydroflow_cli_integration v0.0.1, hydroflow_lang v0.0.1, hydroflow_datalog_core v0.0.1, hydroflow_datalog v0.0.1, hydroflow_macro v0.0.1, lattices v0.1.0, variadics v0.0.2, pusherator v0.0.1, hydroflow v0.0.2 ([`d91ebc9`](https://github.com/hydro-project/hydro/commit/d91ebc9e8e23965089c929558a09fc430ee72f2c))
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
    - Add split and switch pusherators ([`b756ab9`](https://github.com/hydro-project/hydro/commit/b756ab9a2d81bf7dcc0640043ca19b28ffe18be1))
 * **[#259](https://github.com/hydro-project/hydro/issues/259)**
    - Rename split->unzip, implement surface op ([`d6d4648`](https://github.com/hydro-project/hydro/commit/d6d464830008f1ab4c3f52b70c11a155a235e86e))
 * **[#261](https://github.com/hydro-project/hydro/issues/261)**
    - Add demux operator ([`0f5e95e`](https://github.com/hydro-project/hydro/commit/0f5e95ed1a338e1df1144f7392e5cfc22653581c))
 * **[#617](https://github.com/hydro-project/hydro/issues/617)**
    - Update `Cargo.toml`s for publishing ([`3a08bb2`](https://github.com/hydro-project/hydro/commit/3a08bb2501482323e069c6c1f808d611ac679f1f))
 * **Uncategorized**
    - Setup release workflow ([`f4eb56d`](https://github.com/hydro-project/hydro/commit/f4eb56dacebe96a92cb7448bcce14b8b5093c9d5))
    - Rename variadics/tuple_list macros ([`d443697`](https://github.com/hydro-project/hydro/commit/d4436975b85542bd62e862fdcefcd7249f5a732e))
    - Rename pkg `type_list` -> `variadics` ([`30777e2`](https://github.com/hydro-project/hydro/commit/30777e2608c72a3353733ef353373914b79407e2))
    - Implement `inspect()` surface syntax operator, fix #208 ([`f83815b`](https://github.com/hydro-project/hydro/commit/f83815b82cda484e11fc0bac3b2c56721ee8fc4c))
    - Remove `#![feature(generic_associated_types)]` b/c stabilization! ([`ee54fdc`](https://github.com/hydro-project/hydro/commit/ee54fdc003943c2854444aa68e12c265b9024686))
    - Standardizing pusherators, implement wrap specs ([`ee363cd`](https://github.com/hydro-project/hydro/commit/ee363cd17eb69183a66cfaa2eee83e7afa017e04))
    - Refactor for foundation of properties iterators ([`dab9af9`](https://github.com/hydro-project/hydro/commit/dab9af9c37d5ac9e1adaf2cb6466f084de9c8a51))
</details>

