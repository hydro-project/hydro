import HydroV2.MonoRel
import HydroV2.Paxos.Types
import Mathlib.Data.Multiset.Bind
import HydroV2.HydroDef

/-!
# `recommit_after_leader_election` (paxos.rs:595–672)

A fresh leader reconciles the quorum's accepted logs: per slot, keep the
max-ballot entry (recommitting it under the new ballot unless already
committed on more than `f` acceptors or checkpointed away), and fill the
log holes below the max slot with no-ops.

The per-slot fold carries the same `commutative = manual_proof!(TODO)`
hole as `acceptor_p2`'s log merge; the V2 model computes the whole tick
value as **canonical functions of the batch multiset** (`logView` for
the champions, cardinality for the counts) — functions out of the
quotient are order-safe by construction, so batch-arrival order cannot
leak.
-/

namespace HydroV2

variable {P : Type} [DecidableEq P]

/-- `accepted_logs.filter_map(checkpoint).max()` (paxos.rs:606–610). -/
def rcMaxCheckpoint {nP : Nat} (logs : Multiset (ALog P nP)) :
    Option Nat :=
  @Multiset.foldl _ _
    (fun acc c => match acc, c with
      | none, c => some c
      | some a, c => some (Nat.max a c))
    ⟨fun s x y => by
      cases s with
      | none => exact congrArg some (Nat.max_comm x y)
      | some a => exact congrArg some (max_right_comm a x y)⟩
    none (logs.filterMap (fun lg => lg.1))

/-- All accepted log entries, pooled (Rust's `flatten_unordered`). -/
def rcEntries {nP : Nat} (logs : Multiset (ALog P nP)) :
    Multiset (Nat × LogValue P nP) :=
  logs.bind (fun lg => Multiset.ofList lg.2)

/-- The number of accepted entries agreeing with the champion value at
one slot (Rust's incremental `count`, made canonical). -/
def rcCount {nP : Nat} (ents : Multiset (Nat × LogValue P nP))
    (slot : Nat) (v : Option P) : Nat :=
  (ents.filter (fun e => e.1 = slot ∧ e.2.value = v)).card

/-- `p_log_to_try_commit ++ p_log_holes` (paxos.rs:611–670), as one
canonical function of the accepted-log batch. -/
def recommitList {nP : Nat} (f : Nat) (myBallot : Ballot nP)
    (logs : Multiset (ALog P nP)) :
    List ((Nat × Ballot nP) × Option P) :=
  let ents := rcEntries logs
  let ckpt := rcMaxCheckpoint logs
  let champs : LogMap P nP := logView ents
  let tryCommit := champs.filterMap (fun (sl : Nat × LogValue P nP) =>
    if f < rcCount ents sl.1 sl.2.value then none
    else if (match ckpt with
      | some c => decide (sl.1 ≤ c)
      | none => false) then none
    else some ((sl.1, myBallot), sl.2.value))
  let proposed : List Nat := champs.map (·.1)
  let lo := match ckpt with | some c => c + 1 | none => 0
  let holes := (match (champs.map
      (fun (sl : Nat × LogValue P nP) => sl.1)).max? with
    | some maxSlot => (List.range' lo (maxSlot - lo)).filter
        (fun slot => !proposed.contains slot)
    | none => []).map (fun slot => ((slot, myBallot), (none : Option P)))
  tryCommit ++ holes

/-- `p_max_slot` (paxos.rs:655). -/
def rcMaxSlot {nP : Nat} (logs : Multiset (ALog P nP)) : Option Nat :=
  ((logView (rcEntries logs)).map
    (fun (sl : Nat × LogValue P nP) => sl.1)).max?

/-- The counted champions (Rust's keyed fold value, `(count, entry)`
per slot). -/
def rcChampCounts {nP : Nat} (logs : Multiset (ALog P nP)) :
    List (Nat × (Nat × LogValue P nP)) :=
  (logView (rcEntries logs)).map
    (fun sl => (rcCount (rcEntries logs) sl.1 sl.2.value, sl))

/-- The wire-chain tick — counted champions crossed with the ballot
and the checkpoint, chained with the holes — IS `recommitList`: the
`filterMap`/`map` fusions of the dataflow ops. -/
theorem recommit_chain_eq {nP : Nat} (f : Nat) (b : Ballot nP)
    (logs : Multiset (ALog P nP)) :
    ((rcChampCounts logs).filterMap
      (fun csl =>
        if f < csl.1 then none
        else if (match rcMaxCheckpoint logs with
          | some c => decide (csl.2.1 ≤ c)
          | none => false) then none
        else some ((csl.2.1, b), csl.2.2.value)))
    ++ ((match ((rcChampCounts logs).map
          (fun (csl : Nat × Nat × LogValue P nP) => csl.2.1)).max? with
        | some maxSlot =>
          (List.range' (match rcMaxCheckpoint logs with
              | some c => c + 1 | none => 0)
            (maxSlot - (match rcMaxCheckpoint logs with
              | some c => c + 1 | none => 0))).filter
            (fun slot => !((rcChampCounts logs).map
              (fun (csl : Nat × Nat × LogValue P nP) =>
                csl.2.1)).contains slot)
        | none => []).map
          (fun slot => ((slot, b), (none : Option P))))
    = recommitList f b logs := by
  unfold recommitList rcChampCounts
  rw [List.filterMap_map, List.map_map]
  rfl

/-- The max-slot leg of the chain, fused. -/
theorem rcMaxSlot_chain_eq {nP : Nat} (logs : Multiset (ALog P nP)) :
    ((rcChampCounts logs).map
        (fun (csl : Nat × Nat × LogValue P nP) => csl.2.1)).max?
      = rcMaxSlot logs := by
  unfold rcChampCounts rcMaxSlot
  rw [List.map_map]
  rfl

/-- The whole recommit wire chain, over the zipped (batch, ballot)
trace: definitionally the module's `p_log_to_try_commit.chain(holes)`
wire at `Values`; propositionally the canonical `recommitList` run. -/
theorem recommit_wires_eq {nP : Nat} (f : Nat)
    (bs : Trace (Multiset (ALog P nP))) (pb : Trace (Ballot nP)) :
    (Trace.zip
      ((Trace.zip
        (Trace.zip ((Trace.zip bs pb).map (fun bx => rcChampCounts bx.1))
          pb)
        ((Trace.zip bs pb).map (fun bx => rcMaxCheckpoint bx.1))).map
        (fun x => x.1.1.filterMap (fun csl =>
          if f < csl.1 then none
          else if (match x.2 with
            | some c => decide (csl.2.1 ≤ c)
            | none => false) then none
          else some ((csl.2.1, x.1.2), csl.2.2.value))))
      ((Trace.zip
        (Trace.zip
          (((Trace.zip bs pb).map (fun bx => rcChampCounts bx.1)).map
            (fun ch => (ch.map
              (fun (csl : Nat × Nat × LogValue P nP) =>
                csl.2.1)).max?))
          ((Trace.zip bs pb).map (fun bx => rcMaxCheckpoint bx.1)))
        (Trace.zip ((Trace.zip bs pb).map (fun bx => rcChampCounts bx.1))
          pb)).map
        (fun x =>
          (match x.1.1 with
            | some maxSlot =>
              (List.range'
                (match x.1.2 with | some c => c + 1 | none => 0)
                (maxSlot
                  - (match x.1.2 with
                    | some c => c + 1 | none => 0))).filter
                (fun slot => !(x.2.1.map
                  (fun (csl : Nat × Nat × LogValue P nP) =>
                    csl.2.1)).contains slot)
            | none => []).map
              (fun slot => ((slot, x.2.2), (none : Option P)))))).map
      (fun x => x.1 ++ x.2)
    = (Trace.zip bs pb).map (fun bx => recommitList f bx.2 bx.1) := by
  apply List.ext_getElem
  · simp only [List.length_map, Trace.zip, List.length_zip]
    omega
  · intro t h1 h2
    simp only [List.getElem_map, Trace.zip, List.getElem_zip]
    exact recommit_chain_eq f _ _

/-- The max-slot wire, over the zipped trace. -/
theorem rcMaxSlot_wires_eq {nP : Nat}
    (bs : Trace (Multiset (ALog P nP))) (pb : Trace (Ballot nP)) :
    ((Trace.zip bs pb).map (fun bx => rcChampCounts bx.1)).map
      (fun ch => (ch.map
        (fun (csl : Nat × Nat × LogValue P nP) => csl.2.1)).max?)
    = (Trace.zip bs pb).map (fun bx => rcMaxSlot bx.1) := by
  rw [List.map_map]
  exact List.map_congr_left (fun z _ => rcMaxSlot_chain_eq _)

variable {L : Type} {mem : L → Nat}

/-- **Recommits are owned**: every emitted `(slot, ballot) ↦ value`
quotes the leader's own ballot — recommit traffic stays inside the
per-ballot vote-counting argument. -/
theorem recommitList_ballot {nP : Nat} (f : Nat) (myBallot : Ballot nP)
    (logs : Multiset (ALog P nP)) :
    ∀ e ∈ recommitList f myBallot logs, (e : (Nat × Ballot nP)
      × Option P).1.2 = myBallot := by
  intro e he
  unfold recommitList at he
  rcases List.mem_append.mp he with h | h
  · obtain ⟨sl, -, hsl⟩ := List.mem_filterMap.mp h
    by_cases h1 : f < rcCount (rcEntries logs) sl.1 sl.2.value
    · rw [if_pos h1] at hsl
      cases hsl
    · rw [if_neg h1] at hsl
      by_cases h2 : (match rcMaxCheckpoint logs with
        | some c => decide (sl.1 ≤ c)
        | none => false) = true
      · rw [if_pos h2] at hsl
        cases hsl
      · rw [if_neg h2] at hsl
        injection hsl with hsl'
        rw [← hsl']
  · obtain ⟨slot, -, hslot⟩ := List.mem_map.mp h
    rw [← hslot]

/-- What `recommit_after_leader_election` **ensures**, over the
`Values` denotation. -/
structure RCEnsures (prop : L) (f : Nat)
    (bs : Fin (mem prop) → Trace (Multiset (ALog P (mem prop))))
    (pb : TickV (mem prop) (Ballot (mem prop)) .unbounded)
    (out : TickV (mem prop)
        (List ((Nat × Ballot (mem prop)) × Option P)) .unbounded
      × TickV (mem prop) (Option Nat) .unbounded) : Prop where
  /-- **The commit face**: each tick's recommit list IS the canonical
  `recommitList` of the tick's batch at the tick's ballot. -/
  commits_eq : ∀ i, out.1 i
    = (Trace.zip (bs i) (pb i)).map (fun bx => recommitList f bx.2 bx.1)
  /-- **The max-slot face**. -/
  maxslot_eq : ∀ i, out.2 i
    = (Trace.zip (bs i) (pb i)).map (fun bx => rcMaxSlot bx.1)
  /-- **Recommits are owned**: every emitted entry quotes the tick's
  own ballot — recommit traffic stays inside the per-ballot
  vote-counting argument. -/
  owned : ∀ (i : Fin (mem prop)) {t : Nat} (ht : t < (out.1 i).length),
    ∀ e ∈ (out.1 i)[t]'ht, ∃ hb : t < (pb i).length,
      (e : (Nat × Ballot (mem prop)) × Option P).1.2 = (pb i)[t]'hb

set_option maxHeartbeats 1000000 in
/-- **paxos.rs:595–672 `recommit_after_leader_election`** over the
proposer cluster. Returns (`p_log_to_try_commit.chain(p_log_holes)`,
`p_max_slot`) per tick. -/
hydro def recommit_after_leader_election (H : HydroSem L mem) (prop : L)
    (accepted_logs :
      H.TickStream prop (ALog P (mem prop)) .noOrder .exactlyOnce)
    (p_ballot : H.TickSingleton prop (Ballot (mem prop)) .unbounded)
    (f : Nat) :
    (H.TickSingleton prop
        (List ((Nat × Ballot (mem prop)) × Option P)) .unbounded
      × H.TickSingleton prop (Option Nat) .unbounded)
  ensures out => RCEnsures prop f accepted_logs p_ballot out :=
  -- p_p1b_max_checkpoint = accepted_logs
  --   .filter_map(|(checkpoint, _log)| checkpoint).max() (paxos.rs:606–610)
  let p_p1b_max_checkpoint := H.mapBatchesUnordered accepted_logs
    p_ballot (fun _me logs _b => rcMaxCheckpoint logs)
  -- p_p1b_highest_entries_and_count = accepted_logs.map(log)
  --   .flatten_unordered().into_keyed().fold((count, entry), …)
  --   (paxos.rs:611–637)
  let p_p1b_highest_entries_and_count := H.mapBatchesUnordered
    accepted_logs p_ballot (fun _me logs _b => rcChampCounts logs)
  -- p_log_to_try_commit = ….entries().cross_singleton(p_ballot)
  --   .cross_singleton(p_p1b_max_checkpoint).filter_map(…)
  --   (paxos.rs:638–652)
  let p_log_to_try_commit := H.mapTick
    (H.zipTick (H.zipTick p_p1b_highest_entries_and_count p_ballot)
      p_p1b_max_checkpoint)
    (fun _me x => x.1.1.filterMap (fun csl =>
      if f < csl.1 then none
      else if (match x.2 with
        | some c => decide (csl.2.1 ≤ c)
        | none => false) then none
      else some ((csl.2.1, x.1.2), csl.2.2.value)))
  -- p_max_slot = ….keys().max() (paxos.rs:655)
  let p_max_slot := H.mapTick p_p1b_highest_entries_and_count
    (fun _me ch => (ch.map
      (fun (csl : Nat × Nat × LogValue P (mem prop)) =>
        csl.2.1)).max?)
  -- p_log_holes = p_max_slot.zip(p_p1b_max_checkpoint)
  --   .flat_map_ordered(range).filter_not_in(p_proposed_slots)
  --   .cross_singleton(p_ballot).map((slot, ballot), None)
  --   (paxos.rs:656–668)
  let p_log_holes := H.mapTick
    (H.zipTick (H.zipTick p_max_slot p_p1b_max_checkpoint)
      (H.zipTick p_p1b_highest_entries_and_count p_ballot))
    (fun _me x =>
      (match x.1.1 with
        | some maxSlot =>
          (List.range' (match x.1.2 with | some c => c + 1 | none => 0)
            (maxSlot
              - (match x.1.2 with | some c => c + 1 | none => 0))).filter
            (fun slot => !(x.2.1.map
              (fun (csl : Nat × Nat × LogValue P (mem prop)) =>
                csl.2.1)).contains slot)
        | none => []).map
          (fun slot => ((slot, x.2.2), (none : Option P))))
  -- (p_log_to_try_commit.chain(p_log_holes), p_max_slot) (paxos.rs:670)
  let p_log_chained := H.mapTick
    (H.zipTick p_log_to_try_commit p_log_holes)
    (fun _me x => x.1 ++ x.2)
  -- the chain ticks ARE the canonical `recommitList`/`rcMaxSlot` of
  -- the tick's batch at the tick's ballot (the fused faces)
  ghost have hceq : ∀ (i : Fin (mem prop)), p_log_chained i
      = (Trace.zip (accepted_logs i) (p_ballot i)).map
          (fun bx => recommitList f bx.2 bx.1) := fun i =>
    recommit_wires_eq f (accepted_logs i) (p_ballot i)
  ghost have hms : ∀ (i : Fin (mem prop)), p_max_slot i
      = (Trace.zip (accepted_logs i) (p_ballot i)).map
          (fun bx => rcMaxSlot bx.1) := fun i =>
    rcMaxSlot_wires_eq (accepted_logs i) (p_ballot i)
  (p_log_chained, p_max_slot)
  prove
    commits_eq := fun i => hceq i,
    maxslot_eq := fun i => hms i,
    owned := fun i {t} ht e he => by
      -- the tick's list is the canonical recommit of the tick's reads …
      have ht' : t < ((Trace.zip (accepted_logs i) (p_ballot i)).map
          (fun bx => recommitList f bx.2 bx.1)).length := by
        rw [← hceq i]
        exact ht
      obtain ⟨hb, hpb, hread⟩ := Trace.zip_map_getElem ht'
      have he' : e ∈ recommitList f ((p_ballot i)[t]'hpb)
          ((accepted_logs i)[t]'hb) := by
        have h0 : e ∈ ((Trace.zip (accepted_logs i) (p_ballot i)).map
            (fun bx => recommitList f bx.2 bx.1))[t]'ht' := by
          rw [← List.getElem_of_eq (hceq i) ht]
          exact he
        rwa [hread] at h0
      -- … and recommits quote the tick's own ballot
      exact ⟨hpb, recommitList_ballot f _ _ e he'⟩

/-! ## Executable smoke tests -/

-- A quorum of two logs: slot 0's champion is the ballot-1 entry (value
-- 9); slot 2 forces a hole at slot 1; everything recommits at the new
-- leader's ballot 3.
#guard (recommitList (P := Nat) (nP := 1) 1 (Ballot.mk 3 0)
    {((none : Option Nat), [(0, ⟨Ballot.mk 1 0, some 9⟩)]),
     ((none : Option Nat), [(0, ⟨Ballot.mk 0 0, some 4⟩),
       (2, ⟨Ballot.mk 0 0, some 7⟩)])})
  = [((0, Ballot.mk 3 0), some 9), ((2, Ballot.mk 3 0), some 7),
     ((1, Ballot.mk 3 0), none)]

-- A value already on more than `f` acceptors is not recommitted.
#guard (recommitList (P := Nat) (nP := 1) 1 (Ballot.mk 3 0)
    {((none : Option Nat), [(0, ⟨Ballot.mk 1 0, some 9⟩)]),
     ((none : Option Nat), [(0, ⟨Ballot.mk 1 0, some 9⟩)])})
  = []

-- Checkpointed slots are skipped (the checkpoint travels in the logs).
#guard (recommitList (P := Nat) (nP := 1) 1 (Ballot.mk 3 0)
    {((some 0 : Option Nat), [(0, ⟨Ballot.mk 1 0, some 9⟩)])})
  = []

#guard (rcMaxSlot (P := Nat) (nP := 1)
    {((none : Option Nat), [(0, ⟨Ballot.mk 1 0, some 9⟩),
      (2, ⟨Ballot.mk 0 0, some 7⟩)])}) = some 2

end HydroV2
