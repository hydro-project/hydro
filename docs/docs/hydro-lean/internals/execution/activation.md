---
sidebar_position: 4
title: The Activation
---

# The Activation

[Processes](./processes.md) proved, as `Interleaving.outputs`, that a process whose every step is `Dag.locStep` on its own cells (its state, `Dag.LocState m d`, for the **member** `m`, one element of a location's cluster) emits the increments of the graph's run. `Dag.pass` is that step as a deployed loop body: enqueue the messages that arrived, materialize the cells, take one `Dag.locStep`. One run of a process's loop body is an **activation**. Executed directly, `Dag.pass` evaluates every node of a knot block once per round of the knot iteration. A process instead executes `Dag.iterate`, built on a **sweep** of the graph that steps each node once, and `Dag.iterate_eq_pass` proves the two equal. The compiled form fuses five walks of the graph into one (`Dag.sweepF`), and a fixed set of `@[csimp]` equations transforms the code the theorem is about into the binary's code.

The page uses the graph's vocabulary from [Graph IR and Causality](./graph-ir-causality.md) (nodes, the `letN` chain, lags, [validity](./graph-ir-causality.md#cascade-validity)) and the process-side vocabulary from [Processes](./processes.md) (increments `Kind.locDelta m`, the oracle of cursors and timer readings). **Chain order** is the order of the `letN` chain.

## Knot Wires Within a Step

A [knot](./graph-ir-causality.md#why-a-graph) block `Node.fixN ks body` feeds its body's first `ks.length` outputs back as its first `ks.length` inputs. In DFIR a `forward_ref` inside a tick is a same-tick alias (`CycleSink` emits `identity()`), so the consumer runs after the producer in stratum order within one tick. The model matches that: a **knot wire** at a step is its body's knot output at the same step. Nothing delays it.

The lock-step step computes such a wire without knowing the body's internal order: iterate the whole body's step from empty knot increments, as many rounds as there are knots, then step the body once more on the result (`Hydro/Project/Wired.lean`):

```lean
  | _, _, .fixN ks body, ora, Δ, s =>
    let K := knotIter (fun K => (Dag.locStep m tk body ora (K.append Δ) s).2.1.take) ks.length (knotFillLoc m ks)
    Dag.locStep m tk body ora (K.append Δ) s
```

`ks.length` rounds suffice because the body is cascade-valid (each round fixes at least one more knot; see [Cascade Validity](./graph-ir-causality.md#cascade-validity)). As a specification this is correct, but as an execution it evaluates every node of a block `ks.length + 1` times, a nested block that many times per round of its parent, and it re-steps stateful nodes whose results it then discards. DFIR steps each operator once per tick, and the sweep below does the same.

## The Sweep

The sweep (`Hydro/Project/Sweep.lean`) walks the graph in chain order carrying a **mask**, one `Bool` per wire saying whether the wire's increment for this step is known yet:

```lean
abbrev Mask (κs : List (Kind L mem)) : Type := HList (fun _ => Bool) κs
```

At each node it applies one of three rules:

- A node whose inputs are all known (`Node.inputsKnown`) is stepped and marked `done`; its outputs and sends are stored.
- A **strict** node whose input is not yet known is released and marked `released`. The strict nodes are `broadcast_closed`, `demux` and `defer_tick` (`Node.isStrict`): a network node's output at a step is what its queue delivers, and `defer_tick`'s is its buffered head, so their outputs are a function of their cells alone. Their input is only pushed (sent, or buffered), and the push waits until the input is known.
- Any other node with an unknown input is skipped and stays `todo`. A typed filler (`Node.fillerLoc`) stands in for its outputs so the sweep's result has the step's types; no node that is stepped reads a filler, because a stepped node's inputs are known.

The split of a strict node into release and push is a law about the step, proved per node in `Hydro/Project/Model/SweepSpec.lean`. The law holds for every environment `Δ'`, because the release reads no environment.

```lean
theorem Node.locStep_strict (m : Member L mem) (tk : Bool) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} →
    (n : Node Γ κs) → Node.isStrict n = true → (ora : Node.Oracle m n) → (Δ Δ' : HList (Kind.locDelta m) Γ) →
    (s : Node.LocState m n) →
    Node.locStep m tk n ora Δ s =
      ((Node.pushStep m n Δ (Node.release m tk n ora Δ' s).1 (Node.release m tk n ora Δ' s).2.2).1,
       (Node.release m tk n ora Δ' s).2.1,
       (Node.pushStep m n Δ (Node.release m tk n ora Δ' s).1 (Node.release m tk n ora Δ' s).2.2).2)
```

The per-node state across the sweeps of one step, the node's **part**, is a `NodePart` (a call or a knot block additionally holds its sub-graph's parts and, for a knot block, the knots bound so far):

```lean
inductive NodePart (O S : Type) where
  | todo
  | released (outs : O) (snd : S)
  | done (outs : O) (snd : S)
```

and the sweep of a **plain** node (neither a call of another module's graph nor a knot block) is a case split on it. A `done` node is never stepped again, so a node costs one step per activation however often it is visited. A `released` node's outputs count as known (`Mask.const κs true`) from the moment of release, so a cycle through a hop makes progress. The oracle is read off the cells (`Node.oracleOf m n tm s`) at the visit that steps or releases the node, before its cells change.

```lean
  match part.1 with
  | .done o snd => ((s, part), (o, Mask.const κs true, snd))
  | .released o snd =>
    if Node.inputsKnown n mk then
      let r := Node.pushStep m n Δ s snd
      ((r.1, (.done o r.2, part.2)), (o, Mask.const κs true, r.2))
    else ((s, part), (o, Mask.const κs true, snd))
  | .todo =>
    if Node.inputsKnown n mk then
      let r := Node.locStep m tk n (Node.oracleOf m n tm s) Δ s
      ((r.1, (.done r.2.1 r.2.2, part.2)), (r.2.1, Mask.const κs true, r.2.2))
    else if Node.isStrict n then
      let r := Node.release m tk n (Node.oracleOf m n tm s) Δ s
      ((r.1, (.released r.2.1 r.2.2, part.2)), (r.2.1, Mask.const κs true, r.2.2))
    else ((s, part), (Node.fillerLoc m n Δ s, Mask.const κs false, Node.sendsNil m n))
```

### The Knot Loop

At a knot block the sweep runs its body with the knot wires unknown. The knot outputs that come out known are those that read no knot wire at this step (they pass a strict node first); they are bound, and the body is swept again with them known, which determines the knots that read only those, and so on:

```lean
def knotLoop {K outs : List (Kind L mem)} {S P Snd : Type}
    (sw : HList (Kind.locDelta m) K → Mask K → S → P → (S × P) × (HList (Kind.locDelta m) (K ++ outs) × Mask (K ++ outs) × Snd)) :
    Nat → Mask K → HList (Kind.locDelta m) K → S → P →
    (S × (Mask K × HList (Kind.locDelta m) K) × P) × (HList (Kind.locDelta m) (K ++ outs) × Mask (K ++ outs) × Snd)
  | 0, b, Kv, s, bp => knotFinalize (sw Kv b s bp) b Kv
  | fuel + 1, b, Kv, s, bp =>
    let r := sw Kv b s bp
    let b' := Mask.or b (HList.take (κs := K) r.2.2.1)
    if Mask.beq b' b then knotFinalize r b Kv
    else knotLoop sw fuel b' (Mask.select b Kv (HList.take (κs := K) r.2.1)) r.1.1 r.1.2
```

`Node.sweep` calls it with fuel `ks.length` and the block's stored mask and increments (`part.2.1`), so a block's progress survives between sweeps of its parent. A loop stops when a sweep binds nothing new. That happens for one of two reasons: every knot was already known, so that sweep was the final one and stepped the remaining `todo` nodes; or an external input of the block is unknown, and the block waits for the enclosing sweep to know it. In the second case a nested block is swept partially and finished later.

Several sweeps are needed because real programs have knots that read other knots at the same step. `leader_election`'s block has three, with knot-output lags `[0, 0, 1]` at the validity probe ([Cascade Validity](./graph-ir-causality.md#cascade-validity)): two of its knot outputs pass a strict node before reading any knot wire, and `p_is_leader`'s reads the `i_am_leader` knot at the same step. The first sweep binds the two at lag 0, and the second binds `p_is_leader`.

On the Paxos benchmark's leader, an idle activation has 233 nodes, 1235 node visits (5.3 per node), 33 knot-body sweeps across six nested blocks, of which 1059 visits met a node already `done` and 100 stepped one. The knot chain crosses nesting levels (a knot of `leader_election` feeds one of `paxos_core`, which feeds `leader_election` again), and every hop re-sweeps a whole inner body. The stepping is about 1 % of the time and the visits take the rest; the fused sweep below and [staging](./staging.md) remove the visits.

## Correctness of the Sweep

`Dag.sweepTop_eq` says that on a valid graph the sweep from fresh parts on a fully known environment is `Dag.locStep` at the oracle read off the cells. The proof is an invariant over the sweeps of one step, relative to the reference step from cells `s₀` on the full environment `Δf`. For a plain node it is `Node.PhaseCons`:

```lean
def Node.PhaseCons (m : Member L mem) (tk : Bool) {Γ : Ctx L mem} {κs : List (Kind L mem)} (n : Node Γ κs)
    (tm : Node.Timing m n) (Δf : HList (Kind.locDelta m) Γ) (s₀ s : Node.LocState m n) :
    NodePart (HList (Kind.locDelta m) κs) (Node.Sends m n) → Prop
  | .todo => s = s₀
  | .released o snd => Node.isStrict n = true ∧ ∃ Δ', (s, o, snd) = Node.release m tk n (Node.oracleOf m n tm s₀) Δ' s₀
  | .done o snd => (s, o, snd) = Node.locStep m tk n (Node.oracleOf m n tm s₀) Δf s₀
```

At a knot block the invariant is:

```lean
  | _, _, .fixN ks body, tm, Δf, s₀, s, part =>
    -- the knots bound so far are the reference's, and the body is consistent with the
    -- reference on every environment of the knots not yet bound
    Agree part.2.1.1 part.2.1.2 (knotRef m tk ks body tm Δf s₀) ∧
    ∀ X : HList (Kind.locDelta m) (ks.map KnotKind.toKind), Agree part.2.1.1 X part.2.1.2 →
      Dag.Cons m tk body tm (X.append Δf) s₀ s part.2.2
```

`Agree b X Y` says `X` and `Y` are equal at the wires `b` marks known. `knotRef` is the reference's knot value, the `knotIter` of the `fixN` arm above. The invariant quantifies over `X`: the body's part is consistent with the reference on every choice of the unbound knots, because no node stepped so far has read them. When a later sweep binds a knot, the invariant for the old `X`s still covers the new value. `knotLoop_sound` then needs only that the reference is a fixpoint of the body's step (`Dag.locStep_knotIter_fix`, from the lag analysis), so a knot read off a sweep's known outputs equals the reference's.

Three theorems extend the invariant to the whole graph:

| theorem | says |
|---|---|
| `Dag.sweep_sound` | a sweep on an environment agreeing with `Δf` on the known wires preserves consistency, and its known outputs equal the reference's |
| `Dag.sweep_known` | the outputs it knows are at least what the lag analysis predicts with known wires at lag `0` and unknown at `1` (`maskLags`); a knot loop on a valid body with known externals binds every knot (`knotLoop_all`, the Jacobi iteration of the body's lag functional) |
| `Dag.sweep_complete` | on a fully known environment everything ends `done` |

and `Dag.sweepTop_eq` composes them:

```lean
theorem Dag.sweepTop_eq (m : Member L mem) (tk : Bool) {Γ : Ctx L mem} {outs : List (Kind L mem)} (d : Dag Γ outs)
    (hv : Dag.valid d = true) (tm : Dag.Timing m d) (Δ : HList (Kind.locDelta m) Γ) (s : Dag.LocState m d) :
    Dag.sweepTop m tk d tm Δ s = Dag.locStep m tk d (Dag.oracleOf m d tm s) Δ s := by
  have hs := Dag.sweep_sound m tk d hv tm Δ Δ (Mask.const Γ true) s s (Dag.Part.init m d) (Dag.Cons_init m tk d tm Δ s)
    (Agree.refl _ _)
  have hc := Dag.sweep_complete m tk d hv tm Δ (Mask.const Γ true) s (Dag.Part.init m d) (Mask.all_const_true Γ)
  have hf := Dag.Cons_final m tk d tm Δ s _ _ hc.2 hs.1
  exact Prod.ext hf.1 (Prod.ext (Agree.eq_of_all hc.1 hs.2.1) (hs.2.2.trans hf.2))
```

The hypothesis `hv` is discharged by `M.dag_valid`, which the generator proves for each program `M`; nothing about the sweep is assumed by the headline theorems. Knownness is structural: it depends on the graph and not on the data, so the number of sweeps is the same at every activation.

## Process Loop Bodies

The process-level functions come in pairs, one through the sweep (what runs) and one through `Dag.locStep` (what the proofs use), at three representations of the inbox (`Hydro/Project/Process.lean`):

| runs | proved about | inbox | equation |
|---|---|---|---|
| `Dag.iterate` | `Dag.pass` | typed messages per connection (`Dag.Inbox`) | `Dag.iterate_eq_pass` |
| `Dag.iterateWire` | `Dag.passWire` | messages `(node, sender, payload)` per sender, decoded in one walk (`Dag.inboxOfWire`) | `Dag.iterateWire_eq_passWire` |
| `Dag.iterateBytes` | `Dag.passBytes` | bytes per connection, frames parsed, tails returned | `Dag.iterateBytes_eq_passBytes` |

The two bodies differ in their last line only:

```lean
def Dag.pass (m : Member L mem) (tk : Bool) {Γ : Ctx L mem} {outs : List (Kind L mem)} (d : Dag Γ outs)
    (tm : Dag.Timing m d) (ib : Dag.Inbox m d) (Δ : HList (Kind.locDelta m) Γ) (s : Dag.LocState m d) :
    Dag.LocState m d × HList (Kind.locDelta m) outs × Dag.Sends m d :=
  let s' := Dag.memoState m d (Dag.enqueueInbox m d ib s)
  Dag.locStep m tk d (Dag.oracleOf m d tm s') Δ s'

def Dag.iterate (m : Member L mem) (tk : Bool) {Γ : Ctx L mem} {outs : List (Kind L mem)} (d : Dag Γ outs)
    (tm : Dag.Timing m d) (ib : Dag.Inbox m d) (Δ : HList (Kind.locDelta m) Γ) (s : Dag.LocState m d) :
    Dag.LocState m d × HList (Kind.locDelta m) outs × Dag.Sends m d :=
  let s' := Dag.memoState m d (Dag.enqueueInbox m d ib s)
  Dag.sweepTop m tk d tm Δ s'
```

so `Dag.iterate_eq_pass` is `Dag.sweepTop_eq m tk d hv _ Δ _`. The sends go only to the network; an activation does not enqueue its own loopback messages, and nothing is pending after it. `Dag.iterateBytes` is the function the **shells** (the executables that own the sockets) call, and `WireTrace` and `paxos_safe_wired` are statements about it ([Wire Codecs and Byte-Level Traces](./wire-and-traces.md)).

## The Fused Sweep

Written as above, `Dag.iterateBytes` is five structural walks of the graph: decode the inbox (`inboxOfWire`), enqueue it (`enqueueInbox`), materialize the cells (`memoState`), sweep, encode the sends (`wireEncode`). Each walk allocates a fresh spine of pairs and recomputes every node's context `κs ++ Γ`, which is runtime data: a wire's type in the graph, its `Kind`, carries its location and its [grades](../semantics/grades.md). A node visit cost about 230 ns, dominated by that allocation.

`Dag.sweepF` (`Hydro/Project/Fused.lean`) does all five to each node as the sweep meets it. Two flags thread through: `enq` (this is the first visit this activation, so decode and enqueue the node's inbox) and `enc` (this is the sweep whose sends are final, so encode them). The result of a visit is one structure, `SweepRes`, updated in place when unique, instead of five nested pairs. The common case of a re-sweep, a plain node already `done`, is read off its part at the `letN` without a node-level call or a rebuild of its memo tables (the materialized per-member values, [Processes](./processes.md)):

```lean
    match hp : Node.isPlain n, part.1.1 with
    | true, .done o snd =>
      -- a plain node already `done`, the common case of a re-sweep: read off its part, no
      -- node-level call; its inbox, the first visit's, was enqueued when it was stepped
      let s1 := if enq then Node.memoState m n (Node.enqueueInbox m n (Node.inboxOfWire m n cod.1 base (buckets base)).1 s.1) else s.1
      let r' := Dag.sweepF m tk rest cod.2 (Node.wireCount n base) buckets tm.2 enq enc (o.append Δ) ((Mask.const _ true).append mk) s.2 part.2
      ⟨(s1, r'.cells), (part.1, r'.part), r'.outs, r'.mask, Node.encodeMsgs m n cod.1 base enc snd ++ r'.msgs, r'.base⟩
```

The fused knot loop `knotLoopF` encodes only in the body sweep with every knot known (`fin := Mask.all b`), since that is the sweep that leaves the block's sends final. If the loop exits earlier (an unknown external, or fuel exhausted) it encodes by one walk over the parts' sends instead.

Before the sweep, `bucketArray` splits the inbox by wire node. `Node.inboxOfWire` at a network node scans every batch for that node's number; `bucketArray` computes every node's bucket in one pass, and `bucketArray_get` says bucket `k` is `bucketOf batches k`, the batches with only node `k`'s messages. `decodeAt` reads only those messages (`decodeAt_filter`).

`Node.sweepF_eq`/`Dag.sweepF_eq` state that the fused sweep equals the separate walks, node for node, unconditionally (no validity hypothesis: the fused sweep performs the same work, rearranged). The compiled activation and its equation are then:

```lean
def Dag.iterateBytesImpl (m : Member L mem) (tk : Bool) {Γ : Ctx L mem} {outs : List (Kind L mem)} (d : Dag Γ outs)
    (cod : Dag.Codecs d) (tm : Dag.Timing m d) (chans : List ((_src : Member L mem) × ByteArray))
    (Δ : HList (Kind.locDelta m) Γ) (s : Dag.LocState m d) :
    Dag.LocState m d × HList (Kind.locDelta m) outs × List WireMsg × List ByteArray :=
  let parsed := chans.map fun c => (⟨c.1, parseFrames c.2⟩ : (_src : Member L mem) × List WireMsg × ByteArray)
  let batches := parsed.map fun c => (⟨c.1, c.2.1⟩ : (_src : Member L mem) × List WireMsg)
  let arr := bucketArray (Dag.wireCount d 0) batches
  let r := Dag.sweepF m tk d cod 0 (fun k => (arr[k]?).getD (bucketOf batches k)) tm true true Δ (Mask.const Γ true) s
    (Dag.Part.init m d)
  (r.cells, r.outs, r.msgs, parsed.map fun c => c.2.2)

@[csimp] theorem Dag.iterateBytes_eq_impl : @Dag.iterateBytes = @Dag.iterateBytesImpl := by
  funext L mem _ m tk Γ outs d cod tm chans Δ s
  simp only [Dag.iterateBytes, Dag.iterateBytesImpl, Dag.iterateWire, Dag.iterate, Dag.sweepTop,
    List.map_map, Function.comp_def]
  rw [Dag.sweepF_eq m tk _ _ (fun k => bucketArray_getD _ _ k)]
  simp only [Dag.sweepSpec, SweepRes.ofTuple, ↓reduceIte, Dag.sweep_sends]
```

The bucket lookup falls back to `bucketOf batches k` when the index is out of range, so the equation needs no bound on `k`. On the leader's idle activation the full activation is 477 µs fused against 632 µs with separate walks; the rest of the gap to one visit per node is closed by [staging](./staging.md), which executes this sweep at elaboration time.

## The `@[csimp]` Seams

`@[csimp] theorem f_eq : @f = @g` tells Lean's compiler to replace the constant `f` by `g` wherever it compiles code. The replacement happens in the compiler's front end, constant for constant: it inspects no arguments. The equation is an ordinary theorem, checked by the kernel, so a seam may change how a value is computed: sharing, the data structure, the number of walks, reading bytes in place. It may not change what is computed, because the logic cannot tell `f` from `g`. It also cannot specialize: a seam cannot say "`Dag.iterateBytes` at this graph is this straight-line code", so the staged activations are definitions with their own equations rather than seams ([Staging](./staging.md)).

These equations are the only place where the compiled program differs from the terms the theorems are about:

| file | seam | what the implementation changes |
|---|---|---|
| `Hydro/Project/Fused.lean` | `Dag.iterateBytes_eq_impl` | the fused sweep in place of five walks |
| | `bucketArray_eq_impl` | buckets by an array indexed by node number, no hash map |
| `Hydro/Project/Wired.lean` | `memoBox_eq_memoBoxImpl`, `memoBox2_eq_memoBox2Impl` | per-member functions materialized as tables ([Processes](./processes.md)) |
| | `Dag.locStep_eq_impl` | nodes the process does not touch skipped |
| `Hydro/MemberMap.lean` | `mk'_eq_mkImpl` | each member's cell forced once |
| `Hydro/Project/Process.lean` | `parseFrames_eq_impl` | frames decoded in place at their offset ([Wire Codecs and Byte-Level Traces](./wire-and-traces.md)) |
| | `frameMsgs_eq_impl` | messages framed straight onto one buffer |
| `Hydro/Wire.lean` | `getNat_eq_impl` | LEB128 with a multiplier instead of `<<<` (the runtime's shift allocates a big integer) |
| | `frameAll_eq_impl` | a left fold onto a growing buffer instead of the quadratic right-nested `++` |
| `Hydro/KeyedHash.lean` | `dedupH_eq_dedupImpl`, and `arrFilterNotIn`, `arrAntiJoin`, `arrJoinH`, `keyedFoldAssocH`, `keyedFoldArrH`, `keyedReduceAssocH`, `SchedKMap.joinH`, `reduceWatermarkEagerH`, each `_eq_impl` | hash-backed keyed operators in place of their list definitions |

A seam is a theorem about the logic, so the compiled code is covered only as far as the compiler is trusted to implement the logic. The framework checks that edge by execution: `lake exe wired --traces` runs the scripted byte-level traces under the compiled activation and under the **interpreter** `Dag.iterateBytes` (the function that walks the graph as data) and compares what they commit, and `Hydro/HydroStageCheck.lean` `#guard`s the staged toy against the interpreter on data.
