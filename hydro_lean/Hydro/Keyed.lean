import Hydro.Trace
import Mathlib.Data.Finset.Max
import Mathlib.Data.Fintype.Basic
import Mathlib.Algebra.Order.BigOperators.Group.Finset

/-!
# Hydro · the keyed carriers (the grades' keyed chapter)

Rust's keyed live collections, at the denotation:

- `KeyedPool K V ord ret` — `KeyedStream<K, V, …, ord, ret>` content: a
  **finite map from keys to graded per-key content**. Keys are
  unordered (a `Finset`); a key's content is its own grade quotient
  (`PoolCarrier V ord ret`): `TotalOrder` is *per-key* order, with no
  cross-key order anywhere in the carrier — exactly Rust's promise.
  Rust's `into_keyed` (a `Cast` at the runtime) is grouping here; the
  network's `KeyedStream<MemberId, T>` is the per-sender map
  (`transport`).
- `KeyedMap K V` — `KeyedSingleton<K, V, Tick, Bounded>` / a keyed
  fold's result: a finite map from keys to **one value each**; insertion
  order is never observable (every downstream op emits `NoOrder` or
  demands a commutative algebra), so the carrier has none.

Reads of a keyed-sourced singleton (`fold_early_stop` →
`get_max_key` → `snapshot`) take **increment cuts of pairs**
(`CutDec (K × V) .noOrder`): per tick, the multiset of pairs consumed,
legal iff per key it is the next prefix of that key's content
(`TotalOrder`) or fits the key's multiset (`NoOrder`). Illegal cuts
block (legality is realizability), as every cut vocabulary does.

The step machine keeps the runtime's flat `List (K × V)` for every
keyed carrier (`into_keyed` is the identity there — Rust's cast); the
coupling (`CoupledKeyed`, `CoupledKMap`) relates the flat list to the
map per key.
-/

namespace Hydro

/-! ## `KeyedPool` — a keyed stream's content -/

/-- A keyed stream's content: a finite key set with graded per-key
content; keys outside the set carry the grade's bottom. -/
structure KeyedPool (K V : Type) [DecidableEq K] [DecidableEq V]
    (ord : StrOrd) (ret : Retries) where
  keys : Finset K
  grp : K → PoolCarrier V ord ret
  dom : ∀ k, k ∉ keys → grp k = PoolBot ord ret

namespace KeyedPool
variable {K V : Type} [DecidableEq K] [DecidableEq V] {ord : StrOrd}
  {ret : Retries}

theorem ext {a b : KeyedPool K V ord ret} (hk : a.keys = b.keys)
    (hg : a.grp = b.grp) : a = b := by
  cases a; cases b
  cases hk; cases hg
  rfl

/-- The empty keyed content. -/
def bot : KeyedPool K V ord ret := ⟨∅, fun _ => PoolBot ord ret, fun _ _ => rfl⟩

/-- Rust `into_keyed()` on ordered pairs: group by key, each key's
values in their order. -/
def ofList (l : List (K × V)) : KeyedPool K V .totalOrder .exactlyOnce :=
  ⟨(l.map Prod.fst).toFinset, fun k => keyVals k l, fun k hk => by
    rw [List.mem_toFinset] at hk
    unfold keyVals
    rw [List.filter_eq_nil_iff.mpr (fun e he hke => hk ?_)]
    · rfl
    · exact List.mem_map.mpr ⟨e, he, of_decide_eq_true hke⟩⟩

/-- Rust `into_keyed()` on unordered pairs: the per-key multisets. -/
def ofMultiset (m : Multiset (K × V)) : KeyedPool K V .noOrder .exactlyOnce :=
  ⟨(m.map Prod.fst).toFinset, fun k => (m.filter (fun e => e.1 = k)).map Prod.snd,
   fun k hk => by
    rw [Multiset.mem_toFinset] at hk
    show Multiset.map Prod.snd (Multiset.filter (fun e => e.1 = k) m) = 0
    rw [Multiset.filter_eq_nil.mpr (fun e he hke => hk (Multiset.mem_map.mpr ⟨e, he, hke⟩))]
    rfl⟩

/-- Rust `into_keyed()` at the exactly-once grades. -/
def ofPool : {ord : StrOrd} → PoolCarrier (K × V) ord .exactlyOnce →
    KeyedPool K V ord .exactlyOnce
  | .totalOrder => ofList
  | .noOrder => ofMultiset

/-- The machine's flat batch of pairs, grouped at a grade. -/
def ofBatch : {ord : StrOrd} → List (K × V) → KeyedPool K V ord .exactlyOnce
  | .totalOrder => ofList
  | .noOrder => fun l => ofMultiset (↑l)

/-- The network's keyed stream: one group per sender member. -/
def transport {c : Nat} (s : Fin c → PoolCarrier V ord ret) :
    KeyedPool (Fin c) V ord ret :=
  ⟨Finset.univ, s, fun k hk => absurd (Finset.mem_univ k) hk⟩

/-- A key's content as a multiset (the grade forgotten into `NoOrder`). -/
def grpMS : {ord : StrOrd} → {ret : Retries} → PoolCarrier V ord ret → Multiset V
  | .totalOrder, .exactlyOnce => fun l => (↑l : Multiset V)
  | .noOrder, .exactlyOnce => fun m => m
  | .totalOrder, .atLeastOnce => fun s => (↑s.norm : Multiset V)
  | .noOrder, .atLeastOnce => fun p => (RetryPool.support p).val

/-- Every key's content, merged into one multiset. -/
def sumMS (p : KeyedPool K V ord ret) : Multiset V :=
  (p.keys.val.map (fun k => grpMS (p.grp k))).sum

/-- Rust `.values()`: every key's content, merged — the interleaving
rides the `NoOrder` type (at the retry grade, as an entitlement). -/
def values : {ord : StrOrd} → {ret : Retries} → KeyedPool K V ord ret →
    PoolCarrier V .noOrder ret
  | _, .exactlyOnce => fun p => p.sumMS
  | _, .atLeastOnce => fun p => RetryPool.mk p.sumMS

/-- The content as a flat multiset of pairs (exactly-once grades). -/
def toMultiset (p : KeyedPool K V ord .exactlyOnce) : Multiset (K × V) :=
  (p.keys.val.map (fun k => (grpMS (p.grp k)).map (fun v => (k, v)))).sum

/-- Per-key growth: the keyed stream grows when every key's content
grows in its grade. -/
def Le (a b : KeyedPool K V ord ret) : Prop :=
  ∀ k, PoolLe ord ret (a.grp k) (b.grp k)

theorem Le.refl (a : KeyedPool K V ord ret) : Le a a :=
  fun k => PoolLe.refl ord ret (a.grp k)

theorem Le.trans {a b c : KeyedPool K V ord ret} (h₁ : Le a b) (h₂ : Le b c) :
    Le a c := fun k => PoolLe.trans (h₁ k) (h₂ k)

/-! ### Growth lemmas (the Flo-monotonicity faces of the keyed ops) -/

theorem grpMS_bot : grpMS (V := V) (PoolBot ord ret) = 0 := by
  cases ord <;> cases ret <;> rfl

/-- A key's forgotten content grows with the key's content. -/
theorem grpMS_le {a b : PoolCarrier V ord ret} (h : PoolLe ord ret a b) :
    grpMS a ≤ grpMS b := by
  cases ord <;> cases ret
  · exact Multiset.coe_le.mpr h.sublist.subperm
  · exact Multiset.coe_le.mpr (List.IsPrefix.sublist h).subperm
  · exact h
  · exact (Multiset.le_iff_subset (Finset.nodup _)).mpr
      (fun x hx => RetryPool.mem_support_val.mpr
        (h x (RetryPool.mem_support_val.mp hx)))

/-- Summing a per-key multiset over the keys: keys outside the key set
contribute nothing, so the sum may be taken over any superset. -/
theorem sum_keys_eq {β : Type} [AddCommMonoid β] (p : KeyedPool K V ord ret)
    (f : K → PoolCarrier V ord ret → β) (hz : ∀ k, f k (PoolBot ord ret) = 0)
    {t : Finset K} (ht : p.keys ⊆ t) :
    (p.keys.val.map (fun k => f k (p.grp k))).sum
      = (t.val.map (fun k => f k (p.grp k))).sum := by
  rw [← Finset.sum_eq_multiset_sum, ← Finset.sum_eq_multiset_sum]
  exact Finset.sum_subset ht (fun k _ hk => by rw [p.dom k hk, hz])

theorem sumMS_le {a b : KeyedPool K V ord ret} (h : Le a b) : a.sumMS ≤ b.sumMS := by
  unfold sumMS
  rw [sum_keys_eq a (fun _ g => grpMS g) (fun _ => grpMS_bot) (Finset.subset_union_left (s₂ := b.keys)),
    sum_keys_eq b (fun _ g => grpMS g) (fun _ => grpMS_bot) (Finset.subset_union_right (s₁ := a.keys))]
  exact Multiset.sum_map_le_sum_map _ _ (fun k _ => grpMS_le (h k))

theorem toMultiset_le {a b : KeyedPool K V ord .exactlyOnce} (h : Le a b) :
    a.toMultiset ≤ b.toMultiset := by
  unfold toMultiset
  rw [sum_keys_eq a (fun k g => (grpMS g).map (fun v => (k, v)))
      (fun k => by rw [grpMS_bot]; rfl) (Finset.subset_union_left (s₂ := b.keys)),
    sum_keys_eq b (fun k g => (grpMS g).map (fun v => (k, v)))
      (fun k => by rw [grpMS_bot]; rfl) (Finset.subset_union_right (s₁ := a.keys))]
  exact Multiset.sum_map_le_sum_map _ _ (fun k _ => Multiset.map_le_map (grpMS_le (h k)))

theorem values_le {a b : KeyedPool K V ord ret} (h : Le a b) :
    PoolLe .noOrder ret a.values b.values := by
  cases ret
  · exact sumMS_le h
  · exact fun x hx => Multiset.mem_of_le (sumMS_le h) hx

theorem ofList_prefix {l l' : List (K × V)} (h : l <+: l') :
    Le (ofList l) (ofList l') := fun k => keyVals_prefix h

theorem ofMultiset_le {m m' : Multiset (K × V)} (h : m ≤ m') :
    Le (ofMultiset m) (ofMultiset m') := fun k =>
  Multiset.map_le_map (Multiset.filter_le_filter _ h)

theorem ofPool_le {ord : StrOrd} {a b : PoolCarrier (K × V) ord .exactlyOnce}
    (h : PoolLe ord .exactlyOnce a b) : Le (ofPool a) (ofPool b) := by
  cases ord
  · exact ofList_prefix h
  · exact ofMultiset_le h

theorem transport_le {c : Nat} {s s' : Fin c → PoolCarrier V ord ret}
    (h : ∀ j, PoolLe ord ret (s j) (s' j)) : Le (transport s) (transport s') := h

end KeyedPool

/-! ## `KeyedMap` — a keyed singleton's content -/

/-- A keyed singleton: a finite key set with exactly one value per
key in it (and none outside). -/
structure KeyedMap (K V : Type) [DecidableEq K] where
  keys : Finset K
  get : K → Option V
  dom : ∀ k, (get k).isSome ↔ k ∈ keys

namespace KeyedMap
variable {K V W : Type} [DecidableEq K]

theorem ext {a b : KeyedMap K V} (hk : a.keys = b.keys) (hg : a.get = b.get) :
    a = b := by
  cases a; cases b
  cases hk; cases hg
  rfl

/-- The empty map. -/
def empty : KeyedMap K V := ⟨∅, fun _ => none, fun _ => by simp⟩

/-- The entries `(k, v)`, as an unordered stream (Rust `.entries()`). -/
def entries (m : KeyedMap K V) : Multiset (K × V) :=
  m.keys.val.filterMap (fun k => (m.get k).map (fun v => (k, v)))

/-- The keys (Rust `.keys()`). -/
def keysMS (m : KeyedMap K V) : Multiset K := m.keys.val

/-- Rust `KeyedSingleton::filter(|v| …)`: keep the keys whose value
passes. -/
def filter (m : KeyedMap K V) (p : V → Bool) : KeyedMap K V :=
  ⟨m.keys.filter (fun k => (m.get k).any p), fun k => (m.get k).filter p,
   fun k => by
    rw [Finset.mem_filter, ← m.dom k]
    cases m.get k <;> simp⟩

/-- Rust `KeyedSingleton::map(|v| …)`. -/
def map (m : KeyedMap K V) (f : V → W) : KeyedMap K W :=
  ⟨m.keys, fun k => (m.get k).map f, fun k => by rw [← m.dom k]; cases m.get k <;> simp⟩

/-- Rust `KeyedSingleton::join_keyed_singleton(other)`: the keys both
maps hold, with the pair of values. -/
def join (a : KeyedMap K V) (b : KeyedMap K W) : KeyedMap K (V × W) :=
  ⟨a.keys ∩ b.keys, fun k => (a.get k).bind (fun v => (b.get k).map (fun w => (v, w))),
   fun k => by
    rw [Finset.mem_inter, ← a.dom k, ← b.dom k]
    cases a.get k <;> cases b.get k <;> simp⟩

/-- Rust `KeyedSingleton::filter_key_not_in(keys)`: drop the keys in the
set. -/
def filterKeyNotIn (m : KeyedMap K V) (ks : Multiset K) : KeyedMap K V :=
  ⟨m.keys.filter (fun k => k ∉ ks), fun k => if k ∈ ks then none else m.get k,
   fun k => by
    rw [Finset.mem_filter, ← m.dom k]
    by_cases h : k ∈ ks <;> simp [h]⟩

/-- Rust `KeyedSingleton::key_count()`. -/
def keyCount (m : KeyedMap K V) : Nat := m.keys.card

/-- Rust `get_max_key()`: the entry at the largest key (`None` when
empty). -/
def maxEntry [LinearOrder K] (m : KeyedMap K V) : Option (K × V) :=
  match m.keys.max with
  | ⊥ => none
  | (k : K) => (m.get k).map (fun v => (k, v))

/-- The keyed fold of a keyed pool (Rust `KeyedStream::fold`): one value
per key, the key's content folded with the graded obligation
(conditional on `Pre` over the key's own values). -/
def ofFold {ord : StrOrd} {ret : Retries} [DecidableEq V] {A : Type}
    (g : A → V → A) (init : A) (Pre : V → V → Prop) [DecidableRel Pre]
    (ok : FoldOkP ord ret Pre g)
    (p : KeyedPool K V ord ret) : KeyedMap K A :=
  ⟨p.keys, fun k => if k ∈ p.keys then some (PoolFold ord ret Pre g init ok (p.grp k)) else none,
   fun k => by by_cases h : k ∈ p.keys <;> simp [h]⟩

end KeyedMap

/-- **The per-key pairwise fact**: at every key, the content's pairs
satisfy `Pre` (what a keyed `ghost pre` proves; hereditary to the
machine's per-key realized content). -/
def KeyedPool.Pairwise {K V : Type} [DecidableEq K] [DecidableEq V] {ord : StrOrd} {ret : Retries}
    (Pre : V → V → Prop) (p : KeyedPool K V ord ret) : Prop :=
  ∀ k, PoolPairwise ord ret Pre (p.grp k)

@[simp] theorem KeyedPool.Pairwise_true {K V : Type} [DecidableEq K] [DecidableEq V]
    {ord : StrOrd} {ret : Retries}
    (p : KeyedPool K V ord ret) : p.Pairwise (fun _ _ => True) :=
  fun k => PoolPairwise_true ord ret (p.grp k)

/-! ## Reduces (Rust `reduce`/`reduce_watermark`) -/

/-- A reduce as a fold with an optional accumulator: the first value
seeds, later ones combine. Its graded `FoldOk` is the reduce's
commutativity/idempotence obligation. -/
def reduceStep {V : Type} (f : V → V → V) (acc : Option V) (x : V) : Option V :=
  some (match acc with | none => x | some a => f a x)

/-! ## Early-stopping folds (Rust `fold_early_stop`) -/

/-- Run `f` over the values until it signals stop; `some acc` iff it
stopped. -/
def foldEarlyStop {V A : Type} (f : A → V → A × Bool) : A → List V → Option A
  | _, [] => none
  | acc, v :: vs =>
    let (acc', stop) := f acc v
    if stop then some acc' else foldEarlyStop f acc' vs

/-- The completed buckets of an early-stopping keyed fold over ordered
keyed content: an entry per key whose fold stopped. -/
def KeyedMap.ofFoldEarlyStop {K V A : Type} [DecidableEq K] [DecidableEq V]
    (f : A → V → A × Bool) (init : A)
    (p : KeyedPool K V .totalOrder .exactlyOnce) : KeyedMap K A :=
  ⟨p.keys.filter (fun k => (foldEarlyStop f init (p.grp k)).isSome),
   fun k => if k ∈ p.keys then foldEarlyStop f init (p.grp k) else none,
   fun k => by
    rw [Finset.mem_filter]
    by_cases h : k ∈ p.keys <;> simp [h]⟩

/-! ## Reads of a keyed-sourced singleton: increment cuts of pairs

A root `KeyedSingleton` (a keyed fold's live map) is read at ticks like
any fold singleton: the decision is the per-tick consumed increment of
the keyed SOURCE's pairs (`CutDec (K × V) .noOrder`), legal iff per key
the consumed pairs are the next prefix of that key's content (ordered
content) or fit the key's multiset (unordered). The view at a legal cut
is itself keyed content (`readAt`), which the fold reads. -/

namespace KeyedPool
variable {K V : Type} [DecidableEq K] [DecidableEq V]

/-- Is the cumulative consumed multiset `M` a legal view of `p`? -/
def legalView : {ord : StrOrd} → KeyedPool K V ord .exactlyOnce →
    Multiset (K × V) → Bool
  | .totalOrder => fun p M =>
      decide (∀ k ∈ (M.map Prod.fst).toFinset,
        (M.filter (fun e => e.1 = k)).map Prod.snd
          = ↑((p.grp k).take ((M.filter (fun e => e.1 = k)).card)))
  | .noOrder => fun p M => decide (M ≤ p.toMultiset)

/-- The keyed content a legal cumulative view `M` exposes. -/
def readAt : {ord : StrOrd} → KeyedPool K V ord .exactlyOnce →
    Multiset (K × V) → KeyedPool K V ord .exactlyOnce
  | .totalOrder => fun p M =>
      ⟨(M.map Prod.fst).toFinset,
       fun k => (p.grp k).take ((M.filter (fun e => e.1 = k)).card),
       fun k hk => by
        rw [Multiset.mem_toFinset] at hk
        show List.take _ _ = []
        rw [Multiset.filter_eq_nil.mpr
          (fun e he hke => hk (Multiset.mem_map.mpr ⟨e, he, hke⟩))]
        rfl⟩
  | .noOrder => fun _ M => ofMultiset M

/-- The realized cumulative views of a keyed source under increment
cuts (an illegal increment blocks, as `snapshotCuts`). -/
def kviews {ord : StrOrd} (p : KeyedPool K V ord .exactlyOnce) :
    Multiset (K × V) → List (Multiset (K × V)) → Trace (Multiset (K × V))
  | _, [] => []
  | acc, b :: ds =>
    if legalView p (acc + b) then (acc + b) :: kviews p (acc + b) ds
    else []

end KeyedPool

/-! ### Reads grow with the source (the Flo face of keyed reads) -/

namespace KeyedPool
variable {K V : Type} [DecidableEq K] [DecidableEq V]

/-- A view legal for a pool is legal for any larger pool. -/
theorem legalView_mono {ord : StrOrd} {a b : KeyedPool K V ord .exactlyOnce}
    (h : Le a b) {M : Multiset (K × V)} (hl : legalView a M = true) :
    legalView b M = true := by
  cases ord
  · simp only [legalView, decide_eq_true_eq] at hl ⊢
    intro k hk
    have hk' := hl k hk
    have hlen : ((a.grp k).take ((M.filter (fun e => e.1 = k)).card)).length
        = (M.filter (fun e => e.1 = k)).card := by
      have := congrArg Multiset.card hk'
      rw [Multiset.card_map, Multiset.coe_card] at this
      exact this.symm
    rw [List.length_take] at hlen
    have hle : (M.filter (fun e => e.1 = k)).card ≤ (a.grp k).length := by
      rcases Nat.le_total ((M.filter (fun e => e.1 = k)).card) (a.grp k).length with h' | h'
      · exact h'
      · rw [Nat.min_eq_right h'] at hlen; omega
    obtain ⟨e, he⟩ := h k
    rw [← he, List.take_append_of_le_length hle]
    exact hk'
  · simp only [legalView, decide_eq_true_eq] at hl ⊢
    exact le_trans hl (toMultiset_le h)

/-- On a legal view, the read does not depend on the pool beyond it. -/
theorem readAt_eq_of_legal {ord : StrOrd} {a b : KeyedPool K V ord .exactlyOnce}
    (h : Le a b) {M : Multiset (K × V)} (hl : legalView a M = true) :
    readAt a M = readAt b M := by
  cases ord
  · simp only [legalView, decide_eq_true_eq] at hl
    refine ext rfl (funext fun k => ?_)
    show (a.grp k).take _ = (b.grp k).take _
    by_cases hk : k ∈ (M.map Prod.fst).toFinset
    · have hk' := hl k hk
      have hlen : ((a.grp k).take ((M.filter (fun e => e.1 = k)).card)).length
          = (M.filter (fun e => e.1 = k)).card := by
        have := congrArg Multiset.card hk'
        rw [Multiset.card_map, Multiset.coe_card] at this
        exact this.symm
      rw [List.length_take] at hlen
      have hle : (M.filter (fun e => e.1 = k)).card ≤ (a.grp k).length := by
        rcases Nat.le_total ((M.filter (fun e => e.1 = k)).card) (a.grp k).length with h' | h'
        · exact h'
        · rw [Nat.min_eq_right h'] at hlen; omega
      obtain ⟨e, he⟩ := h k
      rw [← he, List.take_append_of_le_length hle]
    · rw [Multiset.mem_toFinset] at hk
      have hz : (M.filter (fun e => e.1 = k)).card = 0 := by
        rw [Multiset.card_eq_zero]
        exact Multiset.filter_eq_nil.mpr
          (fun e he hke => hk (Multiset.mem_map.mpr ⟨e, he, hke⟩))
      rw [hz]
      rfl
  · rfl

/-- Views realized against a pool are realized against any larger pool,
and the reads agree on them. -/
theorem kviews_map_readAt_le {ord : StrOrd} {a b : KeyedPool K V ord .exactlyOnce}
    (h : Le a b) {β : Type} (F : KeyedPool K V ord .exactlyOnce → β) :
    ∀ (acc : Multiset (K × V)) (d : List (Multiset (K × V))),
      (kviews a acc d).map (fun M => F (readAt a M))
        <+: (kviews b acc d).map (fun M => F (readAt b M))
  | _, [] => List.prefix_refl _
  | acc, inc :: ds => by
    simp only [kviews]
    by_cases hl : legalView a (acc + inc) = true
    · rw [if_pos hl, if_pos (legalView_mono h hl), List.map_cons, List.map_cons,
        readAt_eq_of_legal h hl]
      exact List.cons_prefix_cons.mpr ⟨rfl, kviews_map_readAt_le h F (acc + inc) ds⟩
    · rw [if_neg hl]
      exact List.nil_prefix

end KeyedPool

/-! ### Reading the keyed-singleton chain (`fold_early_stop` →
`get_max_key` → `snapshot`; the `p_p1b` readers, D71)

The views a keyed read realizes are legal cumulative cuts that ascend
along ticks; per key, a view exposes a prefix of the key's ordered
content, so a stopped bucket, once realized, is read identically at
every later tick. The closed forms below are what a program's ghost
proofs read off the operators. -/

namespace KeyedPool
variable {K V : Type} [DecidableEq K] [DecidableEq V]

/-- Every realized view is legal. -/
theorem kviews_legal {ord : StrOrd} (p : KeyedPool K V ord .exactlyOnce) :
    ∀ (acc : Multiset (K × V)) (d : List (Multiset (K × V))) {t : Nat}
      {M : Multiset (K × V)},
      (kviews p acc d)[t]? = some M → legalView p M = true
  | _, [], _, _, h => by cases h
  | acc, b :: ds, t, M, h => by
    unfold kviews at h
    by_cases hl : legalView p (acc + b) = true
    · rw [if_pos hl] at h
      cases t with
      | zero => exact (Option.some.inj h) ▸ hl
      | succ t => exact kviews_legal p (acc + b) ds h
    · rw [if_neg hl] at h
      cases h

/-- Realized views ascend along ticks (cumulative increments). -/
theorem kviews_le {ord : StrOrd} (p : KeyedPool K V ord .exactlyOnce) :
    ∀ (acc : Multiset (K × V)) (d : List (Multiset (K × V))) {t t' : Nat}
      {M M' : Multiset (K × V)}, t ≤ t' →
      (kviews p acc d)[t]? = some M → (kviews p acc d)[t']? = some M' → M ≤ M'
  | _, [], _, _, _, _, _, h, _ => by cases h
  | acc, b :: ds, t, t', M, M', htt, h, h' => by
    unfold kviews at h h'
    by_cases hl : legalView p (acc + b) = true
    · rw [if_pos hl] at h h'
      cases t with
      | zero =>
        obtain rfl := Option.some.inj h
        cases t' with
        | zero => exact le_of_eq (Option.some.inj h')
        | succ t' => exact kviews_ge_acc p (acc + b) ds h'
      | succ t =>
        cases t' with
        | zero => omega
        | succ t' => exact kviews_le p (acc + b) ds (Nat.le_of_succ_le_succ htt) h h'
    · rw [if_neg hl] at h
      cases h
where
  /-- Every realized view contains the running accumulator. -/
  kviews_ge_acc {ord : StrOrd} (p : KeyedPool K V ord .exactlyOnce) :
      ∀ (acc : Multiset (K × V)) (d : List (Multiset (K × V))) {t : Nat}
        {M : Multiset (K × V)}, (kviews p acc d)[t]? = some M → acc ≤ M
    | _, [], _, _, h => by cases h
    | acc, b :: ds, t, M, h => by
      unfold kviews at h
      by_cases hl : legalView p (acc + b) = true
      · rw [if_pos hl] at h
        cases t with
        | zero => exact (Option.some.inj h) ▸ Multiset.le_add_right _ _
        | succ t => exact le_trans (Multiset.le_add_right _ _) (kviews_ge_acc p (acc + b) ds h)
      · rw [if_neg hl] at h
        cases h

/-- A key's read content at an ordered view is a prefix of the key's
content. -/
theorem readAt_grp_prefix (p : KeyedPool K V .totalOrder .exactlyOnce)
    (M : Multiset (K × V)) (k : K) : (p.readAt M).grp k <+: p.grp k :=
  List.take_prefix _ _

/-- Ordered reads at ascending views ascend per key. -/
theorem readAt_grp_prefix_of_le (p : KeyedPool K V .totalOrder .exactlyOnce)
    {M M' : Multiset (K × V)} (h : M ≤ M') (k : K) :
    (p.readAt M).grp k <+: (p.readAt M').grp k :=
  List.take_prefix_take_left
    (Multiset.card_le_card (Multiset.filter_le_filter _ h))

/-- A key with content is a key of the pool. -/
theorem mem_keys_of_grp_ne {ord : StrOrd} {ret : Retries} (p : KeyedPool K V ord ret) {k : K}
    (h : p.grp k ≠ PoolBot ord ret) : k ∈ p.keys := by
  by_contra hk
  exact h (p.dom k hk)

end KeyedPool

/-- A prefix of a list is its `take` of its own length, so a `take` that
the prefix already covers reads the same on both. -/
theorem List.take_eq_of_prefix_of_le {α : Type} {c : Nat} {xs ys : List α}
    (h : xs <+: ys) (hfull : c ≤ xs.length) : ys.take c = xs.take c := by
  obtain ⟨e, rfl⟩ := h
  rw [List.take_append_of_le_length hfull]

/-- The capped push (paxos.rs:553–559's `fold_early_stop` closure —
collect until `q` values): from an accumulator below the cap, the fold
stops exactly when the cap is reached, yielding the first `max q 1`
values; it never stops on fewer. -/
theorem foldEarlyStop_push {V : Type} (q : Nat) :
    ∀ (acc vs : List V), acc.length < max q 1 →
      foldEarlyStop (fun logs log => (logs ++ [log], decide (q ≤ logs.length + 1))) acc vs
        = if max q 1 ≤ acc.length + vs.length then some ((acc ++ vs).take (max q 1)) else none
  | acc, [], hlt => by
    simp only [foldEarlyStop, List.length_nil, Nat.add_zero]
    rw [if_neg (Nat.not_le.mpr hlt)]
  | acc, v :: vs, hlt => by
    by_cases hq : q ≤ acc.length + 1
    · simp only [foldEarlyStop, decide_eq_true hq, if_true]
      have hmax : max q 1 = acc.length + 1 := by omega
      rw [if_pos (by simp only [List.length_cons]; omega), hmax]
      rw [show acc ++ v :: vs = (acc ++ [v]) ++ vs by simp,
        List.take_append_of_le_length (by simp), List.take_of_length_le (by simp)]
    · simp only [foldEarlyStop, decide_eq_false hq, Bool.false_eq_true, if_false]
      have hlt' : (acc ++ [v]).length < max q 1 := by
        simp only [List.length_append, List.length_singleton]; omega
      rw [foldEarlyStop_push q (acc ++ [v]) vs hlt']
      simp only [List.length_append, List.length_cons, List.length_nil,
        List.append_assoc, List.singleton_append]
      rw [show acc.length + 1 + vs.length = acc.length + (vs.length + 1) by omega]

/-- The capped push from the empty accumulator. -/
theorem foldEarlyStop_push_nil {V : Type} (q : Nat) (vs : List V) :
    foldEarlyStop (fun logs log => (logs ++ [log], decide (q ≤ logs.length + 1))) [] vs
      = if max q 1 ≤ vs.length then some (vs.take (max q 1)) else none := by
  have := foldEarlyStop_push q [] vs (by simp)
  simpa using this

namespace KeyedMap
variable {K V : Type} [DecidableEq K] [DecidableEq V]

/-- An early-stopping fold's entry at a key: the key's fold, when it
stopped. -/
theorem ofFoldEarlyStop_get {A : Type} (f : A → V → A × Bool) (init : A)
    (p : KeyedPool K V .totalOrder .exactlyOnce) (k : K) :
    (ofFoldEarlyStop f init p).get k
      = if k ∈ p.keys then foldEarlyStop f init (p.grp k) else none := rfl

/-- **The bucket closed form**: under the capped push, a key has an entry
iff `max q 1` values arrived at it, and the entry is their first `max q 1`
in the key's order. -/
theorem ofFoldEarlyStop_push_get_eq_some_iff (q : Nat)
    (p : KeyedPool K V .totalOrder .exactlyOnce) (k : K) (vs : List V) :
    (ofFoldEarlyStop (fun logs log => (logs ++ [log], decide (q ≤ logs.length + 1))) [] p).get k
        = some vs
      ↔ max q 1 ≤ (p.grp k).length ∧ vs = (p.grp k).take (max q 1) := by
  rw [ofFoldEarlyStop_get, foldEarlyStop_push_nil]
  constructor
  · intro h
    by_cases hk : k ∈ p.keys
    · rw [if_pos hk] at h
      by_cases hfull : max q 1 ≤ (p.grp k).length
      · rw [if_pos hfull] at h
        exact ⟨hfull, (Option.some.inj h).symm⟩
      · rw [if_neg hfull] at h
        cases h
    · rw [if_neg hk] at h
      cases h
  · rintro ⟨hfull, rfl⟩
    have hne : p.grp k ≠ PoolBot .totalOrder .exactlyOnce := by
      intro h
      rw [h] at hfull
      change max q 1 ≤ ([] : List V).length at hfull
      simp at hfull
    rw [if_pos (p.mem_keys_of_grp_ne hne), if_pos hfull]

omit [DecidableEq V] in
/-- `get_max_key`'s answer is the entry at the largest key: it IS an
entry, and its key dominates every key. -/
theorem maxEntry_eq_some_iff [LinearOrder K] (m : KeyedMap K V) (k : K) (v : V) :
    m.maxEntry = some (k, v) ↔ m.get k = some v ∧ ∀ k' ∈ m.keys, k' ≤ k := by
  unfold maxEntry
  cases hmax : m.keys.max with
  | bot =>
    have hempty := Finset.max_eq_bot.mp hmax
    constructor
    · intro h; cases h
    · rintro ⟨hget, -⟩
      have := (m.dom k).mp (by rw [hget]; rfl)
      rw [hempty] at this
      exact absurd this (Finset.notMem_empty _)
  | coe k₀ =>
    have hk₀ := Finset.mem_of_max hmax
    change (m.get k₀).map (fun v => (k₀, v)) = some (k, v) ↔ _
    constructor
    · intro h
      obtain ⟨v₀, hv₀, heq⟩ := Option.map_eq_some_iff.mp h
      obtain ⟨rfl, rfl⟩ := Prod.mk.inj heq
      exact ⟨hv₀, fun k' hk' => Finset.le_max_of_eq hk' hmax⟩
    · rintro ⟨hget, hdom⟩
      have hk : k ∈ m.keys := (m.dom k).mp (by rw [hget]; rfl)
      have h1 : k ≤ k₀ := Finset.le_max_of_eq hk hmax
      have h2 : k₀ ≤ k := hdom k₀ hk₀
      obtain rfl := le_antisymm h1 h2
      rw [hget]
      rfl

omit [DecidableEq V] in
/-- An entry forces `get_max_key` to answer, at a key at least the
entry's. -/
theorem maxEntry_of_get [LinearOrder K] {m : KeyedMap K V} {k : K} {v : V}
    (h : m.get k = some v) : ∃ r, m.maxEntry = some r ∧ k ≤ r.1 := by
  have hk : k ∈ m.keys := (m.dom k).mp (by rw [h]; rfl)
  obtain ⟨k₀, hk₀⟩ := Finset.max_of_mem hk
  have hmem := Finset.mem_of_max hk₀
  obtain ⟨v₀, hv₀⟩ := Option.isSome_iff_exists.mp ((m.dom k₀).mpr hmem)
  refine ⟨(k₀, v₀), ?_, Finset.le_max_of_eq hk hk₀⟩
  exact (maxEntry_eq_some_iff m k₀ v₀).mpr ⟨hv₀, fun k' hk' => Finset.le_max_of_eq hk' hk₀⟩

end KeyedMap

/-! ## Reduce and watermark GC across ticks (the atomic-trace keyed ops) -/

/-- The running maximum of an optional watermark trace: a watermark only
rises (a lower or absent reading is ignored). -/
def runMaxOpt {K : Type} [LinearOrder K] : Option K → List (Option K) → List (Option K)
  | _, [] => []
  | cur, w :: ws =>
    let cur' := match cur, w with
      | none, w => w
      | some a, none => some a
      | some a, some b => some (max a b)
    cur' :: runMaxOpt cur' ws

theorem runMaxOpt_prefix {K : Type} [LinearOrder K] :
    ∀ (cur : Option K) {l l' : List (Option K)}, l <+: l' →
      runMaxOpt cur l <+: runMaxOpt cur l'
  | _, [], _, _ => List.nil_prefix
  | cur, w :: l, l', h => by
    obtain ⟨e, rfl⟩ := h
    rw [List.cons_append]
    simp only [runMaxOpt]
    exact List.cons_prefix_cons.mpr ⟨rfl, runMaxOpt_prefix _ ⟨e, rfl⟩⟩

namespace KeyedMap
variable {K V : Type} [DecidableEq K]

/-- Collapse an optional-valued map: the keys with a present value. -/
def collapse (m : KeyedMap K (Option V)) : KeyedMap K V :=
  ⟨m.keys.filter (fun k => ((m.get k).join).isSome), fun k => (m.get k).join,
   fun k => by
    rw [Finset.mem_filter, ← m.dom k]
    cases h : m.get k with
    | none => simp
    | some o => cases o <;> simp⟩

/-- The keyed reduce of a keyed pool (Rust `KeyedStream::reduce`): keys
with content, each reduced with the graded obligation. -/
def ofReduce {ord : StrOrd} {ret : Retries} [DecidableEq V] (f : V → V → V)
    (Pre : V → V → Prop) [DecidableRel Pre]
    (ok : FoldOkP ord ret Pre (reduceStep f)) (p : KeyedPool K V ord ret) : KeyedMap K V :=
  (ofFold (reduceStep f) none Pre ok p).collapse

/-- Garbage collection below a watermark: keep the keys at or above it
(no watermark: keep all). -/
def gcBelow [LinearOrder K] (w : Option K) (m : KeyedMap K V) : KeyedMap K V :=
  match w with
  | none => m
  | some w => ⟨m.keys.filter (fun k => w ≤ k),
      fun k => if w ≤ k then m.get k else none,
      fun k => by
        rw [Finset.mem_filter, ← m.dom k]
        by_cases h : w ≤ k <;> simp [h]⟩

/-! ### Reading the keys back -/

/-- The keys of a filtered map: the keys whose value passes. -/
theorem mem_keysMS_filter {m : KeyedMap K V} {p : V → Bool} {k : K} :
    k ∈ (m.filter p).keysMS ↔ k ∈ m.keysMS ∧ (m.get k).any p = true := by
  show k ∈ (m.keys.filter _).val ↔ k ∈ m.keys.val ∧ _
  rw [Finset.filter_val, Multiset.mem_filter]

/-- The keys of a keyed fold: the pool's keys. -/
theorem keysMS_ofFold {ord : StrOrd} {ret : Retries} [DecidableEq V] {A : Type}
    (g : A → V → A) (init : A) (Pre : V → V → Prop) [DecidableRel Pre]
    (ok : FoldOkP ord ret Pre g) (p : KeyedPool K V ord ret) :
    (ofFold g init Pre ok p).keysMS = p.keys.val := rfl

end KeyedMap

/-- The keys of a grouped multiset: its keys, once each. -/
theorem KeyedPool.keys_ofMultiset {K V : Type} [DecidableEq K] [DecidableEq V]
    (m : Multiset (K × V)) : (KeyedPool.ofMultiset m).keys.val = (m.map Prod.fst).dedup := rfl

/-- Per-key concatenation of keyed content (exactly-once grades). -/
def KeyedPool.append {K V : Type} [DecidableEq K] [DecidableEq V] {ord : StrOrd}
    (a b : KeyedPool K V ord .exactlyOnce) : KeyedPool K V ord .exactlyOnce :=
  ⟨a.keys ∪ b.keys, fun k => poolChain (ord := ord) (a.grp k) (b.grp k), fun k hk => by
    rw [Finset.mem_union, not_or] at hk
    rw [a.dom k hk.1, b.dom k hk.2]
    cases ord <;> rfl⟩

/-- Rust `KeyedStream::map(|v| …)`: the per-key content mapped, keys kept. -/
def KeyedPool.map {K V W : Type} [DecidableEq K] [DecidableEq V] [DecidableEq W]
    {ord : StrOrd} (f : V → W) (p : KeyedPool K V ord .exactlyOnce) :
    KeyedPool K W ord .exactlyOnce :=
  ⟨p.keys, fun k => mapPool (ord := ord) f (p.grp k), fun k hk => by
    rw [p.dom k hk]
    cases ord <;> rfl⟩

/-- Rust `KeyedStream::weaken_ordering()`: every key's content forgotten
into the `NoOrder` quotient. -/
def KeyedPool.weakenOrder {K V : Type} [DecidableEq K] [DecidableEq V] {ord : StrOrd}
    (p : KeyedPool K V ord .exactlyOnce) : KeyedPool K V .noOrder .exactlyOnce :=
  ⟨p.keys, fun k => poolWeakenOrder (ord := ord) (p.grp k), fun k hk => by
    rw [p.dom k hk]
    cases ord <;> rfl⟩

/-- Rust `KeyedSingleton::into_keyed_stream()`: one element per key. -/
def KeyedPool.ofMap {K V : Type} [DecidableEq K] [DecidableEq V] (m : KeyedMap K V) :
    KeyedPool K V .totalOrder .exactlyOnce :=
  ⟨m.keys, fun k => (m.get k).toList, fun k hk => by
    have h : ¬ (m.get k).isSome := fun hs => hk ((m.dom k).mp hs)
    cases hm : m.get k with
    | none => rfl
    | some v => exact absurd (by rw [hm]; rfl) h⟩

/-- Rust `KeyedStream::entries()` at the denotation: the content as a flat
unordered stream of pairs (the keyed structure forgotten). -/
def KeyedPool.toPool {K V : Type} [DecidableEq K] [DecidableEq V] {ord : StrOrd} :
    (ret : Retries) → KeyedPool K V ord ret → PoolCarrier (K × V) .noOrder ret
  | .exactlyOnce => fun p => p.toMultiset
  | .atLeastOnce => fun p => RetryPool.mk
      ((p.keys.val.map (fun k => (KeyedPool.grpMS (p.grp k)).map (fun v => (k, v)))).sum)

/-- `reduce_watermark` across ticks at the denotation: the content seen so
far (every tick's batch appended), this tick's watermark raised, and
the per-key reduce of the content at or above it — one step per tick
(the whole-map form of hydro-project/hydro#3189, read at every tick). -/
def reduceWatermarkStep {K V : Type} [DecidableEq K] [DecidableEq V] [LinearOrder K]
    {ord : StrOrd} (f : V → V → V) (Pre : V → V → Prop) [DecidableRel Pre]
    (ok : FoldOkP ord .exactlyOnce Pre (reduceStep f))
    (st : KeyedPool K V ord .exactlyOnce × Option K)
    (inp : KeyedPool K V ord .exactlyOnce × Option K) :
    (KeyedPool K V ord .exactlyOnce × Option K) × KeyedMap K V :=
  let w := match st.2, inp.2 with
    | none, w => w
    | some a, none => some a
    | some a, some b => some (max a b)
  let acc := st.1.append inp.1
  ((acc, w), KeyedMap.gcBelow w (KeyedMap.ofReduce f Pre ok acc))

/-! ## The flat machine representation and its coupling

The step machine keeps the runtime's flat lists: a keyed stream is the
wire of pairs (`into_keyed` is a cast); a keyed singleton is the
runtime's association list **with the map's unique-keys invariant**
(`SchedKMap` — a `HashMap`'s entries have distinct keys; every machine
operator preserves it). The coupling with the denotation is per key:
grouping the list IS the pool (`CoupledKeyed`), the association list
read as a map IS the map (`KeyedMap.ofAssoc`). -/

/-- The machine's keyed singleton: an association list with distinct
keys (the runtime's map, flattened). -/
def SchedKMap (K V : Type) : Type := {l : List (K × V) // (l.map Prod.fst).Nodup}

namespace SchedKMap
variable {K V W : Type} [DecidableEq K]

def entries (m : SchedKMap K V) : List (K × V) := m.val
def keys (m : SchedKMap K V) : List K := m.val.map Prod.fst
def lookup (m : SchedKMap K V) (k : K) : Option V :=
  (m.val.find? (fun e => decide (e.1 = k))).map Prod.snd

/-- The empty map. -/
def empty : SchedKMap K V := ⟨[], List.nodup_nil⟩

/-- The association list with first-arrival keys dedup'd (how every
machine keyed fold is keyed). -/
def ofKeyed (keys : List K) (f : K → Option V) : SchedKMap K V :=
  ⟨keys.dedup.filterMap (fun k => (f k).map (fun v => (k, v))), by
    have : (keys.dedup.filterMap (fun k => (f k).map (fun v => (k, v)))).map Prod.fst
        = keys.dedup.filter (fun k => (f k).isSome) := by
      induction keys.dedup with
      | nil => rfl
      | cons k rest ih =>
        simp only [List.filterMap_cons, List.filter_cons]
        cases f k with
        | none => simpa using ih
        | some v => simpa using ih
    rw [this]
    exact (List.nodup_dedup keys).filter _⟩

/-- Keep the entries passing a predicate (keys stay distinct). -/
def filterEntries (m : SchedKMap K V) (p : K × V → Bool) : SchedKMap K V :=
  ⟨m.val.filter p, (List.Sublist.map Prod.fst List.filter_sublist).nodup m.property⟩

def filter (m : SchedKMap K V) (p : V → Bool) : SchedKMap K V :=
  m.filterEntries (fun e => p e.2)

def map (m : SchedKMap K V) (f : V → W) : SchedKMap K W :=
  ⟨m.val.map (fun e => (e.1, f e.2)), by
    have : (m.val.map (fun e => (e.1, f e.2))).map Prod.fst = m.val.map Prod.fst := by
      rw [List.map_map]; rfl
    rw [this]; exact m.property⟩

/-- Rust `join_keyed_singleton` on association lists: `a`'s keys, each
paired with `b`'s value when `b` holds it (keyed as every machine keyed
fold is, by `ofKeyed`). -/
def join (a : SchedKMap K V) (b : SchedKMap K W) : SchedKMap K (V × W) :=
  ofKeyed a.keys (fun k => (a.lookup k).bind (fun v => (b.lookup k).map (fun w => (v, w))))

/-- Rust `key_count` on the association list. -/
def keyCount (m : SchedKMap K V) : Nat := m.val.length

/-- Rust `get_max_key` on the association list. -/
def maxEntry [LinearOrder K] (m : SchedKMap K V) : Option (K × V) :=
  m.val.foldl (fun acc e => match acc with
    | none => some e
    | some a => if a.1 < e.1 then some e else some a) none

/-- Watermark GC on the association list. -/
def gcBelow [LinearOrder K] (w : Option K) (m : SchedKMap K V) : SchedKMap K V :=
  match w with
  | none => m
  | some w => m.filterEntries (fun e => decide (w ≤ e.1))

end SchedKMap

/-- The machine's keyed fold on an arrival list: per key in first-arrival
order, the key's values folded in arrival order. -/
def keyedFoldAssoc {K V A : Type} [DecidableEq K] (g : A → V → A) (init : A)
    (l : List (K × V)) : SchedKMap K A :=
  SchedKMap.ofKeyed (l.map Prod.fst) (fun k => some ((keyVals k l).foldl g init))

/-- The machine's `fold_early_stop` on an arrival list: the completed
buckets. -/
def keyedFoldEarlyStopAssoc {K V A : Type} [DecidableEq K]
    (f : A → V → A × Bool) (init : A) (l : List (K × V)) : SchedKMap K A :=
  SchedKMap.ofKeyed (l.map Prod.fst) (fun k => foldEarlyStop f init (keyVals k l))

/-- The machine's keyed reduce on an arrival list. -/
def keyedReduceAssoc {K V : Type} [DecidableEq K] (f : V → V → V)
    (l : List (K × V)) : SchedKMap K V :=
  SchedKMap.ofKeyed (l.map Prod.fst) (fun k => (keyVals k l).foldl (reduceStep f) none)

/-- The accumulate-form of `reduce_watermark`'s step on lists (the
denotation's `reduceWatermarkStep`, mirrored on the machine's flat
lists): the arrivals so far, this tick's appended; the watermark raised;
the per-key reduce in arrival order, GC'd. A proof device: the machine
runs DFIR's eager fold (`reduceWatermarkEager`), simulated by this. -/
def reduceWatermarkStepAssoc {K V : Type} [DecidableEq K] [LinearOrder K]
    (f : V → V → V) (st : List (K × V) × Option K) (inp : List (K × V) × Option K) :
    (List (K × V) × Option K) × SchedKMap K V :=
  let w := match st.2, inp.2 with
    | none, w => w
    | some a, none => some a
    | some a, some b => some (max a b)
  let acc := st.1 ++ inp.1
  ((acc, w), SchedKMap.gcBelow w (keyedReduceAssoc f acc))


theorem SchedKMap.lookup_isSome_iff {K V : Type} [DecidableEq K] (m : SchedKMap K V) (k : K) :
    (m.lookup k).isSome ↔ k ∈ m.val.map Prod.fst := by
  unfold SchedKMap.lookup
  rw [Option.isSome_map, List.find?_isSome]
  constructor
  · rintro ⟨e, he, hk⟩
    exact List.mem_map.mpr ⟨e, he, of_decide_eq_true hk⟩
  · intro h
    obtain ⟨e, he, rfl⟩ := List.mem_map.mp h
    exact ⟨e, he, by simp⟩

/-- An association list read as a map. -/
def KeyedMap.ofAssoc {K V : Type} [DecidableEq K] (m : SchedKMap K V) : KeyedMap K V :=
  ⟨(m.val.map Prod.fst).toFinset, m.lookup, fun k => by
    rw [List.mem_toFinset]; exact m.lookup_isSome_iff k⟩

/-- Two association lists with the same lookups read as the same map. -/
theorem KeyedMap.ofAssoc_ext {K V : Type} [DecidableEq K] {m m' : SchedKMap K V}
    (h : ∀ k, m.lookup k = m'.lookup k) : KeyedMap.ofAssoc m = KeyedMap.ofAssoc m' := by
  refine KeyedMap.ext ?_ (funext h)
  ext k
  show k ∈ (m.val.map Prod.fst).toFinset ↔ k ∈ (m'.val.map Prod.fst).toFinset
  rw [List.mem_toFinset, List.mem_toFinset, ← m.lookup_isSome_iff, ← m'.lookup_isSome_iff, h]


/-- One tick's keyed stream, machine list vs keyed pool (exactly-once:
grouping the list IS the pool; the retry grades carry no per-tick
coupling, as `CoupledBatch`). -/
def CoupledKeyed {K V : Type} [DecidableEq K] [DecidableEq V] :
    (ord : StrOrd) → (ret : Retries) → List (K × V) →
      KeyedPool K V ord ret → Prop
  | .totalOrder, .exactlyOnce => fun l p => KeyedPool.ofList l = p
  | .noOrder, .exactlyOnce => fun l p => KeyedPool.ofMultiset (↑l) = p
  | .totalOrder, .atLeastOnce => fun _ _ => True
  | .noOrder, .atLeastOnce => fun _ _ => True

/-! ### The machine's association lists read as maps -/

namespace SchedKMap
variable {K V : Type} [DecidableEq K]

theorem find?_filterMap_tag (f : K → Option V) (k : K) :
    ∀ (ks : List K), ks.Nodup →
      (ks.filterMap (fun k' => (f k').map (fun v => (k', v)))).find? (fun e => decide (e.1 = k))
        = if k ∈ ks then (f k).map (fun v => (k, v)) else none
  | [], _ => by simp
  | k' :: ks, hnd => by
    have ih := find?_filterMap_tag f k ks (List.nodup_cons.mp hnd).2
    have hnot := (List.nodup_cons.mp hnd).1
    cases hf : f k' with
    | none =>
      simp only [List.filterMap_cons, hf, Option.map_none, ih, List.mem_cons]
      by_cases hk : k = k'
      · subst hk
        simp [hf, hnot]
      · simp [hk]
    | some v =>
      simp only [List.filterMap_cons, hf, Option.map_some, List.find?_cons, List.mem_cons]
      by_cases hk : k' = k
      · subst hk
        simp [hf]
      · have hk' : ¬ k = k' := fun h => hk h.symm
        simp [hk, hk', ih]

theorem lookup_ofKeyed (ks : List K) (f : K → Option V) (k : K) :
    (ofKeyed ks f).lookup k = if k ∈ ks then f k else none := by
  unfold lookup ofKeyed
  show ((ks.dedup.filterMap (fun k' => (f k').map (fun v => (k', v)))).find?
      (fun e => decide (e.1 = k))).map Prod.snd = _
  rw [find?_filterMap_tag f k ks.dedup (List.nodup_dedup ks)]
  by_cases hm : k ∈ ks
  · rw [if_pos (List.mem_dedup.mpr hm), if_pos hm]
    cases f k <;> rfl
  · rw [if_neg (fun h => hm (List.mem_dedup.mp h)), if_neg hm]
    rfl

theorem keys_ofKeyed (ks : List K) (f : K → Option V) :
    ((ofKeyed ks f).val.map Prod.fst).toFinset = ks.toFinset.filter (fun k => (f k).isSome) := by
  ext k
  rw [List.mem_toFinset, Finset.mem_filter, List.mem_toFinset]
  unfold ofKeyed
  show k ∈ (ks.dedup.filterMap (fun k' => (f k').map (fun v => (k', v)))).map Prod.fst ↔ _
  constructor
  · intro h
    obtain ⟨e, he, rfl⟩ := List.mem_map.mp h
    obtain ⟨k', hk', hfe⟩ := List.mem_filterMap.mp he
    obtain ⟨v, hv, rfl⟩ := Option.map_eq_some_iff.mp hfe
    exact ⟨List.mem_dedup.mp hk', by simp [hv]⟩
  · rintro ⟨hm, hs⟩
    obtain ⟨v, hv⟩ := Option.isSome_iff_exists.mp hs
    exact List.mem_map.mpr ⟨(k, v), List.mem_filterMap.mpr ⟨k, List.mem_dedup.mpr hm, by simp [hv]⟩, rfl⟩

end SchedKMap

/-! ### DFIR's `ReduceKeyedWatermark` (`hydro_lang/src/compile/ir/mod.rs:5221–5310`)

A `fold::<'static>` over `(HashMap, Option<watermark>)`, fed this tick's
payloads then this tick's watermark (`chain()` of the two inputs): a
payload at a key below the current watermark is dropped, otherwise
inserted or reduced into its entry; a watermark `≤` the current one is
ignored, otherwise the map retains the keys `≥` it and the watermark is
set; the map is the output. The machine runs exactly this on the
association list. -/

/-- `k < curr_watermark` (no watermark yet: nothing is below). -/
def belowWm {K : Type} [LinearOrder K] (w : Option K) (k : K) : Bool :=
  match w with
  | none => false
  | some w => decide (k < w)

namespace SchedKMap
variable {K V : Type} [DecidableEq K]

/-- `map.entry(k)`: `Vacant → insert v`, `Occupied → f(entry, v)`, on the
association list. -/
def insertReduceList (f : V → V → V) (l : List (K × V)) (kv : K × V) : List (K × V) :=
  if (l.find? (fun e => decide (e.1 = kv.1))).isSome then
    l.map (fun e => if e.1 = kv.1 then (e.1, f e.2 kv.2) else e)
  else l ++ [kv]

/-- The same, keys kept distinct. -/
def insertReduce (f : V → V → V) (m : SchedKMap K V) (kv : K × V) : SchedKMap K V :=
  ⟨insertReduceList f m.val kv, by
    unfold insertReduceList
    split
    · have : (m.val.map (fun e => if e.1 = kv.1 then (e.1, f e.2 kv.2) else e)).map Prod.fst
          = m.val.map Prod.fst := by
        rw [List.map_map]
        exact List.map_congr_left (fun e _ => by
          show (if e.1 = kv.1 then (e.1, f e.2 kv.2) else e).1 = e.1
          split <;> rfl)
      rw [this]; exact m.property
    · rename_i h
      rw [List.map_append, List.map_singleton, List.nodup_append_comm, List.singleton_append,
        List.nodup_cons]
      refine ⟨fun hk => h ?_, m.property⟩
      rw [List.find?_isSome]
      obtain ⟨e, he, hek⟩ := List.mem_map.mp hk
      exact ⟨e, he, decide_eq_true hek⟩⟩

theorem lookup_insertReduce (f : V → V → V) (m : SchedKMap K V) (kv : K × V) (k : K) :
    (m.insertReduce f kv).lookup k
      = if k = kv.1 then reduceStep f (m.lookup k) kv.2 else m.lookup k := by
  show ((insertReduceList f m.val kv).find? (fun e => decide (e.1 = k))).map Prod.snd = _
  unfold insertReduceList
  by_cases h : (m.val.find? (fun e => decide (e.1 = kv.1))).isSome = true
  · rw [if_pos h, List.find?_map]
    have hp : ((fun e : K × V => decide (e.1 = k)) ∘
        fun e => if e.1 = kv.1 then (e.1, f e.2 kv.2) else e) = fun e => decide (e.1 = k) := by
      funext e
      show decide ((if e.1 = kv.1 then (e.1, f e.2 kv.2) else e).1 = k) = decide (e.1 = k)
      split <;> rfl
    rw [hp]
    unfold lookup
    by_cases hk : k = kv.1
    · rw [if_pos hk, hk]
      obtain ⟨e, he⟩ := Option.isSome_iff_exists.mp h
      rw [he]
      have hd : decide (e.1 = kv.1) = true := List.find?_some (p := fun e : K × V => decide (e.1 = kv.1)) he
      have hk' : e.1 = kv.1 := of_decide_eq_true hd
      simp [hk', reduceStep]
    · rw [if_neg hk]
      cases hf : m.val.find? (fun e => decide (e.1 = k)) with
      | none => rfl
      | some e =>
        have hd : decide (e.1 = k) = true := List.find?_some (p := fun e : K × V => decide (e.1 = k)) hf
        have hk' : e.1 = k := of_decide_eq_true hd
        have : ¬ e.1 = kv.1 := fun h' => hk (hk'.symm.trans h')
        simp [this]
  · rw [if_neg h, List.find?_append]
    unfold lookup
    by_cases hk : k = kv.1
    · rw [if_pos hk, hk, Option.not_isSome_iff_eq_none.mp h]
      simp [reduceStep]
    · rw [if_neg hk]
      cases hf : m.val.find? (fun e => decide (e.1 = k)) with
      | none =>
        have : ¬ kv.1 = k := fun h' => hk h'.symm
        simp [this]
      | some e => rfl

theorem lookup_gcBelow [LinearOrder K] (w : Option K) (m : SchedKMap K V) (k : K) :
    (m.gcBelow w).lookup k = if belowWm w k then none else m.lookup k := by
  cases w with
  | none => rfl
  | some w =>
    show ((m.val.filter (fun e => decide (w ≤ e.1))).find? (fun e => decide (e.1 = k))).map
      Prod.snd = if decide (k < w) then none else m.lookup k
    unfold lookup
    rw [List.find?_filter]
    by_cases hk : k < w
    · rw [if_pos (by simpa using hk)]
      rw [List.find?_eq_none.mpr (fun e _ => by
        by_cases he : e.1 = k
        · simp [he, not_le.mpr hk]
        · simp [he])]
      rfl
    · rw [if_neg (by simpa using hk)]
      have hp : (fun a : K × V => decide (decide (w ≤ a.1) = true ∧ decide (a.1 = k) = true))
          = fun e => decide (e.1 = k) := by
        funext e
        by_cases he : e.1 = k
        · simp [he, not_lt.mp hk]
        · simp [he]
      rw [hp]

end SchedKMap

/-- DFIR's per-payload step: drop below the current watermark, else
insert-or-reduce. -/
def reduceWatermarkPayload {K V : Type} [DecidableEq K] [LinearOrder K] (f : V → V → V)
    (w : Option K) (m : SchedKMap K V) (kv : K × V) : SchedKMap K V :=
  if belowWm w kv.1 then m else m.insertReduce f kv

/-- DFIR's per-watermark step: ignore if not above the current one, else
retain the keys at or above it and set. -/
def reduceWatermarkAdvance {K V : Type} [DecidableEq K] [LinearOrder K]
    (st : SchedKMap K V × Option K) (wm : Option K) : SchedKMap K V × Option K :=
  match st.2, wm with
  | _, none => st
  | none, some b => (st.1.gcBelow (some b), some b)
  | some a, some b => if b ≤ a then st else (st.1.gcBelow (some b), some b)

/-- `reduce_watermark`'s per-tick step on the machine, as DFIR runs it:
this tick's payloads folded in (`chain` input `[0]`), then this tick's
watermark (`[1]`); the map is the output. -/
def reduceWatermarkEager {K V : Type} [DecidableEq K] [LinearOrder K] (f : V → V → V)
    (st : SchedKMap K V × Option K) (inp : List (K × V) × Option K) :
    (SchedKMap K V × Option K) × SchedKMap K V :=
  let st' := reduceWatermarkAdvance (inp.1.foldl (reduceWatermarkPayload f st.2) st.1, st.2) inp.2
  (st', st'.1)

/-- The eager fold's map, by key: below the watermark nothing; above it,
the key's values folded onto the entry. -/
theorem lookup_foldl_payload {K V : Type} [DecidableEq K] [LinearOrder K] (f : V → V → V)
    (w : Option K) (k : K) :
    ∀ (l : List (K × V)) (m : SchedKMap K V),
      (l.foldl (reduceWatermarkPayload f w) m).lookup k
        = if belowWm w k then m.lookup k
          else (keyVals k l).foldl (reduceStep f) (m.lookup k)
  | [], m => by cases belowWm w k <;> rfl
  | kv :: l, m => by
    rw [List.foldl_cons, lookup_foldl_payload f w k l]
    unfold reduceWatermarkPayload
    by_cases hb : belowWm w kv.1 = true
    · rw [if_pos hb]
      by_cases hk : belowWm w k = true
      · rw [if_pos hk, if_pos hk]
      · rw [if_neg hk, if_neg hk]
        have hne : ¬ kv.1 = k := fun h => hk (h ▸ hb)
        simp [keyVals, hne]
    · rw [if_neg hb, SchedKMap.lookup_insertReduce]
      by_cases hk : belowWm w k = true
      · rw [if_pos hk, if_pos hk]
        have hne : ¬ k = kv.1 := fun h => hb (h ▸ hk)
        rw [if_neg hne]
      · rw [if_neg hk, if_neg hk]
        by_cases hke : kv.1 = k
        · rw [if_pos hke.symm]
          simp [keyVals, hke]
        · rw [if_neg (fun h => hke h.symm)]
          simp [keyVals, hke]

/-- The reduce of a key's arrivals (`none` iff there were none). -/
theorem lookup_keyedReduceAssoc {K V : Type} [DecidableEq K] (f : V → V → V)
    (l : List (K × V)) (k : K) :
    (keyedReduceAssoc f l).lookup k = (keyVals k l).foldl (reduceStep f) none := by
  unfold keyedReduceAssoc
  rw [SchedKMap.lookup_ofKeyed]
  by_cases hk : k ∈ l.map Prod.fst
  · rw [if_pos hk]
  · rw [if_neg hk]
    have : keyVals k l = [] := by
      unfold keyVals
      rw [List.filter_eq_nil_iff.mpr (fun e he hke => hk ?_)]
      · rfl
      · exact List.mem_map.mpr ⟨e, he, of_decide_eq_true hke⟩
    rw [this]; rfl

/-- The raised watermark, as the accumulate form computes it. -/
def raiseWm {K : Type} [LinearOrder K] (a b : Option K) : Option K :=
  match a, b with
  | none, w => w
  | some a, none => some a
  | some a, some b => some (max a b)

theorem reduceWatermarkAdvance_snd {K V : Type} [DecidableEq K] [LinearOrder K]
    (st : SchedKMap K V × Option K) (wm : Option K) :
    (reduceWatermarkAdvance st wm).2 = raiseWm st.2 wm := by
  unfold reduceWatermarkAdvance raiseWm
  rcases st with ⟨m, w⟩
  cases w <;> cases wm <;> simp only
  rename_i a b
  split
  · simp [max_eq_left ‹_›]
  · simp [max_eq_right (not_le.mp ‹_›).le]

/-- The map after the watermark step, by key — for a map already empty
below its watermark (the fold's invariant). -/
theorem lookup_reduceWatermarkAdvance_fst {K V : Type} [DecidableEq K] [LinearOrder K]
    (st : SchedKMap K V × Option K) (wm : Option K) (k : K)
    (hinv : ∀ k, belowWm st.2 k = true → st.1.lookup k = none) :
    (reduceWatermarkAdvance st wm).1.lookup k
      = if belowWm (raiseWm st.2 wm) k then none else st.1.lookup k := by
  unfold reduceWatermarkAdvance raiseWm
  rcases st with ⟨m, w⟩
  cases w with
  | none =>
    cases wm with
    | none => rfl
    | some b => exact SchedKMap.lookup_gcBelow (some b) m k
  | some a =>
    have hkeep : m.lookup k = if belowWm (some a) k then none else m.lookup k := by
      by_cases hb : belowWm (some a) k = true
      · rw [if_pos hb]; exact hinv k hb
      · rw [if_neg hb]
    cases wm with
    | none => exact hkeep
    | some b =>
      simp only
      split
      · rename_i hba
        rw [max_eq_left hba]
        exact hkeep
      · rename_i hba
        rw [max_eq_right (not_le.mp hba).le]
        exact SchedKMap.lookup_gcBelow (some b) m k

/-- A watermark only rises: below the old one is below the raised one. -/
theorem belowWm_raise {K : Type} [LinearOrder K] (a b : Option K) (k : K) :
    belowWm a k = true → belowWm (raiseWm a b) k = true := by
  unfold belowWm raiseWm
  cases a with
  | none => intro h; cases h
  | some a =>
    cases b with
    | none => exact id
    | some b =>
      intro h
      exact decide_eq_true (lt_of_lt_of_le (of_decide_eq_true h) (le_max_left a b))

/-- **The eager fold is the accumulate form, step by step** (the
simulation the coupling rides): a machine state agrees with an
accumulate state when the watermarks agree and the map, by key, is the
GC'd reduce of the arrivals so far. -/
def EagerSim {K V : Type} [DecidableEq K] [LinearOrder K] (f : V → V → V)
    (st : SchedKMap K V × Option K) (acc : List (K × V) × Option K) : Prop :=
  st.2 = acc.2 ∧ ∀ k, st.1.lookup k = (SchedKMap.gcBelow acc.2 (keyedReduceAssoc f acc.1)).lookup k

theorem EagerSim.init {K V : Type} [DecidableEq K] [LinearOrder K] (f : V → V → V) :
    EagerSim f (SchedKMap.empty (K := K) (V := V), none) ([], none) :=
  ⟨rfl, fun _ => rfl⟩

theorem EagerSim.step {K V : Type} [DecidableEq K] [LinearOrder K] (f : V → V → V)
    {st : SchedKMap K V × Option K} {acc : List (K × V) × Option K}
    (h : EagerSim f st acc) (inp : List (K × V) × Option K) :
    EagerSim f (reduceWatermarkEager f st inp).1 (reduceWatermarkStepAssoc f acc inp).1
      ∧ ∀ k, (reduceWatermarkEager f st inp).2.lookup k
          = (reduceWatermarkStepAssoc f acc inp).2.lookup k := by
  obtain ⟨hw, hm⟩ := h
  have hraise : (reduceWatermarkStepAssoc f acc inp).1.2 = raiseWm acc.2 inp.2 := rfl
  have hout : ∀ k, (reduceWatermarkEager f st inp).2.lookup k
      = (reduceWatermarkStepAssoc f acc inp).2.lookup k := by
    intro k
    show (reduceWatermarkAdvance (inp.1.foldl (reduceWatermarkPayload f st.2) st.1, st.2) inp.2).1.lookup k
      = (SchedKMap.gcBelow (raiseWm acc.2 inp.2) (keyedReduceAssoc f (acc.1 ++ inp.1))).lookup k
    have hinv : ∀ k, belowWm st.2 k = true →
        (inp.1.foldl (reduceWatermarkPayload f st.2) st.1).lookup k = none := fun k hb => by
      rw [lookup_foldl_payload, if_pos hb, hm k, SchedKMap.lookup_gcBelow, if_pos (hw ▸ hb)]
    rw [lookup_reduceWatermarkAdvance_fst _ _ _ hinv, SchedKMap.lookup_gcBelow,
      lookup_keyedReduceAssoc, lookup_foldl_payload]
    simp only [hw]
    by_cases hbr : belowWm (raiseWm acc.2 inp.2) k = true
    · rw [if_pos hbr, if_pos hbr]
    · rw [if_neg hbr, if_neg hbr]
      have hb : ¬ belowWm acc.2 k = true := fun hb => hbr (belowWm_raise _ _ _ hb)
      rw [if_neg hb, hm k, SchedKMap.lookup_gcBelow, if_neg hb, lookup_keyedReduceAssoc,
        keyVals_append, List.foldl_append]
  refine ⟨⟨?_, fun k => ?_⟩, hout⟩
  · show (reduceWatermarkAdvance _ inp.2).2 = _
    rw [reduceWatermarkAdvance_snd, hraise]
    show raiseWm st.2 inp.2 = raiseWm acc.2 inp.2
    rw [hw]
  · exact hout k

/-- The eager fold's output trace is the accumulate form's. -/
theorem scan_eager_eq_assoc {K V : Type} [DecidableEq K] [LinearOrder K] (f : V → V → V) :
    ∀ (l : List (List (K × V) × Option K)) (st : SchedKMap K V × Option K)
      (acc : List (K × V) × Option K), EagerSim f st acc →
      (scanAcrossTicksTrace (reduceWatermarkEager f) st l).map KeyedMap.ofAssoc
        = (scanAcrossTicksTrace (reduceWatermarkStepAssoc f) acc l).map KeyedMap.ofAssoc
  | [], _, _, _ => rfl
  | a :: l, st, acc, h => by
    obtain ⟨h1, h2⟩ := EagerSim.step f h a
    show KeyedMap.ofAssoc (reduceWatermarkEager f st a).2 :: _
      = KeyedMap.ofAssoc (reduceWatermarkStepAssoc f acc a).2 :: _
    rw [KeyedMap.ofAssoc_ext h2, scan_eager_eq_assoc f l _ _ h1]

namespace KeyedMap
variable {K V : Type} [DecidableEq K]

/-- An association list built per key, read as a map. -/
theorem ofAssoc_ofKeyed (ks : List K) (f : K → Option V) :
    ofAssoc (SchedKMap.ofKeyed ks f) = ⟨ks.toFinset.filter (fun k => (f k).isSome),
      fun k => if k ∈ ks then f k else none, fun k => by
        rw [Finset.mem_filter, List.mem_toFinset]
        by_cases h : k ∈ ks <;> simp [h]⟩ :=
  KeyedMap.ext (SchedKMap.keys_ofKeyed ks f) (funext (SchedKMap.lookup_ofKeyed ks f))

/-- Membership in a distinct-keys association list is the lookup. -/
theorem SchedKMap.mem_iff_lookup (m : SchedKMap K V) (k : K) (v : V) :
    (k, v) ∈ m.val ↔ m.lookup k = some v := by
  unfold SchedKMap.lookup
  constructor
  · intro h
    have hs : (m.val.find? (fun e => decide (e.1 = k))).isSome :=
      List.find?_isSome.mpr ⟨(k, v), h, by simp⟩
    obtain ⟨e, he⟩ := Option.isSome_iff_exists.mp hs
    have hmem := List.mem_of_find?_eq_some he
    have hk : e.1 = k := of_decide_eq_true (List.find?_some (p := fun e : K × V => decide (e.1 = k)) he)
    have : e = (k, v) := List.inj_on_of_nodup_map m.property hmem h (by simp [hk])
    rw [he, this]
    rfl
  · intro h
    obtain ⟨e, he, hev⟩ := Option.map_eq_some_iff.mp h
    have hmem := List.mem_of_find?_eq_some he
    have hk : e.1 = k := of_decide_eq_true (List.find?_some (p := fun e : K × V => decide (e.1 = k)) he)
    have : e = (k, v) := Prod.ext hk hev
    exact this ▸ hmem

theorem SchedKMap.lookup_eq_none_iff (m : SchedKMap K V) (k : K) :
    m.lookup k = none ↔ k ∉ m.val.map Prod.fst := by
  rw [← SchedKMap.lookup_isSome_iff]
  cases m.lookup k <;> simp

theorem ofAssoc_filter (m : SchedKMap K V) (p : V → Bool) :
    ofAssoc (m.filter p) = (ofAssoc m).filter p := by
  have hget : ∀ k, (m.filter p).lookup k = (m.lookup k).filter p := by
    intro k
    cases hl : m.lookup k with
    | none =>
      rw [Option.filter_none]
      rw [SchedKMap.lookup_eq_none_iff] at hl ⊢
      intro hk
      obtain ⟨e, he, rfl⟩ := List.mem_map.mp hk
      exact hl (List.mem_map.mpr ⟨e, (List.mem_filter.mp he).1, rfl⟩)
    | some v =>
      have hmem := (SchedKMap.mem_iff_lookup m k v).mpr hl
      by_cases hp : p v = true
      · rw [Option.filter_some, if_pos hp]
        exact (SchedKMap.mem_iff_lookup (m.filter p) k v).mp
          (List.mem_filter.mpr ⟨hmem, hp⟩)
      · rw [Option.filter_some, if_neg hp]
        rw [SchedKMap.lookup_eq_none_iff]
        intro hk
        obtain ⟨e, he, hek⟩ := List.mem_map.mp hk
        obtain ⟨he', hpe⟩ := List.mem_filter.mp he
        have : e = (k, v) := List.inj_on_of_nodup_map m.property he' hmem (by simp [hek])
        subst this
        exact hp hpe
  refine KeyedMap.ext ?_ (funext hget)
  ext k
  show k ∈ ((m.filter p).val.map Prod.fst).toFinset
    ↔ k ∈ (m.val.map Prod.fst).toFinset.filter (fun k => (m.lookup k).any p)
  rw [Finset.mem_filter, List.mem_toFinset, ← SchedKMap.lookup_isSome_iff, hget k,
    List.mem_toFinset, ← SchedKMap.lookup_isSome_iff]
  cases m.lookup k with
  | none => simp
  | some v => by_cases hp : p v = true <;> simp [hp]

theorem ofAssoc_map {W : Type} (m : SchedKMap K V) (f : V → W) :
    ofAssoc (m.map f) = (ofAssoc m).map f := by
  refine KeyedMap.ext ?_ (funext fun k => ?_)
  · show ((m.val.map (fun e => (e.1, f e.2))).map Prod.fst).toFinset = (m.val.map Prod.fst).toFinset
    rw [List.map_map]; rfl
  · show (m.map f).lookup k = (m.lookup k).map f
    cases hl : m.lookup k with
    | none =>
      rw [Option.map_none, SchedKMap.lookup_eq_none_iff]
      rw [SchedKMap.lookup_eq_none_iff] at hl
      intro hk
      obtain ⟨e, he, hek⟩ := List.mem_map.mp hk
      obtain ⟨e', he', rfl⟩ := List.mem_map.mp he
      exact hl (List.mem_map.mpr ⟨e', he', hek⟩)
    | some v =>
      rw [Option.map_some]
      exact (SchedKMap.mem_iff_lookup (m.map f) k (f v)).mp
        (List.mem_map.mpr ⟨(k, v), (SchedKMap.mem_iff_lookup m k v).mpr hl, rfl⟩)

theorem coe_keys_ofAssoc (m : SchedKMap K V) :
    (↑(m.keys) : Multiset K) = (ofAssoc m).keysMS := by
  show (↑(m.val.map Prod.fst) : Multiset K) = ((m.val.map Prod.fst).toFinset).val
  rw [List.toFinset_val, List.dedup_eq_self.mpr m.property]

theorem coe_entries_ofAssoc (m : SchedKMap K V) :
    (↑(m.entries) : Multiset (K × V)) = (ofAssoc m).entries := by
  show (↑m.val : Multiset (K × V))
    = ((m.val.map Prod.fst).toFinset).val.filterMap (fun k => (m.lookup k).map (fun v => (k, v)))
  rw [List.toFinset_val, List.dedup_eq_self.mpr m.property, Multiset.filterMap_coe,
    List.filterMap_map, Multiset.coe_eq_coe]
  refine List.Perm.of_eq ?_
  conv_lhs => rw [← List.map_id m.val, ← List.filterMap_eq_map]
  refine List.filterMap_congr (fun e he => ?_)
  show some e = (m.lookup e.1).map (fun v => (e.1, v))
  rw [(SchedKMap.mem_iff_lookup m e.1 e.2).mp he]
  rfl

/-- Filtering the entries by a key predicate, read as a map. -/
theorem ofAssoc_filterEntries_key (m : SchedKMap K V) (q : K → Bool) :
    ofAssoc (m.filterEntries (fun e => q e.1))
      = ⟨(ofAssoc m).keys.filter (fun k => q k), fun k => if q k then (ofAssoc m).get k else none,
         fun k => by
          rw [Finset.mem_filter, ← (ofAssoc m).dom k]
          by_cases h : q k = true <;> simp [h]⟩ := by
  have hget : ∀ k, (m.filterEntries (fun e => q e.1)).lookup k
      = if q k then m.lookup k else none := by
    intro k
    by_cases hq : q k = true
    · rw [if_pos hq]
      cases hl : m.lookup k with
      | none =>
        rw [SchedKMap.lookup_eq_none_iff] at hl ⊢
        intro hk
        obtain ⟨e, he, rfl⟩ := List.mem_map.mp hk
        exact hl (List.mem_map.mpr ⟨e, (List.mem_filter.mp he).1, rfl⟩)
      | some v =>
        exact (SchedKMap.mem_iff_lookup _ k v).mp
          (List.mem_filter.mpr ⟨(SchedKMap.mem_iff_lookup m k v).mpr hl, by simpa using hq⟩)
    · rw [if_neg hq, SchedKMap.lookup_eq_none_iff]
      intro hk
      obtain ⟨e, he, hek⟩ := List.mem_map.mp hk
      have := (List.mem_filter.mp he).2
      rw [hek] at this
      exact hq (by simpa using this)
  refine KeyedMap.ext ?_ (funext hget)
  ext k
  show k ∈ ((m.filterEntries (fun e => q e.1)).val.map Prod.fst).toFinset
    ↔ k ∈ (m.val.map Prod.fst).toFinset.filter (fun k => q k)
  rw [Finset.mem_filter, List.mem_toFinset, ← SchedKMap.lookup_isSome_iff, hget k,
    List.mem_toFinset, ← SchedKMap.lookup_isSome_iff]
  by_cases hq : q k = true <;> simp [hq]

theorem ofAssoc_gcBelow [LinearOrder K] (w : Option K) (m : SchedKMap K V) :
    ofAssoc (SchedKMap.gcBelow w m) = gcBelow w (ofAssoc m) := by
  cases w with
  | none => rfl
  | some w =>
    show ofAssoc (m.filterEntries (fun e => decide (w ≤ e.1))) = _
    rw [ofAssoc_filterEntries_key m (fun k => decide (w ≤ k))]
    refine KeyedMap.ext ?_ (funext fun k => ?_)
    · show (ofAssoc m).keys.filter (fun k => decide (w ≤ k) = true) = (ofAssoc m).keys.filter (fun k => w ≤ k)
      exact Finset.filter_congr (fun k _ => by simp)
    · show (if decide (w ≤ k) = true then _ else none) = (if w ≤ k then _ else none)
      by_cases h : w ≤ k <;> simp [h]

/-- The machine's keyed fold, read as a map, is the denotation's. -/
theorem ofAssoc_keyedFoldAssoc {A : Type} [DecidableEq V] {ord : StrOrd} (g : A → V → A)
    (init : A) (Pre : V → V → Prop) [DecidableRel Pre] (ok : FoldOkP ord .exactlyOnce Pre g)
    {l : List (K × V)}
    {p : KeyedPool K V ord .exactlyOnce} (h : CoupledKeyed ord .exactlyOnce l p)
    (hpre : p.Pairwise Pre) :
    ofAssoc (keyedFoldAssoc g init l) = ofFold g init Pre ok p := by
  unfold keyedFoldAssoc
  rw [ofAssoc_ofKeyed]
  cases ord
  · subst h
    refine KeyedMap.ext ?_ (funext fun k => ?_)
    · show (l.map Prod.fst).toFinset.filter (fun _ => true) = (l.map Prod.fst).toFinset
      simp
    · show (if k ∈ l.map Prod.fst then some ((keyVals k l).foldl g init) else none)
        = if k ∈ (l.map Prod.fst).toFinset then some ((keyVals k l).foldl g init) else none
      by_cases hk : k ∈ l.map Prod.fst
      · rw [if_pos hk, if_pos (List.mem_toFinset.mpr hk)]
      · rw [if_neg hk, if_neg (fun h => hk (List.mem_toFinset.mp h))]
  · subst h
    refine KeyedMap.ext ?_ (funext fun k => ?_)
    · show (l.map Prod.fst).toFinset.filter (fun _ => true)
        = ((↑l : Multiset (K × V)).map Prod.fst).toFinset
      simp
    · show (if k ∈ l.map Prod.fst then some ((keyVals k l).foldl g init) else none)
        = if k ∈ ((↑l : Multiset (K × V)).map Prod.fst).toFinset
          then some (Multiset.foldlOn Pre g init ok
            (((↑l : Multiset (K × V)).filter (fun e => e.1 = k)).map Prod.snd)) else none
      have hm : k ∈ ((↑l : Multiset (K × V)).map Prod.fst).toFinset ↔ k ∈ l.map Prod.fst := by
        rw [Multiset.map_coe, Multiset.mem_toFinset, Multiset.mem_coe]
      have hgrp : (((↑l : Multiset (K × V)).filter (fun e => e.1 = k)).map Prod.snd)
          = (↑(keyVals k l) : Multiset V) := by
        rw [Multiset.filter_coe, Multiset.map_coe]
        rfl
      have hpk : PairwiseL Pre (keyVals k l) := by
        have this : PairwiseM Pre (((↑l : Multiset (K × V)).filter (fun e => e.1 = k)).map Prod.snd) :=
          hpre k
        rw [hgrp] at this
        exact PairwiseM.coe.mp this
      have hv : Multiset.foldlOn Pre g init ok
          (((↑l : Multiset (K × V)).filter (fun e => e.1 = k)).map Prod.snd)
          = (keyVals k l).foldl g init := by
        rw [hgrp, Multiset.foldlOn_coe Pre g init ok hpk]
      rw [hv]
      by_cases hk : k ∈ l.map Prod.fst
      · rw [if_pos hk, if_pos (hm.mpr hk)]
      · rw [if_neg hk, if_neg (fun h => hk (hm.mp h))]

/-- A prefix of the arrival list is a legal view of its keyed content. -/
theorem legalView_ofList_prefix {l w : List (K × V)} [DecidableEq V] (h : w <+: l) :
    (KeyedPool.ofList l).legalView (↑w) = true := by
  simp only [KeyedPool.legalView, decide_eq_true_eq]
  intro k _
  rw [Multiset.filter_coe, Multiset.map_coe, Multiset.coe_card]
  have hkv : (w.filter (fun e => decide (e.1 = k))).map Prod.snd = keyVals k w := rfl
  have hlen : (w.filter (fun e => decide (e.1 = k))).length = (keyVals k w).length := by
    unfold keyVals; rw [List.length_map]
  rw [hkv, hlen]
  show (↑(keyVals k w) : Multiset V) = ↑((keyVals k l).take (keyVals k w).length)
  rw [← List.prefix_iff_eq_take.mp (keyVals_prefix h)]

theorem readAt_ofList_prefix {l w : List (K × V)} [DecidableEq V] (h : w <+: l) :
    (KeyedPool.ofList l).readAt (↑w) = KeyedPool.ofList w := by
  refine KeyedPool.ext ?_ (funext fun k => ?_)
  · show ((↑w : Multiset (K × V)).map Prod.fst).toFinset = (w.map Prod.fst).toFinset
    rw [Multiset.map_coe]; rfl
  · show (keyVals k l).take (((↑w : Multiset (K × V)).filter (fun e => e.1 = k)).card)
      = keyVals k w
    rw [Multiset.filter_coe, Multiset.coe_card]
    have hlen : (w.filter (fun e => decide (e.1 = k))).length = (keyVals k w).length := by
      unfold keyVals; rw [List.length_map]
    rw [hlen]
    exact (List.prefix_iff_eq_take.mp (keyVals_prefix h)).symm

/-- An arrival list whose per-key values are prefixes of a pool's content
is a legal view of that pool. -/
theorem legalView_of_keyed_prefix [DecidableEq V] {w : List (K × V)}
    {p : KeyedPool K V .totalOrder .exactlyOnce} (h : ∀ k, keyVals k w <+: p.grp k) :
    p.legalView (↑w) = true := by
  simp only [KeyedPool.legalView, decide_eq_true_eq]
  intro k _
  rw [Multiset.filter_coe, Multiset.map_coe, Multiset.coe_card]
  have hkv : (w.filter (fun e => decide (e.1 = k))).map Prod.snd = keyVals k w := rfl
  have hlen : (w.filter (fun e => decide (e.1 = k))).length = (keyVals k w).length := by
    unfold keyVals; rw [List.length_map]
  rw [hkv, hlen, ← List.prefix_iff_eq_take.mp (h k)]

theorem readAt_of_keyed_prefix [DecidableEq V] {w : List (K × V)}
    {p : KeyedPool K V .totalOrder .exactlyOnce} (h : ∀ k, keyVals k w <+: p.grp k) :
    p.readAt (↑w) = KeyedPool.ofList w := by
  refine KeyedPool.ext ?_ (funext fun k => ?_)
  · show ((↑w : Multiset (K × V)).map Prod.fst).toFinset = (w.map Prod.fst).toFinset
    rw [Multiset.map_coe]; rfl
  · show (p.grp k).take (((↑w : Multiset (K × V)).filter (fun e => e.1 = k)).card) = keyVals k w
    rw [Multiset.filter_coe, Multiset.coe_card]
    have hlen : (w.filter (fun e => decide (e.1 = k))).length = (keyVals k w).length := by
      unfold keyVals; rw [List.length_map]
    rw [hlen]
    exact (List.prefix_iff_eq_take.mp (h k)).symm

/-- The machine's `fold_early_stop`, read as a map, is the denotation's. -/
theorem ofAssoc_keyedFoldEarlyStopAssoc {A : Type} [DecidableEq V] (f : A → V → A × Bool)
    (init : A) (w : List (K × V)) :
    ofAssoc (keyedFoldEarlyStopAssoc f init w) = ofFoldEarlyStop f init (KeyedPool.ofList w) := by
  unfold keyedFoldEarlyStopAssoc
  rw [ofAssoc_ofKeyed]
  refine KeyedMap.ext rfl (funext fun k => ?_)
  show (if k ∈ w.map Prod.fst then foldEarlyStop f init (keyVals k w) else none)
    = if k ∈ (w.map Prod.fst).toFinset then foldEarlyStop f init (keyVals k w) else none
  by_cases hk : k ∈ w.map Prod.fst
  · rw [if_pos hk, if_pos (List.mem_toFinset.mpr hk)]
  · rw [if_neg hk, if_neg (fun h => hk (List.mem_toFinset.mp h))]

/-- The machine's keyed reduce, read as a map, is the denotation's. -/
theorem ofAssoc_keyedReduceAssoc [DecidableEq V] {ord : StrOrd} (f : V → V → V)
    (Pre : V → V → Prop) [DecidableRel Pre]
    (ok : FoldOkP ord .exactlyOnce Pre (reduceStep f)) {l : List (K × V)}
    {p : KeyedPool K V ord .exactlyOnce} (h : CoupledKeyed ord .exactlyOnce l p)
    (hpre : p.Pairwise Pre) :
    ofAssoc (keyedReduceAssoc f l) = ofReduce f Pre ok p := by
  unfold keyedReduceAssoc ofReduce
  rw [ofAssoc_ofKeyed]
  have hfold : ∀ k, PoolFold ord .exactlyOnce Pre (reduceStep f) none ok (p.grp k)
      = (keyVals k l).foldl (reduceStep f) none := by
    intro k
    cases ord
    · subst h; rfl
    · subst h
      have hgrp : (((↑l : Multiset (K × V)).filter (fun e => e.1 = k)).map Prod.snd)
          = (↑(keyVals k l) : Multiset V) := by
        rw [Multiset.filter_coe, Multiset.map_coe]
        rfl
      have hpk : PairwiseL Pre (keyVals k l) := by
        have this : PairwiseM Pre (((↑l : Multiset (K × V)).filter (fun e => e.1 = k)).map Prod.snd) :=
          hpre k
        rw [hgrp] at this
        exact PairwiseM.coe.mp this
      show Multiset.foldlOn Pre (reduceStep f) none ok
          (((↑l : Multiset (K × V)).filter (fun e => e.1 = k)).map Prod.snd) = _
      rw [hgrp, Multiset.foldlOn_coe Pre _ none ok hpk]
  have hkeys : p.keys = (l.map Prod.fst).toFinset := by
    cases ord
    · subst h; rfl
    · subst h
      show ((↑l : Multiset (K × V)).map Prod.fst).toFinset = _
      rw [Multiset.map_coe]; rfl
  refine KeyedMap.ext ?_ (funext fun k => ?_)
  · show (l.map Prod.fst).toFinset.filter (fun k => ((keyVals k l).foldl (reduceStep f) none).isSome)
      = (ofFold (reduceStep f) none Pre ok p).keys.filter _
    show _ = p.keys.filter (fun k => ((if k ∈ p.keys then
      some (PoolFold ord .exactlyOnce Pre (reduceStep f) none ok (p.grp k)) else none).join).isSome)
    rw [hkeys]
    refine Finset.filter_congr (fun k hk => ?_)
    rw [if_pos hk, Option.join_some, hfold]
  · show (if k ∈ l.map Prod.fst then (keyVals k l).foldl (reduceStep f) none else none)
      = ((if k ∈ p.keys then some (PoolFold ord .exactlyOnce Pre (reduceStep f) none ok (p.grp k))
          else none).join)
    rw [hkeys]
    by_cases hk : k ∈ l.map Prod.fst
    · rw [if_pos hk, if_pos (List.mem_toFinset.mpr hk), Option.join_some, hfold]
    · rw [if_neg hk, if_neg (fun h => hk (List.mem_toFinset.mp h))]; rfl

/-- The arrival-list `get_max_key` is the map's. -/
theorem maxEntry_ofAssoc [LinearOrder K] (m : SchedKMap K V) :
    (ofAssoc m).maxEntry = m.maxEntry := by
  -- the fold's specification: `none` iff empty; otherwise a maximal-key entry
  have spec : ∀ (l : List (K × V)) (acc : Option (K × V)),
      (l.foldl (fun acc e => match acc with
        | none => some e
        | some a => if a.1 < e.1 then some e else some a) acc = none ↔ acc = none ∧ l = []) ∧
      (∀ e, l.foldl (fun acc e => match acc with
        | none => some e
        | some a => if a.1 < e.1 then some e else some a) acc = some e →
        (e ∈ l ∨ acc = some e) ∧ (∀ e' ∈ l, e'.1 ≤ e.1) ∧ (∀ a, acc = some a → a.1 ≤ e.1)) := by
    intro l
    induction l with
    | nil =>
      intro acc
      refine ⟨by simp, fun e he => ⟨Or.inr he, by simp, fun a ha => by rw [ha] at he; cases he; exact le_rfl⟩⟩
    | cons x l ih =>
      intro acc
      rw [List.foldl_cons]
      have step : ∀ (acc' : Option (K × V)), (match acc with
          | none => some x
          | some a => if a.1 < x.1 then some x else some a) = acc' →
          acc' ≠ none ∧ ∀ e, acc' = some e → (e = x ∨ acc = some e) ∧ x.1 ≤ e.1 ∧
            (∀ a, acc = some a → a.1 ≤ e.1) := by
        intro acc' h
        cases acc with
        | none =>
          subst h
          exact ⟨by simp, fun e he => by cases he; exact ⟨Or.inl rfl, le_rfl, fun a ha => by cases ha⟩⟩
        | some a =>
          change (if a.1 < x.1 then some x else some a) = acc' at h
          by_cases hlt : a.1 < x.1
          · rw [if_pos hlt] at h; subst h
            exact ⟨by simp, fun e he => by cases he; exact ⟨Or.inl rfl, le_rfl, fun a' ha' => by cases ha'; exact le_of_lt hlt⟩⟩
          · rw [if_neg hlt] at h; subst h
            exact ⟨by simp, fun e he => by cases he; exact ⟨Or.inr rfl, not_lt.mp hlt, fun a' ha' => by cases ha'; exact le_rfl⟩⟩
      obtain ⟨hne, hsome⟩ := step _ rfl
      refine ⟨?_, ?_⟩
      · constructor
        · intro h
          exact absurd ((ih _).1.mp h).1 hne
        · rintro ⟨_, h⟩; cases h
      · intro e he
        obtain ⟨hmem, hmax, hacc⟩ := (ih _).2 e he
        rcases hmem with hmem | hmem
        · refine ⟨Or.inl (List.mem_cons_of_mem _ hmem), fun e' he' => ?_, fun a ha => ?_⟩
          · rcases List.mem_cons.mp he' with rfl | he'
            · obtain ⟨a', ha'⟩ := Option.ne_none_iff_exists'.mp hne
              exact le_trans (hsome a' ha').2.1 (hacc a' ha')
            · exact hmax e' he'
          · obtain ⟨a', ha'⟩ := Option.ne_none_iff_exists'.mp hne
            exact le_trans ((hsome a' ha').2.2 a ha) (hacc a' ha')
        · obtain ⟨hx, hxe, hacc'⟩ := hsome e hmem
          refine ⟨?_, fun e' he' => ?_, hacc'⟩
          · rcases hx with rfl | hx
            · exact Or.inl (List.mem_cons_self ..)
            · exact Or.inr hx
          · rcases List.mem_cons.mp he' with rfl | he'
            · exact hxe
            · exact hmax e' he'
  unfold maxEntry SchedKMap.maxEntry
  cases hl : m.val with
  | nil =>
    have : ((m.val.map Prod.fst).toFinset) = ∅ := by rw [hl]; rfl
    show (match ((m.val.map Prod.fst).toFinset).max with
      | ⊥ => none | (k : K) => (m.lookup k).map (fun v => (k, v))) = _
    rw [this, Finset.max_empty]
    rfl
  | cons x l =>
    have hne : m.val ≠ [] := by rw [hl]; simp
    -- the fold yields a maximal-key entry of the list
    have hres : ∃ e, m.val.foldl (fun acc e => match acc with
        | none => some e
        | some a => if a.1 < e.1 then some e else some a) none = some e := by
      cases h : m.val.foldl (fun acc e => match acc with
        | none => some e
        | some a => if a.1 < e.1 then some e else some a) none with
      | none => exact absurd ((spec m.val none).1.mp h).2 hne
      | some e => exact ⟨e, rfl⟩
    obtain ⟨e, he⟩ := hres
    obtain ⟨hmem, hmax, -⟩ := (spec m.val none).2 e he
    have hmem' : e ∈ m.val := hmem.resolve_right (by simp)
    rw [← hl] at *
    rw [he]
    -- the finset max is `e.1`
    have hkmem : e.1 ∈ (m.val.map Prod.fst).toFinset :=
      List.mem_toFinset.mpr (List.mem_map.mpr ⟨e, hmem', rfl⟩)
    have hmax' : ((m.val.map Prod.fst).toFinset).max = ↑e.1 := by
      obtain ⟨k, hk⟩ := Finset.max_of_mem hkmem
      have hkm := Finset.mem_of_max hk
      rw [List.mem_toFinset] at hkm
      obtain ⟨e', he', rfl⟩ := List.mem_map.mp hkm
      have h1 : e'.1 ≤ e.1 := hmax e' he'
      have h2 : e.1 ≤ e'.1 := WithBot.coe_le_coe.mp (hk ▸ Finset.le_max hkmem)
      rw [hk, le_antisymm h1 h2]
    show (match ((m.val.map Prod.fst).toFinset).max with
      | ⊥ => none | (k : K) => (m.lookup k).map (fun v => (k, v))) = some e
    rw [hmax']
    show (m.lookup e.1).map (fun v => (e.1, v)) = some e
    rw [(SchedKMap.mem_iff_lookup m e.1 e.2).mp hmem']
    rfl

end KeyedMap

/-- Keyed views extend under cut extension. -/
theorem KeyedPool.kviews_mono_dec {K V : Type} [DecidableEq K] [DecidableEq V] {ord : StrOrd}
    (p : KeyedPool K V ord .exactlyOnce) (acc : Multiset (K × V))
    {d d' : List (Multiset (K × V))} (h : d <+: d') :
    p.kviews acc d <+: p.kviews acc d' := by
  obtain ⟨e, rfl⟩ := h
  induction d generalizing acc with
  | nil => exact List.nil_prefix
  | cons b ds ih =>
    rw [List.cons_append]
    unfold KeyedPool.kviews
    by_cases hc : p.legalView (acc + b) = true
    · rw [if_pos hc, if_pos hc]
      exact List.cons_prefix_cons.mpr ⟨rfl, ih _⟩
    · rw [if_neg hc, if_neg hc]

/-- The empty arrivals group to the empty keyed content. -/
theorem CoupledKeyed.bot {K V : Type} [DecidableEq K] [DecidableEq V] {ord : StrOrd} :
    CoupledKeyed ord .exactlyOnce ([] : List (K × V)) KeyedPool.bot := by
  cases ord
  · exact KeyedPool.ext rfl rfl
  · exact KeyedPool.ext rfl rfl

/-- A relation-preserving scan: when a step relation `R` on states is
preserved and the emissions agree through `F`, the machine's scan, imaged
by `F`, is the denotation's scan on the imaged inputs. -/
theorem scanAcrossTicksTrace_map_of_rel {ι ι' σ σ' β β' : Type}
    (gS : σ → ι → σ × β) (gV : σ' → ι' → σ' × β') (R : σ → σ' → Prop)
    (c : ι → ι') (F : β → β')
    (hstep : ∀ s s' a, R s s' → R (gS s a).1 (gV s' (c a)).1 ∧ F (gS s a).2 = (gV s' (c a)).2) :
    ∀ (l : List ι) (s : σ) (s' : σ'), R s s' →
      (scanAcrossTicksTrace gS s l).map F = scanAcrossTicksTrace gV s' (l.map c)
  | [], _, _, _ => rfl
  | a :: l, s, s', h => by
    obtain ⟨h1, h2⟩ := hstep s s' a h
    show F (gS s a).2 :: (scanAcrossTicksTrace gS (gS s a).1 l).map F
      = (gV s' (c a)).2 :: scanAcrossTicksTrace gV (gV s' (c a)).1 (l.map c)
    rw [h2, scanAcrossTicksTrace_map_of_rel gS gV R c F hstep l _ _ h1]

/-- Keyed batch traces, machine vs denotation (as `BatchTraceLe`: equality
through grouping at the exactly-once grades, nothing at the retry
grades). -/
@[reducible] def KBatchTraceLe {K V : Type} [DecidableEq K] [DecidableEq V] :
    (ord : StrOrd) → (ret : Retries) →
      Trace (List (K × V)) → Trace (KeyedPool K V ord ret) → Prop
  | ord, .exactlyOnce => fun s v => s.map (KeyedPool.ofBatch (ord := ord)) <+: v
  | _, .atLeastOnce => fun _ _ => True

theorem List.Forall₂.map_eq {α β γ : Type} {R : α → β → Prop} {f : α → γ} {g : β → γ}
    (hfg : ∀ a b, R a b → f a = g b) :
    ∀ {l₁ : List α} {l₂ : List β}, List.Forall₂ R l₁ l₂ → l₁.map f = l₂.map g
  | [], [], _ => rfl
  | _ :: _, _ :: _, List.Forall₂.cons h hs => by
    rw [List.map_cons, List.map_cons, hfg _ _ h, List.Forall₂.map_eq hfg hs]

/-- Grouping concatenated arrivals is the appended keyed content. -/
theorem CoupledKeyed.append {K V : Type} [DecidableEq K] [DecidableEq V] {ord : StrOrd}
    {a l : List (K × V)} {pa p : KeyedPool K V ord .exactlyOnce}
    (ha : CoupledKeyed ord .exactlyOnce a pa) (hl : CoupledKeyed ord .exactlyOnce l p) :
    CoupledKeyed ord .exactlyOnce (a ++ l) (pa.append p) := by
  cases ord
  · subst ha; subst hl
    refine KeyedPool.ext ?_ (funext fun k => ?_)
    · show ((a ++ l).map Prod.fst).toFinset = (a.map Prod.fst).toFinset ∪ (l.map Prod.fst).toFinset
      rw [List.map_append, List.toFinset_append]
    · exact keyVals_append k a l
  · subst ha; subst hl
    refine KeyedPool.ext ?_ (funext fun k => ?_)
    · show ((↑(a ++ l) : Multiset (K × V)).map Prod.fst).toFinset
        = ((↑a : Multiset (K × V)).map Prod.fst).toFinset ∪ ((↑l : Multiset (K × V)).map Prod.fst).toFinset
      rw [← Multiset.coe_add, Multiset.map_add, Multiset.toFinset_add]
    · show ((↑(a ++ l) : Multiset (K × V)).filter (fun e => e.1 = k)).map Prod.snd
        = ((↑a : Multiset (K × V)).filter (fun e => e.1 = k)).map Prod.snd
          + ((↑l : Multiset (K × V)).filter (fun e => e.1 = k)).map Prod.snd
      rw [← Multiset.coe_add, Multiset.filter_add, Multiset.map_add]

/-- `CoupledKeyed` at the exactly-once grades IS grouping. -/
theorem CoupledKeyed.iff_ofBatch {K V : Type} [DecidableEq K] [DecidableEq V] {ord : StrOrd}
    {l : List (K × V)} {p : KeyedPool K V ord .exactlyOnce} :
    CoupledKeyed ord .exactlyOnce l p ↔ KeyedPool.ofBatch l = p := by
  cases ord <;> exact Iff.rfl

/-! ### The accumulated content of a keyed batch trace (what a
conditional `reduce_watermark`'s pairwise fact is about) -/

namespace KeyedPool
variable {K V : Type} [DecidableEq K] [DecidableEq V] {ord : StrOrd}

/-- Grouping concatenated arrivals is the appended grouping. -/
theorem ofBatch_append (a l : List (K × V)) :
    (ofBatch (ord := ord) (a ++ l)) = (ofBatch a).append (ofBatch l) :=
  CoupledKeyed.iff_ofBatch.mp
    (CoupledKeyed.append (CoupledKeyed.iff_ofBatch.mpr rfl) (CoupledKeyed.iff_ofBatch.mpr rfl))

theorem ofBatch_nil : (ofBatch (ord := ord) ([] : List (K × V))) = bot :=
  CoupledKeyed.iff_ofBatch.mp CoupledKeyed.bot

/-- Every tick's keyed content, appended (the content a
`reduce_watermark` has seen by the end of a batch trace). -/
def accum : List (KeyedPool K V ord .exactlyOnce) → KeyedPool K V ord .exactlyOnce
  | [] => bot
  | p :: ps => p.append (accum ps)

theorem accum_map_ofBatch : ∀ (xs : List (List (K × V))),
    accum (xs.map (ofBatch (ord := ord))) = ofBatch xs.flatten
  | [] => ofBatch_nil.symm
  | a :: xs => by
    show (ofBatch a).append (accum (xs.map ofBatch)) = ofBatch (a ++ xs.flatten)
    rw [accum_map_ofBatch xs, ofBatch_append]

/-- A pool is below its append with anything (per key: prefix / sub-multiset). -/
theorem Le.append_left (a b : KeyedPool K V ord .exactlyOnce) : Le a (a.append b) := by
  intro k
  show PoolLe ord .exactlyOnce (a.grp k) (poolChain (ord := ord) (a.grp k) (b.grp k))
  cases ord
  · exact ⟨_, rfl⟩
  · exact Multiset.le_add_right _ _

theorem Le.append_mono_right (a : KeyedPool K V ord .exactlyOnce)
    {b c : KeyedPool K V ord .exactlyOnce} (h : Le b c) : Le (a.append b) (a.append c) := by
  intro k
  have hk := h k
  show PoolLe ord .exactlyOnce (poolChain (ord := ord) (a.grp k) (b.grp k))
    (poolChain (ord := ord) (a.grp k) (c.grp k))
  cases ord
  · exact (List.prefix_append_right_inj _).mpr hk
  · exact Multiset.add_le_add_left hk

/-- Per-value images grow with the content. -/
theorem map_Le {W : Type} [DecidableEq W] (f : V → W) {a b : KeyedPool K V ord .exactlyOnce}
    (h : Le a b) : Le (KeyedPool.map f a) (KeyedPool.map f b) := by
  intro k
  show PoolLe ord .exactlyOnce (mapPool (ord := ord) f (a.grp k)) (mapPool (ord := ord) f (b.grp k))
  cases ord
  · exact (h k).map f
  · exact Multiset.map_le_map (h k)

/-- The accumulation of a prefix sits below the whole's. -/
theorem accum_take_le : ∀ (t : Nat) (l : List (KeyedPool K V ord .exactlyOnce)),
    Le (accum (l.take t)) (accum l)
  | 0, l => by
    show Le bot (accum l)
    intro k
    exact PoolLe.bot_le _ _ _
  | _ + 1, [] => Le.refl _
  | t + 1, p :: ps => by
    show Le (p.append (accum (ps.take t))) (p.append (accum ps))
    exact Le.append_mono_right p (accum_take_le t ps)

/-- The accumulation of a prefix sits below the whole's (prefix form). -/
theorem accum_prefix_le {l l' : List (KeyedPool K V ord .exactlyOnce)} (h : l <+: l') :
    Le (accum l) (accum l') := by
  obtain ⟨e, rfl⟩ := h
  have := accum_take_le l.length (l ++ e)
  rwa [List.take_left] at this

/-- The pairwise fact is hereditary downwards in the keyed growth order. -/
theorem Pairwise.of_le {Pre : V → V → Prop} {a b : KeyedPool K V ord .exactlyOnce}
    (h : Le a b) (hp : b.Pairwise Pre) : a.Pairwise Pre := by
  intro k
  have hk := h k
  have hpk := hp k
  cases ord
  · trivial
  · exact PairwiseM.of_le hk hpk

end KeyedPool

/-! ### Readers for `ghost pre` proofs (the pairwise fact over an
unordered keyed batch trace, read off the flat pool) -/

namespace KeyedPool
variable {K V : Type} [DecidableEq K] [DecidableEq V]

/-- The pairwise fact on grouped unordered pairs, read on the pairs:
every two pairs at one key satisfy `Pre` on their values. -/
theorem Pairwise_ofMultiset_iff {Pre : V → V → Prop} (m : Multiset (K × V)) :
    (ofMultiset m).Pairwise Pre ↔
      ∀ a ∈ m, ∀ b ∈ m, a.1 = b.1 → Pre a.2 b.2 := by
  constructor
  · intro h a ha b hb hk
    have hk' : PairwiseM Pre ((m.filter (fun e => e.1 = a.1)).map Prod.snd) := h a.1
    exact hk' a.2 (Multiset.mem_map.mpr ⟨a, Multiset.mem_filter.mpr ⟨ha, rfl⟩, rfl⟩)
      b.2 (Multiset.mem_map.mpr ⟨b, Multiset.mem_filter.mpr ⟨hb, hk.symm⟩, rfl⟩)
  · intro h k x hx y hy
    obtain ⟨a, ha, rfl⟩ := Multiset.mem_map.mp hx
    obtain ⟨b, hb, rfl⟩ := Multiset.mem_map.mp hy
    obtain ⟨ha, hak⟩ := Multiset.mem_filter.mp ha
    obtain ⟨hb, hbk⟩ := Multiset.mem_filter.mp hb
    exact h a ha b hb (hak.trans hbk.symm)

/-- Appending unordered groupings is grouping the sum. -/
theorem ofMultiset_add (a b : Multiset (K × V)) :
    (ofMultiset a).append (ofMultiset b) = ofMultiset (a + b) := by
  refine KeyedPool.ext ?_ (funext fun k => ?_)
  · show (a.map Prod.fst).toFinset ∪ (b.map Prod.fst).toFinset = ((a + b).map Prod.fst).toFinset
    rw [Multiset.map_add, Multiset.toFinset_add]
  · show ((a.filter (fun e => e.1 = k)).map Prod.snd) + ((b.filter (fun e => e.1 = k)).map Prod.snd)
      = ((a + b).filter (fun e => e.1 = k)).map Prod.snd
    rw [Multiset.filter_add, Multiset.map_add]

/-- The accumulation of an unordered batch trace is the grouping of the
batches' sum. -/
theorem accum_map_ofMultiset : ∀ (l : List (Multiset (K × V))),
    accum (l.map ofMultiset) = ofMultiset l.sum
  | [] => by
    show bot = ofMultiset 0
    exact KeyedPool.ext rfl rfl
  | b :: l => by
    show (ofMultiset b).append (accum (l.map ofMultiset)) = ofMultiset (b :: l).sum
    rw [accum_map_ofMultiset l, ofMultiset_add, List.sum_cons]

/-- **The `ghost pre` reader for `reduce_watermark (tinto_keyed (batch s d))`**:
the accumulated content's pairwise fact follows from the pairwise fact
on the source pool (batches are sub-pools of the source). -/
theorem accum_tinto_keyed_batch_pairwise {n : Nat} {Pre : V → V → Prop}
    (s : Fin n → Multiset (K × V)) (d : Fin n → List (Multiset (K × V))) (i : Fin n)
    (h : ∀ a ∈ s i, ∀ b ∈ s i, a.1 = b.1 → Pre a.2 b.2) :
    (accum ((batchCuts (s i) 0 (d i)).map (ofPool (ord := .noOrder)))).Pairwise Pre := by
  show (accum ((batchCuts (s i) 0 (d i)).map ofMultiset)).Pairwise Pre
  rw [accum_map_ofMultiset]
  have hle : (batchCuts (s i) 0 (d i)).sum ≤ s i := by
    have := batchCuts_sum_le (pool := s i) (d := d i) (consumed := 0) (Multiset.zero_le _)
    rwa [Multiset.zero_add] at this
  exact (Pairwise_ofMultiset_iff _).mpr fun a ha b hb hk =>
    h a (Multiset.mem_of_le hle ha) b (Multiset.mem_of_le hle hb) hk

end KeyedPool

/-- The machine's flat prefix, grouped, sits below the denotation's
accumulated content whenever the batch traces couple — so the pairwise
fact over the whole accumulation covers every realized prefix. -/
theorem KeyedPool.pairwise_flat_prefix {K V : Type} [DecidableEq K] [DecidableEq V]
    {ord : StrOrd} {Pre : V → V → Prop}
    {xs ms : List (List (K × V))} (hxs : xs <+: ms)
    {rr : Trace (KeyedPool K V ord .exactlyOnce)}
    (hms : ms.map (KeyedPool.ofBatch (ord := ord)) <+: rr)
    (hp : (KeyedPool.accum rr).Pairwise Pre) :
    (KeyedPool.ofBatch (ord := ord) xs.flatten).Pairwise Pre := by
  have h1 : xs.map (KeyedPool.ofBatch (ord := ord)) <+: rr :=
    (List.IsPrefix.map _ hxs).trans hms
  have h2 : xs.map (KeyedPool.ofBatch (ord := ord)) = rr.take (xs.map KeyedPool.ofBatch).length :=
    List.prefix_iff_eq_take.mp h1
  rw [← KeyedPool.accum_map_ofBatch, h2]
  exact KeyedPool.Pairwise.of_le (KeyedPool.accum_take_le _ _) hp

/-- The first components of a zip's prefix are a prefix of the first list. -/
theorem map_fst_zip_take_prefix {α β : Type _} (a : List α) (b : List β) (j : Nat) :
    (((List.zip a b).take j).map Prod.fst) <+: a := by
  have h1 : ((List.zip a b).take j).map Prod.fst <+: (List.zip a b).map Prod.fst :=
    List.IsPrefix.map _ (List.take_prefix j _)
  refine h1.trans ?_
  clear h1 j
  induction a generalizing b with
  | nil => exact List.nil_prefix
  | cons x xs ih =>
    cases b with
    | nil => exact List.nil_prefix
    | cons y ys =>
      show x :: (List.zip xs ys).map Prod.fst <+: x :: xs
      exact List.cons_prefix_cons.mpr ⟨rfl, ih ys⟩

/-- **The coupled reduce scan**: on arrivals whose accumulated content
satisfies the pairwise fact at every prefix, the machine's
accumulate-form scan, read as maps, is the denotation's scan on the
grouped inputs. (The hypothesis is stated over the machine's flat
prefixes — the shape the per-tick induction consumes.) -/
theorem reduce_scan_coupled {K V : Type} [DecidableEq K] [DecidableEq V] [LinearOrder K]
    {ord : StrOrd} (f : V → V → V) (Pre : V → V → Prop) [DecidableRel Pre]
    (ok : FoldOkP ord .exactlyOnce Pre (reduceStep f)) :
    ∀ (l : List (List (K × V) × Option K)) (acc : List (K × V))
      (p : KeyedPool K V ord .exactlyOnce) (w : Option K),
      CoupledKeyed ord .exactlyOnce acc p →
      (∀ j, (KeyedPool.ofBatch (ord := ord)
        (acc ++ ((l.take j).map Prod.fst).flatten)).Pairwise Pre) →
      (scanAcrossTicksTrace (reduceWatermarkStepAssoc f) (acc, w) l).map KeyedMap.ofAssoc
        = scanAcrossTicksTrace (reduceWatermarkStep f Pre ok) (p, w)
            (l.map (fun a => (KeyedPool.ofBatch a.1, a.2)))
  | [], _, _, _, _, _ => rfl
  | a :: l, acc, p, w, h, hp => by
    have hcpl : CoupledKeyed ord .exactlyOnce (acc ++ a.1) (p.append (KeyedPool.ofBatch a.1)) :=
      CoupledKeyed.append h (CoupledKeyed.iff_ofBatch.mpr rfl)
    have hpre1 : (p.append (KeyedPool.ofBatch a.1)).Pairwise Pre := by
      have := hp 1
      rw [← CoupledKeyed.iff_ofBatch.mp hcpl]
      simpa using this
    have hw : (match w, a.2 with
        | none, w => w | some a, none => some a | some a, some b => some (max a b)) =
        (match w, a.2 with
        | none, w => w | some a, none => some a | some a, some b => some (max a b)) := rfl
    show KeyedMap.ofAssoc (SchedKMap.gcBelow _ (keyedReduceAssoc f (acc ++ a.1)))
        :: (scanAcrossTicksTrace (reduceWatermarkStepAssoc f) (acc ++ a.1, _) l).map KeyedMap.ofAssoc
      = KeyedMap.gcBelow _ (KeyedMap.ofReduce f Pre ok (p.append (KeyedPool.ofBatch a.1)))
        :: scanAcrossTicksTrace (reduceWatermarkStep f Pre ok) (p.append (KeyedPool.ofBatch a.1), _)
            (l.map (fun a => (KeyedPool.ofBatch a.1, a.2)))
    rw [KeyedMap.ofAssoc_gcBelow, KeyedMap.ofAssoc_keyedReduceAssoc f Pre ok hcpl hpre1]
    congr 1
    refine reduce_scan_coupled f Pre ok l (acc ++ a.1) _ _ hcpl (fun j => ?_)
    have := hp (j + 1)
    simpa [List.append_assoc] using this

/-- One tick's batch, grouped, is coupled with the grouped pool. -/
theorem CoupledKeyed.ofPool_batch {K V : Type} [DecidableEq K] [DecidableEq V] {ord : StrOrd}
    {l : List (K × V)} {v : PoolCarrier (K × V) ord .exactlyOnce}
    (h : match ord with
      | .totalOrder => l = v
      | .noOrder => (↑l : Multiset (K × V)) = v) :
    CoupledKeyed ord .exactlyOnce l (KeyedPool.ofPool v) := by
  cases ord
  · show KeyedPool.ofList l = KeyedPool.ofList v
    rw [h]
  · show KeyedPool.ofMultiset ↑l = KeyedPool.ofMultiset v
    rw [h]

/-! ## Reading `reduce_watermark` at the denotation (unordered content)

The ghost readers for a program whose keyed source is unordered
(`tinto_keyed` of a tick wire of multisets — the acceptor log): the
register after `n` ticks is the grouped sum of the consumed batches
paired with the running watermark, and tick `t`'s published map is the
per-key reduce of the content so far, collected below the running
watermark. Then the map's own readers (`entries`, `gcBelow`,
`ofReduce` at a key) and the `HashMap`-insert fold the entries feed. -/

/-- The watermark join of `reduceWatermarkStep` (the running maximum;
`none` is bottom). -/
def wmJoin {K : Type} [LinearOrder K] : Option K → Option K → Option K
  | none, w => w
  | some a, none => some a
  | some a, some b => some (max a b)

/-- The running watermark of a checkpoint trace. -/
def wmRun {K : Type} [LinearOrder K] (l : List (Option K)) : Option K :=
  l.foldl wmJoin none

theorem reduceWatermarkStep_eq {K V : Type} [DecidableEq K] [DecidableEq V] [LinearOrder K]
    {ord : StrOrd} (f : V → V → V) (Pre : V → V → Prop) [DecidableRel Pre]
    (ok : FoldOkP ord .exactlyOnce Pre (reduceStep f))
    (st inp : KeyedPool K V ord .exactlyOnce × Option K) :
    reduceWatermarkStep f Pre ok st inp
      = ((st.1.append inp.1, wmJoin st.2 inp.2),
          KeyedMap.gcBelow (wmJoin st.2 inp.2) (KeyedMap.ofReduce f Pre ok (st.1.append inp.1))) := by
  obtain ⟨p, w⟩ := st
  obtain ⟨q, w'⟩ := inp
  cases w <;> cases w' <;> rfl

theorem KeyedPool.ofMultiset_zero {K V : Type} [DecidableEq K] [DecidableEq V] :
    KeyedPool.ofMultiset (0 : Multiset (K × V)) = KeyedPool.bot :=
  (KeyedPool.ext rfl rfl).symm

/-- **The register** of an unordered `reduce_watermark` after the ticks
`l`: the grouped sum of the batches and the running watermark. -/
theorem scanState_reduceWatermark_ofMultiset {K V : Type} [DecidableEq K] [DecidableEq V]
    [LinearOrder K] (f : V → V → V) (Pre : V → V → Prop) [DecidableRel Pre]
    (ok : FoldOkP .noOrder .exactlyOnce Pre (reduceStep f)) :
    ∀ (l : List (Multiset (K × V) × Option K)) (m : Multiset (K × V)) (w : Option K),
      scanAcrossTicksState (reduceWatermarkStep f Pre ok) (KeyedPool.ofMultiset m, w)
          (l.map (Prod.map KeyedPool.ofMultiset id))
        = (KeyedPool.ofMultiset (m + (l.map Prod.fst).sum), (l.map Prod.snd).foldl wmJoin w)
  | [], m, w => by
    show (KeyedPool.ofMultiset m, w) = _
    rw [List.map_nil, List.sum_nil, Multiset.add_zero, List.map_nil, List.foldl_nil]
  | (q, c) :: l, m, w => by
    show scanAcrossTicksState (reduceWatermarkStep f Pre ok)
        (reduceWatermarkStep f Pre ok (KeyedPool.ofMultiset m, w) (KeyedPool.ofMultiset q, c)).1
        (l.map (Prod.map KeyedPool.ofMultiset id)) = _
    rw [reduceWatermarkStep_eq]
    show scanAcrossTicksState (reduceWatermarkStep f Pre ok)
        ((KeyedPool.ofMultiset m).append (KeyedPool.ofMultiset q), wmJoin w c)
        (l.map (Prod.map KeyedPool.ofMultiset id)) = _
    rw [KeyedPool.ofMultiset_add, scanState_reduceWatermark_ofMultiset f Pre ok l (m + q) (wmJoin w c)]
    simp only [List.map_cons, List.sum_cons, List.foldl_cons, Multiset.add_assoc]

/-- A zip's prefix is the zip of the prefixes. -/
theorem List.take_zip' {α β : Type _} : ∀ (a : List α) (b : List β) (t : Nat),
    (a.zip b).take t = (a.take t).zip (b.take t)
  | [], _, _ => by simp
  | _ :: _, [], _ => by simp
  | _ :: _, _ :: _, 0 => rfl
  | x :: a, y :: b, t + 1 => by
    show (x, y) :: (a.zip b).take t = (x :: a.take t).zip (y :: b.take t)
    rw [List.take_zip' a b t]; rfl

/-- **Tick `t`'s published map** of an unordered `reduce_watermark` over
a zip of batch and watermark traces: the reduce of the batches consumed
through `t`, collected below the running watermark through `t`. -/
theorem reduceWatermark_getElem?_ofMultiset {K V : Type} [DecidableEq K] [DecidableEq V]
    [LinearOrder K] (f : V → V → V) (Pre : V → V → Prop) [DecidableRel Pre]
    (ok : FoldOkP .noOrder .exactlyOnce Pre (reduceStep f))
    (qs : Trace (Multiset (K × V))) (cks : Trace (Option K)) (t : Nat) :
    (scanAcrossTicksTrace (reduceWatermarkStep f Pre ok) (KeyedPool.bot, none)
        (Trace.zip (qs.map KeyedPool.ofMultiset) cks))[t]?
      = ((Trace.zip qs cks)[t]?).map (fun _ =>
          KeyedMap.gcBelow (wmRun (cks.take (t + 1)))
            (KeyedMap.ofReduce f Pre ok (KeyedPool.ofMultiset ((qs.take (t + 1)).sum)))) := by
  have hzip : Trace.zip (qs.map KeyedPool.ofMultiset) cks
      = (Trace.zip qs cks).map (Prod.map KeyedPool.ofMultiset id) := List.zip_map_left
  rw [hzip, scanAcrossTicksTrace_getElem?, List.getElem?_map, Option.map_map, ← List.map_take,
    ← KeyedPool.ofMultiset_zero, scanState_reduceWatermark_ofMultiset, Multiset.zero_add]
  cases ht : (Trace.zip qs cks)[t]? with
  | none => rfl
  | some x =>
    obtain ⟨q, c⟩ := x
    obtain ⟨hq, hc⟩ := List.getElem?_zip_eq_some.mp ht
    have hqt : t < qs.length := List.getElem?_eq_some_iff.mp hq |>.1
    have hct : t < cks.length := List.getElem?_eq_some_iff.mp hc |>.1
    simp only [Option.map_some, Function.comp, Prod.map]
    rw [reduceWatermarkStep_eq]
    have h1 : ((Trace.zip qs cks).take t).map Prod.fst = qs.take t := by
      rw [show Trace.zip qs cks = qs.zip cks from rfl, List.take_zip', List.map_fst_zip]
      simp only [List.length_take]; omega
    have h2 : ((Trace.zip qs cks).take t).map Prod.snd = cks.take t := by
      rw [show Trace.zip qs cks = qs.zip cks from rfl, List.take_zip', List.map_snd_zip]
      simp only [List.length_take]; omega
    rw [h1, h2, List.take_succ_eq_append_getElem hqt, List.take_succ_eq_append_getElem hct,
      List.sum_append, List.sum_singleton, wmRun, List.foldl_append, List.foldl_cons, List.foldl_nil,
      List.getElem?_eq_getElem hqt, List.getElem?_eq_getElem hct] at *
    injection hq with hq; injection hc with hc
    subst hq hc
    show some (KeyedMap.gcBelow _ (KeyedMap.ofReduce f Pre ok ((KeyedPool.ofMultiset ((qs.take t).sum)).append (KeyedPool.ofMultiset (qs[t]))))) = _
    rw [KeyedPool.ofMultiset_add]
    rfl

namespace KeyedMap
variable {K V : Type} [DecidableEq K]

theorem mem_entries_iff (m : KeyedMap K V) (k : K) (v : V) :
    (k, v) ∈ m.entries ↔ m.get k = some v := by
  unfold entries
  rw [Multiset.mem_filterMap]
  constructor
  · rintro ⟨k', -, h⟩
    rw [Option.map_eq_some_iff] at h
    obtain ⟨v', hv', heq⟩ := h
    obtain ⟨rfl, rfl⟩ := Prod.mk.inj heq
    exact hv'
  · intro h
    refine ⟨k, ?_, by rw [h]; rfl⟩
    exact (m.dom k).mp (by rw [h]; rfl)

theorem get_gcBelow_none [LinearOrder K] (m : KeyedMap K V) (k : K) :
    (m.gcBelow none).get k = m.get k := rfl

theorem get_gcBelow_some [LinearOrder K] (w : K) (m : KeyedMap K V) (k : K) :
    (m.gcBelow (some w)).get k = if w ≤ k then m.get k else none := rfl

theorem get_ofReduce_ofMultiset [DecidableEq V] (f : V → V → V) (Pre : V → V → Prop)
    [DecidableRel Pre] (ok : FoldOkP .noOrder .exactlyOnce Pre (reduceStep f))
    (E : Multiset (K × V)) (k : K) :
    (ofReduce f Pre ok (KeyedPool.ofMultiset E)).get k
      = if k ∈ (E.map Prod.fst).toFinset then
          Multiset.foldlOn Pre (reduceStep f) none ok ((E.filter (fun e => e.1 = k)).map Prod.snd)
        else none := by
  show ((if k ∈ (E.map Prod.fst).toFinset then some (Multiset.foldlOn Pre (reduceStep f) none ok
      ((E.filter (fun e => e.1 = k)).map Prod.snd)) else none)).join = _
  by_cases h : k ∈ (E.map Prod.fst).toFinset
  · rw [if_pos h, if_pos h]; rfl
  · rw [if_neg h, if_neg h]; rfl

/-- Two entries of a map at one key are the same entry (the structural
fact behind Rust's `commutative = manual_proof!(/** no overlapping keys */)`
on a `.entries().fold(HashMap::new, insert)`). -/
theorem entries_pairwise_key [DecidableEq V] (m : KeyedMap K V) :
    PoolPairwise .noOrder .exactlyOnce (fun x y : K × V => x.1 = y.1 → x = y) m.entries := by
  intro x hx y hy hk
  obtain ⟨kx, vx⟩ := x
  obtain ⟨ky, vy⟩ := y
  rw [mem_entries_iff] at hx hy
  cases hk
  rw [hx] at hy
  cases hy
  rfl

/-- **Reading a collected reduce at a key**: a value sits at `k` iff `k`
is at or above the watermark, has content, and the key's fold answers
it. -/
theorem get_gcBelow_ofReduce_ofMultiset [LinearOrder K] [DecidableEq V] (f : V → V → V)
    (Pre : V → V → Prop) [DecidableRel Pre] (ok : FoldOkP .noOrder .exactlyOnce Pre (reduceStep f))
    (c : Option K) (E : Multiset (K × V)) (k : K) (v : V) :
    (gcBelow c (ofReduce f Pre ok (KeyedPool.ofMultiset E))).get k = some v ↔
      (∀ w, c = some w → w ≤ k) ∧ k ∈ (E.map Prod.fst).toFinset
      ∧ Multiset.foldlOn Pre (reduceStep f) none ok ((E.filter (fun e => e.1 = k)).map Prod.snd)
          = some v := by
  have hred := KeyedMap.get_ofReduce_ofMultiset f Pre ok E k
  have hbody : (ofReduce f Pre ok (KeyedPool.ofMultiset E)).get k = some v ↔
      k ∈ (E.map Prod.fst).toFinset
      ∧ Multiset.foldlOn Pre (reduceStep f) none ok ((E.filter (fun e => e.1 = k)).map Prod.snd)
          = some v := by
    rw [hred]
    by_cases hk : k ∈ (E.map Prod.fst).toFinset
    · rw [if_pos hk]
      exact ⟨fun h => ⟨hk, h⟩, fun h => h.2⟩
    · rw [if_neg hk]
      exact ⟨fun h => (nomatch h), fun h => absurd h.1 hk⟩
  cases c with
  | none =>
    rw [get_gcBelow_none, hbody]
    exact ⟨fun h => ⟨fun w hw => (nomatch hw), h.1, h.2⟩, fun h => ⟨h.2.1, h.2.2⟩⟩
  | some w =>
    rw [get_gcBelow_some]
    by_cases hw : w ≤ k
    · rw [if_pos hw, hbody]
      exact ⟨fun h => ⟨fun w' hw' => (Option.some.inj hw') ▸ hw, h.1, h.2⟩, fun h => ⟨h.2.1, h.2.2⟩⟩
    · rw [if_neg hw]
      exact ⟨fun h => (nomatch h), fun h => absurd (h.1 w rfl) hw⟩

end KeyedMap

/-- The running watermark of an ascending checkpoint trace is its latest
read (`Monotonic` checkpoints: GC never runs ahead of the published
checkpoint). -/
theorem wmRun_take_of_ascending {l : Trace (Option Nat)} (hl : Ascending ValueOrder.optNat l) :
    ∀ (t : Nat) (ht : t < l.length), wmRun (l.take (t + 1)) = l[t] := by
  intro t ht
  induction t with
  | zero =>
    rw [List.take_succ_eq_append_getElem ht, List.take_zero, List.nil_append]
    cases l[0] <;> rfl
  | succ n ih =>
    rw [List.take_succ_eq_append_getElem ht, wmRun, List.foldl_append, ← wmRun,
      ih (Nat.lt_of_succ_lt ht), List.foldl_cons, List.foldl_nil]
    have hasc : ValueOrder.optNat.le (l[n]'(Nat.lt_of_succ_lt ht)) (l[n + 1]) :=
      hl (Nat.le_succ n) ht
    generalize l[n]'(Nat.lt_of_succ_lt ht) = a at hasc
    generalize l[n + 1] = b at hasc
    cases a <;> cases b <;> simp_all [ValueOrder.optNatLE, wmJoin]

/-- Prefix sums of a trace of multisets grow with the prefix. -/
theorem List.sum_take_le_sum_take {α : Type _} (l : List (Multiset α)) {n n' : Nat} (h : n ≤ n') :
    (l.take n).sum ≤ (l.take n').sum := by
  have : l.take n' = l.take n ++ ((l.take n').drop n) := by
    conv_lhs => rw [← List.take_append_drop n (l.take n')]
    rw [List.take_take, Nat.min_eq_left h]
  rw [this, List.sum_append]
  exact Multiset.le_add_right _ _

theorem List.sum_take_le_sum {α : Type _} (l : List (Multiset α)) (n : Nat) :
    (l.take n).sum ≤ l.sum := by
  conv_rhs => rw [← List.take_append_drop n l]
  rw [List.sum_append]
  exact Multiset.le_add_right _ _


/-- **No overlapping keys**: two values of a keyed batch at one key are
the same value (Rust's `commutative = manual_proof!(/** inserting
elements into map is commutative because no keys will overlap */)` on
an `.entries().fold(HashMap::new, insert)` — structural for the entries
of a map, `KeyedMap.entries_pairwise_key`). -/
def NoKeyOverlap {K V : Type} (x y : K × V) : Prop := x.1 = y.1 → x = y

instance {K V : Type} [DecidableEq K] [DecidableEq V] : DecidableRel (NoKeyOverlap (K := K) (V := V)) :=
  fun x y => inferInstanceAs (Decidable (x.1 = y.1 → x = y))

/-- Rust's `HashMap::insert` on the canonical sorted association list
(the Lean image of a `HashMap`: keys ascending, one value per key). -/
def insertSorted {K V : Type} [LinearOrder K] : List (K × V) → K × V → List (K × V)
  | [], e => [e]
  | x :: l, e => if e.1 ≤ x.1 then e :: x :: l else x :: insertSorted l e

theorem mem_insertSorted {K V : Type} [LinearOrder K] (l : List (K × V)) (e y : K × V) :
    y ∈ insertSorted l e ↔ y = e ∨ y ∈ l := by
  induction l with
  | nil => simp [insertSorted]
  | cons x l ih =>
    unfold insertSorted
    split_ifs
    · simp
    · rw [List.mem_cons, ih, List.mem_cons]
      tauto

/-- Inserting two values at distinct keys commutes (in any order: the
fold's obligation is over every accumulator). -/
theorem insertSorted_comm {K V : Type} [LinearOrder K] (l : List (K × V)) (a b : K × V)
    (hab : a.1 ≠ b.1) :
    insertSorted (insertSorted l a) b = insertSorted (insertSorted l b) a := by
  induction l with
  | nil =>
    simp only [insertSorted]
    rcases lt_or_gt_of_ne hab with h | h
    · rw [if_neg (not_le.mpr h), if_pos (le_of_lt h)]
    · rw [if_pos (le_of_lt h), if_neg (not_le.mpr h)]
  | cons x l ih =>
    by_cases ha : a.1 ≤ x.1 <;> by_cases hb : b.1 ≤ x.1
    · rcases lt_or_gt_of_ne hab with h | h
      · simp [insertSorted, ha, hb, le_of_lt h, not_le.mpr h]
      · simp [insertSorted, ha, hb, le_of_lt h, not_le.mpr h]
    · have h : ¬ b.1 ≤ a.1 := fun h' => hb (le_trans h' ha)
      simp [insertSorted, ha, hb, h]
    · have h : ¬ a.1 ≤ b.1 := fun h' => ha (le_trans h' hb)
      simp [insertSorted, ha, hb, h]
    · simp [insertSorted, ha, hb, ih]

/-- The fold obligation of the insert fold under `NoKeyOverlap`. -/
theorem insertSorted_foldOk {K V : Type} [LinearOrder K] [DecidableEq V] :
    FoldOkP .noOrder .exactlyOnce (NoKeyOverlap (K := K) (V := V)) insertSorted := by
  intro s x y hxy
  by_cases h : x.1 = y.1
  · rw [hxy h]
  · exact insertSorted_comm s x y h

/-- The insert fold's content is the batch's (every pairwise-coherent
multiset: one list representative). -/
theorem mem_foldl_insertSorted {K V : Type} [LinearOrder K] (l : List (K × V)) (acc : List (K × V))
    (y : K × V) : y ∈ l.foldl insertSorted acc ↔ y ∈ acc ∨ y ∈ l := by
  induction l generalizing acc with
  | nil => simp
  | cons e l ih =>
    rw [List.foldl_cons, ih, mem_insertSorted, List.mem_cons]
    tauto

theorem mem_foldlOn_insertSorted {K V : Type} [LinearOrder K] [DecidableEq K] [DecidableEq V]
    (m : Multiset (K × V)) (hm : PairwiseM NoKeyOverlap m) (acc : List (K × V)) (y : K × V) :
    y ∈ Multiset.foldlOn NoKeyOverlap insertSorted acc insertSorted_foldOk m ↔ y ∈ acc ∨ y ∈ m := by
  induction m using Multiset.induction_on generalizing acc with
  | empty => simp [Multiset.foldlOn_zero]
  | cons a m ih =>
    rw [Multiset.foldlOn_cons _ _ _ _ hm, ih (PairwiseM.cons hm), mem_insertSorted, Multiset.mem_cons]
    tauto


/-! ## D81 — the bench client's tick API: coupling lemmas

The machine's lists vs the denotation's keyed pools/maps, one lemma per
new in-tick operator (`Couple.lean` consumes them). -/

namespace KeyedPool
variable {K V W : Type} [DecidableEq K] [DecidableEq V] [DecidableEq W]

theorem keyVals_map_snd (k : K) (f : V → W) (l : List (K × V)) :
    keyVals k (l.map (fun e => (e.1, f e.2))) = (keyVals k l).map f := by
  unfold keyVals
  rw [List.filter_map, List.map_map, List.map_map]
  have hp : ((fun e : K × W => decide (e.1 = k)) ∘ (fun e : K × V => (e.1, f e.2)))
      = fun e => decide (e.1 = k) := funext fun _ => rfl
  rw [hp]
  rfl

theorem ofList_map (f : V → W) (l : List (K × V)) :
    ofList (l.map (fun e => (e.1, f e.2))) = (ofList l).map f := by
  refine KeyedPool.ext ?_ (funext fun k => ?_)
  · show ((l.map (fun e => (e.1, f e.2))).map Prod.fst).toFinset = (l.map Prod.fst).toFinset
    rw [List.map_map]; rfl
  · exact keyVals_map_snd k f l

theorem ofMultiset_map (f : V → W) (m : Multiset (K × V)) :
    ofMultiset (m.map (fun e => (e.1, f e.2))) = (ofMultiset m).map f := by
  refine KeyedPool.ext ?_ (funext fun k => ?_)
  · show ((m.map (fun e => (e.1, f e.2))).map Prod.fst).toFinset = (m.map Prod.fst).toFinset
    rw [Multiset.map_map]; rfl
  · show ((m.map (fun e => (e.1, f e.2))).filter (fun e => e.1 = k)).map Prod.snd
      = ((m.filter (fun e => e.1 = k)).map Prod.snd).map f
    rw [Multiset.map_map, Multiset.filter_map, Multiset.map_map]
    rfl

theorem ofList_append (a b : List (K × V)) :
    ofList (a ++ b) = (ofList a).append (ofList b) := by
  refine KeyedPool.ext ?_ (funext fun k => ?_)
  · show ((a ++ b).map Prod.fst).toFinset = (a.map Prod.fst).toFinset ∪ (b.map Prod.fst).toFinset
    rw [List.map_append, List.toFinset_append]
  · exact keyVals_append k a b

theorem weakenOrder_ofList (l : List (K × V)) :
    (ofList l).weakenOrder = ofMultiset (↑l) := by
  refine KeyedPool.ext rfl (funext fun k => ?_)
  show (↑(keyVals k l) : Multiset V) = ((↑l : Multiset (K × V)).filter (fun e => e.1 = k)).map Prod.snd
  unfold keyVals
  rw [Multiset.filter_coe, Multiset.map_coe]

theorem weakenOrder_noOrder (p : KeyedPool K V .noOrder .exactlyOnce) :
    p.weakenOrder = p :=
  KeyedPool.ext rfl rfl

end KeyedPool

/-! `CoupledKeyed` transported through the new keyed in-tick operators. -/
namespace CoupledKeyed
variable {K V W : Type} [DecidableEq K] [DecidableEq V] [DecidableEq W]

theorem map {ord : StrOrd} {l : List (K × V)} {p : KeyedPool K V ord .exactlyOnce}
    (h : CoupledKeyed ord .exactlyOnce l p) (f : V → W) :
    CoupledKeyed ord .exactlyOnce (l.map (fun e => (e.1, f e.2))) (p.map f) := by
  cases ord
  · show KeyedPool.ofList _ = _
    rw [KeyedPool.ofList_map, show KeyedPool.ofList l = p from h]
  · show KeyedPool.ofMultiset _ = _
    rw [← Multiset.map_coe, KeyedPool.ofMultiset_map, show KeyedPool.ofMultiset ↑l = p from h]

theorem chain {ord : StrOrd} {a b : List (K × V)} {p q : KeyedPool K V ord .exactlyOnce}
    (ha : CoupledKeyed ord .exactlyOnce a p) (hb : CoupledKeyed ord .exactlyOnce b q) :
    CoupledKeyed ord .exactlyOnce (a ++ b) (p.append q) := by
  cases ord
  · show KeyedPool.ofList _ = _
    rw [KeyedPool.ofList_append, show KeyedPool.ofList a = p from ha,
      show KeyedPool.ofList b = q from hb]
  · show KeyedPool.ofMultiset _ = _
    rw [← Multiset.coe_add, ← KeyedPool.ofMultiset_add, show KeyedPool.ofMultiset ↑a = p from ha,
      show KeyedPool.ofMultiset ↑b = q from hb]

theorem weakenOrder {ord : StrOrd} {l : List (K × V)} {p : KeyedPool K V ord .exactlyOnce}
    (h : CoupledKeyed ord .exactlyOnce l p) :
    CoupledKeyed .noOrder .exactlyOnce l p.weakenOrder := by
  cases ord
  · show KeyedPool.ofMultiset _ = _
    rw [← KeyedPool.weakenOrder_ofList, show KeyedPool.ofList l = p from h]
  · show KeyedPool.ofMultiset _ = _
    rw [KeyedPool.weakenOrder_noOrder]; exact h

end CoupledKeyed

namespace KeyedMap
variable {K V W : Type} [DecidableEq K]

/-- `join_keyed_singleton` on the association lists reads as the maps'
join. -/
theorem ofAssoc_join (a : SchedKMap K V) (b : SchedKMap K W) :
    ofAssoc (a.join b) = (ofAssoc a).join (ofAssoc b) := by
  unfold SchedKMap.join
  rw [ofAssoc_ofKeyed]
  refine KeyedMap.ext ?_ (funext fun k => ?_)
  · ext k
    rw [Finset.mem_filter, List.mem_toFinset]
    show k ∈ a.val.map Prod.fst ∧ _ ↔ k ∈ (ofAssoc a).keys ∩ (ofAssoc b).keys
    rw [Finset.mem_inter, ← (ofAssoc a).dom k, ← (ofAssoc b).dom k]
    show k ∈ a.val.map Prod.fst ∧ ((a.lookup k).bind (fun v => (b.lookup k).map (fun w => (v, w)))).isSome
      ↔ (a.lookup k).isSome ∧ (b.lookup k).isSome
    rw [← SchedKMap.lookup_isSome_iff]
    cases a.lookup k <;> cases b.lookup k <;> simp
  · show (if k ∈ a.keys then (a.lookup k).bind (fun v => (b.lookup k).map (fun w => (v, w))) else none)
      = (a.lookup k).bind (fun v => (b.lookup k).map (fun w => (v, w)))
    by_cases hm : k ∈ a.keys
    · rw [if_pos hm]
    · rw [if_neg hm, (SchedKMap.lookup_eq_none_iff a k).mpr hm]
      rfl

/-- `filter_key_not_in` on the association list reads as the map's. -/
theorem ofAssoc_filterKeyNotIn (m : SchedKMap K V) (ks : List K) :
    ofAssoc (m.filterEntries (fun e => !(decide (e.1 ∈ ks))))
      = (ofAssoc m).filterKeyNotIn (↑ks : Multiset K) := by
  rw [ofAssoc_filterEntries_key m (fun k => !(decide (k ∈ ks)))]
  refine KeyedMap.ext ?_ (funext fun k => ?_)
  · ext k
    simp [filterKeyNotIn]
  · show (if (!(decide (k ∈ ks))) = true then (ofAssoc m).get k else none)
      = (if k ∈ (↑ks : Multiset K) then none else (ofAssoc m).get k)
    by_cases h : k ∈ ks <;> simp [h]

/-- `key_count` on the association list is the map's key count. -/
theorem keyCount_ofAssoc (m : SchedKMap K V) : (ofAssoc m).keyCount = m.keyCount := by
  show (m.val.map Prod.fst).toFinset.card = m.val.length
  rw [List.toFinset_card_of_nodup m.property, List.length_map]

/-- For distinct keys, a key's values are its lookup, listed. -/
theorem keyVals_eq_lookup_toList (m : SchedKMap K V) (k : K) :
    keyVals k m.val = (m.lookup k).toList := by
  obtain ⟨l, hl⟩ := m
  show keyVals k l = ((l.find? (fun e => decide (e.1 = k))).map Prod.snd).toList
  induction l with
  | nil => rfl
  | cons e l ih =>
    have hl' : (l.map Prod.fst).Nodup := (List.nodup_cons.mp hl).2
    have hnot : e.1 ∉ l.map Prod.fst := (List.nodup_cons.mp hl).1
    unfold keyVals
    rw [List.filter_cons, List.find?_cons]
    by_cases h : e.1 = k
    · have hd : decide (e.1 = k) = true := decide_eq_true h
      simp only [hd]
      have : (l.filter (fun e => decide (e.1 = k))) = [] := by
        rw [List.filter_eq_nil_iff]
        intro x hx hxk
        exact hnot (List.mem_map.mpr ⟨x, hx, (of_decide_eq_true hxk).trans h.symm⟩)
      rw [this]; rfl
    · have hd : decide (e.1 = k) = false := decide_eq_false h
      simp only [hd]
      exact ih hl'

/-- `into_keyed_stream` on the association list groups as the map's. -/
theorem ofList_entries [DecidableEq V] (m : SchedKMap K V) :
    KeyedPool.ofList m.entries = KeyedPool.ofMap (ofAssoc m) :=
  KeyedPool.ext rfl (funext fun k => keyVals_eq_lookup_toList m k)

end KeyedMap

end Hydro
