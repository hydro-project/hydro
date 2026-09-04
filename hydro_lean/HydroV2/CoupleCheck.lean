import HydroV2.CoupleProj

/-!
# HydroV2 · coupling-corner validation (`CoupleCheck`)

The D39 mechanism, end to end on the blueprint knot (a `fix` whose
body batches the knot wire and re-enters through `allTicks ∘ union`
with an external input):

1. **naming** — the corner run's denotational leg *is* the `Values`
   run at a decision record read off by unification (`∃ d`, one
   `co_transfer`);
2. **machine naming** — the corner run's machine leg *is* the
   `SchedSem` run (one `co_transfer`);
3. **well-formedness** — the knot's three residual obligations,
   discharged from the `∀ H'`-generic body: causality (a per-op
   congruence walk), the Kleene chain (`Values` monotonicity), and the
   graded coupling (re-instantiating the body at the `j`-lowered
   interpretation — the H'-genericity payoff);
4. **the coupling** — machine views below the horizon sit inside the
   denotational pools (`cpl` at the discharged `wf`).
-/

namespace HydroV2

section CoupleCheck

variable {L : Type} {mem : L → Nat}
  {pacing : (ℓ : L) → Fin (mem ℓ) → Nat → Bool} {ℓ : L}

/-- The blueprint body, generic over the interpretation (captures: the
batch decision and the external input). -/
def ccBody (H' : HydroSem L mem)
    (caps : H'.BatchDec (mem ℓ) Nat
      × H'.Stream ℓ Nat .noOrder .exactlyOnce)
    (s : H'.Stream ℓ Nat .noOrder .exactlyOnce) :
    H'.Stream ℓ Nat .noOrder .exactlyOnce :=
  H'.union (H'.allTicks (H'.batch s caps.1)) caps.2

/-- The blueprint program: the closed knot, batched and merged again
with the input (so the batch site occurs at top level *and* inside the
knot). -/
def ccProg (H' : HydroSem L mem)
    (caps : H'.BatchDec (mem ℓ) Nat
      × H'.Stream ℓ Nat .noOrder .exactlyOnce)
    (df : H'.FixDec) : H'.Stream ℓ Nat .noOrder .exactlyOnce :=
  ccBody H' caps
    (H'.fix df caps (fun H'' caps' s => ccBody H'' caps' s))

variable (T : Nat)
  (h : (SchedSem L mem pacing).Stream ℓ Nat .noOrder .exactlyOnce)
  (v : (Values L mem).Stream ℓ Nat .noOrder .exactlyOnce)
  (hc : ∀ t i, ListLe .noOrder .exactlyOnce ((h i).view t) (v i))

/-- The corner run of the blueprint program. -/
noncomputable def ccRun :=
  ccProg (CoupleSem L mem pacing T T (Nat.le_refl T))
    (CoDec.batch Nat, CoStream.inputC h v hc) CoDec.fix

/-- **Machine naming**: the corner's machine leg is the `SchedSem`
run. -/
theorem cc_sr (sbd : (SchedSem L mem pacing).BatchDec (mem ℓ) Nat)
    (sdf : (SchedSem L mem pacing).FixDec) :
    (ccRun (pacing := pacing) T h v hc).sr
      = ccProg (SchedSem L mem pacing) (sbd, h) sdf := by
  show (ccProg (CoupleSem L mem pacing T T (Nat.le_refl T))
    (CoDec.batch Nat, CoStream.inputC h v hc) CoDec.fix).sr = _
  co_transfer [ccProg, ccBody, co_fixSrC_eq, co_tick_fixSrC_eq]

/-- **Naming**: the corner's denotational leg is the `Values` run at
*some* decision record — read off by unification. -/
theorem cc_rr :
    ∃ dc : (Values L mem).BatchDec (mem ℓ) Nat × (Values L mem).FixDec,
      (ccRun (pacing := pacing) T h v hc).rr
        = ccProg (Values L mem) (dc.1, v) dc.2 := by
  apply Exists.intro ((_, _))
  show (ccProg (CoupleSem L mem pacing T T (Nat.le_refl T))
    (CoDec.batch Nat, CoStream.inputC h v hc) CoDec.fix).rr = _
  co_transfer [ccProg, ccBody]

/-- **Well-formedness**: the knot's three residual obligations. The
graded coupling (`hcplj`) is the H'-genericity payoff: the body is
re-instantiated at the `j`-lowered interpretation, whose carrier-borne
coupling *is* the obligation — no tactic walks the body's semantics. -/
theorem cc_wf : (ccRun (pacing := pacing) T h v hc).wf := by
  show (ccProg (CoupleSem L mem pacing T T (Nat.le_refl T))
    (CoDec.batch Nat, CoStream.inputC h v hc) CoDec.fix).wf
  co_wf_simp [ccProg, ccBody]
  refine ⟨?hcaus, ?hchain, ?hcplj⟩
  case hcaus =>
    intro h' x y hxy
    show SAgree h' ((SchedSem L mem pacing).union
        ((SchedSem L mem pacing).allTicks
          ((SchedSem L mem pacing).batch x ())) h)
      ((SchedSem L mem pacing).union
        ((SchedSem L mem pacing).allTicks
          ((SchedSem L mem pacing).batch y ())) h)
    exact causal_union
      (causal_allTicks (causal_batch () hxy)) (SAgree.refl h' h)
  case hchain =>
    intro m i
    refine iterate_le_succ
      (R := fun (a b : Fin (mem ℓ)
          → PoolCarrier Nat .noOrder .exactlyOnce) =>
        ∀ j, PoolLe .noOrder .exactlyOnce (a j) (b j))
      (fun j => poolBot_le _ _ _) (fun {a b} hab j => ?_) m i
    show PoolLe .noOrder .exactlyOnce
      ((Values L mem).union (ℓ := ℓ)
        ((Values L mem).allTicks (ord := .noOrder)
          ((Values L mem).batch a (fun i' => _))) v j)
      ((Values L mem).union (ℓ := ℓ)
        ((Values L mem).allTicks (ord := .noOrder)
          ((Values L mem).batch b (fun i' => _))) v j)
    exact le_trans
      (Multiset.add_le_add_right
        (sum_le_sum_of_prefix (batchCuts_le (hab j) 0 _)))
      (le_refl _)
  case hcplj =>
    intro j hj x v' hxv i
    -- re-instantiate the generic body at the j-lowered interpretation
    have hout := (ccBody (CoupleSem L mem pacing j T hj)
      (CoDec.batch Nat, CoStream.inputC h v hc)
      (CoStream.mkC x v' (fun i' => hxv j (Nat.le_refl _) i'))).cpl
      (by co_wf_simp [ccBody]) i
    -- its machine leg is the machine body's, its reader the probe's
    have hsr_eq : (ccBody (CoupleSem L mem pacing j T hj)
        (CoDec.batch Nat, CoStream.inputC h v hc)
        (CoStream.mkC x v' (fun i' => hxv j (Nat.le_refl _) i'))).sr
        = CoStream.sbody
          (fun s => ccBody (CoupleSem L mem pacing T T (Nat.le_refl T))
            (CoDec.batch Nat, CoStream.inputC h v hc) s) x := by
      co_transfer [ccBody, CoStream.sbody]
    have hrr_eq : (ccBody (CoupleSem L mem pacing j T hj)
        (CoDec.batch Nat, CoStream.inputC h v hc)
        (CoStream.mkC x v' (fun i' => hxv j (Nat.le_refl _) i'))).rr
        = ((fun s => ccBody (CoupleSem L mem pacing T T (Nat.le_refl T))
            (CoDec.batch Nat, CoStream.inputC h v hc) s)
          (CoStream.probe2 x v')).rr := by
      co_transfer [ccBody]
    rw [hsr_eq, hrr_eq] at hout
    exact hout

/-- **The coupling**: the machine run's views below the horizon sit
inside the denotational pools. -/
theorem cc_cpl (i : Fin (mem ℓ)) :
    ListLe .noOrder .exactlyOnce
      (((ccRun (pacing := pacing) T h v hc).sr i).view T)
      ((ccRun (pacing := pacing) T h v hc).rr i) :=
  (ccRun (pacing := pacing) T h v hc).cpl (cc_wf T h v hc) i

end CoupleCheck

end HydroV2
