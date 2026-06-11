

## v0.1.0-alpha.1 (2026-06-11)

### Chore

 - <csr-id-e70eab6a0c793ef095e2cd747220d5419f7bf1a4/> revert accidental `v1.0.0-alpha.0` releases of `dfir_lang` & `variadics`, update `cargo-smart-release` fork version

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 1 commit contributed to the release.
 - 1 commit was understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Revert accidental `v1.0.0-alpha.0` releases of `dfir_lang` & `variadics`, update `cargo-smart-release` fork version ([`e70eab6`](https://github.com/hydro-project/hydro/commit/e70eab6a0c793ef095e2cd747220d5419f7bf1a4))
</details>

## v0.1.0-alpha.0 (2026-06-10)

### Bug Fixes

 - <csr-id-56ca7e115f20bfacbc4b401bab71e6a35cad13ce/> forward `size_hint` from pull to push in `SendPush` pivot [ci-bench]
   Before starting the pull loop, forward the pull's `size_hint` to the
   push
   side so terminal operators like `VecPush` can pre-allocate capacity via
   `Vec::reserve`. This avoids repeated doubling reallocations when the
   input
   size is known (e.g., draining a handoff buffer with known length).
   
   The hint is forwarded once per `SendPush` future (guarded by a bool
   flag)
   to avoid redundant reserve calls on subsequent polls.

### Bug Fixes (BREAKING)

 - <csr-id-d1f89203215cb223aac3aa9cba27e487d1b46c24/> fix broken type inference for `demux_var`, used in `partition` codegen

### Refactor (BREAKING)

 - <csr-id-eca38c8a5e9c23ff652ce6af1079a8b34988c01a/> Add two mandatory output ports (`[items]`, `[state]`) to `state`/`state_by` operators
   This is part of a small detour to remove the old DFIR singleton system
   while keeping these operators around. Alternative would be to just
   delete them.
   
   The `state` and `state_by` lattice operators now have two fixed output
   ports:
   - `[items]`: emits the input items that actually changed the lattice
   (deltas), same as the old single output
   - `[state]`: emits a clone of the accumulated lattice value after all
   items are processed
   
   This separates the two semantically different outputs that were
   previously conflated:
   the delta stream (for downstream dataflow) and the accumulated state
   (for singleton
   references). The `[state]` port can now be piped into `singleton()` for
   the new
   reference system, decoupling the lattice state operator from the old
   `has_singleton_output` mechanism.
   
   Key implementation details:
   - `state_by` uses a custom inline `StatePush` struct implementing the
   `Push` trait
     to handle both outputs in a single push combinator
   - Items are filtered to `[items]` on each `start_send`; state is emitted
   to `[state]`
     during `poll_finalize`
   - `ports_out` is set to `Fixed(parse_quote!(items, state))` enforcing
   both ports
   - `has_singleton_output: true` is kept so the old `#var` reference
   system still works
   - Push sink `let` bindings in `meta_graph.rs` changed to `let mut`
   (needed for pin projection)
   
   All existing tests updated to use the new port syntax. Unused `[state]`
   ports are
   directed to `null()`.
 - <csr-id-bfec02db5b1176505770eb30db6d0ce537696f8b/> rename Push::poll_flush to Push::poll_finalize in dfir_pipes

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 5 commits contributed to the release.
 - 40 days passed between releases.
 - 4 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 4 unique issues were worked on: [#2853](https://github.com/hydro-project/hydro/issues/2853), [#2854](https://github.com/hydro-project/hydro/issues/2854), [#2881](https://github.com/hydro-project/hydro/issues/2881), [#2883](https://github.com/hydro-project/hydro/issues/2883)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#2853](https://github.com/hydro-project/hydro/issues/2853)**
    - Rename Push::poll_flush to Push::poll_finalize in dfir_pipes ([`bfec02d`](https://github.com/hydro-project/hydro/commit/bfec02db5b1176505770eb30db6d0ce537696f8b))
 * **[#2854](https://github.com/hydro-project/hydro/issues/2854)**
    - Fix broken type inference for `demux_var`, used in `partition` codegen ([`d1f8920`](https://github.com/hydro-project/hydro/commit/d1f89203215cb223aac3aa9cba27e487d1b46c24))
 * **[#2881](https://github.com/hydro-project/hydro/issues/2881)**
    - Forward `size_hint` from pull to push in `SendPush` pivot [ci-bench] ([`56ca7e1`](https://github.com/hydro-project/hydro/commit/56ca7e115f20bfacbc4b401bab71e6a35cad13ce))
 * **[#2883](https://github.com/hydro-project/hydro/issues/2883)**
    - Add two mandatory output ports (`[items]`, `[state]`) to `state`/`state_by` operators ([`eca38c8`](https://github.com/hydro-project/hydro/commit/eca38c8a5e9c23ff652ce6af1079a8b34988c01a))
 * **Uncategorized**
    - Release hydro_build_utils v0.1.1-alpha.0, dfir_lang v1.0.0-alpha.0, dfir_macro v0.17.0-alpha.0, variadics v1.0.0-alpha.0, variadics_macro v0.8.0-alpha.0, lattices v0.8.0-alpha.0, dfir_pipes v0.1.0-alpha.0, sinktools v0.2.0-alpha.0, hydro_deploy_integration v0.17.0-alpha.0, dfir_rs v0.17.0-alpha.0, hydro_deploy v0.17.0-alpha.0, hydro_lang v0.17.0-alpha.0, hydro_std v0.17.0-alpha.0, safety bump 10 crates ([`12e7666`](https://github.com/hydro-project/hydro/commit/12e76666f7104f81b48de5ddf397b8e72c8a6711))
</details>

## v0.0.1 (2026-05-01)

<csr-id-bbe8617b47a059d36e55ac1be1940023083cf6cb/>
<csr-id-be35ffa266cf564cf967bb653720dc664b24b813/>
<csr-id-d0f9d3b5ae0b281401c78702f28399fcec5a6fcd/>
<csr-id-f76b0224d62af0d4ac34a7386e45c52d4f58a81a/>
<csr-id-db8d7f1a7fb2556302d92ff77dbb40beef8f3a44/>
<csr-id-3e6e26c4cc87d6f7857591b10876074cba97caff/>

### Chore

 - <csr-id-c9d078e533de7ee7d85b5f4c31c9cc049fb230e3/> fix dfir_pipes/Cargo.toml

### Chore

 - <csr-id-c9d078e533de7ee7d85b5f4c31c9cc049fb230e3/> fix dfir_pipes/Cargo.toml

### New Features

 - <csr-id-eacc3cd85a0a9bdd70c9f6b9da4da312059f3c5a/> Add scan_async_blocking operator to hydro_lang and dfir
 - <csr-id-128e85a7b6980f76575ad0e7306f34e7de0076c6/> resolve_future[s]_blocking for streams, singleton
 - <csr-id-892bcb67cc7789db6f749c2a5c38796bcb17f14a/> Add pull::FlatMapStream operator to dfir_pipes
   Implements the missing `pull::FlatMapStream` combinator, completing the
   2x2 matrix of pull/push × flatten/flat_map stream operators.
   
   - Created `dfir_pipes/src/pull/flat_map_stream.rs` with the
   `FlatMapStream` struct that maps each item to a `Stream` via a closure
   and flattens the results by polling each inner stream. Modeled after the
   existing `pull::FlattenStream` (for structure/pinning) and
   `pull::FlatMap` (for the closure pattern).
   - Registered the module and re-export in `dfir_pipes/src/pull/mod.rs`.
   - Added `flat_map_stream` method to the `Pull` trait.
   - Includes `FusedPull` impl and two tests (basic + pending propagation).
 - <csr-id-9aac9a74004e99d0c7dd4a752322fdb7008998e0/> Add Pull version of FlattenStream operator to dfir_pipes
   Created `dfir_pipes/src/pull/flatten_stream.rs` implementing a
   pull-based `FlattenStream` combinator. This operator takes a pull whose
   items are `futures_core::Stream`s and flattens them by polling each
   inner stream, yielding individual elements downstream.
   
   Key design decisions:
   - Mirrors the existing push `FlattenStream` but as a pull operator
   - Follows the same pattern as the existing pull `Flatten` (for
   iterators)
   - Uses `pin_project_lite` for safe pin projection of the inner stream
   - `CanPend` is always `Yes` since inner streams may pend
   - `CanEnd` inherits from the upstream pull
   - Requires `core::task::Context` since inner streams need polling
   - Implements `FusedPull` when the upstream is `FusedPull`
   
   Also added `flatten_stream()` method to the `Pull` trait and registered
   the module/export in `pull/mod.rs`.
 - <csr-id-a0cd424a6b4b6d8d5c0f96ed4e2aeb7e6b64a2ac/> Add StreamCompat type in dfir_pipes/pull, mirroring SinkCompat in push
   Created `dfir_pipes/src/pull/stream_compat.rs` with a
   `StreamCompat<Pul>` adapter that wraps a `Pull` and implements
   `futures_core::stream::Stream`, dropping any `Meta` data
 - <csr-id-26afc34c237bd0821c5490d8777256245176692c/> Add flat_map_stream operator to dfir_pipes::push
   Created `FlatMapStream` push combinator that maps each input item to a
   stream via a user-provided function, then flattens the resulting stream
   by polling it and pushing each element downstream. This mirrors the
   relationship between `flat_map` and `flatten` but for the async stream
   variants (`flat_map_stream` is to `flatten_stream` what `flat_map` is to
   `flatten`).
 - <csr-id-b9b0322abf37ff99cdafbc80eb0df62a8c0d53c9/> Add FlattenStream push operator for flattening Stream items
   Added a new `FlattenStream<Next, St, Meta>` push combinator in
   `dfir_pipes/src/push/flatten_stream.rs` that is the async counterpart to
   the existing `Flatten` operator. While `Flatten` synchronously iterates
   over `IntoIterator` items, `FlattenStream` polls `Stream` items
   asynchronously and propagates `Poll::Pending` as `PushStep::Pending`.
   
   Key design:
   - `CanPend = Yes` since streams can pend
   - `Ctx = core::task::Context` for async polling
   - Buffers one stream and one item at a time, draining the stream in
   `poll_ready` before accepting new items
   
   Also registered the module and added a `flatten_stream` constructor
   function in `push/mod.rs`, plus two tests verifying correct drain
   behavior and pending propagation.
   
   This does not yet add the operator to `dfir_lang`/`dfir_rs`
 - <csr-id-62966dd0fa22ba6a60dc7f41cebda6165eb4990b/> Add `SinkCompat` to turn `Push` into `Sink`, replaces `PendingFlushSink` test, various cleanups

### Bug Fixes

 - <csr-id-0fe74c4025e2b402dc46df29c21855e8ad635fde/> fix Zip/ZipLongest size_hint and starvation bugs, fix #2664
   Both `Zip` and `ZipLongest` had two bugs:
   
   1. `size_hint()` did not account for the buffered `item1` field, so the
   reported remaining count could be off by one when an item was already
   fetched from one stream but the other stream returned Pending.
   
   2. The buffer only ever stored items from stream 1 (`item1`), meaning
   stream 2 could starve if stream 1 was always pending — stream 2 would
   never get polled.

### New Features (BREAKING)

 - <csr-id-52ed1062f8fb30b9b2ec8f4615d9187bba62e2b0/> Add `Push::size_hint`, `VecPush` terminal operator, use in dfir codegen [ci-bench]
   Added `size_hint(self: Pin<&mut Self>, hint: (usize, Option<usize>))` to
   the `Push` trait as the push-side analog of `Pull::size_hint`. This
   allows producers to announce how many items they're about to send,
   enabling downstream operators and sinks to pre-allocate.
   
   New terminal operator:
   - `VecPush<Buf>`: pushes items into a `Vec`, uses `size_hint` to call
   `Vec::reserve(hint.0)` for pre-allocation. Gated on `alloc` feature.
   - Constructor: `push::vec_push(buf)` creates a VecPush from a `&mut
   Vec<T>`.
 - <csr-id-a662ff38541e58bec801644b81b2bfc505779e7b/> use custom `dfir_pipes::Pull` trait [ci-bench]
   This is the pull-half of a big change from using other iterators
   (`std::iter::Iterator` or `futures_core::stream::Stream`) to our own
   `Pull` trait. Key to this more powerful iterator trait is the step enum:
   ```rust
   pub enum Step<Item, Meta, CanPend: Toggle, CanEnd: Toggle> {
   /// An item is ready with associated metadata.
   Ready(Item, Meta),
   /// The pull is not ready yet (only possible when `CanPend = Yes`).
   Pending(CanPend),
   /// The pull has ended (only possible when `CanEnd = Yes`).
   Ended(CanEnd),
   }
   ```
   This abstraction allows `Pull` to represent both synchronous `Iterator`s
   and asynchronous `Stream`s with zero cost. (As well as distinguishing
   between infinite vs finite iterators, which I guess is not actually that
   useful to us). In the future we will also add an `Error` variant
   (#2635). The `Meta` metadata field may be used for full record-level
   tracing (#2242).
   
   This trait has some pseudo-specialization around `Fuse`, and further
   performance improvements may come from true nightly
   `min_specialization`, as well as from converting from `Pusherator/Sink`
   to a new `Push` trait.
   
   Other changes:
   * Moves much of `dfir_rs::compiled::pull` into `dfir_pipes`, using new
   trait
   * Update itertools to `0.14`

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 20 commits contributed to the release.
 - 18 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 16 unique issues were worked on: [#2618](https://github.com/hydro-project/hydro/issues/2618), [#2644](https://github.com/hydro-project/hydro/issues/2644), [#2665](https://github.com/hydro-project/hydro/issues/2665), [#2666](https://github.com/hydro-project/hydro/issues/2666), [#2671](https://github.com/hydro-project/hydro/issues/2671), [#2673](https://github.com/hydro-project/hydro/issues/2673), [#2674](https://github.com/hydro-project/hydro/issues/2674), [#2675](https://github.com/hydro-project/hydro/issues/2675), [#2678](https://github.com/hydro-project/hydro/issues/2678), [#2680](https://github.com/hydro-project/hydro/issues/2680), [#2681](https://github.com/hydro-project/hydro/issues/2681), [#2682](https://github.com/hydro-project/hydro/issues/2682), [#2683](https://github.com/hydro-project/hydro/issues/2683), [#2684](https://github.com/hydro-project/hydro/issues/2684), [#2686](https://github.com/hydro-project/hydro/issues/2686), [#2710](https://github.com/hydro-project/hydro/issues/2710)

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **[#2618](https://github.com/hydro-project/hydro/issues/2618)**
    - Use custom `dfir_pipes::Pull` trait [ci-bench] ([`a662ff3`](https://github.com/hydro-project/hydro/commit/a662ff38541e58bec801644b81b2bfc505779e7b))
 * **[#2644](https://github.com/hydro-project/hydro/issues/2644)**
    - Use custom `dfir_pipes::Push` trait instead of `Sink` [ci-bench] ([`3e6e26c`](https://github.com/hydro-project/hydro/commit/3e6e26c4cc87d6f7857591b10876074cba97caff))
 * **[#2665](https://github.com/hydro-project/hydro/issues/2665)**
    - TakeWhile should not do its own fusing, fix #2659 ([`db8d7f1`](https://github.com/hydro-project/hydro/commit/db8d7f1a7fb2556302d92ff77dbb40beef8f3a44))
 * **[#2666](https://github.com/hydro-project/hydro/issues/2666)**
    - Fix Zip/ZipLongest size_hint and starvation bugs, fix #2664 ([`0fe74c4`](https://github.com/hydro-project/hydro/commit/0fe74c4025e2b402dc46df29c21855e8ad635fde))
 * **[#2671](https://github.com/hydro-project/hydro/issues/2671)**
    - Remove `Pin<&Self>`, use `&self` in `Pull::size_hint`, fix #2652 ([`be35ffa`](https://github.com/hydro-project/hydro/commit/be35ffa266cf564cf967bb653720dc664b24b813))
 * **[#2673](https://github.com/hydro-project/hydro/issues/2673)**
    - Simplify pull test utilities to use history-based TestPull, fix #2661 ([`f76b022`](https://github.com/hydro-project/hydro/commit/f76b0224d62af0d4ac34a7386e45c52d4f58a81a))
 * **[#2674](https://github.com/hydro-project/hydro/issues/2674)**
    - Simplify push test utilities to use history-based TestPush, fix #2661 ([`d0f9d3b`](https://github.com/hydro-project/hydro/commit/d0f9d3b5ae0b281401c78702f28399fcec5a6fcd))
 * **[#2675](https://github.com/hydro-project/hydro/issues/2675)**
    - Add `SinkCompat` to turn `Push` into `Sink`, replaces `PendingFlushSink` test, various cleanups ([`62966dd`](https://github.com/hydro-project/hydro/commit/62966dd0fa22ba6a60dc7f41cebda6165eb4990b))
 * **[#2678](https://github.com/hydro-project/hydro/issues/2678)**
    - Add `Push::size_hint`, `VecPush` terminal operator, use in dfir codegen [ci-bench] ([`52ed106`](https://github.com/hydro-project/hydro/commit/52ed1062f8fb30b9b2ec8f4615d9187bba62e2b0))
 * **[#2680](https://github.com/hydro-project/hydro/issues/2680)**
    - Add FlattenStream push operator for flattening Stream items ([`b9b0322`](https://github.com/hydro-project/hydro/commit/b9b0322abf37ff99cdafbc80eb0df62a8c0d53c9))
 * **[#2681](https://github.com/hydro-project/hydro/issues/2681)**
    - Add Pull version of FlattenStream operator to dfir_pipes ([`9aac9a7`](https://github.com/hydro-project/hydro/commit/9aac9a74004e99d0c7dd4a752322fdb7008998e0))
 * **[#2682](https://github.com/hydro-project/hydro/issues/2682)**
    - Add flat_map_stream operator to dfir_pipes::push ([`26afc34`](https://github.com/hydro-project/hydro/commit/26afc34c237bd0821c5490d8777256245176692c))
 * **[#2683](https://github.com/hydro-project/hydro/issues/2683)**
    - Add StreamCompat type in dfir_pipes/pull, mirroring SinkCompat in push ([`a0cd424`](https://github.com/hydro-project/hydro/commit/a0cd424a6b4b6d8d5c0f96ed4e2aeb7e6b64a2ac))
 * **[#2684](https://github.com/hydro-project/hydro/issues/2684)**
    - Add pull::FlatMapStream operator to dfir_pipes ([`892bcb6`](https://github.com/hydro-project/hydro/commit/892bcb67cc7789db6f749c2a5c38796bcb17f14a))
 * **[#2686](https://github.com/hydro-project/hydro/issues/2686)**
    - Resolve_future[s]_blocking for streams, singleton ([`128e85a`](https://github.com/hydro-project/hydro/commit/128e85a7b6980f76575ad0e7306f34e7de0076c6))
 * **[#2710](https://github.com/hydro-project/hydro/issues/2710)**
    - Add scan_async_blocking operator to hydro_lang and dfir ([`eacc3cd`](https://github.com/hydro-project/hydro/commit/eacc3cd85a0a9bdd70c9f6b9da4da312059f3c5a))
 * **Uncategorized**
    - Release dfir_pipes v0.0.1, example_test v0.0.1, sinktools v0.1.0, hydro_deploy_integration v0.16.0, lattices_macro v0.6.0, variadics_macro v0.7.0, lattices v0.7.0, multiplatform_test v0.7.0, dfir_rs v0.16.0, copy_span v0.1.1, hydro_deploy v0.16.0, hydro_lang v0.16.0, hydro_std v0.16.0 ([`118b356`](https://github.com/hydro-project/hydro/commit/118b356447d92e778313d72a351e5a8d2814aa1a))
    - Fix dfir_pipes/Cargo.toml ([`c9d078e`](https://github.com/hydro-project/hydro/commit/c9d078e533de7ee7d85b5f4c31c9cc049fb230e3))
    - Release hydro_build_utils v0.1.0, dfir_lang v0.16.0, dfir_macro v0.16.0, variadics v0.1.0, dfir_pipes v0.0.1, example_test v0.0.1, sinktools v0.1.0, hydro_deploy_integration v0.16.0, lattices_macro v0.6.0, variadics_macro v0.7.0, lattices v0.7.0, multiplatform_test v0.7.0, dfir_rs v0.16.0, copy_span v0.1.1, hydro_deploy v0.16.0, hydro_lang v0.16.0, hydro_std v0.16.0, safety bump 13 crates ([`c20757a`](https://github.com/hydro-project/hydro/commit/c20757ae0e9e10463b2a499de4b7d37ab02269d0))
    - Prepare for release ([`bbe8617`](https://github.com/hydro-project/hydro/commit/bbe8617b47a059d36e55ac1be1940023083cf6cb))
</details>

