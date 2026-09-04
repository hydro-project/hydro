import HydroV2.Std.QuorumTheory
import HydroV2.Values
import HydroV2.HydroDef

/-!
# `hydro_std/src/quorum.rs` — the shared verified quorum stage

One Rust function = one Lean definition: `collect_quorum`
(quorum.rs:90–160) and `collect_quorum_with_response` (quorum.rs:7–88),
each carrying its colocated contract. The `sliced!` register machine
(`CQState`, `cqTick`, `cqwrTick`) is defined ONCE, below the contracts,
and the program bodies fold with it — no spec mirror. Every protocol
that collects quorums consumes THIS module (Paxos phase 1 via
`collect_quorum_with_response` in `p_p1b`, phase 2 via `collect_quorum`
in `sequence_payload`) — the single shared quorum stage.

The one `nondet!` site per collector is its `use::batch` (`BatchCuts`):
"we always persist values that have not reached quorum, so even with
arbitrary batching we always produce deterministic quorum results" —
the contracts below make that comment a theorem. The usage contract
(quorum.rs's deployment assumption, hypotheses of the capped clauses):
`1 ≤ min ≤ max` and no key receives more than `max` responses.

Layout: contracts → the register machine (core logic) → step
obligations (per-tick case analyses over `cqTick`/`cqwrTick`) → run
facts (one generic `Trace.lean` scan-combinator application each) →
the program definitions → smoke tests. Contract vocabulary and pool
algebra: `Std/QuorumTheory.lean`.
-/

namespace HydroV2

variable {L : Type} {mem : L → Nat}
variable {K E V : Type} [DecidableEq K] [DecidableEq E] [DecidableEq V]

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

/-! ## The `sliced!` register machine (quorum.rs core logic) -/

/-- The `sliced!` state of quorum.rs's collectors: the responses of keys
still below `max` (`not_all`) and the keys at or above `min` but below
`max` (`min_but_not_max`, used only when `min < max`). -/
structure CQState (K E V : Type) where
  notAll : Multiset (K × Except E V)
  minButNotMax : Multiset K

/-- `use::state_null` for both registers. -/
def CQState.init : CQState K E V := ⟨0, 0⟩

/-- One tick of `collect_quorum` (quorum.rs:92–152), Rust-literal:
count per key over kept + new responses, emit the keys just reaching
`min` successes, persist below-`max` responses. -/
def cqTick (min max : Nat) (s : CQState K E Unit)
    (new_inputs : Multiset (K × Except E Unit)) :
    CQState K E Unit × Multiset K :=
  -- `current_responses = not_all.chain(new_inputs)` is `s.notAll +
  -- new_inputs`; `count_per_key` (the keyed commutative counter fold —
  -- quorum.rs's `manual_proof!`) is `cqOkCount`/`cqKeyCount` of it;
  -- `reached_min_count` is its keys' dedup filtered at `min ≤ ok`.
  if min = max then
    -- not_all = current_responses.anti_join(reached_min_count);
    -- just_reached_quorum = reached_min_count
    (⟨(s.notAll + new_inputs).filter
        (fun r => ¬ min ≤ cqOkCount (s.notAll + new_inputs) r.1),
      s.minButNotMax⟩,
     ((s.notAll + new_inputs).map Prod.fst).dedup.filter
       (fun k => min ≤ cqOkCount (s.notAll + new_inputs) k))
  else
    -- received_from_all = count_per_key.filter(success+error >= max).keys();
    -- not_all = current_responses.anti_join(received_from_all);
    -- out = reached_min_count.filter_not_in(min_but_not_max);
    -- min_but_not_max = reached_min_count.filter_not_in(received_from_all)
    (⟨(s.notAll + new_inputs).filter
        (fun r => ¬ max ≤ cqKeyCount (s.notAll + new_inputs) r.1),
      (((s.notAll + new_inputs).map Prod.fst).dedup.filter
        (fun k => min ≤ cqOkCount (s.notAll + new_inputs) k)).filter
        (fun k => ¬ max ≤ cqKeyCount (s.notAll + new_inputs) k)⟩,
     (((s.notAll + new_inputs).map Prod.fst).dedup.filter
       (fun k => min ≤ cqOkCount (s.notAll + new_inputs) k)).filter
       (fun k => s.minButNotMax.count k = 0))

/-- One tick of `collect_quorum_with_response` (quorum.rs:7–76),
Rust-literal: same registers; the emission is the just-reached keys'
`Ok` **responses** (`filter_map(Ok(v) → (key, v))`). -/
def cqwrTick (min max : Nat) (s : CQState K E V)
    (new_inputs : Multiset (K × Except E V)) :
    CQState K E V × Multiset (K × V) :=
  -- `current_responses = not_all.chain(new_inputs)` is `s.notAll +
  -- new_inputs` throughout.
  if min = max then
    -- not_all = current_responses.anti_join(reached_min_count);
    -- out = current_responses.anti_join(not_reached_min_count)
    --         .filter_map(Ok(v) → (key, v))
    (⟨(s.notAll + new_inputs).filter
        (fun r => ¬ min ≤ cqOkCount (s.notAll + new_inputs) r.1),
      s.minButNotMax⟩,
     ((s.notAll + new_inputs).filter
        (fun r => min ≤ cqOkCount (s.notAll + new_inputs) r.1)).filterMap
       cqOkProj)
  else
    -- not_all = current_responses.anti_join(received_from_all);
    -- out = current_responses.anti_join(not_reached_min_count)
    --         .anti_join(min_but_not_max).filter_map(Ok(v) → (key, v));
    -- min_but_not_max = reached_min_count.filter_not_in(received_from_all)
    (⟨(s.notAll + new_inputs).filter
        (fun r => ¬ max ≤ cqKeyCount (s.notAll + new_inputs) r.1),
      (((s.notAll + new_inputs).map Prod.fst).dedup).filter
        (fun k => min ≤ cqOkCount (s.notAll + new_inputs) k
          ∧ ¬ max ≤ cqKeyCount (s.notAll + new_inputs) k)⟩,
     ((s.notAll + new_inputs).filter
        (fun r => min ≤ cqOkCount (s.notAll + new_inputs) r.1
          ∧ s.minButNotMax.count r.1 = 0)).filterMap cqOkProj)

/-! ## Step obligations (per-tick facts about `cqTick`/`cqwrTick`) -/

/-- Per-tick emission soundness: an emitted key holds `min` `Ok` votes
in the tick's window. -/
theorem cqTick_emit_ok (min max : Nat) (s : CQState K E Unit)
    (b : Multiset (K × Except E Unit)) {k : K}
    (hk : k ∈ (cqTick min max s b).2) :
    min ≤ cqOkCount (s.notAll + b) k := by
  unfold cqTick at hk
  by_cases hmm : min = max
  · rw [if_pos hmm] at hk
    exact (Multiset.mem_filter.mp hk).2
  · rw [if_neg hmm] at hk
    exact (Multiset.mem_filter.mp (Multiset.mem_filter.mp hk).1).2

/-- Per-tick emission soundness: an emitted response quotes a consumed
`Ok` response of a key holding `min` votes in the tick's window. -/
theorem cqwrTick_emit_ok (min max : Nat) (s : CQState K E V)
    (b : Multiset (K × Except E V)) {r : K × V}
    (hr : r ∈ (cqwrTick min max s b).2) :
    (r.1, .ok r.2) ∈ s.notAll + b
      ∧ min ≤ cqOkCount (s.notAll + b) r.1 := by
  have hfm : ∃ x ∈ (s.notAll + b), cqOkProj x = some r
      ∧ min ≤ cqOkCount (s.notAll + b) x.1 := by
    unfold cqwrTick at hr
    by_cases hmm : min = max
    · rw [if_pos hmm] at hr
      obtain ⟨x, hx, hproj⟩ := (Multiset.mem_filterMap _ _).mp hr
      have := Multiset.mem_filter.mp hx
      exact ⟨x, this.1, hproj, this.2⟩
    · rw [if_neg hmm] at hr
      obtain ⟨x, hx, hproj⟩ := (Multiset.mem_filterMap _ _).mp hr
      have := Multiset.mem_filter.mp hx
      exact ⟨x, this.1, hproj, this.2.1⟩
  obtain ⟨x, hx, hproj, hok⟩ := hfm
  have hshape : x = ((r.1, .ok r.2) : K × Except E V) := by
    obtain ⟨xk, xres⟩ := x
    cases xres with
    | ok v' =>
      have hp : ((xk, v') : K × V) = r := Option.some.inj hproj
      rw [show xk = r.1 from congrArg Prod.fst hp,
        show v' = r.2 from congrArg Prod.snd hp]
    | error e => cases hproj
  rw [hshape] at hx hok
  exact ⟨hx, hok⟩

/-- The potential step (unconditional): a tick's key-filtered emission
plus the next tick's retained key window are bounded by the current
window plus the batch's contribution at the key — and the register
invariant (`min_but_not_max` empty at the key, or the key's retained
window already at `min`) survives. The `scan_bound` obligation behind
`cqwr_run_le_init`. -/
theorem cqwrTick_bound_step (min max : Nat) (k : K)
    (s : CQState K E V) (b : Multiset (K × Except E V))
    (hinv : s.minButNotMax.count k = 0
      ∨ (min ≠ max ∧ min ≤ cqOkCount s.notAll k)) :
    ((cqwrTick min max s b).1.minButNotMax.count k = 0
      ∨ (min ≠ max
          ∧ min ≤ cqOkCount (cqwrTick min max s b).1.notAll k))
    ∧ (cqwrTick min max s b).2.filter (fun r => r.1 = k)
        + (if (cqwrTick min max s b).1.minButNotMax.count k = 0
            then (((cqwrTick min max s b).1.notAll).filter
              (fun r => r.1 = k)).filterMap cqOkProj
            else 0)
      ≤ (if s.minButNotMax.count k = 0
          then (s.notAll.filter (fun r => r.1 = k)).filterMap cqOkProj
          else 0)
        + (b.filter (fun r => r.1 = k)).filterMap cqOkProj := by
  by_cases hmm : min = max
  · -- the `min = max` branch: `min_but_not_max` is never touched
    have hek : s.minButNotMax.count k = 0 := by
      rcases hinv with h | h
      · exact h
      · exact absurd hmm h.1
    have hst : (cqwrTick min max s b).1
        = ⟨(s.notAll + b).filter
            (fun r => ¬ min ≤ cqOkCount (s.notAll + b) r.1),
           s.minButNotMax⟩ := by
      unfold cqwrTick; rw [if_pos hmm]
    have hout_k : ((cqwrTick min max s b).2).filter (fun r => r.1 = k)
        = if min ≤ cqOkCount (s.notAll + b) k
          then ((s.notAll + b).filter
            (fun r => r.1 = k)).filterMap cqOkProj
          else 0 := by
      rw [show (cqwrTick min max s b).2
          = ((s.notAll + b).filter
              (fun r => min ≤ cqOkCount (s.notAll + b) r.1)).filterMap
            cqOkProj from by unfold cqwrTick; rw [if_pos hmm],
        filterMap_okProj_kpart_comm,
        filter_key_of_pred _
          (fun k' => min ≤ cqOkCount (s.notAll + b) k') k]
      by_cases hc : min ≤ cqOkCount (s.notAll + b) k
      · rw [if_pos hc, if_pos hc]
      · rw [if_neg hc, if_neg hc, Multiset.filterMap_zero]
    have hwin' : ((cqwrTick min max s b).1.notAll).filter
          (fun r => r.1 = k)
        = if ¬ min ≤ cqOkCount (s.notAll + b) k
          then (s.notAll + b).filter (fun r => r.1 = k) else 0 := by
      rw [hst]
      exact filter_key_of_pred _
        (fun k' => ¬ min ≤ cqOkCount (s.notAll + b) k') k
    have hek' : (cqwrTick min max s b).1.minButNotMax.count k = 0 := by
      rw [hst]; exact hek
    have hB' : (if (cqwrTick min max s b).1.minButNotMax.count k = 0
          then (((cqwrTick min max s b).1.notAll).filter
            (fun r => r.1 = k)).filterMap cqOkProj
          else 0)
        = if ¬ min ≤ cqOkCount (s.notAll + b) k
          then ((s.notAll + b).filter (fun r => r.1 = k)).filterMap
            cqOkProj
          else 0 := by
      rw [if_pos hek', hwin']
      by_cases hc : min ≤ cqOkCount (s.notAll + b) k
      · rw [if_neg (not_not_intro hc), if_neg (not_not_intro hc),
          Multiset.filterMap_zero]
      · rw [if_pos hc, if_pos hc]
    have hrhs : (if s.minButNotMax.count k = 0
          then (s.notAll.filter (fun r => r.1 = k)).filterMap cqOkProj
          else 0)
        + (b.filter (fun r => r.1 = k)).filterMap cqOkProj
        = ((s.notAll + b).filter (fun r => r.1 = k)).filterMap
            cqOkProj := by
      rw [if_pos hek, Multiset.filter_add, Multiset.filterMap_add]
    refine ⟨Or.inl hek', ?_⟩
    rw [hout_k, hB', hrhs]
    by_cases hc : min ≤ cqOkCount (s.notAll + b) k
    · rw [if_pos hc, if_neg (not_not_intro hc), Multiset.add_zero]
    · rw [if_neg hc, if_pos hc, Multiset.zero_add]
  · -- the `min < max` branch: the three-register step
    have hst : (cqwrTick min max s b).1
        = ⟨(s.notAll + b).filter
            (fun r => ¬ max ≤ cqKeyCount (s.notAll + b) r.1),
           (((s.notAll + b).map Prod.fst).dedup).filter
            (fun k' => min ≤ cqOkCount (s.notAll + b) k'
              ∧ ¬ max ≤ cqKeyCount (s.notAll + b) k')⟩ := by
      unfold cqwrTick; rw [if_neg hmm]
    have hout_k : ((cqwrTick min max s b).2).filter (fun r => r.1 = k)
        = if (min ≤ cqOkCount (s.notAll + b) k
            ∧ s.minButNotMax.count k = 0)
          then ((s.notAll + b).filter
            (fun r => r.1 = k)).filterMap cqOkProj
          else 0 := by
      rw [show (cqwrTick min max s b).2
          = ((s.notAll + b).filter
              (fun r => min ≤ cqOkCount (s.notAll + b) r.1
                ∧ s.minButNotMax.count r.1 = 0)).filterMap cqOkProj
            from by unfold cqwrTick; rw [if_neg hmm],
        filterMap_okProj_kpart_comm,
        filter_key_of_pred _
          (fun k' => min ≤ cqOkCount (s.notAll + b) k'
            ∧ s.minButNotMax.count k' = 0) k]
      by_cases hc : min ≤ cqOkCount (s.notAll + b) k
          ∧ s.minButNotMax.count k = 0
      · rw [if_pos hc, if_pos hc]
      · rw [if_neg hc, if_neg hc, Multiset.filterMap_zero]
    have hwin' : ((cqwrTick min max s b).1.notAll).filter
          (fun r => r.1 = k)
        = if ¬ max ≤ cqKeyCount (s.notAll + b) k
          then (s.notAll + b).filter (fun r => r.1 = k) else 0 := by
      rw [hst]
      exact filter_key_of_pred _
        (fun k' => ¬ max ≤ cqKeyCount (s.notAll + b) k') k
    have hek' : (cqwrTick min max s b).1.minButNotMax.count k
        = if k ∈ (s.notAll + b).map Prod.fst
            ∧ (min ≤ cqOkCount (s.notAll + b) k
              ∧ ¬ max ≤ cqKeyCount (s.notAll + b) k)
          then 1 else 0 := by
      rw [hst]
      exact count_dedup_keys_filter _
        (fun k' => min ≤ cqOkCount (s.notAll + b) k'
          ∧ ¬ max ≤ cqKeyCount (s.notAll + b) k') k
    by_cases hek : s.minButNotMax.count k = 0
    · have hrhs : (if s.minButNotMax.count k = 0
            then (s.notAll.filter (fun r => r.1 = k)).filterMap cqOkProj
            else 0)
          + (b.filter (fun r => r.1 = k)).filterMap cqOkProj
          = ((s.notAll + b).filter (fun r => r.1 = k)).filterMap
              cqOkProj := by
        rw [if_pos hek, Multiset.filter_add, Multiset.filterMap_add]
      by_cases hdrop : max ≤ cqKeyCount (s.notAll + b) k
      · -- overflow reset: registers drop; the retained window is empty
        have hek'0 : (cqwrTick min max s b).1.minButNotMax.count k
            = 0 := by
          rw [hek', if_neg (fun hc => hc.2.2 hdrop)]
        have hB' : (if (cqwrTick min max s b).1.minButNotMax.count k = 0
              then (((cqwrTick min max s b).1.notAll).filter
                (fun r => r.1 = k)).filterMap cqOkProj
              else 0) = 0 := by
          rw [if_pos hek'0, hwin', if_neg (not_not_intro hdrop),
            Multiset.filterMap_zero]
        refine ⟨Or.inl hek'0, ?_⟩
        rw [hout_k, hB', hrhs, Multiset.add_zero]
        by_cases hcross : min ≤ cqOkCount (s.notAll + b) k
        · rw [if_pos ⟨hcross, hek⟩]
        · rw [if_neg (fun hc => hcross hc.1)]
          exact Multiset.zero_le _
      · by_cases hcross : min ≤ cqOkCount (s.notAll + b) k
        · by_cases hmemk : k ∈ (s.notAll + b).map Prod.fst
          · -- crossing: emit the window; the register locks the key
            have hek'1 : (cqwrTick min max s b).1.minButNotMax.count k
                = 1 := by
              rw [hek', if_pos ⟨hmemk, hcross, hdrop⟩]
            have hokwin : min
                ≤ cqOkCount ((cqwrTick min max s b).1.notAll) k := by
              rw [← cqOkCount_kpart, hwin', if_pos hdrop,
                cqOkCount_kpart]
              exact hcross
            have hB' : (if (cqwrTick min max s b).1.minButNotMax.count k
                    = 0
                  then (((cqwrTick min max s b).1.notAll).filter
                    (fun r => r.1 = k)).filterMap cqOkProj
                  else 0) = 0 := by
              rw [if_neg (by rw [hek'1]; exact Nat.one_ne_zero)]
            refine ⟨Or.inr ⟨hmm, hokwin⟩, ?_⟩
            rw [hout_k, hB', hrhs, Multiset.add_zero,
              if_pos ⟨hcross, hek⟩]
          · -- vacuous crossing (`min = 0`, no responses at the key)
            have hkc0 : cqKeyCount (s.notAll + b) k = 0 := by
              by_contra hc
              exact hmemk ((cq_mem_keys_iff _ k).mpr
                (Nat.pos_of_ne_zero hc))
            have hcur0 : (s.notAll + b).filter (fun r => r.1 = k)
                = 0 := kpart_eq_zero_of_keyCount hkc0
            have hek'0 : (cqwrTick min max s b).1.minButNotMax.count k
                = 0 := by
              rw [hek', if_neg (fun hc => hmemk hc.1)]
            have hB' : (if (cqwrTick min max s b).1.minButNotMax.count k
                    = 0
                  then (((cqwrTick min max s b).1.notAll).filter
                    (fun r => r.1 = k)).filterMap cqOkProj
                  else 0) = 0 := by
              rw [if_pos hek'0, hwin', if_pos hdrop, hcur0,
                Multiset.filterMap_zero]
            refine ⟨Or.inl hek'0, ?_⟩
            rw [hout_k, hB', hrhs, Multiset.add_zero,
              if_pos ⟨hcross, hek⟩, hcur0]
        · -- below `min`: nothing emitted; the window carries over
          have hek'0 : (cqwrTick min max s b).1.minButNotMax.count k
              = 0 := by
            rw [hek', if_neg (fun hc => hcross hc.2.1)]
          have hB' : (if (cqwrTick min max s b).1.minButNotMax.count k
                  = 0
                then (((cqwrTick min max s b).1.notAll).filter
                  (fun r => r.1 = k)).filterMap cqOkProj
                else 0)
              = ((s.notAll + b).filter (fun r => r.1 = k)).filterMap
                  cqOkProj := by
            rw [if_pos hek'0, hwin', if_pos hdrop]
          refine ⟨Or.inl hek'0, ?_⟩
          rw [hout_k, hB', hrhs, if_neg (fun hc => hcross hc.1),
            Multiset.zero_add]
    · -- already emitted (`min_but_not_max` holds the key): silent tick
      obtain ⟨-, hokw⟩ := hinv.resolve_left hek
      have hout0 : ((cqwrTick min max s b).2).filter (fun r => r.1 = k)
          = 0 := by
        rw [hout_k, if_neg (fun hc => hek hc.2)]
      have hrhs : (if s.minButNotMax.count k = 0
            then (s.notAll.filter (fun r => r.1 = k)).filterMap cqOkProj
            else 0)
          + (b.filter (fun r => r.1 = k)).filterMap cqOkProj
          = (b.filter (fun r => r.1 = k)).filterMap cqOkProj := by
        rw [if_neg hek, Multiset.zero_add]
      by_cases hdrop : max ≤ cqKeyCount (s.notAll + b) k
      · have hek'0 : (cqwrTick min max s b).1.minButNotMax.count k
            = 0 := by
          rw [hek', if_neg (fun hc => hc.2.2 hdrop)]
        have hB' : (if (cqwrTick min max s b).1.minButNotMax.count k = 0
              then (((cqwrTick min max s b).1.notAll).filter
                (fun r => r.1 = k)).filterMap cqOkProj
              else 0) = 0 := by
          rw [if_pos hek'0, hwin', if_neg (not_not_intro hdrop),
            Multiset.filterMap_zero]
        refine ⟨Or.inl hek'0, ?_⟩
        rw [hout0, hB', hrhs, Multiset.add_zero]
        exact Multiset.zero_le _
      · have hcross : min ≤ cqOkCount (s.notAll + b) k :=
          le_trans hokw (cqOkCount_mono (Multiset.le_add_right ..) k)
        by_cases hmemk : k ∈ (s.notAll + b).map Prod.fst
        · have hek'1 : (cqwrTick min max s b).1.minButNotMax.count k
              = 1 := by
            rw [hek', if_pos ⟨hmemk, hcross, hdrop⟩]
          have hokwin : min
              ≤ cqOkCount ((cqwrTick min max s b).1.notAll) k := by
            rw [← cqOkCount_kpart, hwin', if_pos hdrop, cqOkCount_kpart]
            exact hcross
          have hB' : (if (cqwrTick min max s b).1.minButNotMax.count k
                  = 0
                then (((cqwrTick min max s b).1.notAll).filter
                  (fun r => r.1 = k)).filterMap cqOkProj
                else 0) = 0 := by
            rw [if_neg (by rw [hek'1]; exact Nat.one_ne_zero)]
          refine ⟨Or.inr ⟨hmm, hokwin⟩, ?_⟩
          rw [hout0, hB', hrhs, Multiset.add_zero]
          exact Multiset.zero_le _
        · have hkc0 : cqKeyCount (s.notAll + b) k = 0 := by
            by_contra hc
            exact hmemk ((cq_mem_keys_iff _ k).mpr
              (Nat.pos_of_ne_zero hc))
          have hcur0 : (s.notAll + b).filter (fun r => r.1 = k)
              = 0 := kpart_eq_zero_of_keyCount hkc0
          have hek'0 : (cqwrTick min max s b).1.minButNotMax.count k
              = 0 := by
            rw [hek', if_neg (fun hc => hmemk hc.1)]
          have hB' : (if (cqwrTick min max s b).1.minButNotMax.count k
                  = 0
                then (((cqwrTick min max s b).1.notAll).filter
                  (fun r => r.1 = k)).filterMap cqOkProj
                else 0) = 0 := by
            rw [if_pos hek'0, hwin', if_pos hdrop, hcur0,
              Multiset.filterMap_zero]
          refine ⟨Or.inl hek'0, ?_⟩
          rw [hout0, hB', hrhs, Multiset.add_zero]
          exact Multiset.zero_le _

/-- The window never grows beyond the tick's available responses. -/
theorem cqTick_notAll_le (min max : Nat) (s : CQState K E Unit)
    (b : Multiset (K × Except E Unit)) :
    (cqTick min max s b).1.notAll ≤ s.notAll + b := by
  unfold cqTick
  by_cases hmm : min = max
  · rw [if_pos hmm]
    exact Multiset.filter_le _ _
  · rw [if_neg hmm]
    exact Multiset.filter_le _ _

theorem cqwrTick_notAll_le [DecidableEq E] [DecidableEq V]
    (min max : Nat) (s : CQState K E V)
    (b : Multiset (K × Except E V)) :
    (cqwrTick min max s b).1.notAll ≤ s.notAll + b := by
  unfold cqwrTick
  by_cases hmm : min = max
  · rw [if_pos hmm]
    exact Multiset.filter_le _ _
  · rw [if_neg hmm]
    exact Multiset.filter_le _ _

/-- The per-key register characterization after consuming `pfx`: the
window holds exactly the key's consumed responses until the drop point,
and `min_but_not_max` names exactly the keys at `min` `Ok`s but below
`max` responses. -/
def CQKeyGood (min max : Nat) (s : CQState K E V)
    (pfx : Multiset (K × Except E V)) (k : K) : Prop :=
  (s.notAll.filter (fun r => r.1 = k)
    = if cqDropped min max pfx k then 0
      else pfx.filter (fun r => r.1 = k))
  ∧ (min < max →
      (0 < s.minButNotMax.count k
        ↔ (min ≤ cqOkCount pfx k ∧ cqKeyCount pfx k < max)))

/-- `use::state_null` is per-key good at the empty consumption. -/
theorem CQKeyGood_init (min max : Nat) (h1 : 1 ≤ min) (k : K) :
    CQKeyGood min max (CQState.init (K := K) (E := E) (V := V)) 0 k := by
  constructor
  · show Multiset.filter _ 0 = _
    rw [Multiset.filter_zero]
    by_cases hd : cqDropped min max (0 : Multiset (K × Except E V)) k
    · rw [if_pos hd]
    · rw [if_neg hd]
  · intro _
    constructor
    · intro hc
      exact absurd rfl (Nat.ne_of_gt hc)
    · rintro ⟨hok, -⟩
      refine absurd (le_trans h1 hok) ?_
      show ¬ 1 ≤ cqOkCount 0 k
      unfold cqOkCount
      rw [Multiset.filter_zero, Multiset.card_zero]
      omega

/-- Under the cap, a dropped key receives no further responses. -/
theorem cq_dropped_kills_batch {min max : Nat}
    {pfx b : Multiset (K × Except E V)} {k : K}
    (hcap : cqKeyCount (pfx + b) k ≤ max)
    (hd : cqDropped min max pfx k) :
    b.filter (fun r => r.1 = k) = 0 := by
  refine kpart_eq_zero_of_keyCount ?_
  rw [cqKeyCount_add] at hcap
  have hol := cqOkCount_le_keyCount pfx k
  unfold cqDropped at hd
  rcases hd with ⟨hmm, hok⟩ | ⟨-, hkey⟩ <;> omega

/-- Before the drop point the window's key part is the consumed key
part. -/
theorem cq_current_kpart {min max : Nat} {s : CQState K E V}
    {pfx : Multiset (K × Except E V)} {k : K}
    (hna : s.notAll.filter (fun r => r.1 = k)
      = if cqDropped min max pfx k then 0
        else pfx.filter (fun r => r.1 = k))
    (hnd : ¬ cqDropped min max pfx k)
    (b : Multiset (K × Except E V)) :
    (s.notAll + b).filter (fun r => r.1 = k)
      = (pfx + b).filter (fun r => r.1 = k) := by
  rw [Multiset.filter_add, Multiset.filter_add, hna, if_neg hnd]

/-- Past the drop point the window's key part is empty (under the
cap). -/
theorem cq_current_kpart_dropped {min max : Nat} {s : CQState K E V}
    {pfx b : Multiset (K × Except E V)} {k : K}
    (hna : s.notAll.filter (fun r => r.1 = k)
      = if cqDropped min max pfx k then 0
        else pfx.filter (fun r => r.1 = k))
    (hcap : cqKeyCount (pfx + b) k ≤ max)
    (hd : cqDropped min max pfx k) :
    (s.notAll + b).filter (fun r => r.1 = k) = 0 := by
  rw [Multiset.filter_add, hna, if_pos hd,
    cq_dropped_kills_batch hcap hd]
  rfl

/-- One consumed batch, per key: the registers stay characterized and
the emission count is exactly the crossing indicator. -/
theorem cqTick_key_step (min max : Nat) (h1 : 1 ≤ min)
    (hminmax : min ≤ max)
    (s : CQState K E Unit) (pfx b : Multiset (K × Except E Unit))
    (k : K) (hcap : cqKeyCount (pfx + b) k ≤ max)
    (hg : CQKeyGood min max s pfx k) :
    CQKeyGood min max (cqTick min max s b).1 (pfx + b) k
    ∧ ((cqTick min max s b).2.count k
        = if min ≤ cqOkCount (pfx + b) k ∧ ¬ min ≤ cqOkCount pfx k
          then 1 else 0) := by
  obtain ⟨hna, hmb⟩ := hg
  have hokmono : cqOkCount pfx k ≤ cqOkCount (pfx + b) k := by
    rw [cqOkCount_add]
    omega
  by_cases hmm : min = max
  case pos =>
    have hst : (cqTick min max s b).1
        = ⟨(s.notAll + b).filter
            (fun r => ¬ min ≤ cqOkCount (s.notAll + b) r.1),
          s.minButNotMax⟩ := by
      unfold cqTick
      rw [if_pos hmm]
    have hout : (cqTick min max s b).2
        = ((s.notAll + b).map Prod.fst).dedup.filter
            (fun k => min ≤ cqOkCount (s.notAll + b) k) := by
      unfold cqTick
      rw [if_pos hmm]
    by_cases hd : cqDropped min max pfx k
    case pos =>
      have hokp : min ≤ cqOkCount pfx k :=
        (cqDropped_of_eq hmm pfx k).mp hd
      have hcur0 : (s.notAll + b).filter (fun r => r.1 = k) = 0 :=
        cq_current_kpart_dropped hna hcap hd
      have hok0 : cqOkCount (s.notAll + b) k = 0 :=
        cqOkCount_eq_zero_of_kpart hcur0
      have hd' : cqDropped min max (pfx + b) k := by
        rw [cqDropped_of_eq hmm]
        omega
      refine ⟨⟨?_, fun hlt => absurd hmm (Nat.ne_of_lt hlt)⟩, ?_⟩
      · rw [hst]
        show ((s.notAll + b).filter
            (fun r => ¬ min ≤ cqOkCount (s.notAll + b) r.1)).filter
            (fun r => r.1 = k) = _
        rw [filter_key_of_pred _
            (fun k' => ¬ min ≤ cqOkCount (s.notAll + b) k') k,
          if_pos hd']
        by_cases hpk : ¬ min ≤ cqOkCount (s.notAll + b) k
        · rw [if_pos hpk, hcur0]
        · rw [if_neg hpk]
      · rw [hout, count_dedup_keys_filter,
          if_neg (show ¬(k ∈ (s.notAll + b).map Prod.fst
              ∧ min ≤ cqOkCount (s.notAll + b) k) from
            fun hc => absurd hc.2 (by omega)),
          if_neg (show ¬(min ≤ cqOkCount (pfx + b) k
              ∧ ¬ min ≤ cqOkCount pfx k) from
            fun hc => hc.2 hokp)]
    case neg =>
      have hcur : (s.notAll + b).filter (fun r => r.1 = k)
          = (pfx + b).filter (fun r => r.1 = k) :=
        cq_current_kpart hna hd b
      have hokc : cqOkCount (s.notAll + b) k = cqOkCount (pfx + b) k :=
        cqOkCount_eq_of_kpart_eq hcur
      have hnokp : ¬ min ≤ cqOkCount pfx k := by
        intro hc
        exact hd ((cqDropped_of_eq hmm pfx k).mpr hc)
      refine ⟨⟨?_, fun hlt => absurd hmm (Nat.ne_of_lt hlt)⟩, ?_⟩
      · rw [hst]
        show ((s.notAll + b).filter
            (fun r => ¬ min ≤ cqOkCount (s.notAll + b) r.1)).filter
            (fun r => r.1 = k) = _
        rw [filter_key_of_pred _
          (fun k' => ¬ min ≤ cqOkCount (s.notAll + b) k') k]
        by_cases hcross : min ≤ cqOkCount (pfx + b) k
        · rw [if_neg (show ¬¬ min ≤ cqOkCount (s.notAll + b) k by
              rw [hokc]
              omega),
            if_pos ((cqDropped_of_eq hmm (pfx + b) k).mpr hcross)]
        · rw [if_pos (show ¬ min ≤ cqOkCount (s.notAll + b) k by
              rw [hokc]
              omega),
            hcur,
            if_neg (show ¬ cqDropped min max (pfx + b) k from fun hc =>
              hcross ((cqDropped_of_eq hmm (pfx + b) k).mp hc))]
      · rw [hout, count_dedup_keys_filter]
        by_cases hcross : min ≤ cqOkCount (pfx + b) k
        · rw [if_pos (show k ∈ (s.notAll + b).map Prod.fst
                ∧ min ≤ cqOkCount (s.notAll + b) k from
              ⟨cq_mem_keys_of_ok (by omega), by rw [hokc]; exact hcross⟩),
            if_pos (show min ≤ cqOkCount (pfx + b) k
                ∧ ¬ min ≤ cqOkCount pfx k from ⟨hcross, hnokp⟩)]
        · rw [if_neg (show ¬(k ∈ (s.notAll + b).map Prod.fst
                ∧ min ≤ cqOkCount (s.notAll + b) k) from
              fun hc => hcross (by
                have h2 := hc.2
                omega)),
            if_neg (show ¬(min ≤ cqOkCount (pfx + b) k
                ∧ ¬ min ≤ cqOkCount pfx k) from
              fun hc => hcross hc.1)]
  case neg =>
    have hltmm : min < max := Nat.lt_of_le_of_ne hminmax hmm
    have hst : (cqTick min max s b).1
        = ⟨(s.notAll + b).filter
            (fun r => ¬ max ≤ cqKeyCount (s.notAll + b) r.1),
          (((s.notAll + b).map Prod.fst).dedup.filter
            (fun k => min ≤ cqOkCount (s.notAll + b) k)).filter
            (fun k => ¬ max ≤ cqKeyCount (s.notAll + b) k)⟩ := by
      unfold cqTick
      rw [if_neg hmm]
    have hout : (cqTick min max s b).2
        = (((s.notAll + b).map Prod.fst).dedup.filter
            (fun k => min ≤ cqOkCount (s.notAll + b) k)).filter
            (fun k => s.minButNotMax.count k = 0) := by
      unfold cqTick
      rw [if_neg hmm]
    have hmbiff := hmb hltmm
    by_cases hd : cqDropped min max pfx k
    case pos =>
      have hkeyp : max ≤ cqKeyCount pfx k :=
        (cqDropped_of_ne hmm pfx k).mp hd
      have hkb := cq_dropped_kills_batch hcap hd
      have hkeyb : cqKeyCount b k = 0 :=
        cqKeyCount_eq_zero_of_kpart hkb
      have hokb : cqOkCount b k = 0 := cqOkCount_eq_zero_of_kpart hkb
      have hcur0 : (s.notAll + b).filter (fun r => r.1 = k) = 0 :=
        cq_current_kpart_dropped hna hcap hd
      have hok0 : cqOkCount (s.notAll + b) k = 0 :=
        cqOkCount_eq_zero_of_kpart hcur0
      have hokpb : cqOkCount (pfx + b) k = cqOkCount pfx k := by
        rw [cqOkCount_add]
        omega
      have hkeypb : max ≤ cqKeyCount (pfx + b) k := by
        rw [cqKeyCount_add]
        omega
      have hd' : cqDropped min max (pfx + b) k :=
        (cqDropped_of_ne hmm (pfx + b) k).mpr hkeypb
      refine ⟨⟨?_, ?_⟩, ?_⟩
      · rw [hst]
        show ((s.notAll + b).filter
            (fun r => ¬ max ≤ cqKeyCount (s.notAll + b) r.1)).filter
            (fun r => r.1 = k) = _
        rw [filter_key_of_pred _
            (fun k' => ¬ max ≤ cqKeyCount (s.notAll + b) k') k,
          if_pos hd']
        by_cases hpk : ¬ max ≤ cqKeyCount (s.notAll + b) k
        · rw [if_pos hpk, hcur0]
        · rw [if_neg hpk]
      · intro _
        rw [hst]
        show 0 < Multiset.count k
            ((((s.notAll + b).map Prod.fst).dedup.filter
              (fun k => min ≤ cqOkCount (s.notAll + b) k)).filter
              (fun k => ¬ max ≤ cqKeyCount (s.notAll + b) k)) ↔ _
        rw [count_dedup_keys_filter₂,
          if_neg (show ¬(k ∈ (s.notAll + b).map Prod.fst
              ∧ min ≤ cqOkCount (s.notAll + b) k
              ∧ ¬ max ≤ cqKeyCount (s.notAll + b) k) from
            fun hc => absurd hc.2.1 (by omega))]
        refine iff_of_false (by omega) ?_
        rintro ⟨-, hkc⟩
        omega
      · rw [hout, count_dedup_keys_filter₂,
          if_neg (show ¬(k ∈ (s.notAll + b).map Prod.fst
              ∧ min ≤ cqOkCount (s.notAll + b) k
              ∧ s.minButNotMax.count k = 0) from
            fun hc => absurd hc.2.1 (by omega)),
          if_neg (show ¬(min ≤ cqOkCount (pfx + b) k
              ∧ ¬ min ≤ cqOkCount pfx k) from fun hc => by
            rw [hokpb] at hc
            exact hc.2 hc.1)]
    case neg =>
      have hcur : (s.notAll + b).filter (fun r => r.1 = k)
          = (pfx + b).filter (fun r => r.1 = k) :=
        cq_current_kpart hna hd b
      have hokc : cqOkCount (s.notAll + b) k = cqOkCount (pfx + b) k :=
        cqOkCount_eq_of_kpart_eq hcur
      have hkeyc : cqKeyCount (s.notAll + b) k
          = cqKeyCount (pfx + b) k :=
        cqKeyCount_eq_of_kpart_eq hcur
      have hkeyplt : ¬ max ≤ cqKeyCount pfx k := by
        intro hc
        exact hd ((cqDropped_of_ne hmm pfx k).mpr hc)
      refine ⟨⟨?_, ?_⟩, ?_⟩
      · rw [hst]
        show ((s.notAll + b).filter
            (fun r => ¬ max ≤ cqKeyCount (s.notAll + b) r.1)).filter
            (fun r => r.1 = k) = _
        rw [filter_key_of_pred _
          (fun k' => ¬ max ≤ cqKeyCount (s.notAll + b) k') k]
        by_cases hkx : max ≤ cqKeyCount (pfx + b) k
        · rw [if_neg (show ¬¬ max ≤ cqKeyCount (s.notAll + b) k by
              rw [hkeyc]
              omega),
            if_pos ((cqDropped_of_ne hmm (pfx + b) k).mpr hkx)]
        · rw [if_pos (show ¬ max ≤ cqKeyCount (s.notAll + b) k by
              rw [hkeyc]
              omega),
            hcur,
            if_neg (show ¬ cqDropped min max (pfx + b) k from fun hc =>
              hkx ((cqDropped_of_ne hmm (pfx + b) k).mp hc))]
      · intro _
        rw [hst]
        show 0 < Multiset.count k
            ((((s.notAll + b).map Prod.fst).dedup.filter
              (fun k => min ≤ cqOkCount (s.notAll + b) k)).filter
              (fun k => ¬ max ≤ cqKeyCount (s.notAll + b) k)) ↔ _
        rw [count_dedup_keys_filter₂]
        by_cases hkx : max ≤ cqKeyCount (pfx + b) k
        · rw [if_neg (show ¬(k ∈ (s.notAll + b).map Prod.fst
                ∧ min ≤ cqOkCount (s.notAll + b) k
                ∧ ¬ max ≤ cqKeyCount (s.notAll + b) k) from
              fun hc => hc.2.2 (by rw [hkeyc]; exact hkx))]
          exact iff_of_false (by omega) (fun hc => by omega)
        · by_cases hcross : min ≤ cqOkCount (pfx + b) k
          · rw [if_pos (show k ∈ (s.notAll + b).map Prod.fst
                  ∧ min ≤ cqOkCount (s.notAll + b) k
                  ∧ ¬ max ≤ cqKeyCount (s.notAll + b) k from
                ⟨cq_mem_keys_of_ok (by omega),
                 by rw [hokc]; exact hcross,
                 by rw [hkeyc]; exact hkx⟩)]
            exact iff_of_true (by omega) ⟨hcross, by omega⟩
          · rw [if_neg (show ¬(k ∈ (s.notAll + b).map Prod.fst
                  ∧ min ≤ cqOkCount (s.notAll + b) k
                  ∧ ¬ max ≤ cqKeyCount (s.notAll + b) k) from
                fun hc => hcross (by
                  have h2 := hc.2.1
                  omega))]
            exact iff_of_false (by omega) (fun hc => hcross hc.1)
      · have hmbz : (Multiset.count k s.minButNotMax = 0)
            ↔ ¬ min ≤ cqOkCount pfx k := by
          constructor
          · intro hz hc
            have hpos := hmbiff.mpr ⟨hc, by omega⟩
            omega
          · intro hc
            by_contra hnz
            exact hc (hmbiff.mp (Nat.pos_of_ne_zero hnz)).1
        rw [hout, count_dedup_keys_filter₂]
        by_cases hprev : min ≤ cqOkCount pfx k
        · rw [if_neg (show ¬(k ∈ (s.notAll + b).map Prod.fst
                ∧ min ≤ cqOkCount (s.notAll + b) k
                ∧ Multiset.count k s.minButNotMax = 0) from
              fun hc => (hmbz.mp hc.2.2) hprev),
            if_neg (show ¬(min ≤ cqOkCount (pfx + b) k
                ∧ ¬ min ≤ cqOkCount pfx k) from
              fun hc => hc.2 hprev)]
        · by_cases hcross : min ≤ cqOkCount (pfx + b) k
          · rw [if_pos (show k ∈ (s.notAll + b).map Prod.fst
                  ∧ min ≤ cqOkCount (s.notAll + b) k
                  ∧ Multiset.count k s.minButNotMax = 0 from
                ⟨cq_mem_keys_of_ok (by omega),
                 by rw [hokc]; exact hcross,
                 hmbz.mpr hprev⟩),
              if_pos (show min ≤ cqOkCount (pfx + b) k
                  ∧ ¬ min ≤ cqOkCount pfx k from ⟨hcross, hprev⟩)]
          · rw [if_neg (show ¬(k ∈ (s.notAll + b).map Prod.fst
                  ∧ min ≤ cqOkCount (s.notAll + b) k
                  ∧ Multiset.count k s.minButNotMax = 0) from
                fun hc => hcross (by
                  have h2 := hc.2.1
                  omega)),
              if_neg (show ¬(min ≤ cqOkCount (pfx + b) k
                  ∧ ¬ min ≤ cqOkCount pfx k) from
                fun hc => hcross hc.1)]

/-- One consumed batch, per key: the registers stay characterized and
the key's emission is its accumulated `Ok` responses exactly at the
crossing. -/
theorem cqwrTick_key_step [DecidableEq E] [DecidableEq V]
    (min max : Nat) (h1 : 1 ≤ min) (hminmax : min ≤ max)
    (s : CQState K E V) (pfx b : Multiset (K × Except E V))
    (k : K) (hcap : cqKeyCount (pfx + b) k ≤ max)
    (hg : CQKeyGood min max s pfx k) :
    CQKeyGood min max (cqwrTick min max s b).1 (pfx + b) k
    ∧ ((cqwrTick min max s b).2.filter (fun r => r.1 = k)
        = if min ≤ cqOkCount (pfx + b) k ∧ ¬ min ≤ cqOkCount pfx k
          then ((pfx + b).filter (fun r => r.1 = k)).filterMap cqOkProj
          else 0) := by
  obtain ⟨hna, hmb⟩ := hg
  have hokmono : cqOkCount pfx k ≤ cqOkCount (pfx + b) k := by
    rw [cqOkCount_add]
    omega
  by_cases hmm : min = max
  case pos =>
    have hst : (cqwrTick min max s b).1
        = ⟨(s.notAll + b).filter
            (fun r => ¬ min ≤ cqOkCount (s.notAll + b) r.1),
          s.minButNotMax⟩ := by
      unfold cqwrTick
      rw [if_pos hmm]
    have hout : (cqwrTick min max s b).2
        = ((s.notAll + b).filter
            (fun r => min ≤ cqOkCount (s.notAll + b) r.1)).filterMap
          cqOkProj := by
      unfold cqwrTick
      rw [if_pos hmm]
    by_cases hd : cqDropped min max pfx k
    case pos =>
      have hokp : min ≤ cqOkCount pfx k :=
        (cqDropped_of_eq hmm pfx k).mp hd
      have hcur0 : (s.notAll + b).filter (fun r => r.1 = k) = 0 :=
        cq_current_kpart_dropped hna hcap hd
      have hok0 : cqOkCount (s.notAll + b) k = 0 :=
        cqOkCount_eq_zero_of_kpart hcur0
      have hd' : cqDropped min max (pfx + b) k := by
        rw [cqDropped_of_eq hmm]
        omega
      refine ⟨⟨?_, fun hlt => absurd hmm (Nat.ne_of_lt hlt)⟩, ?_⟩
      · rw [hst]
        show ((s.notAll + b).filter
            (fun r => ¬ min ≤ cqOkCount (s.notAll + b) r.1)).filter
            (fun r => r.1 = k) = _
        rw [filter_key_of_pred _
            (fun k' => ¬ min ≤ cqOkCount (s.notAll + b) k') k,
          if_pos hd']
        by_cases hpk : ¬ min ≤ cqOkCount (s.notAll + b) k
        · rw [if_pos hpk, hcur0]
        · rw [if_neg hpk]
      · rw [hout, filterMap_okProj_kpart_comm,
          filter_key_of_pred _
            (fun k' => min ≤ cqOkCount (s.notAll + b) k') k,
          if_neg (show ¬ min ≤ cqOkCount (s.notAll + b) k by omega),
          if_neg (show ¬(min ≤ cqOkCount (pfx + b) k
              ∧ ¬ min ≤ cqOkCount pfx k) from fun hc => hc.2 hokp)]
        rfl
    case neg =>
      have hcur : (s.notAll + b).filter (fun r => r.1 = k)
          = (pfx + b).filter (fun r => r.1 = k) :=
        cq_current_kpart hna hd b
      have hokc : cqOkCount (s.notAll + b) k = cqOkCount (pfx + b) k :=
        cqOkCount_eq_of_kpart_eq hcur
      have hnokp : ¬ min ≤ cqOkCount pfx k := by
        intro hc
        exact hd ((cqDropped_of_eq hmm pfx k).mpr hc)
      refine ⟨⟨?_, fun hlt => absurd hmm (Nat.ne_of_lt hlt)⟩, ?_⟩
      · rw [hst]
        show ((s.notAll + b).filter
            (fun r => ¬ min ≤ cqOkCount (s.notAll + b) r.1)).filter
            (fun r => r.1 = k) = _
        rw [filter_key_of_pred _
          (fun k' => ¬ min ≤ cqOkCount (s.notAll + b) k') k]
        by_cases hcross : min ≤ cqOkCount (pfx + b) k
        · rw [if_neg (show ¬¬ min ≤ cqOkCount (s.notAll + b) k by
              rw [hokc]
              omega),
            if_pos ((cqDropped_of_eq hmm (pfx + b) k).mpr hcross)]
        · rw [if_pos (show ¬ min ≤ cqOkCount (s.notAll + b) k by
              rw [hokc]
              omega),
            hcur,
            if_neg (show ¬ cqDropped min max (pfx + b) k from fun hc =>
              hcross ((cqDropped_of_eq hmm (pfx + b) k).mp hc))]
      · rw [hout, filterMap_okProj_kpart_comm,
          filter_key_of_pred _
            (fun k' => min ≤ cqOkCount (s.notAll + b) k') k]
        by_cases hcross : min ≤ cqOkCount (pfx + b) k
        · rw [if_pos (show min ≤ cqOkCount (s.notAll + b) k by
              rw [hokc]
              exact hcross),
            hcur,
            if_pos (show min ≤ cqOkCount (pfx + b) k
                ∧ ¬ min ≤ cqOkCount pfx k from ⟨hcross, hnokp⟩)]
        · rw [if_neg (show ¬ min ≤ cqOkCount (s.notAll + b) k by
              rw [hokc]
              exact hcross),
            if_neg (show ¬(min ≤ cqOkCount (pfx + b) k
                ∧ ¬ min ≤ cqOkCount pfx k) from
              fun hc => hcross hc.1)]
          rfl
  case neg =>
    have hltmm : min < max := Nat.lt_of_le_of_ne hminmax hmm
    have hst : (cqwrTick min max s b).1
        = ⟨(s.notAll + b).filter
            (fun r => ¬ max ≤ cqKeyCount (s.notAll + b) r.1),
          ((s.notAll + b).map Prod.fst).dedup.filter
            (fun k => min ≤ cqOkCount (s.notAll + b) k
              ∧ ¬ max ≤ cqKeyCount (s.notAll + b) k)⟩ := by
      unfold cqwrTick
      rw [if_neg hmm]
    have hout : (cqwrTick min max s b).2
        = ((s.notAll + b).filter
            (fun r => min ≤ cqOkCount (s.notAll + b) r.1
              ∧ s.minButNotMax.count r.1 = 0)).filterMap cqOkProj := by
      unfold cqwrTick
      rw [if_neg hmm]
    have hmbiff := hmb hltmm
    by_cases hd : cqDropped min max pfx k
    case pos =>
      have hkeyp : max ≤ cqKeyCount pfx k :=
        (cqDropped_of_ne hmm pfx k).mp hd
      have hkb := cq_dropped_kills_batch hcap hd
      have hkeyb : cqKeyCount b k = 0 :=
        cqKeyCount_eq_zero_of_kpart hkb
      have hokb : cqOkCount b k = 0 := cqOkCount_eq_zero_of_kpart hkb
      have hcur0 : (s.notAll + b).filter (fun r => r.1 = k) = 0 :=
        cq_current_kpart_dropped hna hcap hd
      have hok0 : cqOkCount (s.notAll + b) k = 0 :=
        cqOkCount_eq_zero_of_kpart hcur0
      have hokpb : cqOkCount (pfx + b) k = cqOkCount pfx k := by
        rw [cqOkCount_add]
        omega
      have hkeypb : max ≤ cqKeyCount (pfx + b) k := by
        rw [cqKeyCount_add]
        omega
      have hd' : cqDropped min max (pfx + b) k :=
        (cqDropped_of_ne hmm (pfx + b) k).mpr hkeypb
      refine ⟨⟨?_, ?_⟩, ?_⟩
      · rw [hst]
        show ((s.notAll + b).filter
            (fun r => ¬ max ≤ cqKeyCount (s.notAll + b) r.1)).filter
            (fun r => r.1 = k) = _
        rw [filter_key_of_pred _
            (fun k' => ¬ max ≤ cqKeyCount (s.notAll + b) k') k,
          if_pos hd']
        by_cases hpk : ¬ max ≤ cqKeyCount (s.notAll + b) k
        · rw [if_pos hpk, hcur0]
        · rw [if_neg hpk]
      · intro _
        rw [hst]
        show 0 < Multiset.count k
            (((s.notAll + b).map Prod.fst).dedup.filter
              (fun k => min ≤ cqOkCount (s.notAll + b) k
                ∧ ¬ max ≤ cqKeyCount (s.notAll + b) k)) ↔ _
        rw [count_dedup_keys_filter,
          if_neg (show ¬(k ∈ (s.notAll + b).map Prod.fst
              ∧ min ≤ cqOkCount (s.notAll + b) k
              ∧ ¬ max ≤ cqKeyCount (s.notAll + b) k) from
            fun hc => absurd hc.2.1 (by omega))]
        refine iff_of_false (by omega) ?_
        rintro ⟨-, hkc⟩
        omega
      · rw [hout, filterMap_okProj_kpart_comm,
          filter_key_of_pred _
            (fun k' => min ≤ cqOkCount (s.notAll + b) k'
              ∧ s.minButNotMax.count k' = 0) k,
          if_neg (show ¬(min ≤ cqOkCount (s.notAll + b) k
              ∧ Multiset.count k s.minButNotMax = 0) from
            fun hc => absurd hc.1 (by omega)),
          if_neg (show ¬(min ≤ cqOkCount (pfx + b) k
              ∧ ¬ min ≤ cqOkCount pfx k) from fun hc => by
            rw [hokpb] at hc
            exact hc.2 hc.1)]
        rfl
    case neg =>
      have hcur : (s.notAll + b).filter (fun r => r.1 = k)
          = (pfx + b).filter (fun r => r.1 = k) :=
        cq_current_kpart hna hd b
      have hokc : cqOkCount (s.notAll + b) k = cqOkCount (pfx + b) k :=
        cqOkCount_eq_of_kpart_eq hcur
      have hkeyc : cqKeyCount (s.notAll + b) k
          = cqKeyCount (pfx + b) k :=
        cqKeyCount_eq_of_kpart_eq hcur
      have hkeyplt : ¬ max ≤ cqKeyCount pfx k := by
        intro hc
        exact hd ((cqDropped_of_ne hmm pfx k).mpr hc)
      have hmbz : (Multiset.count k s.minButNotMax = 0)
          ↔ ¬ min ≤ cqOkCount pfx k := by
        constructor
        · intro hz hc
          have hpos := hmbiff.mpr ⟨hc, by omega⟩
          omega
        · intro hc
          by_contra hnz
          exact hc (hmbiff.mp (Nat.pos_of_ne_zero hnz)).1
      refine ⟨⟨?_, ?_⟩, ?_⟩
      · rw [hst]
        show ((s.notAll + b).filter
            (fun r => ¬ max ≤ cqKeyCount (s.notAll + b) r.1)).filter
            (fun r => r.1 = k) = _
        rw [filter_key_of_pred _
          (fun k' => ¬ max ≤ cqKeyCount (s.notAll + b) k') k]
        by_cases hkx : max ≤ cqKeyCount (pfx + b) k
        · rw [if_neg (show ¬¬ max ≤ cqKeyCount (s.notAll + b) k by
              rw [hkeyc]
              omega),
            if_pos ((cqDropped_of_ne hmm (pfx + b) k).mpr hkx)]
        · rw [if_pos (show ¬ max ≤ cqKeyCount (s.notAll + b) k by
              rw [hkeyc]
              omega),
            hcur,
            if_neg (show ¬ cqDropped min max (pfx + b) k from fun hc =>
              hkx ((cqDropped_of_ne hmm (pfx + b) k).mp hc))]
      · intro _
        rw [hst]
        show 0 < Multiset.count k
            (((s.notAll + b).map Prod.fst).dedup.filter
              (fun k => min ≤ cqOkCount (s.notAll + b) k
                ∧ ¬ max ≤ cqKeyCount (s.notAll + b) k)) ↔ _
        rw [count_dedup_keys_filter]
        by_cases hkx : max ≤ cqKeyCount (pfx + b) k
        · rw [if_neg (show ¬(k ∈ (s.notAll + b).map Prod.fst
                ∧ min ≤ cqOkCount (s.notAll + b) k
                ∧ ¬ max ≤ cqKeyCount (s.notAll + b) k) from
              fun hc => hc.2.2 (by rw [hkeyc]; exact hkx))]
          exact iff_of_false (by omega) (fun hc => by omega)
        · by_cases hcross : min ≤ cqOkCount (pfx + b) k
          · rw [if_pos (show k ∈ (s.notAll + b).map Prod.fst
                  ∧ min ≤ cqOkCount (s.notAll + b) k
                  ∧ ¬ max ≤ cqKeyCount (s.notAll + b) k from
                ⟨cq_mem_keys_of_ok (by omega),
                 by rw [hokc]; exact hcross,
                 by rw [hkeyc]; exact hkx⟩)]
            exact iff_of_true (by omega) ⟨hcross, by omega⟩
          · rw [if_neg (show ¬(k ∈ (s.notAll + b).map Prod.fst
                  ∧ min ≤ cqOkCount (s.notAll + b) k
                  ∧ ¬ max ≤ cqKeyCount (s.notAll + b) k) from
                fun hc => hcross (by
                  have h2 := hc.2.1
                  omega))]
            exact iff_of_false (by omega) (fun hc => hcross hc.1)
      · rw [hout, filterMap_okProj_kpart_comm,
          filter_key_of_pred _
            (fun k' => min ≤ cqOkCount (s.notAll + b) k'
              ∧ s.minButNotMax.count k' = 0) k]
        by_cases hprev : min ≤ cqOkCount pfx k
        · rw [if_neg (show ¬(min ≤ cqOkCount (s.notAll + b) k
                ∧ Multiset.count k s.minButNotMax = 0) from
              fun hc => (hmbz.mp hc.2) hprev),
            if_neg (show ¬(min ≤ cqOkCount (pfx + b) k
                ∧ ¬ min ≤ cqOkCount pfx k) from
              fun hc => hc.2 hprev)]
          rfl
        · by_cases hcross : min ≤ cqOkCount (pfx + b) k
          · rw [if_pos (show min ≤ cqOkCount (s.notAll + b) k
                  ∧ Multiset.count k s.minButNotMax = 0 from
                ⟨by rw [hokc]; exact hcross, hmbz.mpr hprev⟩),
              hcur,
              if_pos (show min ≤ cqOkCount (pfx + b) k
                  ∧ ¬ min ≤ cqOkCount pfx k from ⟨hcross, hprev⟩)]
          · rw [if_neg (show ¬(min ≤ cqOkCount (s.notAll + b) k
                  ∧ Multiset.count k s.minButNotMax = 0) from
                fun hc => hcross (by
                  have h2 := hc.1
                  omega)),
              if_neg (show ¬(min ≤ cqOkCount (pfx + b) k
                  ∧ ¬ min ≤ cqOkCount pfx k) from
                fun hc => hcross hc.1)]
            rfl

/-! ## Run facts (the generic scan combinators, instantiated)

Each is one application of a `Trace.lean` combinator (`scan_sound`,
`scan_emit_ind`, `scan_bound`) at this module's step obligations — no
run-level induction lives here. -/

/-- `collect_quorum`'s run soundness: an emitted key holds `min` `Ok`
votes among the consumed responses. -/
theorem cq_run_sound (min max : Nat) (k : K)
    (bs : List (Multiset (K × Except E Unit)))
    (s : CQState K E Unit) (pfx : Multiset (K × Except E Unit))
    (hw : s.notAll ≤ pfx)
    (hk : k ∈ (scanAcrossTicksTrace (cqTick min max) s bs).sum) :
    min ≤ cqOkCount (pfx + bs.sum) k := by
  have h := scan_sound (cqTick min max) id CQState.notAll
    (fun p k' => min ≤ cqOkCount p k')
    (fun {p p' y} hle hq => le_trans hq (cqOkCount_mono hle y))
    (cqTick_notAll_le min max)
    (fun s' b {k'} hk' => cqTick_emit_ok min max s' b hk')
    bs s pfx hw hk
  rwa [List.map_id] at h

/-- `collect_quorum_with_response`'s run soundness: an emitted response
quotes a consumed `Ok` response of a key holding `min` votes among the
consumed pool. -/
theorem cqwr_run_sound (min max : Nat) (k : K) (v : V)
    (bs : List (Multiset (K × Except E V)))
    (s : CQState K E V) (pfx : Multiset (K × Except E V))
    (hw : s.notAll ≤ pfx)
    (hk : (k, v) ∈ (scanAcrossTicksTrace (cqwrTick min max) s bs).sum) :
    (k, .ok v) ∈ pfx + bs.sum ∧ min ≤ cqOkCount (pfx + bs.sum) k := by
  have h := scan_sound (cqwrTick min max) id CQState.notAll
    (fun p (r : K × V) => (r.1, Except.ok r.2) ∈ p
      ∧ min ≤ cqOkCount p r.1)
    (fun {p p' y} hle hq =>
      ⟨Multiset.mem_of_le hle hq.1,
       le_trans hq.2 (cqOkCount_mono hle y.1)⟩)
    (cqwrTick_notAll_le min max)
    (fun s' b {r} hr => cqwrTick_emit_ok min max s' b hr)
    bs s pfx hw hk
  rwa [List.map_id] at h

/-- **The crossing characterization at the run** (quorum.rs's usage
contract as the only hypothesis): the run emits a key **iff** it
reached `min` `Ok` votes among the consumed pool — exactly once. -/
theorem cq_run_count (min max : Nat) (h1 : 1 ≤ min)
    (hminmax : min ≤ max) (k : K)
    (bs : List (Multiset (K × Except E Unit)))
    (hcap : cqKeyCount bs.sum k ≤ max) :
    ((scanAcrossTicksTrace (cqTick min max)
        (CQState.init (K := K) (E := E) (V := Unit)) bs).sum.count k)
      = if min ≤ cqOkCount bs.sum k then 1 else 0 := by
  have h := scan_emit_ind (cqTick min max) id bs.sum
    (fun s pfx => CQKeyGood min max s pfx k)
    (fun pfx acc => acc.count k
      = if min ≤ cqOkCount bs.sum k ∧ ¬ min ≤ cqOkCount pfx k
        then 1 else 0)
    (fun s pfx b hle hg =>
      (cqTick_key_step min max h1 hminmax s pfx b k
        (le_trans (cqKeyCount_mono hle k) hcap) hg).1)
    (by rw [Multiset.count_zero, if_neg (fun hc => hc.2 hc.1)])
    (fun s pfx b acc hle hg hacc => by
      simp only [id_eq] at hle hacc
      obtain ⟨-, hcnt⟩ := cqTick_key_step min max h1 hminmax s pfx b k
        (le_trans (cqKeyCount_mono hle k) hcap) hg
      rw [Multiset.count_add, hcnt, hacc]
      have h1m : cqOkCount pfx k ≤ cqOkCount (pfx + b) k := by
        rw [cqOkCount_add]; omega
      have h2m : cqOkCount (pfx + b) k ≤ cqOkCount bs.sum k :=
        cqOkCount_mono hle k
      split_ifs <;> omega)
    bs CQState.init 0 (by rw [List.map_id, Multiset.zero_add])
    (CQKeyGood_init min max h1 k)
  have hz : ¬ min ≤ cqOkCount (0 : Multiset (K × Except E Unit)) k := by
    unfold cqOkCount
    rw [Multiset.filter_zero, Multiset.card_zero]
    omega
  rw [h]
  by_cases hc : min ≤ cqOkCount bs.sum k
  · rw [if_pos ⟨hc, hz⟩, if_pos hc]
  · rw [if_neg (fun hx => hc hx.1), if_neg hc]

/-- The per-key run characterization of `collect_quorum_with_response`
(under the usage contract): a crossed key never re-emits; emissions
embed in the key's consumed `Ok` responses; a key crossing during the
run emits at least `min` responses. -/
theorem cqwr_run_key (min max : Nat) (h1 : 1 ≤ min)
    (hminmax : min ≤ max) (k : K)
    (bs : List (Multiset (K × Except E V)))
    (s : CQState K E V) (pfx : Multiset (K × Except E V))
    (hcap : cqKeyCount (pfx + bs.sum) k ≤ max)
    (hg : CQKeyGood min max s pfx k) :
    ((min ≤ cqOkCount pfx k →
        (scanAcrossTicksTrace (cqwrTick min max) s bs).sum.filter
          (fun r => r.1 = k) = 0)
      ∧ (scanAcrossTicksTrace (cqwrTick min max) s bs).sum.filter
          (fun r => r.1 = k)
        ≤ ((pfx + bs.sum).filter (fun r => r.1 = k)).filterMap
            cqOkProj
      ∧ (¬ min ≤ cqOkCount pfx k →
          min ≤ cqOkCount (pfx + bs.sum) k →
          min ≤ ((scanAcrossTicksTrace (cqwrTick min max) s
            bs).sum.filter (fun r => r.1 = k)).card)) := by
  have htot : pfx + (bs.map id).sum = pfx + bs.sum := by
    rw [List.map_id]
  refine scan_emit_ind (cqwrTick min max) id (pfx + bs.sum)
    (fun s' pfx' => CQKeyGood min max s' pfx' k)
    (fun pfx' acc =>
      (min ≤ cqOkCount pfx' k →
        acc.filter (fun r => r.1 = k) = 0)
      ∧ acc.filter (fun r => r.1 = k)
          ≤ ((pfx + bs.sum).filter (fun r => r.1 = k)).filterMap
              cqOkProj
      ∧ (¬ min ≤ cqOkCount pfx' k →
          min ≤ cqOkCount (pfx + bs.sum) k →
          min ≤ (acc.filter (fun r => r.1 = k)).card))
    (fun s' pfx' b hle hg' =>
      (cqwrTick_key_step min max h1 hminmax s' pfx' b k
        (le_trans (cqKeyCount_mono hle k) hcap) hg').1)
    ⟨fun _ => rfl,
     (by
        show (0 : Multiset (K × V)) ≤ _
        exact Multiset.zero_le _),
     fun hn hy => absurd hy hn⟩
    (fun s' pfx' b acc hle hg' hacc => by
      simp only [id_eq] at hle hacc
      obtain ⟨-, hem⟩ := cqwrTick_key_step min max h1 hminmax s' pfx' b
        k (le_trans (cqKeyCount_mono hle k) hcap) hg'
      obtain ⟨ih1, ih2, ih3⟩ := hacc
      have hokmono : cqOkCount pfx' k ≤ cqOkCount (pfx' + b) k := by
        rw [cqOkCount_add]; omega
      have hkmono : ((pfx' + b).filter (fun r => r.1 = k)).filterMap
          cqOkProj
          ≤ ((pfx + bs.sum).filter (fun r => r.1 = k)).filterMap
              cqOkProj :=
        Multiset.filterMap_le_filterMap _
          (Multiset.filter_le_filter _ hle)
      rw [Multiset.filter_add]
      refine ⟨?_, ?_, ?_⟩
      · intro hprev
        rw [hem,
          if_neg (show ¬(min ≤ cqOkCount (pfx' + b) k
              ∧ ¬ min ≤ cqOkCount pfx' k) from fun hc => hc.2 hprev),
          ih1 (by omega), Multiset.zero_add]
      · rw [hem]
        by_cases hcross : min ≤ cqOkCount (pfx' + b) k
            ∧ ¬ min ≤ cqOkCount pfx' k
        · rw [if_pos hcross, ih1 (by
              have := hcross.1
              omega), Multiset.add_zero]
          exact hkmono
        · rw [if_neg hcross, Multiset.zero_add]
          exact ih2
      · intro hprev htotc
        rw [hem]
        by_cases hcb : min ≤ cqOkCount (pfx' + b) k
        · rw [if_pos ⟨hcb, hprev⟩, ih1 (by omega), Multiset.add_zero,
            card_okProj_kpart]
          exact hcb
        · rw [if_neg (fun hc => hcb hc.1), Multiset.zero_add]
          exact ih3 hcb htotc)
    bs s pfx htot hg

/-- **Unconditional soundness of the success stream** (multiset form,
from the null registers): a run's emissions embed in the consumed
pool's `Ok` projection — with multiplicity, and no usage caps. -/
theorem cqwr_run_le_init (min max : Nat)
    (bs : List (Multiset (K × Except E V))) :
    (scanAcrossTicksTrace (cqwrTick min max) CQState.init bs).sum
      ≤ (bs.sum).filterMap cqOkProj := by
  rw [Multiset.le_iff_count]
  rintro ⟨k, v⟩
  have h := scan_bound (cqwrTick min max)
    (fun x => x.filter (fun r => r.1 = k))
    (Multiset.filter_zero _)
    (fun x y => Multiset.filter_add _ x y)
    (fun b => (b.filter (fun r => r.1 = k)).filterMap cqOkProj)
    (fun s => s.minButNotMax.count k = 0
      ∨ (min ≠ max ∧ min ≤ cqOkCount s.notAll k))
    (fun s => if s.minButNotMax.count k = 0
      then (s.notAll.filter (fun r => r.1 = k)).filterMap cqOkProj
      else 0)
    (fun s b hinv => cqwrTick_bound_step min max k s b hinv)
    bs CQState.init (Or.inl (Multiset.count_zero k))
  rw [show (CQState.init : CQState K E V).minButNotMax
      = (0 : Multiset K) from rfl,
    show (CQState.init : CQState K E V).notAll
      = (0 : Multiset (K × Except E V)) from rfl,
    if_pos (Multiset.count_zero k), Multiset.filter_zero,
    Multiset.filterMap_zero, Multiset.zero_add,
    map_sum_additive
      (fun x => ((x.filter (fun r => r.1 = k)).filterMap cqOkProj))
      (by rw [Multiset.filter_zero, Multiset.filterMap_zero])
      (fun x y => by rw [Multiset.filter_add, Multiset.filterMap_add])
      bs,
    ← filterMap_okProj_kpart_comm] at h
  have hc := Multiset.count_le_of_le ((k, v) : K × V) h
  rwa [Multiset.count_filter, if_pos rfl, Multiset.count_filter,
    if_pos rfl] at hc

set_option maxHeartbeats 1000000 in
/-- **quorum.rs:90–160 `collect_quorum`** over location `ℓ`: emit each
key once as it reaches `min` successful responses (persisting
below-quorum responses across arbitrary batching), and surface every
error. Returns (`just_reached_quorum`, `fails`). -/
hydro def collect_quorum (H : HydroSem L mem) (ℓ : L)
    (responses : H.Stream ℓ (K × Except E Unit) .noOrder .exactlyOnce)
    (min max : Nat)
    (dec : H.BatchDec (mem ℓ) (K × Except E Unit))
    (demit : H.EmitDec (mem ℓ) K) :
    (H.Stream ℓ K .noOrder .exactlyOnce
      × H.Stream ℓ (K × E) .noOrder .exactlyOnce)
  ensures out => CQEnsures ℓ min max responses dec out :=
  -- let new_inputs = use::batch(responses.clone(), nondet!(…));
  let new_inputs := H.batch responses dec
  -- the realized per-member cut list (spec-only)
  ghost let cuts := fun i => batchCuts (responses i) 0 (dec i)
  -- the `sliced!` register loop (not_all + min_but_not_max),
  -- quorum.rs:92–152 Rust-literal — the step is `cqTick` above
  let just_reached_quorum := H.scan_batches_unordered new_inputs
    (fun _me => cqTick min max) CQState.init
  -- what the register loop guarantees, at the loop: a key it emits
  -- holds `min` `Ok` votes among the consumed responses
  ghost have scan_sound : ∀ (i : Fin (mem ℓ)) (k : K),
      k ∈ (scanAcrossTicksTrace (cqTick min max) CQState.init
        (cuts i)).sum →
      min ≤ cqOkCount (cqConsumed (responses i) (dec i)) k := fun i k hk => by
    have h := cq_run_sound min max k (cuts i) CQState.init 0
      (Multiset.zero_le _) hk
    rwa [Multiset.zero_add] at h
  (H.allTicks (H.emitMultisetBatches just_reached_quorum demit),
    -- responses.filter_map(Err(e) → Some((key, e)))
    H.filterMap responses (fun _me => cqErrProj))
  prove
    emit_sound := scan_sound,
    emit_count := fun i k h1 hminmax hcap =>
      cq_run_count min max h1 hminmax k (cuts i) hcap,
    fails_eq := fun i => rfl

set_option maxHeartbeats 1000000 in
/-- **quorum.rs:7–88 `collect_quorum_with_response`** over location
`ℓ`: as `collect_quorum`, but each just-reached key emits its
accumulated `Ok` **responses**. Returns (`quorums`, `fails`). -/
hydro def collect_quorum_with_response (H : HydroSem L mem) (ℓ : L)
    (responses : H.Stream ℓ (K × Except E V) .noOrder .exactlyOnce)
    (min max : Nat)
    (dec : H.BatchDec (mem ℓ) (K × Except E V))
    (demit : H.EmitDec (mem ℓ) (K × V)) :
    (H.Stream ℓ (K × V) .noOrder .exactlyOnce
      × H.Stream ℓ (K × E) .noOrder .exactlyOnce)
  ensures out => CQWREnsures ℓ min max responses dec out :=
  -- let new_inputs = use::batch(responses.clone(), nondet!(…));
  let new_inputs := H.batch responses dec
  -- the realized per-member cut list (spec-only)
  ghost let cuts := fun i => batchCuts (responses i) 0 (dec i)
  -- the `sliced!` register loop, quorum.rs:12–76 Rust-literal — the
  -- step is `cqwrTick` above
  let quorums := H.scan_batches_unordered new_inputs
    (fun _me => cqwrTick min max) CQState.init
  -- the register-loop key characterization, at the loop: emissions of
  -- a capped key embed in its consumed `Ok` responses (with
  -- multiplicity), and a key reaching `min` emits at least `min`
  ghost have run_key : ∀ (i : Fin (mem ℓ)) (k : K), 1 ≤ min →
      min ≤ max → cqKeyCount (cqConsumed (responses i) (dec i)) k ≤ max →
      ((scanAcrossTicksTrace (cqwrTick min max) CQState.init
          (cuts i)).sum.filter (fun r => r.1 = k)
        ≤ ((cqConsumed (responses i) (dec i)).filter
            (fun r => r.1 = k)).filterMap cqOkProj)
      ∧ (min ≤ cqOkCount (cqConsumed (responses i) (dec i)) k →
          min ≤ ((scanAcrossTicksTrace (cqwrTick min max) CQState.init
            (cuts i)).sum.filter (fun r => r.1 = k)).card) :=
    fun i k h1 hminmax hcap => by
      have h := cqwr_run_key min max h1 hminmax k (cuts i) CQState.init 0
        (by rw [Multiset.zero_add]; exact hcap) (CQKeyGood_init min max h1 k)
      rw [Multiset.zero_add] at h
      have hz : ¬ min ≤ cqOkCount (0 : Multiset (K × Except E V)) k := by
        unfold cqOkCount
        rw [Multiset.filter_zero, Multiset.card_zero]
        omega
      exact ⟨h.2.1, fun hok => h.2.2 hz hok⟩
  (H.allTicks (H.emitMultisetBatches quorums demit),
   H.filterMap responses (fun _me => cqErrProj))
  prove
    emit_mem_sound := fun i k v hk => by
      have h := cqwr_run_sound min max k v (cuts i) CQState.init 0
        (Multiset.zero_le _) hk
      rwa [Multiset.zero_add] at h,
    emit_le := fun i k h1 hminmax hcap =>
      (run_key i k h1 hminmax hcap).1,
    emit_complete := fun i k h1 hminmax hcap hok =>
      (run_key i k h1 hminmax hcap).2 hok,
    emit_pool_le := fun i => cqwr_run_le_init min max (cuts i),
    fails_eq := fun i => rfl

/-! ## Executable smoke tests (mirroring quorum.rs's unit tests) -/

private abbrev oneLoc : Unit → Nat := fun _ => 1

-- `collect_quorum_functionality` (quorum.rs), min 2 of max 3:
-- key 1 reaches quorum with 2 Oks …
#guard (collect_quorum (Values Unit oneLoc) () (fun _ =>
    ({(1, .ok ()), (1, .ok ())} : Multiset (Nat × Except Nat Unit)))
    2 3 (fun _ => [{(1, .ok ()), (1, .ok ())}]) ()).val.1 0
  = {1}
-- … key 3 (1 Ok, 2 Errs) does not, and its errors surface
#guard (collect_quorum (Values Unit oneLoc) () (fun _ =>
    ({(3, .ok ()), (3, .error 7), (3, .error 8)}
      : Multiset (Nat × Except Nat Unit)))
    2 3 (fun _ => [{(3, .ok ()), (3, .error 7)}, {(3, .error 8)}]) ()).val.1 0
  = 0
#guard (collect_quorum (Values Unit oneLoc) () (fun _ =>
    ({(3, .ok ()), (3, .error 7), (3, .error 8)}
      : Multiset (Nat × Except Nat Unit)))
    2 3 (fun _ => [{(3, .ok ()), (3, .error 7)}, {(3, .error 8)}]) ()).val.2 0
  = ({(3, 7), (3, 8)} : Multiset (Nat × Nat))
-- `collect_quorum_no_double_quorum_before_max` (min 2, max 4): extra
-- Oks after the crossing never re-emit the key
#guard (collect_quorum (Values Unit oneLoc) () (fun _ =>
    ({(1, .ok ()), (1, .ok ()), (1, .ok ()), (1, .ok ())}
      : Multiset (Nat × Except Nat Unit)))
    2 4 (fun _ => [{(1, .ok ()), (1, .ok ())},
                   {(1, .ok ())}, {(1, .ok ())}]) ()).val.1 0
  = {1}
-- `collect_quorum_min_equals_max` (min = max = 2): 1 Ok + 1 Err fails,
-- 2 Oks succeed
#guard (collect_quorum (Values Unit oneLoc) () (fun _ =>
    ({(2, .ok ()), (2, .error 9), (3, .ok ()), (3, .ok ())}
      : Multiset (Nat × Except Nat Unit)))
    2 2 (fun _ => [{(2, .ok ()), (2, .error 9), (3, .ok ()),
                    (3, .ok ())}]) ()).val.1 0
  = {3}
-- `collect_quorum_with_response_no_order` (min = max = 2): the
-- just-reached keys emit their accumulated responses
#guard (collect_quorum_with_response (Values Unit oneLoc) () (fun _ =>
    ({(1, .ok 10), (1, .ok 11), (2, .ok 20), (3, .ok 30), (3, .ok 31)}
      : Multiset (Nat × Except Nat Nat)))
    2 2 (fun _ => [{(1, .ok 10), (1, .ok 11), (2, .ok 20)},
                   {(3, .ok 30), (3, .ok 31)}]) ()).val.1 0
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
    (demit : Fin 1 → List (List K)) (mn mx : Nat) (T : Nat)
    (i : Fin 1) (k : K)
    (hk : k ∈ ((collect_quorum (SchedSem Unit (fun _ => 1) pacing) ()
      h mn mx () demit).val.1 i).view T) :
    ∃ d : (Values Unit (fun _ => 1)).BatchDec 1 (K × Except E Unit),
      mn ≤ cqOkCount (cqConsumed (v i) (d i)) k := by
  refine ⟨collect_quorum_vdec (Td := T) (pacing := pacing) () mn mx h
    demit, ?_⟩
  have hcpl := (collect_quorum
      (CoupleSem Unit (fun _ => 1) pacing T T (Nat.le_refl T)) ()
      (CoStream.inputC (ord := .noOrder) (ret := .exactlyOnce) h v hc)
      mn mx (CoDec.batch _) (CoDec.emit demit)).val.1.cpl
    (collect_quorum_co_wf₁ (L := Unit) (mem := fun _ => 1)
      (pacing := pacing) (Tc := T) (Td := T) (hjT := Nat.le_refl T)
      () (CoStream.inputC h v hc) mn mx
      (CoDec.batch _) (CoDec.emit demit) trivial) i
  have hsr := collect_quorum_co_sr₁ (Tc := T) (Td := T)
    (hjT := Nat.le_refl T) (pacing := pacing) ()
    (CoStream.inputC (ord := .noOrder) (ret := .exactlyOnce) h v hc)
    mn mx (CoDec.batch _) (CoDec.emit demit)
  have hrr := collect_quorum_co_rr₁ (pacing := pacing) (Tc := T)
    (Td := T) (hjT := Nat.le_refl T) ()
    (CoStream.inputC (ord := .noOrder) (ret := .exactlyOnce) h v hc)
    mn mx (CoDec.batch _) (CoDec.emit demit)
  rw [hsr, hrr] at hcpl
  have hens := (collect_quorum (Values Unit (fun _ => 1)) ()
    v mn mx (collect_quorum_vdec (Td := T) (pacing := pacing) () mn mx
      h demit) ()).property rfl
  exact hens.emit_sound i k (Multiset.mem_of_le hcpl
    (Multiset.mem_coe.mpr hk))

end CQSafe

end HydroV2
