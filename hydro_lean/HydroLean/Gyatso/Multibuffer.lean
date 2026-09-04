import HydroLean.Flo.Operator
import HydroLean.Gyatso.ARS

/-!
# Cluster multibuffers (dissertation §3.3.3)

Clusters run the same operator on `n` machines with independent inputs
(SPMD). Gyatso models this with *multibuffers*: a stream placed on a cluster
keeps one buffer per member, and the operational semantics nondeterministically
picks which member steps (Figs 3.4–3.6).

- `multiColl n L`: the multibuffer collection: carrier `Fin n → L.C` with
  pointwise concatenation (the paper's partial-map deltas are total maps with
  `∅` at absent members — equivalent since `∅` is a right identity).
- `Operator.clusterOp n op`: the cluster upgrade (Fig 3.5/3.6): state is one
  copy of `op`'s state per member; a step delegates to one member and emits a
  delta that is `op`'s delta at that member and `∅` elsewhere.
- `Operator.clusterOp_lawful`: **all Flo obligations lift**: because a member's
  step touches only that member's slice, cluster executions project to
  independent member executions and member executions lift back (a strong
  bisimulation per member). Confluence, eager execution, and streaming progress
  are then assembled member-by-member (§3.3.3: "we still preserve the
  determinism and progress properties of Flo when considering the inputs and
  outputs across all members"). Nondeterministic member interleaving is
  provably unobservable — the cluster analogue of eventual determinism.
-/

namespace HydroLean

universe u

namespace Gyatso

/-- The multibuffer collection (§3.3.3): one buffer per cluster member,
pointwise concatenation, `fix`, and `∅`. -/
def multiColl (n : Nat) (L : Coll.{u}) : Coll.{u} where
  C := Fin n → L.C
  concat f g := fun x => L.concat (f x) (g x)
  empty := fun _ => L.empty
  fix f := fun x => L.fix (f x)
  concat_empty f := funext fun x => L.concat_empty (f x)
  fix_fixed f δ := funext fun x => L.fix_fixed (f x) (δ x)

/-- A multibuffer is fixed iff every member's buffer is fixed. -/
theorem multiColl_fixed_iff {n : Nat} {L : Coll.{u}} (f : Fin n → L.C) :
    (multiColl n L).Fixed f ↔ ∀ x, L.Fixed (f x) := by
  constructor
  · intro h x δ
    exact congrFun (h fun _ => δ) x
  · intro h δ
    exact funext fun x => h x (δ x)

/-- Update one slot of a function (local `Function.update`; core lacks it). -/
def upd {n : Nat} {γ : Type u} (f : Fin n → γ) (x : Fin n) (v : γ) : Fin n → γ :=
  fun y => if y = x then v else f y

@[simp] theorem upd_self {n : Nat} {γ : Type u} (f : Fin n → γ) (x : Fin n) (v : γ) :
    upd f x v x = v := by simp [upd]

theorem upd_other {n : Nat} {γ : Type u} (f : Fin n → γ) (x y : Fin n) (v : γ)
    (h : y ≠ x) : upd f x v y = f y := by simp [upd, h]

/-! ## The one-member-decreases order and its well-foundedness -/

/-- `OneDec r f g`: exactly one member strictly decreases (via `r`), all others
are unchanged — the shape of the cluster step's effect on the per-member
termination measures. -/
def OneDec {n : Nat} {γ : Type u} (r : γ → γ → Prop) (f g : Fin n → γ) : Prop :=
  ∃ x, r (f x) (g x) ∧ ∀ y, y ≠ x → f y = g y

/-- Lexicographic products of well-founded relations are well-founded
(local copy; core provides `Prod.Lex` but not its well-foundedness). -/
theorem lex_wf {γ : Type u} {δ : Type v} {ra : γ → γ → Prop} {rb : δ → δ → Prop}
    (ha : WellFounded ra) (hb : WellFounded rb) : WellFounded (Prod.Lex ra rb) := by
  constructor
  rintro ⟨a, b⟩
  induction a using ha.induction generalizing b with
  | _ a iha =>
    induction b using hb.induction with
    | _ b ihb =>
      constructor
      rintro ⟨a', b'⟩ h
      cases h with
      | left _ _ h => exact iha a' h b'
      | right _ h => exact ihb b' h

/-- **Finite-product well-foundedness**: the one-member-decreases order over
`Fin n → γ` is well-founded when the member order is, by induction on `n`
through the isomorphism `(Fin (n+1) → γ) ≃ γ × (Fin n → γ)` and `Prod.Lex`. -/
theorem oneDec_wf {γ : Type u} {r : γ → γ → Prop} (hr : WellFounded r) :
    ∀ n, WellFounded (OneDec (n := n) r)
  | 0 => by
    constructor
    intro f
    constructor
    rintro g ⟨x, -, -⟩
    exact x.elim0
  | n + 1 => by
    have ih := oneDec_wf hr n
    have hlex := lex_wf hr ih
    have hsub : Subrelation (OneDec (n := n + 1) r)
        (InvImage (Prod.Lex r (OneDec (n := n) r))
          (fun f : Fin (n + 1) → γ => (f 0, fun i : Fin n => f i.succ))) := by
      rintro f g ⟨x, hx, hother⟩
      show Prod.Lex r (OneDec (n := n) r)
        (f 0, fun i : Fin n => f i.succ) (g 0, fun i : Fin n => g i.succ)
      rcases Fin.eq_zero_or_eq_succ x with rfl | ⟨i, rfl⟩
      · exact Prod.Lex.left _ _ hx
      · have h0 : f 0 = g 0 := hother 0 (by simp [Fin.ext_iff])
        rw [h0]
        refine Prod.Lex.right _ ⟨i, hx, fun j hj => ?_⟩
        exact hother j.succ (by simpa [Fin.succ_inj] using hj)
    exact Subrelation.wf hsub (InvImage.wf _ hlex)

/-! ## Per-member views of multibuffer tuples -/

/-- Head of a nonempty tuple. -/
def vhead {L : Coll.{u}} {ls : List Coll.{u}} : Vals (L :: ls) → L.C
  | .cons x _ => x

/-- Tail of a nonempty tuple. -/
def vtail {L : Coll.{u}} {ls : List Coll.{u}} : Vals (L :: ls) → Vals ls
  | .cons _ xs => xs

/-- Project member `x`'s slice out of a tuple of multibuffers. -/
def projV {n : Nat} (x : Fin n) :
    {ls : List Coll.{u}} → Vals (ls.map (multiColl n)) → Vals ls
  | [], _ => .nil
  | _ :: _, .cons f fs => .cons (f x) (projV x fs)

/-- Replace member `x`'s slice in a tuple of multibuffers. -/
def updateV {n : Nat} (x : Fin n) :
    {ls : List Coll.{u}} → Vals (ls.map (multiColl n)) → Vals ls →
      Vals (ls.map (multiColl n))
  | [], v, _ => v
  | _ :: _, .cons f fs, .cons w ws => .cons (upd f x w) (updateV x fs ws)

/-- Embed a member delta as a multibuffer delta (`∅` at all other members). -/
def injectV {n : Nat} (x : Fin n) :
    {ls : List Coll.{u}} → Vals ls → Vals (ls.map (multiColl n))
  | [], _ => .nil
  | L :: _, .cons w ws => .cons (fun y => if y = x then w else L.empty) (injectV x ws)

@[simp] theorem projV_concat {n : Nat} (x : Fin n) :
    ∀ {ls : List Coll.{u}} (v w : Vals (ls.map (multiColl n))),
      projV x (v.concat w) = (projV x v).concat (projV x w)
  | [], .nil, .nil => rfl
  | _ :: _, .cons f fs, .cons g gs => by
    show Vals.cons ((multiColl _ _).concat f g x) (projV x (fs.concat gs)) =
      Vals.cons (Coll.concat _ (f x) (g x)) ((projV x fs).concat (projV x gs))
    rw [projV_concat x fs gs]
    rfl

@[simp] theorem projV_updateV_self {n : Nat} (x : Fin n) :
    ∀ {ls : List Coll.{u}} (v : Vals (ls.map (multiColl n))) (w : Vals ls),
      projV x (updateV x v w) = w
  | [], v, .nil => rfl
  | _ :: _, .cons f fs, .cons w ws => by
    show Vals.cons (upd f x w x) (projV x (updateV x fs ws)) = Vals.cons w ws
    rw [projV_updateV_self x fs ws]
    exact congrArg (fun v => Vals.cons v ws) (upd_self f x w)

theorem projV_updateV_other {n : Nat} {x y : Fin n} (h : y ≠ x) :
    ∀ {ls : List Coll.{u}} (v : Vals (ls.map (multiColl n))) (w : Vals ls),
      projV y (updateV x v w) = projV y v
  | [], v, .nil => rfl
  | _ :: _, .cons f fs, .cons w ws => by
    show Vals.cons (upd f x w y) (projV y (updateV x fs ws)) = Vals.cons (f y) (projV y fs)
    rw [projV_updateV_other h fs ws]
    exact congrArg (fun v => Vals.cons v (projV y fs)) (upd_other f x y w h)

@[simp] theorem projV_injectV_self {n : Nat} (x : Fin n) :
    ∀ {ls : List Coll.{u}} (w : Vals ls), projV x (injectV (ls := ls) x w) = w
  | [], .nil => rfl
  | _ :: _, .cons w ws => by
    show Vals.cons (if x = x then w else _) (projV x (injectV x ws)) = Vals.cons w ws
    rw [if_pos rfl, projV_injectV_self x ws]

theorem projV_injectV_other {n : Nat} {x y : Fin n} (h : y ≠ x) :
    ∀ {ls : List Coll.{u}} (w : Vals ls),
      projV y (injectV (ls := ls) x w) = Vals.empty ls
  | [], .nil => rfl
  | _ :: _, .cons w ws => by
    show Vals.cons (if y = x then w else _) (projV y (injectV x ws)) =
      Vals.cons (Coll.empty _) (Vals.empty _)
    rw [if_neg h, projV_injectV_other h ws]

@[simp] theorem projV_fixAll {n : Nat} (x : Fin n) :
    ∀ {ls : List Coll.{u}} (v : Vals (ls.map (multiColl n))),
      projV x v.fixAll = (projV x v).fixAll
  | [], .nil => rfl
  | _ :: _, .cons f fs => by
    show Vals.cons ((multiColl _ _).fix f x) (projV x fs.fixAll) =
      Vals.cons (Coll.fix _ (f x)) (projV x fs).fixAll
    rw [projV_fixAll x fs]
    rfl

/-- Multibuffer tuples with equal member views are equal. -/
theorem projV_ext {n : Nat} :
    ∀ {ls : List Coll.{u}} {v w : Vals (ls.map (multiColl n))},
      (∀ x, projV x v = projV x w) → v = w
  | [], .nil, .nil, _ => rfl
  | _ :: _, .cons f fs, .cons g gs, h => by
    have hpair : ∀ x, f x = g x ∧ projV x fs = projV x gs := by
      intro x
      have hx := h x
      exact ⟨congrArg vhead hx, congrArg vtail hx⟩
    have hhead : f = g := funext fun x => (hpair x).1
    have htail : fs = gs := projV_ext fun x => (hpair x).2
    rw [hhead, htail]

/-- `FixedWhere` for multibuffers is `FixedWhere` at every member. -/
theorem fixedWhere_projV {n : Nat} :
    ∀ {ls : List Coll.{u}} (v : Vals (ls.map (multiColl n))) (bs : List Boundedness),
      v.FixedWhere bs → ∀ x, (projV x v).FixedWhere bs
  | [], .nil, _, _, _ => trivial
  | _ :: _, .cons _ _, [], _, _ => trivial
  | _ :: _, .cons f fs, _ :: bs, h, x =>
    ⟨fun hb => (multiColl_fixed_iff f).mp (h.1 hb) x,
     fixedWhere_projV fs bs h.2 x⟩

/-- `FixedWhere` for multibuffers from all member views. -/
theorem fixedWhere_of_projV {n : Nat} :
    ∀ {ls : List Coll.{u}} (v : Vals (ls.map (multiColl n))) (bs : List Boundedness),
      (∀ x, (projV x v).FixedWhere bs) → v.FixedWhere bs
  | [], .nil, _, _ => trivial
  | _ :: _, .cons _ _, [], _ => trivial
  | _ :: _, .cons f fs, _ :: bs, h =>
    ⟨fun hb => (multiColl_fixed_iff f).mpr fun x => (h x).1 hb,
     fixedWhere_of_projV fs bs fun x => (h x).2⟩

/-! ## The cluster operator upgrade (Figs 3.5, 3.6) -/

namespace Cluster

open Gyatso

variable {ins outs : List Coll.{u}}

end Cluster

end Gyatso

namespace Operator

open Gyatso

variable {ins outs : List Coll.{u}}

/-- The cluster upgrade of an operator (Figs 3.5/3.6): `n` independent copies,
one per member; a step nondeterministically picks a member `x`, runs `op`'s
step on `x`'s slice, and emits `op`'s delta at `x` (with `∅` elsewhere). -/
def clusterOp (n : Nat) (op : Operator ins outs) :
    Operator (ins.map (multiColl n)) (outs.map (multiColl n)) where
  State := Fin n → op.State
  step I s I' s' δ :=
    ∃ (x : Fin n) (Ix : Vals ins) (sx : op.State) (δx : Vals outs),
      op.step (projV x I) (s x) Ix sx δx ∧
      I' = updateV x I Ix ∧ s' = upd s x sx ∧ δ = injectV x δx
  inBounds := op.inBounds
  outBounds := op.outBounds
  Inv I s O := ∀ x, op.Inv (projV x I) (s x) (projV x O)

namespace clusterOp

variable {n : Nat} {op : Operator ins outs}

/-- Member `x`'s view of a cluster configuration. -/
def projCfg (x : Fin n) (c : (clusterOp n op).Config) : op.Config :=
  ⟨projV x c.I, c.st x, projV x c.O⟩

theorem projCfg_ext {c c' : (clusterOp n op).Config}
    (h : ∀ x, projCfg x c = projCfg x c') : c = c' := by
  obtain ⟨I, s, O⟩ := c
  obtain ⟨I', s', O'⟩ := c'
  have hI : I = I' := projV_ext fun x => congrArg Operator.Config.I (h x)
  have hO : O = O' := projV_ext fun x => congrArg Operator.Config.O (h x)
  have hs : s = s' := funext fun x => congrArg Operator.Config.st (h x)
  rw [hI, hO, hs]

/-- A cluster step is a member step at some `x`, invisible to other members. -/
theorem step_proj {c c' : (clusterOp n op).Config}
    (h : (clusterOp n op).OpStep c c') :
    ∃ x, op.OpStep (projCfg x c) (projCfg x c') ∧
      ∀ y, y ≠ x → projCfg y c' = projCfg y c := by
  obtain ⟨δ, ⟨x, Ix, sx, δx, hstep, hI', hs', hδ⟩, hO⟩ := h
  have hsx : c'.st x = sx := by
    rw [hs']; exact upd_self c.st x sx
  have hIx : projV x c'.I = Ix := by
    rw [hI']; exact projV_updateV_self x c.I Ix
  refine ⟨x, ⟨δx, ?_, ?_⟩, ?_⟩
  · show op.step (projV x c.I) (c.st x) (projV x c'.I) (c'.st x) δx
    rw [hsx, hIx]
    exact hstep
  · show projV x c'.O = (projV x c.O).concat δx
    rw [hO, hδ, projV_concat, projV_injectV_self]
  · intro y hy
    have h1 : projV y c'.I = projV y c.I := by
      rw [hI']; exact projV_updateV_other hy c.I Ix
    have h2 : c'.st y = c.st y := by
      rw [hs']; exact upd_other c.st x y sx hy
    have h3 : projV y c'.O = projV y c.O := by
      rw [hO, hδ, projV_concat, projV_injectV_other hy, Vals.concat_empty]
    show (⟨projV y c'.I, c'.st y, projV y c'.O⟩ : op.Config) = ⟨_, _, _⟩
    rw [h1, h2, h3]

/-- A member step lifts to a cluster step, invisible to other members. -/
theorem step_lift {c : (clusterOp n op).Config} {x : Fin n} {d : op.Config}
    (h : op.OpStep (projCfg x c) d) :
    ∃ c', (clusterOp n op).OpStep c c' ∧ projCfg x c' = d ∧
      ∀ y, y ≠ x → projCfg y c' = projCfg y c := by
  obtain ⟨δx, hstep, hO⟩ := h
  refine ⟨⟨updateV x c.I d.I, upd c.st x d.st, c.O.concat (injectV x δx)⟩,
    ⟨injectV x δx, ⟨x, d.I, d.st, δx, hstep, rfl, rfl, rfl⟩, rfl⟩, ?_, ?_⟩
  · show (⟨projV x (updateV x c.I d.I), upd c.st x d.st x,
      projV x (c.O.concat (injectV x δx))⟩ : op.Config) = d
    have h1 : projV x (updateV x c.I d.I) = d.I := projV_updateV_self x c.I d.I
    have h2 : upd c.st x d.st x = d.st := upd_self c.st x d.st
    have h3 : projV x (c.O.concat (injectV x δx)) = d.O := by
      rw [projV_concat, projV_injectV_self]
      exact hO.symm
    rw [h1, h2, h3]
  · intro y hy
    have h1 : projV y (updateV x c.I d.I) = projV y c.I :=
      projV_updateV_other hy c.I d.I
    have h2 : upd c.st x d.st y = c.st y := upd_other c.st x y d.st hy
    have h3 : projV y (c.O.concat (injectV x δx)) = projV y c.O := by
      rw [projV_concat, projV_injectV_other hy, Vals.concat_empty]
    show (⟨projV y (updateV x c.I d.I), upd c.st x d.st y,
      projV y (c.O.concat (injectV x δx))⟩ : op.Config) = ⟨_, _, _⟩
    rw [h1, h2, h3]

/-- Cluster traces project to member traces. -/
theorem star_proj {c c' : (clusterOp n op).Config}
    (h : Star (clusterOp n op).OpStep c c') (x : Fin n) :
    Star op.OpStep (projCfg x c) (projCfg x c') := by
  induction h with
  | refl => exact Star.refl _
  | tail _ hbc ih =>
    obtain ⟨y, hy, hother⟩ := step_proj hbc
    by_cases hxy : x = y
    · subst hxy
      exact ih.tail hy
    · rw [hother x hxy]
      exact ih

/-- Member traces lift to cluster traces, invisible to other members. -/
theorem star_lift {c : (clusterOp n op).Config} {x : Fin n} {d : op.Config}
    (h : Star op.OpStep (projCfg x c) d) :
    ∃ c', Star (clusterOp n op).OpStep c c' ∧ projCfg x c' = d ∧
      ∀ y, y ≠ x → projCfg y c' = projCfg y c := by
  generalize hstart : projCfg x c = start at h
  induction h generalizing c with
  | refl => exact ⟨c, Star.refl _, hstart, fun _ _ => rfl⟩
  | @tail b e _ hbe ih =>
    obtain ⟨c₁, hstar₁, hproj₁, hother₁⟩ := ih hstart
    obtain ⟨c₂, hstep₂, hproj₂, hother₂⟩ := step_lift (hproj₁ ▸ hbe)
    exact ⟨c₂, hstar₁.tail hstep₂, hproj₂,
      fun y hy => (hother₂ y hy).trans (hother₁ y hy)⟩

/-- A cluster configuration is stuck iff every member is. -/
theorem stuck_iff {c : (clusterOp n op).Config} :
    Stuck (clusterOp n op).OpStep c ↔ ∀ x, Stuck op.OpStep (projCfg x c) := by
  constructor
  · rintro h x ⟨d, hd⟩
    obtain ⟨c', hc', -, -⟩ := step_lift hd
    exact h ⟨c', hc'⟩
  · rintro h ⟨c', hc'⟩
    obtain ⟨x, hx, -⟩ := step_proj hc'
    exact h x ⟨_, hx⟩

/-- Lift independent member runs (over a duplicate-free list of members) into
one cluster run reaching all of their endpoints simultaneously. -/
theorem lift_many {c : (clusterOp n op).Config} {d : Fin n → op.Config} :
    ∀ (todo : List (Fin n)), todo.Nodup →
      (∀ x ∈ todo, Star op.OpStep (projCfg x c) (d x)) →
      ∃ e, Star (clusterOp n op).OpStep c e ∧
        (∀ x ∈ todo, projCfg x e = d x) ∧
        (∀ y, y ∉ todo → projCfg y e = projCfg y c)
  | [], _, _ => ⟨c, Star.refl _, by simp, fun _ _ => rfl⟩
  | x :: rest, hnodup, hall => by
    obtain ⟨c₁, hstar₁, hproj₁, hother₁⟩ :=
      star_lift (hall x (List.mem_cons_self ..))
    have hxrest : x ∉ rest := (List.nodup_cons.mp hnodup).1
    obtain ⟨e, hstar₂, hproj₂, hother₂⟩ :=
      lift_many (c := c₁) (d := d) rest (List.nodup_cons.mp hnodup).2
        (fun y hy => by
          rw [hother₁ y (fun hyx => hxrest (hyx ▸ hy))]
          exact hall y (List.mem_cons_of_mem _ hy))
    refine ⟨e, hstar₁.trans hstar₂, ?_, ?_⟩
    · intro z hz
      rcases List.mem_cons.mp hz with rfl | hz
      · rw [hother₂ z hxrest, hproj₁]
      · exact hproj₂ z hz
    · intro y hy
      have hyx : y ≠ x := fun h => hy (h ▸ List.mem_cons_self ..)
      have hyrest : y ∉ rest := fun h => hy (List.mem_cons_of_mem _ h)
      rw [hother₂ y hyrest, hother₁ y hyx]

/-- Lift one member run per member into a single cluster run. -/
theorem lift_all {c : (clusterOp n op).Config} {d : Fin n → op.Config}
    (h : ∀ x, Star op.OpStep (projCfg x c) (d x)) :
    ∃ e, Star (clusterOp n op).OpStep c e ∧ ∀ x, projCfg x e = d x := by
  obtain ⟨e, hstar, hin, -⟩ :=
    lift_many (List.finRange n) (List.nodup_finRange n)
      (fun x _ => h x)
  exact ⟨e, hstar, fun x => hin x (List.mem_finRange x)⟩

/-- Member views of delta introduction. -/
theorem projCfg_addDelta (x : Fin n) (c : (clusterOp n op).Config)
    (Δ : Vals (ins.map (multiColl n))) :
    projCfg x (c.addDelta Δ) = (projCfg x c).addDelta (projV x Δ) := by
  show (⟨projV x (c.I.concat Δ), c.st x, projV x c.O⟩ : op.Config) = _
  rw [projV_concat]
  rfl

/-- Member views of input fixing. -/
theorem projCfg_fixInputs (x : Fin n) (c : (clusterOp n op).Config) :
    projCfg x c.fixInputs = (projCfg x c).fixInputs := by
  show (⟨projV x c.I.fixAll, c.st x, projV x c.O⟩ : op.Config) = _
  rw [projV_fixAll]
  rfl

/-- **The cluster upgrade preserves all Flo obligations** (§3.3.3): member
interleaving is unobservable, so a cluster of lawful operators is a lawful
operator. -/
theorem lawful (n : Nat) (hop : op.Lawful) : (clusterOp n op).Lawful := by
  obtain ⟨r, hwfr, hdec⟩ := hop.wf_decreasing
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_⟩
  · -- wf_decreasing: one member decreases in `op`'s order, others unchanged
    refine ⟨InvImage (OneDec r)
      (fun p => fun x => (p.1 x, projV x p.2)), InvImage.wf _ (oneDec_wf hwfr n), ?_⟩
    rintro I s I' s' δ ⟨x, Ix, sx, δx, hstep, hI', hs', -⟩
    refine ⟨x, ?_, ?_⟩
    · show r (s' x, projV x I') (s x, projV x I)
      have h1 : s' x = sx := by rw [hs']; exact upd_self s x sx
      have h2 : projV x I' = Ix := by rw [hI']; exact projV_updateV_self x I Ix
      rw [h1, h2]
      exact hdec hstep
    · intro y hy
      show ((s' y, projV y I') : op.State × Vals ins) = (s y, projV y I)
      have h1 : s' y = s y := by rw [hs']; exact upd_other s x y sx hy
      have h2 : projV y I' = projV y I := by
        rw [hI']; exact projV_updateV_other hy I Ix
      rw [h1, h2]
  · -- confluence: join member-by-member, then lift all joins
    intro c c₁ c₂ h₁ h₂
    have hjoin : ∀ x, Joinable op.OpStep (projCfg x c₁) (projCfg x c₂) :=
      fun x => hop.confluent _ _ _ (star_proj h₁ x) (star_proj h₂ x)
    have hd₁ := fun x => (Classical.choose_spec (hjoin x)).1
    have hd₂ := fun x => (Classical.choose_spec (hjoin x)).2
    obtain ⟨e₁, he₁, he₁proj⟩ := lift_all hd₁
    obtain ⟨e₂, he₂, he₂proj⟩ := lift_all hd₂
    have : e₁ = e₂ := projCfg_ext fun x => (he₁proj x).trans (he₂proj x).symm
    exact ⟨e₁, he₁, this ▸ he₂⟩
  · -- eager execution: member-by-member via `op`'s eagerness
    intro Δ c c' hstep
    obtain ⟨x, hx, hother⟩ := step_proj hstep
    obtain ⟨dx, hdx₁, hdx₂⟩ := hop.eager (projV x Δ) _ _ hx
    -- target member views after the delta
    have hd : ∀ y, Star op.OpStep (projCfg y (c.addDelta Δ))
        (if hyx : y = x then dx else projCfg y (c'.addDelta Δ)) := by
      intro y
      by_cases hyx : y = x
      · subst hyx
        rw [dif_pos rfl, projCfg_addDelta]
        exact hdx₁
      · rw [dif_neg hyx, projCfg_addDelta, projCfg_addDelta, hother y hyx]
        exact Star.refl _
    have hd' : ∀ y, Star op.OpStep (projCfg y (c'.addDelta Δ))
        (if hyx : y = x then dx else projCfg y (c'.addDelta Δ)) := by
      intro y
      by_cases hyx : y = x
      · subst hyx
        rw [dif_pos rfl, projCfg_addDelta]
        exact hdx₂
      · rw [dif_neg hyx]
        exact Star.refl _
    obtain ⟨e₁, he₁, he₁proj⟩ := lift_all hd
    obtain ⟨e₂, he₂, he₂proj⟩ := lift_all hd'
    have : e₁ = e₂ := projCfg_ext fun y => (he₁proj y).trans (he₂proj y).symm
    exact ⟨e₁, he₁, this ▸ he₂⟩
  · -- streaming progress: apply `op`'s progress at each member, lift the
    -- maximality runs one member at a time
    intro c f hinv hfix hnorm
    have hmember : ∀ x,
        op.OutputsMaximal (projCfg x c) (projCfg x f) ∧
          (projCfg x f).O.FixedWhere op.outBounds := by
      intro x
      refine hop.progress _ _ (hinv x) (fixedWhere_projV c.I _ hfix x) ?_
      exact ⟨star_proj hnorm.1 x, stuck_iff.mp hnorm.2 x⟩
    have hm := fun x => (hmember x).1
    have hmax := fun x =>
      Classical.choose_spec (Classical.choose_spec (hm x))
    -- lift the per-member maximality runs
    obtain ⟨e, he, heproj⟩ := lift_all (c := c.fixInputs)
      (d := fun x => ⟨Classical.choose (hm x),
        Classical.choose (Classical.choose_spec (hm x)), (projCfg x f).O.fixAll⟩)
      (fun x => by rw [projCfg_fixInputs]; exact (hmax x).1)
    constructor
    · -- outputs maximal
      have heO : e.O = f.O.fixAll := projV_ext fun x => by
        have := congrArg Operator.Config.O (heproj x)
        simp only at this
        rw [projV_fixAll]
        exact this
      refine ⟨e.I, e.st, ?_, ?_⟩
      · have : e = ⟨e.I, e.st, f.O.fixAll⟩ := by rw [← heO]
        exact this ▸ he
      · have hstucke : Stuck (clusterOp n op).OpStep e := by
          rw [stuck_iff]
          intro x
          rw [heproj x]
          exact (hmax x).2
        have : e = ⟨e.I, e.st, f.O.fixAll⟩ := by rw [← heO]
        exact this ▸ hstucke
    · -- bounded outputs fixed, member by member
      exact fixedWhere_of_projV f.O _ fun x => (hmember x).2
  · -- inv_step
    intro c c' hinv hstep
    obtain ⟨x, hx, hother⟩ := step_proj hstep
    intro y
    by_cases hyx : y = x
    · subst hyx
      exact hop.inv_step (hinv y) hx
    · have := hother y hyx
      have h1 : projV y c'.I = projV y c.I := congrArg Operator.Config.I this
      have h2 : c'.st y = c.st y := congrArg Operator.Config.st this
      have h3 : projV y c'.O = projV y c.O := congrArg Operator.Config.O this
      rw [h1, h2, h3]
      exact hinv y
  · -- inv_delta
    intro I s O Δ hinv x
    rw [projV_concat]
    exact hop.inv_delta _ (hinv x)

end clusterOp

end Operator

end HydroLean
