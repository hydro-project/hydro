# Rust-Parity Naming Audit

> **Status: historical.** This document describes the pre-decisions-as-inputs
> layers (multiset carriers, sealed components, `Holds` simulation runs),
> which were deleted as unused after the (a)/(b) deliverables were migrated
> to the decisions-as-inputs surface (docs/10; typed M-form stages with
> colocated verified faces). Names below refer to deleted code — recover
> from jj history. Kept as the record of the design and of why it was
> superseded.

Audit of Lean surface-API names against the Rust Hydro API, per DESIGN.md's
"Naming policy (Rust parity)". Canonical snake_case aliases live in
`HydroLean/Hydro/RustParity.lean` (import it; the camelCase originals remain
valid as legacy spellings — existing proofs reference them). Lemma/theorem
names are Lean-idiomatic by policy and out of scope.

Rust sources of truth: `hydro_lang/src/live_collections/stream/mod.rs`,
`.../stream/networking.rs`, `.../keyed_stream/`, `.../singleton.rs`,
`.../optional.rs`, `hydro_std/src/quorum.rs`, `.../request_response.rs`.

## Aliased in `RustParity.lean` (canonical → legacy)

| Rust name | Lean canonical | Legacy spelling | File of original |
|---|---|---|---|
| `Stream::filter_map` | `Stream.filter_map` | `Stream.filterMap` | Hydro/Stream.lean |
| `Stream::flat_map_ordered` (no plain `flat_map` in Rust) | `Stream.flat_map_ordered` | `Stream.flatMap` | Hydro/Stream.lean |
| `Stream::cross_singleton` | `Stream.cross_singleton` | `Stream.crossSingleton` | Hydro/Stream.lean |
| `Stream::anti_join` | `Stream.anti_join` | `Stream.antiJoin` | Hydro/Stream.lean |
| `Stream::filter_not_in` | `Stream.filter_not_in` | `Stream.filterNotIn` | Hydro/Stream.lean |
| `Stream::into_keyed().fold(..)` | `Stream.keyed_fold` | `Stream.keyedFold` | Hydro/Stream.lean |
| `Stream::weaken_ordering` | `Stream.weaken_ordering` | `Stream.toMStream` | Hydro/MsetStream.lean |
| `Stream::filter_map` (NoOrder) | `MStream.filter_map` | `MStream.filterMap` | Hydro/MsetStream.lean |
| `Stream::fold_commutative` | `MStream.fold_commutative` | `MStream.foldCommutative` | Hydro/MsetStream.lean |
| `Stream::cross_singleton` (NoOrder) | `MStream.cross_singleton` | `MStream.crossSingleton` | Hydro/MsetStream.lean |
| `Stream::anti_join` (NoOrder) | `MStream.anti_join` | `MStream.antiJoin` | Hydro/MsetStream.lean |
| `Stream::filter_not_in` (NoOrder) | `MStream.filter_not_in` | `MStream.filterNotIn` | Hydro/MsetStream.lean |
| `Stream::into_keyed().fold(commutative = ..)` | `MStream.keyed_fold` | `MStream.keyedFold` | Hydro/MsetStream.lean |
| `Stream::send_bincode` | `send_bincode` | `sendBincode` | Hydro/Net.lean |
| `Stream::all_ticks` | `all_ticks` | `allTicks` | Hydro/Tick.lean |
| `Singleton::snapshot` | `Traj.snapshot`, `MonotonicSingleton.snapshot` | `.observeAt` | Hydro/SingletonTraj.lean, Bounds.lean |
| `Singleton::snapshot_atomic` / `latest_atomic` | `Traj.snapshot_atomic`, `MonotonicSingleton.snapshot_atomic` | `.observeAtomic` | Hydro/Atomic.lean, Bounds.lean |
| `KeyedSingleton::snapshot` | `KTraj.snapshot` | `KTraj.observeAt` | Hydro/KeyedSingletonTraj.lean |
| `KeyedStream::fold(monotone = ..)` | `KTraj.keyed_fold` | `KTraj.keyedFold` | Hydro/KeyedSingletonTraj.lean |
| `KeyedStream::reduce` / `reduce_watermark` | `KTraj.keyed_reduce` | `KTraj.keyedReduce` | Hydro/KeyedSingletonTraj.lean |

## Already matching (no alias needed)

| Rust | Lean | File |
|---|---|---|
| `map`, `filter`, `chain`, `enumerate`, `fold`, `reduce`, `keys` | `Stream.map/filter/chain/enumerate/fold/reduce/keys` | Hydro/Stream.lean |
| `map`, `filter`, `chain`, `keys` | `MStream.*` | Hydro/MsetStream.lean |
| `join` | `Stream.join`, `MStream.join` | Hydro/Join.lean, MsetStream.lean |
| `demux` | `demux` | Hydro/Net.lean |
| `broadcast_closed` | `broadcast_closed` | Hydro/Net.lean (added when the policy landed) |
| `max` | `Stream.max` (Monotonic constructor) | Hydro/Bounds.lean |
| `first`, `reduce` | `InitNoneOptional.first/reduce` | Hydro/Bounds.lean |
| `Singleton`, `Optional` | `Singleton`, `Optional` (type abbrevs) | Hydro/Stream.lean |

## Proof-layer — deliberately NOT aliased (no Rust counterpart)

| Lean | Why |
|---|---|
| `Stream.countKey`, `countKeyP` (and MStream/Keyed forms) | spec observables used to state contracts; the Rust closure equivalents are inline folds |
| `SlicedComponent.*`, `HComponent.*`, `denote`, `model`, `abs`, combinators (`seq*`, `erase`, ...) | proof algebra |
| `Traj`, `settled`, `MTraj`/`sealFold`, `MonoAlong`, marker wrappers as *types* | denotational trajectory layer (constructors mirror Rust; the layer itself has no Rust name) |
| `Batching`, `SnapshotSchedule`, `Interleaving`, `SharedObs`, `IndependentObs` | choice spaces = sim hook payloads (`BatchHook`/`SnapshotHook` correspondence documented in docs/05) |
| `Net.interleave`, `IsInterleaving`, `taggedCat`, `ClusterVal`, `updateBuf` | denotational models of the cluster receive side of `send_bincode` |
| `TickLoop.run/runFrom/outputs/finalState`, `logThenAck`, `atomicInputs` | tick-machine plumbing; the Rust counterpart is the `sliced!` macro structure itself |
| `Sim.*`, `SimGraph.*` (`Holds*`, `Settles`, `simRun`, ...) | simulator mirror; vocabulary intentionally names Rust sim *concepts*, not APIs |
| `Collections/*` (`Multiset`, `DupList`, `foldComm`, `AccComm`, ...) | carrier library (Mathlib-style); underpins markers, not user-facing ops |

## Deferred to a future breaking-rename pass

| Item | Reason |
|---|---|
| `Hydro.broadcast` (legacy name for closed semantics) | `broadcast_closed` is canonical; the bare name should eventually be repurposed for the dynamic-membership variant modeled in `Programs/TwoPCOpenMembership.lean` (`Hydro/OpenBroadcast.lean`); renaming now would be non-additive |
| Program entry points: `collectQuorum`(+`Tick`/`WR`), `joinResponsesTick`, `indexPayloadsTick`, `twoPC*` ↔ `collect_quorum`, `join_responses`, `index_payloads`, `two_pc` | heavily referenced across proof files incl. `Programs/Paxos/*` (owned by the active safety agent); alias additively after the capstone lands |
| `Multiset.dedup` vs Rust `Stream::unique` | different layer (carrier vs stream op); add an `MStream.unique` wrapper when a port needs it |
| Dot-notation ergonomics on `Stream`/`MStream` | `Stream` is a reducible `abbrev` of `List`, so `s.filter_map` resolves into the `List` namespace; the codebase convention is explicit `Stream.op s ...` application (pre-existing, unrelated to aliasing) |
| docs/ snippets using legacy spellings | docs describe the code as it exists and state the policy; sweep snippets to canonical names together with the breaking rename |
