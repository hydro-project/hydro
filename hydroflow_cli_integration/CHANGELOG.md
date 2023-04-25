# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.0.1 (2023-04-25)

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 20 commits contributed to the release over the course of 46 calendar days.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 19 unique issues were worked on: [#452](https://github.com/MingweiSamuel/hydroflow/issues/452), [#461](https://github.com/MingweiSamuel/hydroflow/issues/461), [#466](https://github.com/MingweiSamuel/hydroflow/issues/466), [#472](https://github.com/MingweiSamuel/hydroflow/issues/472), [#477](https://github.com/MingweiSamuel/hydroflow/issues/477), [#479](https://github.com/MingweiSamuel/hydroflow/issues/479), [#484](https://github.com/MingweiSamuel/hydroflow/issues/484), [#498](https://github.com/MingweiSamuel/hydroflow/issues/498), [#513](https://github.com/MingweiSamuel/hydroflow/issues/513), [#533](https://github.com/MingweiSamuel/hydroflow/issues/533), [#541](https://github.com/MingweiSamuel/hydroflow/issues/541), [#545](https://github.com/MingweiSamuel/hydroflow/issues/545), [#560](https://github.com/MingweiSamuel/hydroflow/issues/560), [#563](https://github.com/MingweiSamuel/hydroflow/issues/563), [#575](https://github.com/MingweiSamuel/hydroflow/issues/575), [#576](https://github.com/MingweiSamuel/hydroflow/issues/576), [#584](https://github.com/MingweiSamuel/hydroflow/issues/584), [#613](https://github.com/MingweiSamuel/hydroflow/issues/613), [#617](https://github.com/MingweiSamuel/hydroflow/issues/617)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#452](https://github.com/MingweiSamuel/hydroflow/issues/452)**
    - Build CLI wheels in CI and minimize CLI dependencies ([`3e33d0c`](https://github.com/MingweiSamuel/hydroflow/commit/3e33d0cf6b068f0567e55462732598f8a4e2da6a))
 * **[#461](https://github.com/MingweiSamuel/hydroflow/issues/461)**
    - Support networking topologies that mix local and cloud through SSH tunneling ([`0ec6d88`](https://github.com/MingweiSamuel/hydroflow/commit/0ec6d889469331a212c04f9568136f770f0c973d))
 * **[#466](https://github.com/MingweiSamuel/hydroflow/issues/466)**
    - Add APIs for sending data to a Hydroflow service from Python ([`c2203a1`](https://github.com/MingweiSamuel/hydroflow/commit/c2203a15f0144308365af227f3ca044ae6a7954b))
 * **[#472](https://github.com/MingweiSamuel/hydroflow/issues/472)**
    - Some clippy/windows fixes for hydro cli code ([`bbf7b50`](https://github.com/MingweiSamuel/hydroflow/commit/bbf7b506463d7fceb5d87005c7cb270a2b0521df))
 * **[#477](https://github.com/MingweiSamuel/hydroflow/issues/477)**
    - Properly handle interrupts and fix non-flushing demux ([`00ea017`](https://github.com/MingweiSamuel/hydroflow/commit/00ea017e40b796e7561979efa0921658dfe072fd))
 * **[#479](https://github.com/MingweiSamuel/hydroflow/issues/479)**
    - Allow custom ports to be used as sinks ([`8da15b7`](https://github.com/MingweiSamuel/hydroflow/commit/8da15b7cbd8bdbf960d3ed58b69f98538ccacd2c))
 * **[#484](https://github.com/MingweiSamuel/hydroflow/issues/484)**
    - Add merge API to CLI to have multiple sources for one sink ([`e09b567`](https://github.com/MingweiSamuel/hydroflow/commit/e09b5670795292f66a004f41314c3c4aa7a24eeb))
 * **[#498](https://github.com/MingweiSamuel/hydroflow/issues/498)**
    - Add API to get CLI connection config as JSON ([`323e0f0`](https://github.com/MingweiSamuel/hydroflow/commit/323e0f0afd73b66f321b2e88498627e76a186a4e))
 * **[#513](https://github.com/MingweiSamuel/hydroflow/issues/513)**
    - Add `hydro.null` API to connect no-op sources and sinks ([`9b2a4a6`](https://github.com/MingweiSamuel/hydroflow/commit/9b2a4a690798d2a976221901fa25a908b7600f52))
 * **[#533](https://github.com/MingweiSamuel/hydroflow/issues/533)**
    - Add `hydro.mux` operator and initial API tests ([`c25272b`](https://github.com/MingweiSamuel/hydroflow/commit/c25272b90f8cc5ec7614caa29f0be889d2220510))
 * **[#541](https://github.com/MingweiSamuel/hydroflow/issues/541)**
    - Fixup! Start accepting connections in the background of CLI initialization to avoid deadlocks ([`d2480f7`](https://github.com/MingweiSamuel/hydroflow/commit/d2480f7b92a9067f63d736c6ba72f1dbc0614d0f))
    - Start accepting connections in the background of CLI initialization to avoid deadlocks ([`681a6ba`](https://github.com/MingweiSamuel/hydroflow/commit/681a6baef73de8a67e140526ede4b36e239976f0))
 * **[#545](https://github.com/MingweiSamuel/hydroflow/issues/545)**
    - Fixup! Start accepting connections in the background of CLI initialization to avoid deadlocks ([`d2480f7`](https://github.com/MingweiSamuel/hydroflow/commit/d2480f7b92a9067f63d736c6ba72f1dbc0614d0f))
 * **[#560](https://github.com/MingweiSamuel/hydroflow/issues/560)**
    - Refactor `hydro.mux` to `source.tagged(id)` and support connections where the tagged source is the server ([`3f0ecc9`](https://github.com/MingweiSamuel/hydroflow/commit/3f0ecc92abed7a0c95c04255adcc6d39c0767703))
 * **[#563](https://github.com/MingweiSamuel/hydroflow/issues/563)**
    - Don't drop the write half of a stream even if we are only reading in the CLI ([`aef43be`](https://github.com/MingweiSamuel/hydroflow/commit/aef43be9c0a3cc39229c90cde9c8f5ed8b8198c8))
 * **[#575](https://github.com/MingweiSamuel/hydroflow/issues/575)**
    - Place a buffer over each sink of a demux to avoid serial message sending ([`a26f759`](https://github.com/MingweiSamuel/hydroflow/commit/a26f759d717fd1685fc53181b4c2fd9b7b93a544))
 * **[#576](https://github.com/MingweiSamuel/hydroflow/issues/576)**
    - Add classic counter CRDT benchmark to compare against ([`2f3bf04`](https://github.com/MingweiSamuel/hydroflow/commit/2f3bf04ab33768b04d44f3f58907f958d4cd8dc8))
 * **[#584](https://github.com/MingweiSamuel/hydroflow/issues/584)**
    - Fix windows build ([`5aa96c4`](https://github.com/MingweiSamuel/hydroflow/commit/5aa96c451ba69ff2beed41528b8c847b75c45ea7))
 * **[#613](https://github.com/MingweiSamuel/hydroflow/issues/613)**
    - Improve error message when key is not found in a demux ([`f9f9e72`](https://github.com/MingweiSamuel/hydroflow/commit/f9f9e729affe41d37b2414e7c5dfc5e54caf82a7))
 * **[#617](https://github.com/MingweiSamuel/hydroflow/issues/617)**
    - Update `Cargo.toml`s for publishing ([`a78ff9a`](https://github.com/MingweiSamuel/hydroflow/commit/a78ff9aace6771787c2b72aad83be6ad8d49a828))
 * **Uncategorized**
    - Setup release workflow ([`32ef36f`](https://github.com/MingweiSamuel/hydroflow/commit/32ef36f0f4c7baecf1a31d845fee6359366ade47))
</details>

