---
sidebar_position: 2
title: The Lock-Step Machine
---

# The Lock-Step Machine

[The graph](./graph-ir-causality.md) has two readings. `Dag.denote` reads it with `SchedSem`'s definitions and computes whole runs: a stream wire is a `StepHist`, its view at every global step, and an atomic wire is a `Trace` per step, one entry per **tick** (a step at which the member runs its tick blocks). The second reading, `Dag.step`, runs the graph one step at a time. Every `SchedSem` operator is a fold over steps or over ticks, so none needs the whole history it is given, and the **lock-step machine** keeps only the fold's state: it holds one cell of state per node, steps every member of every location at every global step, and on each step consumes its inputs' **increments** (what is new on a wire at this step) and emits its outputs' increments. `Dag.run_spec` proves that for every [valid](./graph-ir-causality.md#cascade-validity) graph the machine's outputs are, step by step, the increments of `Dag.denote`'s run.

Folding the definitions does not give the machine directly. An atomic wire's increment has to be a single tick entry, which is a theorem about the graph's runs, not a fact about the type. A channel delivers what the sender sent at earlier steps, so its cell must hold exactly the undelivered suffix. A [knot](./graph-ir-causality.md#why-a-graph) wire at a step equals the body's output at the same step (`bekic_fix`), so the machine has to find a fixpoint inside every step.

## Increments

```lean
@[reducible] def delta : Kind L mem → Type
  | .stream ℓ α _ _ _ => Fin (mem ℓ) → Array α
  | .keyed ℓ K V _ _ _ _ => Fin (mem ℓ) → Array (K × V)
  | .sing ℓ _ σ _ _ _ _ => Fin (mem ℓ) → σ
  | .ksing ℓ _ K V _ _ _ _ _ _ => Fin (mem ℓ) → SchedKMap K V
  | .atomic ℓ σ => Fin (mem ℓ) → Option σ
  | .atomicStream ℓ α _ _ _ => Fin (mem ℓ) → Option (Array α)
  | .atomicKeyed ℓ K V _ _ _ _ => Fin (mem ℓ) → Option (Array (K × V))
  | .atomicKSing ℓ K V _ _ => Fin (mem ℓ) → Option (SchedKMap K V)
```

A stream's increment is its new items. A singleton's increment is its current value, which is what a tick reads. An atomic wire's increment is **one optional slot**: this step's tick entry, or nothing. `Kind.incAt` reads a `SchedSem` wire's increment at a step back off its whole run:

```lean
  | .atomic _ _, w, t => fun i => (incTrace (w i) t).head?
```

`head?` would silently drop entries if a trace could grow by two at one step. It cannot, for any wire of a valid graph whose inputs are well-formed: `Dag.denote_tpaced` ([graph theorems](./graph-ir-causality.md#the-four-graph-theorems)) proves every atomic wire gains at most one entry per step, because a member ticks at most once per step. The single-slot increment is therefore a consequence of the graph theorems, and `HTPaced` is a hypothesis of the machine theorem below for this reason.

## Cells

`Node.State n` is the node's cell, per member: a fold's accumulator, a batch's pending buffer, a tick block's registers and unmatched input suffixes, a knot body's or a callee's cells, and for a network node one channel cell per (receiver, sender) pair:

```lean
structure ChanCell (α : Type) where
  /-- The queue: what has arrived and is not yet delivered, in arrival order. It is an
  `Array` because the batches that enter and leave it are the same arrays. -/
  pending : Array α
  delivered : Nat
…
def ChanCell.step {α : Type} (c : Nat) (sends : Array α) (cell : ChanCell α) :
    ChanCell α × Array α :=
  let out := cell.pending.extract 0 (c - cell.delivered)
  (⟨cell.pending.extract out.size cell.pending.size ++ sends, cell.delivered + out.size⟩, out)

def ChanCell.stepG {α : Type} (tick : Bool) (c : Nat) (sends : Array α) (cell : ChanCell α) :
    ChanCell α × Array α :=
  if tick then cell.step c sends else (cell.enqueue sends, #[])
```

`c` is the **cursor**: how many of the sender's items the receiver has read by now, a `cumMax` of the [schedule](./graph-ir-causality.md#representation) so it never moves back. The cell delivers up to the cursor and then enqueues this step's sends. That order is `SchedSem`'s delivery floor (`StepHist.deliver` shows the receiver the sender's view one step back), and it is also the machine-level form of [strictness](./graph-ir-causality.md#cascade-validity): the delivered output is a function of the cell alone (`ChanCell.stepG_snd`), so a cycle through a channel reads nothing of the current step. A receiver that is not active this step only enqueues, so an inactive member is inert, which [Processes](./processes.md) depends on.

A step knows its global index and who ticks and who is active:

```lean
structure StepCtx (L : Type) (mem : L → Nat) where
  t : Nat
  /-- Whose tick bodies run this step. -/
  tick : (ℓ : L) → Fin (mem ℓ) → Bool
  /-- Who is active this step, ticking or stuttering: deliveries land, knot feedback
  propagates. A member that is not active is inert. -/
  act : (ℓ : L) → Fin (mem ℓ) → Bool
```

An active member that does not tick **stutters**: deliveries land and stream operators fire, tick bodies do not run.

## One Step

`Node.step` consumes the input increments `Δ` and the cell `s`. The `batch`, `defer_tick` and `fixN` arms:

```lean
  | _, _, .batch (ℓ := ℓ) _ _ _ x _, _, Δ, s =>
    let p := fun i => s i ++ Δ.get x i
    (fun i => if ctx.tick ℓ i then #[] else p i,
     HList.cons (fun i => if ctx.tick ℓ i then some (p i) else none) HList.nil)
…
  | _, _, .defer_tick (ℓ := ℓ) _ x, _, Δ, s =>
    (fun i => (if ctx.tick ℓ i then (s i).tail else s i) ++ (Δ.get x i).toList,
     HList.cons (fun i => if ctx.tick ℓ i then (s i).head? else none) HList.nil)
…
  | _, _, .fixN ks body, sch, Δ, s =>
    let K := knotIter (fun K => (Dag.step ctx body sch (K.append Δ) s).2.take) ks.length (knotFill ks)
    Dag.step ctx body sch (K.append Δ) s
```

`batch` buffers until its member ticks, then emits the buffer as the tick's slot. `defer_tick` releases the head of its buffer before pushing this step's slot, so its output is a function of its cell and a cycle through a tick reads nothing of the current step. The `fixN` arm handles the knot. The body's knot outputs at this step may read other knot wires at this step (that is what cascade validity permits), so the machine iterates the body's step on the knot increments starting from empty ones (`knotFill`), `|ks|` rounds, and then steps the body once more at the result. The intermediate rounds' cells are discarded; only the final step's cells are kept. That costs `|ks| + 1` body steps per global step, which is acceptable for a specification. The executed activation computes the same fixpoint with one evaluation per node, and `Dag.iterate_eq_pass` is the equation between them ([The Activation](./activation.md)).

`Dag.step` threads the chain in `let` order, and `Dag.run` iterates it from `Dag.init` with the step context read off a **pacing** (per member and step: idle, stutter or tick).

## The Specification

```lean
theorem Dag.step_spec : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) → (sch : Dag.Sched d) →
    Dag.valid d = true → ∀ (env : Env Γ), HTMono env → HTPaced pacing env → ∀ (t : Nat),
    Dag.step (ctxAt pacing t) d sch (envIncAt env t) (Dag.prevState pacing d sch env t)
      = (Dag.stateOf pacing d sch env t, envIncAt (Dag.denote pacing d sch env) t)

theorem Dag.run_spec {Γ : Ctx L mem} {outs : List (Kind L mem)} (d : Dag Γ outs) (sch : Dag.Sched d)
    (hv : Dag.valid d = true) (env : Env Γ)
    (he : HTMono env) (hp : HTPaced pacing env) (t : Nat) :
    Dag.run d sch pacing (fun t => envIncAt env t) t
      = (Dag.stateOf pacing d sch env t, envIncAt (Dag.denote pacing d sch env) t)
```

The conclusion is an equation, not a simulation relation. The machine's cells after step `t` equal `Dag.stateOf`, a function of the `SchedSem` wires of `Dag.denote`'s run, and the machine's outputs equal the increments of `Dag.denote`. `Dag.valid d` makes every knot its body's fixpoint and bounds the knot iteration. `HTMono env` (every atomic input grows by prefixes) lets the tick-buffer cells read their pending suffix as "the trace beyond what was consumed". `HTPaced pacing env` makes every atomic input increment a single slot. Inside the proof, `Dag.denote_tmono` and `Dag.denote_tpaced` carry the last two from each node's inputs to its outputs along the `letN` chain, so they are assumed only of the graph's inputs, and [Processes](./processes.md) shows that every interleaving's inputs satisfy them by construction.

### The State Abstraction

`Node.stateOf` says, per node, what the cell holds at the end of step `t` as a function of the `SchedSem` wires (`Hydro/Project/Model/StepSpec.lean`):

```lean
  | _, _, .broadcast_closed (p := p) _ x, d, env, t => fun i j => chanCellAt (env.get x j) (pacing.active p i) (d i j) t
…
  | _, _, .fold _ g init _ _ _ x, _, env, t => fun i => ((env.get x i).view t).foldl g init
…
  | _, _, .batch (ℓ := ℓ) _ _ _ x _, _, env, t => fun i =>
    (((env.get x i).view t).drop (consumedAfter ((env.get x i).view) (tickSteps (pacing.ticks ℓ i) t) 0)).toArray
…
  | _, _, .defer_tick (ℓ := ℓ) init x, _, env, t => fun i =>
    (init :: env.get x i t).drop (deferReleased (pacing.ticks ℓ i) (fun k => (env.get x i (k - 1)).length) t)
…
  | _, _, .fixN ks body, sch, env, t =>
    Dag.stateOf body sch ((bekic pacing ks (fun xs => (Dag.denote pacing body sch (xs.append env)).take)).append env) t
```

`Dag.stateOf` defines a fold's cell as the `foldl` of the view, a batch's as the view beyond what the ticks so far consumed, a channel's as the sender's view beyond the delivered view (`chanCellAt`), and `defer_tick`'s as `init :: input` beyond the released count. A knot body's cells are read at the closed knots, `bekic`, so the knot case below is an equation between two readings of the same run. `Dag.prevState` shifts `Dag.stateOf` one step back, with `Dag.init` before step `0`.

The rest of `StepSpec.lean` holds the prefix-plus-increment algebra every operator lemma reads its inputs through: `view_eq_prev_inc` (the view at `t` is the view before `t` plus the increment), `trace_eq_prev_inc` for monotone traces, `incView_map`/`incView_filterMap`/`incView_mergeN`, `deliver_view_eq` (at an active step the delivered view is the cursor's prefix of the sender's previous view), and `tickSteps_eq_prev` for tick skeletons.

### One Lemma per Operator

Every operator case has the same shape:

```lean
def NodeSpec {Γ : Ctx L mem} {κs : List (Kind L mem)} (n : Node Γ κs) (sch : Node.Sched n) (env : Env Γ) (t : Nat) : Prop :=
  Node.step (ctxAt pacing t) n sch (envIncAt env t) (Node.prevState pacing n sch env t)
    = (Node.stateOf pacing n sch env t, envIncAt (Node.denote pacing n sch env) t)
```

A stateless operator is one increment lemma. A stateful one is a one-step law of its cell:

```lean
theorem spec_map {Γ : Ctx L mem} {ℓ : L} {α β : Type} (iα : DecidableEq α) (iβ : DecidableEq β)
    {ord : StrOrd} (x : Var Γ (.stream ℓ α iα ord .exactlyOnce)) (f : Fin (mem ℓ) → α → β)
    (env : Env Γ) (t : Nat) : NodeSpec pacing (.map iα iβ x f) () env t := by
  simp only [NodeSpec, Node.step, Node.denote, envIncAt_get]
  show ((), ((fun i => (incView (env.get x i) t).toArray.map (f i) : Fin (mem ℓ) → Array β), ()))
    = ((), ((fun i => (incView ((env.get x i).map (f i)) t).toArray : Fin (mem ℓ) → Array β), ()))
  congr 2
  funext i
  rw [incView_map, List.map_toArray]

theorem chan_cell_step {α : Type} (h : StepHist α) (p : Nat → Bool) (c : Nat → Nat) (t : Nat) :
    (prevChanCell h p c t).stepG (p t) (cumMax c t) (incView h t).toArray
      = (chanCellAt h p c t, (incView (h.deliver p c) t).toArray)
```

`chan_cell_step` carries the whole network argument for one pair; `spec_broadcast_closed` applies it per (receiver, sender) and merges the delivered batches with `incView_mergeN`. The lemmas are split by the kind of cell:

| File | Operators | What the cell argument needs |
|---|---|---|
| `StepOps.lean` | stateless operators, folds, `snapshot`, timers, `defer_tick`, `allTicks` | `defer_tick`: the released count never exceeds the buffer (`TMono`), one slot per step (`TPaced`), then `defer_fifo_step` |
| `StepOps2.lean` | `batch`, `zipTick`, `reduce_watermark` | the consumed count read off the tick skeleton; unmatched suffixes beyond the jointly realized length |
| `StepOps3.lean` | `tick_scan` | the `zipTick` argument lifted through `TickShape`, leaf by leaf |
| `StepOps4.lean` | `kfold_early_stop` | keys ordered by last arrival, because `SchedKMap.ofKeyed` dedups by last occurrence |
| `StepOps5.lean` | `broadcast_closed`, `demux` | `chan_cell_step` per pair |
| `StepOps6.lean` | keyed wires, `tfold`, `latest`, `time`, `snapshot_bounded` | a scan over the new batches; the last present value |

### The Structural Cases and the Knot

`Dag.step_spec` and `Node.step_spec` are one mutual induction (`Hydro/Project/Model/StepMain.lean`). An operator case dispatches to its `spec_<op>`. `letN` rewrites the node's step by its spec and then applies the induction hypothesis to the rest on the environment extended by the node's `Node.denote` run; the `TMono`/`TPaced` facts for that extended environment come from `Dag.denote_tmono`/`Dag.denote_tpaced`. `call` moves `envIncAt` through the argument substitution (`envIncAt_mapSubst`).

The `fixN` case combines two facts.

1. **The closed knots' increments are a fixpoint of the body's step.** `bekic_fix` says the closed knots equal the body's knot outputs, `B (bekic B) = bekic B`. Stepping the body at the closed knots' increments gives, by the induction hypothesis, the increments of the body's `Dag.denote` run at the closed knots, so the body's knot outputs equal the closed knots' increments (`hK` in the proof).
2. **The machine's iteration reaches any fixpoint.** This is `Node.step_fixN_eq` (`Hydro/Project/Model/StepAgree.lean`):

```lean
theorem Node.step_fixN_eq {Γ : Ctx L mem} (ks : List (KnotKind L mem)) {outs : List (Kind L mem)}
    (body : Dag (ks.map KnotKind.toKind ++ Γ) (ks.map KnotKind.toKind ++ outs)) (sch : Node.Sched (.fixN ks body))
    (hv : Node.valid (.fixN ks body) = true) (Δ : HList (Kind.delta (mem := mem)) Γ) (s : Node.State (.fixN ks body))
    (K : HList (Kind.delta (mem := mem)) (ks.map KnotKind.toKind))
    (hK : (Dag.step ctx body sch (K.append Δ) s).2.take = K) :
    Node.step ctx (.fixN ks body) sch Δ s = Dag.step ctx body sch (K.append Δ) s
```

The proof reuses the lag analysis at the machine and does not reason about the body's operators. `DAgree0 l Δ Δ'` says two increment environments agree at the positions where `l` is `0`, and `Dag.step_agree0` says one step respects the analysis: inputs agreeing at `l`'s zero positions give outputs agreeing at `Dag.lagOf d l`'s zero positions. An immediate operator rewrites by the agreement; a strict node reads nothing from `Δ` (`ChanCell.stepG_snd`, `defer_tick`'s head). Iterating it, `Dag.step_knotIter_agree0` says that after `i` rounds from increments `X` that agree with the fixpoint `K` where `l` is `0`, the iterate agrees with `K` wherever `knotIter (Dag.knotLag …) i l` is `0`. Starting from `knotFill` at lag `1` everywhere (nothing known), `iter_zero` brings the lag iterate to `0` everywhere in `|ks|` rounds, so the machine's iterate equals `K`:

```lean
  have h := Dag.step_knotIter_agree0 ctx ks body sch Δ s K hK ks.length (Lags.const _ 1) (knotFill ks)
    (DAgree0.const_of _ (fun h0 => absurd h0 Nat.one_ne_zero))
  have hz := iter_zero (ks.map KnotKind.toKind) (Dag.knotLag ks body (Lags.const Γ 0))
    (Dag.knotLag_laws ks body (Lags.const Γ 0)).self (Node.valid_fixN ks body hv).1
  rw [List.length_map] at hz
  rw [hz] at h
  rw [DAgree0.eq_of_zero (Lags.Le.refl _) h]
```

The fixpoint's uniqueness therefore needs no per-program proof and no measure on data: it follows from the `cascadeValid` check the generator already ran, read at the machine. Each round determines every knot whose output reads only knots already determined, which correspond to the strata that DFIR, Hydro's Rust dataflow runtime, schedules. `Dag.run_spec` is induction on `t` with `Dag.step_spec` at each step.

## What the Theorem Covers

For every graph the generator emits (each carries `M.dag_valid`) and every pacing and schedule, the lock-step machine started from `Dag.init` and fed the increments of a tick-monotone, paced input run emits exactly the increments of `Dag.denote` on that run, which `M.dag_denote` identifies with the program's `SchedSem` run. The machine is not what a process runs: it steps every member of every location at every step and re-runs a knot body `|ks| + 1` times. [Processes](./processes.md) cuts it to one member and lifts the equation to arbitrary interleavings, and [The Activation](./activation.md) replaces the per-step iteration with one sweep.
