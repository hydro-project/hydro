

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

 - 5 commits contributed to the release.
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
    - Missing description in Cargo.toml ([`7925343`](https://github.com/hydro-project/hydro/commit/79253439ba9cd10886fbd4994c84b3b23113c813))
    - Release hydro_build_utils v0.0.1, dfir_lang v0.15.0, dfir_macro v0.15.0, variadics v0.0.10, sinktools v0.0.1, hydro_deploy_integration v0.15.0, lattices_macro v0.5.11, variadics_macro v0.6.2, lattices v0.6.2, multiplatform_test v0.6.0, dfir_rs v0.15.0, copy_span v0.1.0, hydro_deploy v0.15.0, hydro_lang v0.15.0, hydro_std v0.15.0, safety bump 5 crates ([`092de25`](https://github.com/hydro-project/hydro/commit/092de252238dfb9fa6b01e777c6dd8bf9db93398))
</details>

