import Hydro.Project.Step

/-!
# Hydro · Project — the wired machine (per process)

The lock-step machine (`Project/Step.lean`) steps every member of every
location at every global step. A **process** is one member
(`Member`); it owns the member's cells (`LocState`), consumes their input
increments, reads its network **oracle** (how far each incoming channel
is delivered now), and produces the increments
of its wires plus the **sends** it put on its channels. Other
processes' cells it never touches; what crosses between processes is
exactly the channel append (`enqueue`).

- `Node.LocState`/`Dag.LocState`: the member's cells (a network
  node's cells sit with the receiver: its per-sender queues);
- `Node.Oracle`: the delivery cursors read this step, at the member's receiving ends;
- `Node.Sends`: what the member put on the wire this step;
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

/-! ## Processes are members -/

/-- A cluster member — at runtime, one process. -/
abbrev Member (L : Type) (mem : L → Nat) := (ℓ : L) × Fin (mem ℓ)

/-- The member `m` at location `ℓ`: the index `i` with `⟨ℓ, i⟩ = m` (none if `m` lives
elsewhere). A process's cells, increments and sends are indexed by it. -/
abbrev Hosted (m : Member L mem) (ℓ : L) := { i : Fin (mem ℓ) // (⟨ℓ, i⟩ : Member L mem) = m }

/-- A member of another process is not this one. -/
theorem Hosted.ne_of_ne {m m' : Member L mem} (hd : m' ≠ m) {ℓ : L} (i : Hosted m' ℓ) :
    (⟨ℓ, i.1⟩ : Member L mem) ≠ m := fun h => hd (i.2.symm.trans h)

section Decidable

variable [DecidableEq L]

/-- `m` as the lock-step machine's activity mask: true exactly at `m`. -/
def Member.hosts (m : Member L mem) : (ℓ : L) → Fin (mem ℓ) → Bool :=
  fun ℓ i => decide ((⟨ℓ, i⟩ : Member L mem) = m)

@[simp] theorem Member.hosts_eq_true (m : Member L mem) (ℓ : L) (i : Fin (mem ℓ)) :
    m.hosts ℓ i = true ↔ (⟨ℓ, i⟩ : Member L mem) = m := by simp [Member.hosts]

@[simp] theorem Member.hosts_eq_false (m : Member L mem) (ℓ : L) (i : Fin (mem ℓ)) :
    m.hosts ℓ i = false ↔ (⟨ℓ, i⟩ : Member L mem) ≠ m := by simp [Member.hosts]

@[simp] theorem Member.hosts_self (m : Member L mem) : m.hosts m.1 m.2 = true := by simp [Member.hosts]

namespace Kind

/-- A process's increment of a wire: the member's part (`Hosted m ℓ` is the member's index at `ℓ`, if any). -/
@[reducible] def locDelta (m : Member L mem) : Kind L mem → Type
  | .stream ℓ α _ _ _ => Hosted m ℓ → List α
  | .keyed ℓ K V _ _ _ _ => Hosted m ℓ → List (K × V)
  | .sing ℓ _ σ _ _ _ _ => Hosted m ℓ → σ
  | .ksing ℓ _ K V _ _ _ _ _ _ => Hosted m ℓ → SchedKMap K V
  | .atomic ℓ σ => Hosted m ℓ → Option σ
  | .atomicStream ℓ α _ _ _ => Hosted m ℓ → Option (List α)
  | .atomicKeyed ℓ K V _ _ _ _ => Hosted m ℓ → Option (List (K × V))
  | .atomicKSing ℓ K V _ _ => Hosted m ℓ → Option (SchedKMap K V)

/-- Cut a global increment to the member. -/
def restrictDelta (m : Member L mem) : (κ : Kind L mem) → κ.delta → κ.locDelta m
  | .stream .., Δ => fun i => Δ i.1
  | .keyed .., Δ => fun i => Δ i.1
  | .sing .., Δ => fun i => Δ i.1
  | .ksing .., Δ => fun i => Δ i.1
  | .atomic .., Δ => fun i => Δ i.1
  | .atomicStream .., Δ => fun i => Δ i.1
  | .atomicKeyed .., Δ => fun i => Δ i.1
  | .atomicKSing .., Δ => fun i => Δ i.1

/-- Write a process's increment back over a global one. -/
def overrideDelta (m : Member L mem) : (κ : Kind L mem) → κ.delta → κ.locDelta m → κ.delta
  | .stream ℓ .., Δ, Δ' => fun i => if h : (⟨ℓ, i⟩ : Member L mem) = m then Δ' ⟨i, h⟩ else Δ i
  | .keyed ℓ .., Δ, Δ' => fun i => if h : (⟨ℓ, i⟩ : Member L mem) = m then Δ' ⟨i, h⟩ else Δ i
  | .sing ℓ .., Δ, Δ' => fun i => if h : (⟨ℓ, i⟩ : Member L mem) = m then Δ' ⟨i, h⟩ else Δ i
  | .ksing ℓ .., Δ, Δ' => fun i => if h : (⟨ℓ, i⟩ : Member L mem) = m then Δ' ⟨i, h⟩ else Δ i
  | .atomic ℓ .., Δ, Δ' => fun i => if h : (⟨ℓ, i⟩ : Member L mem) = m then Δ' ⟨i, h⟩ else Δ i
  | .atomicStream ℓ .., Δ, Δ' => fun i => if h : (⟨ℓ, i⟩ : Member L mem) = m then Δ' ⟨i, h⟩ else Δ i
  | .atomicKeyed ℓ .., Δ, Δ' => fun i => if h : (⟨ℓ, i⟩ : Member L mem) = m then Δ' ⟨i, h⟩ else Δ i
  | .atomicKSing ℓ .., Δ, Δ' => fun i => if h : (⟨ℓ, i⟩ : Member L mem) = m then Δ' ⟨i, h⟩ else Δ i

/-- Every other member's increment is empty — for the list-like kinds and the
atomic slots (`none`, D85); a singleton's increment is its current value and
says nothing. -/
def idleDelta (m : Member L mem) : (κ : Kind L mem) → κ.delta → Prop
  | .stream ℓ .., Δ => ∀ i, (⟨ℓ, i⟩ : Member L mem) ≠ m → Δ i = []
  | .keyed ℓ .., Δ => ∀ i, (⟨ℓ, i⟩ : Member L mem) ≠ m → Δ i = []
  | .sing .., _ => True
  | .ksing .., _ => True
  | .atomic ℓ .., Δ => ∀ i, (⟨ℓ, i⟩ : Member L mem) ≠ m → Δ i = none
  | .atomicStream ℓ .., Δ => ∀ i, (⟨ℓ, i⟩ : Member L mem) ≠ m → Δ i = none
  | .atomicKeyed ℓ .., Δ => ∀ i, (⟨ℓ, i⟩ : Member L mem) ≠ m → Δ i = none
  | .atomicKSing ℓ .., Δ => ∀ i, (⟨ℓ, i⟩ : Member L mem) ≠ m → Δ i = none

/-- A list-like or slot increment that is empty everywhere (singletons: no constraint). -/
def allEmpty : (κ : Kind L mem) → κ.delta → Prop
  | .stream .., Δ => ∀ i, Δ i = []
  | .keyed .., Δ => ∀ i, Δ i = []
  | .sing .., _ => True
  | .ksing .., _ => True
  | .atomic .., Δ => ∀ i, Δ i = none
  | .atomicStream .., Δ => ∀ i, Δ i = none
  | .atomicKeyed .., Δ => ∀ i, Δ i = none
  | .atomicKSing .., Δ => ∀ i, Δ i = none

end Kind

/-- Cut an increment environment to a process. -/
def envRestrict (m : Member L mem) {Γ : Ctx L mem} (Δ : HList (Kind.delta (mem := mem)) Γ) :
    HList (Kind.locDelta m) Γ :=
  Δ.map (fun {κ} Δ => Kind.restrictDelta m κ Δ)

/-- Write a process's increments back over a global environment. -/
def envOverride (m : Member L mem) : {Γ : Ctx L mem} → HList (Kind.delta (mem := mem)) Γ →
    HList (Kind.locDelta m) Γ → HList (Kind.delta (mem := mem)) Γ
  | [], _, _ => HList.nil
  | _ :: _, Δ, Δ' => (Kind.overrideDelta m _ Δ.1 Δ'.1, envOverride m Δ.2 Δ'.2)

/-- Every other member's list-like increment is empty. -/
def IdleEnv (m : Member L mem) : {Γ : Ctx L mem} → HList (Kind.delta (mem := mem)) Γ → Prop
  | [], _ => True
  | _ :: _, Δ => Kind.idleDelta m _ Δ.1 ∧ IdleEnv m Δ.2

/-- Every list-like increment is empty. -/
def EmptyEnv : {Γ : Ctx L mem} → HList (Kind.delta (mem := mem)) Γ → Prop
  | [], _ => True
  | _ :: _, Δ => Kind.allEmpty _ Δ.1 ∧ EmptyEnv Δ.2

/-! ## The process's cells, oracle, sends -/

mutual

/-- The member's cells of a node (`Node.State` cut to `Hosted m`). -/
def Node.LocState (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → Node Γ κs → Type
  | _, _, .broadcast_closed (c := c) (p := p) (α := α) _ _ => Hosted m p → Fin (mem c) → ChanCell α
  | _, _, .demux (c := c) (p := p) (α := α) _ _ _ => Hosted m p → Fin (mem c) → ChanCell α
  | _, _, .kfold_early_stop (ℓ := ℓ) (K := K) (A := A) _ _ _ _ _ _ => Hosted m ℓ → KFESCell K A
  | _, _, .fold (ℓ := ℓ) (σ := σ) _ _ _ _ _ _ _ => Hosted m ℓ → σ
  | _, _, .fold_monotone (ℓ := ℓ) (σ := σ) _ _ _ _ _ _ _ => Hosted m ℓ → σ
  | _, _, .batch (ℓ := ℓ) (α := α) _ _ _ _ _ => Hosted m ℓ → List α
  | _, _, .source_interval (ℓ := ℓ) _ _ => Hosted m ℓ → Nat
  | _, _, .kfold (ℓ := ℓ) (K := K) (A := A) _ _ _ _ _ _ _ _ _ _ => Hosted m ℓ → SchedKMap K A
  | _, _, .tfold (ℓ := ℓ) (σ := σ) _ _ _ _ _ _ _ => Hosted m ℓ → σ
  | _, _, .latest (ℓ := ℓ) (σ := σ) _ _ => Hosted m ℓ → Option σ
  | _, _, .latest_monotone (ℓ := ℓ) _ => Hosted m ℓ → Option Nat
  | _, _, .time (ℓ := ℓ) => Hosted m ℓ → Nat
  | _, _, .snapshot_bounded (ℓ := ℓ) _ _ => Hosted m ℓ → Nat
  | _, _, .zipTick (ℓ := ℓ) (α := α) (β := β) _ _ => Hosted m ℓ → List α × List β
  | _, _, .defer_tick (ℓ := ℓ) (σ := σ) _ _ => Hosted m ℓ → List σ
  | _, _, .reduce_watermark (ℓ := ℓ) (K := K) (V := V) _ _ _ _ _ _ _ _ _ _ =>
    Hosted m ℓ → (List (List (K × V)) × List (Option K)) × (SchedKMap K V × Option K)
  | _, _, .tick_scan (ℓ := ℓ) sts ins _ _ _ _ => Hosted m ℓ → PendingOf ins × BoundedOf BS BStr BKS BKStr sts
  | _, _, .call N _ => Dag.LocState m N
  | _, _, .fixN ks body => Dag.LocState m body × HList (Kind.locDelta m) (ks.map KnotKind.toKind)
  | _, _, _ => Unit

def Dag.LocState (m : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → Dag Γ outs → Type
  | _, _, .ret _ => Unit
  | _, _, .letN n rest => Node.LocState m n × Dag.LocState m rest

end

mutual

/-- What a process reads from its execution this step: at its receiving ends, per
sender, how many of the sender's items are delivered by now (network nodes); per
hosted member, **this tick's** timing — the clock reading, the interval pulse (timer
nodes; D84). A process never holds a
timing history: the lock-step machine's script is indexed by the node's tick count
(`timingAt`), the shell reads its clock and timers once per activation. -/
def Node.Oracle (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → Node Γ κs → Type
  | _, _, .broadcast_closed (c := c) (p := p) _ _ => Hosted m p → Fin (mem c) → Nat
  | _, _, .demux (c := c) (p := p) _ _ _ => Hosted m p → Fin (mem c) → Nat
  | _, _, .source_interval (ℓ := ℓ) _ _ => Hosted m ℓ → Nat
  | _, _, .time (ℓ := ℓ) => Hosted m ℓ → Nat
  | _, _, .call N _ => Dag.Oracle m N
  | _, _, .fixN _ body => Dag.Oracle m body
  | _, _, _ => Unit

def Dag.Oracle (m : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → Dag Γ outs → Type
  | _, _, .ret _ => Unit
  | _, _, .letN n rest => Node.Oracle m n × Dag.Oracle m rest

end

mutual

/-- What the member puts on a network node's wire this step
(the receiver-side filtering of a `demux` happens at the append). -/
def Node.Sends (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → Node Γ κs → Type
  | _, _, .broadcast_closed (c := c) (α := α) _ _ => Hosted m c → List α
  | _, _, .demux (c := c) (α := α) _ _ _ => Hosted m c → List (Nat × α)
  | _, _, .call N _ => Dag.Sends m N
  | _, _, .fixN _ body => Dag.Sends m body
  | _, _, _ => Unit

def Dag.Sends (m : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → Dag Γ outs → Type
  | _, _, .ret _ => Unit
  | _, _, .letN n rest => Node.Sends m n × Dag.Sends m rest

end

/-! ## Cutting and writing back -/

mutual

/-- The global cells cut to the process. -/
def Node.restrict (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    Node.State n → Node.LocState m n
  | _, _, .broadcast_closed .., s => fun i => s i.1
  | _, _, .demux .., s => fun i => s i.1
  | _, _, .kfold_early_stop .., s => fun i => s i.1
  | _, _, .fold .., s => fun i => s i.1
  | _, _, .fold_monotone .., s => fun i => s i.1
  | _, _, .batch .., s => fun i => s i.1
  | _, _, .source_interval .., s => fun i => s i.1
  | _, _, .kfold .., s => fun i => s i.1
  | _, _, .tfold .., s => fun i => s i.1
  | _, _, .latest .., s => fun i => s i.1
  | _, _, .latest_monotone .., s => fun i => s i.1
  | _, _, .time .., s => fun i => s i.1
  | _, _, .snapshot_bounded .., s => fun i => s i.1
  | _, _, .zipTick .., s => fun i => s i.1
  | _, _, .defer_tick .., s => fun i => s i.1
  | _, _, .reduce_watermark .., s => fun i => s i.1
  | _, _, .tick_scan .., s => fun i => s i.1
  | _, _, .call N _, s => Dag.restrict m N s
  | _, _, .fixN _ body, s => (Dag.restrict m body s.1, envRestrict m s.2)
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
  | _, _, .firstTick .., _ => ()
  | _, _, .allTicks .., _ => ()
  | _, _, .flattenOrdered .., _ => ()
  | _, _, .flattenUnordered .., _ => ()
  | _, _, .tinto_keyed .., _ => ()
  | _, _, .tassume_ordering .., _ => ()
  | _, _, .kentries .., _ => ()
  | _, _, .kmap .., _ => ()
  | _, _, .kallTicks .., _ => ()
  | _, _, .ksnapshot .., _ => ()

def Dag.restrict (m : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    Dag.State d → Dag.LocState m d
  | _, _, .ret _, _ => ()
  | _, _, .letN n rest, s => (Node.restrict m n s.1, Dag.restrict m rest s.2)

end

mutual

/-- The process's cells written back over the global ones. -/
def Node.override (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    Node.State n → Node.LocState m n → Node.State n
  | _, _, .broadcast_closed (p := p) .., s, l => fun i => if h : (⟨p, i⟩ : Member L mem) = m then l ⟨i, h⟩ else s i
  | _, _, .demux (p := p) .., s, l => fun i => if h : (⟨p, i⟩ : Member L mem) = m then l ⟨i, h⟩ else s i
  | _, _, .kfold_early_stop (ℓ := ℓ) .., s, l => fun i => if h : (⟨ℓ, i⟩ : Member L mem) = m then l ⟨i, h⟩ else s i
  | _, _, .fold (ℓ := ℓ) .., s, l => fun i => if h : (⟨ℓ, i⟩ : Member L mem) = m then l ⟨i, h⟩ else s i
  | _, _, .fold_monotone (ℓ := ℓ) .., s, l => fun i => if h : (⟨ℓ, i⟩ : Member L mem) = m then l ⟨i, h⟩ else s i
  | _, _, .batch (ℓ := ℓ) .., s, l => fun i => if h : (⟨ℓ, i⟩ : Member L mem) = m then l ⟨i, h⟩ else s i
  | _, _, .source_interval (ℓ := ℓ) _ _ .., s, l => fun i => if h : (⟨ℓ, i⟩ : Member L mem) = m then l ⟨i, h⟩ else s i
  | _, _, .kfold (ℓ := ℓ) .., s, l => fun i => if h : (⟨ℓ, i⟩ : Member L mem) = m then l ⟨i, h⟩ else s i
  | _, _, .tfold (ℓ := ℓ) .., s, l => fun i => if h : (⟨ℓ, i⟩ : Member L mem) = m then l ⟨i, h⟩ else s i
  | _, _, .latest (ℓ := ℓ) .., s, l => fun i => if h : (⟨ℓ, i⟩ : Member L mem) = m then l ⟨i, h⟩ else s i
  | _, _, .latest_monotone (ℓ := ℓ) .., s, l => fun i => if h : (⟨ℓ, i⟩ : Member L mem) = m then l ⟨i, h⟩ else s i
  | _, _, .time (ℓ := ℓ) .., s, l => fun i => if h : (⟨ℓ, i⟩ : Member L mem) = m then l ⟨i, h⟩ else s i
  | _, _, .snapshot_bounded (ℓ := ℓ) .., s, l => fun i => if h : (⟨ℓ, i⟩ : Member L mem) = m then l ⟨i, h⟩ else s i
  | _, _, .zipTick (ℓ := ℓ) .., s, l => fun i => if h : (⟨ℓ, i⟩ : Member L mem) = m then l ⟨i, h⟩ else s i
  | _, _, .defer_tick (ℓ := ℓ) .., s, l => fun i => if h : (⟨ℓ, i⟩ : Member L mem) = m then l ⟨i, h⟩ else s i
  | _, _, .reduce_watermark (ℓ := ℓ) .., s, l => fun i => if h : (⟨ℓ, i⟩ : Member L mem) = m then l ⟨i, h⟩ else s i
  | _, _, .tick_scan (ℓ := ℓ) .., s, l => fun i => if h : (⟨ℓ, i⟩ : Member L mem) = m then l ⟨i, h⟩ else s i
  | _, _, .call N _, s, l => Dag.override m N s l
  | _, _, .fixN _ body, s, l => (Dag.override m body s.1 l.1, envOverride m s.2 l.2)
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
  | _, _, .firstTick .., s, _ => s
  | _, _, .allTicks .., s, _ => s
  | _, _, .flattenOrdered .., s, _ => s
  | _, _, .flattenUnordered .., s, _ => s
  | _, _, .tinto_keyed .., s, _ => s
  | _, _, .tassume_ordering .., s, _ => s
  | _, _, .kentries .., s, _ => s
  | _, _, .kmap .., s, _ => s
  | _, _, .kallTicks .., s, _ => s
  | _, _, .ksnapshot .., s, _ => s

def Dag.override (m : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    Dag.State d → Dag.LocState m d → Dag.State d
  | _, _, .ret _, _, _ => ()
  | _, _, .letN n rest, s, l => (Node.override m n s.1 l.1, Dag.override m rest s.2 l.2)

end

mutual

/-- The active process's sends appended to every **other** receiver's
queue (its own receivers get them by `enqueueLoc m m` — the model's network
queue for the loopback connection). -/
def Node.enqueue (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    Node.Sends m n → Node.State n → Node.State n
  | _, _, .broadcast_closed (c := c) (p := p) .., snd, s => fun i j =>
    if (⟨p, i⟩ : Member L mem) = m then s i j
    else if h : (⟨c, j⟩ : Member L mem) = m then (s i j).enqueue (snd ⟨j, h⟩) else s i j
  | _, _, .demux (c := c) (p := p) _ _ addr, snd, s => fun i j =>
    if (⟨p, i⟩ : Member L mem) = m then s i j
    else if h : (⟨c, j⟩ : Member L mem) = m then
      (s i j).enqueue ((snd ⟨j, h⟩).filterMap (fun dx => if dx.1 = addr i then some dx.2 else none))
    else s i j
  | _, _, .call N _, snd, s => Dag.enqueue m N snd s
  | _, _, .fixN _ body, snd, s => (Dag.enqueue m body snd s.1, s.2)
  | _, _, .kfold_early_stop .., _, s => s
  | _, _, .fold .., _, s => s
  | _, _, .fold_monotone .., _, s => s
  | _, _, .batch .., _, s => s
  | _, _, .source_interval .., _, s => s
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
  | _, _, .firstTick .., _, s => s
  | _, _, .allTicks .., _, s => s
  | _, _, .flattenOrdered .., _, s => s
  | _, _, .flattenUnordered .., _, s => s
  | _, _, .tinto_keyed .., _, s => s
  | _, _, .tassume_ordering .., _, s => s
  | _, _, .kentries .., _, s => s
  | _, _, .kmap .., _, s => s
  | _, _, .kallTicks .., _, s => s
  | _, _, .kfold .., _, s => s
  | _, _, .ksnapshot .., _, s => s
  | _, _, .tfold .., _, s => s
  | _, _, .latest .., _, s => s
  | _, _, .latest_monotone .., _, s => s
  | _, _, .time .., _, s => s
  | _, _, .snapshot_bounded .., _, s => s

def Dag.enqueue (m : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    Dag.Sends m d → Dag.State d → Dag.State d
  | _, _, .ret _, _, _ => ()
  | _, _, .letN n rest, snd, s => (Node.enqueue m n snd.1 s.1, Dag.enqueue m rest snd.2 s.2)

end

mutual

/-- The same append, landing in another process's cells (its receiving ends). -/
def Node.enqueueLoc (m : Member L mem) (src : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} →
    (n : Node Γ κs) → Node.Sends src n → Node.LocState m n → Node.LocState m n
  | _, _, .broadcast_closed (c := c) .., snd, s => fun i j =>
    if h : (⟨c, j⟩ : Member L mem) = src then (s i j).enqueue (snd ⟨j, h⟩) else s i j
  | _, _, .demux (c := c) _ _ addr, snd, s => fun i j =>
    if h : (⟨c, j⟩ : Member L mem) = src then
      (s i j).enqueue ((snd ⟨j, h⟩).filterMap (fun dx => if dx.1 = addr i.1 then some dx.2 else none))
    else s i j
  | _, _, .call N _, snd, s => Dag.enqueueLoc m src N snd s
  | _, _, .fixN _ body, snd, s => (Dag.enqueueLoc m src body snd s.1, s.2)
  | _, _, .kfold_early_stop .., _, s => s
  | _, _, .fold .., _, s => s
  | _, _, .fold_monotone .., _, s => s
  | _, _, .batch .., _, s => s
  | _, _, .source_interval .., _, s => s
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
  | _, _, .firstTick .., _, s => s
  | _, _, .allTicks .., _, s => s
  | _, _, .flattenOrdered .., _, s => s
  | _, _, .flattenUnordered .., _, s => s
  | _, _, .tinto_keyed .., _, s => s
  | _, _, .tassume_ordering .., _, s => s
  | _, _, .kentries .., _, s => s
  | _, _, .kmap .., _, s => s
  | _, _, .kallTicks .., _, s => s
  | _, _, .kfold .., _, s => s
  | _, _, .ksnapshot .., _, s => s
  | _, _, .tfold .., _, s => s
  | _, _, .latest .., _, s => s
  | _, _, .latest_monotone .., _, s => s
  | _, _, .time .., _, s => s
  | _, _, .snapshot_bounded .., _, s => s

def Dag.enqueueLoc (m : Member L mem) (src : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} →
    (d : Dag Γ outs) → Dag.Sends src d → Dag.LocState m d → Dag.LocState m d
  | _, _, .ret _, _, _ => ()
  | _, _, .letN n rest, snd, s => (Node.enqueueLoc m src n snd.1 s.1, Dag.enqueueLoc m src rest snd.2 s.2)

end

end Decidable

/-! ## Materializing a step's per-member values

A node's increments and cells are per-member functions; left as the
closures `locStep` builds, every consumer would recompute its producer
(call-by-name over a graph with fan-out — the `n^depth` unsharing D73 met
at `Values`). `memo`/`memo2` are the identity in the logic and build a
value in compiled code (`@[csimp]`, the one sanctioned seam: the equation
is kernel-checked). `Dag.locStep` materializes each node's cells and
increments before the rest of the graph reads them. The materialized form is
one `Option` per node (`hostedSelf`): a process is one member, so a
hosted-member function has at most one value to hold.

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
structure MemoBox (m : Member L mem) (ℓ : L) (X : Type) where
  fn : Hosted m ℓ → X
  tag : Unit := ()

/-- The identity box (materialized when compiled, `memoBox_eq_memoBoxImpl`). -/
@[reducible] def memoBox [DecidableEq L] {m : Member L mem} {ℓ : L} {X : Type} (f : Hosted m ℓ → X) : MemoBox m ℓ X := ⟨f, ()⟩

/-- The member of `ℓ` the process hosts, if any: `Hosted m ℓ` has at most one inhabitant
(`m` itself when `m` lives at `ℓ`), so a hosted-member function is one optional value
(D86 — the earlier table was an array over `mem ℓ` with a member test per entry, rebuilt
for every node on every activation). -/
def hostedSelf [DecidableEq L] : (m : Member L mem) → (ℓ : L) → Option (Hosted m ℓ)
  | ⟨ℓ', i⟩, ℓ => if h : ℓ' = ℓ then some ⟨h ▸ i, by subst h; rfl⟩ else none

theorem hostedSelf_eq [DecidableEq L] {m : Member L mem} {ℓ : L} (i : Hosted m ℓ) : hostedSelf m ℓ = some i := by
  obtain ⟨ℓ', j⟩ := m
  obtain ⟨i, hi⟩ := i
  have h : ℓ = ℓ' := (Sigma.mk.inj_iff.mp hi).1
  subst h
  have hj : i = j := eq_of_heq (Sigma.mk.inj_iff.mp hi).2
  subst hj
  simp [hostedSelf]

/-- Read a materialized hosted value. The value is present at every hosted member (`hs`), so
the stored closure captures the value **and nothing else** — in particular not the function
it tabulates, whose free variables are the previous cells and increments (D84: capturing it
retained every past state of every cell). `@[noinline]` so the partial application is the
stored closure. -/
@[noinline] def lookupHosted {m : Member L mem} {ℓ : L} {X : Type}
    (o : Option X) (hs : ∀ _ : Hosted m ℓ, o.isSome) : Hosted m ℓ → X :=
  fun i => o.get (hs i)

def memoBoxImpl [DecidableEq L] {m : Member L mem} {ℓ : L} {X : Type} (f : Hosted m ℓ → X) : MemoBox m ℓ X :=
  ⟨lookupHosted ((hostedSelf m ℓ).map f) (fun i => by rw [hostedSelf_eq i]; rfl), ()⟩

theorem lookupHosted_hostedSelf [DecidableEq L] {m : Member L mem} {ℓ : L} {X : Type} (f : Hosted m ℓ → X)
    (hs : ∀ _ : Hosted m ℓ, ((hostedSelf m ℓ).map f).isSome) (i : Hosted m ℓ) :
    lookupHosted ((hostedSelf m ℓ).map f) hs i = f i := by
  unfold lookupHosted
  apply Option.some_inj.mp
  rw [Option.some_get, hostedSelf_eq i]
  rfl

@[csimp] theorem memoBox_eq_memoBoxImpl : @memoBox = @memoBoxImpl := by
  funext L mem _ m ℓ X f
  simp only [memoBox, memoBoxImpl, MemoBox.mk.injEq, and_true]
  funext i
  rw [lookupHosted_hostedSelf]

/-- The identity on hosted-member functions (materialized when compiled). -/
@[macro_inline, reducible] def memo [DecidableEq L] {m : Member L mem} {ℓ : L} {X : Type} (f : Hosted m ℓ → X) : Hosted m ℓ → X :=
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

/-- The identity on two-level member functions (network cells: receiver
× sender). The inner table is built as the optional value's *content*: a
lambda `fun i => (memoFinBox (s i)).fn` would be eta-expanded by the compiler and
rebuild the inner table on every call. -/
@[reducible] def memoBox2 [DecidableEq L] {m : Member L mem} {ℓ : L} {n : Nat} {X : Type} (f : Hosted m ℓ → Fin n → X) :
    MemoBox m ℓ (Fin n → X) := ⟨f, ()⟩

/-- Read the hosted member's inner box and its member function (`@[noinline]`, as
`lookupHosted`). -/
@[noinline] def lookupHosted2 {m : Member L mem} {ℓ : L} {n : Nat} {X : Type}
    (o : Option (MemoFinBox n X)) (hs : ∀ _ : Hosted m ℓ, o.isSome) : Hosted m ℓ → Fin n → X :=
  fun i => (o.get (hs i)).fn

theorem lookupHosted2_hostedSelf [DecidableEq L] {m : Member L mem} {ℓ : L} {n : Nat} {X : Type}
    (f : Hosted m ℓ → MemoFinBox n X)
    (hs : ∀ _ : Hosted m ℓ, ((hostedSelf m ℓ).map f).isSome) (i : Hosted m ℓ) :
    lookupHosted2 ((hostedSelf m ℓ).map f) hs i = (f i).fn := by
  unfold lookupHosted2
  congr 1
  apply Option.some_inj.mp
  rw [Option.some_get, hostedSelf_eq i]
  rfl

def memoBox2Impl [DecidableEq L] {m : Member L mem} {ℓ : L} {n : Nat} {X : Type} (f : Hosted m ℓ → Fin n → X) :
    MemoBox m ℓ (Fin n → X) :=
  ⟨lookupHosted2 ((hostedSelf m ℓ).map fun i => memoFinBoxImpl (f i))
    (fun i => by rw [hostedSelf_eq i]; rfl), ()⟩

@[csimp] theorem memoBox2_eq_memoBox2Impl : @memoBox2 = @memoBox2Impl := by
  funext L mem _ m ℓ n X f
  simp only [memoBox2, memoBox2Impl, MemoBox.mk.injEq, and_true]
  funext i j
  rw [lookupHosted2_hostedSelf]
  simp only [memoFinBoxImpl, lookupFin, Array.getElem_ofFn, Fin.eta]

variable [DecidableEq L]

@[macro_inline, reducible] def memo2 {m : Member L mem} {ℓ : L} {n : Nat} {X : Type}
    (f : Hosted m ℓ → Fin n → X) : Hosted m ℓ → Fin n → X :=
  (memoBox2 f).fn

/-- `memo` is the identity (the proofs' reading of a materialized value). -/
@[simp] theorem memo_apply {m : Member L mem} {ℓ : L} {X : Type} (f : Hosted m ℓ → X) (i : Hosted m ℓ) :
    memo f i = f i := rfl

@[simp] theorem memo2_apply {m : Member L mem} {ℓ : L} {n : Nat} {X : Type} (f : Hosted m ℓ → Fin n → X)
    (i : Hosted m ℓ) (j : Fin n) : memo2 f i j = f i j := rfl

/-- Materialize every increment of a process's environment. -/
def memoEnv (m : Member L mem) : {Γ : Ctx L mem} → HList (Kind.locDelta m) Γ → HList (Kind.locDelta m) Γ
  | [], _ => HList.nil
  | .stream .. :: _, Δ => (memo Δ.1, memoEnv m Δ.2)
  | .keyed .. :: _, Δ => (memo Δ.1, memoEnv m Δ.2)
  | .sing .. :: _, Δ => (memo Δ.1, memoEnv m Δ.2)
  | .ksing .. :: _, Δ => (memo Δ.1, memoEnv m Δ.2)
  | .atomic .. :: _, Δ => (memo Δ.1, memoEnv m Δ.2)
  | .atomicStream .. :: _, Δ => (memo Δ.1, memoEnv m Δ.2)
  | .atomicKeyed .. :: _, Δ => (memo Δ.1, memoEnv m Δ.2)
  | .atomicKSing .. :: _, Δ => (memo Δ.1, memoEnv m Δ.2)

theorem memoEnv_eq (m : Member L mem) : {Γ : Ctx L mem} → (Δ : HList (Kind.locDelta m) Γ) → memoEnv m Δ = Δ
  | [], _ => rfl
  | .stream .. :: _, Δ => by show (Δ.1, memoEnv m Δ.2) = Δ; rw [memoEnv_eq m Δ.2]
  | .keyed .. :: _, Δ => by show (Δ.1, memoEnv m Δ.2) = Δ; rw [memoEnv_eq m Δ.2]
  | .sing .. :: _, Δ => by show (Δ.1, memoEnv m Δ.2) = Δ; rw [memoEnv_eq m Δ.2]
  | .ksing .. :: _, Δ => by show (Δ.1, memoEnv m Δ.2) = Δ; rw [memoEnv_eq m Δ.2]
  | .atomic .. :: _, Δ => by show (Δ.1, memoEnv m Δ.2) = Δ; rw [memoEnv_eq m Δ.2]
  | .atomicStream .. :: _, Δ => by show (Δ.1, memoEnv m Δ.2) = Δ; rw [memoEnv_eq m Δ.2]
  | .atomicKeyed .. :: _, Δ => by show (Δ.1, memoEnv m Δ.2) = Δ; rw [memoEnv_eq m Δ.2]
  | .atomicKSing .. :: _, Δ => by show (Δ.1, memoEnv m Δ.2) = Δ; rw [memoEnv_eq m Δ.2]

/-- A node's cells, boxed. The match arms of `Node.memoStateBox` return this
structure rather than the cells themselves: an arm whose type reduces to a
function type gets eta-expanded by the compiler, which would move the table
construction into the returned closure. -/
structure StateBox (m : Member L mem) {Γ : Ctx L mem} {κs : List (Kind L mem)} (n : Node Γ κs) where
  val : Node.LocState m n
  tag : Unit := ()

mutual

/-- Materialize a node's cells (boxed). -/
def Node.memoStateBox (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    Node.LocState m n → StateBox m n
  | _, _, .broadcast_closed .., s => ⟨memo2 s, ()⟩
  | _, _, .demux .., s => ⟨memo2 s, ()⟩
  | _, _, .kfold_early_stop .., s => ⟨memo s, ()⟩
  | _, _, .fold .., s => ⟨memo s, ()⟩
  | _, _, .fold_monotone .., s => ⟨memo s, ()⟩
  | _, _, .batch .., s => ⟨memo s, ()⟩
  | _, _, .source_interval .., s => ⟨memo s, ()⟩
  | _, _, .zipTick .., s => ⟨memo s, ()⟩
  | _, _, .defer_tick .., s => ⟨memo s, ()⟩
  | _, _, .reduce_watermark .., s => ⟨memo s, ()⟩
  | _, _, .tick_scan .., s => ⟨memo s, ()⟩
  | _, _, .call N _, s => ⟨Dag.memoState m N s, ()⟩
  | _, _, .fixN _ body, s => ⟨(Dag.memoState m body s.1, memoEnv m s.2), ()⟩
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
  | _, _, .firstTick .., s => ⟨s, ()⟩
  | _, _, .allTicks .., s => ⟨s, ()⟩
  | _, _, .flattenOrdered .., s => ⟨s, ()⟩
  | _, _, .flattenUnordered .., s => ⟨s, ()⟩
  | _, _, .tinto_keyed .., s => ⟨s, ()⟩
  | _, _, .tassume_ordering .., s => ⟨s, ()⟩
  | _, _, .kentries .., s => ⟨s, ()⟩
  | _, _, .kmap .., s => ⟨s, ()⟩
  | _, _, .kallTicks .., s => ⟨s, ()⟩
  | _, _, .ksnapshot .., s => ⟨s, ()⟩
  | _, _, .kfold .., s => ⟨memo s, ()⟩
  | _, _, .tfold .., s => ⟨memo s, ()⟩
  | _, _, .latest .., s => ⟨memo s, ()⟩
  | _, _, .latest_monotone .., s => ⟨memo s, ()⟩
  | _, _, .time .., s => ⟨memo s, ()⟩
  | _, _, .snapshot_bounded .., s => ⟨memo s, ()⟩

/-- Materialize a graph's cells. -/
def Dag.memoState (m : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    Dag.LocState m d → Dag.LocState m d
  | _, _, .ret _, s => s
  | _, _, .letN n rest, s => ((Node.memoStateBox m n s.1).val, Dag.memoState m rest s.2)

end

/-- Materialize a node's cells. -/
@[reducible] def Node.memoState (m : Member L mem) {Γ : Ctx L mem} {κs : List (Kind L mem)} (n : Node Γ κs)
    (s : Node.LocState m n) : Node.LocState m n :=
  (Node.memoStateBox m n s).val

mutual

theorem Node.memoState_eq (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    (s : Node.LocState m n) → Node.memoState m n s = s
  | _, _, .broadcast_closed .., _ => rfl
  | _, _, .demux .., _ => rfl
  | _, _, .kfold_early_stop .., _ => rfl
  | _, _, .fold .., _ => rfl
  | _, _, .fold_monotone .., _ => rfl
  | _, _, .batch .., _ => rfl
  | _, _, .source_interval .., _ => rfl
  | _, _, .zipTick .., _ => rfl
  | _, _, .defer_tick .., _ => rfl
  | _, _, .reduce_watermark .., _ => rfl
  | _, _, .tick_scan .., _ => rfl
  | _, _, .call N _, s => Dag.memoState_eq m N s
  | _, _, .fixN _ body, s => by
    show (Dag.memoState m body s.1, memoEnv m s.2) = s
    rw [Dag.memoState_eq m body s.1, memoEnv_eq]
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
  | _, _, .firstTick .., _ => rfl
  | _, _, .allTicks .., _ => rfl
  | _, _, .flattenOrdered .., _ => rfl
  | _, _, .flattenUnordered .., _ => rfl
  | _, _, .tinto_keyed .., _ => rfl
  | _, _, .tassume_ordering .., _ => rfl
  | _, _, .kentries .., _ => rfl
  | _, _, .kmap .., _ => rfl
  | _, _, .kallTicks .., _ => rfl
  | _, _, .ksnapshot .., _ => rfl
  | _, _, .kfold .., _ => rfl
  | _, _, .tfold .., _ => rfl
  | _, _, .latest .., _ => rfl
  | _, _, .latest_monotone .., _ => rfl
  | _, _, .time .., _ => rfl
  | _, _, .snapshot_bounded .., _ => rfl

theorem Dag.memoState_eq (m : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    (s : Dag.LocState m d) → Dag.memoState m d s = s
  | _, _, .ret _, _ => rfl
  | _, _, .letN n rest, s => by
    show (Node.memoState m n s.1, Dag.memoState m rest s.2) = s
    rw [Node.memoState_eq m n s.1, Dag.memoState_eq m rest s.2]
    rfl

end

/-! ## The process's step -/

/-- The hosted part of a shaped tick input's increments. -/
def readShapeLoc (m : Member L mem) {Γ : Ctx L mem} (Δ : HList (Kind.locDelta m) Γ) (ℓ : L) :
    (sh : TickShape) → VarsOf Γ ℓ sh → HList (Kind.locDelta m) (shapeKinds ℓ sh)
  | .sing _, v => HList.cons (Δ.get v) HList.nil
  | .stream _ _ _ _, v => HList.cons (Δ.get v) HList.nil
  | .ksing _ _ _ _, v => HList.cons (Δ.get v) HList.nil
  | .kstream _ _ _ _ _ _, v => HList.cons (Δ.get v) HList.nil
  | .pair a b, (va, vb) => (readShapeLoc m Δ ℓ a va).append (readShapeLoc m Δ ℓ b vb)

/-- Push the member's leaf increments (a per-member view of `PendingOf.push`). -/
def PendingOf.pushLoc (m : Member L mem) {ℓ : L} : (sh : TickShape) → PendingOf sh →
    (Hosted m ℓ → HList (Kind.locDelta m) (shapeKinds ℓ sh)) → Hosted m ℓ → PendingOf sh
  | .sing _, p, Δ, i => p ++ ((Δ i).head i).toList
  | .stream _ _ _ _, p, Δ, i => p ++ ((Δ i).head i).toList
  | .ksing _ _ _ _, p, Δ, i => p ++ ((Δ i).head i).toList
  | .kstream _ _ _ _ _ _, p, Δ, i => p ++ ((Δ i).head i).toList
  | .pair a b, (pa, pb), Δ, i =>
    (pushLoc m a pa (fun i => (Δ i).take) i, pushLoc m b pb (fun i => (Δ i).drop) i)

/-- Unzip the member's shaped outputs into per-leaf increments. -/
def unzipShapeLoc (m : Member L mem) (ℓ : L) : (sh : TickShape) →
    (Hosted m ℓ → Option (BoundedOf BS BStr BKS BKStr sh)) → HList (Kind.locDelta m) (shapeKinds ℓ sh)
  | .sing _, l => HList.cons (fun i => l i) HList.nil
  | .stream _ _ _ _, l => HList.cons (fun i => l i) HList.nil
  | .ksing _ _ _ _, l => HList.cons (fun i => l i) HList.nil
  | .kstream _ _ _ _ _ _, l => HList.cons (fun i => l i) HList.nil
  | .pair a b, l =>
    (unzipShapeLoc m ℓ a (fun i => (l i).map Prod.fst)).append (unzipShapeLoc m ℓ b (fun i => (l i).map Prod.snd))

mutual

/-- **One step of a process**: the member's cells and increments
only. The process is *active*: deliveries land (a network node delivers up
to the oracle's cursor), knot feedback propagates, stream operators fire;
its tick bodies run iff `tk` (a step with `tk = false` is a **stutter** —
the asynchronous dataflow progresses between ticks; a DFIR tick is a tick,
then stutters while a knot slot holds feedback — D93, `Dag.iterate`). At
runtime a process is one cluster member and
`tk` ticks every `Tick` location of it at once. A network node takes the
its own sends into its own queues (`ChanCell.step`: deliver, then
enqueue), reporting them as `Sends` for the other processes' queues. -/
def Node.locStep (m : Member L mem) (tk : Bool) :
    {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) → Node.Oracle m n →
    HList (Kind.locDelta m) Γ → Node.LocState m n →
    Node.LocState m n × HList (Kind.locDelta m) κs × Node.Sends m n
  | _, _, .map _ _ x f, _, Δ, s => (s, HList.cons (fun i => (Δ.get x i).map (f i.1)) HList.nil, ())
  | _, _, .filterMap _ _ x f, _, Δ, s => (s, HList.cons (fun i => (Δ.get x i).filterMap (f i.1)) HList.nil, ())
  -- the network: deliver per the oracle; this step's sends go out (`Sends`) and come
  -- back through the network like everyone else's — a process keeps nothing in flight
  | _, _, .broadcast_closed (c := c) _ x, ora, Δ, s =>
    -- `memo`/`memo2`: the identity, materialized when compiled — a value the cells and
    -- the increments both read is computed once per member (D86)
    let cells := memo2 fun i j => (s i j).step (ora i j) []
    (fun i j => (cells i j).1,
     HList.cons (fun i => (List.finRange (mem c)).flatMap (fun j => ((cells i j).2).map (fun v => (j, v)))) HList.nil,
     fun j => Δ.get x j)
  | _, _, .demux (c := c) _ x _, ora, Δ, s =>
    let cells := memo2 fun i j => (s i j).step (ora i j) []
    (fun i j => (cells i j).1,
     HList.cons (fun i => (List.finRange (mem c)).flatMap (fun j => ((cells i j).2).map (fun v => (j, v)))) HList.nil,
     fun j => Δ.get x j)
  | _, _, .values _ _ x, _, Δ, s => (s, HList.cons (fun i => (Δ.get x i).map Prod.snd) HList.nil, ())
  | _, _, .into_keyed (ℓ := ℓ) (K := K) (V := V) _ _ (ord := ord) x, _, Δ, s =>
    (s, HList.cons (fun i => (Δ.get x : (Kind.stream (mem := mem) ℓ (K × V) inferInstance ord .exactlyOnce).locDelta m) i) HList.nil, ())
  | _, _, .kassume_ordering (ℓ := ℓ) (K := K) (V := V) iK iV x _, _, Δ, s =>
    (s, HList.cons (fun i => (Δ.get x : (Kind.keyed (mem := mem) ℓ K V iK iV .noOrder .exactlyOnce).locDelta m) i) HList.nil, ())
  | _, _, .kfold_early_stop _ _ _ x f init, _, Δ, s =>
    let s' := memo fun i => (Δ.get x i).foldl (kfesInsert f init) (s i)
    (s', HList.cons (fun i => kfesOutMap (s' i)) HList.nil, ())
  | _, _, .kget_max_key _ _ _ _ x, _, Δ, s => (s, HList.cons (fun i => (Δ.get x i).maxEntry) HList.nil, ())
  | _, _, .weaken_retries (ℓ := ℓ) (α := α) iα (ord := ord) x, _, Δ, s =>
    (s, HList.cons (fun i => (Δ.get x : (Kind.stream (mem := mem) ℓ α iα ord .exactlyOnce).locDelta m) i) HList.nil, ())
  | _, _, .union _ a b, _, Δ, s => (s, HList.cons (fun i => Δ.get a i ++ Δ.get b i) HList.nil, ())
  | _, _, .assume_ordering (ℓ := ℓ) (α := α) iα x _, _, Δ, s =>
    (s, HList.cons (fun i => (Δ.get x : (Kind.stream (mem := mem) ℓ α iα .noOrder .exactlyOnce).locDelta m) i) HList.nil, ())
  | _, _, .fold _ g _ _ _ _ x, _, Δ, s =>
    let s' := memo fun i => (Δ.get x i).foldl g (s i)
    (s', HList.cons s' HList.nil, ())
  | _, _, .fold_monotone _ _ g _ _ _ x, _, Δ, s =>
    let s' := memo fun i => (Δ.get x i).foldl g (s i)
    (s', HList.cons s' HList.nil, ())
  | _, _, .snapshot (ℓ := ℓ) _ x _, _, Δ, s =>
    (s, HList.cons (fun i => if tk then some (Δ.get x i) else none) HList.nil, ())
  | _, _, .batch (ℓ := ℓ) _ _ _ x _, _, Δ, s =>
    let p := memo fun i => s i ++ Δ.get x i
    (fun i => if tk then [] else p i, HList.cons (fun i => if tk then some (p i) else none) HList.nil, ())
  -- the interval timer (D91): this tick's oracle value is the fire count — the `()`s that
  -- arrive at the tick; the cell counts ticks
  | _, _, .source_interval (ℓ := ℓ) _ _, d, _, s =>
    (fun i => if tk then s i + 1 else s i, HList.cons (fun i => if tk then List.replicate (d i) () else []) HList.nil, ())
  | _, _, .mapTick x f, _, Δ, s => (s, HList.cons (fun i => (Δ.get x i).map (f i.1)) HList.nil, ())
  | _, _, .firstTick _ _ x, _, Δ, s => (s, HList.cons (fun i => (Δ.get x i).map List.head?) HList.nil, ())
  | _, _, .zipTick a b, _, Δ, s =>
    let res := memo fun i =>
      let pa := (s i).1 ++ (Δ.get a i).toList
      let pb := (s i).2 ++ (Δ.get b i).toList
      let n := min pa.length pb.length
      ((pa.drop n, pb.drop n), (List.zip pa pb).head?)
    (fun i => (res i).1, HList.cons (fun i => (res i).2) HList.nil, ())
  -- DFIR's `defer_tick`: the buffer of unreleased entries; a tick releases its head
  | _, _, .defer_tick _ x, _, Δ, s =>
    let buf := memo fun i => s i ++ (Δ.get x i).toList
    (fun i => if tk then (buf i).tail else buf i,
     HList.cons (fun i => if tk then (buf i).head? else none) HList.nil, ())
  | _, _, .allTicks _ x, _, Δ, s => (s, HList.cons (fun i => (Δ.get x i).getD []) HList.nil, ())
  | _, _, .flattenOrdered (ℓ := ℓ) (β := β) _ x, _, Δ, s =>
    (s, HList.cons (fun i => (Δ.get x : (Kind.atomic (mem := mem) ℓ (List β)).locDelta m) i) HList.nil, ())
  | _, _, .flattenUnordered (ℓ := ℓ) (β := β) _ x, _, Δ, s =>
    (s, HList.cons (fun i => (Δ.get x : (Kind.atomic (mem := mem) ℓ (List β)).locDelta m) i) HList.nil, ())
  | _, _, .tinto_keyed (ℓ := ℓ) (K := K) (V := V) _ _ (ord := ord) x, _, Δ, s =>
    (s, HList.cons (fun i => (Δ.get x : (Kind.atomicStream (mem := mem) ℓ (K × V) inferInstance ord .exactlyOnce).locDelta m) i) HList.nil, ())
  | _, _, .tassume_ordering (ℓ := ℓ) (α := α) iα x _, _, Δ, s =>
    (s, HList.cons (fun i => (Δ.get x : (Kind.atomicStream (mem := mem) ℓ α iα .noOrder .exactlyOnce).locDelta m) i) HList.nil, ())
  | _, _, .reduce_watermark _ _ _ _ x wm f _ _ _, _, Δ, s =>
    let res := memo fun i =>
      let pa := (s i).1.1 ++ (Δ.get x i).toList
      let pb := (s i).1.2 ++ (Δ.get wm i).toList
      let n := min pa.length pb.length
      let run := scanRun (reduceWatermarkEagerH (K := _) f) (s i).2 (List.zip pa pb)
      (((pa.drop n, pb.drop n), run.1), run.2.head?)
    (fun i => (res i).1, HList.cons (fun i => (res i).2) HList.nil, ())
  | _, _, .kentries (ℓ := ℓ) (K := K) (V := V) iK iV (ord := ord) x, _, Δ, s =>
    (s, HList.cons (fun i => (Δ.get x : (Kind.keyed (mem := mem) ℓ K V iK iV ord .exactlyOnce).locDelta m) i) HList.nil, ())
  | _, _, .kmap _ _ _ x f, _, Δ, s =>
    (s, HList.cons (fun i => (Δ.get x i).map (fun e => (e.1, f i.1 e.2))) HList.nil, ())
  | _, _, .kallTicks _ _ x, _, Δ, s => (s, HList.cons (fun i => (Δ.get x i).getD []) HList.nil, ())
  | _, _, .kfold _ _ _ _ g init _ _ _ x, _, Δ, s =>
    let s' := memo fun i => kfoldStepH g init (s i) (Δ.get x i)
    (s', HList.cons s' HList.nil, ())
  | _, _, .ksnapshot _ _ _ x _, _, Δ, s =>
    (s, HList.cons (fun i => if tk then some (Δ.get x i) else none) HList.nil, ())
  | _, _, .tfold _ g _ _ _ _ x, _, Δ, s =>
    let run := memo fun i => scanRun (tfoldStep g) (s i) (Δ.get x i).toList
    (fun i => (run i).1, HList.cons (fun i => (run i).2.head?) HList.nil, ())
  | _, _, .latest _ x, _, Δ, s =>
    let s' := memo fun i => match Δ.get x i with
      | some o => o
      | none => s i
    (s', HList.cons s' HList.nil, ())
  | _, _, .latest_monotone x, _, Δ, s =>
    let s' := memo fun i => match Δ.get x i with
      | some o => o
      | none => s i
    (s', HList.cons s' HList.nil, ())
  | _, _, .time, d, _, s =>
    (fun i => if tk then s i + 1 else s i, HList.cons (fun i => if tk then some (d i) else none) HList.nil, ())
  | _, _, .snapshot_bounded v _, _, _, s =>
    (fun i => if tk then s i + 1 else s i, HList.cons (fun i => if tk then some v else none) HList.nil, ())
  | _, _, .tick_scan (ℓ := ℓ) _ ins outs x g _, _, Δ, s =>
    let leaves : Hosted m ℓ → HList (Kind.locDelta m) (shapeKinds ℓ ins) :=
      fun _ => readShapeLoc m Δ ℓ ins x
    -- the member's tick run, once: the cells and every output leaf read it
    let res := memo fun i =>
      let p := PendingOf.pushLoc m ins (s i).1 leaves i
      let n := PendingOf.avail ins p
      let run := scanRun (g i.1) (s i).2 (PendingOf.takeZip ins p n)
      ((PendingOf.dropN ins p n, run.1), run.2.head?)
    (fun i => (res i).1, unzipShapeLoc m ℓ outs (fun i => (res i).2), ())
  | _, _, .call N args, ora, Δ, s => Dag.locStep m tk N ora (args.map (fun v => Δ.get v)) s
  | _, _, .fixN _ body, ora, Δ, s =>
    -- active: release all pending feedback, keep the body's new output
    let r := Dag.locStep m tk body ora (s.2.append Δ) s.1
    ((r.1, r.2.1.take), s.2.append r.2.1.drop, r.2.2)

/-- One step of a process over a graph. -/
def Dag.locStep (m : Member L mem) (tk : Bool) :
    {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) → Dag.Oracle m d →
    HList (Kind.locDelta m) Γ → Dag.LocState m d →
    Dag.LocState m d × HList (Kind.locDelta m) outs × Dag.Sends m d
  | _, _, .ret xs, _, Δ, s => (s, xs.map (fun v => Δ.get v), ())
  | _, _, .letN n rest, ora, Δ, s =>
    let r := Node.locStep m tk n ora.1 Δ s.1
    -- materialize this node's cells and increments before the rest reads them
    let st := Node.memoState m n r.1
    let out := memoEnv m r.2.1
    let r' := Dag.locStep m tk rest ora.2 (out.append Δ) s.2
    ((st, r'.1), r'.2.1, (r.2.2, r'.2.2))

end

/-! ## Skipping the nodes of other locations (D86)

A process is one member; a node whose locations it does not host has empty
`Hosted` domains everywhere — its cells, increments and sends are functions
nobody can call. `Dag.locStep` still visits it (and materializes its empty
values); the compiled form `Dag.locStepImpl` leaves such a node's cells
untouched and hands the rest of the graph empty increments, which is the same
value (`Node.locStep_idle`: every component is a function out of an empty type).
`@[csimp]`, the one sanctioned seam. -/

/-- A wire's location. -/
def Kind.loc : Kind L mem → L
  | .stream ℓ .. => ℓ
  | .keyed ℓ .. => ℓ
  | .sing ℓ .. => ℓ
  | .ksing ℓ .. => ℓ
  | .atomic ℓ .. => ℓ
  | .atomicStream ℓ .. => ℓ
  | .atomicKeyed ℓ .. => ℓ
  | .atomicKSing ℓ .. => ℓ

/-- No member of `ℓ` is this process. -/
theorem Hosted.elim {m : Member L mem} {ℓ : L} (h : ¬ m.1 = ℓ) (i : Hosted m ℓ) : False :=
  h (congrArg Sigma.fst i.2).symm

/-- The empty increment of a wire at a location the process does not host. -/
def Kind.idleLoc (m : Member L mem) : (κ : Kind L mem) → ¬ m.1 = κ.loc → κ.locDelta m
  | .stream .., h => fun i => (Hosted.elim h i).elim
  | .keyed .., h => fun i => (Hosted.elim h i).elim
  | .sing .., h => fun i => (Hosted.elim h i).elim
  | .ksing .., h => fun i => (Hosted.elim h i).elim
  | .atomic .., h => fun i => (Hosted.elim h i).elim
  | .atomicStream .., h => fun i => (Hosted.elim h i).elim
  | .atomicKeyed .., h => fun i => (Hosted.elim h i).elim
  | .atomicKSing .., h => fun i => (Hosted.elim h i).elim

/-- Every function out of an empty hosted domain is the empty increment. -/
theorem Kind.locDelta_eq_idle (m : Member L mem) : (κ : Kind L mem) → (h : ¬ m.1 = κ.loc) →
    (Δ : κ.locDelta m) → Δ = Kind.idleLoc m κ h
  | .stream .., h, Δ => funext fun i => (Hosted.elim h i).elim
  | .keyed .., h, Δ => funext fun i => (Hosted.elim h i).elim
  | .sing .., h, Δ => funext fun i => (Hosted.elim h i).elim
  | .ksing .., h, Δ => funext fun i => (Hosted.elim h i).elim
  | .atomic .., h, Δ => funext fun i => (Hosted.elim h i).elim
  | .atomicStream .., h, Δ => funext fun i => (Hosted.elim h i).elim
  | .atomicKeyed .., h, Δ => funext fun i => (Hosted.elim h i).elim
  | .atomicKSing .., h, Δ => funext fun i => (Hosted.elim h i).elim

/-- The empty increments of wires at locations the process does not host. -/
def HList.idle (m : Member L mem) : (κs : List (Kind L mem)) → (∀ κ ∈ κs, ¬ m.1 = κ.loc) →
    HList (Kind.locDelta m) κs
  | [], _ => HList.nil
  | κ :: κs, h => HList.cons (Kind.idleLoc m κ (h κ List.mem_cons_self))
      (HList.idle m κs (fun κ' hκ' => h κ' (List.mem_cons_of_mem _ hκ')))

theorem HList.eq_idle (m : Member L mem) : (κs : List (Kind L mem)) → (h : ∀ κ ∈ κs, ¬ m.1 = κ.loc) →
    (Δ : HList (Kind.locDelta m) κs) → Δ = HList.idle m κs h
  | [], _, _ => rfl
  | κ :: κs, h, Δ => by
    show (Δ.1, Δ.2) = (_, _)
    rw [Kind.locDelta_eq_idle m κ (h κ List.mem_cons_self) Δ.1,
      HList.eq_idle m κs (fun κ' hκ' => h κ' (List.mem_cons_of_mem _ hκ')) Δ.2]

/-- The node's location besides its outputs': the sender cluster of a network node
(its sends live there); a callee graph or a knot spans many — always walked. -/
def Node.extraLoc (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → Node Γ κs → Bool
  | _, _, .broadcast_closed (c := c) .. => decide (m.1 = c)
  | _, _, .demux (c := c) .. => decide (m.1 = c)
  | _, _, .call .. => true
  | _, _, .fixN .. => true
  | _, _, _ => false

/-- Whether the process hosts any of the node's locations (its outputs' and its
sender cluster's; callees and knots count as hosted and are walked). -/
def Node.touches (m : Member L mem) {Γ : Ctx L mem} {κs : List (Kind L mem)} (n : Node Γ κs) : Bool :=
  Node.extraLoc m n || κs.any (fun κ => decide (m.1 = κ.loc))

theorem Node.touches_false {Γ : Ctx L mem} {κs : List (Kind L mem)} (m : Member L mem) (n : Node Γ κs)
    (h : Node.touches m n = false) : Node.extraLoc m n = false ∧ ∀ κ ∈ κs, ¬ m.1 = κ.loc := by
  unfold Node.touches at h
  rw [Bool.or_eq_false_iff, List.any_eq_false] at h
  exact ⟨h.1, fun κ hκ => by simpa using h.2 κ hκ⟩

/-- A node the process does not touch has empty sends. -/
def Node.idleSends (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    Node.extraLoc m n = false → Node.Sends m n
  | _, _, .broadcast_closed .., h => fun j => (Hosted.elim (by simpa [Node.extraLoc] using h) j).elim
  | _, _, .demux .., h => fun j => (Hosted.elim (by simpa [Node.extraLoc] using h) j).elim
  | _, _, .call .., h => by simp [Node.extraLoc] at h
  | _, _, .fixN .., h => by simp [Node.extraLoc] at h
  | _, _, .map .., _ => ()
  | _, _, .filterMap .., _ => ()
  | _, _, .values .., _ => ()
  | _, _, .into_keyed .., _ => ()
  | _, _, .kassume_ordering .., _ => ()
  | _, _, .kfold_early_stop .., _ => ()
  | _, _, .kget_max_key .., _ => ()
  | _, _, .weaken_retries .., _ => ()
  | _, _, .union .., _ => ()
  | _, _, .assume_ordering .., _ => ()
  | _, _, .fold .., _ => ()
  | _, _, .fold_monotone .., _ => ()
  | _, _, .snapshot .., _ => ()
  | _, _, .batch .., _ => ()
  | _, _, .source_interval .., _ => ()
  | _, _, .mapTick .., _ => ()
  | _, _, .firstTick .., _ => ()
  | _, _, .zipTick .., _ => ()
  | _, _, .defer_tick .., _ => ()
  | _, _, .allTicks .., _ => ()
  | _, _, .flattenOrdered .., _ => ()
  | _, _, .flattenUnordered .., _ => ()
  | _, _, .tinto_keyed .., _ => ()
  | _, _, .tassume_ordering .., _ => ()
  | _, _, .kentries .., _ => ()
  | _, _, .kmap .., _ => ()
  | _, _, .kallTicks .., _ => ()
  | _, _, .kfold .., _ => ()
  | _, _, .ksnapshot .., _ => ()
  | _, _, .tfold .., _ => ()
  | _, _, .latest .., _ => ()
  | _, _, .latest_monotone .., _ => ()
  | _, _, .time .., _ => ()
  | _, _, .snapshot_bounded .., _ => ()
  | _, _, .reduce_watermark .., _ => ()
  | _, _, .tick_scan .., _ => ()

/-- A tick shape's wires all live at the tick's location, and there is at least one. -/
theorem shapeKinds_not_hosted (m : Member L mem) (ℓ : L) : (sh : TickShape) →
    (∀ κ ∈ shapeKinds (mem := mem) ℓ sh, ¬ m.1 = κ.loc) → ¬ m.1 = ℓ
  | .sing _, h => h _ List.mem_cons_self
  | .stream .., h => h _ List.mem_cons_self
  | .ksing .., h => h _ List.mem_cons_self
  | .kstream .., h => h _ List.mem_cons_self
  | .pair a _, h => shapeKinds_not_hosted m ℓ a (fun κ hκ => h κ (List.mem_append_left _ hκ))

/-- **A node the process does not touch is inert**: its step leaves the cells as they are
and emits the empty increments and sends — every component is a function out of an empty
hosted domain. -/
theorem Node.locStep_idle (m : Member L mem) (tk : Bool) {Γ : Ctx L mem} {κs : List (Kind L mem)}
    (n : Node Γ κs) (ora : Node.Oracle m n) (Δ : HList (Kind.locDelta m) Γ) (s : Node.LocState m n)
    (h : Node.touches m n = false) :
    Node.locStep m tk n ora Δ s = (s, HList.idle m κs (Node.touches_false m n h).2,
      Node.idleSends m n (Node.touches_false m n h).1) := by
  obtain ⟨hx, hκ⟩ := Node.touches_false m n h
  -- the cells: a function out of the node's hosted domain, which the outputs' locations
  -- show empty; the increments: `HList.eq_idle`; the sends: empty or `Unit`
  refine Prod.ext ?_ (Prod.ext (HList.eq_idle m κs hκ _) ?_)
  · cases n <;> first
      | rfl
      | (exfalso; simp [Node.extraLoc] at hx; done)
      | exact funext fun i => (Hosted.elim (hκ _ List.mem_cons_self) i).elim
      | exact funext fun i => (Hosted.elim (shapeKinds_not_hosted m _ _ hκ) i).elim
  · cases n <;> first
      | rfl
      | (exfalso; simp [Node.extraLoc] at hx; done)
      | exact funext fun j => (Hosted.elim (by simpa [Node.extraLoc] using hx) j).elim

/-- Materialize a node's cells, compiled: a callee graph's or a knot body's cells were
materialized node by node as it was stepped (`Dag.locStepImpl`), so they are left as they
are — `Node.memoState` would walk the callee again at every level of nesting. -/
def Node.memoStateImpl (m : Member L mem) {Γ : Ctx L mem} {κs : List (Kind L mem)} (n : Node Γ κs)
    (s : Node.LocState m n) : Node.LocState m n :=
  match n, s with
  | .call .., s => s
  | .fixN .., s => s
  | n, s => Node.memoState m n s

theorem Node.memoStateImpl_eq (m : Member L mem) {Γ : Ctx L mem} {κs : List (Kind L mem)} (n : Node Γ κs)
    (s : Node.LocState m n) : Node.memoStateImpl m n s = s := by
  unfold Node.memoStateImpl
  split
  · rfl
  · rfl
  · exact Node.memoState_eq m _ s

mutual

/-- `Node.locStep`, compiled: callees and knots walk `Dag.locStepImpl`; every other node
is `Node.locStep`. -/
def Node.locStepImpl (m : Member L mem) (tk : Bool) :
    {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) → Node.Oracle m n →
    HList (Kind.locDelta m) Γ → Node.LocState m n →
    Node.LocState m n × HList (Kind.locDelta m) κs × Node.Sends m n
  | _, _, .call N args, ora, Δ, s => Dag.locStepImpl m tk N ora (args.map (fun v => Δ.get v)) s
  | _, _, .fixN _ body, ora, Δ, s =>
    let r := Dag.locStepImpl m tk body ora (s.2.append Δ) s.1
    ((r.1, r.2.1.take), s.2.append r.2.1.drop, r.2.2)
  | _, _, n, ora, Δ, s => Node.locStep m tk n ora Δ s

/-- `Dag.locStep`, compiled: a node the process does not touch is skipped — its cells stay,
the rest of the graph reads empty increments (the same values, `Node.locStep_idle`). -/
def Dag.locStepImpl (m : Member L mem) (tk : Bool) :
    {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) → Dag.Oracle m d →
    HList (Kind.locDelta m) Γ → Dag.LocState m d →
    Dag.LocState m d × HList (Kind.locDelta m) outs × Dag.Sends m d
  | _, _, .ret xs, _, Δ, s => (s, xs.map (fun v => Δ.get v), ())
  | _, _, .letN n rest, ora, Δ, s =>
    if h : Node.touches m n = false then
      let r' := Dag.locStepImpl m tk rest ora.2 ((HList.idle m _ (Node.touches_false m n h).2).append Δ) s.2
      ((s.1, r'.1), r'.2.1, (Node.idleSends m n (Node.touches_false m n h).1, r'.2.2))
    else
      let r := Node.locStepImpl m tk n ora.1 Δ s.1
      let st := Node.memoStateImpl m n r.1
      let out := memoEnv m r.2.1
      let r' := Dag.locStepImpl m tk rest ora.2 (out.append Δ) s.2
      ((st, r'.1), r'.2.1, (r.2.2, r'.2.2))

end

mutual

theorem Node.locStepImpl_eq (m : Member L mem) (tk : Bool) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} →
    (n : Node Γ κs) → (ora : Node.Oracle m n) → (Δ : HList (Kind.locDelta m) Γ) → (s : Node.LocState m n) →
    Node.locStepImpl m tk n ora Δ s = Node.locStep m tk n ora Δ s
  | _, _, .call N args, ora, Δ, s => Dag.locStepImpl_eq m tk N ora (args.map (fun v => Δ.get v)) s
  | _, _, .fixN ks body, ora, Δ, s => by
    have h := Dag.locStepImpl_eq m tk body ora (s.2.append Δ) s.1
    show (let r := Dag.locStepImpl m tk body ora (s.2.append Δ) s.1
      ((r.1, r.2.1.take), s.2.append r.2.1.drop, r.2.2)) = _
    rw [h]
    rfl
  | _, _, .map .., _, _, _ => rfl
  | _, _, .filterMap .., _, _, _ => rfl
  | _, _, .broadcast_closed .., _, _, _ => rfl
  | _, _, .demux .., _, _, _ => rfl
  | _, _, .values .., _, _, _ => rfl
  | _, _, .into_keyed .., _, _, _ => rfl
  | _, _, .kassume_ordering .., _, _, _ => rfl
  | _, _, .kfold_early_stop .., _, _, _ => rfl
  | _, _, .kget_max_key .., _, _, _ => rfl
  | _, _, .weaken_retries .., _, _, _ => rfl
  | _, _, .union .., _, _, _ => rfl
  | _, _, .assume_ordering .., _, _, _ => rfl
  | _, _, .fold .., _, _, _ => rfl
  | _, _, .fold_monotone .., _, _, _ => rfl
  | _, _, .snapshot .., _, _, _ => rfl
  | _, _, .batch .., _, _, _ => rfl
  | _, _, .source_interval .., _, _, _ => rfl
  | _, _, .mapTick .., _, _, _ => rfl
  | _, _, .firstTick .., _, _, _ => rfl
  | _, _, .zipTick .., _, _, _ => rfl
  | _, _, .defer_tick .., _, _, _ => rfl
  | _, _, .allTicks .., _, _, _ => rfl
  | _, _, .flattenOrdered .., _, _, _ => rfl
  | _, _, .flattenUnordered .., _, _, _ => rfl
  | _, _, .tinto_keyed .., _, _, _ => rfl
  | _, _, .tassume_ordering .., _, _, _ => rfl
  | _, _, .kentries .., _, _, _ => rfl
  | _, _, .kmap .., _, _, _ => rfl
  | _, _, .kallTicks .., _, _, _ => rfl
  | _, _, .kfold .., _, _, _ => rfl
  | _, _, .ksnapshot .., _, _, _ => rfl
  | _, _, .tfold .., _, _, _ => rfl
  | _, _, .latest .., _, _, _ => rfl
  | _, _, .latest_monotone .., _, _, _ => rfl
  | _, _, .time .., _, _, _ => rfl
  | _, _, .snapshot_bounded .., _, _, _ => rfl
  | _, _, .reduce_watermark .., _, _, _ => rfl
  | _, _, .tick_scan .., _, _, _ => rfl

theorem Dag.locStepImpl_eq (m : Member L mem) (tk : Bool) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} →
    (d : Dag Γ outs) → (ora : Dag.Oracle m d) → (Δ : HList (Kind.locDelta m) Γ) → (s : Dag.LocState m d) →
    Dag.locStepImpl m tk d ora Δ s = Dag.locStep m tk d ora Δ s
  | _, _, .ret _, _, _, _ => rfl
  | _, _, .letN n rest, ora, Δ, s => by
    unfold Dag.locStepImpl
    split
    · rename_i h
      -- the skipped node's step is the idle triple
      show _ = (let r := Node.locStep m tk n ora.1 Δ s.1
        let st := Node.memoState m n r.1
        let out := memoEnv m r.2.1
        let r' := Dag.locStep m tk rest ora.2 (out.append Δ) s.2
        ((st, r'.1), r'.2.1, (r.2.2, r'.2.2)))
      simp only [Node.locStep_idle m tk n ora.1 Δ s.1 h, Node.memoState_eq, memoEnv_eq, Dag.locStepImpl_eq]
      rfl
    · show (let r := Node.locStepImpl m tk n ora.1 Δ s.1
        let st := Node.memoStateImpl m n r.1
        let out := memoEnv m r.2.1
        let r' := Dag.locStepImpl m tk rest ora.2 (out.append Δ) s.2
        ((st, r'.1), r'.2.1, (r.2.2, r'.2.2)))
        = (let r := Node.locStep m tk n ora.1 Δ s.1
        let st := Node.memoState m n r.1
        let out := memoEnv m r.2.1
        let r' := Dag.locStep m tk rest ora.2 (out.append Δ) s.2
        ((st, r'.1), r'.2.1, (r.2.2, r'.2.2)))
      simp only [Node.locStepImpl_eq, Dag.locStepImpl_eq, Node.memoStateImpl_eq, Node.memoState_eq]

end

@[csimp] theorem Dag.locStep_eq_impl : @Dag.locStep = @Dag.locStepImpl := by
  funext L mem _ m tk Γ outs d ora Δ s
  exact (Dag.locStepImpl_eq m tk d ora Δ s).symm

end Project

end Hydro
