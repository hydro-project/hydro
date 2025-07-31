

## v0.14.0 (2025-07-31)

<csr-id-5ab815f3567d51e9bd114f90af8e837fe0732cd8/>

### New Features

 - <csr-id-17f4a832dac816902eebd19118dc2c4902953261/> Aggregate client throughput/latency
   Co-authored with @shadaj
   
   ---------
 - <csr-id-b333b45e0936bbe481d7fbc285790d942779c494/> upgrade Stageleft to eliminate `__staged` compilation during development
   Before Stageleft 0.9, we always compiled the `__staged` module in stage
   0, which resulted in significant compilation penalties and Rust Analyzer
   thrashing since any file changes triggered a re-run of the `build.rs`.
   With Stageleft 0.9, we can defer compiling this module to the trybuild
   stage 1.
   
   Stageleft 0.9 also cleans up how paths are rewritten to use the
   `__staged` module, so we can simplify our logic as well. The only
   significant rewrite remaining is when running unit tests, where we have
   to regenerate `__staged` to access test-only module, and therefore have
   to rewrite all paths to use that module.
   
   Finally, in the spirit of improving compilation efficiency, we disable
   incremental builds for trybuild stage 1. We generate files with hash
   based on contents, so we were never benefitting from incremental
   compilation anyways. This reduces the disk space used significantly.

### Refactor

 - <csr-id-5ab815f3567d51e9bd114f90af8e837fe0732cd8/> use `async-ssh2-russh` (instead of `libssh2` bindings), fix #1463

### New Features (BREAKING)

 - <csr-id-45bd6e9759410dcb747c9224758c82f9874378d2/> add stream markers for tracking non-deterministic retries
   This introduces an additional type paramter to `Stream` called
   `Retries`, which tracks the presence (or lack) of non-determinstic
   retries in the stream. `ExactlyOnce` means that each element has
   deterministic order, while `AtLeastOnce` means that there may be
   non-deterministic duplicates.
   
   A `TotalOrder, AtLeastOnce` stream describes elements with consecutive
   duplication, but deterministic order if we ignore those immediate
   elements. A `NoOrder, AtLeastOnce` stream has set semantics.
   
   Also fixes a bug in the return type for `*_keyed_*`, where the output
   type was previously `TotalOrder` but now is `NoOrder`. We stream the
   results of a keyed aggregation out of a `HashMap`, so the order will
   indeed be non-deterministic.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release over the course of 91 calendar days.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 4 unique issues were worked on: [#1803](https://github.com/hydro-project/hydro/issues/1803), [#1900](https://github.com/hydro-project/hydro/issues/1900), [#1907](https://github.com/hydro-project/hydro/issues/1907), [#1910](https://github.com/hydro-project/hydro/issues/1910)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1803](https://github.com/hydro-project/hydro/issues/1803)**
    - Use `async-ssh2-russh` (instead of `libssh2` bindings), fix #1463 ([`5ab815f`](https://github.com/hydro-project/hydro/commit/5ab815f3567d51e9bd114f90af8e837fe0732cd8))
 * **[#1900](https://github.com/hydro-project/hydro/issues/1900)**
    - Aggregate client throughput/latency ([`17f4a83`](https://github.com/hydro-project/hydro/commit/17f4a832dac816902eebd19118dc2c4902953261))
 * **[#1907](https://github.com/hydro-project/hydro/issues/1907)**
    - Upgrade Stageleft to eliminate `__staged` compilation during development ([`b333b45`](https://github.com/hydro-project/hydro/commit/b333b45e0936bbe481d7fbc285790d942779c494))
 * **[#1910](https://github.com/hydro-project/hydro/issues/1910)**
    - Add stream markers for tracking non-deterministic retries ([`45bd6e9`](https://github.com/hydro-project/hydro/commit/45bd6e9759410dcb747c9224758c82f9874378d2))
 * **Uncategorized**
    - Release dfir_lang v0.14.0, dfir_macro v0.14.0, hydro_deploy_integration v0.14.0, lattices_macro v0.5.10, variadics_macro v0.6.1, dfir_rs v0.14.0, hydro_deploy v0.14.0, hydro_lang v0.14.0, hydro_optimize v0.13.0, hydro_std v0.14.0, safety bump 6 crates ([`0683595`](https://github.com/hydro-project/hydro/commit/06835950c12884d661100c13f73ad23a98bfad9f))
</details>

## v0.13.0 (2025-04-11)

### New Features

 - <csr-id-5ac247ca2006bbb45c5511c78dc6d9028f7451da/> update Stageleft and reduce reliance on DFIR re-exports
 - <csr-id-e0c4abb02054fc3d5dc866286b18f3f2bcd2ad36/> update Stageleft to reduce viral dependencies
   Now that Stageleft handles quoted snippets that refer to local
   dependencies, we do not need to duplicate deps into downstream crates.

### New Features (BREAKING)

 - <csr-id-dfb7a1b5ad47f03822e9b7cae7dae81914b305e2/> don't pull in dfir_rs during the compilation stage
   Because `hydro_lang` is responsible for _generating_ DFIR code, it
   doesn't actually need to depend on the runtime (`dfir_rs`), other than
   when it is used in the (legacy) macro mode or when we want to include
   utilities for runtime logic (`resource_measurement`). This sticks those
   pieces under feature flags and makes `dfir_rs` an optional dependency,
   which reduces the compile tree for crates like `hydro_test`.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 27 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 3 unique issues were worked on: [#1791](https://github.com/hydro-project/hydro/issues/1791), [#1796](https://github.com/hydro-project/hydro/issues/1796), [#1797](https://github.com/hydro-project/hydro/issues/1797)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1791](https://github.com/hydro-project/hydro/issues/1791)**
    - Update Stageleft to reduce viral dependencies ([`e0c4abb`](https://github.com/hydro-project/hydro/commit/e0c4abb02054fc3d5dc866286b18f3f2bcd2ad36))
 * **[#1796](https://github.com/hydro-project/hydro/issues/1796)**
    - Update Stageleft and reduce reliance on DFIR re-exports ([`5ac247c`](https://github.com/hydro-project/hydro/commit/5ac247ca2006bbb45c5511c78dc6d9028f7451da))
 * **[#1797](https://github.com/hydro-project/hydro/issues/1797)**
    - Don't pull in dfir_rs during the compilation stage ([`dfb7a1b`](https://github.com/hydro-project/hydro/commit/dfb7a1b5ad47f03822e9b7cae7dae81914b305e2))
 * **Uncategorized**
    - Release dfir_lang v0.13.0, dfir_datalog_core v0.13.0, dfir_datalog v0.13.0, dfir_macro v0.13.0, hydro_deploy_integration v0.13.0, dfir_rs v0.13.0, hydro_deploy v0.13.0, hydro_lang v0.13.0, hydro_std v0.13.0, hydro_cli v0.13.0, safety bump 8 crates ([`400fd8f`](https://github.com/hydro-project/hydro/commit/400fd8f2e8cada253f54980e7edce0631be70a82))
</details>

## v0.12.1 (2025-03-15)

<csr-id-38e6721be69f6a41aa47a01a9d06d56a01be1355/>

### Chore

 - <csr-id-38e6721be69f6a41aa47a01a9d06d56a01be1355/> remove stageleft from repo, fix #1764
   They grow up so fast 🥹

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

### New Features

 - <csr-id-7f0a9e8ef59adf462ddd4b798811ec32e61bcb47/> add benchmarking utilities

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release.
 - 7 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 3 unique issues were worked on: [#1765](https://github.com/hydro-project/hydro/issues/1765), [#1774](https://github.com/hydro-project/hydro/issues/1774), [#1787](https://github.com/hydro-project/hydro/issues/1787)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1765](https://github.com/hydro-project/hydro/issues/1765)**
    - Add benchmarking utilities ([`7f0a9e8`](https://github.com/hydro-project/hydro/commit/7f0a9e8ef59adf462ddd4b798811ec32e61bcb47))
 * **[#1774](https://github.com/hydro-project/hydro/issues/1774)**
    - Remove stageleft from repo, fix #1764 ([`38e6721`](https://github.com/hydro-project/hydro/commit/38e6721be69f6a41aa47a01a9d06d56a01be1355))
 * **[#1787](https://github.com/hydro-project/hydro/issues/1787)**
    - Demote python deploy docs, fix docsrs configs, fix #1392, fix #1629 ([`b235a42`](https://github.com/hydro-project/hydro/commit/b235a42a3071e55da7b09bdc8bc710b18e0fe053))
 * **Uncategorized**
    - Release include_mdtests v0.0.0, dfir_rs v0.12.1, hydro_deploy v0.12.1, hydro_lang v0.12.1, hydro_std v0.12.1, hydro_cli v0.12.1 ([`faf0d3e`](https://github.com/hydro-project/hydro/commit/faf0d3ed9f172275f2e2f219c5ead1910c209a36))
    - Release dfir_lang v0.12.1, dfir_datalog_core v0.12.1, dfir_datalog v0.12.1, dfir_macro v0.12.1, hydro_deploy_integration v0.12.1, lattices v0.6.1, pusherator v0.0.12, dfir_rs v0.12.1, hydro_deploy v0.12.1, hydro_lang v0.12.1, hydro_std v0.12.1, hydro_cli v0.12.1 ([`23221b5`](https://github.com/hydro-project/hydro/commit/23221b53b30918707ddaa85529d04cd7919166b4))
</details>

## v0.12.0 (2025-03-08)

<csr-id-49a387d4a21f0763df8ec94de73fb953c9cd333a/>
<csr-id-41e5bb93eb9c19a88167a63bce0ceb800f8f300d/>
<csr-id-80407a2f0fdaa8b8a81688d181166a0da8aa7b52/>
<csr-id-2fd6119afed850a0c50ecc69e5c4d8de61a2f4cb/>
<csr-id-524fa67232b54f5faeb797b43070f2f197c558dd/>
<csr-id-ec3795a678d261a38085405b6e9bfea943dafefb/>

### Chore

 - <csr-id-49a387d4a21f0763df8ec94de73fb953c9cd333a/> upgrade to Rust 2024 edition
   - Updates `Cargo.toml` to use new shared workspace keys
   - Updates lint settings (in workspace `Cargo.toml`)
   - `rustfmt` has changed slightly, resulting in a big diff - there are no
   actual code changes
   - Adds a script to `rustfmt` the template src files

### Refactor (BREAKING)

 - <csr-id-2fd6119afed850a0c50ecc69e5c4d8de61a2f4cb/> rename `_interleaved` to `_anonymous`
   Also address docs feedback for streams.
 - <csr-id-524fa67232b54f5faeb797b43070f2f197c558dd/> rename timestamp to atomic and provide batching shortcuts

### Chore

 - <csr-id-ec3795a678d261a38085405b6e9bfea943dafefb/> upgrade to Rust 2024 edition
   - Updates `Cargo.toml` to use new shared workspace keys
   - Updates lint settings (in workspace `Cargo.toml`)
   - `rustfmt` has changed slightly, resulting in a big diff - there are no
   actual code changes
   - Adds a script to `rustfmt` the template src files

### Documentation

 - <csr-id-73444373dabeedd7a03a8231952684fb01bdf895/> add initial Rustdoc for some Stream APIs
 - <csr-id-d7741d55a3ea9b172e962e7398f0414d0427c3f9/> add initial Rustdoc for some Stream APIs

### New Features

 - <csr-id-eee28d3a17ea542c69a2d7e535c38333f42d4398/> Add metadata field to HydroNode
 - <csr-id-6d77db9e52ece0b668587187c59f2862670db7cf/> send_partitioned operator and move decoupling
   Allows specifying a distribution policy (for deciding which partition to
   send each message to) before networking. Designed to be as easy as
   possible to inject (so the distribution policy function definition takes
   in the cluster ID, for example, even though it doesn't need to, because
   this way we can avoid project->map->join)
 - <csr-id-69831f9dc724ba7915b8ade8134839c42786ac76/> Add metadata field to HydroNode
 - <csr-id-ca291dd618fc4065c4e30097c5ea605226383cec/> send_partitioned operator and move decoupling
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
 - <csr-id-48b275c1247f4f6fe7e6b63a5ae184c5d85b6fa1/> use correct `__staged` path when rewriting `crate::` imports
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
 - <csr-id-a7e22cdd312b8483163aa89751833e1657703b8d/> reduce where `#[cfg(stageleft_runtime)]` needs to be used
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

 - 9 commits contributed to the release.
 - 74 days passed between releases.
 - 8 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 8 unique issues were worked on: [#1632](https://github.com/hydro-project/hydro/issues/1632), [#1650](https://github.com/hydro-project/hydro/issues/1650), [#1652](https://github.com/hydro-project/hydro/issues/1652), [#1657](https://github.com/hydro-project/hydro/issues/1657), [#1681](https://github.com/hydro-project/hydro/issues/1681), [#1695](https://github.com/hydro-project/hydro/issues/1695), [#1721](https://github.com/hydro-project/hydro/issues/1721), [#1747](https://github.com/hydro-project/hydro/issues/1747)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1632](https://github.com/hydro-project/hydro/issues/1632)**
    - Add metadata field to HydroNode ([`69831f9`](https://github.com/hydro-project/hydro/commit/69831f9dc724ba7915b8ade8134839c42786ac76))
 * **[#1650](https://github.com/hydro-project/hydro/issues/1650)**
    - Add initial Rustdoc for some Stream APIs ([`d7741d5`](https://github.com/hydro-project/hydro/commit/d7741d55a3ea9b172e962e7398f0414d0427c3f9))
 * **[#1652](https://github.com/hydro-project/hydro/issues/1652)**
    - Send_partitioned operator and move decoupling ([`ca291dd`](https://github.com/hydro-project/hydro/commit/ca291dd618fc4065c4e30097c5ea605226383cec))
 * **[#1657](https://github.com/hydro-project/hydro/issues/1657)**
    - Use correct `__staged` path when rewriting `crate::` imports ([`48b275c`](https://github.com/hydro-project/hydro/commit/48b275c1247f4f6fe7e6b63a5ae184c5d85b6fa1))
 * **[#1681](https://github.com/hydro-project/hydro/issues/1681)**
    - Rename timestamp to atomic and provide batching shortcuts ([`524fa67`](https://github.com/hydro-project/hydro/commit/524fa67232b54f5faeb797b43070f2f197c558dd))
 * **[#1695](https://github.com/hydro-project/hydro/issues/1695)**
    - Rename `_interleaved` to `_anonymous` ([`2fd6119`](https://github.com/hydro-project/hydro/commit/2fd6119afed850a0c50ecc69e5c4d8de61a2f4cb))
 * **[#1721](https://github.com/hydro-project/hydro/issues/1721)**
    - Reduce where `#[cfg(stageleft_runtime)]` needs to be used ([`a7e22cd`](https://github.com/hydro-project/hydro/commit/a7e22cdd312b8483163aa89751833e1657703b8d))
 * **[#1747](https://github.com/hydro-project/hydro/issues/1747)**
    - Upgrade to Rust 2024 edition ([`ec3795a`](https://github.com/hydro-project/hydro/commit/ec3795a678d261a38085405b6e9bfea943dafefb))
 * **Uncategorized**
    - Release dfir_lang v0.12.0, dfir_datalog_core v0.12.0, dfir_datalog v0.12.0, dfir_macro v0.12.0, hydroflow_deploy_integration v0.12.0, lattices_macro v0.5.9, variadics v0.0.9, variadics_macro v0.6.0, lattices v0.6.0, multiplatform_test v0.5.0, pusherator v0.0.11, dfir_rs v0.12.0, hydro_deploy v0.12.0, stageleft_macro v0.6.0, stageleft v0.7.0, stageleft_tool v0.6.0, hydro_lang v0.12.0, hydro_std v0.12.0, hydro_cli v0.12.0, safety bump 10 crates ([`973c925`](https://github.com/hydro-project/hydro/commit/973c925e87ed78344494581bd7ce1bbb4186a2f3))
</details>

## v0.11.0 (2024-12-23)

<csr-id-03b3a349013a71b324276bca5329c33d400a73ff/>
<csr-id-162e49cf8a8cf944cded7f775d6f78afe4a89837/>
<csr-id-a6f60c92ae7168eb86eb311ca7b7afb10025c7de/>
<csr-id-54f461acfce091276b8ce7574c0690e6d648546d/>

### Chore

 - <csr-id-03b3a349013a71b324276bca5329c33d400a73ff/> bump versions manually for renamed crates, per `RELEASING.md`
 - <csr-id-162e49cf8a8cf944cded7f775d6f78afe4a89837/> Rename HydroflowPlus to Hydro

### Chore

 - <csr-id-a6f60c92ae7168eb86eb311ca7b7afb10025c7de/> bump versions manually for renamed crates, per `RELEASING.md`
 - <csr-id-54f461acfce091276b8ce7574c0690e6d648546d/> Rename HydroflowPlus to Hydro

### Documentation

 - <csr-id-28cd220c68e3660d9ebade113949a2346720cd04/> add `repository` field to `Cargo.toml`s, fix #1452
   #1452 
   
   Will trigger new releases of the following:
   `unchanged = 'hydroflow_deploy_integration', 'variadics',
   'variadics_macro', 'pusherator'`
   
   (All other crates already have changes, so would be released anyway)
 - <csr-id-6ab625273d822812e83a333e928c3dea1c3c9ccb/> cleanups for the rename, fixing links
 - <csr-id-204bd117ca3a8845b4986539efb91a0c612dfa05/> add `repository` field to `Cargo.toml`s, fix #1452
   #1452 
   
   Will trigger new releases of the following:
   `unchanged = 'hydroflow_deploy_integration', 'variadics',
   'variadics_macro', 'pusherator'`
   
   (All other crates already have changes, so would be released anyway)
 - <csr-id-987f7ad8668d9740ceea577a595035228898d530/> cleanups for the rename, fixing links

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 4 unique issues were worked on: [#1501](https://github.com/hydro-project/hydro/issues/1501), [#1617](https://github.com/hydro-project/hydro/issues/1617), [#1624](https://github.com/hydro-project/hydro/issues/1624), [#1627](https://github.com/hydro-project/hydro/issues/1627)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1501](https://github.com/hydro-project/hydro/issues/1501)**
    - Add `repository` field to `Cargo.toml`s, fix #1452 ([`204bd11`](https://github.com/hydro-project/hydro/commit/204bd117ca3a8845b4986539efb91a0c612dfa05))
 * **[#1617](https://github.com/hydro-project/hydro/issues/1617)**
    - Rename HydroflowPlus to Hydro ([`54f461a`](https://github.com/hydro-project/hydro/commit/54f461acfce091276b8ce7574c0690e6d648546d))
 * **[#1624](https://github.com/hydro-project/hydro/issues/1624)**
    - Cleanups for the rename, fixing links ([`987f7ad`](https://github.com/hydro-project/hydro/commit/987f7ad8668d9740ceea577a595035228898d530))
 * **[#1627](https://github.com/hydro-project/hydro/issues/1627)**
    - Bump versions manually for renamed crates, per `RELEASING.md` ([`a6f60c9`](https://github.com/hydro-project/hydro/commit/a6f60c92ae7168eb86eb311ca7b7afb10025c7de))
 * **Uncategorized**
    - Release stageleft_macro v0.5.0, stageleft v0.6.0, stageleft_tool v0.5.0, hydro_lang v0.11.0, hydro_std v0.11.0, hydro_cli v0.11.0 ([`7633c38`](https://github.com/hydro-project/hydro/commit/7633c38c4a56acf7e5b3b6f2a72ccc1d6e6eeba1))
    - Release dfir_lang v0.11.0, dfir_datalog_core v0.11.0, dfir_datalog v0.11.0, dfir_macro v0.11.0, hydroflow_deploy_integration v0.11.0, lattices_macro v0.5.8, variadics v0.0.8, variadics_macro v0.5.6, lattices v0.5.9, multiplatform_test v0.4.0, pusherator v0.0.10, dfir_rs v0.11.0, hydro_deploy v0.11.0, stageleft_macro v0.5.0, stageleft v0.6.0, stageleft_tool v0.5.0, hydro_lang v0.11.0, hydro_std v0.11.0, hydro_cli v0.11.0, safety bump 6 crates ([`361b443`](https://github.com/hydro-project/hydro/commit/361b4439ef9c781860f18d511668ab463a8c5203))
</details>

