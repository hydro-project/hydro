# Architecture

Reference for contributors. Everything named here is a declaration in `Hydro/`; the
history of each decision is in `FINDINGS.md` by D-number. Read `docs/` first if the
tree is new to you.

```
 Rust program (paxos.rs, quorum.rs)          one Lean `hydro def` per Rust fn
          │  1:1 mirror                              │
          ▼                                          ▼
   HydroSem signature (Sem.lean) ◀── program text written ONCE against the class
          │
          ├── Values      the denotation: graded quotient pools, nondeterminism = decision data
          ├── SchedSem    the concurrent step machine: plain lists, cursors, per-member pacing
          ├── Eager       Values with materialized data (what the executables run)
          ├── CoupleSem   the coupling corner: a SchedSem leg + a Values leg + the proof relating them
          ├── MonoRel     the diagonal relational instance (Flo monotonicity for free)
          └── HRel layer  generated binary parametricity (mono / causal / eager instances)
```

## 1. Grades and carriers (`Grades.lean`, `Order.lean`)

Rust grades every stream by `(Ordering, Retries)`. Here the grade **chooses the
content carrier**, so an illegal observation is ill-typed rather than a discipline:

| grade (`StrOrd × Retries`) | carrier `PoolCarrier α ord ret` | quotient identifies | consumer obligation `FoldOk` |
|---|---|---|---|
| `.totalOrder, .exactlyOnce` | `List α` | — | none |
| `.noOrder, .exactlyOnce` | `Multiset α` | reorderings | commutativity |
| `.totalOrder, .atLeastOnce` | `StutterSeq α` | consecutive re-delivery | consecutive idempotence |
| `.noOrder, .atLeastOnce` | `RetryPool α` | reorderings ∧ multiplicity | commutativity ∧ idempotence |

`PoolFold` is the only whole-content eliminator; its `Quotient.lift` *consumes* the
`FoldOk` obligation as its respect proof. `PoolLe` is the grade's growth order (prefix /
stutter-prefix / sub-multiset / support inclusion) — the vocabulary of monotonicity.
`KeyedAlgebra` (D66) holds the keyed-fold closed forms the programs' `into_keyed().fold`
sites reduce to.

## 2. The signature (`Sem.lean`)

`class HydroSem (L : Type) (mem : L → Nat)` — `L` is the **type of locations** of one
deployment (the set of its clusters — for Paxos `inductive PaxLoc | prop | acc`,
`Paxos/Types.lean:690`), `mem ℓ` the size of cluster `ℓ`. `L` is not *a* location:
every stream-shaped carrier is indexed by its own `ℓ : L`,

```lean
-- Hydro/Sem.lean:160–163
  Stream : L → (α : Type) → [DecidableEq α] → StrOrd → Retries → Type
  KeyedStream : L → L → (α : Type) → [DecidableEq α] → StrOrd → Retries → Type
  Singleton : L → (α σ : Type) → [DecidableEq α] → StrOrd → Retries → SingBound σ → Type
```

so `H.Stream prop P …` and `H.Ticked acc (ALog …)` live at different clusters, as
`Stream<P, Cluster<Proposer>, …>` and `Stream<_, Cluster<Acceptor>, …>` do in Rust, and
a wire can cross clusters **only** through the network operators (`broadcast_closed`,
`demux` — each taking a `TransportDec (mem p) (mem c)`, the per-(receiver, sender)
delivery cursor — then `values` at the receiver). No operator maps a `Stream c` to a
`Stream p` without one. At `Values` a stream at `ℓ` is one pool per member of `ℓ`
(`Fin (mem ℓ) → PoolCarrier …`); a `KeyedStream p c` at the receiver is one pool per
(receiver member, sender member). Fields fall into four groups:

**Carrier families** (interpretation-dependent types): `Stream ℓ α ord ret`,
`KeyedStream`, `Singleton ℓ α (…) bound` (`SingBound`: `.unbounded` / `.monotonic` —
Rust's `SingletonBound::Monotonic`, `singleton.rs:39`), `Ticked ℓ σ` (a tick-located
trace), `BoundedStream α ord ret` (one tick's content — quotient at `Values`, plain
`List` at `SchedSem`, D60), `BoundedSingleton`/`BoundedOptional`, `TickStream :=
Ticked (BoundedStream …)`.

**Decision families** (the nondeterminism vocabulary, declared per interpretation —
D54): content nondets `SnapDec`, `BatchDec`, `OrdBatchDec`, `OrderSelDec`, timing
`SampleDec`/`TimerDec`/`PulseDec`, `FixDec` (knot fuel); the adversary's `TransportDec`
(delivery cursors — `Unit` at `Values`, real at the machine). Modules bundle their
sites into `…Dec` (content) and `…Sched` (adversary) records nested along the call
structure; `#nondet_census M (nondets := a) (scheds := b) (fuels := c)` is
build-enforced per module and must equal the Rust `nondet!` tally.

**Stream-level operators** (Rust names): `map`, `filterMap`, `broadcast_closed`,
`demux`, `values`, `weaken_retries`, `union`, `assume_ordering`, `fold`,
`fold_monotone`, `snapshot`, `batch`, `batch_ordered`, `sample_every`,
`timeout_snapshot`, `source_interval_batch`, `mapTick`, `zipTick`, `defer_tick`,
`allTicks`, `flattenOrdered`, `flattenUnordered`, `fix_stream`, `fix_tick`.

**In-tick operators** over `BoundedStream`/`BoundedSingleton` (the body of a Rust
`sliced!`, D60–D61), with Rust's grade constraints in their types: `bmap`,
`bfilterMap`, `bflatMapOrdered`/`bflatMapUnordered`, `bofList`, `bcount` (ExactlyOnce),
`bfold` (`FoldOk`), `benumerate`/`bfirst` (TotalOrder), `bcrossSingleton`, `bchain`,
`bweakenOrder`, `bfilter`, `bkeyedFold`, `bkeys`, `bjoin`, `bantiJoin`, `bfilterNotIn`,
`bmax`, `bfilterIf`; singleton API `bsPure`/`bsMap`/`bsZip`, `boMap`/`boUnwrapOr`/
`boFilter`/`boIsSome`; and the former `tick_scan sts ins outs` (a structural fold over a
location's ticks, shapes zipped at the type level) that the `tick` construct elaborates
to.

Derived: `HydroSem.fix`/`fixTick` — instance-generic knot bodies with curried fuel caps
(D38), the target of the `fix … complete` construct.

## 3. The interpretations

| instance | file | carrier (stream at `ℓ`) | what it is for |
|---|---|---|---|
| `Values` (`@[reducible]`) | `Values.lean` | `Fin (mem ℓ) → PoolCarrier α ord ret`; tick wires are `Fin (mem ℓ) → Trace σ` | the denotation; all contracts are stated here; every proof lives here |
| `SchedSem pacing` | `Sched.lean` | `Fin (mem ℓ) → StepHist α` (prefix-monotone "buffer as of step `t`"); `pacing : (ℓ : L) → Fin (mem ℓ) → Nat → Bool` | the concurrent operational semantics: per-pair delivery cursors, per-member tick skeletons, consume-all-at-tick, Kleene-diagonal knots; **no quotients** — grades are erased |
| `Eager` | `Eager.lean`, `EagerProj.lean`, `EagerRel.lean` | a `Vector` of member cells ⊗ the `Values` carrier ⊗ pointwise agreement | executables; `paxos_eager_den` (= `paxos_core_param₂` at the eager relation `eagC`/`eagLaws`, `Paxos/EagerCheck.lean:91`) says what `lake exe paxos` prints is the `Values` run |
| `CoupleSem pacing Tc Td (hjT : Tc ≤ Td)` | `Couple.lean`, `CoupleProj.lean`, `Transfer.lean` | `CoStream {sr : SchedSem leg, rr : Values leg, wf : Prop, cpl : wf → ListLe (sr.view Tc) rr}` | the coupling corner: each op derives its `Values` decision from the machine leg (`batchDerive`, `snapDerive`, …) and carries its realization proof; knots close by horizon induction (`co_fix_cpl`, `co_tick_fix_cpl`); in-tick ops carry the List-vs-quotient coupling per op (`CoBounded`, D60) |
| `MonoRel` | `MonoRel.lean` | pairs of `Values` carriers related by `PoolLe` | a program at `MonoRel` is its own Flo-monotonicity proof |

Supporting layers: `Trace.lean` (traces = `List σ`, cuts, scans, `scanAcrossTicks_*`,
the option-indexed `TickReads` lemmas of D63, `StabilizesAt` in `TransferTheory.lean`),
`Decisions.lean` (`BatchCuts`, `SnapshotCuts`, `OrderSelection`, `SampleTimes`,
`TimerVerdicts`, `TimingPulses`, `UnfoldFuel`), `SchedCausal.lean`/`KnotTactics.lean`/
`WfTactics.lean` (the knot `wf` discharge kit), `TransferChecks.lean` (the `#guard`
adversary suite — the executable half of `SCHED_AUDIT.md`'s checklist).

## 4. The relational layer (`HydroRel.lean`, `HydroRelLaws.lean`, `HydroParam.lean`)

`hydro_rel_laws` generates, from the `HydroSem` class declaration itself, the binary
parametricity lift of every field (`HRel.Laws`, one law per op). An instance of
`HRelC` + `Laws` is a guarantee: `MonoHRel.lean` (the ⊑-diagonal — Flo monotonicity),
`CausalHRel.lean` (causality), `EagerRel.lean` (data/denotation agreement).
`hydro def` runs `genParam` for every module, emitting `<M>_param` (single output
leg) or `<M>_param₁…ₙ` — the module's free theorem, instantiable at any `Laws`
instance. The diagonal mono corollaries `<M>_mono…` are what consumers cite for
"re-execution with larger inputs preserves my fact" (`DOCTRINE.md` §faces).

## 5. The surface (`HydroDef.lean`, `HydroTick.lean`)

```lean
hydro [unfold-hints]? def M (H : HydroSem L mem) … (dec : MDec H …) (sched : MSched H …) :
    <output wire type>
  ensures out => MEnsures … out :=
  -- Rust line quoted above every program line
  let w₁ := H.op …                       -- the let-chain is the Rust body, 1:1
  ghost have h₁ := callee.ensures …      -- a callee's contract face, by name
  tick (state r : τ := seed) (state s : H.BoundedStream …)
       (input x := <wire>) …
       (invariant (out r s x…) => P) :=    -- optional loop invariant (Verus shape)
    <per-tick let-chain over the b-ops>
    rebind (r := …, s := …) emit (a := …) yield (b := …)
    prove init := …, tick := …;           -- the invariant's two obligations
  fix (w : τ) … via (dec.fuel…) invariant w (…) => P, base := …, step := … :=
    <cycle body>
  complete (…);                           -- Rust forward_ref / complete_cycle
  <output tuple>
  prove field₁ := …, field₂ := …          -- the Ensures, field by field
```

- **`ensures out => P`** builds the contract-faced subtype; the def's value is the
  program at any `H`, its `.ensures` the proof at `Values`.
- **Ghost layer**: `ghost have/obtain/witness/intro/subst` are spec-only bindings at
  program points — stripped from the computational leg, replayed in the proof leg (at
  the closed wires inside `fix`). Ghosts refer to wires by their program names.
- **`tick`** (D58–D61) mirrors `sliced!`/`use::state`: registers read the previous
  tick's value by construction; the body is Rust's in-tick dataflow over the b-ops; it
  elaborates to `tick_scan`. The construct emits, per block: `<out>_step` (the body as
  a function), `h<out>_run` (the wire *is* the scan), `h<out>_at` (option-indexed tick
  reader), `h<out>_reg` (∃-abstracted register: seed/read/step/stall), and with an
  `invariant` clause `h<out>_inv`/`h<out>_inv_take` (the invariant at every prefix).
  `prove tick :=` is one loop-body step in loop-invariant normal form (emissions-so-far
  + registers; no history indices).
- **`fix … complete`** (D53, D57): Bekić decomposition into hoisted knots
  `<M>.<wire>` with folded `@[reducible]` bodies; generated `stages`/`stages_zero`/
  `stages_succ`/`stages_fix`/`stages_mono` (the Kleene chain); the `invariant` clause
  packages the bounded-chain induction and lands in the prove leg as `h<wire>_inv`.
- **Reading a wire at the denotation**: `simp only [<wire lets>, den]` — the `den`
  simp set is every `HydroSem` op at `Values` as one `rfl` step (stream-level and
  in-tick). What a ghost cites, by what it meets: an operator → `den`; a `tick` block →
  its `_run/_at/_reg/_inv` readers; a module call → the callee's `.ensures` face,
  never unfolded; a `fix` knot → its generated `stages`.

## 6. The generation pipeline (`HydroGen.lean`, `HydroGenKnot.lean`)

`hydro def` elaborates the definition, records its composition spec, then runs the
pipeline routed from the spec (`HydroGen.runPipeline`): a knot wrapper (contains
`fix`) → the structural knot route `runKnotStackT` (+ `genKnotStages`); a glue module
(a callee reaches a knot) → `runGlueT` + causal + wf + mono; a knot-free module →
`runCoupleT` + causal + wf + mono; then `genParam` for all. Knots are generated eagerly
during `fix` elaboration so the enclosing def's ghosts can cite them (D57).

Generated per module (pinned by `#check` in `HydroGenCheck.lean`): `<M>_co_sr₁…`,
`<M>_co_rr₁…`, `<M>_co_rr_ex`, `<M>_vdec` (corner projection namings and the derived
decision), `<M>_causal₁…`, `<M>_co_wf₁…` (the knot/body well-formedness the corner's
`cpl` needs), `<M>_mono₁…`, `<M>_param…`, `<M>.ensures`; per knot `<M>.<w>_co_*`,
`<M>.<w>.stages*`, `<M>.<w>.inv`. The engine's per-module discipline: kernel defeq
never crosses a module or knot boundary (D40/D44 — whole-program defeq is the recurring
failure class); candidate resolvers key on head constant + projection path before any
`isDefEq` (D56).

## 7. From contract to machine theorem (`CORRESPONDENCE.md` has the full story)

`paxos_core.ensures` is a `Values` fact. `paxos_co_wf` (the five knots' `wf`, built
from generated `co_wf` artifacts) → `paxos_co_cpl` (the coupling, premise-free) →
`paxos_co_sr`/`paxos_co_rr` (the corner's legs *are* the `SchedSem` and `Values` runs) →
`paxos_safe_sched'` (`Paxos/CoupleWf.lean`): machine-run agreement under any pacing,
schedule and horizon, with no satisfiability premise and no decision witness.
`cq_safe_sched'` (`Std/Quorum.lean`) is the same four-name pipeline for the shared
stage. Liveness composes the same corner (`collect_quorum_tight`, `LIVENESS.md`).

## 8. Module map

Each file: prerequisites for the contract face → `hydro def` → smoke tests → census.
Census = (content nondets / adversary scheds / knot fuels), equal to the Rust `nondet!`
tally plus the cycles.

| module | Rust | def at line | census | contract highlights |
|---|---|---|---|---|
| `Std/Quorum.lean` `collect_quorum`, `collect_quorum_with_response` | `quorum.rs:7–88`, `:90–160` | 662, 906 | 1/0/0, 1/0/0 | `CQEnsures` (`emit_sound`, `emit_count`, `fails_eq`): emissions hold `min` `Ok` votes among the consumed pool; crossing is an iff, once; the error leg is `rfl`. `CQWREnsures` (`emit_mem_sound`, `emit_le`, `emit_complete`): emissions quote consumed `Ok` responses and embed with multiplicity. One shared loop invariant `CQRegInv` on the two `use::state` registers (D65) |
| `Std/RequestResponse.lean` `join_responses` | `request_response.rs:15–43` | 65 | 1/0/0 | `JREnsures`: joined outputs quote their key's metadata and consumed response |
| `TwoPC.lean` `two_pc` | `two_pc.rs` | 134 | 2/4/0 | a plain `def … ensures` (contract face without the generation pipeline — it is a consumer, not a stage): `TPCEnsures` composed from two `collect_quorum.ensures`; `two_pc_unanimous`/`_once`/`_commit_iff` |
| `Paxos/Types.lean` | `Ballot`, `LogValue`, … | — | — | `Ballot` lex order (`LinearOrder`), `logView` (the `manual_proof!` tie made checkable), `OnceInv` (once-per-ballot register: B1's P1a dedup and B2's gate are the same register, D64), `LeaderDiscipline`, `rcEntries`/`rcCount` |
| `Paxos/PBallotCalc.lean` `p_ballot_calc` | `paxos.rs:348–412` | 48 | — | `PBCEnsures`: `own`, `mono`, `hasLargest_true` (`p_has_largest_ballot ≡ true`, D26) |
| `Paxos/PLeaderHeartbeat.lean` `p_leader_heartbeat` | `paxos.rs:414–482` | 82 | 3/1/0 | `PLHEnsures`: `trigger_gate` (fires only at non-leader ticks), `heartbeat_src` |
| `Paxos/AcceptorP1.lean` `acceptor_p1` | `paxos.rs:484–524` | 124 | 0/1/0 | `AP1Ensures`: `max_face`, `max_mono` (ascent as a face, D60), `reply_decomp`, `from_ballot_cap`, `from_src` |
| `Paxos/AcceptorP2.lean` `acceptor_p2` | `paxos.rs:808–899` | 148 | 2/1/0 | `AP2Ensures`: `log_len_le_ck`, `log_covers_mono`, `log_entry_src` (every published entry quotes a consumed P2a), `acks_by_acceptor`; proven from the module's internal `hlog_pool` ghost (the log as the canonical `logView` of a consumed pool); `across_ticks`+`reduce_watermark` mirrored as a persisted-pool fold (D58 audit; GC is safety-neutral) |
| `Paxos/PP1b.lean` `p_p1b` | `paxos.rs:527–593` | 249 | 3/0/0 | `PP1bEnsures` (output-shaped, D66): `accepted_src`, `leader_batch`, `pinned`, `ballot_stable`, `fails_src`; the D21 fabricated-reign regress is a face |
| `Paxos/Recommit.lean` `recommit_after_leader_election` | `paxos.rs:595–672` | 92 | — | `RCEnsures` via `RCTick` (consumer vocabulary): owned / slots nodup / slot ≤ max / champion value; champion calculus inline (D64) |
| `Paxos/IndexPayloads.lean` `index_payloads` | `paxos.rs:776–806` | 59 | — | `IPEnsures`: `rebase_dominates`, `slots_dominate`, `slots_nodup` |
| `Paxos/SequencePayload.lean` `sequence_payload` | `paxos.rs:678–774` | 203 | 5/2/0 | `SPEnsures` (output-shaped, D64): `log_entry_open`, `log_entry_agree`, `commit_spec`, `commit_distinct`, `log_len_le_ck`, `log_covers_mono`; B2 gate as an `OnceInv` `tick` block |
| `Paxos/LeaderElection.lean` `leader_election` | `paxos.rs:253–346` | 152 | 8/3/3 | `LEEnsures`: `discipline : 1 ≤ qs → LeaderDiscipline …` (own/mono/lead_ne/pinned/reign), `view_promise`, `providers`, `max_mono`; three `forward_ref` cycles in one `fix`; B1's P1a dedup as an `OnceInv` block |
| `Paxos/PaxosCore.lean` `paxos_core` | `paxos.rs:136–246` | 121 | 13/5/5 | `PCEnsures.slot_functional`; the `a_log`/`sequencing_max_ballots` knot carries K4 (quorum intersection + the provenance regress) as its `invariant` clause over log entries |
| `Paxos/CoupleSafety.lean`, `Paxos/CoupleWf.lean` | — | — | — | `paxos_co_sr/_rr`, `paxos_co_wf`, `paxos_co_cpl`, **`paxos_safe_sched'`** |
| `Paxos/EagerCheck.lean` | — | — | — | `paxos_eager_den`, `paxos_eager_den_ballots`, `paxos_eager_commits` |
| `Paxos/Falsification.lean`, `Paxos/Exploration.lean` | `paxos.rs:311–317` (B1), the per-tick rebase (B2) | — | — | the executable falsifiers (`lake exe falsify`) and the D21 causality record (`lake exe explore`) |
| `Liveness.lean`, `LivenessChain.lean` | — | — | — | fairness vocabulary (`FairTicks`), chains (`IsCutChain`, `Exhausts`, `Ch*` temporal operators), `collect_quorum_tight`, `cq_live`/`cq_live_chain` |
| `AxCheck.lean` | — | — | — | `#print axioms` for every headline and generated artifact class — the gate's axiom oracle |

Toys and checks: `HydroGenToy.lean`/`HydroGenCheck.lean` (`toy_relay`, `toy_step`,
`toy_loop`, `toy_safe_sched` — every generated artifact class pinned), `HydroTickCheck.lean`
(the `tick` construct's shapes and invariants), `HydroParamCheck.lean`, `CoupleCheck.lean`
(the D39 corner blueprint), `TransferChecks.lean` (adversary witnesses §1–§9).

## 9. Performance envelope

Full build ≈ 6 min (`LeaderElection` ≈ 250 s, `SequencePayload` ≈ 35 s, `PaxosCore`
≈ 28 s, everything else < 20 s). Exactly two `set_option maxHeartbeats … in` sites
exist (`LeaderElection` 1.6M, `PaxosCore` 400k), both measured at power-of-two by
`scripts/budget_probe.sh` (D62); the engine carries no budgets. Perf questions are
answered with `trace.profiler`, never by raising a budget (`ENGINE_NOTES.md`).
