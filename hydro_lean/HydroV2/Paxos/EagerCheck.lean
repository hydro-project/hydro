import HydroV2.Paxos.PaxosCore
import HydroV2.EagerRel

/-!
# `paxos_core` under the eager projection

The whole verified Paxos core, instantiated at the materialized
interpretation `Eager` — and **pinned to the `Values` denotation by
theorem**: `paxos_eager_den`/`paxos_eager_den_ballots` say the
denotation leg of the eager run IS the `Values` run at the same
decisions. Both are two-line corollaries of the program's free
theorem (`paxos_core_param`, generated at the `PaxosCore.lean` tail)
instantiated at the eager-agreement relation (`eagC`/`eagLaws`) — no
per-knot naming machinery, no program-specific proofs. With the
carrier-level `agree` field this pins the *executed data* to the
denotation (`paxos_eager_commits`), so `lake exe` artifacts
(`falsify`, `v2paxos`) run the theorem-covered semantics — the two
interpretations cannot silently diverge.
-/

namespace HydroV2

variable {L : Type} {mem : L → Nat} {P : Type} [DecidableEq P]

/-! ## Decision-record repacks (the vocabularies coincide:
`Eager.*Dec := (Values …).*Dec`) -/

/-- Repack `Values`-typed election decisions for `Eager` (the decision
vocabularies coincide; `ledecMR` precedent). -/
def ledecE {nP nA : Nat} (d : LEDec (Values L mem) nP nA P) :
    LEDec (Eager L mem) nP nA P :=
  ⟨d.receivedMax, ⟨d.hb.sample, d.hb.timeout, d.hb.interval, d.hb.ial⟩,
   d.p1aCh, d.p1aBatch, d.p1bCh,
   ⟨d.p1b.cqwr, d.p1b.cqwrEmit, d.p1b.order, d.p1b.snap⟩,
   d.fuelFail, d.fuelIAL, d.fuelLead⟩

/-- Repack `Values`-typed sequencing decisions for `Eager`. -/
def spdecE {nP nA : Nat} (d : SPDec (Values L mem) nP nA P) :
    SPDec (Eager L mem) nP nA P :=
  ⟨d.payloadBatch, d.rcEmit, d.p2aCh, d.p2aBatch, d.p2bCh,
   d.cqBatch, d.cqEmit, d.jrBatch, d.jrEmit⟩

/-- Repack `Values`-typed `paxos_core` decisions for `Eager`. -/
def pcdecE {nP nA : Nat} (d : PaxosCoreDec (Values L mem) nP nA P) :
    PaxosCoreDec (Eager L mem) nP nA P :=
  ⟨ledecE d.le, spdecE d.sp, d.fuelSeqMax, d.fuelALog⟩

/-- Repack `Eager`-typed election decisions as `Values` ones (inverse
of `ledecE` up to record eta). -/
@[reducible] def ledecVE {nP nA : Nat}
    (de : LEDec (Eager L mem) nP nA P) :
    LEDec (Values L mem) nP nA P :=
  ⟨de.receivedMax,
   ⟨de.hb.sample, de.hb.timeout, de.hb.interval, de.hb.ial⟩,
   de.p1aCh, de.p1aBatch, de.p1bCh,
   ⟨de.p1b.cqwr, de.p1b.cqwrEmit, de.p1b.order, de.p1b.snap⟩,
   de.fuelFail, de.fuelIAL, de.fuelLead⟩

/-- Repack `Eager`-typed sequencing decisions as `Values` ones. -/
@[reducible] def spdecVE {nP nA : Nat}
    (de : SPDec (Eager L mem) nP nA P) :
    SPDec (Values L mem) nP nA P :=
  ⟨de.payloadBatch, de.rcEmit, de.p2aCh, de.p2aBatch, de.p2bCh,
   de.cqBatch, de.cqEmit, de.jrBatch, de.jrEmit⟩

/-- Repack `Eager`-typed `paxos_core` decisions as `Values` ones. -/
@[reducible] def pcdecVE {nP nA : Nat}
    (de : PaxosCoreDec (Eager L mem) nP nA P) :
    PaxosCoreDec (Values L mem) nP nA P :=
  ⟨ledecVE de.le, spdecVE de.sp, de.fuelSeqMax, de.fuelALog⟩

variable (variant : PaxosVariant) (prop acc : L) (f : Nat)
  (cp : (Eager L mem).Stream prop P .totalOrder .exactlyOnce)
  (ck : (Eager L mem).TickSingleton acc (Option Nat) .unbounded)
  (d : PaxosCoreDec (Values L mem) (mem prop) (mem acc) P)

/-- **The eager-projection identity, commits leg**: the denotation of
the materialized run IS the `Values` run — through all knots, no
hypothesis. A corollary of the free theorem at the eager instance. -/
theorem paxos_eager_den :
    (paxos_core (Eager L mem) variant prop acc f cp ck
        (pcdecE d)).val.2.den
      = (paxos_core (Values L mem) variant prop acc f cp.den ck.den
          d).val.2 :=
  paxos_core_param₂ (eagC L mem) eagLaws () variant prop acc f
    cp cp.den (fun _ _ => rfl) ck ck.den (fun _ _ => rfl)
    (pcdecE d) d
    (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl)
    (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl)
    (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl)
    (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl)
    (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl)
    (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl)
    (fun _ _ => rfl) (fun _ _ => rfl)
    () le_rfl

/-- **The eager-projection identity, new-leader-announcements leg**. -/
theorem paxos_eager_den_ballots :
    (paxos_core (Eager L mem) variant prop acc f cp ck
        (pcdecE d)).val.1.den
      = (paxos_core (Values L mem) variant prop acc f cp.den ck.den
          d).val.1 :=
  paxos_core_param₁ (eagC L mem) eagLaws () variant prop acc f
    cp cp.den (fun _ _ => rfl) ck ck.den (fun _ _ => rfl)
    (pcdecE d) d
    (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl)
    (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl)
    (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl)
    (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl)
    (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl)
    (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl) (fun _ _ => rfl)
    (fun _ _ => rfl) (fun _ _ => rfl)
    () le_rfl

/-- **The executed data is the denotation**: what `lake exe falsify`
prints is, member by member, the `Values` run the headline theorems
quantify over. -/
theorem paxos_eager_commits (i : Fin (mem prop)) :
    (paxos_core (Eager L mem) variant prop acc f cp ck
        (pcdecE d)).val.2.data.get i
      = (paxos_core (Values L mem) variant prop acc f cp.den ck.den
          d).val.2 i := by
  rw [(paxos_core (Eager L mem) variant prop acc f cp ck
    (pcdecE d)).val.2.agree i]
  rw [paxos_eager_den]

end HydroV2
