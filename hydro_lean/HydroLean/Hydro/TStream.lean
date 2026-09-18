import HydroLean.Hydro.Stream
import HydroLean.Hydro.ForwardRef
import HydroLean.Hydro.ClusterFamily

/-!
# Tick-located streams and the decisions-as-inputs semantics

(The `simp` base cases, realized-view lemmas, and Rust-parity operators in
this module are the surface's documented interface — a face set, kept
intentionally whether or not each is currently consumed by a program.)

The denotational surface for **multi-tick Hydro functions** (user-ratified
design; supersedes clock/runner encodings for protocol programs):

- A collection located in a tick (`Stream<T, Tick<L>, Bounded>`) across the
  whole execution is a **stream of collections**: `TStream α := List (List α)`
  — the Flo §2.5 nesting (outer list = realized ticks, inner list = that
  tick's collection). A tick-located `Singleton`/`Optional` is one value per
  realized tick: `TSing β := List β`.
- **Operators act per inner collection**: `fold` on a tick stream folds each
  tick's batch (never across the outer list); `across_ticks` aggregations are
  the scans that carry state across the nesting.
- **Every `nondet!` site is an explicit decision input**: batch sites take the
  per-tick demand sequence, snapshot sites take the per-tick cut sequence.
  Properties are stated `∀ decisions, …` — the Lean mirror of the simulator's
  fuzzed decision space, with *no schedules and no interleavings*: given the
  decisions, ticks run left to right.
- **Monotonicity (Flo streaming progress)**: with decisions fixed, every
  operator's output tick-list must only *grow* as its inputs grow — a tick,
  once realized, is final. Operators therefore **block** instead of emitting
  a partial tick: `batch` realizes a tick only when its full demand is
  available, `snapshots` only when the cut is within the input, and any
  operator zipping several tick inputs yields outputs only up to the tick
  index where **all** inputs have a value. The `*_prefix` lemmas below are
  those framework guarantees.
- **`forward_ref` is a higher-order fixpoint** (`forward_ref` below): the
  body takes the cycle stream and returns (what it sends into the cycle,
  its other outputs); the combinator re-runs the whole body, feeding the
  grown cycle value back — a Kleene iteration, well-defined because outputs
  grow monotonically and batching consumes a maximal prefix of the decision
  stream given current input sizes. Users never write recursion; proofs use
  `iterate_ind` (fixpoint induction over reachable iterates — the ONE
  generic induction of the methodology) and `iterate_chain` (iterates form
  a ⊑-chain when the body is monotone, so every realized tick is final and
  within-tick knots are exact at every iterate).
- **Clusters are maps**: a cluster-located stream is a member-indexed family
  `Fin n → Stream α` (and a tick collection on a cluster is a map to
  stream-of-streams `Fin n → TStream α`); decisions for cluster clocks are
  likewise member-indexed. Networking `cluster → x` is *effectively the
  identity* on the keyed view: broadcast delivers each member's stream,
  demux is a `filterMap` on the destination tag — latency and loss are the
  consumer's cut decisions, never channel state. `NoOrder` fan-in from a
  cluster is `batchC`/`batchD` over the members' `unionF` multiset.

The operational lens (`TypedGraphSys`/`Choreo`) remains in the framework; a
coherence proof (denotational run = elaborated-system run at the I/O
boundary) is the staged follow-up recorded in docs/10.
-/

namespace HydroLean.Hydro

universe u v w

/-- A tick-located stream across the whole execution: outer = realized ticks,
inner = the tick's collection (Flo §2.5 stream-of-streams). -/
abbrev TStream (α : Type u) : Type u := List (List α)

/-- A tick-located singleton across the whole execution: one settled value
per realized tick. -/
abbrev TSing (β : Type u) : Type u := List β

/-! ## Scans: `use::state` and `across_ticks` tick faces -/

/-- Output scan: run a stateful per-tick step, collecting outputs (the tick
face of a `sliced!` block with `use::state`). -/
def scan {ι : Type u} {σ : Type v} {β : Type w} (f : σ → ι → σ × β) :
    σ → List ι → List β
  | _, [] => []
  | s, x :: xs => (f s x).2 :: scan f (f s x).1 xs

/-- State scan: the post-tick states (the tick face of an `across_ticks`
aggregation presented as an in-tick singleton). -/
def scanSt {ι : Type u} {σ : Type v} (f : σ → ι → σ) : σ → List ι → List σ
  | _, [] => []
  | s, x :: xs => f s x :: scanSt f (f s x) xs

@[simp] theorem scan_nil {ι σ β} (f : σ → ι → σ × β) (s : σ) :
    scan f s [] = [] := rfl

@[simp] theorem scan_cons {ι σ β} (f : σ → ι → σ × β) (s : σ) (x : ι)
    (xs : List ι) :
    scan f s (x :: xs) = (f s x).2 :: scan f (f s x).1 xs := rfl

@[simp] theorem scanSt_nil {ι σ} (f : σ → ι → σ) (s : σ) :
    scanSt f s [] = [] := rfl

@[simp] theorem scanSt_cons {ι σ} (f : σ → ι → σ) (s : σ) (x : ι)
    (xs : List ι) :
    scanSt f s (x :: xs) = f s x :: scanSt f (f s x) xs := rfl

theorem scan_length {ι σ β} (f : σ → ι → σ × β) (s : σ) (xs : List ι) :
    (scan f s xs).length = xs.length := by
  induction xs generalizing s with
  | nil => rfl
  | cons x xs ih => simp [ih]

theorem scanSt_length {ι σ} (f : σ → ι → σ) (s : σ) (xs : List ι) :
    (scanSt f s xs).length = xs.length := by
  induction xs generalizing s with
  | nil => rfl
  | cons x xs ih => simp [ih]

theorem scan_append {ι σ β} (f : σ → ι → σ × β) (s : σ) (xs ys : List ι) :
    scan f s (xs ++ ys) = scan f s xs ++ scan f (xs.foldl (fun a x => (f a x).1) s) ys := by
  induction xs generalizing s with
  | nil => rfl
  | cons x xs ih => simp [scan, ih]

theorem scanSt_append {ι σ} (f : σ → ι → σ) (s : σ) (xs ys : List ι) :
    scanSt f s (xs ++ ys) = scanSt f s xs ++ scanSt f (xs.foldl f s) ys := by
  induction xs generalizing s with
  | nil => rfl
  | cons x xs ih => simp [scanSt, ih]

/-- Prefix-monotonicity of scans: realized outputs are final. -/
theorem scan_prefix {ι σ β} (f : σ → ι → σ × β) (s : σ) {xs ys : List ι}
    (h : xs <+: ys) : scan f s xs <+: scan f s ys := by
  obtain ⟨t, rfl⟩ := h
  rw [scan_append]
  exact List.prefix_append _ _

theorem scanSt_prefix {ι σ} (f : σ → ι → σ) (s : σ) {xs ys : List ι}
    (h : xs <+: ys) : scanSt f s xs <+: scanSt f s ys := by
  obtain ⟨t, rfl⟩ := h
  rw [scanSt_append]
  exact List.prefix_append _ _

/-- The state visible at tick `t` is the fold of the consumed prefix — the
model equation connecting the scan face to run-level folds. -/
theorem scanSt_getElem {ι σ} (f : σ → ι → σ) (s : σ) (xs : List ι) (t : Nat)
    (ht : t < (scanSt f s xs).length) :
    (scanSt f s xs)[t] = (xs.take (t + 1)).foldl f s := by
  induction xs generalizing s t with
  | nil => simp [scanSt] at ht
  | cons x xs ih =>
    cases t with
    | zero => simp [scanSt]
    | succ t =>
      have ht' : t < (scanSt f (f s x) xs).length := by
        have := ht
        simp only [scanSt_length] at this ⊢
        simpa [scanSt] using Nat.lt_of_succ_lt_succ (by
          simpa [scanSt_length, List.length_cons] using this)
      simp only [scanSt, List.getElem_cons_succ, List.take_succ_cons,
        List.foldl_cons]
      exact ih (f s x) t ht'

/-- The scan output at tick `t` is the step applied to the fold of the
strictly-earlier prefix. -/
theorem scan_getElem {ι σ β} (f : σ → ι → σ × β) (s : σ) (xs : List ι)
    (t : Nat) (ht : t < (scan f s xs).length) (hx : t < xs.length) :
    (scan f s xs)[t]
      = (f ((xs.take t).foldl (fun a x => (f a x).1) s) (xs[t]'hx)).2 := by
  induction xs generalizing s t with
  | nil => simp at hx
  | cons x xs ih =>
    cases t with
    | zero => simp [scan]
    | succ t =>
      have hx' : t < xs.length := Nat.lt_of_succ_lt_succ hx
      have ht' : t < (scan f (f s x).1 xs).length := by
        rw [scan_length]
        exact hx'
      simp only [scan, List.getElem_cons_succ, List.take_succ_cons,
        List.foldl_cons]
      exact ih (f s x).1 t ht' hx'

/-- Relation of `scan` to `TickLoop.outputs` (the established run form). -/
theorem scan_eq_outputs {In St Out : Type _} (t : TickLoop In St Out)
    (bs : List In) : scan t.step t.init bs = t.outputs bs := by
  suffices h : ∀ (s : St) (bs : List In),
      scan t.step s bs = (t.runFrom s bs).2 by
    exact h t.init bs
  intro s bs
  induction bs generalizing s with
  | nil => rfl
  | cons b bs ih => simp [scan, TickLoop.runFrom_cons, ih]

/-- Relation of `scanSt` folds to `TickLoop.finalState`. -/
theorem foldl_eq_finalState {In St Out : Type _} (t : TickLoop In St Out)
    (bs : List In) :
    bs.foldl (fun s b => (t.step s b).1) t.init = t.finalState bs := by
  suffices h : ∀ (s : St) (bs : List In),
      bs.foldl (fun s b => (t.step s b).1) s = (t.runFrom s bs).1 by
    exact h t.init bs
  intro s bs
  induction bs generalizing s with
  | nil => rfl
  | cons b bs ih => simp [TickLoop.runFrom_cons, ih]

/-! ## Blocking combinators on tick-located values -/

namespace TStream

variable {α : Type u} {β : Type v}

/-- Per-tick `map` (Rust `Stream::map` on a tick stream). -/
def map (ts : TStream α) (f : α → β) : TStream β := List.map (List.map f) ts

/-- Per-tick `filter`. -/
def filter (ts : TStream α) (p : α → Bool) : TStream α :=
  List.map (List.filter p) ts

/-- Per-tick `filter_map`. -/
def filterMap (ts : TStream α) (f : α → Option β) : TStream β :=
  List.map (List.filterMap f) ts

/-- Per-tick `flat_map`. -/
def flatMap (ts : TStream α) (f : α → List β) : TStream β :=
  List.map (List.flatMap f) ts

/-- Rust `all_ticks()`: release the per-tick batches as one asynchronous
stream. Prefix-monotone because ticks are final. -/
def allTicks (ts : TStream α) : Stream α := ts.flatten

/-- Rust `cross_singleton` between a tick stream and a tick singleton:
tick-aligned, **blocking** — output ticks only where both are realized. -/
def crossSingleton (ts : TStream α) (v : TSing β) : TStream (α × β) :=
  List.zipWith (fun t b => t.map (·, b)) ts v

/-- Rust `filter_if` on a tick stream (gate each tick's batch by that tick's
boolean singleton); blocking. -/
def filterIf (ts : TStream α) (g : TSing Bool) : TStream α :=
  List.zipWith (fun t b => if b then t else []) ts g

@[simp] theorem map_length (ts : TStream α) (f : α → β) :
    (map ts f).length = ts.length := by simp [map]

@[simp] theorem allTicks_nil : allTicks ([] : TStream α) = [] := rfl

end TStream

namespace TSing

variable {α : Type u} {β : Type v} {γ : Type w}

/-- Tick-aligned pairing of two singletons (blocking). -/
def zip (a : TSing α) (b : TSing β) : TSing (α × β) := List.zip a b

/-- Rust `filter_if` on a singleton: an optional per tick, presented as a
tick stream (`[]`/`[v]` per tick); blocking. -/
def filterIf (v : TSing α) (g : TSing Bool) : TStream α :=
  List.zipWith (fun x b => if b then [x] else []) v g

/-- Rust `defer_tick` with an initial value: tick `t` sees tick `t-1`'s
value (`d` at tick 0). -/
def defer (v : TSing β) (d : β) : TSing β := d :: v

/-- Rust `.into_optional().defer_tick()`: tick `t` sees `some` of tick
`t-1`'s value, `none` at tick 0. -/
def deferOpt (v : TSing β) : TSing (Option β) := none :: v.map some

end TSing

/-! ## Batch and snapshot sites (the `nondet!` decisions, materialized)

Three regimes, by the input's ordering marker:

- **`TotalOrder`** (`batch`): the decision is a per-tick demand count; the
  tick consumes the next `d` elements (prefix cuts — order is real).
- **`NoOrder + ExactlyOnce`** (`batchC`): from the `NoOrder` point on *all*
  ordering information is gone — the collection is a multiset, and any
  `assume_ordering` downstream is a full shuffle. The decision is therefore
  the **consumed batch itself** (elements in adversary-chosen order); it is
  legal iff its multiset is contained in produced-so-far minus
  consumed-so-far. No shuffling is ever performed: downstream operators are
  order-tolerant by the Flo/Gyatso typing, and the adversarial order of the
  decision list IS the shuffle.
- **`NoOrder + AtLeastOnce`** (`batchD`): duplication is also free adversary
  power — legality is mere membership in produced-so-far.

A `NoOrder` *snapshot* is batch-plus-accumulate: the per-tick views are the
cumulative unions of the increments (arrivals accumulate; arrival order and
timing are the decision).

All three block instead of emitting a partial tick, and are monotone in the
input with decisions fixed (count- or membership-monotone for the `NoOrder`
forms) — realized ticks are final, the growth discipline `forward_ref`
iteration relies on. -/

/-- Rust `stream.batch(tick, nondet!(…))` on a **`TotalOrder`** stream:
demands `d` are the materialized decision; tick `t` consumes the next `d t`
elements (blocking). -/
def batch {α : Type u} (s : Stream α) : (d : List Nat) → TStream α
  | [] => []
  | d :: ds =>
    if d ≤ s.length then s.take d :: batch (s.drop d) ds else []

/-- Multiset-legality of one more consumed batch: with `consumed` already
drawn, drawing `b` as well stays within `avail`'s multiset. (Checking the
values of `b` suffices: other values' consumed counts are unchanged.) -/
def BatchLegal {α : Type u} [DecidableEq α] (avail consumed b : List α) :
    Bool :=
  b.all fun v => (consumed ++ b).count v ≤ avail.count v

/-- Rust `.batch`/`use::batch` on a **`NoOrder (+ ExactlyOnce)`** stream:
the decision is the consumed batch itself (adversary-ordered); a tick
realizes iff it is multiset-legal against availability minus prior
consumption. -/
def batchC {α : Type u} [DecidableEq α] (avail : List α)
    (consumed : List α) : (d : List (List α)) → TStream α
  | [] => []
  | b :: ds =>
    if BatchLegal avail consumed b then
      b :: batchC avail (consumed ++ b) ds
    else []

/-- Rust batch of a **`NoOrder + AtLeastOnce`** stream (e.g. heartbeat
fan-ins): re-delivery is free, so legality is membership. -/
def batchD {α : Type u} [DecidableEq α] (avail : List α) :
    (d : List (List α)) → TStream α
  | [] => []
  | b :: ds =>
    if b.all (· ∈ avail) then b :: batchD avail ds else []

/-- The `NoOrder` union of a member-indexed family (`merge_unordered` /
cluster fan-in): the multiset of all members' elements. Its list order is
never exposed — consumers go through `batchC`/`batchD` decisions. -/
def unionF {n : Nat} {α : Type u} (srcs : Fin n → Stream α) : List α :=
  (List.finRange n).flatMap srcs

/-- Rust `.sample_every(…, nondet!)` of a tick value: read the value at
adversarially chosen tick indices, blocking at the first unavailable index. -/
def sampleEvery {α : Type u} (v : TSing α) : (samples : List Nat) → Stream α
  | [] => []
  | t :: ts => if h : t < v.length then v[t] :: sampleEvery v ts else []

/-! ### Wiring facts (equational; no induction at use sites) -/

/-- `batch` never drops or fabricates: the consumed elements are a prefix of
the input. -/
theorem batch_flatten_prefix {α : Type u} (s : Stream α) (d : List Nat) :
    (batch s d).flatten <+: s := by
  induction d generalizing s with
  | nil => exact List.nil_prefix
  | cons c cs ih =>
    unfold batch
    by_cases h : c ≤ s.length
    · rw [if_pos h, List.flatten_cons]
      obtain ⟨u, hu⟩ := ih (s.drop c)
      exact ⟨u, by rw [List.append_assoc, hu, List.take_append_drop]⟩
    · rw [if_neg h]
      exact List.nil_prefix

/-- `batch` is prefix-monotone in the input (realized ticks are final). -/
theorem batch_prefix {α : Type u} {s s' : Stream α} (h : s <+: s')
    (d : List Nat) : batch s d <+: batch s' d := by
  induction d generalizing s s' with
  | nil => exact List.prefix_refl _
  | cons c cs ih =>
    unfold batch
    by_cases hc : c ≤ s.length
    · rw [if_pos hc, if_pos (Nat.le_trans hc h.length_le)]
      have htake : s.take c = s'.take c := by
        obtain ⟨u, rfl⟩ := h
        rw [List.take_append_of_le_length hc]
      have hdrop : s.drop c <+: s'.drop c := by
        obtain ⟨u, rfl⟩ := h
        rw [List.drop_append_of_le_length hc]
        exact ⟨u, rfl⟩
      rw [htake]
      exact List.cons_prefix_cons.mpr ⟨rfl, ih hdrop⟩
    · rw [if_neg hc]
      exact List.nil_prefix

/-- `batchC` consumption invariant: total consumption (prior + drawn) stays
within the availability multiset, value by value. -/
theorem batchC_flatten_count {α : Type u} [DecidableEq α]
    {avail consumed : List α} (d : List (List α))
    (hpre : ∀ v, consumed.count v ≤ avail.count v) :
    ∀ v, (consumed ++ (batchC avail consumed d).flatten).count v
      ≤ avail.count v := by
  induction d generalizing consumed with
  | nil => intro v; simpa [batchC] using hpre v
  | cons b ds ih =>
    unfold batchC
    by_cases hb : BatchLegal avail consumed b
    · rw [if_pos hb]
      intro v
      have := ih (consumed := consumed ++ b) (fun v => ?_) v
      · rw [List.flatten_cons, ← List.append_assoc]
        exact this
      · by_cases hv : v ∈ b
        · exact (decide_eq_true_iff).mp
            ((List.all_eq_true.mp hb) v hv)
        · rw [List.count_append,
            List.count_eq_zero_of_not_mem hv]
          simpa using hpre v
    · rw [if_neg hb]
      intro v
      simpa using hpre v

/-- `batchC` never fabricates: consumption is a sub-multiset of
availability. -/
theorem batchC_count_le {α : Type u} [DecidableEq α] (avail : List α)
    (d : List (List α)) (v : α) :
    ((batchC avail [] d).flatten).count v ≤ avail.count v := by
  have := batchC_flatten_count (avail := avail) (consumed := []) d
    (fun v => by simp) v
  simpa using this

/-- `batchC` membership provenance. -/
theorem batchC_mem {α : Type u} [DecidableEq α] {avail : List α}
    {d : List (List α)} {x : α}
    (h : x ∈ (batchC avail [] d).flatten) : x ∈ avail := by
  have hc := batchC_count_le avail d x
  have hpos : 0 < ((batchC avail [] d).flatten).count x :=
    List.count_pos_iff.mpr h
  exact List.count_pos_iff.mp (Nat.lt_of_lt_of_le hpos hc)

/-- `batchC` is monotone under availability-count growth (realized ticks
are final: the legality check only relaxes, and legal batches are copied
verbatim). -/
theorem batchC_le_count {α : Type u} [DecidableEq α]
    {avail avail' : List α}
    (h : ∀ v, avail.count v ≤ avail'.count v) (consumed : List α)
    (d : List (List α)) :
    batchC avail consumed d <+: batchC avail' consumed d := by
  induction d generalizing consumed with
  | nil => exact List.prefix_refl _
  | cons b ds ih =>
    unfold batchC
    by_cases hb : BatchLegal avail consumed b
    · have hb' : BatchLegal avail' consumed b := by
        rw [BatchLegal, List.all_eq_true] at hb ⊢
        intro v hv
        have := (decide_eq_true_iff).mp (hb v hv)
        exact decide_eq_true (Nat.le_trans this (h _))
      rw [if_pos hb, if_pos hb']
      exact List.cons_prefix_cons.mpr ⟨rfl, ih _⟩
    · rw [if_neg hb]
      exact List.nil_prefix

/-- A `batchC` decision **consumes** an availability when its flattened
batches are exactly the availability as a multiset (the complete-consumption
legality; quiescence face of a `NoOrder + ExactlyOnce` site). -/
def Consumes {α : Type u} (avail : List α) (d : List (List α)) : Prop :=
  d.flatten.Perm avail

/-- Complete decisions are legal: `batchC` realizes every decided batch
(consumption counts along any batch prefix are dominated by the total, which
is the availability count). -/
theorem Consumes.batchC_eq {α : Type u} [DecidableEq α] {avail : List α}
    {d : List (List α)} (h : Consumes avail d) :
    batchC avail [] d = d := by
  have key : ∀ (consumed : List α) (d' : List (List α)),
      (∀ v, (consumed ++ d'.flatten).count v ≤ avail.count v) →
      batchC avail consumed d' = d' := by
    intro consumed d'
    induction d' generalizing consumed with
    | nil => intro _; rfl
    | cons b ds ih =>
      intro hle
      unfold batchC
      have hcnt : ∀ v, (consumed ++ b).count v + ds.flatten.count v
          ≤ avail.count v := by
        intro v
        have := hle v
        rw [List.flatten_cons, ← List.append_assoc] at this
        rw [← List.count_append]
        exact this
      have hb : BatchLegal avail consumed b := by
        rw [BatchLegal, List.all_eq_true]
        intro v hv
        refine decide_eq_true ?_
        have := hcnt v
        omega
      rw [if_pos hb]
      exact congrArg (b :: ·) (ih (consumed ++ b) fun v => by
        rw [List.count_append]; exact hcnt v)
  refine key [] d fun v => ?_
  rw [List.nil_append, List.Perm.count_eq h v]
  exact Nat.le_refl _

/-- `batchD` membership provenance. -/
theorem batchD_mem {α : Type u} [DecidableEq α] {avail : List α}
    {d : List (List α)} {x : α}
    (h : x ∈ (batchD avail d).flatten) : x ∈ avail := by
  induction d with
  | nil => cases h
  | cons b ds ih =>
    unfold batchD at h
    by_cases hb : b.all (· ∈ avail)
    · rw [if_pos hb, List.flatten_cons] at h
      rcases List.mem_append.mp h with hx | hx
      · exact (decide_eq_true_iff).mp
          ((List.all_eq_true.mp hb) x hx)
      · exact ih hx
    · rw [if_neg hb] at h
      cases h

/-- `batchD` is monotone under availability-membership growth. -/
theorem batchD_subset {α : Type u} [DecidableEq α] {avail avail' : List α}
    (h : ∀ x ∈ avail, x ∈ avail') (d : List (List α)) :
    batchD avail d <+: batchD avail' d := by
  induction d with
  | nil => exact List.prefix_refl _
  | cons b ds ih =>
    unfold batchD
    by_cases hb : b.all (· ∈ avail)
    · have hb' : b.all (· ∈ avail') := by
        rw [List.all_eq_true] at hb ⊢
        intro v hv
        exact decide_eq_true (h v ((decide_eq_true_iff).mp
          (hb v hv)))
      rw [if_pos hb, if_pos hb']
      exact List.cons_prefix_cons.mpr ⟨rfl, ih⟩
    · rw [if_neg hb]
      exact List.nil_prefix

/-- `unionF` counts are the member-indexed sums (the cluster-as-map counting
exchange). -/
theorem unionF_count {n : Nat} {α : Type u} [DecidableEq α]
    (srcs : Fin n → Stream α) (v : α) :
    (unionF srcs).count v
      = ((List.finRange n).map (fun j => (srcs j).count v)).sum := by
  unfold unionF
  induction (List.finRange n) with
  | nil => rfl
  | cons j js ih =>
    rw [List.flatMap_cons, List.map_cons, List.sum_cons, List.count_append,
      ih]

/-- Union membership decomposes to a member. -/
theorem unionF_mem {n : Nat} {α : Type u} {srcs : Fin n → Stream α} {x : α}
    (h : x ∈ unionF srcs) : ∃ j, x ∈ srcs j := by
  obtain ⟨j, -, hj⟩ := List.mem_flatMap.mp h
  exact ⟨j, hj⟩

/-- `unionF` counts grow under member-wise prefix growth. -/
theorem unionF_count_mono {n : Nat} {α : Type u} [DecidableEq α]
    {srcs srcs' : Fin n → Stream α} (h : ∀ j, srcs j <+: srcs' j) (v : α) :
    (unionF srcs).count v ≤ (unionF srcs').count v := by
  rw [unionF_count, unionF_count]
  exact sum_map_le_sum_map _ _ _ (fun j _ => count_le_of_prefix (h j) v)

/-- **NoOrder singleton snapshot** (fold trajectory): the decision is the
per-tick *increment* of observed elements (adversary-ordered — the faithful
model of `assume_ordering` and of the many trajectories a fold over a
`NoOrder` stream admits); the view at each tick is the accumulated
sub-multiset, on which the caller applies its (commutative) fold. Legality
is multiset containment; membership-legal variant `snapshotD` for
`AtLeastOnce` sources (re-delivery free — for idempotent folds such as
`max`, duplication is absorbed). Protocol proofs consume the realized-view
lemmas below ("which values can appear in a snapshot"); illegal decisions
block the tick and need never be reasoned about. -/
def snapshotC {α : Type u} [DecidableEq α] (avail : List α)
    (acc : List α) : (d : List (List α)) → TSing (List α)
  | [] => []
  | b :: ds =>
    if BatchLegal avail acc b then
      (acc ++ b) :: snapshotC avail (acc ++ b) ds
    else []

/-- `AtLeastOnce` variant: increments legal by membership. -/
def snapshotD {α : Type u} [DecidableEq α] (avail : List α)
    (acc : List α) : (d : List (List α)) → TSing (List α)
  | [] => []
  | b :: ds =>
    if b.all (· ∈ avail) then (acc ++ b) :: snapshotD avail (acc ++ b) ds
    else []

/-- Realized `snapshotC` views draw from availability (count form). -/
theorem snapshotC_view_count {α : Type u} [DecidableEq α]
    {avail acc : List α} {d : List (List α)} {v : List α}
    (hpre : ∀ x, acc.count x ≤ avail.count x)
    (h : v ∈ snapshotC avail acc d) : ∀ x, v.count x ≤ avail.count x := by
  induction d generalizing acc with
  | nil => cases h
  | cons b ds ih =>
    unfold snapshotC at h
    by_cases hb : BatchLegal avail acc b
    · rw [if_pos hb] at h
      have hstep : ∀ x, (acc ++ b).count x ≤ avail.count x := by
        intro x
        by_cases hx : x ∈ b
        · exact (decide_eq_true_iff).mp
            ((List.all_eq_true.mp hb) x hx)
        · rw [List.count_append, List.count_eq_zero_of_not_mem hx]
          simpa using hpre x
      rcases List.mem_cons.mp h with rfl | h'
      · exact hstep
      · exact ih hstep h'
    · rw [if_neg hb] at h
      cases h

/-- Realized `snapshotC` views draw from availability (membership form). -/
theorem snapshotC_view_mem {α : Type u} [DecidableEq α]
    {avail : List α} {d : List (List α)} {v : List α}
    (h : v ∈ snapshotC avail [] d) : ∀ x ∈ v, x ∈ avail := by
  intro x hx
  have := snapshotC_view_count (avail := avail) (acc := [])
    (fun _ => by simp) h x
  exact List.count_pos_iff.mp
    (Nat.lt_of_lt_of_le (List.count_pos_iff.mpr hx) this)

/-- Realized `snapshotD` views draw from availability. -/
theorem snapshotD_view_mem {α : Type u} [DecidableEq α]
    {avail acc : List α} {d : List (List α)} {v : List α}
    (hpre : ∀ x ∈ acc, x ∈ avail)
    (h : v ∈ snapshotD avail acc d) : ∀ x ∈ v, x ∈ avail := by
  induction d generalizing acc with
  | nil => cases h
  | cons b ds ih =>
    unfold snapshotD at h
    by_cases hb : b.all (· ∈ avail)
    · rw [if_pos hb] at h
      have hstep : ∀ x ∈ acc ++ b, x ∈ avail := by
        intro x hx
        rcases List.mem_append.mp hx with hx | hx
        · exact hpre x hx
        · exact (decide_eq_true_iff).mp
            ((List.all_eq_true.mp hb) x hx)
      rcases List.mem_cons.mp h with rfl | h'
      · exact hstep
      · exact ih hstep h'
    · rw [if_neg hb] at h
      cases h

/-- `snapshotC` is monotone under availability-count growth. -/
theorem snapshotC_le_count {α : Type u} [DecidableEq α]
    {avail avail' : List α}
    (h : ∀ v, avail.count v ≤ avail'.count v) (acc : List α)
    (d : List (List α)) :
    snapshotC avail acc d <+: snapshotC avail' acc d := by
  induction d generalizing acc with
  | nil => exact List.prefix_refl _
  | cons b ds ih =>
    unfold snapshotC
    by_cases hb : BatchLegal avail acc b
    · have hb' : BatchLegal avail' acc b := by
        rw [BatchLegal, List.all_eq_true] at hb ⊢
        intro v hv
        have := (decide_eq_true_iff).mp (hb v hv)
        exact decide_eq_true (Nat.le_trans this (h _))
      rw [if_pos hb, if_pos hb']
      exact List.cons_prefix_cons.mpr ⟨rfl, ih _⟩
    · rw [if_neg hb]
      exact List.nil_prefix

/-- `snapshotD` is monotone under availability-membership growth. -/
theorem snapshotD_subset {α : Type u} [DecidableEq α]
    {avail avail' : List α} (h : ∀ x ∈ avail, x ∈ avail') (acc : List α)
    (d : List (List α)) :
    snapshotD avail acc d <+: snapshotD avail' acc d := by
  induction d generalizing acc with
  | nil => exact List.prefix_refl _
  | cons b ds ih =>
    unfold snapshotD
    by_cases hb : b.all (· ∈ avail)
    · have hb' : b.all (· ∈ avail') := by
        rw [List.all_eq_true] at hb ⊢
        intro v hv
        exact decide_eq_true (h v ((decide_eq_true_iff).mp
          (hb v hv)))
      rw [if_pos hb, if_pos hb']
      exact List.cons_prefix_cons.mpr ⟨rfl, ih _⟩
    · rw [if_neg hb]
      exact List.nil_prefix

/-- Prefixes dominate counts (`DecidableEq` form, matching the `NoOrder`
combinators' instances). -/
theorem count_le_of_prefix_d {α : Type u} [DecidableEq α] {l l' : List α}
    (h : l <+: l') (v : α) : l.count v ≤ l'.count v := by
  obtain ⟨u, rfl⟩ := h
  rw [List.count_append]
  omega

/-- Counting under multiset domination: if every value's count is dominated,
so is every `countP`. -/
theorem countP_le_of_count_le {α : Type u} [DecidableEq α]
    {l l' : List α} (h : ∀ v, l.count v ≤ l'.count v) (p : α → Bool) :
    l.countP p ≤ l'.countP p := by
  induction l generalizing l' with
  | nil => exact Nat.zero_le _
  | cons x t ih =>
    have hcx : 0 < l'.count x := by
      refine Nat.lt_of_lt_of_le ?_ (h x)
      simp [List.count_cons_self]
    have hx : x ∈ l' := List.count_pos_iff.mp hcx
    have hperm : l'.Perm (x :: l'.erase x) := List.perm_cons_erase hx
    have ht : ∀ v, t.count v ≤ (l'.erase x).count v := by
      intro v
      have hv := h v
      rw [List.count_cons] at hv
      rw [List.count_erase]
      by_cases hvx : v = x
      · subst hvx
        simp only [beq_self_eq_true, if_pos] at hv ⊢
        omega
      · have hb1 : (v == x) = false := beq_eq_false_iff_ne.mpr hvx
        have hb2 : (x == v) = false := beq_eq_false_iff_ne.mpr (Ne.symm hvx)
        simp only [hb1, hb2, if_false, Bool.false_eq_true] at hv ⊢
        omega
    calc (x :: t).countP p = t.countP p + (if p x then 1 else 0) := by
          rw [List.countP_cons]
      _ ≤ (l'.erase x).countP p + (if p x then 1 else 0) :=
          Nat.add_le_add_right (ih ht) _
      _ = (x :: l'.erase x).countP p := by rw [List.countP_cons]
      _ = l'.countP p := (hperm.countP_eq p).symm

/-- `sampleEvery` is prefix-monotone in the sampled value. -/
theorem sampleEvery_prefix {α : Type u} {v v' : TSing α} (h : v <+: v')
    (samples : List Nat) : sampleEvery v samples <+: sampleEvery v' samples := by
  induction samples with
  | nil => exact List.prefix_refl _
  | cons t ts ih =>
    unfold sampleEvery
    by_cases ht : t < v.length
    · have ht' : t < v'.length := Nat.lt_of_lt_of_le ht h.length_le
      rw [dif_pos ht, dif_pos ht']
      exact List.cons_prefix_cons.mpr ⟨List.IsPrefix.getElem h ht, ih⟩
    · rw [dif_neg ht]
      exact List.nil_prefix

/-- Sampled values come from the tick value stream. -/
theorem mem_sampleEvery {α : Type u} {v : TSing α} {samples : List Nat}
    {x : α} (h : x ∈ sampleEvery v samples) : x ∈ v := by
  induction samples with
  | nil => cases h
  | cons t ts ih =>
    unfold sampleEvery at h
    by_cases ht : t < v.length
    · rw [dif_pos ht] at h
      rcases List.mem_cons.mp h with rfl | h'
      · exact List.getElem_mem ht
      · exact ih h'
    · rw [dif_neg ht] at h
      cases h

/-! ### Generic prefix-monotonicity toolkit (composition closes) -/

theorem prefix_map {α β : Type _} {l l' : List α} (h : l <+: l')
    (f : α → β) : l.map f <+: l'.map f := h.map f

theorem prefix_filterMap {α β : Type _} {l l' : List α} (h : l <+: l')
    (f : α → Option β) : l.filterMap f <+: l'.filterMap f := h.filterMap f

theorem prefix_flatten {α : Type _} {l l' : List (List α)} (h : l <+: l') :
    l.flatten <+: l'.flatten := by
  obtain ⟨u, rfl⟩ := h
  rw [List.flatten_append]
  exact List.prefix_append _ _

theorem prefix_zipWith {α β γ : Type _} {a a' : List α} {b b' : List β}
    (ha : a <+: a') (hb : b <+: b') (f : α → β → γ) :
    List.zipWith f a b <+: List.zipWith f a' b' := by
  induction a generalizing a' b b' with
  | nil => exact List.nil_prefix
  | cons x xs ih =>
    obtain ⟨u, rfl⟩ := ha
    cases b with
    | nil => exact List.nil_prefix
    | cons y ys =>
      obtain ⟨v, rfl⟩ := hb
      simp only [List.cons_append, List.zipWith_cons_cons]
      exact List.cons_prefix_cons.mpr
        ⟨rfl, ih (List.prefix_append _ _) (List.prefix_append _ _)⟩

/-! ### Tick loops over located streams

Lifting an established `TickLoop` (a `sliced!` body) to the tick-located
surface: outputs per tick, and the per-tick post-states (the loop's in-tick
singleton across the execution). -/

namespace TickLoop

variable {In : Type u} {St : Type v} {Out : Type w}

/-- Per-tick post-states (the published in-tick singleton, across ticks). -/
def states (t : TickLoop In St Out) (bs : List In) : List St :=
  scanSt (fun s b => (t.step s b).1) t.init bs

@[simp] theorem states_length (t : TickLoop In St Out) (bs : List In) :
    (t.states bs).length = bs.length := scanSt_length _ _ _

/-- The state visible at tick `n` is the final state of the consumed
prefix. -/
theorem states_getElem (t : TickLoop In St Out) (bs : List In) (n : Nat)
    (hn : n < (t.states bs).length) :
    (t.states bs)[n] = t.finalState (bs.take (n + 1)) := by
  have hn' : n < (scanSt (fun s b => (t.step s b).1) t.init bs).length := hn
  have h := scanSt_getElem (fun s b => (t.step s b).1) t.init bs n hn'
  rw [foldl_eq_finalState] at h
  exact h

theorem states_prefix (t : TickLoop In St Out) {bs bs' : List In}
    (h : bs <+: bs') : t.states bs <+: t.states bs' :=
  scanSt_prefix _ _ h

/-- Outputs are prefix-monotone in the consumed batches. -/
theorem outputs_prefix (t : TickLoop In St Out) {bs bs' : List In}
    (h : bs <+: bs') : t.outputs bs <+: t.outputs bs' := by
  rw [← scan_eq_outputs, ← scan_eq_outputs]
  exact scan_prefix _ _ h

/-- The output batch at tick `n` is the step at the prefix state (model
equation). -/
theorem outputs_getElem (t : TickLoop In St Out) (bs : List In) (n : Nat)
    (hn : n < (t.outputs bs).length) (hb : n < bs.length) :
    (t.outputs bs)[n]
      = (t.step (t.finalState (bs.take n)) (bs[n]'hb)).2 := by
  have hn' : n < (scan t.step t.init bs).length := by
    rw [scan_eq_outputs]
    exact hn
  have h := scan_getElem t.step t.init bs n hn' hb
  rw [foldl_eq_finalState] at h
  calc (t.outputs bs)[n]
      = (scan t.step t.init bs)[n]'hn' := by
        congr 1
        exact (scan_eq_outputs t bs).symm
    _ = _ := h

@[simp] theorem outputs_length (t : TickLoop In St Out) (bs : List In) :
    (t.outputs bs).length = bs.length := by
  rw [← scan_eq_outputs, scan_length]

/-- **Provenance eliminator**: every released element was produced by some
tick's step, from the state of the strictly-earlier consumed prefix. -/
theorem mem_outputs_elim {β : Type _} (t : TickLoop In St (List β))
    {bs : List In} {x : β} (hx : x ∈ (t.outputs bs).flatten) :
    ∃ n, ∃ hb : n < bs.length,
      x ∈ (t.step (t.finalState (bs.take n)) (bs[n]'hb)).2 := by
  obtain ⟨l, hl, hxl⟩ := List.mem_flatten.mp hx
  obtain ⟨n, hn, rfl⟩ := List.mem_iff_getElem.mp hl
  have hb : n < bs.length := by
    have := hn
    rw [outputs_length] at this
    exact this
  exact ⟨n, hb, by rw [← outputs_getElem t bs n hn hb]; exact hxl⟩

end TickLoop

/-- **Provenance eliminator for raw scans**: every element of the flattened
scan output was produced by some tick's step, from the fold state of the
strictly-earlier consumed prefix. -/
theorem mem_scan_flatten_elim {ι : Type u} {σ : Type v} {β : Type w}
    {g : σ → ι → σ × List β} {init : σ} {ins : List ι} {x : β}
    (hx : x ∈ (scan g init ins).flatten) :
    ∃ t, ∃ ht : t < ins.length,
      x ∈ (g ((ins.take t).foldl (fun s b => (g s b).1) init)
        (ins[t]'ht)).2 := by
  obtain ⟨l, hl, hxl⟩ := List.mem_flatten.mp hx
  obtain ⟨t, hn, rfl⟩ := List.mem_iff_getElem.mp hl
  have hb : t < ins.length := by
    have := hn
    rwa [scan_length] at this
  exact ⟨t, hb, by rw [← scan_getElem g init ins t hn hb]; exact hxl⟩

/-- Realized `snapshotC` views extend the running accumulator. -/
theorem snapshotC_acc_prefix {α : Type u} [DecidableEq α]
    {avail : List α} {d : List (List α)} :
    ∀ {acc v : List α}, v ∈ snapshotC avail acc d → acc <+: v := by
  induction d with
  | nil => intro acc v h; cases h
  | cons b ds ih =>
    intro acc v h
    unfold snapshotC at h
    by_cases hb : BatchLegal avail acc b
    · rw [if_pos hb] at h
      rcases List.mem_cons.mp h with rfl | h'
      · exact List.prefix_append _ _
      · exact (List.prefix_append acc b).trans (ih h')
    · rw [if_neg hb] at h
      cases h

/-- **Snapshot views chain by prefix along ticks**: the cut at a later tick
extends the cut at an earlier tick (accumulated increments). -/
theorem snapshotC_getElem_prefix {α : Type u} [DecidableEq α]
    {avail : List α} {d : List (List α)} :
    ∀ {acc : List α} {t t' : Nat} (h : t ≤ t')
      (ht' : t' < (snapshotC avail acc d).length),
      (snapshotC avail acc d)[t]'(Nat.lt_of_le_of_lt h ht')
        <+: (snapshotC avail acc d)[t']'ht' := by
  induction d with
  | nil =>
    intro acc t t' h ht'
    simp [snapshotC] at ht'
  | cons b ds ih =>
    intro acc t t' h ht'
    by_cases hb : BatchLegal avail acc b
    · have hview : snapshotC avail acc (b :: ds)
          = (acc ++ b) :: snapshotC avail (acc ++ b) ds := by
        show (if BatchLegal avail acc b then
            (acc ++ b) :: snapshotC avail (acc ++ b) ds else [])
          = (acc ++ b) :: snapshotC avail (acc ++ b) ds
        rw [if_pos hb]
      have hlen : t' < ((acc ++ b) :: snapshotC avail (acc ++ b) ds).length := by
        rw [← hview]
        exact ht'
      rw [List.getElem_of_eq hview, List.getElem_of_eq hview]
      cases t with
      | zero =>
        cases t' with
        | zero => exact List.prefix_refl _
        | succ s' =>
          show acc ++ b <+: (snapshotC avail (acc ++ b) ds)[s']'_
          exact snapshotC_acc_prefix (List.getElem_mem _)
      | succ s =>
        cases t' with
        | zero => omega
        | succ s' =>
          exact ih (Nat.le_of_succ_le_succ h) _
    · exfalso
      unfold snapshotC at ht'
      rw [if_neg hb] at ht'
      simp at ht'

/-- Opening a mapped zip4 at an index (the shape of a four-input blocking
stage's tick input). -/
theorem zip4_map_getElem {α β γ δ : Type _} (a : List α) (b : List β)
    (c : List γ) (d : List δ) {t : Nat}
    (ht : t < ((List.zip a (List.zip b (List.zip c d))).map
      (fun x => (x.1, x.2.1, x.2.2.1, x.2.2.2))).length) :
    ∃ (ha : t < a.length) (hb : t < b.length) (hc : t < c.length)
      (hd : t < d.length),
      ((List.zip a (List.zip b (List.zip c d))).map
        (fun x => (x.1, x.2.1, x.2.2.1, x.2.2.2)))[t]'ht
        = (a[t]'ha, b[t]'hb, c[t]'hc, d[t]'hd) := by
  have hzip : t < (List.zip a (List.zip b (List.zip c d))).length := by
    have := ht
    rwa [List.length_map] at this
  have hlens := hzip
  rw [List.length_zip, List.length_zip, List.length_zip] at hlens
  have ha : t < a.length := by omega
  have hb : t < b.length := by omega
  have hc : t < c.length := by omega
  have hd : t < d.length := by omega
  refine ⟨ha, hb, hc, hd, ?_⟩
  rw [List.getElem_map, List.getElem_zip]
  dsimp only
  rw [List.getElem_zip]
  dsimp only
  rw [List.getElem_zip]

/-- Elements of realized prefixes agree (take-equality). -/
theorem prefix_take_eq {α : Type _} {l l' : List α} (h : l <+: l')
    (n : Nat) (hn : n ≤ l.length) : l.take n = l'.take n := by
  obtain ⟨u, rfl⟩ := h
  rw [List.take_append_of_le_length hn]

/-- Take-equality transfers elements. -/
theorem getElem_eq_of_take_eq {α : Type _} {l l' : List α} {n t : Nat}
    (h : l.take n = l'.take n) (htn : t < n) (ht : t < l.length)
    (ht' : t < l'.length) : l[t] = l'[t] := by
  have h1 : (l.take n)[t]'(by rw [List.length_take]; omega) = l[t] :=
    List.getElem_take
  have h2 : (l'.take n)[t]'(by rw [List.length_take]; omega) = l'[t] :=
    List.getElem_take
  rw [← h1, ← h2]
  congr 1

/-- Split a `take (t+1)` as `take t ++ [l[t]]`. -/
theorem take_succ_eq {α : Type _} (l : List α) (t : Nat) (ht : t < l.length) :
    l.take (t + 1) = l.take t ++ [l[t]] := by
  rw [List.take_succ, List.getElem?_eq_getElem ht]
  rfl

/-- A gated singleton release (`filter_if` + `all_ticks`) is a sublist of
the tick values. -/
theorem TSing.filterIf_flatten_sublist {α : Type u} (v : TSing α)
    (g : TSing Bool) :
    (TStream.allTicks (TSing.filterIf v g)).Sublist v := by
  unfold TStream.allTicks TSing.filterIf
  induction v generalizing g with
  | nil => simp
  | cons x xs ih =>
    cases g with
    | nil => simp
    | cons b bs =>
      rw [List.zipWith_cons_cons, List.flatten_cons]
      by_cases hb : b
      · subst hb
        simpa using (ih bs).cons₂ x
      · have : b = false := Bool.eq_false_iff.mpr hb
        subst this
        simpa using (ih bs).cons x

/-- A gated singleton release, eliminated at its tick: every released value
is the singleton's value at a gate-true tick. -/
theorem TSing.filterIf_flatten_elim {α : Type u} {v : TSing α}
    {g : TSing Bool} {x : α}
    (h : x ∈ TStream.allTicks (TSing.filterIf v g)) :
    ∃ (u : Nat) (hu : u < v.length) (hg : u < g.length),
      v[u]'hu = x ∧ g[u]'hg = true := by
  unfold TStream.allTicks TSing.filterIf at h
  induction v generalizing g with
  | nil => simp at h
  | cons y ys ih =>
    cases g with
    | nil => simp at h
    | cons b bs =>
      rw [List.zipWith_cons_cons, List.flatten_cons] at h
      rcases List.mem_append.mp h with hhead | htail
      · cases b with
        | true =>
          rcases List.mem_singleton.mp (by simpa using hhead) with rfl
          exact ⟨0, Nat.succ_pos _, Nat.succ_pos _, rfl, rfl⟩
        | false => simp at hhead
      · obtain ⟨u, hu, hg, hv, hgt⟩ := ih htail
        exact ⟨u + 1, Nat.succ_lt_succ hu, Nat.succ_lt_succ hg, hv, hgt⟩

end HydroLean.Hydro
