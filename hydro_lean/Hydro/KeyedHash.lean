import Std.Data.HashMap
import Std.Data.HashSet
import Hydro.Keyed

/-!
# Hash-backed in-tick keyed operators

Rust's `fold_keyed`, `join`, `anti_join` and `filter_not_in` build a `HashMap` or `HashSet`
inside the tick and demand `K: Hash + Eq` at the operator (the keyed collection types carry no
bound). The machine (`SchedSem`) defines them on association lists — `keyedFoldAssoc`,
`listJoin`, `filter (· ∉ ·)`. These list forms define the semantics (one list, keys in last-arrival
order), but they cost O(n·k) per tick where the Rust costs O(n). This file gives each operator
the Rust's implementation under the Rust's bound: the `…H` wrappers carry `[Hashable K]` and are
definitionally the list form, the `…Impl` versions compute with `Std.HashMap`/`Std.HashSet`, and
a kernel-checked `@[csimp]` equation lets the compiler substitute the `…Impl` for the `…H`. The
proofs stay on the lists; the list form defines the machine's semantics and the hash map
implements it.
-/

namespace Hydro

open Std

variable {K V W A : Type} [DecidableEq K]

/-! ## Membership tests through a hash set -/

/-- `List.dedup` (last occurrence kept, original order) through a hash set, in one pass
from the right. -/
def dedupImpl [Hashable K] (l : List K) : List K :=
  (l.foldr (fun x (acc : HashSet K × List K) =>
    if acc.1.contains x then acc else (acc.1.insert x, x :: acc.2)) (∅, [])).2

/-- The right fold of `dedupImpl` tracks the set of elements seen and produces `l.dedup`. -/
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

/-- `List.dedup` with a `Hashable` in scope: the identity wrapper the compiler replaces by
`dedupImpl`. -/
def dedupH [Hashable K] (l : List K) : List K := l.dedup

@[csimp] theorem dedupH_eq_dedupImpl : @dedupH = @dedupImpl := by
  funext K _ _ l
  exact (dedupImpl_spec l).2.symm

/-- `.filter_not_in(other)` on batches, the machine's semantics. The batch is an `Array`;
the proofs read it through `toList`. -/
def arrFilterNotIn {α : Type} [DecidableEq α] [Hashable α] (a o : Array α) : Array α :=
  a.filter (fun x => !(decide (x ∈ o.toList)))

@[simp] theorem arrFilterNotIn_toList {α : Type} [DecidableEq α] [Hashable α] (a o : Array α) :
    (arrFilterNotIn a o).toList = a.toList.filter (fun x => !(decide (x ∈ o.toList))) :=
  Array.toList_filter

/-- `HashSet.ofList` with the table sized for the list, so it does not rehash while it
fills. -/
def hashSetOfList {α : Type} [BEq α] [Hashable α] (o : List α) : HashSet α :=
  (HashSet.emptyWithCapacity o.length).insertMany o

/-- Membership in `hashSetOfList o` is membership in `o`. -/
theorem HashSet.contains_ofList_iff {α : Type} [DecidableEq α] [Hashable α] (o : List α) (x : α) :
    (hashSetOfList o).contains x = true ↔ x ∈ o := by
  unfold hashSetOfList
  rw [HashSet.contains_insertMany_list, HashSet.contains_emptyWithCapacity, Bool.false_or,
    List.contains_iff_mem]

/-- `arrFilterNotIn` through a hash set of the excluded batch. -/
def arrFilterNotInImpl {α : Type} [DecidableEq α] [Hashable α] (a o : Array α) : Array α :=
  let s : HashSet α := hashSetOfList o.toList
  a.filter (fun x => !s.contains x)

@[csimp] theorem arrFilterNotIn_eq_impl : @arrFilterNotIn = @arrFilterNotInImpl := by
  funext α _ _ a o
  unfold arrFilterNotIn arrFilterNotInImpl
  simp only
  apply Array.toList_inj.mp
  rw [Array.toList_filter, Array.toList_filter]
  apply List.filter_congr
  intro x _
  congr 1
  rw [Bool.eq_iff_iff, decide_eq_true_eq, HashSet.contains_ofList_iff]

/-- `.anti_join(keys)` on batches, the machine's semantics. -/
def arrAntiJoin [Hashable K] (a : Array (K × V)) (ks : Array K) : Array (K × V) :=
  a.filter (fun e => !(decide (e.1 ∈ ks.toList)))

@[simp] theorem arrAntiJoin_toList [Hashable K] (a : Array (K × V)) (ks : Array K) :
    (arrAntiJoin a ks).toList = a.toList.filter (fun e => !(decide (e.1 ∈ ks.toList))) :=
  Array.toList_filter

/-- `arrAntiJoin` through a hash set of the excluded keys. -/
def arrAntiJoinImpl [Hashable K] (a : Array (K × V)) (ks : Array K) : Array (K × V) :=
  let s : HashSet K := hashSetOfList ks.toList
  a.filter (fun e => !s.contains e.1)

@[csimp] theorem arrAntiJoin_eq_impl : @arrAntiJoin = @arrAntiJoinImpl := by
  funext K V _ _ a ks
  unfold arrAntiJoin arrAntiJoinImpl
  simp only
  apply Array.toList_inj.mp
  rw [Array.toList_filter, Array.toList_filter]
  apply List.filter_congr
  intro e _
  congr 1
  rw [Bool.eq_iff_iff, decide_eq_true_eq, HashSet.contains_ofList_iff]

/-! ## `.join` through a hash map of the right side -/

/-- The right side grouped by key. Values are consed as the fold meets them, so each key's
list is in reverse arrival order. -/
def groupRev [Hashable K] (b : List (K × W)) : HashMap K (List W) :=
  b.foldl (fun m f => m.alter f.1 (fun o => some (f.2 :: o.getD []))) (HashMap.emptyWithCapacity b.length)

/-- A key's group in `groupRev b` is `keyVals k b` reversed. -/
theorem groupRev_get [Hashable K] (b : List (K × W)) (k : K) :
    ((groupRev b)[k]?).getD [] = (keyVals k b).reverse := by
  unfold groupRev
  suffices h : ∀ (m : HashMap K (List W)),
      ((b.foldl (fun m f => m.alter f.1 (fun o => some (f.2 :: o.getD []))) m)[k]?).getD []
        = (keyVals k b).reverse ++ (m[k]?).getD [] by
    simpa using h (HashMap.emptyWithCapacity b.length)
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

/-- `.join(other)` on batches, the machine's semantics: `listJoin` of the contents. -/
def arrJoinH [Hashable K] (a : Array (K × V)) (b : Array (K × W)) : Array (K × (V × W)) :=
  (listJoin a.toList b.toList).toArray

@[simp] theorem arrJoinH_toList [Hashable K] (a : Array (K × V)) (b : Array (K × W)) :
    (arrJoinH a b).toList = listJoin a.toList b.toList := List.toList_toArray

/-- `arrJoinH` through `groupRev` of the right side: one lookup per left entry. -/
def arrJoinImpl [Hashable K] (a : Array (K × V)) (b : Array (K × W)) : Array (K × (V × W)) :=
  let m := groupRev b.toList
  a.flatMap (fun e => (((m[e.1]?).getD []).reverse.map (fun w => (e.1, (e.2, w)))).toArray)

@[csimp] theorem arrJoinH_eq_impl : @arrJoinH = @arrJoinImpl := by
  funext K V W _ _ a b
  apply Array.toList_inj.mp
  unfold arrJoinH arrJoinImpl listJoin
  simp only [List.toList_toArray, Array.toList_flatMap, Array.toList_map]
  apply List.flatMap_congr
  intro e _
  rw [groupRev_get, List.reverse_reverse]
  unfold keyVals
  rw [List.map_map]
  rfl

/-- A key absent from the key list has no values. -/
theorem keyVals_eq_nil_of_not_mem' (k : K) (l : List (K × V)) (h : k ∉ l.map Prod.fst) :
    (l.filter (fun e => decide (e.1 = k))).map Prod.snd = [] := by
  rw [List.map_eq_nil_iff, List.filter_eq_nil_iff]
  intro e he hk
  exact h (List.mem_map.mpr ⟨e, he, of_decide_eq_true hk⟩)

/-- The accumulators of a keyed fold: each key's values folded in arrival order. -/
def foldAcc [Hashable K] (g : A → V → A) (init : A) (l : List (K × V)) : HashMap K A :=
  l.foldl (fun m e => m.alter e.1 (fun o => some (g (o.getD init) e.2))) ∅

/-- `foldAcc` holds a key iff the key occurs in `l`, and then holds the fold of `keyVals`. -/
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

/-- `fold_keyed` on an arrival list: the machine's semantics (`keyedFoldAssoc`) with the
Rust's bound. -/
def keyedFoldAssocH [Hashable K] (g : A → V → A) (init : A) (l : List (K × V)) : SchedKMap K A :=
  keyedFoldAssoc g init l

@[simp] theorem keyedFoldAssocH_def [Hashable K] (g : A → V → A) (init : A) (l : List (K × V)) :
    keyedFoldAssocH g init l = keyedFoldAssoc g init l := rfl

/-- One element of a right pass over an arrival list. The pass meets elements from the last
to the first, so consing each value onto its key's list leaves the values in arrival order
and consing each key on its first (rightmost) sighting leaves the keys in `List.dedup`
order (last occurrence, original order). One hash operation per element into a map sized
for the list. -/
def groupKeysStep [Hashable K] (e : K × V) (acc : HashMap K (List V) × List K) : HashMap K (List V) × List K :=
  match acc.1[e.1]? with
  | some vs => (acc.1.insert e.1 (e.2 :: vs), acc.2)
  | none => (acc.1.insert e.1 [e.2], e.1 :: acc.2)

/-- The right pass of `groupKeysStep` over a whole list: per key its values in arrival
order, and the keys in `List.dedup` order. -/
def groupKeys [Hashable K] (l : List (K × V)) : HashMap K (List V) × List K :=
  l.foldr groupKeysStep (HashMap.emptyWithCapacity l.length, [])

/-- `keyVals` of a cons: the head's value is kept iff its key matches. -/
theorem keyVals_cons (k : K) (e : K × V) (l : List (K × V)) :
    keyVals k (e :: l) = if e.1 = k then e.2 :: keyVals k l else keyVals k l := by
  unfold keyVals
  simp only [List.filter_cons]
  by_cases h : e.1 = k <;> simp [h]

/-- The invariant of the right pass at any initial capacity: the map holds `keyVals k l` for
exactly the keys of `l`, and the key list is `(l.map Prod.fst).dedup`. -/
theorem groupKeys_foldr_spec [Hashable K] (c : Nat) : ∀ (l : List (K × V)),
    (∀ k, (l.foldr groupKeysStep (HashMap.emptyWithCapacity c, [])).1[k]?
        = if k ∈ l.map Prod.fst then some (keyVals k l) else none) ∧
      (l.foldr groupKeysStep (HashMap.emptyWithCapacity c, [])).2 = (l.map Prod.fst).dedup
  | [] => by simp [keyVals]
  | e :: l => by
    obtain ⟨ih1, ih2⟩ := groupKeys_foldr_spec c l
    rw [List.foldr_cons]
    generalize hr : l.foldr groupKeysStep (HashMap.emptyWithCapacity c, []) = r at ih1 ih2
    unfold groupKeysStep
    by_cases hmem : e.1 ∈ l.map Prod.fst
    · have hlook : r.1[e.1]? = some (keyVals e.1 l) := by rw [ih1, if_pos hmem]
      simp only [hlook]
      refine ⟨fun k => ?_, ?_⟩
      · rw [HashMap.getElem?_insert]
        by_cases hk : e.1 = k
        · subst hk
          simp [keyVals_cons, hmem]
        · have hk' : (e.1 == k) = false := by simp [hk]
          simp only [hk', Bool.false_eq_true, if_false, ih1, List.map_cons, List.mem_cons]
          rw [keyVals_cons, if_neg hk]
          by_cases hkl : k ∈ l.map Prod.fst
          · simp [hkl]
          · have hke : ¬ k = e.1 := fun h => hk h.symm
            simp [hkl, hke]
      · simp only [List.map_cons]
        rw [List.dedup_cons_of_mem hmem]
        exact ih2
    · have hlook : r.1[e.1]? = none := by rw [ih1, if_neg hmem]
      simp only [hlook]
      refine ⟨fun k => ?_, ?_⟩
      · rw [HashMap.getElem?_insert]
        by_cases hk : e.1 = k
        · subst hk
          have h0 : keyVals e.1 l = [] := keyVals_eq_nil_of_not_mem' e.1 l hmem
          simp [keyVals_cons, h0]
        · have hk' : (e.1 == k) = false := by simp [hk]
          simp only [hk', Bool.false_eq_true, if_false, ih1, List.map_cons, List.mem_cons]
          rw [keyVals_cons, if_neg hk]
          by_cases hkl : k ∈ l.map Prod.fst
          · simp [hkl]
          · have hke : ¬ k = e.1 := fun h => hk h.symm
            simp [hkl, hke]
      · simp only [List.map_cons]
        rw [List.dedup_cons_of_notMem hmem, ih2]

/-- `groupKeys_foldr_spec` at the capacity `groupKeys` uses. -/
theorem groupKeys_spec [Hashable K] (l : List (K × V)) :
    (∀ k, (groupKeys l).1[k]? = if k ∈ l.map Prod.fst then some (keyVals k l) else none) ∧
      (groupKeys l).2 = (l.map Prod.fst).dedup :=
  groupKeys_foldr_spec l.length l

/-- `keyedFoldAssocH` through `groupKeys`: the keys in `dedup` order, each paired with the
fold of its group. -/
def keyedFoldAssocImpl [Hashable K] (g : A → V → A) (init : A) (l : List (K × V)) : SchedKMap K A :=
  let r := groupKeys l
  ⟨r.2.map (fun k => (k, ((r.1[k]?).getD []).foldl g init)), by
    rw [List.map_map]
    have : (Prod.fst ∘ fun k => (k, ((r.1[k]?).getD []).foldl g init)) = (id : K → K) := rfl
    rw [this, List.map_id]
    show (groupKeys l).2.Nodup
    rw [(groupKeys_spec l).2]
    exact List.nodup_dedup _⟩

@[csimp] theorem keyedFoldAssocH_eq_impl : @keyedFoldAssocH = @keyedFoldAssocImpl := by
  funext K V A _ _ g init l
  unfold keyedFoldAssocH keyedFoldAssocImpl keyedFoldAssoc SchedKMap.ofKeyed
  apply Subtype.ext
  show List.filterMap _ _ = List.map _ (groupKeys l).2
  obtain ⟨h1, h2⟩ := groupKeys_spec l
  rw [h2]
  simp only [Option.map_some]
  rw [show (fun x : K => some (x, List.foldl g init (keyVals x l)))
      = some ∘ (fun x : K => (x, List.foldl g init (keyVals x l))) from rfl, List.filterMap_eq_map]
  apply List.map_congr_left
  intro k hk
  rw [h1, if_pos (List.mem_dedup.mp hk)]
  rfl

/-- `fold_keyed` on a batch (the in-tick `bkfold`): the machine's semantics on the batch's
contents. -/
def keyedFoldArrH [Hashable K] (g : A → V → A) (init : A) (a : Array (K × V)) : SchedKMap K A :=
  keyedFoldAssoc g init a.toList

@[simp] theorem keyedFoldArrH_def [Hashable K] (g : A → V → A) (init : A) (a : Array (K × V)) :
    keyedFoldArrH g init a = keyedFoldAssoc g init a.toList := rfl

/-- One element of the right pass over a batch: the key's slot is looked up and, if new,
assigned in one hash operation (`getThenInsertIfNew?`), and the value is consed onto the
slot's list in place. -/
def groupPosStep [Hashable K] (e : K × V) (acc : HashMap K Nat × Array K × Array (List V)) :
    HashMap K Nat × Array K × Array (List V) :=
  match acc.1.getThenInsertIfNew? e.1 acc.2.1.size with
  | (some p, m) => (m, acc.2.1, acc.2.2.modify p (e.2 :: ·))
  | (none, m) => (m, acc.2.1.push e.1, acc.2.2.push [e.2])

/-- The slots read out from the last to the first. Slot `0` was met first by the right pass
(the last key of the arrival list), so reading upward and consing puts it last: the keys end
up in `List.dedup` order. -/
def collectRev (g : A → V → A) (init : A) (ks : Array K) (vs : Array (List V)) :
    Nat → Nat → List (K × A) → List (K × A)
  | 0, _, acc => acc
  | fuel + 1, i, acc =>
    if h : i < ks.size then collectRev g init ks vs fuel (i + 1) ((ks[i], (vs.getD i []).foldl g init) :: acc)
    else acc

omit [DecidableEq K] in
/-- `collectRev` from slot `i` with fuel `ks.size - i` is the reversed map of `F` over the
remaining slots, when `F` describes every slot. -/
theorem collectRev_spec (g : A → V → A) (init : A) (ks : Array K) (vs : Array (List V))
    (F : K → K × A) (hE : ∀ (i : Nat) (h : i < ks.size), (ks[i], (vs.getD i []).foldl g init) = F ks[i]) :
    ∀ (fuel i : Nat) (acc : List (K × A)), fuel + i = ks.size →
      collectRev g init ks vs fuel i acc = ((ks.toList.drop i).map F).reverse ++ acc
  | 0, i, acc, h => by
    have : i = ks.size := by omega
    subst this
    simp [collectRev, List.drop_of_length_le]
  | fuel + 1, i, acc, h => by
    have hi : i < ks.size := by omega
    rw [collectRev.eq_2]
    simp only [hi, ↓reduceDIte]
    rw [collectRev_spec g init ks vs F hE fuel (i + 1) _ (by omega)]
    rw [List.drop_eq_getElem_cons (i := i) (l := ks.toList) (by simpa using hi), List.map_cons,
      List.reverse_cons, List.append_assoc, List.singleton_append, Array.getElem_toList, hE i hi]

/-- The invariant of the right pass over `l`: the slots are the keys in reverse `dedup`
order, the value array has one slot per key, each slot holds its key's values in arrival
order, and the map sends each key to its slot index. -/
theorem groupPos_foldr_spec [Hashable K] (c : Nat) : ∀ (l : List (K × V)),
    let r := l.foldr groupPosStep (HashMap.emptyWithCapacity c, #[], #[])
    r.2.1.toList.reverse = (l.map Prod.fst).dedup ∧
    r.2.2.size = r.2.1.size ∧
    (∀ (p : Nat) (h : p < r.2.1.size), r.2.2.getD p [] = keyVals (r.2.1[p]) l) ∧
    (∀ k p, r.1[k]? = some p ↔ ∃ h : p < r.2.1.size, r.2.1[p] = k)
  | [] => by
    refine ⟨rfl, rfl, fun p h => absurd h (by simp), fun k p => ?_⟩
    simp
  | e :: l => by
    intro r
    obtain ⟨ih1, ih2, ih3, ih4⟩ := groupPos_foldr_spec c l
    have hr : r = groupPosStep e (l.foldr groupPosStep (HashMap.emptyWithCapacity c, #[], #[])) := rfl
    generalize hs : l.foldr groupPosStep (HashMap.emptyWithCapacity c, #[], #[]) = s at ih1 ih2 ih3 ih4 hr
    obtain ⟨m, ks, vs⟩ := s
    simp only at ih1 ih2 ih3 ih4
    have hnd : ks.toList.Nodup := by
      rw [← List.nodup_reverse, ih1]; exact List.nodup_dedup _
    have hmem : ∀ k, k ∈ ks.toList ↔ k ∈ l.map Prod.fst := by
      intro k; rw [← List.mem_reverse, ih1, List.mem_dedup]
    -- a `none` lookup is exactly absence from the slot array
    have hlook : ∀ k, m[k]? = none ↔ k ∉ ks.toList := by
      intro k
      constructor
      · intro hn hk
        obtain ⟨p, hp, hkp⟩ := List.getElem_of_mem hk
        rw [Array.getElem_toList] at hkp
        have := (ih4 k p).mpr ⟨by simpa using hp, hkp⟩
        rw [hn] at this; cases this
      · intro hk
        cases hm : m[k]? with
        | none => rfl
        | some p =>
          obtain ⟨hp, hkp⟩ := (ih4 k p).mp hm
          exact absurd (hkp ▸ Array.getElem_mem_toList hp) hk
    unfold groupPosStep at hr
    simp only at hr
    cases hget : m[e.1]? with
    | some p =>
      have hp := (ih4 e.1 p).mp hget
      obtain ⟨hp, hkp⟩ := hp
      have hmemE : e.1 ∈ l.map Prod.fst := (hmem e.1).mp (hkp ▸ Array.getElem_mem_toList hp)
      have hstep : r = (m.insertIfNew e.1 ks.size, ks, vs.modify p (e.2 :: ·)) := by
        rw [hr]
        have h1 : (m.getThenInsertIfNew? e.1 ks.size) = (some p, m.insertIfNew e.1 ks.size) := by
          apply Prod.ext
          · rw [HashMap.getThenInsertIfNew?_fst, hget]
          · rw [HashMap.getThenInsertIfNew?_snd]
        rw [h1]
      rw [hstep]
      simp only
      have hin : e.1 ∈ m := by
        rw [HashMap.mem_iff_isSome_getElem?, hget]; rfl
      have hins : ∀ k : K, (m.insertIfNew e.1 ks.size)[k]? = m[k]? := by
        intro k
        rw [HashMap.getElem?_insertIfNew]
        simp [hin]
      refine ⟨?_, ?_, ?_, ?_⟩
      · rw [ih1, List.map_cons, List.dedup_cons_of_mem hmemE]
      · rw [Array.size_modify, ih2]
      · intro q hq
        rw [Array.getD_eq_getD_getElem?, Array.getElem?_modify, keyVals_cons]
        by_cases hpq : p = q
        · subst hpq
          rw [Array.getElem?_eq_getElem (by rw [ih2]; exact hq), hkp]
          simp only [Option.map_some, Option.getD_some, if_true]
          have := ih3 p hq
          rw [Array.getD_eq_getD_getElem?, Array.getElem?_eq_getElem (by rw [ih2]; exact hq)] at this
          simp only [Option.getD_some] at this
          rw [this, hkp]
        · have hne : ¬ e.1 = ks[q] := by
            intro heq
            apply hpq
            have := (hnd.getElem_inj_iff (hi := by simpa using hp) (hj := by simpa using hq)).mp
              (by rw [Array.getElem_toList, Array.getElem_toList, hkp, heq])
            exact this
          simp only [hpq, if_false, hne]
          have := ih3 q hq
          rw [Array.getD_eq_getD_getElem?] at this
          exact this
      · intro k q
        rw [hins]
        exact ih4 k q
    | none =>
      have hnot : e.1 ∉ ks.toList := (hlook e.1).mp hget
      have hnotE : e.1 ∉ l.map Prod.fst := fun h => hnot ((hmem e.1).mpr h)
      have hstep : r = (m.insertIfNew e.1 ks.size, ks.push e.1, vs.push [e.2]) := by
        rw [hr]
        have h1 : (m.getThenInsertIfNew? e.1 ks.size) = (none, m.insertIfNew e.1 ks.size) := by
          apply Prod.ext
          · rw [HashMap.getThenInsertIfNew?_fst, hget]
          · rw [HashMap.getThenInsertIfNew?_snd]
        rw [h1]
      rw [hstep]
      simp only
      have hnin : e.1 ∉ m := by
        rw [HashMap.mem_iff_isSome_getElem?, hget]; simp
      refine ⟨?_, ?_, ?_, ?_⟩
      · rw [Array.toList_push, List.reverse_append, List.reverse_singleton, List.singleton_append, ih1,
          List.map_cons, List.dedup_cons_of_notMem hnotE]
      · rw [Array.size_push, Array.size_push, ih2]
      · intro q hq
        rw [Array.size_push] at hq
        rw [Array.getD_eq_getD_getElem?, Array.getElem?_push]
        by_cases hqs : q = ks.size
        · subst hqs
          rw [Array.getElem_push_eq, if_pos ih2.symm]
          simp only [Option.getD_some]
          rw [keyVals_cons, if_pos rfl]
          show [e.2] = e.2 :: keyVals e.1 l
          rw [keyVals, keyVals_eq_nil_of_not_mem' _ _ hnotE]
        · have hq' : q < ks.size := by omega
          rw [Array.getElem_push_lt hq', if_neg (by rw [ih2]; exact hqs), keyVals_cons]
          have hne : ¬ e.1 = ks[q] := fun h => hnot (h ▸ Array.getElem_mem_toList hq')
          rw [if_neg hne]
          have := ih3 q hq'
          rw [Array.getD_eq_getD_getElem?] at this
          exact this
      · intro k q
        rw [HashMap.getElem?_insertIfNew]
        simp only [beq_iff_eq, hnin, not_false_eq_true, and_true, Array.size_push]
        constructor
        · intro h
          split at h
          · rename_i heq
            cases h
            exact ⟨by omega, by rw [Array.getElem_push_eq]; exact heq⟩
          · obtain ⟨hq, hkq⟩ := (ih4 k q).mp h
            exact ⟨by omega, by rw [Array.getElem_push_lt hq]; exact hkq⟩
        · rintro ⟨hq, hkq⟩
          by_cases hqs : q = ks.size
          · subst hqs
            rw [Array.getElem_push_eq] at hkq
            rw [if_pos hkq]
          · have hq' : q < ks.size := by omega
            rw [Array.getElem_push_lt hq'] at hkq
            have hne : ¬ e.1 = k := by
              intro h; apply hnot; rw [h, ← hkq]; exact Array.getElem_mem_toList hq'
            rw [if_neg hne]
            exact (ih4 k q).mpr ⟨hq', hkq⟩

/-- Reading the slots with `collectRev` gives the same list `keyedFoldAssocImpl` builds from
`groupKeys`, under the slot invariants `h1` and `h3`. -/
theorem collectRev_eq_groupKeys [Hashable K] (g : A → V → A) (init : A) (ks : Array K) (vs : Array (List V))
    (l : List (K × V)) (h1 : ks.toList.reverse = (l.map Prod.fst).dedup)
    (h3 : ∀ (p : Nat) (h : p < ks.size), vs.getD p [] = keyVals ks[p] l) :
    collectRev g init ks vs ks.size 0 [] =
      (groupKeys l).2.map (fun k => (k, (((groupKeys l).1[k]?).getD []).foldl g init)) := by
  rw [collectRev_spec g init ks vs (fun k => (k, (keyVals k l).foldl g init))
    (fun i h => by rw [h3 i h]) _ 0 [] (Nat.add_zero _), List.append_nil, List.drop_zero,
    ← List.map_reverse, h1]
  obtain ⟨g1, g2⟩ := groupKeys_spec l
  rw [g2]
  apply List.map_congr_left
  intro k hk
  rw [g1 k, if_pos (List.mem_dedup.mp hk)]
  rfl

/-- The keys read out by `collectRev` are distinct, by `collectRev_eq_groupKeys`. -/
theorem collectRev_nodup [Hashable K] (g : A → V → A) (init : A) (ks : Array K) (vs : Array (List V))
    (l : List (K × V)) (h1 : ks.toList.reverse = (l.map Prod.fst).dedup)
    (h3 : ∀ (p : Nat) (h : p < ks.size), vs.getD p [] = keyVals ks[p] l) :
    ((collectRev g init ks vs ks.size 0 []).map Prod.fst).Nodup := by
  rw [collectRev_eq_groupKeys g init ks vs l h1 h3, List.map_map]
  have : (Prod.fst ∘ fun k => (k, (((groupKeys l).1[k]?).getD []).foldl g init)) = (id : K → K) := rfl
  rw [this, List.map_id, (groupKeys_spec l).2]
  exact List.nodup_dedup _

/-- `fold_keyed` on a batch with one hash operation per element: `groupPosStep` looks the
key's slot up and assigns it in one `getThenInsertIfNew?`, the values are consed onto their
slot in place, and one pass over the slots folds each. -/
def keyedFoldArrImpl [Hashable K] (g : A → V → A) (init : A) (a : Array (K × V)) : SchedKMap K A :=
  let r := a.foldr groupPosStep (HashMap.emptyWithCapacity a.size, #[], #[])
  ⟨collectRev g init r.2.1 r.2.2 r.2.1.size 0 [], by
    obtain ⟨h1, _, h3, _⟩ := groupPos_foldr_spec a.size a.toList
    rw [Array.foldr_toList] at h1 h3
    exact collectRev_nodup g init _ _ a.toList h1 h3⟩

/-- The slot-based batch fold agrees with the `groupKeys` fold on the batch's list. -/
theorem keyedFoldArrImpl_eq [Hashable K] (g : A → V → A) (init : A) (a : Array (K × V)) :
    keyedFoldArrImpl g init a = keyedFoldAssocImpl g init a.toList := by
  apply Subtype.ext
  obtain ⟨h1, _, h3, _⟩ := groupPos_foldr_spec a.size a.toList
  rw [Array.foldr_toList] at h1 h3
  exact collectRev_eq_groupKeys g init _ _ a.toList h1 h3

@[csimp] theorem keyedFoldArrH_eq_impl : @keyedFoldArrH = @keyedFoldArrImpl := by
  funext K V A _ _ g init a
  rw [keyedFoldArrImpl_eq]
  exact congrFun (congrFun (congrFun (congrFun (congrFun (congrFun (congrFun (congrFun
    keyedFoldAssocH_eq_impl K) V) A) _) _) g) init) a.toList

/-! ## `fold_keyed` across ticks, incrementally

DFIR's `fold_keyed` keeps its `HashMap` of accumulators across ticks and folds each arrival
into its key's entry. The machine's `kfold` reads `keyedFoldAssoc` over the arrivals so far;
the production cell holds the map and advances it per arrival (`insertFold`), which gives
the same list by `keyedFoldAssoc_append_foldl`. The key's entry moves to the end on each
arrival because the machine's maps are keyed in last-arrival order (`List.dedup`). -/

/-- `List.dedup` of a list with one element appended: the element's earlier occurrences
drop out and it closes the list (last-occurrence order). -/
theorem dedup_append_singleton (xs : List K) (a : K) :
    (xs ++ [a]).dedup = xs.dedup.filter (fun x => !decide (x = a)) ++ [a] := by
  induction xs with
  | nil => simp
  | cons x xs ih =>
    rw [List.cons_append]
    by_cases hx : x ∈ xs ++ [a]
    · rw [List.dedup_cons_of_mem hx, ih]
      rcases List.mem_append.mp hx with hxs | hxa
      · rw [List.dedup_cons_of_mem hxs]
      · have hxa : x = a := by simpa using hxa
        by_cases hxs : x ∈ xs
        · rw [List.dedup_cons_of_mem hxs]
        · rw [List.dedup_cons_of_notMem hxs, List.filter_cons, hxa]
          simp
    · have hxs : x ∉ xs := fun h => hx (List.mem_append.mpr (Or.inl h))
      have hxa : ¬ x = a := fun h => hx (List.mem_append.mpr (Or.inr (by simp [h])))
      rw [List.dedup_cons_of_notMem hx, List.dedup_cons_of_notMem hxs, ih, List.filter_cons]
      simp [hxa]

/-- One arrival into a keyed fold's map: the key's accumulator advanced (`init` for a new
key) and its entry moved to the end. This is `fold_keyed.rs`'s
`#hashtable_ident.entry(kv.0).or_insert_with(#initfn)` followed by the combine, in the
machine's last-arrival order. -/
def SchedKMap.insertFoldList (g : A → V → A) (init : A) (l : List (K × A)) (e : K × V) : List (K × A) :=
  l.filter (fun p => !decide (p.1 = e.1))
    ++ [(e.1, g (((l.find? (fun p => decide (p.1 = e.1))).map Prod.snd).getD init) e.2)]

/-- `insertFoldList` keeps the keys distinct: the filter removes the key before the append. -/
theorem SchedKMap.insertFoldList_keys_nodup (g : A → V → A) (init : A) (l : List (K × A)) (e : K × V)
    (h : (l.map Prod.fst).Nodup) : ((SchedKMap.insertFoldList g init l e).map Prod.fst).Nodup := by
  unfold SchedKMap.insertFoldList
  rw [List.map_append, List.map_singleton, List.nodup_append_comm, List.singleton_append, List.nodup_cons]
  refine ⟨fun hk => ?_, (List.Sublist.map Prod.fst List.filter_sublist).nodup h⟩
  obtain ⟨p, hp, hpk⟩ := List.mem_map.mp hk
  have := (List.mem_filter.mp hp).2
  simp [hpk] at this

/-- `insertFoldList` on a `SchedKMap`. -/
def SchedKMap.insertFold (g : A → V → A) (init : A) (m : SchedKMap K A) (e : K × V) : SchedKMap K A :=
  ⟨SchedKMap.insertFoldList g init m.val e, SchedKMap.insertFoldList_keys_nodup g init m.val e m.property⟩

/-- The keyed fold of one more arrival is the fold's map advanced by it. -/
theorem keyedFoldAssoc_append_singleton (g : A → V → A) (init : A) (l : List (K × V)) (e : K × V) :
    keyedFoldAssoc g init (l ++ [e]) = (keyedFoldAssoc g init l).insertFold g init e := by
  apply Subtype.ext
  unfold keyedFoldAssoc SchedKMap.insertFold SchedKMap.ofKeyed
  simp only [SchedKMap.insertFoldList, Option.map_some, List.map_append, List.map_singleton]
  simp only [List.filterMap_eq_map']
  rw [dedup_append_singleton, List.map_append, List.map_singleton, List.filter_map]
  -- the accumulator the new entry starts from: the key's fold so far (`init` for a new key)
  have hacc : ((((l.map Prod.fst).dedup.map (fun k : K => (k, List.foldl g init (keyVals k l)))).find?
      (fun p => decide (p.1 = e.1))).map Prod.snd).getD init = List.foldl g init (keyVals e.1 l) := by
    rw [List.find?_map]
    have hp : ((fun p : K × A => decide (p.1 = e.1)) ∘ fun k : K => (k, List.foldl g init (keyVals k l)))
        = fun k : K => decide (k = e.1) := rfl
    rw [hp]
    by_cases hm : e.1 ∈ (l.map Prod.fst).dedup
    · obtain ⟨k, hk⟩ := Option.isSome_iff_exists.mp
        (List.find?_isSome.mpr ⟨e.1, hm, decide_eq_true rfl⟩ :
          ((l.map Prod.fst).dedup.find? (fun k : K => decide (k = e.1))).isSome = true)
      have hke : k = e.1 := of_decide_eq_true (List.find?_some (p := fun k : K => decide (k = e.1)) hk)
      rw [hk, hke]
      rfl
    · rw [List.find?_eq_none.mpr (fun k hk hke => hm ((of_decide_eq_true hke : k = e.1) ▸ hk))]
      have : keyVals e.1 l = [] := by
        unfold keyVals
        rw [List.map_eq_nil_iff, List.filter_eq_nil_iff]
        intro p hp hpe
        exact hm (List.mem_dedup.mpr (List.mem_map.mpr ⟨p, hp, of_decide_eq_true hpe⟩))
      rw [this]
      rfl
  rw [hacc]
  congr 1
  · apply List.map_congr_left
    intro k hk
    have hne : ¬ k = e.1 := by simpa using (List.mem_filter.mp hk).2
    rw [keyVals_append, keyVals_singleton, if_neg (fun h => hne h.symm), List.append_nil]
  · rw [keyVals_append, keyVals_singleton, if_pos rfl, List.foldl_append, List.foldl_cons, List.foldl_nil]

/-- The keyed fold of a batch of arrivals: the map advanced arrival by arrival. -/
theorem keyedFoldAssoc_append_foldl (g : A → V → A) (init : A) (l Δ : List (K × V)) :
    keyedFoldAssoc g init (l ++ Δ) = Δ.foldl (SchedKMap.insertFold g init) (keyedFoldAssoc g init l) := by
  induction Δ generalizing l with
  | nil => simp
  | cons e Δ ih =>
    rw [List.foldl_cons, ← keyedFoldAssoc_append_singleton, ← ih, List.append_assoc, List.singleton_append]

/-- The production cell's step with the Rust's bound: this tick's arrivals folded into the
map one by one. -/
def kfoldStepH [Hashable K] (g : A → V → A) (init : A) (m : SchedKMap K A) (Δ : Array (K × V)) : SchedKMap K A :=
  Δ.foldl (SchedKMap.insertFold g init) m

@[simp] theorem kfoldStepH_def [Hashable K] (g : A → V → A) (init : A) (m : SchedKMap K A) (Δ : Array (K × V)) :
    kfoldStepH g init m Δ = Δ.toList.foldl (SchedKMap.insertFold g init) m := (Array.foldl_toList _).symm

/-- Advancing the fold of `l` by the batch `Δ` is the fold of `l ++ Δ`. -/
theorem kfoldStepH_keyedFoldAssocH [Hashable K] (g : A → V → A) (init : A) (l : List (K × V)) (Δ : Array (K × V)) :
    kfoldStepH g init (keyedFoldAssocH g init l) Δ = keyedFoldAssocH g init (l ++ Δ.toList) := by
  rw [kfoldStepH_def, keyedFoldAssocH_def, keyedFoldAssocH_def, keyedFoldAssoc_append_foldl]

/-! ## The keyed-singleton in-tick operators of the benchmark client

`bench_client`'s stamping tick keeps one start time per in-flight request
(`start_times : KeyedSingleton`) and, every tick, joins it with the outputs
(`join_keyed_singleton`), drops the completed keys (`filter_key_not_in`) and reduces the
chained new entries (`reduce`). On the association list `SchedKMap.join` looks every key up
linearly and `keyedReduceAssoc` groups by a per-key filter, O(n²) per tick in the in-flight
count. The hash-backed forms below compute the same lists, by the `@[csimp]` equations. -/

/-- Dropping the `none`-valued entries keeps a sublist of the keys. -/
theorem map_fst_filterMap_optVal (l : List (K × Option V)) :
    (l.filterMap (fun e => e.2.map (fun v => (e.1, v)))).map Prod.fst
      = (l.filter (fun e => e.2.isSome)).map Prod.fst := by
  induction l with
  | nil => rfl
  | cons e rest ih =>
    simp only [List.filterMap_cons, List.filter_cons]
    cases e.2 with
    | none => simpa using ih
    | some v => simpa using ih

/-- `KeyedStream::reduce` on an arrival list: the machine's semantics (`keyedReduceAssoc`)
with the Rust's bound. -/
def keyedReduceAssocH [Hashable K] (f : V → V → V) (l : List (K × V)) : SchedKMap K V :=
  keyedReduceAssoc f l

@[simp] theorem keyedReduceAssocH_def [Hashable K] (f : V → V → V) (l : List (K × V)) :
    keyedReduceAssocH f l = keyedReduceAssoc f l := rfl

/-- The hashed fold with `reduceStep f` from `none`, then the `none` entries dropped. A key
of the list has at least one value, so no entry is `none` and the filter removes nothing.
The filter lets the result type be `SchedKMap K V`. -/
def keyedReduceAssocImpl [Hashable K] (f : V → V → V) (l : List (K × V)) : SchedKMap K V :=
  let m := keyedFoldAssocImpl (reduceStep f) none l
  ⟨m.val.filterMap (fun e => e.2.map (fun v => (e.1, v))), by
    rw [map_fst_filterMap_optVal]
    exact (List.Sublist.map Prod.fst List.filter_sublist).nodup m.property⟩

@[csimp] theorem keyedReduceAssocH_eq_impl : @keyedReduceAssocH = @keyedReduceAssocImpl := by
  funext K V _ _ f l
  unfold keyedReduceAssocH keyedReduceAssocImpl keyedReduceAssoc
  apply Subtype.ext
  simp only
  rw [← keyedFoldAssocH_eq_impl]
  unfold keyedFoldAssocH keyedFoldAssoc SchedKMap.ofKeyed
  simp only [Option.map_some]
  rw [show (fun x : K => some (x, List.foldl (reduceStep f) none (keyVals x l)))
      = some ∘ (fun x : K => (x, List.foldl (reduceStep f) none (keyVals x l))) from rfl,
    List.filterMap_eq_map, List.filterMap_map]
  rfl

/-- The first entry at a key carries the head of the key's values. -/
theorem find?_key_eq_head?_keyVals (k : K) (l : List (K × V)) :
    (l.find? (fun e => decide (e.1 = k))).map Prod.snd = (keyVals k l).head? := by
  induction l with
  | nil => rfl
  | cons e rest ih =>
    unfold keyVals
    simp only [List.find?_cons, List.filter_cons]
    by_cases h : e.1 = k
    · simp [h]
    · simp only [h, decide_false, Bool.false_eq_true, if_false]
      exact ih

/-- With distinct keys, the lookup of a present entry is its value. -/
theorem SchedKMap.lookup_of_mem (m : SchedKMap K V) {k : K} {v : V} (h : (k, v) ∈ m.val) :
    m.lookup k = some v := by
  unfold SchedKMap.lookup
  obtain ⟨l, hl⟩ := m
  induction l with
  | nil => cases h
  | cons e rest ih =>
    simp only [List.find?_cons]
    by_cases he : e.1 = k
    · simp only [he, decide_true, Option.map_some, Option.some.injEq]
      rcases List.mem_cons.mp h with rfl | hrest
      · rfl
      · exfalso
        have hk : k ∈ rest.map Prod.fst := List.mem_map.mpr ⟨(k, v), hrest, rfl⟩
        rw [List.map_cons, List.nodup_cons] at hl
        exact hl.1 (he ▸ hk)
    · simp only [he, decide_false, if_false]
      rcases List.mem_cons.mp h with rfl | hrest
      · exact absurd rfl he
      · rw [List.map_cons, List.nodup_cons] at hl
        exact ih hl.2 hrest

/-- `join_keyed_singleton` on association lists with the Rust's bound. -/
def SchedKMap.joinH [Hashable K] (a : SchedKMap K V) (b : SchedKMap K W) : SchedKMap K (V × W) :=
  a.join b

@[simp] theorem SchedKMap.joinH_def [Hashable K] (a : SchedKMap K V) (b : SchedKMap K W) :
    a.joinH b = a.join b := rfl

/-- `SchedKMap.joinH` through `groupRev` of the right side: each left entry looked up once.
`groupRev` stores a key's values reversed, so `getLast?` of the group is the first value at
the key, `b.lookup`. -/
def SchedKMap.joinImpl [Hashable K] (a : SchedKMap K V) (b : SchedKMap K W) : SchedKMap K (V × W) :=
  let m := groupRev b.val
  ⟨a.val.filterMap (fun e => (((m[e.1]?).getD []).getLast?).map (fun w => (e.1, (e.2, w)))), by
    have : (a.val.filterMap (fun e => (((m[e.1]?).getD []).getLast?).map (fun w => (e.1, (e.2, w)))))
        = (a.val.map (fun e => (e.1, (((m[e.1]?).getD []).getLast?).map (fun w => (e.2, w))))).filterMap
            (fun e => e.2.map (fun v => (e.1, v))) := by
      rw [List.filterMap_map]
      apply List.filterMap_congr
      intro e _
      simp only [Function.comp_apply]
      cases ((m[e.1]?).getD []).getLast? <;> rfl
    rw [this, map_fst_filterMap_optVal]
    refine (List.Sublist.map Prod.fst List.filter_sublist).nodup ?_
    rw [List.map_map]
    exact a.property⟩

@[csimp] theorem SchedKMap.joinH_eq_impl : @SchedKMap.joinH = @SchedKMap.joinImpl := by
  funext K V W _ _ a b
  unfold SchedKMap.joinH SchedKMap.joinImpl SchedKMap.join SchedKMap.ofKeyed
  apply Subtype.ext
  simp only
  rw [SchedKMap.keys, a.property.dedup, List.filterMap_map]
  apply List.filterMap_congr
  intro e he
  rw [Function.comp_apply, SchedKMap.lookup_of_mem a he, Option.bind_some, Option.map_map,
    SchedKMap.lookup, find?_key_eq_head?_keyVals, groupRev_get, List.getLast?_reverse]
  rfl

/-- `filter_key_not_in` on association lists with the Rust's bound: the map's entries pass
through `arrAntiJoin`, hashed. -/
def SchedKMap.filterKeyNotInH [Hashable K] (m : SchedKMap K V) (ks : Array K) : SchedKMap K V :=
  ⟨(arrAntiJoin m.val.toArray ks).toList, by
    rw [arrAntiJoin_toList, List.toList_toArray]
    exact (List.Sublist.map Prod.fst List.filter_sublist).nodup m.property⟩

@[simp] theorem SchedKMap.filterKeyNotInH_def [Hashable K] (m : SchedKMap K V) (ks : Array K) :
    m.filterKeyNotInH ks = m.filterEntries (fun e => !(decide (e.1 ∈ ks.toList))) := by
  apply Subtype.ext
  show (arrAntiJoin m.val.toArray ks).toList = _
  rw [arrAntiJoin_toList, List.toList_toArray]
  rfl

/-! ## `reduce_watermark`'s per-tick payload batch through a hash map

DFIR's `ReduceKeyedWatermark` folds this tick's payloads one by one into its map
(`reduceWatermarkPayload`: drop below the watermark, else `map.entry(k)` insert-or-reduce).
On the association list each insert is a `find?` over the whole log. The batch form below
groups the kept payloads by key in a hash map (values in arrival order), updates every
existing key in one pass, and appends the new keys in first-arrival order; `payloadsImpl_eq`
shows it is the same list. -/

section ReduceWatermark

variable [LinearOrder K]

/-- First-arrival dedup past a set of already-seen keys: the elements not in `seen`, each at
its first occurrence. -/
def fdGo (seen : List K) : List K → List K
  | [] => []
  | x :: xs => if x ∈ seen then fdGo seen xs else x :: fdGo (x :: seen) xs

/-- `fdGo` depends on `seen` only through membership. -/
theorem fdGo_congr : ∀ (xs : List K) (s₁ s₂ : List K), (∀ x, x ∈ s₁ ↔ x ∈ s₂) → fdGo s₁ xs = fdGo s₂ xs
  | [], _, _, _ => rfl
  | x :: xs, s₁, s₂, h => by
    simp only [fdGo]
    by_cases hx : x ∈ s₁
    · rw [if_pos hx, if_pos ((h x).mp hx)]; exact fdGo_congr xs s₁ s₂ h
    · rw [if_neg hx, if_neg (fun h' => hx ((h x).mpr h'))]
      congr 1
      exact fdGo_congr xs _ _ (fun y => by simp only [List.mem_cons, h y])

/-- Membership in `fdGo seen xs`: in `xs` and not in `seen`. -/
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

/-- `fdGo` yields distinct elements. -/
theorem fdGo_nodup : ∀ (xs seen : List K), (fdGo seen xs).Nodup
  | [], _ => List.nodup_nil
  | x :: xs, seen => by
    simp only [fdGo]
    split
    · exact fdGo_nodup xs seen
    · rw [List.nodup_cons, mem_fdGo]
      exact ⟨fun h => h.2 (List.mem_cons_self), fdGo_nodup xs _⟩

/-- `fdGo` through a hash set, in one left pass (the result is consed, so reversed at the
end). -/
def fdGoImpl [Hashable K] (seen : HashSet K) (xs : List K) : List K :=
  (xs.foldl (fun (acc : HashSet K × List K) x =>
    if acc.1.contains x then acc else (acc.1.insert x, x :: acc.2)) (seen, [])).2.reverse

/-- The invariant of `fdGoImpl`'s fold: with `seen` describing `sl` and `acc` already
produced, the reversed result is `acc.reverse ++ fdGo sl xs`. -/
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

/-- `fdGoImpl` is `fdGo` when the hash set describes the seen list. -/
theorem fdGoImpl_eq [Hashable K] (seen : HashSet K) (sl : List K)
    (hs : ∀ x, seen.contains x = true ↔ x ∈ sl) (xs : List K) : fdGoImpl seen xs = fdGo sl xs := by
  unfold fdGoImpl
  rw [fdGoImpl_go xs seen sl hs []]
  rfl

/-- The keys after the one-by-one payload fold: the old keys, then the kept arrivals' new keys
in first-arrival order. -/
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
      · -- the key is present: `insertReduce` keeps the key list, and `fdGo` skips the key
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
      · -- the key is absent: `insertReduce` appends it, and it heads `fdGo`'s new keys
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

/-- An association list with distinct keys is determined by its key list and its lookups. -/
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
    · -- the head's key: distinct keys put it in neither tail, so both lookups are `none`
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

/-- The fresh entries: each new key with its group folded, the first value seeding the fold.
A key with an empty group yields no entry. -/
def freshOf (f : V → V → V) (G : K → List V) (ns : List K) : List (K × V) :=
  ns.filterMap (fun k => match G k with | v :: vs => some (k, vs.foldl f v) | [] => none)

/-- The fresh entries' keys are a sublist of the new keys. -/
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
      exact (freshOf_keys_sublist f G ns).cons_cons _

/-- When every new key has a non-empty group, the fresh entries' keys are the new keys. -/
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

/-- The lookup of a new key with a non-empty group in the fresh entries is its group's fold. -/
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

/-- The lookup of a key outside the new keys in the fresh entries is `none`. -/
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

/-- The payload batch through a hash map: the kept payloads grouped per key in arrival order
(`G`), every existing entry folded with its group (`upd`), then the fresh entries for the new
keys in first-arrival order (`freshOf`). -/
def payloadsImpl [Hashable K] (f : V → V → V) (w : Option K) (m : SchedKMap K V) (l : List (K × V)) :
    SchedKMap K V :=
  let kept := l.filter (fun kv => !belowWm w kv.1)
  let grp := groupRev kept
  let G : K → List V := fun k => ((grp[k]?).getD []).reverse
  let keysM : HashSet K := hashSetOfList (m.val.map Prod.fst)
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

/-- `reduceStep f` from `some a` over a list is `some` of the plain fold. -/
theorem foldl_reduceStep_some (f : V → V → V) (a : V) :
    ∀ (vs : List V), vs.foldl (reduceStep f) (some a) = some (vs.foldl f a)
  | [] => rfl
  | v :: vs => by rw [List.foldl_cons, List.foldl_cons]; exact foldl_reduceStep_some f (f a v) vs

/-- `reduceStep f` from `none` over a non-empty list seeds with the head. -/
theorem foldl_reduceStep_none (f : V → V → V) (v : V) (vs : List V) :
    (v :: vs).foldl (reduceStep f) none = some (vs.foldl f v) := by
  rw [List.foldl_cons]
  exact foldl_reduceStep_some f v vs

/-- Filtering by the watermark does not change the values of a key at or above it. -/
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

/-- Filtering by the watermark leaves a key below it with no values. -/
theorem keyVals_eq_nil_of_below (w : Option K) (l : List (K × V)) (k : K) (hk : belowWm w k = true) :
    keyVals k (l.filter (fun kv => !belowWm w kv.1)) = [] := by
  unfold keyVals
  rw [List.map_eq_nil_iff, List.filter_eq_nil_iff]
  intro e he hek
  have := (List.mem_filter.mp he).2
  rw [of_decide_eq_true hek, hk] at this
  simp at this

/-- Lookup in a key-preserving map of an association list is the mapped lookup. -/
theorem lookup_map_key_preserving (l : List (K × V)) (g : K × V → V) (k : K) :
    ((l.map (fun e => (e.1, g e))).find? (fun e => decide (e.1 = k))).map Prod.snd
      = ((l.find? (fun e => decide (e.1 = k))).map (fun e => g e)) := by
  rw [List.find?_map]
  have hp : ((fun e : K × V => decide (e.1 = k)) ∘ fun e => (e.1, g e)) = fun e => decide (e.1 = k) := rfl
  rw [hp, Option.map_map]
  rfl

/-- The entry `find?` returns at key `k` has key `k`. -/
theorem find?_fst_eq (l : List (K × V)) (k : K) (e : K × V) (h : l.find? (fun e => decide (e.1 = k)) = some e) :
    e.1 = k := by
  have := List.find?_some h
  simpa using this

/-- The hash-backed batch is the one-by-one fold. The proof is by `assoc_ext`: the key lists
agree by `keys_foldl_payload`, and the lookups agree key by key through `lookup_foldl_payload`. -/
theorem payloadsImpl_eq [Hashable K] (f : V → V → V) (w : Option K) (m : SchedKMap K V) (l : List (K × V)) :
    payloadsImpl f w m l = l.foldl (reduceWatermarkPayload f w) m := by
  apply Subtype.ext
  set kept := l.filter (fun kv => !belowWm w kv.1) with hkept
  set G : K → List V := fun k => (((groupRev kept)[k]?).getD []).reverse with hG
  have hGk : ∀ k, G k = keyVals k kept := fun k => by
    simp only [hG, groupRev_get, List.reverse_reverse]
  set news := fdGoImpl (hashSetOfList (m.val.map Prod.fst)) (kept.map Prod.fst) with hnewsdef
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
  -- the lookups
  show _ = (l.foldl (reduceWatermarkPayload f w) m).lookup k
  rw [lookup_foldl_payload, List.find?_append]
  by_cases hk : k ∈ m.val.map Prod.fst
  · -- an existing key is found in the updated prefix
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
  · -- a new key is absent from the prefix; it is among the fresh keys iff it arrived at or
    -- above the watermark
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
    · -- arrived at or above the watermark: among the new keys, with its group
      have hb : belowWm w k = false := by
        obtain ⟨e, he, hek⟩ := List.mem_map.mp hin
        have := (List.mem_filter.mp he).2
        rw [hek] at this
        simpa using this
      rw [if_neg (by simp [hb])]
      obtain ⟨v, vs, hvs⟩ := List.exists_cons_of_ne_nil (hgrp k hin)
      rw [freshOf_find_mem f G _ (fdGo_nodup _ _) k ((mem_fdGo _ _ _).mpr ⟨hin, hk⟩) v vs hvs]
      rw [← keyVals_filter_not_below w l k hb, ← hGk, hvs, foldl_reduceStep_none]
    · -- never arrived at or above the watermark: not a new key, and the fold reads `none`
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
semantics (`reduceWatermarkEager`), computed through a hash map by `reduceWatermarkEagerImpl`. -/
def reduceWatermarkEagerH [Hashable K] (f : V → V → V)
    (st : SchedKMap K V × Option K) (inp : List (K × V) × Option K) :
    (SchedKMap K V × Option K) × SchedKMap K V :=
  reduceWatermarkEager f st inp

@[simp] theorem reduceWatermarkEagerH_def [Hashable K] (f : V → V → V)
    (st : SchedKMap K V × Option K) (inp : List (K × V) × Option K) :
    reduceWatermarkEagerH f st inp = reduceWatermarkEager f st inp := rfl

/-- `reduceWatermarkEager` with the payload batch through `payloadsImpl`; the watermark
advance is unchanged. -/
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
