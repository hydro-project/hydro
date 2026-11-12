import Hydro.Project.Step
import Hydro.Graph.Causal

/-!
# Hydro · Project — the lock-step machine respects the lag analysis

`Dag.step_agree0`: two increment environments that agree at the wires the lag analysis puts
at lag `0` give step outputs that agree at the outputs the analysis puts at lag `0`. It is the
machine-level shadow of `Dag.denote_agreeL` restricted to "lag `0` or not". An immediate
operator reads only its inputs; a strict node (`broadcast_closed`, `demux`, `defer_tick`) emits
a function of its cell alone and only pushes its input (`ChanCell.stepG_snd`); a knot block's
knot outputs sit at the `max` of its externals' lags.

For a valid knot body, iterating its step on the knot increments from empty ones tracks the
lag analysis's Jacobi iteration (`Dag.step_knotIter_agree0`), which reaches "every knot known"
in as many rounds as there are knots (`iter_zero`). Therefore the lock-step machine's `fixN` is
the body at any fixpoint of the knot increments (`Node.step_fixN_eq`); `Dag.step_spec`
(`Project/Model/StepMain.lean`) and `Dag.step_wired` (`Project/Model/WiredMain.lean`) rewrite
the knot case into this shape.
-/

namespace Hydro

namespace Project

variable {L : Type} {mem : L → Nat}

/-- Two increment environments agree at the lag-`0` positions of a lag vector. The carrier `f`
is generic so the same relation serves the lock-step `Kind.delta` and the process's
`Kind.locDelta`. -/
def DAgree0 {f : Kind L mem → Type} : {κs : List (Kind L mem)} → Lags κs → HList f κs → HList f κs → Prop
  | [], _, _, _ => True
  | _ :: _, l, x, y => (l.1 = 0 → x.1 = y.1) ∧ DAgree0 l.2 x.2 y.2

namespace DAgree0
variable {f : Kind L mem → Type}

theorem refl : {κs : List (Kind L mem)} → (l : Lags κs) → (x : HList f κs) → DAgree0 l x x
  | [], _, _ => trivial
  | _ :: _, l, x => ⟨fun _ => rfl, refl l.2 x.2⟩

theorem get : {Γ : Ctx L mem} → {κ : Kind L mem} → {l : Lags Γ} → {x y : HList f Γ} →
    DAgree0 l x y → (v : Var Γ κ) → l.get v = 0 → x.get v = y.get v
  | _ :: _, _, _, _, _, h, .zero, h0 => h.1 h0
  | _ :: _, _, _, _, _, h, .succ v, h0 => get h.2 v h0

theorem append : {κs Γ : List (Kind L mem)} → {l : Lags κs} → {l' : Lags Γ} →
    {x y : HList f κs} → {x' y' : HList f Γ} →
    DAgree0 l x y → DAgree0 l' x' y' → DAgree0 (l.append l') (x.append x') (y.append y')
  | [], _, _, _, _, _, _, _, _, h' => h'
  | _ :: _, _, _, _, _, _, _, _, h, h' => ⟨h.1, append h.2 h'⟩

theorem take : {κs Γ : List (Kind L mem)} → {l : Lags (κs ++ Γ)} → {x y : HList f (κs ++ Γ)} →
    DAgree0 l x y → DAgree0 (HList.take (κs := κs) l) (HList.take (κs := κs) x) (HList.take (κs := κs) y)
  | [], _, _, _, _, _ => trivial
  | _ :: _, _, _, _, _, h => ⟨h.1, take h.2⟩

theorem drop : {κs Γ : List (Kind L mem)} → {l : Lags (κs ++ Γ)} → {x y : HList f (κs ++ Γ)} →
    DAgree0 l x y → DAgree0 (HList.drop (κs := κs) l) (HList.drop (κs := κs) x) (HList.drop (κs := κs) y)
  | [], _, _, _, _, h => h
  | _ :: κs, Γ, _, _, _, h => drop (κs := κs) (Γ := Γ) h.2

theorem mapSubst {Γ Γ' : Ctx L mem} {l : Lags Γ} {x y : HList f Γ} (h : DAgree0 l x y)
    (args : Subst Γ' Γ) :
    DAgree0 (args.map (fun v => l.get v)) (args.map (fun v => x.get v)) (args.map (fun v => y.get v)) := by
  induction Γ' with
  | nil => trivial
  | cons κ κs ih => exact ⟨h.get args.1, ih args.2⟩

/-- Raising the lag vector pointwise (`Lags.Le`) weakens the claim: fewer positions are at `0`. -/
theorem mono_lag : {κs : List (Kind L mem)} → {l l' : Lags κs} → Lags.Le l l' →
    {x y : HList f κs} → DAgree0 l x y → DAgree0 l' x y
  | [], _, _, _, _, _, _ => trivial
  | _ :: _, _, _, hl, _, _, h => ⟨fun h0 => h.1 (Nat.le_zero.mp (h0 ▸ hl.1)), mono_lag hl.2 h.2⟩

/-- At an all-zero lag vector, agreement is equality. -/
theorem eq_of_zero : {κs : List (Kind L mem)} → {l : Lags κs} → Lags.Le l (Lags.const κs 0) →
    {x y : HList f κs} → DAgree0 l x y → x = y
  | [], _, _, _, _, _ => rfl
  | _ :: _, _, hl, x, y, h => by
    show (x.1, x.2) = (y.1, y.2)
    rw [h.1 (Nat.le_zero.mp hl.1), eq_of_zero hl.2 h.2]

/-- At a constant lag vector, one condition covers every position: equality if the constant is
`0`. -/
theorem const_of : (κs : List (Kind L mem)) → {v : Nat} → {x y : HList f κs} →
    (v = 0 → x = y) → DAgree0 (Lags.const κs v) x y
  | [], _, _, _, _ => trivial
  | _ :: κs, v, x, y, h => ⟨fun h0 => congrArg Prod.fst (h h0), const_of κs (fun h0 => congrArg Prod.snd (h h0))⟩

end DAgree0

/-- `take` and `drop` of an `HList` reassemble it. -/
theorem HList.take_append_drop {f : Kind L mem → Type} : {κs Γ : List (Kind L mem)} → (x : HList f (κs ++ Γ)) →
    (HList.take (κs := κs) x).append (HList.drop (κs := κs) x) = x
  | [], _, _ => rfl
  | _ :: κs, Γ, x => by
    show (x.1, (HList.take (κs := κs) x.2).append (HList.drop (κs := κs) x.2)) = x
    rw [HList.take_append_drop (κs := κs) (Γ := Γ) x.2]
    rfl

/-- When a shaped tick input's lag (`shapeLag`, the `max` over its leaves) is `0`, every leaf is
at lag `0`, so the two environments read the same shaped increment. -/
theorem readShapeDelta_agree0 {Γ : Ctx L mem} {l : Lags Γ} {Δ Δ' : HList (Kind.delta (mem := mem)) Γ}
    (h : DAgree0 l Δ Δ') (ℓ : L) : (sh : TickShape) → (vs : VarsOf Γ ℓ sh) → shapeLag l sh vs = 0 →
    readShapeDelta Δ ℓ sh vs = readShapeDelta Δ' ℓ sh vs
  | .sing _, v, h0 => by show HList.cons (Δ.get v) HList.nil = HList.cons (Δ'.get v) HList.nil; rw [h.get v h0]
  | .stream _ _ _ _, v, h0 => by show HList.cons (Δ.get v) HList.nil = HList.cons (Δ'.get v) HList.nil; rw [h.get v h0]
  | .ksing _ _ _ _, v, h0 => by show HList.cons (Δ.get v) HList.nil = HList.cons (Δ'.get v) HList.nil; rw [h.get v h0]
  | .kstream _ _ _ _ _ _, v, h0 => by show HList.cons (Δ.get v) HList.nil = HList.cons (Δ'.get v) HList.nil; rw [h.get v h0]
  | .pair a b, (va, vb), h0 => by
    have h0' := Nat.max_eq_zero_iff.mp h0
    show (readShapeDelta Δ ℓ a va).append (readShapeDelta Δ ℓ b vb) = (readShapeDelta Δ' ℓ a va).append (readShapeDelta Δ' ℓ b vb)
    rw [readShapeDelta_agree0 h ℓ a va h0'.1, readShapeDelta_agree0 h ℓ b vb h0'.2]

/-- A channel cell's delivery is a function of the cell alone: this step's sends are enqueued
after the release and do not appear in the output. -/
theorem ChanCell.stepG_snd {α : Type} (tick : Bool) (c : Nat) (sends : Array α) (cell : ChanCell α) :
    (cell.stepG tick c sends).2 = if tick then cell.pending.extract 0 (c - cell.delivered) else #[] := by
  unfold ChanCell.stepG ChanCell.step ChanCell.enqueue
  split <;> rfl

section Agree
variable (ctx : StepCtx L mem)

mutual

/-- **One node's step respects the lag analysis**: increments agreeing at `l`'s lag-`0` positions
(`h`) give outputs agreeing at `Node.lagOf n l`'s lag-`0` positions. Immediate operators rewrite
by `h.get`; strict nodes are `ChanCell.stepG_snd` or read nothing from `Δ`; `fixN` splits on
whether every external is at lag `0` (then `Δ = Δ'`) or some is not (then the knot outputs are
above `0` and the exposed outputs follow from the body with the knots at lag `1`). -/
theorem Node.step_agree0 : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) → (sch : Node.Sched n) →
    (l : Lags Γ) → (Δ Δ' : HList (Kind.delta (mem := mem)) Γ) → (s : Node.State n) → DAgree0 l Δ Δ' →
    DAgree0 (Node.lagOf n l) (Node.step ctx n sch Δ s).2 (Node.step ctx n sch Δ' s).2
  | _, _, .map _ _ x _, _, l, Δ, Δ', s, h => ⟨fun h0 => by simp only [Node.step]; rw [h.get x h0], trivial⟩
  | _, _, .filterMap _ _ x _, _, l, Δ, Δ', s, h => ⟨fun h0 => by simp only [Node.step]; rw [h.get x h0], trivial⟩
  | _, _, .broadcast_closed _ x, d, l, Δ, Δ', s, h => ⟨fun _ => by
      simp only [Node.step, ChanCell.stepG_snd], trivial⟩
  | _, _, .demux _ x addr, d, l, Δ, Δ', s, h => ⟨fun _ => by
      simp only [Node.step, ChanCell.stepG_snd], trivial⟩
  | _, _, .values _ _ x, _, l, Δ, Δ', s, h => ⟨fun h0 => by simp only [Node.step]; rw [h.get x h0], trivial⟩
  | _, _, .into_keyed _ _ x, _, l, Δ, Δ', s, h => ⟨fun h0 => by simp only [Node.step]; rw [h.get x h0], trivial⟩
  | _, _, .kassume_ordering _ _ x _, _, l, Δ, Δ', s, h => ⟨fun h0 => by simp only [Node.step]; rw [h.get x h0], trivial⟩
  | _, _, .kfold_early_stop _ _ _ x _ _, _, l, Δ, Δ', s, h => ⟨fun h0 => by simp only [Node.step]; rw [h.get x h0], trivial⟩
  | _, _, .kget_max_key _ _ _ _ x, _, l, Δ, Δ', s, h => ⟨fun h0 => by simp only [Node.step]; rw [h.get x h0], trivial⟩
  | _, _, .weaken_retries _ x, _, l, Δ, Δ', s, h => ⟨fun h0 => by simp only [Node.step]; rw [h.get x h0], trivial⟩
  | _, _, .union _ a b, _, l, Δ, Δ', s, h => ⟨fun h0 => by
      have h0' := Nat.max_eq_zero_iff.mp h0
      simp only [Node.step]; rw [h.get a h0'.1, h.get b h0'.2], trivial⟩
  | _, _, .assume_ordering _ x _, _, l, Δ, Δ', s, h => ⟨fun h0 => by simp only [Node.step]; rw [h.get x h0], trivial⟩
  | _, _, .fold _ _ _ _ _ _ x, _, l, Δ, Δ', s, h => ⟨fun h0 => by simp only [Node.step]; rw [h.get x h0], trivial⟩
  | _, _, .fold_monotone _ _ _ _ _ _ x, _, l, Δ, Δ', s, h => ⟨fun h0 => by simp only [Node.step]; rw [h.get x h0], trivial⟩
  | _, _, .snapshot _ x _, _, l, Δ, Δ', s, h => ⟨fun h0 => by simp only [Node.step]; rw [h.get x h0], trivial⟩
  | _, _, .batch _ _ _ x _, _, l, Δ, Δ', s, h => ⟨fun h0 => by simp only [Node.step]; rw [h.get x h0], trivial⟩
  | _, _, .source_interval _ _, _, _, _, _, _, _ => ⟨fun _ => rfl, trivial⟩
  | _, _, .firstTick _ _ x, _, l, Δ, Δ', s, h => ⟨fun h0 => by simp only [Node.step]; rw [h.get x h0], trivial⟩
  | _, _, .mapTick x _, _, l, Δ, Δ', s, h => ⟨fun h0 => by simp only [Node.step]; rw [h.get x h0], trivial⟩
  | _, _, .zipTick a b, _, l, Δ, Δ', s, h => ⟨fun h0 => by
      have h0' := Nat.max_eq_zero_iff.mp h0
      simp only [Node.step]; rw [h.get a h0'.1, h.get b h0'.2], trivial⟩
  | _, _, .defer_tick _ x, _, l, Δ, Δ', s, h => ⟨fun _ => by simp only [Node.step], trivial⟩
  | _, _, .allTicks _ x, _, l, Δ, Δ', s, h => ⟨fun h0 => by simp only [Node.step]; rw [h.get x h0], trivial⟩
  | _, _, .flattenOrdered _ x, _, l, Δ, Δ', s, h => ⟨fun h0 => by simp only [Node.step]; rw [h.get x h0], trivial⟩
  | _, _, .flattenUnordered _ x, _, l, Δ, Δ', s, h => ⟨fun h0 => by simp only [Node.step]; rw [h.get x h0], trivial⟩
  | _, _, .tinto_keyed _ _ x, _, l, Δ, Δ', s, h => ⟨fun h0 => by simp only [Node.step]; rw [h.get x h0], trivial⟩
  | _, _, .tassume_ordering _ x _, _, l, Δ, Δ', s, h => ⟨fun h0 => by simp only [Node.step]; rw [h.get x h0], trivial⟩
  | _, _, .reduce_watermark _ _ _ _ x wm _ _ _ _, _, l, Δ, Δ', s, h => ⟨fun h0 => by
      have h0' := Nat.max_eq_zero_iff.mp h0
      simp only [Node.step]; rw [h.get x h0'.1, h.get wm h0'.2], trivial⟩
  | _, _, .kentries _ _ x, _, l, Δ, Δ', s, h => ⟨fun h0 => by simp only [Node.step]; rw [h.get x h0], trivial⟩
  | _, _, .kmap _ _ _ x _, _, l, Δ, Δ', s, h => ⟨fun h0 => by simp only [Node.step]; rw [h.get x h0], trivial⟩
  | _, _, .kallTicks _ _ x, _, l, Δ, Δ', s, h => ⟨fun h0 => by simp only [Node.step]; rw [h.get x h0], trivial⟩
  | _, _, .kfold _ _ _ _ _ _ _ _ _ x, _, l, Δ, Δ', s, h => ⟨fun h0 => by simp only [Node.step]; rw [h.get x h0], trivial⟩
  | _, _, .ksnapshot _ _ _ x _, _, l, Δ, Δ', s, h => ⟨fun h0 => by simp only [Node.step]; rw [h.get x h0], trivial⟩
  | _, _, .tfold _ _ _ _ _ _ x, _, l, Δ, Δ', s, h => ⟨fun h0 => by simp only [Node.step]; rw [h.get x h0], trivial⟩
  | _, _, .latest _ x, _, l, Δ, Δ', s, h => ⟨fun h0 => by simp only [Node.step]; rw [h.get x h0], trivial⟩
  | _, _, .latest_monotone x, _, l, Δ, Δ', s, h => ⟨fun h0 => by simp only [Node.step]; rw [h.get x h0], trivial⟩
  | _, _, .time, _, _, _, _, _, _ => ⟨fun _ => rfl, trivial⟩
  | _, _, .snapshot_bounded _ _, _, _, _, _, _, _ => ⟨fun _ => rfl, trivial⟩
  | _, _, .tick_scan (ℓ := ℓ) sts ins outs x g seed, _, l, Δ, Δ', s, h =>
    DAgree0.const_of _ (fun h0 => by simp only [Node.step]; rw [readShapeDelta_agree0 h ℓ ins x h0])
  | _, _, .call N args, sch, l, Δ, Δ', s, h => Dag.step_agree0 N sch _ _ _ s (h.mapSubst args)
  | _, _, .fixN ks body, sch, l, Δ, Δ', s, h => by
    show DAgree0 ((Lags.const _ (Lags.maxAll l)).append (HList.drop (Dag.lagOf body ((Lags.const _ (Lags.maxAll l)).append l))))
      (Dag.step ctx body sch ((knotIter (fun X => (Dag.step ctx body sch (X.append Δ) s).2.take) ks.length (knotFill ks)).append Δ) s).2
      (Dag.step ctx body sch ((knotIter (fun X => (Dag.step ctx body sch (X.append Δ') s).2.take) ks.length (knotFill ks)).append Δ') s).2
    rcases Nat.eq_zero_or_pos (Lags.maxAll l) with h0 | hpos
    · -- every external is at lag `0`, so the two environments are equal
      have hΔ : Δ = Δ' := DAgree0.eq_of_zero (Lags.le_const_of_maxAll l (Nat.le_of_eq h0)) h
      subst hΔ
      exact DAgree0.refl _ _
    · -- the knot outputs sit above lag `0`; the exposed outputs come from the body with the
      -- knots at lag `1`, whose lags bound `Dag.lagOf` at the actual knot lags
      generalize knotIter (fun X => (Dag.step ctx body sch (X.append Δ) s).2.take) ks.length (knotFill ks) = KΔ
      generalize knotIter (fun X => (Dag.step ctx body sch (X.append Δ') s).2.take) ks.length (knotFill ks) = KΔ'
      have ih := Dag.step_agree0 body sch ((Lags.const (ks.map KnotKind.toKind) 1).append l)
        (KΔ.append Δ) (KΔ'.append Δ') s
        (DAgree0.append (DAgree0.const_of _ (fun h0 => absurd h0 Nat.one_ne_zero)) h)
      have ih' := ih.mono_lag (Dag.lagOf_mono body (Lags.Le.append (Lags.Le.const hpos) (Lags.Le.refl l)))
      rw [← HList.take_append_drop (κs := ks.map KnotKind.toKind) (Dag.step ctx body sch (KΔ.append Δ) s).2,
        ← HList.take_append_drop (κs := ks.map KnotKind.toKind) (Dag.step ctx body sch (KΔ'.append Δ') s).2]
      exact DAgree0.append (DAgree0.const_of _ (fun h0 => absurd h0 (Nat.pos_iff_ne_zero.mp hpos))) (DAgree0.drop ih')

/-- `Node.step_agree0` along the `letN` chain. -/
theorem Dag.step_agree0 : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) → (sch : Dag.Sched d) →
    (l : Lags Γ) → (Δ Δ' : HList (Kind.delta (mem := mem)) Γ) → (s : Dag.State d) → DAgree0 l Δ Δ' →
    DAgree0 (Dag.lagOf d l) (Dag.step ctx d sch Δ s).2 (Dag.step ctx d sch Δ' s).2
  | _, _, .ret xs, _, l, Δ, Δ', s, h => h.mapSubst xs
  | _, _, .letN n rest, sch, l, Δ, Δ', s, h =>
    Dag.step_agree0 rest sch.2 _ _ _ s.2 ((Node.step_agree0 n sch.1 l Δ Δ' s.1 h).append h)

end

/-- **The Jacobi iteration of a knot body's step tracks the lag analysis.** `hK` says `K`
is a fixpoint of the body's step on the knot increments; after `i` rounds from increments `X`
agreeing with `K` at `l`'s lag-`0` positions, the knot increments agree with `K` wherever
`Dag.knotLag` iterated `i` times from `l` says `0`. Each round is `Dag.step_agree0` with the
externals at lag `0`. -/
theorem Dag.step_knotIter_agree0 {Γ : Ctx L mem} (ks : List (KnotKind L mem)) {outs : List (Kind L mem)}
    (body : Dag (ks.map KnotKind.toKind ++ Γ) (ks.map KnotKind.toKind ++ outs)) (sch : Dag.Sched body)
    (Δ : HList (Kind.delta (mem := mem)) Γ) (s : Dag.State body)
    (K : HList (Kind.delta (mem := mem)) (ks.map KnotKind.toKind))
    (hK : (Dag.step ctx body sch (K.append Δ) s).2.take = K) :
    ∀ (i : Nat) (l : Lags (ks.map KnotKind.toKind)) (X : HList (Kind.delta (mem := mem)) (ks.map KnotKind.toKind)),
      DAgree0 l X K →
      DAgree0 (knotIter (Dag.knotLag ks body (Lags.const Γ 0)) i l)
        (knotIter (fun X => (Dag.step ctx body sch (X.append Δ) s).2.take) i X) K
  | 0, _, _, h => h
  | i + 1, l, X, h => by
    show DAgree0 (knotIter (Dag.knotLag ks body (Lags.const Γ 0)) i (Dag.knotLag ks body (Lags.const Γ 0) l))
      (knotIter (fun X => (Dag.step ctx body sch (X.append Δ) s).2.take) i (Dag.step ctx body sch (X.append Δ) s).2.take) K
    refine Dag.step_knotIter_agree0 ks body sch Δ s K hK i _ _ ?_
    have h1 := (Dag.step_agree0 ctx body sch (l.append (Lags.const Γ 0)) (X.append Δ) (K.append Δ) s
      (DAgree0.append h (DAgree0.const_of _ (fun _ => rfl)))).take
    rw [hK] at h1
    exact h1

/-- **The lock-step machine's `fixN` step equals the body's step at any fixpoint of the knot
increments.** For a
valid body (`hv`), `iter_zero` brings the lag iteration from all-`1` to all-`0` in `ks.length`
rounds, so by `Dag.step_knotIter_agree0` the machine's `knotIter` from empty increments equals
the fixpoint `K` of `hK`. -/
theorem Node.step_fixN_eq {Γ : Ctx L mem} (ks : List (KnotKind L mem)) {outs : List (Kind L mem)}
    (body : Dag (ks.map KnotKind.toKind ++ Γ) (ks.map KnotKind.toKind ++ outs)) (sch : Node.Sched (.fixN ks body))
    (hv : Node.valid (.fixN ks body) = true) (Δ : HList (Kind.delta (mem := mem)) Γ) (s : Node.State (.fixN ks body))
    (K : HList (Kind.delta (mem := mem)) (ks.map KnotKind.toKind))
    (hK : (Dag.step ctx body sch (K.append Δ) s).2.take = K) :
    Node.step ctx (.fixN ks body) sch Δ s = Dag.step ctx body sch (K.append Δ) s := by
  show Dag.step ctx body sch
    ((knotIter (fun X => (Dag.step ctx body sch (X.append Δ) s).2.take) ks.length (knotFill ks)).append Δ) s = _
  have h := Dag.step_knotIter_agree0 ctx ks body sch Δ s K hK ks.length (Lags.const _ 1) (knotFill ks)
    (DAgree0.const_of _ (fun h0 => absurd h0 Nat.one_ne_zero))
  have hz := iter_zero (ks.map KnotKind.toKind) (Dag.knotLag ks body (Lags.const Γ 0))
    (Dag.knotLag_laws ks body (Lags.const Γ 0)).self (Node.valid_fixN ks body hv).1
  rw [List.length_map] at hz
  rw [hz] at h
  rw [DAgree0.eq_of_zero (Lags.Le.refl _) h]

end Agree

end Project

end Hydro
