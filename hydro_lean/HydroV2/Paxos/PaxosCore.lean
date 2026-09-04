import HydroV2.Paxos.LeaderElection
import HydroV2.Paxos.SequencePayload
import HydroV2.HydroDef
import Mathlib.Data.Fintype.Card

/-!
# `paxos_core` (paxos.rs:136–246)

The algorithm: `leader_election` and `sequence_payload`, tied by the
two remaining `forward_ref` cycles —

- **`sequencing_max_ballots`** (stream at proposers, `NoOrder
  ExactlyOnce`): P2b rejection ballots feed leader election's
  received-max merge;
- **`a_log`** (tick singleton at acceptors): `acceptor_p2`'s
  `(checkpoint, log)` wire closes into `acceptor_p1`'s same-tick reply
  read — Rust's `snapshot_atomic` *write-before-ack* (a tick's P1b
  replies exist only once that tick's log value is realized).

Returns (the new-leader ballot stream, `p_to_replicas`).

The safety face is `SlotFunctional` (agreement per slot across
replicas): `paxos_core` returns it **colocated** (`PCEnsures`), and
K4 — the ONE protocol induction (quorum intersection + the provenance
regress `k+1 → k` through the `a_log` knot) — lives in the
`slot_functional` prove leg BELOW, over the program's own wires. The
knot's stage vocabulary (`paxos_core.a_log.stages` + its
`_zero`/`_succ`/`_fix`/`_mono` laws) is GENERATED at the knot by the
eager `fix` pipeline: the any-body Kleene-chain truths come from the
engine; only the Paxos content is written here. The end-to-end commit
run is exercised executably by `lake exe v2paxos`.
-/

namespace HydroV2

variable {L : Type} {mem : L → Nat} {P : Type} [DecidableEq P]

/-- Quorum intersection (pigeonhole): two duplicate-free member lists
jointly longer than the cluster share a member. -/
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

/-- `leader_election`'s face, projected to `sequence_payload`'s input
requirement: the four contract legs are LE guarantees, and `mono` is
the monotonic ballot wire's own type. (The composition-point bridge —
LE's face and SP's requirement meet only here.) -/
private theorem LEEnsures.spRequires {variant : PaxosVariant}
    {prop acc : L} {qs : Nat}
    {al : TickV (mem acc) (ALog P (mem prop)) .unbounded}
    {o : TickV (mem prop) (Ballot (mem prop))
        (.monotonic (Ballot.numVO (nP := mem prop)))
      × TickV (mem prop) Bool .unbounded
      × (Fin (mem prop) → Trace (Multiset (ALog P (mem prop))))
      × TickV (mem acc) (Option (Ballot (mem prop)))
          (.monotonic Ballot.obtVO)}
    (h : LEEnsures variant prop acc qs al o) (hqs : 1 ≤ qs) :
    SPRequires (mem prop) P (fun i => ((o.1 i).vals)) o.2.1 o.2.2.1 :=
  { own := h.own
    mono := fun i {t t'} htt ht' => (o.1 i).ascending htt ht'
    lead_ne := fun i {t} hpl hpr hl => h.lead_ne hqs i hpl hpr hl
    stable := fun i {t} ht1 hb1 h1 h0 => h.stable hqs i ht1 hb1 h1 h0
    pinned := fun i {t t'} hpl hpl' hpr hpr' hpb hpb' h1 h2 h3 =>
      h.pinned hqs i hpl hpl' hpr hpr' hpb hpb' h1 h2 h3 }

/-- All **nondet** decision data `paxos_core` threads to its callees —
nested by owning module (`LEDec` and `SPDec` carry their own `nondet!`
sites), plus the two outer knot fuels. Adversary-side (sched-det) data
travels separately in `PaxosCoreSched`. -/
structure PaxosCoreDec (H : HydroSem L mem) (nP nA : Nat) (P : Type)
    [DecidableEq P] (ckα : Type) [DecidableEq ckα]
    (ckord : StrOrd) where
  /-- `leader_election`'s decisions (received-max snapshot, heartbeat
  timing, P1a batching, quorum collection, its three cycle fuels). -/
  le : LEDec H nP nA P
  /-- `sequence_payload`'s decisions (payload/P2a/P2b batching + the
  checkpoint snapshot, B2). -/
  sp : SPDec H nP nA P ckα ckord
  /-- `sequencing_max_ballot` knot depth (`forward_ref`). -/
  fuelSeqMax : H.FixDec
  /-- `a_log` knot depth (`forward_ref`, `snapshot_atomic`'s
  write-before-ack staging). -/
  fuelALog : H.FixDec

/-- **`paxos_core`'s adversary-side (sched-det) bundle** (`Unit` at
`Values`; see `Sem.lean`'s classification table), nested by owning
module like `PaxosCoreDec`. -/
structure PaxosCoreSched (H : HydroSem L mem) (nP nA : Nat) (P : Type)
    [DecidableEq P] where
  /-- `leader_election`'s bundle. -/
  le : LESched H nP nA P
  /-- `sequence_payload`'s bundle. -/
  sp : SPSched H nP nA P

/-- The trivial bundle at the denotation. -/
def PaxosCoreSched.triv {nP nA : Nat} {P : Type} [DecidableEq P] :
    PaxosCoreSched (Values L mem) nP nA P := ⟨.triv, .triv⟩

/-- What `paxos_core` **ensures**, over the `Values` denotation — THE
HEADLINE: with the bugfix flags (`variant = .guarded`) and intersecting
`f + 1` quorums (`mem acc ≤ 2f + 1`), commits are slot-functional, over
the full decision space. -/
structure PCEnsures (variant : PaxosVariant) (prop acc : L) (f : Nat)
    (out : (Fin (mem prop) → Trace (Ballot (mem prop)))
      × (Fin (mem prop) → Multiset (Nat × Option P))) : Prop where
  /-- Per-slot agreement of the replica stream. -/
  slot_functional : variant = .guarded → mem acc ≤ 2 * f + 1 →
    SlotFunctional out.2

set_option maxHeartbeats 12800000 in
set_option maxRecDepth 65536 in
/-- **paxos.rs:136–246 `paxos_core`**: `leader_election` +
`sequence_payload`, tied by the `sequencing_max_ballot` and `a_log`
`forward_ref` knots. Returns (new-leader ballots, `p_to_replicas`),
with the safety headline colocated.

K4 — the ONE protocol induction (quorum intersection + the provenance
regress through the knot) — is the `invariant` clause on the `a_log`
cycle below, Verus-ordered: the predicate relates this stage's
sequencing runs (`le`/`sp`, the fix body's own `let`s) to the closed
knot's (`le'`/`sp'`); `base`/`step` are its obligations, with the
faces (`hle`, `hsp_step`, …), the chain orders (`hw`, `hbw`, `htop`)
and the packaged induction supplied by the construct. No proof
mentions stages; the prove leg consumes only ghost facts — the
construct-supplied closed-knot induction (`ha_log_inv`) and the
body's own `ghost have` faces (`hle`/`hsp`), replayed at the closed
wires. -/
hydro [p1bPairDecEq] def paxos_core {ckα : Type} [DecidableEq ckα]
    {ckord : StrOrd} {ckret : Retries}
    (H : HydroSem L mem) (variant : PaxosVariant)
    (prop acc : L) (f : Nat)
    (c_to_proposers : H.Stream prop P .totalOrder .exactlyOnce)
    (a_checkpoint : H.Singleton acc ckα (Option Nat) ckord ckret
      .unbounded)
    (dec : PaxosCoreDec H (mem prop) (mem acc) P ckα ckord)
    (sched : PaxosCoreSched H (mem prop) (mem acc) P) :
    (H.Stream prop (Ballot (mem prop)) .totalOrder .exactlyOnce
      × H.Stream prop (Nat × Option P) .noOrder .exactlyOnce)
  ensures out => PCEnsures variant prop acc f out :=
  -- paxos.rs:141–148: the `a_log` and `sequencing_max_ballot`
  -- forward_ref cycles, closed mutually (`a_log` re-closed per
  -- sequencing iterate — Rust's `snapshot_atomic` write-before-ack)
  fix (a_log : H.TickSingleton acc (ALog P (mem prop)) .unbounded)
      (sequencing_max_ballots : H.Stream prop (Ballot (mem prop))
        .noOrder .exactlyOnce)
      via (dec.fuelALog, dec.fuelSeqMax)
    -- **K4.** Chosen-at-b₁ (at the closed knot) dominates: any
    -- emission of this stage at a higher ballot carries the chosen
    -- value. `le`/`sp` are this stage's runs, `le'`/`sp'` the closed
    -- knot's.
    invariant a_log (le sp le' sp') =>
      variant = .guarded → mem acc ≤ 2 * f + 1 →
      ∀ (slot : Nat) (b₁ b₂ : Ballot (mem prop)) (val₁ v₂ : Option P)
        (i₁ i₂ : Fin (mem prop)),
        b₁.proposerId = i₁ →
        SPEmission variant f (c_to_proposers i₁)
          (dec.sp.payloadBatch i₁) ((le'.1 i₁).vals) (le'.2.1 i₁)
          (le'.2.2.1 i₁) slot b₁ val₁ →
        SPChosen f (fun j => a_checkpoint j (dec.sp.ap2.ckSnap j))
          (fun j => ((le'.2.2.2 j).vals)) sp'.2.1 slot b₁ →
        b₁.blt b₂ = true →
        SPEmission variant f (c_to_proposers i₂)
          (dec.sp.payloadBatch i₂) ((le.1 i₂).vals) (le.2.1 i₂)
          (le.2.2.1 i₂) slot b₂ v₂ →
        v₂ = val₁,
      base := (by
        -- the seed stage has no emissions: no acceptor has ever
        -- ticked, so a leader tick's promise quorum cannot exist
        intro a_log' le sp le' sp'
        intro _htop _hle' _hsp' hle _hsp hvar hnA
        intro slot b₁ b₂ val₁ v₂ i₁ i₂ hb₁own hm₁ hch hlt hm₂
        exfalso
        obtain ⟨t, hpl, hpb, hgv, hl, -, -⟩ :=
          spSentTrace_open variant f _ _ _ _ _ hm₂
        obtain ⟨hpr2, hpb2, hcard, hprom⟩ := hle.view_promise i₂ hpl hl
        have hpos : ∃ v0, v0 ∈ (le.2.2.1 i₂)[t]'hpr2 := by
          refine Multiset.card_pos_iff_exists_mem.mp ?_
          omega
        obtain ⟨v0, hv0⟩ := hpos
        obtain ⟨j, tj, htj, -⟩ := hprom v0 hv0
        simp at htj),
      step := (by
        intro a_log' le sp le_step sp_step le' sp'
        intro htop hle' hsp' hle hsp hle_step hsp_step hw hbw _hbw0 ih
        intro hvar hnA
        rw [hvar] at ih
        intro slot b₁ b₂ val₁ v₂ i₁ i₂ hb₁own hm₁ hch hlt hm₂
        rw [hvar] at hm₁ hm₂
        -- this stage's published log is below the top's (the
        -- write-before-ack step, then the top's own)
        have hpublift : ∀ jj, sp.2.1 jj <+: sp'.2.1 jj :=
          fun jj => (hbw jj).trans (htop jj)
        -- the election runs lifted along the wires (generated monos)
        have hlift₁ := leader_election_mono₁
          (p_received_p2b_ballots := sequencing_max_ballots)
          (p_received_p2b_ballots' := sequencing_max_ballots)
          variant prop acc (f + 1) (2 * f + 1) dec.le sched.le
          (fun _ => PoolLe.refl _ _ _) hw
        have hlift₂ := leader_election_mono₂
          (p_received_p2b_ballots := sequencing_max_ballots)
          (p_received_p2b_ballots' := sequencing_max_ballots)
          variant prop acc (f + 1) (2 * f + 1) dec.le sched.le
          (fun _ => PoolLe.refl _ _ _) hw
        have hlift₃ := leader_election_mono₃
          (p_received_p2b_ballots := sequencing_max_ballots)
          (p_received_p2b_ballots' := sequencing_max_ballots)
          variant prop acc (f + 1) (2 * f + 1) dec.le sched.le
          (fun _ => PoolLe.refl _ _ _) hw
        have hblift₄ := leader_election_mono₄
          (p_received_p2b_ballots := sequencing_max_ballots)
          (p_received_p2b_ballots' := sequencing_max_ballots)
          variant prop acc (f + 1) (2 * f + 1) dec.le sched.le
          (fun _ => PoolLe.refl _ _ _) hbw
        -- the ballot/leader/view requirements, per valuation (the
        -- LE faces, projected)
        have hreq := hle.spRequires (by omega)
        have hreq_step := hle_step.spRequires (by omega)
        have hreq' := hle'.spRequires (by omega)
        -- emissions of this stage persist to the top (sent traces
        -- are final along the lifted election runs)
        have hemW : ∀ {i₀ : Fin (mem prop)} {slot₀ : Nat}
            {b₀ : Ballot (mem prop)} {v₀ : Option P},
            SPEmission .guarded f (c_to_proposers i₀)
              (dec.sp.payloadBatch i₀) ((le.1 i₀).vals) (le.2.1 i₀)
              (le.2.2.1 i₀) slot₀ b₀ v₀ →
            SPEmission .guarded f (c_to_proposers i₀)
              (dec.sp.payloadBatch i₀) ((le'.1 i₀).vals) (le'.2.1 i₀)
              (le'.2.2.1 i₀) slot₀ b₀ v₀ := by
          intro i₀ slot₀ b₀ v₀ hem
          have hpre := spSentTrace_prefix .guarded f
            (dec.sp.payloadBatch i₀)
            (List.prefix_refl (c_to_proposers i₀))
            (hlift₁ i₀) (hlift₂ i₀) (hlift₃ i₀)
          obtain ⟨ch, hch2, hin⟩ := List.mem_flatten.mp hem
          exact List.mem_flatten.mpr ⟨ch, hpre.subset hch2, hin⟩
        -- the emission's leader tick and its input-view
        -- characterization
        obtain ⟨t, hpl, hpb, hpr, hl, hbal, hcovd⟩ :=
          spSentTrace_open_input f (c_to_proposers i₂)
            (dec.sp.payloadBatch i₂) i₂ _ _ _
            (hreq_step.own i₂)
            (fun h ht' => hreq_step.mono i₂ h ht')
            (fun ht1 hb1 h1 h0 => hreq_step.stable i₂ ht1 hb1 h1 h0)
            (fun hpl hpr hll => hreq_step.lead_ne i₂ hpl hpr hll)
            (fun hpl hpl' hpr hpr' hpb hpb' h1 h2 h3 =>
              hreq_step.pinned i₂ hpl hpl' hpr hpr' hpb hpb'
                h1 h2 h3)
            hm₂
        -- f + 1 distinct promisers at the emission tick
        obtain ⟨hpr', hpb', S, hSnd, hSlen, hSprov⟩ :=
          hle_step.providers hvar (by omega) i₂ hpl hl
        -- f + 1 distinct voters for the chosen key
        obtain ⟨C, hCnd, hClen, hCvote⟩ := id hch
        -- quorum intersection
        obtain ⟨j, hjS, hjC⟩ := nodup_inter_of_length hSnd hCnd
          (by omega)
        obtain ⟨vj, hvj, tj, htj, hpay, hmj, hmax⟩ := hSprov j hjS
        obtain ⟨t₁, hta, hama, hcovck⟩ := hCvote j hjC
        -- the promise's ballot, on the top max wire
        have hmjM : tj < ((le'.2.2.2 j)).vals.length :=
          Nat.lt_of_lt_of_le hmj (hblift₄ j).length_le
        have hmax2 : ((le_step.2.2.2 j)).vals[tj]'hmj = some b₂ := by
          rw [hmax]
          exact congrArg some hbal
        have hvalj : ((le'.2.2.2 j)).vals[tj]'hmjM = some b₂ :=
          (prefix_getElem_lift (hblift₄ j) hmj).trans hmax2
        -- the vote precedes the promise on the ascending max wire
        have ht1tj : t₁ < tj := by
          by_contra hle2
          push_neg at hle2
          have hasc := ((le'.2.2.2 j)).ascending hle2 hta
          rw [hvalj, hama] at hasc
          exact Ballot.ble_blt_asymm hasc hlt
        -- the vote's coverage is realized on the top log at the
        -- promise tick (the promise saw the log later)
        have hcklen : t₁ < (a_checkpoint j (dec.sp.ap2.ckSnap j)).length := by
          have h2 := hsp.log_len_le_ck j
          omega
        obtain ⟨htl₁, hcov₁⟩ := hcovck hcklen
        have htjM' : tj < ((sp'.2.1) j).length :=
          Nat.lt_of_lt_of_le htj (hpublift j).length_le
        have hcovtj : LogCovers ((((sp'.2.1) j)[tj]'htjM').2)
            slot b₁ :=
          hsp'.log_covers_mono (Nat.le_of_lt ht1tj) htjM' hcov₁
        -- the promise payload IS this stage's published log entry
        have hpayeq : vj = ((sp'.2.1) j)[tj]'htjM' :=
          hpay.trans (prefix_getElem_lift (hpublift j) htj).symm
        have hcovvj : LogCovers vj.2 slot b₁ := by
          rw [hpayeq]
          exact hcovtj
        obtain ⟨e₀, he₀mem, he₀ble⟩ := hcovvj
        -- the covering entry lives in the emission tick's input view
        have hE : (slot, e₀) ∈ rcEntries
            (((le_step.2.2.1) i₂)[t]'hpr) := by
          unfold rcEntries
          refine Multiset.mem_bind.mpr ⟨vj, hvj, ?_⟩
          exact Multiset.mem_coe.mpr he₀mem
        -- the emission's value is the view's champion at the slot
        obtain ⟨best, hbestmem, hveq, hdom⟩ := hcovd e₀ hE
        have hb₁best : b₁.ble best.ballot = true :=
          Ballot.ble_trans he₀ble hdom
        -- every view entry at the champion's key regresses to an
        -- emission of THIS stage (back through the knot, entrywise)
        have hgroup_em : ∀ lv : LogValue P (mem prop),
            (slot, lv) ∈ rcEntries
              (((le_step.2.2.1) i₂)[t]'hpr) →
            SPEmission .guarded f
              (c_to_proposers lv.ballot.proposerId)
              (dec.sp.payloadBatch lv.ballot.proposerId)
              ((le.1 lv.ballot.proposerId).vals)
              (le.2.1 lv.ballot.proposerId)
              (le.2.2.1 lv.ballot.proposerId)
              slot lv.ballot lv.value := by
          intro lv hlv
          unfold rcEntries at hlv
          obtain ⟨vj', hvj', hlv2⟩ := Multiset.mem_bind.mp hlv
          obtain ⟨hprv, hpbv, -, hprom⟩ :=
            hle_step.view_promise i₂ hpl hl
          have hvj'2 : vj' ∈ ((le_step.2.2.1) i₂)[t]'hprv := hvj'
          obtain ⟨j', tj', htj', hpay', -⟩ := hprom vj' hvj'2
          have hEk : (slot, lv) ∈ ((((sp.2.1) j')[tj']'htj').2
              : LogMap P (mem prop)) := by
            have hbl : (slot, lv) ∈ (vj'.2 : LogMap P (mem prop)) :=
              Multiset.mem_coe.mp hlv2
            rw [hpay'] at hbl
            exact hbl
          exact hsp.log_entry hreq hvar htj' hEk
        -- the champion's group agrees on one value (send-once at
        -- this stage): the champion's value is an emission's value
        obtain ⟨hbestval, lv₀, hlv₀, hlv₀b⟩ := logView_entry _ hbestmem
        have hem₀ := hgroup_em lv₀ hlv₀
        rw [hlv₀b] at hem₀
        have hgroup_val : ∀ w₀ ∈ ((((rcEntries
            (((le_step.2.2.1) i₂)[t]'hpr)).filter
              (fun x : Nat × LogValue P (mem prop) =>
                x.1 = slot)).map Prod.snd).filter
              (fun lv : LogValue P (mem prop) =>
                lv.ballot = best.ballot)).map (·.value),
            w₀ = lv₀.value := by
          intro w₀ hw₀
          obtain ⟨lv, hlvg, rfl⟩ := Multiset.mem_map.mp hw₀
          have hlv1 := Multiset.mem_filter.mp hlvg
          obtain ⟨x, hx, rfl⟩ := Multiset.mem_map.mp hlv1.1
          have hx1 := Multiset.mem_filter.mp hx
          have hxE : (slot, x.2) ∈ rcEntries
              (((le_step.2.2.1) i₂)[t]'hpr) := by
            rw [show ((slot, x.2) : Nat × LogValue P (mem prop))
              = x by
              cases x
              simp_all]
            exact hx1.1
          have hemx := hgroup_em x.2 hxE
          rw [hlv1.2] at hemx
          exact hreq.send_once best.ballot.proposerId hemx hem₀
        have hbest_lv₀ : best.value = lv₀.value := by
          rw [hbestval]
          refine valOf_const _ ?_ hgroup_val
          intro habs
          have hlv₀g : lv₀ ∈ (((rcEntries
              (((le_step.2.2.1) i₂)[t]'hpr)).filter
                (fun x : Nat × LogValue P (mem prop) =>
                  x.1 = slot)).map Prod.snd).filter
                (fun lv : LogValue P (mem prop) =>
                  lv.ballot = best.ballot) := by
            refine Multiset.mem_filter.mpr ⟨?_, hlv₀b⟩
            exact Multiset.mem_map.mpr ⟨(slot, lv₀),
              Multiset.mem_filter.mpr ⟨hlv₀, rfl⟩, rfl⟩
          have hmem0 : lv₀.value ∈ (((((rcEntries
              (((le_step.2.2.1) i₂)[t]'hpr)).filter
                (fun x : Nat × LogValue P (mem prop) =>
                  x.1 = slot)).map Prod.snd).filter
                (fun lv : LogValue P (mem prop) =>
                  lv.ballot = best.ballot)).map (·.value)) :=
            Multiset.mem_map.mpr ⟨lv₀, hlv₀g, rfl⟩
          rw [habs] at hmem0
          cases hmem0
        -- dichotomy at the champion's ballot
        rcases Ballot.eq_or_blt_of_ble hb₁best with hbeq | hblt
        · -- the champion is the chosen ballot's own emission: lift
          -- it to the top and use send-once against the ambient one
          rw [← hbeq] at hem₀
          rw [hb₁own] at hem₀
          have hv0 := hreq'.send_once i₁ (hemW hem₀) hm₁
          rw [hveq, hbest_lv₀, hv0]
        · -- strictly higher champion: the invariant at this stage
          have hstep2 := ih rfl hnA slot b₁ best.ballot val₁
            lv₀.value i₁ best.ballot.proposerId hb₁own hm₁ hch hblt
            hem₀
          rw [hveq, hbest_lv₀, hstep2])
  :=
      -- (p_ballot, p_is_leader, p_relevant_p1bs, a_max_ballot) =
      --   leader_election(proposers, acceptors, …, seq_max→, a_log→)
      let leS := leader_election H variant prop acc (f + 1) (2 * f + 1)
        dec.le sched.le sequencing_max_ballots a_log
      let le := leS.val
      -- the election contract, at these wires (replayed for the prove
      -- leg at the CLOSED knot)
      ghost have hle := leS.property rfl
      -- just_became_leader = p_is_leader.and(¬ p_is_leader.defer_tick())
      let just_became_leader := H.mapTick
        (H.zipTick le.2.1 (H.defer false le.2.1))
        (fun _me x => x.1 && !x.2)
      -- (p_to_replicas, a_log, sequencing_max_ballots) =
      --   sequence_payload(…, c_to_proposers, a_checkpoint, p_ballot, …)
      let spS := sequence_payload H variant prop acc c_to_proposers
        a_checkpoint
        (H.forgetBound le.1) le.2.1 le.2.2.1 f
        (H.forgetBound le.2.2.2) dec.sp sched.sp
      let sp := spS.val
      -- the sequencing contract, at these wires (replayed likewise)
      ghost have hsp := spS.property rfl
    -- a_log_complete_cycle.complete(a_log);
    -- sequencing_max_ballot_complete_cycle.complete(seq_max_ballots)
    complete (sp.2.1, sp.2.2)
    (-- p_ballot.filter_if(just_became_leader).all_ticks()
     H.allTicks (H.emitBatches (H.mapTick
       (H.zipTick (H.forgetBound le.1) just_became_leader)
       (fun _me x => if x.2 then [x.1] else []))),
     sp.1)
    prove
      slot_functional := fun hvar hnA => by
        subst hvar
        -- the packaged knot induction, at the closed wires (the
        -- construct-supplied `ha_log_inv`)
        have hI := ha_log_inv rfl hnA
        -- `hle`/`hsp` (the ghost faces above, replayed at the closed
        -- knot) are the final-stage contracts
        have hreq' := hle.spRequires (by omega)
        -- the headline assembly: two commits, three-way dichotomy
        intro i i' slot v v' h h'
        obtain ⟨b, hbo, hem, hch⟩ :=
          hsp.commit_spec hreq' rfl h
        obtain ⟨b', hbo', hem', hch'⟩ :=
          hsp.commit_spec hreq' rfl h'
        by_cases hbb : b.blt b' = true
        · exact (hI slot b b' v v' i i' hbo hem hch hbb hem').symm
        · by_cases hbb' : b'.blt b = true
          · exact hI slot b' b v' v i' i hbo' hem' hch' hbb' hem
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
            exact hreq'.send_once _ hem hem'

/-! ## Executable non-vacuity

The end-to-end guarded-commit scenario (one proposer, one acceptor,
`f = 0`: election at tick 1, payload `42` sequenced at slot 0, accepted
through the same-tick `a_log` knot, committed by the singleton quorum)
runs in the compiled demo executable `v2paxos` — kernel-level `#guard`
evaluation of the nested Kleene closures is prohibitively slow, so the
non-vacuity check is a build artifact instead (`lake exe v2paxos`). -/


#nondet_census paxos_core (nondets := 13) (scheds := 9) (fuels := 5)

end HydroV2
