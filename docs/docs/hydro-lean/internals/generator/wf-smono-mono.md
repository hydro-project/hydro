---
sidebar_position: 8
title: "The wf, smono and mono Phases"
---

# The `wf`, `smono` and `mono` Phases

The `sr` and `rr` equations of [the couple phase](./couple-phase.md) and [the glue phase](./glue-phase.md) identify the machine run and the denotational run of each output of a module's corner run. Before a contract can become a machine theorem, or a proof can discharge a knot's residual, the generator needs three more facts per output `i`:

| Lemma | Statement, informally | Consumed by |
|---|---|---|
| `M_co_wfᵢ` | if every input wire's residual holds (and the module's `requires` premise about its inputs' denotations holds), output `i`'s residual holds at the corner | the output's coupling `cpl`, which needs the residual before it says anything |
| `M_smonoᵢ` | if every tick-typed machine input only grows, so does output `i` | the `wf` of a tick knot whose body calls `M` |
| `M_monoᵢ` | if every input at `Values` grows, output `i` grows | knot `invariant` obligations, and the `mono` lemma of every knot whose body calls `M` |

Each is logically shallow: an induction-free walk over the module's operators, one per-operator law at each. Their difficulty is cost. Unfolding a callee multiplies the goal by its size, and unfolding through a knot multiplies it again per nesting level ([Elaboration and Kernel Cost](../cost-model.md)), so no step of these phases unfolds a callee or a knot. The three phases solve that three ways: the residual by a script whose closers apply callee lemmas only after a syntactic check that they can match; step monotonicity by a walk that dispatches on the head operator instead of trying rules; and monotonicity with no tactic at all, by running the program at the relational instance `MonoRel`, where the statement is a projection of the run.

## Well-Formedness at the Corner

Every corner carrier has a proposition `wf` and a coupling `cpl` that holds under it (`hydro_lean/Hydro/Couple.lean`). For a stream, `cpl` says that at every step `k` with `k + lag ≤ T`, the machine's view sits below the denotation's pool in the order of the stream's **grade** (its ordering and retry guarantees `ord` and `ret`, [Grades](../semantics/grades.md)), where `T` is the step the coupling is taken at and the **lag** is how far below `T` it reaches:

```lean
structure CoStream (T : Nat) (n : Nat) (α : Type) [DecidableEq α]
    (ord : StrOrd) (ret : Retries) : Type where
  sr : Fin n → StepHist α
  rr : MemberMap n (PoolCarrier α ord ret)
  …
  lag : Nat
  wf : Prop
  cpl : wf → ∀ k, k + lag ≤ T → ∀ i, ListLe ord ret ((sr i).view k) (rr i)
```

Each operator of `CoupleSem` builds its output's `wf` from its inputs', and for most operators it is their conjunction (`co_union_wf`). The exception is a conditional fold. A stream's denotation carries its nondeterminism unresolved, and a fold resolves it without a decision only by giving one result for every history the stream captures; the fold's commutativity proof (`FoldOk`) guarantees that ([Grades](../semantics/grades.md)). A closure that commutes only on pairs satisfying a prerequisite `Pre` gives one result only on streams whose values pairwise satisfy `Pre`. Whether the values satisfy `Pre` is a fact about content, and a program written against an abstract `H` has no content to prove it from, so the fold's `wf` carries it as a conjunct for the program to discharge at the denotation (`Hydro/CoupleProj.lean`):

```lean
theorem co_reduce_watermark_wf {K V : Type} [DecidableEq K] [Hashable K] [DecidableEq V] [LinearOrder K]
    …
    ((CoupleSem L mem pacing Tc Td hjT).reduce_watermark ks wm f Pre ok).wf
      = (ks.wf ∧ wm.wf ∧ ∀ i, (KeyedPool.accum (ks.rr i)).Pairwise Pre) := rfl
```

A module's output `wf` is therefore a formula over its whole graph: input `wf`s, the pairwise facts of its conditional folds, the components of its knots' `wf`s, and the `requires` clauses of its callees. `M_co_wfᵢ` reduces that formula to the inputs: its hypotheses are one `wf` per input wire, which the generated statements spell `inp.4` (the raw projection), and the module's `requires` (`toy_pre_co_wf₁` below shows both). The generator puts each hypothesis through `whnfR`, which produces the raw `.proj` node that the `wf` normalizer leaves in goals, so that a reducible `assumption` closes those goals by syntactic match. The end-to-end check `toy_safe_sched` (`Hydro/HydroGenCheck.lean`) applies `toy_relay_co_wf₁` to its input's `wf` (`trivial` for an input boundary) and passes the result to the output's `cpl`.

## The `wf` Script

`genWf` (`Hydro/HydroGen.lean`) states one lemma per output and proves it with a tactic script built as a string and elaborated by `proveByTac` ([The Generator](./overview.md) covers proof by script). The script, with `M` the module, `extras` its inline hints, and the bracketed lists filled from the callees' registered lemmas:

```lean
        s!"(intros; co_wf_simp [{modList}{rulesSuffix mc.extras}]; " ++
        -- `assumption` and `⟨⟩`-splitting at default transparency whnf through folded
        -- callee deltas, so every alternative that unifies stays reducible; the two
        -- default-transparency closers come last
        s!"repeat' first | exact trivial \
           | (with_reducible assumption) \
           | (with_reducible refine And.intro ?_ ?_)\
           {knotAlts}{closeAlt}{preAlt} \
           | (co_wf_simp [{modList}{rulesSuffix mc.extras}]) \
           | (co_simp [{modList}{rulesSuffix mc.extras}]) " ++
        "| assumption | refine ⟨?_, ?_⟩)")
```

For `toy_pre` (below; no callees) `HYDROGEN_TAC_DIAG` prints the instance:

```text
(intros; co_wf_simp [Hydro.toy_pre]; repeat' first | exact trivial | (with_reducible assumption) | (with_reducible refine And.intro ?_ ?_) | (hydro_pre_close [Hydro.toy_pre.pre.hpre]      ) | (co_wf_simp [Hydro.toy_pre]) | (co_simp [Hydro.toy_pre]) | assumption | refine ⟨?_, ?_⟩)
```

In general `knotAlts` and `closeAlt` are `| (hydro_rule_apply [ws])` and `| (hydro_wf_close [ws])` with `ws` the callees' `wf` lemmas, and `preAlt` is the `hydro_pre_close` alternative below. Step by step:

- `co_wf_simp [M]` unfolds only `M`. Its simp set (`coWfSimpRules`, `Hydro/CoupleProj.lean`) holds one `rfl` lemma `co_<op>_wf` per operator, so the goal becomes a conjunction of input `wf` projections, callee `wf` projections such as `(collect_quorum … ).1.wf`, pairwise facts, and `True` for every boundary constructor.
- `exact trivial`, reducible `assumption` and reducible `And.intro` close the `True`s and the inputs (against the `whnfR`'d hypotheses) and split the conjunctions.
- `hydro_rule_apply` and `hydro_wf_close` close a callee's `wf` projection by the callee's own `C_co_wfⱼ` or a knot's `K_co_wf₁`, so every callee stays folded; unfolding it would force every module to carry the closer lists of all its transitive callees. `hydro_rule_apply` is the keyed `apply` of every generated walk: it skips a rule whose conclusion cannot match the goal by head and literal projection path, before any unification ([The Glue Phase](./glue-phase.md)).
- A callee's lemma opens the callee's hypotheses as goals, the `wf` of its input wires spelled over this module's operators, so the loop runs `co_wf_simp` (and `co_simp`) again.
- Default-transparency `assumption` and `⟨?_, ?_⟩` come last, because at default transparency both whnf through the deltas of folded callees.

### `hydro_wf_close`

Some callee goals need default transparency: a callee whose output is a contract subtype carries matcher spellings that reducible unification cannot cross. Default-transparency unification against an unrelated goal deltas whole modules, so `hydro_wf_close` refuses to `apply` a lemma unless two syntactic checks pass. The goal must be a `wf` projection whose subject is headed by the lemma's module, and the projection path down to that module (the output) must equal the lemma's:

```lean
  let goalHead := headOf subj
  for id in ids.getElems do
    let n ← realizeGlobalConstNoOverload id
    -- the lemma's module: `M_co_wfᵢ` → `M` (the generated naming scheme)
    let calleeStr := (n.toString.splitOn "_co_wf").head!
    let callee := calleeStr.toName
    if goalHead == some callee then
      -- the leg check: the lemma's conclusion subject path must equal the goal's
      …
      unless lemPath == goalPath do continue
      let ok ← tryTactic <| evalTactic
        (← `(tactic| apply $(mkIdent n)))
      if ok then return
```

`headOf` strips `.proj`, `fst`, `snd` and `val` and returns the first constant; `pathOf` records the same steps as numbers (`fst ↦ 0`, `snd ↦ 1`, `val ↦ 100`). A callee that occurs only inside an argument is not a candidate. If `hydro_wf_close` matched by occurrence instead, the goal `(collect_quorum (… kv_replica (… paxos_with_client …) …)).1.wf` would try `kv_replica_co_wf₁`, whose `apply` unifies two module applications through their knots: 3.6 billion heartbeats and 318 s for one output, against 8.6 million and 0.8 s with the head rule.

### Content Facts and `hydro_pre_close`

The pairwise conjunct of a conditional fold, and a callee's `requires` clause, are facts about content. The program proves them where it has content, at `Values`, before the site: `ghost pre h : S := prf` ([Proofs over Values](../../using/proving/proofs-over-values.md)). `toy_pre` (`Hydro/HydroTickCheck.lean`) has the shape of the Paxos acceptor's log merge, a `reduce_watermark` whose combiner commutes only on pairs that agree under `ToyTie`:

```lean
hydro def toy_pre (H : HydroSem L mem) (ℓ : L)
    …
  requires (∀ i, ∀ a ∈ inp i, ∀ b ∈ inp i, a.1 = b.1 → ToyTie a.2 b.2)
  …
  let batched := H.batch inp dec
  let keyed := H.tinto_keyed batched
  …
  ghost pre hpre : ∀ i, (KeyedPool.accum (keyed i)).Pairwise ToyTie :=
    fun i => KeyedPool.accum_tinto_keyed_batch_pairwise inp dec i (hreq i)
  let m := H.reduce_watermark keyed wm toyMerge ToyTie toyMerge_comm
```

The `ghost pre` becomes a closed theorem over the module's `Values` binders, with the let-chain up to the site as `let`s and the conclusion wrapped in `PreFact`:

```lean
@toy_pre.pre.hpre : ∀ {L : Type} {mem : L → ℕ} (ℓ : L)
  (inp : HydroSem.Stream mem ℓ (ℕ × ℕ × ℕ) StrOrd.noOrder Retries.exactlyOnce)
  (dec : HydroSem.BatchDec mem (mem ℓ) (ℕ × ℕ × ℕ) StrOrd.noOrder Retries.exactlyOnce),
  let batched := HydroSem.batch inp dec;
  let keyed := HydroSem.tinto_keyed batched;
  PreFact (∀ (i : Fin (mem ℓ)), ∀ a ∈ MemberMap.get inp i, ∀ b ∈ MemberMap.get inp i, a.1 = b.1 → ToyTie a.2 b.2)
    (∀ (i : Fin (mem ℓ)), KeyedPool.Pairwise ToyTie (KeyedPool.accum (MemberMap.get keyed i)))
```

`PreFact R S` is `R → S` wrapped in its own `def` (`Hydro/Sem.lean`), so the closer finds the split between the clause `R` and the fact `S` without guessing where an implication chain ends. The module's own `requires` becomes a trailing hypothesis `hreq` of every `M_co_wfᵢ`, stated over the inputs' `rr` components, and it is the `R` of every `ghost pre` in the module:

```lean
@toy_pre_co_wf₁ : ∀ {L : Type} {mem : L → ℕ} {pacing : Pacing L mem} {Tc Td : ℕ} {hjT : Tc.le Td} (ℓ : L)
  (inp : HydroSem.Stream mem ℓ (ℕ × ℕ × ℕ) StrOrd.noOrder Retries.exactlyOnce) (wm : HydroSem.Atomic mem ℓ (Option ℕ))
  (dec : HydroSem.BatchDec mem (mem ℓ) (ℕ × ℕ × ℕ) StrOrd.noOrder Retries.exactlyOnce),
  inp.4 →
    wm.4 →
      (∀ (i : Fin (mem ℓ)), ∀ a ∈ inp.rr.get i, ∀ b ∈ inp.rr.get i, a.1 = b.1 → ToyTie a.2 b.2) →
        (toy_pre (CoupleSem L mem pacing Tc Td hjT) ℓ inp wm dec).wf
```

A caller discharges `hreq` by a `ghost pre` of its own (`toy_pre_caller`).

`hydro_pre_close [M.pre.h₁, …] rw [callee rr lemmas]` tries each fact on the goal. It turns the fact's binders into metavariables and zeta-reduces its `let`s, down to `PreFact R S`. It unifies `S` with the goal, closes `R` by a hypothesis in scope (the lemma's `hreq`) or as `True`, and assigns any decision field the fact does not mention an arbitrary inhabitant (the theorem is parametric in it). A decision-record binder (a structure of the module's `nondet!` inputs, [The `dec` and `sched` Records](../../using/programs/nondeterminism.md#the-dec-and-sched-records)) `dec : D (Values L mem) n` becomes a constructor application over fresh field metavariables (`decRecordMVar?`), not one metavariable, because the goal spells each field through its derived decision, the decision the corner computes from the machine run (`dec.batch ↦ batchDerive …`, [The Coupling Corner](../corner/coupling.md)). A projection of a bare metavariable never unifies with a term, while a projection of a constructor reduces to the field's metavariable.

On `toy_pre`, the goal that `co_wf_simp` leaves for the fold's conjunct is about the corner, while the fact is about `Values` (`HYDROGEN_PRE_DIAG` output):

```text
[hydro_pre_close] pass 1 (raw, reducible) on: ∀ (i : Fin (mem✝ ℓ✝)),
  KeyedPool.Pairwise ToyTie (KeyedPool.accum ((HydroSem.tinto_keyed (HydroSem.batch inp✝ dec✝)).rr.get i))
[hydro_pre_close]   Hydro.toy_pre.pre.hpre: hydro_pre_close: Hydro.toy_pre.pre.hpre does not match the goal
[hydro_pre_close] pass 3 (default transparency) closed
```

The closer makes three passes: the raw goal at reducible transparency; the goal rewritten by the callees' `rr` equations, which move a corner-side `(C corner args).rr` to `Values`, where the facts live, again reducible; and the rewritten goal at default transparency, last, because it deltas the callee modules on both sides. A conditional fold with no matching `ghost pre` leaves its pairwise goal open, the script fails with `HydroGen: unsolved goals (n) under …` followed by the script and the open goals, and the module does not elaborate. With `HYDROGEN_PRE_DIAG` set, each pass and each fact's failure also print to stderr, because a failing `first` alternative's messages are discarded.

## Step Monotonicity

A tick-family carrier of the machine (`Atomic`, `AtomicStream`, and the keyed forms) is `Fin n → Nat → Trace σ`: per member of the location (one process each), per step, the ticks realized so far. The machine never rewrites a realized tick, but the type does not say so, and the corner needs that property: the corner derives a per-tick order decision at `Values` from the tick list the machine holds at the last step, and the coupling at an earlier step must know that list extends the one the machine held then ([The Coupling Corner](../corner/coupling.md)). So every tick carrier of the corner records `mono : wf → TMono sr`, the `wf` of a tick knot (a knot over a tick-typed wire) requires its body to preserve `TMono`, and `M_smonoᵢ` supplies that fact for every module the body calls (`Hydro/SchedMono.lean`):

```lean
def TMono {n : Nat} {σ : Type} (x : Fin n → Nat → Trace σ) : Prop :=
  ∀ i k k', k ≤ k' → x i k <+: x i k'

/-- `TMono` leafwise over a shaped tick tuple. -/
@[reducible] def TMonoOf {L : Type} {mem : L → Nat} {ℓ : L} : (sh : TickShape) →
    AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) (SchedTick.TKStr mem) ℓ sh → Prop
  | .sing _, x => TMono x
  | .stream _ _ _ _, x => TMono x
  | .ksing _ _ _ _, x => TMono x
  | .kstream _ _ _ _ _ _, x => TMono x
  | .pair a b, x => TMonoOf a x.1 ∧ TMonoOf b x.2
```

`genSMono` states the fact for every output and every wire, at `SchedSem`, with the wires as implicit binders and one hypothesis per wire. A stream output or wire gets the placeholder `TMonoTriv x := True`, so that every wire binder is still determined by unification from its hypothesis when a caller applies the lemma. `toy_relay`'s second output is a tick stream and its input a plain stream:

```lean
@toy_relay_smono₂ : ∀ {L : Type} {mem : L → ℕ} {pacing : Pacing L mem} (ℓ : L)
  {inp : HydroSem.Stream mem ℓ ℕ StrOrd.noOrder Retries.exactlyOnce} (dec : ToyRelayDec (SchedSem L mem pacing) (mem ℓ))
  (sched : ToyRelaySched (SchedSem L mem pacing) (mem ℓ)),
  TMonoTriv inp → TMono (toy_relay (SchedSem L mem pacing) ℓ inp dec sched).2
```

A non-tick output is closed by `(intros; exact TMonoTriv.intro _)`. A tick output is closed by `(intros; simp only [M]; hydro_smono_walk [callee smono lemmas])`, and the walk is a `repeat'` over an ordered list of alternatives (`Hydro/HydroGen.lean`):

```lean
macro "hydro_smono_walk" "[" rules:ident,* "]" : tactic => do
  …
  let base : Array (Lean.TSyntax `tactic) := #[
    ← `(tactic| with_reducible assumption),
    ← `(tactic| with_reducible exact TMonoTriv.intro _)]
  let steps : Array (Lean.TSyntax `tactic) := #[
    ← `(tactic| co_smono_step),
    ← `(tactic| (with_reducible refine And.intro ?_ ?_)),
    ← `(tactic| split),
    ← `(tactic| dsimp only [])]
  let alts := base ++ userRws ++ steps
```

`userRws` is `hydro_rule_apply [callee smono lemmas]`, ahead of the per-operator step because a folded callee application must be closed by its own lemma, never opened; the keyed test is what keeps a wrong rule from costing seconds against a deep projection spine ([Keyed Rule Application](./glue-phase.md#keyed-rule-application)).

`co_smono_step` (`Hydro/WfTactics.lean`) applies one `tmono_<op>` law per head operator, from a table, with no `whnf` on a failing branch; the table and the reason it dispatches instead of trying rules are on [Projections and Tactics](../corner/projections-and-tactics.md#step-monotonicity-by-dispatch).

A tick block is one former, `tick_scan sts ins outs x g init`, whose inputs and outputs are tuples typed by `TickShape`s ([The Tick Elaborator](./tick-elaborator.md)), and its law concludes `TMonoOf outs (tick_scan …)` from `TMonoOf ins x`. That hypothesis asks about a tuple, so the walk meets goals `TMonoOf a s.1` about one component and steps them by `TMonoOf_fst a b`. Unification cannot find `a` and `b`: the type `AtomicOf … (.pair ?a ?b)` is computed from the shape, and matching it against a concrete product type does not invert the computation. `projShapeOfSubject` reads the pair shape off the tuple's type (`shapeOfCarrierType` maps `HydroSem.AtomicStream …` to `TickShape.stream …` and `Prod` to `TickShape.pair`), and `applyWithShapes` applies the lemma with its two `TickShape` arguments pinned, leaving every other argument to unification.

## Monotonicity of the Denotation

`M_monoᵢ` says that the denotation is monotone in its wires: inputs that grow in their order give an output that grows, where the grade fixes what growing means. Wires are doubled as implicit binders, with one order hypothesis each, and the order comes from the carrier family (`valuesRelOf`): `PoolLe` per member for streams, `KeyedPool.Le` for keyed streams, trace prefix for tick carriers, read-prefix at every cut decision (the `nondet!` choice of where a snapshot reads) for singletons.

```lean
@toy_relay_mono₁ : ∀ {L : Type} {mem : L → ℕ} (ℓ : L)
  {inp inp' : HydroSem.Stream mem ℓ ℕ StrOrd.noOrder Retries.exactlyOnce} (dec : ToyRelayDec (Values L mem) (mem ℓ))
  (sched : ToyRelaySched (Values L mem) (mem ℓ)),
  (∀ (i : Fin (mem ℓ)), PoolLe StrOrd.noOrder Retries.exactlyOnce (MemberMap.get inp i) (MemberMap.get inp' i)) →
    ∀ (i : Fin (mem ℓ)),
      PoolLe StrOrd.noOrder Retries.exactlyOnce (MemberMap.get (toy_relay (Values L mem) ℓ inp dec sched).1 i)
        (MemberMap.get (toy_relay (Values L mem) ℓ inp' dec sched).1 i)
```

No tactic runs. `MonoRel` is an instance of `HydroSem` whose carrier is a subtype of pairs of `Values` carriers related by the grade's order, and whose every operator runs the `Values` operator on both components ([Monotonicity by Instantiation](../semantics/monorel.md)):

```lean
  Stream ℓ α _ ord ret :=
    {p : MemberMap (mem ℓ) (PoolCarrier α ord ret)
        × MemberMap (mem ℓ) (PoolCarrier α ord ret) //
      ∀ i, PoolLe ord ret (p.1 i) (p.2 i)}
```

The module is generic in `H`, so `M` at `MonoRel`, with each wire packed as `⟨(a, a'), hm⟩`, is a pair whose components are `M` at `Values` on `a` and on `a'`, by definition. Its `.property` is then the statement, and `genMono` hands that term to the kernel as the proof (the packing passes the subtype's predicate to `Subtype.mk` explicitly, because inference cannot see through the instance to it):

```lean
      let prf ← mkProjection (← mc.legAppAt monoI monoArgs leg) `property
      let stmt ← mkForallFVars
        (mc.ctxArgs ++ argBinders ++ hyps) concl
      -- the `MonoRel` projection is the proof (module-scale defeq, judged by the kernel)
      let val ← mkLambdaFVars (mc.ctxArgs ++ argBinders ++ hyps) prf
      addThm name stmt val
```

The definitional equality stops at the first knot. `MonoRel.fix_stream fuel body` is `iterate body ⟨(⊥, ⊥), _⟩ fuel` over the subtype, and with `fuel` a variable `iterate` does not reduce, so the first component of the `MonoRel` iterate equals the `Values` iterate only by induction on the fuel. `runModMonoT` therefore takes this route only when no callee reaches a knot (`reachesKnotN`). Otherwise it takes the compositional route `genGlueMono` (`Hydro/HydroGenKnot.lean`), which composes the callees' `mono` lemmas output by output, and a knot's own `K_mono₁` comes from its iteration from `⊥` ([The Knot Stack](./knot-stack.md)).

A user meets these lemmas in knot `invariant` obligations: `toy_loop`'s `base` and `step` (`Hydro/HydroGenToy.lean`) are both `toy_step_mono₁`, applied to the order between successive iterates of the loop.

The other diagnostics of these phases are `HYDROGEN_WF_DIAG` (no default-transparency fallbacks, so a failing goal prints reducibly) and `HYDROGEN_RULE_DIAG` (every keyed rule's keys and outcome).
