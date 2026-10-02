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
variable {ckα : Type} [DecidableEq ckα] {ckord : StrOrd}
  {ckret : Retries}

/-- The machine schedule, re-packaged for the corner (machine data at
cursor/timing/emission sites, `Unit` at content sites — the corner
derives those itself). -/
def paxosCoDec
    (sdec : PaxosCoreDec (SchedSem L mem pacing) (mem prop) (mem acc)
      P ckα ckord) :
    PaxosCoreDec (CoupleSem L mem pacing T T (Nat.le_refl T))
      (mem prop) (mem acc) P ckα ckord :=
  { le := { receivedMax := CoDec.snap _ _
            hb := { sample := CoDec.sample sdec.le.hb.sample
                    timeout := CoDec.timer sdec.le.hb.timeout
                    interval := CoDec.pulse sdec.le.hb.interval }
            p1aBatch := CoDec.batch _
            p1b := { cqwr := CoDec.batch _
                     order := CoDec.orderSel _
                     snap := CoDec.snap _ _ }
            fuelFail := CoDec.fix
            fuelIAL := CoDec.fix
            fuelLead := CoDec.fix }
    sp := { payloadBatch := CoDec.ordBatch
            ap2 := { p2aBatch := CoDec.batch _
                     ckSnap := CoDec.snap _ _ }
            cqBatch := CoDec.batch _
            jrBatch := CoDec.batch _ }
    fuelSeqMax := CoDec.fix
    fuelALog := CoDec.fix }

/-- The machine sched-det bundle, re-packaged for the corner (cursor
and emission machine data at every site). -/
def paxosCoSched
    (ssched : PaxosCoreSched (SchedSem L mem pacing) (mem prop)
      (mem acc) P) :
    PaxosCoreSched (CoupleSem L mem pacing T T (Nat.le_refl T))
      (mem prop) (mem acc) P :=
  { le := { p1aCh := CoDec.cursors ssched.le.p1aCh
            hb := ⟨CoDec.cursors ssched.le.hb.ial⟩
            ap1 := ⟨CoDec.cursors ssched.le.ap1.p1bCh⟩
            p1b := ⟨⟨CoDec.emit ssched.le.p1b.cqwr.emit⟩⟩ }
    sp := { rcEmit := CoDec.emit ssched.sp.rcEmit
            p2aCh := CoDec.cursors ssched.sp.p2aCh
            ap2 := ⟨CoDec.cursors ssched.sp.ap2.p2bCh⟩
            cq := ⟨CoDec.emit ssched.sp.cq.emit⟩
            jr := ⟨CoDec.emit ssched.sp.jr.emit⟩ } }

variable (f : Nat)
  (cpS : (SchedSem L mem pacing).Stream prop P .totalOrder .exactlyOnce)
  (cpV : (Values L mem).Stream prop P .totalOrder .exactlyOnce)
  (hcp : ∀ t i, (cpS i).view t <+: cpV i)
  (ckS : (SchedSem L mem pacing).Singleton acc ckα (Option Nat) ckord
    ckret .unbounded)
  (ckV : (Values L mem).Singleton acc ckα (Option Nat) ckord ckret
    .unbounded)
  (hck : ∃ (g : Option Nat → ckα → Option Nat) (init : Option Nat)
    (ok : FoldOkP ckord ckret g)
    (pool : Fin (mem acc) → PoolCarrier ckα ckord ckret),
    (∀ j, ckV j = snapTrace ckord ckret g init ok (pool j)) ∧
    (∀ j, (ckS j).read = fun l => l.foldl g init) ∧
    (∀ t j, ListLe ckord ckret (((ckS j).src.view t)) (pool j)))
  (sdec : PaxosCoreDec (SchedSem L mem pacing) (mem prop) (mem acc)
    P ckα ckord)
  (ssched : PaxosCoreSched (SchedSem L mem pacing) (mem prop)
    (mem acc) P)

/-- **The machine-projection identity**: the corner run's machine leg
is the `SchedSem` run — the generated `paxos_core_co_sr₂` (input
gadgets and the machine decision repack collapse definitionally). -/
theorem paxos_co_sr :
    (paxos_core (CoupleSem L mem pacing T T (Nat.le_refl T))
        .guarded prop acc f (CoStream.inputC cpS cpV hcp)
        (CoSing.inputC ckS ckV hck)
        (paxosCoDec pacing prop acc T sdec)
        (paxosCoSched pacing prop acc T ssched)).val.2.sr
      = (paxos_core (SchedSem L mem pacing) .guarded prop acc f
          cpS ckS sdec ssched).val.2 :=
  paxos_core_co_sr₂ .guarded prop acc f (CoStream.inputC cpS cpV hcp)
    (CoSing.inputC ckS ckV hck) (paxosCoDec pacing prop acc T sdec)
    (paxosCoSched pacing prop acc T ssched)

/-- The corner-derived `Values` decision record at the guarded
corner: the generated `paxos_core_vdec` applied at the machine
schedule's pass-through sites (fix fuels are the literal `T + 1`;
content decisions are the corner's own derived witnesses;
`noncomputable` — generated definitions carry no compiled code). -/
noncomputable def paxosVDecG
    (sdec : PaxosCoreDec (SchedSem L mem pacing) (mem prop) (mem acc)
      P ckα ckord)
    (ssched : PaxosCoreSched (SchedSem L mem pacing) (mem prop)
      (mem acc) P) :
    PaxosCoreDec (Values L mem) (mem prop) (mem acc) P ckα ckord :=
  let dec := paxosCoDec pacing prop acc T sdec
  let csched := paxosCoSched pacing prop acc T ssched
  paxos_core_vdec (pacing := pacing) (Td := T) .guarded prop acc f
    cpS ckS
    dec.le.hb.sample dec.le.hb.timeout dec.le.hb.interval
    csched.le.p1aCh csched.le.hb.ial csched.le.ap1.p1bCh
    csched.le.p1b.cqwr.emit csched.sp.rcEmit
    csched.sp.p2aCh csched.sp.ap2.p2bCh csched.sp.cq.emit
    csched.sp.jr.emit

/-- **The naming identity**: the corner run's denotational leg is the
`Values` run at the corner's own derived decisions (the generated
`paxos_core_vdec` — an explicit record; no satisfiability premise, no
metavariables). -/
theorem paxos_co_rr :
    (paxos_core (CoupleSem L mem pacing T T (Nat.le_refl T))
        .guarded prop acc f (CoStream.inputC cpS cpV hcp)
        (CoSing.inputC ckS ckV hck)
        (paxosCoDec pacing prop acc T sdec)
        (paxosCoSched pacing prop acc T ssched)).val.2.rr
      = (paxos_core (Values L mem) .guarded prop acc f
          cpV ckV
          (paxosVDecG pacing prop acc T f cpS ckS sdec ssched)
          PaxosCoreSched.triv).val.2 :=
  paxos_core_co_rr₂ .guarded prop acc f (CoStream.inputC cpS cpV hcp)
    (CoSing.inputC ckS ckV hck) (paxosCoDec pacing prop acc T sdec)
    (paxosCoSched pacing prop acc T ssched)

end CoupleSafety

end HydroV2
