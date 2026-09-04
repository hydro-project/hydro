# Components, Shallow Models, and Composition

> **Status: historical.** This document describes the pre-decisions-as-inputs
> layers (multiset carriers, sealed components, `Holds` simulation runs),
> which were deleted as unused after the (a)/(b) deliverables were migrated
> to the decisions-as-inputs surface (docs/10; typed M-form stages with
> colocated verified faces). Names below refer to deleted code — recover
> from jj history. Kept as the record of the design and of why it was
> superseded.

The unit of modular verification is the **component with a shallow verified model**:
a Rust Hydro function becomes one Lean component whose implementation is proven
equivalent to a small pure function, after which *the implementation is never
looked at again* — composition proofs consume models and contracts only. This is
the mechanized form of the paper's compositional subgraph semantics (§2.4).

## `HComponent` (`Hydro/Component.lean`)

```lean
structure HComponent (H I In β) where
  St    : Type
  tick  : TickLoop In St (List β)
  Dom   : H → I → Prop            -- value-level contract (preconditions)
  Sched : H → I → List In → Prop  -- temporal contract on the per-tick schedule
  model : H → I → Multiset β      -- the shallow model: a PURE function
  abs   : Dom h i → Sched h i ts →
          Multiset.ofList (allTicks (tick.run ts).2) = model h i
```

- `H` is the **choice space**: exactly the component's visible Rust `nondet!`
  parameters. Given all choices, runs are deterministic (the simulator's own
  structure), so a *function* model always exists.
- `abs` is **the module proof** — an abstraction theorem: every operational
  execution consistent with the choices equals the model. Above `abs`, tick states
  and slice clocks are gone.
- `Sched` hosts temporal contracts (e.g. `join_responses`' causality: metadata in
  the same-or-earlier tick than the response) that are about schedules, not values.

Entry points: `ofSliced` (a sealed component is the `H = Unit` case: model =
`denote`, abs = `adequacy`); `HComponent.erase` — **choice erasure**, the Lean form
of discharging a `nondet!` guard as *locally resolved*: supply the irrelevance proof
(the formal content of the Rust doc comment) and the choice disappears from the
signature; `contramapChoice` — project a composite guard tuple (e.g. Paxos's
`nondet_leader`/`nondet_commit`) down to a sub-component's choices.

## The combinator algebra: composition without unfolding

Every combinator derives the composite `abs` from the parts' `abs` — implementations
are never opened; the only per-seam obligation is the **contract handoff**
(upstream's model satisfies downstream's `Dom`/`Sched`):

| Combinator | Clock relationship | Notes |
|---|---|---|
| `seqWith` / `seq` | same clock (`_atomic`-linked slices) | downstream consumes the upstream's same-tick output; `seq`'s model is definitionally `model₂ ∘ model₁` |
| `seqSealed` / `sealedSeq` | same clock, one side sealed | the nondet-free side's ∀-schedule `abs` makes the induced schedule automatically legal (`hstream := id`) — **a nondet-free function composes denotationally as-is** |
| `seqAsync` / `seqAsyncWith` / `bothSealedAsync` (`ComponentAsync.lean`) | **independent clocks** — the general Hydro mechanism (`use::batch` of a sibling slice's output) | see retiming below |
| `seqSnapshot` (`SnapshotEdge.lean`) | trajectory read | see snapshot edges below |
| `par`, `mapInput`, `mapOut`, `withDom` | — | plumbing |

### Retiming: fused ≡ split (`Hydro/ComponentAsync.lean`)

Real Hydro slices run on independent clocks: a downstream slice reads an upstream
slice's output *history* through its own monotone cuts. The **retiming theorem**
(`asyncDeltas_batching_of_quiescent`, sealed form `asyncDeltas_denote`) is eager
execution (Def 2.3.1) in solved form: any interleaving of the two clocks, with any
cuts, induces *just another batching* of the upstream's total output — so for a
sealed upstream the downstream provably sees a batching of `denote(input)`, and for
a batching-invariant downstream the cut schedule is **erased** from the composite
(`seqAsync`; `seqAsync_model_eq_sealedSeq_model` states fused ≡ split as an API
equation). Cut-sensitive consumers instead keep the cut schedule visible in `H`
(`seqAsyncWith`) — the visible-nondet discipline again.

### The worked composition: `sequence_payload`

Paxos's `sequence_payload` (paxos.rs) composes `collect_quorum` into
`join_responses`. `Programs/SequencePayloadModel.lean` builds the same-clock
composite `sequencePayloadH` via `seqWith`: the *single manual proof* is the handoff
`spGlue` (quorum emissions are Nodup and causally covered by metadata — the
formalized content of the Rust `nondet!` comment "metadata will always be generated
before we get a quorum"), and the composite model comes out definitionally:
`join metadata (tagged (qualifiedKeys min responses))` (`sequencePayloadH_model`,
membership form `sequencePayloadH_mem`). `Programs/SequencePayloadAsync.lean` then
re-derives the same model for the **true split-clock schedule space**
(`sequencePayloadAsyncH`, `…_model_eq_fused`, `…_mem`) via `seqAsync` — no tick was
unfolded in either file.

## Evolving values at component boundaries

### Marker-typed singletons (`Hydro/Bounds.lean`, `KeyedSingletonTraj.lean`)

hydro_lang's extended bound hierarchy (beyond the dissertation's B/U) becomes typed
wrappers whose invariants are **generic theorems proven once per marker**:

- `MonotonicSingleton r α` (Rust `Monotonic`; constructors `fold` with a
  monotonicity witness — the Rust `monotone = manual_proof!` as a hypothesis —
  `lvar`, and `Stream.max`): `observe_mono` (cross-tick monotonicity under any
  snapshot schedule — the type-level `MaxBallotMono`), `observe_le_settled`,
  `threshold_stable` (the LVar threshold pattern, §2.6.2), `reads_chain`,
  `observeAtomic_mono`.
- `InitNoneOptional` (Rust `InitNone`): `once_some_stays_some`; constructors
  `first`, `reduce` (Rust's `AggregatedOptional`: unbounded aggregation ⇒ InitNone).
- Keyed (`KTraj` with `MonotonicKeys` / `MonotonicValue r` / `BoundedValue`,
  mirroring keyed_singleton.rs): per-key `observe_stays`, `observe_mono`,
  `threshold_stable`; constructors `keyedFold`, `keyedReduce` (the
  `reduce_watermark` shape — the Paxos acceptor log under ballot order).

`Programs/BoundsDemo.lean` instantiates these at real Paxos types: `maxBallot_mono`,
`acceptorLog_slot_mono` etc. are one-liners.

### Snapshot edges (`Hydro/SnapshotEdge.lean`)

Trajectory reads are a **first-class edge kind**, not batch edges: `SnapshotEdge`
carries the producer's contract capabilities — `Monotone r` (marker facts) and/or
`SealedAt sealF` (exact values via the sealed lattice function, doc 03) — and
consumers observe through `snapshot` (free cut, the Rust `nondet!` guard made
visible) or `snapshot_atomic` (pinned cut). `LegalObs` is the consumer-side
contract; `seqSnapshot` composes a component with an edge so the composite `abs` is
derived with producer facts entering only through the edge's discharge lemmas
(`pairwise_of_monotone`, `getElem_seal`, `mem_seal`, `consumed_chain`). Demo:
`Programs/SnapshotEdgeDemo.lean` (a threshold gate over a running max — the
`p_ballot_calc` shape), where every fact is derived from edge contracts alone.

### Shared trajectories: opt-in coherence (`Hydro/SharedTraj.lean`)

Cloning/broadcasting a singleton does **not** re-execute the fold — destinations
observe cuts of **one** chain. That correlation is *not* implied by sealedness: the
proven negative example `independent_incoherent` shows a commutative fold observed
by two independent re-executions can yield order-incomparable values. So coherence
is **opt-in**: the `SharedObs` bundle (one arrival order, per-observer cut
schedules) provides `cuts_comparable`, `observations_chain`, `cross_observer_mono`,
and **`threshold_transfer`** (the cross-observer quorum-reasoning primitive; keyed:
`observeK_threshold_transfer`, the a_log shape). `toIndependent`/`robust_transport`
prove shared ⊆ independent, so marker/sealed-trajectory facts remain
**placement-robust** (they survive fold-duplication rewrites, cf. Ch. 7
optimizations) while `SharedObs`-consuming theorems are greppably
placement-dependent. This is orthogonal to channel guarantees
(EventualConsistency/NoConsistency): coherence is about *where state is
materialized*, not delivery.

## The composition discipline, summarized

1. Nondet-free pure functions: compose as functions (`mapInput`/`mapOut`); zero
   obligations.
2. Sealed components: compose denotationally as-is (`ofSliced`, `seqSealed`,
   `seqAsync`); their ∀-schedule `abs` absorbs any induced schedule.
3. Nondet-containing components: their `H` propagates into the composite tuple
   (`contramapChoice` to route it); their model is still a function — of `(H, input)`.
4. The only manual proof at any seam is the contract handoff — a genuinely semantic
   fact, which is exactly where human insight belongs.
