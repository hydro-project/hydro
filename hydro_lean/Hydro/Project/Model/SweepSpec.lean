import Hydro.Project.Process
import Hydro.Project.Model.StepAgree

/-!
# Hydro · Project — the sweep is the step (D95)

`Dag.iterate_eq_pass`: on a valid graph (`Dag.valid`, the lag analysis of
`Graph/Causal.lean`), the activation as a sweep (`Dag.sweepTop`,
`Project/Sweep.lean`) is the reference step `Dag.locStep` — the one
every process-level theorem (`Dag.pass_eq`, `Dag.step_wired`) is about.

The proof is an invariant over the sweeps of one step. Relative to the
reference step from the cells `s₀` on the full environment `Δf`:

- **consistency** (`Dag.Cons`): a `done` node holds the reference step's
  cells, outputs and sends; a `released` node holds its release half
  (`Node.release`); a `todo` node still holds `s₀`. A knot block's knots
  bound so far are the reference's (`knotRef`, the fixpoint the reference
  `fixN` iterates to), and its body is consistent with the reference on
  *every* environment of the knots not yet bound (nothing stepped so far
  has read them).
- **soundness** (`Dag.sweep_sound`): a sweep on an environment that agrees
  with `Δf` on the known wires (`Agree`) preserves consistency, and its
  known outputs are the reference's. A stepped node reads known wires only
  (`Node.locStep_congr`); a strict node's step is its release then its push
  (`Node.locStep_strict`); the knots a knot loop binds are the reference's
  because the reference is a fixpoint of the body's step
  (`Dag.locStep_knotIter_fix`, from `Dag.locStep_agree0` — the process's
  step respects the lag analysis).
- **knownness** (`Dag.sweep_known`): the output knownness is at least what
  the lag analysis predicts — known wires at lag `0`, unknown at `1`
  (`maskLags`); a knot block's loop binds every knot on a valid body with
  known externals (`knotLoop_all`: the Jacobi iteration of the body's lag
  functional, `iter_zero`/`postfix_zero`).
- **completeness** (`Dag.sweep_complete`): on a fully known environment
  everything ends `done`.

`Dag.Cons_final` reads the reference step off a fully `done` part.
-/

namespace Hydro

namespace Project

variable {L : Type} {mem : L → Nat}

/-! ## Agreement on the known wires -/

/-- Two environments agree on every wire the mask knows. -/
def Agree {f : Kind L mem → Type} : {κs : List (Kind L mem)} → Mask κs → HList f κs → HList f κs → Prop
  | [], _, _, _ => True
  | _ :: _, mk, a, b => (mk.1 = true → a.1 = b.1) ∧ Agree mk.2 a.2 b.2

namespace Agree
variable {f : Kind L mem → Type}

theorem refl : {κs : List (Kind L mem)} → (mk : Mask κs) → (a : HList f κs) → Agree mk a a
  | [], _, _ => trivial
  | _ :: _, mk, a => ⟨fun _ => rfl, refl mk.2 a.2⟩

theorem of_eq {κs : List (Kind L mem)} {mk : Mask κs} {a b : HList f κs} (h : a = b) : Agree mk a b :=
  h ▸ refl mk a

theorem get : {κs : List (Kind L mem)} → {κ : Kind L mem} → {mk : Mask κs} → {a b : HList f κs} →
    Agree mk a b → (v : Var κs κ) → mk.get v = true → a.get v = b.get v
  | _ :: _, _, _, _, _, h, .zero, hv => h.1 hv
  | _ :: _, _, _, _, _, h, .succ v, hv => get h.2 v hv

theorem append : {κs Γ : List (Kind L mem)} → {mk : Mask κs} → {mk' : Mask Γ} → {a b : HList f κs} →
    {a' b' : HList f Γ} → Agree mk a b → Agree mk' a' b' → Agree (mk.append mk') (a.append a') (b.append b')
  | [], _, _, _, _, _, _, _, _, h' => h'
  | _ :: _, _, _, _, _, _, _, _, h, h' => ⟨h.1, append h.2 h'⟩

theorem take : {κs Γ : List (Kind L mem)} → {mk : Mask (κs ++ Γ)} → {a b : HList f (κs ++ Γ)} → Agree mk a b →
    Agree (HList.take (κs := κs) mk) (HList.take (κs := κs) a) (HList.take (κs := κs) b)
  | [], _, _, _, _, _ => trivial
  | _ :: κs, Γ, _, _, _, h => ⟨h.1, take (κs := κs) (Γ := Γ) h.2⟩

theorem drop : {κs Γ : List (Kind L mem)} → {mk : Mask (κs ++ Γ)} → {a b : HList f (κs ++ Γ)} → Agree mk a b →
    Agree (HList.drop (κs := κs) mk) (HList.drop (κs := κs) a) (HList.drop (κs := κs) b)
  | [], _, _, _, _, h => h
  | _ :: κs, Γ, _, _, _, h => drop (κs := κs) (Γ := Γ) h.2

theorem mapSubst {Γ Γ' : Ctx L mem} {mk : Mask Γ} {a b : HList f Γ} (h : Agree mk a b) (args : Subst Γ' Γ) :
    Agree (args.map (fun v => mk.get v)) (args.map (fun v => a.get v)) (args.map (fun v => b.get v)) := by
  induction Γ' with
  | nil => trivial
  | cons κ κs ih => exact ⟨fun hv => h.get args.1 hv, ih args.2⟩

theorem const_false : {κs : List (Kind L mem)} → (a b : HList f κs) → Agree (Mask.const κs false) a b
  | [], _, _ => trivial
  | _ :: _, a, b => ⟨fun h => Bool.noConfusion h, const_false a.2 b.2⟩

theorem eq_of_all : {κs : List (Kind L mem)} → {mk : Mask κs} → {a b : HList f κs} → Mask.all mk = true →
    Agree mk a b → a = b
  | [], _, _, _, _, _ => rfl
  | _ :: _, mk, a, b, hk, h => by
    have hk' := Bool.and_eq_true_iff.mp hk
    show (a.1, a.2) = (b.1, b.2)
    rw [h.1 hk'.1, eq_of_all hk'.2 h.2]

theorem symm : {κs : List (Kind L mem)} → {mk : Mask κs} → {a b : HList f κs} → Agree mk a b → Agree mk b a
  | [], _, _, _, _ => trivial
  | _ :: _, _, _, _, h => ⟨fun hk => (h.1 hk).symm, symm h.2⟩

theorem trans : {κs : List (Kind L mem)} → {mk : Mask κs} → {a b c : HList f κs} → Agree mk a b → Agree mk b c → Agree mk a c
  | [], _, _, _, _, _, _ => trivial
  | _ :: _, _, _, _, _, h₁, h₂ => ⟨fun hk => (h₁.1 hk).trans (h₂.1 hk), trans h₁.2 h₂.2⟩

/-- Agreement on a disjunction gives agreement on its left half. -/
theorem of_or_left : {κs : List (Kind L mem)} → {b c : Mask κs} → {x y : HList f κs} → Agree (Mask.or b c) x y → Agree b x y
  | [], _, _, _, _, _ => trivial
  | _ :: _, b, c, _, _, h => ⟨fun hk => h.1 (by show (b.1 || c.1) = true; rw [hk]; rfl), of_or_left h.2⟩

/-- The selection keeps the known entries. -/
theorem select_left : {κs : List (Kind L mem)} → (b : Mask κs) → (a n : HList f κs) → Agree b (Mask.select b a n) a
  | [], _, _, _ => trivial
  | _ :: _, b, a, n => ⟨fun hk => by show (if b.1 then a.1 else n.1) = a.1; rw [hk]; rfl, select_left b.2 a.2 n.2⟩

/-- Binding the newly known entries from `n`: the result agrees with `r` on the
union when `a` does on `b` and `n` on `c`. -/
theorem or_select : {κs : List (Kind L mem)} → {b c : Mask κs} → {a n r : HList f κs} → Agree b a r → Agree c n r →
    Agree (Mask.or b c) (Mask.select b a n) r
  | [], _, _, _, _, _, _, _ => trivial
  | _ :: _, b, c, a, n, r, ha, hn => by
    refine ⟨fun hk => ?_, or_select ha.2 hn.2⟩
    show (if b.1 then a.1 else n.1) = r.1
    cases hb : b.1
    · have hc : c.1 = true := by
        have : (b.1 || c.1) = true := hk
        rw [hb] at this; simpa using this
      exact hn.1 hc
    · exact ha.1 hb

end Agree

/-! ## Masks -/

namespace Mask

theorem all_get : {κs : List (Kind L mem)} → {κ : Kind L mem} → {mk : Mask κs} → all mk = true → (v : Var κs κ) →
    mk.get v = true
  | _ :: _, _, _, h, .zero => (Bool.and_eq_true_iff.mp h).1
  | _ :: _, _, _, h, .succ v => all_get (Bool.and_eq_true_iff.mp h).2 v

theorem all_const_true : (κs : List (Kind L mem)) → all (const κs true) = true
  | [] => rfl
  | _ :: κs => by show (true && all (const κs true)) = true; rw [all_const_true κs]; rfl

theorem all_mapSubst {Γ Γ' : Ctx L mem} {mk : Mask Γ} (h : all mk = true) (args : Subst Γ' Γ) :
    all (args.map (fun v => mk.get v)) = true := by
  induction Γ' with
  | nil => rfl
  | cons κ κs ih =>
    show (mk.get args.1 && all (args.2.map (fun v => mk.get v))) = true
    rw [all_get h args.1, ih args.2]; rfl

theorem all_append : {κs Γ : List (Kind L mem)} → {a : Mask κs} → {b : Mask Γ} → all a = true → all b = true →
    all (a.append b) = true
  | [], _, _, _, _, hb => hb
  | _ :: _, _, a, b, ha, hb => by
    have ha' := Bool.and_eq_true_iff.mp ha
    show (a.1 && all (a.2.append b)) = true
    rw [ha'.1, all_append ha'.2 hb]; rfl

theorem all_take : {κs Γ : List (Kind L mem)} → {a : Mask (κs ++ Γ)} → all a = true → all (HList.take (κs := κs) a) = true
  | [], _, _, _ => rfl
  | _ :: κs, Γ, a, h => by
    have h' := Bool.and_eq_true_iff.mp h
    show (a.1 && all (HList.take (κs := κs) a.2)) = true
    rw [h'.1, all_take (κs := κs) (Γ := Γ) h'.2]; rfl

theorem all_drop : {κs Γ : List (Kind L mem)} → {a : Mask (κs ++ Γ)} → all a = true → all (HList.drop (κs := κs) a) = true
  | [], _, _, h => h
  | _ :: κs, Γ, _, h => all_drop (κs := κs) (Γ := Γ) (Bool.and_eq_true_iff.mp h).2

theorem all_or_left : {κs : List (Kind L mem)} → {a : Mask κs} → (c : Mask κs) → all a = true → all (or a c) = true
  | [], _, _, _ => rfl
  | _ :: _, a, c, h => by
    have h' := Bool.and_eq_true_iff.mp h
    show ((a.1 || c.1) && all (or a.2 c.2)) = true
    rw [h'.1, all_or_left c.2 h'.2]; rfl

theorem or_of_all : {κs : List (Kind L mem)} → {a : Mask κs} → (c : Mask κs) → all a = true → or a c = a
  | [], _, _, _ => rfl
  | _ :: _, a, c, h => by
    have h' := Bool.and_eq_true_iff.mp h
    show HList.cons (a.1 || c.1) (or a.2 c.2) = a
    rw [h'.1, or_of_all c.2 h'.2]
    show (true, a.2) = a
    rw [← h'.1]

theorem beq_iff : {κs : List (Kind L mem)} → {a b : Mask κs} → (beq a b = true ↔ a = b)
  | [], _, _ => ⟨fun _ => rfl, fun _ => rfl⟩
  | _ :: _, a, b => by
    show (((a.1 == b.1) && beq a.2 b.2) = true ↔ a = b)
    rw [Bool.and_eq_true_iff, beq_iff]
    constructor
    · intro h; exact Prod.ext (by simpa using h.1) h.2
    · intro h; subst h; exact ⟨by simp, rfl⟩

end Mask

theorem shapeKnown_of_all {Γ : Ctx L mem} {ℓ : L} {mk : Mask Γ} (h : Mask.all mk = true) :
    (sh : TickShape) → (x : VarsOf Γ ℓ sh) → shapeKnown mk sh x = true
  | .sing _, v => Mask.all_get h v
  | .stream _ _ _ _, v => Mask.all_get h v
  | .ksing _ _ _ _, v => Mask.all_get h v
  | .kstream _ _ _ _ _ _, v => Mask.all_get h v
  | .pair a b, (va, vb) => by
    show (shapeKnown mk a va && shapeKnown mk b vb) = true
    rw [shapeKnown_of_all h a va, shapeKnown_of_all h b vb]; rfl

/-- The lags a mask stands for: a known wire at `0`, an unknown one at `1`. -/
def maskLags : {κs : List (Kind L mem)} → Mask κs → Lags κs
  | [], _ => HList.nil
  | _ :: _, mk => HList.cons (if mk.1 then 0 else 1) (maskLags mk.2)

namespace maskLags

theorem get : {κs : List (Kind L mem)} → {κ : Kind L mem} → (mk : Mask κs) → (v : Var κs κ) →
    (maskLags mk).get v = if mk.get v then 0 else 1
  | _ :: _, _, _, .zero => rfl
  | _ :: _, _, mk, .succ v => get mk.2 v

theorem get_eq_one {κs : List (Kind L mem)} {κ : Kind L mem} (mk : Mask κs) (v : Var κs κ) (h : mk.get v = false) :
    (maskLags mk).get v = 1 := by rw [get, h]; rfl

theorem append : {κs Γ : List (Kind L mem)} → (a : Mask κs) → (b : Mask Γ) →
    maskLags (a.append b) = (maskLags a).append (maskLags b)
  | [], _, _, _ => rfl
  | _ :: _, _, a, b => by
    show HList.cons (if a.1 then 0 else 1) (maskLags (a.2.append b)) = HList.cons (if a.1 then 0 else 1) ((maskLags a.2).append (maskLags b))
    rw [append a.2 b]

theorem take : {κs Γ : List (Kind L mem)} → (a : Mask (κs ++ Γ)) →
    maskLags (HList.take (κs := κs) a) = HList.take (κs := κs) (maskLags a)
  | [], _, _ => rfl
  | _ :: κs, Γ, a => by
    show HList.cons (if a.1 then 0 else 1) (maskLags (HList.take (κs := κs) a.2)) = HList.cons (if a.1 then 0 else 1) (HList.take (κs := κs) (maskLags a.2))
    rw [take (κs := κs) (Γ := Γ) a.2]

theorem drop : {κs Γ : List (Kind L mem)} → (a : Mask (κs ++ Γ)) →
    maskLags (HList.drop (κs := κs) a) = HList.drop (κs := κs) (maskLags a)
  | [], _, _ => rfl
  | _ :: κs, Γ, a => drop (κs := κs) (Γ := Γ) a.2

theorem const_true : (κs : List (Kind L mem)) → maskLags (Mask.const κs true) = Lags.const κs 0
  | [] => rfl
  | _ :: κs => by
    show HList.cons 0 (maskLags (Mask.const κs true)) = HList.cons 0 (Lags.const κs 0)
    rw [const_true κs]

theorem const_false : (κs : List (Kind L mem)) → maskLags (Mask.const κs false) = Lags.const κs 1
  | [] => rfl
  | _ :: κs => by
    show HList.cons 1 (maskLags (Mask.const κs false)) = HList.cons 1 (Lags.const κs 1)
    rw [const_false κs]

theorem mapSubst {Γ Γ' : Ctx L mem} (mk : Mask Γ) (args : Subst Γ' Γ) :
    maskLags (args.map (fun v => mk.get v)) = args.map (fun v => (maskLags mk).get v) := by
  induction Γ' with
  | nil => rfl
  | cons κ κs ih =>
    show HList.cons (if mk.get args.1 then 0 else 1) (maskLags (args.2.map (fun v => mk.get v))) = HList.cons ((maskLags mk).get args.1) _
    rw [ih args.2, get]

theorem le_one : {κs : List (Kind L mem)} → (mk : Mask κs) → Lags.Le (maskLags mk) (Lags.const κs 1)
  | [], _ => trivial
  | _ :: _, mk => ⟨by show (if mk.1 then 0 else 1) ≤ 1; split <;> exact Nat.le_of_lt_succ (by omega), le_one mk.2⟩

theorem of_all : {κs : List (Kind L mem)} → {mk : Mask κs} → Mask.all mk = true → maskLags mk = Lags.const κs 0
  | [], _, _ => rfl
  | _ :: κs, mk, h => by
    have h' := Bool.and_eq_true_iff.mp h
    show HList.cons (if mk.1 then 0 else 1) (maskLags mk.2) = HList.cons 0 (Lags.const κs 0)
    rw [h'.1, of_all h'.2]; rfl

theorem all_of_le : {κs : List (Kind L mem)} → {mk : Mask κs} → Lags.Le (maskLags mk) (Lags.const κs 0) → Mask.all mk = true
  | [], _, _ => rfl
  | _ :: κs, mk, h => by
    show (mk.1 && Mask.all mk.2) = true
    have h1 : mk.1 = true := by
      have := h.1
      revert this
      show (if mk.1 then 0 else 1) ≤ 0 → mk.1 = true
      cases mk.1 <;> simp
    rw [h1, all_of_le h.2]; rfl

/-- Knowing more lowers the lags: the disjunction's lags are below either half's. -/
theorem or_le_right : {κs : List (Kind L mem)} → (a c : Mask κs) → Lags.Le (maskLags (Mask.or a c)) (maskLags c)
  | [], _, _ => trivial
  | _ :: _, a, c => ⟨by
      show (if (a.1 || c.1) then 0 else 1) ≤ (if c.1 then 0 else 1)
      cases a.1 <;> cases c.1 <;> simp, or_le_right a.2 c.2⟩

theorem one_le_maxAll : {κs : List (Kind L mem)} → {mk : Mask κs} → Mask.all mk = false → 1 ≤ Lags.maxAll (maskLags mk)
  | [], _, h => Bool.noConfusion h
  | _ :: κs, mk, h => by
    show 1 ≤ max (if mk.1 then 0 else 1) (Lags.maxAll (maskLags mk.2))
    cases hm : mk.1 with
    | true =>
      have : Mask.all mk.2 = false := by
        have h' : (mk.1 && Mask.all mk.2) = false := h
        rw [hm] at h'; simpa using h'
      exact (one_le_maxAll this).trans (Nat.le_max_right _ _)
    | false => exact (Nat.le_max_left _ _).trans' (Nat.le_refl 1)

end maskLags

namespace Lags

theorem Le.of_take_drop : {κs Γ : List (Kind L mem)} → {a b : Lags (κs ++ Γ)} →
    Le (HList.take (κs := κs) a) (HList.take (κs := κs) b) → Le (HList.drop (κs := κs) a) (HList.drop (κs := κs) b) → Le a b
  | [], _, _, _, _, hd => hd
  | _ :: κs, Γ, _, _, ht, hd => ⟨ht.1, Le.of_take_drop (κs := κs) (Γ := Γ) ht.2 hd⟩

end Lags

/-! ## A stepped node reads known wires only -/

theorem readShapeLoc_congr (m : Member L mem) {Γ : Ctx L mem} {mk : Mask Γ} {Δ Δ' : HList (Kind.locDelta m) Γ}
    (h : Agree mk Δ Δ') (ℓ : L) : (sh : TickShape) → (x : VarsOf Γ ℓ sh) → shapeKnown mk sh x = true →
    readShapeLoc m Δ ℓ sh x = readShapeLoc m Δ' ℓ sh x
  | .sing _, v, hk => by show HList.cons (Δ.get v) HList.nil = HList.cons (Δ'.get v) HList.nil; rw [h.get v hk]
  | .stream _ _ _ _, v, hk => by show HList.cons (Δ.get v) HList.nil = HList.cons (Δ'.get v) HList.nil; rw [h.get v hk]
  | .ksing _ _ _ _, v, hk => by show HList.cons (Δ.get v) HList.nil = HList.cons (Δ'.get v) HList.nil; rw [h.get v hk]
  | .kstream _ _ _ _ _ _, v, hk => by show HList.cons (Δ.get v) HList.nil = HList.cons (Δ'.get v) HList.nil; rw [h.get v hk]
  | .pair a b, (va, vb), hk => by
    have hk' := Bool.and_eq_true_iff.mp hk
    show (readShapeLoc m Δ ℓ a va).append (readShapeLoc m Δ ℓ b vb) = (readShapeLoc m Δ' ℓ a va).append (readShapeLoc m Δ' ℓ b vb)
    rw [readShapeLoc_congr m h ℓ a va hk'.1, readShapeLoc_congr m h ℓ b vb hk'.2]

/-- A strict node's push reads its (known) input only. -/
theorem Node.pushStep_congr (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} →
    (n : Node Γ κs) → {mk : Mask Γ} → {Δ Δ' : HList (Kind.locDelta m) Γ} →
    (s : Node.LocState m n) → (snd : Node.Sends m n) → Node.inputsKnown n mk = true → Agree mk Δ Δ' →
    Node.pushStep m n Δ s snd = Node.pushStep m n Δ' s snd

  | _, _, .map .., _, _, _, _, _, _, _ => rfl
  | _, _, .filterMap .., _, _, _, _, _, _, _ => rfl
  | _, _, .broadcast_closed _ x, _, _, _, _, _, hk, ha => by simp only [Node.pushStep, ha.get x hk]
  | _, _, .demux _ x _, _, _, _, _, _, hk, ha => by simp only [Node.pushStep, ha.get x hk]
  | _, _, .values .., _, _, _, _, _, _, _ => rfl
  | _, _, .into_keyed .., _, _, _, _, _, _, _ => rfl
  | _, _, .kassume_ordering .., _, _, _, _, _, _, _ => rfl
  | _, _, .kfold_early_stop .., _, _, _, _, _, _, _ => rfl
  | _, _, .kget_max_key .., _, _, _, _, _, _, _ => rfl
  | _, _, .weaken_retries .., _, _, _, _, _, _, _ => rfl
  | _, _, .union .., _, _, _, _, _, _, _ => rfl
  | _, _, .assume_ordering .., _, _, _, _, _, _, _ => rfl
  | _, _, .fold .., _, _, _, _, _, _, _ => rfl
  | _, _, .fold_monotone .., _, _, _, _, _, _, _ => rfl
  | _, _, .snapshot .., _, _, _, _, _, _, _ => rfl
  | _, _, .batch .., _, _, _, _, _, _, _ => rfl
  | _, _, .source_interval .., _, _, _, _, _, _, _ => rfl
  | _, _, .firstTick .., _, _, _, _, _, _, _ => rfl
  | _, _, .mapTick .., _, _, _, _, _, _, _ => rfl
  | _, _, .zipTick .., _, _, _, _, _, _, _ => rfl
  | _, _, .defer_tick _ x, _, _, _, _, _, hk, ha => by simp only [Node.pushStep, ha.get x hk]
  | _, _, .allTicks .., _, _, _, _, _, _, _ => rfl
  | _, _, .flattenOrdered .., _, _, _, _, _, _, _ => rfl
  | _, _, .flattenUnordered .., _, _, _, _, _, _, _ => rfl
  | _, _, .tinto_keyed .., _, _, _, _, _, _, _ => rfl
  | _, _, .tassume_ordering .., _, _, _, _, _, _, _ => rfl
  | _, _, .reduce_watermark .., _, _, _, _, _, _, _ => rfl
  | _, _, .kentries .., _, _, _, _, _, _, _ => rfl
  | _, _, .kmap .., _, _, _, _, _, _, _ => rfl
  | _, _, .kallTicks .., _, _, _, _, _, _, _ => rfl
  | _, _, .kfold .., _, _, _, _, _, _, _ => rfl
  | _, _, .ksnapshot .., _, _, _, _, _, _, _ => rfl
  | _, _, .tfold .., _, _, _, _, _, _, _ => rfl
  | _, _, .latest .., _, _, _, _, _, _, _ => rfl
  | _, _, .latest_monotone .., _, _, _, _, _, _, _ => rfl
  | _, _, .time, _, _, _, _, _, _, _ => rfl
  | _, _, .snapshot_bounded .., _, _, _, _, _, _, _ => rfl
  | _, _, .tick_scan .., _, _, _, _, _, _, _ => rfl
  | _, _, .call .., _, _, _, _, _, _, _ => rfl
  | _, _, .fixN .., _, _, _, _, _, _, _ => rfl

section Congr
variable [DecidableEq L]

/-- A node whose inputs are known computes the same on any environment that
agrees with the reference on the known wires. -/
theorem Node.locStep_congr (m : Member L mem) (tk : Bool) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} →
    (n : Node Γ κs) → (ora : Node.Oracle m n) → {mk : Mask Γ} → {Δ Δ' : HList (Kind.locDelta m) Γ} →
    (s : Node.LocState m n) → Node.inputsKnown n mk = true → Agree mk Δ Δ' →
    Node.locStep m tk n ora Δ s = Node.locStep m tk n ora Δ' s

  | _, _, .map _ _ x _, _, _, _, _, _, hk, ha => by simp only [Node.locStep, ha.get x hk]
  | _, _, .filterMap _ _ x _, _, _, _, _, _, hk, ha => by simp only [Node.locStep, ha.get x hk]
  | _, _, .broadcast_closed _ x, _, _, _, _, _, hk, ha => by simp only [Node.locStep, ha.get x hk]
  | _, _, .demux _ x _, _, _, _, _, _, hk, ha => by simp only [Node.locStep, ha.get x hk]
  | _, _, .values _ _ x, _, _, _, _, _, hk, ha => by simp only [Node.locStep, ha.get x hk]
  | _, _, .into_keyed _ _ x, _, _, _, _, _, hk, ha => by simp only [Node.locStep, ha.get x hk]
  | _, _, .kassume_ordering _ _ x _, _, _, _, _, _, hk, ha => by simp only [Node.locStep, ha.get x hk]
  | _, _, .kfold_early_stop _ _ _ x _ _, _, _, _, _, _, hk, ha => by simp only [Node.locStep, ha.get x hk]
  | _, _, .kget_max_key _ _ _ _ x, _, _, _, _, _, hk, ha => by simp only [Node.locStep, ha.get x hk]
  | _, _, .weaken_retries _ x, _, _, _, _, _, hk, ha => by simp only [Node.locStep, ha.get x hk]
  | _, _, .union _ a b, _, _, _, _, _, hk, ha => by
    have hk' := Bool.and_eq_true_iff.mp hk
    simp only [Node.locStep, ha.get a hk'.1, ha.get b hk'.2]
  | _, _, .assume_ordering _ x _, _, _, _, _, _, hk, ha => by simp only [Node.locStep, ha.get x hk]
  | _, _, .fold _ _ _ _ _ _ x, _, _, _, _, _, hk, ha => by simp only [Node.locStep, ha.get x hk]
  | _, _, .fold_monotone _ _ _ _ _ _ x, _, _, _, _, _, hk, ha => by simp only [Node.locStep, ha.get x hk]
  | _, _, .snapshot _ x _, _, _, _, _, _, hk, ha => by simp only [Node.locStep, ha.get x hk]
  | _, _, .batch _ _ _ x _, _, _, _, _, _, hk, ha => by simp only [Node.locStep, ha.get x hk]
  | _, _, .source_interval _ _, _, _, _, _, _, _, _ => rfl
  | _, _, .firstTick _ _ x, _, _, _, _, _, hk, ha => by simp only [Node.locStep, ha.get x hk]
  | _, _, .mapTick x _, _, _, _, _, _, hk, ha => by simp only [Node.locStep, ha.get x hk]
  | _, _, .zipTick a b, _, _, _, _, _, hk, ha => by
    have hk' := Bool.and_eq_true_iff.mp hk
    simp only [Node.locStep, ha.get a hk'.1, ha.get b hk'.2]
  | _, _, .defer_tick _ x, _, _, _, _, _, hk, ha => by simp only [Node.locStep, ha.get x hk]
  | _, _, .allTicks _ x, _, _, _, _, _, hk, ha => by simp only [Node.locStep, ha.get x hk]
  | _, _, .flattenOrdered _ x, _, _, _, _, _, hk, ha => by simp only [Node.locStep, ha.get x hk]
  | _, _, .flattenUnordered _ x, _, _, _, _, _, hk, ha => by simp only [Node.locStep, ha.get x hk]
  | _, _, .tinto_keyed _ _ x, _, _, _, _, _, hk, ha => by simp only [Node.locStep, ha.get x hk]
  | _, _, .tassume_ordering _ x _, _, _, _, _, _, hk, ha => by simp only [Node.locStep, ha.get x hk]
  | _, _, .reduce_watermark _ _ _ _ x wm _ _ _ _, _, _, _, _, _, hk, ha => by
    have hk' := Bool.and_eq_true_iff.mp hk
    simp only [Node.locStep, ha.get x hk'.1, ha.get wm hk'.2]
  | _, _, .kentries _ _ x, _, _, _, _, _, hk, ha => by simp only [Node.locStep, ha.get x hk]
  | _, _, .kmap _ _ _ x _, _, _, _, _, _, hk, ha => by simp only [Node.locStep, ha.get x hk]
  | _, _, .kallTicks _ _ x, _, _, _, _, _, hk, ha => by simp only [Node.locStep, ha.get x hk]
  | _, _, .kfold _ _ _ _ _ _ _ _ _ x, _, _, _, _, _, hk, ha => by simp only [Node.locStep, ha.get x hk]
  | _, _, .ksnapshot _ _ _ x _, _, _, _, _, _, hk, ha => by simp only [Node.locStep, ha.get x hk]
  | _, _, .tfold _ _ _ _ _ _ x, _, _, _, _, _, hk, ha => by simp only [Node.locStep, ha.get x hk]
  | _, _, .latest _ x, _, _, _, _, _, hk, ha => by simp only [Node.locStep, ha.get x hk]
  | _, _, .latest_monotone x, _, _, _, _, _, hk, ha => by simp only [Node.locStep, ha.get x hk]
  | _, _, .time, _, _, _, _, _, _, _ => rfl
  | _, _, .snapshot_bounded _ _, _, _, _, _, _, _, _ => rfl
  | _, _, .tick_scan _ ins _ x _ _, _, _, _, _, _, hk, ha => by simp only [Node.locStep, readShapeLoc_congr m ha _ ins x hk]
  | _, _, .call N args, ora, _, Δ, Δ', s, hk, ha => by
    show Dag.locStep m tk N ora (args.map (fun v => Δ.get v)) s = Dag.locStep m tk N ora (args.map (fun v => Δ'.get v)) s
    rw [Agree.eq_of_all hk (ha.mapSubst args)]
  | _, _, .fixN _ _, _, _, _, _, _, hk, ha => by rw [Agree.eq_of_all hk ha]

/-- **The strict law**: a strict node's step is its release (on any
environment) followed by its push. -/
theorem Node.locStep_strict (m : Member L mem) (tk : Bool) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} →
    (n : Node Γ κs) → Node.isStrict n = true → (ora : Node.Oracle m n) → (Δ Δ' : HList (Kind.locDelta m) Γ) →
    (s : Node.LocState m n) →
    Node.locStep m tk n ora Δ s =
      ((Node.pushStep m n Δ (Node.release m tk n ora Δ' s).1 (Node.release m tk n ora Δ' s).2.2).1,
       (Node.release m tk n ora Δ' s).2.1,
       (Node.pushStep m n Δ (Node.release m tk n ora Δ' s).1 (Node.release m tk n ora Δ' s).2.2).2)

  | _, _, .map .., h, _, _, _, _ => nomatch h
  | _, _, .filterMap .., h, _, _, _, _ => nomatch h
  | _, _, .broadcast_closed .., _, _, _, _, _ => rfl
  | _, _, .demux .., _, _, _, _, _ => rfl
  | _, _, .values .., h, _, _, _, _ => nomatch h
  | _, _, .into_keyed .., h, _, _, _, _ => nomatch h
  | _, _, .kassume_ordering .., h, _, _, _, _ => nomatch h
  | _, _, .kfold_early_stop .., h, _, _, _, _ => nomatch h
  | _, _, .kget_max_key .., h, _, _, _, _ => nomatch h
  | _, _, .weaken_retries .., h, _, _, _, _ => nomatch h
  | _, _, .union .., h, _, _, _, _ => nomatch h
  | _, _, .assume_ordering .., h, _, _, _, _ => nomatch h
  | _, _, .fold .., h, _, _, _, _ => nomatch h
  | _, _, .fold_monotone .., h, _, _, _, _ => nomatch h
  | _, _, .snapshot .., h, _, _, _, _ => nomatch h
  | _, _, .batch .., h, _, _, _, _ => nomatch h
  | _, _, .source_interval .., h, _, _, _, _ => nomatch h
  | _, _, .firstTick .., h, _, _, _, _ => nomatch h
  | _, _, .mapTick .., h, _, _, _, _ => nomatch h
  | _, _, .zipTick .., h, _, _, _, _ => nomatch h
  | _, _, .defer_tick .., _, _, _, _, _ => rfl
  | _, _, .allTicks .., h, _, _, _, _ => nomatch h
  | _, _, .flattenOrdered .., h, _, _, _, _ => nomatch h
  | _, _, .flattenUnordered .., h, _, _, _, _ => nomatch h
  | _, _, .tinto_keyed .., h, _, _, _, _ => nomatch h
  | _, _, .tassume_ordering .., h, _, _, _, _ => nomatch h
  | _, _, .reduce_watermark .., h, _, _, _, _ => nomatch h
  | _, _, .kentries .., h, _, _, _, _ => nomatch h
  | _, _, .kmap .., h, _, _, _, _ => nomatch h
  | _, _, .kallTicks .., h, _, _, _, _ => nomatch h
  | _, _, .kfold .., h, _, _, _, _ => nomatch h
  | _, _, .ksnapshot .., h, _, _, _, _ => nomatch h
  | _, _, .tfold .., h, _, _, _, _ => nomatch h
  | _, _, .latest .., h, _, _, _, _ => nomatch h
  | _, _, .latest_monotone .., h, _, _, _, _ => nomatch h
  | _, _, .time, h, _, _, _, _ => nomatch h
  | _, _, .snapshot_bounded .., h, _, _, _, _ => nomatch h
  | _, _, .tick_scan .., h, _, _, _, _ => nomatch h
  | _, _, .call .., h, _, _, _, _ => nomatch h
  | _, _, .fixN .., h, _, _, _, _ => nomatch h

end Congr

/-! ## The lag analysis and the masks -/

/-- Every node's inputs are known on a fully known environment. -/
theorem Node.inputsKnown_of_all : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) → {mk : Mask Γ} →
    Mask.all mk = true → Node.inputsKnown n mk = true

  | _, _, .map _ _ x _, _, h => Mask.all_get h x
  | _, _, .filterMap _ _ x _, _, h => Mask.all_get h x
  | _, _, .broadcast_closed _ x, _, h => Mask.all_get h x
  | _, _, .demux _ x _, _, h => Mask.all_get h x
  | _, _, .values _ _ x, _, h => Mask.all_get h x
  | _, _, .into_keyed _ _ x, _, h => Mask.all_get h x
  | _, _, .kassume_ordering _ _ x _, _, h => Mask.all_get h x
  | _, _, .kfold_early_stop _ _ _ x _ _, _, h => Mask.all_get h x
  | _, _, .kget_max_key _ _ _ _ x, _, h => Mask.all_get h x
  | _, _, .weaken_retries _ x, _, h => Mask.all_get h x
  | _, _, .union _ a b, _, h => by show (_ && _) = true; rw [Mask.all_get h a, Mask.all_get h b]; rfl
  | _, _, .assume_ordering _ x _, _, h => Mask.all_get h x
  | _, _, .fold _ _ _ _ _ _ x, _, h => Mask.all_get h x
  | _, _, .fold_monotone _ _ _ _ _ _ x, _, h => Mask.all_get h x
  | _, _, .snapshot _ x _, _, h => Mask.all_get h x
  | _, _, .batch _ _ _ x _, _, h => Mask.all_get h x
  | _, _, .source_interval _ _, _, _ => rfl
  | _, _, .firstTick _ _ x, _, h => Mask.all_get h x
  | _, _, .mapTick x _, _, h => Mask.all_get h x
  | _, _, .zipTick a b, _, h => by show (_ && _) = true; rw [Mask.all_get h a, Mask.all_get h b]; rfl
  | _, _, .defer_tick _ x, _, h => Mask.all_get h x
  | _, _, .allTicks _ x, _, h => Mask.all_get h x
  | _, _, .flattenOrdered _ x, _, h => Mask.all_get h x
  | _, _, .flattenUnordered _ x, _, h => Mask.all_get h x
  | _, _, .tinto_keyed _ _ x, _, h => Mask.all_get h x
  | _, _, .tassume_ordering _ x _, _, h => Mask.all_get h x
  | _, _, .reduce_watermark _ _ _ _ x wm _ _ _ _, _, h => by show (_ && _) = true; rw [Mask.all_get h x, Mask.all_get h wm]; rfl
  | _, _, .kentries _ _ x, _, h => Mask.all_get h x
  | _, _, .kmap _ _ _ x _, _, h => Mask.all_get h x
  | _, _, .kallTicks _ _ x, _, h => Mask.all_get h x
  | _, _, .kfold _ _ _ _ _ _ _ _ _ x, _, h => Mask.all_get h x
  | _, _, .ksnapshot _ _ _ x _, _, h => Mask.all_get h x
  | _, _, .tfold _ _ _ _ _ _ x, _, h => Mask.all_get h x
  | _, _, .latest _ x, _, h => Mask.all_get h x
  | _, _, .latest_monotone x, _, h => Mask.all_get h x
  | _, _, .time, _, _ => rfl
  | _, _, .snapshot_bounded _ _, _, _ => rfl
  | _, _, .tick_scan _ ins _ x _ _, _, h => shapeKnown_of_all h ins x
  | _, _, .call _ args, _, h => Mask.all_mapSubst h args
  | _, _, .fixN _ _, _, h => h

theorem shapeLag_pos {Γ : Ctx L mem} {ℓ : L} (mk : Mask Γ) : (sh : TickShape) → (x : VarsOf Γ ℓ sh) →
    shapeKnown mk sh x = false → 1 ≤ shapeLag (maskLags mk) sh x
  | .sing _, v, h => by rw [shapeLag, maskLags.get_eq_one mk v h]
  | .stream _ _ _ _, v, h => by rw [shapeLag, maskLags.get_eq_one mk v h]
  | .ksing _ _ _ _, v, h => by rw [shapeLag, maskLags.get_eq_one mk v h]
  | .kstream _ _ _ _ _ _, v, h => by rw [shapeLag, maskLags.get_eq_one mk v h]
  | .pair a b, (va, vb), h => by
    show 1 ≤ max (shapeLag (maskLags mk) a va) (shapeLag (maskLags mk) b vb)
    have h' : shapeKnown mk a va = false ∨ shapeKnown mk b vb = false := by
      have : (shapeKnown mk a va && shapeKnown mk b vb) = false := h
      cases ha : shapeKnown mk a va <;> cases hb : shapeKnown mk b vb <;> simp_all
    rcases h' with h' | h'
    · exact (shapeLag_pos mk a va h').trans (Nat.le_max_left _ _)
    · exact (shapeLag_pos mk b vb h').trans (Nat.le_max_right _ _)

/-- A plain, non-strict node with an unknown input has every output at a
positive lag. -/
theorem Node.lagOf_unknown : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) → Node.isPlain n = true →
    Node.isStrict n = false → {mk : Mask Γ} → Node.inputsKnown n mk = false →
    Lags.Le (Lags.const κs 1) (Node.lagOf n (maskLags mk))

  | _, _, .map _ _ x _, _, _, mk, h => ⟨by show 1 ≤ (maskLags mk).get x; rw [maskLags.get_eq_one mk x h], trivial⟩
  | _, _, .filterMap _ _ x _, _, _, mk, h => ⟨by show 1 ≤ (maskLags mk).get x; rw [maskLags.get_eq_one mk x h], trivial⟩
  | _, _, .broadcast_closed .., _, hs, _, _ => nomatch hs
  | _, _, .demux .., _, hs, _, _ => nomatch hs
  | _, _, .values _ _ x, _, _, mk, h => ⟨by show 1 ≤ (maskLags mk).get x; rw [maskLags.get_eq_one mk x h], trivial⟩
  | _, _, .into_keyed _ _ x, _, _, mk, h => ⟨by show 1 ≤ (maskLags mk).get x; rw [maskLags.get_eq_one mk x h], trivial⟩
  | _, _, .kassume_ordering _ _ x _, _, _, mk, h => ⟨by show 1 ≤ (maskLags mk).get x; rw [maskLags.get_eq_one mk x h], trivial⟩
  | _, _, .kfold_early_stop _ _ _ x _ _, _, _, mk, h => ⟨by show 1 ≤ (maskLags mk).get x; rw [maskLags.get_eq_one mk x h], trivial⟩
  | _, _, .kget_max_key _ _ _ _ x, _, _, mk, h => ⟨by show 1 ≤ (maskLags mk).get x; rw [maskLags.get_eq_one mk x h], trivial⟩
  | _, _, .weaken_retries _ x, _, _, mk, h => ⟨by show 1 ≤ (maskLags mk).get x; rw [maskLags.get_eq_one mk x h], trivial⟩
  | _, _, .union _ a b, _, _, mk, h => by
    refine ⟨?_, trivial⟩
    show 1 ≤ max ((maskLags mk).get a) ((maskLags mk).get b)
    have h' : mk.get a = false ∨ mk.get b = false := by
      have : (mk.get a && mk.get b) = false := h
      cases ha : mk.get a <;> cases hb : mk.get b <;> simp_all
    rcases h' with h' | h'
    · rw [maskLags.get_eq_one mk a h']; exact Nat.le_max_left _ _
    · rw [maskLags.get_eq_one mk b h']; exact Nat.le_max_right _ _
  | _, _, .assume_ordering _ x _, _, _, mk, h => ⟨by show 1 ≤ (maskLags mk).get x; rw [maskLags.get_eq_one mk x h], trivial⟩
  | _, _, .fold _ _ _ _ _ _ x, _, _, mk, h => ⟨by show 1 ≤ (maskLags mk).get x; rw [maskLags.get_eq_one mk x h], trivial⟩
  | _, _, .fold_monotone _ _ _ _ _ _ x, _, _, mk, h => ⟨by show 1 ≤ (maskLags mk).get x; rw [maskLags.get_eq_one mk x h], trivial⟩
  | _, _, .snapshot _ x _, _, _, mk, h => ⟨by show 1 ≤ (maskLags mk).get x; rw [maskLags.get_eq_one mk x h], trivial⟩
  | _, _, .batch _ _ _ x _, _, _, mk, h => ⟨by show 1 ≤ (maskLags mk).get x; rw [maskLags.get_eq_one mk x h], trivial⟩
  | _, _, .source_interval _ _, _, _, _, h => nomatch h
  | _, _, .firstTick _ _ x, _, _, mk, h => ⟨by show 1 ≤ (maskLags mk).get x; rw [maskLags.get_eq_one mk x h], trivial⟩
  | _, _, .mapTick x _, _, _, mk, h => ⟨by show 1 ≤ (maskLags mk).get x; rw [maskLags.get_eq_one mk x h], trivial⟩
  | _, _, .zipTick a b, _, _, mk, h => by
    refine ⟨?_, trivial⟩
    show 1 ≤ max ((maskLags mk).get a) ((maskLags mk).get b)
    have h' : mk.get a = false ∨ mk.get b = false := by
      have : (mk.get a && mk.get b) = false := h
      cases ha : mk.get a <;> cases hb : mk.get b <;> simp_all
    rcases h' with h' | h'
    · rw [maskLags.get_eq_one mk a h']; exact Nat.le_max_left _ _
    · rw [maskLags.get_eq_one mk b h']; exact Nat.le_max_right _ _
  | _, _, .defer_tick .., _, hs, _, _ => nomatch hs
  | _, _, .allTicks _ x, _, _, mk, h => ⟨by show 1 ≤ (maskLags mk).get x; rw [maskLags.get_eq_one mk x h], trivial⟩
  | _, _, .flattenOrdered _ x, _, _, mk, h => ⟨by show 1 ≤ (maskLags mk).get x; rw [maskLags.get_eq_one mk x h], trivial⟩
  | _, _, .flattenUnordered _ x, _, _, mk, h => ⟨by show 1 ≤ (maskLags mk).get x; rw [maskLags.get_eq_one mk x h], trivial⟩
  | _, _, .tinto_keyed _ _ x, _, _, mk, h => ⟨by show 1 ≤ (maskLags mk).get x; rw [maskLags.get_eq_one mk x h], trivial⟩
  | _, _, .tassume_ordering _ x _, _, _, mk, h => ⟨by show 1 ≤ (maskLags mk).get x; rw [maskLags.get_eq_one mk x h], trivial⟩
  | _, _, .reduce_watermark _ _ _ _ x wm _ _ _ _, _, _, mk, h => by
    refine ⟨?_, trivial⟩
    show 1 ≤ max ((maskLags mk).get x) ((maskLags mk).get wm)
    have h' : mk.get x = false ∨ mk.get wm = false := by
      have : (mk.get x && mk.get wm) = false := h
      cases ha : mk.get x <;> cases hb : mk.get wm <;> simp_all
    rcases h' with h' | h'
    · rw [maskLags.get_eq_one mk x h']; exact Nat.le_max_left _ _
    · rw [maskLags.get_eq_one mk wm h']; exact Nat.le_max_right _ _
  | _, _, .kentries _ _ x, _, _, mk, h => ⟨by show 1 ≤ (maskLags mk).get x; rw [maskLags.get_eq_one mk x h], trivial⟩
  | _, _, .kmap _ _ _ x _, _, _, mk, h => ⟨by show 1 ≤ (maskLags mk).get x; rw [maskLags.get_eq_one mk x h], trivial⟩
  | _, _, .kallTicks _ _ x, _, _, mk, h => ⟨by show 1 ≤ (maskLags mk).get x; rw [maskLags.get_eq_one mk x h], trivial⟩
  | _, _, .kfold _ _ _ _ _ _ _ _ _ x, _, _, mk, h => ⟨by show 1 ≤ (maskLags mk).get x; rw [maskLags.get_eq_one mk x h], trivial⟩
  | _, _, .ksnapshot _ _ _ x _, _, _, mk, h => ⟨by show 1 ≤ (maskLags mk).get x; rw [maskLags.get_eq_one mk x h], trivial⟩
  | _, _, .tfold _ _ _ _ _ _ x, _, _, mk, h => ⟨by show 1 ≤ (maskLags mk).get x; rw [maskLags.get_eq_one mk x h], trivial⟩
  | _, _, .latest _ x, _, _, mk, h => ⟨by show 1 ≤ (maskLags mk).get x; rw [maskLags.get_eq_one mk x h], trivial⟩
  | _, _, .latest_monotone x, _, _, mk, h => ⟨by show 1 ≤ (maskLags mk).get x; rw [maskLags.get_eq_one mk x h], trivial⟩
  | _, _, .time, _, _, _, h => nomatch h
  | _, _, .snapshot_bounded _ _, _, _, _, h => nomatch h
  | _, _, .tick_scan _ ins _ x _ _, _, _, mk, h => Lags.Le.const (shapeLag_pos mk ins x h)
  | _, _, .call .., hp, _, _, _ => nomatch hp
  | _, _, .fixN .., hp, _, _, _ => nomatch hp


/-! ## The process's step respects the lag analysis (D95)

`Dag.locStep_agree0`: the process's shadow of `Dag.step_agree0` — two
increment environments agreeing at the lag-`0` wires give outputs agreeing
at the lag-`0` outputs. A plain node reads known wires only
(`Node.locStep_congr`), a strict node's outputs are its cells' alone
(`Node.locStep_strict`), any other node with an unknown input has every
output above lag `0` (`Node.lagOf_unknown`); a knot block iterates its body. -/

/-- The mask of the lag-`0` wires. -/
def Lags.mask : {κs : List (Kind L mem)} → Lags κs → Mask κs
  | [], _ => HList.nil
  | _ :: _, l => HList.cons (l.1 == 0) (Lags.mask l.2)

theorem DAgree0.toAgree {f : Kind L mem → Type} : {κs : List (Kind L mem)} → {l : Lags κs} → {x y : HList f κs} →
    DAgree0 l x y → Agree (Lags.mask l) x y
  | [], _, _, _, _ => trivial
  | _ :: _, l, _, _, h => ⟨fun h0 => h.1 (by have h0' : (l.1 == 0) = true := h0; simpa using h0'), DAgree0.toAgree h.2⟩

theorem DAgree0.of_pos : {κs : List (Kind L mem)} → {l : Lags κs} → Lags.Le (Lags.const κs 1) l →
    (x y : HList (Kind.locDelta m) κs) → DAgree0 l x y
  | [], _, _, _, _ => trivial
  | _ :: _, l, h, x, y => ⟨fun h0 => by
      have h1 : 1 ≤ l.1 := h.1
      rw [h0] at h1
      exact absurd h1 (Nat.not_succ_le_zero 0), DAgree0.of_pos h.2 x.2 y.2⟩

theorem maskLags.mask_le : {κs : List (Kind L mem)} → (l : Lags κs) → Lags.Le (maskLags (Lags.mask l)) l
  | [], _ => trivial
  | _ :: _, l => ⟨by
      show (if (l.1 == 0) = true then 0 else 1) ≤ l.1
      by_cases h : l.1 = 0
      · rw [if_pos (by simpa using h)]; exact Nat.zero_le _
      · rw [if_neg (by simpa using h)]; exact Nat.pos_of_ne_zero h, maskLags.mask_le l.2⟩

section LocAgree
variable [DecidableEq L]

/-- A plain node's step respects the lag analysis. -/
theorem Node.locStep_agree0_plain (m : Member L mem) (tk : Bool) {Γ : Ctx L mem} {κs : List (Kind L mem)} (n : Node Γ κs)
    (hp : Node.isPlain n = true) (ora : Node.Oracle m n) (l : Lags Γ) (Δ Δ' : HList (Kind.locDelta m) Γ)
    (s : Node.LocState m n) (h : DAgree0 l Δ Δ') :
    DAgree0 (Node.lagOf n l) (Node.locStep m tk n ora Δ s).2.1 (Node.locStep m tk n ora Δ' s).2.1 := by
  cases hk : Node.inputsKnown n (Lags.mask l)
  · cases hs : Node.isStrict n
    · exact DAgree0.of_pos ((Node.lagOf_unknown n hp hs hk).trans (Node.lagOf_mono n (maskLags.mask_le l))) _ _
    · rw [Node.locStep_strict m tk n hs ora Δ Δ' s, Node.locStep_strict m tk n hs ora Δ' Δ' s]
      exact DAgree0.refl _ _
  · rw [Node.locStep_congr m tk n ora s hk (DAgree0.toAgree h)]
    exact DAgree0.refl _ _

mutual

theorem Node.locStep_agree0 (m : Member L mem) (tk : Bool) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    (ora : Node.Oracle m n) → (l : Lags Γ) → (Δ Δ' : HList (Kind.locDelta m) Γ) → (s : Node.LocState m n) → DAgree0 l Δ Δ' →
    DAgree0 (Node.lagOf n l) (Node.locStep m tk n ora Δ s).2.1 (Node.locStep m tk n ora Δ' s).2.1
  | _, _, .call N args, ora, l, Δ, Δ', s, h => Dag.locStep_agree0 m tk N ora _ _ _ s (h.mapSubst args)
  | _, _, .fixN ks body, ora, l, Δ, Δ', s, h => by
    show DAgree0 ((Lags.const _ (Lags.maxAll l)).append (HList.drop (Dag.lagOf body ((Lags.const _ (Lags.maxAll l)).append l))))
      (Dag.locStep m tk body ora
        ((knotIter (fun K => (Dag.locStep m tk body ora (K.append Δ) s).2.1.take) ks.length (knotFillLoc m ks)).append Δ) s).2.1
      (Dag.locStep m tk body ora
        ((knotIter (fun K => (Dag.locStep m tk body ora (K.append Δ') s).2.1.take) ks.length (knotFillLoc m ks)).append Δ') s).2.1
    rcases Nat.eq_zero_or_pos (Lags.maxAll l) with h0 | hpos
    · have hΔ : Δ = Δ' := DAgree0.eq_of_zero (Lags.le_const_of_maxAll l (Nat.le_of_eq h0)) h
      subst hΔ
      exact DAgree0.refl _ _
    · generalize knotIter (fun K => (Dag.locStep m tk body ora (K.append Δ) s).2.1.take) ks.length (knotFillLoc m ks) = KΔ
      generalize knotIter (fun K => (Dag.locStep m tk body ora (K.append Δ') s).2.1.take) ks.length (knotFillLoc m ks) = KΔ'
      have ih := Dag.locStep_agree0 m tk body ora ((Lags.const (ks.map KnotKind.toKind) 1).append l)
        (KΔ.append Δ) (KΔ'.append Δ') s
        (DAgree0.append (DAgree0.const_of _ (fun h0 => absurd h0 Nat.one_ne_zero)) h)
      have ih' := ih.mono_lag (Dag.lagOf_mono body (Lags.Le.append (Lags.Le.const hpos) (Lags.Le.refl l)))
      rw [← HList.take_append_drop (κs := ks.map KnotKind.toKind) (Dag.locStep m tk body ora (KΔ.append Δ) s).2.1,
        ← HList.take_append_drop (κs := ks.map KnotKind.toKind) (Dag.locStep m tk body ora (KΔ'.append Δ') s).2.1]
      exact DAgree0.append (DAgree0.const_of _ (fun h0 => absurd h0 (Nat.pos_iff_ne_zero.mp hpos))) (DAgree0.drop ih')
  | _, _, .map .., ora, l, Δ, Δ', s, h => Node.locStep_agree0_plain m tk _ rfl ora l Δ Δ' s h
  | _, _, .filterMap .., ora, l, Δ, Δ', s, h => Node.locStep_agree0_plain m tk _ rfl ora l Δ Δ' s h
  | _, _, .broadcast_closed .., ora, l, Δ, Δ', s, h => Node.locStep_agree0_plain m tk _ rfl ora l Δ Δ' s h
  | _, _, .demux .., ora, l, Δ, Δ', s, h => Node.locStep_agree0_plain m tk _ rfl ora l Δ Δ' s h
  | _, _, .values .., ora, l, Δ, Δ', s, h => Node.locStep_agree0_plain m tk _ rfl ora l Δ Δ' s h
  | _, _, .into_keyed .., ora, l, Δ, Δ', s, h => Node.locStep_agree0_plain m tk _ rfl ora l Δ Δ' s h
  | _, _, .kassume_ordering .., ora, l, Δ, Δ', s, h => Node.locStep_agree0_plain m tk _ rfl ora l Δ Δ' s h
  | _, _, .kfold_early_stop .., ora, l, Δ, Δ', s, h => Node.locStep_agree0_plain m tk _ rfl ora l Δ Δ' s h
  | _, _, .kget_max_key .., ora, l, Δ, Δ', s, h => Node.locStep_agree0_plain m tk _ rfl ora l Δ Δ' s h
  | _, _, .weaken_retries .., ora, l, Δ, Δ', s, h => Node.locStep_agree0_plain m tk _ rfl ora l Δ Δ' s h
  | _, _, .union .., ora, l, Δ, Δ', s, h => Node.locStep_agree0_plain m tk _ rfl ora l Δ Δ' s h
  | _, _, .assume_ordering .., ora, l, Δ, Δ', s, h => Node.locStep_agree0_plain m tk _ rfl ora l Δ Δ' s h
  | _, _, .fold .., ora, l, Δ, Δ', s, h => Node.locStep_agree0_plain m tk _ rfl ora l Δ Δ' s h
  | _, _, .fold_monotone .., ora, l, Δ, Δ', s, h => Node.locStep_agree0_plain m tk _ rfl ora l Δ Δ' s h
  | _, _, .snapshot .., ora, l, Δ, Δ', s, h => Node.locStep_agree0_plain m tk _ rfl ora l Δ Δ' s h
  | _, _, .batch .., ora, l, Δ, Δ', s, h => Node.locStep_agree0_plain m tk _ rfl ora l Δ Δ' s h
  | _, _, .source_interval .., ora, l, Δ, Δ', s, h => Node.locStep_agree0_plain m tk _ rfl ora l Δ Δ' s h
  | _, _, .firstTick .., ora, l, Δ, Δ', s, h => Node.locStep_agree0_plain m tk _ rfl ora l Δ Δ' s h
  | _, _, .mapTick .., ora, l, Δ, Δ', s, h => Node.locStep_agree0_plain m tk _ rfl ora l Δ Δ' s h
  | _, _, .zipTick .., ora, l, Δ, Δ', s, h => Node.locStep_agree0_plain m tk _ rfl ora l Δ Δ' s h
  | _, _, .defer_tick .., ora, l, Δ, Δ', s, h => Node.locStep_agree0_plain m tk _ rfl ora l Δ Δ' s h
  | _, _, .allTicks .., ora, l, Δ, Δ', s, h => Node.locStep_agree0_plain m tk _ rfl ora l Δ Δ' s h
  | _, _, .flattenOrdered .., ora, l, Δ, Δ', s, h => Node.locStep_agree0_plain m tk _ rfl ora l Δ Δ' s h
  | _, _, .flattenUnordered .., ora, l, Δ, Δ', s, h => Node.locStep_agree0_plain m tk _ rfl ora l Δ Δ' s h
  | _, _, .tinto_keyed .., ora, l, Δ, Δ', s, h => Node.locStep_agree0_plain m tk _ rfl ora l Δ Δ' s h
  | _, _, .tassume_ordering .., ora, l, Δ, Δ', s, h => Node.locStep_agree0_plain m tk _ rfl ora l Δ Δ' s h
  | _, _, .reduce_watermark .., ora, l, Δ, Δ', s, h => Node.locStep_agree0_plain m tk _ rfl ora l Δ Δ' s h
  | _, _, .kentries .., ora, l, Δ, Δ', s, h => Node.locStep_agree0_plain m tk _ rfl ora l Δ Δ' s h
  | _, _, .kmap .., ora, l, Δ, Δ', s, h => Node.locStep_agree0_plain m tk _ rfl ora l Δ Δ' s h
  | _, _, .kallTicks .., ora, l, Δ, Δ', s, h => Node.locStep_agree0_plain m tk _ rfl ora l Δ Δ' s h
  | _, _, .kfold .., ora, l, Δ, Δ', s, h => Node.locStep_agree0_plain m tk _ rfl ora l Δ Δ' s h
  | _, _, .ksnapshot .., ora, l, Δ, Δ', s, h => Node.locStep_agree0_plain m tk _ rfl ora l Δ Δ' s h
  | _, _, .tfold .., ora, l, Δ, Δ', s, h => Node.locStep_agree0_plain m tk _ rfl ora l Δ Δ' s h
  | _, _, .latest .., ora, l, Δ, Δ', s, h => Node.locStep_agree0_plain m tk _ rfl ora l Δ Δ' s h
  | _, _, .latest_monotone .., ora, l, Δ, Δ', s, h => Node.locStep_agree0_plain m tk _ rfl ora l Δ Δ' s h
  | _, _, .time, ora, l, Δ, Δ', s, h => Node.locStep_agree0_plain m tk _ rfl ora l Δ Δ' s h
  | _, _, .snapshot_bounded .., ora, l, Δ, Δ', s, h => Node.locStep_agree0_plain m tk _ rfl ora l Δ Δ' s h
  | _, _, .tick_scan .., ora, l, Δ, Δ', s, h => Node.locStep_agree0_plain m tk _ rfl ora l Δ Δ' s h

theorem Dag.locStep_agree0 (m : Member L mem) (tk : Bool) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    (ora : Dag.Oracle m d) → (l : Lags Γ) → (Δ Δ' : HList (Kind.locDelta m) Γ) → (s : Dag.LocState m d) → DAgree0 l Δ Δ' →
    DAgree0 (Dag.lagOf d l) (Dag.locStep m tk d ora Δ s).2.1 (Dag.locStep m tk d ora Δ' s).2.1
  | _, _, .ret xs, _, l, Δ, Δ', s, h => h.mapSubst xs
  | _, _, .letN n rest, ora, l, Δ, Δ', s, h => by
    have ih := Dag.locStep_agree0 m tk rest ora.2 _ _ _ s.2 ((Node.locStep_agree0 m tk n ora.1 l Δ Δ' s.1 h).append h)
    simp only [Dag.locStep, memoEnv_eq]
    exact ih

end

/-- **The knot increments the process's `fixN` reaches are a fixpoint of the
body's step** (cascade-valid body): the Jacobi iteration tracks the lag
analysis (`Dag.step_knotIter_agree0`'s shadow), which reaches "every knot
known" in as many rounds as there are knots (`iter_zero`). -/
theorem Dag.locStep_knotIter_agree0 (m : Member L mem) (tk : Bool) {Γ : Ctx L mem} (ks : List (KnotKind L mem))
    {outs : List (Kind L mem)} (body : Dag (ks.map KnotKind.toKind ++ Γ) (ks.map KnotKind.toKind ++ outs))
    (ora : Dag.Oracle m body) (Δ : HList (Kind.locDelta m) Γ) (s : Dag.LocState m body) :
    ∀ (i : Nat) (l : Lags (ks.map KnotKind.toKind)) (X Y : HList (Kind.locDelta m) (ks.map KnotKind.toKind)),
      DAgree0 l X Y →
      DAgree0 (knotIter (Dag.knotLag ks body (Lags.const Γ 0)) i l)
        (knotIter (fun K => (Dag.locStep m tk body ora (K.append Δ) s).2.1.take) i X)
        (knotIter (fun K => (Dag.locStep m tk body ora (K.append Δ) s).2.1.take) i Y)
  | 0, _, _, _, h => h
  | i + 1, l, X, Y, h =>
    Dag.locStep_knotIter_agree0 m tk ks body ora Δ s i _ _ _
      ((Dag.locStep_agree0 m tk body ora (l.append (Lags.const Γ 0)) (X.append Δ) (Y.append Δ) s
        (DAgree0.append h (DAgree0.const_of _ (fun _ => rfl)))).take)

theorem Dag.locStep_knotIter_fix (m : Member L mem) (tk : Bool) {Γ : Ctx L mem} (ks : List (KnotKind L mem))
    {outs : List (Kind L mem)} (body : Dag (ks.map KnotKind.toKind ++ Γ) (ks.map KnotKind.toKind ++ outs))
    (hv : cascadeValid (ks.map KnotKind.toKind) (Dag.knotLag ks body (Lags.const Γ 0)) = true)
    (ora : Dag.Oracle m body) (Δ : HList (Kind.locDelta m) Γ) (s : Dag.LocState m body) :
    (Dag.locStep m tk body ora
      ((knotIter (fun K => (Dag.locStep m tk body ora (K.append Δ) s).2.1.take) ks.length (knotFillLoc m ks)).append Δ) s).2.1.take
      = knotIter (fun K => (Dag.locStep m tk body ora (K.append Δ) s).2.1.take) ks.length (knotFillLoc m ks) := by
  have h := Dag.locStep_knotIter_agree0 m tk ks body ora Δ s ks.length (Lags.const _ 1) (knotFillLoc m ks)
    ((Dag.locStep m tk body ora ((knotFillLoc m ks).append Δ) s).2.1.take)
    (DAgree0.const_of _ (fun h0 => absurd h0 Nat.one_ne_zero))
  have hz := iter_zero (ks.map KnotKind.toKind) (Dag.knotLag ks body (Lags.const Γ 0))
    (Dag.knotLag_laws ks body (Lags.const Γ 0)).self hv
  rw [List.length_map] at hz
  rw [hz] at h
  have e := DAgree0.eq_of_zero (Lags.Le.refl _) h
  exact ((knotIter_succ' _ ks.length (knotFillLoc m ks)).symm.trans e.symm)

end LocAgree

/-! ## Consistency with the reference step -/

/-- Whether a plain node's phase is `done`. -/
def NodePart.isDone {O S : Type} : NodePart O S → Prop
  | .done _ _ => True
  | _ => False

/-! ### The knot blocks' knownness -/

/-- The lag bound of a knot block from its body's, after any body sweep, when
an external is unknown (the knots then sit at or below the externals' maximum). -/
theorem fixN_known_partial {Γ : Ctx L mem} (ks : List (KnotKind L mem)) {outs : List (Kind L mem)}
    (body : Dag (ks.map KnotKind.toKind ++ Γ) (ks.map KnotKind.toKind ++ outs))
    {mk : Mask Γ} (hall : Mask.all mk = false) (b : Mask (ks.map KnotKind.toKind)) (rmask : Mask (ks.map KnotKind.toKind ++ outs))
    (h : Lags.Le (maskLags rmask) (Dag.lagOf body ((maskLags b).append (maskLags mk)))) :
    Lags.Le (maskLags rmask)
      ((Lags.const (ks.map KnotKind.toKind) (Lags.maxAll (maskLags mk))).append
        (HList.drop (κs := ks.map KnotKind.toKind)
          (Dag.lagOf body ((Lags.const (ks.map KnotKind.toKind) (Lags.maxAll (maskLags mk))).append (maskLags mk))))) := by
  apply Lags.Le.of_take_drop
  · rw [HList.take_append, ← maskLags.take]
    exact (maskLags.le_one _).trans (Lags.Le.const (maskLags.one_le_maxAll hall))
  · rw [HList.drop_append]
    exact (Lags.Le.drop h).trans (Lags.Le.drop (Dag.lagOf_mono body
      (Lags.Le.append ((maskLags.le_one b).trans (Lags.Le.const (maskLags.one_le_maxAll hall))) (Lags.Le.refl _))))

section Cons
variable [DecidableEq L]

/-! ### The knot loop's invariants

Generic over the body sweep `sw` (`knotLoop`, `Project/Sweep.lean`). -/

/-- **The knot loop ends with every knot known** on a valid body whose knot
outputs' knownness follows the lag functional (`hsw`): a sweep that binds
nothing new has a post-fixpoint of the lag functional below `1` — which is
`0` (`postfix_zero`) — and every other sweep makes progress along the Jacobi
iteration (`iter_zero`), so the fuel never runs out first. -/
theorem knotLoop_all {K outs : List (Kind L mem)} {S P Snd : Type}
    (sw : HList (Kind.locDelta m) K → Mask K → S → P → (S × P) × (HList (Kind.locDelta m) (K ++ outs) × Mask (K ++ outs) × Snd))
    (Φ₀ : Lags K → Lags K) (hl : LagLaws Φ₀ Φ₀) (hv : cascadeValid K Φ₀ = true)
    (hsw : ∀ Kv b s bp, Lags.Le (maskLags (HList.take (κs := K) (sw Kv b s bp).2.2.1)) (Φ₀ (maskLags b)))
    (Q : (S × (Mask K × HList (Kind.locDelta m) K) × P) × (HList (Kind.locDelta m) (K ++ outs) × Mask (K ++ outs) × Snd) → Prop)
    (hQ : ∀ Kv b s bp, Mask.all b = true → Q (knotFinalize (sw Kv b s bp) b Kv)) :
    ∀ (fuel : Nat) (b : Mask K) (Kv : HList (Kind.locDelta m) K) (s : S) (bp : P), fuel ≤ K.length →
      Lags.Le (maskLags b) (knotIter Φ₀ (K.length - fuel) (Lags.const K 1)) → Q (knotLoop sw fuel b Kv s bp)
  | 0, b, Kv, s, bp, _, hb => by
    rw [Nat.sub_zero, iter_zero K Φ₀ hl hv] at hb
    exact hQ Kv b s bp (maskLags.all_of_le hb)
  | fuel + 1, b, Kv, s, bp, hfuel, hb => by
    unfold knotLoop
    dsimp only
    split
    · rename_i heq
      have heq' := Mask.beq_iff.mp heq
      -- nothing new: `maskLags b ≤ Φ₀ (maskLags b)`, a post-fixpoint below `1`
      have hpost : Lags.Le (maskLags b) (Φ₀ (maskLags b)) := by
        have h1 := maskLags.or_le_right b (HList.take (κs := K) (sw Kv b s bp).2.2.1)
        rw [heq'] at h1
        exact h1.trans (hsw Kv b s bp)
      have hz := postfix_zero K Φ₀ hl hv _ hpost (maskLags.le_one b)
      exact hQ Kv b s bp (maskLags.all_of_le (by rw [hz]; exact Lags.Le.refl _))
    · refine knotLoop_all sw Φ₀ hl hv hsw Q hQ fuel _ _ _ _ (Nat.le_of_succ_le hfuel) ?_
      have h1 := (maskLags.or_le_right b (HList.take (κs := K) (sw Kv b s bp).2.2.1)).trans (hsw Kv b s bp)
      refine h1.trans ?_
      have e : K.length - fuel = (K.length - (fuel + 1)) + 1 := by omega
      rw [e, knotIter_succ']
      exact hl.mono hb

/-- Any exit of the knot loop, when the bound on a body sweep's knownness does
not depend on the knots. -/
theorem knotLoop_bound {K outs : List (Kind L mem)} {S P Snd : Type}
    (sw : HList (Kind.locDelta m) K → Mask K → S → P → (S × P) × (HList (Kind.locDelta m) (K ++ outs) × Mask (K ++ outs) × Snd))
    (bound : Lags (K ++ outs)) (hsw : ∀ Kv b s bp, Lags.Le (maskLags (sw Kv b s bp).2.2.1) bound)
    (fuel : Nat) (b : Mask K) (Kv : HList (Kind.locDelta m) K) (s : S) (bp : P) :
    Lags.Le (maskLags (knotLoop sw fuel b Kv s bp).2.2.1) bound := by
  obtain ⟨b₁, Kv₁, s₁, bp₁, e⟩ := knotLoop_final sw fuel b Kv s bp
  rw [e]
  exact hsw _ _ _ _

/-- **Soundness of the knot loop** relative to a reference `ref` of the knots
that is a fixpoint of the reference outputs (`hfix`): the bound knots agree
with `ref`, the body's state is consistent with the reference on every
environment of the unbound knots, and the known outputs are the reference's. -/
theorem knotLoop_sound {K outs : List (Kind L mem)} {S P Snd : Type}
    (sw : HList (Kind.locDelta m) K → Mask K → S → P → (S × P) × (HList (Kind.locDelta m) (K ++ outs) × Mask (K ++ outs) × Snd))
    (ref : HList (Kind.locDelta m) K) (Out : HList (Kind.locDelta m) K → HList (Kind.locDelta m) (K ++ outs))
    (Cons : HList (Kind.locDelta m) K → S → P → Prop)
    (hfix : HList.take (κs := K) (Out ref) = ref)
    (hsw : ∀ X Kv b s bp, Cons X s bp → Agree b Kv X →
      Cons X (sw Kv b s bp).1.1 (sw Kv b s bp).1.2 ∧ Agree (sw Kv b s bp).2.2.1 (sw Kv b s bp).2.1 (Out X)) :
    ∀ (fuel : Nat) (b : Mask K) (Kv : HList (Kind.locDelta m) K) (s : S) (bp : P),
      Agree b Kv ref → (∀ X, Agree b X Kv → Cons X s bp) →
      (Agree (knotLoop sw fuel b Kv s bp).1.2.1.1 (knotLoop sw fuel b Kv s bp).1.2.1.2 ref ∧
        ∀ X, Agree (knotLoop sw fuel b Kv s bp).1.2.1.1 X (knotLoop sw fuel b Kv s bp).1.2.1.2 →
          Cons X (knotLoop sw fuel b Kv s bp).1.1 (knotLoop sw fuel b Kv s bp).1.2.2) ∧
      Agree (knotLoop sw fuel b Kv s bp).2.2.1 (knotLoop sw fuel b Kv s bp).2.1 (Out ref)
  | fuel, b, Kv, s, bp, hKv, hC => by
    -- one round from `(b, Kv)` preserves the invariant
    have round : ∀ (b : Mask K) (Kv : HList (Kind.locDelta m) K) (s : S) (bp : P),
        Agree b Kv ref → (∀ X, Agree b X Kv → Cons X s bp) →
        (Agree (Mask.or b (HList.take (κs := K) (sw Kv b s bp).2.2.1)) (Mask.select b Kv (HList.take (κs := K) (sw Kv b s bp).2.1)) ref ∧
          ∀ X, Agree (Mask.or b (HList.take (κs := K) (sw Kv b s bp).2.2.1)) X (Mask.select b Kv (HList.take (κs := K) (sw Kv b s bp).2.1)) →
            Cons X (sw Kv b s bp).1.1 (sw Kv b s bp).1.2) ∧
        Agree (sw Kv b s bp).2.2.1 (sw Kv b s bp).2.1 (Out ref) := by
      intro b Kv s bp hKv hC
      have hr := hsw ref Kv b s bp (hC ref hKv.symm) hKv
      refine ⟨⟨?_, ?_⟩, hr.2⟩
      · refine Agree.or_select hKv ?_
        have h := hr.2.take
        rw [hfix] at h
        exact h
      · intro X hX
        have hXKv : Agree b X Kv := (hX.of_or_left).trans (Agree.select_left b Kv _)
        exact (hsw X Kv b s bp (hC X hXKv) hXKv.symm).1
    cases fuel with
    | zero => exact round b Kv s bp hKv hC
    | succ fuel =>
      unfold knotLoop
      dsimp only
      split
      · exact round b Kv s bp hKv hC
      · have hr := round b Kv s bp hKv hC
        exact knotLoop_sound sw ref Out Cons hfix hsw fuel _ _ _ _ hr.1.1 hr.1.2


/-- A plain node's phase against the reference step from `s₀` on `Δf`: a
`done` node holds the step, a `released` node its release half, a `todo`
node its initial cells. -/
def Node.PhaseCons (m : Member L mem) (tk : Bool) {Γ : Ctx L mem} {κs : List (Kind L mem)} (n : Node Γ κs)
    (tm : Node.Timing m n) (Δf : HList (Kind.locDelta m) Γ) (s₀ s : Node.LocState m n) :
    NodePart (HList (Kind.locDelta m) κs) (Node.Sends m n) → Prop
  | .todo => s = s₀
  | .released o snd => Node.isStrict n = true ∧ ∃ Δ', (s, o, snd) = Node.release m tk n (Node.oracleOf m n tm s₀) Δ' s₀
  | .done o snd => (s, o, snd) = Node.locStep m tk n (Node.oracleOf m n tm s₀) Δf s₀

/-- The reference knot increments of a knot block (`Node.locStep`'s `fixN`):
the body's step iterated on the knot increments from empty ones. -/
def knotRef (m : Member L mem) (tk : Bool) {Γ : Ctx L mem} (ks : List (KnotKind L mem)) {outs : List (Kind L mem)}
    (body : Dag (ks.map KnotKind.toKind ++ Γ) (ks.map KnotKind.toKind ++ outs)) (tm : Dag.Timing m body)
    (Δf : HList (Kind.locDelta m) Γ) (s₀ : Dag.LocState m body) : HList (Kind.locDelta m) (ks.map KnotKind.toKind) :=
  knotIter (fun K => (Dag.locStep m tk body (Dag.oracleOf m body tm s₀) (K.append Δf) s₀).2.1.take) ks.length (knotFillLoc m ks)

mutual

/-- A node's sweep state is consistent with the reference step from `s₀` on `Δf`. -/
def Node.Cons (m : Member L mem) (tk : Bool) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    Node.Timing m n → HList (Kind.locDelta m) Γ → Node.LocState m n → Node.LocState m n → Node.Part m n → Prop
  | _, _, .call N args, tm, Δf, s₀, s, part => Dag.Cons m tk N tm (args.map (fun v => Δf.get v)) s₀ s part.2
  | _, _, .fixN ks body, tm, Δf, s₀, s, part =>
    -- the knots bound so far are the reference's; the body is consistent with the
    -- reference on every environment of the knots not yet bound
    Agree part.2.1.1 part.2.1.2 (knotRef m tk ks body tm Δf s₀) ∧
    ∀ X : HList (Kind.locDelta m) (ks.map KnotKind.toKind), Agree part.2.1.1 X part.2.1.2 →
      Dag.Cons m tk body tm (X.append Δf) s₀ s part.2.2
  | _, _, n, tm, Δf, s₀, s, part => Node.PhaseCons m tk n tm Δf s₀ s part.1

/-- A graph's sweep state is consistent with the reference step from `s₀` on `Δf`. -/
def Dag.Cons (m : Member L mem) (tk : Bool) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    Dag.Timing m d → HList (Kind.locDelta m) Γ → Dag.LocState m d → Dag.LocState m d → Dag.Part m d → Prop
  | _, _, .ret _, _, _, _, _, _ => True
  | _, _, .letN n rest, tm, Δf, s₀, s, part =>
    Node.Cons m tk n tm.1 Δf s₀.1 s.1 part.1 ∧
    Dag.Cons m tk rest tm.2 ((Node.locStep m tk n (Node.oracleOf m n tm.1 s₀.1) Δf s₀.1).2.1.append Δf) s₀.2 s.2 part.2

end

mutual

/-- Every node is `done`. -/
def Node.allDone (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    Node.Part m n → Prop
  | _, _, .call N _, part => Dag.allDone m N part.2
  | _, _, .fixN _ body, part => Mask.all part.2.1.1 = true ∧ Dag.allDone m body part.2.2
  | _, _, _, part => part.1.isDone

def Dag.allDone (m : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    Dag.Part m d → Prop
  | _, _, .ret _, _ => True
  | _, _, .letN n rest, part => Node.allDone m n part.1 ∧ Dag.allDone m rest part.2

end

/-! ### The plain nodes -/

theorem Node.sweepPlain_sound (m : Member L mem) (tk : Bool) {Γ : Ctx L mem} {κs : List (Kind L mem)} (n : Node Γ κs)
    (tm : Node.Timing m n) (Δf Δ : HList (Kind.locDelta m) Γ) (mk : Mask Γ) (s₀ s : Node.LocState m n)
    (part : Node.Part m n) (hc : Node.PhaseCons m tk n tm Δf s₀ s part.1) (ha : Agree mk Δ Δf) :
    Node.PhaseCons m tk n tm Δf s₀ (Node.sweepPlain m tk n tm Δ mk s part).1.1 (Node.sweepPlain m tk n tm Δ mk s part).1.2.1 ∧
    Agree (Node.sweepPlain m tk n tm Δ mk s part).2.2.1 (Node.sweepPlain m tk n tm Δ mk s part).2.1
      (Node.locStep m tk n (Node.oracleOf m n tm s₀) Δf s₀).2.1 ∧
    (Node.sweepPlain m tk n tm Δ mk s part).2.2.2 =
      (Node.sweepPlain m tk n tm Δ mk s part).1.2.1.sends (Node.sendsNil m n) := by
  obtain ⟨ph, sub⟩ := part
  cases ph with
  | done o snd =>
    have hc' : (s, o, snd) = Node.locStep m tk n (Node.oracleOf m n tm s₀) Δf s₀ := hc
    simp only [Node.sweepPlain]
    exact ⟨hc, Agree.of_eq (congrArg (fun p => p.2.1) hc'), rfl⟩
  | released o snd =>
    obtain ⟨hs, Δ', hc'⟩ := hc
    have hout : o = (Node.locStep m tk n (Node.oracleOf m n tm s₀) Δf s₀).2.1 := by
      rw [Node.locStep_strict m tk n hs (Node.oracleOf m n tm s₀) Δf Δ' s₀, ← hc']
    cases hk : Node.inputsKnown n mk
    · simp only [Node.sweepPlain, hk, Bool.false_eq_true, ↓reduceIte]
      exact ⟨⟨hs, Δ', hc'⟩, Agree.of_eq hout, rfl⟩
    · simp only [Node.sweepPlain, hk, ↓reduceIte]
      refine ⟨?_, Agree.of_eq hout, rfl⟩
      show (_, o, _) = Node.locStep m tk n (Node.oracleOf m n tm s₀) Δf s₀
      rw [Node.locStep_strict m tk n hs (Node.oracleOf m n tm s₀) Δf Δ' s₀, ← hc', Node.pushStep_congr m n _ _ hk ha]
  | todo =>
    have hc' : s = s₀ := hc
    subst hc'
    cases hk : Node.inputsKnown n mk
    · cases hs : Node.isStrict n
      · simp only [Node.sweepPlain, hk, hs, Bool.false_eq_true, ↓reduceIte]
        exact ⟨rfl, Agree.const_false _ _, rfl⟩
      · simp only [Node.sweepPlain, hk, hs, Bool.false_eq_true, ↓reduceIte]
        refine ⟨⟨hs, Δ, rfl⟩, Agree.of_eq ?_, rfl⟩
        rw [Node.locStep_strict m tk n hs (Node.oracleOf m n tm s) Δf Δ s]
    · simp only [Node.sweepPlain, hk, ↓reduceIte]
      rw [Node.locStep_congr m tk n (Node.oracleOf m n tm s) s hk ha]
      exact ⟨rfl, Agree.refl _ _, rfl⟩

theorem Node.sweepPlain_known (m : Member L mem) (tk : Bool) {Γ : Ctx L mem} {κs : List (Kind L mem)} (n : Node Γ κs)
    (hp : Node.isPlain n = true) (tm : Node.Timing m n) (Δ : HList (Kind.locDelta m) Γ) (mk : Mask Γ)
    (s : Node.LocState m n) (part : Node.Part m n) :
    Lags.Le (maskLags (Node.sweepPlain m tk n tm Δ mk s part).2.2.1) (Node.lagOf n (maskLags mk)) := by
  obtain ⟨ph, sub⟩ := part
  cases ph with
  | done o snd =>
    simp only [Node.sweepPlain, maskLags.const_true]
    exact Lags.zero_le _
  | released o snd =>
    cases hk : Node.inputsKnown n mk <;> simp only [Node.sweepPlain, hk, Bool.false_eq_true, ↓reduceIte, maskLags.const_true] <;>
      exact Lags.zero_le _
  | todo =>
    cases hk : Node.inputsKnown n mk
    · cases hs : Node.isStrict n
      · simp only [Node.sweepPlain, hk, hs, Bool.false_eq_true, ↓reduceIte, maskLags.const_false]
        exact Node.lagOf_unknown n hp hs hk
      · simp only [Node.sweepPlain, hk, hs, Bool.false_eq_true, ↓reduceIte, maskLags.const_true]
        exact Lags.zero_le _
    · simp only [Node.sweepPlain, hk, ↓reduceIte, maskLags.const_true]
      exact Lags.zero_le _

theorem Node.sweepPlain_complete (m : Member L mem) (tk : Bool) {Γ : Ctx L mem} {κs : List (Kind L mem)} (n : Node Γ κs)
    (tm : Node.Timing m n) (Δ : HList (Kind.locDelta m) Γ) (mk : Mask Γ) (s : Node.LocState m n) (part : Node.Part m n)
    (hk : Mask.all mk = true) :
    Mask.all (Node.sweepPlain m tk n tm Δ mk s part).2.2.1 = true ∧ (Node.sweepPlain m tk n tm Δ mk s part).1.2.1.isDone := by
  have hk' := Node.inputsKnown_of_all n hk
  obtain ⟨ph, sub⟩ := part
  cases ph <;> simp only [Node.sweepPlain, hk', ↓reduceIte] <;> exact ⟨Mask.all_const_true _, trivial⟩

theorem Node.phase_final (m : Member L mem) (tk : Bool) {Γ : Ctx L mem} {κs : List (Kind L mem)} (n : Node Γ κs)
    (tm : Node.Timing m n) (Δf : HList (Kind.locDelta m) Γ) (s₀ s : Node.LocState m n)
    (ph : NodePart (HList (Kind.locDelta m) κs) (Node.Sends m n)) (hd : ph.isDone)
    (hc : Node.PhaseCons m tk n tm Δf s₀ s ph) :
    s = (Node.locStep m tk n (Node.oracleOf m n tm s₀) Δf s₀).1 ∧ ph.sends (Node.sendsNil m n) = (Node.locStep m tk n (Node.oracleOf m n tm s₀) Δf s₀).2.2 := by
  cases ph with
  | done o snd =>
    have hc' : (s, o, snd) = Node.locStep m tk n (Node.oracleOf m n tm s₀) Δf s₀ := hc
    exact ⟨congrArg Prod.fst hc', congrArg (fun p => p.2.2) hc'⟩
  | todo => exact hd.elim
  | released _ _ => exact hd.elim

/-! ### The sweep invariants -/

mutual

/-- Fresh parts are consistent. -/
theorem Node.Cons_init (m : Member L mem) (tk : Bool) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    (tm : Node.Timing m n) → (Δf : HList (Kind.locDelta m) Γ) → (s₀ : Node.LocState m n) →
    Node.Cons m tk n tm Δf s₀ s₀ (.todo, Node.Sub.init m n)
  | _, _, .call N _, tm, _, s₀ => Dag.Cons_init m tk N tm _ s₀
  | _, _, .fixN _ body, tm, Δf, s₀ => ⟨Agree.const_false _ _, fun X _ => Dag.Cons_init m tk body tm (X.append Δf) s₀⟩

  | _, _, .map .., _, _, _ => rfl
  | _, _, .filterMap .., _, _, _ => rfl
  | _, _, .broadcast_closed .., _, _, _ => rfl
  | _, _, .demux .., _, _, _ => rfl
  | _, _, .values .., _, _, _ => rfl
  | _, _, .into_keyed .., _, _, _ => rfl
  | _, _, .kassume_ordering .., _, _, _ => rfl
  | _, _, .kfold_early_stop .., _, _, _ => rfl
  | _, _, .kget_max_key .., _, _, _ => rfl
  | _, _, .weaken_retries .., _, _, _ => rfl
  | _, _, .union .., _, _, _ => rfl
  | _, _, .assume_ordering .., _, _, _ => rfl
  | _, _, .fold .., _, _, _ => rfl
  | _, _, .fold_monotone .., _, _, _ => rfl
  | _, _, .snapshot .., _, _, _ => rfl
  | _, _, .batch .., _, _, _ => rfl
  | _, _, .source_interval .., _, _, _ => rfl
  | _, _, .firstTick .., _, _, _ => rfl
  | _, _, .mapTick .., _, _, _ => rfl
  | _, _, .zipTick .., _, _, _ => rfl
  | _, _, .defer_tick .., _, _, _ => rfl
  | _, _, .allTicks .., _, _, _ => rfl
  | _, _, .flattenOrdered .., _, _, _ => rfl
  | _, _, .flattenUnordered .., _, _, _ => rfl
  | _, _, .tinto_keyed .., _, _, _ => rfl
  | _, _, .tassume_ordering .., _, _, _ => rfl
  | _, _, .reduce_watermark .., _, _, _ => rfl
  | _, _, .kentries .., _, _, _ => rfl
  | _, _, .kmap .., _, _, _ => rfl
  | _, _, .kallTicks .., _, _, _ => rfl
  | _, _, .kfold .., _, _, _ => rfl
  | _, _, .ksnapshot .., _, _, _ => rfl
  | _, _, .tfold .., _, _, _ => rfl
  | _, _, .latest .., _, _, _ => rfl
  | _, _, .latest_monotone .., _, _, _ => rfl
  | _, _, .time, _, _, _ => rfl
  | _, _, .snapshot_bounded .., _, _, _ => rfl
  | _, _, .tick_scan .., _, _, _ => rfl

theorem Dag.Cons_init (m : Member L mem) (tk : Bool) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    (tm : Dag.Timing m d) → (Δf : HList (Kind.locDelta m) Γ) → (s₀ : Dag.LocState m d) →
    Dag.Cons m tk d tm Δf s₀ s₀ (Dag.Part.init m d)
  | _, _, .ret _, _, _, _ => trivial
  | _, _, .letN n rest, tm, Δf, s₀ => ⟨Node.Cons_init m tk n tm.1 Δf s₀.1, Dag.Cons_init m tk rest tm.2 _ s₀.2⟩

end

mutual

/-- **Soundness**: a sweep on an environment agreeing with the reference on
the known wires preserves consistency; its known outputs are the reference's;
its sends are its parts'. -/
theorem Node.sweep_sound (m : Member L mem) (tk : Bool) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    Node.valid n = true → (tm : Node.Timing m n) → (Δf Δ : HList (Kind.locDelta m) Γ) → (mk : Mask Γ) → (s₀ s : Node.LocState m n) →
    (part : Node.Part m n) → Node.Cons m tk n tm Δf s₀ s part → Agree mk Δ Δf →
    Node.Cons m tk n tm Δf s₀ (Node.sweep m tk n tm Δ mk s part).1.1 (Node.sweep m tk n tm Δ mk s part).1.2 ∧
    Agree (Node.sweep m tk n tm Δ mk s part).2.2.1 (Node.sweep m tk n tm Δ mk s part).2.1
      (Node.locStep m tk n (Node.oracleOf m n tm s₀) Δf s₀).2.1 ∧
    (Node.sweep m tk n tm Δ mk s part).2.2.2 = Node.partSends m n (Node.sweep m tk n tm Δ mk s part).1.2
  | _, _, .call N args, hv, tm, Δf, Δ, mk, s₀, s, part, hc, ha =>
    Dag.sweep_sound m tk N hv tm _ _ _ s₀ s part.2 hc (ha.mapSubst args)
  | _, _, .fixN ks body, hv, tm, Δf, Δ, mk, s₀, s, part, hc, ha => by
    obtain ⟨hvk, hvb⟩ := Node.valid_fixN ks body hv
    change NodePart _ _ × ((Mask (ks.map KnotKind.toKind) × HList (Kind.locDelta m) (ks.map KnotKind.toKind)) × Dag.Part m body) at part
    refine ⟨?_, ?_, Node.sweep_sends m tk (.fixN ks body) tm Δ mk s part⟩ <;>
    · have h := knotLoop_sound (fun Kv b s bp => Dag.sweep m tk body tm (Kv.append Δ) (b.append mk) s bp)
        (knotRef m tk ks body tm Δf s₀) (fun X => (Dag.locStep m tk body (Dag.oracleOf m body tm s₀) (X.append Δf) s₀).2.1)
        (fun X s bp => Dag.Cons m tk body tm (X.append Δf) s₀ s bp)
        (Dag.locStep_knotIter_fix m tk ks body hvk (Dag.oracleOf m body tm s₀) Δf s₀)
        (fun X Kv b s bp hC hKX =>
          let r := Dag.sweep_sound m tk body hvb tm (X.append Δf) (Kv.append Δ) (b.append mk) s₀ s bp hC (Agree.append hKX ha)
          ⟨r.1, r.2.1⟩)
        ks.length part.2.1.1 part.2.1.2 s part.2.2 hc.1 hc.2
      first | exact h.1 | exact h.2

  | _, _, .map .., _, tm, Δf, Δ, mk, s₀, s, part, hc, ha => Node.sweepPlain_sound m tk _ tm Δf Δ mk s₀ s part hc ha
  | _, _, .filterMap .., _, tm, Δf, Δ, mk, s₀, s, part, hc, ha => Node.sweepPlain_sound m tk _ tm Δf Δ mk s₀ s part hc ha
  | _, _, .broadcast_closed .., _, tm, Δf, Δ, mk, s₀, s, part, hc, ha => Node.sweepPlain_sound m tk _ tm Δf Δ mk s₀ s part hc ha
  | _, _, .demux .., _, tm, Δf, Δ, mk, s₀, s, part, hc, ha => Node.sweepPlain_sound m tk _ tm Δf Δ mk s₀ s part hc ha
  | _, _, .values .., _, tm, Δf, Δ, mk, s₀, s, part, hc, ha => Node.sweepPlain_sound m tk _ tm Δf Δ mk s₀ s part hc ha
  | _, _, .into_keyed .., _, tm, Δf, Δ, mk, s₀, s, part, hc, ha => Node.sweepPlain_sound m tk _ tm Δf Δ mk s₀ s part hc ha
  | _, _, .kassume_ordering .., _, tm, Δf, Δ, mk, s₀, s, part, hc, ha => Node.sweepPlain_sound m tk _ tm Δf Δ mk s₀ s part hc ha
  | _, _, .kfold_early_stop .., _, tm, Δf, Δ, mk, s₀, s, part, hc, ha => Node.sweepPlain_sound m tk _ tm Δf Δ mk s₀ s part hc ha
  | _, _, .kget_max_key .., _, tm, Δf, Δ, mk, s₀, s, part, hc, ha => Node.sweepPlain_sound m tk _ tm Δf Δ mk s₀ s part hc ha
  | _, _, .weaken_retries .., _, tm, Δf, Δ, mk, s₀, s, part, hc, ha => Node.sweepPlain_sound m tk _ tm Δf Δ mk s₀ s part hc ha
  | _, _, .union .., _, tm, Δf, Δ, mk, s₀, s, part, hc, ha => Node.sweepPlain_sound m tk _ tm Δf Δ mk s₀ s part hc ha
  | _, _, .assume_ordering .., _, tm, Δf, Δ, mk, s₀, s, part, hc, ha => Node.sweepPlain_sound m tk _ tm Δf Δ mk s₀ s part hc ha
  | _, _, .fold .., _, tm, Δf, Δ, mk, s₀, s, part, hc, ha => Node.sweepPlain_sound m tk _ tm Δf Δ mk s₀ s part hc ha
  | _, _, .fold_monotone .., _, tm, Δf, Δ, mk, s₀, s, part, hc, ha => Node.sweepPlain_sound m tk _ tm Δf Δ mk s₀ s part hc ha
  | _, _, .snapshot .., _, tm, Δf, Δ, mk, s₀, s, part, hc, ha => Node.sweepPlain_sound m tk _ tm Δf Δ mk s₀ s part hc ha
  | _, _, .batch .., _, tm, Δf, Δ, mk, s₀, s, part, hc, ha => Node.sweepPlain_sound m tk _ tm Δf Δ mk s₀ s part hc ha
  | _, _, .source_interval .., _, tm, Δf, Δ, mk, s₀, s, part, hc, ha => Node.sweepPlain_sound m tk _ tm Δf Δ mk s₀ s part hc ha
  | _, _, .firstTick .., _, tm, Δf, Δ, mk, s₀, s, part, hc, ha => Node.sweepPlain_sound m tk _ tm Δf Δ mk s₀ s part hc ha
  | _, _, .mapTick .., _, tm, Δf, Δ, mk, s₀, s, part, hc, ha => Node.sweepPlain_sound m tk _ tm Δf Δ mk s₀ s part hc ha
  | _, _, .zipTick .., _, tm, Δf, Δ, mk, s₀, s, part, hc, ha => Node.sweepPlain_sound m tk _ tm Δf Δ mk s₀ s part hc ha
  | _, _, .defer_tick .., _, tm, Δf, Δ, mk, s₀, s, part, hc, ha => Node.sweepPlain_sound m tk _ tm Δf Δ mk s₀ s part hc ha
  | _, _, .allTicks .., _, tm, Δf, Δ, mk, s₀, s, part, hc, ha => Node.sweepPlain_sound m tk _ tm Δf Δ mk s₀ s part hc ha
  | _, _, .flattenOrdered .., _, tm, Δf, Δ, mk, s₀, s, part, hc, ha => Node.sweepPlain_sound m tk _ tm Δf Δ mk s₀ s part hc ha
  | _, _, .flattenUnordered .., _, tm, Δf, Δ, mk, s₀, s, part, hc, ha => Node.sweepPlain_sound m tk _ tm Δf Δ mk s₀ s part hc ha
  | _, _, .tinto_keyed .., _, tm, Δf, Δ, mk, s₀, s, part, hc, ha => Node.sweepPlain_sound m tk _ tm Δf Δ mk s₀ s part hc ha
  | _, _, .tassume_ordering .., _, tm, Δf, Δ, mk, s₀, s, part, hc, ha => Node.sweepPlain_sound m tk _ tm Δf Δ mk s₀ s part hc ha
  | _, _, .reduce_watermark .., _, tm, Δf, Δ, mk, s₀, s, part, hc, ha => Node.sweepPlain_sound m tk _ tm Δf Δ mk s₀ s part hc ha
  | _, _, .kentries .., _, tm, Δf, Δ, mk, s₀, s, part, hc, ha => Node.sweepPlain_sound m tk _ tm Δf Δ mk s₀ s part hc ha
  | _, _, .kmap .., _, tm, Δf, Δ, mk, s₀, s, part, hc, ha => Node.sweepPlain_sound m tk _ tm Δf Δ mk s₀ s part hc ha
  | _, _, .kallTicks .., _, tm, Δf, Δ, mk, s₀, s, part, hc, ha => Node.sweepPlain_sound m tk _ tm Δf Δ mk s₀ s part hc ha
  | _, _, .kfold .., _, tm, Δf, Δ, mk, s₀, s, part, hc, ha => Node.sweepPlain_sound m tk _ tm Δf Δ mk s₀ s part hc ha
  | _, _, .ksnapshot .., _, tm, Δf, Δ, mk, s₀, s, part, hc, ha => Node.sweepPlain_sound m tk _ tm Δf Δ mk s₀ s part hc ha
  | _, _, .tfold .., _, tm, Δf, Δ, mk, s₀, s, part, hc, ha => Node.sweepPlain_sound m tk _ tm Δf Δ mk s₀ s part hc ha
  | _, _, .latest .., _, tm, Δf, Δ, mk, s₀, s, part, hc, ha => Node.sweepPlain_sound m tk _ tm Δf Δ mk s₀ s part hc ha
  | _, _, .latest_monotone .., _, tm, Δf, Δ, mk, s₀, s, part, hc, ha => Node.sweepPlain_sound m tk _ tm Δf Δ mk s₀ s part hc ha
  | _, _, .time, _, tm, Δf, Δ, mk, s₀, s, part, hc, ha => Node.sweepPlain_sound m tk _ tm Δf Δ mk s₀ s part hc ha
  | _, _, .snapshot_bounded .., _, tm, Δf, Δ, mk, s₀, s, part, hc, ha => Node.sweepPlain_sound m tk _ tm Δf Δ mk s₀ s part hc ha
  | _, _, .tick_scan .., _, tm, Δf, Δ, mk, s₀, s, part, hc, ha => Node.sweepPlain_sound m tk _ tm Δf Δ mk s₀ s part hc ha

theorem Dag.sweep_sound (m : Member L mem) (tk : Bool) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    Dag.valid d = true → (tm : Dag.Timing m d) → (Δf Δ : HList (Kind.locDelta m) Γ) → (mk : Mask Γ) → (s₀ s : Dag.LocState m d) →
    (part : Dag.Part m d) → Dag.Cons m tk d tm Δf s₀ s part → Agree mk Δ Δf →
    Dag.Cons m tk d tm Δf s₀ (Dag.sweep m tk d tm Δ mk s part).1.1 (Dag.sweep m tk d tm Δ mk s part).1.2 ∧
    Agree (Dag.sweep m tk d tm Δ mk s part).2.2.1 (Dag.sweep m tk d tm Δ mk s part).2.1
      (Dag.locStep m tk d (Dag.oracleOf m d tm s₀) Δf s₀).2.1 ∧
    (Dag.sweep m tk d tm Δ mk s part).2.2.2 = Dag.partSends m d (Dag.sweep m tk d tm Δ mk s part).1.2
  | _, _, .ret xs, _, _, _, _, _, _, _, _, _, ha => ⟨trivial, ha.mapSubst xs, rfl⟩
  | _, _, .letN n rest, hv, tm, Δf, Δ, mk, s₀, s, part, hc, ha => by
    obtain ⟨hcn, hcr⟩ := hc
    obtain ⟨hvn, hvr⟩ := Dag.valid_letN n rest hv
    have ih1 := Node.sweep_sound m tk n hvn tm.1 Δf Δ mk s₀.1 s.1 part.1 hcn ha
    have ih2 := Dag.sweep_sound m tk rest hvr tm.2 ((Node.locStep m tk n (Node.oracleOf m n tm.1 s₀.1) Δf s₀.1).2.1.append Δf)
      ((Node.sweep m tk n tm.1 Δ mk s.1 part.1).2.1.append Δ) ((Node.sweep m tk n tm.1 Δ mk s.1 part.1).2.2.1.append mk)
      s₀.2 s.2 part.2 hcr (Agree.append ih1.2.1 ha)
    dsimp only [Dag.sweep, Dag.locStep, Dag.oracleOf, Dag.Cons, Dag.partSends]
    rw [Node.memoState_eq, memoEnv_eq, memoEnv_eq]
    exact ⟨⟨ih1.1, ih2.1⟩, ih2.2.1, Prod.ext ih1.2.2 ih2.2.2⟩

end

mutual

/-- **Knownness**: a sweep knows at least what the lag analysis predicts
(known wires at lag `0`, unknown at `1`). -/
theorem Node.sweep_known (m : Member L mem) (tk : Bool) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    Node.valid n = true → (tm : Node.Timing m n) → (Δ : HList (Kind.locDelta m) Γ) → (mk : Mask Γ) →
    (s : Node.LocState m n) → (part : Node.Part m n) →
    Lags.Le (maskLags (Node.sweep m tk n tm Δ mk s part).2.2.1) (Node.lagOf n (maskLags mk))
  | _, _, .call N args, hv, tm, Δ, mk, s, part => by
    have ih := Dag.sweep_known m tk N hv tm (args.map (fun v => Δ.get v)) (args.map (fun v => mk.get v)) s part.2
    rw [maskLags.mapSubst] at ih
    exact ih
  | _, _, .fixN ks body, hv, tm, Δ, mk, s, part => by
    obtain ⟨hvk, hvb⟩ := Node.valid_fixN ks body hv
    change NodePart _ _ × ((Mask (ks.map KnotKind.toKind) × HList (Kind.locDelta m) (ks.map KnotKind.toKind)) × Dag.Part m body) at part
    show Lags.Le (maskLags (knotLoop (fun Kv b s bp => Dag.sweep m tk body tm (Kv.append Δ) (b.append mk) s bp)
        ks.length part.2.1.1 part.2.1.2 s part.2.2).2.2.1)
      ((Lags.const _ (Lags.maxAll (maskLags mk))).append
        (HList.drop (κs := ks.map KnotKind.toKind) (Dag.lagOf body ((Lags.const _ (Lags.maxAll (maskLags mk))).append (maskLags mk)))))
    cases hall : Mask.all mk
    · -- an external unknown: any exit of the loop satisfies the block's bound
      exact knotLoop_bound _ _ (fun Kv b s bp => by
        have h := Dag.sweep_known m tk body hvb tm (Kv.append Δ) (b.append mk) s bp
        rw [maskLags.append] at h
        exact fixN_known_partial ks body hall b _ h) ks.length part.2.1.1 part.2.1.2 s part.2.2
    · -- every external known: the loop ends with every knot known, and that sweep knows everything
      refine knotLoop_all _ (Dag.knotLag ks body (Lags.const _ 0)) (Dag.knotLag_laws ks body (Lags.const _ 0)).self hvk
        (fun Kv b s bp => ?_) (fun r => Lags.Le (maskLags r.2.2.1) _) (fun Kv b s bp hb => ?_)
        ks.length part.2.1.1 part.2.1.2 s part.2.2 (by rw [List.length_map])
        (by rw [List.length_map, Nat.sub_self]; exact maskLags.le_one _)
      · have h := Dag.sweep_known m tk body hvb tm (Kv.append Δ) (b.append mk) s bp
        rw [maskLags.append, maskLags.of_all hall] at h
        have h' := Lags.Le.take (κs := ks.map KnotKind.toKind) h
        rw [← maskLags.take] at h'
        exact h'
      · have h := Dag.sweep_known m tk body hvb tm (Kv.append Δ) (b.append mk) s bp
        rw [maskLags.append, maskLags.of_all hb, maskLags.of_all hall, Lags.const_append, Dag.lagOf_zero] at h
        exact h.trans (Lags.zero_le _)
  | _, _, .map .., _, tm, Δ, mk, s, part => Node.sweepPlain_known m tk _ rfl tm Δ mk s part
  | _, _, .filterMap .., _, tm, Δ, mk, s, part => Node.sweepPlain_known m tk _ rfl tm Δ mk s part
  | _, _, .broadcast_closed .., _, tm, Δ, mk, s, part => Node.sweepPlain_known m tk _ rfl tm Δ mk s part
  | _, _, .demux .., _, tm, Δ, mk, s, part => Node.sweepPlain_known m tk _ rfl tm Δ mk s part
  | _, _, .values .., _, tm, Δ, mk, s, part => Node.sweepPlain_known m tk _ rfl tm Δ mk s part
  | _, _, .into_keyed .., _, tm, Δ, mk, s, part => Node.sweepPlain_known m tk _ rfl tm Δ mk s part
  | _, _, .kassume_ordering .., _, tm, Δ, mk, s, part => Node.sweepPlain_known m tk _ rfl tm Δ mk s part
  | _, _, .kfold_early_stop .., _, tm, Δ, mk, s, part => Node.sweepPlain_known m tk _ rfl tm Δ mk s part
  | _, _, .kget_max_key .., _, tm, Δ, mk, s, part => Node.sweepPlain_known m tk _ rfl tm Δ mk s part
  | _, _, .weaken_retries .., _, tm, Δ, mk, s, part => Node.sweepPlain_known m tk _ rfl tm Δ mk s part
  | _, _, .union .., _, tm, Δ, mk, s, part => Node.sweepPlain_known m tk _ rfl tm Δ mk s part
  | _, _, .assume_ordering .., _, tm, Δ, mk, s, part => Node.sweepPlain_known m tk _ rfl tm Δ mk s part
  | _, _, .fold .., _, tm, Δ, mk, s, part => Node.sweepPlain_known m tk _ rfl tm Δ mk s part
  | _, _, .fold_monotone .., _, tm, Δ, mk, s, part => Node.sweepPlain_known m tk _ rfl tm Δ mk s part
  | _, _, .snapshot .., _, tm, Δ, mk, s, part => Node.sweepPlain_known m tk _ rfl tm Δ mk s part
  | _, _, .batch .., _, tm, Δ, mk, s, part => Node.sweepPlain_known m tk _ rfl tm Δ mk s part
  | _, _, .source_interval .., _, tm, Δ, mk, s, part => Node.sweepPlain_known m tk _ rfl tm Δ mk s part
  | _, _, .firstTick .., _, tm, Δ, mk, s, part => Node.sweepPlain_known m tk _ rfl tm Δ mk s part
  | _, _, .mapTick .., _, tm, Δ, mk, s, part => Node.sweepPlain_known m tk _ rfl tm Δ mk s part
  | _, _, .zipTick .., _, tm, Δ, mk, s, part => Node.sweepPlain_known m tk _ rfl tm Δ mk s part
  | _, _, .defer_tick .., _, tm, Δ, mk, s, part => Node.sweepPlain_known m tk _ rfl tm Δ mk s part
  | _, _, .allTicks .., _, tm, Δ, mk, s, part => Node.sweepPlain_known m tk _ rfl tm Δ mk s part
  | _, _, .flattenOrdered .., _, tm, Δ, mk, s, part => Node.sweepPlain_known m tk _ rfl tm Δ mk s part
  | _, _, .flattenUnordered .., _, tm, Δ, mk, s, part => Node.sweepPlain_known m tk _ rfl tm Δ mk s part
  | _, _, .tinto_keyed .., _, tm, Δ, mk, s, part => Node.sweepPlain_known m tk _ rfl tm Δ mk s part
  | _, _, .tassume_ordering .., _, tm, Δ, mk, s, part => Node.sweepPlain_known m tk _ rfl tm Δ mk s part
  | _, _, .reduce_watermark .., _, tm, Δ, mk, s, part => Node.sweepPlain_known m tk _ rfl tm Δ mk s part
  | _, _, .kentries .., _, tm, Δ, mk, s, part => Node.sweepPlain_known m tk _ rfl tm Δ mk s part
  | _, _, .kmap .., _, tm, Δ, mk, s, part => Node.sweepPlain_known m tk _ rfl tm Δ mk s part
  | _, _, .kallTicks .., _, tm, Δ, mk, s, part => Node.sweepPlain_known m tk _ rfl tm Δ mk s part
  | _, _, .kfold .., _, tm, Δ, mk, s, part => Node.sweepPlain_known m tk _ rfl tm Δ mk s part
  | _, _, .ksnapshot .., _, tm, Δ, mk, s, part => Node.sweepPlain_known m tk _ rfl tm Δ mk s part
  | _, _, .tfold .., _, tm, Δ, mk, s, part => Node.sweepPlain_known m tk _ rfl tm Δ mk s part
  | _, _, .latest .., _, tm, Δ, mk, s, part => Node.sweepPlain_known m tk _ rfl tm Δ mk s part
  | _, _, .latest_monotone .., _, tm, Δ, mk, s, part => Node.sweepPlain_known m tk _ rfl tm Δ mk s part
  | _, _, .time, _, tm, Δ, mk, s, part => Node.sweepPlain_known m tk _ rfl tm Δ mk s part
  | _, _, .snapshot_bounded .., _, tm, Δ, mk, s, part => Node.sweepPlain_known m tk _ rfl tm Δ mk s part
  | _, _, .tick_scan .., _, tm, Δ, mk, s, part => Node.sweepPlain_known m tk _ rfl tm Δ mk s part

theorem Dag.sweep_known (m : Member L mem) (tk : Bool) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    Dag.valid d = true → (tm : Dag.Timing m d) → (Δ : HList (Kind.locDelta m) Γ) → (mk : Mask Γ) →
    (s : Dag.LocState m d) → (part : Dag.Part m d) →
    Lags.Le (maskLags (Dag.sweep m tk d tm Δ mk s part).2.2.1) (Dag.lagOf d (maskLags mk))
  | _, _, .ret xs, _, _, _, mk, _, _ => by
    show Lags.Le (maskLags (xs.map (fun v => mk.get v))) _
    rw [maskLags.mapSubst]
    exact Lags.Le.refl _
  | _, _, .letN n rest, hv, tm, Δ, mk, s, part => by
    obtain ⟨hvn, hvr⟩ := Dag.valid_letN n rest hv
    have ih1 := Node.sweep_known m tk n hvn tm.1 Δ mk s.1 part.1
    have ih2 := Dag.sweep_known m tk rest hvr tm.2 ((memoEnv m (Node.sweep m tk n tm.1 Δ mk s.1 part.1).2.1).append Δ)
      ((Node.sweep m tk n tm.1 Δ mk s.1 part.1).2.2.1.append mk) s.2 part.2
    rw [maskLags.append] at ih2
    exact ih2.trans (Dag.lagOf_mono rest (Lags.Le.append ih1 (Lags.Le.refl _)))

end

mutual

/-- **Completeness**: on a fully known environment a sweep of a valid graph
leaves everything `done` and known. -/
theorem Node.sweep_complete (m : Member L mem) (tk : Bool) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    Node.valid n = true → (tm : Node.Timing m n) → (Δ : HList (Kind.locDelta m) Γ) → (mk : Mask Γ) →
    (s : Node.LocState m n) → (part : Node.Part m n) → Mask.all mk = true →
    Mask.all (Node.sweep m tk n tm Δ mk s part).2.2.1 = true ∧ Node.allDone m n (Node.sweep m tk n tm Δ mk s part).1.2
  | _, _, .call N args, hv, tm, Δ, mk, s, part, hk =>
    Dag.sweep_complete m tk N hv tm _ _ s part.2 (Mask.all_mapSubst hk args)
  | _, _, .fixN ks body, hv, tm, Δ, mk, s, part, hk => by
    obtain ⟨hvk, hvb⟩ := Node.valid_fixN ks body hv
    change NodePart _ _ × ((Mask (ks.map KnotKind.toKind) × HList (Kind.locDelta m) (ks.map KnotKind.toKind)) × Dag.Part m body) at part
    show Mask.all (knotLoop (fun Kv b s bp => Dag.sweep m tk body tm (Kv.append Δ) (b.append mk) s bp)
        ks.length part.2.1.1 part.2.1.2 s part.2.2).2.2.1 = true ∧
      (Mask.all (knotLoop (fun Kv b s bp => Dag.sweep m tk body tm (Kv.append Δ) (b.append mk) s bp)
        ks.length part.2.1.1 part.2.1.2 s part.2.2).1.2.1.1 = true ∧
       Dag.allDone m body (knotLoop (fun Kv b s bp => Dag.sweep m tk body tm (Kv.append Δ) (b.append mk) s bp)
        ks.length part.2.1.1 part.2.1.2 s part.2.2).1.2.2)
    refine knotLoop_all _ (Dag.knotLag ks body (Lags.const _ 0)) (Dag.knotLag_laws ks body (Lags.const _ 0)).self hvk
      (fun Kv b s bp => ?_) (fun r => Mask.all r.2.2.1 = true ∧ (Mask.all r.1.2.1.1 = true ∧ Dag.allDone m body r.1.2.2))
      (fun Kv b s bp hb => ?_) ks.length part.2.1.1 part.2.1.2 s part.2.2 (by rw [List.length_map])
      (by rw [List.length_map, Nat.sub_self]; exact maskLags.le_one _)
    · have h := Dag.sweep_known m tk body hvb tm (Kv.append Δ) (b.append mk) s bp
      rw [maskLags.append, maskLags.of_all hk] at h
      have h' := Lags.Le.take (κs := ks.map KnotKind.toKind) h
      rw [← maskLags.take] at h'
      exact h'
    · have h := Dag.sweep_complete m tk body hvb tm (Kv.append Δ) (b.append mk) s bp (Mask.all_append hb hk)
      exact ⟨h.1, Mask.all_or_left _ hb, h.2⟩
  | _, _, .map .., _, tm, Δ, mk, s, part, hk => Node.sweepPlain_complete m tk _ tm Δ mk s part hk
  | _, _, .filterMap .., _, tm, Δ, mk, s, part, hk => Node.sweepPlain_complete m tk _ tm Δ mk s part hk
  | _, _, .broadcast_closed .., _, tm, Δ, mk, s, part, hk => Node.sweepPlain_complete m tk _ tm Δ mk s part hk
  | _, _, .demux .., _, tm, Δ, mk, s, part, hk => Node.sweepPlain_complete m tk _ tm Δ mk s part hk
  | _, _, .values .., _, tm, Δ, mk, s, part, hk => Node.sweepPlain_complete m tk _ tm Δ mk s part hk
  | _, _, .into_keyed .., _, tm, Δ, mk, s, part, hk => Node.sweepPlain_complete m tk _ tm Δ mk s part hk
  | _, _, .kassume_ordering .., _, tm, Δ, mk, s, part, hk => Node.sweepPlain_complete m tk _ tm Δ mk s part hk
  | _, _, .kfold_early_stop .., _, tm, Δ, mk, s, part, hk => Node.sweepPlain_complete m tk _ tm Δ mk s part hk
  | _, _, .kget_max_key .., _, tm, Δ, mk, s, part, hk => Node.sweepPlain_complete m tk _ tm Δ mk s part hk
  | _, _, .weaken_retries .., _, tm, Δ, mk, s, part, hk => Node.sweepPlain_complete m tk _ tm Δ mk s part hk
  | _, _, .union .., _, tm, Δ, mk, s, part, hk => Node.sweepPlain_complete m tk _ tm Δ mk s part hk
  | _, _, .assume_ordering .., _, tm, Δ, mk, s, part, hk => Node.sweepPlain_complete m tk _ tm Δ mk s part hk
  | _, _, .fold .., _, tm, Δ, mk, s, part, hk => Node.sweepPlain_complete m tk _ tm Δ mk s part hk
  | _, _, .fold_monotone .., _, tm, Δ, mk, s, part, hk => Node.sweepPlain_complete m tk _ tm Δ mk s part hk
  | _, _, .snapshot .., _, tm, Δ, mk, s, part, hk => Node.sweepPlain_complete m tk _ tm Δ mk s part hk
  | _, _, .batch .., _, tm, Δ, mk, s, part, hk => Node.sweepPlain_complete m tk _ tm Δ mk s part hk
  | _, _, .source_interval .., _, tm, Δ, mk, s, part, hk => Node.sweepPlain_complete m tk _ tm Δ mk s part hk
  | _, _, .firstTick .., _, tm, Δ, mk, s, part, hk => Node.sweepPlain_complete m tk _ tm Δ mk s part hk
  | _, _, .mapTick .., _, tm, Δ, mk, s, part, hk => Node.sweepPlain_complete m tk _ tm Δ mk s part hk
  | _, _, .zipTick .., _, tm, Δ, mk, s, part, hk => Node.sweepPlain_complete m tk _ tm Δ mk s part hk
  | _, _, .defer_tick .., _, tm, Δ, mk, s, part, hk => Node.sweepPlain_complete m tk _ tm Δ mk s part hk
  | _, _, .allTicks .., _, tm, Δ, mk, s, part, hk => Node.sweepPlain_complete m tk _ tm Δ mk s part hk
  | _, _, .flattenOrdered .., _, tm, Δ, mk, s, part, hk => Node.sweepPlain_complete m tk _ tm Δ mk s part hk
  | _, _, .flattenUnordered .., _, tm, Δ, mk, s, part, hk => Node.sweepPlain_complete m tk _ tm Δ mk s part hk
  | _, _, .tinto_keyed .., _, tm, Δ, mk, s, part, hk => Node.sweepPlain_complete m tk _ tm Δ mk s part hk
  | _, _, .tassume_ordering .., _, tm, Δ, mk, s, part, hk => Node.sweepPlain_complete m tk _ tm Δ mk s part hk
  | _, _, .reduce_watermark .., _, tm, Δ, mk, s, part, hk => Node.sweepPlain_complete m tk _ tm Δ mk s part hk
  | _, _, .kentries .., _, tm, Δ, mk, s, part, hk => Node.sweepPlain_complete m tk _ tm Δ mk s part hk
  | _, _, .kmap .., _, tm, Δ, mk, s, part, hk => Node.sweepPlain_complete m tk _ tm Δ mk s part hk
  | _, _, .kallTicks .., _, tm, Δ, mk, s, part, hk => Node.sweepPlain_complete m tk _ tm Δ mk s part hk
  | _, _, .kfold .., _, tm, Δ, mk, s, part, hk => Node.sweepPlain_complete m tk _ tm Δ mk s part hk
  | _, _, .ksnapshot .., _, tm, Δ, mk, s, part, hk => Node.sweepPlain_complete m tk _ tm Δ mk s part hk
  | _, _, .tfold .., _, tm, Δ, mk, s, part, hk => Node.sweepPlain_complete m tk _ tm Δ mk s part hk
  | _, _, .latest .., _, tm, Δ, mk, s, part, hk => Node.sweepPlain_complete m tk _ tm Δ mk s part hk
  | _, _, .latest_monotone .., _, tm, Δ, mk, s, part, hk => Node.sweepPlain_complete m tk _ tm Δ mk s part hk
  | _, _, .time, _, tm, Δ, mk, s, part, hk => Node.sweepPlain_complete m tk _ tm Δ mk s part hk
  | _, _, .snapshot_bounded .., _, tm, Δ, mk, s, part, hk => Node.sweepPlain_complete m tk _ tm Δ mk s part hk
  | _, _, .tick_scan .., _, tm, Δ, mk, s, part, hk => Node.sweepPlain_complete m tk _ tm Δ mk s part hk

theorem Dag.sweep_complete (m : Member L mem) (tk : Bool) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    Dag.valid d = true → (tm : Dag.Timing m d) → (Δ : HList (Kind.locDelta m) Γ) → (mk : Mask Γ) →
    (s : Dag.LocState m d) → (part : Dag.Part m d) → Mask.all mk = true →
    Mask.all (Dag.sweep m tk d tm Δ mk s part).2.2.1 = true ∧ Dag.allDone m d (Dag.sweep m tk d tm Δ mk s part).1.2
  | _, _, .ret xs, _, _, _, mk, _, _, hk => ⟨Mask.all_mapSubst hk xs, trivial⟩
  | _, _, .letN n rest, hv, tm, Δ, mk, s, part, hk => by
    obtain ⟨hvn, hvr⟩ := Dag.valid_letN n rest hv
    have ih1 := Node.sweep_complete m tk n hvn tm.1 Δ mk s.1 part.1 hk
    have ih2 := Dag.sweep_complete m tk rest hvr tm.2 ((memoEnv m (Node.sweep m tk n tm.1 Δ mk s.1 part.1).2.1).append Δ)
      ((Node.sweep m tk n tm.1 Δ mk s.1 part.1).2.2.1.append mk) s.2 part.2 (Mask.all_append ih1.1 hk)
    exact ⟨ih2.1, ih1.2, ih2.2⟩

end

mutual

/-- A fully `done`, consistent part holds the reference step. -/
theorem Node.Cons_final (m : Member L mem) (tk : Bool) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    (tm : Node.Timing m n) → (Δf : HList (Kind.locDelta m) Γ) → (s₀ s : Node.LocState m n) → (part : Node.Part m n) →
    Node.allDone m n part → Node.Cons m tk n tm Δf s₀ s part →
    s = (Node.locStep m tk n (Node.oracleOf m n tm s₀) Δf s₀).1 ∧ Node.partSends m n part = (Node.locStep m tk n (Node.oracleOf m n tm s₀) Δf s₀).2.2
  | _, _, .call N _, tm, Δf, s₀, s, part, hd, hc => Dag.Cons_final m tk N tm _ s₀ s part.2 hd hc
  | _, _, .fixN ks body, tm, Δf, s₀, s, part, hd, hc => by
    change NodePart _ _ × ((Mask (ks.map KnotKind.toKind) × HList (Kind.locDelta m) (ks.map KnotKind.toKind)) × Dag.Part m body) at part
    have hKe : part.2.1.2 = knotRef m tk ks body tm Δf s₀ := Agree.eq_of_all hd.1 hc.1
    exact Dag.Cons_final m tk body tm _ s₀ s part.2.2 hd.2 (hc.2 _ (by rw [hKe]; exact Agree.refl _ _))

  | _, _, .map .., tm, Δf, s₀, s, part, hd, hc => Node.phase_final m tk _ tm Δf s₀ s part.1 hd hc
  | _, _, .filterMap .., tm, Δf, s₀, s, part, hd, hc => Node.phase_final m tk _ tm Δf s₀ s part.1 hd hc
  | _, _, .broadcast_closed .., tm, Δf, s₀, s, part, hd, hc => Node.phase_final m tk _ tm Δf s₀ s part.1 hd hc
  | _, _, .demux .., tm, Δf, s₀, s, part, hd, hc => Node.phase_final m tk _ tm Δf s₀ s part.1 hd hc
  | _, _, .values .., tm, Δf, s₀, s, part, hd, hc => Node.phase_final m tk _ tm Δf s₀ s part.1 hd hc
  | _, _, .into_keyed .., tm, Δf, s₀, s, part, hd, hc => Node.phase_final m tk _ tm Δf s₀ s part.1 hd hc
  | _, _, .kassume_ordering .., tm, Δf, s₀, s, part, hd, hc => Node.phase_final m tk _ tm Δf s₀ s part.1 hd hc
  | _, _, .kfold_early_stop .., tm, Δf, s₀, s, part, hd, hc => Node.phase_final m tk _ tm Δf s₀ s part.1 hd hc
  | _, _, .kget_max_key .., tm, Δf, s₀, s, part, hd, hc => Node.phase_final m tk _ tm Δf s₀ s part.1 hd hc
  | _, _, .weaken_retries .., tm, Δf, s₀, s, part, hd, hc => Node.phase_final m tk _ tm Δf s₀ s part.1 hd hc
  | _, _, .union .., tm, Δf, s₀, s, part, hd, hc => Node.phase_final m tk _ tm Δf s₀ s part.1 hd hc
  | _, _, .assume_ordering .., tm, Δf, s₀, s, part, hd, hc => Node.phase_final m tk _ tm Δf s₀ s part.1 hd hc
  | _, _, .fold .., tm, Δf, s₀, s, part, hd, hc => Node.phase_final m tk _ tm Δf s₀ s part.1 hd hc
  | _, _, .fold_monotone .., tm, Δf, s₀, s, part, hd, hc => Node.phase_final m tk _ tm Δf s₀ s part.1 hd hc
  | _, _, .snapshot .., tm, Δf, s₀, s, part, hd, hc => Node.phase_final m tk _ tm Δf s₀ s part.1 hd hc
  | _, _, .batch .., tm, Δf, s₀, s, part, hd, hc => Node.phase_final m tk _ tm Δf s₀ s part.1 hd hc
  | _, _, .source_interval .., tm, Δf, s₀, s, part, hd, hc => Node.phase_final m tk _ tm Δf s₀ s part.1 hd hc
  | _, _, .firstTick .., tm, Δf, s₀, s, part, hd, hc => Node.phase_final m tk _ tm Δf s₀ s part.1 hd hc
  | _, _, .mapTick .., tm, Δf, s₀, s, part, hd, hc => Node.phase_final m tk _ tm Δf s₀ s part.1 hd hc
  | _, _, .zipTick .., tm, Δf, s₀, s, part, hd, hc => Node.phase_final m tk _ tm Δf s₀ s part.1 hd hc
  | _, _, .defer_tick .., tm, Δf, s₀, s, part, hd, hc => Node.phase_final m tk _ tm Δf s₀ s part.1 hd hc
  | _, _, .allTicks .., tm, Δf, s₀, s, part, hd, hc => Node.phase_final m tk _ tm Δf s₀ s part.1 hd hc
  | _, _, .flattenOrdered .., tm, Δf, s₀, s, part, hd, hc => Node.phase_final m tk _ tm Δf s₀ s part.1 hd hc
  | _, _, .flattenUnordered .., tm, Δf, s₀, s, part, hd, hc => Node.phase_final m tk _ tm Δf s₀ s part.1 hd hc
  | _, _, .tinto_keyed .., tm, Δf, s₀, s, part, hd, hc => Node.phase_final m tk _ tm Δf s₀ s part.1 hd hc
  | _, _, .tassume_ordering .., tm, Δf, s₀, s, part, hd, hc => Node.phase_final m tk _ tm Δf s₀ s part.1 hd hc
  | _, _, .reduce_watermark .., tm, Δf, s₀, s, part, hd, hc => Node.phase_final m tk _ tm Δf s₀ s part.1 hd hc
  | _, _, .kentries .., tm, Δf, s₀, s, part, hd, hc => Node.phase_final m tk _ tm Δf s₀ s part.1 hd hc
  | _, _, .kmap .., tm, Δf, s₀, s, part, hd, hc => Node.phase_final m tk _ tm Δf s₀ s part.1 hd hc
  | _, _, .kallTicks .., tm, Δf, s₀, s, part, hd, hc => Node.phase_final m tk _ tm Δf s₀ s part.1 hd hc
  | _, _, .kfold .., tm, Δf, s₀, s, part, hd, hc => Node.phase_final m tk _ tm Δf s₀ s part.1 hd hc
  | _, _, .ksnapshot .., tm, Δf, s₀, s, part, hd, hc => Node.phase_final m tk _ tm Δf s₀ s part.1 hd hc
  | _, _, .tfold .., tm, Δf, s₀, s, part, hd, hc => Node.phase_final m tk _ tm Δf s₀ s part.1 hd hc
  | _, _, .latest .., tm, Δf, s₀, s, part, hd, hc => Node.phase_final m tk _ tm Δf s₀ s part.1 hd hc
  | _, _, .latest_monotone .., tm, Δf, s₀, s, part, hd, hc => Node.phase_final m tk _ tm Δf s₀ s part.1 hd hc
  | _, _, .time, tm, Δf, s₀, s, part, hd, hc => Node.phase_final m tk _ tm Δf s₀ s part.1 hd hc
  | _, _, .snapshot_bounded .., tm, Δf, s₀, s, part, hd, hc => Node.phase_final m tk _ tm Δf s₀ s part.1 hd hc
  | _, _, .tick_scan .., tm, Δf, s₀, s, part, hd, hc => Node.phase_final m tk _ tm Δf s₀ s part.1 hd hc

theorem Dag.Cons_final (m : Member L mem) (tk : Bool) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    (tm : Dag.Timing m d) → (Δf : HList (Kind.locDelta m) Γ) → (s₀ s : Dag.LocState m d) → (part : Dag.Part m d) →
    Dag.allDone m d part → Dag.Cons m tk d tm Δf s₀ s part →
    s = (Dag.locStep m tk d (Dag.oracleOf m d tm s₀) Δf s₀).1 ∧ Dag.partSends m d part = (Dag.locStep m tk d (Dag.oracleOf m d tm s₀) Δf s₀).2.2
  | _, _, .ret _, _, _, _, _, _, _, _ => ⟨rfl, rfl⟩
  | _, _, .letN n rest, tm, Δf, s₀, s, part, hd, hc => by
    have ih1 := Node.Cons_final m tk n tm.1 Δf s₀.1 s.1 part.1 hd.1 hc.1
    have ih2 := Dag.Cons_final m tk rest tm.2 _ s₀.2 s.2 part.2 hd.2 hc.2
    dsimp only [Dag.locStep, Dag.oracleOf, Dag.partSends]
    rw [Node.memoState_eq, memoEnv_eq]
    exact ⟨Prod.ext ih1.1 ih2.1, Prod.ext ih1.2 ih2.2⟩

end

/-! ## The sweep is the step -/

/-- **The activation as a sweep is the reference step** on a valid graph. -/
theorem Dag.sweepTop_eq (m : Member L mem) (tk : Bool) {Γ : Ctx L mem} {outs : List (Kind L mem)} (d : Dag Γ outs)
    (hv : Dag.valid d = true) (tm : Dag.Timing m d) (Δ : HList (Kind.locDelta m) Γ) (s : Dag.LocState m d) :
    Dag.sweepTop m tk d tm Δ s = Dag.locStep m tk d (Dag.oracleOf m d tm s) Δ s := by
  have hs := Dag.sweep_sound m tk d hv tm Δ Δ (Mask.const Γ true) s s (Dag.Part.init m d) (Dag.Cons_init m tk d tm Δ s)
    (Agree.refl _ _)
  have hc := Dag.sweep_complete m tk d hv tm Δ (Mask.const Γ true) s (Dag.Part.init m d) (Mask.all_const_true Γ)
  have hf := Dag.Cons_final m tk d tm Δ s _ _ hc.2 hs.1
  exact Prod.ext hf.1 (Prod.ext (Agree.eq_of_all hc.1 hs.2.1) (hs.2.2.trans hf.2))

/-- **The activation is the pass** (D95): on a valid graph, `Dag.iterate` —
the sweep — computes exactly `Dag.pass`, the reference step every
process-level theorem is about. -/
theorem Dag.iterate_eq_pass (m : Member L mem) (tk : Bool) {Γ : Ctx L mem} {outs : List (Kind L mem)} (d : Dag Γ outs)
    (hv : Dag.valid d = true) (tm : Dag.Timing m d) (ib : Dag.Inbox m d) (Δ : HList (Kind.locDelta m) Γ)
    (s : Dag.LocState m d) : Dag.iterate m tk d tm ib Δ s = Dag.pass m tk d tm ib Δ s :=
  Dag.sweepTop_eq m tk d hv _ Δ _

/-- The activation at the wire is the pass at the wire. -/
theorem Dag.iterateWire_eq_passWire (m : Member L mem) (tk : Bool) {Γ : Ctx L mem} {outs : List (Kind L mem)} (d : Dag Γ outs)
    (hv : Dag.valid d = true) (cod : Dag.Codecs d) (tm : Dag.Timing m d) (batches : List ((src : Member L mem) × List WireMsg))
    (Δ : HList (Kind.locDelta m) Γ) (s : Dag.LocState m d) :
    Dag.iterateWire m tk d cod tm batches Δ s = Dag.passWire m tk d cod tm batches Δ s := by
  unfold Dag.iterateWire Dag.passWire
  rw [Dag.iterate_eq_pass m tk d hv]

/-- **The production body at the bytes is the pass at the bytes** (D95): what `lake exe wired`
runs on a valid graph is the proof layer's unit. -/
theorem Dag.iterateBytes_eq_passBytes (m : Member L mem) (tk : Bool) {Γ : Ctx L mem} {outs : List (Kind L mem)} (d : Dag Γ outs)
    (hv : Dag.valid d = true) (cod : Dag.Codecs d) (tm : Dag.Timing m d) (chans : List ((src : Member L mem) × ByteArray))
    (Δ : HList (Kind.locDelta m) Γ) (s : Dag.LocState m d) :
    Dag.iterateBytes m tk d cod tm chans Δ s = Dag.passBytes m tk d cod tm chans Δ s := by
  simp only [Dag.iterateBytes, Dag.passBytes]
  rw [Dag.iterateWire_eq_passWire m tk d hv]

end Cons

end Project

end Hydro
