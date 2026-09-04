---
sidebar_position: 1
title: "Quotient Carriers and Fold Obligations"
---

# Quotient Carriers and Fold Obligations

In the denotational semantics of Hydro Lean a stream captures all the histories it could have had, so its nondeterminism stays unresolved inside the value until an operator observes it ([The Approach](../approach.md#quotient-denotations) gives what it replaces and costs). Concretely, each stream grade has a quotient type as its carrier, and every operator on that carrier is a `Quotient.lift` whose respect proof shows the operator gives one answer on the whole class. An operator with no such proof does not exist at that grade and has to take a decision instead. A fold that commutes only under a condition on its content checks the condition and returns its seed on content that violates it.

The vocabulary used throughout:

- A Hydro stream carries two markers, an ordering (`TotalOrder` or `NoOrder`) and a retry guarantee (`ExactlyOnce` or `AtLeastOnce`). The pair is the stream's **grade**.
- A program is written once against the type class `HydroSem` and run at several **instances** of it. `Values` is the **denotational semantics**, the one contracts are stated about ([Values](./values.md)); a program's meaning there is its **denotation**. `SchedSem` is the **operational semantics**, a step machine that runs the program step by step ([The Step Machine](./sched.md)). `CoupleSem` runs both at once and proves that the machine's runs stay below the denotation ([The Coupling Corner](../corner/coupling.md)).
- A **carrier** is the type an instance assigns to a kind of collection. A location can be a cluster of `mem ℓ` **members**, and a carrier holds one cell per member. `PoolCarrier α ord ret` is the denotation's carrier for one member's stream content of grade `(ord, ret)`, called its **pool**.
- A **decision** is an explicit argument that resolves one `nondet!` site ([Nondeterminism](../../using/programs/nondeterminism.md)).

## Quotient Carriers

A grade determines which histories a stream can have. An unordered stream can have arrived in any order. An at-least-once stream can have delivered any element any positive number of times. Each grade's carrier is the quotient of raw histories by the differences the grade allows (`hydro_lean/Hydro/Grades.lean`):

```lean
@[reducible] def PoolCarrier (α : Type _) [DecidableEq α] :
    StrOrd → Retries → Type _
  | .totalOrder, .exactlyOnce => List α
  | .totalOrder, .atLeastOnce => StutterSeq α
  | .noOrder, .exactlyOnce => Multiset α
  | .noOrder, .atLeastOnce => RetryPool α
```

| grade | carrier | histories identified | normalized view |
|---|---|---|---|
| `TotalOrder, ExactlyOnce` | `List α` | none | the list |
| `NoOrder, ExactlyOnce` | `Multiset α` = `List α / Perm` | reorderings | (none needed) |
| `TotalOrder, AtLeastOnce` | `StutterSeq α` = `List α / stutter` | consecutive re-deliveries | `destutter l` |
| `NoOrder, AtLeastOnce` | `RetryPool α` = `Multiset α / support` | reorderings and re-deliveries | the support `Finset` |

`Multiset` is Mathlib's. The two retry quotients are built in the framework from core `Quotient`. At `TotalOrder, AtLeastOnce` the transport is ordered, so a retry re-delivers in place: it duplicates an element next to itself and never moves it past another. Two arrival lists are the same history class when they agree after collapsing consecutive duplicates:

```lean
def destutter {α : Type _} [DecidableEq α] : List α → List α
  | [] => []
  | [x] => [x]
  | x :: y :: rest =>
    if x = y then destutter (y :: rest) else x :: destutter (y :: rest)

def stutterSetoid (α : Type _) [DecidableEq α] : Setoid (List α) where
  r a b := destutter a = destutter b
  iseqv := ⟨fun _ => rfl, Eq.symm, Eq.trans⟩

def StutterSeq (α : Type _) [DecidableEq α] : Type _ :=
  Quotient (stutterSetoid α)
```

At `NoOrder, AtLeastOnce` neither order nor count is fixed, but no element is lost, so what is left is membership. Two arrival multisets are the same class when they have the same members:

```lean
def supportSetoid (α : Type _) : Setoid (Multiset α) where
  r a b := ∀ x, x ∈ a ↔ x ∈ b
  …
def RetryPool (α : Type _) : Type _ := Quotient (supportSetoid α)
```

The carrier is the class and not its normalized view. The normal form exists (`StutterSeq.norm`, `RetryPool.support`), but it is a derived function, and the histories live anywhere in the class, duplicates included. Therefore `StutterSeq.mk l` of the machine's actual arrival list is a value of the carrier, and the lemmas below (`foldlOn_coe`, `foldOn_mk_coe`) state that folding the class equals folding that list as the machine does. With the normal form as the carrier, every operator's correspondence proof would first need a lemma relating the machine's list to its normal form.

## Lifting Operators to Quotients

A function leaves a quotient only through `Quotient.lift f h`, where `h` proves that `f` gives equal results on identified histories. Each operator on a quotient carrier is such a lift, and its respect proof `h` makes the operator well defined at that grade. Membership in a retry pool is the simplest case:

```lean
def Mem (x : α) (p : RetryPool α) : Prop :=
  Quotient.lift (fun m => x ∈ m)
    (fun a b h => by
      have h' : ∀ y, y ∈ a ↔ y ∈ b := h
      exact propext (h' x)) p
```

Union (`H.union` on `AtLeastOnce` streams) lifts in both arguments. The respect proof is that support-equal inputs give support-equal sums:

```lean
def union (a b : RetryPool α) : RetryPool α :=
  Quotient.lift₂ (fun m n => mk (m + n))
    (fun m n m' n' hm hn => by
      have hm' : ∀ y, y ∈ m ↔ y ∈ m' := hm
      have hn' : ∀ y, y ∈ n ↔ y ∈ n' := hn
      exact Quotient.sound fun y => by
        rw [Multiset.mem_add, Multiset.mem_add]
        exact or_congr (hm' y) (hn' y))
    a b
```

`.first()` on ordered at-least-once content is a lift because destuttering never changes the head (`destutter_head?`):

```lean
def head? (s : StutterSeq α) : Option α :=
  Quotient.lift List.head?
    (fun a b h => by
      have h' : destutter a = destutter b := h
      rw [← destutter_head? a, ← destutter_head? b, h']) s
```

An operator that is not well-defined on a grade's quotient has no lift there, so the signature (`Hydro/Sem.lean`) does not offer it at that grade. For the in-tick operators that read a whole batch, the Rust trait bounds mostly draw the same line:

| operator | grades it exists at | why | Rust bound |
|---|---|---|---|
| `bfirst` (`.first()`) | `TotalOrder`, either retry grade | the head is a function of a stutter class, not of a multiset | `O: IsOrdered` |
| `bcount` (`.count()`) | `ExactlyOnce` only | the count is not a function of a stutter or support class | `count` is in `impl … Stream<T, L, B, O, ExactlyOnce>` |
| `benumerate` | `TotalOrder, ExactlyOnce` | positions exist only in a list | `O: IsOrdered, R: IsExactlyOnce` |
| `bisEmpty` | every grade | emptiness is a function of every class (`s.norm.isEmpty`, `support p = ∅`) | none |
| `bsort` | `ExactlyOnce`, either order | at `NoOrder`, `Multiset.sort` is well-defined because a `LinearOrder` leaves no ties between distinct elements | any retry grade; the Lean signature is narrower |
| `weaken_retries` | `ExactlyOnce → AtLeastOnce` | a coarsening: `StutterSeq.mk`, `RetryPool.mk` are functions out of the finer carrier, so no proof is owed | none |

`weaken_retries` and `bweakenOrder` (`.weaken_ordering()`) move to a coarser quotient. That direction is always a function. The opposite direction, `assume_ordering`, picks a representative, and picking one is a decision ([below](#order-dependent-folds)).

## Fold Obligations

A fold is a lift too, and its respect proof establishes the algebraic property that makes the fold a function of the class, the property Rust's `commutative =` and `idempotent =` arguments assert. For `StutterSeq` the proof needs `foldl_destutter`, which holds only for a step that is idempotent on consecutive repeats:

```lean
/-- **The consumer gate**: a fold exists only for consecutive-idempotent steps, because the
respect proof of the lift is the idempotence obligation. -/
def fold {σ : Type _} (g : σ → α → σ) (init : σ)
    (idem : ∀ s x, g (g s x) x = g s x) (s : StutterSeq α) : σ :=
  Quotient.lift (fun l => l.foldl g init)
    (fun a b h => by
      have h' : destutter a = destutter b := h
      rw [foldl_destutter g idem a init, foldl_destutter g idem b init, h'])
    s
```

The four grades share one eliminator, `PoolFold`. It takes the grade's obligation as an argument whose type `FoldOkP` computes from the grade:

```lean
@[reducible] def FoldOkP {α σ : Type _} :
    StrOrd → Retries → (Pre : α → α → Prop) → (σ → α → σ) → Prop
  | .totalOrder, .exactlyOnce => fun _ _ => True
  | .noOrder, .exactlyOnce => fun Pre g => ∀ s x y, Pre x y → g (g s x) y = g (g s y) x
  | .totalOrder, .atLeastOnce => fun _ g => ∀ s x, g (g s x) x = g s x
  | .noOrder, .atLeastOnce => fun Pre g =>
      (∀ s x y, Pre x y → g (g s x) y = g (g s y) x) ∧ (∀ s x, g (g s x) x = g s x)

def PoolFold {α σ : Type _} [DecidableEq α] :
    (ord : StrOrd) → (ret : Retries) → (Pre : α → α → Prop) → [DecidableRel Pre] →
    (g : σ → α → σ) → (init : σ) →
    (ok : @FoldOkP α σ ord ret Pre g) → PoolCarrier α ord ret → σ
  | .totalOrder, .exactlyOnce => fun _ _ g init _ l => l.foldl g init
  | .noOrder, .exactlyOnce => fun Pre _ g init comm m =>
      Multiset.foldlOn Pre g init comm m
  | .totalOrder, .atLeastOnce => fun _ _ g init idem s =>
      StutterSeq.fold g init idem s
  | .noOrder, .atLeastOnce => fun Pre _ g init ok p =>
      RetryPool.foldOn Pre g init ok.1 ok.2 p
```

A fold with this obligation resolves deferred nondeterminism without a decision. `collect_quorum` (`hydro_std/src/quorum.rs`, mirrored in `Hydro/Std/Quorum.lean`) counts each key's responses inside a tick. The Rust first, then the Lean:

```rust
        let count_per_key = current_responses.clone().into_keyed().fold(
            q!(move || (0, 0)),
            q!(move |accum, value| {
                if value.is_ok() {
                    accum.0 += 1;
                } else {
                    accum.1 += 1;
                }
            }, commutative = manual_proof!(/** increment counters is commutative */)),
        );
```

```lean
    let count_per_key := H.bkfold cqCount (0, 0) (fun _ _ => True)
      (fun s x y _ => cqCount_comm s x y) (H.binto_keyed current_responses)
```

The batch is `NoOrder`, so its denotation is a multiset class and no arrival order has been chosen. The third argument is the prerequisite `Pre` (here `fun _ _ => True`, explained [below](#conditional-commutativity)), and the fourth is the `FoldOk` proof, here the unconditional `cqCount_comm`. With it the count is one value for the whole class. Nothing downstream depends on an order, and the program takes no decision for one. The Rust's `manual_proof!` is a comment the compiler trusts; the Lean argument is a proof the kernel checks.

`PoolFold` is the only eliminator over a whole pool, and the signature re-exports `FoldOkP` as `FoldOk` for every folding operator (`fold`, `bfold`, `bkfold`, `breduce`, `tfold`, `kfold`, `reduce_watermark`). A fold over `NoOrder` content with a closure that does not commute has no `ok` argument, so it does not typecheck. `FoldOkP` is `@[reducible]` because programs pass obligation proofs inline and the corner's tactics (`co_transfer`, [Projections and Tactics](../corner/projections-and-tactics.md)) match on their types at reducible transparency.

## Conditional Commutativity

Rust's `commutative = manual_proof!(…)` often claims commutativity only under a protocol fact. Paxos's log merge (`paxos.rs:898–906`, `if entry.ballot > prev_entry.ballot { *prev_entry = entry }`) commutes on two entries only if they carry the same value whenever they carry the same ballot. The Paxos port defines the prerequisite as its own predicate, `LogTie`:

```lean
def LogTie {P : Type} {nP : Nat} (x y : LogValue P nP) : Prop :=
  x.ballot = y.ballot → x.value = y.value
```

`FoldOkP` therefore asks for commutativity only on pairs satisfying a program-chosen `Pre`. That is enough for a well-defined fold on content whose pairs all satisfy `Pre`, because Mathlib's `List.Perm.foldl_eq'` needs commutativity only on the pairs of the list being permuted. On other content no well-defined answer exists, so the fold checks the content:

```lean
def Multiset.foldlOn {α σ : Type _} (Pre : α → α → Prop) [DecidableRel Pre]
    (g : σ → α → σ) (init : σ)
    (hcomm : ∀ s x y, Pre x y → g (g s x) y = g (g s y) x) (m : Multiset α) : σ :=
  Quot.liftOn m (fun l => if PairwiseL Pre l then l.foldl g init else init)
    (fun l₁ l₂ h => by
      have hp : PairwiseL Pre l₁ ↔ PairwiseL Pre l₂ := PairwiseL.perm h
      by_cases h1 : PairwiseL Pre l₁
      · rw [if_pos h1, if_pos (hp.mp h1)]
        exact h.foldl_eq' (fun x hx y hy z => hcomm z x y (h1 x hx y hy)) init
      · rw [if_neg h1, if_neg (fun h2 => h1 (hp.mpr h2))])
```

The `if` is what lets the lift go through. `PairwiseL Pre` is invariant under permutation (`PairwiseL.perm`), so two permutations of one multiset take the same branch. In the true branch `foldl_eq'` gets its commutativity on the list's pairs from `hcomm` and the content's own fact `h1`. In the false branch the list fold could depend on the order, and the function returns its seed, which is order-independent. The two branches are stated as theorems:

```lean
theorem foldlOn_coe {l : List α} (hl : PairwiseL Pre l) :
    foldlOn Pre g init hcomm (↑l : Multiset α) = l.foldl g init

theorem foldlOn_of_not_pairwise {m : Multiset α} (h : ¬ PairwiseM Pre m) :
    foldlOn Pre g init hcomm m = init
```

`foldlOn_coe` is the reading lemma: on good content the quotient fold equals the list fold of any representative, the machine's arrival order included. With `Pre := fun _ _ => True`, `foldlOn_true` recovers Mathlib's `Multiset.foldl`.

The retry-pool fold layers both quotients. It folds the deduplicated content, and its respect proof uses the fact that support-equal multisets have equal `dedup`:

```lean
def foldOn [DecidableEq α] {σ : Type _} (Pre : α → α → Prop) [DecidableRel Pre]
    (g : σ → α → σ) (init : σ)
    (hcomm : ∀ s x y, Pre x y → g (g s x) y = g (g s y) x)
    (_idem : ∀ s x, g (g s x) x = g s x) (p : RetryPool α) : σ :=
  Quotient.lift
    (fun m => if PairwiseM Pre m then Multiset.foldlOn Pre g init hcomm m.dedup else init)
    (fun a b h => by
      …
      have hd : a.dedup = b.dedup :=
        ((Multiset.nodup_dedup a).ext (Multiset.nodup_dedup b)).mpr …
      …)
    p
```

Idempotence is not needed to define `foldOn`, since `dedup` already removed the duplicates. It is needed for the reading lemma `foldOn_mk_coe`, which equates the class fold with `l.foldl g init` on the machine's arrival list with its duplicates. Its proof, `foldl_dedup_of`, uses commutativity on the list's pairs to move a repeated element next to its earlier copy, and idempotence to collapse the two.

### Refused Content

A non-`True` prerequisite is a content claim, and a program generic over the instance cannot prove content. A `hydro def` (a module: a program with a `requires` clause on its inputs and an `ensures` clause on its outputs, [Contracts](../../using/programs/contracts.md)) therefore must justify it at the operator site with a `ghost pre`, a proof inside the program text that the content reaching the operator is pairwise `Pre`, derived from the `requires` ([Proofs over Values](../../using/proving/proofs-over-values.md)). `toy_pre` (`Hydro/HydroTickCheck.lean`) is the minimal case. The Rust it mirrors is quoted in the module's comment, then the Lean:

```rust
// let m = batched.across_ticks(|s| s.into_keyed().reduce_watermark(wm,
//   q!(|prev, e| if e.0 > prev.0 { *prev = e }),
//   commutative = manual_proof!(requires: same key ∧ same ballot ⇒ same value)));
```

```lean
  requires (∀ i, ∀ a ∈ inp i, ∀ b ∈ inp i, a.1 = b.1 → ToyTie a.2 b.2)
  …
  ghost pre hpre : ∀ i, (KeyedPool.accum (keyed i)).Pairwise ToyTie :=
    fun i => KeyedPool.accum_tinto_keyed_batch_pairwise inp dec i (hreq i)
  let m := H.reduce_watermark keyed wm toyMerge ToyTie toyMerge_comm
```

The seed branch is therefore unreachable from a compiled `hydro def`. It remains reachable from a raw `Values` run on input that violates the `requires`. The `#guard`s and the falsifier (`lake exe falsify`, which runs `Values` on inputs built to break a contract) exercise it through such runs. Two entries at key `1` with ballot `5` and different values make the key's group fail the tie, and key `1` drops out:

```lean
#guard (toy_pre (Values Unit (fun _ => 1)) ()
    (.mk' fun _ => ({(1, (5, 100)), (1, (5, 200))} : Multiset (Nat × (Nat × Nat))))
    (.mk' fun _ => [none])
    (fun _ => [{(1, (5, 100)), (1, (5, 200))}])) 0
  = {}
```

`acceptor_p2`'s `#guard_eval` (`Examples/Paxos/AcceptorP2.lean`) shows the same at Paxos scale: a slot with two same-ballot entries carrying different values is absent from the log. `Values` executes a `manual_proof!` that the content does not support as this refusal: it produces neither an arbitrary value nor the result of one of the orders.

The step machine has no `if`: `SchedSem.bfold` is `b.foldl g init` on the runtime's array. The corner equates the two through the content fact the `ghost pre` proves, which it carries as `PoolPairwise` and which every list below the pool inherits (`PoolPairwise.of_coe_le`). See [The Coupling Corner](../corner/coupling.md).

## Order-Dependent Folds

Some folds really depend on order. The Paxos recommit fold (`paxos.rs:656–679`) counts the entries that carry the current champion's value and resets the count when a higher ballot brings a new value, so two orders of one batch can disagree on whether a slot is skipped. On `NoOrder` content no lift exists for such a fold. The program makes the order explicit with `assume_ordering` (per tick, `tassume_ordering`), whose decision fixes the order. At `Values` the selection draws without replacement from the multiset. `Multiset.erase` and membership are operations on the quotient, so the result is a function of the class and the decision:

```lean
def selectOrder {α : Type _} [DecidableEq α] (pool : Multiset α) :
    List α → List α
  | [] => []
  | x :: xs => if x ∈ pool then x :: selectOrder (pool.erase x) xs else []

  assume_ordering u d := .mk' fun i => selectOrder (u i) (d i)
```

The output is `TotalOrder, ExactlyOnce` content, where `FoldOkP` is `True`, so any closure folds it. A contract over the result is quantified over the selection, so it holds in every order. The recommit uses the per-tick form, `tassume_ordering`, whose denotation realizes each tick's batch by a permutation of it (`realizeTicks`, [Values](./values.md#decisions-as-cuts)); [Nondeterminism](../../using/programs/nondeterminism.md) shows its contract. Rust's own `count` is written the same way, as a fold after an internal `assume_ordering_trusted::<TotalOrder>(nondet!(/// Order does not affect eventual count, …))`.

### Monotone Folds

A folded `Singleton` is read at ticks. At each tick a **cut** (the read decision, [Values](./values.md#decisions-as-cuts)) says how much of the input has arrived. `fold_monotone` produces a `Singleton` with the `Monotonic` bound, meaning every trace of reads ascends. Its commutativity argument is `FoldOk ord ret (fun _ _ => True) g`, with no prerequisite. A refusable fold could return a folded value at one cut and `init` at a later cut whose content violates `Pre`, and the reads would descend. `snapTrace_ascending` (`Hydro/Values.lean`) is therefore stated for the unconditional obligation only.

## The Growth Order

Monotonicity (growing inputs give growing outputs at fixed decisions, [Monotonicity by Instantiation](./monorel.md)) and the coupling's bound of a machine run by a pool both need an order on pools. There is one per grade:

```lean
def PoolLe {α : Type _} [DecidableEq α] :
    (ord : StrOrd) → (ret : Retries) →
    PoolCarrier α ord ret → PoolCarrier α ord ret → Prop
  | .totalOrder, .exactlyOnce => fun a b => a <+: b
  | .totalOrder, .atLeastOnce => StutterSeq.le
  | .noOrder, .exactlyOnce => fun a b => a ≤ b
  | .noOrder, .atLeastOnce => RetryPool.le
```

`StutterSeq.le` is prefix of normal forms. That order describes arrival growth only because destuttering is prefix-stable: a longer arrival sequence destutters to an extension of the shorter one's normal form (`destutter_append_prefix`), so appending arrivals always moves up (`le_mk_append`). `RetryPool.le` is support inclusion, because the class forgets counts. `PoolBot` is the least element at each grade (`PoolLe.bot_le`), and `Values` iterates a stream **knot** (a `forward_ref` cycle) from that seed ([Values](./values.md#knots)).

The interval timer is a stutter class. `source_interval` is a `TotalOrder, AtLeastOnce` stream of `()`. Every non-empty run of `()`s destutters to `[()]` (`destutter_replicate_unit`), so the timer's denotation is the constant `StutterSeq.mk [()]`, and when the timer fires is visible only through the `batch` that reads it.

## Guarantees and Limits

- A theorem about `Values` content can only be about what every history in the class has in common, because nothing else is a function of the carrier.
- A closure without the algebra its grade requires does not typecheck. An algebra that holds only conditionally makes its content premise explicit (`Pre`, `ghost pre`) instead of leaving it inside a `manual_proof!`.
- The step machine does not see the quotients. `SchedSem` runs plain `Array`s and lists at every grade, in arrival order. The correspondence between a machine list and a class is `ListLe`, the grade's order read with the list as a representative, and proving it is a per-operator obligation of the corner ([The Coupling Corner](../corner/coupling.md)), not a coercion.
- The grades say nothing about order across keys. A keyed stream's `TotalOrder` is per key, and its carrier `KeyedPool` has no cross-key order ([Keyed pools](./values.md#keyed-pools-and-legal-views)).
- The carrier leaves batch boundaries open in the same way. The denotation of an unbounded stream is its whole pool, and where the batches fall is a cut decision at `batch` ([Values](./values.md#decisions-as-cuts)).
