import HydroLean.Hydro.StreamLemmas

/-!
# Cluster collections as member-indexed families: the counting rules

**The cluster-as-map insight** (user directive, mirroring Flo §2.5): just as
tick collections are streams-of-streams — so cross-tick invariants are
ordinary statements about the *nesting* — cluster collections are
**maps-to-streams**: a collection living on a cluster of `n` members is one
object `Fin n → List α` (member ↦ that member's stream), and collections
inside ticks on clusters are maps-to-streams-of-streams. Invariants *across
cluster members* are therefore ordinary statements about the family, not
per-edge plumbing: "a quorum exists" is a *counting statement over the map*
(`≥ m` members whose stream satisfies `P`).

This module provides the generic rules of that reading:

- `family_count_le` / `countP_fanin_exchange`: counting over a fan-in of
  per-member streams exchanges with the member-indexed sum — the bridge from
  a consumer's flat view (e.g. a quorum slice's consumed input) to the family
  view.
- `family_extract_distinct`: a total count ≥ `m` over a family of one-shot
  members (each member's stream satisfies `P` at most once) yields `m`
  **distinct** members — quorum-multiplicity ⇒ distinct participants, the
  fact whose absence is FINDINGS.md B1.
- `providers_extract`: the keyed/value refinement — per-value multiplicities
  covered by one-shot per-member capacities yield distinct members *each
  providing one of the values* (used when quorum entries carry payloads whose
  provenance the proof must track back to distinct members).
- `nodup_inter_of_length`: quorum intersection over `Fin n` (pigeonhole).

Everything here is pure list/`Fin` combinatorics: the rules are
protocol-independent and belong to the calculus, not to any program.
-/

namespace HydroLean.Hydro

/-! ## Sum helpers -/

theorem sum_map_le_sum_map {ι : Type _} (l : List ι) (g h : ι → Nat)
    (hle : ∀ j ∈ l, g j ≤ h j) :
    (l.map g).sum ≤ (l.map h).sum := by
  induction l with
  | nil => exact Nat.le_refl 0
  | cons a t ih =>
    rw [List.map_cons, List.map_cons, List.sum_cons, List.sum_cons]
    exact Nat.add_le_add (hle a (List.mem_cons_self ..))
      (ih (fun j hj => hle j (List.mem_cons_of_mem _ hj)))

theorem sum_pos_exists {ι : Type _} {l : List ι} {g : ι → Nat}
    (h : 0 < (l.map g).sum) : ∃ j ∈ l, 0 < g j := by
  induction l with
  | nil => cases h
  | cons a t ih =>
    rw [List.map_cons, List.sum_cons] at h
    by_cases ha : 0 < g a
    · exact ⟨a, List.mem_cons_self .., ha⟩
    · have : g a = 0 := Nat.eq_zero_of_not_pos ha
      rw [this, Nat.zero_add] at h
      obtain ⟨j, hj, hpos⟩ := ih h
      exact ⟨j, List.mem_cons_of_mem _ hj, hpos⟩

theorem sum_of_all_zero {l : List Nat} (h : ∀ x ∈ l, x = 0) : l.sum = 0 := by
  induction l with
  | nil => rfl
  | cons a t ih =>
    rw [List.sum_cons, h a (List.mem_cons_self ..),
      ih (fun x hx => h x (List.mem_cons_of_mem _ hx))]

/-- Zeroing a member of a nodup list removes exactly its contribution. -/
theorem sum_map_zero_at {ι : Type _} [DecidableEq ι] {l : List ι}
    (hnd : l.Nodup) {j₀ : ι} (hj₀ : j₀ ∈ l) (g : ι → Nat) :
    (l.map (fun j => if j = j₀ then 0 else g j)).sum + g j₀
      = (l.map g).sum := by
  induction l with
  | nil => cases hj₀
  | cons a t ih =>
    rw [List.map_cons, List.map_cons, List.sum_cons, List.sum_cons]
    rcases List.mem_cons.mp hj₀ with rfl | hj₀t
    · rw [if_pos rfl, Nat.zero_add]
      have hnotin : j₀ ∉ t := (List.nodup_cons.mp hnd).1
      have : (t.map (fun j => if j = j₀ then 0 else g j)).sum
          = (t.map g).sum := by
        congr 1
        refine List.map_congr_left ?_
        intro x hx
        rw [if_neg (fun hc => hnotin (by rw [← hc]; exact hx))]
      rw [this, Nat.add_comm]
    · have ha : a ≠ j₀ := fun hc =>
        (List.nodup_cons.mp hnd).1 (hc ▸ hj₀t)
      rw [if_neg ha, Nat.add_assoc,
        ih (List.nodup_cons.mp hnd).2 hj₀t]

/-- Pointwise sums split. -/
theorem sum_map_add {ι : Type _} (l : List ι) (x y : ι → Nat) :
    (l.map (fun j => x j + y j)).sum = (l.map x).sum + (l.map y).sum := by
  induction l with
  | nil => rfl
  | cons a t ih =>
    rw [List.map_cons, List.map_cons, List.map_cons, List.sum_cons,
      List.sum_cons, List.sum_cons, ih]
    omega

/-! ## Count helpers -/

/-- `countP` is monotone under prefixes. -/
theorem countP_le_of_prefix {α : Type _} {l l' : List α} (h : l <+: l')
    (p : α → Bool) : l.countP p ≤ l'.countP p := by
  obtain ⟨rest, rfl⟩ := h
  rw [List.countP_append]
  exact Nat.le_add_right _ _

/-- `count` is monotone under prefixes. -/
theorem count_le_of_prefix {α : Type _} [BEq α] {l l' : List α} (h : l <+: l')
    (a : α) : l.count a ≤ l'.count a := by
  obtain ⟨rest, rfl⟩ := h
  rw [List.count_append]
  exact Nat.le_add_right _ _

/-- Nodup lists count every element at most once. -/
theorem count_le_one_of_nodup {α : Type _} [DecidableEq α] {l : List α}
    (h : l.Nodup) (a : α) : l.count a ≤ 1 := by
  induction l with
  | nil => exact Nat.zero_le 1
  | cons x t ih =>
    rw [List.count_cons]
    by_cases hxa : x = a
    · subst hxa
      have hnotin : x ∉ t := (List.nodup_cons.mp h).1
      have hzero : t.count x = 0 := by
        rw [List.count_eq_zero]
        exact hnotin
      simp [hzero]
    · rw [if_neg (by simpa using fun hc => hxa (by exact hc))]
      have := ih (List.nodup_cons.mp h).2
      omega

/-- Key-nodup lists count any fixed key at most once (for any predicate that
pins the key). -/
theorem countP_key_le_one {α κ : Type _} [DecidableEq κ]
    {l : List α} {g : α → κ} (hnd : (l.map g).Nodup) (a : κ)
    {p : α → Bool} (hp : ∀ x, p x = true → g x = a) :
    l.countP p ≤ 1 := by
  induction l with
  | nil => exact Nat.zero_le 1
  | cons x t ih =>
    rw [List.map_cons, List.nodup_cons] at hnd
    rw [List.countP_cons]
    by_cases hx : p x = true
    · rw [if_pos hx]
      have hz : t.countP p = 0 := by
        rw [List.countP_eq_zero]
        intro y hy hpy
        exact hnd.1 (List.mem_map.mpr ⟨y, hy, (hp y hpy).trans (hp x hx).symm⟩)
      omega
    · rw [if_neg hx, Nat.add_zero]
      exact ih hnd.2

/-- Two disjoint predicates that both imply a third have summed counts
bounded by its count. -/
theorem countP_disjoint_le {α : Type _} (l : List α) (p q r : α → Bool)
    (hdisj : ∀ a, ¬(p a = true ∧ q a = true))
    (hp : ∀ a, p a = true → r a = true)
    (hq : ∀ a, q a = true → r a = true) :
    l.countP p + l.countP q ≤ l.countP r := by
  induction l with
  | nil => exact Nat.le_refl 0
  | cons a t ih =>
    rw [List.countP_cons, List.countP_cons, List.countP_cons]
    by_cases hpa : p a = true
    · have hqa : ¬ q a = true := fun hqa => hdisj a ⟨hpa, hqa⟩
      rw [if_pos hpa, if_neg hqa, if_pos (hp a hpa)]
      omega
    · by_cases hqa : q a = true
      · rw [if_neg hpa, if_pos hqa, if_pos (hq a hqa)]
        omega
      · rw [if_neg hpa, if_neg hqa]
        have := ih
        by_cases hra : r a = true
        · rw [if_pos hra]
          omega
        · rw [if_neg hra]
          omega

/-! ## The family fan-in exchange

A cluster-side consumer (e.g. a quorum slice with one port per member) sees a
flat stream assembled per tick from per-member batches. Counting over that
flat view **exchanges with the member-indexed sum** — the flat runtime
representation and the map-of-streams proof representation agree, which is
the collection-level two-lens correspondence for clusters. -/

/-- `countP` over `flatMap` is the member-indexed sum of per-piece counts. -/
theorem countP_flatMap_eq_sum {α β : Type _} (l : List α) (f : α → List β)
    (p : β → Bool) :
    (l.flatMap f).countP p = (l.map (fun a => (f a).countP p)).sum := by
  induction l with
  | nil => rfl
  | cons a rest ih =>
    rw [List.flatMap_cons, List.countP_append, List.map_cons, List.sum_cons, ih]

/-- Double-flatten exchange: counting over per-tick per-member fan-in equals
the member-indexed sum of per-member consumption counts. -/
theorem countP_fanin_exchange {α ι M : Type _} (bs : List M) (l : List ι)
    (g : M → ι → List α) (p : α → Bool) :
    ((bs.map (fun bb => l.flatMap (fun j => g bb j))).flatten).countP p
      = (l.map (fun j => ((bs.map (fun bb => g bb j)).flatten).countP p)).sum := by
  induction bs with
  | nil =>
    have hmap : (l.map (fun j =>
        ((List.map (fun bb => g bb j) ([] : List M)).flatten).countP p))
        = l.map (fun _ => 0) :=
      List.map_congr_left (fun j _ => rfl)
    rw [hmap]
    have hz : (l.map (fun _ : ι => (0 : Nat))).sum = 0 :=
      sum_of_all_zero (by
        intro x hx
        obtain ⟨_, _, rfl⟩ := List.mem_map.mp hx
        rfl)
    rw [hz]
    rfl
  | cons bb bs ih =>
    rw [List.map_cons, List.flatten_cons, List.countP_append, ih,
      countP_flatMap_eq_sum]
    rw [show (l.map (fun j =>
        ((List.map (fun bb' => g bb' j) (bb :: bs)).flatten).countP p))
      = l.map (fun j => (g bb j).countP p +
          ((List.map (fun bb' => g bb' j) bs).flatten).countP p) from by
        refine List.map_congr_left ?_
        intro j _
        rw [List.map_cons, List.flatten_cons, List.countP_append]]
    rw [sum_map_add]

/-! ## Distinct-member extraction (the quorum rules) -/

/-- **Provider extraction** (keyed/value form): per-value multiplicities of a
list `L` covered by one-shot per-member capacities yield `L.length`
**distinct** members, each providing some value of `L`. This is the
combinatorial content of "`f+1` collected payloads ⇒ `f+1` distinct cluster
members" — the fact whose absence is FINDINGS.md B1. -/
theorem providers_extract {V : Type _} [DecidableEq V] {n : Nat}
    (L : List V) :
    ∀ (cap : Fin n → V → Nat),
    (∀ j v v', 1 ≤ cap j v → 1 ≤ cap j v' → v = v') →
    (∀ j v, cap j v ≤ 1) →
    (∀ v, L.count v ≤ ((List.finRange n).map (fun j => cap j v)).sum) →
    ∃ S : List (Fin n), S.Nodup ∧ L.length ≤ S.length ∧
      ∀ j ∈ S, ∃ v ∈ L, 1 ≤ cap j v := by
  induction L with
  | nil => exact fun _ _ _ _ => ⟨[], List.nodup_nil, Nat.le_refl 0,
      fun j hj => nomatch hj⟩
  | cons v t ih =>
    intro cap hcap1 hcap2 hcount
    -- find a provider for the head
    have hpos : 0 < ((List.finRange n).map (fun j => cap j v)).sum := by
      have h1 : 1 ≤ (v :: t).count v := by
        rw [List.count_cons_self]
        exact Nat.le_add_left 1 _
      exact Nat.lt_of_lt_of_le h1 (hcount v)
    obtain ⟨j₀, -, hj₀pos⟩ := sum_pos_exists hpos
    -- recurse on the tail with j₀'s capacity zeroed
    have hrec := ih (fun j w => if j = j₀ then 0 else cap j w)
      (fun j w w' hw hw' => by
        by_cases hj : j = j₀
        · rw [if_pos hj] at hw
          cases hw
        · rw [if_neg hj] at hw hw'
          exact hcap1 j w w' hw hw')
      (fun j w => by
        by_cases hj : j = j₀
        · rw [if_pos hj]
          exact Nat.zero_le 1
        · rw [if_neg hj]
          exact hcap2 j w)
      (fun w => by
        by_cases hw : w = v
        · -- head value: the count drops by one, and j₀'s (≥1) capacity is gone
          subst hw
          have hc : t.count w + 1 ≤ ((List.finRange n).map
              (fun j => cap j w)).sum := by
            have := hcount w
            rwa [List.count_cons_self] at this
          have hz := sum_map_zero_at (List.nodup_finRange n)
            (List.mem_finRange j₀) (fun j => cap j w)
          have hj₀1 : 1 ≤ cap j₀ w := hj₀pos
          have hj₀le : cap j₀ w ≤ 1 := hcap2 j₀ w
          omega
        · -- other values: j₀'s capacity for them is zero anyway
          have hc : t.count w ≤ ((List.finRange n).map
              (fun j => cap j w)).sum := by
            have := hcount w
            rwa [List.count_cons_of_ne (fun hc => hw hc.symm)] at this
          have hz := sum_map_zero_at (List.nodup_finRange n)
            (List.mem_finRange j₀) (fun j => cap j w)
          have hj₀0 : cap j₀ w = 0 := by
            by_cases h1 : 1 ≤ cap j₀ w
            · exact absurd (hcap1 j₀ w v h1 hj₀pos) hw
            · omega
          omega)
    obtain ⟨S', hnd', hlen', hprov'⟩ := hrec
    refine ⟨j₀ :: S', ?_, ?_, ?_⟩
    · rw [List.nodup_cons]
      refine ⟨fun hin => ?_, hnd'⟩
      obtain ⟨w, -, hcapw⟩ := hprov' j₀ hin
      rw [if_pos rfl] at hcapw
      cases hcapw
    · rw [List.length_cons]
      exact Nat.succ_le_succ hlen'
    · intro j hj
      rcases List.mem_cons.mp hj with rfl | hjS'
      · exact ⟨v, List.mem_cons_self .., hj₀pos⟩
      · obtain ⟨w, hw, hcapw⟩ := hprov' j hjS'
        by_cases hjj : j = j₀
        · rw [if_pos hjj] at hcapw
          cases hcapw
        · rw [if_neg hjj] at hcapw
          exact ⟨w, List.mem_cons_of_mem _ hw, hcapw⟩

/-- A capacity sum with one-shot members forces that many positive members
(the keyless quorum extraction). -/
theorem filter_pos_length_ge {n : Nat} (c : Fin n → Nat)
    (hc : ∀ j, c j ≤ 1) (m : Nat)
    (hsum : m ≤ ((List.finRange n).map c).sum) :
    m ≤ ((List.finRange n).filter (fun j => decide (0 < c j))).length := by
  have hgen : ∀ l : List (Fin n),
      (l.map c).sum ≤ (l.filter (fun j => decide (0 < c j))).length := by
    intro l
    induction l with
    | nil => exact Nat.le_refl 0
    | cons a t ih =>
      rw [List.map_cons, List.sum_cons, List.filter_cons]
      by_cases ha : 0 < c a
      · rw [if_pos (by simpa using ha), List.length_cons]
        have hca : c a ≤ 1 := hc a
        omega
      · rw [if_neg (by simpa using ha)]
        have hca : c a = 0 := Nat.eq_zero_of_not_pos ha
        rw [hca, Nat.zero_add]
        exact ih
  exact Nat.le_trans hsum (hgen _)

/-- **Distinct-member extraction, family form**: if per-member counts of `P`
over a family (`Fin n → List α`) are one-shot (`≤ 1`) and their sum is at
least `m`, then at least `m` distinct members each have a `P`-element in
their stream. The map-of-streams reading of "a quorum of size `m` exists". -/
theorem family_extract_distinct {α : Type _} {n : Nat}
    (F : Fin n → List α) (p : α → Bool) (m : Nat)
    (hcap : ∀ j, (F j).countP p ≤ 1)
    (hsum : m ≤ ((List.finRange n).map (fun j => (F j).countP p)).sum) :
    ∃ S : List (Fin n), S.Nodup ∧ m ≤ S.length ∧
      ∀ j ∈ S, ∃ x ∈ F j, p x = true := by
  refine ⟨(List.finRange n).filter (fun j => decide (0 < (F j).countP p)),
    (List.nodup_finRange n).filter _,
    filter_pos_length_ge _ hcap m hsum, ?_⟩
  intro j hj
  have hpos : 0 < (F j).countP p := by
    have := (List.mem_filter.mp hj).2
    simpa using this
  obtain ⟨x, hx, hpx⟩ := List.countP_pos_iff.mp hpos
  exact ⟨x, hx, hpx⟩

/-- Count-≤-1 everywhere gives nodup (`countP`/`decide` form, avoiding
`BEq` instance ambiguity on product keys). -/
theorem nodup_of_count_le_one {α : Type _} [DecidableEq α] {l : List α}
    (h : ∀ a, l.countP (fun x => decide (x = a)) ≤ 1) : l.Nodup := by
  induction l with
  | nil => exact List.nodup_nil
  | cons x t ih =>
    refine List.nodup_cons.mpr ⟨fun hx => ?_, ih (fun a => ?_)⟩
    · have hcx := h x
      rw [List.countP_cons, if_pos (by simp)] at hcx
      have hz : t.countP (fun y => decide (y = x)) = 0 := by omega
      have hpos : 0 < t.countP (fun y => decide (y = x)) :=
        List.countP_pos_iff.mpr ⟨x, hx, by simp⟩
      omega
    · have := h a
      rw [List.countP_cons] at this
      omega

/-- A family supported at one member sums to that member's contribution. -/
theorem sum_map_le_single {n : Nat} (g : Fin n → Nat) (i₀ : Fin n) (c : Nat)
    (hz : ∀ i, i ≠ i₀ → g i = 0) (hc : g i₀ ≤ c) :
    ((List.finRange n).map g).sum ≤ c := by
  have hzero := sum_map_zero_at (List.nodup_finRange n)
    (List.mem_finRange i₀) g
  have hz' : ((List.finRange n).map
      (fun j => if j = i₀ then 0 else g j)).sum = 0 := by
    refine sum_of_all_zero ?_
    intro x hx
    obtain ⟨j, -, rfl⟩ := List.mem_map.mp hx
    by_cases hj : j = i₀
    · rw [if_pos hj]
    · rw [if_neg hj]
      exact hz j hj
  omega

/-- `countP` through a `filterMap`: bounded by any predicate the source
elements of hits satisfy. -/
theorem countP_filterMap_le {α β : Type _} {l : List α} {g : α → Option β}
    {p : β → Bool} {q : α → Bool}
    (h : ∀ a ∈ l, ∀ b, g a = some b → p b = true → q a = true) :
    (l.filterMap g).countP p ≤ l.countP q := by
  rw [List.countP_filterMap]
  refine List.countP_mono_left ?_
  intro a ha hpa
  cases hg : g a with
  | none =>
    rw [hg] at hpa
    cases hpa
  | some b =>
    rw [hg] at hpa
    exact h a ha b hg (by simpa using hpa)

/-- The first components of a zip are a prefix of the left input. -/
theorem zip_fst_prefix {α β : Type _} (a : List α) (b : List β) :
    (a.zip b).map Prod.fst <+: a := by
  induction a generalizing b with
  | nil => exact List.nil_prefix
  | cons x xs ih =>
    cases b with
    | nil => exact List.nil_prefix
    | cons y ys =>
      rw [List.zip_cons_cons, List.map_cons]
      exact List.cons_prefix_cons.mpr ⟨rfl, ih ys⟩

/-- The second components of a zip are a prefix of the right input. -/
theorem zip_snd_prefix {α β : Type _} (a : List α) (b : List β) :
    (a.zip b).map Prod.snd <+: b := by
  induction a generalizing b with
  | nil => exact List.nil_prefix
  | cons x xs ih =>
    cases b with
    | nil => exact List.prefix_refl _
    | cons y ys =>
      rw [List.zip_cons_cons, List.map_cons]
      exact List.cons_prefix_cons.mpr ⟨rfl, ih ys⟩

/-- A keys-nodup list has equal entries at equal keys (generic form). -/
theorem eq_of_nodup_keys {α κ : Type _} {l : List α} {g : α → κ}
    (hl : (l.map g).Nodup) {x y : α} (hx : x ∈ l) (hy : y ∈ l)
    (hxy : g x = g y) : x = y := by
  induction l with
  | nil => cases hx
  | cons hd rest ih =>
    rw [List.map_cons, List.nodup_cons] at hl
    rcases List.mem_cons.mp hx with rfl | hxr
    · rcases List.mem_cons.mp hy with rfl | hyr
      · rfl
      · exact absurd (List.mem_map.mpr ⟨y, hyr, hxy.symm⟩) hl.1
    · rcases List.mem_cons.mp hy with rfl | hyr
      · exact absurd (List.mem_map.mpr ⟨x, hxr, hxy⟩) hl.1
      · exact ih hl.2 hxr hyr

/-! ## Quorum intersection (pigeonhole over the member index) -/

/-- A nodup list included in another list is no longer than it. -/
theorem nodup_subset_length_le {α : Type _} [DecidableEq α] :
    ∀ {l l' : List α}, l.Nodup → l ⊆ l' → l.length ≤ l'.length := by
  intro l
  induction l with
  | nil => exact fun _ _ => Nat.zero_le _
  | cons a t ih =>
    intro l' hnd hsub
    have ha : a ∈ l' := hsub (List.mem_cons_self ..)
    have hat : a ∉ t := (List.nodup_cons.mp hnd).1
    have htsub : t ⊆ l'.erase a := by
      intro x hx
      have hxa : x ≠ a := fun hc => hat (hc ▸ hx)
      exact (List.mem_erase_of_ne hxa).mpr (hsub (List.mem_cons_of_mem _ hx))
    have := ih (List.nodup_cons.mp hnd).2 htsub
    have herase : (l'.erase a).length = l'.length - 1 :=
      List.length_erase_of_mem ha
    have hpos : 1 ≤ l'.length := List.length_pos_of_mem ha
    rw [List.length_cons]
    omega

/-- A nodup list over `Fin n` has length at most `n`. -/
theorem nodup_length_le_card {n : Nat} {l : List (Fin n)} (h : l.Nodup) :
    l.length ≤ n := by
  have := nodup_subset_length_le h (fun x _ => List.mem_finRange x)
  simpa using this

/-- **Quorum intersection** (pigeonhole): two nodup member lists with
combined length exceeding the cluster size share a member. -/
theorem nodup_inter_of_length {n : Nat} {S C : List (Fin n)}
    (hS : S.Nodup) (hC : C.Nodup) (hlen : n < S.length + C.length) :
    ∃ j, j ∈ S ∧ j ∈ C := by
  cases Classical.em (∃ j, j ∈ S ∧ j ∈ C) with
  | inl h => exact h
  | inr hno =>
  exfalso
  have hdisj : ∀ j ∈ S, j ∉ C := by
    intro j hjS hjC
    exact hno ⟨j, hjS, hjC⟩
  have hnd : (S ++ C).Nodup := by
    rw [List.nodup_append]
    refine ⟨hS, hC, ?_⟩
    intro a ha bb hbb hab
    exact hdisj a ha (hab ▸ hbb)
  have := nodup_length_le_card hnd
  rw [List.length_append] at this
  omega

end HydroLean.Hydro
