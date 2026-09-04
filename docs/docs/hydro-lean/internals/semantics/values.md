---
sidebar_position: 2
title: "The Denotational Semantics"
---

# The Denotational Semantics

`Values` (`hydro_lean/Hydro/Values.lean`) is the **denotational semantics**: the instance of the type class `HydroSem` at which a program's meaning, its **denotation**, is a pure function from inputs and explicit decisions to outputs, where each stream value captures all the histories the stream could have had. Contracts ([Contracts](../../using/programs/contracts.md)) are stated about it, and the executables (`#guard_eval`, `lake exe falsify`, `lake exe explore`) run it. Its stream carriers are the graded quotients of [Grades](./grades.md), which also defines grade, carrier, member and pool.

Four more terms recur. A **wire** is a collection-valued binding in a program (`let b := H.batch …`). Every carrier is **member-indexed**: it holds one cell per member of its location, read as `s i`. A **tick** is one firing of a `sliced!` block at one member. A **trace** (`Trace σ := List σ`) is a tick-located value across all realized ticks, so entry `n` is tick `n`'s value.

Sending a stream over the network does not change the stream's denotation ([below](#the-network-as-identity)). Every `nondet!` site takes its choice as an input from a total space, so a theorem over all inputs needs no side condition ([Decisions as Cuts](#decisions-as-cuts)). At `Values` a `sliced!` block of any arity is one fold over ticks that a proof reads tick by tick ([Tick Shapes](#tick-shapes)). The executable computes each wire once, which Lean's compiler does not do for a function-valued carrier without help ([`MemberMap`](#membermap)).

## Carriers

| signature family | `Values` carrier, per member | Rust |
|---|---|---|
| `Stream ℓ α ord ret` | `PoolCarrier α ord ret` (the grade's quotient) | `Stream<α, ℓ, Unbounded, ord, ret>` |
| `Singleton ℓ α σ ord ret b` | `CutDec α ord → Trace σ` (a read function) | `Singleton<σ, ℓ, b>` |
| `Atomic ℓ σ` | `Trace σ` | `Singleton<σ, Tick<ℓ>, Bounded>` across ticks |
| `AtomicStream ℓ α ord ret` | `Trace (PoolCarrier α ord ret)` | `Stream<α, Tick<ℓ>, Bounded, ord, ret>` across ticks |
| `KeyedStream ℓ K V ord ret` | `KeyedPool K V ord ret` | `KeyedStream<K, V, ℓ, Unbounded, ord, ret>` |
| `BoundedStream α ord ret` | `PoolCarrier α ord ret` | one tick's batch, inside a tick body |

A folded singleton is not a value but a **read function**. A fold over an unbounded stream has no value until someone reads it. What a read sees depends on how much input has arrived, and the `snapshot` site's `nondet!` decides that amount. So `fold` produces the function from a read decision (a cut, [below](#decisions-as-cuts)) to the trace of reads, and `snapshot` applies it:

```lean
  fold {_ℓ _α _σ _ ord ret} g init Pre _ ok s := .mk' fun i d =>
    snapTrace ord ret Pre g init ok (s i) d
  …
  snapshot {_ℓ _α _σ _ _ord _ret b} s d :=
    match b, s with
    | .unbounded, s => .mk' fun i => s i (d i)
    | .monotonic _vo, s => .mk' fun i => (s i).val (d i)
```

At the `Monotonic` bound (Rust's `Singleton<σ, ℓ, Monotonic>`, whose reads only ascend in a declared order) the cell is a subtype, `{f : CutDec α ord → Trace σ // ∀ d, Ascending vo (f d)}`, so ascent of every read is bundled into the carrier and proved once by `snapTrace_ascending`.

## The Network as Identity

At `Values` a network hop changes nothing: the receiver's stream holds the senders' pools, keyed by sender. [The Approach](../approach.md#quotient-denotations) explains why, and how that differs from treating the network as state. A network edge is `broadcast_closed` (every member of the receiving location gets every sender's stream) or `demux` (each element is addressed to one receiver). At `Values` both only re-key the senders' pools by sender:

```lean
  broadcast_closed _d s := .mk' fun _p => KeyedPool.transport (fun j => s j)
  demux {_c _p _α _ ord} _d s addr := .mk' fun i => KeyedPool.transport (fun j =>
    filterMapPool (ord := ord)
      (fun dx => if dx.1 = addr i then some dx.2 else none) (s j))
```

`KeyedPool.transport` (`Hydro/Keyed.lean`) builds the keyed stream whose key is the sender member and whose group at sender `j` is `j`'s pool, unchanged:

```lean
def transport {c : Nat} (s : Fin c → PoolCarrier V ord ret) :
    KeyedPool (Fin c) V ord ret :=
  ⟨Finset.univ, s, fun k hk => absurd (Finset.mem_univ k) hk⟩
```

The transport decision `_d` has type `Unit` at `Values` and is unused: there is no delivery to decide, because the pool's quotient type already identifies every arrival order, and how much has arrived by a tick is decided where `batch` or `snapshot` observes it. Every receiver sees the same keyed stream (`_p` is ignored), and a `demux` receiver sees the senders' pools filtered to its address. The rewrite rules that state this are `rfl`:

```lean
@[den] theorem values_broadcast_closed {c : L} {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (d : (Values L mem).TransportDec (mem ℓ) (mem c))
    (s : (Values L mem).Stream c α ord ret) (p : Fin (mem ℓ)) :
    (Values L mem).broadcast_closed d s p = KeyedPool.transport (fun j => s j) := rfl
```

```lean
@[den] theorem values_broadcast_sum {c : L} {α : Type} [DecidableEq α]
    (d : (Values L mem).TransportDec (mem ℓ) (mem c))
    (s : (Values L mem).Stream c α .noOrder .exactlyOnce) (p : Fin (mem ℓ)) :
    (Values L mem).values ((Values L mem).broadcast_closed d s) p
      = ((List.finRange (mem c)).map (fun j => s j)).sum := rfl
```

`values_broadcast_sum` is what a proof uses after `.values()` drops the sender key: at an unordered exactly-once grade the receiver's pool equals the sum of the senders' pools. `values_broadcast_sum_totalOrder` states the same equation with each sender's list coerced to a multiset (the merge of several ordered senders is unordered), `values_demux` and `values_demux_sum` state the `demux` forms (`demux_sum` is `rfl` after a case split on the ordering), and at the retry grades, where the merged pool is a class with forgotten counts, `values_broadcast_mem_alo` and `values_broadcast_mem_stutter` give membership: `b` is received if and only if some sender's pool holds it.

A proof about a program that crosses the network therefore rewrites the hop away and reasons about the senders' pools directly. The time a message takes, the order in which messages from different senders interleave, and whether the receiver is awake are all absent here. They exist only in the step machine, whose delivery reads its sender one step back ([The Step Machine](./sched.md#delivery-and-the-1-floor)), and the coupling relates the two: in the denotational run `rr` a hop is this identity operator, and in the machine run `sr` it costs one step. The coupling records that step as one unit of **lag** (how many steps short of its horizon the coupling holds for a wire) ([The Coupling Corner](../corner/coupling.md#the-network-hop)).

## Decisions as Cuts

Every `nondet!` site takes a **decision**: an argument whose type is a field of the instance (`BatchDec`, `SnapDec`, `OrderSelDec`, …), so that each instance chooses its own nondeterminism vocabulary ([Nondeterminism](../../using/programs/nondeterminism.md)). At `Values` the decision of a consumption site is a **cut**: per member and per tick, the consumed increment itself. For `.batch(&tick, nondet!(…))` on `NoOrder, ExactlyOnce` content, a cut is the multiset the tick consumes (`BatchCuts n α := Fin n → List (Multiset α)`):

```lean
def batchCuts {α : Type _} [DecidableEq α] (pool : Multiset α)
    (consumed : Multiset α) : (d : List (Multiset α)) →
      Trace (Multiset α)
  | [] => []
  | b :: ds =>
    if consumed + b ≤ pool then b :: batchCuts pool (consumed + b) ds
    else []
```

The decision list is arbitrary data, and most lists are not realizable: they consume elements the pool does not have. The `else []` makes the space total anyway. At the first illegal cut the trace stops, so the run at an illegal decision covers the ticks before it: the same trace a run that stopped ticking there would produce. A theorem quantified over every `d : BatchCuts n α` therefore quantifies over realizable runs and their truncations, and needs no premise "`d` is legal". Hydro's own simulator treats the batch boundary the same way, as an input: `Stream::batch`'s documentation says that in simulation tests "the batching decisions can be scripted by attaching a `BatchHook` to the guard".

Blocking, as opposed to clamping an illegal cut to what is available, is also what makes `batch` monotone in its input at a fixed decision. A larger pool only admits more of the same cuts:

```lean
theorem batchCuts_le {α : Type _} [DecidableEq α] {pool pool' : Multiset α}
    (h : pool ≤ pool') (consumed : Multiset α) (d : List (Multiset α)) :
    batchCuts pool consumed d <+: batchCuts pool' consumed d
```

A clamping semantics would rewrite tick 0's batch whenever the pool grew, and the batch trace would not grow by prefix. [MonoRel](./monorel.md) depends on `batchCuts_le` and its siblings.

The grade decides what a cut is and when it is legal (`BatchCutsOf` in `Hydro/Decisions.lean`):

| grade | decision per tick | legal when | denotation |
|---|---|---|---|
| `NoOrder, ExactlyOnce` | `Multiset α` increment | consumed so far plus it fits the pool | `batchCuts` |
| `NoOrder, AtLeastOnce` | `Finset α` | every element is in the pool's support; nothing is consumed | `retryBatchCuts` |
| `TotalOrder, ExactlyOnce` | slice size `Nat` | the slice stays inside the realized sequence | `sliceCuts` |
| `TotalOrder, AtLeastOnce` | the realized `List α` | the destuttered concatenation so far is a prefix of the pool's normal form | `stutterBatchCuts` |

Snapshot cuts follow the same pattern (`prefixCuts`, `snapshotCuts`, `snapshotMemCuts`), and so does `assume_ordering`, whose selection draws without replacement and stops at the first element the pool does not hold (`selectOrder`).

The per-tick order selection behind `tassume_ordering` blocks at the level of whole ticks:

```lean
def realizeTicks {α : Type _} [DecidableEq α] :
    Trace (Multiset α) → List (List α) → Trace (List α)
  | p :: ps, s :: ss => if Multiset.ofList s = p then s :: realizeTicks ps ss else []
  | _, _ => []
```

A tick is realized only by a permutation of that tick's batch. The first tick whose selected list is not one cuts the trace, so a batch is never realized in an order that is not an order of its contents. `latestMonoCell` uses the same device to give `latest_monotone` its `Monotonic` bound without a premise: its reads stop at the first cut whose prefix does not ascend, so an unjustified `Monotonic` claim shows up as a blocked read and not as a smoothed value.

Totality does not give adequacy in the other direction. The decision space over-approximates: `selectOrder` licenses every permutation of a pool, including orders that no run of the step machine ([The Step Machine](./sched.md)) produces. Safety theorems are sound under over-approximation. The coupling, which derives the denotation's decisions from a machine run instead of quantifying over them, is covered in [The Coupling Corner](../corner/coupling.md).

### Knots

At `Values` a `forward_ref` cycle, a **knot**, is Kleene iteration from the empty wire, with its depth as a decision (`FixDec := UnfoldFuel`, a `Nat`):

```lean
  fix_stream {_ℓ _α _ ord ret} fuel body :=
    iterate body (.mk' fun _i => PoolBot ord ret) fuel
  fix_tick fuel body := iterate body (.mk' fun _i => []) fuel
```

The fuel is a decision like any other, so a contract over a knot holds at every depth. The coupling ([The Coupling Corner](../corner/coupling.md#knots)) sets it from the machine run, to one more than the number of machine steps it relates. The iterates `iterate body ⊥ m` are the knot's **stages**. [Knot Invariants](../../using/proving/knot-invariants.md) shows how a loop invariant becomes an obligation over them.

## Tick Shapes

A `sliced!` block reads any number of wires (singletons, batches, keyed batches), keeps any number of registers (`use::state`), and emits any mix. A structure field of `HydroSem` cannot be arity-polymorphic, so the signature has one former, `tick_scan`, indexed by a **shape**: the type-level zip of the block's inputs (and, separately, its registers and its emissions):

```lean
inductive TickShape : Type 1 where
  | sing (τ : Type)
  | stream (α : Type) (inst : DecidableEq α) (ord : StrOrd) (ret : Retries)
  | ksing (K V : Type) (instK : DecidableEq K) (instV : DecidableEq V)
  | kstream (K V : Type) (instK : DecidableEq K) (instV : DecidableEq V)
      (ord : StrOrd) (ret : Retries)
  | pair (a b : TickShape)
```

`AtomicOf … ℓ sh` computes the tuple of wires a shape stands for, and `BoundedOf … sh` the tuple of this tick's values. Both are recursive functions over the instance's carrier families, so `tick_scan`'s type is well-formed at every instance (`Hydro/Sem.lean`). You write a `tick` block. Here is `toy_relay`'s (`Hydro/HydroGenToy.lean`):

```lean
  tick (input bb := b) (input g := gate) :=
    yield (emitted := H.bfilterIf bb (H.boIsSome (H.bfirst g)));
```

The `tick` construct computes the shapes from the wires' types and generates the unzip inside the body. In the dataflow graph the generator compiles each module into ([Graph IR and Causality](../execution/graph-ir-causality.md)), from `#print toy_relay.dag`:

```lean
          (let emitted_step := fun me st inpt =>
            have bb := inpt.1;
            have g := inpt.2;
            (HydroSem.bsPure (), HydroSem.bfilterIf bb (HydroSem.boIsSome (HydroSem.bfirst g)));
          Project.Dag.letN
            (Project.Node.tick_scan (TickShape.sing Unit)
              ((TickShape.stream ℕ instDecidableEqNat StrOrd.noOrder Retries.exactlyOnce).pair
                (TickShape.stream Unit instDecidableEqPUnit StrOrd.totalOrder Retries.atLeastOnce))
              (TickShape.stream ℕ instDecidableEqNat StrOrd.noOrder Retries.exactlyOnce)
              …
```

The register shape is `sing Unit` because the block is stateless, the input shape is a pair of two batch streams of different grades, and the emission is one batch stream. At `Values` the former slices the inputs into one trace of per-tick tuples (`Trace.zip`, truncated to the jointly realized ticks), scans the body over it, and unslices the emissions:

```lean
def scan {ℓ : L} (sts ins outs : TickShape)
    (x : AtomicOf (Tk mem) (TS mem) (TKS mem) (TKStr mem) ℓ ins)
    (g : Fin (mem ℓ) → BoundedOf BS PoolCarrier BKS BKStr sts →
      BoundedOf BS PoolCarrier BKS BKStr ins →
      BoundedOf BS PoolCarrier BKS BKStr sts × BoundedOf BS PoolCarrier BKS BKStr outs)
    (init : SeedOf sts) : AtomicOf (Tk mem) (TS mem) (TKS mem) (TKStr mem) ℓ outs :=
  unslice outs (.mk' fun i => scanAcrossTicksTrace (g i) (seed sts init) (slice ins x i))
```

Hydro compiles no instantaneous cycle inside a tick, so a register is a fold (`scanAcrossTicksTrace`, a Mealy machine over ticks) and not a fixpoint. A stream register (`use::state_null`) starts at the grade's `PoolBot`.

The shaped former is definitionally a scan, but a proof that states the zipped form with `show` makes the elaborator unfold the former by definitional reduction, `TickShape.rec` by the hundred thousand. `Values.lean` therefore states each step of the unfolding as an `rfl` rewrite rule, and proofs `rw` with them:

```lean
theorem tick_scan_stream {α : Type} [inst : DecidableEq α] {ord : StrOrd}
    {ret : Retries} (sts ins : TickShape)
    …
    (Values L mem).tick_scan sts ins (.stream α inst ord ret) x g init i
      = scanAcrossTicksTrace (g i) (seed sts init) (slice ins x i) := rfl

theorem slice_pair (a b : TickShape)
    (x : AtomicOf (Tk mem) (TS mem) (TKS mem) (TKStr mem) ℓ (.pair a b)) (i : Fin (mem ℓ)) :
    slice (.pair a b) x i = Trace.zip (slice a x.1 i) (slice b x.2 i) := rfl
```

The `tick` elaborator builds on these rules the per-block **readers** that users see, lemmas that state an output's value at tick `n` in terms of that tick's inputs (`hemitted_at`, the loop invariant) ([Tick Invariants](../../using/proving/tick-invariants.md), [The Tick Elaborator](../generator/tick-elaborator.md)).

## Keyed Pools and Legal Views

A keyed stream has an order per key, at its grade, and no order across keys. Its carrier is a finite map with graded content at each key, and no cross-key order exists anywhere in it:

```lean
structure KeyedPool (K V : Type) [DecidableEq K] [DecidableEq V]
    (ord : StrOrd) (ret : Retries) where
  keys : Finset K
  grp : K → PoolCarrier V ord ret
  dom : ∀ k, k ∉ keys → grp k = PoolBot ord ret
```

`into_keyed`, a cast at the runtime, becomes grouping here (`KeyedPool.ofPool`). The network builds its keyed stream with the per-sender map `KeyedPool.transport` ([above](#the-network-as-identity)).

A keyed fold's live map (`kfold`, `kfold_early_stop`) is read like a folded singleton, through cuts. Its cut is a multiset of `(key, value)` pairs, and legality has to respect the per-key grade:

```lean
def legalView : {ord : StrOrd} → KeyedPool K V ord .exactlyOnce →
    Multiset (K × V) → Bool
  | .totalOrder => fun p M =>
      decide (∀ k ∈ (M.map Prod.fst).toFinset,
        (M.filter (fun e => e.1 = k)).map Prod.snd
          = ↑((p.grp k).take ((M.filter (fun e => e.1 = k)).card)))
  | .noOrder => fun p M => decide (M ≤ p.toMultiset)
```

At `TotalOrder` the cumulative cut must, at every key it touches, consist of that key's next values in order: a prefix per key, interleaved arbitrarily across keys. At `NoOrder` it must fit the pool. `kviews` blocks at the first illegal cumulative view, as `batchCuts` does, and `legalView_mono`, the keyed form of `batchCuts_le`, makes a view legal for a pool legal for every larger pool.

## `MemberMap`

Mathematically a member-indexed carrier is a function `Fin n → C`. As a bare function it executes call-by-name. Lean's compiler eta-expands every declaration to the arity of its type (`Decl.etaExpand`), so in `let v := Vector.ofFn f; fun i => v.get i` the `let` moves under the lambda and is recomputed at every read, and every read re-runs the chain that produced the wire. Through module composition and knot iteration the cost compounds. With bare functions, one output of the leader-election scenario run read its `p2b` input 4,775,952 times (48.6 s).

A structure return type is not eta-expanded, so the carrier is a structure (`Hydro/MemberMap.lean`):

```lean
structure MemberMap (n : Nat) (C : Type u) where
  cells : Fin n → C
  boxed : Unit := ()

def mkImpl (f : Fin n → C) : MemberMap n C :=
  let v := Vector.ofFn f
  ⟨fun i => v.get i, ()⟩

@[reducible]
def mk' (f : Fin n → C) : MemberMap n C := ⟨f, ()⟩

@[csimp] theorem mk'_eq_mkImpl : @mk' = @mkImpl := by
  funext n C f
  exact (mkImpl_eq f).symm
```

The proofs and the compiled code need different constructors, and the `@[csimp]` theorem connects them. The kernel and the `den` rewrite rules (which read one operator at a time at `Values`, [Proofs over Values](../../using/proving/proofs-over-values.md)) need `(mk' f) i` to be `f i` definitionally, by projection of a constructor, which `mkImpl` with its `Vector` does not give. The compiled code needs `mkImpl`, which forces each member's cell once at construction. Every operator in `Values` builds with `.mk'`. `@[csimp]` lets the compiler substitute `mkImpl` only through a kernel-checked equation, so the executable computes the denotation. Every split between compiled and logical code in the framework is a `@[csimp]` of this kind.

The field `boxed : Unit` keeps the sharing intact through compilation. The compiler erases a structure with exactly one relevant field to that field (`hasTrivialStructure?`), so with `cells` alone a `MemberMap` is a function again to the later passes over LCNF, the compiler's intermediate representation. Two rewrites then destroy the forcing. `Simp` eta-expands local lambdas whose erased type is an arrow, which turns a `fix_stream` into a function that re-runs the whole Kleene iteration per read. `simpAppApp?` merges `let s := mkImpl n g; s i` into `mkImpl n g i` inside the consumer's cell lambda. A second relevant field makes the map a heap object with no arrow type to expand. [Processes](../execution/processes.md) covers the same compiler behaviour in the graph's memo tables.

`MemberMap.get` is a `def`, not the projection `cells`, because the unifier skips its same-head check for projection functions, and `get A i =?= get A' i` would then `whnf` the whole upstream program. `Values` is `@[reducible]` so that `simp` sees its fields, but instance lookup does not unfold a class projection of a reducible instance (`unfoldProjInst?`), so `s i` on `(Values L mem).Stream ℓ α ord ret` needs its own `CoeFun` instance per carrier family (`instCoeFunValuesStream` and its siblings).
