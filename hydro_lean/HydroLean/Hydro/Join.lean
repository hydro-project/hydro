import HydroLean.Hydro.Stream

/-!
# Relational equi-join on streams (Rust: `Stream::join`, `KeyedStream` joins)

The `join` combinator used by `hydro_std::request_response::join_responses`:
join two keyed streams on their keys, pairing the payloads. This file provides
the combinator (computable, on the `List` carrier) and the lemmas the
`join_responses` correctness proof needs:

- `mem_join`: membership characterization (a pair is emitted iff both sides
  contain the corresponding keyed entries);
- `keys_join_nodup`: if both sides have no duplicate keys, neither does the
  join output — the "exactly once" half of the request–response contract.
-/

namespace HydroLean.Hydro.Stream

universe u v w

variable {K : Type u} {M : Type v} {V : Type w} [DecidableEq K]

/-- Rust: `Stream::join` — for each `(k, m)` on the left, emit `(k, (m, v))`
for every `(k, v)` on the right. Left-major order (a canonical representative;
the Rust output is `NoOrder`). -/
def join (l : List (K × M)) (r : List (K × V)) : List (K × (M × V)) :=
  l.flatMap fun (k, m) => (r.filter (fun q => q.1 = k)).map (fun q => (k, (m, q.2)))

@[simp] theorem join_nil (r : List (K × V)) : join ([] : List (K × M)) r = [] := rfl

@[simp] theorem nil_join (l : List (K × M)) : join l ([] : List (K × V)) = [] := by
  simp [join]

theorem join_cons (x : K × M) (l : List (K × M)) (r : List (K × V)) :
    join (x :: l) r
      = (r.filter (fun q => q.1 = x.1)).map (fun q => (x.1, (x.2, q.2))) ++ join l r := by
    simp [join, List.flatMap_cons]

theorem join_append (l₁ l₂ : List (K × M)) (r : List (K × V)) :
    join (l₁ ++ l₂) r = join l₁ r ++ join l₂ r := by
  simp [join, List.flatMap_append]

/-- Membership characterization of the join: `(k, (m, v))` is emitted iff
`(k, m)` is on the left and `(k, v)` is on the right. -/
theorem mem_join {l : List (K × M)} {r : List (K × V)} {k : K} {m : M} {v : V} :
    (k, (m, v)) ∈ join l r ↔ (k, m) ∈ l ∧ (k, v) ∈ r := by
  induction l with
  | nil => simp
  | cons x xs ih =>
    rw [join_cons]
    constructor
    · intro h
      cases List.mem_append.mp h with
      | inl h =>
        obtain ⟨q, hq, heq⟩ := List.mem_map.mp h
        obtain ⟨hqr, hqk⟩ := List.mem_filter.mp hq
        cases heq
        have : q.1 = x.1 := by simpa using hqk
        refine ⟨List.mem_cons_self .., ?_⟩
        · simpa [← this] using hqr
      | inr h =>
        have := ih.mp h
        exact ⟨List.mem_cons_of_mem _ this.1, this.2⟩
    · intro ⟨hl, hr⟩
      cases List.mem_cons.mp hl with
      | inl heq =>
        cases heq
        refine List.mem_append.mpr (.inl ?_)
        exact List.mem_map.mpr ⟨(k, v), List.mem_filter.mpr ⟨hr, by simp⟩, rfl⟩
      | inr h => exact List.mem_append.mpr (.inr (ih.mpr ⟨h, hr⟩))

/-- Keys appearing in the join output come from the left stream. -/
theorem mem_keys_join_left {l : List (K × M)} {r : List (K × V)} {k : K}
    (h : k ∈ (join l r).map Prod.fst) : k ∈ l.map Prod.fst := by
  obtain ⟨q, hq, rfl⟩ := List.mem_map.mp h
  obtain ⟨k', m', v'⟩ := q
  exact List.mem_map.mpr ⟨(k', m'), (mem_join.mp hq).1, rfl⟩

/-- Keys appearing in the join output come from the right stream. -/
theorem mem_keys_join_right {l : List (K × M)} {r : List (K × V)} {k : K}
    (h : k ∈ (join l r).map Prod.fst) : k ∈ r.map Prod.fst := by
  obtain ⟨q, hq, rfl⟩ := List.mem_map.mp h
  obtain ⟨k', m', v'⟩ := q
  exact List.mem_map.mpr ⟨(k', v'), (mem_join.mp hq).2, rfl⟩

/-- With no duplicate keys on the right, filtering by a key yields at most one
element. -/
theorem filter_key_of_nodup {r : List (K × V)} (h : (r.map Prod.fst).Nodup) (k : K) :
    r.filter (fun q => q.1 = k) = [] ∨
      ∃ v, r.filter (fun q => q.1 = k) = [(k, v)] := by
  induction r with
  | nil => exact .inl rfl
  | cons x xs ih =>
    have hx : x.1 ∉ xs.map Prod.fst := (List.nodup_cons.mp h).1
    have hxs : (xs.map Prod.fst).Nodup := (List.nodup_cons.mp h).2
    by_cases hk : x.1 = k
    · refine .inr ⟨x.2, ?_⟩
      have hempty : xs.filter (fun q => q.1 = k) = [] := by
        rw [List.filter_eq_nil_iff]
        intro q hq hqk
        exact hx (by
          subst hk
          exact List.mem_map.mpr ⟨q, hq, by simpa using hqk⟩)
      simp [hk, hempty]
      exact Prod.ext hk rfl
    · cases ih hxs with
      | inl h' => exact .inl (by simp [hk, h'])
      | inr h' =>
        obtain ⟨v, hv⟩ := h'
        exact .inr ⟨v, by simp [hk, hv]⟩

/-- If both sides have duplicate-free keys, so does the join output
("exactly once" delivery of joined pairs, per key). -/
theorem keys_join_nodup {l : List (K × M)} {r : List (K × V)}
    (hl : (l.map Prod.fst).Nodup) (hr : (r.map Prod.fst).Nodup) :
    ((join l r).map Prod.fst).Nodup := by
  induction l with
  | nil => exact .nil
  | cons x xs ih =>
    have hx : x.1 ∉ xs.map Prod.fst := (List.nodup_cons.mp hl).1
    have hxs : (xs.map Prod.fst).Nodup := (List.nodup_cons.mp hl).2
    rw [join_cons, List.map_append]
    have hrest := ih hxs
    have hheadKeys : ∀ k' ∈ ((r.filter (fun q => q.1 = x.1)).map
        (fun q => (x.1, (x.2, q.2)))).map Prod.fst, k' = x.1 := by
      intro k' hk'
      obtain ⟨q, hq, rfl⟩ := List.mem_map.mp hk'
      obtain ⟨q', _, rfl⟩ := List.mem_map.mp hq
      rfl
    have hheadNodup : (((r.filter (fun q => q.1 = x.1)).map
        (fun q => (x.1, (x.2, q.2)))).map Prod.fst).Nodup := by
      cases filter_key_of_nodup hr x.1 with
      | inl h => simp [h]
      | inr h => obtain ⟨v, hv⟩ := h; simp [hv]
    rw [List.nodup_append]
    refine ⟨hheadNodup, hrest, ?_⟩
    intro a ha b hb
    have hax : a = x.1 := hheadKeys a ha
    subst hax
    intro hab
    subst hab
    exact hx (mem_keys_join_left hb)

end HydroLean.Hydro.Stream
