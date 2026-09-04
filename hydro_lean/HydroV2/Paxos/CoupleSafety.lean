import HydroV2.CoupleProj
import HydroV2.Paxos.PaxosCore

/-!
# Machine-run Paxos safety through the coupling corner (D39)

The `CoupleSem` pipeline at `paxos_core`: the corner run's machine leg
is the `SchedSem` run (`paxos_co_sr`), its denotational leg is the
`Values` run at the corner's derived decision record
(`paxos_co_rr` — the generated `paxos_core_vdec`). Both are direct
applications of the artifacts `hydro_glue paxos_core` generates at the
tail of `Paxos/PaxosCore.lean`. The whole-program well-formedness
(`paxos_co_wf`) and the premise-free headline (`paxos_safe_sched'`)
live in `Paxos/CoupleWf.lean`, which builds on these namings.
-/

namespace HydroV2

variable {L : Type} {mem : L → Nat} {P : Type} [DecidableEq P]

section CoupleSafety

variable (pacing : (ℓ : L) → Fin (mem ℓ) → Nat → Bool) (prop acc : L)
  (T : Nat)

/-- The machine schedule, re-packaged for the corner (machine data at
cursor/timing/emission sites, `Unit` at content sites — the corner
derives those itself). -/
def paxosCoDec
    (sdec : PaxosCoreDec (SchedSem L mem pacing) (mem prop) (mem acc) P) :
    PaxosCoreDec (CoupleSem L mem pacing T T (Nat.le_refl T))
      (mem prop) (mem acc) P :=
  { le := { receivedMax := CoDec.snap _ _
            hb := { sample := CoDec.sample sdec.le.hb.sample
                    timeout := CoDec.timer sdec.le.hb.timeout
                    interval := CoDec.pulse sdec.le.hb.interval
                    ial := CoDec.cursors sdec.le.hb.ial }
            p1aCh := CoDec.cursors sdec.le.p1aCh
            p1aBatch := CoDec.batch _
            p1bCh := CoDec.cursors sdec.le.p1bCh
            p1b := { cqwr := CoDec.batch _
                     cqwrEmit := CoDec.emit sdec.le.p1b.cqwrEmit
                     order := CoDec.orderSel _
                     snap := CoDec.snap _ _ }
            fuelFail := CoDec.fix
            fuelIAL := CoDec.fix
            fuelLead := CoDec.fix }
    sp := { payloadBatch := CoDec.ordBatch
            rcEmit := CoDec.emit sdec.sp.rcEmit
            p2aCh := CoDec.cursors sdec.sp.p2aCh
            p2aBatch := CoDec.batch _
            p2bCh := CoDec.cursors sdec.sp.p2bCh
            cqBatch := CoDec.batch _
            cqEmit := CoDec.emit sdec.sp.cqEmit
            jrBatch := CoDec.batch _
            jrEmit := CoDec.emit sdec.sp.jrEmit }
    fuelSeqMax := CoDec.fix
    fuelALog := CoDec.fix }

variable (f : Nat)
  (cpS : (SchedSem L mem pacing).Stream prop P .totalOrder .exactlyOnce)
  (cpV : (Values L mem).Stream prop P .totalOrder .exactlyOnce)
  (hcp : ∀ t i, (cpS i).view t <+: cpV i)
  (ckS : (SchedSem L mem pacing).TickSingleton acc (Option Nat)
    .unbounded)
  (ckV : (Values L mem).TickSingleton acc (Option Nat) .unbounded)
  (hck : ∀ t j, ckS j t <+: ckV j)
  (sdec : PaxosCoreDec (SchedSem L mem pacing) (mem prop) (mem acc) P)

/-- **The machine-projection identity**: the corner run's machine leg
is the `SchedSem` run — the generated `paxos_core_co_sr₂` (input
gadgets and the machine decision repack collapse definitionally). -/
theorem paxos_co_sr :
    (paxos_core (CoupleSem L mem pacing T T (Nat.le_refl T))
        .guarded prop acc f (CoStream.inputC cpS cpV hcp)
        (CoTickSing.inputC ckS ckV hck)
        (paxosCoDec pacing prop acc T sdec)).val.2.sr
      = (paxos_core (SchedSem L mem pacing) .guarded prop acc f
          cpS ckS sdec).val.2 :=
  paxos_core_co_sr₂ .guarded prop acc f (CoStream.inputC cpS cpV hcp)
    (CoTickSing.inputC ckS ckV hck) (paxosCoDec pacing prop acc T sdec)

/-- The corner-derived `Values` decision record at the guarded
corner: the generated `paxos_core_vdec` applied at the machine
schedule's pass-through sites (fix fuels are the literal `T + 1`;
content decisions are the corner's own derived witnesses;
`noncomputable` — generated definitions carry no compiled code). -/
noncomputable def paxosVDecG
    (sdec : PaxosCoreDec (SchedSem L mem pacing) (mem prop) (mem acc) P) :
    PaxosCoreDec (Values L mem) (mem prop) (mem acc) P :=
  let dec := paxosCoDec pacing prop acc T sdec
  paxos_core_vdec (pacing := pacing) (Td := T) .guarded prop acc f
    cpS ckS
    dec.le.hb.sample dec.le.hb.timeout dec.le.hb.interval dec.le.hb.ial
    dec.le.p1aCh dec.le.p1bCh dec.le.p1b.cqwrEmit dec.sp.rcEmit
    dec.sp.p2aCh dec.sp.p2bCh dec.sp.cqEmit dec.sp.jrEmit

/-- **The naming identity**: the corner run's denotational leg is the
`Values` run at the corner's own derived decisions (the generated
`paxos_core_vdec` — an explicit record; no satisfiability premise, no
metavariables). -/
theorem paxos_co_rr :
    (paxos_core (CoupleSem L mem pacing T T (Nat.le_refl T))
        .guarded prop acc f (CoStream.inputC cpS cpV hcp)
        (CoTickSing.inputC ckS ckV hck)
        (paxosCoDec pacing prop acc T sdec)).val.2.rr
      = (paxos_core (Values L mem) .guarded prop acc f
          cpV ckV
          (paxosVDecG pacing prop acc T f cpS ckS sdec)).val.2 :=
  paxos_core_co_rr₂ .guarded prop acc f (CoStream.inputC cpS cpV hcp)
    (CoTickSing.inputC ckS ckV hck) (paxosCoDec pacing prop acc T sdec)

end CoupleSafety

end HydroV2
