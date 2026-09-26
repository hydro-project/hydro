import HydroV2.MonoRel

/-!
# `hydro_std/src/quorum.rs` · contract vocabulary and pool algebra

The count vocabulary the quorum contracts speak (`cqOkCount` /
`cqKeyCount` / `cqConsumed` and the `Ok`/`Err` projections), its
algebra, and the per-key multiset toolkit the step obligations consume.
The register machine (`CQState`, `cqTick`, `cqwrTick`), its step
obligations, and the program definitions live in `Std/Quorum.lean` —
one definition, no mirror; the run-level inductions are the generic
scan combinators in `Trace.lean`. Consumers operate on the surface
contracts, never on the machine.
-/

namespace HydroV2

variable {K E V : Type} [DecidableEq K]

/-! ## Count vocabulary -/

/-- `Ok` responses of a key in a response pool
(`count_per_key`'s success counter). -/
def cqOkCount (pool : Multiset (K × Except E V)) (k : K) : Nat :=
  (pool.filter (fun r => r.1 = k ∧ r.2.isOk = true)).card

/-- All responses of a key in a response pool
(`count_per_key`'s success + error total). -/
def cqKeyCount (pool : Multiset (K × Except E V)) (k : K) : Nat :=
  (pool.filter (fun r => r.1 = k)).card

/-- The success projection (`filter_map(Ok(v) → Some((key, v)))`). -/
def cqOkProj : (K × Except E V) → Option (K × V)
  | (k, .ok v) => some (k, v)
  | (_, .error _) => none

/-- The error projection (`filter_map(Err(e) → Some((key, e)))`). -/
def cqErrProj : (K × Except E V) → Option (K × E)
  | (_, .ok _) => none
  | (k, .error e) => some (k, e)

/-- The total consumed pool of a realized batch run. -/
def cqConsumed {α : Type _} [DecidableEq α] (pool : Multiset α)
    (d : List (Multiset α)) : Multiset α :=
  (batchCuts pool 0 d).sum

/-- The consumed pool never exceeds the source. -/
theorem cqConsumed_le {α : Type _} [DecidableEq α] (pool : Multiset α)
    (d : List (Multiset α)) : cqConsumed pool d ≤ pool := by
  unfold cqConsumed
  have := batchCuts_sum_le (pool := pool) (d := d) (consumed := 0)
    (Multiset.zero_le _)
  rwa [Multiset.zero_add] at this

theorem cqOkCount_add (a b : Multiset (K × Except E V)) (k : K) :
    cqOkCount (a + b) k = cqOkCount a k + cqOkCount b k := by
  unfold cqOkCount
  rw [Multiset.filter_add, Multiset.card_add]

theorem cqKeyCount_add (a b : Multiset (K × Except E V)) (k : K) :
    cqKeyCount (a + b) k = cqKeyCount a k + cqKeyCount b k := by
  unfold cqKeyCount
  rw [Multiset.filter_add, Multiset.card_add]

theorem cqOkCount_mono {a b : Multiset (K × Except E V)} (h : a ≤ b)
    (k : K) : cqOkCount a k ≤ cqOkCount b k :=
  Multiset.card_le_card (Multiset.filter_le_filter _ h)

theorem cqKeyCount_mono {a b : Multiset (K × Except E V)} (h : a ≤ b)
    (k : K) : cqKeyCount a k ≤ cqKeyCount b k :=
  Multiset.card_le_card (Multiset.filter_le_filter _ h)

theorem cqOkCount_le_keyCount (m : Multiset (K × Except E V)) (k : K) :
    cqOkCount m k ≤ cqKeyCount m k := by
  unfold cqOkCount cqKeyCount
  exact Multiset.card_le_card (Multiset.monotone_filter_right _
    (fun r hr => hr.1))

/-- The key part determines the key's counts. -/
theorem cqOkCount_kpart (m : Multiset (K × Except E V)) (k : K) :
    cqOkCount (m.filter (fun r => r.1 = k)) k = cqOkCount m k := by
  unfold cqOkCount
  rw [Multiset.filter_filter]
  congr 1
  exact Multiset.filter_congr (fun r _ => by
    constructor
    · rintro ⟨⟨h1, h2⟩, -⟩
      exact ⟨h1, h2⟩
    · rintro ⟨h1, h2⟩
      exact ⟨⟨h1, h2⟩, h1⟩)

theorem cqKeyCount_kpart (m : Multiset (K × Except E V)) (k : K) :
    cqKeyCount (m.filter (fun r => r.1 = k)) k = cqKeyCount m k := by
  unfold cqKeyCount
  rw [Multiset.filter_filter]
  congr 1
  exact Multiset.filter_congr (fun r _ => by
    constructor
    · rintro ⟨h1, -⟩
      exact h1
    · intro h1
      exact ⟨h1, h1⟩)

/-- A key is among the pool's keys iff it has a response. -/
theorem cq_mem_keys_iff (m : Multiset (K × Except E V)) (k : K) :
    k ∈ m.map Prod.fst ↔ 0 < cqKeyCount m k := by
  unfold cqKeyCount
  rw [Multiset.card_pos_iff_exists_mem]
  constructor
  · intro h
    obtain ⟨r, hr, hk⟩ := Multiset.mem_map.mp h
    exact ⟨r, Multiset.mem_filter.mpr ⟨hr, hk⟩⟩
  · rintro ⟨r, hr⟩
    have := Multiset.mem_filter.mp hr
    exact Multiset.mem_map.mpr ⟨r, this.1, this.2⟩

/-! ## Multiset filter helpers (the per-key algebra) -/

/-- A key-predicate filter restricted to one key is all-or-nothing. -/
theorem filter_key_of_pred {α : Type _} (m : Multiset (K × α))
    (P : K → Prop) [DecidablePred P] (k : K) :
    (m.filter (fun r => P r.1)).filter (fun r => r.1 = k)
      = if P k then m.filter (fun r => r.1 = k) else 0 := by
  by_cases hp : P k
  · rw [if_pos hp, Multiset.filter_filter]
    exact Multiset.filter_congr (fun r _ => by
      constructor
      · rintro ⟨h1, -⟩
        exact h1
      · intro h1
        exact ⟨h1, by rw [h1]; exact hp⟩)
  · rw [if_neg hp, Multiset.filter_filter, Multiset.filter_eq_nil.mpr]
    rintro r - ⟨h1, h2⟩
    exact hp (h1 ▸ h2)

/-- Count of a key in a filtered dedup of the key projection. -/
theorem count_dedup_keys_filter (m : Multiset (K × Except E V))
    (P : K → Prop) [DecidablePred P] (k : K) :
    (((m.map Prod.fst).dedup).filter P).count k
      = if k ∈ m.map Prod.fst ∧ P k then 1 else 0 := by
  rw [Multiset.count_filter]
  by_cases hp : P k
  · rw [if_pos hp, Multiset.count_dedup]
    by_cases hm : k ∈ m.map Prod.fst
    · rw [if_pos hm, if_pos ⟨hm, hp⟩]
    · rw [if_neg hm, if_neg (fun hc => hm hc.1)]
  · rw [if_neg hp, if_neg (fun hc => hp hc.2)]

/-- A key with an `Ok` vote is among the pool's keys. -/
theorem cq_mem_keys_of_ok {m : Multiset (K × Except E V)} {k : K}
    (h : 1 ≤ cqOkCount m k) : k ∈ m.map Prod.fst := by
  obtain ⟨r, hr⟩ := Multiset.card_pos_iff_exists_mem.mp h
  have := Multiset.mem_filter.mp hr
  exact Multiset.mem_map.mpr ⟨r, this.1, this.2.1⟩

/-- An empty key part kills the key's counts. -/
theorem cqOkCount_eq_zero_of_kpart {m : Multiset (K × Except E V)}
    {k : K} (h : m.filter (fun r => r.1 = k) = 0) :
    cqOkCount m k = 0 := by
  rw [← cqOkCount_kpart, h]
  rfl

omit [DecidableEq K] in
/-- The `Ok` projection counts the `Ok` responses. -/
theorem card_filterMap_okProj [DecidableEq E] [DecidableEq V]
    (m : Multiset (K × Except E V)) :
    (m.filterMap cqOkProj).card
      = (m.filter (fun r => r.2.isOk = true)).card := by
  induction m using Multiset.induction with
  | empty => rfl
  | cons r rest ih =>
    obtain ⟨rk, rres⟩ := r
    cases rres with
    | ok v =>
      rw [Multiset.filterMap_cons_some _ _ _
          (show cqOkProj ((rk, .ok v) : K × Except E V) = some (rk, v)
            from rfl),
        Multiset.card_cons, ih, Multiset.filter_cons,
        if_pos (show ((rk, Except.ok v) : K × Except E V).2.isOk = true
          from rfl),
        Multiset.card_add, Multiset.card_singleton]
      omega
    | error e =>
      rw [Multiset.filterMap_cons_none _ _
          (show cqOkProj ((rk, .error e) : K × Except E V) = none
            from rfl),
        ih, Multiset.filter_cons,
        if_neg (show ¬((rk, Except.error e) : K × Except E V).2.isOk
            = true from fun hc => by cases hc),
        Multiset.zero_add]

/-- The `Ok` projection of a key part counts the key's `Ok` votes. -/
theorem card_okProj_kpart [DecidableEq E] [DecidableEq V]
    (m : Multiset (K × Except E V)) (k : K) :
    ((m.filter (fun r => r.1 = k)).filterMap cqOkProj).card
      = cqOkCount m k := by
  rw [card_filterMap_okProj, Multiset.filter_filter]
  unfold cqOkCount
  congr 1
  exact Multiset.filter_congr (fun r _ => by
    constructor
    · rintro ⟨h2, h1⟩
      exact ⟨h1, h2⟩
    · rintro ⟨h1, h2⟩
      exact ⟨h2, h1⟩)

/-- A pool with zero key count has an empty key part. -/
theorem kpart_eq_zero_of_keyCount {m : Multiset (K × Except E V)}
    {k : K} (h : cqKeyCount m k = 0) :
    m.filter (fun r => r.1 = k) = 0 :=
  Multiset.card_eq_zero.mp h

/-! ## The crossing characterization (under the usage contract)

quorum.rs's deployment assumption ("quorum-of-`max`-participants, each
responding at most once per key"): no key receives more than `max`
responses. Under that cap the registers are per-key functions of the
consumed pool, and each key is emitted exactly at its crossing tick. -/

/-- The drop point: past it, a key's responses have left the window
(emitted at `min = max`; at `received_from_all` otherwise). -/
def cqDropped (min max : Nat) (pfx : Multiset (K × Except E V))
    (k : K) : Prop :=
  (min = max ∧ min ≤ cqOkCount pfx k)
    ∨ (min ≠ max ∧ max ≤ cqKeyCount pfx k)

instance (min max : Nat) (pfx : Multiset (K × Except E V)) (k : K) :
    Decidable (cqDropped min max pfx k) := by
  unfold cqDropped
  infer_instance

theorem cqDropped_of_eq {min max : Nat} (hmm : min = max)
    (pfx : Multiset (K × Except E V)) (k : K) :
    cqDropped min max pfx k ↔ min ≤ cqOkCount pfx k := by
  unfold cqDropped
  constructor
  · rintro (⟨-, h⟩ | ⟨hne, -⟩)
    · exact h
    · exact absurd hmm hne
  · intro h
    exact Or.inl ⟨hmm, h⟩

theorem cqDropped_of_ne {min max : Nat} (hmm : min ≠ max)
    (pfx : Multiset (K × Except E V)) (k : K) :
    cqDropped min max pfx k ↔ max ≤ cqKeyCount pfx k := by
  unfold cqDropped
  constructor
  · rintro (⟨heq, -⟩ | ⟨-, h⟩)
    · exact absurd heq hmm
    · exact h
  · intro h
    exact Or.inr ⟨hmm, h⟩

/-- Same key parts, same key counts. -/
theorem cqOkCount_eq_of_kpart_eq {m m' : Multiset (K × Except E V)}
    {k : K}
    (h : m.filter (fun r => r.1 = k) = m'.filter (fun r => r.1 = k)) :
    cqOkCount m k = cqOkCount m' k := by
  rw [← cqOkCount_kpart, h, cqOkCount_kpart]

theorem cqKeyCount_eq_of_kpart_eq {m m' : Multiset (K × Except E V)}
    {k : K}
    (h : m.filter (fun r => r.1 = k) = m'.filter (fun r => r.1 = k)) :
    cqKeyCount m k = cqKeyCount m' k := by
  rw [← cqKeyCount_kpart, h, cqKeyCount_kpart]

theorem cqKeyCount_eq_zero_of_kpart {m : Multiset (K × Except E V)}
    {k : K} (h : m.filter (fun r => r.1 = k) = 0) :
    cqKeyCount m k = 0 := by
  rw [← cqKeyCount_kpart, h]
  rfl

/-- Count of a key in a doubly filtered dedup of the key projection. -/
theorem count_dedup_keys_filter₂ (m : Multiset (K × Except E V))
    (P Q : K → Prop) [DecidablePred P] [DecidablePred Q] (k : K) :
    ((((m.map Prod.fst).dedup).filter P).filter Q).count k
      = if k ∈ m.map Prod.fst ∧ P k ∧ Q k then 1 else 0 := by
  rw [Multiset.count_filter, count_dedup_keys_filter]
  by_cases hq : Q k
  · by_cases hpm : k ∈ m.map Prod.fst ∧ P k
    · rw [if_pos hpm, if_pos hq, if_pos ⟨hpm.1, hpm.2, hq⟩]
    · rw [if_neg hpm, if_pos hq, if_neg (fun hc => hpm ⟨hc.1, hc.2.1⟩)]
  · rw [if_neg hq, if_neg (fun hc => hq hc.2.2)]

/-! ## `collect_quorum_with_response`: the per-key emission -/

/-- The `Ok` projection commutes with key restriction. -/
theorem filterMap_okProj_kpart_comm [DecidableEq E] [DecidableEq V]
    (X : Multiset (K × Except E V)) (k : K) :
    (X.filterMap cqOkProj).filter (fun r => r.1 = k)
      = (X.filter (fun r => r.1 = k)).filterMap cqOkProj := by
  induction X using Multiset.induction with
  | empty => rfl
  | cons r rest ih =>
    obtain ⟨rk, rres⟩ := r
    cases rres with
    | ok v =>
      rw [Multiset.filterMap_cons_some _ _ _
          (show cqOkProj ((rk, .ok v) : K × Except E V) = some (rk, v)
            from rfl),
        Multiset.filter_cons, Multiset.filter_cons]
      by_cases hk : rk = k
      · rw [if_pos (show ((rk, v) : K × V).1 = k from hk),
          if_pos (show ((rk, Except.ok v) : K × Except E V).1 = k
            from hk),
          Multiset.filterMap_add, ih]
        congr 1
      · rw [if_neg (show ¬((rk, v) : K × V).1 = k from hk),
          if_neg (show ¬((rk, Except.ok v) : K × Except E V).1 = k
            from hk),
          Multiset.filterMap_add, ih]
        congr 1
    | error e =>
      rw [Multiset.filterMap_cons_none _ _
          (show cqOkProj ((rk, .error e) : K × Except E V) = none
            from rfl),
        Multiset.filter_cons]
      by_cases hk : rk = k
      · rw [if_pos (show ((rk, Except.error e) : K × Except E V).1 = k
            from hk),
          Multiset.filterMap_add, ih]
        rw [show Multiset.filterMap cqOkProj
            ({((rk, Except.error e) : K × Except E V)} : Multiset _)
            = 0 from rfl, Multiset.zero_add]
      · rw [if_neg (show ¬((rk, Except.error e) : K × Except E V).1 = k
            from hk),
          Multiset.zero_add, ih]

end HydroV2
