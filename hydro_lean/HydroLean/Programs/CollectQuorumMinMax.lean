import HydroLean.Programs.CollectQuorum
import HydroLean.Hydro.StreamLemmas
import HydroLean.Hydro.TStream

/-!
# `collect_quorum` with `min = max`: the count characterization (shared core)

The `min = max = n` branch of `hydro_std::quorum::collect_quorum`,
characterized purely by per-key success counts of the consumed input — the
**batching-free** face behind "deterministic quorum results" claims. Shared
by `two_pc` (`n`-of-`n`, `Programs/TwoPC.lean` / `TwoPCProof.lean`), the
open-membership epoch classes (`k`-of-`k`,
`Programs/TwoPCOpenMembership.lean`), and available to any decisions-as-
inputs program that consumes `collect_quorumM` (the loop here is the same
`collectQuorumTick` that `Programs/CollectQuorumStreams.lean` wires for
Paxos).

Everything is stated over a decided batch list `d : List (List _)` — exactly
the `batchC` decision carrier of the new surface (`docs/10`).
-/

namespace HydroLean.Programs

open HydroLean.Hydro
open HydroLean.Hydro.Stream

variable {κ : Type} [DecidableEq κ] {E : Type}


variable {E : Type}

/-- Successes of key `k` in `q` (shorthand). -/
def cnt (q : Stream (κ × Except E Unit)) (k : κ) : Nat :=
  q.countKeyP k Except.isOk

/-- The per-key count characterization of the loop state after consuming
prefix `p` (under any batching): `not_all` holds exactly the rows of keys
that have not yet reached `n` successes. Quantified over all value predicates
`P`, which is everything a downstream observable can see. -/
def StateCnt (n : Nat) (p : Stream (κ × Except E Unit))
    (s : QuorumState κ E) : Prop :=
  ∀ (k : κ) (P : Except E Unit → Bool),
    s.notAll.countKeyP k P =
      if cnt p k < n then p.countKeyP k P else 0

theorem stateCnt_init (n : Nat) :
    StateCnt (κ := κ) (E := E) n [] (collectQuorumTick n n).init := by
  intro k P
  by_cases h : cnt (κ := κ) (E := E) [] k < n <;>
    simp [collectQuorumTick, h] <;> rfl

/-- The three facts about one tick of the `min = max` branch: the state
invariant advances from `p` to `p ++ b`, and the emitted keys are exactly the
threshold-crossers, without duplicates. -/
theorem step_char (n : Nat) {p b : Stream (κ × Except E Unit)}
    {s : QuorumState κ E} (hs : StateCnt n p s)
    (hmax : ∀ k, (p ++ b).countKey k ≤ n) :
    StateCnt n (p ++ b) ((collectQuorumTick n n).step s b).1 ∧
    ((collectQuorumTick n n).step s b).2.Nodup ∧
    ∀ k, k ∈ ((collectQuorumTick n n).step s b).2 ↔
      (cnt p k < n ∧ n ≤ cnt (p ++ b) k) := by
  -- keys that reached quorum in `p` receive nothing in `b` (response contract)
  have hquorum_no_b : ∀ (k : κ) (P : Except E Unit → Bool), ¬ cnt p k < n →
      b.countKeyP k P = 0 := by
    intro k P hk
    have h₁ : n ≤ p.countKey k :=
      Nat.le_trans (Nat.le_of_not_lt hk) (countKeyP_le_countKey p k _)
    have h₂ := hmax k
    rw [Stream.countKey_append] at h₂
    have hbz : b.countKey k = 0 := by omega
    have hle := countKeyP_le_countKey b k P
    omega
  -- per-key counts of `current = not_all ++ b`
  have hcur : ∀ (k : κ) (P : Except E Unit → Bool),
      (s.notAll.chain b).countKeyP k P =
        if cnt p k < n then (p ++ b).countKeyP k P else 0 := by
    intro k P
    show Stream.countKeyP (s.notAll ++ b) k P = _
    rw [Stream.countKeyP_append, hs k P]
    by_cases h : cnt p k < n
    · simp only [h, if_true, Stream.countKeyP_append]
    · simp only [h, if_false, Nat.zero_add, hquorum_no_b k P h]
  -- the current success count, specialized
  have hcurcnt : ∀ k : κ, cnt (s.notAll.chain b) k =
      if cnt p k < n then cnt (p ++ b) k else 0 := fun k => hcur k Except.isOk
  have hcurtot : ∀ k : κ, (s.notAll.chain b).countKey k =
      if cnt p k < n then (p ++ b).countKey k else 0 := by
    intro k
    rw [countKey_eq_countKeyP_true, hcur k (fun _ => true),
      ← countKey_eq_countKeyP_true]
  -- reduce the tick to the min = max branch
  have hstep : (collectQuorumTick (κ := κ) (E := E) n n).step s b =
      (⟨(s.notAll.chain b).antiJoin
          ((s.notAll.chain b).keys.filter
            (fun k => decide (n ≤ cnt (s.notAll.chain b) k))), []⟩,
        (s.notAll.chain b).keys.filter
          (fun k => decide (n ≤ cnt (s.notAll.chain b) k))) := by
    simp only [collectQuorumTick]
    rfl
  -- membership in the emitted keys
  have hmem_out : ∀ k : κ,
      k ∈ (s.notAll.chain b).keys.filter
          (fun k => decide (n ≤ cnt (s.notAll.chain b) k)) ↔
        (cnt p k < n ∧ n ≤ cnt (p ++ b) k) := by
    intro k
    show k ∈ List.filter _ _ ↔ _
    rw [List.mem_filter, mem_keys_iff, decide_eq_true_eq, hcurcnt k, hcurtot k]
    by_cases h : cnt p k < n
    · simp only [h, if_true, true_and]
      constructor
      · rintro ⟨-, h₂⟩
        exact h₂
      · intro h₂
        have hle : cnt (p ++ b) k ≤ (p ++ b).countKey k :=
          countKeyP_le_countKey _ _ _
        have : 1 ≤ n := by
          rcases Nat.eq_zero_or_pos n with hn | hn
          · exact absurd (hn ▸ h) (Nat.not_lt_zero _)
          · exact hn
        refine ⟨by unfold cnt at h₂ hle; omega, h₂⟩
    · simp only [h, if_false, false_and, iff_false, not_and]
      intro h₀
      omega
  rw [hstep]
  refine ⟨?_, ?_, hmem_out⟩
  · -- new state counts like p ++ b
    intro k P
    show Stream.countKeyP (Stream.antiJoin _ _) k P = _
    rw [Stream.antiJoin, Stream.filter]
    rw [countKeyP_filter_keyconst (s.notAll.chain b)
      (fun k => !List.contains _ k) k P, hcur k P]
    by_cases hout : cnt p k < n ∧ n ≤ cnt (p ++ b) k
    · -- emitted this tick ⇒ dropped from the state; quorum reached in p ++ b
      have hmem := (hmem_out k).mpr hout
      have hcontains : List.contains ((s.notAll.chain b).keys.filter
          (fun k => decide (n ≤ cnt (s.notAll.chain b) k))) k = true :=
        List.contains_iff_mem.mpr hmem
      simp only [hcontains, Bool.not_true]
      have : ¬ cnt (p ++ b) k < n := by omega
      simp [this]
    · have hncontains : List.contains ((s.notAll.chain b).keys.filter
          (fun k => decide (n ≤ cnt (s.notAll.chain b) k))) k = false := by
        rw [← Bool.not_eq_true, List.contains_iff_mem]
        exact fun hmem => hout ((hmem_out k).mp hmem)
      simp only [hncontains, Bool.not_false, if_true]
      by_cases h : cnt p k < n
      · -- not yet reached in p ++ b either (else it would have been emitted)
        have h₂ : ¬ n ≤ cnt (p ++ b) k := fun hle => hout ⟨h, hle⟩
        have h₃ : cnt (p ++ b) k < n := Nat.lt_of_not_le h₂
        simp [h, h₃]
      · -- already reached in p: counts are 0 on both sides
        have h₃ : ¬ cnt (p ++ b) k < n := by
          have : cnt p k ≤ cnt (p ++ b) k := by
            unfold cnt
            rw [Stream.countKeyP_append]
            omega
          omega
        simp [h, h₃]
  · -- emitted keys are duplicate-free
    exact List.Sublist.nodup List.filter_sublist (keys_nodup _)

/-- Multi-tick characterization: running the loop over any batch list, from a
state counting like prefix `p`, emits exactly the keys crossing the threshold
during those batches, each exactly once. -/
theorem run_char (n : Nat) :
    ∀ (bs : List (Stream (κ × Except E Unit))) (p : Stream (κ × Except E Unit))
      (s : QuorumState κ E), StateCnt n p s →
      (∀ k, (p ++ bs.flatten).countKey k ≤ n) →
      ((collectQuorumTick n n).runFrom s bs).2.flatten.Nodup ∧
      ∀ k, k ∈ ((collectQuorumTick n n).runFrom s bs).2.flatten ↔
        (cnt p k < n ∧ n ≤ cnt (p ++ bs.flatten) k) := by
  intro bs
  induction bs with
  | nil =>
    intro p s hs hmax
    refine ⟨List.nodup_nil, fun k => ?_⟩
    simp only [TickLoop.runFrom_nil, List.flatten_nil, List.not_mem_nil,
      false_iff, List.append_nil]
    rintro ⟨h₁, h₂⟩
    omega
  | cons b bs ih =>
    intro p s hs hmax
    have hmax₁ : ∀ k, (p ++ b).countKey k ≤ n := by
      intro k
      have h := hmax k
      simp only [List.flatten_cons, ← List.append_assoc] at h
      have : ((p ++ b) ++ bs.flatten).countKey k =
          (p ++ b).countKey k + Stream.countKey bs.flatten k :=
        Stream.countKey_append _ _ _
      omega
    obtain ⟨hs', hnodup₀, hmem₀⟩ := step_char n hs hmax₁
    have hmax₂ : ∀ k, ((p ++ b) ++ bs.flatten).countKey k ≤ n := by
      intro k
      have := hmax k
      simp only [List.flatten_cons, ← List.append_assoc] at this
      exact this
    obtain ⟨hnodup₁, hmem₁⟩ :=
      ih (p ++ b) ((collectQuorumTick n n).step s b).1 hs' hmax₂
    have hflat : ((collectQuorumTick n n).runFrom s (b :: bs)).2.flatten =
        ((collectQuorumTick n n).step s b).2 ++
          ((collectQuorumTick n n).runFrom
            ((collectQuorumTick n n).step s b).1 bs).2.flatten := by
      simp [TickLoop.runFrom_cons]
    rw [hflat]
    have cnt_le_append : ∀ (q r : Stream (κ × Except E Unit)) k,
        cnt q k ≤ cnt (q ++ r) k := by
      intro q r k
      show cnt q k ≤ cnt (q ++ r) k
      have h := Stream.countKeyP_append q r k Except.isOk
      unfold cnt
      omega
    have hmono : ∀ k, cnt p k ≤ cnt (p ++ b) k := fun k => cnt_le_append p b k
    have hmono₂ : ∀ k, cnt (p ++ b) k ≤ cnt ((p ++ b) ++ bs.flatten) k :=
      fun k => cnt_le_append (p ++ b) bs.flatten k
    constructor
    · rw [List.nodup_append]
      refine ⟨hnodup₀, hnodup₁, fun k hk k' hk' => ?_⟩
      rintro rfl
      have h₀ := (hmem₀ k).mp hk
      have h₁ := (hmem₁ k).mp hk'
      omega
    · intro k
      rw [List.mem_append, hmem₀ k, hmem₁ k]
      have hassoc : (p ++ (b :: bs).flatten) = ((p ++ b) ++ bs.flatten) := by
        simp [List.append_assoc]
      rw [hassoc]
      have h₁ := hmono k
      have h₂ := hmono₂ k
      constructor
      · rintro (⟨ha, hb⟩ | ⟨ha, hb⟩)
        · exact ⟨ha, by omega⟩
        · exact ⟨by omega, hb⟩
      · rintro ⟨ha, hb⟩
        by_cases hc : n ≤ cnt (p ++ b) k
        · exact .inl ⟨ha, hc⟩
        · exact .inr ⟨by omega, hb⟩

/-- **Correctness of `collect_quorum` with `min = max = n`** (the
configuration used by 2PC), for every decided batch list: under the response
contract (at most `n` responses per key), the emitted keys are exactly those
with `n` successful responses, each emitted exactly once. The
characterization mentions only the *flattened* consumption — the decision
(batching) shape is irrelevant, which is the deterministic-quorum-results
claim of the Rust `nondet!` justification. -/
theorem collectQuorum_minEqMax_correct (n : Nat) (hn : 1 ≤ n)
    (d : List (Stream (κ × Except E Unit)))
    (hmax : ∀ k, Stream.countKey d.flatten k ≤ n) :
    ((collectQuorumTick (κ := κ) (E := E) n n).outputs d).flatten.Nodup ∧
    ∀ k, k ∈ ((collectQuorumTick (κ := κ) (E := E) n n).outputs d).flatten ↔
      n ≤ cnt d.flatten k := by
  have h := run_char (κ := κ) (E := E) n d [] (collectQuorumTick n n).init
    (stateCnt_init n) (by simpa using hmax)
  simp only [List.nil_append] at h
  refine ⟨h.1, fun k => ?_⟩
  rw [show (collectQuorumTick (κ := κ) (E := E) n n).outputs d =
    ((collectQuorumTick n n).runFrom (collectQuorumTick n n).init d).2 from rfl]
  rw [h.2 k]
  constructor
  · rintro ⟨-, h₂⟩
    exact h₂
  · intro h₂
    refine ⟨?_, h₂⟩
    show cnt (κ := κ) (E := E) [] k < n
    unfold cnt
    show Stream.countKeyP [] k _ < n
    simp [Stream.countKeyP]
    omega

/-- Keyed counts over the cluster fan-in union (`unionF`) are sums of
per-member counts — the clusters-as-maps counting exchange. -/
theorem countKeyP_unionF {n : Nat} {V : Type}
    (bufs : Fin n → List (κ × V)) (k : κ) (Pr : V → Bool) :
    Stream.countKeyP (unionF bufs) k Pr
      = ((List.finRange n).map
          (fun i => Stream.countKeyP (bufs i) k Pr)).sum := by
  unfold Stream.countKeyP unionF
  rw [List.countP_flatMap]
  rfl

end HydroLean.Programs
