import HydroLean.Programs.CollectQuorum
import HydroLean.Hydro.StreamLemmas

/-!
# Unbounded correctness of `collect_quorum` (proof)

This file proves `CollectQuorumCorrect` (stated in `Programs/CollectQuorum.lean`):
for **all** inputs, **all** `1 ≤ min ≤ max` with the at-most-`max`-responses
contract, and **all** adversarial batchings of the input — i.e. every possible
materialization of the `nondet!` batching guard — the tick loop emits exactly
the keys with at least `min` successful responses, each exactly once.

This is the "unbounded" counterpart of the Rust simulator's `exhaustive` tests
(`hydro_std/src/quorum.rs`): where the simulator enumerates batch boundaries
for finite instances, the theorem quantifies over every input and batching.
It also *discharges the justification written inside the Rust `nondet!`*:

> "We always persist values that have not reached quorum, so even with
> arbitrary batching we always produce deterministic quorum results."

## Proof structure

We characterize the looped state after consuming any input prefix `p`
*exactly*, per branch of the Rust code:

- `min = max` branch: `notAll = p.filter (okCount p ·.1 < min)` — the pending
  responses are precisely those whose key has not yet reached `min` successes.
- `min < max` branch: `notAll = p.filter (totCount p ·.1 < max)` and
  `minButNotMax ≈ { k | min ≤ okCount p k ∧ totCount p k < max }` (a nodup
  enumeration).

and characterize each tick's emission *as a set* (the emission **order**
depends on the batching — the output is `NoOrder` in Rust, and the spec is
membership + `Nodup`, which is order-insensitive):

`k ∈ out ↔ min ≤ okCount (p ++ b) k ∧ okCount p k < min` — "k became
qualified during this tick".

The at-most-`max` contract enters in exactly one place (per branch): a key
whose responses were dropped from the state (reached quorum / received all)
can never receive another response, so dropping is safe. This is where the
Rust comment's "we always persist values that have not reached quorum" is
load-bearing, and the proof fails without the contract (see the counterexample
discussion in `Programs/CollectQuorum.lean`).
-/

namespace HydroLean.Programs

open HydroLean.Hydro

universe u v

/-! ## Stream-level helpers

`nodup_eraseDups` and the `HydroLean.Hydro.Stream` keyed-count helpers
(`mem_keys`, `nodup_keys`, `countKeyP_filter_key`, `countKey_filter_key`,
`countKeyP_le_countKey`, `mem_map_fst_of_countKeyP_pos`,
`not_mem_map_fst_of_countKey_eq_zero`, `countKey_pos_of_mem`) formerly proven
here now live in `HydroLean/Hydro/StreamLemmas.lean` (imported above) under
the same fully-qualified names. -/

end HydroLean.Programs

/-! ## The `min = max` branch -/

namespace HydroLean.Programs

open HydroLean.Hydro HydroLean.Hydro.Stream

universe u' v'

variable {κ : Type u'} {E : Type v'} [DecidableEq κ]

/-- Success count of key `k` in `p` (the `into_keyed().fold` counting `Ok`s). -/
def okc (p : Stream (κ × Except E Unit)) (k : κ) : Nat :=
  p.countKeyP k Except.isOk

/-- Total response count of key `k` in `p`. -/
def totc (p : Stream (κ × Except E Unit)) (k : κ) : Nat :=
  p.countKey k

/-! Fold-form arithmetic facts about `okc`/`totc` (stated in terms of `okc` and
`totc` applications so that `omega` sees consistent atoms). -/

theorem okc_append (p b : Stream (κ × Except E Unit)) (k : κ) :
    okc (p ++ b) k = okc p k + okc b k := Stream.countKeyP_append p b k _

theorem totc_append (p b : Stream (κ × Except E Unit)) (k : κ) :
    totc (p ++ b) k = totc p k + totc b k := Stream.countKey_append p b k

theorem okc_le_totc (p : Stream (κ × Except E Unit)) (k : κ) :
    okc p k ≤ totc p k := countKeyP_le_countKey p k _

theorem totc_pos_of_mem {b : Stream (κ × Except E Unit)} {r : κ × Except E Unit}
    (hr : r ∈ b) : 0 < totc b r.1 := countKey_pos_of_mem hr

theorem mem_map_fst_of_okc_pos {p : Stream (κ × Except E Unit)} {k : κ}
    (h : 0 < okc p k) : k ∈ p.map Prod.fst := mem_map_fst_of_countKeyP_pos h

/-- The exact looped state of the `min = max` branch after consuming prefix
`p`: the responses whose key has not yet reached `min` successes ("we always
persist values that have not reached quorum"). -/
def pendingMinEq (min : Nat) (p : Stream (κ × Except E Unit)) :
    Stream (κ × Except E Unit) :=
  List.filter (fun r => decide (okc p r.1 < min)) p

/-- The buffered responses at tick start of the `min = max` branch:
characterized state for prefix `p`, chained with the new batch `b`. -/
def curMinEq (min : Nat) (p b : Stream (κ × Except E Unit)) :
    Stream (κ × Except E Unit) :=
  pendingMinEq min p ++ b

private theorem okc_pending (min : Nat) (p : Stream (κ × Except E Unit)) (k : κ) :
    okc (pendingMinEq min p) k = if okc p k < min then okc p k else 0 := by
  have h := countKeyP_filter_key p (fun k => decide (okc p k < min)) k Except.isOk
  simp only [decide_eq_true_eq] at h
  exact h

private theorem b_empty_of_reached {min : Nat} {p b : Stream (κ × Except E Unit)}
    (hmax : ∀ k, totc (p ++ b) k ≤ min) {k : κ} (hk : min ≤ okc p k) :
    totc b k = 0 := by
  have h1 := hmax k
  rw [totc_append] at h1
  have h2 := okc_le_totc p k
  omega

private theorem okc_cur {min : Nat} {p b : Stream (κ × Except E Unit)}
    (hmax : ∀ k, totc (p ++ b) k ≤ min) (k : κ) :
    okc (curMinEq min p b) k = if okc p k < min then okc (p ++ b) k else 0 := by
  have hsplit : okc (curMinEq min p b) k = okc (pendingMinEq min p) k + okc b k :=
    okc_append _ b k
  by_cases hlt : okc p k < min
  · rw [hsplit, okc_pending, if_pos hlt, if_pos hlt, okc_append]
  · have hb : okc b k = 0 := by
      have h0 := b_empty_of_reached hmax (Nat.le_of_not_lt hlt)
      have hle := okc_le_totc b k
      omega
    rw [hsplit, okc_pending, if_neg hlt, if_neg hlt, hb]

/-- The batch emitted by one tick of the `min = max` branch. -/
def emitMinEq (min : Nat) (p b : Stream (κ × Except E Unit)) : Stream κ :=
  List.filter (fun k => decide (min ≤ okc (curMinEq min p b) k)) (curMinEq min p b).keys

/-- Emission membership: exactly the keys that became qualified this tick. -/
private theorem mem_emitMinEq {min : Nat} (hmin : 1 ≤ min)
    {p b : Stream (κ × Except E Unit)}
    (hmax : ∀ k, totc (p ++ b) k ≤ min) (k : κ) :
    k ∈ emitMinEq min p b ↔ (min ≤ okc (p ++ b) k ∧ okc p k < min) := by
  rw [emitMinEq, List.mem_filter, mem_keys]
  constructor
  · rintro ⟨_, hge⟩
    simp only [decide_eq_true_eq] at hge
    by_cases hlt : okc p k < min
    · rw [okc_cur hmax k, if_pos hlt] at hge
      exact ⟨hge, hlt⟩
    · rw [okc_cur hmax k, if_neg hlt] at hge
      omega
  · rintro ⟨hge, hlt⟩
    have hcurk : okc (curMinEq min p b) k = okc (p ++ b) k := by
      rw [okc_cur hmax k, if_pos hlt]
    refine ⟨mem_map_fst_of_okc_pos (by omega), by
      simp only [decide_eq_true_eq]
      omega⟩

private theorem nodup_emitMinEq (min : Nat) (p b : Stream (κ × Except E Unit)) :
    (emitMinEq min p b).Nodup :=
  (List.filter_sublist).nodup (nodup_keys _)

/-- State preservation: dropping just-qualified keys from the buffer yields
exactly the characterized state for `p ++ b`. Safe only under the response
contract (`hmax`): a dropped key can receive no further response. -/
private theorem state_minEq {min : Nat} {p b : Stream (κ × Except E Unit)}
    (hmax : ∀ k, totc (p ++ b) k ≤ min) :
    (curMinEq min p b).antiJoin (emitMinEq min p b) = pendingMinEq min (p ++ b) := by
  have h1 : (curMinEq min p b).antiJoin (emitMinEq min p b)
      = List.filter (fun r => decide (okc (curMinEq min p b) r.1 < min)) (curMinEq min p b) := by
    refine List.filter_congr fun r hr => ?_
    have hrmem : r.1 ∈ (curMinEq min p b).map Prod.fst := List.mem_map.mpr ⟨r, hr, rfl⟩
    by_cases hge : min ≤ okc (curMinEq min p b) r.1
    · have hin : r.1 ∈ emitMinEq min p b :=
        List.mem_filter.mpr ⟨mem_keys.mpr hrmem, by simpa using hge⟩
      simp [hin, Nat.not_lt_of_le hge]
    · have hnin : r.1 ∉ emitMinEq min p b := by
        intro hmem
        rw [emitMinEq, List.mem_filter] at hmem
        exact hge (by simpa using hmem.2)
      simp [hnin, Nat.lt_of_not_le hge]
  rw [h1]
  show List.filter _ (pendingMinEq min p ++ b) = _
  rw [List.filter_append]
  have hp : List.filter (fun r => decide (okc (curMinEq min p b) r.1 < min))
        (pendingMinEq min p)
      = List.filter (fun r => decide (okc (p ++ b) r.1 < min)) p := by
    rw [pendingMinEq, List.filter_filter]
    refine List.filter_congr fun r _ => ?_
    by_cases hlt : okc p r.1 < min
    · have heq : okc (curMinEq min p b) r.1 = okc (p ++ b) r.1 := by
        rw [okc_cur hmax r.1, if_pos hlt]
      simp [heq, hlt]
    · have hge' : min ≤ okc (p ++ b) r.1 := by
        have hmono := okc_append p b r.1
        omega
      simp [hlt, Nat.not_lt_of_le hge']
  have hb : List.filter (fun r => decide (okc (curMinEq min p b) r.1 < min)) b
      = List.filter (fun r => decide (okc (p ++ b) r.1 < min)) b := by
    refine List.filter_congr fun r hr => ?_
    by_cases hlt : okc p r.1 < min
    · have heq : okc (curMinEq min p b) r.1 = okc (p ++ b) r.1 := by
        rw [okc_cur hmax r.1, if_pos hlt]
      simp [heq]
    · exfalso
      have h0 := b_empty_of_reached hmax (Nat.le_of_not_lt hlt)
      have hpos := totc_pos_of_mem hr
      omega
  rw [hp, hb, pendingMinEq, List.filter_append]

/-- **One-tick reduction, `min = max` branch**: the step from the characterized
state for `p` on batch `b` produces the antiJoin state and emits
`emitMinEq min p b`. -/
private theorem step_minEqMax (min : Nat)
    (p b : Stream (κ × Except E Unit)) :
    (collectQuorumTick (E := E) min min).step ⟨pendingMinEq min p, []⟩ b
      = (⟨(curMinEq min p b).antiJoin (emitMinEq min p b), []⟩, emitMinEq min p b) := by
  simp only [collectQuorumTick, Stream.chain, curMinEq, emitMinEq]
  rfl


/-- **Multi-tick preservation, `min = max` branch**: running any batch list
from the characterized state for prefix `p` lands in the characterized state
for the full consumed input, and the flattened emissions are exactly the keys
that became qualified after `p` — duplicate-free. -/
private theorem runFrom_minEqMax (min : Nat) (hmin : 1 ≤ min)
    (bs : List (Stream (κ × Except E Unit))) (p : Stream (κ × Except E Unit))
    (hmax : ∀ k, totc (p ++ bs.flatten) k ≤ min) :
    ((collectQuorumTick (E := E) min min).runFrom ⟨pendingMinEq min p, []⟩ bs).1
      = ⟨pendingMinEq min (p ++ bs.flatten), []⟩ ∧
    (((collectQuorumTick (E := E) min min).runFrom ⟨pendingMinEq min p, []⟩ bs).2.flatten).Nodup ∧
    ∀ k, k ∈ ((collectQuorumTick (E := E) min min).runFrom ⟨pendingMinEq min p, []⟩ bs).2.flatten ↔
      (min ≤ okc (p ++ bs.flatten) k ∧ okc p k < min) := by
  induction bs generalizing p with
  | nil =>
    refine ⟨by simp, by simp, fun k => ?_⟩
    simp only [TickLoop.runFrom, List.flatten_nil, List.not_mem_nil, List.append_nil,
      false_iff]
    omega
  | cons b bs ih =>
    -- contract for the extended prefix and for the tail
    have hmax₁ : ∀ k, totc (p ++ b) k ≤ min := by
      intro k
      have h := hmax k
      rw [show p ++ (b :: bs).flatten = (p ++ b) ++ bs.flatten by
        simp [List.flatten_cons, List.append_assoc]] at h
      rw [totc_append] at h
      omega
    have hmax₂ : ∀ k, totc ((p ++ b) ++ bs.flatten) k ≤ min := by
      intro k
      have h := hmax k
      rw [show p ++ (b :: bs).flatten = (p ++ b) ++ bs.flatten by
        simp [List.flatten_cons, List.append_assoc]] at h
      exact h
    -- one step, then the induction hypothesis from the extended prefix
    have hstep := step_minEqMax (E := E) min p b
    have hstate := state_minEq (min := min) hmax₁
    obtain ⟨ihState, ihNodup, ihMem⟩ := ih (p ++ b) hmax₂
    have hflat : p ++ (b :: bs).flatten = (p ++ b) ++ bs.flatten := by
      simp [List.flatten_cons, List.append_assoc]
    rw [TickLoop.runFrom_cons, hstep]
    simp only [hstate]
    refine ⟨by rw [ihState, hflat], ?_, ?_⟩
    · -- Nodup of this tick's emission ++ later emissions
      rw [List.flatten_cons, List.nodup_append]
      refine ⟨nodup_emitMinEq min p b, ihNodup, fun a ha a' ha' => ?_⟩
      have h₁ := (mem_emitMinEq hmin hmax₁ a).mp ha
      intro heq
      subst heq
      have h₂ := (ihMem a).mp ha'
      omega
    · intro k
      rw [List.flatten_cons, List.mem_append, mem_emitMinEq hmin hmax₁, ihMem, hflat]
      constructor
      · rintro (⟨hge, hlt⟩ | ⟨hge, hlt⟩)
        · have := okc_append (p ++ b) bs.flatten k
          exact ⟨by omega, hlt⟩
        · have := okc_append p b k
          omega
      · rintro ⟨hge, hlt⟩
        by_cases hmid : min ≤ okc (p ++ b) k
        · exact Or.inl ⟨hmid, hlt⟩
        · exact Or.inr ⟨hge, by omega⟩


/-! ## The `min < max` branch

Here the state keeps *all* responses of keys that have not yet received all
`max` responses (`notAll`), and separately tracks the already-emitted keys
still awaiting responses (`minButNotMax`, replaced every tick). -/

/-- The exact `notAll` state of the general branch after consuming prefix `p`:
responses whose key has not yet received all `max` responses. -/
def pendingGen (max : Nat) (p : Stream (κ × Except E Unit)) :
    Stream (κ × Except E Unit) :=
  List.filter (fun r => decide (totc p r.1 < max)) p

/-- Buffered responses at tick start of the general branch. -/
def curGen (max : Nat) (p b : Stream (κ × Except E Unit)) :
    Stream (κ × Except E Unit) :=
  pendingGen max p ++ b

/-- Set-level characterization of `minButNotMax` after prefix `p`: the keys
already qualified (`min ≤ okc`) but still awaiting responses (`totc < max`).
Only the *set* is batching-invariant, hence the `Nodup` + membership form. -/
def MbnmSpec (min max : Nat) (p : Stream (κ × Except E Unit))
    (mb : Stream κ) : Prop :=
  mb.Nodup ∧ ∀ k, k ∈ mb ↔
    (k ∈ p.map Prod.fst ∧ min ≤ okc p k ∧ totc p k < max)

private theorem totc_pendingGen (max : Nat) (p : Stream (κ × Except E Unit)) (k : κ) :
    totc (pendingGen max p) k = if totc p k < max then totc p k else 0 := by
  have h := countKey_filter_key p (fun k => decide (totc p k < max)) k
  simp only [decide_eq_true_eq] at h
  exact h

private theorem okc_pendingGen (max : Nat) (p : Stream (κ × Except E Unit)) (k : κ) :
    okc (pendingGen max p) k = if totc p k < max then okc p k else 0 := by
  have h := countKeyP_filter_key p (fun k => decide (totc p k < max)) k Except.isOk
  simp only [decide_eq_true_eq] at h
  exact h

private theorem b_empty_gen {max : Nat} {p b : Stream (κ × Except E Unit)}
    (hmax : ∀ k, totc (p ++ b) k ≤ max) {k : κ} (hk : max ≤ totc p k) :
    totc b k = 0 := by
  have h1 := hmax k
  rw [totc_append] at h1
  omega

private theorem totc_curGen {max : Nat} {p b : Stream (κ × Except E Unit)}
    (hmax : ∀ k, totc (p ++ b) k ≤ max) (k : κ) :
    totc (curGen max p b) k = if totc p k < max then totc (p ++ b) k else 0 := by
  have hsplit : totc (curGen max p b) k = totc (pendingGen max p) k + totc b k :=
    totc_append _ b k
  by_cases hlt : totc p k < max
  · rw [hsplit, totc_pendingGen, if_pos hlt, if_pos hlt, totc_append]
  · have hb := b_empty_gen hmax (Nat.le_of_not_lt hlt)
    rw [hsplit, totc_pendingGen, if_neg hlt, if_neg hlt, hb]

private theorem okc_curGen {max : Nat} {p b : Stream (κ × Except E Unit)}
    (hmax : ∀ k, totc (p ++ b) k ≤ max) (k : κ) :
    okc (curGen max p b) k = if totc p k < max then okc (p ++ b) k else 0 := by
  have hsplit : okc (curGen max p b) k = okc (pendingGen max p) k + okc b k :=
    okc_append _ b k
  by_cases hlt : totc p k < max
  · rw [hsplit, okc_pendingGen, if_pos hlt, if_pos hlt, okc_append]
  · have hb : okc b k = 0 := by
      have h0 := b_empty_gen hmax (Nat.le_of_not_lt hlt)
      have hle := okc_le_totc b k
      omega
    rw [hsplit, okc_pendingGen, if_neg hlt, if_neg hlt, hb]

private theorem mem_map_fst_curGen {max : Nat} {p b : Stream (κ × Except E Unit)}
    (hmax : ∀ k, totc (p ++ b) k ≤ max) (k : κ) :
    k ∈ (curGen max p b).map Prod.fst ↔
      (k ∈ (p ++ b).map Prod.fst ∧ totc p k < max) := by
  constructor
  · intro hk
    obtain ⟨r, hr, hfst⟩ := List.mem_map.mp hk
    have hpos : 0 < totc (curGen max p b) k := hfst ▸ totc_pos_of_mem hr
    have hlt : totc p k < max := by
      rcases Nat.lt_or_ge (totc p k) max with h | h
      · exact h
      · rw [totc_curGen hmax k, if_neg (Nat.not_lt_of_le h)] at hpos
        omega
    refine ⟨?_, hlt⟩
    rcases List.mem_append.mp hr with hrp | hrb
    · have : r ∈ p := (List.mem_filter.mp hrp).1
      exact List.mem_map.mpr ⟨r, List.mem_append.mpr (Or.inl this), hfst⟩
    · exact List.mem_map.mpr ⟨r, List.mem_append.mpr (Or.inr hrb), hfst⟩
  · rintro ⟨hk, hlt⟩
    obtain ⟨r, hr, hfst⟩ := List.mem_map.mp hk
    rcases List.mem_append.mp hr with hrp | hrb
    · have : r ∈ pendingGen max p :=
        List.mem_filter.mpr ⟨hrp, by simpa [hfst] using hlt⟩
      exact List.mem_map.mpr ⟨r, List.mem_append.mpr (Or.inl this), hfst⟩
    · exact List.mem_map.mpr ⟨r, List.mem_append.mpr (Or.inr hrb), hfst⟩


/-- Keys of `cur` that reached `min` successes (Rust: `reached_min_count`). -/
def reachedMinGen (min max : Nat) (p b : Stream (κ × Except E Unit)) : Stream κ :=
  List.filter (fun k => decide (min ≤ okc (curGen max p b) k)) (curGen max p b).keys

/-- Keys of `cur` that received all `max` responses (Rust: `received_from_all`). -/
def receivedFromAllGen (_min max : Nat) (p b : Stream (κ × Except E Unit)) : Stream κ :=
  List.filter (fun k => decide (max ≤ totc (curGen max p b) k)) (curGen max p b).keys

private theorem mem_reachedMinGen {min max : Nat} {p b : Stream (κ × Except E Unit)} (k : κ) :
    k ∈ reachedMinGen min max p b ↔
      (k ∈ (curGen max p b).map Prod.fst ∧ min ≤ okc (curGen max p b) k) := by
  rw [reachedMinGen, List.mem_filter, mem_keys]
  simp

private theorem mem_receivedFromAllGen {min max : Nat} {p b : Stream (κ × Except E Unit)} (k : κ) :
    k ∈ receivedFromAllGen min max p b ↔
      (k ∈ (curGen max p b).map Prod.fst ∧ max ≤ totc (curGen max p b) k) := by
  rw [receivedFromAllGen, List.mem_filter, mem_keys]
  simp

/-- The batch emitted by one tick of the general branch (Rust:
`reached_min_count.filter_not_in(min_but_not_max)`), for the incoming
`minButNotMax` list `mb`. -/
def emitGen (min max : Nat) (p b : Stream (κ × Except E Unit)) (mb : Stream κ) : Stream κ :=
  (reachedMinGen min max p b).filterNotIn mb

/-- **Emission membership, general branch**: assuming the `minButNotMax`
characterization for prefix `p`, the emitted keys are exactly those that became
qualified during this tick. -/
private theorem mem_emitGen {min max : Nat} (hmin : 1 ≤ min)
    {p b : Stream (κ × Except E Unit)} {mb : Stream κ}
    (hmb : MbnmSpec min max p mb)
    (hmax : ∀ k, totc (p ++ b) k ≤ max) (k : κ) :
    k ∈ emitGen min max p b mb ↔ (min ≤ okc (p ++ b) k ∧ okc p k < min) := by
  have hmem : k ∈ emitGen min max p b mb ↔
      (k ∈ reachedMinGen min max p b ∧ k ∉ mb) := by
    rw [emitGen, Stream.filterNotIn, Stream.filter, List.mem_filter]
    simp
  rw [hmem, mem_reachedMinGen, hmb.2]
  constructor
  · rintro ⟨⟨hkcur, hge⟩, hnmb⟩
    have hlt : totc p k < max := ((mem_map_fst_curGen hmax k).mp hkcur).2
    rw [okc_curGen hmax k, if_pos hlt] at hge
    refine ⟨hge, ?_⟩
    rcases Nat.lt_or_ge (okc p k) min with h | h
    · exact h
    · exfalso
      exact hnmb ⟨mem_map_fst_of_okc_pos (by omega),
        h, hlt⟩
  · rintro ⟨hge, hlt⟩
    -- the batch must contain a response for `k`, so `k` had not received all
    have hokb : 1 ≤ okc b k := by
      have := okc_append p b k
      omega
    have htotb : 1 ≤ totc b k := by
      have := okc_le_totc b k
      omega
    have htotp : totc p k < max := by
      rcases Nat.lt_or_ge (totc p k) max with h | h
      · exact h
      · have := b_empty_gen hmax h
        omega
    have hcur : okc (curGen max p b) k = okc (p ++ b) k := by
      rw [okc_curGen hmax k, if_pos htotp]
    refine ⟨⟨?_, by omega⟩, ?_⟩
    · exact mem_map_fst_of_okc_pos (p := curGen max p b) (by omega)
    · rintro ⟨_, hminp, _⟩
      omega

private theorem nodup_emitGen (min max : Nat) (p b : Stream (κ × Except E Unit))
    (mb : Stream κ) : (emitGen min max p b mb).Nodup :=
  (List.filter_sublist).nodup ((List.filter_sublist).nodup (nodup_keys _))

/-- **`minButNotMax` preservation**: the freshly computed
`reached_min.filter_not_in(received_from_all)` satisfies the characterization
for `p ++ b`. -/
private theorem mbnm_next {min max : Nat} (_hmin : 1 ≤ min)
    {p b : Stream (κ × Except E Unit)}
    (hmax : ∀ k, totc (p ++ b) k ≤ max) :
    MbnmSpec min max (p ++ b)
      ((reachedMinGen min max p b).filterNotIn (receivedFromAllGen min max p b)) := by
  constructor
  · exact (List.filter_sublist).nodup ((List.filter_sublist).nodup (nodup_keys _))
  · intro k
    have hmem : k ∈ (reachedMinGen min max p b).filterNotIn (receivedFromAllGen min max p b) ↔
        (k ∈ reachedMinGen min max p b ∧ k ∉ receivedFromAllGen min max p b) := by
      rw [Stream.filterNotIn, Stream.filter, List.mem_filter]
      simp
    rw [hmem, mem_reachedMinGen, mem_receivedFromAllGen]
    constructor
    · rintro ⟨⟨hkcur, hge⟩, hnfa⟩
      have hlt : totc p k < max := ((mem_map_fst_curGen hmax k).mp hkcur).2
      have hoke : okc (curGen max p b) k = okc (p ++ b) k := by
        rw [okc_curGen hmax k, if_pos hlt]
      have htote : totc (curGen max p b) k = totc (p ++ b) k := by
        rw [totc_curGen hmax k, if_pos hlt]
      refine ⟨((mem_map_fst_curGen hmax k).mp hkcur).1, by omega, ?_⟩
      rcases Nat.lt_or_ge (totc (p ++ b) k) max with h | h
      · exact h
      · exact absurd ⟨hkcur, by omega⟩ hnfa
    · rintro ⟨hkpb, hge, hlt⟩
      have htotp : totc p k < max := by
        have := totc_append p b k
        omega
      have hoke : okc (curGen max p b) k = okc (p ++ b) k := by
        rw [okc_curGen hmax k, if_pos htotp]
      have htote : totc (curGen max p b) k = totc (p ++ b) k := by
        rw [totc_curGen hmax k, if_pos htotp]
      have hkcur : k ∈ (curGen max p b).map Prod.fst :=
        (mem_map_fst_curGen hmax k).mpr ⟨hkpb, htotp⟩
      exact ⟨⟨hkcur, by omega⟩, fun hfa => by omega⟩

/-- **`notAll` preservation, general branch**: dropping keys that received all
`max` responses yields exactly the characterized state for `p ++ b`. -/
private theorem state_gen {min max : Nat} {p b : Stream (κ × Except E Unit)}
    (hmax : ∀ k, totc (p ++ b) k ≤ max) :
    (curGen max p b).antiJoin (receivedFromAllGen min max p b)
      = pendingGen max (p ++ b) := by
  have h1 : (curGen max p b).antiJoin (receivedFromAllGen min max p b)
      = List.filter (fun r => decide (totc (curGen max p b) r.1 < max)) (curGen max p b) := by
    refine List.filter_congr fun r hr => ?_
    have hrmem : r.1 ∈ (curGen max p b).map Prod.fst := List.mem_map.mpr ⟨r, hr, rfl⟩
    by_cases hge : max ≤ totc (curGen max p b) r.1
    · have hin : r.1 ∈ receivedFromAllGen min max p b :=
        (mem_receivedFromAllGen r.1).mpr ⟨hrmem, hge⟩
      simp [hin, Nat.not_lt_of_le hge]
    · have hnin : r.1 ∉ receivedFromAllGen min max p b := by
        intro hmem
        exact hge ((mem_receivedFromAllGen r.1).mp hmem).2
      simp [hnin, Nat.lt_of_not_le hge]
  rw [h1]
  show List.filter _ (pendingGen max p ++ b) = _
  rw [List.filter_append]
  have hp : List.filter (fun r => decide (totc (curGen max p b) r.1 < max))
        (pendingGen max p)
      = List.filter (fun r => decide (totc (p ++ b) r.1 < max)) p := by
    rw [pendingGen, List.filter_filter]
    refine List.filter_congr fun r _ => ?_
    by_cases hlt : totc p r.1 < max
    · have heq : totc (curGen max p b) r.1 = totc (p ++ b) r.1 := by
        rw [totc_curGen hmax r.1, if_pos hlt]
      simp [heq, hlt]
    · have hge' : max ≤ totc (p ++ b) r.1 := by
        have hmono := totc_append p b r.1
        omega
      simp [hlt, Nat.not_lt_of_le hge']
  have hb : List.filter (fun r => decide (totc (curGen max p b) r.1 < max)) b
      = List.filter (fun r => decide (totc (p ++ b) r.1 < max)) b := by
    refine List.filter_congr fun r hr => ?_
    by_cases hlt : totc p r.1 < max
    · have heq : totc (curGen max p b) r.1 = totc (p ++ b) r.1 := by
        rw [totc_curGen hmax r.1, if_pos hlt]
      simp [heq]
    · exfalso
      have h0 := b_empty_gen hmax (Nat.le_of_not_lt hlt)
      have hpos := totc_pos_of_mem hr
      omega
  rw [hp, hb, pendingGen, List.filter_append]

/-- **One-tick reduction, general branch** (`min ≠ max`). -/
private theorem step_gen {min max : Nat} (hne : min ≠ max)
    (p b : Stream (κ × Except E Unit)) (mb : Stream κ) :
    (collectQuorumTick (E := E) min max).step ⟨pendingGen max p, mb⟩ b
      = (⟨(curGen max p b).antiJoin (receivedFromAllGen min max p b),
          (reachedMinGen min max p b).filterNotIn (receivedFromAllGen min max p b)⟩,
         emitGen min max p b mb) := by
  simp only [collectQuorumTick, Stream.chain, curGen, emitGen, reachedMinGen,
    receivedFromAllGen, if_neg hne]
  rfl


/-- **Multi-tick preservation, general branch** (`min ≠ max`). -/
private theorem runFrom_gen {min max : Nat} (hmin : 1 ≤ min) (hne : min ≠ max)
    (bs : List (Stream (κ × Except E Unit))) (p : Stream (κ × Except E Unit))
    (mb : Stream κ) (hmb : MbnmSpec min max p mb)
    (hmax : ∀ k, totc (p ++ bs.flatten) k ≤ max) :
    ((collectQuorumTick (E := E) min max).runFrom ⟨pendingGen max p, mb⟩ bs).1.notAll
      = pendingGen max (p ++ bs.flatten) ∧
    MbnmSpec min max (p ++ bs.flatten)
      ((collectQuorumTick (E := E) min max).runFrom ⟨pendingGen max p, mb⟩ bs).1.minButNotMax ∧
    (((collectQuorumTick (E := E) min max).runFrom ⟨pendingGen max p, mb⟩ bs).2.flatten).Nodup ∧
    ∀ k, k ∈ ((collectQuorumTick (E := E) min max).runFrom ⟨pendingGen max p, mb⟩ bs).2.flatten ↔
      (min ≤ okc (p ++ bs.flatten) k ∧ okc p k < min) := by
  induction bs generalizing p mb with
  | nil =>
    refine ⟨by simp, by simpa using hmb, by simp, fun k => ?_⟩
    simp only [TickLoop.runFrom, List.flatten_nil, List.not_mem_nil, List.append_nil,
      false_iff]
    omega
  | cons b bs ih =>
    have hmax₁ : ∀ k, totc (p ++ b) k ≤ max := by
      intro k
      have h := hmax k
      rw [show p ++ (b :: bs).flatten = (p ++ b) ++ bs.flatten by
        simp [List.flatten_cons, List.append_assoc]] at h
      rw [totc_append] at h
      omega
    have hmax₂ : ∀ k, totc ((p ++ b) ++ bs.flatten) k ≤ max := by
      intro k
      have h := hmax k
      rw [show p ++ (b :: bs).flatten = (p ++ b) ++ bs.flatten by
        simp [List.flatten_cons, List.append_assoc]] at h
      exact h
    have hstep := step_gen (E := E) hne p b mb
    have hstate := state_gen (min := min) hmax₁
    have hmb' := mbnm_next (min := min) hmin hmax₁
    obtain ⟨ihState, ihMb, ihNodup, ihMem⟩ := ih (p ++ b) _ hmb' hmax₂
    have hflat : p ++ (b :: bs).flatten = (p ++ b) ++ bs.flatten := by
      simp [List.flatten_cons, List.append_assoc]
    rw [TickLoop.runFrom_cons, hstep]
    simp only [hstate]
    refine ⟨by rw [ihState, hflat], by rw [hflat]; exact ihMb, ?_, ?_⟩
    · rw [List.flatten_cons, List.nodup_append]
      refine ⟨nodup_emitGen min max p b mb, ihNodup, fun a ha a' ha' => ?_⟩
      have h₁ := (mem_emitGen hmin hmb hmax₁ a).mp ha
      intro heq
      subst heq
      have h₂ := (ihMem a).mp ha'
      omega
    · intro k
      rw [List.flatten_cons, List.mem_append, mem_emitGen hmin hmb hmax₁, ihMem, hflat]
      constructor
      · rintro (⟨hge, hlt⟩ | ⟨hge, hlt⟩)
        · have := okc_append (p ++ b) bs.flatten k
          exact ⟨by omega, hlt⟩
        · have := okc_append p b k
          omega
      · rintro ⟨hge, hlt⟩
        by_cases hmid : min ≤ okc (p ++ b) k
        · exact Or.inl ⟨hmid, hlt⟩
        · exact Or.inr ⟨hge, by omega⟩

/-! ## Assembly -/

/-- Correctness for a full run of either branch, from the empty initial state. -/
private theorem allTicksOutput_characterize {min max : Nat} (hmin : 1 ≤ min)
    (input : Stream (κ × Except E Unit)) (b : Batching (κ × Except E Unit))
    (hb : b.of input) (hmax : AtMostMaxResponses max input) :
    (((collectQuorumTick (κ := κ) (E := E) min max).allTicksOutput b).Nodup) ∧
    ∀ k, k ∈ (collectQuorumTick (κ := κ) (E := E) min max).allTicksOutput b ↔
      min ≤ okc input k := by
  have hmax' : ∀ k, totc (([] : Stream (κ × Except E Unit)) ++ b.flatten) k ≤ max := by
    intro k
    have := hmax k
    simp only [Batching.of] at hb
    simpa [totc, hb] using this
  by_cases hmm : min = max
  · subst hmm
    have h := runFrom_minEqMax (E := E) min hmin b [] hmax'
    have hinit : (collectQuorumTick (κ := κ) (E := E) min min).init
        = ⟨pendingMinEq min [], []⟩ := rfl
    simp only [TickLoop.allTicksOutput, TickLoop.outputs, TickLoop.run, allTicks, hinit]
    simp only [Batching.of] at hb
    refine ⟨h.2.1, fun k => ?_⟩
    rw [h.2.2 k]
    rw [show ([] : Stream (κ × Except E Unit)) ++ b.flatten = input by simpa using hb]
    constructor
    · rintro ⟨hge, _⟩; exact hge
    · intro hge
      refine ⟨hge, show okc ([] : Stream (κ × Except E Unit)) k < min from ?_⟩
      show (0 : Nat) < min
      omega
  · have hmb0 : MbnmSpec min max ([] : Stream (κ × Except E Unit)) [] := by
      refine ⟨List.Pairwise.nil, fun k => ?_⟩
      constructor
      · intro h
        cases h
      · rintro ⟨h, -⟩
        rw [Stream.map, List.map_nil] at h
        cases h
    have h := runFrom_gen (E := E) hmin hmm b [] [] hmb0 hmax'
    have hinit : (collectQuorumTick (κ := κ) (E := E) min max).init
        = ⟨pendingGen max [], []⟩ := rfl
    simp only [TickLoop.allTicksOutput, TickLoop.outputs, TickLoop.run, allTicks, hinit]
    simp only [Batching.of] at hb
    refine ⟨h.2.2.1, fun k => ?_⟩
    rw [h.2.2.2 k]
    rw [show ([] : Stream (κ × Except E Unit)) ++ b.flatten = input by simpa using hb]
    constructor
    · rintro ⟨hge, _⟩; exact hge
    · intro hge
      exact ⟨hge, show okc ([] : Stream (κ × Except E Unit)) k < min by
        show (0 : Nat) < min
        omega⟩

/-- **Unbounded correctness of `collect_quorum`** (dissertation goal (a)):
for all key/error types, all `1 ≤ min ≤ max`, all inputs satisfying the
response contract, and **all batchings** — i.e. every materialization of the
`nondet!` batching guard — the emitted keys are exactly the qualified keys,
each exactly once. This discharges, once and for all inputs, the justification
written inside the Rust `nondet!(...)` in `hydro_std/src/quorum.rs`. -/
theorem collectQuorum_correct : CollectQuorumCorrect := by
  intro κ E _ min max hmin hminmax input b hb hmax
  obtain ⟨hnodup, hmem⟩ :=
    allTicksOutput_characterize (min := min) (max := max) hmin input b hb hmax
  refine ⟨hnodup, fun k => ?_⟩
  rw [hmem k]
  constructor
  · intro hge
    refine ⟨?_, hge⟩
    have : 0 < okc input k := by omega
    exact mem_map_fst_of_okc_pos this
  · rintro ⟨_, hge⟩
    exact hge

/-- **Determinism corollary**: any two batchings of the same input yield
outputs that are permutations of each other — the settled output of
`collect_quorum` is a well-defined *multiset* (`NoOrder` stream), independent
of the adversarial batching. This is `Theorem 3.4.1`-style eventual
determinism materialized for a concrete program. -/
theorem collectQuorum_deterministic (κ E : Type) [DecidableEq κ]
    (min max : Nat) (hmin : 1 ≤ min) (_hminmax : min ≤ max)
    (input : Stream (κ × Except E Unit)) (b₁ b₂ : Batching (κ × Except E Unit))
    (h₁ : b₁.of input) (h₂ : b₂.of input) (hmax : AtMostMaxResponses max input) :
    ((collectQuorumTick (κ := κ) (E := E) min max).allTicksOutput b₁).Perm
      ((collectQuorumTick (κ := κ) (E := E) min max).allTicksOutput b₂) := by
  obtain ⟨hn₁, hm₁⟩ := allTicksOutput_characterize (max := max) hmin input b₁ h₁ hmax
  obtain ⟨hn₂, hm₂⟩ := allTicksOutput_characterize (max := max) hmin input b₂ h₂ hmax
  exact (List.perm_ext_iff_of_nodup hn₁ hn₂).mpr fun k => by rw [hm₁ k, hm₂ k]

end HydroLean.Programs


