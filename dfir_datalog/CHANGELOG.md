# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.12.1 (2025-03-15)

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

## 0.12.0 (2025-03-08)

<csr-id-49a387d4a21f0763df8ec94de73fb953c9cd333a/>
<csr-id-44fb2806cf2d165d86695910f4755e0944c11832/>

### Chore

 - <csr-id-49a387d4a21f0763df8ec94de73fb953c9cd333a/> upgrade to Rust 2024 edition
   - Updates `Cargo.toml` to use new shared workspace keys
   - Updates lint settings (in workspace `Cargo.toml`)
   - `rustfmt` has changed slightly, resulting in a big diff - there are no
   actual code changes
   - Adds a script to `rustfmt` the template src files

### Chore (BREAKING)

 - <csr-id-3966d9063dae52e65b077321e0bd1150f2b0c3f1/> use DFIR name instead of Hydroflow in some places, fix #1644
   Fix partially #1712
   
   * Renames `WriteContextArgs.hydroflow` to `WriteContextArgs.df_ident`
   for DFIR operator codegen
   * Removes some dead code/files

### Chore

 - <csr-id-ec3795a678d261a38085405b6e9bfea943dafefb/> upgrade to Rust 2024 edition
   - Updates `Cargo.toml` to use new shared workspace keys
   - Updates lint settings (in workspace `Cargo.toml`)
   - `rustfmt` has changed slightly, resulting in a big diff - there are no
   actual code changes
   - Adds a script to `rustfmt` the template src files

### Documentation

 - <csr-id-f8313b018f6a1101935e4c06abbe5af3aafb400c/> fix broken links, fix #1613
 - <csr-id-d45273943b0ca087b05f0fe4331b12cbe2ff4e90/> fix broken links, fix #1613

### Chore (BREAKING)

 - <csr-id-44fb2806cf2d165d86695910f4755e0944c11832/> use DFIR name instead of Hydroflow in some places, fix #1644
   Fix partially #1712
   
   * Renames `WriteContextArgs.hydroflow` to `WriteContextArgs.df_ident`
   for DFIR operator codegen
   * Removes some dead code/files

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 74 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 3 unique issues were worked on: [#1686](https://github.com/hydro-project/hydro/issues/1686), [#1713](https://github.com/hydro-project/hydro/issues/1713), [#1747](https://github.com/hydro-project/hydro/issues/1747)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1686](https://github.com/hydro-project/hydro/issues/1686)**
    - Fix broken links, fix #1613 ([`d452739`](https://github.com/hydro-project/hydro/commit/d45273943b0ca087b05f0fe4331b12cbe2ff4e90))
 * **[#1713](https://github.com/hydro-project/hydro/issues/1713)**
    - Use DFIR name instead of Hydroflow in some places, fix #1644 ([`3966d90`](https://github.com/hydro-project/hydro/commit/3966d9063dae52e65b077321e0bd1150f2b0c3f1))
 * **[#1747](https://github.com/hydro-project/hydro/issues/1747)**
    - Upgrade to Rust 2024 edition ([`ec3795a`](https://github.com/hydro-project/hydro/commit/ec3795a678d261a38085405b6e9bfea943dafefb))
 * **Uncategorized**
    - Release dfir_lang v0.12.0, dfir_datalog_core v0.12.0, dfir_datalog v0.12.0, dfir_macro v0.12.0, hydroflow_deploy_integration v0.12.0, lattices_macro v0.5.9, variadics v0.0.9, variadics_macro v0.6.0, lattices v0.6.0, multiplatform_test v0.5.0, pusherator v0.0.11, dfir_rs v0.12.0, hydro_deploy v0.12.0, stageleft_macro v0.6.0, stageleft v0.7.0, stageleft_tool v0.6.0, hydro_lang v0.12.0, hydro_std v0.12.0, hydro_cli v0.12.0, safety bump 10 crates ([`973c925`](https://github.com/hydro-project/hydro/commit/973c925e87ed78344494581bd7ce1bbb4186a2f3))
</details>

## 0.11.0 (2024-12-23)

<csr-id-251b1039c71d45d3f86123dba1926026ded80824/>
<csr-id-5196f247e0124a31567af940541044ce1906cdc1/>
<csr-id-03b3a349013a71b324276bca5329c33d400a73ff/>
<csr-id-3291c07b37c9f9031837a2a32953e8f8854ec298/>

### Refactor (BREAKING)

 - <csr-id-251b1039c71d45d3f86123dba1926026ded80824/> use `cfg(nightly)` instead of feature, remove `-Z` flag, use `Diagnostic::try_emit`
   Previous PR (#1587) website build did not work because `panic = "abort"`
   is set on wasm, leading to aborts for `proc_macro2::Span::unwrap()`
   calls.
   
   All tests except trybuild seem to pass on stable, WIP #1587 next

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
 - <csr-id-c707659afe188a2b46b093ce3438f64c6b0e1e30/> fix some broken github tree/main links
 - <csr-id-e1a08e5d165fbc80da2ae695e507078a97a9031f/> update `CHANGELOG.md`s for big rename
   Generated before rename per `RELEASING.md` instructions.
 - <csr-id-6ab625273d822812e83a333e928c3dea1c3c9ccb/> cleanups for the rename, fixing links
 - <csr-id-204bd117ca3a8845b4986539efb91a0c612dfa05/> add `repository` field to `Cargo.toml`s, fix #1452
   #1452 
   
   Will trigger new releases of the following:
   `unchanged = 'hydroflow_deploy_integration', 'variadics',
   'variadics_macro', 'pusherator'`
   
   (All other crates already have changes, so would be released anyway)
 - <csr-id-a652ead6a51ffae9f835124dcd40aec58dd15ff4/> fix some broken github tree/main links
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

### `hydroflow_datalog` Commit Statistics

<csr-read-only-do-not-edit/>

 - 1 commit contributed to the release.
 - 38 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1606](https://github.com/hydro-project/hydroflow/issues/1606)

### `hydroflow_datalog` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1606](https://github.com/hydro-project/hydroflow/issues/1606)**
    - Use `cfg(nightly)` instead of feature, remove `-Z` flag, use `Diagnostic::try_emit` ([`251b103`](https://github.com/hydro-project/hydroflow/commit/251b1039c71d45d3f86123dba1926026ded80824))
</details>

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 8 commits contributed to the release.
 - 7 commits were understood as [conventional](https://www.conventionalcommits.org).
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
    - Fix some broken github tree/main links ([`a652ead`](https://github.com/hydro-project/hydro/commit/a652ead6a51ffae9f835124dcd40aec58dd15ff4))
    - Update `CHANGELOG.md`s for big rename ([`27c40e2`](https://github.com/hydro-project/hydro/commit/27c40e2ca5a822f6ebd31c7f01213aa6d407418a))
</details>

## 0.10.0 (2024-11-08)

<csr-id-d5677604e93c07a5392f4229af94a0b736eca382/>

### Chore

 - <csr-id-d5677604e93c07a5392f4229af94a0b736eca382/> update pinned rust version, clippy lints, remove some dead code

### `hydroflow_datalog` Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 69 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1444](https://github.com/hydro-project/hydroflow/issues/1444)

### `hydroflow_datalog` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1444](https://github.com/hydro-project/hydroflow/issues/1444)**
    - Update pinned rust version, clippy lints, remove some dead code ([`d567760`](https://github.com/hydro-project/hydroflow/commit/d5677604e93c07a5392f4229af94a0b736eca382))
 * **Uncategorized**
    - Release hydroflow_lang v0.10.0, hydroflow_datalog_core v0.10.0, hydroflow_datalog v0.10.0, hydroflow_deploy_integration v0.10.0, hydroflow_macro v0.10.0, lattices_macro v0.5.7, variadics v0.0.7, variadics_macro v0.5.5, lattices v0.5.8, multiplatform_test v0.3.0, pusherator v0.0.9, hydroflow v0.10.0, hydro_deploy v0.10.0, stageleft_macro v0.4.0, stageleft v0.5.0, stageleft_tool v0.4.0, hydroflow_plus v0.10.0, hydro_cli v0.10.0, safety bump 8 crates ([`dcd48fc`](https://github.com/hydro-project/hydroflow/commit/dcd48fc7ee805898d9b5ef0d082870e30615e95b))
</details>

## 0.9.0 (2024-08-30)

<csr-id-11af32828bab6e4a4264d2635ff71a12bb0bb778/>

### Chore

 - <csr-id-11af32828bab6e4a4264d2635ff71a12bb0bb778/> lower min dependency versions where possible, update `Cargo.lock`
   Moved from #1418
   
   ---------

### `hydroflow_datalog` Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 38 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1423](https://github.com/hydro-project/hydroflow/issues/1423)

### `hydroflow_datalog` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1423](https://github.com/hydro-project/hydroflow/issues/1423)**
    - Lower min dependency versions where possible, update `Cargo.lock` ([`11af328`](https://github.com/hydro-project/hydroflow/commit/11af32828bab6e4a4264d2635ff71a12bb0bb778))
 * **Uncategorized**
    - Release hydroflow_lang v0.9.0, hydroflow_datalog_core v0.9.0, hydroflow_datalog v0.9.0, hydroflow_deploy_integration v0.9.0, hydroflow_macro v0.9.0, lattices_macro v0.5.6, lattices v0.5.7, multiplatform_test v0.2.0, variadics v0.0.6, pusherator v0.0.8, hydroflow v0.9.0, stageleft_macro v0.3.0, stageleft v0.4.0, stageleft_tool v0.3.0, hydroflow_plus v0.9.0, hydro_deploy v0.9.0, hydro_cli v0.9.0, hydroflow_plus_deploy v0.9.0, safety bump 8 crates ([`0750117`](https://github.com/hydro-project/hydroflow/commit/0750117de7088c01a439b102adeb4c832889f171))
</details>

## 0.8.0 (2024-07-23)

<csr-id-186310f453f6a935ac5f53fdcaf07fe1337833bf/>

Unchanged from previous release.

### Chore

 - <csr-id-186310f453f6a935ac5f53fdcaf07fe1337833bf/> mark `hydroflow_datalog` and `hydroflow_macro` as unchanged for 0.8.0 release

### `hydroflow_datalog` Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 59 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### `hydroflow_datalog` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release hydroflow_lang v0.8.0, hydroflow_datalog_core v0.8.0, hydroflow_datalog v0.8.0, hydroflow_macro v0.8.0, lattices_macro v0.5.5, lattices v0.5.6, variadics v0.0.5, pusherator v0.0.7, hydroflow v0.8.0, hydroflow_plus v0.8.0, hydro_deploy v0.8.0, hydro_cli v0.8.0, hydroflow_plus_cli_integration v0.8.0, safety bump 7 crates ([`ca6c16b`](https://github.com/hydro-project/hydroflow/commit/ca6c16b4a7ce35e155fe7fc6c7d1676c37c9e4de))
    - Mark `hydroflow_datalog` and `hydroflow_macro` as unchanged for 0.8.0 release ([`186310f`](https://github.com/hydro-project/hydroflow/commit/186310f453f6a935ac5f53fdcaf07fe1337833bf))
</details>

## 0.7.0 (2024-05-24)

<csr-id-f21fe6f896a2eac2118fe5da9c71e051365473a6/>

Unchanged from previous release.

### Chore

 - <csr-id-f21fe6f896a2eac2118fe5da9c71e051365473a6/> mark `hydroflow_datalog` as unchanged for 0.7 release

### `hydroflow_datalog` Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 83 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### `hydroflow_datalog` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release hydroflow_lang v0.7.0, hydroflow_datalog_core v0.7.0, hydroflow_datalog v0.7.0, hydroflow_macro v0.7.0, lattices v0.5.5, multiplatform_test v0.1.0, pusherator v0.0.6, hydroflow v0.7.0, stageleft_macro v0.2.0, stageleft v0.3.0, stageleft_tool v0.2.0, hydroflow_plus v0.7.0, hydro_deploy v0.7.0, hydro_cli v0.7.0, hydroflow_plus_cli_integration v0.7.0, safety bump 8 crates ([`2852147`](https://github.com/hydro-project/hydroflow/commit/285214740627685e911781793e05d234ab2ad2bd))
    - Mark `hydroflow_datalog` as unchanged for 0.7 release ([`f21fe6f`](https://github.com/hydro-project/hydroflow/commit/f21fe6f896a2eac2118fe5da9c71e051365473a6))
</details>

## 0.6.0 (2024-03-02)

<csr-id-83cac6bb7fccd7589a5b3fcc36c465496b33bf2b/>

Unchanged from previous release.

### Chore

 - <csr-id-83cac6bb7fccd7589a5b3fcc36c465496b33bf2b/> mark hydroflow_datalog, hydroflow_macro as unchanged for release

### `hydroflow_datalog` Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 32 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### `hydroflow_datalog` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release hydroflow_lang v0.6.0, hydroflow_datalog_core v0.6.0, hydroflow_datalog v0.6.0, hydroflow_macro v0.6.0, lattices v0.5.3, variadics v0.0.4, pusherator v0.0.5, hydroflow v0.6.0, stageleft v0.2.0, hydroflow_plus v0.6.0, hydro_deploy v0.6.0, hydro_cli v0.6.0, hydroflow_plus_cli_integration v0.6.0, safety bump 7 crates ([`09ea65f`](https://github.com/hydro-project/hydroflow/commit/09ea65fe9cd45c357c43bffca30e60243fa45cc8))
    - Mark hydroflow_datalog, hydroflow_macro as unchanged for release ([`83cac6b`](https://github.com/hydro-project/hydroflow/commit/83cac6bb7fccd7589a5b3fcc36c465496b33bf2b))
</details>

## 0.5.1 (2024-01-29)

<csr-id-1b555e57c8c812bed4d6495d2960cbf77fb0b3ef/>

### Chore

 - <csr-id-1b555e57c8c812bed4d6495d2960cbf77fb0b3ef/> manually set lockstep-versioned crates (and `lattices`) to version `0.5.1`
   Setting manually since
   https://github.com/frewsxcv/rust-crates-index/issues/159 is messing with
   smart-release

### `hydroflow_datalog` Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 110 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### `hydroflow_datalog` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release hydroflow_cli_integration v0.5.1, hydroflow_lang v0.5.1, hydroflow_datalog_core v0.5.1, hydroflow_datalog v0.5.1, hydroflow_macro v0.5.1, lattices v0.5.1, variadics v0.0.3, pusherator v0.0.4, hydroflow v0.5.1, stageleft_macro v0.1.0, stageleft v0.1.0, hydroflow_plus v0.5.1, hydro_deploy v0.5.1, hydro_cli v0.5.1 ([`478aebc`](https://github.com/hydro-project/hydroflow/commit/478aebc8fee2aa78eab86bd386322db1c70bde6a))
    - Manually set lockstep-versioned crates (and `lattices`) to version `0.5.1` ([`1b555e5`](https://github.com/hydro-project/hydroflow/commit/1b555e57c8c812bed4d6495d2960cbf77fb0b3ef))
</details>

## 0.5.0 (2023-10-11)

<csr-id-f19eccc79d6d7c88de7ba1ef6a0abf1caaef377f/>

### Chore

 - <csr-id-f19eccc79d6d7c88de7ba1ef6a0abf1caaef377f/> bump proc-macro2 min version to 1.0.63

### `hydroflow_datalog` Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 56 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### `hydroflow_datalog` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release hydroflow_lang v0.5.0, hydroflow_datalog_core v0.5.0, hydroflow_datalog v0.5.0, hydroflow_macro v0.5.0, lattices v0.5.0, hydroflow v0.5.0, hydro_cli v0.5.0, safety bump 4 crates ([`2e2d8b3`](https://github.com/hydro-project/hydroflow/commit/2e2d8b386fb086c8276a2853d2a1f96ad4d7c221))
    - Bump proc-macro2 min version to 1.0.63 ([`f19eccc`](https://github.com/hydro-project/hydroflow/commit/f19eccc79d6d7c88de7ba1ef6a0abf1caaef377f))
</details>

## 0.4.0 (2023-08-15)

<csr-id-5faee64ab82eeb7a24f62a1b55c46d72d8eb5320/>

Unchanged from previous release.

### Chore

 - <csr-id-5faee64ab82eeb7a24f62a1b55c46d72d8eb5320/> mark hydro_datalog as unchanged for 0.4 release

### `hydroflow_datalog` Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 42 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### `hydroflow_datalog` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release hydroflow_lang v0.4.0, hydroflow_datalog_core v0.4.0, hydroflow_datalog v0.4.0, hydroflow_macro v0.4.0, lattices v0.4.0, pusherator v0.0.3, hydroflow v0.4.0, hydro_cli v0.4.0, safety bump 4 crates ([`cb313f0`](https://github.com/hydro-project/hydroflow/commit/cb313f0635214460a8308d05cbef4bf7f4bfaa15))
    - Mark hydro_datalog as unchanged for 0.4 release ([`5faee64`](https://github.com/hydro-project/hydroflow/commit/5faee64ab82eeb7a24f62a1b55c46d72d8eb5320))
</details>

## 0.3.0 (2023-07-04)

### New Features

 - <csr-id-22abcaff806c7de6e4a7725656bbcf201e7d9259/> allow stable build, refactors behind `nightly` feature flag

### Bug Fixes

 - <csr-id-8d3494b5afee858114a602a3e23077bb6d24dd77/> update proc-macro2, use new span location API where possible
   requires latest* rust nightly version
   
   *latest = 2023-06-28 or something

### `hydroflow_datalog` Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 33 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#780](https://github.com/hydro-project/hydroflow/issues/780), [#801](https://github.com/hydro-project/hydroflow/issues/801)

### `hydroflow_datalog` Commit Details

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

### `hydroflow_datalog` Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 7 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### `hydroflow_datalog` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release hydroflow_lang v0.2.0, hydroflow_datalog_core v0.2.0, hydroflow_datalog v0.2.0, hydroflow_macro v0.2.0, lattices v0.2.0, hydroflow v0.2.0, hydro_cli v0.2.0 ([`ca464c3`](https://github.com/hydro-project/hydroflow/commit/ca464c32322a7ad39eb53e1794777c849aa548a0))
    - Manually bump versions for v0.2.0 release ([`fd896fb`](https://github.com/hydro-project/hydroflow/commit/fd896fbe925fbd8ef1d16be7206ac20ba585081a))
</details>

## 0.1.0 (2023-05-23)

<csr-id-52ee8f8e443f0a8b5caf92d2c5f028c00302a79b/>

### Chore

 - <csr-id-52ee8f8e443f0a8b5caf92d2c5f028c00302a79b/> bump versions to 0.1.0 for release
   For release on crates.io for v0.1

### Documentation

 - <csr-id-a8957ec4457aae1cfd6fae031bede5e3f4fcc75d/> Add rustdocs to hydroflow's proc macros

### `hydroflow_datalog` Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 2 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#677](https://github.com/hydro-project/hydroflow/issues/677), [#684](https://github.com/hydro-project/hydroflow/issues/684)

### `hydroflow_datalog` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#677](https://github.com/hydro-project/hydroflow/issues/677)**
    - Add rustdocs to hydroflow's proc macros ([`a8957ec`](https://github.com/hydro-project/hydroflow/commit/a8957ec4457aae1cfd6fae031bede5e3f4fcc75d))
 * **[#684](https://github.com/hydro-project/hydroflow/issues/684)**
    - Bump versions to 0.1.0 for release ([`52ee8f8`](https://github.com/hydro-project/hydroflow/commit/52ee8f8e443f0a8b5caf92d2c5f028c00302a79b))
 * **Uncategorized**
    - Release hydroflow_cli_integration v0.1.0, hydroflow_internalmacro v0.1.0, hydroflow_lang v0.1.0, hydroflow_datalog_core v0.1.0, hydroflow_datalog v0.1.0, hydroflow_macro v0.1.0, lattices v0.1.1, hydroflow v0.1.0 ([`7324974`](https://github.com/hydro-project/hydroflow/commit/73249744293c9b89cbaa2d84b23ca3f25b00ae4e))
</details>

## 0.0.1 (2023-05-21)

<csr-id-20a1b2c0cd04a8b495a02ce345db3d48a99ea0e9/>

### Style

 - <csr-id-20a1b2c0cd04a8b495a02ce345db3d48a99ea0e9/> rustfmt group imports

### `hydroflow_datalog` Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 25 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#660](https://github.com/hydro-project/hydroflow/issues/660)

### `hydroflow_datalog` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#660](https://github.com/hydro-project/hydroflow/issues/660)**
    - Rustfmt group imports ([`20a1b2c`](https://github.com/hydro-project/hydroflow/commit/20a1b2c0cd04a8b495a02ce345db3d48a99ea0e9))
 * **Uncategorized**
    - Release hydroflow_cli_integration v0.0.1, hydroflow_lang v0.0.1, hydroflow_datalog_core v0.0.1, hydroflow_datalog v0.0.1, hydroflow_macro v0.0.1, lattices v0.1.0, variadics v0.0.2, pusherator v0.0.1, hydroflow v0.0.2 ([`809395a`](https://github.com/hydro-project/hydroflow/commit/809395acddb78949d7a2bf036e1a94972f23b1ad))
</details>

## 0.0.0 (2023-04-25)

### `hydroflow_datalog` Commit Statistics

<csr-read-only-do-not-edit/>

 - 34 commits contributed to the release.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 17 unique issues were worked on: [#155](https://github.com/hydro-project/hydroflow/issues/155), [#184](https://github.com/hydro-project/hydroflow/issues/184), [#187](https://github.com/hydro-project/hydroflow/issues/187), [#204](https://github.com/hydro-project/hydroflow/issues/204), [#223](https://github.com/hydro-project/hydroflow/issues/223), [#232](https://github.com/hydro-project/hydroflow/issues/232), [#284](https://github.com/hydro-project/hydroflow/issues/284), [#302](https://github.com/hydro-project/hydroflow/issues/302), [#320](https://github.com/hydro-project/hydroflow/issues/320), [#321](https://github.com/hydro-project/hydroflow/issues/321), [#329](https://github.com/hydro-project/hydroflow/issues/329), [#360](https://github.com/hydro-project/hydroflow/issues/360), [#371](https://github.com/hydro-project/hydroflow/issues/371), [#467](https://github.com/hydro-project/hydroflow/issues/467), [#518](https://github.com/hydro-project/hydroflow/issues/518), [#609](https://github.com/hydro-project/hydroflow/issues/609), [#617](https://github.com/hydro-project/hydroflow/issues/617)

### `hydroflow_datalog` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#155](https://github.com/hydro-project/hydroflow/issues/155)**
    - Add datalog frontend via a proc macro ([`fd3867f`](https://github.com/hydro-project/hydroflow/commit/fd3867fde4302aabd747ca81564dfba6016a6395))
 * **[#184](https://github.com/hydro-project/hydroflow/issues/184)**
    - Generate nested joins for rules with more than two RHS relations ([`863fdc8`](https://github.com/hydro-project/hydroflow/commit/863fdc8fea27d3b41dd3bd94212bee515a923340))
 * **[#187](https://github.com/hydro-project/hydroflow/issues/187)**
    - Emit relation filters when there are local constraints ([`28ed51b`](https://github.com/hydro-project/hydroflow/commit/28ed51bcd785a9098d42d4c1e6838c95831b42f4))
 * **[#204](https://github.com/hydro-project/hydroflow/issues/204)**
    - Use Rust Sitter release from crates.io ([`83ab8a5`](https://github.com/hydro-project/hydroflow/commit/83ab8a500c7aad0e4f82f95199954764ed67816f))
 * **[#223](https://github.com/hydro-project/hydroflow/issues/223)**
    - Add surface graph snapshot tests for datalog. ([`b235746`](https://github.com/hydro-project/hydroflow/commit/b2357466115dd2fe6257da01af855840f1ff33c9))
 * **[#232](https://github.com/hydro-project/hydroflow/issues/232)**
    - Extract parts of `expand_join_plan` into new functions. ([`3b79280`](https://github.com/hydro-project/hydroflow/commit/3b79280d900458b38be0cbc48c669465447f4873))
 * **[#284](https://github.com/hydro-project/hydroflow/issues/284)**
    - Rename source and dest surface syntax operators, fix #216 #276 ([`b7074eb`](https://github.com/hydro-project/hydroflow/commit/b7074ebb5d376493b52efe471b65f6e2c06fce7c))
 * **[#302](https://github.com/hydro-project/hydroflow/issues/302)**
    - Format `hydroflow_datalog` snaps w/ `prettyplease` ([`57be9a2`](https://github.com/hydro-project/hydroflow/commit/57be9a21c9b407155ef9418aec48156081ba141d))
 * **[#320](https://github.com/hydro-project/hydroflow/issues/320)**
    - Better mermaid graphs ([`f2ee139`](https://github.com/hydro-project/hydroflow/commit/f2ee139666da9ab72093dde80812df6bc7bc0193))
 * **[#321](https://github.com/hydro-project/hydroflow/issues/321)**
    - Better graphs for both mermaid and dot ([`876fb31`](https://github.com/hydro-project/hydroflow/commit/876fb3140374588c55b4a7ec7a51e7cf6317eb67))
 * **[#329](https://github.com/hydro-project/hydroflow/issues/329)**
    - Get hydroflow to compile to WASM ([`24354d2`](https://github.com/hydro-project/hydroflow/commit/24354d2e11c69e38e4e021aa4acf1525b376b2b1))
 * **[#360](https://github.com/hydro-project/hydroflow/issues/360)**
    - Preserve varnames info, display in mermaid, fix #327 ([`e7acecc`](https://github.com/hydro-project/hydroflow/commit/e7acecc480fbc2031e83777f58e7eb16603b8f26))
 * **[#371](https://github.com/hydro-project/hydroflow/issues/371)**
    - Get Datalog compiler to build on WASM ([`bef2435`](https://github.com/hydro-project/hydroflow/commit/bef24356a9696b494f89e014aec49063892b5b5e))
 * **[#467](https://github.com/hydro-project/hydroflow/issues/467)**
    - Parse error and return vector of diagnostics ([`1841f2c`](https://github.com/hydro-project/hydroflow/commit/1841f2c462a132272b1f0ffac51669fc1df2f593))
 * **[#518](https://github.com/hydro-project/hydroflow/issues/518)**
    - Attach spans to generated Hydroflow code in Dedalus ([`f00d865`](https://github.com/hydro-project/hydroflow/commit/f00d8655aa4404ddcc812e0decf8c1e48e62b0fd))
 * **[#609](https://github.com/hydro-project/hydroflow/issues/609)**
    - Update syn to 2.0 ([`2e7d802`](https://github.com/hydro-project/hydroflow/commit/2e7d8024f35893ef0abcb6851e370b00615f9562))
 * **[#617](https://github.com/hydro-project/hydroflow/issues/617)**
    - Update `Cargo.toml`s for publishing ([`a78ff9a`](https://github.com/hydro-project/hydroflow/commit/a78ff9aace6771787c2b72aad83be6ad8d49a828))
 * **Uncategorized**
    - Setup release workflow ([`108d0e9`](https://github.com/hydro-project/hydroflow/commit/108d0e933a08b183c4dadf8c3499e4946696e263))
    - Improve datalog diagnostic robustness ([`0b3e085`](https://github.com/hydro-project/hydroflow/commit/0b3e08521131989dfaee821c060a931771936f80))
    - Add persistence lifetimes to join #272 ([`47b2941`](https://github.com/hydro-project/hydroflow/commit/47b2941d74704792e5e2a7f30fa088c81c3ab506))
    - Add type guard before `Pivot` #263 ([`c215e8c`](https://github.com/hydro-project/hydroflow/commit/c215e8c4523a1e465eafa3320daa34d6cb35aa11))
    - Add type guard to `merge` #263 ([`6db3f60`](https://github.com/hydro-project/hydroflow/commit/6db3f6013a934b3087c8d116e61fbfc293e1baa0))
    - Emit type guards inline, configurable #263 ([`c6510da`](https://github.com/hydro-project/hydroflow/commit/c6510da4b4cb46ec026e3c1c69b5ce29b17c473c))
    - Add very good type guard to `join` op #263 ([`3ee9d33`](https://github.com/hydro-project/hydroflow/commit/3ee9d338c27859b31a057be53ee9251248ca235c))
    - Improve `Iterator`/`Pusherator` typeguards by erasing types, using local fns #263 ([`6413fa4`](https://github.com/hydro-project/hydroflow/commit/6413fa417cab0481e3db1adbcaf71525eb866cc9))
    - Rename variadics/tuple_list macros ([`91d37b0`](https://github.com/hydro-project/hydroflow/commit/91d37b022b1cd0ed590765c40ef43244027c8035))
    - Allow `clippy::uninlined-format-args` in `.cargo/config.toml` ([`17be5dd`](https://github.com/hydro-project/hydroflow/commit/17be5dd3993ee3239a3fbdb81572923479b0cc3e))
    - Add parsing of named ports (WIP, compiling) ([`bd8313c`](https://github.com/hydro-project/hydroflow/commit/bd8313cf59a30bb121c07d754099d92c13daa734))
    - Remove surface API, fix #224 ([`7b75f5e`](https://github.com/hydro-project/hydroflow/commit/7b75f5eb73046c3fe9f50970e05b4665bc0bf7fc))
    - Update datalog snapshots ([`6d9616e`](https://github.com/hydro-project/hydroflow/commit/6d9616e8740a98f16fbff84fa5b6e8295a1d9a15))
    - Update `recv_stream` to handle all `Stream`s instead of just `tokio::mpsc::unbounded_channel` ([`8b68c64`](https://github.com/hydro-project/hydroflow/commit/8b68c643b55e9a04f373bded939b512be4ee0d7f))
    - Use `DiMulGraph` in `flat_to_partitioned.rs` and `PartitionedGraph`, working ([`cdd45fe`](https://github.com/hydro-project/hydroflow/commit/cdd45fe8eeefaa997bc2d38386fb9d33daf47b50))
    - Update datalog codegen snapshots ([`9c9a27b`](https://github.com/hydro-project/hydroflow/commit/9c9a27b42c9855ab9d725214b68d66c6c273da2b))
    - Update datalog snapshot tests ([`c252b05`](https://github.com/hydro-project/hydroflow/commit/c252b0565bc86b37e5e25941ba1e9ed3c80d7863))
</details>

