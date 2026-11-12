---
sidebar_position: 3
title: "The Step Machine (SchedSem)"
---

# The Step Machine (`SchedSem`)

`SchedSem L mem pacing` (`hydro_lean/Hydro/Sched.lean`) is the **operational semantics** of Hydro Lean: a step machine that runs a program on a global clock, with explicit pacing (which members run at each step), explicit delivery (how much of each sender's output each receiver has received) and buffers of plain lists in arrival order. The denotational semantics `Values` ([Values](./values.md)) says what a program means; `SchedSem` says what it does over time on many processes, and execution must match it. The compiled dataflow graph and the processes are proved equal to it ([Graph and Execution](../execution/graph-ir-causality.md)), and the coupling proves once, for every program, that its runs stay below the denotation ([The Coupling Corner](../corner/coupling.md)).

What distinguishes the machine from `Values` is that every stream is a flat, concrete sequence: per member a list that grows step by step, and inside a tick an `Array`. No carrier contains a quotient, and every grade ([Grades](./grades.md)) executes as plain lists and arrays. The machine therefore always has one actual order and one actual batching, as a deployed process does, and its nondeterministic points do nothing. Every content decision type is `Unit`, and the ordering assumptions are identity operators:

```lean
  assume_ordering u _d := u
```

These flat streams make `SchedSem` the bridge from proofs to execution. Its orders and batchings are not chosen to suit a proof, and the coupling must show that whatever the machine produces is covered by some decision of the denotation ([Derived Decisions](../corner/coupling.md#derived-decisions)). A denotation that quietly relied on a convenient order could not be coupled to it. The machine is not as close to production as the processes exchanging bytes, but it is much closer than `Values`, and the rest of the way down is equations.

The machine is written as an instance of the same type class `HydroSem` as `Values`, so a program runs at it unchanged and its run is built operator by operator from the program's own text. That makes it compositional, and it is still operational in character: every operator is a function of step histories that a real machine computes step by step.

Every adversarial behaviour of the runtime has a run of the machine: arbitrary network latency, members that tick at unrelated rates, members that sleep. The machine stays close enough to DFIR, the dataflow runtime Hydro compiles to, that the compiled graph is proved equal to it. Every `forward_ref` cycle (a **knot**) has a well-defined meaning on the machine without a guardedness proof from each program.

## Time, Wires and Pacing

Time is one global **step** clock. A live stream wire (a collection-valued binding of the program) is, per member (one process of a cluster location), its concrete content as of each step, prefix-monotone because buffers only grow:

```lean
structure StepHist (α : Type) where
  view : Nat → List α
  mono : ∀ t, view t <+: view (t + 1)
```

The other carriers follow the runtime's representation:

| family | `SchedSem` carrier | what it holds |
|---|---|---|
| `Stream`, `KeyedStream` | `Fin (mem ℓ) → StepHist α` (pairs for keyed) | per member, the content received so far, in arrival order; `into_keyed` is a cast |
| `Singleton` | `Fin (mem ℓ) → Nat → σ` | a fold's live value per step, `(src.view t).foldl g init` |
| `Atomic ℓ σ` | `Fin (mem ℓ) → Nat → Trace σ` | per member and step, the realized tick sequence so far (a `Trace σ`, the list of per-tick values); no monotonicity bundled |
| `BoundedStream α` | `Array α` | one tick's batch, the concrete array the runtime holds, at every grade |
| `BoundedKeyedSingleton` | `SchedKMap K V` | an association list with distinct keys |

A folded singleton needs no commutativity to run: the grade obligations are consumed by the coupling, not by execution.

The **pacing**, a parameter of the instance, says what each member does at each step:

```lean
inductive Pace
  | idle
  | stutter
  | tick

abbrev Pacing (L : Type) (mem : L → Nat) := (ℓ : L) → Fin (mem ℓ) → Nat → Pace
```

`idle` is inert, a sleeping process. `stutter` is active without a tick: deliveries land and asynchronous dataflow progresses, but no `sliced!` body runs. `tick` runs the tick bodies. A DFIR tick is a run of `stutter`s followed by a `tick`. Idle steps are legal and observationally invisible, because observations are indexed by tick and content, never by step. They are how the model gives the adversary unbounded lateness and asynchrony between members. Operators read the pacing through two step predicates per member, `pacing.active ℓ i` (not idle) and `pacing.ticks ℓ i` (ticks).

The machine's nondeterminism mirrors the denotation's. `Values` takes content decisions (cuts, order selections, fuel) and `Unit` at the transport and interval-timer sites. `SchedSem` takes timing decisions and `Unit` at content sites:

| decision family | `Values` | `SchedSem` |
|---|---|---|
| `BatchDec`, `SnapDec`, `OrderSelDec`, `TickOrderSelDec` | cuts and selections | `Unit`: a tick consumes everything available, in arrival order |
| `FixDec` | unfolding fuel | `Unit`: unfolding is step progression |
| `TransportDec p c` | `Unit` | `Fin p → Fin c → Nat → Nat`, a delivery cursor per receiver–sender pair and step |
| `IntervalDec n`, `ClockDec n` | `Unit` / readings | `TickScript n Nat`, a value per tick |
| `TicksDec n` | tick-count readings | `Unit`: the pacing decides how many ticks there are |

Pacing is not a decision field because no `nondet!` site owns it; it is a parameter of the instance, and machine theorems quantify over it. The instance's carrier types do not mention the pacing, so the compiled dataflow graph is elaborated once at `Pacing.idle` and its meaning `Dag.denote pacing …` is taken at any pacing.

## Delivery and the `+1` Floor

The one primitive that moves content between members is network delivery. At `Values` the network is an identity operator ([Values](./values.md#the-network-as-identity)); delay, latency and per-pair order exist only here. There is no separate message buffer: for each receiver–sender pair, the sender's own history plays the role of the queue, and a **delivery cursor** (the `TransportDec` decision) says how much of it the receiver has received at each step. The undelivered suffix is what is in flight. Delivery is gated by the receiver's activity:

```lean
def gateView {β : Type} (p : Nat → Bool) (f : Nat → List β) : Nat → List β
  | 0 => []
  | t + 1 => if p (t + 1) then f t else gateView p f t

def StepHist.deliver {α : Type} (h : StepHist α) (p : Nat → Bool) (c : Nat → Nat) :
    StepHist α where
  view := gateView p (fun t => (h.view t).take (cumMax c (t + 1)))
  mono := …
```

At an active step `t+1` the receiver sees a prefix of the sender's wire as of `t`, one step back. At an inactive step it sees what it saw before, so a sleeping process receives nothing and drains its inbox when it runs. This is what makes a sleeping process of a wired run coincide with an inactive member of the machine ([Processes](../execution/processes.md)). The cursor `c` is read through its running maximum,

```lean
def cumMax (c : Nat → Nat) : Nat → Nat
  | 0 => c 0
  | t + 1 => max (cumMax c t) (c (t + 1))
```

so a delivered message stays delivered whatever the raw cursor does, and every cursor function is a legal decision. A stalled cursor is unbounded latency, and `0` forever is silence. Delivery is per-pair FIFO and never duplicates: `AtLeastOnce` duplicates are content created at sampling sites, not transport artefacts.

The one-step lag is the **`+1` floor**, and it is structural: above it the cursor may still deliver any amount at any step. It exists so that a cycle through the network advances the clock. `Hydro/SchedCausal.lean` states it with `SAgree h s s'`, which says that two stream wires agree at every member and every step up to `h`: agreement of two sender wires up to step `h` gives agreement of the delivered wires up to `h + 1`.

```lean
theorem strict_broadcast_closed {c p : L} {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries} {h : Nat}
    (d : Fin (mem p) → Fin (mem c) → Nat → Nat)
    {s s' : (SchedSem L mem pacing).Stream c α ord ret}
    (hs : SAgree h s s') :
    SAgree (h + 1) ((SchedSem L mem pacing).broadcast_closed d s)
      ((SchedSem L mem pacing).broadcast_closed d s')
```

Without the floor, a ping-pong between two members could pass content around a cycle any number of times within one step. The value of a knot wire at a step would then depend on how far the iteration was unfolded, and no finite unfolding would determine it. Everything below about knots rests on this lemma and its `defer_tick` counterpart.

Fan-in takes no decision. `mergeN` appends every sender's increment of the step, in sender order:

```lean
def mergeNView {m : Nat} {α : Type} (k : Fin m → StepHist α) :
    Nat → List α
  | 0 => (List.finRange m).flatMap (fun j => (k j).view 0)
  | t + 1 => mergeNView k t
      ++ (List.finRange m).flatMap (fun j => (k j).inc t)
```

The fixed order within a step loses no behaviour: cursors are per pair and per step, so a run that wants sender 1's message before sender 0's delivers them at different steps. The interleaving across senders is a consequence of delivery timing, and the `NoOrder` type of `values` makes it unobservable at `Values`.

## Ticks and `defer_tick`

A tick body runs at the member's `tick` steps (`tickSteps`). `batch` hands each tick everything that arrived since the previous tick (`batchesFrom`), and `snapshot` reads a fold's live value at the tick's step. `defer_tick` carries a value to the next tick. DFIR's operator "buffers all input items and releases them at the next time boundary" (`dfir_lang/src/graph/ops/defer_tick.rs`), and `HydroNode::DeferTick` compiles to it:

```rust
                    HydroNode::DeferTick { .. } => {
                        …
                                    parse_quote! {
                                        #defer_tick_ident = #input_ident -> defer_tick_lazy();
                                    },
```

```lean
  defer_tick {ℓ _σ} init t := fun i step =>
    (init :: t i (step - 1)).take
      (deferReleased (pacing.ticks ℓ i) (fun k => (t i (k - 1)).length) step)
```

The buffer is `init :: input`, and a tick releases its next entry before the input pushes. A tick releases from the buffer as of the previous step (`t i (step - 1)`), and the released count reads the input's length one step back, so what the input pushes at a tick is released at the next tick, as in DFIR. If release read the current step, a tick could release what it had just pushed, a zero-time loop through `defer_tick`. The released count agrees with the production FIFO on any input:

```lean
def deferReleased (p : Nat → Bool) (len : Nat → Nat) : Nat → Nat
  | 0 => if p 0 then 1 else 0
  | k + 1 =>
    if p (k + 1) && decide (deferReleased p len k ≤ len (k + 1)) then deferReleased p len k + 1
    else deferReleased p len k
```

On an input that is never more than one tick behind the tick count, which covers every wire a Hydro program can build, it equals the tick count (`deferReleased_eq_ticks` in `Hydro/SchedPaced.lean`), so the deferred wire is `(init :: t).take ticks`. At step `0` the `step - 1` is `0`, which is harmless because at most the seed is released (`defer_tick_zero`). Release-before-push makes `defer_tick` strict in the same sense as delivery (`strict_defer_tick : TAgree h t t' → TAgree (h + 1) …`).

## The Ungated Knot

`SchedSem` defines a knot wire at step `t` from the body's output at step `t`. There is no gate on the feedback edge:

```lean
  fix_stream {ℓ _α _ _ord _ret} _d body :=
    famHist (fun t i => ((iterate body (fun _ => StepHist.bot) (t + 1)) i).view t)
  fix_tick {_ℓ _σ} _d body := fun i t =>
    stepGate (fun t => (iterate body (fun _ _ => []) (t + 1)) i t) t
```

This is the **Kleene diagonal**: at step `t`, the `(t+1)`-th iterate of the body from the empty wire, read at `t`. It matches DFIR, where a `forward_ref` whose cycle passes a `defer_tick` or a network hop is a binding within the same tick, and only `defer_tick` edges leave topological order. A gated knot (the feedback read one step late) would be well-defined by construction but would add a step of latency that the runtime does not have. The ungated diagonal is not obviously a fixpoint: nothing in the definition says that stage `t+1` at step `t` equals the body applied to the diagonal itself.

It is a fixpoint when the body is **strict** in the knot wire, meaning that the body's output at a step depends only on the wire strictly before that step:

```lean
def SStrict {n : Nat} {α : Type} (A : (Fin n → StepHist α) → (Fin n → StepHist α)) : Prop :=
  ∀ h (x y : Fin n → StepHist α), SAgreeL h 1 x y → SAgree h (A x) (A y)
```

`SAgreeL h n x y` is agreement at **lag** `n`: the wires agree at every step `t` with `t + n ≤ h`. A body is strict when every path from the knot wire to the output passes a network hop or a `defer_tick`, because those are the operators that take one off the lag. The step a hop costs exists only in the machine; at `Values` the same hop is an identity operator, and the coupling carries the difference as lag. From strictness, the stages stabilize below their count, by strong induction on the step:

```lean
theorem stab_iterate_strict {n : Nat} {α : Type} {h L : Nat}
    {A A' : (Fin n → StepHist α) → (Fin n → StepHist α)}
    (hAB : ∀ h', h' ≤ h → ∀ (x y : Fin n → StepHist α),
      SAgreeL h' (L + 1) x y → SAgreeL h' L (A x) (A' y)) :
    ∀ (t m m' : Nat), t + L ≤ h → t < m → t < m' → ∀ i,
      ((iterate A (fun _ => StepHist.bot) m) i).view t
      = ((iterate A' (fun _ => StepHist.bot) m') i).view t
```

At step `t`, stages `m` and `m'` are `A` applied to stages `m-1` and `m'-1`. By the induction hypothesis those agree at every step before `t`, and strictness turns that into agreement at `t`. Every stage beyond `t` therefore has the same view at `t`, the diagonal picks that value, and the body applied to the diagonal agrees with it:

```lean
theorem fix_stream_eq {ℓ : L} {α : Type} [DecidableEq α] {ord : StrOrd} {ret : Retries}
    {body : (SchedSem L mem pacing).Stream ℓ α ord ret → (SchedSem L mem pacing).Stream ℓ α ord ret}
    (hb : SStrict body) (i : Fin (mem ℓ)) (t : Nat) :
    (((SchedSem L mem pacing).fix_stream () body) i).view t
      = ((body ((SchedSem L mem pacing).fix_stream () body)) i).view t
```

`fix_tick_eq` states the same equation for tick knots. Strictness is a fact about each program's body, and the generator (the `hydro def` command, which emits each module's theorems, [The Generator](../generator/overview.md)) proves it per knot (`K_strict₁`, [The Knot Stack](../generator/knot-stack.md)). No end-to-end theorem ([Soundness](../soundness.md)) carries it as a hypothesis. A block of several knots is closed one knot at a time (Bekić): each knot must be strict in its own wire once the earlier ones are closed, which is weaker than every knot output being strict in every knot wire. `leader_election` needs the weaker form, because `p_is_leader` reads the `i_am_leader` knot immediately while every cycle still passes a hop. `hydro def` rejects a cycle with no hop and no `defer_tick` while its `fix` elaborates, because the generator cannot derive the strictness lemma; for `fix w := union w inp` the message is `HydroGen: a lag-2 agreement cannot be read at lag 1` ([The Knot Stack](../generator/knot-stack.md#strictness) explains the reads behind it).

Without strictness, `causal_fix_stream` and `causal_fix_tick` still show that two bodies which agree up to a step whenever their inputs do (causally related bodies) have knots that agree up to that step. Strictness is what makes the diagonal a solution of the knot's equation.

### `famFreeze`

`fix_stream`, `allTicks` and `kallTicks` produce stream wires, which must be `StepHist`s, from raw per-step views that are not prefix-monotone by type. `famFreeze` advances the whole member family only when every member's next view extends its current one:

```lean
def famFreeze {n : Nat} {β : Type} [DecidableEq β]
    (h : Nat → Fin n → List β) : Nat → Fin n → List β
  | 0 => h 0
  | t + 1 =>
    let prev := famFreeze h t
    if (List.finRange n).all
        (fun i => (prev i).isPrefixOf (h (t + 1) i))
    then h (t + 1) else prev
```

It is total on arbitrary input and an identity function on input that already grows by prefix (`famFreeze_eq_of_ascending'`, which `fix_stream_view` applies to a strict body's diagonal). Freezing is family-atomic because of the coupling. Every frozen family view is one raw family view from a single step (`famFreeze_eq_raw : ∃ k, k ≤ t ∧ famFreeze h t = h k`). A per-member freeze could show member 0 at step 5 next to member 1 at step 3, a state no run ever held, and three operator couplings could not be proved over it.

### `stepGate`

`stepGate f t` is `f t` by cases on `t`, and `stepGate_eq` proves it is an identity function. It keeps the kernel from unfolding knots. For a symbolic `t` the `Nat.casesOn` is stuck, so the tick knot applied at an unknown step does not unfold. Without the gate, `iterate body ⊥ (t + 1)` unfolds once for every application at a symbolic step (`t + 1` is `Nat.succ t`). The generated lemma that equates a module's compiled graph, run with `Dag.denote`, to its `SchedSem` run is checked by the kernel, which then evaluates the knot's whole body at each such application; on the production graphs that check was refused at 12 GB. `fix_stream` needs no gate because `famFreeze` already recurses on the step. Proofs reach through the gate with `fix_tick_apply`. [Elaboration and Kernel Cost](../cost-model.md) collects the rules this forced.

## Causality, Monotonicity and Pacing

The machine's carriers do not encode causality, step-monotonicity of ticks or one-entry-per-step pacing in their types. Three files prove each of these per operator, so that the graph layer can lift them to every wire of a compiled graph ([Graph IR and Causality](../execution/graph-ir-causality.md)).

**Causality** (`Hydro/SchedCausal.lean`). An operator's outputs at steps `≤ h` are a function of its inputs at steps `≤ h`: one congruence lemma per operator (`causal_map`, `causal_batch`, `causal_tick_scan`, …) over the agreement relations `SAgree` (stream wires), `FAgree` (per-step values) and `TAgree` (tick traces). The graded form tracks lags (`causalL_<op>`): immediate operators keep the maximum of their inputs' lags, and `broadcast_closed`, `demux` and `defer_tick` take one off. The coverage is total over the signature, because an operator without a congruence lemma could pass information backwards in time.

**Tick monotonicity** (`Hydro/SchedMono.lean`). A realized tick never changes:

```lean
def TMono {n : Nat} {σ : Type} (x : Fin n → Nat → Trace σ) : Prop :=
  ∀ i k k', k ≤ k' → x i k <+: x i k'
```

`tmono_<op>` proves it per operator. The tick knot spends the body's strictness to stabilize the diagonal and the body's monotonicity to extend it (`tmono_fix_tick`). The coupling needs it where a per-tick `assume_ordering` feeds a non-commutative fold: the list the coupling derived its order from at the end of the run must be the list the machine held at every earlier step. The generated lemmas `M_smono₁, M_smono₂, …` state it for each output of a module `M`.

**Pacing** (`Hydro/SchedPaced.lean`). A tick wire gains at most one entry per step, and none at a step where its member is inactive:

```lean
def TPaced {n : Nat} {σ : Type} (act : Fin n → Nat → Bool) (x : Fin n → Nat → Trace σ) : Prop :=
  ∀ i k, (incTrace (x i) k).length ≤ 1 ∧ (act i k = false → incTrace (x i) k = [])
```

It justifies the production interpreter's single-slot increment for tick wires: `incTrace_eq_toList_head?` says that reading a step's increment as an `Option` loses nothing ([The Lock-Step Machine](../execution/lockstep-machine.md)).
