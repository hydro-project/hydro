---
sidebar_position: 2
title: Projections and Tactics
---

# Projections and Tactics

Running a module `M` (one `hydro def` function) at the coupling instance `CoupleSem` produces a corner per output: a machine run `sr`, a denotational run `rr`, a lag and a residual `wf` ([The Coupling Instance](./coupling.md) defines all four, and the knots, conditional folds and derived decisions mentioned below). Every theorem built on the corner first rewrites those fields: `(M (CoupleSem …) args).sr` equals `M (SchedSem …)` at the machine arguments, `.rr` equals `M (Values …)` at the derived decision record, `.lag` equals an arithmetic expression in the inputs' lags, and `.wf` unfolds to the conjunction of the inputs' residuals and the knots' and conditional folds' obligations. The generator emits these equations for every module as `M_co_srᵢ`, `M_co_rrᵢ`, `M_co_lagᵢ` and `M_co_wfᵢ` ([The Couple Phase](../generator/couple-phase.md)).

At a single operator each of these equations is `rfl`: every `CoupleSem` field is defined as the corresponding `SchedSem` or `Values` operator on `sr` and `rr`. A whole-module equation could be decided the same way, but the kernel decides it by unfolding the module, every callee and every knot body, and through knots that unfolding is exponential (measurements below). Hydro Lean therefore keeps definitional equality at operator scale and does everything above it by rewriting, with the projection lemmas of `Hydro/CoupleProj.lean`, the macros that apply them, and the knot tactics of `Hydro/KnotTactics.lean` and `Hydro/WfTactics.lean`.

## Projection Lemmas

For every operator `op` of the signature, `Hydro/CoupleProj.lean` states the fields of its corner as separate `rfl` lemmas:

| lemma | states |
|---|---|
| `co_<op>_rr` | the output's `rr` equals the `Values` operator on the inputs' `rr`, at the decision derived from the machine run |
| `co_<op>_sr` | the output's `sr` equals the `SchedSem` operator on the inputs' `sr`, at the machine decision |
| `co_<op>_wf` | the output's `wf` in terms of the inputs' |
| `co_<op>_lag` | the output's lag in terms of the inputs' |
| `co_<op>_src` | for fold corners, the machine source the fold remembers |

The three lemmas for `batch` show what "at the derived decision" means in a statement:

```lean
theorem co_batch_rr {α : Type} [DecidableEq α]
    (s : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α .noOrder .exactlyOnce)
    (cutd : (CoupleSem L mem pacing Tc Td hjT).BatchDec (mem ℓ) α .noOrder .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).batch s cutd).rr
      = (Values L mem).batch s.rr (batchDerive (pacing.ticks ℓ) Td s.sr)
    := rfl

theorem co_batch_sr {α : Type} [DecidableEq α]
    (s : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α .noOrder .exactlyOnce)
    (cutd : (CoupleSem L mem pacing Tc Td hjT).BatchDec (mem ℓ) α .noOrder .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).batch s cutd).sr
      = (SchedSem L mem pacing).batch (ℓ := ℓ) (ord := .noOrder) (ret := .exactlyOnce) s.sr () := rfl

theorem co_batch_wf {α : Type} [DecidableEq α]
    (s : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α .noOrder .exactlyOnce)
    (cutd : (CoupleSem L mem pacing Tc Td hjT).BatchDec (mem ℓ) α .noOrder .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).batch s cutd).wf = s.wf := rfl
```

The corner's `batch` matches on the grade, so there is one rule per grade (`co_batch_rr`, `co_batch_retries_rr`, `co_batch_ordered_rr`, `co_batch_stutter_rr`), each with its grade fixed in the statement, so that the keyed rule selection of the glue phase (`hydro_rule_apply`, [The Glue Phase](../generator/glue-phase.md)) matches the right `Values` cut function: the four grades use four different ones. The lag rules carry the strictness arithmetic:

```lean
theorem co_broadcast_closed_lag {c p : L} {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries}
    (dt : (CoupleSem L mem pacing Tc Td hjT).TransportDec (mem p) (mem c))
    (s : (CoupleSem L mem pacing Tc Td hjT).Stream c α ord ret) :
    ((CoupleSem L mem pacing Tc Td hjT).broadcast_closed (p := p) dt s).lag = s.lag - 1 := rfl
```

Each lemma is `rfl` at variable arguments, so its proof is checked once, here, and no program re-proves it. A module's `sr` or `rr` equation then costs one rewrite per operator occurrence, and the kernel's remaining work is one comparison of two terms already in the same normal form.

## Instance-Typed Spellings

Most of the file exists because of two facts about how `simp` matches, stated in its header:

> `simp` unifies at `instances` transparency, so every rule is stated at the instance-projected carrier types (`(CoupleSem …).Stream ℓ α ord ret`, not `CoStream Tc (mem ℓ) α ord ret`); the boundary constructors (`inputC`, `mkC`, `probeC`, `schedC`, `lowerC`, `lagProbeC`, `CoDec.*`) and the `CoTickI` facade exist to give the raw `Couple.lean` definitions at those types. And where a lemma's binder types cannot meet the walk's terms (projected `tick_scan` outputs, the lags of boundary gadgets over raw legs) a simproc computes the definitional right-hand side instead.

A program's goals mention `(CoupleSem L mem pacing Tc Td hjT).Stream ℓ α ord ret`. The instance's field reduces to `CoStream …` only at default transparency, which `simp` does not use, so a rule stated over `CoStream` never fires. Each raw constructor of `Hydro/Couple.lean` therefore has an instance-typed twin, and its projections are their own `rfl` rules:

```lean
def CoStream.inputC {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (h : (SchedSem L mem pacing).Stream ℓ α ord ret)
    (v : (Values L mem).Stream ℓ α ord ret)
    (hc : ∀ t i, ListLe ord ret ((h i).view t) (v i)) :
    (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α ord ret :=
  CoStream.input h v hc

theorem co_input_rr {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries}
    (h : (SchedSem L mem pacing).Stream ℓ α ord ret)
    (v : (Values L mem).Stream ℓ α ord ret)
    (hc : ∀ t i, ListLe ord ret ((h i).view t) (v i)) :
    (CoStream.inputC (Tc := Tc) (Td := Td) (hjT := hjT)
      (pacing := pacing) h v hc).rr = v := rfl
```

The same mismatch also occurs in the opposite direction. When `simp` assigns a metavariable during matching, it type-checks the assignment at reducible transparency too. The knot's graded obligation binds its machine runs at the raw carrier types (`Fin (mem ℓ) → StepHist α`), so a rule whose binders are instance-typed cannot be instantiated there. Those rules exist twice, the second time with raw binders and the suffix `_raw` (`co_mkC_rr_raw`, `co_probe2_rr_raw`, `co_embedS_sr_raw`, …).

Decisions have the same problem. A content decision at the corner is `()`, but a raw `()` sits at type `Unit`, not at `(CoupleSem …).BatchDec …`, and a rule pinned on the decision argument does not match it. The decision constructors are `@[reducible]` wrappers at the instance types (`CoDec.batch`, `CoDec.snap`, `CoDec.cursors`, `CoDec.interval`, …), and every operator whose decision type is `Unit` at the target instance gets a normalizer that rewrites any decision to `()`:

```lean
theorem sched_batch_unit {α : Type} [DecidableEq α] {ord : StrOrd} {ret : Retries}
    (u : (SchedSem L mem pacing).BatchDec (mem ℓ) α ord ret)
    (s : (SchedSem L mem pacing).Stream ℓ α ord ret) :
    (SchedSem L mem pacing).batch (ℓ := ℓ) s u
      = (SchedSem L mem pacing).batch (ℓ := ℓ) s () := rfl
```

so that the two sides of an `sr` or `rr` equation meet with the same literal in the decision position.

## Simprocs

**`coTickScanProj`.** A `tick` block is one former `tick_scan sts ins outs x g init` whose output is a shaped tuple ([Ticks](./coupling.md#ticks)). Its corner is built by `CoTick.mk`, which recurses on the output shape, so for every projection path `π` into the tuple the machine run `(π (tick_scan …)).sr` is definitionally `π (SchedSem.tick_scan sts ins outs (srLegs ins x) (gS sts ins outs g) init)`. No lemma can state this for all paths at once, and the `mk`-with-coupling spelling has a proof argument whose type the walk cannot keep in step with the rewritten `sr` and `rr`. The simproc computes the path and emits the right-hand side directly:

```lean
  let rhs ← match leg with
    | 0 =>
      let sched := mkAppN (mkConst ``SchedSem) #[L, mem, pacing]
      applyProdPath (formerAt sched (helper ``CoTickI.srLegs #[ins, x])
        (helper ``CoTickI.gS #[sts, ins, outs, g])) path
    | 1 =>
      let vals := mkAppN (mkConst ``Values) #[L, mem]
      applyProdPath (formerAt vals (helper ``CoTickI.rrLegs #[ins, x])
        (helper ``CoTickI.gR #[sts, ins, outs, g])) path
    | 2 => pure (helper ``CoTickI.scanWf #[sts, ins, outs, x, g])
    | _ => pure (helper ``CoTickI.Lag #[ins, x])
  return .visit { expr := mkAppN rhs extra }
```

(`Hydro/CoupleProj.lean`, the tail of `coTickScanProj`.) Its key is an `.sr`, `.rr`, `.wf` or `.lag` projection of a projected former, so it fires where a walk's `.sr`/`.rr`/`.wf`/`.lag` reaches a former's output and never on a bare `tick_scan` inside a callee's argument. The `CoTickI` helpers it emits restate the `CoTick` machinery of `Hydro/Couple.lean` at the instance-projected types, for the first spelling fact above.

**`coLagLeaf`.** The lag of a boundary gadget (an input, a probe, a `mkC`, a horizon lowering) is a literal in the gadget's constructor, but the gadget's arguments are often raw-typed runs (the knot's machine diagonal under a probe), where no lemma binder matches. The simproc unfolds only the gadget heads listed in `coLagLeafHeads`, at default transparency, down to the structure literal, and returns the `lag` field.

## Rule Sets and Macros

The rules are collected in three arrays (`coSimpRules`, `coWfSimpRules`, `coLagRules`) and loaded at import into three simp attributes declared in `Hydro/SimpAttr.lean` (`co_simp_rules`, `co_wf_simp_rules`, `co_lag_rules`). As attributes the sets are built once per module instead of once per tactic call. The macros:

```lean
macro "co_simp" "[" ids:Lean.Parser.Tactic.simpLemma,* "]" : tactic =>
  `(tactic| simp only [co_simp_rules, coTickScanProj, $ids,*])

macro "co_transfer" "[" ids:Lean.Parser.Tactic.simpLemma,* "]" :
    tactic =>
  `(tactic| (co_simp [$ids,*]; first
    | done
    | with_reducible rfl
    | exact rfl))

macro "co_wf_simp" "[" ids:Lean.Parser.Tactic.simpLemma,* "]" : tactic =>
  `(tactic| simp only [co_wf_simp_rules, coTickScanProj, $ids,*])

macro "co_lag_simp" "[" ids:Lean.Parser.Tactic.simpLemma,* "]" : tactic =>
  `(tactic| simp only [co_lag_rules, coTickScanProj, ↓coLagLeaf, $ids,*])

macro "co_lag" "[" ids:Lean.Parser.Tactic.simpLemma,* "]" : tactic =>
  `(tactic| ((try co_lag_simp [$ids,*]) <;> omega))
```

(`Hydro/CoupleProj.lean`.) The `ids` are the module definitions to unfold and the callees' own `sr` and `rr` equations. A callee stays folded, and its own equation rewrites it; no macro unfolds a callee into the caller's goal.

- `co_simp` pushes `rr`, `sr` and `src` through every operator. The generator's couple phase runs it to prove a module's `sr` and `rr` equations.
- `co_transfer` is `co_simp` with a closer. `with_reducible rfl` closes goals whose sides agree up to reducible unfolding; `exact rfl` at default transparency comes last and is there for decision-record pins, whose witness metavariables were assigned at raw carrier types and need default transparency to type-check once both sides are already in simp normal form.
- `co_wf_simp` reduces a residual to its components: every operator conjoins its inputs' `wf`, boundary inputs contribute `True`, and what is left is the knots' triples and the conditional folds' pairwise facts. Its rule set also contains the `CoTickI` facade rules that turn a tick former's `scanWf` into its leaf conditions (W1, W2, W3; see [Ticks](./coupling.md#ticks)).
- `co_lag` closes a lag-graded side condition `k + (…).lag ≤ T` by normalizing the lag and calling `omega`. A program uses it where it reads a coupling at the horizon (`T (by co_lag [toy_relay_co_lag₁])` in `toy_safe_sched`).

### Lag Expressions

A lag expression is a `max` of leaves `a - k` (`a` an input's lag, `k` a numeral) and constants. Through nested knots it grows multiplicatively, because each knot's lag substitutes its body's output lag for the loop input's. The normal form pushes subtractions to the leaves, right-nests and sorts the `max`, and merges two leaves of one atom into `a - min k k'`; the arithmetic pushing is in `coLagRules`, and the sorting and merging is `lagCanon` in `Hydro/HydroGenKnot.lean`, a term-level normalizer, because AC rewriting with `simp` is super-linear on these trees.

The knot's own side condition, from the graded coupling at the lowered corner `j + L` ([Knots](./coupling.md#knots)), is `j + Λ ≤ j + L`, where `L = E[1]` is the knot's lag (the body's output lag `E` with the loop input at lag `1`) and `Λ = E[L + 1]` is the body's output lag at the lowered corner. `omega` decides it, but case-splits every `max`, which is exponential. `co_lag_le` decides it leaf by leaf instead:

```lean
macro "co_lag_le" hlam:term:max hl:term:max : tactic =>
  `(tactic| (
    rw [$hlam:term]
    refine Nat.add_le_add_left ?_ _
    try simp only [← Nat.sub_max_sub_right, Nat.sub_sub, Nat.max_le, Nat.sub_le, le_refl,
      Nat.le_refl, Nat.zero_le, Nat.zero_sub, Nat.zero_max, Nat.max_zero, Nat.sub_zero,
      Nat.sub_self, Nat.add_sub_cancel, lag_probe_leaf_le, Nat.reduceSub, Nat.reduceAdd,
      Nat.reduceLeDiff, true_and, and_true, and_self]
    try simp only [$hl:term]
    try simp only [← Nat.sub_max_sub_right, Nat.sub_sub, le_max_iff, le_refl, Nat.le_refl,
      true_or, or_true, Nat.zero_le, Nat.zero_sub, Nat.zero_max, Nat.max_zero, Nat.sub_zero,
      true_and, and_true, and_self]))
```

After `rw [hlam]` and cancelling `j`, the first `simp only` splits `max … ≤ L` into one goal per leaf. A leaf from the loop input has the form `L + 1 - k` with `k ≥ 1`, because every loop path passes a strict operator (a network hop or `defer_tick`, [Lag](./coupling.md#lag)), and closes by

```lean
theorem lag_probe_leaf_le {L m k : Nat} (h : m ≤ k) : L + m - k ≤ L := by omega
```

A leaf from an external input, `a - k`, is closed by unfolding `L` with `hl` and finding the same leaf in `E[1]` under `le_max_iff`: both leaves come from the same path through the body.

## Worked Example

The heaviest user of `co_simp` is the couple phase, which proves a module's `rr` equation through an existential and closes it with one script ([The Couple Phase](../generator/couple-phase.md#the-script) shows the statement, the script and every step). Inside that script the rule set does the following. For `toy_relay` (`Hydro/HydroGenToy.lean`, a batch of its input, a timer, a `tick` block, an `allTicks`), the goal after the witness skeleton is opened has the corner run's `rr` on the left and the module at `Values`, with holes for the decision, on the right:

```lean
(toy_relay (CoupleSem L mem pacing Tc' Td h') ℓ w1 w2 w3).1.rr =
    (toy_relay (Values L mem) ℓ w1.rr dV { interval := () }).1
```

`co_simp [toy_relay]` unfolds `toy_relay` on both sides and pushes `.rr` inward one operator at a time, each step a lemma that holds by `rfl`:

- `co_allTicks_rr` and the `coTickScanProj` simproc open the tick block's former to the `Values` former over the `rr` fields of its inputs.
- `co_batch_rr` turns the content batch into `(Values L mem).batch … (batchDerive (pacing.ticks ℓ) Td s.sr)`, where `s` is the mapped input; `co_map_sr` rewrites `s.sr` to the `SchedSem` map of `w1.sr`.
- `co_batch_stutter_rr` does the same for the timer's batch with `batchStutterDerive`, over the machine timer that `co_source_interval_sr` produces.

After these rewrites no `CoupleSem` operator is left on the left-hand side: it is the `Values` program applied to derived cuts, and the right-hand side is the same program applied to holes, so a single `rfl` assigns the decision. Every operator of the signature therefore has an `_rr` and an `_sr` lemma in the set: one missing lemma leaves a `CoupleSem` operator on the left, and the closing `rfl` fails. A module that calls another passes the callee's `_co_rr₁` and `_co_sr₁` as extra rules, and those equations rewrite the callee by name; the script never unfolds it.

## Rewriting through Knots

Definitional equality through nested knots is exponential in the nesting depth. One default-transparency `rfl` on the real `leader_election` module, on one machine:

| shape | time |
|---|---|
| body, no knot | 9 ms |
| one knot | 7 ms |
| two nested knots | 21 ms |
| three nested knots (`leader_election`), `sr` equation | ~4 min |
| three nested knots, `rr` equation | >70 min |
| five nested knots (`leader_election`'s three under `paxos_core`'s two) | >80 min, stopped at 51 M heartbeats |

Each outer knot's body re-closes the inner knots, and the kernel's substitution loses sharing, so the term multiplies once per level. Module-scale and body-scale `rfl`s stay in milliseconds; the cliff is only the knots. The framework therefore follows one rule: **definitional equality never crosses a knot boundary**. The generator proves a knot's `sr` and `rr` equations one knot at a time, with definitional steps only at record scale and leaf scale, and it keeps the body folded by rewriting with the body's own equations and the inner knots' equations. This needs the knot bodies to be top-level constants (for the toy knot, `toy_loop.w`, `toy_loop.w.body` and `toy_loop.body`, which the elaborator hoists; see [Elaborating `hydro def`](../generator/elaborating-hydro-def.md)), so that there is a name to rewrite with.

The knot projections are stated against the program-facing `HydroSem.fix` wrapper, with the machine diagonal as its own top-level `def`, `fixSrC`:

```lean
def CoStream.fixSrC {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (caps : Γ (CoupleSem L mem pacing Tc Td hjT))
    (body : ∀ H', Γ H' → H'.Stream ℓ α ord ret → H'.Stream ℓ α ord ret) :
    (SchedSem L mem pacing).Stream ℓ α ord ret :=
  (SchedSem L mem pacing).fix_stream (ℓ := ℓ) ()
    (fun x => (body _ caps (CoStream.schedC x)).sr)

theorem co_hfix_stream_rr {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries}
    (df : (CoupleSem L mem pacing Tc Td hjT).FixDec)
    (caps : Γ (CoupleSem L mem pacing Tc Td hjT))
    (body : ∀ H', Γ H' → H'.Stream ℓ α ord ret → H'.Stream ℓ α ord ret) :
    ((CoupleSem L mem pacing Tc Td hjT).fix (ℓ := ℓ) df caps body).rr
      = (Values L mem).fix_stream (ℓ := ℓ) (α := α) (ord := ord)
          (ret := ret) (Td + 1)
          (fun v => (body _ caps (CoStream.probeC
            (CoStream.fixSrC caps body) v)).rr) := rfl
```

`fixSrC` stays folded in the `rr` equation. It appears there only inside the arguments derived decisions are computed from, and opening it would re-normalize the whole machine body once per knot; folded, the in-knot and top-level occurrences of a decision site are the same term. Only the proof of the `sr` equation opens it (`co_fixSrC_eq`).

The rewrite loop and the tactic for the `sr` equation:

```lean
macro "co_body_rw" "[" rules:term,* "]" : tactic => do
  let closers : Array (TSyntax `tactic) := #[
    ← `(tactic| rw [co_probeC_rr]),
    ← `(tactic| rw [co_probeC_sr]),
    ← `(tactic| rw [co_tick_probeC_rr]),
    ← `(tactic| rw [co_tick_probeC_sr])]
  let userRws ← rules.getElems.mapM fun r => `(tactic| rw [$r:term])
  let alts := userRws ++ closers
  let failTac ← `(tactic| fail "co_body_rw: no rule applies")
  let alt ← alts.foldrM (init := failTac) fun t acc =>
    `(tactic| first | $t:tactic | $acc:tactic)
  `(tactic| repeat $alt:tactic)

macro "co_knot_sr" "[" defs:ident,* "]" "[" rules:term,* "]" : tactic => do
  let ds := defs.getElems
  `(tactic| (
    unfold $[$ds]*
    -- target side: the `H.fix` wrapper to `fix_stream`/`fix_tick`, by transitivity
    first
      | refine Eq.trans ?_ (sched_hfix_stream' _ _ _).symm
      | refine Eq.trans ?_ (sched_hfix_tick' _ _ _).symm
    -- corner side: project the fix to the machine diagonal
    first
      | rw [co_hfix_stream_sr, co_fixSrC_eq]
      | rw [co_hfix_tick_sr, co_tick_fixSrC_eq]
    -- the knot boundary: fuel legs by record-scale defeq, then one generic iterate
    refine congrArg₂ _ rfl ?_
    funext x
    co_body_rw [$rules,*]
    try simp only [co_schedC_sr, co_tick_schedC_sr]
    try with_reducible rfl
    try exact rfl))
```

(`Hydro/KnotTactics.lean`.) Line by line:

- `unfold $[$ds]*` opens the knot definition and the body definitions between the `fix` and the projections the rules rewrite.
- The target side `SchedSem.fix …` is aligned with `fix_stream` through `Eq.trans` with the wrapper lemma. A higher-order `rw` cannot match a `fix` whose body is a hoisted constant; unifying the fully applied lemma against one side of the goal is first-order and always succeeds.
- `rw [co_hfix_stream_sr, co_fixSrC_eq]` turns the corner's `fix(...).sr` into `SchedSem.fix_stream () (fun x => (body _ caps (schedC x)).sr)`.
- `congrArg₂ _ rfl ?_` splits the two `fix_stream` applications: the decision argument closes by `rfl` at record scale, and `funext x` leaves the two bodies at an arbitrary loop wire `x`.
- `co_body_rw` folds the body: `repeat (first | rw [r₁] | … | rw [rₙ] | <probe closers>)`, the caller's rules first (the body module's `_co_sr` and the inner knots' `_co_sr₁`), the probe projections last. A `rw` consumes one instance per call, so the `repeat` drains every occurrence.
- The leaf residue (`(schedC x).sr = x`, a `Unit`-eta in a fuel field) closes by `simp only`, reducible `rfl` or `exact rfl`.

The loop uses `rw` and not `simp` because of a third spelling fact: `simp` does not rewrite a module projection nested in another module application's argument when that argument position is dependently typed, which the contract-carrying subtypes of `hydro def` make every wire argument. The same lemma instance is definitionally equal there at reducible transparency, and `rw`'s `kabstract` reaches the position; `simp`'s congruence treats the whole application as atomic.

`co_knot_rr hsr [defs] [rules]` has the same skeleton on the `Values` side (`values_hfix_stream'`, `co_hfix_stream_rr`), with one extra step: after `funext v` it rewrites with `hsr`, the knot's own `sr` equation fully applied. This `rw` replaces the folded machine diagonal inside the `Values` iterate by the machine knot. The generator's variant `co_knot_rr₂` adds closers for the fuel fields, which equal `Td + 1` propositionally (by the generated lemmas `M_vdec_fuelᵢ`) and not definitionally. The generator emits the `sr` equation with the template

```lean
  addThmByTac name stmt s!"(intros; co_knot_sr [{defs}] [{rules}])"
```

(`Hydro/HydroGenKnot.lean`), so the proof of a knot's `sr` equation is one tactic call. [The Knot Stack](../generator/knot-stack.md) shows the statements.

## Step-Monotonicity by Dispatch

A tick knot's residual has a fourth conjunct, `∀ x, TMono x → TMono (sbody x)`: the machine body preserves step-monotonicity of tick traces (`TMono`, [Ticks](./coupling.md#ticks)). The generator proves `TMono` goals with the walk `hydro_smono_walk` (`Hydro/HydroGen.lean`), a `repeat'` over alternatives in a fixed order: close the goal from a hypothesis, close it by a callee's `_smono` lemma through `hydro_rule_apply`, take one operator step with `co_smono_step`, split a conjunction, or split a `match`. `co_smono_step` (`Hydro/WfTactics.lean`) is the operator step, and it applies exactly one law, chosen by the head operator:

```lean
private def coSMonoDispatch : List (Name × Name) := [
  (``HydroSem.snapshot, ``tmono_snapshot),
  (``HydroSem.batch, ``tmono_batch),
  (``HydroSem.firstTick, ``tmono_firstTick),
  (``HydroSem.mapTick, ``tmono_mapTick),
  (``HydroSem.zipTick, ``tmono_zipTick),
  (``HydroSem.defer_tick, ``tmono_defer_tick),
  …
  (``HydroSem.tick_scan, ``tmono_tick_scan)]
```

On a `Prod.fst`/`Prod.snd` of a shaped tick tuple it applies `TMonoOf_fst`/`TMonoOf_snd` with the pair's shapes pinned from the tuple's type (`applyWithShapes`); on a `tick_scan` it restates the goal as `TMonoOf outs` and applies `tmono_tick_scan`; on anything else it looks the head up and applies that law, or fails with `co_smono_step: no dispatch entry for …`.

### A Failing `apply` on a Projection Spine

Both halves of the walk avoid trying rules in turn, because a failing `apply` against a module's output is exponential in its depth. A module with many outputs returns a right-nested tuple, so its outputs are `(M …).1`, `(M …).2.1`, `(M …).2.2.1` and so on, a **projection spine** of `Prod.snd` applications over one shared application. A rule for one output tried against a goal about another matches the spine structurally and learns of the mismatch only at the bottom. On the way back up the unifier retries every level in its unfolded `.proj` form, because `Prod.snd` is reducible, and with metavariables on the rule side the failures are not cached. Measured on `leader_election`'s body, a failing `with_reducible apply leader_election.body_smono₁₄` against a subject fourteen `Prod.snd` levels deep takes 7.08 s; at twelve levels 2.44 s, at ten 0.83 s, about 1.7× per shared level.

A `first | apply r₁ | … | apply rₙ` walk pays that for every wrong rule before it reaches the right one. `co_smono_step` therefore applies one operator law per node and never a wrong one, and `hydro_rule_apply` compares a cheap key of the goal's subject (its head constant and projection path) with each callee rule's before unifying, so it skips a rule that cannot match without trying it ([The Glue Phase](../generator/glue-phase.md#keyed-rule-application)). The same file carries `@[simp]` projection rules for the raw probes (`co_schedEmbed_sr`, `co_probe2_rr`, …), which proofs use when they rewrite a machine run with the body's `sr` equations. [The `wf`, `smono` and `mono` Phases](../generator/wf-smono-mono.md) describes the phase that runs the walk.
