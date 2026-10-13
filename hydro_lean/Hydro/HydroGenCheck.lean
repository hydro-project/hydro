import Hydro.HydroGenToy

/-!
# Hydro · `HydroGen` validation (`HydroGenCheck`)

The artifacts `hydro def` generates for the toy modules, pinned by
`#check`, and consumed end to end in a machine-run safety theorem
(the `cq_safe_sched'` shape).
-/

namespace Hydro

/-! The knot-free module: namings (choice route), derived decision,
causality, wf, monotonicity, free theorems. -/

#check @toy_relay_co_sr₁
#check @toy_relay_co_sr₂
#check @toy_relay_co_rr_ex
#check @toy_relay_vdec
#check @toy_relay_co_rr₁
#check @toy_relay_co_rr₂
#check @toy_relay_causal₁
#check @toy_relay_causal₂
#check @toy_relay_co_wf₁
#check @toy_relay_co_wf₂
#check @toy_relay_mono₁
#check @toy_relay_mono₂
#check @toy_relay_param₁
#check @toy_relay.ensures

/-! The glue module (calls `toy_relay`): the same stack over a folded
callee. -/

#check @toy_step_co_sr₁
#check @toy_step_co_rr₁
#check @toy_step_vdec
#check @toy_step_causal₁
#check @toy_step_co_wf₁
#check @toy_step_mono₁
#check @toy_step_param

/-! The knot: the structural route — namings, causality, monotonicity,
the wf triple, the Kleene-chain vocabulary, the hoisted invariant. -/

#check @toy_loop.w_co_sr₁
#check @toy_loop.w_co_rr₁
#check @toy_loop.w_causal₁
#check @toy_loop.w_mono₁
#check @toy_loop.w_co_wf₁
#check @toy_loop.w.stages
#check @toy_loop.w.stages_zero
#check @toy_loop.w.stages_succ
#check @toy_loop.w.stages_fix
#check @toy_loop.w.stages_mono
#check @toy_loop.w.inv
#check @toy_loop_param

/-! ## End-to-end: a machine-run safety theorem through the generated
stack (the `cq_safe_sched'` shape) — every value the STEP MACHINE's
`toy_relay` emits, at any pacing, any schedule, any horizon, is
positive, by: the corner's `cpl` (via the generated wf), the two
generated namings, and the colocated `Values` contract. -/

theorem toy_safe_sched
    {L : Type} {mem : L → Nat}
    {pacing : (ℓ : L) → Fin (mem ℓ) → Nat → Bool} (ℓ : L)
    (h : (SchedSem L mem pacing).Stream ℓ Nat .noOrder .exactlyOnce)
    (v : (Values L mem).Stream ℓ Nat .noOrder .exactlyOnce)
    (hc : ∀ t i, ListLe .noOrder .exactlyOnce ((h i).view t) (v i))
    (pt : (SchedSem L mem pacing).PulseDec (mem ℓ))
    (T : Nat) (i : Fin (mem ℓ)) (x : Nat)
    (hx : x ∈ ((toy_relay (SchedSem L mem pacing) ℓ h () pt).1 i).view T) :
    1 ≤ x := by
  have hwf := toy_relay_co_wf₁ (Tc := T) (Td := T)
    (hjT := Nat.le_refl T) (pacing := pacing) ℓ
    (CoStream.inputC h v hc) (CoDec.batch Nat) pt trivial
  have hcpl := (toy_relay (CoupleSem L mem pacing T T (Nat.le_refl T))
    ℓ (CoStream.inputC h v hc) (CoDec.batch Nat) pt).1.cpl hwf i
  have hsr := toy_relay_co_sr₁ (Tc := T) (Td := T)
    (hjT := Nat.le_refl T) (pacing := pacing) ℓ
    (CoStream.inputC h v hc) (CoDec.batch Nat) pt
  have hrr := toy_relay_co_rr₁ (Tc := T) (Td := T)
    (hjT := Nat.le_refl T) (pacing := pacing) ℓ
    (CoStream.inputC h v hc) (CoDec.batch Nat) pt
  rw [hsr, hrr] at hcpl
  have hens := toy_relay.ensures ℓ v
    (toy_relay_vdec (pacing := pacing) (Td := T) ℓ h pt) pt
  exact hens.pos i x (Multiset.mem_of_le hcpl (Multiset.mem_coe.mpr hx))

/-! Trust audit: the generated artifacts sit on the standard three
axioms (choice enters only through the `vdec` choice-naming). -/

#print axioms toy_loop.w_co_wf₁
#print axioms toy_loop.w_co_rr₁
#print axioms toy_loop.w.inv
#print axioms toy_safe_sched

end Hydro
