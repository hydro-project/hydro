import HydroV2.MonoRel
import HydroV2.Paxos.Types
import Mathlib.Data.Multiset.Bind

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

/-- **paxos.rs:595–672 `recommit_after_leader_election`** over the
proposer cluster. Returns (`p_log_to_try_commit.chain(p_log_holes)`,
`p_max_slot`) per tick. -/
def recommit_after_leader_election (H : HydroSem L mem) (prop : L)
    (accepted_logs :
      H.TickStream prop (ALog P (mem prop)) .noOrder .exactlyOnce)
    (p_ballot : H.TickSingleton prop (Ballot (mem prop)) .unbounded)
    (f : Nat) :
    {out : H.TickSingleton prop
        (List ((Nat × Ballot (mem prop)) × Option P)) .unbounded
      × H.TickSingleton prop (Option Nat) .unbounded //
      ∀ hv : H = Values L mem,
        match H, hv, accepted_logs, p_ballot, out with
        | _, rfl, bs, pb, o => RCEnsures prop f bs pb o} :=
  ⟨(H.mapBatchesUnordered accepted_logs p_ballot
      (fun _me logs b => recommitList f b logs),
    H.mapBatchesUnordered accepted_logs p_ballot
      (fun _me logs _b => rcMaxSlot logs)), by
  intro hv; subst hv
  refine ⟨fun i => rfl, fun i => rfl, ?_⟩
  intro i t ht e he
  have ht' : t < ((Trace.zip (accepted_logs i) (p_ballot i)).map
      (fun bx => recommitList f bx.2 bx.1)).length := ht
  have hlen : t < (accepted_logs i).length ∧ t < (p_ballot i).length := by
    simpa [Trace.zip] using ht'
  refine ⟨hlen.2, ?_⟩
  have he' : e ∈ recommitList f ((p_ballot i)[t]'hlen.2)
      ((accepted_logs i)[t]'hlen.1) := by
    have h0 : e ∈ ((Trace.zip (accepted_logs i) (p_ballot i)).map
        (fun bx => recommitList f bx.2 bx.1))[t]'ht' := he
    simp only [Trace.zip, List.getElem_map, List.getElem_zip] at h0
    exact h0
  exact recommitList_ballot f _ _ e he'⟩

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
