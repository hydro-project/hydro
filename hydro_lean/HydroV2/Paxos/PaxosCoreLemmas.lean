import HydroV2.Paxos.LeaderElection
import HydroV2.Paxos.SequencePayload
import Mathlib.Data.Fintype.Card

/-!
# `paxos_core` · the agreement layer (K4)

The `a_log` knot's stage vocabulary — the Kleene iterates at a fixed
received-max wire, and the election/sequencing runs at each depth — and
K4, the ONE protocol induction: the provenance regress steps `k+1 → k`
through the knot. `paxos_core`'s colocated headline (in
`PaxosCore.lean`) is one application of `paxos_core_agree` at its own
knot wires.
-/

namespace HydroV2

variable {L : Type} {mem : L → Nat} {P : Type} [DecidableEq P]

private theorem nodup_inter_of_length {n : Nat} {S C : List (Fin n)}
    (hS : S.Nodup) (hC : C.Nodup) (h : n < S.length + C.length) :
    ∃ j, j ∈ S ∧ j ∈ C := by
  by_contra habs
  push_neg at habs
  have hdisj : S.Disjoint C := fun {a} ha hb => habs a ha hb
  have hnd : (S ++ C).Nodup := List.Nodup.append hS hC hdisj
  have hle : (S ++ C).length ≤ n := by
    have h2 := List.Nodup.length_le_card hnd
    simpa using h2
  rw [List.length_append] at hle
  omega

/-- Per-slot agreement of the replica stream (the safety face): any two
committed values at the same slot agree, across any two proposers. -/
def SlotFunctional {n : Nat} (out : Fin n → Multiset (Nat × Option P)) :
    Prop :=
  ∀ (i j : Fin n) (s : Nat) (v w : Option P),
    (s, v) ∈ out i → (s, w) ∈ out j → v = w

section Agreement

variable (prop acc : L) (f : Nat)
variable (cp : (Values L mem).Stream prop P .totalOrder .exactlyOnce)
variable (ck : (Values L mem).TickSingleton acc (Option Nat) .unbounded)
variable (ledec : LEDec (mem prop) (mem acc) P)
variable (spdec : SPDec (mem prop) (mem acc) P)
variable (fuelALog : UnfoldFuel)
variable (sm : (Values L mem).Stream prop (Ballot (mem prop))
  .noOrder .exactlyOnce)

/-- One `a_log` knot step: the published log of the sequencing run over
the election run at the given `a_log` wire (the loop body's `.2.2.1`
projection, spelled at the module boundary). -/
def pcAlogStep (al : TickV (mem acc) (ALog P (mem prop)) .unbounded) :
    TickV (mem acc) (ALog P (mem prop)) .unbounded :=
  (sequence_payload (Values L mem) .guarded prop acc cp ck
    ((Values L mem).forgetBound ((leader_election (Values L mem) .guarded
      prop acc (f + 1) ledec 0 1 2 3 10 11 12).val sm al).1)
    ((leader_election (Values L mem) .guarded prop acc (f + 1)
      ledec 0 1 2 3 10 11 12).val sm al).2.1
    ((leader_election (Values L mem) .guarded prop acc (f + 1)
      ledec 0 1 2 3 10 11 12).val sm al).2.2.1 f
    ((Values L mem).forgetBound ((leader_election (Values L mem) .guarded
      prop acc (f + 1) ledec 0 1 2 3 10 11 12).val sm al).2.2.2)
    spdec 4 5).val.2.1

/-- The `a_log` knot stages (Kleene iterates from the empty wire). -/
def pcAlogW : Nat → TickV (mem acc) (ALog P (mem prop)) .unbounded :=
  fun m => iterate (pcAlogStep prop acc f cp ck ledec spdec sm)
    (fun _ => []) m

/-- The election run at stage `m` (the raw output tuple; the contracts
come from `leader_election`'s colocated clause). -/
def pcLE (m : Nat) :=
  (leader_election (Values L mem) .guarded prop acc (f + 1)
    ledec 0 1 2 3 10 11 12).val sm
    (pcAlogW prop acc f cp ck ledec spdec sm m)

/-- The sequencing run at stage `m`. -/
def pcSP (m : Nat) :=
  sequence_payload (Values L mem) .guarded prop acc cp ck
    ((Values L mem).forgetBound (pcLE prop acc f cp ck ledec spdec sm m).1)
    (pcLE prop acc f cp ck ledec spdec sm m).2.1
    (pcLE prop acc f cp ck ledec spdec sm m).2.2.1 f
    ((Values L mem).forgetBound
      (pcLE prop acc f cp ck ledec spdec sm m).2.2.2)
    spdec 4 5

set_option maxRecDepth 65536 in
/-- The knot, opened once: the next stage's `a_log` IS this stage's
published log (`snapshot_atomic`'s write-before-ack, staged). -/
theorem pcAlogW_succ (m : Nat) :
    pcAlogW prop acc f cp ck ledec spdec sm (m + 1)
      = (pcSP prop acc f cp ck ledec spdec sm m).val.2.1 := rfl

theorem pcAlogW_zero : pcAlogW prop acc f cp ck ledec spdec sm 0
    = fun _ => [] := rfl

set_option maxHeartbeats 3200000 in
set_option maxRecDepth 8192 in
/-- **K4 — slot functionality of the guarded knot** (any received-max
wire, any decisions, intersecting quorums): any two commits at one
slot, across any two proposers, carry the same value. -/
theorem paxos_core_agree (hnA : mem acc ≤ 2 * f + 1) :
    SlotFunctional
      ((pcSP prop acc f cp ck ledec spdec sm fuelALog).val.1) := by
  have hknot : ∀ m, pcAlogW prop acc f cp ck ledec spdec sm (m + 1)
      = (pcSP prop acc f cp ck ledec spdec sm m).val.2.1 :=
    fun m => pcAlogW_succ prop acc f cp ck ledec spdec sm m
  -- the stage contracts
  have hle : ∀ m, LEEnsures .guarded prop acc (f + 1)
      (pcAlogW prop acc f cp ck ledec spdec sm m)
      (pcLE prop acc f cp ck ledec spdec sm m) := fun m =>
    ((leader_election (Values L mem) .guarded prop acc (f + 1)
      ledec 0 1 2 3 10 11 12).property rfl).1 sm
      (pcAlogW prop acc f cp ck ledec spdec sm m)
  have hsp := fun m => (pcSP prop acc f cp ck ledec spdec sm m).property rfl
  have hreq : ∀ m, SPRequires (mem prop) P
      (fun i => ((pcLE prop acc f cp ck ledec spdec sm m).1 i).vals) ((pcLE prop acc f cp ck ledec spdec sm m).2.1)
      ((pcLE prop acc f cp ck ledec spdec sm m).2.2.1) := fun m =>
    { own := (hle m).own
      mono := fun i {t t'} h ht' => ((pcLE prop acc f cp ck ledec spdec sm m).1 i).ascending h ht'
      lead_ne := fun i {t} hpl hpr hl =>
        (hle m).lead_ne (by omega) i hpl hpr hl
      stable := fun i {t} ht1 hb1 h1 h0 =>
        (hle m).stable (by omega) i ht1 hb1 h1 h0
      pinned := fun i {t t'} hpl hpl' hpr hpr' hpb hpb' h1 h2 h3 =>
        (hle m).pinned (by omega) i hpl hpl' hpr hpr' hpb hpb'
          h1 h2 h3 }
  -- stage monotonicity of the knot wire (Kleene chain)
  have halog_step : ∀ (al al' : TickV (mem acc)
      (ALog P (mem prop)) .unbounded), (∀ j, al j <+: al' j) →
      ∀ j, pcAlogStep prop acc f cp ck ledec spdec sm al j
        <+: pcAlogStep prop acc f cp ck ledec spdec sm al' j := by
    intro al al' hal
    have hLE := ((leader_election (Values L mem) .guarded prop acc
      (f + 1) ledec 0 1 2 3 10 11 12).property rfl).2
      sm sm al al' (fun _ => le_refl _) hal
    exact (sequence_payload_mono .guarded prop acc _ _ _ _ _ _ _ _
      _ _ f _ _ spdec 4 5 (fun _ => List.prefix_refl _)
      (fun _ => List.prefix_refl _) (fun i => hLE.1 i)
      (fun i => hLE.2.1 i) (fun i => hLE.2.2.1 i)
      (fun j => hLE.2.2.2 j)).2.1
  have halog : ∀ {m m' : Nat}, m ≤ m' →
      ∀ j, pcAlogW prop acc f cp ck ledec spdec sm m j <+: pcAlogW prop acc f cp ck ledec spdec sm m' j := by
    intro m m' hmm'
    exact iterate_chain (R := fun (a b : TickV (mem acc)
        (ALog P (mem prop)) .unbounded) => ∀ j, a j <+: b j)
      (fun a j => List.prefix_refl _)
      (fun {a b c} h1 h2 j => (h1 j).trans (h2 j))
      (fun j => List.nil_prefix)
      (fun {a b} hab => halog_step a b hab) hmm'
  have hleM : ∀ {m m' : Nat}, m ≤ m' →
      (∀ i, ((pcLE prop acc f cp ck ledec spdec sm m).1 i).vals
        <+: ((pcLE prop acc f cp ck ledec spdec sm m').1 i).vals)
      ∧ (∀ i, (pcLE prop acc f cp ck ledec spdec sm m).2.1 i
        <+: (pcLE prop acc f cp ck ledec spdec sm m').2.1 i)
      ∧ (∀ i, (pcLE prop acc f cp ck ledec spdec sm m).2.2.1 i
        <+: (pcLE prop acc f cp ck ledec spdec sm m').2.2.1 i)
      ∧ (∀ j, ((pcLE prop acc f cp ck ledec spdec sm m).2.2.2 j).vals
        <+: ((pcLE prop acc f cp ck ledec spdec sm m').2.2.2 j).vals) :=
    fun {m m'} hmm' =>
    ((leader_election (Values L mem) .guarded prop acc (f + 1)
      ledec 0 1 2 3 10 11 12).property rfl).2 sm sm
      (pcAlogW prop acc f cp ck ledec spdec sm m)
      (pcAlogW prop acc f cp ck ledec spdec sm m')
      (fun _ => le_refl _) (halog hmm')
  -- emissions persist along the stages
  have hemM : ∀ {m m' : Nat}, m ≤ m' →
      ∀ {i₀ : Fin (mem prop)} {slot₀ : Nat} {b₀ : Ballot (mem prop)}
        {v₀ : Option P},
      SPEmission .guarded f (cp i₀)
        (spdec.payloadBatch i₀) (((pcLE prop acc f cp ck ledec spdec sm m).1 i₀).vals)
        ((pcLE prop acc f cp ck ledec spdec sm m).2.1 i₀) ((pcLE prop acc f cp ck ledec spdec sm m).2.2.1 i₀) slot₀ b₀ v₀ →
      SPEmission .guarded f (cp i₀)
        (spdec.payloadBatch i₀) (((pcLE prop acc f cp ck ledec spdec sm m').1 i₀).vals)
        ((pcLE prop acc f cp ck ledec spdec sm m').2.1 i₀) ((pcLE prop acc f cp ck ledec spdec sm m').2.2.1 i₀) slot₀ b₀ v₀ := by
    intro m m' hmm' i₀ slot₀ b₀ v₀ hem
    have hpre := spSentTrace_prefix .guarded f
      (spdec.payloadBatch i₀) (List.prefix_refl (cp i₀))
      ((hleM hmm').1 i₀) ((hleM hmm').2.1 i₀) ((hleM hmm').2.2.1 i₀)
    obtain ⟨ch, hch, hin⟩ := List.mem_flatten.mp hem
    exact List.mem_flatten.mpr ⟨ch, hpre.subset hch, hin⟩
  -- same-stage same-key emissions carry one value (send-once)
  have hfun : ∀ (m : Nat) (o : Fin (mem prop)) {slot₀ : Nat}
      {b₀ : Ballot (mem prop)} {va vb : Option P},
      SPEmission .guarded f (cp o)
        (spdec.payloadBatch o) (((pcLE prop acc f cp ck ledec spdec sm m).1 o).vals)
        ((pcLE prop acc f cp ck ledec spdec sm m).2.1 o) ((pcLE prop acc f cp ck ledec spdec sm m).2.2.1 o) slot₀ b₀ va →
      SPEmission .guarded f (cp o)
        (spdec.payloadBatch o) (((pcLE prop acc f cp ck ledec spdec sm m).1 o).vals)
        ((pcLE prop acc f cp ck ledec spdec sm m).2.1 o) ((pcLE prop acc f cp ck ledec spdec sm m).2.2.1 o) slot₀ b₀ vb →
      va = vb := by
    intro m o slot₀ b₀ va vb ha hb
    have hnd := spSentTrace_key_nodup f (cp o)
      (spdec.payloadBatch o) o _ _ _ ((hreq m).own o)
      (fun h ht' => (hreq m).mono o h ht')
      (fun ht1 hb1 h1 h0 => (hreq m).stable o ht1 hb1 h1 h0)
      (fun hpl hpr hl => (hreq m).lead_ne o hpl hpr hl)
    have := nodup_keys_inj hnd ha hb rfl
    exact congrArg Prod.snd this
  -- the zero stage has no emissions (no acceptor has ever ticked)
  have hzero : ∀ {i₀ : Fin (mem prop)} {slot₀ : Nat}
      {b₀ : Ballot (mem prop)} {v₀ : Option P},
      SPEmission .guarded f (cp i₀)
        (spdec.payloadBatch i₀) (((pcLE prop acc f cp ck ledec spdec sm 0).1 i₀).vals)
        ((pcLE prop acc f cp ck ledec spdec sm 0).2.1 i₀) ((pcLE prop acc f cp ck ledec spdec sm 0).2.2.1 i₀) slot₀ b₀ v₀ →
      False := by
    intro i₀ slot₀ b₀ v₀ hem
    obtain ⟨t, hpl, hpb, hgv, hl, -, -⟩ :=
      spSentTrace_open .guarded f _ _ _ _ _ hem
    obtain ⟨hpr2, hpb2, hcard, hprom⟩ := (hle 0).view_promise i₀ hpl hl
    have hpos : ∃ v0, v0 ∈ ((pcLE prop acc f cp ck ledec spdec sm 0).2.2.1 i₀)[t]'hpr2 := by
      refine Multiset.card_pos_iff_exists_mem.mp ?_
      omega
    obtain ⟨v0, hv0⟩ := hpos
    obtain ⟨j, tj, htj, -⟩ := hprom v0 hv0
    have hnil : pcAlogW prop acc f cp ck ledec spdec sm 0 j = [] := by
      rw [pcAlogW_zero]
    rw [hnil] at htj
    exact absurd htj (by simp)
  -- K4 — the protocol induction: an emission over a chosen slot at a
  -- strictly lower ballot carries the chosen value; the provenance
  -- regress steps k+1 → k through the `a_log` knot
  have hagree : ∀ (K : Nat) (slot₀ : Nat) (i₁ : Fin (mem prop))
      (b₁ : Ballot (mem prop)) (val₁ : Option P),
      b₁.proposerId = i₁ →
      SPEmission .guarded f (cp i₁)
        (spdec.payloadBatch i₁) (((pcLE prop acc f cp ck ledec spdec sm K).1 i₁).vals)
        ((pcLE prop acc f cp ck ledec spdec sm K).2.1 i₁) ((pcLE prop acc f cp ck ledec spdec sm K).2.2.1 i₁) slot₀ b₁ val₁ →
      SPChosen f (fun j => ck j)
        (fun j => ((pcLE prop acc f cp ck ledec spdec sm K).2.2.2 j).vals) ((pcSP prop acc f cp ck ledec spdec sm K).val.2.1)
        slot₀ b₁ →
      ∀ (k : Nat) (i₂ : Fin (mem prop)) (b₂ : Ballot (mem prop))
        (v₂ : Option P), b₁.blt b₂ = true →
        SPEmission .guarded f (cp i₂)
          (spdec.payloadBatch i₂) (((pcLE prop acc f cp ck ledec spdec sm k).1 i₂).vals)
          ((pcLE prop acc f cp ck ledec spdec sm k).2.1 i₂) ((pcLE prop acc f cp ck ledec spdec sm k).2.2.1 i₂) slot₀ b₂ v₂ →
        v₂ = val₁ := by
    intro K slot₀ i₁ b₁ val₁ hb₁own hm₁ hch k
    induction k with
    | zero =>
      intro i₂ b₂ v₂ hlt hm₂
      exact absurd hm₂ hzero
    | succ k ih =>
      intro i₂ b₂ v₂ hlt hm₂
      -- the emission's leader tick and its input-view characterization
      obtain ⟨t, hpl, hpb, hpr, hl, hbal, hcovd⟩ :=
        spSentTrace_open_input f (cp i₂)
          (spdec.payloadBatch i₂) i₂ _ _ _
          ((hreq (k + 1)).own i₂)
          (fun h ht' => (hreq (k + 1)).mono i₂ h ht')
          (fun ht1 hb1 h1 h0 =>
            (hreq (k + 1)).stable i₂ ht1 hb1 h1 h0)
          (fun hpl hpr hll => (hreq (k + 1)).lead_ne i₂ hpl hpr hll)
          (fun hpl hpl' hpr hpr' hpb hpb' h1 h2 h3 =>
            (hreq (k + 1)).pinned i₂ hpl hpl' hpr hpr' hpb hpb'
              h1 h2 h3)
          hm₂
      -- f + 1 distinct promisers at the emission tick
      obtain ⟨hpr', hpb', S, hSnd, hSlen, hSprov⟩ :=
        (hle (k + 1)).providers rfl (by omega) i₂ hpl hl
      -- f + 1 distinct voters for the chosen key
      obtain ⟨C, hCnd, hClen, hCvote⟩ := hch
      -- quorum intersection
      obtain ⟨j, hjS, hjC⟩ := nodup_inter_of_length hSnd hCnd
        (by omega)
      obtain ⟨vj, hvj, tj, htj, hpay, hmj, hmax⟩ := hSprov j hjS
      obtain ⟨t₁, hta, hama, hcovck⟩ := hCvote j hjC
      -- the two max-wire facts, lifted to the max stage
      have hkM : k + 1 ≤ max (k + 1) K := Nat.le_max_left _ _
      have hKM : K ≤ max (k + 1) K := Nat.le_max_right _ _
      have hamaxp1 : ((pcLE prop acc f cp ck ledec spdec sm (k + 1)).2.2.2 j).vals
          <+: ((pcLE prop acc f cp ck ledec spdec sm (max (k + 1) K)).2.2.2 j).vals :=
        (hleM hkM).2.2.2 j
      have hamaxpK : ((pcLE prop acc f cp ck ledec spdec sm K).2.2.2 j).vals
          <+: ((pcLE prop acc f cp ck ledec spdec sm (max (k + 1) K)).2.2.2 j).vals :=
        (hleM hKM).2.2.2 j
      have hmjM : tj
          < ((pcLE prop acc f cp ck ledec spdec sm (max (k + 1) K)).2.2.2 j).vals.length :=
        Nat.lt_of_lt_of_le hmj hamaxp1.length_le
      have htaM : t₁
          < ((pcLE prop acc f cp ck ledec spdec sm (max (k + 1) K)).2.2.2 j).vals.length :=
        Nat.lt_of_lt_of_le hta hamaxpK.length_le
      have hmax2 : ((pcLE prop acc f cp ck ledec spdec sm (k + 1)).2.2.2 j).vals[tj]'hmj
          = some b₂ := by
        rw [hmax]
        exact congrArg some hbal
      have hvalj : ((pcLE prop acc f cp ck ledec spdec sm (max (k + 1) K)).2.2.2 j).vals[tj]'hmjM
          = some b₂ := by
        rw [← List.IsPrefix.getElem hamaxp1 hmj]
        exact hmax2
      have hval1 : ((pcLE prop acc f cp ck ledec spdec sm (max (k + 1) K)).2.2.2 j).vals[t₁]'htaM
          = some b₁ := by
        rw [← List.IsPrefix.getElem hamaxpK hta]
        exact hama
      -- the vote precedes the promise on the shared, ascending max wire
      have ht1tj : t₁ < tj := by
        by_contra hle'
        push_neg at hle'
        have hasc := ((pcLE prop acc f cp ck ledec spdec sm (max (k + 1) K)).2.2.2 j).ascending
          hle' htaM
        rw [hvalj, hval1] at hasc
        exact Ballot.ble_blt_asymm hasc hlt
      -- the vote's coverage is realized (the promise saw the log later)
      have hpubk_len : tj < ((pcSP prop acc f cp ck ledec spdec sm k).val.2.1 j).length := by
        have h0 := htj
        rw [hknot k] at h0
        exact h0
      have hcklen : t₁ < (ck j).length := by
        have h2 := (hsp k).log_len_le_ck j
        omega
      obtain ⟨htl₁, hcov₁⟩ := hcovck hcklen
      -- transport the covering value to the max stage and ascend to
      -- the promise tick
      have hpubK : (pcSP prop acc f cp ck ledec spdec sm K).val.2.1 j
          <+: (pcSP prop acc f cp ck ledec spdec sm (max (k + 1) K)).val.2.1 j := by
        have h0 := halog (Nat.succ_le_succ hKM) j
        rw [hknot K, hknot (max (k + 1) K)] at h0
        exact h0
      have hpubk : (pcSP prop acc f cp ck ledec spdec sm k).val.2.1 j
          <+: (pcSP prop acc f cp ck ledec spdec sm (max (k + 1) K)).val.2.1 j := by
        have h0 := halog
          (Nat.succ_le_succ (Nat.le_of_succ_le hkM)) j
        rw [hknot k, hknot (max (k + 1) K)] at h0
        exact h0
      have htlM : t₁ < ((pcSP prop acc f cp ck ledec spdec sm (max (k + 1) K)).val.2.1 j).length :=
        Nat.lt_of_lt_of_le htl₁ hpubK.length_le
      have hcovM : LogCovers
          (((pcSP prop acc f cp ck ledec spdec sm (max (k + 1) K)).val.2.1 j)[t₁]'htlM).2
          slot₀ b₁ := by
        rw [← List.IsPrefix.getElem hpubK htl₁]
        exact hcov₁
      have htjM2 : tj < ((pcSP prop acc f cp ck ledec spdec sm (max (k + 1) K)).val.2.1 j).length :=
        Nat.lt_of_lt_of_le hpubk_len hpubk.length_le
      have hcovtj : LogCovers
          (((pcSP prop acc f cp ck ledec spdec sm (max (k + 1) K)).val.2.1 j)[tj]'htjM2).2
          slot₀ b₁ :=
        (hsp (max (k + 1) K)).log_covers_mono (Nat.le_of_lt ht1tj)
          htjM2 hcovM
      -- back through the knot: the promise payload is the k-stage
      -- published log, i.e. the max-stage value at `tj`
      have hpayk : vj = ((pcSP prop acc f cp ck ledec spdec sm k).val.2.1 j)[tj]'hpubk_len := by
        rw [hpay]
        congr 1
      have hpayeq : vj
          = ((pcSP prop acc f cp ck ledec spdec sm (max (k + 1) K)).val.2.1 j)[tj]'htjM2 := by
        rw [hpayk]
        exact List.IsPrefix.getElem hpubk hpubk_len
      have hcovvj : LogCovers vj.2 slot₀ b₁ := by
        rw [hpayeq]
        exact hcovtj
      obtain ⟨e₀, he₀mem, he₀ble⟩ := hcovvj
      -- the covering entry lives in the emission tick's input view
      have hE : (slot₀, e₀) ∈ rcEntries
          (((pcLE prop acc f cp ck ledec spdec sm (k + 1)).2.2.1 i₂)[t]'hpr) := by
        unfold rcEntries
        refine Multiset.mem_bind.mpr ⟨vj, hvj, ?_⟩
        exact Multiset.mem_coe.mpr he₀mem
      -- the emission's value is the view's champion at the slot
      obtain ⟨best, hbestmem, hveq, hdom⟩ := hcovd e₀ hE
      have hb₁best : b₁.ble best.ballot = true :=
        Ballot.ble_trans he₀ble hdom
      -- every view entry at the champion's key regresses to a stage-k
      -- emission (the knot again, entrywise)
      have hgroup_em : ∀ lv : LogValue P (mem prop),
          (slot₀, lv) ∈ rcEntries
            (((pcLE prop acc f cp ck ledec spdec sm (k + 1)).2.2.1 i₂)[t]'hpr) →
          SPEmission .guarded f
            (cp lv.ballot.proposerId)
            (spdec.payloadBatch lv.ballot.proposerId)
            (((pcLE prop acc f cp ck ledec spdec sm k).1 lv.ballot.proposerId).vals)
            ((pcLE prop acc f cp ck ledec spdec sm k).2.1 lv.ballot.proposerId)
            ((pcLE prop acc f cp ck ledec spdec sm k).2.2.1 lv.ballot.proposerId)
            slot₀ lv.ballot lv.value := by
        intro lv hlv
        unfold rcEntries at hlv
        obtain ⟨vj', hvj', hlv2⟩ := Multiset.mem_bind.mp hlv
        obtain ⟨hprv, hpbv, -, hprom⟩ :=
          (hle (k + 1)).view_promise i₂ hpl hl
        have hvj'2 : vj' ∈ ((pcLE prop acc f cp ck ledec spdec sm (k + 1)).2.2.1 i₂)[t]'hprv :=
          hvj'
        obtain ⟨j', tj', htj', hpay', -⟩ := hprom vj' hvj'2
        have htlk : tj' < ((pcSP prop acc f cp ck ledec spdec sm k).val.2.1 j').length := by
          have h0 := htj'
          rw [hknot k] at h0
          exact h0
        have hEk : (slot₀, lv)
            ∈ ((((pcSP prop acc f cp ck ledec spdec sm k).val.2.1 j')[tj']'htlk).2
              : LogMap P (mem prop)) := by
          have hbl : (slot₀, lv) ∈ (vj'.2 : LogMap P (mem prop)) :=
            Multiset.mem_coe.mp hlv2
          have hpayk' : vj' = ((pcSP prop acc f cp ck ledec spdec sm k).val.2.1 j')[tj']'htlk := by
            rw [hpay']
            congr 1
          rw [hpayk'] at hbl
          exact hbl
        exact (hsp k).log_entry (hreq k) rfl htlk hEk
      -- the champion's group agrees on one value: the champion's value
      -- is a stage-k emission's value at its own key
      obtain ⟨hbestval, lv₀, hlv₀, hlv₀b⟩ := logView_entry _ hbestmem
      have hlv₀E : (slot₀, lv₀) ∈ rcEntries
          (((pcLE prop acc f cp ck ledec spdec sm (k + 1)).2.2.1 i₂)[t]'hpr) := hlv₀
      have hem₀ := hgroup_em lv₀ hlv₀E
      rw [hlv₀b] at hem₀
      have hgroup_val : ∀ w ∈ ((((rcEntries
          (((pcLE prop acc f cp ck ledec spdec sm (k + 1)).2.2.1 i₂)[t]'hpr)).filter
            (fun x : Nat × LogValue P (mem prop) =>
              x.1 = slot₀)).map Prod.snd).filter
            (fun lv : LogValue P (mem prop) =>
              lv.ballot = best.ballot)).map (·.value),
          w = lv₀.value := by
        intro w hw
        obtain ⟨lv, hlvg, rfl⟩ := Multiset.mem_map.mp hw
        have hlv1 := Multiset.mem_filter.mp hlvg
        obtain ⟨x, hx, rfl⟩ := Multiset.mem_map.mp hlv1.1
        have hx1 := Multiset.mem_filter.mp hx
        have hxE : (slot₀, x.2) ∈ rcEntries
            (((pcLE prop acc f cp ck ledec spdec sm (k + 1)).2.2.1 i₂)[t]'hpr) := by
          rw [show ((slot₀, x.2) : Nat × LogValue P (mem prop))
            = x by
            cases x
            simp_all]
          exact hx1.1
        have hemx := hgroup_em x.2 hxE
        rw [hlv1.2] at hemx
        exact hfun k best.ballot.proposerId hemx hem₀
      have hbest_lv₀ : best.value = lv₀.value := by
        rw [hbestval]
        refine valOf_const _ ?_ hgroup_val
        intro habs
        have hlv₀g : lv₀ ∈ (((rcEntries
            (((pcLE prop acc f cp ck ledec spdec sm (k + 1)).2.2.1 i₂)[t]'hpr)).filter
              (fun x : Nat × LogValue P (mem prop) =>
                x.1 = slot₀)).map Prod.snd).filter
              (fun lv : LogValue P (mem prop) =>
                lv.ballot = best.ballot) := by
          refine Multiset.mem_filter.mpr ⟨?_, hlv₀b⟩
          exact Multiset.mem_map.mpr ⟨(slot₀, lv₀),
            Multiset.mem_filter.mpr ⟨hlv₀E, rfl⟩, rfl⟩
        have hmem0 : lv₀.value ∈ (((((rcEntries
            (((pcLE prop acc f cp ck ledec spdec sm (k + 1)).2.2.1 i₂)[t]'hpr)).filter
              (fun x : Nat × LogValue P (mem prop) =>
                x.1 = slot₀)).map Prod.snd).filter
              (fun lv : LogValue P (mem prop) =>
                lv.ballot = best.ballot)).map (·.value)) :=
          Multiset.mem_map.mpr ⟨lv₀, hlv₀g, rfl⟩
        rw [habs] at hmem0
        cases hmem0
      -- dichotomy at the champion's ballot
      rcases Ballot.eq_or_blt_of_ble hb₁best with hbeq | hblt
      · -- the champion is the chosen ballot's own emission
        rw [← hbeq] at hem₀
        rw [hb₁own] at hem₀
        -- lift both to a common stage and use send-once
        have hkK : k ≤ max k K := Nat.le_max_left _ _
        have hKk : K ≤ max k K := Nat.le_max_right _ _
        have hv0 := hfun (max k K) i₁ (hemM hkK hem₀) (hemM hKk hm₁)
        rw [hveq, hbest_lv₀, hv0]
      · -- strictly higher champion: the induction hypothesis at `k`
        have hstep := ih best.ballot.proposerId best.ballot lv₀.value
          hblt hem₀
        rw [hveq, hbest_lv₀, hstep]
  -- the headline assembly: two commits, three-way ballot dichotomy
  intro i i' slot v v' h h'
  obtain ⟨b, hbo, hem, hch⟩ :=
    (hsp fuelALog).commit_spec (hreq fuelALog) rfl h
  obtain ⟨b', hbo', hem', hch'⟩ :=
    (hsp fuelALog).commit_spec (hreq fuelALog) rfl h'
  by_cases hbb : b.blt b' = true
  · exact (hagree fuelALog slot i b v hbo hem hch fuelALog i'
      b' v' hbb hem').symm
  · by_cases hbb' : b'.blt b = true
    · exact hagree fuelALog slot i' b' v' hbo' hem' hch'
        fuelALog i b v hbb' hem
    · -- equal ballots: one proposer, one key, one emission
      have hble : b.ble b' = true := by
        have h0 : b'.blt b = false := by simpa using hbb'
        exact Ballot.ble_of_not_blt h0
      have heq : b = b' := by
        rcases Ballot.eq_or_blt_of_ble hble with heq | hblt2
        · exact heq
        · exact absurd hblt2 (by simpa using hbb)
      subst heq
      have hii : i' = i := (hbo.symm.trans hbo').symm
      subst hii
      exact hfun fuelALog _ hem hem'

end Agreement

end HydroV2
