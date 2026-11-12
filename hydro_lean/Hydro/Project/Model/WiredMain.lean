import Hydro.Project.Model.WiredSpec

/-!
# Hydro · Project — wired machine: one step is one process

`Node.step_wired`/`Dag.step_wired`: a lock-step step in which the process `m`'s members and no
others are active (`ctx.act = m.hosts`) and tick iff the process ticks
(`ctx.tick = fun ℓ i => m.hosts ℓ i && tk`), on an environment whose increments are empty
off the process (`IdleEnv`) and with an oracle that reads the schedule at `ctx.t`
(`OracleAt`), equals the process's own `Dag.locStep m tk` written back over the state
(`Dag.override`) with its sends appended to every member's queues (`Dag.enqueue` for the
global state, `Dag.enqueueLoc` for the process's own cells). The step's increments cut to the
process are the process's (`envRestrict`), and are empty at every other member (`IdleEnv`).

The only condition on the state is `Drained`: the multi-input tick cells hold no complete
item, which every state the machine produces satisfies (`Dag.init_drained`,
`Dag.step_drained`). Nothing is assumed about the other processes: an inactive member is
inert, and its channel cells only enqueue. A knot block's step is its body's at the knot
increments the body produces, on both sides: the lock-step `fixN` and the process's each
iterate the body's step on the knot increments `ks.length` rounds, so the case reduces to the
body's, round by round (`hround`).
-/

namespace Hydro

namespace Project

variable {L : Type} {mem : L → Nat}

/-- The conclusion of a wired step for a node: with `r` the process's `Node.locStep` on the cut
environment and cells, the lock-step cells are `r`'s written back with `r`'s sends enqueued
everywhere, the lock-step outputs cut to the process are `r`'s outputs, and they are idle off
the process. -/
def NodeWired [DecidableEq L] (ctx : StepCtx L mem) (m : Member L mem) (tk : Bool) {Γ : Ctx L mem} {κs : List (Kind L mem)}
    (n : Node Γ κs) (sch : Node.Sched n) (ora : Node.Oracle m n) (Δ : HList (Kind.delta (mem := mem)) Γ)
    (s : Node.State n) : Prop :=
  let r := Node.locStep m tk n ora (envRestrict m Δ) (Node.restrict m n s)
  (Node.step ctx n sch Δ s).1 = Node.override m n (Node.enqueue m n r.2.2 s) (Node.enqueueLoc m m n r.2.2 r.1) ∧
  envRestrict m (Node.step ctx n sch Δ s).2 = r.2.1 ∧
  IdleEnv m (Node.step ctx n sch Δ s).2

/-- `NodeWired` for a graph. -/
def DagWired [DecidableEq L] (ctx : StepCtx L mem) (m : Member L mem) (tk : Bool) {Γ : Ctx L mem} {outs : List (Kind L mem)}
    (d : Dag Γ outs) (sch : Dag.Sched d) (ora : Dag.Oracle m d) (Δ : HList (Kind.delta (mem := mem)) Γ)
    (s : Dag.State d) : Prop :=
  let r := Dag.locStep m tk d ora (envRestrict m Δ) (Dag.restrict m d s)
  (Dag.step ctx d sch Δ s).1 = Dag.override m d (Dag.enqueue m d r.2.2 s) (Dag.enqueueLoc m m d r.2.2 r.1) ∧
  envRestrict m (Dag.step ctx d sch Δ s).2 = r.2.1 ∧
  IdleEnv m (Dag.step ctx d sch Δ s).2

/-! ## Drained cells: every state the machine produces -/

mutual

/-- The initial cells are drained: the tick buffers start empty. -/
theorem Node.init_drained : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) → (sch : Node.Sched n) →
    Node.Drained n sch (Node.init n)
  | _, _, .zipTick .., _ => fun _ => Or.inl rfl
  | _, _, .reduce_watermark .., _ => fun _ => Or.inl rfl
  | _, _, .tick_scan _ ins _ _ _ _, _ => fun _ => PendingOf.avail_empty ins
  | _, _, .call N _, sch => Dag.init_drained N sch
  | _, _, .fixN _ body, sch => Dag.init_drained body sch
  | _, _, .map .., _ => trivial
  | _, _, .filterMap .., _ => trivial
  | _, _, .broadcast_closed .., _ => trivial
  | _, _, .demux .., _ => trivial
  | _, _, .values .., _ => trivial
  | _, _, .into_keyed .., _ => trivial
  | _, _, .kassume_ordering .., _ => trivial
  | _, _, .kfold_early_stop .., _ => trivial
  | _, _, .kget_max_key .., _ => trivial
  | _, _, .weaken_retries .., _ => trivial
  | _, _, .union .., _ => trivial
  | _, _, .assume_ordering .., _ => trivial
  | _, _, .fold .., _ => trivial
  | _, _, .fold_monotone .., _ => trivial
  | _, _, .snapshot .., _ => trivial
  | _, _, .batch .., _ => trivial
  | _, _, .source_interval .., _ => trivial
  | _, _, .mapTick .., _ => trivial
  | _, _, .firstTick .., _ => trivial
  | _, _, .defer_tick .., _ => trivial
  | _, _, .allTicks .., _ => trivial
  | _, _, .flattenOrdered .., _ => trivial
  | _, _, .flattenUnordered .., _ => trivial
  | _, _, .tinto_keyed .., _ => trivial
  | _, _, .tassume_ordering .., _ => trivial
  | _, _, .kentries .., _ => trivial
  | _, _, .kmap .., _ => trivial
  | _, _, .kallTicks .., _ => trivial
  | _, _, .kfold .., _ => trivial
  | _, _, .ksnapshot .., _ => trivial
  | _, _, .tfold .., _ => trivial
  | _, _, .latest .., _ => trivial
  | _, _, .latest_monotone .., _ => trivial
  | _, _, .time .., _ => trivial
  | _, _, .snapshot_bounded .., _ => trivial

/-- `Node.init_drained` along the `letN` chain. -/
theorem Dag.init_drained : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) → (sch : Dag.Sched d) →
    Dag.Drained d sch (Dag.init d)
  | _, _, .ret _, _ => trivial
  | _, _, .letN n rest, sch => ⟨Node.init_drained n sch.1, Dag.init_drained rest sch.2⟩

end

mutual

/-- After any step the multi-input tick cells are drained: a pair step drops the shorter length
from both sides (`zip_drop_min_empty`), a tick block drops every available tuple
(`PendingOf.avail_dropN_self`). -/
theorem Node.step_drained (ctx : StepCtx L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    (sch : Node.Sched n) → (Δ : HList (Kind.delta (mem := mem)) Γ) → (s : Node.State n) →
    Node.Drained n sch (Node.step ctx n sch Δ s).1
  | _, _, .zipTick a b, _, Δ, s => fun i => by
    simp only [Node.step]
    exact zip_drop_min_empty _ _
  | _, _, .reduce_watermark _ _ _ _ x wm f _ _ _, _, Δ, s => fun i => by
    simp only [Node.step]
    exact zip_drop_min_empty _ _
  | _, _, .tick_scan _ ins _ x g _, _, Δ, s => fun i => by
    simp only [Node.step]
    exact PendingOf.avail_dropN_self ins _
  | _, _, .call N args, sch, Δ, s => Dag.step_drained ctx N sch (args.map (fun v => Δ.get v)) s
  | _, _, .fixN ks body, sch, Δ, s => Dag.step_drained ctx body sch _ s
  | _, _, .map .., _, _, _ => trivial
  | _, _, .filterMap .., _, _, _ => trivial
  | _, _, .broadcast_closed .., _, _, _ => trivial
  | _, _, .demux .., _, _, _ => trivial
  | _, _, .values .., _, _, _ => trivial
  | _, _, .into_keyed .., _, _, _ => trivial
  | _, _, .kassume_ordering .., _, _, _ => trivial
  | _, _, .kfold_early_stop .., _, _, _ => trivial
  | _, _, .kget_max_key .., _, _, _ => trivial
  | _, _, .weaken_retries .., _, _, _ => trivial
  | _, _, .union .., _, _, _ => trivial
  | _, _, .assume_ordering .., _, _, _ => trivial
  | _, _, .fold .., _, _, _ => trivial
  | _, _, .fold_monotone .., _, _, _ => trivial
  | _, _, .snapshot .., _, _, _ => trivial
  | _, _, .batch .., _, _, _ => trivial
  | _, _, .source_interval .., _, _, _ => trivial
  | _, _, .mapTick .., _, _, _ => trivial
  | _, _, .firstTick .., _, _, _ => trivial
  | _, _, .defer_tick .., _, _, _ => trivial
  | _, _, .allTicks .., _, _, _ => trivial
  | _, _, .flattenOrdered .., _, _, _ => trivial
  | _, _, .flattenUnordered .., _, _, _ => trivial
  | _, _, .tinto_keyed .., _, _, _ => trivial
  | _, _, .tassume_ordering .., _, _, _ => trivial
  | _, _, .kentries .., _, _, _ => trivial
  | _, _, .kmap .., _, _, _ => trivial
  | _, _, .kallTicks .., _, _, _ => trivial
  | _, _, .kfold .., _, _, _ => trivial
  | _, _, .ksnapshot .., _, _, _ => trivial
  | _, _, .tfold .., _, _, _ => trivial
  | _, _, .latest .., _, _, _ => trivial
  | _, _, .latest_monotone .., _, _, _ => trivial
  | _, _, .time .., _, _, _ => trivial
  | _, _, .snapshot_bounded .., _, _, _ => trivial

/-- `Node.step_drained` along the `letN` chain. -/
theorem Dag.step_drained (ctx : StepCtx L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    (sch : Dag.Sched d) → (Δ : HList (Kind.delta (mem := mem)) Γ) → (s : Dag.State d) →
    Dag.Drained d sch (Dag.step ctx d sch Δ s).1
  | _, _, .ret _, _, _, _ => trivial
  | _, _, .letN n rest, sch, Δ, s => ⟨Node.step_drained ctx n sch.1 Δ s.1, Dag.step_drained ctx rest sch.2 _ s.2⟩

end

/-! ## Channel cells -/

/-- A channel step with sends is the step without them followed by `enqueue`: the release reads
the pending suffix before the sends land. -/
theorem ChanCell.step_enqueue {α : Type} (cell : ChanCell α) (c : Nat) (sends : Array α) :
    cell.step c sends = ((cell.step c #[]).1.enqueue sends, (cell.step c #[]).2) := by
  simp [ChanCell.step, ChanCell.enqueue]

/-- Enqueueing nothing changes nothing. -/
theorem ChanCell.enqueue_nil {α : Type} (cell : ChanCell α) : cell.enqueue #[] = cell := by
  simp [ChanCell.enqueue]

variable [DecidableEq L]

/-- The empty knot increments cut to a process are the process's empty knot increments. -/
theorem knotFill_restrict (m : Member L mem) : (ks : List (KnotKind L mem)) →
    envRestrict m (knotFill ks) = knotFillLoc m ks
  | [] => rfl
  | k :: ks => by
    show (Kind.restrictDelta m _ k.fill, envRestrict m (knotFill ks)) = (k.fillLoc m, knotFillLoc m ks)
    rw [knotFill_restrict m ks]
    cases k <;> rfl

/-- The empty knot increments are idle at every member. -/
theorem knotFill_idle (m : Member L mem) : (ks : List (KnotKind L mem)) → IdleEnv m (knotFill ks)
  | [] => trivial
  | k :: ks => ⟨by cases k <;> exact fun _ _ => rfl, knotFill_idle m ks⟩

/-- `override_pointwise` with the hosted cells given separately: a per-member cell map equal to
the process's cells `l` at hosted members (`hhost`) and to `s` at the others (`hidle`) is the
write-back of `l` over `s`. -/
theorem override_pointwise' (m : Member L mem) (ℓ : L) {X : Type} (s F : Fin (mem ℓ) → X)
    (l : Hosted m ℓ → X) (hhost : ∀ (i : Hosted m ℓ), F i.1 = l i)
    (hidle : ∀ i, (⟨ℓ, i⟩ : Member L mem) ≠ m → F i = s i) :
    (fun i => if h : (⟨ℓ, i⟩ : Member L mem) = m then l ⟨i, h⟩ else s i) = F := by
  funext i
  split
  · rename_i h; exact (hhost ⟨i, h⟩).symm
  · rename_i hn
    exact (hidle i hn).symm

section Main

variable (ctx : StepCtx L mem) (m : Member L mem) (tk : Bool) (hact : ctx.act = m.hosts)
  (htick : ctx.tick = fun ℓ i => m.hosts ℓ i && tk)
include hact htick

/-! `hact` and `htick` read at a member: hosted members are active and tick iff `tk`; the others
are inactive and do not tick. -/

omit htick in
theorem act_hosted (ℓ : L) (i : Hosted m ℓ) : ctx.act ℓ i.1 = true := by rw [hact]; simpa using i.2
omit htick in
theorem act_idle (ℓ : L) (i : Fin (mem ℓ)) (hi : (⟨ℓ, i⟩ : Member L mem) ≠ m) : ctx.act ℓ i = false := by rw [hact]; simpa using hi
omit hact in
theorem tick_hosted (ℓ : L) (i : Hosted m ℓ) : ctx.tick ℓ i.1 = tk := by rw [htick]; simp [i.2]
omit hact in
theorem tick_idle (ℓ : L) (i : Fin (mem ℓ)) (hi : (⟨ℓ, i⟩ : Member L mem) ≠ m) : ctx.tick ℓ i = false := by rw [htick]; simp [hi]

omit htick in
/-- The channel cells of a network node after a wired step, per receiver. A hosted receiver
releases per the oracle (the process's step on the pending suffix) and then enqueues its own
sends, the loopback queue; another receiver only enqueues the sends addressed to it, and
delivers nothing. `hidleS` says the global sends are idle, `hsends`/`hsendsI` relate the
process's sends to them. -/
theorem chan_cells_wired {c p : L} {α : Type} (d : Fin (mem p) → Fin (mem c) → Nat → Nat)
    (sendsG : Fin (mem c) → Array α) (s : Fin (mem p) → Fin (mem c) → ChanCell α)
    (ora : Hosted m p → Fin (mem c) → Nat) (sendsL : Fin (mem c) → Array α)
    (hora : ∀ i j, ora i j = cumMax (d i.1 j) ctx.t)
    (hidleS : ∀ j, (⟨c, j⟩ : Member L mem) ≠ m → sendsG j = #[])
    (hsends : ∀ j, (⟨c, j⟩ : Member L mem) = m → sendsL j = sendsG j)
    (hsendsI : ∀ j, (⟨c, j⟩ : Member L mem) ≠ m → sendsL j = #[]) :
    (∀ (i : Hosted m p) j, (s i.1 j).stepG (ctx.act p i.1) (cumMax (d i.1 j) ctx.t) (sendsG j)
      = (((s i.1 j).step (ora i j) #[]).1.enqueue (sendsL j), ((s i.1 j).step (ora i j) #[]).2)) ∧
    (∀ i j, (⟨p, i⟩ : Member L mem) ≠ m →
      ((s i j).stepG (ctx.act p i) (cumMax (d i j) ctx.t) (sendsG j)).1
        = if _ : (⟨c, j⟩ : Member L mem) = m then (s i j).enqueue (sendsG j) else s i j) ∧
    (∀ i j, (⟨p, i⟩ : Member L mem) ≠ m → ((s i j).stepG (ctx.act p i) (cumMax (d i j) ctx.t) (sendsG j)).2 = #[]) := by
  refine ⟨fun i j => ?_, fun i j hi => ?_, fun i j hi => ?_⟩
  · rw [act_hosted ctx m hact p i, hora]
    simp only [ChanCell.stepG, if_true]
    rw [ChanCell.step_enqueue]
    by_cases hj : (⟨c, j⟩ : Member L mem) = m
    · rw [hsends j hj]
    · rw [hsendsI j hj, hidleS j hj]
  · rw [act_idle ctx m hact p i hi]
    simp only [ChanCell.stepG, Bool.false_eq_true, if_false]
    split
    · rfl
    · rename_i hj
      rw [hidleS j hj]
      exact ChanCell.enqueue_nil _
  · rw [act_idle ctx m hact p i hi]
    simp only [ChanCell.stepG, Bool.false_eq_true, if_false]

-- `hact`/`htick` are used by the node theorem; the graph theorem of the mutual block carries
-- them along unused
set_option linter.unusedSectionVars false

/-! The keyed wires, `tfold`, `latest`, `time` and `snapshot_bounded` as standalone wired-step
lemmas. They do not recurse, so keeping them out of the mutual theorem keeps its elaboration
budget flat. Each has three parts: the cells (a write-back, `override_pointwise`), the cut
outputs, and the idleness of the outputs off the process. -/

/-- `kentries`, wired. -/
theorem wired_kentries {Γ : Ctx L mem} {ℓ : L} {K V : Type} (iK : DecidableEq K) (iV : DecidableEq V) {ord : StrOrd}
    (x : Var Γ (.keyed ℓ K V iK iV ord .exactlyOnce)) (Δ : HList (Kind.delta (mem := mem)) Γ) (s : Node.State (.kentries iK iV x))
    (hΔ : IdleEnv m Δ) :
    NodeWired ctx m tk (.kentries iK iV x) () () Δ s := by
  refine ⟨?_, ?_, ?_⟩
  · simp only [Node.step, Node.override, Node.enqueue]
  · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get_stream, envRestrict_get_keyed, envRestrict_get, Kind.restrictDelta]
    simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
    try rfl
  · simp only [Node.step, IdleEnv, Kind.idleDelta]
    exact ⟨fun i hi => IdleEnv.get_keyed m hΔ x i hi, trivial⟩

/-- `kmap`, wired. -/
theorem wired_kmap {Γ : Ctx L mem} {ℓ : L} {K V W : Type} (iK : DecidableEq K) (iV : DecidableEq V) (iW : DecidableEq W) {ord : StrOrd}
    (x : Var Γ (.keyed ℓ K V iK iV ord .exactlyOnce)) (f : Fin (mem ℓ) → V → W) (Δ : HList (Kind.delta (mem := mem)) Γ)
    (s : Node.State (.kmap iK iV iW x f)) (hΔ : IdleEnv m Δ) :
    NodeWired ctx m tk (.kmap iK iV iW x f) () () Δ s := by
  refine ⟨?_, ?_, ?_⟩
  · simp only [Node.step, Node.override, Node.enqueue]
  · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get_stream, envRestrict_get_keyed, envRestrict_get, Kind.restrictDelta]
    simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
    try rfl
  · simp only [Node.step, IdleEnv, Kind.idleDelta]
    exact ⟨fun i hi => by rw [IdleEnv.get_keyed m hΔ x i hi]; simp, trivial⟩

/-- `kallTicks`, wired. -/
theorem wired_kallTicks {Γ : Ctx L mem} {ℓ : L} {K V : Type} (iK : DecidableEq K) (iV : DecidableEq V) {ord : StrOrd}
    (x : Var Γ (.atomicKeyed ℓ K V iK iV ord .exactlyOnce)) (Δ : HList (Kind.delta (mem := mem)) Γ) (s : Node.State (.kallTicks iK iV x))
    (hΔ : IdleEnv m Δ) :
    NodeWired ctx m tk (.kallTicks iK iV x) () () Δ s := by
  refine ⟨?_, ?_, ?_⟩
  · simp only [Node.step, Node.override, Node.enqueue]
  · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get_stream, envRestrict_get_keyed, envRestrict_get, Kind.restrictDelta]
    simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
    try rfl
  · simp only [Node.step, IdleEnv, Kind.idleDelta]
    exact ⟨fun i hi => by rw [IdleEnv.get_atomicKeyed m hΔ x i hi]; rfl, trivial⟩

/-- `ksnapshot`, wired: the tick read at a hosted member is `tk` (`tick_hosted`), and `none`
elsewhere (`tick_idle`). -/
theorem wired_ksnapshot {Γ : Ctx L mem} {ℓ : L} {α K V : Type} (iα : DecidableEq α) (iK : DecidableEq K) (iV : DecidableEq V)
    {ord : StrOrd} {ret : Retries} {b : KSBound} (x : Var Γ (.ksing ℓ α K V iα iK iV ord ret b)) (d : Unit)
    (Δ : HList (Kind.delta (mem := mem)) Γ) (s : Node.State (.ksnapshot iα iK iV x d)) :
    NodeWired ctx m tk (.ksnapshot iα iK iV x d) () () Δ s := by
  refine ⟨?_, ?_, ?_⟩
  · simp only [Node.step, Node.override, Node.enqueue]
  · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get_stream, envRestrict_get_keyed, envRestrict_get, Kind.restrictDelta]
    show HList.cons (Kind.restrictDelta m _ _) HList.nil = _
    refine Prod.ext (funext fun i => ?_) rfl
    show (if ctx.tick ℓ i.1 then some (Δ.get x i.1) else none) = if tk then some (Δ.get x i.1) else none
    rw [tick_hosted ctx m tk htick ℓ i]
  · simp only [Node.step, IdleEnv, Kind.idleDelta]
    exact ⟨fun i hi => by rw [tick_idle ctx m tk htick ℓ i hi]; rfl, trivial⟩

/-- `kfold`, wired: another member's cell folds an empty increment and stands still. -/
theorem wired_kfold {Γ : Ctx L mem} {ℓ : L} {K V A : Type} (iK : DecidableEq K) (iH : Hashable K) (iV : DecidableEq V) (iA : DecidableEq A)
    {ord : StrOrd} (g : A → V → A) (init : A) (Pre : V → V → Prop) (iPre : DecidableRel Pre) (ok : FoldOk ord .exactlyOnce Pre g)
    (x : Var Γ (.keyed ℓ K V iK iV ord .exactlyOnce)) (Δ : HList (Kind.delta (mem := mem)) Γ)
    (s : Node.State (.kfold iK iH iV iA g init Pre iPre ok x)) (hΔ : IdleEnv m Δ) :
    NodeWired ctx m tk (.kfold iK iH iV iA g init Pre iPre ok x) () () Δ s := by
  refine ⟨?_, ?_, ?_⟩
  · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, Node.override, Node.enqueue, envRestrict_get_stream, envRestrict_get_keyed, envRestrict_get,
        Kind.restrictDelta]
    exact (override_pointwise m _ s _ (fun i hi => by
      rw [IdleEnv.get_keyed m hΔ x i hi]; rfl)).symm
  · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get_stream, envRestrict_get_keyed, envRestrict_get, Kind.restrictDelta]
    simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
    try rfl
  · simp only [IdleEnv, Kind.idleDelta]
    exact ⟨trivial, trivial⟩

/-- `tfold`, wired: another member's register scans an empty slot and stands still. -/
theorem wired_tfold {Γ : Ctx L mem} {ℓ : L} {α σ : Type} (iα : DecidableEq α) {ord : StrOrd} {ret : Retries}
    (g : σ → α → σ) (init : σ) (Pre : α → α → Prop) (iPre : DecidableRel Pre) (ok : FoldOk ord ret Pre g)
    (x : Var Γ (.atomicStream ℓ α iα ord ret)) (Δ : HList (Kind.delta (mem := mem)) Γ)
    (s : Node.State (.tfold iα g init Pre iPre ok x)) (hΔ : IdleEnv m Δ) :
    NodeWired ctx m tk (.tfold iα g init Pre iPre ok x) () () Δ s := by
  refine ⟨?_, ?_, ?_⟩
  · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, Node.override, Node.enqueue, envRestrict_get_stream, envRestrict_get_keyed, envRestrict_get,
        Kind.restrictDelta]
    exact (override_pointwise m _ s _ (fun i hi => by rw [IdleEnv.get_atomicStream m hΔ x i hi]; rfl)).symm
  · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get_stream, envRestrict_get_keyed, envRestrict_get, Kind.restrictDelta]
    simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
    try rfl
  · simp only [Node.step, IdleEnv, Kind.idleDelta]
    exact ⟨fun i hi => by rw [IdleEnv.get_atomicStream m hΔ x i hi]; rfl, trivial⟩

/-- `latest`, wired: another member's slot is `none`, so its cell stands still. -/
theorem wired_latest {Γ : Ctx L mem} {ℓ : L} {σ : Type} (iσ : DecidableEq σ) (x : Var Γ (.atomic ℓ (Option σ)))
    (Δ : HList (Kind.delta (mem := mem)) Γ)
    (s : Node.State (.latest iσ x)) (hΔ : IdleEnv m Δ) :
    NodeWired ctx m tk (.latest iσ x) () () Δ s := by
  refine ⟨?_, ?_, ?_⟩
  · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, Node.override, Node.enqueue, envRestrict_get_stream, envRestrict_get_keyed, envRestrict_get,
        Kind.restrictDelta]
    exact (override_pointwise m _ s _ (fun i hi => by rw [IdleEnv.get_atomic m hΔ x i hi])).symm
  · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get_stream, envRestrict_get_keyed, envRestrict_get, Kind.restrictDelta]
    simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
    try rfl
  · simp only [IdleEnv, Kind.idleDelta]
    exact ⟨trivial, trivial⟩

/-- `latest_monotone`, wired. -/
theorem wired_latest_monotone {Γ : Ctx L mem} {ℓ : L} (x : Var Γ (.atomic ℓ (Option Nat))) (Δ : HList (Kind.delta (mem := mem)) Γ)
    (s : Node.State (.latest_monotone x)) (hΔ : IdleEnv m Δ) :
    NodeWired ctx m tk (.latest_monotone x) () () Δ s := by
  refine ⟨?_, ?_, ?_⟩
  · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, Node.override, Node.enqueue, envRestrict_get_stream, envRestrict_get_keyed, envRestrict_get,
        Kind.restrictDelta]
    exact (override_pointwise m _ s _ (fun i hi => by rw [IdleEnv.get_atomic m hΔ x i hi])).symm
  · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get_stream, envRestrict_get_keyed, envRestrict_get, Kind.restrictDelta]
    simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
    try rfl
  · simp only [IdleEnv, Kind.idleDelta]
    exact ⟨trivial, trivial⟩

/-- `time`, wired: `hora` fixes the oracle as the script at the cells' tick counts, so the
process's reading is the lock-step's at hosted members. -/
theorem wired_time {Γ : Ctx L mem} {ℓ : L} (d : TickScript (mem ℓ) Nat) (ora : Node.Oracle m (.time (Γ := Γ) (ℓ := ℓ)))
    (Δ : HList (Kind.delta (mem := mem)) Γ) (s : Node.State (.time (Γ := Γ) (ℓ := ℓ)))
    (hora : Node.OracleAt m (.time (Γ := Γ) (ℓ := ℓ)) d ora ctx.t s) :
    NodeWired ctx m tk (.time (Γ := Γ) (ℓ := ℓ)) d ora Δ s := by
  have hora' : ora = fun i => d i.1 (s i.1) := funext hora
  subst hora'
  refine ⟨?_, ?_, ?_⟩
  · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, Node.override, Node.enqueue]
    exact (override_pointwise' m ℓ s (fun i => if ctx.tick ℓ i then s i + 1 else s i) (fun i => if tk then s i.1 + 1 else s i.1)
      (fun i => by rw [tick_hosted ctx m tk htick ℓ i])
      (fun i hi => by rw [tick_idle ctx m tk htick ℓ i hi]; rfl)).symm
  · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict]
    show HList.cons (Kind.restrictDelta m _ _) HList.nil = _
    refine Prod.ext (funext fun i => ?_) rfl
    show (if ctx.tick ℓ i.1 then some (d i.1 (s i.1)) else none) = if tk then some (d i.1 (s i.1)) else none
    rw [tick_hosted ctx m tk htick ℓ i]
  · simp only [Node.step, IdleEnv, Kind.idleDelta]
    exact ⟨fun i hi => by rw [tick_idle ctx m tk htick ℓ i hi]; rfl, trivial⟩

/-- `snapshot_bounded`, wired. -/
theorem wired_snapshot_bounded {Γ : Ctx L mem} {ℓ : L} {σ : Type} (v : σ) (d : Unit) (Δ : HList (Kind.delta (mem := mem)) Γ)
    (s : Node.State (.snapshot_bounded (Γ := Γ) (ℓ := ℓ) v d)) :
    NodeWired ctx m tk (.snapshot_bounded (Γ := Γ) (ℓ := ℓ) v d) () () Δ s := by
  refine ⟨?_, ?_, ?_⟩
  · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, Node.override, Node.enqueue]
    exact (override_pointwise' m ℓ s (fun i => if ctx.tick ℓ i then s i + 1 else s i) (fun i => if tk then s i.1 + 1 else s i.1)
      (fun i => by rw [tick_hosted ctx m tk htick ℓ i])
      (fun i hi => by rw [tick_idle ctx m tk htick ℓ i hi]; rfl)).symm
  · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict]
    show HList.cons (Kind.restrictDelta m _ _) HList.nil = _
    refine Prod.ext (funext fun i => ?_) rfl
    show (if ctx.tick ℓ i.1 then some v else none) = if tk then some v else none
    rw [tick_hosted ctx m tk htick ℓ i]
  · simp only [Node.step, IdleEnv, Kind.idleDelta]
    exact ⟨fun i hi => by rw [tick_idle ctx m tk htick ℓ i hi]; rfl, trivial⟩

/-- `demux`, wired: `chan_cells_wired` per receiver with the sender's increment filtered to the
receiver's address. Standalone because its elaboration context is the node's own types and not
the mutual theorem's motive. -/
theorem wired_demux {Γ : Ctx L mem} {c p : L} {α : Type} (iα : DecidableEq α) {ord : StrOrd}
    (x : Var Γ (.stream c (Nat × α) inferInstance ord .exactlyOnce)) (addr : Fin (mem p) → Nat)
    (d : Fin (mem p) → Fin (mem c) → Nat → Nat) (ora : Hosted m p → Fin (mem c) → Nat)
    (Δ : HList (Kind.delta (mem := mem)) Γ) (s : Fin (mem p) → Fin (mem c) → ChanCell α)
    (hΔ : IdleEnv m Δ) (hora : ∀ i j, ora i j = cumMax (d i.1 j) ctx.t) :
    NodeWired ctx m tk (.demux (p := p) iα x addr) d ora Δ s := by
  have hidleS : ∀ j, (⟨c, j⟩ : Member L mem) ≠ m → Δ.get x j = #[] := IdleEnv.get_stream m hΔ x
  let pred : Fin (mem p) → Nat × α → Option α := fun i dx => if dx.1 = addr i then some dx.2 else none
  have hc : ∀ i : Fin (mem p), _ := fun i => chan_cells_wired ctx m hact d
    (fun j => filterMapB (pred i) (Δ.get x j)) s ora
    (fun j => if _ : (⟨c, j⟩ : Member L mem) = m then filterMapB (pred i) (Δ.get x j) else #[])
    hora (fun j hj => by rw [hidleS j hj, filterMapB_empty])
    (fun j hj => by simp only [hj, dite_true])
    (fun j hj => by simp only [hj, dite_false])
  refine ⟨?_, ?_, ?_⟩
  · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, Node.override, Node.enqueue, Node.enqueueLoc,
      envRestrict_get_stream, envRestrict_get_keyed, envRestrict_get, Kind.restrictDelta]
    funext i
    split
    · rename_i hi
      funext j
      rw [congrArg Prod.fst ((hc i).1 ⟨i, hi⟩ j)]
      split
      · rfl
      · simp only [ChanCell.enqueue_nil]
    · rename_i hi
      funext j
      have hi' := hi
      rw [(hc i).2.1 i j hi']
  · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get_stream, envRestrict_get_keyed,
      envRestrict_get, Kind.restrictDelta]
    show HList.cons (Kind.restrictDelta m _ _) HList.nil = _
    refine Prod.ext (funext fun i => ?_) rfl
    show tagConcat _ = tagConcat _
    congr 1
    funext j
    rw [congrArg Prod.snd ((hc i.1).1 i j)]
  · simp only [Node.step, IdleEnv, Kind.idleDelta]
    refine ⟨fun i hi => ?_, trivial⟩
    apply tagConcat_eq_empty
    intro j
    rw [(hc i).2.2 i j hi]

mutual

/-- **One node's lock-step step equals the process's step, written back.** The hypotheses: the inputs are idle off
the process (`IdleEnv`), the oracle reads the schedule at `ctx.t` (`OracleAt`), and the cells
are drained (`Drained`, used by `zipTick`, `reduce_watermark` and `tick_scan`, whose cells
would otherwise release a pending item at an inactive member). -/
theorem Node.step_wired : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) → (sch : Node.Sched n) →
    (ora : Node.Oracle m n) → (Δ : HList (Kind.delta (mem := mem)) Γ) → (s : Node.State n) →
    IdleEnv m Δ → Node.OracleAt m n sch ora ctx.t s → Node.Drained n sch s →
    NodeWired ctx m tk n sch ora Δ s
  -- stateless, stream in / stream out: the cut outputs are the outputs of the cut inputs
  | _, _, .map _ _ x f, _, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.override, Node.enqueue]
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get_stream, envRestrict_get_keyed, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => by rw [IdleEnv.get_stream m hΔ x i hi]; simp, trivial⟩
  | _, _, .filterMap _ _ x f, _, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.override, Node.enqueue]
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get_stream, envRestrict_get_keyed, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => by rw [IdleEnv.get_stream m hΔ x i hi]; simp, trivial⟩
  | _, _, .values _ _ x, _, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.override, Node.enqueue]
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get_stream, envRestrict_get_keyed, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => by rw [IdleEnv.get_keyed m hΔ x i hi]; simp, trivial⟩
  | _, _, .into_keyed _ _ x, _, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.override, Node.enqueue]
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get_stream, envRestrict_get_keyed, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => IdleEnv.get_stream m hΔ x i hi, trivial⟩
  | _, _, .kassume_ordering _ _ x _, _, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.override, Node.enqueue]
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get_stream, envRestrict_get_keyed, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => IdleEnv.get_keyed m hΔ x i hi, trivial⟩
  | _, _, .kget_max_key _ _ _ _ x, _, _, Δ, s, _, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.override, Node.enqueue]
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get_stream, envRestrict_get_keyed, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [IdleEnv, Kind.idleDelta]
      exact ⟨trivial, trivial⟩
  | _, _, .weaken_retries _ x, _, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.override, Node.enqueue]
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get_stream, envRestrict_get_keyed, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => IdleEnv.get_stream m hΔ x i hi, trivial⟩
  | _, _, .union _ a b, _, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.override, Node.enqueue]
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get_stream, envRestrict_get_keyed, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => by rw [IdleEnv.get_stream m hΔ a i hi, IdleEnv.get_stream m hΔ b i hi]; rfl, trivial⟩
  | _, _, .assume_ordering _ x _, _, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.override, Node.enqueue]
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get_stream, envRestrict_get_keyed, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => IdleEnv.get_stream m hΔ x i hi, trivial⟩
  | _, _, .mapTick x f, _, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.override, Node.enqueue]
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get_stream, envRestrict_get_keyed, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => by rw [IdleEnv.get_atomic m hΔ x i hi]; rfl, trivial⟩
  | _, _, .firstTick _ _ x, _, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.override, Node.enqueue]
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get_stream, envRestrict_get_keyed, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => by rw [IdleEnv.get_atomicStream m hΔ x i hi]; rfl, trivial⟩
  | _, _, .allTicks _ x, _, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.override, Node.enqueue]
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get_stream, envRestrict_get_keyed, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => by rw [IdleEnv.get_atomicStream m hΔ x i hi]; rfl, trivial⟩
  | _, _, .flattenOrdered _ x, _, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.override, Node.enqueue]
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get_stream, envRestrict_get_keyed, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => by rw [IdleEnv.get_atomic m hΔ x i hi]; rfl, trivial⟩
  | _, _, .flattenUnordered _ x, _, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.override, Node.enqueue]
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get_stream, envRestrict_get_keyed, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => by rw [IdleEnv.get_atomic m hΔ x i hi]; rfl, trivial⟩
  | _, _, .tinto_keyed _ _ x, _, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.override, Node.enqueue]
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get_stream, envRestrict_get_keyed, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => IdleEnv.get_atomicStream m hΔ x i hi, trivial⟩
  | _, _, .tassume_ordering _ x _, _, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.override, Node.enqueue]
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get_stream, envRestrict_get_keyed, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => IdleEnv.get_atomicStream m hΔ x i hi, trivial⟩
  | _, _, .kentries iK iV x, _, _, Δ, s, hΔ, _, _ => wired_kentries ctx m tk hact htick iK iV x Δ s hΔ
  | _, _, .kmap iK iV iW x f, _, _, Δ, s, hΔ, _, _ => wired_kmap ctx m tk hact htick iK iV iW x f Δ s hΔ
  | _, _, .kallTicks iK iV x, _, _, Δ, s, hΔ, _, _ => wired_kallTicks ctx m tk hact htick iK iV x Δ s hΔ
  | _, _, .ksnapshot iα iK iV x d, _, _, Δ, s, _, _, _ => wired_ksnapshot ctx m tk hact htick iα iK iV x d Δ s
  | _, _, .kfold iK iH iV iA g init Pre iPre ok x, _, _, Δ, s, hΔ, _, _ =>
    wired_kfold ctx m tk hact htick iK iH iV iA g init Pre iPre ok x Δ s hΔ
  | _, _, .tfold iα g init Pre iPre ok x, _, _, Δ, s, hΔ, _, _ => wired_tfold ctx m tk hact htick iα g init Pre iPre ok x Δ s hΔ
  | _, _, .latest iσ x, _, _, Δ, s, hΔ, _, _ => wired_latest ctx m tk hact htick iσ x Δ s hΔ
  | _, _, .latest_monotone x, _, _, Δ, s, hΔ, _, _ => wired_latest_monotone ctx m tk hact htick x Δ s hΔ
  | _, _, .time, d, ora, Δ, s, _, hora, _ => wired_time ctx m tk hact htick d ora Δ s hora
  | _, _, .snapshot_bounded v d, _, _, Δ, s, _, _, _ => wired_snapshot_bounded ctx m tk hact htick v d Δ s
  -- tick-gated, stateless: the gate is `tk` at hosted members and `false` elsewhere
  | _, _, .snapshot (ℓ := ℓ) _ x _, _, _, Δ, s, _, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.override, Node.enqueue]
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get_stream, envRestrict_get_keyed, envRestrict_get, Kind.restrictDelta]
      show HList.cons (Kind.restrictDelta m _ _) HList.nil = _
      refine Prod.ext (funext fun i => ?_) rfl
      show (if ctx.tick ℓ i.1 then some (Δ.get x i.1) else none) = if tk then some (Δ.get x i.1) else none
      rw [tick_hosted ctx m tk htick ℓ i]
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => by rw [tick_idle ctx m tk htick ℓ i hi]; rfl, trivial⟩
  -- stateful: the cells are a write-back, because another member's cell sees an empty
  -- increment, no tick, and (when drained) nothing to release
  | _, _, .kfold_early_stop _ _ _ x f init, _, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, Node.override, Node.enqueue, envRestrict_get_stream, envRestrict_get_keyed, envRestrict_get,
        Kind.restrictDelta]
      refine (override_pointwise m _ s _ (fun i hi => ?_)).symm
      rw [IdleEnv.get_keyed m hΔ x i hi, foldlB_empty]
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get_stream, envRestrict_get_keyed, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [IdleEnv, Kind.idleDelta]
      exact ⟨trivial, trivial⟩
  | _, _, .fold _ g _ _ _ _ x, _, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, Node.override, Node.enqueue, envRestrict_get_stream, envRestrict_get_keyed, envRestrict_get,
        Kind.restrictDelta]
      refine (override_pointwise m _ s _ (fun i hi => ?_)).symm
      rw [IdleEnv.get_stream m hΔ x i hi, foldlB_empty]
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get_stream, envRestrict_get_keyed, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [IdleEnv, Kind.idleDelta]
      exact ⟨trivial, trivial⟩
  | _, _, .fold_monotone _ _ g _ _ _ x, _, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, Node.override, Node.enqueue, envRestrict_get_stream, envRestrict_get_keyed, envRestrict_get,
        Kind.restrictDelta]
      refine (override_pointwise m _ s _ (fun i hi => ?_)).symm
      rw [IdleEnv.get_stream m hΔ x i hi, foldlB_empty]
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get_stream, envRestrict_get_keyed, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [IdleEnv, Kind.idleDelta]
      exact ⟨trivial, trivial⟩
  | _, _, .batch (ℓ := ℓ) _ _ _ x _, _, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, Node.override, Node.enqueue, envRestrict_get_stream, envRestrict_get_keyed, envRestrict_get,
        Kind.restrictDelta]
      exact (override_pointwise' m ℓ s (fun i => if ctx.tick ℓ i then #[] else s i ++ Δ.get x i)
        (fun i => if tk then #[] else s i.1 ++ Δ.get x i.1)
        (fun i => by rw [tick_hosted ctx m tk htick ℓ i])
        (fun i hi => by rw [tick_idle ctx m tk htick ℓ i hi, IdleEnv.get_stream m hΔ x i hi]; simp)).symm
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get_stream, envRestrict_get_keyed, envRestrict_get, Kind.restrictDelta]
      show HList.cons (Kind.restrictDelta m _ _) HList.nil = _
      refine Prod.ext (funext fun i => ?_) rfl
      show (if ctx.tick ℓ i.1 then some (s i.1 ++ Δ.get x i.1) else none)
        = if tk then some (s i.1 ++ Δ.get x i.1) else none
      rw [tick_hosted ctx m tk htick ℓ i]
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => by rw [tick_idle ctx m tk htick ℓ i hi]; rfl, trivial⟩
  | _, _, .source_interval (ℓ := ℓ) _ _, d, ora, Δ, s, _, hora, _ => by
    have hora' : ora = fun i => d i.1 (s i.1) := funext hora
    subst hora'
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, Node.override, Node.enqueue]
      exact (override_pointwise' m ℓ s (fun i => if ctx.tick ℓ i then s i + 1 else s i) (fun i => if tk then s i.1 + 1 else s i.1)
        (fun i => by rw [tick_hosted ctx m tk htick ℓ i])
        (fun i hi => by rw [tick_idle ctx m tk htick ℓ i hi]; rfl)).symm
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict]
      show HList.cons (Kind.restrictDelta m _ _) HList.nil = _
      refine Prod.ext (funext fun i => ?_) rfl
      show (if ctx.tick ℓ i.1 then Array.replicate (d i.1 (s i.1)) () else #[])
        = if tk then Array.replicate (d i.1 (s i.1)) () else #[]
      rw [tick_hosted ctx m tk htick ℓ i]
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => by rw [tick_idle ctx m tk htick ℓ i hi]; rfl, trivial⟩
  | _, _, .zipTick (ℓ := ℓ) a b, _, _, Δ, s, hΔ, _, hdr => by
    have hidle : ∀ i, (⟨ℓ, i⟩ : Member L mem) ≠ m →
        (((s i).1 ++ (Δ.get a i).toList).drop (min ((s i).1 ++ (Δ.get a i).toList).length ((s i).2 ++ (Δ.get b i).toList).length),
         ((s i).2 ++ (Δ.get b i).toList).drop (min ((s i).1 ++ (Δ.get a i).toList).length ((s i).2 ++ (Δ.get b i).toList).length)) = s i :=
      fun i hi => by
        rw [IdleEnv.get_atomic m hΔ a i hi, IdleEnv.get_atomic m hΔ b i hi]
        simp only [Option.toList_none, List.append_nil]
        rcases hdr i with h | h <;> simp [h, Prod.ext_iff]
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, Node.override, Node.enqueue, envRestrict_get_stream, envRestrict_get_keyed, envRestrict_get,
        Kind.restrictDelta]
      exact (override_pointwise m _ s _ hidle).symm
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get_stream, envRestrict_get_keyed, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      refine ⟨fun i hi => ?_, trivial⟩
      rw [IdleEnv.get_atomic m hΔ a i hi, IdleEnv.get_atomic m hΔ b i hi]
      simp only [Option.toList_none, List.append_nil]
      rcases hdr i with h | h <;> simp [h]
  | _, _, .defer_tick (ℓ := ℓ) _ x, _, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, Node.override, Node.enqueue, envRestrict_get_stream, envRestrict_get_keyed, envRestrict_get,
        Kind.restrictDelta]
      refine (override_pointwise' m ℓ s
        (fun i => (if ctx.tick ℓ i then (s i).tail else s i) ++ (Δ.get x i).toList)
        (fun i => (if tk then (s i.1).tail else s i.1) ++ (Δ.get x i.1).toList)
        (fun i => ?_) (fun i hi => ?_)).symm
      · rw [tick_hosted ctx m tk htick ℓ i]
      · rw [tick_idle ctx m tk htick ℓ i hi, IdleEnv.get_atomic m hΔ x i hi, Option.toList_none, List.append_nil]
        rfl
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get_stream, envRestrict_get_keyed, envRestrict_get, Kind.restrictDelta]
      show HList.cons (Kind.restrictDelta m _ _) HList.nil = _
      refine Prod.ext (funext fun i => ?_) rfl
      show (if ctx.tick ℓ i.1 then (s i.1).head? else none) = if tk then (s i.1).head? else none
      rw [tick_hosted ctx m tk htick ℓ i]
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      refine ⟨fun i hi => ?_, trivial⟩
      rw [tick_idle ctx m tk htick ℓ i hi]
      rfl
  | _, _, .reduce_watermark (ℓ := ℓ) _ _ _ _ x wm f _ _ _, _, _, Δ, s, hΔ, _, hdr => by
    have hidle : ∀ i, (⟨ℓ, i⟩ : Member L mem) ≠ m →
        ((((s i).1.1 ++ ((Δ.get x i).map Array.toList).toList).drop (min ((s i).1.1 ++ ((Δ.get x i).map Array.toList).toList).length ((s i).1.2 ++ (Δ.get wm i).toList).length),
          ((s i).1.2 ++ (Δ.get wm i).toList).drop (min ((s i).1.1 ++ ((Δ.get x i).map Array.toList).toList).length ((s i).1.2 ++ (Δ.get wm i).toList).length)),
         (scanRun (reduceWatermarkEager f) (s i).2
           (List.zip ((s i).1.1 ++ ((Δ.get x i).map Array.toList).toList) ((s i).1.2 ++ (Δ.get wm i).toList))).1) = s i :=
      fun i hi => by
        rw [IdleEnv.get_atomicKeyed m hΔ x i hi, IdleEnv.get_atomic m hΔ wm i hi]
        simp only [Option.map_none, Option.toList_none, List.append_nil]
        rcases hdr i with h | h <;> simp [h, scanRun, Prod.ext_iff]
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, Node.override, Node.enqueue, envRestrict_get_stream, envRestrict_get_keyed, envRestrict_get,
        Kind.restrictDelta]
      exact (override_pointwise m _ s _ hidle).symm
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get_stream, envRestrict_get_keyed, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      refine ⟨fun i hi => ?_, trivial⟩
      rw [IdleEnv.get_atomicKeyed m hΔ x i hi, IdleEnv.get_atomic m hΔ wm i hi]
      simp only [Option.map_none, Option.toList_none, List.append_nil]
      rcases hdr i with h | h <;> simp [h, scanRun]
  | _, _, .tick_scan (ℓ := ℓ) sts ins outs x g seed, _, _, Δ, s, hΔ, _, hdr => by
    have hleaf : IdleEnv m (readShapeDelta Δ ℓ ins x) := readShapeDelta_idle m Δ hΔ ℓ ins x
    have hpush : ∀ i, (⟨ℓ, i⟩ : Member L mem) ≠ m →
        PendingOf.push ins (s i).1 (fun _ => readShapeDelta Δ ℓ ins x) i = (s i).1 :=
      fun i hi => PendingOf.push_idle m ins (s i).1 _ i hleaf hi
    have hrun : ∀ i, (⟨ℓ, i⟩ : Member L mem) ≠ m →
        scanRun (g i) (s i).2 (PendingOf.takeZip ins (PendingOf.push ins (s i).1 (fun _ => readShapeDelta Δ ℓ ins x) i)
          (PendingOf.avail ins (PendingOf.push ins (s i).1 (fun _ => readShapeDelta Δ ℓ ins x) i))) = ((s i).2, []) :=
      fun i hi => by rw [hpush i hi, hdr i, PendingOf.takeZip_zero]; rfl
    have hloc : ∀ i : Hosted m ℓ,
        PendingOf.pushLoc m ins (s i.1).1 (fun _ => readShapeLoc m (envRestrict m Δ) ℓ ins x) i
          = PendingOf.push ins (s i.1).1 (fun _ => readShapeDelta Δ ℓ ins x) i.1 := fun i => by
      rw [readShapeLoc_restrict]
      exact PendingOf.pushLoc_restrict m ins (s i.1).1 (fun _ => readShapeDelta Δ ℓ ins x) i
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, Node.override, Node.enqueue, hloc]
      refine (override_pointwise m _ s _ (fun i hi => ?_)).symm
      rw [hrun i hi, hpush i hi, hdr i, PendingOf.dropN_zero]
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, hloc]
      rw [unzipShape_restrict]
    · simp only [Node.step]
      exact unzipShape_idle m ℓ outs _ (fun i hi => by rw [hrun i hi]; rfl)
  -- the network: `chan_cells_wired` per receiver
  | _, _, .broadcast_closed (c := c) (p := p) _ x, d, ora, Δ, s, hΔ, hora, _ => by
    have hidleS : ∀ j, (⟨c, j⟩ : Member L mem) ≠ m → Δ.get x j = #[] := IdleEnv.get_stream m hΔ x
    have hc := chan_cells_wired ctx m hact d (fun j => Δ.get x j) s ora
      (fun j => if _ : (⟨c, j⟩ : Member L mem) = m then Δ.get x j else #[]) hora hidleS
      (fun j hj => by simp only [hj, dite_true])
      (fun j hj => by simp only [hj, dite_false])
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, Node.override, Node.enqueue, Node.enqueueLoc, envRestrict_get_stream, envRestrict_get_keyed, envRestrict_get,
        Kind.restrictDelta]
      funext i
      split
      · rename_i hi
        funext j
        rw [congrArg Prod.fst (hc.1 ⟨i, hi⟩ j)]
        split
        · rfl
        · simp only [ChanCell.enqueue_nil]
      · rename_i hi
        funext j
        have hi' := hi
        rw [hc.2.1 i j hi']
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get_stream, envRestrict_get_keyed, envRestrict_get, Kind.restrictDelta]
      show HList.cons (Kind.restrictDelta m _ _) HList.nil = _
      refine Prod.ext (funext fun i => ?_) rfl
      show tagConcat _ = tagConcat _
      congr 1
      funext j
      rw [congrArg Prod.snd (hc.1 i j)]
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      refine ⟨fun i hi => ?_, trivial⟩
      apply tagConcat_eq_empty
      intro j
      rw [hc.2.2 i j hi]
  | _, _, .demux iα x addr, d, ora, Δ, s, hΔ, hora, _ => wired_demux ctx m tk hact htick iα x addr d ora Δ s hΔ hora
  -- composites: `call` substitutes; `fixN` reduces to the body's step at the knot increments
  | _, _, .call N args, sch, ora, Δ, s, hΔ, hora, hdr => by
    have ih := Dag.step_wired N sch ora (args.map (fun v => Δ.get v)) s (IdleEnv.mapSubst m Δ hΔ args) hora hdr
    simp only [DagWired] at ih
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, Node.override, Node.enqueue]
      rw [← envRestrict_mapSubst]
      exact ih.1
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict]
      rw [← envRestrict_mapSubst]
      exact ih.2.1
    · simp only [Node.step]
      exact ih.2.2
  | _, _, .fixN ks body, sch, ora, Δ, s, hΔ, hora, hdr => by
    -- round by round, the lock-step's knot increments are idle off the process and, cut to it,
    -- the process's knot increments; the empty fill restricts to the empty fill
    have hround : ∀ i, IdleEnv m (knotIter (fun X => (Dag.step ctx body sch (X.append Δ) s).2.take) i (knotFill ks)) ∧
        envRestrict m (knotIter (fun X => (Dag.step ctx body sch (X.append Δ) s).2.take) i (knotFill ks))
          = knotIter (fun K => (Dag.locStep m tk body ora (K.append (envRestrict m Δ)) (Dag.restrict m body s)).2.1.take)
              i (knotFillLoc m ks) := by
      intro i
      induction i with
      | zero => exact ⟨knotFill_idle m ks, knotFill_restrict m ks⟩
      | succ i ih =>
        rw [knotIter_succ', knotIter_succ']
        have hΔ₁ : IdleEnv m ((knotIter (fun X => (Dag.step ctx body sch (X.append Δ) s).2.take) i (knotFill ks)).append Δ) :=
          IdleEnv.append m _ Δ ih.1 hΔ
        have ih₁ := Dag.step_wired body sch ora _ s hΔ₁ hora hdr
        simp only [DagWired] at ih₁
        refine ⟨IdleEnv.take m _ ih₁.2.2, ?_⟩
        rw [envRestrict_take, ih₁.2.1, envRestrict_append, ih.2]
    have hK := hround ks.length
    -- the final body step at the knot increments after `ks.length` rounds
    have hΔ₂ : IdleEnv m ((knotIter (fun X => (Dag.step ctx body sch (X.append Δ) s).2.take) ks.length (knotFill ks)).append Δ) :=
      IdleEnv.append m _ Δ hK.1 hΔ
    have ih₂ := Dag.step_wired body sch ora _ s hΔ₂ hora hdr
    simp only [DagWired] at ih₂
    rw [envRestrict_append, hK.2] at ih₂
    exact ih₂

/-- **One lock-step step equals the process's step, written back**, along the `letN` chain: the rest of the graph
reads the node's outputs, which are idle off the process and cut to the process's. -/
theorem Dag.step_wired : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) → (sch : Dag.Sched d) →
    (ora : Dag.Oracle m d) → (Δ : HList (Kind.delta (mem := mem)) Γ) → (s : Dag.State d) →
    IdleEnv m Δ → Dag.OracleAt m d sch ora ctx.t s → Dag.Drained d sch s →
    DagWired ctx m tk d sch ora Δ s
  | _, _, .ret xs, _, _, Δ, s, hΔ, _, _ => by
    refine ⟨rfl, ?_, IdleEnv.mapSubst m Δ hΔ xs⟩
    simp only [Dag.step, Dag.locStep, Dag.restrict]
    exact envRestrict_mapSubst m Δ xs
  | _, _, .letN n rest, sch, ora, Δ, s, hΔ, hora, hdr => by
    have ihn := Node.step_wired n sch.1 ora.1 Δ s.1 hΔ hora.1 hdr.1
    simp only [NodeWired] at ihn
    have hΔ' : IdleEnv m ((Node.step ctx n sch.1 Δ s.1).2.append Δ) := IdleEnv.append m _ _ ihn.2.2 hΔ
    have ihr := Dag.step_wired rest sch.2 ora.2 ((Node.step ctx n sch.1 Δ s.1).2.append Δ) s.2 hΔ' hora.2 hdr.2
    simp only [DagWired] at ihr
    have hr : envRestrict m ((Node.step ctx n sch.1 Δ s.1).2.append Δ)
        = (Node.locStep m tk n ora.1 (envRestrict m Δ) (Node.restrict m n s.1)).2.1.append (envRestrict m Δ) := by
      rw [envRestrict_append, ihn.2.1]
    refine ⟨?_, ?_, ?_⟩
    · simp only [Dag.step, Dag.locStep, Dag.restrict, Dag.override, Dag.enqueue, Dag.enqueueLoc, Node.memoState_eq,
        memoEnv_eq]
      rw [← hr, ihn.1, ihr.1]
    · simp only [Dag.step, Dag.locStep, Dag.restrict, memoEnv_eq]
      rw [← hr, ihr.2.1]
    · simp only [Dag.step]
      exact ihr.2.2

end

end Main

end Project

end Hydro
