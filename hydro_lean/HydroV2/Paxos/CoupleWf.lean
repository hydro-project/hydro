import HydroV2.Paxos.CoupleSafety

/-!
# Machine-run Paxos safety: whole-program wf + the premise-free
headline (D41)

`paxos_co_wf` (the corner run of `paxos_core` carries no residual
obligation), its corollary `paxos_co_cpl` (the machine/denotational
coupling with **no** satisfiability premise), and the headline
`paxos_safe_sched'` — all assembled from the artifacts generated at
the tail of `Paxos/PaxosCore.lean`.
-/

namespace HydroV2

variable {L : Type} {mem : L → Nat} {P : Type} [DecidableEq P]

section PaxosWf

variable (pacing : (ℓ : L) → Fin (mem ℓ) → Nat → Bool) (prop acc : L)
  (T : Nat) (f : Nat)
  (cpS : (SchedSem L mem pacing).Stream prop P .totalOrder .exactlyOnce)
  (cpV : (Values L mem).Stream prop P .totalOrder .exactlyOnce)
  (hcp : ∀ t i, ((cpS i).view t) <+: cpV i)
  (ckS : (SchedSem L mem pacing).TickSingleton acc (Option Nat)
    .unbounded)
  (ckV : (Values L mem).TickSingleton acc (Option Nat) .unbounded)
  (hck : ∀ t j, ckS j t <+: ckV j)
  (sdec : PaxosCoreDec (SchedSem L mem pacing) (mem prop) (mem acc) P)

/-- **Whole-program well-formedness**: the corner run of `paxos_core`
carries no residual obligation — all five knots' triples are
discharged by construction (D41; the generated
`paxos_core_co_wf₂`). -/
theorem paxos_co_wf :
    (paxos_core (CoupleSem L mem pacing T T (Nat.le_refl T))
        .guarded prop acc f (CoStream.inputC cpS cpV hcp)
        (CoTickSing.inputC ckS ckV hck)
        (paxosCoDec pacing prop acc T sdec)).val.2.wf :=
  paxos_core_co_wf₂ .guarded prop acc f (CoStream.inputC cpS cpV hcp)
    (CoTickSing.inputC ckS ckV hck) (paxosCoDec pacing prop acc T sdec)
    trivial trivial

/-- **The coupling, premise-free**: the machine run's replica views
below any horizon sit inside the denotational pools. -/
theorem paxos_co_cpl (i : Fin (mem prop)) :
    ListLe .noOrder .exactlyOnce
      (((paxos_core (CoupleSem L mem pacing T T (Nat.le_refl T))
          .guarded prop acc f (CoStream.inputC cpS cpV hcp)
          (CoTickSing.inputC ckS ckV hck)
          (paxosCoDec pacing prop acc T sdec)).val.2.sr i).view T)
      ((paxos_core (CoupleSem L mem pacing T T (Nat.le_refl T))
          .guarded prop acc f (CoStream.inputC cpS cpV hcp)
          (CoTickSing.inputC ckS ckV hck)
          (paxosCoDec pacing prop acc T sdec)).val.2.rr i) :=
  (paxos_core (CoupleSem L mem pacing T T (Nat.le_refl T))
      .guarded prop acc f (CoStream.inputC cpS cpV hcp)
      (CoTickSing.inputC ckS ckV hck)
      (paxosCoDec pacing prop acc T sdec)).val.2.cpl
    (paxos_co_wf pacing prop acc T f cpS cpV hcp ckS ckV hck sdec) i

include cpV ckV hcp hck in
/-- **Machine-run Paxos safety, premise-free** (the D41 headline): any
two commits the step machine has emitted at one slot, by any horizon,
under any pacing and any schedule, agree — with **no** satisfiability
premise and **no** decision argument. (Stated here rather than in
`CoupleSafety.lean` because it consumes `paxos_co_cpl`, which lives
above `CoupleSafety` in the import order.) -/
theorem paxos_safe_sched' (hnA : mem acc ≤ 2 * f + 1)
    (i j : Fin (mem prop)) (s : Nat) (w w' : Option P)
    (hw : (s, w) ∈ ((paxos_core (SchedSem L mem pacing) .guarded prop
      acc f cpS ckS sdec).val.2 i).view T)
    (hw' : (s, w') ∈ ((paxos_core (SchedSem L mem pacing) .guarded prop
      acc f cpS ckS sdec).val.2 j).view T) :
    w = w' := by
  have hens := ((paxos_core (Values L mem) .guarded prop acc f cpV ckV
    (paxosVDecG pacing prop acc T f cpS ckS
      sdec)).property rfl).slot_functional rfl hnA
  have hcpl_i := paxos_co_cpl pacing prop acc T f cpS cpV hcp ckS ckV
    hck sdec i
  have hcpl_j := paxos_co_cpl pacing prop acc T f cpS cpV hcp ckS ckV
    hck sdec j
  rw [paxos_co_sr pacing prop acc T f cpS cpV hcp ckS ckV hck sdec,
    paxos_co_rr pacing prop acc T f cpS cpV hcp ckS ckV hck sdec]
    at hcpl_i hcpl_j
  exact hens i j s w w'
    (Multiset.mem_of_le hcpl_i (Multiset.mem_coe.mpr hw))
    (Multiset.mem_of_le hcpl_j (Multiset.mem_coe.mpr hw'))

end PaxosWf

end HydroV2
