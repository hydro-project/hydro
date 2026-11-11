---
sidebar_position: 3
title: "Carriers, Grades and Operators"
---

# Carriers, Grades and Operators

A Hydro Lean program is built from the live collections and stream operators of `hydro_lang`, and the Lean API mirrors the Rust API: a `Stream` carries its ordering and retries markers, `.map` becomes `H.map`, and `.batch(&tick, nondet!)` becomes `H.batch s d`. What differs is where the operators come from. They are the fields of one Lean class, `HydroSem L mem` in `hydro_lean/Hydro/Sem.lean`, the **signature**, together with the **carriers** (the Lean types of the live collections) and the decision families. A program is a function of an instance of the class, and each semantics is an instance, also called an **interpretation**: the denotational semantics `Values`, in which a stream captures all the histories it could have had; the step machine `SchedSem`, in which a stream is a list that grows on a global clock; and the coupling `CoupleSem`, which runs both and relates them ([Tagless-Final Programs](../../internals/approach.md#tagless-final-programs)).

Each semantics represents a stream differently, so the program text names no representation. What it does state is each stream's **grade**, its ordering and retries markers. As in Rust, the grade decides which operators need a `nondet!` and which algebraic properties a fold must have; in Lean those properties are proofs the fold takes as arguments. This page documents what a program may say: the carriers and the Rust types they stand for, the grades and the fold proofs, the operators, values inside a tick, closures, and cycles.

## Carrier Families

Each carrier family takes the location and the element types. The stream carriers also take two indices, `StrOrd` and `Retries`, which together are the stream's grade:

```lean
class HydroSem (L : Type) (mem : L → Nat) where
  Stream : L → (α : Type) → [DecidableEq α] → StrOrd → Retries → Type
…
  Atomic : L → Type → Type
…
  BoundedStream : (α : Type) → [DecidableEq α] → StrOrd → Retries → Type
…
  BoundedSingleton : Type → Type
```

Each instance defines the families. At `Values` (`Hydro/Values.lean`) a stream is one **pool** per member: its content, which captures all its possible histories, in a type that depends on the grade; at the step machine (`Hydro/Sched.lean`) it is a per-member step history, and the batch of a **tick** (one firing of a `sliced!` block at one member) is an `Array`, as in the runtime:

```lean
@[reducible] def Values (L : Type) (mem : L → Nat) : HydroSem L mem where
  Stream ℓ α _ ord ret := MemberMap (mem ℓ) (PoolCarrier α ord ret)
```

```lean
  BoundedStream α _ _ _ := Array α
```

A carrier is a **whole collection**: `H.Stream ℓ α ord ret` holds the stream at every member of `ℓ` at once, `MemberMap (mem ℓ) _` at `Values` ([Members](./distributed-programs.mdx#members)).

## Carriers and Rust Types

Each carrier corresponds to one Rust live-collection type. The table lists the correspondence for porting; the carriers' meaning comes from the instances, not from the Rust types.

| Lean carrier | Rust type |
|---|---|
| `Stream ℓ α ord ret` | `Stream<α, ℓ, Unbounded, ord, ret>` |
| `KeyedStream ℓ K V ord ret` | `KeyedStream<K, V, ℓ, Unbounded, ord, ret>`; the network's outputs are keyed by sender (`K = Fin (mem c)`) |
| `Singleton ℓ α σ ord ret b` | `Singleton<σ, ℓ, b>` with `b` ∈ {`Unbounded`, `Monotonic`} |
| `KeyedSingleton ℓ α K V ord ret b` | `KeyedSingleton<K, V, ℓ, b>` with `b` ∈ {`Unbounded`, `BoundedValue`} |
| `Atomic ℓ σ` / `Atomic ℓ (Option σ)` | `Singleton<σ, Tick<ℓ>, Bounded>` / `Optional<σ, Tick<ℓ>, Bounded>`, as a wire across ticks |
| `AtomicStream ℓ α ord ret` | `Stream<α, Tick<ℓ>, Bounded, ord, ret>`, as a wire across ticks |
| `AtomicKeyedSingleton ℓ K V`, `AtomicKeyedStream ℓ K V ord ret` | the keyed tick collections, as wires across ticks |
| `BoundedStream α ord ret`, `BoundedSingleton σ`, `BoundedKeyedStream K V ord ret`, `BoundedKeyedSingleton K V` | the same collections **inside** a tick body: this tick's value |

The carriers form three layers.

- **Live collections at a root location** (`Stream`, `KeyedStream`, `Singleton`, `KeyedSingleton`) grow asynchronously.
- **Tick wires** (`Atomic`, `AtomicStream`, …) hold one value per tick per member. At `Values` an `Atomic ℓ σ` is the realized sequence of tick values; at the machine it also records at which step each tick ran. Rust's `Atomic<ℓ>` location holds the same per-tick trace, so `all_ticks_atomic` and `batched_atomic` are identities here, and Rust's `x.across_ticks(|s| s.op(…))` is written `H.op x …` on the atomic wire.
- **Bounded values** exist only inside a `tick` body, where the in-tick operators act on them.

A `Singleton` carries its source's element type `α` and grade `ord ret`, which Rust's `Singleton<σ, ℓ, b>` does not. At `Values` a folded singleton is a read function: given per-tick **cuts** of the source stream (how much of the source each `snapshot` read has seen, supplied as a decision, see [Nondeterminism](./nondeterminism.md)), it returns the fold's value at each read. The type of those cuts depends on the source's grade (prefix lengths for an ordered source, multiset increments for an unordered one), so the carrier must remember it.

## Grades

A running stream has one history: its elements arrived in some order, and with at-least-once delivery some of them arrived more than once. The denotational semantics does not pick that history. A stream captures all the histories it could have had, and the nondeterminism stays unresolved inside the value until an operator observes it ([Deferred Nondeterminism](../../internals/approach.md#deferred-nondeterminism) explains the idea). The grade says which aspects of the history are left open:

| grade | left open | `Values` content carrier | Rust markers |
|---|---|---|---|
| `.totalOrder, .exactlyOnce` | nothing | `List α` | `TotalOrder, ExactlyOnce` |
| `.noOrder, .exactlyOnce` | the arrival order | `Multiset α` | `NoOrder, ExactlyOnce` |
| `.totalOrder, .atLeastOnce` | how often each element is re-delivered in place | `StutterSeq α` | `TotalOrder, AtLeastOnce` |
| `.noOrder, .atLeastOnce` | the order and the number of copies | `RetryPool α` | `NoOrder, AtLeastOnce` |

The carrier column is `PoolCarrier α ord ret` (`Hydro/Grades.lean`): a quotient that identifies exactly the histories the grade leaves open. [Grades](../../internals/semantics/grades.md#quotient-carriers) gives the construction. An operator on a stream is available without a decision exactly when its result is the same on every history in the class:

- **Available at every grade.** `bisEmpty` (emptiness does not depend on order or copies), `bfilterIf`, `values` (the interleaving across senders is left open in its `.noOrder` result), `union` of two unordered streams (likewise), and `broadcast_closed`, whose senders' pools are re-keyed by receiver at `Values`, so a transport changes nothing a proof can see.
- **Available at `.exactlyOnce`.** `bcount` takes `.exactlyOnce` input at either order: a count is the same for every arrival order but changes with re-deliveries. `bsort` also takes `.exactlyOnce` input at either order and returns `.totalOrder`, so an order computed from the content needs no decision. `bmax`, `bmin`, `bfilter`, `demux` and the stream-level `map` and `filterMap` are typed at `.exactlyOnce` as well.
- **Available at `.totalOrder`.** `benumerate` and `bfirst` take `.totalOrder` input, because the position of an element and the first element differ between arrival orders. To apply them to an unordered stream, the program first chooses one order with `assume_ordering` or `tassume_ordering`, which take an order-selection decision ([Nondeterminism](./nondeterminism.md)).
- **Loosening is free.** `weaken_retries`, `bweakenOrder` and `bkweakenOrder` move a stream to a grade that leaves more open, and every operator of the looser grade still applies.

### Fold Proofs

A fold observes the whole content, so it is well defined on the class only if its closure cannot tell the histories apart. The signature makes that proof an argument of every fold operator (`fold`, `bfold`, `bkfold`, `kfold`, `breduce`, `reduce_watermark`, …). Its type, `FoldOk ord ret Pre g`, is computed from the grade:

```lean
@[reducible] def FoldOkP {α σ : Type _} :
    StrOrd → Retries → (Pre : α → α → Prop) → (σ → α → σ) → Prop
  | .totalOrder, .exactlyOnce => fun _ _ => True
  | .noOrder, .exactlyOnce => fun Pre g => ∀ s x y, Pre x y → g (g s x) y = g (g s y) x
  | .totalOrder, .atLeastOnce => fun _ g => ∀ s x, g (g s x) x = g s x
  | .noOrder, .atLeastOnce => fun Pre g =>
      (∀ s x y, Pre x y → g (g s x) y = g (g s y) x) ∧ (∀ s x, g (g s x) x = g s x)
```

Each case asks for what the grade leaves open: commutativity where the order is open, idempotence on a repeated element where re-delivery is, both where both are, and nothing for a fully determined list. These are the proofs Rust's `commutative = …` and `idempotent = …` annotations stand for ([Proof Obligations](../../../hydro/reference/correctness/proof-obligations.md)). A fold with the wrong algebra has no proof to pass, so the program does not typecheck. A fold whose closure is order-dependent cannot be written on an unordered stream at all; the program first chooses an order with `assume_ordering`, which is a decision.

`Pre` states the protocol fact under which a `manual_proof!` holds. The Paxos log merge commutes only on entries where equal ballots carry equal values, so commutativity is demanded only on pairs satisfying `Pre`. At `Values` such a fold refuses content whose pairs violate `Pre` and returns its seed ([Grades](../../internals/semantics/grades.md#conditional-commutativity)). A program never relies on that branch: a fold with a non-trivial `Pre` requires a `ghost pre` at the site proving that its input satisfies `Pre`, and the definition does not elaborate without one ([Proofs over Values](../proving/proofs-over-values.md#requires-and-ghost-pre)).

## Operators

Operator names follow `hydro_lang`. Stream-level closures take the member index first, which is Rust's `CLUSTER_SELF_ID` capture. An argument `d` is a **decision**: an explicit argument that resolves a choice the operator observes (a `nondet!` site), or the runtime timing the step machine needs, with a type that each interpretation declares ([Nondeterminism](./nondeterminism.md)).

| Lean | Rust | note |
|---|---|---|
| `map`, `filterMap` | `.map`, `.filter_map` | |
| `broadcast_closed d s` | `.broadcast(&cluster, …)`, `.send(&process, …)` | closed membership; `d : TransportDec`; an identity operator on content at `Values` |
| `demux d s addr` | `.demux(&cluster, …)` | `addr` reads the receiver off the payload |
| `values` | `.values()` | the result is `.noOrder` |
| `into_keyed`, `kentries`, `kmap` | `.into_keyed()`, `KeyedStream::entries()`, `KeyedStream::map` | |
| `union` | `.merge_unordered(…)` | no decision; the interleaving is left open in the `.noOrder` result |
| `weaken_retries` | `.weaken_retries()` | |
| `assume_ordering s d`, `kassume_ordering s d` | `.assume_ordering::<TotalOrder>(nondet!)` | `d : OrderSelDec` |
| `fold g init Pre ok`, `fold_monotone vo g init ok h` | `.fold(…, commutative = …)`, with `monotone = …` | `ok : FoldOk ord ret Pre g` |
| `kfold`, `kfold_early_stop`, `kget_max_key` | `KeyedStream::fold`, `.fold_early_stop`, `.get_max_key()` | |
| `snapshot s d`, `ksnapshot s d` | `.snapshot(&tick, nondet!)` | `d : SnapDec` |
| `batch s d` | `.batch(&tick, nondet!)`, `use::batch` | `d : BatchDec`, graded like `s` |
| `source_interval delay period d` | `source_interval(_delayed)` | `d : IntervalDec` |
| `time d`, `snapshot_bounded x d` | `use::time(nondet!)`, `use::snapshot` of a bounded singleton | `d : ClockDec`, `TicksDec` |
| `firstTick`, `mapTick`, `zipTick` | `.first()`, `.map`, `.zip` on tick collections | |
| `defer_tick init a` | `.defer_tick()` | tick 0 reads `init` |
| `allTicks`, `kallTicks` | `.all_ticks()` | the grade is preserved |
| `flattenOrdered`, `flattenUnordered` | `.flatten_ordered()`, `.flatten_unordered()` | |
| `tinto_keyed`, `tassume_ordering s d`, `tfold` | `across_ticks(…)` with `into_keyed`, `assume_ordering`, `fold` | `d : TickOrderSelDec` |
| `reduce_watermark` | `.reduce_watermark(wm, …)` | garbage collection below the watermark |
| `latest`, `latest_monotone` | `Optional::latest()` | the second for a `Monotonic` consumer |
| `fix_stream`, `fix_tick` (via `HydroSem.fix`, `fixTick`) | `forward_ref` / `complete` | `d : FixDec` |
| `tick_scan` | `sliced!` with `use::state` | written by the `tick` construct only |

The in-tick operators are Rust's `Tick`-located API, named with a prefix for the carrier they act on:

| prefix | carrier | operators |
|---|---|---|
| `b` | `BoundedStream` | `bmap`, `bfilterMap`, `bfilter`, `bflatMapOrdered`, `bflatMapUnordered`, `bofList`, `bcount`, `bisEmpty`, `bfold`, `breduce`, `benumerate`, `bfirst`, `bmax`, `bmin`, `bsort`, `bchain`, `bweakenOrder`, `bcrossSingleton`, `bcrossOptional`, `bjoin`, `bantiJoin`, `bfilterNotIn`, `bfilterIf`, `binto_keyed` |
| `bk` | `BoundedKeyedStream` | `bkfold`, `bkreduce`, `bkmap`, `bkchain`, `bkcrossSingleton`, `bkweakenOrder` |
| `bks` | `BoundedKeyedSingleton` | `bksFilter`, `bksMap`, `bksKeys`, `bksEntries`, `bksJoin`, `bksFilterKeyNotIn`, `bksKeyCount`, `bksIntoStream` |
| `bs` | `BoundedSingleton σ` | `bsPure`, `bsMap`, `bsZip`, `bsFilterIf`, `bsFilterMap`, `bsZipOptional` |
| `bo` | `BoundedSingleton (Option σ)` | `boMap`, `boUnwrapOr`, `boOr`, `boFilterIf`, `boFilter`, `boFilterMap`, `boIsSome`, `boIsNone`, `boZip`, `boZipSingleton`, `boIntoStream` |

`bkfold`, `bkreduce`, `bjoin`, `bantiJoin`, `bfilterNotIn` and `bksJoin` carry `[Hashable K]` where Rust demands `K: Hash`, because the machine implements them with a hash map. Rust's `Optional` inside a tick is `BoundedSingleton (Option σ)`, spelled `H.BoundedOptional σ`.

## In-Tick Values

`H.BoundedSingleton σ` is abstract in the program, so a tick body cannot take a singleton apart with Lean's own syntax. Combining two singletons goes through `bsZip`, as in `index_payloads` (`Examples/Paxos/IndexPayloads.lean`):

```lean
    -- next_slot = num_payloads.zip(base_slot).map(q!(|(num_payloads, base_slot)| base_slot + num_payloads));
    rebind (next_slot := H.bsMap (H.bsZip num_payloads base_slot)
      (fun (num_payloads, base_slot) => base_slot + num_payloads))
```

At `Values` and at the machine `BoundedSingleton σ` is `σ` and `bsZip a b` is `(a, b)`. The indirection is needed by the coupling instance (`Hydro/Couple.lean`), where every carrier is a **corner** ([Corner Carriers](../../internals/corner/coupling.md#corner-carriers)): it pairs the machine run `sr` with the denotational run `rr`, together with a proposition `wf` and a proof `cpl` that the two runs agree given `wf`:

```lean
structure CoBSing (σ : Type) : Type where
  sr : σ
  rr : σ
  wf : Prop
  cpl : wf → sr = rr
```

Every `tick` block elaborates to one call of `tick_scan`, a single signature field whose inputs, registers and emissions are each described by a `TickShape`, a type-level zip of the wires' kinds, so one field covers every arity. You never write it; the `tick` construct computes the shapes from your binders ([Tick Blocks](./ticks-and-cycles.mdx#tick-blocks)). At the coupling instance `tick_scan` runs the body once and projects the machine run and the denotational run from the result. The projection works because every value inside the body is a corner holding both runs, as every wire outside the body is. A value taken apart with Lean syntax keeps only one run, and the projection then fails. The same holds for bounded streams: the machine run's batch is an `Array` and the denotational run's a quotient pool, and each in-tick operator proves, at the coupling instance, that its two implementations agree.

## Closures

A `q!` closure is a Lean `fun`. Nothing is quoted or staged, so the closure the proof reads is the closure the machine runs.

- Stream-level closures have type `Fin (mem ℓ) → α → β`; in-tick closures `α → β`. Write `fun _me x => …` where the Rust closure does not capture the member id.
- A closure's algebraic property is a theorem about the function, passed where the operator's type asks for it (`cqCount_comm` for `bkfold cqCount (0, 0) …`, in [Porting from Rust](../quickstart/porting-from-rust.mdx)).
- Pattern-matching lambdas are ordinary `match`es. `fun (key, (success, _error)) => …` appears in generated statements as `fun x => match x with | (key, success, _error) => …`, which `simp` reduces on a constructor.
- The **production graph**, the typed dataflow graph `hydro def` generates for a program and the executable processes run, captures the closure as data. The graph generated for `hydro_lean/Hydro/HydroGenToy.lean`'s `toy_relay` begins with `Project.Node.map instDecidableEqNat instDecidableEqNat Project.Var.zero fun _i n => n + 1`: the program's `fun _i n => n + 1`, unchanged, at a node of the graph the processes execute.
- At `Values` a program is executable, so `#guard` evaluates it on concrete inputs and decisions.

## Cycles

Rust's `forward_ref` closes a cycle. The signature field is `fix_stream d body` with `body : Stream ℓ α ord ret → Stream ℓ α ord ret` at the current instance. Programs use a derived form whose body is generic over the instance:

```lean
def HydroSem.fix {L : Type} {mem : L → Nat} (H : HydroSem L mem)
    {Γ : HydroSem L mem → Type} {ℓ : L} {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries}
    (d : H.FixDec) (caps : Γ H)
    (body : ∀ (H' : HydroSem L mem), Γ H' →
        H'.Stream ℓ α ord ret → H'.Stream ℓ α ord ret) :
    H.Stream ℓ α ord ret :=
  H.fix_stream d (body H caps)
```

`caps` is the tuple of wires and decisions the body captures, and `Γ` is its type as a function of the instance. A cycle closed this way is a **knot**. Because the body quantifies over `H'`, a proof can instantiate the same body at another interpretation: at `MonoRel` (whose carriers are pairs of runs, one below the other) for monotonicity, and at the coupling instance to couple the knot's wire by induction on the step bound. A structure field cannot quantify over the structure's own type, so this form lives outside the class. You write a `fix (w : τ) via fuel … := body` block that ends in `complete (v)`, and the elaborator builds the capture tuple, hoists the body into its own top-level `def` and calls `HydroSem.fix` ([Cycles](./ticks-and-cycles.mdx#cycles)). `fuel` is the knot's `FixDec`, the depth to which `Values` iterates the body from the empty collection ([Nondeterminism](./nondeterminism.md)).
