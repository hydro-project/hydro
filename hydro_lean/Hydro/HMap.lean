import Mathlib.Data.Multiset.Sort
import Mathlib.Data.List.Sort
import Std.Data.ExtHashMap
import Hydro.Grades
import Hydro.Keyed
import Hydro.Wire

/-!
# `HMap` — Rust's `HashMap<K, V>` as a value (D75)

A Rust `HashMap` that travels inside a message or sits in a singleton (the acceptor's
log snapshot, `paxos.rs:890–897`) is a *value*: it needs decidable equality and
proof-friendly extensionality. `Std.ExtHashMap` is exactly that — the hash map
quotiented by contents (equal iff `∀ k, m₁[k]? = m₂[k]?`), a real hash map at runtime,
`insert` O(1). This file wraps it with the vocabulary proofs read a map in: membership of
`(k, v)` pairs (`Std.ExtHashMap`'s own `∈` is on keys), the entries as a `Multiset`, the
insert fold's content under `NoKeyOverlap`, and — where an order is forced on it anyway,
the wire (`Wire`) and `Repr` — the canonical listing by key (unique keys make the sort
a function of the quotient; needs `[LinearOrder K]`).

It is the shared type: `LogMap P nP := HMap Nat (LogValue P nP)` (`Paxos/Types.lean`).
Before D75 the log was a sorted association list built with `insertSorted` (O(n) per
insert, O(n²) per tick); the Rust is O(n) per tick.
-/

namespace Hydro

open Std (ExtHashMap ExtDHashMap DHashMap)

set_option linter.unusedSectionVars false

/-- Rust `HashMap<K, V>` carried as a value: the extensional hash map. -/
structure HMap (K V : Type) [BEq K] [Hashable K] where
  map : Std.ExtHashMap K V

namespace HMap

variable {K V : Type} [DecidableEq K] [Hashable K]

instance [DecidableEq V] : DecidableEq (HMap K V) := fun a b =>
  decidable_of_iff (a.map = b.map) ⟨fun h => by cases a; cases b; cases h; rfl, fun h => h ▸ rfl⟩

/-- `HashMap::new()`. -/
def empty : HMap K V := ⟨∅⟩

/-- `map.insert(k, v)`. -/
def insert (m : HMap K V) (e : K × V) : HMap K V := ⟨m.map.insert e.1 e.2⟩

/-- `map.get(k)`. -/
def get? (m : HMap K V) (k : K) : Option V := m.map[k]?

/-- `(k, v) ∈ m`: the map holds `v` at `k`. -/
instance : Membership (K × V) (HMap K V) := ⟨fun m e => m.map[e.1]? = some e.2⟩

theorem mem_def {m : HMap K V} {e : K × V} : e ∈ m ↔ m.map[e.1]? = some e.2 := Iff.rfl

instance [DecidableEq V] (m : HMap K V) (e : K × V) : Decidable (e ∈ m) :=
  decidable_of_iff (m.map[e.1]? = some e.2) mem_def.symm

@[simp] theorem not_mem_empty (e : K × V) : e ∉ (empty : HMap K V) := by
  simp [mem_def, empty]

theorem mem_insert {m : HMap K V} {e y : K × V} :
    y ∈ m.insert e ↔ y = e ∨ (y ∈ m ∧ y.1 ≠ e.1) := by
  simp only [mem_def, insert, ExtHashMap.getElem?_insert, beq_iff_eq]
  by_cases h : e.1 = y.1
  · rw [if_pos h]
    simp only [Option.some.injEq, ne_eq, h.symm, not_true_eq_false, and_false, or_false]
    constructor
    · intro hv; exact Prod.ext h.symm hv.symm
    · intro hy; rw [hy]
  · simp only [if_neg h, ne_eq]
    constructor
    · intro hy; exact Or.inr ⟨hy, fun h' => h h'.symm⟩
    · rintro (rfl | ⟨hy, _⟩)
      · exact absurd rfl h
      · exact hy

/-- Keys are unique: two entries at one key are the same entry. -/
theorem eq_of_mem_of_key {m : HMap K V} {y z : K × V} (hy : y ∈ m) (hz : z ∈ m) (hk : y.1 = z.1) :
    y = z := by
  rw [mem_def] at hy hz
  rw [hk] at hy
  exact Prod.ext hk (Option.some.inj (hy.symm.trans hz))

theorem ext' {m₁ m₂ : HMap K V} (h : ∀ k : K, (m₁.map[k]? : Option V) = m₂.map[k]?) : m₁ = m₂ := by
  cases m₁; cases m₂
  simp only [HMap.mk.injEq]
  exact ExtHashMap.ext_getElem? h

/-- Inserting at distinct keys commutes; at one key with one value, trivially. -/
theorem insert_foldOk : FoldOkP .noOrder .exactlyOnce (NoKeyOverlap (K := K) (V := V)) insert := by
  intro s x y hxy
  by_cases h : x.1 = y.1
  · rw [hxy h]
  · apply ext'
    intro k
    simp only [insert, ExtHashMap.getElem?_insert, beq_iff_eq]
    by_cases hy : y.1 = k <;> by_cases hx : x.1 = k <;> simp_all

/-- The insert fold's content is the batch's (every pairwise-coherent multiset: one
map), given the seed is coherent with the batch. -/
theorem mem_foldlOn_insert [DecidableEq V] (m : Multiset (K × V)) (hm : PairwiseM NoKeyOverlap m)
    (acc : HMap K V) (hacc : ∀ y ∈ acc, ∀ z ∈ m, y.1 = z.1 → y = z) (y : K × V) :
    y ∈ Multiset.foldlOn NoKeyOverlap insert acc insert_foldOk m ↔ y ∈ acc ∨ y ∈ m := by
  induction m using Multiset.induction_on generalizing acc with
  | empty => simp [Multiset.foldlOn_zero]
  | cons a m ih =>
    rw [Multiset.foldlOn_cons _ _ _ _ hm]
    have hacc' : ∀ y ∈ acc.insert a, ∀ z ∈ m, y.1 = z.1 → y = z := by
      intro y hy z hz hk
      rcases mem_insert.mp hy with rfl | ⟨hy, _⟩
      · exact hm _ (Multiset.mem_cons_self _ _) _ (Multiset.mem_cons_of_mem hz) hk
      · exact hacc y hy z (Multiset.mem_cons_of_mem hz) hk
    rw [ih (PairwiseM.cons hm) _ hacc', mem_insert, Multiset.mem_cons]
    constructor
    · rintro ((rfl | ⟨hy, _⟩) | hy)
      · exact Or.inr (Or.inl rfl)
      · exact Or.inl hy
      · exact Or.inr (Or.inr hy)
    · rintro (hy | rfl | hy)
      · by_cases hk : y.1 = a.1
        · exact Or.inl (Or.inl (hacc y hy a (Multiset.mem_cons_self _ _) hk))
        · exact Or.inl (Or.inr ⟨hy, hk⟩)
      · exact Or.inl (Or.inl rfl)
      · exact Or.inr hy

theorem mem_foldlOn_insert_empty [DecidableEq V] (m : Multiset (K × V)) (hm : PairwiseM NoKeyOverlap m)
    (y : K × V) :
    y ∈ Multiset.foldlOn NoKeyOverlap insert empty insert_foldOk m ↔ y ∈ m := by
  rw [mem_foldlOn_insert m hm empty (fun y hy => absurd hy (not_mem_empty y))]
  simp

/-! ## The entries, as a multiset and as the canonical list -/

/-- The `(slot, entry)` pairs, unordered (`HashMap::iter()`): a function of the
quotient because any two representatives list a permutation. -/
def entries (m : HMap K V) : Multiset (K × V) :=
  m.map.inner.lift (fun h => (↑(DHashMap.Const.toList h) : Multiset (K × V)))
    (fun _ _ hab => Multiset.coe_eq_coe.mpr (DHashMap.Equiv.constToList_perm hab))

theorem entries_mk (h : DHashMap K (fun _ => V)) :
    entries ⟨⟨ExtDHashMap.mk h⟩⟩ = (↑(DHashMap.Const.toList h) : Multiset (K × V)) := rfl

theorem getElem?_mk (h : DHashMap K (fun _ => V)) (k : K) :
    (⟨ExtDHashMap.mk h⟩ : ExtHashMap K V)[k]? = DHashMap.Const.get? h k := rfl

theorem mem_entries {m : HMap K V} {e : K × V} : e ∈ m.entries ↔ e ∈ m := by
  obtain ⟨⟨q⟩⟩ := m
  induction q using ExtDHashMap.inductionOn with
  | mk h =>
    rw [entries_mk, mem_def, getElem?_mk, Multiset.mem_coe]
    exact DHashMap.Const.mem_toList_iff_get?_eq_some

/-- The entries are key-coherent (the fold obligation's hypothesis). -/
theorem entries_pairwise_key (m : HMap K V) : PairwiseM NoKeyOverlap m.entries :=
  fun _ hy _ hz hk => eq_of_mem_of_key (mem_entries.mp hy) (mem_entries.mp hz) hk

section Ordered

variable [LinearOrder K]

/-- Key order on pairs, as the sort's comparator. -/
def leKey (a b : K × V) : Bool := decide (a.1 ≤ b.1)

/-- The keys of a representative's listing are distinct. -/
theorem toList_keys_nodup (h : DHashMap K (fun _ => V)) :
    ((DHashMap.Const.toList h).map Prod.fst).Nodup := by
  rw [List.nodup_iff_pairwise_ne, List.pairwise_map]
  exact (DHashMap.Const.distinct_keys_toList (m := h)).imp fun hne => by
    simpa [beq_iff_eq] using hne

/-- A sorted listing with distinct keys is unique among permutations. -/
theorem sorted_unique {l₁ l₂ : List (K × V)} (hp : l₁.Perm l₂)
    (hn : (l₁.map Prod.fst).Nodup)
    (h₁ : l₁.Pairwise (fun a b => leKey a b = true)) (h₂ : l₂.Pairwise (fun a b => leKey a b = true)) :
    l₁ = l₂ := by
  have hn₂ : (l₂.map Prod.fst).Nodup := (hp.map Prod.fst).nodup_iff.mp hn
  have key : ∀ {l : List (K × V)}, (l.map Prod.fst).Nodup →
      l.Pairwise (fun a b => leKey a b = true) → l.Pairwise (fun a b : K × V => a.1 < b.1) := by
    intro l hn hs
    rw [List.nodup_iff_pairwise_ne, List.pairwise_map] at hn
    exact (hs.and hn).imp fun ⟨hle, hne⟩ => _root_.lt_of_le_of_ne (by simpa [leKey] using hle) hne
  exact List.Perm.eq_of_pairwise (fun _ _ _ _ h₁ h₂ => absurd (_root_.lt_trans h₁ h₂) (_root_.lt_irrefl _))
    (key hn h₁) (key hn₂ h₂) hp

/-- The canonical listing: keys ascending (`HashMap::iter()` sorted — the order every
representative agrees on). -/
def toList (m : HMap K V) : List (K × V) :=
  m.map.inner.lift (fun h => (DHashMap.Const.toList h).mergeSort leKey)
    (fun a b hab => by
      have hperm : ((DHashMap.Const.toList a).mergeSort leKey).Perm
          ((DHashMap.Const.toList b).mergeSort leKey) :=
        (List.mergeSort_perm _ _).trans ((DHashMap.Equiv.constToList_perm hab).trans
          (List.mergeSort_perm _ _).symm)
      have hn : (((DHashMap.Const.toList a).mergeSort leKey).map Prod.fst).Nodup :=
        ((List.mergeSort_perm _ _).map Prod.fst).nodup_iff.mpr (toList_keys_nodup a)
      have htrans : ∀ a b c : K × V, leKey a b → leKey b c → leKey a c := by
        intro a b c hab hbc; simp only [leKey, decide_eq_true_eq] at *; exact le_trans hab hbc
      have htotal : ∀ a b : K × V, (leKey a b || leKey b a) = true := by
        intro a b; simp only [leKey, Bool.or_eq_true, decide_eq_true_eq]; exact le_total _ _
      exact sorted_unique hperm hn (List.pairwise_mergeSort htrans htotal _) (List.pairwise_mergeSort htrans htotal _))

theorem toList_mk (h : DHashMap K (fun _ => V)) :
    toList ⟨⟨ExtDHashMap.mk h⟩⟩ = (DHashMap.Const.toList h).mergeSort leKey := rfl

/-- The listing's content is the entries. -/
theorem coe_toList (m : HMap K V) : (↑m.toList : Multiset (K × V)) = m.entries := by
  obtain ⟨⟨q⟩⟩ := m
  induction q using ExtDHashMap.inductionOn with
  | mk h =>
    rw [toList_mk, entries_mk]
    exact Multiset.coe_eq_coe.mpr (List.mergeSort_perm _ _)

theorem mem_toList {m : HMap K V} {e : K × V} : e ∈ m.toList ↔ e ∈ m := by
  rw [← Multiset.mem_coe, coe_toList, mem_entries]

/-- The listing's keys are distinct. -/
theorem toList_keys_nodup' (m : HMap K V) : (m.toList.map Prod.fst).Nodup := by
  obtain ⟨⟨q⟩⟩ := m
  induction q using ExtDHashMap.inductionOn with
  | mk h =>
    rw [toList_mk]
    exact ((List.mergeSort_perm _ _).map Prod.fst).nodup_iff.mpr (toList_keys_nodup h)

instance [Repr K] [Repr V] : Repr (HMap K V) := ⟨fun m n => reprPrec m.toList n⟩

end Ordered

/-- A map from a list (tests and decoding): later pairs win. -/
def ofList (l : List (K × V)) : HMap K V := ⟨ExtHashMap.ofList l⟩

/-- A map is its listing's map. -/
theorem ofList_toList [LinearOrder K] (m : HMap K V) : ofList m.toList = m := by
  apply ext'
  intro k
  show (ExtHashMap.ofList m.toList)[k]? = m.map[k]?
  have hdistinct : m.toList.Pairwise (fun a b => (a.1 == b.1) = false) := by
    have := toList_keys_nodup' m
    rw [List.nodup_iff_pairwise_ne, List.pairwise_map] at this
    exact this.imp fun hne => by simpa [beq_iff_eq] using hne
  cases hk : m.map[k]? with
  | none =>
    apply ExtHashMap.getElem?_ofList_of_contains_eq_false
    rw [Bool.eq_false_iff, Ne, List.contains_iff_mem]
    intro hmem
    obtain ⟨e, he, hek⟩ := List.mem_map.mp hmem
    have := (mem_toList.mp he)
    rw [mem_def, hek] at this
    rw [this] at hk
    cases hk
  | some v =>
    exact ExtHashMap.getElem?_ofList_of_mem (beq_self_eq_true k) hdistinct
      (mem_toList.mpr (show (k, v) ∈ m from hk))

instance [LinearOrder K] [Wire K] [Wire V] : Wire (HMap K V) := Wire.ofEquiv toList ofList ofList_toList

end HMap

end Hydro
