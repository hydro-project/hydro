import HydroV2.HydroGenKnot
import HydroV2.HydroGenToy

/-!
# HydroV2 · `HydroGen` validation (`HydroGenCheck`)

The generated artifacts, exercised on the toy modules and checked
against the hand-written house patterns.
-/

namespace HydroV2

set_option maxHeartbeats 1600000 in
hydro_couple toy_relay

#check @toy_relay_co_sr₁
#check @toy_relay_co_sr₂
#check @toy_relay_co_rr_ex
#check @toy_relay_vdec
#check @toy_relay_co_rr₁
#check @toy_relay_co_rr₂

set_option maxHeartbeats 1600000 in
hydro_causal toy_relay

set_option maxHeartbeats 1600000 in
hydro_wf toy_relay

set_option maxHeartbeats 1600000 in
hydro_mono toy_relay

#check @toy_relay_causal₁
#check @toy_relay_causal₂
#check @toy_relay_co_wf₁
#check @toy_relay_co_wf₂
#check @toy_relay_mono₁
#check @toy_relay_mono₂

/-! The knot body module: full stack. -/

set_option maxHeartbeats 1600000 in
hydro_couple toy_step
set_option maxHeartbeats 1600000 in
hydro_causal toy_step
set_option maxHeartbeats 1600000 in
hydro_wf toy_step
set_option maxHeartbeats 1600000 in
hydro_mono toy_step

/-! The knot: the full generated stack (`hydro_knot` below emits the
namings, causality, monotonicity, and the wf triple). -/

section KnotBlueprint

variable {L : Type} {mem : L → Nat}
  {pacing : (ℓ : L) → Fin (mem ℓ) → Nat → Bool} {Tc Td : Nat}
  {hjT : Tc ≤ Td} (ℓ : L)
  (inp : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ Nat
    .noOrder .exactlyOnce)
  (dec : (CoupleSem L mem pacing Tc Td hjT).BatchDec (mem ℓ) Nat)
  (pt : (CoupleSem L mem pacing Tc Td hjT).PulseDec (mem ℓ))
  (de : (CoupleSem L mem pacing Tc Td hjT).EmitDec (mem ℓ) Nat)
  (df : (CoupleSem L mem pacing Tc Td hjT).FixDec)

/-- **The knot wf** (the `cc_wf` pattern at the generated-artifact
interface: hcaus from the body's generated causality, hchain from its
generated monotonicity, hcplj from its H'-generic reinstantiation at
the lowered corner bridged by its generated namings). -/
theorem toy_loop_co_wf_blueprint (hwi : inp.wf) :
    (toy_loop (CoupleSem L mem pacing Tc Td hjT) ℓ inp dec pt de
      df).wf := by
  co_wf_simp [toy_loop]
  refine ⟨?hcaus, ?hchain, ?hcplj⟩
  case hcaus =>
    intro h' x y hxy
    have hx := toy_step_co_sr₁ (Tc := Tc) (Td := Td) (hjT := hjT)
      (pacing := pacing) ℓ (CoStream.schedC x) inp dec pt de
    have hy := toy_step_co_sr₁ (Tc := Tc) (Td := Td) (hjT := hjT)
      (pacing := pacing) ℓ (CoStream.schedC y) inp dec pt de
    show SAgree h'
      ((toy_step (CoupleSem L mem pacing Tc Td hjT) ℓ
        (CoStream.schedC x) inp dec pt de).sr)
      ((toy_step (CoupleSem L mem pacing Tc Td hjT) ℓ
        (CoStream.schedC y) inp dec pt de).sr)
    rw [hx, hy]
    exact toy_step_causal₁ (pacing := pacing) ℓ () pt de hxy
      (SAgree.refl _ _)
  case hchain =>
    intro m i
    have hva : ∀ (v : Fin (mem ℓ) → PoolCarrier Nat .noOrder
        .exactlyOnce),
        CoStream.vbody
          ((fun (H'' : HydroSem L mem)
              (caps : H''.BatchDec (mem ℓ) Nat × H''.PulseDec (mem ℓ)
                × H''.EmitDec (mem ℓ) Nat
                × H''.Stream ℓ Nat .noOrder .exactlyOnce) s =>
              toy_step H'' ℓ s caps.2.2.2 caps.1 caps.2.1 caps.2.2.1)
            (CoupleSem L mem pacing Tc Td hjT) (dec, pt, de, inp)) v
        = toy_step (Values L mem) ℓ v inp.rr
            (toy_step_vdec (pacing := pacing) (Td := Td) ℓ
              (CoStream.fixSr
                ((fun (H'' : HydroSem L mem)
                    (caps : H''.BatchDec (mem ℓ) Nat
                      × H''.PulseDec (mem ℓ)
                      × H''.EmitDec (mem ℓ) Nat
                      × H''.Stream ℓ Nat .noOrder .exactlyOnce) s =>
                    toy_step H'' ℓ s caps.2.2.2 caps.1 caps.2.1
                      caps.2.2.1)
                  (CoupleSem L mem pacing Tc Td hjT)
                  (dec, pt, de, inp)))
              inp.sr pt de)
            pt () := by
      intro v
      exact toy_step_co_rr₁ (Tc := Tc) (Td := Td) (hjT := hjT)
        (pacing := pacing) ℓ
        (CoStream.probe2 (CoStream.fixSr _) v) inp dec pt de
    refine iterate_le_succ
      (R := fun (a b : Fin (mem ℓ)
          → PoolCarrier Nat .noOrder .exactlyOnce) =>
        ∀ j, PoolLe .noOrder .exactlyOnce (a j) (b j))
      (fun j => poolBot_le _ _ _) (fun {a b} hab j => ?_) m i
    rw [hva a, hva b]
    exact toy_step_mono₁ ℓ _ _ _ hab
      (fun _ => PoolLe.refl _ _ _) j
  case hcplj =>
    intro j hj x v' hxv i
    have hwf := toy_step_co_wf₁ (Tc := j) (Td := Td)
      (hjT := le_trans hj hjT) (pacing := pacing) ℓ
      (CoStream.mkC x v' (fun i' => hxv j (Nat.le_refl _) i'))
      (CoStream.lowerC hj (le_trans hj hjT) inp)
      dec pt de trivial hwi
    have hout := (toy_step (CoupleSem L mem pacing j Td
        (le_trans hj hjT)) ℓ
        (CoStream.mkC x v' (fun i' => hxv j (Nat.le_refl _) i'))
        (CoStream.lowerC hj (le_trans hj hjT) inp)
        dec pt de).cpl hwf i
    have hsr_eq := (toy_step_co_sr₁ (Tc := j) (Td := Td)
        (hjT := le_trans hj hjT) (pacing := pacing) ℓ
        (CoStream.mkC x v' (fun i' => hxv j (Nat.le_refl _) i'))
        (CoStream.lowerC hj (le_trans hj hjT) inp)
        dec pt de).trans
      (toy_step_co_sr₁ (Tc := Tc) (Td := Td) (hjT := hjT)
        (pacing := pacing) ℓ (CoStream.schedC x) inp dec pt de).symm
    have hrr_eq := (toy_step_co_rr₁ (Tc := j) (Td := Td)
        (hjT := le_trans hj hjT) (pacing := pacing) ℓ
        (CoStream.mkC x v' (fun i' => hxv j (Nat.le_refl _) i'))
        (CoStream.lowerC hj (le_trans hj hjT) inp)
        dec pt de).trans
      (toy_step_co_rr₁ (Tc := Tc) (Td := Td) (hjT := hjT)
        (pacing := pacing) ℓ (CoStream.probe2 x v') inp dec pt
        de).symm
    rw [hsr_eq, hrr_eq] at hout
    exact hout

end KnotBlueprint

/-! The mechanized knot wf (reproduces the blueprint; the
`toy_loop_co_wf_blueprint` above is the validated hand template). -/

set_option maxHeartbeats 1600000 in
hydro_knot toy_loop

#check @toy_loop_co_sr₁
#check @toy_loop_co_rr₁
#check @toy_loop_causal₁
#check @toy_loop_mono₁
#check @toy_loop_co_wf₁

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
    (de : (SchedSem L mem pacing).EmitDec (mem ℓ) Nat)
    (T : Nat) (i : Fin (mem ℓ)) (x : Nat)
    (hx : x ∈ ((toy_relay (SchedSem L mem pacing) ℓ h () pt
      de).val.1 i).view T) :
    1 ≤ x := by
  have hwf := toy_relay_co_wf₁ (Tc := T) (Td := T)
    (hjT := Nat.le_refl T) (pacing := pacing) ℓ
    (CoStream.inputC h v hc) (CoDec.batch Nat) pt de trivial
  have hcpl := (toy_relay (CoupleSem L mem pacing T T (Nat.le_refl T))
    ℓ (CoStream.inputC h v hc) (CoDec.batch Nat) pt
    de).val.1.cpl hwf i
  have hsr := toy_relay_co_sr₁ (Tc := T) (Td := T)
    (hjT := Nat.le_refl T) (pacing := pacing) ℓ
    (CoStream.inputC h v hc) (CoDec.batch Nat) pt de
  have hrr := toy_relay_co_rr₁ (Tc := T) (Td := T)
    (hjT := Nat.le_refl T) (pacing := pacing) ℓ
    (CoStream.inputC h v hc) (CoDec.batch Nat) pt de
  rw [hsr, hrr] at hcpl
  have hens := (toy_relay (Values L mem) ℓ v
    (toy_relay_vdec (pacing := pacing) (Td := T) ℓ h pt de) pt
    ()).property rfl
  exact hens.pos i x (Multiset.mem_of_le hcpl (Multiset.mem_coe.mpr hx))

/-! Trust audit: the generated artifacts sit on the standard three
axioms (choice enters only through the `vdec` choice-naming). -/

#print axioms toy_loop_co_wf₁
#print axioms toy_loop_co_rr₁
#print axioms toy_safe_sched

end HydroV2
