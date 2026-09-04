import Hydro.Values
import Hydro.HydroTick
import Mathlib.Data.Nat.Find

/-!
# `hydro_std/src/quorum.rs` — the shared verified quorum stage

One Rust function = one Lean definition: `collect_quorum`
(quorum.rs:90–160) and `collect_quorum_with_response` (quorum.rs:7–88),
each carrying its colocated contract. Every protocol that collects
quorums consumes THIS module (Paxos phase 1 via
`collect_quorum_with_response` in `p_p1b`, phase 2 via `collect_quorum`
in `sequence_payload`; `two_pc` twice) — the single shared quorum stage.

The one `nondet!` site per collector is its `use::batch` (`BatchCuts`):
"we always persist values that have not reached quorum, so even with
arbitrary batching we always produce deterministic quorum results" —
the contracts below make that comment a theorem. The usage contract
(quorum.rs's deployment assumption, hypotheses of the capped clauses):
`1 ≤ min ≤ max` and no key receives more than `max` responses.

Before the programs: only what their contract faces and loop invariant
name — the count vocabulary (`cqOkCount`/`cqKeyCount`, the `Ok`/`Err`
projections, the keyed counter `cqCount` the bodies fold with, read back
through `keyedFold_cqCount`), the two `Ensures` faces, and the
**register discipline** `CQRegInv` the two `sliced!` blocks share (their
`not_all`/`min_but_not_max` rebinds are the same Rust lines; only the
emission differs) with its two loop lemmas `init`/`step` — the one
deliberate pre-program proof budget (FINDINGS D65, R3; `OnceInv`'s
precedent). **The programs start at `hydro def collect_quorum`**; every
emission fact is a ghost about the construct's own step, read under
`den`, after its block. No register-machine mirror, no body-shape
bridge, no run face (FINDINGS D64 E6/E7).
-/

namespace Hydro

variable {L : Type} {mem : L → Nat}
variable {K E V : Type} [DecidableEq K]

/-! ## Prerequisites for the contract face — the programs start at
`hydro def collect_quorum`

### Count vocabulary -/

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

/-- An empty key part kills the key's counts. -/
theorem cqOkCount_eq_zero_of_kpart {m : Multiset (K × Except E V)}
    {k : K} (h : m.filter (fun r => r.1 = k) = 0) :
    cqOkCount m k = 0 := by
  rw [← cqOkCount_kpart, h]
  rfl

theorem cqKeyCount_eq_zero_of_kpart {m : Multiset (K × Except E V)}
    {k : K} (h : m.filter (fun r => r.1 = k) = 0) :
    cqKeyCount m k = 0 := by
  rw [← cqKeyCount_kpart, h]
  rfl

/-- A pool with zero key count has an empty key part. -/
theorem kpart_eq_zero_of_keyCount {m : Multiset (K × Except E V)}
    {k : K} (h : cqKeyCount m k = 0) :
    m.filter (fun r => r.1 = k) = 0 :=
  Multiset.card_eq_zero.mp h

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

omit [DecidableEq K] in
/-- The `Ok` projection counts the `Ok` responses. -/
theorem card_filterMap_okProj (m : Multiset (K × Except E V)) :
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
theorem card_okProj_kpart (m : Multiset (K × Except E V)) (k : K) :
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

/-- The `Ok` projection commutes with key restriction. -/
theorem filterMap_okProj_kpart_comm (X : Multiset (K × Except E V)) (k : K) :
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

omit [DecidableEq K] in
/-- An `Ok`-projected response is a consumed `Ok` response of its key. -/
theorem okProj_eq_some {r : K × Except E V} {p : K × V}
    (h : cqOkProj r = some p) : r = (p.1, .ok p.2) := by
  obtain ⟨rk, rres⟩ := r
  cases rres with
  | ok v =>
    have hp : ((rk, v) : K × V) = p := Option.some.inj h
    rw [← hp]
  | error e => cases h

/-! ### The keyed counter (quorum.rs's `into_keyed().fold((0,0), …)`)

The bodies fold `cqCount` with the in-tick keyed fold; `keyedFold_cqCount`
reads the fold's entries back into the count vocabulary (the
`commutative = manual_proof!` made honest), and the two derived readers
turn `.filter_map`/`.filter(…).keys()` on the entries into key filters. -/

/-- quorum.rs's per-key counter fold: `(successes, errors)`. -/
def cqCount (accum : Nat × Nat) (value : Except E V) : Nat × Nat :=
  -- if value.is_ok() { accum.0 += 1 } else { accum.1 += 1 }
  if value.isOk then (accum.1 + 1, accum.2) else (accum.1, accum.2 + 1)

/-- `commutative = manual_proof!(/** increment counters is commutative */)` -/
theorem cqCount_comm (s : Nat × Nat) (x y : Except E V) :
    cqCount (cqCount s x) y = cqCount (cqCount s y) x := by
  unfold cqCount
  cases x <;> cases y <;> rfl

/-- The error count of a key (the counter's second component). -/
def cqErrCount (pool : Multiset (K × Except E V)) (k : K) : Nat :=
  (pool.filter (fun r => r.1 = k ∧ ¬ r.2.isOk = true)).card

theorem cqOkCount_add_cqErrCount (pool : Multiset (K × Except E V)) (k : K) :
    cqOkCount pool k + cqErrCount pool k = cqKeyCount pool k := by
  unfold cqOkCount cqErrCount cqKeyCount
  rw [← Multiset.card_add]
  congr 1
  have h := Multiset.filter_add_not (fun r : K × Except E V => r.2.isOk = true)
    (pool.filter (fun r => r.1 = k))
  rw [Multiset.filter_filter, Multiset.filter_filter] at h
  rw [← h]
  congr 1 <;> exact Multiset.filter_congr (fun r _ => and_comm)

/-- The counter fold over a key's values counts its successes and
errors. -/
theorem foldl_cqCount (vs : Multiset (Except E V)) (a b : Nat) :
    @Multiset.foldl (Except E V) (Nat × Nat) cqCount
        ⟨fun s x y => cqCount_comm s x y⟩ (a, b) vs
      = (a + (vs.filter (fun v => v.isOk = true)).card,
         b + (vs.filter (fun v => ¬ v.isOk = true)).card) := by
  induction vs using Multiset.induction_on generalizing a b with
  | empty => simp only [Multiset.foldl_zero, Multiset.filter_zero, Multiset.card_zero, add_zero]
  | cons v vs ih =>
    rw [Multiset.foldl_cons, Multiset.filter_cons, Multiset.filter_cons]
    by_cases hv : v.isOk = true
    · have hc : cqCount (a, b) v = (a + 1, b) := by
        unfold cqCount; rw [if_pos hv]
      rw [hc, ih, if_pos hv, if_neg (not_not.mpr hv)]
      simp only [Multiset.card_add, Multiset.card_singleton, Multiset.card_zero]
      congr 1 <;> omega
    · have hc : cqCount (a, b) v = (a, b + 1) := by
        unfold cqCount; rw [if_neg hv]
      rw [hc, ih, if_neg hv, if_pos hv]
      simp only [Multiset.card_add, Multiset.card_singleton, Multiset.card_zero]
      congr 1 <;> omega

/-- The keyed counter fold's entries: every key of the pool once, with
its `(successes, errors)`. -/
theorem keyedFold_cqCount (pool : Multiset (K × Except E V)) :
    keyedFoldMultiset cqCount (0, 0) (fun s x y => cqCount_comm s x y) pool
      = (pool.map Prod.fst).dedup.map
          (fun k => (k, (cqOkCount pool k, cqErrCount pool k))) := by
  unfold keyedFoldMultiset
  congr 1
  funext k
  rw [foldl_cqCount, Nat.zero_add, Nat.zero_add]
  unfold cqOkCount cqErrCount
  rw [Multiset.filter_map, Multiset.filter_map, Multiset.card_map,
    Multiset.card_map, Multiset.filter_filter, Multiset.filter_filter]
  congr 3 <;> exact Multiset.filter_congr (fun r _ => and_comm)

/-- `.entries().filter_map(|(key, (success, _))| if success >= min
{Some(key)} else {None})` is the keys at `min` successes. -/
theorem cq_reached_eq (min : Nat) (pool : Multiset (K × Except E V)) :
    Multiset.filterMap (fun e : K × (Nat × Nat) =>
        if min ≤ e.2.1 then some e.1 else none)
      (keyedFoldMultiset cqCount (0, 0) (fun s x y => cqCount_comm s x y) pool)
      = (pool.map Prod.fst).dedup.filter (fun k => min ≤ cqOkCount pool k) := by
  rw [keyedFold_cqCount, Multiset.filterMap_map, ← Multiset.filterMap_eq_filter]
  congr 1
  funext k
  simp only [Function.comp_apply, Option.guard, decide_eq_true_eq]

/-- `.filter(p).keys()` on the counter's entries is the keys at the
predicate on their counts. -/
theorem cq_keys_filter_eq (q : K × (Nat × Nat) → Prop) [DecidablePred q]
    (pool : Multiset (K × Except E V)) :
    Multiset.map Prod.fst (Multiset.filter q
      (keyedFoldMultiset cqCount (0, 0) (fun s x y => cqCount_comm s x y) pool))
      = (pool.map Prod.fst).dedup.filter
          (fun k => q (k, (cqOkCount pool k, cqErrCount pool k))) := by
  rw [keyedFold_cqCount, Multiset.filter_map, Multiset.map_map]
  exact Multiset.map_id' _

/-- `anti_join` against a filtered key set of the pool itself (read under
`den`): a plain filter on the complement. -/
theorem filter_notmem_keys_filter (pool : Multiset (K × Except E V))
    (q : K → Prop) [DecidablePred q] :
    pool.filter (fun r => r.1 ∉ (pool.map Prod.fst).dedup.filter q)
      = pool.filter (fun r => ¬ q r.1) := by
  apply Multiset.filter_congr
  intro r hr
  rw [Multiset.mem_filter]
  exact ⟨fun h hq => h ⟨mem_keys_of_mem hr, hq⟩, fun h hh => h hh.2⟩

/-- `filter_not_in` between two filtered key sets of one pool (read under
`den`): a filter on the complement. -/
theorem keys_filter_notmem_keys_filter (pool : Multiset (K × Except E V))
    (q q' : K → Prop) [DecidablePred q] [DecidablePred q'] :
    ((pool.map Prod.fst).dedup.filter q).filter
        (fun k => k ∉ (pool.map Prod.fst).dedup.filter q')
      = ((pool.map Prod.fst).dedup.filter q).filter (fun k => ¬ q' k) := by
  apply Multiset.filter_congr
  intro k hk
  rw [Multiset.mem_filter]
  exact ⟨fun h hq => h ⟨(Multiset.mem_filter.mp hk).1, hq⟩, fun h hh => h hh.2⟩

/-! ### The contract faces -/

section Faces
variable [DecidableEq E] [DecidableEq V]

/-- What `collect_quorum` **ensures**, over the `Values` denotation.
`cqConsumed (resp i) (dec i)` is member `i`'s realized consumed pool. -/
structure CQEnsures (ℓ : L) (min max : Nat)
    (resp : Fin (mem ℓ) → Multiset (K × Except E Unit))
    (dec : BatchCuts (mem ℓ) (K × Except E Unit))
    (out : (Fin (mem ℓ) → Multiset K)
      × (Fin (mem ℓ) → Multiset (K × E))) : Prop where
  /-- **Soundness** (unconditional): an emitted key holds `min` `Ok`
  votes among the consumed responses. -/
  emit_sound : ∀ (i : Fin (mem ℓ)), ∀ k ∈ out.1 i,
    min ≤ cqOkCount (cqConsumed (resp i) (dec i)) k
  /-- **The crossing count** (under the usage contract): a key is
  emitted **iff** it reached `min` `Ok` votes among the consumed pool —
  exactly once (the emission multiset's count is the indicator). -/
  emit_count : ∀ (i : Fin (mem ℓ)) (k : K), 1 ≤ min → min ≤ max →
    cqKeyCount (cqConsumed (resp i) (dec i)) k ≤ max →
    (out.1 i).count k
      = if min ≤ cqOkCount (cqConsumed (resp i) (dec i)) k
        then 1 else 0
  /-- **The error leg is pure**: the fails quote the raw stream's `Err`
  responses (no clock, no decision). -/
  fails_eq : ∀ (i : Fin (mem ℓ)),
    out.2 i = (resp i).filterMap cqErrProj

/-- What `collect_quorum_with_response` **ensures**, over the `Values`
denotation. -/
structure CQWREnsures (ℓ : L) (min max : Nat)
    (resp : Fin (mem ℓ) → Multiset (K × Except E V))
    (dec : BatchCuts (mem ℓ) (K × Except E V))
    (out : (Fin (mem ℓ) → Multiset (K × V))
      × (Fin (mem ℓ) → Multiset (K × E))) : Prop where
  /-- **Membership soundness** (unconditional): an emitted response
  quotes a consumed `Ok` response of a key holding `min` votes among
  the consumed pool. -/
  emit_mem_sound : ∀ (i : Fin (mem ℓ)) {k : K} {v : V},
    (k, v) ∈ out.1 i →
    (k, .ok v) ∈ cqConsumed (resp i) (dec i)
      ∧ min ≤ cqOkCount (cqConsumed (resp i) (dec i)) k
  /-- **Multiplicity soundness** (under the usage contract): a key's
  emissions embed in its consumed `Ok` responses **with
  multiplicity** — nothing is emitted twice. -/
  emit_le : ∀ (i : Fin (mem ℓ)) (k : K), 1 ≤ min → min ≤ max →
    cqKeyCount (cqConsumed (resp i) (dec i)) k ≤ max →
    (out.1 i).filter (fun r => r.1 = k)
      ≤ ((cqConsumed (resp i) (dec i)).filter
          (fun r => r.1 = k)).filterMap cqOkProj
  /-- **Completeness** (under the usage contract): a key reaching
  `min` `Ok` votes among the consumed pool emits at least `min`
  responses. -/
  emit_complete : ∀ (i : Fin (mem ℓ)) (k : K), 1 ≤ min → min ≤ max →
    cqKeyCount (cqConsumed (resp i) (dec i)) k ≤ max →
    min ≤ cqOkCount (cqConsumed (resp i) (dec i)) k →
    min ≤ ((out.1 i).filter (fun r => r.1 = k)).card
  /-- **Pool soundness** (unconditional, `fails_eq`'s analogue on the
  success leg): the emission pool embeds **with multiplicity** in the
  `Ok` projection of the consumed pool — everything emitted quotes a
  distinct consumed `Ok` response. -/
  emit_pool_le : ∀ (i : Fin (mem ℓ)),
    out.1 i ≤ (cqConsumed (resp i) (dec i)).filterMap cqOkProj
  /-- **The error leg is pure**: the fails quote the raw stream's `Err`
  responses. -/
  fails_eq : ∀ (i : Fin (mem ℓ)),
    out.2 i = (resp i).filterMap cqErrProj

end Faces

/-! ### The register discipline — the loop invariant both blocks share

quorum.rs's deployment assumption ("quorum-of-`max`-participants, each
responding at most once per key"): no key receives more than `max`
responses. Under that cap the two `sliced!` registers are per-key
functions of the consumed prefix: `not_all` holds exactly the key's
consumed responses until its DROP point (emitted at `min = max`;
`received_from_all` otherwise), and `min_but_not_max` names exactly the
keys at `min` `Ok`s but below `max` responses. The `rebind` lines of
`collect_quorum` (quorum.rs:116–151) and `collect_quorum_with_response`
(quorum.rs:38–75) are the same; `CQRegInv.step` is their one loop-body
proof, consumed by both blocks' `prove tick`. -/

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

/-- Dropping is permanent (the counts only grow). -/
theorem cqDropped_mono {min max : Nat} {pfx pfx' : Multiset (K × Except E V)}
    (h : pfx ≤ pfx') {k : K} (hd : cqDropped min max pfx k) :
    cqDropped min max pfx' k := by
  unfold cqDropped at hd ⊢
  rcases hd with ⟨hmm, hok⟩ | ⟨hmm, hkey⟩
  · exact Or.inl ⟨hmm, le_trans hok (cqOkCount_mono h k)⟩
  · exact Or.inr ⟨hmm, le_trans hkey (cqKeyCount_mono h k)⟩

/-- **The register discipline** of the quorum `sliced!` blocks after
consuming `pfx`: the window sits below the consumed pool; under the
usage contract, per key, the window holds exactly the key's consumed
responses until the drop point, and `min_but_not_max` names exactly the
keys at `min` `Ok`s but below `max` responses. -/
structure CQRegInv (min max : Nat) (notAll : Multiset (K × Except E V))
    (mbnm : Multiset K) (pfx : Multiset (K × Except E V)) : Prop where
  /-- The window never exceeds the consumed responses. -/
  window_le : notAll ≤ pfx
  /-- (capped) The window's key part is the consumed key part until the
  drop point, then empty. -/
  window : 1 ≤ min → min ≤ max → ∀ k, cqKeyCount pfx k ≤ max →
    notAll.filter (fun r => r.1 = k)
      = if cqDropped min max pfx k then 0 else pfx.filter (fun r => r.1 = k)
  /-- (capped) `min_but_not_max` names the keys at `min` `Ok`s below
  `max` responses (used only when `min < max`). -/
  locked : 1 ≤ min → min < max → ∀ k, cqKeyCount pfx k ≤ max →
    (k ∈ mbnm ↔ (min ≤ cqOkCount pfx k ∧ cqKeyCount pfx k < max))

namespace CQRegInv

variable {min max : Nat}

/-- `use::state_null` is good at the empty consumption. -/
theorem init : CQRegInv (K := K) (E := E) (V := V) min max 0 0 0 := by
  refine ⟨le_refl _, fun _ _ k _ => ?_, fun h1 _ k _ => ?_⟩
  · rw [Multiset.filter_zero]
    by_cases hd : cqDropped min max (0 : Multiset (K × Except E V)) k
    · rw [if_pos hd]
    · rw [if_neg hd]
  · constructor
    · intro hc
      exact absurd hc (Multiset.notMem_zero k)
    · rintro ⟨hok, -⟩
      refine absurd (le_trans h1 hok) ?_
      show ¬ 1 ≤ cqOkCount 0 k
      unfold cqOkCount
      rw [Multiset.filter_zero, Multiset.card_zero]
      omega

/-- Under the cap, a dropped key receives no further responses. -/
theorem dropped_kills_batch {pfx b : Multiset (K × Except E V)} {k : K}
    (hcap : cqKeyCount (pfx + b) k ≤ max)
    (hd : cqDropped min max pfx k) :
    b.filter (fun r => r.1 = k) = 0 := by
  refine kpart_eq_zero_of_keyCount ?_
  rw [cqKeyCount_add] at hcap
  have hol := cqOkCount_le_keyCount pfx k
  unfold cqDropped at hd
  rcases hd with ⟨hmm, hok⟩ | ⟨-, hkey⟩ <;> omega

/-- The tick's window at a key: the consumed key part when not dropped,
empty (under the cap) when dropped. -/
theorem window_kpart {notAll pfx b : Multiset (K × Except E V)} {mbnm : Multiset K} {k : K}
    (h1 : 1 ≤ min) (hmm : min ≤ max)
    (hcap : cqKeyCount (pfx + b) k ≤ max)
    (ih : CQRegInv min max notAll mbnm pfx) :
    (notAll + b).filter (fun r => r.1 = k)
      = if cqDropped min max pfx k then 0 else (pfx + b).filter (fun r => r.1 = k) := by
  have hcapp : cqKeyCount pfx k ≤ max :=
    le_trans (cqKeyCount_mono (Multiset.le_add_right _ _) k) hcap
  rw [Multiset.filter_add, ih.window h1 hmm k hcapp]
  by_cases hd : cqDropped min max pfx k
  · rw [if_pos hd, if_pos hd, dropped_kills_batch hcap hd]
    rfl
  · rw [if_neg hd, if_neg hd, Multiset.filter_add]

/-- **ONE TICK, `min = max`** (quorum.rs:116–118 / :38–40): the window
drops the keys that just reached `min`; `min_but_not_max` is untouched. -/
theorem step_eq {notAll pfx b : Multiset (K × Except E V)} {mbnm : Multiset K}
    (hmm : min = max) (ih : CQRegInv min max notAll mbnm pfx) :
    CQRegInv min max
      ((notAll + b).filter (fun r => ¬ min ≤ cqOkCount (notAll + b) r.1))
      mbnm (pfx + b) := by
  refine ⟨?_, fun h1 hmx k hcap => ?_, fun _ hlt _ _ => absurd hmm (Nat.ne_of_lt hlt)⟩
  · exact le_trans (Multiset.filter_le _ _) (add_le_add ih.window_le (le_refl _))
  · rw [filter_key_of_pred _ (fun k' => ¬ min ≤ cqOkCount (notAll + b) k') k,
      window_kpart h1 hmx hcap ih]
    by_cases hd : cqDropped min max pfx k
    · -- dropped before: the window is empty and stays dropped
      have hd' : cqDropped min max (pfx + b) k :=
        cqDropped_mono (Multiset.le_add_right _ _) hd
      rw [if_pos hd, if_pos hd']
      split_ifs <;> rfl
    · -- the key's counts are those of the consumed pool
      have hk : cqOkCount (notAll + b) k = cqOkCount (pfx + b) k :=
        cqOkCount_eq_of_kpart_eq (by rw [window_kpart h1 hmx hcap ih, if_neg hd])
      rw [hk, if_neg hd]
      by_cases hc : min ≤ cqOkCount (pfx + b) k
      · rw [if_neg (not_not_intro hc), if_pos ((cqDropped_of_eq hmm _ k).mpr hc)]
      · rw [if_pos hc, if_neg (fun hd' => hc ((cqDropped_of_eq hmm _ k).mp hd'))]

/-- **ONE TICK, `min < max`** (quorum.rs:119–151 / :41–75): the window
drops the keys heard from everyone; `min_but_not_max` becomes the keys
at `min` `Ok`s not heard from everyone. -/
theorem step_ne {notAll pfx b : Multiset (K × Except E V)} {mbnm : Multiset K}
    (hmm : min ≠ max) (ih : CQRegInv min max notAll mbnm pfx) :
    CQRegInv min max
      ((notAll + b).filter (fun r => ¬ max ≤ cqKeyCount (notAll + b) r.1))
      ((((notAll + b).map Prod.fst).dedup.filter
          (fun k => min ≤ cqOkCount (notAll + b) k)).filter
        (fun k => ¬ max ≤ cqKeyCount (notAll + b) k))
      (pfx + b) := by
  refine ⟨?_, fun h1 hmx k hcap => ?_, fun h1 hlt k hcap => ?_⟩
  · exact le_trans (Multiset.filter_le _ _) (add_le_add ih.window_le (le_refl _))
  · rw [filter_key_of_pred _ (fun k' => ¬ max ≤ cqKeyCount (notAll + b) k') k,
      window_kpart h1 hmx hcap ih]
    by_cases hd : cqDropped min max pfx k
    · have hd' : cqDropped min max (pfx + b) k :=
        cqDropped_mono (Multiset.le_add_right _ _) hd
      rw [if_pos hd, if_pos hd']
      split_ifs <;> rfl
    · have hk : cqKeyCount (notAll + b) k = cqKeyCount (pfx + b) k :=
        cqKeyCount_eq_of_kpart_eq (by rw [window_kpart h1 hmx hcap ih, if_neg hd])
      rw [hk, if_neg hd]
      by_cases hc : max ≤ cqKeyCount (pfx + b) k
      · rw [if_neg (not_not_intro hc), if_pos ((cqDropped_of_ne hmm _ k).mpr hc)]
      · rw [if_pos hc, if_neg (fun hd' => hc ((cqDropped_of_ne hmm _ k).mp hd'))]
  · have hmx : min ≤ max := Nat.le_of_lt hlt
    rw [Multiset.mem_filter, Multiset.mem_filter, Multiset.mem_dedup, cq_mem_keys_iff]
    by_cases hd : cqDropped min max pfx k
    · -- dropped before: no responses in the window, none arrive, and the
      -- consumed pool is already at `max`
      have hcur : (notAll + b).filter (fun r => r.1 = k) = 0 := by
        rw [window_kpart h1 hmx hcap ih, if_pos hd]
      have hkc : cqKeyCount (notAll + b) k = 0 := cqKeyCount_eq_zero_of_kpart hcur
      have hfull : max ≤ cqKeyCount (pfx + b) k :=
        le_trans ((cqDropped_of_ne hmm pfx k).mp hd)
          (cqKeyCount_mono (Multiset.le_add_right _ _) k)
      constructor
      · rintro ⟨⟨hpos, -⟩, -⟩
        rw [hkc] at hpos
        exact absurd hpos (Nat.lt_irrefl 0)
      · rintro ⟨-, hlt'⟩
        exact absurd hfull (Nat.not_le_of_lt hlt')
    · have hcur : (notAll + b).filter (fun r => r.1 = k)
          = (pfx + b).filter (fun r => r.1 = k) := by
        rw [window_kpart h1 hmx hcap ih, if_neg hd]
      have hok : cqOkCount (notAll + b) k = cqOkCount (pfx + b) k :=
        cqOkCount_eq_of_kpart_eq hcur
      have hkey : cqKeyCount (notAll + b) k = cqKeyCount (pfx + b) k :=
        cqKeyCount_eq_of_kpart_eq hcur
      rw [hok, hkey]
      constructor
      · rintro ⟨⟨-, hc⟩, hn⟩
        exact ⟨hc, Nat.lt_of_not_le hn⟩
      · rintro ⟨hc, hlt'⟩
        refine ⟨⟨?_, hc⟩, Nat.not_le_of_lt hlt'⟩
        exact Nat.lt_of_lt_of_le (Nat.lt_of_lt_of_le Nat.zero_lt_one (le_trans h1 hc))
          (cqOkCount_le_keyCount _ k)

end CQRegInv

/-! ## The programs -/

section Programs
variable [DecidableEq E] [DecidableEq V]

/-- **quorum.rs:90–160 `collect_quorum`** over location `ℓ`: emit each
key once as it reaches `min` successful responses (persisting
below-quorum responses across arbitrary batching), and surface every
error. Returns (`just_reached_quorum`, `fails`).

Rust `nondet!` tally: 1 (the `use::batch`). -/
hydro def collect_quorum (H : HydroSem L mem) (ℓ : L)
    (responses : H.Stream ℓ (K × Except E Unit) .noOrder .exactlyOnce)
    (min max : Nat)
    (dec : H.BatchDec (mem ℓ) (K × Except E Unit)) :
    (H.Stream ℓ K .noOrder .exactlyOnce
      × H.Stream ℓ (K × E) .noOrder .exactlyOnce)
  ensures out => CQEnsures ℓ min max responses dec out :=
  -- let just_reached_quorum = sliced! {
  --   let new_inputs = use::batch(responses.clone(), nondet!(…));
  --   let mut not_all = use::state_null::<Stream<_, _, Bounded, Order>>();
  --   let mut min_but_not_max = use::state_null::<Stream<K, _, Bounded, NoOrder>>();
  tick (state not_all : H.BoundedStream (K × Except E Unit) .noOrder .exactlyOnce)
      (state min_but_not_max : H.BoundedStream K .noOrder .exactlyOnce)
      (input new_inputs := H.batch responses dec)
      -- the loop invariant: the register discipline against the
      -- responses consumed so far
      (invariant ((just_reached_quorum : List (Multiset K))
          (not_all : Multiset (K × Except E Unit)) (min_but_not_max : Multiset K)
          (new_inputs : Trace (Multiset (K × Except E Unit)))) =>
        CQRegInv min max not_all min_but_not_max
          ((new_inputs.take just_reached_quorum.length).sum)) :=
    -- let current_responses = not_all.chain(new_inputs);
    let current_responses := H.bchain not_all new_inputs
    -- let count_per_key = current_responses.clone().into_keyed().fold(
    --   q!(move || (0, 0)),
    --   q!(move |accum, value| { if value.is_ok() { accum.0 += 1; } else { accum.1 += 1; } },
    --      commutative = manual_proof!(/** increment counters is commutative */)));
    let count_per_key := H.bkeyedFold cqCount (0, 0)
      (fun s x y => cqCount_comm s x y) current_responses
    -- let reached_min_count = count_per_key.clone().entries()
    --   .filter_map(q!(move |(key, (success, _error))| if success >= min { Some(key) } else { None }));
    let reached_min_count := H.bfilterMap count_per_key
      (fun (key, (success, _error)) => if min ≤ success then some key else none)
    -- let just_reached_quorum = if max == min {
    --   not_all = current_responses.anti_join(reached_min_count.clone());
    --   reached_min_count
    -- } else {
    let branch := if max = min then
        (H.bantiJoin current_responses reached_min_count,
         (min_but_not_max, reached_min_count))
      else
        -- let received_from_all = count_per_key
        --   .filter(q!(move |(success, error)| (success + error) >= max)).keys();
        let received_from_all := H.bkeys (H.bfilter count_per_key
          (fun (_key, (success, error)) => decide (max ≤ success + error)))
        -- not_all = current_responses.anti_join(received_from_all.clone());
        -- let out = reached_min_count.clone().filter_not_in(min_but_not_max);
        -- min_but_not_max = reached_min_count.filter_not_in(received_from_all);
        -- out
        (H.bantiJoin current_responses received_from_all,
         (H.bfilterNotIn reached_min_count received_from_all,
          H.bfilterNotIn reached_min_count min_but_not_max))
    -- };
    rebind (not_all := branch.1, min_but_not_max := branch.2.1)
    -- just_reached_quorum
    yield (just_reached_quorum := branch.2.2)
    -- the loop obligations: the empty run, and ONE TICK (the shared
    -- register step, at `min = max` or `min < max`)
    prove init := fun _i => by
        simp only [List.length_nil, List.take_zero, List.sum_nil, ValuesTick.seed_pair,
          ValuesTick.seed_stream, den]
        exact CQRegInv.init,
      tick := fun i n out st b_t hb hlen ih => by
        simp only [List.append_eq, List.length_append, List.length_singleton, hlen,
          List.take_add_one, hb, Option.toList_some, List.sum_append, List.sum_singleton] at ih ⊢
        simp only [just_reached_quorum_step, den]
        by_cases hmm : max = min
        · simp only [if_pos hmm, cq_reached_eq, Bool.not_eq_true', decide_eq_false_iff_not,
            filter_notmem_keys_filter]
          exact CQRegInv.step_eq hmm.symm ih
        · simp only [if_neg hmm, cq_reached_eq, cq_keys_filter_eq, decide_eq_true_eq,
            cqOkCount_add_cqErrCount, Bool.not_eq_true', decide_eq_false_iff_not,
            filter_notmem_keys_filter, keys_filter_notmem_keys_filter]
          exact CQRegInv.step_ne (Ne.symm hmm) ih;
  -- };
  -- **one tick, decoded** (the Rust lines at the denotation): tick `n`'s
  -- emission is the keys of the tick's window (`not_all` before `n` plus
  -- this tick's batch) at `min` `Ok`s — at `min < max`, those not already
  -- in `min_but_not_max` — and the registers before `n` carry the
  -- discipline against the responses consumed before `n`
  ghost have htick : ∀ (i : Fin (mem ℓ)) (n : Nat) (e : Multiset K),
      (just_reached_quorum i)[n]? = some e →
      ∃ (b_t : Multiset (K × Except E Unit)) (S : Multiset (K × Except E Unit) × Multiset K),
        (new_inputs i)[n]? = some b_t
        ∧ CQRegInv min max S.1 S.2 ((new_inputs i).take n).sum
        ∧ e = (if max = min then
            ((S.1 + b_t).map Prod.fst).dedup.filter (fun k => min ≤ cqOkCount (S.1 + b_t) k)
          else
            (((S.1 + b_t).map Prod.fst).dedup.filter (fun k => min ≤ cqOkCount (S.1 + b_t) k)).filter
              (fun k => k ∉ S.2)) := fun i n e he => by
    have hn : n < (just_reached_quorum i).length := Trace.read_lt he
    obtain ⟨b_t, hb, rfl⟩ := (hjust_reached_quorum_at i n e).mp he
    have h := hjust_reached_quorum_inv_take i n
    simp only [List.length_take_of_le (Nat.le_of_lt hn)] at h
    refine ⟨b_t, _, hb, h, ?_⟩
    simp only [just_reached_quorum_step, den]
    by_cases hmm : max = min
    · simp only [if_pos hmm, cq_reached_eq]
    · simp only [if_neg hmm, cq_reached_eq, Bool.not_eq_true', decide_eq_false_iff_not]
  -- the responses consumed through tick `n` sit below the consumed pool
  ghost have hpfx_le : ∀ (i : Fin (mem ℓ)) (n : Nat),
      ((new_inputs i).take n).sum ≤ cqConsumed (responses i) (dec i) := fun i n =>
    sublist_sum_le (List.take_sublist _ _)
  -- **one tick's crossing count** (under the usage contract): tick `n`
  -- emits `k` exactly when `k` reaches `min` `Ok`s on this tick
  ghost have hcount_tick : ∀ (i : Fin (mem ℓ)) (n : Nat) (e : Multiset K) (k : K),
      1 ≤ min → min ≤ max → cqKeyCount ((new_inputs i).take (n + 1)).sum k ≤ max →
      (just_reached_quorum i)[n]? = some e →
      e.count k = if min ≤ cqOkCount ((new_inputs i).take (n + 1)).sum k
          ∧ ¬ min ≤ cqOkCount ((new_inputs i).take n).sum k then 1 else 0 :=
    fun i n e k h1 hmx hcap he => by
    obtain ⟨b_t, S, hb, hS, rfl⟩ := htick i n e he
    have hsum : ((new_inputs i).take (n + 1)).sum = ((new_inputs i).take n).sum + b_t := by
      rw [List.take_add_one, hb, List.sum_append]
      simp only [Option.toList_some, List.sum_singleton]
    rw [hsum] at hcap ⊢
    set pfx := ((new_inputs i).take n).sum with hpfx
    have hcur := CQRegInv.window_kpart (b := b_t) (k := k) h1 hmx hcap hS
    have hcapp : cqKeyCount pfx k ≤ max :=
      le_trans (cqKeyCount_mono (Multiset.le_add_right _ _) k) hcap
    by_cases hmm : max = min
    · rw [if_pos hmm, count_dedup_keys_filter]
      by_cases hd : cqDropped min max pfx k
      · -- dropped before: no responses in the window, nothing to emit
        have hok0 : cqOkCount (S.1 + b_t) k = 0 := by
          rw [← cqOkCount_kpart, hcur, if_pos hd]; rfl
        have hokp : min ≤ cqOkCount pfx k := (cqDropped_of_eq hmm.symm pfx k).mp hd
        rw [if_neg (fun hc => by rw [hok0] at hc; omega), if_neg (fun hc => hc.2 hokp)]
      · have hok : cqOkCount (S.1 + b_t) k = cqOkCount (pfx + b_t) k :=
          cqOkCount_eq_of_kpart_eq (by rw [hcur, if_neg hd])
        have hkey : cqKeyCount (S.1 + b_t) k = cqKeyCount (pfx + b_t) k :=
          cqKeyCount_eq_of_kpart_eq (by rw [hcur, if_neg hd])
        have hnot : ¬ min ≤ cqOkCount pfx k := fun hc => hd ((cqDropped_of_eq hmm.symm pfx k).mpr hc)
        simp only [cq_mem_keys_iff, hok, hkey]
        by_cases hc : min ≤ cqOkCount (pfx + b_t) k
        · rw [if_pos ⟨Nat.lt_of_lt_of_le (Nat.lt_of_lt_of_le Nat.zero_lt_one (le_trans h1 hc))
            (cqOkCount_le_keyCount _ k), hc⟩, if_pos ⟨hc, hnot⟩]
        · rw [if_neg (fun hx => hc hx.2), if_neg (fun hx => hc hx.1)]
    · have hlt : min < max := Nat.lt_of_le_of_ne hmx (Ne.symm hmm)
      rw [if_neg hmm, count_dedup_keys_filter₂]
      simp only [hS.locked h1 hlt k hcapp]
      by_cases hd : cqDropped min max pfx k
      · have hok0 : cqOkCount (S.1 + b_t) k = 0 := by
          rw [← cqOkCount_kpart, hcur, if_pos hd]; rfl
        have hfull : max ≤ cqKeyCount pfx k := (cqDropped_of_ne (Ne.symm hmm) pfx k).mp hd
        have hb0 : b_t.filter (fun r => r.1 = k) = 0 := CQRegInv.dropped_kills_batch hcap hd
        have hokb : cqOkCount (pfx + b_t) k = cqOkCount pfx k := by
          rw [cqOkCount_add, cqOkCount_eq_zero_of_kpart hb0, Nat.add_zero]
        rw [if_neg (fun hc => by have := hc.2.1; rw [hok0] at this; omega)]
        by_cases hokp : min ≤ cqOkCount pfx k
        · rw [if_neg (fun hc => hc.2 hokp)]
        · rw [if_neg (fun hc => hokp (hokb ▸ hc.1))]
      · have hok : cqOkCount (S.1 + b_t) k = cqOkCount (pfx + b_t) k :=
          cqOkCount_eq_of_kpart_eq (by rw [hcur, if_neg hd])
        have hkey : cqKeyCount (S.1 + b_t) k = cqKeyCount (pfx + b_t) k :=
          cqKeyCount_eq_of_kpart_eq (by rw [hcur, if_neg hd])
        have hlt' : cqKeyCount pfx k < max :=
          Nat.lt_of_not_le (fun hc => hd ((cqDropped_of_ne (Ne.symm hmm) pfx k).mpr hc))
        simp only [cq_mem_keys_iff, hok, hkey]
        by_cases hc : min ≤ cqOkCount (pfx + b_t) k
        · by_cases hokp : min ≤ cqOkCount pfx k
          · rw [if_neg (fun hx => hx.2.2 ⟨hokp, hlt'⟩), if_neg (fun hx => hx.2 hokp)]
          · rw [if_pos ⟨Nat.lt_of_lt_of_le (Nat.lt_of_lt_of_le Nat.zero_lt_one (le_trans h1 hc))
              (cqOkCount_le_keyCount _ k), hc, fun hx => hokp hx.1⟩, if_pos ⟨hc, hokp⟩]
        · rw [if_neg (fun hx => hc hx.2.1), if_neg (fun hx => hc hx.1)]
  -- **the crossing count over a run prefix**: summing the per-tick
  -- indicators, `k` is emitted once iff it has reached `min` so far
  ghost have hcount : ∀ (i : Fin (mem ℓ)) (k : K), 1 ≤ min → min ≤ max →
      ∀ (n : Nat), n ≤ (just_reached_quorum i).length →
      cqKeyCount ((new_inputs i).take n).sum k ≤ max →
      ((just_reached_quorum i).take n).sum.count k
        = if min ≤ cqOkCount ((new_inputs i).take n).sum k then 1 else 0 :=
    fun i k h1 hmx n => by
    induction n with
    | zero =>
      intro _ _
      have hz : ¬ min ≤ cqOkCount (([] : List (Multiset (K × Except E Unit))).sum) k := by
        show ¬ min ≤ cqOkCount (0 : Multiset (K × Except E Unit)) k
        unfold cqOkCount
        rw [Multiset.filter_zero, Multiset.card_zero]
        omega
      rw [List.take_zero, List.take_zero]
      show (0 : Multiset K).count k = _
      rw [Multiset.count_zero, if_neg hz]
    | succ n ih =>
      intro hn hcap
      have hcapn : cqKeyCount ((new_inputs i).take n).sum k ≤ max :=
        le_trans (cqKeyCount_mono (sublist_sum_le (List.take_sublist_take_left (Nat.le_succ n))) k)
          hcap
      have ih := ih (Nat.le_of_succ_le hn) hcapn
      obtain ⟨e, he⟩ : ∃ e, (just_reached_quorum i)[n]? = some e :=
        ⟨_, List.getElem?_eq_getElem (Nat.lt_of_succ_le hn)⟩
      rw [List.take_add_one, he, List.sum_append]
      simp only [Option.toList_some, List.sum_singleton]
      rw [Multiset.count_add, ih, hcount_tick i n e k h1 hmx hcap he]
      have hmono : cqOkCount ((new_inputs i).take n).sum k
          ≤ cqOkCount ((new_inputs i).take (n + 1)).sum k :=
        cqOkCount_mono (sublist_sum_le (List.take_sublist_take_left (Nat.le_succ n))) k
      split_ifs <;> omega
  -- (
  --   just_reached_quorum.assert_has_consistency_of(manual_proof!(/** TODO */)),
  --   responses.filter_map(q!(move |(key, res)| match res { Ok(_) => None, Err(e) => Some((key, e)) })),
  -- )
  (H.allTicks just_reached_quorum,
    H.filterMap responses (fun _me => cqErrProj))
  prove
    emit_sound := fun i k hk => by
      -- `k` left on some tick: that tick's window held `min` `Ok`s for it,
      -- and the window sits below the consumed pool
      obtain ⟨e, he, hke⟩ := mem_list_sum.mp hk
      obtain ⟨n, hn⟩ := List.mem_iff_getElem?.mp he
      obtain ⟨b_t, S, hb, hS, rfl⟩ := htick i n e hn
      have hwin : S.1 + b_t ≤ cqConsumed (responses i) (dec i) := by
        refine le_trans (add_le_add hS.window_le (le_refl b_t)) ?_
        have : ((new_inputs i).take n).sum + b_t = ((new_inputs i).take (n + 1)).sum := by
          rw [List.take_add_one, hb, List.sum_append]
          simp only [Option.toList_some, List.sum_singleton]
        rw [this]
        exact hpfx_le i (n + 1)
      have hok : min ≤ cqOkCount (S.1 + b_t) k := by
        split_ifs at hke with hmm
        · exact (Multiset.mem_filter.mp hke).2
        · exact (Multiset.mem_filter.mp (Multiset.mem_filter.mp hke).1).2
      exact le_trans hok (cqOkCount_mono hwin k),
    emit_count := fun i k h1 hmx hcap => by
      show ((just_reached_quorum i).sum).count k = _
      have hlen : (just_reached_quorum i).length = (new_inputs i).length := by
        rw [hjust_reached_quorum_run i, scanAcrossTicksTrace_length]
      have htot : ((new_inputs i).take (new_inputs i).length).sum
          = cqConsumed (responses i) (dec i) := by
        rw [List.take_length]; rfl
      have h := hcount i k h1 hmx (just_reached_quorum i).length (le_refl _)
        (by rw [hlen, htot]; exact hcap)
      rw [List.take_length, hlen, htot] at h
      exact h,
    fails_eq := fun i => rfl

#nondet_census collect_quorum (nondets := 1) (scheds := 0) (fuels := 0)

/-- **quorum.rs:7–88 `collect_quorum_with_response`** over location
`ℓ`: as `collect_quorum`, but each just-reached key emits its
accumulated `Ok` **responses**. Returns (`quorums`, `fails`).

Rust `nondet!` tally: 1 (the `use::batch`). -/
hydro def collect_quorum_with_response (H : HydroSem L mem) (ℓ : L)
    (responses : H.Stream ℓ (K × Except E V) .noOrder .exactlyOnce)
    (min max : Nat)
    (dec : H.BatchDec (mem ℓ) (K × Except E V)) :
    (H.Stream ℓ (K × V) .noOrder .exactlyOnce
      × H.Stream ℓ (K × E) .noOrder .exactlyOnce)
  ensures out => CQWREnsures ℓ min max responses dec out :=
  -- let quorums = sliced! {
  --   let new_inputs = use::batch(responses.clone(), nondet!(…));
  --   let mut not_all = use::state_null::<Stream<_, _, Bounded, Order>>();
  --   let mut min_but_not_max = use::state_null::<Stream<K, _, Bounded, NoOrder>>();
  tick (state not_all : H.BoundedStream (K × Except E V) .noOrder .exactlyOnce)
      (state min_but_not_max : H.BoundedStream K .noOrder .exactlyOnce)
      (input new_inputs := H.batch responses dec)
      -- the loop invariant: the same register discipline as `collect_quorum`
      (invariant ((quorums : List (Multiset (K × V)))
          (not_all : Multiset (K × Except E V)) (min_but_not_max : Multiset K)
          (new_inputs : Trace (Multiset (K × Except E V)))) =>
        CQRegInv min max not_all min_but_not_max
          ((new_inputs.take quorums.length).sum)) :=
    -- let current_responses = not_all.chain(new_inputs);
    let current_responses := H.bchain not_all new_inputs
    -- let count_per_key = current_responses.clone().into_keyed().fold(
    --   q!(move || (0, 0)),
    --   q!(move |accum, value| { if value.is_ok() { accum.0 += 1; } else { accum.1 += 1; } },
    --      commutative = manual_proof!(/** increment counters is commutative */)));
    let count_per_key := H.bkeyedFold cqCount (0, 0)
      (fun s x y => cqCount_comm s x y) current_responses
    -- let not_reached_min_count = count_per_key.clone()
    --   .filter(q!(move |(success, _error)| success < &min)).keys();
    let not_reached_min_count := H.bkeys (H.bfilter count_per_key
      (fun (_key, (success, _error)) => decide (success < min)))
    -- let reached_min_count = count_per_key.clone()
    --   .filter(q!(move |(success, _error)| success >= &min)).keys();
    let reached_min_count := H.bkeys (H.bfilter count_per_key
      (fun (_key, (success, _error)) => decide (min ≤ success)))
    -- let just_reached_quorum = if max == min {
    --   not_all = current_responses.clone().anti_join(reached_min_count);
    --   current_responses.anti_join(not_reached_min_count)
    -- } else {
    let branch := if max = min then
        (H.bantiJoin current_responses reached_min_count,
         (min_but_not_max,
          H.bantiJoin current_responses not_reached_min_count))
      else
        -- let received_from_all = count_per_key
        --   .filter(q!(move |(success, error)| (success + error) >= max)).keys();
        let received_from_all := H.bkeys (H.bfilter count_per_key
          (fun (_key, (success, error)) => decide (max ≤ success + error)))
        -- not_all = current_responses.clone().anti_join(received_from_all.clone());
        -- let out = current_responses.anti_join(not_reached_min_count).anti_join(min_but_not_max);
        -- min_but_not_max = reached_min_count.filter_not_in(received_from_all);
        -- out
        (H.bantiJoin current_responses received_from_all,
         (H.bfilterNotIn reached_min_count received_from_all,
          H.bantiJoin (H.bantiJoin current_responses not_reached_min_count)
            min_but_not_max))
    -- };
    rebind (not_all := branch.1, min_but_not_max := branch.2.1)
    -- just_reached_quorum.filter_map(q!(move |(key, res)| match res { Ok(v) => Some((key, v)), Err(_) => None }))
    yield (quorums := H.bfilterMap branch.2.2 cqOkProj)
    -- the loop obligations: the empty run, and ONE TICK (the shared
    -- register step)
    prove init := fun _i => by
        simp only [List.length_nil, List.take_zero, List.sum_nil, ValuesTick.seed_pair,
          ValuesTick.seed_stream, den]
        exact CQRegInv.init,
      tick := fun i n out st b_t hb hlen ih => by
        simp only [List.append_eq, List.length_append, List.length_singleton, hlen,
          List.take_add_one, hb, Option.toList_some, List.sum_append, List.sum_singleton] at ih ⊢
        simp only [quorums_step, den]
        by_cases hmm : max = min
        · simp only [if_pos hmm, cq_keys_filter_eq, decide_eq_true_eq, Bool.not_eq_true',
            decide_eq_false_iff_not, filter_notmem_keys_filter]
          exact CQRegInv.step_eq hmm.symm ih
        · simp only [if_neg hmm, cq_keys_filter_eq, decide_eq_true_eq,
            cqOkCount_add_cqErrCount, Bool.not_eq_true', decide_eq_false_iff_not,
            filter_notmem_keys_filter, keys_filter_notmem_keys_filter]
          exact CQRegInv.step_ne (Ne.symm hmm) ih;
  -- };
  -- **one tick, decoded**: tick `n`'s emission is the `Ok` projection of
  -- the window's responses at keys at `min` `Ok`s — at `min < max`, keys
  -- not already in `min_but_not_max`
  ghost have hopen : ∀ (i : Fin (mem ℓ)) (S : Multiset (K × Except E V) × Multiset K)
      (b : Multiset (K × Except E V)),
      @id (Multiset (K × V)) (quorums_step i S b).2
        = Multiset.filterMap cqOkProj (if max = min then
            (S.1 + b).filter (fun r => min ≤ cqOkCount (S.1 + b) r.1)
          else
            (S.1 + b).filter (fun r => min ≤ cqOkCount (S.1 + b) r.1 ∧ r.1 ∉ S.2)) :=
    fun i S b => by
    simp only [id, quorums_step, den]
    by_cases hmm : max = min
    · simp only [if_pos hmm, cq_keys_filter_eq, decide_eq_true_eq, Bool.not_eq_true',
        decide_eq_false_iff_not, filter_notmem_keys_filter, not_lt]
    · simp only [if_neg hmm, cq_keys_filter_eq, decide_eq_true_eq, Bool.not_eq_true',
        decide_eq_false_iff_not, filter_notmem_keys_filter, not_lt]
      rw [Multiset.filter_filter]
      refine congrArg _ (Multiset.filter_congr fun r _ => ?_)
      -- the membership's `Decidable` instance is over the unfolded
      -- `poolWeakenOrder` form (FINDINGS D64 gotcha v): generalize it
      have hmem : ∀ (hd : Decidable (r.1 ∈ S.2)),
          (@decide (r.1 ∈ S.2) hd = false ∧ min ≤ cqOkCount (S.1 + b) r.1
            ↔ min ≤ cqOkCount (S.1 + b) r.1 ∧ r.1 ∉ S.2) := fun hd => by
        rw [decide_eq_false_iff_not]
        exact and_comm
      exact hmem _
  -- the register before a tick, with the discipline it carries
  ghost have htick : ∀ (i : Fin (mem ℓ)) (n : Nat) (e : Multiset (K × V)),
      (quorums i)[n]? = some e →
      ∃ (b_t : Multiset (K × Except E V)) (S : Multiset (K × Except E V) × Multiset K),
        (new_inputs i)[n]? = some b_t
        ∧ CQRegInv min max S.1 S.2 ((new_inputs i).take n).sum
        ∧ e = Multiset.filterMap cqOkProj (if max = min then
            (S.1 + b_t).filter (fun r => min ≤ cqOkCount (S.1 + b_t) r.1)
          else
            (S.1 + b_t).filter (fun r => min ≤ cqOkCount (S.1 + b_t) r.1 ∧ r.1 ∉ S.2)) :=
    fun i n e he => by
    have hn : n < (quorums i).length := Trace.read_lt he
    obtain ⟨b_t, hb, rfl⟩ := (hquorums_at i n e).mp he
    have h := hquorums_inv_take i n
    simp only [List.length_take_of_le (Nat.le_of_lt hn)] at h
    exact ⟨b_t, _, hb, h, hopen i _ b_t⟩
  -- the responses consumed through tick `n` sit below the consumed pool
  ghost have hpfx_le : ∀ (i : Fin (mem ℓ)) (n : Nat),
      ((new_inputs i).take n).sum ≤ cqConsumed (responses i) (dec i) := fun i n =>
    sublist_sum_le (List.take_sublist _ _)
  ghost have hpfx_succ : ∀ (i : Fin (mem ℓ)) (n : Nat) (b_t : Multiset (K × Except E V)),
      (new_inputs i)[n]? = some b_t →
      ((new_inputs i).take (n + 1)).sum = ((new_inputs i).take n).sum + b_t := fun i n b_t hb => by
    rw [List.take_add_one, hb, List.sum_append]
    simp only [Option.toList_some, List.sum_singleton]
  -- **a tick's emission at a key, at the crossing**: when the key is not
  -- yet at `min` before the tick and the usage contract holds, the tick
  -- emits exactly the key's consumed `Ok` responses (if it reaches `min`)
  ghost have hcross_tick : ∀ (i : Fin (mem ℓ)) (n : Nat) (e : Multiset (K × V)) (k : K),
      1 ≤ min → min ≤ max → cqKeyCount ((new_inputs i).take (n + 1)).sum k ≤ max →
      (quorums i)[n]? = some e →
      ¬ min ≤ cqOkCount ((new_inputs i).take n).sum k →
      min ≤ cqOkCount ((new_inputs i).take (n + 1)).sum k →
      e.filter (fun r => r.1 = k)
        = (((new_inputs i).take (n + 1)).sum.filter (fun r => r.1 = k)).filterMap cqOkProj :=
    fun i n e k h1 hmx hcap he hbefore hafter => by
    obtain ⟨b_t, S, hb, hS, rfl⟩ := htick i n e he
    rw [hpfx_succ i n b_t hb] at hcap hafter ⊢
    set pfx := ((new_inputs i).take n).sum with hpfx
    have hcapp : cqKeyCount pfx k ≤ max :=
      le_trans (cqKeyCount_mono (Multiset.le_add_right _ _) k) hcap
    -- not dropped before the tick: at `min = max` by `hbefore`; at
    -- `min < max` a dropped key gets no more responses, so it could not
    -- reach `min` on this tick
    have hnd : ¬ cqDropped min max pfx k := by
      intro hd
      have hb0 := CQRegInv.dropped_kills_batch hcap hd
      have hokb : cqOkCount (pfx + b_t) k = cqOkCount pfx k := by
        rw [cqOkCount_add, cqOkCount_eq_zero_of_kpart hb0, Nat.add_zero]
      exact hbefore (hokb ▸ hafter)
    have hcur : (S.1 + b_t).filter (fun r => r.1 = k) = (pfx + b_t).filter (fun r => r.1 = k) := by
      rw [CQRegInv.window_kpart h1 hmx hcap hS, if_neg hnd]
    have hok : cqOkCount (S.1 + b_t) k = cqOkCount (pfx + b_t) k := cqOkCount_eq_of_kpart_eq hcur
    rw [filterMap_okProj_kpart_comm, ← hcur]
    by_cases hmm : max = min
    · rw [if_pos hmm, filter_key_of_pred _ (fun k' => min ≤ cqOkCount (S.1 + b_t) k') k,
        if_pos (hok ▸ hafter)]
    · have hlt : min < max := Nat.lt_of_le_of_ne hmx (Ne.symm hmm)
      have hnot : k ∉ S.2 := fun hk => hbefore ((hS.locked h1 hlt k hcapp).mp hk).1
      rw [if_neg hmm, filter_key_of_pred _ (fun k' => min ≤ cqOkCount (S.1 + b_t) k' ∧ k' ∉ S.2) k,
        if_pos ⟨hok ▸ hafter, hnot⟩]
  -- **one tick's registers, decoded** (the `rebind` lines at the
  -- denotation — the same formulas `CQRegInv.step` speaks of)
  ghost have hopen_st : ∀ (i : Fin (mem ℓ)) (S : Multiset (K × Except E V) × Multiset K)
      (b : Multiset (K × Except E V)),
      @id (Multiset (K × Except E V) × Multiset K) (quorums_step i S b).1
        = if max = min then
            ((S.1 + b).filter (fun r => ¬ min ≤ cqOkCount (S.1 + b) r.1), S.2)
          else
            ((S.1 + b).filter (fun r => ¬ max ≤ cqKeyCount (S.1 + b) r.1),
             (((S.1 + b).map Prod.fst).dedup.filter (fun k => min ≤ cqOkCount (S.1 + b) k)).filter
               (fun k => ¬ max ≤ cqKeyCount (S.1 + b) k)) :=
    fun i S b => by
    simp only [id, quorums_step, den]
    by_cases hmm : max = min
    · simp only [if_pos hmm, cq_keys_filter_eq, decide_eq_true_eq, Bool.not_eq_true',
        decide_eq_false_iff_not, filter_notmem_keys_filter]
    · simp only [if_neg hmm, cq_keys_filter_eq, decide_eq_true_eq, cqOkCount_add_cqErrCount,
        Bool.not_eq_true', decide_eq_false_iff_not, filter_notmem_keys_filter,
        keys_filter_notmem_keys_filter]
  -- **the pool bound** (unconditional — no usage contract): per key, the
  -- emissions so far plus the window's `Ok`s (unless the key is locked in
  -- `min_but_not_max`) embed in the consumed `Ok`s; a locked key's window
  -- already holds `min` `Ok`s. The potential argument behind "nothing is
  -- emitted twice": ONE TICK, over the decoded step
  ghost have hpool_step : ∀ (i : Fin (mem ℓ)) (S : Multiset (K × Except E V) × Multiset K)
      (b : Multiset (K × Except E V)) (k : K),
      (k ∉ S.2 ∨ (min ≠ max ∧ min ≤ cqOkCount S.1 k)) →
      (k ∉ (@id (Multiset (K × Except E V) × Multiset K) (quorums_step i S b).1).2
          ∨ (min ≠ max
            ∧ min ≤ cqOkCount (@id (Multiset (K × Except E V) × Multiset K) (quorums_step i S b).1).1 k))
        ∧ (@id (Multiset (K × V)) (quorums_step i S b).2).filter (fun r => r.1 = k)
          + (if k ∈ (@id (Multiset (K × Except E V) × Multiset K) (quorums_step i S b).1).2 then 0
              else ((@id (Multiset (K × Except E V) × Multiset K) (quorums_step i S b).1).1.filter
                (fun r => r.1 = k)).filterMap cqOkProj)
          ≤ (if k ∈ S.2 then 0 else (S.1.filter (fun r => r.1 = k)).filterMap cqOkProj)
            + (b.filter (fun r => r.1 = k)).filterMap cqOkProj := fun i S b k hinv => by
    rw [hopen_st i S b, hopen i S b]
    set cur := S.1 + b with hcur
    have hcurk : cur.filter (fun r => r.1 = k)
        = S.1.filter (fun r => r.1 = k) + b.filter (fun r => r.1 = k) := Multiset.filter_add _ _ _
    by_cases hmm : max = min
    · -- `min = max`: the window drops exactly what is emitted
      have hk : k ∉ S.2 := by
        rcases hinv with h | ⟨hne, -⟩
        · exact h
        · exact absurd hmm.symm hne
      simp only [if_pos hmm]
      refine ⟨Or.inl hk, ?_⟩
      rw [if_neg hk, if_neg hk, filterMap_okProj_kpart_comm,
        filter_key_of_pred _ (fun k' => min ≤ cqOkCount cur k') k,
        filter_key_of_pred _ (fun k' => ¬ min ≤ cqOkCount cur k') k]
      by_cases hc : min ≤ cqOkCount cur k
      · rw [if_pos hc, if_neg (not_not_intro hc), Multiset.filterMap_zero, Multiset.add_zero,
          hcurk, Multiset.filterMap_add]
      · rw [if_neg hc, if_pos hc, Multiset.filterMap_zero, Multiset.zero_add,
          hcurk, Multiset.filterMap_add]
    · -- `min < max`: the window drops the keys heard from everyone; the
      -- lock set names the keys at `min` not heard from everyone
      simp only [if_neg hmm]
      have hmem : k ∈ ((cur.map Prod.fst).dedup.filter (fun k' => min ≤ cqOkCount cur k')).filter
            (fun k' => ¬ max ≤ cqKeyCount cur k')
          ↔ (k ∈ cur.map Prod.fst ∧ min ≤ cqOkCount cur k) ∧ ¬ max ≤ cqKeyCount cur k := by
        rw [Multiset.mem_filter, Multiset.mem_filter, Multiset.mem_dedup]
      have hwin : (cur.filter (fun r => ¬ max ≤ cqKeyCount cur r.1)).filter (fun r => r.1 = k)
          = if ¬ max ≤ cqKeyCount cur k then cur.filter (fun r => r.1 = k) else 0 :=
        filter_key_of_pred _ (fun k' => ¬ max ≤ cqKeyCount cur k') k
      have hem : ((cur.filter (fun r => min ≤ cqOkCount cur r.1 ∧ r.1 ∉ S.2)).filterMap
            cqOkProj).filter (fun r => r.1 = k)
          = if (min ≤ cqOkCount cur k ∧ k ∉ S.2)
            then (cur.filter (fun r => r.1 = k)).filterMap cqOkProj else 0 := by
        rw [filterMap_okProj_kpart_comm,
          filter_key_of_pred _ (fun k' => min ≤ cqOkCount cur k' ∧ k' ∉ S.2) k]
        split_ifs <;> simp only [Multiset.filterMap_zero]
      -- a key absent from the window has an empty key part
      have hnokeys : k ∉ cur.map Prod.fst → cur.filter (fun r => r.1 = k) = 0 := fun hn =>
        kpart_eq_zero_of_keyCount (by
          by_contra hc
          exact hn ((cq_mem_keys_iff cur k).mpr (Nat.pos_of_ne_zero hc)))
      refine ⟨?_, ?_⟩
      · -- the register disjunction survives
        by_cases hl : k ∈ ((cur.map Prod.fst).dedup.filter (fun k' => min ≤ cqOkCount cur k')).filter
            (fun k' => ¬ max ≤ cqKeyCount cur k')
        · obtain ⟨⟨-, hc⟩, hn⟩ := hmem.mp hl
          refine Or.inr ⟨Ne.symm hmm, ?_⟩
          rw [← cqOkCount_kpart, hwin, if_pos hn, cqOkCount_kpart]
          exact hc
        · exact Or.inl hl
      · rw [hem]
        by_cases hk : k ∈ S.2
        · -- locked: nothing is emitted; the window held `min` `Ok`s, so the
          -- key stays locked or leaves the window
          have hokS : min ≤ cqOkCount S.1 k := by
            rcases hinv with h | ⟨-, h⟩
            · exact absurd hk h
            · exact h
          have hokc : min ≤ cqOkCount cur k :=
            le_trans hokS (cqOkCount_mono (Multiset.le_add_right _ _) k)
          rw [if_neg (fun hc => hc.2 hk), if_pos hk, Multiset.zero_add, Multiset.zero_add]
          by_cases hl : k ∈ ((cur.map Prod.fst).dedup.filter (fun k' => min ≤ cqOkCount cur k')).filter
              (fun k' => ¬ max ≤ cqKeyCount cur k')
          · rw [if_pos hl]
            exact Multiset.zero_le _
          · rw [if_neg hl, hwin]
            by_cases hn : ¬ max ≤ cqKeyCount cur k
            · have hkeys : k ∉ cur.map Prod.fst := fun hc => hl (hmem.mpr ⟨⟨hc, hokc⟩, hn⟩)
              rw [if_pos hn, hnokeys hkeys, Multiset.filterMap_zero]
              exact Multiset.zero_le _
            · rw [if_neg hn, Multiset.filterMap_zero]
              exact Multiset.zero_le _
        · -- unlocked: the window's `Ok`s plus this batch's are the key's
          -- consumed `Ok`s; they are emitted (and the key leaves the
          -- window or locks) or carried
          rw [if_neg hk, ← Multiset.filterMap_add, ← hcurk]
          by_cases hc : min ≤ cqOkCount cur k
          · rw [if_pos ⟨hc, hk⟩]
            by_cases hl : k ∈ ((cur.map Prod.fst).dedup.filter (fun k' => min ≤ cqOkCount cur k')).filter
                (fun k' => ¬ max ≤ cqKeyCount cur k')
            · rw [if_pos hl, Multiset.add_zero]
            · rw [if_neg hl, hwin]
              by_cases hn : ¬ max ≤ cqKeyCount cur k
              · have hkeys : k ∉ cur.map Prod.fst := fun hm => hl (hmem.mpr ⟨⟨hm, hc⟩, hn⟩)
                rw [if_pos hn, hnokeys hkeys, Multiset.filterMap_zero, Multiset.add_zero]
              · rw [if_neg hn, Multiset.filterMap_zero, Multiset.add_zero]
          · rw [if_neg (fun hx => hc hx.1), Multiset.zero_add,
              if_neg (fun hl => hc (hmem.mp hl).1.2), hwin]
            by_cases hn : ¬ max ≤ cqKeyCount cur k
            · rw [if_pos hn]
            · rw [if_neg hn, Multiset.filterMap_zero]
              exact Multiset.zero_le _
  -- the register run: seeded empty, stepped on a read, frozen on a stall
  ghost obtain ⟨reg, hreg0, hreg_at, hreg_succ, hreg_stall, -⟩ := hquorums_reg
  ghost have hlen : ∀ (i : Fin (mem ℓ)), (quorums i).length = (new_inputs i).length := fun i => by
    rw [hquorums_run i, scanAcrossTicksTrace_length]
  -- **the pool bound over a run prefix**: the potential, summed
  ghost have hpool : ∀ (i : Fin (mem ℓ)) (k : K) (n : Nat),
      (k ∉ (reg i n).2 ∨ (min ≠ max ∧ min ≤ cqOkCount (reg i n).1 k))
      ∧ ((quorums i).take n).sum.filter (fun r => r.1 = k)
          + (if k ∈ (reg i n).2 then 0 else ((reg i n).1.filter (fun r => r.1 = k)).filterMap cqOkProj)
        ≤ (((new_inputs i).take n).sum.filter (fun r => r.1 = k)).filterMap cqOkProj :=
    fun i k n => by
    induction n with
    | zero =>
      rw [hreg0 i]
      simp [ValuesTick.seed_pair, ValuesTick.seed_stream, den]
    | succ n ih =>
      obtain ⟨ih1, ih2⟩ := ih
      cases hb : (new_inputs i)[n]? with
      | none =>
        -- the input ended: the register freezes and nothing more is emitted
        have hge : (new_inputs i).length ≤ n := List.getElem?_eq_none_iff.mp hb
        have hq : (quorums i).length ≤ n := by rw [hlen i]; exact hge
        rw [List.take_of_length_le hq, List.take_of_length_le hge] at ih2
        rw [hreg_stall i n hb, List.take_of_length_le (Nat.le_succ_of_le hq),
          List.take_of_length_le (Nat.le_succ_of_le hge)]
        exact ⟨ih1, ih2⟩
      | some b_t =>
        have he : (quorums i)[n]? = some (quorums_step i (reg i n) b_t).2 :=
          (hreg_at i n _).mpr ⟨b_t, hb, rfl⟩
        rw [hreg_succ i n b_t hb, List.take_add_one, List.take_add_one, he, hb, List.sum_append,
          List.sum_append]
        simp only [Option.toList_some, List.sum_singleton]
        obtain ⟨hs1, hs2⟩ := hpool_step i (reg i n) b_t k ih1
        simp only [id] at hs1 hs2
        refine ⟨hs1, ?_⟩
        rw [Multiset.filter_add, Multiset.filter_add, Multiset.filterMap_add, Multiset.add_assoc]
        exact le_trans (add_le_add (le_refl _) hs2)
          (by rw [← Multiset.add_assoc]; exact add_le_add ih2 (le_refl _))
  -- (
  --   quorums.assert_has_consistency_of(manual_proof!(/** TODO */)),
  --   responses.filter_map(q!(move |(key, res)| match res { Ok(_) => None, Err(e) => Some((key, e)) })),
  -- )
  (H.allTicks quorums,
   H.filterMap responses (fun _me => cqErrProj))
  prove
    emit_mem_sound := fun i k v hk => by
      obtain ⟨e, he, hke⟩ := mem_list_sum.mp hk
      obtain ⟨n, hn⟩ := List.mem_iff_getElem?.mp he
      obtain ⟨b_t, S, hb, hS, rfl⟩ := htick i n e hn
      have hwin : S.1 + b_t ≤ cqConsumed (responses i) (dec i) := by
        refine le_trans (add_le_add hS.window_le (le_refl b_t)) ?_
        rw [← hpfx_succ i n b_t hb]
        exact hpfx_le i (n + 1)
      obtain ⟨x, hx, hproj⟩ := (Multiset.mem_filterMap _ _).mp hke
      obtain rfl := okProj_eq_some hproj
      have hok : ((k, Except.ok v) : K × Except E V) ∈ S.1 + b_t ∧ min ≤ cqOkCount (S.1 + b_t) k := by
        split_ifs at hx with hmm
        · exact Multiset.mem_filter.mp hx
        · exact ⟨(Multiset.mem_filter.mp hx).1, (Multiset.mem_filter.mp hx).2.1⟩
      exact ⟨Multiset.mem_of_le hwin hok.1, le_trans hok.2 (cqOkCount_mono hwin k)⟩,
    emit_le := fun i k _ _ _ => by
      show ((quorums i).sum).filter (fun r => r.1 = k) ≤ _
      have h := (hpool i k (quorums i).length).2
      rw [List.take_length, hlen i, List.take_length] at h
      exact le_trans (Multiset.le_add_right _ _) h,
    emit_complete := fun i k h1 hmx hcap hok => by
      show min ≤ (((quorums i).sum).filter (fun r => r.1 = k)).card
      have htot : ((new_inputs i).take (new_inputs i).length).sum
          = cqConsumed (responses i) (dec i) := by
        rw [List.take_length]; rfl
      -- the prefix counts climb from `0` to `min`: a first tick crosses
      have hz : ¬ min ≤ cqOkCount (((new_inputs i).take 0).sum) k := by
        rw [List.take_zero]
        show ¬ min ≤ cqOkCount (0 : Multiset (K × Except E V)) k
        unfold cqOkCount
        rw [Multiset.filter_zero, Multiset.card_zero]
        omega
      have hpos : 0 < (new_inputs i).length := by
        by_contra hc
        have h0 : (new_inputs i).length = 0 := Nat.eq_zero_of_not_pos hc
        rw [← htot, h0] at hok
        exact hz hok
      have hex : ∃ n, min ≤ cqOkCount (((new_inputs i).take (n + 1)).sum) k :=
        ⟨(new_inputs i).length - 1, by
          rw [Nat.sub_add_cancel hpos, htot]; exact hok⟩
      classical
      obtain ⟨n₀, hn₀, hmin⟩ : ∃ n₀, min ≤ cqOkCount (((new_inputs i).take (n₀ + 1)).sum) k
          ∧ ∀ m, m < n₀ → ¬ min ≤ cqOkCount (((new_inputs i).take (m + 1)).sum) k :=
        ⟨Nat.find hex, Nat.find_spec hex, fun m hm => Nat.find_min hex hm⟩
      have hbefore : ¬ min ≤ cqOkCount (((new_inputs i).take n₀).sum) k := by
        cases n₀ with
        | zero => exact hz
        | succ m => exact hmin m (Nat.lt_succ_self m)
      have hlt : n₀ < (new_inputs i).length := by
        by_contra hc
        rw [List.take_of_length_le (Nat.le_of_not_lt hc)] at hbefore
        exact hbefore hok
      obtain ⟨e, he⟩ : ∃ e, (quorums i)[n₀]? = some e :=
        ⟨_, List.getElem?_eq_getElem (by rw [hlen i]; exact hlt)⟩
      have hcap' : cqKeyCount (((new_inputs i).take (n₀ + 1)).sum) k ≤ max :=
        le_trans (cqKeyCount_mono (hpfx_le i (n₀ + 1)) k) hcap
      have hek := hcross_tick i n₀ e k h1 hmx hcap' he hbefore hn₀
      have hcard : min ≤ (e.filter (fun r => r.1 = k)).card := by
        rw [hek, card_okProj_kpart]
        exact hn₀
      refine le_trans hcard (Multiset.card_le_card (Multiset.filter_le_filter _ ?_))
      have := sublist_sum_le (List.singleton_sublist.mpr (Trace.mem_of_read he))
      rwa [List.sum_singleton] at this,
    emit_pool_le := fun i => by
      show (quorums i).sum ≤ _
      rw [Multiset.le_iff_count]
      rintro ⟨k, v⟩
      have h := (hpool i k (quorums i).length).2
      rw [List.take_length, hlen i, List.take_length] at h
      have h' : ((quorums i).sum).filter (fun r => r.1 = k)
          ≤ ((cqConsumed (responses i) (dec i)).filterMap cqOkProj).filter (fun r => r.1 = k) := by
        rw [filterMap_okProj_kpart_comm]
        exact le_trans (Multiset.le_add_right _ _) h
      have hc := Multiset.count_le_of_le ((k, v) : K × V) h'
      rwa [Multiset.count_filter, if_pos rfl, Multiset.count_filter, if_pos rfl] at hc,
    fails_eq := fun i => rfl

#nondet_census collect_quorum_with_response (nondets := 1)
  (scheds := 0) (fuels := 0)

end Programs

/-! ## Executable smoke tests (mirroring quorum.rs's unit tests) -/

private abbrev oneLoc : Unit → Nat := fun _ => 1

-- `collect_quorum_functionality` (quorum.rs), min 2 of max 3:
-- key 1 reaches quorum with 2 Oks …
#guard (collect_quorum (Values Unit oneLoc) () (fun _ =>
    ({(1, .ok ()), (1, .ok ())} : Multiset (Nat × Except Nat Unit)))
    2 3 (fun _ => [{(1, .ok ()), (1, .ok ())}])).1 0
  = {1}
-- … key 3 (1 Ok, 2 Errs) does not, and its errors surface
#guard (collect_quorum (Values Unit oneLoc) () (fun _ =>
    ({(3, .ok ()), (3, .error 7), (3, .error 8)}
      : Multiset (Nat × Except Nat Unit)))
    2 3 (fun _ => [{(3, .ok ()), (3, .error 7)}, {(3, .error 8)}])
   ).1 0
  = 0
#guard (collect_quorum (Values Unit oneLoc) () (fun _ =>
    ({(3, .ok ()), (3, .error 7), (3, .error 8)}
      : Multiset (Nat × Except Nat Unit)))
    2 3 (fun _ => [{(3, .ok ()), (3, .error 7)}, {(3, .error 8)}])
   ).2 0
  = ({(3, 7), (3, 8)} : Multiset (Nat × Nat))
-- `collect_quorum_no_double_quorum_before_max` (min 2, max 4): extra
-- Oks after the crossing never re-emit the key
#guard (collect_quorum (Values Unit oneLoc) () (fun _ =>
    ({(1, .ok ()), (1, .ok ()), (1, .ok ()), (1, .ok ())}
      : Multiset (Nat × Except Nat Unit)))
    2 4 (fun _ => [{(1, .ok ()), (1, .ok ())},
                   {(1, .ok ())}, {(1, .ok ())}])).1 0
  = {1}
-- `collect_quorum_min_equals_max` (min = max = 2): 1 Ok + 1 Err fails,
-- 2 Oks succeed
#guard (collect_quorum (Values Unit oneLoc) () (fun _ =>
    ({(2, .ok ()), (2, .error 9), (3, .ok ()), (3, .ok ())}
      : Multiset (Nat × Except Nat Unit)))
    2 2 (fun _ => [{(2, .ok ()), (2, .error 9), (3, .ok ()),
                    (3, .ok ())}])).1 0
  = {3}
-- `collect_quorum_with_response_no_order` (min = max = 2): the
-- just-reached keys emit their accumulated responses
#guard (collect_quorum_with_response (Values Unit oneLoc) () (fun _ =>
    ({(1, .ok 10), (1, .ok 11), (2, .ok 20), (3, .ok 30), (3, .ok 31)}
      : Multiset (Nat × Except Nat Nat)))
    2 2 (fun _ => [{(1, .ok 10), (1, .ok 11), (2, .ok 20)},
                   {(3, .ok 30), (3, .ok 31)}])).1 0
  = ({(1, 10), (1, 11), (3, 30), (3, 31)} : Multiset (Nat × Nat))

/-! ## The quorum safety headline over the generated artifacts -/

section CQSafe

variable {K E : Type} [DecidableEq K] [DecidableEq E]

/-- Machine-run quorum safety, premise-free — assembled from the
generated artifacts above (the D41 pattern at the smallest scale). -/
theorem cq_safe_sched' {pacing : Unit → Fin 1 → Nat → Bool}
    (h : Fin 1 → StepHist (K × Except E Unit))
    (v : Fin 1 → Multiset (K × Except E Unit))
    (hc : ∀ T i, Multiset.ofList ((h i).view T) ≤ v i)
    (mn mx : Nat) (T : Nat)
    (i : Fin 1) (k : K)
    (hk : k ∈ ((collect_quorum (SchedSem Unit (fun _ => 1) pacing) ()
      h mn mx ()).1 i).view T) :
    ∃ d : (Values Unit (fun _ => 1)).BatchDec 1 (K × Except E Unit),
      mn ≤ cqOkCount (cqConsumed (v i) (d i)) k := by
  refine ⟨collect_quorum_vdec (Td := T) (pacing := pacing) () mn mx h, ?_⟩
  have hcpl := (collect_quorum
      (CoupleSem Unit (fun _ => 1) pacing T T (Nat.le_refl T)) ()
      (CoStream.inputC (ord := .noOrder) (ret := .exactlyOnce) h v hc)
      mn mx (CoDec.batch _)).1.cpl
    (collect_quorum_co_wf₁ (L := Unit) (mem := fun _ => 1)
      (pacing := pacing) (Tc := T) (Td := T) (hjT := Nat.le_refl T)
      () (CoStream.inputC h v hc) mn mx
      (CoDec.batch _) trivial) i
  have hsr := collect_quorum_co_sr₁ (Tc := T) (Td := T)
    (hjT := Nat.le_refl T) (pacing := pacing) ()
    (CoStream.inputC (ord := .noOrder) (ret := .exactlyOnce) h v hc)
    mn mx (CoDec.batch _)
  have hrr := collect_quorum_co_rr₁ (pacing := pacing) (Tc := T)
    (Td := T) (hjT := Nat.le_refl T) ()
    (CoStream.inputC (ord := .noOrder) (ret := .exactlyOnce) h v hc)
    mn mx (CoDec.batch _)
  rw [hsr, hrr] at hcpl
  have hens := collect_quorum.ensures (L := Unit) (mem := fun _ => 1) ()
    v mn mx (collect_quorum_vdec (Td := T) (pacing := pacing) () mn mx h)
  exact hens.emit_sound i k (Multiset.mem_of_le hcpl
    (Multiset.mem_coe.mpr hk))

end CQSafe

end Hydro
