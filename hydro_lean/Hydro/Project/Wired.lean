import Hydro.Project.StepMain

/-!
# Hydro · Project — the wired machine (per process)

The lock-step machine (`Project/Step.lean`) steps every member of every
location at every global step. A **process** hosts some members
(`Sel`); it owns their cells (`LocState`), consumes their input
increments, reads its network **oracle** (how far each incoming channel
is delivered now), and produces the increments
of its hosted wires plus the **sends** of its hosted senders. Other
processes' cells it never touches; what crosses between processes is
exactly the channel append (`enqueue`).

- `Node.LocState`/`Dag.LocState`: the hosted members' cells (a network
  node's cells sit with the receiver: its per-sender queues);
- `Node.Oracle`: the delivery cursors read this step, per hosted receiver;
- `Node.Sends`: what the hosted senders put on the wire this step;
- `Node.locStep`/`Dag.locStep`: the process's step, every hosted member
  ticking — *the production interpreter of one process*;
- `restrict`/`override`/`enqueue`: the global state cut to a process, a
  process's cells written back, the active process's sends appended to
  everyone else's queues.

`Project/WiredMain.lean` proves the cut is exact (`Dag.step_wired`): one
lock-step step in which exactly the process's members tick, with inputs
only at its members and its oracle reading the baked cursors, *is* its
`locStep` written back plus `enqueue` — from **any** state whose
multi-input tick cells are drained (`Dag.Drained`, true of every state
the machine produces). Non-ticking members are inert (`Project/Step.lean`),
so nothing about the other processes is assumed. `Project/WiredRun.lean`
iterates it over an interleaving.
-/

namespace Hydro

namespace Project

variable {L : Type} {mem : L → Nat}

/-! ## Processes -/

/-- Which members a process hosts. -/
abbrev Sel (L : Type) (mem : L → Nat) := (ℓ : L) → Fin (mem ℓ) → Bool

/-- The hosted members of a location. -/
abbrev Hosted (sel : Sel L mem) (ℓ : L) := { i : Fin (mem ℓ) // sel ℓ i = true }

namespace Kind

/-- A process's increment of a wire: the hosted members' part. -/
@[reducible] def locDelta (sel : Sel L mem) : Kind L mem → Type
  | .stream ℓ α _ _ _ => Hosted sel ℓ → List α
  | .keyed ℓ K V _ _ _ _ => Hosted sel ℓ → List (K × V)
  | .sing ℓ _ σ _ _ _ _ => Hosted sel ℓ → σ
  | .ksing ℓ _ K V _ _ _ _ _ _ => Hosted sel ℓ → SchedKMap K V
  | .atomic ℓ σ => Hosted sel ℓ → List σ
  | .atomicStream ℓ α _ _ _ => Hosted sel ℓ → List (List α)
  | .atomicKeyed ℓ K V _ _ _ _ => Hosted sel ℓ → List (List (K × V))
  | .atomicKSing ℓ K V _ _ => Hosted sel ℓ → List (SchedKMap K V)

/-- Cut a global increment to the hosted members. -/
def restrictDelta (sel : Sel L mem) : (κ : Kind L mem) → κ.delta → κ.locDelta sel
  | .stream .., Δ => fun i => Δ i.1
  | .keyed .., Δ => fun i => Δ i.1
  | .sing .., Δ => fun i => Δ i.1
  | .ksing .., Δ => fun i => Δ i.1
  | .atomic .., Δ => fun i => Δ i.1
  | .atomicStream .., Δ => fun i => Δ i.1
  | .atomicKeyed .., Δ => fun i => Δ i.1
  | .atomicKSing .., Δ => fun i => Δ i.1

/-- Write a process's increment back over a global one. -/
def overrideDelta (sel : Sel L mem) : (κ : Kind L mem) → κ.delta → κ.locDelta sel → κ.delta
  | .stream ℓ .., Δ, Δ' => fun i => if h : sel ℓ i = true then Δ' ⟨i, h⟩ else Δ i
  | .keyed ℓ .., Δ, Δ' => fun i => if h : sel ℓ i = true then Δ' ⟨i, h⟩ else Δ i
  | .sing ℓ .., Δ, Δ' => fun i => if h : sel ℓ i = true then Δ' ⟨i, h⟩ else Δ i
  | .ksing ℓ .., Δ, Δ' => fun i => if h : sel ℓ i = true then Δ' ⟨i, h⟩ else Δ i
  | .atomic ℓ .., Δ, Δ' => fun i => if h : sel ℓ i = true then Δ' ⟨i, h⟩ else Δ i
  | .atomicStream ℓ .., Δ, Δ' => fun i => if h : sel ℓ i = true then Δ' ⟨i, h⟩ else Δ i
  | .atomicKeyed ℓ .., Δ, Δ' => fun i => if h : sel ℓ i = true then Δ' ⟨i, h⟩ else Δ i
  | .atomicKSing ℓ .., Δ, Δ' => fun i => if h : sel ℓ i = true then Δ' ⟨i, h⟩ else Δ i

/-- Non-hosted members' increments are empty — for the list-like kinds;
a singleton's increment is its current value and says nothing. -/
def idleDelta (sel : Sel L mem) : (κ : Kind L mem) → κ.delta → Prop
  | .stream ℓ .., Δ => ∀ i, sel ℓ i = false → Δ i = []
  | .keyed ℓ .., Δ => ∀ i, sel ℓ i = false → Δ i = []
  | .sing .., _ => True
  | .ksing .., _ => True
  | .atomic ℓ .., Δ => ∀ i, sel ℓ i = false → Δ i = []
  | .atomicStream ℓ .., Δ => ∀ i, sel ℓ i = false → Δ i = []
  | .atomicKeyed ℓ .., Δ => ∀ i, sel ℓ i = false → Δ i = []
  | .atomicKSing ℓ .., Δ => ∀ i, sel ℓ i = false → Δ i = []

/-- A list-like increment that is empty everywhere (singletons: no constraint). -/
def allEmpty : (κ : Kind L mem) → κ.delta → Prop
  | .stream .., Δ => ∀ i, Δ i = []
  | .keyed .., Δ => ∀ i, Δ i = []
  | .sing .., _ => True
  | .ksing .., _ => True
  | .atomic .., Δ => ∀ i, Δ i = []
  | .atomicStream .., Δ => ∀ i, Δ i = []
  | .atomicKeyed .., Δ => ∀ i, Δ i = []
  | .atomicKSing .., Δ => ∀ i, Δ i = []

end Kind

/-- Cut an increment environment to a process. -/
def envRestrict (sel : Sel L mem) {Γ : Ctx L mem} (Δ : HList (Kind.delta (mem := mem)) Γ) :
    HList (Kind.locDelta sel) Γ :=
  Δ.map (fun {κ} Δ => Kind.restrictDelta sel κ Δ)

/-- Write a process's increments back over a global environment. -/
def envOverride (sel : Sel L mem) : {Γ : Ctx L mem} → HList (Kind.delta (mem := mem)) Γ →
    HList (Kind.locDelta sel) Γ → HList (Kind.delta (mem := mem)) Γ
  | [], _, _ => HList.nil
  | _ :: _, Δ, Δ' => (Kind.overrideDelta sel _ Δ.1 Δ'.1, envOverride sel Δ.2 Δ'.2)

/-- Every non-hosted member's list-like increment is empty. -/
def IdleEnv (sel : Sel L mem) : {Γ : Ctx L mem} → HList (Kind.delta (mem := mem)) Γ → Prop
  | [], _ => True
  | _ :: _, Δ => Kind.idleDelta sel _ Δ.1 ∧ IdleEnv sel Δ.2

/-- Every list-like increment is empty. -/
def EmptyEnv : {Γ : Ctx L mem} → HList (Kind.delta (mem := mem)) Γ → Prop
  | [], _ => True
  | _ :: _, Δ => Kind.allEmpty _ Δ.1 ∧ EmptyEnv Δ.2

/-! ## The process's cells, oracle, sends -/

mutual

/-- The hosted members' cells of a node (`Node.State` cut to `Hosted`). -/
def Node.LocState (sel : Sel L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → Node Γ κs → Type
  | _, _, .broadcast_closed (c := c) (p := p) (α := α) _ _ _ => Hosted sel p → Fin (mem c) → ChanCell α
  | _, _, .demux (c := c) (p := p) (α := α) _ _ _ _ => Hosted sel p → Fin (mem c) → ChanCell α
  | _, _, .kfold_early_stop (ℓ := ℓ) (K := K) (A := A) _ _ _ _ _ _ => Hosted sel ℓ → KFESCell K A
  | _, _, .fold (ℓ := ℓ) (σ := σ) _ _ _ _ _ _ _ => Hosted sel ℓ → σ
  | _, _, .fold_monotone (ℓ := ℓ) (σ := σ) _ _ _ _ _ _ _ => Hosted sel ℓ → σ
  | _, _, .batch (ℓ := ℓ) (α := α) _ _ _ => Hosted sel ℓ → List α
  | _, _, .batch_ordered (ℓ := ℓ) (α := α) _ _ _ => Hosted sel ℓ → List α
  | _, _, .sample_every (ℓ := ℓ) (α := α) _ _ _ => Hosted sel ℓ → Trace (Option α) × Nat
  | _, _, .timeout_snapshot (ℓ := ℓ) _ _ _ => Hosted sel ℓ → Nat
  | _, _, .source_interval_batch (ℓ := ℓ) _ => Hosted sel ℓ → Nat
  | _, _, .zipTick (ℓ := ℓ) (α := α) (β := β) _ _ => Hosted sel ℓ → List α × List β
  | _, _, .defer_tick (ℓ := ℓ) (σ := σ) _ _ => Hosted sel ℓ → Bool × List σ
  | _, _, .reduce_watermark (ℓ := ℓ) (K := K) (V := V) _ _ _ _ _ _ _ _ _ _ =>
    Hosted sel ℓ → (List (List (K × V)) × List (Option K)) × (SchedKMap K V × Option K)
  | _, _, .tick_scan (ℓ := ℓ) sts ins _ _ _ _ => Hosted sel ℓ → PendingOf ins × BoundedOf BS BStr BKS sts
  | _, _, .call N _ => Dag.LocState sel N
  | _, _, .fixN ks body => Dag.LocState sel body × HList (Kind.locDelta sel) (ks.map KnotKind.toKind)
  | _, _, _ => Unit

def Dag.LocState (sel : Sel L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → Dag Γ outs → Type
  | _, _, .ret _ => Unit
  | _, _, .letN n rest => Node.LocState sel n × Dag.LocState sel rest

end

mutual

/-- The delivery cursors a process reads this step: per hosted receiver,
per sender, how many of the sender's items are delivered by now. -/
def Node.Oracle (sel : Sel L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → Node Γ κs → Type
  | _, _, .broadcast_closed (c := c) (p := p) _ _ _ => Hosted sel p → Fin (mem c) → Nat
  | _, _, .demux (c := c) (p := p) _ _ _ _ => Hosted sel p → Fin (mem c) → Nat
  | _, _, .call N _ => Dag.Oracle sel N
  | _, _, .fixN _ body => Dag.Oracle sel body
  | _, _, _ => Unit

def Dag.Oracle (sel : Sel L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → Dag Γ outs → Type
  | _, _, .ret _ => Unit
  | _, _, .letN n rest => Node.Oracle sel n × Dag.Oracle sel rest

end

mutual

/-- What the hosted senders of a network node put on the wire this step
(the receiver-side filtering of a `demux` happens at the append). -/
def Node.Sends (sel : Sel L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → Node Γ κs → Type
  | _, _, .broadcast_closed (c := c) (α := α) _ _ _ => Hosted sel c → List α
  | _, _, .demux (c := c) (α := α) _ _ _ _ => Hosted sel c → List (Nat × α)
  | _, _, .call N _ => Dag.Sends sel N
  | _, _, .fixN _ body => Dag.Sends sel body
  | _, _, _ => Unit

def Dag.Sends (sel : Sel L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → Dag Γ outs → Type
  | _, _, .ret _ => Unit
  | _, _, .letN n rest => Node.Sends sel n × Dag.Sends sel rest

end

/-! ## Cutting and writing back -/

mutual

/-- The global cells cut to the process. -/
def Node.restrict (sel : Sel L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    Node.State n → Node.LocState sel n
  | _, _, .broadcast_closed .., s => fun i => s i.1
  | _, _, .demux .., s => fun i => s i.1
  | _, _, .kfold_early_stop .., s => fun i => s i.1
  | _, _, .fold .., s => fun i => s i.1
  | _, _, .fold_monotone .., s => fun i => s i.1
  | _, _, .batch .., s => fun i => s i.1
  | _, _, .batch_ordered .., s => fun i => s i.1
  | _, _, .sample_every .., s => fun i => s i.1
  | _, _, .timeout_snapshot .., s => fun i => s i.1
  | _, _, .source_interval_batch .., s => fun i => s i.1
  | _, _, .zipTick .., s => fun i => s i.1
  | _, _, .defer_tick .., s => fun i => s i.1
  | _, _, .reduce_watermark .., s => fun i => s i.1
  | _, _, .tick_scan .., s => fun i => s i.1
  | _, _, .call N _, s => Dag.restrict sel N s
  | _, _, .fixN _ body, s => (Dag.restrict sel body s.1, envRestrict sel s.2)
  | _, _, .map .., _ => ()
  | _, _, .filterMap .., _ => ()
  | _, _, .values .., _ => ()
  | _, _, .into_keyed .., _ => ()
  | _, _, .kassume_ordering .., _ => ()
  | _, _, .kget_max_key .., _ => ()
  | _, _, .weaken_retries .., _ => ()
  | _, _, .union .., _ => ()
  | _, _, .assume_ordering .., _ => ()
  | _, _, .snapshot .., _ => ()
  | _, _, .mapTick .., _ => ()
  | _, _, .allTicks .., _ => ()
  | _, _, .flattenOrdered .., _ => ()
  | _, _, .flattenUnordered .., _ => ()
  | _, _, .tinto_keyed .., _ => ()
  | _, _, .tassume_ordering .., _ => ()

def Dag.restrict (sel : Sel L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    Dag.State d → Dag.LocState sel d
  | _, _, .ret _, _ => ()
  | _, _, .letN n rest, s => (Node.restrict sel n s.1, Dag.restrict sel rest s.2)

end

mutual

/-- The process's cells written back over the global ones. -/
def Node.override (sel : Sel L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    Node.State n → Node.LocState sel n → Node.State n
  | _, _, .broadcast_closed (p := p) .., s, l => fun i => if h : sel p i = true then l ⟨i, h⟩ else s i
  | _, _, .demux (p := p) .., s, l => fun i => if h : sel p i = true then l ⟨i, h⟩ else s i
  | _, _, .kfold_early_stop (ℓ := ℓ) .., s, l => fun i => if h : sel ℓ i = true then l ⟨i, h⟩ else s i
  | _, _, .fold (ℓ := ℓ) .., s, l => fun i => if h : sel ℓ i = true then l ⟨i, h⟩ else s i
  | _, _, .fold_monotone (ℓ := ℓ) .., s, l => fun i => if h : sel ℓ i = true then l ⟨i, h⟩ else s i
  | _, _, .batch (ℓ := ℓ) .., s, l => fun i => if h : sel ℓ i = true then l ⟨i, h⟩ else s i
  | _, _, .batch_ordered (ℓ := ℓ) .., s, l => fun i => if h : sel ℓ i = true then l ⟨i, h⟩ else s i
  | _, _, .sample_every (ℓ := ℓ) .., s, l => fun i => if h : sel ℓ i = true then l ⟨i, h⟩ else s i
  | _, _, .timeout_snapshot (ℓ := ℓ) .., s, l => fun i => if h : sel ℓ i = true then l ⟨i, h⟩ else s i
  | _, _, .source_interval_batch (ℓ := ℓ) .., s, l => fun i => if h : sel ℓ i = true then l ⟨i, h⟩ else s i
  | _, _, .zipTick (ℓ := ℓ) .., s, l => fun i => if h : sel ℓ i = true then l ⟨i, h⟩ else s i
  | _, _, .defer_tick (ℓ := ℓ) .., s, l => fun i => if h : sel ℓ i = true then l ⟨i, h⟩ else s i
  | _, _, .reduce_watermark (ℓ := ℓ) .., s, l => fun i => if h : sel ℓ i = true then l ⟨i, h⟩ else s i
  | _, _, .tick_scan (ℓ := ℓ) .., s, l => fun i => if h : sel ℓ i = true then l ⟨i, h⟩ else s i
  | _, _, .call N _, s, l => Dag.override sel N s l
  | _, _, .fixN _ body, s, l => (Dag.override sel body s.1 l.1, envOverride sel s.2 l.2)
  | _, _, .map .., s, _ => s
  | _, _, .filterMap .., s, _ => s
  | _, _, .values .., s, _ => s
  | _, _, .into_keyed .., s, _ => s
  | _, _, .kassume_ordering .., s, _ => s
  | _, _, .kget_max_key .., s, _ => s
  | _, _, .weaken_retries .., s, _ => s
  | _, _, .union .., s, _ => s
  | _, _, .assume_ordering .., s, _ => s
  | _, _, .snapshot .., s, _ => s
  | _, _, .mapTick .., s, _ => s
  | _, _, .allTicks .., s, _ => s
  | _, _, .flattenOrdered .., s, _ => s
  | _, _, .flattenUnordered .., s, _ => s
  | _, _, .tinto_keyed .., s, _ => s
  | _, _, .tassume_ordering .., s, _ => s

def Dag.override (sel : Sel L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    Dag.State d → Dag.LocState sel d → Dag.State d
  | _, _, .ret _, _, _ => ()
  | _, _, .letN n rest, s, l => (Node.override sel n s.1 l.1, Dag.override sel rest s.2 l.2)

end

mutual

/-- The active process's sends appended to every **other** receiver's
queue (its own receivers took them inside `locStep`). -/
def Node.enqueue (sel : Sel L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    Node.Sends sel n → Node.State n → Node.State n
  | _, _, .broadcast_closed (c := c) (p := p) .., snd, s => fun i j =>
    if sel p i = true then s i j
    else if h : sel c j = true then (s i j).enqueue (snd ⟨j, h⟩) else s i j
  | _, _, .demux (c := c) (p := p) _ _ _ addr, snd, s => fun i j =>
    if sel p i = true then s i j
    else if h : sel c j = true then
      (s i j).enqueue ((snd ⟨j, h⟩).filterMap (fun dx => if dx.1 = addr i then some dx.2 else none))
    else s i j
  | _, _, .call N _, snd, s => Dag.enqueue sel N snd s
  | _, _, .fixN _ body, snd, s => (Dag.enqueue sel body snd s.1, s.2)
  | _, _, .kfold_early_stop .., _, s => s
  | _, _, .fold .., _, s => s
  | _, _, .fold_monotone .., _, s => s
  | _, _, .batch .., _, s => s
  | _, _, .batch_ordered .., _, s => s
  | _, _, .sample_every .., _, s => s
  | _, _, .timeout_snapshot .., _, s => s
  | _, _, .source_interval_batch .., _, s => s
  | _, _, .zipTick .., _, s => s
  | _, _, .defer_tick .., _, s => s
  | _, _, .reduce_watermark .., _, s => s
  | _, _, .tick_scan .., _, s => s
  | _, _, .map .., _, s => s
  | _, _, .filterMap .., _, s => s
  | _, _, .values .., _, s => s
  | _, _, .into_keyed .., _, s => s
  | _, _, .kassume_ordering .., _, s => s
  | _, _, .kget_max_key .., _, s => s
  | _, _, .weaken_retries .., _, s => s
  | _, _, .union .., _, s => s
  | _, _, .assume_ordering .., _, s => s
  | _, _, .snapshot .., _, s => s
  | _, _, .mapTick .., _, s => s
  | _, _, .allTicks .., _, s => s
  | _, _, .flattenOrdered .., _, s => s
  | _, _, .flattenUnordered .., _, s => s
  | _, _, .tinto_keyed .., _, s => s
  | _, _, .tassume_ordering .., _, s => s

def Dag.enqueue (sel : Sel L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    Dag.Sends sel d → Dag.State d → Dag.State d
  | _, _, .ret _, _, _ => ()
  | _, _, .letN n rest, snd, s => (Node.enqueue sel n snd.1 s.1, Dag.enqueue sel rest snd.2 s.2)

end

mutual

/-- The same append, landing in another process's cells (its hosted receivers). -/
def Node.enqueueLoc (sel : Sel L mem) (selS : Sel L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} →
    (n : Node Γ κs) → Node.Sends selS n → Node.LocState sel n → Node.LocState sel n
  | _, _, .broadcast_closed (c := c) .., snd, s => fun i j =>
    if h : selS c j = true then (s i j).enqueue (snd ⟨j, h⟩) else s i j
  | _, _, .demux (c := c) _ _ _ addr, snd, s => fun i j =>
    if h : selS c j = true then
      (s i j).enqueue ((snd ⟨j, h⟩).filterMap (fun dx => if dx.1 = addr i.1 then some dx.2 else none))
    else s i j
  | _, _, .call N _, snd, s => Dag.enqueueLoc sel selS N snd s
  | _, _, .fixN _ body, snd, s => (Dag.enqueueLoc sel selS body snd s.1, s.2)
  | _, _, .kfold_early_stop .., _, s => s
  | _, _, .fold .., _, s => s
  | _, _, .fold_monotone .., _, s => s
  | _, _, .batch .., _, s => s
  | _, _, .batch_ordered .., _, s => s
  | _, _, .sample_every .., _, s => s
  | _, _, .timeout_snapshot .., _, s => s
  | _, _, .source_interval_batch .., _, s => s
  | _, _, .zipTick .., _, s => s
  | _, _, .defer_tick .., _, s => s
  | _, _, .reduce_watermark .., _, s => s
  | _, _, .tick_scan .., _, s => s
  | _, _, .map .., _, s => s
  | _, _, .filterMap .., _, s => s
  | _, _, .values .., _, s => s
  | _, _, .into_keyed .., _, s => s
  | _, _, .kassume_ordering .., _, s => s
  | _, _, .kget_max_key .., _, s => s
  | _, _, .weaken_retries .., _, s => s
  | _, _, .union .., _, s => s
  | _, _, .assume_ordering .., _, s => s
  | _, _, .snapshot .., _, s => s
  | _, _, .mapTick .., _, s => s
  | _, _, .allTicks .., _, s => s
  | _, _, .flattenOrdered .., _, s => s
  | _, _, .flattenUnordered .., _, s => s
  | _, _, .tinto_keyed .., _, s => s
  | _, _, .tassume_ordering .., _, s => s

def Dag.enqueueLoc (sel : Sel L mem) (selS : Sel L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} →
    (d : Dag Γ outs) → Dag.Sends selS d → Dag.LocState sel d → Dag.LocState sel d
  | _, _, .ret _, _, _ => ()
  | _, _, .letN n rest, snd, s => (Node.enqueueLoc sel selS n snd.1 s.1, Dag.enqueueLoc sel selS rest snd.2 s.2)

end

/-! ## Materializing a step's per-member values

A node's increments and cells are per-member functions; left as the
closures `locStep` builds, every consumer would recompute its producer
(call-by-name over a graph with fan-out — the `n^depth` unsharing D73 met
at `Values`). `memo`/`memo2` are the identity in the logic and build a
table in compiled code (`@[csimp]`, the one sanctioned seam: the equation
is kernel-checked). `Dag.locStep` materializes each node's cells and
increments before the rest of the graph reads them.

Three compiler behaviours shape the encoding (each defeated the first
attempts, measured at the demo):
1. every declaration *and local function* whose type is a function type is
   eta-expanded (LCNF `etaExpand`), floating a `let tbl := …` into the
   returned closure, so the table was rebuilt on every call — hence the
   tables live in structures (`MemoBox`), never eta-expanded;
2. a structure with one relevant field is erased to that field ("trivial
   structure"), making it a function type again — hence the `tag` fields;
3. a `match` arm whose type reduces to a function type is eta-expanded as
   well — hence `Node.memoStateBox` returns `StateBox` and `Node.memoState`
   projects outside the match. -/

/-- A hosted-member function (the memoized form is a table behind it). The
`tag` field keeps the structure from being a "trivial structure" (one relevant
field), which the compiler would erase to the bare function type and then
eta-expand away. -/
structure MemoBox (sel : Sel L mem) (ℓ : L) (X : Type) where
  fn : Hosted sel ℓ → X
  tag : Unit := ()

/-- The identity box (materialized when compiled, `memoBox_eq_memoBoxImpl`). -/
@[reducible] def memoBox {sel : Sel L mem} {ℓ : L} {X : Type} (f : Hosted sel ℓ → X) : MemoBox sel ℓ X := ⟨f, ()⟩

/-- Look a hosted member up in a materialized table (`@[noinline]` so the
partial application `lookupHosted f tbl h` is the stored closure). -/
@[noinline] def lookupHosted {sel : Sel L mem} {ℓ : L} {X : Type} (f : Hosted sel ℓ → X)
    (tbl : Array (Option X)) (h : tbl.size = mem ℓ) : Hosted sel ℓ → X :=
  fun i => match tbl[i.1.val]'(by rw [h]; exact i.1.isLt) with
    | some v => v
    | none => f i

def memoBoxImpl {sel : Sel L mem} {ℓ : L} {X : Type} (f : Hosted sel ℓ → X) : MemoBox sel ℓ X :=
  ⟨lookupHosted f (Array.ofFn fun (i : Fin (mem ℓ)) => if h : sel ℓ i = true then some (f ⟨i, h⟩) else none)
    Array.size_ofFn, ()⟩

@[csimp] theorem memoBox_eq_memoBoxImpl : @memoBox = @memoBoxImpl := by
  funext L mem sel ℓ X f
  simp only [memoBox, memoBoxImpl, MemoBox.mk.injEq, and_true]
  funext i
  simp only [lookupHosted, Array.getElem_ofFn, Fin.eta, i.2, dite_true]

/-- The identity on hosted-member functions (materialized when compiled). -/
@[macro_inline, reducible] def memo {sel : Sel L mem} {ℓ : L} {X : Type} (f : Hosted sel ℓ → X) : Hosted sel ℓ → X :=
  (memoBox f).fn

/-- A member function behind a table (`tag` as in `MemoBox`); the inner level of `memo2`. -/
structure MemoFinBox (n : Nat) (X : Type) where
  fn : Fin n → X
  tag : Unit := ()

/-- Look a member up in a materialized table (`@[noinline]`, as `lookupHosted`). -/
@[noinline] def lookupFin {n : Nat} {X : Type} (tbl : Array X) (h : tbl.size = n) : Fin n → X :=
  fun i => tbl[i.val]'(by rw [h]; exact i.isLt)

def memoFinBoxImpl {n : Nat} {X : Type} (f : Fin n → X) : MemoFinBox n X :=
  ⟨lookupFin (Array.ofFn f) Array.size_ofFn, ()⟩

/-- The identity on two-level member functions (network cells: hosted receiver
× sender). The inner tables are built as array *elements*: a lambda
`fun i => (memoFinBox (s i)).fn` would be eta-expanded by the compiler and
rebuild the inner table on every call. -/
@[reducible] def memoBox2 {sel : Sel L mem} {ℓ : L} {n : Nat} {X : Type} (f : Hosted sel ℓ → Fin n → X) :
    MemoBox sel ℓ (Fin n → X) := ⟨f, ()⟩

def memoBox2Impl {sel : Sel L mem} {ℓ : L} {n : Nat} {X : Type} (f : Hosted sel ℓ → Fin n → X) :
    MemoBox sel ℓ (Fin n → X) :=
  ⟨lookupHosted f (Array.ofFn fun (i : Fin (mem ℓ)) =>
      if h : sel ℓ i = true then some (memoFinBoxImpl (f ⟨i, h⟩)).fn else none)
    Array.size_ofFn, ()⟩

@[csimp] theorem memoBox2_eq_memoBox2Impl : @memoBox2 = @memoBox2Impl := by
  funext L mem sel ℓ n X f
  simp only [memoBox2, memoBox2Impl, MemoBox.mk.injEq, and_true]
  funext i j
  simp only [lookupHosted, Array.getElem_ofFn, Fin.eta, i.2, dite_true, memoFinBoxImpl, lookupFin]

@[macro_inline, reducible] def memo2 {sel : Sel L mem} {ℓ : L} {n : Nat} {X : Type}
    (f : Hosted sel ℓ → Fin n → X) : Hosted sel ℓ → Fin n → X :=
  (memoBox2 f).fn

/-- Materialize every increment of a process's environment. -/
def memoEnv (sel : Sel L mem) : {Γ : Ctx L mem} → HList (Kind.locDelta sel) Γ → HList (Kind.locDelta sel) Γ
  | [], _ => HList.nil
  | .stream .. :: _, Δ => (memo Δ.1, memoEnv sel Δ.2)
  | .keyed .. :: _, Δ => (memo Δ.1, memoEnv sel Δ.2)
  | .sing .. :: _, Δ => (memo Δ.1, memoEnv sel Δ.2)
  | .ksing .. :: _, Δ => (memo Δ.1, memoEnv sel Δ.2)
  | .atomic .. :: _, Δ => (memo Δ.1, memoEnv sel Δ.2)
  | .atomicStream .. :: _, Δ => (memo Δ.1, memoEnv sel Δ.2)
  | .atomicKeyed .. :: _, Δ => (memo Δ.1, memoEnv sel Δ.2)
  | .atomicKSing .. :: _, Δ => (memo Δ.1, memoEnv sel Δ.2)

theorem memoEnv_eq (sel : Sel L mem) : {Γ : Ctx L mem} → (Δ : HList (Kind.locDelta sel) Γ) → memoEnv sel Δ = Δ
  | [], _ => rfl
  | .stream .. :: _, Δ => by show (Δ.1, memoEnv sel Δ.2) = Δ; rw [memoEnv_eq sel Δ.2]
  | .keyed .. :: _, Δ => by show (Δ.1, memoEnv sel Δ.2) = Δ; rw [memoEnv_eq sel Δ.2]
  | .sing .. :: _, Δ => by show (Δ.1, memoEnv sel Δ.2) = Δ; rw [memoEnv_eq sel Δ.2]
  | .ksing .. :: _, Δ => by show (Δ.1, memoEnv sel Δ.2) = Δ; rw [memoEnv_eq sel Δ.2]
  | .atomic .. :: _, Δ => by show (Δ.1, memoEnv sel Δ.2) = Δ; rw [memoEnv_eq sel Δ.2]
  | .atomicStream .. :: _, Δ => by show (Δ.1, memoEnv sel Δ.2) = Δ; rw [memoEnv_eq sel Δ.2]
  | .atomicKeyed .. :: _, Δ => by show (Δ.1, memoEnv sel Δ.2) = Δ; rw [memoEnv_eq sel Δ.2]
  | .atomicKSing .. :: _, Δ => by show (Δ.1, memoEnv sel Δ.2) = Δ; rw [memoEnv_eq sel Δ.2]

/-- A node's cells, boxed. The match arms of `Node.memoStateBox` return this
structure rather than the cells themselves: an arm whose type reduces to a
function type gets eta-expanded by the compiler, which would move the table
construction into the returned closure. -/
structure StateBox (sel : Sel L mem) {Γ : Ctx L mem} {κs : List (Kind L mem)} (n : Node Γ κs) where
  val : Node.LocState sel n
  tag : Unit := ()

mutual

/-- Materialize a node's cells (boxed). -/
def Node.memoStateBox (sel : Sel L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    Node.LocState sel n → StateBox sel n
  | _, _, .broadcast_closed .., s => ⟨memo2 s, ()⟩
  | _, _, .demux .., s => ⟨memo2 s, ()⟩
  | _, _, .kfold_early_stop .., s => ⟨memo s, ()⟩
  | _, _, .fold .., s => ⟨memo s, ()⟩
  | _, _, .fold_monotone .., s => ⟨memo s, ()⟩
  | _, _, .batch .., s => ⟨memo s, ()⟩
  | _, _, .batch_ordered .., s => ⟨memo s, ()⟩
  | _, _, .sample_every .., s => ⟨memo s, ()⟩
  | _, _, .timeout_snapshot .., s => ⟨memo s, ()⟩
  | _, _, .source_interval_batch .., s => ⟨memo s, ()⟩
  | _, _, .zipTick .., s => ⟨memo s, ()⟩
  | _, _, .defer_tick .., s => ⟨memo s, ()⟩
  | _, _, .reduce_watermark .., s => ⟨memo s, ()⟩
  | _, _, .tick_scan .., s => ⟨memo s, ()⟩
  | _, _, .call N _, s => ⟨Dag.memoState sel N s, ()⟩
  | _, _, .fixN _ body, s => ⟨(Dag.memoState sel body s.1, memoEnv sel s.2), ()⟩
  | _, _, .map .., s => ⟨s, ()⟩
  | _, _, .filterMap .., s => ⟨s, ()⟩
  | _, _, .values .., s => ⟨s, ()⟩
  | _, _, .into_keyed .., s => ⟨s, ()⟩
  | _, _, .kassume_ordering .., s => ⟨s, ()⟩
  | _, _, .kget_max_key .., s => ⟨s, ()⟩
  | _, _, .weaken_retries .., s => ⟨s, ()⟩
  | _, _, .union .., s => ⟨s, ()⟩
  | _, _, .assume_ordering .., s => ⟨s, ()⟩
  | _, _, .snapshot .., s => ⟨s, ()⟩
  | _, _, .mapTick .., s => ⟨s, ()⟩
  | _, _, .allTicks .., s => ⟨s, ()⟩
  | _, _, .flattenOrdered .., s => ⟨s, ()⟩
  | _, _, .flattenUnordered .., s => ⟨s, ()⟩
  | _, _, .tinto_keyed .., s => ⟨s, ()⟩
  | _, _, .tassume_ordering .., s => ⟨s, ()⟩

/-- Materialize a graph's cells. -/
def Dag.memoState (sel : Sel L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    Dag.LocState sel d → Dag.LocState sel d
  | _, _, .ret _, s => s
  | _, _, .letN n rest, s => ((Node.memoStateBox sel n s.1).val, Dag.memoState sel rest s.2)

end

/-- Materialize a node's cells. -/
@[reducible] def Node.memoState (sel : Sel L mem) {Γ : Ctx L mem} {κs : List (Kind L mem)} (n : Node Γ κs)
    (s : Node.LocState sel n) : Node.LocState sel n :=
  (Node.memoStateBox sel n s).val

mutual

theorem Node.memoState_eq (sel : Sel L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    (s : Node.LocState sel n) → Node.memoState sel n s = s
  | _, _, .broadcast_closed .., _ => rfl
  | _, _, .demux .., _ => rfl
  | _, _, .kfold_early_stop .., _ => rfl
  | _, _, .fold .., _ => rfl
  | _, _, .fold_monotone .., _ => rfl
  | _, _, .batch .., _ => rfl
  | _, _, .batch_ordered .., _ => rfl
  | _, _, .sample_every .., _ => rfl
  | _, _, .timeout_snapshot .., _ => rfl
  | _, _, .source_interval_batch .., _ => rfl
  | _, _, .zipTick .., _ => rfl
  | _, _, .defer_tick .., _ => rfl
  | _, _, .reduce_watermark .., _ => rfl
  | _, _, .tick_scan .., _ => rfl
  | _, _, .call N _, s => Dag.memoState_eq sel N s
  | _, _, .fixN _ body, s => by
    show (Dag.memoState sel body s.1, memoEnv sel s.2) = s
    rw [Dag.memoState_eq sel body s.1, memoEnv_eq]
    rfl
  | _, _, .map .., _ => rfl
  | _, _, .filterMap .., _ => rfl
  | _, _, .values .., _ => rfl
  | _, _, .into_keyed .., _ => rfl
  | _, _, .kassume_ordering .., _ => rfl
  | _, _, .kget_max_key .., _ => rfl
  | _, _, .weaken_retries .., _ => rfl
  | _, _, .union .., _ => rfl
  | _, _, .assume_ordering .., _ => rfl
  | _, _, .snapshot .., _ => rfl
  | _, _, .mapTick .., _ => rfl
  | _, _, .allTicks .., _ => rfl
  | _, _, .flattenOrdered .., _ => rfl
  | _, _, .flattenUnordered .., _ => rfl
  | _, _, .tinto_keyed .., _ => rfl
  | _, _, .tassume_ordering .., _ => rfl

theorem Dag.memoState_eq (sel : Sel L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    (s : Dag.LocState sel d) → Dag.memoState sel d s = s
  | _, _, .ret _, _ => rfl
  | _, _, .letN n rest, s => by
    show (Node.memoState sel n s.1, Dag.memoState sel rest s.2) = s
    rw [Node.memoState_eq sel n s.1, Dag.memoState_eq sel rest s.2]
    rfl

end

/-! ## The process's step -/

/-- The hosted part of a shaped tick input's increments. -/
def readShapeLoc (sel : Sel L mem) {Γ : Ctx L mem} (Δ : HList (Kind.locDelta sel) Γ) (ℓ : L) :
    (sh : TickShape) → VarsOf Γ ℓ sh → HList (Kind.locDelta sel) (shapeKinds ℓ sh)
  | .sing _, v => HList.cons (Δ.get v) HList.nil
  | .stream _ _ _ _, v => HList.cons (Δ.get v) HList.nil
  | .ksing _ _ _ _, v => HList.cons (Δ.get v) HList.nil
  | .pair a b, (va, vb) => (readShapeLoc sel Δ ℓ a va).append (readShapeLoc sel Δ ℓ b vb)

/-- Push one hosted member's leaf increments (a per-member view of `PendingOf.push`). -/
def PendingOf.pushLoc (sel : Sel L mem) {ℓ : L} : (sh : TickShape) → PendingOf sh →
    (Hosted sel ℓ → HList (Kind.locDelta sel) (shapeKinds ℓ sh)) → Hosted sel ℓ → PendingOf sh
  | .sing _, p, Δ, i => p ++ (Δ i).head i
  | .stream _ _ _ _, p, Δ, i => p ++ (Δ i).head i
  | .ksing _ _ _ _, p, Δ, i => p ++ (Δ i).head i
  | .pair a b, (pa, pb), Δ, i =>
    (pushLoc sel a pa (fun i => (Δ i).take) i, pushLoc sel b pb (fun i => (Δ i).drop) i)

/-- Unzip hosted members' shaped outputs into per-leaf increments. -/
def unzipShapeLoc (sel : Sel L mem) (ℓ : L) : (sh : TickShape) →
    (Hosted sel ℓ → List (BoundedOf BS BStr BKS sh)) → HList (Kind.locDelta sel) (shapeKinds ℓ sh)
  | .sing _, l => HList.cons (fun i => l i) HList.nil
  | .stream _ _ _ _, l => HList.cons (fun i => l i) HList.nil
  | .ksing _ _ _ _, l => HList.cons (fun i => l i) HList.nil
  | .pair a b, l =>
    (unzipShapeLoc sel ℓ a (fun i => (l i).map Prod.fst)).append (unzipShapeLoc sel ℓ b (fun i => (l i).map Prod.snd))

mutual

/-- **One step of a process**: its hosted members' cells and increments
only. The process is *active*: deliveries land (a network node delivers up
to the oracle's cursor), knot feedback propagates, stream operators fire;
its tick bodies run iff `tk` (a step with `tk = false` is a **stutter** —
the asynchronous dataflow progresses between ticks; a DFIR tick is
"stutters, then a tick"). At runtime a process is one cluster member and
`tk` ticks every `Tick` location of it at once. A network node takes the
hosted senders' items into its own queues (`ChanCell.step`: deliver, then
enqueue), reporting them as `Sends` for the other processes' queues. -/
def Node.locStep (sel : Sel L mem) (tk : Bool) :
    {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) → Node.Oracle sel n →
    HList (Kind.locDelta sel) Γ → Node.LocState sel n →
    Node.LocState sel n × HList (Kind.locDelta sel) κs × Node.Sends sel n
  | _, _, .map _ _ x f, _, Δ, s => (s, HList.cons (fun i => (Δ.get x i).map (f i.1)) HList.nil, ())
  | _, _, .filterMap _ _ x f, _, Δ, s => (s, HList.cons (fun i => (Δ.get x i).filterMap (f i.1)) HList.nil, ())
  | _, _, .broadcast_closed (c := c) _ _ x, ora, Δ, s =>
    let sends := fun j => if h : sel c j = true then Δ.get x ⟨j, h⟩ else []
    let cells := fun i j => (s i j).step (ora i j) (sends j)
    (fun i j => (cells i j).1,
     HList.cons (fun i => (List.finRange (mem c)).flatMap (fun j => ((cells i j).2).map (fun v => (j, v)))) HList.nil,
     fun j => Δ.get x j)
  | _, _, .demux (c := c) _ _ x addr, ora, Δ, s =>
    let sends := fun i j => if h : sel c j = true then
      (Δ.get x ⟨j, h⟩).filterMap (fun dx => if dx.1 = addr i then some dx.2 else none) else []
    let cells := fun i j => (s i j).step (ora i j) (sends i.1 j)
    (fun i j => (cells i j).1,
     HList.cons (fun i => (List.finRange (mem c)).flatMap (fun j => ((cells i j).2).map (fun v => (j, v)))) HList.nil,
     fun j => Δ.get x j)
  | _, _, .values _ _ x, _, Δ, s => (s, HList.cons (fun i => (Δ.get x i).map Prod.snd) HList.nil, ())
  | _, _, .into_keyed (ℓ := ℓ) (K := K) (V := V) _ _ (ord := ord) x, _, Δ, s =>
    (s, HList.cons (fun i => (Δ.get x : (Kind.stream (mem := mem) ℓ (K × V) inferInstance ord .exactlyOnce).locDelta sel) i) HList.nil, ())
  | _, _, .kassume_ordering (ℓ := ℓ) (K := K) (V := V) iK iV x _, _, Δ, s =>
    (s, HList.cons (fun i => (Δ.get x : (Kind.keyed (mem := mem) ℓ K V iK iV .noOrder .exactlyOnce).locDelta sel) i) HList.nil, ())
  | _, _, .kfold_early_stop _ _ _ x f init, _, Δ, s =>
    let s' := fun i => (Δ.get x i).foldl (kfesInsert f init) (s i)
    (s', HList.cons (fun i => kfesOutMap (s' i)) HList.nil, ())
  | _, _, .kget_max_key _ _ _ _ x, _, Δ, s => (s, HList.cons (fun i => (Δ.get x i).maxEntry) HList.nil, ())
  | _, _, .weaken_retries (ℓ := ℓ) (α := α) iα (ord := ord) x, _, Δ, s =>
    (s, HList.cons (fun i => (Δ.get x : (Kind.stream (mem := mem) ℓ α iα ord .exactlyOnce).locDelta sel) i) HList.nil, ())
  | _, _, .union _ a b, _, Δ, s => (s, HList.cons (fun i => Δ.get a i ++ Δ.get b i) HList.nil, ())
  | _, _, .assume_ordering (ℓ := ℓ) (α := α) iα x _, _, Δ, s =>
    (s, HList.cons (fun i => (Δ.get x : (Kind.stream (mem := mem) ℓ α iα .noOrder .exactlyOnce).locDelta sel) i) HList.nil, ())
  | _, _, .fold _ g _ _ _ _ x, _, Δ, s =>
    let s' := fun i => (Δ.get x i).foldl g (s i)
    (s', HList.cons s' HList.nil, ())
  | _, _, .fold_monotone _ _ g _ _ _ x, _, Δ, s =>
    let s' := fun i => (Δ.get x i).foldl g (s i)
    (s', HList.cons s' HList.nil, ())
  | _, _, .snapshot (ℓ := ℓ) _ x _, _, Δ, s =>
    (s, HList.cons (fun i => if tk then [Δ.get x i] else []) HList.nil, ())
  | _, _, .batch (ℓ := ℓ) _ x _, _, Δ, s =>
    let p := fun i => s i ++ Δ.get x i
    (fun i => if tk then [] else p i, HList.cons (fun i => if tk then [p i] else []) HList.nil, ())
  | _, _, .batch_ordered (ℓ := ℓ) _ x _, _, Δ, s =>
    let p := fun i => s i ++ Δ.get x i
    (fun i => if tk then [] else p i, HList.cons (fun i => if tk then [p i] else []) HList.nil, ())
  | _, _, .sample_every _ x d, _, Δ, s =>
    let tr := fun i => (s i).1 ++ Δ.get x i
    let adv := fun i => sampleAdvance (tr i) (d i.1) (s i).2
    (fun i => (tr i, (adv i).1), HList.cons (fun i => (adv i).2) HList.nil, ())
  | _, _, .timeout_snapshot (ℓ := ℓ) _ _ d, _, _, s =>
    (fun i => if tk then s i + 1 else s i, HList.cons (fun i => if tk then ((d i.1)[s i]?).toList else []) HList.nil, ())
  | _, _, .source_interval_batch (ℓ := ℓ) d, _, _, s =>
    (fun i => if tk then s i + 1 else s i, HList.cons (fun i => if tk then ((d i.1)[s i]?).toList else []) HList.nil, ())
  | _, _, .mapTick x f, _, Δ, s => (s, HList.cons (fun i => (Δ.get x i).map (f i.1)) HList.nil, ())
  | _, _, .zipTick a b, _, Δ, s =>
    let pa := fun i => (s i).1 ++ Δ.get a i
    let pb := fun i => (s i).2 ++ Δ.get b i
    let n := fun i => min (pa i).length (pb i).length
    (fun i => ((pa i).drop (n i), (pb i).drop (n i)),
     HList.cons (fun i => List.zip (pa i) (pb i)) HList.nil, ())
  | _, _, .defer_tick init x, _, Δ, s =>
    -- fired: pass through; unfired at a tick: fire with the buffered input; unfired at a stutter: buffer
    (fun i => if (s i).1 then s i else if tk then (true, []) else (false, (s i).2 ++ Δ.get x i),
     HList.cons (fun i => if (s i).1 then Δ.get x i
       else if tk then init :: ((s i).2 ++ Δ.get x i) else []) HList.nil, ())
  | _, _, .allTicks _ x, _, Δ, s => (s, HList.cons (fun i => (Δ.get x i).flatten) HList.nil, ())
  | _, _, .flattenOrdered (ℓ := ℓ) (β := β) _ x, _, Δ, s =>
    (s, HList.cons (fun i => (Δ.get x : (Kind.atomic (mem := mem) ℓ (List β)).locDelta sel) i) HList.nil, ())
  | _, _, .flattenUnordered (ℓ := ℓ) (β := β) _ x, _, Δ, s =>
    (s, HList.cons (fun i => (Δ.get x : (Kind.atomic (mem := mem) ℓ (List β)).locDelta sel) i) HList.nil, ())
  | _, _, .tinto_keyed (ℓ := ℓ) (K := K) (V := V) _ _ (ord := ord) x, _, Δ, s =>
    (s, HList.cons (fun i => (Δ.get x : (Kind.atomicStream (mem := mem) ℓ (K × V) inferInstance ord .exactlyOnce).locDelta sel) i) HList.nil, ())
  | _, _, .tassume_ordering (ℓ := ℓ) (α := α) iα x _, _, Δ, s =>
    (s, HList.cons (fun i => (Δ.get x : (Kind.atomicStream (mem := mem) ℓ α iα .noOrder .exactlyOnce).locDelta sel) i) HList.nil, ())
  | _, _, .reduce_watermark _ iH _ _ x wm f _ _ _, _, Δ, s =>
    let pa := fun i => (s i).1.1 ++ Δ.get x i
    let pb := fun i => (s i).1.2 ++ Δ.get wm i
    let n := fun i => min (pa i).length (pb i).length
    let run := fun i => scanRun (reduceWatermarkEagerH (K := _) f) (s i).2 (List.zip (pa i) (pb i))
    (fun i => (((pa i).drop (n i), (pb i).drop (n i)), (run i).1),
     HList.cons (fun i => (run i).2) HList.nil, ())
  | _, _, .tick_scan (ℓ := ℓ) _ ins outs x g _, _, Δ, s =>
    let leaves : Hosted sel ℓ → HList (Kind.locDelta sel) (shapeKinds ℓ ins) :=
      fun _ => readShapeLoc sel Δ ℓ ins x
    let p := fun i => PendingOf.pushLoc sel ins (s i).1 leaves i
    let n := fun i => PendingOf.avail ins (p i)
    let run := fun i => scanRun (g i.1) (s i).2 (PendingOf.takeZip ins (p i) (n i))
    (fun i => (PendingOf.dropN ins (p i) (n i), (run i).1), unzipShapeLoc sel ℓ outs (fun i => (run i).2), ())
  | _, _, .call N args, ora, Δ, s => Dag.locStep sel tk N ora (args.map (fun v => Δ.get v)) s
  | _, _, .fixN _ body, ora, Δ, s =>
    -- active: release all pending feedback, keep the body's new output
    let r := Dag.locStep sel tk body ora (s.2.append Δ) s.1
    ((r.1, r.2.1.take), s.2.append r.2.1.drop, r.2.2)

/-- One step of a process over a graph. -/
def Dag.locStep (sel : Sel L mem) (tk : Bool) :
    {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) → Dag.Oracle sel d →
    HList (Kind.locDelta sel) Γ → Dag.LocState sel d →
    Dag.LocState sel d × HList (Kind.locDelta sel) outs × Dag.Sends sel d
  | _, _, .ret xs, _, Δ, s => (s, xs.map (fun v => Δ.get v), ())
  | _, _, .letN n rest, ora, Δ, s =>
    let r := Node.locStep sel tk n ora.1 Δ s.1
    -- materialize this node's cells and increments before the rest reads them
    let st := Node.memoState sel n r.1
    let out := memoEnv sel r.2.1
    let r' := Dag.locStep sel tk rest ora.2 (out.append Δ) s.2
    ((st, r'.1), r'.2.1, (r.2.2, r'.2.2))

end

end Project

end Hydro
