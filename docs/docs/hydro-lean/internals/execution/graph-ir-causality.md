---
sidebar_position: 1
title: The Graph IR and Its Causality Theorems
---

# The Graph IR and Its Causality Theorems

A deployed process runs a **step function**: it holds a cell of state per operator, consumes the input items that arrived at this step, emits this step's output items, and does each operator's work once. The semantics a Hydro Lean program is proved against have a different form. A program is written once against the `HydroSem` signature and read by several instances of it ([The Approach](../approach.md)). `Values` is its **denotational semantics**: a stream captures all the histories it could have had, and the network is an identity operator ([The Network as Identity](../semantics/values.md#the-network-as-identity)). `SchedSem` is its **operational semantics** ([the step machine](../semantics/sched.md)): every wire advances on one global clock of **steps**, network hops deliver with a delay, and each member is paced through its ticks. Because `SchedSem` is an instance of the same signature, it computes a wire's whole run, its view at every step, as a function of the whole runs of the wire's inputs. Hydro Lean connects the two forms by compiling each program to a typed dataflow graph that has both a whole-run reading and a step function, and by proving four theorems about the graph once, for every program. The machine proofs stand on those theorems.

## Why a Graph

The obvious design is one more instance of `HydroSem`, a stateful one. It cannot be written, for two reasons.

- **An instance sees applications, not sharing.** In `let a := …; H.fold a …; H.filterMap a …` both operators receive `a` as a value. Nothing in the instance can tell that the two arguments are one **wire** (one operator output, read twice), so a stateful instance would give `a`'s producer two cells and run it twice.
- **An instance cannot step a `fix` body.** A **knot** is the set of mutually recursive wires of one `fix … complete` block, Rust's `forward_ref` (`CycleSource`/`CycleSink` in `hydro_lang/src/compile/ir/mod.rs`). An instance's `fix_stream` receives the body as a function `W → W`. It can apply that function to a whole run, which is what `SchedSem` does, but it cannot run the body's operators once per step against cells.

`hydro_lang` reifies sharing and feedback through `Rc` pointer identity (`HydroNode::Tee`, the cycle nodes). Pointer identity is `unsafe` in Lean. The one agent that sees a body's `let`s is the `hydro def` elaborator, therefore it compiles the body of each module `M` into the graph below (`M.dag`). The graph has two readings. `Dag.denote` reads it operator by operator with `SchedSem`'s definitions, giving whole runs; `Dag.step` ([The Lock-Step Machine](./lockstep-machine.md)) runs it one step at a time against cells. The generated `M.dag_denote` proves that the first reading equals the program's run at `SchedSem` ([The Project Phase](../generator/project-phase.md) shows how it is proved in two hops with callee graphs kept folded), and `Dag.run_spec` proves that the second agrees with the first.

## Representation

A **`Kind`** describes a wire's **carrier** (the type a semantics gives a live collection) at the machine, together with its location. There is one constructor per stream-level carrier of `HydroSem`; the values a tick block's body computes internally are not wires.

```lean
inductive Kind (L : Type) (mem : L → Nat) : Type 1
  | stream (ℓ : L) (α : Type) (inst : DecidableEq α) (ord : StrOrd) (ret : Retries)
  | keyed (ℓ : L) (K V : Type) (iK : DecidableEq K) (iV : DecidableEq V)
      (ord : StrOrd) (ret : Retries)
  | sing (ℓ : L) (α σ : Type) (inst : DecidableEq α) (ord : StrOrd) (ret : Retries)
      (b : SingBound σ)
…
  | atomic (ℓ : L) (σ : Type)
…
```

`L` is the type of locations and `mem ℓ` the number of members of the cluster at location `ℓ`; every carrier is per member. `Kind.sched` maps a kind to its `SchedSem` carrier, definitionally equal to the instance's projections: `Fin (mem ℓ) → StepHist α` for a stream (its view at every step) and `Fin (mem ℓ) → Nat → Trace σ` for an **atomic** wire, which holds one entry per **tick** (a step at which the member runs its tick blocks, Rust's `sliced!` bodies).

A context `Γ : Ctx L mem` is a `List (Kind L mem)`, and a variable `Var Γ κ` is a typed de Bruijn index into it. An environment of wires is an `HList`, which is a recursive type function rather than an inductive:

```lean
@[reducible] def HList (f : Kind L mem → Type u) : List (Kind L mem) → Type u
  | [] => PUnit
  | κ :: κs => f κ × HList f κs
```

The choice is forced by universes: an inductive indexed by `Kind` would live in `Type 1`, while the right-nested product lives in the universe of its elements, so `Env Γ := HList Kind.sched Γ` is a `Type` and the cells and increments built from it later stay in `Type`.

A **`Node Γ κs`** is one operator application whose wire arguments are variables into `Γ` and which binds the output kinds `κs`. Data arguments (closures, seeds, proofs) are stored at the machine's types. The proofs are the fold obligations (`FoldOk`): a fold over an unordered stream must give one result for every arrival order, because `Values` leaves the order unresolved and the fold has to give one result for all the histories the stream captures ([Grades and Quotient Carriers](../semantics/grades.md)). The signature's `fold` takes the proof as an argument at every instance, so the node holds one for `Dag.denote` to pass to `SchedSem`'s fold. A **`Dag Γ outs`** is the A-normal-form `let`-chain:

```lean
inductive Node : Ctx L mem → List (Kind L mem) → Type 1
  | map {Γ} {ℓ : L} {α β : Type} (iα : DecidableEq α) (iβ : DecidableEq β) {ord : StrOrd}
      (x : Var Γ (.stream ℓ α iα ord .exactlyOnce)) (f : Fin (mem ℓ) → α → β) :
      Node Γ [.stream ℓ β iβ ord .exactlyOnce]
…
  | broadcast_closed {Γ} {c p : L} {α : Type} (iα : DecidableEq α) {ord : StrOrd}
      {ret : Retries} (x : Var Γ (.stream c α iα ord ret)) :
      Node Γ [.keyed p (Fin (mem c)) α inferInstance iα ord ret]
…
  | call {Γ} {Γ' : Ctx L mem} {outs : List (Kind L mem)} (N : Dag Γ' outs) (args : Subst Γ' Γ) :
      Node Γ outs
  /-- One `fix … complete` block: the knot wires `ks` (every `forward_ref` of the block) and
  the body's exposed `let`s `outs`. The body sees the knot wires at indices `0 … |ks|-1` and
  returns their next values first, then the exposed lets. -/
  | fixN {Γ} (ks : List (KnotKind L mem)) {outs : List (Kind L mem)}
      (body : Dag (ks.map KnotKind.toKind ++ Γ) (ks.map KnotKind.toKind ++ outs)) :
      Node Γ (ks.map KnotKind.toKind ++ outs)

/-- The A-normal-form `let`-chain. -/
inductive Dag : Ctx L mem → List (Kind L mem) → Type 1
  | ret {Γ} {outs : List (Kind L mem)} (xs : HList (Var Γ) outs) : Dag Γ outs
  | letN {Γ} {κs outs : List (Kind L mem)} (n : Node Γ κs) (rest : Dag (κs ++ Γ) outs) :
      Dag Γ outs
```

`call` embeds a callee module's own graph with its inputs substituted, so a module is compiled once and called by reference. `fixN` is a whole `fix … complete` block: the body sees the knot wires at indices `0 … |ks|-1` and returns their next values first, then the `let`s the body exposes, so the continuation after `complete` reads them from the same body instance.

This is `toy_relay`'s graph as the elaborator emits it (`#print`, from `Hydro/HydroGenToy.lean`):

```lean
fun {L} {mem} ℓ =>
  Project.Dag.letN (Project.Node.map instDecidableEqNat instDecidableEqNat Project.Var.zero fun _i n => n + 1)
    (Project.Dag.letN (Project.Node.batch instDecidableEqNat StrOrd.noOrder Retries.exactlyOnce Project.Var.zero ())
      (Project.Dag.letN (Project.Node.source_interval (fun x => 0) 1)
        (Project.Dag.letN
          (Project.Node.batch instDecidableEqPUnit StrOrd.totalOrder Retries.atLeastOnce Project.Var.zero ())
          (let emitted_step := fun me st inpt =>
            have bb := inpt.1;
            have g := inpt.2;
            (HydroSem.bsPure (), HydroSem.bfilterIf bb (HydroSem.boIsSome (HydroSem.bfirst g)));
          Project.Dag.letN
            (Project.Node.tick_scan (TickShape.sing Unit)
…
              (Project.Var.zero.succ.succ, Project.Var.zero) emitted_step ())
            (Project.Dag.letN (Project.Node.allTicks instDecidableEqNat Project.Var.zero)
              (Project.Dag.ret
                (Project.HList.cons Project.Var.zero (Project.HList.cons Project.Var.zero.succ Project.HList.nil))))))))
```

Every **decision** argument (an input that resolves a `nondet!` choice, [Nondeterminism and Decisions](../../using/programs/nondeterminism.md)) is `()`. At `SchedSem` the batch cuts (which items each tick of a `batch` takes) are given by the machine's own ticks, and the network's behaviour is execution data, not program data: a **schedule** `Dag.Sched d` holds, per network node, the delivery cursor of each (receiver, sender) pair at each step, and per timer node its script. The graph is compiled once and the schedule is supplied to `Dag.denote d sch` and to the machine `Dag.run d sch`. The generated `M.schedOf`/`M.recordOf` convert between a module's decision records and the schedule, and `M.schedOf_recordOf` proves the conversion is an isomorphism, so quantifying over schedules is equivalent to quantifying over the module's decisions.

## Knots and the Bekić Cascade

`Dag.denote` reads a knot block as the same cascade the `fix` elaborator emits for the program (`HydroDef.closeWire`). The first wire is solved innermost, for every value of the later wires, and each later wire's fixpoint closes over the earlier ones:

```lean
def bekic : (ks : List (KnotKind L mem)) →
    (HList (Kind.sched (mem := mem)) (ks.map KnotKind.toKind) →
      HList (Kind.sched (mem := mem)) (ks.map KnotKind.toKind)) →
    HList (Kind.sched (mem := mem)) (ks.map KnotKind.toKind)
  | [], _ => HList.nil
  | k :: ks, B =>
    let inner : HList (Kind.sched (mem := mem)) (ks.map KnotKind.toKind) → k.toKind.sched :=
      fun xs => knotFix pacing k (fun l => (B (HList.cons l xs)).head)
    let rest := bekic ks (fun xs => (B (HList.cons (inner xs) xs)).tail)
    HList.cons (inner rest) rest
```

```lean
  | _, _, .fixN ks body, sch, env =>
    let closed := bekic pacing ks (fun xs => (Dag.denote body sch (xs.append env)).take)
    closed.append (Dag.denote body sch (closed.append env)).drop
```

`knotFix` is `SchedSem`'s `fix_stream` or `fix_tick`, the ungated Kleene diagonal ([the step machine](../semantics/sched.md)). Because the graph's reading of a knot has exactly the shape the elaborator produced for the program, `M.dag_denote` at a knot is an unfolding, not a fixpoint argument.

## Lags

The causality theorems compare two runs up to a **horizon** `h`, the last step at which they are claimed to agree, and grade the claim by a **lag**: a natural number per wire, the number of steps before the horizon at which that wire's agreement stops. Two stream wires agree at lag `n` below horizon `h` when their views agree at every step `t` with `t + n ≤ h` (`Hydro/SchedCausal.lean`):

```lean
def SAgreeL {m : Nat} {α : Type} (h n : Nat) (x y : Fin m → StepHist α) : Prop :=
  ∀ i t, t + n ≤ h → (x i).view t = (y i).view t
```

`Node.lagOf` computes a node's output lags from its inputs' (`Hydro/Graph/Causal.lean`):

```lean
  | _, _, .map _ _ x _, l => HList.cons (l.get x) HList.nil
  | _, _, .filterMap _ _ x _, l => HList.cons (l.get x) HList.nil
  | _, _, .broadcast_closed _ x, l => HList.cons (l.get x - 1) HList.nil
…
  | _, _, .union _ a b, l => HList.cons (max (l.get a) (l.get b)) HList.nil
…
  | _, _, .source_interval _ _, _ => HList.cons 0 HList.nil
…
  | _, _, .defer_tick _ x, l => HList.cons (l.get x - 1) HList.nil
…
  | _, _, .fixN ks body, l =>
    let Lg := Lags.maxAll l
    (Lags.const (ks.map KnotKind.toKind) Lg).append
      (HList.drop (κs := ks.map KnotKind.toKind) (Dag.lagOf body ((Lags.const (ks.map KnotKind.toKind) Lg).append l)))
```

An immediate operator inherits the maximum of its inputs' lags. A network hop and `defer_tick` take one off, because each reads its input one step back: `StepHist.deliver` shows a receiver at step `t+1` a prefix of the sender's view at `t` (the **delivery floor**), and `defer_tick` releases before it pushes. Sources are at `0`. A knot block's wires sit at the maximum of its externals' lags, which is coarse and sound (`closeLags_le` bounds the true closure lags by it).

The arithmetic is the tropical (max, +) algebra on lag vectors: `Lags.sup` is the pointwise `max`, `Lags.addC` adds a constant everywhere. A knot body's lag functional `Φ` (its knot-output lags as a function of its knot-input lags, with the **externals**, the block's inputs from outside it, held fixed) satisfies five laws, proved once for every graph by `Dag.knotLag_laws`:

```lean
structure LagLaws {K : List (Kind L mem)} (Φ₀ Φ : Lags K → Lags K) : Prop where
  mono : ∀ {m m' : Lags K}, Lags.Le m m' → Lags.Le (Φ₀ m) (Φ₀ m')
  sub : ∀ (m : Lags K) (c : Nat), Lags.Le (Φ₀ (Lags.addC m c)) (Lags.addC (Φ₀ m) c)
  max : ∀ (m m' : Lags K), Φ₀ (Lags.sup m m') = Lags.sup (Φ₀ m) (Φ₀ m')
  zero : Φ₀ (Lags.const K 0) = Lags.const K 0
  mix : ∀ (m : Lags K), Φ m = Lags.sup (Φ₀ m) (Φ (Lags.const K 0))
```

`Φ₀` is the same body with the externals at lag `0`. `mix` splits any evaluation into the knots' own contribution and the externals' constant one, so a cascade argument can reason about the knots with the externals factored out.

## Cascade Validity

A knot is a fixpoint of its body only if the body cannot read the knot at the same step: every path from a knot wire back to its own output has to pass a hop or a `defer_tick`. The obvious check is that every knot output is strict in every knot wire. It rejects `leader_election`: at the probe (knots at lag `1`, externals at `0`) its block's knot-output lags are `[0, 0, 1]`, because `p_is_leader`'s output reads the `i_am_leader` knot at the same step, while `i_am_leader` reads `p_is_leader` only through a hop. Every cycle is strict, which is Rust's "acyclic through strata", but not every edge.

The framework's check closes the knots one at a time, as `bekic` does:

```lean
namespace LagFun

/-- The lag functional of the rest of the cascade, the first knot closed. -/
def next {κ : Kind L mem} {κs : List (Kind L mem)} (Φ : Lags (κ :: κs) → Lags (κ :: κs)) : Lags κs → Lags κs :=
  fun m => (Φ (HList.cons (Φ (HList.cons 0 m)).1 m)).2

end LagFun
…
def cascadeValid : (κs : List (Kind L mem)) → (Lags κs → Lags κs) → Bool
  | [], _ => true
  | _ :: κs, Φ => ((Φ (HList.cons 1 (Lags.const κs 0))).1 == 0) && cascadeValid κs (LagFun.next Φ)
```

The first knot must have output lag `0` when its own wire is at `1` and everything else at `0`: it is strict in itself. `LagFun.next` then closes that knot by substituting its closure lag, its output with its own wire at `0` (sound by `LagLaws.head_bound`), and checks the rest of the cascade. A graph is **valid** (`Dag.valid`) when every `fixN` body, at every nesting, passes `cascadeValid` of `Dag.knotLag ks body (Lags.const Γ 0)`.

For `toy_loop` (one knot `w`, body `toy_step = union (values (broadcast_closed (toy_relay w).1)) inp`), the probe puts `w` at `1` and `inp` at `0`; `toy_relay`'s output stays at `1`, `broadcast_closed` takes it to `0`, `values` keeps `0`, `union` takes `max 0 0`. The knot is strict. The project phase runs this check on every `fixN` node of a module's graph, and a block that fails stops elaboration with this error (`Hydro/HydroGenLag.lean`):

```lean
          throwError "HydroGen: knot {i} of a knot block is not strict in its own wire once the earlier \
            knots are closed (knot output lags {r} at its own wire at lag 1, the rest at 0) — a cycle \
            without a network hop or `defer_tick`:{indentExpr body}"
```

In practice `hydro def` rejects a cycle without a hop before this point: while the `fix` elaborates, the generator derives each knot's strictness lemma from the chain module's own lag analysis, and that derivation fails first ([The Knot Stack](../generator/knot-stack.md#strictness)). The cascade check makes the graph's validity a theorem, `M.dag_valid`, which the machine proofs use.

The machine and the activation use two consequences of validity: the Jacobi iteration of the lag functional from "every knot unknown" (all `1`) reaches "every knot known" (all `0`) in as many rounds as there are knots (`iter_zero`), and the only post-fixpoint below `1` is `0` (`postfix_zero`):

```lean
theorem iter_zero (κs : List (Kind L mem)) (Φ₀ : Lags κs → Lags κs) (h : LagLaws Φ₀ Φ₀)
    (hv : cascadeValid κs Φ₀ = true) : knotIter Φ₀ κs.length (Lags.const κs 1) = Lags.const κs 0
```

[The Lock-Step Machine](./lockstep-machine.md) turns `iter_zero` into the statement that `|ks|` rounds of the body's step compute the knot increments of a step.

## The Four Graph Theorems

One structural induction over `Dag` proves each of these theorems, from the per-operator lemmas of `Hydro/SchedCausal.lean`, `SchedMono.lean` and `SchedPaced.lean`. A module gets its instance by rewriting with `M.dag_denote`, so no program proves them again.

| Theorem | Says | Hypothesis | Used by |
|---|---|---|---|
| `Dag.denote_causal` | inputs agreeing up to `h` give outputs agreeing up to `h` | none | machine headlines that truncate an input at the horizon |
| `Dag.denote_agreeL` | inputs agreeing at lags `l` give outputs agreeing at `Dag.lagOf d l` | `Dag.valid d` | the generated `M.dag_agreeL`, the knot stack's strictness, `Dag.fixN_strict` |
| `Dag.denote_tmono` | every atomic output is step-monotone (`TMono`) | `Dag.valid d`, atomic inputs `TMono` | the machine's tick-buffer cells |
| `Dag.denote_tpaced` | every atomic output gains at most one entry per step, none when its member is inactive (`TPaced`) | `Dag.valid d`, inputs `TMono` and `TPaced` | the machine's single-slot increments |

```lean
theorem Dag.denote_agreeL : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) → (sch : Dag.Sched d) →
    Dag.valid d = true → ∀ (h : Nat) (lags : Lags Γ) (env env' : Env Γ), HAgreeL h lags env env' →
    HAgreeL h (Dag.lagOf d lags) (Dag.denote pacing d sch env) (Dag.denote pacing d sch env')
```

- **Causality** reduces a theorem about a run to a theorem about a run whose input stops at the horizon. `cq_safe_sched` (`Hydro/Std/Quorum.lean`) truncates the response history at `T`, applies `Dag.denote_causal` to `collect_quorum.dag`, rewrites both sides with `collect_quorum.dag_denote`, and so discharges, inside the proof, the pool premise of `cq_safe_sched'`: that machine theorem, obtained from the contract through the coupling, assumes that the input's view at every step, coerced to a multiset, sits below the pool it reasons about (`hc`). The headline then quantifies over inputs with no side condition ([Transferring to the Machine](../../using/proving/transferring-to-the-machine.md)).
- **Graded causality** carries the strictness argument. `Dag.fixN_strict` packages it for a valid block as `HStrict`, and `bekic_fix` (`Hydro/Project/Model/Knot.lean`) concludes that the cascade solves the simultaneous knot equations, `B (bekic B) = bekic B`. The knot stack's `K_strict₁` artifacts are generated from the module's `M.dag_agreeL` at literal lags ([The Knot Stack](../generator/knot-stack.md)).
- **Tick-monotonicity and pacing** make an atomic wire's per-step increment a single optional slot, so the machine's cells can buffer tick entries without losing any ([The Lock-Step Machine](./lockstep-machine.md)).

## Proving Validity per Node

`Dag.valid` and `Dag.lagOf` are `Bool`- and `Nat`-valued structural recursions, so the obvious proof of `M.dag_valid : (M.dag …).valid = true` is `rfl` (`decide`), a kernel evaluation. On `leader_election`'s graph that evaluation runs to gigabytes (the kernel blowups measured in the framework reached 19 GB), for two reasons. The environment-passing recursion shares nothing under call-by-name: every read of a wire's lag re-runs its producer's analysis, exponential in fan-out along a chain. And the `brecOn` encoding of the recursion rebuilds its `below` towers per projection of the graph.

The generator therefore computes the lags itself, as `Nat`s, walking the graph literal in `MetaM` with a memo keyed on `(graph, input lags)` (`Hydro/HydroGenLag.lean`), and proves each node's step with a lemma whose every intermediate is a literal:

```lean
theorem Dag.lagOf_letN_lit {Γ : Ctx L mem} {κs outs : List (Kind L mem)} (n : Node Γ κs) (rest : Dag (κs ++ Γ) outs)
    (l : Lags Γ) (ln : Lags κs) (l' : Lags (κs ++ Γ)) (r : Lags outs)
    (h1 : Node.lagOf n l = ln) (h0 : ln.append l = l') (h2 : Dag.lagOf rest l' = r) :
    Dag.lagOf (.letN n rest) l = r

theorem cascadeValid_cons_lit {κ : Kind L mem} {κs : List (Kind L mem)} (Φ : Lags (κ :: κs) → Lags (κ :: κs))
    (p r : Lags (κ :: κs)) (hp : HList.cons 1 (Lags.const κs 0) = p) (hr : Φ p = r) (h0 : r.1 = 0)
    (hrest : cascadeValid κs (LagFun.next Φ) = true) : cascadeValid (κ :: κs) Φ = true
```

A plain node's `h1` is `Eq.refl` on a literal lag vector: the kernel checks one constructor's rule and one `get`. A `call` recurses into the callee at the argument lags, and a callee's validity is its own generated `N.dag_valid`. A knot block runs the cascade: knot `i` is checked at its own probe, and evaluating `LagFun.next` applied `i` times costs `2^i` body evaluations (`LagFun.next_lit` evaluates `Φ` twice), memoized per probe. The generated statements are short:

```lean
@toy_relay.dag_valid : ∀ {L : Type} {mem : L → ℕ} (ℓ : L), (toy_relay.dag ℓ).valid = true
```

`M.dag_agreeL` (shortened here; the full statement repeats the module's `SchedSem` run on both sides) is `Dag.denote_agreeL` at the module, through `M.dag_denote`:

```lean
@toy_relay.dag_agreeL : ∀ … (h : ℕ) (lags : Project.HList (fun _x => ℕ) […]) (env env' : …),
  Project.HAgreeL h lags env env' →
    Project.HAgreeL h ((toy_relay.dag ℓ).lagOf lags)
      (Project.HList.cons (toy_relay (SchedSem L mem pacing) ℓ (env.get Project.Var.zero) dec sched).1 …)
      (Project.HList.cons (toy_relay (SchedSem L mem pacing) ℓ (env'.get Project.Var.zero) dec sched).1 …)
```

This is one case of the rule that the kernel never evaluates a lag analysis, a module application or a knot wire on a whole program; [Elaboration and Kernel Cost](../cost-model.md) collects it with the related rules.
