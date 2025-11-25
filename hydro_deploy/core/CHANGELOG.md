# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.15.0 (2025-11-25)

### Chore

 - <csr-id-83b77a16a0771a2efe86075ec98b60639a1510b7/> only depend on `samply` on supported systems, remove leftover DTrace code
   Minimizes dependencies we pull in on Unix.
 - <csr-id-97426b8a7e3b3af8a58b4c44c768c3f48cd0ed71/> update pinned nightly to 2025-08-20, fix lints

### New Features

 - <csr-id-fc3bf5bafdd225bf6a59fbb2a01c0450e8404315/> make glibc vs musl configurable for cloud deployments
 - <csr-id-891a25d3f72b2fa10a5154940624a146bb460ed7/> use dynamically linked libraries in test mode
 - <csr-id-a65893a65bc43cb13c516c95a8d86ad424446565/> share `__staged` across trybuild targets
   Previously, each trybuild target (in test-mode) would have its own copy
   of the `__staged` module containing the entire sources of the original
   crate. This creates a huge compilation burden that is repeated on every
   deployment.
   
   With an accompanying change in Stageleft, we are now able to share the
   `__staged` module in the common `lib.rs`. This also moves the simulation
   dylibs to be compiled as an example so that they do not conflict with
   the caching of the shared `lib.rs`.
   
   Also reduces thrashing due to the lockfile having to be updated after
   each time it is written (because packages have to be removed which were
   needed in the workspace but not the trybuild).
 - <csr-id-1dd2606932fa30cf1812d75a5a3733f817f344b5/> add support for aws via terraform

### Bug Fixes

 - <csr-id-f0acd7cb0e60d163c9e5a7c053ebdba995c8f289/> fix full path replacement for staged code errors
 - <csr-id-807eb0afe3f5fcbec573959b5e4323f762cebe6e/> make `cargo sim` fuzzing command work again [ci-full]
   Accidentally broken when we moved to sharing the trybuild-lib as a
   dylib.
 - <csr-id-6d2e7d05102c79e75e410804988b8a4b3b563aea/> simulator on Windows and test macOS in CI [ci-full]
   Now that we have a bunch of platform-specific bits for dynamic
   libraries, we should test all our supported dev platforms.
 - <csr-id-dad4abeb494a45650d9ae0f600e917bae2679b87/> don't emit duplicate leafs for building Rust service
   Because the `.build()` API creates a leaf itself, there is no need to
   wrap it in another leaf (which results in two rows in the progress
   display).

### New Features (BREAKING)

 - <csr-id-5d40134fbd4a82a2ee8b24b02b52abd506880435/> introduce support for dynamic external clients
   So far, Hydro has only supported deployments where there is a statically
   known number of single-connection external clients for sending /
   receiving data. This is not practical in production applications where
   external users will dynamically connect to the Hydro service.
   
   This takes the first step towards addressing this, by setting up the
   networking mechanism for handling dynamically connected clients and
   adding an API for sending/receiving data.
   
   This also changes the `source_external_bytes` APIs to return the raw
   `BytesMut` sent over the network, rather than adding a freezing step.
   
   Finally, we show an end-to-end example using this interface to write a
   simple HTTP server.

### Refactor (BREAKING)

 - <csr-id-9e07af1af8ad7c84a29d7a960bae7229bca67d68/> remove deprecated `add_connection` API
   Has not been used for a long while...

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 12 commits contributed to the release.
 - 12 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 12 unique issues were worked on: [#1966](https://github.com/hydro-project/hydro/issues/1966), [#1967](https://github.com/hydro-project/hydro/issues/1967), [#2024](https://github.com/hydro-project/hydro/issues/2024), [#2083](https://github.com/hydro-project/hydro/issues/2083), [#2177](https://github.com/hydro-project/hydro/issues/2177), [#2187](https://github.com/hydro-project/hydro/issues/2187), [#2191](https://github.com/hydro-project/hydro/issues/2191), [#2192](https://github.com/hydro-project/hydro/issues/2192), [#2194](https://github.com/hydro-project/hydro/issues/2194), [#2205](https://github.com/hydro-project/hydro/issues/2205), [#2269](https://github.com/hydro-project/hydro/issues/2269), [#2288](https://github.com/hydro-project/hydro/issues/2288)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1966](https://github.com/hydro-project/hydro/issues/1966)**
    - Introduce support for dynamic external clients ([`5d40134`](https://github.com/hydro-project/hydro/commit/5d40134fbd4a82a2ee8b24b02b52abd506880435))
 * **[#1967](https://github.com/hydro-project/hydro/issues/1967)**
    - Remove deprecated `add_connection` API ([`9e07af1`](https://github.com/hydro-project/hydro/commit/9e07af1af8ad7c84a29d7a960bae7229bca67d68))
 * **[#2024](https://github.com/hydro-project/hydro/issues/2024)**
    - Update pinned nightly to 2025-08-20, fix lints ([`97426b8`](https://github.com/hydro-project/hydro/commit/97426b8a7e3b3af8a58b4c44c768c3f48cd0ed71))
 * **[#2083](https://github.com/hydro-project/hydro/issues/2083)**
    - Add support for aws via terraform ([`1dd2606`](https://github.com/hydro-project/hydro/commit/1dd2606932fa30cf1812d75a5a3733f817f344b5))
 * **[#2177](https://github.com/hydro-project/hydro/issues/2177)**
    - Only depend on `samply` on supported systems, remove leftover DTrace code ([`83b77a1`](https://github.com/hydro-project/hydro/commit/83b77a16a0771a2efe86075ec98b60639a1510b7))
 * **[#2187](https://github.com/hydro-project/hydro/issues/2187)**
    - Share `__staged` across trybuild targets ([`a65893a`](https://github.com/hydro-project/hydro/commit/a65893a65bc43cb13c516c95a8d86ad424446565))
 * **[#2191](https://github.com/hydro-project/hydro/issues/2191)**
    - Don't emit duplicate leafs for building Rust service ([`dad4abe`](https://github.com/hydro-project/hydro/commit/dad4abeb494a45650d9ae0f600e917bae2679b87))
 * **[#2192](https://github.com/hydro-project/hydro/issues/2192)**
    - Use dynamically linked libraries in test mode ([`891a25d`](https://github.com/hydro-project/hydro/commit/891a25d3f72b2fa10a5154940624a146bb460ed7))
 * **[#2194](https://github.com/hydro-project/hydro/issues/2194)**
    - Simulator on Windows and test macOS in CI [ci-full] ([`6d2e7d0`](https://github.com/hydro-project/hydro/commit/6d2e7d05102c79e75e410804988b8a4b3b563aea))
 * **[#2205](https://github.com/hydro-project/hydro/issues/2205)**
    - Make `cargo sim` fuzzing command work again [ci-full] ([`807eb0a`](https://github.com/hydro-project/hydro/commit/807eb0afe3f5fcbec573959b5e4323f762cebe6e))
 * **[#2269](https://github.com/hydro-project/hydro/issues/2269)**
    - Make glibc vs musl configurable for cloud deployments ([`fc3bf5b`](https://github.com/hydro-project/hydro/commit/fc3bf5bafdd225bf6a59fbb2a01c0450e8404315))
 * **[#2288](https://github.com/hydro-project/hydro/issues/2288)**
    - Fix full path replacement for staged code errors ([`f0acd7c`](https://github.com/hydro-project/hydro/commit/f0acd7cb0e60d163c9e5a7c053ebdba995c8f289))
</details>

## 0.14.0 (2025-07-31)

<csr-id-555b83e4b07e4c1f5ce25ef1293cd715a30108fd/>
<csr-id-c983f1f25f89c4eba357b1b7f29d7a4f91b06544/>
<csr-id-b39aa41cf95bf994401a68db3e73ca59da67d729/>
<csr-id-5ab815f3567d51e9bd114f90af8e837fe0732cd8/>
<csr-id-de6c8ce3d258ecd1a2038e2a09d5ea8860e8ad42/>

### Documentation

 - <csr-id-fb7d45ddd13ed6944ba176099d7d0a4a8608f335/> add basic `hydro_deploy`, `tracing` docs, fix #1205
   Also removes extensions from links, for simplicity.

### New Features

 - <csr-id-0e6403cc21c89ae397828a84b4908204c9f4dba5/> improve logging for profiling
 - <csr-id-bd1afdff5fd7b8dc6d2c567cd2659542a84c6216/> allow running generated binaries with single-threaded Tokio runtime
   Before, we had a janky architecture for establishing network connections
   which relied on blocking on futures outside an async context, which
   required a multi-threaded runtime. Now, we establish all connections
   before launching the DFIR code, so that no blocking is required there.
 - <csr-id-99a8f1dfdde087578f25c19e502ad13e1d98a394/> Decoupling analysis
   A Gurobi license is required to run code that uses `hydro_optimize` (for ILP over decoupling decisions)
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
 - <csr-id-8705f97699badb29daa0f9c89d43a14d83c2400f/> Allow VM names to be customized to ease debugging
   Co-authored with @shadaj
 - <csr-id-6ba3b33f665ec47f3a13883e15d920d49d42f666/> update how progress is displayed, fix #1415

### Bug Fixes

 - <csr-id-3a3ce7ff459f15d1fbee47d1e374d99ea122ff3a/> VM names violating GCP's regex
 - <csr-id-bea805d1c7c93bfdaf0fa5bfcaafb8b3f405e07f/> use `--target-dir` instead of environment variable to improve caching
   sccache includes all environment variables starting with `CARGO_` in the
   cache key, so this would cause misses for all trybuild compilation.
   Along with https://github.com/mozilla/sccache/pull/2424, this improves
   compilation caching.
 - <csr-id-6699197d60a5efe8fd2fe951463649eca54ea9db/> correctly enable staged-trybuild mode when cross-compiling
   `RUSTFLAGS` are not passed to build scripts, use a regular environment
   variable instead. Should also dramatically improve cache hit rate for
   sccache since the rustflags for non-stageleft crates are untouched.

### Other

 - <csr-id-555b83e4b07e4c1f5ce25ef1293cd715a30108fd/> remove hydro_cli to fix build on AL2

### Refactor

 - <csr-id-c983f1f25f89c4eba357b1b7f29d7a4f91b06544/> Encapsulate stdout/stderr handling in new `PriorityBroadcast` type, fix #1357
 - <csr-id-b39aa41cf95bf994401a68db3e73ca59da67d729/> use `blake3` hash intead of random for build `unique_id`, fix #1337
 - <csr-id-5ab815f3567d51e9bd114f90af8e837fe0732cd8/> use `async-ssh2-russh` (instead of `libssh2` bindings), fix #1463

### New Features (BREAKING)

 - <csr-id-d6ae619060339eb3dac5bec17d384430e3588093/> re-add loop lifetimes for anti_join_multiset, tests, remove MonotonicMap, fix #1830, fix #1823
   Redo of #1835
   
   Also updates path of trybuild errors to allow them to be clicked in the
   IDE
   
   ---
   
   Previous commit:
   
   Also implements loop lifetimes for `difference_multiset` which uses the
   `anti_join_multiset` codegen.
   
   Updates tests for `difference`, `difference_multiset`, `anti_join`, and
   `anti_join_multiset`

### Refactor (BREAKING)

 - <csr-id-de6c8ce3d258ecd1a2038e2a09d5ea8860e8ad42/> use direct `&dyn Any` upcasting for Rust 1.86, update pyo3, fix #1821

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 18 commits contributed to the release.
 - 110 days passed between releases.
 - 16 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 16 unique issues were worked on: [#1803](https://github.com/hydro-project/hydro/issues/1803), [#1825](https://github.com/hydro-project/hydro/issues/1825), [#1844](https://github.com/hydro-project/hydro/issues/1844), [#1845](https://github.com/hydro-project/hydro/issues/1845), [#1849](https://github.com/hydro-project/hydro/issues/1849), [#1856](https://github.com/hydro-project/hydro/issues/1856), [#1859](https://github.com/hydro-project/hydro/issues/1859), [#1901](https://github.com/hydro-project/hydro/issues/1901), [#1907](https://github.com/hydro-project/hydro/issues/1907), [#1911](https://github.com/hydro-project/hydro/issues/1911), [#1918](https://github.com/hydro-project/hydro/issues/1918), [#1938](https://github.com/hydro-project/hydro/issues/1938), [#1941](https://github.com/hydro-project/hydro/issues/1941), [#1943](https://github.com/hydro-project/hydro/issues/1943), [#1955](https://github.com/hydro-project/hydro/issues/1955), [#1961](https://github.com/hydro-project/hydro/issues/1961)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1803](https://github.com/hydro-project/hydro/issues/1803)**
    - Use `async-ssh2-russh` (instead of `libssh2` bindings), fix #1463 ([`5ab815f`](https://github.com/hydro-project/hydro/commit/5ab815f3567d51e9bd114f90af8e837fe0732cd8))
 * **[#1825](https://github.com/hydro-project/hydro/issues/1825)**
    - Use direct `&dyn Any` upcasting for Rust 1.86, update pyo3, fix #1821 ([`de6c8ce`](https://github.com/hydro-project/hydro/commit/de6c8ce3d258ecd1a2038e2a09d5ea8860e8ad42))
 * **[#1844](https://github.com/hydro-project/hydro/issues/1844)**
    - Use `blake3` hash intead of random for build `unique_id`, fix #1337 ([`b39aa41`](https://github.com/hydro-project/hydro/commit/b39aa41cf95bf994401a68db3e73ca59da67d729))
 * **[#1845](https://github.com/hydro-project/hydro/issues/1845)**
    - Update how progress is displayed, fix #1415 ([`6ba3b33`](https://github.com/hydro-project/hydro/commit/6ba3b33f665ec47f3a13883e15d920d49d42f666))
 * **[#1849](https://github.com/hydro-project/hydro/issues/1849)**
    - Encapsulate stdout/stderr handling in new `PriorityBroadcast` type, fix #1357 ([`c983f1f`](https://github.com/hydro-project/hydro/commit/c983f1f25f89c4eba357b1b7f29d7a4f91b06544))
 * **[#1856](https://github.com/hydro-project/hydro/issues/1856)**
    - Add basic `hydro_deploy`, `tracing` docs, fix #1205 ([`fb7d45d`](https://github.com/hydro-project/hydro/commit/fb7d45ddd13ed6944ba176099d7d0a4a8608f335))
 * **[#1859](https://github.com/hydro-project/hydro/issues/1859)**
    - Decoupling analysis ([`99a8f1d`](https://github.com/hydro-project/hydro/commit/99a8f1dfdde087578f25c19e502ad13e1d98a394))
 * **[#1901](https://github.com/hydro-project/hydro/issues/1901)**
    - Allow VM names to be customized to ease debugging ([`8705f97`](https://github.com/hydro-project/hydro/commit/8705f97699badb29daa0f9c89d43a14d83c2400f))
 * **[#1907](https://github.com/hydro-project/hydro/issues/1907)**
    - Upgrade Stageleft to eliminate `__staged` compilation during development ([`b333b45`](https://github.com/hydro-project/hydro/commit/b333b45e0936bbe481d7fbc285790d942779c494))
 * **[#1911](https://github.com/hydro-project/hydro/issues/1911)**
    - Re-add loop lifetimes for anti_join_multiset, tests, remove MonotonicMap, fix #1830, fix #1823 ([`d6ae619`](https://github.com/hydro-project/hydro/commit/d6ae619060339eb3dac5bec17d384430e3588093))
 * **[#1918](https://github.com/hydro-project/hydro/issues/1918)**
    - Remove hydro_cli to fix build on AL2 ([`555b83e`](https://github.com/hydro-project/hydro/commit/555b83e4b07e4c1f5ce25ef1293cd715a30108fd))
 * **[#1938](https://github.com/hydro-project/hydro/issues/1938)**
    - Allow running generated binaries with single-threaded Tokio runtime ([`bd1afdf`](https://github.com/hydro-project/hydro/commit/bd1afdff5fd7b8dc6d2c567cd2659542a84c6216))
 * **[#1941](https://github.com/hydro-project/hydro/issues/1941)**
    - Correctly enable staged-trybuild mode when cross-compiling ([`6699197`](https://github.com/hydro-project/hydro/commit/6699197d60a5efe8fd2fe951463649eca54ea9db))
 * **[#1943](https://github.com/hydro-project/hydro/issues/1943)**
    - Use `--target-dir` instead of environment variable to improve caching ([`bea805d`](https://github.com/hydro-project/hydro/commit/bea805d1c7c93bfdaf0fa5bfcaafb8b3f405e07f))
 * **[#1955](https://github.com/hydro-project/hydro/issues/1955)**
    - Improve logging for profiling ([`0e6403c`](https://github.com/hydro-project/hydro/commit/0e6403cc21c89ae397828a84b4908204c9f4dba5))
 * **[#1961](https://github.com/hydro-project/hydro/issues/1961)**
    - VM names violating GCP's regex ([`3a3ce7f`](https://github.com/hydro-project/hydro/commit/3a3ce7ff459f15d1fbee47d1e374d99ea122ff3a))
 * **Uncategorized**
    - Release example_test v0.0.0, dfir_rs v0.14.0, hydro_deploy v0.14.0, hydro_lang v0.14.0, hydro_optimize v0.13.0, hydro_std v0.14.0 ([`5f69ee0`](https://github.com/hydro-project/hydro/commit/5f69ee080a9e257bc07cdc4deda90ce5525a3d0e))
    - Release dfir_lang v0.14.0, dfir_macro v0.14.0, hydro_deploy_integration v0.14.0, lattices_macro v0.5.10, variadics_macro v0.6.1, dfir_rs v0.14.0, hydro_deploy v0.14.0, hydro_lang v0.14.0, hydro_optimize v0.13.0, hydro_std v0.14.0, safety bump 6 crates ([`0683595`](https://github.com/hydro-project/hydro/commit/06835950c12884d661100c13f73ad23a98bfad9f))
</details>

## 0.13.0 (2025-04-11)

### New Features

 - <csr-id-6d24901550fa873fc8b4b474f9f6316d98cf7aa8/> implement profiling for macOS and Windows using samply

### Bug Fixes

 - <csr-id-fbb5fab72c5a64a07653c9b6389186ad079703ec/> handle `-1` addresses from samply, fix `_counter()` rollover
   This fixes samply profiling on my "ancient" 2019 x86-64 macbook pro
   15.3.2 (24D81)
   
   This pull request aims to fix the handling of –1 address values from
   samply by updating tracing filenames and refactoring related error and
   type handling. Key changes include:
   - Better error messages when `dtrace` or `samply` are not instaled.

### Bug Fixes (BREAKING)

 - <csr-id-52221ec4b68882a783e0d86e6e4ea80441b4a79b/> fix perf setup, remove GCP `startup_script`, use `TracingOptions::setup_command` instead

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 27 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 3 unique issues were worked on: [#1800](https://github.com/hydro-project/hydro/issues/1800), [#1812](https://github.com/hydro-project/hydro/issues/1812), [#1814](https://github.com/hydro-project/hydro/issues/1814)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1800](https://github.com/hydro-project/hydro/issues/1800)**
    - Fix perf setup, remove GCP `startup_script`, use `TracingOptions::setup_command` instead ([`52221ec`](https://github.com/hydro-project/hydro/commit/52221ec4b68882a783e0d86e6e4ea80441b4a79b))
 * **[#1812](https://github.com/hydro-project/hydro/issues/1812)**
    - Implement profiling for macOS and Windows using samply ([`6d24901`](https://github.com/hydro-project/hydro/commit/6d24901550fa873fc8b4b474f9f6316d98cf7aa8))
 * **[#1814](https://github.com/hydro-project/hydro/issues/1814)**
    - Handle `-1` addresses from samply, fix `_counter()` rollover ([`fbb5fab`](https://github.com/hydro-project/hydro/commit/fbb5fab72c5a64a07653c9b6389186ad079703ec))
 * **Uncategorized**
    - Release dfir_lang v0.13.0, dfir_datalog_core v0.13.0, dfir_datalog v0.13.0, dfir_macro v0.13.0, hydro_deploy_integration v0.13.0, dfir_rs v0.13.0, hydro_deploy v0.13.0, hydro_lang v0.13.0, hydro_std v0.13.0, hydro_cli v0.13.0, safety bump 8 crates ([`400fd8f`](https://github.com/hydro-project/hydro/commit/400fd8f2e8cada253f54980e7edce0631be70a82))
</details>

## 0.12.1 (2025-03-15)

<csr-id-260902b210378af5291ec71a574256d7a5bcb463/>
<csr-id-056ac62611319b7bd10a751d7e231423a1b8dc4e/>
<csr-id-7dd71d67da162d2e4f3043b271a52037a3c983c0/>

### Chore

 - <csr-id-260902b210378af5291ec71a574256d7a5bcb463/> set `hydro_deploy_integration` to release as `0.12.1`

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

 - <csr-id-892b29b0caaad32dbfc46f1d43d25df583a36721/> copy down remote perf raw data and shift Terraform logs to stderr

### Bug Fixes

 - <csr-id-530604ccce4e1825ea5a35caa696dec5e846fefb/> fix codegen non-determinism that triggers rebuilds
   Also makes the generated `Cargo.toml` fixed regardless of the "extra
   Hydro features" or "test mode", by shifting them into special features
   defined on the trybuild repo. Overall, this results in the only dynamic
   piece being the generated `src/bin` files, which means that deploying
   the same code multiple times does not result in any recompilation.

### Style

 - <csr-id-056ac62611319b7bd10a751d7e231423a1b8dc4e/> cleanup old clippy lints, remove deprecated `relalg` crate

### Refactor (BREAKING)

 - <csr-id-7dd71d67da162d2e4f3043b271a52037a3c983c0/> remove "hydroflow" for `hydro_deploy_integration`, `hydro_deploy::rust_crate`, fix #1712
   Opted to use `RustCrate` as the replacement prefix with the expectation
   that @shadaj may have a more coincise name in mind?

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 8 commits contributed to the release.
 - 7 days passed between releases.
 - 6 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 5 unique issues were worked on: [#1773](https://github.com/hydro-project/hydro/issues/1773), [#1777](https://github.com/hydro-project/hydro/issues/1777), [#1779](https://github.com/hydro-project/hydro/issues/1779), [#1785](https://github.com/hydro-project/hydro/issues/1785), [#1787](https://github.com/hydro-project/hydro/issues/1787)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1773](https://github.com/hydro-project/hydro/issues/1773)**
    - Remove "hydroflow" for `hydro_deploy_integration`, `hydro_deploy::rust_crate`, fix #1712 ([`7dd71d6`](https://github.com/hydro-project/hydro/commit/7dd71d67da162d2e4f3043b271a52037a3c983c0))
 * **[#1777](https://github.com/hydro-project/hydro/issues/1777)**
    - Copy down remote perf raw data and shift Terraform logs to stderr ([`892b29b`](https://github.com/hydro-project/hydro/commit/892b29b0caaad32dbfc46f1d43d25df583a36721))
 * **[#1779](https://github.com/hydro-project/hydro/issues/1779)**
    - Fix codegen non-determinism that triggers rebuilds ([`530604c`](https://github.com/hydro-project/hydro/commit/530604ccce4e1825ea5a35caa696dec5e846fefb))
 * **[#1785](https://github.com/hydro-project/hydro/issues/1785)**
    - Cleanup old clippy lints, remove deprecated `relalg` crate ([`056ac62`](https://github.com/hydro-project/hydro/commit/056ac62611319b7bd10a751d7e231423a1b8dc4e))
 * **[#1787](https://github.com/hydro-project/hydro/issues/1787)**
    - Demote python deploy docs, fix docsrs configs, fix #1392, fix #1629 ([`b235a42`](https://github.com/hydro-project/hydro/commit/b235a42a3071e55da7b09bdc8bc710b18e0fe053))
 * **Uncategorized**
    - Release include_mdtests v0.0.0, dfir_rs v0.12.1, hydro_deploy v0.12.1, hydro_lang v0.12.1, hydro_std v0.12.1, hydro_cli v0.12.1 ([`faf0d3e`](https://github.com/hydro-project/hydro/commit/faf0d3ed9f172275f2e2f219c5ead1910c209a36))
    - Release dfir_lang v0.12.1, dfir_datalog_core v0.12.1, dfir_datalog v0.12.1, dfir_macro v0.12.1, hydro_deploy_integration v0.12.1, lattices v0.6.1, pusherator v0.0.12, dfir_rs v0.12.1, hydro_deploy v0.12.1, hydro_lang v0.12.1, hydro_std v0.12.1, hydro_cli v0.12.1 ([`23221b5`](https://github.com/hydro-project/hydro/commit/23221b53b30918707ddaa85529d04cd7919166b4))
    - Set `hydro_deploy_integration` to release as `0.12.1` ([`260902b`](https://github.com/hydro-project/hydro/commit/260902b210378af5291ec71a574256d7a5bcb463))
</details>

## 0.12.0 (2025-03-08)

<csr-id-49a387d4a21f0763df8ec94de73fb953c9cd333a/>
<csr-id-473ee4ab8c3d77356b5d3199f1612e6f710eac51/>
<csr-id-6ac0c53fa02853be5e17998f19a36d1a30641201/>
<csr-id-a1572f4f6e665041012769f518be43e404383081/>
<csr-id-c293cca6855695107e9cef5c5df99fb04a571934/>
<csr-id-8b3b60812d9f561cb7f59120993fbf2e23191e2b/>
<csr-id-44fb2806cf2d165d86695910f4755e0944c11832/>
<csr-id-3966d9063dae52e65b077321e0bd1150f2b0c3f1/>
<csr-id-3f76e91766a0bd9e61f11f9013d76f688467fb5e/>
<csr-id-81a1d3afc3bdfbfd4daea0f46025c020edc8625b/>
<csr-id-2681b9b8bb65b67146f3f5b33810045657186425/>
<csr-id-5cd0a9625822620dcc99b99356edfecbf0549497/>
<csr-id-ec3795a678d261a38085405b6e9bfea943dafefb/>
<csr-id-9ce31f65a5d400f8116ab536dc7a8cca848a4a93/>

### Chore

 - <csr-id-49a387d4a21f0763df8ec94de73fb953c9cd333a/> upgrade to Rust 2024 edition
   - Updates `Cargo.toml` to use new shared workspace keys
   - Updates lint settings (in workspace `Cargo.toml`)
   - `rustfmt` has changed slightly, resulting in a big diff - there are no
   actual code changes
   - Adds a script to `rustfmt` the template src files
 - <csr-id-473ee4ab8c3d77356b5d3199f1612e6f710eac51/> update buildstructor

### Chore (BREAKING)

 - <csr-id-3966d9063dae52e65b077321e0bd1150f2b0c3f1/> use DFIR name instead of Hydroflow in some places, fix #1644
   Fix partially #1712
   
   * Renames `WriteContextArgs.hydroflow` to `WriteContextArgs.df_ident`
   for DFIR operator codegen
   * Removes some dead code/files

### Style

 - <csr-id-3f76e91766a0bd9e61f11f9013d76f688467fb5e/> fix all unexpected cfgs
   Testing in https://github.com/MingweiSamuel/hydroflow

### Refactor

 - <csr-id-81a1d3afc3bdfbfd4daea0f46025c020edc8625b/> show a lot more error info on build failure
   From debugging today
   
   Main useful one is the STDERR output
   
   Uses more memory
 - <csr-id-2681b9b8bb65b67146f3f5b33810045657186425/> fix lifetime issue with command forming for Rust 2024
 - <csr-id-5cd0a9625822620dcc99b99356edfecbf0549497/> enable lints, cleanups for Rust 2024 #1732

### Chore

 - <csr-id-ec3795a678d261a38085405b6e9bfea943dafefb/> upgrade to Rust 2024 edition
   - Updates `Cargo.toml` to use new shared workspace keys
   - Updates lint settings (in workspace `Cargo.toml`)
   - `rustfmt` has changed slightly, resulting in a big diff - there are no
   actual code changes
   - Adds a script to `rustfmt` the template src files
 - <csr-id-9ce31f65a5d400f8116ab536dc7a8cca848a4a93/> update buildstructor

### New Features

 - <csr-id-733494ea2655cdc1460da7f902d999f0e3797411/> Link DFIR operators to Hydro operators in perf
 - <csr-id-a4adb08700fdc5fdbc949fc656e6cb309e7159a5/> provide in-memory access to perf tracing results
 - <csr-id-1d48fde45a741e5eec59ce3b27a4a8f195198428/> Link DFIR operators to Hydro operators in perf
 - <csr-id-5ba6236555113dc019fe61adaf1d5aa34e07bb58/> provide in-memory access to perf tracing results

### Bug Fixes

 - <csr-id-02858077604330299de18b10bb261e6d25bde6cd/> always write logs to stdout
 - <csr-id-070b6e0a300bfb4ccb47d231a008bd7ce2c93a7a/> improve error message when crates fail to build
 - <csr-id-75eb323a612fd5d2609e464fe7690bc2b6a8457a/> use correct `__staged` path when rewriting `crate::` imports
   Previously, a rewrite would first turn `crate` into `crate::__staged`,
   and another would rewrite `crate::__staged` into `hydro_test::__staged`.
   The latter global rewrite is unnecessary because the stageleft logic
   already will use the full crate name when handling public types, so we
   drop it.
 - <csr-id-cf8e59a651f4dadff3afd10fbb394621622109a9/> always write logs to stdout
 - <csr-id-f8000c503de2236552fa430ed859e15ce594d3ec/> improve error message when crates fail to build
 - <csr-id-48b275c1247f4f6fe7e6b63a5ae184c5d85b6fa1/> use correct `__staged` path when rewriting `crate::` imports
   Previously, a rewrite would first turn `crate` into `crate::__staged`,
   and another would rewrite `crate::__staged` into `hydro_test::__staged`.
   The latter global rewrite is unnecessary because the stageleft logic
   already will use the full crate name when handling public types, so we
   drop it.

### Refactor

 - <csr-id-6ac0c53fa02853be5e17998f19a36d1a30641201/> show a lot more error info on build failure
   From debugging today
   
   Main useful one is the STDERR output
   
   Uses more memory
 - <csr-id-a1572f4f6e665041012769f518be43e404383081/> fix lifetime issue with command forming for Rust 2024
 - <csr-id-c293cca6855695107e9cef5c5df99fb04a571934/> enable lints, cleanups for Rust 2024 #1732

### Style

 - <csr-id-8b3b60812d9f561cb7f59120993fbf2e23191e2b/> fix all unexpected cfgs
   Testing in https://github.com/MingweiSamuel/hydroflow

### Chore (BREAKING)

 - <csr-id-44fb2806cf2d165d86695910f4755e0944c11832/> use DFIR name instead of Hydroflow in some places, fix #1644
   Fix partially #1712
   
   * Renames `WriteContextArgs.hydroflow` to `WriteContextArgs.df_ident`
   for DFIR operator codegen
   * Removes some dead code/files

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 13 commits contributed to the release.
 - 74 days passed between releases.
 - 12 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 12 unique issues were worked on: [#1648](https://github.com/hydro-project/hydro/issues/1648), [#1657](https://github.com/hydro-project/hydro/issues/1657), [#1691](https://github.com/hydro-project/hydro/issues/1691), [#1700](https://github.com/hydro-project/hydro/issues/1700), [#1713](https://github.com/hydro-project/hydro/issues/1713), [#1719](https://github.com/hydro-project/hydro/issues/1719), [#1720](https://github.com/hydro-project/hydro/issues/1720), [#1723](https://github.com/hydro-project/hydro/issues/1723), [#1737](https://github.com/hydro-project/hydro/issues/1737), [#1744](https://github.com/hydro-project/hydro/issues/1744), [#1747](https://github.com/hydro-project/hydro/issues/1747), [#1752](https://github.com/hydro-project/hydro/issues/1752)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1648](https://github.com/hydro-project/hydro/issues/1648)**
    - Fix all unexpected cfgs ([`3f76e91`](https://github.com/hydro-project/hydro/commit/3f76e91766a0bd9e61f11f9013d76f688467fb5e))
 * **[#1657](https://github.com/hydro-project/hydro/issues/1657)**
    - Use correct `__staged` path when rewriting `crate::` imports ([`48b275c`](https://github.com/hydro-project/hydro/commit/48b275c1247f4f6fe7e6b63a5ae184c5d85b6fa1))
 * **[#1691](https://github.com/hydro-project/hydro/issues/1691)**
    - Improve error message when crates fail to build ([`f8000c5`](https://github.com/hydro-project/hydro/commit/f8000c503de2236552fa430ed859e15ce594d3ec))
 * **[#1700](https://github.com/hydro-project/hydro/issues/1700)**
    - Always write logs to stdout ([`cf8e59a`](https://github.com/hydro-project/hydro/commit/cf8e59a651f4dadff3afd10fbb394621622109a9))
 * **[#1713](https://github.com/hydro-project/hydro/issues/1713)**
    - Use DFIR name instead of Hydroflow in some places, fix #1644 ([`3966d90`](https://github.com/hydro-project/hydro/commit/3966d9063dae52e65b077321e0bd1150f2b0c3f1))
 * **[#1719](https://github.com/hydro-project/hydro/issues/1719)**
    - Provide in-memory access to perf tracing results ([`5ba6236`](https://github.com/hydro-project/hydro/commit/5ba6236555113dc019fe61adaf1d5aa34e07bb58))
 * **[#1720](https://github.com/hydro-project/hydro/issues/1720)**
    - Update buildstructor ([`9ce31f6`](https://github.com/hydro-project/hydro/commit/9ce31f65a5d400f8116ab536dc7a8cca848a4a93))
 * **[#1723](https://github.com/hydro-project/hydro/issues/1723)**
    - Link DFIR operators to Hydro operators in perf ([`1d48fde`](https://github.com/hydro-project/hydro/commit/1d48fde45a741e5eec59ce3b27a4a8f195198428))
 * **[#1737](https://github.com/hydro-project/hydro/issues/1737)**
    - Enable lints, cleanups for Rust 2024 #1732 ([`5cd0a96`](https://github.com/hydro-project/hydro/commit/5cd0a9625822620dcc99b99356edfecbf0549497))
 * **[#1744](https://github.com/hydro-project/hydro/issues/1744)**
    - Fix lifetime issue with command forming for Rust 2024 ([`2681b9b`](https://github.com/hydro-project/hydro/commit/2681b9b8bb65b67146f3f5b33810045657186425))
 * **[#1747](https://github.com/hydro-project/hydro/issues/1747)**
    - Upgrade to Rust 2024 edition ([`ec3795a`](https://github.com/hydro-project/hydro/commit/ec3795a678d261a38085405b6e9bfea943dafefb))
 * **[#1752](https://github.com/hydro-project/hydro/issues/1752)**
    - Show a lot more error info on build failure ([`81a1d3a`](https://github.com/hydro-project/hydro/commit/81a1d3afc3bdfbfd4daea0f46025c020edc8625b))
 * **Uncategorized**
    - Release dfir_lang v0.12.0, dfir_datalog_core v0.12.0, dfir_datalog v0.12.0, dfir_macro v0.12.0, hydroflow_deploy_integration v0.12.0, lattices_macro v0.5.9, variadics v0.0.9, variadics_macro v0.6.0, lattices v0.6.0, multiplatform_test v0.5.0, pusherator v0.0.11, dfir_rs v0.12.0, hydro_deploy v0.12.0, stageleft_macro v0.6.0, stageleft v0.7.0, stageleft_tool v0.6.0, hydro_lang v0.12.0, hydro_std v0.12.0, hydro_cli v0.12.0, safety bump 10 crates ([`973c925`](https://github.com/hydro-project/hydro/commit/973c925e87ed78344494581bd7ce1bbb4186a2f3))
</details>

## 0.11.0 (2024-12-23)

### Documentation

 - <csr-id-28cd220c68e3660d9ebade113949a2346720cd04/> add `repository` field to `Cargo.toml`s, fix #1452
   #1452 
   
   Will trigger new releases of the following:
   `unchanged = 'hydroflow_deploy_integration', 'variadics',
   'variadics_macro', 'pusherator'`
   
   (All other crates already have changes, so would be released anyway)
 - <csr-id-e1a08e5d165fbc80da2ae695e507078a97a9031f/> update `CHANGELOG.md`s for big rename
   Generated before rename per `RELEASING.md` instructions.
 - <csr-id-204bd117ca3a8845b4986539efb91a0c612dfa05/> add `repository` field to `Cargo.toml`s, fix #1452
   #1452 
   
   Will trigger new releases of the following:
   `unchanged = 'hydroflow_deploy_integration', 'variadics',
   'variadics_macro', 'pusherator'`
   
   (All other crates already have changes, so would be released anyway)
 - <csr-id-27c40e2ca5a822f6ebd31c7f01213aa6d407418a/> update `CHANGELOG.md`s for big rename
   Generated before rename per `RELEASING.md` instructions.

### New Features

 - <csr-id-8d550b94ae2c08486e1c2222d37e3ca8b5f018b7/> use regular println when no tasks are active
   Significantly improves the appearance of Hydroflow+ logs when the
   terminal causes wrapping.
 - <csr-id-8e026595dce1e8d00ab61dad33c4d6046cfed7cb/> use regular println when no tasks are active
   Significantly improves the appearance of Hydroflow+ logs when the
   terminal causes wrapping.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 45 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#1501](https://github.com/hydro-project/hydro/issues/1501), [#1577](https://github.com/hydro-project/hydro/issues/1577)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1501](https://github.com/hydro-project/hydro/issues/1501)**
    - Add `repository` field to `Cargo.toml`s, fix #1452 ([`204bd11`](https://github.com/hydro-project/hydro/commit/204bd117ca3a8845b4986539efb91a0c612dfa05))
 * **[#1577](https://github.com/hydro-project/hydro/issues/1577)**
    - Use regular println when no tasks are active ([`8e02659`](https://github.com/hydro-project/hydro/commit/8e026595dce1e8d00ab61dad33c4d6046cfed7cb))
 * **Uncategorized**
    - Release dfir_lang v0.11.0, dfir_datalog_core v0.11.0, dfir_datalog v0.11.0, dfir_macro v0.11.0, hydroflow_deploy_integration v0.11.0, lattices_macro v0.5.8, variadics v0.0.8, variadics_macro v0.5.6, lattices v0.5.9, multiplatform_test v0.4.0, pusherator v0.0.10, dfir_rs v0.11.0, hydro_deploy v0.11.0, stageleft_macro v0.5.0, stageleft v0.6.0, stageleft_tool v0.5.0, hydro_lang v0.11.0, hydro_std v0.11.0, hydro_cli v0.11.0, safety bump 6 crates ([`361b443`](https://github.com/hydro-project/hydro/commit/361b4439ef9c781860f18d511668ab463a8c5203))
    - Update `CHANGELOG.md`s for big rename ([`27c40e2`](https://github.com/hydro-project/hydro/commit/27c40e2ca5a822f6ebd31c7f01213aa6d407418a))
</details>

## v0.10.0 (2024-11-08)

<csr-id-d5677604e93c07a5392f4229af94a0b736eca382/>
<csr-id-8442d1b524621a9f8b43372a9c25991efb33c25e/>
<csr-id-159c2dc39d41cb82ecd2f562c3c27a3c64dc4bfc/>
<csr-id-014ebb2628b5b80ea1b6426b58c4d62706edb9ef/>

### Chore

 - <csr-id-d5677604e93c07a5392f4229af94a0b736eca382/> update pinned rust version, clippy lints, remove some dead code

### Style

 - <csr-id-159c2dc39d41cb82ecd2f562c3c27a3c64dc4bfc/> fixes for latest nightly clippy

### Chore

 - <csr-id-014ebb2628b5b80ea1b6426b58c4d62706edb9ef/> update pinned rust version, clippy lints, remove some dead code

### New Features

 - <csr-id-afe78c343658472513b34d28658634b253148aee/> add ability to have staged flows inside unit tests
   Whenever a Hydroflow+ program is compiled, it depends on a generated
   `__staged` module, which contains the entire contents of the crate but
   with every type / function made `pub` and exported, so that the compiled
   UDFs can resolve local references appropriately.
   
   Previously, we would not do this for `#[cfg(test)]` modules, since they
   may use `dev-dependencies` and therefore the generated module may fail
   to compile when not in test mode. To solve this, when running a unit
   test (marked with `hydroflow_plus::deploy::init_test()`) that uses
   trybuild, we emit a version of the `__staged` module with `#[cfg(test)]`
   modules included _into the generated trybuild sources_ because we can
   guarantee via trybuild that the appropriate `dev-dependencies` are
   available.
   
   This by itself allows crates depending on `hydroflow_plus` to have local
   unit tests with Hydroflow+ logic inside them. But we also want to use
   this support for unit tests inside `hydroflow_plus` itself. To enable
   that, we eliminate the `hydroflow_plus_deploy` crate and move its
   contents directly to `hydroflow_plus` itself so that we can access the
   trybuild machinery without incurring a circular dependency.
   
   Also fixes #1408
 - <csr-id-8a809315cd37929687fcabc34a12042db25d5767/> add API for external network inputs
   This is a key step towards being able to unit-test HF+ graphs, by being
   able to have controlled inputs. Outputs next.
 - <csr-id-5b74749a0d7033d332b0c435f5cc4cf3f5cbd337/> add ability to have staged flows inside unit tests
   Whenever a Hydroflow+ program is compiled, it depends on a generated
   `__staged` module, which contains the entire contents of the crate but
   with every type / function made `pub` and exported, so that the compiled
   UDFs can resolve local references appropriately.
   
   Previously, we would not do this for `#[cfg(test)]` modules, since they
   may use `dev-dependencies` and therefore the generated module may fail
   to compile when not in test mode. To solve this, when running a unit
   test (marked with `hydroflow_plus::deploy::init_test()`) that uses
   trybuild, we emit a version of the `__staged` module with `#[cfg(test)]`
   modules included _into the generated trybuild sources_ because we can
   guarantee via trybuild that the appropriate `dev-dependencies` are
   available.
   
   This by itself allows crates depending on `hydroflow_plus` to have local
   unit tests with Hydroflow+ logic inside them. But we also want to use
   this support for unit tests inside `hydroflow_plus` itself. To enable
   that, we eliminate the `hydroflow_plus_deploy` crate and move its
   contents directly to `hydroflow_plus` itself so that we can access the
   trybuild machinery without incurring a circular dependency.
   
   Also fixes #1408
 - <csr-id-89c3401c70805169769b4e981c5c5491afcea57b/> add API for external network inputs
   This is a key step towards being able to unit-test HF+ graphs, by being
   able to have controlled inputs. Outputs next.

### Style

 - <csr-id-8442d1b524621a9f8b43372a9c25991efb33c25e/> fixes for latest nightly clippy

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release.
 - 69 days passed between releases.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 4 unique issues were worked on: [#1444](https://github.com/hydro-project/hydro/issues/1444), [#1449](https://github.com/hydro-project/hydro/issues/1449), [#1450](https://github.com/hydro-project/hydro/issues/1450), [#1537](https://github.com/hydro-project/hydro/issues/1537)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1444](https://github.com/hydro-project/hydro/issues/1444)**
    - Update pinned rust version, clippy lints, remove some dead code ([`014ebb2`](https://github.com/hydro-project/hydro/commit/014ebb2628b5b80ea1b6426b58c4d62706edb9ef))
 * **[#1449](https://github.com/hydro-project/hydro/issues/1449)**
    - Add API for external network inputs ([`89c3401`](https://github.com/hydro-project/hydro/commit/89c3401c70805169769b4e981c5c5491afcea57b))
 * **[#1450](https://github.com/hydro-project/hydro/issues/1450)**
    - Add ability to have staged flows inside unit tests ([`5b74749`](https://github.com/hydro-project/hydro/commit/5b74749a0d7033d332b0c435f5cc4cf3f5cbd337))
 * **[#1537](https://github.com/hydro-project/hydro/issues/1537)**
    - Fixes for latest nightly clippy ([`159c2dc`](https://github.com/hydro-project/hydro/commit/159c2dc39d41cb82ecd2f562c3c27a3c64dc4bfc))
 * **Uncategorized**
    - Release hydroflow_lang v0.10.0, hydroflow_datalog_core v0.10.0, hydroflow_datalog v0.10.0, hydroflow_deploy_integration v0.10.0, hydroflow_macro v0.10.0, lattices_macro v0.5.7, variadics v0.0.7, variadics_macro v0.5.5, lattices v0.5.8, multiplatform_test v0.3.0, pusherator v0.0.9, hydroflow v0.10.0, hydro_deploy v0.10.0, stageleft_macro v0.4.0, stageleft v0.5.0, stageleft_tool v0.4.0, hydroflow_plus v0.10.0, hydro_cli v0.10.0, safety bump 8 crates ([`258f480`](https://github.com/hydro-project/hydro/commit/258f4805dbcca36750cbfaaf36db00d3a007d817))
</details>

## v0.9.0 (2024-08-30)

<csr-id-a2ec110ccadb97e293b19d83a155d98d94224bba/>
<csr-id-11af32828bab6e4a4264d2635ff71a12bb0bb778/>
<csr-id-a88a550cefde3a56790859127edc6a4e27e07090/>
<csr-id-77246e77df47a0006dcb3eaeeb76882efacfd25c/>
<csr-id-3fde68d0db0414017cfb771a218b14b8f57d1686/>
<csr-id-0a465e55dd39c76bc1aefb020460a639d792fe87/>
<csr-id-bb081d3b0af6dbce9630e23dfe8b7d1363751c2b/>
<csr-id-a2147864b24110c9ae2c1553e9e8b55bd5065f15/>
<csr-id-8856c8596d5ad9d5f24a46467690bfac1549fae2/>
<csr-id-25989c7d938a0e93355a670f8d78a5aea900fce0/>
<csr-id-ac8dcbf7c6dbe018907a3012b71b0e4fcf4d2cb6/>
<csr-id-9a503cf85225ff1fcfe7a815fda3a4ac34a75c42/>
<csr-id-8bcd86c15bc4d9d2e3b564061be879bfe8820e25/>
<csr-id-5545c8b3329902b6b2418476d00191228f5f3e8d/>
<csr-id-36300cfe3879e5fed04a8f0806762626612ca9f7/>
<csr-id-a5d649b5a5cc54c7bc56011db33d509a5cb370a2/>
<csr-id-3508f5aeda3e18a6857df4ceb77e5e1015c02a17/>
<csr-id-2c04f51f1ec44f7898307b6610371dcb490ea686/>

### Chore

 - <csr-id-a2ec110ccadb97e293b19d83a155d98d94224bba/> manually set versions for crates renamed in #1413
 - <csr-id-11af32828bab6e4a4264d2635ff71a12bb0bb778/> lower min dependency versions where possible, update `Cargo.lock`
   Moved from #1418
   
   ---------

### Refactor (BREAKING)

 - <csr-id-10bd978793ccde8fc287aedd77729c0c6e5f1784/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

### Refactor (BREAKING)

 - <csr-id-10bd978793ccde8fc287aedd77729c0c6e5f1784/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

<csr-id-10bd978793ccde8fc287aedd77729c0c6e5f1784/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

### Refactor (BREAKING)

 - <csr-id-10bd978793ccde8fc287aedd77729c0c6e5f1784/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

<csr-id-10bd978793ccde8fc287aedd77729c0c6e5f1784/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

<csr-id-10bd978793ccde8fc287aedd77729c0c6e5f1784/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

### Refactor (BREAKING)

 - <csr-id-10bd978793ccde8fc287aedd77729c0c6e5f1784/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

<csr-id-10bd978793ccde8fc287aedd77729c0c6e5f1784/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

<csr-id-10bd978793ccde8fc287aedd77729c0c6e5f1784/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

<csr-id-10bd978793ccde8fc287aedd77729c0c6e5f1784/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

### Refactor (BREAKING)

 - <csr-id-10bd978793ccde8fc287aedd77729c0c6e5f1784/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

<csr-id-10bd978793ccde8fc287aedd77729c0c6e5f1784/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

<csr-id-10bd978793ccde8fc287aedd77729c0c6e5f1784/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

<csr-id-10bd978793ccde8fc287aedd77729c0c6e5f1784/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

<csr-id-10bd978793ccde8fc287aedd77729c0c6e5f1784/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

### Refactor (BREAKING)

 - <csr-id-25989c7d938a0e93355a670f8d78a5aea900fce0/> rename integration crates to drop CLI references
 - <csr-id-10bd978793ccde8fc287aedd77729c0c6e5f1784/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-ac8dcbf7c6dbe018907a3012b71b0e4fcf4d2cb6/> end-to-end flamegraph generation, fix #1365
   Depends on #1370
 - <csr-id-9a503cf85225ff1fcfe7a815fda3a4ac34a75c42/> `Deployment.stop()` for graceful shutdown including updated `perf` profile downloading
   * `perf` profile downloading moved from the `drop()` impl to `async fn
   stop()`
   * download perf data via stdout
   * update async-ssh2-lite to 0.5 to cleanup tokio compat issues
   
   WIP for #1365
 - <csr-id-8bcd86c15bc4d9d2e3b564061be879bfe8820e25/> use `buildstructor` to handle excessive `Deployment` method arguments, fix #1364
   Adds new method `Deployment::AzureHost`

<csr-id-10bd978793ccde8fc287aedd77729c0c6e5f1784/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-ac8dcbf7c6dbe018907a3012b71b0e4fcf4d2cb6/> end-to-end flamegraph generation, fix #1365
   Depends on #1370
 - <csr-id-9a503cf85225ff1fcfe7a815fda3a4ac34a75c42/> `Deployment.stop()` for graceful shutdown including updated `perf` profile downloading
   * `perf` profile downloading moved from the `drop()` impl to `async fn
   stop()`
   * download perf data via stdout
   * update async-ssh2-lite to 0.5 to cleanup tokio compat issues
   
   WIP for #1365
 - <csr-id-8bcd86c15bc4d9d2e3b564061be879bfe8820e25/> use `buildstructor` to handle excessive `Deployment` method arguments, fix #1364
   Adds new method `Deployment::AzureHost`

<csr-id-10bd978793ccde8fc287aedd77729c0c6e5f1784/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-ac8dcbf7c6dbe018907a3012b71b0e4fcf4d2cb6/> end-to-end flamegraph generation, fix #1365
   Depends on #1370
 - <csr-id-9a503cf85225ff1fcfe7a815fda3a4ac34a75c42/> `Deployment.stop()` for graceful shutdown including updated `perf` profile downloading
   * `perf` profile downloading moved from the `drop()` impl to `async fn
   stop()`
   * download perf data via stdout
   * update async-ssh2-lite to 0.5 to cleanup tokio compat issues
   
   WIP for #1365
 - <csr-id-8bcd86c15bc4d9d2e3b564061be879bfe8820e25/> use `buildstructor` to handle excessive `Deployment` method arguments, fix #1364
   Adds new method `Deployment::AzureHost`

<csr-id-10bd978793ccde8fc287aedd77729c0c6e5f1784/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-ac8dcbf7c6dbe018907a3012b71b0e4fcf4d2cb6/> end-to-end flamegraph generation, fix #1365
   Depends on #1370
 - <csr-id-9a503cf85225ff1fcfe7a815fda3a4ac34a75c42/> `Deployment.stop()` for graceful shutdown including updated `perf` profile downloading
   * `perf` profile downloading moved from the `drop()` impl to `async fn
   stop()`
   * download perf data via stdout
   * update async-ssh2-lite to 0.5 to cleanup tokio compat issues
   
   WIP for #1365
 - <csr-id-8bcd86c15bc4d9d2e3b564061be879bfe8820e25/> use `buildstructor` to handle excessive `Deployment` method arguments, fix #1364
   Adds new method `Deployment::AzureHost`

<csr-id-10bd978793ccde8fc287aedd77729c0c6e5f1784/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-ac8dcbf7c6dbe018907a3012b71b0e4fcf4d2cb6/> end-to-end flamegraph generation, fix #1365
   Depends on #1370
 - <csr-id-9a503cf85225ff1fcfe7a815fda3a4ac34a75c42/> `Deployment.stop()` for graceful shutdown including updated `perf` profile downloading
   * `perf` profile downloading moved from the `drop()` impl to `async fn
   stop()`
   * download perf data via stdout
   * update async-ssh2-lite to 0.5 to cleanup tokio compat issues
   
   WIP for #1365
 - <csr-id-8bcd86c15bc4d9d2e3b564061be879bfe8820e25/> use `buildstructor` to handle excessive `Deployment` method arguments, fix #1364
   Adds new method `Deployment::AzureHost`

<csr-id-10bd978793ccde8fc287aedd77729c0c6e5f1784/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-ac8dcbf7c6dbe018907a3012b71b0e4fcf4d2cb6/> end-to-end flamegraph generation, fix #1365
   Depends on #1370
 - <csr-id-9a503cf85225ff1fcfe7a815fda3a4ac34a75c42/> `Deployment.stop()` for graceful shutdown including updated `perf` profile downloading
   * `perf` profile downloading moved from the `drop()` impl to `async fn
   stop()`
   * download perf data via stdout
   * update async-ssh2-lite to 0.5 to cleanup tokio compat issues
   
   WIP for #1365
 - <csr-id-8bcd86c15bc4d9d2e3b564061be879bfe8820e25/> use `buildstructor` to handle excessive `Deployment` method arguments, fix #1364
   Adds new method `Deployment::AzureHost`

### Style

 - <csr-id-5545c8b3329902b6b2418476d00191228f5f3e8d/> use `name_of!` macro

### Refactor

 - <csr-id-36300cfe3879e5fed04a8f0806762626612ca9f7/> adjust `ProgressTracker::println`
   A small refactor pulled out of the perf tracing work, barely related to
   #1359
 - <csr-id-a5d649b5a5cc54c7bc56011db33d509a5cb370a2/> cleanup handling of arc `Weak` in `deployment.rs`

### Chore

 - <csr-id-3508f5aeda3e18a6857df4ceb77e5e1015c02a17/> manually set versions for crates renamed in #1413
 - <csr-id-2c04f51f1ec44f7898307b6610371dcb490ea686/> lower min dependency versions where possible, update `Cargo.lock`
   Moved from #1418
   
   ---------

### Refactor (BREAKING)

 - <csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

### Refactor (BREAKING)

 - <csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

### Refactor (BREAKING)

 - <csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

### Refactor (BREAKING)

 - <csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394

### Documentation

 - <csr-id-f5f1eb0c612f5c0c1752360d972ef6853c5e12f0/> cleanup doc comments for clippy latest
 - <csr-id-1766c8b0aa23df83ad242b581184b37e85afe27b/> cleanup doc comments for clippy latest

### New Features

 - <csr-id-6568263e03899d4e96837690e6e59284c194d7ff/> Add end-to-end flamegraph generation for macos and linux localhost, fix #1351
 - <csr-id-fedd3ef60fe16ab862244d816f7973269a7295e8/> improve progress UX by collapsing nested groups
   Now, when a group only has a single active task, we skip printing a line
   for the group itself and instead collapse its information into the line
   for the inner task (recursively as necessary). This allows us to show
   more fine grained progress without overflowing the console.
 - <csr-id-46a8a2cb08732bb21096e824bc4542d208c68fb2/> use trybuild to compile subgraph binaries
 - <csr-id-3cd309b3d0ef2661a096c0bdc38e271f9f9ce326/> Add end-to-end flamegraph generation for macos and linux localhost, fix #1351
 - <csr-id-d68f280ed944e001a7b3ca6954beeef2e4d984bb/> improve progress UX by collapsing nested groups
   Now, when a group only has a single active task, we skip printing a line
   for the group itself and instead collapse its information into the line
   for the inner task (recursively as necessary). This allows us to show
   more fine grained progress without overflowing the console.
 - <csr-id-2a49c13f2f4e3b47d79c34167015d6ba98a89888/> use trybuild to compile subgraph binaries

### Bug Fixes

 - <csr-id-c4683caca43f2927694c920b43ef35a6d1629eaa/> only record usermode events in perf
   When kernel stacks are included, the DWARF traces can become corrupted /
   overflown leading to flamegraphs with broken parents. We only are
   interested in usermode, anyways, and can measure I/O overhead through
   other methods.
 - <csr-id-63b528feeb2e6dac2ed12c02b2e39e0d42133a74/> only instantiate `Localhost` once
 - <csr-id-654b77d8f65ae6eb62c164a2d736168ff96cb168/> avoid Terraform crashing on empty provider block
 - <csr-id-cd0417229f3c268362013265f514d703d4af2c3d/> only record usermode events in perf
   When kernel stacks are included, the DWARF traces can become corrupted /
   overflown leading to flamegraphs with broken parents. We only are
   interested in usermode, anyways, and can measure I/O overhead through
   other methods.
 - <csr-id-628066bf8250b541493c8cf5efd6c7bf01900640/> only instantiate `Localhost` once
 - <csr-id-fd72bffe75295b448f826ab04276ce8888ef52b1/> avoid Terraform crashing on empty provider block

### Refactor

 - <csr-id-a88a550cefde3a56790859127edc6a4e27e07090/> adjust `ProgressTracker::println`
   A small refactor pulled out of the perf tracing work, barely related to
   #1359
 - <csr-id-77246e77df47a0006dcb3eaeeb76882efacfd25c/> cleanup handling of arc `Weak` in `deployment.rs`

### Style

 - <csr-id-3fde68d0db0414017cfb771a218b14b8f57d1686/> use `name_of!` macro

### New Features (BREAKING)

 - <csr-id-749a10307f4eff2a46a1056735e84ed94d44b39e/> Perf works over SSH
   See documentation on how to use in
   [Notion](https://www.notion.so/hydro-project/perf-Measuring-CPU-usage-6135b6ce56a94af38eeeba0a55deef9c).
 - <csr-id-43a411ea6ca0ad5110754fe788bb7593519cba51/> Perf works over SSH
   See documentation on how to use in
   [Notion](https://www.notion.so/hydro-project/perf-Measuring-CPU-usage-6135b6ce56a94af38eeeba0a55deef9c).

### Refactor (BREAKING)

 - <csr-id-0a465e55dd39c76bc1aefb020460a639d792fe87/> rename integration crates to drop CLI references
 - <csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-bb081d3b0af6dbce9630e23dfe8b7d1363751c2b/> end-to-end flamegraph generation, fix #1365
   Depends on #1370
 - <csr-id-a2147864b24110c9ae2c1553e9e8b55bd5065f15/> `Deployment.stop()` for graceful shutdown including updated `perf` profile downloading
   * `perf` profile downloading moved from the `drop()` impl to `async fn
   stop()`
   * download perf data via stdout
   * update async-ssh2-lite to 0.5 to cleanup tokio compat issues
   
   WIP for #1365
 - <csr-id-8856c8596d5ad9d5f24a46467690bfac1549fae2/> use `buildstructor` to handle excessive `Deployment` method arguments, fix #1364
   Adds new method `Deployment::AzureHost`

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-bb081d3b0af6dbce9630e23dfe8b7d1363751c2b/> end-to-end flamegraph generation, fix #1365
   Depends on #1370
 - <csr-id-a2147864b24110c9ae2c1553e9e8b55bd5065f15/> `Deployment.stop()` for graceful shutdown including updated `perf` profile downloading
   * `perf` profile downloading moved from the `drop()` impl to `async fn
   stop()`
   * download perf data via stdout
   * update async-ssh2-lite to 0.5 to cleanup tokio compat issues
   
   WIP for #1365
 - <csr-id-8856c8596d5ad9d5f24a46467690bfac1549fae2/> use `buildstructor` to handle excessive `Deployment` method arguments, fix #1364
   Adds new method `Deployment::AzureHost`

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-bb081d3b0af6dbce9630e23dfe8b7d1363751c2b/> end-to-end flamegraph generation, fix #1365
   Depends on #1370
 - <csr-id-a2147864b24110c9ae2c1553e9e8b55bd5065f15/> `Deployment.stop()` for graceful shutdown including updated `perf` profile downloading
   * `perf` profile downloading moved from the `drop()` impl to `async fn
   stop()`
   * download perf data via stdout
   * update async-ssh2-lite to 0.5 to cleanup tokio compat issues
   
   WIP for #1365
 - <csr-id-8856c8596d5ad9d5f24a46467690bfac1549fae2/> use `buildstructor` to handle excessive `Deployment` method arguments, fix #1364
   Adds new method `Deployment::AzureHost`

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-bb081d3b0af6dbce9630e23dfe8b7d1363751c2b/> end-to-end flamegraph generation, fix #1365
   Depends on #1370
 - <csr-id-a2147864b24110c9ae2c1553e9e8b55bd5065f15/> `Deployment.stop()` for graceful shutdown including updated `perf` profile downloading
   * `perf` profile downloading moved from the `drop()` impl to `async fn
   stop()`
   * download perf data via stdout
   * update async-ssh2-lite to 0.5 to cleanup tokio compat issues
   
   WIP for #1365
 - <csr-id-8856c8596d5ad9d5f24a46467690bfac1549fae2/> use `buildstructor` to handle excessive `Deployment` method arguments, fix #1364
   Adds new method `Deployment::AzureHost`

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-bb081d3b0af6dbce9630e23dfe8b7d1363751c2b/> end-to-end flamegraph generation, fix #1365
   Depends on #1370
 - <csr-id-a2147864b24110c9ae2c1553e9e8b55bd5065f15/> `Deployment.stop()` for graceful shutdown including updated `perf` profile downloading
   * `perf` profile downloading moved from the `drop()` impl to `async fn
   stop()`
   * download perf data via stdout
   * update async-ssh2-lite to 0.5 to cleanup tokio compat issues
   
   WIP for #1365
 - <csr-id-8856c8596d5ad9d5f24a46467690bfac1549fae2/> use `buildstructor` to handle excessive `Deployment` method arguments, fix #1364
   Adds new method `Deployment::AzureHost`

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-bb081d3b0af6dbce9630e23dfe8b7d1363751c2b/> end-to-end flamegraph generation, fix #1365
   Depends on #1370
 - <csr-id-a2147864b24110c9ae2c1553e9e8b55bd5065f15/> `Deployment.stop()` for graceful shutdown including updated `perf` profile downloading
   * `perf` profile downloading moved from the `drop()` impl to `async fn
   stop()`
   * download perf data via stdout
   * update async-ssh2-lite to 0.5 to cleanup tokio compat issues
   
   WIP for #1365
 - <csr-id-8856c8596d5ad9d5f24a46467690bfac1549fae2/> use `buildstructor` to handle excessive `Deployment` method arguments, fix #1364
   Adds new method `Deployment::AzureHost`

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-bb081d3b0af6dbce9630e23dfe8b7d1363751c2b/> end-to-end flamegraph generation, fix #1365
   Depends on #1370
 - <csr-id-a2147864b24110c9ae2c1553e9e8b55bd5065f15/> `Deployment.stop()` for graceful shutdown including updated `perf` profile downloading
   * `perf` profile downloading moved from the `drop()` impl to `async fn
   stop()`
   * download perf data via stdout
   * update async-ssh2-lite to 0.5 to cleanup tokio compat issues
   
   WIP for #1365
 - <csr-id-8856c8596d5ad9d5f24a46467690bfac1549fae2/> use `buildstructor` to handle excessive `Deployment` method arguments, fix #1364
   Adds new method `Deployment::AzureHost`

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-bb081d3b0af6dbce9630e23dfe8b7d1363751c2b/> end-to-end flamegraph generation, fix #1365
   Depends on #1370
 - <csr-id-a2147864b24110c9ae2c1553e9e8b55bd5065f15/> `Deployment.stop()` for graceful shutdown including updated `perf` profile downloading
   * `perf` profile downloading moved from the `drop()` impl to `async fn
   stop()`
   * download perf data via stdout
   * update async-ssh2-lite to 0.5 to cleanup tokio compat issues
   
   WIP for #1365
 - <csr-id-8856c8596d5ad9d5f24a46467690bfac1549fae2/> use `buildstructor` to handle excessive `Deployment` method arguments, fix #1364
   Adds new method `Deployment::AzureHost`

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-bb081d3b0af6dbce9630e23dfe8b7d1363751c2b/> end-to-end flamegraph generation, fix #1365
   Depends on #1370
 - <csr-id-a2147864b24110c9ae2c1553e9e8b55bd5065f15/> `Deployment.stop()` for graceful shutdown including updated `perf` profile downloading
   * `perf` profile downloading moved from the `drop()` impl to `async fn
   stop()`
   * download perf data via stdout
   * update async-ssh2-lite to 0.5 to cleanup tokio compat issues
   
   WIP for #1365
 - <csr-id-8856c8596d5ad9d5f24a46467690bfac1549fae2/> use `buildstructor` to handle excessive `Deployment` method arguments, fix #1364
   Adds new method `Deployment::AzureHost`

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-bb081d3b0af6dbce9630e23dfe8b7d1363751c2b/> end-to-end flamegraph generation, fix #1365
   Depends on #1370
 - <csr-id-a2147864b24110c9ae2c1553e9e8b55bd5065f15/> `Deployment.stop()` for graceful shutdown including updated `perf` profile downloading
   * `perf` profile downloading moved from the `drop()` impl to `async fn
   stop()`
   * download perf data via stdout
   * update async-ssh2-lite to 0.5 to cleanup tokio compat issues
   
   WIP for #1365
 - <csr-id-8856c8596d5ad9d5f24a46467690bfac1549fae2/> use `buildstructor` to handle excessive `Deployment` method arguments, fix #1364
   Adds new method `Deployment::AzureHost`

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-bb081d3b0af6dbce9630e23dfe8b7d1363751c2b/> end-to-end flamegraph generation, fix #1365
   Depends on #1370
 - <csr-id-a2147864b24110c9ae2c1553e9e8b55bd5065f15/> `Deployment.stop()` for graceful shutdown including updated `perf` profile downloading
   * `perf` profile downloading moved from the `drop()` impl to `async fn
   stop()`
   * download perf data via stdout
   * update async-ssh2-lite to 0.5 to cleanup tokio compat issues
   
   WIP for #1365
 - <csr-id-8856c8596d5ad9d5f24a46467690bfac1549fae2/> use `buildstructor` to handle excessive `Deployment` method arguments, fix #1364
   Adds new method `Deployment::AzureHost`

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 20 commits contributed to the release.
 - 38 days passed between releases.
 - 18 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 17 unique issues were worked on: [#1313](https://github.com/hydro-project/hydro/issues/1313), [#1360](https://github.com/hydro-project/hydro/issues/1360), [#1366](https://github.com/hydro-project/hydro/issues/1366), [#1369](https://github.com/hydro-project/hydro/issues/1369), [#1370](https://github.com/hydro-project/hydro/issues/1370), [#1372](https://github.com/hydro-project/hydro/issues/1372), [#1378](https://github.com/hydro-project/hydro/issues/1378), [#1394](https://github.com/hydro-project/hydro/issues/1394), [#1396](https://github.com/hydro-project/hydro/issues/1396), [#1398](https://github.com/hydro-project/hydro/issues/1398), [#1403](https://github.com/hydro-project/hydro/issues/1403), [#1411](https://github.com/hydro-project/hydro/issues/1411), [#1413](https://github.com/hydro-project/hydro/issues/1413), [#1423](https://github.com/hydro-project/hydro/issues/1423), [#1428](https://github.com/hydro-project/hydro/issues/1428), [#1429](https://github.com/hydro-project/hydro/issues/1429), [#1431](https://github.com/hydro-project/hydro/issues/1431)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1313](https://github.com/hydro-project/hydro/issues/1313)**
    - Fixup! feat(hydro_deploy)!: Perf works over SSH ([`03006d3`](https://github.com/hydro-project/hydro/commit/03006d3964ef296109664989e2ee368ef0c35186))
    - Perf works over SSH ([`43a411e`](https://github.com/hydro-project/hydro/commit/43a411ea6ca0ad5110754fe788bb7593519cba51))
 * **[#1360](https://github.com/hydro-project/hydro/issues/1360)**
    - Avoid Terraform crashing on empty provider block ([`fd72bff`](https://github.com/hydro-project/hydro/commit/fd72bffe75295b448f826ab04276ce8888ef52b1))
 * **[#1366](https://github.com/hydro-project/hydro/issues/1366)**
    - Use `buildstructor` to handle excessive `Deployment` method arguments, fix #1364 ([`8bcd86c`](https://github.com/hydro-project/hydro/commit/8bcd86c15bc4d9d2e3b564061be879bfe8820e25))
 * **[#1369](https://github.com/hydro-project/hydro/issues/1369)**
    - Cleanup handling of arc `Weak` in `deployment.rs` ([`a5d649b`](https://github.com/hydro-project/hydro/commit/a5d649b5a5cc54c7bc56011db33d509a5cb370a2))
 * **[#1370](https://github.com/hydro-project/hydro/issues/1370)**
    - `Deployment.stop()` for graceful shutdown including updated `perf` profile downloading ([`9a503cf`](https://github.com/hydro-project/hydro/commit/9a503cf85225ff1fcfe7a815fda3a4ac34a75c42))
 * **[#1372](https://github.com/hydro-project/hydro/issues/1372)**
    - End-to-end flamegraph generation, fix #1365 ([`ac8dcbf`](https://github.com/hydro-project/hydro/commit/ac8dcbf7c6dbe018907a3012b71b0e4fcf4d2cb6))
 * **[#1378](https://github.com/hydro-project/hydro/issues/1378)**
    - Adjust `ProgressTracker::println` ([`36300cf`](https://github.com/hydro-project/hydro/commit/36300cfe3879e5fed04a8f0806762626612ca9f7))
 * **[#1394](https://github.com/hydro-project/hydro/issues/1394)**
    - Simplify process/cluster specs ([`10bd978`](https://github.com/hydro-project/hydro/commit/10bd978793ccde8fc287aedd77729c0c6e5f1784))
 * **[#1396](https://github.com/hydro-project/hydro/issues/1396)**
    - Add end-to-end flamegraph generation for macos and linux localhost, fix #1351 ([`3cd309b`](https://github.com/hydro-project/hydro/commit/3cd309b3d0ef2661a096c0bdc38e271f9f9ce326))
 * **[#1398](https://github.com/hydro-project/hydro/issues/1398)**
    - Use trybuild to compile subgraph binaries ([`2a49c13`](https://github.com/hydro-project/hydro/commit/2a49c13f2f4e3b47d79c34167015d6ba98a89888))
 * **[#1403](https://github.com/hydro-project/hydro/issues/1403)**
    - Only instantiate `Localhost` once ([`628066b`](https://github.com/hydro-project/hydro/commit/628066bf8250b541493c8cf5efd6c7bf01900640))
 * **[#1411](https://github.com/hydro-project/hydro/issues/1411)**
    - Improve progress UX by collapsing nested groups ([`d68f280`](https://github.com/hydro-project/hydro/commit/d68f280ed944e001a7b3ca6954beeef2e4d984bb))
 * **[#1413](https://github.com/hydro-project/hydro/issues/1413)**
    - Rename integration crates to drop CLI references ([`25989c7`](https://github.com/hydro-project/hydro/commit/25989c7d938a0e93355a670f8d78a5aea900fce0))
 * **[#1423](https://github.com/hydro-project/hydro/issues/1423)**
    - Lower min dependency versions where possible, update `Cargo.lock` ([`2c04f51`](https://github.com/hydro-project/hydro/commit/2c04f51f1ec44f7898307b6610371dcb490ea686))
 * **[#1428](https://github.com/hydro-project/hydro/issues/1428)**
    - Cleanup doc comments for clippy latest ([`1766c8b`](https://github.com/hydro-project/hydro/commit/1766c8b0aa23df83ad242b581184b37e85afe27b))
 * **[#1429](https://github.com/hydro-project/hydro/issues/1429)**
    - Use `name_of!` macro ([`5545c8b`](https://github.com/hydro-project/hydro/commit/5545c8b3329902b6b2418476d00191228f5f3e8d))
 * **[#1431](https://github.com/hydro-project/hydro/issues/1431)**
    - Only record usermode events in perf ([`cd04172`](https://github.com/hydro-project/hydro/commit/cd0417229f3c268362013265f514d703d4af2c3d))
 * **Uncategorized**
    - Release hydroflow_lang v0.9.0, hydroflow_datalog_core v0.9.0, hydroflow_datalog v0.9.0, hydroflow_deploy_integration v0.9.0, hydroflow_macro v0.9.0, lattices_macro v0.5.6, lattices v0.5.7, multiplatform_test v0.2.0, variadics v0.0.6, pusherator v0.0.8, hydroflow v0.9.0, stageleft_macro v0.3.0, stageleft v0.4.0, stageleft_tool v0.3.0, hydroflow_plus v0.9.0, hydro_deploy v0.9.0, hydro_cli v0.9.0, hydroflow_plus_deploy v0.9.0, safety bump 8 crates ([`1d54331`](https://github.com/hydro-project/hydro/commit/1d54331976040c049e4c97a9fba0e66930efee52))
    - Manually set versions for crates renamed in #1413 ([`3508f5a`](https://github.com/hydro-project/hydro/commit/3508f5aeda3e18a6857df4ceb77e5e1015c02a17))
</details>

## v0.8.0 (2024-07-23)

<csr-id-e3e69334fcba8488b6fad3975fb0ba88e82a4b02/>
<csr-id-0feae7454e4674eea1f3308b3d6d4e9d459cda67/>
<csr-id-947ebc1cb21a07fbfacae4ac956dbd0015a8a418/>
<csr-id-22865583a4260fe401c28aa39a74987478edc73d/>
<csr-id-c5a8de28e7844b3c29d58116d8340967f2e6bcc4/>
<csr-id-f536eccf7297be8185108b60897e92ad0efffe4a/>
<csr-id-057a0a510568cf81932368c8c65e056f91af7202/>
<csr-id-60390782dd7dcec18d193c800af716843a944dba/>
<csr-id-141eae1c3a1869fa42756250618a21ea2a2c7e34/>
<csr-id-12b8ba53f28eb9de1318b41cdf1e23282f6f0eb6/>
<csr-id-fbd7fb9bed9fd8d2afdfb5ad0edf076c3ad0f83f/>
<csr-id-bb98c570fd41bd4c4b2566ff0388ce0323ab0867/>
<csr-id-a97480ab834293bcc81d81fcd10d8944eb312417/>
<csr-id-be590007152c9439bfb1a0e153ff89e514265877/>
<csr-id-453fbce73423815752667c560318efe8b78014f8/>
<csr-id-0983248beab176debc602f92fa617f9beb02dad3/>
<csr-id-dd759aea1ac225654501e836b890dd8d144868b4/>
<csr-id-d56c731482e25f3ab397c4912df35a6375fcb23a/>
<csr-id-bd0a4cdae3a14862b28df6a2eea8521ffdf16070/>
<csr-id-dfe7a0938c302353db05d9889eb8d88640887443/>

### Refactor

 - <csr-id-e3e69334fcba8488b6fad3975fb0ba88e82a4b02/> remove unneeded `Arc<RwLock<` wrapping of `launch_binary` return value (1/3)
   > Curious if there was any intention behind why it was `Arc<RwLock<`?
   
   > I think before some refactors we took the I/O handles instead of using broadcast channels.
 - <csr-id-0feae7454e4674eea1f3308b3d6d4e9d459cda67/> build cache cleanup
   * Replace mystery tuple with new `struct BuildOutput`
   * Replace `Mutex` and `Arc`-infested `HashMap` with `memo-map` crate,
   greatly simplifying build cache typing
   * Remove redundant build caching in `HydroflowCrateService`, expose and
   use cache parameters as `BuildParams`
   * Remove `once_cell` and `async-once-cell` dependencies, use `std`'s
   `OnceLock`
   * Add `Failed to execute command: {}` context to `perf` error message
   * Cleanup some repeated `format!` expressions

### Style (BREAKING)

 - <csr-id-fbd7fb9bed9fd8d2afdfb5ad0edf076c3ad0f83f/> enable clippy `upper-case-acronyms-aggressive`
   * rename `GCP` -> `Gcp`, `NodeID` -> `NodeId`
   * update CI `cargo-generate` template testing to use PR's branch instead
   of whatever `main` happens to be

### Refactor (BREAKING)

 - <csr-id-bb98c570fd41bd4c4b2566ff0388ce0323ab0867/> make `Service::collect_resources` take `&self` instead of `&mut self`
   #430 but still has `RwLock` wrapping
   
   Depends on #1347
 - <csr-id-a97480ab834293bcc81d81fcd10d8944eb312417/> make `Host` trait use `&self` interior mutability to remove `RwLock` wrappings #430
   Depends on #1346
 - <csr-id-be590007152c9439bfb1a0e153ff89e514265877/> Make `Host::provision` not async anymore
   I noticed that none of the method impls have any `await`s
 - <csr-id-453fbce73423815752667c560318efe8b78014f8/> make `HydroflowSource`, `HydroflowSink` traits use `&self` interior mutability to remove `RwLock` wrappings #430
   Depends on #1339
 - <csr-id-0983248beab176debc602f92fa617f9beb02dad3/> replace `async-channel` with `tokio::sync::mpsc::unbounded_channel`
   Depends on #1339
   
   We could make the publicly facing `stdout`, `stderr` APIs return `impl Stream<Output = String>` in the future, maybe
 - <csr-id-dd759aea1ac225654501e836b890dd8d144868b4/> replace some uses of `tokio::sync::RwLock` with `std::sync::Mutex` #430 (3/3)

### Style

 - <csr-id-d56c731482e25f3ab397c4912df35a6375fcb23a/> rename `SSH` -> `Ssh`

### Refactor

 - <csr-id-bd0a4cdae3a14862b28df6a2eea8521ffdf16070/> remove unneeded `Arc<RwLock<` wrapping of `launch_binary` return value (1/3)
   > Curious if there was any intention behind why it was `Arc<RwLock<`?
   
   > I think before some refactors we took the I/O handles instead of using broadcast channels.
 - <csr-id-dfe7a0938c302353db05d9889eb8d88640887443/> build cache cleanup
   * Replace mystery tuple with new `struct BuildOutput`
   * Replace `Mutex` and `Arc`-infested `HashMap` with `memo-map` crate,
   greatly simplifying build cache typing
   * Remove redundant build caching in `HydroflowCrateService`, expose and
   use cache parameters as `BuildParams`
   * Remove `once_cell` and `async-once-cell` dependencies, use `std`'s
   `OnceLock`
   * Add `Failed to execute command: {}` context to `perf` error message
   * Cleanup some repeated `format!` expressions

### Style

 - <csr-id-947ebc1cb21a07fbfacae4ac956dbd0015a8a418/> rename `SSH` -> `Ssh`

### Refactor (BREAKING)

 - <csr-id-22865583a4260fe401c28aa39a74987478edc73d/> make `Service::collect_resources` take `&self` instead of `&mut self`
   #430 but still has `RwLock` wrapping
   
   Depends on #1347
 - <csr-id-c5a8de28e7844b3c29d58116d8340967f2e6bcc4/> make `Host` trait use `&self` interior mutability to remove `RwLock` wrappings #430
   Depends on #1346
 - <csr-id-f536eccf7297be8185108b60897e92ad0efffe4a/> Make `Host::provision` not async anymore
   I noticed that none of the method impls have any `await`s
 - <csr-id-057a0a510568cf81932368c8c65e056f91af7202/> make `HydroflowSource`, `HydroflowSink` traits use `&self` interior mutability to remove `RwLock` wrappings #430
   Depends on #1339
 - <csr-id-60390782dd7dcec18d193c800af716843a944dba/> replace `async-channel` with `tokio::sync::mpsc::unbounded_channel`
   Depends on #1339
   
   We could make the publicly facing `stdout`, `stderr` APIs return `impl Stream<Output = String>` in the future, maybe
 - <csr-id-141eae1c3a1869fa42756250618a21ea2a2c7e34/> replace some uses of `tokio::sync::RwLock` with `std::sync::Mutex` #430 (3/3)

### Style (BREAKING)

 - <csr-id-12b8ba53f28eb9de1318b41cdf1e23282f6f0eb6/> enable clippy `upper-case-acronyms-aggressive`
   * rename `GCP` -> `Gcp`, `NodeID` -> `NodeId`
   * update CI `cargo-generate` template testing to use PR's branch instead
   of whatever `main` happens to be

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 11 commits contributed to the release.
 - 59 days passed between releases.
 - 10 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 10 unique issues were worked on: [#1334](https://github.com/hydro-project/hydro/issues/1334), [#1338](https://github.com/hydro-project/hydro/issues/1338), [#1339](https://github.com/hydro-project/hydro/issues/1339), [#1340](https://github.com/hydro-project/hydro/issues/1340), [#1343](https://github.com/hydro-project/hydro/issues/1343), [#1345](https://github.com/hydro-project/hydro/issues/1345), [#1346](https://github.com/hydro-project/hydro/issues/1346), [#1347](https://github.com/hydro-project/hydro/issues/1347), [#1348](https://github.com/hydro-project/hydro/issues/1348), [#1356](https://github.com/hydro-project/hydro/issues/1356)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1334](https://github.com/hydro-project/hydro/issues/1334)**
    - Build cache cleanup ([`dfe7a09`](https://github.com/hydro-project/hydro/commit/dfe7a0938c302353db05d9889eb8d88640887443))
 * **[#1338](https://github.com/hydro-project/hydro/issues/1338)**
    - Remove unneeded `Arc<RwLock<` wrapping of `launch_binary` return value (1/3) ([`bd0a4cd`](https://github.com/hydro-project/hydro/commit/bd0a4cdae3a14862b28df6a2eea8521ffdf16070))
 * **[#1339](https://github.com/hydro-project/hydro/issues/1339)**
    - Replace some uses of `tokio::sync::RwLock` with `std::sync::Mutex` #430 (3/3) ([`dd759ae`](https://github.com/hydro-project/hydro/commit/dd759aea1ac225654501e836b890dd8d144868b4))
 * **[#1340](https://github.com/hydro-project/hydro/issues/1340)**
    - Rename `SSH` -> `Ssh` ([`d56c731`](https://github.com/hydro-project/hydro/commit/d56c731482e25f3ab397c4912df35a6375fcb23a))
 * **[#1343](https://github.com/hydro-project/hydro/issues/1343)**
    - Make `Host::provision` not async anymore ([`be59000`](https://github.com/hydro-project/hydro/commit/be590007152c9439bfb1a0e153ff89e514265877))
 * **[#1345](https://github.com/hydro-project/hydro/issues/1345)**
    - Enable clippy `upper-case-acronyms-aggressive` ([`fbd7fb9`](https://github.com/hydro-project/hydro/commit/fbd7fb9bed9fd8d2afdfb5ad0edf076c3ad0f83f))
 * **[#1346](https://github.com/hydro-project/hydro/issues/1346)**
    - Make `HydroflowSource`, `HydroflowSink` traits use `&self` interior mutability to remove `RwLock` wrappings #430 ([`453fbce`](https://github.com/hydro-project/hydro/commit/453fbce73423815752667c560318efe8b78014f8))
 * **[#1347](https://github.com/hydro-project/hydro/issues/1347)**
    - Make `Host` trait use `&self` interior mutability to remove `RwLock` wrappings #430 ([`a97480a`](https://github.com/hydro-project/hydro/commit/a97480ab834293bcc81d81fcd10d8944eb312417))
 * **[#1348](https://github.com/hydro-project/hydro/issues/1348)**
    - Make `Service::collect_resources` take `&self` instead of `&mut self` ([`bb98c57`](https://github.com/hydro-project/hydro/commit/bb98c570fd41bd4c4b2566ff0388ce0323ab0867))
 * **[#1356](https://github.com/hydro-project/hydro/issues/1356)**
    - Replace `async-channel` with `tokio::sync::mpsc::unbounded_channel` ([`0983248`](https://github.com/hydro-project/hydro/commit/0983248beab176debc602f92fa617f9beb02dad3))
 * **Uncategorized**
    - Release hydroflow_lang v0.8.0, hydroflow_datalog_core v0.8.0, hydroflow_datalog v0.8.0, hydroflow_macro v0.8.0, lattices_macro v0.5.5, lattices v0.5.6, variadics v0.0.5, pusherator v0.0.7, hydroflow v0.8.0, hydroflow_plus v0.8.0, hydro_deploy v0.8.0, hydro_cli v0.8.0, hydroflow_plus_cli_integration v0.8.0, safety bump 7 crates ([`7b9c367`](https://github.com/hydro-project/hydro/commit/7b9c3678930af8010f8e2ffd4069583ece528119))
</details>

## v0.7.0 (2024-05-24)

### New Features

 - <csr-id-29a263fb564c5ce4bc495ea4e9d20b8b2621b645/> add support for collecting counts and running perf
 - <csr-id-a33d9e29bcab427961dbfe2f03d80a9b87ecda6c/> add support for collecting counts and running perf

### Bug Fixes

 - <csr-id-92c72ba9527241f88dfb23f64b999c8e4bd2b26c/> end processes with SIGTERM instead of SIGKILL
   fix(hydro_deploy): end processes with SIGTERM instead of SIGKILL
 - <csr-id-d5d6bd65e747b74bfc89e3ac6168f6731b869aa1/> end processes with SIGTERM instead of SIGKILL
   fix(hydro_deploy): end processes with SIGTERM instead of SIGKILL

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 44 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#1129](https://github.com/hydro-project/hydro/issues/1129), [#1157](https://github.com/hydro-project/hydro/issues/1157)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1129](https://github.com/hydro-project/hydro/issues/1129)**
    - End processes with SIGTERM instead of SIGKILL ([`d5d6bd6`](https://github.com/hydro-project/hydro/commit/d5d6bd65e747b74bfc89e3ac6168f6731b869aa1))
 * **[#1157](https://github.com/hydro-project/hydro/issues/1157)**
    - Add support for collecting counts and running perf ([`a33d9e2`](https://github.com/hydro-project/hydro/commit/a33d9e29bcab427961dbfe2f03d80a9b87ecda6c))
 * **Uncategorized**
    - Release hydroflow_lang v0.7.0, hydroflow_datalog_core v0.7.0, hydroflow_datalog v0.7.0, hydroflow_macro v0.7.0, lattices v0.5.5, multiplatform_test v0.1.0, pusherator v0.0.6, hydroflow v0.7.0, stageleft_macro v0.2.0, stageleft v0.3.0, stageleft_tool v0.2.0, hydroflow_plus v0.7.0, hydro_deploy v0.7.0, hydro_cli v0.7.0, hydroflow_plus_cli_integration v0.7.0, safety bump 8 crates ([`855fda6`](https://github.com/hydro-project/hydro/commit/855fda65442ad7a9074a099ecc29e74322332418))
</details>

## v0.6.1 (2024-04-09)

<csr-id-7958fb0d900be8fe7359326abfa11dcb8fb35e8a/>
<csr-id-864ea856ecbabfe6786990924021a70fb4252765/>

### Style

 - <csr-id-7958fb0d900be8fe7359326abfa11dcb8fb35e8a/> qualified path cleanups for clippy

### Style

 - <csr-id-864ea856ecbabfe6786990924021a70fb4252765/> qualified path cleanups for clippy

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 38 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1090](https://github.com/hydro-project/hydro/issues/1090)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1090](https://github.com/hydro-project/hydro/issues/1090)**
    - Qualified path cleanups for clippy ([`864ea85`](https://github.com/hydro-project/hydro/commit/864ea856ecbabfe6786990924021a70fb4252765))
 * **Uncategorized**
    - Release hydroflow_plus v0.6.1, hydro_deploy v0.6.1, hydro_cli v0.6.1, hydroflow_plus_cli_integration v0.6.1 ([`499965b`](https://github.com/hydro-project/hydro/commit/499965b2bd27d3fca7e328b19960761bb64c0c0e))
    - Release hydroflow_lang v0.6.2, hydroflow v0.6.2, hydroflow_plus v0.6.1, hydro_deploy v0.6.1, hydro_cli v0.6.1, hydroflow_plus_cli_integration v0.6.1, stageleft_tool v0.1.1 ([`67e16d0`](https://github.com/hydro-project/hydro/commit/67e16d069a2d565039dcf17e6caf0a23e258f983))
    - Release hydroflow_cli_integration v0.5.2, hydroflow_lang v0.6.1, hydroflow_datalog_core v0.6.1, lattices v0.5.4, hydroflow v0.6.1, stageleft_macro v0.1.1, stageleft v0.2.1, hydroflow_plus v0.6.1, hydro_deploy v0.6.1, hydro_cli v0.6.1, hydroflow_plus_cli_integration v0.6.1, stageleft_tool v0.1.1 ([`fb82e52`](https://github.com/hydro-project/hydro/commit/fb82e523bb217658775989a276e18a1af68103c8))
</details>

## v0.6.0 (2024-03-02)

<csr-id-39ab8b0278e9e3fe96552ace0a4ae768a6bc10d8/>
<csr-id-e9639f608f8dafd3f384837067800a66951b25df/>
<csr-id-d8203407a97c2ccbcb5ce0cc739d8ae5a89a40c7/>
<csr-id-65c7ebe3d64c478e7a4f0d8eb12e2bb3c1b267a3/>

### Chore

 - <csr-id-39ab8b0278e9e3fe96552ace0a4ae768a6bc10d8/> appease various clippy lints

### Other

 - <csr-id-d8203407a97c2ccbcb5ce0cc739d8ae5a89a40c7/> consolidate tasks and use sccache and nextest

### Chore

 - <csr-id-65c7ebe3d64c478e7a4f0d8eb12e2bb3c1b267a3/> appease various clippy lints

### New Features

 - <csr-id-fcf43bf86fe550247dffa4641a9ce3aff3b9afc3/> Add support for azure
   I accidentally committed some large files, so you won't see the commit
   history because I copied over the changes onto a fresh clone.
 - <csr-id-8021da6e5fa5127dc67420157dff980d51c710ed/> Add support for azure
   I accidentally committed some large files, so you won't see the commit
   history because I copied over the changes onto a fresh clone.

### Other

 - <csr-id-e9639f608f8dafd3f384837067800a66951b25df/> consolidate tasks and use sccache and nextest

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 3 unique issues were worked on: [#1015](https://github.com/hydro-project/hydro/issues/1015), [#1043](https://github.com/hydro-project/hydro/issues/1043), [#1084](https://github.com/hydro-project/hydro/issues/1084)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1015](https://github.com/hydro-project/hydro/issues/1015)**
    - Consolidate tasks and use sccache and nextest ([`d820340`](https://github.com/hydro-project/hydro/commit/d8203407a97c2ccbcb5ce0cc739d8ae5a89a40c7))
 * **[#1043](https://github.com/hydro-project/hydro/issues/1043)**
    - Add support for azure ([`8021da6`](https://github.com/hydro-project/hydro/commit/8021da6e5fa5127dc67420157dff980d51c710ed))
 * **[#1084](https://github.com/hydro-project/hydro/issues/1084)**
    - Appease various clippy lints ([`65c7ebe`](https://github.com/hydro-project/hydro/commit/65c7ebe3d64c478e7a4f0d8eb12e2bb3c1b267a3))
 * **Uncategorized**
    - Release hydroflow_lang v0.6.0, hydroflow_datalog_core v0.6.0, hydroflow_datalog v0.6.0, hydroflow_macro v0.6.0, lattices v0.5.3, variadics v0.0.4, pusherator v0.0.5, hydroflow v0.6.0, stageleft v0.2.0, hydroflow_plus v0.6.0, hydro_deploy v0.6.0, hydro_cli v0.6.0, hydroflow_plus_cli_integration v0.6.0, safety bump 7 crates ([`0e94db4`](https://github.com/hydro-project/hydro/commit/0e94db41c842c1181574c5e69179027cfa7a19cf))
</details>

## v0.5.1 (2024-01-29)

<csr-id-1b555e57c8c812bed4d6495d2960cbf77fb0b3ef/>
<csr-id-7c48faf0d8301b498fa59e5eee5cddf5fa341229/>

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

### Documentation

 - <csr-id-3b36020d16792f26da4df3c5b09652a4ab47ec4f/> actually committing empty CHANGELOG.md is required
 - <csr-id-b9bf86c5f104dda98f76182641927c7916b54ee5/> actually committing empty CHANGELOG.md is required

### New Features

 - <csr-id-20fd1e5f876c5977e44a58757f41c66bdf6a3d15/> improve build error message debuggability
 - <csr-id-46d87fa364d3fe01422cf3c404fbc8a1d5e9fb88/> pass subgraph ID through deploy metadata
 - <csr-id-b7aafd3c97897db4bff62c4ab0b7480ef9a799e0/> improve API naming and eliminate wire API for builders
 - <csr-id-53d7aee8dcc574d47864ec89bfea30a82eab0ee7/> improve Rust API for defining services
 - <csr-id-c50ca121b6d5e30dc07843f82caa135b68626301/> split Rust core from Python bindings
 - <csr-id-fae7b4168905910bb55be9e35420ceb3f475dc36/> improve build error message debuggability
 - <csr-id-6a1ea22312466fb641194133cfba3def16734f09/> pass subgraph ID through deploy metadata
 - <csr-id-f441378f4194333af9e220284132ec82e6d87124/> improve API naming and eliminate wire API for builders
 - <csr-id-4133f52a40f7f77fb1d0bb44952815bc1fa4f1a5/> improve Rust API for defining services
 - <csr-id-04553830046ac51fcaa212c2565a742f56b3a3e5/> split Rust core from Python bindings

### Bug Fixes

 - <csr-id-d23c2299098dd62058c0951c99a62bb9e0af5b25/> avoid inflexible `\\?\` canonical paths on windows to mitigate `/` separator errors
 - <csr-id-f8a0b95113e92e003061d2a3865c84d69851dd8e/> race conditions when handshake channels capture other outputs
   Timeouts in Hydroflow+ tests were being caused by a race condition in Hydro Deploy where stdout sent after a handshake message would sometimes be sent to the `cli_stdout` channel for handshakes.
   
   This PR adjusts the handshake channels to always be oneshot, so that the broadcaster immediately knows when to send data to the regular stdout channels.
   
   Also refactors Hydro Deploy sources to split up more modules.
 - <csr-id-1ae27de6aafb72cee5da0cce6cf52748161d0f33/> don't vendor openssl and fix docker build
 - <csr-id-1d8adc1df15bac74c6f4496589d615e361019f50/> fix docs and remove unnecessary async_trait
 - <csr-id-9a6995c7e110350a18f0ce04d9425b3b45bfc94f/> avoid inflexible `\\?\` canonical paths on windows to mitigate `/` separator errors
 - <csr-id-39f646f3f4db44597abd018b6881d7a25b17c32d/> race conditions when handshake channels capture other outputs
   Timeouts in Hydroflow+ tests were being caused by a race condition in Hydro Deploy where stdout sent after a handshake message would sometimes be sent to the `cli_stdout` channel for handshakes.
   
   This PR adjusts the handshake channels to always be oneshot, so that the broadcaster immediately knows when to send data to the regular stdout channels.
   
   Also refactors Hydro Deploy sources to split up more modules.
 - <csr-id-eef407e063aa0d9079dc800bd300c39185f4390a/> don't vendor openssl and fix docker build
 - <csr-id-119f055a7a094c3240495c34f00e1df3d49fedf9/> fix docs and remove unnecessary async_trait

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 12 commits contributed to the release over the course of 39 calendar days.
 - 11 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 9 unique issues were worked on: [#1010](https://github.com/hydro-project/hydro/issues/1010), [#1014](https://github.com/hydro-project/hydro/issues/1014), [#986](https://github.com/hydro-project/hydro/issues/986), [#987](https://github.com/hydro-project/hydro/issues/987), [#992](https://github.com/hydro-project/hydro/issues/992), [#994](https://github.com/hydro-project/hydro/issues/994), [#995](https://github.com/hydro-project/hydro/issues/995), [#996](https://github.com/hydro-project/hydro/issues/996), [#999](https://github.com/hydro-project/hydro/issues/999)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1010](https://github.com/hydro-project/hydro/issues/1010)**
    - Improve build error message debuggability ([`fae7b41`](https://github.com/hydro-project/hydro/commit/fae7b4168905910bb55be9e35420ceb3f475dc36))
 * **[#1014](https://github.com/hydro-project/hydro/issues/1014)**
    - Avoid inflexible `\\?\` canonical paths on windows to mitigate `/` separator errors ([`9a6995c`](https://github.com/hydro-project/hydro/commit/9a6995c7e110350a18f0ce04d9425b3b45bfc94f))
 * **[#986](https://github.com/hydro-project/hydro/issues/986)**
    - Split Rust core from Python bindings ([`0455383`](https://github.com/hydro-project/hydro/commit/04553830046ac51fcaa212c2565a742f56b3a3e5))
 * **[#987](https://github.com/hydro-project/hydro/issues/987)**
    - Improve Rust API for defining services ([`4133f52`](https://github.com/hydro-project/hydro/commit/4133f52a40f7f77fb1d0bb44952815bc1fa4f1a5))
 * **[#992](https://github.com/hydro-project/hydro/issues/992)**
    - Fix docs and remove unnecessary async_trait ([`119f055`](https://github.com/hydro-project/hydro/commit/119f055a7a094c3240495c34f00e1df3d49fedf9))
 * **[#994](https://github.com/hydro-project/hydro/issues/994)**
    - Don't vendor openssl and fix docker build ([`eef407e`](https://github.com/hydro-project/hydro/commit/eef407e063aa0d9079dc800bd300c39185f4390a))
 * **[#995](https://github.com/hydro-project/hydro/issues/995)**
    - Improve API naming and eliminate wire API for builders ([`f441378`](https://github.com/hydro-project/hydro/commit/f441378f4194333af9e220284132ec82e6d87124))
 * **[#996](https://github.com/hydro-project/hydro/issues/996)**
    - Pass subgraph ID through deploy metadata ([`6a1ea22`](https://github.com/hydro-project/hydro/commit/6a1ea22312466fb641194133cfba3def16734f09))
 * **[#999](https://github.com/hydro-project/hydro/issues/999)**
    - Race conditions when handshake channels capture other outputs ([`39f646f`](https://github.com/hydro-project/hydro/commit/39f646f3f4db44597abd018b6881d7a25b17c32d))
 * **Uncategorized**
    - Release hydro_deploy v0.5.1 ([`7d8e778`](https://github.com/hydro-project/hydro/commit/7d8e778254e98a15a07f939cb3c5ddc88504f25b))
    - Actually committing empty CHANGELOG.md is required ([`b9bf86c`](https://github.com/hydro-project/hydro/commit/b9bf86c5f104dda98f76182641927c7916b54ee5))
    - Manually set lockstep-versioned crates (and `lattices`) to version `0.5.1` ([`7c48faf`](https://github.com/hydro-project/hydro/commit/7c48faf0d8301b498fa59e5eee5cddf5fa341229))
</details>

