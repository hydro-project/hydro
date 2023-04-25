# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 41 commits contributed to the release over the course of 301 calendar days.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 13 unique issues were worked on: [#162](https://github.com/hydro-project/hydroflow/issues/162), [#310](https://github.com/hydro-project/hydroflow/issues/310), [#311](https://github.com/hydro-project/hydroflow/issues/311), [#318](https://github.com/hydro-project/hydroflow/issues/318), [#329](https://github.com/hydro-project/hydroflow/issues/329), [#404](https://github.com/hydro-project/hydroflow/issues/404), [#419](https://github.com/hydro-project/hydroflow/issues/419), [#441 11/14](https://github.com/hydro-project/hydroflow/issues/441), [#441 14/14](https://github.com/hydro-project/hydroflow/issues/441), [#501](https://github.com/hydro-project/hydroflow/issues/501), [#603](https://github.com/hydro-project/hydroflow/issues/603), [#609](https://github.com/hydro-project/hydroflow/issues/609), [#617](https://github.com/hydro-project/hydroflow/issues/617)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#162](https://github.com/hydro-project/hydroflow/issues/162)**
    - SerdeGraph from parser to be callable at runtime ([`17dd150`](https://github.com/hydro-project/hydroflow/commit/17dd1500be1dab5f7abbd498d8f96b6ed00dba59))
 * **[#310](https://github.com/hydro-project/hydroflow/issues/310)**
    - Sort ops while generating the op doc in the book ([`8193409`](https://github.com/hydro-project/hydroflow/commit/8193409eff2e20c4c192b4435df609ea99ea5598))
 * **[#311](https://github.com/hydro-project/hydroflow/issues/311)**
    - Better autogen of input/output specs for ops docs ([`2cbd3e7`](https://github.com/hydro-project/hydroflow/commit/2cbd3e7757da427a47fdde74278de3ec8cbbf9fb))
 * **[#318](https://github.com/hydro-project/hydroflow/issues/318)**
    - Reintroduce code to generate streaming/blocking annotations in docs ([`9530fd7`](https://github.com/hydro-project/hydroflow/commit/9530fd7f501fd078972c6fefe3aa211f23bc1814))
 * **[#329](https://github.com/hydro-project/hydroflow/issues/329)**
    - Get hydroflow to compile to WASM ([`24354d2`](https://github.com/hydro-project/hydroflow/commit/24354d2e11c69e38e4e021aa4acf1525b376b2b1))
 * **[#404](https://github.com/hydro-project/hydroflow/issues/404)**
    - Fix op docs "blocking" to check elided port names, fix #400 ([`608e65b`](https://github.com/hydro-project/hydroflow/commit/608e65b61788376a06ab56b7f92dfd45820b4c0e))
 * **[#419](https://github.com/hydro-project/hydroflow/issues/419)**
    - Encapsulate `FlatGraph`, separate `FlatGraphBuilder` ([`fceaea5`](https://github.com/hydro-project/hydroflow/commit/fceaea5659ac76c2275c1487582a17b646858602))
 * **[#441 11/14](https://github.com/hydro-project/hydroflow/issues/441)**
    - Remove `FlatGraph`, unify under `PartitionedGraph` ([`b640b53`](https://github.com/hydro-project/hydroflow/commit/b640b532e34b29f44c768d523fbf780dba9785ff))
 * **[#441 14/14](https://github.com/hydro-project/hydroflow/issues/441)**
    - Cleanup graph docs, organize method names ([`09d3b57`](https://github.com/hydro-project/hydroflow/commit/09d3b57eb03f3920bd10f5c10277d3ef4f9cb0ec))
 * **[#501](https://github.com/hydro-project/hydroflow/issues/501)**
    - Preserve serialize diagnostics for hydroflow graph, stop emitting expected warnings in tests ([`0c810e5`](https://github.com/hydro-project/hydroflow/commit/0c810e5fdd3445923c0c7afbe651f2b4a72c115e))
 * **[#603](https://github.com/hydro-project/hydroflow/issues/603)**
    - Improve operator docs page ([`4241ca0`](https://github.com/hydro-project/hydroflow/commit/4241ca07bad6c8777b4e1a05c6c900cfa8276c81))
 * **[#609](https://github.com/hydro-project/hydroflow/issues/609)**
    - Update syn to 2.0 ([`2e7d802`](https://github.com/hydro-project/hydroflow/commit/2e7d8024f35893ef0abcb6851e370b00615f9562))
 * **[#617](https://github.com/hydro-project/hydroflow/issues/617)**
    - Update `Cargo.toml`s for publishing ([`a78ff9a`](https://github.com/hydro-project/hydroflow/commit/a78ff9aace6771787c2b72aad83be6ad8d49a828))
 * **Uncategorized**
    - Use `HydroflowGraph` for graph writing, delete `SerdeGraph` ([`d1ef14e`](https://github.com/hydro-project/hydroflow/commit/d1ef14ee459c51d5a2dd9e7ea03050772e14178c))
    - Refactor `FlatGraph` assembly into separate `FlatGraphBuilder` ([`9dd3bd9`](https://github.com/hydro-project/hydroflow/commit/9dd3bd91586966484abaf01c4330d831804b1983))
    - Emit type guards inline, configurable #263 ([`c6510da`](https://github.com/hydro-project/hydroflow/commit/c6510da4b4cb46ec026e3c1c69b5ce29b17c473c))
    - Separate surface doctests by operator ([`851d97d`](https://github.com/hydro-project/hydroflow/commit/851d97de7ba3435bac98264f4b8679973536486a))
    - Add `hydroflow_macr/build.rs` to autogen operator book docs ([`a5de404`](https://github.com/hydro-project/hydroflow/commit/a5de404cd06c10137f7584d152269327c698a65d))
    - Refactor out surface syntax diagnostics (error messages) ([`008425b`](https://github.com/hydro-project/hydroflow/commit/008425bb436042524f540fc05a855f5fa5535c76))
    - Enable partitioned output on hydroflow_parser! ([`4936198`](https://github.com/hydro-project/hydroflow/commit/4936198a2328057fa4115e38d49898bfd18fb3bb))
    - Add more tests, fix surface syntax bugs ([`eb62ef1`](https://github.com/hydro-project/hydroflow/commit/eb62ef1a47ec58abcf6a11745667e00d69df6d93))
    - Fix surface syntax macro import issues on internal doctests and examples ([`d0be1fa`](https://github.com/hydro-project/hydroflow/commit/d0be1fa76443a8adaa5a2d8ccc1f4e8a3db40280))
    - Cleanups, rename `hydroflow_core` to `hydroflow_lang` ([`c8f2b56`](https://github.com/hydro-project/hydroflow/commit/c8f2b56295555c04e8240432ff686d89fccef01c))
    - Make surface syntax macro fail early for better error messages ([`cbf8a62`](https://github.com/hydro-project/hydroflow/commit/cbf8a62ea24131b5092cb91aea5939764e681760))
    - Wip on codegen w/ some code cleanups ([`d29fb7f`](https://github.com/hydro-project/hydroflow/commit/d29fb7fc275c2774be3f5c08b75f12fdaf6970ff))
    - Add #![allow(clippy::explicit_auto_deref)] due to false positives ([`20382f1`](https://github.com/hydro-project/hydroflow/commit/20382f13d9baf49ee896a6c643bb25788aff2db0))
    - Cleanup and rearrange hydroflow_core graph code ([`49476d3`](https://github.com/hydro-project/hydroflow/commit/49476d397e14a8616e8a963451f19f9752befaa6))
    - Separate into FlatGraph and PartitionedGraph ([`13b7830`](https://github.com/hydro-project/hydroflow/commit/13b783098b5b87ff0e1819b5e5fbb16395c9e308))
    - Move hydroflow macro code into hydroflow_core ([`2623c70`](https://github.com/hydro-project/hydroflow/commit/2623c70b80431a0d6a5e3531f93e3248443af03a))
    - Fix unused method and complex type lints ([`70727c0`](https://github.com/hydro-project/hydroflow/commit/70727c04fd6062b9e6c01799dd87f94bada19cd3))
    - Display handoffs separately in mermaid ([`a913dc6`](https://github.com/hydro-project/hydroflow/commit/a913dc67655e5c934d05576e4bf8ecac9551afdf))
    - Add handling of multi-edges, insertion of handoffs ([`dc8f2db`](https://github.com/hydro-project/hydroflow/commit/dc8f2db9e02304964c2f36caa11891c971c385f7))
    - Subgraph partitioning algorithm working ([`cc8c29c`](https://github.com/hydro-project/hydroflow/commit/cc8c29ccb52e662b80989904b32bb7ef8b487c28))
    - Add checker for operator arity ([`8e7f85a`](https://github.com/hydro-project/hydroflow/commit/8e7f85a7681e62354d5640fd95a703247b984bfb))
    - Cleanup old code, add helpful comments ([`0fe0f40`](https://github.com/hydro-project/hydroflow/commit/0fe0f40dd49bcd1164032ea331f06c209de2ce16))
    - Add mermaid rendering of surface syntax ([`09c9647`](https://github.com/hydro-project/hydroflow/commit/09c964784006898825f1a91893dc20c30bc7853f))
    - New parsing with nice error messages ([`b896108`](https://github.com/hydro-project/hydroflow/commit/b896108792a809e4cbc5053d5214a891c37d330b))
    - WIP issues with keeping spans around, idx collision ([`c963290`](https://github.com/hydro-project/hydroflow/commit/c963290d0e838031d2e0b9a29ce89fb2af047629))
    - Parse updated arrow syntax ([`b7f131c`](https://github.com/hydro-project/hydroflow/commit/b7f131ce38cffc6c8491c778500ceb32d44221d8))
    - Implement basic arrow syntax ([`de8ed49`](https://github.com/hydro-project/hydroflow/commit/de8ed492c1220a131052544079085f44266fe87f))
    - Hydroflow_macro boilerplate ([`b2a8b85`](https://github.com/hydro-project/hydroflow/commit/b2a8b853907ee93ad02ceeb39b95da08a0970330))
</details>

