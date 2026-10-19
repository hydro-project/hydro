import Std.Data.HashMap
import Std.Data.HashSet
import Hydro.Keyed

/-!
# Hash-backed in-tick keyed operators (D75)

Rust's `fold_keyed`, `join`, `anti_join` and `filter_not_in` build a `HashMap`/
`HashSet` inside the tick and demand `K: Hash + Eq` *at the operator* (the keyed
collection types themselves carry no bound). The machine (`SchedSem`) mirrors them on
association lists — `keyedFoldAssoc`, `listJoin`, `filter (· ∉ ·)` — which is the
right *semantics* (one list, keys by last arrival) but O(n·k) per tick, where the Rust
is O(n). This file gives each the Rust's implementation under the Rust's bound: the
`…H` wrappers carry `[Hashable K]`, the `…Impl` versions compute with `Std.HashMap`/
`Std.HashSet`, and a `@[csimp]` equation — kernel-checked — lets the compiler
substitute. The proofs stay on the lists.
-/

namespace Hydro

open Std

variable {K V W A : Type} [DecidableEq K]

/-! ## Membership tests through a hash set -/

/-- `List.dedup` (last occurrence kept, original order) through a hash set: one pass
from the right. -/
def dedupImpl [Hashable K] (l : List K) : List K :=
  (l.foldr (fun x (acc : HashSet K × List K) =>
    if acc.1.contains x then acc else (acc.1.insert x, x :: acc.2)) (∅, [])).2

theorem dedupImpl_spec [Hashable K] (l : List K) :
    let r := l.foldr (fun x (acc : HashSet K × List K) =>
      if acc.1.contains x then acc else (acc.1.insert x, x :: acc.2)) (∅, [])
    (∀ x, r.1.contains x = true ↔ x ∈ l) ∧ r.2 = l.dedup := by
  induction l with
  | nil => simp
  | cons a l ih =>
    obtain ⟨ih1, ih2⟩ := ih
    simp only [List.foldr_cons]
    by_cases h : a ∈ l
    · have hc : (l.foldr (fun x (acc : HashSet K × List K) =>
          if acc.1.contains x then acc else (acc.1.insert x, x :: acc.2)) (∅, [])).1.contains a = true :=
        (ih1 a).mpr h
      rw [if_pos hc, List.dedup_cons_of_mem h]
      refine ⟨fun x => ?_, ih2⟩
      rw [ih1, List.mem_cons]
      constructor
      · exact Or.inr
      · rintro (rfl | hx)
        · exact h
        · exact hx
    · have hc : (l.foldr (fun x (acc : HashSet K × List K) =>
          if acc.1.contains x then acc else (acc.1.insert x, x :: acc.2)) (∅, [])).1.contains a = false := by
        rw [Bool.eq_false_iff]; intro hc; exact h ((ih1 a).mp hc)
      rw [if_neg (by rw [hc]; exact Bool.false_ne_true), List.dedup_cons_of_notMem h]
      refine ⟨fun x => ?_, by rw [ih2]⟩
      simp only [HashSet.contains_insert, beq_iff_eq, List.mem_cons, Bool.or_eq_true, ih1]
      constructor
      · rintro (h | h)
        · exact Or.inl h.symm
        · exact Or.inr h
      · rintro (rfl | h)
        · exact Or.inl rfl
        · exact Or.inr h

/-- `List.dedup` with a `Hashable` in scope (the identity wrapper the compiler swaps). -/
def dedupH [Hashable K] (l : List K) : List K := l.dedup

@[csimp] theorem dedupH_eq_dedupImpl : @dedupH = @dedupImpl := by
  funext K _ _ l
  exact (dedupImpl_spec l).2.symm

/-- `.filter_not_in(other)` on lists (the machine's semantics). -/
def listFilterNotIn {α : Type} [DecidableEq α] [Hashable α] (a o : List α) : List α :=
  a.filter (fun x => !(decide (x ∈ o)))

@[simp] theorem listFilterNotIn_def {α : Type} [DecidableEq α] [Hashable α] (a o : List α) :
    listFilterNotIn a o = a.filter (fun x => !(decide (x ∈ o))) := rfl

def listFilterNotInImpl {α : Type} [DecidableEq α] [Hashable α] (a o : List α) : List α :=
  let s : HashSet α := HashSet.ofList o
  a.filter (fun x => !s.contains x)

theorem HashSet.contains_ofList_iff {α : Type} [DecidableEq α] [Hashable α] (o : List α) (x : α) :
    (HashSet.ofList o).contains x = true ↔ x ∈ o := by
  rw [HashSet.contains_ofList, List.contains_iff_mem]

@[csimp] theorem listFilterNotIn_eq_impl : @listFilterNotIn = @listFilterNotInImpl := by
  funext α _ _ a o
  unfold listFilterNotIn listFilterNotInImpl
  simp only
  apply List.filter_congr
  intro x _
  congr 1
  rw [Bool.eq_iff_iff, decide_eq_true_eq, HashSet.contains_ofList_iff]

/-- `.anti_join(keys)` on lists (the machine's semantics). -/
def listAntiJoin [Hashable K] (a : List (K × V)) (ks : List K) : List (K × V) :=
  a.filter (fun e => !(decide (e.1 ∈ ks)))

@[simp] theorem listAntiJoin_def [Hashable K] (a : List (K × V)) (ks : List K) :
    listAntiJoin a ks = a.filter (fun e => !(decide (e.1 ∈ ks))) := rfl

def listAntiJoinImpl [Hashable K] (a : List (K × V)) (ks : List K) : List (K × V) :=
  let s : HashSet K := HashSet.ofList ks
  a.filter (fun e => !s.contains e.1)

@[csimp] theorem listAntiJoin_eq_impl : @listAntiJoin = @listAntiJoinImpl := by
  funext K V _ _ a ks
  unfold listAntiJoin listAntiJoinImpl
  simp only
  apply List.filter_congr
  intro e _
  congr 1
  rw [Bool.eq_iff_iff, decide_eq_true_eq, HashSet.contains_ofList_iff]

/-! ## `.join` through a hash map of the right side -/

/-- The right side grouped by key (values reversed: consed as they arrive). -/
def groupRev [Hashable K] (b : List (K × W)) : HashMap K (List W) :=
  b.foldl (fun m f => m.alter f.1 (fun o => some (f.2 :: o.getD []))) ∅

theorem groupRev_get [Hashable K] (b : List (K × W)) (k : K) :
    ((groupRev b)[k]?).getD [] = (keyVals k b).reverse := by
  unfold groupRev
  suffices h : ∀ (m : HashMap K (List W)),
      ((b.foldl (fun m f => m.alter f.1 (fun o => some (f.2 :: o.getD []))) m)[k]?).getD []
        = (keyVals k b).reverse ++ (m[k]?).getD [] by
    simpa using h ∅
  induction b with
  | nil => intro m; simp [keyVals]
  | cons f b ih =>
    intro m
    rw [List.foldl_cons, ih]
    unfold keyVals
    simp only [List.filter_cons]
    by_cases hk : f.1 = k
    · simp only [hk, decide_true, List.map_cons, List.reverse_cons, HashMap.getElem?_alter, beq_iff_eq, if_true,
        Option.getD_some, List.append_assoc, List.singleton_append]
    · simp only [hk, decide_false, HashMap.getElem?_alter, beq_iff_eq, if_false, Bool.false_eq_true]

/-- `.join(other)` on lists (the machine's semantics). -/
def listJoinH [Hashable K] (a : List (K × V)) (b : List (K × W)) : List (K × (V × W)) :=
  listJoin a b

@[simp] theorem listJoinH_def [Hashable K] (a : List (K × V)) (b : List (K × W)) :
    listJoinH a b = listJoin a b := rfl

def listJoinImpl [Hashable K] (a : List (K × V)) (b : List (K × W)) : List (K × (V × W)) :=
  let m := groupRev b
  a.flatMap (fun e => ((m[e.1]?).getD []).reverse.map (fun w => (e.1, (e.2, w))))

@[csimp] theorem listJoinH_eq_impl : @listJoinH = @listJoinImpl := by
  funext K V W _ _ a b
  unfold listJoinH listJoinImpl listJoin
  simp only
  apply List.flatMap_congr
  intro e _
  rw [groupRev_get, List.reverse_reverse]
  unfold keyVals
  rw [List.map_map]
  rfl

/-! ## `fold_keyed` through a hash map of accumulators -/

theorem keyVals_eq_nil_of_not_mem' (k : K) (l : List (K × V)) (h : k ∉ l.map Prod.fst) :
    (l.filter (fun e => decide (e.1 = k))).map Prod.snd = [] := by
  rw [List.map_eq_nil_iff, List.filter_eq_nil_iff]
  intro e he hk
  exact h (List.mem_map.mpr ⟨e, he, of_decide_eq_true hk⟩)

/-- The accumulators: the key's values folded in arrival order. -/
def foldAcc [Hashable K] (g : A → V → A) (init : A) (l : List (K × V)) : HashMap K A :=
  l.foldl (fun m e => m.alter e.1 (fun o => some (g (o.getD init) e.2))) ∅

theorem foldAcc_get [Hashable K] (g : A → V → A) (init : A) (l : List (K × V)) (k : K) :
    (foldAcc g init l)[k]? = if k ∈ l.map Prod.fst then some ((keyVals k l).foldl g init) else none := by
  unfold foldAcc
  suffices h : ∀ (m : HashMap K A),
      (l.foldl (fun m e => m.alter e.1 (fun o => some (g (o.getD init) e.2))) m)[k]?
        = if k ∈ l.map Prod.fst then some ((keyVals k l).foldl g ((m[k]?).getD init)) else m[k]? by
    rw [h ∅]; simp
  induction l with
  | nil => intro m; simp [keyVals]
  | cons e l ih =>
    intro m
    rw [List.foldl_cons, ih]
    unfold keyVals
    simp only [List.filter_cons, List.map_cons, List.mem_cons, HashMap.getElem?_alter, beq_iff_eq]
    by_cases hk : e.1 = k
    · simp only [hk, decide_true, List.map_cons, List.foldl_cons, if_true, Option.getD_some, true_or]
      split
      · rfl
      · rename_i hnot
        rw [keyVals_eq_nil_of_not_mem' k l hnot]
        rfl
    · have hk' : ¬ k = e.1 := fun h => hk h.symm
      simp only [hk, hk', decide_false, if_false, false_or, Bool.false_eq_true]

/-- `fold_keyed` on an arrival list, the machine's semantics (`keyedFoldAssoc`) with the
Rust's bound. -/
def keyedFoldAssocH [Hashable K] (g : A → V → A) (init : A) (l : List (K × V)) : SchedKMap K A :=
  keyedFoldAssoc g init l

@[simp] theorem keyedFoldAssocH_def [Hashable K] (g : A → V → A) (init : A) (l : List (K × V)) :
    keyedFoldAssocH g init l = keyedFoldAssoc g init l := rfl

def keyedFoldAssocImpl [Hashable K] (g : A → V → A) (init : A) (l : List (K × V)) : SchedKMap K A :=
  let keys := dedupImpl (l.map Prod.fst)
  let m := foldAcc g init l
  ⟨keys.map (fun k => (k, (m[k]?).getD init)), by
    rw [List.map_map]
    have : (Prod.fst ∘ fun k => (k, (m[k]?).getD init)) = (id : K → K) := rfl
    rw [this, List.map_id]
    show (dedupImpl (l.map Prod.fst)).Nodup
    unfold dedupImpl
    rw [(dedupImpl_spec _).2]
    exact List.nodup_dedup _⟩

@[csimp] theorem keyedFoldAssocH_eq_impl : @keyedFoldAssocH = @keyedFoldAssocImpl := by
  funext K V A _ _ g init l
  unfold keyedFoldAssocH keyedFoldAssocImpl keyedFoldAssoc SchedKMap.ofKeyed
  apply Subtype.ext
  show List.filterMap _ _ = List.map _ (dedupImpl _)
  unfold dedupImpl
  rw [(dedupImpl_spec _).2]
  simp only [Option.map_some]
  rw [show (fun x : K => some (x, List.foldl g init (keyVals x l)))
      = some ∘ (fun x : K => (x, List.foldl g init (keyVals x l))) from rfl, List.filterMap_eq_map]
  apply List.map_congr_left
  intro k hk
  rw [foldAcc_get, if_pos (List.mem_dedup.mp hk)]
  rfl

/-! ## `reduce_watermark`'s per-tick payload batch through a hash map

DFIR's `ReduceKeyedWatermark` folds this tick's payloads one by one into its map
(`reduceWatermarkPayload`: drop below the watermark, else `map.entry(k)`
insert-or-reduce). On the association list each insert is a `find?` over the whole
log. The batch form below groups the kept payloads by key in a hash map (values in
arrival order), updates every existing key in one pass, and appends the new keys in
first-arrival order — the same list. -/

section ReduceWatermark

variable [LinearOrder K]

/-- First-arrival dedup past a set of already-seen keys: the elements not in
`seen`, each at its first occurrence. -/
def fdGo (seen : List K) : List K → List K
  | [] => []
  | x :: xs => if x ∈ seen then fdGo seen xs else x :: fdGo (x :: seen) xs

theorem fdGo_congr : ∀ (xs : List K) (s₁ s₂ : List K), (∀ x, x ∈ s₁ ↔ x ∈ s₂) → fdGo s₁ xs = fdGo s₂ xs
  | [], _, _, _ => rfl
  | x :: xs, s₁, s₂, h => by
    simp only [fdGo]
    by_cases hx : x ∈ s₁
    · rw [if_pos hx, if_pos ((h x).mp hx)]; exact fdGo_congr xs s₁ s₂ h
    · rw [if_neg hx, if_neg (fun h' => hx ((h x).mpr h'))]
      congr 1
      exact fdGo_congr xs _ _ (fun y => by simp only [List.mem_cons, h y])

theorem mem_fdGo : ∀ (xs seen : List K) (k : K), k ∈ fdGo seen xs ↔ k ∈ xs ∧ k ∉ seen
  | [], _, k => by simp [fdGo]
  | x :: xs, seen, k => by
    simp only [fdGo]
    by_cases hx : x ∈ seen
    · rw [if_pos hx, mem_fdGo xs seen k, List.mem_cons]
      constructor
      · rintro ⟨h1, h2⟩; exact ⟨Or.inr h1, h2⟩
      · rintro ⟨h1 | h1, h2⟩
        · exact absurd (h1 ▸ hx) h2
        · exact ⟨h1, h2⟩
    · rw [if_neg hx, List.mem_cons, mem_fdGo xs _ k, List.mem_cons, List.mem_cons]
      by_cases hk : k = x
      · subst hk; simp [hx]
      · simp [hk]

theorem fdGo_nodup : ∀ (xs seen : List K), (fdGo seen xs).Nodup
  | [], _ => List.nodup_nil
  | x :: xs, seen => by
    simp only [fdGo]
    split
    · exact fdGo_nodup xs seen
    · rw [List.nodup_cons, mem_fdGo]
      exact ⟨fun h => h.2 (List.mem_cons_self), fdGo_nodup xs _⟩

/-- `fdGo` through a hash set, one pass. -/
def fdGoImpl [Hashable K] (seen : HashSet K) (xs : List K) : List K :=
  (xs.foldl (fun (acc : HashSet K × List K) x =>
    if acc.1.contains x then acc else (acc.1.insert x, x :: acc.2)) (seen, [])).2.reverse

theorem fdGoImpl_go [Hashable K] : ∀ (xs : List K) (seen : HashSet K) (sl : List K)
    (hs : ∀ x, seen.contains x = true ↔ x ∈ sl) (acc : List K),
    (xs.foldl (fun (acc : HashSet K × List K) x =>
      if acc.1.contains x then acc else (acc.1.insert x, x :: acc.2)) (seen, acc)).2.reverse
      = acc.reverse ++ fdGo sl xs
  | [], _, _, _, acc => by simp [fdGo]
  | x :: xs, seen, sl, hs, acc => by
    rw [List.foldl_cons]
    simp only [fdGo]
    by_cases hx : x ∈ sl
    · rw [if_pos ((hs x).mpr hx), if_pos hx, fdGoImpl_go xs seen sl hs acc]
    · have hc : seen.contains x = false := by
        rw [Bool.eq_false_iff]; intro h; exact hx ((hs x).mp h)
      rw [if_neg (by rw [hc]; exact Bool.false_ne_true), if_neg hx,
        fdGoImpl_go xs (seen.insert x) (x :: sl) (fun y => by
          rw [HashSet.contains_insert, List.mem_cons, Bool.or_eq_true, beq_iff_eq, hs]
          constructor
          · rintro (h | h); exact Or.inl h.symm; exact Or.inr h
          · rintro (h | h); exact Or.inl h.symm; exact Or.inr h) (x :: acc)]
      simp

theorem fdGoImpl_eq [Hashable K] (seen : HashSet K) (sl : List K)
    (hs : ∀ x, seen.contains x = true ↔ x ∈ sl) (xs : List K) : fdGoImpl seen xs = fdGo sl xs := by
  unfold fdGoImpl
  rw [fdGoImpl_go xs seen sl hs []]
  rfl

/-- The payload fold appends the kept arrivals' new keys in first-arrival order. -/
theorem keys_foldl_payload (f : V → V → V) (w : Option K) :
    ∀ (l : List (K × V)) (m : SchedKMap K V),
      (l.foldl (reduceWatermarkPayload f w) m).val.map Prod.fst
        = m.val.map Prod.fst ++ fdGo (m.val.map Prod.fst) ((l.filter (fun kv => !belowWm w kv.1)).map Prod.fst)
  | [], m => by simp [fdGo]
  | kv :: l, m => by
    rw [List.foldl_cons, keys_foldl_payload f w l]
    unfold reduceWatermarkPayload
    by_cases hb : belowWm w kv.1 = true
    · rw [if_pos hb]
      simp only [List.filter_cons, hb, Bool.not_true, Bool.false_eq_true, if_false]
    · rw [if_neg hb]
      simp only [List.filter_cons, hb, Bool.not_false, if_true, List.map_cons, fdGo]
      by_cases hk : kv.1 ∈ m.val.map Prod.fst
      · -- present: the keys are unchanged, the key is skipped
        have hkeys : (m.insertReduce f kv).val.map Prod.fst = m.val.map Prod.fst := by
          show (SchedKMap.insertReduceList f m.val kv).map Prod.fst = _
          unfold SchedKMap.insertReduceList
          have hf : (m.val.find? (fun e => decide (e.1 = kv.1))).isSome = true := by
            obtain ⟨e, he, hek⟩ := List.mem_map.mp hk
            exact List.find?_isSome.mpr ⟨e, he, by simpa using hek⟩
          rw [if_pos hf, List.map_map]
          exact List.map_congr_left (fun e _ => by
            show (if e.1 = kv.1 then (e.1, f e.2 kv.2) else e).1 = e.1
            split <;> rfl)
        rw [hkeys, if_pos hk]
      · -- absent: appended; the new key heads the new keys
        have hkeys : (m.insertReduce f kv).val.map Prod.fst = m.val.map Prod.fst ++ [kv.1] := by
          show (SchedKMap.insertReduceList f m.val kv).map Prod.fst = _
          unfold SchedKMap.insertReduceList
          have hf : (m.val.find? (fun e => decide (e.1 = kv.1))).isSome = false := by
            rw [Bool.eq_false_iff]
            intro h
            obtain ⟨e, he, hek⟩ := List.find?_isSome.mp h
            exact hk (List.mem_map.mpr ⟨e, he, of_decide_eq_true hek⟩)
          rw [if_neg (by simp [hf]), List.map_append, List.map_singleton]
        rw [hkeys, if_neg hk, List.append_assoc, List.singleton_append]
        congr 2
        exact fdGo_congr _ _ _ (fun x => by simp [or_comm])

/-- An association list with distinct keys is its key list and its lookups. -/
theorem assoc_ext : ∀ (l₁ l₂ : List (K × V)), (l₁.map Prod.fst).Nodup →
    l₁.map Prod.fst = l₂.map Prod.fst →
    (∀ k, (l₁.find? (fun e => decide (e.1 = k))).map Prod.snd
      = (l₂.find? (fun e => decide (e.1 = k))).map Prod.snd) → l₁ = l₂
  | [], [], _, _, _ => rfl
  | [], _ :: _, _, hk, _ => by simp at hk
  | _ :: _, [], _, hk, _ => by simp at hk
  | e₁ :: l₁, e₂ :: l₂, hn, hk, hl => by
    simp only [List.map_cons, List.cons.injEq] at hk
    obtain ⟨hk1, hk2⟩ := hk
    rw [List.map_cons, List.nodup_cons] at hn
    have he : e₁ = e₂ := by
      have hv := hl e₁.1
      rw [List.find?_cons_of_pos (by simp), List.find?_cons_of_pos (by simp [hk1])] at hv
      exact Prod.ext hk1 (Option.some.inj hv)
    subst he
    congr 1
    refine assoc_ext l₁ l₂ hn.2 hk2 fun k => ?_
    by_cases hke : e₁.1 = k
    · -- neither tail has the head's key
      have h1 : (l₁.find? (fun e => decide (e.1 = k))) = none := by
        rw [List.find?_eq_none]
        intro e he hek
        exact hn.1 (hke ▸ (of_decide_eq_true hek) ▸ List.mem_map.mpr ⟨e, he, rfl⟩)
      have h2 : (l₂.find? (fun e => decide (e.1 = k))) = none := by
        rw [List.find?_eq_none]
        intro e he hek
        have : e.1 ∈ l₂.map Prod.fst := List.mem_map.mpr ⟨e, he, rfl⟩
        rw [← hk2] at this
        exact hn.1 (hke ▸ (of_decide_eq_true hek) ▸ this)
      rw [h1, h2]
    · have := hl k
      simp only [List.find?_cons, hke, decide_false] at this
      exact this

/-- The fresh entries: each new key with its group folded (the first value seeds). -/
def freshOf (f : V → V → V) (G : K → List V) (ns : List K) : List (K × V) :=
  ns.filterMap (fun k => match G k with | v :: vs => some (k, vs.foldl f v) | [] => none)

theorem freshOf_keys_sublist (f : V → V → V) (G : K → List V) :
    ∀ (ns : List K), ((freshOf f G ns).map Prod.fst).Sublist ns
  | [] => List.Sublist.refl _
  | n :: ns => by
    unfold freshOf
    rw [List.filterMap_cons]
    cases hG : G n with
    | nil => exact (freshOf_keys_sublist f G ns).cons _
    | cons v vs =>
      simp only [List.map_cons]
      exact (freshOf_keys_sublist f G ns).cons₂ _

theorem freshOf_keys (f : V → V → V) (G : K → List V) :
    ∀ (ns : List K), (∀ k ∈ ns, G k ≠ []) → (freshOf f G ns).map Prod.fst = ns
  | [], _ => rfl
  | n :: ns, h => by
    unfold freshOf
    rw [List.filterMap_cons]
    cases hG : G n with
    | nil => exact absurd hG (h n List.mem_cons_self)
    | cons v vs =>
      simp only [List.map_cons]
      congr 1
      exact freshOf_keys f G ns (fun k hk => h k (List.mem_cons_of_mem _ hk))

theorem freshOf_find_mem (f : V → V → V) (G : K → List V) :
    ∀ (ns : List K), ns.Nodup → ∀ (k : K), k ∈ ns → ∀ (v : V) (vs : List V), G k = v :: vs →
      ((freshOf f G ns).find? (fun e => decide (e.1 = k))).map Prod.snd = some (vs.foldl f v)
  | [], _, k, hk, _, _, _ => absurd hk List.not_mem_nil
  | n :: ns, hnd, k, hk, v, vs, hG => by
    unfold freshOf
    rw [List.filterMap_cons, List.nodup_cons] at *
    by_cases hnk : n = k
    · subst hnk
      rw [hG]
      simp
    · have hk' : k ∈ ns := by
        rcases List.mem_cons.mp hk with h | h
        · exact absurd h.symm hnk
        · exact h
      cases hGn : G n with
      | nil => exact freshOf_find_mem f G ns hnd.2 k hk' v vs hG
      | cons v' vs' =>
        simp only
        rw [List.find?_cons_of_neg (by simp [hnk])]
        exact freshOf_find_mem f G ns hnd.2 k hk' v vs hG

theorem freshOf_find_not_mem (f : V → V → V) (G : K → List V) (ns : List K) (k : K) (hk : k ∉ ns) :
    (freshOf f G ns).find? (fun e => decide (e.1 = k)) = none := by
  rw [List.find?_eq_none]
  intro e he hek
  obtain ⟨n, hn, hne⟩ := List.mem_filterMap.mp he
  have : e.1 = n := by
    revert hne
    cases G n with
    | nil => intro h; cases h
    | cons v vs => intro h; cases h; rfl
  have hek' : e.1 = k := of_decide_eq_true hek
  rw [← hek', this] at hk
  exact hk hn

/-- The payload batch, through a hash map (values grouped per key in arrival order). -/
def payloadsImpl [Hashable K] (f : V → V → V) (w : Option K) (m : SchedKMap K V) (l : List (K × V)) :
    SchedKMap K V :=
  let kept := l.filter (fun kv => !belowWm w kv.1)
  let grp := groupRev kept
  let G : K → List V := fun k => ((grp[k]?).getD []).reverse
  let keysM : HashSet K := HashSet.ofList (m.val.map Prod.fst)
  let news := fdGoImpl keysM (kept.map Prod.fst)
  let upd := m.val.map (fun e => (e.1, (G e.1).foldl f e.2))
  ⟨upd ++ freshOf f G news, by
    rw [List.map_append, List.nodup_append]
    have hupd : upd.map Prod.fst = m.val.map Prod.fst := by
      simp only [upd, List.map_map]; rfl
    have hnews : news = fdGo (m.val.map Prod.fst) (kept.map Prod.fst) :=
      fdGoImpl_eq _ _ (fun x => HashSet.contains_ofList_iff _ x) _
    refine ⟨hupd ▸ m.property, (freshOf_keys_sublist f G news).nodup (hnews ▸ fdGo_nodup _ _), ?_⟩
    intro a ha b hb hab
    subst hab
    rw [hupd] at ha
    have hb' := (freshOf_keys_sublist f G news).subset hb
    rw [hnews, mem_fdGo] at hb'
    exact hb'.2 ha⟩

theorem foldl_reduceStep_some (f : V → V → V) (a : V) :
    ∀ (vs : List V), vs.foldl (reduceStep f) (some a) = some (vs.foldl f a)
  | [] => rfl
  | v :: vs => by rw [List.foldl_cons, List.foldl_cons]; exact foldl_reduceStep_some f (f a v) vs

theorem foldl_reduceStep_none (f : V → V → V) (v : V) (vs : List V) :
    (v :: vs).foldl (reduceStep f) none = some (vs.foldl f v) := by
  rw [List.foldl_cons]
  exact foldl_reduceStep_some f v vs

theorem keyVals_filter_not_below (w : Option K) (l : List (K × V)) (k : K) (hk : belowWm w k = false) :
    keyVals k (l.filter (fun kv => !belowWm w kv.1)) = keyVals k l := by
  unfold keyVals
  rw [List.filter_filter]
  congr 1
  apply List.filter_congr
  intro e _
  by_cases he : e.1 = k
  · simp [he, hk]
  · simp [he]

theorem keyVals_eq_nil_of_below (w : Option K) (l : List (K × V)) (k : K) (hk : belowWm w k = true) :
    keyVals k (l.filter (fun kv => !belowWm w kv.1)) = [] := by
  unfold keyVals
  rw [List.map_eq_nil_iff, List.filter_eq_nil_iff]
  intro e he hek
  have := (List.mem_filter.mp he).2
  rw [of_decide_eq_true hek, hk] at this
  simp at this

theorem lookup_map_key_preserving (l : List (K × V)) (g : K × V → V) (k : K) :
    ((l.map (fun e => (e.1, g e))).find? (fun e => decide (e.1 = k))).map Prod.snd
      = ((l.find? (fun e => decide (e.1 = k))).map (fun e => g e)) := by
  rw [List.find?_map]
  have hp : ((fun e : K × V => decide (e.1 = k)) ∘ fun e => (e.1, g e)) = fun e => decide (e.1 = k) := rfl
  rw [hp, Option.map_map]
  rfl

theorem find?_fst_eq (l : List (K × V)) (k : K) (e : K × V) (h : l.find? (fun e => decide (e.1 = k)) = some e) :
    e.1 = k := by
  have := List.find?_some h
  simpa using this

/-- The hash-backed batch IS the one-by-one fold. -/
theorem payloadsImpl_eq [Hashable K] (f : V → V → V) (w : Option K) (m : SchedKMap K V) (l : List (K × V)) :
    payloadsImpl f w m l = l.foldl (reduceWatermarkPayload f w) m := by
  apply Subtype.ext
  -- names for the pieces
  set kept := l.filter (fun kv => !belowWm w kv.1) with hkept
  set G : K → List V := fun k => (((groupRev kept)[k]?).getD []).reverse with hG
  have hGk : ∀ k, G k = keyVals k kept := fun k => by
    simp only [hG, groupRev_get, List.reverse_reverse]
  set news := fdGoImpl (HashSet.ofList (m.val.map Prod.fst)) (kept.map Prod.fst) with hnewsdef
  have hnews : news = fdGo (m.val.map Prod.fst) (kept.map Prod.fst) :=
    fdGoImpl_eq _ _ (fun x => HashSet.contains_ofList_iff _ x) _
  have hval : (payloadsImpl f w m l).val
      = m.val.map (fun e => (e.1, (G e.1).foldl f e.2)) ++ freshOf f G news := rfl
  have hgrp : ∀ k ∈ kept.map Prod.fst, G k ≠ [] := by
    intro k hk
    rw [hGk]
    obtain ⟨e, he, hek⟩ := List.mem_map.mp hk
    intro h
    have : e.2 ∈ keyVals k kept := by
      unfold keyVals
      exact List.mem_map.mpr ⟨e, List.mem_filter.mpr ⟨he, by simpa using hek⟩, rfl⟩
    rw [h] at this
    exact absurd this List.not_mem_nil
  have hkeysImpl : (payloadsImpl f w m l).val.map Prod.fst
      = m.val.map Prod.fst ++ fdGo (m.val.map Prod.fst) (kept.map Prod.fst) := by
    rw [hval, List.map_append, List.map_map]
    congr 1
    rw [hnews] at *
    exact freshOf_keys f G _ (fun k hk => hgrp k ((mem_fdGo _ _ _).mp hk).1)
  rw [hval]
  refine assoc_ext _ _ (hval ▸ (payloadsImpl f w m l).property)
    (by rw [← hval, hkeysImpl, keys_foldl_payload]) fun k => ?_
  -- lookups
  show _ = (l.foldl (reduceWatermarkPayload f w) m).lookup k
  rw [lookup_foldl_payload, List.find?_append]
  by_cases hk : k ∈ m.val.map Prod.fst
  · -- an existing key: found in the updated prefix
    obtain ⟨e, he, hek⟩ := List.mem_map.mp hk
    have hfind : m.val.find? (fun e => decide (e.1 = k)) ≠ none := by
      rw [Ne, List.find?_eq_none]
      intro h; exact h e he (by simpa using hek)
    obtain ⟨e', he'⟩ := Option.ne_none_iff_exists'.mp hfind
    have hk' := find?_fst_eq m.val k e' he'
    have hlook : m.lookup k = some e'.2 := by
      show (m.val.find? _).map Prod.snd = _
      rw [he']; rfl
    rw [List.find?_map]
    have hp : ((fun e : K × V => decide (e.1 = k)) ∘ fun e : K × V => (e.1, (G e.1).foldl f e.2))
        = fun e : K × V => decide (e.1 = k) := rfl
    rw [hp, he']
    simp only [Option.map_some, Option.or_some]
    rw [hGk, hlook, hk']
    by_cases hb : belowWm w k = true
    · rw [if_pos hb, keyVals_eq_nil_of_below w l k hb]
      rfl
    · rw [if_neg hb, keyVals_filter_not_below w l k (by simpa using hb), foldl_reduceStep_some]
      rfl
  · -- a new key: absent from the prefix, found among the fresh keys iff it arrived above the watermark
    have hnone : (m.val.map (fun e => (e.1, (G e.1).foldl f e.2))).find? (fun e => decide (e.1 = k)) = none := by
      rw [List.find?_eq_none]
      intro e he hek
      obtain ⟨e₀, he₀, rfl⟩ := List.mem_map.mp he
      exact hk (List.mem_map.mpr ⟨e₀, he₀, of_decide_eq_true hek⟩)
    rw [hnone, Option.none_or]
    have hlook : m.lookup k = none := by
      show (m.val.find? _).map Prod.snd = _
      rw [List.find?_eq_none.mpr]
      · rfl
      · intro e he hek; exact hk (List.mem_map.mpr ⟨e, he, of_decide_eq_true hek⟩)
    rw [hlook, hnews]
    by_cases hin : k ∈ kept.map Prod.fst
    · -- arrived above the watermark: among the new keys, with its group
      have hb : belowWm w k = false := by
        obtain ⟨e, he, hek⟩ := List.mem_map.mp hin
        have := (List.mem_filter.mp he).2
        rw [hek] at this
        simpa using this
      rw [if_neg (by simp [hb])]
      obtain ⟨v, vs, hvs⟩ := List.exists_cons_of_ne_nil (hgrp k hin)
      rw [freshOf_find_mem f G _ (fdGo_nodup _ _) k ((mem_fdGo _ _ _).mpr ⟨hin, hk⟩) v vs hvs]
      rw [← keyVals_filter_not_below w l k hb, ← hGk, hvs, foldl_reduceStep_none]
    · -- never arrived above the watermark: not a new key; the spec reads `none`
      rw [freshOf_find_not_mem f G _ k (fun h => hin ((mem_fdGo _ _ _).mp h).1)]
      by_cases hb : belowWm w k = true
      · rw [if_pos hb]; rfl
      · rw [if_neg hb]
        have : keyVals k l = [] := by
          rw [← keyVals_filter_not_below w l k (by simpa using hb)]
          unfold keyVals
          rw [List.map_eq_nil_iff, List.filter_eq_nil_iff]
          intro e he hek
          exact hin (List.mem_map.mpr ⟨e, he, of_decide_eq_true hek⟩)
        rw [this]; rfl

/-- `reduce_watermark`'s per-tick step with the Rust's bound (`K: Hash`): the machine's
semantics (`reduceWatermarkEager`), computed through a hash map. -/
def reduceWatermarkEagerH [Hashable K] (f : V → V → V)
    (st : SchedKMap K V × Option K) (inp : List (K × V) × Option K) :
    (SchedKMap K V × Option K) × SchedKMap K V :=
  reduceWatermarkEager f st inp

@[simp] theorem reduceWatermarkEagerH_def [Hashable K] (f : V → V → V)
    (st : SchedKMap K V × Option K) (inp : List (K × V) × Option K) :
    reduceWatermarkEagerH f st inp = reduceWatermarkEager f st inp := rfl

def reduceWatermarkEagerImpl [Hashable K] (f : V → V → V)
    (st : SchedKMap K V × Option K) (inp : List (K × V) × Option K) :
    (SchedKMap K V × Option K) × SchedKMap K V :=
  let st' := reduceWatermarkAdvance (payloadsImpl f st.2 st.1 inp.1, st.2) inp.2
  (st', st'.1)

@[csimp] theorem reduceWatermarkEagerH_eq_impl : @reduceWatermarkEagerH = @reduceWatermarkEagerImpl := by
  funext K V _ _ _ f st inp
  unfold reduceWatermarkEagerH reduceWatermarkEagerImpl reduceWatermarkEager
  rw [payloadsImpl_eq]

end ReduceWatermark

end Hydro
