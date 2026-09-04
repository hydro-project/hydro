import HydroV2.MonoRel
import HydroV2.Paxos.Types

/-!
# `index_payloads` (paxos.rs:776–806)

Assign consecutive slots to the leader's payload batch: the base slot is
`p_max_slot + 1` when phase-1 reconciliation produced one (a fresh
leader continues after the recovered log), else the cross-tick
`next_slot` state; the batch is enumerated from the base and the state
advances by the batch size. A `use::state` tick loop over the **ordered**
payload batches (`scan_batches_across_ticks`).
-/

namespace HydroV2

variable {L : Type} {mem : L → Nat} {P : Type} [DecidableEq P]

/-- One `index_payloads` tick (the `sliced!` body, paxos.rs:781–804). -/
def ipStep (next_slot : Nat) (batch : List P) (maxSlot : Option Nat) :
    Nat × List (Nat × P) :=
  let base := match maxSlot with | some s => s + 1 | none => next_slot
  (base + batch.length,
   batch.zipIdx.map (fun pi => (base + pi.2, pi.1)))

/-- What `index_payloads` **ensures**, over the `Values` denotation. -/
structure IPEnsures (prop : L)
    (ms : TickV (mem prop) (Option Nat) .unbounded)
    (bs : Fin (mem prop) → Trace (List P))
    (out : TickV (mem prop) (List (Nat × P)) .unbounded) : Prop where
  /-- **The run face**: the indexed batches ARE the `ipStep` state-loop run
  over the zipped (payload batch, max slot) trace — slot arithmetic
  (`ipStep_slots` below) applies tick by tick. -/
  run_eq : ∀ i, out i
    = scanAcrossTicksTrace (fun s bt => ipStep s bt.1 bt.2) 0 (Trace.zip (bs i) (ms i))

/-- **paxos.rs:776–806 `index_payloads`**. -/
def index_payloads (H : HydroSem L mem) (prop : L)
    (p_max_slot : H.TickSingleton prop (Option Nat) .unbounded)
    (c_to_proposers : H.TickStream prop P .totalOrder .exactlyOnce) :
    {out : H.TickSingleton prop (List (Nat × P)) .unbounded //
      ∀ hv : H = Values L mem,
        match H, hv, p_max_slot, c_to_proposers, out with
        | _, rfl, ms, bs, o => IPEnsures prop ms bs o} :=
  ⟨H.scan_batches_across_ticks c_to_proposers p_max_slot
    (fun _me next_slot batch maxSlot => ipStep next_slot batch maxSlot) 0,
   by intro hv; subst hv; exact ⟨fun i => rfl⟩⟩

theorem zipIdx_slots (base : Nat) :
    ∀ (l : List P) (k : Nat),
      (l.zipIdx k).map (fun pi => base + pi.2)
        = List.range' (base + k) l.length
  | [], _ => rfl
  | p :: rest, k => by
    rw [List.zipIdx_cons, List.map_cons, zipIdx_slots base rest (k + 1),
      List.length_cons, List.range'_succ, ← Nat.add_assoc]

/-- The emitted slots are exactly the next `batch.length` slots from the
base — consecutive, no gaps, no repeats (`List.range'`). -/
theorem ipStep_slots (next_slot : Nat) (batch : List P)
    (maxSlot : Option Nat) :
    ((ipStep next_slot batch maxSlot).2.map Prod.fst)
      = List.range'
          (match maxSlot with | some s => s + 1 | none => next_slot)
          batch.length := by
  show ((batch.zipIdx.map _).map Prod.fst) = _
  rw [List.map_map]
  have h := zipIdx_slots
    (base := match maxSlot with | some s => s + 1 | none => next_slot)
    (P := P) batch 0
  rw [Nat.add_zero] at h
  exact h

/-! ## Executable smoke tests -/

-- A stable leader: consecutive slots across ticks from the state.
#guard (index_payloads (Values PaxLoc (paxMem 1 1)) .prop
    (fun _ => [none, none])
    (fun _ => [[10, 11], [12]] : Fin 1 → Trace (List Nat))).val 0
  = [[(0, 10), (1, 11)], [(2, 12)]]

-- A fresh leader reconciles: the recovered max slot rebases indexing.
#guard (index_payloads (Values PaxLoc (paxMem 1 1)) .prop
    (fun _ => [some 5, none])
    (fun _ => [[10, 11], [12]] : Fin 1 → Trace (List Nat))).val 0
  = [[(6, 10), (7, 11)], [(8, 12)]]

end HydroV2
