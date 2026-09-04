import HydroLean.Hydro.Stream

/-!
# Shared List/Stream counting lemmas

General-purpose lemmas about `List.eraseDups`, duplicate-freedom, `countP`
algebra, and the keyed-count observables of `Hydro.Stream` (`countKey`,
`countKeyP`, `keys`). These were proven independently by the CollectQuorum,
TwoPC, and JoinResponses proof efforts; this file is the canonical home
(promoted during the dedup pass — the frozen program files keep private
copies where removing them would churn their public surface).

The keyed-count observables are the workhorse of batching-invariance proofs:
every state retained by a quorum-style tick loop is characterized *per key,
by counts*, and counts are order-insensitive — which is exactly why `NoOrder`
inputs are tractable (dissertation §4.3.4).
-/

namespace HydroLean

/-! ## `List` preliminaries missing from core -/

/-- `List.eraseDups` produces a duplicate-free list. -/
theorem _root_.List.eraseDups_nodup {α : Type u} [BEq α] [LawfulBEq α] :
    ∀ (l : List α), l.eraseDups.Nodup
  | [] => List.nodup_nil
  | a :: as => by
    rw [List.eraseDups_cons]
    refine List.nodup_cons.mpr ⟨fun hmem => ?_, ?_⟩
    · have hf := List.mem_filter.mp (List.mem_eraseDups.mp hmem)
      simp at hf
    · exact List.eraseDups_nodup (as.filter (fun b => !b == a))
termination_by l => l.length
decreasing_by
  simpa using Nat.lt_succ_of_le (List.length_filter_le _ _)

/-- In a duplicate-free list every element occurs at most once. -/
theorem _root_.List.Nodup.count_le_one {α : Type u} [DecidableEq α]
    {l : List α} (h : l.Nodup) (a : α) : l.count a ≤ 1 := by
  induction l with
  | nil => simp
  | cons x xs ih =>
    rw [List.count_cons]
    rcases List.nodup_cons.mp h with ⟨hx, hxs⟩
    by_cases hax : a = x
    · subst hax
      have h0 : xs.count a = 0 := List.count_eq_zero.mpr hx
      simp [h0]
    · have h1 := ih hxs
      have hxa : (x == a) = false :=
        beq_eq_false_iff_ne.mpr (fun h => hax h.symm)
      simp only [hxa, Bool.false_eq_true, if_false]
      omega

/-- In a duplicate-free list, counts are membership indicators. -/
theorem _root_.List.Nodup.count_eq_ite {α : Type u} [DecidableEq α]
    {l : List α} (h : l.Nodup) (k : α) :
    l.count k = if k ∈ l then 1 else 0 := by
  by_cases hk : k ∈ l
  · have h₁ := h.count_le_one k
    have h₂ := List.count_pos_iff.mpr hk
    simp only [hk, if_true]
    omega
  · simp [List.count_eq_zero.mpr hk, hk]

/-- A list whose image under `f` is duplicate-free is itself duplicate-free. -/
theorem _root_.List.Nodup.of_map {α : Type u} {β : Type v} {f : α → β}
    {l : List α} (h : (l.map f).Nodup) : l.Nodup := by
  induction l with
  | nil => exact List.Pairwise.nil
  | cons x xs ih =>
    rw [List.map_cons, List.nodup_cons] at h
    exact List.Pairwise.cons
      (fun y hy hxy => h.1 (hxy ▸ List.mem_map_of_mem hy)) (ih h.2)

/-- Counting elements equal to `k` that additionally satisfy `Q` collapses to
an if-then-else on `Q k`. -/
theorem _root_.List.countP_eq_and_ite {α : Type u} [DecidableEq α]
    (l : List α) (k : α) (Q : α → Bool) :
    l.countP (fun x => decide (x = k) && Q x) =
      if Q k then l.count k else 0 := by
  by_cases hq : Q k
  · simp only [hq, if_true, List.count_eq_countP]
    exact List.countP_congr fun x _ => by
      by_cases hx : x = k
      · subst hx; simp [hq]
      · simp [hx]
  · simp only [Bool.not_eq_true] at hq
    simp only [hq, Bool.false_eq_true, if_false]
    refine List.countP_eq_zero.mpr fun x _ => ?_
    by_cases hx : x = k
    · subst hx; simp [hq]
    · simp [hx]

/-- Sum of an `{0,1}`-valued map is a `countP`. -/
theorem _root_.List.sum_map_ite_one {α : Type u} (l : List α) (P : α → Bool) :
    (l.map (fun a => if P a then 1 else 0)).sum = l.countP P := by
  induction l with
  | nil => rfl
  | cons x xs ih =>
    rw [List.map_cons, List.sum_cons, ih, List.countP_cons]
    by_cases hx : P x <;> simp [hx, Nat.add_comm]

/-- Sum of a constant map is length times the constant. -/
theorem _root_.List.sum_map_const {α : Type u} (l : List α) (c : Nat) :
    (l.map (fun _ => c)).sum = l.length * c := by
  induction l with
  | nil => simp
  | cons x xs ih => simp [ih, Nat.succ_mul, Nat.add_comm]

/-- `eraseDups` produces a duplicate-free list. Alias of
`List.eraseDups_nodup`, kept under this (historical) name because the
CollectQuorum proof exposed it publicly. -/
theorem Programs.nodup_eraseDups {α : Type u} [BEq α] [LawfulBEq α]
    (l : List α) : l.eraseDups.Nodup :=
  List.eraseDups_nodup l

namespace Hydro.Stream

variable {κ : Type u} {V : Type v} [DecidableEq κ]

/-! ## Keyed-count observables of `Hydro.Stream` -/

theorem mem_keys {s : Stream (κ × V)} {k : κ} :
    k ∈ s.keys ↔ k ∈ s.map Prod.fst := List.mem_eraseDups

theorem nodup_keys (s : Stream (κ × V)) : s.keys.Nodup :=
  List.eraseDups_nodup _

/-- `Stream.keys` is duplicate-free (alias of `nodup_keys`). -/
theorem keys_nodup (s : Stream (κ × V)) : s.keys.Nodup :=
  nodup_keys s

/-- A key is in the key column iff it has a positive response count. -/
theorem mem_map_fst_iff_countKey_pos {s : Stream (κ × V)} {k : κ} :
    k ∈ s.map Prod.fst ↔ 0 < s.countKey k := by
  rw [Stream.countKey, List.countP_pos_iff]
  constructor
  · intro h
    obtain ⟨r, hr, rfl⟩ := List.mem_map.mp h
    exact ⟨r, hr, by simp⟩
  · rintro ⟨r, hr, hk⟩
    simp only [decide_eq_true_eq] at hk
    exact List.mem_map.mpr ⟨r, hr, hk⟩

/-- Membership in `Stream.keys` is positivity of the response count. -/
theorem mem_keys_iff {s : Stream (κ × V)} {k : κ} :
    k ∈ s.keys ↔ 0 < s.countKey k := by
  rw [Stream.keys, List.mem_eraseDups]
  exact mem_map_fst_iff_countKey_pos

/-- `countKey` is `countKeyP` at the trivial predicate. -/
theorem countKey_eq_countKeyP_true (s : Stream (κ × V)) (k : κ) :
    s.countKey k = s.countKeyP k (fun _ => true) := by
  unfold Stream.countKey Stream.countKeyP
  exact List.countP_congr fun r _ => by simp

/-- `countKeyP` of a filter whose predicate depends only on the key. -/
theorem countKeyP_filter_key (s : Stream (κ × V)) (q : κ → Bool) (k : κ)
    (pr : V → Bool) :
    (s.filter (fun r => q r.1)).countKeyP k pr
      = if q k then s.countKeyP k pr else 0 := by
  unfold countKeyP filter
  rw [List.countP_filter]
  split
  · next hq =>
    refine List.countP_congr fun x _ => ?_
    by_cases hx : x.1 = k
    · simp [hx, hq]
    · simp [hx]
  · next hq =>
    rw [List.countP_eq_zero]
    intro x _ hx
    obtain ⟨⟨hk, _⟩, hqx⟩ := by simpa using hx
    exact hq (hk ▸ hqx)

/-- `countKey` of a filter whose predicate depends only on the key. -/
theorem countKey_filter_key (s : Stream (κ × V)) (q : κ → Bool) (k : κ) :
    (s.filter (fun r => q r.1)).countKey k
      = if q k then s.countKey k else 0 := by
  unfold countKey filter
  rw [List.countP_filter]
  split
  · next hq =>
    refine List.countP_congr fun x _ => ?_
    by_cases hx : x.1 = k
    · simp [hx, hq]
    · simp [hx]
  · next hq =>
    rw [List.countP_eq_zero]
    intro x _ hx
    obtain ⟨hk, hqx⟩ := by simpa using hx
    exact hq (hk ▸ hqx)

/-- Filtering by a per-key-constant predicate, stated on the raw
`List.filter` spelling (the shape used by the 2PC proof). -/
theorem countKeyP_filter_keyconst (s : Stream (κ × V))
    (f : κ → Bool) (k : κ) (P : V → Bool) :
    Stream.countKeyP (List.filter (fun r => f r.1) s) k P
      = if f k then s.countKeyP k P else 0 :=
  countKeyP_filter_key s f k P

/-- Success counts are bounded by total counts. -/
theorem countKeyP_le_countKey (s : Stream (κ × V)) (k : κ) (pr : V → Bool) :
    s.countKeyP k pr ≤ s.countKey k :=
  List.countP_mono_left fun x _ h => by
    simp only [Bool.and_eq_true, decide_eq_true_eq] at h ⊢
    exact h.1

/-- A key with a positive filtered count appears in the key projection. -/
theorem mem_map_fst_of_countKeyP_pos {s : Stream (κ × V)} {k : κ} {pr : V → Bool}
    (h : 0 < s.countKeyP k pr) : k ∈ s.map Prod.fst := by
  obtain ⟨x, hx, hpx⟩ := List.countP_pos_iff.mp h
  simp only [Bool.and_eq_true, decide_eq_true_eq] at hpx
  exact List.mem_map.mpr ⟨x, hx, hpx.1⟩

/-- A key with no responses does not appear in the key projection. -/
theorem not_mem_map_fst_of_countKey_eq_zero {s : Stream (κ × V)} {k : κ}
    (h : s.countKey k = 0) : k ∉ s.map Prod.fst := by
  intro hk
  obtain ⟨x, hx, hfst⟩ := List.mem_map.mp hk
  have := List.countP_eq_zero.mp h x hx
  simp [hfst] at this

/-- An element of the stream has positive total count for its key. -/
theorem countKey_pos_of_mem {s : Stream (κ × V)} {x : κ × V} (hx : x ∈ s) :
    0 < s.countKey x.1 :=
  List.countP_pos_iff.mpr ⟨x, hx, by simp⟩

/-- Two duplicate-free lists with equal membership are permutations of each
other. (Standard; not in core Lean, so proven here.) -/
theorem _root_.List.perm_of_nodup_of_mem_iff {l₁ l₂ : List α}
    (h₁ : l₁.Nodup) (h₂ : l₂.Nodup) (hmem : ∀ a, a ∈ l₁ ↔ a ∈ l₂) :
    l₁.Perm l₂ := by
  induction l₁ generalizing l₂ with
  | nil =>
    cases l₂ with
    | nil => exact .nil
    | cons b t => exact absurd ((hmem b).mpr (List.mem_cons_self)) (by simp)
  | cons a t₁ ih =>
    have ha₂ : a ∈ l₂ := (hmem a).mp List.mem_cons_self
    obtain ⟨p, q, rfl⟩ := List.append_of_mem ha₂
    have hnd := List.nodup_append.mp h₂
    have hndpq : (p ++ q).Nodup := by
      refine List.nodup_append.mpr ⟨hnd.1, (List.nodup_cons.mp hnd.2.1).2, ?_⟩
      intro x hx y hy
      exact hnd.2.2 x hx y (List.mem_cons_of_mem a hy)
    have hat₁ : a ∉ t₁ := (List.nodup_cons.mp h₁).1
    have ht₁ : t₁.Perm (p ++ q) := by
      refine ih (List.nodup_cons.mp h₁).2 hndpq fun x => ⟨fun hx => ?_, fun hx => ?_⟩
      · have hx₂ : x ∈ p ++ a :: q := (hmem x).mp (List.mem_cons_of_mem a hx)
        have hxa : x ≠ a := fun h => hat₁ (h ▸ hx)
        simp only [List.mem_append, List.mem_cons] at hx₂ ⊢
        rcases hx₂ with h | h | h
        · exact Or.inl h
        · exact absurd h hxa
        · exact Or.inr h
      · have hx₁ : x ∈ a :: t₁ := (hmem x).mpr (by
          simp only [List.mem_append, List.mem_cons] at hx ⊢
          rcases hx with h | h
          · exact Or.inl h
          · exact Or.inr (Or.inr h))
        rcases List.mem_cons.mp hx₁ with rfl | h
        · -- x = a cannot be in p ++ q since l₂ is nodup
          exfalso
          simp only [List.mem_append] at hx
          rcases hx with h | h
          · exact hnd.2.2 x h x List.mem_cons_self rfl
          · exact (List.nodup_cons.mp hnd.2.1).1 h
        · exact h
    exact (ht₁.cons a).trans List.perm_middle.symm

end Hydro.Stream

end HydroLean
