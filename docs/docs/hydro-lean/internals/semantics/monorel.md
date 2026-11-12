---
sidebar_position: 4
title: "Monotonicity by Instantiation (MonoRel)"
---

# Monotonicity by Instantiation (`MonoRel`)

**Monotonicity** of a module is the statement that growing its input **wires** (its collection-valued arguments and bindings) gives growing output wires, with the **decisions** (the explicit arguments standing for the runtime's choices at `nondet!` sites, [Nondeterminism and Decisions](../../using/programs/nondeterminism.md)) held fixed. Growth is measured in the grade's order `PoolLe` on streams ([Grades](./grades.md#the-growth-order)) and prefix on tick traces. Monotonicity is a theorem about the program in its own right, and other proofs depend on it. A knot's stages (the iterates `iterate body ⊥ m` of a `forward_ref` cycle's body at the denotational semantics `Values`, [Values](./values.md#knots)) form an ascending chain only if the body is monotone, and both knot invariants and the coupling ([The Coupling Corner](../corner/coupling.md)) rely on that chain. The step of a knot invariant is usually a monotonicity fact about the body module.

Proving monotonicity by induction over each program's structure, or by a tactic that walks the program composing per-operator lemmas, is per-program work whose cost grows with the program. Instead, Hydro Lean runs the program at an instance whose operators carry the proof, so the program proves its own monotonicity.

## The Relational Instance

A program is a function generic over the instance `H : HydroSem L mem`. `MonoRel L mem` (`hydro_lean/Hydro/MonoRel.lean`) is an instance whose carriers are pairs of `Values` carriers related by the growth order, and whose operators run the `Values` operator on both components and prove the relation for the output:

```lean
def MonoRel (L : Type) (mem : L → Nat) : HydroSem L mem where
  …
  Stream ℓ α _ ord ret :=
    {p : MemberMap (mem ℓ) (PoolCarrier α ord ret)
        × MemberMap (mem ℓ) (PoolCarrier α ord ret) //
      ∀ i, PoolLe ord ret (p.1 i) (p.2 i)}
  …
  map {_ℓ _α _β _ _ ord} s f :=
    ⟨(.mk' fun i => mapPool (ord := ord) (f i) (s.val.1 i),
      .mk' fun i => mapPool (ord := ord) (f i) (s.val.2 i)),
     fun i => pool_map_le (f i) (s.property i)⟩
```

Run a module at `MonoRel` on related input pairs and its output is a related pair. The first and second components are definitionally the module's `Values` runs on the two inputs, and `.property` is the monotonicity proof. No induction is involved and no per-program content: the preservation lemmas are proved once per operator, in the instance.

`MonoRel` reuses the `Values` decision vocabulary (`BatchDec n α ord ret := BatchCutsOf n α ord ret`, …), and both components receive the same decision. The theorem therefore states monotonicity at a fixed decision. That it holds at all depends on the denotation's blocking semantics:

```lean
  batch {_ℓ _α _ ord ret} s d :=
    match ord, ret, s, d with
    | .noOrder, .exactlyOnce, s, d =>
      ⟨(.mk' fun i => batchCuts (s.val.1 i) 0 (d i),
        .mk' fun i => batchCuts (s.val.2 i) 0 (d i)),
       fun i => batchCuts_le (s.property i) 0 (d i)⟩
    …
```

`batchCuts_le` holds because an illegal cut truncates the trace instead of being clamped. A larger pool admits more of the same cuts, so the batch trace grows by prefix. With clamping, tick 0's batch would change when the pool grew ([Values](./values.md#decisions-as-cuts)).

## Relations per Carrier

| carrier | relation | name |
|---|---|---|
| stream | `∀ i, PoolLe ord ret (a i) (b i)` | (inline) |
| keyed stream | `∀ i, KeyedPool.Le (a i) (b i)`, per-key `PoolLe` | (inline) |
| folded singleton | `∀ i d, a i d <+: b i d`, read-prefix at every cut | `SingRel` |
| tick-located wire | `∀ i, a i <+: b i`, trajectory prefix | `TickRel` |
| one tick's value inside a body | none | the `Values` operator |

The tick relation is trajectory prefix only. It says that the smaller run's realized ticks are the first ticks of the larger run, and nothing about the values at the larger run's extra ticks. Therefore tick bodies are unrestricted. The in-tick operators of `MonoRel` are defined as the `Values` operators unchanged (`bmap b f := (Values L mem).bmap b f`, and so on), and the tick former relates its two outputs through `ValuesTick.scan_le`, whose content is that a scan over a prefix of the input trace is a prefix of the scan:

```lean
  tick_scan sts ins outs t g init :=
    MonoTick.mk outs (ValuesTick.scan sts ins outs (MonoTick.fst ins t) g init)
      (ValuesTick.scan sts ins outs (MonoTick.snd ins t) g init)
      (ValuesTick.scan_le sts ins outs _ _ (MonoTick.le ins t) g init)
```

A tick body may use `bantiJoin`, `bfilterNotIn`, `bcount` or any other operator that is not monotone in its tick's content, and the module is still monotone. Each tick's emission is a function of that tick's inputs and the register, and a prefix of the ticks gives a prefix of the emissions. The same reasoning covers `defer_tick` (`init :: t`, prefix by `cons`) and `allTicks` (flatten or sum of a prefix).

Cross-tick ascent of values, such as a ballot that only grows, is a different property. It is a contract fact of the producing module, stated in its `ensures` clause ([Contracts](../../using/programs/contracts.md)), or the `Monotonic` bound of `fold_monotone` ([Grades](./grades.md#monotone-folds)). `MonoRel` does not provide it.

## The Generated Statement

For each output `i` of a module `M`, the generator (the `hydro def` command, [The Generator](../generator/overview.md)) emits `M_monoᵢ`. For `toy_relay` (from `#check`):

```lean
@toy_relay_mono₁ : ∀ {L : Type} {mem : L → ℕ} (ℓ : L)
  {inp inp' : HydroSem.Stream mem ℓ ℕ StrOrd.noOrder Retries.exactlyOnce} (dec : ToyRelayDec (Values L mem) (mem ℓ))
  (sched : ToyRelaySched (Values L mem) (mem ℓ)),
  (∀ (i : Fin (mem ℓ)), PoolLe StrOrd.noOrder Retries.exactlyOnce (MemberMap.get inp i) (MemberMap.get inp' i)) →
    ∀ (i : Fin (mem ℓ)),
      PoolLe StrOrd.noOrder Retries.exactlyOnce (MemberMap.get (toy_relay (Values L mem) ℓ inp dec sched).1 i)
        (MemberMap.get (toy_relay (Values L mem) ℓ inp' dec sched).1 i)
```

Every wire argument is doubled (`inp`, `inp'`) with one order hypothesis each, and decisions are shared. The proof is a single term. `genMono` (`Hydro/HydroGen.lean`) packs each wire pair with its hypothesis as `⟨(inp, inp'), hm⟩`, applies the module at `MonoRel`, and projects:

```lean
      let prf ← mkProjection (← mc.legAppAt monoI monoArgs leg) `property
      let stmt ← mkForallFVars
        (mc.ctxArgs ++ argBinders ++ hyps) concl
      -- the `MonoRel` projection is the proof (module-scale defeq, judged by the kernel)
      let val ← mkLambdaFVars (mc.ctxArgs ++ argBinders ++ hyps) prf
      addThm name stmt val
```

The statement is phrased over `toy_relay (Values L mem) …`, and the proof's type is phrased over `.val.1` and `.val.2` of `toy_relay (MonoRel L mem) …`. The kernel checks that the two agree definitionally, at module scale. [Proof Generation](../proof-generation.md) explains why such checks go to the kernel rather than to `Meta.isDefEq`.

## Knots

At `MonoRel` a knot is Kleene iteration on pairs:

```lean
  fix_stream {_ℓ _α _ ord ret} fuel body :=
    iterate body
      ⟨(.mk' fun _i => PoolBot ord ret, .mk' fun _i => PoolBot ord ret),
       fun _i => PoolLe.refl ord ret _⟩ fuel
```

The fuel is a variable. The first component of `iterate F x k` equals `iterate F₁ x₁ k` only by induction on `k`, not definitionally, so the module-scale projection fails at the first knot. Modules that reach a knot take two other routes.

- **The knot itself** gets `K_mono₁` from `iterate_mono_param`, which needs a body that preserves the relation, and the body's own `M_mono` lemma supplies it:

  ```lean
  theorem iterate_mono_param {τ : Type _} {R : τ → τ → Prop}
      {F F' : τ → τ} (hFF' : ∀ {a b}, R a b → R (F a) (F' b))
      {x x' : τ} (hx : R x x') :
      ∀ k, R (iterate F x k) (iterate F' x' k)
  ```

  `vmono_fix_stream` and `vmono_fix_tick` (`Hydro/ValuesMono.lean`) are this theorem at the `Values` spelling.
- **A module whose callees reach a knot** gets its `M_mono` compositionally (`genGlueMono` in `Hydro/HydroGenKnot.lean`): the callees' mono lemmas are resolved recursively and the remaining operators are closed by the per-operator `vmono_<op>` lemmas. These restate `MonoRel`'s preservation proofs at the instance spelling with two independent inputs, because an application of a lemma can unify only with that form. Unfolding the callees instead would put whole programs under the unifier.

## Uses of Monotonicity

The Kleene chain of a knot is `iterate_chain` (`Hydro/Trace.lean`). Its body-preservation premise is discharged by the body's `M_mono`, and the generator states the result per knot:

```lean
@toy_loop.w.stages_mono : ∀ {L : Type} {mem : L → ℕ} (ℓ : L)
  (inp : HydroSem.Stream mem ℓ ℕ StrOrd.noOrder Retries.exactlyOnce) (tdec : ToyLoopDec (Values L mem) (mem ℓ))
  (sched : ToyStepSched (Values L mem) (mem ℓ)) {m m' : ℕ},
  m ≤ m' →
    ∀ (i : Fin (mem ℓ)),
      PoolLe StrOrd.noOrder Retries.exactlyOnce (MemberMap.get (toy_loop.w.stages ℓ inp tdec sched m) i)
        (MemberMap.get (toy_loop.w.stages ℓ inp tdec sched m') i)
```

A knot invariant's obligations are often monotonicity facts about the body module. In `toy_loop` (`Hydro/HydroGenToy.lean`) both the base and the step obligation of the chain invariant are applications of the body's mono lemma:

```lean
  fix (w : H.Stream ℓ Nat .noOrder .exactlyOnce) via tdec.fuel
    invariant w (out out') => ∀ i, out i ≤ out' i,
      base := fun _htop => toy_step_mono₁ (inp := inp) (inp' := inp)
        ℓ tdec.relay sched
        (fun _ => PoolLe.bot_le _ _ _) (fun _ => PoolLe.refl _ _ _),
      step := fun _htop _hw hbw _hbw0 _ih => toy_step_mono₁ (inp := inp)
        (inp' := inp) ℓ tdec.relay sched hbw
        (fun _ => PoolLe.refl _ _ _)
```

[Knot Invariants](../../using/proving/knot-invariants.md) explains the `base`/`step` form. The coupling uses the same chain to relate a machine knot to the denotation's iterate.

## Three Monotonicity Facts

The framework uses "mono" for three different facts:

| lemma | instance | statement | proved by |
|---|---|---|---|
| `M_monoᵢ` | `Values` | two runs, inputs related by the growth order, same decisions: outputs related | `MonoRel` projection, or compositionally past knots |
| `K_mono₁` | `Values`, one knot | the knot at related inputs is related | `iterate_mono_param` over the body's `M_mono` |
| `M_smonoᵢ` | `SchedSem` | one machine run: a realized tick never changes, `TMono` | `tmono_<op>` and `tmono_fix_tick` ([The Step Machine](./sched.md#causality-monotonicity-and-pacing)) |

`M_smono` is about the machine step clock within one run. `M_mono` is about two denotational runs at different inputs. They have no logical connection. Both feed the coupling: `M_mono` through the Kleene chain, `M_smono` through the corner's record that tick traces only grow ([The Coupling Corner](../corner/coupling.md)).
