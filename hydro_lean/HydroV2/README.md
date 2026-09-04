# HydroV2 — the graded, quotient-guarded verification surface

A ground-up second architecture for verifying [Hydro](../..) dataflow
programs in Lean, superseding `HydroLean/` (v1, legacy pending
deletion). One program text per Rust function, written once against a
combinator signature (`Sem.lean`) and interpreted many ways; **what a
stream's marker types refuse to promise is unobservable by
construction**.

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
  decision data at named types (`Decisions.lean`): `BatchCuts` /
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

## Perf finding (arc M5): call-by-name vs the `Machine`

`Values` carriers are functions, so evaluation is call-by-name — every
wire access re-runs its producing chain, and sharing compounds
multiplicatively across module boundaries and Kleene knots. Measured
(`lake exe v2paxos`): one `leader_election` evaluation ≈ 6 s; naive
`paxos_core` (5 nested knots) aborted after 15+ min; the same scenario
with wires **materialized as data in evaluation order** at module/knot
boundaries (in `IO` — pure `let`-forcing is undone by the compiler)
completes in ≈ 110 s. The staged fix is the eager **`Machine`
interpretation**: evaluate wires bottom-up as data with sharing;
`v2paxos` realizes that move at the outermost boundaries and is
extensionally the `Values` run.

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
- **`Sched`/`Corr` transfer theorem** on the graded surface (the cheat
  detector): operational schedules (concrete arrival lists; duplication
  realized at `AtLeastOnce`; consecutive-only stutter at `TotalOrder`)
  coupled to `Values` pools, soundness as prefix-shaped agreement at
  extracted decisions, adequacy as replay. (Its pre-quotient
  incarnation existed and validated the shape; re-grading it is
  mechanical but large.)
- Functorial ops (`map`/`filterMap`) at `AtLeastOnce` grades (need
  destutter-commutation lemmas); `assume_ordering` from `AtLeastOnce`
  (dedup selection); `TotalOrder × AtLeastOnce` snapshots.
- The full `SlotFunctional` agreement derivation from the module
  contracts (v1 has the complete proof on its surface; the V2 module
  contracts — ok-pins-max, quorum gates, commit calculus — are its
  ingredients).
- The eager `Machine` interpretation proper.

Done (formerly staged): the iterate-projection principle is shipped
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
  Paxos/          the port (one file per Rust fn)
V2Paxos.lean      lake exe v2paxos — end-to-end demo + benchmark
```
