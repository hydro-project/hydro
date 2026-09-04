import Hydro.Project.Model.WiredMain
import Hydro.Project.Model.StepMain
import Hydro.Project.Model.EnvOfInc

/-!
# Hydro · Project — wired machine: the run, per process

**The correspondence.** A process is a cluster member (`Member`). An
interleaving of a graph `d` says which member runs at
each clocked step (`who`), whether its tick bodies run or it only
stutters — its asynchronous dataflow progressing: deliveries land, knot
feedback propagates (`tick`) — which input increments arrive (`inp`),
and what the network delivers (`sched`: per network edge, per receiver,
per sender, the cursor at each step — how many of the sender's messages
the receiver has read by then; D77); the running member is exactly the
active one (`pacing`), the inputs at other members are ignored (`env` —
they cannot be consumed by a sleeping process; an interleaving lists
them at the step their process runs), and the running process reads its
cursors (`Dag.cursorsAt`: the schedule cut to its members, monotone). A
DFIR tick is a tick then stutters at one member (D76/D93: `Dag.iterate`, the
settling after the tick).
**Unconditionally**, every step of the lock-step run **is** the active
process's `locStep` on its own view, written back, with its sends
appended to every other process's view (`view_active`/`view_idle`); and
its output increments are the machine denotation's increments at its
members (`outputs`, via `Dag.run_spec`).

So the processes, each running `Dag.locStep` on its own cells and
exchanging only `Sends`, compute exactly the `SchedSem` run of the graph
under the pacing `I.pacing` and the schedule `I.sched` — any wired
execution is a `SchedSem` run, and every `SchedSem` headline applies to
it. No settling, no quiet, no idle-process condition: a member that is
not active is inert (`Project/Step.lean`), so a sleeping process's cells
are exactly the lock-step machine's.
-/

namespace Hydro

namespace Project

variable {L : Type} {mem : L → Nat} [DecidableEq L]

/-! ## Views: the global state cut to a process -/

theorem restrict_override_self_pw (m : Member L mem) (ℓ : L) {X : Type} (s : Fin (mem ℓ) → X)
    (l : Hosted m ℓ → X) :
    (fun (i : Hosted m ℓ) => (fun i => if h : (⟨ℓ, i⟩ : Member L mem) = m then l ⟨i, h⟩ else s i) i.1) = l := by
  funext i
  simp only [i.2, dite_true]

theorem restrict_override_other_pw (m m' : Member L mem) (hd : m' ≠ m)
    (ℓ : L) {X : Type} (s : Fin (mem ℓ) → X) (l : Hosted m ℓ → X) :
    (fun (i : Hosted m' ℓ) => (fun i => if h : (⟨ℓ, i⟩ : Member L mem) = m then l ⟨i, h⟩ else s i) i.1) = fun i => s i.1 := by
  funext i
  simp only [Hosted.ne_of_ne hd i, dite_false]

theorem envRestrict_override_self (m : Member L mem) : {Γ : Ctx L mem} → (Δ : HList (Kind.delta (mem := mem)) Γ) →
    (Δ' : HList (Kind.locDelta m) Γ) → envRestrict m (envOverride m Δ Δ') = Δ'
  | [], _, _ => rfl
  | κ :: _, Δ, Δ' => by
    show (Kind.restrictDelta m κ (Kind.overrideDelta m κ Δ.1 Δ'.1), envRestrict m (envOverride m Δ.2 Δ'.2))
      = (Δ'.1, Δ'.2)
    rw [envRestrict_override_self m Δ.2 Δ'.2]
    cases κ <;> exact congrArg (·, Δ'.2) (funext fun i => by simp only [Kind.restrictDelta, Kind.overrideDelta, i.2, dite_true])

theorem envRestrict_override_other (m m' : Member L mem) (hd : m' ≠ m) :
    {Γ : Ctx L mem} → (Δ : HList (Kind.delta (mem := mem)) Γ) → (Δ' : HList (Kind.locDelta m) Γ) →
    envRestrict m' (envOverride m Δ Δ') = envRestrict m' Δ
  | [], _, _ => rfl
  | κ :: _, Δ, Δ' => by
    show (Kind.restrictDelta m' κ (Kind.overrideDelta m κ Δ.1 Δ'.1), envRestrict m' (envOverride m Δ.2 Δ'.2))
      = (Kind.restrictDelta m' κ Δ.1, envRestrict m' Δ.2)
    rw [envRestrict_override_other m m' hd Δ.2 Δ'.2]
    cases κ <;> exact congrArg (·, envRestrict m' Δ.2)
      (funext fun i => by simp only [Kind.restrictDelta, Kind.overrideDelta, Hosted.ne_of_ne hd i, dite_false])

mutual

/-- A process reads back exactly what it wrote. -/
theorem Node.restrict_override_self (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} →
    (n : Node Γ κs) → (s : Node.State n) → (l : Node.LocState m n) →
    Node.restrict m n (Node.override m n s l) = l
  | _, _, .broadcast_closed .., s, l => restrict_override_self_pw m _ s l
  | _, _, .demux .., s, l => restrict_override_self_pw m _ s l
  | _, _, .kfold_early_stop .., s, l => restrict_override_self_pw m _ s l
  | _, _, .fold .., s, l => restrict_override_self_pw m _ s l
  | _, _, .fold_monotone .., s, l => restrict_override_self_pw m _ s l
  | _, _, .batch .., s, l => restrict_override_self_pw m _ s l
  | _, _, .source_interval .., s, l => restrict_override_self_pw m _ s l
  | _, _, .kfold .., s, l => restrict_override_self_pw m _ s l
  | _, _, .tfold .., s, l => restrict_override_self_pw m _ s l
  | _, _, .latest .., s, l => restrict_override_self_pw m _ s l
  | _, _, .latest_monotone .., s, l => restrict_override_self_pw m _ s l
  | _, _, .time .., s, l => restrict_override_self_pw m _ s l
  | _, _, .snapshot_bounded .., s, l => restrict_override_self_pw m _ s l
  | _, _, .zipTick .., s, l => restrict_override_self_pw m _ s l
  | _, _, .defer_tick .., s, l => restrict_override_self_pw m _ s l
  | _, _, .reduce_watermark .., s, l => restrict_override_self_pw m _ s l
  | _, _, .tick_scan .., s, l => restrict_override_self_pw m _ s l
  | _, _, .call N _, s, l => Dag.restrict_override_self m N s l
  | _, _, .fixN _ body, s, l => by
    show (Dag.restrict m body (Dag.override m body s.1 l.1), envRestrict m (envOverride m s.2 l.2)) = l
    rw [Dag.restrict_override_self m body s.1 l.1, envRestrict_override_self]
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
  | _, _, .firstTick .., _, _ => rfl
  | _, _, .allTicks .., _, _ => rfl
  | _, _, .flattenOrdered .., _, _ => rfl
  | _, _, .flattenUnordered .., _, _ => rfl
  | _, _, .tinto_keyed .., _, _ => rfl
  | _, _, .tassume_ordering .., _, _ => rfl
  | _, _, .kentries .., _, _ => rfl
  | _, _, .kmap .., _, _ => rfl
  | _, _, .kallTicks .., _, _ => rfl
  | _, _, .ksnapshot .., _, _ => rfl

theorem Dag.restrict_override_self (m : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} →
    (d : Dag Γ outs) → (s : Dag.State d) → (l : Dag.LocState m d) →
    Dag.restrict m d (Dag.override m d s l) = l
  | _, _, .ret _, _, _ => rfl
  | _, _, .letN n rest, s, l => by
    show (Node.restrict m n (Node.override m n s.1 l.1), Dag.restrict m rest (Dag.override m rest s.2 l.2)) = l
    rw [Node.restrict_override_self m n s.1 l.1, Dag.restrict_override_self m rest s.2 l.2]
    rfl

end

section Other

variable (m m' : Member L mem) (hd : m' ≠ m)
include hd

set_option linter.unusedSectionVars false

mutual

/-- Another process's view is untouched by a write-back. -/
theorem Node.restrict_override_other : {Γ : Ctx L mem} → {κs : List (Kind L mem)} →
    (n : Node Γ κs) → (s : Node.State n) → (l : Node.LocState m n) →
    Node.restrict m' n (Node.override m n s l) = Node.restrict m' n s
  | _, _, .broadcast_closed .., s, l => restrict_override_other_pw m m' hd _ s l
  | _, _, .demux .., s, l => restrict_override_other_pw m m' hd _ s l
  | _, _, .kfold_early_stop .., s, l => restrict_override_other_pw m m' hd _ s l
  | _, _, .fold .., s, l => restrict_override_other_pw m m' hd _ s l
  | _, _, .fold_monotone .., s, l => restrict_override_other_pw m m' hd _ s l
  | _, _, .batch .., s, l => restrict_override_other_pw m m' hd _ s l
  | _, _, .source_interval .., s, l => restrict_override_other_pw m m' hd _ s l
  | _, _, .kfold .., s, l => restrict_override_other_pw m m' hd _ s l
  | _, _, .tfold .., s, l => restrict_override_other_pw m m' hd _ s l
  | _, _, .latest .., s, l => restrict_override_other_pw m m' hd _ s l
  | _, _, .latest_monotone .., s, l => restrict_override_other_pw m m' hd _ s l
  | _, _, .time .., s, l => restrict_override_other_pw m m' hd _ s l
  | _, _, .snapshot_bounded .., s, l => restrict_override_other_pw m m' hd _ s l
  | _, _, .zipTick .., s, l => restrict_override_other_pw m m' hd _ s l
  | _, _, .defer_tick .., s, l => restrict_override_other_pw m m' hd _ s l
  | _, _, .reduce_watermark .., s, l => restrict_override_other_pw m m' hd _ s l
  | _, _, .tick_scan .., s, l => restrict_override_other_pw m m' hd _ s l
  | _, _, .call N _, s, l => Dag.restrict_override_other N s l
  | _, _, .fixN _ body, s, l => by
    show (Dag.restrict m' body (Dag.override m body s.1 l.1), envRestrict m' (envOverride m s.2 l.2))
      = (Dag.restrict m' body s.1, envRestrict m' s.2)
    rw [Dag.restrict_override_other body s.1 l.1, envRestrict_override_other m m' hd]
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
  | _, _, .firstTick .., _, _ => rfl
  | _, _, .allTicks .., _, _ => rfl
  | _, _, .flattenOrdered .., _, _ => rfl
  | _, _, .flattenUnordered .., _, _ => rfl
  | _, _, .tinto_keyed .., _, _ => rfl
  | _, _, .tassume_ordering .., _, _ => rfl
  | _, _, .kentries .., _, _ => rfl
  | _, _, .kmap .., _, _ => rfl
  | _, _, .kallTicks .., _, _ => rfl
  | _, _, .ksnapshot .., _, _ => rfl

theorem Dag.restrict_override_other : {Γ : Ctx L mem} → {outs : List (Kind L mem)} →
    (d : Dag Γ outs) → (s : Dag.State d) → (l : Dag.LocState m d) →
    Dag.restrict m' d (Dag.override m d s l) = Dag.restrict m' d s
  | _, _, .ret _, _, _ => rfl
  | _, _, .letN n rest, s, l => by
    show (Node.restrict m' n (Node.override m n s.1 l.1), Dag.restrict m' rest (Dag.override m rest s.2 l.2))
      = (Node.restrict m' n s.1, Dag.restrict m' rest s.2)
    rw [Node.restrict_override_other n s.1 l.1, Dag.restrict_override_other rest s.2 l.2]

end

mutual

/-- Another process sees the active process's sends land in its own queues. -/
theorem Node.restrict_enqueue : {Γ : Ctx L mem} → {κs : List (Kind L mem)} →
    (n : Node Γ κs) → (snd : Node.Sends m n) → (s : Node.State n) →
    Node.restrict m' n (Node.enqueue m n snd s) = Node.enqueueLoc m' m n snd (Node.restrict m' n s)
  | _, _, .broadcast_closed (p := p) .., snd, s => by
    funext i j
    show (if (⟨p, i.1⟩ : Member L mem) = m then s i.1 j else if h : _ then _ else s i.1 j) = _
    rw [if_neg (Hosted.ne_of_ne hd i)]
    rfl
  | _, _, .demux (p := p) .., snd, s => by
    funext i j
    show (if (⟨p, i.1⟩ : Member L mem) = m then s i.1 j else if h : _ then _ else s i.1 j) = _
    rw [if_neg (Hosted.ne_of_ne hd i)]
    rfl
  | _, _, .call N _, snd, s => Dag.restrict_enqueue N snd s
  | _, _, .fixN _ body, snd, s => by
    show (Dag.restrict m' body (Dag.enqueue m body snd s.1), envRestrict m' s.2)
      = (Dag.enqueueLoc m' m body snd (Dag.restrict m' body s.1), envRestrict m' s.2)
    rw [Dag.restrict_enqueue body snd s.1]
  | _, _, .kfold_early_stop .., _, _ => rfl
  | _, _, .fold .., _, _ => rfl
  | _, _, .fold_monotone .., _, _ => rfl
  | _, _, .batch .., _, _ => rfl
  | _, _, .source_interval .., _, _ => rfl
  | _, _, .kfold .., _, _ => rfl
  | _, _, .tfold .., _, _ => rfl
  | _, _, .latest .., _, _ => rfl
  | _, _, .latest_monotone .., _, _ => rfl
  | _, _, .time .., _, _ => rfl
  | _, _, .snapshot_bounded .., _, _ => rfl
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
  | _, _, .firstTick .., _, _ => rfl
  | _, _, .allTicks .., _, _ => rfl
  | _, _, .flattenOrdered .., _, _ => rfl
  | _, _, .flattenUnordered .., _, _ => rfl
  | _, _, .tinto_keyed .., _, _ => rfl
  | _, _, .tassume_ordering .., _, _ => rfl
  | _, _, .kentries .., _, _ => rfl
  | _, _, .kmap .., _, _ => rfl
  | _, _, .kallTicks .., _, _ => rfl
  | _, _, .ksnapshot .., _, _ => rfl

theorem Dag.restrict_enqueue : {Γ : Ctx L mem} → {outs : List (Kind L mem)} →
    (d : Dag Γ outs) → (snd : Dag.Sends m d) → (s : Dag.State d) →
    Dag.restrict m' d (Dag.enqueue m d snd s) = Dag.enqueueLoc m' m d snd (Dag.restrict m' d s)
  | _, _, .ret _, _, _ => rfl
  | _, _, .letN n rest, snd, s => by
    show (Node.restrict m' n (Node.enqueue m n snd.1 s.1), Dag.restrict m' rest (Dag.enqueue m rest snd.2 s.2))
      = (Node.enqueueLoc m' m n snd.1 (Node.restrict m' n s.1), Dag.enqueueLoc m' m rest snd.2 (Dag.restrict m' rest s.2))
    rw [Node.restrict_enqueue n snd.1 s.1, Dag.restrict_enqueue rest snd.2 s.2]

end

end Other

/-! ### Masking an increment environment to a process -/

namespace Kind

/-- The increment at the member, empty elsewhere (singletons: unchanged — a
singleton's increment is its value and is read only at ticks). -/
def maskDelta (m : Member L mem) : (κ : Kind L mem) → κ.delta → κ.delta
  | .stream ℓ .., Δ => fun i => if (⟨ℓ, i⟩ : Member L mem) = m then Δ i else []
  | .keyed ℓ .., Δ => fun i => if (⟨ℓ, i⟩ : Member L mem) = m then Δ i else []
  | .sing .., Δ => Δ
  | .ksing .., Δ => Δ
  | .atomic ℓ .., Δ => fun i => if (⟨ℓ, i⟩ : Member L mem) = m then Δ i else none
  | .atomicStream ℓ .., Δ => fun i => if (⟨ℓ, i⟩ : Member L mem) = m then Δ i else none
  | .atomicKeyed ℓ .., Δ => fun i => if (⟨ℓ, i⟩ : Member L mem) = m then Δ i else none
  | .atomicKSing ℓ .., Δ => fun i => if (⟨ℓ, i⟩ : Member L mem) = m then Δ i else none

theorem maskDelta_idle (m : Member L mem) : (κ : Kind L mem) → (Δ : κ.delta) → idleDelta m κ (maskDelta m κ Δ)
  | .stream .., _ => fun i hi => by simp [maskDelta, hi]
  | .keyed .., _ => fun i hi => by simp [maskDelta, hi]
  | .sing .., _ => trivial
  | .ksing .., _ => trivial
  | .atomic .., _ => fun i hi => by simp [maskDelta, hi]
  | .atomicStream .., _ => fun i hi => by simp [maskDelta, hi]
  | .atomicKeyed .., _ => fun i hi => by simp [maskDelta, hi]
  | .atomicKSing .., _ => fun i hi => by simp [maskDelta, hi]

theorem restrictDelta_mask (m : Member L mem) : (κ : Kind L mem) → (Δ : κ.delta) →
    restrictDelta m κ (maskDelta m κ Δ) = restrictDelta m κ Δ
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
def envMask (m : Member L mem) : {Γ : Ctx L mem} → HList (Kind.delta (mem := mem)) Γ → HList (Kind.delta (mem := mem)) Γ
  | [], _ => HList.nil
  | _ :: _, Δ => (Kind.maskDelta m _ Δ.1, envMask m Δ.2)

theorem envMask_idle (m : Member L mem) : {Γ : Ctx L mem} → (Δ : HList (Kind.delta (mem := mem)) Γ) →
    IdleEnv m (envMask m Δ)
  | [], _ => trivial
  | _ :: _, Δ => ⟨Kind.maskDelta_idle m _ Δ.1, envMask_idle m Δ.2⟩

theorem envRestrict_mask (m : Member L mem) : {Γ : Ctx L mem} → (Δ : HList (Kind.delta (mem := mem)) Γ) →
    envRestrict m (envMask m Δ) = envRestrict m Δ
  | [], _ => rfl
  | _ :: _, Δ => by
    show (Kind.restrictDelta m _ (Kind.maskDelta m _ Δ.1), envRestrict m (envMask m Δ.2))
      = (Kind.restrictDelta m _ Δ.1, envRestrict m Δ.2)
    rw [Kind.restrictDelta_mask, envRestrict_mask m Δ.2]

/-! ### Interleavings -/

/-- The pacing of an interleaving: the running member is active — ticking or
stuttering per `tick k` — and every other member is idle. -/
def pacingOf (who : Nat → Member L mem) (tick : Nat → Bool) : Pacing L mem := fun ℓ i k =>
  if (⟨ℓ, i⟩ : Member L mem) = who k then (if tick k then .tick else .stutter) else .idle

/-- An interleaving of a graph: the member (process) that runs at each clocked
step, whether it ticks (else it stutters), the input increments — only the running
member's list-like increments are consumed (`envMask`); a register input (`Kind.sing`,
e.g. the acceptors' checkpoint) is a value every member has at every step and is read
only by a member when it ticks — and the schedule —
per network edge, per (receiver, sender), the cursor at each step (only the running
member's matter at a step, and a cursor below an earlier one reads as the earlier
one: `cumMax`, per-connection FIFO, delivered counts only grow); per timer, per
member, its verdicts. Everything else is derived — the running member is the active
one (`pacing`), its inputs are the ones consumed (`env`), its oracle is the schedule
cut to its members (`Dag.cursorsAt`). The graph is pacing-free (`M.dag`); the
interleaving's pacing enters only the denotation (`Dag.denote I.pacing d …`). -/
structure Interleaving {Γ : Ctx L mem} {outs : List (Kind L mem)} (d : Dag Γ outs) where
  who : Nat → Member L mem
  tick : Nat → Bool
  inp : Nat → HList (Kind.delta (mem := mem)) Γ
  sched : Dag.Sched d

namespace Interleaving

variable {Γ : Ctx L mem} {outs : List (Kind L mem)} {d : Dag Γ outs} (I : Interleaving d)

/-- The pacing: the running member is active — ticking or stuttering per `I.tick k` —
and every other member is idle. -/
abbrev pacing : Pacing L mem := pacingOf I.who I.tick

/-- The input increments the lock-step machine consumes at step `k`: the active
process's; the other members' are ignored. -/
def env (k : Nat) : HList (Kind.delta (mem := mem)) Γ := envMask (I.who k) (I.inp k)

/-- The step context of step `k`. -/
def ctx (k : Nat) : StepCtx L mem :=
  ⟨k, fun ℓ i => I.pacing.ticks ℓ i k, fun ℓ i => I.pacing.active ℓ i k⟩

theorem ctx_act (k : Nat) : (I.ctx k).act = (I.who k).hosts := by
  funext ℓ i
  show (if (⟨ℓ, i⟩ : Member L mem) = I.who k then (if I.tick k then Pace.tick else Pace.stutter) else Pace.idle).active
    = (I.who k).hosts ℓ i
  split <;> (try split) <;> simp_all [Member.hosts]

theorem ctx_tick (k : Nat) : (I.ctx k).tick = fun ℓ i => (I.who k).hosts ℓ i && I.tick k := by
  funext ℓ i
  show (if (⟨ℓ, i⟩ : Member L mem) = I.who k then (if I.tick k then Pace.tick else Pace.stutter) else Pace.idle).ticks
    = ((I.who k).hosts ℓ i && I.tick k)
  split <;> (try split) <;> simp_all


/-- The lock-step cells after step `k`. -/
def state (k : Nat) : Dag.State d := (Dag.run d I.sched I.pacing I.env k).1

/-- The cells step `k` starts from. -/
def before : Nat → Dag.State d
  | 0 => Dag.init d
  | k + 1 => I.state k

theorem state_eq_step (k : Nat) : I.state k = (Dag.step (I.ctx k) d I.sched (I.env k) (I.before k)).1 := by
  cases k <;> rfl

/-- The step's own increments. -/
theorem run_snd (k : Nat) :
    (Dag.run d I.sched I.pacing I.env k).2 = (Dag.step (I.ctx k) d I.sched (I.env k) (I.before k)).2 := by
  cases k <;> rfl

/-- Every state the run passes through has its multi-input tick cells drained. -/
theorem drained_before : ∀ k, Dag.Drained d I.sched (I.before k)
  | 0 => Dag.init_drained d I.sched
  | k + 1 => by
    show Dag.Drained d I.sched (I.state k)
    rw [I.state_eq_step k]
    exact Dag.step_drained _ d _ _ _

/-- Process `m`'s cells at the start of step `k`. -/
def view (k : Nat) (m : Member L mem) : Dag.LocState m d := Dag.restrict m d (I.before k)

/-- The active process's oracle at step `k`: the schedule's cursors at `k`, its timing
scripts at its own cells' tick counts. -/
def oracle (k : Nat) : Dag.Oracle (I.who k) d :=
  Dag.cursorsAt (I.who k) d I.sched k (I.view k (I.who k))

/-- **Step `k` is the active process's step** — for every interleaving. -/
theorem step_wired (k : Nat) :
    DagWired (I.ctx k) (I.who k) (I.tick k) d I.sched (I.oracle k) (I.env k) (I.before k) :=
  Dag.step_wired (I.ctx k) (I.who k) (I.tick k) (I.ctx_act k) (I.ctx_tick k) d I.sched _ (I.env k) (I.before k)
    (envMask_idle (I.who k) (I.inp k)) (Dag.cursorsAt_oracleAt (I.who k) d I.sched k (I.before k)) (I.drained_before k)

/-! ### The processes -/

/-- The active process's step at `k`: its new cells, its output increments, its
sends — `locStep` on its own cells (ticking iff `I.tick k`), with its own inputs,
reading its cursors from the interleaving's schedule. -/
def active (k : Nat) :=
  Dag.locStep (I.who k) (I.tick k) d (I.oracle k) (envRestrict (I.who k) (I.inp k))
    (I.view k (I.who k))

theorem active_eq (k : Nat) :
    I.active k = Dag.locStep (I.who k) (I.tick k) d (I.oracle k) (envRestrict (I.who k) (I.env k))
      (Dag.restrict (I.who k) d (I.before k)) := by
  show Dag.locStep _ _ d _ (envRestrict (I.who k) (I.inp k)) _
    = Dag.locStep _ _ d _ (envRestrict (I.who k) (envMask (I.who k) (I.inp k))) _
  rw [envRestrict_mask]
  rfl

/-- **The active process steps on its own cells** — and its own sends enter its own
queues through the network (`enqueueLoc m m`: the loopback connection, held by the
lock-step cells like every other in-flight message). -/
theorem view_active (k : Nat) :
    I.view (k + 1) (I.who k) = Dag.enqueueLoc (I.who k) (I.who k) d (I.active k).2.2 (I.active k).1 := by
  show Dag.restrict (I.who k) d (I.state k) = _
  rw [I.state_eq_step, (I.step_wired k).1, Dag.restrict_override_self, active_eq]

/-- **Every other process only receives the active process's sends.** -/
theorem view_idle (k : Nat) (m : Member L mem) (hm : m ≠ I.who k) :
    I.view (k + 1) m = Dag.enqueueLoc m (I.who k) d (I.active k).2.2 (I.view k m) := by
  show Dag.restrict m d (I.state k) = _
  rw [I.state_eq_step, (I.step_wired k).1,
    Dag.restrict_override_other (I.who k) m hm,
    Dag.restrict_enqueue (I.who k) m hm, active_eq]
  rfl

/-- A member that is idle at step `k` is not the running one. -/
theorem active_false_ne {ℓ : L} {i : Fin (mem ℓ)} {k : Nat} (h : I.pacing.active ℓ i k = false) :
    (⟨ℓ, i⟩ : Member L mem) ≠ I.who k := by
  intro heq
  simp [Interleaving.pacing, pacingOf, Pacing.active, heq] at h
  revert h; cases I.tick k <;> simp [Pace.active]

/-- The consumed inputs' atomic slots are empty at idle members: `envMask` puts
`none` there (D85 — every interleaving's inputs are single-slot paced). -/
theorem env_idleSlots : {Γ' : Ctx L mem} → (inp : Nat → HList (Kind.delta (mem := mem)) Γ') →
    HIdleSlots I.pacing (fun k => envMask (I.who k) (inp k))
  | [], _ => trivial
  | κ :: Γ', inp => by
    refine ⟨?_, env_idleSlots (fun k => (inp k).2)⟩
    have hmask : ∀ k, Kind.idleDelta (I.who k) κ (Kind.maskDelta (I.who k) κ (inp k).1) :=
      fun k => Kind.maskDelta_idle (I.who k) κ _
    show Kind.idleSlots I.pacing κ (fun k => Kind.maskDelta (I.who k) κ (inp k).1)
    generalize (fun k => (inp k).1 : (k : Nat) → κ.delta) = inc at hmask ⊢
    cases κ with
    | atomic ℓ _ => exact fun i k hk => hmask k i (I.active_false_ne hk)
    | atomicStream ℓ _ _ _ _ => exact fun i k hk => hmask k i (I.active_false_ne hk)
    | atomicKeyed ℓ _ _ _ _ _ _ => exact fun i k hk => hmask k i (I.active_false_ne hk)
    | atomicKSing ℓ _ _ _ _ => exact fun i k hk => hmask k i (I.active_false_ne hk)
    | stream => exact trivial
    | keyed => exact trivial
    | sing => exact trivial
    | ksing => exact trivial

/-- The active process's outputs are the machine's increments at its members, for
any `SchedSem` environment the consumed inputs are the increments of (`Dag.run_spec`). -/
theorem outputs_env (env : Env Γ) (he : HTMono env) (hp : HTPaced I.pacing env)
    (hinp : I.env = fun k => envIncAt env k) (k : Nat) :
    (I.active k).2.1 = envRestrict (I.who k) (envIncAt (Dag.denote I.pacing d I.sched env) k) := by
  rw [active_eq, ← (I.step_wired k).2.1, ← I.run_snd]
  have h := Dag.run_spec I.pacing d I.sched env he hp k
  rw [hinp, h]

/-- **The interleaving's inputs as `SchedSem` wires**: the environment whose increments
are the consumed inputs (`envIncAt_envOfInc`). -/
def wires : Env Γ := envOfInc I.env

theorem env_eq_wires : I.env = fun k => envIncAt I.wires k :=
  funext fun k => (envIncAt_envOfInc I.env k).symm

theorem wires_mono : HTMono I.wires := HTMono_envOfInc I.env

/-- Every interleaving's consumed atomic inputs are single slots (D85). -/
theorem wires_paced : HTPaced I.pacing I.wires :=
  HTPaced_envOfInc I.pacing I.env (I.env_idleSlots I.inp)

/-- **The active process's outputs are the machine's increments at its members** —
the machine run on the interleaving's own wires. No hypotheses: an interleaving's
inputs are monotone and single-slot by construction. -/
theorem outputs (k : Nat) :
    (I.active k).2.1 = envRestrict (I.who k) (envIncAt (Dag.denote I.pacing d I.sched I.wires) k) :=
  I.outputs_env I.wires I.wires_mono I.wires_paced I.env_eq_wires k

end Interleaving

end Project

end Hydro
