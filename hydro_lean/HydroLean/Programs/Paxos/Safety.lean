import HydroLean.Programs.Paxos.PaxosCore

/-!
# Paxos safety (K4): the final assembly

The headline `commit_agreement` for the **guarded** variant, assembled
purely from the module **contracts** — `sequence_payload`'s abstract
witnesses `SPEmission`/`SPChosen` with `sp_commit_spec` /
`sp_emission_spec` / `sp_log_entry_spec` / `sp_emission_functional`, and
`leader_election`'s leader-view contracts — instantiated at the run by the
wiring layer (`PaxosCore.lean`: `pcG`, `run_discipline`,
`run_leader_providers`, `run_promise_covers`, `alog_succ`,
`run_emission_zero`). This file never opens a module's internals and never
names an internal wire: every step consumes a contract at the signature.

Named proof inputs of the verified face:
- the client callback is typed `→ₘ` (a Hydro program of the ballot streams
  is prefix-monotone by construction — carried by the type, not a
  hypothesis);
- `nA ≤ 2 * f + 1` — quorum intersection is possible.

Everything else — key uniqueness (paxos.rs:862's missing `assume`), the B1
send-once and B2 recommit-once contracts, and the paxos.rs:186–189
leader-ballot stability (the module contract `le_ballot_stable`: the
election trigger reads `!p_is_leader` through the `forward_ref` cycle, so
a fabricated reign cannot bootstrap) — is **derived** from the guarded
variant, not assumed. Per ACCEPTANCE §3, this file holds only the final
assembly; every counting/provenance fact lives in the module that owns the
data, exported as a contract on its signature.
-/

namespace HydroLean.Programs.Paxos

open HydroLean.Hydro
open HydroLean.Programs

set_option synthInstance.maxSize 1024

variable {P : Type} {nP nA : Nat} [DecidableEq P]

/-- The guarded variant, fixed for the safety statement. -/
local notation "G" => PaxosVariant.guarded

section Assembly

variable (f : Nat)
variable (cbM : (Fin nP → Stream (Ballot nP)) →ₘ (Fin nP → Stream P))
variable (nd : PaxosNondet P nP nA)

set_option maxHeartbeats 1000000 in
/-- **K4 — the ONE protocol induction** (`Nat`-induction over the unfolding
depth; the provenance regress steps `k + 1 → k` through the `a_log` knot):
any emission at a slot chosen at a strictly lower ballot carries the chosen
key's value. Every step consumes a named module contract at the run. -/
theorem emission_chosen_agree
    (hnA : nA ≤ 2 * f + 1) (K : Nat) (slot : Nat)
    {i₁ : Fin nP} {b₁ : Ballot nP} {val₁ : Option P}
    (hb₁own : b₁.proposerId = i₁)
    (hm₁ : SPEmission G f nd.sp (pcG f cbM nd K) i₁ slot b₁ val₁)
    (hch : SPChosen f nd.sp (pcG f cbM nd K) i₁ slot b₁) :
    ∀ (k : Nat) (i₂ : Fin nP) (b₂ : Ballot nP) (v₂ : Option P),
      b₁.blt b₂ = true →
      SPEmission G f nd.sp (pcG f cbM nd k) i₂ slot b₂ v₂ →
      v₂ = val₁ := by
  intro k
  induction k with
  | zero =>
    intro i₂ b₂ v₂ hlt hm₂
    exact absurd hm₂ (fun hc => run_emission_zero f cbM nd hc)
  | succ k ih =>
    intro i₂ b₂ v₂ hlt hm₂
    -- the emission's leader tick, ballot, and covered-value clause
    obtain ⟨t, hpl, hpb, hpr, hl, hbal, hcovd⟩ :=
      sp_emission_spec f nd.sp
        (run_discipline f cbM nd (pcHist G f cbM nd (k + 1))) hm₂
    -- `f + 1` distinct promisers at the same tick
    obtain ⟨hpr', hpb', S, hSnd, hSlen, hSprov⟩ :=
      run_leader_providers f cbM nd (k + 1) i₂ hpl hl
    -- `f + 1` distinct voters for the chosen key
    obtain ⟨C, hCnd, hClen, hCvote⟩ := hch
    -- quorum intersection
    obtain ⟨j, hjS, hjC⟩ := nodup_inter_of_length hSnd hCnd (by omega)
    obtain ⟨vj, hvj, tj, htj, hpay, hmj, hmax⟩ := hSprov j hjS
    obtain ⟨t', hta, htl, hama, hcov⟩ := hCvote j hjC
    -- the promise carries the emission's ballot on the shared max wire
    have hbal' : ((pcG f cbM nd (k + 1)).2.1 i₂)[t]'hpb' = b₂ := hbal
    rw [hbal'] at hmax
    -- K1: the promise payload covers the chosen key
    have hcovj := run_promise_covers f cbM nd hlt htj ⟨hmj, hmax⟩
      hta hama htl hcov
    obtain ⟨e₀, hfind, he₀⟩ := hcovj
    -- the covering entry lives in the emission tick's view
    have hpaymem : (slot, e₀) ∈ (vj.2 : LogMap P nP) := by
      rw [hpay]
      exact LogMap.find?_mem hfind
    have hvj' : vj ∈ ((pcG f cbM nd (k + 1)).2.2.2.1 i₂)[t]'hpr := hvj
    have hE : (slot, e₀)
        ∈ ((((pcG f cbM nd (k + 1)).2.2.2.1 i₂)[t]'hpr).map
            Prod.snd).flatten :=
      List.mem_flatten.mpr ⟨vj.2, List.mem_map.mpr ⟨vj, hvj', rfl⟩, hpaymem⟩
    -- the emission's value is the view's merged max-ballot entry
    obtain ⟨best, hval, hdom, hbest⟩ := hcovd e₀ hE
    have hb₁best : b₁.ble best.ballot = true := by
      rcases he₀ with heq₀ | hlt₀
      · rw [← heq₀]
        exact hdom
      · exact Ballot.ble_trans (Ballot.ble_of_blt hlt₀) hdom
    -- the merged entry's carrier is a promise payload; regress to index `k`
    obtain ⟨l', hl', hbestl⟩ := List.mem_flatten.mp hbest
    obtain ⟨v', hv'mem, rfl⟩ := List.mem_map.mp hl'
    obtain ⟨hprv, hpbv, -, hprom⟩ :=
      run_leader_view_promise f cbM nd (k + 1) i₂ hpl hl
    have hv'mem' : v' ∈ ((pcG f cbM nd (k + 1)).2.2.2.1 i₂)[t]'hprv :=
      hv'mem
    obtain ⟨j', tj', htj', hpay', -⟩ := hprom v' hv'mem'
    -- the promise payload is the previous index's published log (the knot)
    have htlk : tj' < ((spOut G f nd.sp
        (pcG f cbM nd k)).2.1 j').vals.length := by
      have := htj'
      rwa [alog_succ] at this
    have hEk : (slot, best)
        ∈ (((((spOut G f nd.sp (pcG f cbM nd k)).2.1
            j').vals[tj']'htlk).2) : LogMap P nP) := by
      have hbl : (slot, best) ∈ (v'.2 : LogMap P nP) := hbestl
      rw [hpay'] at hbl
      have hgeq : (((pcHist G f cbM nd (k + 1)).a_log j')[tj']'htj')
          = ((spOut G f nd.sp (pcG f cbM nd k)).2.1 j').vals[tj']'htlk := by
        rw [List.getElem_of_eq (alog_succ (variant := G) (f := f)
          (cbM := cbM) (nd := nd) k j') htj']
      rw [← hgeq]
      exact hbl
    have hm₃ := sp_log_entry_spec f nd.sp
      (run_discipline f cbM nd (pcHist G f cbM nd k)) htlk hEk
    -- dichotomy at the merged entry's ballot
    rcases Ballot.eq_or_blt_of_ble hb₁best with hbeq | hblt
    · -- the entry is the chosen ballot's own emission: functionality
      rw [← hbeq, hb₁own] at hm₃
      have hm₃M := SPEmission.mono G f nd.sp
        (pcG_le f cbM nd (Nat.le_max_left k K)) hm₃
      have hm₁M := SPEmission.mono G f nd.sp
        (pcG_le f cbM nd (Nat.le_max_right k K)) hm₁
      have hbv := sp_emission_functional f nd.sp
        (run_discipline f cbM nd (pcHist G f cbM nd (max k K)))
        (pcG_le f cbM nd (Nat.le_refl _)) hm₃M hm₁M
      rw [hval, hbv]
    · -- strictly higher entry ballot: the induction hypothesis at `k`
      have := ih best.ballot.proposerId best.ballot best.value hblt hm₃
      rw [hval, this]

/-- **THE HEADLINE — CommitAgreement / slot-functionality of
`p_to_replicas`** (deliverable (c)): for the guarded variant, over the
**full** decision space (every batching/snapshot/shuffle decision, every
client payload timing, every cycle-unfolding depth in `nd`), any two
commits at one slot — across any two proposers — carry the same value.

Verified face of `paxos_core` (proof inputs):
- the client callback `cbM` is typed `→ₘ` — prefix-monotonicity is carried
  by the type (automatic for every combinator-built callback), not stated
  as a hypothesis;
- `nA ≤ 2 * f + 1` — the acceptor cluster is small enough for `f + 1`
  quorums to intersect.

The B1 send-once and B2 recommit-once guards, the paxos.rs:862 key
uniqueness, and the paxos.rs:186–189 leader-ballot stability
(`le_ballot_stable`) are all **derived** (they are the guarded variant
plus the election protocol's own trigger gate), not assumed. The faithful
variant falsifies this statement (`Paxos/Falsification.lean`). -/
theorem commit_agreement
    (hnA : nA ≤ 2 * f + 1)
    {i i' : Fin nP} {slot : Nat} {v v' : Option P}
    (h : (slot, v) ∈ (paxos_core G f cbM nd).2 i)
    (h' : (slot, v') ∈ (paxos_core G f cbM nd).2 i') :
    v = v' := by
  rw [paxos_core_out_eq f cbM nd i] at h
  rw [paxos_core_out_eq f cbM nd i'] at h'
  obtain ⟨b, hbo, hem, hch⟩ := sp_commit_spec f nd.sp
    (run_discipline f cbM nd (pcHist G f cbM nd nd.fuel)) h
  obtain ⟨b', hbo', hem', hch'⟩ := sp_commit_spec f nd.sp
    (run_discipline f cbM nd (pcHist G f cbM nd nd.fuel)) h'
  by_cases hbb : b.blt b' = true
  · -- b < b': the higher emission over the `b`-chosen slot
    have hagree := emission_chosen_agree f cbM nd hnA nd.fuel slot
      hbo hem hch nd.fuel i' b' v' hbb hem'
    exact hagree.symm
  · by_cases hbb' : b'.blt b = true
    · -- b' < b: symmetric
      exact emission_chosen_agree f cbM nd hnA nd.fuel slot
        hbo' hem' hch' nd.fuel i b v hbb' hem
    · -- equal ballots: same proposer, same key ⇒ the same emission
      have hble : b'.ble b = true :=
        Ballot.ble_of_not_blt (fun hc => hbb hc)
      have heq : b' = b := by
        rcases Ballot.eq_or_blt_of_ble hble with heq | hblt
        · exact heq
        · exact absurd hblt (fun hc => hbb' hc)
      subst heq
      have hii : i' = i := by rw [← hbo, ← hbo']
      subst hii
      exact sp_emission_functional f nd.sp
        (run_discipline f cbM nd (pcHist G f cbM nd nd.fuel))
        (pcG_le f cbM nd (Nat.le_refl _)) hem hem'

end Assembly

end HydroLean.Programs.Paxos
