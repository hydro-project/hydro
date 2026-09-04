---
sidebar_position: 8
title: Soundness and the Trust Base
---

# Soundness and the Trust Base

The Paxos protocol argument is proved over `Values`, the denotational semantics, where a stream captures all the histories it could have had and the network is an identity operator ([The Approach](./approach.md#quotient-denotations)). The headline `paxos_safe_wired` is about something else: the bytes that processes running a compiled function exchange over TCP. Between the two runs a chain of theorems, and every link is proved. What a reader has to check is the hypotheses each link adds, because a hypothesis nobody discharges, or one that no run can satisfy, makes everything above it empty, and the definitions the headline's statement mentions, because they are what the theorem is about.

The chain passes three levels, and Paxos has a safety theorem at each: `paxos_core.ensures` at the denotational level (link 1), `paxos_safe_sched` at the operational level of `SchedSem` (link 3), and `paxos_safe_wired` at the compiled graph's execution as processes (link 8). Links 2–3 cross from the first level to the second through the coupling; links 4–8 cross from the second to the third, starting with `dag_denote`, which says the graph read with `SchedSem`'s definitions equals the program's `SchedSem` run ([The Approach](./approach.md) has the levels). The sections below take the links from the denotation down to the bytes, then list the trust base; the [Architecture Overview](./index.md#the-theorem-chain) draws the chain as a diagram.

## The Links at a Glance

| link | what it reaches | theorem | hypotheses it adds |
|---|---|---|---|
| 1 | the protocol at `Values` (level 1) | `paxos_core.ensures` (`slot_functional`) | `.guarded` variant, quorum condition |
| 2 | the step machine `SchedSem` | `paxos_co_sr`, `paxos_co_rr`, `paxos_co_wf`, `paxos_co_cpl`, `paxos_safe_sched'` | `Values` pools above the inputs |
| 3 | `SchedSem`, inputs unconstrained (level 2) | `paxos_safe_sched` | checkpoints only grow (`CkMono`); the pools are gone |
| 4 | the lock-step machine on the graph | `paxos_core.dag_denote`, `Dag.run_spec`, `schedOf_recordOf` | graph validity, inputs monotone and paced |
| 5 | processes in any interleaving | `Dag.step_wired`, `Interleaving.outputs` | graph validity; the input hypotheses of link 4 hold by construction |
| 6 | the activation a process runs | `Dag.iterate_eq_pass`, `Interleaving.loopPass_outputs` | graph validity |
| 7 | traces of bytes | `WireTrace.streamOutputsAt_eq` | graph validity |
| 8 | the staged code; the headline `paxos_safe_wired` (level 3) | `paxosRun.eq_impl`, `paxosRun.eq`, `paxosBody_eq` | none (a kernel judgement and a `@[csimp]` equation) |

The objects the links connect: `Values` ([grades](./semantics/grades.md), [values](./semantics/values.md)); the step machine `SchedSem`, a run on a global clock with queues, delivery cursors (how many of a sender's messages each receiver has taken) and a **pacing** (which member idles, receives or ticks at each step) ([sched](./semantics/sched.md)); the corner `CoupleSem` ([coupling](./corner/coupling.md)); the compiled graph `M.dag` and its meaning `Dag.denote` ([graph IR](./execution/graph-ir-causality.md)); the **lock-step machine** `Dag.run`, which steps every member of the graph at each global step ([lock-step](./execution/lockstep-machine.md)); an `Interleaving`, one process active per step ([processes](./execution/processes.md)); and a `WireTrace`, who ran with which inputs and clock readings and how many bytes each read off each connection ([traces](./execution/wire-and-traces.md)).

## Link 1: The Contract on the Denotation

```lean
def SlotFunctional {n : Nat} (out : MemberMap n (Multiset (Nat × Option P))) :
    Prop :=
  ∀ (i j : Fin n) (s : Nat) (v w : Option P),
    (s, v) ∈ out i → (s, w) ∈ out j → v = w

structure PCEnsures (variant : PaxosVariant) (prop acc : L) (f : Nat)
    (out : (MemberMap (mem prop) (Trace (Ballot (mem prop))))
      × (MemberMap (mem prop) (Multiset (Nat × Option P)))) : Prop where
  /-- Per-slot agreement of the replica stream. -/
  slot_functional : variant = .guarded → mem acc ≤ 2 * f + 1 →
    SlotFunctional out.2
```

`paxos_core.ensures` is the module's contract: `PCEnsures` for every input class and every decision record. Its fields are the module's **faces** ([contracts](../using/programs/contracts.md)). No legality condition on the decisions is needed, because every decision space is total: an illegal batch cut truncates the run, and an illegal order blocks it ([nondeterminism](../using/programs/nondeterminism.md)).

The proof of the field `slot_functional` sits inside `paxos_core` (`Examples/Paxos/PaxosCore.lean`). It consumes two facts. The first is the packaged induction of the `a_log` **knot** (a `fix … complete` cycle, Rust's `forward_ref`): a value chosen at a lower ballot is the value every later leader opens, by quorum intersection and by the provenance of published entries through the cycle ([knot invariants](../using/proving/knot-invariants.md)). The second is `sequence_payload.ensures`, whose `commit_distinct` gives two witness ballots for two distinct commits. No callee is unfolded, and no message in flight is mentioned.

The two hypotheses are the variant and the quorum condition. `.faithful` transcribes the Rust as written, and it violates agreement ([fidelity](https://github.com/hydro-project/hydro/blob/main/hydro_lean/internal/FIDELITY.md)).

## Link 2: The Coupling Corner

Instantiating `paxos_core` at `CoupleSem` runs the step machine and `Values` in one evaluation. Each corner wire carries a machine run `sr`, a denotational run `rr`, a **lag**, a **residual** `wf`, and a proof `cpl` that under `wf` the machine's list at each step `k` with `k + lag ≤ T` lies below the denotation's class, `T` being the **horizon** ([the approach](./approach.md#coupling-the-denotation-to-the-step-machine) shows the carrier). The order between list and class (`ListLe`) is prefix, sub-multiset or support inclusion according to the grade. An inclusion is enough here, because `SlotFunctional` is closed downward: a sub-multiset of a slot-functional output is slot-functional. Liveness needs an equality and gets one separately ([liveness](./liveness.md)).

Each operator whose result depends on a decision computes its `Values` decision from its own machine run. The generated record collects these decisions, and the denotational run `rr` equals the `Values` run at that record:

```lean
theorem paxos_co_rr :
    (paxos_core (CoupleSem L mem pacing T T (Nat.le_refl T))
        .guarded prop acc config (CoStream.inputC cpS cpV hcp)
        (CoSing.inputC ckS ckSrc ckV hck)
        (paxosCoDec pacing prop acc T sdec)
        (paxosCoSched pacing prop acc T ssched)).2.rr
      = (paxos_core (Values L mem) .guarded prop acc config
          cpV ckV
          (paxosVDecG pacing prop acc T config cpS ckS ckSrc sdec ssched)
          PaxosCoreSched.triv).2
```

`paxosVDecG` computes the decision from the machine's inputs and schedule, so `paxos_co_rr` holds with no premise that a matching decision exists. [The approach](./approach.md#coupling-the-denotation-to-the-step-machine) shows why such a premise fails for nested knots: it requires consecutive Kleene stages of an outer knot to be equal, and they differ until the iteration converges.

`paxos_co_sr` says the machine run `sr` equals the `SchedSem` run. `paxos_co_wf` says the residual holds: it is the generated `paxos_core_co_wf₂` applied to `trivial trivial rfl`, the residuals of the two input constructors and the module's `requires variant = .guarded`, and it covers all five knots' obligations. `paxos_co_cpl` is `cpl` at the full horizon. `paxos_safe_sched'` (`Examples/Paxos/CoupleWf.lean`) then takes `slot_functional` at the derived record, rewrites the coupling at both proposers with `paxos_co_sr` and `paxos_co_rr`, and moves each commit's membership through the inclusion with `Multiset.mem_of_le`. The same four moves for `collect_quorum` are walked through in [machine theorems](../using/proving/transferring-to-the-machine.md).

The link adds hypotheses of shape, not of fact: a `Values` class `cpV` above the client input (`hcp : ∀ t i, ((cpS i).view t) <+: cpV i`) and a checkpoint class `ckV` with `hck` relating the register to a fold over a source. They are what the input constructors `CoStream.inputC` and `CoSing.inputC` need, and the next link removes them.

## Link 3: Unconstrained Inputs

For an unbounded input stream a `Values` class above every view need not exist in a useful form. `paxos_safe_sched` (`Examples/Paxos/CoupleWf.lean`) avoids the question. It cuts the inputs at the horizon, so the truncated stream's view at `T` is its own class, and carries the commits back to the untruncated run by causality of the compiled graph:

```lean
  let cpS' : (SchedSem L mem pacing).Stream prop P .totalOrder .exactlyOnce := fun i => (cpS i).truncate T
  let ckS' : CkInput pacing acc := fun j t => ckS j (min t T)
  …
  have hcaus := Dag.denote_causal (pacing := pacing) (paxos_core.dag (ckret := .exactlyOnce) .guarded prop acc config)
    (paxos_core.schedOf pacing .guarded prop acc config sdec ssched) T env env' hagree
  rw [paxos_core.dag_denote, paxos_core.dag_denote] at hcaus
```

`Dag.denote_causal` says that two input environments agreeing up to a step give graph runs that agree up to that step. It is proved once for every graph, so no program carries a causality proof, and `dag_denote` carries it to the program. The checkpoint register's class is built as the `optMax` fold of its own value history, and that fold reproduces the register only if the register never decreases (`foldl_optMax_ckList`). Monotonicity is therefore the one premise that stays:

```lean
theorem paxos_safe_sched (T : Nat) (hnA : mem acc ≤ 2 * config.f + 1)
    (cpS : (SchedSem L mem pacing).Stream prop P .totalOrder .exactlyOnce)
    (ckS : CkInput pacing acc) (hck : CkMono pacing acc ckS)
    (sdec : PaxosCoreDec (SchedSem L mem pacing) (mem prop) (mem acc) P (Option Nat) .totalOrder)
    (ssched : PaxosCoreSched (SchedSem L mem pacing) (mem prop) (mem acc) P)
    (i j : Fin (mem prop)) (s : Nat) (w w' : Option P)
    (hw : (s, w) ∈ ((paxos_core (SchedSem L mem pacing) .guarded prop acc config cpS ckS sdec ssched).2 i).view T)
    (hw' : (s, w') ∈ ((paxos_core (SchedSem L mem pacing) .guarded prop acc config cpS ckS sdec ssched).2 j).view T) :
    w = w'
```

`CkMono` restates the program's `Monotonic` typing of the `a_checkpoint` input as a fact about the input, not a device of the proof: the acceptors' log garbage collection is unsound on a checkpoint that goes back (finding B4 in [fidelity](https://github.com/hydro-project/hydro/blob/main/hydro_lean/internal/FIDELITY.md#b4-a-fresh-payload-at-a-collected-slot)). The constant `none` satisfies it.

## Link 4: The Graph and the Lock-Step Machine

A `HydroSem` instance sees operator applications, not the body's `let`-sharing, and cannot step a `fix` body once per global step, so execution goes through a graph ([the approach](./approach.md#the-dataflow-graph)). For every module the generator proves `M.dag_denote`: the graph's reading `Dag.denote` equals the module's `SchedSem` run (the statement for `toy_relay` is in [project phase](./generator/project-phase.md)). It also proves that every schedule of the graph is some pair of decision and schedule bundles:

```lean
@toy_relay.schedOf_recordOf : ∀ {L : Type} {mem : L → ℕ} (pacing : Pacing L mem) (ℓ : L) (S : (toy_relay.dag ℓ).Sched),
  toy_relay.schedOf pacing ℓ (toy_relay.recordOf pacing ℓ S).1 (toy_relay.recordOf pacing ℓ S).2 = S
```

The graph carries no decisions, so the quantifier over schedules sits on the execution side, and the headline can take the schedule from the trace and turn it back into bundles.

The lock-step machine reproduces `Dag.denote` step for step:

```lean
theorem Dag.run_spec {Γ : Ctx L mem} {outs : List (Kind L mem)} (d : Dag Γ outs) (sch : Dag.Sched d)
    (hv : Dag.valid d = true) (env : Env Γ)
    (he : HTMono env) (hp : HTPaced pacing env) (t : Nat) :
    Dag.run d sch pacing (fun t => envIncAt env t) t
      = (Dag.stateOf pacing d sch env t, envIncAt (Dag.denote pacing d sch env) t)
```

- `HTMono` says the input histories grow by prefixes.
- `HTPaced` says an atomic (tick-located) input gains at most one entry per step, and none when its member is idle.
- `Dag.valid` says every cycle among a block's knots passes a network hop or a `defer_tick`. Validity makes each knot wire the fixpoint of its body within one step, and the step-for-step equality needs that fixpoint ([sched](./semantics/sched.md)).

For `paxos_core` validity is `paxos_core.dag_valid`, which the generator proves one node at a time rather than by evaluating `Dag.valid` on the whole graph ([cost model](./cost-model.md#kernel-evaluation-of-graphs-lags-and-knot-wires)).

## Link 5: Interleaved Processes

An `Interleaving` is four functions, `who`, `tick`, `inp` and `sched`, and every quadruple is one. `Dag.step_wired` says that a lock-step step in which only the active process's members are active equals that process's `Dag.locStep` on its own cells, with its sends enqueued everywhere. Its one condition on the state, `Drained`, holds of every state the machine produces (`Dag.init_drained`, `Dag.step_drained`). Lifted to interleavings (`Hydro/Project/Model/WiredRun.lean`):

```lean
theorem outputs (hv : Dag.valid d = true) (k : Nat) :
    (I.active k).2.1 = envRestrict (I.who k) (envIncAt (Dag.denote I.pacing d I.sched I.wires) k) :=
```

`I.wires` is the environment whose increments are the interleaving's inputs masked to the active process, and `I.wires_mono` and `I.wires_paced` discharge `run_spec`'s two input hypotheses by construction. The interleaving's inputs are therefore arbitrary, and there is no settling or quiescence hypothesis. A member that is not active is inert in `SchedSem`: delivery, knot feedback and `defer_tick` are all gated by the receiver's **pace** at the step (`idle`, `stutter`, that is active with no tick body, or `tick`), so a sleeping process is an idle member of the lock-step machine.

## Link 6: The Activation

What a process runs per activation is `Dag.iterate`: it enqueues the inbox, materializes the cells, and makes one sweep of the graph in which every knot wire is bound to its body's output of the same tick. The proofs are about `Dag.pass`, the same activation with `Dag.locStep` in place of the sweep (`Hydro/Project/Model/SweepSpec.lean`):

```lean
theorem Dag.iterate_eq_pass (m : Member L mem) (tk : Bool) {Γ : Ctx L mem} {outs : List (Kind L mem)} (d : Dag Γ outs)
    (hv : Dag.valid d = true) (tm : Dag.Timing m d) (ib : Dag.Inbox m d) (Δ : HList (Kind.locDelta m) Γ)
    (s : Dag.LocState m d) : Dag.iterate m tk d tm ib Δ s = Dag.pass m tk d tm ib Δ s
```

Validity appears again: the sweep's knot loop binds each knot to its reference value only because the body's step respects the lag analysis (`Dag.locStep_agree0`; [activation](./execution/activation.md)). `Dag.iterateBytes_eq_passBytes` states the same equation at the bytes, and `Interleaving.loopPass_outputs` identifies a process's outputs under `Dag.pass` with the interleaving's step.

## Link 7: Byte Traces

```lean
structure WireTrace {Γ : Ctx L mem} {outs : List (Kind L mem)} (d : Dag Γ outs) where
  who : Nat → Member L mem
  inp : Nat → HList (Kind.delta (mem := mem)) Γ
  readings : (k : Nat) → TickReadings (who k)
  reads : Nat → List (Member L mem × Nat)
```

There are no proof fields. `WireTrace.run` hands the active process each listed connection's kept tail plus the requested bytes (as many as are queued, cut anywhere, mid-frame included); the body parses the complete frames and keeps the incomplete tail. The link's theorem is (`Hydro/Project/Model/WireTrace.lean`)

```lean
theorem streamOutputsAt_eq (hv : d.valid = true) (cod : Dag.Codecs d) (τ : WireTrace d) (T : Nat) (m : Member L mem) {ℓ : L}
    {α : Type} {iα : DecidableEq α} {ord : StrOrd} {ret : Retries} (v : Var outs (.stream ℓ α iα ord ret)) (i : Hosted m ℓ) :
    τ.streamOutputsAt (Body.ofCodecs cod) T m v i = (τ.interleaving cod).streamOutputsAt T m v i
```

The trace induces the interleaving; nothing about it is assumed. Its delivery cursor on a connection at step `t` is the number of that connection's messages among the frames the receiver has parsed by `t` (`Dag.traceSched`). Two invariants carry the induction: `RunInv` says a receiver's byte cursor is within what the sender wrote and its kept tail is the bytes past the last complete frame, and `Dag.TraceQueues` says each receiving queue holds the connection's messages past the parsed prefix. The framing law `Wire.framesFrom_prefix_frameAll` turns bytes read into messages parsed ([wire and traces](./execution/wire-and-traces.md)).

## Link 8: The Staged Function

`paxosRun` is `paxosDag` evaluated at elaboration time into straight-line code by `hydro_run` ([staging](./execution/staging.md)). The stager proves two equations. `paxosRun.eq_impl`, a kernel judgement (`addThmByKernelRfl`), says that the staged code equals `Dag.iterateBytesImpl`, the compiled implementation of the interpreter. `paxosRun.eq` composes it with the `@[csimp]` equation `Dag.iterateBytes_eq_impl` (applied by `congrFun` to the arguments) to reach the interpreter itself. Shown with the binders after the instances elided:

```lean
@paxosRun.eq : ∀ {L : Type} {mem : L → ℕ} [inst : DecidableEq L] (P : Type) [inst_1 : DecidableEq P] [inst_2 : Wire P]
  …
  paxosRun P prop acc config m tk r chans Δ s =
    Project.Dag.iterateBytes m tk (paxosDag prop acc config) (paxosCodecs prop acc config)
      (Project.Dag.Timing.ofReadings m r (paxosDag prop acc config)) chans Δ s
```

`paxosBody` is `paxosRun … m true`, the function `lake exe wired` calls, and `paxosBody_eq` is `paxosRun.eq` pointwise.

## The Headline

The statement, with its source comments (`Examples/Paxos/WiredSafety.lean`):

```lean
theorem paxos_safe_wired [Wire P]
    -- Paxos's quorum condition: at most `f` of the acceptors may be missing from a quorum.
    (hnA : mem acc ≤ 2 * config.f + 1)
    -- The trace at the wire.
    (τ : WireTrace (paxosDag (mem := mem) (P := P) prop acc config))
    -- The checkpoint input never decreases (the `a_checkpoint` wire's `Monotonic` type;
    -- the acceptors' log GC is unsound without it — B4). Satisfied by the constant `none`.
    (hck : (τ.interleaving (paxosCodecs prop acc config)).CkMono prop acc)
    -- A time, two proposers, a slot, two values.
    (T : Nat) (i j : Fin (mem prop)) {s : Nat} {w w' : Option P}
    -- The two commits: what proposer `i` (the process `⟨prop, i⟩`) has output on
    -- `Var.zero.succ` — the second output, the `sequenced` commit stream — by time `T`.
    (hw : (s, w) ∈ τ.streamOutputsAt (paxosBody prop acc config) T ⟨prop, i⟩ Var.zero.succ ⟨i, rfl⟩)
    (hw' : (s, w') ∈ τ.streamOutputsAt (paxosBody prop acc config) T ⟨prop, j⟩ Var.zero.succ ⟨j, rfl⟩) :
    w = w' := by
  -- the compiled body equals the interpreter's (`paxosRun.eq`), and the trace's run under it
  -- equals the induced interleaving's (the graph is valid: `paxosDag_valid`)
  rw [paxosBody_eq, τ.streamOutputsAt_eq (paxosDag_valid prop acc config)] at hw hw'
  set I := τ.interleaving (paxosCodecs prop acc config) with hI
  -- the activations that output them, and their outputs as the lock-step step's
  obtain ⟨k, _, hk, hw⟩ := I.mem_streamOutputsAt hw
  obtain ⟨k', _, hk', hw'⟩ := I.mem_streamOutputsAt hw'
  rw [I.loopPass_outputs] at hw hw'
  …
  exact paxos_safe_sched I.pacing prop acc config (max k k') hnA cpS ckS hckS sdec ssched _ _ s w w' hw1 hw2
```

The proof runs the chain from the bytes up: links 8 and 7 by rewriting, link 6 by `loopPass_outputs`, the schedule back to bundles by `recordOf` (link 4), the outputs to the lock-step machine and `dag_denote` (links 5 and 4), and the machine to `paxos_safe_sched`. The two commits come from activations `k` and `k'`, so the machine theorem is applied at `max k k'`, using that the commit history only grows.

`hck` mentions `τ.interleaving`, but only its inputs: `PassTrace.interleaving` is `⟨τ.who, τ.tick, τ.inp, τ.sched cod⟩`, so `(τ.interleaving cod).inp` is `τ.inp` by definition, and the premise is about the trace's checkpoint inputs.

## The Trust Base

**Axioms.** `#print axioms` reports, for the headline:

```text
'Hydro.paxos_safe_wired' depends on axioms: [propext, Classical.choice, Quot.sound]
```

`Hydro/AxCheck.lean` and `Examples/AxCheck.lean` print the same for every link above and for a sample of each generated artifact class, and the [gate](https://github.com/hydro-project/hydro/blob/main/hydro_lean/internal/GATE.md) fails on any other axiom and on any `sorry`.

**Definitions in the statement.** Only the definitions that occur in `paxos_safe_wired`'s statement can make it mean something other than what it says. `Values`, `SchedSem`, `CoupleSem`, `Dag.denote`, `Dag.run` and `Interleaving` occur only in the proof: a mistake in one of them can make the proof fail, but not the theorem false. What the statement does mention must be read as a model:

| definition | what it must describe |
|---|---|
| `WireTrace`, `WireTrace.run`/`step`/`actOf` | a deployment: per-connection byte queues, reads of any prefix, kept tails, sends appended as frames, crash-stop as a process that stops activating |
| `WireTrace.streamOutputsAt` | what a process has output: its activations' output increments, in order |
| `paxosBody` = `paxosRun … m true` | the function the shell calls, and through `paxosDag`, the compilation of `paxos_core` |
| `Var.zero.succ` of `paxosDag` | the `sequenced` commit stream (the second output) |
| `Interleaving.CkMono` | checkpoints only grow |

The theorem is only as good as the claim that `paxos_core` and the node semantics (`Node.locStep`) mirror the Rust program and DFIR's operators. The reader checks that correspondence by reading: one Rust line per Lean line, with each operator's execution model citing the production behaviour it mirrors ([doctrine](./doctrine.md) P1 and R13, [fidelity](https://github.com/hydro-project/hydro/blob/main/hydro_lean/internal/FIDELITY.md)). A model that admitted too little (no reordering, synchronized ticks, reads that never land) would make the headline true and empty. `Hydro/WireTraceChecks.lean` exhibits by `#guard` that `WireTrace` admits each adversarial behaviour: cross-connection interleaving, silence, mid-frame cuts, inert activations, the fixpoint race, pacing-dependent cuts, consulted timer readings and per-member independence. `Examples/Paxos/WireTraceCheck.lean` runs the staged body on scenarios in which commits happen.

**The denotation, for statements that mention it.** A `Values` contract such as `paxos_core.ensures` holds of every history in the class the denotation assigns and says nothing about a history outside it. Read on its own, it describes the program's runs only if every history a run can produce is in the class ([grades](./semantics/grades.md)), so such a contract trusts the denotation as a model. `paxos_safe_wired` does not: the corner proves, for every machine run, that its history is inside the class (`cpl`), and the statement mentions only bytes.

**The `@[csimp]` equations.** These are the only places where the compiled code is a different term from the one the theorems are about. Each is a kernel-checked equation `@f = @fImpl`, which the compiler substitutes at every occurrence:

| file | equations |
|---|---|
| `Hydro/MemberMap.lean` | `mk'_eq_mkImpl` |
| `Hydro/Project/Wired.lean` | `memoBox_eq_memoBoxImpl`, `memoBox2_eq_memoBox2Impl`, `Dag.locStep_eq_impl` |
| `Hydro/Project/Process.lean` | `parseFrames_eq_impl`, `frameMsgs_eq_impl` |
| `Hydro/Project/Fused.lean` | `bucketArray_eq_impl`, `Dag.iterateBytes_eq_impl` |
| `Hydro/Wire.lean` | `getNat_eq_impl`, `frameAll_eq_impl` |
| `Hydro/KeyedHash.lean` | `dedupH_eq_dedupImpl`, `arrFilterNotIn_eq_impl`, `arrAntiJoin_eq_impl`, `arrJoinH_eq_impl`, `keyedFoldAssocH_eq_impl`, `keyedFoldArrH_eq_impl`, `keyedReduceAssocH_eq_impl`, `SchedKMap.joinH_eq_impl`, `reduceWatermarkEagerH_eq_impl` |

An `implemented_by` would be an unchecked claim of the same shape. With `@[csimp]` the claim is a theorem, and what remains trusted is the compiler's application of it. Because the seams and the stager's compiled output are outside every theorem, `lake exe wired --traces` runs both the staged body and the interpreter's body `Body.ofCodecs` on the same traces and compares what they commit.

**Outside the theorem.**

- The sockets (`Hydro/Project/Sockets.lean`), which hand the body the bytes TCP delivered.
- The Lean compiler and runtime.
- The channel scope: every network edge of the mirrored programs is `TCP.fail_stop`, and the cursor model is per-connection FIFO prefix delivery, which is what fail-stop TCP provides. Lossy links are not modelled.
- The Rust-to-Lean correspondence above, which is checked by reading.

## The Other Headlines

`cq_safe_sched` (`Hydro/Std/Quorum.lean`) and `two_pc_safe_sched`/`two_pc_once_sched` (`Examples/TwoPC.lean`) are links 1 to 3 for `collect_quorum` and `two_pc`. `cq_safe_sched` ends in an existential over the `Values` decision whose witness is `collect_quorum_vdec`, the decision the run realized ([machine theorems](../using/proving/transferring-to-the-machine.md)).
