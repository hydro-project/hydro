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
          │               (member carriers are `MemberMap`s — what the executables run, D73)
          ├── SchedSem    the concurrent step machine: plain lists, cursors, per-member pacing
          ├── CoupleSem   the coupling corner: a SchedSem leg + a Values leg + the proof relating them
          ├── MonoRel     the diagonal relational instance (Flo monotonicity for free)
          └── HRel layer  generated binary parametricity (mono / causal instances)
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
-- Hydro/Sem.lean
  Stream : L → (α : Type) → [DecidableEq α] → StrOrd → Retries → Type
  KeyedStream : L → (K V : Type) → [DecidableEq K] → [DecidableEq V] → StrOrd → Retries → Type
  Singleton : L → (α σ : Type) → [DecidableEq α] → StrOrd → Retries → SingBound σ → Type
  KeyedSingleton : L → (α K V : Type) → [...] → StrOrd → Retries → KSBound → Type
```

so `H.Stream prop P …` and `H.Atomic acc (ALog …)` live at different clusters, as
`Stream<P, Cluster<Proposer>, …>` and `Stream<_, Cluster<Acceptor>, …>` do in Rust, and
a wire can cross clusters **only** through the network operators (`broadcast_closed`,
`demux` — each taking a `TransportDec (mem p) (mem c)`, the per-(receiver, sender)
delivery cursor — then `values` at the receiver). No operator maps a `Stream c` to a
`Stream p` without one. At `Values` a stream at `ℓ` is one pool per member of `ℓ`
(`MemberMap (mem ℓ) (PoolCarrier …)`); the network's output is the general
`KeyedStream p (Fin (mem c)) α` (Rust `KeyedStream<MemberId<C>, α, …>`), at `Values` a
`KeyedPool` — a finite key set with per-key graded content, keyed by sender; `values`
merges the keys (D70). Fields fall into four groups:

**Carrier families** (interpretation-dependent types): `Stream ℓ α ord ret`,
`KeyedStream ℓ K V ord ret` (grouped content: `KeyedPool` at `Values`, flat pairs at the
machine), `Singleton ℓ α σ ord ret bound` (`SingBound`: `.unbounded` / `.monotonic` —
Rust's `SingletonBound::Monotonic`, `singleton.rs:39`; `α` is the SOURCE type the
singleton is read off, `ord` the cut vocabulary of its reads), `KeyedSingleton ℓ α K V
ord ret kb` (`KSBound`: `.unbounded` / `.boundedValue` — Rust's `KeyedSingletonBound`;
`KeyedMap` at `Values`, a nodup assoc list `SchedKMap` at the machine), `Atomic ℓ σ`
(Rust `Atomic<ℓ>`: a tick-located trace — formerly `Ticked`, renamed D70),
`BoundedStream α ord ret` (one tick's content — quotient at `Values`, plain `List` at
`SchedSem`, D60), `BoundedSingleton`/`BoundedOptional`, `BoundedKeyedStream K V ord
ret`/`BoundedKeyedSingleton K V` (one tick's keyed content), and the atomic defaults
`AtomicStream := Atomic (BoundedStream …)`, `AtomicKeyedStream`, `AtomicKeyedSingleton`
(Rust's `all_ticks_atomic`/`batched_atomic` are casts and identities here: Rust's
`x.across_ticks(|s| s.op(…))` is written `H.op x …` directly on the atomic wire).

**Decision families** (the nondeterminism vocabulary, declared per interpretation —
D54): content nondets `SnapDec`, `BatchDec`, `OrdBatchDec`, `OrderSelDec`,
`TickOrderSelDec` (a per-tick order for one tick's unordered batch — D71), timing
`SampleDec`/`TimerDec`/`PulseDec`, `FixDec` (knot fuel); the adversary's `TransportDec`
(delivery cursors — `Unit` at `Values`, real at the machine). Modules bundle their
sites into `…Dec` (content) and `…Sched` (adversary) records nested along the call
structure; `#nondet_census M (nondets := a) (scheds := b) (fuels := c)` is
build-enforced per module and must equal the Rust `nondet!` tally.

**Stream-level operators** (Rust names): `map`, `filterMap`, `broadcast_closed`,
`demux`, `values`, `weaken_retries`, `union`, `assume_ordering`, `fold`,
`fold_monotone`, `snapshot`, `batch`, `batch_ordered`, `sample_every`,
`timeout_snapshot`, `source_interval_batch`, `mapTick`, `zipTick`, `defer_tick`,
`allTicks`, `flattenOrdered`, `flattenUnordered`, `fix_stream`, `fix_tick`; the keyed
chain (D70) `into_keyed`, `kassume_ordering` (`KeyedStream::assume_ordering`),
`kfold_early_stop` (→ `.boundedValue`), `kget_max_key` (→ a `Singleton` read at
`.noOrder` cuts of the keyed source), and on the atomic traces `tinto_keyed`,
`reduce_watermark` (hydro-project/hydro#3189's semantics: a rising watermark, entries
below it removed, arrivals below it dropped), and `tassume_ordering` (D71: Rust's
`.into_keyed().assume_ordering::<TotalOrder>(nondet!)` on a tick's batch — a
`TickOrderSelDec` picks each tick's order; at `Values` the trace is `realizeTicks`:
a tick is realized only by a selection that is a permutation of its batch, otherwise
the trace is CUT there — "an illegal selection blocks", so the decision space is total
by construction and no legality premise exists; at the machine the order is the
arrival order; the corner derives the `Values` selection from the machine's realized
tick and couples it by `mono`, below).

**In-tick operators** over `BoundedStream`/`BoundedSingleton` (the body of a Rust
`sliced!`, D60–D61), with Rust's grade constraints in their types: `bmap`,
`bfilterMap`, `bflatMapOrdered`/`bflatMapUnordered`, `bofList`, `bcount` (ExactlyOnce),
`bfold` (`FoldOk … Pre`), `benumerate`/`bfirst` (TotalOrder), `bcrossSingleton`, `bchain`,
`bweakenOrder`, `bfilter`, `bjoin`, `bantiJoin`, `bfilterNotIn`, `bmax`, `bfilterIf`;
the keyed API `binto_keyed`, `bkfold` (`FoldOk … Pre`, value-only closure as in Rust),
`bksFilter`/`bksMap`/`bksKeys`/`bksEntries` (D70); singleton API `bsPure`/`bsMap`/
`bsZip`, `boMap`/`boUnwrapOr`/`boFilter`/`boIsSome`; and the former `tick_scan sts ins
outs` (a structural fold over a location's ticks, shapes zipped at the type level —
leaves `.sing`/`.stream`/`.ksing`) that the `tick` construct elaborates to.

Derived: `HydroSem.fix`/`fixTick` — instance-generic knot bodies with curried fuel caps
(D38), the target of the `fix … complete` construct.

**Conditional commutativity (R3, D71).** The fold family (`fold`, `bfold`, `bkfold`,
`reduce_watermark`) takes a prerequisite `Pre : α → α → Prop` and the obligation
`FoldOk ord ret Pre g` (commutativity on `Pre`-pairs — Rust's `commutative =
manual_proof!(…)` made honest: the literal closure is provably commutative *under* the
tie it assumes). The content half — every two values that meet at a key satisfy `Pre` —
is not an operator argument (a program against an abstract `H` cannot prove content):
the denotation is a permutation-invariant per-key reduce with `⊥` where the pairwise
fact fails, the `hydro def` proves the fact at the denotation before the site (`ghost
pre`, mandatory for a non-`True` `Pre`; `requires` names the module-level premise it
may use), and the corner's `wf` consumes the packaged `PreFact`. `Pre := fun _ _ =>
True` is the unconditional obligation. The genuinely order-DEPENDENT closure
(paxos.rs's recommit `(count, champion)` fold) is not folded under a `Pre`: it runs on
a `tassume_ordering` decision, as the Rust now does.

## 3. The interpretations

| instance | file | carrier (stream at `ℓ`) | what it is for |
|---|---|---|---|
| `Values` (`@[reducible]`) | `Values.lean`, `MemberMap.lean` | `MemberMap (mem ℓ) (PoolCarrier α ord ret)`; tick wires are `MemberMap (mem ℓ) (Trace σ)` — a `MemberMap n C` is `{ cells : Fin n → C, boxed : Unit }`, read `m i`, built `.mk' fun i => …` | the denotation; all contracts are stated here; every proof lives here; **and what the executables run** (D73): the `@[csimp]` equation `MemberMap.mk'_eq_mkImpl` lets the compiler replace `mk'` by `mkImpl`, which forces the cells into a `Vector` once — the substitution exists only because the equation is kernel-checked (no `implemented_by` in `Hydro/`) |
| `SchedSem pacing` | `Sched.lean` | `Fin (mem ℓ) → StepHist α` (prefix-monotone "buffer as of step `t`"); `pacing : Pacing L mem` (per member per step: idle / stutter / tick) | the concurrent operational semantics: per-pair delivery cursors, per-member paces, consume-all-at-tick, Kleene-diagonal knots; **no quotients** — grades are erased |
| `CoupleSem pacing Tc Td (hjT : Tc ≤ Td)` | `Couple.lean`, `CoupleProj.lean`, `Transfer.lean`, `SchedMono.lean` | `CoStream {sr : SchedSem leg, rr : Values leg, wf : Prop, cpl : wf → ListLe (sr.view Tc) rr}`; tick carriers also `mono : wf → TMono sr` | the coupling corner: each op derives its `Values` decision from the machine leg (`batchDerive`, `snapDerive`, `tkOrdSelDerive`, …) and carries its realization proof; knots close by horizon induction (`co_fix_cpl`, `co_tick_fix_cpl`); in-tick ops carry the List-vs-quotient coupling per op (`CoBounded`, D60); tick-trace monotonicity (`TMono`: a member's tick trace at step `k` is a prefix of its trace at `k' ≥ k`) is recorded in the corner, not in `Sched` (D71 ruling (A): `Sched` stays minimal and auditable, the corner is where proofs live) — it is what lets a per-tick decision be read off the machine's realized tick |
| `MonoRel` | `MonoRel.lean` | pairs of `Values` carriers related by `PoolLe` | a program at `MonoRel` is its own Flo-monotonicity proof |

Supporting layers: `Trace.lean` (traces = `List σ`, cuts, scans, `scanAcrossTicks_*`,
the option-indexed `TickReads` lemmas of D63, `StabilizesAt` in `TransferTheory.lean`),
`Decisions.lean` (`BatchCuts`, `SnapshotCuts`, `OrderSelection`, `TickOrderSelection`,
`SampleTimes`, `TimerVerdicts`, `TimingPulses`, `UnfoldFuel`), `SchedCausal.lean`/
`SchedMono.lean`/`KnotTactics.lean`/`WfTactics.lean` (the knot `wf` discharge kit:
causality, tick-trace monotonicity, coupling), `TransferChecks.lean` (the `#guard`
adversary suite — the executable half of `SCHED_AUDIT.md`'s checklist).

## 4. The relational layer (`HydroRel.lean`, `HydroRelLaws.lean`, `HydroParam.lean`)

`hydro_rel_laws` generates, from the `HydroSem` class declaration itself, the binary
parametricity lift of every field (`HRel.Laws`, one law per op). An instance of
`HRelC` + `Laws` is a guarantee: `MonoHRel.lean` (the ⊑-diagonal — Flo monotonicity),
`CausalHRel.lean` (causality). (D73 retired the third, `EagerRel.lean`'s
data/denotation agreement: with `MemberMap` the executed data IS the denotation.)
`hydro def` runs `genParam` for every module, emitting `<M>_param` (single output
leg) or `<M>_param₁…ₙ` — the module's free theorem, instantiable at any `Laws`
instance. The diagonal mono corollaries `<M>_mono…` are what consumers cite for
"re-execution with larger inputs preserves my fact" (`DOCTRINE.md` §faces).

## 5. The surface (`HydroDef.lean`, `HydroTick.lean`)

```lean
hydro [unfold-hints]? def M (H : HydroSem L mem) … (dec : MDec H …) (sched : MSched H …) :
    <output wire type>
  requires R                             -- optional: the module's premise (Verus `requires`)
  ensures out => MEnsures … out :=
  -- Rust line quoted above every program line
  let w₁ := H.op …                       -- the let-chain is the Rust body, 1:1
  ghost have h₁ := callee.ensures …      -- a callee's contract face, by name
  ghost pre hp : S := …                  -- the site fact of a conditional fold / a callee's requires
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
  the closed wires inside `fix`). Ghosts refer to wires by their program names. Each
  top-level `ghost have` is HOISTED to its own theorem `M.<h>` (D71: the proof leg
  cites it instead of replaying it, so a module's ghost text is elaborated once; each
  hoisted fact gets its own heartbeat baseline, and every OTHER module is `[irreducible]`
  while it elaborates — a ghost reads a callee through its `.ensures` face, never its
  body; a ghost that legitimately needs a callee's body says `with_unfolding_all`.
  `HYDRO_GHOST_DIAG=1` prints each fact's cost).
- **`requires R`** (D71) is the module's premise, a hypothesis `hreq` for its `ghost
  pre` proofs and for the prove leg — the Verus `requires`, NOT a conditional contract
  face (`ensures` stays unconditional over outputs). **`ghost pre h : S := prf`** proves
  the site fact of a conditional fold (its `Pre` content half) or of a callee's
  `requires`, at the denotation, before the site; packaged as `M.pre_h : PreFact R S`
  and consumed by the generated `_co_wf` lemmas through `hydro_pre_close`. A
  conditional fold without its `ghost pre` does not elaborate.
- **`#guard_eval`** (D71): a `#guard` for whole-program witnesses that the kernel
  cannot afford (`#guard` on a nested-knot run took 36 min); it evaluates with the
  compiler and fails the build on `false`. Falsification witnesses and the smoke
  tests of every Paxos module use it.
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
  One `invariant` keyword (D71): when the predicate speaks of the wire alone it is
  ALSO assumable by the body of its loop input (the Verus loop invariant proper —
  conjoined into the hoisted chain's `requires`, fed to the body at exactly the Kleene
  stage the `wf` probes); a predicate over chain lets is export-only.
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
decision), `<M>_causal₁…`, `<M>_smono₁…` (D71: each output's machine leg is tick-trace
monotone — `TMono` for tick families, the trivial `TMonoTriv` for the rest; a knot's
`K_smono₁` closes it over `fix_tick` by stabilization + body monotonicity), `<M>_co_wf₁…`
(the knot/body well-formedness the corner's `cpl` needs; a `fix_tick` `wf` is four
conjuncts — causality, chain, coupling, and W4 "the body preserves `TMono`"),
`<M>_mono₁…`, `<M>_param…`, `<M>.ensures`, `<M>.<ghost>` (the hoisted ghost facts),
`<M>.pre_<h>`; per knot `<M>.<w>_co_*`, `<M>.<w>.stages*`, `<M>.<w>.inv`. The engine's per-module discipline: kernel defeq
never crosses a module or knot boundary (D40/D44 — whole-program defeq is the recurring
failure class); candidate resolvers key on head constant + projection path before any
`isDefEq` (D56).

## 7. From contract to machine theorem (`CORRESPONDENCE.md` has the full story)

`paxos_core.ensures` is a `Values` fact. `paxos_co_wf` (the five knots' `wf`, built
from generated `co_wf` artifacts) → `paxos_co_cpl` (the coupling, premise-free) →
`paxos_co_sr`/`paxos_co_rr` (the corner's legs *are* the `SchedSem` and `Values` runs) →
`paxos_safe_sched'` (the transfer lemma: Values pools above the inputs as hypotheses) →
`paxos_safe_sched` (`Paxos/CoupleWf.lean`; the pools built from the inputs truncated at
the horizon, commits carried back by `Dag.denote_causal`): machine-run agreement under
any pacing, schedule, horizon and inputs, with no satisfiability premise, no decision
witness and no Values-side input.
`cq_safe_sched'` (`Std/Quorum.lean`) is the same four-name pipeline for the shared
stage, and `two_pc_safe_sched'`/`two_pc_once_sched'` (`TwoPC.lean`) for the consumer
built on it (two module calls, no knots: the pipeline is pure composition). Liveness composes the same corner (`collect_quorum_tight`, `LIVENESS.md`).

## 7b. The production interpreter: `Project` (D74)

The interpretations of §3 run a program as one closed term. `Project` runs it as
**processes**: the elaborator compiles each `hydro def`'s body into a typed dataflow
graph, and a process is a function that steps the graph's cells *at its own members
only*, reading what arrived on its sockets and emitting what to send. The chain of
correspondence is proved all the way up to `paxos_safe_wired`
(`Paxos/WiredSafety.lean`), and `lake exe wired` runs `paxos_core` as five TCP
processes.

**Why a compilation and not a fifth instance.** A tagless-final instance sees applications
of the signature, not the program's `let`-sharing, and cannot step the body of a `fix`
once per global step — the knot's one-step delay is a property of the *graph*, not of
any operator. So `Project` is a phase of the `hydro def` pipeline (`HydroGenProject.lean`,
after `_param`), with `HydroSem` unchanged.

**The IR** (`Project/IR.lean`). `Kind` (the eight wire kinds: stream/keyed/singleton/…,
atomic variants), `Var Γ κ` (de Bruijn into a context of kinds), `Node Γ κs` (32 operator
constructors — each `HydroSem` method once, plus `call` for a module application and
`fixN` for a knot group), `Dag Γ outs` (`letN`/`ret`). `Dag.denote pacing d env` is the
graph's meaning in `SchedSem` — knots by `bekic`/`knotFix`, the same fixpoint the
denotation uses. Per module the phase emits `M.dag` and **`M.dag_denote : ∀ binders env,
Dag.denote pacing (M.dag …) env = (M (SchedSem …) … (env.get v₀) …).1 …`** — kernel-judged
`Eq.refl` for leaf modules (27), and for modules with callees/knots (17) a two-hop
proof: `M.dag_denote_graphs` (kernel; callee applications opened to component tuples
of their own `Dag.denote`, knot groups as `bekic` cascades) then `Meta.simp` with the
callees' `dag_denote` lemmas (bodies stay opaque) and a closing kernel `Eq.refl`.
Also `M.wireCodecs : Dag.Codecs (M.dag …)` — a JSON codec per network node, synthesized
from the message types' `ToJson`/`FromJson` (`Wire.lean`; `Paxos/Types.lean` derives
them, the mirror of Rust's `#[derive(Serialize, Deserialize)]`).

**The lock-step machine** (`Project/Step.lean`, spec in `ProjectStepSpec/Ops*/Main`).
`Dag.step`: one global step of every cell — stateful operators keep cells
(`ChanCell` per network edge: pending queue + delivered count; `PendingOf` for
`tick_scan`; …), a network node delivers up to the oracle's cursor then enqueues this
step's sends, a knot feeds its body's previous outputs back. **`Dag.run_spec`**: the
run's per-step output increments are the denotation's (`envIncAt (Dag.denote …) k`) —
the decisions baked into the graph (`sdec`/`ssched`) are the oracle.

**The wired machine** (`Project/Wired.lean`, spec `Project/WiredSpec`, theorems
`Project/WiredMain`, `Project/WiredRun`). `Sel L mem` picks the hosted members; `Dag.LocState sel d`
is the hosted members' cells (network cells live at the *receiver*, one queue per
sender); **`Dag.locStep sel d ora Δ s`** is one step of one process — every hosted
member ticks — the hosted cells, the hosted increments, and `Dag.Sends` (what the
hosted senders emitted); `Dag.enqueueLoc` puts another process's sends into this one's
queues; `Dag.oracleAll` delivers everything that has arrived. **`Dag.step_wired`**: a
lock-step step in which exactly the hosted members are active (`ctx.act = sel`) and
tick iff `tk` (`ctx.tick = sel && tk`; `tk = false` is a *stutter*: deliveries land, knot
feedback propagates, no tick body fires), with inputs only at them (`IdleEnv`) and the
oracle reading the graph's cursors (`OracleAt`), *is* `locStep sel tk` written back,
the rest only receiving its sends — from
**any** state whose multi-input tick cells are drained (`Dag.Drained`, true of every
state the machine produces: `step_drained`/`init_drained`). Nothing is assumed about
the other processes: **a member that is not active is inert** (D76 — `SchedSem`'s
delivery, knot feedback and `defer_tick` are gated by the member's pace, `Pace` =
idle / stutter / tick). An
**`Interleaving L mem Γ`** is just which member (`Member L mem = (ℓ : L) × Fin (mem ℓ)`
— every cluster member is a process, `Sel.single`) runs at step `k`, whether it ticks or
stutters (`I.tick k`) and the inputs; its
pacing (`I.pacing`: the running member is active, ticking per `I.tick`; the rest idle), consumed inputs (`I.env`)
and cursors (`Dag.cursorsAt`) are derived. Unconditionally: `view_active` (the active
process steps its own cells), `view_idle` (the others only receive), and
**`Interleaving.outputs`**: the active process's increments are the denotation's at
that step. `paxos_safe_wired` composes it with `paxos_core.dag_denote` and
`paxos_safe_sched`: processes running `paxos_core.dag` under any interleaving never
commit two values at one slot.

**Running it** (`WiredDemo.lean`, `lake exe wired`). Two proposers and three acceptors,
each a `Dag.locStep` loop over its `Loc p = Dag.LocState (selOf p) D` on
`127.0.0.1:9400+p`; sends go out as JSON lines (`Dag.wireEncode` with
`paxos_core.wireCodecs`), a receiver drains its inbox (`wireDecode` → `enqueueLoc`),
ticks once (an activation is one tick; a knot's pending feedback waits for the next
arrival or the idle timer). Each process is its own thread and shares nothing with the
others but its sockets (the harness holds a stop flag and reads each process's own
commit record after the stop); the run is an interleaving because each tick reads
before it sends (D76). Proposer 0 wins ballot 0, commits `1000` at slot 0; the commits
are checked slot-functional across processes. `--bench total
batch` measures throughput (≈ 10k commits/s at batch 50, ≈ 30k at batch 200 on
loopback). Keyed operators that take `K: Hash` in Rust take `[Hashable K]` here and run
through `Std.HashMap` in compiled code (`KeyedHash.lean`, `@[csimp]`); the acceptor's
log snapshot is `Std.ExtHashMap` (`HMap.lean`). Per-member values are memoized in compiled code only
(`memo`/`memo2`, `@[csimp]`-checked identities; `ENGINE_NOTES.md` "Runtime" has the
three compiler behaviours the encoding has to dodge).

## 8. Module map

Each file: prerequisites for the contract face → `hydro def` → smoke tests → census.
Census = (content nondets / adversary scheds / knot fuels), equal to the Rust `nondet!`
tally plus the cycles.

| module | Rust | def at line | census | contract highlights |
|---|---|---|---|---|
| `Std/Quorum.lean` `collect_quorum`, `collect_quorum_with_response` | `quorum.rs:7–88`, `:90–160` | 662, 906 | 1/0/0, 1/0/0 | `CQEnsures` (`emit_sound`, `emit_count`, `fails_eq`): emissions hold `min` `Ok` votes among the consumed pool; crossing is an iff, once; the error leg is `rfl`. `CQWREnsures` (`emit_mem_sound`, `emit_le`, `emit_complete`): emissions quote consumed `Ok` responses and embed with multiplicity. One shared loop invariant `CQRegInv` on the two `use::state` registers (D65) |
| `Std/RequestResponse.lean` `join_responses` | `request_response.rs:15–43` | 65 | 1/0/0 | `JREnsures`: joined outputs quote their key's metadata and consumed response |
| `TwoPC.lean` `two_pc` | `two_pc.rs:14–69` | 113 | 2/4/0 | `TPCEnsures` over `c_all_commit` and the inputs (`commit_proposed`, `commit_unanimous`, `commit_once` — the last under Rust's one-coordinator premise `mem coord = 1`), proven inline from two `collect_quorum.ensures` + `den` reads of the echo wires; **`two_pc_safe_sched'`**, **`two_pc_once_sched'`** (machine-run, premise-free, the `cq_safe_sched'` pipeline: `twoPCCoDec`/`twoPCCoSched`/`twoPCVDec`, `two_pc_co_sr/_rr/_cpl`) |
| `Paxos/Types.lean` | `Ballot`, `LogValue`, … | — | — | `Ballot` lex order (`LinearOrder`), `logMerge`/`LogTie` (paxos.rs:862's tie made a `Pre`), `rcStep` (paxos.rs:617–640's `(count, champion)` closure, literal) with `rcFold_champ` (champion ∈ entries, dominates, count ≤ holders), `OnceInv` (once-per-ballot register: B1's P1a dedup and B2's gate are the same register, D64), `LeaderDiscipline`, `rcEntries`, `rcMaxSlot` |
| `Paxos/PBallotCalc.lean` `p_ballot_calc` | `paxos.rs:348–412` | 48 | — | `PBCEnsures`: `own`, `mono`, `hasLargest_true` (`p_has_largest_ballot ≡ true`, D26) |
| `Paxos/PLeaderHeartbeat.lean` `p_leader_heartbeat` | `paxos.rs:414–482` | 82 | 3/1/0 | `PLHEnsures`: `trigger_gate` (fires only at non-leader ticks), `heartbeat_src` |
| `Paxos/AcceptorP1.lean` `acceptor_p1` | `paxos.rs:484–524` | 124 | 0/1/0 | `AP1Ensures`: `max_face`, `max_mono` (ascent as a face, D60), `reply_decomp`, `from_ballot_cap`, `from_src` |
| `Paxos/AcceptorP2.lean` `acceptor_p2` | `paxos.rs:808–899` | 148 | 2/1/0 | `AP2Ensures`: `log_len_le_ck`, `log_covers_mono`, `log_entry_src` (every published entry quotes a consumed P2a), `acks_by_acceptor`; proven from the module's internal `hlog_pool` ghost (the log as the canonical `logView` of a consumed pool); `across_ticks`+`reduce_watermark` mirrored as a persisted-pool fold (D58 audit; GC is safety-neutral) |
| `Paxos/PP1b.lean` `p_p1b` | `paxos.rs:527–593` | 249 | 3/0/0 | `PP1bEnsures` (output-shaped, D66): `accepted_src`, `leader_batch`, `pinned`, `ballot_stable`, `fails_src`; the D21 fabricated-reign regress is a face |
| `Paxos/Recommit.lean` `recommit_after_leader_election` | `paxos.rs:595–672` | — | 1/0/0 | `RCDec.order` (the fold's per-tick order, D71 — the Rust's new `assume_ordering` `nondet!`); `RCEnsures` via `RCTick` (consumer vocabulary): owned / slots nodup / slot ≤ max / above checkpoint / `value_best` (a recommit's value is a view entry's at the slot dominating all ballots) / `skipped_holders` (a skipped slot is executed or its champion is held by > f entries); the X·Y·Z order-dependence pair as `#guard_eval` |
| `Paxos/IndexPayloads.lean` `index_payloads` | `paxos.rs:776–806` | 59 | — | `IPEnsures`: `rebase_dominates`, `slots_dominate`, `slots_nodup` |
| `Paxos/SequencePayload.lean` `sequence_payload` | `paxos.rs:678–774` | 203 | 6/2/0 | `SPEnsures` (output-shaped, D64): `log_entry_open`, `log_entry_agree`, `commit_spec`, `commit_distinct`, `log_len_le_ck`, `log_covers_mono`; B2 gate as an `OnceInv` `tick` block |
| `Paxos/LeaderElection.lean` `leader_election` | `paxos.rs:253–346` | 152 | 8/3/3 | `LEEnsures`: `discipline : 1 ≤ qs → LeaderDiscipline …` (own/mono/lead_ne/pinned/reign), `view_promise`, `providers`, `max_mono`; three `forward_ref` cycles in one `fix`; B1's P1a dedup as an `OnceInv` block |
| `Paxos/PaxosCore.lean` `paxos_core` | `paxos.rs:136–246` | 121 | 14/5/5 | `PCEnsures.slot_functional`; the `a_log`/`sequencing_max_ballots` knot carries K4 (quorum intersection + the provenance regress) as its `invariant` clause over log entries |
| `Paxos/CoupleSafety.lean`, `Paxos/CoupleWf.lean` | — | — | — | `paxos_co_sr/_rr`, `paxos_co_wf`, `paxos_co_cpl`, `paxos_safe_sched'` (transfer lemma), **`paxos_safe_sched`** |
| `Paxos/Falsification.lean`, `Paxos/Exploration.lean` | `paxos.rs:311–317` (B1), the per-tick rebase (B2) | — | — | the executable falsifiers (`lake exe falsify`) and the D21 causality record (`lake exe explore`) |
| `Liveness.lean`, `LivenessChain.lean` | — | — | — | fairness vocabulary (`FairTicks`), chains (`IsCutChain`, `Exhausts`, `Ch*` temporal operators), `collect_quorum_tight`, `cq_live`/`cq_live_chain` |
| `AxCheck.lean` | — | — | — | `#print axioms` for every headline and generated artifact class — the gate's axiom oracle |
| `Project/IR.lean`, `Project/Step.lean`, `ProjectCausal/Knot/StepSpec/StepOps1–5/StepMain.lean` | — | — | — | the typed dataflow IR, its `SchedSem` denotation, the lock-step machine and **`Dag.run_spec`** |
| `Project/Wired.lean`, `ProjectWiredSpec/Main/Run.lean`, `Project/Wire.lean`, `Wire.lean` | — | — | — | one process's cells and step (`Dag.locStep`), **`Dag.step_wired`** (unconditional up to `Drained`), `Dag.step_drained`, `Interleaving` with **`view_active`/`view_idle`/`outputs`**; wire codecs, `enqueueLoc`/`oracleAll`/`quietLoc` |
| `HydroGenProject.lean` | — | — | — | the `_project` phase: `M.dag`, `M.dag_denote`, `M.wireCodecs` for all 44 modules |
| `Paxos/WiredSafety.lean` | — | — | — | **`paxos_safe_wired`**: Paxos safety for processes over a network |
| `KeyedHash.lean`, `HMap.lean` | — | — | — | D75: the hash-backed in-tick keyed operators and `reduce_watermark`'s batch step (`@[csimp]` over the list semantics; `assoc_ext`), and `HMap` — Rust's `HashMap` as a value (`Std.ExtHashMap`) with the proofs' membership/`entries` vocabulary |

Toys and checks: `HydroGenToy.lean`/`HydroGenCheck.lean` (`toy_relay`, `toy_step`,
`toy_loop`, `toy_safe_sched` — every generated artifact class pinned), `HydroTickCheck.lean`
(the `tick` construct's shapes and invariants), `HydroParamCheck.lean`, `CoupleCheck.lean`
(the D39 corner blueprint), `TransferChecks.lean` (adversary witnesses §1–§9).

## 9. Performance envelope

Full build ≈ 3¼ min on 16 cores, 843 jobs (`LeaderElection` ≈ 83 s, `SequencePayload`
≈ 44 s, `HydroTickCheck` ≈ 42 s — twenty toys, `PaxosCore` ≈ 41 s,
`Project/WiredMain` ≈ 67 s — the 32-constructor `step_wired` mutual block, everything
else < 12 s; the `_project` phase adds ≤ 4.5 s per module, D74).
Heartbeats are a proxy; proof time is the goal (D72). The engine carries no budget;
one program def carries a measured `set_option maxHeartbeats … in` (`LeaderElection`
600k) because one `hydro def` command elaborates the def and every declaration it
generates and Lean counts them together (`leader_election` ≈500M raw in all, no
single generated theorem above 90M — `scripts/heartbeat_table.sh` lists them;
`PaxosCore`'s 250k went in D73 when `conclKeyMismatch` learned to skip a callee-leg
rule against an operator-headed subject, 605M → 189M). The two time sinks D72 removed: the walks' `first | with_reducible apply r₁ |
…` over callee rules — a rule for another LEG of the same callee fails to unify only
at the bottom of the shared `Prod.snd` spine and costs ~1.7× per level on the way back
up (7 s per failing rule at depth 14; `hydro_rule_apply` skips such rules by key,
`leader_election_smono₃` 11.4 s → 16 ms) — and `LeaderElection`'s three `#guard_eval`s
interpreting a call-by-name `Values` run (207 s of the file's 320 s). D73 made `Values`
itself the executable: member carriers are `MemberMap`s whose compiled constructor
forces the cells once (`MemberMap.lean`), the `Eager` interpretation and its pins are
gone, and the three guards run in milliseconds at `Values`. Two compiler facts had to
be learned on the way (`ENGINE_NOTES.md` "Runtime"): a single-relevant-field structure
is erased to the field, so a one-field `MemberMap` was a function to the mono-phase
LCNF passes, which eta-expand local lambdas returning it and merge a let-bound map into
its read (`simpAppApp?`) — `n^depth` rebuilds, `leader_election` at four proposers
3.9M map constructions; the `boxed : Unit` field makes it a heap object (686). Ghost facts are hoisted (D71): the SP ghost blowup (161M raw heartbeats
for one `obtain`) was a default-transparency `whnf` unfolding callee modules to their
tick formers — callees are irreducible during ghost elaboration now (4.85M). Perf
questions are answered with `scripts/heartbeat_table.sh` (per declaration) and
`trace.profiler`, never by raising a budget (`ENGINE_NOTES.md`).
