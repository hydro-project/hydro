---
sidebar_position: 7
---

# The Knot Stack

A **knot** is a cycle in the program, Rust's `forward_ref`/`complete_cycle`, written `fix (w : τ) via fuel := body` in a `hydro def`; its **knot wire** `w` is in scope in its own body. At `Values` a knot is a Kleene iteration of the body from the empty value, run for `fuel` rounds. At `SchedSem` the knot wire at a step equals the body's output at the same step (`fix_stream_eq`), and this fixpoint equation holds because the body is **strict** in `w`: every path from `w` back to the body's output passes a network hop or a `defer_tick` ([The Ungated Knot](../semantics/sched.md#the-ungated-knot)).

A module that calls a knot consumes the same artifacts from it as from any other callee: the `sr` equation (the corner run's `.sr` equals the knot run at `SchedSem`), the `rr` equation (the corner run's `.rr` equals the knot run at `Values` at a derived decision record), the lag equation, a residual lemma and monotonicity lemmas. The knot stack produces them, and the difficulty is that the generator cannot prove any of them the way it proves a knot-free module's. Definitional equality through nested knots is exponential in the nesting depth: one `rfl` on `leader_election`'s body takes milliseconds with up to two knots, about 4 minutes with its real three, and ran out of budget after more than 80 minutes with five, under `paxos_core` ([the full measurements](../cost-model.md#definitional-equality-through-knots)). Each outer knot's body re-closes the inner knots (the sequential Bekić closure of what is semantically one mutual fixpoint), and kernel substitution does not preserve sharing, so the program text multiplies once per nesting level.

The knot stack (`runKnotStackT` in `hydro_lean/Hydro/HydroGenKnot.lean`) therefore works one knot at a time, and never crosses a knot by definitional equality. The knot and its body are separate top-level `def`s, so terms carry the two constants and not their values; the stack uses every inner knot folded, through its own generated lemmas; every judgement left to definitional equality is either record-sized or a single delta step, and the rest is rewriting. The residual needs an induction over the knot's stages. The instance proves that induction once (`co_fix_cpl`), so the generator only discharges its premises for a body it treats as opaque. `runPipeline` (`hydro_lean/Hydro/HydroDef.lean`) sends a module here exactly when its value is `H.fix …`, which after hoisting means a knot constant `<M>.<w>`; the module around the knot is a glue module ([Routing](./overview.md#routing)).

## The Hoisted Shape

The `fix` elaborator ([Elaborating `hydro def`](./elaborating-hydro-def.md)) hoists each component of a `fix` block to a top-level knot `<M>.<w>` whose body is a separate constant `<M>.<w>.body`, applied to a tuple of captured values. The body itself is a projection of a registered **chain module** `<M>.body`, the hoisted text of the `fix` block, which goes through the ordinary pipeline first. For `toy_loop` (`hydro_lean/Hydro/HydroGenToy.lean`):

```lean
def Hydro.toy_loop.w : {L : Type} →
  {mem : L → ℕ} →
    (H : HydroSem L mem) →
      (ℓ : L) →
        HydroSem.Stream mem ℓ ℕ StrOrd.noOrder Retries.exactlyOnce →
          ToyLoopDec H (mem ℓ) → ToyStepSched H (mem ℓ) → HydroSem.Stream mem ℓ ℕ StrOrd.noOrder Retries.exactlyOnce :=
fun {L} {mem} H ℓ inp tdec sched => H.fix tdec.fuel (inp, tdec, sched) (toy_loop.w.body ℓ)
```

```lean
@[reducible] def Hydro.toy_loop.w.body : {L : Type} →
  {mem : L → ℕ} →
    (ℓ : L) →
      (H'' : HydroSem L mem) →
        HydroSem.Stream mem ℓ ℕ StrOrd.noOrder Retries.exactlyOnce × ToyLoopDec H'' (mem ℓ) × ToyStepSched H'' (mem ℓ) →
          HydroSem.Stream mem ℓ ℕ StrOrd.noOrder Retries.exactlyOnce →
            HydroSem.Stream mem ℓ ℕ StrOrd.noOrder Retries.exactlyOnce :=
fun {L} {mem} ℓ H'' caps l => (toy_loop.body H'' ℓ caps.1 caps.2.1 caps.2.2 l).1
```

The body is generic in its interpretation `H''` (the `HydroSem` instance it is run at), because the knot's `sr` and `rr` equations relate the same body at three interpretations: the corner, `SchedSem` and `Values`. The corner's own `fix` runs the body on **probes**: corner values built from a machine value alone (`schedC x`, with a dummy denotational run) or from a machine and a denotation value (`probeC x v`), with no coupling claimed (`wf := False`). It is a constant because the knot laws are stated over an opaque `body`, and an unfolded body under the `fix` makes every rewrite carry it. It is `@[reducible]` so that the generic `fix` laws see through it to the chain module's output (`(toy_loop.body …).1`), and the stack rewrites with that output's generated equations.

Bekić's rule decomposes a block of `n` wires into `n` nested single-wire knots (`closeWire`, shown on [Elaborating `hydro def`](./elaborating-hydro-def.md#the-bekić-decomposition)): component `i`'s body substitutes each earlier wire `j < i` by `j`'s knot, re-closed at the current loop variable and the later wires. A component's trailing wire binders for the later wires of its block are its **Bekić parameters**. In `paxos_core`, `paxos_core.a_log` takes `sequencing_max_ballots` as a parameter, and the strictness lemma below assumes its parameters at lag `1`.

The knot stack runs for each hoisted knot, innermost first and inside the `fix` elaboration, so the proofs inside the enclosing `hydro def` (its `ghost` clauses and the proofs of its `prove` fields; see [Proofs over Values](../../using/proving/proofs-over-values.md)) can cite the generated vocabulary. The order is `sr`, `lag`, `rr`, `strict`, `smono`, `mono`, `stages`, the `invariant` hook, `wf`.

| Artifact | Statement (for `K` = `toy_loop.w` unless noted) | Proof |
|---|---|---|
| `K_co_sr₁` | `(K corner args).sr = K sched (machine args)` | `co_knot_sr` script |
| `K_co_lag₁` | `(K corner args).lag = inp.lag` | `knotLagOpen`, the body's lag equation, `lagCanon` |
| `K_co_rr₁` | `(K corner args).rr = K values (…, body_vdec … (K sched …), …)` | `co_knot_rr₂` script |
| `K_strict₁` | `SAgree h inp inp' → SAgreeL h 0 (K sched inp …) (K sched inp' …)` | direct term: `causalL_fix_stream` over the chain's graph lag analysis |
| `K_smono₁`, `K_mono₁` | tick-monotonicity of `K sched`, `Values` monotonicity of `K values` | `tmono_fix_tick`; `iterate_mono_param` over the body's `_mono` |
| `K.stages`, `_zero`, `_succ`, `_fix`, `_mono` | the Kleene chain at `Values` | definitions and `rfl`; `iterate_chain` |
| `K.inv`, `K.inv_stages` | the `invariant` clause at the closed knot and at the stages | `iterate_invariant` (the `fix` elaborator's hook) |
| `K_co_wf₁` | `inp.wf → (K corner args).wf` | seven installed facts and a fixed script |

## The `sr` Equation

`K_co_sr₁` identifies the machine run `.sr` of the corner run with the knot run at `SchedSem`:

```lean
@toy_loop.w_co_sr₁ : ∀ {L : Type} {mem : L → ℕ} {pacing : Pacing L mem} {Tc Td : ℕ} {hjT : Tc.le Td} (ℓ : L)
  (inp : HydroSem.Stream mem ℓ ℕ StrOrd.noOrder Retries.exactlyOnce)
  (tdec : ToyLoopDec (CoupleSem L mem pacing Tc Td hjT) (mem ℓ))
  (sched : ToyStepSched (CoupleSem L mem pacing Tc Td hjT) (mem ℓ)),
  (toy_loop.w (CoupleSem L mem pacing Tc Td hjT) ℓ inp tdec sched).sr =
    toy_loop.w (SchedSem L mem pacing) ℓ inp.sr { relay := { batch := (), gate := () }, fuel := () }
      { relay := { interval := sched.relay.interval }, net := sched.net }
```

`genKnotSr` proves it with one tactic call, `(intros; co_knot_sr [{defs}] [{rules}])`. The macro (`hydro_lean/Hydro/KnotTactics.lean`) is listed and explained line by line on [Projections and Tactics](../corner/projections-and-tactics.md#rewriting-through-knots). The generator supplies the macro's two lists, which make up the whole per-knot content of the proof:

- `defs` are the constants to unfold: the knot and the wrappers opened while parsing its body (`toy_loop.w`, `toy_loop.w.body`). The macro unfolds only these; the chain module `toy_loop.body` stays folded.
- `rules` are the chain module's `sr` equation (`toy_loop.body_co_sr₁`) and the `K_co_sr₁` of every inner knot that occurs in the body's arguments.

After `unfold`, both sides are `H.fix` over the same body at two interpretations. The macro turns the corner side into the **machine diagonal**, the machine knot spelled through the body (`fix_stream () (fun x => (body _ caps (CoStream.schedC x)).sr)`, where `schedC x` embeds a machine value as a corner value with a dummy denotational run), splits the two `fix_stream` applications, closes the fuel arguments by a record-sized `rfl`, and leaves, after `funext x`, the body-level goal `(body corner (schedC x)).sr = body sched x`. The rewrite loop `co_body_rw [rules]` closes it: the body's `sr` equation rewrites the chain module, and each inner knot's `sr` equation rewrites one occurrence of that knot, so a knot nested `k` deep costs `k` rule applications, not an unfolding through `k` levels.

## The Lag Equation

The **lag** of a corner value is how far below the **couple horizon** `Tc` (the step up to which the corner claims its coupling) the corner guarantees the coupling. A program wire has lag `0`, a knot's loop input lag `1`, and a network hop or `defer_tick` subtracts one. A knot's lag is `CoStream.fixLag body`, the body's output lag on a lag probe carrying lag `1`, an `@[irreducible]` definition (`Hydro/Couple.lean`) so that horizon comparisons `j + lag ≤ T` never evaluate the body ([Lag in knots](../corner/coupling.md#lag-in-knots)).

`genKnotLag` opens it with the generic lemmas `co_hfix_stream_lag` and `co_hfix_stream_fixLag` (`hydro_lean/Hydro/CoupleProj.lean`), which take the fix's `df`, `caps` and `body` as arguments, so the only judgements left to the kernel are deltas of `K` and its body meeting the same probe term on both sides. The stack never asks the kernel to compare `.lag` of two differently headed module applications, because the kernel decides that comparison by evaluating the chain, which does not share and exhausts memory. The opened lag is then the right side of the body's lag equation at the probe, with the inner knots' lag equations substituted (`lagEqOf`, by `kabstract` and `congrArg`) and normalized.

For `toy_loop.w` the body is `union (values (broadcast_closed net (toy_relay s …).1)) inp`. The lag projections give `max (s.lag - 1) inp.lag`; at the probe `s.lag = 1`, so the lag equation is `(toy_loop.w (CoupleSem …) …).lag = inp.lag`.

The normal form (`lagCanon`) is a sorted, right-nested `max` with one leaf `a - k` per lag atom (its least `k`) and the largest numeral. Its size is bounded by the number of atoms, while the raw tree grows multiplicatively through nested knots. `lagCanon` proves the equation `E = C` by `Nat.le_antisymm` from leaf-wise `max_le` splits and `le_max` paths, a term of size `|E| · |C|` with no search and no `omega` (`omega` is exponential on nested `max`es).

## The `rr` Equation

`K_co_rr₁` identifies the denotational run `.rr` of the corner run with the knot run at `Values` at a derived decision record:

```lean
@toy_loop.w_co_rr₁ : ∀ {L : Type} {mem : L → ℕ} {pacing : Pacing L mem} {Tc Td : ℕ} {hjT : Tc.le Td} (ℓ : L)
  (inp : HydroSem.Stream mem ℓ ℕ StrOrd.noOrder Retries.exactlyOnce)
  (tdec : ToyLoopDec (CoupleSem L mem pacing Tc Td hjT) (mem ℓ))
  (sched : ToyStepSched (CoupleSem L mem pacing Tc Td hjT) (mem ℓ)),
  (toy_loop.w (CoupleSem L mem pacing Tc Td hjT) ℓ inp tdec sched).rr =
    toy_loop.w (Values L mem) ℓ inp.rr
      (toy_loop.body_vdec ℓ inp.sr sched.relay.interval sched.net
        (toy_loop.w (SchedSem L mem pacing) ℓ inp.sr { relay := { batch := (), gate := () }, fuel := () }
          { relay := { interval := sched.relay.interval }, net := sched.net }))
      { relay := { interval := () }, net := () }
```

The decision record is the body module's `body_vdec`, which the generator defines by `Classical.choose` ([The Couple Phase](./couple-phase.md#defining-m_vdec-by-choice)), with the loop wire's machine data set to the machine knot `toy_loop.w (SchedSem …) …`. The corner's semantics fixes this record: its `Values` body reads the machine context of every decision site inside the loop from the knot's own machine diagonal (`CoStream.vbody body v := (body (probe2 (fixSr body) v)).rr`), so every Kleene stage, and every occurrence of the same site outside the knot, derives the same decision.

The script is `(co_knot_rr₂ HSR [{defs}] [{pinList}] [{rules}])`, run with installed facts: `HSR` is `K_co_sr₁` fully applied, and `PIN1 …` are the body module's fuel pins `body_vdec_fuelᵢ` instantiated at the knot's arguments.

```lean
  `(tactic| (
    have h := $hsr
    unfold $[$ds]* at h ⊢
    first
      | rw [co_hfix_stream_sr] at h
      | rw [co_hfix_tick_sr] at h
    first
      | refine Eq.trans ?_ (values_hfix_stream' _ _ _).symm
      | refine Eq.trans ?_ (values_hfix_tick' _ _ _).symm
    first
      | rw [co_hfix_stream_rr]
      | rw [co_hfix_tick_rr]
    refine congrArg₂ _ ?_ ?_
    · $fuelTac:tactic
    funext v
    rw [h]
    co_body_rw [$rules,*]
    try simp only [co_schedC_sr, co_tick_schedC_sr]
    try with_reducible rfl
    try exact rfl))
```

The script follows `co_knot_sr` on the `Values` side and differs from it in two places. The corner's denotational run is `fix_stream (Td + 1) (fun v => (body _ caps (probeC (fixSrC caps body) v)).rr)`, so after `funext v` the goal mentions the machine diagonal `fixSrC caps body`, and `rw [h]` replaces the diagonal by the machine knot `K sched …`. This `rw` is where the knot's `sr` equation enters the proof of its `rr` equation. And `rfl` does not close the fuel field. The target's fuel is the record field `(body_vdec …).fuel`, a projection of a `Classical.choose` term, which is propositionally but not definitionally `Td + 1`. `fuelTac` tries `with_reducible rfl`, `rfl`, then `exact Eq.symm PINᵢ` for each pin.

## Strictness

The machine knot is causal in its external inputs and strict in its Bekić parameters:

```lean
@toy_loop.w_strict₁ : ∀ {L : Type} {mem : L → ℕ} {pacing : Pacing L mem} {h : ℕ} (ℓ : L)
  {inp inp' : HydroSem.Stream mem ℓ ℕ StrOrd.noOrder Retries.exactlyOnce}
  (tdec : ToyLoopDec (SchedSem L mem pacing) (mem ℓ)) (sched : ToyStepSched (SchedSem L mem pacing) (mem ℓ)),
  SAgree h inp inp' →
    SAgreeL h 0 (toy_loop.w (SchedSem L mem pacing) ℓ inp tdec sched)
      (toy_loop.w (SchedSem L mem pacing) ℓ inp' tdec sched)
```

`SAgree h x y` says two machine streams agree at every step up to `h`, and `SAgreeL h n` that they agree up to `h - n`. In `paxos_core.a_log_strict₁` the Bekić parameter `sequencing_max_ballots` is assumed at `SAgreeL h 1`, one step behind.

The proof is a direct `Expr` term, `causalL_fix_stream` (`hydro_lean/Hydro/SchedCausal.lean`) applied to the body function on each side:

```lean
theorem causalL_fix_stream {ℓ : L} {α : Type} [DecidableEq α] {ord : StrOrd} {ret : Retries}
    {h Lg : Nat} (d : Unit)
    {body body' : (SchedSem L mem pacing).Stream ℓ α ord ret → (SchedSem L mem pacing).Stream ℓ α ord ret}
    (hs : SStrict body) (hs' : SStrict body')
    (hb : ∀ h', h' ≤ h → ∀ (x y : (SchedSem L mem pacing).Stream ℓ α ord ret),
      SAgreeL h' (Lg + 1) x y → SAgreeL h' Lg (body x) (body' y)) :
    SAgreeL h Lg ((SchedSem L mem pacing).fix_stream d body)
      ((SchedSem L mem pacing).fix_stream d body') := by
```

Both premises are facts about the body, and they come from the chain module's **graph lag analysis**: `<chain>.dag_agreeL`, generated by the [project phase](./project-phase.md), states that agreement of the inputs at a lag vector gives agreement of the outputs at `dag.lagOf lags`. `composedStrict` instantiates it at lag `1` for the loop wire and the parameters and `0` elsewhere, computes the output lags node by node (`proveDagLag`, `hydro_lean/Hydro/HydroGenLag.lean`), and recurses into inner knots through their own `K_strict₁`. The two reads the lemma needs come out of that walk: `hb` reads the body's output at the knot's lag `Lg` when the loop input is at `Lg + 1`, and `SStrict` reads it at `0` when the loop input is at `1`. A body without a hop or a `defer_tick` on the cycle keeps the loop input's lag, so the walk's lag exceeds the one the statement reads, and the stack stops while the `fix` elaborates. For the minimal offending knot, `fix w := union w inp`, the first error is the graded read:

```text
error: HydroGen: a lag-2 agreement cannot be read at lag 1
```

A body whose graded read passes but whose base read does not gets the dedicated message (`agreeBaseOf`):

```lean
    throwError "HydroGen: {what}: the body's output lag is {μ}, not `0` — the body is not \
      strict in the knot wire (a cycle without a network hop or `defer_tick`)"
```

The project phase projects the chain module to its graph before its knots' stacks run, because this lemma reads the chain module's `dag_agreeL`. The project phase checks the enclosing module's own graph once more, with the knot block as one `fixN` node, by cascade validity, which states the same rule per knot of a block ([`dag_valid`](./project-phase.md#proving-dag_valid-per-node)).

The statement is about the knot application `K sched args`, while the facts are about the generator's own spelling of the body function (`machineBodyFn`). The two agree after unfolding `K` and its body constant. That judgement is left to the kernel's check in `addThm`, never to `Meta`: a `simp only [K]; rw [sched_hfix_stream']; exact …` script would make `Meta` evaluate the chain once the facts mention the chain module.

## The Kleene Stages

The denotation of a knot is `iterate F seed fuel`, with `F` the body at `Values`, and proofs about it need the stages by name. `genKnotStages` emits them at `Values`, over the knot's binders:

```lean
def Hydro.toy_loop.w.stages : {L : Type} →
  …
fun {L} {mem} ℓ inp tdec sched m =>
  iterate (fun w => (toy_loop.body (Values L mem) ℓ inp tdec sched w).1)
    (MemberMap.mk' fun i => PoolBot StrOrd.noOrder Retries.exactlyOnce) m
```

```lean
@toy_loop.w.stages_succ : ∀ {L : Type} {mem : L → ℕ} (ℓ : L)
  (inp : HydroSem.Stream mem ℓ ℕ StrOrd.noOrder Retries.exactlyOnce) (tdec : ToyLoopDec (Values L mem) (mem ℓ))
  (sched : ToyStepSched (Values L mem) (mem ℓ)) (m : ℕ),
  toy_loop.w.stages ℓ inp tdec sched (m + 1) =
    (toy_loop.body (Values L mem) ℓ inp tdec sched (toy_loop.w.stages ℓ inp tdec sched m)).1
@toy_loop.w.stages_fix : ∀ {L : Type} {mem : L → ℕ} (ℓ : L)
  (inp : HydroSem.Stream mem ℓ ℕ StrOrd.noOrder Retries.exactlyOnce) (tdec : ToyLoopDec (Values L mem) (mem ℓ))
  (sched : ToyStepSched (Values L mem) (mem ℓ)),
  toy_loop.w (Values L mem) ℓ inp tdec sched = toy_loop.w.stages ℓ inp tdec sched tdec.fuel
```

`stages_zero` (the **seed**: one empty pool per member of the location, the process instances `Fin (mem ℓ)`), `stages_succ` (one step at the chain-module boundary, so its statement uses the program's own module application) and `stages_fix` (the knot is its stage at the fuel) are `rfl`. `stages_mono` (`m ≤ m' →` stage `m` is below stage `m'` pointwise) is `iterate_chain` applied to the body's composed `Values` monotonicity, which `composedMono` assembles from the chain module's and inner knots' `_mono` lemmas.

## The Invariant

A `fix` may carry an `invariant` clause with `base` and `step` obligations ([Knot Invariants](../../using/proving/knot-invariants.md) covers the user side). The `fix` elaborator registers a hook that the stack calls between `stages` and `wf`; the hook packages the obligations into one of two chain inductions from `hydro_lean/Hydro/Trace.lean`:

```lean
theorem iterate_invariant {τ : Type _} {F : τ → τ} {x : τ}
    {I : τ → Prop} :
    ∀ (n : Nat), I x →
      (∀ m, m < n → I (iterate F x m) → I (F (iterate F x m))) →
      I (iterate F x n)
```

```lean
theorem iterate_invariant_all {τ : Type _} {F : τ → τ} {x : τ}
    {I : τ → Prop} (hx : I x) (hstep : ∀ v, I v → I (F v)) :
    ∀ m, I (iterate F x m)
```

It emits `K.inv` (the predicate at the closed knot, which the enclosing `prove` field proofs receive as `h<w>_inv`) and `K.inv_stages`. A predicate that mentions the closed knot or the body's chain `let`s (the `let`s of the `fix` body) is relative to the fuel: its `step` may use the chain order `w ⊑ w'` (the current stage is below the closed knot in the denotation's order) against the closed knot, so the induction is bounded and `inv_stages` carries `∀ m ≤ fuel` (`paxos_core.a_log.inv_stages`, `toy_loop.w.inv_stages`). A predicate over the wire alone is **assumable**: the induction is unbounded, and the body may assume it of its loop input.

```lean
@toy_pre_knot.w.inv_stages : ∀ {L : Type} {mem : L → ℕ} (ℓ : L)
  (src : HydroSem.Stream mem ℓ (ℕ × ℕ) StrOrd.noOrder Retries.exactlyOnce) (wm : HydroSem.Atomic mem ℓ (Option ℕ))
  (tdec : ToyPreLoopDec (Values L mem) (mem ℓ)) (sched : ToyHopSched (Values L mem) (mem ℓ)) (m : ℕ) (i : Fin (mem ℓ)),
  ∀ a ∈ MemberMap.get (toy_pre_knot.w.stages ℓ src wm tdec sched m) i, a.2.2 = a.2.1 * 2
```

An assumed invariant becomes part of the chain module's `requires` clause (a module's precondition, stated over its inputs' denotational runs), so the body's `_co_wf` lemma takes it as a hypothesis:

```lean
@toy_pre_knot.body_co_wf₁ : ∀ {L : Type} {mem : L → ℕ} {pacing : Pacing L mem} {Tc Td : ℕ} {hjT : Tc.le Td} (ℓ : L)
  …
  (w : HydroSem.Stream mem ℓ (ℕ × ℕ × ℕ) StrOrd.noOrder Retries.exactlyOnce),
  src.4 →
    wm.4 →
      w.4 →
        (∀ (i : Fin (mem ℓ)), ∀ a ∈ w.rr.get i, a.2.2 = a.2.1 * 2) →
          (toy_pre_knot.body (CoupleSem L mem pacing Tc Td hjT) ℓ src wm tdec sched w).1.wf
```

The statement is shortened: the elided binders are the knot's arguments. The knot `wf` discharges that hypothesis with `inv_stages` at the stage it probes. The stage form is all it needs, because the corner's `wf` probes the body only at Kleene stages. `loopInvAtStage` transports `I (stages m)` to `I (iterate vbody seed m)`, the corner's spelling, along `iterate_congr` with the `HVA` fact below, and checks that the two seeds and the two bodies agree definitionally, with an error that shows both spellings if they do not.

## The `wf` Triple

The corner's `wf` for a stream knot, declared by the `fix_stream` field of `CoupleSem` (`hydro_lean/Hydro/Couple.lean`), is a triple: **strictness** of the machine body `CoStream.sbody body` in the loop wire; the **Kleene chain** of the denotation body `CoStream.vbody body`, stage `m` below stage `m + 1`; and the **graded coupling**: at every horizon `j` with `j + fixLag ≤ Tc`, a loop input coupled strictly below `j` with stage `j` gives a body output coupled at `j`. A tick knot (`fixTick`, a knot over a tick-located trace) adds a fourth conjunct, that the machine body preserves step-monotonicity (`TMono`: a realized tick never changes). From these, `co_fix_cpl` couples the knot by strong induction on the horizon ([The knot's residual](../corner/coupling.md#the-knots-residual) shows the induction). The induction lives in the instance; the generator only has to discharge the conjuncts for a body it treats as opaque.

`genKnotWf` builds seven facts as `Expr` proofs, installs them as local hypotheses, and closes the goal with a fixed script. The facts have no source text; their statements are paraphrased here:

| Fact | Statement | Source |
|---|---|---|
| `FX` | `∀ x, (body at corner, loop at schedC x).sr = body at sched, loop at x` | the chain module's `_co_sr` and the inner knots' |
| `FC` | `∀ h x y, agreeL h 1 x y → agree h (body x) (body y)` | `composedStrict`, context wires reflexive |
| `HVA` | `∀ v, vbody v = body at Values, loop at v` | the chain module's `_co_rr` at the probe |
| `FM` | the body's `Values` monotonicity | `composedMono` |
| `FH` | the Kleene chain from the seed | `iterate_le_succ` with `FM` and `HVA` |
| `FOUT` | the graded coupling at stage `j` | the lowered instance, below |
| `FSM` | `∀ x, TMono x → TMono (body x)` (tick knots) | `composedSMono` |

The script for a stream knot, verbatim:

```lean
      s!"(hb_block \"wf-simp\" (co_wf_simp [{mc.mName}]; refine ⟨?_, ?_, ?_⟩); " ++
      "(hb_block \"wf-strict\" (intro h x y hxy; have h1 := FC h x y hxy; " ++
      "rw [← FX x, ← FX y] at h1; exact h1)); " ++
      "(hb_block \"wf-chain\" (exact FH)); " ++
      "(hb_block \"wf-cpl\" (intro j hj x hxv i; exact FOUT j hj x hxv i)))"
```

`co_wf_simp [K]` unfolds the knot and exposes the triple. The strictness goal is about `CoStream.sbody body`, the corner body run on a machine probe and projected to `.sr`. `FC` is about the machine body, so `rw [← FX x, ← FX y]` turns its conclusion into the corner spelling. The chain and coupling goals are closed by `FH` and `FOUT` directly. `hb_block` only labels a block for `HYDROGEN_HB_DIAG` cost reports.

### The Lowered Instance

`FOUT` carries the knot's induction step, and `genKnotWf` builds it without walking the body. Fix a stage `j`, a horizon bound `hj : j + L ≤ Tc` (`L` is the knot's lag) and a machine loop input `x` with `hxv : ∀ t < j, ∀ i, …` coupling it below stage `v = iterate vbody seed j`. `genKnotWf` re-instantiates the body at the lowered corner `CoupleSem … (j + L) Td`. `lowerC` lowers every external wire: it keeps the machine and denotational runs and restricts the coupling. The loop position gets a corner value assembled from `x` and `v` with lag `L + 1`:

```lean
    let hc ← withLocalDecl `k .default (mkConst ``Nat) fun kE => do
      let kL ← mkAppM ``HAdd.hAdd #[kE, lagE1]
      withLocalDecl `hk .default (← mkAppM ``LE.le #[kL, jL]) fun hkE => do
      -- `k + (L + 1) ≤ j + L → k < j`: `k + L + 1 ≤ j + L`, cancel `L`
      let hkj ← mkAppM ``Nat.lt_of_add_lt_add_right
        #[← mkAppM ``Nat.lt_of_succ_le
          #[← mkExpectedTypeHint hkE
            (← mkAppM ``LE.le #[← mkAppM ``Nat.succ #[← mkAppM ``HAdd.hAdd #[kE, lagE]], jL])]]
      mkLambdaFVars #[kE, hkE] (mkApp (mkApp hxvE kE) hkj)
```

`CoStream.mkC x v (L + 1) hc` is coupled at every `k` with `k + (L + 1) ≤ j + L`, and that bound holds iff `k < j`, the range `hxv` covers. The body module's generated `_co_wf` lemma, applied at this lowered instance, gives its output's `wf`; the output's `cpl` field read at step `j` gives the coupling, with the side condition `j + Λ ≤ j + L` (`Λ` the lowered output's lag), which `co_lag_le HLAM HL` closes. Here `HLAM : Λ = E[L + 1]` and `HL : L = E[1]`, where `E[λ]` is the right side of the body's lag equation with the loop input at lag `λ`. The tactic decides the bound leaf by leaf: a loop leaf `L + 1 - k ≤ L` holds because `k ≥ 1` on every loop path (the body's strictness), and an external's leaf finds its own copy in `E[1]`. If the body module has a `requires` clause, its hypothesis is the knot's own `hreq`, the assumed invariant at stage `j` (`loopInvAtStage`), or both.

The resulting coupling is about the lowered body. `FOUT`'s statement is about the knot's `sbody` and the body at `probe2 x v`. Two transports bridge them, one by the `sr` equations and one by the `rr` equations at the two corners. Both meet because every `sr` and `rr` equation is horizon-generic: the lowered wires have the same `.sr` as the originals, `mkC x v …` has the same `.sr` as `probeC x v`, and the body's `vdec` does not depend on `Tc` ([The Couple Phase](./couple-phase.md#the-existential)). The transports are `Eq.mp` over a motive abstracted at reducible transparency:

```lean
      let abst ← withReducible <| kabstract hTy lhs
      unless abst.hasLooseBVars do
        throwError "HydroGen: knot {mc.mName}: the {what} naming's left side does not \
          occur in the coupling fact:{indentExpr lhs}\nin{indentExpr hTy}"
      let motive := Expr.lam `z (← inferType lhs) abst .default
      mkAppM ``Eq.mp #[← mkCongrArg motive eq, h]
```

`rw` would do the same with `kabstract` at default transparency, which runs `isDefEq` on every subterm with the same head and so unfolds the module through its knots at each one.

The knot's lag appears in every lowered horizon `j + L`, so the generator defines it as a separate `def`, `K_lag binders := fixLag B`. Spelled out, it would carry the whole body into the type of every lowered corner argument, and those types grow multiplicatively through nested knots.

## Guarantees and Limits

A proof about a knot unfolds at most `K` and `K.body`, by one delta step each, and never unfolds the chain module or an inner knot. Each artifact is one script or one `Expr` term over the body's generated lemmas and the inner knots' generated lemmas, so nesting adds one lemma application per inner knot, not a factor of the program text. The stack's error messages name the program-level cause: a body that is not strict in its wire, an inner knot that the stack has not generated yet (`inner knot … is not registered (generate it first)`), a body module without an `sr`, `rr` or lag equation for the selected output.

The stack requires a bare single-output knot whose body parses as one output of a registered module, possibly with inner knots in its arguments (`hydro_knot expects a bare single-leg knot`). The `fix` elaborator produces that shape, and the chain module's own pipeline runs first.
