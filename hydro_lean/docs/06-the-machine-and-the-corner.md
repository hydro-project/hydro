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
  Atomic ℓ σ := Fin (mem ℓ) → Nat → Trace σ
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
-- Hydro/Paxos/CoupleWf.lean
theorem paxos_safe_sched (T : Nat) (hnA : mem acc ≤ 2 * f + 1)
    (cpS : (SchedSem L mem pacing).Stream prop P .totalOrder .exactlyOnce)
    (ckS : CkInput pacing acc) (hck : CkMono pacing acc ckS)
    (sdec : PaxosCoreDec (SchedSem L mem pacing) (mem prop) (mem acc) P (Option Nat) .totalOrder)
    (ssched : PaxosCoreSched (SchedSem L mem pacing) (mem prop) (mem acc) P)
    (i j : Fin (mem prop)) (s : Nat) (w w' : Option P)
    (hw : (s, w) ∈ ((paxos_core (SchedSem L mem pacing) .guarded prop acc f cpS ckS sdec ssched).2 i).view T)
    (hw' : (s, w') ∈ ((paxos_core (SchedSem L mem pacing) .guarded prop acc f cpS ckS sdec ssched).2 j).view T) :
    w = w'
```

`pacing` — **any** per-member pace (idle / stutter / tick at each step); `sdec`/`ssched` —
**any** timing and delivery cursors; `T` — **any** horizon; `cpS` — **any** client stream;
`ckS` — **any** checkpoint register that only grows (`CkMono`, the wire's own `Monotonic`
typing). Conclusion: two commits the *step machine's* `paxos_core` has emitted at one
slot agree.

It is proved through the transfer lemma `paxos_safe_sched'` (same file), whose
statement still carries the coupling's shape — Values pools `cpV`/`ckV` above the inputs
with `hcp`/`hck` — as hypotheses. `paxos_safe_sched` builds those from the inputs
themselves: truncated at `T`, the view is its own pool and the register is the
`optMax`-fold of its own history; causality (`Dag.denote_causal` through
`paxos_core.dag_denote`) carries the commits back to the untruncated run. This is what
closes `SCHED_AUDIT.md` F2.

Absent: a decision argument, a satisfiability hypothesis. The theorem's predecessor had
one (`hsat : δ T d`) and it turned out to be **unsatisfiable for nested knots** — the
headline was vacuously true for a year in a zero-sorry tree (D37;
`../CORRESPONDENCE.md` §cautionary). That episode is why `DOCTRINE.md` R7 exists.

## The corner

The transfer lemma's proof is four names and one contract application (`CoupleWf.lean`):

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

## From the machine to processes (D74)

`SchedSem` is one closed run. `Project` (`../ARCHITECTURE.md` §7b) is how that run
becomes **processes**: `hydro def` also compiles the body into a typed dataflow graph,
`paxos_core.dag`, with a theorem that stepping the graph denotes the `SchedSem` run
(`paxos_core.dag_denote`, generated — kernel `rfl` for leaf modules, callee lemmas by
`simp` for the rest). A process is `Dag.locStep` over the graph at *its own members'*
cells (`Hydro/Project/Wired.lean`): it reads what arrived on its queues, ticks — or
*stutters*: lets deliveries land and knot feedback propagate without firing any tick
body (`locStep m tk`) — and emits `Sends`. `Dag.step_wired` (`Project/Model/WiredMain.lean`) says that when nobody else is
active, the global step cut to this process IS `locStep` and the others only receive;
`Interleaving.outputs` (`Project/Model/WiredRun.lean`) lifts this to a whole interleaving (any
"who goes now", and whether it ticks or stutters — nothing else: a member that is not
active is inert, D76; one DFIR tick is a tick then stutters — and, where a knot feeds a
tick input, a further tick — settled inside one activation, D93);
`paxos_safe_wired` (`Paxos/WiredSafety.lean`) then is the headline for a network:

```
theorem paxos_safe_wired (hnA : mem acc ≤ 2 * f + 1) (I : Interleaving (paxosDag prop acc f)) (hck : I.CkMono …)
    (T : Nat) (i j : Fin (mem prop)) … :
    (slot, v) ∈ I.streamOutputsAt T ⟨prop, i⟩ sequenced _ → (slot, v') ∈ I.streamOutputsAt T ⟨prop, j⟩ sequenced _ → v = v'
```

`I` is the whole execution: who runs at each step, whether it ticks or stutters, the
inputs, and the schedule — per network edge, per (receiver, sender), how many of the
sender's messages the receiver has read by each step; per timer, per member, its
verdicts. The compiled graph `paxosDag` carries no decisions (D77); the program's
decision and schedule bundles are rebuilt from `I.sched` inside the proof
(`paxos_core.recordOf`, with `paxos_core.schedOf_recordOf` by kernel `rfl`).

`I.streamOutputsAt T m v i` is what process `m` has output on stream `v` by time `T`,
where a process is the **loop body it runs** (D78): `Dag.iterate` (`Project/Process.lean`)
— read the inbox, deliver everything, one tick of the projected graph, send — is a
library function; `lake exe wired` calls it; `Interleaving.loop_active` says an
activation of it, fed the messages its connections deliver next, *is* the lock-step
step. What the theorem does not cover is the shell: sockets, line framing, and the
accumulator that hands `iterate` one FIFO per connection.

A process's increments are what Rust's wires carry: a `Singleton`/`Optional` wire moves
one value per tick, so its increment is a slot (`Option`), and the one buffered value a
`defer_tick` holds is released at the next tick — the FIFO DFIR runs
(`defer_tick.rs:6`), which is also the machine's definition of `defer_tick` (D85;
`toy_defer_next_tick` in `HydroTickCheck.lean` runs the machine and the node side by
side). The per-step bound behind the slot, `TPaced`, is a proved property of every
Atomic-producing operator and a hypothesis of `Dag.run_spec`.

`lake exe wired` runs it: two proposers and three acceptors on `127.0.0.1:9400–9404`,
one frame per message on the wire (`paxos_core.wireCodecs`, from the message types'
`Wire` instances), each receiver cutting the byte stream wherever TCP cut it
(`Dag.iterateBytes`, D90); proposer 0 commits `1000` at slot 0, and the commits are checked
slot-functional across the processes — a TCP run is a `WireTrace`, and the theorem ranges
over all of them.
