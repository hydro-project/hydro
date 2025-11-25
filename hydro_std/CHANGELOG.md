

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
 - <csr-id-194280baf21d554e2deb3c225ed9fdad197c3db2/> introduce `sliced!` syntax for processing with anonymous ticks
 - <csr-id-1fbd6e05d782388aa3542023b685172e6275baf8/> replay failed simulation instances and show logs
   Previously, a failed simulation would only show the backtrace, and would
   have to be re-run with `HYDRO_SIM_LOG=1` to display the simulation
   steps. This would result in lots of log pollution since logging would
   also be enabled for passing instances.
   
   Now, we've made some tweaks to bolero so that it re-executes the test
   after finding a failing input, passing an `is_replay` flag. We use this
   to enable rich logging, so that the failure is displayed with all the
   simulation steps that led to it.
 - <csr-id-a4bdf399f90581f409c89a72ae960405998fb33b/> add APIs for unordered stream assertions and tests for `collect_quorum`
   AI Disclosure: all the tests were generated using Kiro (!)
   
   Currently, the core functionality test has to be split up into several
   exhaustive units because otherwise the search space becomes too large
   for CI (~400s). Eventually, keyed streams may help but splitting up the
   test is a reasonable short-term fix.
 - <csr-id-224b2db7dc7e7c5cbd4c6ce024e0410957ce6747/> deterministic simulator for Hydro programs
   This introduces a deterministic simulator for Hydro that can simulate
   various asynchronous scenarios to identify bugs in code that use
   non-deterministic operators, such as `batch`. This PR focuses on just
   the infrastructure and support for simulating `batch` on a
   totally-ordered, exactly-once stream. Support for additional
   non-deterministic operators will follow in separate PRs (currently, an
   exception is thrown to prevent use of the simulator on programs that use
   such operators).
   
   The simulator's job is to explore the space of potential asynchronous
   executions. Because "top-level" operators guarantee "eventual
   determinism" (per Flo), we do not need to simulate every possible
   interleaving of message arrivals and processing. Instead, we only need
   to simulate sources of non-determinism at the points in the program
   where a user intentionally observes them (such as `batch` or
   `assume_ordering`).
   
   When compiling a Hydro program for the simulator, we emit several DFIR
   programs. One of these is the `async_dfir`, which contains all
   asynchronously executed top-level operators in the Hydro program. Again,
   thanks to Flo semantics, we do not need to simulate the behavior of
   executing these operators on different prefixes of the input, since we
   know that none of the downstream operators change their behavior based
   on the provided prefix (this is somewhat more complicated for unbounded
   singletons, whose intermediate states are affected by the set of
   elements processed in each batch, but we will address this separately).
   
   Because each tick relies on a set of decisions being made to select
   their inputs (`batch`, `snapshot`), we emit each tick's code into a
   separate DFIR graph. The top-level simulator then schedules
   (`LaunchedSim::scheduler`) the async graph and tick graphs by always
   trying to make progress with the async graph first (so that we have the
   full set of possible inputs at each batch boundary), and when the async
   graph cannot make any further progress it selects one of the ticks,
   makes a batching decision for each of its inputs
   (`autonomous_decision`), and then executes the tick.
   
   The selection of which tick to run and which elements to release in each
   batch are driven by a source of non-determinism, which is either:
   a) libfuzzer (if using `.sim().fuzz` and running with `cargo sim`)
   b) a RNG with 8192 iterations (if using `.sim().fuzz` and running with
   `cargo test` and no reproducer is available)
   c) a static input of decisions (if using `.sim().fuzz` and running with
   `cargo test` and a reproducer is available)
   d) an exhaustive, depth-first search algorithm (if using
   `.sim().exhaustive` and running with `cargo test`)
   
   Whenever a fuzzer finds a failure, it stores the sequence of decisions
   that leads to the crash in a `.bin` file as a reproducer, so that we can
   re-execute quickly in testing environments.
   
   Because Hydro uses a staged compilation model, our approach to compiling
   and executing the Hydro program is also a bit unique. Because the fuzzer
   needs to track branch coverage inside the Hydro program to be effective,
   and because we need low-latency interactions between the user assertions
   and the Hydro code, we cannot run the compiled program in a separate
   process. Instead, we compile the Hydro code into a _shared library_ and
   dynamically load it into the host process (which has the testing code).
   The shared library only provides the compiled DFIR graphs, so the
   simulator scheduler runs in the host process to enable low-latency
   switching between the DFIR and testing code.
   
   This PR includes a couple of toy examples testing the simulator's
   functionality. Of particular interest is
   `sim_crash_with_fuzzed_batching`, which takes forever with an exhaustive
   search but quickly finds a failure case with a fuzzer.
 - <csr-id-b412fa0af43d011c527eaa21d2343d57e1c941c2/> Generalize bench_client's workload generation
   Allow custom functions for generating bench_client workloads (beyond
   u32,u32).
 - <csr-id-920d9a394223a218ccbc077de61e2135886a2b15/> add keyed singletons and optionals to eliminate unsafety in membership tracking

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

### Refactor

 - <csr-id-0be5729dd87a91a70001f88283b380d3da8df7d0/> reduce syntactic overhead of connecting test inputs / outputs
   Rather than having separate `source` / `sink` / `bytes` / `bincode`
   APIs, we use a single `connect` method that uses a trait to resolve the
   appropriate connection result.
 - <csr-id-057192afde1373caedbbfc24516c28a96d12928c/> reduce atomic pollution in quorum counting
   Also fixes a sync bug in Compartmentalized Paxos. Due to batching
   semantics, we could end up in a situation where responses have missing
   metadata (for example if the batch refuses to release any elements).
   
   The simulator would hopefully have caught this, we can use this as an
   example.
 - <csr-id-2e2cd770fd18cd219ec1acdd2c74d46a5ee1b2de/> remove uses of legacy `*_keyed` APIs
   Also adds missing doctests to the aggregation APIs on `KeyedStream`.

### Test

 - <csr-id-1fc751515d5fd4b6ec07fec8e83b4aff70b3acca/> add test for collecting unordered quorum

### Documentation (BREAKING)

 - <csr-id-1af3a666b1d0787f0c023411b7f88ad3f8da5423/> add docs for `for_each` / `dest_sink`, restrict to strict streams
   Breaking Change: `for_each` / `dest_sink` now require a totally-ordered,
   retry-free stream since the downstream may not tolerate such
   non-determinism. This helps highlight cases where the developer needs to
   reason carefully about the safety of their side effects.

### New Features (BREAKING)

 - <csr-id-6b1d66aa61056ffb4dd2896e98288489e64d654f/> add specialized `sim_input` / `sim_output` to reduce simulation boilerplate
 - <csr-id-5579acd1c7101a3f14c49236e1933398de0f0958/> cluster member ids are now clone instead of copy
   This is part one of a series of changes. The first part just changes
   MemberIds from being Copy to Clone, so that they can later support more
   use cases.
 - <csr-id-a15797058950245d9eb762d885d35e5326bcf8b3/> restrict `KeyedSingleton` to not allow asynchronous key removal
   Previously, `KeyedSingleton` supported `filter` on unbounded collections
   and `latest` to yield snapshots from a tick. Both of these APIs are
   problematic because they let developers asynchronously remove elements
   from the collection, which goes against the "Singleton" naming. We have
   reasonable alternatives for all these use-sites, so remove for now to
   minimize the semantic complexity (also for the simulator).
   
   Eventually, we will want to bring back `KeyedOptional` which permits
   such asynchronous removal.
 - <csr-id-e535156ceb7e83550b68a6ae6b5e925f57f1882f/> add `Ordering` and `Retries` traits with basic axioms
   Reduces the need to use `nondet!` inside broadcast APIs, and also
   prepares us for graph viz metadata by requiring these traits to be
   implemented for the type parameters in `Stream` / `KeyedStream`.
 - <csr-id-eadd7d0cfcbdc312bf336b8aec961f6d421c6551/> refine types when values of a `KeyedSingleton` are immutable
   There are many cases where once a value is released into a
   `KeyedSingleton`, it will never be changes. In such cases, we can permit
   APIs like `.entries` even if the set of entries is growing.
   
   We use this type to improve the API surface for looking up values for a
   request. Now, a `KeyedSingleton` of requests can look up values from
   another `KeyedSingleton`.

### Bug Fixes (BREAKING)

 - <csr-id-21ce30cdd04a25bf4a67e00ec16e592183748bf4/> fix cardinality for `Optional::or`
   Using `HydroNode::Chain` is very dangerous for singletons / optionals,
   because it can lead to cardinality > 1 within a single batch, which
   breaks a fundamental invariant.
   
   This introduces a new `HydroNode::ChainFirst` operator that only emits
   the first value from the chain. This is paired with a new DFIR operator
   `chain_first` for this behavior.
   
   We also rewrite some `Singleton` logic to use `Optional` under the hood,
   which reduces the places where we deal with chaining in the IR,
   hopefully avoiding future incidents.

### Refactor (BREAKING)

 - <csr-id-b256cba932a8d6d7a6be7b1c98c2f8c20b299375/> don't return results from `SimSender::send`
   Also makes the `assert_*` APIs on `SimReceiver` more general to support
   asymmetric `PartialEq`
 - <csr-id-fa4e9d9914ed52aa5a7237c32a0dc57d713ec14a/> allow `sim` feature without `deploy` feature
   Also removes leftover prototype "properties" code.
 - <csr-id-a4d8af603e6ad14659d1d43ca168495c883a58eb/> migrate Hydro IR to have Flo semantics at all levels
   Previously, the Hydro -> Hydro IR layer did a bunch of work to translate
   between Flo semantics and DFIR semantics, so that Hydro IR -> DFIR was
   generally 1:1. In preparation for the simulator and DFIR's migration to
   Flo semantics, we now use Flo semantics in the Hydro IR (top level
   operators do not reset their state on batch boundaries), and shift the
   Flo -> DFIR semantics translation to the IR -> DFIR layer.
   
   This also comes with a breaking API change to clean up naming and avoid
   function overloading. In particular, `batch` / `snapshot` are renamed to
   `batch_atomic` and `snapshot_atomic` for the case where the batched
   result is supposed to be in-sync with the atomic execution context.
   
   There were a couple of bugs during the transition to the new IR
   semantics that were not caught by existing unit tests, so I added
   additional tests that cover each of them.
   
   Also makes some minor improvements / bug fixes the way:
   - Restricts `KeyedSingleton::snapshot` to unbounded-value only, since
   bounded value keyed singletons do not replay their elements
   - Groups together definitions for `resolve_futures` and
   `resolve_futures_ordered`
   - Removes various unnecessary `NoAtomic` trait bounds
 - <csr-id-537309f9aac44498aa617c8517fdbc21616cbebf/> rename `continue_if*` to `filter_if*` and document
   Eventually, we will want to change these APIs to take a "Condition" live
   collection instead of an optional, but this at least improves the naming
   for now.
 - <csr-id-5f8a4da212eba8b673f9c7a464c9e92d7c0602cd/> clean up API for cycles / forward references, document
   Ongoing quest to reduce the public API surface. Moves the `DeferTick` IR
   node to be at the root of the output side of the cycle, rather than
   inserted at the sink.
 - <csr-id-4bf1c05583c838e7e4d183382fd72743402f889d/> move `boundedness` module under `live_collections`
   Type tags for boundedness are only used in the type parameters for live
   collections.
 - <csr-id-05145bf191bf0fcc794d282c3b18c0bd378a20ac/> set up prelude, move collections under `live_collections` module
   Bigger refactor to clean up the top-level namespace for `hydro_lang`.
   First, we move all the definitions for different live collections into
   the `live_collections` module.
   
   We also rename the `unsafety` module to `nondet`.
   
   Then, instead of having a bunch of types exported directly from
   `hydro_lang`, we instead create a `prelude` module that exports them.
   This reduces the pollution of the namespace when importing from
   `hydro_lang`.
 - <csr-id-4925e2c77a8e57d45d200c98a31859571a04d150/> reduce namespace pollution
 - <csr-id-628c1c870f1833dd05b3f57ee3e2e1235183cecb/> rename `ClusterId` to `MemberId`
   Will allow us to use this for external clients as well.
 - <csr-id-1c26bc7899f29cb5b75446381ac5545f7ce017d8/> adjust cluster membership APIs to allow dynamic clusters
 - <csr-id-381be86c8729403d60575bbd7297b852b6b09ec0/> remove `KeyedOptional`
   Keyed optionals don't make sense to distinguish from keyed singleton,
   since a null optional is equivalent to the key not being present at all.
   So we can capture all cases with just a keyed singleton.
   
   Eventually... we may want to re-distinguish these if we have keys with a
   null value. But right now we don't need that so avoid unnecessary code.
 - <csr-id-804f9955dfc9ea64cb0f5177bcda5b9347fafe80/> nondet instead of unsafe, snapshot instead of latest_tick, batch instead of tick_batch
   Pulled together into one big PR to avoid conflicts, this makes three
   significant changes to the core Hydro APIs. No semantic changes, only
   syntax and naming.
   
   1. Instead of non-deterministic functions being marked with the `unsafe`
   keyword, which conflates non-determinism with memory unsafety, we now
   pass around "non-determinism guards" that act as tokens forcing
   developers to be aware of the non-determinism. With the VSCode Highlight
   extension, we preserve highlighting of these instances.
   2. We unify naming for "collection-like" types (Stream, KeyedStream) to
   use `.batch` to batch elements
   3. We unify naming for "floating-value-like" types (Singleton, Optional,
   KeyedSingleton, KeyedOptional) to use `.snapshot` to grab an instance of
   the value (or its contents)
 - <csr-id-1a344a98fce99d004e0ba86a67c7509d807c37bb/> simplify networking APIs according to Process/Cluster types
   Breaking change: when sending to a cluster, you must use
   `demux_bincode`. The `*_anonymous` APIs have been removed in favor of
   the `.values()` API on keyed streams. This also eliminates the
   `send_bytes` APIs, in favor of the bidirectional external client APIs.
   
   This refactors the networking APIs to rely less on complex traits and
   instead use `Process` and `Cluster` types to determine the input /
   output type restrictions. We also now emit `KeyedStream` whenever the
   sender is a `Cluster`.

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 37 commits contributed to the release.
 - 36 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 36 unique issues were worked on: [#1970](https://github.com/hydro-project/hydro/issues/1970), [#1975](https://github.com/hydro-project/hydro/issues/1975), [#1983](https://github.com/hydro-project/hydro/issues/1983), [#1984](https://github.com/hydro-project/hydro/issues/1984), [#1990](https://github.com/hydro-project/hydro/issues/1990), [#1995](https://github.com/hydro-project/hydro/issues/1995), [#2011](https://github.com/hydro-project/hydro/issues/2011), [#2016](https://github.com/hydro-project/hydro/issues/2016), [#2028](https://github.com/hydro-project/hydro/issues/2028), [#2032](https://github.com/hydro-project/hydro/issues/2032), [#2033](https://github.com/hydro-project/hydro/issues/2033), [#2035](https://github.com/hydro-project/hydro/issues/2035), [#2060](https://github.com/hydro-project/hydro/issues/2060), [#2067](https://github.com/hydro-project/hydro/issues/2067), [#2073](https://github.com/hydro-project/hydro/issues/2073), [#2075](https://github.com/hydro-project/hydro/issues/2075), [#2099](https://github.com/hydro-project/hydro/issues/2099), [#2104](https://github.com/hydro-project/hydro/issues/2104), [#2108](https://github.com/hydro-project/hydro/issues/2108), [#2111](https://github.com/hydro-project/hydro/issues/2111), [#2135](https://github.com/hydro-project/hydro/issues/2135), [#2136](https://github.com/hydro-project/hydro/issues/2136), [#2140](https://github.com/hydro-project/hydro/issues/2140), [#2158](https://github.com/hydro-project/hydro/issues/2158), [#2172](https://github.com/hydro-project/hydro/issues/2172), [#2173](https://github.com/hydro-project/hydro/issues/2173), [#2181](https://github.com/hydro-project/hydro/issues/2181), [#2209](https://github.com/hydro-project/hydro/issues/2209), [#2219](https://github.com/hydro-project/hydro/issues/2219), [#2227](https://github.com/hydro-project/hydro/issues/2227), [#2243](https://github.com/hydro-project/hydro/issues/2243), [#2256](https://github.com/hydro-project/hydro/issues/2256), [#2265](https://github.com/hydro-project/hydro/issues/2265), [#2272](https://github.com/hydro-project/hydro/issues/2272), [#2293](https://github.com/hydro-project/hydro/issues/2293), [#2304](https://github.com/hydro-project/hydro/issues/2304)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#1970](https://github.com/hydro-project/hydro/issues/1970)**
    - Generalize bench_client's workload generation ([`b412fa0`](https://github.com/hydro-project/hydro/commit/b412fa0af43d011c527eaa21d2343d57e1c941c2))
 * **[#1975](https://github.com/hydro-project/hydro/issues/1975)**
    - Simplify networking APIs according to Process/Cluster types ([`1a344a9`](https://github.com/hydro-project/hydro/commit/1a344a98fce99d004e0ba86a67c7509d807c37bb))
 * **[#1983](https://github.com/hydro-project/hydro/issues/1983)**
    - Add keyed singletons and optionals to eliminate unsafety in membership tracking ([`920d9a3`](https://github.com/hydro-project/hydro/commit/920d9a394223a218ccbc077de61e2135886a2b15))
 * **[#1984](https://github.com/hydro-project/hydro/issues/1984)**
    - Remove uses of legacy `*_keyed` APIs ([`2e2cd77`](https://github.com/hydro-project/hydro/commit/2e2cd770fd18cd219ec1acdd2c74d46a5ee1b2de))
 * **[#1990](https://github.com/hydro-project/hydro/issues/1990)**
    - Nondet instead of unsafe, snapshot instead of latest_tick, batch instead of tick_batch ([`804f995`](https://github.com/hydro-project/hydro/commit/804f9955dfc9ea64cb0f5177bcda5b9347fafe80))
 * **[#1995](https://github.com/hydro-project/hydro/issues/1995)**
    - Remove strange use of batching in bench_client ([`ab22c44`](https://github.com/hydro-project/hydro/commit/ab22c44aaabf2140315ba26104d9155e357a34ac))
 * **[#2011](https://github.com/hydro-project/hydro/issues/2011)**
    - Remove `KeyedOptional` ([`381be86`](https://github.com/hydro-project/hydro/commit/381be86c8729403d60575bbd7297b852b6b09ec0))
 * **[#2016](https://github.com/hydro-project/hydro/issues/2016)**
    - Adjust cluster membership APIs to allow dynamic clusters ([`1c26bc7`](https://github.com/hydro-project/hydro/commit/1c26bc7899f29cb5b75446381ac5545f7ce017d8))
 * **[#2028](https://github.com/hydro-project/hydro/issues/2028)**
    - Refactor github actions workflows, make stable the default toolchain ([`c40876e`](https://github.com/hydro-project/hydro/commit/c40876ec4bd3b31254d683e479b9a235f3d11f67))
 * **[#2032](https://github.com/hydro-project/hydro/issues/2032)**
    - Rename `ClusterId` to `MemberId` ([`628c1c8`](https://github.com/hydro-project/hydro/commit/628c1c870f1833dd05b3f57ee3e2e1235183cecb))
 * **[#2033](https://github.com/hydro-project/hydro/issues/2033)**
    - Refine types when values of a `KeyedSingleton` are immutable ([`eadd7d0`](https://github.com/hydro-project/hydro/commit/eadd7d0cfcbdc312bf336b8aec961f6d421c6551))
 * **[#2035](https://github.com/hydro-project/hydro/issues/2035)**
    - Client aggregator waits for all clients before outputting ([`f9e5955`](https://github.com/hydro-project/hydro/commit/f9e595559b3dd9641ed6f413c2a729047ebf353e))
 * **[#2060](https://github.com/hydro-project/hydro/issues/2060)**
    - Reduce namespace pollution ([`4925e2c`](https://github.com/hydro-project/hydro/commit/4925e2c77a8e57d45d200c98a31859571a04d150))
 * **[#2067](https://github.com/hydro-project/hydro/issues/2067)**
    - Set up prelude, move collections under `live_collections` module ([`05145bf`](https://github.com/hydro-project/hydro/commit/05145bf191bf0fcc794d282c3b18c0bd378a20ac))
 * **[#2073](https://github.com/hydro-project/hydro/issues/2073)**
    - Move `boundedness` module under `live_collections` ([`4bf1c05`](https://github.com/hydro-project/hydro/commit/4bf1c05583c838e7e4d183382fd72743402f889d))
 * **[#2075](https://github.com/hydro-project/hydro/issues/2075)**
    - Clean up API for cycles / forward references, document ([`5f8a4da`](https://github.com/hydro-project/hydro/commit/5f8a4da212eba8b673f9c7a464c9e92d7c0602cd))
 * **[#2099](https://github.com/hydro-project/hydro/issues/2099)**
    - Add `Ordering` and `Retries` traits with basic axioms ([`e535156`](https://github.com/hydro-project/hydro/commit/e535156ceb7e83550b68a6ae6b5e925f57f1882f))
 * **[#2104](https://github.com/hydro-project/hydro/issues/2104)**
    - Add docs for `for_each` / `dest_sink`, restrict to strict streams ([`1af3a66`](https://github.com/hydro-project/hydro/commit/1af3a666b1d0787f0c023411b7f88ad3f8da5423))
 * **[#2108](https://github.com/hydro-project/hydro/issues/2108)**
    - Fix cardinality for `Optional::or` ([`21ce30c`](https://github.com/hydro-project/hydro/commit/21ce30cdd04a25bf4a67e00ec16e592183748bf4))
 * **[#2111](https://github.com/hydro-project/hydro/issues/2111)**
    - Rename `continue_if*` to `filter_if*` and document ([`537309f`](https://github.com/hydro-project/hydro/commit/537309f9aac44498aa617c8517fdbc21616cbebf))
 * **[#2135](https://github.com/hydro-project/hydro/issues/2135)**
    - Staggered client ([`1c13977`](https://github.com/hydro-project/hydro/commit/1c139772baa20d5c9aa8ab060cc6650d6f239ca0))
 * **[#2136](https://github.com/hydro-project/hydro/issues/2136)**
    - Migrate Hydro IR to have Flo semantics at all levels ([`a4d8af6`](https://github.com/hydro-project/hydro/commit/a4d8af603e6ad14659d1d43ca168495c883a58eb))
 * **[#2140](https://github.com/hydro-project/hydro/issues/2140)**
    - Reduce atomic pollution in quorum counting ([`057192a`](https://github.com/hydro-project/hydro/commit/057192afde1373caedbbfc24516c28a96d12928c))
 * **[#2158](https://github.com/hydro-project/hydro/issues/2158)**
    - Deterministic simulator for Hydro programs ([`224b2db`](https://github.com/hydro-project/hydro/commit/224b2db7dc7e7c5cbd4c6ce024e0410957ce6747))
 * **[#2172](https://github.com/hydro-project/hydro/issues/2172)**
    - Reduce syntactic overhead of connecting test inputs / outputs ([`0be5729`](https://github.com/hydro-project/hydro/commit/0be5729dd87a91a70001f88283b380d3da8df7d0))
 * **[#2173](https://github.com/hydro-project/hydro/issues/2173)**
    - Add APIs for unordered stream assertions and tests for `collect_quorum` ([`a4bdf39`](https://github.com/hydro-project/hydro/commit/a4bdf399f90581f409c89a72ae960405998fb33b))
 * **[#2181](https://github.com/hydro-project/hydro/issues/2181)**
    - Replay failed simulation instances and show logs ([`1fbd6e0`](https://github.com/hydro-project/hydro/commit/1fbd6e05d782388aa3542023b685172e6275baf8))
 * **[#2209](https://github.com/hydro-project/hydro/issues/2209)**
    - Allow `sim` feature without `deploy` feature ([`fa4e9d9`](https://github.com/hydro-project/hydro/commit/fa4e9d9914ed52aa5a7237c32a0dc57d713ec14a))
 * **[#2219](https://github.com/hydro-project/hydro/issues/2219)**
    - Restrict `KeyedSingleton` to not allow asynchronous key removal ([`a157970`](https://github.com/hydro-project/hydro/commit/a15797058950245d9eb762d885d35e5326bcf8b3))
 * **[#2227](https://github.com/hydro-project/hydro/issues/2227)**
    - New template and walkthrough ([`3fd84aa`](https://github.com/hydro-project/hydro/commit/3fd84aa8812fa027db293727d8e304708db66916))
 * **[#2243](https://github.com/hydro-project/hydro/issues/2243)**
    - Don't return results from `SimSender::send` ([`b256cba`](https://github.com/hydro-project/hydro/commit/b256cba932a8d6d7a6be7b1c98c2f8c20b299375))
 * **[#2256](https://github.com/hydro-project/hydro/issues/2256)**
    - Introduce `sliced!` syntax for processing with anonymous ticks ([`194280b`](https://github.com/hydro-project/hydro/commit/194280baf21d554e2deb3c225ed9fdad197c3db2))
 * **[#2265](https://github.com/hydro-project/hydro/issues/2265)**
    - Cluster member ids are now clone instead of copy ([`5579acd`](https://github.com/hydro-project/hydro/commit/5579acd1c7101a3f14c49236e1933398de0f0958))
 * **[#2272](https://github.com/hydro-project/hydro/issues/2272)**
    - Add cluster-membership ir node ([`98b899b`](https://github.com/hydro-project/hydro/commit/98b899baa342617bc8634220324849b1067f6233))
 * **[#2293](https://github.com/hydro-project/hydro/issues/2293)**
    - Add test for collecting unordered quorum ([`1fc7515`](https://github.com/hydro-project/hydro/commit/1fc751515d5fd4b6ec07fec8e83b4aff70b3acca))
 * **[#2304](https://github.com/hydro-project/hydro/issues/2304)**
    - Add specialized `sim_input` / `sim_output` to reduce simulation boilerplate ([`6b1d66a`](https://github.com/hydro-project/hydro/commit/6b1d66aa61056ffb4dd2896e98288489e64d654f))
 * **Uncategorized**
    - Release hydro_build_utils v0.0.1, dfir_lang v0.15.0, dfir_macro v0.15.0, variadics v0.0.10, sinktools v0.0.1, hydro_deploy_integration v0.15.0, lattices_macro v0.5.11, variadics_macro v0.6.2, lattices v0.6.2, multiplatform_test v0.6.0, dfir_rs v0.15.0, copy_span v0.1.0, hydro_deploy v0.15.0, hydro_lang v0.15.0, hydro_std v0.15.0, safety bump 5 crates ([`092de25`](https://github.com/hydro-project/hydro/commit/092de252238dfb9fa6b01e777c6dd8bf9db93398))
</details>

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

 - 6 commits contributed to the release.
 - 110 days passed between releases.
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
    - Release example_test v0.0.0, dfir_rs v0.14.0, hydro_deploy v0.14.0, hydro_lang v0.14.0, hydro_optimize v0.13.0, hydro_std v0.14.0 ([`5f69ee0`](https://github.com/hydro-project/hydro/commit/5f69ee080a9e257bc07cdc4deda90ce5525a3d0e))
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

