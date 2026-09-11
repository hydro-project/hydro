import HydroLean.Hydro.TStream
import HydroLean.Hydro.Growth
import HydroLean.Programs.PaxosQuorumModel
import HydroLean.Programs.CollectQuorumProof
import HydroLean.Programs.PaxosQuorumCount

/-!
# `collect_quorum` / `collect_quorum_with_response` over located streams

The `hydro_std::quorum` functions (quorum.rs:7–133) in the
decisions-as-inputs surface (`Hydro/TStream.lean`): each takes the
**member-indexed family** view of its `NoOrder` input (`cluster → x`
networking is the identity on the keyed view) and the materialized `nondet!`
batching decision. The input is `NoOrder`, so the decision is the consumed
batch itself (`batchC` over the members' union multiset — all ordering
information, including per-member arrival order, is adversarial) and the
returned pair is exactly the Rust output (successes, fails): the success
side is the established verified tick loop (`collectQuorumTick` /
`collectQuorumWRTick`) run over the decided batches; the error side is
computed outside the loop from the raw input, exactly as in Rust
(quorum.rs:82–85).

**Verified face** (interface theorems, module-owned): quorum extraction with
the module's *input requirement* — at most one success per member per key,
the contract paxos.rs violates in FINDINGS.md B1 — as an explicit
hypothesis; the consumption side is discharged *internally* from the batch
legality (`batchC_count_le`), so callers see only their own streams.
-/

set_option synthInstance.maxSize 1024

namespace HydroLean.Programs

open HydroLean.Hydro

universe u v w


variable {κ : Type u} {V : Type v} {E : Type w} [DecidableEq κ] {n : Nat}

/-- The `Ok`-at-key-`k` indicator on quorum inputs (any payload). -/
def okKey (k : κ) : κ × Except E V → Bool :=
  fun e => decide (e.1 = k) && Except.isOk e.2

/-- quorum.rs:7–87 `collect_quorum`, as the typed dataflow (family view of
the `NoOrder` input; `nondet` is the consumed-batch decision of its
`sliced!` clock). Returns (successes, per-member fails). -/
def collect_quorumM [DecidableEq E] (min max : Nat)
    (nondet : List (List (κ × Except E Unit))) :
    (Fin n → Stream (κ × Except E Unit)) →ₘ
      Stream κ × (Fin n → Stream (κ × E)) :=
  let batches := (MonoMap.id.unionF).batchC nondet
  MonoMap.pair
    ((batches.loop (collectQuorumTick min max)).flatten)
    (MonoMap.piMap (fun _j => filterMapM (fun kr =>
      match kr.2 with
      | .error e => some (kr.1, e)
      | .ok _ => none)))

/-- The transcription's function face (`.f` of `collect_quorumM`). -/
abbrev collect_quorum [DecidableEq E]
    (responses : Fin n → Stream (κ × Except E Unit))
    (min max : Nat) (nondet : List (List (κ × Except E Unit))) :
    Stream κ × (Fin n → Stream (κ × E)) :=
  (collect_quorumM min max nondet).f responses

/-- quorum.rs:89–133 `collect_quorum_with_response`, as the typed dataflow.
Returns ((key, payload) successes, per-member fails). -/
def collect_quorum_with_responseM [DecidableEq E] [DecidableEq V]
    (min max : Nat) (nondet : List (List (κ × Except E V))) :
    (Fin n → Stream (κ × Except E V)) →ₘ
      Stream (κ × V) × (Fin n → Stream (κ × E)) :=
  let batches := (MonoMap.id.unionF).batchC nondet
  MonoMap.pair
    ((batches.loop (collectQuorumWRTick min max)).flatten)
    (MonoMap.piMap (fun _j => filterMapM (fun kr =>
      match kr.2 with
      | .error e => some (kr.1, e)
      | .ok _ => none)))

/-- The transcription's function face (`.f` of
`collect_quorum_with_responseM`). -/
abbrev collect_quorum_with_response [DecidableEq E] [DecidableEq V]
    (responses : Fin n → Stream (κ × Except E V))
    (min max : Nat) (nondet : List (List (κ × Except E V))) :
    Stream (κ × V) × (Fin n → Stream (κ × E)) :=
  (collect_quorum_with_responseM min max nondet).f responses

section Faces

variable [DecidableEq E] [DecidableEq V]

/-- Consumption is `countP`-dominated by the member-indexed sum (batch
legality + the cluster-as-map counting exchange, discharged once here). -/
theorem batchC_countP_le_sum {responses : Fin n → Stream (κ × Except E V)}
    (nondet : List (List (κ × Except E V))) (p : κ × Except E V → Bool) :
    ((batchC (unionF responses) [] nondet).flatten).countP p
      ≤ ((List.finRange n).map (fun j => (responses j).countP p)).sum := by
  calc ((batchC (unionF responses) [] nondet).flatten).countP p
      ≤ (unionF responses).countP p :=
        countP_le_of_count_le (batchC_count_le _ nondet) p
    _ = ((List.finRange n).map (fun j => (responses j).countP p)).sum := by
        unfold unionF
        exact countP_flatMap_eq_sum _ _ p

/-- **Face: emitted key ⇒ ≥ `min` distinct members succeeded** — the
`collect_quorum` quorum extraction, with the module's input requirement
(≤ 1 success per member per key) as its only hypothesis. -/
theorem collect_quorum_distinct_members
    {responses : Fin n → Stream (κ × Except E Unit)} {min max : Nat}
    {nondet : List (List (κ × Except E Unit))} {k : κ}
    (hk : k ∈ (collect_quorum responses min max nondet).1)
    (hcap : ∀ j : Fin n, (responses j).countP (okKey k) ≤ 1) :
    ∃ S : List (Fin n), S.Nodup ∧ min ≤ S.length ∧
      ∀ j ∈ S, ∃ x ∈ responses j, okKey k x = true := by
  have hemit := cqRun_emit_sound (min := min) (max := max) hk
  have hsum : min ≤ ((List.finRange n).map
      (fun j => (responses j).countP (okKey k))).sum :=
    Nat.le_trans hemit (batchC_countP_le_sum nondet (okKey k))
  exact family_extract_distinct responses (okKey k) min hcap hsum

/-- `okIs k v` refines `okKey k`. -/
theorem okIs_imp_okKey (k : κ) (v : V) (e : κ × Except E V)
    (h : okIs k v e = true) : okKey k e = true := by
  unfold okIs at h
  unfold okKey
  rw [Bool.and_eq_true] at h ⊢
  refine ⟨h.1, ?_⟩
  cases hr : e.2 with
  | ok w => rfl
  | error err =>
    rw [hr] at h
    cases h.2

/-- Distinct `okIs` classes are disjoint. -/
theorem okIs_disjoint (k : κ) {v v' : V} (hne : v ≠ v')
    (e : κ × Except E V) :
    ¬(okIs k v e = true ∧ okIs k v' e = true) := by
  rintro ⟨h1, h2⟩
  unfold okIs at h1 h2
  rw [Bool.and_eq_true] at h1 h2
  cases hr : e.2 with
  | ok w =>
    rw [hr] at h1 h2
    have hv : w = v := by
      have := h1.2
      simpa using this
    have hv' : w = v' := by
      have := h2.2
      simpa using this
    exact hne (hv ▸ hv')
  | error err =>
    rw [hr] at h1
    cases h1.2

/-- A positive `okIs` count locates a genuine `(k, .ok v)` element. -/
theorem mem_of_okIs_pos {l : List (κ × Except E V)} {k : κ} {v : V}
    (h : 1 ≤ l.countP (okIs (E := E) k v)) : (k, Except.ok v) ∈ l := by
  obtain ⟨e, he, hpe⟩ := List.countP_pos_iff.mp h
  unfold okIs at hpe
  rw [Bool.and_eq_true] at hpe
  obtain ⟨e1, e2⟩ := e
  cases hr : e2 with
  | ok w =>
    rw [hr] at hpe
    have h1 : e1 = k := by simpa using hpe.1
    have h2 : w = v := by
      have := hpe.2
      rw [show ((e1, Except.ok w).2 : Except E V) = Except.ok w from rfl] at this
      simpa using this
    rw [← h1, ← h2]
    rw [hr] at he
    exact he
  | error err =>
    rw [hr] at hpe
    cases hpe.2

/-- **Face: collected payload lists come from distinct providers** — the
`collect_quorum_with_response` provider extraction: a payload list `L`
covered (with multiplicity) by the emissions at key `k` is contributed by
≥ `L.length` distinct members, each of whose streams contains `(k, .ok v)`
for some `v ∈ L`. Input requirement: ≤ 1 success per member per key. -/
theorem collect_quorum_with_response_providers
    {responses : Fin n → Stream (κ × Except E V)} {min max : Nat}
    {nondet : List (List (κ × Except E V))} {k : κ} (hmin : 1 ≤ min)
    (L : List V)
    (hL : ∀ v : V, L.countP (fun w => decide (w = v))
      ≤ outCnt k v (collect_quorum_with_response responses min max nondet).1)
    (hcap : ∀ j : Fin n, (responses j).countP (okKey k) ≤ 1) :
    ∃ S : List (Fin n), S.Nodup ∧ L.length ≤ S.length ∧
      ∀ j ∈ S, ∃ v ∈ L, (k, Except.ok v) ∈ responses j := by
  -- per-member per-value capacities within the members' own streams
  let cap : Fin n → V → Nat := fun j v =>
    (responses j).countP (okIs (E := E) k v)
  have hcap2 : ∀ j v, cap j v ≤ 1 := by
    intro j v
    refine Nat.le_trans ?_ (hcap j)
    exact List.countP_mono_left (fun e _ he => okIs_imp_okKey k v e he)
  have hcap1 : ∀ j v v', 1 ≤ cap j v → 1 ≤ cap j v' → v = v' := by
    intro j v v' hv hv'
    refine Classical.byContradiction fun hne => ?_
    have hdisj := countP_disjoint_le (responses j) (okIs (E := E) k v)
      (okIs (E := E) k v') (okKey k) (fun e => okIs_disjoint k hne e)
      (fun e => okIs_imp_okKey k v e) (fun e => okIs_imp_okKey k v' e)
    have := hcap j
    have hv₁ : 1 ≤ (responses j).countP (okIs (E := E) k v) := hv
    have hv₂ : 1 ≤ (responses j).countP (okIs (E := E) k v') := hv'
    omega
  have hcount : ∀ v, L.count v
      ≤ ((List.finRange n).map (fun j => cap j v)).sum := by
    intro v
    refine Nat.le_trans (hL v) ?_
    refine Nat.le_trans (wrRun_outCnt_le hmin _ k v) ?_
    exact batchC_countP_le_sum nondet (okIs (E := E) k v)
  obtain ⟨S, hnd, hlen, hprov⟩ := providers_extract L cap hcap1 hcap2 hcount
  refine ⟨S, hnd, hlen, fun j hj => ?_⟩
  obtain ⟨v, hv, hcapv⟩ := hprov j hj
  exact ⟨v, hv, mem_of_okIs_pos hcapv⟩

/-- Emitted payloads are genuine inputs (run-level `wrRun` membership). -/
theorem wrRun_emit_mem {min max : Nat} {ts : List (Stream (κ × Except E V))}
    {k : κ} {v : V}
    (h : (k, v) ∈ allTicks ((collectQuorumWRTick min max).run ts).2) :
    (k, Except.ok v) ∈ ts.flatten := by
  suffices hgen : ∀ (consumed : Stream (κ × Except E V))
      (s : QuorumWRState κ V E), (∀ x ∈ s.notAll, x ∈ consumed) →
      (k, v) ∈ allTicks ((collectQuorumWRTick min max).runFrom s ts).2 →
      (k, Except.ok v) ∈ consumed ++ ts.flatten by
    have := hgen [] ⟨[], []⟩ (fun _ hx => nomatch hx) h
    simpa using this
  clear h
  induction ts with
  | nil => intro _ _ _ h; cases h
  | cons b bs ih =>
    intro consumed s hsub h
    rw [TickLoop.runFrom_cons] at h
    rcases List.mem_append.mp (h : (k, v) ∈ _ ++ allTicks _) with hhead | htail
    · have := wrStep_emit_mem_chain hhead
      rcases List.mem_append.mp this with hs | hb
      · exact List.mem_append_left _ (hsub _ hs)
      · rw [List.flatten_cons, ← List.append_assoc]
        exact List.mem_append_left _ (List.mem_append_right _ hb)
    · have := ih (consumed ++ b) _ (fun x hx =>
        match List.mem_append.mp (wrStep_notAll_mem_chain hx) with
        | .inl hs => List.mem_append_left _ (hsub _ hs)
        | .inr hb => List.mem_append_right _ hb) htail
      rwa [List.flatten_cons, ← List.append_assoc]

/-- **Face: emitted payloads are genuine member responses** (provenance;
no contract hypothesis — quorums are never fabricated even off-contract). -/
theorem collect_quorum_with_response_mem
    {responses : Fin n → Stream (κ × Except E V)} {min max : Nat}
    {nondet : List (List (κ × Except E V))} {k : κ} {v : V}
    (hkv : (k, v) ∈ (collect_quorum_with_response responses min max nondet).1) :
    ∃ j : Fin n, (k, Except.ok v) ∈ responses j := by
  have hmem := wrRun_emit_mem (min := min) (max := max) hkv
  exact unionF_mem (batchC_mem hmem)

end Faces

/-! ## The module's full-run face: `QuorumSpec` (dissertation goal (a))

The unbounded-correctness result of `Programs/CollectQuorumProof.lean`,
restated at this module's boundary: **inputs** = the response contract
(`AtMostMaxResponses`) and a complete consumption decision (`Consumes`);
**output** = the emitted keys are exactly the quorum-qualified keys, once
each (`QuorumSpec`). -/

/-- `QuorumSpec` only sees the input through counts, so it transports along
input permutations (the `NoOrder` quotient). -/
theorem QuorumSpec_perm [DecidableEq E] {min : Nat}
    {input input' : Stream (κ × Except E Unit)} (hp : input.Perm input')
    {emitted : Stream κ} (h : QuorumSpec min input emitted) :
    QuorumSpec min input' emitted := by
  refine ⟨h.1, fun k => ?_⟩
  rw [h.2 k]
  constructor
  · rintro ⟨hmem, hcnt⟩
    refine ⟨(hp.map Prod.fst).mem_iff.mp hmem, ?_⟩
    rwa [Stream.countKeyP_perm hp] at hcnt
  · rintro ⟨hmem, hcnt⟩
    refine ⟨(hp.map Prod.fst).mem_iff.mpr hmem, ?_⟩
    rwa [Stream.countKeyP_perm hp.symm] at hcnt

/-- **Verified face of `collect_quorum`** over the member-family run: for
every complete batching decision of the fan-in union, under the response
contract, the success output satisfies `QuorumSpec` — each quorum-qualified
key exactly once, nothing else. (Instantiates `collectQuorum_correct`, goal
(a), at the typed stage.) -/
theorem collect_quorum_spec {κ E : Type} [DecidableEq κ] [DecidableEq E]
    {n : Nat} {min max : Nat}
    (hmin : 1 ≤ min) (hmm : min ≤ max)
    (responses : Fin n → Stream (κ × Except E Unit))
    {d : List (List (κ × Except E Unit))}
    (hd : Consumes (unionF responses) d)
    (hmax : AtMostMaxResponses max (unionF responses)) :
    QuorumSpec min (unionF responses)
      (collect_quorum responses min max d).1 := by
  have hout : (collect_quorum responses min max d).1
      = (collectQuorumTick (κ := κ) (E := E) min max).allTicksOutput d := by
    have hdef : (collect_quorum responses min max d).1
        = ((collectQuorumTick (κ := κ) (E := E) min max).outputs
            (batchC (unionF responses) [] d)).flatten := rfl
    rw [hdef, hd.batchC_eq]
    rfl
  have hmax' : AtMostMaxResponses max d.flatten := by
    intro k
    show Stream.countKey d.flatten k ≤ max
    calc Stream.countKey d.flatten k
        = Stream.countKey (unionF responses) k := hd.countP_eq _
      _ ≤ max := hmax k
  have h := collectQuorum_correct κ E inferInstance min max hmin hmm
    d.flatten d rfl hmax'
  rw [hout]
  exact QuorumSpec_perm hd h


end HydroLean.Programs
