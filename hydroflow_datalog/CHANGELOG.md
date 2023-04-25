# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.0.1 (2023-04-25)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 34 commits contributed to the release over the course of 244 calendar days.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 17 unique issues were worked on: [#155](https://github.com/MingweiSamuel/hydroflow/issues/155), [#184](https://github.com/MingweiSamuel/hydroflow/issues/184), [#187](https://github.com/MingweiSamuel/hydroflow/issues/187), [#204](https://github.com/MingweiSamuel/hydroflow/issues/204), [#223](https://github.com/MingweiSamuel/hydroflow/issues/223), [#232](https://github.com/MingweiSamuel/hydroflow/issues/232), [#284](https://github.com/MingweiSamuel/hydroflow/issues/284), [#302](https://github.com/MingweiSamuel/hydroflow/issues/302), [#320](https://github.com/MingweiSamuel/hydroflow/issues/320), [#321](https://github.com/MingweiSamuel/hydroflow/issues/321), [#329](https://github.com/MingweiSamuel/hydroflow/issues/329), [#360](https://github.com/MingweiSamuel/hydroflow/issues/360), [#371](https://github.com/MingweiSamuel/hydroflow/issues/371), [#467](https://github.com/MingweiSamuel/hydroflow/issues/467), [#518](https://github.com/MingweiSamuel/hydroflow/issues/518), [#609](https://github.com/MingweiSamuel/hydroflow/issues/609), [#617](https://github.com/MingweiSamuel/hydroflow/issues/617)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#155](https://github.com/MingweiSamuel/hydroflow/issues/155)**
    - Add datalog frontend via a proc macro ([`fd3867f`](https://github.com/MingweiSamuel/hydroflow/commit/fd3867fde4302aabd747ca81564dfba6016a6395))
 * **[#184](https://github.com/MingweiSamuel/hydroflow/issues/184)**
    - Generate nested joins for rules with more than two RHS relations ([`863fdc8`](https://github.com/MingweiSamuel/hydroflow/commit/863fdc8fea27d3b41dd3bd94212bee515a923340))
 * **[#187](https://github.com/MingweiSamuel/hydroflow/issues/187)**
    - Emit relation filters when there are local constraints ([`28ed51b`](https://github.com/MingweiSamuel/hydroflow/commit/28ed51bcd785a9098d42d4c1e6838c95831b42f4))
 * **[#204](https://github.com/MingweiSamuel/hydroflow/issues/204)**
    - Use Rust Sitter release from crates.io ([`83ab8a5`](https://github.com/MingweiSamuel/hydroflow/commit/83ab8a500c7aad0e4f82f95199954764ed67816f))
 * **[#223](https://github.com/MingweiSamuel/hydroflow/issues/223)**
    - Add surface graph snapshot tests for datalog. ([`b235746`](https://github.com/MingweiSamuel/hydroflow/commit/b2357466115dd2fe6257da01af855840f1ff33c9))
 * **[#232](https://github.com/MingweiSamuel/hydroflow/issues/232)**
    - Extract parts of `expand_join_plan` into new functions. ([`3b79280`](https://github.com/MingweiSamuel/hydroflow/commit/3b79280d900458b38be0cbc48c669465447f4873))
 * **[#284](https://github.com/MingweiSamuel/hydroflow/issues/284)**
    - Rename source and dest surface syntax operators, fix #216 #276 ([`b7074eb`](https://github.com/MingweiSamuel/hydroflow/commit/b7074ebb5d376493b52efe471b65f6e2c06fce7c))
 * **[#302](https://github.com/MingweiSamuel/hydroflow/issues/302)**
    - Format `hydroflow_datalog` snaps w/ `prettyplease` ([`57be9a2`](https://github.com/MingweiSamuel/hydroflow/commit/57be9a21c9b407155ef9418aec48156081ba141d))
 * **[#320](https://github.com/MingweiSamuel/hydroflow/issues/320)**
    - Better mermaid graphs ([`f2ee139`](https://github.com/MingweiSamuel/hydroflow/commit/f2ee139666da9ab72093dde80812df6bc7bc0193))
 * **[#321](https://github.com/MingweiSamuel/hydroflow/issues/321)**
    - Better graphs for both mermaid and dot ([`876fb31`](https://github.com/MingweiSamuel/hydroflow/commit/876fb3140374588c55b4a7ec7a51e7cf6317eb67))
 * **[#329](https://github.com/MingweiSamuel/hydroflow/issues/329)**
    - Get hydroflow to compile to WASM ([`24354d2`](https://github.com/MingweiSamuel/hydroflow/commit/24354d2e11c69e38e4e021aa4acf1525b376b2b1))
 * **[#360](https://github.com/MingweiSamuel/hydroflow/issues/360)**
    - Preserve varnames info, display in mermaid, fix #327 ([`e7acecc`](https://github.com/MingweiSamuel/hydroflow/commit/e7acecc480fbc2031e83777f58e7eb16603b8f26))
 * **[#371](https://github.com/MingweiSamuel/hydroflow/issues/371)**
    - Get Datalog compiler to build on WASM ([`bef2435`](https://github.com/MingweiSamuel/hydroflow/commit/bef24356a9696b494f89e014aec49063892b5b5e))
 * **[#467](https://github.com/MingweiSamuel/hydroflow/issues/467)**
    - Parse error and return vector of diagnostics ([`1841f2c`](https://github.com/MingweiSamuel/hydroflow/commit/1841f2c462a132272b1f0ffac51669fc1df2f593))
 * **[#518](https://github.com/MingweiSamuel/hydroflow/issues/518)**
    - Attach spans to generated Hydroflow code in Dedalus ([`f00d865`](https://github.com/MingweiSamuel/hydroflow/commit/f00d8655aa4404ddcc812e0decf8c1e48e62b0fd))
 * **[#609](https://github.com/MingweiSamuel/hydroflow/issues/609)**
    - Update syn to 2.0 ([`2e7d802`](https://github.com/MingweiSamuel/hydroflow/commit/2e7d8024f35893ef0abcb6851e370b00615f9562))
 * **[#617](https://github.com/MingweiSamuel/hydroflow/issues/617)**
    - Update `Cargo.toml`s for publishing ([`a78ff9a`](https://github.com/MingweiSamuel/hydroflow/commit/a78ff9aace6771787c2b72aad83be6ad8d49a828))
 * **Uncategorized**
    - Setup release workflow ([`32ef36f`](https://github.com/MingweiSamuel/hydroflow/commit/32ef36f0f4c7baecf1a31d845fee6359366ade47))
    - Improve datalog diagnostic robustness ([`0b3e085`](https://github.com/MingweiSamuel/hydroflow/commit/0b3e08521131989dfaee821c060a931771936f80))
    - Add persistence lifetimes to join #272 ([`47b2941`](https://github.com/MingweiSamuel/hydroflow/commit/47b2941d74704792e5e2a7f30fa088c81c3ab506))
    - Add type guard before `Pivot` #263 ([`c215e8c`](https://github.com/MingweiSamuel/hydroflow/commit/c215e8c4523a1e465eafa3320daa34d6cb35aa11))
    - Add type guard to `merge` #263 ([`6db3f60`](https://github.com/MingweiSamuel/hydroflow/commit/6db3f6013a934b3087c8d116e61fbfc293e1baa0))
    - Emit type guards inline, configurable #263 ([`c6510da`](https://github.com/MingweiSamuel/hydroflow/commit/c6510da4b4cb46ec026e3c1c69b5ce29b17c473c))
    - Add very good type guard to `join` op #263 ([`3ee9d33`](https://github.com/MingweiSamuel/hydroflow/commit/3ee9d338c27859b31a057be53ee9251248ca235c))
    - Improve `Iterator`/`Pusherator` typeguards by erasing types, using local fns #263 ([`6413fa4`](https://github.com/MingweiSamuel/hydroflow/commit/6413fa417cab0481e3db1adbcaf71525eb866cc9))
    - Rename variadics/tuple_list macros ([`91d37b0`](https://github.com/MingweiSamuel/hydroflow/commit/91d37b022b1cd0ed590765c40ef43244027c8035))
    - Allow `clippy::uninlined-format-args` in `.cargo/config.toml` ([`17be5dd`](https://github.com/MingweiSamuel/hydroflow/commit/17be5dd3993ee3239a3fbdb81572923479b0cc3e))
    - Add parsing of named ports (WIP, compiling) ([`bd8313c`](https://github.com/MingweiSamuel/hydroflow/commit/bd8313cf59a30bb121c07d754099d92c13daa734))
    - Remove surface API, fix #224 ([`7b75f5e`](https://github.com/MingweiSamuel/hydroflow/commit/7b75f5eb73046c3fe9f50970e05b4665bc0bf7fc))
    - Update datalog snapshots ([`6d9616e`](https://github.com/MingweiSamuel/hydroflow/commit/6d9616e8740a98f16fbff84fa5b6e8295a1d9a15))
    - Update `recv_stream` to handle all `Stream`s instead of just `tokio::mpsc::unbounded_channel` ([`8b68c64`](https://github.com/MingweiSamuel/hydroflow/commit/8b68c643b55e9a04f373bded939b512be4ee0d7f))
    - Use `DiMulGraph` in `flat_to_partitioned.rs` and `PartitionedGraph`, working ([`cdd45fe`](https://github.com/MingweiSamuel/hydroflow/commit/cdd45fe8eeefaa997bc2d38386fb9d33daf47b50))
    - Update datalog codegen snapshots ([`9c9a27b`](https://github.com/MingweiSamuel/hydroflow/commit/9c9a27b42c9855ab9d725214b68d66c6c273da2b))
    - Update datalog snapshot tests ([`c252b05`](https://github.com/MingweiSamuel/hydroflow/commit/c252b0565bc86b37e5e25941ba1e9ed3c80d7863))
</details>

