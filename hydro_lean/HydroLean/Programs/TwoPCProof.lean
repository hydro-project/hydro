import HydroLean.Programs.TwoPC

/-!
# Unbounded correctness of `two_pc` (all inputs, all decisions)

The Rust `nondet!(/** TODO */)` justifications of `two_pc.rs`, supplied as
theorems over the decisions-as-inputs port (`Programs/TwoPC.lean`): for
**every** complete pair of quorum decisions (= every arrival shuffle and
every batching of both fan-ins), the committed output is — as a multiset —
exactly the unanimously-approved input payloads. Determinism, unanimity
safety, and exactly-once delivery are corollaries of the single master
equation.

The quorum engine facts live in the shared core
(`Programs/CollectQuorumMinMax.lean`); this file only counts the
broadcast-echo traffic and assembles.
-/

namespace HydroLean.Programs

open HydroLean.Hydro
open HydroLean.Hydro.Stream


/-! ## The 2PC theorems -/

section Main

variable {P : Type} [DecidableEq P]

/-- The committed output is the quorum loop over the (complete, hence legal)
acks decision — the run-level unfolding of `two_pc` through
`Consumes.batchC_eq`. -/
theorem two_pc_outputs {n : Nat} {vote : Fin n → P → Bool}
    {payloads : Stream P} {d : TwoPCDecisions n P}
    (hv : d.Valid n vote payloads) :
    two_pc n vote payloads d
      = ((collectQuorumTick (E := Unit) n n).outputs d.acks).flatten := by
  have hdef : two_pc n vote payloads d
      = ((collectQuorumTick (E := Unit) n n).outputs
          (batchC
            (unionF ((tpc_c_commitsM n vote d.votes).f payloads))
            [] d.acks)).flatten := rfl
  rw [hdef, hv.2.batchC_eq]

/-- Likewise for the phase-1 output. -/
theorem two_pcVoteYes_outputs {n : Nat} {vote : Fin n → P → Bool}
    {payloads : Stream P} {d : TwoPCDecisions n P}
    (hv : d.Valid n vote payloads) :
    two_pcVoteYes n vote payloads d.votes
      = ((collectQuorumTick (E := Unit) n n).outputs d.votes).flatten := by
  have hdef : two_pcVoteYes n vote payloads d.votes
      = ((collectQuorumTick (E := Unit) n n).outputs
          (batchC (unionF ((tpc_c_votesM n vote).f payloads))
            [] d.votes)).flatten := rfl
  rw [hdef, hv.1.batchC_eq]

/-- Phase 1, characterized: for every complete decision, the
`c_all_vote_yes` stream contains exactly the payloads unanimously voted yes,
each exactly once. -/
theorem twoPCVoteYes_char {n : Nat} (hn : 1 ≤ n) (vote : Fin n → P → Bool)
    {payloads : Stream P} (hnd : payloads.Nodup) {d : TwoPCDecisions n P}
    (hv : d.Valid n vote payloads) :
    (two_pcVoteYes n vote payloads d.votes).Nodup ∧
    ∀ k, k ∈ two_pcVoteYes n vote payloads d.votes ↔
      (k ∈ payloads ∧ ∀ i, vote i k) := by
  -- transported per-key counts (decision-complete ⇒ counts = family sums)
  have hcnts : ∀ (k : P) (Pr : Except Unit Unit → Bool),
      Stream.countKeyP d.votes.flatten k Pr =
        ((List.finRange n).map
          (fun i => if Pr (voteE vote i k) then payloads.count k else 0)).sum := by
    intro k Pr
    rw [Stream.countKeyP_perm hv.1]
    exact (countKeyP_unionF (fun i => (tpc_c_votesM n vote).f payloads i) k
      Pr).trans
      (congrArg (fun f => (List.map f (List.finRange n)).sum)
        (funext fun i => votesBuf_countKeyP vote payloads i k Pr))
  -- the response contract: at most n responses per key
  have hmax : ∀ k, Stream.countKey d.votes.flatten k ≤ n := by
    intro k
    rw [countKey_eq_countKeyP_true, hcnts k (fun _ => true)]
    simp only [if_true]
    rw [List.sum_map_const, List.length_finRange]
    have h1 : n * List.count k payloads ≤ n * 1 :=
      Nat.mul_le_mul_left n (hnd.count_le_one k)
    simpa using h1
  obtain ⟨hnodup, hmem⟩ :=
    collectQuorum_minEqMax_correct n hn d.votes hmax
  rw [two_pcVoteYes_outputs hv]
  refine ⟨hnodup, fun k => ?_⟩
  rw [hmem k]
  unfold cnt
  rw [hcnts k Except.isOk]
  simp only [isOk_voteE]
  rw [hnd.count_eq_ite k]
  by_cases hk : k ∈ payloads
  · simp only [hk, if_true, true_and]
    have : ((List.finRange n).map (fun i => if vote i k then 1 else 0)).sum
        = (List.finRange n).countP (fun i => vote i k) :=
      List.sum_map_ite_one _ _
    rw [show ((List.finRange n).map
          (fun i => if vote i k = true then (1 : Nat) else 0))
        = ((List.finRange n).map (fun i => if vote i k then 1 else 0)) from rfl,
      this]
    have hle : (List.finRange n).countP (fun i => vote i k)
        ≤ (List.finRange n).length := List.countP_le_length
    rw [List.length_finRange] at hle
    constructor
    · intro h
      have heq : (List.finRange n).countP (fun i => vote i k)
          = (List.finRange n).length := by
        rw [List.length_finRange]; omega
      have := List.countP_eq_length.mp heq
      exact fun i => this i (List.mem_finRange i)
    · intro h
      have : (List.finRange n).countP (fun i => vote i k)
          = (List.finRange n).length :=
        List.countP_eq_length.mpr fun i _ => h i
      rw [List.length_finRange] at this
      omega
  · simp only [hk, if_false, false_and, iff_false]
    intro hle
    have : ((List.finRange n).map
        (fun i => if vote i k = true then (0 : Nat) else 0)).sum = 0 := by
      rw [show (fun i : Fin n => if vote i k = true then (0 : Nat) else 0)
          = (fun _ : Fin n => (0 : Nat)) from funext fun i => by
        by_cases h : vote i k <;> simp [h]]
      rw [List.sum_map_const]
      omega
    omega

/-- **Master theorem for 2PC**: for every valid pair of decisions (the
entire content of the Rust `nondet!` guards and `NoOrder` markers), the
committed stream is — as a multiset (`Perm`) — the unanimously-approved
input payloads. The right-hand side mentions no decision: determinism,
safety, and exactly-once delivery are all corollaries of this single
equation. -/
theorem twoPC_committed_eq {n : Nat} (hn : 1 ≤ n) (vote : Fin n → P → Bool)
    {payloads : Stream P} (hnd : payloads.Nodup) {d : TwoPCDecisions n P}
    (hv : d.Valid n vote payloads) :
    (two_pc n vote payloads d).Perm
      (payloads.filter (fun p => (List.finRange n).all (vote · p))) := by
  obtain ⟨hyesNodup, hyesMem⟩ := twoPCVoteYes_char hn vote hnd hv
  have hcnts : ∀ (k : P) (Pr : Except Unit Unit → Bool),
      Stream.countKeyP d.acks.flatten k Pr =
        ((List.finRange n).map
          (fun _ => if Pr (.ok ()) then
            (two_pcVoteYes n vote payloads d.votes).count k else 0)).sum := by
    intro k Pr
    rw [Stream.countKeyP_perm hv.2]
    exact (countKeyP_unionF
      (fun i => (tpc_c_commitsM n vote d.votes).f payloads i) k
        Pr).trans
      (congrArg (fun f => (List.map f (List.finRange n)).sum)
        (funext fun i =>
          acksBuf_countKeyP vote payloads d.votes i k Pr))
  have hcount : ∀ k : P, (two_pcVoteYes n vote payloads d.votes).count k
      = if k ∈ two_pcVoteYes n vote payloads d.votes then 1 else 0 :=
    hyesNodup.count_eq_ite
  have hmax : ∀ k, Stream.countKey d.acks.flatten k ≤ n := by
    intro k
    rw [countKey_eq_countKeyP_true, hcnts k (fun _ => true)]
    simp only [if_true]
    rw [List.sum_map_const, List.length_finRange, hcount k]
    by_cases hk : k ∈ two_pcVoteYes n vote payloads d.votes <;> simp [hk]
  obtain ⟨hnodup, hmem⟩ :=
    collectQuorum_minEqMax_correct n hn d.acks hmax
  -- the committed keys are exactly the phase-1 keys
  have hmem' : ∀ k, k ∈ two_pc n vote payloads d ↔
      k ∈ two_pcVoteYes n vote payloads d.votes := by
    intro k
    rw [two_pc_outputs hv, hmem k]
    unfold cnt
    rw [hcnts k Except.isOk]
    rw [show (Except.isOk (Except.ok ()) : Bool) = true from rfl]
    simp only [if_true]
    rw [List.sum_map_const, List.length_finRange, hcount k]
    by_cases hk : k ∈ two_pcVoteYes n vote payloads d.votes
    · simp only [hk, if_true, Nat.mul_one, iff_true]
      exact Nat.le_refl n
    · simp only [hk, if_false, Nat.mul_zero, iff_false]
      omega
  -- assemble: committed ~ payloads filtered by unanimity
  have hcommitNodup : (two_pc n vote payloads d).Nodup := by
    rw [two_pc_outputs hv]; exact hnodup
  refine List.perm_of_nodup_of_mem_iff hcommitNodup
    (List.Sublist.nodup List.filter_sublist hnd) fun k => ?_
  rw [hmem' k, hyesMem k]
  show _ ↔ k ∈ List.filter
    (fun p => (List.finRange n).all (fun i => vote i p)) payloads
  rw [List.mem_filter, List.all_eq_true]
  exact and_congr_right fun _ =>
    ⟨fun h i _ => h i, fun h i => h i (List.mem_finRange i)⟩

/-- **Unanimity safety**: a committed payload was voted yes by *every*
participant — under every decision. (The Rust `nondet!(/** TODO */)`
justification, supplied.) -/
theorem twoPC_unanimity {n : Nat} (hn : 1 ≤ n) (vote : Fin n → P → Bool)
    {payloads : Stream P} (hnd : payloads.Nodup) {d : TwoPCDecisions n P}
    (hv : d.Valid n vote payloads) {p : P}
    (hp : p ∈ two_pc n vote payloads d) :
    p ∈ payloads ∧ ∀ i, vote i p := by
  have hp' := (twoPC_committed_eq hn vote hnd hv).mem_iff.mp hp
  have hf := List.mem_filter.mp hp'
  exact ⟨hf.1, fun i => List.all_eq_true.mp hf.2 i (List.mem_finRange i)⟩

/-- **Exactly-once**: no payload commits twice, under every decision. -/
theorem twoPC_nodup {n : Nat} (hn : 1 ≤ n) (vote : Fin n → P → Bool)
    {payloads : Stream P} (hnd : payloads.Nodup) {d : TwoPCDecisions n P}
    (hv : d.Valid n vote payloads) : (two_pc n vote payloads d).Nodup :=
  (twoPC_committed_eq hn vote hnd hv).nodup_iff.mpr
    (List.Sublist.nodup List.filter_sublist hnd)

/-- **All-yes determinism** (the Rust code's actual configuration, where
every echo is `Ok(())`): the committed multiset *is* the input payload
multiset — 2PC delivers exactly the submitted transactions, once each, no
matter what the decisions are. -/
theorem twoPC_all_yes_deterministic {n : Nat} (hn : 1 ≤ n)
    {payloads : Stream P} (hnd : payloads.Nodup) {d : TwoPCDecisions n P}
    (hv : d.Valid n (fun _ _ => true) payloads) :
    (two_pc n (fun _ _ => true) payloads d).Perm payloads := by
  have h := twoPC_committed_eq hn (fun _ _ => true) hnd hv
  have hfe : payloads.filter
      (fun p => (List.finRange n).all ((fun _ _ => true) · p)) = payloads := by
    show List.filter _ payloads = payloads
    exact List.filter_eq_self.mpr fun a _ => by simp
  rwa [hfe] at h

/-- **Eventual determinism**: any two valid decision pairs produce the
*same* committed multiset. This is Gyatso's eventual-determinism property
(Thm 3.4.1) manifested at the program level: the `NoOrder` output
denotation is a function of the inputs alone. -/
theorem twoPC_deterministic {n : Nat} (hn : 1 ≤ n) (vote : Fin n → P → Bool)
    {payloads : Stream P} (hnd : payloads.Nodup) {d₁ d₂ : TwoPCDecisions n P}
    (hv₁ : d₁.Valid n vote payloads) (hv₂ : d₂.Valid n vote payloads) :
    (two_pc n vote payloads d₁).Perm (two_pc n vote payloads d₂) :=
  (twoPC_committed_eq hn vote hnd hv₁).trans
    (twoPC_committed_eq hn vote hnd hv₂).symm

end Main

end HydroLean.Programs
