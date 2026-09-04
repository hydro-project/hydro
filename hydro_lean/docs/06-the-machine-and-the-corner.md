# 06 — The machine and the corner

Chapters 02–05 never mentioned time. `SchedSem` (`Hydro/Sched.lean:331`) is the
concurrent operational semantics — the same program text instantiated at a machine with
a step clock, asynchronous delivery and per-member ticking — and the **coupling corner**
(`Hydro/Couple.lean`) is how a `Values` contract becomes a theorem about every run of
that machine. The full story, the trust base and the audit are `../CORRESPONDENCE.md`
and `../SCHED_AUDIT.md`; this chapter is the shape.

## The machine

```lean
-- Hydro/Sched.lean:331–345
def SchedSem (L : Type) (mem : L → Nat)
    (pacing : (ℓ : L) → Fin (mem ℓ) → Nat → Bool) :
    HydroSem L mem where
  Stream ℓ α _ _ _ := Fin (mem ℓ) → StepHist α
  …
  Ticked ℓ σ := Fin (mem ℓ) → Nat → Trace σ
  -- one tick's bounded stream: the concrete list the runtime holds, at
  -- EVERY grade (the quotient is the denotation's; here order/retries
  -- are unobservable only through the in-tick operators' typing)
  BoundedStream α _ _ _ := List α
```

- A stream is a **history**: `StepHist α` (`Sched.lean:88`) is `view : Nat → List α`
  with `mono : ∀ t, view t <+: view (t + 1)` — the buffer as of step `t`, growing by
  prefix. No quotient anywhere: at every grade the machine holds a plain list, as the
  runtime holds a `Vec` (`SCHED_AUDIT.md` S4 §14).
- **Ticks are per member**: `pacing ℓ i t` says whether member `i` of `ℓ` ticks at step
  `t`. Ticks are unsynchronized across locations *and* members; stutter ticks exist
  (D35; `SCHED_AUDIT.md` §1).
- **Delivery is a cursor**: `broadcast_closed d s := fun i j => (s j).deliver (d i j)`
  (`:358`) — `TransportDec p c := Fin p → Fin c → Nat → Nat`, one monotone cursor per
  (receiver, sender) pair, reading the sender's `view (t-1)`. Any cursor is legal:
  unbounded delay, silence forever, a crash (stall at `k`). This is exactly
  `TCP.fail_stop` (`SCHED_AUDIT.md` F6).
- **Decisions swap sides**: the content decisions that were real at `Values` (`BatchDec
  _ _ := Unit`) are `Unit` here — the machine *derives* them from its run; the transport
  cursors that were `Unit` at `Values` are real. `batch s _d := fun i t => batchesFrom
  ((s i).view) (tickSteps (pacing ℓ i) t) 0` (`:372`): a tick consumes everything that
  has arrived — "partial batches" arise only from arrival timing.
- **The knot is the Kleene diagonal** (`fix_stream`, `:388`): step `t` of the cycle is
  the `t+1`-fold iterate of the body, shifted one step — cycle unfolding *is* step
  progression, no fuel, with a one-step floor that excludes exactly the Zeno executions.

`TransferChecks.lean` is the `#guard` suite showing the machine *admits* each
adversarial behavior (interleaving, latency, stutter ticks, the fixpoint race, per-member
skew).

## The headline, and what it quantifies over

```lean
-- Hydro/Paxos/CoupleWf.lean:85–92
theorem paxos_safe_sched' (hnA : mem acc ≤ 2 * f + 1)
    (i j : Fin (mem prop)) (s : Nat) (w w' : Option P)
    (hw : (s, w) ∈ ((paxos_core (SchedSem L mem pacing) .guarded prop
      acc f cpS ckS sdec ssched).2 i).view T)
    (hw' : (s, w') ∈ ((paxos_core (SchedSem L mem pacing) .guarded prop
      acc f cpS ckS sdec ssched).2 j).view T) :
    w = w' := by
```

Section variables: `pacing` — **any** per-member tick skeleton; `sdec`/`ssched` — **any**
timing and delivery schedule; `T` — **any** horizon; `cpS`/`ckS` the machine inputs with
abstract pools `cpV`/`ckV` they are coupled to (`hcp`/`hck`: the machine's view at every
horizon is a prefix of the pool — the one premise, discussed as `SCHED_AUDIT.md` F2).
Conclusion: two commits the *step machine's* `paxos_core` has emitted at one slot agree.

Absent: a decision argument, a satisfiability hypothesis. The theorem's predecessor had
one (`hsat : δ T d`) and it turned out to be **unsatisfiable for nested knots** — the
headline was vacuously true for a year in a zero-sorry tree (D37;
`../CORRESPONDENCE.md` §cautionary). That episode is why `DOCTRINE.md` R7 exists.

## The corner

The proof is four names and one contract application (`CoupleWf.lean:93–108`):

```lean
  have hens := (paxos_core.ensures .guarded prop acc f cpV ckV
    (paxosVDecG pacing prop acc T f cpS ckS sdec ssched)
    PaxosCoreSched.triv).slot_functional rfl hnA
  have hcpl_i := paxos_co_cpl pacing prop acc T f cpS cpV hcp ckS ckV hck sdec ssched i
  have hcpl_j := paxos_co_cpl … j
  rw [paxos_co_sr …, paxos_co_rr …] at hcpl_i hcpl_j
  exact hens i j s w w'
    (Multiset.mem_of_le hcpl_i (Multiset.mem_coe.mpr hw))
    (Multiset.mem_of_le hcpl_j (Multiset.mem_coe.mpr hw'))
```

1. **The abstract contract** (`hens`): `paxos_core.ensures` — the `Values` theorem of
   chapter 05 — applied at the **derived** decision record `paxosVDecG …`: the batch
   cuts, snapshot cuts and fuels that *this schedule realized*, computed from the
   machine run (`batchDerive`, `snapDerive`; `Transfer.lean`). This is the refinement
   mapping, as a function.
2. **The coupling** (`paxos_co_cpl`): instantiate `paxos_core` at `CoupleSem` — the
   instance whose carriers pack a machine leg `sr`, a `Values` leg `rr`, a residual `wf`,
   and `cpl : wf → ListLe (sr.view Tc) rr` (`Couple.lean`). Every operator derives its own
   `Values` decision from its machine leg and carries its realization proof, so running
   the program at the corner **is** running the simulation argument over its syntax,
   operator by operator. `paxos_co_cpl = .cpl paxos_co_wf`, where `paxos_co_wf` is the
   five knots' well-formedness triples (body causal, Kleene ascent, one coupling step),
   assembled from the generated `<M>.<w>_co_wf₁` artifacts.
3. **Naming the legs** (`paxos_co_sr`, `paxos_co_rr`, `Paxos/CoupleSafety.lean`): the
   corner's `sr` *is* `paxos_core@SchedSem`, its `rr` *is* `paxos_core@Values` at the
   derived decisions — per-module `rfl` identities (`CoupleProj.lean`, `co_transfer`),
   assembled with modules folded because whole-program defeq through nested knots is
   exponential (D40).
4. **Transport**: a machine commit is in the machine's replica view; the coupling says
   that view is a sub-multiset of the abstract pool; the abstract contract decides.

The same four moves with different names are `cq_safe_sched'` (`Std/Quorum.lean:1386`)
for the shared quorum stage.

## What the corner is not

- Not adequacy: there is no `∀ decisions ∃ schedule`, and there cannot be — the machine
  realizes one emission order where the quotient licenses all permutations
  (`../CORRESPONDENCE.md` §not proven). Safety needs machine ⊆ denotation only.
- Not a semantics of compiled Rust: it is a semantics of the Hydro combinators, audited
  against the runtime axis by axis in `../SCHED_AUDIT.md`. The one place a judgment
  cannot be mechanized is "is the adversary strong *enough*" — the audit checklist is
  that review.

## In-tick operators at the corner

Since one tick's content is a `Multiset` at `Values` and a `List` at `SchedSem`, every
in-tick operator is implemented twice and **coupled per operator** (`CoBounded`,
`Couple.lean`, D60): the corner proves that running the body on the list and on the
quotient agree up to the grade's relation. The `tick` block's `wf` is exactly this body
coupling (`CoTick.scanWf`), which is why a Rust `NoOrder` body cannot observe the
schedule's order — and why no "emission linearization" decision exists anymore (D61).
