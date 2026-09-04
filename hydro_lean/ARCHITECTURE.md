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
D54): content nondets `SnapDec`, `BatchDec n α ord ret` (one graded family for Rust's one
`Stream::batch`, D91: multiset increments at `NoOrder × ExactlyOnce`, sets drawn from the
pool (nothing consumed) at `NoOrder × AtLeastOnce`, slice sizes at `TotalOrder ×
ExactlyOnce`, the realized batches at `TotalOrder × AtLeastOnce` — `BatchCutsOf`),
`OrderSelDec`, `TickOrderSelDec` (a per-tick order for one tick's unordered batch —
D71), `TicksDec` (how many ticks a bounded-register read runs — `use::snapshot` of a
`Bounded` singleton, D81), timing `ClockDec` (`use::time(nondet!)`: the
wall-clock readings per tick — at `Values` the readings ARE the Atomic trace,
`ClockReadings n := Fin n → List Nat`; at the machine a **total per-tick script**
`TickScript n Nat := Fin n → Nat → Nat` indexed by the pacing's tick count, so every
tick has its reading and a process is handed one entry per activation — D84; the
corner derives the lists from the scripts at the horizon, `scriptDerive`/`sampleDerive`),
`FixDec` (knot fuel); the adversary's `TransportDec` (delivery cursors — `Unit` at
`Values`, real at the machine) and `IntervalDec` (an interval timer's per-tick fire
counts — `Unit` at `Values`, a `TickScript n Nat` at the machine; D91: `source_interval`
is a deterministic `AtLeastOnce` stream of `()`, its timing is runtime freedom the
denotation never sees, and Rust marks no `nondet!` there). Modules bundle their
sites into `…Dec` (content) and `…Sched` (adversary) records nested along the call
structure; `#nondet_census M (nondets := a) (scheds := b) (fuels := c)` is
build-enforced per module and must equal the Rust `nondet!` tally.

**Stream-level operators** (Rust names): `map`, `filterMap`, `broadcast_closed`,
`demux`, `values`, `weaken_retries`, `union`, `assume_ordering`, `fold`,
`fold_monotone`, `snapshot`, `batch` (every grade, D91 — the `TotalOrder × AtLeastOnce`
case cuts a `StutterSeq` by `stutterBatchCuts`), `source_interval (delay) (period)`
(D91: the interval timer as Rust has it, a `Stream ℓ Unit .totalOrder .atLeastOnce`;
`timeout` and `sample_every` are programs over it, `batch` and `time` —
`Std/Timers.lean`), `firstTick` (the tick-level `Stream::first`, D91), `mapTick`,
`zipTick`, `defer_tick`,
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
tick and couples it by `mono`, below); and the benchmark's (D81) `kentries`
(`KeyedStream::entries`), `kmap`, `kallTicks` (a `sliced!` leaving with a keyed stream),
`kfold` (`KeyedStream::fold`/`reduce`, `[Hashable K]`, → a `KeyedSingleton` read at
`.noOrder` cuts), `ksnapshot` (`KeyedSingleton::snapshot`), `tfold`
(`across_ticks(|s| s.fold(…))`: the fold's state retained across ticks),
`latest` (D89: `Optional::latest()` — the tick optional as `Optional<σ, L, Unbounded>`,
spelled as a singleton of an `Option`; a `snapshot` reads the **last update** at its cut,
absent before any and absent when the latest tick's optional was `None`; the cut is
asynchronous at `Values`, the latest value at `SchedSem`), `latest_monotone` (the same read
as a `Monotonic` singleton; its corner `wf` carries `∀ i, Ascending optNat (t.rr i)` —
`none ⊑ some x ⊑ some y` on the update trace — the content half a `ghost pre` proves, the
checkpoint knot's invariant in `paxos_protocol`; off that premise the Values reads block
at the first non-ascending prefix, a branch dead on every coupled run and queued for the
ghost-parameter design), `time`, `snapshot_bounded`.

**In-tick operators** over `BoundedStream`/`BoundedSingleton` (the body of a Rust
`sliced!`, D60–D61), with Rust's grade constraints in their types: `bmap`,
`bfilterMap`, `bflatMapOrdered`/`bflatMapUnordered`, `bofList`, `bcount` (ExactlyOnce),
`bfold` (`FoldOk … Pre`), `benumerate`/`bfirst` (TotalOrder), `bcrossSingleton`, `bchain`,
`bweakenOrder`, `bfilter`, `bjoin`, `bantiJoin`, `bfilterNotIn`, `bmax`, `bfilterIf`;
the keyed API `binto_keyed`, `bkfold` (`FoldOk … Pre`, value-only closure as in Rust),
`bksFilter`/`bksMap`/`bksKeys`/`bksEntries` (D70); singleton API `bsPure`/`bsMap`/
`bsZip`, `boMap`/`boUnwrapOr`/`boFilter`/`boIsSome`; D81's 21 (`boIsNone`, `boZip`,
`boFilterMap`, `boIntoStream`, `boZipSingleton`, `bsFilterIf`, `bsFilterMap`,
`bsZipOptional`, `bmin`, `breduce`, `bsort`, `bcrossOptional`, the keyed-stream `bkmap`/
`bkcrossSingleton`/`bkchain`/`bkweakenOrder`/`bkreduce`, the keyed-singleton `bksJoin`/
`bksFilterKeyNotIn`/`bksKeyCount`/`bksIntoStream`); and the former `tick_scan sts ins
outs` (a structural fold over a location's ticks, shapes zipped at the type level —
leaves `.sing`/`.stream`/`.ksing`/`.kstream`, the last D81's `kyield`) that the `tick`
construct elaborates to. A tick `state` may be a `BoundedKeyedSingleton K V` seeded
empty (`use::state_null::<KeyedSingleton<…>>()`, D81).

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
`Decisions.lean` (`BatchCuts`, `RetryBatchCuts`, `OrderedBatchCuts`, `StutterBatchCuts`
(`BatchCutsOf` by grade), `SnapshotCuts`, `OrderSelection`, `TickOrderSelection`, `ClockReadings` — the denotation's
timing lists; `TickScript`/`scriptTrace`/`flaggedBelow` — the machine's per-tick
scripts, D84; `UnfoldFuel`), `SchedCausal.lean`/
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
denotation uses. Per module the phase emits `M.dag` — **pacing-free**: the data terms'
`SchedSem L mem pacing` residue is taken at `Pacing.idle`, so one graph serves every
pacing and an `Interleaving` is of a plain `Dag` (D79) — and **`M.dag_denote : ∀ pacing
binders env, Dag.denote pacing (M.dag …) env = (M (SchedSem L mem pacing) … (env.get v₀) …).1 …`** — kernel-judged
`Eq.refl` for leaf modules (27), and for modules with callees/knots (17) a two-hop
proof: `M.dag_denote_graphs` (kernel; callee applications opened to component tuples
of their own `Dag.denote`, knot groups as `bekic` cascades) then `Meta.simp` with the
callees' `dag_denote` lemmas (bodies stay opaque) and a closing kernel `Eq.refl`.
Also `M.wireCodecs : Dag.Codecs (M.dag …)` — a binary codec per network node, synthesized
from the message types' `Wire` instances (`Wire.lean`: fields in order, LEB128 naturals, frames = varint size then the bytes with the law that any cut of a stream of frames parses to the frames that fit — D90;
one-byte tags, length-prefixed lists — the mirror of Rust's `#[derive(Serialize,
Deserialize)]` under `bincode`; `Paxos/Types.lean` writes the message structs' instances
at their definitions). No theorem mentions the representation: `decodeAt`/`wireEncode`
use `cod.dec`/`cod.enc` only, and a process's inbox is a given of the headlines.

**The lock-step machine** (`Project/Step.lean`, spec in `Project/Model/StepSpec`, `Project/StepOps*`, `Project/Model/StepMain`).
`Dag.step`: one global step of every cell — stateful operators keep cells
(`ChanCell` per network edge: pending queue + delivered count; `PendingOf` for
`tick_scan`; …), a network node delivers up to the oracle's cursor then enqueues this
step's sends, a knot feeds its body's previous outputs back. The graph carries no
decisions (D77): `Dag.Sched d` — per network node the per-(receiver, sender, step)
cursors, per timer node the per-member verdict lists, `Unit` elsewhere — is a separate
argument of `Dag.denote`/`Dag.run`, and the generated `M.schedOf` / `M.recordOf`
(`M.schedOf_recordOf`, kernel `rfl`) convert between it and the module's decision and
schedule bundles. **`Dag.run_spec`**: the run's per-step output increments are the
denotation's (`envIncAt (Dag.denote … sch …) k`) under the same schedule.

**Single slots** (D85). A `Singleton`/`Optional` wire carries one value per tick, so its
production increment is a slot: `Kind.delta` of the atomic kinds is `Fin (mem ℓ) → Option X`
(the lock-step `Kind.incAt` is `(incTrace …).head?`), every atomic emission is `some v`/`none`,
and the cells that buffer atomics append the slot (`zipTick`'s and `reduce_watermark`'s
pendings, `++ Δ.toList`); the timer nodes keep a tick counter and `latest`/`latest_monotone`
a register (the tick's optional, `None` included).
The fact that makes `head?` lossless is per operator: **`TPaced act x`**
(`SchedPaced.lean`) — an Atomic trace gains at most one entry per step and none at an
inactive step — holds of every atomic-producing operator on paced inputs
(`tpaced_<op>`, including the knot wire: `gateView_paced`, `tpaced_tick_diag`), is
threaded through `Dag.denote_tpaced`/`HTPaced`, and is a hypothesis of `Dag.run_spec`.
An interleaving's inputs as wires (`Interleaving.wires`) are monotone and single-slot by
construction (`wires_mono`, `wires_paced` via `env_idleSlots`), so `Interleaving.outputs`
takes no hypotheses and no program proof sees the fact. `defer_tick` is the
FIFO it is in DFIR (`defer_tick.rs:6`, "releases them at the next time boundary"): the
machine's `defer_tick init t` is `(init :: t).take (deferReleased …)` — a tick releases
the buffered entry if there is one — and the production cell is the buffer itself
(`[init]`, then `s ++ Δ.toList`, a tick emits its head). On inputs at most one tick
behind the tick count (every Rust-expressible wire) the released count is the tick count
(`deferReleased_eq_ticks`); the step form is what the FIFO computes on *any* input, which
is what `spec_defer_tick` needs — the model follows production, not the proof.

**The wired machine** (`Project/Wired.lean`, spec `Project/Model/WiredSpec`, theorems
`Project/Model/WiredMain`, `Project/Model/WiredRun`). A process is a `Member L mem = (ℓ : L) × Fin (mem ℓ)`;
`Hosted m ℓ` is its index at `ℓ` (a subsingleton — the one `i` with `⟨ℓ, i⟩ = m`);
`Dag.LocState m d` is the member's cells (network cells live at the *receiver*, one queue per
sender); **`Dag.locStep m tk d ora Δ s`** is one step of one process — the member active,
ticking iff `tk` — its cells, its increments, and
`Dag.Sends` (what it emitted on its channels); the oracle `ora : Dag.Oracle m d` is
what the process reads from its execution this step — its cursors, and **this tick's**
timing (the clock reading, the timeout verdict, the pulse, the sample flags: `Nat`/`Bool`
per hosted member, never a history — D84); `Dag.enqueueLoc` puts another process's sends
into this one's queues; `Dag.oracleOf tm s` delivers everything that has arrived at the
timing `tm : Dag.Timing m d` the shell read this activation, and `Dag.timingAt sch s`
reads the lock-step machine's scripts at the cells' tick counts — the one seam between a
process and the schedule (`Dag.oracleAll sch s := oracleOf (timingAt sch s) s`).
**`Dag.step_wired`**: a
lock-step step in which exactly the hosted members are active (`ctx.act = sel`) and
tick iff `tk` (`ctx.tick = sel && tk`; `tk = false` is a *stutter*: deliveries land, knot
feedback propagates, no tick body fires), with inputs only at them (`IdleEnv`) and the
oracle reading the graph's cursors (`OracleAt`), *is* `locStep m tk` written back,
the rest only receiving its sends — from
**any** state whose multi-input tick cells are drained (`Dag.Drained`, true of every
state the machine produces: `step_drained`/`init_drained`). Nothing is assumed about
the other processes: **a member that is not active is inert** (D76 — `SchedSem`'s
delivery, knot feedback and `defer_tick` are gated by the member's pace, `Pace` =
idle / stutter / tick). An
**`Interleaving`** is just which member (`I.who k : Member L mem`
— every cluster member is a process) runs at step `k`, whether it ticks or
stutters (`I.tick k`) and the inputs; its
pacing (`I.pacing`: the running member is active, ticking per `I.tick`; the rest idle), consumed inputs (`I.env`)
and cursors (`Dag.cursorsAt`) are derived. Unconditionally: `view_active` (the active
process steps its own cells), `view_idle` (the others only receive), and
**`Interleaving.outputs`**: the active process's increments are the denotation's at
that step. `paxos_safe_wired` composes it with `paxos_core.dag_denote` and
`paxos_safe_sched`: processes running `paxos_core.dag` under any interleaving never
commit two values at one slot.

**Running it** (`WiredDemo.lean`, `lake exe wired`). Two proposers and three acceptors,
each a `Dag.iterateBytes` loop over its `Loc p = Dag.LocState (selOf p) D` on
`127.0.0.1:9400+p`; sends go out as one frame per message (`Dag.wireEncode` with
`paxos_core.wireCodecs`, `Wire.frame`: varint size, then the bytes — `frameMsgs`), a
receiver hands the library each connection's kept tail plus whatever its socket delivered
since, cut wherever TCP cut it (D90); the library finds the complete frames
(`parseFrames`, the framing law `Wire.framesFrom_prefix_frameAll`), keeps the tails, builds
the inbox in one walk (`Dag.inboxOfWire`, `Dag.inboxOf` of the decoding by
`inboxOfWire_eq` → `enqueueInbox`), ticks once and then **settles** (`Dag.iterate`, D93:
stutter passes while a knot slot of the process holds feedback, a further tick — at the
quieted timing — while a tick input holds what the stutters put there, bounded by
`Dag.settleFuel`), so the activation emits what DFIR's one tick emits and the process may
idle until its sockets or timers wake it. `wired --traces` runs the same body along the
scripted byte-level traces of `WireTraceCheck.lean`, including the DFIR wake rule as a
greedy schedule (`lazy`). Each process is its own thread and shares nothing with the
others but its sockets (the harness holds a stop flag and reads each process's own
commit record after the stop); the run is an interleaving because each tick reads
before it sends (D76). Proposer 0 wins ballot 0, commits `1000` at slot 0; the commits
are checked slot-functional across processes (the witness only: the open-loop `--bench`
mode was removed in D92; `bench` is the measured system). Keyed operators that take `K: Hash` in Rust take `[Hashable K]` here and run
through `Std.HashMap` in compiled code (`KeyedHash.lean`, `@[csimp]`); the acceptor's
log snapshot is `Std.ExtHashMap` (`HMap.lean`). Per-member values are memoized in compiled code only
(`memo`/`memo2`, `@[csimp]`-checked identities; `ENGINE_NOTES.md` "Runtime" has the
compiler behaviours the encoding has to dodge). In compiled code `Dag.locStep` is
`Dag.locStepImpl` (`@[csimp] Dag.locStep_eq_impl`, D86): nodes the process does not host
are skipped (`Node.locStep_idle`: their step is the identity on the cells and empty
increments), and a callee's or knot body's cells are not re-materialized at each level of
nesting (`Node.memoStateImpl_eq`). What a process actually runs per activation is
**one walk** of the graph (`Project/Fused.lean`, D92): the harness calls
`Dag.iterateBytes` — the function `paxos_safe_wired` and `WireTrace.act` are stated over,
verbatim — and the compiler substitutes `Dag.iterateBytesImpl` (`@[csimp]
Dag.iterateBytes_eq_impl`), which parses each connection once, groups the messages by network node in one pass (`bucketArray`, so a node
decodes only its own: `Node.inboxOfWire_bucketOf`), and visits each node once doing its
inbox, enqueue, materialization, timing, oracle, step and encode together
(`Node.activatePlain`, `Node.activateFused`/`Dag.activateFused`; the unfused composition is
`Dag.activateSpec` and `Dag.activateFused_eq` the equality), skipping nodes the process does
not touch. Sends leave as frames written in place (`frameOneInto`, `@[csimp]
frameMsgs_eq_impl`; payloads by the `Wire` class's `put`, whose law `put_eq` makes `encode`
the specification and `put` the encoder — `Codec.ofWire_enc`), routed by the harness to the
processes that receive them: the receiver cluster's members, and at a `demux` the one whose
address the payload's first varint names (`Dag.wireRoutes`, `Shell.routeSends`;
`demuxTarget`, which reads the address of any `Codec.ofWire` payload — `demuxTarget_ofWire`). `BenchDemo.lean` (`lake exe bench`) runs `paxos_bench`
as ten OS processes (`bench --proc P`), each with an `Outbox` writer thread per connection
and a condition-variable inbox; `HYDRO_BENCH_PROFILE`/`HYDRO_BENCH_NODES`/
`HYDRO_BENCH_TRACE`, `bench idle P N` and `scripts/bench_perf_role.sh P` (perf on one
role, the machine thread separated) are the profiling faces (`Project/Profile.lean`).
FINDINGS D86–D87 and D92 are the measurements and the cost model: fixed per-activation
walks, per-in-flight register re-reads, per-request hash-map operations at Lean's
constant; after D92 the leader's per-tick re-fold of all in-flight state (the program's
`use::state`, which DFIR runs incrementally) is 41 % of its time and the wall.

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
| `Std/BenchClient.lean` `bench_client`, `compute_throughput_latency`, `aggregate_bench_results` | `bench_client/mod.rs:44–146`, `:130–202`, `:204–270` | — | 6/0/0, 2/0/0, 3/2/0 | D81/D82: the virtual-client spawner (`snapshot_bounded`/`TicksDec`), the start/end stamping block (`H.time`/`ClockDec`, `tassume_ordering` + keyed reduce, `BoundedKeyedSingleton` state, `kyield`), the measurement folds; `HdrHistogram` — a faithful port of hdrhistogram 7.5.4 `Histogram::<u64>::new(3)` (sparse `HMap`, `record_comm`/`add_comm_ok` as the folds' `FoldOk`) |
| `Paxos/KvReplica.lean` `sequence_payloads`, `kv_replica` | `kv_replica/sequence_payloads.rs:9–57`, `kv_replica/mod.rs:46–111` | — | 1/0/0, 1/0/2 | `SequencedKv` with a `LinearOrder` (lexicographic; Rust's `Ord` is `seq`-only, SCHED_AUDIT); the two tick cycles one `fix` (`tfold` for the KV store and the max checkpoint) |
| `Paxos/PaxosWithClient.lean` `payloads_to_leader`, `paxos_with_client` | `paxos_with_client.rs:52–100` (the closure), `:47–110` | — | 3/2/0, 17/7/6 | `PWCEnsures.slot_functional` — `paxos_core`'s headline on `with_client`'s one output, by the knot-replayed `ghost have hcore` |
| `Paxos/PaxosBench.lean` `inc_i32_workload_generator`, `tag_with_sender`, `acceptor_checkpoint_from_replicas`, `paxos_protocol`, `paxos_bench` | `paxos_bench.rs:138–148`, `:38–43`, `:69–96`, `:32–120`, `:16–135` | — | 0/0/0, 1/0/0, 1/1/0, 21/10/9, 32/12/11 | `ACQEnsures.ascending : nR = f + 1 → ∀ i, Ascending ValueOrder.optNat (out i)` (`checkpointQuorumMin_ascends`); `paxos_protocol`'s checkpoint knot `invariant` (assumed by the body for `latest_monotone`, stepped by the block's `ensures`); `requires variant = .guarded ∧ mem replicas = f + 1` |
| `BenchDemo.lean` (`lake exe bench`) | — | — | — | `paxos_bench.dag` as ten TCP processes with the clock and timers read online — the benchmark modules' executable witness (D83) |
| `Paxos/CoupleSafety.lean`, `Paxos/CoupleWf.lean` | — | — | — | `paxos_co_sr/_rr`, `paxos_co_wf`, `paxos_co_cpl`, `paxos_safe_sched'` (transfer lemma), **`paxos_safe_sched`** |
| `Paxos/Falsification.lean`, `Paxos/Exploration.lean` | `paxos.rs:311–317` (B1), the per-tick rebase (B2) | — | — | the executable falsifiers (`lake exe falsify`) and the D21 causality record (`lake exe explore`) |
| `Liveness.lean`, `LivenessChain.lean` | — | — | — | fairness vocabulary (`FairTicks`), chains (`IsCutChain`, `Exhausts`, `Ch*` temporal operators), `collect_quorum_tight`, `cq_live`/`cq_live_chain` |
| `AxCheck.lean` | — | — | — | `#print axioms` for every headline and generated artifact class — the gate's axiom oracle |
| `Project/IR.lean`, `Project/Step.lean`, `Project/{Causal,Knot,StepSpec,StepOps1–5,StepMain}.lean` | — | — | — | the typed dataflow IR (no decisions in it; `Dag.Sched` is the execution's, D77), its `SchedSem` denotation, the lock-step machine and **`Dag.run_spec`** |
| `Project/Wired.lean`, `Project/{WiredSpec,WiredMain,WiredRun}.lean`, `Project/Wire.lean`, `Wire.lean` | — | — | — | one process's cells and step (`Dag.locStep`), **`Dag.step_wired`** (unconditional up to `Drained`), `Dag.step_drained`, `Interleaving` with **`view_active`/`view_idle`/`outputs`**; wire codecs, `enqueueLoc`/`oracleAll` |
| `HydroGenProject.lean` | — | — | — | the `_project` phase: `M.dag`, `M.schedOf`/`M.recordOf`/`M.schedOf_recordOf` (bundles ↔ `Dag.Sched`, D77), `M.dag_denote`, `M.wireCodecs` for all 44 modules |
| `Project/Process.lean` (production), `Project/Shell.lean` (production), `Project/Model/DistributedModel.lean` (model) | — | — | — | one pass `Dag.pass` (inbox → deliver all → `locStep` → nothing in flight) and its bridge **`pass_eq`**, **`Interleaving.loop_pass`/`loop_idle`** (D78, renamed D93); the activation **`Dag.iterate`** = a pass, then `Dag.settle` (D93: `Dag.pass m false` while `Dag.knotPending`, `Dag.pass m true` at `Dag.Timing.quiet` while `Dag.batchPending`, counter `Dag.settleFuel`; `passState`/`settlePass`, `settle_getElem?`, `settle_foldl_fst`); `Dag.iterateWire` (batches of messages) and **`Dag.iterateBytes`** (per connection the unread bytes; the complete frames are the messages, the tails come back — what the shells call, D90), their one-pass forms `Dag.passWire`/`Dag.passBytes`; `Interleaving.Batched` (any batches whose decode is the arrivals) with **`loop_pass_wire`**, `view_idle_wire`/`view_active_wire` (D88/D90); `Shell`: `hello`, `groupChunks`, `chansOf`, `storeTails` — the shells' share between the sockets and the body |
| `Project/Fused.lean` (production) | — | — | — | D92: the one-walk pass — `Node.activatePlain`/`Node.activateFused`/`Dag.activateFused` with `Dag.activateSpec` and **`Dag.activateFused_eq`**; decode buckets `bucketArray`/`bucketOf` (`bucketArray_getD`, `Node.inboxOfWire_bucketOf`); D93: `Dag.settleImpl` (one walk per settling pass, `settleImpl_eq`); `Dag.iterateBytesImpl` and `@[csimp] Dag.iterateBytes_eq_impl` — what `wired`/`bench`'s calls to `Dag.iterateBytes` compile to |
| `Project/Model/WireTrace.lean`, `WireTraceCheck.lean` | — | — | — | D88–D90, D93: a **`WireTrace`** (who ran, inputs, readings, bytes read per connection — no proof fields: every quadruple is a trace) runs `Dag.iterateBytes` over per-connection byte queues (`run`). The proof layer is the pass-level **`PassTrace`** (a fifth field, `tick`, per step; `Dag.passBytes`): `RunInv` (cursors within the stream, tails past the last frame) and `parse_eq` turn bytes read into messages parsed (`pc`, `PcOk`); the induced interleaving (`traceSched`), the queue invariant `Dag.TraceQueues`, and **`pass_eq_of_inv`/`PassTrace.streamOutputsAt_eq`**: a pass trace's run is the model's run at the wire. A `WireTrace` **expands** to a `PassTrace` (`expand`: each activation a block of passes — the tick at `blockStart`, then the settling passes with `passFlag`, quieted inputs and readings, nothing read; `Expands`/`Inner`, **`expands`**, **`streamOutputsAt_expand`**, **`WireTrace.streamOutputsAt_eq`**); `runOutputs` the one-pass evaluator; `WireTraceCheck`: six scripted traces on the Paxos graph (`wired --traces`), `lazy` the DFIR wake rule as a greedy schedule |
| `Paxos/WiredSafety.lean` | — | — | — | **`paxos_safe_wired`**: Paxos safety for processes over a network, stated over a `WireTrace` — the byte-level loop body `Dag.iterateBytes` they run along any history of their connections (D78, D88, D90) |
| `Std/Timers.lean`, `Duration.lean` | ✓ | ✓ | ✓ | D89: `timeout` (`stream/mod.rs`; `ensures fresh_not_expired`) and `sample_every` (`optional.rs`; `ensures sampled_src`) as programs over `time`/`source_interval` + `batch`/`firstTick` (D91); durations in µs, `Duration.from_secs`/`from_millis` where the Rust calls them |
| `KeyedHash.lean`, `HMap.lean` | — | — | — | D75: the hash-backed in-tick keyed operators and `reduce_watermark`'s batch step (`@[csimp]` over the list semantics; `assoc_ext`), D83: the benchmark client's `reduce`/`join_keyed_singleton`/`filter_key_not_in` (`keyedReduceAssocH`, `SchedKMap.joinH`, `filterKeyNotInH`), and `HMap` — Rust's `HashMap` as a value (`Std.ExtHashMap`) with the proofs' membership/`entries` vocabulary |

Toys and checks: `HydroGenToy.lean`/`HydroGenCheck.lean` (`toy_relay`, `toy_step`,
`toy_loop`, `toy_safe_sched` — every generated artifact class pinned), `HydroTickCheck.lean`
(the `tick` construct's shapes and invariants), `HydroParamCheck.lean`, `CoupleCheck.lean`
(the D39 corner blueprint), `TransferChecks.lean` (adversary witnesses §1–§9).

## 9. Performance envelope

Full build ≈ 5 min on 16 cores, 853 jobs (`LeaderElection` ≈ 77 s, `SequencePayload`
≈ 46 s, `HydroTickCheck` ≈ 45 s — twenty-three toys, `PaxosCore` ≈ 48 s,
`Project/Model/WiredMain` ≈ 67 s — the 32-constructor `step_wired` mutual block,
`Paxos/PaxosBench` ≈ 140 s — the deepest nesting, half of it the production-graph
lemmas (D83 queue), `Paxos/PaxosWithClient` ≈ 30 s, everything else < 15 s; the
`_project` phase adds ≤ 4.5 s per leaf module, D74). D81's simp attributes took the
pre-benchmark build from 4m50s to 2m51s (the class-size tax, `ENGINE_NOTES.md`).
Heartbeats are a proxy; proof time is the goal (D72). The engine carries no budget;
two program defs carry a measured `set_option maxHeartbeats … in` (`LeaderElection`
600k, `PaxosCore` 300k — back since D81's signature growth, re-measured at D91) because one `hydro def` command elaborates the def and every declaration it
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
