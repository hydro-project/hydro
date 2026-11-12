---
sidebar_position: 2
title: "Generated Artifacts"
---

# Generated Artifacts

`hydro def M` adds dozens of declarations to the environment: the plain program and its face, the ghost facts as theorems, the coupling lemmas that carry the face from the denotation to the step machine, monotonicity and causality, the knot stack of every hoisted cycle, and the module compiled to a graph. This page catalogues them: each group with its statement for a small module, the phase that produces it, and what consumes it. Your proofs cite only a few of them ([Transferring to the Machine](../../using/proving/transferring-to-the-machine.md#the-lemmas-you-cite)); the rest exist for the generator's own phases and for callers' generated lemmas.

The statements are `#check` output for the toy suite in `Hydro/HydroGenToy.lean`, which exercises every artifact class: `toy_relay`, a knot-free module with a `tick` block, two `nondet!` batch sites, an interval timer and an `ensures` face; and `toy_loop`, whose `fix` knot closes a body that calls `toy_relay` through `toy_step`. Three interpretations appear in the statements: `Values L mem`, the denotational semantics; `SchedSem L mem pacing`, the step machine; and `CoupleSem L mem pacing Tc Td hjT`, the coupling, which runs both up to horizons `Tc ≤ Td` ([The Coupling Corner](../corner/coupling.md)). A **knot** `K = <M>.<wire>` is a hoisted `fix` cycle with its **chain module** `<M>.body` ([Elaborating `hydro def`](./elaborating-hydro-def.md#hoisting-the-knots)). An **output** is one component of the module's output tuple, and the subscript `ᵢ` counts outputs from 1.

## Overview

| group | names | question it answers | who cites it |
|---|---|---|---|
| module and face | `M._spec`, `M`, `M.ensures` | what the program computes, and what it guarantees at `Values` | callers, headline theorems |
| facts you proved | `M.ghost.<h>`, `M.pre.<h>`, `K.inv`, `K.inv_stages` | the program's ghost facts, as closed theorems | replays, `hydro_pre_close` |
| coupling | `M_co_srᵢ`, `M_co_rr_ex`, `M_vdec`, `M_co_rrᵢ`, `M_co_lagᵢ`, `M_co_srcᵢ`, `M_co_wfᵢ` | how a coupled run's `sr` and `rr` relate to the plain runs | machine theorems, callers' generated lemmas |
| order | `M_smonoᵢ`, `M_monoᵢ`, `K_strict₁` | tick-monotonicity on the machine, monotonicity at `Values`, causality of a knot | callers' generated lemmas, knot proofs |
| knot stack | `K_co_sr₁`, `K_co_rr₁`, `K_co_lag₁`, `K_co_wf₁`, `K.stages*`, `<M>.body_vdec` | the same questions for a hoisted cycle | the enclosing module's lemmas |
| graph | `M.dag`, `M.schedOf`, `M.recordOf`, `M.schedOf_recordOf`, `M.dag_denote`, `M.dag_valid`, `M.dag_agreeL`, `M.wireCodecs` | the executable form, and that it computes the machine run | execution and wiring theorems |

For example, `paxos_core_co_wf₂` is the coupling group's residual lemma for `paxos_core`'s second output, `p_to_replicas`.

## The Module and Its Face

`hydro def M … : τ ensures out => P := …` elaborates as `M._spec`, a function returning the value paired with its proof:

```lean
@toy_relay._spec : {L : Type} →
  {mem : L → ℕ} →
    (H : HydroSem L mem) →
      (ℓ : L) →
        (inp : HydroSem.Stream mem ℓ ℕ StrOrd.noOrder Retries.exactlyOnce) →
          (dec : ToyRelayDec H (mem ℓ)) →
            (sched : ToyRelaySched H (mem ℓ)) →
              { out //
                ∀ (hv : H = Values L mem),
                  match H, hv, inp, dec, sched, out with
                  | .(Values L mem), ⋯, inp, dec, sched, out => ToyEnsures out }
```

From that `hydro def` derives the program `M`, the same value with the proof removed, and the face theorem at `Values`:

```lean
@toy_relay.ensures : ∀ {L : Type} {mem : L → ℕ} (ℓ : L)
  (inp : HydroSem.Stream mem ℓ ℕ StrOrd.noOrder Retries.exactlyOnce) (dec : ToyRelayDec (Values L mem) (mem ℓ))
  (sched : ToyRelaySched (Values L mem) (mem ℓ)), ToyEnsures (toy_relay (Values L mem) ℓ inp dec sched)
```

Callers use `M H …` and `M.ensures …`. `_spec` exists only to produce the other two. `M` is a non-dependent function because a result type `{out // … args …}` would fix the arguments in place, and `simp` could not rewrite under a callee application, so none of the generated rewrite lemmas would fire there (`deriveModule`, `Hydro/HydroDef.lean`).

## Ghost Facts as Theorems

Each hoisted ghost fact is a closed declaration that you can `#check` and cite outside the module:

- `M.ghost.<h>` for each `ghost have h`, including the tick readers `h<out>_run`, `h<out>_at`, `h<out>_reg`, `h<out>_inv` and `h<out>_inv_take` ([Tick Blocks](../../using/proving/tick-invariants.md#the-readers)), for example `toy_tick2.ghost.houtv_run`. A `ghost have` that follows a `ghost witness`, `ghost intro` or `ghost subst` is replayed as a tactic and has no declaration, and neither do `ghost obtain` and `ghost let` ([When a ghost is checked](./elaborating-hydro-def.md#the-ghost-layer)).
- `M.pre.<h> : ∀ binders, let wires…; PreFact R S` for each `ghost pre h`, for example `toy_pre.pre.hpre` ([Proofs over Values](../../using/proving/proofs-over-values.md#requires-and-ghost-pre)).
- `<M>.<w>.inv` and `<M>.<w>.inv_stages` for a `fix` `invariant` clause, for example `toy_loop.w.inv` ([Knots](../../using/proving/knot-invariants.md#the-invariant-clause)).

## The Coupling Lemmas

A `CoupleSem` value is a **corner**. It pairs the machine run `sr` with the denotational run `rr`, a **lag** (how many steps below the horizon the coupling holds), a residual proposition `wf`, and

```lean
  cpl : wf → ∀ k, k + lag ≤ T → ∀ i, ListLe ord ret ((sr i).view k) (rr i)
```

for a stream. Under `wf`, the machine's view at every step `k` with `k + lag ≤ T` lies below the denotation in the grade's order (`Hydro/Couple.lean`). In printed statements, `.4` is the stream corner's `wf` field (fields are `sr`, `rr`, `lag`, `wf`), printed as a raw projection; a singleton's is `.5`. The coupling lemmas turn this into statements about the plain runs. The `sr` equation and the `rr` equation each state that one of the two runs of a coupled output equals a plain run at another interpretation.

**The `sr` equation.** The corner's machine run equals the program's run on the machine. The **content decisions** (batch cuts, snapshot cuts and order selections, the choices the denotational semantics takes as arguments) become `()`, because at the machine a batch is whatever arrived:

```lean
@toy_relay_co_sr₁ : … (dec : ToyRelayDec (CoupleSem L mem pacing Tc Td hjT) (mem ℓ))
  (sched : ToyRelaySched (CoupleSem L mem pacing Tc Td hjT) (mem ℓ)),
  (toy_relay (CoupleSem L mem pacing Tc Td hjT) ℓ inp dec sched).1.sr =
    (toy_relay (SchedSem L mem pacing) ℓ inp.sr { batch := (), gate := () } { interval := sched.interval }).1
```

**Derived decision and the `rr` equation.** The corner's denotational run equals the program's run at `Values`, at a decision record computed from the machine run's inputs. That record is `M_vdec`:

```lean
@toy_relay_vdec : {L : Type} →
  {mem : L → ℕ} →
    {pacing : Pacing L mem} →
      {Td : ℕ} →
        (ℓ : L) →
          HydroSem.Stream mem ℓ ℕ StrOrd.noOrder Retries.exactlyOnce →
            HydroSem.IntervalDec mem (mem ℓ) → ToyRelayDec (Values L mem) (mem ℓ)
@toy_relay_co_rr₁ : … (dec : ToyRelayDec (CoupleSem L mem pacing Tc Td hjT) (mem ℓ))
  (sched : ToyRelaySched (CoupleSem L mem pacing Tc Td hjT) (mem ℓ)),
  (toy_relay (CoupleSem L mem pacing Tc Td hjT) ℓ inp dec sched).1.rr =
    (toy_relay (Values L mem) ℓ inp.rr (toy_relay_vdec ℓ inp.sr sched.interval) { interval := () }).1
```

`vdec` takes the machine input `inp.sr` and the machine's timer schedule, so the denotational run uses the decisions this machine run realized, not a quantified "some decision"; [The Satisfiability Premise](../corner/coupling.md#the-satisfiability-premise) explains why it is derived. How `vdec` is obtained depends on the module's route through the generator ([Finding an artifact](#finding-an-artifact)). For a knot-free module with a content decision, the generator defines it by `Classical.choose` from the existential `M_co_rr_ex`, and `M_co_rrᵢ` is that existential's specification at output `i`; the definition of `M_vdec` by `Classical.choose` is where `Classical.choice` enters the generated lemmas (`#print axioms` in `Hydro/HydroGenCheck.lean`). For a **glue module**, one the generator routes to its glue phase because some callee reaches a knot (the callee is a knot, or calls one directly or through its own callees; here `toy_loop`, whose callee is the knot `toy_loop.w`), the generator reads `vdec` off the callees' own `rr` equations. A knot uses its chain module's `<M>.body_vdec` (below).

**Lag.** `M_co_lagᵢ` writes the output's lag in terms of the inputs' lags. `toy_relay` passes it through: `(toy_relay … ).1.lag = inp.lag`. A network hop or `defer_tick` subtracts one. At `Values` a hop is an identity operator ([The Network as Identity](../semantics/values.md#the-network-as-identity)), so the lag measures only the step machine's one-step delay.

**Well-formedness.** `M_co_wfᵢ` threads `wf` from the inputs to output `i`. If the module has `requires R`, `R` at the inputs' denotational runs is the last hypothesis:

```lean
@toy_relay_co_wf₁ : … (dec : ToyRelayDec (CoupleSem L mem pacing Tc Td hjT) (mem ℓ))
  (sched : ToyRelaySched (CoupleSem L mem pacing Tc Td hjT) (mem ℓ)),
  inp.4 → (toy_relay (CoupleSem L mem pacing Tc Td hjT) ℓ inp dec sched).1.wf
```

**Sources.** `M_co_srcᵢ` exists for singleton outputs. It identifies the machine stream that a consumer's `snapshot` derives its own decision from.

The statements above omit the leading binders `{L} {mem} {pacing} {Tc Td} {hjT} (ℓ) (inp)`.

## Monotonicity and Causality

`M_monoᵢ` is monotonicity at `Values`: larger inputs give a larger output, at the same decisions. It is proved by instantiating the program at `MonoRel`, the interpretation whose values are pairs of related `Values` runs:

```lean
@toy_relay_mono₁ : ∀ {L : Type} {mem : L → ℕ} (ℓ : L)
  {inp inp' : HydroSem.Stream mem ℓ ℕ StrOrd.noOrder Retries.exactlyOnce} (dec : ToyRelayDec (Values L mem) (mem ℓ))
  (sched : ToyRelaySched (Values L mem) (mem ℓ)),
  (∀ (i : Fin (mem ℓ)), PoolLe StrOrd.noOrder Retries.exactlyOnce (MemberMap.get inp i) (MemberMap.get inp' i)) →
    ∀ (i : Fin (mem ℓ)),
      PoolLe StrOrd.noOrder Retries.exactlyOnce (MemberMap.get (toy_relay (Values L mem) ℓ inp dec sched).1 i)
        (MemberMap.get (toy_relay (Values L mem) ℓ inp' dec sched).1 i)
```

You cite these inside knot proofs: `toy_loop`'s invariant is two applications of `toy_step_mono₁`, and the Paxos step lifts a stage's max-ballot wire with `leader_election_mono₄`. `M_smonoᵢ` is tick-monotonicity of the machine run; a knot's `wf` needs it for tick wires. For stream outputs it is trivial (`TMonoTriv`). `K_strict₁` is a knot's causality. Inputs that agree up to step `h`, and Bekić parameters (the later wires of a multi-wire `fix` block; see [Knots](../../using/proving/knot-invariants.md)) that agree strictly below `h`, give knot outputs that agree up to `h`:

```lean
@toy_loop.w_strict₁ : ∀ {L : Type} {mem : L → ℕ} {pacing : Pacing L mem} {h : ℕ} (ℓ : L)
  {inp inp' : HydroSem.Stream mem ℓ ℕ StrOrd.noOrder Retries.exactlyOnce}
  (tdec : ToyLoopDec (SchedSem L mem pacing) (mem ℓ)) (sched : ToyStepSched (SchedSem L mem pacing) (mem ℓ)),
  SAgree h inp inp' →
    SAgreeL h 0 (toy_loop.w (SchedSem L mem pacing) ℓ inp tdec sched)
      (toy_loop.w (SchedSem L mem pacing) ℓ inp' tdec sched)
```

## The Knot Stack

Each hoisted knot `K` answers the same questions with its own lemmas. The difference is in the `rr` equation: the body's derived decision `<M>.body_vdec` takes the machine knot itself as its loop-position argument:

```lean
@toy_loop.w_co_rr₁ : … ,
  (toy_loop.w (CoupleSem L mem pacing Tc Td hjT) ℓ inp tdec sched).rr =
    toy_loop.w (Values L mem) ℓ inp.rr
      (toy_loop.body_vdec ℓ inp.sr sched.relay.interval sched.net
        (toy_loop.w (SchedSem L mem pacing) ℓ inp.sr { relay := { batch := (), gate := () }, fuel := () }
          { relay := { interval := sched.relay.interval }, net := sched.net }))
      { relay := { interval := () }, net := () }
```

(shortened binders). `K_co_wf₁` is assembled from the knot's strictness, its Kleene chain and the graded coupling of its body. Its statement has the plain shape (`inp.wf → (toy_loop.w (CoupleSem …) …).wf`), so the enclosing module consumes it like any callee's. The stage vocabulary `K.stages`, `_zero`, `_succ`, `_fix` and `_mono` is covered on [Knots](../../using/proving/knot-invariants.md#the-stage-vocabulary). [The Knot Stack](./knot-stack.md) explains how each lemma is proved without unfolding through the knot.

## The Graph

Every module except a knot is also compiled to a typed graph, `M.dag`. A `let` becomes a node, a callee becomes `call C.dag`, and a `fix` block becomes `fixN` over `call <M>.body.dag`. The non-wire binders it keeps are the ones the graph mentions. Shortened:

```lean
fun {L} {mem} ℓ =>
  Project.Dag.letN (Project.Node.map instDecidableEqNat instDecidableEqNat Project.Var.zero fun _i n => n + 1)
    (Project.Dag.letN (Project.Node.batch instDecidableEqNat StrOrd.noOrder Retries.exactlyOnce Project.Var.zero ())
      (Project.Dag.letN (Project.Node.source_interval (fun x => 0) 1)
      …
```

`M.schedOf` repackages the module's machine decisions and schedule into the graph's own `Dag.Sched` shape, and `M.recordOf` inverts it. `M.schedOf_recordOf` states that they round-trip, by kernel `rfl`. The central theorem is `M.dag_denote`: the graph's reading `Dag.denote` equals the module's machine run, for every pacing.

```lean
@toy_relay.dag_denote : ∀ {L : Type} {mem : L → ℕ} (pacing : Pacing L mem) (ℓ : L)
  (dec : ToyRelayDec (SchedSem L mem pacing) (mem ℓ)) (sched : ToyRelaySched (SchedSem L mem pacing) (mem ℓ))
  (env :
    Project.HList Project.Kind.sched [Project.Kind.stream ℓ ℕ instDecidableEqNat StrOrd.noOrder Retries.exactlyOnce]),
  Project.Dag.denote pacing (toy_relay.dag ℓ) (toy_relay.schedOf pacing ℓ dec sched) env =
    Project.HList.cons (toy_relay (SchedSem L mem pacing) ℓ (env.get Project.Var.zero) dec sched).1
      (Project.HList.cons (toy_relay (SchedSem L mem pacing) ℓ (env.get Project.Var.zero) dec sched).2
        Project.HList.nil)
```

`M.dag_valid : (M.dag …).valid = true` states that every cycle in the graph passes a network hop or a `defer_tick`. `M.dag_agreeL` is the graph's lag law: inputs agreeing up to their lags give outputs agreeing up to the lags the analysis assigns. `M.wireCodecs` gives one byte codec per network node. It is skipped, with a note, when a payload type has no `Wire` instance. [Wire Codecs and Byte-Level Traces](../execution/wire-and-traces.md) consumes these. `cq_safe_sched` uses `collect_quorum.dag_denote` together with `Dag.denote_causal` to drop a premise from the machine theorem.

## An End-to-End Consumer

`toy_safe_sched` (`Hydro/HydroGenCheck.lean`) is the smallest theorem built from these artifacts: every value the machine's `toy_relay` emits is positive, at any pacing, schedule and horizon.

```lean
theorem toy_safe_sched
    {L : Type} {mem : L → Nat}
    {pacing : Pacing L mem} (ℓ : L)
    (h : (SchedSem L mem pacing).Stream ℓ Nat .noOrder .exactlyOnce)
    (v : (Values L mem).Stream ℓ Nat .noOrder .exactlyOnce)
    (hc : ∀ t i, ListLe .noOrder .exactlyOnce ((h i).view t) (v i))
    (fires : (SchedSem L mem pacing).IntervalDec (mem ℓ))
    (T : Nat) (i : Fin (mem ℓ)) (x : Nat)
    (hx : x ∈ ((toy_relay (SchedSem L mem pacing) ℓ h ⟨(), ()⟩ ⟨fires⟩).1 i).view T) :
    1 ≤ x := by
```

Its proof cites `toy_relay_co_wf₁` and the corner's `cpl` (the machine view lies below the denotation), `toy_relay_co_lag₁` (the lag is 0), `toy_relay_co_sr₁` and `toy_relay_co_rr₁` (rewrite `sr` and `rr` to plain runs), and `toy_relay.ensures` at `toy_relay_vdec` (the contract at the derived decision). Those are the four moves of every machine theorem, developed for `collect_quorum` and Paxos in [The Four Moves](../../using/proving/transferring-to-the-machine.md#the-four-moves).

## Finding an Artifact

The names are stable and greppable, and `#check @toy_relay_co_wf₁` prints any of them. Which coupling lemmas a module has depends on the route `runPipeline` (`Hydro/HydroDef.lean`) sends it down, decided from what the module's body applies:

| route | modules | coupling lemmas |
|---|---|---|
| knot | the hoisted knots `K = <M>.<wire>` | `K_co_sr₁`, `K_co_rr₁` at `<M>.body_vdec`, `K_co_lag₁`, `K_co_wf₁`, `K.stages*` |
| glue | modules with a callee that reaches a knot (`toy_loop`, `paxos_core`) | `M_vdec` read off the callees' `rr` equations, `M_co_rrᵢ` |
| couple | every other module | with a content decision, `M_co_rr_ex`, `M_vdec` and `M_co_rrᵢ`; without one, `M_co_rrᵢ` alone |

Every module gets `M_co_srᵢ` and `M_co_lagᵢ` (a knot its `K_…₁` forms), and the glue and couple routes both continue with step monotonicity, `wf` and monotonicity. Every route ends in the graph phase, except that a knot has no graph of its own: its chain module `<M>.body` does. Setting `HYDRO_PHASE_DIAG` prints each phase's wall time for a module. [Generator Overview](./overview.md#routing) explains why each route exists.
