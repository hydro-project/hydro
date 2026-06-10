

## v0.2.0-alpha.0 (2026-06-10)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 1 commit contributed to the release.
 - 40 days passed between releases.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Release hydro_build_utils v0.1.1-alpha.0, dfir_lang v1.0.0-alpha.0, dfir_macro v0.17.0-alpha.0, variadics v1.0.0-alpha.0, variadics_macro v0.8.0-alpha.0, lattices v0.8.0-alpha.0, dfir_pipes v0.1.0-alpha.0, sinktools v0.2.0-alpha.0, hydro_deploy_integration v0.17.0-alpha.0, dfir_rs v0.17.0-alpha.0, hydro_deploy v0.17.0-alpha.0, hydro_lang v0.17.0-alpha.0, hydro_std v0.17.0-alpha.0, safety bump 10 crates ([`12e7666`](https://github.com/hydro-project/hydro/commit/12e76666f7104f81b48de5ddf397b8e72c8a6711))
</details>

## v0.1.0 (2026-05-01)

<csr-id-d60ebfb6fd5b935beb5fee9b72238a815553b88e/>
<csr-id-efaa8f61c124c4b3c691b92a58df1686751cf45c/>

### New Features

 - <csr-id-9e1f9f08ee4ebce1abe814dddcdad8979fb050e3/> add local containerized deployments
 - <csr-id-f03b66b81a91f2cc9a5a7890fcf52f9adc426d57/> add LazySinkSource, a splittable lazily initialized sink and source
 - <csr-id-9927a0ea5fd9c7ca971fd3e0eae10c06bd3ef0d2/> add lazy version of demux_map

### Bug Fixes

 - <csr-id-edaa3835df9f3ad32071998f1f0daa658321e547/> fix `#[no_std]` handling
 - <csr-id-c16e13a8bdae3b099d498f9b7f1f43872cfdc939/> flag non-determinstic hashmap iterators, fix hydro_lang codegen nondeterminism fix #2464
   Out of an abundance of caution, the `hydro_lang` IR `Demux` variants
   containing `HashMap<u32 ...>` have been replaced with `BTreeMap`
 - <csr-id-045b88d66b5f74e4dacb9a28d1f75b541c26f01b/> make an unreachable state more explicitly unreachable
 - <csr-id-be5e52299c249442dac0b7b30785ef0915e76470/> make lazy source have additional trait bounds to make type inference more reliable

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 11 commits contributed to the release.
 - 156 days passed between releases.
 - 9 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 9 unique issues were worked on: [#2179](https://github.com/hydro-project/hydro/issues/2179), [#2326](https://github.com/hydro-project/hydro/issues/2326), [#2327](https://github.com/hydro-project/hydro/issues/2327), [#2329](https://github.com/hydro-project/hydro/issues/2329), [#2422](https://github.com/hydro-project/hydro/issues/2422), [#2424](https://github.com/hydro-project/hydro/issues/2424), [#2511](https://github.com/hydro-project/hydro/issues/2511), [#2525](https://github.com/hydro-project/hydro/issues/2525), [#2606](https://github.com/hydro-project/hydro/issues/2606)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#2179](https://github.com/hydro-project/hydro/issues/2179)**
    - Add local containerized deployments ([`9e1f9f0`](https://github.com/hydro-project/hydro/commit/9e1f9f08ee4ebce1abe814dddcdad8979fb050e3))
 * **[#2326](https://github.com/hydro-project/hydro/issues/2326)**
    - Add lazy version of demux_map ([`9927a0e`](https://github.com/hydro-project/hydro/commit/9927a0ea5fd9c7ca971fd3e0eae10c06bd3ef0d2))
 * **[#2327](https://github.com/hydro-project/hydro/issues/2327)**
    - Make lazy source have additional trait bounds to make type inference more reliable ([`be5e522`](https://github.com/hydro-project/hydro/commit/be5e52299c249442dac0b7b30785ef0915e76470))
 * **[#2329](https://github.com/hydro-project/hydro/issues/2329)**
    - Add LazySinkSource, a splittable lazily initialized sink and source ([`f03b66b`](https://github.com/hydro-project/hydro/commit/f03b66b81a91f2cc9a5a7890fcf52f9adc426d57))
 * **[#2422](https://github.com/hydro-project/hydro/issues/2422)**
    - Remove unused buffered_lazy_sink_source ([`d60ebfb`](https://github.com/hydro-project/hydro/commit/d60ebfb6fd5b935beb5fee9b72238a815553b88e))
 * **[#2424](https://github.com/hydro-project/hydro/issues/2424)**
    - Make an unreachable state more explicitly unreachable ([`045b88d`](https://github.com/hydro-project/hydro/commit/045b88d66b5f74e4dacb9a28d1f75b541c26f01b))
 * **[#2511](https://github.com/hydro-project/hydro/issues/2511)**
    - Flag non-determinstic hashmap iterators, fix hydro_lang codegen nondeterminism fix #2464 ([`c16e13a`](https://github.com/hydro-project/hydro/commit/c16e13a8bdae3b099d498f9b7f1f43872cfdc939))
 * **[#2525](https://github.com/hydro-project/hydro/issues/2525)**
    - Update pinned rust to 1.92, add lints/fixes for redundant cloning, string handling ([`efaa8f6`](https://github.com/hydro-project/hydro/commit/efaa8f61c124c4b3c691b92a58df1686751cf45c))
 * **[#2606](https://github.com/hydro-project/hydro/issues/2606)**
    - Fix `#[no_std]` handling ([`edaa383`](https://github.com/hydro-project/hydro/commit/edaa3835df9f3ad32071998f1f0daa658321e547))
 * **Uncategorized**
    - Release dfir_pipes v0.0.1, example_test v0.0.1, sinktools v0.1.0, hydro_deploy_integration v0.16.0, lattices_macro v0.6.0, variadics_macro v0.7.0, lattices v0.7.0, multiplatform_test v0.7.0, dfir_rs v0.16.0, copy_span v0.1.1, hydro_deploy v0.16.0, hydro_lang v0.16.0, hydro_std v0.16.0 ([`118b356`](https://github.com/hydro-project/hydro/commit/118b356447d92e778313d72a351e5a8d2814aa1a))
    - Release hydro_build_utils v0.1.0, dfir_lang v0.16.0, dfir_macro v0.16.0, variadics v0.1.0, dfir_pipes v0.0.1, example_test v0.0.1, sinktools v0.1.0, hydro_deploy_integration v0.16.0, lattices_macro v0.6.0, variadics_macro v0.7.0, lattices v0.7.0, multiplatform_test v0.7.0, dfir_rs v0.16.0, copy_span v0.1.1, hydro_deploy v0.16.0, hydro_lang v0.16.0, hydro_std v0.16.0, safety bump 13 crates ([`c20757a`](https://github.com/hydro-project/hydro/commit/c20757ae0e9e10463b2a499de4b7d37ab02269d0))
</details>

## v0.0.1 (2025-11-25)

### Bug Fixes

 - <csr-id-79253439ba9cd10886fbd4994c84b3b23113c813/> missing description in Cargo.toml

### New Features

 - <csr-id-ef6f3138bd341a6a6138078abc92e41a7ad1ed84/> add LazySink and LazySource
 - <csr-id-335ded3fc72c6525b3210f2750ea11a63d60117e/> `DemuxMap`, use in `hydro_deploy_integration`
 - <csr-id-7efe1dc660ab6c0c68762f71d4359f347cfe73b6/> `sinktools` crate
   this also converts `variadic` to use `#[no_std]`, and adds
   `feature="std"`, and fixes an issue causing trybuild tests to not run
   
   will replace `pusherator` in upcoming PR

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 6 commits contributed to the release.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 3 unique issues were worked on: [#2157](https://github.com/hydro-project/hydro/issues/2157), [#2163](https://github.com/hydro-project/hydro/issues/2163), [#2182](https://github.com/hydro-project/hydro/issues/2182)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#2157](https://github.com/hydro-project/hydro/issues/2157)**
    - `sinktools` crate ([`7efe1dc`](https://github.com/hydro-project/hydro/commit/7efe1dc660ab6c0c68762f71d4359f347cfe73b6))
 * **[#2163](https://github.com/hydro-project/hydro/issues/2163)**
    - `DemuxMap`, use in `hydro_deploy_integration` ([`335ded3`](https://github.com/hydro-project/hydro/commit/335ded3fc72c6525b3210f2750ea11a63d60117e))
 * **[#2182](https://github.com/hydro-project/hydro/issues/2182)**
    - Add LazySink and LazySource ([`ef6f313`](https://github.com/hydro-project/hydro/commit/ef6f3138bd341a6a6138078abc92e41a7ad1ed84))
 * **Uncategorized**
    - Release sinktools v0.0.1, hydro_deploy_integration v0.15.0, lattices_macro v0.5.11, variadics_macro v0.6.2, lattices v0.6.2, multiplatform_test v0.6.0, dfir_rs v0.15.0, copy_span v0.1.0, hydro_deploy v0.15.0, hydro_lang v0.15.0, hydro_std v0.15.0 ([`ac88df1`](https://github.com/hydro-project/hydro/commit/ac88df1e98af9fa2027488252f6014efa7bef229))
    - Missing description in Cargo.toml ([`7925343`](https://github.com/hydro-project/hydro/commit/79253439ba9cd10886fbd4994c84b3b23113c813))
    - Release hydro_build_utils v0.0.1, dfir_lang v0.15.0, dfir_macro v0.15.0, variadics v0.0.10, sinktools v0.0.1, hydro_deploy_integration v0.15.0, lattices_macro v0.5.11, variadics_macro v0.6.2, lattices v0.6.2, multiplatform_test v0.6.0, dfir_rs v0.15.0, copy_span v0.1.0, hydro_deploy v0.15.0, hydro_lang v0.15.0, hydro_std v0.15.0, safety bump 5 crates ([`092de25`](https://github.com/hydro-project/hydro/commit/092de252238dfb9fa6b01e777c6dd8bf9db93398))
</details>

