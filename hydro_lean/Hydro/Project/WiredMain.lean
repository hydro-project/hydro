import Hydro.Project.WiredSpec

/-!
# Hydro · Project — wired machine: one step is one process

`Node.step_wired`/`Dag.step_wired`: a lock-step step in which **exactly**
the process `sel`'s members are active (`ctx.act = sel`) and tick iff the
process ticks (`ctx.tick = fun ℓ i => sel ℓ i && tk`), only its members
receive input increments (`IdleEnv`), and its oracle reads the cursors
(`OracleAt`) equals the process's own `locStep sel tk` written back over the
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
def NodeWired (ctx : StepCtx L mem) (sel : Sel L mem) (tk : Bool) {Γ : Ctx L mem} {κs : List (Kind L mem)}
    (n : Node Γ κs) (ora : Node.Oracle sel n) (Δ : HList (Kind.delta (mem := mem)) Γ) (s : Node.State n) : Prop :=
  let r := Node.locStep sel tk n ora (envRestrict sel Δ) (Node.restrict sel n s)
  (Node.step ctx n Δ s).1 = Node.override sel n (Node.enqueue sel n r.2.2 s) r.1 ∧
  envRestrict sel (Node.step ctx n Δ s).2 = r.2.1 ∧
  IdleEnv sel (Node.step ctx n Δ s).2

/-- The conclusions of a wired step for a graph. -/
def DagWired (ctx : StepCtx L mem) (sel : Sel L mem) (tk : Bool) {Γ : Ctx L mem} {outs : List (Kind L mem)}
    (d : Dag Γ outs) (ora : Dag.Oracle sel d) (Δ : HList (Kind.delta (mem := mem)) Γ) (s : Dag.State d) : Prop :=
  let r := Dag.locStep sel tk d ora (envRestrict sel Δ) (Dag.restrict sel d s)
  (Dag.step ctx d Δ s).1 = Dag.override sel d (Dag.enqueue sel d r.2.2 s) r.1 ∧
  envRestrict sel (Dag.step ctx d Δ s).2 = r.2.1 ∧
  IdleEnv sel (Dag.step ctx d Δ s).2

/-! ## Drained cells: every state the machine produces -/

mutual

theorem Node.init_drained : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    Node.Drained n (Node.init n)
  | _, _, .zipTick .. => fun _ => Or.inl rfl
  | _, _, .reduce_watermark .. => fun _ => Or.inl rfl
  | _, _, .tick_scan _ ins _ _ _ _ => fun _ => PendingOf.avail_empty ins
  | _, _, .sample_every _ _ d => fun i => by
    show sampleAdvance [] (d i) 0 = (0, [])
    rw [sampleAdvance]
    split
    · rfl
    · simp
  | _, _, .call N _ => Dag.init_drained N
  | _, _, .fixN _ body => Dag.init_drained body
  | _, _, .map .. => trivial
  | _, _, .filterMap .. => trivial
  | _, _, .broadcast_closed .. => trivial
  | _, _, .demux .. => trivial
  | _, _, .values .. => trivial
  | _, _, .into_keyed .. => trivial
  | _, _, .kassume_ordering .. => trivial
  | _, _, .kfold_early_stop .. => trivial
  | _, _, .kget_max_key .. => trivial
  | _, _, .weaken_retries .. => trivial
  | _, _, .union .. => trivial
  | _, _, .assume_ordering .. => trivial
  | _, _, .fold .. => trivial
  | _, _, .fold_monotone .. => trivial
  | _, _, .snapshot .. => trivial
  | _, _, .batch .. => trivial
  | _, _, .batch_ordered .. => trivial
  | _, _, .timeout_snapshot .. => trivial
  | _, _, .source_interval_batch .. => trivial
  | _, _, .mapTick .. => trivial
  | _, _, .defer_tick .. => trivial
  | _, _, .allTicks .. => trivial
  | _, _, .flattenOrdered .. => trivial
  | _, _, .flattenUnordered .. => trivial
  | _, _, .tinto_keyed .. => trivial
  | _, _, .tassume_ordering .. => trivial

theorem Dag.init_drained : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    Dag.Drained d (Dag.init d)
  | _, _, .ret _ => trivial
  | _, _, .letN n rest => ⟨Node.init_drained n, Dag.init_drained rest⟩

end

mutual

/-- After any step the multi-input tick cells are drained. -/
theorem Node.step_drained (ctx : StepCtx L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    (Δ : HList (Kind.delta (mem := mem)) Γ) → (s : Node.State n) → Node.Drained n (Node.step ctx n Δ s).1
  | _, _, .zipTick a b, Δ, s => fun i => by
    simp only [Node.step]
    exact zip_drop_min_empty _ _
  | _, _, .reduce_watermark _ _ _ _ x wm f _ _ _, Δ, s => fun i => by
    simp only [Node.step]
    exact zip_drop_min_empty _ _
  | _, _, .tick_scan _ ins _ x g _, Δ, s => fun i => by
    simp only [Node.step]
    exact PendingOf.avail_dropN_self ins _
  | _, _, .sample_every _ x d, Δ, s => fun i => by
    simp only [Node.step]
    exact sampleAdvance_fix _ _ _
  | _, _, .call N args, Δ, s => Dag.step_drained ctx N (args.map (fun v => Δ.get v)) s
  | _, _, .fixN ks body, Δ, s => Dag.step_drained ctx body _ s.1
  | _, _, .map .., _, _ => trivial
  | _, _, .filterMap .., _, _ => trivial
  | _, _, .broadcast_closed .., _, _ => trivial
  | _, _, .demux .., _, _ => trivial
  | _, _, .values .., _, _ => trivial
  | _, _, .into_keyed .., _, _ => trivial
  | _, _, .kassume_ordering .., _, _ => trivial
  | _, _, .kfold_early_stop .., _, _ => trivial
  | _, _, .kget_max_key .., _, _ => trivial
  | _, _, .weaken_retries .., _, _ => trivial
  | _, _, .union .., _, _ => trivial
  | _, _, .assume_ordering .., _, _ => trivial
  | _, _, .fold .., _, _ => trivial
  | _, _, .fold_monotone .., _, _ => trivial
  | _, _, .snapshot .., _, _ => trivial
  | _, _, .batch .., _, _ => trivial
  | _, _, .batch_ordered .., _, _ => trivial
  | _, _, .timeout_snapshot .., _, _ => trivial
  | _, _, .source_interval_batch .., _, _ => trivial
  | _, _, .mapTick .., _, _ => trivial
  | _, _, .defer_tick .., _, _ => trivial
  | _, _, .allTicks .., _, _ => trivial
  | _, _, .flattenOrdered .., _, _ => trivial
  | _, _, .flattenUnordered .., _, _ => trivial
  | _, _, .tinto_keyed .., _, _ => trivial
  | _, _, .tassume_ordering .., _, _ => trivial

theorem Dag.step_drained (ctx : StepCtx L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    (Δ : HList (Kind.delta (mem := mem)) Γ) → (s : Dag.State d) → Dag.Drained d (Dag.step ctx d Δ s).1
  | _, _, .ret _, _, _ => trivial
  | _, _, .letN n rest, Δ, s => ⟨Node.step_drained ctx n Δ s.1, Dag.step_drained ctx rest _ s.2⟩

end

/-! ## The knot feedback under a process's ticks -/

section Knots
variable (ctx : StepCtx L mem) (sel : Sel L mem) (hact : ctx.act = sel)
include hact

theorem KnotKind.feed_restrict : (k : KnotKind L mem) → (d : k.toKind.delta) →
    Kind.restrictDelta sel k.toKind (k.feed ctx d) = Kind.restrictDelta sel k.toKind d
  | .stream .., d => funext fun i => by
    show (if ctx.act _ i.1 then d i.1 else []) = d i.1
    rw [hact, i.2]; rfl
  | .atomic .., d => funext fun i => by
    show (if ctx.act _ i.1 then d i.1 else []) = d i.1
    rw [hact, i.2]; rfl

theorem KnotKind.feed_idle : (k : KnotKind L mem) → (d : k.toKind.delta) →
    Kind.idleDelta sel k.toKind (k.feed ctx d)
  | .stream .., d => fun i hi => by
    show (if ctx.act _ i then d i else []) = []
    rw [hact, hi]; rfl
  | .atomic .., d => fun i hi => by
    show (if ctx.act _ i then d i else []) = []
    rw [hact, hi]; rfl

/-- Keeping under the process's ticks: hosted members drop their pending
feedback and keep the body's new output; idle members keep theirs (the
body's output there is empty). -/
theorem KnotKind.keep_override : (k : KnotKind L mem) → (d inc : k.toKind.delta) →
    Kind.idleDelta sel k.toKind inc →
    k.keep ctx d inc = Kind.overrideDelta sel k.toKind d (Kind.restrictDelta sel k.toKind inc)
  | .stream .., d, inc, hinc => funext fun i => by
    show (if ctx.act _ i then [] else d i) ++ inc i = if h : sel _ i = true then inc i else d i
    rw [hact]
    split
    · exact List.nil_append _
    · rename_i h
      rw [hinc i (Bool.eq_false_iff.mpr h), List.append_nil]
  | .atomic .., d, inc, hinc => funext fun i => by
    show (if ctx.act _ i then [] else d i) ++ inc i = if h : sel _ i = true then inc i else d i
    rw [hact]
    split
    · exact List.nil_append _
    · rename_i h
      rw [hinc i (Bool.eq_false_iff.mpr h), List.append_nil]

theorem knotFeed_restrict : (ks : List (KnotKind L mem)) →
    (d : HList (Kind.delta (mem := mem)) (ks.map KnotKind.toKind)) →
    envRestrict sel (knotFeed ctx ks d) = envRestrict sel d
  | [], _ => rfl
  | k :: ks, d => by
    show (Kind.restrictDelta sel _ (k.feed ctx d.1), envRestrict sel (knotFeed ctx ks d.2))
      = (Kind.restrictDelta sel _ d.1, envRestrict sel d.2)
    rw [KnotKind.feed_restrict ctx sel hact k d.1, knotFeed_restrict ks d.2]

theorem knotFeed_idle : (ks : List (KnotKind L mem)) →
    (d : HList (Kind.delta (mem := mem)) (ks.map KnotKind.toKind)) →
    IdleEnv sel (knotFeed ctx ks d)
  | [], _ => trivial
  | k :: ks, d => ⟨KnotKind.feed_idle ctx sel hact k d.1, knotFeed_idle ks d.2⟩

theorem knotKeep_override : (ks : List (KnotKind L mem)) →
    (d inc : HList (Kind.delta (mem := mem)) (ks.map KnotKind.toKind)) → IdleEnv sel inc →
    knotKeep ctx ks d inc = envOverride sel d (envRestrict sel inc)
  | [], _, _, _ => rfl
  | k :: ks, d, inc, hinc => by
    show (k.keep ctx d.1 inc.1, knotKeep ctx ks d.2 inc.2)
      = (Kind.overrideDelta sel _ d.1 (Kind.restrictDelta sel _ inc.1), envOverride sel d.2 (envRestrict sel inc.2))
    rw [KnotKind.keep_override ctx sel hact k d.1 inc.1 hinc.1, knotKeep_override ks d.2 inc.2 hinc.2]

end Knots

/-- A per-member cell map agreeing with the process's cells at hosted
members and unchanged at idle ones is the write-back. -/
theorem override_pointwise' (sel : Sel L mem) (ℓ : L) {X : Type} (s F : Fin (mem ℓ) → X)
    (l : Hosted sel ℓ → X) (hhost : ∀ (i : Hosted sel ℓ), F i.1 = l i)
    (hidle : ∀ i, sel ℓ i = false → F i = s i) :
    (fun i => if h : sel ℓ i = true then l ⟨i, h⟩ else s i) = F := by
  funext i
  split
  · rename_i h; exact (hhost ⟨i, h⟩).symm
  · rename_i hn
    exact (hidle i (Bool.eq_false_iff.mpr hn)).symm

section Main

variable (ctx : StepCtx L mem) (sel : Sel L mem) (tk : Bool) (hact : ctx.act = sel)
  (htick : ctx.tick = fun ℓ i => sel ℓ i && tk)
include hact htick

theorem act_hosted (ℓ : L) (i : Hosted sel ℓ) : ctx.act ℓ i.1 = true := by rw [hact]; exact i.2
theorem act_idle (ℓ : L) (i : Fin (mem ℓ)) (hi : sel ℓ i = false) : ctx.act ℓ i = false := by rw [hact]; exact hi
theorem tick_hosted (ℓ : L) (i : Hosted sel ℓ) : ctx.tick ℓ i.1 = tk := by rw [htick]; simp [i.2]
theorem tick_idle (ℓ : L) (i : Fin (mem ℓ)) (hi : sel ℓ i = false) : ctx.tick ℓ i = false := by rw [htick]; simp [hi]

/-- The channel cells of a network node after a wired step, per receiver:
hosted receivers deliver per the oracle and take the hosted senders'
items; idle receivers only enqueue them. -/
theorem chan_cells_wired {c p : L} {α : Type} (d : Fin (mem p) → Fin (mem c) → Nat → Nat)
    (sendsG : Fin (mem c) → List α) (s : Fin (mem p) → Fin (mem c) → ChanCell α)
    (ora : Hosted sel p → Fin (mem c) → Nat) (sendsL : Fin (mem c) → List α)
    (hora : ∀ i j, ora i j = cumMax (d i.1 j) ctx.t)
    (hidleS : ∀ j, sel c j = false → sendsG j = [])
    (hsends : ∀ j, sel c j = true → sendsL j = sendsG j)
    (hsendsI : ∀ j, sel c j = false → sendsL j = []) :
    (∀ (i : Hosted sel p) j, (s i.1 j).stepG (ctx.act p i.1) (cumMax (d i.1 j) ctx.t) (sendsG j)
      = (s i.1 j).step (ora i j) (sendsL j)) ∧
    (∀ i j, sel p i = false →
      ((s i j).stepG (ctx.act p i) (cumMax (d i j) ctx.t) (sendsG j)).1
        = if _ : sel c j = true then (s i j).enqueue (sendsG j) else s i j) ∧
    (∀ i j, sel p i = false → ((s i j).stepG (ctx.act p i) (cumMax (d i j) ctx.t) (sendsG j)).2 = []) := by
  refine ⟨fun i j => ?_, fun i j hi => ?_, fun i j hi => ?_⟩
  · rw [act_hosted ctx sel tk hact htick p i, hora]
    simp only [ChanCell.stepG, if_true]
    by_cases hj : sel c j = true
    · rw [hsends j hj]
    · rw [hsendsI j (Bool.eq_false_iff.mpr hj), hidleS j (Bool.eq_false_iff.mpr hj)]
  · rw [act_idle ctx sel tk hact htick p i hi]
    simp only [ChanCell.stepG, Bool.false_eq_true, if_false]
    split
    · rfl
    · rename_i hj
      rw [hidleS j (Bool.eq_false_iff.mpr hj)]
      show ChanCell.mk ((s i j).pending ++ []) _ = s i j
      rw [List.append_nil]
  · rw [act_idle ctx sel tk hact htick p i hi]
    simp only [ChanCell.stepG, Bool.false_eq_true, if_false]

-- `hact`/`htick` are used by the node theorem; the graph theorem of the mutual block carries them along
set_option linter.unusedSectionVars false

mutual

theorem Node.step_wired : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    (ora : Node.Oracle sel n) → (Δ : HList (Kind.delta (mem := mem)) Γ) → (s : Node.State n) →
    IdleEnv sel Δ → Node.OracleAt sel n ora ctx.t → Node.Drained n s →
    NodeWired ctx sel tk n ora Δ s
  -- stateless pointwise, stream in / stream out
  | _, _, .map _ _ x f, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.override, Node.enqueue]
    · simp only [Node.step, Node.locStep, Node.restrict, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => by rw [IdleEnv.get_stream sel hΔ x i hi]; rfl, trivial⟩
  | _, _, .filterMap _ _ x f, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.override, Node.enqueue]
    · simp only [Node.step, Node.locStep, Node.restrict, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => by rw [IdleEnv.get_stream sel hΔ x i hi]; rfl, trivial⟩
  | _, _, .values _ _ x, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.override, Node.enqueue]
    · simp only [Node.step, Node.locStep, Node.restrict, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => by rw [IdleEnv.get_keyed sel hΔ x i hi]; rfl, trivial⟩
  | _, _, .into_keyed _ _ x, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.override, Node.enqueue]
    · simp only [Node.step, Node.locStep, Node.restrict, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => IdleEnv.get_stream sel hΔ x i hi, trivial⟩
  | _, _, .kassume_ordering _ _ x _, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.override, Node.enqueue]
    · simp only [Node.step, Node.locStep, Node.restrict, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => IdleEnv.get_keyed sel hΔ x i hi, trivial⟩
  | _, _, .kget_max_key _ _ _ _ x, _, Δ, s, _, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.override, Node.enqueue]
    · simp only [Node.step, Node.locStep, Node.restrict, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [IdleEnv, Kind.idleDelta]
      exact ⟨trivial, trivial⟩
  | _, _, .weaken_retries _ x, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.override, Node.enqueue]
    · simp only [Node.step, Node.locStep, Node.restrict, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => IdleEnv.get_stream sel hΔ x i hi, trivial⟩
  | _, _, .union _ a b, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.override, Node.enqueue]
    · simp only [Node.step, Node.locStep, Node.restrict, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => by rw [IdleEnv.get_stream sel hΔ a i hi, IdleEnv.get_stream sel hΔ b i hi]; rfl, trivial⟩
  | _, _, .assume_ordering _ x _, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.override, Node.enqueue]
    · simp only [Node.step, Node.locStep, Node.restrict, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => IdleEnv.get_stream sel hΔ x i hi, trivial⟩
  | _, _, .mapTick x f, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.override, Node.enqueue]
    · simp only [Node.step, Node.locStep, Node.restrict, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => by rw [IdleEnv.get_atomic sel hΔ x i hi]; rfl, trivial⟩
  | _, _, .allTicks _ x, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.override, Node.enqueue]
    · simp only [Node.step, Node.locStep, Node.restrict, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => by rw [IdleEnv.get_atomicStream sel hΔ x i hi]; rfl, trivial⟩
  | _, _, .flattenOrdered _ x, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.override, Node.enqueue]
    · simp only [Node.step, Node.locStep, Node.restrict, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => IdleEnv.get_atomic sel hΔ x i hi, trivial⟩
  | _, _, .flattenUnordered _ x, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.override, Node.enqueue]
    · simp only [Node.step, Node.locStep, Node.restrict, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => IdleEnv.get_atomic sel hΔ x i hi, trivial⟩
  | _, _, .tinto_keyed _ _ x, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.override, Node.enqueue]
    · simp only [Node.step, Node.locStep, Node.restrict, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => IdleEnv.get_atomicStream sel hΔ x i hi, trivial⟩
  | _, _, .tassume_ordering _ x _, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.override, Node.enqueue]
    · simp only [Node.step, Node.locStep, Node.restrict, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => IdleEnv.get_atomicStream sel hΔ x i hi, trivial⟩
  -- tick-gated, stateless
  | _, _, .snapshot (ℓ := ℓ) _ x _, _, Δ, s, _, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.override, Node.enqueue]
    · simp only [Node.step, Node.locStep, Node.restrict, envRestrict_get, Kind.restrictDelta]
      show HList.cons (Kind.restrictDelta sel _ _) HList.nil = _
      refine Prod.ext (funext fun i => ?_) rfl
      show (if ctx.tick ℓ i.1 then [Δ.get x i.1] else []) = if tk then [Δ.get x i.1] else []
      rw [tick_hosted ctx sel tk hact htick ℓ i]
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => by rw [tick_idle ctx sel tk hact htick ℓ i hi]; rfl, trivial⟩
  -- stateful pointwise
  | _, _, .kfold_early_stop _ _ _ x f init, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.locStep, Node.restrict, Node.override, Node.enqueue, envRestrict_get,
        Kind.restrictDelta]
      exact (override_pointwise sel _ s _ (fun i hi => by rw [IdleEnv.get_keyed sel hΔ x i hi]; rfl)).symm
    · simp only [Node.step, Node.locStep, Node.restrict, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [IdleEnv, Kind.idleDelta]
      exact ⟨trivial, trivial⟩
  | _, _, .fold _ g _ _ _ _ x, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.locStep, Node.restrict, Node.override, Node.enqueue, envRestrict_get,
        Kind.restrictDelta]
      exact (override_pointwise sel _ s _ (fun i hi => by rw [IdleEnv.get_stream sel hΔ x i hi]; rfl)).symm
    · simp only [Node.step, Node.locStep, Node.restrict, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [IdleEnv, Kind.idleDelta]
      exact ⟨trivial, trivial⟩
  | _, _, .fold_monotone _ _ g _ _ _ x, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.locStep, Node.restrict, Node.override, Node.enqueue, envRestrict_get,
        Kind.restrictDelta]
      exact (override_pointwise sel _ s _ (fun i hi => by rw [IdleEnv.get_stream sel hΔ x i hi]; rfl)).symm
    · simp only [Node.step, Node.locStep, Node.restrict, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [IdleEnv, Kind.idleDelta]
      exact ⟨trivial, trivial⟩
  | _, _, .batch (ℓ := ℓ) _ x _, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.locStep, Node.restrict, Node.override, Node.enqueue, envRestrict_get,
        Kind.restrictDelta]
      exact (override_pointwise' sel ℓ s (fun i => if ctx.tick ℓ i then [] else s i ++ Δ.get x i)
        (fun i => if tk then [] else s i.1 ++ Δ.get x i.1)
        (fun i => by rw [tick_hosted ctx sel tk hact htick ℓ i])
        (fun i hi => by rw [tick_idle ctx sel tk hact htick ℓ i hi, IdleEnv.get_stream sel hΔ x i hi]; simp)).symm
    · simp only [Node.step, Node.locStep, Node.restrict, envRestrict_get, Kind.restrictDelta]
      show HList.cons (Kind.restrictDelta sel _ _) HList.nil = _
      refine Prod.ext (funext fun i => ?_) rfl
      show (if ctx.tick ℓ i.1 then [s i.1 ++ Δ.get x i.1] else []) = if tk then [s i.1 ++ Δ.get x i.1] else []
      rw [tick_hosted ctx sel tk hact htick ℓ i]
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => by rw [tick_idle ctx sel tk hact htick ℓ i hi]; rfl, trivial⟩
  | _, _, .batch_ordered (ℓ := ℓ) _ x _, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.locStep, Node.restrict, Node.override, Node.enqueue, envRestrict_get,
        Kind.restrictDelta]
      exact (override_pointwise' sel ℓ s (fun i => if ctx.tick ℓ i then [] else s i ++ Δ.get x i)
        (fun i => if tk then [] else s i.1 ++ Δ.get x i.1)
        (fun i => by rw [tick_hosted ctx sel tk hact htick ℓ i])
        (fun i hi => by rw [tick_idle ctx sel tk hact htick ℓ i hi, IdleEnv.get_stream sel hΔ x i hi]; simp)).symm
    · simp only [Node.step, Node.locStep, Node.restrict, envRestrict_get, Kind.restrictDelta]
      show HList.cons (Kind.restrictDelta sel _ _) HList.nil = _
      refine Prod.ext (funext fun i => ?_) rfl
      show (if ctx.tick ℓ i.1 then [s i.1 ++ Δ.get x i.1] else []) = if tk then [s i.1 ++ Δ.get x i.1] else []
      rw [tick_hosted ctx sel tk hact htick ℓ i]
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => by rw [tick_idle ctx sel tk hact htick ℓ i hi]; rfl, trivial⟩
  | _, _, .sample_every (ℓ := ℓ) _ x d, _, Δ, s, hΔ, _, hdr => by
    have hidle : ∀ i, sel ℓ i = false →
        ((s i).1 ++ Δ.get x i, (sampleAdvance ((s i).1 ++ Δ.get x i) (d i) (s i).2).1) = s i := fun i hi => by
      rw [IdleEnv.get_atomic sel hΔ x i hi, List.append_nil, hdr i]
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.locStep, Node.restrict, Node.override, Node.enqueue, envRestrict_get,
        Kind.restrictDelta]
      exact (override_pointwise sel _ s _ hidle).symm
    · simp only [Node.step, Node.locStep, Node.restrict, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => by rw [IdleEnv.get_atomic sel hΔ x i hi, List.append_nil, hdr i], trivial⟩
  | _, _, .timeout_snapshot (ℓ := ℓ) _ _ d, _, Δ, s, _, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.locStep, Node.restrict, Node.override, Node.enqueue]
      exact (override_pointwise' sel ℓ s (fun i => if ctx.tick ℓ i then s i + 1 else s i) (fun i => if tk then s i.1 + 1 else s i.1)
        (fun i => by rw [tick_hosted ctx sel tk hact htick ℓ i])
        (fun i hi => by rw [tick_idle ctx sel tk hact htick ℓ i hi]; rfl)).symm
    · simp only [Node.step, Node.locStep, Node.restrict]
      show HList.cons (Kind.restrictDelta sel _ _) HList.nil = _
      refine Prod.ext (funext fun i => ?_) rfl
      show (if ctx.tick ℓ i.1 then ((d i.1)[s i.1]?).toList else []) = if tk then ((d i.1)[s i.1]?).toList else []
      rw [tick_hosted ctx sel tk hact htick ℓ i]
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => by rw [tick_idle ctx sel tk hact htick ℓ i hi]; rfl, trivial⟩
  | _, _, .source_interval_batch (ℓ := ℓ) d, _, Δ, s, _, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.locStep, Node.restrict, Node.override, Node.enqueue]
      exact (override_pointwise' sel ℓ s (fun i => if ctx.tick ℓ i then s i + 1 else s i) (fun i => if tk then s i.1 + 1 else s i.1)
        (fun i => by rw [tick_hosted ctx sel tk hact htick ℓ i])
        (fun i hi => by rw [tick_idle ctx sel tk hact htick ℓ i hi]; rfl)).symm
    · simp only [Node.step, Node.locStep, Node.restrict]
      show HList.cons (Kind.restrictDelta sel _ _) HList.nil = _
      refine Prod.ext (funext fun i => ?_) rfl
      show (if ctx.tick ℓ i.1 then ((d i.1)[s i.1]?).toList else []) = if tk then ((d i.1)[s i.1]?).toList else []
      rw [tick_hosted ctx sel tk hact htick ℓ i]
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      exact ⟨fun i hi => by rw [tick_idle ctx sel tk hact htick ℓ i hi]; rfl, trivial⟩
  | _, _, .zipTick (ℓ := ℓ) a b, _, Δ, s, hΔ, _, hdr => by
    have hidle : ∀ i, sel ℓ i = false →
        (((s i).1 ++ Δ.get a i).drop (min ((s i).1 ++ Δ.get a i).length ((s i).2 ++ Δ.get b i).length),
         ((s i).2 ++ Δ.get b i).drop (min ((s i).1 ++ Δ.get a i).length ((s i).2 ++ Δ.get b i).length)) = s i :=
      fun i hi => by
        rw [IdleEnv.get_atomic sel hΔ a i hi, IdleEnv.get_atomic sel hΔ b i hi, List.append_nil, List.append_nil]
        rcases hdr i with h | h <;> simp [h, Prod.ext_iff]
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.locStep, Node.restrict, Node.override, Node.enqueue, envRestrict_get,
        Kind.restrictDelta]
      exact (override_pointwise sel _ s _ hidle).symm
    · simp only [Node.step, Node.locStep, Node.restrict, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      refine ⟨fun i hi => ?_, trivial⟩
      rw [IdleEnv.get_atomic sel hΔ a i hi, IdleEnv.get_atomic sel hΔ b i hi, List.append_nil, List.append_nil]
      rcases hdr i with h | h <;> simp [h, Prod.ext_iff]
  | _, _, .defer_tick (ℓ := ℓ) init x, _, Δ, s, hΔ, _, _ => by
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.locStep, Node.restrict, Node.override, Node.enqueue, envRestrict_get,
        Kind.restrictDelta]
      refine (override_pointwise' sel ℓ s
        (fun i => if (s i).1 then s i else if ctx.tick ℓ i then (true, []) else (false, (s i).2 ++ Δ.get x i))
        (fun i => if (s i.1).1 then s i.1 else if tk then (true, []) else (false, (s i.1).2 ++ Δ.get x i.1))
        (fun i => ?_) (fun i hi => ?_)).symm
      · rw [tick_hosted ctx sel tk hact htick ℓ i]
      · rw [tick_idle ctx sel tk hact htick ℓ i hi, IdleEnv.get_atomic sel hΔ x i hi, List.append_nil]
        split
        · rfl
        · rename_i h
          exact (Prod.ext (Bool.eq_false_iff.mpr h).symm rfl)
    · simp only [Node.step, Node.locStep, Node.restrict, envRestrict_get, Kind.restrictDelta]
      show HList.cons (Kind.restrictDelta sel _ _) HList.nil = _
      refine Prod.ext (funext fun i => ?_) rfl
      show (if (s i.1).1 then Δ.get x i.1 else if ctx.tick ℓ i.1 then init :: ((s i.1).2 ++ Δ.get x i.1) else [])
        = if (s i.1).1 then Δ.get x i.1 else if tk then init :: ((s i.1).2 ++ Δ.get x i.1) else []
      rw [tick_hosted ctx sel tk hact htick ℓ i]
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      refine ⟨fun i hi => ?_, trivial⟩
      rw [tick_idle ctx sel tk hact htick ℓ i hi, IdleEnv.get_atomic sel hΔ x i hi]
      split <;> rfl
  | _, _, .reduce_watermark (ℓ := ℓ) _ _ _ _ x wm f _ _ _, _, Δ, s, hΔ, _, hdr => by
    have hidle : ∀ i, sel ℓ i = false →
        ((((s i).1.1 ++ Δ.get x i).drop (min ((s i).1.1 ++ Δ.get x i).length ((s i).1.2 ++ Δ.get wm i).length),
          ((s i).1.2 ++ Δ.get wm i).drop (min ((s i).1.1 ++ Δ.get x i).length ((s i).1.2 ++ Δ.get wm i).length)),
         (scanRun (reduceWatermarkEager f) (s i).2
           (List.zip ((s i).1.1 ++ Δ.get x i) ((s i).1.2 ++ Δ.get wm i))).1) = s i :=
      fun i hi => by
        rw [IdleEnv.get_atomicKeyed sel hΔ x i hi, IdleEnv.get_atomic sel hΔ wm i hi, List.append_nil, List.append_nil]
        rcases hdr i with h | h <;> simp [h, scanRun, Prod.ext_iff]
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.locStep, Node.restrict, Node.override, Node.enqueue, envRestrict_get,
        Kind.restrictDelta]
      exact (override_pointwise sel _ s _ hidle).symm
    · simp only [Node.step, Node.locStep, Node.restrict, envRestrict_get, Kind.restrictDelta]
      simp only [envRestrict, HList.map, HList.cons, HList.nil, Kind.restrictDelta]
      try rfl
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      refine ⟨fun i hi => ?_, trivial⟩
      rw [IdleEnv.get_atomicKeyed sel hΔ x i hi, IdleEnv.get_atomic sel hΔ wm i hi, List.append_nil, List.append_nil]
      rcases hdr i with h | h <;> simp [h, scanRun, Prod.ext_iff]
  | _, _, .tick_scan (ℓ := ℓ) sts ins outs x g seed, _, Δ, s, hΔ, _, hdr => by
    have hleaf : IdleEnv sel (readShapeDelta Δ ℓ ins x) := readShapeDelta_idle sel Δ hΔ ℓ ins x
    have hpush : ∀ i, sel ℓ i = false →
        PendingOf.push ins (s i).1 (fun _ => readShapeDelta Δ ℓ ins x) i = (s i).1 :=
      fun i hi => PendingOf.push_idle sel ins (s i).1 _ i hleaf hi
    have hrun : ∀ i, sel ℓ i = false →
        scanRun (g i) (s i).2 (PendingOf.takeZip ins (PendingOf.push ins (s i).1 (fun _ => readShapeDelta Δ ℓ ins x) i)
          (PendingOf.avail ins (PendingOf.push ins (s i).1 (fun _ => readShapeDelta Δ ℓ ins x) i))) = ((s i).2, []) :=
      fun i hi => by rw [hpush i hi, hdr i, PendingOf.takeZip_zero]; rfl
    have hloc : ∀ i : Hosted sel ℓ,
        PendingOf.pushLoc sel ins (s i.1).1 (fun _ => readShapeLoc sel (envRestrict sel Δ) ℓ ins x) i
          = PendingOf.push ins (s i.1).1 (fun _ => readShapeDelta Δ ℓ ins x) i.1 := fun i => by
      rw [readShapeLoc_restrict]
      exact PendingOf.pushLoc_restrict sel ins (s i.1).1 (fun _ => readShapeDelta Δ ℓ ins x) i
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.locStep, Node.restrict, Node.override, Node.enqueue, hloc]
      refine (override_pointwise sel _ s _ (fun i hi => ?_)).symm
      rw [hrun i hi, hpush i hi, hdr i, PendingOf.dropN_zero]
    · simp only [Node.step, Node.locStep, Node.restrict, hloc]
      rw [unzipShape_restrict]
    · simp only [Node.step]
      exact unzipShape_idle sel ℓ outs _ (fun i hi => by rw [hrun i hi])
  -- the network
  | _, _, .broadcast_closed (c := c) (p := p) _ d x, ora, Δ, s, hΔ, hora, _ => by
    have hidleS : ∀ j, sel c j = false → Δ.get x j = [] := IdleEnv.get_stream sel hΔ x
    have hc := chan_cells_wired ctx sel tk hact htick d (fun j => Δ.get x j) s ora
      (fun j => if _ : sel c j = true then Δ.get x j else []) hora hidleS
      (fun j hj => by simp only [hj, dite_true])
      (fun j hj => by simp only [hj, Bool.false_eq_true, dite_false])
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.locStep, Node.restrict, Node.override, Node.enqueue, envRestrict_get,
        Kind.restrictDelta]
      funext i
      split
      · rename_i hi
        funext j
        rw [hc.1 ⟨i, hi⟩ j]
      · rename_i hi
        funext j
        have hi' := Bool.eq_false_iff.mpr hi
        rw [hc.2.1 i j hi']
    · simp only [Node.step, Node.locStep, Node.restrict, envRestrict_get, Kind.restrictDelta]
      show HList.cons (Kind.restrictDelta sel _ _) HList.nil = _
      refine Prod.ext (funext fun i => ?_) rfl
      show List.flatMap _ _ = List.flatMap _ _
      congr 1
      funext j
      rw [hc.1 i j]
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      refine ⟨fun i hi => ?_, trivial⟩
      rw [List.flatMap_eq_nil_iff]
      intro j _
      rw [hc.2.2 i j hi]
      rfl
  | _, _, .demux (c := c) (p := p) _ d x addr, ora, Δ, s, hΔ, hora, _ => by
    have hidleS : ∀ j, sel c j = false → Δ.get x j = [] := IdleEnv.get_stream sel hΔ x
    have hc : ∀ i : Fin (mem p), _ := fun i => chan_cells_wired ctx sel tk hact htick d
      (fun j => (Δ.get x j).filterMap (fun dx => if dx.1 = addr i then some dx.2 else none)) s ora
      (fun j => if _ : sel c j = true then
        (Δ.get x j).filterMap (fun dx => if dx.1 = addr i then some dx.2 else none) else [])
      hora (fun j hj => by rw [hidleS j hj]; rfl)
      (fun j hj => by simp only [hj, dite_true])
      (fun j hj => by simp only [hj, Bool.false_eq_true, dite_false])
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.locStep, Node.restrict, Node.override, Node.enqueue, envRestrict_get,
        Kind.restrictDelta]
      funext i
      split
      · rename_i hi
        funext j
        rw [(hc i).1 ⟨i, hi⟩ j]
      · rename_i hi
        funext j
        have hi' := Bool.eq_false_iff.mpr hi
        rw [(hc i).2.1 i j hi']
    · simp only [Node.step, Node.locStep, Node.restrict, envRestrict_get, Kind.restrictDelta]
      show HList.cons (Kind.restrictDelta sel _ _) HList.nil = _
      refine Prod.ext (funext fun i => ?_) rfl
      show List.flatMap _ _ = List.flatMap _ _
      congr 1
      funext j
      rw [(hc i.1).1 i j]
    · simp only [Node.step, IdleEnv, Kind.idleDelta]
      refine ⟨fun i hi => ?_, trivial⟩
      rw [List.flatMap_eq_nil_iff]
      intro j _
      rw [(hc i).2.2 i j hi]
      rfl
  -- composites
  | _, _, .call N args, ora, Δ, s, hΔ, hora, hdr => by
    have ih := Dag.step_wired N ora (args.map (fun v => Δ.get v)) s (IdleEnv.mapSubst sel Δ hΔ args) hora hdr
    simp only [DagWired] at ih
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.locStep, Node.restrict, Node.override, Node.enqueue]
      rw [← envRestrict_mapSubst]
      exact ih.1
    · simp only [Node.step, Node.locStep, Node.restrict]
      rw [← envRestrict_mapSubst]
      exact ih.2.1
    · simp only [Node.step]
      exact ih.2.2
  | _, _, .fixN ks body, ora, Δ, s, hΔ, hora, hdr => by
    -- the released feedback is the hosted pending feedback, and nothing elsewhere
    have hΔ' : IdleEnv sel ((knotFeed ctx ks s.2).append Δ) :=
      IdleEnv.append sel _ Δ (knotFeed_idle ctx sel hact ks s.2) hΔ
    have ih := Dag.step_wired body ora ((knotFeed ctx ks s.2).append Δ) s.1 hΔ' hora hdr
    simp only [DagWired] at ih
    have hr : envRestrict sel ((knotFeed ctx ks s.2).append Δ) = (envRestrict sel s.2).append (envRestrict sel Δ) := by
      rw [envRestrict_append, knotFeed_restrict ctx sel hact ks s.2]
    rw [hr] at ih
    refine ⟨?_, ?_, ?_⟩
    · simp only [Node.step, Node.locStep, Node.restrict, Node.override, Node.enqueue]
      rw [ih.1]
      show (_, knotKeep ctx ks s.2 (HList.take (Dag.step ctx body ((knotFeed ctx ks s.2).append Δ) s.1).2))
        = (_, envOverride sel s.2 (HList.take (Dag.locStep sel tk body ora _ (Dag.restrict sel body s.1)).2.1))
      rw [← ih.2.1, ← envRestrict_take,
        knotKeep_override ctx sel hact ks s.2 _ (IdleEnv.take sel _ ih.2.2)]
    · simp only [Node.step, Node.locStep, Node.restrict]
      rw [envRestrict_append, ← ih.2.1, envRestrict_drop, knotFeed_restrict ctx sel hact ks s.2]
    · simp only [Node.step]
      exact IdleEnv.append sel _ _ (knotFeed_idle ctx sel hact ks s.2) (IdleEnv.drop sel _ ih.2.2)

theorem Dag.step_wired : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    (ora : Dag.Oracle sel d) → (Δ : HList (Kind.delta (mem := mem)) Γ) → (s : Dag.State d) →
    IdleEnv sel Δ → Dag.OracleAt sel d ora ctx.t → Dag.Drained d s →
    DagWired ctx sel tk d ora Δ s
  | _, _, .ret xs, _, Δ, s, hΔ, _, _ => by
    refine ⟨rfl, ?_, IdleEnv.mapSubst sel Δ hΔ xs⟩
    simp only [Dag.step, Dag.locStep, Dag.restrict]
    exact envRestrict_mapSubst sel Δ xs
  | _, _, .letN n rest, ora, Δ, s, hΔ, hora, hdr => by
    have ihn := Node.step_wired n ora.1 Δ s.1 hΔ hora.1 hdr.1
    simp only [NodeWired] at ihn
    have hΔ' : IdleEnv sel ((Node.step ctx n Δ s.1).2.append Δ) := IdleEnv.append sel _ _ ihn.2.2 hΔ
    have ihr := Dag.step_wired rest ora.2 ((Node.step ctx n Δ s.1).2.append Δ) s.2 hΔ' hora.2 hdr.2
    simp only [DagWired] at ihr
    have hr : envRestrict sel ((Node.step ctx n Δ s.1).2.append Δ)
        = (Node.locStep sel tk n ora.1 (envRestrict sel Δ) (Node.restrict sel n s.1)).2.1.append (envRestrict sel Δ) := by
      rw [envRestrict_append, ihn.2.1]
    refine ⟨?_, ?_, ?_⟩
    · simp only [Dag.step, Dag.locStep, Dag.restrict, Dag.override, Dag.enqueue, Node.memoState_eq, memoEnv_eq]
      rw [← hr, ihn.1, ihr.1]
    · simp only [Dag.step, Dag.locStep, Dag.restrict, memoEnv_eq]
      rw [← hr, ihr.2.1]
    · simp only [Dag.step]
      exact ihr.2.2

end

end Main

end Project

end Hydro
