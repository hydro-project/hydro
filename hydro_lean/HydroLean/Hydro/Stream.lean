import HydroLean.Hydro.Tick

/-!
# Streams and live-collection operators (dissertation §4.3)

The denotational carrier for a `TotalOrder + ExactlyOnce` stream (Rust:
`Stream<T, L, Unbounded, TotalOrder, ExactlyOnce>`) is simply `List α`: the
deterministic sequence of elements the stream settles to. Safe operators are
total functions on carriers (Chapter 4's API), so *eventual determinism is
definitional at this layer* — the operational justification is the Flo/Gyatso
metatheory (`Flo/Theorems.lean`).

Everything here is computable: Hydro programs written against this API run
with `#eval` (the Lean analogue of executing under the simulator with a fixed
schedule) *and* support unbounded proofs.

Naming follows the Rust API (§4.3); each operator's doc comment cites its Rust
counterpart. Operators whose Rust versions demand markers or algebraic
side-conditions are documented accordingly:

- `fold`/`scan` here require the `List` carrier (= `TotalOrder + ExactlyOnce`),
  matching Rust's `Stream::fold` marker bound (§4.3.4);
- the `NoOrder` variants (`fold_commutative` etc.) arrive with the `Multiset`
  carrier in `HydroLean/Collections/` and take the algebraic laws as real
  hypotheses rather than Rust's `manual_proof!` promises.

`Singleton`s (§4.3.3) with a fixed settled value are just values `β`;
`Optional`s are `Option β`.
-/

namespace HydroLean.Hydro

universe u v w

/-- The `TotalOrder + ExactlyOnce` stream carrier (§4.3.1). Kept as an
`abbrev` so the whole `List` API remains available on streams. -/
abbrev Stream (α : Type u) : Type u := List α

namespace Stream

variable {α : Type u} {β : Type v}

/-! ### One-by-one transformations (marker-generic in Rust, §4.3.2) -/

/-- Rust: `Stream::map` (Fig 4.9). -/
def map (s : Stream α) (f : α → β) : Stream β := List.map f s

/-- Rust: `Stream::filter`. -/
def filter (s : Stream α) (p : α → Bool) : Stream α := List.filter p s

/-- Rust: `Stream::filter_map`. -/
def filterMap (s : Stream α) (f : α → Option β) : Stream β := List.filterMap f s

/-- Rust: `Stream::flat_map` (used e.g. for broadcast, Fig 4.12). -/
def flatMap (s : Stream α) (f : α → List β) : Stream β := List.flatMap f s

/-- Rust: `Stream::chain` (concatenate a bounded stream before another; in
`collect_quorum`, `not_all.chain(new_inputs)`). -/
def chain (s t : Stream α) : Stream α := s ++ t

/-- Rust: `Stream::enumerate` — requires `TotalOrder + ExactlyOnce` (Fig 4.9),
which is exactly this carrier. -/
def enumerate (s : Stream α) : Stream (Nat × α) := (List.zipIdx s).map (fun (a, i) => (i, a))

/-! ### Aggregations (§4.3.3): streams to singletons -/

/-- Rust: `Stream::fold` (Fig 4.10) — safe only on `TotalOrder + ExactlyOnce`
(this carrier). The result is the settled value of the output `Singleton`. -/
def fold (s : Stream α) (init : β) (f : β → α → β) : β := List.foldl f init s

/-- Rust: `Stream::reduce` — like `fold` without an initial value; the settled
value of the output `Optional` (§4.3.3). -/
def reduce (s : Stream α) (f : α → α → α) : Option α :=
  match s with
  | [] => none
  | x :: xs => some (List.foldl f x xs)

/-- Rust: `Stream::cross_singleton` (§4.5.2): pair each element with (a
snapshot of) a singleton's value. -/
def crossSingleton (s : Stream α) (v : β) : Stream (α × β) := s.map (·, v)

/-! ### `across_ticks` aggregations: the tick face of unbounded folds

Rust's `s.across_ticks(|s| s.agg())` aggregates the *whole unbounded* stream
and presents the running value inside each tick. The tick face threads the
previous tick's value explicitly (`prev`): one tick's contribution is the
fold of that tick's batch into `prev`. The batch is `NoOrder`, so using these
requires the aggregation to be commutative — the `manual_proof!` sites of the
Rust callers; here the obligation is the corresponding sealing lemma
(`countKeyP_perm`-style), proven once per aggregation. -/

/-- Rust: `s.across_ticks(|s| s.max()).into_singleton()` — the running max as
an in-tick `Option` singleton (`none` until the first element). `max` must be
commutative-associative (a semilattice; e.g. any total order's max). -/
def acrossTicksMax (s : Stream α) (prev : Option α) (max : α → α → α) :
    Option α :=
  s.fold prev (fun acc a =>
    some (match acc with
      | none => a
      | some b => max b a))

/-- Assoc-list `insert_with`: combine with the existing entry at the key, or
append a fresh entry. -/
def insertWith [DecidableEq κ] {V : Type v} (comb : V → V → V) (k : κ) (v : V) :
    List (κ × V) → List (κ × V)
  | [] => [(k, v)]
  | (k', w) :: rest =>
    if k' = k then (k', comb w v) :: rest
    else (k', w) :: insertWith comb k v rest

/-- Rust: `s.into_keyed().reduce_watermark(…, comb)` under `across_ticks` —
the running keyed reduce as an in-tick map singleton (garbage collection
below the watermark dropped, as authorized). `comb old new` must be
commutative in the multiset sense on the inputs actually supplied — the
`manual_proof!` obligation of the Rust call site. -/
def acrossTicksKeyedReduce [DecidableEq κ] {V : Type v}
    (s : Stream (κ × V)) (prev : List (κ × V)) (comb : V → V → V) :
    List (κ × V) :=
  s.fold prev (fun m kv => insertWith comb kv.1 kv.2 m)

/-! ### Key-based operators (`KeyedStream`, used by `collect_quorum`) -/

/-- Rust: `Stream::anti_join` — keep pairs whose key is *not* in `ks`. -/
def antiJoin [DecidableEq κ] {V : Type v} (s : Stream (κ × V)) (ks : Stream κ) :
    Stream (κ × V) :=
  s.filter (fun p => !ks.contains p.1)

/-- Rust: `Stream::filter_not_in` — keep elements not present in `other`. -/
def filterNotIn [DecidableEq κ] (s : Stream κ) (other : Stream κ) : Stream κ :=
  s.filter (fun k => !other.contains k)

/-- The distinct keys of a keyed stream, in first-appearance order
(Rust: `KeyedStream::keys`, modulo emission order — the Rust output is
`NoOrder`, so any order refining the same set is a valid denotation; we pick a
canonical one). -/
def keys [BEq κ] {V : Type v} (s : Stream (κ × V)) : Stream κ :=
  (s.map Prod.fst).eraseDups

/-- Count the values for key `k` (cardinality of `KeyedStream` group). -/
def countKey [DecidableEq κ] {V : Type v} (s : Stream (κ × V)) (k : κ) : Nat :=
  s.countP (fun p => p.1 = k)

/-- Count values for key `k` satisfying `p`. `collect_quorum`'s
`into_keyed().fold(counting Ok/Err)` is `countKeyP · isOk` / `countKeyP · isErr`
— the Rust closure's commutativity obligation (`manual_proof!(/** increment
counters is commutative */)`) is discharged here *by construction*: counting
is order-invariant (see `countKeyP_perm`). -/
def countKeyP [DecidableEq κ] {V : Type v} (s : Stream (κ × V)) (k : κ)
    (p : V → Bool) : Nat :=
  s.countP (fun q => decide (q.1 = k) && p q.2)

/-- Rust: `KeyedStream::fold` over the values of one key, in stream order
(Rust-parity surface operator — kept whether or not currently consumed). -/
def keyedFold [DecidableEq κ] {V : Type v} (s : Stream (κ × V)) (init : β)
    (f : β → V → β) (k : κ) : β :=
  ((s.filter (fun q => q.1 = k)).map Prod.snd).foldl f init

/-! ### Order-invariance lemmas

These are the `List`-level facts that make counting-based logic a *congruence
for the `NoOrder` quotient*: permuting the input does not change the result.
They are what wave 2 uses to transport `collect_quorum` correctness to
`NoOrder` input streams (the Rust `Order: Ordering` generic). -/

/-- Count-API completer (sibling of `countKeyP_perm`; kept as the keyed-count
face whether or not currently consumed). -/
theorem countKey_perm [DecidableEq κ] {V : Type v} {s t : Stream (κ × V)}
    (h : s.Perm t) (k : κ) : countKey s k = countKey t k :=
  h.countP_eq _

theorem countKeyP_perm [DecidableEq κ] {V : Type v} {s t : Stream (κ × V)}
    (h : s.Perm t) (k : κ) (p : V → Bool) : countKeyP s k p = countKeyP t k p :=
  h.countP_eq _

/-! ### Basic algebra of the keyed operators -/

@[simp] theorem countKey_append [DecidableEq κ] {V : Type v}
    (s t : Stream (κ × V)) (k : κ) :
    countKey (s ++ t) k = countKey s k + countKey t k := by
  simp [countKey, List.countP_append]

@[simp] theorem countKeyP_append [DecidableEq κ] {V : Type v}
    (s t : Stream (κ × V)) (k : κ) (p : V → Bool) :
    countKeyP (s ++ t) k p = countKeyP s k p + countKeyP t k p := by
  simp [countKeyP, List.countP_append]

end Stream

/-- The settled value of a `Singleton` (§4.3.3) is just a value; kept as a
transparent alias for documentation. -/
abbrev Singleton (β : Type v) : Type v := β

/-- The settled value of an `Optional` (§4.3.3). -/
abbrev Optional (β : Type v) : Type v := Option β

end HydroLean.Hydro
