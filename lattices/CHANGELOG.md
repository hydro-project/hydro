# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.8.0-alpha.1 (2026-06-11)

### Chore

 - <csr-id-e70eab6a0c793ef095e2cd747220d5419f7bf1a4/> revert accidental `v1.0.0-alpha.0` releases of `dfir_lang` & `variadics`, update `cargo-smart-release` fork version

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 1 commit contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Revert accidental `v1.0.0-alpha.0` releases of `dfir_lang` & `variadics`, update `cargo-smart-release` fork version ([`e70eab6`](https://github.com/hydro-project/hydro/commit/e70eab6a0c793ef095e2cd747220d5419f7bf1a4))
</details>

## 0.8.0-alpha.0 (2026-06-10)

### Chore

 - <csr-id-33a606afb3f123f01e4b9ce916c75cf9aa03e5fd/> update to rust 1.95.0
   - Bump `rust-toolchain.toml` from Rust 1.93.1 to 1.95.0.
   - Apply minor iterator/sort refactors to satisfy newer clippy lints and
   keep deterministic ordering where relevant.
   - Refresh trybuild compile-fail `.stderr` snapshots to match Rust 1.95
   diagnostic formatting.

### New Features

 - <csr-id-abf5d747bcea29588edc8ab5c3accee0225fba58/> support `#[no_std]` via `std` and `alloc` default features
   Also removes unused `AsBytes` trait
   
   ---------

### Bug Fixes

 - <csr-id-938cf91f4adff7b96f3f3536f4427f38f07f4ea6/> gate proc_macro_diagnostics behind codegen feature, remove several stabilized features. [ci-full]

### Test

 - <csr-id-7161b32a4dca04ca7c163a974d792552308b3a13/> cleanup trybuild stable vs nightly handling
   Previous setup meant you had to wrangle symlinks every time you added
   more tests, and they were tracked in git. This has the test helper set
   up the symlinks, and they are no longer tracked in git.
   
   **Structure per crate:**
   ```
   tests/compile-fail/
   ├─ .gitignore    # ignores stable/*.rs and nightly/*.rs
   ├─ *.rs          # source test files (single canonical copy)
   ├─ stable/
   │  └─ *.stderr   # expected errors for stable rustc
   └─ nightly/
      └─ *.stderr   # expected errors for nightly rustc
   ```
   
   **Usage in each test file:**
   ```rust
   #[test]
   fn test_all() {
       hydro_build_utils::trybuild_compile_fail!("surface_*.rs"); // or "*.rs"
   }
   ```
   
   **How it works:**
   - The macro symlinks `.rs` files into the active channel's subdir at
   test time
   - Trybuild runs against the channel subdir, making `.rs` symlinks next
   to the `.stderr` files
   - `TRYBUILD=overwrite` writes updated `.stderr` directly into the
   correct committed directory

### New Features (BREAKING)

 - <csr-id-dcef67d9572055f7eeb64f92fa95fa4babebd6c8/> `#[no_std]` support
   Adds default `std` and `alloc` features

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 7 commits contributed to the release.
 - 40 days passed between releases.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 5 unique issues were worked on: [#2869](https://github.com/hydro-project/hydro/issues/2869), [#2870](https://github.com/hydro-project/hydro/issues/2870), [#2878](https://github.com/hydro-project/hydro/issues/2878), [#2893](https://github.com/hydro-project/hydro/issues/2893), [#2919](https://github.com/hydro-project/hydro/issues/2919)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#2869](https://github.com/hydro-project/hydro/issues/2869)**
    - Cleanup trybuild stable vs nightly handling ([`7161b32`](https://github.com/hydro-project/hydro/commit/7161b32a4dca04ca7c163a974d792552308b3a13))
 * **[#2870](https://github.com/hydro-project/hydro/issues/2870)**
    - Update to rust 1.95.0 ([`33a606a`](https://github.com/hydro-project/hydro/commit/33a606afb3f123f01e4b9ce916c75cf9aa03e5fd))
 * **[#2878](https://github.com/hydro-project/hydro/issues/2878)**
    - Gate proc_macro_diagnostics behind codegen feature, remove several stabilized features. [ci-full] ([`938cf91`](https://github.com/hydro-project/hydro/commit/938cf91f4adff7b96f3f3536f4427f38f07f4ea6))
 * **[#2893](https://github.com/hydro-project/hydro/issues/2893)**
    - `#[no_std]` support ([`dcef67d`](https://github.com/hydro-project/hydro/commit/dcef67d9572055f7eeb64f92fa95fa4babebd6c8))
 * **[#2919](https://github.com/hydro-project/hydro/issues/2919)**
    - Support `#[no_std]` via `std` and `alloc` default features ([`abf5d74`](https://github.com/hydro-project/hydro/commit/abf5d747bcea29588edc8ab5c3accee0225fba58))
 * **Uncategorized**
    - Release lattices_macro v0.6.1-alpha.0, lattices v0.8.0-alpha.0, dfir_pipes v0.1.0-alpha.0, sinktools v0.2.0-alpha.0, hydro_deploy_integration v0.17.0-alpha.0, dfir_rs v0.17.0-alpha.0, hydro_deploy v0.17.0-alpha.0, hydro_lang v0.17.0-alpha.0, hydro_std v0.17.0-alpha.0 ([`2fabf68`](https://github.com/hydro-project/hydro/commit/2fabf6839e34f9275b5d698f396c1864e1539082))
    - Release hydro_build_utils v0.1.1-alpha.0, dfir_lang v1.0.0-alpha.0, dfir_macro v0.17.0-alpha.0, variadics v1.0.0-alpha.0, variadics_macro v0.8.0-alpha.0, lattices v0.8.0-alpha.0, dfir_pipes v0.1.0-alpha.0, sinktools v0.2.0-alpha.0, hydro_deploy_integration v0.17.0-alpha.0, dfir_rs v0.17.0-alpha.0, hydro_deploy v0.17.0-alpha.0, hydro_lang v0.17.0-alpha.0, hydro_std v0.17.0-alpha.0, safety bump 10 crates ([`12e7666`](https://github.com/hydro-project/hydro/commit/12e76666f7104f81b48de5ddf397b8e72c8a6711))
</details>

## 0.7.0 (2026-05-01)

<csr-id-e26d40c96f0aa022ee90388aad0a51ecd23ae345/>
<csr-id-1c8f85366c592b2768df65ba1ee2e98d2c06d496/>
<csr-id-efaa8f61c124c4b3c691b92a58df1686751cf45c/>

### Bug Fixes

 - <csr-id-c16e13a8bdae3b099d498f9b7f1f43872cfdc939/> flag non-determinstic hashmap iterators, fix hydro_lang codegen nondeterminism fix #2464
   Out of an abundance of caution, the `hydro_lang` IR `Demux` variants
   containing `HashMap<u32 ...>` have been replaced with `BTreeMap`

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release over the course of 105 calendar days.
 - 156 days passed between releases.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 4 unique issues were worked on: [#2448](https://github.com/hydro-project/hydro/issues/2448), [#2511](https://github.com/hydro-project/hydro/issues/2511), [#2525](https://github.com/hydro-project/hydro/issues/2525), [#2623](https://github.com/hydro-project/hydro/issues/2623)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#2448](https://github.com/hydro-project/hydro/issues/2448)**
    - Remove unused deps ([`e26d40c`](https://github.com/hydro-project/hydro/commit/e26d40c96f0aa022ee90388aad0a51ecd23ae345))
 * **[#2511](https://github.com/hydro-project/hydro/issues/2511)**
    - Flag non-determinstic hashmap iterators, fix hydro_lang codegen nondeterminism fix #2464 ([`c16e13a`](https://github.com/hydro-project/hydro/commit/c16e13a8bdae3b099d498f9b7f1f43872cfdc939))
 * **[#2525](https://github.com/hydro-project/hydro/issues/2525)**
    - Update pinned rust to 1.92, add lints/fixes for redundant cloning, string handling ([`efaa8f6`](https://github.com/hydro-project/hydro/commit/efaa8f61c124c4b3c691b92a58df1686751cf45c))
 * **[#2623](https://github.com/hydro-project/hydro/issues/2623)**
    - Leading colons to workaround rustfmt change ([`1c8f853`](https://github.com/hydro-project/hydro/commit/1c8f85366c592b2768df65ba1ee2e98d2c06d496))
 * **Uncategorized**
    - Release dfir_pipes v0.0.1, example_test v0.0.1, sinktools v0.1.0, hydro_deploy_integration v0.16.0, lattices_macro v0.6.0, variadics_macro v0.7.0, lattices v0.7.0, multiplatform_test v0.7.0, dfir_rs v0.16.0, copy_span v0.1.1, hydro_deploy v0.16.0, hydro_lang v0.16.0, hydro_std v0.16.0 ([`118b356`](https://github.com/hydro-project/hydro/commit/118b356447d92e778313d72a351e5a8d2814aa1a))
    - Release hydro_build_utils v0.1.0, dfir_lang v0.16.0, dfir_macro v0.16.0, variadics v0.1.0, dfir_pipes v0.0.1, example_test v0.0.1, sinktools v0.1.0, hydro_deploy_integration v0.16.0, lattices_macro v0.6.0, variadics_macro v0.7.0, lattices v0.7.0, multiplatform_test v0.7.0, dfir_rs v0.16.0, copy_span v0.1.1, hydro_deploy v0.16.0, hydro_lang v0.16.0, hydro_std v0.16.0, safety bump 13 crates ([`c20757a`](https://github.com/hydro-project/hydro/commit/c20757ae0e9e10463b2a499de4b7d37ab02269d0))
</details>

## 0.6.2 (2025-11-25)

<csr-id-97426b8a7e3b3af8a58b4c44c768c3f48cd0ed71/>
<csr-id-806a6239a649e24fe10c3c90dd30bd18debd41d2/>

### New Features

 - <csr-id-1f05026c2d695522713b152e3b872455e1c1b439/> Add efficient tombstone storage for lattices (RoaringBitmap + FST)
   This PR implements space-efficient tombstone storage for
   `SetUnionWithTombstones` and `MapUnionWithTombstones` using
   RoaringBitmap (for integers) and FST (for strings).
 - <csr-id-7efe1dc660ab6c0c68762f71d4359f347cfe73b6/> `sinktools` crate
   this also converts `variadic` to use `#[no_std]`, and adds
   `feature="std"`, and fixes an issue causing trybuild tests to not run
   
   will replace `pusherator` in upcoming PR

### Bug Fixes

 - <csr-id-c40876ec4bd3b31254d683e479b9a235f3d11f67/> refactor github actions workflows, make stable the default toolchain

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 7 commits contributed to the release.
 - 255 days passed between releases.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 4 unique issues were worked on: [#2024](https://github.com/hydro-project/hydro/issues/2024), [#2028](https://github.com/hydro-project/hydro/issues/2028), [#2157](https://github.com/hydro-project/hydro/issues/2157), [#2271](https://github.com/hydro-project/hydro/issues/2271)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#2024](https://github.com/hydro-project/hydro/issues/2024)**
    - Update pinned nightly to 2025-08-20, fix lints ([`97426b8`](https://github.com/hydro-project/hydro/commit/97426b8a7e3b3af8a58b4c44c768c3f48cd0ed71))
 * **[#2028](https://github.com/hydro-project/hydro/issues/2028)**
    - Refactor github actions workflows, make stable the default toolchain ([`c40876e`](https://github.com/hydro-project/hydro/commit/c40876ec4bd3b31254d683e479b9a235f3d11f67))
 * **[#2157](https://github.com/hydro-project/hydro/issues/2157)**
    - `sinktools` crate ([`7efe1dc`](https://github.com/hydro-project/hydro/commit/7efe1dc660ab6c0c68762f71d4359f347cfe73b6))
 * **[#2271](https://github.com/hydro-project/hydro/issues/2271)**
    - Add efficient tombstone storage for lattices (RoaringBitmap + FST) ([`1f05026`](https://github.com/hydro-project/hydro/commit/1f05026c2d695522713b152e3b872455e1c1b439))
 * **Uncategorized**
    - Release sinktools v0.0.1, hydro_deploy_integration v0.15.0, lattices_macro v0.5.11, variadics_macro v0.6.2, lattices v0.6.2, multiplatform_test v0.6.0, dfir_rs v0.15.0, copy_span v0.1.0, hydro_deploy v0.15.0, hydro_lang v0.15.0, hydro_std v0.15.0 ([`ac88df1`](https://github.com/hydro-project/hydro/commit/ac88df1e98af9fa2027488252f6014efa7bef229))
    - Release hydro_build_utils v0.0.1, dfir_lang v0.15.0, dfir_macro v0.15.0, variadics v0.0.10, sinktools v0.0.1, hydro_deploy_integration v0.15.0, lattices_macro v0.5.11, variadics_macro v0.6.2, lattices v0.6.2, multiplatform_test v0.6.0, dfir_rs v0.15.0, copy_span v0.1.0, hydro_deploy v0.15.0, hydro_lang v0.15.0, hydro_std v0.15.0, safety bump 5 crates ([`092de25`](https://github.com/hydro-project/hydro/commit/092de252238dfb9fa6b01e777c6dd8bf9db93398))
    - Ensure `hydro_build_utils` is published in the correct order ([`806a623`](https://github.com/hydro-project/hydro/commit/806a6239a649e24fe10c3c90dd30bd18debd41d2))
</details>

## 0.6.1 (2025-03-15)

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

 - 2 commits contributed to the release.
 - 6 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1787](https://github.com/hydro-project/hydro/issues/1787)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1787](https://github.com/hydro-project/hydro/issues/1787)**
    - Demote python deploy docs, fix docsrs configs, fix #1392, fix #1629 ([`b235a42`](https://github.com/hydro-project/hydro/commit/b235a42a3071e55da7b09bdc8bc710b18e0fe053))
 * **Uncategorized**
    - Release dfir_lang v0.12.1, dfir_datalog_core v0.12.1, dfir_datalog v0.12.1, dfir_macro v0.12.1, hydro_deploy_integration v0.12.1, lattices v0.6.1, pusherator v0.0.12, dfir_rs v0.12.1, hydro_deploy v0.12.1, hydro_lang v0.12.1, hydro_std v0.12.1, hydro_cli v0.12.1 ([`23221b5`](https://github.com/hydro-project/hydro/commit/23221b53b30918707ddaa85529d04cd7919166b4))
</details>

## 0.6.0 (2025-03-08)

<csr-id-49a387d4a21f0763df8ec94de73fb953c9cd333a/>
<csr-id-2fd6aa7417dfa29f389c04c5b9674b80bfed6cf2/>
<csr-id-39a2963518a9cc63c7e60a5c542cfa2509064a0c/>
<csr-id-c1983308743d912e5bf2583b7cccbb47d8a8b5d1/>
<csr-id-edffa95f5fe44f4e0cbb4b6c93754e9047f0fd3d/>
<csr-id-fd85262930c678601a80c080fb79778675124964/>
<csr-id-ec3795a678d261a38085405b6e9bfea943dafefb/>
<csr-id-8f4426089dcbbe5d1098f89e367c7be49a03e401/>

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release.
 - 74 days passed between releases.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 4 unique issues were worked on: [#1640](https://github.com/hydro-project/hydro/issues/1640), [#1704](https://github.com/hydro-project/hydro/issues/1704), [#1747](https://github.com/hydro-project/hydro/issues/1747), [#1749](https://github.com/hydro-project/hydro/issues/1749)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1640](https://github.com/hydro-project/hydro/issues/1640)**
    - Clippy cleanups for latest stable rust ([`fd85262`](https://github.com/hydro-project/hydro/commit/fd85262930c678601a80c080fb79778675124964))
 * **[#1704](https://github.com/hydro-project/hydro/issues/1704)**
    - Update pinned nightly to 2025-02-10, cleanups for clippy ([`8f44260`](https://github.com/hydro-project/hydro/commit/8f4426089dcbbe5d1098f89e367c7be49a03e401))
 * **[#1747](https://github.com/hydro-project/hydro/issues/1747)**
    - Upgrade to Rust 2024 edition ([`ec3795a`](https://github.com/hydro-project/hydro/commit/ec3795a678d261a38085405b6e9bfea943dafefb))
 * **[#1749](https://github.com/hydro-project/hydro/issues/1749)**
    - Fix small format issue ([`edffa95`](https://github.com/hydro-project/hydro/commit/edffa95f5fe44f4e0cbb4b6c93754e9047f0fd3d))
 * **Uncategorized**
    - Release dfir_lang v0.12.0, dfir_datalog_core v0.12.0, dfir_datalog v0.12.0, dfir_macro v0.12.0, hydroflow_deploy_integration v0.12.0, lattices_macro v0.5.9, variadics v0.0.9, variadics_macro v0.6.0, lattices v0.6.0, multiplatform_test v0.5.0, pusherator v0.0.11, dfir_rs v0.12.0, hydro_deploy v0.12.0, stageleft_macro v0.6.0, stageleft v0.7.0, stageleft_tool v0.6.0, hydro_lang v0.12.0, hydro_std v0.12.0, hydro_cli v0.12.0, safety bump 10 crates ([`973c925`](https://github.com/hydro-project/hydro/commit/973c925e87ed78344494581bd7ce1bbb4186a2f3))
</details>

## 0.5.9 (2024-12-23)

<csr-id-3291c07b37c9f9031837a2a32953e8f8854ec298/>
<csr-id-5e58e346612a094c7e637919c84ab1e78b59be27/>

### Documentation

 - <csr-id-204bd117ca3a8845b4986539efb91a0c612dfa05/> add `repository` field to `Cargo.toml`s, fix #1452
   #1452 
   
   Will trigger new releases of the following:
   `unchanged = 'hydroflow_deploy_integration', 'variadics',
   'variadics_macro', 'pusherator'`
   
   (All other crates already have changes, so would be released anyway)
 - <csr-id-987f7ad8668d9740ceea577a595035228898d530/> cleanups for the rename, fixing links

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
    - Add `repository` field to `Cargo.toml`s, fix #1452 ([`204bd11`](https://github.com/hydro-project/hydro/commit/204bd117ca3a8845b4986539efb91a0c612dfa05))
 * **[#1620](https://github.com/hydro-project/hydro/issues/1620)**
    - Rename Hydroflow -> DFIR ([`5e58e34`](https://github.com/hydro-project/hydro/commit/5e58e346612a094c7e637919c84ab1e78b59be27))
 * **[#1624](https://github.com/hydro-project/hydro/issues/1624)**
    - Cleanups for the rename, fixing links ([`987f7ad`](https://github.com/hydro-project/hydro/commit/987f7ad8668d9740ceea577a595035228898d530))
 * **Uncategorized**
    - Release dfir_lang v0.11.0, dfir_datalog_core v0.11.0, dfir_datalog v0.11.0, dfir_macro v0.11.0, hydroflow_deploy_integration v0.11.0, lattices_macro v0.5.8, variadics v0.0.8, variadics_macro v0.5.6, lattices v0.5.9, multiplatform_test v0.4.0, pusherator v0.0.10, dfir_rs v0.11.0, hydro_deploy v0.11.0, stageleft_macro v0.5.0, stageleft v0.6.0, stageleft_tool v0.5.0, hydro_lang v0.11.0, hydro_std v0.11.0, hydro_cli v0.11.0, safety bump 6 crates ([`361b443`](https://github.com/hydro-project/hydro/commit/361b4439ef9c781860f18d511668ab463a8c5203))
</details>

## 0.5.8 (2024-11-08)

<csr-id-d5677604e93c07a5392f4229af94a0b736eca382/>
<csr-id-47cb703e771f7d1c451ceb9d185ada96410949da/>
<csr-id-cebd1dc35282514f025e047a9b94800f546dd62f/>
<csr-id-014ebb2628b5b80ea1b6426b58c4d62706edb9ef/>

### New Features

 - <csr-id-48e4eb28a9ce652037ac81b580d30f93159dae9b/> generalized hash trie indexes for relational tuples
   Generalized Hash Tries are part of the SIGMOD '23 FreeJoin
   [paper](https://dl.acm.org/doi/abs/10.1145/3589295) by
   Wang/Willsey/Suciu. They provide a compressed ("factorized")
   representation of relations. By operating in the factorized domain, join
   algorithms can defer cross-products and achieve asymptotically optimal
   performance.
   
   ---------

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
    - Update pinned rust version, clippy lints, remove some dead code ([`014ebb2`](https://github.com/hydro-project/hydro/commit/014ebb2628b5b80ea1b6426b58c4d62706edb9ef))
 * **[#1503](https://github.com/hydro-project/hydro/issues/1503)**
    - Generalized hash trie indexes for relational tuples ([`48e4eb2`](https://github.com/hydro-project/hydro/commit/48e4eb28a9ce652037ac81b580d30f93159dae9b))
 * **[#1505](https://github.com/hydro-project/hydro/issues/1505)**
    - Fixes for nightly clippy ([`cebd1dc`](https://github.com/hydro-project/hydro/commit/cebd1dc35282514f025e047a9b94800f546dd62f))
 * **Uncategorized**
    - Release hydroflow_lang v0.10.0, hydroflow_datalog_core v0.10.0, hydroflow_datalog v0.10.0, hydroflow_deploy_integration v0.10.0, hydroflow_macro v0.10.0, lattices_macro v0.5.7, variadics v0.0.7, variadics_macro v0.5.5, lattices v0.5.8, multiplatform_test v0.3.0, pusherator v0.0.9, hydroflow v0.10.0, hydro_deploy v0.10.0, stageleft_macro v0.4.0, stageleft v0.5.0, stageleft_tool v0.4.0, hydroflow_plus v0.10.0, hydro_cli v0.10.0, safety bump 8 crates ([`258f480`](https://github.com/hydro-project/hydro/commit/258f4805dbcca36750cbfaaf36db00d3a007d817))
</details>

## 0.5.7 (2024-08-30)

<csr-id-11af32828bab6e4a4264d2635ff71a12bb0bb778/>
<csr-id-2c04f51f1ec44f7898307b6610371dcb490ea686/>

### Documentation

 - <csr-id-1766c8b0aa23df83ad242b581184b37e85afe27b/> cleanup doc comments for clippy latest

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
    - Lower min dependency versions where possible, update `Cargo.lock` ([`2c04f51`](https://github.com/hydro-project/hydro/commit/2c04f51f1ec44f7898307b6610371dcb490ea686))
 * **[#1428](https://github.com/hydro-project/hydro/issues/1428)**
    - Cleanup doc comments for clippy latest ([`1766c8b`](https://github.com/hydro-project/hydro/commit/1766c8b0aa23df83ad242b581184b37e85afe27b))
 * **Uncategorized**
    - Release hydroflow_lang v0.9.0, hydroflow_datalog_core v0.9.0, hydroflow_datalog v0.9.0, hydroflow_deploy_integration v0.9.0, hydroflow_macro v0.9.0, lattices_macro v0.5.6, lattices v0.5.7, multiplatform_test v0.2.0, variadics v0.0.6, pusherator v0.0.8, hydroflow v0.9.0, stageleft_macro v0.3.0, stageleft v0.4.0, stageleft_tool v0.3.0, hydroflow_plus v0.9.0, hydro_deploy v0.9.0, hydro_cli v0.9.0, hydroflow_plus_deploy v0.9.0, safety bump 8 crates ([`1d54331`](https://github.com/hydro-project/hydro/commit/1d54331976040c049e4c97a9fba0e66930efee52))
</details>

## 0.5.6 (2024-07-23)

<csr-id-3098f77fd99882aae23c4b31017aa4b761306197/>
<csr-id-45091d413f6da32927b640df781ce671a6e17c15/>

### New Features

 - <csr-id-33b9795f207804e9561f228fa0307c5973745241/> add `#[derive(Lattice)]` derive macros, fix #1247
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

 - <csr-id-7fd17b3f5504719467d119f64cd7bfe17c2660a7/> change fuzz test bounds to require `PartialEq` instead of `Eq`, fix #1302
 - <csr-id-c163909795d6be2e887daa57bb2057fc9ba74b7c/> Make inner for `WithTop` & `WithBot` private
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
    - Make inner for `WithTop` & `WithBot` private ([`c163909`](https://github.com/hydro-project/hydro/commit/c163909795d6be2e887daa57bb2057fc9ba74b7c))
 * **[#1250](https://github.com/hydro-project/hydro/issues/1250)**
    - Add `#[derive(Lattice)]` derive macros, fix #1247 ([`33b9795`](https://github.com/hydro-project/hydro/commit/33b9795f207804e9561f228fa0307c5973745241))
 * **[#1309](https://github.com/hydro-project/hydro/issues/1309)**
    - Update pinned rust version to 2024-06-17 ([`45091d4`](https://github.com/hydro-project/hydro/commit/45091d413f6da32927b640df781ce671a6e17c15))
 * **[#1326](https://github.com/hydro-project/hydro/issues/1326)**
    - Change fuzz test bounds to require `PartialEq` instead of `Eq`, fix #1302 ([`7fd17b3`](https://github.com/hydro-project/hydro/commit/7fd17b3f5504719467d119f64cd7bfe17c2660a7))
 * **Uncategorized**
    - Release hydroflow_lang v0.8.0, hydroflow_datalog_core v0.8.0, hydroflow_datalog v0.8.0, hydroflow_macro v0.8.0, lattices_macro v0.5.5, lattices v0.5.6, variadics v0.0.5, pusherator v0.0.7, hydroflow v0.8.0, hydroflow_plus v0.8.0, hydro_deploy v0.8.0, hydro_cli v0.8.0, hydroflow_plus_cli_integration v0.8.0, safety bump 7 crates ([`7b9c367`](https://github.com/hydro-project/hydro/commit/7b9c3678930af8010f8e2ffd4069583ece528119))
</details>

## 0.5.5 (2024-05-24)

### Documentation

 - <csr-id-147eea51dec2ff764351d5915fbe3e8b995c6db4/> Updating CONTRIBUTING.md with some info about feature branches
   Also updating GitHub workflows to run on feature branches as well.

### New Features

<csr-id-c2577bd0ad1969f4badf23874a9e7a6c1622c5c3/>
<csr-id-d8e4d9dc784ae28fcefe5f32a0561698c1196d31/>
<csr-id-c3f5a37ff746401a2383a900f9004e33072d5b1a/>
<csr-id-e0a09c8147fcc5c092b611e0f2779efa296c37fe/>
<csr-id-636b2cea52a45a7cd942e578d04083d08147cac1/>
<csr-id-41bf0a78b97c1373724af6063aff5c4133e8dbdd/>
<csr-id-e97e8c33a323db87959d86084cd679015d1cb5f2/>


### Bug Fixes

 - <csr-id-67ad8e269a2b7af5277775ac60edf414e53237a7/> typos in lattice docs

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
    - Add Integral domain to algebra library ([`a724998`](https://github.com/hydro-project/hydro/commit/a7249986497d602f9d6ea08242b0c09093dc0dc7))
 * **[#1156](https://github.com/hydro-project/hydro/issues/1156)**
    - Add prototype of tagging algebraic properties ([`e97e8c3`](https://github.com/hydro-project/hydro/commit/e97e8c33a323db87959d86084cd679015d1cb5f2))
 * **[#1174](https://github.com/hydro-project/hydro/issues/1174)**
    - Typos in lattice docs ([`67ad8e2`](https://github.com/hydro-project/hydro/commit/67ad8e269a2b7af5277775ac60edf414e53237a7))
 * **[#1181](https://github.com/hydro-project/hydro/issues/1181)**
    - Part 1 ([`f7f344e`](https://github.com/hydro-project/hydro/commit/f7f344ec64126f75fca2b948e65c8a0fb9ecb9b6))
 * **[#1230](https://github.com/hydro-project/hydro/issues/1230)**
    - Expose PairBimorphism public. ([`636b2ce`](https://github.com/hydro-project/hydro/commit/636b2cea52a45a7cd942e578d04083d08147cac1))
 * **[#1233](https://github.com/hydro-project/hydro/issues/1233)**
    - Make Pair<> members public ([`e0a09c8`](https://github.com/hydro-project/hydro/commit/e0a09c8147fcc5c092b611e0f2779efa296c37fe))
 * **[#1236](https://github.com/hydro-project/hydro/issues/1236)**
    - Updating CONTRIBUTING.md with some info about feature branches ([`147eea5`](https://github.com/hydro-project/hydro/commit/147eea51dec2ff764351d5915fbe3e8b995c6db4))
 * **Uncategorized**
    - Release hydroflow_lang v0.7.0, hydroflow_datalog_core v0.7.0, hydroflow_datalog v0.7.0, hydroflow_macro v0.7.0, lattices v0.5.5, multiplatform_test v0.1.0, pusherator v0.0.6, hydroflow v0.7.0, stageleft_macro v0.2.0, stageleft v0.3.0, stageleft_tool v0.2.0, hydroflow_plus v0.7.0, hydro_deploy v0.7.0, hydro_cli v0.7.0, hydroflow_plus_cli_integration v0.7.0, safety bump 8 crates ([`855fda6`](https://github.com/hydro-project/hydro/commit/855fda65442ad7a9074a099ecc29e74322332418))
    - Definitions of linearity and bilinearity in algebra lib ([`41bf0a7`](https://github.com/hydro-project/hydro/commit/41bf0a78b97c1373724af6063aff5c4133e8dbdd))
</details>

## 0.5.4 (2024-04-05)

<csr-id-2a10c4f395bbf3a320bdde6ec24c3c6abd5d6ed0/>
<csr-id-4e3c188dbe7cb83401fa3df537f7f8e83d1c9641/>

Unchanged from previous release.

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
    - Initial Algebra Library ([`39752fd`](https://github.com/hydro-project/hydro/commit/39752fd86f30be33424639c7817a75a118b72bea))
 * **Uncategorized**
    - Release hydroflow_cli_integration v0.5.2, hydroflow_lang v0.6.1, hydroflow_datalog_core v0.6.1, lattices v0.5.4, hydroflow v0.6.1, stageleft_macro v0.1.1, stageleft v0.2.1, hydroflow_plus v0.6.1, hydro_deploy v0.6.1, hydro_cli v0.6.1, hydroflow_plus_cli_integration v0.6.1, stageleft_tool v0.1.1 ([`fb82e52`](https://github.com/hydro-project/hydro/commit/fb82e523bb217658775989a276e18a1af68103c8))
    - Mark `lattices` as unchanged for `0.6.1` release ([`4e3c188`](https://github.com/hydro-project/hydro/commit/4e3c188dbe7cb83401fa3df537f7f8e83d1c9641))
</details>

## 0.5.3 (2024-03-02)

<csr-id-39ab8b0278e9e3fe96552ace0a4ae768a6bc10d8/>
<csr-id-71353f0d4dfd9766dfdc715c4a91a028081f910f/>
<csr-id-6b0a78ba0b4fd58302f7151254976c158a61b18c/>
<csr-id-65c7ebe3d64c478e7a4f0d8eb12e2bb3c1b267a3/>

### New Features

 - <csr-id-8d3286ac1d099e78fa1590b7749cc6316730164e/> impl missing `SimpleCollectionRef` for various collections types
 - <csr-id-17da2726ff302e3e9bd70824e4cdf4ba808df7ec/> add `Lattice[Bi]Morphism` traits, impls for cartesian product, pair, and keyed

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
    - Impl missing `SimpleCollectionRef` for various collections types ([`8d3286a`](https://github.com/hydro-project/hydro/commit/8d3286ac1d099e78fa1590b7749cc6316730164e))
 * **[#1062](https://github.com/hydro-project/hydro/issues/1062)**
    - Add `Lattice[Bi]Morphism` traits, impls for cartesian product, pair, and keyed ([`17da272`](https://github.com/hydro-project/hydro/commit/17da2726ff302e3e9bd70824e4cdf4ba808df7ec))
 * **[#1084](https://github.com/hydro-project/hydro/issues/1084)**
    - Appease various clippy lints ([`65c7ebe`](https://github.com/hydro-project/hydro/commit/65c7ebe3d64c478e7a4f0d8eb12e2bb3c1b267a3))
 * **Uncategorized**
    - Release hydroflow_lang v0.6.0, hydroflow_datalog_core v0.6.0, hydroflow_datalog v0.6.0, hydroflow_macro v0.6.0, lattices v0.5.3, variadics v0.0.4, pusherator v0.0.5, hydroflow v0.6.0, stageleft v0.2.0, hydroflow_plus v0.6.0, hydro_deploy v0.6.0, hydro_cli v0.6.0, hydroflow_plus_cli_integration v0.6.0, safety bump 7 crates ([`0e94db4`](https://github.com/hydro-project/hydro/commit/0e94db41c842c1181574c5e69179027cfa7a19cf))
    - Fix imports for clippy ([`6b0a78b`](https://github.com/hydro-project/hydro/commit/6b0a78ba0b4fd58302f7151254976c158a61b18c))
</details>

## 0.5.2 (2024-02-02)

### New Features

 - <csr-id-c636fd073a070a3e4ca67a8e33908d4c9be7a536/> add map_union_with_tombstones, fix #336

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1052](https://github.com/hydro-project/hydro/issues/1052)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1052](https://github.com/hydro-project/hydro/issues/1052)**
    - Add map_union_with_tombstones, fix #336 ([`c636fd0`](https://github.com/hydro-project/hydro/commit/c636fd073a070a3e4ca67a8e33908d4c9be7a536))
 * **Uncategorized**
    - Release hydroflow_lang v0.5.2, hydroflow_datalog_core v0.5.2, hydroflow_macro v0.5.2, lattices v0.5.2, hydroflow v0.5.2, hydro_cli v0.5.1, hydroflow_plus_cli_integration v0.5.1 ([`c6af815`](https://github.com/hydro-project/hydro/commit/c6af815d0dc1133477cfd54e0159939f337bf94f))
</details>

## 0.5.1 (2024-01-29)

<csr-id-1b555e57c8c812bed4d6495d2960cbf77fb0b3ef/>
<csr-id-ba6afab8416ad66eee4fdb9d0c73e62d45752617/>
<csr-id-f6a729925ddeb6063fa8c4b03d6621c1c35f0cc8/>
<csr-id-7c48faf0d8301b498fa59e5eee5cddf5fa341229/>
<csr-id-d08ceffdbe87215d942b8c24815cabc7909822f5/>
<csr-id-18c1fa5c6602dbf660bffbb06290f6db373312cc/>

### New Features

 - <csr-id-9846d82567e6d7c129e6962c874e552e363af2fa/> Add `DeepReveal` trait
 - <csr-id-5c63873430ecefb10302f8e4f47a5a70d01a748b/> add set_union_with_tombstones

### Bug Fixes

 - <csr-id-b4b8ca9bf35793dbc4d7e351898522d76e4ab0a3/> chat and two_pc no longer replay

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 8 commits contributed to the release.
 - 6 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 4 unique issues were worked on: [#1032](https://github.com/hydro-project/hydro/issues/1032), [#942](https://github.com/hydro-project/hydro/issues/942), [#960](https://github.com/hydro-project/hydro/issues/960), [#967](https://github.com/hydro-project/hydro/issues/967)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1032](https://github.com/hydro-project/hydro/issues/1032)**
    - Fixup! feat(lattices): Add `DeepReveal` trait ([`06b0b90`](https://github.com/hydro-project/hydro/commit/06b0b90a6a09d982dde071e609bc2cbf8350bfdf))
    - Add `DeepReveal` trait ([`9846d82`](https://github.com/hydro-project/hydro/commit/9846d82567e6d7c129e6962c874e552e363af2fa))
 * **[#942](https://github.com/hydro-project/hydro/issues/942)**
    - Fix `clippy::items_after_test_module`, simplify rustdoc links ([`18c1fa5`](https://github.com/hydro-project/hydro/commit/18c1fa5c6602dbf660bffbb06290f6db373312cc))
 * **[#960](https://github.com/hydro-project/hydro/issues/960)**
    - Fix clippy lints on latest nightly ([`d08ceff`](https://github.com/hydro-project/hydro/commit/d08ceffdbe87215d942b8c24815cabc7909822f5))
 * **[#967](https://github.com/hydro-project/hydro/issues/967)**
    - Chat and two_pc no longer replay ([`b4b8ca9`](https://github.com/hydro-project/hydro/commit/b4b8ca9bf35793dbc4d7e351898522d76e4ab0a3))
 * **Uncategorized**
    - Release hydroflow_cli_integration v0.5.1, hydroflow_lang v0.5.1, hydroflow_datalog_core v0.5.1, hydroflow_datalog v0.5.1, hydroflow_macro v0.5.1, lattices v0.5.1, variadics v0.0.3, pusherator v0.0.4, hydroflow v0.5.1, stageleft_macro v0.1.0, stageleft v0.1.0, hydroflow_plus v0.5.1, hydro_deploy v0.5.1, hydro_cli v0.5.1 ([`5a5e6d5`](https://github.com/hydro-project/hydro/commit/5a5e6d5933cf3c20ff23768d4592b0dde94e940b))
    - Manually set lockstep-versioned crates (and `lattices`) to version `0.5.1` ([`7c48faf`](https://github.com/hydro-project/hydro/commit/7c48faf0d8301b498fa59e5eee5cddf5fa341229))
    - Add set_union_with_tombstones ([`5c63873`](https://github.com/hydro-project/hydro/commit/5c63873430ecefb10302f8e4f47a5a70d01a748b))
</details>

## 0.5.0 (2023-10-11)

<csr-id-e788989737fbd501173bc99c6f9f5f5ba514ec9c/>
<csr-id-e89dcfcdd2d3ad072ae3ddb8211116fec9332fed/>

### Documentation

 - <csr-id-d780f08767a8e632ebcadcc4d780cdff633cdea9/> fix lattice math link

### New Features

 - <csr-id-f80490e6e2d9967471c670e5100d9af502bbabd2/> Add serde derives to collections
 - <csr-id-7ad05ead59c4b334536bb50c99ef17b4a0dba07f/> Add `UnionFind` lattice

### Bug Fixes (BREAKING)

 - <csr-id-53be8c8bd7eba970ffbba27995f0c93f1f8a6ea5/> fix some types and semantics for atomization

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
    - Add `UnionFind` lattice ([`7ad05ea`](https://github.com/hydro-project/hydro/commit/7ad05ead59c4b334536bb50c99ef17b4a0dba07f))
    - Fix some types and semantics for atomization ([`53be8c8`](https://github.com/hydro-project/hydro/commit/53be8c8bd7eba970ffbba27995f0c93f1f8a6ea5))
 * **[#922](https://github.com/hydro-project/hydro/issues/922)**
    - Add serde derives to collections ([`f80490e`](https://github.com/hydro-project/hydro/commit/f80490e6e2d9967471c670e5100d9af502bbabd2))
 * **Uncategorized**
    - Release hydroflow_macro v0.5.0, lattices v0.5.0, hydroflow v0.5.0, hydro_cli v0.5.0 ([`ee00056`](https://github.com/hydro-project/hydro/commit/ee000564aae553adeb5655d39bc9923de9d762bb))
    - Release hydroflow_lang v0.5.0, hydroflow_datalog_core v0.5.0, hydroflow_datalog v0.5.0, hydroflow_macro v0.5.0, lattices v0.5.0, hydroflow v0.5.0, hydro_cli v0.5.0, safety bump 4 crates ([`582d9aa`](https://github.com/hydro-project/hydro/commit/582d9aabc5575ac5433ecadc2047c2ef495af3e5))
    - Fix lattice math link ([`d780f08`](https://github.com/hydro-project/hydro/commit/d780f08767a8e632ebcadcc4d780cdff633cdea9))
    - Fix `clippy::implied_bounds_in_impls` from latest nightlies ([`e89dcfc`](https://github.com/hydro-project/hydro/commit/e89dcfcdd2d3ad072ae3ddb8211116fec9332fed))
</details>

## 0.4.0 (2023-08-15)

<csr-id-f60053f70da3071c54de4a0eabb059a143aa2ccc/>
<csr-id-6a2ad6b770c2ccf470548320d8753025b3a66c0a/>
<csr-id-262166e7cecf8ffb5a2c7bc989e8cf66c4524a68/>
<csr-id-7b0485b20939ec86ed8e74ecc9c75ac1b5d01072/>
<csr-id-f36ccd34f349b85ec39ad432b9f68b6f34dde532/>
<csr-id-e0d1061908f94ea8282be08598d783393512bb34/>
<csr-id-4a8f46a3f8f46e9493acf0900a4ac09ce4dc9dfb/>
<csr-id-dd270adee8ed4d29a20628c4082b0f29cfd6ebac/>

### Documentation

 - <csr-id-8a4528c31a9c6c9407e94a6b999b41cb0c5b4407/> Improve `Atomize` docs

### New Features

 - <csr-id-c07254d4bcdc89b12a90a990de13eacafe8b06a4/> formalize `Default::default()` as returning bottom for lattice types
   Not a breaking change since changed names were introduced only since last release
 - <csr-id-90714dbe0df85db84b1929e5d1a037a98ba2cc4f/> Implement `SimpleKeyedRef` for map types
 - <csr-id-a5014a435094bc1475f1fc34b5b947a21497f7d9/> Add atomize trait, impls, tests

### New Features (BREAKING)

* `WithTop(Some(TOP))` and `WithTop(None)` are now considered to both be top, equal.
* `check_lattice_bot/top` now check that `is_bot` and `is_top` must be consistent among all equal elements
 - <csr-id-e09ac1cc2cb5c75e47ee2c7403ade7bf8d78cf1a/> Add bottom (+top) collapsing, implement `IsBot`/`IsTop` for all lattice types
   * `WithBot(Some(BOTTOM))` and `WithBot(None)` are now considered to both be bottom, equal. Also, `MapUnion({})` and `MapUnion({key: BOTTOM})` are considered to both be bottom, equal.
* `WithTop(Some(TOP))` and `WithTop(None)` are now considered to both be top, equal.
* `check_lattice_bot/top` now check that `is_bot` and `is_top` must be consistent among all equal elements

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
    - Fix lint, format errors for latest nightly version (without updated pinned) ([`dd270ad`](https://github.com/hydro-project/hydro/commit/dd270adee8ed4d29a20628c4082b0f29cfd6ebac))
 * **[#849](https://github.com/hydro-project/hydro/issues/849)**
    - Rename `Seq` -> `VecUnion` ([`f36ccd3`](https://github.com/hydro-project/hydro/commit/f36ccd34f349b85ec39ad432b9f68b6f34dde532))
 * **[#854](https://github.com/hydro-project/hydro/issues/854)**
    - Add atomize trait, impls, tests ([`a5014a4`](https://github.com/hydro-project/hydro/commit/a5014a435094bc1475f1fc34b5b947a21497f7d9))
 * **[#860](https://github.com/hydro-project/hydro/issues/860)**
    - Improve `Atomize` docs ([`8a4528c`](https://github.com/hydro-project/hydro/commit/8a4528c31a9c6c9407e94a6b999b41cb0c5b4407))
 * **[#865](https://github.com/hydro-project/hydro/issues/865)**
    - Add bottom (+top) collapsing, implement `IsBot`/`IsTop` for all lattice types ([`e09ac1c`](https://github.com/hydro-project/hydro/commit/e09ac1cc2cb5c75e47ee2c7403ade7bf8d78cf1a))
 * **[#866](https://github.com/hydro-project/hydro/issues/866)**
    - Implement `SimpleKeyedRef` for map types ([`90714db`](https://github.com/hydro-project/hydro/commit/90714dbe0df85db84b1929e5d1a037a98ba2cc4f))
 * **[#867](https://github.com/hydro-project/hydro/issues/867)**
    - Change `Atomize` to require returning empty iff lattice is bottom ([`4a8f46a`](https://github.com/hydro-project/hydro/commit/4a8f46a3f8f46e9493acf0900a4ac09ce4dc9dfb))
 * **[#879](https://github.com/hydro-project/hydro/issues/879)**
    - Formalize `Default::default()` as returning bottom for lattice types ([`c07254d`](https://github.com/hydro-project/hydro/commit/c07254d4bcdc89b12a90a990de13eacafe8b06a4))
 * **Uncategorized**
    - Release hydroflow_lang v0.4.0, hydroflow_datalog_core v0.4.0, hydroflow_datalog v0.4.0, hydroflow_macro v0.4.0, lattices v0.4.0, pusherator v0.0.3, hydroflow v0.4.0, hydro_cli v0.4.0, safety bump 4 crates ([`8d53ee5`](https://github.com/hydro-project/hydro/commit/8d53ee51686b41e403c2e91de23dfa7b8f9d1583))
    - Fix new clippy lints on latest nightly 1.73.0-nightly (db7ff98a7 2023-07-31) ([`e0d1061`](https://github.com/hydro-project/hydro/commit/e0d1061908f94ea8282be08598d783393512bb34))
</details>

## 0.3.0 (2023-07-04)

<csr-id-0cbbaeaec5e192e2539771bb247926271c2dc4a3/>
<csr-id-70c88a51c4c83a4dc2fc67a0cd344786a4ff26f7/>
<csr-id-4a727ecf1232e0f03f5300547282bfbe73342cfa/>
<csr-id-5c7e4d3aea1dfb61d51bcb0291740281824e3090/>
<csr-id-1bdadb82b25941d11f3fa24eaac35109927c852f/>
<csr-id-336172dcaa31ea281ff534a09e13f9ff1c41e154/>
<csr-id-fe38515c456625c5374843d2f766f401e76dc51a/>
<csr-id-0f2e768fcf359de671bc6289a1d44502057c2656/>
<csr-id-618a18b89a699f9272241ef97994e9dbbfe724ad/>
<csr-id-1c739496f8286269a0cd47753468998fd759bf4e/>

### Documentation

 - <csr-id-8ecc14760210fe0d715123548a61d0406a03ffde/> List `WithTop` in README 4/4

### New Features

 - <csr-id-6cc1079f2587dfa85555efba6c122ec19f5a0751/> make unit `()` a point lattice
 - <csr-id-8f8c148ca34b0c4a909c4486a77f4272c1cb899e/> impl `IsTop`, `IsBot` for `Min`, `Max` over numeric types
 - <csr-id-a173f8396f4b67df9b407702457fb47308eb6323/> Add `Conflict<T>` lattice
 - <csr-id-eb66ee05c8afe78cebb4fe9b522a687afd6f6e76/> add top lattice, opposite of bottom
 - <csr-id-d9a60d0196c2e48ed1764c828086a3f3b3b5d25b/> Add `Seq` lattice.

### Bug Fixes

* fix: address comments
 - <csr-id-902d426dfec7754cbe949d80c669e3d3f1a1d262/> removed unused nightly features `impl_trait_in_assoc_type`, `type_alias_impl_trait`
 - <csr-id-dd95beacee1ab67047c964643762b8364073b6a2/> fix ConvertFrom for bottom to actually convert the type
   * fix: fix type inference with doubly-nested bottom types
* fix: address comments

### New Features (BREAKING)

<csr-id-deb26af6bcd547f91bf339367387d36e5e59565a/>
<csr-id-07d115443b54e94d9a03240d12b88be5e3f2883f/>
<csr-id-37e90cd9bf917b5ffa724e79791c5e87db4c1450/>
<csr-id-6d49db05d30692b70825b4cd6af1590913913ae4/>


### Bug Fixes (BREAKING)

 - <csr-id-87cc3c83847da4e616b502a638337c51bb6bf9bf/> Remove `Default` impl for `WithTop` 3/4
   Is confusing, probably not what users want.

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
    - Fix ConvertFrom for bottom to actually convert the type ([`dd95bea`](https://github.com/hydro-project/hydro/commit/dd95beacee1ab67047c964643762b8364073b6a2))
 * **[#744](https://github.com/hydro-project/hydro/issues/744)**
    - Add top lattice, opposite of bottom ([`eb66ee0`](https://github.com/hydro-project/hydro/commit/eb66ee05c8afe78cebb4fe9b522a687afd6f6e76))
 * **[#761](https://github.com/hydro-project/hydro/issues/761)**
    - Rename `Immut` -> `Point` lattice. ([`0f2e768`](https://github.com/hydro-project/hydro/commit/0f2e768fcf359de671bc6289a1d44502057c2656))
 * **[#763](https://github.com/hydro-project/hydro/issues/763)**
    - List `WithTop` in README 4/4 ([`8ecc147`](https://github.com/hydro-project/hydro/commit/8ecc14760210fe0d715123548a61d0406a03ffde))
    - Remove `Default` impl for `WithTop` 3/4 ([`87cc3c8`](https://github.com/hydro-project/hydro/commit/87cc3c83847da4e616b502a638337c51bb6bf9bf))
    - Rename `Bottom` -> `WithBot`, `Top` -> `WithTop`, constructors now take `Option`s 2/4 ([`fe38515`](https://github.com/hydro-project/hydro/commit/fe38515c456625c5374843d2f766f401e76dc51a))
    - Rename `bottom.rs` -> `with_bot.rs`, `top.rs` -> `with_top.rs` 1/4 ([`1c73949`](https://github.com/hydro-project/hydro/commit/1c739496f8286269a0cd47753468998fd759bf4e))
 * **[#765](https://github.com/hydro-project/hydro/issues/765)**
    - Rename `ConvertFrom::from` -> `LatticeFrom::lattice_from` ([`336172d`](https://github.com/hydro-project/hydro/commit/336172dcaa31ea281ff534a09e13f9ff1c41e154))
 * **[#766](https://github.com/hydro-project/hydro/issues/766)**
    - Add `IsBot::is_bot` and `IsTop::is_top` traits ([`6d49db0`](https://github.com/hydro-project/hydro/commit/6d49db05d30692b70825b4cd6af1590913913ae4))
 * **[#767](https://github.com/hydro-project/hydro/issues/767)**
    - Add `Conflict<T>` lattice ([`a173f83`](https://github.com/hydro-project/hydro/commit/a173f8396f4b67df9b407702457fb47308eb6323))
 * **[#772](https://github.com/hydro-project/hydro/issues/772)**
    - Add `Provenance` generic param token to `Point`. ([`37e90cd`](https://github.com/hydro-project/hydro/commit/37e90cd9bf917b5ffa724e79791c5e87db4c1450))
 * **[#773](https://github.com/hydro-project/hydro/issues/773)**
    - `warn` missing docs (instead of `deny`) to allow code before docs ([`618a18b`](https://github.com/hydro-project/hydro/commit/618a18b89a699f9272241ef97994e9dbbfe724ad))
 * **[#780](https://github.com/hydro-project/hydro/issues/780)**
    - Removed unused nightly features `impl_trait_in_assoc_type`, `type_alias_impl_trait` ([`902d426`](https://github.com/hydro-project/hydro/commit/902d426dfec7754cbe949d80c669e3d3f1a1d262))
 * **[#789](https://github.com/hydro-project/hydro/issues/789)**
    - Add `reveal` methods, make fields private ([`07d1154`](https://github.com/hydro-project/hydro/commit/07d115443b54e94d9a03240d12b88be5e3f2883f))
 * **[#793](https://github.com/hydro-project/hydro/issues/793)**
    - Make unit `()` a point lattice ([`6cc1079`](https://github.com/hydro-project/hydro/commit/6cc1079f2587dfa85555efba6c122ec19f5a0751))
    - Impl `IsTop`, `IsBot` for `Min`, `Max` over numeric types ([`8f8c148`](https://github.com/hydro-project/hydro/commit/8f8c148ca34b0c4a909c4486a77f4272c1cb899e))
 * **Uncategorized**
    - Release hydroflow_cli_integration v0.3.0, hydroflow_lang v0.3.0, hydroflow_datalog_core v0.3.0, hydroflow_datalog v0.3.0, hydroflow_macro v0.3.0, lattices v0.3.0, pusherator v0.0.2, hydroflow v0.3.0, hydro_cli v0.3.0, safety bump 5 crates ([`c1ac8a0`](https://github.com/hydro-project/hydro/commit/c1ac8a0c95d4fee82fa55c0c4273091d168f8b86))
    - Add `Seq` lattice. ([`d9a60d0`](https://github.com/hydro-project/hydro/commit/d9a60d0196c2e48ed1764c828086a3f3b3b5d25b))
</details>

## 0.2.0 (2023-05-31)

<csr-id-fd896fbe925fbd8ef1d16be7206ac20ba585081a/>
<csr-id-10b308532245db8f4480ce53b67aea050ae1918d/>
<csr-id-c0f165e32a1dcdcadefe6cdcf0b068a31ef9d1d7/>
<csr-id-b94cf68343c5dcaaaa0c18bb068f435441f32b09/>

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
    - Release hydroflow_lang v0.2.0, hydroflow_datalog_core v0.2.0, hydroflow_datalog v0.2.0, hydroflow_macro v0.2.0, lattices v0.2.0, hydroflow v0.2.0, hydro_cli v0.2.0 ([`6b51d7d`](https://github.com/hydro-project/hydro/commit/6b51d7dfa577fd72a041768981c2c7bae9803c4c))
    - Manually bump versions for v0.2.0 release ([`b94cf68`](https://github.com/hydro-project/hydro/commit/b94cf68343c5dcaaaa0c18bb068f435441f32b09))
    - Rename `Fake` -> `Immut` ([`c0f165e`](https://github.com/hydro-project/hydro/commit/c0f165e32a1dcdcadefe6cdcf0b068a31ef9d1d7))
</details>

## 0.1.2 (2023-05-30)

### New Features

 - <csr-id-efde5811ba9b3ded39ea30e2f84579521cc092e5/> print out items when lattice identity tests fail

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
    - Print out items when lattice identity tests fail ([`efde581`](https://github.com/hydro-project/hydro/commit/efde5811ba9b3ded39ea30e2f84579521cc092e5))
 * **Uncategorized**
    - Release hydroflow_cli_integration v0.1.1, hydroflow_lang v0.1.1, hydroflow_datalog_core v0.1.1, hydroflow_macro v0.1.1, lattices v0.1.2, hydroflow v0.1.1, hydro_cli v0.1.0 ([`023e8e9`](https://github.com/hydro-project/hydro/commit/023e8e9ab6949accc2fbc21a93ffa2b3767b73b9))
</details>

## 0.1.1 (2023-05-23)

<csr-id-3bee6f858a78d82b7431e124ef9792002c8d77ce/>
<csr-id-0d8930b94a1ff3e3f22924a505721d217f632446/>

### Documentation

 - <csr-id-d4d3d42438a3885002a5c07483e7ff364219e5c1/> simplified explanations, fixed typos, removed dead named links
 - <csr-id-e7927026703fc7f12faacefb1e10b1531de7359e/> update links from old to new book

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
    - Migrate docs to a unified Docusuarus site ([`a41bfad`](https://github.com/hydro-project/hydro/commit/a41bfad5a450f62062e4c41e6edacbfd02197c7e))
 * **[#674](https://github.com/hydro-project/hydro/issues/674)**
    - Update cc-traits to v2, remove `SimpleKeyedRef` shim ([`0d8930b`](https://github.com/hydro-project/hydro/commit/0d8930b94a1ff3e3f22924a505721d217f632446))
 * **[#687](https://github.com/hydro-project/hydro/issues/687)**
    - Simplified explanations, fixed typos, removed dead named links ([`d4d3d42`](https://github.com/hydro-project/hydro/commit/d4d3d42438a3885002a5c07483e7ff364219e5c1))
    - Update links from old to new book ([`e792702`](https://github.com/hydro-project/hydro/commit/e7927026703fc7f12faacefb1e10b1531de7359e))
 * **Uncategorized**
    - Release hydroflow_cli_integration v0.1.0, hydroflow_internalmacro v0.1.0, hydroflow_lang v0.1.0, hydroflow_datalog_core v0.1.0, hydroflow_datalog v0.1.0, hydroflow_macro v0.1.0, lattices v0.1.1, hydroflow v0.1.0 ([`61e906a`](https://github.com/hydro-project/hydro/commit/61e906aa7541fef42bfe91a50f31082f7780dc0f))
</details>

## 0.1.0 (2023-05-21)

<csr-id-cd0a86d9271d0e3daab59c46f079925f863424e1/>
<csr-id-20a1b2c0cd04a8b495a02ce345db3d48a99ea0e9/>
<csr-id-1eda91a2ef8794711ef037240f15284e8085d863/>
<csr-id-7818bafa3361890101864f82815b1c94130d97f4/>
<csr-id-21a503e795593173b1fd114d70a7cfad3e79ecfe/>
<csr-id-2a144a622682a958d44377df71a71b59cf1b39c4/>

### Documentation

<csr-id-fc8f73980d0cf711bf6ac3fcb8558540d0f05acd/>


### New Features

 - <csr-id-8ad06384c88aea30fbb168901d5ba5ec25d9d2bb/> Make lattice test helpers public, restructure
   Also impl `LatticeOrd` for `SetUnion`

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 14 commits contributed to the release.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 10 unique issues were worked on: [#625](https://github.com/hydro-project/hydro/issues/625), [#637](https://github.com/hydro-project/hydro/issues/637), [#638](https://github.com/hydro-project/hydro/issues/638), [#642](https://github.com/hydro-project/hydro/issues/642), [#644](https://github.com/hydro-project/hydro/issues/644), [#645](https://github.com/hydro-project/hydro/issues/645), [#658](https://github.com/hydro-project/hydro/issues/658), [#660](https://github.com/hydro-project/hydro/issues/660), [#664](https://github.com/hydro-project/hydro/issues/664), [#667](https://github.com/hydro-project/hydro/issues/667)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#625](https://github.com/hydro-project/hydro/issues/625)**
    - Use `cc-traits` instead of own `Collection`, remove `tag` indirection ([`7d899de`](https://github.com/hydro-project/hydro/commit/7d899de5b062bc4f54ed008e3510a7b6572683d3))
 * **[#637](https://github.com/hydro-project/hydro/issues/637)**
    - Add bottom and fake lattice types ([`cbf04a6`](https://github.com/hydro-project/hydro/commit/cbf04a632a0c241575a552d60097a38462ba5fcd))
 * **[#638](https://github.com/hydro-project/hydro/issues/638)**
    - Remove old lattice code ([`0f71738`](https://github.com/hydro-project/hydro/commit/0f7173813d7a2a2f16e9d5f52eb68aa857e068c3))
 * **[#642](https://github.com/hydro-project/hydro/issues/642)**
    - Remove zmq, use unsync channels locally, use sync mpsc cross-thread, use cross_join+enumerate instead of broadcast channel,remove Eq requirement from multisetjoin ([`8cc1261`](https://github.com/hydro-project/hydro/commit/8cc1261873c106360305b3df9d3eaedb61637414))
 * **[#644](https://github.com/hydro-project/hydro/issues/644)**
    - Remove Compare trait, add tests, make all lattice types PartialOrd, Eq, PartialEq ([`a1cabbf`](https://github.com/hydro-project/hydro/commit/a1cabbfe7b5acf4a4accad8971602cc1757aa96f))
 * **[#645](https://github.com/hydro-project/hydro/issues/645)**
    - Fix `Pair` `PartialOrd` implementation, add consistency tests with `NaiveOrd` ([`d5b5b70`](https://github.com/hydro-project/hydro/commit/d5b5b7094c9e1743a0174cbf2a84918deb6bcff5))
 * **[#658](https://github.com/hydro-project/hydro/issues/658)**
    - Allow fake to merge, compare equal values ([`0680009`](https://github.com/hydro-project/hydro/commit/0680009a35f0701a05a31cb2dec4e40ebbf77f60))
 * **[#660](https://github.com/hydro-project/hydro/issues/660)**
    - Warn lint `unused_qualifications` ([`7818baf`](https://github.com/hydro-project/hydro/commit/7818bafa3361890101864f82815b1c94130d97f4))
    - Rustfmt group imports ([`21a503e`](https://github.com/hydro-project/hydro/commit/21a503e795593173b1fd114d70a7cfad3e79ecfe))
    - Rustfmt prescribe flat-module `use` format ([`2a144a6`](https://github.com/hydro-project/hydro/commit/2a144a622682a958d44377df71a71b59cf1b39c4))
 * **[#664](https://github.com/hydro-project/hydro/issues/664)**
    - Make lattice test helpers public, restructure ([`8ad0638`](https://github.com/hydro-project/hydro/commit/8ad06384c88aea30fbb168901d5ba5ec25d9d2bb))
 * **[#667](https://github.com/hydro-project/hydro/issues/667)**
    - Bump lattices version to `0.1.0` ([`40ae27f`](https://github.com/hydro-project/hydro/commit/40ae27fa6296eaf8f665abd6d99aa0688a4b3013))
    - Update docs, add book chapter for `lattices` crate ([`fc8f739`](https://github.com/hydro-project/hydro/commit/fc8f73980d0cf711bf6ac3fcb8558540d0f05acd))
 * **Uncategorized**
    - Release hydroflow_cli_integration v0.0.1, hydroflow_lang v0.0.1, hydroflow_datalog_core v0.0.1, hydroflow_datalog v0.0.1, hydroflow_macro v0.0.1, lattices v0.1.0, variadics v0.0.2, pusherator v0.0.1, hydroflow v0.0.2 ([`d91ebc9`](https://github.com/hydro-project/hydro/commit/d91ebc9e8e23965089c929558a09fc430ee72f2c))
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
    - Fixup! Move lattice2 into new separate `lattices` crate ([`49058f0`](https://github.com/hydro-project/hydro/commit/49058f0547dde10c0d84ec5f349ecf5e6aa6315b))
    - Move lattice2 into new separate `lattices` crate ([`7881716`](https://github.com/hydro-project/hydro/commit/788171642b090a282412614ef862143357431f5c))
 * **[#636](https://github.com/hydro-project/hydro/issues/636)**
    - Fixup! Move lattice2 into new separate `lattices` crate ([`49058f0`](https://github.com/hydro-project/hydro/commit/49058f0547dde10c0d84ec5f349ecf5e6aa6315b))
</details>

