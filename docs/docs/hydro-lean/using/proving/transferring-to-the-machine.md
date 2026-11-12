---
sidebar_position: 4
title: "Transferring to the Machine and the Network"
---

# Transferring to the Machine and the Network

So far our proofs are about the denotational semantics `Values` only, where a broadcast delivers everything and each `nondet!` choice is an argument. Nothing yet says that unanimity or slot agreement holds when the program runs. The theorems that say so are about runs: first of the step machine `SchedSem`, which runs the same program text step by step under a **pacing** (per member and step: idle, active without running tick bodies, or ticking), delivery **cursors** (how much of each sender's output each receiver has been handed) and timers; then of the processes exchanging bytes.

Neither step needs new protocol reasoning. Getting from a face to the step machine takes the same four moves for every module, citing lemmas `hydro def` generates for it, plus a causality argument that removes the one remaining premise. Getting from the step machine to the processes is one more proof whose shape is the same for every program. This page walks through both, for two-phase commit and then for Paxos.

## The Statement

`two_pc_safe_sched` (`hydro_lean/Examples/TwoPC.lean`) is unanimity for every run of the step machine:

```lean
-- for any run of the step machine: if coordinator `c` has committed `p` by
-- step `T`, and `p` was proposed once among the proposals received by `T`,
-- then every participant voted yes
theorem two_pc_safe_sched (hn : 1 ≤ mem part) (c : Fin (mem coord)) (p : P)
    (hp : p ∈ ((two_pc (SchedSem L mem pacing) coord part (mem part) vote
      cpS sdec ssched) c).view T)
    (hone : (tpcProposed fun c => (↑((cpS c).view T) : Multiset P)).count p = 1) :
    ∀ j, vote j p = true := by
```

`cpS` is any proposal stream, given as a **step history**: per member, the list of items that have arrived by each step. `pacing`, `ssched` (the delivery cursors of the four network hops) and the horizon `T` are arbitrary. `hp` says coordinator `c` has committed `p` by step `T`. `hone` says `p` was proposed once among the proposals received by `T`. There is no premise about the network, the schedule or a decision.

We prove it in two steps. The face we want to apply is stated over a `Values` input, a pool, so we first prove a version with an extra premise, a pool above the machine's input (`two_pc_safe_sched'`). Then we remove that premise.

## The Coupled Run

The contract speaks about the denotational run's output, and `hp` speaks about the machine run's, so we need a fact relating the two runs of the same program. That fact comes from a third interpretation, `CoupleSem`, which runs the program at `SchedSem` and at `Values` at once. Each collection in a `CoupleSem` run is a **corner** with a machine run `sr` (the collection as `SchedSem` computes it), a denotational run `rr` (as `Values` computes it), and the coupling

```lean
  -- given the side condition `wf`: at every step `k` up to the horizon, each
  -- member's machine output so far is below its denotational output
  cpl : wf → ∀ k, k + lag ≤ T → ∀ i, ListLe ord ret ((sr i).view k) (rr i)
```

Up to the horizon `T`, what the machine has produced at each step is below what `Values` produces: a sub-multiset for a `.noOrder` stream, a prefix for a `.totalOrder` one. `wf` and `lag` are side conditions, and generated lemmas discharge both. Hence anything the contract says about every element of the denotational run's output holds of every element the machine has produced.

You build the coupled run's input from the machine input, a pool above it, and the proof that it is above (`Hydro/CoupleProj.lean`):

```lean
-- a coupled input from a machine history `h`, a pool `v`, and a proof `hc`
-- that every view of `h` is below `v`
def CoStream.inputC {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (h : (SchedSem L mem pacing).Stream ℓ α ord ret)
    (v : (Values L mem).Stream ℓ α ord ret)
    (hc : ∀ t i, ListLe ord ret ((h i).view t) (v i)) :
    (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α ord ret :=
```

`CoSing.inputC` is the same for a singleton input. The decision record at `CoupleSem` has `CoDec.batch _` (which is `()`) at each content decision, because the coupled run computes those itself, and the machine's own cursors and timers at each schedule site (`CoDec.cursors`, `CoDec.interval`). For `two_pc`:

```lean
-- the content decisions (the two batchings) are computed by the coupled run
def twoPCCoDec :
    TwoPCDec (CoupleSem L mem pacing T T (Nat.le_refl T)) (mem coord)
      (mem part) P :=
  ⟨CoDec.batch _, CoDec.batch _⟩
…
-- the delivery cursors of the four hops are the machine's own
def twoPCCoSched :
    TwoPCSched (CoupleSem L mem pacing T T (Nat.le_refl T)) (mem coord)
      (mem part) P :=
  ⟨CoDec.cursors ssched.prepCh, CoDec.cursors ssched.voteCh,
   CoDec.cursors ssched.commitCh, CoDec.cursors ssched.ackCh⟩
```

## The Lemmas You Cite

For every module `M`, with outputs numbered from 1, Hydro Lean generates:

| lemma | says |
|---|---|
| `M_co_srᵢ` | `sr` of the coupled run equals the `SchedSem` run, with `()` at the content decisions |
| `M_vdec` | a `Values` decision record computed from the machine's input and schedule |
| `M_co_rrᵢ` | `rr` of the coupled run equals the `Values` run at `M_vdec …` |
| `M_co_wfᵢ` | the output's `wf` holds, given the inputs' (and `M`'s `requires`, if it has one) |
| `M_co_lagᵢ` | the output's `lag`, as a function of the inputs' (0 for most modules) |

For `collect_quorum`, the binders before `responses` elided:

```lean
-- the machine run `sr` is `collect_quorum` at `SchedSem`, batching `()`
@collect_quorum_co_sr₁ : … (responses : HydroSem.Stream mem ℓ (K × Except E Unit) StrOrd.noOrder Retries.exactlyOnce) (min max : ℕ)
  (dec : HydroSem.BatchDec mem (mem ℓ) (K × Except E Unit) StrOrd.noOrder Retries.exactlyOnce),
  (collect_quorum (CoupleSem L mem pacing Tc Td hjT) ℓ responses min max dec).1.sr =
    (collect_quorum (SchedSem L mem pacing) ℓ responses.sr min max ()).1
-- the denotational run `rr` is `collect_quorum` at `Values`, at the batching
-- `collect_quorum_vdec` computes from the machine input
@collect_quorum_co_rr₁ : … ,
  (collect_quorum (CoupleSem L mem pacing Tc Td hjT) ℓ responses min max dec).1.rr =
    (collect_quorum (Values L mem) ℓ responses.rr min max (collect_quorum_vdec ℓ min max responses.sr)).1
-- the output's lag is the input's
@collect_quorum_co_lag₁ : … ,
  (collect_quorum (CoupleSem L mem pacing Tc Td hjT) ℓ responses min max dec).1.lag = responses.lag
-- the output's side condition holds if the input's does
@collect_quorum_co_wf₁ : … ,
  responses.4 → (collect_quorum (CoupleSem L mem pacing Tc Td hjT) ℓ responses min max dec).1.wf
```

`responses.4` is the input corner's `wf`, which is `trivial` for an input built with `inputC`. The decision of the denotational run, `collect_quorum_vdec ℓ min max responses.sr`, is a function of the machine input: it reads back the batching this machine run realized. The machine theorem therefore applies the contract at a concrete decision term that the machine run determines.

## The Four Moves

A machine theorem combines these lemmas in four moves:

1. Run the program at `CoupleSem` on an input built with `inputC`, and take the output's `cpl`, discharging its `wf` with `M_co_wf` and its lag with `M_co_lag`. The result says the machine run `sr` is below the denotational run `rr` up to `T`.
2. Rewrite `sr` with `M_co_sr`, which equates `sr` with the plain `SchedSem` run.
3. Rewrite `rr` with `M_co_rr`, which equates `rr` with the plain `Values` run at `M_vdec`.
4. Apply `M.ensures` at `M_vdec`, and carry the machine's output into it through the inclusion.

`two_pc`'s transfer follows this pattern, as does every machine theorem in the repository. Move 1 runs the program at `CoupleSem` and takes its `cpl`, with `wf` from `two_pc_co_wf₁` and the lag closed by the `co_lag` tactic:

```lean
-- move 1: in the coupled run, coordinator `c`'s machine commits by `T` are a
-- sub-multiset of its denotational commits
theorem two_pc_co_cpl (c : Fin (mem coord)) :
    ListLe .noOrder .exactlyOnce
      (((two_pc (CoupleSem L mem pacing T T (Nat.le_refl T)) coord part
          (mem part) vote (CoStream.inputC cpS cpV hcp)
          (twoPCCoDec pacing coord part T)
          (twoPCCoSched pacing coord part T ssched)).sr c).view T)
      ((two_pc (CoupleSem L mem pacing T T (Nat.le_refl T)) coord part
          (mem part) vote (CoStream.inputC cpS cpV hcp)
          (twoPCCoDec pacing coord part T)
          (twoPCCoSched pacing coord part T ssched)).rr c) :=
  (two_pc (CoupleSem L mem pacing T T (Nat.le_refl T)) coord part
      (mem part) vote (CoStream.inputC cpS cpV hcp)
      (twoPCCoDec pacing coord part T)
      (twoPCCoSched pacing coord part T ssched)).cpl
    -- the side condition, from the generated `wf` lemma (`trivial` for the input)
    (two_pc_co_wf₁ coord part (mem part) vote (CoStream.inputC cpS cpV hcp)
      (twoPCCoDec pacing coord part T) (twoPCCoSched pacing coord part T ssched)
      -- the lag is 0, so the coupling reaches the horizon `T`
      trivial) T (by co_lag [two_pc_co_lag₁]) c
```

Moves 2 and 3 rewrite `sr` and `rr`: `two_pc_co_sr` applies `two_pc_co_sr₁` (`sr` equals `two_pc` at `SchedSem` on `cpS`), and `two_pc_co_rr` applies `two_pc_co_rr₁` (`rr` equals `two_pc` at `Values` on `cpV` with the decision record `twoPCVDec`, `two_pc_vdec` applied to the machine's cursors). Move 4 applies the contract at that decision:

```lean
theorem two_pc_safe_sched' (hn : 1 ≤ mem part) (c : Fin (mem coord)) (p : P)
    (hp : p ∈ ((two_pc (SchedSem L mem pacing) coord part (mem part) vote
      cpS sdec ssched) c).view T)
    (hone : (tpcProposed cpV).count p = 1) :
    ∀ j, vote j p = true := by
  -- move 4's contract: unanimity at `Values`, at the decisions this run realized
  have hens := two_pc.ensures coord part (mem part) vote cpV
    (twoPCVDec pacing coord part T vote cpS ssched) TwoPCSched.triv rfl
  -- move 1: machine commits ⊆ denotational commits, in the coupled run
  have hcpl := two_pc_co_cpl pacing coord part T vote cpS cpV hcp ssched c
  -- moves 2 and 3: rewrite the two sides to the plain machine run and the plain
  -- denotational run
  rw [two_pc_co_sr pacing coord part T vote cpS cpV hcp sdec ssched,
    two_pc_co_rr pacing coord part T vote cpS cpV hcp ssched] at hcpl
  -- `p` is a machine commit, so a denotational commit; apply unanimity there
  exact hens.commit_unanimous c p
    (Multiset.mem_of_le hcpl (Multiset.mem_coe.mpr hp)) hn hone
```

After the `rw`, `hcpl` says the machine's commits by `T` are a sub-multiset of the denotational run's commits, and mentions `CoupleSem` nowhere. `hp`'s payload is therefore in the denotational run's output, and `commit_unanimous` applies. The `rfl` passed to `two_pc.ensures` discharges the face's premise `num_participants = mem part`.

For your own module, copy these four moves with its names. `collect_quorum`'s `cq_safe_sched'` (`Hydro/Std/Quorum.lean`) and `toy_safe_sched` (`Hydro/HydroGenCheck.lean`) are two more instances.

## Removing the Pool Premise

`two_pc_safe_sched'` holds for every machine run, but only given a `Values` pool `cpV` above every view of the input, and an infinite input has no such finite pool. To drop the premise we use that the machine's commits by `T` depend only on the input up to `T`: we cut the input at `T`, and the cut input's view at `T` is a finite pool above all its views.

The causality fact comes from the compiled graph of the program. Every module has a graph `M.dag` with a schedule `M.schedOf …`, and two artifacts tie it to the program:

```lean
-- reading the graph gives `collect_quorum` at `SchedSem`, both outputs
@collect_quorum.dag_denote : … (pacing : Pacing L mem) (ℓ : L) (min max : ℕ)
  (dec : HydroSem.BatchDec mem (mem ℓ) (K × Except E Unit) StrOrd.noOrder Retries.exactlyOnce)
  (env : …),
  Project.Dag.denote pacing (collect_quorum.dag ℓ min max) (collect_quorum.schedOf pacing ℓ min max dec) env =
    Project.HList.cons (collect_quorum (SchedSem L mem pacing) ℓ (env.get Project.Var.zero) min max dec).1
      (Project.HList.cons (collect_quorum (SchedSem L mem pacing) ℓ (env.get Project.Var.zero) min max dec).2
        Project.HList.nil)
```

(shortened: the implicit binders and the input environment's type are elided). `M.dag_denote` says reading the graph gives the program's `SchedSem` run. `Dag.denote_causal` (`Hydro/Graph/Causal.lean`) holds for every graph: inputs that agree up to `h` give outputs that agree up to `h`.

```lean
-- inputs that agree up to step `h` give outputs that agree up to step `h`
theorem Dag.denote_causal : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) → (sch : Dag.Sched d) →
    ∀ (h : Nat) (env env' : Env Γ), HAgree h env env' →
    HAgree h (Dag.denote pacing d sch env) (Dag.denote pacing d sch env')
```

`two_pc_truncate` uses the two together. It packs the input and its truncation as environments, shows they agree up to `T`, applies causality, and rewrites both sides with `dag_denote`:

```lean
-- the commits by `T` are the same on the input cut at `T`
theorem two_pc_truncate (c : Fin (mem coord)) :
    ((two_pc (SchedSem L mem pacing) coord part (mem part) vote cpS sdec ssched) c).view T
      = ((two_pc (SchedSem L mem pacing) coord part (mem part) vote
          (fun c => (cpS c).truncate T) sdec ssched) c).view T := by
  -- the input, and the input cut at `T`, as graph environments
  let env : HList (Kind.sched (mem := mem)) [Kind.stream coord P inferInstance .noOrder .exactlyOnce] :=
    HList.cons (f := Kind.sched (mem := mem)) cpS HList.nil
  let env' : HList (Kind.sched (mem := mem)) [Kind.stream coord P inferInstance .noOrder .exactlyOnce] :=
    HList.cons (f := Kind.sched (mem := mem)) (fun c => (cpS c).truncate T) HList.nil
  -- they agree up to `T`
  have hagree : HAgree T env env' := by
    refine ⟨fun i t ht => ?_, trivial⟩
    show (cpS i).view t = (cpS i).view (min t T)
    rw [Nat.min_eq_left ht]
  -- so the graph's outputs agree up to `T`
  have hcaus := Dag.denote_causal (pacing := pacing) (two_pc.dag coord part (mem part) vote)
    (two_pc.schedOf pacing coord part (mem part) vote sdec ssched) T env env' hagree
  -- and the graph's outputs are the program's machine runs
  rw [two_pc.dag_denote, two_pc.dag_denote] at hcaus
  exact hcaus.1 c T (Nat.le_refl T)
```

The final theorem rewrites `hp` with `two_pc_truncate` and applies the primed one to the truncated input, with the pool `tpcPoolAt` (the view at `T`):

```lean
  -- move the commit to the run on the cut input
  rw [two_pc_truncate pacing coord part T vote cpS sdec ssched] at hp
  -- and apply the primed theorem there, with the view at `T` as the pool
  exact two_pc_safe_sched' pacing coord part T vote (fun c => (cpS c).truncate T)
    (tpcPoolAt pacing coord T cpS) (tpcPoolAt_above pacing coord T cpS) sdec ssched hn c p hp hone
```

## The Paxos Machine Theorem

`paxos_core` has sixteen content decisions, seven schedule decisions and five cycles, and its transfer uses the same four moves. Three things differ.

**The `requires` is discharged at the call.** `paxos_core`'s clause `variant = .guarded` is the last premise of its `wf` lemma, and the machine theorem passes `rfl` (`Examples/Paxos/CoupleWf.lean`):

```lean
-- the side condition for Paxos's commit output; the last argument `rfl`
-- proves `paxos_core`'s `requires variant = .guarded`
theorem paxos_co_wf :
    (paxos_core (CoupleSem L mem pacing T T (Nat.le_refl T))
        .guarded prop acc config (CoStream.inputC cpS cpV hcp)
        (CoSing.inputC ckS ckSrc ckV hck)
        (paxosCoDec pacing prop acc T sdec)
        (paxosCoSched pacing prop acc T ssched)).2.wf :=
  paxos_core_co_wf₂ .guarded prop acc config (CoStream.inputC cpS cpV hcp)
    (CoSing.inputC ckS ckSrc ckV hck) (paxosCoDec pacing prop acc T sdec)
    (paxosCoSched pacing prop acc T ssched) trivial trivial rfl
```

Everything inside, including the three obligations of each cycle and every callee's `requires`, is closed by the generated lemma. `.guarded` is the Lean variant of `paxos.rs` with the guards for the findings in [Fidelity](https://github.com/hydro-project/hydro/blob/main/hydro_lean/internal/FIDELITY.md#findings-in-the-rust).

**The transfer is two-sided.** Slot agreement compares two commits, so the coupling is taken at two proposers and both memberships are carried into the denotational run:

```lean
  -- both commits are denotational commits, so the contract's slot agreement applies
  exact hens i j s w w'
    (Multiset.mem_of_le hcpl_i (Multiset.mem_coe.mpr hw))
    (Multiset.mem_of_le hcpl_j (Multiset.mem_coe.mpr hw'))
```

### Unconstrained Inputs

`paxos_safe_sched` removes the pool premises as `two_pc_safe_sched` does, so its statement mentions `SchedSem` only:

```lean
-- for any run of the step machine: two commits at one slot, by any two
-- proposers, by step `T`, carry the same value
theorem paxos_safe_sched (T : Nat) (hnA : mem acc ≤ 2 * config.f + 1)
    (cpS : (SchedSem L mem pacing).Stream prop P .totalOrder .exactlyOnce)
    (ckS : CkInput pacing acc) (hck : CkMono pacing acc ckS)
    (sdec : PaxosCoreDec (SchedSem L mem pacing) (mem prop) (mem acc) P (Option Nat) .totalOrder)
    (ssched : PaxosCoreSched (SchedSem L mem pacing) (mem prop) (mem acc) P)
    (i j : Fin (mem prop)) (s : Nat) (w w' : Option P)
    (hw : (s, w) ∈ ((paxos_core (SchedSem L mem pacing) .guarded prop acc config cpS ckS sdec ssched).2 i).view T)
    (hw' : (s, w') ∈ ((paxos_core (SchedSem L mem pacing) .guarded prop acc config cpS ckS sdec ssched).2 j).view T) :
    w = w' := by
```

- `hnA` is the quorum condition: with at most `2f + 1` acceptors, two quorums of `f + 1` intersect.
- `cpS`, the client payloads, is any step history.
- `ckS` is the acceptors' record of the replicas' progress, and `hck : CkMono pacing acc ckS` says it never goes down. The program types this input as monotonic, and Rust's acceptor relies on that to garbage-collect its log ([B4 in Fidelity](https://github.com/hydro-project/hydro/blob/main/hydro_lean/internal/FIDELITY.md#b4-a-fresh-payload-at-a-collected-slot)). The constant `none` satisfies it.
- `sdec` and `ssched` are any machine records: the clock readings, the delivery cursors and the timers' fire scripts.

[Soundness](../../internals/soundness.md#link-3-unconstrained-inputs) covers how the checkpoint pool is built inside the proof.

## From the Machine to the Network

So far the theorems are about the step machine, which has a global clock that every member shares. The deployed program runs as separate processes with no shared clock, and each process reads whatever bytes TCP has delivered. The last step carries the theorems from the step machine to these processes. `paxos_safe_wired` (`Examples/Paxos/WiredSafety.lean`) states slot agreement for the processes: each cluster member runs the compiled function `paxosBody` on its own state, exchanging bytes, under any byte-level trace `τ` of activations and reads. [Wire Codecs and Byte-Level Traces](../../internals/execution/wire-and-traces.md) gives the trace model, and [Soundness](../../internals/soundness.md#the-headline) reads the statement. Its proof walks from the trace down to a `SchedSem` run and applies `paxos_safe_sched`, using four more generated artifacts of `paxos_core`:

| artifact | says |
|---|---|
| `M.dag_valid` | every cycle of the graph passes a network hop or a `defer_tick` |
| `M.wireCodecs` | one byte codec per network node, from the message types' `Wire` instances |
| `M.recordOf`, `M.schedOf_recordOf` | every schedule of the graph is `M.schedOf` of some machine records |
| `M.dag_denote` | `Dag.denote` of the graph equals the program at `SchedSem` |

The proof, with its comments:

```lean
  -- the compiled body equals the interpreter's (`paxosRun.eq`), and the trace's run under it
  -- equals the induced interleaving's (the graph is valid: `paxosDag_valid`)
  rw [paxosBody_eq, τ.streamOutputsAt_eq (paxosDag_valid prop acc config)] at hw hw'
  set I := τ.interleaving (paxosCodecs prop acc config) with hI
  -- the activations that output them, and their outputs as the lock-step step's
  obtain ⟨k, _, hk, hw⟩ := I.mem_streamOutputsAt hw
  obtain ⟨k', _, hk', hw'⟩ := I.mem_streamOutputsAt hw'
  rw [I.loopPass_outputs] at hw hw'
  -- the schedule, as the program's decision and schedule bundles (`recordOf`: every
  -- schedule of the graph is some bundles')
  let sdec := (paxos_core.recordOf I.pacing .guarded prop acc config I.sched).1
  let ssched := (paxos_core.recordOf I.pacing .guarded prop acc config I.sched).2
  have hS : paxos_core.schedOf I.pacing .guarded prop acc config sdec ssched = I.sched :=
    paxos_core.schedOf_recordOf I.pacing .guarded prop acc config I.sched
  …
    rw [← hS, paxos_core.dag_denote I.pacing .guarded prop acc config sdec ssched env, envRestrict_get]
  …
  exact paxos_safe_sched I.pacing prop acc config (max k k') hnA cpS ckS hckS sdec ssched _ _ s w w' hw1 hw2
```

1. `paxosBody_eq` replaces the compiled body by the graph interpreter, and `WireTrace.streamOutputsAt_eq` replaces the byte trace by the **interleaving** it induces (which member runs at each step, with which inputs and delivery cursors). Both are framework theorems; the second needs `dag_valid`.
2. `mem_streamOutputsAt` finds the activation `k` that output each commit, and `loopPass_outputs` states its outputs as one step of the graph.
3. `recordOf` reads the interleaving's schedule back as `paxos_core`'s decision and schedule records, and `schedOf_recordOf` says that loses nothing.
4. `dag_denote` turns the graph's outputs into `paxos_core` at `SchedSem`, and `paxos_safe_sched` closes the goal at horizon `max k k'`.

For another program the proof is the same with its own graph, codecs and machine theorem.

## Guarantees and Limits

| theorem | quantified over | premises |
|---|---|---|
| `two_pc_safe_sched` | pacing, proposals, cursors, horizon | `1 ≤ mem part`, proposed once |
| `cq_safe_sched` | pacing, response history, horizon | none |
| `paxos_safe_sched` | pacing, client stream, checkpoint record, decisions and schedules, horizon | `hnA`, `CkMono` |
| `paxos_safe_wired` | byte-level traces | `hnA`, the checkpoint never decreases |

The direction is machine run ⊆ denotational run. A fact the contract states about every element of the output transfers; a fact of the form "the output contains x" does not, because the machine at `T` may not have produced it yet. [Liveness](../../internals/liveness.md) covers facts that need the other direction, under fairness.

A theorem whose premises no run satisfies is true and says nothing, so every main theorem has a witness that runs the program itself: `#guard` lines apply each module at `Values` to concrete decision records, `lake exe paxos` and `lake exe twopc` run the programs to a commit, and `lake exe wired` runs `paxosBody` as five processes. In the other direction, `lake exe falsify` shows that `PaxosVariant.faithful`, the Rust as written, violates slot agreement on concrete decision records, so the premises of the Paxos theorems do not exclude the runs that go wrong.
