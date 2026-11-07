import Hydro.Graph.Lags
import Hydro.SchedMono
import Hydro.SchedPaced

/-!
# Hydro · Graph — causality and tick-monotonicity of every graph

Two facts the stateful interpreter's correctness stands on, proven once
for every `Dag` by structural induction (the per-operator pieces are
`SchedCausal.lean`'s `causal_*` and `SchedMono.lean`'s `tmono_*`; the
latter are also what the per-module `_smono` phase dispatches):

- **causality** (`Dag.denote_causal`): a graph's outputs at steps `≤ h`
  are a function of its inputs at steps `≤ h`;
- **graded causality** (`Dag.denote_agreeL`, D95): the same with a *lag*
  per wire — inputs agreeing at the steps `t + lag ≤ h` give outputs
  agreeing at the steps `t + lag' ≤ h`, where `lag'` is the IR-level lag
  arithmetic `Dag.lagOf` (immediate ops inherit or take the `max`, a
  network hop and `defer_tick` take one off, sources are `0`, a knot's
  wires sit at the `max` of its externals' lags). A graph is **valid**
  (`Dag.valid`) when every knot body is **cascade-valid**: every knot, the
  earlier knots of its block closed, is strict in its own wire — every
  cycle among the knots passes a network hop or a `defer_tick` (Rust's
  "acyclic through strata"; `cascadeValid`, `Graph/Lags.lean`) —
  which is what makes the ungated Kleene diagonal each knot's fixpoint
  (`fix_stream_eq`/`fix_tick_eq`) and what the production interpreter's
  sweeps compute (`Project/Sweep.lean`);
- **tick-trace monotonicity** (`Dag.denote_tmono`) and **single-slot
  pacing** (`Dag.denote_tpaced`): every atomic output is step-monotone and
  gains at most one entry per step — for valid graphs (the tick knot's
  diagonal is monotone/paced because its body is strict).
-/

namespace Hydro

namespace Project

variable {L : Type} {mem : L → Nat}

/-! ## Agreement below a horizon, per kind -/

/-- Two machine wires of a kind agree at all views/entries/values up to
`h` (`SchedCausal`'s `SAgree`/`FAgree`/`TAgree`). -/
def Kind.agree (h : Nat) : (κ : Kind L mem) → κ.sched → κ.sched → Prop
  | .stream _ _ _ _ _, x, y => SAgree h x y
  | .keyed _ _ _ _ _ _ _, x, y => SAgree h x y
  | .sing _ _ _ _ _ _ _, x, y => FAgree h x y
  | .ksing _ _ _ _ _ _ _ _ _ _, x, y => FAgree h x y
  | .atomic _ _, x, y => TAgree h x y
  | .atomicStream _ _ _ _ _, x, y => TAgree h x y
  | .atomicKeyed _ _ _ _ _ _ _, x, y => TAgree h x y
  | .atomicKSing _ _ _ _ _, x, y => TAgree h x y

/-- Pointwise agreement of two environments. -/
def HAgree (h : Nat) : {κs : List (Kind L mem)} → HList (Kind.sched (mem := mem)) κs →
    HList (Kind.sched (mem := mem)) κs → Prop
  | [], _, _ => True
  | κ :: _, x, y => Kind.agree h κ x.1 y.1 ∧ HAgree h x.2 y.2

theorem Kind.agree.refl (h : Nat) (κ : Kind L mem) (x : κ.sched) : Kind.agree h κ x x := by
  cases κ <;> first | exact SAgree.refl h x | exact FAgree.refl h x | exact TAgree.refl h x | skip

theorem HAgree.refl (h : Nat) : {κs : List (Kind L mem)} → (x : HList (Kind.sched (mem := mem)) κs) →
    HAgree h x x
  | [], _ => trivial
  | _ :: _, x => ⟨Kind.agree.refl h _ x.1, HAgree.refl h x.2⟩

theorem HAgree.get {h : Nat} : {Γ : Ctx L mem} → {κ : Kind L mem} →
    {x y : HList (Kind.sched (mem := mem)) Γ} → HAgree h x y → (v : Var Γ κ) →
    Kind.agree h κ (x.get v) (y.get v)
  | _ :: _, _, _, _, hxy, .zero => hxy.1
  | _ :: _, _, _, _, hxy, .succ v => HAgree.get hxy.2 v

theorem HAgree.append {h : Nat} : {κs Γ : List (Kind L mem)} →
    {x y : HList (Kind.sched (mem := mem)) κs} → {x' y' : HList (Kind.sched (mem := mem)) Γ} →
    HAgree h x y → HAgree h x' y' → HAgree h (x.append x') (y.append y')
  | [], _, _, _, _, _, _, h' => h'
  | _ :: _, _, _, _, _, _, hxy, h' => ⟨hxy.1, HAgree.append hxy.2 h'⟩

theorem HAgree.take {h : Nat} : {κs Γ : List (Kind L mem)} →
    {x y : HList (Kind.sched (mem := mem)) (κs ++ Γ)} → HAgree h x y →
    HAgree h (HList.take (κs := κs) x) (HList.take (κs := κs) y)
  | [], _, _, _, _ => trivial
  | _ :: _, _, _, _, hxy => ⟨hxy.1, HAgree.take hxy.2⟩

theorem HAgree.drop {h : Nat} : {κs Γ : List (Kind L mem)} →
    {x y : HList (Kind.sched (mem := mem)) (κs ++ Γ)} → HAgree h x y →
    HAgree h (HList.drop (κs := κs) x) (HList.drop (κs := κs) y)
  | [], _, _, _, hxy => hxy
  | _ :: κs, Γ, _, _, hxy => HAgree.drop (κs := κs) (Γ := Γ) hxy.2

theorem HAgree.head {h : Nat} {κ : Kind L mem} {κs : List (Kind L mem)}
    {x y : HList (Kind.sched (mem := mem)) (κ :: κs)} (hxy : HAgree h x y) :
    Kind.agree h κ x.head y.head := hxy.1

theorem HAgree.tail {h : Nat} {κ : Kind L mem} {κs : List (Kind L mem)}
    {x y : HList (Kind.sched (mem := mem)) (κ :: κs)} (hxy : HAgree h x y) :
    HAgree h x.tail y.tail := hxy.2

theorem HAgree.mapSubst {h : Nat} {Γ Γ' : Ctx L mem} {x y : Env Γ} (hxy : HAgree h x y)
    (args : Subst Γ' Γ) :
    HAgree h (args.map (fun v => x.get v)) (args.map (fun v => y.get v)) := by
  induction Γ' with
  | nil => trivial
  | cons κ κs ih => exact ⟨hxy.get args.1, ih args.2⟩

/-- Shaped inputs: agreement leafwise. -/
theorem readShape_agree {h : Nat} {Γ : Ctx L mem} {x y : Env Γ} (hxy : HAgree h x y) (ℓ : L) :
    (sh : TickShape) → (vs : VarsOf Γ ℓ sh) →
    TAgreeOf h sh (readShape x ℓ sh vs) (readShape y ℓ sh vs)
  | .sing _, v => hxy.get v
  | .stream _ _ _ _, v => hxy.get v
  | .ksing _ _ _ _, v => hxy.get v
  | .kstream _ _ _ _ _ _, v => hxy.get v
  | .pair a b, (va, vb) => ⟨readShape_agree hxy ℓ a va, readShape_agree hxy ℓ b vb⟩

theorem shapeToList_agree {h : Nat} (ℓ : L) : (sh : TickShape) →
    (x y : AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) (SchedTick.TKStr mem) ℓ sh) →
    TAgreeOf h sh x y → HAgree h (shapeToList ℓ sh x) (shapeToList ℓ sh y)
  | .sing _, _, _, hxy => ⟨hxy, trivial⟩
  | .stream _ _ _ _, _, _, hxy => ⟨hxy, trivial⟩
  | .ksing _ _ _ _, _, _, hxy => ⟨hxy, trivial⟩
  | .kstream _ _ _ _ _ _, _, _, hxy => ⟨hxy, trivial⟩
  | .pair a b, x, y, hxy =>
    HAgree.append (shapeToList_agree ℓ a x.1 y.1 hxy.1) (shapeToList_agree ℓ b x.2 y.2 hxy.2)

/-! ## Agreement is downward closed -/

theorem Kind.agree.mono {h h' : Nat} (hh : h' ≤ h) {κ : Kind L mem} {x y : κ.sched}
    (hxy : Kind.agree h κ x y) : Kind.agree h' κ x y := by
  cases κ <;> first | exact SAgree.mono hh hxy | exact FAgree.mono hh hxy | exact TAgree.mono hh hxy

theorem HAgree.mono {h h' : Nat} (hh : h' ≤ h) : {κs : List (Kind L mem)} →
    {x y : HList (Kind.sched (mem := mem)) κs} → HAgree h x y → HAgree h' x y
  | [], _, _, _ => trivial
  | _ :: _, _, _, hxy => ⟨hxy.1.mono hh, HAgree.mono hh hxy.2⟩


/-! ## The IR lag analysis

`Dag.lagOf d lags`: the lag of every output of `d` from the lags of its
inputs (the corner's lag arithmetic, `Couple.lean`, here on the graph).
`Dag.valid d`: every knot body — at every nesting — is cascade-valid
(`cascadeValid` of its lag functional `Dag.knotLag`): every cycle among
its knots passes a network hop or a `defer_tick`. Both are structural recursions and
evaluate by `whnf`/`decide` on a concrete graph whose data payloads are
free variables (the payloads are never inspected). -/

/-- The lag of a shaped tick input: the maximum over its leaves. -/
def shapeLag {Γ : Ctx L mem} {ℓ : L} (l : Lags Γ) : (sh : TickShape) → VarsOf Γ ℓ sh → Nat
  | .sing _, v => l.get v
  | .stream _ _ _ _, v => l.get v
  | .ksing _ _ _ _, v => l.get v
  | .kstream _ _ _ _ _ _, v => l.get v
  | .pair a b, (va, vb) => max (shapeLag l a va) (shapeLag l b vb)

mutual

/-- The lags of a node's outputs from the lags of its context. -/
def Node.lagOf : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → Node Γ κs → Lags Γ → Lags κs
  | _, _, .map _ _ x _, l => HList.cons (l.get x) HList.nil
  | _, _, .filterMap _ _ x _, l => HList.cons (l.get x) HList.nil
  | _, _, .broadcast_closed _ x, l => HList.cons (l.get x - 1) HList.nil
  | _, _, .demux _ x _, l => HList.cons (l.get x - 1) HList.nil
  | _, _, .values _ _ x, l => HList.cons (l.get x) HList.nil
  | _, _, .into_keyed _ _ x, l => HList.cons (l.get x) HList.nil
  | _, _, .kassume_ordering _ _ x _, l => HList.cons (l.get x) HList.nil
  | _, _, .kfold_early_stop _ _ _ x _ _, l => HList.cons (l.get x) HList.nil
  | _, _, .kget_max_key _ _ _ _ x, l => HList.cons (l.get x) HList.nil
  | _, _, .weaken_retries _ x, l => HList.cons (l.get x) HList.nil
  | _, _, .union _ a b, l => HList.cons (max (l.get a) (l.get b)) HList.nil
  | _, _, .assume_ordering _ x _, l => HList.cons (l.get x) HList.nil
  | _, _, .fold _ _ _ _ _ _ x, l => HList.cons (l.get x) HList.nil
  | _, _, .fold_monotone _ _ _ _ _ _ x, l => HList.cons (l.get x) HList.nil
  | _, _, .snapshot _ x _, l => HList.cons (l.get x) HList.nil
  | _, _, .batch _ _ _ x _, l => HList.cons (l.get x) HList.nil
  | _, _, .source_interval _ _, _ => HList.cons 0 HList.nil
  | _, _, .firstTick _ _ x, l => HList.cons (l.get x) HList.nil
  | _, _, .mapTick x _, l => HList.cons (l.get x) HList.nil
  | _, _, .zipTick a b, l => HList.cons (max (l.get a) (l.get b)) HList.nil
  | _, _, .defer_tick _ x, l => HList.cons (l.get x - 1) HList.nil
  | _, _, .allTicks _ x, l => HList.cons (l.get x) HList.nil
  | _, _, .flattenOrdered _ x, l => HList.cons (l.get x) HList.nil
  | _, _, .flattenUnordered _ x, l => HList.cons (l.get x) HList.nil
  | _, _, .tinto_keyed _ _ x, l => HList.cons (l.get x) HList.nil
  | _, _, .tassume_ordering _ x _, l => HList.cons (l.get x) HList.nil
  | _, _, .reduce_watermark _ _ _ _ x wm _ _ _ _, l => HList.cons (max (l.get x) (l.get wm)) HList.nil
  | _, _, .kentries _ _ x, l => HList.cons (l.get x) HList.nil
  | _, _, .kmap _ _ _ x _, l => HList.cons (l.get x) HList.nil
  | _, _, .kallTicks _ _ x, l => HList.cons (l.get x) HList.nil
  | _, _, .kfold _ _ _ _ _ _ _ _ _ x, l => HList.cons (l.get x) HList.nil
  | _, _, .ksnapshot _ _ _ x _, l => HList.cons (l.get x) HList.nil
  | _, _, .tfold _ _ _ _ _ _ x, l => HList.cons (l.get x) HList.nil
  | _, _, .latest _ x, l => HList.cons (l.get x) HList.nil
  | _, _, .latest_monotone x, l => HList.cons (l.get x) HList.nil
  | _, _, .time, _ => HList.cons 0 HList.nil
  | _, _, .snapshot_bounded _ _, _ => HList.cons 0 HList.nil
  | _, _, .tick_scan (ℓ := ℓ) _ ins outs x _ _, l => Lags.const (shapeKinds ℓ outs) (shapeLag l ins x)
  | _, _, .call N args, l => Dag.lagOf N (args.map (fun v => l.get v))
  | _, _, .fixN ks body, l =>
    let Lg := Lags.maxAll l
    (Lags.const (ks.map KnotKind.toKind) Lg).append
      (HList.drop (κs := ks.map KnotKind.toKind) (Dag.lagOf body ((Lags.const (ks.map KnotKind.toKind) Lg).append l)))

/-- The lags of a graph's outputs from the lags of its inputs. -/
def Dag.lagOf : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → Dag Γ outs → Lags Γ → Lags outs
  | _, _, .ret xs, l => xs.map (fun v => l.get v)
  | _, _, .letN n rest, l => Dag.lagOf rest ((Node.lagOf n l).append l)

end

/-- The lags a knot body's analysis starts from: its knot wires at `1`, its
externals at `0`. -/
def knotProbe (ks : List (KnotKind L mem)) (Γ : Ctx L mem) : Lags (ks.map KnotKind.toKind ++ Γ) :=
  (Lags.const (ks.map KnotKind.toKind) 1).append (Lags.const Γ 0)

/-- **The knot body's lag functional** (D95): its knot-output lags as a
function of its knot-input lags, the externals at the lags `l`. -/
def Dag.knotLag {Γ : Ctx L mem} (ks : List (KnotKind L mem)) {outs : List (Kind L mem)}
    (body : Dag (ks.map KnotKind.toKind ++ Γ) (ks.map KnotKind.toKind ++ outs)) (l : Lags Γ) :
    Lags (ks.map KnotKind.toKind) → Lags (ks.map KnotKind.toKind) :=
  fun m => HList.take (κs := ks.map KnotKind.toKind) (Dag.lagOf body (m.append l))

mutual

/-- Every knot body of the node is **cascade-valid** (D95): every knot,
with the earlier knots of its block closed, is strict in its own wire —
every cycle among the knots passes a network hop or a `defer_tick`
(`cascadeValid`, `Graph/Lags.lean`). -/
def Node.valid : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → Node Γ κs → Bool
  | _, _, .call N _ => Dag.valid N
  | Γ, _, .fixN ks body =>
    cascadeValid (ks.map KnotKind.toKind) (Dag.knotLag ks body (Lags.const Γ 0)) && Dag.valid body
  | _, _, _ => true

/-- Every knot body of the graph is cascade-valid. -/
def Dag.valid : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → Dag Γ outs → Bool
  | _, _, .ret _ => true
  | _, _, .letN n rest => Node.valid n && Dag.valid rest

end

theorem Node.valid_call {Γ Γ' : Ctx L mem} {outs : List (Kind L mem)} (N : Dag Γ' outs) (args : Subst Γ' Γ) :
    Node.valid (.call N args) = Dag.valid N := rfl

theorem Node.valid_fixN {Γ : Ctx L mem} (ks : List (KnotKind L mem)) {outs : List (Kind L mem)}
    (body : Dag (ks.map KnotKind.toKind ++ Γ) (ks.map KnotKind.toKind ++ outs))
    (hv : Node.valid (.fixN ks body) = true) :
    cascadeValid (ks.map KnotKind.toKind) (Dag.knotLag ks body (Lags.const Γ 0)) = true ∧ Dag.valid body = true :=
  Bool.and_eq_true_iff.mp hv

theorem Dag.valid_letN {Γ : Ctx L mem} {κs outs : List (Kind L mem)} (n : Node Γ κs) (rest : Dag (κs ++ Γ) outs)
    (hv : Dag.valid (.letN n rest) = true) : Node.valid n = true ∧ Dag.valid rest = true :=
  Bool.and_eq_true_iff.mp hv

/-! ### The analysis, one node at a time (D95)

A concrete graph's lags and validity are never EVALUATED by the kernel
(or by `Meta`): the environment-passing recursion shares nothing under
call-by-name — every read of a wire's lag re-runs its producer's, and a
`brecOn` over the graph rebuilds its `below` towers per projection — and
`leader_election`'s `Dag.valid … = true` by `rfl` took 19 GB. The
generator computes the lags itself (`HydroGenLag.lean`) and proves them
by these lemmas, one node per step, every intermediate a literal: each
kernel check is one node's rule on literal inputs. -/

theorem Dag.lagOf_ret_lit {Γ outs : Ctx L mem} (xs : HList (Var Γ) outs) (l : Lags Γ) (r : Lags outs)
    (h : xs.map (fun v => l.get v) = r) : Dag.lagOf (.ret xs) l = r := h

theorem Dag.lagOf_letN_lit {Γ : Ctx L mem} {κs outs : List (Kind L mem)} (n : Node Γ κs) (rest : Dag (κs ++ Γ) outs)
    (l : Lags Γ) (ln : Lags κs) (l' : Lags (κs ++ Γ)) (r : Lags outs)
    (h1 : Node.lagOf n l = ln) (h0 : ln.append l = l') (h2 : Dag.lagOf rest l' = r) :
    Dag.lagOf (.letN n rest) l = r := by
  show Dag.lagOf rest ((Node.lagOf n l).append l) = r
  rw [h1, h0, h2]

theorem Node.lagOf_call_lit {Γ Γ' : Ctx L mem} {outs : List (Kind L mem)} (N : Dag Γ' outs) (args : Subst Γ' Γ)
    (l : Lags Γ) (l' : Lags Γ') (r : Lags outs)
    (hargs : args.map (fun v => l.get v) = l') (h : Dag.lagOf N l' = r) :
    Node.lagOf (.call N args) l = r := by
  show Dag.lagOf N (args.map (fun v => l.get v)) = r
  rw [hargs, h]

theorem Node.lagOf_fixN_lit {Γ : Ctx L mem} (ks : List (KnotKind L mem)) {outs : List (Kind L mem)}
    (body : Dag (ks.map KnotKind.toKind ++ Γ) (ks.map KnotKind.toKind ++ outs))
    (l : Lags Γ) (Lg : Nat) (l' : Lags (ks.map KnotKind.toKind ++ Γ)) (r : Lags (ks.map KnotKind.toKind ++ outs))
    (res : Lags (ks.map KnotKind.toKind ++ outs))
    (hLg : Lags.maxAll l = Lg) (henv : (Lags.const (ks.map KnotKind.toKind) Lg).append l = l')
    (hbody : Dag.lagOf body l' = r)
    (hres : (Lags.const (ks.map KnotKind.toKind) Lg).append (HList.drop (κs := ks.map KnotKind.toKind) r) = res) :
    Node.lagOf (.fixN ks body) l = res := by
  show (Lags.const (ks.map KnotKind.toKind) (Lags.maxAll l)).append
    (HList.drop (κs := ks.map KnotKind.toKind) (Dag.lagOf body ((Lags.const (ks.map KnotKind.toKind) (Lags.maxAll l)).append l))) = res
  rw [hLg, henv, hbody, hres]

theorem Dag.valid_ret_lit {Γ outs : Ctx L mem} (xs : HList (Var Γ) outs) : Dag.valid (.ret xs) = true := rfl

theorem Dag.valid_letN_lit {Γ : Ctx L mem} {κs outs : List (Kind L mem)} (n : Node Γ κs) (rest : Dag (κs ++ Γ) outs)
    (h1 : Node.valid n = true) (h2 : Dag.valid rest = true) : Dag.valid (.letN n rest) = true := by
  show (Node.valid n && Dag.valid rest) = true
  rw [h1, h2]; rfl

theorem Node.valid_call_lit {Γ Γ' : Ctx L mem} {outs : List (Kind L mem)} (N : Dag Γ' outs) (args : Subst Γ' Γ)
    (h : Dag.valid N = true) : Node.valid (.call N args) = true := h

theorem Node.valid_fixN_lit {Γ : Ctx L mem} (ks : List (KnotKind L mem)) {outs : List (Kind L mem)}
    (body : Dag (ks.map KnotKind.toKind ++ Γ) (ks.map KnotKind.toKind ++ outs))
    (hc : cascadeValid (ks.map KnotKind.toKind) (Dag.knotLag ks body (Lags.const Γ 0)) = true)
    (hv : Dag.valid body = true) :
    Node.valid (.fixN ks body) = true := by
  show (cascadeValid (ks.map KnotKind.toKind) (Dag.knotLag ks body (Lags.const Γ 0)) && Dag.valid body) = true
  rw [hc, hv]; rfl

/-- One evaluation of a knot body's lag functional on literals: the knot
lags `p` appended to the externals' (`x`), the body's lags at it (`y`, the
generator's `Dag.lagOf` chain), the knot outputs' (`r`). -/
theorem Dag.knotLag_lit {Γ : Ctx L mem} (ks : List (KnotKind L mem)) {outs : List (Kind L mem)}
    (body : Dag (ks.map KnotKind.toKind ++ Γ) (ks.map KnotKind.toKind ++ outs)) (l : Lags Γ)
    (p : Lags (ks.map KnotKind.toKind)) (x : Lags (ks.map KnotKind.toKind ++ Γ))
    (y : Lags (ks.map KnotKind.toKind ++ outs)) (r : Lags (ks.map KnotKind.toKind))
    (hx : p.append l = x) (hy : Dag.lagOf body x = y) (hr : HList.take (κs := ks.map KnotKind.toKind) y = r) :
    Dag.knotLag ks body l p = r := by
  show HList.take (κs := ks.map KnotKind.toKind) (Dag.lagOf body (p.append l)) = r
  rw [hx, hy, hr]

/-! ### The lag analysis is monotone and sub-additive -/

theorem shapeLag_mono {Γ : Ctx L mem} {ℓ : L} {l l' : Lags Γ} (h : Lags.Le l l') :
    (sh : TickShape) → (vs : VarsOf Γ ℓ sh) → shapeLag l sh vs ≤ shapeLag l' sh vs
  | .sing _, v => h.get v
  | .stream _ _ _ _, v => h.get v
  | .ksing _ _ _ _, v => h.get v
  | .kstream _ _ _ _ _ _, v => h.get v
  | .pair a b, (va, vb) => max_le_max (shapeLag_mono h a va) (shapeLag_mono h b vb)

theorem shapeLag_addC {Γ : Ctx L mem} {ℓ : L} (l : Lags Γ) (c : Nat) :
    (sh : TickShape) → (vs : VarsOf Γ ℓ sh) → shapeLag (Lags.addC l c) sh vs ≤ shapeLag l sh vs + c
  | .sing _, v => by rw [shapeLag, shapeLag, Lags.get_addC]
  | .stream _ _ _ _, v => by rw [shapeLag, shapeLag, Lags.get_addC]
  | .ksing _ _ _ _, v => by rw [shapeLag, shapeLag, Lags.get_addC]
  | .kstream _ _ _ _ _ _, v => by rw [shapeLag, shapeLag, Lags.get_addC]
  | .pair a b, (va, vb) => by
    show max (shapeLag (Lags.addC l c) a va) (shapeLag (Lags.addC l c) b vb) ≤ max (shapeLag l a va) (shapeLag l b vb) + c
    exact Nat.max_le.mpr ⟨(shapeLag_addC l c a va).trans (Nat.add_le_add_right (Nat.le_max_left _ _) c),
      (shapeLag_addC l c b vb).trans (Nat.add_le_add_right (Nat.le_max_right _ _) c)⟩

mutual

theorem Node.lagOf_mono : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) → {l l' : Lags Γ} →
    Lags.Le l l' → Lags.Le (Node.lagOf n l) (Node.lagOf n l')
  | _, _, .map _ _ x _, _, _, h => ⟨h.get x, trivial⟩
  | _, _, .filterMap _ _ x _, _, _, h => ⟨h.get x, trivial⟩
  | _, _, .broadcast_closed _ x, _, _, h => ⟨Nat.sub_le_sub_right (h.get x) 1, trivial⟩
  | _, _, .demux _ x _, _, _, h => ⟨Nat.sub_le_sub_right (h.get x) 1, trivial⟩
  | _, _, .values _ _ x, _, _, h => ⟨h.get x, trivial⟩
  | _, _, .into_keyed _ _ x, _, _, h => ⟨h.get x, trivial⟩
  | _, _, .kassume_ordering _ _ x _, _, _, h => ⟨h.get x, trivial⟩
  | _, _, .kfold_early_stop _ _ _ x _ _, _, _, h => ⟨h.get x, trivial⟩
  | _, _, .kget_max_key _ _ _ _ x, _, _, h => ⟨h.get x, trivial⟩
  | _, _, .weaken_retries _ x, _, _, h => ⟨h.get x, trivial⟩
  | _, _, .union _ a b, _, _, h => ⟨max_le_max (h.get a) (h.get b), trivial⟩
  | _, _, .assume_ordering _ x _, _, _, h => ⟨h.get x, trivial⟩
  | _, _, .fold _ _ _ _ _ _ x, _, _, h => ⟨h.get x, trivial⟩
  | _, _, .fold_monotone _ _ _ _ _ _ x, _, _, h => ⟨h.get x, trivial⟩
  | _, _, .snapshot _ x _, _, _, h => ⟨h.get x, trivial⟩
  | _, _, .batch _ _ _ x _, _, _, h => ⟨h.get x, trivial⟩
  | _, _, .source_interval _ _, _, _, _ => ⟨Nat.le_refl _, trivial⟩
  | _, _, .firstTick _ _ x, _, _, h => ⟨h.get x, trivial⟩
  | _, _, .mapTick x _, _, _, h => ⟨h.get x, trivial⟩
  | _, _, .zipTick a b, _, _, h => ⟨max_le_max (h.get a) (h.get b), trivial⟩
  | _, _, .defer_tick _ x, _, _, h => ⟨Nat.sub_le_sub_right (h.get x) 1, trivial⟩
  | _, _, .allTicks _ x, _, _, h => ⟨h.get x, trivial⟩
  | _, _, .flattenOrdered _ x, _, _, h => ⟨h.get x, trivial⟩
  | _, _, .flattenUnordered _ x, _, _, h => ⟨h.get x, trivial⟩
  | _, _, .tinto_keyed _ _ x, _, _, h => ⟨h.get x, trivial⟩
  | _, _, .tassume_ordering _ x _, _, _, h => ⟨h.get x, trivial⟩
  | _, _, .reduce_watermark _ _ _ _ x wm _ _ _ _, _, _, h => ⟨max_le_max (h.get x) (h.get wm), trivial⟩
  | _, _, .kentries _ _ x, _, _, h => ⟨h.get x, trivial⟩
  | _, _, .kmap _ _ _ x _, _, _, h => ⟨h.get x, trivial⟩
  | _, _, .kallTicks _ _ x, _, _, h => ⟨h.get x, trivial⟩
  | _, _, .kfold _ _ _ _ _ _ _ _ _ x, _, _, h => ⟨h.get x, trivial⟩
  | _, _, .ksnapshot _ _ _ x _, _, _, h => ⟨h.get x, trivial⟩
  | _, _, .tfold _ _ _ _ _ _ x, _, _, h => ⟨h.get x, trivial⟩
  | _, _, .latest _ x, _, _, h => ⟨h.get x, trivial⟩
  | _, _, .latest_monotone x, _, _, h => ⟨h.get x, trivial⟩
  | _, _, .time, _, _, _ => ⟨Nat.le_refl _, trivial⟩
  | _, _, .snapshot_bounded _ _, _, _, _ => ⟨Nat.le_refl _, trivial⟩
  | _, _, .tick_scan _ ins _ x _ _, _, _, h => Lags.Le.const (shapeLag_mono h ins x)
  | _, _, .call N args, _, _, h => Dag.lagOf_mono N (h.mapSubst args)
  | _, _, .fixN ks body, l, l', h => by
    have hm : Lags.maxAll l ≤ Lags.maxAll l' := Lags.maxAll_mono h
    exact Lags.Le.append (Lags.Le.const hm)
      (Lags.Le.drop (Dag.lagOf_mono body (Lags.Le.append (Lags.Le.const hm) h)))

theorem Dag.lagOf_mono : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) → {l l' : Lags Γ} →
    Lags.Le l l' → Lags.Le (Dag.lagOf d l) (Dag.lagOf d l')
  | _, _, .ret xs, _, _, h => h.mapSubst xs
  | _, _, .letN n rest, _, _, h => Dag.lagOf_mono rest (Lags.Le.append (Node.lagOf_mono n h) h)

end

mutual

theorem Node.lagOf_add_le : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) → (l : Lags Γ) → (c : Nat) →
    Lags.Le (Node.lagOf n (Lags.addC l c)) (Lags.addC (Node.lagOf n l) c)
  | _, _, .map _ _ x _, l, c => ⟨by show (Lags.addC l c).get x ≤ l.get x + c; rw [Lags.get_addC], trivial⟩
  | _, _, .filterMap _ _ x _, l, c => ⟨by show (Lags.addC l c).get x ≤ l.get x + c; rw [Lags.get_addC], trivial⟩
  | _, _, .broadcast_closed _ x, l, c => ⟨by show (Lags.addC l c).get x - 1 ≤ l.get x - 1 + c; rw [Lags.get_addC]; omega, trivial⟩
  | _, _, .demux _ x _, l, c => ⟨by show (Lags.addC l c).get x - 1 ≤ l.get x - 1 + c; rw [Lags.get_addC]; omega, trivial⟩
  | _, _, .values _ _ x, l, c => ⟨by show (Lags.addC l c).get x ≤ l.get x + c; rw [Lags.get_addC], trivial⟩
  | _, _, .into_keyed _ _ x, l, c => ⟨by show (Lags.addC l c).get x ≤ l.get x + c; rw [Lags.get_addC], trivial⟩
  | _, _, .kassume_ordering _ _ x _, l, c => ⟨by show (Lags.addC l c).get x ≤ l.get x + c; rw [Lags.get_addC], trivial⟩
  | _, _, .kfold_early_stop _ _ _ x _ _, l, c => ⟨by show (Lags.addC l c).get x ≤ l.get x + c; rw [Lags.get_addC], trivial⟩
  | _, _, .kget_max_key _ _ _ _ x, l, c => ⟨by show (Lags.addC l c).get x ≤ l.get x + c; rw [Lags.get_addC], trivial⟩
  | _, _, .weaken_retries _ x, l, c => ⟨by show (Lags.addC l c).get x ≤ l.get x + c; rw [Lags.get_addC], trivial⟩
  | _, _, .union _ a b, l, c => ⟨by
      show max ((Lags.addC l c).get a) ((Lags.addC l c).get b) ≤ max (l.get a) (l.get b) + c
      rw [Lags.get_addC, Lags.get_addC]; exact Lags.max_add_le _ _ c, trivial⟩
  | _, _, .assume_ordering _ x _, l, c => ⟨by show (Lags.addC l c).get x ≤ l.get x + c; rw [Lags.get_addC], trivial⟩
  | _, _, .fold _ _ _ _ _ _ x, l, c => ⟨by show (Lags.addC l c).get x ≤ l.get x + c; rw [Lags.get_addC], trivial⟩
  | _, _, .fold_monotone _ _ _ _ _ _ x, l, c => ⟨by show (Lags.addC l c).get x ≤ l.get x + c; rw [Lags.get_addC], trivial⟩
  | _, _, .snapshot _ x _, l, c => ⟨by show (Lags.addC l c).get x ≤ l.get x + c; rw [Lags.get_addC], trivial⟩
  | _, _, .batch _ _ _ x _, l, c => ⟨by show (Lags.addC l c).get x ≤ l.get x + c; rw [Lags.get_addC], trivial⟩
  | _, _, .source_interval _ _, _, _ => ⟨Nat.zero_le _, trivial⟩
  | _, _, .firstTick _ _ x, l, c => ⟨by show (Lags.addC l c).get x ≤ l.get x + c; rw [Lags.get_addC], trivial⟩
  | _, _, .mapTick x _, l, c => ⟨by show (Lags.addC l c).get x ≤ l.get x + c; rw [Lags.get_addC], trivial⟩
  | _, _, .zipTick a b, l, c => ⟨by
      show max ((Lags.addC l c).get a) ((Lags.addC l c).get b) ≤ max (l.get a) (l.get b) + c
      rw [Lags.get_addC, Lags.get_addC]; exact Lags.max_add_le _ _ c, trivial⟩
  | _, _, .defer_tick _ x, l, c => ⟨by show (Lags.addC l c).get x - 1 ≤ l.get x - 1 + c; rw [Lags.get_addC]; omega, trivial⟩
  | _, _, .allTicks _ x, l, c => ⟨by show (Lags.addC l c).get x ≤ l.get x + c; rw [Lags.get_addC], trivial⟩
  | _, _, .flattenOrdered _ x, l, c => ⟨by show (Lags.addC l c).get x ≤ l.get x + c; rw [Lags.get_addC], trivial⟩
  | _, _, .flattenUnordered _ x, l, c => ⟨by show (Lags.addC l c).get x ≤ l.get x + c; rw [Lags.get_addC], trivial⟩
  | _, _, .tinto_keyed _ _ x, l, c => ⟨by show (Lags.addC l c).get x ≤ l.get x + c; rw [Lags.get_addC], trivial⟩
  | _, _, .tassume_ordering _ x _, l, c => ⟨by show (Lags.addC l c).get x ≤ l.get x + c; rw [Lags.get_addC], trivial⟩
  | _, _, .reduce_watermark _ _ _ _ x wm _ _ _ _, l, c => ⟨by
      show max ((Lags.addC l c).get x) ((Lags.addC l c).get wm) ≤ max (l.get x) (l.get wm) + c
      rw [Lags.get_addC, Lags.get_addC]; exact Lags.max_add_le _ _ c, trivial⟩
  | _, _, .kentries _ _ x, l, c => ⟨by show (Lags.addC l c).get x ≤ l.get x + c; rw [Lags.get_addC], trivial⟩
  | _, _, .kmap _ _ _ x _, l, c => ⟨by show (Lags.addC l c).get x ≤ l.get x + c; rw [Lags.get_addC], trivial⟩
  | _, _, .kallTicks _ _ x, l, c => ⟨by show (Lags.addC l c).get x ≤ l.get x + c; rw [Lags.get_addC], trivial⟩
  | _, _, .kfold _ _ _ _ _ _ _ _ _ x, l, c => ⟨by show (Lags.addC l c).get x ≤ l.get x + c; rw [Lags.get_addC], trivial⟩
  | _, _, .ksnapshot _ _ _ x _, l, c => ⟨by show (Lags.addC l c).get x ≤ l.get x + c; rw [Lags.get_addC], trivial⟩
  | _, _, .tfold _ _ _ _ _ _ x, l, c => ⟨by show (Lags.addC l c).get x ≤ l.get x + c; rw [Lags.get_addC], trivial⟩
  | _, _, .latest _ x, l, c => ⟨by show (Lags.addC l c).get x ≤ l.get x + c; rw [Lags.get_addC], trivial⟩
  | _, _, .latest_monotone x, l, c => ⟨by show (Lags.addC l c).get x ≤ l.get x + c; rw [Lags.get_addC], trivial⟩
  | _, _, .time, _, _ => ⟨Nat.zero_le _, trivial⟩
  | _, _, .snapshot_bounded _ _, _, _ => ⟨Nat.zero_le _, trivial⟩
  | _, _, .tick_scan (ℓ := ℓ) _ ins outs x _ _, l, c => by
    show Lags.Le (Lags.const (shapeKinds ℓ outs) (shapeLag (Lags.addC l c) ins x))
      (Lags.addC (Lags.const (shapeKinds ℓ outs) (shapeLag l ins x)) c)
    rw [Lags.addC_const]
    exact Lags.Le.const (shapeLag_addC l c ins x)
  | _, _, .call N args, l, c => by
    show Lags.Le (Dag.lagOf N (args.map (fun v => (Lags.addC l c).get v))) _
    rw [Lags.mapSubst_addC]
    exact Dag.lagOf_add_le N _ c
  | _, _, .fixN ks body, l, c => by
    have hm : Lags.maxAll (Lags.addC l c) ≤ Lags.maxAll l + c := Lags.maxAll_addC_le l c
    show Lags.Le ((Lags.const _ (Lags.maxAll (Lags.addC l c))).append (HList.drop (Dag.lagOf body
        ((Lags.const _ (Lags.maxAll (Lags.addC l c))).append (Lags.addC l c)))))
      (Lags.addC ((Lags.const _ (Lags.maxAll l)).append (HList.drop (Dag.lagOf body
        ((Lags.const _ (Lags.maxAll l)).append l)))) c)
    rw [Lags.addC_append, Lags.addC_const, ← Lags.drop_addC]
    refine Lags.Le.append (Lags.Le.const hm) (Lags.Le.drop ?_)
    refine (Dag.lagOf_mono body (Lags.Le.append (Lags.Le.const hm) (Lags.Le.refl _))).trans ?_
    rw [← Lags.addC_const, ← Lags.addC_append]
    exact Dag.lagOf_add_le body _ c

theorem Dag.lagOf_add_le : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) → (l : Lags Γ) → (c : Nat) →
    Lags.Le (Dag.lagOf d (Lags.addC l c)) (Lags.addC (Dag.lagOf d l) c)
  | _, _, .ret xs, l, c => by
    show Lags.Le (xs.map (fun v => (Lags.addC l c).get v)) _
    rw [Lags.mapSubst_addC]
    exact Lags.Le.refl _
  | _, _, .letN n rest, l, c => by
    show Lags.Le (Dag.lagOf rest ((Node.lagOf n (Lags.addC l c)).append (Lags.addC l c))) _
    refine (Dag.lagOf_mono rest (Lags.Le.append (Node.lagOf_add_le n l c) (Lags.Le.refl _))).trans ?_
    rw [← Lags.addC_append]
    exact Dag.lagOf_add_le rest _ c

end

/-! ### The lag analysis distributes over the maximum and vanishes at `0` (D95)

Every output lag is a maximum over paths of (an input's lag, less the strict
nodes on the path), so the analysis is max-distributive and `0` at `0` — the
laws the knot cascade's lag algebra stands on (`LagLaws`,
`Graph/Lags.lean`). -/

theorem shapeLag_sup {Γ : Ctx L mem} {ℓ : L} (l l' : Lags Γ) :
    (sh : TickShape) → (vs : VarsOf Γ ℓ sh) → shapeLag (Lags.sup l l') sh vs = max (shapeLag l sh vs) (shapeLag l' sh vs)
  | .sing _, v => Lags.get_sup l l' v
  | .stream _ _ _ _, v => Lags.get_sup l l' v
  | .ksing _ _ _ _, v => Lags.get_sup l l' v
  | .kstream _ _ _ _ _ _, v => Lags.get_sup l l' v
  | .pair a b, (va, vb) => by
    show max (shapeLag (Lags.sup l l') a va) (shapeLag (Lags.sup l l') b vb)
      = max (max (shapeLag l a va) (shapeLag l b vb)) (max (shapeLag l' a va) (shapeLag l' b vb))
    rw [shapeLag_sup l l' a va, shapeLag_sup l l' b vb, Lags.max_max_max]

theorem shapeLag_const {Γ : Ctx L mem} {ℓ : L} (c : Nat) :
    (sh : TickShape) → (vs : VarsOf Γ ℓ sh) → shapeLag (Lags.const Γ c) sh vs = c
  | .sing _, v => Lags.get_const c v
  | .stream _ _ _ _, v => Lags.get_const c v
  | .ksing _ _ _ _, v => Lags.get_const c v
  | .kstream _ _ _ _ _ _, v => Lags.get_const c v
  | .pair a b, (va, vb) => by
    show max (shapeLag (Lags.const Γ c) a va) (shapeLag (Lags.const Γ c) b vb) = c
    rw [shapeLag_const c a va, shapeLag_const c b vb, Nat.max_self]

mutual

theorem Node.lagOf_sup : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) → (l l' : Lags Γ) →
    Node.lagOf n (Lags.sup l l') = Lags.sup (Node.lagOf n l) (Node.lagOf n l')
  | _, _, .map _ _ x _, l, l' => by
    show HList.cons (f := fun _ => Nat) ((Lags.sup l l').get x) HList.nil = HList.cons (f := fun _ => Nat) (max (l.get x) (l'.get x)) HList.nil
    rw [Lags.get_sup]
  | _, _, .filterMap _ _ x _, l, l' => by
    show HList.cons (f := fun _ => Nat) ((Lags.sup l l').get x) HList.nil = HList.cons (f := fun _ => Nat) (max (l.get x) (l'.get x)) HList.nil
    rw [Lags.get_sup]
  | _, _, .broadcast_closed _ x, l, l' => by
    show HList.cons (f := fun _ => Nat) ((Lags.sup l l').get x - 1) HList.nil = HList.cons (f := fun _ => Nat) (max (l.get x - 1) (l'.get x - 1)) HList.nil
    rw [Lags.get_sup, Lags.max_sub_one]
  | _, _, .demux _ x _, l, l' => by
    show HList.cons (f := fun _ => Nat) ((Lags.sup l l').get x - 1) HList.nil = HList.cons (f := fun _ => Nat) (max (l.get x - 1) (l'.get x - 1)) HList.nil
    rw [Lags.get_sup, Lags.max_sub_one]
  | _, _, .values _ _ x, l, l' => by
    show HList.cons (f := fun _ => Nat) ((Lags.sup l l').get x) HList.nil = HList.cons (f := fun _ => Nat) (max (l.get x) (l'.get x)) HList.nil
    rw [Lags.get_sup]
  | _, _, .into_keyed _ _ x, l, l' => by
    show HList.cons (f := fun _ => Nat) ((Lags.sup l l').get x) HList.nil = HList.cons (f := fun _ => Nat) (max (l.get x) (l'.get x)) HList.nil
    rw [Lags.get_sup]
  | _, _, .kassume_ordering _ _ x _, l, l' => by
    show HList.cons (f := fun _ => Nat) ((Lags.sup l l').get x) HList.nil = HList.cons (f := fun _ => Nat) (max (l.get x) (l'.get x)) HList.nil
    rw [Lags.get_sup]
  | _, _, .kfold_early_stop _ _ _ x _ _, l, l' => by
    show HList.cons (f := fun _ => Nat) ((Lags.sup l l').get x) HList.nil = HList.cons (f := fun _ => Nat) (max (l.get x) (l'.get x)) HList.nil
    rw [Lags.get_sup]
  | _, _, .kget_max_key _ _ _ _ x, l, l' => by
    show HList.cons (f := fun _ => Nat) ((Lags.sup l l').get x) HList.nil = HList.cons (f := fun _ => Nat) (max (l.get x) (l'.get x)) HList.nil
    rw [Lags.get_sup]
  | _, _, .weaken_retries _ x, l, l' => by
    show HList.cons (f := fun _ => Nat) ((Lags.sup l l').get x) HList.nil = HList.cons (f := fun _ => Nat) (max (l.get x) (l'.get x)) HList.nil
    rw [Lags.get_sup]
  | _, _, .union _ a b, l, l' => by
    show HList.cons (f := fun _ => Nat) (max ((Lags.sup l l').get a) ((Lags.sup l l').get b)) HList.nil
      = HList.cons (f := fun _ => Nat) (max (max (l.get a) (l.get b)) (max (l'.get a) (l'.get b))) HList.nil
    rw [Lags.get_sup, Lags.get_sup, Lags.max_max_max]
  | _, _, .assume_ordering _ x _, l, l' => by
    show HList.cons (f := fun _ => Nat) ((Lags.sup l l').get x) HList.nil = HList.cons (f := fun _ => Nat) (max (l.get x) (l'.get x)) HList.nil
    rw [Lags.get_sup]
  | _, _, .fold _ _ _ _ _ _ x, l, l' => by
    show HList.cons (f := fun _ => Nat) ((Lags.sup l l').get x) HList.nil = HList.cons (f := fun _ => Nat) (max (l.get x) (l'.get x)) HList.nil
    rw [Lags.get_sup]
  | _, _, .fold_monotone _ _ _ _ _ _ x, l, l' => by
    show HList.cons (f := fun _ => Nat) ((Lags.sup l l').get x) HList.nil = HList.cons (f := fun _ => Nat) (max (l.get x) (l'.get x)) HList.nil
    rw [Lags.get_sup]
  | _, _, .snapshot _ x _, l, l' => by
    show HList.cons (f := fun _ => Nat) ((Lags.sup l l').get x) HList.nil = HList.cons (f := fun _ => Nat) (max (l.get x) (l'.get x)) HList.nil
    rw [Lags.get_sup]
  | _, _, .batch _ _ _ x _, l, l' => by
    show HList.cons (f := fun _ => Nat) ((Lags.sup l l').get x) HList.nil = HList.cons (f := fun _ => Nat) (max (l.get x) (l'.get x)) HList.nil
    rw [Lags.get_sup]
  | _, _, .source_interval _ _, _, _ => rfl
  | _, _, .firstTick _ _ x, l, l' => by
    show HList.cons (f := fun _ => Nat) ((Lags.sup l l').get x) HList.nil = HList.cons (f := fun _ => Nat) (max (l.get x) (l'.get x)) HList.nil
    rw [Lags.get_sup]
  | _, _, .mapTick x _, l, l' => by
    show HList.cons (f := fun _ => Nat) ((Lags.sup l l').get x) HList.nil = HList.cons (f := fun _ => Nat) (max (l.get x) (l'.get x)) HList.nil
    rw [Lags.get_sup]
  | _, _, .zipTick a b, l, l' => by
    show HList.cons (f := fun _ => Nat) (max ((Lags.sup l l').get a) ((Lags.sup l l').get b)) HList.nil
      = HList.cons (f := fun _ => Nat) (max (max (l.get a) (l.get b)) (max (l'.get a) (l'.get b))) HList.nil
    rw [Lags.get_sup, Lags.get_sup, Lags.max_max_max]
  | _, _, .defer_tick _ x, l, l' => by
    show HList.cons (f := fun _ => Nat) ((Lags.sup l l').get x - 1) HList.nil = HList.cons (f := fun _ => Nat) (max (l.get x - 1) (l'.get x - 1)) HList.nil
    rw [Lags.get_sup, Lags.max_sub_one]
  | _, _, .allTicks _ x, l, l' => by
    show HList.cons (f := fun _ => Nat) ((Lags.sup l l').get x) HList.nil = HList.cons (f := fun _ => Nat) (max (l.get x) (l'.get x)) HList.nil
    rw [Lags.get_sup]
  | _, _, .flattenOrdered _ x, l, l' => by
    show HList.cons (f := fun _ => Nat) ((Lags.sup l l').get x) HList.nil = HList.cons (f := fun _ => Nat) (max (l.get x) (l'.get x)) HList.nil
    rw [Lags.get_sup]
  | _, _, .flattenUnordered _ x, l, l' => by
    show HList.cons (f := fun _ => Nat) ((Lags.sup l l').get x) HList.nil = HList.cons (f := fun _ => Nat) (max (l.get x) (l'.get x)) HList.nil
    rw [Lags.get_sup]
  | _, _, .tinto_keyed _ _ x, l, l' => by
    show HList.cons (f := fun _ => Nat) ((Lags.sup l l').get x) HList.nil = HList.cons (f := fun _ => Nat) (max (l.get x) (l'.get x)) HList.nil
    rw [Lags.get_sup]
  | _, _, .tassume_ordering _ x _, l, l' => by
    show HList.cons (f := fun _ => Nat) ((Lags.sup l l').get x) HList.nil = HList.cons (f := fun _ => Nat) (max (l.get x) (l'.get x)) HList.nil
    rw [Lags.get_sup]
  | _, _, .reduce_watermark _ _ _ _ x wm _ _ _ _, l, l' => by
    show HList.cons (f := fun _ => Nat) (max ((Lags.sup l l').get x) ((Lags.sup l l').get wm)) HList.nil
      = HList.cons (f := fun _ => Nat) (max (max (l.get x) (l.get wm)) (max (l'.get x) (l'.get wm))) HList.nil
    rw [Lags.get_sup, Lags.get_sup, Lags.max_max_max]
  | _, _, .kentries _ _ x, l, l' => by
    show HList.cons (f := fun _ => Nat) ((Lags.sup l l').get x) HList.nil = HList.cons (f := fun _ => Nat) (max (l.get x) (l'.get x)) HList.nil
    rw [Lags.get_sup]
  | _, _, .kmap _ _ _ x _, l, l' => by
    show HList.cons (f := fun _ => Nat) ((Lags.sup l l').get x) HList.nil = HList.cons (f := fun _ => Nat) (max (l.get x) (l'.get x)) HList.nil
    rw [Lags.get_sup]
  | _, _, .kallTicks _ _ x, l, l' => by
    show HList.cons (f := fun _ => Nat) ((Lags.sup l l').get x) HList.nil = HList.cons (f := fun _ => Nat) (max (l.get x) (l'.get x)) HList.nil
    rw [Lags.get_sup]
  | _, _, .kfold _ _ _ _ _ _ _ _ _ x, l, l' => by
    show HList.cons (f := fun _ => Nat) ((Lags.sup l l').get x) HList.nil = HList.cons (f := fun _ => Nat) (max (l.get x) (l'.get x)) HList.nil
    rw [Lags.get_sup]
  | _, _, .ksnapshot _ _ _ x _, l, l' => by
    show HList.cons (f := fun _ => Nat) ((Lags.sup l l').get x) HList.nil = HList.cons (f := fun _ => Nat) (max (l.get x) (l'.get x)) HList.nil
    rw [Lags.get_sup]
  | _, _, .tfold _ _ _ _ _ _ x, l, l' => by
    show HList.cons (f := fun _ => Nat) ((Lags.sup l l').get x) HList.nil = HList.cons (f := fun _ => Nat) (max (l.get x) (l'.get x)) HList.nil
    rw [Lags.get_sup]
  | _, _, .latest _ x, l, l' => by
    show HList.cons (f := fun _ => Nat) ((Lags.sup l l').get x) HList.nil = HList.cons (f := fun _ => Nat) (max (l.get x) (l'.get x)) HList.nil
    rw [Lags.get_sup]
  | _, _, .latest_monotone x, l, l' => by
    show HList.cons (f := fun _ => Nat) ((Lags.sup l l').get x) HList.nil = HList.cons (f := fun _ => Nat) (max (l.get x) (l'.get x)) HList.nil
    rw [Lags.get_sup]
  | _, _, .time, _, _ => rfl
  | _, _, .snapshot_bounded _ _, _, _ => rfl
  | _, _, .tick_scan (ℓ := ℓ) _ ins outs x _ _, l, l' => by
    show Lags.const (shapeKinds ℓ outs) (shapeLag (Lags.sup l l') ins x)
      = Lags.sup (Lags.const (shapeKinds ℓ outs) (shapeLag l ins x)) (Lags.const (shapeKinds ℓ outs) (shapeLag l' ins x))
    rw [shapeLag_sup, Lags.const_sup]
  | _, _, .call N args, l, l' => by
    show Dag.lagOf N (args.map (fun v => (Lags.sup l l').get v)) = _
    rw [Lags.mapSubst_sup]
    exact Dag.lagOf_sup N _ _
  | _, _, .fixN ks body, l, l' => by
    show (Lags.const (ks.map KnotKind.toKind) (Lags.maxAll (Lags.sup l l'))).append
        (HList.drop (κs := ks.map KnotKind.toKind) (Dag.lagOf body ((Lags.const (ks.map KnotKind.toKind) (Lags.maxAll (Lags.sup l l'))).append (Lags.sup l l'))))
      = Lags.sup ((Lags.const (ks.map KnotKind.toKind) (Lags.maxAll l)).append
          (HList.drop (κs := ks.map KnotKind.toKind) (Dag.lagOf body ((Lags.const (ks.map KnotKind.toKind) (Lags.maxAll l)).append l))))
        ((Lags.const (ks.map KnotKind.toKind) (Lags.maxAll l')).append
          (HList.drop (κs := ks.map KnotKind.toKind) (Dag.lagOf body ((Lags.const (ks.map KnotKind.toKind) (Lags.maxAll l')).append l'))))
    rw [Lags.maxAll_sup, Lags.const_sup, ← Lags.sup_append, Dag.lagOf_sup body, Lags.drop_sup, Lags.sup_append]

theorem Dag.lagOf_sup : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) → (l l' : Lags Γ) →
    Dag.lagOf d (Lags.sup l l') = Lags.sup (Dag.lagOf d l) (Dag.lagOf d l')
  | _, _, .ret xs, l, l' => by
    show xs.map (fun v => (Lags.sup l l').get v) = _
    exact Lags.mapSubst_sup l l' xs
  | _, _, .letN n rest, l, l' => by
    show Dag.lagOf rest ((Node.lagOf n (Lags.sup l l')).append (Lags.sup l l')) = _
    rw [Node.lagOf_sup n, ← Lags.sup_append]
    exact Dag.lagOf_sup rest _ _

end

mutual

theorem Node.lagOf_zero : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    Node.lagOf n (Lags.const Γ 0) = Lags.const κs 0
  | Γ, _, .map _ _ x _ => by
    show HList.cons (f := fun _ => Nat) ((Lags.const Γ 0).get x) HList.nil = HList.cons (f := fun _ => Nat) 0 HList.nil
    rw [Lags.get_const]
  | Γ, _, .filterMap _ _ x _ => by
    show HList.cons (f := fun _ => Nat) ((Lags.const Γ 0).get x) HList.nil = HList.cons (f := fun _ => Nat) 0 HList.nil
    rw [Lags.get_const]
  | Γ, _, .broadcast_closed _ x => by
    show HList.cons (f := fun _ => Nat) ((Lags.const Γ 0).get x - 1) HList.nil = HList.cons (f := fun _ => Nat) 0 HList.nil
    rw [Lags.get_const]
  | Γ, _, .demux _ x _ => by
    show HList.cons (f := fun _ => Nat) ((Lags.const Γ 0).get x - 1) HList.nil = HList.cons (f := fun _ => Nat) 0 HList.nil
    rw [Lags.get_const]
  | Γ, _, .values _ _ x => by
    show HList.cons (f := fun _ => Nat) ((Lags.const Γ 0).get x) HList.nil = HList.cons (f := fun _ => Nat) 0 HList.nil
    rw [Lags.get_const]
  | Γ, _, .into_keyed _ _ x => by
    show HList.cons (f := fun _ => Nat) ((Lags.const Γ 0).get x) HList.nil = HList.cons (f := fun _ => Nat) 0 HList.nil
    rw [Lags.get_const]
  | Γ, _, .kassume_ordering _ _ x _ => by
    show HList.cons (f := fun _ => Nat) ((Lags.const Γ 0).get x) HList.nil = HList.cons (f := fun _ => Nat) 0 HList.nil
    rw [Lags.get_const]
  | Γ, _, .kfold_early_stop _ _ _ x _ _ => by
    show HList.cons (f := fun _ => Nat) ((Lags.const Γ 0).get x) HList.nil = HList.cons (f := fun _ => Nat) 0 HList.nil
    rw [Lags.get_const]
  | Γ, _, .kget_max_key _ _ _ _ x => by
    show HList.cons (f := fun _ => Nat) ((Lags.const Γ 0).get x) HList.nil = HList.cons (f := fun _ => Nat) 0 HList.nil
    rw [Lags.get_const]
  | Γ, _, .weaken_retries _ x => by
    show HList.cons (f := fun _ => Nat) ((Lags.const Γ 0).get x) HList.nil = HList.cons (f := fun _ => Nat) 0 HList.nil
    rw [Lags.get_const]
  | Γ, _, .union _ a b => by
    show HList.cons (f := fun _ => Nat) (max ((Lags.const Γ 0).get a) ((Lags.const Γ 0).get b)) HList.nil = HList.cons (f := fun _ => Nat) 0 HList.nil
    rw [Lags.get_const, Lags.get_const, Nat.max_self]
  | Γ, _, .assume_ordering _ x _ => by
    show HList.cons (f := fun _ => Nat) ((Lags.const Γ 0).get x) HList.nil = HList.cons (f := fun _ => Nat) 0 HList.nil
    rw [Lags.get_const]
  | Γ, _, .fold _ _ _ _ _ _ x => by
    show HList.cons (f := fun _ => Nat) ((Lags.const Γ 0).get x) HList.nil = HList.cons (f := fun _ => Nat) 0 HList.nil
    rw [Lags.get_const]
  | Γ, _, .fold_monotone _ _ _ _ _ _ x => by
    show HList.cons (f := fun _ => Nat) ((Lags.const Γ 0).get x) HList.nil = HList.cons (f := fun _ => Nat) 0 HList.nil
    rw [Lags.get_const]
  | Γ, _, .snapshot _ x _ => by
    show HList.cons (f := fun _ => Nat) ((Lags.const Γ 0).get x) HList.nil = HList.cons (f := fun _ => Nat) 0 HList.nil
    rw [Lags.get_const]
  | Γ, _, .batch _ _ _ x _ => by
    show HList.cons (f := fun _ => Nat) ((Lags.const Γ 0).get x) HList.nil = HList.cons (f := fun _ => Nat) 0 HList.nil
    rw [Lags.get_const]
  | _, _, .source_interval _ _ => rfl
  | Γ, _, .firstTick _ _ x => by
    show HList.cons (f := fun _ => Nat) ((Lags.const Γ 0).get x) HList.nil = HList.cons (f := fun _ => Nat) 0 HList.nil
    rw [Lags.get_const]
  | Γ, _, .mapTick x _ => by
    show HList.cons (f := fun _ => Nat) ((Lags.const Γ 0).get x) HList.nil = HList.cons (f := fun _ => Nat) 0 HList.nil
    rw [Lags.get_const]
  | Γ, _, .zipTick a b => by
    show HList.cons (f := fun _ => Nat) (max ((Lags.const Γ 0).get a) ((Lags.const Γ 0).get b)) HList.nil = HList.cons (f := fun _ => Nat) 0 HList.nil
    rw [Lags.get_const, Lags.get_const, Nat.max_self]
  | Γ, _, .defer_tick _ x => by
    show HList.cons (f := fun _ => Nat) ((Lags.const Γ 0).get x - 1) HList.nil = HList.cons (f := fun _ => Nat) 0 HList.nil
    rw [Lags.get_const]
  | Γ, _, .allTicks _ x => by
    show HList.cons (f := fun _ => Nat) ((Lags.const Γ 0).get x) HList.nil = HList.cons (f := fun _ => Nat) 0 HList.nil
    rw [Lags.get_const]
  | Γ, _, .flattenOrdered _ x => by
    show HList.cons (f := fun _ => Nat) ((Lags.const Γ 0).get x) HList.nil = HList.cons (f := fun _ => Nat) 0 HList.nil
    rw [Lags.get_const]
  | Γ, _, .flattenUnordered _ x => by
    show HList.cons (f := fun _ => Nat) ((Lags.const Γ 0).get x) HList.nil = HList.cons (f := fun _ => Nat) 0 HList.nil
    rw [Lags.get_const]
  | Γ, _, .tinto_keyed _ _ x => by
    show HList.cons (f := fun _ => Nat) ((Lags.const Γ 0).get x) HList.nil = HList.cons (f := fun _ => Nat) 0 HList.nil
    rw [Lags.get_const]
  | Γ, _, .tassume_ordering _ x _ => by
    show HList.cons (f := fun _ => Nat) ((Lags.const Γ 0).get x) HList.nil = HList.cons (f := fun _ => Nat) 0 HList.nil
    rw [Lags.get_const]
  | Γ, _, .reduce_watermark _ _ _ _ x wm _ _ _ _ => by
    show HList.cons (f := fun _ => Nat) (max ((Lags.const Γ 0).get x) ((Lags.const Γ 0).get wm)) HList.nil = HList.cons (f := fun _ => Nat) 0 HList.nil
    rw [Lags.get_const, Lags.get_const, Nat.max_self]
  | Γ, _, .kentries _ _ x => by
    show HList.cons (f := fun _ => Nat) ((Lags.const Γ 0).get x) HList.nil = HList.cons (f := fun _ => Nat) 0 HList.nil
    rw [Lags.get_const]
  | Γ, _, .kmap _ _ _ x _ => by
    show HList.cons (f := fun _ => Nat) ((Lags.const Γ 0).get x) HList.nil = HList.cons (f := fun _ => Nat) 0 HList.nil
    rw [Lags.get_const]
  | Γ, _, .kallTicks _ _ x => by
    show HList.cons (f := fun _ => Nat) ((Lags.const Γ 0).get x) HList.nil = HList.cons (f := fun _ => Nat) 0 HList.nil
    rw [Lags.get_const]
  | Γ, _, .kfold _ _ _ _ _ _ _ _ _ x => by
    show HList.cons (f := fun _ => Nat) ((Lags.const Γ 0).get x) HList.nil = HList.cons (f := fun _ => Nat) 0 HList.nil
    rw [Lags.get_const]
  | Γ, _, .ksnapshot _ _ _ x _ => by
    show HList.cons (f := fun _ => Nat) ((Lags.const Γ 0).get x) HList.nil = HList.cons (f := fun _ => Nat) 0 HList.nil
    rw [Lags.get_const]
  | Γ, _, .tfold _ _ _ _ _ _ x => by
    show HList.cons (f := fun _ => Nat) ((Lags.const Γ 0).get x) HList.nil = HList.cons (f := fun _ => Nat) 0 HList.nil
    rw [Lags.get_const]
  | Γ, _, .latest _ x => by
    show HList.cons (f := fun _ => Nat) ((Lags.const Γ 0).get x) HList.nil = HList.cons (f := fun _ => Nat) 0 HList.nil
    rw [Lags.get_const]
  | Γ, _, .latest_monotone x => by
    show HList.cons (f := fun _ => Nat) ((Lags.const Γ 0).get x) HList.nil = HList.cons (f := fun _ => Nat) 0 HList.nil
    rw [Lags.get_const]
  | _, _, .time => rfl
  | _, _, .snapshot_bounded _ _ => rfl
  | Γ, _, .tick_scan (ℓ := ℓ) _ ins outs x _ _ => by
    show Lags.const (shapeKinds ℓ outs) (shapeLag (Lags.const Γ 0) ins x) = Lags.const (shapeKinds ℓ outs) 0
    rw [shapeLag_const]
  | Γ, _, .call N args => by
    show Dag.lagOf N (args.map (fun v => (Lags.const Γ 0).get v)) = _
    rw [Lags.mapSubst_const]
    exact Dag.lagOf_zero N
  | Γ, _, .fixN ks body => by
    show (Lags.const (ks.map KnotKind.toKind) (Lags.maxAll (Lags.const Γ 0))).append
        (HList.drop (κs := ks.map KnotKind.toKind) (Dag.lagOf body ((Lags.const (ks.map KnotKind.toKind) (Lags.maxAll (Lags.const Γ 0))).append (Lags.const Γ 0))))
      = Lags.const _ 0
    rw [Lags.maxAll_const_zero, Lags.const_append, Dag.lagOf_zero body, Lags.drop_const, Lags.const_append]

theorem Dag.lagOf_zero : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    Dag.lagOf d (Lags.const Γ 0) = Lags.const outs 0
  | Γ, _, .ret xs => by
    show xs.map (fun v => (Lags.const Γ 0).get v) = _
    exact Lags.mapSubst_const 0 xs
  | Γ, _, .letN n rest => by
    show Dag.lagOf rest ((Node.lagOf n (Lags.const Γ 0)).append (Lags.const Γ 0)) = _
    rw [Node.lagOf_zero n, Lags.const_append]
    exact Dag.lagOf_zero rest

end

/-- The analysis is non-expansive: no output lags more than the largest input. -/
theorem Dag.lagOf_le_maxAll {Γ : Ctx L mem} {outs : List (Kind L mem)} (d : Dag Γ outs) (l : Lags Γ) :
    Lags.Le (Dag.lagOf d l) (Lags.const outs (Lags.maxAll l)) := by
  refine (Dag.lagOf_mono d (Lags.le_const_of_maxAll l (Nat.le_refl _))).trans ?_
  have h := Dag.lagOf_add_le d (Lags.const Γ 0) (Lags.maxAll l)
  rw [Lags.addC_const, Nat.zero_add, Dag.lagOf_zero, Lags.addC_const, Nat.zero_add] at h
  exact h

/-- **A knot body's lag functional obeys the laws** (`LagLaws`): with the
externals at `0` (`Φ₀`) and at `l` (`Φ`). -/
theorem Dag.knotLag_laws {Γ : Ctx L mem} (ks : List (KnotKind L mem)) {outs : List (Kind L mem)}
    (body : Dag (ks.map KnotKind.toKind ++ Γ) (ks.map KnotKind.toKind ++ outs)) (l : Lags Γ) :
    LagLaws (Dag.knotLag ks body (Lags.const Γ 0)) (Dag.knotLag ks body l) where
  mono := fun hm => Lags.Le.take (Dag.lagOf_mono body (Lags.Le.append hm (Lags.Le.refl _)))
  sub := fun m c => by
    show Lags.Le (HList.take (κs := ks.map KnotKind.toKind) (Dag.lagOf body ((Lags.addC m c).append (Lags.const Γ 0))))
      (Lags.addC (HList.take (κs := ks.map KnotKind.toKind) (Dag.lagOf body (m.append (Lags.const Γ 0)))) c)
    rw [← Lags.take_addC]
    refine Lags.Le.take ((Dag.lagOf_mono body ?_).trans (Dag.lagOf_add_le body _ c))
    rw [Lags.addC_append, Lags.addC_const, Nat.zero_add]
    exact Lags.Le.append (Lags.Le.refl _) (Lags.Le.const (Nat.zero_le c))
  max := fun m m' => by
    show HList.take (κs := ks.map KnotKind.toKind) (Dag.lagOf body ((Lags.sup m m').append (Lags.const Γ 0)))
      = Lags.sup (HList.take (κs := ks.map KnotKind.toKind) (Dag.lagOf body (m.append (Lags.const Γ 0))))
          (HList.take (κs := ks.map KnotKind.toKind) (Dag.lagOf body (m'.append (Lags.const Γ 0))))
    rw [← Lags.take_sup, ← Dag.lagOf_sup, Lags.sup_append, Lags.sup_self]
  zero := by
    show HList.take (κs := ks.map KnotKind.toKind) (Dag.lagOf body ((Lags.const _ 0).append (Lags.const Γ 0))) = Lags.const _ 0
    rw [Lags.const_append, Dag.lagOf_zero, Lags.take_const]
  mix := fun m => by
    show HList.take (κs := ks.map KnotKind.toKind) (Dag.lagOf body (m.append l))
      = Lags.sup (HList.take (κs := ks.map KnotKind.toKind) (Dag.lagOf body (m.append (Lags.const Γ 0))))
          (HList.take (κs := ks.map KnotKind.toKind) (Dag.lagOf body ((Lags.const _ 0).append l)))
    rw [← Lags.take_sup, ← Dag.lagOf_sup, Lags.sup_append, Lags.sup_const_zero_right, Lags.sup_const_zero_left]

/-! ## Graded agreement, per kind (D95) -/

/-- Two machine wires of a kind agree at lag `n` below the horizon `h`
(`SchedCausal`'s `SAgreeL`/`FAgreeL`/`TAgreeL`). -/
def Kind.agreeL (h n : Nat) : (κ : Kind L mem) → κ.sched → κ.sched → Prop
  | .stream _ _ _ _ _, x, y => SAgreeL h n x y
  | .keyed _ _ _ _ _ _ _, x, y => SAgreeL h n x y
  | .sing _ _ _ _ _ _ _, x, y => FAgreeL h n x y
  | .ksing _ _ _ _ _ _ _ _ _ _, x, y => FAgreeL h n x y
  | .atomic _ _, x, y => TAgreeL h n x y
  | .atomicStream _ _ _ _ _, x, y => TAgreeL h n x y
  | .atomicKeyed _ _ _ _ _ _ _, x, y => TAgreeL h n x y
  | .atomicKSing _ _ _ _ _, x, y => TAgreeL h n x y

/-- Pointwise graded agreement of two environments at a lag per wire. -/
def HAgreeL (h : Nat) : {κs : List (Kind L mem)} → HList (fun _ => Nat) κs → HList (Kind.sched (mem := mem)) κs →
    HList (Kind.sched (mem := mem)) κs → Prop
  | [], _, _, _ => True
  | κ :: _, lags, x, y => Kind.agreeL h lags.1 κ x.1 y.1 ∧ HAgreeL h lags.2 x.2 y.2

theorem Kind.agreeL_zero {h : Nat} {κ : Kind L mem} {x y : κ.sched} : Kind.agreeL h 0 κ x y ↔ Kind.agree h κ x y := by
  cases κ <;> first | exact SAgreeL_zero | exact FAgreeL_zero | exact TAgreeL_zero

theorem Kind.agreeL.refl (h n : Nat) (κ : Kind L mem) (x : κ.sched) : Kind.agreeL h n κ x x := by
  cases κ <;> first | exact SAgreeL.refl h n x | exact FAgreeL.refl h n x

theorem Kind.agreeL.mono_lag {h n n' : Nat} (hn : n ≤ n') {κ : Kind L mem} {x y : κ.sched}
    (hxy : Kind.agreeL h n κ x y) : Kind.agreeL h n' κ x y := by
  cases κ <;> first | exact SAgreeL.mono_lag hn hxy | exact FAgreeL.mono_lag hn hxy

theorem Kind.agreeL.mono {h h' n : Nat} (hh : h' ≤ h) {κ : Kind L mem} {x y : κ.sched}
    (hxy : Kind.agreeL h n κ x y) : Kind.agreeL h' n κ x y := by
  cases κ <;> first | exact SAgreeL.mono hh hxy | exact FAgreeL.mono hh hxy

theorem HAgreeL_zero {h : Nat} : {κs : List (Kind L mem)} → {x y : HList (Kind.sched (mem := mem)) κs} →
    (HAgreeL h (Lags.const κs 0) x y ↔ HAgree h x y)
  | [], _, _ => Iff.rfl
  | _ :: _, _, _ => and_congr Kind.agreeL_zero HAgreeL_zero

theorem HAgreeL.refl (h : Nat) : {κs : List (Kind L mem)} → (lags : Lags κs) → (x : HList (Kind.sched (mem := mem)) κs) →
    HAgreeL h lags x x
  | [], _, _ => trivial
  | _ :: _, lags, x => ⟨Kind.agreeL.refl h lags.1 _ x.1, HAgreeL.refl h lags.2 x.2⟩

theorem HAgreeL.mono_lag {h : Nat} : {κs : List (Kind L mem)} → {lags lags' : Lags κs} → Lags.Le lags lags' →
    {x y : HList (Kind.sched (mem := mem)) κs} → HAgreeL h lags x y → HAgreeL h lags' x y
  | [], _, _, _, _, _, _ => trivial
  | _ :: _, _, _, hl, _, _, hxy => ⟨hxy.1.mono_lag hl.1, HAgreeL.mono_lag hl.2 hxy.2⟩

theorem HAgreeL.mono {h h' : Nat} (hh : h' ≤ h) : {κs : List (Kind L mem)} → {lags : Lags κs} →
    {x y : HList (Kind.sched (mem := mem)) κs} → HAgreeL h lags x y → HAgreeL h' lags x y
  | [], _, _, _, _ => trivial
  | _ :: _, _, _, _, hxy => ⟨hxy.1.mono hh, HAgreeL.mono hh hxy.2⟩

theorem HAgreeL.get {h : Nat} : {Γ : Ctx L mem} → {κ : Kind L mem} → {lags : Lags Γ} →
    {x y : HList (Kind.sched (mem := mem)) Γ} → HAgreeL h lags x y → (v : Var Γ κ) →
    Kind.agreeL h (lags.get v) κ (x.get v) (y.get v)
  | _ :: _, _, _, _, _, hxy, .zero => hxy.1
  | _ :: _, _, _, _, _, hxy, .succ v => HAgreeL.get hxy.2 v

theorem HAgreeL.append {h : Nat} : {κs Γ : List (Kind L mem)} → {l : Lags κs} → {l' : Lags Γ} →
    {x y : HList (Kind.sched (mem := mem)) κs} → {x' y' : HList (Kind.sched (mem := mem)) Γ} →
    HAgreeL h l x y → HAgreeL h l' x' y' → HAgreeL h (l.append l') (x.append x') (y.append y')
  | [], _, _, _, _, _, _, _, _, h' => h'
  | _ :: _, _, _, _, _, _, _, _, hxy, h' => ⟨hxy.1, HAgreeL.append hxy.2 h'⟩

theorem HAgreeL.take {h : Nat} : {κs Γ : List (Kind L mem)} → {l : Lags (κs ++ Γ)} →
    {x y : HList (Kind.sched (mem := mem)) (κs ++ Γ)} → HAgreeL h l x y →
    HAgreeL h (HList.take (κs := κs) l) (HList.take (κs := κs) x) (HList.take (κs := κs) y)
  | [], _, _, _, _, _ => trivial
  | _ :: _, _, _, _, _, hxy => ⟨hxy.1, HAgreeL.take hxy.2⟩

theorem HAgreeL.drop {h : Nat} : {κs Γ : List (Kind L mem)} → {l : Lags (κs ++ Γ)} →
    {x y : HList (Kind.sched (mem := mem)) (κs ++ Γ)} → HAgreeL h l x y →
    HAgreeL h (HList.drop (κs := κs) l) (HList.drop (κs := κs) x) (HList.drop (κs := κs) y)
  | [], _, _, _, _, hxy => hxy
  | _ :: κs, Γ, _, _, _, hxy => HAgreeL.drop (κs := κs) (Γ := Γ) hxy.2

theorem HAgreeL.mapSubst {h : Nat} {Γ Γ' : Ctx L mem} {lags : Lags Γ} {x y : Env Γ} (hxy : HAgreeL h lags x y)
    (args : Subst Γ' Γ) :
    HAgreeL h (args.map (fun v => lags.get v)) (args.map (fun v => x.get v)) (args.map (fun v => y.get v)) := by
  induction Γ' with
  | nil => trivial
  | cons κ κs ih => exact ⟨hxy.get args.1, ih args.2⟩

/-- Shaped inputs: agreement leafwise, at the maximum of the leaves' lags. -/
theorem readShape_agreeL {h : Nat} {Γ : Ctx L mem} {lags : Lags Γ} {x y : Env Γ} (hxy : HAgreeL h lags x y) (ℓ : L) :
    (sh : TickShape) → (vs : VarsOf Γ ℓ sh) →
    TAgreeOfL h (shapeLag lags sh vs) sh (readShape x ℓ sh vs) (readShape y ℓ sh vs)
  | .sing _, v => hxy.get v
  | .stream _ _ _ _, v => hxy.get v
  | .ksing _ _ _ _, v => hxy.get v
  | .kstream _ _ _ _ _ _, v => hxy.get v
  | .pair a b, (va, vb) =>
    ⟨TAgreeOfL.mono_lag (Nat.le_max_left _ _) a (readShape_agreeL hxy ℓ a va),
     TAgreeOfL.mono_lag (Nat.le_max_right _ _) b (readShape_agreeL hxy ℓ b vb)⟩
where
  TAgreeOfL.mono_lag {ℓ : L} {h n n' : Nat} (hn : n ≤ n') : (sh : TickShape) →
      {x y : AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) (SchedTick.TKStr mem) ℓ sh} →
      TAgreeOfL h n sh x y → TAgreeOfL h n' sh x y
    | .sing _, _, _, hxy => FAgreeL.mono_lag hn hxy
    | .stream _ _ _ _, _, _, hxy => FAgreeL.mono_lag hn hxy
    | .ksing _ _ _ _, _, _, hxy => FAgreeL.mono_lag hn hxy
    | .kstream _ _ _ _ _ _, _, _, hxy => FAgreeL.mono_lag hn hxy
    | .pair a b, _, _, hxy => ⟨TAgreeOfL.mono_lag hn a hxy.1, TAgreeOfL.mono_lag hn b hxy.2⟩

theorem shapeToList_agreeL {h n : Nat} (ℓ : L) : (sh : TickShape) →
    (x y : AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) (SchedTick.TKStr mem) ℓ sh) →
    TAgreeOfL h n sh x y → HAgreeL h (Lags.const (shapeKinds ℓ sh) n) (shapeToList ℓ sh x) (shapeToList ℓ sh y)
  | .sing _, _, _, hxy => ⟨hxy, trivial⟩
  | .stream _ _ _ _, _, _, hxy => ⟨hxy, trivial⟩
  | .ksing _ _ _ _, _, _, hxy => ⟨hxy, trivial⟩
  | .kstream _ _ _ _ _ _, _, _, hxy => ⟨hxy, trivial⟩
  | .pair a b, x, y, hxy => by
    show HAgreeL h (Lags.const (shapeKinds ℓ a ++ shapeKinds ℓ b) n)
      ((shapeToList ℓ a x.1).append (shapeToList ℓ b x.2)) ((shapeToList ℓ a y.1).append (shapeToList ℓ b y.2))
    rw [← Lags.const_append]
    exact HAgreeL.append (shapeToList_agreeL ℓ a x.1 y.1 hxy.1) (shapeToList_agreeL ℓ b x.2 y.2 hxy.2)

/-! ## Knot closure is causal (base) -/

section Knots
variable (pacing : Pacing L mem)

/-- Closing one knot wire is causal in a body that is (hetero-)causal. -/
theorem knotFix_agree {h : Nat} (k : KnotKind L mem)
    {f g : k.toKind.sched → k.toKind.sched}
    (hfg : ∀ h', h' ≤ h → ∀ x y, Kind.agree h' k.toKind x y → Kind.agree h' k.toKind (f x) (g y)) :
    Kind.agree h k.toKind (knotFix pacing k f) (knotFix pacing k g) := by
  cases k with
  | stream ℓ α i ord ret => exact causal_fix_stream (pacing := pacing) () hfg
  | atomic ℓ σ => exact causal_fix_tick (pacing := pacing) () hfg

/-- The Bekić cascade is causal in a (hetero-)causal body. -/
theorem bekic_agree {h : Nat} : (ks : List (KnotKind L mem)) →
    {B B' : HList (Kind.sched (mem := mem)) (ks.map KnotKind.toKind) →
      HList (Kind.sched (mem := mem)) (ks.map KnotKind.toKind)} →
    (∀ h', h' ≤ h → ∀ x y, HAgree h' x y → HAgree h' (B x) (B' y)) →
    HAgree h (bekic pacing ks B) (bekic pacing ks B')
  | [], _, _, _ => trivial
  | k :: ks, B, B', hB => by
    -- `inner` is causal in the later wires; `rest` is causal
    have hinner : ∀ h', h' ≤ h → ∀ xs ys, HAgree h' xs ys →
        Kind.agree h' k.toKind (knotFix pacing k (fun l => (B (HList.cons l xs)).head))
          (knotFix pacing k (fun l => (B' (HList.cons l ys)).head)) := by
      intro h' hh' xs ys hxy
      exact knotFix_agree pacing k (fun h'' hh'' x y hxy' =>
        (hB h'' (hh''.trans hh') _ _ ⟨hxy', HAgree.mono hh'' hxy⟩).head)
    have hrest : HAgree h (bekic pacing ks (fun xs => (B (HList.cons
        (knotFix pacing k (fun l => (B (HList.cons l xs)).head)) xs)).tail))
        (bekic pacing ks (fun xs => (B' (HList.cons
          (knotFix pacing k (fun l => (B' (HList.cons l xs)).head)) xs)).tail)) :=
      bekic_agree ks (fun h' hh' xs ys hxy =>
        (hB h' hh' _ _ ⟨hinner h' hh' xs ys hxy, hxy⟩).tail)
    exact ⟨hinner h (Nat.le_refl _) _ _ hrest, hrest⟩

/-! ### Knot closure, graded (D95)

A cascade body `B` — the body's knot outputs as a function of its knot
inputs, the externals captured — is **graded-causal at a lag functional**
`Φ₀` (`BodyCausal`): knot inputs agreeing at the lags `m` give knot outputs
agreeing at `Φ₀ m`. Two bodies are **related at `Φ`** below a horizon
(`BodyRel`). A body is **strict** (`HStrict`) when it is graded-causal at a
lawful (`LagLaws`), cascade-valid (`cascadeValid`) lag functional: then
every knot of the Bekić cascade is strict in its own wire once the earlier
ones are closed, the ungated diagonal is each knot's fixpoint
(`fix_stream_eq`/`fix_tick_eq`, `Project/Model/Knot.lean`) and two
cascades of related bodies are related at the closure lags
(`bekic_agreeL`). -/

/-- Strictness of a knot body function, per knot kind (`SchedCausal`'s
`SStrict`/`TStrict`). -/
def KnotKind.strictOf : (k : KnotKind L mem) → (k.toKind.sched → k.toKind.sched) → Prop
  | .stream _ _ _ _ _, f => SStrict f
  | .atomic _ _, f => TStrict f

/-- `B` is graded-causal at the lag functional `Φ₀`. -/
def BodyCausal {K : List (Kind L mem)} (Φ₀ : Lags K → Lags K)
    (B : HList (Kind.sched (mem := mem)) K → HList (Kind.sched (mem := mem)) K) : Prop :=
  ∀ (h : Nat) (m : Lags K) (x y : HList (Kind.sched (mem := mem)) K), HAgreeL h m x y → HAgreeL h (Φ₀ m) (B x) (B y)

/-- `B` and `B'` are related at the lag functional `Φ` below the horizon `h`. -/
def BodyRel (h : Nat) {K : List (Kind L mem)} (Φ : Lags K → Lags K)
    (B B' : HList (Kind.sched (mem := mem)) K → HList (Kind.sched (mem := mem)) K) : Prop :=
  ∀ h', h' ≤ h → ∀ (m : Lags K) (x y : HList (Kind.sched (mem := mem)) K), HAgreeL h' m x y → HAgreeL h' (Φ m) (B x) (B' y)

/-- Strictness of a cascade body in its knot wires: graded-causal at a
lawful, cascade-valid lag functional. -/
def HStrict (ks : List (KnotKind L mem))
    (B : HList (Kind.sched (mem := mem)) (ks.map KnotKind.toKind) → HList (Kind.sched (mem := mem)) (ks.map KnotKind.toKind)) :
    Prop :=
  ∃ Φ₀ : Lags (ks.map KnotKind.toKind) → Lags (ks.map KnotKind.toKind),
    LagLaws Φ₀ Φ₀ ∧ cascadeValid (ks.map KnotKind.toKind) Φ₀ = true ∧ BodyCausal Φ₀ B

/-- The first knot's body function is strict when the body is graded-causal
at a functional with `(Φ₀ (1 :: 0)).1 = 0`. -/
theorem KnotKind.strictOf_of {k : KnotKind L mem} {ks : List (KnotKind L mem)}
    {B : HList (Kind.sched (mem := mem)) ((k :: ks).map KnotKind.toKind) →
      HList (Kind.sched (mem := mem)) ((k :: ks).map KnotKind.toKind)}
    {Φ₀ : Lags ((k :: ks).map KnotKind.toKind) → Lags ((k :: ks).map KnotKind.toKind)}
    (hs : BodyCausal Φ₀ B) (hhead : (Φ₀ (HList.cons 1 (Lags.const (ks.map KnotKind.toKind) 0))).1 = 0)
    (xs : HList (Kind.sched (mem := mem)) (ks.map KnotKind.toKind)) :
    k.strictOf (fun l => (B (HList.cons l xs)).head) := by
  cases k with
  | stream ℓ α i ord ret =>
    intro h x y hxy
    have h1 := (hs h (HList.cons 1 (Lags.const (ks.map KnotKind.toKind) 0)) (HList.cons x xs) (HList.cons y xs)
      ⟨hxy, HAgreeL.refl h _ xs⟩).1
    rw [hhead] at h1
    exact SAgreeL_zero.mp h1
  | atomic ℓ σ =>
    intro h x y hxy
    have h1 := (hs h (HList.cons 1 (Lags.const (ks.map KnotKind.toKind) 0)) (HList.cons x xs) (HList.cons y xs)
      ⟨hxy, HAgreeL.refl h _ xs⟩).1
    rw [hhead] at h1
    exact FAgreeL_zero.mp h1

/-- Closing one knot wire, graded: strict bodies related at every lag `≥ Lg`
give knots related at lag `Lg`. -/
theorem knotFix_agreeL {h Lg : Nat} (k : KnotKind L mem)
    {f g : k.toKind.sched → k.toKind.sched} (hs : k.strictOf f) (hs' : k.strictOf g)
    (hfg : ∀ L, Lg ≤ L → ∀ h', h' ≤ h → ∀ x y,
      Kind.agreeL h' (L + 1) k.toKind x y → Kind.agreeL h' L k.toKind (f x) (g y)) :
    Kind.agreeL h Lg k.toKind (knotFix pacing k f) (knotFix pacing k g) := by
  cases k with
  | stream ℓ α i ord ret => exact causalL_fix_stream (pacing := pacing) () hs hs' (hfg Lg (Nat.le_refl _))
  | atomic ℓ σ => exact causalL_fix_tick (pacing := pacing) () (hfg Lg (Nat.le_refl _))

/-- The first knot of two cascades, graded, as a function of the later
wires: it agrees at its closure lag `(Φ (0 :: m)).1` when the rests agree
at `m` (the Bekić inner step, `LagLaws.head_bound`). -/
theorem knotInner_agreeL {h : Nat} (k : KnotKind L mem) (ks : List (KnotKind L mem))
    {B B' : HList (Kind.sched (mem := mem)) ((k :: ks).map KnotKind.toKind) →
      HList (Kind.sched (mem := mem)) ((k :: ks).map KnotKind.toKind)}
    {Φ₀ Φ : Lags ((k :: ks).map KnotKind.toKind) → Lags ((k :: ks).map KnotKind.toKind)}
    (hl : LagLaws Φ₀ Φ) (hhead : (Φ₀ (HList.cons 1 (Lags.const (ks.map KnotKind.toKind) 0))).1 = 0)
    (hs : BodyCausal Φ₀ B) (hs' : BodyCausal Φ₀ B') (hB : BodyRel h Φ B B') :
    ∀ h', h' ≤ h → ∀ (m : Lags (ks.map KnotKind.toKind)) xs ys, HAgreeL h' m xs ys →
      Kind.agreeL h' (Φ (HList.cons 0 m)).1 k.toKind (knotFix pacing k (fun l => (B (HList.cons l xs)).head))
        (knotFix pacing k (fun l => (B' (HList.cons l ys)).head)) := by
  intro h' hh' m xs ys hxy
  exact knotFix_agreeL pacing k (KnotKind.strictOf_of hs hhead xs) (KnotKind.strictOf_of hs' hhead ys)
    (fun L hL h'' hh'' x y hxy' =>
      Kind.agreeL.mono_lag (hl.head_bound hhead m L hL)
        (hB h'' (hh''.trans hh') (HList.cons (L + 1) m) (HList.cons x xs) (HList.cons y ys) ⟨hxy', hxy.mono hh''⟩).1)

/-- The rest of a cascade body (the first knot closed) is graded-causal at
the rest's lag functional. -/
theorem BodyCausal.tail {k : KnotKind L mem} {ks : List (KnotKind L mem)}
    {B : HList (Kind.sched (mem := mem)) ((k :: ks).map KnotKind.toKind) →
      HList (Kind.sched (mem := mem)) ((k :: ks).map KnotKind.toKind)}
    {Φ₀ : Lags ((k :: ks).map KnotKind.toKind) → Lags ((k :: ks).map KnotKind.toKind)}
    (hl : LagLaws Φ₀ Φ₀) (hhead : (Φ₀ (HList.cons 1 (Lags.const (ks.map KnotKind.toKind) 0))).1 = 0)
    (hs : BodyCausal Φ₀ B) :
    BodyCausal (LagFun.next Φ₀) (fun xs => (B (HList.cons (knotFix pacing k (fun l => (B (HList.cons l xs)).head)) xs)).tail) := by
  intro h' m xs ys hxy
  have hi := knotInner_agreeL pacing (h := h') k ks hl hhead hs hs (fun h'' _ m x y hxy => hs h'' m x y hxy)
    h' (Nat.le_refl _) m xs ys hxy
  exact (hs h' (HList.cons _ m) (HList.cons _ xs) (HList.cons _ ys) ⟨hi, hxy⟩).2

/-- The rests of two related cascades are related at the rest's lag functional. -/
theorem BodyRel.tail {h : Nat} {k : KnotKind L mem} {ks : List (KnotKind L mem)}
    {B B' : HList (Kind.sched (mem := mem)) ((k :: ks).map KnotKind.toKind) →
      HList (Kind.sched (mem := mem)) ((k :: ks).map KnotKind.toKind)}
    {Φ₀ Φ : Lags ((k :: ks).map KnotKind.toKind) → Lags ((k :: ks).map KnotKind.toKind)}
    (hl : LagLaws Φ₀ Φ) (hhead : (Φ₀ (HList.cons 1 (Lags.const (ks.map KnotKind.toKind) 0))).1 = 0)
    (hs : BodyCausal Φ₀ B) (hs' : BodyCausal Φ₀ B') (hB : BodyRel h Φ B B') :
    BodyRel h (LagFun.next Φ)
      (fun xs => (B (HList.cons (knotFix pacing k (fun l => (B (HList.cons l xs)).head)) xs)).tail)
      (fun ys => (B' (HList.cons (knotFix pacing k (fun l => (B' (HList.cons l ys)).head)) ys)).tail) := by
  intro h' hh' m xs ys hxy
  have hi := knotInner_agreeL pacing k ks hl hhead hs hs' hB h' hh' m xs ys hxy
  exact (hB h' hh' (HList.cons _ m) (HList.cons _ xs) (HList.cons _ ys) ⟨hi, hxy⟩).2

/-- **The Bekić cascade, graded**: two cascades of bodies graded-causal at a
lawful, cascade-valid `Φ₀` and related at `Φ` close to knots related at the
closure lags `closeLags Φ`. -/
theorem bekic_agreeL {h : Nat} : (ks : List (KnotKind L mem)) →
    {B B' : HList (Kind.sched (mem := mem)) (ks.map KnotKind.toKind) →
      HList (Kind.sched (mem := mem)) (ks.map KnotKind.toKind)} →
    (Φ₀ Φ : Lags (ks.map KnotKind.toKind) → Lags (ks.map KnotKind.toKind)) →
    LagLaws Φ₀ Φ → cascadeValid (ks.map KnotKind.toKind) Φ₀ = true →
    BodyCausal Φ₀ B → BodyCausal Φ₀ B' → BodyRel h Φ B B' →
    HAgreeL h (closeLags (ks.map KnotKind.toKind) Φ) (bekic pacing ks B) (bekic pacing ks B')
  | [], _, _, _, _, _, _, _, _, _ => trivial
  | k :: ks, B, B', Φ₀, Φ, hl, hv, hs, hs', hB => by
    obtain ⟨hhead, hv'⟩ := cascadeValid_cons hv
    have hrest := bekic_agreeL ks (LagFun.next Φ₀) (LagFun.next Φ) hl.next hv'
      (BodyCausal.tail pacing hl.self hhead hs) (BodyCausal.tail pacing hl.self hhead hs')
      (BodyRel.tail pacing hl hhead hs hs' hB)
    exact ⟨knotInner_agreeL pacing k ks hl hhead hs hs' hB h (Nat.le_refl _) _ _ _ hrest, hrest⟩

end Knots

/-! ## Every graph is causal -/

section Causal
variable (pacing : Pacing L mem)

mutual

theorem Node.denote_causal : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) → (sch : Node.Sched n) →
    ∀ (h : Nat) (env env' : Env Γ), HAgree h env env' →
    HAgree h (Node.denote pacing n sch env) (Node.denote pacing n sch env')
  | _, _, .map _ _ x f, _, h, env, env', he => ⟨causal_map f (he.get x), trivial⟩
  | _, _, .filterMap _ _ x f, _, h, env, env', he => ⟨causal_filterMap f (he.get x), trivial⟩
  | _, _, .broadcast_closed _ x, d, h, env, env', he => ⟨causal_broadcast_closed d (he.get x), trivial⟩
  | _, _, .demux _ x addr, d, h, env, env', he => ⟨causal_demux d addr (he.get x), trivial⟩
  | _, _, .values _ _ x, _, h, env, env', he => ⟨causal_values (he.get x), trivial⟩
  | _, _, .into_keyed _ _ x, _, h, env, env', he => ⟨causal_into_keyed (he.get x), trivial⟩
  | _, _, .kassume_ordering _ _ x d, _, h, env, env', he => ⟨causal_kassume_ordering d (he.get x), trivial⟩
  | _, _, .kfold_early_stop _ _ _ x f init, _, h, env, env', he =>
    ⟨causal_kfold_early_stop f init (he.get x), trivial⟩
  | _, _, .kget_max_key _ _ _ _ x, _, h, env, env', he => ⟨causal_kget_max_key (he.get x), trivial⟩
  | _, _, .weaken_retries _ x, _, h, env, env', he => ⟨causal_weaken_retries (he.get x), trivial⟩
  | _, _, .union _ a b, _, h, env, env', he => ⟨causal_union (he.get a) (he.get b), trivial⟩
  | _, _, .assume_ordering _ x d, _, h, env, env', he => ⟨causal_assume_ordering d (he.get x), trivial⟩
  | _, _, .fold _ g init Pre _ ok x, _, h, env, env', he => ⟨causal_fold g init Pre ok (he.get x), trivial⟩
  | _, _, .fold_monotone _ vo g init ok hinfl x, _, h, env, env', he =>
    ⟨causal_fold_monotone vo g init ok hinfl (he.get x), trivial⟩
  | _, _, .snapshot _ x d, _, h, env, env', he => ⟨causal_snapshot d (he.get x), trivial⟩
  | _, _, .batch _ _ _ x d, _, h, env, env', he => ⟨causal_batch d (he.get x), trivial⟩
  | _, _, .source_interval delay period, d, h, env, env', he => ⟨causal_source_interval delay period d, trivial⟩
  | _, _, .mapTick x f, _, h, env, env', he => ⟨causal_mapTick f (he.get x), trivial⟩
  | _, _, .firstTick _ _ x, _, h, env, env', he => ⟨causal_firstTick (he.get x), trivial⟩
  | _, _, .zipTick a b, _, h, env, env', he => ⟨causal_zipTick (he.get a) (he.get b), trivial⟩
  | _, _, .defer_tick init x, _, h, env, env', he => ⟨causal_defer_tick init (he.get x), trivial⟩
  | _, _, .allTicks _ x, _, h, env, env', he => ⟨causal_allTicks (he.get x), trivial⟩
  | _, _, .flattenOrdered _ x, _, h, env, env', he => ⟨causal_flattenOrdered (he.get x), trivial⟩
  | _, _, .flattenUnordered _ x, _, h, env, env', he => ⟨causal_flattenUnordered (he.get x), trivial⟩
  | _, _, .tinto_keyed _ _ x, _, h, env, env', he => ⟨causal_tinto_keyed (he.get x), trivial⟩
  | _, _, .tassume_ordering _ x d, _, h, env, env', he => ⟨causal_tassume_ordering d (he.get x), trivial⟩
  | _, _, .reduce_watermark _ _ _ _ x wm f Pre _ ok, _, h, env, env', he =>
    ⟨causal_reduce_watermark f Pre ok (he.get x) (he.get wm), trivial⟩
  | _, _, .kentries _ _ x, _, h, env, env', he => ⟨causal_kentries (he.get x), trivial⟩
  | _, _, .kmap _ _ _ x f, _, h, env, env', he => ⟨causal_kmap f (he.get x), trivial⟩
  | _, _, .kallTicks _ _ x, _, h, env, env', he => ⟨causal_kallTicks (he.get x), trivial⟩
  | _, _, .kfold _ _ _ _ g init Pre _ ok x, _, h, env, env', he =>
    ⟨causal_kfold g init Pre ok (he.get x), trivial⟩
  | _, _, .ksnapshot _ _ _ x d, _, h, env, env', he => ⟨causal_ksnapshot d (he.get x), trivial⟩
  | _, _, .tfold _ g init Pre _ ok x, _, h, env, env', he => ⟨causal_tfold g init Pre ok (he.get x), trivial⟩
  | _, _, .latest _ x, _, h, env, env', he => ⟨causal_latest (he.get x), trivial⟩
  | _, _, .latest_monotone x, _, h, env, env', he => ⟨causal_latest_monotone (he.get x), trivial⟩
  | _, _, .time, d, h, env, env', he => ⟨causal_time d, trivial⟩
  | _, _, .snapshot_bounded v d, _, h, env, env', he => ⟨causal_snapshot_bounded v d, trivial⟩
  | _, _, .tick_scan (ℓ := ℓ) sts ins outs x g seed, _, h, env, env', he =>
    shapeToList_agree ℓ outs _ _ (causal_tick_scan sts ins outs g seed (readShape_agree he ℓ ins x))
  | _, _, .call N args, sch, h, env, env', he => Dag.denote_causal N sch h _ _ (he.mapSubst args)
  | _, _, .fixN ks body, sch, h, env, env', he => by
    have hcl : HAgree h (bekic pacing ks (fun xs => (Dag.denote pacing body sch (xs.append env)).take))
        (bekic pacing ks (fun xs => (Dag.denote pacing body sch (xs.append env')).take)) :=
      bekic_agree pacing ks (fun h' hh' xs ys hxy =>
        (Dag.denote_causal body sch h' _ _ (hxy.append (he.mono hh'))).take)
    exact hcl.append (Dag.denote_causal body sch h _ _ (hcl.append he)).drop

theorem Dag.denote_causal : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) → (sch : Dag.Sched d) →
    ∀ (h : Nat) (env env' : Env Γ), HAgree h env env' →
    HAgree h (Dag.denote pacing d sch env) (Dag.denote pacing d sch env')
  | _, _, .ret xs, _, h, env, env', he => he.mapSubst xs
  | _, _, .letN n rest, sch, h, env, env', he =>
    Dag.denote_causal rest sch.2 h _ _ ((Node.denote_causal n sch.1 h env env' he).append he)

end

/-! ## Every valid graph is graded-causal (D95) -/

mutual

theorem Node.denote_agreeL : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) → (sch : Node.Sched n) →
    Node.valid n = true → ∀ (h : Nat) (lags : Lags Γ) (env env' : Env Γ), HAgreeL h lags env env' →
    HAgreeL h (Node.lagOf n lags) (Node.denote pacing n sch env) (Node.denote pacing n sch env')
  | _, _, .map _ _ x f, _, _, h, lags, env, env', he => ⟨causalL_map f (he.get x), trivial⟩
  | _, _, .filterMap _ _ x f, _, _, h, lags, env, env', he => ⟨causalL_filterMap f (he.get x), trivial⟩
  | _, _, .broadcast_closed _ x, d, _, h, lags, env, env', he => ⟨causalL_broadcast_closed d (he.get x), trivial⟩
  | _, _, .demux _ x addr, d, _, h, lags, env, env', he => ⟨causalL_demux d addr (he.get x), trivial⟩
  | _, _, .values _ _ x, _, _, h, lags, env, env', he => ⟨causalL_values (he.get x), trivial⟩
  | _, _, .into_keyed _ _ x, _, _, h, lags, env, env', he => ⟨causalL_into_keyed (he.get x), trivial⟩
  | _, _, .kassume_ordering _ _ x d, _, _, h, lags, env, env', he => ⟨causalL_kassume_ordering d (he.get x), trivial⟩
  | _, _, .kfold_early_stop _ _ _ x f init, _, _, h, lags, env, env', he =>
    ⟨causalL_kfold_early_stop f init (he.get x), trivial⟩
  | _, _, .kget_max_key _ _ _ _ x, _, _, h, lags, env, env', he => ⟨causalL_kget_max_key (he.get x), trivial⟩
  | _, _, .weaken_retries _ x, _, _, h, lags, env, env', he => ⟨causalL_weaken_retries (he.get x), trivial⟩
  | _, _, .union _ a b, _, _, h, lags, env, env', he => ⟨causalL_union (he.get a) (he.get b), trivial⟩
  | _, _, .assume_ordering _ x d, _, _, h, lags, env, env', he => ⟨causalL_assume_ordering d (he.get x), trivial⟩
  | _, _, .fold _ g init Pre _ ok x, _, _, h, lags, env, env', he => ⟨causalL_fold g init Pre ok (he.get x), trivial⟩
  | _, _, .fold_monotone _ vo g init ok hinfl x, _, _, h, lags, env, env', he =>
    ⟨causalL_fold_monotone vo g init ok hinfl (he.get x), trivial⟩
  | _, _, .snapshot _ x d, _, _, h, lags, env, env', he => ⟨causalL_snapshot d (he.get x), trivial⟩
  | _, _, .batch _ _ _ x d, _, _, h, lags, env, env', he => ⟨causalL_batch d (he.get x), trivial⟩
  | _, _, .source_interval delay period, d, _, h, lags, env, env', he => ⟨SAgreeL.refl h 0 _, trivial⟩
  | _, _, .mapTick x f, _, _, h, lags, env, env', he => ⟨causalL_mapTick f (he.get x), trivial⟩
  | _, _, .firstTick _ _ x, _, _, h, lags, env, env', he => ⟨causalL_firstTick (he.get x), trivial⟩
  | _, _, .zipTick a b, _, _, h, lags, env, env', he => ⟨causalL_zipTick (he.get a) (he.get b), trivial⟩
  | _, _, .defer_tick init x, _, _, h, lags, env, env', he => ⟨causalL_defer_tick init (he.get x), trivial⟩
  | _, _, .allTicks _ x, _, _, h, lags, env, env', he => ⟨causalL_allTicks (he.get x), trivial⟩
  | _, _, .flattenOrdered _ x, _, _, h, lags, env, env', he => ⟨causalL_flattenOrdered (he.get x), trivial⟩
  | _, _, .flattenUnordered _ x, _, _, h, lags, env, env', he => ⟨causalL_flattenUnordered (he.get x), trivial⟩
  | _, _, .tinto_keyed _ _ x, _, _, h, lags, env, env', he => ⟨causalL_tinto_keyed (he.get x), trivial⟩
  | _, _, .tassume_ordering _ x d, _, _, h, lags, env, env', he => ⟨causalL_tassume_ordering d (he.get x), trivial⟩
  | _, _, .reduce_watermark _ _ _ _ x wm f Pre _ ok, _, _, h, lags, env, env', he =>
    ⟨causalL_reduce_watermark f Pre ok (he.get x) (he.get wm), trivial⟩
  | _, _, .kentries _ _ x, _, _, h, lags, env, env', he => ⟨causalL_kentries (he.get x), trivial⟩
  | _, _, .kmap _ _ _ x f, _, _, h, lags, env, env', he => ⟨causalL_kmap f (he.get x), trivial⟩
  | _, _, .kallTicks _ _ x, _, _, h, lags, env, env', he => ⟨causalL_kallTicks (he.get x), trivial⟩
  | _, _, .kfold _ _ _ _ g init Pre _ ok x, _, _, h, lags, env, env', he =>
    ⟨causalL_kfold g init Pre ok (he.get x), trivial⟩
  | _, _, .ksnapshot _ _ _ x d, _, _, h, lags, env, env', he => ⟨causalL_ksnapshot d (he.get x), trivial⟩
  | _, _, .tfold _ g init Pre _ ok x, _, _, h, lags, env, env', he => ⟨causalL_tfold g init Pre ok (he.get x), trivial⟩
  | _, _, .latest _ x, _, _, h, lags, env, env', he => ⟨causalL_latest (he.get x), trivial⟩
  | _, _, .latest_monotone x, _, _, h, lags, env, env', he => ⟨causalL_latest_monotone (he.get x), trivial⟩
  | _, _, .time, d, _, h, lags, env, env', he => ⟨FAgreeL.refl h 0 _, trivial⟩
  | _, _, .snapshot_bounded v d, _, _, h, lags, env, env', he => ⟨FAgreeL.refl h 0 _, trivial⟩
  | _, _, .tick_scan (ℓ := ℓ) sts ins outs x g seed, _, _, h, lags, env, env', he =>
    shapeToList_agreeL ℓ outs _ _ (causalL_tick_scan sts ins outs g seed (readShape_agreeL he ℓ ins x))
  | _, _, .call N args, sch, hv, h, lags, env, env', he => Dag.denote_agreeL N sch hv h _ _ _ (he.mapSubst args)
  | Γ, _, .fixN ks body, sch, hv, h, lags, env, env', he => by
    obtain ⟨hvk, hvb⟩ := Node.valid_fixN ks body hv
    have hb := Dag.denote_agreeL body sch hvb
    have hl := Dag.knotLag_laws ks body lags
    have hs : BodyCausal (Dag.knotLag ks body (Lags.const Γ 0)) (fun xs => (Dag.denote pacing body sch (xs.append env)).take) :=
      fun h' m x y hxy => (hb h' _ _ _ (hxy.append (HAgreeL.refl h' (Lags.const Γ 0) env))).take
    have hs' : BodyCausal (Dag.knotLag ks body (Lags.const Γ 0)) (fun xs => (Dag.denote pacing body sch (xs.append env')).take) :=
      fun h' m x y hxy => (hb h' _ _ _ (hxy.append (HAgreeL.refl h' (Lags.const Γ 0) env'))).take
    have hB : BodyRel h (Dag.knotLag ks body lags) (fun xs => (Dag.denote pacing body sch (xs.append env)).take)
        (fun xs => (Dag.denote pacing body sch (xs.append env')).take) :=
      fun h' hh' m x y hxy => (hb h' _ _ _ (hxy.append (he.mono hh'))).take
    -- the closure lags sit below the externals' maximum
    have hE : Lags.Le (Dag.knotLag ks body lags (Lags.const _ 0)) (Lags.const _ (Lags.maxAll lags)) := by
      have h1 := Lags.Le.take (κs := ks.map KnotKind.toKind)
        ((Dag.lagOf_le_maxAll body ((Lags.const (ks.map KnotKind.toKind) 0).append lags)).trans
          (Lags.Le.const (by rw [Lags.maxAll_append, Lags.maxAll_const_zero, Nat.zero_max])))
      rw [Lags.take_const] at h1
      exact h1
    have hcl : HAgreeL h (Lags.const _ (Lags.maxAll lags))
        (bekic pacing ks (fun xs => (Dag.denote pacing body sch (xs.append env)).take))
        (bekic pacing ks (fun xs => (Dag.denote pacing body sch (xs.append env')).take)) :=
      HAgreeL.mono_lag (closeLags_le _ _ _ hl hE) (bekic_agreeL pacing ks _ _ hl hvk hs hs' hB)
    exact hcl.append (hb h _ _ _ (hcl.append he)).drop

theorem Dag.denote_agreeL : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) → (sch : Dag.Sched d) →
    Dag.valid d = true → ∀ (h : Nat) (lags : Lags Γ) (env env' : Env Γ), HAgreeL h lags env env' →
    HAgreeL h (Dag.lagOf d lags) (Dag.denote pacing d sch env) (Dag.denote pacing d sch env')
  | _, _, .ret xs, _, _, h, lags, env, env', he => he.mapSubst xs
  | _, _, .letN n rest, sch, hv, h, lags, env, env', he =>
    Dag.denote_agreeL rest sch.2 (Dag.valid_letN n rest hv).2 h _ _ _
      ((Node.denote_agreeL n sch.1 (Dag.valid_letN n rest hv).1 h lags env env' he).append he)

end

/-- The cascade body of a valid knot block in a valid graph is strict. -/
theorem Dag.fixN_strict {Γ : Ctx L mem} (ks : List (KnotKind L mem)) {outs : List (Kind L mem)}
    (body : Dag (ks.map KnotKind.toKind ++ Γ) (ks.map KnotKind.toKind ++ outs)) (sch : Dag.Sched body)
    (hv : Node.valid (.fixN ks body) = true) (env : Env Γ) :
    HStrict ks (fun xs => (Dag.denote pacing body sch (xs.append env)).take) :=
  ⟨Dag.knotLag ks body (Lags.const Γ 0), (Dag.knotLag_laws ks body (Lags.const Γ 0)).self, (Node.valid_fixN ks body hv).1,
    fun h m x y hxy => (Dag.denote_agreeL pacing body sch (Node.valid_fixN ks body hv).2 h _ _ _
      (hxy.append (HAgreeL.refl h (Lags.const Γ 0) env))).take⟩

end Causal

/-! ## Every atomic wire of a graph is tick-monotone -/

/-- Step-monotonicity of a machine wire of a kind: trivially true of
stream/singleton carriers (prefix-monotone by construction, or
valueless), `TMono` of the atomic ones. -/
def Kind.tmono : (κ : Kind L mem) → κ.sched → Prop
  | .stream _ _ _ _ _, _ => True
  | .keyed _ _ _ _ _ _ _, _ => True
  | .sing _ _ _ _ _ _ _, _ => True
  | .ksing _ _ _ _ _ _ _ _ _ _, _ => True
  | .atomic _ _, x => TMono x
  | .atomicStream _ _ _ _ _, x => TMono x
  | .atomicKeyed _ _ _ _ _ _ _, x => TMono x
  | .atomicKSing _ _ _ _ _, x => TMono x

def HTMono : {κs : List (Kind L mem)} → HList (Kind.sched (mem := mem)) κs → Prop
  | [], _ => True
  | κ :: _, x => Kind.tmono κ x.1 ∧ HTMono x.2

theorem HTMono.get : {Γ : Ctx L mem} → {κ : Kind L mem} → {x : HList (Kind.sched (mem := mem)) Γ} →
    HTMono x → (v : Var Γ κ) → Kind.tmono κ (x.get v)
  | _ :: _, _, _, hx, .zero => hx.1
  | _ :: _, _, _, hx, .succ v => HTMono.get hx.2 v

theorem HTMono.append : {κs Γ : List (Kind L mem)} → {x : HList (Kind.sched (mem := mem)) κs} →
    {y : HList (Kind.sched (mem := mem)) Γ} → HTMono x → HTMono y → HTMono (x.append y)
  | [], _, _, _, _, hy => hy
  | _ :: _, _, _, _, hx, hy => ⟨hx.1, HTMono.append hx.2 hy⟩

theorem HTMono.take : {κs Γ : List (Kind L mem)} → {x : HList (Kind.sched (mem := mem)) (κs ++ Γ)} →
    HTMono x → HTMono (HList.take (κs := κs) x)
  | [], _, _, _ => trivial
  | _ :: κs, Γ, _, hx => ⟨hx.1, HTMono.take (κs := κs) (Γ := Γ) hx.2⟩

theorem HTMono.drop : {κs Γ : List (Kind L mem)} → {x : HList (Kind.sched (mem := mem)) (κs ++ Γ)} →
    HTMono x → HTMono (HList.drop (κs := κs) x)
  | [], _, _, hx => hx
  | _ :: κs, Γ, _, hx => HTMono.drop (κs := κs) (Γ := Γ) hx.2

theorem HTMono.mapSubst {Γ Γ' : Ctx L mem} {x : Env Γ} (hx : HTMono x) (args : Subst Γ' Γ) :
    HTMono (args.map (fun v => x.get v)) := by
  induction Γ' with
  | nil => trivial
  | cons κ κs ih => exact ⟨hx.get args.1, ih args.2⟩

theorem readShape_tmono {Γ : Ctx L mem} {x : Env Γ} (hx : HTMono x) (ℓ : L) :
    (sh : TickShape) → (vs : VarsOf Γ ℓ sh) → TMonoOf sh (readShape x ℓ sh vs)
  | .sing _, v => hx.get v
  | .stream _ _ _ _, v => hx.get v
  | .ksing _ _ _ _, v => hx.get v
  | .kstream _ _ _ _ _ _, v => hx.get v
  | .pair a b, (va, vb) => ⟨readShape_tmono hx ℓ a va, readShape_tmono hx ℓ b vb⟩

theorem shapeToList_tmono (ℓ : L) : (sh : TickShape) →
    (x : AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) (SchedTick.TKStr mem) ℓ sh) →
    TMonoOf sh x → HTMono (shapeToList ℓ sh x)
  | .sing _, _, hx => ⟨hx, trivial⟩
  | .stream _ _ _ _, _, hx => ⟨hx, trivial⟩
  | .ksing _ _ _ _, _, hx => ⟨hx, trivial⟩
  | .kstream _ _ _ _ _ _, _, hx => ⟨hx, trivial⟩
  | .pair a b, x, hx => HTMono.append (shapeToList_tmono ℓ a x.1 hx.1) (shapeToList_tmono ℓ b x.2 hx.2)

section Mono
variable (pacing : Pacing L mem)

/-- Closing one knot wire preserves tick-monotonicity when the body is
strict and monotonicity-preserving. -/
theorem knotFix_tmono (k : KnotKind L mem) {f : k.toKind.sched → k.toKind.sched}
    (hstrict : k.strictOf f)
    (hmono : ∀ x, Kind.tmono k.toKind x → Kind.tmono k.toKind (f x)) :
    Kind.tmono k.toKind (knotFix pacing k f) := by
  cases k with
  | stream ℓ α i ord ret => trivial
  | atomic ℓ σ => exact tmono_fix_tick (pacing := pacing) () hstrict hmono

theorem bekic_tmono_aux : (ks : List (KnotKind L mem)) →
    {B : HList (Kind.sched (mem := mem)) (ks.map KnotKind.toKind) →
      HList (Kind.sched (mem := mem)) (ks.map KnotKind.toKind)} →
    (Φ₀ : Lags (ks.map KnotKind.toKind) → Lags (ks.map KnotKind.toKind)) →
    LagLaws Φ₀ Φ₀ → cascadeValid (ks.map KnotKind.toKind) Φ₀ = true → BodyCausal Φ₀ B →
    (∀ x, HTMono x → HTMono (B x)) →
    HTMono (bekic pacing ks B)
  | [], _, _, _, _, _, _ => trivial
  | k :: ks, B, Φ₀, hl, hv, hs, hm => by
    obtain ⟨hhead, hv'⟩ := cascadeValid_cons hv
    have hinner : ∀ xs, HTMono xs →
        Kind.tmono k.toKind (knotFix pacing k (fun l => (B (HList.cons l xs)).head)) := by
      intro xs hxs
      exact knotFix_tmono pacing k (KnotKind.strictOf_of hs hhead xs) (fun x hx => (hm _ ⟨hx, hxs⟩).1)
    have hrest : HTMono (bekic pacing ks (fun xs => (B (HList.cons
        (knotFix pacing k (fun l => (B (HList.cons l xs)).head)) xs)).tail)) :=
      bekic_tmono_aux ks (LagFun.next Φ₀) hl.next hv' (BodyCausal.tail pacing hl hhead hs)
        (fun xs hxs => (hm _ ⟨hinner xs hxs, hxs⟩).2)
    exact ⟨hinner _ hrest, hrest⟩

theorem bekic_tmono (ks : List (KnotKind L mem))
    {B : HList (Kind.sched (mem := mem)) (ks.map KnotKind.toKind) →
      HList (Kind.sched (mem := mem)) (ks.map KnotKind.toKind)}
    (hs : HStrict ks B) (hm : ∀ x, HTMono x → HTMono (B x)) : HTMono (bekic pacing ks B) :=
  let ⟨Φ₀, hl, hv, hc⟩ := hs
  bekic_tmono_aux pacing ks Φ₀ hl hv hc hm

mutual

theorem Node.denote_tmono : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) → (sch : Node.Sched n) →
    Node.valid n = true → ∀ (env : Env Γ), HTMono env → HTMono (Node.denote pacing n sch env)
  | _, _, .map .., _, _, _, _ => ⟨trivial, trivial⟩
  | _, _, .filterMap .., _, _, _, _ => ⟨trivial, trivial⟩
  | _, _, .broadcast_closed .., _, _, _, _ => ⟨trivial, trivial⟩
  | _, _, .demux .., _, _, _, _ => ⟨trivial, trivial⟩
  | _, _, .values .., _, _, _, _ => ⟨trivial, trivial⟩
  | _, _, .into_keyed .., _, _, _, _ => ⟨trivial, trivial⟩
  | _, _, .kassume_ordering .., _, _, _, _ => ⟨trivial, trivial⟩
  | _, _, .kfold_early_stop .., _, _, _, _ => ⟨trivial, trivial⟩
  | _, _, .kget_max_key .., _, _, _, _ => ⟨trivial, trivial⟩
  | _, _, .weaken_retries .., _, _, _, _ => ⟨trivial, trivial⟩
  | _, _, .union .., _, _, _, _ => ⟨trivial, trivial⟩
  | _, _, .assume_ordering .., _, _, _, _ => ⟨trivial, trivial⟩
  | _, _, .fold .., _, _, _, _ => ⟨trivial, trivial⟩
  | _, _, .fold_monotone .., _, _, _, _ => ⟨trivial, trivial⟩
  | _, _, .snapshot _ x d, _, _, env, _ => ⟨tmono_snapshot (env.get x) d, trivial⟩
  | _, _, .batch _ _ _ x d, _, _, env, _ => ⟨tmono_batch (env.get x) d, trivial⟩
  | _, _, .source_interval .., _, _, _, _ => ⟨trivial, trivial⟩
  | _, _, .mapTick x f, _, _, env, he => ⟨tmono_mapTick f (he.get x), trivial⟩
  | _, _, .firstTick _ _ x, _, _, env, he => ⟨tmono_firstTick (he.get x), trivial⟩
  | _, _, .zipTick a b, _, _, env, he => ⟨tmono_zipTick (he.get a) (he.get b), trivial⟩
  | _, _, .defer_tick init x, _, _, env, he => ⟨tmono_defer_tick init (he.get x), trivial⟩
  | _, _, .allTicks .., _, _, _, _ => ⟨trivial, trivial⟩
  | _, _, .flattenOrdered _ x, _, _, env, he => ⟨tmono_flattenOrdered (he.get x), trivial⟩
  | _, _, .flattenUnordered _ x, _, _, env, he => ⟨tmono_flattenUnordered (he.get x), trivial⟩
  | _, _, .tinto_keyed _ _ x, _, _, env, he => ⟨tmono_tinto_keyed (he.get x), trivial⟩
  | _, _, .tassume_ordering _ x d, _, _, env, he => ⟨tmono_tassume_ordering d (he.get x), trivial⟩
  | _, _, .reduce_watermark _ _ _ _ x wm f Pre _ ok, _, _, env, he =>
    ⟨tmono_reduce_watermark f Pre ok (he.get x) (he.get wm), trivial⟩
  | _, _, .kentries .., _, _, _, _ => ⟨trivial, trivial⟩
  | _, _, .kmap .., _, _, _, _ => ⟨trivial, trivial⟩
  | _, _, .kallTicks .., _, _, _, _ => ⟨trivial, trivial⟩
  | _, _, .kfold .., _, _, _, _ => ⟨trivial, trivial⟩
  | _, _, .ksnapshot _ _ _ x d, _, _, env, _ => ⟨tmono_ksnapshot (env.get x) d, trivial⟩
  | _, _, .tfold _ g init Pre _ ok x, _, _, env, he => ⟨tmono_tfold g init Pre ok (he.get x), trivial⟩
  | _, _, .latest .., _, _, _, _ => ⟨trivial, trivial⟩
  | _, _, .latest_monotone .., _, _, _, _ => ⟨trivial, trivial⟩
  | _, _, .time, d, _, _, _ => ⟨tmono_time d, trivial⟩
  | _, _, .snapshot_bounded v d, _, _, _, _ => ⟨tmono_snapshot_bounded v d, trivial⟩
  | _, _, .tick_scan (ℓ := ℓ) sts ins outs x g seed, _, _, env, he =>
    shapeToList_tmono ℓ outs _ (tmono_tick_scan sts ins outs g seed (readShape_tmono he ℓ ins x))
  | _, _, .call N args, sch, hv, env, he => Dag.denote_tmono N sch hv _ (he.mapSubst args)
  | _, _, .fixN ks body, sch, hv, env, he => by
    have hvb := (Node.valid_fixN ks body hv).2
    have hcl : HTMono (bekic pacing ks (fun xs => (Dag.denote pacing body sch (xs.append env)).take)) :=
      bekic_tmono pacing ks (Dag.fixN_strict pacing ks body sch hv env)
        (fun xs hxs => (Dag.denote_tmono body sch hvb _ (hxs.append he)).take)
    exact hcl.append (Dag.denote_tmono body sch hvb _ (hcl.append he)).drop

theorem Dag.denote_tmono : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) → (sch : Dag.Sched d) →
    Dag.valid d = true → ∀ (env : Env Γ), HTMono env → HTMono (Dag.denote pacing d sch env)
  | _, _, .ret xs, _, _, _, he => he.mapSubst xs
  | _, _, .letN n rest, sch, hv, env, he =>
    Dag.denote_tmono rest sch.2 (Dag.valid_letN n rest hv).2 _
      ((Node.denote_tmono n sch.1 (Dag.valid_letN n rest hv).1 env he).append he)

end

end Mono

/-! ## Every atomic wire of a graph gains at most one entry per step (D85) -/

/-- Single-slot pacing of a machine wire of a kind: trivially true of
stream/singleton carriers, `TPaced` (by the member's activity) of the
atomic ones. -/
def Kind.tpaced (pacing : Pacing L mem) : (κ : Kind L mem) → κ.sched → Prop
  | .stream _ _ _ _ _, _ => True
  | .keyed _ _ _ _ _ _ _, _ => True
  | .sing _ _ _ _ _ _ _, _ => True
  | .ksing _ _ _ _ _ _ _ _ _ _, _ => True
  | .atomic ℓ _, x => TPaced (pacing.active ℓ) x
  | .atomicStream ℓ _ _ _ _, x => TPaced (pacing.active ℓ) x
  | .atomicKeyed ℓ _ _ _ _ _ _, x => TPaced (pacing.active ℓ) x
  | .atomicKSing ℓ _ _ _ _, x => TPaced (pacing.active ℓ) x

def HTPaced (pacing : Pacing L mem) : {κs : List (Kind L mem)} → HList (Kind.sched (mem := mem)) κs → Prop
  | [], _ => True
  | κ :: _, x => Kind.tpaced pacing κ x.1 ∧ HTPaced pacing x.2

section Paced
variable {pacing : Pacing L mem}

theorem HTPaced.get : {Γ : Ctx L mem} → {κ : Kind L mem} → {x : HList (Kind.sched (mem := mem)) Γ} →
    HTPaced pacing x → (v : Var Γ κ) → Kind.tpaced pacing κ (x.get v)
  | _ :: _, _, _, hx, .zero => hx.1
  | _ :: _, _, _, hx, .succ v => HTPaced.get hx.2 v

theorem HTPaced.append : {κs Γ : List (Kind L mem)} → {x : HList (Kind.sched (mem := mem)) κs} →
    {y : HList (Kind.sched (mem := mem)) Γ} → HTPaced pacing x → HTPaced pacing y → HTPaced pacing (x.append y)
  | [], _, _, _, _, hy => hy
  | _ :: _, _, _, _, hx, hy => ⟨hx.1, HTPaced.append hx.2 hy⟩

theorem HTPaced.take : {κs Γ : List (Kind L mem)} → {x : HList (Kind.sched (mem := mem)) (κs ++ Γ)} →
    HTPaced pacing x → HTPaced pacing (HList.take (κs := κs) x)
  | [], _, _, _ => trivial
  | _ :: κs, Γ, _, hx => ⟨hx.1, HTPaced.take (κs := κs) (Γ := Γ) hx.2⟩

theorem HTPaced.drop : {κs Γ : List (Kind L mem)} → {x : HList (Kind.sched (mem := mem)) (κs ++ Γ)} →
    HTPaced pacing x → HTPaced pacing (HList.drop (κs := κs) x)
  | [], _, _, hx => hx
  | _ :: κs, Γ, _, hx => HTPaced.drop (κs := κs) (Γ := Γ) hx.2

theorem HTPaced.mapSubst {Γ Γ' : Ctx L mem} {x : Env Γ} (hx : HTPaced pacing x) (args : Subst Γ' Γ) :
    HTPaced pacing (args.map (fun v => x.get v)) := by
  induction Γ' with
  | nil => trivial
  | cons κ κs ih => exact ⟨hx.get args.1, ih args.2⟩

theorem readShape_tpaced {Γ : Ctx L mem} {x : Env Γ} (hx : HTPaced pacing x) (ℓ : L) :
    (sh : TickShape) → (vs : VarsOf Γ ℓ sh) → TPacedOf (pacing.active ℓ) sh (readShape x ℓ sh vs)
  | .sing _, v => hx.get v
  | .stream _ _ _ _, v => hx.get v
  | .ksing _ _ _ _, v => hx.get v
  | .kstream _ _ _ _ _ _, v => hx.get v
  | .pair a b, (va, vb) => ⟨readShape_tpaced hx ℓ a va, readShape_tpaced hx ℓ b vb⟩

theorem shapeToList_tpaced (ℓ : L) : (sh : TickShape) →
    (x : AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) (SchedTick.TKStr mem) ℓ sh) →
    TPacedOf (pacing.active ℓ) sh x → HTPaced pacing (shapeToList ℓ sh x)
  | .sing _, _, hx => ⟨hx, trivial⟩
  | .stream _ _ _ _, _, hx => ⟨hx, trivial⟩
  | .ksing _ _ _ _, _, hx => ⟨hx, trivial⟩
  | .kstream _ _ _ _ _ _, _, hx => ⟨hx, trivial⟩
  | .pair a b, x, hx => HTPaced.append (shapeToList_tpaced ℓ a x.1 hx.1) (shapeToList_tpaced ℓ b x.2 hx.2)

/-- The raw tick diagonal of a strict body that preserves pacing on
monotone inputs is paced (`SchedPaced.tpaced_tick_diag` with the body's
pacing-preservation conditioned on monotonicity, as `defer_tick`'s is). -/
theorem tpaced_tick_diag_mono {n : Nat} {σ : Type} {act : Fin n → Nat → Bool}
    {A : (Fin n → Nat → Trace σ) → (Fin n → Nat → Trace σ)}
    (hstrict : TStrict A) (hmono : ∀ x, TMono x → TMono (A x))
    (hA : ∀ x, TMono x → TPaced act x → TPaced act (A x)) :
    TPaced act (fun i t => (iterate A (fun _ _ => ([] : Trace σ)) (t + 1)) i t) := by
  have stages : ∀ m, TPaced act (iterate A (fun _ _ => ([] : Trace σ)) m) := by
    intro m
    induction m with
    | zero => exact TPaced.bot act
    | succ m ih => exact hA _ (TMono.stages hmono m) ih
  intro i k
  let X : Nat → Fin n → Nat → Trace σ := iterate A (fun _ _ => ([] : Trace σ))
  cases k with
  | zero => exact stages 1 i 0
  | succ k =>
    have hstab : X (k + 1) i k = X (k + 2) i k :=
      stab_iterate_tick_strict (h := k) (L := 0) (A := A) (A' := A)
        (fun h' _ x y hxy => hstrict.graded h' x y hxy) k (k + 1) (k + 2) (by omega)
        (Nat.lt_succ_self _) (by omega) i
    have hp := stages (k + 2) i (k + 1)
    show ((X (k + 2) i (k + 1)).drop (X (k + 1) i k).length).length ≤ 1
      ∧ (act i (k + 1) = false → (X (k + 2) i (k + 1)).drop (X (k + 1) i k).length = [])
    rw [hstab]
    exact hp

/-- Closing one knot wire preserves pacing when the body is strict,
monotonicity-preserving and pacing-preserving on monotone inputs. -/
theorem knotFix_tpaced (k : KnotKind L mem) {f : k.toKind.sched → k.toKind.sched}
    (hstrict : k.strictOf f)
    (hmono : ∀ x, Kind.tmono k.toKind x → Kind.tmono k.toKind (f x))
    (hpaced : ∀ x, Kind.tmono k.toKind x → Kind.tpaced pacing k.toKind x → Kind.tpaced pacing k.toKind (f x)) :
    Kind.tpaced pacing k.toKind (knotFix pacing k f) := by
  cases k with
  | stream ℓ α i ord ret => trivial
  | atomic ℓ σ =>
    show TPaced (fun j => pacing.active ℓ j) ((SchedSem L mem pacing).fix_tick () f)
    rw [fix_tick_eq_diag (pacing := pacing) () f]
    exact tpaced_tick_diag_mono (A := f) (act := fun j => pacing.active ℓ j) hstrict hmono hpaced

theorem bekic_tpaced_aux : (ks : List (KnotKind L mem)) →
    {B : HList (Kind.sched (mem := mem)) (ks.map KnotKind.toKind) →
      HList (Kind.sched (mem := mem)) (ks.map KnotKind.toKind)} →
    (Φ₀ : Lags (ks.map KnotKind.toKind) → Lags (ks.map KnotKind.toKind)) →
    LagLaws Φ₀ Φ₀ → cascadeValid (ks.map KnotKind.toKind) Φ₀ = true → BodyCausal Φ₀ B →
    (∀ x, HTMono x → HTMono (B x)) →
    (∀ x, HTMono x → HTPaced pacing x → HTPaced pacing (B x)) →
    HTPaced pacing (bekic pacing ks B)
  | [], _, _, _, _, _, _, _ => trivial
  | k :: ks, B, Φ₀, hl, hv, hs, hm, hp => by
    obtain ⟨hhead, hv'⟩ := cascadeValid_cons hv
    have hinnerM : ∀ xs, HTMono xs →
        Kind.tmono k.toKind (knotFix pacing k (fun l => (B (HList.cons l xs)).head)) := by
      intro xs hxs
      exact knotFix_tmono pacing k (KnotKind.strictOf_of hs hhead xs) (fun x hx => (hm _ ⟨hx, hxs⟩).1)
    have hinner : ∀ xs, HTMono xs → HTPaced pacing xs →
        Kind.tpaced pacing k.toKind (knotFix pacing k (fun l => (B (HList.cons l xs)).head)) := by
      intro xs hxm hxs
      exact knotFix_tpaced k (KnotKind.strictOf_of hs hhead xs) (fun x hx => (hm _ ⟨hx, hxm⟩).1)
        (fun x hxm' hx => (hp _ ⟨hxm', hxm⟩ ⟨hx, hxs⟩).1)
    have hrestM : HTMono (bekic pacing ks (fun xs => (B (HList.cons
        (knotFix pacing k (fun l => (B (HList.cons l xs)).head)) xs)).tail)) :=
      bekic_tmono_aux pacing ks (LagFun.next Φ₀) hl.next hv' (BodyCausal.tail pacing hl hhead hs)
        (fun xs hxs => (hm _ ⟨hinnerM xs hxs, hxs⟩).2)
    have hrest : HTPaced pacing (bekic pacing ks (fun xs => (B (HList.cons
        (knotFix pacing k (fun l => (B (HList.cons l xs)).head)) xs)).tail)) :=
      bekic_tpaced_aux ks (LagFun.next Φ₀) hl.next hv' (BodyCausal.tail pacing hl hhead hs)
        (fun xs hxs => (hm _ ⟨hinnerM xs hxs, hxs⟩).2)
        (fun xs hxm hxs => (hp _ ⟨hinnerM xs hxm, hxm⟩ ⟨hinner xs hxm hxs, hxs⟩).2)
    exact ⟨hinner _ hrestM hrest, hrest⟩

theorem bekic_tpaced (ks : List (KnotKind L mem))
    {B : HList (Kind.sched (mem := mem)) (ks.map KnotKind.toKind) →
      HList (Kind.sched (mem := mem)) (ks.map KnotKind.toKind)}
    (hs : HStrict ks B) (hm : ∀ x, HTMono x → HTMono (B x))
    (hp : ∀ x, HTMono x → HTPaced pacing x → HTPaced pacing (B x)) : HTPaced pacing (bekic pacing ks B) :=
  let ⟨Φ₀, hl, hv, hc⟩ := hs
  bekic_tpaced_aux ks Φ₀ hl hv hc hm hp

mutual

theorem Node.denote_tpaced (pacing : Pacing L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) → (sch : Node.Sched n) →
    Node.valid n = true → ∀ (env : Env Γ), HTMono env → HTPaced pacing env → HTPaced pacing (Node.denote pacing n sch env)
  | _, _, .map .., _, _, _, _, _ => ⟨trivial, trivial⟩
  | _, _, .filterMap .., _, _, _, _, _ => ⟨trivial, trivial⟩
  | _, _, .broadcast_closed .., _, _, _, _, _ => ⟨trivial, trivial⟩
  | _, _, .demux .., _, _, _, _, _ => ⟨trivial, trivial⟩
  | _, _, .values .., _, _, _, _, _ => ⟨trivial, trivial⟩
  | _, _, .into_keyed .., _, _, _, _, _ => ⟨trivial, trivial⟩
  | _, _, .kassume_ordering .., _, _, _, _, _ => ⟨trivial, trivial⟩
  | _, _, .kfold_early_stop .., _, _, _, _, _ => ⟨trivial, trivial⟩
  | _, _, .kget_max_key .., _, _, _, _, _ => ⟨trivial, trivial⟩
  | _, _, .weaken_retries .., _, _, _, _, _ => ⟨trivial, trivial⟩
  | _, _, .union .., _, _, _, _, _ => ⟨trivial, trivial⟩
  | _, _, .assume_ordering .., _, _, _, _, _ => ⟨trivial, trivial⟩
  | _, _, .fold .., _, _, _, _, _ => ⟨trivial, trivial⟩
  | _, _, .fold_monotone .., _, _, _, _, _ => ⟨trivial, trivial⟩
  | _, _, .snapshot _ x d, _, _, env, _, _ => ⟨tpaced_snapshot (env.get x) d, trivial⟩
  | _, _, .batch _ _ _ x d, _, _, env, _, _ => ⟨tpaced_batch (env.get x) d, trivial⟩
  | _, _, .source_interval .., _, _, _, _, _ => ⟨trivial, trivial⟩
  | _, _, .mapTick x f, _, _, env, _, he => ⟨tpaced_mapTick f (he.get x), trivial⟩
  | _, _, .firstTick _ _ x, _, _, env, _, he => ⟨tpaced_firstTick (he.get x), trivial⟩
  | _, _, .zipTick a b, _, _, env, _, he => ⟨tpaced_zipTick (he.get a) (he.get b), trivial⟩
  | _, _, .defer_tick init x, _, _, env, hm, he => ⟨tpaced_defer_tick init (hm.get x) (he.get x), trivial⟩
  | _, _, .allTicks .., _, _, _, _, _ => ⟨trivial, trivial⟩
  | _, _, .flattenOrdered _ x, _, _, env, _, he => ⟨tpaced_flattenOrdered (he.get x), trivial⟩
  | _, _, .flattenUnordered _ x, _, _, env, _, he => ⟨tpaced_flattenUnordered (he.get x), trivial⟩
  | _, _, .tinto_keyed _ _ x, _, _, env, _, he => ⟨tpaced_tinto_keyed (he.get x), trivial⟩
  | _, _, .tassume_ordering _ x d, _, _, env, _, he => ⟨tpaced_tassume_ordering d (he.get x), trivial⟩
  | _, _, .reduce_watermark _ _ _ _ x wm f Pre _ ok, _, _, env, _, he =>
    ⟨tpaced_reduce_watermark f Pre ok (he.get x) (he.get wm), trivial⟩
  | _, _, .kentries .., _, _, _, _, _ => ⟨trivial, trivial⟩
  | _, _, .kmap .., _, _, _, _, _ => ⟨trivial, trivial⟩
  | _, _, .kallTicks .., _, _, _, _, _ => ⟨trivial, trivial⟩
  | _, _, .kfold .., _, _, _, _, _ => ⟨trivial, trivial⟩
  | _, _, .ksnapshot _ _ _ x d, _, _, env, _, _ => ⟨tpaced_ksnapshot (env.get x) d, trivial⟩
  | _, _, .tfold _ g init Pre _ ok x, _, _, env, _, he => ⟨tpaced_tfold g init Pre ok (he.get x), trivial⟩
  | _, _, .latest .., _, _, _, _, _ => ⟨trivial, trivial⟩
  | _, _, .latest_monotone .., _, _, _, _, _ => ⟨trivial, trivial⟩
  | _, _, .time, d, _, _, _, _ => ⟨tpaced_time d, trivial⟩
  | _, _, .snapshot_bounded v d, _, _, _, _, _ => ⟨tpaced_snapshot_bounded v d, trivial⟩
  | _, _, .tick_scan (ℓ := ℓ) sts ins outs x g seed, _, _, env, _, he =>
    shapeToList_tpaced ℓ outs _ (tpaced_tick_scan sts ins outs g seed (readShape_tpaced he ℓ ins x))
  | _, _, .call N args, sch, hv, env, hm, he => Dag.denote_tpaced pacing N sch hv _ (hm.mapSubst args) (he.mapSubst args)
  | _, _, .fixN ks body, sch, hv, env, hm, he => by
    have hvb := (Node.valid_fixN ks body hv).2
    have hs := Dag.fixN_strict pacing ks body sch hv env
    have hclM : HTMono (bekic pacing ks (fun xs => (Dag.denote pacing body sch (xs.append env)).take)) :=
      bekic_tmono pacing ks hs (fun xs hxs => (Dag.denote_tmono pacing body sch hvb _ (hxs.append hm)).take)
    have hcl : HTPaced pacing (bekic pacing ks (fun xs => (Dag.denote pacing body sch (xs.append env)).take)) :=
      bekic_tpaced ks hs
        (fun xs hxs => (Dag.denote_tmono pacing body sch hvb _ (hxs.append hm)).take)
        (fun xs hxm hxs => (Dag.denote_tpaced pacing body sch hvb _ (hxm.append hm) (hxs.append he)).take)
    exact hcl.append (Dag.denote_tpaced pacing body sch hvb _ (hclM.append hm) (hcl.append he)).drop

theorem Dag.denote_tpaced (pacing : Pacing L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) → (sch : Dag.Sched d) →
    Dag.valid d = true → ∀ (env : Env Γ), HTMono env → HTPaced pacing env → HTPaced pacing (Dag.denote pacing d sch env)
  | _, _, .ret xs, _, _, _, _, he => he.mapSubst xs
  | _, _, .letN n rest, sch, hv, env, hm, he =>
    Dag.denote_tpaced pacing rest sch.2 (Dag.valid_letN n rest hv).2 _
      ((Node.denote_tmono pacing n sch.1 (Dag.valid_letN n rest hv).1 env hm).append hm)
      ((Node.denote_tpaced pacing n sch.1 (Dag.valid_letN n rest hv).1 env hm he).append he)

end

end Paced

end Project

end Hydro
