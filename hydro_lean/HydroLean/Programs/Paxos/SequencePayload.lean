import HydroLean.Programs.Paxos.Recommit
import HydroLean.Programs.Paxos.AcceptorP2
import HydroLean.Programs.CollectQuorumStreams
import HydroLean.Programs.IndexPayloads
import HydroLean.Programs.JoinResponses

/-!
# `sequence_payload` (paxos.rs:679–774) — module

The Rust function over the located surface, 1:1 with its signature:

```rust
fn sequence_payload(proposers, acceptors, proposer_tick, acceptor_tick,
    c_to_proposers : Stream<P, Cluster<Proposer>>,
    a_checkpoint : Optional<usize, Cluster<Acceptor>>,
    p_ballot, p_is_leader, p_relevant_p1bs, f, a_max_ballot, nondet_commit)
  -> (p_to_replicas : Stream<(usize, Option<P>), Cluster<Proposer>, NoOrder>,
      a_log : Optional<(Option<usize>, HashMap<…>), Atomic<Cluster<Acceptor>>>,
      sequencing_max_ballots : Stream<Ballot, Cluster<Proposer>, NoOrder>)
```

`a_checkpoint` is dropped (`none`) as authorized (garbage collection only).
The proposer-side dataflow (`recommit_after_leader_election` →
`index_payloads` → `payloads_to_send` → `join_responses` metadata) is the
per-proposer tick scan `sp_send_step`; `acceptor_p2` and the p2b
`collect_quorum` slice are the established modules; `join_responses`
(request_response.rs) is the established tick loop, consuming this tick's
metadata (`batch_atomic`, paxos.rs:760) and the `NoOrder`-batched quorum
keys.

The guarded variant's `recommittedAt` gate is the B2 fix (faithful
paxos.rs re-runs recommit on every tick that re-presents the quorum
snapshot — the executable violation in `Paxos/Falsification.lean`).
-/

namespace HydroLean.Programs.Paxos

open HydroLean.Hydro
open HydroLean.Programs

set_option synthInstance.maxSize 1024

variable {P : Type} {nP nA : Nat}

/-- The materialized `nondet_commit` decisions of `sequence_payload`. -/
structure SPNondet (P : Type) (nP nA : Nat) where
  /-- Per-proposer payload batch demands (`c_to_proposers.batch`,
  paxos.rs:711–722; `TotalOrder` input ⇒ prefix demands). -/
  payloadBatch : Fin nP → List Nat
  /-- Per-acceptor consumed P2a batches (paxos.rs:816–824; `NoOrder`). -/
  p2aBatch : Fin nA → List (List (P2a P nP))
  /-- Per-proposer p2b `collect_quorum` slice batches (paxos.rs:755). -/
  quorumBatch : Fin nP →
    List (List ((Nat × Ballot nP) × Except (Option (Ballot nP)) Unit))
  /-- Per-proposer `join_responses` response batches
  (request_response.rs; `NoOrder`). -/
  joinBatch : Fin nP → List (List (Nat × Ballot nP))

/-- Proposer-side looped state: `next_slot` (paxos.rs:783), the guarded
variant's `recommittedAt` (B2), and the previous tick's leader flag (for
the guarded `just_became_leader` gate). -/
structure SPSendSt (nP : Nat) where
  nextSlot : Nat
  recommittedAt : Option (Ballot nP)
  wasLeader : Bool

/-- One proposer tick of the send side (paxos.rs:706–734):
`recommit_after_leader_election` → `index_payloads` → `payloads_to_send`. -/
def sp_send_step (variant : PaxosVariant) (f : Nat) (s : SPSendSt nP)
    (t : List (P1bPayload P nP) × Ballot nP × Bool × List P) :
    SPSendSt nP × List ((Nat × Ballot nP) × Option P) :=
  let (qlogs, ballot, isLeader, payloads) := t
  let justBecame := isLeader && !s.wasLeader
  -- FAITHFUL: recommit re-runs on every tick with a relevant quorum view;
  -- GUARDED (B2 fix): once per ballot, on becoming leader.
  let recommitEnabled := !qlogs.isEmpty &&
    (if variant.recommitOnce then
      justBecame && s.recommittedAt ≠ some ballot
     else true)
  -- (p_log_to_recommit, p_max_slot) = recommit_after_leader_election(…)
  let rm := if recommitEnabled then recommitAfterLeaderElection f qlogs ballot
    else ([], none)
  -- indexed_payloads = index_payloads(p_max_slot, payloads.filter_if(leader))
  let ix := (indexPayloadsTick P).step s.nextSlot
    ⟨rm.2, if isLeader then payloads else []⟩
  -- payloads_to_send = indexed × ballot ++ recommit, filter_if(leader)
  let payloads_to_send :=
    if isLeader then
      (ix.2.map fun sp => ((sp.1, ballot), some sp.2)) ++ rm.1
    else []
  (⟨ix.1, if recommitEnabled then some ballot else s.recommittedAt, isLeader⟩,
   payloads_to_send)

/-- The growth carrier of `sequence_payload`'s dataflow: the input wires
`(c_to_proposers, p_ballot, p_is_leader, p_relevant_p1bs, a_max_ballot)`. -/
abbrev SPG (P : Type) (nP nA : Nat) : Type :=
  (Fin nP → Stream P) × (Fin nP → TSing (Ballot nP)) × (Fin nP → TSing Bool)
    × (Fin nP → TStream (P1bPayload P nP))
    × (Fin nA → TSing (Option (Ballot nP)))

/-- The Rust output triple `(p_to_replicas, a_log, sequencing_max_ballots)`
(the `a_log` family at its coverage-`Monotonic` wire type; the fail stream
in keyed per-acceptor form). -/
abbrev SPOut (P : Type) (nP nA : Nat) [DecidableEq P] : Type :=
  (Fin nP → Stream (Nat × Option P))
    × (Fin nA → MonoSing (covVOc (P := P) (nP := nP)))
    × (Fin nP → Fin nA → Stream (Ballot nP))




/-! ## The send-side step, characterized (guarded variant) -/

section SendStep

variable (f : Nat)

/-- One send-side tick input:
`(p_relevant_p1bs, p_ballot, p_is_leader, payload batch)`. -/
abbrev SPTick (P : Type) (nP : Nat) : Type :=
  List (P1bPayload P nP) × Ballot nP × Bool × List P

def spJustB (st : SPSendSt nP) (leader : Bool) : Bool :=
  leader && !st.wasLeader

def spRecEn (st : SPSendSt nP) (qlogs : List (P1bPayload P nP))
    (b' : Ballot nP) (leader : Bool) : Bool :=
  !qlogs.isEmpty && (spJustB st leader && st.recommittedAt ≠ some b')

def spRm (st : SPSendSt nP) (qlogs : List (P1bPayload P nP))
    (b' : Ballot nP) (leader : Bool) :
    List ((Nat × Ballot nP) × Option P) × Option Nat :=
  if spRecEn st qlogs b' leader then recommitAfterLeaderElection f qlogs b'
  else ([], none)

def spBase (st : SPSendSt nP) (qlogs : List (P1bPayload P nP))
    (b' : Ballot nP) (leader : Bool) : Nat :=
  ((spRm f st qlogs b' leader).2.map (· + 1)).getD st.nextSlot

/-- The guarded step, decomposed. -/
theorem sp_send_step_guarded_eq (st : SPSendSt nP)
    (qlogs : List (P1bPayload P nP)) (b' : Ballot nP) (leader : Bool)
    (pays : List P) :
    sp_send_step PaxosVariant.guarded f st (qlogs, b', leader, pays)
      = (⟨spBase f st qlogs b' leader
            + (if leader then pays else []).length,
          if spRecEn st qlogs b' leader then some b' else st.recommittedAt,
          leader⟩,
         if leader then
           (enumFrom (spBase f st qlogs b' leader) pays).map
             (fun sp => ((sp.1, b'), some sp.2))
             ++ (spRm f st qlogs b' leader).1
         else []) := by
  show sp_send_step PaxosVariant.guarded f st (qlogs, b', leader, pays) = _
  unfold sp_send_step
  dsimp only
  rw [show (if PaxosVariant.guarded.recommitOnce = true then
      (leader && !st.wasLeader) && decide (st.recommittedAt ≠ some b')
    else true)
    = ((leader && !st.wasLeader) && decide (st.recommittedAt ≠ some b'))
    from by rw [if_pos (show PaxosVariant.guarded.recommitOnce = true
      from rfl)]]
  rw [indexPayloadsTick_step]
  have hbase : baseSlot st.nextSlot
      ⟨(if !qlogs.isEmpty
          && ((leader && !st.wasLeader) && decide (st.recommittedAt ≠ some b'))
        then recommitAfterLeaderElection f qlogs b' else ([], none)).2,
        if leader then pays else []⟩
      = spBase f st qlogs b' leader := by
    unfold baseSlot spBase spRm spRecEn spJustB
    rfl
  rw [hbase]
  by_cases hl : leader
  · subst hl
    simp only [if_pos rfl]
    rfl
  · have : leader = false := Bool.eq_false_iff.mpr hl
    subst this
    simp only [Bool.false_and, if_neg (Bool.false_ne_true)]
    rfl

end SendStep

/-! ## The P2a key-discipline invariant -/

/-- Hypothesis chain for the send-side scan: ballot ownership,
num-monotonicity, leader ⇒ nonempty quorum view, and the
`LeaderBallotStable` stability clause (paxos.rs:186–189's `nondet!`
guarantee — discharged at the run by the derived `leader_ballot_stable`,
FINDINGS D21). `bn`/`wl` are the previous tick's ballot number and leader
flag. -/
inductive SPChain (me : Fin nP) : Nat → Bool → List (SPTick P nP) → Prop
  | nil {bn : Nat} {wl : Bool} : SPChain me bn wl []
  | cons {bn : Nat} {wl : Bool} {x : SPTick P nP} {xs : List (SPTick P nP)} :
      x.2.1.proposerId = me → bn ≤ x.2.1.num →
      (x.2.2.1 = true → x.1 ≠ []) →
      (x.2.2.1 = true → wl = true → x.2.1 = ⟨bn, me⟩) →
      SPChain me x.2.1.num x.2.2.1 xs →
      SPChain me bn wl (x :: xs)

/-- Keys sent over a scan suffix from state `st`. -/
def spKeys (f : Nat) (st : SPSendSt nP) (ins : List (SPTick P nP)) :
    List (Nat × Ballot nP) :=
  ((scan (sp_send_step PaxosVariant.guarded f) st ins).flatten).map (·.1)

@[simp] theorem spKeys_nil (f : Nat) (st : SPSendSt nP) :
    spKeys (P := P) f st [] = [] := rfl

theorem spKeys_cons (f : Nat) (st : SPSendSt nP) (x : SPTick P nP)
    (xs : List (SPTick P nP)) :
    spKeys f st (x :: xs)
      = ((sp_send_step PaxosVariant.guarded f st x).2).map (·.1)
        ++ spKeys f ((sp_send_step PaxosVariant.guarded f st x).1) xs := by
  unfold spKeys
  rw [scan_cons, List.flatten_cons, List.map_append]

set_option maxHeartbeats 2000000 in
/-- **The guarded key discipline** (the B2 fix): over any chain-respecting
input, the emitted `(slot, ballot)` keys extend without duplicates, stay
owned, and respect the running-ballot/next-slot bookkeeping. -/
theorem spKeys_inv (f : Nat) (me : Fin nP) :
    ∀ (ins : List (SPTick P nP)) (bn : Nat) (wl : Bool) (st : SPSendSt nP)
      (oldKeys : List (Nat × Ballot nP)),
    SPChain me bn wl ins →
    st.wasLeader = wl →
    (wl = true → st.recommittedAt = some ⟨bn, me⟩) →
    (∀ b, st.recommittedAt = some b → b.num ≤ bn ∧ b.proposerId = me) →
    oldKeys.Nodup →
    (∀ key ∈ oldKeys, key.2.num ≤ bn ∧ key.2.proposerId = me) →
    (∀ key ∈ oldKeys, key.2.num = bn →
      key.1 < st.nextSlot ∧ st.recommittedAt = some key.2) →
    (oldKeys ++ spKeys f st ins).Nodup ∧
    (∀ key ∈ spKeys f st ins, key.2.proposerId = me)
  | [], bn, wl, st, oldKeys, _, _, _, _, hnd, _, _ => by
    rw [spKeys_nil, List.append_nil]
    exact ⟨hnd, fun key hk => nomatch hk⟩
  | x :: xs, bn, wl, st, oldKeys, hchain, hwl, hrec, hrecle, hnd, hball,
      hslot => by
    obtain ⟨qlogs, b', leader, pays⟩ := x
    cases hchain with
    | cons hown0 hle0 hqne0 hstable0 hchain' =>
    have hown : b'.proposerId = me := hown0
    have hle : bn ≤ b'.num := hle0
    have hqne : leader = true → qlogs ≠ [] := hqne0
    have hstable : leader = true → wl = true → b' = ⟨bn, me⟩ := hstable0
    rw [spKeys_cons, sp_send_step_guarded_eq]
    -- names for the tick's derived values
    have hstep1 : (sp_send_step PaxosVariant.guarded f st
        (qlogs, b', leader, pays)).1
        = ⟨spBase f st qlogs b' leader
            + (if leader then pays else []).length,
          if spRecEn st qlogs b' leader then some b' else st.recommittedAt,
          leader⟩ := by
      rw [sp_send_step_guarded_eq]
    -- the fresh keys of this tick
    have hnewkeys : ((if leader then
        (enumFrom (spBase f st qlogs b' leader) pays).map
          (fun sp => ((sp.1, b'), some sp.2))
          ++ (spRm f st qlogs b' leader).1
      else []) : List ((Nat × Ballot nP) × Option P)).map (·.1)
        = (if leader then
            (enumFrom (spBase f st qlogs b' leader) pays).map
              (fun sp => (sp.1, b'))
            ++ ((spRm f st qlogs b' leader).1).map (·.1)
          else []) := by
      by_cases hl : leader
      · subst hl
        rw [if_pos rfl, if_pos rfl, List.map_append, List.map_map]
        rfl
      · have : leader = false := Bool.eq_false_iff.mpr hl
        subst this
        rfl
    rw [hnewkeys]
    -- ── structure of the fresh keys
    have hfresh_nodup : ((enumFrom (spBase f st qlogs b' leader) pays).map
        (fun sp => ((sp.1, b') : Nat × Ballot nP))).Nodup := by
      have hlt := enumFrom_pairwise_lt (spBase f st qlogs b' leader) pays
      rw [List.pairwise_map] at hlt
      show ((enumFrom (spBase f st qlogs b' leader) pays).map
        (fun sp => ((sp.1, b') : Nat × Ballot nP))).Pairwise (· ≠ ·)
      rw [List.pairwise_map]
      refine hlt.imp ?_
      exact fun {a c} h heq => by
        have := congrArg Prod.fst heq
        dsimp only at this
        omega
    have hfresh_mem : ∀ key ∈ (enumFrom (spBase f st qlogs b' leader)
        pays).map (fun sp => ((sp.1, b') : Nat × Ballot nP)),
        (spBase f st qlogs b' leader ≤ key.1
          ∧ key.1 < spBase f st qlogs b' leader + pays.length)
          ∧ key.2 = b' := by
      intro key hk
      obtain ⟨sp, hsp, rfl⟩ := List.mem_map.mp hk
      exact ⟨fst_mem_enumFrom hsp, rfl⟩
    -- ── structure of the recommit keys
    have hrm_ballot : ∀ key ∈ ((spRm f st qlogs b' leader).1).map (·.1),
        (key : Nat × Ballot nP).2 = b' := by
      intro key hk
      obtain ⟨e, he, rfl⟩ := List.mem_map.mp hk
      unfold spRm at he
      by_cases hre : spRecEn st qlogs b' leader
      · rw [if_pos hre] at he
        exact recommit_ballot e he
      · rw [if_neg hre] at he
        cases he
    have hrm_lt_base : ∀ key ∈ ((spRm f st qlogs b' leader).1).map (·.1),
        (key : Nat × Ballot nP).1 < spBase f st qlogs b' leader := by
      intro key hk
      obtain ⟨e, he, rfl⟩ := List.mem_map.mp hk
      by_cases hre : spRecEn st qlogs b' leader
      · have hrm : spRm f st qlogs b' leader
            = recommitAfterLeaderElection f qlogs b' := by
          unfold spRm
          rw [if_pos hre]
        rw [hrm] at he
        cases hms : (recommitAfterLeaderElection f qlogs b').2 with
        | none =>
          rw [recommit_none hms] at he
          cases he
        | some ms =>
          have hb := recommit_slot_le hms e he
          show e.1.1 < spBase f st qlogs b' leader
          unfold spBase
          rw [hrm, hms]
          show e.1.1 < ms + 1
          omega
      · have hrm : spRm f st qlogs b' leader
            = (([], none) : List ((Nat × Ballot nP) × Option P) × Option Nat) := by
          unfold spRm
          rw [if_neg hre]
        rw [hrm] at he
        cases he
    have hrm_nodup : (((spRm f st qlogs b' leader).1).map (·.1)).Nodup := by
      unfold spRm
      by_cases hre : spRecEn st qlogs b' leader
      · rw [if_pos hre]
        have hnd := recommit_slots_nodup (f := f) (qlogs := qlogs) (b := b')
        -- keys nodup from slot nodup + constant ballot
        have : ((recommitAfterLeaderElection f qlogs b').1.map (·.1)).map
            Prod.fst = (recommitAfterLeaderElection f qlogs b').1.map
              (fun e => e.1.1) := by
          rw [List.map_map]
          rfl
        refine List.Nodup.of_map (f := Prod.fst) ?_
        rw [this]
        exact hnd
      · rw [if_neg hre]
        exact List.nodup_nil
    -- ── all new keys: ballot b', slot < nextSlot'
    have hnew_ballot : ∀ key ∈ (if leader then
        (enumFrom (spBase f st qlogs b' leader) pays).map
          (fun sp => ((sp.1, b') : Nat × Ballot nP))
        ++ ((spRm f st qlogs b' leader).1).map (·.1) else []),
        (key : Nat × Ballot nP).2 = b' := by
      intro key hk
      by_cases hl : leader
      · subst hl
        rw [if_pos rfl] at hk
        rcases List.mem_append.mp hk with hk | hk
        · exact (hfresh_mem key hk).2
        · exact hrm_ballot key hk
      · have : leader = false := Bool.eq_false_iff.mpr hl
        subst this
        cases hk
    have hnew_lt : ∀ key ∈ (if leader then
        (enumFrom (spBase f st qlogs b' leader) pays).map
          (fun sp => ((sp.1, b') : Nat × Ballot nP))
        ++ ((spRm f st qlogs b' leader).1).map (·.1) else []),
        (key : Nat × Ballot nP).1 < spBase f st qlogs b' leader
          + (if leader then pays else []).length := by
      intro key hk
      by_cases hl : leader
      · subst hl
        rw [if_pos rfl] at hk ⊢
        rcases List.mem_append.mp hk with hk | hk
        · exact (hfresh_mem key hk).1.2
        · exact Nat.lt_of_lt_of_le (hrm_lt_base key hk) (Nat.le_add_right _ _)
      · have : leader = false := Bool.eq_false_iff.mpr hl
        subst this
        cases hk
    have hnew_nodup : ((if leader then
        (enumFrom (spBase f st qlogs b' leader) pays).map
          (fun sp => ((sp.1, b') : Nat × Ballot nP))
        ++ ((spRm f st qlogs b' leader).1).map (·.1) else []) :
        List (Nat × Ballot nP)).Nodup := by
      by_cases hl : leader
      · subst hl
        rw [if_pos rfl]
        refine List.nodup_append.mpr ⟨hfresh_nodup, hrm_nodup, ?_⟩
        intro k1 hk1 k2 hk2 hne
        subst hne
        have h1 := (hfresh_mem k1 hk1).1.1
        have h2 := hrm_lt_base k1 hk2
        omega
      · have : leader = false := Bool.eq_false_iff.mpr hl
        subst this
        exact List.nodup_nil
    -- ── no old key collides with a new key
    have hdisj : ∀ key ∈ oldKeys, ∀ key' ∈ (if leader then
        (enumFrom (spBase f st qlogs b' leader) pays).map
          (fun sp => ((sp.1, b') : Nat × Ballot nP))
        ++ ((spRm f st qlogs b' leader).1).map (·.1) else []),
        key ≠ key' := by
      intro key hk key' hk' heq
      subst heq
      have hb' := hnew_ballot key hk'
      have hnum := (hball key hk).1
      rw [hb'] at hnum
      -- the old key is at the current ballot: its num equals bn = b'.num
      have hbn : b'.num = bn := Nat.le_antisymm hnum hle
      have hkey2 : key.2 = b' := hb'
      -- so the state has recommittedAt = some key.2 and slot < nextSlot
      have hnumbn : key.2.num = bn := by rw [hkey2, hbn]
      obtain ⟨hslt, hsrec⟩ := hslot key hk hnumbn
      -- new keys need `leader`; case on the recommit gate
      by_cases hl : leader
      · subst hl
        by_cases hre : spRecEn st qlogs b' true
        · -- recommit ran ⇒ recommittedAt ≠ some b', but it IS some key.2 = b'
          unfold spRecEn spJustB at hre
          rw [Bool.and_eq_true, Bool.and_eq_true] at hre
          have hne := hre.2.2
          rw [decide_eq_true_iff] at hne
          rw [hkey2] at hsrec
          exact hne hsrec
        · -- no recommit ⇒ base = nextSlot ⇒ new slots ≥ nextSlot > old slot
          have hbase : spBase f st qlogs b' true = st.nextSlot := by
            unfold spBase spRm
            rw [if_neg hre]
            rfl
          have hlt2 : key.1 < st.nextSlot := hslt
          rw [if_pos rfl] at hk'
          rcases List.mem_append.mp hk' with hk' | hk'
          · have := (hfresh_mem key hk').1.1
            rw [hbase] at this
            omega
          · -- recommit keys are empty without the gate
            unfold spRm at hk'
            rw [if_neg hre] at hk'
            cases hk'
      · have : leader = false := Bool.eq_false_iff.mpr hl
        subst this
        cases hk'
    -- ── conditions for the recursive call
    have hql : leader = true → qlogs ≠ [] := hqne
    have hrec' : leader = true →
        (if spRecEn st qlogs b' leader then some b' else st.recommittedAt)
          = some ⟨b'.num, me⟩ := by
      intro hl
      subst hl
      have hbeq : b' = ⟨b'.num, me⟩ := by
        refine Ballot.ext' rfl hown
      by_cases hre : spRecEn st qlogs b' true
      · rw [if_pos hre, ← hbeq]
      · rw [if_neg hre]
        unfold spRecEn spJustB at hre
        have hqne' : (!qlogs.isEmpty) = true := by
          have := hql rfl
          cases hq : qlogs with
          | nil => exact absurd hq this
          | cons a l => rfl
        rw [Bool.and_eq_true, Bool.and_eq_true] at hre
        by_cases hjb : (!st.wasLeader) = true
        · -- just became leader with recommittedAt = some b' already
          by_cases hnrec : st.recommittedAt = some b'
          · rw [hnrec, ← hbeq]
          · exfalso
            refine hre ⟨hqne', ?_, decide_eq_true hnrec⟩
            rw [hjb]
            rfl
        · -- continuing leader: the stability clause pins the ballot
          have hwas : st.wasLeader = true := by
            cases hw : st.wasLeader
            · rw [hw] at hjb
              exact absurd rfl hjb
            · rfl
          have hwl' : wl = true := by rw [← hwl, hwas]
          have hbb := hstable rfl hwl'
          have hra := hrec hwl'
          rw [hbb, hra]
    have hrecle' : ∀ b, (if spRecEn st qlogs b' leader then some b'
          else st.recommittedAt) = some b →
        b.num ≤ b'.num ∧ b.proposerId = me := by
      intro b hb
      by_cases hre : spRecEn st qlogs b' leader
      · rw [if_pos hre] at hb
        cases Option.some.inj hb
        exact ⟨Nat.le_refl _, hown⟩
      · rw [if_neg hre] at hb
        have := hrecle b hb
        exact ⟨Nat.le_trans this.1 hle, this.2⟩
    have hball' : ∀ key ∈ oldKeys ++ (if leader then
        (enumFrom (spBase f st qlogs b' leader) pays).map
          (fun sp => ((sp.1, b') : Nat × Ballot nP))
        ++ ((spRm f st qlogs b' leader).1).map (·.1) else []),
        (key : Nat × Ballot nP).2.num ≤ b'.num
          ∧ key.2.proposerId = me := by
      intro key hk
      rcases List.mem_append.mp hk with hk | hk
      · have := hball key hk
        exact ⟨Nat.le_trans this.1 hle, this.2⟩
      · have := hnew_ballot key hk
        rw [this]
        exact ⟨Nat.le_refl _, hown⟩
    have hslot' : ∀ key ∈ oldKeys ++ (if leader then
        (enumFrom (spBase f st qlogs b' leader) pays).map
          (fun sp => ((sp.1, b') : Nat × Ballot nP))
        ++ ((spRm f st qlogs b' leader).1).map (·.1) else []),
        (key : Nat × Ballot nP).2.num = b'.num →
        key.1 < spBase f st qlogs b' leader
            + (if leader then pays else []).length
          ∧ (if spRecEn st qlogs b' leader then some b'
              else st.recommittedAt) = some key.2 := by
      intro key hk hknum
      rcases List.mem_append.mp hk with hk | hk
      · -- an old key at the current num: bn = b'.num, and (by `hdisj`
        -- analysis) the gate is closed and the base is the old nextSlot
        have hnum := (hball key hk).1
        have hbn : b'.num = bn := Nat.le_antisymm (hknum ▸ hnum) hle
        obtain ⟨hslt, hsrec⟩ := hslot key hk (by rw [hknum, hbn])
        have hkey2 : key.2 = b' := by
          refine Ballot.ext' hknum.symm ?_ |>.symm
          rw [hown, (hball key hk).2]
        by_cases hre : spRecEn st qlogs b' leader
        · -- gate open contradicts recommittedAt = some key.2 = some b'
          exfalso
          unfold spRecEn spJustB at hre
          rw [Bool.and_eq_true, Bool.and_eq_true] at hre
          have hne := hre.2.2
          rw [decide_eq_true_iff] at hne
          rw [hkey2] at hsrec
          exact hne hsrec
        · have hbase : spBase f st qlogs b' leader = st.nextSlot := by
            unfold spBase spRm
            rw [if_neg hre]
            rfl
          rw [if_neg hre, hbase]
          exact ⟨Nat.lt_of_lt_of_le hslt (Nat.le_add_right _ _),
            by rw [hkey2] at hsrec ⊢; exact hsrec⟩
      · -- a new key: below the new next slot, recommitted at b'
        refine ⟨hnew_lt key hk, ?_⟩
        have hkb := hnew_ballot key hk
        -- new keys exist only when leading
        by_cases hl : leader
        · subst hl
          have := hrec' rfl
          rw [this, hkb]
          have hbeq : b' = ⟨b'.num, me⟩ := Ballot.ext' rfl hown
          rw [← hbeq]
        · have : leader = false := Bool.eq_false_iff.mpr hl
          subst this
          cases hk
    -- ── recurse
    have hrecurse := spKeys_inv f me xs b'.num leader
      ⟨spBase f st qlogs b' leader + (if leader then pays else []).length,
        if spRecEn st qlogs b' leader then some b' else st.recommittedAt,
        leader⟩
      (oldKeys ++ (if leader then
        (enumFrom (spBase f st qlogs b' leader) pays).map
          (fun sp => ((sp.1, b') : Nat × Ballot nP))
        ++ ((spRm f st qlogs b' leader).1).map (·.1) else []))
      hchain' rfl hrec' hrecle'
      (by
        refine List.nodup_append.mpr ⟨hnd, hnew_nodup, ?_⟩
        intro k1 hk1 k2 hk2
        exact hdisj k1 hk1 k2 hk2)
      hball' hslot'
    constructor
    · rw [← List.append_assoc]
      exact hrecurse.1
    · intro key hk
      rcases List.mem_append.mp hk with hk | hk
      · have := hnew_ballot key hk
        rw [this]
        exact hown
      · exact hrecurse.2 key hk

/-! ## The covered-slot discipline (guarded variant)

The second half of the P2a discipline: `spKeys_inv` says keys never repeat;
`spCovered_inv` says *where values live* — an emitted key whose slot is
covered by the merge of its ballot's (frozen) quorum view carries the merged
max-ballot entry's value. Fresh payloads never land on covered slots
(the recommit tick's slot base governs every later fresh slot of the same
leadership) and holes are never covered. -/

/-- `SPChain` plus the frozen-bucket pinning clause: at leader ticks the
quorum view IS the cut-independent bucket `Q` of the tick's ballot number —
supplied by `leader_election`'s `pinned` contract (frozen quorum buckets)
at instantiation. -/
inductive SPChainQ (me : Fin nP) (Q : Nat → List (P1bPayload P nP)) :
    Nat → Bool → List (SPTick P nP) → Prop
  | nil {bn : Nat} {wl : Bool} : SPChainQ me Q bn wl []
  | cons {bn : Nat} {wl : Bool} {x : SPTick P nP} {xs : List (SPTick P nP)} :
      x.2.1.proposerId = me → bn ≤ x.2.1.num →
      (x.2.2.1 = true → x.1 ≠ []) →
      (x.2.2.1 = true → wl = true → x.2.1 = ⟨bn, me⟩) →
      (x.2.2.1 = true → x.1 = Q x.2.1.num) →
      SPChainQ me Q x.2.1.num x.2.2.1 xs →
      SPChainQ me Q bn wl (x :: xs)

/-- Forget the pinning clause. -/
theorem SPChainQ.toChain {me : Fin nP} {Q : Nat → List (P1bPayload P nP)} :
    ∀ {bn : Nat} {wl : Bool} {ins : List (SPTick P nP)},
      SPChainQ me Q bn wl ins → SPChain me bn wl ins
  | _, _, _, .nil => .nil
  | _, _, _, .cons h1 h2 h3 h4 _ h6 => .cons h1 h2 h3 h4 h6.toChain

/-- The merge-covered slots of ballot number `n`'s pinned quorum view. -/
def spCoveredSlots (Q : Nat → List (P1bPayload P nP)) (n : Nat) : List Nat :=
  (recommitMerged (P := P) (nP := nP) (Q n)).map Prod.fst

set_option maxHeartbeats 2000000 in
/-- **The covered-slot discipline** (the second guarded scan invariant):
over any pinned-chain input, every emitted `((slot, ballot), value)` whose
slot is covered by the merge of the ballot's pinned quorum view quotes the
merged max-ballot entry (report count ≤ f, value equal). -/
theorem spCovered_inv (f : Nat) (me : Fin nP)
    (Q : Nat → List (P1bPayload P nP)) :
    ∀ (ins : List (SPTick P nP)) (bn : Nat) (wl : Bool) (st : SPSendSt nP),
    SPChainQ me Q bn wl ins →
    st.wasLeader = wl →
    (wl = true → st.recommittedAt = some ⟨bn, me⟩) →
    (∀ b, st.recommittedAt = some b → b.num ≤ bn ∧ b.proposerId = me) →
    (∀ b, st.recommittedAt = some b →
      ∀ s ∈ spCoveredSlots Q b.num, s < st.nextSlot) →
    ∀ e ∈ (scan (sp_send_step PaxosVariant.guarded f) st ins).flatten,
      (e : (Nat × Ballot nP) × Option P).1.1 ∈ spCoveredSlots Q e.1.2.num →
      ∃ cnt entry,
        (e.1.1, (cnt, entry)) ∈ recommitMerged (P := P) (Q e.1.2.num) ∧
        cnt ≤ f ∧ e.2 = entry.value
  | [], bn, wl, st, _, _, _, _, _ => by
    intro e he
    cases he
  | x :: xs, bn, wl, st, hchain, hwl, hrec, hrecle, hcov => by
    obtain ⟨qlogs, b', leader, pays⟩ := x
    cases hchain with
    | cons hown0 hle0 hqne0 hstable0 hpin0 hchain' =>
    have hown : b'.proposerId = me := hown0
    have hle : bn ≤ b'.num := hle0
    have hqne : leader = true → qlogs ≠ [] := hqne0
    have hstable : leader = true → wl = true → b' = ⟨bn, me⟩ := hstable0
    have hpin : leader = true → qlogs = Q b'.num := hpin0
    intro e he hcovered
    rw [scan_cons, List.flatten_cons, sp_send_step_guarded_eq] at he
    -- the gate-closed leader tick pins recommittedAt at the current ballot
    have hgate : leader = true → spRecEn st qlogs b' leader = false →
        st.recommittedAt = some b' := by
      intro hl hre
      subst hl
      unfold spRecEn at hre
      have hqne' : (!qlogs.isEmpty) = true := by
        cases hq : qlogs with
        | nil => exact absurd hq (hqne rfl)
        | cons a l => rfl
      rw [hqne', Bool.true_and] at hre
      by_cases hjb : spJustB st true = true
      · rw [hjb, Bool.true_and] at hre
        have := of_decide_eq_false hre
        exact Classical.byContradiction fun hc => (this fun hh => hc hh).elim
      · have hwas : st.wasLeader = true := by
          unfold spJustB at hjb
          cases hw : st.wasLeader
          · rw [hw] at hjb
            exact absurd rfl hjb
          · rfl
        have hwl' : wl = true := by rw [← hwl, hwas]
        have hbb := hstable rfl hwl'
        rw [hbb]
        exact hrec hwl'
    -- the open gate computes the base past every covered slot
    have hbase_open : spRecEn st qlogs b' leader = true →
        ∀ s ∈ spCoveredSlots Q b'.num, s < spBase f st qlogs b' leader := by
      intro hre s hs
      have hl : leader = true := by
        unfold spRecEn spJustB at hre
        rw [Bool.and_eq_true, Bool.and_eq_true, Bool.and_eq_true] at hre
        exact hre.2.1.1
      have hql : qlogs = Q b'.num := hpin hl
      rw [show spCoveredSlots Q b'.num
          = (recommitMerged (P := P) (nP := nP) qlogs).map Prod.fst from by
        rw [spCoveredSlots, hql]] at hs
      obtain ⟨m, hm, hsm⟩ := recommitMax_mem_le hs
      have hb : spBase f st qlogs b' leader = m + 1 := by
        unfold spBase spRm
        rw [if_pos hre]
        rw [show (recommitAfterLeaderElection f qlogs b').2
            = recommitMax (P := P) (nP := nP) qlogs from by
          rw [recommit_eq]]
        rw [hm]
        rfl
      omega
    -- the closed gate keeps the base at nextSlot, past covered slots
    have hbase_closed : leader = true →
        spRecEn st qlogs b' leader = false →
        ∀ s ∈ spCoveredSlots Q b'.num, s < spBase f st qlogs b' leader := by
      intro hl hre s hs
      have hra := hgate hl hre
      have hslt := hcov b' hra s hs
      have hb : spBase f st qlogs b' leader = st.nextSlot := by
        unfold spBase spRm
        rw [if_neg (by rw [hre]; exact Bool.false_ne_true)]
        rfl
      omega
    rcases List.mem_append.mp he with hhead | htail
    · -- emitted this tick
      by_cases hl : leader
      · subst hl
        rw [if_pos rfl] at hhead
        rcases List.mem_append.mp hhead with hfresh | hrm
        · -- fresh payload on a covered slot: impossible
          exfalso
          obtain ⟨sp, hsp, rfl⟩ := List.mem_map.mp hfresh
          have hge := fst_mem_enumFrom hsp
          have hcv : sp.1 ∈ spCoveredSlots Q b'.num := hcovered
          by_cases hre : spRecEn st qlogs b' true
          · have := hbase_open hre sp.1 hcv
            omega
          · have := hbase_closed rfl (Bool.eq_false_iff.mpr hre) sp.1 hcv
            omega
        · -- recommit emission
          have hre : spRecEn st qlogs b' true = true := by
            by_cases hre : spRecEn st qlogs b' true
            · exact hre
            · exfalso
              unfold spRm at hrm
              rw [if_neg hre] at hrm
              cases hrm
          have hql : qlogs = Q b'.num := hpin rfl
          have hrm' : e ∈ (recommitAfterLeaderElection f qlogs b').1 := by
            unfold spRm at hrm
            rwa [if_pos hre] at hrm
          rw [recommit_eq] at hrm'
          rcases List.mem_append.mp hrm' with hc | hh
          · obtain ⟨cnt, entry, hmem, hcnt, hball, hval⟩ :=
              recommitToCommit_mem hc
            refine ⟨cnt, entry, ?_, hcnt, hval⟩
            rw [hball, ← hql]
            exact hmem
          · exfalso
            have hnc := recommitHoles_not_covered hh
            have hball : e.1.2 = b' := by
              revert hh
              unfold recommitHoles
              cases recommitMax (P := P) (nP := nP) qlogs with
              | none => intro hh; cases hh
              | some m =>
                intro hh
                obtain ⟨s, -, rfl⟩ := List.mem_map.mp hh
                rfl
            rw [hball, spCoveredSlots, ← hql] at hcovered
            exact hnc hcovered
      · have : leader = false := Bool.eq_false_iff.mpr hl
        subst this
        rw [if_neg Bool.false_ne_true] at hhead
        cases hhead
    · -- emitted later: recurse with the updated state
      -- recommittedAt update facts (as in `spKeys_inv`)
      have hrec' : leader = true →
          (if spRecEn st qlogs b' leader then some b' else st.recommittedAt)
            = some ⟨b'.num, me⟩ := by
        intro hl
        subst hl
        have hbeq : b' = ⟨b'.num, me⟩ := Ballot.ext' rfl hown
        by_cases hre : spRecEn st qlogs b' true
        · rw [if_pos hre, ← hbeq]
        · rw [if_neg hre]
          rw [hgate rfl (Bool.eq_false_iff.mpr hre), ← hbeq]
      have hrecle' : ∀ b, (if spRecEn st qlogs b' leader then some b'
            else st.recommittedAt) = some b →
          b.num ≤ b'.num ∧ b.proposerId = me := by
        intro b hb
        by_cases hre : spRecEn st qlogs b' leader
        · rw [if_pos hre] at hb
          cases Option.some.inj hb
          exact ⟨Nat.le_refl _, hown⟩
        · rw [if_neg hre] at hb
          have := hrecle b hb
          exact ⟨Nat.le_trans this.1 hle, this.2⟩
      have hcov' : ∀ b, (if spRecEn st qlogs b' leader then some b'
            else st.recommittedAt) = some b →
          ∀ s ∈ spCoveredSlots Q b.num,
            s < spBase f st qlogs b' leader
              + (if leader then pays else []).length := by
        intro b hb s hs
        by_cases hre : spRecEn st qlogs b' leader
        · rw [if_pos hre] at hb
          cases Option.some.inj hb
          have := hbase_open hre s hs
          omega
        · rw [if_neg hre] at hb
          have hslt := hcov b hb s hs
          have hb' : st.nextSlot ≤ spBase f st qlogs b' leader := by
            unfold spBase spRm
            rw [if_neg hre]
            exact Nat.le_refl _
          omega
      exact spCovered_inv f me Q xs b'.num leader
        ⟨spBase f st qlogs b' leader + (if leader then pays else []).length,
          if spRecEn st qlogs b' leader then some b' else st.recommittedAt,
          leader⟩
        hchain' rfl hrec' hrecle' hcov' e htail hcovered

/-- Build the pinned chain from per-tick indexed facts (the form the run
instantiation produces). -/
theorem SPChainQ_of_getElem (me : Fin nP) (Q : Nat → List (P1bPayload P nP)) :
    ∀ (ins : List (SPTick P nP)) (bn : Nat) (wl : Bool),
    (∀ t (ht : t < ins.length), (ins[t]'ht).2.1.proposerId = me) →
    (∀ t (ht : t < ins.length) t' (ht' : t' < ins.length), t ≤ t' →
      (ins[t]'ht).2.1.num ≤ (ins[t']'ht').2.1.num) →
    (∀ h0 : 0 < ins.length, bn ≤ (ins[0]'h0).2.1.num) →
    (∀ t (ht : t < ins.length), (ins[t]'ht).2.2.1 = true →
      (ins[t]'ht).1 ≠ []) →
    (∀ t (ht : t + 1 < ins.length),
      (ins[t + 1]'ht).2.2.1 = true →
      (ins[t]'(Nat.lt_of_succ_lt ht)).2.2.1 = true →
      (ins[t + 1]'ht).2.1 = (ins[t]'(Nat.lt_of_succ_lt ht)).2.1) →
    (∀ h0 : 0 < ins.length, (ins[0]'h0).2.2.1 = true → wl = true →
      (ins[0]'h0).2.1 = ⟨bn, me⟩) →
    (∀ t (ht : t < ins.length), (ins[t]'ht).2.2.1 = true →
      (ins[t]'ht).1 = Q (ins[t]'ht).2.1.num) →
    SPChainQ me Q bn wl ins
  | [], bn, wl, _, _, _, _, _, _, _ => SPChainQ.nil
  | x :: xs, bn, wl, hown, hmono, hbn, hqne, hstab, hstab0, hpin => by
    have h0 : 0 < (x :: xs).length := Nat.succ_pos _
    refine SPChainQ.cons (hown 0 h0) (hbn h0) (hqne 0 h0) (hstab0 h0)
      (hpin 0 h0) ?_
    refine SPChainQ_of_getElem me Q xs x.2.1.num x.2.2.1
      (fun t ht => hown (t + 1) (Nat.succ_lt_succ ht))
      (fun t ht t' ht' hle => hmono (t + 1) (Nat.succ_lt_succ ht) (t' + 1)
        (Nat.succ_lt_succ ht') (Nat.succ_le_succ hle))
      (fun hx0 => hmono 0 h0 1 (Nat.succ_lt_succ hx0) (Nat.zero_le 1))
      (fun t ht => hqne (t + 1) (Nat.succ_lt_succ ht))
      (fun t ht => hstab (t + 1) (Nat.succ_lt_succ ht))
      (fun hx0 hl hwl => ?_)
      (fun t ht => hpin (t + 1) (Nat.succ_lt_succ ht))
    -- head stability of the tail = the consecutive clause at 0 + ownership
    have := hstab 0 (Nat.succ_lt_succ hx0) hl hwl
    rw [show ((x :: xs)[1]'(Nat.succ_lt_succ hx0)) = xs[0]'hx0 from rfl] at this
    rw [this]
    exact Ballot.ext' rfl (hown 0 h0)

/-- Every guarded emission happens at a leader tick and carries the tick's
ballot. -/
theorem sp_send_step_out_ballot {f : Nat} {st : SPSendSt nP}
    {x : SPTick P nP} {e : (Nat × Ballot nP) × Option P}
    (he : e ∈ (sp_send_step PaxosVariant.guarded f st x).2) :
    x.2.2.1 = true ∧ e.1.2 = x.2.1 := by
  obtain ⟨qlogs, b', leader, pays⟩ := x
  rw [sp_send_step_guarded_eq] at he
  by_cases hl : leader
  · subst hl
    rw [if_pos rfl] at he
    refine ⟨rfl, ?_⟩
    rcases List.mem_append.mp he with hf | hr
    · obtain ⟨sp, -, rfl⟩ := List.mem_map.mp hf
      rfl
    · unfold spRm at hr
      by_cases hre : spRecEn st qlogs b' true
      · rw [if_pos hre] at hr
        exact recommit_ballot e hr
      · rw [if_neg hre] at hr
        cases hr
  · have : leader = false := Bool.eq_false_iff.mpr hl
    subst this
    rw [if_neg Bool.false_ne_true] at he
    cases he

/-! ## Module faces at the standard start state

The eliminators the wiring layer composes: every fact below is stated over
the module's own inputs (`ins`, a pinned bucket map `Q`, the typed carrier
`SPG`), with the input requirements as named hypotheses — never over a
particular run. -/

/-- **Emission opening (module face)**: every guarded emission happens at a
leader tick and carries the tick's ballot (indexed form of
`sp_send_step_out_ballot`). -/
theorem sp_emit_elim {f : Nat} {st : SPSendSt nP} {ins : List (SPTick P nP)}
    {e : (Nat × Ballot nP) × Option P}
    (he : e ∈ (scan (sp_send_step PaxosVariant.guarded f) st ins).flatten) :
    ∃ t, ∃ ht : t < ins.length,
      (ins[t]'ht).2.2.1 = true ∧ e.1.2 = (ins[t]'ht).2.1 := by
  obtain ⟨t, ht, hstep⟩ := mem_scan_flatten_elim he
  obtain ⟨hl, hb⟩ := sp_send_step_out_ballot hstep
  exact ⟨t, ht, hl, hb⟩

/-- **The key discipline at the start state (module face)**: over any
chain-respecting input from the initial state, the emitted `(slot, ballot)`
keys are duplicate-free and owned (`spKeys_inv`, side conditions
discharged). -/
theorem sp_keys_face (f : Nat) (me : Fin nP) {ins : List (SPTick P nP)}
    (hchain : SPChain me 0 false ins) :
    (spKeys f ⟨0, none, false⟩ ins).Nodup ∧
    ∀ key ∈ spKeys f ⟨0, none, false⟩ ins, key.2.proposerId = me := by
  have h := spKeys_inv f me ins 0 false ⟨0, none, false⟩ [] hchain rfl
    (fun hw => absurd hw Bool.false_ne_true)
    (fun b hb => nomatch hb) List.nodup_nil
    (fun key hk => nomatch hk) (fun key hk => nomatch hk)
  exact ⟨by simpa using h.1, h.2⟩

/-- **The covered-value discipline at the start state (module face)**: over
any pinned chain from the initial state, an emission whose slot appears in
some member log of its ballot's pinned view carries the view's merged
max-ballot entry — the entry dominates any such witness and itself lives in
a member log of the same view (`spCovered_inv` + the merge spec, side
conditions discharged). -/
theorem sp_covered_value (f : Nat) (me : Fin nP)
    {Q : Nat → List (P1bPayload P nP)} {ins : List (SPTick P nP)}
    (hchain : SPChainQ me Q 0 false ins)
    {e : (Nat × Ballot nP) × Option P}
    (he : e ∈ (scan (sp_send_step PaxosVariant.guarded f)
      ⟨0, none, false⟩ ins).flatten)
    {e₀ : LogValue P nP}
    (hE : (e.1.1, e₀) ∈ ((Q e.1.2.num).map Prod.snd).flatten) :
    ∃ best : LogValue P nP,
      e.2 = best.value ∧ e₀.ballot.ble best.ballot = true ∧
      (e.1.1, best) ∈ ((Q e.1.2.num).map Prod.snd).flatten := by
  obtain ⟨cnt, best, hbestmem, hbestdom⟩ :=
    mergeQuorumLogs_covers ((Q e.1.2.num).map Prod.snd) hE
  have hcovered : e.1.1 ∈ spCoveredSlots Q e.1.2.num := by
    unfold spCoveredSlots
    refine List.mem_map.mpr ⟨(e.1.1, (cnt, best)), ?_, rfl⟩
    show (e.1.1, (cnt, best)) ∈ recommitMerged (P := P) (Q e.1.2.num)
    exact hbestmem
  obtain ⟨cnt', entry, hentmem, -, hval⟩ := spCovered_inv f me Q ins 0 false
    ⟨0, none, false⟩ hchain rfl
    (fun hw => absurd hw Bool.false_ne_true)
    (fun b hb => nomatch hb)
    (fun b hb => nomatch hb) e he hcovered
  have hentmem' : (e.1.1, (cnt', entry))
      ∈ mergeQuorumLogs ((Q e.1.2.num).map Prod.snd) := hentmem
  obtain ⟨-, hentbest⟩ := mergeQuorumLogs_slot_unique _ hentmem' hbestmem
  rw [hentbest] at hval
  obtain ⟨hbestflat, -⟩ := mergeQuorumLogs_max_ballot _ hbestmem
  exact ⟨best, hval, hbestdom, hbestflat⟩



/-! ## The module contract at the run

`spTicks` (the send-side tick-input trace of the carrier) and `spPin` (the
pinned quorum-view map) are **input-side vocabulary**: pure functions of
the input `g`, feeding the abstract witness `SPEmission`. Consumers see
only `SPEmission` / `SPChosen` (witness predicates over the module's
signature — the input carrier `g` and the output `out`) and the monotone
transports; the guarantees themselves ride `sequence_payload.ensures`.
The wire requirements a carrier must satisfy (`SPRequires`) are exactly
`leader_election`'s contracts. -/

section RunContract

variable (variant : PaxosVariant) (f : Nat) (nd : SPNondet P nP nA)
variable [DecidableEq P]
variable (g : SPG P nP nA)

/-- The send-side tick-input trace of proposer `i` at carrier `g` (the
per-tick `(p_relevant_p1bs, p_ballot, p_is_leader, payload batch)`
quadruples — input-side vocabulary; `SPEmission`'s trace). -/
def spTicks (i : Fin nP) : List (SPTick P nP) :=
  (TSing.zip (g.2.2.2.1 i)
    (TSing.zip (g.2.1 i)
      (TSing.zip (g.2.2.1 i)
        (batch (g.1 i) (nd.payloadBatch i))))).map
    (fun x => (x.1, x.2.1, x.2.2.1, x.2.2.2))


/-- The tick trace, opened into wire components. -/
theorem spTicks_getElem (i : Fin nP) {t : Nat}
    (ht : t < (spTicks nd g i).length) :
    ∃ (hpr : t < (g.2.2.2.1 i).length) (hpb : t < (g.2.1 i).length)
      (hpl : t < (g.2.2.1 i).length),
      ((spTicks nd g i)[t]'ht).1 = (g.2.2.2.1 i)[t]'hpr ∧
      ((spTicks nd g i)[t]'ht).2.1 = (g.2.1 i)[t]'hpb ∧
      ((spTicks nd g i)[t]'ht).2.2.1 = (g.2.2.1 i)[t]'hpl := by
  unfold spTicks at ht ⊢
  simp only [TSing.zip] at ht ⊢
  obtain ⟨ha, hb, hc, hd, hval⟩ := zip4_map_getElem _ _ _ _ ht
  exact ⟨ha, hb, hc, by rw [hval], by rw [hval], by rw [hval]⟩

/-- `sequence_payload`'s **requirements** on its input carrier, named —
exactly `leader_election`'s guarantees (`LEEnsures`): ballot ownership and
`num`-monotonicity, leader ticks see full views, ballot stability along
reigns, and view pinning by ballot number. Composition discharges these
by projection. -/
structure SPRequires (g : SPG P nP nA) : Prop where
  own : ∀ (i : Fin nP), ∀ b ∈ (g.2.1 i : TSing (Ballot nP)),
    b.proposerId = i
  mono : ∀ (i : Fin nP) {t t' : Nat} (h : t ≤ t')
    (ht' : t' < (g.2.1 i).length),
    ((g.2.1 i)[t]'(Nat.lt_of_le_of_lt h ht')).num ≤ ((g.2.1 i)[t']'ht').num
  lead_ne : ∀ (i : Fin nP) {t : Nat} (hpl : t < (g.2.2.1 i).length)
    (hpr : t < (g.2.2.2.1 i).length), (g.2.2.1 i)[t]'hpl = true →
    (g.2.2.2.1 i)[t]'hpr ≠ []
  stable : ∀ (i : Fin nP) {t : Nat} (ht1 : t + 1 < (g.2.2.1 i).length)
    (hb1 : t + 1 < (g.2.1 i).length),
    (g.2.2.1 i)[t + 1]'ht1 = true →
    (g.2.2.1 i)[t]'(Nat.lt_of_succ_lt ht1) = true →
    (g.2.1 i)[t + 1]'hb1 = (g.2.1 i)[t]'(Nat.lt_of_succ_lt hb1)
  pin : ∀ (i : Fin nP) {t t' : Nat} (hpl : t < (g.2.2.1 i).length)
    (hpl' : t' < (g.2.2.1 i).length)
    (hpr : t < (g.2.2.2.1 i).length) (hpr' : t' < (g.2.2.2.1 i).length)
    (hpb : t < (g.2.1 i).length) (hpb' : t' < (g.2.1 i).length),
    (g.2.2.1 i)[t]'hpl = true → (g.2.2.1 i)[t']'hpl' = true →
    ((g.2.1 i)[t]'hpb).num = ((g.2.1 i)[t']'hpb').num →
    (g.2.2.2.1 i)[t]'hpr = (g.2.2.2.1 i)[t']'hpr'

variable {g}

/-- Two leader ticks of the trace at the same ballot number see the same
view (device form of the pin). -/
theorem spTicks_qlogs_pin (hw : SPRequires g) (i : Fin nP)
    {t t' : Nat} (ht : t < (spTicks nd g i).length)
    (ht' : t' < (spTicks nd g i).length)
    (hl : ((spTicks nd g i)[t]'ht).2.2.1 = true)
    (hl' : ((spTicks nd g i)[t']'ht').2.2.1 = true)
    (hnum : ((spTicks nd g i)[t]'ht).2.1.num
      = ((spTicks nd g i)[t']'ht').2.1.num) :
    ((spTicks nd g i)[t]'ht).1 = ((spTicks nd g i)[t']'ht').1 := by
  obtain ⟨hpr, hpb, hpl, hv, hb, hfl⟩ := spTicks_getElem nd g i ht
  obtain ⟨hpr', hpb', hpl', hv', hb', hfl'⟩ := spTicks_getElem nd g i ht'
  rw [hv, hv']
  refine hw.pin i hpl hpl' hpr hpr' hpb hpb' ?_ ?_ ?_
  · rw [← hfl]; exact hl
  · rw [← hfl']; exact hl'
  · rw [← hb, ← hb']; exact hnum

/-- The pinned quorum-view map of proposer `i` (by ballot number;
well-defined by `spTicks_qlogs_pin`). Device. -/
noncomputable def spPin (i : Fin nP) : Nat → List (P1bPayload P nP) :=
  fun n =>
    if hex : ∃ t, ∃ ht : t < (spTicks nd g i).length,
        ((spTicks nd g i)[t]'ht).2.2.1 = true ∧
        ((spTicks nd g i)[t]'ht).2.1.num = n
    then ((spTicks nd g i)[hex.choose]'hex.choose_spec.choose).1
    else []

/-- Every leader tick's view IS its ballot's pinned view. -/
theorem spTicks_pinned (hw : SPRequires g) (i : Fin nP) {t : Nat}
    (ht : t < (spTicks nd g i).length)
    (hl : ((spTicks nd g i)[t]'ht).2.2.1 = true) :
    ((spTicks nd g i)[t]'ht).1
      = spPin nd (g := g) i (((spTicks nd g i)[t]'ht).2.1.num) := by
  unfold spPin
  have hex : ∃ t', ∃ ht' : t' < (spTicks nd g i).length,
      ((spTicks nd g i)[t']'ht').2.2.1 = true ∧
      ((spTicks nd g i)[t']'ht').2.1.num
        = ((spTicks nd g i)[t]'ht).2.1.num := ⟨t, ht, hl, rfl⟩
  rw [dif_pos hex]
  obtain ⟨hl₀, hn₀⟩ := hex.choose_spec.choose_spec
  exact spTicks_qlogs_pin nd hw i ht hex.choose_spec.choose hl hl₀ hn₀.symm

/-- The pinned chain over the tick trace (K3's `SPChainQ`, constructed
from the wire discipline). -/
theorem spTicks_chain (hw : SPRequires g) (i : Fin nP) :
    SPChainQ i (spPin nd (g := g) i) 0 false (spTicks nd g i) := by
  refine SPChainQ_of_getElem i _ _ 0 false
    (fun t ht => ?_) (fun t ht t' ht' hle => ?_) (fun _ => Nat.zero_le _)
    (fun t ht hl => ?_) (fun t ht hl hl' => ?_)
    (fun _ _ hw' => absurd hw' Bool.false_ne_true)
    (fun t ht hl => spTicks_pinned nd hw i ht hl)
  · obtain ⟨-, hpb, -, -, hb, -⟩ := spTicks_getElem nd g i ht
    rw [hb]
    exact hw.own i _ (List.getElem_mem hpb)
  · obtain ⟨-, hpb, -, -, hb, -⟩ := spTicks_getElem nd g i ht
    obtain ⟨-, hpb', -, -, hb', -⟩ := spTicks_getElem nd g i ht'
    rw [hb, hb']
    exact hw.mono i hle hpb'
  · obtain ⟨hpr, -, hpl, hv, -, hfl⟩ := spTicks_getElem nd g i ht
    rw [hv]
    refine hw.lead_ne i hpl hpr ?_
    rw [← hfl]
    exact hl
  · -- ballot stability along a reign (consecutive leader ticks)
    obtain ⟨-, hpb1, hpl1, -, hb1, hfl1⟩ := spTicks_getElem nd g i ht
    obtain ⟨-, hpb0, hpl0, -, hb0, hfl0⟩ :=
      spTicks_getElem nd g i (Nat.lt_of_succ_lt ht)
    rw [hb1, hb0]
    refine hw.stable i hpl1 hpb1 ?_ ?_
    · rw [← hfl1]; exact hl
    · rw [← hfl0]; exact hl'

/-! ### The abstract witnesses and the contracts (guarded) -/

omit [DecidableEq P] in
/-- The tick trace grows with the carrier (realized ticks are final). -/
theorem spTicks_prefix {g g' : SPG P nP nA} (hgg' : g ⊑ g') (i : Fin nP) :
    spTicks nd g i <+: spTicks nd g' i := by
  unfold spTicks
  exact (prefix_zipWith (hgg'.2.2.2.1 i)
    (prefix_zipWith (hgg'.2.1 i)
      (prefix_zipWith (hgg'.2.2.1 i)
        (batch_prefix (hgg'.1 i) _) Prod.mk) Prod.mk) Prod.mk).map _

variable (g) in
/-- An **emission** of proposer `i` at key `(slot, b)` with value `v` — the
signature-level shadow of a send-side P2a (membership in the send scan's
output over the input tick trace `spTicks`). Abstract witness: consumers
use only the exported facts about it, never its definition. -/
def SPEmission (i : Fin nP) (slot : Nat) (b : Ballot nP) (v : Option P) :
    Prop :=
  ((slot, b), v) ∈ (scan (sp_send_step variant f) ⟨0, none, false⟩
    (spTicks nd g i)).flatten

omit [DecidableEq P] in
/-- Emissions persist under carrier growth (realized ticks are final). -/
theorem SPEmission.mono {g' : SPG P nP nA} (hgg' : g ⊑ g')
    {i : Fin nP} {slot : Nat} {b : Ballot nP} {v : Option P}
    (h : SPEmission variant f nd g i slot b v) :
    SPEmission variant f nd g' i slot b v :=
  (prefix_flatten (scan_prefix _ _ (spTicks_prefix nd hgg' i))).subset h



variable (g) in
/-- A **chosen key** at carrier `g` with output `out`: `f + 1` distinct
acceptors each carry a vote — the `a_max_ballot` input wire holds exactly
the ballot at some tick whose published `a_log` output already covers the
slot at that ballot (write-before-ack, on the output wire). -/
def SPChosen (out : SPOut P nP nA) (slot : Nat) (b : Ballot nP) : Prop :=
  ∃ C : List (Fin nA), C.Nodup ∧ f + 1 ≤ C.length ∧
    ∀ j ∈ C, ∃ (t' : Nat) (hta : t' < (g.2.2.2.2 j).length)
      (htl : t' < (out.2.1 j).vals.length),
      (g.2.2.2.2 j)[t']'hta = some b ∧
      LogCovers (((out.2.1 j).vals[t']'htl).2) slot b

/-- Chosen keys persist under carrier/output growth (composition passes
the artifact's own monotonicity as `hout`). -/
theorem SPChosen.mono {g' : SPG P nP nA} {out out' : SPOut P nP nA}
    (hgg' : g ⊑ g') (hout : out ⊑ out')
    {slot : Nat} {b : Ballot nP}
    (h : SPChosen f g out slot b) : SPChosen f g' out' slot b := by
  obtain ⟨C, hnd', hlen, hvote⟩ := h
  refine ⟨C, hnd', hlen, fun j hj => ?_⟩
  obtain ⟨t', hta, htl, ham, hcov⟩ := hvote j hj
  have hamp : (g.2.2.2.2 j : TSing (Option (Ballot nP)))
      <+: g'.2.2.2.2 j := hgg'.2.2.2.2 j
  have hlogp : (out.2.1 j).vals <+: (out'.2.1 j).vals := hout.2.1 j
  have hta' : t' < (g'.2.2.2.2 j).length :=
    Nat.lt_of_lt_of_le hta hamp.length_le
  have htl' : t' < (out'.2.1 j).vals.length :=
    Nat.lt_of_lt_of_le htl hlogp.length_le
  refine ⟨t', hta', htl', ?_, ?_⟩
  · rw [← List.IsPrefix.getElem hamp hta]
    exact ham
  · rw [← List.IsPrefix.getElem hlogp htl]
    exact hcov


end RunContract

/-! ## The verified face: one artifact — dataflow, monotonicity, contract

`sequence_payload` is a `Verified` map: the Rust `let` chain inline (one
`let` per Rust `let`, in Rust order), prefix-monotone by type, with its
guarantees (`SPEnsures`) on the signature — stated over the input carrier
`g` and the output `out` themselves. Each guarantee carries its own named
preconditions (`SPRequires`, exactly `leader_election`'s guarantees) and
the bugfix-flag guard (`variant = .guarded → …`; the faithful variant's
type claims nothing). The proofs live inside the definition, as ghost
`have`s over the `let`-bound wires (`let` for data, `have` for proofs). -/

section Verified

variable (variant : PaxosVariant) (f : Nat) (nd : SPNondet P nP nA)
variable [DecidableEq P]
variable (g : SPG P nP nA)

/-- What `sequence_payload` **ensures**: the Rust output triple
`(p_to_replicas, a_log, sequencing_max_ballots)` against the carrier
`g`. One field per guarantee; each field's preconditions are its own. -/
structure SPEnsures (out : SPOut P nP nA) : Prop where
  /-- **Commit** (guarded): every emitted `(slot, value)` of
  `p_to_replicas` is an owned emission at a **chosen** key. -/
  commit_spec : SPRequires g → variant = .guarded → ∀ {i : Fin nP}
    {slot : Nat} {v : Option P}, (slot, v) ∈ out.1 i →
    ∃ b : Ballot nP, b.proposerId = i ∧
      SPEmission PaxosVariant.guarded f nd g i slot b v ∧
      SPChosen f g out slot b
  /-- **Emission opening** (guarded): an emission pins a leader tick of
  the input wires carrying its ballot; if its slot is covered by any
  member log of the tick's view, its value is the view's merged
  max-ballot entry. -/
  emission_spec : SPRequires g → variant = .guarded → ∀ {i : Fin nP}
    {slot : Nat} {b : Ballot nP} {v : Option P},
    SPEmission PaxosVariant.guarded f nd g i slot b v →
    ∃ (t : Nat) (hpl : t < (g.2.2.1 i).length)
      (hpb : t < (g.2.1 i).length) (hpr : t < (g.2.2.2.1 i).length),
      (g.2.2.1 i)[t]'hpl = true ∧
      (g.2.1 i)[t]'hpb = b ∧
      ∀ e₀ : LogValue P nP,
        (slot, e₀) ∈ (((g.2.2.2.1 i)[t]'hpr).map Prod.snd).flatten →
        ∃ best : LogValue P nP, v = best.value ∧
          e₀.ballot.ble best.ballot = true ∧
          (slot, best) ∈ (((g.2.2.2.1 i)[t]'hpr).map Prod.snd).flatten
  /-- **Log entry** (guarded): every entry of the published `a_log`
  output quotes an owned emission (write-before-ack provenance). -/
  log_entry_spec : SPRequires g → variant = .guarded → ∀ {j : Fin nA}
    {t : Nat} (htl : t < (out.2.1 j).vals.length) {slot : Nat}
    {e : LogValue P nP},
    (slot, e) ∈ (((out.2.1 j).vals[t]'htl).2 : LogMap P nP) →
    SPEmission PaxosVariant.guarded f nd g e.ballot.proposerId slot
      e.ballot e.value
  /-- **Emission functionality** (guarded): two emissions at the same
  key — one possibly at a smaller carrier — carry the same value. -/
  emission_functional : SPRequires g → variant = .guarded →
    ∀ {g₀ : SPG P nP nA}, g₀ ⊑ g →
    ∀ {i : Fin nP} {slot : Nat} {b : Ballot nP} {v v' : Option P},
    SPEmission PaxosVariant.guarded f nd g₀ i slot b v →
    SPEmission PaxosVariant.guarded f nd g i slot b v' →
    v = v'

set_option maxHeartbeats 1000000 in
/-- **paxos.rs:679–774 `sequence_payload`** — the single verified
artifact: the Rust `let` chain (the return tuple
`(p_to_replicas, a_log, sequencing_max_ballots)`; the fail stream in
keyed per-acceptor form; the `a_log` family at its coverage-`Monotonic`
wire type), with the contract paid on the signature and the proofs as
ghost `have`s inside. -/
def sequence_payload :
    Verified (SPG P nP nA) (SPOut P nP nA) (SPEnsures variant f nd) :=
  -- the input wires, destructured (the Rust parameter list)
  let c_to_proposers : SPG P nP nA →ₘ (Fin nP → Stream P) := MonoMap.fst
  let p_ballot : SPG P nP nA →ₘ (Fin nP → TSing (Ballot nP)) :=
    MonoMap.fst ∘ₘ MonoMap.snd
  let p_is_leader : SPG P nP nA →ₘ (Fin nP → TSing Bool) :=
    MonoMap.fst ∘ₘ MonoMap.snd ∘ₘ MonoMap.snd
  let p_relevant_p1bs : SPG P nP nA →ₘ (Fin nP → TStream (P1bPayload P nP)) :=
    MonoMap.fst ∘ₘ MonoMap.snd ∘ₘ MonoMap.snd ∘ₘ MonoMap.snd
  let a_max_ballot : SPG P nP nA →ₘ (Fin nA → TSing (Option (Ballot nP))) :=
    MonoMap.snd ∘ₘ MonoMap.snd ∘ₘ MonoMap.snd ∘ₘ MonoMap.snd
  -- let payloads_to_send = … (paxos.rs:706–734)
  let payloads_to_send := fun (i : Fin nP) =>
    (((p_relevant_p1bs.member i).zip ((p_ballot.member i).zip
        ((p_is_leader.member i).zip
          ((c_to_proposers.member i).batch (nd.payloadBatch i))))).map
      (fun x => (x.1, x.2.1, x.2.2.1, x.2.2.2))).scan
      (sp_send_step variant f) ⟨0, none, false⟩
  -- let p_to_acceptors_p2a = … (paxos.rs:736–749)
  let p_to_acceptors_p2a := fun (i : Fin nP) =>
    ((payloads_to_send i).flatten).map
      (fun sv => P2a.mk i sv.1.2 sv.1.1 sv.2)
  -- let (a_log, a_to_proposers_p2b) = acceptor_p2(…) (paxos.rs:751–753)
  let acceptor_p2_out := fun (j : Fin nA) =>
    acceptor_p2_ticks.toMonoMap ∘ₘ MonoMap.pair
      (((MonoMap.pi (fun i => p_to_acceptors_p2a i)).unionF).batchC
        (nd.p2aBatch j))
      (a_max_ballot.member j)
  let a_log := fun (j : Fin nA) => (acceptor_p2_out j).fstOf
  let a_to_proposers_p2b := fun (i : Fin nP) (j : Fin nA) =>
    (((acceptor_p2_out j).sndOf).flatten).filterMap
      (fun dm => if dm.1 = i then
        some ((dm.2.slot, dm.2.ballot), dm.2.res) else none)
  -- let (quorums, fails) = collect_quorum(…) (paxos.rs:755)
  let quorum := fun (i : Fin nP) =>
    collect_quorumM (f + 1) (2 * f + 1) (nd.quorumBatch i)
      ∘ₘ MonoMap.pi (fun j => a_to_proposers_p2b i j)
  -- let p_to_replicas = join_responses(quorums.map(k ↦ (k, ())),
  --   payloads_to_send.batch_atomic(…)) (paxos.rs:757–767), projected
  --   ((slot, _ballot), (value, _)) ↦ (slot, value) (:770)
  let p_to_replicas := fun (i : Fin nP) =>
    let respTicks :=
      ((((quorum i).fstOf).asCnt).batchC (nd.joinBatch i)).map
        (List.map (fun k => (k, ())))
    (((((payloads_to_send i).zip respTicks).map
        (fun mr => JoinTickIn.mk mr.1 mr.2)).scan
      (joinResponsesTick (Nat × Ballot nP) (Option P) Unit).step
        []).flatten).map
      (fun kmv => (kmv.1.1, kmv.2.1))
  Verified.ofMono
    (-- (p_to_replicas, a_log, sequencing_max_ballots) (paxos.rs:772–774)
     MonoMap.pair (MonoMap.pi (fun i => p_to_replicas i))
      (MonoMap.pair
        (MonoMap.pi (fun j => a_log j))
        (-- fails.flat_map_ordered(|(_, ballot)| ballot) (paxos.rs:772)
         MonoMap.pi (fun i => MonoMap.pi (fun j =>
          (((quorum i).sndOf).member j).filterMap (fun ke => ke.2))))))
    (fun g =>
      -- ghost values: the guarded traces at this carrier (`emitG i` is
      -- `SPEmission`'s witness stream; `b2G j` acceptor `j`'s consumed
      -- P2a batches)
      let emitG : Fin nP → TStream ((Nat × Ballot nP) × Option P) :=
        fun i => scan (sp_send_step PaxosVariant.guarded f) ⟨0, none, false⟩
          (spTicks nd g i)
      let p2asG : Fin nP → Stream (P2a P nP) :=
        fun i => (emitG i).flatten.map (fun sv => P2a.mk i sv.1.2 sv.1.1 sv.2)
      let b2G : Fin nA → TStream (P2a P nP) :=
        fun j => batchC (unionF p2asG) [] (nd.p2aBatch j)
      let p2bG : Fin nP → Fin nA →
          Stream ((Nat × Ballot nP) × Except (Option (Ballot nP)) Unit) :=
        fun i j => ((acceptor_p2_ticks.f (b2G j, g.2.2.2.2 j)).2.flatten).filterMap
          (fun dm => if dm.1 = i then
            some ((dm.2.slot, dm.2.ballot), dm.2.res) else none)
      -- ghost: emission keys are duplicate-free and owned (the B2
      -- discipline + paxos.rs:862, at the carrier)
      have hkeys : SPRequires g → ∀ i : Fin nP,
          (((emitG i).flatten).map (·.1)).Nodup ∧
          ∀ key ∈ ((emitG i).flatten).map (·.1),
            (key : Nat × Ballot nP).2.proposerId = i :=
        fun hw i => sp_keys_face f i (spTicks_chain nd hw i).toChain
      -- ghost: P2a ballot ownership and sender tags (paxos.rs:747)
      have hp2a_own : SPRequires g → ∀ i : Fin nP, ∀ m ∈ p2asG i,
          (m : P2a P nP).ballot.proposerId = i ∧ m.sender = i :=
        fun hw i m hm => by
          obtain ⟨sv, hsv, rfl⟩ := List.mem_map.mp hm
          exact ⟨(hkeys hw i).2 sv.1 (List.mem_map.mpr ⟨sv, hsv, rfl⟩), rfl⟩
      -- ghost: per-proposer P2a key uniqueness
      have hp2a_nodup : SPRequires g → ∀ i : Fin nP,
          ((p2asG i).map (fun m => ((m : P2a P nP).slot, m.ballot))).Nodup :=
        fun hw i => by
          have hfuse : (p2asG i).map (fun m => ((m : P2a P nP).slot, m.ballot))
              = ((emitG i).flatten).map (·.1) := by
            show List.map _ (List.map _ _) = _
            rw [List.map_map]
            rfl
          rw [hfuse]
          exact (hkeys hw i).1
      -- ghost: the phase-2 fan-in keys are duplicate-free (guarded key
      -- discipline + ownership disjointness — paxos.rs:862's `assume`,
      -- discharged)
      have hb2_nodup : SPRequires g → ∀ j : Fin nA,
          (((b2G j).flatten).map
            (fun m => ((m : P2a P nP).slot, m.ballot))).Nodup :=
        fun hw j => by
          refine nodup_of_count_le_one (fun κ => ?_)
          rw [List.countP_map]
          show ((b2G j).flatten).countP
            (fun m => decide ((m.slot, m.ballot) = κ)) ≤ 1
          have h2 : ((b2G j).flatten).countP
                (fun m => decide ((m.slot, m.ballot) = κ))
              ≤ (unionF p2asG).countP
                (fun m => decide ((m.slot, m.ballot) = κ)) :=
            countP_le_of_count_le (fun v => batchC_count_le _ _ v) _
          have h3 : (unionF p2asG).countP
              (fun m => decide ((m.slot, m.ballot) = κ)) ≤ 1 := by
            unfold unionF
            rw [countP_flatMap_eq_sum]
            refine sum_map_le_single _ κ.2.proposerId 1 (fun i hi => ?_) ?_
            · rw [List.countP_eq_zero]
              intro m hm hkm
              have hkm' : (m.slot, m.ballot) = κ := of_decide_eq_true hkm
              have hown := (hp2a_own hw i m hm).1
              apply hi
              rw [← hkm']
              exact hown.symm
            · refine countP_key_le_one (hp2a_nodup hw κ.2.proposerId) κ ?_
              intro m hm
              exact of_decide_eq_true hm
          exact Nat.le_trans h2 h3
      -- ghost: p2b vote cap — at most one `Ok` per (slot, ballot) per
      -- acceptor per proposer (`acceptor_p2`'s decode-cap contract at the
      -- duplicate-free fan-in)
      have hcap : SPRequires g → ∀ (i : Fin nP) (j : Fin nA) (slot : Nat)
          (b : Ballot nP),
          (p2bG i j).countP
            (fun e => decide (e.1 = (slot, b)) && e.2.isOk) ≤ 1 :=
        fun hw i j slot b =>
          (acceptor_p2_ticks.ensures (b2G j, g.2.2.2.2 j)).decode_cap
            (hb2_nodup hw j) i slot b
      -- ghost: p_to_replicas elimination — every emitted `(slot, value)`
      -- quotes a send-side emission (the join_responses metadata) and a
      -- chosen quorum key (`join_responses` never fabricates;
      -- variant-generic)
      have helim : ∀ (i : Fin nP) {slot : Nat} {v : Option P},
          (slot, v) ∈ (p_to_replicas i).f g →
          ∃ sv ∈ ((payloads_to_send i).f g).flatten,
            sv.1.1 = slot ∧ sv.2 = v ∧ sv.1 ∈ ((quorum i).f g).1 :=
        fun i {slot} {v} hc => by
          have hc' : (slot, v) ∈ List.map
              (fun kmv : (Nat × Ballot nP) × Option P × Unit =>
                (kmv.1.1, kmv.2.1))
              ((scan (joinResponsesTick (Nat × Ballot nP) (Option P) Unit).step
                [] ((TSing.zip ((payloads_to_send i).f g)
                  (TStream.map (batchC ((quorum i).f g).1 []
                    (nd.joinBatch i)) (fun key => (key, ())))).map
                  (fun mr => JoinTickIn.mk mr.1 mr.2))).flatten) := hc
          obtain ⟨kmv, hkmv, hpair⟩ := List.mem_map.mp hc'
          obtain ⟨key, mv, u⟩ := kmv
          have hkey1 : key.1 = slot := congrArg Prod.fst hpair
          have hmv : mv = v := congrArg Prod.snd hpair
          obtain ⟨hmeta, hresp⟩ := joinResponses_zip_elim hkmv
          have hkeyq : key ∈ ((quorum i).f g).1 := by
            obtain ⟨l', hl', hkl⟩ := List.mem_flatten.mp hresp
            obtain ⟨l₀, hl₀, rfl⟩ := List.mem_map.mp hl'
            obtain ⟨x, hx, hfx⟩ := List.mem_map.mp hkl
            have hxk : x = key := congrArg Prod.fst hfx
            subst hxk
            exact batchC_mem (List.mem_flatten.mpr ⟨l₀, hl₀, hx⟩)
          exact ⟨(key, mv), hmeta, hkey1, hmv, hkeyq⟩
      { commit_spec := fun hw hv {i} {slot} {v} hc => by
          subst hv
          obtain ⟨sv, hsv, hs, hv', hq⟩ := helim i hc
          have hsvG : sv ∈ (emitG i).flatten := hsv
          have hkeyown : sv.1.2.proposerId = i :=
            (hkeys hw i).2 sv.1 (List.mem_map.mpr ⟨sv, hsvG, rfl⟩)
          have hkeyeq : sv.1 = (slot, sv.1.2) := by
            rw [← hs]
          refine ⟨sv.1.2, hkeyown, ?_, ?_⟩
          · show ((slot, sv.1.2), v) ∈ (emitG i).flatten
            rw [← hs, ← hv']
            exact hsvG
          · -- the chosen witness, on the published `a_log` output
            have hq' : (slot, sv.1.2)
                ∈ (collect_quorum (fun j => p2bG i j)
                  (f + 1) (2 * f + 1) (nd.quorumBatch i)).1 := by
              rw [← hkeyeq]
              exact hq
            obtain ⟨C, hndC, hlen, hprov⟩ := collect_quorum_distinct_members
              hq' (fun j => hcap hw i j slot sv.1.2)
            refine ⟨C, hndC, hlen, fun j hj => ?_⟩
            obtain ⟨x, hx, hok⟩ := hprov j hj
            obtain ⟨dm, hdm, hsome⟩ := List.mem_filterMap.mp hx
            unfold okKey at hok
            rw [Bool.and_eq_true, decide_eq_true_iff] at hok
            have hdmi : dm.1 = i := by
              by_cases hi : dm.1 = i
              · exact hi
              · rw [if_neg hi] at hsome
                cases hsome
            rw [if_pos hdmi] at hsome
            cases hsome
            have hkey := hok.1
            have hslot : dm.2.slot = slot := congrArg Prod.fst hkey
            have hballot : dm.2.ballot = sv.1.2 := congrArg Prod.snd hkey
            have hres : dm.2.res = .ok () := by
              cases hres : dm.2.res with
              | ok u => rfl
              | error e =>
                rw [show ((dm.2.slot, dm.2.ballot), dm.2.res).2 = dm.2.res
                  from rfl, hres] at hok
                cases hok.2
            -- `acceptor_p2`'s write-before-ack contract at the fan-in
            obtain ⟨t', hta, htl, ham, hcov⟩ :=
              (acceptor_p2_ticks.ensures (b2G j, g.2.2.2.2 j)).ok_spec
                hdm hres
            refine ⟨t', hta, htl, ?_, ?_⟩
            · rw [ham, hballot]
            · have hcov' : LogCovers
                  (((acceptor_p2_ticks.f (b2G j,
                    g.2.2.2.2 j)).1.vals[t']'htl).2) dm.2.slot dm.2.ballot :=
                hcov
              rw [hslot, hballot] at hcov'
              exact hcov'
        emission_spec := fun hw hv {i} {slot} {b} {v} h => by
          subst hv
          have he : ((slot, b), v) ∈ (emitG i).flatten := h
          obtain ⟨t, ht, hl, hb⟩ := sp_emit_elim he
          obtain ⟨hpr, hpb, hpl, hv', hbal, hfl⟩ := spTicks_getElem nd g i ht
          have hwire : (g.2.2.2.1 i)[t]'hpr = spPin nd (g := g) i b.num := by
            rw [← hv', spTicks_pinned nd hw i ht hl]
            have hbn : (b : Ballot nP).num
                = ((spTicks nd g i)[t]'ht).2.1.num :=
              congrArg Ballot.num hb
            rw [hbn]
          refine ⟨t, hpl, hpb, hpr, ?_, ?_, ?_⟩
          · rw [← hfl]; exact hl
          · rw [← hbal]; exact hb.symm
          · intro e₀ hE
            have hE' : (slot, e₀)
                ∈ ((spPin nd (g := g) i b.num).map Prod.snd).flatten := by
              rw [← hwire]; exact hE
            obtain ⟨best, hval, hdom, hbest⟩ := sp_covered_value f i
              (spTicks_chain nd hw i) he hE'
            refine ⟨best, hval, hdom, ?_⟩
            rw [hwire]
            exact hbest
        log_entry_spec := fun hw hv {j} {t} htl {slot} {e} hE => by
          subst hv
          have htl' : t < ((acceptor_p2_ticks.f
              (b2G j, g.2.2.2.2 j)).1).vals.length := htl
          have hE' : (slot, e) ∈ ((((acceptor_p2_ticks.f
              (b2G j, g.2.2.2.2 j)).1).vals[t]'htl').2 : LogMap P nP) := hE
          obtain ⟨m, hm2, hms, hentry⟩ :=
            (acceptor_p2_ticks.ensures (b2G j, g.2.2.2.2 j)).log_entry
              htl' hE'
          have hmb : m.ballot = e.ballot :=
            (congrArg LogValue.ballot hentry).symm
          have hmv : m.value = e.value :=
            (congrArg LogValue.value hentry).symm
          -- provenance to some proposer's broadcast
          obtain ⟨i₃, hm₃⟩ := unionF_mem (batchC_mem hm2)
          have hown := (hp2a_own hw i₃ m hm₃).1
          obtain ⟨sv, hsv, hmk⟩ := List.mem_map.mp hm₃
          have hsvs : sv.1.1 = m.slot := by rw [← hmk]
          have hsvb : sv.1.2 = m.ballot := by rw [← hmk]
          have hsvv : sv.2 = m.value := by rw [← hmk]
          have hi₃ : i₃ = e.ballot.proposerId := by
            rw [← hmb]
            exact hown.symm
          subst hi₃
          have hsv' : ((sv.1.1, sv.1.2), sv.2)
              ∈ (emitG e.ballot.proposerId).flatten := hsv
          rw [hsvs, ← hms, hsvb, hmb, hsvv, hmv] at hsv'
          exact hsv'
        emission_functional := fun hw hv {g₀} hgg' {i} {slot} {b}
            {v} {v'} h h' => by
          subst hv
          have hmem : ((slot, b), v) ∈ (emitG i).flatten :=
            SPEmission.mono PaxosVariant.guarded f nd hgg' h
          have h'' : ((slot, b), v') ∈ (emitG i).flatten := h'
          have heq := eq_of_nodup_keys (hkeys hw i).1 hmem h'' (by rfl)
          exact congrArg Prod.snd heq })

end Verified

end HydroLean.Programs.Paxos
