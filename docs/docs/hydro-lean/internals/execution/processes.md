---
sidebar_position: 3
title: Processes and the Compiled Step
---

# Processes and the Compiled Step

The [lock-step machine](./lockstep-machine.md) steps every member of every location at every global step, with every channel's cells in one global state. In a deployment a **process** is one cluster member: it owns only its own cells, it learns from the network how far each incoming channel has been delivered, and what it sends leaves over the network. `Dag.locStep` is that per-process step. `Dag.step_wired` proves that a lock-step step at one process equals the process's step written back into the global state, and `Interleaving.outputs` lifts this to any **interleaving** (a sequence saying which process runs at each step). The compiled form of `Dag.locStep` has its sharing forced against three behaviours of Lean's compiler.

## Members and Hosted Indices

```lean
abbrev Member (L : Type) (mem : L → Nat) := (ℓ : L) × Fin (mem ℓ)

abbrev Hosted (m : Member L mem) (ℓ : L) := { i : Fin (mem ℓ) // (⟨ℓ, i⟩ : Member L mem) = m }
```

`Hosted m ℓ` is the set of members of location `ℓ` that process `m` is. It has at most one inhabitant, `m` itself when `m` lives at `ℓ`, and none otherwise. Every process-side carrier is obtained from the lock-step one by replacing `Fin (mem ℓ)` with `Hosted m ℓ` (`Kind.locDelta`, `Node.LocState`). A network node's cells sit with the receiver (one `ChanCell` per sender), so the queues a process reads are its own state.

Two more per-node families describe what crosses the process boundary:

```lean
def Node.Oracle (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → Node Γ κs → Type
  | _, _, .broadcast_closed (c := c) (p := p) _ _ => Hosted m p → Fin (mem c) → Nat
  | _, _, .demux (c := c) (p := p) _ _ _ => Hosted m p → Fin (mem c) → Nat
  | _, _, .source_interval (ℓ := ℓ) _ _ => Hosted m ℓ → Nat
  | _, _, .time (ℓ := ℓ) => Hosted m ℓ → Nat
…
def Node.Sends (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → Node Γ κs → Type
  | _, _, .broadcast_closed (c := c) (α := α) _ _ => Hosted m c → Array α
  | _, _, .demux (c := c) (α := α) _ _ _ => Hosted m c → Array (Nat × α)
…
```

The **oracle** is what the process reads from its execution this step: per receiving end and sender the cursor, and per timer this tick's reading. A process holds no timing history; the lock-step schedule's script is indexed by the timer cell's tick count.

## The Process Step

`Node.locStep m tk` mirrors `Node.step` with one difference at the network: a channel delivers per the oracle with **empty** sends, and this step's items leave as `Sends`.

```lean
  | _, _, .broadcast_closed (c := c) _ x, ora, Δ, s =>
    -- `memo`/`memo2`: the identity, materialized when compiled, so a value the cells and
    -- the increments both read is computed once per member
    let cells := memo2 fun i j => (s i j).step (ora i j) #[]
    (fun i j => (cells i j).1,
     HList.cons (fun i => tagConcat (fun j => (cells i j).2)) HList.nil,
     fun j => Δ.get x j)
```

A process keeps nothing in flight. Its own sends to itself travel the loopback connection and come back through the network like everyone else's. `tk` says whether the process [ticks](./lockstep-machine.md) (runs its tick blocks); `tk = false` is a [stutter](./lockstep-machine.md#cells). The chain threads cells, increments and sends, and materializes each node's results before the rest reads them:

```lean
  | _, _, .letN n rest, ora, Δ, s =>
    let r := Node.locStep m tk n ora.1 Δ s.1
    -- materialize this node's cells and increments before the rest reads them
    let st := Node.memoState m n r.1
    let out := memoEnv m r.2.1
    let r' := Dag.locStep m tk rest ora.2 (out.append Δ) s.2
    ((st, r'.1), r'.2.1, (r.2.2, r'.2.2))
```

`memo`, `memoState` and `memoEnv` are identity functions in the logic (`memo_apply`, `memoState_eq`, `memoEnv_eq` are `rfl` or induction over `rfl`); [Sharing in Compiled Code](#sharing-in-compiled-code) explains why they exist.

## Lock-Step Steps and Process Steps

`Dag.step_wired` (`Hydro/Project/Model/WiredMain.lean`) is stated for a lock-step context in which exactly `m`'s members are active (`ctx.act = m.hosts`) and tick iff `tk`. Its conclusion, per graph:

```lean
def DagWired [DecidableEq L] (ctx : StepCtx L mem) (m : Member L mem) (tk : Bool) {Γ : Ctx L mem} {outs : List (Kind L mem)}
    (d : Dag Γ outs) (sch : Dag.Sched d) (ora : Dag.Oracle m d) (Δ : HList (Kind.delta (mem := mem)) Γ)
    (s : Dag.State d) : Prop :=
  let r := Dag.locStep m tk d ora (envRestrict m Δ) (Dag.restrict m d s)
  (Dag.step ctx d sch Δ s).1 = Dag.override m d (Dag.enqueue m d r.2.2 s) (Dag.enqueueLoc m m d r.2.2 r.1) ∧
  envRestrict m (Dag.step ctx d sch Δ s).2 = r.2.1 ∧
  IdleEnv m (Dag.step ctx d sch Δ s).2

theorem Dag.step_wired : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) → (sch : Dag.Sched d) →
    (ora : Dag.Oracle m d) → (Δ : HList (Kind.delta (mem := mem)) Γ) → (s : Dag.State d) →
    IdleEnv m Δ → Dag.OracleAt m d sch ora ctx.t s → Dag.Drained d sch s →
    DagWired ctx m tk d sch ora Δ s
```

In words: the global cells after the step equal the process's new cells written back over the global state, with the process's sends appended to every receiver's queue (`Dag.enqueue` for the others, `Dag.enqueueLoc m m` for its own loopback queues); the step's outputs cut to `m` equal the process's outputs; and they are empty at every other member.

Nothing is assumed about the other processes, because an inactive member is inert in the lock-step machine: its channel cells only enqueue (`ChanCell.stepG` with `tick = false`), and its tick bodies do not run. The three hypotheses:

- `IdleEnv m Δ`: the input increments are empty off `m` (for list-like kinds; a singleton's increment is a value and says nothing).
- `OracleAt`: the oracle reads the [schedule](./graph-ir-causality.md#representation) at `ctx.t`. `Dag.cursorsAt` builds such an oracle from the process's own cells, and `Dag.cursorsAt_oracleAt` proves it.
- `Drained`: the multi-input tick cells hold no complete item.

```lean
def Node.Drained : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) → Node.Sched n → Node.State n → Prop
  | _, _, .zipTick .., _, s => ∀ i, (s i).1 = [] ∨ (s i).2 = []
  | _, _, .reduce_watermark .., _, s => ∀ i, (s i).1.1 = [] ∨ (s i).1.2 = []
  | _, _, .tick_scan _ ins _ _ _ _, _, s => ∀ i, PendingOf.avail ins (s i).1 = 0
…
```

Without it a `zipTick` cell holding an item on both sides would release a pair at an inactive member, and the other members' cells would not stand still. `Drained` is not a restriction on runs: `Dag.init_drained` and `Dag.step_drained` show every state the machine produces is drained, because each step consumes every complete item.

The [knot](./graph-ir-causality.md#why-a-graph) case reduces to the body's case, round by round, on both sides: the lock-step `fixN` and the process's `fixN` each iterate the body's step `|ks|` rounds from empty knot increments, and `knotFill_restrict` identifies the starting points.

## Interleavings

```lean
structure Interleaving {Γ : Ctx L mem} {outs : List (Kind L mem)} (d : Dag Γ outs) where
  who : Nat → Member L mem
  tick : Nat → Bool
  inp : Nat → HList (Kind.delta (mem := mem)) Γ
  sched : Dag.Sched d
```

An interleaving says which member runs at each step (`who`), whether it ticks or stutters (`tick`), what input arrives (`inp`) and what the network delivers (`sched`). Everything else is derived: the **pacing** (who is active and who ticks at each step) is `pacingOf I.who I.tick`; the consumed inputs `I.env k` are `I.inp k` masked to the running process, because a sleeping process cannot consume input; the running process's oracle is `Dag.cursorsAt` on its view. `Interleaving.step_wired` discharges all three hypotheses of `Dag.step_wired` at every step, from which `view_active` (the running process steps on its own cells) and `view_idle` (every other process only receives its sends) follow.

`Interleaving.outputs` states:

```lean
theorem outputs (hv : Dag.valid d = true) (k : Nat) :
    (I.active k).2.1 = envRestrict (I.who k) (envIncAt (Dag.denote I.pacing d I.sched I.wires) k)
```

The running process's outputs at every step equal the increments, at its members, of the graph's run `Dag.denote` on the interleaving's own pacing and schedule; `M.dag_denote` makes that the program's `SchedSem` run ([the step machine](../semantics/sched.md)). The proof is `Dag.run_spec` on the lock-step machine, cut to the process by `step_wired`. `Dag.run_spec` needs an environment of `SchedSem` wires that is tick-monotone and paced; an interleaving supplies only increments. `envOfInc` (`Hydro/Project/Model/EnvOfInc.lean`) builds the wires whose increments they are, the history at `t` being the concatenation of the increments up to `t`:

```lean
def Kind.ofInc : (κ : Kind L mem) → (Nat → κ.delta) → κ.sched
  | .stream _ _ _ _ _, inc => fun i => histOfInc (fun t => (inc t i).toList)
…
  | .atomic _ _, inc => fun i => accum (fun t => (inc t i).toList)
…
```

An accumulated trace is monotone by construction (`HTMono_envOfInc`) and gains at most one entry per step because an increment is one `Option` slot; it gains none at an idle step because `envMask` puts `none` there (`Interleaving.env_idleSlots`, `HTPaced_envOfInc`). So the one hypothesis of `outputs` is `Dag.valid d`, which the generator proves per module as `M.dag_valid`. [Wire Codecs and Byte-Level Traces](./wire-and-traces.md) shows how a byte-level trace of real processes induces such an interleaving, and [The Activation](./activation.md) shows that what a process executes, `Dag.iterate`, equals `Dag.pass`, the process's `Dag.locStep` after its inbox is enqueued (`Dag.iterate_eq_pass`).

## Sharing in Compiled Code

Per-member values in `Dag.locStep` are closures, `Hosted m ℓ → X`. Compiled as written, every consumer re-runs its producer: call-by-name over a graph with fan-out is exponential in depth, and through a knot it re-runs the iteration per read. Without the tables below, one step of a Paxos proposer took 3.3 s and the sixth step never finished. The same leak at `Values`' member maps (`MemberMap`, [Values](../semantics/values.md)) made `leader_election` at `Values` construct 852, 14k, 344k and 3.9M member maps at one to four proposers (about `16^(n−1)`, 16 being its chain-evaluation count): the whole knot solution was re-run per read, recursively. With the fixes, the count is 694, 694 and 686 at two, three and four proposers.

The remedy is a table built once, behind an identity function the proofs see. `@[csimp]` swaps in the table in compiled code through a kernel-checked equation. Three compiler behaviours each defeated a straightforward table.

**1. Function-typed declarations and local functions are eta-expanded to their type's arity** (`etaExpand` in LCNF, the compiler's intermediate representation). In `let v := Vector.ofFn f; fun i => v.get i`, the `let` floats into the lambda and is rebuilt per call. Tables therefore live in structures, whose return type is not an arrow:

```lean
def mkImpl (f : Fin n → C) : MemberMap n C :=
  let v := Vector.ofFn f
  ⟨fun i => v.get i, ()⟩

@[reducible]
def mk' (f : Fin n → C) : MemberMap n C := ⟨f, ()⟩

@[csimp] theorem mk'_eq_mkImpl : @mk' = @mkImpl := by
  funext n C f
  exact (mkImpl_eq f).symm
```

**2. A structure with exactly one relevant field is erased to that field** (`hasTrivialStructure?`). A one-field `MemberMap` or `MemoBox` was, to the compiler's passes after type erasure, a function again: `Simp` eta-expanded lambdas returning it (turning `fix_stream` into `fun fuel body i => iterate body seed fuel i`, the whole Kleene iteration per read), and `simpAppApp?` merged a let-bound map into its read (`let s := mkImpl n g; s i ⟹ mkImpl n g i`). Each structure therefore carries a second, `Unit` field:

```lean
structure MemberMap (n : Nat) (C : Type u) where
  /-- The cells, as a function. -/
  cells : Fin n → C
  /-- Representation pin (see above); always `()`. -/
  boxed : Unit := ()

structure MemoBox (m : Member L mem) (ℓ : L) (X : Type) where
  fn : Hosted m ℓ → X
  tag : Unit := ()
```

**3. A `match` arm whose type reduces to a function type is eta-expanded too.** `Node.memoState`'s arms returned the cells, `Node.LocState m n`, which reduces to an arrow at most nodes, so the arm `| .fold .., s => memo s` became `fun s i => (memoBoxImpl s).fn i`. The arms return a two-field box instead, and the projection happens outside the match:

```lean
structure StateBox (m : Member L mem) {Γ : Ctx L mem} {κs : List (Kind L mem)} (n : Node Γ κs) where
  val : Node.LocState m n
  tag : Unit := ()

@[reducible] def Node.memoState (m : Member L mem) {Γ : Ctx L mem} {κs : List (Kind L mem)} (n : Node Γ κs)
    (s : Node.LocState m n) : Node.LocState m n :=
  (Node.memoStateBox m n s).val
```

The two-level network cells (receiver × sender) build the inner table as the content of the outer optional value (`memoBox2`), because `fun i => (memoFinBox (s i)).fn` would be eta-expanded and rebuild the inner table per call.

A fourth rule does not come from the compiler: **a table must not capture what it tabulates.** The tabulated function's free variables are the previous cells and increments; a closure holding it would retain every past state. Since `Hosted m ℓ` has at most one inhabitant, the table is one optional value, and the read is a `@[noinline]` function so that its partial application is the stored closure, which captures only the value:

```lean
@[noinline] def lookupHosted {m : Member L mem} {ℓ : L} {X : Type}
    (o : Option X) (hs : ∀ _ : Hosted m ℓ, o.isSome) : Hosted m ℓ → X :=
  fun i => o.get (hs i)

def memoBoxImpl [DecidableEq L] {m : Member L mem} {ℓ : L} {X : Type} (f : Hosted m ℓ → X) : MemoBox m ℓ X :=
  ⟨lookupHosted ((hostedSelf m ℓ).map f) (fun i => by rw [hostedSelf_eq i]; rfl), ()⟩

@[macro_inline, reducible] def memo [DecidableEq L] {m : Member L mem} {ℓ : L} {X : Type} (f : Hosted m ℓ → X) : Hosted m ℓ → X :=
  (memoBox f).fn
```

`memo` is a `@[macro_inline]` projection of the box because the `@[csimp]` replacement runs after `macro_inline`: every use site is first rewritten to `(memoBox f).fn`, and then `memoBox` is replaced by `memoBoxImpl`.

### Skipping Other Locations

A node whose locations the process does not host has empty `Hosted` domains: its cells, increments and sends are functions nobody can call. `Dag.locStepImpl` skips it when `Node.touches m n = false`, leaving its cells and handing the rest empty increments, which `Node.locStep_idle` proves are the same values. It also leaves a callee's or knot body's cells as `Dag.locStepImpl` materialized them node by node (`Node.memoStateImpl`), because `Node.memoState` would walk the callee again at every level of nesting. `@[csimp] Dag.locStep_eq_impl : @Dag.locStep = @Dag.locStepImpl` makes compiled code run it.

### Detecting a Sharing Leak

A sharing leak shows as wall-clock time exponential in the member count or composition depth, at constant memory. The `paxos`, `twopc`, `falsify` and `explore` executables of [The Gate](https://github.com/hydro-project/hydro/blob/main/hydro_lean/internal/GATE.md) run in well under a second; one that takes seconds has a sharing bug, and a `bench` process whose memory grows holds history it should not. To locate it:

- **Count constructions per member count.** Put a `dbgTrace` on the forced vector inside the constructor (not on the map itself: wrapping the map in `dbgTrace msg fun _ => …` re-introduces the laziness), run at one, two, three and four members, and count lines with `sort | uniq -c`. Flat or linear is sharing; `n^depth` is a rebuild. Remove the trace afterwards; `dbgTrace` is subject to the same eta-expansion.
- **Read the IR.** `trace.Compiler.result` on a two-operator toy, or the generated C under `.lake/build/ir/Hydro/Project/`, shows a producer rebuilt inside a consumer's lambda or an extra trailing parameter on an op lambda.
- **Find the hot node.** `Dag.locStepProfiled` (`Hydro/Project/Profile.lean`, `lake exe benchprof nodes`) times each node of a process's step. A hung binary's stack (`gdb -p`, `thread apply all bt`) names the cell being forced; the work runs on a helper thread, and a flat profile of leaf functions points at symptoms.

[Elaboration and Kernel Cost](../cost-model.md) collects the compile-time and kernel-time counterparts of these rules.
