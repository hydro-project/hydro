import Hydro.Project.IR
import Hydro.SchedMono

/-!
# Hydro · Project — causality and tick-monotonicity of every graph

Two facts the stateful interpreter's correctness stands on, proven once
for every `Dag` by structural induction (the per-operator pieces are
`SchedCausal.lean`'s `causal_*` and `SchedMono.lean`'s `tmono_*`, which
the per-module `_causal`/`_smono` phases dispatch today):

- **causality** (`Dag.denote_causal`): a graph's outputs at steps `≤ h`
  are a function of its inputs at steps `≤ h` — what identifies the
  one-step-delayed feedback the interpreter runs with the Kleene
  diagonal `SchedSem.fix_*` is defined by;
- **tick-trace monotonicity** (`Dag.denote_tmono`): every atomic output
  is step-monotone — what makes "the ticks appended at step `t`" a
  suffix and `famFreeze` the identity on `allTicks`/`sample_every`/knots.

Then the **knot characterisations**: for a causal body, the stream knot
satisfies `view 0 = []` and `view (t+1) = (body D).view t`, the tick knot
`D i 0 = []` and `D i (t+1) = body D i t` — the equations the
interpreter's `fixN` cell realises.
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
  | .pair a b, (va, vb) => ⟨readShape_agree hxy ℓ a va, readShape_agree hxy ℓ b vb⟩

theorem shapeToList_agree {h : Nat} (ℓ : L) : (sh : TickShape) →
    (x y : AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) ℓ sh) →
    TAgreeOf h sh x y → HAgree h (shapeToList ℓ sh x) (shapeToList ℓ sh y)
  | .sing _, _, _, hxy => ⟨hxy, trivial⟩
  | .stream _ _ _ _, _, _, hxy => ⟨hxy, trivial⟩
  | .ksing _ _ _ _, _, _, hxy => ⟨hxy, trivial⟩
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

/-! ## Knot closure is causal -/

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

end Knots

/-! ## Every graph is causal -/

section Causal
variable (pacing : Pacing L mem)

mutual

theorem Node.denote_causal : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    ∀ (h : Nat) (env env' : Env Γ), HAgree h env env' →
    HAgree h (Node.denote pacing n env) (Node.denote pacing n env')
  | _, _, .map _ _ x f, h, env, env', he => ⟨causal_map f (he.get x), trivial⟩
  | _, _, .filterMap _ _ x f, h, env, env', he => ⟨causal_filterMap f (he.get x), trivial⟩
  | _, _, .broadcast_closed _ d x, h, env, env', he => ⟨causal_broadcast_closed d (he.get x), trivial⟩
  | _, _, .demux _ d x addr, h, env, env', he => ⟨causal_demux d addr (he.get x), trivial⟩
  | _, _, .values _ _ x, h, env, env', he => ⟨causal_values (he.get x), trivial⟩
  | _, _, .into_keyed _ _ x, h, env, env', he => ⟨causal_into_keyed (he.get x), trivial⟩
  | _, _, .kassume_ordering _ _ x d, h, env, env', he => ⟨causal_kassume_ordering d (he.get x), trivial⟩
  | _, _, .kfold_early_stop _ _ _ x f init, h, env, env', he =>
    ⟨causal_kfold_early_stop f init (he.get x), trivial⟩
  | _, _, .kget_max_key _ _ _ _ x, h, env, env', he => ⟨causal_kget_max_key (he.get x), trivial⟩
  | _, _, .weaken_retries _ x, h, env, env', he => ⟨causal_weaken_retries (he.get x), trivial⟩
  | _, _, .union _ a b, h, env, env', he => ⟨causal_union (he.get a) (he.get b), trivial⟩
  | _, _, .assume_ordering _ x d, h, env, env', he => ⟨causal_assume_ordering d (he.get x), trivial⟩
  | _, _, .fold _ g init Pre _ ok x, h, env, env', he => ⟨causal_fold g init Pre ok (he.get x), trivial⟩
  | _, _, .fold_monotone _ vo g init ok hinfl x, h, env, env', he =>
    ⟨causal_fold_monotone vo g init ok hinfl (he.get x), trivial⟩
  | _, _, .snapshot _ x d, h, env, env', he => ⟨causal_snapshot d (he.get x), trivial⟩
  | _, _, .batch _ x d, h, env, env', he => ⟨causal_batch d (he.get x), trivial⟩
  | _, _, .batch_ordered _ x d, h, env, env', he => ⟨causal_batch_ordered d (he.get x), trivial⟩
  | _, _, .sample_every _ x d, h, env, env', he => ⟨causal_sample_every d (he.get x), trivial⟩
  | _, _, .timeout_snapshot _ x d, h, env, env', he => ⟨causal_timeout_snapshot (s := env.get x) (s' := env'.get x) d, trivial⟩
  | _, _, .source_interval_batch d, h, env, env', he => ⟨causal_source_interval_batch d, trivial⟩
  | _, _, .mapTick x f, h, env, env', he => ⟨causal_mapTick f (he.get x), trivial⟩
  | _, _, .zipTick a b, h, env, env', he => ⟨causal_zipTick (he.get a) (he.get b), trivial⟩
  | _, _, .defer_tick init x, h, env, env', he => ⟨causal_defer_tick init (he.get x), trivial⟩
  | _, _, .allTicks _ x, h, env, env', he => ⟨causal_allTicks (he.get x), trivial⟩
  | _, _, .flattenOrdered _ x, h, env, env', he => ⟨causal_flattenOrdered (he.get x), trivial⟩
  | _, _, .flattenUnordered _ x, h, env, env', he => ⟨causal_flattenUnordered (he.get x), trivial⟩
  | _, _, .tinto_keyed _ _ x, h, env, env', he => ⟨causal_tinto_keyed (he.get x), trivial⟩
  | _, _, .tassume_ordering _ x d, h, env, env', he => ⟨causal_tassume_ordering d (he.get x), trivial⟩
  | _, _, .reduce_watermark _ _ _ _ x wm f Pre _ ok, h, env, env', he =>
    ⟨causal_reduce_watermark f Pre ok (he.get x) (he.get wm), trivial⟩
  | _, _, .tick_scan (ℓ := ℓ) sts ins outs x g seed, h, env, env', he =>
    shapeToList_agree ℓ outs _ _ (causal_tick_scan sts ins outs g seed (readShape_agree he ℓ ins x))
  | _, _, .call N args, h, env, env', he => Dag.denote_causal N h _ _ (he.mapSubst args)
  | _, _, .fixN ks body, h, env, env', he => by
    have hcl : HAgree h (bekic pacing ks (fun xs => (Dag.denote pacing body (xs.append env)).take))
        (bekic pacing ks (fun xs => (Dag.denote pacing body (xs.append env')).take)) :=
      bekic_agree pacing ks (fun h' hh' xs ys hxy =>
        (Dag.denote_causal body h' _ _ (hxy.append (he.mono hh'))).take)
    exact hcl.append (Dag.denote_causal body h _ _ (hcl.append he)).drop

theorem Dag.denote_causal : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    ∀ (h : Nat) (env env' : Env Γ), HAgree h env env' →
    HAgree h (Dag.denote pacing d env) (Dag.denote pacing d env')
  | _, _, .ret xs, h, env, env', he => he.mapSubst xs
  | _, _, .letN n rest, h, env, env', he =>
    Dag.denote_causal rest h _ _ ((Node.denote_causal n h env env' he).append he)

end

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
  | .pair a b, (va, vb) => ⟨readShape_tmono hx ℓ a va, readShape_tmono hx ℓ b vb⟩

theorem shapeToList_tmono (ℓ : L) : (sh : TickShape) →
    (x : AtomicOf (SchedTick.Tk mem) (SchedTick.TS mem) (SchedTick.TKS mem) ℓ sh) →
    TMonoOf sh x → HTMono (shapeToList ℓ sh x)
  | .sing _, _, hx => ⟨hx, trivial⟩
  | .stream _ _ _ _, _, hx => ⟨hx, trivial⟩
  | .ksing _ _ _ _, _, hx => ⟨hx, trivial⟩
  | .pair a b, x, hx => HTMono.append (shapeToList_tmono ℓ a x.1 hx.1) (shapeToList_tmono ℓ b x.2 hx.2)

section Mono
variable (pacing : Pacing L mem)

/-- Closing one knot wire preserves tick-monotonicity when the body is
causal and monotonicity-preserving. -/
theorem knotFix_tmono (k : KnotKind L mem) {f : k.toKind.sched → k.toKind.sched}
    (hcaus : ∀ h' x y, Kind.agree h' k.toKind x y → Kind.agree h' k.toKind (f x) (f y))
    (hmono : ∀ x, Kind.tmono k.toKind x → Kind.tmono k.toKind (f x)) :
    Kind.tmono k.toKind (knotFix pacing k f) := by
  cases k with
  | stream ℓ α i ord ret => trivial
  | atomic ℓ σ => exact tmono_fix_tick (pacing := pacing) () hcaus hmono

theorem bekic_tmono : (ks : List (KnotKind L mem)) →
    {B : HList (Kind.sched (mem := mem)) (ks.map KnotKind.toKind) →
      HList (Kind.sched (mem := mem)) (ks.map KnotKind.toKind)} →
    (∀ h' x y, HAgree h' x y → HAgree h' (B x) (B y)) →
    (∀ x, HTMono x → HTMono (B x)) →
    HTMono (bekic pacing ks B)
  | [], _, _, _ => trivial
  | k :: ks, B, hc, hm => by
    have hinner : ∀ xs, HTMono xs →
        Kind.tmono k.toKind (knotFix pacing k (fun l => (B (HList.cons l xs)).head)) := by
      intro xs hxs
      exact knotFix_tmono pacing k
        (fun h' x y hxy => (hc h' _ _ ⟨hxy, HAgree.refl h' xs⟩).1)
        (fun x hx => (hm _ ⟨hx, hxs⟩).1)
    have hrest : HTMono (bekic pacing ks (fun xs => (B (HList.cons
        (knotFix pacing k (fun l => (B (HList.cons l xs)).head)) xs)).tail)) :=
      bekic_tmono ks
        (fun h' xs ys hxy => (hc h' _ _ ⟨knotFix_agree pacing k (fun h'' hh'' x y hxy' =>
          (hc h'' _ _ ⟨hxy', hxy.mono hh''⟩).head), hxy⟩).2)
        (fun xs hxs => (hm _ ⟨hinner xs hxs, hxs⟩).2)
    exact ⟨hinner _ hrest, hrest⟩

mutual

theorem Node.denote_tmono : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    ∀ (env : Env Γ), HTMono env → HTMono (Node.denote pacing n env)
  | _, _, .map .., _, _ => ⟨trivial, trivial⟩
  | _, _, .filterMap .., _, _ => ⟨trivial, trivial⟩
  | _, _, .broadcast_closed .., _, _ => ⟨trivial, trivial⟩
  | _, _, .demux .., _, _ => ⟨trivial, trivial⟩
  | _, _, .values .., _, _ => ⟨trivial, trivial⟩
  | _, _, .into_keyed .., _, _ => ⟨trivial, trivial⟩
  | _, _, .kassume_ordering .., _, _ => ⟨trivial, trivial⟩
  | _, _, .kfold_early_stop .., _, _ => ⟨trivial, trivial⟩
  | _, _, .kget_max_key .., _, _ => ⟨trivial, trivial⟩
  | _, _, .weaken_retries .., _, _ => ⟨trivial, trivial⟩
  | _, _, .union .., _, _ => ⟨trivial, trivial⟩
  | _, _, .assume_ordering .., _, _ => ⟨trivial, trivial⟩
  | _, _, .fold .., _, _ => ⟨trivial, trivial⟩
  | _, _, .fold_monotone .., _, _ => ⟨trivial, trivial⟩
  | _, _, .snapshot _ x d, env, _ => ⟨tmono_snapshot (env.get x) d, trivial⟩
  | _, _, .batch _ x d, env, _ => ⟨tmono_batch (env.get x) d, trivial⟩
  | _, _, .batch_ordered _ x d, env, _ => ⟨tmono_batch_ordered (env.get x) d, trivial⟩
  | _, _, .sample_every .., _, _ => ⟨trivial, trivial⟩
  | _, _, .timeout_snapshot _ x d, env, _ => ⟨tmono_timeout_snapshot (env.get x) d, trivial⟩
  | _, _, .source_interval_batch d, _, _ => ⟨tmono_source_interval_batch d, trivial⟩
  | _, _, .mapTick x f, env, he => ⟨tmono_mapTick f (he.get x), trivial⟩
  | _, _, .zipTick a b, env, he => ⟨tmono_zipTick (he.get a) (he.get b), trivial⟩
  | _, _, .defer_tick init x, env, he => ⟨tmono_defer_tick init (he.get x), trivial⟩
  | _, _, .allTicks .., _, _ => ⟨trivial, trivial⟩
  | _, _, .flattenOrdered _ x, env, he => ⟨tmono_flattenOrdered (he.get x), trivial⟩
  | _, _, .flattenUnordered _ x, env, he => ⟨tmono_flattenUnordered (he.get x), trivial⟩
  | _, _, .tinto_keyed _ _ x, env, he => ⟨tmono_tinto_keyed (he.get x), trivial⟩
  | _, _, .tassume_ordering _ x d, env, he => ⟨tmono_tassume_ordering d (he.get x), trivial⟩
  | _, _, .reduce_watermark _ _ _ _ x wm f Pre _ ok, env, he =>
    ⟨tmono_reduce_watermark f Pre ok (he.get x) (he.get wm), trivial⟩
  | _, _, .tick_scan (ℓ := ℓ) sts ins outs x g seed, env, he =>
    shapeToList_tmono ℓ outs _ (tmono_tick_scan sts ins outs g seed (readShape_tmono he ℓ ins x))
  | _, _, .call N args, env, he => Dag.denote_tmono N _ (he.mapSubst args)
  | _, _, .fixN ks body, env, he => by
    have hcl : HTMono (bekic pacing ks (fun xs => (Dag.denote pacing body (xs.append env)).take)) :=
      bekic_tmono pacing ks
        (fun h' xs ys hxy => (Dag.denote_causal pacing body h' _ _ (hxy.append (HAgree.refl h' env))).take)
        (fun xs hxs => (Dag.denote_tmono body _ (hxs.append he)).take)
    exact hcl.append (Dag.denote_tmono body _ (hcl.append he)).drop

theorem Dag.denote_tmono : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    ∀ (env : Env Γ), HTMono env → HTMono (Dag.denote pacing d env)
  | _, _, .ret xs, _, he => he.mapSubst xs
  | _, _, .letN n rest, env, he =>
    Dag.denote_tmono rest _ ((Node.denote_tmono n env he).append he)

end

end Mono

end Project

end Hydro
