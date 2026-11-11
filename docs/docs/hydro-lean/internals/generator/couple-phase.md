---
sidebar_position: 5
---

# The Couple Phase

You prove a contract at `Values`; the theorem you want is about `SchedSem`. The coupling corner `CoupleSem` connects them: run at it, each output of a module (one component of its output tuple) is a corner. Its field `.sr` is the **machine run**, an operational run; its field `.rr` is the **denotational run**; and its coupling field `cpl` says that under the residual `.wf`, at every step `k` with `k + lag ≤ Tc`, the machine's view sits below the pool ([The Coupling Instance](../corner/coupling.md)). To use `cpl` on a concrete module, a proof needs to know which runs `sr` and `rr` are. The couple phase proves that for every knot-free module: `M_co_srᵢ` says the machine run of output `i` is `M` run at `SchedSem`, and `M_co_rrᵢ` says its denotational run is `M` run at `Values` at a **derived decision**, a decision record computed from the machine run. With the residual lemma `M_co_wfᵢ` ([The wf, smono and mono Phases](./wf-smono-mono.md)) these two equations turn a contract proved at `Values` into a theorem about the machine.

The `sr` equation is mechanical, because every per-operator `co_<op>_sr` projection lemma is `rfl`. The `rr` equation is not, because one argument of `M (Values …)` is a decision record that nobody passed in. The corner ignores its decision argument and derives each **content** decision (one whose `Values`-side data matters: a batch's cuts, a snapshot's read point, an order selection) from its operator's machine run at the **derive horizon** `Td`, for instance `batchDerive (pacing.ticks ℓ) Td s.sr` for a `batch`, where `pacing.ticks` is the per-process tick timing ([Derived Decisions](../corner/coupling.md#derived-decisions) explains why the corner derives decisions instead of quantifying over them). The record that makes the `rr` equation true composes those derivations over the module's wires.

The obvious statement writes that record down. The generator cannot: the composite is whatever the operator equations compute after unfolding the module, and it has no closed form the generator could spell from the module's type. The record also has to be the same at every couple horizon `Tc ≤ Td`, because the residual of a knot re-runs its body module at a lowered horizon and needs the same decision there ([The Lowered Instance](./knot-stack.md#the-lowered-instance)). The phase therefore states an existential whose witness may depend only on the machine data, lets the closing unification of a fixed script find the witness while it checks the equation, and defines `M_vdec` as `Classical.choose` of the existential. This technique is `genRrEx` (`runCoupleT` in `hydro_lean/Hydro/HydroGen.lean`), and most of this page is about it. The generator derives the other equations with the rewriting engine the phase shares with the glue phase (the **glue engine**, [The Glue Phase](./glue-phase.md)).

## Routing to This Phase

`runPipeline` (`hydro_lean/Hydro/HydroDef.lean`) routes each registered module on its recorded composition spec ([Routing](./overview.md#routing)). A hoisted knot constant, whose value is `H.fix …`, takes the [knot stack](./knot-stack.md). A module with a **callee** (a registered module applied in its body) that reaches a knot, transitively, is a **glue module** and takes the [glue phase](./glue-phase.md). Every other module takes this phase. In the toy suite (`hydro_lean/Hydro/HydroGenToy.lean`), `toy_relay` (no callees) and `toy_step` (which calls the knot-free `toy_relay`) are couple modules; `toy_loop`, whose body applies the knot constant `toy_loop.w`, is a glue module. The chain body `toy_loop.body` hoisted out of `toy_loop`'s `fix` calls only `toy_step`, so it is a couple module too, and its `vdec` is what the knot stack consumes.

## Generated Artifacts

Inside the phase, `runCoupleT` chooses how to derive the `rr` equation from the module's binder classification:

| Artifact | Statement | Produced by |
|---|---|---|
| `M_co_srᵢ` | `(M corner args).ᵢ.sr = (M sched (machine args)).ᵢ` | the glue engine (`genNaming`) |
| `M_co_srcᵢ` | `(M corner args).ᵢ.src = S`, per singleton output | the glue engine (`genSrcNamingImpl`) |
| `M_co_lagᵢ` | `(M corner args).ᵢ.lag = E`, `E` over the inputs' lags | the glue engine (`genLagNamingImpl`) |
| `M_co_rrᵢ` (no content decision) | `(M corner args).ᵢ.rr = (M values (denotation args)).ᵢ` | the glue engine |
| `M_co_rr_ex`, `M_vdec`, `M_co_rrᵢ`, `M_vdec_fuelᵢ` (one content unit) | the choice triple below | `genRrEx` |

`src` is the machine source a fold singleton remembers, from which a downstream `snapshot` derives its cut. A module without content decisions needs no witness: the glue engine normalizes both sides of its `co_rr` until they meet.

A **content unit** is a binder whose `Values`-side data the corner derives: a primitive decision of a content family (`batch`, `snap`, `orderSel`, `tickOrderSel`, `ticks`, `clock`) or a decision record containing one. `genRrEx` handles one content unit per module, so programs bundle their `nondet!` sites into one record per module (`ToyRelayDec` holds both of `toy_relay`'s batch cuts). The generator rejects a module with two such binders, with `M` the module's name:

```text
HydroGen: M: exactly one content unit supported (found 2)
```

## The Existential

For `toy_relay` (`hydro_lean/Hydro/HydroGenToy.lean`, one input wire, a content record `dec` and a timer bundle `sched`), the generated statement is:

```lean
@toy_relay_co_rr_ex : ∀ {L : Type} {mem : L → ℕ} {pacing : Pacing L mem} {Td : ℕ} (ℓ : L)
  (x1 : HydroSem.Stream mem ℓ ℕ StrOrd.noOrder Retries.exactlyOnce) (d3_interval : HydroSem.IntervalDec mem (mem ℓ)),
  ∃ dV,
    ∀ (Tc' : ℕ) (h' : Tc'.le Td) (w1 : HydroSem.Stream mem ℓ ℕ StrOrd.noOrder Retries.exactlyOnce)
      (w2 : ToyRelayDec (CoupleSem L mem pacing Tc' Td h') (mem ℓ))
      (w3 : ToyRelaySched (CoupleSem L mem pacing Tc' Td h') (mem ℓ)),
      w1.sr = x1 →
        w3.interval = d3_interval →
          (toy_relay (CoupleSem L mem pacing Tc' Td h') ℓ w1 w2 w3).1.rr =
              (toy_relay (Values L mem) ℓ w1.rr dV { interval := () }).1 ∧
            (toy_relay (CoupleSem L mem pacing Tc' Td h') ℓ w1 w2 w3).2.rr =
              (toy_relay (Values L mem) ℓ w1.rr dV { interval := () }).2
```

The order of the binders fixes what the witness may depend on:

- The **outer binders** are the machine data of the corner arguments: `x1` is the input wire's machine run (`inp` is argument 1, so its binder is `x1`), and `d3_interval` is the timer script, a **machine-data** field of the record in argument 3 (data the machine consumes as given, such as a timing script or a transport cursor). Each wire contributes `x{i}`, a singleton produced by a fold contributes `x{i}_src` as well (the machine source a downstream `snapshot` derives its cut from), and each machine-data decision field contributes `d{i}_<path>`. Their types are at `SchedSem`, which has no horizon in it.
- The statement binds `dV` next, so `dV` can depend only on `pacing`, `Td`, the plain arguments and the outer binders.
- The **inner binders** re-bind the corner arguments at an arbitrary lower horizon `Tc' ≤ Td`. Their types mention `Tc'`, so `dV` cannot mention them.
- The **pins** `w1.sr = x1` and `w3.interval = d3_interval` tie the inner arguments' machine data to the outer binders. They are the only route from an inner argument to something `dV` can see.

The witness is therefore horizon-generic by construction: the decision that makes the equation true at `Tc'` is a function of the machine data and `Td` alone. The knot `wf` depends on this. It instantiates a knot's body module at the lowered corner `CoupleSem … (j + L) Td` and compares the result with the body at the knot's own corner, and the two denotational runs meet because both apply the same `M_vdec` to the same machine data (see [the lowered instance](./knot-stack.md#the-lowered-instance)).

`genRrEx` assembles the statement at the `Expr` level from the module telescope (`withModCtx`):

```lean
  let conj ← mkAndN legEqs
  let innerForall ← mkForallFVars (#[Tc', h'] ++ inners ++ pins) conj
  let exStmtBody ← mkAppM ``Exists #[← mkLambdaFVars #[dV] innerForall]
  let exBinders := mc.ctxArgs ++ #[mc.pacing, mc.Td] ++ plainArgs ++ outers
  let exStmt ← mkForallFVars exBinders exStmtBody
```

## The Script

`genRrEx` proves the existential with one tactic string, assembled from the module's shape:

```lean
  let script :=
    s!"(intros; apply Exists.intro (({witness})); intro {introNames}; {refinePart}" ++
    s!"(try co_simp [{mc.mName}{rulesSuffix rules}{rulesSuffix mc.extras}]); " ++
    s!"{calleeRws}{pinRws}{closer}))"
```

The script runs through `proveByTac`, in an empty local context with error recovery off (see [Overview](./overview.md)). For `toy_relay` the pieces are filled in as follows.

`intros` binds the outer binders. `apply Exists.intro ((⟨_, _⟩))` opens the witness as a **skeleton** with one hole per field, not as a bare hole:

```lean
partial def skeletonOf (t : Array DecFieldTree) : String :=
  "⟨" ++ String.intercalate ", "
    (t.toList.map fun
      | .prim _ f =>
        if f.schedPass && !f.isContent then "()" else "_"
      | .node _ _ sub => skeletonOf sub) ++ "⟩"
```

The goal will constrain the record through its projections (`?dV.batch` appears inside `Values.batch`). A projection of a bare metavariable is not solvable by unification, but a projection of an anonymous constructor reduces to the field's hole, which is. A machine-only field is `Unit` at `Values` and never occurs in the reduced equations, so its hole would stay unassigned. The skeleton fills it with the literal `()`.

`intro Tc' h' w1 w2 w3 hp_x1 hp_d3_interval` binds the inner universals and the pins. `refine ⟨?_, ?_⟩ <;> (…)` splits the conjunction into one goal per output, and every remaining step runs on each.

`co_simp [toy_relay, …]` unfolds the module on both sides and pushes `rr` through every operator with the `rfl` projection set. On the left the batch site becomes the `Values` batch at its derived cut, through

```lean
theorem co_batch_rr {α : Type} [DecidableEq α]
    (s : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α .noOrder .exactlyOnce)
    (cutd : (CoupleSem L mem pacing Tc Td hjT).BatchDec (mem ℓ) α .noOrder .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).batch s cutd).rr
      = (Values L mem).batch s.rr (batchDerive (pacing.ticks ℓ) Td s.sr)
    := rfl
```

so the left side mentions `batchDerive (pacing.ticks ℓ) Td ((SchedSem L mem pacing).map w1.sr …)`. On the right the same operator appears at the witness hole `?a`.

When the module has callees (`toy_step` calls `toy_relay`), `co_residuals [callee rules]` follows. It rewrites with the callees' generated equations, in the positions `simp`'s congruence cannot reach, on the left side only. A `rw` loop over the whole goal would run `kabstract` against every candidate, the witness holes included, and unify each one against them. The tactic packages the glue engine's residual pass for this use ([The Glue Phase](./glue-phase.md#the-co_residuals-tactic)).

The pin rewrites come next:

```lean
  let pinRws := String.join (pinReqs.toList.map fun (n, _) =>
    s!"(try rw [{n}]); (try simp only [{n}]); ")
```

They are what makes the closing unification legal. `apply Exists.intro` creates the holes `?a` and `?b` before `intro` binds `w1`, so `w1` is not in their local context, and the unifier would reject the assignment `?a := batchDerive … w1.sr …`. After `rw [hp_x1]` the derived cut mentions `x1`, which is in scope for the holes. The script tries both `rw` and `simp only` because a callee singleton's `co_src` equation puts the pinned machine run under a binder (`famHist (fun s j => w.sr j s)`), where `rw` cannot reach.

`(try exact rfl)` closes each goal. Both sides are now the same operator tree, with `batchDerive … x1 …` on the left where the right has a hole, so the definitional check assigns the holes. Running the module's equations is what computes the witness.

Every `try` in the string is parenthesized, because in a one-line script an unparenthesized `try` takes the rest of the sequence as its argument, and a failing pin rewrite would then silently skip the closing `rfl`. With `HYDROGEN_SR_DIAG` set the closer is omitted, so the unsolved goal prints in normalized form with the witness holes visible.

## Fuel and Default Pins

Some fields of `dV` never occur in any output's equation, and their holes would stay unassigned after the closing `rfl`. `genRrEx` adds explicit trailing conjuncts for them, each closed by the same `rfl`:

- The record carries a `fixd` field (a knot's fuel), but only the knots inside use it, and those knots do not occur in this module's body. The corner pins every fuel to `Td + 1`, so the conjunct is `dV.<path> = Td + 1`. `genRrEx` turns these conjuncts into the theorems `M_vdec_fuelᵢ`, and the knot stack's `co_knot_rr₂` consumes them: `M_vdec` is a `Classical.choose` term, so its fuel fields equal `Td + 1` propositionally, not definitionally.
- A field the body never projects gets the conjunct `dV.<path> = default`. Whether a field is projected is read off the body's used constants, and a record passed whole to a callee counts as using every field (`recordFlowsWhole`). A field type without an `Inhabited` instance stays existentially quantified, with the message `hydro_couple M: unused dec field … has no Inhabited default — left ∃-quantified`.

## Defining `M_vdec` by Choice

`genRrEx` defines `M_vdec` as `Classical.choose` of the existential and takes each output's equation from `Classical.choose_spec`:

```lean
  let exApp := mkAppN (mkConst exName) exBinders
  let vdecName := mc.mName.appendAfter "_vdec"
  let vdecTyAll ← mkForallFVars exBinders vDecTy
  let vdecVal ← mkLambdaFVars exBinders
    (← mkAppM ``Classical.choose #[exApp])
  addDefn vdecName vdecTyAll vdecVal
```

`M_vdec` takes the existential's outer binders as its arguments, so it has no `Tc`:

```lean
@toy_relay_vdec : {L : Type} →
  {mem : L → ℕ} →
    {pacing : Pacing L mem} →
      {Td : ℕ} →
        (ℓ : L) →
          HydroSem.Stream mem ℓ ℕ StrOrd.noOrder Retries.exactlyOnce →
            HydroSem.IntervalDec mem (mem ℓ) → ToyRelayDec (Values L mem) (mem ℓ)
```

Each `M_co_rrᵢ` is the `i`-th conjunct of `Classical.choose_spec` instantiated at the corner's own arguments: the outer binders at the arguments' machine data, `Tc' := Tc`, the inner binders at the arguments themselves, and every pin by `Eq.refl`. `genRrEx` then writes the statement with `M_vdec` in place of the choice term; the two are definitionally equal:

```lean
@toy_relay_co_rr₁ : ∀ {L : Type} {mem : L → ℕ} {pacing : Pacing L mem} {Tc Td : ℕ} {hjT : Tc.le Td} (ℓ : L)
  (inp : HydroSem.Stream mem ℓ ℕ StrOrd.noOrder Retries.exactlyOnce)
  (dec : ToyRelayDec (CoupleSem L mem pacing Tc Td hjT) (mem ℓ))
  (sched : ToyRelaySched (CoupleSem L mem pacing Tc Td hjT) (mem ℓ)),
  (toy_relay (CoupleSem L mem pacing Tc Td hjT) ℓ inp dec sched).1.rr =
    (toy_relay (Values L mem) ℓ inp.rr (toy_relay_vdec ℓ inp.sr sched.interval) { interval := () }).1
```

Only the existential is proved by a script; `M_vdec`, `M_co_rrᵢ` and `M_vdec_fuelᵢ` are `Expr`-built projections of it. The kernel checks every declaration. `Classical.choose` is the triple's one non-constructive step, and `#print axioms toy_safe_sched` in `HydroGenCheck.lean`, whose proof goes through `toy_relay_vdec`, reports only the standard three axioms.

## Uses and Limits

A consumer never sees the derived record's shape. `toy_safe_sched` in `hydro_lean/Hydro/HydroGenCheck.lean` turns `toy_relay`'s `ensures` contract (a property of its denotation, proved in the program) into a machine theorem. It uses the corner's coupling field `cpl`, the `sr` and `rr` equations, and the contract instantiated at the opaque `vdec`:

```lean
  rw [hsr, hrr] at hcpl
  have hens := toy_relay.ensures ℓ v
    (toy_relay_vdec (pacing := pacing) (Td := T) ℓ h fires) ⟨()⟩
  exact hens.pos i x (Multiset.mem_of_le hcpl (Multiset.mem_coe.mpr hx))
```

The contract quantifies over every decision record, so instantiating it at a record nobody can inspect is sound and costs nothing. Every contract on the denotation becomes a theorem on the machine this way ([Transferring to the Machine](../../using/proving/transferring-to-the-machine.md)).

A body module's `vdec` is also what the knot stack applies at the knot's own machine wires, with the loop position at the machine knot (`toy_loop.body_vdec … (toy_loop.w (SchedSem L mem pacing) …)` in `toy_loop.w_co_rr₁`). Because the `vdec` is horizon-generic, the same term serves the knot's corner and every lowered corner the knot `wf` builds.

A module has at most one content unit. The witness exists only if `co_simp`, the residual pass and the pins bring the two sides to the same operator tree, so a new operator needs `co_<op>_rr`/`_sr` projections that are `rfl` (see [Projections and Tactics](../corner/projections-and-tactics.md)). An unassigned hole surfaces as an unsolved-goals error from `proveByTac` that shows the script and the residual goal.
