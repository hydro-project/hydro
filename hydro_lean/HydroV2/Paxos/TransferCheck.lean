import HydroV2.Paxos.PaxosCore
import HydroV2.TransferTheory

/-!
# `paxos_core` under the transfer (ruling-4 validation)

The whole verified Paxos core, instantiated at the coupling semantics
`CorrSem` — machine runs paired with covering sets of denotational
runs. The per-program cost is exactly what ruling 4 demands: **a
decision record and a projection** — no program-specific definitions,
no program-specific proofs. `replica_covered` reads the coupling off
the instantiation: at every step, the machine's replica-stream views
sit below some denotational run (whose safety is the separately-proven
`∀ d` headline, `PCEnsures`).

The `CorrSem` decision record is the *operational* vocabulary: delivery
cursors for the four transports (here: generous, `c t = t`), concrete
timing (here: quiet timers), emission linearizations (here: empty), and
`Unit` at every content-consumption site — the machine consumes its
buffers in arrival order.
-/

namespace HydroV2
namespace PaxosTransferCheck

abbrev oneMem : Bool → Nat := fun _ => 1

/-- The coupling semantics: one proposer (`true`), one acceptor
(`false`), every member ticking every step. -/
abbrev C : HydroSem Bool oneMem := CorrSem Bool oneMem (fun _ _ _ => true)

/-- Generous delivery: everything sent is delivered step-for-step. -/
def cursors {nX nY : Nat} : Fin nX → Fin nY → Nat → Nat :=
  fun _ _ t => t

/-- The full operational decision record for `paxos_core` at `C`. -/
def dec : PaxosCoreDec C 1 1 Nat where
  le :=
    { receivedMax := ()
      hb :=
        { sample := fun _ => []
          timeout := fun _ => []
          interval := fun _ => []
          ial := cursors }
      p1aCh := cursors
      p1aBatch := ()
      p1bCh := cursors
      p1b :=
        { cqwr := ()
          cqwrEmit := fun _ => []
          order := ()
          snap := () }
      fuelFail := ()
      fuelIAL := ()
      fuelLead := () }
  sp :=
    { payloadBatch := ()
      rcEmit := fun _ => []
      p2aCh := cursors
      p2aBatch := ()
      p2bCh := cursors
      cqBatch := ()
      cqEmit := fun _ => []
      jrBatch := ()
      jrEmit := fun _ => [] }
  fuelSeqMax := ()
  fuelALog := ()

/-- A coupled client stream: one committed value, fully present. -/
def clientIn : C.Stream true Nat .totalOrder .exactlyOnce :=
  ⟨(fun _i => StepHist.const [42], {v | v = fun _i => [42]}),
   fun _t => ⟨fun _i => [42], rfl, fun _i => List.prefix_refl _⟩⟩

/-- A coupled (empty) checkpoint singleton. -/
def ckIn : C.TickSingleton false (Option Nat) .unbounded :=
  ⟨(fun _i _t => [], {v | v = fun _i => []}),
   fun _T => ⟨fun _i => [], rfl, fun _i => List.nil_prefix⟩⟩

set_option maxHeartbeats 3200000 in
set_option maxRecDepth 65536 in
/-- The instantiation (line one of the two-liner). -/
noncomputable def run :=
  paxos_core C .guarded true false 0 clientIn ckIn dec

/-- **The projection (line two)**: at every step, the machine's
replica-stream views are covered by some denotational run of the
relational carrier — the ∃-form transfer for the whole Paxos core,
with zero program-specific proof content. -/
theorem replica_covered (t : Nat) :
    ∃ v ∈ run.val.2.val.2, ∀ i,
      ListLe .noOrder .exactlyOnce ((run.val.2.val.1 i).view t) (v i) :=
  run.val.2.property t

/-- Same reading for the new-leader ballot stream (ordered grade). -/
theorem ballots_covered (t : Nat) :
    ∃ v ∈ run.val.1.val.2, ∀ i,
      ListLe .totalOrder .exactlyOnce
        ((run.val.1.val.1 i).view t) (v i) :=
  run.val.1.property t

end PaxosTransferCheck
end HydroV2
