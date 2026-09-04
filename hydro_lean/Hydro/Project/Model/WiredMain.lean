import Hydro.Project.Model.WiredSpec

/-!
# Hydro · Project — wired machine: one step is one process

`Node.step_wired`/`Dag.step_wired`: a lock-step step in which **exactly**
the process `m`'s members are active (`ctx.act = m`) and tick iff the
process ticks (`ctx.tick = fun ℓ i => m.hosts ℓ i && tk`), only its member
receive input increments (`IdleEnv`), and its oracle reads the cursors
(`OracleAt`) equals the process's own `locStep m tk` written back over the
state with its sends appended to everyone else's queues
(`override`/`enqueue`); the step's increments cut to the process are the
process's, and are empty at everyone else's members.

The only condition on the state is `Drained` — the multi-input tick cells
hold no complete item — which every state the machine produces satisfies
(`Dag.step_drained`, `Dag.init_drained`). Nothing is assumed about the
other processes: an inactive member is inert (channels only enqueue at it,
knots hold their pending feedback).
-/

namespace Hydro

namespace Project

variable {L : Type} {mem : L → Nat}

/-- The conclusions of a wired step for a node. -/
def NodeWired [DecidableEq L] (ctx : StepCtx L mem) (m : Member L mem) (tk : Bool) {Γ : Ctx L mem} {κs : List (Kind L mem)}
    (n : Node Γ κs) (sch : Node.Sched n) (ora : Node.Oracle m n) (Δ : HList (Kind.delta (mem := mem)) Γ)
    (s : Node.State n) : Prop :=
  let r := Node.locStep m tk n ora (envRestrict m Δ) (Node.restrict m n s)
  (Node.step ctx n sch Δ s).1 = Node.override m n (Node.enqueue m n r.2.2 s) (Node.enqueueLoc m m n r.2.2 r.1) ∧
  envRestrict m (Node.step ctx n sch Δ s).2 = r.2.1 ∧
  IdleEnv m (Node.step ctx n sch Δ s).2

/-- The conclusions of a wired step for a graph. -/
def DagWired [DecidableEq L] (ctx : StepCtx L mem) (m : Member L mem) (tk : Bool) {Γ : Ctx L mem} {outs : List (Kind L mem)}
    (d : Dag Γ outs) (sch : Dag.Sched d) (ora : Dag.Oracle m d) (Δ : HList (Kind.delta (mem := mem)) Γ)
    (s : Dag.State d) : Prop :=
  let r := Dag.locStep m tk d ora (envRestrict m Δ) (Dag.restrict m d s)
  (Dag.step ctx d sch Δ s).1 = Dag.override m d (Dag.enqueue m d r.2.2 s) (Dag.enqueueLoc m m d r.2.2 r.1) ∧
  envRestrict m (Dag.step ctx d sch Δ s).2 = r.2.1 ∧
  IdleEnv m (Dag.step ctx d sch Δ s).2

/-! ## Drained cells: every state the machine produces -/

mutual

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

theorem Dag.init_drained : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) → (sch : Dag.Sched d) →
    Dag.Drained d sch (Dag.init d)
  | _, _, .ret _, _ => trivial
  | _, _, .letN n rest, sch => ⟨Node.init_drained n sch.1, Dag.init_drained rest sch.2⟩

end

mutual

/-- After any step the multi-input tick cells are drained. -/
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
  | _, _, .fixN ks body, sch, Δ, s => Dag.step_drained ctx body sch _ s.1
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

theorem Dag.step_drained (ctx : StepCtx L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    (sch : Dag.Sched d) → (Δ : HList (Kind.delta (mem := mem)) Γ) → (s : Dag.State d) →
    Dag.Drained d sch (Dag.step ctx d sch Δ s).1
  | _, _, .ret _, _, _, _ => trivial
  | _, _, .letN n rest, sch, Δ, s => ⟨Node.step_drained ctx n sch.1 Δ s.1, Dag.step_drained ctx rest sch.2 _ s.2⟩

end

/-! ## The knot feedback under a process's ticks -/

/-- Delivering then enqueueing is delivering, then enqueueing afterwards. -/
theorem ChanCell.step_enqueue {α : Type} (cell : ChanCell α) (c : Nat) (sends : List α) :
    cell.step c sends = ((cell.step c []).1.enqueue sends, (cell.step c []).2) := by
  simp [ChanCell.step, ChanCell.enqueue]

theorem ChanCell.enqueue_nil {α : Type} (cell : ChanCell α) : cell.enqueue [] = cell := by
  simp [ChanCell.enqueue]

variable [DecidableEq L]

section Knots
variable (ctx : StepCtx L mem) (m : Member L mem) (hact : ctx.act = m.hosts)
include hact

theorem KnotKind.feed_restrict : (k : KnotKind L mem) → (d : k.toKind.delta) →
    Kind.restrictDelta m k.toKind (k.feed ctx d) = Kind.restrictDelta m k.toKind d
  | .stream .., d => funext fun i => by
    show (if ctx.act _ i.1 then d i.1 else []) = d i.1
    rw [hact]; simp [i.2]
  | .atomic .., d => funext fun i => by
    show (if ctx.act _ i.1 then d i.1 else none) = d i.1
    rw [hact]; simp [i.2]

theorem KnotKind.feed_idle : (k : KnotKind L mem) → (d : k.toKind.delta) →
    Kind.idleDelta m k.toKind (k.feed ctx d)
  | .stream .., d => fun i hi => by
    show (if ctx.act _ i then d i else []) = []
    rw [hact]; simp [hi]
  | .atomic .., d => fun i hi => by
    show (if ctx.act _ i then d i else none) = none
    rw [hact]; simp [hi]

/-- Keeping under the process's ticks: the member drops its pending
feedback and keep the body's new output; idle members keep theirs (the
body's output there is empty). -/
theorem KnotKind.keep_override : (k : KnotKind L mem) → (d inc : k.toKind.delta) →
    Kind.idleDelta m k.toKind inc →
    k.keep ctx d inc = Kind.overrideDelta m k.toKind d (Kind.restrictDelta m k.toKind inc)
  | .stream .., d, inc, hinc => funext fun i => by
    show (if ctx.act _ i then [] else d i) ++ inc i = if h : (⟨_, i⟩ : Member L mem) = m then inc i else d i
    rw [hact]; simp only [Member.hosts_eq_true]
    split
    · exact List.nil_append _
    · rename_i h
      rw [hinc i h, List.append_nil]
  | .atomic .., d, inc, hinc => funext fun i => by
    show (if ctx.act _ i then inc i else (d i).or (inc i)) = if h : (⟨_, i⟩ : Member L mem) = m then inc i else d i
    rw [hact]; simp only [Member.hosts_eq_true]
    split
    · rfl
    · rename_i h
      rw [hinc i h, Option.or_none]

theorem knotFeed_restrict : (ks : List (KnotKind L mem)) →
    (d : HList (Kind.delta (mem := mem)) (ks.map KnotKind.toKind)) →
    envRestrict m (knotFeed ctx ks d) = envRestrict m d
  | [], _ => rfl
  | k :: ks, d => by
    show (Kind.restrictDelta m _ (k.feed ctx d.1), envRestrict m (knotFeed ctx ks d.2))
      = (Kind.restrictDelta m _ d.1, envRestrict m d.2)
    rw [KnotKind.feed_restrict ctx m hact k d.1, knotFeed_restrict ks d.2]

theorem knotFeed_idle : (ks : List (KnotKind L mem)) →
    (d : HList (Kind.delta (mem := mem)) (ks.map KnotKind.toKind)) →
    IdleEnv m (knotFeed ctx ks d)
  | [], _ => trivial
  | k :: ks, d => ⟨KnotKind.feed_idle ctx m hact k d.1, knotFeed_idle ks d.2⟩

theorem knotKeep_override : (ks : List (KnotKind L mem)) →
    (d inc : HList (Kind.delta (mem := mem)) (ks.map KnotKind.toKind)) → IdleEnv m inc →
    knotKeep ctx ks d inc = envOverride m d (envRestrict m inc)
  | [], _, _, _ => rfl
  | k :: ks, d, inc, hinc => by
    show (k.keep ctx d.1 inc.1, knotKeep ctx ks d.2 inc.2)
      = (Kind.overrideDelta m _ d.1 (Kind.restrictDelta m _ inc.1), envOverride m d.2 (envRestrict m inc.2))
    rw [KnotKind.keep_override ctx m hact k d.1 inc.1 hinc.1, knotKeep_override ks d.2 inc.2 hinc.2]

end Knots

/-- A per-member cell map agreeing with the process's cells at hosted
members and unchanged at idle ones is the write-back. -/
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

omit htick in
theorem act_hosted (ℓ : L) (i : Hosted m ℓ) : ctx.act ℓ i.1 = true := by rw [hact]; simpa using i.2
omit htick in
theorem act_idle (ℓ : L) (i : Fin (mem ℓ)) (hi : (⟨ℓ, i⟩ : Member L mem) ≠ m) : ctx.act ℓ i = false := by rw [hact]; simpa using hi
omit hact in
theorem tick_hosted (ℓ : L) (i : Hosted m ℓ) : ctx.tick ℓ i.1 = tk := by rw [htick]; simp [i.2]
omit hact in
theorem tick_idle (ℓ : L) (i : Fin (mem ℓ)) (hi : (⟨ℓ, i⟩ : Member L mem) ≠ m) : ctx.tick ℓ i = false := by rw [htick]; simp [hi]

omit htick in
/-- The channel cells of a network node after a wired step, per receiver:
the member's receiving ends deliver per the oracle (the process step) and then take
its own sends (the loopback queue); idle receivers only enqueue them. -/
theorem chan_cells_wired {c p : L} {α : Type} (d : Fin (mem p) → Fin (mem c) → Nat → Nat)
    (sendsG : Fin (mem c) → List α) (s : Fin (mem p) → Fin (mem c) → ChanCell α)
    (ora : Hosted m p → Fin (mem c) → Nat) (sendsL : Fin (mem c) → List α)
    (hora : ∀ i j, ora i j = cumMax (d i.1 j) ctx.t)
    (hidleS : ∀ j, (⟨c, j⟩ : Member L mem) ≠ m → sendsG j = [])
    (hsends : ∀ j, (⟨c, j⟩ : Member L mem) = m → sendsL j = sendsG j)
    (hsendsI : ∀ j, (⟨c, j⟩ : Member L mem) ≠ m → sendsL j = []) :
    (∀ (i : Hosted m p) j, (s i.1 j).stepG (ctx.act p i.1) (cumMax (d i.1 j) ctx.t) (sendsG j)
      = (((s i.1 j).step (ora i j) []).1.enqueue (sendsL j), ((s i.1 j).step (ora i j) []).2)) ∧
    (∀ i j, (⟨p, i⟩ : Member L mem) ≠ m →
      ((s i j).stepG (ctx.act p i) (cumMax (d i j) ctx.t) (sendsG j)).1
        = if _ : (⟨c, j⟩ : Member L mem) = m then (s i j).enqueue (sendsG j) else s i j) ∧
    (∀ i j, (⟨p, i⟩ : Member L mem) ≠ m → ((s i j).stepG (ctx.act p i) (cumMax (d i j) ctx.t) (sendsG j)).2 = []) := by
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
      show ChanCell.mk ((s i j).pending ++ []) _ = s i j
      rw [List.append_nil]
  · rw [act_idle ctx m hact p i hi]
    simp only [ChanCell.stepG, Bool.false_eq_true, if_false]

-- `hact`/`htick` are used by the node theorem; the graph theorem of the mutual block carries them along
set_option linter.unusedSectionVars false

/-! D81: the bench client's vocabulary, as standalone wired-step lemmas (they do not
recurse, so the mutual theorem's budget stays flat). -/

theorem wired_kentries {Γ : Ctx L mem} {ℓ : L} {K V : Type} (iK : DecidableEq K) (iV : DecidableEq V) {ord : StrOrd}
    (x : Var Γ (.keyed ℓ K V iK iV ord .exactlyOnce)) (Δ : HList (Kind.delta (mem := mem)) Γ) (s : Node.State (.kentries iK iV x))
    (hΔ : IdleEnv m Δ) :
    NodeWired ctx m tk (.kentries iK iV x) () () Δ s := by
  refine ⟨?_, ?_, ?_⟩
  · simp only [Node.step, Node.override, Node.enqueue]
  · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get, Kind.restrictDelta]
    simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
    try rfl
  · simp only [Node.step, IdleEnv, Kind.idleDelta]
    exact ⟨fun i hi => IdleEnv.get_keyed m hΔ x i hi, trivial⟩

theorem wired_kmap {Γ : Ctx L mem} {ℓ : L} {K V W : Type} (iK : DecidableEq K) (iV : DecidableEq V) (iW : DecidableEq W) {ord : StrOrd}
    (x : Var Γ (.keyed ℓ K V iK iV ord .exactlyOnce)) (f : Fin (mem ℓ) → V → W) (Δ : HList (Kind.delta (mem := mem)) Γ)
    (s : Node.State (.kmap iK iV iW x f)) (hΔ : IdleEnv m Δ) :
    NodeWired ctx m tk (.kmap iK iV iW x f) () () Δ s := by
  refine ⟨?_, ?_, ?_⟩
  · simp only [Node.step, Node.override, Node.enqueue]
  · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get, Kind.restrictDelta]
    simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
    try rfl
  · simp only [Node.step, IdleEnv, Kind.idleDelta]
    exact ⟨fun i hi => by rw [IdleEnv.get_keyed m hΔ x i hi]; rfl, trivial⟩

theorem wired_kallTicks {Γ : Ctx L mem} {ℓ : L} {K V : Type} (iK : DecidableEq K) (iV : DecidableEq V) {ord : StrOrd}
    (x : Var Γ (.atomicKeyed ℓ K V iK iV ord .exactlyOnce)) (Δ : HList (Kind.delta (mem := mem)) Γ) (s : Node.State (.kallTicks iK iV x))
    (hΔ : IdleEnv m Δ) :
    NodeWired ctx m tk (.kallTicks iK iV x) () () Δ s := by
  refine ⟨?_, ?_, ?_⟩
  · simp only [Node.step, Node.override, Node.enqueue]
  · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get, Kind.restrictDelta]
    simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
    try rfl
  · simp only [Node.step, IdleEnv, Kind.idleDelta]
    exact ⟨fun i hi => by rw [IdleEnv.get_atomicKeyed m hΔ x i hi]; rfl, trivial⟩

theorem wired_ksnapshot {Γ : Ctx L mem} {ℓ : L} {α K V : Type} (iα : DecidableEq α) (iK : DecidableEq K) (iV : DecidableEq V)
    {ord : StrOrd} {ret : Retries} {b : KSBound} (x : Var Γ (.ksing ℓ α K V iα iK iV ord ret b)) (d : Unit)
    (Δ : HList (Kind.delta (mem := mem)) Γ) (s : Node.State (.ksnapshot iα iK iV x d)) :
    NodeWired ctx m tk (.ksnapshot iα iK iV x d) () () Δ s := by
  refine ⟨?_, ?_, ?_⟩
  · simp only [Node.step, Node.override, Node.enqueue]
  · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get, Kind.restrictDelta]
    show HList.cons (Kind.restrictDelta m _ _) HList.nil = _
    refine Prod.ext (funext fun i => ?_) rfl
    show (if ctx.tick ℓ i.1 then some (Δ.get x i.1) else none) = if tk then some (Δ.get x i.1) else none
    rw [tick_hosted ctx m tk htick ℓ i]
  · simp only [Node.step, IdleEnv, Kind.idleDelta]
    exact ⟨fun i hi => by rw [tick_idle ctx m tk htick ℓ i hi]; rfl, trivial⟩

theorem wired_kfold {Γ : Ctx L mem} {ℓ : L} {K V A : Type} (iK : DecidableEq K) (iH : Hashable K) (iV : DecidableEq V) (iA : DecidableEq A)
    {ord : StrOrd} (g : A → V → A) (init : A) (Pre : V → V → Prop) (iPre : DecidableRel Pre) (ok : FoldOk ord .exactlyOnce Pre g)
    (x : Var Γ (.keyed ℓ K V iK iV ord .exactlyOnce)) (Δ : HList (Kind.delta (mem := mem)) Γ)
    (s : Node.State (.kfold iK iH iV iA g init Pre iPre ok x)) (hΔ : IdleEnv m Δ) :
    NodeWired ctx m tk (.kfold iK iH iV iA g init Pre iPre ok x) () () Δ s := by
  refine ⟨?_, ?_, ?_⟩
  · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, Node.override, Node.enqueue, envRestrict_get,
      Kind.restrictDelta]
    exact (override_pointwise m _ s _ (fun i hi => by
      rw [IdleEnv.get_keyed m hΔ x i hi]; rfl)).symm
  · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get, Kind.restrictDelta]
    simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
    try rfl
  · simp only [IdleEnv, Kind.idleDelta]
    exact ⟨trivial, trivial⟩

theorem wired_tfold {Γ : Ctx L mem} {ℓ : L} {α σ : Type} (iα : DecidableEq α) {ord : StrOrd} {ret : Retries}
    (g : σ → α → σ) (init : σ) (Pre : α → α → Prop) (iPre : DecidableRel Pre) (ok : FoldOk ord ret Pre g)
    (x : Var Γ (.atomicStream ℓ α iα ord ret)) (Δ : HList (Kind.delta (mem := mem)) Γ)
    (s : Node.State (.tfold iα g init Pre iPre ok x)) (hΔ : IdleEnv m Δ) :
    NodeWired ctx m tk (.tfold iα g init Pre iPre ok x) () () Δ s := by
  refine ⟨?_, ?_, ?_⟩
  · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, Node.override, Node.enqueue, envRestrict_get,
      Kind.restrictDelta]
    exact (override_pointwise m _ s _ (fun i hi => by rw [IdleEnv.get_atomicStream m hΔ x i hi]; rfl)).symm
  · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get, Kind.restrictDelta]
    simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
    try rfl
  · simp only [Node.step, IdleEnv, Kind.idleDelta]
    exact ⟨fun i hi => by rw [IdleEnv.get_atomicStream m hΔ x i hi]; rfl, trivial⟩

theorem wired_latest {Γ : Ctx L mem} {ℓ : L} {σ : Type} (iσ : DecidableEq σ) (x : Var Γ (.atomic ℓ (Option σ)))
    (Δ : HList (Kind.delta (mem := mem)) Γ)
    (s : Node.State (.latest iσ x)) (hΔ : IdleEnv m Δ) :
    NodeWired ctx m tk (.latest iσ x) () () Δ s := by
  refine ⟨?_, ?_, ?_⟩
  · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, Node.override, Node.enqueue, envRestrict_get,
      Kind.restrictDelta]
    exact (override_pointwise m _ s _ (fun i hi => by rw [IdleEnv.get_atomic m hΔ x i hi])).symm
  · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get, Kind.restrictDelta]
    simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
    try rfl
  · simp only [IdleEnv, Kind.idleDelta]
    exact ⟨trivial, trivial⟩

theorem wired_latest_monotone {Γ : Ctx L mem} {ℓ : L} (x : Var Γ (.atomic ℓ (Option Nat))) (Δ : HList (Kind.delta (mem := mem)) Γ)
    (s : Node.State (.latest_monotone x)) (hΔ : IdleEnv m Δ) :
    NodeWired ctx m tk (.latest_monotone x) () () Δ s := by
  refine ⟨?_, ?_, ?_⟩
  · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, Node.override, Node.enqueue, envRestrict_get,
      Kind.restrictDelta]
    exact (override_pointwise m _ s _ (fun i hi => by rw [IdleEnv.get_atomic m hΔ x i hi])).symm
  · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get, Kind.restrictDelta]
    simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
    try rfl
  · simp only [IdleEnv, Kind.idleDelta]
    exact ⟨trivial, trivial⟩

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

mutual

theorem Node.step_wired : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) → (sch : Node.Sched n) →
    (ora : Node.Oracle m n) → (Δ : HList (Kind.delta (mem := mem)) Γ) → (s : Node.State n) →
    IdleEnv m Δ → Node.OracleAt m n sch ora ctx.t s → Node.Drained n sch s →
    NodeWired ctx m tk n sch ora Δ s
  -- stateless pointwise, stream in / stream out
  | _, _, .map _ _ x f, _, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.override, Node.enqueue]
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => by rw [IdleEnv.get_stream m hΔ x i hi]; rfl, trivial⟩
  | _, _, .filterMap _ _ x f, _, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.override, Node.enqueue]
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => by rw [IdleEnv.get_stream m hΔ x i hi]; rfl, trivial⟩
  | _, _, .values _ _ x, _, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.override, Node.enqueue]
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => by rw [IdleEnv.get_keyed m hΔ x i hi]; rfl, trivial⟩
  | _, _, .into_keyed _ _ x, _, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.override, Node.enqueue]
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => IdleEnv.get_stream m hΔ x i hi, trivial⟩
  | _, _, .kassume_ordering _ _ x _, _, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.override, Node.enqueue]
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => IdleEnv.get_keyed m hΔ x i hi, trivial⟩
  | _, _, .kget_max_key _ _ _ _ x, _, _, Δ, s, _, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.override, Node.enqueue]
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [IdleEnv, Kind.idleDelta]
      exact ⟨trivial, trivial⟩
  | _, _, .weaken_retries _ x, _, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.override, Node.enqueue]
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => IdleEnv.get_stream m hΔ x i hi, trivial⟩
  | _, _, .union _ a b, _, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.override, Node.enqueue]
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => by rw [IdleEnv.get_stream m hΔ a i hi, IdleEnv.get_stream m hΔ b i hi]; rfl, trivial⟩
  | _, _, .assume_ordering _ x _, _, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.override, Node.enqueue]
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => IdleEnv.get_stream m hΔ x i hi, trivial⟩
  | _, _, .mapTick x f, _, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.override, Node.enqueue]
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => by rw [IdleEnv.get_atomic m hΔ x i hi]; rfl, trivial⟩
  | _, _, .firstTick _ _ x, _, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.override, Node.enqueue]
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => by rw [IdleEnv.get_atomicStream m hΔ x i hi]; rfl, trivial⟩
  | _, _, .allTicks _ x, _, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.override, Node.enqueue]
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => by rw [IdleEnv.get_atomicStream m hΔ x i hi]; rfl, trivial⟩
  | _, _, .flattenOrdered _ x, _, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.override, Node.enqueue]
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => IdleEnv.get_atomic m hΔ x i hi, trivial⟩
  | _, _, .flattenUnordered _ x, _, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.override, Node.enqueue]
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => IdleEnv.get_atomic m hΔ x i hi, trivial⟩
  | _, _, .tinto_keyed _ _ x, _, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.override, Node.enqueue]
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => IdleEnv.get_atomicStream m hΔ x i hi, trivial⟩
  | _, _, .tassume_ordering _ x _, _, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.override, Node.enqueue]
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get, Kind.restrictDelta]
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
  -- tick-gated, stateless
  | _, _, .snapshot (ℓ := ℓ) _ x _, _, _, Δ, s, _, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.override, Node.enqueue]
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get, Kind.restrictDelta]
      show HList.cons (Kind.restrictDelta m _ _) HList.nil = _
      refine Prod.ext (funext fun i => ?_) rfl
      show (if ctx.tick ℓ i.1 then some (Δ.get x i.1) else none) = if tk then some (Δ.get x i.1) else none
      rw [tick_hosted ctx m tk htick ℓ i]
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => by rw [tick_idle ctx m tk htick ℓ i hi]; rfl, trivial⟩
  -- stateful pointwise
  | _, _, .kfold_early_stop _ _ _ x f init, _, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, Node.override, Node.enqueue, envRestrict_get,
        Kind.restrictDelta]
      exact (override_pointwise m _ s _ (fun i hi => by rw [IdleEnv.get_keyed m hΔ x i hi]; rfl)).symm
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [IdleEnv, Kind.idleDelta]
      exact ⟨trivial, trivial⟩
  | _, _, .fold _ g _ _ _ _ x, _, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, Node.override, Node.enqueue, envRestrict_get,
        Kind.restrictDelta]
      exact (override_pointwise m _ s _ (fun i hi => by rw [IdleEnv.get_stream m hΔ x i hi]; rfl)).symm
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [IdleEnv, Kind.idleDelta]
      exact ⟨trivial, trivial⟩
  | _, _, .fold_monotone _ _ g _ _ _ x, _, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, Node.override, Node.enqueue, envRestrict_get,
        Kind.restrictDelta]
      exact (override_pointwise m _ s _ (fun i hi => by rw [IdleEnv.get_stream m hΔ x i hi]; rfl)).symm
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [IdleEnv, Kind.idleDelta]
      exact ⟨trivial, trivial⟩
  | _, _, .batch (ℓ := ℓ) _ _ _ x _, _, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, Node.override, Node.enqueue, envRestrict_get,
        Kind.restrictDelta]
      exact (override_pointwise' m ℓ s (fun i => if ctx.tick ℓ i then [] else s i ++ Δ.get x i)
        (fun i => if tk then [] else s i.1 ++ Δ.get x i.1)
        (fun i => by rw [tick_hosted ctx m tk htick ℓ i])
        (fun i hi => by rw [tick_idle ctx m tk htick ℓ i hi, IdleEnv.get_stream m hΔ x i hi]; simp)).symm
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get, Kind.restrictDelta]
      show HList.cons (Kind.restrictDelta m _ _) HList.nil = _
      refine Prod.ext (funext fun i => ?_) rfl
      show (if ctx.tick ℓ i.1 then some (s i.1 ++ Δ.get x i.1) else none) = if tk then some (s i.1 ++ Δ.get x i.1) else none
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
      show (if ctx.tick ℓ i.1 then List.replicate (d i.1 (s i.1)) () else []) = if tk then List.replicate (d i.1 (s i.1)) () else []
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
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, Node.override, Node.enqueue, envRestrict_get,
        Kind.restrictDelta]
      exact (override_pointwise m _ s _ hidle).symm
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      refine ⟨fun i hi => ?_, trivial⟩
      rw [IdleEnv.get_atomic m hΔ a i hi, IdleEnv.get_atomic m hΔ b i hi]
      simp only [Option.toList_none, List.append_nil]
      rcases hdr i with h | h <;> simp [h]
  | _, _, .defer_tick (ℓ := ℓ) _ x, _, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, Node.override, Node.enqueue, envRestrict_get,
        Kind.restrictDelta]
      refine (override_pointwise' m ℓ s
        (fun i => if ctx.tick ℓ i then (s i ++ (Δ.get x i).toList).tail else s i ++ (Δ.get x i).toList)
        (fun i => if tk then (s i.1 ++ (Δ.get x i.1).toList).tail else s i.1 ++ (Δ.get x i.1).toList)
        (fun i => ?_) (fun i hi => ?_)).symm
      · rw [tick_hosted ctx m tk htick ℓ i]
      · rw [tick_idle ctx m tk htick ℓ i hi, IdleEnv.get_atomic m hΔ x i hi, Option.toList_none, List.append_nil]
        rfl
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get, Kind.restrictDelta]
      show HList.cons (Kind.restrictDelta m _ _) HList.nil = _
      refine Prod.ext (funext fun i => ?_) rfl
      show (if ctx.tick ℓ i.1 then (s i.1 ++ (Δ.get x i.1).toList).head? else none)
        = if tk then (s i.1 ++ (Δ.get x i.1).toList).head? else none
      rw [tick_hosted ctx m tk htick ℓ i]
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      refine ⟨fun i hi => ?_, trivial⟩
      rw [tick_idle ctx m tk htick ℓ i hi]
      rfl
  | _, _, .reduce_watermark (ℓ := ℓ) _ _ _ _ x wm f _ _ _, _, _, Δ, s, hΔ, _, hdr => by
    have hidle : ∀ i, (⟨ℓ, i⟩ : Member L mem) ≠ m →
        ((((s i).1.1 ++ (Δ.get x i).toList).drop (min ((s i).1.1 ++ (Δ.get x i).toList).length ((s i).1.2 ++ (Δ.get wm i).toList).length),
          ((s i).1.2 ++ (Δ.get wm i).toList).drop (min ((s i).1.1 ++ (Δ.get x i).toList).length ((s i).1.2 ++ (Δ.get wm i).toList).length)),
         (scanRun (reduceWatermarkEager f) (s i).2
           (List.zip ((s i).1.1 ++ (Δ.get x i).toList) ((s i).1.2 ++ (Δ.get wm i).toList))).1) = s i :=
      fun i hi => by
        rw [IdleEnv.get_atomicKeyed m hΔ x i hi, IdleEnv.get_atomic m hΔ wm i hi]
        simp only [Option.toList_none, List.append_nil]
        rcases hdr i with h | h <;> simp [h, scanRun, Prod.ext_iff]
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, Node.override, Node.enqueue, envRestrict_get,
        Kind.restrictDelta]
      exact (override_pointwise m _ s _ hidle).symm
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      refine ⟨fun i hi => ?_, trivial⟩
      rw [IdleEnv.get_atomicKeyed m hΔ x i hi, IdleEnv.get_atomic m hΔ wm i hi]
      simp only [Option.toList_none, List.append_nil]
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
  -- the network
  | _, _, .broadcast_closed (c := c) (p := p) _ x, d, ora, Δ, s, hΔ, hora, _ => by
    have hidleS : ∀ j, (⟨c, j⟩ : Member L mem) ≠ m → Δ.get x j = [] := IdleEnv.get_stream m hΔ x
    have hc := chan_cells_wired ctx m hact d (fun j => Δ.get x j) s ora
      (fun j => if _ : (⟨c, j⟩ : Member L mem) = m then Δ.get x j else []) hora hidleS
      (fun j hj => by simp only [hj, dite_true])
      (fun j hj => by simp only [hj, dite_false])
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, Node.override, Node.enqueue, Node.enqueueLoc, envRestrict_get,
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
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get, Kind.restrictDelta]
      show HList.cons (Kind.restrictDelta m _ _) HList.nil = _
      refine Prod.ext (funext fun i => ?_) rfl
      show List.flatMap _ _ = List.flatMap _ _
      congr 1
      funext j
      rw [congrArg Prod.snd (hc.1 i j)]
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      refine ⟨fun i hi => ?_, trivial⟩
      rw [List.flatMap_eq_nil_iff]
      intro j _
      rw [hc.2.2 i j hi]
      rfl
  | _, _, .demux (c := c) (p := p) _ x addr, d, ora, Δ, s, hΔ, hora, _ => by
    have hidleS : ∀ j, (⟨c, j⟩ : Member L mem) ≠ m → Δ.get x j = [] := IdleEnv.get_stream m hΔ x
    have hc : ∀ i : Fin (mem p), _ := fun i => chan_cells_wired ctx m hact d
      (fun j => (Δ.get x j).filterMap (fun dx => if dx.1 = addr i then some dx.2 else none)) s ora
      (fun j => if _ : (⟨c, j⟩ : Member L mem) = m then
        (Δ.get x j).filterMap (fun dx => if dx.1 = addr i then some dx.2 else none) else [])
      hora (fun j hj => by rw [hidleS j hj]; rfl)
      (fun j hj => by simp only [hj, dite_true])
      (fun j hj => by simp only [hj, dite_false])
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, Node.override, Node.enqueue, Node.enqueueLoc, envRestrict_get,
        Kind.restrictDelta]
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
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, envRestrict_get, Kind.restrictDelta]
      show HList.cons (Kind.restrictDelta m _ _) HList.nil = _
      refine Prod.ext (funext fun i => ?_) rfl
      show List.flatMap _ _ = List.flatMap _ _
      congr 1
      funext j
      rw [congrArg Prod.snd ((hc i.1).1 i j)]
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      refine ⟨fun i hi => ?_, trivial⟩
      rw [List.flatMap_eq_nil_iff]
      intro j _
      rw [(hc i).2.2 i j hi]
      rfl
  -- composites
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
    -- the released feedback is the member's pending feedback, and nothing elsewhere
    have hΔ' : IdleEnv m ((knotFeed ctx ks s.2).append Δ) :=
      IdleEnv.append m _ Δ (knotFeed_idle ctx m hact ks s.2) hΔ
    have ih := Dag.step_wired body sch ora ((knotFeed ctx ks s.2).append Δ) s.1 hΔ' hora hdr
    simp only [DagWired] at ih
    have hr : envRestrict m ((knotFeed ctx ks s.2).append Δ) = (envRestrict m s.2).append (envRestrict m Δ) := by
      rw [envRestrict_append, knotFeed_restrict ctx m hact ks s.2]
    rw [hr] at ih
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict, Node.override, Node.enqueue, Node.enqueueLoc]
      rw [ih.1]
      show (_, knotKeep ctx ks s.2 (HList.take (Dag.step ctx body sch ((knotFeed ctx ks s.2).append Δ) s.1).2))
        = (_, envOverride m s.2 (HList.take (Dag.locStep m tk body ora _ (Dag.restrict m body s.1)).2.1))
      rw [← ih.2.1, ← envRestrict_take,
        knotKeep_override ctx m hact ks s.2 _ (IdleEnv.take m _ ih.2.2)]
    · simp only [Node.step, Node.locStep, memo_apply, memo2_apply, Node.restrict]
      rw [envRestrict_append, ← ih.2.1, envRestrict_drop, knotFeed_restrict ctx m hact ks s.2]
    · simp only [Node.step]
      exact IdleEnv.append m _ _ (knotFeed_idle ctx m hact ks s.2) (IdleEnv.drop m _ ih.2.2)

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
