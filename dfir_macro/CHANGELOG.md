# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.14.0 (2025-07-30)

### Chore

 - <csr-id-3d40d1a65c41dca3893867fb567993a27491fa0c/> update `proc-macro-crate`
 - <csr-id-98baec71a6f1d01d55a3c983fdbb7824c45305cd/> update pinned nightly to 2025-04-27, update span API usage

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release over the course of 92 calendar days.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#1843](https://github.com/hydro-project/hydro/issues/1843), [#1944](https://github.com/hydro-project/hydro/issues/1944)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1843](https://github.com/hydro-project/hydro/issues/1843)**
    - Update pinned nightly to 2025-04-27, update span API usage ([`98baec7`](https://github.com/hydro-project/hydro/commit/98baec71a6f1d01d55a3c983fdbb7824c45305cd))
 * **[#1944](https://github.com/hydro-project/hydro/issues/1944)**
    - Update `proc-macro-crate` ([`3d40d1a`](https://github.com/hydro-project/hydro/commit/3d40d1a65c41dca3893867fb567993a27491fa0c))
</details>

## 0.13.0 (2025-04-11)

<csr-id-7f3ec9dcce0ef9d52af03083970c8d26b9993fc0/>

### Other

 - <csr-id-7f3ec9dcce0ef9d52af03083970c8d26b9993fc0/> fix `docsrs` broken stable test, actually run `dfir_lang` op doc tests

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 27 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1790](https://github.com/hydro-project/hydro/issues/1790)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1790](https://github.com/hydro-project/hydro/issues/1790)**
    - Fix `docsrs` broken stable test, actually run `dfir_lang` op doc tests ([`7f3ec9d`](https://github.com/hydro-project/hydro/commit/7f3ec9dcce0ef9d52af03083970c8d26b9993fc0))
 * **Uncategorized**
    - Release dfir_lang v0.13.0, dfir_datalog_core v0.13.0, dfir_datalog v0.13.0, dfir_macro v0.13.0, hydro_deploy_integration v0.13.0, dfir_rs v0.13.0, hydro_deploy v0.13.0, hydro_lang v0.13.0, hydro_std v0.13.0, hydro_cli v0.13.0, safety bump 8 crates ([`400fd8f`](https://github.com/hydro-project/hydro/commit/400fd8f2e8cada253f54980e7edce0631be70a82))
</details>

## 0.12.1 (2025-03-15)

<csr-id-8058d2d6fe45e0286feb8ad48a44b1228f56d9bc/>
<csr-id-056ac62611319b7bd10a751d7e231423a1b8dc4e/>

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

### Refactor

 - <csr-id-8058d2d6fe45e0286feb8ad48a44b1228f56d9bc/> move markdown doctesting macro into own crate, fix #1651

### Style

 - <csr-id-056ac62611319b7bd10a751d7e231423a1b8dc4e/> cleanup old clippy lints, remove deprecated `relalg` crate

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 6 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 3 unique issues were worked on: [#1769](https://github.com/hydro-project/hydro/issues/1769), [#1785](https://github.com/hydro-project/hydro/issues/1785), [#1787](https://github.com/hydro-project/hydro/issues/1787)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1769](https://github.com/hydro-project/hydro/issues/1769)**
    - Move markdown doctesting macro into own crate, fix #1651 ([`8058d2d`](https://github.com/hydro-project/hydro/commit/8058d2d6fe45e0286feb8ad48a44b1228f56d9bc))
 * **[#1785](https://github.com/hydro-project/hydro/issues/1785)**
    - Cleanup old clippy lints, remove deprecated `relalg` crate ([`056ac62`](https://github.com/hydro-project/hydro/commit/056ac62611319b7bd10a751d7e231423a1b8dc4e))
 * **[#1787](https://github.com/hydro-project/hydro/issues/1787)**
    - Demote python deploy docs, fix docsrs configs, fix #1392, fix #1629 ([`b235a42`](https://github.com/hydro-project/hydro/commit/b235a42a3071e55da7b09bdc8bc710b18e0fe053))
 * **Uncategorized**
    - Release dfir_lang v0.12.1, dfir_datalog_core v0.12.1, dfir_datalog v0.12.1, dfir_macro v0.12.1, hydro_deploy_integration v0.12.1, lattices v0.6.1, pusherator v0.0.12, dfir_rs v0.12.1, hydro_deploy v0.12.1, hydro_lang v0.12.1, hydro_std v0.12.1, hydro_cli v0.12.1 ([`23221b5`](https://github.com/hydro-project/hydro/commit/23221b53b30918707ddaa85529d04cd7919166b4))
</details>

## 0.12.0 (2025-03-08)

<csr-id-49a387d4a21f0763df8ec94de73fb953c9cd333a/>
<csr-id-3343fe2d58d8b7a7aa2766bfba9fbb4955114706/>
<csr-id-44fb2806cf2d165d86695910f4755e0944c11832/>
<csr-id-3966d9063dae52e65b077321e0bd1150f2b0c3f1/>
<csr-id-8fc582d54ebc7dc974d2fd849b9de59134c1428d/>
<csr-id-ec3795a678d261a38085405b6e9bfea943dafefb/>

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

### Test

 - <csr-id-8fc582d54ebc7dc974d2fd849b9de59134c1428d/> add globbing utility for testing Markdown as doctests

### Chore

 - <csr-id-ec3795a678d261a38085405b6e9bfea943dafefb/> upgrade to Rust 2024 edition
   - Updates `Cargo.toml` to use new shared workspace keys
   - Updates lint settings (in workspace `Cargo.toml`)
   - `rustfmt` has changed slightly, resulting in a big diff - there are no
   actual code changes
   - Adds a script to `rustfmt` the template src files

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

### Test

 - <csr-id-3343fe2d58d8b7a7aa2766bfba9fbb4955114706/> add globbing utility for testing Markdown as doctests

### Chore (BREAKING)

 - <csr-id-44fb2806cf2d165d86695910f4755e0944c11832/> use DFIR name instead of Hydroflow in some places, fix #1644
   Fix partially #1712
   
   * Renames `WriteContextArgs.hydroflow` to `WriteContextArgs.df_ident`
   for DFIR operator codegen
   * Removes some dead code/files

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 7 commits contributed to the release.
 - 74 days passed between releases.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 5 unique issues were worked on: [#1649](https://github.com/hydro-project/hydro/issues/1649), [#1686](https://github.com/hydro-project/hydro/issues/1686), [#1690](https://github.com/hydro-project/hydro/issues/1690), [#1713](https://github.com/hydro-project/hydro/issues/1713), [#1747](https://github.com/hydro-project/hydro/issues/1747)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1649](https://github.com/hydro-project/hydro/issues/1649)**
    - Fixup! test: add globbing utility for testing Markdown as doctests ([`9b2e9f6`](https://github.com/hydro-project/hydro/commit/9b2e9f65afc25742d5ec67086ac826b0578d3a41))
    - Add globbing utility for testing Markdown as doctests ([`8fc582d`](https://github.com/hydro-project/hydro/commit/8fc582d54ebc7dc974d2fd849b9de59134c1428d))
 * **[#1686](https://github.com/hydro-project/hydro/issues/1686)**
    - Fix broken links, fix #1613 ([`d452739`](https://github.com/hydro-project/hydro/commit/d45273943b0ca087b05f0fe4331b12cbe2ff4e90))
 * **[#1690](https://github.com/hydro-project/hydro/issues/1690)**
    - Fix extraneous `\<` escaping introduced in #1558, fix #1614 ([`8e61291`](https://github.com/hydro-project/hydro/commit/8e612917f97edbb3739381ceb7f20daa1e4403b1))
 * **[#1713](https://github.com/hydro-project/hydro/issues/1713)**
    - Use DFIR name instead of Hydroflow in some places, fix #1644 ([`3966d90`](https://github.com/hydro-project/hydro/commit/3966d9063dae52e65b077321e0bd1150f2b0c3f1))
 * **[#1747](https://github.com/hydro-project/hydro/issues/1747)**
    - Upgrade to Rust 2024 edition ([`ec3795a`](https://github.com/hydro-project/hydro/commit/ec3795a678d261a38085405b6e9bfea943dafefb))
 * **Uncategorized**
    - Release dfir_lang v0.12.0, dfir_datalog_core v0.12.0, dfir_datalog v0.12.0, dfir_macro v0.12.0, hydroflow_deploy_integration v0.12.0, lattices_macro v0.5.9, variadics v0.0.9, variadics_macro v0.6.0, lattices v0.6.0, multiplatform_test v0.5.0, pusherator v0.0.11, dfir_rs v0.12.0, hydro_deploy v0.12.0, stageleft_macro v0.6.0, stageleft v0.7.0, stageleft_tool v0.6.0, hydro_lang v0.12.0, hydro_std v0.12.0, hydro_cli v0.12.0, safety bump 10 crates ([`973c925`](https://github.com/hydro-project/hydro/commit/973c925e87ed78344494581bd7ce1bbb4186a2f3))
</details>

## 0.11.0 (2024-12-23)

<csr-id-c58f13cff39b838fa283fae2711501c8b7894ff4/>
<csr-id-251b1039c71d45d3f86123dba1926026ded80824/>
<csr-id-5196f247e0124a31567af940541044ce1906cdc1/>
<csr-id-03b3a349013a71b324276bca5329c33d400a73ff/>
<csr-id-3291c07b37c9f9031837a2a32953e8f8854ec298/>
<csr-id-84ee06755a0ed7cabf32b334f1696bb600797c92/>
<csr-id-a6f60c92ae7168eb86eb311ca7b7afb10025c7de/>
<csr-id-5e58e346612a094c7e637919c84ab1e78b59be27/>

### Chore

 - <csr-id-c58f13cff39b838fa283fae2711501c8b7894ff4/> upgrade to Docusaurus v3
   Main breaking change is MDX parsing, which trips up on unescaped `<` in
   the generated docs, so we have to adjust the generator logic.

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

### New Features (BREAKING)

 - <csr-id-f2a4bee8cd6945937bed5bc22fd85efd8d0aef0a/> remove `import!`, fix #1110
   in prep for rust stable #1587
   
   No good way to resolve the source file paths on stable
   
   No way to get good diagnostics on external files in general, at all

### Refactor (BREAKING)

 - <csr-id-251b1039c71d45d3f86123dba1926026ded80824/> use `cfg(nightly)` instead of feature, remove `-Z` flag, use `Diagnostic::try_emit`
   Previous PR (#1587) website build did not work because `panic = "abort"`
   is set on wasm, leading to aborts for `proc_macro2::Span::unwrap()`
   calls.
   
   All tests except trybuild seem to pass on stable, WIP #1587 next

### `hydroflow_macro` Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 38 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 3 unique issues were worked on: [#1558](https://github.com/hydro-project/hydroflow/issues/1558), [#1600](https://github.com/hydro-project/hydroflow/issues/1600), [#1606](https://github.com/hydro-project/hydroflow/issues/1606)

### `hydroflow_macro` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1558](https://github.com/hydro-project/hydroflow/issues/1558)**
    - Upgrade to Docusaurus v3 ([`c58f13c`](https://github.com/hydro-project/hydroflow/commit/c58f13cff39b838fa283fae2711501c8b7894ff4))
 * **[#1600](https://github.com/hydro-project/hydroflow/issues/1600)**
    - Remove `import!`, fix #1110 ([`f2a4bee`](https://github.com/hydro-project/hydroflow/commit/f2a4bee8cd6945937bed5bc22fd85efd8d0aef0a))
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

### `hydroflow_macro` Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 69 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1444](https://github.com/hydro-project/hydroflow/issues/1444)

### `hydroflow_macro` Commit Details

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

### New Features

 - <csr-id-9e5f58ef773f0aee39a9705d9845361a2488649b/> allow `demux_enum` to have any number of outputs, fix #1329

### `hydroflow_macro` Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 38 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#1409](https://github.com/hydro-project/hydroflow/issues/1409), [#1423](https://github.com/hydro-project/hydroflow/issues/1423)

### `hydroflow_macro` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1409](https://github.com/hydro-project/hydroflow/issues/1409)**
    - Allow `demux_enum` to have any number of outputs, fix #1329 ([`9e5f58e`](https://github.com/hydro-project/hydroflow/commit/9e5f58ef773f0aee39a9705d9845361a2488649b))
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

### `hydroflow_macro` Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 59 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### `hydroflow_macro` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release hydroflow_lang v0.8.0, hydroflow_datalog_core v0.8.0, hydroflow_datalog v0.8.0, hydroflow_macro v0.8.0, lattices_macro v0.5.5, lattices v0.5.6, variadics v0.0.5, pusherator v0.0.7, hydroflow v0.8.0, hydroflow_plus v0.8.0, hydro_deploy v0.8.0, hydro_cli v0.8.0, hydroflow_plus_cli_integration v0.8.0, safety bump 7 crates ([`ca6c16b`](https://github.com/hydro-project/hydroflow/commit/ca6c16b4a7ce35e155fe7fc6c7d1676c37c9e4de))
    - Mark `hydroflow_datalog` and `hydroflow_macro` as unchanged for 0.8.0 release ([`186310f`](https://github.com/hydro-project/hydroflow/commit/186310f453f6a935ac5f53fdcaf07fe1337833bf))
</details>

## 0.7.0 (2024-05-24)

<csr-id-826dbd9a709de2f883992bdcefa8f2d566d74ecb/>

### Refactor

 - <csr-id-826dbd9a709de2f883992bdcefa8f2d566d74ecb/> simplify `demux_enum()`, somewhat improves error messages #1201

### `hydroflow_macro` Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 83 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1204](https://github.com/hydro-project/hydroflow/issues/1204)

### `hydroflow_macro` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1204](https://github.com/hydro-project/hydroflow/issues/1204)**
    - Simplify `demux_enum()`, somewhat improves error messages #1201 ([`826dbd9`](https://github.com/hydro-project/hydroflow/commit/826dbd9a709de2f883992bdcefa8f2d566d74ecb))
 * **Uncategorized**
    - Release hydroflow_lang v0.7.0, hydroflow_datalog_core v0.7.0, hydroflow_datalog v0.7.0, hydroflow_macro v0.7.0, lattices v0.5.5, multiplatform_test v0.1.0, pusherator v0.0.6, hydroflow v0.7.0, stageleft_macro v0.2.0, stageleft v0.3.0, stageleft_tool v0.2.0, hydroflow_plus v0.7.0, hydro_deploy v0.7.0, hydro_cli v0.7.0, hydroflow_plus_cli_integration v0.7.0, safety bump 8 crates ([`2852147`](https://github.com/hydro-project/hydroflow/commit/285214740627685e911781793e05d234ab2ad2bd))
</details>

## 0.6.0 (2024-03-02)

<csr-id-83cac6bb7fccd7589a5b3fcc36c465496b33bf2b/>

Unchanged from previous release.

### Chore

 - <csr-id-83cac6bb7fccd7589a5b3fcc36c465496b33bf2b/> mark hydroflow_datalog, hydroflow_macro as unchanged for release

### `hydroflow_macro` Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 28 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### `hydroflow_macro` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release hydroflow_lang v0.6.0, hydroflow_datalog_core v0.6.0, hydroflow_datalog v0.6.0, hydroflow_macro v0.6.0, lattices v0.5.3, variadics v0.0.4, pusherator v0.0.5, hydroflow v0.6.0, stageleft v0.2.0, hydroflow_plus v0.6.0, hydro_deploy v0.6.0, hydro_cli v0.6.0, hydroflow_plus_cli_integration v0.6.0, safety bump 7 crates ([`09ea65f`](https://github.com/hydro-project/hydroflow/commit/09ea65fe9cd45c357c43bffca30e60243fa45cc8))
    - Mark hydroflow_datalog, hydroflow_macro as unchanged for release ([`83cac6b`](https://github.com/hydro-project/hydroflow/commit/83cac6bb7fccd7589a5b3fcc36c465496b33bf2b))
</details>

## 0.5.2 (2024-02-02)

### New Features

 - <csr-id-7a791b8ccc489050ef10ddb186409cc046bd30f0/> implement state operator
   (#929)

### `hydroflow_macro` Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 4 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1041](https://github.com/hydro-project/hydroflow/issues/1041)

### `hydroflow_macro` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1041](https://github.com/hydro-project/hydroflow/issues/1041)**
    - Implement state operator ([`7a791b8`](https://github.com/hydro-project/hydroflow/commit/7a791b8ccc489050ef10ddb186409cc046bd30f0))
 * **Uncategorized**
    - Release hydroflow_lang v0.5.2, hydroflow_datalog_core v0.5.2, hydroflow_macro v0.5.2, lattices v0.5.2, hydroflow v0.5.2, hydro_cli v0.5.1, hydroflow_plus_cli_integration v0.5.1 ([`6ac8720`](https://github.com/hydro-project/hydroflow/commit/6ac872081753548ebb8ec95549b4d820dc050d3e))
</details>

## 0.5.1 (2024-01-29)

<csr-id-1b555e57c8c812bed4d6495d2960cbf77fb0b3ef/>

### Chore

 - <csr-id-1b555e57c8c812bed4d6495d2960cbf77fb0b3ef/> manually set lockstep-versioned crates (and `lattices`) to version `0.5.1`
   Setting manually since
   https://github.com/frewsxcv/rust-crates-index/issues/159 is messing with
   smart-release

### New Features

 - <csr-id-6158a7aae2ef9b58245c23fc668715a3fb2ff7dc/> new implementation and Hydro Deploy setup
   --
 - <csr-id-6158a7aae2ef9b58245c23fc668715a3fb2ff7dc/> new implementation and Hydro Deploy setup
   --
 - <csr-id-6158a7aae2ef9b58245c23fc668715a3fb2ff7dc/> new implementation and Hydro Deploy setup
   --
 - <csr-id-6158a7aae2ef9b58245c23fc668715a3fb2ff7dc/> new implementation and Hydro Deploy setup
   --
 - <csr-id-6158a7aae2ef9b58245c23fc668715a3fb2ff7dc/> new implementation and Hydro Deploy setup
   --
 - <csr-id-6158a7aae2ef9b58245c23fc668715a3fb2ff7dc/> new implementation and Hydro Deploy setup
   --
 - <csr-id-6158a7aae2ef9b58245c23fc668715a3fb2ff7dc/> new implementation and Hydro Deploy setup
   --
 - <csr-id-6158a7aae2ef9b58245c23fc668715a3fb2ff7dc/> new implementation and Hydro Deploy setup
   --

### `hydroflow_macro` Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 110 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#909](https://github.com/hydro-project/hydroflow/issues/909)

### `hydroflow_macro` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#909](https://github.com/hydro-project/hydroflow/issues/909)**
    - New implementation and Hydro Deploy setup ([`6158a7a`](https://github.com/hydro-project/hydroflow/commit/6158a7aae2ef9b58245c23fc668715a3fb2ff7dc))
 * **Uncategorized**
    - Release hydroflow_cli_integration v0.5.1, hydroflow_lang v0.5.1, hydroflow_datalog_core v0.5.1, hydroflow_datalog v0.5.1, hydroflow_macro v0.5.1, lattices v0.5.1, variadics v0.0.3, pusherator v0.0.4, hydroflow v0.5.1, stageleft_macro v0.1.0, stageleft v0.1.0, hydroflow_plus v0.5.1, hydro_deploy v0.5.1, hydro_cli v0.5.1 ([`478aebc`](https://github.com/hydro-project/hydroflow/commit/478aebc8fee2aa78eab86bd386322db1c70bde6a))
    - Manually set lockstep-versioned crates (and `lattices`) to version `0.5.1` ([`1b555e5`](https://github.com/hydro-project/hydroflow/commit/1b555e57c8c812bed4d6495d2960cbf77fb0b3ef))
</details>

## 0.5.0 (2023-10-11)

<csr-id-f19eccc79d6d7c88de7ba1ef6a0abf1caaef377f/>
<csr-id-2eac8ce42545cb543892b28267fb2f7089a92cdb/>
<csr-id-1126266e69c2c4364bc8de558f11859e5bad1c69/>

### Chore

 - <csr-id-f19eccc79d6d7c88de7ba1ef6a0abf1caaef377f/> bump proc-macro2 min version to 1.0.63
 - <csr-id-2eac8ce42545cb543892b28267fb2f7089a92cdb/> refactor collect-String into fold for `clippy::clippy::format_collect` lint
   using `.map(|| format!()).collect()` results in a heap `String` allocation for each item in the iterator, the fold only does one. Doesn't really matter for this case, just appeasing the lint

### New Features

 - <csr-id-b3d114827256f2b82a3c357f3419c6853a97f5c0/> initial technically working version of `demux_enum` with very bad error messages
   Technically does not check port names at all, just depends on their order.
 - <csr-id-f013c3ca15f2cc9413fcfb92898f71d5fc00073a/> add import!() expression
 - <csr-id-7714403e130969b96c8f405444d4daf451450fdf/> Add `monotonic_fn` and `morphism` macros, update snapshots for flow props.

### Bug Fixes

 - <csr-id-5ac9ddebedf615f87684d1092382ba64826c1c1c/> separate internal compiler operators in docs name/category/sort order
 - <csr-id-80d985f2870b80771c23eed9e2d9d2589d17088e/> properly feature gate `macro_invocation_path` `source_file()` #937

### Refactor

 - <csr-id-1126266e69c2c4364bc8de558f11859e5bad1c69/> `demux_enum` requires enum type name, add better error handling

### New Features (BREAKING)

 - <csr-id-9ed0ce02128a0eeaf0b603efcbe896427e47ef62/> Simplify graph printing code, add delta/cumul green edges, allow hiding of vars/subgraphs

### `hydroflow_macro` Commit Statistics

<csr-read-only-do-not-edit/>

 - 11 commits contributed to the release.
 - 56 days passed between releases.
 - 9 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 6 unique issues were worked on: [#881](https://github.com/hydro-project/hydroflow/issues/881), [#882](https://github.com/hydro-project/hydroflow/issues/882), [#884](https://github.com/hydro-project/hydroflow/issues/884), [#898](https://github.com/hydro-project/hydroflow/issues/898), [#932](https://github.com/hydro-project/hydroflow/issues/932), [#938](https://github.com/hydro-project/hydroflow/issues/938)

### `hydroflow_macro` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#881](https://github.com/hydro-project/hydroflow/issues/881)**
    - Refactor collect-String into fold for `clippy::clippy::format_collect` lint ([`2eac8ce`](https://github.com/hydro-project/hydroflow/commit/2eac8ce42545cb543892b28267fb2f7089a92cdb))
 * **[#882](https://github.com/hydro-project/hydroflow/issues/882)**
    - Add `monotonic_fn` and `morphism` macros, update snapshots for flow props. ([`7714403`](https://github.com/hydro-project/hydroflow/commit/7714403e130969b96c8f405444d4daf451450fdf))
 * **[#884](https://github.com/hydro-project/hydroflow/issues/884)**
    - Separate internal compiler operators in docs name/category/sort order ([`5ac9dde`](https://github.com/hydro-project/hydroflow/commit/5ac9ddebedf615f87684d1092382ba64826c1c1c))
 * **[#898](https://github.com/hydro-project/hydroflow/issues/898)**
    - Add import!() expression ([`f013c3c`](https://github.com/hydro-project/hydroflow/commit/f013c3ca15f2cc9413fcfb92898f71d5fc00073a))
 * **[#932](https://github.com/hydro-project/hydroflow/issues/932)**
    - Simplify graph printing code, add delta/cumul green edges, allow hiding of vars/subgraphs ([`9ed0ce0`](https://github.com/hydro-project/hydroflow/commit/9ed0ce02128a0eeaf0b603efcbe896427e47ef62))
 * **[#938](https://github.com/hydro-project/hydroflow/issues/938)**
    - Properly feature gate `macro_invocation_path` `source_file()` #937 ([`80d985f`](https://github.com/hydro-project/hydroflow/commit/80d985f2870b80771c23eed9e2d9d2589d17088e))
 * **Uncategorized**
    - Release hydroflow_macro v0.5.0, lattices v0.5.0, hydroflow v0.5.0, hydro_cli v0.5.0 ([`12697c2`](https://github.com/hydro-project/hydroflow/commit/12697c2f19bd96802591fa63a5b6b12104ecfe0d))
    - Release hydroflow_lang v0.5.0, hydroflow_datalog_core v0.5.0, hydroflow_datalog v0.5.0, hydroflow_macro v0.5.0, lattices v0.5.0, hydroflow v0.5.0, hydro_cli v0.5.0, safety bump 4 crates ([`2e2d8b3`](https://github.com/hydro-project/hydroflow/commit/2e2d8b386fb086c8276a2853d2a1f96ad4d7c221))
    - Bump proc-macro2 min version to 1.0.63 ([`f19eccc`](https://github.com/hydro-project/hydroflow/commit/f19eccc79d6d7c88de7ba1ef6a0abf1caaef377f))
    - `demux_enum` requires enum type name, add better error handling ([`1126266`](https://github.com/hydro-project/hydroflow/commit/1126266e69c2c4364bc8de558f11859e5bad1c69))
    - Initial technically working version of `demux_enum` with very bad error messages ([`b3d1148`](https://github.com/hydro-project/hydroflow/commit/b3d114827256f2b82a3c357f3419c6853a97f5c0))
</details>

## 0.4.0 (2023-08-15)

### New Features

 - <csr-id-b4b9644a19e8e7e7725c9c5b88e3a6b8c2be7364/> Add `use` statements to hydroflow syntax
   And use in doc tests.

### `hydroflow_macro` Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 42 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#845](https://github.com/hydro-project/hydroflow/issues/845)

### `hydroflow_macro` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#845](https://github.com/hydro-project/hydroflow/issues/845)**
    - Add `use` statements to hydroflow syntax ([`b4b9644`](https://github.com/hydro-project/hydroflow/commit/b4b9644a19e8e7e7725c9c5b88e3a6b8c2be7364))
 * **Uncategorized**
    - Release hydroflow_lang v0.4.0, hydroflow_datalog_core v0.4.0, hydroflow_datalog v0.4.0, hydroflow_macro v0.4.0, lattices v0.4.0, pusherator v0.0.3, hydroflow v0.4.0, hydro_cli v0.4.0, safety bump 4 crates ([`cb313f0`](https://github.com/hydro-project/hydroflow/commit/cb313f0635214460a8308d05cbef4bf7f4bfaa15))
</details>

## 0.3.0 (2023-07-04)

### Documentation

 - <csr-id-c6b39e72590a01590690b0e42237c9853b105edc/> avoid double-extension for generated files which breaks navigation

### New Features

 - <csr-id-ea65349d241873f8460d7a8b024d64c63180246f/> emit `compile_error!` diagnostics for stable
 - <csr-id-22abcaff806c7de6e4a7725656bbcf201e7d9259/> allow stable build, refactors behind `nightly` feature flag

### Bug Fixes

 - <csr-id-8d3494b5afee858114a602a3e23077bb6d24dd77/> update proc-macro2, use new span location API where possible
   requires latest* rust nightly version
   
   *latest = 2023-06-28 or something

### `hydroflow_macro` Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release.
 - 33 days passed between releases.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 3 unique issues were worked on: [#758](https://github.com/hydro-project/hydroflow/issues/758), [#780](https://github.com/hydro-project/hydroflow/issues/780), [#801](https://github.com/hydro-project/hydroflow/issues/801)

### `hydroflow_macro` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#758](https://github.com/hydro-project/hydroflow/issues/758)**
    - Avoid double-extension for generated files which breaks navigation ([`c6b39e7`](https://github.com/hydro-project/hydroflow/commit/c6b39e72590a01590690b0e42237c9853b105edc))
 * **[#780](https://github.com/hydro-project/hydroflow/issues/780)**
    - Emit `compile_error!` diagnostics for stable ([`ea65349`](https://github.com/hydro-project/hydroflow/commit/ea65349d241873f8460d7a8b024d64c63180246f))
    - Allow stable build, refactors behind `nightly` feature flag ([`22abcaf`](https://github.com/hydro-project/hydroflow/commit/22abcaff806c7de6e4a7725656bbcf201e7d9259))
 * **[#801](https://github.com/hydro-project/hydroflow/issues/801)**
    - Update proc-macro2, use new span location API where possible ([`8d3494b`](https://github.com/hydro-project/hydroflow/commit/8d3494b5afee858114a602a3e23077bb6d24dd77))
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

### `hydroflow_macro` Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release.
 - 1 day passed between releases.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#728](https://github.com/hydro-project/hydroflow/issues/728), [#730](https://github.com/hydro-project/hydroflow/issues/730)

### `hydroflow_macro` Commit Details

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

### Documentation

 - <csr-id-28c90251dd877dd84f28886eecb7b366abf3d45b/> Add initial Hydro Deploy docs
   Renamed from Hydro CLI because the CLI isn't really the main thing. Also moves the Hydroflow docs to a subdirectory and sets up a dropdown for multiple docs.

### `hydroflow_macro` Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 6 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#686](https://github.com/hydro-project/hydroflow/issues/686)

### `hydroflow_macro` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#686](https://github.com/hydro-project/hydroflow/issues/686)**
    - Add initial Hydro Deploy docs ([`28c9025`](https://github.com/hydro-project/hydroflow/commit/28c90251dd877dd84f28886eecb7b366abf3d45b))
 * **Uncategorized**
    - Release hydroflow_cli_integration v0.1.1, hydroflow_lang v0.1.1, hydroflow_datalog_core v0.1.1, hydroflow_macro v0.1.1, lattices v0.1.2, hydroflow v0.1.1, hydro_cli v0.1.0 ([`d9fa8b3`](https://github.com/hydro-project/hydroflow/commit/d9fa8b387e303b33d9614dbde80abf1af08bd8eb))
</details>

## 0.1.0 (2023-05-23)

<csr-id-52ee8f8e443f0a8b5caf92d2c5f028c00302a79b/>

### Chore

 - <csr-id-52ee8f8e443f0a8b5caf92d2c5f028c00302a79b/> bump versions to 0.1.0 for release
   For release on crates.io for v0.1

### Documentation

 - <csr-id-a8957ec4457aae1cfd6fae031bede5e3f4fcc75d/> Add rustdocs to hydroflow's proc macros

### `hydroflow_macro` Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release.
 - 2 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 4 unique issues were worked on: [#661](https://github.com/hydro-project/hydroflow/issues/661), [#671](https://github.com/hydro-project/hydroflow/issues/671), [#677](https://github.com/hydro-project/hydroflow/issues/677), [#684](https://github.com/hydro-project/hydroflow/issues/684)

### `hydroflow_macro` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#661](https://github.com/hydro-project/hydroflow/issues/661)**
    - Add hydroflow_{test, main} so that hydroflow is actually singlethreaded ([`f61054e`](https://github.com/hydro-project/hydroflow/commit/f61054eaeca6fab1ab0cb588b7ed546b87772e91))
 * **[#671](https://github.com/hydro-project/hydroflow/issues/671)**
    - Migrate docs to a unified Docusuarus site ([`feed326`](https://github.com/hydro-project/hydroflow/commit/feed3268c0aabeb027b19abd9ed06c565a0462f4))
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

### `hydroflow_macro` Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 25 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#643](https://github.com/hydro-project/hydroflow/issues/643), [#660](https://github.com/hydro-project/hydroflow/issues/660)

### `hydroflow_macro` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#643](https://github.com/hydro-project/hydroflow/issues/643)**
    - Fix book operators build ([`30d68a6`](https://github.com/hydro-project/hydroflow/commit/30d68a6865ae9769b073d31c39ccf40f724355d9))
 * **[#660](https://github.com/hydro-project/hydroflow/issues/660)**
    - Rustfmt group imports ([`20a1b2c`](https://github.com/hydro-project/hydroflow/commit/20a1b2c0cd04a8b495a02ce345db3d48a99ea0e9))
 * **Uncategorized**
    - Release hydroflow_cli_integration v0.0.1, hydroflow_lang v0.0.1, hydroflow_datalog_core v0.0.1, hydroflow_datalog v0.0.1, hydroflow_macro v0.0.1, lattices v0.1.0, variadics v0.0.2, pusherator v0.0.1, hydroflow v0.0.2 ([`809395a`](https://github.com/hydro-project/hydroflow/commit/809395acddb78949d7a2bf036e1a94972f23b1ad))
</details>

## 0.0.0 (2023-04-25)

### `hydroflow_macro` Commit Statistics

<csr-read-only-do-not-edit/>

 - 42 commits contributed to the release.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 13 unique issues were worked on: [#162](https://github.com/hydro-project/hydroflow/issues/162), [#310](https://github.com/hydro-project/hydroflow/issues/310), [#311](https://github.com/hydro-project/hydroflow/issues/311), [#318](https://github.com/hydro-project/hydroflow/issues/318), [#329](https://github.com/hydro-project/hydroflow/issues/329), [#404](https://github.com/hydro-project/hydroflow/issues/404), [#419](https://github.com/hydro-project/hydroflow/issues/419), [#441 11/14](https://github.com/hydro-project/hydroflow/issues/441 11/14), [#441 14/14](https://github.com/hydro-project/hydroflow/issues/441 14/14), [#501](https://github.com/hydro-project/hydroflow/issues/501), [#603](https://github.com/hydro-project/hydroflow/issues/603), [#609](https://github.com/hydro-project/hydroflow/issues/609), [#617](https://github.com/hydro-project/hydroflow/issues/617)

### `hydroflow_macro` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#162](https://github.com/hydro-project/hydroflow/issues/162)**
    - SerdeGraph from parser to be callable at runtime ([`17dd150`](https://github.com/hydro-project/hydroflow/commit/17dd1500be1dab5f7abbd498d8f96b6ed00dba59))
 * **[#310](https://github.com/hydro-project/hydroflow/issues/310)**
    - Sort ops while generating the op doc in the book ([`8193409`](https://github.com/hydro-project/hydroflow/commit/8193409eff2e20c4c192b4435df609ea99ea5598))
 * **[#311](https://github.com/hydro-project/hydroflow/issues/311)**
    - Better autogen of input/output specs for ops docs ([`2cbd3e7`](https://github.com/hydro-project/hydroflow/commit/2cbd3e7757da427a47fdde74278de3ec8cbbf9fb))
 * **[#318](https://github.com/hydro-project/hydroflow/issues/318)**
    - Reintroduce code to generate streaming/blocking annotations in docs ([`9530fd7`](https://github.com/hydro-project/hydroflow/commit/9530fd7f501fd078972c6fefe3aa211f23bc1814))
 * **[#329](https://github.com/hydro-project/hydroflow/issues/329)**
    - Get hydroflow to compile to WASM ([`24354d2`](https://github.com/hydro-project/hydroflow/commit/24354d2e11c69e38e4e021aa4acf1525b376b2b1))
 * **[#404](https://github.com/hydro-project/hydroflow/issues/404)**
    - Fix op docs "blocking" to check elided port names, fix #400 ([`608e65b`](https://github.com/hydro-project/hydroflow/commit/608e65b61788376a06ab56b7f92dfd45820b4c0e))
 * **[#419](https://github.com/hydro-project/hydroflow/issues/419)**
    - Encapsulate `FlatGraph`, separate `FlatGraphBuilder` ([`fceaea5`](https://github.com/hydro-project/hydroflow/commit/fceaea5659ac76c2275c1487582a17b646858602))
 * **[#441 11/14](https://github.com/hydro-project/hydroflow/issues/441 11/14)**
    - Remove `FlatGraph`, unify under `PartitionedGraph` ([`b640b53`](https://github.com/hydro-project/hydroflow/commit/b640b532e34b29f44c768d523fbf780dba9785ff))
 * **[#441 14/14](https://github.com/hydro-project/hydroflow/issues/441 14/14)**
    - Cleanup graph docs, organize method names ([`09d3b57`](https://github.com/hydro-project/hydroflow/commit/09d3b57eb03f3920bd10f5c10277d3ef4f9cb0ec))
 * **[#501](https://github.com/hydro-project/hydroflow/issues/501)**
    - Preserve serialize diagnostics for hydroflow graph, stop emitting expected warnings in tests ([`0c810e5`](https://github.com/hydro-project/hydroflow/commit/0c810e5fdd3445923c0c7afbe651f2b4a72c115e))
 * **[#603](https://github.com/hydro-project/hydroflow/issues/603)**
    - Improve operator docs page ([`4241ca0`](https://github.com/hydro-project/hydroflow/commit/4241ca07bad6c8777b4e1a05c6c900cfa8276c81))
 * **[#609](https://github.com/hydro-project/hydroflow/issues/609)**
    - Update syn to 2.0 ([`2e7d802`](https://github.com/hydro-project/hydroflow/commit/2e7d8024f35893ef0abcb6851e370b00615f9562))
 * **[#617](https://github.com/hydro-project/hydroflow/issues/617)**
    - Update `Cargo.toml`s for publishing ([`a78ff9a`](https://github.com/hydro-project/hydroflow/commit/a78ff9aace6771787c2b72aad83be6ad8d49a828))
 * **Uncategorized**
    - Setup release workflow ([`108d0e9`](https://github.com/hydro-project/hydroflow/commit/108d0e933a08b183c4dadf8c3499e4946696e263))
    - Use `HydroflowGraph` for graph writing, delete `SerdeGraph` ([`d1ef14e`](https://github.com/hydro-project/hydroflow/commit/d1ef14ee459c51d5a2dd9e7ea03050772e14178c))
    - Refactor `FlatGraph` assembly into separate `FlatGraphBuilder` ([`9dd3bd9`](https://github.com/hydro-project/hydroflow/commit/9dd3bd91586966484abaf01c4330d831804b1983))
    - Emit type guards inline, configurable #263 ([`c6510da`](https://github.com/hydro-project/hydroflow/commit/c6510da4b4cb46ec026e3c1c69b5ce29b17c473c))
    - Separate surface doctests by operator ([`851d97d`](https://github.com/hydro-project/hydroflow/commit/851d97de7ba3435bac98264f4b8679973536486a))
    - Add `hydroflow_macr/build.rs` to autogen operator book docs ([`a5de404`](https://github.com/hydro-project/hydroflow/commit/a5de404cd06c10137f7584d152269327c698a65d))
    - Refactor out surface syntax diagnostics (error messages) ([`008425b`](https://github.com/hydro-project/hydroflow/commit/008425bb436042524f540fc05a855f5fa5535c76))
    - Enable partitioned output on hydroflow_parser! ([`4936198`](https://github.com/hydro-project/hydroflow/commit/4936198a2328057fa4115e38d49898bfd18fb3bb))
    - Add more tests, fix surface syntax bugs ([`eb62ef1`](https://github.com/hydro-project/hydroflow/commit/eb62ef1a47ec58abcf6a11745667e00d69df6d93))
    - Fix surface syntax macro import issues on internal doctests and examples ([`d0be1fa`](https://github.com/hydro-project/hydroflow/commit/d0be1fa76443a8adaa5a2d8ccc1f4e8a3db40280))
    - Cleanups, rename `hydroflow_core` to `hydroflow_lang` ([`c8f2b56`](https://github.com/hydro-project/hydroflow/commit/c8f2b56295555c04e8240432ff686d89fccef01c))
    - Make surface syntax macro fail early for better error messages ([`cbf8a62`](https://github.com/hydro-project/hydroflow/commit/cbf8a62ea24131b5092cb91aea5939764e681760))
    - Wip on codegen w/ some code cleanups ([`d29fb7f`](https://github.com/hydro-project/hydroflow/commit/d29fb7fc275c2774be3f5c08b75f12fdaf6970ff))
    - Add #![allow(clippy::explicit_auto_deref)] due to false positives ([`20382f1`](https://github.com/hydro-project/hydroflow/commit/20382f13d9baf49ee896a6c643bb25788aff2db0))
    - Cleanup and rearrange hydroflow_core graph code ([`49476d3`](https://github.com/hydro-project/hydroflow/commit/49476d397e14a8616e8a963451f19f9752befaa6))
    - Separate into FlatGraph and PartitionedGraph ([`13b7830`](https://github.com/hydro-project/hydroflow/commit/13b783098b5b87ff0e1819b5e5fbb16395c9e308))
    - Move hydroflow macro code into hydroflow_core ([`2623c70`](https://github.com/hydro-project/hydroflow/commit/2623c70b80431a0d6a5e3531f93e3248443af03a))
    - Fix unused method and complex type lints ([`70727c0`](https://github.com/hydro-project/hydroflow/commit/70727c04fd6062b9e6c01799dd87f94bada19cd3))
    - Display handoffs separately in mermaid ([`a913dc6`](https://github.com/hydro-project/hydroflow/commit/a913dc67655e5c934d05576e4bf8ecac9551afdf))
    - Add handling of multi-edges, insertion of handoffs ([`dc8f2db`](https://github.com/hydro-project/hydroflow/commit/dc8f2db9e02304964c2f36caa11891c971c385f7))
    - Subgraph partitioning algorithm working ([`cc8c29c`](https://github.com/hydro-project/hydroflow/commit/cc8c29ccb52e662b80989904b32bb7ef8b487c28))
    - Add checker for operator arity ([`8e7f85a`](https://github.com/hydro-project/hydroflow/commit/8e7f85a7681e62354d5640fd95a703247b984bfb))
    - Cleanup old code, add helpful comments ([`0fe0f40`](https://github.com/hydro-project/hydroflow/commit/0fe0f40dd49bcd1164032ea331f06c209de2ce16))
    - Add mermaid rendering of surface syntax ([`09c9647`](https://github.com/hydro-project/hydroflow/commit/09c964784006898825f1a91893dc20c30bc7853f))
    - New parsing with nice error messages ([`b896108`](https://github.com/hydro-project/hydroflow/commit/b896108792a809e4cbc5053d5214a891c37d330b))
    - WIP issues with keeping spans around, idx collision ([`c963290`](https://github.com/hydro-project/hydroflow/commit/c963290d0e838031d2e0b9a29ce89fb2af047629))
    - Parse updated arrow syntax ([`b7f131c`](https://github.com/hydro-project/hydroflow/commit/b7f131ce38cffc6c8491c778500ceb32d44221d8))
    - Implement basic arrow syntax ([`de8ed49`](https://github.com/hydro-project/hydroflow/commit/de8ed492c1220a131052544079085f44266fe87f))
    - Hydroflow_macro boilerplate ([`b2a8b85`](https://github.com/hydro-project/hydroflow/commit/b2a8b853907ee93ad02ceeb39b95da08a0970330))
</details>

