---
sidebar_position: 4
---

# The `tick` Elaborator

A Rust `sliced!` block with `use::state` runs a per-tick body over this tick's batches and the registers' previous values, and hands the next register values to the following tick. The Lean surface for it is the `tick` block, elaborated by `hydro_lean/Hydro/HydroTick.lean` into one application of a single operator of the signature `HydroSem`, the class every program is generic over. Every **interpretation** (the denotational semantics `Values`, the operational semantics `SchedSem`, the corner `CoupleSem`, the monotonicity instance `MonoRel`; [Tagless-Final Programs](../approach.md#tagless-final-programs)) and every generator phase ([Generator Overview](./overview.md)) therefore handles the block once, not once per block shape. The elaborator also emits facts about the block (what tick `n` emitted, what the register held before tick `n`), so the proofs inside the program never unfold a fold whose state is a tuple of shaped carriers.

How you use the readers it emits and how you state a block invariant are on [Tick Blocks and Their Invariants](../../using/proving/tick-invariants.md); what `tick_scan` computes in each interpretation is on [Values](../semantics/values.md).

## Registers as Structural Folds

A register read inside the body is the previous tick's value, so a `tick` block is syntactically a cycle: `cycle_with_initial`, `defer_tick` and the body. The obvious encoding is therefore the `fix` construct over tick traces, a **knot** (a cycle wire hoisted as a constant whose denotation is iterated a fuel's number of times, [Elaborating `hydro def`](./elaborating-hydro-def.md#hoisting-the-knots)). That encoding carries the whole per-knot proof stack for every block ([The Knot Stack](./knot-stack.md): a `FixDec` fuel per block, `K_co_wf₁`, the stage vocabulary), and a register invariant becomes a two-axis induction over the Kleene stages and the tick prefixes.

Hydro compiles no instantaneous cycle: every cycle passes a `defer_tick` or a network boundary, and in a `tick` block the one-tick delay is built into the construct's shape. A tick register is therefore not a fixpoint. It is a **structural fold over the location's tick index**, and the elaborator emits that: one application of the signature field `tick_scan`, with no fuel, no knot and no stage index. `fix` stays the construct for cycles through the network, where the iteration is over exchange rounds.

## The `tick_scan` Former

`tick_scan` is one field of `HydroSem` (`Hydro/Sem.lean`). The states, the inputs and the emissions are each a `TickShape`, a tree of leaves zipped by `pair`, and the **carriers** (the types an interpretation gives its wires) are computed from the shape at the type level:

```lean
inductive TickShape : Type 1 where
  /-- A singleton wire's slice: this tick's value. -/
  | sing (τ : Type)
  /-- A bounded-stream wire's slice: this tick's batch. -/
  | stream (α : Type) (inst : DecidableEq α) (ord : StrOrd) (ret : Retries)
  …
  /-- Zip. -/
  | pair (a b : TickShape)
```

```lean
  tick_scan : ∀ {ℓ : L} (sts ins outs : TickShape),
    AtomicOf Atomic AtomicStream AtomicKeyedSingleton AtomicKeyedStream ℓ ins →
    (Fin (mem ℓ) → BoundedOf BoundedSingleton BoundedStream BoundedKeyedSingleton BoundedKeyedStream sts →
      BoundedOf BoundedSingleton BoundedStream BoundedKeyedSingleton BoundedKeyedStream ins →
      BoundedOf BoundedSingleton BoundedStream BoundedKeyedSingleton BoundedKeyedStream sts
        × BoundedOf BoundedSingleton BoundedStream BoundedKeyedSingleton BoundedKeyedStream outs) →
    SeedOf sts → AtomicOf Atomic AtomicStream AtomicKeyedSingleton AtomicKeyedStream ℓ outs
```

`AtomicOf … ℓ sh` is the tuple of wires a shape describes (`Atomic ℓ τ` for a `.sing τ` leaf, `AtomicStream ℓ α ord ret` for a `.stream` leaf, products for `pair`); `BoundedOf … sh` is this tick's values of the same shape; `SeedOf sh` is a value for a register leaf and `Unit` for a stream register, which starts empty. Because the arity lives in the shape and not in the field, the interpretations, the coupling instance's projection lemmas ([Projections and Tactics](../corner/projections-and-tactics.md)), the step-monotonicity law (`tmono_tick_scan`) and the graph node (`Node.tick_scan`) are each written once.

## The Desugaring

The elaborator is a syntax transform. It never analyses an operator application; the block tail (`rebind`/`emit`/`yield`/`kyield`) is located by marker-level surgery (`replaceTail`) and replaced by the step's result tuple. The module docstring states the output:

```lean
    let x₁ := <wire₁> … let xₘ := <wireₘ>
    let <out₁>_step := fun me st inpt =>
      let r₁ := st.1 …                        -- the states (a shaped tuple)
      let x₁ := inpt.1 …                      -- this tick's inputs
      <body>
      ((e₁, (…)), (a, (b, …)))                -- next states, emissions
    let <out₁>_r := H.tick_scan <sts> <ins> <outs> (x₁, …) <out₁>_step (seed₁, …)
    let a : H.Atomic _ _ := <out₁>_r.1 …      -- the output wires
    ghost have h<out>_run : … ; ghost have h<out>_at : … ; (ghost have h<out₁>_reg : …)?
    <continuation>
```

For `toy_relay` (`Hydro/HydroGenToy.lean`), a stateless block with two inputs and one `yield`:

```lean
  tick (input bb := b) (input g := gate) :=
    yield (emitted := H.bfilterIf bb (H.boIsSome (H.bfirst g)));
```

the elaborated module value (`#print toy_relay`, the lines from the inputs to the outputs):

```lean
  let bb := b;
  let g := gate;
  let emitted_step := fun me st inpt =>
    have bb := inpt.1;
    have g := inpt.2;
    (HydroSem.bsPure (), HydroSem.bfilterIf bb (HydroSem.boIsSome (HydroSem.bfirst g)));
  let emitted_r :=
    HydroSem.tick_scan (TickShape.sing Unit)
      ((TickShape.stream ℕ instDecidableEqNat StrOrd.noOrder Retries.exactlyOnce).pair
        (TickShape.stream Unit instDecidableEqPUnit StrOrd.totalOrder Retries.atLeastOnce))
      (TickShape.stream ℕ instDecidableEqNat StrOrd.noOrder Retries.exactlyOnce) (bb, g) emitted_step ();
  let emitted := emitted_r;
```

A stateless block carries the unit register: the state shape is `TickShape.sing Unit`, the rebind tuple is `H.bsPure ()` and the seed is `()`. The two inputs are opened from `inpt` by projection; with one input the step's binder is the input's own name.

### Type-Directed Shapes

Two steps are type-directed. Each input's shape is read off its wire's type:

```lean
def tickInputShape (xId : Ident) (wire : TSyntax `term) : TermElabM Expr := do
  let e ← withSynthesize (postpone := .yes) <| elabTerm wire none
  let ty ← instantiateMVars (← inferType e)
  let ty ← whnfR ty
  match ty.getAppFn.constName?, ty.getAppArgs with
  | some ``HydroSem.Atomic, #[_, _, _, _, τ] =>
    pure (mkApp (mkConst ``TickShape.sing) τ)
  | some ``HydroSem.AtomicStream, #[_, _, _, _, α, inst, ord, ret] =>
    pure (mkApp4 (mkConst ``TickShape.stream) α inst ord ret)
  …
```

and each state's kind is read off its written type (`parseTickState`: `H.BoundedStream …` is a stream state seeded empty, `H.BoundedKeyedStream …` and `H.BoundedKeyedSingleton …` likewise, anything else a register that needs `:= seed`). The output shape's structure comes from the keywords and its leaves are holes:

```lean
    for _ in emits do outShapes := outShapes.push (← `(TickShape.sing _))
    for _ in yields do outShapes := outShapes.push (← `(TickShape.stream _ _ _ _))
    for _ in kyields do outShapes := outShapes.push (← `(TickShape.kstream _ _ _ _ _ _))
```

Unification against the body's result type fills them. The output wires are then re-ascribed with their leaf carrier types:

```lean
        let ty : TSyntax `term ←
          if k < emits.size then `(($hId).Atomic _ _)
          else if k < emits.size + yields.size then `(($hId).AtomicStream _ _ _ _)
          else `(($hId).AtomicKeyedStream _ _ _ _ _)
```

The ascription is definitionally a no-op but required: without it a downstream wire's type is `AtomicOf … ℓ (.sing _)`, and every type-directed consumer (the generator's binder classification `classifyArg`, its output peeling `peelLegsTy`, the project walker's `kindOfType`) dispatches on a carrier head constant and would see none.

### The Step Lambda

`mkStepLam` builds `fun me st inpt => …`. With several states the register is one shaped tuple typed `BoundedOf … stsShape` and opened into the declared names by projection; a register's in-tick carrier is `H.BoundedSingleton σ`, a persisted stream's is its written `H.BoundedStream …`. The body therefore uses the in-tick operators (`H.bsZip`, `H.bsMap`, `H.bcount`, …) on the declared names, as the Rust body uses `.zip(…).map(…)` on its slices. The seeds become the shape's `SeedOf` tuple, and the register's seed at `Values` is `ValuesTick.seed stsShape seeds` when a stream state is present, because a stream state starts as the empty pool, which is not a written term.

## The Readers

Every block gets **ghost** facts about itself (proof-only facts, stripped from the program), stated under the `Values` substitution, where the fold is definitional: `h<out>_run` (the output equals the fold of the step over the zipped inputs), `h<out>_at` (tick `n`'s emission, one read per input), `h<out₁>_reg` for a stateful block (the register, existentially abstracted) and, with an `invariant` clause, `h<out>_inv` and `h<out>_inv_take`. Their statements and how a proof consumes them are on [The readers](../../using/proving/tick-invariants.md#the-readers). They are `ghost have`s, so each is hoisted to a theorem `<M>.ghost.h<out>_run` and replayed by name; the `let`s in the hoisted statement are what lets a replayed reader mention the wires by their program names ([Elaborating `hydro def`](./elaborating-hydro-def.md#the-ghost-layer)). Below, `S n` is the register before tick `n`, `scanAcrossTicksState (<out>_step i) seed ((Z i).take n)` with `Z i` the zipped input trace `Trace.zip (x₁ i) (… (xₘ i))`.

### Proving the Readers

At `Values` the former is definitionally the fold, but the elaborator never proves a reader by `show`ing the zipped form. That would make the unifier reduce the whole shaped former, `AtomicOf`/`BoundedOf` recursion included. The run reader is closed by one-step rewrite rules instead:

```lean
    -- the simp set that reads the former at the denotation: one-step rewrite
    -- rules, never a definitional `show` of the zipped form, which would reduce
    -- the whole shaped former
    let readerSimp : TSyntax `tactic ← `(tactic|
      simp only [$outLemmas,*, $resId:ident, ValuesTick.tick_scan_stream,
        ValuesTick.tick_scan_sing, ValuesTick.tick_scan_ksing, ValuesTick.tick_scan_kstream,
        ValuesTick.tick_scan_pair, ValuesTick.unslice_sing,
        …
        ValuesTick.seed_sing, ValuesTick.seed_pair])
```

and the run proof tries, in order, `readerSimp; done`, `rfl`, `readerSimp; rfl`, and `readerSimp; simp only [List.map_map]; rfl`. The `_at` reader's proof rewrites with the run reader (`rw [h<out>_run i]`), then applies `scanAcrossTicksTrace_getElem?_eq_some` (tick `n` of a fold equals the step on the state before `n`), then splits the zipped read into one read per input by `Trace.getElem?_zip_eq_some'`. The split is what makes the reader per input: a consumer destructures `(x₁ i)[n]? = some x₁_t` and `(x₂ i)[n]? = some x₂_t`, never a read of a nested tuple.

### The Abstract Register

`mkRegGhost` states the register as an existential, `∃ reg, reg i 0 = seed ∧ (the _at clause over reg i n) ∧ (one step) ∧ (one stall per input) [∧ the invariant before every tick]`, so a consumer that `ghost obtain`s it works with `reg i n` and never sees `scanAcrossTicksState` or the zip. The witness is the fold's own register, `fun i n => S n`. The step clause is proved by `scanAcrossTicksState_take_succ?`, and each stall clause (once input `xⱼ` has ended, the register freezes) by `scanAcrossTicksState_take_stall` on the zip's `none` read. The stall clauses exist because the zipped trace ends when its shortest input ends: without them a consumer reasoning about tick `n` past one input's end would have to reason about the zip. The read clauses reassemble one zip read from the component reads with `Trace.getElem?_zip_eq_some'` (`zipReadOf`).

## The Invariant Clause

What a block invariant is and how its two obligations are written is on [The invariant normal form](../../using/proving/tick-invariants.md#the-invariant-normal-form). The elaborator ascribes both obligations with their full types before elaborating them, so the user's terms see concrete expected types. `tick` is stated over this tick's reads, one per input, never the zipped tuple:

```lean
        let tickCore : TSyntax `term ←
          `(($outId).length = $nId → $p2 $outId $stVId →
            $p2 (List.append $outId [@Prod.snd $stTy $outElemTy (($stepId $iId) $stVId
                $tickTuple)])
              (@Prod.fst $stTy $outElemTy (($stepId $iId) $stVId $tickTuple)))
        let tickTy : TSyntax `term ←
          `(∀ $iId:ident ($nId : Nat) ($outId : List $outElemTy) ($stVId : $stTy)
              $tickBinders*, $(← tickHyps tickCore))
```

The library lemma `scanAcrossTicks_invariant?` (`Hydro/Trace.lean`) takes the step with one read of the zipped input. A generated `bridge` lambda splits that read into the component reads and calls the user's `tick`, so the obligation never mentions the tuple. The invariant becomes `h<out>_inv` (by `scanAcrossTicks_invariant?`) and `h<out>_inv_take` (by `scanAcrossTicks_invariant_take?`), and is folded into `h<out₁>_reg` as its last clause over the abstract register.

The output binder must be typed `(out : List τ)`, because `τ` is spelled into the obligation as `@Prod.snd $stTy $outElemTy` above: at `Values` the step's components are `BoundedSingleton τ`, only definitionally `τ`, and the obligations' simp lemmas need the plain type to fire. A stream state's type is left to unification (`_`), because its written type names `H`, which no longer exists under the ghost's `Values` substitution.

## Error Messages

The block's shape is checked before anything elaborates:

```text
tick: `rebind` must assign every `state` of this block, in declaration order (#[acc, cnt]) — and only those
tick: input `x` must be an `Atomic`, `AtomicStream`, `AtomicKeyedSingleton` or `AtomicKeyedStream` wire (its type is …)
tick: a register state needs its seed (`:= seed`)
tick: a stream state is seeded empty — no `:= seed`
tick invariant: available on single-output blocks (one `emit` or one `yield` leg; v2)
tick: an `invariant` clause needs its trailing `prove init := …, tick := …`
```

## Generator Phases on `tick_scan`

No phase has a case specific to `tick`. Every phase treats `tick_scan` as one more operator: the couple phase's projection set reaches a projected `tick_scan` at the coupling instance through the `coTickScanProj` simproc (a lemma's binder types cannot meet a projected former's spelling, so the definitional right-hand side is computed instead); the step-monotonicity walk restates a `TMono (tick_scan …)` goal as `TMonoOf outs …` and applies `tmono_tick_scan` (`co_smono_step`, `Hydro/WfTactics.lean`); the project phase emits one `Node.tick_scan` with the input tuple as a shaped tree of variables. A glue module whose body contains a `tick_scan` takes the full projection set into its simp pass, because the simproc and the in-tick rewrites under the step's binders are out of reach of the keyed residual pass ([Glue Phase](./glue-phase.md)).
