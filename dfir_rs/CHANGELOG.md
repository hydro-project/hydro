# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.15.0 (2025-11-25)

### Chore

 - <csr-id-d22507cf6064e4b9f6a02404d58f644cde3dcf0b/> update to rust 1.91.1
 - <csr-id-623ed9ea41a04b442a44f85208ca51e22fb743b6/> update back to Rust 1.90.0
 - <csr-id-bcef0557add258930a76900d6f2ce641c7f9b148/> pin Rust to 1.89
   GitHub Actions is going to be flip/flopping between 1.89 and 1.90, and
   we have users who need support on 1.89.
   
   Also updates precheck.bash to use nextest for _speed_.
 - <csr-id-e5d90803ae16993ac3db24a7795d0864abc4ac52/> update wasm
 - <csr-id-97426b8a7e3b3af8a58b4c44c768c3f48cd0ed71/> update pinned nightly to 2025-08-20, fix lints

### Documentation

 - <csr-id-2d0968777563a0129db07ca9068a1d6fecbacacb/> consistently refer to Hydro as a framework
 - <csr-id-273bc1e40e925343b8deed5e40a1879915a9e61b/> fix "DFIR Playground" link

### New Features

 - <csr-id-f7ecb53e1941f67e59bde32e94e9f320f4bf5410/> add `resolve_futures_blocking` for resolving async calls by blocking the subgraph
 - <csr-id-335ded3fc72c6525b3210f2750ea11a63d60117e/> `DemuxMap`, use in `hydro_deploy_integration`
 - <csr-id-7efe1dc660ab6c0c68762f71d4359f347cfe73b6/> `sinktools` crate
   this also converts `variadic` to use `#[no_std]`, and adds
   `feature="std"`, and fixes an issue causing trybuild tests to not run
   
   will replace `pusherator` in upcoming PR

### Bug Fixes

 - <csr-id-dd62edfed25e52669d0f9169cfcefbe278cbaf65/> fix code generation for `scan` as a push-operator
 - <csr-id-1b818e321805f3134b66359fc0a6c70599a71422/> update backtrace snapshots for Rust 1.90 and remove unused types
   The line number of `core::ops::function::FnOnce::call_once` in the
   standard library changed.
 - <csr-id-09c07701f03862f3a755420c202cacd5218cd114/> remove python udf support
 - <csr-id-4e84a3fc03af1eacf8af467e52407121f3b6ec9e/> [ci-full] fix ci issues
   - run docs/website on pr
   - use bash by default on windows
   - build website with nightly
   - increase test timeout to 45m
   - run docs with nightly
   - make rustfmt use native line endings instead of unix
   - fix compile-fail nightly stderr outputs
   - add the ability to include [ci-full] into the body/title of a PR to
   get it to run the full stable/nightly linux/windows test matrix.
 - <csr-id-c40876ec4bd3b31254d683e479b9a235f3d11f67/> refactor github actions workflows, make stable the default toolchain
 - <csr-id-669aefa679df02369ac51b2aa753a5908a76897b/> add trybuild stable tests
 - <csr-id-bdc54eba06f75c29f9393c69da6c07db0d602976/> snapshot tests don't depend on the parent crate name
 - <csr-id-d0570728bbdeae63571936d00d087673924842c3/> check dfir warnings in tests on non-nightly

### Other

 - <csr-id-806a6239a649e24fe10c3c90dd30bd18debd41d2/> ensure `hydro_build_utils` is published in the correct order

### Refactor

 - <csr-id-f4a26b3268a3fa4a6e907d33a3e5ac7529188f20/> Make `join_fused` use new `Accumulator` trait
   To make the codegen less magical.
   
   ---------

### Test

 - <csr-id-356292ed01d8ffa07838687f2e1e8e2b682cbc26/> rearrange `sort` and `anti_join` tests out of `surface_codegen`
   does not change any code, just moves it to new files
 - <csr-id-562d1081abef29175a8dc0bb7c3934114f6939e1/> ensure `surface_demux_enum` actually assert results

### Refactor (BREAKING)

 - <csr-id-9d943ac294a8735452f8535ad13767c60ce46ec7/> Make DFIR use `sinktools` for pushing to outputs [ci-bench]
   This allows DFIR to handle `dest_sink` directly instead of having to
   offload to a separate task, which causes latency and unwanted batching
   (on single threaded runtimes)
   
   `pusherator` crate is no longer depended upon and is on a path to removal as it has been
   replaced with `Sink`s
   
   Fixes some spanning bugs in codegen that using `Sink`s revealed
 - <csr-id-8535940a34dba130156eb3605ae56483586bb62a/> remove `demux` operator, replace usage with `demux_enum`
   This is in preparation of making DFIR support async `Sink`s. The
   overpowered demux API is incompatible with the more constrained
   mechanics of `Sink`s.
   
   ## Pull Request Overview
   
   This PR refactors the `dfir_rs` library by removing the `demux` operator
   and replacing its usage with the `demux_enum` operator, which provides
   better type safety and ergonomics.
   
   - Removes the `demux` operator implementation and related code
   - Updates examples to use `demux_enum` with enums for message handling
   - Removes all compile-fail tests specific to the `demux` operator
 - <csr-id-29027701471205a7c43e26ef2f8cee98663c578e/> superficially make subgraphs/operators asynchronous
   Subgraphs now are async (create a future when running) which must be
   awaited. However this PR does not actually change any
   subgraphs/operators, so all subgraphs/operators return `Poll::Ready(())`
   immediately

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 25 commits contributed to the release.
 - 25 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 24 unique issues were worked on: [#1997](https://github.com/hydro-project/hydro/issues/1997), [#2010](https://github.com/hydro-project/hydro/issues/2010), [#2020](https://github.com/hydro-project/hydro/issues/2020), [#2021](https://github.com/hydro-project/hydro/issues/2021), [#2022](https://github.com/hydro-project/hydro/issues/2022), [#2024](https://github.com/hydro-project/hydro/issues/2024), [#2028](https://github.com/hydro-project/hydro/issues/2028), [#2029](https://github.com/hydro-project/hydro/issues/2029), [#2038](https://github.com/hydro-project/hydro/issues/2038), [#2087](https://github.com/hydro-project/hydro/issues/2087), [#2091](https://github.com/hydro-project/hydro/issues/2091), [#2119](https://github.com/hydro-project/hydro/issues/2119), [#2122](https://github.com/hydro-project/hydro/issues/2122), [#2132](https://github.com/hydro-project/hydro/issues/2132), [#2134](https://github.com/hydro-project/hydro/issues/2134), [#2137](https://github.com/hydro-project/hydro/issues/2137), [#2147](https://github.com/hydro-project/hydro/issues/2147), [#2157](https://github.com/hydro-project/hydro/issues/2157), [#2163](https://github.com/hydro-project/hydro/issues/2163), [#2170](https://github.com/hydro-project/hydro/issues/2170), [#2281](https://github.com/hydro-project/hydro/issues/2281), [#2295](https://github.com/hydro-project/hydro/issues/2295), [#2309](https://github.com/hydro-project/hydro/issues/2309), [#2318](https://github.com/hydro-project/hydro/issues/2318)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1997](https://github.com/hydro-project/hydro/issues/1997)**
    - Fix "DFIR Playground" link ([`273bc1e`](https://github.com/hydro-project/hydro/commit/273bc1e40e925343b8deed5e40a1879915a9e61b))
 * **[#2010](https://github.com/hydro-project/hydro/issues/2010)**
    - Check dfir warnings in tests on non-nightly ([`d057072`](https://github.com/hydro-project/hydro/commit/d0570728bbdeae63571936d00d087673924842c3))
 * **[#2020](https://github.com/hydro-project/hydro/issues/2020)**
    - Consistently refer to Hydro as a framework ([`2d09687`](https://github.com/hydro-project/hydro/commit/2d0968777563a0129db07ca9068a1d6fecbacacb))
 * **[#2021](https://github.com/hydro-project/hydro/issues/2021)**
    - Snapshot tests don't depend on the parent crate name ([`bdc54eb`](https://github.com/hydro-project/hydro/commit/bdc54eba06f75c29f9393c69da6c07db0d602976))
 * **[#2022](https://github.com/hydro-project/hydro/issues/2022)**
    - Add trybuild stable tests ([`669aefa`](https://github.com/hydro-project/hydro/commit/669aefa679df02369ac51b2aa753a5908a76897b))
 * **[#2024](https://github.com/hydro-project/hydro/issues/2024)**
    - Update pinned nightly to 2025-08-20, fix lints ([`97426b8`](https://github.com/hydro-project/hydro/commit/97426b8a7e3b3af8a58b4c44c768c3f48cd0ed71))
 * **[#2028](https://github.com/hydro-project/hydro/issues/2028)**
    - Refactor github actions workflows, make stable the default toolchain ([`c40876e`](https://github.com/hydro-project/hydro/commit/c40876ec4bd3b31254d683e479b9a235f3d11f67))
 * **[#2029](https://github.com/hydro-project/hydro/issues/2029)**
    - [ci-full] fix ci issues ([`4e84a3f`](https://github.com/hydro-project/hydro/commit/4e84a3fc03af1eacf8af467e52407121f3b6ec9e))
 * **[#2038](https://github.com/hydro-project/hydro/issues/2038)**
    - Remove python udf support ([`09c0770`](https://github.com/hydro-project/hydro/commit/09c07701f03862f3a755420c202cacd5218cd114))
 * **[#2087](https://github.com/hydro-project/hydro/issues/2087)**
    - Update wasm ([`e5d9080`](https://github.com/hydro-project/hydro/commit/e5d90803ae16993ac3db24a7795d0864abc4ac52))
 * **[#2091](https://github.com/hydro-project/hydro/issues/2091)**
    - Superficially make subgraphs/operators asynchronous ([`2902770`](https://github.com/hydro-project/hydro/commit/29027701471205a7c43e26ef2f8cee98663c578e))
 * **[#2119](https://github.com/hydro-project/hydro/issues/2119)**
    - Remove `demux` operator, replace usage with `demux_enum` ([`8535940`](https://github.com/hydro-project/hydro/commit/8535940a34dba130156eb3605ae56483586bb62a))
 * **[#2122](https://github.com/hydro-project/hydro/issues/2122)**
    - Update backtrace snapshots for Rust 1.90 and remove unused types ([`1b818e3`](https://github.com/hydro-project/hydro/commit/1b818e321805f3134b66359fc0a6c70599a71422))
 * **[#2132](https://github.com/hydro-project/hydro/issues/2132)**
    - Ensure `surface_demux_enum` actually assert results ([`562d108`](https://github.com/hydro-project/hydro/commit/562d1081abef29175a8dc0bb7c3934114f6939e1))
 * **[#2134](https://github.com/hydro-project/hydro/issues/2134)**
    - Make DFIR use `sinktools` for pushing to outputs [ci-bench] ([`9d943ac`](https://github.com/hydro-project/hydro/commit/9d943ac294a8735452f8535ad13767c60ce46ec7))
 * **[#2137](https://github.com/hydro-project/hydro/issues/2137)**
    - Pin Rust to 1.89 ([`bcef055`](https://github.com/hydro-project/hydro/commit/bcef0557add258930a76900d6f2ce641c7f9b148))
 * **[#2147](https://github.com/hydro-project/hydro/issues/2147)**
    - Fix code generation for `scan` as a push-operator ([`dd62edf`](https://github.com/hydro-project/hydro/commit/dd62edfed25e52669d0f9169cfcefbe278cbaf65))
 * **[#2157](https://github.com/hydro-project/hydro/issues/2157)**
    - `sinktools` crate ([`7efe1dc`](https://github.com/hydro-project/hydro/commit/7efe1dc660ab6c0c68762f71d4359f347cfe73b6))
 * **[#2163](https://github.com/hydro-project/hydro/issues/2163)**
    - `DemuxMap`, use in `hydro_deploy_integration` ([`335ded3`](https://github.com/hydro-project/hydro/commit/335ded3fc72c6525b3210f2750ea11a63d60117e))
 * **[#2170](https://github.com/hydro-project/hydro/issues/2170)**
    - Update back to Rust 1.90.0 ([`623ed9e`](https://github.com/hydro-project/hydro/commit/623ed9ea41a04b442a44f85208ca51e22fb743b6))
 * **[#2281](https://github.com/hydro-project/hydro/issues/2281)**
    - Add `resolve_futures_blocking` for resolving async calls by blocking the subgraph ([`f7ecb53`](https://github.com/hydro-project/hydro/commit/f7ecb53e1941f67e59bde32e94e9f320f4bf5410))
 * **[#2295](https://github.com/hydro-project/hydro/issues/2295)**
    - Update to rust 1.91.1 ([`d22507c`](https://github.com/hydro-project/hydro/commit/d22507cf6064e4b9f6a02404d58f644cde3dcf0b))
 * **[#2309](https://github.com/hydro-project/hydro/issues/2309)**
    - Rearrange `sort` and `anti_join` tests out of `surface_codegen` ([`356292e`](https://github.com/hydro-project/hydro/commit/356292ed01d8ffa07838687f2e1e8e2b682cbc26))
 * **[#2318](https://github.com/hydro-project/hydro/issues/2318)**
    - Make `join_fused` use new `Accumulator` trait ([`f4a26b3`](https://github.com/hydro-project/hydro/commit/f4a26b3268a3fa4a6e907d33a3e5ac7529188f20))
 * **Uncategorized**
    - Ensure `hydro_build_utils` is published in the correct order ([`806a623`](https://github.com/hydro-project/hydro/commit/806a6239a649e24fe10c3c90dd30bd18debd41d2))
</details>

## 0.14.0 (2025-07-31)

<csr-id-59041df58be2de0717b851cc1c3355479cd722f2/>
<csr-id-cb54ace31866d68f424798f876f9e4e8ffd9d881/>
<csr-id-c3ccee6638f2e006f837fd6f946d1b942e40c144/>
<csr-id-09fc5f3d430e455a11fee054d7243c02ecb3c02d/>
<csr-id-de6c8ce3d258ecd1a2038e2a09d5ea8860e8ad42/>

### Documentation

 - <csr-id-ec1d8a0c1b91a0a7f1bb322e975d59c5bcbcf801/> add note to install nodejs

### New Features

 - <csr-id-bd1afdff5fd7b8dc6d2c567cd2659542a84c6216/> allow running generated binaries with single-threaded Tokio runtime
   Before, we had a janky architecture for establishing network connections
   which relied on blocking on futures outside an async context, which
   required a multi-threaded runtime. Now, we establish all connections
   before launching the DFIR code, so that no blocking is required there.
 - <csr-id-b58dfc899c67ee17a1818c484fa6cba7db3dd240/> add `scan` operator
 - <csr-id-99a8f1dfdde087578f25c19e502ad13e1d98a394/> Decoupling analysis
   A Gurobi license is required to run code that uses `hydro_optimize` (for ILP over decoupling decisions)

### Bug Fixes

 - <csr-id-5b5bbe57b54a5d038bc28c0e674ed68ca34245d1/> Revert anti join allocation
   Added unit test for Paxos compilation and non-negative throughtput
 - <csr-id-0d841a536e1ab58838136e6c33b2115325ec1541/> add type arguments to `anti_join_multiset`, `difference_multiset` to mitigate #1857
 - <csr-id-96ec97a324254929b4677e9e769dbccab917d767/> workaround to publish `example_test`

### Refactor

 - <csr-id-59041df58be2de0717b851cc1c3355479cd722f2/> minimize Tokio feature flags
   Now that `hydro_lang` no longer needs multi-threaded runtime, we can
   eliminate it from the features used in `trybuild` compilation. Minimizes
   Tokio features elsewhere too.
 - <csr-id-cb54ace31866d68f424798f876f9e4e8ffd9d881/> move example testing code into separate crate
   To prep for testing of hydro_deploy #1374 #1810

### Test

 - <csr-id-c3ccee6638f2e006f837fd6f946d1b942e40c144/> test some hydro examples on localhost, fix #1374

### Chore (BREAKING)

 - <csr-id-09fc5f3d430e455a11fee054d7243c02ecb3c02d/> move datalog from repo, remove datalog playground from web, #1809
   #1809
   
   moved to https://github.com/hydro-project/dfir-datalog
   
   tests moved in https://github.com/hydro-project/dfir-datalog/pull/1
   
   Removes dedalus examples in `hydro_cli_examples`
   
   Changes pinned nightly rust version from 2024-04-05 to 2024-04-04 as the
   former did not have intel mac support.

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
 - <csr-id-9bb9d1f3a0108a4789de6065af9e644c47601b9f/> display loops in graph visualizations, refactor, fix #1699
   Adds loops to display, new `GraphWrite.no_loops` option.
   
   Refactors how the heirarchy of `GraphWrite` items is handled to be
   simpler.

### Refactor (BREAKING)

 - <csr-id-de6c8ce3d258ecd1a2038e2a09d5ea8860e8ad42/> use direct `&dyn Any` upcasting for Rust 1.86, update pyo3, fix #1821

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 16 commits contributed to the release.
 - 110 days passed between releases.
 - 14 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 13 unique issues were worked on: [#1825](https://github.com/hydro-project/hydro/issues/1825), [#1837](https://github.com/hydro-project/hydro/issues/1837), [#1847](https://github.com/hydro-project/hydro/issues/1847), [#1848](https://github.com/hydro-project/hydro/issues/1848), [#1851](https://github.com/hydro-project/hydro/issues/1851), [#1858](https://github.com/hydro-project/hydro/issues/1858), [#1859](https://github.com/hydro-project/hydro/issues/1859), [#1860](https://github.com/hydro-project/hydro/issues/1860), [#1911](https://github.com/hydro-project/hydro/issues/1911), [#1912](https://github.com/hydro-project/hydro/issues/1912), [#1929](https://github.com/hydro-project/hydro/issues/1929), [#1938](https://github.com/hydro-project/hydro/issues/1938), [#1939](https://github.com/hydro-project/hydro/issues/1939)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1825](https://github.com/hydro-project/hydro/issues/1825)**
    - Use direct `&dyn Any` upcasting for Rust 1.86, update pyo3, fix #1821 ([`de6c8ce`](https://github.com/hydro-project/hydro/commit/de6c8ce3d258ecd1a2038e2a09d5ea8860e8ad42))
 * **[#1837](https://github.com/hydro-project/hydro/issues/1837)**
    - Move datalog from repo, remove datalog playground from web, #1809 ([`09fc5f3`](https://github.com/hydro-project/hydro/commit/09fc5f3d430e455a11fee054d7243c02ecb3c02d))
 * **[#1847](https://github.com/hydro-project/hydro/issues/1847)**
    - Move example testing code into separate crate ([`cb54ace`](https://github.com/hydro-project/hydro/commit/cb54ace31866d68f424798f876f9e4e8ffd9d881))
 * **[#1848](https://github.com/hydro-project/hydro/issues/1848)**
    - Test some hydro examples on localhost, fix #1374 ([`c3ccee6`](https://github.com/hydro-project/hydro/commit/c3ccee6638f2e006f837fd6f946d1b942e40c144))
 * **[#1851](https://github.com/hydro-project/hydro/issues/1851)**
    - Display loops in graph visualizations, refactor, fix #1699 ([`9bb9d1f`](https://github.com/hydro-project/hydro/commit/9bb9d1f3a0108a4789de6065af9e644c47601b9f))
 * **[#1858](https://github.com/hydro-project/hydro/issues/1858)**
    - Add type arguments to `anti_join_multiset`, `difference_multiset` to mitigate #1857 ([`0d841a5`](https://github.com/hydro-project/hydro/commit/0d841a536e1ab58838136e6c33b2115325ec1541))
 * **[#1859](https://github.com/hydro-project/hydro/issues/1859)**
    - Decoupling analysis ([`99a8f1d`](https://github.com/hydro-project/hydro/commit/99a8f1dfdde087578f25c19e502ad13e1d98a394))
 * **[#1860](https://github.com/hydro-project/hydro/issues/1860)**
    - Revert anti join allocation ([`5b5bbe5`](https://github.com/hydro-project/hydro/commit/5b5bbe57b54a5d038bc28c0e674ed68ca34245d1))
 * **[#1911](https://github.com/hydro-project/hydro/issues/1911)**
    - Re-add loop lifetimes for anti_join_multiset, tests, remove MonotonicMap, fix #1830, fix #1823 ([`d6ae619`](https://github.com/hydro-project/hydro/commit/d6ae619060339eb3dac5bec17d384430e3588093))
 * **[#1912](https://github.com/hydro-project/hydro/issues/1912)**
    - Add note to install nodejs ([`ec1d8a0`](https://github.com/hydro-project/hydro/commit/ec1d8a0c1b91a0a7f1bb322e975d59c5bcbcf801))
 * **[#1929](https://github.com/hydro-project/hydro/issues/1929)**
    - Add `scan` operator ([`b58dfc8`](https://github.com/hydro-project/hydro/commit/b58dfc899c67ee17a1818c484fa6cba7db3dd240))
 * **[#1938](https://github.com/hydro-project/hydro/issues/1938)**
    - Allow running generated binaries with single-threaded Tokio runtime ([`bd1afdf`](https://github.com/hydro-project/hydro/commit/bd1afdff5fd7b8dc6d2c567cd2659542a84c6216))
 * **[#1939](https://github.com/hydro-project/hydro/issues/1939)**
    - Minimize Tokio feature flags ([`59041df`](https://github.com/hydro-project/hydro/commit/59041df58be2de0717b851cc1c3355479cd722f2))
 * **Uncategorized**
    - Release example_test v0.0.0, dfir_rs v0.14.0, hydro_deploy v0.14.0, hydro_lang v0.14.0, hydro_optimize v0.13.0, hydro_std v0.14.0 ([`5f69ee0`](https://github.com/hydro-project/hydro/commit/5f69ee080a9e257bc07cdc4deda90ce5525a3d0e))
    - Workaround to publish `example_test` ([`96ec97a`](https://github.com/hydro-project/hydro/commit/96ec97a324254929b4677e9e769dbccab917d767))
    - Release dfir_lang v0.14.0, dfir_macro v0.14.0, hydro_deploy_integration v0.14.0, lattices_macro v0.5.10, variadics_macro v0.6.1, dfir_rs v0.14.0, hydro_deploy v0.14.0, hydro_lang v0.14.0, hydro_optimize v0.13.0, hydro_std v0.14.0, safety bump 6 crates ([`0683595`](https://github.com/hydro-project/hydro/commit/06835950c12884d661100c13f73ad23a98bfad9f))
</details>

## 0.13.0 (2025-04-11)

<csr-id-3aec2f739acd0a2305f99fcbde4c14bc1cd53e7a/>
<csr-id-b07926f0b1007875eebf3fdc49223aa0cc0ed035/>
<csr-id-2fdd5da1cf902a0c2f99cf8770bb48f9e046e38f/>
<csr-id-304a8efd2213b31ff80d50925ebda177621dd6ee/>

### New Features

 - <csr-id-127df13b7d7fc12d5265f1d7517f3b66a774dc07/> add resolve_futures and resolve_futures_ordered APIs

### Bug Fixes

 - <csr-id-40075b198b13cb6b4804633b76c96c520394fa71/> fix loop hooks triggered too often, implement lifetimes for `zip`

### Other

 - <csr-id-3aec2f739acd0a2305f99fcbde4c14bc1cd53e7a/> adjust env vars, make DFIR printed span paths relative & re-enable most trybuild tests
   * Add `PYO3_USE_ABI3_FORWARD_COMPATIBILITY`, move
   `RUST_LOG`/`RUST_BACKTRACE` env vars to `.cargo/config.toml` (removed
   from `.vscode/settings.json`). Will now apply in general, so no need to
   specify everywhere.
   * Makes display of compiler spans' `source_file()` in `dfir_lang`
   relative to `DFIR_BASE_DIR` (new) or `CARGO_MANIFEST_DIR`, if possible.
   * Include `rust-src` `rustup` component to make trybuild error output
   more consistent:
   https://github.com/dtolnay/trybuild?tab=readme-ov-file#troubleshooting
   * Almost all `dfir_rs` trybuild test are re-enabled (except a couple
   obsolete ones are deleted, and two have inconsistent printing)
   * Updates docs build to remove old operators

### Refactor

 - <csr-id-b07926f0b1007875eebf3fdc49223aa0cc0ed035/> update examples to use template, fix #1123

### New Features (BREAKING)

 - <csr-id-fbaab5b12c0c661ee08d8ded6862a38834ba62ae/> loop lifetimes for `anti_join_multiset`, tests, remove `MonotonicMap`, fix #1830, fix #1823
   Also implements loop lifetimes for `difference_multiset` which uses the
   `anti_join_multiset` codegen.
   
   Updates tests for `difference`, `difference_multiset`, `anti_join`, and
   `anti_join_multiset`
 - <csr-id-dfb7a1b5ad47f03822e9b7cae7dae81914b305e2/> don't pull in dfir_rs during the compilation stage
   Because `hydro_lang` is responsible for _generating_ DFIR code, it
   doesn't actually need to depend on the runtime (`dfir_rs`), other than
   when it is used in the (legacy) macro mode or when we want to include
   utilities for runtime logic (`resource_measurement`). This sticks those
   pieces under feature flags and makes `dfir_rs` an optional dependency,
   which reduces the compile tree for crates like `hydro_test`.

### Refactor (BREAKING)

 - <csr-id-2fdd5da1cf902a0c2f99cf8770bb48f9e046e38f/> loop machinery, add `'loop` lifetimes, fix #1745
 - <csr-id-304a8efd2213b31ff80d50925ebda177621dd6ee/> use slotvec for state, disallow state removal #1745
   Refactor in prep for #1745
   
   Instead of each state having an optional end-of-tick reset hook, instead
   each state will be tied to a particular lifespan (loop iteration, loop
   execution, tick, static) for its reset hook. The scheduler will maintain
   various lists of state for each lifespan, and removing state makes
   maintaining those lists just a tiny bit harder. Since we have never
   removed state (and its against the spirit of monotonicity), its easier
   to just remove that unused functionality

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 9 commits contributed to the release.
 - 27 days passed between releases.
 - 8 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 8 unique issues were worked on: [#1741](https://github.com/hydro-project/hydro/issues/1741), [#1795](https://github.com/hydro-project/hydro/issues/1795), [#1797](https://github.com/hydro-project/hydro/issues/1797), [#1808](https://github.com/hydro-project/hydro/issues/1808), [#1815](https://github.com/hydro-project/hydro/issues/1815), [#1822](https://github.com/hydro-project/hydro/issues/1822), [#1833](https://github.com/hydro-project/hydro/issues/1833), [#1835](https://github.com/hydro-project/hydro/issues/1835)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1741](https://github.com/hydro-project/hydro/issues/1741)**
    - Add resolve_futures and resolve_futures_ordered APIs ([`127df13`](https://github.com/hydro-project/hydro/commit/127df13b7d7fc12d5265f1d7517f3b66a774dc07))
 * **[#1795](https://github.com/hydro-project/hydro/issues/1795)**
    - Adjust env vars, make DFIR printed span paths relative & re-enable most trybuild tests ([`3aec2f7`](https://github.com/hydro-project/hydro/commit/3aec2f739acd0a2305f99fcbde4c14bc1cd53e7a))
 * **[#1797](https://github.com/hydro-project/hydro/issues/1797)**
    - Don't pull in dfir_rs during the compilation stage ([`dfb7a1b`](https://github.com/hydro-project/hydro/commit/dfb7a1b5ad47f03822e9b7cae7dae81914b305e2))
 * **[#1808](https://github.com/hydro-project/hydro/issues/1808)**
    - Update examples to use template, fix #1123 ([`b07926f`](https://github.com/hydro-project/hydro/commit/b07926f0b1007875eebf3fdc49223aa0cc0ed035))
 * **[#1815](https://github.com/hydro-project/hydro/issues/1815)**
    - Use slotvec for state, disallow state removal #1745 ([`304a8ef`](https://github.com/hydro-project/hydro/commit/304a8efd2213b31ff80d50925ebda177621dd6ee))
 * **[#1822](https://github.com/hydro-project/hydro/issues/1822)**
    - Loop machinery, add `'loop` lifetimes, fix #1745 ([`2fdd5da`](https://github.com/hydro-project/hydro/commit/2fdd5da1cf902a0c2f99cf8770bb48f9e046e38f))
 * **[#1833](https://github.com/hydro-project/hydro/issues/1833)**
    - Fix loop hooks triggered too often, implement lifetimes for `zip` ([`40075b1`](https://github.com/hydro-project/hydro/commit/40075b198b13cb6b4804633b76c96c520394fa71))
 * **[#1835](https://github.com/hydro-project/hydro/issues/1835)**
    - Loop lifetimes for `anti_join_multiset`, tests, remove `MonotonicMap`, fix #1830, fix #1823 ([`fbaab5b`](https://github.com/hydro-project/hydro/commit/fbaab5b12c0c661ee08d8ded6862a38834ba62ae))
 * **Uncategorized**
    - Release dfir_lang v0.13.0, dfir_datalog_core v0.13.0, dfir_datalog v0.13.0, dfir_macro v0.13.0, hydro_deploy_integration v0.13.0, dfir_rs v0.13.0, hydro_deploy v0.13.0, hydro_lang v0.13.0, hydro_std v0.13.0, hydro_cli v0.13.0, safety bump 8 crates ([`400fd8f`](https://github.com/hydro-project/hydro/commit/400fd8f2e8cada253f54980e7edce0631be70a82))
</details>

## 0.12.1 (2025-03-15)

<csr-id-260902b210378af5291ec71a574256d7a5bcb463/>
<csr-id-f14174e92875b3264ef811effd954ed76cb3c948/>
<csr-id-8058d2d6fe45e0286feb8ad48a44b1228f56d9bc/>
<csr-id-056ac62611319b7bd10a751d7e231423a1b8dc4e/>
<csr-id-7dd71d67da162d2e4f3043b271a52037a3c983c0/>
<csr-id-fb8393ae61091b04c0d29692aab097573b8db5e8/>

### Chore

 - <csr-id-260902b210378af5291ec71a574256d7a5bcb463/> set `hydro_deploy_integration` to release as `0.12.1`
 - <csr-id-f14174e92875b3264ef811effd954ed76cb3c948/> update pinned nightly to 2025-03-10, clippy cleanups

### Chore

 - <csr-id-fb8393ae61091b04c0d29692aab097573b8db5e8/> attempt to fix releasing of `include_mdtests`, `dfir_rs`

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

### Refactor (BREAKING)

 - <csr-id-7dd71d67da162d2e4f3043b271a52037a3c983c0/> remove "hydroflow" for `hydro_deploy_integration`, `hydro_deploy::rust_crate`, fix #1712
   Opted to use `RustCrate` as the replacement prefix with the expectation
   that @shadaj may have a more coincise name in mind?

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 9 commits contributed to the release.
 - 7 days passed between releases.
 - 7 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 5 unique issues were worked on: [#1768](https://github.com/hydro-project/hydro/issues/1768), [#1769](https://github.com/hydro-project/hydro/issues/1769), [#1773](https://github.com/hydro-project/hydro/issues/1773), [#1785](https://github.com/hydro-project/hydro/issues/1785), [#1787](https://github.com/hydro-project/hydro/issues/1787)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1768](https://github.com/hydro-project/hydro/issues/1768)**
    - Update pinned nightly to 2025-03-10, clippy cleanups ([`f14174e`](https://github.com/hydro-project/hydro/commit/f14174e92875b3264ef811effd954ed76cb3c948))
 * **[#1769](https://github.com/hydro-project/hydro/issues/1769)**
    - Move markdown doctesting macro into own crate, fix #1651 ([`8058d2d`](https://github.com/hydro-project/hydro/commit/8058d2d6fe45e0286feb8ad48a44b1228f56d9bc))
 * **[#1773](https://github.com/hydro-project/hydro/issues/1773)**
    - Remove "hydroflow" for `hydro_deploy_integration`, `hydro_deploy::rust_crate`, fix #1712 ([`7dd71d6`](https://github.com/hydro-project/hydro/commit/7dd71d67da162d2e4f3043b271a52037a3c983c0))
 * **[#1785](https://github.com/hydro-project/hydro/issues/1785)**
    - Cleanup old clippy lints, remove deprecated `relalg` crate ([`056ac62`](https://github.com/hydro-project/hydro/commit/056ac62611319b7bd10a751d7e231423a1b8dc4e))
 * **[#1787](https://github.com/hydro-project/hydro/issues/1787)**
    - Demote python deploy docs, fix docsrs configs, fix #1392, fix #1629 ([`b235a42`](https://github.com/hydro-project/hydro/commit/b235a42a3071e55da7b09bdc8bc710b18e0fe053))
 * **Uncategorized**
    - Release include_mdtests v0.0.0, dfir_rs v0.12.1, hydro_deploy v0.12.1, hydro_lang v0.12.1, hydro_std v0.12.1, hydro_cli v0.12.1 ([`faf0d3e`](https://github.com/hydro-project/hydro/commit/faf0d3ed9f172275f2e2f219c5ead1910c209a36))
    - Attempt to fix releasing of `include_mdtests`, `dfir_rs` ([`fb8393a`](https://github.com/hydro-project/hydro/commit/fb8393ae61091b04c0d29692aab097573b8db5e8))
    - Release dfir_lang v0.12.1, dfir_datalog_core v0.12.1, dfir_datalog v0.12.1, dfir_macro v0.12.1, hydro_deploy_integration v0.12.1, lattices v0.6.1, pusherator v0.0.12, dfir_rs v0.12.1, hydro_deploy v0.12.1, hydro_lang v0.12.1, hydro_std v0.12.1, hydro_cli v0.12.1 ([`23221b5`](https://github.com/hydro-project/hydro/commit/23221b53b30918707ddaa85529d04cd7919166b4))
    - Set `hydro_deploy_integration` to release as `0.12.1` ([`260902b`](https://github.com/hydro-project/hydro/commit/260902b210378af5291ec71a574256d7a5bcb463))
</details>

## 0.12.0 (2025-03-08)

<csr-id-49a387d4a21f0763df8ec94de73fb953c9cd333a/>
<csr-id-2fd6aa7417dfa29f389c04c5b9674b80bfed6cf2/>
<csr-id-be50e86bd70b0b451bc906de7363a9a783aeaf25/>
<csr-id-ddbc5990647d567c941bdfb16606842fc93a08be/>
<csr-id-c293cca6855695107e9cef5c5df99fb04a571934/>
<csr-id-2e4f6b828e3d54ee6b1753c1a814006a3d0b3e34/>
<csr-id-efb31e159ef107b26657d8550d8c2d445a8b6e4b/>
<csr-id-8b3b60812d9f561cb7f59120993fbf2e23191e2b/>
<csr-id-c1983308743d912e5bf2583b7cccbb47d8a8b5d1/>
<csr-id-3343fe2d58d8b7a7aa2766bfba9fbb4955114706/>
<csr-id-44fb2806cf2d165d86695910f4755e0944c11832/>
<csr-id-dc51d94d390672bd687a7fdc9c826a778bdf0256/>
<csr-id-3c5bb05487a25345ea2f70b5a1ffbe74a216c684/>
<csr-id-e0d9b26709011a2d5d56cc3f86a3ee640983e75a/>
<csr-id-3966d9063dae52e65b077321e0bd1150f2b0c3f1/>
<csr-id-bf4b40151f98439570ac81ee6d23440a629b972f/>
<csr-id-8fc582d54ebc7dc974d2fd849b9de59134c1428d/>
<csr-id-3f76e91766a0bd9e61f11f9013d76f688467fb5e/>
<csr-id-fd85262930c678601a80c080fb79778675124964/>
<csr-id-e5c2e23359055f41492344edf19efbe3f2afd7ce/>
<csr-id-5cd0a9625822620dcc99b99356edfecbf0549497/>
<csr-id-41ef00719671b087062628462a947c211d0d765c/>
<csr-id-df804ec1ec2a9370c8e6e5f7db9ee1e316677a35/>
<csr-id-ec3795a678d261a38085405b6e9bfea943dafefb/>
<csr-id-8f4426089dcbbe5d1098f89e367c7be49a03e401/>
<csr-id-c3eedc6b44367519b6055bcdf7366b0a15efb17d/>

### Chore

 - <csr-id-49a387d4a21f0763df8ec94de73fb953c9cd333a/> upgrade to Rust 2024 edition
   - Updates `Cargo.toml` to use new shared workspace keys
   - Updates lint settings (in workspace `Cargo.toml`)
   - `rustfmt` has changed slightly, resulting in a big diff - there are no
   actual code changes
   - Adds a script to `rustfmt` the template src files
 - <csr-id-2fd6aa7417dfa29f389c04c5b9674b80bfed6cf2/> update pinned nightly to 2025-02-10, cleanups for clippy
 - <csr-id-be50e86bd70b0b451bc906de7363a9a783aeaf25/> remove unused cross_join, fix #1616

### Refactor (BREAKING)

 - <csr-id-e0d9b26709011a2d5d56cc3f86a3ee640983e75a/> require semicolons after loop braces, fix #1726
   Is better for syntax highlighting.

### Chore (BREAKING)

 - <csr-id-3966d9063dae52e65b077321e0bd1150f2b0c3f1/> use DFIR name instead of Hydroflow in some places, fix #1644
   Fix partially #1712
   
   * Renames `WriteContextArgs.hydroflow` to `WriteContextArgs.df_ident`
   for DFIR operator codegen
   * Removes some dead code/files
 - <csr-id-bf4b40151f98439570ac81ee6d23440a629b972f/> switch transducer -> process (fix #1572)

### Test

 - <csr-id-8fc582d54ebc7dc974d2fd849b9de59134c1428d/> add globbing utility for testing Markdown as doctests

### Style

 - <csr-id-3f76e91766a0bd9e61f11f9013d76f688467fb5e/> fix all unexpected cfgs
   Testing in https://github.com/MingweiSamuel/hydroflow
 - <csr-id-fd85262930c678601a80c080fb79778675124964/> clippy cleanups for latest stable rust

### Refactor

 - <csr-id-e5c2e23359055f41492344edf19efbe3f2afd7ce/> unchecked casts for handoffs & state, pedantic version
 - <csr-id-5cd0a9625822620dcc99b99356edfecbf0549497/> enable lints, cleanups for Rust 2024 #1732
 - <csr-id-41ef00719671b087062628462a947c211d0d765c/> improve naming of types involved with networking
   I got confused looking at my own code for client-server handshakes (in
   preparation for external client support). Time to do some renaming!
 - <csr-id-df804ec1ec2a9370c8e6e5f7db9ee1e316677a35/> use `SlotMap`-esque for Subgraph & Handoff datastructure
   Previously we used wrapper types around usize indexes, but any accesses
   into the `Vec` required unwrapping the usize value. This adds a proper
   wrapper class called `SlotVec` which has an API based on
   [`SlotMap`](https://docs.rs/slotmap/latest/slotmap/struct.SlotMap.html)
   with the main difference being that `SlotVec` does not support removals
   and therefore doesn't need the (very slight) overhead of generational
   indices

### Chore

 - <csr-id-ec3795a678d261a38085405b6e9bfea943dafefb/> upgrade to Rust 2024 edition
   - Updates `Cargo.toml` to use new shared workspace keys
   - Updates lint settings (in workspace `Cargo.toml`)
   - `rustfmt` has changed slightly, resulting in a big diff - there are no
   actual code changes
   - Adds a script to `rustfmt` the template src files
 - <csr-id-8f4426089dcbbe5d1098f89e367c7be49a03e401/> update pinned nightly to 2025-02-10, cleanups for clippy
 - <csr-id-c3eedc6b44367519b6055bcdf7366b0a15efb17d/> remove unused cross_join, fix #1616

### Documentation

 - <csr-id-f8313b018f6a1101935e4c06abbe5af3aafb400c/> fix broken links, fix #1613
 - <csr-id-d45273943b0ca087b05f0fe4331b12cbe2ff4e90/> fix broken links, fix #1613

### New Features

 - <csr-id-7009a042837e4d8bbde82b680ee6779d9dec062b/> `next_iteration()` operator, `all_iterations()` un-windowing op, k-means example, fix #1612, fix #1731
 - <csr-id-1ba45050f02a9f40860d7a52f144d35aebec031e/> add basic `_counter()` op for cardinality metrics, fix #1729
   This is the simplest possible implementation, there's a lot of room for
   improvement with a metrics system built into the runtime, but keeping it
   simple for now.
 - <csr-id-a72f7d56a720d56f6e5fd08e9dc6f50a2f302c57/> loop iteration counter, fix #1622
 - <csr-id-ee280c972848170783d105ba047b26dfa7249f24/> `loop {` scheduler
   Fix https://github.com/hydro-project/hydro/issues/1580
 - <csr-id-9aee8b770116b1fc70263b9fab7176bfa9ffe73f/> `next_iteration()` operator, `all_iterations()` un-windowing op, k-means example, fix #1612, fix #1731
 - <csr-id-11e07ead7e7b3fb0b552745d3332e0f6804a30a5/> add basic `_counter()` op for cardinality metrics, fix #1729
   This is the simplest possible implementation, there's a lot of room for
   improvement with a metrics system built into the runtime, but keeping it
   simple for now.
 - <csr-id-17bb5fa5a2b660a1202d3fa1c64fa606e1605f3a/> loop iteration counter, fix #1622
 - <csr-id-206be0e0bb5d2ad8d19c502d6ac13e0ca1fed552/> `loop {` scheduler
   Fix https://github.com/hydro-project/hydro/issues/1580

### Bug Fixes

 - <csr-id-a7d4d473b49d67818290a46a822b4e30e83478ff/> update stateful operators to be 'stateless' in loops, fix #1618
   eventually, when the old 'tick/'static machinery is removed, we can unify under one set of lifetimes
 - <csr-id-247a21d9617d44f13755c696224a6942afb9922a/> update stateful operators to be 'stateless' in loops, fix #1618
   eventually, when the old 'tick/'static machinery is removed, we can unify under one set of lifetimes

### Refactor

 - <csr-id-ddbc5990647d567c941bdfb16606842fc93a08be/> unchecked casts for handoffs & state, pedantic version
 - <csr-id-c293cca6855695107e9cef5c5df99fb04a571934/> enable lints, cleanups for Rust 2024 #1732
 - <csr-id-2e4f6b828e3d54ee6b1753c1a814006a3d0b3e34/> improve naming of types involved with networking
   I got confused looking at my own code for client-server handshakes (in
   preparation for external client support). Time to do some renaming!
 - <csr-id-efb31e159ef107b26657d8550d8c2d445a8b6e4b/> use `SlotMap`-esque for Subgraph & Handoff datastructure
   Previously we used wrapper types around usize indexes, but any accesses
   into the `Vec` required unwrapping the usize value. This adds a proper
   wrapper class called `SlotVec` which has an API based on
   [`SlotMap`](https://docs.rs/slotmap/latest/slotmap/struct.SlotMap.html)
   with the main difference being that `SlotVec` does not support removals
   and therefore doesn't need the (very slight) overhead of generational
   indices

### Style

 - <csr-id-8b3b60812d9f561cb7f59120993fbf2e23191e2b/> fix all unexpected cfgs
   Testing in https://github.com/MingweiSamuel/hydroflow
 - <csr-id-c1983308743d912e5bf2583b7cccbb47d8a8b5d1/> clippy cleanups for latest stable rust

### Test

 - <csr-id-3343fe2d58d8b7a7aa2766bfba9fbb4955114706/> add globbing utility for testing Markdown as doctests

### Chore (BREAKING)

 - <csr-id-44fb2806cf2d165d86695910f4755e0944c11832/> use DFIR name instead of Hydroflow in some places, fix #1644
   Fix partially #1712
   
   * Renames `WriteContextArgs.hydroflow` to `WriteContextArgs.df_ident`
   for DFIR operator codegen
   * Removes some dead code/files
 - <csr-id-dc51d94d390672bd687a7fdc9c826a778bdf0256/> switch transducer -> process (fix #1572)

### Refactor (BREAKING)

 - <csr-id-3c5bb05487a25345ea2f70b5a1ffbe74a216c684/> require semicolons after loop braces, fix #1726
   Is better for syntax highlighting.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 20 commits contributed to the release.
 - 74 days passed between releases.
 - 19 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 19 unique issues were worked on: [#1637](https://github.com/hydro-project/hydro/issues/1637), [#1640](https://github.com/hydro-project/hydro/issues/1640), [#1648](https://github.com/hydro-project/hydro/issues/1648), [#1649](https://github.com/hydro-project/hydro/issues/1649), [#1684](https://github.com/hydro-project/hydro/issues/1684), [#1686](https://github.com/hydro-project/hydro/issues/1686), [#1688](https://github.com/hydro-project/hydro/issues/1688), [#1692](https://github.com/hydro-project/hydro/issues/1692), [#1704](https://github.com/hydro-project/hydro/issues/1704), [#1713](https://github.com/hydro-project/hydro/issues/1713), [#1714](https://github.com/hydro-project/hydro/issues/1714), [#1715](https://github.com/hydro-project/hydro/issues/1715), [#1725](https://github.com/hydro-project/hydro/issues/1725), [#1727](https://github.com/hydro-project/hydro/issues/1727), [#1733](https://github.com/hydro-project/hydro/issues/1733), [#1737](https://github.com/hydro-project/hydro/issues/1737), [#1747](https://github.com/hydro-project/hydro/issues/1747), [#1757](https://github.com/hydro-project/hydro/issues/1757), [#1759](https://github.com/hydro-project/hydro/issues/1759)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1637](https://github.com/hydro-project/hydro/issues/1637)**
    - Use `SlotMap`-esque for Subgraph & Handoff datastructure ([`df804ec`](https://github.com/hydro-project/hydro/commit/df804ec1ec2a9370c8e6e5f7db9ee1e316677a35))
 * **[#1640](https://github.com/hydro-project/hydro/issues/1640)**
    - Clippy cleanups for latest stable rust ([`fd85262`](https://github.com/hydro-project/hydro/commit/fd85262930c678601a80c080fb79778675124964))
 * **[#1648](https://github.com/hydro-project/hydro/issues/1648)**
    - Fix all unexpected cfgs ([`3f76e91`](https://github.com/hydro-project/hydro/commit/3f76e91766a0bd9e61f11f9013d76f688467fb5e))
 * **[#1649](https://github.com/hydro-project/hydro/issues/1649)**
    - Add globbing utility for testing Markdown as doctests ([`8fc582d`](https://github.com/hydro-project/hydro/commit/8fc582d54ebc7dc974d2fd849b9de59134c1428d))
 * **[#1684](https://github.com/hydro-project/hydro/issues/1684)**
    - Switch transducer -> process (fix #1572) ([`bf4b401`](https://github.com/hydro-project/hydro/commit/bf4b40151f98439570ac81ee6d23440a629b972f))
 * **[#1686](https://github.com/hydro-project/hydro/issues/1686)**
    - Fix broken links, fix #1613 ([`d452739`](https://github.com/hydro-project/hydro/commit/d45273943b0ca087b05f0fe4331b12cbe2ff4e90))
 * **[#1688](https://github.com/hydro-project/hydro/issues/1688)**
    - Remove unused cross_join, fix #1616 ([`c3eedc6`](https://github.com/hydro-project/hydro/commit/c3eedc6b44367519b6055bcdf7366b0a15efb17d))
 * **[#1692](https://github.com/hydro-project/hydro/issues/1692)**
    - `loop {` scheduler ([`206be0e`](https://github.com/hydro-project/hydro/commit/206be0e0bb5d2ad8d19c502d6ac13e0ca1fed552))
 * **[#1704](https://github.com/hydro-project/hydro/issues/1704)**
    - Update pinned nightly to 2025-02-10, cleanups for clippy ([`8f44260`](https://github.com/hydro-project/hydro/commit/8f4426089dcbbe5d1098f89e367c7be49a03e401))
 * **[#1713](https://github.com/hydro-project/hydro/issues/1713)**
    - Use DFIR name instead of Hydroflow in some places, fix #1644 ([`3966d90`](https://github.com/hydro-project/hydro/commit/3966d9063dae52e65b077321e0bd1150f2b0c3f1))
 * **[#1714](https://github.com/hydro-project/hydro/issues/1714)**
    - Loop iteration counter, fix #1622 ([`17bb5fa`](https://github.com/hydro-project/hydro/commit/17bb5fa5a2b660a1202d3fa1c64fa606e1605f3a))
 * **[#1715](https://github.com/hydro-project/hydro/issues/1715)**
    - Improve naming of types involved with networking ([`41ef007`](https://github.com/hydro-project/hydro/commit/41ef00719671b087062628462a947c211d0d765c))
 * **[#1725](https://github.com/hydro-project/hydro/issues/1725)**
    - `next_iteration()` operator, `all_iterations()` un-windowing op, k-means example, fix #1612, fix #1731 ([`9aee8b7`](https://github.com/hydro-project/hydro/commit/9aee8b770116b1fc70263b9fab7176bfa9ffe73f))
 * **[#1727](https://github.com/hydro-project/hydro/issues/1727)**
    - Require semicolons after loop braces, fix #1726 ([`e0d9b26`](https://github.com/hydro-project/hydro/commit/e0d9b26709011a2d5d56cc3f86a3ee640983e75a))
 * **[#1733](https://github.com/hydro-project/hydro/issues/1733)**
    - Add basic `_counter()` op for cardinality metrics, fix #1729 ([`11e07ea`](https://github.com/hydro-project/hydro/commit/11e07ead7e7b3fb0b552745d3332e0f6804a30a5))
 * **[#1737](https://github.com/hydro-project/hydro/issues/1737)**
    - Enable lints, cleanups for Rust 2024 #1732 ([`5cd0a96`](https://github.com/hydro-project/hydro/commit/5cd0a9625822620dcc99b99356edfecbf0549497))
 * **[#1747](https://github.com/hydro-project/hydro/issues/1747)**
    - Upgrade to Rust 2024 edition ([`ec3795a`](https://github.com/hydro-project/hydro/commit/ec3795a678d261a38085405b6e9bfea943dafefb))
 * **[#1757](https://github.com/hydro-project/hydro/issues/1757)**
    - Unchecked casts for handoffs & state, pedantic version ([`e5c2e23`](https://github.com/hydro-project/hydro/commit/e5c2e23359055f41492344edf19efbe3f2afd7ce))
 * **[#1759](https://github.com/hydro-project/hydro/issues/1759)**
    - Update stateful operators to be 'stateless' in loops, fix #1618 ([`247a21d`](https://github.com/hydro-project/hydro/commit/247a21d9617d44f13755c696224a6942afb9922a))
 * **Uncategorized**
    - Release dfir_lang v0.12.0, dfir_datalog_core v0.12.0, dfir_datalog v0.12.0, dfir_macro v0.12.0, hydroflow_deploy_integration v0.12.0, lattices_macro v0.5.9, variadics v0.0.9, variadics_macro v0.6.0, lattices v0.6.0, multiplatform_test v0.5.0, pusherator v0.0.11, dfir_rs v0.12.0, hydro_deploy v0.12.0, stageleft_macro v0.6.0, stageleft v0.7.0, stageleft_tool v0.6.0, hydro_lang v0.12.0, hydro_std v0.12.0, hydro_cli v0.12.0, safety bump 10 crates ([`973c925`](https://github.com/hydro-project/hydro/commit/973c925e87ed78344494581bd7ce1bbb4186a2f3))
</details>

## 0.11.0 (2024-12-23)

<csr-id-7dea92b19e6b86566cc27babb457415896b6b608/>
<csr-id-33eed2f8edf56c2914afa536167ef895d107277d/>
<csr-id-9ea5f061ee0f116caf8fc4ea99b62a9c7691be2a/>
<csr-id-e440623a553f28c16ac0f87871c135aaefeec7fd/>
<csr-id-251b1039c71d45d3f86123dba1926026ded80824/>
<csr-id-5196f247e0124a31567af940541044ce1906cdc1/>
<csr-id-03b3a349013a71b324276bca5329c33d400a73ff/>
<csr-id-accb13cad718c99d350e4bafe82e0ca38bf94c62/>
<csr-id-3291c07b37c9f9031837a2a32953e8f8854ec298/>
<csr-id-84ee06755a0ed7cabf32b334f1696bb600797c92/>
<csr-id-a6f60c92ae7168eb86eb311ca7b7afb10025c7de/>
<csr-id-2a22d50285ae1be1a5f888d5d15321cc1bb13c82/>
<csr-id-5e58e346612a094c7e637919c84ab1e78b59be27/>

### Documentation

 - <csr-id-5c53e12d6845f60c95248164eea3e2c48a8ff270/> `getLines` support named section markers, fix #1320
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

### New Features

 - <csr-id-b8acd843bdbcfb445bf942e697447f6bf58a10da/> initial flo implementation
   Basic first pass implementation, without changing the current scheduler,
   no breaking behavior. Adds the flo syntax (`loop { ... }`), basic graph
   structure checks, nested loop blocks, and two basic windowing operators
   (`batch()` and `all_once()`).
   
   Next steps:
   - Needs scheduler changes:

### Bug Fixes

 - <csr-id-64578f39d6f87e3a96fe4c3a8599b92606e1510e/> avoid pulling in `clap` as a dependency
   We only need it for examples, so this reduces compilation burden for
   `trybuild`.

### Other

 - <csr-id-7dea92b19e6b86566cc27babb457415896b6b608/> fix stable tests
   Sets `TRYBUILD=overwrite` on stable, as the output messages will often
   not match `pinned-nightly`
   
   Adds env `HYDROFLOW_EXPECT_WARNINGS=ignore` to allow warning tests to
   pass

### Chore

 - <csr-id-84ee06755a0ed7cabf32b334f1696bb600797c92/> update links for renamed repo (excluding `CHANGELOG.md`s), fix #1571
 - <csr-id-a6f60c92ae7168eb86eb311ca7b7afb10025c7de/> bump versions manually for renamed crates, per `RELEASING.md`
 - <csr-id-2a22d50285ae1be1a5f888d5d15321cc1bb13c82/> cleanup snapshots
 - <csr-id-5e58e346612a094c7e637919c84ab1e78b59be27/> Rename Hydroflow -> DFIR
   Work In Progress:
   - [x] hydroflow_macro
   - [x] hydroflow_datalog_core
   - [x] hydroflow_datalog
   - [x] hydroflow_lang
   - [x] hydroflow

### Chore

 - <csr-id-5196f247e0124a31567af940541044ce1906cdc1/> update links for renamed repo (excluding `CHANGELOG.md`s), fix #1571
 - <csr-id-03b3a349013a71b324276bca5329c33d400a73ff/> bump versions manually for renamed crates, per `RELEASING.md`
 - <csr-id-accb13cad718c99d350e4bafe82e0ca38bf94c62/> cleanup snapshots
 - <csr-id-3291c07b37c9f9031837a2a32953e8f8854ec298/> Rename Hydroflow -> DFIR
   Work In Progress:
   - [x] hydroflow_macro
   - [x] hydroflow_datalog_core
   - [x] hydroflow_datalog
   - [x] hydroflow_lang
   - [x] hydroflow

### Refactor

 - <csr-id-33eed2f8edf56c2914afa536167ef895d107277d/> rename `two_pc` -> `two_pc_hf` for collision with hf+ `two_pc` example
 - <csr-id-9ea5f061ee0f116caf8fc4ea99b62a9c7691be2a/> use `match_box` macro to compile on stable
 - <csr-id-e440623a553f28c16ac0f87871c135aaefeec7fd/> move rest of scheduler state into `Context`

### New Features (BREAKING)

 - <csr-id-c65b4c49f3a3f2dcc7c8c28d1871b88c6c954822/> minimize dependencies pulled into `trybuild` builds
   We don't need `hydroflow_lang` in the runtime builds, do some feature
   flagging to avoid it.
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

### `hydroflow` Commit Statistics

<csr-read-only-do-not-edit/>

 - 10 commits contributed to the release.
 - 38 days passed between releases.
 - 10 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 10 unique issues were worked on: [#1555](https://github.com/hydro-project/hydroflow/issues/1555), [#1578](https://github.com/hydro-project/hydroflow/issues/1578), [#1585](https://github.com/hydro-project/hydroflow/issues/1585), [#1594](https://github.com/hydro-project/hydroflow/issues/1594), [#1597](https://github.com/hydro-project/hydroflow/issues/1597), [#1600](https://github.com/hydro-project/hydroflow/issues/1600), [#1601](https://github.com/hydro-project/hydroflow/issues/1601), [#1606](https://github.com/hydro-project/hydroflow/issues/1606), [#1608](https://github.com/hydro-project/hydroflow/issues/1608), [#1611](https://github.com/hydro-project/hydroflow/issues/1611)

### `hydroflow` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1555](https://github.com/hydro-project/hydroflow/issues/1555)**
    - `getLines` support named section markers, fix #1320 ([`5c53e12`](https://github.com/hydro-project/hydroflow/commit/5c53e12d6845f60c95248164eea3e2c48a8ff270))
 * **[#1578](https://github.com/hydro-project/hydroflow/issues/1578)**
    - Avoid pulling in `clap` as a dependency ([`64578f3`](https://github.com/hydro-project/hydroflow/commit/64578f39d6f87e3a96fe4c3a8599b92606e1510e))
 * **[#1585](https://github.com/hydro-project/hydroflow/issues/1585)**
    - Initial flo implementation ([`b8acd84`](https://github.com/hydro-project/hydroflow/commit/b8acd843bdbcfb445bf942e697447f6bf58a10da))
 * **[#1594](https://github.com/hydro-project/hydroflow/issues/1594)**
    - Move rest of scheduler state into `Context` ([`e440623`](https://github.com/hydro-project/hydroflow/commit/e440623a553f28c16ac0f87871c135aaefeec7fd))
 * **[#1597](https://github.com/hydro-project/hydroflow/issues/1597)**
    - Use `match_box` macro to compile on stable ([`9ea5f06`](https://github.com/hydro-project/hydroflow/commit/9ea5f061ee0f116caf8fc4ea99b62a9c7691be2a))
 * **[#1600](https://github.com/hydro-project/hydroflow/issues/1600)**
    - Remove `import!`, fix #1110 ([`f2a4bee`](https://github.com/hydro-project/hydroflow/commit/f2a4bee8cd6945937bed5bc22fd85efd8d0aef0a))
 * **[#1601](https://github.com/hydro-project/hydroflow/issues/1601)**
    - Rename `two_pc` -> `two_pc_hf` for collision with hf+ `two_pc` example ([`33eed2f`](https://github.com/hydro-project/hydroflow/commit/33eed2f8edf56c2914afa536167ef895d107277d))
 * **[#1606](https://github.com/hydro-project/hydroflow/issues/1606)**
    - Use `cfg(nightly)` instead of feature, remove `-Z` flag, use `Diagnostic::try_emit` ([`251b103`](https://github.com/hydro-project/hydroflow/commit/251b1039c71d45d3f86123dba1926026ded80824))
 * **[#1608](https://github.com/hydro-project/hydroflow/issues/1608)**
    - Fix stable tests ([`7dea92b`](https://github.com/hydro-project/hydroflow/commit/7dea92b19e6b86566cc27babb457415896b6b608))
 * **[#1611](https://github.com/hydro-project/hydroflow/issues/1611)**
    - Minimize dependencies pulled into `trybuild` builds ([`c65b4c4`](https://github.com/hydro-project/hydroflow/commit/c65b4c49f3a3f2dcc7c8c28d1871b88c6c954822))
</details>

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 9 commits contributed to the release.
 - 8 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 6 unique issues were worked on: [#1501](https://github.com/hydro-project/hydro/issues/1501), [#1620](https://github.com/hydro-project/hydro/issues/1620), [#1623](https://github.com/hydro-project/hydro/issues/1623), [#1624](https://github.com/hydro-project/hydro/issues/1624), [#1627](https://github.com/hydro-project/hydro/issues/1627), [#1628](https://github.com/hydro-project/hydro/issues/1628)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1501](https://github.com/hydro-project/hydro/issues/1501)**
    - Add `repository` field to `Cargo.toml`s, fix #1452 ([`204bd11`](https://github.com/hydro-project/hydro/commit/204bd117ca3a8845b4986539efb91a0c612dfa05))
 * **[#1620](https://github.com/hydro-project/hydro/issues/1620)**
    - Rename Hydroflow -> DFIR ([`5e58e34`](https://github.com/hydro-project/hydro/commit/5e58e346612a094c7e637919c84ab1e78b59be27))
 * **[#1623](https://github.com/hydro-project/hydro/issues/1623)**
    - Cleanup snapshots ([`2a22d50`](https://github.com/hydro-project/hydro/commit/2a22d50285ae1be1a5f888d5d15321cc1bb13c82))
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

<csr-id-e564b133a4db192e0331d427aa80ef52cd97608c/>
<csr-id-d5677604e93c07a5392f4229af94a0b736eca382/>
<csr-id-8442d1b524621a9f8b43372a9c25991efb33c25e/>
<csr-id-47cb703e771f7d1c451ceb9d185ada96410949da/>
<csr-id-656ee328c8710bce7370c851437a80ca3db46a5a/>

### Chore

 - <csr-id-e564b133a4db192e0331d427aa80ef52cd97608c/> update `proc-macro2`, fixes span info, fix #729
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

### Bug Fixes

 - <csr-id-16b730c75cfca79ea5f869308b1e1e14b3e9c155/> `cross_singleton()` forgot value if multiple runs in a tick, fix #1518
   Adds the minimal reproducer test from @shadaj
   
   Note this may have negative performance implications, as the singleton value now is stored in the state API (heap) instead of locally. If we use singleton syntax this duplicate allocation could probably be avoided.
   
   > Confirmed that this fixed the bugs in our Paxos implementation, no noticeable performance impact. @shadj
 - <csr-id-b96123369cec3d6d407af973f1896b2734fd92ef/> cleanup temp tcp networking code, fix race condition fix #1458
   consolidate into one task to prevent races

### Style

 - <csr-id-8442d1b524621a9f8b43372a9c25991efb33c25e/> fixes for latest nightly clippy
 - <csr-id-47cb703e771f7d1c451ceb9d185ada96410949da/> fixes for nightly clippy
   a couple few spurious `too_many_arguments` and a spurious
   `zombie_processes` still on current nightly (`clippy 0.1.84 (4392847410
   2024-10-21)`)

### Test

 - <csr-id-656ee328c8710bce7370c851437a80ca3db46a5a/> ignore trybuild tests inconsistent on latest nightly

### Bug Fixes (BREAKING)

 - <csr-id-e796200743f2cc2da5a0e91c492f016ca98008e8/> fix #1401 `lattice_bimorphism()` double-emit, add docs
   Fixes the issue by combining the all values generated per subgraph
   execution into one, which effectively de-duplicates the values.
   
   Adds basic docs.

### `hydroflow` Commit Statistics

<csr-read-only-do-not-edit/>

 - 10 commits contributed to the release.
 - 69 days passed between releases.
 - 9 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 8 unique issues were worked on: [#1444](https://github.com/hydro-project/hydroflow/issues/1444), [#1446](https://github.com/hydro-project/hydroflow/issues/1446), [#1497](https://github.com/hydro-project/hydroflow/issues/1497), [#1503](https://github.com/hydro-project/hydroflow/issues/1503), [#1505](https://github.com/hydro-project/hydroflow/issues/1505), [#1520](https://github.com/hydro-project/hydroflow/issues/1520), [#1522](https://github.com/hydro-project/hydroflow/issues/1522), [#1537](https://github.com/hydro-project/hydroflow/issues/1537)

### `hydroflow` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1444](https://github.com/hydro-project/hydroflow/issues/1444)**
    - Update pinned rust version, clippy lints, remove some dead code ([`d567760`](https://github.com/hydro-project/hydroflow/commit/d5677604e93c07a5392f4229af94a0b736eca382))
 * **[#1446](https://github.com/hydro-project/hydroflow/issues/1446)**
    - Cleanup temp tcp networking code, fix race condition fix #1458 ([`b961233`](https://github.com/hydro-project/hydroflow/commit/b96123369cec3d6d407af973f1896b2734fd92ef))
 * **[#1497](https://github.com/hydro-project/hydroflow/issues/1497)**
    - Update `proc-macro2`, fixes span info, fix #729 ([`e564b13`](https://github.com/hydro-project/hydroflow/commit/e564b133a4db192e0331d427aa80ef52cd97608c))
 * **[#1503](https://github.com/hydro-project/hydroflow/issues/1503)**
    - Generalized hash trie indexes for relational tuples ([`f7e740f`](https://github.com/hydro-project/hydroflow/commit/f7e740fb2ba36d0fcf3fd196d60333552911e3a4))
 * **[#1505](https://github.com/hydro-project/hydroflow/issues/1505)**
    - Fixes for nightly clippy ([`47cb703`](https://github.com/hydro-project/hydroflow/commit/47cb703e771f7d1c451ceb9d185ada96410949da))
 * **[#1520](https://github.com/hydro-project/hydroflow/issues/1520)**
    - `cross_singleton()` forgot value if multiple runs in a tick, fix #1518 ([`16b730c`](https://github.com/hydro-project/hydroflow/commit/16b730c75cfca79ea5f869308b1e1e14b3e9c155))
 * **[#1522](https://github.com/hydro-project/hydroflow/issues/1522)**
    - Fix #1401 `lattice_bimorphism()` double-emit, add docs ([`e796200`](https://github.com/hydro-project/hydroflow/commit/e796200743f2cc2da5a0e91c492f016ca98008e8))
 * **[#1537](https://github.com/hydro-project/hydroflow/issues/1537)**
    - Fixes for latest nightly clippy ([`8442d1b`](https://github.com/hydro-project/hydroflow/commit/8442d1b524621a9f8b43372a9c25991efb33c25e))
 * **Uncategorized**
    - Release hydroflow_lang v0.10.0, hydroflow_datalog_core v0.10.0, hydroflow_datalog v0.10.0, hydroflow_deploy_integration v0.10.0, hydroflow_macro v0.10.0, lattices_macro v0.5.7, variadics v0.0.7, variadics_macro v0.5.5, lattices v0.5.8, multiplatform_test v0.3.0, pusherator v0.0.9, hydroflow v0.10.0, hydro_deploy v0.10.0, stageleft_macro v0.4.0, stageleft v0.5.0, stageleft_tool v0.4.0, hydroflow_plus v0.10.0, hydro_cli v0.10.0, safety bump 8 crates ([`dcd48fc`](https://github.com/hydro-project/hydroflow/commit/dcd48fc7ee805898d9b5ef0d082870e30615e95b))
    - Ignore trybuild tests inconsistent on latest nightly ([`656ee32`](https://github.com/hydro-project/hydroflow/commit/656ee328c8710bce7370c851437a80ca3db46a5a))
</details>

## 0.9.0 (2024-08-30)

<csr-id-a2ec110ccadb97e293b19d83a155d98d94224bba/>
<csr-id-11af32828bab6e4a4264d2635ff71a12bb0bb778/>
<csr-id-3c41e1e9b505bb84474ec10df30fb65c6d6f90cb/>
<csr-id-0a465e55dd39c76bc1aefb020460a639d792fe87/>
<csr-id-a2147864b24110c9ae2c1553e9e8b55bd5065f15/>
<csr-id-fa417205569d8c49c85b0c2324118e0f9b1c8407/>

### Chore

 - <csr-id-a2ec110ccadb97e293b19d83a155d98d94224bba/> manually set versions for crates renamed in #1413
 - <csr-id-11af32828bab6e4a4264d2635ff71a12bb0bb778/> lower min dependency versions where possible, update `Cargo.lock`
   Moved from #1418
   
   ---------
 - <csr-id-3c41e1e9b505bb84474ec10df30fb65c6d6f90cb/> drop unused dependencies

### Documentation

 - <csr-id-f5f1eb0c612f5c0c1752360d972ef6853c5e12f0/> cleanup doc comments for clippy latest

### New Features

 - <csr-id-9e5f58ef773f0aee39a9705d9845361a2488649b/> allow `demux_enum` to have any number of outputs, fix #1329
 - <csr-id-3850b4793b33ce7c4e88c80868335ba7506c4229/> Added simulation framework.
   Will move it out of the blind gossip PR next.
 - <csr-id-bd793e2fc1db735a6e623973028e99675a7933e0/> add `cross_singleton` operator
   This operator is necessary to eliminate performance bottlenecks in Paxos
   where cross-products result in unnecessary cloning and lack of
   short-circuit behavior results in values being cloned out of the
   internal state of `reduce_keyed`.

### Bug Fixes

 - <csr-id-22c72189bb76412955d29b03c5d99894c558a07c/> remove `FlowProps`
 - <csr-id-75dd4fbb772a3ae7e58989aacab2121a1a487557/> `fold<'tick>` should always emit a value

### Refactor (BREAKING)

 - <csr-id-0a465e55dd39c76bc1aefb020460a639d792fe87/> rename integration crates to drop CLI references
 - <csr-id-a2147864b24110c9ae2c1553e9e8b55bd5065f15/> `Deployment.stop()` for graceful shutdown including updated `perf` profile downloading
   * `perf` profile downloading moved from the `drop()` impl to `async fn
   stop()`
   * download perf data via stdout
   * update async-ssh2-lite to 0.5 to cleanup tokio compat issues
   
   WIP for #1365

### Style (BREAKING)

 - <csr-id-fa417205569d8c49c85b0c2324118e0f9b1c8407/> rename some `CLI`->`Deploy`, decapitalize acronym names

### `hydroflow` Commit Statistics

<csr-read-only-do-not-edit/>

 - 13 commits contributed to the release.
 - 38 days passed between releases.
 - 12 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 11 unique issues were worked on: [#1370](https://github.com/hydro-project/hydroflow/issues/1370), [#1373](https://github.com/hydro-project/hydroflow/issues/1373), [#1399](https://github.com/hydro-project/hydroflow/issues/1399), [#1407](https://github.com/hydro-project/hydroflow/issues/1407), [#1409](https://github.com/hydro-project/hydroflow/issues/1409), [#1413](https://github.com/hydro-project/hydroflow/issues/1413), [#1416](https://github.com/hydro-project/hydroflow/issues/1416), [#1417](https://github.com/hydro-project/hydroflow/issues/1417), [#1420](https://github.com/hydro-project/hydroflow/issues/1420), [#1423](https://github.com/hydro-project/hydroflow/issues/1423), [#1428](https://github.com/hydro-project/hydroflow/issues/1428)

### `hydroflow` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1370](https://github.com/hydro-project/hydroflow/issues/1370)**
    - `Deployment.stop()` for graceful shutdown including updated `perf` profile downloading ([`a214786`](https://github.com/hydro-project/hydroflow/commit/a2147864b24110c9ae2c1553e9e8b55bd5065f15))
 * **[#1373](https://github.com/hydro-project/hydroflow/issues/1373)**
    - Add `cross_singleton` operator ([`bd793e2`](https://github.com/hydro-project/hydroflow/commit/bd793e2fc1db735a6e623973028e99675a7933e0))
 * **[#1399](https://github.com/hydro-project/hydroflow/issues/1399)**
    - Rename some `CLI`->`Deploy`, decapitalize acronym names ([`fa41720`](https://github.com/hydro-project/hydroflow/commit/fa417205569d8c49c85b0c2324118e0f9b1c8407))
 * **[#1407](https://github.com/hydro-project/hydroflow/issues/1407)**
    - `fold<'tick>` should always emit a value ([`75dd4fb`](https://github.com/hydro-project/hydroflow/commit/75dd4fbb772a3ae7e58989aacab2121a1a487557))
 * **[#1409](https://github.com/hydro-project/hydroflow/issues/1409)**
    - Allow `demux_enum` to have any number of outputs, fix #1329 ([`9e5f58e`](https://github.com/hydro-project/hydroflow/commit/9e5f58ef773f0aee39a9705d9845361a2488649b))
 * **[#1413](https://github.com/hydro-project/hydroflow/issues/1413)**
    - Rename integration crates to drop CLI references ([`0a465e5`](https://github.com/hydro-project/hydroflow/commit/0a465e55dd39c76bc1aefb020460a639d792fe87))
 * **[#1416](https://github.com/hydro-project/hydroflow/issues/1416)**
    - Added simulation framework. ([`3850b47`](https://github.com/hydro-project/hydroflow/commit/3850b4793b33ce7c4e88c80868335ba7506c4229))
 * **[#1417](https://github.com/hydro-project/hydroflow/issues/1417)**
    - Drop unused dependencies ([`3c41e1e`](https://github.com/hydro-project/hydroflow/commit/3c41e1e9b505bb84474ec10df30fb65c6d6f90cb))
 * **[#1420](https://github.com/hydro-project/hydroflow/issues/1420)**
    - Remove `FlowProps` ([`22c7218`](https://github.com/hydro-project/hydroflow/commit/22c72189bb76412955d29b03c5d99894c558a07c))
 * **[#1423](https://github.com/hydro-project/hydroflow/issues/1423)**
    - Lower min dependency versions where possible, update `Cargo.lock` ([`11af328`](https://github.com/hydro-project/hydroflow/commit/11af32828bab6e4a4264d2635ff71a12bb0bb778))
 * **[#1428](https://github.com/hydro-project/hydroflow/issues/1428)**
    - Cleanup doc comments for clippy latest ([`f5f1eb0`](https://github.com/hydro-project/hydroflow/commit/f5f1eb0c612f5c0c1752360d972ef6853c5e12f0))
 * **Uncategorized**
    - Release hydroflow_lang v0.9.0, hydroflow_datalog_core v0.9.0, hydroflow_datalog v0.9.0, hydroflow_deploy_integration v0.9.0, hydroflow_macro v0.9.0, lattices_macro v0.5.6, lattices v0.5.7, multiplatform_test v0.2.0, variadics v0.0.6, pusherator v0.0.8, hydroflow v0.9.0, stageleft_macro v0.3.0, stageleft v0.4.0, stageleft_tool v0.3.0, hydroflow_plus v0.9.0, hydro_deploy v0.9.0, hydro_cli v0.9.0, hydroflow_plus_deploy v0.9.0, safety bump 8 crates ([`0750117`](https://github.com/hydro-project/hydroflow/commit/0750117de7088c01a439b102adeb4c832889f171))
    - Manually set versions for crates renamed in #1413 ([`a2ec110`](https://github.com/hydro-project/hydroflow/commit/a2ec110ccadb97e293b19d83a155d98d94224bba))
</details>

## 0.8.0 (2024-07-23)

<csr-id-3098f77fd99882aae23c4b31017aa4b761306197/>
<csr-id-f1442a0161fe7a1827a60cb96ff11646d711373d/>
<csr-id-70a3f9e19f70fae0967eb96c454ab922c0f5290b/>
<csr-id-b3c233dadd7ebc57eb7e8ee111e43e8dddb9c882/>
<csr-id-67c0e51fb25ea1a2e3aae197c1984920b46759fa/>
<csr-id-12b8ba53f28eb9de1318b41cdf1e23282f6f0eb6/>

### Chore

 - <csr-id-3098f77fd99882aae23c4b31017aa4b761306197/> update pinned rust version to 2024-06-17

### Documentation

 - <csr-id-2a4881d7c981bdf8f4deae9902e7d305f36c4203/> test code snipets, generate output for `surface_flows`, fix #814

### Bug Fixes

<csr-id-b79f1a4c8f30131c8ca2ab4900efed9ded819581/>
<csr-id-dfb5ec9022d2f854d65d68e55ae5c445ddbb8f5e/>
<csr-id-39ad0d68de5281c1ee4ecc97e05a1d14c602538b/>
<csr-id-1ad690b993f38ac6a03667fdce56e6603076b1d2/>

 - <csr-id-f45b9ddbfca84e11398f3dec774b713b5b071422/> allow `ensure_singleton_referencers_succeed_persist` to ignore `identity`/`tee`/`union` operators, helps #1290
   Adds tests for `ensure_singleton_referencers_succeed_persist` persist
   insertion behavior
   
   Fixes some cases of #1290, when just identity operators
   (union/tee/identity) are between the singleton referencer and its
   preceding flow state.
   
   TODO track per-operator instead: #1311
 - <csr-id-404f0accf08e643a1c5e815f06bb31a65379e8c8/> improve spanning of singleton substitution code, add compile-fail test, fix #1294
   Another small spanning improvement
 - <csr-id-f91c30045dfdf92cf3d383676875d9e749cb8d93/> add `add_state_tick` to state API, reset at end of each tick, fix #1298
   Option 2 of #1298
   
   * Main feature in the title is in `src/scheduled/{context,graph}.rs`
* Codegen for some stateful operators (those which can be used as singletons, and some others) is changed to use the new API.
* Add a test `test_cartesian_product_tick_state` for #1298
* Rest of the diff is snapshot changes

### Refactor

 - <csr-id-f1442a0161fe7a1827a60cb96ff11646d711373d/> improve diagnostics for missing generic params
   Improves diagnostic messages a bit for when no generic params are
   supplied but some are expected. Previously this would span to the entire
   macro invocation.
 - <csr-id-70a3f9e19f70fae0967eb96c454ab922c0f5290b/> improve diagnostics by re-spanning `#root`
   Inspired by fixing the spans in `demux_enum` in #1271
   * re-span `#root` to `op_span` for better diagnostics
   * use better span `func.inputs` in `demux` and `demux_enum`
   * clippy fixups in `source_json`, `source_stdin`
   * fix #1201 (for the most part)

### Reverted

 - <csr-id-256779abece03bee662b351430d27141d10bd5ef/> "feat(hydroflow): Added poll_futures and poll_futures_async operators.", fix #1183
   This reverts commit 997d90a76db9a4e05dbac35073a09548750ce342.
   
   We have been trying to figure out the semantics a bit, and want to give
   it more thought before we commit to maintaining it
   
   Can un-revert and adjust the semantics later when we use it

### Test

 - <csr-id-b3c233dadd7ebc57eb7e8ee111e43e8dddb9c882/> fix `fold_keyed()` and add `reduce_keyed()` compile-fail tests, fix #1279

### Bug Fixes (BREAKING)

 - <csr-id-755e8a6d2c2b30b5d28b60315bb099030d3f3964/> remove singleton referencer `persist::<'static>()` insertion
   Also enables singletons for `persist()` and ensures that only the
   `'static` lifetime is used
   
   Singletons are supposed to act like `cross_join()`. I.e. if we have this
   code:
   ```rust
   stream -> filter(|item| ... #y ...) -> ...
   ```
   It should behave equivalently to
   ```rust
   stream -> cj[0];
   y -> cj[1];
   cj = cross_join() -> filter(|(item, y)| ...) -> ...
   ```
   
   This has a very unintuitive replaying behavior, if `y` receives multiple
   updates:
   1. `y` receives an item `10`
2. `stream` receives an item `20`
3. `(10, 20)` is emitted
4. `y` receives an item `11`
5. `(11, 20)` is emitted
   In this case the item `20` gets emitted twice.

### Refactor (BREAKING)

 - <csr-id-67c0e51fb25ea1a2e3aae197c1984920b46759fa/> require lifetime on `perist*()` operators

### Style (BREAKING)

 - <csr-id-12b8ba53f28eb9de1318b41cdf1e23282f6f0eb6/> enable clippy `upper-case-acronyms-aggressive`
   * rename `GCP` -> `Gcp`, `NodeID` -> `NodeId`
   * update CI `cargo-generate` template testing to use PR's branch instead
   of whatever `main` happens to be

### `hydroflow` Commit Statistics

<csr-read-only-do-not-edit/>

 - 17 commits contributed to the release.
 - 59 days passed between releases.
 - 16 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 17 unique issues were worked on: [#1143](https://github.com/hydro-project/hydroflow/issues/1143), [#1216](https://github.com/hydro-project/hydroflow/issues/1216), [#1244](https://github.com/hydro-project/hydroflow/issues/1244), [#1260](https://github.com/hydro-project/hydroflow/issues/1260), [#1271](https://github.com/hydro-project/hydroflow/issues/1271), [#1273](https://github.com/hydro-project/hydroflow/issues/1273), [#1274](https://github.com/hydro-project/hydroflow/issues/1274), [#1280](https://github.com/hydro-project/hydroflow/issues/1280), [#1283](https://github.com/hydro-project/hydroflow/issues/1283), [#1295](https://github.com/hydro-project/hydroflow/issues/1295), [#1296](https://github.com/hydro-project/hydroflow/issues/1296), [#1297](https://github.com/hydro-project/hydroflow/issues/1297), [#1300](https://github.com/hydro-project/hydroflow/issues/1300), [#1309](https://github.com/hydro-project/hydroflow/issues/1309), [#1312](https://github.com/hydro-project/hydroflow/issues/1312), [#1332](https://github.com/hydro-project/hydroflow/issues/1332), [#1345](https://github.com/hydro-project/hydroflow/issues/1345)

### `hydroflow` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1143](https://github.com/hydro-project/hydroflow/issues/1143)**
    - "feat(hydroflow): Added poll_futures and poll_futures_async operators.", fix #1183 ([`256779a`](https://github.com/hydro-project/hydroflow/commit/256779abece03bee662b351430d27141d10bd5ef))
 * **[#1216](https://github.com/hydro-project/hydroflow/issues/1216)**
    - "feat(hydroflow): Added poll_futures and poll_futures_async operators.", fix #1183 ([`256779a`](https://github.com/hydro-project/hydroflow/commit/256779abece03bee662b351430d27141d10bd5ef))
 * **[#1244](https://github.com/hydro-project/hydroflow/issues/1244)**
    - Make inner for `WithTop` & `WithBot` private ([`1ad690b`](https://github.com/hydro-project/hydroflow/commit/1ad690b993f38ac6a03667fdce56e6603076b1d2))
 * **[#1260](https://github.com/hydro-project/hydroflow/issues/1260)**
    - Test code snipets, generate output for `surface_flows`, fix #814 ([`2a4881d`](https://github.com/hydro-project/hydroflow/commit/2a4881d7c981bdf8f4deae9902e7d305f36c4203))
 * **[#1271](https://github.com/hydro-project/hydroflow/issues/1271)**
    - Allow use of generics in `demux_enum::<...>()` op ([`b79f1a4`](https://github.com/hydro-project/hydroflow/commit/b79f1a4c8f30131c8ca2ab4900efed9ded819581))
 * **[#1273](https://github.com/hydro-project/hydroflow/issues/1273)**
    - Make sure tasks are spawned!!!! ([`39ad0d6`](https://github.com/hydro-project/hydroflow/commit/39ad0d68de5281c1ee4ecc97e05a1d14c602538b))
 * **[#1274](https://github.com/hydro-project/hydroflow/issues/1274)**
    - Yield before `collect_ready_async` to ensure background async tasks can send to the stream ([`dfb5ec9`](https://github.com/hydro-project/hydroflow/commit/dfb5ec9022d2f854d65d68e55ae5c445ddbb8f5e))
 * **[#1280](https://github.com/hydro-project/hydroflow/issues/1280)**
    - Improve diagnostics by re-spanning `#root` ([`70a3f9e`](https://github.com/hydro-project/hydroflow/commit/70a3f9e19f70fae0967eb96c454ab922c0f5290b))
 * **[#1283](https://github.com/hydro-project/hydroflow/issues/1283)**
    - Fix `fold_keyed()` and add `reduce_keyed()` compile-fail tests, fix #1279 ([`b3c233d`](https://github.com/hydro-project/hydroflow/commit/b3c233dadd7ebc57eb7e8ee111e43e8dddb9c882))
 * **[#1295](https://github.com/hydro-project/hydroflow/issues/1295)**
    - Require lifetime on `perist*()` operators ([`67c0e51`](https://github.com/hydro-project/hydroflow/commit/67c0e51fb25ea1a2e3aae197c1984920b46759fa))
 * **[#1296](https://github.com/hydro-project/hydroflow/issues/1296)**
    - Improve diagnostics for missing generic params ([`f1442a0`](https://github.com/hydro-project/hydroflow/commit/f1442a0161fe7a1827a60cb96ff11646d711373d))
 * **[#1297](https://github.com/hydro-project/hydroflow/issues/1297)**
    - Allow `ensure_singleton_referencers_succeed_persist` to ignore `identity`/`tee`/`union` operators, helps #1290 ([`f45b9dd`](https://github.com/hydro-project/hydroflow/commit/f45b9ddbfca84e11398f3dec774b713b5b071422))
 * **[#1300](https://github.com/hydro-project/hydroflow/issues/1300)**
    - Add `add_state_tick` to state API, reset at end of each tick, fix #1298 ([`f91c300`](https://github.com/hydro-project/hydroflow/commit/f91c30045dfdf92cf3d383676875d9e749cb8d93))
 * **[#1309](https://github.com/hydro-project/hydroflow/issues/1309)**
    - Update pinned rust version to 2024-06-17 ([`3098f77`](https://github.com/hydro-project/hydroflow/commit/3098f77fd99882aae23c4b31017aa4b761306197))
 * **[#1312](https://github.com/hydro-project/hydroflow/issues/1312)**
    - Improve spanning of singleton substitution code, add compile-fail test, fix #1294 ([`404f0ac`](https://github.com/hydro-project/hydroflow/commit/404f0accf08e643a1c5e815f06bb31a65379e8c8))
 * **[#1332](https://github.com/hydro-project/hydroflow/issues/1332)**
    - Remove singleton referencer `persist::<'static>()` insertion ([`755e8a6`](https://github.com/hydro-project/hydroflow/commit/755e8a6d2c2b30b5d28b60315bb099030d3f3964))
 * **[#1345](https://github.com/hydro-project/hydroflow/issues/1345)**
    - Enable clippy `upper-case-acronyms-aggressive` ([`12b8ba5`](https://github.com/hydro-project/hydroflow/commit/12b8ba53f28eb9de1318b41cdf1e23282f6f0eb6))
 * **Uncategorized**
    - Release hydroflow_lang v0.8.0, hydroflow_datalog_core v0.8.0, hydroflow_datalog v0.8.0, hydroflow_macro v0.8.0, lattices_macro v0.5.5, lattices v0.5.6, variadics v0.0.5, pusherator v0.0.7, hydroflow v0.8.0, hydroflow_plus v0.8.0, hydro_deploy v0.8.0, hydro_cli v0.8.0, hydroflow_plus_cli_integration v0.8.0, safety bump 7 crates ([`ca6c16b`](https://github.com/hydro-project/hydroflow/commit/ca6c16b4a7ce35e155fe7fc6c7d1676c37c9e4de))
</details>

## 0.7.0 (2024-05-24)

<csr-id-c9dfddc680e0ce5415539d7b77bc5beb97ab59d9/>
<csr-id-b86f11aad344fef6ad9cdd1db0b45bb738c48bd6/>
<csr-id-18015029a725b068696ed9edefd1097583c858a6/>
<csr-id-826dbd9a709de2f883992bdcefa8f2d566d74ecb/>
<csr-id-b649d470cc28a2bbb7e31426adcd050b455bf803/>
<csr-id-20471f11901e3fb15a2efea61752d836d4facba5/>
<csr-id-b072ee026f97f8537165e1fb247101e0ab2fb320/>
<csr-id-a4cd5fe57130e3b517872d1c27a274f9c9f601c2/>
<csr-id-d9b2c0263d508e6f6855f49504896d4ea670c355/>
<csr-id-f46e0ac84e13ad6d713836c948b02e7b199f5faa/>
<csr-id-8f4cd7bfc13bcbd89ff4a0e48c7e95cde5039f1b/>
<csr-id-d2427e2cc901c4174830d41b4a1dfc52fd4f19ce/>
<csr-id-4386fac824d64f63eae7629292675ac6bc8df9f7/>
<csr-id-d7e579c39b370a0ea0b0385d1029e9f8a7351d68/>

### Chore

 - <csr-id-c9dfddc680e0ce5415539d7b77bc5beb97ab59d9/> use workaround for `cargo smart-release` not properly ordering `dev-`/`build-dependencies`
 - <csr-id-b86f11aad344fef6ad9cdd1db0b45bb738c48bd6/> expect custom config names to prevent warnings
   See
   https://doc.rust-lang.org/nightly/cargo/reference/build-scripts.html#rustc-check-cfg
 - <csr-id-18015029a725b068696ed9edefd1097583c858a6/> update pyo3, silence warnings in generated code

### Documentation

 - <csr-id-0d2f14b9237c0eaa8131d1d1118768357ac8133b/> Updating CONTRIBUTING.md with some info about feature branches
   Also updating GitHub workflows to run on feature branches as well.
 - <csr-id-9bcc5265044b467e9fe4ada0a89c7e84fd66dd43/> improve docs/README.md, fix #856

### New Features

 - <csr-id-b0692b0d697980eaf9893c07a443a257e04786c5/> add `'static`/`'tick` support for `state()` operator
 - <csr-id-f184ea145f0c7c3072d7d0f94d42fcda717ac8d9/> insert `persist()` before singleton referencers to enable replay
 - <csr-id-4ca8ce43c0998296e2d86bd74800585ebb24123a/> allow `defer_tick()`, `next_stratum()` to have a type hint arg
 - <csr-id-9df9c6251526903dbe7288e2fd9a532c63a9412c/> error on unused ports, fix #1108
 - <csr-id-997d90a76db9a4e05dbac35073a09548750ce342/> Added poll_futures and poll_futures_async operators.

### Bug Fixes

 - <csr-id-e2fa6b0729cb92b29e1d293c2788458845ff306a/> fix #1050 for `reduce_keyed`, fix #1050
 - <csr-id-d773f9a6938fe6d1521516f7a7c441c6c281a9fa/> fix scheduler spinning on replay, fix #961
   fixes the added tests failing in the previous commit
 - <csr-id-69abccde27644d7d0ed2fdf7d75f491fdbc41de8/> fix `TeeingHandoff` not scheduling outputs `tee()`d after send port is already used via `add_subgraph*`, fix #1163
   Add test for it as well

### Refactor

 - <csr-id-826dbd9a709de2f883992bdcefa8f2d566d74ecb/> simplify `demux_enum()`, somewhat improves error messages #1201
 - <csr-id-b649d470cc28a2bbb7e31426adcd050b455bf803/> use `Duration` instead of int seconds
 - <csr-id-20471f11901e3fb15a2efea61752d836d4facba5/> reorder error message emission for conflicting varnames
 - <csr-id-b072ee026f97f8537165e1fb247101e0ab2fb320/> improve `TeeingHandoff` drop implementation

### Style

 - <csr-id-a4cd5fe57130e3b517872d1c27a274f9c9f601c2/> fix warning unnecessary qualification
 - <csr-id-d9b2c0263d508e6f6855f49504896d4ea670c355/> appease latest nightly clippy

### Test

 - <csr-id-f46e0ac84e13ad6d713836c948b02e7b199f5faa/> join replay static test
 - <csr-id-8f4cd7bfc13bcbd89ff4a0e48c7e95cde5039f1b/> add tests for scheduler spinning #961
   failing
 - <csr-id-d2427e2cc901c4174830d41b4a1dfc52fd4f19ce/> update some compile-fail outputs caused by rustc regression

### New Features (BREAKING)

 - <csr-id-218175cd935c4d94efb2f01f6e73300ace978283/> Change current_tick_start to wall clock time
 - <csr-id-c2f6c9578127a71c879752d52e115df75659e2b0/> Introduce newtypes for working with ticks
 - <csr-id-c9dc66db2bde34aa1385c8847d0ba7c16472f75d/> add detupling syntax and allow interleaving with flattening

### Refactor (BREAKING)

 - <csr-id-4386fac824d64f63eae7629292675ac6bc8df9f7/> change `lattice_bimorphism` to take state via singleton arguments #969 #1058
 - <csr-id-d7e579c39b370a0ea0b0385d1029e9f8a7351d68/> replace `state()` with `state_ref()` implementation

### `hydroflow` Commit Statistics

<csr-read-only-do-not-edit/>

 - 31 commits contributed to the release.
 - 44 days passed between releases.
 - 27 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 23 unique issues were worked on: [#1120](https://github.com/hydro-project/hydroflow/issues/1120), [#1143](https://github.com/hydro-project/hydroflow/issues/1143), [#1152](https://github.com/hydro-project/hydroflow/issues/1152), [#1159](https://github.com/hydro-project/hydroflow/issues/1159), [#1164](https://github.com/hydro-project/hydroflow/issues/1164), [#1166](https://github.com/hydro-project/hydroflow/issues/1166), [#1167](https://github.com/hydro-project/hydroflow/issues/1167), [#1171](https://github.com/hydro-project/hydroflow/issues/1171), [#1176](https://github.com/hydro-project/hydroflow/issues/1176), [#1178](https://github.com/hydro-project/hydroflow/issues/1178), [#1182](https://github.com/hydro-project/hydroflow/issues/1182), [#1190](https://github.com/hydro-project/hydroflow/issues/1190), [#1191](https://github.com/hydro-project/hydroflow/issues/1191), [#1192](https://github.com/hydro-project/hydroflow/issues/1192), [#1193](https://github.com/hydro-project/hydroflow/issues/1193), [#1196](https://github.com/hydro-project/hydroflow/issues/1196), [#1197](https://github.com/hydro-project/hydroflow/issues/1197), [#1198](https://github.com/hydro-project/hydroflow/issues/1198), [#1199](https://github.com/hydro-project/hydroflow/issues/1199), [#1204](https://github.com/hydro-project/hydroflow/issues/1204), [#1232](https://github.com/hydro-project/hydroflow/issues/1232), [#1236](https://github.com/hydro-project/hydroflow/issues/1236), [#1238](https://github.com/hydro-project/hydroflow/issues/1238)

### `hydroflow` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1120](https://github.com/hydro-project/hydroflow/issues/1120)**
    - Extend Chat Example w/ Gossip Based Backend ([`55051e1`](https://github.com/hydro-project/hydroflow/commit/55051e1c8db2ba8e36c6b6ed0e888996e9501f3f))
 * **[#1143](https://github.com/hydro-project/hydroflow/issues/1143)**
    - Added poll_futures and poll_futures_async operators. ([`997d90a`](https://github.com/hydro-project/hydroflow/commit/997d90a76db9a4e05dbac35073a09548750ce342))
 * **[#1152](https://github.com/hydro-project/hydroflow/issues/1152)**
    - Update some compile-fail outputs caused by rustc regression ([`d2427e2`](https://github.com/hydro-project/hydroflow/commit/d2427e2cc901c4174830d41b4a1dfc52fd4f19ce))
    - Update pyo3, silence warnings in generated code ([`1801502`](https://github.com/hydro-project/hydroflow/commit/18015029a725b068696ed9edefd1097583c858a6))
    - Appease latest nightly clippy ([`d9b2c02`](https://github.com/hydro-project/hydroflow/commit/d9b2c0263d508e6f6855f49504896d4ea670c355))
 * **[#1159](https://github.com/hydro-project/hydroflow/issues/1159)**
    - Change `lattice_bimorphism` to take state via singleton arguments #969 #1058 ([`4386fac`](https://github.com/hydro-project/hydroflow/commit/4386fac824d64f63eae7629292675ac6bc8df9f7))
    - Replace `state()` with `state_ref()` implementation ([`d7e579c`](https://github.com/hydro-project/hydroflow/commit/d7e579c39b370a0ea0b0385d1029e9f8a7351d68))
 * **[#1164](https://github.com/hydro-project/hydroflow/issues/1164)**
    - Improve `TeeingHandoff` drop implementation ([`b072ee0`](https://github.com/hydro-project/hydroflow/commit/b072ee026f97f8537165e1fb247101e0ab2fb320))
    - Fix `TeeingHandoff` not scheduling outputs `tee()`d after send port is already used via `add_subgraph*`, fix #1163 ([`69abccd`](https://github.com/hydro-project/hydroflow/commit/69abccde27644d7d0ed2fdf7d75f491fdbc41de8))
 * **[#1166](https://github.com/hydro-project/hydroflow/issues/1166)**
    - Add detupling syntax and allow interleaving with flattening ([`c9dc66d`](https://github.com/hydro-project/hydroflow/commit/c9dc66db2bde34aa1385c8847d0ba7c16472f75d))
 * **[#1167](https://github.com/hydro-project/hydroflow/issues/1167)**
    - Error on unused ports, fix #1108 ([`9df9c62`](https://github.com/hydro-project/hydroflow/commit/9df9c6251526903dbe7288e2fd9a532c63a9412c))
    - Reorder error message emission for conflicting varnames ([`20471f1`](https://github.com/hydro-project/hydroflow/commit/20471f11901e3fb15a2efea61752d836d4facba5))
 * **[#1171](https://github.com/hydro-project/hydroflow/issues/1171)**
    - Fix scheduler spinning on replay, fix #961 ([`d773f9a`](https://github.com/hydro-project/hydroflow/commit/d773f9a6938fe6d1521516f7a7c441c6c281a9fa))
    - Add tests for scheduler spinning #961 ([`8f4cd7b`](https://github.com/hydro-project/hydroflow/commit/8f4cd7bfc13bcbd89ff4a0e48c7e95cde5039f1b))
 * **[#1176](https://github.com/hydro-project/hydroflow/issues/1176)**
    - Fix #1050 for `reduce_keyed`, fix #1050 ([`e2fa6b0`](https://github.com/hydro-project/hydroflow/commit/e2fa6b0729cb92b29e1d293c2788458845ff306a))
 * **[#1178](https://github.com/hydro-project/hydroflow/issues/1178)**
    - Fix warning unnecessary qualification ([`a4cd5fe`](https://github.com/hydro-project/hydroflow/commit/a4cd5fe57130e3b517872d1c27a274f9c9f601c2))
 * **[#1182](https://github.com/hydro-project/hydroflow/issues/1182)**
    - Allow `defer_tick()`, `next_stratum()` to have a type hint arg ([`4ca8ce4`](https://github.com/hydro-project/hydroflow/commit/4ca8ce43c0998296e2d86bd74800585ebb24123a))
 * **[#1190](https://github.com/hydro-project/hydroflow/issues/1190)**
    - `source_interval` no longer emits Instant instances on output. ([`b292f11`](https://github.com/hydro-project/hydroflow/commit/b292f111715fd9c397ffb35cf991bd0bfb01c1e1))
 * **[#1191](https://github.com/hydro-project/hydroflow/issues/1191)**
    - Fixup! test(hydroflow): join replay static test ([`c09a23e`](https://github.com/hydro-project/hydroflow/commit/c09a23e854b10202dd7eec0ad0076de2bb196f61))
    - Join replay static test ([`f46e0ac`](https://github.com/hydro-project/hydroflow/commit/f46e0ac84e13ad6d713836c948b02e7b199f5faa))
 * **[#1192](https://github.com/hydro-project/hydroflow/issues/1192)**
    - Expect custom config names to prevent warnings ([`b86f11a`](https://github.com/hydro-project/hydroflow/commit/b86f11aad344fef6ad9cdd1db0b45bb738c48bd6))
 * **[#1193](https://github.com/hydro-project/hydroflow/issues/1193)**
    - Introduce newtypes for working with ticks ([`c2f6c95`](https://github.com/hydro-project/hydroflow/commit/c2f6c9578127a71c879752d52e115df75659e2b0))
 * **[#1196](https://github.com/hydro-project/hydroflow/issues/1196)**
    - Change current_tick_start to wall clock time ([`218175c`](https://github.com/hydro-project/hydroflow/commit/218175cd935c4d94efb2f01f6e73300ace978283))
 * **[#1197](https://github.com/hydro-project/hydroflow/issues/1197)**
    - Fixup! test(hydroflow): join replay static test ([`c09a23e`](https://github.com/hydro-project/hydroflow/commit/c09a23e854b10202dd7eec0ad0076de2bb196f61))
 * **[#1198](https://github.com/hydro-project/hydroflow/issues/1198)**
    - Insert `persist()` before singleton referencers to enable replay ([`f184ea1`](https://github.com/hydro-project/hydroflow/commit/f184ea145f0c7c3072d7d0f94d42fcda717ac8d9))
 * **[#1199](https://github.com/hydro-project/hydroflow/issues/1199)**
    - Use `Duration` instead of int seconds ([`b649d47`](https://github.com/hydro-project/hydroflow/commit/b649d470cc28a2bbb7e31426adcd050b455bf803))
    - Improve docs/README.md, fix #856 ([`9bcc526`](https://github.com/hydro-project/hydroflow/commit/9bcc5265044b467e9fe4ada0a89c7e84fd66dd43))
 * **[#1204](https://github.com/hydro-project/hydroflow/issues/1204)**
    - Simplify `demux_enum()`, somewhat improves error messages #1201 ([`826dbd9`](https://github.com/hydro-project/hydroflow/commit/826dbd9a709de2f883992bdcefa8f2d566d74ecb))
 * **[#1232](https://github.com/hydro-project/hydroflow/issues/1232)**
    - Add `'static`/`'tick` support for `state()` operator ([`b0692b0`](https://github.com/hydro-project/hydroflow/commit/b0692b0d697980eaf9893c07a443a257e04786c5))
 * **[#1236](https://github.com/hydro-project/hydroflow/issues/1236)**
    - Updating CONTRIBUTING.md with some info about feature branches ([`0d2f14b`](https://github.com/hydro-project/hydroflow/commit/0d2f14b9237c0eaa8131d1d1118768357ac8133b))
 * **[#1238](https://github.com/hydro-project/hydroflow/issues/1238)**
    - Use workaround for `cargo smart-release` not properly ordering `dev-`/`build-dependencies` ([`c9dfddc`](https://github.com/hydro-project/hydroflow/commit/c9dfddc680e0ce5415539d7b77bc5beb97ab59d9))
 * **Uncategorized**
    - Release hydroflow_lang v0.7.0, hydroflow_datalog_core v0.7.0, hydroflow_datalog v0.7.0, hydroflow_macro v0.7.0, lattices v0.5.5, multiplatform_test v0.1.0, pusherator v0.0.6, hydroflow v0.7.0, stageleft_macro v0.2.0, stageleft v0.3.0, stageleft_tool v0.2.0, hydroflow_plus v0.7.0, hydro_deploy v0.7.0, hydro_cli v0.7.0, hydroflow_plus_cli_integration v0.7.0, safety bump 8 crates ([`2852147`](https://github.com/hydro-project/hydroflow/commit/285214740627685e911781793e05d234ab2ad2bd))
</details>

## 0.6.2 (2024-04-09)

<csr-id-d9f72a62c0099d69986fe28f484bb2bf574caaac/>

### New Features

<csr-id-0f16d1f50cd64d9ca52ec811acc4a643a86f14fe/>
<csr-id-5c5b6523d96a22c97382a7c61ee0e36ad77c0a0f/>

 - <csr-id-5679bfb7d3b96089cc020308c7d88021a254e63c/> allow `reduce()` to be referenceable as a singleton, fix docs and bugs
   * fixed bug: accumulator closures could have return values, which would be ignored
* updated docs

### Bug Fixes

 - <csr-id-82b3030eefb759a97053057f717efb95491802b3/> fix singleton reference edges being backwards, fix #1147

### Test

 - <csr-id-d9f72a62c0099d69986fe28f484bb2bf574caaac/> add tests for #1147 (failing)

### `hydroflow` Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release.
 - 3 days passed between releases.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 3 unique issues were worked on: [#1134](https://github.com/hydro-project/hydroflow/issues/1134), [#1148](https://github.com/hydro-project/hydroflow/issues/1148), [#1150](https://github.com/hydro-project/hydroflow/issues/1150)

### `hydroflow` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1134](https://github.com/hydro-project/hydroflow/issues/1134)**
    - Make fold output optional, usable as just a singleton reference ([`0f16d1f`](https://github.com/hydro-project/hydroflow/commit/0f16d1f50cd64d9ca52ec811acc4a643a86f14fe))
    - Allow `fold()` to be referenceable as a singleton ([`5c5b652`](https://github.com/hydro-project/hydroflow/commit/5c5b6523d96a22c97382a7c61ee0e36ad77c0a0f))
 * **[#1148](https://github.com/hydro-project/hydroflow/issues/1148)**
    - Fix singleton reference edges being backwards, fix #1147 ([`82b3030`](https://github.com/hydro-project/hydroflow/commit/82b3030eefb759a97053057f717efb95491802b3))
    - Add tests for #1147 (failing) ([`d9f72a6`](https://github.com/hydro-project/hydroflow/commit/d9f72a62c0099d69986fe28f484bb2bf574caaac))
 * **[#1150](https://github.com/hydro-project/hydroflow/issues/1150)**
    - Allow `reduce()` to be referenceable as a singleton, fix docs and bugs ([`5679bfb`](https://github.com/hydro-project/hydroflow/commit/5679bfb7d3b96089cc020308c7d88021a254e63c))
 * **Uncategorized**
    - Release hydroflow_lang v0.6.2, hydroflow v0.6.2, hydroflow_plus v0.6.1, hydro_deploy v0.6.1, hydro_cli v0.6.1, hydroflow_plus_cli_integration v0.6.1, stageleft_tool v0.1.1 ([`23cfe08`](https://github.com/hydro-project/hydroflow/commit/23cfe0839079aa17d042bbd3976f6d188689d290))
</details>

## 0.6.1 (2024-04-05)

<csr-id-fc447ffdf8fd1b2189545a991f08588238182f00/>
<csr-id-1b19361a87bafb7d7c12be04cc2ce3370d71439a/>
<csr-id-8a22b3a27282b5076db1c89bc3a8572dc16a5e7c/>
<csr-id-0ac2d46dcfc9756219806663042eca18e4eeff3f/>
<csr-id-be8d767aa6347f6df57908d1e9beae7224ea2e53/>
<csr-id-76e501a35ba47803b69ea62920aae570fb60e2f0/>
<csr-id-7958fb0d900be8fe7359326abfa11dcb8fb35e8a/>
<csr-id-99bbe50a6cd214a974aec72d95bf10b9b6da33f6/>
<csr-id-118a563dbe627e730797128098698739db5588e5/>

### Chore

 - <csr-id-fc447ffdf8fd1b2189545a991f08588238182f00/> appease latest nightly clippy
   Also updates `surface_keyed_fold.rs` `test_fold_keyed_infer_basic` test.

### Documentation

 - <csr-id-550b17c0e0edd233b0cceab55fed0de309051503/> more improvements to `persist_mut[_keyed]` docs

### New Features

<csr-id-61cb55a04e518560beff1ea5d446927b6d7096b3/>
<csr-id-fd90d4130e86b16139f6d3a386fbbfb49fae5a9b/>
<csr-id-99657d528c36e4e25b7eaa536c480e7b3d859443/>

 - <csr-id-1bde8a2443523fa8d3aafd7459c9a484de7724b7/> Render singleton references in graphvis
 - <csr-id-54c6874931ebd7ba13140dbcf3e75ad9b2852331/> enable inspect to have no ouputs
 - <csr-id-fa34dd317ab0cb59753bd884acae2605a7630b10/> track which ops have singleton state
   Add a nice error message for referencing a non-singleton op
 - <csr-id-601c484bf9afe9aada61c1a40478c0e093140a56/> improve singleton error messages/handling
 - <csr-id-8cb29fa809e255195b65fd23b589bf7c970c5599/> enable singleton reference usage in all operators
   also cleans up local imports
 - <csr-id-f9b26b6df7ff7d2a3530527579c60f41d93e9d45/> switch singletons references to being per-op intead of per-edge
   Only testing in `filter` op for now
 - <csr-id-0a5e785d4bf0729ba0fe95466c822e8304f96e24/> initial proof-of-concept for singletons
 - <csr-id-602e21fc04da5a0d65ac7128688760c92ffba3c1/> include subgraph ID in pivot_run codegen
 - <csr-id-68d8ffcfb8f378e603d99bd6cac4c0ce69dc25ba/> switch launch to be a macro to enable borrowing locals
 - <csr-id-916a366cba94c739a6e170d281663f19f706d6be/> add syntax for "splatting" columns
   feat(hydroflow_datalog): add syntax for "splatting" columns
   
   The * operator can be used on the left-hand side of a rule to indicate
   that each row should be duplicated into many rows with that column being
   replaced by every value in iterable corresponding to the original
   expression.
 - <csr-id-31a543c236b874a60b7a152f11377013bcd15221/> allow dropping of tee handoff output to prevent memory leak
 - <csr-id-5f21e4c8c565f1c87694d692fcec33ce1cb9d77f/> implement teeing handoff for scheduled layer
   - Cleanup/update scheduled layer `TeeingHandoff` tests

### Bug Fixes

 - <csr-id-f29c710e62d45a819b0c44125f94be207ffd5d83/> `HandoffMeta::is_bottom` impl for `TeeingHandoff` [ci-bench]
 - <csr-id-f8311db2cd9628607887fc04f2ea5933c8b7c11e/> #1050 for `fold_keyed`

### Refactor

 - <csr-id-1b19361a87bafb7d7c12be04cc2ce3370d71439a/> consider singleton references as `DelayType::Stratum` barriers
 - <csr-id-8a22b3a27282b5076db1c89bc3a8572dc16a5e7c/> Cleanup user-facing teeing APIs
 - <csr-id-0ac2d46dcfc9756219806663042eca18e4eeff3f/> Teeing overhead moved to graph assembly time, instead of scheduler
 - <csr-id-be8d767aa6347f6df57908d1e9beae7224ea2e53/> use smallvec `[_; 1]` for handoff pred(s)/succ(s) [ci-bench]

### Style

 - <csr-id-76e501a35ba47803b69ea62920aae570fb60e2f0/> `Never` -> `std::convert::Infallible`
 - <csr-id-7958fb0d900be8fe7359326abfa11dcb8fb35e8a/> qualified path cleanups for clippy

### Test

 - <csr-id-99bbe50a6cd214a974aec72d95bf10b9b6da33f6/> fix some `surface_codegen.rs` tests, actually test output, fix #1096
 - <csr-id-118a563dbe627e730797128098698739db5588e5/> add hydroflow scheduling spin test for #1050 #800

### `hydroflow` Commit Statistics

<csr-read-only-do-not-edit/>

 - 28 commits contributed to the release.
 - 34 days passed between releases.
 - 27 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 18 unique issues were worked on: [#1068](https://github.com/hydro-project/hydroflow/issues/1068), [#1086](https://github.com/hydro-project/hydroflow/issues/1086), [#1087](https://github.com/hydro-project/hydroflow/issues/1087), [#1089](https://github.com/hydro-project/hydroflow/issues/1089), [#1090](https://github.com/hydro-project/hydroflow/issues/1090), [#1091](https://github.com/hydro-project/hydroflow/issues/1091), [#1093](https://github.com/hydro-project/hydroflow/issues/1093), [#1094](https://github.com/hydro-project/hydroflow/issues/1094), [#1102](https://github.com/hydro-project/hydroflow/issues/1102), [#1113](https://github.com/hydro-project/hydroflow/issues/1113), [#1125](https://github.com/hydro-project/hydroflow/issues/1125), [#1128](https://github.com/hydro-project/hydroflow/issues/1128), [#1132](https://github.com/hydro-project/hydroflow/issues/1132), [#1133](https://github.com/hydro-project/hydroflow/issues/1133), [#1137](https://github.com/hydro-project/hydroflow/issues/1137), [#1140](https://github.com/hydro-project/hydroflow/issues/1140), [#1145](https://github.com/hydro-project/hydroflow/issues/1145), [#1146](https://github.com/hydro-project/hydroflow/issues/1146)

### `hydroflow` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1068](https://github.com/hydro-project/hydroflow/issues/1068)**
    - `HandoffMeta::is_bottom` impl for `TeeingHandoff` [ci-bench] ([`f29c710`](https://github.com/hydro-project/hydroflow/commit/f29c710e62d45a819b0c44125f94be207ffd5d83))
    - Cleanup user-facing teeing APIs ([`8a22b3a`](https://github.com/hydro-project/hydroflow/commit/8a22b3a27282b5076db1c89bc3a8572dc16a5e7c))
    - Teeing overhead moved to graph assembly time, instead of scheduler ([`0ac2d46`](https://github.com/hydro-project/hydroflow/commit/0ac2d46dcfc9756219806663042eca18e4eeff3f))
    - Allow dropping of tee handoff output to prevent memory leak ([`31a543c`](https://github.com/hydro-project/hydroflow/commit/31a543c236b874a60b7a152f11377013bcd15221))
    - Implement teeing handoff for scheduled layer ([`5f21e4c`](https://github.com/hydro-project/hydroflow/commit/5f21e4c8c565f1c87694d692fcec33ce1cb9d77f))
 * **[#1086](https://github.com/hydro-project/hydroflow/issues/1086)**
    - #1050 for `fold_keyed` ([`f8311db`](https://github.com/hydro-project/hydroflow/commit/f8311db2cd9628607887fc04f2ea5933c8b7c11e))
    - Scheduler `log` subgraph name (which includes surface syntax sg id) ([`99657d5`](https://github.com/hydro-project/hydroflow/commit/99657d528c36e4e25b7eaa536c480e7b3d859443))
    - Add hydroflow scheduling spin test for #1050 #800 ([`118a563`](https://github.com/hydro-project/hydroflow/commit/118a563dbe627e730797128098698739db5588e5))
 * **[#1087](https://github.com/hydro-project/hydroflow/issues/1087)**
    - Add `kvs_mut` example, fix #785 ([`fd90d41`](https://github.com/hydro-project/hydroflow/commit/fd90d4130e86b16139f6d3a386fbbfb49fae5a9b))
 * **[#1089](https://github.com/hydro-project/hydroflow/issues/1089)**
    - More improvements to `persist_mut[_keyed]` docs ([`550b17c`](https://github.com/hydro-project/hydroflow/commit/550b17c0e0edd233b0cceab55fed0de309051503))
 * **[#1090](https://github.com/hydro-project/hydroflow/issues/1090)**
    - Qualified path cleanups for clippy ([`7958fb0`](https://github.com/hydro-project/hydroflow/commit/7958fb0d900be8fe7359326abfa11dcb8fb35e8a))
 * **[#1091](https://github.com/hydro-project/hydroflow/issues/1091)**
    - Switch singletons references to being per-op intead of per-edge ([`f9b26b6`](https://github.com/hydro-project/hydroflow/commit/f9b26b6df7ff7d2a3530527579c60f41d93e9d45))
    - Initial proof-of-concept for singletons ([`0a5e785`](https://github.com/hydro-project/hydroflow/commit/0a5e785d4bf0729ba0fe95466c822e8304f96e24))
 * **[#1093](https://github.com/hydro-project/hydroflow/issues/1093)**
    - `Never` -> `std::convert::Infallible` ([`76e501a`](https://github.com/hydro-project/hydroflow/commit/76e501a35ba47803b69ea62920aae570fb60e2f0))
 * **[#1094](https://github.com/hydro-project/hydroflow/issues/1094)**
    - Add aggregation for collecting values into a vector ([`61cb55a`](https://github.com/hydro-project/hydroflow/commit/61cb55a04e518560beff1ea5d446927b6d7096b3))
 * **[#1102](https://github.com/hydro-project/hydroflow/issues/1102)**
    - Switch launch to be a macro to enable borrowing locals ([`68d8ffc`](https://github.com/hydro-project/hydroflow/commit/68d8ffcfb8f378e603d99bd6cac4c0ce69dc25ba))
 * **[#1113](https://github.com/hydro-project/hydroflow/issues/1113)**
    - Use smallvec `[_; 1]` for handoff pred(s)/succ(s) [ci-bench] ([`be8d767`](https://github.com/hydro-project/hydroflow/commit/be8d767aa6347f6df57908d1e9beae7224ea2e53))
 * **[#1125](https://github.com/hydro-project/hydroflow/issues/1125)**
    - Fix some `surface_codegen.rs` tests, actually test output, fix #1096 ([`99bbe50`](https://github.com/hydro-project/hydroflow/commit/99bbe50a6cd214a974aec72d95bf10b9b6da33f6))
 * **[#1128](https://github.com/hydro-project/hydroflow/issues/1128)**
    - Enable inspect to have no ouputs ([`54c6874`](https://github.com/hydro-project/hydroflow/commit/54c6874931ebd7ba13140dbcf3e75ad9b2852331))
 * **[#1132](https://github.com/hydro-project/hydroflow/issues/1132)**
    - Add syntax for "splatting" columns ([`916a366`](https://github.com/hydro-project/hydroflow/commit/916a366cba94c739a6e170d281663f19f706d6be))
 * **[#1133](https://github.com/hydro-project/hydroflow/issues/1133)**
    - Track which ops have singleton state ([`fa34dd3`](https://github.com/hydro-project/hydroflow/commit/fa34dd317ab0cb59753bd884acae2605a7630b10))
    - Improve singleton error messages/handling ([`601c484`](https://github.com/hydro-project/hydroflow/commit/601c484bf9afe9aada61c1a40478c0e093140a56))
    - Enable singleton reference usage in all operators ([`8cb29fa`](https://github.com/hydro-project/hydroflow/commit/8cb29fa809e255195b65fd23b589bf7c970c5599))
 * **[#1137](https://github.com/hydro-project/hydroflow/issues/1137)**
    - Include subgraph ID in pivot_run codegen ([`602e21f`](https://github.com/hydro-project/hydroflow/commit/602e21fc04da5a0d65ac7128688760c92ffba3c1))
 * **[#1140](https://github.com/hydro-project/hydroflow/issues/1140)**
    - Appease latest nightly clippy ([`fc447ff`](https://github.com/hydro-project/hydroflow/commit/fc447ffdf8fd1b2189545a991f08588238182f00))
 * **[#1145](https://github.com/hydro-project/hydroflow/issues/1145)**
    - Consider singleton references as `DelayType::Stratum` barriers ([`1b19361`](https://github.com/hydro-project/hydroflow/commit/1b19361a87bafb7d7c12be04cc2ce3370d71439a))
 * **[#1146](https://github.com/hydro-project/hydroflow/issues/1146)**
    - Render singleton references in graphvis ([`1bde8a2`](https://github.com/hydro-project/hydroflow/commit/1bde8a2443523fa8d3aafd7459c9a484de7724b7))
 * **Uncategorized**
    - Release hydroflow_cli_integration v0.5.2, hydroflow_lang v0.6.1, hydroflow_datalog_core v0.6.1, lattices v0.5.4, hydroflow v0.6.1, stageleft_macro v0.1.1, stageleft v0.2.1, hydroflow_plus v0.6.1, hydro_deploy v0.6.1, hydro_cli v0.6.1, hydroflow_plus_cli_integration v0.6.1, stageleft_tool v0.1.1 ([`cd63f22`](https://github.com/hydro-project/hydroflow/commit/cd63f2258c961a40f0e5dbef20ac329a2d570ad0))
</details>

## 0.6.0 (2024-03-02)

<csr-id-5a451ac4ae75024153a06416fc81d834d1fdae6f/>
<csr-id-39ab8b0278e9e3fe96552ace0a4ae768a6bc10d8/>
<csr-id-92ee7eb4e81ff2852f873d6a84a44a5a5bdc6703/>
<csr-id-8361f9c8f83992295cfa40cf4d27a17146afb2d9/>
<csr-id-e9639f608f8dafd3f384837067800a66951b25df/>
<csr-id-71353f0d4dfd9766dfdc715c4a91a028081f910f/>
<csr-id-b391447ec13f1f79c99142f296dc2fa8640034f4/>

### Chore

 - <csr-id-5a451ac4ae75024153a06416fc81d834d1fdae6f/> prep for 0.0.4 release
 - <csr-id-39ab8b0278e9e3fe96552ace0a4ae768a6bc10d8/> appease various clippy lints
 - <csr-id-92ee7eb4e81ff2852f873d6a84a44a5a5bdc6703/> remove unused multiset2 half join state
 - <csr-id-8361f9c8f83992295cfa40cf4d27a17146afb2d9/> update pinned rust nightly version to 2024-02-03

### New Features

 - <csr-id-09f0c5711c2d038d1d5764d400fe6206da995ce3/> add (inefficient) `lattice_bimorphism` operator, fix #1073

### Bug Fixes

 - <csr-id-18ee2ad245dad997494267050a9568f20cbe215f/> cleanup timing in some `surface_async` tests, #1078

### Other

 - <csr-id-e9639f608f8dafd3f384837067800a66951b25df/> consolidate tasks and use sccache and nextest

### Style

 - <csr-id-71353f0d4dfd9766dfdc715c4a91a028081f910f/> fix imports for clippy
 - <csr-id-b391447ec13f1f79c99142f296dc2fa8640034f4/> fix imports

### `hydroflow` Commit Statistics

<csr-read-only-do-not-edit/>

 - 10 commits contributed to the release.
 - 28 days passed between releases.
 - 9 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 6 unique issues were worked on: [#1015](https://github.com/hydro-project/hydroflow/issues/1015), [#1057](https://github.com/hydro-project/hydroflow/issues/1057), [#1060](https://github.com/hydro-project/hydroflow/issues/1060), [#1061](https://github.com/hydro-project/hydroflow/issues/1061), [#1084](https://github.com/hydro-project/hydroflow/issues/1084), [#1085](https://github.com/hydro-project/hydroflow/issues/1085)

### `hydroflow` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1015](https://github.com/hydro-project/hydroflow/issues/1015)**
    - Consolidate tasks and use sccache and nextest ([`e9639f6`](https://github.com/hydro-project/hydroflow/commit/e9639f608f8dafd3f384837067800a66951b25df))
 * **[#1057](https://github.com/hydro-project/hydroflow/issues/1057)**
    - Update pinned rust nightly version to 2024-02-03 ([`8361f9c`](https://github.com/hydro-project/hydroflow/commit/8361f9c8f83992295cfa40cf4d27a17146afb2d9))
 * **[#1060](https://github.com/hydro-project/hydroflow/issues/1060)**
    - Remove unused multiset2 half join state ([`92ee7eb`](https://github.com/hydro-project/hydroflow/commit/92ee7eb4e81ff2852f873d6a84a44a5a5bdc6703))
 * **[#1061](https://github.com/hydro-project/hydroflow/issues/1061)**
    - Add (inefficient) `lattice_bimorphism` operator, fix #1073 ([`09f0c57`](https://github.com/hydro-project/hydroflow/commit/09f0c5711c2d038d1d5764d400fe6206da995ce3))
 * **[#1084](https://github.com/hydro-project/hydroflow/issues/1084)**
    - Appease various clippy lints ([`39ab8b0`](https://github.com/hydro-project/hydroflow/commit/39ab8b0278e9e3fe96552ace0a4ae768a6bc10d8))
 * **[#1085](https://github.com/hydro-project/hydroflow/issues/1085)**
    - Cleanup timing in some `surface_async` tests, #1078 ([`18ee2ad`](https://github.com/hydro-project/hydroflow/commit/18ee2ad245dad997494267050a9568f20cbe215f))
 * **Uncategorized**
    - Release hydroflow_lang v0.6.0, hydroflow_datalog_core v0.6.0, hydroflow_datalog v0.6.0, hydroflow_macro v0.6.0, lattices v0.5.3, variadics v0.0.4, pusherator v0.0.5, hydroflow v0.6.0, stageleft v0.2.0, hydroflow_plus v0.6.0, hydro_deploy v0.6.0, hydro_cli v0.6.0, hydroflow_plus_cli_integration v0.6.0, safety bump 7 crates ([`09ea65f`](https://github.com/hydro-project/hydroflow/commit/09ea65fe9cd45c357c43bffca30e60243fa45cc8))
    - Prep for 0.0.4 release ([`5a451ac`](https://github.com/hydro-project/hydroflow/commit/5a451ac4ae75024153a06416fc81d834d1fdae6f))
    - Fix imports for clippy ([`71353f0`](https://github.com/hydro-project/hydroflow/commit/71353f0d4dfd9766dfdc715c4a91a028081f910f))
    - Fix imports ([`b391447`](https://github.com/hydro-project/hydroflow/commit/b391447ec13f1f79c99142f296dc2fa8640034f4))
</details>

## 0.5.2 (2024-02-02)

### Documentation

 - <csr-id-fe2b4a7a865c32849582b27f8866f77af3637cf0/> add explanation for outputing synchronously from Hydroflow, fix #342

### New Features

 - <csr-id-5cbce2bac61130e3aefc7e73a9bc9080bd2b5a32/> add `StateHandleErased` (no type param) for storage in data structures, fix #1048

### Bug Fixes

 - <csr-id-a572dee118fb9d014954a18d560413a6c6fc5deb/> fix coloring of `_lattice_join_fused_join`, fix #943
 - <csr-id-175cc20e15b90627ae86d488e31ec91278c8beeb/> ensure other graph methods handle edge types
   (#929)

### `hydroflow` Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release.
 - 4 days passed between releases.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 4 unique issues were worked on: [#1041](https://github.com/hydro-project/hydroflow/issues/1041), [#1051](https://github.com/hydro-project/hydroflow/issues/1051), [#1054](https://github.com/hydro-project/hydroflow/issues/1054), [#1055](https://github.com/hydro-project/hydroflow/issues/1055)

### `hydroflow` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1041](https://github.com/hydro-project/hydroflow/issues/1041)**
    - Ensure other graph methods handle edge types ([`175cc20`](https://github.com/hydro-project/hydroflow/commit/175cc20e15b90627ae86d488e31ec91278c8beeb))
 * **[#1051](https://github.com/hydro-project/hydroflow/issues/1051)**
    - Add explanation for outputing synchronously from Hydroflow, fix #342 ([`fe2b4a7`](https://github.com/hydro-project/hydroflow/commit/fe2b4a7a865c32849582b27f8866f77af3637cf0))
 * **[#1054](https://github.com/hydro-project/hydroflow/issues/1054)**
    - Add `StateHandleErased` (no type param) for storage in data structures, fix #1048 ([`5cbce2b`](https://github.com/hydro-project/hydroflow/commit/5cbce2bac61130e3aefc7e73a9bc9080bd2b5a32))
 * **[#1055](https://github.com/hydro-project/hydroflow/issues/1055)**
    - Fix coloring of `_lattice_join_fused_join`, fix #943 ([`a572dee`](https://github.com/hydro-project/hydroflow/commit/a572dee118fb9d014954a18d560413a6c6fc5deb))
 * **Uncategorized**
    - Release hydroflow_lang v0.5.2, hydroflow_datalog_core v0.5.2, hydroflow_macro v0.5.2, lattices v0.5.2, hydroflow v0.5.2, hydro_cli v0.5.1, hydroflow_plus_cli_integration v0.5.1 ([`6ac8720`](https://github.com/hydro-project/hydroflow/commit/6ac872081753548ebb8ec95549b4d820dc050d3e))
</details>

## 0.5.1 (2024-01-29)

<csr-id-1b555e57c8c812bed4d6495d2960cbf77fb0b3ef/>
<csr-id-ba6afab8416ad66eee4fdb9d0c73e62d45752617/>
<csr-id-f6a729925ddeb6063fa8c4b03d6621c1c35f0cc8/>
<csr-id-6bf846f9ce72688fccd9947cf263f8c0ebba4f3a/>
<csr-id-26818be2ec47f936db9de770e935ab98ee26392b/>
<csr-id-7e65a08711775656e435e854777c5f089dd31a05/>
<csr-id-7e1eae6b708ef89c833cd2d2c2e5112c2bd84395/>

### Chore

 - <csr-id-1b555e57c8c812bed4d6495d2960cbf77fb0b3ef/> manually set lockstep-versioned crates (and `lattices`) to version `0.5.1`
   Setting manually since
   https://github.com/frewsxcv/rust-crates-index/issues/159 is messing with
   smart-release
 - <csr-id-ba6afab8416ad66eee4fdb9d0c73e62d45752617/> fix clippy lints on latest nightly
 - <csr-id-f6a729925ddeb6063fa8c4b03d6621c1c35f0cc8/> fix `clippy::items_after_test_module`, simplify rustdoc links
 - <csr-id-6bf846f9ce72688fccd9947cf263f8c0ebba4f3a/> update trybuild output for nightly bump

### New Features

 - <csr-id-5a03ed41548b5766b945efbd1eedb0dfceb714d9/> add core negation operators
 - <csr-id-1edc5ae5b5f70e1390183e8c8eb27eb0ab32196d/> provide simpler API for launching and minimize dependencies
 - <csr-id-46d87fa364d3fe01422cf3c404fbc8a1d5e9fb88/> pass subgraph ID through deploy metadata
 - <csr-id-c50ca121b6d5e30dc07843f82caa135b68626301/> split Rust core from Python bindings
 - <csr-id-174607d12277d7544d0f42890c9a5da2ff184df4/> support building graphs for symmetric clusters in Hydroflow+
 - <csr-id-e5bdd12e32d6ea72fd91a55c12e09f07a0edaa5c/> add initial test using Hydro CLI from Hydroflow+
   This also required a change to Hydroflow core to make it possible to run the dataflow itself on a single thread (using a LocalSet), even if the surrounding runtime is not single-threaded (required to work around deadlocks because we can't use async APIs inside Hydroflow+). This requires us to spawn any Hydroflow tasks (only for `dest_sink` at the moment) right next to when we run the dataflow rather than when the Hydroflow graph is initialized. From a conceptual perspective, this seems _more right_, since now creating a Hydroflow program will not result in any actual tasks running.
   
   In the third PR of this series, I aim to add a new Hydroflow+ operator that will automate the setup of a `dest_sink`/`source_stream` pair that span nodes.
 - <csr-id-6158a7aae2ef9b58245c23fc668715a3fb2ff7dc/> new implementation and Hydro Deploy setup
   --
 - <csr-id-8b635683e5ac3c4ed2d896ae88e2953db1c6312c/> add a functional surface syntax using staging
 - <csr-id-f327b02c8001129e619fb253ab9b6d550e229a48/> add left,right,outer join module examples
 - <csr-id-7df0a0df61597764eed763b68138929fed1413ac/> add defer() which is the same as defer_tick() except that it is lazy
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
 - <csr-id-6158a7aae2ef9b58245c23fc668715a3fb2ff7dc/> new implementation and Hydro Deploy setup
   --

### Bug Fixes

<csr-id-45452ae7f984b6677a430fca7d7fad9478998495/>
<csr-id-01dec519d9be015b68a965c33926dd8bf9eae185/>
<csr-id-f0a03786b47d590477f8169bb0a40fd4981fef9e/>
<csr-id-bc35a5a5e05ccc3990bb3c430129f0a735bc8c0a/>
<csr-id-0539e2a91eb3ba71ed1c9fbe8d0c74b6344ad1bf/>
<csr-id-43280cb698cf6bc070483365ee272106c271dca4/>
<csr-id-35b1e9e83f2a0cfa171b4994a2cffb0d22706abf/>

 - <csr-id-5ed9be478daf4fef91c6d35893f68944da8eac94/> lattice ops are monotone and return stream of lattices
   * fix: lattice ops are monotone and return stream of lattices
* fix: remove trailing whitespace
* fix: remove warnings from unused tees in topolotree
* fix: avoid error message in test
* fix: set `flow_prop_fn` properly in `_lattice_fold_batch.rs`
* fix: test_lattice_join_fused_join_map_union now checks assertions

### Refactor

 - <csr-id-26818be2ec47f936db9de770e935ab98ee26392b/> Update `surface_lattice_join` tests to use `DeepReveal`
 - <csr-id-7e65a08711775656e435e854777c5f089dd31a05/> Improvements prepping for release
   - Adds the "spread"/"splat" `...` syntax to the three variadics macros.
   - Adds `#[sealed]` traits.
   - Adds testing of error messages.
   - Improves docs: `README.md` and Rust docs.
 - <csr-id-7e1eae6b708ef89c833cd2d2c2e5112c2bd84395/> remove intrinsics from KVS bench, fixes warning

### Bug Fixes (BREAKING)

 - <csr-id-3136e0f286f87e944e7f718d926fd7670b44194b/> fold takes initial value by closure rather than by value

### `hydroflow` Commit Statistics

<csr-read-only-do-not-edit/>

 - 27 commits contributed to the release.
 - 110 days passed between releases.
 - 26 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 23 unique issues were worked on: [#1003](https://github.com/hydro-project/hydroflow/issues/1003), [#1005](https://github.com/hydro-project/hydroflow/issues/1005), [#1024](https://github.com/hydro-project/hydroflow/issues/1024), [#1025](https://github.com/hydro-project/hydroflow/issues/1025), [#1026](https://github.com/hydro-project/hydroflow/issues/1026), [#1032](https://github.com/hydro-project/hydroflow/issues/1032), [#1036](https://github.com/hydro-project/hydroflow/issues/1036), [#899](https://github.com/hydro-project/hydroflow/issues/899), [#909](https://github.com/hydro-project/hydroflow/issues/909), [#942](https://github.com/hydro-project/hydroflow/issues/942), [#945](https://github.com/hydro-project/hydroflow/issues/945), [#948](https://github.com/hydro-project/hydroflow/issues/948), [#950](https://github.com/hydro-project/hydroflow/issues/950), [#959](https://github.com/hydro-project/hydroflow/issues/959), [#960](https://github.com/hydro-project/hydroflow/issues/960), [#967](https://github.com/hydro-project/hydroflow/issues/967), [#971](https://github.com/hydro-project/hydroflow/issues/971), [#974](https://github.com/hydro-project/hydroflow/issues/974), [#978](https://github.com/hydro-project/hydroflow/issues/978), [#979](https://github.com/hydro-project/hydroflow/issues/979), [#984](https://github.com/hydro-project/hydroflow/issues/984), [#986](https://github.com/hydro-project/hydroflow/issues/986), [#996](https://github.com/hydro-project/hydroflow/issues/996)

### `hydroflow` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1003](https://github.com/hydro-project/hydroflow/issues/1003)**
    - Provide simpler API for launching and minimize dependencies ([`1edc5ae`](https://github.com/hydro-project/hydroflow/commit/1edc5ae5b5f70e1390183e8c8eb27eb0ab32196d))
 * **[#1005](https://github.com/hydro-project/hydroflow/issues/1005)**
    - Improve type inference for fold accumulators ([`f0a0378`](https://github.com/hydro-project/hydroflow/commit/f0a03786b47d590477f8169bb0a40fd4981fef9e))
 * **[#1024](https://github.com/hydro-project/hydroflow/issues/1024)**
    - Remove stale instructions about python driver for chat ([`01dec51`](https://github.com/hydro-project/hydroflow/commit/01dec519d9be015b68a965c33926dd8bf9eae185))
 * **[#1025](https://github.com/hydro-project/hydroflow/issues/1025)**
    - Re-export `hydroflow_macro::DemuxEnum` derive macro ([`45452ae`](https://github.com/hydro-project/hydroflow/commit/45452ae7f984b6677a430fca7d7fad9478998495))
 * **[#1026](https://github.com/hydro-project/hydroflow/issues/1026)**
    - Lattice ops are monotone and return stream of lattices ([`5ed9be4`](https://github.com/hydro-project/hydroflow/commit/5ed9be478daf4fef91c6d35893f68944da8eac94))
 * **[#1032](https://github.com/hydro-project/hydroflow/issues/1032)**
    - Update `surface_lattice_join` tests to use `DeepReveal` ([`26818be`](https://github.com/hydro-project/hydroflow/commit/26818be2ec47f936db9de770e935ab98ee26392b))
 * **[#1036](https://github.com/hydro-project/hydroflow/issues/1036)**
    - Add core negation operators ([`5a03ed4`](https://github.com/hydro-project/hydroflow/commit/5a03ed41548b5766b945efbd1eedb0dfceb714d9))
 * **[#899](https://github.com/hydro-project/hydroflow/issues/899)**
    - Add a functional surface syntax using staging ([`8b63568`](https://github.com/hydro-project/hydroflow/commit/8b635683e5ac3c4ed2d896ae88e2953db1c6312c))
 * **[#909](https://github.com/hydro-project/hydroflow/issues/909)**
    - New implementation and Hydro Deploy setup ([`6158a7a`](https://github.com/hydro-project/hydroflow/commit/6158a7aae2ef9b58245c23fc668715a3fb2ff7dc))
 * **[#942](https://github.com/hydro-project/hydroflow/issues/942)**
    - Fix `clippy::items_after_test_module`, simplify rustdoc links ([`f6a7299`](https://github.com/hydro-project/hydroflow/commit/f6a729925ddeb6063fa8c4b03d6621c1c35f0cc8))
    - Update trybuild output for nightly bump ([`6bf846f`](https://github.com/hydro-project/hydroflow/commit/6bf846f9ce72688fccd9947cf263f8c0ebba4f3a))
 * **[#945](https://github.com/hydro-project/hydroflow/issues/945)**
    - Add defer() which is the same as defer_tick() except that it is lazy ([`7df0a0d`](https://github.com/hydro-project/hydroflow/commit/7df0a0df61597764eed763b68138929fed1413ac))
 * **[#948](https://github.com/hydro-project/hydroflow/issues/948)**
    - Fold takes initial value by closure rather than by value ([`3136e0f`](https://github.com/hydro-project/hydroflow/commit/3136e0f286f87e944e7f718d926fd7670b44194b))
 * **[#950](https://github.com/hydro-project/hydroflow/issues/950)**
    - Avoid panic-ing on degen `null()` ([`35b1e9e`](https://github.com/hydro-project/hydroflow/commit/35b1e9e83f2a0cfa171b4994a2cffb0d22706abf))
 * **[#959](https://github.com/hydro-project/hydroflow/issues/959)**
    - `multiset_delta` incorrect `is_first_run_this_tick` check, fixes #958 ([`43280cb`](https://github.com/hydro-project/hydroflow/commit/43280cb698cf6bc070483365ee272106c271dca4))
 * **[#960](https://github.com/hydro-project/hydroflow/issues/960)**
    - Fix clippy lints on latest nightly ([`ba6afab`](https://github.com/hydro-project/hydroflow/commit/ba6afab8416ad66eee4fdb9d0c73e62d45752617))
 * **[#967](https://github.com/hydro-project/hydroflow/issues/967)**
    - Chat and two_pc no longer replay ([`0539e2a`](https://github.com/hydro-project/hydroflow/commit/0539e2a91eb3ba71ed1c9fbe8d0c74b6344ad1bf))
 * **[#971](https://github.com/hydro-project/hydroflow/issues/971)**
    - Add 100ms wait to chat example to avoid dropped packet ([`bc35a5a`](https://github.com/hydro-project/hydroflow/commit/bc35a5a5e05ccc3990bb3c430129f0a735bc8c0a))
 * **[#974](https://github.com/hydro-project/hydroflow/issues/974)**
    - Improvements prepping for release ([`7e65a08`](https://github.com/hydro-project/hydroflow/commit/7e65a08711775656e435e854777c5f089dd31a05))
 * **[#978](https://github.com/hydro-project/hydroflow/issues/978)**
    - Add initial test using Hydro CLI from Hydroflow+ ([`e5bdd12`](https://github.com/hydro-project/hydroflow/commit/e5bdd12e32d6ea72fd91a55c12e09f07a0edaa5c))
 * **[#979](https://github.com/hydro-project/hydroflow/issues/979)**
    - Remove intrinsics from KVS bench, fixes warning ([`7e1eae6`](https://github.com/hydro-project/hydroflow/commit/7e1eae6b708ef89c833cd2d2c2e5112c2bd84395))
 * **[#984](https://github.com/hydro-project/hydroflow/issues/984)**
    - Support building graphs for symmetric clusters in Hydroflow+ ([`174607d`](https://github.com/hydro-project/hydroflow/commit/174607d12277d7544d0f42890c9a5da2ff184df4))
 * **[#986](https://github.com/hydro-project/hydroflow/issues/986)**
    - Split Rust core from Python bindings ([`c50ca12`](https://github.com/hydro-project/hydroflow/commit/c50ca121b6d5e30dc07843f82caa135b68626301))
 * **[#996](https://github.com/hydro-project/hydroflow/issues/996)**
    - Pass subgraph ID through deploy metadata ([`46d87fa`](https://github.com/hydro-project/hydroflow/commit/46d87fa364d3fe01422cf3c404fbc8a1d5e9fb88))
 * **Uncategorized**
    - Release hydroflow_cli_integration v0.5.1, hydroflow_lang v0.5.1, hydroflow_datalog_core v0.5.1, hydroflow_datalog v0.5.1, hydroflow_macro v0.5.1, lattices v0.5.1, variadics v0.0.3, pusherator v0.0.4, hydroflow v0.5.1, stageleft_macro v0.1.0, stageleft v0.1.0, hydroflow_plus v0.5.1, hydro_deploy v0.5.1, hydro_cli v0.5.1 ([`478aebc`](https://github.com/hydro-project/hydroflow/commit/478aebc8fee2aa78eab86bd386322db1c70bde6a))
    - Manually set lockstep-versioned crates (and `lattices`) to version `0.5.1` ([`1b555e5`](https://github.com/hydro-project/hydroflow/commit/1b555e57c8c812bed4d6495d2960cbf77fb0b3ef))
    - Add left,right,outer join module examples ([`f327b02`](https://github.com/hydro-project/hydroflow/commit/f327b02c8001129e619fb253ab9b6d550e229a48))
</details>

## 0.5.0 (2023-10-11)

<csr-id-cc15bab782830ac2af23b3ad3139986c8e186303/>
<csr-id-594375803750056ac03b27e160a67bbd4ed9b71a/>
<csr-id-58bf7ba59cf806ac3cc89af11af91fa38d1c1f95/>
<csr-id-e788989737fbd501173bc99c6f9f5f5ba514ec9c/>
<csr-id-7c7eea7fddda7ea9526c5af4191520e821c979dc/>
<csr-id-e519fb1518615463eab08935a42a10be2fe180fc/>
<csr-id-cb90ae184151ab9085ecb6d58f11d668619af9df/>
<csr-id-db9f270e2bdaff8cb4429c13c15ad9ca7bbff61b/>
<csr-id-b0c43c75954741edffe7e4e12909697e75ab1d26/>
<csr-id-3c8dbd709fa5a1c1a458d2aa5e2be9ecd8fc4b49/>
<csr-id-1126266e69c2c4364bc8de558f11859e5bad1c69/>
<csr-id-96e2f4c08fdf3e76cd7648954c6a35c3ea3b6dc4/>
<csr-id-95ed7a5298f1003dabfe97e8b2da8eed2915fbb8/>

### Chore

 - <csr-id-cc15bab782830ac2af23b3ad3139986c8e186303/> update snapshots for previous commit
 - <csr-id-594375803750056ac03b27e160a67bbd4ed9b71a/> cleanup cli quotes, loose TODO comment
 - <csr-id-58bf7ba59cf806ac3cc89af11af91fa38d1c1f95/> Fix string join `clippy::format_collect` lint
 - <csr-id-e788989737fbd501173bc99c6f9f5f5ba514ec9c/> Fix `clippy::implied_bounds_in_impls` from latest nightlies
 - <csr-id-7c7eea7fddda7ea9526c5af4191520e821c979dc/> Replace `or_insert_with(Vec::new)` with `or_default()`
   Clippy lint `unwrap_or_default` complaining on latest nightly

### New Features

<csr-id-9646ca06e61af8c827e2d2fb9826ce62b70b6799/>
<csr-id-b3d114827256f2b82a3c357f3419c6853a97f5c0/>
<csr-id-fc2543359ba11c0947fdc26f5360b2ac43a5a0c4/>
<csr-id-d254e2deb883f9633f8b325a595fb7c61bad42d7/>
<csr-id-1ce5f01cde288930cb1281468966dfb66d2e3e53/>
<csr-id-f013c3ca15f2cc9413fcfb92898f71d5fc00073a/>
<csr-id-d03ffe70c050f35ff6c760dcdfe13fbf48345b69/>
<csr-id-1bdbf73b630e4f2eff009b00b0e66d71be53bb4a/>
<csr-id-63c435c32d170dcb6f1ee2a8da74b528d68e8e50/>
<csr-id-9baf80ccc38c4e41c8a1a2ae048036cec2b723c6/>
<csr-id-fd89cb46c5983d277e16bb7b19f7d3ca83dd60cc/>
<csr-id-38346cf01aec0afa2b491095043aa31587613e24/>
<csr-id-9ab7cf8199ddfa8a6a83b7e5f5bc5e6dc05a3110/>
<csr-id-7714403e130969b96c8f405444d4daf451450fdf/>
<csr-id-fd5cdb583cb5b63dca790825d70836ea547d3d81/>

 - <csr-id-d38ec080ba195acf52997d4a0f7296e43270ad8b/> add kvs with replication example
   have both kvs_replicated and kvs, separate examples
   
   add `flow_props_fn`s to `cross_join` (same as `join`), and `filter`
 - <csr-id-21140f09156e1dad195162854955522f138ae781/> update snapshot tests for previous two commits
 - <csr-id-e7ea6d804ae162c0d7ecbd6e4cbc1084766ce506/> open mermaid/dot graph in browser
   * `HydroflowGraph::open_mermaid()` opens https://mermaid.live/
* `HydroflowGraph::open_dot()` opens https://dreampuf.github.io/GraphvizOnline/

### Bug Fixes

 - <csr-id-2edf77961ca0218265b35f179c2d86c810795266/> restore in-subgraph rendering of self-handoffs
 - <csr-id-3ac9b1634223b50deeebb5535205427b5a9aa201/> fix example `kvs`/`kvs_replicated` tests, improve comments
 - <csr-id-a6708bbc72722f2dc1d5e0e6508583d726013409/> surface python snapshot tests
 - <csr-id-a927dc6afbe3178815b7c7c58ed2838d42d80334/> clippy warning on multiline string in hydro_cli, py_udf
 - <csr-id-5a7e1b157362b0d655a28d6f3e5cd139ab8799f3/> fix demux error messages and add tests
 - <csr-id-159a262ba056ec6ffad5590c4f3e57422022901e/> Clean up degenerate subgraph error message for consistency
   Makes the pinned and latest nightly version have the same stderr output
   for consistent testing.
 - <csr-id-e8b027b67b35820f5710df2239da029b0f7fdc77/> Make deadlock detector filter out spruious UDP `ConnectionReset`s on Windows
 - <csr-id-d9a13509c9e82837b65c4547503293d8146d1f7e/> Ignore PermissionDenied on already-closed socket on Windows
 - <csr-id-3f45ec10f0bcc5484f3e116369cdf66ec53b506f/> add context.current_tick_start() without breaking the scheduler this time
 - <csr-id-9918c781c4186be9472c3978e6468ba5d67497a5/> add context.current_tick_start()
 - <csr-id-5ac9ddebedf615f87684d1092382ba64826c1c1c/> separate internal compiler operators in docs name/category/sort order

### Refactor

 - <csr-id-e519fb1518615463eab08935a42a10be2fe180fc/> kvs_bench graphwrite
 - <csr-id-cb90ae184151ab9085ecb6d58f11d668619af9df/> cleanup kvs example more
   Add `persist` `flow_prop_fn`
 - <csr-id-db9f270e2bdaff8cb4429c13c15ad9ca7bbff61b/> cleanup kvs example with lattice properties
   Open graphs when `--graph` is specified
 - <csr-id-b0c43c75954741edffe7e4e12909697e75ab1d26/> refactor `kvs` example, cleanup, use `demux_enum`
 - <csr-id-3c8dbd709fa5a1c1a458d2aa5e2be9ecd8fc4b49/> update `chat` example to use `demux_enum`
 - <csr-id-1126266e69c2c4364bc8de558f11859e5bad1c69/> `demux_enum` requires enum type name, add better error handling

### Test

 - <csr-id-96e2f4c08fdf3e76cd7648954c6a35c3ea3b6dc4/> record snapshots for `surface_scheduling.rs`
 - <csr-id-95ed7a5298f1003dabfe97e8b2da8eed2915fbb8/> Add flow prop tests

### New Features (BREAKING)

 - <csr-id-9ed0ce02128a0eeaf0b603efcbe896427e47ef62/> Simplify graph printing code, add delta/cumul green edges, allow hiding of vars/subgraphs

### `hydroflow` Commit Statistics

<csr-read-only-do-not-edit/>

 - 46 commits contributed to the release.
 - 56 days passed between releases.
 - 43 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 19 unique issues were worked on: [#882](https://github.com/hydro-project/hydroflow/issues/882), [#884](https://github.com/hydro-project/hydroflow/issues/884), [#885](https://github.com/hydro-project/hydroflow/issues/885), [#886](https://github.com/hydro-project/hydroflow/issues/886), [#887](https://github.com/hydro-project/hydroflow/issues/887), [#892](https://github.com/hydro-project/hydroflow/issues/892), [#893](https://github.com/hydro-project/hydroflow/issues/893), [#896](https://github.com/hydro-project/hydroflow/issues/896), [#897](https://github.com/hydro-project/hydroflow/issues/897), [#898](https://github.com/hydro-project/hydroflow/issues/898), [#902](https://github.com/hydro-project/hydroflow/issues/902), [#906](https://github.com/hydro-project/hydroflow/issues/906), [#918](https://github.com/hydro-project/hydroflow/issues/918), [#919](https://github.com/hydro-project/hydroflow/issues/919), [#923](https://github.com/hydro-project/hydroflow/issues/923), [#924](https://github.com/hydro-project/hydroflow/issues/924), [#926](https://github.com/hydro-project/hydroflow/issues/926), [#932](https://github.com/hydro-project/hydroflow/issues/932), [#935](https://github.com/hydro-project/hydroflow/issues/935)

### `hydroflow` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#882](https://github.com/hydro-project/hydroflow/issues/882)**
    - Add `Cumul` `flow_prop_fn`s for `lattice_fold()` and `lattice_reduce()` ([`63c435c`](https://github.com/hydro-project/hydroflow/commit/63c435c32d170dcb6f1ee2a8da74b528d68e8e50))
    - Update dot/graphviz rendering of delta/cumul and `defer_tick` edges ([`9baf80c`](https://github.com/hydro-project/hydroflow/commit/9baf80ccc38c4e41c8a1a2ae048036cec2b723c6))
    - Record snapshots for `surface_scheduling.rs` ([`96e2f4c`](https://github.com/hydro-project/hydroflow/commit/96e2f4c08fdf3e76cd7648954c6a35c3ea3b6dc4))
    - Add flow prop tests ([`95ed7a5`](https://github.com/hydro-project/hydroflow/commit/95ed7a5298f1003dabfe97e8b2da8eed2915fbb8))
    - Make `propegate_flow_props` fallible, cleanup `flow_prop_fn` definition. ([`fd89cb4`](https://github.com/hydro-project/hydroflow/commit/fd89cb46c5983d277e16bb7b19f7d3ca83dd60cc))
    - Add `cast` operator ([`38346cf`](https://github.com/hydro-project/hydroflow/commit/38346cf01aec0afa2b491095043aa31587613e24))
    - Update mermaid rendering of hydroflow graph to show flow properties ([`9ab7cf8`](https://github.com/hydro-project/hydroflow/commit/9ab7cf8199ddfa8a6a83b7e5f5bc5e6dc05a3110))
    - Add `monotonic_fn` and `morphism` macros, update snapshots for flow props. ([`7714403`](https://github.com/hydro-project/hydroflow/commit/7714403e130969b96c8f405444d4daf451450fdf))
    - Add `source_iter_delta` op for testing, basic flow props test, cleanups. ([`fd5cdb5`](https://github.com/hydro-project/hydroflow/commit/fd5cdb583cb5b63dca790825d70836ea547d3d81))
 * **[#884](https://github.com/hydro-project/hydroflow/issues/884)**
    - Separate internal compiler operators in docs name/category/sort order ([`5ac9dde`](https://github.com/hydro-project/hydroflow/commit/5ac9ddebedf615f87684d1092382ba64826c1c1c))
 * **[#885](https://github.com/hydro-project/hydroflow/issues/885)**
    - Revert "fix: add context.current_tick_start()" ([`6844366`](https://github.com/hydro-project/hydroflow/commit/684436621d16afb5ba45e5345f1d0ce57be0cfaf))
    - Add context.current_tick_start() ([`9918c78`](https://github.com/hydro-project/hydroflow/commit/9918c781c4186be9472c3978e6468ba5d67497a5))
 * **[#886](https://github.com/hydro-project/hydroflow/issues/886)**
    - Revert "fix: add context.current_tick_start()" ([`6844366`](https://github.com/hydro-project/hydroflow/commit/684436621d16afb5ba45e5345f1d0ce57be0cfaf))
 * **[#887](https://github.com/hydro-project/hydroflow/issues/887)**
    - Add context.current_tick_start() without breaking the scheduler this time ([`3f45ec1`](https://github.com/hydro-project/hydroflow/commit/3f45ec10f0bcc5484f3e116369cdf66ec53b506f))
 * **[#892](https://github.com/hydro-project/hydroflow/issues/892)**
    - Clean up degenerate subgraph error message for consistency ([`159a262`](https://github.com/hydro-project/hydroflow/commit/159a262ba056ec6ffad5590c4f3e57422022901e))
 * **[#893](https://github.com/hydro-project/hydroflow/issues/893)**
    - Replace `or_insert_with(Vec::new)` with `or_default()` ([`7c7eea7`](https://github.com/hydro-project/hydroflow/commit/7c7eea7fddda7ea9526c5af4191520e821c979dc))
 * **[#896](https://github.com/hydro-project/hydroflow/issues/896)**
    - Fix string join `clippy::format_collect` lint ([`58bf7ba`](https://github.com/hydro-project/hydroflow/commit/58bf7ba59cf806ac3cc89af11af91fa38d1c1f95))
 * **[#897](https://github.com/hydro-project/hydroflow/issues/897)**
    - Move `hydroflow_expect_warnings` into library, use in `tests/surface_flow_props.rs` ([`d03ffe7`](https://github.com/hydro-project/hydroflow/commit/d03ffe70c050f35ff6c760dcdfe13fbf48345b69))
 * **[#898](https://github.com/hydro-project/hydroflow/issues/898)**
    - Add import!() expression ([`f013c3c`](https://github.com/hydro-project/hydroflow/commit/f013c3ca15f2cc9413fcfb92898f71d5fc00073a))
 * **[#902](https://github.com/hydro-project/hydroflow/issues/902)**
    - Make lattice_fold and lattice_reduce consistent with fold/reduce ([`1ce5f01`](https://github.com/hydro-project/hydroflow/commit/1ce5f01cde288930cb1281468966dfb66d2e3e53))
 * **[#906](https://github.com/hydro-project/hydroflow/issues/906)**
    - Add context.is_first_time_subgraph_is_scheduled to simplify replaying operators ([`d254e2d`](https://github.com/hydro-project/hydroflow/commit/d254e2deb883f9633f8b325a595fb7c61bad42d7))
 * **[#918](https://github.com/hydro-project/hydroflow/issues/918)**
    - Refactor `kvs` example, cleanup, use `demux_enum` ([`b0c43c7`](https://github.com/hydro-project/hydroflow/commit/b0c43c75954741edffe7e4e12909697e75ab1d26))
    - Update `chat` example to use `demux_enum` ([`3c8dbd7`](https://github.com/hydro-project/hydroflow/commit/3c8dbd709fa5a1c1a458d2aa5e2be9ecd8fc4b49))
 * **[#919](https://github.com/hydro-project/hydroflow/issues/919)**
    - Surface python snapshot tests ([`a6708bb`](https://github.com/hydro-project/hydroflow/commit/a6708bbc72722f2dc1d5e0e6508583d726013409))
 * **[#923](https://github.com/hydro-project/hydroflow/issues/923)**
    - Open mermaid/dot graph in browser ([`e7ea6d8`](https://github.com/hydro-project/hydroflow/commit/e7ea6d804ae162c0d7ecbd6e4cbc1084766ce506))
 * **[#924](https://github.com/hydro-project/hydroflow/issues/924)**
    - Cleanup kvs example with lattice properties ([`db9f270`](https://github.com/hydro-project/hydroflow/commit/db9f270e2bdaff8cb4429c13c15ad9ca7bbff61b))
    - Update snapshot tests for previous two commits ([`21140f0`](https://github.com/hydro-project/hydroflow/commit/21140f09156e1dad195162854955522f138ae781))
 * **[#926](https://github.com/hydro-project/hydroflow/issues/926)**
    - Cleanup cli quotes, loose TODO comment ([`5943758`](https://github.com/hydro-project/hydroflow/commit/594375803750056ac03b27e160a67bbd4ed9b71a))
    - Add kvs with replication example ([`d38ec08`](https://github.com/hydro-project/hydroflow/commit/d38ec080ba195acf52997d4a0f7296e43270ad8b))
    - Cleanup kvs example more ([`cb90ae1`](https://github.com/hydro-project/hydroflow/commit/cb90ae184151ab9085ecb6d58f11d668619af9df))
 * **[#932](https://github.com/hydro-project/hydroflow/issues/932)**
    - Fix example `kvs`/`kvs_replicated` tests, improve comments ([`3ac9b16`](https://github.com/hydro-project/hydroflow/commit/3ac9b1634223b50deeebb5535205427b5a9aa201))
    - Update snapshots for previous commit ([`cc15bab`](https://github.com/hydro-project/hydroflow/commit/cc15bab782830ac2af23b3ad3139986c8e186303))
    - Simplify graph printing code, add delta/cumul green edges, allow hiding of vars/subgraphs ([`9ed0ce0`](https://github.com/hydro-project/hydroflow/commit/9ed0ce02128a0eeaf0b603efcbe896427e47ef62))
 * **[#935](https://github.com/hydro-project/hydroflow/issues/935)**
    - Restore in-subgraph rendering of self-handoffs ([`2edf779`](https://github.com/hydro-project/hydroflow/commit/2edf77961ca0218265b35f179c2d86c810795266))
    - Kvs_bench graphwrite ([`e519fb1`](https://github.com/hydro-project/hydroflow/commit/e519fb1518615463eab08935a42a10be2fe180fc))
 * **Uncategorized**
    - Release hydroflow_macro v0.5.0, lattices v0.5.0, hydroflow v0.5.0, hydro_cli v0.5.0 ([`12697c2`](https://github.com/hydro-project/hydroflow/commit/12697c2f19bd96802591fa63a5b6b12104ecfe0d))
    - Release hydroflow_lang v0.5.0, hydroflow_datalog_core v0.5.0, hydroflow_datalog v0.5.0, hydroflow_macro v0.5.0, lattices v0.5.0, hydroflow v0.5.0, hydro_cli v0.5.0, safety bump 4 crates ([`2e2d8b3`](https://github.com/hydro-project/hydroflow/commit/2e2d8b386fb086c8276a2853d2a1f96ad4d7c221))
    - Clippy warning on multiline string in hydro_cli, py_udf ([`a927dc6`](https://github.com/hydro-project/hydroflow/commit/a927dc6afbe3178815b7c7c58ed2838d42d80334))
    - Update documentation and improve error messages for `demux_enum` operator ([`9646ca0`](https://github.com/hydro-project/hydroflow/commit/9646ca06e61af8c827e2d2fb9826ce62b70b6799))
    - `demux_enum` requires enum type name, add better error handling ([`1126266`](https://github.com/hydro-project/hydroflow/commit/1126266e69c2c4364bc8de558f11859e5bad1c69))
    - Initial technically working version of `demux_enum` with very bad error messages ([`b3d1148`](https://github.com/hydro-project/hydroflow/commit/b3d114827256f2b82a3c357f3419c6853a97f5c0))
    - Implement `partition` operator ([`fc25433`](https://github.com/hydro-project/hydroflow/commit/fc2543359ba11c0947fdc26f5360b2ac43a5a0c4))
    - Fix demux error messages and add tests ([`5a7e1b1`](https://github.com/hydro-project/hydroflow/commit/5a7e1b157362b0d655a28d6f3e5cd139ab8799f3))
    - Implement `flow_prop_fn` for `union()` ([`1bdbf73`](https://github.com/hydro-project/hydroflow/commit/1bdbf73b630e4f2eff009b00b0e66d71be53bb4a))
    - Fix `clippy::implied_bounds_in_impls` from latest nightlies ([`e788989`](https://github.com/hydro-project/hydroflow/commit/e788989737fbd501173bc99c6f9f5f5ba514ec9c))
    - Make deadlock detector filter out spruious UDP `ConnectionReset`s on Windows ([`e8b027b`](https://github.com/hydro-project/hydroflow/commit/e8b027b67b35820f5710df2239da029b0f7fdc77))
    - Ignore PermissionDenied on already-closed socket on Windows ([`d9a1350`](https://github.com/hydro-project/hydroflow/commit/d9a13509c9e82837b65c4547503293d8146d1f7e))
</details>

## 0.4.0 (2023-08-15)

<csr-id-949db02e9fa9878e1a7176c180d6f44c5cddf052/>
<csr-id-f60053f70da3071c54de4a0eabb059a143aa2ccc/>
<csr-id-aa52b10b60e733a65bdd1fc0234acf7249c22f1a/>
<csr-id-d38b1bce848b4672109a4f411aaa81b9d210c2c4/>
<csr-id-060715a7549c8477362971e39d39a3da7e26ad29/>

### Chore

 - <csr-id-949db02e9fa9878e1a7176c180d6f44c5cddf052/> fix lints for latest nightly
 - <csr-id-f60053f70da3071c54de4a0eabb059a143aa2ccc/> fix lint, format errors for latest nightly version (without updated pinned)
   For nightly version (d9c13cd45 2023-07-05)

### New Features

<csr-id-8f306e2a36582e168417808099eedf8a9de3b419/>
<csr-id-871002267e3c03da83729ecc2d028f3c7b5c18d2/>

 - <csr-id-a7ab8ff09bc34ebd6fb4127460733056aee2adf7/> add kvs_bench example test
 - <csr-id-b4b9644a19e8e7e7725c9c5b88e3a6b8c2be7364/> Add `use` statements to hydroflow syntax
   And use in doc tests.
 - <csr-id-fe02f23649312bb64c5d0c8870edf578e516f397/> add `iter_batches_stream` util to break up iterator into per-tick batches
   * Also tightens up a bit of `assert_eq`'s code

### Bug Fixes

<csr-id-cc959c762c3a0e036e672801c615028cbfb95168/>
<csr-id-0e715ae2648b8b9ce76a60a69c18e0543cf2c033/>
<csr-id-ebba38230df134b04dd38c1df7c6de8712e3122e/>
<csr-id-a55fc74dc1ebbe26b49359a104beb48d7f6cd449/>
<csr-id-fb517b984735309eaa9a1008b23375555438529d/>
<csr-id-6c98bbc2bd3443fe6f77e0b8689b461edde1b316/>
<csr-id-c6cb86d4549370b9a10c6f49ecbf0f160b481a1d/>
<csr-id-2d53110336b2da5a16887c3d72101da72b2362bb/>

 - <csr-id-c9e7c7dc1c0885640b1c5d6bc15194a256eff833/> fix shopping cart example test
 - <csr-id-d6472b90c2caf26b98c0bd753616e675b7de9769/> change "gated buffer" discussion from `cross_join` to `defer_signal`
   shopping still erroring out for reasons unrelated to this PR
 - <csr-id-d378e5eada3d2bae90f98c5a33b2d055940a8c7f/> unify antijoin and difference with set and multiset semantics
   * fix: unify antijoin and difference with set and multiset semantics
* fix: replay semantics for antijoin and difference now work
   also added cross_join_multiset
* fix: enforce sort for tests of anti_join and difference using assert_eq
* fix: advance __borrow_ident beyond the current tick to prevent replay loops
* fix: add modified snapshots
* fix: temp
* fix: spelling typo in comment
* fix: make anti_join replay more efficient
* fix: ignore test that depends on order of antijoin
* fix: really ignore test_index
* fix: fix specific test ordering in wasm

### Other

 - <csr-id-aa52b10b60e733a65bdd1fc0234acf7249c22f1a/> join probe returns first item directly
   * optimization: join probe returns first item directly
   
   * update comments

### Test

 - <csr-id-d38b1bce848b4672109a4f411aaa81b9d210c2c4/> add `use` compile-fail tests
 - <csr-id-060715a7549c8477362971e39d39a3da7e26ad29/> Change example tests to use localhost IP

### New Features (BREAKING)

 - <csr-id-7a3b4c04779ea38bfa06c246882fa8dfb52bc8f1/> add fused joins, make lattice_join replay correctly
   * feat!: add fused joins, make lattice_join replay correctly
* address comments
* fix clippy

### `hydroflow` Commit Statistics

<csr-read-only-do-not-edit/>

 - 25 commits contributed to the release.
 - 42 days passed between releases.
 - 22 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 23 unique issues were worked on: [#820](https://github.com/hydro-project/hydroflow/issues/820), [#821](https://github.com/hydro-project/hydroflow/issues/821), [#822](https://github.com/hydro-project/hydroflow/issues/822), [#823](https://github.com/hydro-project/hydroflow/issues/823), [#833](https://github.com/hydro-project/hydroflow/issues/833), [#835](https://github.com/hydro-project/hydroflow/issues/835), [#837](https://github.com/hydro-project/hydroflow/issues/837), [#840](https://github.com/hydro-project/hydroflow/issues/840), [#842](https://github.com/hydro-project/hydroflow/issues/842), [#843](https://github.com/hydro-project/hydroflow/issues/843), [#844](https://github.com/hydro-project/hydroflow/issues/844), [#845](https://github.com/hydro-project/hydroflow/issues/845), [#846](https://github.com/hydro-project/hydroflow/issues/846), [#848](https://github.com/hydro-project/hydroflow/issues/848), [#851](https://github.com/hydro-project/hydroflow/issues/851), [#853](https://github.com/hydro-project/hydroflow/issues/853), [#857](https://github.com/hydro-project/hydroflow/issues/857), [#861](https://github.com/hydro-project/hydroflow/issues/861), [#870](https://github.com/hydro-project/hydroflow/issues/870), [#872](https://github.com/hydro-project/hydroflow/issues/872), [#874](https://github.com/hydro-project/hydroflow/issues/874), [#878](https://github.com/hydro-project/hydroflow/issues/878), [#880](https://github.com/hydro-project/hydroflow/issues/880)

### `hydroflow` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#820](https://github.com/hydro-project/hydroflow/issues/820)**
    - Make batch take two inputs [input] and [signal] ([`8710022`](https://github.com/hydro-project/hydroflow/commit/871002267e3c03da83729ecc2d028f3c7b5c18d2))
 * **[#821](https://github.com/hydro-project/hydroflow/issues/821)**
    - `py_udf` operator feature gating ([`2d53110`](https://github.com/hydro-project/hydroflow/commit/2d53110336b2da5a16887c3d72101da72b2362bb))
 * **[#822](https://github.com/hydro-project/hydroflow/issues/822)**
    - Fix lint, format errors for latest nightly version (without updated pinned) ([`f60053f`](https://github.com/hydro-project/hydroflow/commit/f60053f70da3071c54de4a0eabb059a143aa2ccc))
 * **[#823](https://github.com/hydro-project/hydroflow/issues/823)**
    - Book/doc edits ([`4bdd556`](https://github.com/hydro-project/hydroflow/commit/4bdd5568fa0a6674f650f91a029fab302cbf14f4))
 * **[#833](https://github.com/hydro-project/hydroflow/issues/833)**
    - Rename next_tick -> defer, batch -> defer_signal ([`6c98bbc`](https://github.com/hydro-project/hydroflow/commit/6c98bbc2bd3443fe6f77e0b8689b461edde1b316))
 * **[#835](https://github.com/hydro-project/hydroflow/issues/835)**
    - Rename assert => assert_eq, add assert, change underlying implementation to work across ticks ([`8f306e2`](https://github.com/hydro-project/hydroflow/commit/8f306e2a36582e168417808099eedf8a9de3b419))
 * **[#837](https://github.com/hydro-project/hydroflow/issues/837)**
    - Remove python from default features, add it to ci ([`c6cb86d`](https://github.com/hydro-project/hydroflow/commit/c6cb86d4549370b9a10c6f49ecbf0f160b481a1d))
 * **[#840](https://github.com/hydro-project/hydroflow/issues/840)**
    - Make all operators 'tick by default ([`a55fc74`](https://github.com/hydro-project/hydroflow/commit/a55fc74dc1ebbe26b49359a104beb48d7f6cd449))
 * **[#842](https://github.com/hydro-project/hydroflow/issues/842)**
    - Stop two_pc example test from hanging sometimes ([`fb517b9`](https://github.com/hydro-project/hydroflow/commit/fb517b984735309eaa9a1008b23375555438529d))
 * **[#843](https://github.com/hydro-project/hydroflow/issues/843)**
    - Add `iter_batches_stream` util to break up iterator into per-tick batches ([`fe02f23`](https://github.com/hydro-project/hydroflow/commit/fe02f23649312bb64c5d0c8870edf578e516f397))
 * **[#844](https://github.com/hydro-project/hydroflow/issues/844)**
    - Fix lints for latest nightly ([`949db02`](https://github.com/hydro-project/hydroflow/commit/949db02e9fa9878e1a7176c180d6f44c5cddf052))
 * **[#845](https://github.com/hydro-project/hydroflow/issues/845)**
    - Add `use` compile-fail tests ([`d38b1bc`](https://github.com/hydro-project/hydroflow/commit/d38b1bce848b4672109a4f411aaa81b9d210c2c4))
    - Add `use` statements to hydroflow syntax ([`b4b9644`](https://github.com/hydro-project/hydroflow/commit/b4b9644a19e8e7e7725c9c5b88e3a6b8c2be7364))
 * **[#846](https://github.com/hydro-project/hydroflow/issues/846)**
    - Change example tests to use localhost IP ([`060715a`](https://github.com/hydro-project/hydroflow/commit/060715a7549c8477362971e39d39a3da7e26ad29))
 * **[#848](https://github.com/hydro-project/hydroflow/issues/848)**
    - Add kvs_bench example test ([`a7ab8ff`](https://github.com/hydro-project/hydroflow/commit/a7ab8ff09bc34ebd6fb4127460733056aee2adf7))
 * **[#851](https://github.com/hydro-project/hydroflow/issues/851)**
    - Lattice_batch now takes [input] and [signal] ([`ebba382`](https://github.com/hydro-project/hydroflow/commit/ebba38230df134b04dd38c1df7c6de8712e3122e))
 * **[#853](https://github.com/hydro-project/hydroflow/issues/853)**
    - Book updates ([`2e57445`](https://github.com/hydro-project/hydroflow/commit/2e574457246ac5bd231745a8ad068558859698ef))
 * **[#857](https://github.com/hydro-project/hydroflow/issues/857)**
    - Livelock in deadlock detector #810 ([`0e715ae`](https://github.com/hydro-project/hydroflow/commit/0e715ae2648b8b9ce76a60a69c18e0543cf2c033))
 * **[#861](https://github.com/hydro-project/hydroflow/issues/861)**
    - Add fused joins, make lattice_join replay correctly ([`7a3b4c0`](https://github.com/hydro-project/hydroflow/commit/7a3b4c04779ea38bfa06c246882fa8dfb52bc8f1))
 * **[#870](https://github.com/hydro-project/hydroflow/issues/870)**
    - Joins now replay correctly ([`cc959c7`](https://github.com/hydro-project/hydroflow/commit/cc959c762c3a0e036e672801c615028cbfb95168))
 * **[#872](https://github.com/hydro-project/hydroflow/issues/872)**
    - Unify antijoin and difference with set and multiset semantics ([`d378e5e`](https://github.com/hydro-project/hydroflow/commit/d378e5eada3d2bae90f98c5a33b2d055940a8c7f))
 * **[#874](https://github.com/hydro-project/hydroflow/issues/874)**
    - Join probe returns first item directly ([`aa52b10`](https://github.com/hydro-project/hydroflow/commit/aa52b10b60e733a65bdd1fc0234acf7249c22f1a))
 * **[#878](https://github.com/hydro-project/hydroflow/issues/878)**
    - Change "gated buffer" discussion from `cross_join` to `defer_signal` ([`d6472b9`](https://github.com/hydro-project/hydroflow/commit/d6472b90c2caf26b98c0bd753616e675b7de9769))
 * **[#880](https://github.com/hydro-project/hydroflow/issues/880)**
    - Fix shopping cart example test ([`c9e7c7d`](https://github.com/hydro-project/hydroflow/commit/c9e7c7dc1c0885640b1c5d6bc15194a256eff833))
 * **Uncategorized**
    - Release hydroflow_lang v0.4.0, hydroflow_datalog_core v0.4.0, hydroflow_datalog v0.4.0, hydroflow_macro v0.4.0, lattices v0.4.0, pusherator v0.0.3, hydroflow v0.4.0, hydro_cli v0.4.0, safety bump 4 crates ([`cb313f0`](https://github.com/hydro-project/hydroflow/commit/cb313f0635214460a8308d05cbef4bf7f4bfaa15))
</details>

## 0.3.0 (2023-07-04)

<csr-id-70c88a51c4c83a4dc2fc67a0cd344786a4ff26f7/>
<csr-id-5c654f2add8ef389eefeddccc063fd26a08b5be8/>
<csr-id-920b2dfb88243c1d4833dd8fb0b80ea626380df5/>
<csr-id-4675c2c334b6bb1550124a27614728fe29c53e12/>
<csr-id-c99242378caf06810fa7de94e504e36af8aeaaf4/>
<csr-id-aabaa27fd736534a14f5414fb31328fad25984f3/>
<csr-id-4a727ecf1232e0f03f5300547282bfbe73342cfa/>
<csr-id-5c7e4d3aea1dfb61d51bcb0291740281824e3090/>
<csr-id-1bdadb82b25941d11f3fa24eaac35109927c852f/>

### Documentation

 - <csr-id-fa5b180d96498d144f3617bba7722e8f4ac9dd0e/> remove pattern deref from inspect, filter examples
   `*` derefs are easier for Rust beginners to comprehend.
 - <csr-id-23f27e590df648ee8f6bd9ae452f2b2bec5ac652/> import doc examples from runnable code with tested output
 - <csr-id-f55d540532ba0a0970cab2bb5aef81b6a76b317a/> change mermaid colors
   Use a lighter shade of blue and yellow, and dark text.

### New Features

<csr-id-6323980e83bee27a8233a69a35734b5970336701/>
<csr-id-010524615bb78288e339e03880c4dd3b432b6d7f/>
<csr-id-8f67c264f5aed560fc14af70b062edf7d839afe6/>
<csr-id-d83b049e4d643617a2b15b3dbf1698aa79846aeb/>
<csr-id-ea65349d241873f8460d7a8b024d64c63180246f/>
<csr-id-22abcaff806c7de6e4a7725656bbcf201e7d9259/>
<csr-id-a23381854a45f9c5791bd399dd633fee291d400a/>
<csr-id-baf320e7d31e3189adc85a98ff3824a321a60995/>

 - <csr-id-b435bbb1d64d60f1248fdcd636635b15954e7325/> fold and reduce take accumulated value by mutable reference
   * feat: fold and reduce take accumulated value by mutable reference
* address comments
* feat: add lattice_reduce and lattice_fold
* address comments
* simplify lattice fold a bit
* address comments
* feat: add join_multiset()
* address comments
* fix assert
* feat: add assert() operator
* update: change for_each -> assert, make doctest use run_avaialble()
* don't run tests that panic in wasm
* update comments
* address comments

### Bug Fixes

 - <csr-id-8d3494b5afee858114a602a3e23077bb6d24dd77/> update proc-macro2, use new span location API where possible
   requires latest* rust nightly version
   
   *latest = 2023-06-28 or something
 - <csr-id-e628da5b70543ac4001d8c4f0ef8f663f95bc17d/> SparseVec does not need T: Default
 - <csr-id-a3c1fbbd1e3fa7a7299878f61b4bfd12dce0052c/> remove nightly feature `never_type` where unused
 - <csr-id-9bb5528d99e83fdae5aeca9456802379131c2f90/> removed unused nightly features `impl_trait_in_assoc_type`, `type_alias_impl_trait`
 - <csr-id-0ecabc80348093416ecde3de7b6bf0bb22ff30d6/> fix scheduler spinning on stateful operators across strata
 - <csr-id-7c9632a0316c29df0ee793a15b1a02f651c4ff51/> use proper tcp/udp localhost IPs to fix test on windows

### Style

 - <csr-id-70c88a51c4c83a4dc2fc67a0cd344786a4ff26f7/> `warn` missing docs (instead of `deny`) to allow code before docs
 - <csr-id-5c654f2add8ef389eefeddccc063fd26a08b5be8/> use `is_ok` for clippy latest nightly

### Test

 - <csr-id-920b2dfb88243c1d4833dd8fb0b80ea626380df5/> test examples outputs from docs
 - <csr-id-4675c2c334b6bb1550124a27614728fe29c53e12/> add failing spinning stratum-persist bug tests
 - <csr-id-c99242378caf06810fa7de94e504e36af8aeaaf4/> add `test_stratum/tick_loop` tests
 - <csr-id-aabaa27fd736534a14f5414fb31328fad25984f3/> ignore `surface_lattice_merge_badgeneric` `compile-fail` test due to `Seq` inconsistent messages

### New Features (BREAKING)

<csr-id-c1b028089ea9d76ab71cd9cb4eaaaf16aa4b65a6/>

 - <csr-id-931d93887c238025596cb22226e16d43e16a7425/> Add `reveal` methods, make fields private
 - <csr-id-7aec1ac884e01a560770dfab7e0ba64d520415f6/> Add `Provenance` generic param token to `Point`.
   - Use `()` provenance for `kvs_bench` example.
* Adds `tokio` for `#[tokio::test]`.
* Adds `async_std` for `#[async_std::test]`.
* Adds `hydroflow` for `#[hydroflow::test]`.
* Adds `env_logging` for `env_logger` registering.
* Adds `env_tracing` for `EnvFilter` `FmtSubscriber` `tracing`.

### Bug Fixes (BREAKING)

 - <csr-id-6f3c536fcd4d1305d478ec3db62416aad9cf3c68/> make join default to multiset join

### Refactor (BREAKING)

 - <csr-id-4a727ecf1232e0f03f5300547282bfbe73342cfa/> Rename `ConvertFrom::from` -> `LatticeFrom::lattice_from`
 - <csr-id-5c7e4d3aea1dfb61d51bcb0291740281824e3090/> Rename `Bottom` -> `WithBot`, `Top` -> `WithTop`, constructors now take `Option`s 2/4
 - <csr-id-1bdadb82b25941d11f3fa24eaac35109927c852f/> Rename `Immut` -> `Point` lattice.

### `hydroflow` Commit Statistics

<csr-read-only-do-not-edit/>

 - 34 commits contributed to the release.
 - 33 days passed between releases.
 - 31 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 25 unique issues were worked on: [#739](https://github.com/hydro-project/hydroflow/issues/739), [#743](https://github.com/hydro-project/hydroflow/issues/743), [#745](https://github.com/hydro-project/hydroflow/issues/745), [#748](https://github.com/hydro-project/hydroflow/issues/748), [#749](https://github.com/hydro-project/hydroflow/issues/749), [#755](https://github.com/hydro-project/hydroflow/issues/755), [#761](https://github.com/hydro-project/hydroflow/issues/761), [#763](https://github.com/hydro-project/hydroflow/issues/763), [#765](https://github.com/hydro-project/hydroflow/issues/765), [#772](https://github.com/hydro-project/hydroflow/issues/772), [#773](https://github.com/hydro-project/hydroflow/issues/773), [#774](https://github.com/hydro-project/hydroflow/issues/774), [#775](https://github.com/hydro-project/hydroflow/issues/775), [#778](https://github.com/hydro-project/hydroflow/issues/778), [#780](https://github.com/hydro-project/hydroflow/issues/780), [#784](https://github.com/hydro-project/hydroflow/issues/784), [#788](https://github.com/hydro-project/hydroflow/issues/788), [#789](https://github.com/hydro-project/hydroflow/issues/789), [#791](https://github.com/hydro-project/hydroflow/issues/791), [#792](https://github.com/hydro-project/hydroflow/issues/792), [#799](https://github.com/hydro-project/hydroflow/issues/799), [#801](https://github.com/hydro-project/hydroflow/issues/801), [#803](https://github.com/hydro-project/hydroflow/issues/803), [#804](https://github.com/hydro-project/hydroflow/issues/804), [#809](https://github.com/hydro-project/hydroflow/issues/809)

### `hydroflow` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#739](https://github.com/hydro-project/hydroflow/issues/739)**
    - Add bind_tcp and connect_tcp, analogues of bind_udp ([`baf320e`](https://github.com/hydro-project/hydroflow/commit/baf320e7d31e3189adc85a98ff3824a321a60995))
 * **[#743](https://github.com/hydro-project/hydroflow/issues/743)**
    - Use `is_ok` for clippy latest nightly ([`5c654f2`](https://github.com/hydro-project/hydroflow/commit/5c654f2add8ef389eefeddccc063fd26a08b5be8))
 * **[#745](https://github.com/hydro-project/hydroflow/issues/745)**
    - Use proper tcp/udp localhost IPs to fix test on windows ([`7c9632a`](https://github.com/hydro-project/hydroflow/commit/7c9632a0316c29df0ee793a15b1a02f651c4ff51))
 * **[#748](https://github.com/hydro-project/hydroflow/issues/748)**
    - Add `test_stratum/tick_loop` tests ([`c992423`](https://github.com/hydro-project/hydroflow/commit/c99242378caf06810fa7de94e504e36af8aeaaf4))
 * **[#749](https://github.com/hydro-project/hydroflow/issues/749)**
    - Add basic tracing support, use in (some) tests. ([`a233818`](https://github.com/hydro-project/hydroflow/commit/a23381854a45f9c5791bd399dd633fee291d400a))
 * **[#755](https://github.com/hydro-project/hydroflow/issues/755)**
    - `hydroflow`, `logging`/`tracing` features ([`c1b0280`](https://github.com/hydro-project/hydroflow/commit/c1b028089ea9d76ab71cd9cb4eaaaf16aa4b65a6))
 * **[#761](https://github.com/hydro-project/hydroflow/issues/761)**
    - Rename `Immut` -> `Point` lattice. ([`1bdadb8`](https://github.com/hydro-project/hydroflow/commit/1bdadb82b25941d11f3fa24eaac35109927c852f))
 * **[#763](https://github.com/hydro-project/hydroflow/issues/763)**
    - Rename `Bottom` -> `WithBot`, `Top` -> `WithTop`, constructors now take `Option`s 2/4 ([`5c7e4d3`](https://github.com/hydro-project/hydroflow/commit/5c7e4d3aea1dfb61d51bcb0291740281824e3090))
 * **[#765](https://github.com/hydro-project/hydroflow/issues/765)**
    - Rename `ConvertFrom::from` -> `LatticeFrom::lattice_from` ([`4a727ec`](https://github.com/hydro-project/hydroflow/commit/4a727ecf1232e0f03f5300547282bfbe73342cfa))
 * **[#772](https://github.com/hydro-project/hydroflow/issues/772)**
    - Add `Provenance` generic param token to `Point`. ([`7aec1ac`](https://github.com/hydro-project/hydroflow/commit/7aec1ac884e01a560770dfab7e0ba64d520415f6))
 * **[#773](https://github.com/hydro-project/hydroflow/issues/773)**
    - `warn` missing docs (instead of `deny`) to allow code before docs ([`70c88a5`](https://github.com/hydro-project/hydroflow/commit/70c88a51c4c83a4dc2fc67a0cd344786a4ff26f7))
 * **[#774](https://github.com/hydro-project/hydroflow/issues/774)**
    - Make join default to multiset join ([`6f3c536`](https://github.com/hydro-project/hydroflow/commit/6f3c536fcd4d1305d478ec3db62416aad9cf3c68))
 * **[#775](https://github.com/hydro-project/hydroflow/issues/775)**
    - Add persist_mut and persist_mut_keyed for non-monitone deletions ([`8d8247f`](https://github.com/hydro-project/hydroflow/commit/8d8247f0b37d53415f5738099c0c8a021415b158))
 * **[#778](https://github.com/hydro-project/hydroflow/issues/778)**
    - Import doc examples from runnable code with tested output ([`23f27e5`](https://github.com/hydro-project/hydroflow/commit/23f27e590df648ee8f6bd9ae452f2b2bec5ac652))
    - Test examples outputs from docs ([`920b2df`](https://github.com/hydro-project/hydroflow/commit/920b2dfb88243c1d4833dd8fb0b80ea626380df5))
    - Change mermaid colors ([`f55d540`](https://github.com/hydro-project/hydroflow/commit/f55d540532ba0a0970cab2bb5aef81b6a76b317a))
 * **[#780](https://github.com/hydro-project/hydroflow/issues/780)**
    - Emit `compile_error!` diagnostics for stable ([`ea65349`](https://github.com/hydro-project/hydroflow/commit/ea65349d241873f8460d7a8b024d64c63180246f))
    - Allow stable build, refactors behind `nightly` feature flag ([`22abcaf`](https://github.com/hydro-project/hydroflow/commit/22abcaff806c7de6e4a7725656bbcf201e7d9259))
    - Remove nightly feature `never_type` where unused ([`a3c1fbb`](https://github.com/hydro-project/hydroflow/commit/a3c1fbbd1e3fa7a7299878f61b4bfd12dce0052c))
    - Removed unused nightly features `impl_trait_in_assoc_type`, `type_alias_impl_trait` ([`9bb5528`](https://github.com/hydro-project/hydroflow/commit/9bb5528d99e83fdae5aeca9456802379131c2f90))
 * **[#784](https://github.com/hydro-project/hydroflow/issues/784)**
    - Add assert() operator ([`d83b049`](https://github.com/hydro-project/hydroflow/commit/d83b049e4d643617a2b15b3dbf1698aa79846aeb))
 * **[#788](https://github.com/hydro-project/hydroflow/issues/788)**
    - SparseVec does not need T: Default ([`e628da5`](https://github.com/hydro-project/hydroflow/commit/e628da5b70543ac4001d8c4f0ef8f663f95bc17d))
 * **[#789](https://github.com/hydro-project/hydroflow/issues/789)**
    - Add `reveal` methods, make fields private ([`931d938`](https://github.com/hydro-project/hydroflow/commit/931d93887c238025596cb22226e16d43e16a7425))
 * **[#791](https://github.com/hydro-project/hydroflow/issues/791)**
    - Add tests for examples ([`8f67c26`](https://github.com/hydro-project/hydroflow/commit/8f67c264f5aed560fc14af70b062edf7d839afe6))
 * **[#792](https://github.com/hydro-project/hydroflow/issues/792)**
    - Add `py_udf` operator [wip] ([`7dbd5e2`](https://github.com/hydro-project/hydroflow/commit/7dbd5e24d6e71cf8fab7c3ce09d5937c0f301456))
 * **[#799](https://github.com/hydro-project/hydroflow/issues/799)**
    - Remove pattern deref from inspect, filter examples ([`fa5b180`](https://github.com/hydro-project/hydroflow/commit/fa5b180d96498d144f3617bba7722e8f4ac9dd0e))
 * **[#801](https://github.com/hydro-project/hydroflow/issues/801)**
    - Update proc-macro2, use new span location API where possible ([`8d3494b`](https://github.com/hydro-project/hydroflow/commit/8d3494b5afee858114a602a3e23077bb6d24dd77))
 * **[#803](https://github.com/hydro-project/hydroflow/issues/803)**
    - Add lattice_reduce and lattice_fold ([`6323980`](https://github.com/hydro-project/hydroflow/commit/6323980e83bee27a8233a69a35734b5970336701))
 * **[#804](https://github.com/hydro-project/hydroflow/issues/804)**
    - Add join_multiset() ([`0105246`](https://github.com/hydro-project/hydroflow/commit/010524615bb78288e339e03880c4dd3b432b6d7f))
 * **[#809](https://github.com/hydro-project/hydroflow/issues/809)**
    - Fold and reduce take accumulated value by mutable reference ([`b435bbb`](https://github.com/hydro-project/hydroflow/commit/b435bbb1d64d60f1248fdcd636635b15954e7325))
 * **Uncategorized**
    - Release hydroflow_cli_integration v0.3.0, hydroflow_lang v0.3.0, hydroflow_datalog_core v0.3.0, hydroflow_datalog v0.3.0, hydroflow_macro v0.3.0, lattices v0.3.0, pusherator v0.0.2, hydroflow v0.3.0, hydro_cli v0.3.0, safety bump 5 crates ([`ec9633e`](https://github.com/hydro-project/hydroflow/commit/ec9633e2e393c2bf106223abeb0b680200fbdf84))
    - Fix scheduler spinning on stateful operators across strata ([`0ecabc8`](https://github.com/hydro-project/hydroflow/commit/0ecabc80348093416ecde3de7b6bf0bb22ff30d6))
    - Add failing spinning stratum-persist bug tests ([`4675c2c`](https://github.com/hydro-project/hydroflow/commit/4675c2c334b6bb1550124a27614728fe29c53e12))
    - Ignore `surface_lattice_merge_badgeneric` `compile-fail` test due to `Seq` inconsistent messages ([`aabaa27`](https://github.com/hydro-project/hydroflow/commit/aabaa27fd736534a14f5414fb31328fad25984f3))
</details>

## 0.2.0 (2023-05-31)

<csr-id-fd896fbe925fbd8ef1d16be7206ac20ba585081a/>
<csr-id-10b308532245db8f4480ce53b67aea050ae1918d/>

### Chore

 - <csr-id-fd896fbe925fbd8ef1d16be7206ac20ba585081a/> manually bump versions for v0.2.0 release

### Documentation

 - <csr-id-6434dd8928b913370f70cb4ae68a13044d999a82/> Add `hydroflow/README.md`, integrate into book

### Refactor (BREAKING)

 - <csr-id-10b308532245db8f4480ce53b67aea050ae1918d/> rename `Fake` -> `Immut`

### `hydroflow` Commit Statistics

<csr-read-only-do-not-edit/>

 - 4 commits contributed to the release.
 - 1 day passed between releases.
 - 3 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### `hydroflow` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release hydroflow_lang v0.2.0, hydroflow_datalog_core v0.2.0, hydroflow_datalog v0.2.0, hydroflow_macro v0.2.0, lattices v0.2.0, hydroflow v0.2.0, hydro_cli v0.2.0 ([`ca464c3`](https://github.com/hydro-project/hydroflow/commit/ca464c32322a7ad39eb53e1794777c849aa548a0))
    - Add `hydroflow/README.md`, integrate into book ([`6434dd8`](https://github.com/hydro-project/hydroflow/commit/6434dd8928b913370f70cb4ae68a13044d999a82))
    - Manually bump versions for v0.2.0 release ([`fd896fb`](https://github.com/hydro-project/hydroflow/commit/fd896fbe925fbd8ef1d16be7206ac20ba585081a))
    - Rename `Fake` -> `Immut` ([`10b3085`](https://github.com/hydro-project/hydroflow/commit/10b308532245db8f4480ce53b67aea050ae1918d))
</details>

## 0.1.1 (2023-05-30)

<csr-id-d574cb2661ba086059ba8cd6904fd6b6b0a5a8cb/>
<csr-id-9029539fb79a4ca843c0775452236f80bd9510fc/>
<csr-id-e1f043c878858ea1d0531c4f318e5d011abfdb88/>
<csr-id-86f730d55f0cf3ed922d4d1ab3b02eb86d5c77bd/>
<csr-id-d13a01b3a3fa0c52381833f88bcadac7a4ebcda9/>
<csr-id-ea21462cac6d14ad744d8f0c39d5bcddc33d82ce/>
<csr-id-3608de2e8d0c8bbd67b6ecb9aa4261e5cfc955da/>
<csr-id-2843e7e114ac824a684a5400909819ccc5c88fe3/>

### Documentation

 - <csr-id-98d900f44df8c621682cafe46ac76e912e8b25fe/> minor cleanup for shopping example
 - <csr-id-28c90251dd877dd84f28886eecb7b366abf3d45b/> Add initial Hydro Deploy docs
   Renamed from Hydro CLI because the CLI isn't really the main thing. Also moves the Hydroflow docs to a subdirectory and sets up a dropdown for multiple docs.

### New Features

 - <csr-id-4536ac6bbcd14a621b5a039d7fe213bff72a8db1/> finish up WebSocket chat example and avoid deadlocks in network setup
 - <csr-id-977b9c4e8accd2ae4ae8e8798d7b72a637874b77/> add `zip_longest` operator, fix #707
   With a test.
 - <csr-id-78bc06eb09090acd46495b8e0147e3434378c9f6/> add per-tick truncating `zip` operator, fix #707
   With tests.
 - <csr-id-8d88e8e01a985db8ebd8dbc6768163452cedc3ab/> Add `multiset_delta` operator

### Bug Fixes

 - <csr-id-60afd074fdbf91268df6866716ad4c2aeb8ab9d8/> clippy fails on latest nightly in CLI integration
 - <csr-id-c771879f2fb81658f59d286ee0899065b2f2ab90/> multiset_delta not correctly tracking counts beyond two ticks
   We were swapping the `RefCell`s, but we need to swap what's _behind_ them.
 - <csr-id-075c99e7cdcf40ae5cab9efa787ba4447db8a479/> fix `persist` releasing multiple times during the same tick
   Add surface_double_handoff tests

### Other

 - <csr-id-d574cb2661ba086059ba8cd6904fd6b6b0a5a8cb/> merge() to union()
 - <csr-id-9029539fb79a4ca843c0775452236f80bd9510fc/> shopping cart working
 - <csr-id-e1f043c878858ea1d0531c4f318e5d011abfdb88/> shopping cart, compiling, wip
 - <csr-id-86f730d55f0cf3ed922d4d1ab3b02eb86d5c77bd/> Add `lamport_clock` example

### Refactor

 - <csr-id-d13a01b3a3fa0c52381833f88bcadac7a4ebcda9/> add spin(), remove repeat_iter,repeat_iter_external
   * refactor: add spin(), remove repeat_iter,repeat_iter_external
   
   * fix: fix lints
 - <csr-id-ea21462cac6d14ad744d8f0c39d5bcddc33d82ce/> change `lattice_merge` to use `reduce` instead of `fold`, fix #710
   `Default` no longer needed
 - <csr-id-3608de2e8d0c8bbd67b6ecb9aa4261e5cfc955da/> rename `sort_by` -> `sort_by_key`, fix #705
 - <csr-id-2843e7e114ac824a684a5400909819ccc5c88fe3/> Suffixes and remove keyed fold
   * rename: keyed_fold/keyed_reduce -> fold_keyed/reduce_keyed
   
   * remove group_by

### `hydroflow` Commit Statistics

<csr-read-only-do-not-edit/>

 - 18 commits contributed to the release.
 - 6 days passed between releases.
 - 17 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 12 unique issues were worked on: [#686](https://github.com/hydro-project/hydroflow/issues/686), [#690](https://github.com/hydro-project/hydroflow/issues/690), [#692](https://github.com/hydro-project/hydroflow/issues/692), [#696](https://github.com/hydro-project/hydroflow/issues/696), [#697](https://github.com/hydro-project/hydroflow/issues/697), [#702](https://github.com/hydro-project/hydroflow/issues/702), [#706](https://github.com/hydro-project/hydroflow/issues/706), [#708](https://github.com/hydro-project/hydroflow/issues/708), [#714](https://github.com/hydro-project/hydroflow/issues/714), [#716](https://github.com/hydro-project/hydroflow/issues/716), [#719](https://github.com/hydro-project/hydroflow/issues/719), [#721](https://github.com/hydro-project/hydroflow/issues/721)

### `hydroflow` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#686](https://github.com/hydro-project/hydroflow/issues/686)**
    - Add initial Hydro Deploy docs ([`28c9025`](https://github.com/hydro-project/hydroflow/commit/28c90251dd877dd84f28886eecb7b366abf3d45b))
 * **[#690](https://github.com/hydro-project/hydroflow/issues/690)**
    - Shopping cart working ([`9029539`](https://github.com/hydro-project/hydroflow/commit/9029539fb79a4ca843c0775452236f80bd9510fc))
    - Shopping cart, compiling, wip ([`e1f043c`](https://github.com/hydro-project/hydroflow/commit/e1f043c878858ea1d0531c4f318e5d011abfdb88))
    - Add `lamport_clock` example ([`86f730d`](https://github.com/hydro-project/hydroflow/commit/86f730d55f0cf3ed922d4d1ab3b02eb86d5c77bd))
 * **[#692](https://github.com/hydro-project/hydroflow/issues/692)**
    - Minor cleanup for shopping example ([`98d900f`](https://github.com/hydro-project/hydroflow/commit/98d900f44df8c621682cafe46ac76e912e8b25fe))
 * **[#696](https://github.com/hydro-project/hydroflow/issues/696)**
    - Add `multiset_delta` operator ([`8d88e8e`](https://github.com/hydro-project/hydroflow/commit/8d88e8e01a985db8ebd8dbc6768163452cedc3ab))
 * **[#697](https://github.com/hydro-project/hydroflow/issues/697)**
    - Merge() to union() ([`d574cb2`](https://github.com/hydro-project/hydroflow/commit/d574cb2661ba086059ba8cd6904fd6b6b0a5a8cb))
 * **[#702](https://github.com/hydro-project/hydroflow/issues/702)**
    - Suffixes and remove keyed fold ([`2843e7e`](https://github.com/hydro-project/hydroflow/commit/2843e7e114ac824a684a5400909819ccc5c88fe3))
 * **[#706](https://github.com/hydro-project/hydroflow/issues/706)**
    - Rename `sort_by` -> `sort_by_key`, fix #705 ([`3608de2`](https://github.com/hydro-project/hydroflow/commit/3608de2e8d0c8bbd67b6ecb9aa4261e5cfc955da))
 * **[#708](https://github.com/hydro-project/hydroflow/issues/708)**
    - Finish up WebSocket chat example and avoid deadlocks in network setup ([`4536ac6`](https://github.com/hydro-project/hydroflow/commit/4536ac6bbcd14a621b5a039d7fe213bff72a8db1))
 * **[#714](https://github.com/hydro-project/hydroflow/issues/714)**
    - Add spin(), remove repeat_iter,repeat_iter_external ([`d13a01b`](https://github.com/hydro-project/hydroflow/commit/d13a01b3a3fa0c52381833f88bcadac7a4ebcda9))
 * **[#716](https://github.com/hydro-project/hydroflow/issues/716)**
    - Fix `persist` releasing multiple times during the same tick ([`075c99e`](https://github.com/hydro-project/hydroflow/commit/075c99e7cdcf40ae5cab9efa787ba4447db8a479))
 * **[#719](https://github.com/hydro-project/hydroflow/issues/719)**
    - Multiset_delta not correctly tracking counts beyond two ticks ([`c771879`](https://github.com/hydro-project/hydroflow/commit/c771879f2fb81658f59d286ee0899065b2f2ab90))
 * **[#721](https://github.com/hydro-project/hydroflow/issues/721)**
    - Clippy fails on latest nightly in CLI integration ([`60afd07`](https://github.com/hydro-project/hydroflow/commit/60afd074fdbf91268df6866716ad4c2aeb8ab9d8))
 * **Uncategorized**
    - Release hydroflow_cli_integration v0.1.1, hydroflow_lang v0.1.1, hydroflow_datalog_core v0.1.1, hydroflow_macro v0.1.1, lattices v0.1.2, hydroflow v0.1.1, hydro_cli v0.1.0 ([`d9fa8b3`](https://github.com/hydro-project/hydroflow/commit/d9fa8b387e303b33d9614dbde80abf1af08bd8eb))
    - Change `lattice_merge` to use `reduce` instead of `fold`, fix #710 ([`ea21462`](https://github.com/hydro-project/hydroflow/commit/ea21462cac6d14ad744d8f0c39d5bcddc33d82ce))
    - Add `zip_longest` operator, fix #707 ([`977b9c4`](https://github.com/hydro-project/hydroflow/commit/977b9c4e8accd2ae4ae8e8798d7b72a637874b77))
    - Add per-tick truncating `zip` operator, fix #707 ([`78bc06e`](https://github.com/hydro-project/hydroflow/commit/78bc06eb09090acd46495b8e0147e3434378c9f6))
</details>

## 0.1.0 (2023-05-23)

<csr-id-52ee8f8e443f0a8b5caf92d2c5f028c00302a79b/>
<csr-id-faab58f855e4d6f2ad885c6f39f57ebc5662ec20/>
<csr-id-40d755e030d79def61132c005e08cd09e781fdcb/>

### Chore

 - <csr-id-52ee8f8e443f0a8b5caf92d2c5f028c00302a79b/> bump versions to 0.1.0 for release
   For release on crates.io for v0.1

### Documentation

 - <csr-id-a8957ec4457aae1cfd6fae031bede5e3f4fcc75d/> Add rustdocs to hydroflow's proc macros
 - <csr-id-7c40727db17c90406325b0f2537289b576c1122c/> Add rustdocs for the `hydroflow` crate

### Refactor

 - <csr-id-faab58f855e4d6f2ad885c6f39f57ebc5662ec20/> remove `hydroflow::lang` module, move `Clear`, `MonotonicMap` to `hydroflow::util` instead
 - <csr-id-40d755e030d79def61132c005e08cd09e781fdcb/> remove unused `PushHandoff`

### `hydroflow` Commit Statistics

<csr-read-only-do-not-edit/>

 - 8 commits contributed to the release.
 - 2 days passed between releases.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 4 unique issues were worked on: [#661](https://github.com/hydro-project/hydroflow/issues/661), [#671](https://github.com/hydro-project/hydroflow/issues/671), [#677](https://github.com/hydro-project/hydroflow/issues/677), [#684](https://github.com/hydro-project/hydroflow/issues/684)

### `hydroflow` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#661](https://github.com/hydro-project/hydroflow/issues/661)**
    - Add hydroflow_{test, main} so that hydroflow is actually singlethreaded ([`f61054e`](https://github.com/hydro-project/hydroflow/commit/f61054eaeca6fab1ab0cb588b7ed546b87772e91))
 * **[#671](https://github.com/hydro-project/hydroflow/issues/671)**
    - Migrate docs to a unified Docusuarus site ([`feed326`](https://github.com/hydro-project/hydroflow/commit/feed3268c0aabeb027b19abd9ed06c565a0462f4))
 * **[#677](https://github.com/hydro-project/hydroflow/issues/677)**
    - Add rustdocs to hydroflow's proc macros ([`a8957ec`](https://github.com/hydro-project/hydroflow/commit/a8957ec4457aae1cfd6fae031bede5e3f4fcc75d))
    - Add rustdocs for the `hydroflow` crate ([`7c40727`](https://github.com/hydro-project/hydroflow/commit/7c40727db17c90406325b0f2537289b576c1122c))
    - Remove `hydroflow::lang` module, move `Clear`, `MonotonicMap` to `hydroflow::util` instead ([`faab58f`](https://github.com/hydro-project/hydroflow/commit/faab58f855e4d6f2ad885c6f39f57ebc5662ec20))
    - Remove unused `PushHandoff` ([`40d755e`](https://github.com/hydro-project/hydroflow/commit/40d755e030d79def61132c005e08cd09e781fdcb))
 * **[#684](https://github.com/hydro-project/hydroflow/issues/684)**
    - Bump versions to 0.1.0 for release ([`52ee8f8`](https://github.com/hydro-project/hydroflow/commit/52ee8f8e443f0a8b5caf92d2c5f028c00302a79b))
 * **Uncategorized**
    - Release hydroflow_cli_integration v0.1.0, hydroflow_internalmacro v0.1.0, hydroflow_lang v0.1.0, hydroflow_datalog_core v0.1.0, hydroflow_datalog v0.1.0, hydroflow_macro v0.1.0, lattices v0.1.1, hydroflow v0.1.0 ([`7324974`](https://github.com/hydro-project/hydroflow/commit/73249744293c9b89cbaa2d84b23ca3f25b00ae4e))
</details>

## 0.0.2 (2023-05-21)

<csr-id-4d4446c0988ee7c2a991d2845b66a281934d6100/>
<csr-id-cd0a86d9271d0e3daab59c46f079925f863424e1/>
<csr-id-20a1b2c0cd04a8b495a02ce345db3d48a99ea0e9/>
<csr-id-1eda91a2ef8794711ef037240f15284e8085d863/>

### Documentation

 - <csr-id-95d23eaf8218002ad0a6a8c4c6e6c76e6b8f785b/> Update docs, add book chapter for `lattices` crate
   - Adds `mdbook-katex` to the book build for latex support.

### Style

 - <csr-id-4d4446c0988ee7c2a991d2845b66a281934d6100/> rustfmt normalize comments
 - <csr-id-cd0a86d9271d0e3daab59c46f079925f863424e1/> Warn lint `unused_qualifications`
 - <csr-id-20a1b2c0cd04a8b495a02ce345db3d48a99ea0e9/> rustfmt group imports
 - <csr-id-1eda91a2ef8794711ef037240f15284e8085d863/> rustfmt prescribe flat-module `use` format

### `hydroflow` Commit Statistics

<csr-read-only-do-not-edit/>

 - 19 commits contributed to the release.
 - 18 days passed between releases.
 - 5 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 14 unique issues were worked on: [#625](https://github.com/hydro-project/hydroflow/issues/625), [#638](https://github.com/hydro-project/hydroflow/issues/638), [#640](https://github.com/hydro-project/hydroflow/issues/640), [#641](https://github.com/hydro-project/hydroflow/issues/641), [#642](https://github.com/hydro-project/hydroflow/issues/642), [#644](https://github.com/hydro-project/hydroflow/issues/644), [#649](https://github.com/hydro-project/hydroflow/issues/649), [#650](https://github.com/hydro-project/hydroflow/issues/650), [#651](https://github.com/hydro-project/hydroflow/issues/651), [#654](https://github.com/hydro-project/hydroflow/issues/654), [#656](https://github.com/hydro-project/hydroflow/issues/656), [#657](https://github.com/hydro-project/hydroflow/issues/657), [#660](https://github.com/hydro-project/hydroflow/issues/660), [#667](https://github.com/hydro-project/hydroflow/issues/667)

### `hydroflow` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#625](https://github.com/hydro-project/hydroflow/issues/625)**
    - Use `cc-traits` instead of own `Collection`, remove `tag` indirection ([`10ed00d`](https://github.com/hydro-project/hydroflow/commit/10ed00df8e6f2e86d7db737dd2049f2c5dbfeba0))
 * **[#638](https://github.com/hydro-project/hydroflow/issues/638)**
    - Remove old lattice code ([`f4915fa`](https://github.com/hydro-project/hydroflow/commit/f4915fab98c57652e5345d39076d95ebb0a43fd8))
 * **[#640](https://github.com/hydro-project/hydroflow/issues/640)**
    - Better kvs serialization ([`c81c2a8`](https://github.com/hydro-project/hydroflow/commit/c81c2a8d78165a298c36c2b2f3f20cf2bc203986))
 * **[#641](https://github.com/hydro-project/hydroflow/issues/641)**
    - Add unsync mpsc channel ([`827e522`](https://github.com/hydro-project/hydroflow/commit/827e522ef0c3e965bb4d2ae971ec56ef7421d798))
 * **[#642](https://github.com/hydro-project/hydroflow/issues/642)**
    - Remove zmq, use unsync channels locally, use sync mpsc cross-thread, use cross_join+enumerate instead of broadcast channel,remove Eq requirement from multisetjoin ([`b38f5cf`](https://github.com/hydro-project/hydroflow/commit/b38f5cf198e29a8de2f84eb4cd075818fbeffda6))
 * **[#644](https://github.com/hydro-project/hydroflow/issues/644)**
    - Remove Compare trait, add tests, make all lattice types PartialOrd, Eq, PartialEq ([`698b72f`](https://github.com/hydro-project/hydroflow/commit/698b72f8f013288f211a655bf93f2a3cd6d386e7))
 * **[#649](https://github.com/hydro-project/hydroflow/issues/649)**
    - Add lattice_batch ([`af26532`](https://github.com/hydro-project/hydroflow/commit/af265328179f1cb1f77663cbd3e414a618583bf1))
 * **[#650](https://github.com/hydro-project/hydroflow/issues/650)**
    - Autoreturn buffer now has a generic size. ([`7da63de`](https://github.com/hydro-project/hydroflow/commit/7da63de4b20ebb914e8f929205b21bd434f7bc12))
 * **[#651](https://github.com/hydro-project/hydroflow/issues/651)**
    - More kvs improvements ([`f7515c7`](https://github.com/hydro-project/hydroflow/commit/f7515c72513e50f7080aba072bec91deac5d80ca))
 * **[#654](https://github.com/hydro-project/hydroflow/issues/654)**
    - Deduplicate `dest_sink_serde` code by using `dest_sink`'s `write_fn` ([`3b8d2f5`](https://github.com/hydro-project/hydroflow/commit/3b8d2f5e1e3a16c825171adf610d4dd6fa47c6e3))
 * **[#656](https://github.com/hydro-project/hydroflow/issues/656)**
    - Add WebSocket with CLI example and simplify init API ([`1015980`](https://github.com/hydro-project/hydroflow/commit/1015980ed995634ff8735e4daf33796e73bab563))
 * **[#657](https://github.com/hydro-project/hydroflow/issues/657)**
    - Impl `Sink` for `unsync::mpsc::Sender` ([`bc3fc21`](https://github.com/hydro-project/hydroflow/commit/bc3fc214a205c5f9ad2e8275b7e5e92689b5d1ef))
 * **[#660](https://github.com/hydro-project/hydroflow/issues/660)**
    - Rustfmt normalize comments ([`4d4446c`](https://github.com/hydro-project/hydroflow/commit/4d4446c0988ee7c2a991d2845b66a281934d6100))
    - Warn lint `unused_qualifications` ([`cd0a86d`](https://github.com/hydro-project/hydroflow/commit/cd0a86d9271d0e3daab59c46f079925f863424e1))
    - Rustfmt group imports ([`20a1b2c`](https://github.com/hydro-project/hydroflow/commit/20a1b2c0cd04a8b495a02ce345db3d48a99ea0e9))
    - Rustfmt prescribe flat-module `use` format ([`1eda91a`](https://github.com/hydro-project/hydroflow/commit/1eda91a2ef8794711ef037240f15284e8085d863))
 * **[#667](https://github.com/hydro-project/hydroflow/issues/667)**
    - Bump lattices version to `0.1.0` ([`a46ce4a`](https://github.com/hydro-project/hydroflow/commit/a46ce4a522b70661e5acf644f893bfdf56294578))
    - Update docs, add book chapter for `lattices` crate ([`95d23ea`](https://github.com/hydro-project/hydroflow/commit/95d23eaf8218002ad0a6a8c4c6e6c76e6b8f785b))
 * **Uncategorized**
    - Release hydroflow_cli_integration v0.0.1, hydroflow_lang v0.0.1, hydroflow_datalog_core v0.0.1, hydroflow_datalog v0.0.1, hydroflow_macro v0.0.1, lattices v0.1.0, variadics v0.0.2, pusherator v0.0.1, hydroflow v0.0.2 ([`809395a`](https://github.com/hydro-project/hydroflow/commit/809395acddb78949d7a2bf036e1a94972f23b1ad))
</details>

## 0.0.1 (2023-05-03)

<csr-id-e58a58061de1e92f5176c01f4908dfe944a2b74a/>
<csr-id-ad72c6d1f0e95a886d533621ae24afacf13de86a/>

### Other

 - <csr-id-e58a58061de1e92f5176c01f4908dfe944a2b74a/> separate constructors into `new` and `new_from`
 - <csr-id-ad72c6d1f0e95a886d533621ae24afacf13de86a/> Add `NaiveCompare`, fix `Ord` & `SetUnion` bugs
   - Fix `Min` compare reversed
   - Impl compare for `Max`
   - Fix `SetUnion` compare logical error (long standing old bug)
   - Add `NaiveCompare` trait based on merge fn
   - Improve `SetUnion` tests

### `hydroflow` Commit Statistics

<csr-read-only-do-not-edit/>

 - 7 commits contributed to the release.
 - 7 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 6 unique issues were worked on: [#622](https://github.com/hydro-project/hydroflow/issues/622), [#629](https://github.com/hydro-project/hydroflow/issues/629), [#632](https://github.com/hydro-project/hydroflow/issues/632), [#633](https://github.com/hydro-project/hydroflow/issues/633), [#634](https://github.com/hydro-project/hydroflow/issues/634), [#635](https://github.com/hydro-project/hydroflow/issues/635)

### `hydroflow` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#622](https://github.com/hydro-project/hydroflow/issues/622)**
    - Implement alternative lattice format without `LatticeRepr` indirection ([`5278437`](https://github.com/hydro-project/hydroflow/commit/5278437ba662c0191e732047f9bf31bd4f7f8965))
 * **[#629](https://github.com/hydro-project/hydroflow/issues/629)**
    - Add `NaiveCompare`, fix `Ord` & `SetUnion` bugs ([`ad72c6d`](https://github.com/hydro-project/hydroflow/commit/ad72c6d1f0e95a886d533621ae24afacf13de86a))
 * **[#632](https://github.com/hydro-project/hydroflow/issues/632)**
    - Separate constructors into `new` and `new_from` ([`e58a580`](https://github.com/hydro-project/hydroflow/commit/e58a58061de1e92f5176c01f4908dfe944a2b74a))
 * **[#633](https://github.com/hydro-project/hydroflow/issues/633)**
    - Update pinned nightly to `nightly-2023-05-01` ([`1cd506f`](https://github.com/hydro-project/hydroflow/commit/1cd506fd05b121bb2071f1a1d25968e68dc7cfc2))
 * **[#634](https://github.com/hydro-project/hydroflow/issues/634)**
    - Move lattice2 into new separate `lattices` crate ([`c0006c4`](https://github.com/hydro-project/hydroflow/commit/c0006c4c73e0f3f5c65274e3ad76537ea9fe2643))
 * **[#635](https://github.com/hydro-project/hydroflow/issues/635)**
    - Cleanup `hydroflow` dependencies ([`daaa8fa`](https://github.com/hydro-project/hydroflow/commit/daaa8fa8c362d5d8136955c538db8482c65ec5eb))
 * **Uncategorized**
    - Release hydroflow v0.0.1 ([`0f91773`](https://github.com/hydro-project/hydroflow/commit/0f917734bd2c840e47acf788322dd1a4a4100488))
</details>

## 0.0.0 (2023-04-25)

<csr-id-62fcfb157eaaaabedfeb5c77b2a6df89ee1a6852/>
<csr-id-bc3d12f563dab96f4751ec21cd20b193eea95456/>
<csr-id-a2078f7056a54d20f91e2e0f9a7617dc6ef1f627/>
<csr-id-11a12b2daa3496ed36af4ca78d87454411da3839/>
<csr-id-eebd4075e83808d51b40b4850e60ddba52f5862c/>
<csr-id-1edbf7e06c28eb4cc7c0632712891ac547a72e04/>

### Other

 - <csr-id-62fcfb157eaaaabedfeb5c77b2a6df89ee1a6852/> :<'static> now replays #143 #364
 - <csr-id-bc3d12f563dab96f4751ec21cd20b193eea95456/> :<'static> now replays #143 #364
 - <csr-id-a2078f7056a54d20f91e2e0f9a7617dc6ef1f627/> :<'static> now replays #143 #364
 - <csr-id-11a12b2daa3496ed36af4ca78d87454411da3839/> update example-1, write first half of example-2
 - <csr-id-eebd4075e83808d51b40b4850e60ddba52f5862c/> pull benchmark into separate file
   Also remove the reporting of latency information for now, since it was
   vulnerable to coordinated omission and is not reliable.
 - <csr-id-1edbf7e06c28eb4cc7c0632712891ac547a72e04/> add a benchmarking harness
   Still pretty simple, but the beginnings of a configurable benchmarking harness
   for the KVS.

### `hydroflow` Commit Statistics

<csr-read-only-do-not-edit/>

 - 489 commits contributed to the release.
 - 6 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 191 unique issues were worked on: [#100](https://github.com/hydro-project/hydroflow/issues/100), [#101](https://github.com/hydro-project/hydroflow/issues/101), [#102](https://github.com/hydro-project/hydroflow/issues/102), [#111](https://github.com/hydro-project/hydroflow/issues/111), [#113](https://github.com/hydro-project/hydroflow/issues/113), [#116](https://github.com/hydro-project/hydroflow/issues/116), [#12](https://github.com/hydro-project/hydroflow/issues/12), [#120](https://github.com/hydro-project/hydroflow/issues/120), [#121](https://github.com/hydro-project/hydroflow/issues/121), [#122](https://github.com/hydro-project/hydroflow/issues/122), [#127](https://github.com/hydro-project/hydroflow/issues/127), [#137](https://github.com/hydro-project/hydroflow/issues/137), [#146](https://github.com/hydro-project/hydroflow/issues/146), [#147](https://github.com/hydro-project/hydroflow/issues/147), [#15](https://github.com/hydro-project/hydroflow/issues/15), [#155](https://github.com/hydro-project/hydroflow/issues/155), [#160](https://github.com/hydro-project/hydroflow/issues/160), [#162](https://github.com/hydro-project/hydroflow/issues/162), [#163](https://github.com/hydro-project/hydroflow/issues/163), [#164](https://github.com/hydro-project/hydroflow/issues/164), [#165](https://github.com/hydro-project/hydroflow/issues/165), [#18](https://github.com/hydro-project/hydroflow/issues/18), [#184](https://github.com/hydro-project/hydroflow/issues/184), [#187](https://github.com/hydro-project/hydroflow/issues/187), [#190](https://github.com/hydro-project/hydroflow/issues/190), [#207](https://github.com/hydro-project/hydroflow/issues/207), [#209](https://github.com/hydro-project/hydroflow/issues/209), [#211](https://github.com/hydro-project/hydroflow/issues/211), [#213](https://github.com/hydro-project/hydroflow/issues/213), [#222](https://github.com/hydro-project/hydroflow/issues/222), [#230](https://github.com/hydro-project/hydroflow/issues/230), [#231](https://github.com/hydro-project/hydroflow/issues/231), [#234](https://github.com/hydro-project/hydroflow/issues/234), [#235](https://github.com/hydro-project/hydroflow/issues/235), [#236](https://github.com/hydro-project/hydroflow/issues/236), [#237](https://github.com/hydro-project/hydroflow/issues/237), [#238](https://github.com/hydro-project/hydroflow/issues/238), [#239](https://github.com/hydro-project/hydroflow/issues/239), [#248](https://github.com/hydro-project/hydroflow/issues/248), [#249](https://github.com/hydro-project/hydroflow/issues/249), [#250](https://github.com/hydro-project/hydroflow/issues/250), [#254](https://github.com/hydro-project/hydroflow/issues/254), [#259](https://github.com/hydro-project/hydroflow/issues/259), [#261](https://github.com/hydro-project/hydroflow/issues/261), [#262](https://github.com/hydro-project/hydroflow/issues/262), [#268](https://github.com/hydro-project/hydroflow/issues/268), [#277](https://github.com/hydro-project/hydroflow/issues/277), [#278](https://github.com/hydro-project/hydroflow/issues/278), [#279](https://github.com/hydro-project/hydroflow/issues/279), [#28](https://github.com/hydro-project/hydroflow/issues/28), [#282](https://github.com/hydro-project/hydroflow/issues/282), [#284](https://github.com/hydro-project/hydroflow/issues/284), [#285](https://github.com/hydro-project/hydroflow/issues/285), [#288](https://github.com/hydro-project/hydroflow/issues/288), [#294](https://github.com/hydro-project/hydroflow/issues/294), [#295](https://github.com/hydro-project/hydroflow/issues/295), [#296](https://github.com/hydro-project/hydroflow/issues/296), [#298](https://github.com/hydro-project/hydroflow/issues/298), [#3](https://github.com/hydro-project/hydroflow/issues/3), [#30](https://github.com/hydro-project/hydroflow/issues/30), [#300](https://github.com/hydro-project/hydroflow/issues/300), [#301](https://github.com/hydro-project/hydroflow/issues/301), [#307](https://github.com/hydro-project/hydroflow/issues/307), [#309](https://github.com/hydro-project/hydroflow/issues/309), [#32](https://github.com/hydro-project/hydroflow/issues/32), [#321](https://github.com/hydro-project/hydroflow/issues/321), [#329](https://github.com/hydro-project/hydroflow/issues/329), [#33](https://github.com/hydro-project/hydroflow/issues/33), [#333](https://github.com/hydro-project/hydroflow/issues/333), [#34](https://github.com/hydro-project/hydroflow/issues/34), [#344](https://github.com/hydro-project/hydroflow/issues/344), [#350](https://github.com/hydro-project/hydroflow/issues/350), [#358](https://github.com/hydro-project/hydroflow/issues/358), [#363](https://github.com/hydro-project/hydroflow/issues/363), [#369](https://github.com/hydro-project/hydroflow/issues/369), [#37](https://github.com/hydro-project/hydroflow/issues/37), [#372](https://github.com/hydro-project/hydroflow/issues/372), [#374](https://github.com/hydro-project/hydroflow/issues/374), [#376](https://github.com/hydro-project/hydroflow/issues/376), [#38](https://github.com/hydro-project/hydroflow/issues/38), [#381](https://github.com/hydro-project/hydroflow/issues/381), [#382](https://github.com/hydro-project/hydroflow/issues/382), [#383](https://github.com/hydro-project/hydroflow/issues/383), [#388](https://github.com/hydro-project/hydroflow/issues/388), [#39](https://github.com/hydro-project/hydroflow/issues/39), [#397](https://github.com/hydro-project/hydroflow/issues/397), [#40](https://github.com/hydro-project/hydroflow/issues/40), [#403](https://github.com/hydro-project/hydroflow/issues/403), [#409](https://github.com/hydro-project/hydroflow/issues/409), [#411](https://github.com/hydro-project/hydroflow/issues/411), [#412](https://github.com/hydro-project/hydroflow/issues/412), [#413](https://github.com/hydro-project/hydroflow/issues/413), [#417](https://github.com/hydro-project/hydroflow/issues/417), [#420](https://github.com/hydro-project/hydroflow/issues/420), [#43](https://github.com/hydro-project/hydroflow/issues/43), [#431](https://github.com/hydro-project/hydroflow/issues/431), [#435](https://github.com/hydro-project/hydroflow/issues/435), [#437](https://github.com/hydro-project/hydroflow/issues/437), [#442](https://github.com/hydro-project/hydroflow/issues/442), [#443](https://github.com/hydro-project/hydroflow/issues/443), [#444](https://github.com/hydro-project/hydroflow/issues/444), [#445](https://github.com/hydro-project/hydroflow/issues/445), [#448 1/2](https://github.com/hydro-project/hydroflow/issues/448 1/2), [#448 2/2](https://github.com/hydro-project/hydroflow/issues/448 2/2), [#452](https://github.com/hydro-project/hydroflow/issues/452), [#459](https://github.com/hydro-project/hydroflow/issues/459), [#46](https://github.com/hydro-project/hydroflow/issues/46), [#460](https://github.com/hydro-project/hydroflow/issues/460), [#461](https://github.com/hydro-project/hydroflow/issues/461), [#465](https://github.com/hydro-project/hydroflow/issues/465), [#466](https://github.com/hydro-project/hydroflow/issues/466), [#468](https://github.com/hydro-project/hydroflow/issues/468), [#469](https://github.com/hydro-project/hydroflow/issues/469), [#470](https://github.com/hydro-project/hydroflow/issues/470), [#471](https://github.com/hydro-project/hydroflow/issues/471), [#472](https://github.com/hydro-project/hydroflow/issues/472), [#475](https://github.com/hydro-project/hydroflow/issues/475), [#477](https://github.com/hydro-project/hydroflow/issues/477), [#479](https://github.com/hydro-project/hydroflow/issues/479), [#48](https://github.com/hydro-project/hydroflow/issues/48), [#484](https://github.com/hydro-project/hydroflow/issues/484), [#487](https://github.com/hydro-project/hydroflow/issues/487), [#492](https://github.com/hydro-project/hydroflow/issues/492), [#493](https://github.com/hydro-project/hydroflow/issues/493), [#495](https://github.com/hydro-project/hydroflow/issues/495), [#497](https://github.com/hydro-project/hydroflow/issues/497), [#499](https://github.com/hydro-project/hydroflow/issues/499), [#500](https://github.com/hydro-project/hydroflow/issues/500), [#501](https://github.com/hydro-project/hydroflow/issues/501), [#502](https://github.com/hydro-project/hydroflow/issues/502), [#505](https://github.com/hydro-project/hydroflow/issues/505), [#509](https://github.com/hydro-project/hydroflow/issues/509), [#511](https://github.com/hydro-project/hydroflow/issues/511), [#512](https://github.com/hydro-project/hydroflow/issues/512), [#516](https://github.com/hydro-project/hydroflow/issues/516), [#518](https://github.com/hydro-project/hydroflow/issues/518), [#520](https://github.com/hydro-project/hydroflow/issues/520), [#521](https://github.com/hydro-project/hydroflow/issues/521), [#522](https://github.com/hydro-project/hydroflow/issues/522), [#523](https://github.com/hydro-project/hydroflow/issues/523), [#524](https://github.com/hydro-project/hydroflow/issues/524), [#526](https://github.com/hydro-project/hydroflow/issues/526), [#529](https://github.com/hydro-project/hydroflow/issues/529), [#530](https://github.com/hydro-project/hydroflow/issues/530), [#536](https://github.com/hydro-project/hydroflow/issues/536), [#538](https://github.com/hydro-project/hydroflow/issues/538), [#540](https://github.com/hydro-project/hydroflow/issues/540), [#541](https://github.com/hydro-project/hydroflow/issues/541), [#543](https://github.com/hydro-project/hydroflow/issues/543), [#547](https://github.com/hydro-project/hydroflow/issues/547), [#548 #550](https://github.com/hydro-project/hydroflow/issues/548 #550), [#549](https://github.com/hydro-project/hydroflow/issues/549), [#551](https://github.com/hydro-project/hydroflow/issues/551), [#555](https://github.com/hydro-project/hydroflow/issues/555), [#556](https://github.com/hydro-project/hydroflow/issues/556), [#558](https://github.com/hydro-project/hydroflow/issues/558), [#559](https://github.com/hydro-project/hydroflow/issues/559), [#56](https://github.com/hydro-project/hydroflow/issues/56), [#566](https://github.com/hydro-project/hydroflow/issues/566), [#567](https://github.com/hydro-project/hydroflow/issues/567), [#568](https://github.com/hydro-project/hydroflow/issues/568), [#571](https://github.com/hydro-project/hydroflow/issues/571), [#576](https://github.com/hydro-project/hydroflow/issues/576), [#578](https://github.com/hydro-project/hydroflow/issues/578), [#584](https://github.com/hydro-project/hydroflow/issues/584), [#586](https://github.com/hydro-project/hydroflow/issues/586), [#589](https://github.com/hydro-project/hydroflow/issues/589), [#590](https://github.com/hydro-project/hydroflow/issues/590), [#591](https://github.com/hydro-project/hydroflow/issues/591), [#593](https://github.com/hydro-project/hydroflow/issues/593), [#597](https://github.com/hydro-project/hydroflow/issues/597), [#598](https://github.com/hydro-project/hydroflow/issues/598), [#6](https://github.com/hydro-project/hydroflow/issues/6), [#60](https://github.com/hydro-project/hydroflow/issues/60), [#602](https://github.com/hydro-project/hydroflow/issues/602), [#605](https://github.com/hydro-project/hydroflow/issues/605), [#607](https://github.com/hydro-project/hydroflow/issues/607), [#608](https://github.com/hydro-project/hydroflow/issues/608), [#61](https://github.com/hydro-project/hydroflow/issues/61), [#617](https://github.com/hydro-project/hydroflow/issues/617), [#618](https://github.com/hydro-project/hydroflow/issues/618), [#63](https://github.com/hydro-project/hydroflow/issues/63), [#64](https://github.com/hydro-project/hydroflow/issues/64), [#8](https://github.com/hydro-project/hydroflow/issues/8), [#80](https://github.com/hydro-project/hydroflow/issues/80), [#84](https://github.com/hydro-project/hydroflow/issues/84), [#86](https://github.com/hydro-project/hydroflow/issues/86), [#89](https://github.com/hydro-project/hydroflow/issues/89), [#91](https://github.com/hydro-project/hydroflow/issues/91), [#95](https://github.com/hydro-project/hydroflow/issues/95), [#98](https://github.com/hydro-project/hydroflow/issues/98)

### `hydroflow` Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#100](https://github.com/hydro-project/hydroflow/issues/100)**
    - Make groupby tests more relational-looking ([`71fa71a`](https://github.com/hydro-project/hydroflow/commit/71fa71a73cdeade8b18bfa4e5d051f5f0976c990))
 * **[#101](https://github.com/hydro-project/hydroflow/issues/101)**
    - Move KVS to examples ([`54c7070`](https://github.com/hydro-project/hydroflow/commit/54c70707ffb8e9c00d71f8ed7c389f94e04846c2))
 * **[#102](https://github.com/hydro-project/hydroflow/issues/102)**
    - Add a nicer interface to KVS ([`ccdc19f`](https://github.com/hydro-project/hydroflow/commit/ccdc19f37daf6db09251077a326f237fe073bed6))
 * **[#111](https://github.com/hydro-project/hydroflow/issues/111)**
    - 2PC with Group-By ([`423b714`](https://github.com/hydro-project/hydroflow/commit/423b714ca6d28c30c124951848ebbbe4e31a20be))
 * **[#113](https://github.com/hydro-project/hydroflow/issues/113)**
    - Add a benchmarking harness ([`1edbf7e`](https://github.com/hydro-project/hydroflow/commit/1edbf7e06c28eb4cc7c0632712891ac547a72e04))
 * **[#116](https://github.com/hydro-project/hydroflow/issues/116)**
    - Pull benchmark into separate file ([`eebd407`](https://github.com/hydro-project/hydroflow/commit/eebd4075e83808d51b40b4850e60ddba52f5862c))
 * **[#12](https://github.com/hydro-project/hydroflow/issues/12)**
    - Allow creating an input operator from an existing channel ([`1c70cf8`](https://github.com/hydro-project/hydroflow/commit/1c70cf861b20b88ec648f968cc7ca1581bd20c9c))
 * **[#120](https://github.com/hydro-project/hydroflow/issues/120)**
    - Mermaid graph generation ([`1445f80`](https://github.com/hydro-project/hydroflow/commit/1445f8060c51a896070f90a0ba295f2b4dd414cb))
 * **[#121](https://github.com/hydro-project/hydroflow/issues/121)**
    - Add fold_epoch to surface API ([`dcdba24`](https://github.com/hydro-project/hydroflow/commit/dcdba24da73c1e0d000d846e32ef1e853a1b2b6c))
 * **[#122](https://github.com/hydro-project/hydroflow/issues/122)**
    - Fixup! Add another implementation of KVS ([`c33f81a`](https://github.com/hydro-project/hydroflow/commit/c33f81a10963ff2878b687fa9fd4e8bfc3a34fcd))
    - Add another implementation of KVS ([`a0d6711`](https://github.com/hydro-project/hydroflow/commit/a0d67113688344092dea22aeb380a9b6ca8558b6))
 * **[#127](https://github.com/hydro-project/hydroflow/issues/127)**
    - Dot graphs and indents for mermaid ([`47536c4`](https://github.com/hydro-project/hydroflow/commit/47536c487020b32366628dbc60c6d9b9178dd3c6))
 * **[#137](https://github.com/hydro-project/hydroflow/issues/137)**
    - Re-add the "raw" implementation of the KVS ([`d020b01`](https://github.com/hydro-project/hydroflow/commit/d020b01869532136cc7a0bf45629c86dc5c411c2))
 * **[#146](https://github.com/hydro-project/hydroflow/issues/146)**
    - Make KVS benchmark more Anna-like ([`c55745d`](https://github.com/hydro-project/hydroflow/commit/c55745dcc9fb838fd0bebcfb11a5a181918d19b8))
 * **[#147](https://github.com/hydro-project/hydroflow/issues/147)**
    - Improve docs in KVS ([`36886fc`](https://github.com/hydro-project/hydroflow/commit/36886fc8d0f6f25f3b20c79b6f8613a8e81c2be9))
 * **[#15](https://github.com/hydro-project/hydroflow/issues/15)**
    - Add distributed Covid tracing app ([`246d437`](https://github.com/hydro-project/hydroflow/commit/246d437f15437ff258ae835fa49b922d82892c63))
 * **[#155](https://github.com/hydro-project/hydroflow/issues/155)**
    - Add datalog frontend via a proc macro ([`fd3867f`](https://github.com/hydro-project/hydroflow/commit/fd3867fde4302aabd747ca81564dfba6016a6395))
 * **[#160](https://github.com/hydro-project/hydroflow/issues/160)**
    - Clean up transitive closure test ([`b89c197`](https://github.com/hydro-project/hydroflow/commit/b89c1976a180bcf18660ccde817d662791353470))
 * **[#162](https://github.com/hydro-project/hydroflow/issues/162)**
    - SerdeGraph from parser to be callable at runtime ([`17dd150`](https://github.com/hydro-project/hydroflow/commit/17dd1500be1dab5f7abbd498d8f96b6ed00dba59))
 * **[#163](https://github.com/hydro-project/hydroflow/issues/163)**
    - Initial docs on surface syntax ([`7002682`](https://github.com/hydro-project/hydroflow/commit/700268234d110c228c5e89c489ef698840863066))
 * **[#164](https://github.com/hydro-project/hydroflow/issues/164)**
    - Document operators in the book ([`c1893df`](https://github.com/hydro-project/hydroflow/commit/c1893dfdce96d31a3dc15626e0087ac7380403a5))
 * **[#165](https://github.com/hydro-project/hydroflow/issues/165)**
    - Full pass of book with surface syntax ([`db6c16f`](https://github.com/hydro-project/hydroflow/commit/db6c16f329ef47f49b488c1f99dd95ffcf0ec59d))
 * **[#18](https://github.com/hydro-project/hydroflow/issues/18)**
    - Push some of the networking stuff into a library ([`418803b`](https://github.com/hydro-project/hydroflow/commit/418803b7c86a0ebe0cf29ebec47c99108afc1cee))
 * **[#184](https://github.com/hydro-project/hydroflow/issues/184)**
    - Generate nested joins for rules with more than two RHS relations ([`863fdc8`](https://github.com/hydro-project/hydroflow/commit/863fdc8fea27d3b41dd3bd94212bee515a923340))
 * **[#187](https://github.com/hydro-project/hydroflow/issues/187)**
    - Emit relation filters when there are local constraints ([`28ed51b`](https://github.com/hydro-project/hydroflow/commit/28ed51bcd785a9098d42d4c1e6838c95831b42f4))
 * **[#190](https://github.com/hydro-project/hydroflow/issues/190)**
    - Book edits, fix #183 ([`5a17779`](https://github.com/hydro-project/hydroflow/commit/5a1777991cd72a566596b4d7b0375387c6985967))
 * **[#207](https://github.com/hydro-project/hydroflow/issues/207)**
    - Start porting examples to surface syntax ([`01ff0c6`](https://github.com/hydro-project/hydroflow/commit/01ff0c6442bf146d4a7e076e2c121ea4c4317ed9))
 * **[#209](https://github.com/hydro-project/hydroflow/issues/209)**
    - Add filtering expressions to Dedalus rules. #178 ([`7462cc3`](https://github.com/hydro-project/hydroflow/commit/7462cc35e4953e7740a512285dc70ad8628eff6c))
 * **[#211](https://github.com/hydro-project/hydroflow/issues/211)**
    - Add cross join surface syntax operator, update tests, fix #200 ([`c526f9a`](https://github.com/hydro-project/hydroflow/commit/c526f9a70de0d9a5d15655ad99412f3b425b4cab))
 * **[#213](https://github.com/hydro-project/hydroflow/issues/213)**
    - Add flatten op to surface syntax ([`f802b95`](https://github.com/hydro-project/hydroflow/commit/f802b9536cf9d07846e2ace54b09786c919aea11))
 * **[#222](https://github.com/hydro-project/hydroflow/issues/222)**
    - Fix example graph printing ([`f141609`](https://github.com/hydro-project/hydroflow/commit/f141609d8f9560db77b67dee28415a947ef0517c))
 * **[#230](https://github.com/hydro-project/hydroflow/issues/230)**
    - Add testing of surface syntax errors (and warnings) ([`b8394d8`](https://github.com/hydro-project/hydroflow/commit/b8394d8da3479be55a19fe5743285d8480f78c61))
 * **[#231](https://github.com/hydro-project/hydroflow/issues/231)**
    - Move type list code into `type_list` subpackage ([`3b7c998`](https://github.com/hydro-project/hydroflow/commit/3b7c9981a268b0bc6023e4c9d73c9dbceee02a4f))
 * **[#234](https://github.com/hydro-project/hydroflow/issues/234)**
    - Deadlock detector example ([`b90c546`](https://github.com/hydro-project/hydroflow/commit/b90c5460bc66d0386725ba8dae313f27a8dceca1))
 * **[#235](https://github.com/hydro-project/hydroflow/issues/235)**
    - Remove FlowGraph and related graphing facilities ([`39764c5`](https://github.com/hydro-project/hydroflow/commit/39764c554c52ff0e05b91d6e7652794fc01d2b93))
 * **[#236](https://github.com/hydro-project/hydroflow/issues/236)**
    - Add unique operator to remove duplicates ([`e3e8db2`](https://github.com/hydro-project/hydroflow/commit/e3e8db208606bd354426332ca128a894f0e9f76e))
 * **[#237](https://github.com/hydro-project/hydroflow/issues/237)**
    - Add a driver for the chat example ([`2230af8`](https://github.com/hydro-project/hydroflow/commit/2230af8282ed9ddeb2a2e502a6e4737b9d0a2b0d))
 * **[#238](https://github.com/hydro-project/hydroflow/issues/238)**
    - Allow chat clients to connect to non-localhost servers ([`e52054f`](https://github.com/hydro-project/hydroflow/commit/e52054f6ba06ea8e8b72c9883e53fcb5bafec5ee))
 * **[#239](https://github.com/hydro-project/hydroflow/issues/239)**
    - First version of groupby with test and example ([`c85a19d`](https://github.com/hydro-project/hydroflow/commit/c85a19d081e2c53da21700163ad3e6178b59fc33))
 * **[#248](https://github.com/hydro-project/hydroflow/issues/248)**
    - Refining the examples container's definition ([`bf5e7dd`](https://github.com/hydro-project/hydroflow/commit/bf5e7ddd86aea3ac502f0fc8de2de342e846fde4))
 * **[#249](https://github.com/hydro-project/hydroflow/issues/249)**
    - Allow the chat example to bind and connect to DNS names ([`dd3d9e1`](https://github.com/hydro-project/hydroflow/commit/dd3d9e16f9f4fa4ec1426430dd28e4af2a6f445a))
 * **[#250](https://github.com/hydro-project/hydroflow/issues/250)**
    - Limit `null()` to up to one input and/or output. ([`05a05bb`](https://github.com/hydro-project/hydroflow/commit/05a05bb81f780141e727a47cbe4cdcef31e4a311))
 * **[#254](https://github.com/hydro-project/hydroflow/issues/254)**
    - Dedalus support for count, sum, choose, and comments. ([`e28c8d6`](https://github.com/hydro-project/hydroflow/commit/e28c8d694ced16212ece5bef0fe485853b210096))
 * **[#259](https://github.com/hydro-project/hydroflow/issues/259)**
    - Rename split->unzip, implement surface op ([`293c37c`](https://github.com/hydro-project/hydroflow/commit/293c37cd477c88af4ff0a3aaeb15a2da30ea391b))
 * **[#261](https://github.com/hydro-project/hydroflow/issues/261)**
    - Add demux operator ([`d07e5c1`](https://github.com/hydro-project/hydroflow/commit/d07e5c16be1bf3de627cd0f45225146129a6ab41))
 * **[#262](https://github.com/hydro-project/hydroflow/issues/262)**
    - Initial kvs example ([`458aa0b`](https://github.com/hydro-project/hydroflow/commit/458aa0bb383d8102919b056743785d9f2d7538aa))
 * **[#268](https://github.com/hydro-project/hydroflow/issues/268)**
    - Use demux rather than tee and filter_map ([`45f76a8`](https://github.com/hydro-project/hydroflow/commit/45f76a8fc2abec5e88832f297a9b4ad865aec339))
 * **[#277](https://github.com/hydro-project/hydroflow/issues/277)**
    - Improvements to book ([`a98c745`](https://github.com/hydro-project/hydroflow/commit/a98c7453df1ff1733000f5281f4ac9f9f5403537))
 * **[#278](https://github.com/hydro-project/hydroflow/issues/278)**
    - Add operator-specific diagnostics, use in `demux(..)`, fix #265 ([`7341f87`](https://github.com/hydro-project/hydroflow/commit/7341f87c821bcb534d232ce02fd113853c2ef17a))
 * **[#279](https://github.com/hydro-project/hydroflow/issues/279)**
    - Echo Server example ([`9d9a7ec`](https://github.com/hydro-project/hydroflow/commit/9d9a7ec4186b54f2fdb5ec07fe4f364618745258))
 * **[#28](https://github.com/hydro-project/hydroflow/issues/28)**
    - Pull apart scheduled/mod.rs ([`7a35e78`](https://github.com/hydro-project/hydroflow/commit/7a35e78caa98864d5b4f560fbc2d4bcaa1352f4d))
 * **[#282](https://github.com/hydro-project/hydroflow/issues/282)**
    - Simplify boilerplate with new helpers, ops ([`57403cc`](https://github.com/hydro-project/hydroflow/commit/57403ccc3d66c07b4e1631a504904286a9cf28c3))
 * **[#284](https://github.com/hydro-project/hydroflow/issues/284)**
    - Rename source and dest surface syntax operators, fix #216 #276 ([`b7074eb`](https://github.com/hydro-project/hydroflow/commit/b7074ebb5d376493b52efe471b65f6e2c06fce7c))
 * **[#285](https://github.com/hydro-project/hydroflow/issues/285)**
    - `demux` use `Pusherator` automatically, fix #267 ([`36708ab`](https://github.com/hydro-project/hydroflow/commit/36708abaa599a0da4966c1265e97fcc9e5f08224))
 * **[#288](https://github.com/hydro-project/hydroflow/issues/288)**
    - Book edits: echoserver example ([`d7dcd88`](https://github.com/hydro-project/hydroflow/commit/d7dcd888295f48a0e6cbdd9c9a9c38c46cbe9c8f))
 * **[#294](https://github.com/hydro-project/hydroflow/issues/294)**
    - Add chat server example to book ([`419bc30`](https://github.com/hydro-project/hydroflow/commit/419bc308764c080f10ce559a6d73982507eed78e))
 * **[#295](https://github.com/hydro-project/hydroflow/issues/295)**
    - Explicit serde example, resolve #214 ([`96f1481`](https://github.com/hydro-project/hydroflow/commit/96f1481fb73b2411f4afa161142ebd64b901ec60))
 * **[#296](https://github.com/hydro-project/hydroflow/issues/296)**
    - Make ipv4_resolve return a Result for use in clap ([`a2316df`](https://github.com/hydro-project/hydroflow/commit/a2316df30aecee9a083b345702d6f948fd2889a0))
 * **[#298](https://github.com/hydro-project/hydroflow/issues/298)**
    - Better names/structure for serde helper functions, get UdpSocket back from bind_udp_xxx calls ([`e6b1ec5`](https://github.com/hydro-project/hydroflow/commit/e6b1ec569afaba424ad8c7d18fdeef0d5344ca23))
 * **[#3](https://github.com/hydro-project/hydroflow/issues/3)**
    - Make the writer construct the handoff instead of the reader ([`8632228`](https://github.com/hydro-project/hydroflow/commit/863222837878c897f721c2ac9686c72bf239a839))
 * **[#30](https://github.com/hydro-project/hydroflow/issues/30)**
    - Add a "demux" operator ([`9f85e4a`](https://github.com/hydro-project/hydroflow/commit/9f85e4ac9f72ea49a82fea07fc0d4ed416c432cb))
 * **[#300](https://github.com/hydro-project/hydroflow/issues/300)**
    - Align book with cleaned up examples for echo and chat ([`328026d`](https://github.com/hydro-project/hydroflow/commit/328026d512534040197fd5ff3cf000ce5650eae6))
 * **[#301](https://github.com/hydro-project/hydroflow/issues/301)**
    - Add sort_by, rename groupby to group_by ([`b5d6f60`](https://github.com/hydro-project/hydroflow/commit/b5d6f6086b37df15a73b199a4ad638596af82a34))
 * **[#307](https://github.com/hydro-project/hydroflow/issues/307)**
    - Use `cargo generate` template in book ([`5de534b`](https://github.com/hydro-project/hydroflow/commit/5de534b0d2229af1dd3c860c010836089f26e3a4))
 * **[#309](https://github.com/hydro-project/hydroflow/issues/309)**
    - `epoch` --> `tick` replace ([`f4ad527`](https://github.com/hydro-project/hydroflow/commit/f4ad527151f9cb9d04616fe252ed1d54ea13d19d))
 * **[#32](https://github.com/hydro-project/hydroflow/issues/32)**
    - Add a real networked test and polish the API a bit ([`fc48b71`](https://github.com/hydro-project/hydroflow/commit/fc48b7161821ac7945792b73f1a2427e6faa434c))
 * **[#321](https://github.com/hydro-project/hydroflow/issues/321)**
    - Better graphs for both mermaid and dot ([`876fb31`](https://github.com/hydro-project/hydroflow/commit/876fb3140374588c55b4a7ec7a51e7cf6317eb67))
 * **[#329](https://github.com/hydro-project/hydroflow/issues/329)**
    - Get hydroflow to compile to WASM ([`24354d2`](https://github.com/hydro-project/hydroflow/commit/24354d2e11c69e38e4e021aa4acf1525b376b2b1))
 * **[#33](https://github.com/hydro-project/hydroflow/issues/33)**
    - Cross join ([`efdb647`](https://github.com/hydro-project/hydroflow/commit/efdb647f87c3402477f48ef169e32c4f2b458d58))
 * **[#333](https://github.com/hydro-project/hydroflow/issues/333)**
    - Update criterion, add html_reports feature as suggested by criterion ([`4c811e4`](https://github.com/hydro-project/hydroflow/commit/4c811e42a0a5064664860d70e791586c3b001656))
 * **[#34](https://github.com/hydro-project/hydroflow/issues/34)**
    - Add more primitives to construct more complex graphs ([`1b5dcb5`](https://github.com/hydro-project/hydroflow/commit/1b5dcb5953221e0224f3fac9880650f0715915a8))
 * **[#344](https://github.com/hydro-project/hydroflow/issues/344)**
    - Text crdt example: RGA ([`ce071d4`](https://github.com/hydro-project/hydroflow/commit/ce071d4c14aec1ad513ff9a7febc6ebb8784083a))
 * **[#350](https://github.com/hydro-project/hydroflow/issues/350)**
    - Fix `run_tick()` semantics, fix `unique`'s `'static` ([`d8d833c`](https://github.com/hydro-project/hydroflow/commit/d8d833c3b98c7e5f1c664e0731a670cfc5669b32))
 * **[#358](https://github.com/hydro-project/hydroflow/issues/358)**
    - Symmetric hash join improvements ([`22704cd`](https://github.com/hydro-project/hydroflow/commit/22704cd227b9a16b015c9be85d2f6c00d0f43b48))
 * **[#363](https://github.com/hydro-project/hydroflow/issues/363)**
    - Document surface syntax `context` object, cleanup internal usage ([`c259bea`](https://github.com/hydro-project/hydroflow/commit/c259beabb69f22e8e0cc9cd89ceffd0f416a11d2))
 * **[#369](https://github.com/hydro-project/hydroflow/issues/369)**
    - Update trybuild stderr output for new nightly ([`51e5e92`](https://github.com/hydro-project/hydroflow/commit/51e5e92189de6eac282835a831e9bacbfda46b5c))
 * **[#37](https://github.com/hydro-project/hydroflow/issues/37)**
    - Add an echo server test ([`03fa27b`](https://github.com/hydro-project/hydroflow/commit/03fa27ba0a4c10ac9fa8fda1f021a839f5dcb044))
 * **[#372](https://github.com/hydro-project/hydroflow/issues/372)**
    - Provide `&mut Context` alongside handoffs to subgraphs, surface syntax ([`efd6733`](https://github.com/hydro-project/hydroflow/commit/efd673312ec7e3a78da836b63db9df9ae366eb18))
 * **[#374](https://github.com/hydro-project/hydroflow/issues/374)**
    - Support Dedalus rules that send results to the next tick ([`5f58f31`](https://github.com/hydro-project/hydroflow/commit/5f58f3168dbfe9d59e6b543407b6c0defd3a0b44))
 * **[#376](https://github.com/hydro-project/hydroflow/issues/376)**
    - Joins in Dedalus should only have the 'tick lifetime ([`962d903`](https://github.com/hydro-project/hydroflow/commit/962d9038490f32b1dccdd09066720c2d6ef86841))
 * **[#38](https://github.com/hydro-project/hydroflow/issues/38)**
    - Iterate on TCP connections ([`88be54b`](https://github.com/hydro-project/hydroflow/commit/88be54b72703fe7bbc84d9bf5ef9f4135d5a0865))
 * **[#381](https://github.com/hydro-project/hydroflow/issues/381)**
    - Fix `run_async()` not yielding with replay (stateful) operators ([`546b9e0`](https://github.com/hydro-project/hydroflow/commit/546b9e06f499d7f38bd91eb45b9031a8d7ea08de))
 * **[#382](https://github.com/hydro-project/hydroflow/issues/382)**
    - Add `anti_join` operator ([`54bcbaa`](https://github.com/hydro-project/hydroflow/commit/54bcbaa85ccf943ae11002f092cb7659fdc7fe59))
 * **[#383](https://github.com/hydro-project/hydroflow/issues/383)**
    - Allow alias name assignment without any arrow in surface syntax, closes #266 ([`9d17b4d`](https://github.com/hydro-project/hydroflow/commit/9d17b4d5da37efcde633a87cf489541cb5371555))
 * **[#388](https://github.com/hydro-project/hydroflow/issues/388)**
    - Add support for negated relations to Dedalus ([`dc870e8`](https://github.com/hydro-project/hydroflow/commit/dc870e8c2775c352444d47ca063ff561fffda078))
 * **[#39](https://github.com/hydro-project/hydroflow/issues/39)**
    - Clean up the TCP stuff a bit more ([`760efed`](https://github.com/hydro-project/hydroflow/commit/760efed339e3d5a98d403a99602615f4f292d3ac))
 * **[#397](https://github.com/hydro-project/hydroflow/issues/397)**
    - Add basic support for connecting services with Unix/TCP sockets ([`dbdad61`](https://github.com/hydro-project/hydroflow/commit/dbdad61d43412a44449495b4204e37d5d128c12c))
 * **[#40](https://github.com/hydro-project/hydroflow/issues/40)**
    - Switch from a bespoke format -> bincode ([`b57b709`](https://github.com/hydro-project/hydroflow/commit/b57b70999014c16e99eae7d9189e9d013c0ea462))
 * **[#403](https://github.com/hydro-project/hydroflow/issues/403)**
    - Implement simple aggregations for Dedalus ([`f80754e`](https://github.com/hydro-project/hydroflow/commit/f80754e476d979e271ffa30cda1de7fc24c5ccde))
 * **[#409](https://github.com/hydro-project/hydroflow/issues/409)**
    - Drop unnecessary merge() and tee() in CLI examples ([`74979ca`](https://github.com/hydro-project/hydroflow/commit/74979cadd93fd7f9724c56b526675036ee912839))
 * **[#411](https://github.com/hydro-project/hydroflow/issues/411)**
    - Fix non-unix (windows) build referencing unix sockets ([`5dac7e4`](https://github.com/hydro-project/hydroflow/commit/5dac7e4fcd2022c4fb9538d55f9a793139b98c6f))
 * **[#412](https://github.com/hydro-project/hydroflow/issues/412)**
    - Add monotonicity properties to operators (currently unused) ([`9ead3f7`](https://github.com/hydro-project/hydroflow/commit/9ead3f7c654f8fb9fce7d8f53e56b0825c3b07b5))
 * **[#413](https://github.com/hydro-project/hydroflow/issues/413)**
    - Implement async rules in Dedalus by deferring to external sender ([`3d46bde`](https://github.com/hydro-project/hydroflow/commit/3d46bde98d49f07cdff3ea81a7f4d23ffd41cc2e))
 * **[#417](https://github.com/hydro-project/hydroflow/issues/417)**
    - Add API for defining custom services in deployment ([`2fb8871`](https://github.com/hydro-project/hydroflow/commit/2fb88710603948479580aea58f894ab3929280c8))
 * **[#420](https://github.com/hydro-project/hydroflow/issues/420)**
    - Update clap ([`4be709f`](https://github.com/hydro-project/hydroflow/commit/4be709f03acd854d27e551638e31af7ce5b26c0b))
 * **[#43](https://github.com/hydro-project/hydroflow/issues/43)**
    - Rewrite echo server using the surface API ([`4ec58f4`](https://github.com/hydro-project/hydroflow/commit/4ec58f4e0b1f92b42e8a79fd268e90688ae0b1b3))
 * **[#431](https://github.com/hydro-project/hydroflow/issues/431)**
    - Make `unique()` streaming and dedup Dedalus facts ([`68f9bde`](https://github.com/hydro-project/hydroflow/commit/68f9bde464122c41fab3a75897137d46be3bee38))
 * **[#435](https://github.com/hydro-project/hydroflow/issues/435)**
    - Vector clock example ([`4853c0f`](https://github.com/hydro-project/hydroflow/commit/4853c0f7d5728e53ec6a2a0220950c58971de090))
 * **[#437](https://github.com/hydro-project/hydroflow/issues/437)**
    - Extract common logic for establishing CLI-configured connections ([`44cce72`](https://github.com/hydro-project/hydroflow/commit/44cce727b4363d1b6e7f73d72e0a3bec7b6ace53))
 * **[#442](https://github.com/hydro-project/hydroflow/issues/442)**
    - Require users to specify Hydroflow pipelines at the edges of a Dedalus program ([`b107c47`](https://github.com/hydro-project/hydroflow/commit/b107c476a5a817516a5a756e4c6ca6084a78e251))
 * **[#443](https://github.com/hydro-project/hydroflow/issues/443)**
    - Add `.async` Dedalus directive to specify pipeline for inter-Dedalus messaging ([`748242c`](https://github.com/hydro-project/hydroflow/commit/748242c950edde42944c8b6ab9ebca3409406150))
 * **[#444](https://github.com/hydro-project/hydroflow/issues/444)**
    - Add snapshot testing of graph visualizations (mermaid and dot) ([`58a2438`](https://github.com/hydro-project/hydroflow/commit/58a24387c001cbda78ad87c7c2d0c2e2502b3099))
 * **[#445](https://github.com/hydro-project/hydroflow/issues/445)**
    - Add `demux` operator to Hydro CLI to map node IDs to connections ([`886d00f`](https://github.com/hydro-project/hydroflow/commit/886d00f6694ba926c9e1ff184acb31a5d60cee23))
 * **[#448 1/2](https://github.com/hydro-project/hydroflow/issues/448 1/2)**
    - Avoid spinning on internal state replay, fix #380 ([`742ca19`](https://github.com/hydro-project/hydroflow/commit/742ca1962a46db015ef83a2bb18565862626b2a5))
 * **[#448 2/2](https://github.com/hydro-project/hydroflow/issues/448 2/2)**
    - Add no-hang tests ([`28cf07b`](https://github.com/hydro-project/hydroflow/commit/28cf07ba94ca4c3c45aed664f029bfd8fa4f503a))
 * **[#452](https://github.com/hydro-project/hydroflow/issues/452)**
    - Build CLI wheels in CI and minimize CLI dependencies ([`3e33d0c`](https://github.com/hydro-project/hydroflow/commit/3e33d0cf6b068f0567e55462732598f8a4e2da6a))
 * **[#459](https://github.com/hydro-project/hydroflow/issues/459)**
    - Fix coloring (pull vs push) error in serdegraph, recompute colors rather than serializing ([`86d5623`](https://github.com/hydro-project/hydroflow/commit/86d562316a99b0095d32e9a8e5218432396febbb))
 * **[#46](https://github.com/hydro-project/hydroflow/issues/46)**
    - Chat example ([`89fc4cc`](https://github.com/hydro-project/hydroflow/commit/89fc4cce133f4539e44dde12b919a0019ba6e925))
 * **[#460](https://github.com/hydro-project/hydroflow/issues/460)**
    - Allow specifying args to launch `HydroflowCrate` with ([`3575fd3`](https://github.com/hydro-project/hydroflow/commit/3575fd3dd2b4aa98361cc4f723d590eff4794f5f))
 * **[#461](https://github.com/hydro-project/hydroflow/issues/461)**
    - Support networking topologies that mix local and cloud through SSH tunneling ([`0ec6d88`](https://github.com/hydro-project/hydroflow/commit/0ec6d889469331a212c04f9568136f770f0c973d))
 * **[#465](https://github.com/hydro-project/hydroflow/issues/465)**
    - Add generic arg to `identity()`, add tests, close #392 ([`09dd190`](https://github.com/hydro-project/hydroflow/commit/09dd19042cf8d1c9c3d6456cfb0ce33e7117e9af))
 * **[#466](https://github.com/hydro-project/hydroflow/issues/466)**
    - Add APIs for sending data to a Hydroflow service from Python ([`c2203a1`](https://github.com/hydro-project/hydroflow/commit/c2203a15f0144308365af227f3ca044ae6a7954b))
 * **[#468](https://github.com/hydro-project/hydroflow/issues/468)**
    - Add scalar `persist()` operator, #438 ([`688026b`](https://github.com/hydro-project/hydroflow/commit/688026b29490936906eb77314466eb85f95dbab3))
 * **[#469](https://github.com/hydro-project/hydroflow/issues/469)**
    - Automatically clone intermediate values when used multiple times in Dedalus ([`26a1e55`](https://github.com/hydro-project/hydroflow/commit/26a1e557ac175406d5ccd73aabd3c10a00712f96))
 * **[#470](https://github.com/hydro-project/hydroflow/issues/470)**
    - Dedalus support for count, sum, choose, and comments. ([`e28c8d6`](https://github.com/hydro-project/hydroflow/commit/e28c8d694ced16212ece5bef0fe485853b210096))
 * **[#471](https://github.com/hydro-project/hydroflow/issues/471)**
    - Add buffer operator ([`119ba93`](https://github.com/hydro-project/hydroflow/commit/119ba9365c775b3d2a3d89d00460a4af5f9d2225))
 * **[#472](https://github.com/hydro-project/hydroflow/issues/472)**
    - Some clippy/windows fixes for hydro cli code ([`bbf7b50`](https://github.com/hydro-project/hydroflow/commit/bbf7b506463d7fceb5d87005c7cb270a2b0521df))
 * **[#475](https://github.com/hydro-project/hydroflow/issues/475)**
    - Use prettyplease to prettify hydroflow graph output ([`323279a`](https://github.com/hydro-project/hydroflow/commit/323279ad2597b75119b5cb7979702c41fd7e6477))
 * **[#477](https://github.com/hydro-project/hydroflow/issues/477)**
    - Properly handle interrupts and fix non-flushing demux ([`00ea017`](https://github.com/hydro-project/hydroflow/commit/00ea017e40b796e7561979efa0921658dfe072fd))
 * **[#479](https://github.com/hydro-project/hydroflow/issues/479)**
    - Allow custom ports to be used as sinks ([`8da15b7`](https://github.com/hydro-project/hydroflow/commit/8da15b7cbd8bdbf960d3ed58b69f98538ccacd2c))
 * **[#48](https://github.com/hydro-project/hydroflow/issues/48)**
    - Add partition operator to surface API ([`480f203`](https://github.com/hydro-project/hydroflow/commit/480f203898652ed33018da8a6f6e42414b60a5e8))
 * **[#484](https://github.com/hydro-project/hydroflow/issues/484)**
    - Add merge API to CLI to have multiple sources for one sink ([`e09b567`](https://github.com/hydro-project/hydroflow/commit/e09b5670795292f66a004f41314c3c4aa7a24eeb))
 * **[#487](https://github.com/hydro-project/hydroflow/issues/487)**
    - Rename `Hydroflow::serde_graph()` to `Hydroflow::meta_graph()` to prep `SerdeGraph` removal ([`0ad8512`](https://github.com/hydro-project/hydroflow/commit/0ad8512398c62150c135fa4ef0b8f278f8870c61))
 * **[#492](https://github.com/hydro-project/hydroflow/issues/492)**
    - Add API to gracefully shutdown services ([`eda517a`](https://github.com/hydro-project/hydroflow/commit/eda517a3435093830135a9f0384bfae1de5c853e))
 * **[#493](https://github.com/hydro-project/hydroflow/issues/493)**
    - Add `source_interval` op based on `tokio::time::Interval`, close #361 ([`488c001`](https://github.com/hydro-project/hydroflow/commit/488c001bb7a042d1eda4df24d93ca3fc3741d359))
 * **[#495](https://github.com/hydro-project/hydroflow/issues/495)**
    - Re-enable a couple compile-fail tests fixed in recent trybuild releases ([`a6c752c`](https://github.com/hydro-project/hydroflow/commit/a6c752c1bee62d5685687755c436bc055c929a49))
 * **[#497](https://github.com/hydro-project/hydroflow/issues/497)**
    - Add `source_json` operator, use in `two_pc` ([`c5933a5`](https://github.com/hydro-project/hydroflow/commit/c5933a549703b1d7f88d4f5801523864c263069e))
 * **[#499](https://github.com/hydro-project/hydroflow/issues/499)**
    - Dontdrophandoffs ([`b603581`](https://github.com/hydro-project/hydroflow/commit/b603581b83423e161ccac53607022d6e4857fa71))
 * **[#500](https://github.com/hydro-project/hydroflow/issues/500)**
    - Add support for arithmetic expressions on LHS of Dedalus rules ([`82db672`](https://github.com/hydro-project/hydroflow/commit/82db6726ebb3c35cc2e67f313f758cef0e980c53))
 * **[#501](https://github.com/hydro-project/hydroflow/issues/501)**
    - Preserve serialize diagnostics for hydroflow graph, stop emitting expected warnings in tests ([`0c810e5`](https://github.com/hydro-project/hydroflow/commit/0c810e5fdd3445923c0c7afbe651f2b4a72c115e))
 * **[#502](https://github.com/hydro-project/hydroflow/issues/502)**
    - Implement `less_than` magic relation in Dedalus ([`197070d`](https://github.com/hydro-project/hydroflow/commit/197070d2badcd854a9603c642f347fda466d2211))
 * **[#505](https://github.com/hydro-project/hydroflow/issues/505)**
    - Let Rust infer the integer type of Dedalus literals and fix aggregation lifetimes ([`2146c05`](https://github.com/hydro-project/hydroflow/commit/2146c0597ebd4b7adb40170be8c0200ae3f93e99))
 * **[#509](https://github.com/hydro-project/hydroflow/issues/509)**
    - Even faster groupby ([`af304aa`](https://github.com/hydro-project/hydroflow/commit/af304aa7ed35e6d5d7ed0936e3827de2b40e1ddb))
 * **[#511](https://github.com/hydro-project/hydroflow/issues/511)**
    - Fix multi-line code blocks, mermaid styling ([`2e0b4dc`](https://github.com/hydro-project/hydroflow/commit/2e0b4dc17820bf08772022f2b8b45c1aa6971949))
 * **[#512](https://github.com/hydro-project/hydroflow/issues/512)**
    - Display varnames in dot output, fix #385 ([`8746c3c`](https://github.com/hydro-project/hydroflow/commit/8746c3c9bd32ba163fadc6789e95d5a3c69b9eb9))
 * **[#516](https://github.com/hydro-project/hydroflow/issues/516)**
    - Update Rust Sitter to fix Dedalus parser on WASM ([`5a4f408`](https://github.com/hydro-project/hydroflow/commit/5a4f4084a357d329b3bf228f1e4113898917d90c))
 * **[#518](https://github.com/hydro-project/hydroflow/issues/518)**
    - Attach spans to generated Hydroflow code in Dedalus ([`f00d865`](https://github.com/hydro-project/hydroflow/commit/f00d8655aa4404ddcc812e0decf8c1e48e62b0fd))
 * **[#520](https://github.com/hydro-project/hydroflow/issues/520)**
    - Implement `BottomRepr`, add `Merge::merge_owned` helper ([`6382503`](https://github.com/hydro-project/hydroflow/commit/63825036e0f9a4a9d1e7323f5ceb30e4d0a9a1f2))
 * **[#521](https://github.com/hydro-project/hydroflow/issues/521)**
    - Last write wins ([`acca988`](https://github.com/hydro-project/hydroflow/commit/acca988268abf858456674a7b0f85b135c6a4739))
 * **[#522](https://github.com/hydro-project/hydroflow/issues/522)**
    - Don't copy all elements of row being filtered in Dedalus ([`4fa677d`](https://github.com/hydro-project/hydroflow/commit/4fa677d0316d441eedad7660a7f8490dbbecfa61))
 * **[#523](https://github.com/hydro-project/hydroflow/issues/523)**
    - Lattice join ([`f6af455`](https://github.com/hydro-project/hydroflow/commit/f6af455a2a8e49046d70546fbc6f8c69f8c8e3b2))
 * **[#524](https://github.com/hydro-project/hydroflow/issues/524)**
    - Fix lattice join cases ([`90c456e`](https://github.com/hydro-project/hydroflow/commit/90c456ec00bae11bfe0cd71c64e2c0a065bb70a8))
 * **[#526](https://github.com/hydro-project/hydroflow/issues/526)**
    - Add repeat_fn op ([`9620b91`](https://github.com/hydro-project/hydroflow/commit/9620b912fd09bcf92ee29944314083de1a0e6c62))
 * **[#529](https://github.com/hydro-project/hydroflow/issues/529)**
    - `deserialize_bytes` take `AsRef<[u8]>` instead of `BytesMut` ([`922fe60`](https://github.com/hydro-project/hydroflow/commit/922fe60aea8d3f48382324ccf4bf3b43b068b91c))
 * **[#530](https://github.com/hydro-project/hydroflow/issues/530)**
    - Add specialized `lattice_merge::<MyLatRepr>()` operator ([`1a9b652`](https://github.com/hydro-project/hydroflow/commit/1a9b65286e41013178adfba11bcdde4e3b5c44d8))
 * **[#536](https://github.com/hydro-project/hydroflow/issues/536)**
    - Kvs benchmark ([`6b2a12c`](https://github.com/hydro-project/hydroflow/commit/6b2a12c29b6610b910ea13914dbcce2d480490df))
 * **[#538](https://github.com/hydro-project/hydroflow/issues/538)**
    - Source_stream_serde returns Result<T> instead of T ([`7c38361`](https://github.com/hydro-project/hydroflow/commit/7c383611eca4bd80a0d4ee40ae60dcf903939ef5))
 * **[#540](https://github.com/hydro-project/hydroflow/issues/540)**
    - Support arithmetic expressions and literals in Datalog predicates ([`fc2eac3`](https://github.com/hydro-project/hydroflow/commit/fc2eac3478c1f32c8fb22b2eec63316b84203fba))
 * **[#541](https://github.com/hydro-project/hydroflow/issues/541)**
    - Start accepting connections in the background of CLI initialization to avoid deadlocks ([`681a6ba`](https://github.com/hydro-project/hydroflow/commit/681a6baef73de8a67e140526ede4b36e239976f0))
 * **[#543](https://github.com/hydro-project/hydroflow/issues/543)**
    - Add `.persist` annotation to opt into more efficient Dedalus persistence ([`95c7190`](https://github.com/hydro-project/hydroflow/commit/95c7190b851cd82a88e7c7f062e617774238be1e))
 * **[#547](https://github.com/hydro-project/hydroflow/issues/547)**
    - Add transform to remove extra `merge()`s and `tee()`s ([`838ac2a`](https://github.com/hydro-project/hydroflow/commit/838ac2a4d9a2e3ea1a4cdb5f8702c8d2b1eb3e5e))
 * **[#548 #550](https://github.com/hydro-project/hydroflow/issues/548 #550)**
    - Add `test_persist_replay_join` test ([`fe006d5`](https://github.com/hydro-project/hydroflow/commit/fe006d5eabc4eca8c5898e661dccb711b9fd9fd7))
 * **[#549](https://github.com/hydro-project/hydroflow/issues/549)**
    - Support `_` as a wildcard variable in Dedalus rules that is not joined on ([`633cc4f`](https://github.com/hydro-project/hydroflow/commit/633cc4f8f9d7818b5c5d64d1d7d80a7d1a51d7bf))
 * **[#551](https://github.com/hydro-project/hydroflow/issues/551)**
    - Add remainder operator to Dedalus expressions ([`f24b746`](https://github.com/hydro-project/hydroflow/commit/f24b7469bdeaa023197eba42fc3e6e1e3343bd49))
 * **[#555](https://github.com/hydro-project/hydroflow/issues/555)**
    - Antijoin uses FxHash instead of SipHash ([`55fa0a2`](https://github.com/hydro-project/hydroflow/commit/55fa0a2a733a482400e01edd495ef429a54ac555))
 * **[#556](https://github.com/hydro-project/hydroflow/issues/556)**
    - Unique uses FxHash instead of SipHash ([`4323d47`](https://github.com/hydro-project/hydroflow/commit/4323d47efc495940cc4bf41f647e4e187bf1305b))
 * **[#558](https://github.com/hydro-project/hydroflow/issues/558)**
    - Be more careful about the semantics of `count` and wildcard columns ([`ad4a665`](https://github.com/hydro-project/hydroflow/commit/ad4a66536ef6f1ea29535fa7162a4bbf129db999))
 * **[#559](https://github.com/hydro-project/hydroflow/issues/559)**
    - Add optional multiset join operator ([`c70644d`](https://github.com/hydro-project/hydroflow/commit/c70644ddb784449b55a84278cb1bf8cc38557d82))
 * **[#56](https://github.com/hydro-project/hydroflow/issues/56)**
    - First attempt at an exchange-like operator ([`b09dec1`](https://github.com/hydro-project/hydroflow/commit/b09dec11aa4e7df535ad4d4fd4be958821b9a31d))
 * **[#566](https://github.com/hydro-project/hydroflow/issues/566)**
    - Only filter out duplicate elements in one place for persisted relations ([`a37a511`](https://github.com/hydro-project/hydroflow/commit/a37a511c37fd362044b563268e95fdf152700acf))
 * **[#567](https://github.com/hydro-project/hydroflow/issues/567)**
    - Make aggregations of persisted relations incremental ([`6670256`](https://github.com/hydro-project/hydroflow/commit/6670256a30ac656adf46ced3ea385558357122e9))
 * **[#568](https://github.com/hydro-project/hydroflow/issues/568)**
    - Don't reify persists if the target relation is already a persisted one ([`61abfc1`](https://github.com/hydro-project/hydroflow/commit/61abfc18b5e52fc5a8ab63cbb920f670c52b7c0c))
 * **[#571](https://github.com/hydro-project/hydroflow/issues/571)**
    - Add multiplication operator and test behavior of aggregations with grouping expressions ([`b3e790c`](https://github.com/hydro-project/hydroflow/commit/b3e790c5d6cb5e0d420f13fe9ead7e6c43e527d4))
 * **[#576](https://github.com/hydro-project/hydroflow/issues/576)**
    - Add classic counter CRDT benchmark to compare against ([`2f3bf04`](https://github.com/hydro-project/hydroflow/commit/2f3bf04ab33768b04d44f3f58907f958d4cd8dc8))
 * **[#578](https://github.com/hydro-project/hydroflow/issues/578)**
    - Fix `run_async()` spinning ([`9d44e17`](https://github.com/hydro-project/hydroflow/commit/9d44e17fdd16388ab0b2b213e886e5c665bd275f))
 * **[#584](https://github.com/hydro-project/hydroflow/issues/584)**
    - Fix windows build ([`5aa96c4`](https://github.com/hydro-project/hydroflow/commit/5aa96c451ba69ff2beed41528b8c847b75c45ea7))
 * **[#586](https://github.com/hydro-project/hydroflow/issues/586)**
    - Bump pinned nightly and fix build failures on latest nightly ([`84a831e`](https://github.com/hydro-project/hydroflow/commit/84a831efca6eddac20bac140c9c67bf4ab2d5cf8))
 * **[#589](https://github.com/hydro-project/hydroflow/issues/589)**
    - Add smallvec for multiset join ([`472be85`](https://github.com/hydro-project/hydroflow/commit/472be858466aec4cea1f30a75a8ab71f9af5443b))
 * **[#590](https://github.com/hydro-project/hydroflow/issues/590)**
    - Add async repeat_repeat iter test ([`9b72b30`](https://github.com/hydro-project/hydroflow/commit/9b72b3036baae0a6698eeabc2101a130476c4c59))
 * **[#591](https://github.com/hydro-project/hydroflow/issues/591)**
    - Add `keyed_reduce()` operator, make `group_by()` an alias of renamed `keyed_fold()` operator ([`71c72ff`](https://github.com/hydro-project/hydroflow/commit/71c72ffa6d669a098e634a7c6c0fc153c0e596fa))
 * **[#593](https://github.com/hydro-project/hydroflow/issues/593)**
    - Fix `recv_events[_async]()` not returning when already-scheduled subgraph is externally triggered ([`32e76ab`](https://github.com/hydro-project/hydroflow/commit/32e76abff484d87f85a9a0f689898beec78513a8))
 * **[#597](https://github.com/hydro-project/hydroflow/issues/597)**
    - Add Hello World hydroflow example ([`44c83f1`](https://github.com/hydro-project/hydroflow/commit/44c83f198ef0d3863d8eb9c438c3f2a89ccdfc2d))
 * **[#598](https://github.com/hydro-project/hydroflow/issues/598)**
    - Add `index()` operator for getting the index of the current group ([`6f959b6`](https://github.com/hydro-project/hydroflow/commit/6f959b64f0cf494c23f9ec8bc107a23e006aeacf))
 * **[#6](https://github.com/hydro-project/hydroflow/issues/6)**
    - Introduce an "Input" for ingressing data ([`56cec43`](https://github.com/hydro-project/hydroflow/commit/56cec4343a9aaba80c688168d2407634c0211daf))
 * **[#60](https://github.com/hydro-project/hydroflow/issues/60)**
    - Exchange abstraction ([`41206ee`](https://github.com/hydro-project/hydroflow/commit/41206ee46d384749ecb0145cbd6b913d8efed3ef))
 * **[#602](https://github.com/hydro-project/hydroflow/issues/602)**
    - Remove `std`-ified `once_cell` crate, remove dead bespoke `Once` channel code ([`753f38c`](https://github.com/hydro-project/hydroflow/commit/753f38c9c4ee46cf315d68ed4d4978275f6a6b3a))
 * **[#605](https://github.com/hydro-project/hydroflow/issues/605)**
    - Add batch limit to batch and fix scheduling poor behavior ([`f831f9d`](https://github.com/hydro-project/hydroflow/commit/f831f9d8518bbc55f1c5e7b78e9b3ca189b2adfb))
 * **[#607](https://github.com/hydro-project/hydroflow/issues/607)**
    - Don't drop updated_keys in lattice join, drain it and reuse it ([`b06ef93`](https://github.com/hydro-project/hydroflow/commit/b06ef93a35ac7591bd2314bf8ca6b2e1bb22ff20))
 * **[#608](https://github.com/hydro-project/hydroflow/issues/608)**
    - Fix deserialization leak, add kvs back into docker build ([`a8162db`](https://github.com/hydro-project/hydroflow/commit/a8162dbba48b865b5680fafcaa49a64c6794f398))
 * **[#61](https://github.com/hydro-project/hydroflow/issues/61)**
    - Add a `broadcast` operator ([`8c7bf01`](https://github.com/hydro-project/hydroflow/commit/8c7bf017ad019f104d47fc46181231db3136a3e6))
 * **[#617](https://github.com/hydro-project/hydroflow/issues/617)**
    - Update `Cargo.toml`s for publishing ([`a78ff9a`](https://github.com/hydro-project/hydroflow/commit/a78ff9aace6771787c2b72aad83be6ad8d49a828))
 * **[#618](https://github.com/hydro-project/hydroflow/issues/618)**
    - Add static declarations to Dedalus ([`3a77e62`](https://github.com/hydro-project/hydroflow/commit/3a77e62e6006b846db9055385bd76c81e08f2f15))
 * **[#63](https://github.com/hydro-project/hydroflow/issues/63)**
    - Get the distributed covid app running again ([`b5420d2`](https://github.com/hydro-project/hydroflow/commit/b5420d213df6d6705c48501c6113992b854ba94c))
 * **[#64](https://github.com/hydro-project/hydroflow/issues/64)**
    - Enable the derive feature on hydroflow's serde ([`fd132c1`](https://github.com/hydro-project/hydroflow/commit/fd132c1475a25469828c2216ce99c7290e3e0818))
 * **[#8](https://github.com/hydro-project/hydroflow/issues/8)**
    - Make Input threadsafe (optionally) ([`9a9bd51`](https://github.com/hydro-project/hydroflow/commit/9a9bd5130025e6ab514bf0e32c30ea918b501821))
 * **[#80](https://github.com/hydro-project/hydroflow/issues/80)**
    - Batch join ([`babb331`](https://github.com/hydro-project/hydroflow/commit/babb33175ce06668070468d5ca10165e2368ee40))
 * **[#84](https://github.com/hydro-project/hydroflow/issues/84)**
    - Add three_clique hydroflow example ([`c1884ee`](https://github.com/hydro-project/hydroflow/commit/c1884eebb1ed63e6202e2d1a422905c6e1529e44))
 * **[#86](https://github.com/hydro-project/hydroflow/issues/86)**
    - Add Two-Phase Commit example ([`54bafa4`](https://github.com/hydro-project/hydroflow/commit/54bafa4c2ce8c861ca9b176ac53d329acc3185c6))
 * **[#89](https://github.com/hydro-project/hydroflow/issues/89)**
    - Rework batcher to be lattice-based ([`82b62d5`](https://github.com/hydro-project/hydroflow/commit/82b62d581d48d6e4df8d75373fd1e2e67ce6df69))
 * **[#91](https://github.com/hydro-project/hydroflow/issues/91)**
    - Rewrite KVS in dataflow ([`a4d94a4`](https://github.com/hydro-project/hydroflow/commit/a4d94a4bad2483eb11bbe86bc9e7c05f93a30bc8))
 * **[#95](https://github.com/hydro-project/hydroflow/issues/95)**
    - Fix some Compare implementations ([`c5045f5`](https://github.com/hydro-project/hydroflow/commit/c5045f572a74f686347607892d368013f6632d7e))
 * **[#98](https://github.com/hydro-project/hydroflow/issues/98)**
    - Implement reads in KVS ([`68906c4`](https://github.com/hydro-project/hydroflow/commit/68906c442bbea079648a517a040f8a2ed8ae508f))
 * **Uncategorized**
    - Setup release workflow ([`108d0e9`](https://github.com/hydro-project/hydroflow/commit/108d0e933a08b183c4dadf8c3499e4946696e263))
    - Fix expected output for test ([`f6b2c47`](https://github.com/hydro-project/hydroflow/commit/f6b2c47e9258cada5738fc428f734a654a680e54))
    - Use clear rather than default for join state #562 ([`c4f3f97`](https://github.com/hydro-project/hydroflow/commit/c4f3f97bab8a1cb5d3453290f567798b4bc4b60d))
    - Add `dest_file(filename, append)` operator ([`7807687`](https://github.com/hydro-project/hydroflow/commit/7807687fa9ba52c67fb5eb286aece37fab82a67b))
    - Ignore `surface_dest_sink_badsink` compile-fail test ([`2106dfd`](https://github.com/hydro-project/hydroflow/commit/2106dfdfe5726320e49921aa25353c43c3fd97f5))
    - Improve datalog diagnostic robustness ([`0b3e085`](https://github.com/hydro-project/hydroflow/commit/0b3e08521131989dfaee821c060a931771936f80))
    - Use `HydroflowGraph` for graph writing, delete `SerdeGraph` ([`d1ef14e`](https://github.com/hydro-project/hydroflow/commit/d1ef14ee459c51d5a2dd9e7ea03050772e14178c))
    - Serialize `HydroflowGraph` instead of `SerdeGraph` ([`ae205c6`](https://github.com/hydro-project/hydroflow/commit/ae205c69538fab9eeedd8fa460b8eef295d26bc2))
    - Turn on WASM tests ([`d29bc63`](https://github.com/hydro-project/hydroflow/commit/d29bc6382d0d4d97931af10d90161552878903c7))
    - Additional cleanups for PR #407 ([`fff4d0a`](https://github.com/hydro-project/hydroflow/commit/fff4d0a708c15f2609c0db9122e0b19abcaaa779))
    - Comment out inconsistent compile-fail test ([`215f74a`](https://github.com/hydro-project/hydroflow/commit/215f74abead32f1411aa6b36e1b4441a17154090))
    - Add more generics compile-fail tests ([`644980b`](https://github.com/hydro-project/hydroflow/commit/644980b3797169e8c32bd93fa894c221083bec63))
    - Detect name cycles sooner, memoize resolution, better error messages ([`00d5f63`](https://github.com/hydro-project/hydroflow/commit/00d5f63a2b672648831d98d65eae4d4e09bf9ed3))
    - Placeholder commit for adding more tests ([`4113ec5`](https://github.com/hydro-project/hydroflow/commit/4113ec563b5f6651b7ef2391b6a0f2332bba25de))
    - Update examples to use forward name references ([`398cff6`](https://github.com/hydro-project/hydroflow/commit/398cff6b9b27ec8091d90f8f3e844d2574d9429f))
    - Implement forward name references in surface syntax, closes #158 ([`8cc479e`](https://github.com/hydro-project/hydroflow/commit/8cc479ea99fd2a58751fc24f8b46d60e8594d24a))
    - Improve parsing handling/error messages ([`bfe9a90`](https://github.com/hydro-project/hydroflow/commit/bfe9a906d37f9f91ccea3fe7e6414ec62c695c78))
    - Add surface syntax syntactical error tests ([`b00c909`](https://github.com/hydro-project/hydroflow/commit/b00c90986e7568cbe077056375836cecff7dae92))
    - :<'static>` now replays #143 #364 ([`62fcfb1`](https://github.com/hydro-project/hydroflow/commit/62fcfb157eaaaabedfeb5c77b2a6df89ee1a6852))
    - :<'static>` now replays #143 #364 ([`bc3d12f`](https://github.com/hydro-project/hydroflow/commit/bc3d12f563dab96f4751ec21cd20b193eea95456))
    - :<'static>` now replays #143 #364 ([`a2078f7`](https://github.com/hydro-project/hydroflow/commit/a2078f7056a54d20f91e2e0f9a7617dc6ef1f627))
    - `repeat_iter` now repeats via self-scheduling #143 #364 ([`e5f46df`](https://github.com/hydro-project/hydroflow/commit/e5f46df99299771cb52127ff07bfbc26a46cb569))
    - Only receive external events at the start of a tick, before stratum 0 ([`eb7ff56`](https://github.com/hydro-project/hydroflow/commit/eb7ff56b53d275499a6cbb2d22b70c860b436c0e))
    - Add persistence lifetimes to `reduce` ([`050cadf`](https://github.com/hydro-project/hydroflow/commit/050cadffaf6c1287e374c83e81ad57cd6ef67ec3))
    - Add persistence lifetimes to `fold` ([`1283da5`](https://github.com/hydro-project/hydroflow/commit/1283da5f1534d6bf0d2e85ab96e4ec514d1bb845))
    - Replace old references to `'epoch` with `'static` ([`8431060`](https://github.com/hydro-project/hydroflow/commit/84310607b6f07fe5c8fdd4877bf288cad1e0b003))
    - Ops specify persistence/type arg counts, handle separately in `partitioned_graph` ([`cdc83b6`](https://github.com/hydro-project/hydroflow/commit/cdc83b68d989d60732c01fb99957762781d161cb))
    - Add generic type arguments for `group_by` when inference fails #272 ([`75f152e`](https://github.com/hydro-project/hydroflow/commit/75f152ef9170982336da0a19dd334b8065975036))
    - Add persistence lifetimes to join #272 ([`47b2941`](https://github.com/hydro-project/hydroflow/commit/47b2941d74704792e5e2a7f30fa088c81c3ab506))
    - Ignore inconsistent compile-fail tests (nix vs windows full paths) ([`91cddfd`](https://github.com/hydro-project/hydroflow/commit/91cddfd3523d0c251e65275845955e59cdfdb071))
    - Ingore `surface_dest_sink_baditem` due to `rustc` inconsistency ([`a44f08c`](https://github.com/hydro-project/hydroflow/commit/a44f08cf396c38d5feb32b0f50eebf64cac372d9))
    - Type guard for `source_iter`, `repeat_iter` #263 ([`496a7a1`](https://github.com/hydro-project/hydroflow/commit/496a7a11629533944064e2e86fd7b0e2026be8cf))
    - Add type guard to `group_by` #263 ([`3fcfb46`](https://github.com/hydro-project/hydroflow/commit/3fcfb464f7b527a7ddc43926a10827c125c2e8e4))
    - Simplify `dest_sink`, add type guards #263 ([`6aa4d41`](https://github.com/hydro-project/hydroflow/commit/6aa4d41cc75825e5ea1c4c8bfe590f02387fcc5e))
    - Add type guard before `Pivot` #263 ([`c215e8c`](https://github.com/hydro-project/hydroflow/commit/c215e8c4523a1e465eafa3320daa34d6cb35aa11))
    - Add type guard to `merge` #263 ([`6db3f60`](https://github.com/hydro-project/hydroflow/commit/6db3f6013a934b3087c8d116e61fbfc293e1baa0))
    - Emit type guards inline, configurable #263 ([`c6510da`](https://github.com/hydro-project/hydroflow/commit/c6510da4b4cb46ec026e3c1c69b5ce29b17c473c))
    - Add very good type guard to `join` op #263 ([`3ee9d33`](https://github.com/hydro-project/hydroflow/commit/3ee9d338c27859b31a057be53ee9251248ca235c))
    - Improve spanning of write context `make_ident(..)` #263 ([`58668bd`](https://github.com/hydro-project/hydroflow/commit/58668bd6ec758ed091b754f8769ed8c243cbde78))
    - Improve `Iterator`/`Pusherator` typeguards by erasing types, using local fns #263 ([`6413fa4`](https://github.com/hydro-project/hydroflow/commit/6413fa417cab0481e3db1adbcaf71525eb866cc9))
    - Add more "badtypes" compile-fail tests for upcoming changes #263 ([`58ef72d`](https://github.com/hydro-project/hydroflow/commit/58ef72d93faf72c1f4468480dbebe683682860b6))
    - Rename `recv_into` -> `collect_ready` ([`32fddfe`](https://github.com/hydro-project/hydroflow/commit/32fddfec46d2d136b4fc399fc0c438f922012487))
    - Remove `dest_asyncwrite`, consolidate using codecs, now in `hydroflow::util::udp/tcp`, fix #216 ([`5418ea4`](https://github.com/hydro-project/hydroflow/commit/5418ea47c7cbe0cf9be755346b0054faeb98d5c1))
    - Add example usage code to `dest_sink`, `dest_asyncwrite`, #216 ([`05c990f`](https://github.com/hydro-project/hydroflow/commit/05c990fcad2bc7ee64b7d58fce11bb126655a359))
    - Rename variadics/tuple_list macros ([`91d37b0`](https://github.com/hydro-project/hydroflow/commit/91d37b022b1cd0ed590765c40ef43244027c8035))
    - Rename pkg `type_list` -> `variadics` ([`50e7361`](https://github.com/hydro-project/hydroflow/commit/50e7361709cd34fd0e1cbf0c9a9f79343ee9c2e2))
    - Update README.md ([`9b29c6f`](https://github.com/hydro-project/hydroflow/commit/9b29c6f343167a7ce1b6245df94db804268cea55))
    - Re-export `serde`, `serde_json` ([`d9541b2`](https://github.com/hydro-project/hydroflow/commit/d9541b2a29c0ec0f3f8642652314caa5c33d3cf9))
    - Disallow overwriting names in surface syntax (preps for #158) ([`7db1357`](https://github.com/hydro-project/hydroflow/commit/7db13575f97deedc2730f7f43bebc1282d9deec9))
    - Allow `clippy::uninlined-format-args` in `.cargo/config.toml` ([`17be5dd`](https://github.com/hydro-project/hydroflow/commit/17be5dd3993ee3239a3fbdb81572923479b0cc3e))
    - Fix for new clippy lifetime lint ([`f5d141d`](https://github.com/hydro-project/hydroflow/commit/f5d141d84bcc99b8bf651344d3f6ec50134dc1ac))
    - Add/update more operator docs ([`43e32ee`](https://github.com/hydro-project/hydroflow/commit/43e32eefa1ae2c6db7389ac023d16fae21b05e34))
    - Separate surface doctests by operator ([`851d97d`](https://github.com/hydro-project/hydroflow/commit/851d97de7ba3435bac98264f4b8679973536486a))
    - Add `hydroflow_macr/build.rs` to autogen operator book docs ([`a5de404`](https://github.com/hydro-project/hydroflow/commit/a5de404cd06c10137f7584d152269327c698a65d))
    - Update dependencies for `cargo audit` ([`2346dc8`](https://github.com/hydro-project/hydroflow/commit/2346dc8e31e97b60f2d33eeed120ac4f28b971a2))
    - Implement named ports in operators ([`879e977`](https://github.com/hydro-project/hydroflow/commit/879e977205f055e9712c2887a275dcdbee49f540))
    - Add parsing of named ports (WIP, compiling) ([`bd8313c`](https://github.com/hydro-project/hydroflow/commit/bd8313cf59a30bb121c07d754099d92c13daa734))
    - Remove Surface API from book ([`af550ca`](https://github.com/hydro-project/hydroflow/commit/af550ca7c787d705b54532540f078b9d3e5d999b))
    - Pick random ports for udp/tcp tests to prevent hanging ([`625e9a0`](https://github.com/hydro-project/hydroflow/commit/625e9a0ded573066409d01e7eb1443f8b4278cae))
    - Remove surface API, fix #224 ([`7b75f5e`](https://github.com/hydro-project/hydroflow/commit/7b75f5eb73046c3fe9f50970e05b4665bc0bf7fc))
    - Use sequential numbers to index tee output ([`ea99408`](https://github.com/hydro-project/hydroflow/commit/ea99408665e4157ba1129a8401ad3eeb850eed84))
    - Implement `inspect()` surface syntax operator, fix #208 ([`7797c6c`](https://github.com/hydro-project/hydroflow/commit/7797c6c4aff07f780069bb9af2b12b8999b33725))
    - Add type guards, better spans to surface syntax codegen ([`09953f7`](https://github.com/hydro-project/hydroflow/commit/09953f73e96fdfd985daf555e01e46f5d54320b0))
    - Fix #201 `run_async()`, wait when ALL strata have no work ([`fa20c12`](https://github.com/hydro-project/hydroflow/commit/fa20c12f10025133a071e46af67f858366e6e0da))
    - Fix surface syntax port ordering bug ([`c241c05`](https://github.com/hydro-project/hydroflow/commit/c241c0580616d81e725e60afeeb7d60b3a47dab8))
    - Add assertiosn to surface tcp test ([`dfc22e8`](https://github.com/hydro-project/hydroflow/commit/dfc22e84e640baa93b76cdf6cf7702684e55ea0a))
    - Remove awkward testing side-effects from surface syntax async tests ([`84d7406`](https://github.com/hydro-project/hydroflow/commit/84d7406032bd2b2495e02edccca09d75d72e02aa))
    - Add UDP networking & test ([`c9529c5`](https://github.com/hydro-project/hydroflow/commit/c9529c5c58473ff8eba836aa838b73e3f94e0b6f))
    - Add `#[allow(clippy::map_identity)]` to some tests ([`1bf8642`](https://github.com/hydro-project/hydroflow/commit/1bf86425afacbdd23b391567330f3a48a518d6d7))
    - Implement and add test for `sink_async` ([`19424cf`](https://github.com/hydro-project/hydroflow/commit/19424cfa02443a44ea022c1558e4a010545df9d6))
    - Rename `send_async` -> `write_async` to match trait names ([`666d14e`](https://github.com/hydro-project/hydroflow/commit/666d14e63ba870f7d1bb9bb7486ff45720c079e6))
    - Add tcp echo example test ([`9362edf`](https://github.com/hydro-project/hydroflow/commit/9362edf1c48d8a47ffa32c07f523ca3b931ef4e2))
    - Remove internal runtime, use tokio::spawn mechanism (requires tokio context) ([`302b213`](https://github.com/hydro-project/hydroflow/commit/302b213c6432c5d16cf517557eec8a876f46085d))
    - Fix handling of empty `merge()`/`tee()`, add tests ([`3a0ab8a`](https://github.com/hydro-project/hydroflow/commit/3a0ab8a51c31f57145fe52c362fb6ab49f8a6370))
    - Add surface diamond tests ([`7ba4321`](https://github.com/hydro-project/hydroflow/commit/7ba432110b859a14649f709453ea0ea6d54709a6))
    - Update `recv_stream` to handle all `Stream`s instead of just `tokio::mpsc::unbounded_channel` ([`8b68c64`](https://github.com/hydro-project/hydroflow/commit/8b68c643b55e9a04f373bded939b512be4ee0d7f))
    - Check output of tests ([`d1f39db`](https://github.com/hydro-project/hydroflow/commit/d1f39db6a65b64dd725778e6ee48eb4487d37455))
    - Remove `#![feature(generic_associated_types)]` b/c stabilization! ([`4786baf`](https://github.com/hydro-project/hydroflow/commit/4786baf69197a7254ffe1aea964238cbed9d755e))
    - Re-enable detection of conflicting surface syntax ports ([`b76d334`](https://github.com/hydro-project/hydroflow/commit/b76d334cf996da1593bc47d797a64d4267013a0a))
    - Use `DiMulGraph` in `flat_to_partitioned.rs` and `PartitionedGraph`, working ([`cdd45fe`](https://github.com/hydro-project/hydroflow/commit/cdd45fe8eeefaa997bc2d38386fb9d33daf47b50))
    - Fix handing of "complex" expressions in recv_stream ([`7c67e2d`](https://github.com/hydro-project/hydroflow/commit/7c67e2ddc435effd7120bcc8ff8a1ab7e034d457))
    - Standardizing pusherators, implement wrap specs ([`f4081f8`](https://github.com/hydro-project/hydroflow/commit/f4081f8ee7cc6f308136c68f8015309106c43a5a))
    - Refactor for foundation of properties iterators ([`a14c439`](https://github.com/hydro-project/hydroflow/commit/a14c439f82f5811299c352c1eb7508f6c18839ce))
    - Run clippy with `--all-targets` ([`76e9b5d`](https://github.com/hydro-project/hydroflow/commit/76e9b5d563293b16b64f9ccaf8f373c92b5d5771))
    - Fix handling of warnings, degenerate merge and tee ([`13c15d7`](https://github.com/hydro-project/hydroflow/commit/13c15d798a5b2f51c58f9812f2e59b47b760153a))
    - Add stratum consolidation as an optimization ([`7f76dba`](https://github.com/hydro-project/hydroflow/commit/7f76dba1512e2e1c33e94c73e223fd30fb94f059))
    - Use `Rc::clone(&output)` for clarity ([`8e7153e`](https://github.com/hydro-project/hydroflow/commit/8e7153e14d0229bf43cc990967fedd6cb1be8adc))
    - Add `send_async(impl AsyncWrite)` surface syntax operator, Hydroflow tokio runtime ([`e5abe91`](https://github.com/hydro-project/hydroflow/commit/e5abe911a428015bf3d4699812530dd8d4e226ab))
    - Add comments, cleanup for PR ([`03531dd`](https://github.com/hydro-project/hydroflow/commit/03531ddcaf173be7b0361dafcdd13936751e69ce))
    - Fix lint errors ([`5b59c79`](https://github.com/hydro-project/hydroflow/commit/5b59c79041400c45b3f1a1b8efe193ce2d3d99d0))
    - Add sort surface syntax operator, test ([`bb7d334`](https://github.com/hydro-project/hydroflow/commit/bb7d3346762d93b0feb5186f85b4f371b8e773b8))
    - Add more tests, fix surface syntax bugs ([`eb62ef1`](https://github.com/hydro-project/hydroflow/commit/eb62ef1a47ec58abcf6a11745667e00d69df6d93))
    - Add stratification tests ([`dbbce89`](https://github.com/hydro-project/hydroflow/commit/dbbce8921b405240b9254d5ce06eef665603bf86))
    - Add test_reduce_sum ([`3b1e487`](https://github.com/hydro-project/hydroflow/commit/3b1e487a76bc177eb40109fa7f3e7db39c5ab0eb))
    - Async_test for surface syntax ([`1132cb5`](https://github.com/hydro-project/hydroflow/commit/1132cb5f521a63f8072b445f0ff4e8d73ea5e96a))
    - Hack for proc_macro_crate issue in examples ([`00db2ea`](https://github.com/hydro-project/hydroflow/commit/00db2ea267bca0d8e416194f0a70d8c377d097fd))
    - Fix difference forgetfulness ([`093eb45`](https://github.com/hydro-project/hydroflow/commit/093eb45b262d17a26ce2f331bf571305c7cc83d7))
    - Add fold() and reduce() surface syntax operators ([`80d4385`](https://github.com/hydro-project/hydroflow/commit/80d4385386dd0818730820f92b77777dee9e85fa))
    - Inter-instance tokio channel communication ([`b256f17`](https://github.com/hydro-project/hydroflow/commit/b256f177c72c408e9df69596b1a0c889946193f0))
    - Stratification WIP 3/4 ([`7557f2d`](https://github.com/hydro-project/hydroflow/commit/7557f2d78737d3b2bba7742bfd4d42c2a8476776))
    - Stratification WIP 2/4 ([`2c39fe2`](https://github.com/hydro-project/hydroflow/commit/2c39fe2053a2c7ae2ea267d9843f9e6db11183d8))
    - Stratification WIP 1/4 ([`553740f`](https://github.com/hydro-project/hydroflow/commit/553740fe87a47e6858c84064c0fcdc0b99e66d43))
    - Include mdbook tests directly ([`152d377`](https://github.com/hydro-project/hydroflow/commit/152d377d5f0c8c67bf2786ea17e54ccac272872a))
    - Add echo server test ([`43a5a8f`](https://github.com/hydro-project/hydroflow/commit/43a5a8f528549067905783116ea75d8454944010))
    - Check operator number of expression arguments ([`20c3eeb`](https://github.com/hydro-project/hydroflow/commit/20c3eeb6e6b653e92277c35a759c320166693404))
    - Rename `seed` -> `recv_iter`, `input` -> `recv_stream` ([`bc27dcf`](https://github.com/hydro-project/hydroflow/commit/bc27dcf82b29fd0cb477e7eb4fc34aa99e0ba9c6))
    - Make parenthesis optional in surface syntax ([`e528c5f`](https://github.com/hydro-project/hydroflow/commit/e528c5f88bddfe7616d1dd62f0a3de8116cf7b45))
    - Remove automatic index incrementing ([`5f5242f`](https://github.com/hydro-project/hydroflow/commit/5f5242f7c2fb2f5b482856b32d99e33dbfd9dc58))
    - Output source code row/col in mermaid instead of slotmap ID ([`7797342`](https://github.com/hydro-project/hydroflow/commit/7797342cfaed6c98ab02f6c9e51a8a6e21f8beba))
    - Update surface syntax tests, Joe WIP ([`d46aa0b`](https://github.com/hydro-project/hydroflow/commit/d46aa0bd607f50e297e80a8d4bb645db6eb2de9b))
    - Cleanups, rename `hydroflow_core` to `hydroflow_lang` ([`c8f2b56`](https://github.com/hydro-project/hydroflow/commit/c8f2b56295555c04e8240432ff686d89fccef01c))
    - Cleanup surface syntax tests ([`cb901c8`](https://github.com/hydro-project/hydroflow/commit/cb901c82d3c5f330846e346cd151f07c7d2661b1))
    - Improve covid tracing example, fix to allow map, filter, etc. to be either pull or push ([`e5d8c9a`](https://github.com/hydro-project/hydroflow/commit/e5d8c9a9d89da53370a4b3207a6533f2fa614f3a))
    - Codegen covid tracing working ([`4b474b7`](https://github.com/hydro-project/hydroflow/commit/4b474b71a4b02f1dfec5073fcac1da15f6fe95e2))
    - Surface syntax reachability working! ([`b857a86`](https://github.com/hydro-project/hydroflow/commit/b857a860d1f5b90874326b1f0dcca91f19fea17c))
    - Compiling version! ([`a095095`](https://github.com/hydro-project/hydroflow/commit/a0950958021789e19db39b70900552101278ac19))
    - Fix port order via BTreeMap (instead of HashMap) ([`55ffb5a`](https://github.com/hydro-project/hydroflow/commit/55ffb5a76076c5ee50f260d24e6c6c742bc72070))
    - More work on codegen ([`ea4a467`](https://github.com/hydro-project/hydroflow/commit/ea4a467541ec0895134431af1c69273f37e8866f))
    - Build out codegen structure, reachability example almost compiles ([`81c0512`](https://github.com/hydro-project/hydroflow/commit/81c0512373d54b63f4c39072c48984b19c1bfb31))
    - Modularize ops, provide nicer arity errors and now warnings too. ([`5474e87`](https://github.com/hydro-project/hydroflow/commit/5474e877fc895367e8401521666043fe8c027dc2))
    - Wip on codegen w/ some code cleanups ([`d29fb7f`](https://github.com/hydro-project/hydroflow/commit/d29fb7fc275c2774be3f5c08b75f12fdaf6970ff))
    - Add #![allow(clippy::explicit_auto_deref)] due to false positives ([`20382f1`](https://github.com/hydro-project/hydroflow/commit/20382f13d9baf49ee896a6c643bb25788aff2db0))
    - Subgraph partitioning algorithm working ([`cc8c29c`](https://github.com/hydro-project/hydroflow/commit/cc8c29ccb52e662b80989904b32bb7ef8b487c28))
    - Add checker for operator arity ([`8e7f85a`](https://github.com/hydro-project/hydroflow/commit/8e7f85a7681e62354d5640fd95a703247b984bfb))
    - Cleanup old code, add helpful comments ([`0fe0f40`](https://github.com/hydro-project/hydroflow/commit/0fe0f40dd49bcd1164032ea331f06c209de2ce16))
    - Add mermaid rendering of surface syntax ([`09c9647`](https://github.com/hydro-project/hydroflow/commit/09c964784006898825f1a91893dc20c30bc7853f))
    - New parsing with nice error messages ([`b896108`](https://github.com/hydro-project/hydroflow/commit/b896108792a809e4cbc5053d5214a891c37d330b))
    - Parse updated arrow syntax ([`b7f131c`](https://github.com/hydro-project/hydroflow/commit/b7f131ce38cffc6c8491c778500ceb32d44221d8))
    - Implement basic arrow syntax ([`de8ed49`](https://github.com/hydro-project/hydroflow/commit/de8ed492c1220a131052544079085f44266fe87f))
    - Hydroflow_macro boilerplate ([`b2a8b85`](https://github.com/hydro-project/hydroflow/commit/b2a8b853907ee93ad02ceeb39b95da08a0970330))
    - Update SHJ to return `(K, (V1, V2))` for easier chaining (instead of `(K, V1, V2)`) ([`b6a942f`](https://github.com/hydro-project/hydroflow/commit/b6a942f6a282fc142080e234e3eb1c16b3fcc34d))
    - Rename run methods to remove 'tick' naming, add `run_epoch` ([`86a92a3`](https://github.com/hydro-project/hydroflow/commit/86a92a30a4419a26a9fe32351aa754ac1e148ae4))
    - Remove unused lifetime from CrossJoinState impl ([`975ace7`](https://github.com/hydro-project/hydroflow/commit/975ace7151ce651a1c505abdc2593479ed694ba9))
    - Fix build by adding trait bounds to existential types ([`0381ee7`](https://github.com/hydro-project/hydroflow/commit/0381ee7b255482154b81f5ad02b530b6460788e7))
    - Refactor ::compiled::pull into separate files ([`9b8d632`](https://github.com/hydro-project/hydroflow/commit/9b8d63265e8ce72b734c31d7a9bcc82463fa32ec))
    - Add scheduling method to context, update docs ([`5f4618e`](https://github.com/hydro-project/hydroflow/commit/5f4618ee81bd6a3bf9c0bbde509dc5b264a1f646))
    - Provide `&mut Context` to subgraph closure, instead of just ref ([`cf308c5`](https://github.com/hydro-project/hydroflow/commit/cf308c5efc303e210b02e0c377caa58f8a6a0a9b))
    - Rename `into_parts()` to `make_parts()` ([`b7ea31f`](https://github.com/hydro-project/hydroflow/commit/b7ea31f28ee36e1324960ea48d5c1aa6abc19f93))
    - Implement separate `MapScanSurface`, use state API for `map_scan` ([`1a52e4a`](https://github.com/hydro-project/hydroflow/commit/1a52e4a6238fbe1f0b82043b15ecc5fece2f0228))
    - Add `MonotonicMap`, `Map`-like structure that only stores one value for a monotonically increasing key ([`0fd9220`](https://github.com/hydro-project/hydroflow/commit/0fd9220ce162b36257a4b9ddb6e0bd8ed2aa1630))
    - Use `std::iter::Once` instead of `Option`'s `IntoIter` ([`3ec1514`](https://github.com/hydro-project/hydroflow/commit/3ec15140a7be10e48718cdf5bcf33d96b12887d5))
    - Add `Clear` helper trait ([`12cd4f5`](https://github.com/hydro-project/hydroflow/commit/12cd4f56696d37734820bd6b5b54108c6a3a3563))
    - Provide context to initial build of subgraph, surface API ([`bc78af1`](https://github.com/hydro-project/hydroflow/commit/bc78af1c02758d055e797241f10d36daee3d8388))
    - Use `std::iter::Once` instead of `std::array::IntoIter<_, 1>` ([`8e71c57`](https://github.com/hydro-project/hydroflow/commit/8e71c570c3be5cb19a8916c4e780095f0704254e))
    - Combine now-redundant `Push`/`PullBuildBase` traits ([`c8d5cd7`](https://github.com/hydro-project/hydroflow/commit/c8d5cd71350d4b9ac8c0ba1e4be18998aa6fe077))
    - Use `where Self: 'slf` clause to fix build ([`cc8cb86`](https://github.com/hydro-project/hydroflow/commit/cc8cb86dcbe0b3cb04cb6475b573f5e33686a77e))
    - Add book example_4 and example_5 ([`77ee868`](https://github.com/hydro-project/hydroflow/commit/77ee8687df5b42510c8941b4ef1ec4f8cf17ce82))
    - Remove lifetime from Context, avoid need to construct instances ([`d986d01`](https://github.com/hydro-project/hydroflow/commit/d986d010aa0759ee9acf42368c21cd3480ec0618))
    - Add vechandoff to builder::prelude ([`3aa8744`](https://github.com/hydro-project/hydroflow/commit/3aa8744f64df8f8b8d39fbc0f3b9528770712b1c))
    - Always initialize stratum #0 ([`5484e29`](https://github.com/hydro-project/hydroflow/commit/5484e299b854467a441fed0e93b8ce687ff71165))
    - Finish example 2, start example 3 ([`3256532`](https://github.com/hydro-project/hydroflow/commit/32565326fbee70b5b62d54c315aee1a6bc7ad7e8))
    - Make book code run on test, then remove now-redundant examples ([`cb4e8da`](https://github.com/hydro-project/hydroflow/commit/cb4e8dac5106e64d63ecf85e1288fea0be97ddd9))
    - Update example-1, write first half of example-2 ([`11a12b2`](https://github.com/hydro-project/hydroflow/commit/11a12b2daa3496ed36af4ca78d87454411da3839))
    - Move diagram code into flow_graph ([`0a9b96c`](https://github.com/hydro-project/hydroflow/commit/0a9b96cd76f378233e9ff8cb1a8e97ec6b2dab5e))
    - Add example 1 (simplest example) ([`26f5dbf`](https://github.com/hydro-project/hydroflow/commit/26f5dbf2eb89860160773e4ceb5b4afdac43da22))
    - Write concepts and architecture section of book ([`79b315b`](https://github.com/hydro-project/hydroflow/commit/79b315bfd75c2e3b85cac216e2325679d7a61962))
    - Use `NodeId` for mermaid graph instead of `usize` ([`1cf48c4`](https://github.com/hydro-project/hydroflow/commit/1cf48c4d89c5724a46793ec1faea81d3e88c5ec0))
    - Make FlowGraph fields private ([`6dc3e1e`](https://github.com/hydro-project/hydroflow/commit/6dc3e1e4d809f660d1413df4b03855eb20d41dd9))
    - Fixup! Use newtypes for Subgraph/Handoff/State IDs ([`35fa5e9`](https://github.com/hydro-project/hydroflow/commit/35fa5e9870006d46203771381ade4ba7615ec8f2))
    - Error handling in `write_mermaid` ([`73a5631`](https://github.com/hydro-project/hydroflow/commit/73a563141c51f36ba4584f02ee723f97cd098810))
    - Use newtype IDs in mermaid generation ([`8a7bbc8`](https://github.com/hydro-project/hydroflow/commit/8a7bbc81f31cd4152c5c2f9b937dfbac3e2d5352))
    - Use newtypes for Subgraph/Handoff/State IDs ([`f87bef5`](https://github.com/hydro-project/hydroflow/commit/f87bef5c4be5ef0038b77360f18a8fda9440fff3))
    - Fix `Reactor.trigger()` return type ([`bead1ee`](https://github.com/hydro-project/hydroflow/commit/bead1eeb6d807fb9adc682184b9f30890f202c4c))
    - Add mermaid plots as options to Builder-based examples ([`47e1814`](https://github.com/hydro-project/hydroflow/commit/47e1814a0297a9f180baf1ea2d014335e4f2a703))
    - Add partition_with_context, change to use FnMut instead of Fn ([`5618b37`](https://github.com/hydro-project/hydroflow/commit/5618b3723674e3fc14f2378fe27319c06b66d32a))
    - Add context to ForEach surface ([`0108e03`](https://github.com/hydro-project/hydroflow/commit/0108e0372f060fdfcd21c1b4a52156c20784e90e))
    - Add inspect_with_context ([`c94d86a`](https://github.com/hydro-project/hydroflow/commit/c94d86a4c59677f9aa73a183007929124e4c0fb2))
    - Add context to FilterMap surface ([`c4ab9cb`](https://github.com/hydro-project/hydroflow/commit/c4ab9cb3d50ddc178eda895335c51aa97745f0b1))
    - Add context to Filter surface ([`430cab7`](https://github.com/hydro-project/hydroflow/commit/430cab7b94f5e8be68858b890c7ca48bc3b1b0d1))
    - Add context to Map surface ([`af75f72`](https://github.com/hydro-project/hydroflow/commit/af75f726363323db56d1f46c99068f95eddc0a08))
    - Combine old 'hof with context under new 'ctx lifetime ([`3425143`](https://github.com/hydro-project/hydroflow/commit/34251436edfe512bb017cfa9849e65a85b7bbbfd))
    - Cleanup to allow pin streams ([`91901b3`](https://github.com/hydro-project/hydroflow/commit/91901b3e49f8d658697a2a94b6d55b27bd1e2940))
    - Ignore clippy::iter_with_drain due to false positives ([`95f9eec`](https://github.com/hydro-project/hydroflow/commit/95f9eec441bb8524669cf789c11ceb21e4f07cea))
    - Dataflow diagrams for 2pc readme ([`9857a39`](https://github.com/hydro-project/hydroflow/commit/9857a394f4814388997353328080e9885f46f0d6))
    - Update where clause position ([`f44c50a`](https://github.com/hydro-project/hydroflow/commit/f44c50a5dd9d4fe44b0095b8d06b75818e77debb))
    - Clean up old code in chat client.rs ([`a65a3c1`](https://github.com/hydro-project/hydroflow/commit/a65a3c13c0e0bd37748209ad7d40b5864b4edcd3))
    - Rename scheduler `ready_queue` to `stratum_queues` ([`d39d44d`](https://github.com/hydro-project/hydroflow/commit/d39d44d76355db3e0b9b5c1317dbdd9ba314857c))
    - Cleanup `Hydroflow.next_stratum()`, fix bug ([`eda9306`](https://github.com/hydro-project/hydroflow/commit/eda93064244007d56ce74d59282ca9a0df460899))
    - Fix, expose, test epoch and stratum counters ([`a89d66b`](https://github.com/hydro-project/hydroflow/commit/a89d66b9495853deb1a3fc31052aeb8752cad86c))
    - Add non-monotonic median test with surface API ([`a60a02a`](https://github.com/hydro-project/hydroflow/commit/a60a02ac3d93237c2b4aa4c5008500fbec711e8b))
    - Implement custom strata numbers, allow per-stratum `tick`ing. ([`4f25e93`](https://github.com/hydro-project/hydroflow/commit/4f25e939e66ee6a931435872a0020571239d62ce))
    - Implement stratum metadata, but for now keep all subgraphs in stratum 0 ([`0439a90`](https://github.com/hydro-project/hydroflow/commit/0439a9079b4b735b46eb2410bf8f17632aab267c))
    - Add docs to groupby test ([`c2a40e3`](https://github.com/hydro-project/hydroflow/commit/c2a40e3fcced44391a4a71e22858b65fa4b4500a))
    - Add scan to surface API ([`0685ded`](https://github.com/hydro-project/hydroflow/commit/0685ded2b81785081fda70b473a1a11d6403e48d))
    - Add ad-hoc core groupby test ([`b96d0a2`](https://github.com/hydro-project/hydroflow/commit/b96d0a2115afbe8c3d652b24f5cc71f97186e84d))
    - Add `HydroflowBuilder::new()` ([`26fdc89`](https://github.com/hydro-project/hydroflow/commit/26fdc891c95440a4d4a276259f7037942fca8808))
    - Fix hf.run_async() busy spinning. ([`f8efbc5`](https://github.com/hydro-project/hydroflow/commit/f8efbc50837723bb0b4b721a897bcf2c6e1065c8))
    - Use `Name: Into<Cow<'static, str>>` generic to avoid `.into()`s ([`c409c3f`](https://github.com/hydro-project/hydroflow/commit/c409c3f5bde07685d87899a1d03244426c09c322))
    - Require handoffs and subgraphs to have friendly names ([`0feeb66`](https://github.com/hydro-project/hydroflow/commit/0feeb662eaca3dd655faee7c0518698ca07f6115))
    - Add `README.md`s to examples ([`4f8377a`](https://github.com/hydro-project/hydroflow/commit/4f8377a4fd3454c6d15f19b7acf665e68f3a98ed))
    - Add IntoHydroflow trait ([`a2f5f60`](https://github.com/hydro-project/hydroflow/commit/a2f5f60ecdb97839ef5ac7c0db0c207ccd87bbae))
    - Graph reachability example ([`9c3d3e4`](https://github.com/hydro-project/hydroflow/commit/9c3d3e4974a89ea2648666c84f037026cdb3e22e))
    - Simplify surface pull_iter generics ([`af3e990`](https://github.com/hydro-project/hydroflow/commit/af3e990605bb88ecad25ab86ba1b48c1f93047ff))
    - Remove obsolete "variadic_generics" feature ([`efa9a86`](https://github.com/hydro-project/hydroflow/commit/efa9a86a519485a92e07c45be932beea752340d8))
    - Cleanup scheduler & docs on Hydroflow graph struct [ci-bench] ([`53ff7f6`](https://github.com/hydro-project/hydroflow/commit/53ff7f609db9d8ab249306323711a4b93acb85da))
    - Move examples to hydroflow/examples ([`799ec17`](https://github.com/hydro-project/hydroflow/commit/799ec17ad39037c1beb218240364db87fc42d560))
    - Fix tl! doc link ([`2edea84`](https://github.com/hydro-project/hydroflow/commit/2edea84c1e41044d1a39bb1cdfa7979d28eac1a9))
    - Add context to surface build ([`a61638a`](https://github.com/hydro-project/hydroflow/commit/a61638aec9569a55f149feba3aeea55021ce35dc))
    - Add add_input_from_stream() to builder, fix #52 ([`a8dd6f4`](https://github.com/hydro-project/hydroflow/commit/a8dd6f485e5626688e285d28aeaa11b41e84759d))
    - Rename reverse() to push_to(), fix #50 ([`6cb952d`](https://github.com/hydro-project/hydroflow/commit/6cb952d8ac718a0558df10a0bafe41d0f2d7a157))
    - Rename pivot() to pull_to_push(), fix #49 ([`d3afea7`](https://github.com/hydro-project/hydroflow/commit/d3afea7e214a101ff12ebeb3564681812a4d327b))
    - Update add_channel_input to take SendPort as arg instead of returning RecvPort ([`1135b66`](https://github.com/hydro-project/hydroflow/commit/1135b664fba1e5480ff6a493fd62f720c26b9ead))
    - Update add_input to take SendPort as arg instead of returning RecvPort ([`c432f22`](https://github.com/hydro-project/hydroflow/commit/c432f22bb52258dac987c3a51dbad4f02855049a))
    - Rename ports to RecvPort, SendPort ([`f7df1fb`](https://github.com/hydro-project/hydroflow/commit/f7df1fbc99c556443d6ba20b09573f2fbca3fe5e))
    - Add port docs ([`74abbc0`](https://github.com/hydro-project/hydroflow/commit/74abbc071652bfe7873580610e4f82b96bde2143))
    - Use SEND/RECV type tags instead of true/false const generics for `Port`s/`PortCtx`s ([`f210fb5`](https://github.com/hydro-project/hydroflow/commit/f210fb554cd612a8b2c96823e178802031a66365))
    - Remove unused imports ([`7bd4f89`](https://github.com/hydro-project/hydroflow/commit/7bd4f89371546b380a41f44e8641a5512194d93a))
    - Rename make_handoff to make_edge ([`a9a4586`](https://github.com/hydro-project/hydroflow/commit/a9a4586e60fcf3a9f57361b3982eaeb9f9dba5ff))
    - Remove graph_demux code, tests ([`2cae83c`](https://github.com/hydro-project/hydroflow/commit/2cae83c283f96a446867690027e5721a3f8c4fd5))
    - Update surface API module docs for new core API, removal of connectors ([`073bd45`](https://github.com/hydro-project/hydroflow/commit/073bd458d3baad579828692a72bf32e4e4c14554))
    - Update networked.rs test for new core API ([`00a5d9e`](https://github.com/hydro-project/hydroflow/commit/00a5d9e86f5afa36e57893b528925c9a109cc552))
    - Rename add_subgraph_homogeneous to add_subgraph_n_m ([`216d261`](https://github.com/hydro-project/hydroflow/commit/216d2617a6688dc56c19cbd2ba6f8e3018100ac1))
    - Fix/uncomment query API tee(), uncomment tee benchmarks ([`38699e9`](https://github.com/hydro-project/hydroflow/commit/38699e92fe970e88033af2889e87f0c1c55ae0d1))
    - Fix minor formatting ([`507ed77`](https://github.com/hydro-project/hydroflow/commit/507ed77b483f722f38c17927e6f398fbbd966e73))
    - Update fork_join bench for new core API ([`d7b5737`](https://github.com/hydro-project/hydroflow/commit/d7b573732e153911e860bd42e7a23c5a3c091ab2))
    - Update Surface API to new core API (3/3) ([`827e2cf`](https://github.com/hydro-project/hydroflow/commit/827e2cf64d5ecf6833f9a5e5506e293415ebc4c8))
    - Update Surface API to new core API (2/3) ([`40ed9fd`](https://github.com/hydro-project/hydroflow/commit/40ed9fd34ce00f148513244ebcf86e91d21d6929))
    - Update Surface API to new core API (1/3) ([`489de33`](https://github.com/hydro-project/hydroflow/commit/489de33d826f7242ddba758f21cc2117972fc51e))
    - Include Context for add_subgraph_homogeneous ([`2a01cc9`](https://github.com/hydro-project/hydroflow/commit/2a01cc9ebf04a233a46023a13eaed88596445ce6))
    - Update rest of test.rs ([`447a635`](https://github.com/hydro-project/hydroflow/commit/447a635d0e6d41eee7b9b89980e5d338b3b54ebb))
    - Re-add add_(channel_)input methods ([`09b1ca8`](https://github.com/hydro-project/hydroflow/commit/09b1ca8103e742b48efb9d903431b9b3344ef66a))
    - Update some tests in test.rs ([`91ac238`](https://github.com/hydro-project/hydroflow/commit/91ac238f2ad28ffc2f294245d3c828c105a29dd3))
    - Fix SubgraphData succs and preds initialization ([`dc14ed8`](https://github.com/hydro-project/hydroflow/commit/dc14ed8c49203e3d42425843e9347af6880fdf06))
    - Make GraphExt consistent with new API, use macros to generate methods ([`82fc65b`](https://github.com/hydro-project/hydroflow/commit/82fc65b907d0fafa60fd437561c78fbd21ecbc6c))
    - Gut HandoffList, not used directly anymore ([`4eec910`](https://github.com/hydro-project/hydroflow/commit/4eec9104c56f0dbeaa2a91412878654315bb2f1a))
    - Reuse Send/RecvCtx, Input/OutputPort via bool generic, use for type inference of hf.add_subgraph() ([`326689d`](https://github.com/hydro-project/hydroflow/commit/326689d5c608ed3663fb1606622b7f174b5e479b))
    - Refactor hydroflow graph API ([`442b7a9`](https://github.com/hydro-project/hydroflow/commit/442b7a97478fad66bb9664efdf426dded8953421))
    - Update tests to use .flatten() instead of .flat_map(identity) ([`485b472`](https://github.com/hydro-project/hydroflow/commit/485b472cf8c233db5012047a93f909a52c7752db))
    - Add inspect, filter_map to surface API ([`816505e`](https://github.com/hydro-project/hydroflow/commit/816505e02798c2be8dd7306aab43973dd029882b))
    - Decompose flat_map() into map() and *new* flatten() ([`a57f9bf`](https://github.com/hydro-project/hydroflow/commit/a57f9bf5952e35ffd0f4cd0ecf85df7e1201e896))
    - Rename RippleJoin to CrossJoin ([`7ef330f`](https://github.com/hydro-project/hydroflow/commit/7ef330f9d528b378852136cc8dc475df33b0c616))
    - Add internal PortConnector to cleanup HydroflowBuilder ([`7a23a94`](https://github.com/hydro-project/hydroflow/commit/7a23a944349137ab969f02ca0d054e04e0f9b39b))
    - Add basic TCP, "ripple" join to Surface API ([`7b0783f`](https://github.com/hydro-project/hydroflow/commit/7b0783fdc47c55435b55083c4b964af8fec2eaa0))
    - Remove 'slf on existential closures to fix latest nightly build ([`55feadb`](https://github.com/hydro-project/hydroflow/commit/55feadb956d56b587801d5488ec71fe79824312f))
    - Implement Surface API ([`3ac75c4`](https://github.com/hydro-project/hydroflow/commit/3ac75c417de8139c82f1bebca34ce76ed2df43f9))
    - Add FlatMap pusherator ([`2fe89c9`](https://github.com/hydro-project/hydroflow/commit/2fe89c9ddf0da3600223c22481c1ba12b6284ded))
    - Add TypeList, Extend, SplitPrefix variadic helpers ([`2b9a1e5`](https://github.com/hydro-project/hydroflow/commit/2b9a1e5c8fa62347baff9a24a312406f625d1abc))
    - Fix HandoffList doc comment format ([`25aa803`](https://github.com/hydro-project/hydroflow/commit/25aa8030f70885efb6a0953344ab8b4ace2a5f96))
    - Move HandoffList into handoff module ([`2d772c3`](https://github.com/hydro-project/hydroflow/commit/2d772c3bc6d0d3dd4ce0eb9a47c6482d8df9b965))
    - Add #[must_use]es for clippy ([`f437f3f`](https://github.com/hydro-project/hydroflow/commit/f437f3ff6a0c3d53d62797ec423becabf79bed43))
    - Move PusheratorBuild impls into Pusherators (now in separate files) ([`b9d97f3`](https://github.com/hydro-project/hydroflow/commit/b9d97f305a1482bee9f959c1f00c089328a10b7c))
    - Add PusheratorBuild API ([`9fd65d2`](https://github.com/hydro-project/hydroflow/commit/9fd65d24409a8c77c4b48aab9e6305f0e0627fba))
    - Copy spinach's lattice library to Hydroflow, update imports ([`c300d47`](https://github.com/hydro-project/hydroflow/commit/c300d479823b3aa712965c332fe3e54e2ae7d928))
    - Rename tlt! macro to tt! ([`359b162`](https://github.com/hydro-project/hydroflow/commit/359b1625a19566ef8eff5cb6f6d742ebb99ba8e3))
    - Add explanation of Tokio interfacing ([`b47a51b`](https://github.com/hydro-project/hydroflow/commit/b47a51b40c4fe8a5b7f0eee1bd4b6dba8cd3d207))
    - Remove tokio runtime, use Context<'_>'s Waker for networking ([`7f29ae9`](https://github.com/hydro-project/hydroflow/commit/7f29ae98473dddda1c3225a91b8e87da8c4b1bab))
    - Simplify add_input_from_stream using Context<'_>'s Waker ([`468dbb2`](https://github.com/hydro-project/hydroflow/commit/468dbb22dfa036b53b557601dd0c733940cba606))
    - Add async Waker to Context<'_> ([`4f9e0e6`](https://github.com/hydro-project/hydroflow/commit/4f9e0e699f645a992df32a1336bc902aeeed7bc3))
    - Update all hydroflow subgraphs to take a Context<'_> ([`852371f`](https://github.com/hydro-project/hydroflow/commit/852371ff6bc52f7e64ce81879bccab2ee9fa381e))
    - Remove handoff-style state connecting, instead use StateHandle as pointers with Context<'_> ([`9db3b81`](https://github.com/hydro-project/hydroflow/commit/9db3b81b191fde45dac4c0641f7e660a7f7e9af9))
    - Add Context<'_> struct to represent operator context (states and handoffs) ([`8d78df2`](https://github.com/hydro-project/hydroflow/commit/8d78df27559fe9850e2da13367d72092819f2143))
    - Add state handles ([`53accd5`](https://github.com/hydro-project/hydroflow/commit/53accd5603aa197b7c8b17b7b794aa681b3d8629))
    - Rename OpId to SubgraphId and update variables for consistent terminology ([`4897736`](https://github.com/hydro-project/hydroflow/commit/489773675ca56c4edfa6acd21135c39490c0fcc9))
    - Change handoffs to be owned by Hydroflow, use dyn Any casts ([`dd3b774`](https://github.com/hydro-project/hydroflow/commit/dd3b77425f8511a80ca62672c552aa8a49ce7b90))
    - Add HandoffMeta any_ref() for dynamic casting ([`1a0e4ea`](https://github.com/hydro-project/hydroflow/commit/1a0e4ea4c3ee4b8a4274d824a4cfbbbeda9e0f37))
    - Add HandoffMeta: Any for future dynamic cast ([`02048a9`](https://github.com/hydro-project/hydroflow/commit/02048a9636c4a31454055eb596312d0706027ddd))
    - Replace concrete Subgraph types with FnMut() ([`d40c1b8`](https://github.com/hydro-project/hydroflow/commit/d40c1b8789630c987f7f7605ddd21421eaf9aaee))
    - Make Handoff trait take shared &self ([`6bbbaf2`](https://github.com/hydro-project/hydroflow/commit/6bbbaf200f765973573f1df1b92462b57f184248))
    - Use shared &Send/RecvCtxs ([`e3a4d65`](https://github.com/hydro-project/hydroflow/commit/e3a4d65862e7ef0a94e24009c045bf0562263aeb))
    - Make (Try)CanReceive take shared &self ([`09dd4f9`](https://github.com/hydro-project/hydroflow/commit/09dd4f923df8c6d063e71e7799c833b0ad09291a))
    - Revert "Use slightly more efficient implementation of Once [ci-bench]" ([`9ba809b`](https://github.com/hydro-project/hydroflow/commit/9ba809bd6b1d48bff1136bc14833712395eb9d36))
    - Use slightly more efficient implementation of Once [ci-bench] ([`f547e90`](https://github.com/hydro-project/hydroflow/commit/f547e90838baf935e57d91bea936dd9bfa48d75d))
    - Use old once abstraction to clarify input/output port connecting ([`a70f852`](https://github.com/hydro-project/hydroflow/commit/a70f852103c15caac9e2ed85bca2366a4571d23d))
    - Make HandoffId distinct for parallel multigraph edges ([`9a191e2`](https://github.com/hydro-project/hydroflow/commit/9a191e21feb75d5d869c6ce039749d882ff1f35c))
    - CI only publish local packages, no deps ([`d3a9ff3`](https://github.com/hydro-project/hydroflow/commit/d3a9ff36f6b976c857bee3cec778417bcae7b061))
    - Split scheduled module into smaller pieces ([`b1f4d69`](https://github.com/hydro-project/hydroflow/commit/b1f4d69adf9176173b33cd0ede41c6f6c1ded831))
    - Add scheduling from external events [ci-bench] ([`43ee3ba`](https://github.com/hydro-project/hydroflow/commit/43ee3ba5eb43eec8c1f745bc0fcdbaab24c8b544))
    - Add basic scheduler ([`3c67e10`](https://github.com/hydro-project/hydroflow/commit/3c67e107020948a2c4824215b88b49a56a100c1f))
    - Split out handoffs ([`df21b5d`](https://github.com/hydro-project/hydroflow/commit/df21b5d61e524f71f58d502a60a203e6817617f4))
    - Cleanup old comments ([`9e2eca4`](https://github.com/hydro-project/hydroflow/commit/9e2eca4604fe4774e3997cdd01458d7a7c50cf0f))
    - Cleanup fmt & clippy ([`1415d47`](https://github.com/hydro-project/hydroflow/commit/1415d47c82ef37b5d7220da295cd045c3d44e0fa))
    - Add TeeingHandoff ([`c22ed90`](https://github.com/hydro-project/hydroflow/commit/c22ed90898193d05436d6743ed282338755d03f7))
    - Batch join state together ([`b3ceb6e`](https://github.com/hydro-project/hydroflow/commit/b3ceb6e883beff234afcb9cc36d3691d0a8b8556))
    - Covid tracing working ([`916df84`](https://github.com/hydro-project/hydroflow/commit/916df849a1e5bc56cdc6379330025237bc25c55f))
    - Change handoff api ([`0402ae3`](https://github.com/hydro-project/hydroflow/commit/0402ae3f9f93ed9d81df9801689847019f15f5b7))
    - Implement full covid_tracing example (not working) ([`aefcbec`](https://github.com/hydro-project/hydroflow/commit/aefcbec596cc0f5165fd340c680a7ea7511194da))
    - Add slightly more complex reachability benchmark ([`513b209`](https://github.com/hydro-project/hydroflow/commit/513b2091fbc8f53d4a9a883732ae60a98010903b))
    - Add some more compiled operators ([`1167478`](https://github.com/hydro-project/hydroflow/commit/11674780cb55e46b707b5ef9482c64d5eabb9059))
    - Add TeeN Pusherator, hydroflow/compiled fan_out bench ([`e59f2fc`](https://github.com/hydro-project/hydroflow/commit/e59f2fc62d549851c52c8811e37673fda7e81745))
    - Fixup! working layout for compiled layer ([`1d22f91`](https://github.com/hydro-project/hydroflow/commit/1d22f9144cedc29484590286584a05afe815beb5))
    - Working layout for compiled layer ([`561a418`](https://github.com/hydro-project/hydroflow/commit/561a418d41782cdd6e2423310c9420fbc74b0d36))
    - Start working on compiled layer ([`e2521e8`](https://github.com/hydro-project/hydroflow/commit/e2521e8b7cbcea04b652c5f9d0726c30931c95ba))
    - Move scheduled layer to a module ([`803274d`](https://github.com/hydro-project/hydroflow/commit/803274d5f5cf8657fc47c3ee3c2a6e99700c34bb))
    - Fixup! Appease clippy ([`8ec17bc`](https://github.com/hydro-project/hydroflow/commit/8ec17bccf7e2adb2370aad605473e190fb5f07e9))
    - Appease clippy ([`19de331`](https://github.com/hydro-project/hydroflow/commit/19de3315aacc9c8f3891b29d56a5fe083feb4690))
    - Add fan_out benchmark for hydroflow ([`94ed8a2`](https://github.com/hydro-project/hydroflow/commit/94ed8a2a12c6a1593fa3a379d2bb43ca241436d4))
    - Add an n->m operator and fan_in test ([`2c1d7b4`](https://github.com/hydro-project/hydroflow/commit/2c1d7b49e676e811e4058e6ac4dd3a6a98ef8015))
    - Add a simple query builder ([`8a9fb45`](https://github.com/hydro-project/hydroflow/commit/8a9fb45783d556f222b74563ea006dfece0faf0f))
    - Add fork_join benchmark for hydroflow ([`d37ea6c`](https://github.com/hydro-project/hydroflow/commit/d37ea6c653a20b1ca5658f6dbb82ac6854d91fc8))
    - Update to use two CanReceive and TryCanReceive traits ([`57e8178`](https://github.com/hydro-project/hydroflow/commit/57e8178a14f45d617246f5a3415b9603b31c967f))
    - Benchmark, temp add IntoIterator for RecvCtx ([`eb6fb06`](https://github.com/hydro-project/hydroflow/commit/eb6fb0678cd1dd3cdc5bc1b678df52f6c8336a20))
    - Update handoffs ([`138c59b`](https://github.com/hydro-project/hydroflow/commit/138c59b8c4e3bd13fa4d69e0de8202de665bc095))
    - Handoff2 trait 2x &mut version ([`2a231a8`](https://github.com/hydro-project/hydroflow/commit/2a231a8728e61a68acde0e1d16e7d8fb92d3c43c))
    - Move handoffs to own module ([`e04397b`](https://github.com/hydro-project/hydroflow/commit/e04397bbb594bb0c54159d2366d3b12fca8ffc1e))
    - Pass around the handoff instead of Readable/Writable ([`9541e0a`](https://github.com/hydro-project/hydroflow/commit/9541e0aae82b530035c833064d70f84a4ca84a8c))
    - Move hydroflow identity test to benches workspace ([`044c05a`](https://github.com/hydro-project/hydroflow/commit/044c05ae9377e5ab1c6e274f66c5ba924e4e7bb5))
    - Cleanup for clippy ([`b382993`](https://github.com/hydro-project/hydroflow/commit/b3829931ae2b0d8d1f6c0938d4b09963f70d9fdc))
    - Use cargo workspaces ([`f015425`](https://github.com/hydro-project/hydroflow/commit/f015425e10021ee6123c1d413240db83c1cdf390))
</details>

