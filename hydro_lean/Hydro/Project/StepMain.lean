import Hydro.Project.StepOps5

/-!
# Hydro · Project — `Dag.step_spec`: the interpreter reproduces the denotation

Structural induction over the graph. Each operator case is its lemma
(`ProjectStepOps*.lean`); `letN` composes by extending the environment
with the node's denotation, `call` by substitution, `fixN` by the knot
equations (`Project/Knot.lean`): the stored feedback increment is the
closed knot's increment, so stepping the body on it is stepping the body
at the closed knot.

The hypothesis `HTMono env` (every atomic input is tick-monotone) is what
the tick-buffer cells need; `Dag.denote_tmono` propagates it.
-/

namespace Hydro

namespace Project

variable {L : Type} {mem : L → Nat} (pacing : Pacing L mem)

/-! ## Knot increments

A knot wire `x` that is the gated delay of its body output `y`
(`KnotKind.delayed`) is reproduced by the `fixN` cell: the pending
feedback `pendingAt x y t` (the body's output beyond what the knot has
released) is what the machine stores, `feed` releases it exactly as the
knot's increment, and `keep` carries it forward. -/

/-- The knot's view is a prefix of the body's. -/
theorem KnotKind.delayed_prefix {k : KnotKind L mem} {x y : k.toKind.sched} (h : k.delayed pacing x y)
    (hy : Kind.tmono k.toKind y) :
    match k, x, y with
    | .stream ℓ _ _ _ _, x, y => ∀ (i : Fin (mem ℓ)) t, (x i).view t <+: (y i).view t
    | .atomic ℓ _, x, y => ∀ (i : Fin (mem ℓ)) t, x i t <+: y i t := by
  cases k with
  | stream ℓ α i ord ret =>
    intro j t
    induction t with
    | zero => rw [(h j).1]; exact List.nil_prefix
    | succ t ih =>
      rw [(h j).2 t]
      split
      · exact (y j).mono t
      · exact ih.trans ((y j).mono t)
  | atomic ℓ σ =>
    intro j t
    induction t with
    | zero => rw [(h j).1]; exact List.nil_prefix
    | succ t ih =>
      rw [(h j).2 t]
      split
      · exact hy j t (t + 1) (Nat.le_succ t)
      · exact ih.trans (hy j t (t + 1) (Nat.le_succ t))

/-- The knot's increments are the released pending feedback. -/
theorem KnotKind.delayed_feed {k : KnotKind L mem} {x y : k.toKind.sched} (h : k.delayed pacing x y) :
    Kind.incAt k.toKind x 0 = k.feed (ctxAt pacing 0) k.deltaInit ∧
    ∀ t, Kind.incAt k.toKind x (t + 1) = k.feed (ctxAt pacing (t + 1)) (k.pendingAt x y t) := by
  cases k with
  | stream ℓ α i ord ret =>
    refine ⟨funext fun j => ?_, fun t => funext fun j => ?_⟩
    · show (x j).view 0 = if pacing.active ℓ j 0 then [] else []
      rw [(h j).1]; split <;> rfl
    · show ((x j).view (t + 1)).drop ((x j).view t).length
        = if pacing.active ℓ j (t + 1) then ((y j).view t).drop ((x j).view t).length else []
      rw [(h j).2 t]
      split
      · rfl
      · exact List.drop_length
  | atomic ℓ σ =>
    refine ⟨funext fun j => ?_, fun t => funext fun j => ?_⟩
    · show x j 0 = if pacing.active ℓ j 0 then [] else []
      rw [(h j).1]; split <;> rfl
    · show (x j (t + 1)).drop (x j t).length
        = if pacing.active ℓ j (t + 1) then (y j t).drop (x j t).length else []
      rw [(h j).2 t]
      split
      · rfl
      · exact List.drop_length

/-- The pending feedback is carried by `keep`. -/
theorem KnotKind.delayed_keep {k : KnotKind L mem} {x y : k.toKind.sched} (h : k.delayed pacing x y)
    (hy : Kind.tmono k.toKind y) :
    k.pendingAt x y 0 = k.keep (ctxAt pacing 0) k.deltaInit (Kind.incAt k.toKind y 0) ∧
    ∀ t, k.pendingAt x y (t + 1) = k.keep (ctxAt pacing (t + 1)) (k.pendingAt x y t) (Kind.incAt k.toKind y (t + 1)) := by
  have hpre := KnotKind.delayed_prefix pacing h hy
  cases k with
  | stream ℓ α i ord ret =>
    refine ⟨funext fun j => ?_, fun t => funext fun j => ?_⟩
    · show ((y j).view 0).drop ((x j).view 0).length = (if pacing.active ℓ j 0 then [] else []) ++ (y j).view 0
      rw [(h j).1]; split <;> rfl
    · show ((y j).view (t + 1)).drop ((x j).view (t + 1)).length
        = (if pacing.active ℓ j (t + 1) then [] else ((y j).view t).drop ((x j).view t).length)
          ++ ((y j).view (t + 1)).drop ((y j).view t).length
      rw [(h j).2 t]
      split
      · rfl
      · rw [(y j).view_succ t, List.drop_append_of_le_length (hpre j t).length_le, List.drop_left]
  | atomic ℓ σ =>
    refine ⟨funext fun j => ?_, fun t => funext fun j => ?_⟩
    · show (y j 0).drop (x j 0).length = (if pacing.active ℓ j 0 then [] else []) ++ y j 0
      rw [(h j).1]; split <;> rfl
    · show (y j (t + 1)).drop (x j (t + 1)).length
        = (if pacing.active ℓ j (t + 1) then [] else (y j t).drop (x j t).length) ++ (y j (t + 1)).drop (y j t).length
      rw [(h j).2 t]
      split
      · rfl
      · rw [← trace_prev_append_inc (hy j) t, List.drop_append_of_le_length (hpre j t).length_le,
          List.drop_left]

/-- The pending feedback before step `t`. -/
def knotPrevPending (ks : List (KnotKind L mem))
    (x y : HList (Kind.sched (mem := mem)) (ks.map KnotKind.toKind)) :
    Nat → HList (Kind.delta (mem := mem)) (ks.map KnotKind.toKind)
  | 0 => knotDeltaInit ks
  | t + 1 => knotPendingAt ks x y t

theorem HDelayed_feed : (ks : List (KnotKind L mem)) →
    {x y : HList (Kind.sched (mem := mem)) (ks.map KnotKind.toKind)} → HDelayed pacing ks x y →
    ∀ t, knotFeed (ctxAt pacing t) ks (knotPrevPending ks x y t) = envIncAt x t
  | [], _, _, _, _ => rfl
  | k :: ks, x, y, h, t => by
    obtain ⟨h0, hs⟩ := KnotKind.delayed_feed pacing h.1
    cases t with
    | zero =>
      show HList.cons (k.feed _ k.deltaInit) (knotFeed _ ks (knotPrevPending ks x.2 y.2 0))
        = HList.cons (Kind.incAt _ x.1 0) (envIncAt x.2 0)
      rw [h0, HDelayed_feed ks h.2 0]
    | succ t =>
      show HList.cons (k.feed _ (k.pendingAt x.1 y.1 t)) (knotFeed _ ks (knotPrevPending ks x.2 y.2 (t + 1)))
        = HList.cons (Kind.incAt _ x.1 (t + 1)) (envIncAt x.2 (t + 1))
      rw [hs t, HDelayed_feed ks h.2 (t + 1)]

theorem HDelayed_keep : (ks : List (KnotKind L mem)) →
    {x y : HList (Kind.sched (mem := mem)) (ks.map KnotKind.toKind)} → HDelayed pacing ks x y → HTMono y →
    ∀ t, knotKeep (ctxAt pacing t) ks (knotPrevPending ks x y t) (envIncAt y t) = knotPendingAt ks x y t
  | [], _, _, _, _, _ => rfl
  | k :: ks, x, y, h, hy, t => by
    obtain ⟨h0, hs⟩ := KnotKind.delayed_keep pacing h.1 hy.1
    cases t with
    | zero =>
      show HList.cons (k.keep _ k.deltaInit (Kind.incAt _ y.1 0))
          (knotKeep _ ks (knotPrevPending ks x.2 y.2 0) (envIncAt y.2 0))
        = HList.cons (k.pendingAt x.1 y.1 0) (knotPendingAt ks x.2 y.2 0)
      rw [h0, HDelayed_keep ks h.2 hy.2 0]
    | succ t =>
      show HList.cons (k.keep _ (k.pendingAt x.1 y.1 t) (Kind.incAt _ y.1 (t + 1)))
          (knotKeep _ ks (knotPrevPending ks x.2 y.2 (t + 1)) (envIncAt y.2 (t + 1)))
        = HList.cons (k.pendingAt x.1 y.1 (t + 1)) (knotPendingAt ks x.2 y.2 (t + 1))
      rw [hs t, HDelayed_keep ks h.2 hy.2 (t + 1)]

/-! ## The structural theorem -/

/-- The cells before step `t` of a graph, structurally. -/
theorem Dag.prevState_letN {Γ : Ctx L mem} {κs outs : List (Kind L mem)} (n : Node Γ κs)
    (rest : Dag (κs ++ Γ) outs) (env : Env Γ) (t : Nat)
    (sch : Dag.Sched (.letN n rest)) :
    Dag.prevState pacing (.letN n rest) sch env t
      = (Node.prevState pacing n sch.1 env t, Dag.prevState pacing rest sch.2 ((Node.denote pacing n sch.1 env).append env) t) := by
  cases t <;> rfl

theorem Node.prevState_call {Γ Γ' : Ctx L mem} {outs : List (Kind L mem)} (N : Dag Γ' outs) (args : Subst Γ' Γ)
    (sch : Node.Sched (.call N args)) (env : Env Γ) (t : Nat) :
    Node.prevState pacing (.call N args) sch env t = Dag.prevState pacing N sch (args.map (fun v => env.get v)) t := by
  cases t <;> rfl

theorem Node.prevState_fixN {Γ : Ctx L mem} (ks : List (KnotKind L mem)) {outs : List (Kind L mem)}
    (body : Dag (ks.map KnotKind.toKind ++ Γ) (ks.map KnotKind.toKind ++ outs))
    (sch : Node.Sched (.fixN ks body)) (env : Env Γ) (t : Nat) :
    Node.prevState pacing (.fixN ks body) sch env t
      = (Dag.prevState pacing body sch
          ((bekic pacing ks (fun xs => (Dag.denote pacing body sch (xs.append env)).take)).append env) t,
         knotPrevPending ks (bekic pacing ks (fun xs => (Dag.denote pacing body sch (xs.append env)).take))
           (Dag.denote pacing body sch
             ((bekic pacing ks (fun xs => (Dag.denote pacing body sch (xs.append env)).take)).append env)).take t) := by
  cases t <;> rfl

mutual

theorem Node.step_spec : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) → (sch : Node.Sched n) →
    ∀ (env : Env Γ), HTMono env → ∀ (t : Nat), NodeSpec pacing n sch env t
  | _, _, .map iα iβ x f, _, env, _, t => spec_map pacing iα iβ x f env t
  | _, _, .filterMap iα iβ x f, _, env, _, t => spec_filterMap pacing iα iβ x f env t
  | _, _, .broadcast_closed iα x, d, env, _, t => spec_broadcast_closed pacing iα d x env t
  | _, _, .demux iα x addr, d, env, _, t => spec_demux pacing iα d x addr env t
  | _, _, .values iK iV x, _, env, _, t => spec_values pacing iK iV x env t
  | _, _, .into_keyed iK iV x, _, env, _, t => spec_into_keyed pacing iK iV x env t
  | _, _, .kassume_ordering iK iV x d, _, env, _, t => spec_kassume_ordering pacing iK iV x d env t
  | _, _, .kfold_early_stop iK iV iA x f init, _, env, _, t => spec_kfold_early_stop pacing iK iV iA x f init env t
  | _, _, .kget_max_key iα iK iV lo x, _, env, _, t => spec_kget_max_key pacing iα iK iV lo x env t
  | _, _, .weaken_retries iα x, _, env, _, t => spec_weaken_retries pacing iα x env t
  | _, _, .union iα a b, _, env, _, t => spec_union pacing iα a b env t
  | _, _, .assume_ordering iα x d, _, env, _, t => spec_assume_ordering pacing iα x d env t
  | _, _, .fold iα g init Pre iPre ok x, _, env, _, t => spec_fold pacing iα g init Pre iPre ok x env t
  | _, _, .fold_monotone iα vo g init ok hinfl x, _, env, _, t =>
    spec_fold_monotone pacing iα vo g init ok hinfl x env t
  | _, _, .snapshot iα x d, _, env, _, t => spec_snapshot pacing iα x d env t
  | _, _, .batch iα x d, _, env, _, t => spec_batch pacing iα x d env t
  | _, _, .batch_ordered iα x d, _, env, _, t => spec_batch_ordered pacing iα x d env t
  | _, _, .sample_every iα x, d, env, he, t => spec_sample_every pacing iα x d env t (he.get x)
  | _, _, .timeout_snapshot iα x, d, env, _, t => spec_timeout_snapshot pacing iα x d env t
  | _, _, .source_interval_batch, d, env, _, t => spec_source_interval_batch pacing d env t
  | _, _, .mapTick x f, _, env, _, t => spec_mapTick pacing x f env t
  | _, _, .zipTick a b, _, env, he, t => spec_zipTick pacing a b env t (he.get a) (he.get b)
  | _, _, .defer_tick init x, _, env, he, t => spec_defer_tick pacing init x env (he.get x) t
  | _, _, .allTicks iβ x, _, env, he, t => spec_allTicks pacing iβ x env t (he.get x)
  | _, _, .flattenOrdered iβ x, _, env, _, t => spec_flattenOrdered pacing iβ x env t
  | _, _, .flattenUnordered iβ x, _, env, _, t => spec_flattenUnordered pacing iβ x env t
  | _, _, .tinto_keyed iK iV x, _, env, _, t => spec_tinto_keyed pacing iK iV x env t
  | _, _, .tassume_ordering iα x d, _, env, _, t => spec_tassume_ordering pacing iα x d env t
  | _, _, .reduce_watermark iK iH iV lo x wm f Pre iPre ok, _, env, he, t =>
    spec_reduce_watermark pacing iK iH iV lo x wm f Pre iPre ok env t (he.get x) (he.get wm)
  | _, _, .tick_scan (ℓ := ℓ) sts ins outs x g seed, _, env, he, t =>
    spec_tick_scan pacing sts ins outs x g seed env t (readShape_tmono he ℓ ins x)
  | _, _, .call N args, sch, env, he, t => by
    show Node.step (ctxAt pacing t) (.call N args) sch (envIncAt env t) (Node.prevState pacing (.call N args) sch env t) = _
    rw [Node.prevState_call]
    simp only [Node.step, Node.denote, Node.stateOf]
    rw [← envIncAt_mapSubst]
    exact Dag.step_spec N sch _ (he.mapSubst args) t
  | _, _, .fixN ks body, sch, env, he, t => by
    show Node.step (ctxAt pacing t) (.fixN ks body) sch (envIncAt env t) (Node.prevState pacing (.fixN ks body) sch env t) = _
    rw [Node.prevState_fixN]
    simp only [Node.step, Node.denote, Node.stateOf]
    -- the knot equations
    have hdel := bekic_delayed pacing ks (B := fun xs => (Dag.denote pacing body sch (xs.append env)).take)
      (fun h' x y hxy => (Dag.denote_causal pacing body sch h' _ _ (hxy.append (HAgree.refl h' env))).take)
    -- the closed knots are tick-monotone
    have hcl : HTMono (bekic pacing ks (fun xs => (Dag.denote pacing body sch (xs.append env)).take)) :=
      bekic_tmono pacing ks
        (fun h' xs ys hxy => (Dag.denote_causal pacing body sch h' _ _ (hxy.append (HAgree.refl h' env))).take)
        (fun xs hxs => (Dag.denote_tmono pacing body sch _ (hxs.append he)).take)
    have hbody : HTMono (Dag.denote pacing body sch
        ((bekic pacing ks (fun xs => (Dag.denote pacing body sch (xs.append env)).take)).append env)).take :=
      (Dag.denote_tmono pacing body sch _ (hcl.append he)).take
    rw [HDelayed_feed pacing ks hdel t, ← envIncAt_append]
    have ih := Dag.step_spec body sch _ (hcl.append he) t
    rw [ih]
    show ((Dag.stateOf pacing body sch _ t,
          knotKeep (ctxAt pacing t) ks (knotPrevPending ks _ _ t) (envIncAt (Dag.denote pacing body sch _) t).take),
        (envIncAt _ t).append (envIncAt (Dag.denote pacing body sch _) t).drop) = _
    rw [← envIncAt_take, HDelayed_keep pacing ks hdel hbody t, ← envIncAt_drop, ← envIncAt_append]
    rfl

theorem Dag.step_spec : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) → (sch : Dag.Sched d) →
    ∀ (env : Env Γ), HTMono env → ∀ (t : Nat),
    Dag.step (ctxAt pacing t) d sch (envIncAt env t) (Dag.prevState pacing d sch env t)
      = (Dag.stateOf pacing d sch env t, envIncAt (Dag.denote pacing d sch env) t)
  | _, _, .ret xs, sch, env, _, t => by
    show (Dag.prevState pacing (.ret xs) sch env t, xs.map (fun v => (envIncAt env t).get v))
      = ((), envIncAt (xs.map (fun v => env.get v)) t)
    rw [envIncAt_mapSubst]
    cases t <;> rfl
  | _, _, .letN n rest, sch, env, he, t => by
    rw [Dag.prevState_letN]
    simp only [Dag.step, Dag.denote, Dag.stateOf]
    have hn := Node.step_spec n sch.1 env he t
    simp only [NodeSpec] at hn
    rw [hn, ← envIncAt_append]
    rw [Dag.step_spec rest sch.2 _ ((Node.denote_tmono pacing n sch.1 env he).append he) t]

end

/-! ## The lock-step run -/

/-- **The lock-step machine reproduces the machine semantics**: after
step `t` its cells are the prescribed ones and its outputs at `t` are
the denotation's increments at `t`. -/
theorem Dag.run_spec {Γ : Ctx L mem} {outs : List (Kind L mem)} (d : Dag Γ outs) (sch : Dag.Sched d) (env : Env Γ)
    (he : HTMono env) (t : Nat) :
    Dag.run d sch pacing (fun t => envIncAt env t) t
      = (Dag.stateOf pacing d sch env t, envIncAt (Dag.denote pacing d sch env) t) := by
  induction t with
  | zero => exact Dag.step_spec pacing d sch env he 0
  | succ t ih =>
    show Dag.step (ctxAt pacing (t + 1)) d sch (envIncAt env (t + 1)) (Dag.run d sch pacing _ t).1 = _
    rw [ih]
    exact Dag.step_spec pacing d sch env he (t + 1)

end Project

end Hydro
