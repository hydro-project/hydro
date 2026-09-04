import Hydro.Paxos.LeaderElection
import Hydro.Paxos.SequencePayload
import Hydro.HydroDef
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
run is exercised executably by `lake exe paxos`.
-/

namespace Hydro

variable {L : Type} {mem : L → Nat} {P : Type} [DecidableEq P]

/-- Quorum intersection (pigeonhole): two duplicate-free member lists
jointly longer than the cluster share a member. -/
private theorem nodup_inter_of_length {n : Nat} {S C : List (Fin n)}
    (hS : S.Nodup) (hC : C.Nodup) (h : n < S.length + C.length) :
    ∃ j, j ∈ S ∧ j ∈ C := by
  by_contra habs
  have hdisj : S.Disjoint C := fun {a} ha hb => habs ⟨a, ha, hb⟩
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

set_option maxHeartbeats 400000 in
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
  fix (a_log : H.Ticked acc (ALog P (mem prop)))
      (sequencing_max_ballots : H.Stream prop (Ballot (mem prop))
        .noOrder .exactlyOnce)
      via (dec.fuelALog, dec.fuelSeqMax)
    -- **K4.** Chosen-at-b₁ (at the closed knot) dominates: a value this
    -- stage's leader OPENS at a higher ballot (`LeaderOpen`, the face a
    -- published entry/commit pins) is the value every top-level entry
    -- at `(slot, b₁)` carries. `le`/`sp` are this stage's runs,
    -- `le'`/`sp'` the closed knot's.
    invariant a_log (le sp le' sp') =>
      variant = .guarded → mem acc ≤ 2 * f + 1 →
      ∀ (slot : Nat) (b₁ b₂ : Ballot (mem prop)) (val₁ v₂ : Option P)
        (i₂ : Fin (mem prop)),
        SPChosen f (fun j => a_checkpoint j (dec.sp.ap2.ckSnap j))
          le'.2.2.2 sp'.2.1 slot b₁ →
        (∀ (j : Fin (mem acc)) {t : Nat} {lg : ALog P (mem prop)}
          {e : LogValue P (mem prop)},
          (sp'.2.1 j)[t]? = some lg → (slot, e) ∈ lg.2 → e.ballot = b₁ →
          e.value = val₁) →
        b₁.blt b₂ = true →
        LeaderOpen (le.1 i₂) (le.2.1 i₂) (le.2.2.1 i₂) b₂ slot v₂ →
        v₂ = val₁,
      base := (by
        -- the seed stage opens nothing: no acceptor has ever ticked, so
        -- a leader tick's promise quorum cannot exist
        intro a_log' le sp le' sp'
        intro _htop _hle' _hsp' hle _hsp _hvar _hnA
        intro slot b₁ b₂ val₁ v₂ i₂ _hch _hval _hlt hopen
        exfalso
        obtain ⟨t, view, hl, -, -, -⟩ := hopen
        obtain ⟨hpl, hl⟩ := List.getElem?_eq_some_iff.mp hl
        obtain ⟨hpr2, _, hcard, hprom⟩ := hle.view_promise i₂ hpl hl
        obtain ⟨v0, hv0⟩ : ∃ v0, v0 ∈ (le.2.2.1 i₂)[t]'hpr2 :=
          Multiset.card_pos_iff_exists_mem.mp (by omega)
        obtain ⟨j, tj, htj, -⟩ := hprom v0 hv0
        simp at htj),
      step := (by
        intro a_log' le sp le_step sp_step le' sp'
        intro htop hle' hsp' hle hsp hle_step hsp_step hw hbw _hbw0 ih
        intro hvar hnA
        intro slot b₁ b₂ val₁ v₂ i₂ hch hval hlt hopen
        -- this stage's published log is below the top's (the
        -- write-before-ack step, then the top's own)
        have hpublift : ∀ jj, sp.2.1 jj <+: sp'.2.1 jj :=
          fun jj => (hbw jj).trans (htop jj)
        -- the step's max wire is below the top's (generated mono)
        have hblift₄ := leader_election_mono₄
          (p_received_p2b_ballots := sequencing_max_ballots)
          (p_received_p2b_ballots' := sequencing_max_ballots)
          variant prop acc (f + 1) (2 * f + 1) dec.le sched.le
          (fun _ => PoolLe.refl _ _ _) hbw
        -- the ballot/leader/view requirements at this stage (the LE
        -- faces, projected)
        have hreq := hle.discipline (by omega)
        -- the opening leader tick and its input view
        obtain ⟨t, view, hl, hb, hv, hchar⟩ := hopen
        obtain ⟨hpl, hl⟩ := List.getElem?_eq_some_iff.mp hl
        obtain ⟨hpb, hbal⟩ := List.getElem?_eq_some_iff.mp hb
        obtain ⟨hpr, hview⟩ := List.getElem?_eq_some_iff.mp hv
        subst hview
        -- f + 1 distinct promisers at the opening tick
        obtain ⟨hpr', hpb', S, hSnd, hSlen, hSprov⟩ :=
          hle_step.providers hvar (by omega) i₂ hpl hl
        -- f + 1 distinct voters for the chosen key
        obtain ⟨C, hCnd, hClen, hCvote⟩ := id hch
        -- quorum intersection
        obtain ⟨j, hjS, hjC⟩ := nodup_inter_of_length hSnd hCnd (by omega)
        obtain ⟨vj, hvj, tj, htj, hpay, hmj, hmax⟩ := hSprov j hjS
        obtain ⟨t₁, hta, hama, hcovck⟩ := hCvote j hjC
        -- the promise's ballot, on the top max wire
        have hmjM : tj < (le'.2.2.2 j).length :=
          Nat.lt_of_lt_of_le hmj (hblift₄ j).length_le
        have hmax2 : (le_step.2.2.2 j)[tj]'hmj = some b₂ := by
          rw [hmax]
          exact congrArg some hbal
        have hvalj : (le'.2.2.2 j)[tj]'hmjM = some b₂ :=
          (prefix_getElem_lift (hblift₄ j) hmj).trans hmax2
        -- the vote precedes the promise on the ascending max wire
        have ht1tj : t₁ < tj := by
          refine Nat.lt_of_not_le fun hle2 => ?_
          have hasc := hle'.max_mono j hle2 hta
          rw [hvalj, hama] at hasc
          exact Ballot.ble_blt_asymm hasc hlt
        -- the vote's coverage is realized on the top log at the
        -- promise tick (the promise saw the log later)
        have hcklen : t₁ < (a_checkpoint j (dec.sp.ap2.ckSnap j)).length := by
          have h2 := hsp.log_len_le_ck j
          omega
        obtain ⟨htl₁, hcov₁⟩ := hcovck hcklen
        have htjM' : tj < (sp'.2.1 j).length :=
          Nat.lt_of_lt_of_le htj (hpublift j).length_le
        have hcovtj : LogCovers ((sp'.2.1 j)[tj]'htjM').2 slot b₁ :=
          hsp'.log_covers_mono (Nat.le_of_lt ht1tj) htjM' hcov₁
        -- the promise payload IS this stage's published log entry
        have hpayeq : vj = (sp'.2.1 j)[tj]'htjM' :=
          hpay.trans (prefix_getElem_lift (hpublift j) htj).symm
        have hcovvj : LogCovers vj.2 slot b₁ := by
          rw [hpayeq]
          exact hcovtj
        obtain ⟨e₀, he₀mem, he₀ble⟩ := hcovvj
        -- the covering entry lives in the opening tick's input view
        have hE : (slot, e₀) ∈ rcEntries ((le_step.2.2.1 i₂)[t]'hpr) := by
          unfold rcEntries
          exact Multiset.mem_bind.mpr ⟨vj, hvj, Multiset.mem_coe.mpr he₀mem⟩
        -- the opened value is the view's champion at the slot
        obtain ⟨best, hbestmem, hveq, hdom⟩ := hchar e₀ hE
        have hb₁best : b₁.ble best.ballot = true :=
          Ballot.ble_trans he₀ble hdom
        -- every view entry is a published entry of this stage's log
        have hview_entry : ∀ lv : LogValue P (mem prop),
            (slot, lv) ∈ rcEntries ((le_step.2.2.1 i₂)[t]'hpr) →
            ∃ (j' : Fin (mem acc)) (tj' : Nat) (lg : ALog P (mem prop)),
              (sp.2.1 j')[tj']? = some lg ∧ (slot, lv) ∈ lg.2 := by
          intro lv hlv
          unfold rcEntries at hlv
          obtain ⟨vj', hvj', hlv2⟩ := Multiset.mem_bind.mp hlv
          obtain ⟨_, _, -, hprom⟩ := hle_step.view_promise i₂ hpl hl
          obtain ⟨j', tj', htj', hpay', -⟩ := hprom vj' hvj'
          refine ⟨j', tj', _, List.getElem?_eq_getElem htj', ?_⟩
          rw [← hpay']
          exact Multiset.mem_coe.mp hlv2
        -- the champion's group agrees (published entries at one key
        -- agree): the champion's value is a published entry's, at the
        -- champion's ballot
        obtain ⟨⟨lv₀, hlv₀, hlv₀b⟩, hgroup⟩ := logView_entry_value _ hbestmem
        obtain ⟨j₀, tj₀, lg₀, hlg₀, hen₀⟩ := hview_entry lv₀ hlv₀
        have hbest_lv₀ : best.value = lv₀.value :=
          hgroup lv₀.value fun lv hlv hlvb => by
            obtain ⟨j', tj', lg', hlg', hen'⟩ := hview_entry lv hlv
            exact hsp.log_entry_agree hreq hvar j' j₀ hlg' hen' hlg₀ hen₀
              (hlvb.trans hlv₀b.symm)
        rw [hveq, hbest_lv₀]
        -- dichotomy at the champion's ballot
        rcases Ballot.eq_or_blt_of_ble hb₁best with hbeq | hblt
        · -- the champion IS the chosen ballot: its entry, lifted to the
          -- top, carries the chosen value
          obtain ⟨h1, rfl⟩ := List.getElem?_eq_some_iff.mp hlg₀
          exact hval j₀ (List.getElem?_eq_some_iff.mpr
            ⟨Nat.lt_of_lt_of_le h1 (hpublift j₀).length_le,
              prefix_getElem_lift (hpublift j₀) h1⟩) hen₀ (hlv₀b.trans hbeq.symm)
        · -- strictly higher champion: this stage's leader opened it (a
          -- published entry is an opened value) — the invariant
          have ho := hsp.log_entry_open hreq hvar j₀ hlg₀ hen₀
          rw [hlv₀b] at ho
          exact ih hvar hnA slot b₁ best.ballot val₁ lv₀.value
            best.ballot.proposerId hch hval hblt ho)
  :=
      -- (p_ballot, p_is_leader, p_relevant_p1bs, a_max_ballot) =
      --   leader_election(proposers, acceptors, …, seq_max→, a_log→)
      let le := leader_election H variant prop acc (f + 1) (2 * f + 1)
        dec.le sched.le sequencing_max_ballots a_log
      -- the election contract, at these wires (replayed for the prove
      -- leg at the CLOSED knot)
      ghost have hle := leader_election.ensures variant prop acc (f + 1) (2 * f + 1)
        dec.le sched.le sequencing_max_ballots a_log
      -- just_became_leader = p_is_leader.and(¬ p_is_leader.defer_tick())
      let just_became_leader := H.mapTick
        (H.zipTick le.2.1 (H.defer_tick false le.2.1))
        (fun _me x => x.1 && !x.2)
      -- (p_to_replicas, a_log, sequencing_max_ballots) =
      --   sequence_payload(…, c_to_proposers, a_checkpoint, p_ballot, …)
      let sp := sequence_payload H variant prop acc c_to_proposers
        a_checkpoint
        le.1 le.2.1 le.2.2.1 f
        le.2.2.2 dec.sp sched.sp
      -- the sequencing contract, at these wires (replayed likewise)
      ghost have hsp := sequence_payload.ensures variant prop acc c_to_proposers
        a_checkpoint le.1 le.2.1 le.2.2.1 f le.2.2.2 dec.sp sched.sp
    -- a_log_complete_cycle.complete(a_log);
    -- sequencing_max_ballot_complete_cycle.complete(seq_max_ballots)
    complete (sp.2.1, sp.2.2)
    (-- p_ballot.filter_if(just_became_leader).all_ticks()
     H.allTicks (H.flattenOrdered (H.mapTick
       (H.zipTick le.1 just_became_leader)
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
        have hreq' := hle.discipline (by omega)
        -- the headline assembly: two commits of different values have
        -- different witness ballots; the lower is chosen and agreed by
        -- its entries, the higher was opened by its leader — K4
        intro i i' slot v v' h h'
        by_contra hne
        obtain ⟨b, b', hbb, W, W'⟩ := hsp.commit_distinct hreq' rfl h h' hne
        cases hc : b.blt b' with
        | true => exact hne (hI slot b b' v v' i' W.chosen W.entries_agree hc W'.opened).symm
        | false =>
          have hlt := Ballot.blt_of_ble_ne (Ballot.ble_of_not_blt hc) (Ne.symm hbb)
          exact hne (hI slot b' b v' v i W'.chosen W'.entries_agree hlt W.opened)

/-! ## Executable non-vacuity

The end-to-end guarded-commit scenario (one proposer, one acceptor,
`f = 0`: election at tick 1, payload `42` sequenced at slot 0, accepted
through the same-tick `a_log` knot, committed by the singleton quorum)
runs in the compiled demo executable `paxos` — kernel-level `#guard`
evaluation of the nested Kleene closures is prohibitively slow, so the
non-vacuity check is a build artifact instead (`lake exe paxos`). -/


#nondet_census paxos_core (nondets := 13) (scheds := 5) (fuels := 5)

end Hydro
