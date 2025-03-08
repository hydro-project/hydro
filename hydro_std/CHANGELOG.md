

## v0.12.0 (2025-03-08)

### Chore

 - <csr-id-49a387d4a21f0763df8ec94de73fb953c9cd333a/> upgrade to Rust 2024 edition
   - Updates `Cargo.toml` to use new shared workspace keys
   - Updates lint settings (in workspace `Cargo.toml`)
   - `rustfmt` has changed slightly, resulting in a big diff - there are no
   actual code changes
   - Adds a script to `rustfmt` the template src files

### Documentation

 - <csr-id-73444373dabeedd7a03a8231952684fb01bdf895/> add initial Rustdoc for some Stream APIs

### New Features

 - <csr-id-eee28d3a17ea542c69a2d7e535c38333f42d4398/> Add metadata field to HydroNode
 - <csr-id-6d77db9e52ece0b668587187c59f2862670db7cf/> send_partitioned operator and move decoupling
   Allows specifying a distribution policy (for deciding which partition to
   send each message to) before networking. Designed to be as easy as
   possible to inject (so the distribution policy function definition takes
   in the cluster ID, for example, even though it doesn't need to, because
   this way we can avoid project->map->join)

### Bug Fixes

 - <csr-id-75eb323a612fd5d2609e464fe7690bc2b6a8457a/> use correct `__staged` path when rewriting `crate::` imports
   Previously, a rewrite would first turn `crate` into `crate::__staged`,
   and another would rewrite `crate::__staged` into `hydro_test::__staged`.
   The latter global rewrite is unnecessary because the stageleft logic
   already will use the full crate name when handling public types, so we
   drop it.

### Bug Fixes (BREAKING)

 - <csr-id-c49a4913cfdae021404a86e5a4d0597aa4db9fbe/> reduce where `#[cfg(stageleft_runtime)]` needs to be used
   Simplifies the logic for generating the public clone of the code, which
   eliminates the need to sprinkle `#[cfg(stageleft_runtime)]` (renamed
   from `#[stageleft::runtime]`) everywhere. Also adds logic to pass
   through `cfg` attrs when re-exporting public types.

### Refactor (BREAKING)

 - <csr-id-41e5bb93eb9c19a88167a63bce0ceb800f8f300d/> rename `_interleaved` to `_anonymous`
   Also address docs feedback for streams.
 - <csr-id-80407a2f0fdaa8b8a81688d181166a0da8aa7b52/> rename timestamp to atomic and provide batching shortcuts

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 8 commits contributed to the release over the course of 51 calendar days.
 - 8 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 8 unique issues were worked on: [#1632](https://github.com/hydro-project/hydro/issues/1632), [#1650](https://github.com/hydro-project/hydro/issues/1650), [#1652](https://github.com/hydro-project/hydro/issues/1652), [#1657](https://github.com/hydro-project/hydro/issues/1657), [#1681](https://github.com/hydro-project/hydro/issues/1681), [#1695](https://github.com/hydro-project/hydro/issues/1695), [#1721](https://github.com/hydro-project/hydro/issues/1721), [#1747](https://github.com/hydro-project/hydro/issues/1747)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1632](https://github.com/hydro-project/hydro/issues/1632)**
    - Add metadata field to HydroNode ([`eee28d3`](https://github.com/hydro-project/hydro/commit/eee28d3a17ea542c69a2d7e535c38333f42d4398))
 * **[#1650](https://github.com/hydro-project/hydro/issues/1650)**
    - Add initial Rustdoc for some Stream APIs ([`7344437`](https://github.com/hydro-project/hydro/commit/73444373dabeedd7a03a8231952684fb01bdf895))
 * **[#1652](https://github.com/hydro-project/hydro/issues/1652)**
    - Send_partitioned operator and move decoupling ([`6d77db9`](https://github.com/hydro-project/hydro/commit/6d77db9e52ece0b668587187c59f2862670db7cf))
 * **[#1657](https://github.com/hydro-project/hydro/issues/1657)**
    - Use correct `__staged` path when rewriting `crate::` imports ([`75eb323`](https://github.com/hydro-project/hydro/commit/75eb323a612fd5d2609e464fe7690bc2b6a8457a))
 * **[#1681](https://github.com/hydro-project/hydro/issues/1681)**
    - Rename timestamp to atomic and provide batching shortcuts ([`80407a2`](https://github.com/hydro-project/hydro/commit/80407a2f0fdaa8b8a81688d181166a0da8aa7b52))
 * **[#1695](https://github.com/hydro-project/hydro/issues/1695)**
    - Rename `_interleaved` to `_anonymous` ([`41e5bb9`](https://github.com/hydro-project/hydro/commit/41e5bb93eb9c19a88167a63bce0ceb800f8f300d))
 * **[#1721](https://github.com/hydro-project/hydro/issues/1721)**
    - Reduce where `#[cfg(stageleft_runtime)]` needs to be used ([`c49a491`](https://github.com/hydro-project/hydro/commit/c49a4913cfdae021404a86e5a4d0597aa4db9fbe))
 * **[#1747](https://github.com/hydro-project/hydro/issues/1747)**
    - Upgrade to Rust 2024 edition ([`49a387d`](https://github.com/hydro-project/hydro/commit/49a387d4a21f0763df8ec94de73fb953c9cd333a))
</details>

## v0.11.0 (2024-12-23)

<csr-id-03b3a349013a71b324276bca5329c33d400a73ff/>
<csr-id-162e49cf8a8cf944cded7f775d6f78afe4a89837/>

### Chore

 - <csr-id-03b3a349013a71b324276bca5329c33d400a73ff/> bump versions manually for renamed crates, per `RELEASING.md`
 - <csr-id-162e49cf8a8cf944cded7f775d6f78afe4a89837/> Rename HydroflowPlus to Hydro

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

 - 6 commits contributed to the release.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 4 unique issues were worked on: [#1501](https://github.com/hydro-project/hydro/issues/1501), [#1617](https://github.com/hydro-project/hydro/issues/1617), [#1624](https://github.com/hydro-project/hydro/issues/1624), [#1627](https://github.com/hydro-project/hydro/issues/1627)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1501](https://github.com/hydro-project/hydro/issues/1501)**
    - Add `repository` field to `Cargo.toml`s, fix #1452 ([`28cd220`](https://github.com/hydro-project/hydro/commit/28cd220c68e3660d9ebade113949a2346720cd04))
 * **[#1617](https://github.com/hydro-project/hydro/issues/1617)**
    - Rename HydroflowPlus to Hydro ([`162e49c`](https://github.com/hydro-project/hydro/commit/162e49cf8a8cf944cded7f775d6f78afe4a89837))
 * **[#1624](https://github.com/hydro-project/hydro/issues/1624)**
    - Cleanups for the rename, fixing links ([`6ab6252`](https://github.com/hydro-project/hydro/commit/6ab625273d822812e83a333e928c3dea1c3c9ccb))
 * **[#1627](https://github.com/hydro-project/hydro/issues/1627)**
    - Bump versions manually for renamed crates, per `RELEASING.md` ([`03b3a34`](https://github.com/hydro-project/hydro/commit/03b3a349013a71b324276bca5329c33d400a73ff))
 * **Uncategorized**
    - Release stageleft_macro v0.5.0, stageleft v0.6.0, stageleft_tool v0.5.0, hydro_lang v0.11.0, hydro_std v0.11.0, hydro_cli v0.11.0 ([`b58dccc`](https://github.com/hydro-project/hydro/commit/b58dccc7f85380951a0ae91d32548eff0784f3a7))
    - Release dfir_lang v0.11.0, dfir_datalog_core v0.11.0, dfir_datalog v0.11.0, dfir_macro v0.11.0, hydroflow_deploy_integration v0.11.0, lattices_macro v0.5.8, variadics v0.0.8, variadics_macro v0.5.6, lattices v0.5.9, multiplatform_test v0.4.0, pusherator v0.0.10, dfir_rs v0.11.0, hydro_deploy v0.11.0, stageleft_macro v0.5.0, stageleft v0.6.0, stageleft_tool v0.5.0, hydro_lang v0.11.0, hydro_std v0.11.0, hydro_cli v0.11.0, safety bump 6 crates ([`9a7e486`](https://github.com/hydro-project/hydro/commit/9a7e48693fce0face0f8ad16349258cdbe26395f))
</details>

