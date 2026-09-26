# Where the Values ↔ Sched correspondence lives

A guided tour for a reader who knows distributed-systems verification
(simulation relations, refinement mappings, network semantics, Paxos
invariants) but not this codebase: **where are the proofs that the
denotational semantics (`Values`) matches the concurrent step machine
(`SchedSem`), and what is each one doing?**

File/line pointers are as of this commit; line numbers may drift —
theorem and definition names are the stable anchors.

## The claim, in one paragraph

Programs are written once against a combinator signature
(`HydroSem`, `Sem.lean`) and interpreted many ways. Two of the
interpretations matter here: **`Values`** (`Values.lean`) is the
abstract spec — cluster streams are graded quotient pools (multisets,
stutter-sequences, …) and all nondeterminism is explicit *decision
data*; **`SchedSem`** (`Sched.lean`) is the concrete concurrent
machine — plain lists on a global step clock, asynchronous delivery,
no quotients anywhere. The correspondence theorems say: **every
machine run, under every schedule, at every finite horizon, is covered
by a denotational run at decisions *derived from the schedule*** — and
at every observation the program actually makes (fold snapshots, tick
batches) the coverage is an **equality**, not just an inclusion. The
payoff theorem is `paxos_safe_sched'` (`Paxos/CoupleWf.lean`):
per-slot agreement of Paxos commits holds for the *step machine* at
every schedule and horizon — **premise-free** (no satisfiability
hypothesis, no decision argument) — obtained by transporting the
denotational contract across the correspondence.

```
                    Values  (abstract: graded quotient pools,
                   /   ↑   \          nondeterminism = decision data)
      RelSem  ────┘    │    └────  MonoRel (monotonicity for free)
  (∃-packaging:        │
   sets of Values      │  CorrSem   (Transfer.lean: the simulation —
   runs)               │            machine run ⊑ some Values run)
                       │  CoupleSem (Couple.lean: the corner — each op
                       │            DERIVES its Values decision from
                       ↓            the machine leg and carries the
                    SchedSem        coupling proof in its carrier)
                   (concrete: lists, steps,
                    delivery cursors)
```

Five instances of one signature; a program's text is written once and
instantiated at each. `CorrSem` and `CoupleSem` are *coupling*
instances: their carriers pair a `SchedSem` run with `Values` runs and
the relation between them, so instantiating a program at them **is**
running the simulation argument over its syntax, operator by operator.
(`CoupleSem` is the second-generation coupling — the first, a
`∀ d`-quantified "square" with a satisfiability side condition δ, was
falsified for cyclic programs and replaced; the historical note in
Walkthrough C tells that story.)

## Rosetta table: what you expect → what we have

| Classic concept | Our artifact | Where |
|---|---|---|
| Abstract specification / atomic semantics | `Values` | `Values.lean` (`def Values`, l.152) |
| Concrete operational semantics | `SchedSem` | `Sched.lean` (`def SchedSem`, l.298) |
| Network / adversary model (async, unbounded delay, FIFO channels, crash-quiet nodes) | per-pair delivery cursors + tick pacing + emission linearizations | `Sched.lean` header (the `…Dec` vocabulary; time model in the header and FINDINGS D31); audited definition-by-definition in [the last section below](#auditing-the-trust-boundary-the-sched-machine) |
| Simulation relation / coupling invariant | `ListLe` (per-wire) lifted to per-step-∃ carriers `StreamRel`, `KeyedRel`, … | `Transfer.lean` l.44, l.757+ |
| The per-transition step lemma of a simulation proof | per-**operator** coupling lemmas: `corr_values`, `corr_union`, `corr_weaken`, `corr_snapshot`, `corr_batch`, `corr_batch_ordered` | `Transfer.lean`, section "Per-op coupling lemmas" |
| "Every transition preserves the invariant" (the whole induction) | **totality of the `CorrSem` instance** (l.1015): every op must fill its coupling field or the instance doesn't typecheck | `Transfer.lean` |
| Refinement mapping / history variables (Abadi–Lamport) | **derived decisions**: `cutsLen`, `cutsMS`, `snapCut`, `batchDerive`, `snapDerive` compute the abstract nondeterminism *from* the concrete run | `Transfer.lean` (l.192–380 and the "Named derived decisions" section at the end); walked end-to-end in [its own section below](#the-refinement-mapping-computed-batchderive-end-to-end) |
| The main simulation theorem | `sched_snap_eq` (read agreement), `snapshot_tight`, `batch_tight`: ∀ schedule ∀ horizon ∃ derived decisions with **equality** at observations | `Transfer.lean` l.307; `TransferTheory.lean` l.269, l.291 |
| Observing a monotone/commutative aggregate mid-run (CRDT-style state, snapshot consistency) | graded fold + cut-chain decisions: `PoolFold`/`FoldOkP`, `snapshotCuts` (a decision is an ascending *chain*; earlier cuts restrict later ones), `snapTrace_ascending` | `Grades.lean` l.396/l.362, `Trace.lean` l.391, `Values.lean` l.43; walked in [its own section below](#unbounded-commutative-folds-snapshots-of-a-moving-aggregate) |
| "Did you model enough schedules?" (adversary coverage) | 8 executable `#guard`-enforced witnesses: interleaving, latency, FIFO, stutter ticks, at-least-once birth, fixpoint race, snapshot intermediates, stabilization | `TransferChecks.lean` |
| Completeness / bisimulation / backward simulation | **deliberately absent** — adequacy (∀ decisions ∃ schedule) is *false by design*, with a counterexample | `TransferTheory.lean` header, "adequacy is false by design" |
| Quorum intersection (the pigeonhole) | `nodup_inter_of_length` (l.20) inside `paxos_core_agree` (K4, l.104; see the `-- quorum intersection` comment near l.265) | `Paxos/PaxosCoreLemmas.lean` |
| The protocol safety invariant (promise/ballot provenance) | K4's provenance regress + per-module contracts (`AcceptorP1` "Ok pins the max", `PP1b` quorum gate, `SequencePayload` commit calculus) | `Paxos/*.lean` (colocated `Ensures` records) |
| "Safety holds in **all** executions" | `paxos_safe_sched'`: SlotFunctional over `SchedSem` at every pacing, schedule, horizon — premise-free | `Paxos/CoupleWf.lean` (theorem near the end of the file) |
| Non-vacuity (your model isn't empty) | nothing to inhabit: the corner **constructs** its coupling per op (no satisfiability side condition exists to be vacuous) + the executable end-to-end run `lake exe v2paxos` | `Couple.lean` (`cpl` fields); `V2Paxos.lean` |
| Trusted base | Lean kernel + `propext`, `Classical.choice`, `Quot.sound` (audited) | `AxCheck.lean` (`#print axioms` on every headline) |

Three of these rows deserve immediate elaboration for a distsys
reader:

**The refinement mapping runs "backwards", constructively.** In a
classic refinement you map concrete states to abstract states, and
when the abstract side is nondeterministic you reach for history or
prophecy variables. Here the abstract side's nondeterminism is
first-class *data* (decision records), so the mapping is a function
you can just compute: given a schedule and horizon, `snapCut`/
`batchDerive`/`cutsMS` read off *which* abstract decisions this
schedule realized (which tick consumed which prefix/increment). No
prophecy needed — the concrete run determines the abstract
nondeterminism, and the theorem quantifies ∃ over exactly that
witness.

**The simulation induction is per-operator, never per-program.**
There is no "now do the induction over the transition system of
Paxos" step anywhere. Each combinator proves its own coupling
preservation once, generically (`corr_values` is the
message-reordering case, `corr_snapshot` is the observation case, …),
and the type system assembles them: a program instantiated at
`CorrSem` *is* coupled, by construction. Totality is the honesty
check in both directions — an op whose coupling field could not be
filled would mean either the machine reveals more than the abstract
side can cover, or the decision space is too small to express a real
interleaving.

**The network model is checked by executable witnesses, not prose.**
`TransferChecks.lean` is the answer to "but did your machine actually
allow the adversarial behavior my intuition worries about?": each
classic behavior is a tiny concrete program + schedule whose
outcome is `#guard`-checked at build time — cross-sender reordering
(two cursor schedules produce two orders of the same multiset),
unbounded latency and silence (a stalled cursor), per-pair FIFO
(delivered views are prefixes of sent — TCP; only *cross*-sender
shuffles are schedule-reachable), stutter ticks (finer pacing ⇒ more
empty batches and repeated snapshot reads), at-least-once duplication
born at sampling (not in transport — `TCP.fail_stop` preserves the
grade), the **fixpoint race** (a younger message's cycle-offspring
overtakes a stalled elder through a knot — and the raced order is
still covered by the denotational fixpoint), and snapshot
intermediates (pacing is captured; within-step micro-order is
quotiented).

## How the proof is layered

There are two coupling instances because there are two jobs:

1. **`CorrSem`** (`Transfer.lean`) carries the simulation itself.
   A stream carrier is a machine history paired with a covering set
   of denotational runs, related per-step-∃:

   ```lean
   def StreamRel (n : Nat) (α : Type) [DecidableEq α] (ord : StrOrd)
       (ret : Retries) (h : Fin n → StepHist α)
       (V : Set (Fin n → PoolCarrier α ord ret)) : Prop :=
     ∀ t, ∃ v ∈ V, ∀ i, ListLe ord ret ((h i).view t) (v i)
   ```

   "At every step, the machine's buffer sits below *some* abstract
   run" — the witness may vary with the step, which is exactly what
   lets cycles couple (step `t` couples to the depth-`t+1` Kleene
   iterate). `ListLe` is the grade-indexed relation: list-prefix at
   `TotalOrder`, sub-multiset at `NoOrder`, destuttered/support
   versions at `AtLeastOnce`.

2. **`CoupleSem`** (`Couple.lean`) — the *corner* — makes the coupled
   run available **as data with its proof attached**, so denotational
   contracts close onto machine runs with per-module (never
   per-program) proof. Each carrier packs four things
   (`CoStream`, `Couple.lean`):

   ```lean
   sr  : Fin n → StepHist α          -- the machine leg, SchedSem verbatim
   rr  : Fin n → PoolCarrier α ord ret  -- a PLAIN Values leg
   wf  : Prop                        -- residual well-formedness
   cpl : wf → ∀ i, ListLe ord ret ((sr i).view Tc) (rr i)
   ```

   at two ambient horizons (couple-at-`Tc`, derive-at-`Td`,
   `Tc ≤ Td`). The trick (FINDINGS D39): every decision-mediated op
   **derives its own `Values` decision from its machine leg** — e.g.
   `batch`'s abstract leg is
   `(Values …).batch s.rr (batchDerive (pacing ℓ) Td s.sr)` — so each
   op's `cpl` is just its realization theorem applied at its own
   derived decision. There is no decision environment, no lens
   record, and **no satisfiability side condition**: the abstract run
   is constructed, not postulated. Knots (`fix`) close by induction
   on the horizon through the machine fixpoint equation
   (`co_fix_cpl`/`co_tick_fix_cpl`); what remains per knot is the
   `wf` triple — causality of the body, Kleene ascent, and the graded
   coupling step — discharged with the instance-generic knot bodies
   of `HydroSem.fix` (FINDINGS D38) and the tactic kit of
   `SchedCausal.lean`/`KnotTactics.lean`/`WfTactics.lean`.

3. **`CoupleProj.lean`** supplies the glue: for every operator,
   once-proven `rfl` lemmas identify the corner's projections with
   the corresponding `Values`/`SchedSem` op, and the
   `co_transfer [defs…]` macro pushes `.rr` / `.sr` through a
   program's definitions op by op. One wrinkle the square era
   discovered the hard way (FINDINGS D40): kernel defeq through `k`
   nested knots is exponential in `k`, so whole-program projection
   identities are unprovable past two knots — the naming identities
   are therefore stated **per module** (generated at each program
   file's tail by `HydroGen`/`hydro_glue`/`hydro_knot` — FINDINGS
   D44–D45) and assembled with modules folded,
   which is linear. *The per-program content of a machine-run safety
   theorem is its module naming lemmas plus one contract
   application.*

## Walkthrough A: `cq_safe_sched'` — the smallest complete pipeline

`Std/Quorum.lean`, theorem `cq_safe_sched'`: quorum safety for the
*step machine's* `collect_quorum` (the shared `hydro_std` stage),
under any pacing, any coupled inputs, any horizon — premise-free. The
statement: every key `k` the machine has emitted by step `T` carries
at least `mn` Ok-votes among the abstract consumed pool of a derived
decision.

```lean
theorem cq_safe_sched' …
    (hk : k ∈ ((collect_quorum (SchedSem …) () h mn mx () demit)
                 .val.1 i).view T) :
    ∃ d : (Values …).BatchDec 1 (K × Except E Unit),
      mn ≤ cqOkCount (cqConsumed (v i) (d i)) k := by
  -- (1) THE REFINEMENT MAPPING, computed: the ∃-witness is the
  --     abstract batch decision this schedule realized (cqVDec is
  --     record plumbing around batchDerive over the machine input h).
  refine ⟨cqVDec (Td := T) … mn mx h, ?_⟩
  -- (2) THE SIMULATION: instantiate the program at the CORNER
  --     (couple-at-T, derive-at-T) and read off `.cpl` — the corner
  --     op-by-op constructed the abstract run AND its coupling; the
  --     wf residue for this knot-free program is discharged by a
  --     two-line `co_wf_simp` (all conjuncts trivial).
  have hcpl := (collect_quorum (CoupleSem … T T _) ()
      (CoStream.inputC h v hc) mn mx (CoDec.batch _)
      (CoDec.emit demit)).val.1.cpl (by co_wf_simp [collect_quorum]; …) i
  -- (0) NAME THE LEGS: the corner's machine leg IS
  --     collect_quorum@SchedSem, its abstract leg IS
  --     collect_quorum@Values at the derived decision — module naming
  --     lemmas cq_co_sr₁ / cq_co_rr₁ (proved once, by co_transfer).
  rw [hsr, hrr] at hcpl
  -- (3) THE ABSTRACT CONTRACT: the ∀d colocated Ensures of
  --     collect_quorum at Values — soundness of emissions.
  have hens := (collect_quorum (Values …) () v mn mx
    (cqVDec …) ()).property rfl
  -- (4) TRANSPORT: membership of the machine's emission moves along
  --     the sub-multiset coupling into the abstract emission, where
  --     the contract counts its votes.
  exact hens.emit_sound i k (Multiset.mem_of_le hcpl
    (Multiset.mem_coe.mpr hk))
```

(Comments added here; the file's own docstring carries the same
story.) This is the whole method in four moves: *derive the decision,
couple, apply the abstract contract, transport the observation* —
with the derivation and the coupling now performed **by the
interpretation itself** (the corner), so no satisfiability premise
survives into the statement. Note what is **absent**: no induction
over executions, no invariant about `collect_quorum`'s internal
state — the induction happened once, per-operator, inside
`CorrSem`/`CoupleSem`, and `collect_quorum`'s protocol reasoning
lives in its own colocated contract on the `Values` side.

## Walkthrough B: `corr_values` — the message-reordering step case

If you were doing a hand simulation proof for an async network, one
case would read: *"suppose the next event delivers a message; show the
relation is preserved for any interleaving of senders."* That case is
`corr_values` in `Transfer.lean` (section "Per-op coupling lemmas") —
the fan-in operator, where cross-sender interleaving *emerges* from
delivery timing (fan-in takes no decision in the machine):

```lean
/-- `values` fan-in stays coupled, at every grade: emergent
interleaving is below the denotational merge. -/
theorem corr_values …
    (hc : ∀ i j, ListLe ord ret ((h i j).view t) (v i j)) :
    ∀ i, ListLe .noOrder ret (mergeNView (h i) t)
      ((Values L mem).values v i) := by
  intro i
  cases ord <;> cases ret
  case totalOrder.exactlyOnce =>
    -- machine: concatenation of whatever each sender delivered by t,
    -- in arrival order. abstract: the multiset sum of the pools.
    -- prefix-per-sender ⇒ sub-multiset of the sum (order forgotten
    -- by the output grade — this is where reordering is absorbed).
    …
  case noOrder.atLeastOnce =>
    -- support membership: anything delivered came from SOME sender's
    -- pool (mem_mergeNView), so it is in the union of supports.
    …
```

The proof is a grade case-split because the *output type* of fan-in
(`NoOrder`) is what licenses reordering: on the ordered-input cases
the coupling weakens a per-sender prefix into a sub-multiset — the
formal counterpart of "the network may interleave senders arbitrarily,
and consumers typed `NoOrder` cannot observe which interleaving
happened". Executable confirmation that both orders really occur:
`TransferChecks.lean` §1 (`mergedA`/`mergedB`: two cursor schedules,
two orders, one multiset — `#guard`ed).

The observation-side counterpart is `corr_snapshot` (same section):
the machine's fold-snapshot trace at its tick steps **equals** a
denotational snapshot at derived cuts — an instance of the read-
agreement theorem `sched_snap_eq` (l.307), where the grades' fold
obligations (`FoldOkP`: commutativity at `NoOrder`, idempotence at
`AtLeastOnce`) pay the difference between folding the raw arrival
order and folding the quotient.

## Walkthrough C: `paxos_safe_sched'` — the headline, dissected

`Paxos/CoupleWf.lean` (last theorem in the file). First, the
statement shape — what is quantified over, in distsys terms:

```lean
theorem paxos_safe_sched' (hnA : mem acc ≤ 2 * f + 1)  -- quorum size
    -- section variables:
    --   cpS/cpV/hcp, ckS/ckV/hck — client-payload + checkpoint
    --     inputs: machine + abstract, coupled (⊑)
    --   sdec : PaxosCoreDec (SchedSem …) — ANY schedule: delivery
    --     cursors, timing, emission linearizations
    --   T : Nat — ANY finite horizon
    --   pacing — ANY per-member tick pacing
    (i j : Fin (mem prop)) (s : Nat) (w w' : Option P)
    (hw  : (s, w)  ∈ (…SchedSem-run….val.2 i).view T)   -- two commits
    (hw' : (s, w') ∈ (…SchedSem-run….val.2 j).view T)   -- at one slot
    : w = w'                                            -- agree.
```

Note what is **not** there: no decision argument, no satisfiability
hypothesis. The theorem holds for **any** per-member tick pacing,
**any** cursor/timing/emission schedule `sdec`, **any** horizon `T`,
given only coupled inputs and the quorum-size bound. This is the
"safety in all executions" statement a distsys reader expects — with
the executions being the step machine's, not the denotation's.

The body is the same four moves as Walkthrough A:

```lean
  -- (3') the abstract contract: paxos_core's colocated headline at
  --      Values — SlotFunctional — applied at the DERIVED decision
  --      record paxosVDecR (the corner's own decisions, read off the
  --      machine run). THIS is where the actual Paxos argument lives
  --      (quorum intersection, ballot provenance):
  --      PCEnsures.slot_functional = one application of
  --      paxos_core_agree (K4, PaxosCoreLemmas.lean), whose
  --      pigeonhole is nodup_inter_of_length.
  have hens := ((paxos_core (Values …) .guarded prop acc f cpV ckV
    (paxosVDecR …)).property rfl).slot_functional rfl hnA
  -- (2') the simulation: the corner run's coupling for each
  --      proposer's replica wire; its wf residue is paxos_co_wf —
  --      the five knots' causality/ascent/coupling triples, proven
  --      once per knot in this file.
  have hcpl_i := paxos_co_cpl … i
  have hcpl_j := paxos_co_cpl … j
  -- (0') name the legs: the corner's machine leg IS
  --      paxos_core@SchedSem, its abstract leg IS paxos_core@Values
  --      at the derived decisions — paxos_co_sr / paxos_co_rr,
  --      assembled from per-module naming lemmas
  --      (generated: `hydro_couple`/`hydro_knot`, FINDINGS D44–D45).
  rw [paxos_co_sr …, paxos_co_rr …] at hcpl_i hcpl_j
  -- (4') transport the two machine commits into the abstract replica
  --      pools and let SlotFunctional finish.
  exact hens i j s w w'
    (Multiset.mem_of_le hcpl_i (Multiset.mem_coe.mpr hw))
    (Multiset.mem_of_le hcpl_j (Multiset.mem_coe.mpr hw'))
```

Where your intuitions land, explicitly:

- **Quorum intersection** — *not here.* It is in
  `paxos_core_agree` (`Paxos/PaxosCoreLemmas.lean`, K4), on the
  `Values` side: commits need `> f` Ok-votes, `mem acc ≤ 2f+1`, and
  `nodup_inter_of_length` is the pigeonhole giving a common acceptor;
  the provenance regress (K4's induction through the `a_log` knot)
  does the ballot-monotonicity argument. This file only *transports*
  that theorem to machine runs.
- **The invariant over executions** — replaced by the per-operator
  coupling plus the naming identities. `paxos_co_sr`/`paxos_co_rr`
  are assembled from per-module lemmas (one `co_knot_sr`/`co_knot_rr`
  call per knot, one `co_transfer` per module — FINDINGS D40 explains
  why per-module is forced: kernel defeq through `k` nested knots is
  exponential in `k`); no Paxos-specific simulation reasoning exists.
- **What remains per knot** — the `wf` triple (`paxos_co_wf`, this
  file): the knot body is causal (a machine op's output below step
  `t` reads only inputs below `t` — discharged by the head-dispatch
  walker over `SchedCausal.lean`'s per-op lemmas), its Values leg is
  a Kleene ascent (one `MonoRel`/`MonoHRel` instantiation), and the
  coupling survives one body step (discharged by re-instantiating the
  `∀ H'`-generic body at a horizon-lowered corner — the payoff of
  instance-generic knot bodies, FINDINGS D38).
- **A historical note auditors should read** (`SCHED_AUDIT.md` F1,
  FINDINGS D37): the *first* machine-safety headline here was
  `∀ d`-quantified over a decision environment with a satisfiability
  premise `hsat : δ T d`. Attempting to *discharge* that premise — to
  prove the theorem non-vacuous — revealed it was **unsatisfiable for
  nested knots**: a bridge condition inside the knot δ forced two
  consecutive Kleene stages equal, so the old headline was vacuously
  true at every horizon ≥ 1 in a zero-sorry, gate-green tree. The δ
  design was repaired, then dissolved entirely by the corner (whose
  coupling is constructed, not postulated) — and the δ-stack retired
  (FINDINGS D42). The lesson stands: **a premise you have never
  inhabited is a hole exactly the size of your theorem**, and
  quantified side conditions on trusted-statement machinery deserve
  inhabitation proofs, not vibes.
- **Non-vacuity of the whole pipeline** — `lake exe v2paxos` runs an
  end-to-end scenario on these very semantics (one proposer, one
  acceptor, `f = 0`): leader elected at tick 1, payload `42`
  committed at slot 0, announcement exactly once, `SlotFunctional`
  observed on the run.

## The refinement mapping, computed: `batchDerive` end to end

The claim "derived decisions" carries the whole correspondence, so it
deserves to be seen with no abbreviation. This section walks one
consumption site — unordered batching, the op behind Rust's
`.batch(&tick, nondet!(…))` — from the machine's raw buffers to the
`∃ d` witness of a safety theorem, and then explains why the
construction cannot be circular.

### The two sides being related

The **machine** side (`Sched.lean`): a tick consumes everything that
arrived since the last tick. Given the raw view function of a wire and
the steps at which the location ticked, the batches are literal list
segments:

```lean
def batchesFrom {α : Type} (src : Nat → List α) :
    List Nat → Nat → List (List α)
  | [], _ => []
  | s :: rest, consumed =>
    (src s).drop consumed :: batchesFrom src rest (src s).length
```

The **denotational** side (`Trace.lean`, `batchCuts`): a batch run is
*decision data* `d : List (Multiset α)` — a claimed sequence of batch
contents — validated against the pool by a legality guard, blocking on
the first illegal claim:

```lean
def batchCuts {α : Type _} [DecidableEq α] (pool : Multiset α)
    (consumed : Multiset α) : (d : List (Multiset α)) →
      Trace (Multiset α)
  | [] => []
  | b :: ds =>
    if consumed + b ≤ pool then b :: batchCuts pool (consumed + b) ds
    else []
```

Note the asymmetry a reviewer should register: `batchCuts` knows
nothing about steps, schedules, or the machine — it is the abstract
consumption semantics, constrained only by "consumed so far plus the
increment stays within the pool". Its definition would be exactly this
even if `Sched.lean` did not exist.

### The derivation function

`batchDerive` (`Transfer.lean`, "Named derived decisions" section) is
the refinement mapping for this site:
it *reads the abstract decision off the machine run* — which tick
consumed which content, as multisets:

```lean
def batchDerive {n : Nat} {α : Type} [DecidableEq α]
    (p : Fin n → Nat → Bool)
    (T : Nat) (s : Fin n → StepHist α) : BatchCuts n α :=
  fun i => (batchesFrom ((s i).view) (tickSteps (p i) T) 0).map
    (fun b => Multiset.ofList b)
```

Inputs: the per-member tick pacing `p`, the horizon `T`, the machine
wire `s`.
Output: a perfectly ordinary `BatchCuts n α` decision — the same type
a program's `∀ d` contract quantifies over. Its kin cover the other
consumption sites: `batchOrdDerive` (ordered batches: segment
*lengths*), `snapDerive`/`snapCut` (snapshot reads: `cutsLen` derives
length increments of the read chain at ordered grades, `cutsMS`
content increments at unordered grades — `Transfer.lean` l.192/197),
and `ordSelDerive` (order selection: the machine's arrival order,
verbatim). All are horizon-monotone (`batchDerive_mono`,
`snapDerive_mono`, … — more of the machine's run only extends the
derived decision), which is what lets one decision serve nested cycle
stages consistently.

### The realization theorem (equality, and why it isn't circular)

`batchCuts_real` (`Transfer.lean`) is the theorem that the derived
claim *passes the abstract legality check and is reproduced verbatim*:

```lean
theorem batchCuts_real … :
    ∀ (ss : List Nat) (wprev : List α),
      List.IsChain (· <+: ·) (wprev :: ss.map src) →
      (∀ s ∈ ss, Multiset.ofList (src s) ≤ pool) →
      batchCuts pool (Multiset.ofList wprev)
        ((batchesFrom src ss wprev.length).map (fun b => Multiset.ofList b))
        = (batchesFrom src ss wprev.length).map (fun b => Multiset.ofList b)
```

Read the hypotheses: the machine's views at the tick steps chain by
prefix (that is `StepHist.mono` — buffers only grow), and each view is
dominated by the pool (that is the coupling `cpl` from upstream). The
conclusion: `batchCuts` at the derived decision does **not** block —
every increment the machine consumed was legal — and emits exactly the
machine's segments. `corr_batch` (same file) is its per-member wrapper;
`sliceCuts_real`/`corr_batch_ordered` are the ordered twins;
`sched_snap_eq'` with witness `snapCut` is the snapshot analogue (there
the grade obligations `FoldOkP` — commutativity at `NoOrder`,
idempotence at `AtLeastOnce` — pay the difference between folding the
raw arrival order and folding the quotient; snapshots of unbounded
folds are subtle enough to get [their own section
below](#unbounded-commutative-folds-snapshots-of-a-moving-aggregate)).

Why this cannot be vacuous or rigged, in three observations:

1. **The decision type constrains the derivation.** `batchDerive`
   returns a value of the *pre-existing* decision type, and the
   abstract side validates it with a legality guard written for all
   decisions. If the machine consumed something outside the coupled
   pool, `batchCuts` would block and the equality would be false —
   `batchCuts_real`'s proof has to *earn* every `if_pos`.
2. **Equality is at the observation, not by construction.** The
   theorem equates two independently-defined objects: list segments a
   deterministic machine produced, and the output of the abstract
   consumption function on a claim. Neither definition references the
   other.
3. **The abstract side came first.** `batchCuts`/`snapshotCuts`/
   `prefixCuts` (`Trace.lean`) are the semantics the program contracts
   (`Ensures` records) are proven against; the machine and the
   derivation were built to meet them, not vice versa.

### Where it enters the headline theorems

In `CoupleSem.batch` (`Couple.lean`), the derivation is no longer a
side condition — it is *how the op computes its abstract leg*:

```lean
rr := (Values L mem).batch s.rr (batchDerive (pacing ℓ) Td s.sr)
```

and the op's coupling proof is exactly two moves: `corr_batch` gives
*equality* at the derived decision at the coupling horizon, then
horizon-monotonicity (`batchDerive_mono`, via `batchCuts_mono_dec`)
relaxes coupling-at-`Tc` against derivation-at-`Td`. One such
construction per consumption site is the whole story — there is no
accumulated side condition left to satisfy. (In the retired square
design, the same derived decision appeared as a δ *atom*
`batchDerive (pacing ℓ) T s.sr i <+: cutd d i` constraining a
quantified decision environment; see Walkthrough C's historical note
for why that shape died.) And in Walkthrough A you have already seen
the mapping used nakedly: move (1) of `cq_safe_sched'` supplies
`cqVDec … h` — record plumbing around exactly this `batchDerive` —
as the `∃ d` witness.

The connection to the adequacy caveat is now sharp: derivation gives
*machine run → legal decision* for every schedule; the false converse
would be *decision → realizing schedule*. Derived decisions land in a
strict subset of the decision space (e.g. no schedule realizes an
emission order other than the one the program computed), and safety
only ever needs the direction that holds.

**A remark on adaptive adversaries.** Every schedule component is a
plain function of time fixed for the run (`TransportDec p c = Fin p →
Fin c → Nat → Nat`, `EmitDec n β = Fin n → List (List β)`, pacing
`(ℓ : L) → Fin (mem ℓ) → Nat → Bool`) — the adversary looks oblivious *by type*. This
loses nothing for safety: the machine is deterministic given the
schedule tuple, so any adaptive strategy, played against the run it
itself induces, realizes some fixed tuple — and the theorems
quantify over **all** tuples. (The standard argument; it would fail
only for properties about strategies rather than runs, which safety
is not.)

## Unbounded commutative folds: snapshots of a moving aggregate

`.fold(q!(init), q!(g))` over an unordered stream, observed by
`.snapshot(&tick, nondet!(…))`, is the subtlest correspondence site in
the signature, and it is where a reviewer's intuition most needs
precise anchors. The trap is to think commutativity makes the problem
go away. It does not:

- **Commutativity fixes the value over a *fixed* pool, not the
  sequence of intermediate values.** An unbounded fold is never
  "done"; each snapshot exposes a partial aggregate, and *which*
  partial aggregates exist depends on which elements had arrived by
  each tick. Fold `+` over arrivals `{10, 31, 21}`: one interleaving
  exposes `0, 10, 41, 62`; another exposes `0, 31, 62`. Same final
  value, different observation traces — and Paxos-grade logic reads
  the intermediates (a leader is elected against the max ballot *seen
  so far*, not the max ballot ever).
- **A snapshot both reveals and commits.** Having observed the
  aggregate over some sub-pool `P₁`, every later snapshot must be over
  some `P₂ ≥ P₁` folded consistently — arrivals cannot be unseen. A
  model that let each snapshot independently pick "any sub-pool, any
  order" would admit observation traces no execution produces (e.g.
  an aggregate that goes *down*), and a safety proof over that model
  could be vacuously scoped or, worse, silently weaker than intended.

Both points are first-class in the model, and this section shows
where.

### The Values model: a decision is a *chain*, not a set of draws

The snapshot decision vocabulary is graded by the source order
(`CutDec`, `Grades.lean`):

```lean
def CutDec (α : Type _) : StrOrd → Type _
  | .totalOrder => List Nat          -- per-tick prefix cut counts
  | .noOrder => List (Multiset α)    -- per-tick arrival increments
```

Note the type: a **sequence of increments**, not a set of independent
sub-pool choices. The semantics threads them through a running
accumulator with a legality guard (`Trace.lean`, count-legal grade):

```lean
def snapshotCuts {α : Type _} [DecidableEq α] (pool : Multiset α)
    (acc : Multiset α) : (d : List (Multiset α)) →
      Trace (Multiset α)
  | [] => []
  | b :: ds =>
    if acc + b ≤ pool then (acc + b) :: snapshotCuts pool (acc + b) ds
    else []
```

This is the user-visible subtlety, in four tokens: `acc + b`. Each
realized view is *cumulative* (`acc + b :: …` — the trace exposes
accumulated sub-pools, never arbitrary ones), and each increment's
legality is judged against what earlier increments already consumed —
**an earlier cut decision restricts every later one**, by construction
of the recursion, and an illegal claim blocks the trace right there
(legality is realizability, the same discipline as `batchCuts`). The
decision space at a snapshot site is therefore exactly the set of
ascending chains `0 ≤ v₁ ≤ v₂ ≤ … ≤ pool` in the sub-multiset
lattice, encoded by increments. "Multiple paths through the lattice"
is the retained nondeterminism; "you cannot unsee an arrival" is the
retained constraint.

The fold itself is applied to each realized view by the graded
quotient fold (`Grades.lean`):

```lean
def PoolFold …
  | .noOrder, .exactlyOnce => fun g init comm m =>
      @Multiset.foldl α σ g ⟨fun s x y => comm s x y⟩ init m
```

with `snapViews`/`snapTrace` (`Values.lean`) composing the two:
`snapTrace ord ret g init ok pool d = (snapViews … pool d).map
(PoolFold … g init ok)`. Two orthogonal quotients are doing work here,
and it pays to separate them:

1. **Within a view**, arrival order is dead: the carrier is a
   `Multiset`, and `Multiset.foldl` *does not exist* without the
   commutativity witness — that is `FoldOkP` (`Grades.lean`, marked
   reducible), the Lean rendering of Rust's fold-properties API. A
   fold that mishandles reordering is ill-typed, not incorrect.
2. **Between views**, the path through the lattice is alive: it is
   precisely the decision data. Commutativity collapses point 1 only;
   the observation-trace nondeterminism of point 2 is what `∀ d`
   contracts quantify over and what the correspondence must map.

For *inflationary* folds (Rust's `Monotonic` marker —
`fold_monotone`, carrying `∀ s x, vo.le s (g s x)`),
`snapTrace_ascending` (`Values.lean`) proves reads ascend along any
legal chain: because views chain and the fold only moves up along a
chain, no decision whatsoever can exhibit a decreasing aggregate. The
`Values` carrier bakes this in as a subtype — a `.monotonic` fold
singleton *is* `{f : CutDec α ord → Trace σ // ∀ d, Ascending vo
(f d)}` (`Values.lean` l.143): ascent at **every** decision, hence at
every schedule after transfer, is carried in the type. The election's
`p_received_max_ballot` rides exactly this (below); the acceptor's
`a_max_ballot` (`AcceptorP1.lean`) is the same discipline at the
sibling across-ticks op (`fold_batches_across_ticks_monotone`, whose
output is a `MonoTrace` — its Ensures record pointedly has no
"ascends" clause because the type already says it).

### The machine: consistency comes free from one history

The step machine does none of the above bookkeeping (`Sched.lean`):

```lean
structure SchedFold (α σ : Type) where
  src : StepHist α
  read : List α → σ

-- fold:      fun i => ⟨s i, fun l => l.foldl g init⟩
-- snapshot:  fun i t => (tickSteps (pacing ℓ i) t).map
--              (fun st => (s i).read ((s i).src.view st))
```

A machine fold is a raw **arrival-order** `List.foldl` over the
physical buffer — the doc comment in the file says it plainly: *no
commutativity is needed to run; the grade obligations are consumed by
the transfer proof*. A machine snapshot reads that fold at each tick
step of its own member skeleton. Successive machine snapshots are
consistent not because anything checks a guard, but because they all
read **one prefix-monotone history** (`StepHist.mono`): the buffer at
a later tick extends the buffer at an earlier one. The two sides thus
locate the same invariant differently — Values *imposes* chain
consistency on claimed decisions; the machine *inherits* it from
physical buffer growth. The correspondence proof is exactly the
statement that the second is an instance of the first.

### The correspondence: derived increments, and who pays for reordering

The derivation (`snapCut`, `Transfer.lean`; lifted per-member as
`snapDerive`, same file's "Named derived decisions" section) reads
the decision off the run by
differencing the view chain at tick steps:

```lean
def cutsMS {α : Type} [DecidableEq α] (acc : Multiset α) :
    List (List α) → List (Multiset α)
  | [] => []
  | w :: ws => (↑w - acc) :: cutsMS (↑w : Multiset α) ws
```

— tick `k`'s increment is *what arrived between tick `k−1` and tick
`k`*, as a multiset. The realization lemma `snapshotCuts_views`
(`Transfer.lean`) then proves the two facts a reviewer should demand:

- **the derived chain is legal**: each `if acc + b ≤ pool` guard in
  `snapshotCuts` passes, with `StepHist.mono` (prefix growth) and the
  upstream coupling `ListLe … (src.view T) pool` as the only inputs —
  machine history monotonicity *is* cut-chain legality;
- **it reproduces the views verbatim**: `snapshotCuts pool 0 (cutsMS
  0 ws) = ws.map Multiset.ofList` — no blocking, no slack.

That leaves one gap: Values folds the accumulated *multiset*, the
machine folded the accumulated *list in arrival order*. The private
lemma `map_msfold_eq` closes it, and this is the exact point where
the program's commutativity obligation is spent — `FoldOkP` pays the
difference between the raw fold and the quotient fold, per view.
Stacking the three gives read agreement (`sched_snap_eq`, with its
witness named in closed form as `snapCut` in `sched_snap_eq'`,
`Transfer.lean`; repackaged as `snapshot_tight` in
`TransferTheory.lean`):

```lean
theorem sched_snap_eq' …
    (hchain : List.IsChain (· <+: ·) (([] : List α) :: ws))
    (hcpl : ∀ w ∈ ws, ListLe ord ret w pool) :
    ws.map (fun w => w.foldl g init)
      = snapTrace ord ret g init ok pool (snapCut ord ret ws)
```

**Equality** of the machine's observation trace with the denotational
read at the derived chain — the same "not `⊑` but `=`" tightness as
`batchCuts_real`, and non-circular for the same three reasons
(§ above): the derived chain must *earn* every legality guard,
against an abstract semantics (`snapshotCuts`) that never mentions
steps or schedules and predates the machine.

Upstream of these lemmas, the coupling that supplies `hcpl` is
`FoldSingRel` (`Transfer.lean`): a machine `SchedFold` is coupled to a
set of denotational read functions by exhibiting, at every step, the
fold algebra and a dominating pool. `corr_snapshot` is the
per-operator step lemma consuming it, and in the corner instance the
derived chain is *how the op reads* (`Couple.lean`,
`CoupleSem.snapshot`): the abstract leg is the denotational snapshot
at `snapDerive ord ret (pacing ℓ) Td s.sr` — derived, not
quantified —

with `snapDerive_mono` giving horizon-monotonicity (a longer run only
*extends* the derived chain — this is what lets one decision serve
nested cycle stages consistently; there is no separate stabilization
argument at snapshot sites), and `snapTrace_mono_dec` relaxing the
equality-at-derived between the coupling horizon `Tc` and the
derivation horizon `Td`, exactly as at batch sites.

### Anchors, executable and classic

For a distsys reader: within-view quotienting is the CRDT/monotone-
aggregation discipline (order-insensitive merge makes the *state*
well-defined), and between-view chaining is snapshot consistency for
a growing aggregate — the observed sub-pools form a chain, so there
is always a single arrival order that passes through *all* of them
(enumerate `v₁`, then `v₂ − v₁`, …); you never observe two snapshots
no one execution could have produced together. At the
count-legal grade (`ExactlyOnce`) the cut machinery constrains views
to genuine sub-multiset chains of the pool, so every observed
aggregate is one *some* interleaving of the coupled pool produces.
One honest relaxation: at `AtLeastOnce` the guard is membership-only
(`snapshotMemCuts` — increments may duplicate freely but only quote
the pool), so a claimed chain may carry multiplicities no run
produced; the fold obligation at that grade is commutativity **and
idempotence** (`FoldOkP .noOrder .atLeastOnce`), making the fold
value a function of the support alone — phantom multiplicity is
unobservable through the read, and it points in the safe
(over-approximating) direction regardless.

Executable witnesses (`TransferChecks.lean` §4 and §7) make the two
halves of the subtlety concrete and build-time-checked: the sum-fold
run `#guard snapAll 0 4 = [0, 10, 41, 62, 62]` shows intermediates
exposed tick by tick and a stutter once content is exhausted; §7's
paired guards show pacing *changes* which prefixes a snapshot exposes
(`snapAll 0 5 ≠ snapHalf 0 5` — real, captured nondeterminism) while
within-step arrival order is quotiented away (`[10,20]` and `[20,10]`
fold and read identically). The Paxos-scale instance is
`p_received_max_ballot` (`LeaderElection.lean`):

```lean
let p_received_max_ballot := H.snapshot
  (H.fold_monotone Ballot.obtVO Ballot.maxFold none …)
  dec.receivedMax
```

— the Rust `.max().into_singleton().snapshot(&proposer_tick,
nondet_leader)`: an unbounded commutative-and-inflationary max fold
whose tick-by-tick snapshots decide leadership. Everything the
election proof knows about that wire (ascent across ticks, agreement
with the machine's reads at every schedule) is the machinery of this
section instantiated once, through the generic transfer — no
Paxos-specific fold lemma exists.

## What is deliberately NOT proven

- **Adequacy / bisimulation.** There is no `∀ decisions ∃ schedule`
  theorem, and there cannot be (`TransferTheory.lean` header):
  `emitBatchesUnordered` publishes a list whose order the *type*
  forgets but the machine determines, so a downstream
  `assume_ordering` realizes one order per schedule while the
  abstract `selectOrder` space licenses every permutation. The
  quotient over-approximates on purpose: safety transfer needs
  machine ⊆ denotation only, and the surplus makes program
  obligations robust to weaker transports. If your intuition asks
  "is this a forward simulation only?" — yes, and the document you
  are reading is the receipt that this is a design decision, not an
  omission.
- **Liveness.** Nothing here says a commit *happens*. The end-of-time
  kit (`TransferTheory.lean`: `StabilizesAt`, `deliver_id_attains`,
  `batch_flat_attains`, `fix_diag_attains`) upgrades per-wire
  stabilization to *exact* attainment of the abstract pool — per-wire
  and compositional, no global quiescence predicate — but
  "eventually stabilizes under fairness" is future work
  (quiescent-completeness and the fuel-less/lfp `Values` fix are the
  queued upgrades; see the README's staged-work section).
- **Real Rust execution.** The machine is a semantics of the Hydro
  combinators, not of compiled Rust; the 1:1 program-text mirroring
  (one Rust fn = one Lean def) is the bridge discipline.

## Trust base

Every headline is audited by `#print axioms` in `AxCheck.lean` —
including `paxos_safe_sched'`, `cq_safe_sched'`, and the naming/
coupling stack they consume (`paxos_co_sr`, `paxos_co_rr`,
`paxos_co_wf`, `paxos_co_cpl`). Expected foundation: `propext`,
`Classical.choice`, `Quot.sound` (Lean's standard three; no `sorryAx`,
no custom axioms anywhere in `HydroV2/`). The quotient carriers that
make illegal observations ill-typed (`Grades.lean`) rest on core
`Quotient`, i.e. on `Quot.sound` — the same soundness story as
Mathlib's.

Axioms are only half of a trust base, though. The other half is the
**definitions appearing in the theorem statements** — above all
`SchedSem`, because `paxos_safe_sched'` says "safe in every execution
*of this machine*". A machine that secretly could not reorder
messages, silently synchronized ticks, or dropped buffers would make
the theorem true and worthless. That is not something `#print axioms`
can catch; it is a human review obligation, and the next section
exists to make it tractable.

## Auditing the trust boundary: the Sched machine

> A systematic red-team audit of everything below (model vs. the Rust
> runtime, plus Lean-internal premise dischargedness) is recorded in
> [`SCHED_AUDIT.md`](SCHED_AUDIT.md); the checklist at the end of this
> section is its prior art.

`Sched.lean` is deliberately small — 395 lines, and the `SchedSem`
instance itself is one page. This section walks every definition a
reviewer must judge, states what would go wrong if it were subtly
different, and ends with a checklist pairing each judgment with the
executable witness that machine-checks a positive instance
(`TransferChecks.lean`). Be clear about what the witnesses do and do
not give you: a `#guard` proves the model *admits* an adversarial
behavior; no `#guard` can prove the model admits *enough*. Maximality
of the adversary is exactly the thing this review is for.

### Wires: histories that can only grow

```lean
structure StepHist (α : Type) where
  view : Nat → List α
  mono : ∀ t, view t <+: view (t + 1)
```

One global step clock; a wire's state at step `t` is the plain list
`view t`; prefix-monotonicity is bundled, so history immutability is
physical (buffers only grow; nothing can be retracted or reordered
after arrival). There are no quotients anywhere in the machine —
`Multiset`/`StutterSeq`/`RetryPool` exist only on the `Values` side.
The derived quantity `inc t` (`view (t+1)` minus the `view t` prefix)
is "what arrived during step `t`".

*What to check:* that `mono` is prefix (`<+:`), not something weaker —
sub-multiset here would let the machine shuffle its own past.

### Delivery: cursors, and where their freedom lives

```lean
def StepHist.deliver {α : Type} (h : StepHist α) (c : Nat → Nat) :
    StepHist α where
  view
    | 0 => []
    | t + 1 => (h.view t).take (cumMax c (t + 1))
```

`broadcast d s := fun i j => (s j).deliver (d i j)` — each
(receiver `i`, sender `j`) pair gets its own cursor `d i j : Nat →
Nat`, an arbitrary function of the step run through `cumMax` (a
delivered message stays delivered). All the adversary's routing
freedom is here:

- **arbitrary per-pair delay**: the cursor advances whenever the
  schedule pleases — bursts, stalls, one-at-a-time;
- **unbounded latency and silence**: a cursor that returns `0`
  forever is a crashed link (fail-stop); there is no fairness
  assumption anywhere in the safety theorems;
- **no global order**: cursors of different pairs are unrelated, so
  cross-sender arrival order at a receiver is entirely
  schedule-controlled;
- **per-pair FIFO**: delivery is `take` of the sent list — a prefix.
  A cursor can *withhold* messages 2,3,… but can never deliver 3
  while dropping 2, and can never corrupt or duplicate in flight.
  This is the TCP-transport fidelity claim (`TCP.fail_stop` in the
  Rust source), not a proof convenience — judge it against the
  runtime you care about. `AtLeastOnce` duplicates exist in the
  model, but they are *born at sampling sites* (`sample_every` of an
  unchanged latest re-emits it) and ride the wire as content.
- **the `+1` floor**: at step `t` the receiver sees a prefix of the
  sender's wire *as of `t − 1`* — no same-instant network hop. The
  floor bounds delivery *below* only; everything above is free.
  (FINDINGS D31: the network floor is not even needed for the safety
  proofs — every signature cycle passes through `fix`, whose own
  floor suffices — it is kept so a step has a causal, Lamport-style
  reading. Removing it would change no safety theorem.)

*What to check:* the cursor's type. `Nat → Nat` — a function of the
**step only**. It cannot inspect payloads, buffer contents, or
downstream state; content-adaptive routing is expressible only through
the ∀-over-all-cursors quantification (see the adaptive-adversary
remark above), never baked into a definition.

### Fan-in: interleaving is emergent, not chosen

```lean
def mergeNView {m : Nat} {α : Type} (k : Fin m → StepHist α) :
    Nat → List α
  | 0 => (List.finRange m).flatMap (fun j => (k j).view 0)
  | t + 1 => mergeNView k t
      ++ (List.finRange m).flatMap (fun j => (k j).inc t)
```

`values`/`union` take **no decision**: at each step, append every
sender's increment, in canonical sender order *within* the step.
Cross-sender interleaving therefore comes from delivery timing alone —
the adversary splits arrivals across steps to realize any interleaving
(witness §1: two cursor schedules, two merged orders, same multiset).
The within-step canonical order is WLOG by elasticity: inserting idle
steps refines any simultaneous batch into singletons (D31). The
significance for the proof's honesty: the machine's semantics has no
knob with which a proof could pick a *convenient* interleaving; every
theorem must survive whatever the cursors did.

### Ticks and consumption

```lean
def tickSteps (p : Nat → Bool) (t : Nat) : List Nat :=
  (List.range (t + 1)).filter p
```

`pacing : (ℓ : L) → Fin (mem ℓ) → Nat → Bool` — which steps each
**member** of each location ticks at — is an ambient parameter of the
whole instance, universally quantified in every theorem. Tick presence
is independent across cluster members: one member may tick (with
content) at a global step where a sibling member has no tick entry at
all. A tick consumes **everything available and not yet consumed**
(`batchesFrom` above — hydro's actual `batch` semantics), and a
snapshot reads the fold accumulator at each tick step. Stutter ticks
(nothing arrived) are real and observable — empty batches, repeated
snapshot reads (witness §4) — and ticking is unsynchronized across
locations and members.

*What to check.* Check that no tick timing depends on content
(`Nat → Bool` in the step argument only), and that the skeleton is
genuinely per member: every consumption site uses
`tickSteps (pacing ℓ i)` at its own member index `i` — the five sites
are `snapshot`, `batch`, `batch_ordered`, and the two tick-*counting*
ops `timeout_snapshot` / `source_interval_batch` (which take
`(tickSteps …).length`, making tick presence semantically visible).
The executable reproducer is `TransferChecks.lean` §9: two members of
one cluster on *different* skeletons — at global step 1, member 0
ticks with content (`[[], [10]]`) while member 1 has no tick entry at
that step at all (`[[]]`), and the same pulse list consumed per member
skeleton yields different tick counts. Member interaction remains
cursor-mediated, so per-member skew composes with delivery skew rather
than replacing it.

### Re-monotonization: `famFreeze` cannot invent states

Tick-domain results re-enter the stream world at `allTicks`/
`sample_every`. Raw per-tick views need not be prefix-monotone for
*arbitrary* bodies (real flows grow by prefix; the fixpoint iterates
of a pathological body need not), so the machine freezes them:
advance only while **every** member's next view extends its current
one. The theorem a reviewer should demand is proved right below the
definition:

```lean
theorem famFreeze_eq_raw … :
    ∀ t, ∃ k, k ≤ t ∧ famFreeze h t = h k
```

— every frozen family view **is** one of the raw family views (frozen
family-atomically: no splicing member `0`'s view at time 5 with member
`1`'s at time 3). `famFreeze` can lag reality; it can never fabricate
or mix it. On real (guarded) flows it is the identity.

### Emission: where order is born

```lean
def emitLin {β : Type} [DecidableEq β] :
    Trace (Multiset β) → List (List β) → Trace (List β)
  | [], _ => []
  | _ :: _, [] => []
  | m :: vs, l :: ls =>
    if (↑l : Multiset β) = m then l :: emitLin vs ls else []
```

`emitMultisetBatches` is the one site where a program puts *computed
unordered data* onto a wire, so it is the one site where the machine
takes an ordering decision (`EmitDec n β = Fin n → List (List β)`, the
per-tick linearization — operationally: hashmap iteration order). The
claim is **validated**: a linearization that is not a permutation of
the emitted multiset blocks the trace. Legality is realizability — the
adversary chooses among real orders, and only real orders.

### The fixpoint: why the knot is not a synchrony cheat

```lean
fix_stream _d body :=
  famHist (fun t i =>
    ((iterate (fun x j => ((body x) j).shift)
      (fun _ => StepHist.bot) (t + 1)) i).view t)
```

Three ingredients. `shift` delays the feedback wire by one step (the
event-loop pass: content cannot re-enter the loop within the step it
was produced — one full dataflow pass per step is still legal).
`iterate … ⊥ (t+1)` is plain Kleene iteration from the empty wire.
The **diagonal** — at step `t`, read the `(t+1)`-deep iterate at `t` —
is the fuel-less knot: because each iteration's `shift` pushes new
content one step later, everything visible at step `t` is already
determined at depth `t+1`, *for arbitrary bodies* (deeper iterates
agree below the diagonal). So the machine neither assumes guardedness
per program nor grants the loop synchronous (zero-step) round trips —
the floor excludes exactly the Zeno executions under which the
diagonal would be ill-defined (D31; the concrete failure it prevents:
consumption sites inside nested knots deriving conflicting decisions
across Kleene stages). Witness §6 shows the floor and the freedom
together: a younger message's cycle-offspring overtakes a stalled
elder through the knot, the offspring lands exactly one step after its
cause, and the raced order is still covered denotationally.

*What to check:* that `shift`, not the body, pays the step (the
feedback edge is delayed once per pass, independent of program text);
and that the diagonal reads iterate `t+1` at step `t` — reading a
*fixed* depth would be a fuel assumption, reading depth-at-`t` of the
*unshifted* body would allow zero-time loops.

### The audit checklist

| Judge by reading | Definition | Existence witness (`TransferChecks.lean`) |
|---|---|---|
| Histories are immutable (prefix, not sub-multiset) | `StepHist.mono` | — (type-level) |
| Cursors express unbounded delay & silence | `deliver`/`cumMax`: any `Nat → Nat` | §2 stalled cursor: `(mergedSilent 0).view 8 = [10, 11]` |
| Cross-sender reordering is schedule-reachable | fan-in takes no decision | §1 `mergedA ≠ mergedB`, equal multisets |
| Per-pair FIFO, no in-flight corruption/duplication | `deliver` is `take` of the sent list | §3 delivered views are prefixes of sent |
| Nothing schedules on message content | cursor/pacing **types**: functions of step only | — (type-level) |
| Duplicates born at sampling, not transport | `sample_every`; `deliver` can only `take` | §5 consecutive stutter, destuttered by the coupling |
| Ticks are unsynchronized across locations **and members**; stutter ticks observable | `pacing` per member; `batchesFrom` | §4 finer skeleton ⇒ more (empty) batches, same content; §9 per-member skew |
| Consume-all-at-tick (no selective consumption) | `batchesFrom` drops exactly the consumed prefix | §4 flattened batches equal across pacings |
| The knot pays a step but only a step | `shift` + Kleene diagonal | §6 offspring lands cause+1; overtaking still free |
| Re-monotonization cannot fabricate states | `famFreeze_eq_raw` | — (theorem) |
| Emission linearizations are validated | `emitLin` blocks on mismatch | — (definition) |
| Stabilization hypotheses are inhabited | end-of-time kit | §8 computable attainment |
| **Model is adversarial *enough*** | — | **not machine-checkable; this checklist is the review** |

One known modeling commitment to weigh explicitly, discussed above:
per-pair FIFO transport (TCP fidelity — no selective drop, no
in-flight duplication). It errs in the direction visible to review,
and it is not hidden behind a quotient: the machine is 395 lines of
lists.
