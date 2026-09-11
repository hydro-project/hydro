import HydroLean.Programs.CollectQuorumProof
import HydroLean.Programs.CollectQuorumWithResponse

/-!
# Domain-free soundness models for the quorum components (Paxos support)

`Programs/CollectQuorumProof.lean` proves the *full* abstraction theorem for
`collect_quorum` (emitted set = `qualifiedKeys`), but only **on the usage
contract** (`AtMostMaxResponses`). Paxos's safety proof must also reason about
runs where that contract is *violated* — that is exactly the faithful-variant
bug B1 (FINDINGS.md): duplicate P1as overshoot the per-key response cap.
Agreement for the guarded variant is proven by counting, and the counting
argument needs facts that hold **unconditionally**, for every input and every
batching, with no `Dom` hypothesis:

- **emission soundness**: an emitted key had at least `min` successful
  responses among the inputs consumed so far (`cqRun_emit_sound`,
  `wrRun_emit_sound`) — quorums are never fabricated, even off-contract;
- **payload provenance** (`with_response` variant): every emitted `(k, v)`
  pair is a genuine `(k, .ok v)` input (`wrRun_emit_mem`) — the accepted logs
  a leader recovers really came from acceptor replies.

These are the *sound* half of the components' models; the *complete* half
(everything qualified is emitted) is contract-dependent and lives in
`CollectQuorumProof.lean` / phase-2 obligations.

Note (FINDINGS.md B3): for `collect_quorum_with_response` with `min < max`,
the emitted **multiset** is genuinely batching-dependent (straggler successes
of already-emitted keys), so no `Unit`-choice multiset-equality abstraction
theorem exists for it — the honest shallow model is exactly this
soundness/membership characterization.
-/

namespace HydroLean.Programs

open HydroLean.Hydro

universe u v w

variable {κ : Type u} {V : Type v} {E : Type w} [DecidableEq κ]

/-! ## Generic counting lemmas -/

/-- Filtering never increases a keyed count (`antiJoin`/`filter` steps of the
quorum loops are count-decreasing). -/
theorem Stream.countKeyP_filter_le (s : Stream (κ × V)) (q : κ × V → Bool)
    (k : κ) (pr : V → Bool) :
    (s.filter q).countKeyP k pr ≤ s.countKeyP k pr := by
  unfold Stream.countKeyP Stream.filter
  rw [List.countP_filter]
  exact List.countP_mono_left fun x _ h => by
    rw [Bool.and_eq_true] at h
    exact h.1

/-- `antiJoin` never increases a keyed count. -/
theorem Stream.countKeyP_antiJoin_le (s : Stream (κ × V)) (ks : Stream κ)
    (k : κ) (pr : V → Bool) :
    (s.antiJoin ks).countKeyP k pr ≤ s.countKeyP k pr :=
  Stream.countKeyP_filter_le s _ k pr

/-- Elements survive `antiJoin` (it is a filter). -/
theorem Stream.mem_of_mem_antiJoin {s : Stream (κ × V)} {ks : Stream κ}
    {x : κ × V} (h : x ∈ s.antiJoin ks) : x ∈ s :=
  (List.mem_filter.mp h).1

/-! ## `collect_quorum` (`collectQuorumTick`): domain-free soundness -/

/-- The state sub-count invariant: retained (`notAll`) responses never count
more than the consumed input (they are a filtered sub-history of it). -/
def CQSub (consumed : Stream (κ × Except E Unit)) (s : QuorumState κ E) : Prop :=
  ∀ (k : κ) (pr : Except E Unit → Bool),
    s.notAll.countKeyP k pr ≤ consumed.countKeyP k pr

theorem cqSub_init : CQSub ([] : Stream (κ × Except E Unit)) ⟨[], []⟩ :=
  fun _ _ => Nat.le_refl _

/-- One tick preserves the sub-count invariant. -/
theorem cqStep_sub {min max : Nat} {consumed b : Stream (κ × Except E Unit)}
    {s : QuorumState κ E} (h : CQSub consumed s) :
    CQSub (consumed ++ b) ((collectQuorumTick min max).step s b).1 := by
  intro k pr
  have key : ∀ ks : Stream κ,
      ((s.notAll.chain b).antiJoin ks).countKeyP k pr
        ≤ (consumed ++ b).countKeyP k pr := by
    intro ks
    calc ((s.notAll.chain b).antiJoin ks).countKeyP k pr
        ≤ (s.notAll.chain b).countKeyP k pr :=
          Stream.countKeyP_antiJoin_le _ _ _ _
      _ = s.notAll.countKeyP k pr + b.countKeyP k pr :=
          Stream.countKeyP_append _ _ _ _
      _ ≤ consumed.countKeyP k pr + b.countKeyP k pr :=
          Nat.add_le_add_right (h k pr) _
      _ = (consumed ++ b).countKeyP k pr :=
          (Stream.countKeyP_append _ _ _ _).symm
  show ((collectQuorumTick min max).step s b).1.notAll.countKeyP k pr ≤ _
  simp only [collectQuorumTick]
  by_cases hmm : min = max
  · subst hmm
    rw [if_pos rfl]
    exact key _
  · rw [if_neg hmm]
    exact key _

/-- One tick's emissions had ≥ `min` successes among retained + batch. -/
theorem cqStep_emit_sound {min max : Nat} {s : QuorumState κ E}
    {b : Stream (κ × Except E Unit)} {k : κ}
    (hk : k ∈ ((collectQuorumTick min max).step s b).2) :
    min ≤ (s.notAll.chain b).countKeyP k Except.isOk := by
  simp only [collectQuorumTick] at hk
  by_cases hmm : min = max
  · subst hmm
    rw [if_pos rfl] at hk
    have := (List.mem_filter.mp hk).2
    simpa using this
  · rw [if_neg hmm] at hk
    have hk' : k ∈ (s.notAll.chain b).keys.filter
        (fun k' => decide (min ≤ (s.notAll.chain b).countKeyP k' Except.isOk)) :=
      (List.mem_filter.mp hk).1
    have := (List.mem_filter.mp hk').2
    simpa using this

/-- **Emission soundness for `collect_quorum`, run form** (generalized over a
start state satisfying the sub-count invariant): every key emitted by any run
suffix had ≥ `min` successful responses among (previously consumed inputs ++
the suffix's inputs). No usage-contract hypothesis. -/
theorem cqRunFrom_emit_sound {min max : Nat}
    (consumed : Stream (κ × Except E Unit)) (s : QuorumState κ E)
    (hsub : CQSub consumed s) (ts : List (Stream (κ × Except E Unit))) {k : κ}
    (hk : k ∈ allTicks ((collectQuorumTick min max).runFrom s ts).2) :
    min ≤ (consumed ++ ts.flatten).countKeyP k Except.isOk := by
  induction ts generalizing consumed s with
  | nil => cases hk
  | cons b bs ih =>
    rw [TickLoop.runFrom_cons] at hk
    rcases List.mem_append.mp hk with hhead | htail
    · -- emitted this tick: count over notAll ++ b ≤ consumed ++ b ≤ everything
      have h1 := cqStep_emit_sound (max := max) hhead
      have h2 : (s.notAll.chain b).countKeyP k Except.isOk
          ≤ (consumed ++ (b :: bs).flatten).countKeyP k Except.isOk := by
        have hle := hsub k Except.isOk
        simp only [Stream.chain, List.flatten_cons, Stream.countKeyP_append]
        omega
      omega
    · -- emitted later: instance the IH at the post-step state
      have := ih (consumed ++ b) _ (cqStep_sub hsub) htail
      rwa [List.flatten_cons, ← List.append_assoc]

/-- **Emission soundness for `collect_quorum`** from the initial state: an
emitted key had ≥ `min` successful responses among all consumed inputs — for
*every* input and *every* batching, contract or no contract. This is the
count fact Paxos's agreement argument extracts from a commit. -/
theorem cqRun_emit_sound {min max : Nat}
    {ts : List (Stream (κ × Except E Unit))} {k : κ}
    (hk : k ∈ allTicks ((collectQuorumTick min max).run ts).2) :
    min ≤ Stream.countKeyP ts.flatten k Except.isOk := by
  have := cqRunFrom_emit_sound (min := min) (max := max) [] _ cqSub_init ts hk
  simpa using this

/-! ## `collect_quorum_with_response` (`collectQuorumWRTick`) -/

/-- Sub-count + element provenance invariant for the with-response loop.
(The element half is needed because emissions carry *payloads*: we must know
each emitted `(k, v)` is a genuine input, not just count them.) -/
def WRSub (consumed : Stream (κ × Except E V)) (s : QuorumWRState κ V E) :
    Prop :=
  (∀ (k : κ) (pr : Except E V → Bool),
    s.notAll.countKeyP k pr ≤ consumed.countKeyP k pr) ∧
  (∀ x ∈ s.notAll, x ∈ consumed)

theorem wrSub_init : WRSub ([] : Stream (κ × Except E V)) ⟨[], []⟩ :=
  ⟨fun _ _ => Nat.le_refl _, fun _ h => nomatch h⟩

omit [DecidableEq κ] in
/-- The `emit` stage of the with-response loop (filterMap of `Ok`s):
membership decomposition. -/
theorem wr_emit_mem {out : Stream (κ × Except E V)} {k : κ} {v : V}
    (h : (k, v) ∈ out.filterMap (fun (p : κ × Except E V) =>
      match p.2 with
      | .ok w => some (p.1, w)
      | .error _ => none)) :
    (k, .ok v) ∈ out := by
  obtain ⟨⟨k', r⟩, hmem, hf⟩ := List.mem_filterMap.mp h
  cases r with
  | ok w =>
    simp only [Option.some.injEq, Prod.mk.injEq] at hf
    obtain ⟨rfl, rfl⟩ := hf
    exact hmem
  | error e => cases hf

/-- One tick of the with-response loop preserves the invariant. -/
theorem wrStep_sub {min max : Nat} {consumed b : Stream (κ × Except E V)}
    {s : QuorumWRState κ V E} (h : WRSub consumed s) :
    WRSub (consumed ++ b) ((collectQuorumWRTick min max).step s b).1 := by
  have hcount : ∀ ks : Stream κ, ∀ (k : κ) (pr : Except E V → Bool),
      ((s.notAll.chain b).antiJoin ks).countKeyP k pr
        ≤ (consumed ++ b).countKeyP k pr := by
    intro ks k pr
    calc ((s.notAll.chain b).antiJoin ks).countKeyP k pr
        ≤ (s.notAll.chain b).countKeyP k pr :=
          Stream.countKeyP_antiJoin_le _ _ _ _
      _ = s.notAll.countKeyP k pr + b.countKeyP k pr :=
          Stream.countKeyP_append _ _ _ _
      _ ≤ consumed.countKeyP k pr + b.countKeyP k pr :=
          Nat.add_le_add_right (h.1 k pr) _
      _ = (consumed ++ b).countKeyP k pr :=
          (Stream.countKeyP_append _ _ _ _).symm
  have hmem : ∀ ks : Stream κ, ∀ x ∈ (s.notAll.chain b).antiJoin ks,
      x ∈ consumed ++ b := by
    intro ks x hx
    rcases List.mem_append.mp (Stream.mem_of_mem_antiJoin hx) with h₁ | h₂
    · exact List.mem_append.mpr (Or.inl (h.2 x h₁))
    · exact List.mem_append.mpr (Or.inr h₂)
  show WRSub _ ((collectQuorumWRTick min max).step s b).1
  simp only [collectQuorumWRTick]
  by_cases hmm : min = max
  · subst hmm
    rw [if_pos rfl]
    exact ⟨hcount _, hmem _⟩
  · rw [if_neg hmm]
    exact ⟨hcount _, hmem _⟩

/-- One tick's with-response emissions: `(k, v)` is emitted only if `k` had
≥ `min` successes in retained + batch AND `(k, .ok v)` is a genuine retained
or batch element. -/
theorem wrStep_emit_sound {min max : Nat} {s : QuorumWRState κ V E}
    {b : Stream (κ × Except E V)} {k : κ} {v : V}
    (hk : (k, v) ∈ ((collectQuorumWRTick min max).step s b).2) :
    min ≤ (s.notAll.chain b).countKeyP k Except.isOk ∧
      (k, .ok v) ∈ s.notAll.chain b := by
  simp only [collectQuorumWRTick] at hk
  by_cases hmm : min = max
  · subst hmm
    rw [if_pos rfl] at hk
    have hmem := wr_emit_mem hk
    have hfil := List.mem_filter.mp hmem
    refine ⟨?_, hfil.1⟩
    have := hfil.2
    simp only [Bool.not_eq_true', decide_eq_false_iff_not, Nat.not_lt] at this
    exact this
  · rw [if_neg hmm] at hk
    have hmem := wr_emit_mem hk
    have hmem' : ((k : κ), (Except.ok v : Except E V)) ∈
        (s.notAll.chain b).filter
          (fun p => !decide ((s.notAll.chain b).countKeyP p.1 Except.isOk < min)) :=
      Stream.mem_of_mem_antiJoin hmem
    have hfil := List.mem_filter.mp hmem'
    refine ⟨?_, hfil.1⟩
    have := hfil.2
    simp only [Bool.not_eq_true', decide_eq_false_iff_not, Nat.not_lt] at this
    exact this

/-- Run-form soundness for the with-response loop, generalized start. -/
theorem wrRunFrom_emit_sound {min max : Nat}
    (consumed : Stream (κ × Except E V)) (s : QuorumWRState κ V E)
    (hsub : WRSub consumed s) (ts : List (Stream (κ × Except E V)))
    {k : κ} {v : V}
    (hk : (k, v) ∈ allTicks ((collectQuorumWRTick min max).runFrom s ts).2) :
    min ≤ (consumed ++ ts.flatten).countKeyP k Except.isOk ∧
      (k, .ok v) ∈ consumed ++ ts.flatten := by
  induction ts generalizing consumed s with
  | nil => cases hk
  | cons b bs ih =>
    rw [TickLoop.runFrom_cons] at hk
    rcases List.mem_append.mp hk with hhead | htail
    · obtain ⟨hcnt, hel⟩ := wrStep_emit_sound (max := max) hhead
      constructor
      · have h2 : (s.notAll.chain b).countKeyP k Except.isOk
            ≤ (consumed ++ (b :: bs).flatten).countKeyP k Except.isOk := by
          have hle := hsub.1 k Except.isOk
          simp only [Stream.chain, List.flatten_cons, Stream.countKeyP_append]
          omega
        omega
      · rw [List.flatten_cons, ← List.append_assoc]
        rcases List.mem_append.mp hel with h₁ | h₂
        · exact List.mem_append.mpr
            (Or.inl (List.mem_append.mpr (Or.inl (hsub.2 _ h₁))))
        · exact List.mem_append.mpr
            (Or.inl (List.mem_append.mpr (Or.inr h₂)))
    · have := ih (consumed ++ b) _ (wrStep_sub hsub) htail
      rwa [List.flatten_cons, ← List.append_assoc]

/-- **Emission soundness + payload provenance for
`collect_quorum_with_response`**: for every input and every batching (no
contract), an emitted `(k, v)` had ≥ `min` successful responses for `k` among
all consumed inputs, and `(k, .ok v)` is a genuine consumed input. In Paxos:
a "quorum of accepted logs" is only ever assembled from real P1b `Ok` replies
whose ballot key received ≥ `f + 1` `Ok`s (counted with multiplicity — the
faithful-variant bug is precisely that multiplicity ≠ distinct acceptors). -/
theorem wrRun_emit_sound {min max : Nat}
    {ts : List (Stream (κ × Except E V))} {k : κ} {v : V}
    (hk : (k, v) ∈ allTicks ((collectQuorumWRTick min max).run ts).2) :
    min ≤ Stream.countKeyP ts.flatten k Except.isOk ∧ (k, .ok v) ∈ ts.flatten := by
  have := wrRunFrom_emit_sound (min := min) (max := max) [] _ wrSub_init ts hk
  simpa using this

end HydroLean.Programs
