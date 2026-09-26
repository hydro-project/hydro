# HydroV2 — the graded, quotient-guarded verification surface

A ground-up second architecture for verifying [Hydro](../..) dataflow
programs in Lean, superseding `HydroLean/` (v1, legacy pending
deletion). One program text per Rust function, written once against a
combinator signature (`Sem.lean`) and interpreted many ways; **what a
stream's marker types refuse to promise is unobservable by
construction**.

**Reader's guide to the machine↔denotation correspondence proofs:
[`CORRESPONDENCE.md`](CORRESPONDENCE.md)** (simulation relation,
refinement mapping, adversary model, and the Paxos closing theorem —
mapped to classic distributed-systems intuition).

## The anti-cheating core (`Grades.lean`)

Rust `hydro_lang` grades every stream by `(Ordering, Retries)` markers.
V2 makes the grade semantic by choosing the **content carrier** per
grade, so that illegal observation is ill-typed rather than
discipline:

| grade | carrier | quotient identifies | consumer obligation (`FoldOk`) |
|---|---|---|---|
| `TotalOrder, ExactlyOnce` | `List α` | — | none |
| `NoOrder, ExactlyOnce` | `Multiset α` | reorderings | commutativity |
| `TotalOrder, AtLeastOnce` | `StutterSeq α` | consecutive re-delivery | consecutive idempotence |
| `NoOrder, AtLeastOnce` | `RetryPool α` | reorderings ∧ retry counts | commutativity ∧ idempotence |

`StutterSeq` (lists modulo consecutive-duplicate collapse: ordered
transport re-delivers *in place*) and `RetryPool` (multisets modulo
multiplicity: support membership is all you are entitled to) are
`Quotient`s; the only whole-content eliminator is the graded fold
`PoolFold`, whose lift *consumes* the obligation as its respect proof.
`Multiset`/`Finset` come from Mathlib (as a lemma library); the custom
setoids are self-hosted in the same pattern.

**Sampling introduces `AtLeastOnce`**: `sample_every` reads a live
`latest` at nondet times, so an unchanged value sampled twice is a
consecutive stutter — the op's output type is `TotalOrder ×
AtLeastOnce`, and `broadcast().values()` adds `NoOrder`, exactly
Rust's heartbeat type. Exactly-once legs merge into such traffic by
`weaken_retries` (a sound coarsening into the quotient); the receiving
max-fold then owes commutativity *and* idempotence
(`Ballot.maxFold_comm`/`maxFold_idem`).

## The interpretations

- **`Values.lean`** — the graded denotation ("decisions as inputs"): a
  cluster-located stream is `Fin n → PoolCarrier` (every member's
  eventual content, in the grade's quotient); a folded singleton is its
  **read function** from per-tick cut decisions to the trace of fold
  states (`CutDec`: prefix counts when ordered, increment multisets
  when unordered; legality — count-legal at `ExactlyOnce`, membership
  at `AtLeastOnce` — blocks, so *legality is realizability*); tick
  carriers are realized values across all ticks (`Trace σ`), and tick
  batches keep their grade through the boundary. All nondeterminism is
  decision data whose *types are instance fields* (`…Dec` on
  `HydroSem`: each interpretation declares its own nondeterminism
  vocabulary — `Values` pays content decisions and takes `Unit` at
  transport, the step machine pays cursors/timing and takes `Unit` at
  content sites, `RelSem` takes `Unit` everywhere). At `Values` the
  shapes carry domain names (`Decisions.lean`): `BatchCuts` /
  `OrderedBatchCuts` / `SnapshotCuts`, `OrderSelection` /
  `BatchOrderSelection`, `SampleTimes` / `TimerVerdicts` /
  `TimingPulses`, and `UnfoldFuel` (guarded Kleene iteration for Rust
  `forward_ref`); modules bundle their own sites into per-module
  decision structs, nested along the call structure (`PLHDec`,
  `PP1bDec` ⊂ `LEDec`; `SPDec`; `LEDec`+`SPDec` ⊂ `PaxosCoreDec`) —
  a composite never re-declares its callees' decisions.
- **`MonoRel.lean`** — the diagonal relational interpretation: carriers
  are pairs of `Values` carriers related by the type-assigned growth
  order (`PoolLe`: prefix / stutter-prefix / sub-multiset /
  support-inclusion; read-prefix on singletons; trajectory-prefix on
  tick wires), and every op re-proves the relation. A program
  instantiated at `MonoRel` **is** its own Flo-monotonicity proof —
  each module's `*_mono` theorem is a two-line projection.
- **`Sched.lean`** — the **step machine**: the concurrent operational
  semantics, *erased and concrete*. Every grade executes as plain
  lists (`StepHist`: prefix-monotone "buffer as of step `t`" histories
  on one global step clock); fan-in takes no decision — interleaving
  *emerges* from delivery timing; a tick consumes everything available;
  a snapshot reads the fold accumulator's latest value; `fix` is the
  Kleene diagonal (cycle unfolding *is* step progression, no fuel).
  Nondeterminism is the machine's own vocabulary: per-pair delivery
  cursors, tick skeletons (ambient `pacing`, per cluster member — its
  irreducible content is stutter ticks; tick presence is independent
  across members), concrete timing, and emission linearizations at
  `emitMultisetBatches` (`EmitDec` — Rust's hashmap-iteration order at
  `flatten_unordered`: the one site where a program puts *computed
  unordered data* on a wire, so wire order is born there; `Vec`-backed
  state should use `emitBatchesUnordered`, which is decision-free).
  **Time** is *elastic above, floored below*: idle steps are legal and
  observationally invisible (they are the adversary's coverage — async
  ticks, member asymmetry, unbounded lateness, granularity
  refinement), while two structural floors — network delivery `+1` and
  the `fix` feedback edge `+1` (an implicit defer at `fix_tick`) —
  exclude exactly the Zeno executions under which the Kleene diagonal
  is ill-defined. Only the knot floor is load-bearing for safety
  (every cycle passes a `fix`); the network floor gives steps the
  Lamport property (step count dominates causal depth through *every*
  hop), keeping the clock a usable ruler for future latency/liveness
  reasoning. Quiescence = "all remaining steps idle": once time stops
  mattering, the machine *is* its denotation (see FINDINGS D31).
- **`Rel.lean`** — the ∃-packaging (nondeterminism-monad) semantics:
  carriers are **sets of `Values` runs**, every decision `Unit`, every
  op the image of the `Values` op over its decision space. A program
  at `RelSem` denotes the compositionally-defined set of its
  denotational runs (the free/possibilistic reading — diamond reuse
  decorrelates, a sound over-approximation).
- **`Eager.lean` / `EagerProj.lean`** — the **materialized
  denotation**: the same `Values` semantics with a data representation.
  Every carrier pairs a `Vector` of member cells (forced once, in
  dataflow order — a compiled program at `Eager` executes in one strict
  pass) with the `Values` carrier it means and the pointwise agreement
  proof; every op is the `Values` op applied twice (no formula is ever
  re-spelled). The per-op projection lemmas (`EagerProj.lean`, all
  `rfl`) assemble per program by `eager_transfer [<def names>]` into
  identities like `paxos_eager_den` (`Paxos/EagerCheck.lean`), so what
  an executable prints is **provably** the `Values` run the headline
  theorems quantify over. This is what `lake exe
  falsify`/`explore`/`v2paxos`/`v2twopc` run (milliseconds; the same
  scenarios needed hours at call-by-name `Values` — FINDINGS D14/D34).
  The one deliberately-functional seam: an un-snapshotted fold's cell is
  `CutDec → Trace` in both legs (observation is decision-indexed;
  site-local, non-compounding).
- **`Couple.lean` / `CoupleProj.lean`** (+ `SchedCausal.lean`,
  `MonoHRel.lean`, `CausalHRel.lean`, `KnotTactics.lean`, `WfTactics.lean`) — the
  **coupling corner** `CoupleSem` (FINDINGS D39–D41): each carrier
  pairs a machine leg (`SchedSem`, verbatim) with a *plain* `Values`
  leg, a well-formedness residue `wf`, and the coupling proof
  `cpl : wf → ListLe (sr.view Tc) rr` — at two ambient horizons
  (couple-at-`Tc`, derive-at-`Td`). Decision-mediated ops **derive
  their own `Values` decision from the machine leg** (`batchDerive`,
  `snapDerive`, … — `Transfer.lean`), so each op's `cpl` is its
  realization theorem at its own derived decision: no decision
  environment, no lens records, no legality atoms, no satisfiability
  premise. Knots close by horizon induction through the machine
  fixpoint equation (`co_fix_cpl`/`co_tick_fix_cpl`), their `wf`
  triple (causality ∧ Kleene chain ∧ graded coupling) discharged
  per-knot via the instance-generic `HydroSem.fix` bodies (D38) —
  causality by the `SchedCausal.lean` head-dispatch walker, chain by
  one `MonoRel`/`MonoHRel` instantiation, the graded obligation by
  re-instantiating the body at a horizon-lowered corner. Per-program
  cost is per-module naming lemmas (kernel defeq through `k` nested
  knots is exponential in `k` — D40 — so namings assemble
  module-folded, linear): `paxos_safe_sched'`
  (`Paxos/CoupleWf.lean`) and `cq_safe_sched'` (`Std/Quorum.lean`)
  state machine-run safety for **any** pacing, **any** schedule,
  **any** horizon with **no** satisfiability premise and **no**
  decision witness. (The earlier `∀ d`-quantified square coupling
  with its δ-satisfiability premise is retired: the δ route was
  falsified for nested knots — FINDINGS D37 — then dissolved by the
  corner; the ledger D32–D41 and `SCHED_AUDIT.md` F1 keep the full
  record.)
- **`Transfer.lean` / `TransferTheory.lean`** — the **transfer
  coupling** `CorrSem`: carriers pair a machine run with a covering
  set of denotational runs, per-step-∃ (`∀ step, ∃ v ∈ V, …` — cycles
  couple stepwise to Kleene iterates), under the graded concrete-to-
  quotient relation `ListLe` (prefix / destuttered prefix /
  sub-multiset / support inclusion — the erased mirror of `PoolLe`).
  Every op fills its coupling field — totality is the cheat detector
  in both directions (an unfillable field means either an op observes
  more than the machine provides, or the denotational decision space
  is too small to cover a real interleaving). The theorem package:
  - **safety transfer**: ∀ schedule ∀ horizon, machine observations
    sit below some denotational run at *derived* decisions —
    already an **equality** at every decision-mediated observation
    (`sched_snap_eq`, `snapshot_tight`, `batch_tight`: truncation *is*
    the decision at consumption sites, so the values side expresses
    exactly what the machine consumed);
  - **end-of-time equality** (per wire, compositional —
    `StabilizesAt`): a stabilized wire attains its denotational pool
    exactly (`deliver_id_attains`, `batch_flat_attains`,
    `fix_diag_attains` — cycles under the per-program hypothesis that
    the Kleene chain fixes; a partially-churning program still gets
    equality on its stabilized wires);
  - **adequacy is false by design**: no `∀ d ∃ schedule` theorem —
    `emitBatchesUnordered → assume_ordering` is the counterexample
    (the machine realizes one emission order; the denotation licenses
    all permutations). The quotient deliberately over-approximates:
    safety needs machine ⊆ denotation only, and the surplus makes
    program obligations robust to weaker transports.

  Per-program cost is ruling-4's two-liner (`Paxos/TransferCheck.lean`:
  the whole `paxos_core` at `CorrSem` = a decision record + `.property`
  projections; `fix`-closed programs use the `iterate_subtype_*`
  projection lemmas). Executable adversary witnesses live in
  `TransferChecks.lean` (interleaving, latency/silence, per-pair FIFO,
  stutter ticks, `AtLeastOnce` birth at sampling, the fixpoint race —
  offspring overtaking a stalled elder through a knot — snapshot
  intermediates: pacing captured, micro-order quotiented; and
  per-member tick skew — one member ticks with content at a step where
  a sibling has no tick entry at all).

## The shared verified stages (`Std/`)

`hydro_std`, same discipline — one Rust fn, one Lean def, contract
colocated. A file reads Ensures → the register machine (the Rust
`sliced!` step, **defined once**, folded by the program body — no spec
mirror) → step obligations (per-tick case analyses over the module's
own step) → run facts (each a single application of a generic
`Trace.lean` scan combinator: `scan_sound` / `scan_emit_ind` /
`scan_bound` — the run-level induction boilerplate lives once,
generically) → the program definitions → smoke tests. Consumers
compose by contract only — every surface fact is a **clause of the
Ensures record** (no spec functions, no satellite theorems: the
success-leg embedding is `CQWREnsures.emit_pool_le`, stated directly
on the output); no machine name appears outside `Std/`
(grep-enforced):

| module | Rust | contract highlights |
|---|---|---|
| `Std/Quorum` | `quorum.rs` `collect_quorum`, `collect_quorum_with_response` | soundness unconditional (emissions hold `min` `Ok` votes among the consumed pool; `cqwr` emissions embed in the `Ok` projection **with multiplicity** — `emit_pool_le`, *no* usage caps); the crossing count is an **iff, once, for every batching** under the usage caps (quorum.rs:25's `nondet!` comment, now a theorem); error legs are pure `filter_map`s (`rfl`) |
| `Std/RequestResponse` | `request_response.rs` `join_responses` | joined outputs quote their key's metadata and consumed response; completeness under the `atomic` metadata staging — metadata is a tick-stream **parameter**, not a decision |

`PP1b` and `SequencePayload` consume these (FINDINGS D29): the P1b
quorum vocabulary selects `collect_quorum_with_response`'s realized
success pool, and the commit proof composes `join_src` + `emit_sound`.

## The Paxos port (`Paxos/`)

`hydro_test/src/cluster/paxos.rs`, one file per Rust function, program
text once over the signature, contracts per module over `@Values`
(proofs never re-enter callees — composition is by contract):

| module | Rust | contract highlights |
|---|---|---|
| `Types` | `Ballot`, `LogValue`, … | lex order via `Lex (ℕ × ℕ)` embedding; `logView` (below) |
| `PBallotCalc` | `p_ballot_calc` | ballots owned; `p_has_largest_ballot ≡ true`; `Monotonic` rides the type |
| `PLeaderHeartbeat` | `p_leader_heartbeat` | election-trigger gate (fires only at non-leader ticks); heartbeat traceability |
| `AcceptorP1` | `acceptor_p1` | reply characterization (echo + routing + promise vs the tick's max + write-before-ack log); `Ok` pins the max |
| `AcceptorP2` | `acceptor_p2` | log face (canonical view of consumed qualified entries); ack characterization; `Ok` acks quote the max |
| `PP1b` | `p_p1b` | leader gate (quorum for *own* ballot + largest); accepted-log and fail traceability to the pool |
| `Recommit` | `recommit_after_leader_election` | canonical (order-free) recommit list; recommits owned |
| `IndexPayloads` | `index_payloads` | emitted slots are exactly `range' base len` |
| `SequencePayload` | `sequence_payload` | commit calculus: commits need `> f` `Ok` votes, are emitted once, quote the leader's own metadata |
| `LeaderElection` | `leader_election` | three `forward_ref` cycles closed by `fix`; core-level mono; executable election |
| `PaxosCore` | `paxos_core` | `a_log` + `sequencing_max_ballots` knots; `SlotFunctional` face |

**The `manual_proof!` hole, made checkable** (paxos.rs:862): the
per-slot log merge is not commutative when a slot sees equal ballots
with different values. V2 accumulates the raw entry *multiset*
(commutative and inflationary unconditionally — the quotient accepts
it) and computes the log as the canonical view `logView`: per-slot
max ballot with a commutative `ValWitness` consensus tie-break that
degrades conflicts to `none` — unreachable under the slot-functional
contract, i.e. the Rust `assume`, now visible (`#guard`ed in
`AcceptorP2.lean`).

**Executable non-vacuity**: per-module `#guard`s throughout, an
election scenario in `LeaderElection.lean` (leader at tick 1), and the
end-to-end guarded commit in `lake exe v2paxos` (one proposer, one
acceptor, `f = 0`: payload `42` elected-sequenced-accepted-committed to
the replica pool as `(0, some 42)`, slot-functional observed).

## Perf finding (arc M5): call-by-name vs the `Machine` — RESOLVED by `Eager`

`Values` carriers are functions, so evaluation is call-by-name — every
wire access re-runs its producing chain, and sharing compounds
multiplicatively across module boundaries and Kleene knots. Measured:
one `leader_election` evaluation ≈ 6 s; naive `paxos_core` (5 nested
knots) aborted after 15+ min; the historical IO harness (wires
materialized at module/knot boundaries) brought one run to ≈ 110 s.
**Resolved by the `Eager` interpretation** (see "The interpretations"
above): the same `Values` semantics with `Vector` data carriers,
evaluated in one strict pass — `v2paxos` now runs the scenario in
≈ 0 ms with correctness pinned by `paxos_eager_den`, and all
executables are harness-free thin mains.

## Staged work

- **6b — fuel-free cycles (`UnfoldFuel` removal), the agreed design**:
  `fix_stream`/`fix_tick` stop taking an unfolding-depth *decision*;
  the `Values` interpretation computes the **true fixpoint by
  stabilization search** — iterate from the least wire until
  `iterate (k+1) = iterate k` (needs `DecidableEq` on the carriers),
  with an evaluation budget that is an *artifact*, not a semantic
  decision. The framework faces are already shipped
  (`Trace.lean`): `fix_stabilizes` (a bounded, strictly-growing measure
  forces a true fixpoint within the budget — for Paxos wires the
  measure is total pool size, bounded by the finite decision lists),
  `iterate_stab_of_fixed` (stability propagates), `fix_induction` (the
  generic invariance face the fuel inductions rework onto). Retrofit
  surface: `Sem.lean` op signatures, `Values`/`MonoRel` `fix` fields,
  every `*Dec` record (drop `fuel*` fields), K4's stage induction, the
  demo exe. Staged as the next mandate; not implemented here.
- **Proof-at-the-end layout** (user idea, cheap): in the curried
  colocated form, split the pass into a *wires-only* `let` (pure Rust
  text) and move the per-pass contract proof into the single `by` block
  at the bottom (a `let` is transparent — the end-proof reaches the
  wires by name; only `have` is opaque). Reading order becomes: pass
  wires → knot-closing wires → output tuple → ALL proof text last, so
  no proof body interrupts the Rust-equivalent code.
- Functorial ops (`map`/`filterMap`) at `AtLeastOnce` grades (need
  destutter-commutation lemmas); `assume_ordering` from `AtLeastOnce`
  (dedup selection); `TotalOrder × AtLeastOnce` snapshots.
- The full `SlotFunctional` agreement derivation from the module
  contracts (v1 has the complete proof on its surface; the V2 module
  contracts — ok-pins-max, quorum gates, commit calculus — are its
  ingredients).
- The eager `Machine` interpretation proper (its *execution* role is
  delivered by `Eager.lean`; what remains is the step-machine realism
  role — falsify-as-a-real-schedule, liveness).

Done (formerly staged): the **step-machine transfer** is shipped (see
"The interpretations" above — `Sched.lean`/`Rel.lean`/`Transfer.lean`/
`TransferTheory.lean`/`TransferChecks.lean`, with `paxos_core`
validated at `CorrSem`). Note for 6b (fuel-free cycles): with an
lfp-style `Values` fix, the transfer's derived decisions lose their
fuel component and end-of-time equality strengthens to an ω-limit
statement under fairness (no stabilization hypothesis; `StabilizesAt`
becomes the attained-at-finite-`T` special case — the forever-counter
gains a real limit theorem). Also done: the iterate-projection principle is shipped
(`Trace.iterate_val_proj`), and `fix`-closed modules no longer need
external mono theorems at all — `leader_election` carries binary Flo
monotonicity **colocated** (the curried clause: the def returns its I/O
function with the unary contract AND the two-run monotonicity in one
dependent-implication clause; the pass is a reviewable `let`, never
re-spelled by any proof).

## Layout

```
HydroV2/
  Order.lean      value orders (fold inflation calculus)
  Grades.lean     markers, quotient carriers, PoolFold/PoolLe/FoldOk
  Trace.lean      traces, cuts (batch/snapshot/slice/prefix), scans, sampling, iterate
  Sem.lean        the graded combinator signature (HydroSem)
  Values.lean     the graded denotation
  MonoRel.lean    the relational gluing (monotonicity for free)
  Sched.lean      the step machine (erased concrete operational semantics)
  Rel.lean        the ∃-packaging set-valued semantics
  Transfer.lean   ListLe coupling, derived cuts, read agreement, CorrSem
  TransferTheory.lean  projections, tightness, end-of-time kit
  TransferChecks.lean  executable adversary witnesses (#guard suite)
  Std/            hydro_std, verified (quorum.rs, request_response.rs; QuorumTheory.lean = contract vocabulary + pool algebra)
  Paxos/          the port (one file per Rust fn; TransferCheck.lean = paxos_core at CorrSem)
V2Paxos.lean      lake exe v2paxos — end-to-end demo + benchmark
```
