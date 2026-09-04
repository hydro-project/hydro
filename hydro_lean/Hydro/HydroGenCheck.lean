import Hydro.HydroGenToy

/-!
# `HydroGen` checks (`HydroGenCheck`)

The artifacts `hydro def` generates for the toy modules, pinned by `#check`, and
consumed end to end in a machine-run safety theorem (`toy_safe_sched`).
-/

namespace Hydro

/-! The knot-free module: the `sr`/`rr` equations (the choice route), the derived decision,
causality, wf, monotonicity, the face theorem. -/

#check @toy_relay_co_sr₁
#check @toy_relay_co_sr₂
#check @toy_relay_co_rr_ex
#check @toy_relay_vdec
#check @toy_relay_co_rr₁
#check @toy_relay_co_rr₂
#check @toy_relay_co_wf₁
#check @toy_relay_co_wf₂
#check @toy_relay_mono₁
#check @toy_relay_mono₂
#check @toy_relay.ensures

/-! The caller module `toy_step` (calls `toy_relay`, reaches no knot, so the
couple phase): the same stack over a folded callee. -/

#check @toy_step_co_sr₁
#check @toy_step_co_rr₁
#check @toy_step_vdec
#check @toy_step_co_wf₁
#check @toy_step_mono₁

/-! The knot: the structural route. The `sr`/`rr` equations, causality, monotonicity, the wf
triple, the Kleene-chain vocabulary, the hoisted invariant. -/

#check @toy_loop.w_co_sr₁
#check @toy_loop.w_co_rr₁
#check @toy_loop.w_strict₁
#check @toy_loop.w_mono₁
#check @toy_loop.w_co_wf₁
#check @toy_loop.w.stages
#check @toy_loop.w.stages_zero
#check @toy_loop.w.stages_succ
#check @toy_loop.w.stages_fix
#check @toy_loop.w.stages_mono
#check @toy_loop.w.inv

/-! ## End to end: a machine-run safety theorem through the generated stack.
Every value the step machine's `toy_relay` emits, at any pacing, any schedule
and any horizon, is positive, by the corner's `cpl` (through the generated wf),
the generated `sr` and `rr` equations, and the colocated `Values` contract. -/

theorem toy_safe_sched
    {L : Type} {mem : L → Nat}
    {pacing : Pacing L mem} (ℓ : L)
    (h : (SchedSem L mem pacing).Stream ℓ Nat .noOrder .exactlyOnce)
    (v : (Values L mem).Stream ℓ Nat .noOrder .exactlyOnce)
    (hc : ∀ t i, ListLe .noOrder .exactlyOnce ((h i).view t) (v i))
    (fires : (SchedSem L mem pacing).IntervalDec (mem ℓ))
    (T : Nat) (i : Fin (mem ℓ)) (x : Nat)
    (hx : x ∈ ((toy_relay (SchedSem L mem pacing) ℓ h ⟨(), ()⟩ ⟨fires⟩).1 i).view T) :
    1 ≤ x := by
  have hwf := toy_relay_co_wf₁ (Tc := T) (Td := T)
    (hjT := Nat.le_refl T) (pacing := pacing) ℓ
    (CoStream.inputC h v hc) ⟨CoDec.batch Nat, CoDec.batch Unit⟩ ⟨CoDec.interval fires⟩ trivial
  have hcpl := (toy_relay (CoupleSem L mem pacing T T (Nat.le_refl T))
    ℓ (CoStream.inputC h v hc) ⟨CoDec.batch Nat, CoDec.batch Unit⟩ ⟨CoDec.interval fires⟩).1.cpl hwf
    T (by co_lag [toy_relay_co_lag₁]) i
  have hsr := toy_relay_co_sr₁ (Tc := T) (Td := T)
    (hjT := Nat.le_refl T) (pacing := pacing) ℓ
    (CoStream.inputC h v hc) ⟨CoDec.batch Nat, CoDec.batch Unit⟩ ⟨CoDec.interval fires⟩
  have hrr := toy_relay_co_rr₁ (Tc := T) (Td := T)
    (hjT := Nat.le_refl T) (pacing := pacing) ℓ
    (CoStream.inputC h v hc) ⟨CoDec.batch Nat, CoDec.batch Unit⟩ ⟨CoDec.interval fires⟩
  rw [hsr, hrr] at hcpl
  have hens := toy_relay.ensures ℓ v
    (toy_relay_vdec (pacing := pacing) (Td := T) ℓ h fires) ⟨()⟩
  exact hens.pos i x (Multiset.mem_of_le hcpl (Multiset.mem_coe.mpr hx))

/-! Trust audit: the generated artifacts sit on the standard three axioms;
choice enters only through the `_vdec` decisions, which the generator defines by
`Classical.choose`. -/

#print axioms toy_loop.w_co_wf₁
#print axioms toy_loop.w_co_rr₁
#print axioms toy_loop.w.inv
#print axioms toy_safe_sched

end Hydro
