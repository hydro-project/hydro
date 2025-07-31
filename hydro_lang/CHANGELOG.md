# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.14.0 (2025-07-31)

<csr-id-a3280d9b95576aaaf188779feb57ab0a28088a90/>
<csr-id-3d40d1a65c41dca3893867fb567993a27491fa0c/>
<csr-id-98baec71a6f1d01d55a3c983fdbb7824c45305cd/>
<csr-id-22a7d0d9a057356a4a01ccdad6febc8aa229d325/>
<csr-id-49c1918b9563f9ece6f2fc422a3b3c91d592e0cc/>
<csr-id-59041df58be2de0717b851cc1c3355479cd722f2/>
<csr-id-eaac1f40d1050da65d2356597f36f37f0cf439ef/>
<csr-id-5ab815f3567d51e9bd114f90af8e837fe0732cd8/>
<csr-id-fb016b017a42df95286cb2236ef987ea68be9ea7/>
<csr-id-6e29285d4c30901abce3352c2cf358f36f9173c6/>

### Chore

 - <csr-id-a3280d9b95576aaaf188779feb57ab0a28088a90/> enable `viz` feature only in dev-dependencies
   Reduces compilation burden when running `hydro_test` examples.
 - <csr-id-3d40d1a65c41dca3893867fb567993a27491fa0c/> update `proc-macro-crate`
 - <csr-id-98baec71a6f1d01d55a3c983fdbb7824c45305cd/> update pinned nightly to 2025-04-27, update span API usage

### New Features

 - <csr-id-173d9c0f1cc955f2a478195bcf3de0b055284008/> Use partitioning analysis results to partition
   Test/insta changes stem from changed implementation of
   broadcast_bincode, will change again once #1949 is implemented.
   Also added missing cases for Persists hidden behind CrossProduct,
   Difference, AntiJoin, Join, and Scan for decoupler.
 - <csr-id-4035cae8536bf9f8421fee9a8ccca7eda7e846d6/> make it easier to open trybuild-generated files with Rust Analyzer
   To fully compile the generated sources without error, the Stageleft
   environment variable needt to be passed to build scripts, so
   pre-configure that in workspace settings.
 - <csr-id-4d15ff19e604e5b3193abf51bc540ab1a435f7c5/> graph viz for Hydro lang
 - <csr-id-bd1afdff5fd7b8dc6d2c567cd2659542a84c6216/> allow running generated binaries with single-threaded Tokio runtime
   Before, we had a janky architecture for establishing network connections
   which relied on blocking on futures outside an async context, which
   required a multi-threaded runtime. Now, we establish all connections
   before launching the DFIR code, so that no blocking is required there.
 - <csr-id-d44b225f697f8d7ccdf80b0d5dc2afdea1955324/> capture stack traces for each IR node
   Because Hydro is staged, the stack traces capture the structure of the
   program, which is helpful for profiling / visualization.
 - <csr-id-c4b9590552d7e05b352a0d4a215f1bbf274ccc61/> add `scan` operator
 - <csr-id-99a8f1dfdde087578f25c19e502ad13e1d98a394/> Decoupling analysis
   A Gurobi license is required to run code that uses `hydro_optimize` (for ILP over decoupling decisions)
 - <csr-id-6b0483d201f9d9871c9693765bd61284258a5bce/> add `*_idempotent` variations for (keyed) fold / reduce
   We were missing variants where the stream is totally ordered with
   consecutive retries, which show up when sampling singletons. This adds
   those in.
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
 - <csr-id-863cb9eecd9e408fc8eb7c010415bd117d56c4ac/> Assign cycles IDs that are globally unique (across clusters/processes)
   Co-authored with @shadaj

### Bug Fixes

 - <csr-id-1fc9f0dc938a50afc41cfe3fa8d6e34d0d1c947d/> emit appropriate `Persist` / `Unpersist` nodes for `scan`
 - <csr-id-8858abd784a3f20504264fcfeafde2fa5cc6a615/> ensure that singletons and optionals always have cardinality 1 inside a tick
   Otherwise, cycling a singleton can result in a memory leak, as we found
   in PBFT implementation. Now, we strictly require that inside a tick,
   singletons/optionals are represented by a stream with 1 or fewer
   elements.
 - <csr-id-739b622f7a6ee3e58289863519419bd925c1e788/> don't snapshot-test backtraces
   Backtraces aren't stable across Unix / Windows. Just have a separate
   test for them.
   
   Also defers resolution of backtraces until we actually need them to
   improve performance.
 - <csr-id-6699197d60a5efe8fd2fe951463649eca54ea9db/> correctly enable staged-trybuild mode when cross-compiling
   `RUSTFLAGS` are not passed to build scripts, use a regular environment
   variable instead. Should also dramatically improve cache hit rate for
   sccache since the rustflags for non-stageleft crates are untouched.

### Refactor

 - <csr-id-22a7d0d9a057356a4a01ccdad6febc8aa229d325/> separate externals from other location kinds, clean up network operators
   First, we remove externals from `LocationId`, to ensure that a
   `LocationId` only is used for locations where we can concretely place
   compiled logic. This simplifes a lot of pattern matching where we wanted
   to disallow externals.
   
   Keys on network inputs / outputs (`from_key` and `to_key`) are only
   relevant to external networking. We extract the logic to instantiate
   external network collections, so that the core logic does not need to
   deal with keys.
 - <csr-id-49c1918b9563f9ece6f2fc422a3b3c91d592e0cc/> rename `ExternalProcess` to `External`
   In preparation of supporting multi-connection source / sink pairs. There
   is no need to distinguish a single external from multiple at the
   location level, instead we will have separate APIs for declaring a
   single vs multi connection input/output.
 - <csr-id-59041df58be2de0717b851cc1c3355479cd722f2/> minimize Tokio feature flags
   Now that `hydro_lang` no longer needs multi-threaded runtime, we can
   eliminate it from the features used in `trybuild` compilation. Minimizes
   Tokio features elsewhere too.
 - <csr-id-eaac1f40d1050da65d2356597f36f37f0cf439ef/> migrate tests from hydro_test_local
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

### Refactor (BREAKING)

 - <csr-id-fb016b017a42df95286cb2236ef987ea68be9ea7/> invert external sources and clean up locations in IR
   First, instead of creating external sources by invoking
   `external.source_bincode_external(&p)`, we switch the API to
   `p.source_bincode_external(&external)` for symmetry with `source_iter`
   and `source_stream`.
   
   The other, much larger change is to clean up how the IR handles external
   inputs and outputs and keeps track of locations. First, we introduce
   `HydroNode::ExternalInput` and `HydroLeaf::SendExternal` as specialized
   nodes for these, so that we no longer create dummy sources / sinks.
   
   Then, we eliminate places where we have multiple sources of truth for
   where the output of an IR node is located, by instead referring to the
   metadata. Because it is easy in optimizer rewrites to corrupt this
   metadata, we also add a flag to `transform_bottom_up` that lets
   developers enable a metadata validity check. We disable it in most
   transformations for performance, but enable it in the decoupling
   rewrites since it manipulates locations in complex ways.
 - <csr-id-6e29285d4c30901abce3352c2cf358f36f9173c6/> remove support for macro entrypoints

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 26 commits contributed to the release over the course of 92 calendar days.
 - 25 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 25 unique issues were worked on: [#1803](https://github.com/hydro-project/hydro/issues/1803), [#1843](https://github.com/hydro-project/hydro/issues/1843), [#1859](https://github.com/hydro-project/hydro/issues/1859), [#1902](https://github.com/hydro-project/hydro/issues/1902), [#1905](https://github.com/hydro-project/hydro/issues/1905), [#1907](https://github.com/hydro-project/hydro/issues/1907), [#1909](https://github.com/hydro-project/hydro/issues/1909), [#1910](https://github.com/hydro-project/hydro/issues/1910), [#1916](https://github.com/hydro-project/hydro/issues/1916), [#1930](https://github.com/hydro-project/hydro/issues/1930), [#1936](https://github.com/hydro-project/hydro/issues/1936), [#1937](https://github.com/hydro-project/hydro/issues/1937), [#1938](https://github.com/hydro-project/hydro/issues/1938), [#1939](https://github.com/hydro-project/hydro/issues/1939), [#1941](https://github.com/hydro-project/hydro/issues/1941), [#1944](https://github.com/hydro-project/hydro/issues/1944), [#1945](https://github.com/hydro-project/hydro/issues/1945), [#1947](https://github.com/hydro-project/hydro/issues/1947), [#1948](https://github.com/hydro-project/hydro/issues/1948), [#1951](https://github.com/hydro-project/hydro/issues/1951), [#1952](https://github.com/hydro-project/hydro/issues/1952), [#1956](https://github.com/hydro-project/hydro/issues/1956), [#1958](https://github.com/hydro-project/hydro/issues/1958), [#1959](https://github.com/hydro-project/hydro/issues/1959), [#1962](https://github.com/hydro-project/hydro/issues/1962)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1803](https://github.com/hydro-project/hydro/issues/1803)**
    - Use `async-ssh2-russh` (instead of `libssh2` bindings), fix #1463 ([`5ab815f`](https://github.com/hydro-project/hydro/commit/5ab815f3567d51e9bd114f90af8e837fe0732cd8))
 * **[#1843](https://github.com/hydro-project/hydro/issues/1843)**
    - Update pinned nightly to 2025-04-27, update span API usage ([`98baec7`](https://github.com/hydro-project/hydro/commit/98baec71a6f1d01d55a3c983fdbb7824c45305cd))
 * **[#1859](https://github.com/hydro-project/hydro/issues/1859)**
    - Decoupling analysis ([`99a8f1d`](https://github.com/hydro-project/hydro/commit/99a8f1dfdde087578f25c19e502ad13e1d98a394))
 * **[#1902](https://github.com/hydro-project/hydro/issues/1902)**
    - Assign cycles IDs that are globally unique (across clusters/processes) ([`863cb9e`](https://github.com/hydro-project/hydro/commit/863cb9eecd9e408fc8eb7c010415bd117d56c4ac))
 * **[#1905](https://github.com/hydro-project/hydro/issues/1905)**
    - Migrate tests from hydro_test_local ([`eaac1f4`](https://github.com/hydro-project/hydro/commit/eaac1f40d1050da65d2356597f36f37f0cf439ef))
 * **[#1907](https://github.com/hydro-project/hydro/issues/1907)**
    - Upgrade Stageleft to eliminate `__staged` compilation during development ([`b333b45`](https://github.com/hydro-project/hydro/commit/b333b45e0936bbe481d7fbc285790d942779c494))
 * **[#1909](https://github.com/hydro-project/hydro/issues/1909)**
    - Remove support for macro entrypoints ([`6e29285`](https://github.com/hydro-project/hydro/commit/6e29285d4c30901abce3352c2cf358f36f9173c6))
 * **[#1910](https://github.com/hydro-project/hydro/issues/1910)**
    - Add stream markers for tracking non-deterministic retries ([`45bd6e9`](https://github.com/hydro-project/hydro/commit/45bd6e9759410dcb747c9224758c82f9874378d2))
 * **[#1916](https://github.com/hydro-project/hydro/issues/1916)**
    - Add `*_idempotent` variations for (keyed) fold / reduce ([`6b0483d`](https://github.com/hydro-project/hydro/commit/6b0483d201f9d9871c9693765bd61284258a5bce))
 * **[#1930](https://github.com/hydro-project/hydro/issues/1930)**
    - Add `scan` operator ([`c4b9590`](https://github.com/hydro-project/hydro/commit/c4b9590552d7e05b352a0d4a215f1bbf274ccc61))
 * **[#1936](https://github.com/hydro-project/hydro/issues/1936)**
    - Graph viz for Hydro lang ([`4d15ff1`](https://github.com/hydro-project/hydro/commit/4d15ff19e604e5b3193abf51bc540ab1a435f7c5))
 * **[#1937](https://github.com/hydro-project/hydro/issues/1937)**
    - Capture stack traces for each IR node ([`d44b225`](https://github.com/hydro-project/hydro/commit/d44b225f697f8d7ccdf80b0d5dc2afdea1955324))
 * **[#1938](https://github.com/hydro-project/hydro/issues/1938)**
    - Allow running generated binaries with single-threaded Tokio runtime ([`bd1afdf`](https://github.com/hydro-project/hydro/commit/bd1afdff5fd7b8dc6d2c567cd2659542a84c6216))
 * **[#1939](https://github.com/hydro-project/hydro/issues/1939)**
    - Minimize Tokio feature flags ([`59041df`](https://github.com/hydro-project/hydro/commit/59041df58be2de0717b851cc1c3355479cd722f2))
 * **[#1941](https://github.com/hydro-project/hydro/issues/1941)**
    - Correctly enable staged-trybuild mode when cross-compiling ([`6699197`](https://github.com/hydro-project/hydro/commit/6699197d60a5efe8fd2fe951463649eca54ea9db))
 * **[#1944](https://github.com/hydro-project/hydro/issues/1944)**
    - Update `proc-macro-crate` ([`3d40d1a`](https://github.com/hydro-project/hydro/commit/3d40d1a65c41dca3893867fb567993a27491fa0c))
 * **[#1945](https://github.com/hydro-project/hydro/issues/1945)**
    - Make it easier to open trybuild-generated files with Rust Analyzer ([`4035cae`](https://github.com/hydro-project/hydro/commit/4035cae8536bf9f8421fee9a8ccca7eda7e846d6))
 * **[#1947](https://github.com/hydro-project/hydro/issues/1947)**
    - Don't snapshot-test backtraces ([`739b622`](https://github.com/hydro-project/hydro/commit/739b622f7a6ee3e58289863519419bd925c1e788))
 * **[#1948](https://github.com/hydro-project/hydro/issues/1948)**
    - Ensure that singletons and optionals always have cardinality 1 inside a tick ([`8858abd`](https://github.com/hydro-project/hydro/commit/8858abd784a3f20504264fcfeafde2fa5cc6a615))
 * **[#1951](https://github.com/hydro-project/hydro/issues/1951)**
    - Enable `viz` feature only in dev-dependencies ([`a3280d9`](https://github.com/hydro-project/hydro/commit/a3280d9b95576aaaf188779feb57ab0a28088a90))
 * **[#1952](https://github.com/hydro-project/hydro/issues/1952)**
    - Use partitioning analysis results to partition ([`173d9c0`](https://github.com/hydro-project/hydro/commit/173d9c0f1cc955f2a478195bcf3de0b055284008))
 * **[#1956](https://github.com/hydro-project/hydro/issues/1956)**
    - Rename `ExternalProcess` to `External` ([`49c1918`](https://github.com/hydro-project/hydro/commit/49c1918b9563f9ece6f2fc422a3b3c91d592e0cc))
 * **[#1958](https://github.com/hydro-project/hydro/issues/1958)**
    - Invert external sources and clean up locations in IR ([`fb016b0`](https://github.com/hydro-project/hydro/commit/fb016b017a42df95286cb2236ef987ea68be9ea7))
 * **[#1959](https://github.com/hydro-project/hydro/issues/1959)**
    - Emit appropriate `Persist` / `Unpersist` nodes for `scan` ([`1fc9f0d`](https://github.com/hydro-project/hydro/commit/1fc9f0dc938a50afc41cfe3fa8d6e34d0d1c947d))
 * **[#1962](https://github.com/hydro-project/hydro/issues/1962)**
    - Separate externals from other location kinds, clean up network operators ([`22a7d0d`](https://github.com/hydro-project/hydro/commit/22a7d0d9a057356a4a01ccdad6febc8aa229d325))
 * **Uncategorized**
    - Release dfir_lang v0.14.0, dfir_macro v0.14.0, hydro_deploy_integration v0.14.0, lattices_macro v0.5.10, variadics_macro v0.6.1, dfir_rs v0.14.0, hydro_deploy v0.14.0, hydro_lang v0.14.0, hydro_optimize v0.13.0, hydro_std v0.14.0, safety bump 6 crates ([`0683595`](https://github.com/hydro-project/hydro/commit/06835950c12884d661100c13f73ad23a98bfad9f))
</details>

## 0.13.2 (2025-04-17)

<csr-id-b7b040817eb3b5f601989592465a3829f93ef7b7/>
<csr-id-fe91f37ec43b20c1652560aebb15ac8d1596c0a4/>

### Chore

 - <csr-id-b7b040817eb3b5f601989592465a3829f93ef7b7/> update pinned nightly to 2025-04-05, clippy fixes
   Not going past 2024-04-05 due to rustc linker regression on windows:
   https://github.com/rust-lang/rust/issues/139821

### Style

 - <csr-id-fe91f37ec43b20c1652560aebb15ac8d1596c0a4/> use where clauses to increase readability
   A minor issue in the grand scheme of things, but I caught myself
   squinting when trying to tease apart different trait implementations
   specially when following the "implementations" popup navigation. In
   addition, some of the generic variable naming was not consistent and a
   bit confusing to the uninitiated. For example, `Order` was a generic
   variable, but `TotalOrder` and `NoOrder` were a type. Not a big deal
   when you already know the API, but just adds friction someone new to the
   project.
   
   Here are some of the items that guided the PR:
   * Single capital letter for generic variables following a loose
   standard: T for types, O for ordering, B for boundedness, L for
   location, P for process or external process, C for cluster, etc.
   * Use `where` clauses to keep the head of traits and functions lean and
   more easily comparable.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 5 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#1827](https://github.com/hydro-project/hydro/issues/1827), [#1840](https://github.com/hydro-project/hydro/issues/1840)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1827](https://github.com/hydro-project/hydro/issues/1827)**
    - Use where clauses to increase readability ([`fe91f37`](https://github.com/hydro-project/hydro/commit/fe91f37ec43b20c1652560aebb15ac8d1596c0a4))
 * **[#1840](https://github.com/hydro-project/hydro/issues/1840)**
    - Update pinned nightly to 2025-04-05, clippy fixes ([`b7b0408`](https://github.com/hydro-project/hydro/commit/b7b040817eb3b5f601989592465a3829f93ef7b7))
 * **Uncategorized**
    - Release hydro_lang v0.13.2 ([`056cf81`](https://github.com/hydro-project/hydro/commit/056cf818b8c4b6a382b9a4be07b640cf83bed651))
</details>

## 0.13.1 (2025-04-12)

### Bug Fixes

 - <csr-id-e41388edeaeffec689d46503bfdea467a53be797/> Enable file_lock feature in nightly builds
   This appears to be a regression from the commit that uses fs2 on stable
   builds.
   
   @mingweisamuel based on the CI error here it's because the feature was previously stabilized and has since been un-stabilized

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1839](https://github.com/hydro-project/hydro/issues/1839)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1839](https://github.com/hydro-project/hydro/issues/1839)**
    - Enable file_lock feature in nightly builds ([`e41388e`](https://github.com/hydro-project/hydro/commit/e41388edeaeffec689d46503bfdea467a53be797))
 * **Uncategorized**
    - Release hydro_lang v0.13.1 ([`2cb6ee8`](https://github.com/hydro-project/hydro/commit/2cb6ee81665eb24ce76959fc977287e7fc9d3cdf))
</details>

## 0.13.0 (2025-04-11)

<csr-id-79ee5041c2b73bca38a02463a073c082bc33a91c/>

### New Features

 - <csr-id-6d24901550fa873fc8b4b474f9f6316d98cf7aa8/> implement profiling for macOS and Windows using samply
 - <csr-id-bfac9f1c29eee56eae9d4d2524b417235a0e5849/> only serialize once in broadcast
 - <csr-id-5ac247ca2006bbb45c5511c78dc6d9028f7451da/> update Stageleft and reduce reliance on DFIR re-exports
 - <csr-id-e0c4abb02054fc3d5dc866286b18f3f2bcd2ad36/> update Stageleft to reduce viral dependencies
   Now that Stageleft handles quoted snippets that refer to local
   dependencies, we do not need to duplicate deps into downstream crates.
 - <csr-id-127df13b7d7fc12d5265f1d7517f3b66a774dc07/> add resolve_futures and resolve_futures_ordered APIs

### Bug Fixes

<csr-id-28c342ff26b14700aeec97e90a96b51a4ffe0165/>
<csr-id-f783e67c9d5cfa0b8127755bed0856b5fe457b26/>
<csr-id-53d2913bc66c67b45a2baa591740a5f06a087d57/>

 - <csr-id-2219e6b954649906d002e7d4cd3f228b3c9369fe/> correctly resolve `__staged` paths for the crate being launched
 - <csr-id-fbb5fab72c5a64a07653c9b6389186ad079703ec/> handle `-1` addresses from samply, fix `_counter()` rollover
   This fixes samply profiling on my "ancient" 2019 x86-64 macbook pro
   15.3.2 (24D81)
   
   This pull request aims to fix the handling of –1 address values from
   samply by updating tracing filenames and refactoring related error and
   type handling. Key changes include:
   - Better error messages when `dtrace` or `samply` are not instaled.

### Refactor

 - <csr-id-79ee5041c2b73bca38a02463a073c082bc33a91c/> use `fs2` file locking to fix stable build testing #1780

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

 - 13 commits contributed to the release.
 - 27 days passed between releases.
 - 12 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 12 unique issues were worked on: [#1741](https://github.com/hydro-project/hydro/issues/1741), [#1791](https://github.com/hydro-project/hydro/issues/1791), [#1796](https://github.com/hydro-project/hydro/issues/1796), [#1797](https://github.com/hydro-project/hydro/issues/1797), [#1798](https://github.com/hydro-project/hydro/issues/1798), [#1799](https://github.com/hydro-project/hydro/issues/1799), [#1802](https://github.com/hydro-project/hydro/issues/1802), [#1806](https://github.com/hydro-project/hydro/issues/1806), [#1812](https://github.com/hydro-project/hydro/issues/1812), [#1814](https://github.com/hydro-project/hydro/issues/1814), [#1816](https://github.com/hydro-project/hydro/issues/1816), [#1817](https://github.com/hydro-project/hydro/issues/1817)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1741](https://github.com/hydro-project/hydro/issues/1741)**
    - Add resolve_futures and resolve_futures_ordered APIs ([`127df13`](https://github.com/hydro-project/hydro/commit/127df13b7d7fc12d5265f1d7517f3b66a774dc07))
 * **[#1791](https://github.com/hydro-project/hydro/issues/1791)**
    - Update Stageleft to reduce viral dependencies ([`e0c4abb`](https://github.com/hydro-project/hydro/commit/e0c4abb02054fc3d5dc866286b18f3f2bcd2ad36))
 * **[#1796](https://github.com/hydro-project/hydro/issues/1796)**
    - Update Stageleft and reduce reliance on DFIR re-exports ([`5ac247c`](https://github.com/hydro-project/hydro/commit/5ac247ca2006bbb45c5511c78dc6d9028f7451da))
 * **[#1797](https://github.com/hydro-project/hydro/issues/1797)**
    - Don't pull in dfir_rs during the compilation stage ([`dfb7a1b`](https://github.com/hydro-project/hydro/commit/dfb7a1b5ad47f03822e9b7cae7dae81914b305e2))
 * **[#1798](https://github.com/hydro-project/hydro/issues/1798)**
    - Avoid Cargo.toml corruption when concurrently opened in a single process ([`53d2913`](https://github.com/hydro-project/hydro/commit/53d2913bc66c67b45a2baa591740a5f06a087d57))
 * **[#1799](https://github.com/hydro-project/hydro/issues/1799)**
    - Fix doctests without feature unification and improve atomic trybuild logic ([`f783e67`](https://github.com/hydro-project/hydro/commit/f783e67c9d5cfa0b8127755bed0856b5fe457b26))
 * **[#1802](https://github.com/hydro-project/hydro/issues/1802)**
    - Decrease size of IR enums to prevent stack overflow on windows, fix #1801 ([`28c342f`](https://github.com/hydro-project/hydro/commit/28c342ff26b14700aeec97e90a96b51a4ffe0165))
 * **[#1806](https://github.com/hydro-project/hydro/issues/1806)**
    - Only serialize once in broadcast ([`bfac9f1`](https://github.com/hydro-project/hydro/commit/bfac9f1c29eee56eae9d4d2524b417235a0e5849))
 * **[#1812](https://github.com/hydro-project/hydro/issues/1812)**
    - Implement profiling for macOS and Windows using samply ([`6d24901`](https://github.com/hydro-project/hydro/commit/6d24901550fa873fc8b4b474f9f6316d98cf7aa8))
 * **[#1814](https://github.com/hydro-project/hydro/issues/1814)**
    - Handle `-1` addresses from samply, fix `_counter()` rollover ([`fbb5fab`](https://github.com/hydro-project/hydro/commit/fbb5fab72c5a64a07653c9b6389186ad079703ec))
 * **[#1816](https://github.com/hydro-project/hydro/issues/1816)**
    - Use `fs2` file locking to fix stable build testing #1780 ([`79ee504`](https://github.com/hydro-project/hydro/commit/79ee5041c2b73bca38a02463a073c082bc33a91c))
 * **[#1817](https://github.com/hydro-project/hydro/issues/1817)**
    - Correctly resolve `__staged` paths for the crate being launched ([`2219e6b`](https://github.com/hydro-project/hydro/commit/2219e6b954649906d002e7d4cd3f228b3c9369fe))
 * **Uncategorized**
    - Release dfir_lang v0.13.0, dfir_datalog_core v0.13.0, dfir_datalog v0.13.0, dfir_macro v0.13.0, hydro_deploy_integration v0.13.0, dfir_rs v0.13.0, hydro_deploy v0.13.0, hydro_lang v0.13.0, hydro_std v0.13.0, hydro_cli v0.13.0, safety bump 8 crates ([`400fd8f`](https://github.com/hydro-project/hydro/commit/400fd8f2e8cada253f54980e7edce0631be70a82))
</details>

## 0.12.1 (2025-03-15)

<csr-id-38e6721be69f6a41aa47a01a9d06d56a01be1355/>
<csr-id-f14174e92875b3264ef811effd954ed76cb3c948/>
<csr-id-056ac62611319b7bd10a751d7e231423a1b8dc4e/>
<csr-id-7dd71d67da162d2e4f3043b271a52037a3c983c0/>

### Chore

 - <csr-id-38e6721be69f6a41aa47a01a9d06d56a01be1355/> remove stageleft from repo, fix #1764
   They grow up so fast 🥹
 - <csr-id-f14174e92875b3264ef811effd954ed76cb3c948/> update pinned nightly to 2025-03-10, clippy cleanups

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

### Bug Fixes

 - <csr-id-ed662013a29f0acaaf02fec8d90086ebfe868af8/> fix failing Clippy on stable due to unused variable
 - <csr-id-fdcf96a817825d1801b50055395e0284a0db2942/> use file locks to avoid clobbering Cargo configs
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

 - 10 commits contributed to the release.
 - 7 days passed between releases.
 - 8 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 8 unique issues were worked on: [#1768](https://github.com/hydro-project/hydro/issues/1768), [#1773](https://github.com/hydro-project/hydro/issues/1773), [#1774](https://github.com/hydro-project/hydro/issues/1774), [#1779](https://github.com/hydro-project/hydro/issues/1779), [#1780](https://github.com/hydro-project/hydro/issues/1780), [#1784](https://github.com/hydro-project/hydro/issues/1784), [#1785](https://github.com/hydro-project/hydro/issues/1785), [#1787](https://github.com/hydro-project/hydro/issues/1787)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1768](https://github.com/hydro-project/hydro/issues/1768)**
    - Update pinned nightly to 2025-03-10, clippy cleanups ([`f14174e`](https://github.com/hydro-project/hydro/commit/f14174e92875b3264ef811effd954ed76cb3c948))
 * **[#1773](https://github.com/hydro-project/hydro/issues/1773)**
    - Remove "hydroflow" for `hydro_deploy_integration`, `hydro_deploy::rust_crate`, fix #1712 ([`7dd71d6`](https://github.com/hydro-project/hydro/commit/7dd71d67da162d2e4f3043b271a52037a3c983c0))
 * **[#1774](https://github.com/hydro-project/hydro/issues/1774)**
    - Remove stageleft from repo, fix #1764 ([`38e6721`](https://github.com/hydro-project/hydro/commit/38e6721be69f6a41aa47a01a9d06d56a01be1355))
 * **[#1779](https://github.com/hydro-project/hydro/issues/1779)**
    - Fix codegen non-determinism that triggers rebuilds ([`530604c`](https://github.com/hydro-project/hydro/commit/530604ccce4e1825ea5a35caa696dec5e846fefb))
 * **[#1780](https://github.com/hydro-project/hydro/issues/1780)**
    - Use file locks to avoid clobbering Cargo configs ([`fdcf96a`](https://github.com/hydro-project/hydro/commit/fdcf96a817825d1801b50055395e0284a0db2942))
 * **[#1784](https://github.com/hydro-project/hydro/issues/1784)**
    - Fix failing Clippy on stable due to unused variable ([`ed66201`](https://github.com/hydro-project/hydro/commit/ed662013a29f0acaaf02fec8d90086ebfe868af8))
 * **[#1785](https://github.com/hydro-project/hydro/issues/1785)**
    - Cleanup old clippy lints, remove deprecated `relalg` crate ([`056ac62`](https://github.com/hydro-project/hydro/commit/056ac62611319b7bd10a751d7e231423a1b8dc4e))
 * **[#1787](https://github.com/hydro-project/hydro/issues/1787)**
    - Demote python deploy docs, fix docsrs configs, fix #1392, fix #1629 ([`b235a42`](https://github.com/hydro-project/hydro/commit/b235a42a3071e55da7b09bdc8bc710b18e0fe053))
 * **Uncategorized**
    - Release include_mdtests v0.0.0, dfir_rs v0.12.1, hydro_deploy v0.12.1, hydro_lang v0.12.1, hydro_std v0.12.1, hydro_cli v0.12.1 ([`faf0d3e`](https://github.com/hydro-project/hydro/commit/faf0d3ed9f172275f2e2f219c5ead1910c209a36))
    - Release dfir_lang v0.12.1, dfir_datalog_core v0.12.1, dfir_datalog v0.12.1, dfir_macro v0.12.1, hydro_deploy_integration v0.12.1, lattices v0.6.1, pusherator v0.0.12, dfir_rs v0.12.1, hydro_deploy v0.12.1, hydro_lang v0.12.1, hydro_std v0.12.1, hydro_cli v0.12.1 ([`23221b5`](https://github.com/hydro-project/hydro/commit/23221b53b30918707ddaa85529d04cd7919166b4))
</details>

## 0.12.0 (2025-03-08)

<csr-id-49a387d4a21f0763df8ec94de73fb953c9cd333a/>
<csr-id-c293cca6855695107e9cef5c5df99fb04a571934/>
<csr-id-b1514be40f44201866bf9420bd4fc7d2b35c5e9c/>
<csr-id-44fb2806cf2d165d86695910f4755e0944c11832/>
<csr-id-41e5bb93eb9c19a88167a63bce0ceb800f8f300d/>
<csr-id-80407a2f0fdaa8b8a81688d181166a0da8aa7b52/>
<csr-id-2fd6119afed850a0c50ecc69e5c4d8de61a2f4cb/>
<csr-id-524fa67232b54f5faeb797b43070f2f197c558dd/>
<csr-id-3966d9063dae52e65b077321e0bd1150f2b0c3f1/>
<csr-id-11df2e623f7db2cebd657ddbdd5aaac7649ddbb3/>
<csr-id-5cd0a9625822620dcc99b99356edfecbf0549497/>
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

### Chore (BREAKING)

 - <csr-id-3966d9063dae52e65b077321e0bd1150f2b0c3f1/> use DFIR name instead of Hydroflow in some places, fix #1644
   Fix partially #1712
   
   * Renames `WriteContextArgs.hydroflow` to `WriteContextArgs.df_ident`
   for DFIR operator codegen
   * Removes some dead code/files

### Test

 - <csr-id-11df2e623f7db2cebd657ddbdd5aaac7649ddbb3/> add example of doctest for `map`

### Refactor

 - <csr-id-5cd0a9625822620dcc99b99356edfecbf0549497/> enable lints, cleanups for Rust 2024 #1732

### Chore

 - <csr-id-ec3795a678d261a38085405b6e9bfea943dafefb/> upgrade to Rust 2024 edition
   - Updates `Cargo.toml` to use new shared workspace keys
   - Updates lint settings (in workspace `Cargo.toml`)
   - `rustfmt` has changed slightly, resulting in a big diff - there are no
   actual code changes
   - Adds a script to `rustfmt` the template src files

### Documentation

 - <csr-id-d2a1f38fabbfa1e3ea323635c935bda929a9c625/> document APIs for `Stream`
 - <csr-id-e43e7850121e4dfd1d2bfb2d76b1de3294ca2d48/> initial website docs on core Hydro concepts
 - <csr-id-73444373dabeedd7a03a8231952684fb01bdf895/> add initial Rustdoc for some Stream APIs
 - <csr-id-fc53a348b57eef09abb66d9249c2d84bf5207d0f/> document APIs for `Stream`
 - <csr-id-343a33f2ab98bef61252d49370fa2aa1980331d4/> initial website docs on core Hydro concepts
 - <csr-id-d7741d55a3ea9b172e962e7398f0414d0427c3f9/> add initial Rustdoc for some Stream APIs

### New Features

 - <csr-id-ce134fa0bae4085a9f81f9e556a553618e3652ab/> add APIs for getting DFIR without deploying
   Also modifies DFIR to elide Stageleft `type_hint`s when pretty printing
   an operator (e.g. for Mermaid). Also, because we stratify the graph
   before we can print it, adds basic support for printing stratified
   graphs as a surface string.
 - <csr-id-65138b4a61f2f7263c68d26692204e89f5cbad3c/> Insert counter to measure cardinality for specific operators
 - <csr-id-4d9c0f92964c4f49ebc5ca0b72e404a61b434383/> Partitioning
 - <csr-id-1ab64e76d560a59e29094ce4da0391713b1cc35e/> Decoupling
   Mechanism for decoupling at specified nodes
   
   ---------
 - <csr-id-733494ea2655cdc1460da7f902d999f0e3797411/> Link DFIR operators to Hydro operators in perf
 - <csr-id-a4adb08700fdc5fdbc949fc656e6cb309e7159a5/> provide in-memory access to perf tracing results
 - <csr-id-f95252cab4e9bfae2e5e9061ea7cbd3f3d3bb12a/> Compartmentalized Paxos & kv_replica bug fix
 - <csr-id-f3c459036976d87b20356a761bdea9c010ae680b/> Add ability to customize operator tag for stack tracing/flamegraphs
   Actually inserting Hydro-level operator IDs/names is TODO
   
   #1479
 - <csr-id-eee28d3a17ea542c69a2d7e535c38333f42d4398/> Add metadata field to HydroNode
 - <csr-id-e043cedf1999670bd5a5a93960748c89ca092f2b/> make `Stream::cloned` work with borrowed elements
 - <csr-id-316d700cf820fa448898ce8df95b5c6011c33cc0/> provide APIs for blanket-deploying locations
   This makes it easy to implement patterns like deploying everything to
   localhost.
 - <csr-id-6d77db9e52ece0b668587187c59f2862670db7cf/> send_partitioned operator and move decoupling
   Allows specifying a distribution policy (for deciding which partition to
   send each message to) before networking. Designed to be as easy as
   possible to inject (so the distribution policy function definition takes
   in the cluster ID, for example, even though it doesn't need to, because
   this way we can avoid project->map->join)
 - <csr-id-b968f5beccac2019b951cc5ab15891b48da01639/> allow developers to add their own re-export rewrites
 - <csr-id-2ad0be3a843431afdff21ad0119d3661b77666b8/> add APIs for getting DFIR without deploying
   Also modifies DFIR to elide Stageleft `type_hint`s when pretty printing
   an operator (e.g. for Mermaid). Also, because we stratify the graph
   before we can print it, adds basic support for printing stratified
   graphs as a surface string.
 - <csr-id-841886661a589cd94c5ea1b958246e3ec7e7135a/> Insert counter to measure cardinality for specific operators
 - <csr-id-dabb2df936d79ae314d9a6e479d7cc6b7c7ecf68/> Partitioning
 - <csr-id-980d62735b80034cd04f39ca29b7e8a7ce4e4731/> Decoupling
   Mechanism for decoupling at specified nodes
   
   ---------
 - <csr-id-1d48fde45a741e5eec59ce3b27a4a8f195198428/> Link DFIR operators to Hydro operators in perf
 - <csr-id-5ba6236555113dc019fe61adaf1d5aa34e07bb58/> provide in-memory access to perf tracing results
 - <csr-id-edd7ce1c3f16932b8cb597fb5c4baf31bc6895e0/> Compartmentalized Paxos & kv_replica bug fix
 - <csr-id-53b26fcfb5ac942680e830078f4411922f917f91/> Add ability to customize operator tag for stack tracing/flamegraphs
   Actually inserting Hydro-level operator IDs/names is TODO
   
   #1479
 - <csr-id-69831f9dc724ba7915b8ade8134839c42786ac76/> Add metadata field to HydroNode
 - <csr-id-1ca762d5fac0f56f2778c5d19357564050d09cf3/> make `Stream::cloned` work with borrowed elements
 - <csr-id-6ff7fc2c7c0f835c33b99f847cd3765221ed8f7e/> provide APIs for blanket-deploying locations
   This makes it easy to implement patterns like deploying everything to
   localhost.
 - <csr-id-ca291dd618fc4065c4e30097c5ea605226383cec/> send_partitioned operator and move decoupling
   Allows specifying a distribution policy (for deciding which partition to
   send each message to) before networking. Designed to be as easy as
   possible to inject (so the distribution policy function definition takes
   in the cluster ID, for example, even though it doesn't need to, because
   this way we can avoid project->map->join)
 - <csr-id-612e628dc56cbe31c3fb83a41142a7749b3e8492/> allow developers to add their own re-export rewrites

### Bug Fixes

 - <csr-id-0045599af763478c56b3599e7704fe24f26775e6/> update trybuild error message after Rust update
 - <csr-id-1f74d2f25a48963f372764d4cda2522f5a43faf9/> send_bincode_lifetime test
   Test expected stderr stemmed from importing the wrong library, not from
   the actual error regarding lifetimes
 - <csr-id-75eb323a612fd5d2609e464fe7690bc2b6a8457a/> use correct `__staged` path when rewriting `crate::` imports
   Previously, a rewrite would first turn `crate` into `crate::__staged`,
   and another would rewrite `crate::__staged` into `hydro_test::__staged`.
   The latter global rewrite is unnecessary because the stageleft logic
   already will use the full crate name when handling public types, so we
   drop it.
 - <csr-id-9fdb7709a94b09857d986272435e795f856435b3/> avoid crashes and miscompilations with unused locations
   Previously, if a location was not used in the graph, we would crash or
   use faulty IDs in the generated code. This also changes semantics
   slightly so that if there is a cluster with no assigned computation, it
   will be treated as an empty cluster at runtime (the list of cluster IDs
   will be an empty slice).
 - <csr-id-23164c12ea83eb337824dcc3946d5be0ac11fb5a/> update trybuild error message after Rust update
 - <csr-id-c624cd3f4030cd4bad3d6c6146d8c0b8d7687bc9/> send_bincode_lifetime test
   Test expected stderr stemmed from importing the wrong library, not from
   the actual error regarding lifetimes
 - <csr-id-48b275c1247f4f6fe7e6b63a5ae184c5d85b6fa1/> use correct `__staged` path when rewriting `crate::` imports
   Previously, a rewrite would first turn `crate` into `crate::__staged`,
   and another would rewrite `crate::__staged` into `hydro_test::__staged`.
   The latter global rewrite is unnecessary because the stageleft logic
   already will use the full crate name when handling public types, so we
   drop it.
 - <csr-id-24da1a92be3cfa0b5de1ca1e6a573b652b987416/> avoid crashes and miscompilations with unused locations
   Previously, if a location was not used in the graph, we would crash or
   use faulty IDs in the generated code. This also changes semantics
   slightly so that if there is a cluster with no assigned computation, it
   will be treated as an empty cluster at runtime (the list of cluster IDs
   will be an empty slice).

### Refactor

 - <csr-id-c293cca6855695107e9cef5c5df99fb04a571934/> enable lints, cleanups for Rust 2024 #1732

### Test

 - <csr-id-b1514be40f44201866bf9420bd4fc7d2b35c5e9c/> add example of doctest for `map`

### Chore (BREAKING)

 - <csr-id-44fb2806cf2d165d86695910f4755e0944c11832/> use DFIR name instead of Hydroflow in some places, fix #1644
   Fix partially #1712
   
   * Renames `WriteContextArgs.hydroflow` to `WriteContextArgs.df_ident`
   for DFIR operator codegen
   * Removes some dead code/files

### Documentation (BREAKING)

 - <csr-id-146d10a1347aa23e1e42500abef86201851bacfd/> rename `singleton_first_tick` to `optional_first_tick` and add example of doctest
 - <csr-id-89d9c15d5b000fb4ad246c3d511110ed25211ffa/> rename `singleton_first_tick` to `optional_first_tick` and add example of doctest

### Bug Fixes (BREAKING)

 - <csr-id-5c1c16cef25e058281e26c04160998072953ae95/> raise a panic if a user forgets to complete a cycle
 - <csr-id-c49a4913cfdae021404a86e5a4d0597aa4db9fbe/> reduce where `#[cfg(stageleft_runtime)]` needs to be used
   Simplifies the logic for generating the public clone of the code, which
   eliminates the need to sprinkle `#[cfg(stageleft_runtime)]` (renamed
   from `#[stageleft::runtime]`) everywhere. Also adds logic to pass
   through `cfg` attrs when re-exporting public types.
 - <csr-id-5cdbbd0e5950b6b464a5194478198a7e338fcd74/> raise a panic if a user forgets to complete a cycle
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

 - 30 commits contributed to the release.
 - 74 days passed between releases.
 - 29 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 29 unique issues were worked on: [#1632](https://github.com/hydro-project/hydro/issues/1632), [#1636](https://github.com/hydro-project/hydro/issues/1636), [#1638](https://github.com/hydro-project/hydro/issues/1638), [#1641](https://github.com/hydro-project/hydro/issues/1641), [#1650](https://github.com/hydro-project/hydro/issues/1650), [#1652](https://github.com/hydro-project/hydro/issues/1652), [#1657](https://github.com/hydro-project/hydro/issues/1657), [#1659](https://github.com/hydro-project/hydro/issues/1659), [#1668](https://github.com/hydro-project/hydro/issues/1668), [#1676](https://github.com/hydro-project/hydro/issues/1676), [#1678](https://github.com/hydro-project/hydro/issues/1678), [#1681](https://github.com/hydro-project/hydro/issues/1681), [#1689](https://github.com/hydro-project/hydro/issues/1689), [#1695](https://github.com/hydro-project/hydro/issues/1695), [#1701](https://github.com/hydro-project/hydro/issues/1701), [#1702](https://github.com/hydro-project/hydro/issues/1702), [#1706](https://github.com/hydro-project/hydro/issues/1706), [#1707](https://github.com/hydro-project/hydro/issues/1707), [#1713](https://github.com/hydro-project/hydro/issues/1713), [#1719](https://github.com/hydro-project/hydro/issues/1719), [#1721](https://github.com/hydro-project/hydro/issues/1721), [#1723](https://github.com/hydro-project/hydro/issues/1723), [#1724](https://github.com/hydro-project/hydro/issues/1724), [#1728](https://github.com/hydro-project/hydro/issues/1728), [#1734](https://github.com/hydro-project/hydro/issues/1734), [#1735](https://github.com/hydro-project/hydro/issues/1735), [#1737](https://github.com/hydro-project/hydro/issues/1737), [#1747](https://github.com/hydro-project/hydro/issues/1747), [#1758](https://github.com/hydro-project/hydro/issues/1758)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1632](https://github.com/hydro-project/hydro/issues/1632)**
    - Add metadata field to HydroNode ([`69831f9`](https://github.com/hydro-project/hydro/commit/69831f9dc724ba7915b8ade8134839c42786ac76))
 * **[#1636](https://github.com/hydro-project/hydro/issues/1636)**
    - Add example of doctest for `map` ([`11df2e6`](https://github.com/hydro-project/hydro/commit/11df2e623f7db2cebd657ddbdd5aaac7649ddbb3))
 * **[#1638](https://github.com/hydro-project/hydro/issues/1638)**
    - Allow developers to add their own re-export rewrites ([`612e628`](https://github.com/hydro-project/hydro/commit/612e628dc56cbe31c3fb83a41142a7749b3e8492))
 * **[#1641](https://github.com/hydro-project/hydro/issues/1641)**
    - Avoid crashes and miscompilations with unused locations ([`24da1a9`](https://github.com/hydro-project/hydro/commit/24da1a92be3cfa0b5de1ca1e6a573b652b987416))
 * **[#1650](https://github.com/hydro-project/hydro/issues/1650)**
    - Add initial Rustdoc for some Stream APIs ([`d7741d5`](https://github.com/hydro-project/hydro/commit/d7741d55a3ea9b172e962e7398f0414d0427c3f9))
 * **[#1652](https://github.com/hydro-project/hydro/issues/1652)**
    - Send_partitioned operator and move decoupling ([`ca291dd`](https://github.com/hydro-project/hydro/commit/ca291dd618fc4065c4e30097c5ea605226383cec))
 * **[#1657](https://github.com/hydro-project/hydro/issues/1657)**
    - Use correct `__staged` path when rewriting `crate::` imports ([`48b275c`](https://github.com/hydro-project/hydro/commit/48b275c1247f4f6fe7e6b63a5ae184c5d85b6fa1))
 * **[#1659](https://github.com/hydro-project/hydro/issues/1659)**
    - Rename `singleton_first_tick` to `optional_first_tick` and add example of doctest ([`89d9c15`](https://github.com/hydro-project/hydro/commit/89d9c15d5b000fb4ad246c3d511110ed25211ffa))
 * **[#1668](https://github.com/hydro-project/hydro/issues/1668)**
    - Initial website docs on core Hydro concepts ([`343a33f`](https://github.com/hydro-project/hydro/commit/343a33f2ab98bef61252d49370fa2aa1980331d4))
 * **[#1676](https://github.com/hydro-project/hydro/issues/1676)**
    - Provide APIs for blanket-deploying locations ([`6ff7fc2`](https://github.com/hydro-project/hydro/commit/6ff7fc2c7c0f835c33b99f847cd3765221ed8f7e))
 * **[#1678](https://github.com/hydro-project/hydro/issues/1678)**
    - Make `Stream::cloned` work with borrowed elements ([`1ca762d`](https://github.com/hydro-project/hydro/commit/1ca762d5fac0f56f2778c5d19357564050d09cf3))
 * **[#1681](https://github.com/hydro-project/hydro/issues/1681)**
    - Rename timestamp to atomic and provide batching shortcuts ([`524fa67`](https://github.com/hydro-project/hydro/commit/524fa67232b54f5faeb797b43070f2f197c558dd))
 * **[#1689](https://github.com/hydro-project/hydro/issues/1689)**
    - Document APIs for `Stream` ([`fc53a34`](https://github.com/hydro-project/hydro/commit/fc53a348b57eef09abb66d9249c2d84bf5207d0f))
 * **[#1695](https://github.com/hydro-project/hydro/issues/1695)**
    - Rename `_interleaved` to `_anonymous` ([`2fd6119`](https://github.com/hydro-project/hydro/commit/2fd6119afed850a0c50ecc69e5c4d8de61a2f4cb))
 * **[#1701](https://github.com/hydro-project/hydro/issues/1701)**
    - Compartmentalized Paxos & kv_replica bug fix ([`edd7ce1`](https://github.com/hydro-project/hydro/commit/edd7ce1c3f16932b8cb597fb5c4baf31bc6895e0))
 * **[#1702](https://github.com/hydro-project/hydro/issues/1702)**
    - Add ability to customize operator tag for stack tracing/flamegraphs ([`53b26fc`](https://github.com/hydro-project/hydro/commit/53b26fcfb5ac942680e830078f4411922f917f91))
 * **[#1706](https://github.com/hydro-project/hydro/issues/1706)**
    - Send_bincode_lifetime test ([`c624cd3`](https://github.com/hydro-project/hydro/commit/c624cd3f4030cd4bad3d6c6146d8c0b8d7687bc9))
 * **[#1707](https://github.com/hydro-project/hydro/issues/1707)**
    - Update trybuild error message after Rust update ([`23164c1`](https://github.com/hydro-project/hydro/commit/23164c12ea83eb337824dcc3946d5be0ac11fb5a))
 * **[#1713](https://github.com/hydro-project/hydro/issues/1713)**
    - Use DFIR name instead of Hydroflow in some places, fix #1644 ([`3966d90`](https://github.com/hydro-project/hydro/commit/3966d9063dae52e65b077321e0bd1150f2b0c3f1))
 * **[#1719](https://github.com/hydro-project/hydro/issues/1719)**
    - Provide in-memory access to perf tracing results ([`5ba6236`](https://github.com/hydro-project/hydro/commit/5ba6236555113dc019fe61adaf1d5aa34e07bb58))
 * **[#1721](https://github.com/hydro-project/hydro/issues/1721)**
    - Reduce where `#[cfg(stageleft_runtime)]` needs to be used ([`a7e22cd`](https://github.com/hydro-project/hydro/commit/a7e22cdd312b8483163aa89751833e1657703b8d))
 * **[#1723](https://github.com/hydro-project/hydro/issues/1723)**
    - Link DFIR operators to Hydro operators in perf ([`1d48fde`](https://github.com/hydro-project/hydro/commit/1d48fde45a741e5eec59ce3b27a4a8f195198428))
 * **[#1724](https://github.com/hydro-project/hydro/issues/1724)**
    - Decoupling ([`980d627`](https://github.com/hydro-project/hydro/commit/980d62735b80034cd04f39ca29b7e8a7ce4e4731))
 * **[#1728](https://github.com/hydro-project/hydro/issues/1728)**
    - Partitioning ([`dabb2df`](https://github.com/hydro-project/hydro/commit/dabb2df936d79ae314d9a6e479d7cc6b7c7ecf68))
 * **[#1734](https://github.com/hydro-project/hydro/issues/1734)**
    - Raise a panic if a user forgets to complete a cycle ([`5cdbbd0`](https://github.com/hydro-project/hydro/commit/5cdbbd0e5950b6b464a5194478198a7e338fcd74))
 * **[#1735](https://github.com/hydro-project/hydro/issues/1735)**
    - Insert counter to measure cardinality for specific operators ([`8418866`](https://github.com/hydro-project/hydro/commit/841886661a589cd94c5ea1b958246e3ec7e7135a))
 * **[#1737](https://github.com/hydro-project/hydro/issues/1737)**
    - Enable lints, cleanups for Rust 2024 #1732 ([`5cd0a96`](https://github.com/hydro-project/hydro/commit/5cd0a9625822620dcc99b99356edfecbf0549497))
 * **[#1747](https://github.com/hydro-project/hydro/issues/1747)**
    - Upgrade to Rust 2024 edition ([`ec3795a`](https://github.com/hydro-project/hydro/commit/ec3795a678d261a38085405b6e9bfea943dafefb))
 * **[#1758](https://github.com/hydro-project/hydro/issues/1758)**
    - Add APIs for getting DFIR without deploying ([`2ad0be3`](https://github.com/hydro-project/hydro/commit/2ad0be3a843431afdff21ad0119d3661b77666b8))
 * **Uncategorized**
    - Release dfir_lang v0.12.0, dfir_datalog_core v0.12.0, dfir_datalog v0.12.0, dfir_macro v0.12.0, hydroflow_deploy_integration v0.12.0, lattices_macro v0.5.9, variadics v0.0.9, variadics_macro v0.6.0, lattices v0.6.0, multiplatform_test v0.5.0, pusherator v0.0.11, dfir_rs v0.12.0, hydro_deploy v0.12.0, stageleft_macro v0.6.0, stageleft v0.7.0, stageleft_tool v0.6.0, hydro_lang v0.12.0, hydro_std v0.12.0, hydro_cli v0.12.0, safety bump 10 crates ([`973c925`](https://github.com/hydro-project/hydro/commit/973c925e87ed78344494581bd7ce1bbb4186a2f3))
</details>

## 0.11.0 (2024-12-23)

<csr-id-0dc709ed5a53c723f47fa1d10063e57bb50a63c8/>
<csr-id-9ea5f061ee0f116caf8fc4ea99b62a9c7691be2a/>
<csr-id-ec55910f5a41d4f08059b5feda4b96fbd058c959/>
<csr-id-251b1039c71d45d3f86123dba1926026ded80824/>
<csr-id-78f6a3299fff822abaea50841800a07f0e2ae128/>
<csr-id-9f3c8c468c58b7ec50d1c104fc24db0920d13c0d/>
<csr-id-03b3a349013a71b324276bca5329c33d400a73ff/>
<csr-id-accb13cad718c99d350e4bafe82e0ca38bf94c62/>
<csr-id-3291c07b37c9f9031837a2a32953e8f8854ec298/>
<csr-id-162e49cf8a8cf944cded7f775d6f78afe4a89837/>
<csr-id-a6f60c92ae7168eb86eb311ca7b7afb10025c7de/>
<csr-id-2a22d50285ae1be1a5f888d5d15321cc1bb13c82/>
<csr-id-5e58e346612a094c7e637919c84ab1e78b59be27/>
<csr-id-54f461acfce091276b8ce7574c0690e6d648546d/>

### Chore

 - <csr-id-0dc709ed5a53c723f47fa1d10063e57bb50a63c8/> use same hashing library everywhere

### Chore

 - <csr-id-a6f60c92ae7168eb86eb311ca7b7afb10025c7de/> bump versions manually for renamed crates, per `RELEASING.md`
 - <csr-id-2a22d50285ae1be1a5f888d5d15321cc1bb13c82/> cleanup snapshots
 - <csr-id-5e58e346612a094c7e637919c84ab1e78b59be27/> Rename Hydroflow -> DFIR
   Work In Progress:
   - [x] hydroflow_macro
   - [x] hydroflow_datalog_core
   - [x] hydroflow_datalog
   - [x] hydroflow_lang
   - [x] hydroflow
 - <csr-id-54f461acfce091276b8ce7574c0690e6d648546d/> Rename HydroflowPlus to Hydro

### Documentation

 - <csr-id-28cd220c68e3660d9ebade113949a2346720cd04/> add `repository` field to `Cargo.toml`s, fix #1452
   #1452 
   
   Will trigger new releases of the following:
   `unchanged = 'hydroflow_deploy_integration', 'variadics',
   'variadics_macro', 'pusherator'`
   
   (All other crates already have changes, so would be released anyway)
 - <csr-id-e1a08e5d165fbc80da2ae695e507078a97a9031f/> update `CHANGELOG.md`s for big rename
   Generated before rename per `RELEASING.md` instructions.
 - <csr-id-6ab625273d822812e83a333e928c3dea1c3c9ccb/> cleanups for the rename, fixing links
 - <csr-id-204bd117ca3a8845b4986539efb91a0c612dfa05/> add `repository` field to `Cargo.toml`s, fix #1452
   #1452 
   
   Will trigger new releases of the following:
   `unchanged = 'hydroflow_deploy_integration', 'variadics',
   'variadics_macro', 'pusherator'`
   
   (All other crates already have changes, so would be released anyway)
 - <csr-id-27c40e2ca5a822f6ebd31c7f01213aa6d407418a/> update `CHANGELOG.md`s for big rename
   Generated before rename per `RELEASING.md` instructions.
 - <csr-id-987f7ad8668d9740ceea577a595035228898d530/> cleanups for the rename, fixing links

### Chore

 - <csr-id-03b3a349013a71b324276bca5329c33d400a73ff/> bump versions manually for renamed crates, per `RELEASING.md`
 - <csr-id-accb13cad718c99d350e4bafe82e0ca38bf94c62/> cleanup snapshots
 - <csr-id-3291c07b37c9f9031837a2a32953e8f8854ec298/> Rename Hydroflow -> DFIR
   Work In Progress:
   - [x] hydroflow_macro
   - [x] hydroflow_datalog_core
   - [x] hydroflow_datalog
   - [x] hydroflow_lang
   - [x] hydroflow
 - <csr-id-162e49cf8a8cf944cded7f775d6f78afe4a89837/> Rename HydroflowPlus to Hydro

### New Features

 - <csr-id-22de01f5f566cd8cf7cb3bd31ae8bed99bf1e9ab/> add `round_robin` helpers for networking
   Also fixes compiler crashes when using `.enumerate()` on an un-batched
   stream.

### Bug Fixes

 - <csr-id-032cde6492390733e3e82d7126cc84989bf31853/> drop nightly feature flag from trybuild codegen
 - <csr-id-f6989baf12631cf43a814123e274466740c2f159/> restrict lifetime parameters to be actually invariant
   Our lifetimes were accidentally made covariant when the lifetime `'a`
   was removed from the process/cluster tag type. This fixes that typing
   hole, and also loosens some restrictions on the lifetime of deploy
   environments.

### Refactor

 - <csr-id-9ea5f061ee0f116caf8fc4ea99b62a9c7691be2a/> use `match_box` macro to compile on stable
 - <csr-id-ec55910f5a41d4f08059b5feda4b96fbd058c959/> generalize quorum logic

### New Features (BREAKING)

 - <csr-id-c65b4c49f3a3f2dcc7c8c28d1871b88c6c954822/> minimize dependencies pulled into `trybuild` builds
   We don't need `hydroflow_lang` in the runtime builds, do some feature
   flagging to avoid it.
 - <csr-id-939389953875bf5f94ea84503a7a35efd7342282/> mark non-deterministic operators as unsafe and introduce timestamped streams
   Big PR.
   
   First big change is we introduce a `Timestamped` location. This is a bit
   of a hybrid between top-level locations and `Tick` locations. The idea
   is that you choose where timestamps are generated, and then have a
   guarantee that everything after that will be atomically computed (useful
   for making sure we add payloads to the log before ack-ing).
   
   The contract is that an operator or module that takes a `Timestamped`
   input must still be deterministic regardless of the stamps on messages
   (which are hidden unless you `tick_batch`). But unlike a top-level
   stream (which has the same constraints), you have the atomicity
   guarantee. Right now the guarantee is trivial since we have one global
   tick for everything. But in the future when we want to apply
   @davidchuyaya's optimizations this will be helpful to know when there
   are causal dependencies on when data can be sent to others.
   
   Second change is we mark every non-deterministic operator (modulo
   explicit annotations such as `NoOrder`) with Rust's `unsafe` keyword.
   This makes it super clear where non-determinism is taking place.
   
   I've used this to put `unsafe` blocks throughout our example code and
   add `SAFETY` annotations that argue why the non-determinism is safe (or
   point out that we've explicitly documented / expect non-determinism). I
   also added `#![warn(unsafe_op_in_unsafe_fn)]` to the examples and the
   template, since this forces good hygiene of annotating sources of
   non-determinism even inside a module that is intentionally
   non-deterministic.
   
   Paxos changes are mostly refactors, and I verified that the performance
   is the same as before.
 - <csr-id-f96676d2d53d824c5c168e3db69722c2e9956fe2/> allow runtime context to be referenced as a global constant
 - <csr-id-a93a5e59e1681d325b3433193bb86254d23bdc77/> allow cluster self ID to be referenced as a global constant
   This eliminates the need to store `cluster.self_id()` in a local
   variable first, instead you can directly reference `CLUSTER_SELF_ID`.
 - <csr-id-4c5ca31486a9cfcbcba3af03aa30084a8b8dfcce/> introduce an unordered variant of streams to strengthen determinism guarantees
   Previously, sending data from a `Cluster` would return a stream assumed
   to have deterministic contents **and** ordering, which is false. This
   introduces another type parameter for `Stream` which tracks whether
   element ordering is expected to be deterministic, and restricts
   operators such as `fold` and `reduce` to commutative aggregations
   accordingly.

### Bug Fixes (BREAKING)

 - <csr-id-eb1ad3a54705efb06ee3f0647deaa9a52731ae6e/> rename `union` to `chain` and restrict LHS to be bounded
   Returning a `Stream` from `union` on unbounded streams was unsound,
   since the order of outputs is not deterministic.

### Refactor (BREAKING)

 - <csr-id-251b1039c71d45d3f86123dba1926026ded80824/> use `cfg(nightly)` instead of feature, remove `-Z` flag, use `Diagnostic::try_emit`
   Previous PR (#1587) website build did not work because `panic = "abort"`
   is set on wasm, leading to aborts for `proc_macro2::Span::unwrap()`
   calls.
   
   All tests except trybuild seem to pass on stable, WIP #1587 next
 - <csr-id-78f6a3299fff822abaea50841800a07f0e2ae128/> further reduce namespace pollution
 - <csr-id-9f3c8c468c58b7ec50d1c104fc24db0920d13c0d/> don't re-export all of `hydroflow`
   Reduces namespace pollution when wildcard-importing `hydroflow_plus`.

### `hydroflow_plus` Commit Statistics

<csr-read-only-do-not-edit/>

 - 15 commits contributed to the release.
 - 38 days passed between releases.
 - 15 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 15 unique issues were worked on: [#1559](https://github.com/hydro-project/hydroflow/issues/1559), [#1562](https://github.com/hydro-project/hydroflow/issues/1562), [#1565](https://github.com/hydro-project/hydroflow/issues/1565), [#1566](https://github.com/hydro-project/hydroflow/issues/1566), [#1568](https://github.com/hydro-project/hydroflow/issues/1568), [#1574](https://github.com/hydro-project/hydroflow/issues/1574), [#1575](https://github.com/hydro-project/hydroflow/issues/1575), [#1583](https://github.com/hydro-project/hydroflow/issues/1583), [#1584](https://github.com/hydro-project/hydroflow/issues/1584), [#1589](https://github.com/hydro-project/hydroflow/issues/1589), [#1590](https://github.com/hydro-project/hydroflow/issues/1590), [#1597](https://github.com/hydro-project/hydroflow/issues/1597), [#1598](https://github.com/hydro-project/hydroflow/issues/1598), [#1606](https://github.com/hydro-project/hydroflow/issues/1606), [#1611](https://github.com/hydro-project/hydroflow/issues/1611)

### `hydroflow_plus` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1559](https://github.com/hydro-project/hydroflow/issues/1559)**
    - Restrict lifetime parameters to be actually invariant ([`f6989ba`](https://github.com/hydro-project/hydroflow/commit/f6989baf12631cf43a814123e274466740c2f159))
 * **[#1562](https://github.com/hydro-project/hydroflow/issues/1562)**
    - Don't re-export all of `hydroflow` ([`9f3c8c4`](https://github.com/hydro-project/hydroflow/commit/9f3c8c468c58b7ec50d1c104fc24db0920d13c0d))
 * **[#1565](https://github.com/hydro-project/hydroflow/issues/1565)**
    - Rename `union` to `chain` and restrict LHS to be bounded ([`eb1ad3a`](https://github.com/hydro-project/hydroflow/commit/eb1ad3a54705efb06ee3f0647deaa9a52731ae6e))
 * **[#1566](https://github.com/hydro-project/hydroflow/issues/1566)**
    - Add `round_robin` helpers for networking ([`22de01f`](https://github.com/hydro-project/hydroflow/commit/22de01f5f566cd8cf7cb3bd31ae8bed99bf1e9ab))
 * **[#1568](https://github.com/hydro-project/hydroflow/issues/1568)**
    - Introduce an unordered variant of streams to strengthen determinism guarantees ([`4c5ca31`](https://github.com/hydro-project/hydroflow/commit/4c5ca31486a9cfcbcba3af03aa30084a8b8dfcce))
 * **[#1574](https://github.com/hydro-project/hydroflow/issues/1574)**
    - Allow cluster self ID to be referenced as a global constant ([`a93a5e5`](https://github.com/hydro-project/hydroflow/commit/a93a5e59e1681d325b3433193bb86254d23bdc77))
 * **[#1575](https://github.com/hydro-project/hydroflow/issues/1575)**
    - Allow runtime context to be referenced as a global constant ([`f96676d`](https://github.com/hydro-project/hydroflow/commit/f96676d2d53d824c5c168e3db69722c2e9956fe2))
 * **[#1583](https://github.com/hydro-project/hydroflow/issues/1583)**
    - Generalize quorum logic ([`ec55910`](https://github.com/hydro-project/hydroflow/commit/ec55910f5a41d4f08059b5feda4b96fbd058c959))
 * **[#1584](https://github.com/hydro-project/hydroflow/issues/1584)**
    - Mark non-deterministic operators as unsafe and introduce timestamped streams ([`9393899`](https://github.com/hydro-project/hydroflow/commit/939389953875bf5f94ea84503a7a35efd7342282))
 * **[#1589](https://github.com/hydro-project/hydroflow/issues/1589)**
    - Further reduce namespace pollution ([`78f6a32`](https://github.com/hydro-project/hydroflow/commit/78f6a3299fff822abaea50841800a07f0e2ae128))
 * **[#1590](https://github.com/hydro-project/hydroflow/issues/1590)**
    - Use same hashing library everywhere ([`0dc709e`](https://github.com/hydro-project/hydroflow/commit/0dc709ed5a53c723f47fa1d10063e57bb50a63c8))
 * **[#1597](https://github.com/hydro-project/hydroflow/issues/1597)**
    - Use `match_box` macro to compile on stable ([`9ea5f06`](https://github.com/hydro-project/hydroflow/commit/9ea5f061ee0f116caf8fc4ea99b62a9c7691be2a))
 * **[#1598](https://github.com/hydro-project/hydroflow/issues/1598)**
    - Drop nightly feature flag from trybuild codegen ([`032cde6`](https://github.com/hydro-project/hydroflow/commit/032cde6492390733e3e82d7126cc84989bf31853))
 * **[#1606](https://github.com/hydro-project/hydroflow/issues/1606)**
    - Use `cfg(nightly)` instead of feature, remove `-Z` flag, use `Diagnostic::try_emit` ([`251b103`](https://github.com/hydro-project/hydroflow/commit/251b1039c71d45d3f86123dba1926026ded80824))
 * **[#1611](https://github.com/hydro-project/hydroflow/issues/1611)**
    - Minimize dependencies pulled into `trybuild` builds ([`c65b4c4`](https://github.com/hydro-project/hydroflow/commit/c65b4c49f3a3f2dcc7c8c28d1871b88c6c954822))
</details>

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 9 commits contributed to the release.
 - 7 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 6 unique issues were worked on: [#1501](https://github.com/hydro-project/hydro/issues/1501), [#1617](https://github.com/hydro-project/hydro/issues/1617), [#1620](https://github.com/hydro-project/hydro/issues/1620), [#1623](https://github.com/hydro-project/hydro/issues/1623), [#1624](https://github.com/hydro-project/hydro/issues/1624), [#1627](https://github.com/hydro-project/hydro/issues/1627)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1501](https://github.com/hydro-project/hydro/issues/1501)**
    - Add `repository` field to `Cargo.toml`s, fix #1452 ([`204bd11`](https://github.com/hydro-project/hydro/commit/204bd117ca3a8845b4986539efb91a0c612dfa05))
 * **[#1617](https://github.com/hydro-project/hydro/issues/1617)**
    - Rename HydroflowPlus to Hydro ([`54f461a`](https://github.com/hydro-project/hydro/commit/54f461acfce091276b8ce7574c0690e6d648546d))
 * **[#1620](https://github.com/hydro-project/hydro/issues/1620)**
    - Rename Hydroflow -> DFIR ([`5e58e34`](https://github.com/hydro-project/hydro/commit/5e58e346612a094c7e637919c84ab1e78b59be27))
 * **[#1623](https://github.com/hydro-project/hydro/issues/1623)**
    - Cleanup snapshots ([`2a22d50`](https://github.com/hydro-project/hydro/commit/2a22d50285ae1be1a5f888d5d15321cc1bb13c82))
 * **[#1624](https://github.com/hydro-project/hydro/issues/1624)**
    - Cleanups for the rename, fixing links ([`987f7ad`](https://github.com/hydro-project/hydro/commit/987f7ad8668d9740ceea577a595035228898d530))
 * **[#1627](https://github.com/hydro-project/hydro/issues/1627)**
    - Bump versions manually for renamed crates, per `RELEASING.md` ([`a6f60c9`](https://github.com/hydro-project/hydro/commit/a6f60c92ae7168eb86eb311ca7b7afb10025c7de))
 * **Uncategorized**
    - Release stageleft_macro v0.5.0, stageleft v0.6.0, stageleft_tool v0.5.0, hydro_lang v0.11.0, hydro_std v0.11.0, hydro_cli v0.11.0 ([`7633c38`](https://github.com/hydro-project/hydro/commit/7633c38c4a56acf7e5b3b6f2a72ccc1d6e6eeba1))
    - Release dfir_lang v0.11.0, dfir_datalog_core v0.11.0, dfir_datalog v0.11.0, dfir_macro v0.11.0, hydroflow_deploy_integration v0.11.0, lattices_macro v0.5.8, variadics v0.0.8, variadics_macro v0.5.6, lattices v0.5.9, multiplatform_test v0.4.0, pusherator v0.0.10, dfir_rs v0.11.0, hydro_deploy v0.11.0, stageleft_macro v0.5.0, stageleft v0.6.0, stageleft_tool v0.5.0, hydro_lang v0.11.0, hydro_std v0.11.0, hydro_cli v0.11.0, safety bump 6 crates ([`361b443`](https://github.com/hydro-project/hydro/commit/361b4439ef9c781860f18d511668ab463a8c5203))
    - Update `CHANGELOG.md`s for big rename ([`27c40e2`](https://github.com/hydro-project/hydro/commit/27c40e2ca5a822f6ebd31c7f01213aa6d407418a))
</details>

## v0.10.0 (2024-11-08)

<csr-id-d5677604e93c07a5392f4229af94a0b736eca382/>
<csr-id-a1b45203178165683cb4b5ae611c598cc9c14853/>
<csr-id-e9d05bf11a0e85da8ed1a0fe00be7769298308c2/>
<csr-id-9f744052dd4ac744f5a1baa4e0cb9253adaeba1b/>
<csr-id-5b819a2dc6c507222a3e22d71efcde8b43cebad5/>
<csr-id-244207c2acd2243ece6e787d54eadacf06e9e8bb/>
<csr-id-bf9dcd5a923dd4b5efa337a9127086e5609a1722/>
<csr-id-d9634f242a97c06bdb53011bf3d75256425a1598/>
<csr-id-534fe974101e38ecb847cd759dbaf503ff97f822/>
<csr-id-0a5abab3dac224c9591bcdd837d07c6e5c2773c6/>
<csr-id-38b17cd977fb6c00ddc37e7a5b30e45dba17329e/>
<csr-id-8b7b1c60fd33b78f9a4b0873bbbd150260ae2ad5/>
<csr-id-1b18b358c87caa37a6519612131c8674653a2407/>
<csr-id-c752affc2ee2c5d82d19dd992f6a89b7070b8773/>
<csr-id-47cb703e771f7d1c451ceb9d185ada96410949da/>
<csr-id-0bd3a2d2230cbef24210f71a3ea83d82d1cc7244/>
<csr-id-9107841700db0ae72de6269ab6f132be0ae51cd9/>
<csr-id-919099ea3a414560b473ec89b993eeb26dfa2579/>
<csr-id-5657563c989566e7c7b69dcb395e40b024c83c6c/>
<csr-id-e5b456bdafcb80aae6039e4c90a2e60098e499bf/>
<csr-id-30c4f708faff7875ab42e551dd4bccbe231dfdad/>
<csr-id-8ad997b2dfd23bb09f7d361d763d6b5e78f406d6/>

### Chore

 - <csr-id-d5677604e93c07a5392f4229af94a0b736eca382/> update pinned rust version, clippy lints, remove some dead code

### Refactor

 - <csr-id-dff2a40669736014349cf12744d6a057a7992e11/> start splitting out leader election into a separate module
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1485).
   * #1493
   * #1492
   * #1489
   * #1488
   * #1487
   * #1486
   * __->__ #1485

<csr-id-dff2a40669736014349cf12744d6a057a7992e11/> start splitting out leader election into a separate module
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1485).
   * #1493
   * #1492
   * #1489
   * #1488
   * #1487
   * #1486
   * __->__ #1485

<csr-id-dff2a40669736014349cf12744d6a057a7992e11/> start splitting out leader election into a separate module
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1485).
   * #1493
   * #1492
   * #1489
   * #1488
   * #1487
   * #1486
   * __->__ #1485

<csr-id-dff2a40669736014349cf12744d6a057a7992e11/> start splitting out leader election into a separate module
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1485).
   * #1493
   * #1492
   * #1489
   * #1488
   * #1487
   * #1486
   * __->__ #1485

<csr-id-dff2a40669736014349cf12744d6a057a7992e11/> start splitting out leader election into a separate module
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1485).
   * #1493
   * #1492
   * #1489
   * #1488
   * #1487
   * #1486
   * __->__ #1485

<csr-id-dff2a40669736014349cf12744d6a057a7992e11/> start splitting out leader election into a separate module
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1485).
   * #1493
   * #1492
   * #1489
   * #1488
   * #1487
   * #1486
   * __->__ #1485

<csr-id-dff2a40669736014349cf12744d6a057a7992e11/> start splitting out leader election into a separate module
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1485).
   * #1493
   * #1492
   * #1489
   * #1488
   * #1487
   * #1486
   * __->__ #1485

<csr-id-dff2a40669736014349cf12744d6a057a7992e11/> start splitting out leader election into a separate module
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1485).
   * #1493
   * #1492
   * #1489
   * #1488
   * #1487
   * #1486
   * __->__ #1485

<csr-id-dff2a40669736014349cf12744d6a057a7992e11/> start splitting out leader election into a separate module
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1485).
   * #1493
   * #1492
   * #1489
   * #1488
   * #1487
   * #1486
   * __->__ #1485

<csr-id-dff2a40669736014349cf12744d6a057a7992e11/> start splitting out leader election into a separate module
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1485).
   * #1493
   * #1492
   * #1489
   * #1488
   * #1487
   * #1486
   * __->__ #1485

<csr-id-dff2a40669736014349cf12744d6a057a7992e11/> start splitting out leader election into a separate module
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1485).
   * #1493
   * #1492
   * #1489
   * #1488
   * #1487
   * #1486
   * __->__ #1485

### New Features

 - <csr-id-baedf23eaa056bc0dad8331d116bb71176764206/> improve quickstart ergonomics
 - <csr-id-98a21e36bd50d312402e46357fea6330816d0139/> add utility to dedup tees when debugging IR
 - <csr-id-2141c5f04cb7e9cb7cd2f50f849f6c4b3d745377/> add decouple and simple test and two_pc
 - <csr-id-074f2cf76158a126370a7e6b184bc6b928eb6fe2/> implement support for external network outputs
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
 - <csr-id-60d9becaf0b67f9819316ce6d76bd867f7d46505/> splice UDFs with type hints to avoid inference failures

### Bug Fixes

 - <csr-id-2faffdbf2cc886da22e496df64f46aefa380766c/> properly handle `crate::` imports
 - <csr-id-275a0edf1fb8eba467728c24edf3a984c8eaca75/> be more careful about which parts of proposer and acceptor have to be maintained atomically
 - <csr-id-87a68346aa10051d9d205d791407ce85546802da/> adjust default features to allow compilation to musl targets
   Previously, the default `deploy` feature would pull in Hydro Deploy and
   its transitive native dependencies.
   
   Also sets up `examples/paxos.rs` with CLI flags to deploy to GCP.
 - <csr-id-d4320e311562a004c01342a2b0f03ab6e2520562/> add missing `sample_every` for singletons
   Discovered during a live-coding demo, we only had it for optionals
   before.

### Refactor

 - <csr-id-a1b45203178165683cb4b5ae611c598cc9c14853/> move rewrites to a submodule
 - <csr-id-e9d05bf11a0e85da8ed1a0fe00be7769298308c2/> move `HfCompiled` and friends to a module
 - <csr-id-9f744052dd4ac744f5a1baa4e0cb9253adaeba1b/> use `location.flow_state()` to avoid clone
 - <csr-id-5b819a2dc6c507222a3e22d71efcde8b43cebad5/> deduplicate some error messages and drop unused `Interval` IR node
 - <csr-id-244207c2acd2243ece6e787d54eadacf06e9e8bb/> dedup signatures for `Stream` operators
 - <csr-id-bf9dcd5a923dd4b5efa337a9127086e5609a1722/> clean up traits for cycles and forward references
 - <csr-id-d9634f242a97c06bdb53011bf3d75256425a1598/> split up location module and store locations directly in streams
 - <csr-id-534fe974101e38ecb847cd759dbaf503ff97f822/> use `usize` for slot numbers
 - <csr-id-0a5abab3dac224c9591bcdd837d07c6e5c2773c6/> make Paxos-KV generic
 - <csr-id-38b17cd977fb6c00ddc37e7a5b30e45dba17329e/> simplify latency calculations
 - <csr-id-8b7b1c60fd33b78f9a4b0873bbbd150260ae2ad5/> complete split into leader election and sequencing phases
 - <csr-id-dff2a40669736014349cf12744d6a057a7992e11/> start splitting out leader election into a separate module
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1485).
   * #1493
   * #1492
   * #1489
   * #1488
   * #1487
   * #1486
   * __->__ #1485
 - <csr-id-1b18b358c87caa37a6519612131c8674653a2407/> simplify `persist_pullup` code
   Instead of matching on `&mut` and juggling ownership, instead match on
   the owned node and always replaced `*node = new_node` (sometimes itself)
 - <csr-id-c752affc2ee2c5d82d19dd992f6a89b7070b8773/> use max and min in Paxos and make client generic over ballots

<csr-id-dff2a40669736014349cf12744d6a057a7992e11/> start splitting out leader election into a separate module
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1485).
   * #1493
   * #1492
   * #1489
   * #1488
   * #1487
   * #1486
   * __->__ #1485
 - <csr-id-1b18b358c87caa37a6519612131c8674653a2407/> simplify `persist_pullup` code
   Instead of matching on `&mut` and juggling ownership, instead match on
   the owned node and always replaced `*node = new_node` (sometimes itself)
 - <csr-id-c752affc2ee2c5d82d19dd992f6a89b7070b8773/> use max and min in Paxos and make client generic over ballots

<csr-id-dff2a40669736014349cf12744d6a057a7992e11/> start splitting out leader election into a separate module
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1485).
   * #1493
   * #1492
   * #1489
   * #1488
   * #1487
   * #1486
   * __->__ #1485
 - <csr-id-1b18b358c87caa37a6519612131c8674653a2407/> simplify `persist_pullup` code
   Instead of matching on `&mut` and juggling ownership, instead match on
   the owned node and always replaced `*node = new_node` (sometimes itself)
 - <csr-id-c752affc2ee2c5d82d19dd992f6a89b7070b8773/> use max and min in Paxos and make client generic over ballots

<csr-id-dff2a40669736014349cf12744d6a057a7992e11/> start splitting out leader election into a separate module
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1485).
   * #1493
   * #1492
   * #1489
   * #1488
   * #1487
   * #1486
   * __->__ #1485
 - <csr-id-1b18b358c87caa37a6519612131c8674653a2407/> simplify `persist_pullup` code
   Instead of matching on `&mut` and juggling ownership, instead match on
   the owned node and always replaced `*node = new_node` (sometimes itself)
 - <csr-id-c752affc2ee2c5d82d19dd992f6a89b7070b8773/> use max and min in Paxos and make client generic over ballots

<csr-id-dff2a40669736014349cf12744d6a057a7992e11/> start splitting out leader election into a separate module
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1485).
   * #1493
   * #1492
   * #1489
   * #1488
   * #1487
   * #1486
   * __->__ #1485
 - <csr-id-1b18b358c87caa37a6519612131c8674653a2407/> simplify `persist_pullup` code
   Instead of matching on `&mut` and juggling ownership, instead match on
   the owned node and always replaced `*node = new_node` (sometimes itself)
 - <csr-id-c752affc2ee2c5d82d19dd992f6a89b7070b8773/> use max and min in Paxos and make client generic over ballots

<csr-id-dff2a40669736014349cf12744d6a057a7992e11/> start splitting out leader election into a separate module
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1485).
   * #1493
   * #1492
   * #1489
   * #1488
   * #1487
   * #1486
   * __->__ #1485
 - <csr-id-1b18b358c87caa37a6519612131c8674653a2407/> simplify `persist_pullup` code
   Instead of matching on `&mut` and juggling ownership, instead match on
   the owned node and always replaced `*node = new_node` (sometimes itself)
 - <csr-id-c752affc2ee2c5d82d19dd992f6a89b7070b8773/> use max and min in Paxos and make client generic over ballots

<csr-id-dff2a40669736014349cf12744d6a057a7992e11/> start splitting out leader election into a separate module
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1485).
   * #1493
   * #1492
   * #1489
   * #1488
   * #1487
   * #1486
   * __->__ #1485
 - <csr-id-1b18b358c87caa37a6519612131c8674653a2407/> simplify `persist_pullup` code
   Instead of matching on `&mut` and juggling ownership, instead match on
   the owned node and always replaced `*node = new_node` (sometimes itself)
 - <csr-id-c752affc2ee2c5d82d19dd992f6a89b7070b8773/> use max and min in Paxos and make client generic over ballots

<csr-id-dff2a40669736014349cf12744d6a057a7992e11/> start splitting out leader election into a separate module
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1485).
   * #1493
   * #1492
   * #1489
   * #1488
   * #1487
   * #1486
   * __->__ #1485
 - <csr-id-1b18b358c87caa37a6519612131c8674653a2407/> simplify `persist_pullup` code
   Instead of matching on `&mut` and juggling ownership, instead match on
   the owned node and always replaced `*node = new_node` (sometimes itself)
 - <csr-id-c752affc2ee2c5d82d19dd992f6a89b7070b8773/> use max and min in Paxos and make client generic over ballots

<csr-id-dff2a40669736014349cf12744d6a057a7992e11/> start splitting out leader election into a separate module
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1485).
   * #1493
   * #1492
   * #1489
   * #1488
   * #1487
   * #1486
   * __->__ #1485
 - <csr-id-1b18b358c87caa37a6519612131c8674653a2407/> simplify `persist_pullup` code
   Instead of matching on `&mut` and juggling ownership, instead match on
   the owned node and always replaced `*node = new_node` (sometimes itself)
 - <csr-id-c752affc2ee2c5d82d19dd992f6a89b7070b8773/> use max and min in Paxos and make client generic over ballots

<csr-id-dff2a40669736014349cf12744d6a057a7992e11/> start splitting out leader election into a separate module
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1485).
   * #1493
   * #1492
   * #1489
   * #1488
   * #1487
   * #1486
   * __->__ #1485
 - <csr-id-1b18b358c87caa37a6519612131c8674653a2407/> simplify `persist_pullup` code
   Instead of matching on `&mut` and juggling ownership, instead match on
   the owned node and always replaced `*node = new_node` (sometimes itself)
 - <csr-id-c752affc2ee2c5d82d19dd992f6a89b7070b8773/> use max and min in Paxos and make client generic over ballots

<csr-id-dff2a40669736014349cf12744d6a057a7992e11/> start splitting out leader election into a separate module
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1485).
   * #1493
   * #1492
   * #1489
   * #1488
   * #1487
   * #1486
   * __->__ #1485
 - <csr-id-1b18b358c87caa37a6519612131c8674653a2407/> simplify `persist_pullup` code
   Instead of matching on `&mut` and juggling ownership, instead match on
   the owned node and always replaced `*node = new_node` (sometimes itself)
 - <csr-id-c752affc2ee2c5d82d19dd992f6a89b7070b8773/> use max and min in Paxos and make client generic over ballots

<csr-id-dff2a40669736014349cf12744d6a057a7992e11/> start splitting out leader election into a separate module
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1485).
   * #1493
   * #1492
   * #1489
   * #1488
   * #1487
   * #1486
   * __->__ #1485
 - <csr-id-1b18b358c87caa37a6519612131c8674653a2407/> simplify `persist_pullup` code
   Instead of matching on `&mut` and juggling ownership, instead match on
   the owned node and always replaced `*node = new_node` (sometimes itself)
 - <csr-id-c752affc2ee2c5d82d19dd992f6a89b7070b8773/> use max and min in Paxos and make client generic over ballots

### Style

 - <csr-id-47cb703e771f7d1c451ceb9d185ada96410949da/> fixes for nightly clippy
   a couple few spurious `too_many_arguments` and a spurious
   `zombie_processes` still on current nightly (`clippy 0.1.84 (4392847410
   2024-10-21)`)

### New Features (BREAKING)

 - <csr-id-8d8b4b2288746e0aa2a95329d91297820aee7586/> implicitly apply default optimizations
   This also changes the behavior of `with_default_optimize` to be
   terminal, if users want to apply optimizations after these they should
   explicitly invoke the optimizations.
 - <csr-id-5d5209b4a5556618d8a8c8219e1e2a4e837256ef/> add an explicit API for creating tick contexts
   Previously, each location had a (semantic) global clock that drives
   ticks, and so all streams in a tick domain were all in the same atomic
   block. For future optimizations, we'd like developers to be able to
   place streams on the same location into different clocks to eliminate
   synchronization between them, which in turn would allow the computations
   in those separate clocks to be potentially decoupled across machines.
 - <csr-id-edd86496240e4ebb39e0cf3bc153d8f282ff2870/> strongly-typed runtime cluster IDs
   Instead of `u32`s everywhere, we now have a `ClusterId<C>` type that
   ensures that cluster IDs are not misused.
 - <csr-id-4f3b51b4b9187f1187be23e6f04034778fe76388/> provide an API for creating cycles across tick iterations
   Towards making it more clear which parts of a program depend on ticks
   versus don't.

### Refactor (BREAKING)

 - <csr-id-0bd3a2d2230cbef24210f71a3ea83d82d1cc7244/> eliminate remaining `Hf` name prefixes
 - <csr-id-9107841700db0ae72de6269ab6f132be0ae51cd9/> location type parameter before boundedness
   When looking at a prefix in an IDE, the location type argument is
   generally more useful.
 - <csr-id-919099ea3a414560b473ec89b993eeb26dfa2579/> dedup signatures for `Singleton` and `Optional`
   Also renames `cross_singleton` to `zip` when both sides are
   singleton-like.
 - <csr-id-5657563c989566e7c7b69dcb395e40b024c83c6c/> fold `Tick` vs `NoTick` into the location type parameter
   Now, when the location is a top-level `Process` or `Cluster` that
   corresponds to a `NoTick`, and for streams inside a tick we wrap the
   location type (e.g. `Tick<Process<...>>`). This simplifies type
   signatures for a lot of our example code.
 - <csr-id-e5b456bdafcb80aae6039e4c90a2e60098e499bf/> simplify intervals and split Paxos-KV into separate module
 - <csr-id-30c4f708faff7875ab42e551dd4bccbe231dfdad/> move input APIs back to being on locations
 - <csr-id-8ad997b2dfd23bb09f7d361d763d6b5e78f406d6/> move `self_id` and `members` to be APIs on cluster instead of builder

### `hydroflow_plus` Commit Statistics

<csr-read-only-do-not-edit/>

 - 39 commits contributed to the release.
 - 69 days passed between releases.
 - 38 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 38 unique issues were worked on: [#1434](https://github.com/hydro-project/hydroflow/issues/1434), [#1441](https://github.com/hydro-project/hydroflow/issues/1441), [#1443](https://github.com/hydro-project/hydroflow/issues/1443), [#1444](https://github.com/hydro-project/hydroflow/issues/1444), [#1449](https://github.com/hydro-project/hydroflow/issues/1449), [#1450](https://github.com/hydro-project/hydroflow/issues/1450), [#1451](https://github.com/hydro-project/hydroflow/issues/1451), [#1453](https://github.com/hydro-project/hydroflow/issues/1453), [#1455](https://github.com/hydro-project/hydroflow/issues/1455), [#1461](https://github.com/hydro-project/hydroflow/issues/1461), [#1464](https://github.com/hydro-project/hydroflow/issues/1464), [#1468](https://github.com/hydro-project/hydroflow/issues/1468), [#1471](https://github.com/hydro-project/hydroflow/issues/1471), [#1477](https://github.com/hydro-project/hydroflow/issues/1477), [#1485](https://github.com/hydro-project/hydroflow/issues/1485), [#1486](https://github.com/hydro-project/hydroflow/issues/1486), [#1488](https://github.com/hydro-project/hydroflow/issues/1488), [#1491](https://github.com/hydro-project/hydroflow/issues/1491), [#1505](https://github.com/hydro-project/hydroflow/issues/1505), [#1515](https://github.com/hydro-project/hydroflow/issues/1515), [#1516](https://github.com/hydro-project/hydroflow/issues/1516), [#1517](https://github.com/hydro-project/hydroflow/issues/1517), [#1519](https://github.com/hydro-project/hydroflow/issues/1519), [#1521](https://github.com/hydro-project/hydroflow/issues/1521), [#1523](https://github.com/hydro-project/hydroflow/issues/1523), [#1524](https://github.com/hydro-project/hydroflow/issues/1524), [#1525](https://github.com/hydro-project/hydroflow/issues/1525), [#1526](https://github.com/hydro-project/hydroflow/issues/1526), [#1527](https://github.com/hydro-project/hydroflow/issues/1527), [#1540](https://github.com/hydro-project/hydroflow/issues/1540), [#1541](https://github.com/hydro-project/hydroflow/issues/1541), [#1542](https://github.com/hydro-project/hydroflow/issues/1542), [#1543](https://github.com/hydro-project/hydroflow/issues/1543), [#1550](https://github.com/hydro-project/hydroflow/issues/1550), [#1551](https://github.com/hydro-project/hydroflow/issues/1551), [#1553](https://github.com/hydro-project/hydroflow/issues/1553), [#1554](https://github.com/hydro-project/hydroflow/issues/1554), [#1557](https://github.com/hydro-project/hydroflow/issues/1557)

### `hydroflow_plus` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1434](https://github.com/hydro-project/hydroflow/issues/1434)**
    - Splice UDFs with type hints to avoid inference failures ([`60d9bec`](https://github.com/hydro-project/hydroflow/commit/60d9becaf0b67f9819316ce6d76bd867f7d46505))
 * **[#1441](https://github.com/hydro-project/hydroflow/issues/1441)**
    - Provide an API for creating cycles across tick iterations ([`4f3b51b`](https://github.com/hydro-project/hydroflow/commit/4f3b51b4b9187f1187be23e6f04034778fe76388))
 * **[#1443](https://github.com/hydro-project/hydroflow/issues/1443)**
    - Use max and min in Paxos and make client generic over ballots ([`c752aff`](https://github.com/hydro-project/hydroflow/commit/c752affc2ee2c5d82d19dd992f6a89b7070b8773))
 * **[#1444](https://github.com/hydro-project/hydroflow/issues/1444)**
    - Update pinned rust version, clippy lints, remove some dead code ([`d567760`](https://github.com/hydro-project/hydroflow/commit/d5677604e93c07a5392f4229af94a0b736eca382))
 * **[#1449](https://github.com/hydro-project/hydroflow/issues/1449)**
    - Add API for external network inputs ([`8a80931`](https://github.com/hydro-project/hydroflow/commit/8a809315cd37929687fcabc34a12042db25d5767))
 * **[#1450](https://github.com/hydro-project/hydroflow/issues/1450)**
    - Add ability to have staged flows inside unit tests ([`afe78c3`](https://github.com/hydro-project/hydroflow/commit/afe78c343658472513b34d28658634b253148aee))
 * **[#1451](https://github.com/hydro-project/hydroflow/issues/1451)**
    - Implement support for external network outputs ([`074f2cf`](https://github.com/hydro-project/hydroflow/commit/074f2cf76158a126370a7e6b184bc6b928eb6fe2))
 * **[#1453](https://github.com/hydro-project/hydroflow/issues/1453)**
    - Add decouple and simple test and two_pc ([`2141c5f`](https://github.com/hydro-project/hydroflow/commit/2141c5f04cb7e9cb7cd2f50f849f6c4b3d745377))
 * **[#1455](https://github.com/hydro-project/hydroflow/issues/1455)**
    - Simplify `persist_pullup` code ([`1b18b35`](https://github.com/hydro-project/hydroflow/commit/1b18b358c87caa37a6519612131c8674653a2407))
 * **[#1461](https://github.com/hydro-project/hydroflow/issues/1461)**
    - Add missing `sample_every` for singletons ([`d4320e3`](https://github.com/hydro-project/hydroflow/commit/d4320e311562a004c01342a2b0f03ab6e2520562))
 * **[#1464](https://github.com/hydro-project/hydroflow/issues/1464)**
    - Adjust default features to allow compilation to musl targets ([`87a6834`](https://github.com/hydro-project/hydroflow/commit/87a68346aa10051d9d205d791407ce85546802da))
 * **[#1468](https://github.com/hydro-project/hydroflow/issues/1468)**
    - Move `self_id` and `members` to be APIs on cluster instead of builder ([`8ad997b`](https://github.com/hydro-project/hydroflow/commit/8ad997b2dfd23bb09f7d361d763d6b5e78f406d6))
 * **[#1471](https://github.com/hydro-project/hydroflow/issues/1471)**
    - Move input APIs back to being on locations ([`30c4f70`](https://github.com/hydro-project/hydroflow/commit/30c4f708faff7875ab42e551dd4bccbe231dfdad))
 * **[#1477](https://github.com/hydro-project/hydroflow/issues/1477)**
    - Strongly-typed runtime cluster IDs ([`edd8649`](https://github.com/hydro-project/hydroflow/commit/edd86496240e4ebb39e0cf3bc153d8f282ff2870))
 * **[#1485](https://github.com/hydro-project/hydroflow/issues/1485)**
    - Start splitting out leader election into a separate module ([`dff2a40`](https://github.com/hydro-project/hydroflow/commit/dff2a40669736014349cf12744d6a057a7992e11))
 * **[#1486](https://github.com/hydro-project/hydroflow/issues/1486)**
    - Complete split into leader election and sequencing phases ([`8b7b1c6`](https://github.com/hydro-project/hydroflow/commit/8b7b1c60fd33b78f9a4b0873bbbd150260ae2ad5))
 * **[#1488](https://github.com/hydro-project/hydroflow/issues/1488)**
    - Be more careful about which parts of proposer and acceptor have to be maintained atomically ([`275a0ed`](https://github.com/hydro-project/hydroflow/commit/275a0edf1fb8eba467728c24edf3a984c8eaca75))
 * **[#1491](https://github.com/hydro-project/hydroflow/issues/1491)**
    - Add utility to dedup tees when debugging IR ([`98a21e3`](https://github.com/hydro-project/hydroflow/commit/98a21e36bd50d312402e46357fea6330816d0139))
 * **[#1505](https://github.com/hydro-project/hydroflow/issues/1505)**
    - Fixes for nightly clippy ([`47cb703`](https://github.com/hydro-project/hydroflow/commit/47cb703e771f7d1c451ceb9d185ada96410949da))
 * **[#1515](https://github.com/hydro-project/hydroflow/issues/1515)**
    - Simplify latency calculations ([`38b17cd`](https://github.com/hydro-project/hydroflow/commit/38b17cd977fb6c00ddc37e7a5b30e45dba17329e))
 * **[#1516](https://github.com/hydro-project/hydroflow/issues/1516)**
    - Simplify intervals and split Paxos-KV into separate module ([`e5b456b`](https://github.com/hydro-project/hydroflow/commit/e5b456bdafcb80aae6039e4c90a2e60098e499bf))
 * **[#1517](https://github.com/hydro-project/hydroflow/issues/1517)**
    - Make Paxos-KV generic ([`0a5abab`](https://github.com/hydro-project/hydroflow/commit/0a5abab3dac224c9591bcdd837d07c6e5c2773c6))
 * **[#1519](https://github.com/hydro-project/hydroflow/issues/1519)**
    - Fold `Tick` vs `NoTick` into the location type parameter ([`5657563`](https://github.com/hydro-project/hydroflow/commit/5657563c989566e7c7b69dcb395e40b024c83c6c))
 * **[#1521](https://github.com/hydro-project/hydroflow/issues/1521)**
    - Use `usize` for slot numbers ([`534fe97`](https://github.com/hydro-project/hydroflow/commit/534fe974101e38ecb847cd759dbaf503ff97f822))
 * **[#1523](https://github.com/hydro-project/hydroflow/issues/1523)**
    - Split up location module and store locations directly in streams ([`d9634f2`](https://github.com/hydro-project/hydroflow/commit/d9634f242a97c06bdb53011bf3d75256425a1598))
 * **[#1524](https://github.com/hydro-project/hydroflow/issues/1524)**
    - Clean up traits for cycles and forward references ([`bf9dcd5`](https://github.com/hydro-project/hydroflow/commit/bf9dcd5a923dd4b5efa337a9127086e5609a1722))
 * **[#1525](https://github.com/hydro-project/hydroflow/issues/1525)**
    - Dedup signatures for `Stream` operators ([`244207c`](https://github.com/hydro-project/hydroflow/commit/244207c2acd2243ece6e787d54eadacf06e9e8bb))
 * **[#1526](https://github.com/hydro-project/hydroflow/issues/1526)**
    - Dedup signatures for `Singleton` and `Optional` ([`919099e`](https://github.com/hydro-project/hydroflow/commit/919099ea3a414560b473ec89b993eeb26dfa2579))
 * **[#1527](https://github.com/hydro-project/hydroflow/issues/1527)**
    - Properly handle `crate::` imports ([`2faffdb`](https://github.com/hydro-project/hydroflow/commit/2faffdbf2cc886da22e496df64f46aefa380766c))
 * **[#1540](https://github.com/hydro-project/hydroflow/issues/1540)**
    - Deduplicate some error messages and drop unused `Interval` IR node ([`5b819a2`](https://github.com/hydro-project/hydroflow/commit/5b819a2dc6c507222a3e22d71efcde8b43cebad5))
 * **[#1541](https://github.com/hydro-project/hydroflow/issues/1541)**
    - Use `location.flow_state()` to avoid clone ([`9f74405`](https://github.com/hydro-project/hydroflow/commit/9f744052dd4ac744f5a1baa4e0cb9253adaeba1b))
 * **[#1542](https://github.com/hydro-project/hydroflow/issues/1542)**
    - Move `HfCompiled` and friends to a module ([`e9d05bf`](https://github.com/hydro-project/hydroflow/commit/e9d05bf11a0e85da8ed1a0fe00be7769298308c2))
 * **[#1543](https://github.com/hydro-project/hydroflow/issues/1543)**
    - Move rewrites to a submodule ([`a1b4520`](https://github.com/hydro-project/hydroflow/commit/a1b45203178165683cb4b5ae611c598cc9c14853))
 * **[#1550](https://github.com/hydro-project/hydroflow/issues/1550)**
    - Add an explicit API for creating tick contexts ([`5d5209b`](https://github.com/hydro-project/hydroflow/commit/5d5209b4a5556618d8a8c8219e1e2a4e837256ef))
 * **[#1551](https://github.com/hydro-project/hydroflow/issues/1551)**
    - Location type parameter before boundedness ([`9107841`](https://github.com/hydro-project/hydroflow/commit/9107841700db0ae72de6269ab6f132be0ae51cd9))
 * **[#1553](https://github.com/hydro-project/hydroflow/issues/1553)**
    - Improve quickstart ergonomics ([`baedf23`](https://github.com/hydro-project/hydroflow/commit/baedf23eaa056bc0dad8331d116bb71176764206))
 * **[#1554](https://github.com/hydro-project/hydroflow/issues/1554)**
    - Eliminate remaining `Hf` name prefixes ([`0bd3a2d`](https://github.com/hydro-project/hydroflow/commit/0bd3a2d2230cbef24210f71a3ea83d82d1cc7244))
 * **[#1557](https://github.com/hydro-project/hydroflow/issues/1557)**
    - Implicitly apply default optimizations ([`8d8b4b2`](https://github.com/hydro-project/hydroflow/commit/8d8b4b2288746e0aa2a95329d91297820aee7586))
 * **Uncategorized**
    - Release hydroflow_lang v0.10.0, hydroflow_datalog_core v0.10.0, hydroflow_datalog v0.10.0, hydroflow_deploy_integration v0.10.0, hydroflow_macro v0.10.0, lattices_macro v0.5.7, variadics v0.0.7, variadics_macro v0.5.5, lattices v0.5.8, multiplatform_test v0.3.0, pusherator v0.0.9, hydroflow v0.10.0, hydro_deploy v0.10.0, stageleft_macro v0.4.0, stageleft v0.5.0, stageleft_tool v0.4.0, hydroflow_plus v0.10.0, hydro_cli v0.10.0, safety bump 8 crates ([`dcd48fc`](https://github.com/hydro-project/hydroflow/commit/dcd48fc7ee805898d9b5ef0d082870e30615e95b))
</details>

## v0.9.0 (2024-08-30)

<csr-id-11af32828bab6e4a4264d2635ff71a12bb0bb778/>
<csr-id-0a465e55dd39c76bc1aefb020460a639d792fe87/>
<csr-id-5f2789a13d1602f170e678fe9bbc59caf69db4b5/>
<csr-id-09d6d44eafc866881e73719813fe9edeb49ca2a6/>
<csr-id-fa417205569d8c49c85b0c2324118e0f9b1c8407/>

### Chore

 - <csr-id-11af32828bab6e4a4264d2635ff71a12bb0bb778/> lower min dependency versions where possible, update `Cargo.lock`
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
 - <csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

### Refactor (BREAKING)

 - <csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

### Documentation

 - <csr-id-f5f1eb0c612f5c0c1752360d972ef6853c5e12f0/> cleanup doc comments for clippy latest

### New Features

 - <csr-id-71f69aa5e9f2ba187f07c44c0a9f2becfe72aab1/> add API for cycle with initial value
 - <csr-id-82de6f5fc89fd44fd2ac18fddd94d121b4b10c8a/> add unbounded top-level singletons
 - <csr-id-7bf9ee2f707ddd5d8f51853ab7babe035fd8d964/> add paxos
 - <csr-id-46a8a2cb08732bb21096e824bc4542d208c68fb2/> use trybuild to compile subgraph binaries
 - <csr-id-eaf497b601928be37530bc8d81717d200fd5987a/> add operators necessary for Paxos / PBFT

### Bug Fixes

<csr-id-b518e674560971ebd1b32c737151214b8d3310b0/>
<csr-id-c12b2495c70f170eba655e458f4591ef7d0941a4/>
<csr-id-ab12e5b66718f06adc3c34bf879c9581d79ee0d2/>

 - <csr-id-22c72189bb76412955d29b03c5d99894c558a07c/> remove `FlowProps`
 - <csr-id-1aeacb212227f654e8f0cdc8a59816a68f059177/> rewrite IR in place to avoid stack overflow and disable cloning
   Cloning was unsafe because values behind a `Rc<RefCell<...>>` in the
   case of tee would be entangled with the old IR.
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1404).
   * #1405
   * #1398
   * __->__ #1404
* #1398
* __->__ #1404
* #1398
* __->__ #1404
* #1398
* __->__ #1404
* #1398
* __->__ #1404
* #1398
* __->__ #1404
* #1398
* __->__ #1404
* #1398
* __->__ #1404
* #1398
* __->__ #1404
* #1398
* __->__ #1404
* #1398
* __->__ #1404
* #1398
* __->__ #1404
* #1398
* __->__ #1404
 - <csr-id-1aeacb212227f654e8f0cdc8a59816a68f059177/> rewrite IR in place to avoid stack overflow and disable cloning
   Cloning was unsafe because values behind a `Rc<RefCell<...>>` in the
   case of tee would be entangled with the old IR.
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1404).
   * #1405
   * #1398
   * __->__ #1404
* #1398
* __->__ #1404
* #1398
* __->__ #1404
* #1398
* __->__ #1404
* #1398
* __->__ #1404
* #1398
* __->__ #1404
* #1398
* __->__ #1404
* #1398
* __->__ #1404
* #1398
* __->__ #1404
* #1398
* __->__ #1404
* #1398
* __->__ #1404
* #1398
* __->__ #1404
 - <csr-id-1aeacb212227f654e8f0cdc8a59816a68f059177/> rewrite IR in place to avoid stack overflow and disable cloning
   Cloning was unsafe because values behind a `Rc<RefCell<...>>` in the
   case of tee would be entangled with the old IR.
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1404).
   * #1405
   * #1398
   * __->__ #1404
* #1398
* __->__ #1404
* #1398
* __->__ #1404
* #1398
* __->__ #1404
* #1398
* __->__ #1404
* #1398
* __->__ #1404
* #1398
* __->__ #1404
* #1398
* __->__ #1404
* #1398
* __->__ #1404
* #1398
* __->__ #1404
* #1398
* __->__ #1404

### New Features (BREAKING)

 - <csr-id-44c6b149bea102e8598460ba0286e370b36fd25a/> separate singletons into their own types
 - <csr-id-536e6442d68b0947da5bfef9991825003e6867fc/> refactor API to have no-tick semantics by default
   Now, by default streams exist at a "top-level" where there are no ticks
   and operators run over the entire collection. To perform iterative
   computations, developers must explicitly entire a tick domain (using
   `tick_batch`), and return to the outer domain (using `all_ticks`).

### Refactor (BREAKING)

 - <csr-id-0a465e55dd39c76bc1aefb020460a639d792fe87/> rename integration crates to drop CLI references
 - <csr-id-5f2789a13d1602f170e678fe9bbc59caf69db4b5/> disentangle instantiated nodes from locations
 - <csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377
 - <csr-id-09d6d44eafc866881e73719813fe9edeb49ca2a6/> start rearranging stages of flow compilation to prepare for trybuild approach

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377
 - <csr-id-09d6d44eafc866881e73719813fe9edeb49ca2a6/> start rearranging stages of flow compilation to prepare for trybuild approach

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377
 - <csr-id-09d6d44eafc866881e73719813fe9edeb49ca2a6/> start rearranging stages of flow compilation to prepare for trybuild approach

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377
 - <csr-id-09d6d44eafc866881e73719813fe9edeb49ca2a6/> start rearranging stages of flow compilation to prepare for trybuild approach

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377
 - <csr-id-09d6d44eafc866881e73719813fe9edeb49ca2a6/> start rearranging stages of flow compilation to prepare for trybuild approach

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377
 - <csr-id-09d6d44eafc866881e73719813fe9edeb49ca2a6/> start rearranging stages of flow compilation to prepare for trybuild approach

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377
 - <csr-id-09d6d44eafc866881e73719813fe9edeb49ca2a6/> start rearranging stages of flow compilation to prepare for trybuild approach

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377
 - <csr-id-09d6d44eafc866881e73719813fe9edeb49ca2a6/> start rearranging stages of flow compilation to prepare for trybuild approach

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377
 - <csr-id-09d6d44eafc866881e73719813fe9edeb49ca2a6/> start rearranging stages of flow compilation to prepare for trybuild approach

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377
 - <csr-id-09d6d44eafc866881e73719813fe9edeb49ca2a6/> start rearranging stages of flow compilation to prepare for trybuild approach

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377
 - <csr-id-09d6d44eafc866881e73719813fe9edeb49ca2a6/> start rearranging stages of flow compilation to prepare for trybuild approach

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377
 - <csr-id-09d6d44eafc866881e73719813fe9edeb49ca2a6/> start rearranging stages of flow compilation to prepare for trybuild approach

<csr-id-128aaecd40edce57dc254afdcd61ecd5b9948d71/> simplify process/cluster specs
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1394).
   * #1395
   * __->__ #1394
 - <csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377

<csr-id-0eba702f62e7a6816cf931b01a2ea5643bd7321d/> defer network instantiation until after finalizing IR
   ---
   [//]: # (BEGIN SAPLING FOOTER)
   Stack created with [Sapling](https://sapling-scm.com). Best reviewed
   with
   [ReviewStack](https://reviewstack.dev/hydro-project/hydroflow/pull/1377).
   * #1395
   * #1394
   * __->__ #1377
 - <csr-id-09d6d44eafc866881e73719813fe9edeb49ca2a6/> start rearranging stages of flow compilation to prepare for trybuild approach

### Style (BREAKING)

 - <csr-id-fa417205569d8c49c85b0c2324118e0f9b1c8407/> rename some `CLI`->`Deploy`, decapitalize acronym names

### `hydroflow_plus` Commit Statistics

<csr-read-only-do-not-edit/>

 - 21 commits contributed to the release.
 - 38 days passed between releases.
 - 20 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 20 unique issues were worked on: [#1358](https://github.com/hydro-project/hydroflow/issues/1358), [#1368](https://github.com/hydro-project/hydroflow/issues/1368), [#1375](https://github.com/hydro-project/hydroflow/issues/1375), [#1376](https://github.com/hydro-project/hydroflow/issues/1376), [#1377](https://github.com/hydro-project/hydroflow/issues/1377), [#1394](https://github.com/hydro-project/hydroflow/issues/1394), [#1395](https://github.com/hydro-project/hydroflow/issues/1395), [#1398](https://github.com/hydro-project/hydroflow/issues/1398), [#1399](https://github.com/hydro-project/hydroflow/issues/1399), [#1404](https://github.com/hydro-project/hydroflow/issues/1404), [#1405](https://github.com/hydro-project/hydroflow/issues/1405), [#1410](https://github.com/hydro-project/hydroflow/issues/1410), [#1413](https://github.com/hydro-project/hydroflow/issues/1413), [#1420](https://github.com/hydro-project/hydroflow/issues/1420), [#1421](https://github.com/hydro-project/hydroflow/issues/1421), [#1423](https://github.com/hydro-project/hydroflow/issues/1423), [#1425](https://github.com/hydro-project/hydroflow/issues/1425), [#1427](https://github.com/hydro-project/hydroflow/issues/1427), [#1428](https://github.com/hydro-project/hydroflow/issues/1428), [#1430](https://github.com/hydro-project/hydroflow/issues/1430)

### `hydroflow_plus` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1358](https://github.com/hydro-project/hydroflow/issues/1358)**
    - Start rearranging stages of flow compilation to prepare for trybuild approach ([`09d6d44`](https://github.com/hydro-project/hydroflow/commit/09d6d44eafc866881e73719813fe9edeb49ca2a6))
 * **[#1368](https://github.com/hydro-project/hydroflow/issues/1368)**
    - Overly restrictive input types for `send_bincode_interleaved` ([`ab12e5b`](https://github.com/hydro-project/hydroflow/commit/ab12e5b66718f06adc3c34bf879c9581d79ee0d2))
 * **[#1375](https://github.com/hydro-project/hydroflow/issues/1375)**
    - Add `Clone` bounds to `cross_join` and simplify broadcast logic ([`c12b249`](https://github.com/hydro-project/hydroflow/commit/c12b2495c70f170eba655e458f4591ef7d0941a4))
 * **[#1376](https://github.com/hydro-project/hydroflow/issues/1376)**
    - Add operators necessary for Paxos / PBFT ([`eaf497b`](https://github.com/hydro-project/hydroflow/commit/eaf497b601928be37530bc8d81717d200fd5987a))
 * **[#1377](https://github.com/hydro-project/hydroflow/issues/1377)**
    - Defer network instantiation until after finalizing IR ([`0eba702`](https://github.com/hydro-project/hydroflow/commit/0eba702f62e7a6816cf931b01a2ea5643bd7321d))
 * **[#1394](https://github.com/hydro-project/hydroflow/issues/1394)**
    - Simplify process/cluster specs ([`128aaec`](https://github.com/hydro-project/hydroflow/commit/128aaecd40edce57dc254afdcd61ecd5b9948d71))
 * **[#1395](https://github.com/hydro-project/hydroflow/issues/1395)**
    - Disentangle instantiated nodes from locations ([`5f2789a`](https://github.com/hydro-project/hydroflow/commit/5f2789a13d1602f170e678fe9bbc59caf69db4b5))
 * **[#1398](https://github.com/hydro-project/hydroflow/issues/1398)**
    - Use trybuild to compile subgraph binaries ([`46a8a2c`](https://github.com/hydro-project/hydroflow/commit/46a8a2cb08732bb21096e824bc4542d208c68fb2))
 * **[#1399](https://github.com/hydro-project/hydroflow/issues/1399)**
    - Rename some `CLI`->`Deploy`, decapitalize acronym names ([`fa41720`](https://github.com/hydro-project/hydroflow/commit/fa417205569d8c49c85b0c2324118e0f9b1c8407))
 * **[#1404](https://github.com/hydro-project/hydroflow/issues/1404)**
    - Rewrite IR in place to avoid stack overflow and disable cloning ([`1aeacb2`](https://github.com/hydro-project/hydroflow/commit/1aeacb212227f654e8f0cdc8a59816a68f059177))
 * **[#1405](https://github.com/hydro-project/hydroflow/issues/1405)**
    - Wrong stream type for `source_interval` ([`b518e67`](https://github.com/hydro-project/hydroflow/commit/b518e674560971ebd1b32c737151214b8d3310b0))
 * **[#1410](https://github.com/hydro-project/hydroflow/issues/1410)**
    - Add paxos ([`7bf9ee2`](https://github.com/hydro-project/hydroflow/commit/7bf9ee2f707ddd5d8f51853ab7babe035fd8d964))
 * **[#1413](https://github.com/hydro-project/hydroflow/issues/1413)**
    - Rename integration crates to drop CLI references ([`0a465e5`](https://github.com/hydro-project/hydroflow/commit/0a465e55dd39c76bc1aefb020460a639d792fe87))
 * **[#1420](https://github.com/hydro-project/hydroflow/issues/1420)**
    - Remove `FlowProps` ([`22c7218`](https://github.com/hydro-project/hydroflow/commit/22c72189bb76412955d29b03c5d99894c558a07c))
 * **[#1421](https://github.com/hydro-project/hydroflow/issues/1421)**
    - Refactor API to have no-tick semantics by default ([`536e644`](https://github.com/hydro-project/hydroflow/commit/536e6442d68b0947da5bfef9991825003e6867fc))
 * **[#1423](https://github.com/hydro-project/hydroflow/issues/1423)**
    - Lower min dependency versions where possible, update `Cargo.lock` ([`11af328`](https://github.com/hydro-project/hydroflow/commit/11af32828bab6e4a4264d2635ff71a12bb0bb778))
 * **[#1425](https://github.com/hydro-project/hydroflow/issues/1425)**
    - Separate singletons into their own types ([`44c6b14`](https://github.com/hydro-project/hydroflow/commit/44c6b149bea102e8598460ba0286e370b36fd25a))
 * **[#1427](https://github.com/hydro-project/hydroflow/issues/1427)**
    - Add unbounded top-level singletons ([`82de6f5`](https://github.com/hydro-project/hydroflow/commit/82de6f5fc89fd44fd2ac18fddd94d121b4b10c8a))
 * **[#1428](https://github.com/hydro-project/hydroflow/issues/1428)**
    - Cleanup doc comments for clippy latest ([`f5f1eb0`](https://github.com/hydro-project/hydroflow/commit/f5f1eb0c612f5c0c1752360d972ef6853c5e12f0))
 * **[#1430](https://github.com/hydro-project/hydroflow/issues/1430)**
    - Add API for cycle with initial value ([`71f69aa`](https://github.com/hydro-project/hydroflow/commit/71f69aa5e9f2ba187f07c44c0a9f2becfe72aab1))
 * **Uncategorized**
    - Release hydroflow_lang v0.9.0, hydroflow_datalog_core v0.9.0, hydroflow_datalog v0.9.0, hydroflow_deploy_integration v0.9.0, hydroflow_macro v0.9.0, lattices_macro v0.5.6, lattices v0.5.7, multiplatform_test v0.2.0, variadics v0.0.6, pusherator v0.0.8, hydroflow v0.9.0, stageleft_macro v0.3.0, stageleft v0.4.0, stageleft_tool v0.3.0, hydroflow_plus v0.9.0, hydro_deploy v0.9.0, hydro_cli v0.9.0, hydroflow_plus_deploy v0.9.0, safety bump 8 crates ([`0750117`](https://github.com/hydro-project/hydroflow/commit/0750117de7088c01a439b102adeb4c832889f171))
</details>

## v0.8.0 (2024-07-23)

<csr-id-67c0e51fb25ea1a2e3aae197c1984920b46759fa/>

### Reverted

 - <csr-id-256779abece03bee662b351430d27141d10bd5ef/> "feat(hydroflow): Added poll_futures and poll_futures_async operators.", fix #1183
   This reverts commit 997d90a76db9a4e05dbac35073a09548750ce342.
   
   We have been trying to figure out the semantics a bit, and want to give
   it more thought before we commit to maintaining it
   
   Can un-revert and adjust the semantics later when we use it

### Refactor (BREAKING)

 - <csr-id-67c0e51fb25ea1a2e3aae197c1984920b46759fa/> require lifetime on `perist*()` operators

### `hydroflow_plus` Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 59 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 3 unique issues were worked on: [#1143](https://github.com/hydro-project/hydroflow/issues/1143), [#1216](https://github.com/hydro-project/hydroflow/issues/1216), [#1295](https://github.com/hydro-project/hydroflow/issues/1295)

### `hydroflow_plus` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1143](https://github.com/hydro-project/hydroflow/issues/1143)**
    - "feat(hydroflow): Added poll_futures and poll_futures_async operators.", fix #1183 ([`256779a`](https://github.com/hydro-project/hydroflow/commit/256779abece03bee662b351430d27141d10bd5ef))
 * **[#1216](https://github.com/hydro-project/hydroflow/issues/1216)**
    - "feat(hydroflow): Added poll_futures and poll_futures_async operators.", fix #1183 ([`256779a`](https://github.com/hydro-project/hydroflow/commit/256779abece03bee662b351430d27141d10bd5ef))
 * **[#1295](https://github.com/hydro-project/hydroflow/issues/1295)**
    - Require lifetime on `perist*()` operators ([`67c0e51`](https://github.com/hydro-project/hydroflow/commit/67c0e51fb25ea1a2e3aae197c1984920b46759fa))
 * **Uncategorized**
    - Release hydroflow_lang v0.8.0, hydroflow_datalog_core v0.8.0, hydroflow_datalog v0.8.0, hydroflow_macro v0.8.0, lattices_macro v0.5.5, lattices v0.5.6, variadics v0.0.5, pusherator v0.0.7, hydroflow v0.8.0, hydroflow_plus v0.8.0, hydro_deploy v0.8.0, hydro_cli v0.8.0, hydroflow_plus_cli_integration v0.8.0, safety bump 7 crates ([`ca6c16b`](https://github.com/hydro-project/hydroflow/commit/ca6c16b4a7ce35e155fe7fc6c7d1676c37c9e4de))
</details>

## v0.7.0 (2024-05-24)

<csr-id-c9dfddc680e0ce5415539d7b77bc5beb97ab59d9/>

### Chore

 - <csr-id-c9dfddc680e0ce5415539d7b77bc5beb97ab59d9/> use workaround for `cargo smart-release` not properly ordering `dev-`/`build-dependencies`

### New Features

 - <csr-id-6e571726ff40818fbe9bbe9923511877c20fb243/> add API to get the cluster ID of the current node
   feat(hydroflow_plus): add API to get the cluster ID of the current node
 - <csr-id-997d90a76db9a4e05dbac35073a09548750ce342/> Added poll_futures and poll_futures_async operators.
 - <csr-id-c3f5a37ff746401a2383a900f9004e33072d5b1a/> add prototype of tagging algebraic properties
 - <csr-id-29a263fb564c5ce4bc495ea4e9d20b8b2621b645/> add support for collecting counts and running perf

### Bug Fixes

 - <csr-id-0cafbdb74a665412a83aa900b4eb10c00e2498dd/> handle send_bincode with local structs
   fix(hydroflow_plus): handle send_bincode with local structs

### `hydroflow_plus` Commit Statistics

<csr-read-only-do-not-edit/>

 - 7 commits contributed to the release.
 - 44 days passed between releases.
 - 6 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 6 unique issues were worked on: [#1143](https://github.com/hydro-project/hydroflow/issues/1143), [#1151](https://github.com/hydro-project/hydroflow/issues/1151), [#1156](https://github.com/hydro-project/hydroflow/issues/1156), [#1157](https://github.com/hydro-project/hydroflow/issues/1157), [#1194](https://github.com/hydro-project/hydroflow/issues/1194), [#1238](https://github.com/hydro-project/hydroflow/issues/1238)

### `hydroflow_plus` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1143](https://github.com/hydro-project/hydroflow/issues/1143)**
    - Added poll_futures and poll_futures_async operators. ([`997d90a`](https://github.com/hydro-project/hydroflow/commit/997d90a76db9a4e05dbac35073a09548750ce342))
 * **[#1151](https://github.com/hydro-project/hydroflow/issues/1151)**
    - Handle send_bincode with local structs ([`0cafbdb`](https://github.com/hydro-project/hydroflow/commit/0cafbdb74a665412a83aa900b4eb10c00e2498dd))
 * **[#1156](https://github.com/hydro-project/hydroflow/issues/1156)**
    - Add prototype of tagging algebraic properties ([`c3f5a37`](https://github.com/hydro-project/hydroflow/commit/c3f5a37ff746401a2383a900f9004e33072d5b1a))
 * **[#1157](https://github.com/hydro-project/hydroflow/issues/1157)**
    - Add support for collecting counts and running perf ([`29a263f`](https://github.com/hydro-project/hydroflow/commit/29a263fb564c5ce4bc495ea4e9d20b8b2621b645))
 * **[#1194](https://github.com/hydro-project/hydroflow/issues/1194)**
    - Add API to get the cluster ID of the current node ([`6e57172`](https://github.com/hydro-project/hydroflow/commit/6e571726ff40818fbe9bbe9923511877c20fb243))
 * **[#1238](https://github.com/hydro-project/hydroflow/issues/1238)**
    - Use workaround for `cargo smart-release` not properly ordering `dev-`/`build-dependencies` ([`c9dfddc`](https://github.com/hydro-project/hydroflow/commit/c9dfddc680e0ce5415539d7b77bc5beb97ab59d9))
 * **Uncategorized**
    - Release hydroflow_lang v0.7.0, hydroflow_datalog_core v0.7.0, hydroflow_datalog v0.7.0, hydroflow_macro v0.7.0, lattices v0.5.5, multiplatform_test v0.1.0, pusherator v0.0.6, hydroflow v0.7.0, stageleft_macro v0.2.0, stageleft v0.3.0, stageleft_tool v0.2.0, hydroflow_plus v0.7.0, hydro_deploy v0.7.0, hydro_cli v0.7.0, hydroflow_plus_cli_integration v0.7.0, safety bump 8 crates ([`2852147`](https://github.com/hydro-project/hydroflow/commit/285214740627685e911781793e05d234ab2ad2bd))
</details>

## v0.6.1 (2024-04-09)

<csr-id-fc447ffdf8fd1b2189545a991f08588238182f00/>

### Chore

 - <csr-id-fc447ffdf8fd1b2189545a991f08588238182f00/> appease latest nightly clippy
   Also updates `surface_keyed_fold.rs` `test_fold_keyed_infer_basic` test.

### New Features

 - <csr-id-7f68ebf2a23e8e73719229a6f0408bffc7fbe7af/> simplify Location trait to remove lifetimes
 - <csr-id-77f3e5afb9e276d1d6c643574ebac75ed0003939/> simplify lifetime bounds for processes and clusters
   feat(hydroflow_plus): simplify lifetime bounds for processes and
   clusters
   
   This allows `extract` to move the flow builder, which is a prerequisite
   for having developers run the optimizer during deployment as well in
   case it changes the network topology.
 - <csr-id-5b6562662ce3a0dd172ddc1103a591c1c6037e95/> move persist manipulation into a proper optimization
   feat(hydroflow_plus): move persist manipulation into a proper
   optimization
 - <csr-id-cfb3029a6fb0836789db04a7d0d4a1e8b812b629/> add APIs for running optimization passes
   feat(hydroflow_plus): add APIs for running optimization passes

### Bug Fixes

 - <csr-id-2d2c43dc001dbea17d46d73de464c95066b18fa2/> allow BuiltFlow to be cloned even if the deploy flavor can't

### `hydroflow_plus` Commit Statistics

<csr-read-only-do-not-edit/>

 - 9 commits contributed to the release.
 - 38 days passed between releases.
 - 6 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 6 unique issues were worked on: [#1083](https://github.com/hydro-project/hydroflow/issues/1083), [#1098](https://github.com/hydro-project/hydroflow/issues/1098), [#1100](https://github.com/hydro-project/hydroflow/issues/1100), [#1101](https://github.com/hydro-project/hydroflow/issues/1101), [#1107](https://github.com/hydro-project/hydroflow/issues/1107), [#1140](https://github.com/hydro-project/hydroflow/issues/1140)

### `hydroflow_plus` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1083](https://github.com/hydro-project/hydroflow/issues/1083)**
    - Add APIs for running optimization passes ([`cfb3029`](https://github.com/hydro-project/hydroflow/commit/cfb3029a6fb0836789db04a7d0d4a1e8b812b629))
 * **[#1098](https://github.com/hydro-project/hydroflow/issues/1098)**
    - Move persist manipulation into a proper optimization ([`5b65626`](https://github.com/hydro-project/hydroflow/commit/5b6562662ce3a0dd172ddc1103a591c1c6037e95))
 * **[#1100](https://github.com/hydro-project/hydroflow/issues/1100)**
    - Simplify lifetime bounds for processes and clusters ([`77f3e5a`](https://github.com/hydro-project/hydroflow/commit/77f3e5afb9e276d1d6c643574ebac75ed0003939))
 * **[#1101](https://github.com/hydro-project/hydroflow/issues/1101)**
    - Simplify Location trait to remove lifetimes ([`7f68ebf`](https://github.com/hydro-project/hydroflow/commit/7f68ebf2a23e8e73719229a6f0408bffc7fbe7af))
 * **[#1107](https://github.com/hydro-project/hydroflow/issues/1107)**
    - Allow BuiltFlow to be cloned even if the deploy flavor can't ([`2d2c43d`](https://github.com/hydro-project/hydroflow/commit/2d2c43dc001dbea17d46d73de464c95066b18fa2))
 * **[#1140](https://github.com/hydro-project/hydroflow/issues/1140)**
    - Appease latest nightly clippy ([`fc447ff`](https://github.com/hydro-project/hydroflow/commit/fc447ffdf8fd1b2189545a991f08588238182f00))
 * **Uncategorized**
    - Release hydroflow_plus v0.6.1, hydro_deploy v0.6.1, hydro_cli v0.6.1, hydroflow_plus_cli_integration v0.6.1 ([`c385c13`](https://github.com/hydro-project/hydroflow/commit/c385c132c9733d1bace82156aa14216b8e7fef9f))
    - Release hydroflow_lang v0.6.2, hydroflow v0.6.2, hydroflow_plus v0.6.1, hydro_deploy v0.6.1, hydro_cli v0.6.1, hydroflow_plus_cli_integration v0.6.1, stageleft_tool v0.1.1 ([`23cfe08`](https://github.com/hydro-project/hydroflow/commit/23cfe0839079aa17d042bbd3976f6d188689d290))
    - Release hydroflow_cli_integration v0.5.2, hydroflow_lang v0.6.1, hydroflow_datalog_core v0.6.1, lattices v0.5.4, hydroflow v0.6.1, stageleft_macro v0.1.1, stageleft v0.2.1, hydroflow_plus v0.6.1, hydro_deploy v0.6.1, hydro_cli v0.6.1, hydroflow_plus_cli_integration v0.6.1, stageleft_tool v0.1.1 ([`cd63f22`](https://github.com/hydro-project/hydroflow/commit/cd63f2258c961a40f0e5dbef20ac329a2d570ad0))
</details>

## v0.6.0 (2024-03-02)

<csr-id-39ab8b0278e9e3fe96552ace0a4ae768a6bc10d8/>

### Chore

 - <csr-id-39ab8b0278e9e3fe96552ace0a4ae768a6bc10d8/> appease various clippy lints

### New Features

 - <csr-id-c1d1b51ee26cc9946af59ac02c040e0a33d15fde/> unify send/demux/tagged APIs
   feat(hydroflow_plus): unify send/demux/tagged APIs
 - <csr-id-eb34ccd13f56e1d07cbae35ead79daeb3b9bad20/> use an IR before lowering to Hydroflow
   Makes it possible to write custom optimization passes.

### `hydroflow_plus` Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 32 days passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 3 unique issues were worked on: [#1070](https://github.com/hydro-project/hydroflow/issues/1070), [#1080](https://github.com/hydro-project/hydroflow/issues/1080), [#1084](https://github.com/hydro-project/hydroflow/issues/1084)

### `hydroflow_plus` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1070](https://github.com/hydro-project/hydroflow/issues/1070)**
    - Use an IR before lowering to Hydroflow ([`eb34ccd`](https://github.com/hydro-project/hydroflow/commit/eb34ccd13f56e1d07cbae35ead79daeb3b9bad20))
 * **[#1080](https://github.com/hydro-project/hydroflow/issues/1080)**
    - Unify send/demux/tagged APIs ([`c1d1b51`](https://github.com/hydro-project/hydroflow/commit/c1d1b51ee26cc9946af59ac02c040e0a33d15fde))
 * **[#1084](https://github.com/hydro-project/hydroflow/issues/1084)**
    - Appease various clippy lints ([`39ab8b0`](https://github.com/hydro-project/hydroflow/commit/39ab8b0278e9e3fe96552ace0a4ae768a6bc10d8))
 * **Uncategorized**
    - Release hydroflow_lang v0.6.0, hydroflow_datalog_core v0.6.0, hydroflow_datalog v0.6.0, hydroflow_macro v0.6.0, lattices v0.5.3, variadics v0.0.4, pusherator v0.0.5, hydroflow v0.6.0, stageleft v0.2.0, hydroflow_plus v0.6.0, hydro_deploy v0.6.0, hydro_cli v0.6.0, hydroflow_plus_cli_integration v0.6.0, safety bump 7 crates ([`09ea65f`](https://github.com/hydro-project/hydroflow/commit/09ea65fe9cd45c357c43bffca30e60243fa45cc8))
</details>

## v0.5.1 (2024-01-29)

<csr-id-1b555e57c8c812bed4d6495d2960cbf77fb0b3ef/>

### Chore

 - <csr-id-1b555e57c8c812bed4d6495d2960cbf77fb0b3ef/> manually set lockstep-versioned crates (and `lattices`) to version `0.5.1`
   Setting manually since
   https://github.com/frewsxcv/rust-crates-index/issues/159 is messing with
   smart-release

### Documentation

 - <csr-id-3b36020d16792f26da4df3c5b09652a4ab47ec4f/> actually committing empty CHANGELOG.md is required

### New Features

 - <csr-id-5a03ed41548b5766b945efbd1eedb0dfceb714d9/> add core negation operators
 - <csr-id-7d930a2ccf656d3d6bc5db3e22eb63c5fd6d37d1/> add APIs for declaring external ports on clusters
 - <csr-id-73e9b68ec2f5b2627784addcce9fba684848bb55/> implement keyed fold and reduce
 - <csr-id-5e6ebac1a7f128227ae92a8c195235b27532e17a/> add interleaved shortcut when sending from a cluster
 - <csr-id-af6e3be60fdb69ceec1613347910f4dd49980d34/> push down persists and implement Pi example
   Also fixes type inference issues with reduce the same way as we did for fold.
 - <csr-id-6eeb9be9bc4136041a2855f650ae640c478b7fc9/> improve API naming and polish docs
 - <csr-id-44a308f77bddd67b5c51723ac39f3bc10af52553/> tweak naming of windowing operators
 - <csr-id-1edc5ae5b5f70e1390183e8c8eb27eb0ab32196d/> provide simpler API for launching and minimize dependencies
 - <csr-id-b7aafd3c97897db4bff62c4ab0b7480ef9a799e0/> improve API naming and eliminate wire API for builders
 - <csr-id-d288e51f980577510bb2ed45c04554102c4f1e14/> split API for building single-node graphs
 - <csr-id-26f4d6f610b78a75c41b1ae63366d089ad08b322/> require explicit batching for aggregation operators
 - <csr-id-174607d12277d7544d0f42890c9a5da2ff184df4/> support building graphs for symmetric clusters in Hydroflow+
 - <csr-id-9e275824c88b24d060a7de5822e1359959b36b03/> auto-configure Hydro Deploy based on Hydroflow+ plans
 - <csr-id-27dabcf6878576dc3675788ce3381cb25116033a/> add preliminary `send_to` operator for multi-node graphs
 - <csr-id-e5bdd12e32d6ea72fd91a55c12e09f07a0edaa5c/> add initial test using Hydro CLI from Hydroflow+
   This also required a change to Hydroflow core to make it possible to run the dataflow itself on a single thread (using a LocalSet), even if the surrounding runtime is not single-threaded (required to work around deadlocks because we can't use async APIs inside Hydroflow+). This requires us to spawn any Hydroflow tasks (only for `dest_sink` at the moment) right next to when we run the dataflow rather than when the Hydroflow graph is initialized. From a conceptual perspective, this seems _more right_, since now creating a Hydroflow program will not result in any actual tasks running.
   
   In the third PR of this series, I aim to add a new Hydroflow+ operator that will automate the setup of a `dest_sink`/`source_stream` pair that span nodes.
 - <csr-id-05fb1353cf3e0e8c5da9522365150bd78bd3c5f8/> allow Hydroflow+ programs to emit multiple graphs
   This PR adds support for tagging elements of Hydroflow+ graphs with a node ID, an integer which specifies which Hydroflow graph the computation should be emitted to. The generated code includes the Hydroflow graph for each node ID, so that the appropriate graph can be selected at runtime.
   
   At a larger scale, this is a precursor to adding network operators to Hydroflow+, which will allow distributed logic to be described in a single Hydroflow+ program by specifying points at which data is transferred between different graphs.
 - <csr-id-8b635683e5ac3c4ed2d896ae88e2953db1c6312c/> add a functional surface syntax using staging

### Bug Fixes

 - <csr-id-88a17967d0c9e681a04de4b5796f532f4833272c/> persist cluster IDs for broadcast
   I'll follow this up with a unit test for this, but want to get this fixed ASAP first.
 - <csr-id-bd2bf233302e3638c8f4bc9c0460e1a47edc00aa/> rewrite uses of alloc crate in bincode operators
 - <csr-id-2addaed8a8a441bff7acf9a0a265cc09483fd487/> disallow joining streams on different nodes
 - <csr-id-38411ea007d4feb30dd16bdd1505802a111a67d1/> fix spelling of "propagate"

### `hydroflow_plus` Commit Statistics

<csr-read-only-do-not-edit/>

 - 25 commits contributed to the release.
 - 23 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 20 unique issues were worked on: [#1001](https://github.com/hydro-project/hydroflow/issues/1001), [#1003](https://github.com/hydro-project/hydroflow/issues/1003), [#1004](https://github.com/hydro-project/hydroflow/issues/1004), [#1006](https://github.com/hydro-project/hydroflow/issues/1006), [#1013](https://github.com/hydro-project/hydroflow/issues/1013), [#1021](https://github.com/hydro-project/hydroflow/issues/1021), [#1022](https://github.com/hydro-project/hydroflow/issues/1022), [#1023](https://github.com/hydro-project/hydroflow/issues/1023), [#1035](https://github.com/hydro-project/hydroflow/issues/1035), [#1036](https://github.com/hydro-project/hydroflow/issues/1036), [#899](https://github.com/hydro-project/hydroflow/issues/899), [#976](https://github.com/hydro-project/hydroflow/issues/976), [#978](https://github.com/hydro-project/hydroflow/issues/978), [#981](https://github.com/hydro-project/hydroflow/issues/981), [#982](https://github.com/hydro-project/hydroflow/issues/982), [#984](https://github.com/hydro-project/hydroflow/issues/984), [#989](https://github.com/hydro-project/hydroflow/issues/989), [#991](https://github.com/hydro-project/hydroflow/issues/991), [#993](https://github.com/hydro-project/hydroflow/issues/993), [#995](https://github.com/hydro-project/hydroflow/issues/995)

### `hydroflow_plus` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1001](https://github.com/hydro-project/hydroflow/issues/1001)**
    - Disallow joining streams on different nodes ([`2addaed`](https://github.com/hydro-project/hydroflow/commit/2addaed8a8a441bff7acf9a0a265cc09483fd487))
 * **[#1003](https://github.com/hydro-project/hydroflow/issues/1003)**
    - Provide simpler API for launching and minimize dependencies ([`1edc5ae`](https://github.com/hydro-project/hydroflow/commit/1edc5ae5b5f70e1390183e8c8eb27eb0ab32196d))
 * **[#1004](https://github.com/hydro-project/hydroflow/issues/1004)**
    - Rewrite uses of alloc crate in bincode operators ([`bd2bf23`](https://github.com/hydro-project/hydroflow/commit/bd2bf233302e3638c8f4bc9c0460e1a47edc00aa))
 * **[#1006](https://github.com/hydro-project/hydroflow/issues/1006)**
    - Tweak naming of windowing operators ([`44a308f`](https://github.com/hydro-project/hydroflow/commit/44a308f77bddd67b5c51723ac39f3bc10af52553))
 * **[#1013](https://github.com/hydro-project/hydroflow/issues/1013)**
    - Improve API naming and polish docs ([`6eeb9be`](https://github.com/hydro-project/hydroflow/commit/6eeb9be9bc4136041a2855f650ae640c478b7fc9))
 * **[#1021](https://github.com/hydro-project/hydroflow/issues/1021)**
    - Push down persists and implement Pi example ([`af6e3be`](https://github.com/hydro-project/hydroflow/commit/af6e3be60fdb69ceec1613347910f4dd49980d34))
 * **[#1022](https://github.com/hydro-project/hydroflow/issues/1022)**
    - Add interleaved shortcut when sending from a cluster ([`5e6ebac`](https://github.com/hydro-project/hydroflow/commit/5e6ebac1a7f128227ae92a8c195235b27532e17a))
 * **[#1023](https://github.com/hydro-project/hydroflow/issues/1023)**
    - Implement keyed fold and reduce ([`73e9b68`](https://github.com/hydro-project/hydroflow/commit/73e9b68ec2f5b2627784addcce9fba684848bb55))
 * **[#1035](https://github.com/hydro-project/hydroflow/issues/1035)**
    - Persist cluster IDs for broadcast ([`88a1796`](https://github.com/hydro-project/hydroflow/commit/88a17967d0c9e681a04de4b5796f532f4833272c))
 * **[#1036](https://github.com/hydro-project/hydroflow/issues/1036)**
    - Add core negation operators ([`5a03ed4`](https://github.com/hydro-project/hydroflow/commit/5a03ed41548b5766b945efbd1eedb0dfceb714d9))
 * **[#899](https://github.com/hydro-project/hydroflow/issues/899)**
    - Add a functional surface syntax using staging ([`8b63568`](https://github.com/hydro-project/hydroflow/commit/8b635683e5ac3c4ed2d896ae88e2953db1c6312c))
 * **[#976](https://github.com/hydro-project/hydroflow/issues/976)**
    - Allow Hydroflow+ programs to emit multiple graphs ([`05fb135`](https://github.com/hydro-project/hydroflow/commit/05fb1353cf3e0e8c5da9522365150bd78bd3c5f8))
 * **[#978](https://github.com/hydro-project/hydroflow/issues/978)**
    - Add initial test using Hydro CLI from Hydroflow+ ([`e5bdd12`](https://github.com/hydro-project/hydroflow/commit/e5bdd12e32d6ea72fd91a55c12e09f07a0edaa5c))
 * **[#981](https://github.com/hydro-project/hydroflow/issues/981)**
    - Add preliminary `send_to` operator for multi-node graphs ([`27dabcf`](https://github.com/hydro-project/hydroflow/commit/27dabcf6878576dc3675788ce3381cb25116033a))
 * **[#982](https://github.com/hydro-project/hydroflow/issues/982)**
    - Auto-configure Hydro Deploy based on Hydroflow+ plans ([`9e27582`](https://github.com/hydro-project/hydroflow/commit/9e275824c88b24d060a7de5822e1359959b36b03))
 * **[#984](https://github.com/hydro-project/hydroflow/issues/984)**
    - Support building graphs for symmetric clusters in Hydroflow+ ([`174607d`](https://github.com/hydro-project/hydroflow/commit/174607d12277d7544d0f42890c9a5da2ff184df4))
 * **[#989](https://github.com/hydro-project/hydroflow/issues/989)**
    - Fix spelling of "propagate" ([`38411ea`](https://github.com/hydro-project/hydroflow/commit/38411ea007d4feb30dd16bdd1505802a111a67d1))
 * **[#991](https://github.com/hydro-project/hydroflow/issues/991)**
    - Require explicit batching for aggregation operators ([`26f4d6f`](https://github.com/hydro-project/hydroflow/commit/26f4d6f610b78a75c41b1ae63366d089ad08b322))
 * **[#993](https://github.com/hydro-project/hydroflow/issues/993)**
    - Split API for building single-node graphs ([`d288e51`](https://github.com/hydro-project/hydroflow/commit/d288e51f980577510bb2ed45c04554102c4f1e14))
 * **[#995](https://github.com/hydro-project/hydroflow/issues/995)**
    - Improve API naming and eliminate wire API for builders ([`b7aafd3`](https://github.com/hydro-project/hydroflow/commit/b7aafd3c97897db4bff62c4ab0b7480ef9a799e0))
 * **Uncategorized**
    - Release hydroflow_plus v0.5.1 ([`58d1d71`](https://github.com/hydro-project/hydroflow/commit/58d1d7166f026a8c7a08a23bc1d77045d7e5f2a9))
    - Release stageleft_macro v0.1.0, stageleft v0.1.0, hydroflow_plus v0.5.1 ([`1a48db5`](https://github.com/hydro-project/hydroflow/commit/1a48db5a1ba058a718ac777367bf6eba3a236b7c))
    - Actually committing empty CHANGELOG.md is required ([`3b36020`](https://github.com/hydro-project/hydroflow/commit/3b36020d16792f26da4df3c5b09652a4ab47ec4f))
    - Manually set lockstep-versioned crates (and `lattices`) to version `0.5.1` ([`1b555e5`](https://github.com/hydro-project/hydroflow/commit/1b555e57c8c812bed4d6495d2960cbf77fb0b3ef))
    - Add APIs for declaring external ports on clusters ([`7d930a2`](https://github.com/hydro-project/hydroflow/commit/7d930a2ccf656d3d6bc5db3e22eb63c5fd6d37d1))
</details>

