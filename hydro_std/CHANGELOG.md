

## v0.17.0-alpha.6 (2026-09-30)

## v0.17.0-alpha.5 (2026-09-21)

### Style

 - <csr-id-595dd03bf88557cc11fe00b1aed11a71f3f6dcbe/> add match, or_else-related clippy lints
   low-hanging anti-slopification measure

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 69 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#3122](https://github.com/hydro-project/hydro/issues/3122)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#3122](https://github.com/hydro-project/hydro/issues/3122)**
    - Add match, or_else-related clippy lints ([`595dd03`](https://github.com/hydro-project/hydro/commit/595dd03bf88557cc11fe00b1aed11a71f3f6dcbe))
 * **Uncategorized**
    - Release hydro_build_utils v0.1.1-alpha.1, dfir_lang v0.17.0-alpha.4, dfir_macro v0.17.0-alpha.4, variadics v0.2.0-alpha.3, variadics_macro v0.8.0-alpha.2, lattices v0.8.0-alpha.4, example_test v0.0.2-alpha.0, sinktools v0.2.0-alpha.4, hydro_deploy_integration v0.17.0-alpha.3, dfir_rs v0.17.0-alpha.5, copy_span v0.1.2-alpha.0, hydro_concurrent_cargo v0.1.1-alpha.0, hydro_deploy v0.17.0-alpha.4, hydro_lang v0.17.0-alpha.5, hydro_std v0.17.0-alpha.5, safety bump 4 crates ([`38ccb27`](https://github.com/hydro-project/hydro/commit/38ccb27ae7a08b9ac1ab544f4047a4f4592ee230))
</details>

## v0.17.0-alpha.4 (2026-07-14)

### Documentation

 - <csr-id-36830afa7ecc5cf4627a6570649db2212f783186/> fix lots of small docs issues

### New Features (BREAKING)

 - <csr-id-dbdd2f110c63c2486af563b10b8fe0b19a729e5b/> place Tokio dependencies behind a feature flag
   Breaking Changes:
   - the `hydro_lang::telemetry::emf` module now requires the non-default
   `telemetry_emf` feature

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 22 days passed between releases.
 - 2 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 2 unique issues were worked on: [#2960](https://github.com/hydro-project/hydro/issues/2960), [#3012](https://github.com/hydro-project/hydro/issues/3012)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#2960](https://github.com/hydro-project/hydro/issues/2960)**
    - Place Tokio dependencies behind a feature flag ([`dbdd2f1`](https://github.com/hydro-project/hydro/commit/dbdd2f110c63c2486af563b10b8fe0b19a729e5b))
 * **[#3012](https://github.com/hydro-project/hydro/issues/3012)**
    - Fix lots of small docs issues ([`36830af`](https://github.com/hydro-project/hydro/commit/36830afa7ecc5cf4627a6570649db2212f783186))
 * **Uncategorized**
    - Release dfir_lang v0.17.0-alpha.3, variadics v0.2.0-alpha.2, lattices v0.8.0-alpha.3, dfir_pipes v0.1.0-alpha.3, multiplatform_test v0.7.1-alpha.0, dfir_rs v0.17.0-alpha.4, hydro_concurrent_cargo v0.1.0-alpha.0, hydro_deploy v0.17.0-alpha.3, hydro_lang v0.17.0-alpha.4, hydro_std v0.17.0-alpha.4, safety bump 3 crates ([`6287d84`](https://github.com/hydro-project/hydro/commit/6287d84c83b0a37798d6afdeb6bfacaf9a8ce3d1))
</details>

## v0.17.0-alpha.3 (2026-06-22)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 1 commit contributed to the release.
 - 3 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release dfir_macro v0.17.0-alpha.3, lattices v0.8.0-alpha.2, sinktools v0.2.0-alpha.3, hydro_deploy_integration v0.17.0-alpha.2, dfir_rs v0.17.0-alpha.3, hydro_deploy v0.17.0-alpha.2, hydro_lang v0.17.0-alpha.3, hydro_std v0.17.0-alpha.3 ([`295c0ec`](https://github.com/hydro-project/hydro/commit/295c0ec5d2d3f182598972d9a0c5511f5812c6ae))
</details>

## v0.17.0-alpha.2 (2026-06-19)

### Chore

 - <csr-id-1c980fe272b0f8641d04b14b10837366e42531d2/> raise clippy type-complexity-threshold

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 8 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#2947](https://github.com/hydro-project/hydro/issues/2947)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#2947](https://github.com/hydro-project/hydro/issues/2947)**
    - Raise clippy type-complexity-threshold ([`1c980fe`](https://github.com/hydro-project/hydro/commit/1c980fe272b0f8641d04b14b10837366e42531d2))
 * **Uncategorized**
    - Release dfir_lang v0.17.0-alpha.2, dfir_pipes v0.1.0-alpha.2, sinktools v0.2.0-alpha.2, hydro_deploy_integration v0.17.0-alpha.1, dfir_rs v0.17.0-alpha.2, hydro_deploy v0.17.0-alpha.1, hydro_lang v0.17.0-alpha.2, hydro_std v0.17.0-alpha.2 ([`faa7a90`](https://github.com/hydro-project/hydro/commit/faa7a90d1d9524d1870360d4701a8746c804c10c))
</details>

## v0.17.0-alpha.1 (2026-06-11)

### Chore

 - <csr-id-e70eab6a0c793ef095e2cd747220d5419f7bf1a4/> revert accidental `v1.0.0-alpha.0` releases of `dfir_lang` & `variadics`, update `cargo-smart-release` fork version

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 1 day passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release dfir_lang v0.17.0-alpha.1, dfir_macro v0.17.0-alpha.1, variadics v0.2.0-alpha.1, variadics_macro v0.8.0-alpha.1, lattices v0.8.0-alpha.1, dfir_pipes v0.1.0-alpha.1, sinktools v0.2.0-alpha.1, dfir_rs v0.17.0-alpha.1, hydro_lang v0.17.0-alpha.1, hydro_std v0.17.0-alpha.1 ([`2035d2e`](https://github.com/hydro-project/hydro/commit/2035d2e29fabae26c069bb01aefbed58b631742c))
    - Revert accidental `v1.0.0-alpha.0` releases of `dfir_lang` & `variadics`, update `cargo-smart-release` fork version ([`e70eab6`](https://github.com/hydro-project/hydro/commit/e70eab6a0c793ef095e2cd747220d5419f7bf1a4))
</details>

## v0.17.0-alpha.0 (2026-06-10)

### New Features (BREAKING)

 - <csr-id-d4ff79f02f6ab1981f671e364e02c089a4990735/> Make `source_interval` emit `()` and add `current_tick_instant`
   …for wall-clock time

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 2 commits contributed to the release.
 - 40 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#2867](https://github.com/hydro-project/hydro/issues/2867)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#2867](https://github.com/hydro-project/hydro/issues/2867)**
    - Make `source_interval` emit `()` and add `current_tick_instant` ([`d4ff79f`](https://github.com/hydro-project/hydro/commit/d4ff79f02f6ab1981f671e364e02c089a4990735))
 * **Uncategorized**
    - Release hydro_build_utils v0.1.1-alpha.0, dfir_lang v1.0.0-alpha.0, dfir_macro v0.17.0-alpha.0, variadics v1.0.0-alpha.0, variadics_macro v0.8.0-alpha.0, lattices v0.8.0-alpha.0, dfir_pipes v0.1.0-alpha.0, sinktools v0.2.0-alpha.0, hydro_deploy_integration v0.17.0-alpha.0, dfir_rs v0.17.0-alpha.0, hydro_deploy v0.17.0-alpha.0, hydro_lang v0.17.0-alpha.0, hydro_std v0.17.0-alpha.0, safety bump 10 crates ([`12e7666`](https://github.com/hydro-project/hydro/commit/12e76666f7104f81b48de5ddf397b8e72c8a6711))
</details>

## v0.16.0 (2026-05-01)

<csr-id-2f38e7eddf0363f818aa4b204c7bf549c317428a/>
<csr-id-59f5216642e3f08eae896ea67cfc5b213ad86e4a/>
<csr-id-fcce19b958bbc39ccef94277ca146baafc98ce59/>
<csr-id-efaa8f61c124c4b3c691b92a58df1686751cf45c/>
<csr-id-502e26470cbc5f9c645d7907eb6addf95b5c5533/>
<csr-id-1b947b3dab7a93fcb83b732eca968c3f2b049301/>

### New Features

 - <csr-id-fad81f0f79bac3d7524165df515fd746af148bfb/> allow passing runtime environment variables and use for benchmarking
   Binaries no longer need to be recompiled & uploaded when the number of
   virtual clients change. Should greatly reduce testing time.
 - <csr-id-b69438cc963f26a8109b227b2755ab9ba1817d51/> port `request_response` to use `sliced!` and add simulation tests
 - <csr-id-3f65882e04633cb92a2e6ac52edff81a26b35320/> add tagless member ids, add docker member id

### Bug Fixes

 - <csr-id-f104f2b3d4f78ccd05465d2af69b1be34d5ea7a5/> Virtual clients off-by-one error
   We created 1 more virtual client than specified.
 - <csr-id-661c72a402d0a9f102e73772e6cc377ae92c73fc/> bench_client realistic latency measurements
   Replaced `SystemTime` (which is NOT monotonic) with `Instant`. Expect
   reported latency for Paxos to 10x at 1 physical & virtual client (from
   0.07ms to 0.7ms).
 - <csr-id-117d617a76ef11a88df1c069bc2edac0067c080c/> Latency calculation fix
   The implementation works by subtracting start time from end time for
   each payload. The start time is measured whenever a new payload with the
   same key is created, which is effectively the moment the previous output
   is received. This overwrites the actual start time of the previous
   payload and results in unnaturally small latencies.

### New Features (BREAKING)

 - <csr-id-3f8e1c7c91037f98971989d5e0f2c65b65326ddb/> bench_client time series
   Instead of outputting throughput and latency aggregated across time
   (which hides any blips in throughput/latency), output metrics based that
   only contain data for that time interval.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 15 commits contributed to the release.
 - 157 days passed between releases.
 - 12 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 12 unique issues were worked on: [#2330](https://github.com/hydro-project/hydro/issues/2330), [#2379](https://github.com/hydro-project/hydro/issues/2379), [#2381](https://github.com/hydro-project/hydro/issues/2381), [#2394](https://github.com/hydro-project/hydro/issues/2394), [#2435](https://github.com/hydro-project/hydro/issues/2435), [#2522](https://github.com/hydro-project/hydro/issues/2522), [#2525](https://github.com/hydro-project/hydro/issues/2525), [#2554](https://github.com/hydro-project/hydro/issues/2554), [#2558](https://github.com/hydro-project/hydro/issues/2558), [#2614](https://github.com/hydro-project/hydro/issues/2614), [#2626](https://github.com/hydro-project/hydro/issues/2626), [#2700](https://github.com/hydro-project/hydro/issues/2700)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#2330](https://github.com/hydro-project/hydro/issues/2330)**
    - Add tagless member ids, add docker member id ([`3f65882`](https://github.com/hydro-project/hydro/commit/3f65882e04633cb92a2e6ac52edff81a26b35320))
 * **[#2379](https://github.com/hydro-project/hydro/issues/2379)**
    - Port `request_response` to use `sliced!` and add simulation tests ([`b69438c`](https://github.com/hydro-project/hydro/commit/b69438cc963f26a8109b227b2755ab9ba1817d51))
 * **[#2381](https://github.com/hydro-project/hydro/issues/2381)**
    - Port quorum to use `sliced!` ([`fcce19b`](https://github.com/hydro-project/hydro/commit/fcce19b958bbc39ccef94277ca146baafc98ce59))
 * **[#2394](https://github.com/hydro-project/hydro/issues/2394)**
    - Port `bench_client` to use sliced!, introduce `Stream::merge_ordered` ([`59f5216`](https://github.com/hydro-project/hydro/commit/59f5216642e3f08eae896ea67cfc5b213ad86e4a))
 * **[#2435](https://github.com/hydro-project/hydro/issues/2435)**
    - Generalize bench_client ([`2f38e7e`](https://github.com/hydro-project/hydro/commit/2f38e7eddf0363f818aa4b204c7bf549c317428a))
 * **[#2522](https://github.com/hydro-project/hydro/issues/2522)**
    - Refactor client_aggregator so printing is a separate function ([`502e264`](https://github.com/hydro-project/hydro/commit/502e26470cbc5f9c645d7907eb6addf95b5c5533))
 * **[#2525](https://github.com/hydro-project/hydro/issues/2525)**
    - Update pinned rust to 1.92, add lints/fixes for redundant cloning, string handling ([`efaa8f6`](https://github.com/hydro-project/hydro/commit/efaa8f61c124c4b3c691b92a58df1686751cf45c))
 * **[#2554](https://github.com/hydro-project/hydro/issues/2554)**
    - Bench_client time series ([`3f8e1c7`](https://github.com/hydro-project/hydro/commit/3f8e1c7c91037f98971989d5e0f2c65b65326ddb))
 * **[#2558](https://github.com/hydro-project/hydro/issues/2558)**
    - Latency calculation fix ([`117d617`](https://github.com/hydro-project/hydro/commit/117d617a76ef11a88df1c069bc2edac0067c080c))
 * **[#2614](https://github.com/hydro-project/hydro/issues/2614)**
    - Allow passing runtime environment variables and use for benchmarking ([`fad81f0`](https://github.com/hydro-project/hydro/commit/fad81f0f79bac3d7524165df515fd746af148bfb))
 * **[#2626](https://github.com/hydro-project/hydro/issues/2626)**
    - Bench_client realistic latency measurements ([`661c72a`](https://github.com/hydro-project/hydro/commit/661c72a402d0a9f102e73772e6cc377ae92c73fc))
 * **[#2700](https://github.com/hydro-project/hydro/issues/2700)**
    - Virtual clients off-by-one error ([`f104f2b`](https://github.com/hydro-project/hydro/commit/f104f2b3d4f78ccd05465d2af69b1be34d5ea7a5))
 * **Uncategorized**
    - Release hydro_lang v0.16.0, hydro_std v0.16.0 ([`f96e4d2`](https://github.com/hydro-project/hydro/commit/f96e4d2590875352ad560f79e96dff1a04a4727a))
    - Release dfir_pipes v0.0.1, example_test v0.0.1, sinktools v0.1.0, hydro_deploy_integration v0.16.0, lattices_macro v0.6.0, variadics_macro v0.7.0, lattices v0.7.0, multiplatform_test v0.7.0, dfir_rs v0.16.0, copy_span v0.1.1, hydro_deploy v0.16.0, hydro_lang v0.16.0, hydro_std v0.16.0 ([`118b356`](https://github.com/hydro-project/hydro/commit/118b356447d92e778313d72a351e5a8d2814aa1a))
    - Release hydro_build_utils v0.1.0, dfir_lang v0.16.0, dfir_macro v0.16.0, variadics v0.1.0, dfir_pipes v0.0.1, example_test v0.0.1, sinktools v0.1.0, hydro_deploy_integration v0.16.0, lattices_macro v0.6.0, variadics_macro v0.7.0, lattices v0.7.0, multiplatform_test v0.7.0, dfir_rs v0.16.0, copy_span v0.1.1, hydro_deploy v0.16.0, hydro_lang v0.16.0, hydro_std v0.16.0, safety bump 13 crates ([`c20757a`](https://github.com/hydro-project/hydro/commit/c20757ae0e9e10463b2a499de4b7d37ab02269d0))
</details>

## v0.15.0 (2025-11-25)

<csr-id-0be5729dd87a91a70001f88283b380d3da8df7d0/>
<csr-id-057192afde1373caedbbfc24516c28a96d12928c/>
<csr-id-2e2cd770fd18cd219ec1acdd2c74d46a5ee1b2de/>
<csr-id-1fc751515d5fd4b6ec07fec8e83b4aff70b3acca/>
<csr-id-b256cba932a8d6d7a6be7b1c98c2f8c20b299375/>
<csr-id-fa4e9d9914ed52aa5a7237c32a0dc57d713ec14a/>
<csr-id-a4d8af603e6ad14659d1d43ca168495c883a58eb/>
<csr-id-537309f9aac44498aa617c8517fdbc21616cbebf/>
<csr-id-5f8a4da212eba8b673f9c7a464c9e92d7c0602cd/>
<csr-id-4bf1c05583c838e7e4d183382fd72743402f889d/>
<csr-id-05145bf191bf0fcc794d282c3b18c0bd378a20ac/>
<csr-id-4925e2c77a8e57d45d200c98a31859571a04d150/>
<csr-id-628c1c870f1833dd05b3f57ee3e2e1235183cecb/>
<csr-id-1c26bc7899f29cb5b75446381ac5545f7ce017d8/>
<csr-id-381be86c8729403d60575bbd7297b852b6b09ec0/>
<csr-id-804f9955dfc9ea64cb0f5177bcda5b9347fafe80/>
<csr-id-1a344a98fce99d004e0ba86a67c7509d807c37bb/>

### Documentation

 - <csr-id-3fd84aa8812fa027db293727d8e304708db66916/> new template and walkthrough

### New Features

 - <csr-id-98b899baa342617bc8634220324849b1067f6233/> add cluster-membership ir node
 - <csr-id-a4bdf399f90581f409c89a72ae960405998fb33b/> add APIs for unordered stream assertions and tests for `collect_quorum`
   AI Disclosure: all the tests were generated using Kiro (!)
   
   Currently, the core functionality test has to be split up into several
   exhaustive units because otherwise the search space becomes too large
   for CI (~400s). Eventually, keyed streams may help but splitting up the
   test is a reasonable short-term fix.
 - <csr-id-b412fa0af43d011c527eaa21d2343d57e1c941c2/> Generalize bench_client's workload generation
   Allow custom functions for generating bench_client workloads (beyond
   u32,u32).

### Bug Fixes

 - <csr-id-1c139772baa20d5c9aa8ab060cc6650d6f239ca0/> Staggered client
   Client initially outputs 1 message per tick instead of all messages in 1
   giant batch, so if downstream operators do not dynamically adjust the
   batch size, they are not overwhelmed
 - <csr-id-f9e595559b3dd9641ed6f413c2a729047ebf353e/> Client aggregator waits for all clients before outputting
   Outputs N/A as throughput and latency until all clients have responded
   with positive throughput.
 - <csr-id-c40876ec4bd3b31254d683e479b9a235f3d11f67/> refactor github actions workflows, make stable the default toolchain
 - <csr-id-ab22c44aaabf2140315ba26104d9155e357a34ac/> remove strange use of batching in bench_client

### New Features (BREAKING)

 - <csr-id-5579acd1c7101a3f14c49236e1933398de0f0958/> cluster member ids are now clone instead of copy
   This is part one of a series of changes. The first part just changes
   MemberIds from being Copy to Clone, so that they can later support more
   use cases.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 15 commits contributed to the release.
 - 117 days passed between releases.
 - 12 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 12 unique issues were worked on: [#1970](https://github.com/hydro-project/hydro/issues/1970), [#1984](https://github.com/hydro-project/hydro/issues/1984), [#1995](https://github.com/hydro-project/hydro/issues/1995), [#2028](https://github.com/hydro-project/hydro/issues/2028), [#2035](https://github.com/hydro-project/hydro/issues/2035), [#2135](https://github.com/hydro-project/hydro/issues/2135), [#2140](https://github.com/hydro-project/hydro/issues/2140), [#2173](https://github.com/hydro-project/hydro/issues/2173), [#2227](https://github.com/hydro-project/hydro/issues/2227), [#2265](https://github.com/hydro-project/hydro/issues/2265), [#2272](https://github.com/hydro-project/hydro/issues/2272), [#2293](https://github.com/hydro-project/hydro/issues/2293)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1970](https://github.com/hydro-project/hydro/issues/1970)**
    - Generalize bench_client's workload generation ([`b412fa0`](https://github.com/hydro-project/hydro/commit/b412fa0af43d011c527eaa21d2343d57e1c941c2))
 * **[#1984](https://github.com/hydro-project/hydro/issues/1984)**
    - Remove uses of legacy `*_keyed` APIs ([`2e2cd77`](https://github.com/hydro-project/hydro/commit/2e2cd770fd18cd219ec1acdd2c74d46a5ee1b2de))
 * **[#1995](https://github.com/hydro-project/hydro/issues/1995)**
    - Remove strange use of batching in bench_client ([`ab22c44`](https://github.com/hydro-project/hydro/commit/ab22c44aaabf2140315ba26104d9155e357a34ac))
 * **[#2028](https://github.com/hydro-project/hydro/issues/2028)**
    - Refactor github actions workflows, make stable the default toolchain ([`c40876e`](https://github.com/hydro-project/hydro/commit/c40876ec4bd3b31254d683e479b9a235f3d11f67))
 * **[#2035](https://github.com/hydro-project/hydro/issues/2035)**
    - Client aggregator waits for all clients before outputting ([`f9e5955`](https://github.com/hydro-project/hydro/commit/f9e595559b3dd9641ed6f413c2a729047ebf353e))
 * **[#2135](https://github.com/hydro-project/hydro/issues/2135)**
    - Staggered client ([`1c13977`](https://github.com/hydro-project/hydro/commit/1c139772baa20d5c9aa8ab060cc6650d6f239ca0))
 * **[#2140](https://github.com/hydro-project/hydro/issues/2140)**
    - Reduce atomic pollution in quorum counting ([`057192a`](https://github.com/hydro-project/hydro/commit/057192afde1373caedbbfc24516c28a96d12928c))
 * **[#2173](https://github.com/hydro-project/hydro/issues/2173)**
    - Add APIs for unordered stream assertions and tests for `collect_quorum` ([`a4bdf39`](https://github.com/hydro-project/hydro/commit/a4bdf399f90581f409c89a72ae960405998fb33b))
 * **[#2227](https://github.com/hydro-project/hydro/issues/2227)**
    - New template and walkthrough ([`3fd84aa`](https://github.com/hydro-project/hydro/commit/3fd84aa8812fa027db293727d8e304708db66916))
 * **[#2265](https://github.com/hydro-project/hydro/issues/2265)**
    - Cluster member ids are now clone instead of copy ([`5579acd`](https://github.com/hydro-project/hydro/commit/5579acd1c7101a3f14c49236e1933398de0f0958))
 * **[#2272](https://github.com/hydro-project/hydro/issues/2272)**
    - Add cluster-membership ir node ([`98b899b`](https://github.com/hydro-project/hydro/commit/98b899baa342617bc8634220324849b1067f6233))
 * **[#2293](https://github.com/hydro-project/hydro/issues/2293)**
    - Add test for collecting unordered quorum ([`1fc7515`](https://github.com/hydro-project/hydro/commit/1fc751515d5fd4b6ec07fec8e83b4aff70b3acca))
 * **Uncategorized**
    - Release copy_span v0.1.0, hydro_deploy v0.15.0, hydro_lang v0.15.0, hydro_std v0.15.0 ([`bdfd6e0`](https://github.com/hydro-project/hydro/commit/bdfd6e0d10a49f1b6c45f9514982a1c60da80b9f))
    - Release sinktools v0.0.1, hydro_deploy_integration v0.15.0, lattices_macro v0.5.11, variadics_macro v0.6.2, lattices v0.6.2, multiplatform_test v0.6.0, dfir_rs v0.15.0, copy_span v0.1.0, hydro_deploy v0.15.0, hydro_lang v0.15.0, hydro_std v0.15.0 ([`ac88df1`](https://github.com/hydro-project/hydro/commit/ac88df1e98af9fa2027488252f6014efa7bef229))
    - Release hydro_build_utils v0.0.1, dfir_lang v0.15.0, dfir_macro v0.15.0, variadics v0.0.10, sinktools v0.0.1, hydro_deploy_integration v0.15.0, lattices_macro v0.5.11, variadics_macro v0.6.2, lattices v0.6.2, multiplatform_test v0.6.0, dfir_rs v0.15.0, copy_span v0.1.0, hydro_deploy v0.15.0, hydro_lang v0.15.0, hydro_std v0.15.0, safety bump 5 crates ([`092de25`](https://github.com/hydro-project/hydro/commit/092de252238dfb9fa6b01e777c6dd8bf9db93398))
</details>

## v0.14.0 (2025-07-31)

<csr-id-5ab815f3567d51e9bd114f90af8e837fe0732cd8/>

### New Features

 - <csr-id-17f4a832dac816902eebd19118dc2c4902953261/> Aggregate client throughput/latency
   Co-authored with @shadaj
   
   ---------

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 3 commits contributed to the release.
 - 111 days passed between releases.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 1 unique issue was worked on: [#1900](https://github.com/hydro-project/hydro/issues/1900)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1900](https://github.com/hydro-project/hydro/issues/1900)**
    - Aggregate client throughput/latency ([`17f4a83`](https://github.com/hydro-project/hydro/commit/17f4a832dac816902eebd19118dc2c4902953261))
 * **Uncategorized**
    - Release example_test v0.0.0, dfir_rs v0.14.0, hydro_deploy v0.14.0, hydro_lang v0.14.0, hydro_optimize v0.13.0, hydro_std v0.14.0 ([`5f69ee0`](https://github.com/hydro-project/hydro/commit/5f69ee080a9e257bc07cdc4deda90ce5525a3d0e))
    - Release dfir_lang v0.14.0, dfir_macro v0.14.0, hydro_deploy_integration v0.14.0, lattices_macro v0.5.10, variadics_macro v0.6.1, dfir_rs v0.14.0, hydro_deploy v0.14.0, hydro_lang v0.14.0, hydro_optimize v0.13.0, hydro_std v0.14.0, safety bump 6 crates ([`0683595`](https://github.com/hydro-project/hydro/commit/06835950c12884d661100c13f73ad23a98bfad9f))
</details>

## v0.13.0 (2025-04-11)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 1 commit contributed to the release.
 - 27 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release dfir_lang v0.13.0, dfir_datalog_core v0.13.0, dfir_datalog v0.13.0, dfir_macro v0.13.0, hydro_deploy_integration v0.13.0, dfir_rs v0.13.0, hydro_deploy v0.13.0, hydro_lang v0.13.0, hydro_std v0.13.0, hydro_cli v0.13.0, safety bump 8 crates ([`400fd8f`](https://github.com/hydro-project/hydro/commit/400fd8f2e8cada253f54980e7edce0631be70a82))
</details>

## v0.12.1 (2025-03-15)

<csr-id-38e6721be69f6a41aa47a01a9d06d56a01be1355/>

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

### Documentation

 - <csr-id-d7741d55a3ea9b172e962e7398f0414d0427c3f9/> add initial Rustdoc for some Stream APIs

### New Features

 - <csr-id-ca291dd618fc4065c4e30097c5ea605226383cec/> send_partitioned operator and move decoupling
   Allows specifying a distribution policy (for deciding which partition to
   send each message to) before networking. Designed to be as easy as
   possible to inject (so the distribution policy function definition takes
   in the cluster ID, for example, even though it doesn't need to, because
   this way we can avoid project->map->join)

### Bug Fixes (BREAKING)

 - <csr-id-a7e22cdd312b8483163aa89751833e1657703b8d/> reduce where `#[cfg(stageleft_runtime)]` needs to be used
   Simplifies the logic for generating the public clone of the code, which
   eliminates the need to sprinkle `#[cfg(stageleft_runtime)]` (renamed
   from `#[stageleft::runtime]`) everywhere. Also adds logic to pass
   through `cfg` attrs when re-exporting public types.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release.
 - 75 days passed between releases.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 4 unique issues were worked on: [#1650](https://github.com/hydro-project/hydro/issues/1650), [#1652](https://github.com/hydro-project/hydro/issues/1652), [#1721](https://github.com/hydro-project/hydro/issues/1721), [#1747](https://github.com/hydro-project/hydro/issues/1747)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1650](https://github.com/hydro-project/hydro/issues/1650)**
    - Add initial Rustdoc for some Stream APIs ([`d7741d5`](https://github.com/hydro-project/hydro/commit/d7741d55a3ea9b172e962e7398f0414d0427c3f9))
 * **[#1652](https://github.com/hydro-project/hydro/issues/1652)**
    - Send_partitioned operator and move decoupling ([`ca291dd`](https://github.com/hydro-project/hydro/commit/ca291dd618fc4065c4e30097c5ea605226383cec))
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

