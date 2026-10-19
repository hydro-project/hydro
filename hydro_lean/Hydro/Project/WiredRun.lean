import Hydro.Project.WiredMain

/-!
# Hydro · Project — wired machine: the run, per process

**The correspondence.** A process is a cluster member (`Member`,
`Sel.single`). An interleaving says which member runs at each clocked
step (`who`), whether its tick bodies run or it only stutters — its
asynchronous dataflow progressing: deliveries land, knot feedback
propagates (`tick`) — and which input increments arrive (`inp`); the
running member is exactly the active one (`pacing`), and the inputs at
other members are ignored (`env` — they cannot be consumed by a sleeping
process; an interleaving lists them at the step their process runs). A
DFIR tick is "stutters, then a tick" at one member (D76).
**Unconditionally**, every step of the lock-step run **is** the active
process's `locStep` on its own view, written back, with its sends
appended to every other process's view (`view_active`/`view_idle`); and
its output increments are the machine denotation's increments at its
members (`outputs`, via `Dag.run_spec`).

So the processes, each running `Dag.locStep` on its own cells and
exchanging only `Sends`, compute exactly the `SchedSem` run of the graph
under the pacing `I.pacing` and the cursors baked into `d` — any wired
execution is a `SchedSem` run, and every `SchedSem` headline applies to
it. No settling, no quiet, no idle-process condition: a member that is
not active is inert (`Project/Step.lean`), so a sleeping process's cells
are exactly the lock-step machine's.
-/

namespace Hydro

namespace Project

variable {L : Type} {mem : L → Nat}

/-! ## Views: the global state cut to a process -/

theorem restrict_override_self_pw (sel : Sel L mem) (ℓ : L) {X : Type} (s : Fin (mem ℓ) → X)
    (l : Hosted sel ℓ → X) :
    (fun (i : Hosted sel ℓ) => (fun i => if h : sel ℓ i = true then l ⟨i, h⟩ else s i) i.1) = l := by
  funext i
  simp only [i.2, dite_true]

theorem restrict_override_other_pw (sel sel' : Sel L mem) (hd : ∀ ℓ i, sel' ℓ i = true → sel ℓ i = false)
    (ℓ : L) {X : Type} (s : Fin (mem ℓ) → X) (l : Hosted sel ℓ → X) :
    (fun (i : Hosted sel' ℓ) => (fun i => if h : sel ℓ i = true then l ⟨i, h⟩ else s i) i.1) = fun i => s i.1 := by
  funext i
  simp only [hd ℓ i.1 i.2, Bool.false_eq_true, dite_false]

theorem envRestrict_override_self (sel : Sel L mem) : {Γ : Ctx L mem} → (Δ : HList (Kind.delta (mem := mem)) Γ) →
    (Δ' : HList (Kind.locDelta sel) Γ) → envRestrict sel (envOverride sel Δ Δ') = Δ'
  | [], _, _ => rfl
  | κ :: _, Δ, Δ' => by
    show (Kind.restrictDelta sel κ (Kind.overrideDelta sel κ Δ.1 Δ'.1), envRestrict sel (envOverride sel Δ.2 Δ'.2))
      = (Δ'.1, Δ'.2)
    rw [envRestrict_override_self sel Δ.2 Δ'.2]
    cases κ <;> exact congrArg (·, Δ'.2) (funext fun i => by simp only [Kind.restrictDelta, Kind.overrideDelta, i.2, dite_true])

theorem envRestrict_override_other (sel sel' : Sel L mem) (hd : ∀ ℓ i, sel' ℓ i = true → sel ℓ i = false) :
    {Γ : Ctx L mem} → (Δ : HList (Kind.delta (mem := mem)) Γ) → (Δ' : HList (Kind.locDelta sel) Γ) →
    envRestrict sel' (envOverride sel Δ Δ') = envRestrict sel' Δ
  | [], _, _ => rfl
  | κ :: _, Δ, Δ' => by
    show (Kind.restrictDelta sel' κ (Kind.overrideDelta sel κ Δ.1 Δ'.1), envRestrict sel' (envOverride sel Δ.2 Δ'.2))
      = (Kind.restrictDelta sel' κ Δ.1, envRestrict sel' Δ.2)
    rw [envRestrict_override_other sel sel' hd Δ.2 Δ'.2]
    cases κ <;> exact congrArg (·, envRestrict sel' Δ.2)
      (funext fun i => by simp only [Kind.restrictDelta, Kind.overrideDelta, hd _ i.1 i.2, Bool.false_eq_true, dite_false])

mutual

/-- A process reads back exactly what it wrote. -/
theorem Node.restrict_override_self (sel : Sel L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} →
    (n : Node Γ κs) → (s : Node.State n) → (l : Node.LocState sel n) →
    Node.restrict sel n (Node.override sel n s l) = l
  | _, _, .broadcast_closed .., s, l => restrict_override_self_pw sel _ s l
  | _, _, .demux .., s, l => restrict_override_self_pw sel _ s l
  | _, _, .kfold_early_stop .., s, l => restrict_override_self_pw sel _ s l
  | _, _, .fold .., s, l => restrict_override_self_pw sel _ s l
  | _, _, .fold_monotone .., s, l => restrict_override_self_pw sel _ s l
  | _, _, .batch .., s, l => restrict_override_self_pw sel _ s l
  | _, _, .batch_ordered .., s, l => restrict_override_self_pw sel _ s l
  | _, _, .sample_every .., s, l => restrict_override_self_pw sel _ s l
  | _, _, .timeout_snapshot .., s, l => restrict_override_self_pw sel _ s l
  | _, _, .source_interval_batch .., s, l => restrict_override_self_pw sel _ s l
  | _, _, .zipTick .., s, l => restrict_override_self_pw sel _ s l
  | _, _, .defer_tick .., s, l => restrict_override_self_pw sel _ s l
  | _, _, .reduce_watermark .., s, l => restrict_override_self_pw sel _ s l
  | _, _, .tick_scan .., s, l => restrict_override_self_pw sel _ s l
  | _, _, .call N _, s, l => Dag.restrict_override_self sel N s l
  | _, _, .fixN _ body, s, l => by
    show (Dag.restrict sel body (Dag.override sel body s.1 l.1), envRestrict sel (envOverride sel s.2 l.2)) = l
    rw [Dag.restrict_override_self sel body s.1 l.1, envRestrict_override_self]
    rfl
  | _, _, .map .., _, _ => rfl
  | _, _, .filterMap .., _, _ => rfl
  | _, _, .values .., _, _ => rfl
  | _, _, .into_keyed .., _, _ => rfl
  | _, _, .kassume_ordering .., _, _ => rfl
  | _, _, .kget_max_key .., _, _ => rfl
  | _, _, .weaken_retries .., _, _ => rfl
  | _, _, .union .., _, _ => rfl
  | _, _, .assume_ordering .., _, _ => rfl
  | _, _, .snapshot .., _, _ => rfl
  | _, _, .mapTick .., _, _ => rfl
  | _, _, .allTicks .., _, _ => rfl
  | _, _, .flattenOrdered .., _, _ => rfl
  | _, _, .flattenUnordered .., _, _ => rfl
  | _, _, .tinto_keyed .., _, _ => rfl
  | _, _, .tassume_ordering .., _, _ => rfl

theorem Dag.restrict_override_self (sel : Sel L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} →
    (d : Dag Γ outs) → (s : Dag.State d) → (l : Dag.LocState sel d) →
    Dag.restrict sel d (Dag.override sel d s l) = l
  | _, _, .ret _, _, _ => rfl
  | _, _, .letN n rest, s, l => by
    show (Node.restrict sel n (Node.override sel n s.1 l.1), Dag.restrict sel rest (Dag.override sel rest s.2 l.2)) = l
    rw [Node.restrict_override_self sel n s.1 l.1, Dag.restrict_override_self sel rest s.2 l.2]
    rfl

end

section Other

variable (sel sel' : Sel L mem) (hd : ∀ ℓ i, sel' ℓ i = true → sel ℓ i = false)
include hd

set_option linter.unusedSectionVars false

mutual

/-- Another process's view is untouched by a write-back. -/
theorem Node.restrict_override_other : {Γ : Ctx L mem} → {κs : List (Kind L mem)} →
    (n : Node Γ κs) → (s : Node.State n) → (l : Node.LocState sel n) →
    Node.restrict sel' n (Node.override sel n s l) = Node.restrict sel' n s
  | _, _, .broadcast_closed .., s, l => restrict_override_other_pw sel sel' hd _ s l
  | _, _, .demux .., s, l => restrict_override_other_pw sel sel' hd _ s l
  | _, _, .kfold_early_stop .., s, l => restrict_override_other_pw sel sel' hd _ s l
  | _, _, .fold .., s, l => restrict_override_other_pw sel sel' hd _ s l
  | _, _, .fold_monotone .., s, l => restrict_override_other_pw sel sel' hd _ s l
  | _, _, .batch .., s, l => restrict_override_other_pw sel sel' hd _ s l
  | _, _, .batch_ordered .., s, l => restrict_override_other_pw sel sel' hd _ s l
  | _, _, .sample_every .., s, l => restrict_override_other_pw sel sel' hd _ s l
  | _, _, .timeout_snapshot .., s, l => restrict_override_other_pw sel sel' hd _ s l
  | _, _, .source_interval_batch .., s, l => restrict_override_other_pw sel sel' hd _ s l
  | _, _, .zipTick .., s, l => restrict_override_other_pw sel sel' hd _ s l
  | _, _, .defer_tick .., s, l => restrict_override_other_pw sel sel' hd _ s l
  | _, _, .reduce_watermark .., s, l => restrict_override_other_pw sel sel' hd _ s l
  | _, _, .tick_scan .., s, l => restrict_override_other_pw sel sel' hd _ s l
  | _, _, .call N _, s, l => Dag.restrict_override_other N s l
  | _, _, .fixN _ body, s, l => by
    show (Dag.restrict sel' body (Dag.override sel body s.1 l.1), envRestrict sel' (envOverride sel s.2 l.2))
      = (Dag.restrict sel' body s.1, envRestrict sel' s.2)
    rw [Dag.restrict_override_other body s.1 l.1, envRestrict_override_other sel sel' hd]
  | _, _, .map .., _, _ => rfl
  | _, _, .filterMap .., _, _ => rfl
  | _, _, .values .., _, _ => rfl
  | _, _, .into_keyed .., _, _ => rfl
  | _, _, .kassume_ordering .., _, _ => rfl
  | _, _, .kget_max_key .., _, _ => rfl
  | _, _, .weaken_retries .., _, _ => rfl
  | _, _, .union .., _, _ => rfl
  | _, _, .assume_ordering .., _, _ => rfl
  | _, _, .snapshot .., _, _ => rfl
  | _, _, .mapTick .., _, _ => rfl
  | _, _, .allTicks .., _, _ => rfl
  | _, _, .flattenOrdered .., _, _ => rfl
  | _, _, .flattenUnordered .., _, _ => rfl
  | _, _, .tinto_keyed .., _, _ => rfl
  | _, _, .tassume_ordering .., _, _ => rfl

theorem Dag.restrict_override_other : {Γ : Ctx L mem} → {outs : List (Kind L mem)} →
    (d : Dag Γ outs) → (s : Dag.State d) → (l : Dag.LocState sel d) →
    Dag.restrict sel' d (Dag.override sel d s l) = Dag.restrict sel' d s
  | _, _, .ret _, _, _ => rfl
  | _, _, .letN n rest, s, l => by
    show (Node.restrict sel' n (Node.override sel n s.1 l.1), Dag.restrict sel' rest (Dag.override sel rest s.2 l.2))
      = (Node.restrict sel' n s.1, Dag.restrict sel' rest s.2)
    rw [Node.restrict_override_other n s.1 l.1, Dag.restrict_override_other rest s.2 l.2]

end

mutual

/-- Another process sees the active process's sends land in its own queues. -/
theorem Node.restrict_enqueue : {Γ : Ctx L mem} → {κs : List (Kind L mem)} →
    (n : Node Γ κs) → (snd : Node.Sends sel n) → (s : Node.State n) →
    Node.restrict sel' n (Node.enqueue sel n snd s) = Node.enqueueLoc sel' sel n snd (Node.restrict sel' n s)
  | _, _, .broadcast_closed (p := p) .., snd, s => by
    funext i j
    show (if sel p i.1 = true then s i.1 j else if h : _ then _ else s i.1 j) = _
    rw [hd p i.1 i.2]
    rfl
  | _, _, .demux (p := p) .., snd, s => by
    funext i j
    show (if sel p i.1 = true then s i.1 j else if h : _ then _ else s i.1 j) = _
    rw [hd p i.1 i.2]
    rfl
  | _, _, .call N _, snd, s => Dag.restrict_enqueue N snd s
  | _, _, .fixN _ body, snd, s => by
    show (Dag.restrict sel' body (Dag.enqueue sel body snd s.1), envRestrict sel' s.2)
      = (Dag.enqueueLoc sel' sel body snd (Dag.restrict sel' body s.1), envRestrict sel' s.2)
    rw [Dag.restrict_enqueue body snd s.1]
  | _, _, .kfold_early_stop .., _, _ => rfl
  | _, _, .fold .., _, _ => rfl
  | _, _, .fold_monotone .., _, _ => rfl
  | _, _, .batch .., _, _ => rfl
  | _, _, .batch_ordered .., _, _ => rfl
  | _, _, .sample_every .., _, _ => rfl
  | _, _, .timeout_snapshot .., _, _ => rfl
  | _, _, .source_interval_batch .., _, _ => rfl
  | _, _, .zipTick .., _, _ => rfl
  | _, _, .defer_tick .., _, _ => rfl
  | _, _, .reduce_watermark .., _, _ => rfl
  | _, _, .tick_scan .., _, _ => rfl
  | _, _, .map .., _, _ => rfl
  | _, _, .filterMap .., _, _ => rfl
  | _, _, .values .., _, _ => rfl
  | _, _, .into_keyed .., _, _ => rfl
  | _, _, .kassume_ordering .., _, _ => rfl
  | _, _, .kget_max_key .., _, _ => rfl
  | _, _, .weaken_retries .., _, _ => rfl
  | _, _, .union .., _, _ => rfl
  | _, _, .assume_ordering .., _, _ => rfl
  | _, _, .snapshot .., _, _ => rfl
  | _, _, .mapTick .., _, _ => rfl
  | _, _, .allTicks .., _, _ => rfl
  | _, _, .flattenOrdered .., _, _ => rfl
  | _, _, .flattenUnordered .., _, _ => rfl
  | _, _, .tinto_keyed .., _, _ => rfl
  | _, _, .tassume_ordering .., _, _ => rfl

theorem Dag.restrict_enqueue : {Γ : Ctx L mem} → {outs : List (Kind L mem)} →
    (d : Dag Γ outs) → (snd : Dag.Sends sel d) → (s : Dag.State d) →
    Dag.restrict sel' d (Dag.enqueue sel d snd s) = Dag.enqueueLoc sel' sel d snd (Dag.restrict sel' d s)
  | _, _, .ret _, _, _ => rfl
  | _, _, .letN n rest, snd, s => by
    show (Node.restrict sel' n (Node.enqueue sel n snd.1 s.1), Dag.restrict sel' rest (Dag.enqueue sel rest snd.2 s.2))
      = (Node.enqueueLoc sel' sel n snd.1 (Node.restrict sel' n s.1), Dag.enqueueLoc sel' sel rest snd.2 (Dag.restrict sel' rest s.2))
    rw [Node.restrict_enqueue n snd.1 s.1, Dag.restrict_enqueue rest snd.2 s.2]

end

end Other

/-! ## Processes are members -/

/-- A cluster member — at runtime, one process. -/
abbrev Member (L : Type) (mem : L → Nat) := (ℓ : L) × Fin (mem ℓ)

variable [DecidableEq L]

/-- The process hosting exactly the member `m`. -/
def Sel.single (m : Member L mem) : Sel L mem := fun ℓ i => decide ((⟨ℓ, i⟩ : Member L mem) = m)

theorem Sel.single_self (m : Member L mem) : Sel.single m m.1 m.2 = true := by
  simp [Sel.single]

theorem Sel.single_disjoint {m m' : Member L mem} (h : m' ≠ m) :
    ∀ ℓ i, Sel.single m' ℓ i = true → Sel.single m ℓ i = false := fun ℓ i hi => by
  simp only [Sel.single, decide_eq_true_eq] at hi
  simp [Sel.single, hi, h]

/-! ### Masking an increment environment to a process -/

namespace Kind

/-- The increment at the hosted members, empty elsewhere (singletons: unchanged — a
singleton's increment is its value and is read only at ticks). -/
def maskDelta (sel : Sel L mem) : (κ : Kind L mem) → κ.delta → κ.delta
  | .stream ℓ .., Δ => fun i => if sel ℓ i then Δ i else []
  | .keyed ℓ .., Δ => fun i => if sel ℓ i then Δ i else []
  | .sing .., Δ => Δ
  | .ksing .., Δ => Δ
  | .atomic ℓ .., Δ => fun i => if sel ℓ i then Δ i else []
  | .atomicStream ℓ .., Δ => fun i => if sel ℓ i then Δ i else []
  | .atomicKeyed ℓ .., Δ => fun i => if sel ℓ i then Δ i else []
  | .atomicKSing ℓ .., Δ => fun i => if sel ℓ i then Δ i else []

theorem maskDelta_idle (sel : Sel L mem) : (κ : Kind L mem) → (Δ : κ.delta) → idleDelta sel κ (maskDelta sel κ Δ)
  | .stream .., _ => fun i hi => by simp [maskDelta, hi]
  | .keyed .., _ => fun i hi => by simp [maskDelta, hi]
  | .sing .., _ => trivial
  | .ksing .., _ => trivial
  | .atomic .., _ => fun i hi => by simp [maskDelta, hi]
  | .atomicStream .., _ => fun i hi => by simp [maskDelta, hi]
  | .atomicKeyed .., _ => fun i hi => by simp [maskDelta, hi]
  | .atomicKSing .., _ => fun i hi => by simp [maskDelta, hi]

theorem restrictDelta_mask (sel : Sel L mem) : (κ : Kind L mem) → (Δ : κ.delta) →
    restrictDelta sel κ (maskDelta sel κ Δ) = restrictDelta sel κ Δ
  | .stream .., _ => funext fun i => by simp [maskDelta, restrictDelta, i.2]
  | .keyed .., _ => funext fun i => by simp [maskDelta, restrictDelta, i.2]
  | .sing .., _ => rfl
  | .ksing .., _ => rfl
  | .atomic .., _ => funext fun i => by simp [maskDelta, restrictDelta, i.2]
  | .atomicStream .., _ => funext fun i => by simp [maskDelta, restrictDelta, i.2]
  | .atomicKeyed .., _ => funext fun i => by simp [maskDelta, restrictDelta, i.2]
  | .atomicKSing .., _ => funext fun i => by simp [maskDelta, restrictDelta, i.2]

end Kind

/-- An increment environment masked to a process. -/
def envMask (sel : Sel L mem) : {Γ : Ctx L mem} → HList (Kind.delta (mem := mem)) Γ → HList (Kind.delta (mem := mem)) Γ
  | [], _ => HList.nil
  | _ :: _, Δ => (Kind.maskDelta sel _ Δ.1, envMask sel Δ.2)

theorem envMask_idle (sel : Sel L mem) : {Γ : Ctx L mem} → (Δ : HList (Kind.delta (mem := mem)) Γ) →
    IdleEnv sel (envMask sel Δ)
  | [], _ => trivial
  | _ :: _, Δ => ⟨Kind.maskDelta_idle sel _ Δ.1, envMask_idle sel Δ.2⟩

theorem envRestrict_mask (sel : Sel L mem) : {Γ : Ctx L mem} → (Δ : HList (Kind.delta (mem := mem)) Γ) →
    envRestrict sel (envMask sel Δ) = envRestrict sel Δ
  | [], _ => rfl
  | _ :: _, Δ => by
    show (Kind.restrictDelta sel _ (Kind.maskDelta sel _ Δ.1), envRestrict sel (envMask sel Δ.2))
      = (Kind.restrictDelta sel _ Δ.1, envRestrict sel Δ.2)
    rw [Kind.restrictDelta_mask, envRestrict_mask sel Δ.2]

/-! ### Interleavings -/

/-- An interleaving: the member (process) that runs at each clocked step, whether
it ticks (else it stutters), and the input increments. Everything else is derived —
the running member is the active one (`pacing`), its inputs are the ones consumed
(`env`), and the delivery cursors are the graph's own (`Dag.cursorsAt`). -/
structure Interleaving (L : Type) (mem : L → Nat) (Γ : Ctx L mem) where
  who : Nat → Member L mem
  tick : Nat → Bool
  inp : Nat → HList (Kind.delta (mem := mem)) Γ

namespace Interleaving

variable {Γ : Ctx L mem} (I : Interleaving L mem Γ)

/-- The process running at step `k`. -/
def sel (k : Nat) : Sel L mem := Sel.single (I.who k)

/-- The pacing: a member ticks at step `k` iff it is the one running at `k`. -/
def pacing : Pacing L mem := fun ℓ i k =>
  if Sel.single (I.who k) ℓ i then (if I.tick k then .tick else .stutter) else .idle

/-- The input increments the lock-step machine consumes at step `k`: the active
process's; the other members' are ignored. -/
def env (k : Nat) : HList (Kind.delta (mem := mem)) Γ := envMask (I.sel k) (I.inp k)

/-- The step context of step `k`. -/
def ctx (k : Nat) : StepCtx L mem :=
  ⟨k, fun ℓ i => I.pacing.ticks ℓ i k, fun ℓ i => I.pacing.active ℓ i k⟩

theorem ctx_act (k : Nat) : (I.ctx k).act = I.sel k := by
  funext ℓ i
  show (if Sel.single (I.who k) ℓ i then (if I.tick k then Pace.tick else Pace.stutter) else Pace.idle).active
    = Sel.single (I.who k) ℓ i
  split <;> (try split) <;> simp_all

theorem ctx_tick (k : Nat) : (I.ctx k).tick = fun ℓ i => I.sel k ℓ i && I.tick k := by
  funext ℓ i
  show (if Sel.single (I.who k) ℓ i then (if I.tick k then Pace.tick else Pace.stutter) else Pace.idle).ticks
    = (Sel.single (I.who k) ℓ i && I.tick k)
  split <;> (try split) <;> simp_all

variable {outs : List (Kind L mem)} (d : Dag Γ outs)

/-- The lock-step cells after step `k`. -/
def state (k : Nat) : Dag.State d := (Dag.run d I.pacing I.env k).1

/-- The cells step `k` starts from. -/
def before : Nat → Dag.State d
  | 0 => Dag.init d
  | k + 1 => I.state d k

theorem state_eq_step (k : Nat) : I.state d k = (Dag.step (I.ctx k) d (I.env k) (I.before d k)).1 := by
  cases k <;> rfl

/-- The step's own increments. -/
theorem run_snd (k : Nat) : (Dag.run d I.pacing I.env k).2 = (Dag.step (I.ctx k) d (I.env k) (I.before d k)).2 := by
  cases k <;> rfl

/-- Every state the run passes through has its multi-input tick cells drained. -/
theorem drained_before : ∀ k, Dag.Drained d (I.before d k)
  | 0 => Dag.init_drained d
  | k + 1 => by
    show Dag.Drained d (I.state d k)
    rw [I.state_eq_step d k]
    exact Dag.step_drained _ d _ _

/-- **Step `k` is the active process's step** — for every interleaving. -/
theorem step_wired (k : Nat) :
    DagWired (I.ctx k) (I.sel k) (I.tick k) d (Dag.cursorsAt (I.sel k) d k) (I.env k) (I.before d k) :=
  Dag.step_wired (I.ctx k) (I.sel k) (I.tick k) (I.ctx_act k) (I.ctx_tick k) d _ (I.env k) (I.before d k)
    (envMask_idle (I.sel k) (I.inp k)) (Dag.cursorsAt_oracleAt (I.sel k) d k) (I.drained_before d k)

/-! ### The processes -/

/-- Process `m`'s cells at the start of step `k`. -/
def view (k : Nat) (m : Member L mem) : Dag.LocState (Sel.single m) d := Dag.restrict (Sel.single m) d (I.before d k)

/-- The active process's step at `k`: its new cells, its output increments, its
sends — `locStep` on its own cells (ticking iff `I.tick k`), with its own inputs, at
the graph's cursors. -/
def active (k : Nat) :=
  Dag.locStep (I.sel k) (I.tick k) d (Dag.cursorsAt (I.sel k) d k) (envRestrict (I.sel k) (I.inp k))
    (I.view d k (I.who k))

theorem active_eq (k : Nat) :
    I.active d k = Dag.locStep (I.sel k) (I.tick k) d (Dag.cursorsAt (I.sel k) d k) (envRestrict (I.sel k) (I.env k))
      (Dag.restrict (I.sel k) d (I.before d k)) := by
  show Dag.locStep _ _ d _ (envRestrict (I.sel k) (I.inp k)) _
    = Dag.locStep _ _ d _ (envRestrict (I.sel k) (envMask (I.sel k) (I.inp k))) _
  rw [envRestrict_mask]
  rfl

/-- **The active process steps on its own cells.** -/
theorem view_active (k : Nat) : I.view d (k + 1) (I.who k) = (I.active d k).1 := by
  show Dag.restrict (I.sel k) d (I.state d k) = _
  rw [I.state_eq_step, (I.step_wired d k).1, Dag.restrict_override_self, active_eq]

/-- **Every other process only receives the active process's sends.** -/
theorem view_idle (k : Nat) (m : Member L mem) (hm : m ≠ I.who k) :
    I.view d (k + 1) m = Dag.enqueueLoc (Sel.single m) (I.sel k) d (I.active d k).2.2 (I.view d k m) := by
  show Dag.restrict (Sel.single m) d (I.state d k) = _
  rw [I.state_eq_step, (I.step_wired d k).1,
    Dag.restrict_override_other (I.sel k) (Sel.single m) (Sel.single_disjoint hm),
    Dag.restrict_enqueue (I.sel k) (Sel.single m) (Sel.single_disjoint hm), active_eq]
  rfl

/-- **The active process's outputs are the machine's increments at its members.**
With the consumed inputs the increments of `SchedSem` wires `env`, by `Dag.run_spec`. -/
theorem outputs (env : Env Γ) (he : HTMono env) (hinp : I.env = fun k => envIncAt env k) (k : Nat) :
    (I.active d k).2.1 = envRestrict (I.sel k) (envIncAt (Dag.denote I.pacing d env) k) := by
  rw [active_eq, ← (I.step_wired d k).2.1, ← I.run_snd]
  have h := Dag.run_spec I.pacing d env he k
  rw [hinp, h]

end Interleaving

end Project

end Hydro
