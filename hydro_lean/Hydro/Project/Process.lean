import Hydro.Project.Wire
import Hydro.Project.WiredSpec

/-!
# Hydro · Project — the process's loop body (D78)

What a production process does per activation, as one library function:
take the messages that arrived since the last activation (`Dag.Inbox`:
per network node, per receiving end, per sender — a per-connection
FIFO), append them to the queues (`Dag.enqueueInbox`), deliver everything
that has arrived and run one step of the projected graph (`Dag.locStep`
at `Dag.oracleAll`), and hand back the new cells, the outputs and the
sends. The sends — to every peer *and to itself* — go to the network and
nowhere else (`locStep` does not enqueue them; R13: production sends, it
does not keep). This is `Dag.iterate` — one function, what the processes
run and what the theorems are about. The TCP shell's only job is to hand
it the inbox: the sends it decoded, per sender, in arrival order
(`Dag.inboxOf`).

The bridge to the lock-step machine (proof layer only): a lock-step cell
at a process is its loop cell plus the network's in-flight queue toward it
(`Dag.arrivedOnly` drops that queue). The loop body fed the cursor's next
messages (`Dag.arrivalsOf`) from the loop cells computes exactly the
lock-step step's outputs and sends, and the loop cells of its result
(`iterate_locStep`). `Project/Loop.lean` lifts this to a whole
interleaving.
-/

namespace Hydro

namespace Project

variable {L : Type} {mem : L → Nat} [DecidableEq L]

/-! ## The inbox -/

mutual

/-- The messages that arrived at the member's receiving ends since the last activation:
per network node, per receiving end, per sender (the receiver-side items — a
`demux` already filtered by address). -/
def Node.Inbox (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → Node Γ κs → Type
  | _, _, .broadcast_closed (c := c) (p := p) (α := α) _ _ => Hosted m p → Fin (mem c) → List α
  | _, _, .demux (c := c) (p := p) (α := α) _ _ _ => Hosted m p → Fin (mem c) → List α
  | _, _, .call N _ => Dag.Inbox m N
  | _, _, .fixN _ body => Dag.Inbox m body
  | _, _, _ => Unit

def Dag.Inbox (m : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → Dag Γ outs → Type
  | _, _, .ret _ => Unit
  | _, _, .letN n rest => Node.Inbox m n × Dag.Inbox m rest

end

mutual

/-- Append the inbox to the queues. -/
def Node.enqueueInbox (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    Node.Inbox m n → Node.LocState m n → Node.LocState m n
  | _, _, .broadcast_closed .., ib, s => fun i j => (s i j).enqueue (ib i j)
  | _, _, .demux .., ib, s => fun i j => (s i j).enqueue (ib i j)
  | _, _, .call N _, ib, s => Dag.enqueueInbox m N ib s
  | _, _, .fixN _ body, ib, s => (Dag.enqueueInbox m body ib s.1, s.2)
  | _, _, .map .., _, s => s
  | _, _, .filterMap .., _, s => s
  | _, _, .values .., _, s => s
  | _, _, .into_keyed .., _, s => s
  | _, _, .kassume_ordering .., _, s => s
  | _, _, .kfold_early_stop .., _, s => s
  | _, _, .kget_max_key .., _, s => s
  | _, _, .weaken_retries .., _, s => s
  | _, _, .union .., _, s => s
  | _, _, .assume_ordering .., _, s => s
  | _, _, .fold .., _, s => s
  | _, _, .fold_monotone .., _, s => s
  | _, _, .snapshot .., _, s => s
  | _, _, .batch .., _, s => s
  | _, _, .batch_ordered .., _, s => s
  | _, _, .sample_every .., _, s => s
  | _, _, .timeout_snapshot .., _, s => s
  | _, _, .source_interval_batch .., _, s => s
  | _, _, .mapTick .., _, s => s
  | _, _, .zipTick .., _, s => s
  | _, _, .defer_tick .., _, s => s
  | _, _, .allTicks .., _, s => s
  | _, _, .flattenOrdered .., _, s => s
  | _, _, .flattenUnordered .., _, s => s
  | _, _, .tinto_keyed .., _, s => s
  | _, _, .tassume_ordering .., _, s => s
  | _, _, .reduce_watermark .., _, s => s
  | _, _, .tick_scan .., _, s => s

def Dag.enqueueInbox (m : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    Dag.Inbox m d → Dag.LocState m d → Dag.LocState m d
  | _, _, .ret _, _, _ => ()
  | _, _, .letN n rest, ib, s => (Node.enqueueInbox m n ib.1 s.1, Dag.enqueueInbox m rest ib.2 s.2)

end

mutual

/-- The loop cells: a queue holds nothing in flight (what has arrived was delivered
and counted; what is in flight is the network's). -/
def Node.arrivedOnly (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    Node.LocState m n → Node.LocState m n
  | _, _, .broadcast_closed .., s => fun i j => ⟨[], (s i j).delivered⟩
  | _, _, .demux .., s => fun i j => ⟨[], (s i j).delivered⟩
  | _, _, .call N _, s => Dag.arrivedOnly m N s
  | _, _, .fixN _ body, s => (Dag.arrivedOnly m body s.1, s.2)
  | _, _, .map .., s => s
  | _, _, .filterMap .., s => s
  | _, _, .values .., s => s
  | _, _, .into_keyed .., s => s
  | _, _, .kassume_ordering .., s => s
  | _, _, .kfold_early_stop .., s => s
  | _, _, .kget_max_key .., s => s
  | _, _, .weaken_retries .., s => s
  | _, _, .union .., s => s
  | _, _, .assume_ordering .., s => s
  | _, _, .fold .., s => s
  | _, _, .fold_monotone .., s => s
  | _, _, .snapshot .., s => s
  | _, _, .batch .., s => s
  | _, _, .batch_ordered .., s => s
  | _, _, .sample_every .., s => s
  | _, _, .timeout_snapshot .., s => s
  | _, _, .source_interval_batch .., s => s
  | _, _, .mapTick .., s => s
  | _, _, .zipTick .., s => s
  | _, _, .defer_tick .., s => s
  | _, _, .allTicks .., s => s
  | _, _, .flattenOrdered .., s => s
  | _, _, .flattenUnordered .., s => s
  | _, _, .tinto_keyed .., s => s
  | _, _, .tassume_ordering .., s => s
  | _, _, .reduce_watermark .., s => s
  | _, _, .tick_scan .., s => s

def Dag.arrivedOnly (m : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    Dag.LocState m d → Dag.LocState m d
  | _, _, .ret _, _ => ()
  | _, _, .letN n rest, s => (Node.arrivedOnly m n s.1, Dag.arrivedOnly m rest s.2)

end

mutual

/-- The messages the cursor delivers next, read off the lock-step cells: the
in-flight prefix up to the cursor (clamped to what is in flight). -/
def Node.arrivalsOf (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    Node.Oracle m n → Node.LocState m n → Node.Inbox m n
  | _, _, .broadcast_closed .., ora, s => fun i j => (s i j).pending.take (ora i j - (s i j).delivered)
  | _, _, .demux .., ora, s => fun i j => (s i j).pending.take (ora i j - (s i j).delivered)
  | _, _, .call N _, ora, s => Dag.arrivalsOf m N ora s
  | _, _, .fixN _ body, ora, s => Dag.arrivalsOf m body ora s.1
  | _, _, .map .., _, _ => ()
  | _, _, .filterMap .., _, _ => ()
  | _, _, .values .., _, _ => ()
  | _, _, .into_keyed .., _, _ => ()
  | _, _, .kassume_ordering .., _, _ => ()
  | _, _, .kfold_early_stop .., _, _ => ()
  | _, _, .kget_max_key .., _, _ => ()
  | _, _, .weaken_retries .., _, _ => ()
  | _, _, .union .., _, _ => ()
  | _, _, .assume_ordering .., _, _ => ()
  | _, _, .fold .., _, _ => ()
  | _, _, .fold_monotone .., _, _ => ()
  | _, _, .snapshot .., _, _ => ()
  | _, _, .batch .., _, _ => ()
  | _, _, .batch_ordered .., _, _ => ()
  | _, _, .sample_every .., _, _ => ()
  | _, _, .timeout_snapshot .., _, _ => ()
  | _, _, .source_interval_batch .., _, _ => ()
  | _, _, .mapTick .., _, _ => ()
  | _, _, .zipTick .., _, _ => ()
  | _, _, .defer_tick .., _, _ => ()
  | _, _, .allTicks .., _, _ => ()
  | _, _, .flattenOrdered .., _, _ => ()
  | _, _, .flattenUnordered .., _, _ => ()
  | _, _, .tinto_keyed .., _, _ => ()
  | _, _, .tassume_ordering .., _, _ => ()
  | _, _, .reduce_watermark .., _, _ => ()
  | _, _, .tick_scan .., _, _ => ()

def Dag.arrivalsOf (m : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    Dag.Oracle m d → Dag.LocState m d → Dag.Inbox m d
  | _, _, .ret _, _, _ => ()
  | _, _, .letN n rest, ora, s => (Node.arrivalsOf m n ora.1 s.1, Dag.arrivalsOf m rest ora.2 s.2)

end

mutual

/-- A sender process's sends, as this process's inbox (a `demux` filtered by address). -/
def Node.inboxOfSends (m src : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    Node.Sends src n → Node.Inbox m n
  | _, _, .broadcast_closed (c := c) .., snd => fun _ j => if h : (⟨c, j⟩ : Member L mem) = src then snd ⟨j, h⟩ else []
  | _, _, .demux (c := c) _ _ addr, snd => fun i j =>
    if h : (⟨c, j⟩ : Member L mem) = src then (snd ⟨j, h⟩).filterMap (fun dx => if dx.1 = addr i.1 then some dx.2 else none) else []
  | _, _, .call N _, snd => Dag.inboxOfSends m src N snd
  | _, _, .fixN _ body, snd => Dag.inboxOfSends m src body snd
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
  | _, _, .batch_ordered .., _ => ()
  | _, _, .sample_every .., _ => ()
  | _, _, .timeout_snapshot .., _ => ()
  | _, _, .source_interval_batch .., _ => ()
  | _, _, .mapTick .., _ => ()
  | _, _, .zipTick .., _ => ()
  | _, _, .defer_tick .., _ => ()
  | _, _, .allTicks .., _ => ()
  | _, _, .flattenOrdered .., _ => ()
  | _, _, .flattenUnordered .., _ => ()
  | _, _, .tinto_keyed .., _ => ()
  | _, _, .tassume_ordering .., _ => ()
  | _, _, .reduce_watermark .., _ => ()
  | _, _, .tick_scan .., _ => ()

def Dag.inboxOfSends (m src : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    Dag.Sends src d → Dag.Inbox m d
  | _, _, .ret _, _ => ()
  | _, _, .letN n rest, snd => (Node.inboxOfSends m src n snd.1, Dag.inboxOfSends m src rest snd.2)

end

mutual

/-- Enqueueing a sender's sends is enqueueing the inbox they make. -/
theorem Node.enqueueLoc_eq (m src : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    (snd : Node.Sends src n) → (s : Node.LocState m n) →
    Node.enqueueLoc m src n snd s = Node.enqueueInbox m n (Node.inboxOfSends m src n snd) s
  | _, _, .broadcast_closed .., snd, s => by
    funext i j
    simp only [Node.enqueueLoc, Node.enqueueInbox, Node.inboxOfSends]
    split <;> simp [ChanCell.enqueue]
  | _, _, .demux .., snd, s => by
    funext i j
    simp only [Node.enqueueLoc, Node.enqueueInbox, Node.inboxOfSends]
    split <;> simp [ChanCell.enqueue]
  | _, _, .call N _, snd, s => Dag.enqueueLoc_eq m src N snd s
  | _, _, .fixN _ body, snd, s =>
    congrArg (fun x => (x, s.2)) (Dag.enqueueLoc_eq m src body snd s.1)
  | _, _, .map .., _, _ => rfl
  | _, _, .filterMap .., _, _ => rfl
  | _, _, .values .., _, _ => rfl
  | _, _, .into_keyed .., _, _ => rfl
  | _, _, .kassume_ordering .., _, _ => rfl
  | _, _, .kfold_early_stop .., _, _ => rfl
  | _, _, .kget_max_key .., _, _ => rfl
  | _, _, .weaken_retries .., _, _ => rfl
  | _, _, .union .., _, _ => rfl
  | _, _, .assume_ordering .., _, _ => rfl
  | _, _, .fold .., _, _ => rfl
  | _, _, .fold_monotone .., _, _ => rfl
  | _, _, .snapshot .., _, _ => rfl
  | _, _, .batch .., _, _ => rfl
  | _, _, .batch_ordered .., _, _ => rfl
  | _, _, .sample_every .., _, _ => rfl
  | _, _, .timeout_snapshot .., _, _ => rfl
  | _, _, .source_interval_batch .., _, _ => rfl
  | _, _, .mapTick .., _, _ => rfl
  | _, _, .zipTick .., _, _ => rfl
  | _, _, .defer_tick .., _, _ => rfl
  | _, _, .allTicks .., _, _ => rfl
  | _, _, .flattenOrdered .., _, _ => rfl
  | _, _, .flattenUnordered .., _, _ => rfl
  | _, _, .tinto_keyed .., _, _ => rfl
  | _, _, .tassume_ordering .., _, _ => rfl
  | _, _, .reduce_watermark .., _, _ => rfl
  | _, _, .tick_scan .., _, _ => rfl

theorem Dag.enqueueLoc_eq (m src : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    (snd : Dag.Sends src d) → (s : Dag.LocState m d) →
    Dag.enqueueLoc m src d snd s = Dag.enqueueInbox m d (Dag.inboxOfSends m src d snd) s
  | _, _, .ret _, _, _ => rfl
  | _, _, .letN n rest, snd, s => by
    show (Node.enqueueLoc m src n snd.1 s.1, Dag.enqueueLoc m src rest snd.2 s.2) = _
    rw [Node.enqueueLoc_eq, Dag.enqueueLoc_eq]
    rfl

end

/-! ## The loop body -/

mutual

/-- The empty inbox. -/
def Node.Inbox.empty (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) → Node.Inbox m n
  | _, _, .broadcast_closed .. => fun _ _ => []
  | _, _, .demux .. => fun _ _ => []
  | _, _, .call N _ => Dag.Inbox.empty m N
  | _, _, .fixN _ body => Dag.Inbox.empty m body
  | _, _, .map .. => ()
  | _, _, .filterMap .. => ()
  | _, _, .values .. => ()
  | _, _, .into_keyed .. => ()
  | _, _, .kassume_ordering .. => ()
  | _, _, .kfold_early_stop .. => ()
  | _, _, .kget_max_key .. => ()
  | _, _, .weaken_retries .. => ()
  | _, _, .union .. => ()
  | _, _, .assume_ordering .. => ()
  | _, _, .fold .. => ()
  | _, _, .fold_monotone .. => ()
  | _, _, .snapshot .. => ()
  | _, _, .batch .. => ()
  | _, _, .batch_ordered .. => ()
  | _, _, .sample_every .. => ()
  | _, _, .timeout_snapshot .. => ()
  | _, _, .source_interval_batch .. => ()
  | _, _, .mapTick .. => ()
  | _, _, .zipTick .. => ()
  | _, _, .defer_tick .. => ()
  | _, _, .allTicks .. => ()
  | _, _, .flattenOrdered .. => ()
  | _, _, .flattenUnordered .. => ()
  | _, _, .tinto_keyed .. => ()
  | _, _, .tassume_ordering .. => ()
  | _, _, .reduce_watermark .. => ()
  | _, _, .tick_scan .. => ()

def Dag.Inbox.empty (m : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) → Dag.Inbox m d
  | _, _, .ret _ => ()
  | _, _, .letN n rest => (Node.Inbox.empty m n, Dag.Inbox.empty m rest)

end

mutual

/-- Two inboxes, the second's messages after the first's (per connection). -/
def Node.Inbox.append (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    Node.Inbox m n → Node.Inbox m n → Node.Inbox m n
  | _, _, .broadcast_closed .., a, b => fun i j => a i j ++ b i j
  | _, _, .demux .., a, b => fun i j => a i j ++ b i j
  | _, _, .call N _, a, b => Dag.Inbox.append m N a b
  | _, _, .fixN _ body, a, b => Dag.Inbox.append m body a b
  | _, _, .map .., _, _ => ()
  | _, _, .filterMap .., _, _ => ()
  | _, _, .values .., _, _ => ()
  | _, _, .into_keyed .., _, _ => ()
  | _, _, .kassume_ordering .., _, _ => ()
  | _, _, .kfold_early_stop .., _, _ => ()
  | _, _, .kget_max_key .., _, _ => ()
  | _, _, .weaken_retries .., _, _ => ()
  | _, _, .union .., _, _ => ()
  | _, _, .assume_ordering .., _, _ => ()
  | _, _, .fold .., _, _ => ()
  | _, _, .fold_monotone .., _, _ => ()
  | _, _, .snapshot .., _, _ => ()
  | _, _, .batch .., _, _ => ()
  | _, _, .batch_ordered .., _, _ => ()
  | _, _, .sample_every .., _, _ => ()
  | _, _, .timeout_snapshot .., _, _ => ()
  | _, _, .source_interval_batch .., _, _ => ()
  | _, _, .mapTick .., _, _ => ()
  | _, _, .zipTick .., _, _ => ()
  | _, _, .defer_tick .., _, _ => ()
  | _, _, .allTicks .., _, _ => ()
  | _, _, .flattenOrdered .., _, _ => ()
  | _, _, .flattenUnordered .., _, _ => ()
  | _, _, .tinto_keyed .., _, _ => ()
  | _, _, .tassume_ordering .., _, _ => ()
  | _, _, .reduce_watermark .., _, _ => ()
  | _, _, .tick_scan .., _, _ => ()

def Dag.Inbox.append (m : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    Dag.Inbox m d → Dag.Inbox m d → Dag.Inbox m d
  | _, _, .ret _, _, _ => ()
  | _, _, .letN n rest, a, b => (Node.Inbox.append m n a.1 b.1, Dag.Inbox.append m rest a.2 b.2)

end

/-- The inbox the shell hands over: the sends it decoded from its sockets — `(sender,
its sends)`, one entry per message batch, in arrival order — as this process's inbox. -/
def Dag.inboxOf (m : Member L mem) {Γ : Ctx L mem} {outs : List (Kind L mem)} (d : Dag Γ outs)
    (inbox : List ((src : Member L mem) × Dag.Sends src d)) : Dag.Inbox m d :=
  inbox.foldl (fun ib ⟨src, snd⟩ => Dag.Inbox.append m d ib (Dag.inboxOfSends m src d snd)) (Dag.Inbox.empty m d)

/-- **One activation of a process** — the loop body, the function the processes run and
the theorems are about: append the inbox, deliver everything that has arrived
(`oracleAll`), step the projected graph (one tick iff `tk`). Returns the new cells, the
outputs at the member, and the sends (to every peer, itself included — they
travel through the network; nothing stays in the process). `memoState` materializes the
cells once (an identity, `Dag.memoState_eq`; the executable's throughput). -/
def Dag.iterate (m : Member L mem) (tk : Bool) {Γ : Ctx L mem} {outs : List (Kind L mem)} (d : Dag Γ outs)
    (sch : Dag.Sched d) (ib : Dag.Inbox m d) (Δ : HList (Kind.locDelta m) Γ) (s : Dag.LocState m d) :
    Dag.LocState m d × HList (Kind.locDelta m) outs × Dag.Sends m d :=
  let s' := Dag.memoState m d (Dag.enqueueInbox m d ib s)
  Dag.locStep m tk d (Dag.oracleAll m d sch s') Δ s'

/-! ## The bridge, per node -/

theorem ChanCell.loop_step {α : Type} (cell : ChanCell α) (c : Nat) :
    ChanCell.step (cell.delivered + (cell.pending.take (c - cell.delivered)).length) []
        ⟨cell.pending.take (c - cell.delivered), cell.delivered⟩
      = (⟨[], (cell.step c []).1.delivered⟩, (cell.step c []).2) := by
  simp only [ChanCell.step, Nat.add_sub_cancel_left, List.take_length, List.drop_length, List.nil_append]

mutual

theorem Node.iterate_locStep (m : Member L mem) (tk : Bool) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} →
    (n : Node Γ κs) → (sch : Node.Sched n) → (t : Nat) → (Δ : HList (Kind.locDelta m) Γ) →
    (s : Node.LocState m n) →
    let ora := Node.cursorsAt m n sch t
    let s' := Node.enqueueInbox m n (Node.arrivalsOf m n ora s) (Node.arrivedOnly m n s)
    let r := Node.locStep m tk n ora Δ s
    let r' := Node.locStep m tk n (Node.oracleAll m n sch s') Δ s'
    r'.1 = Node.arrivedOnly m n r.1 ∧ r'.2.1 = r.2.1 ∧ r'.2.2 = r.2.2
  | _, _, .broadcast_closed .., _, _, Δ, s => by
    intro ora s' r r'
    simp only [Node.locStep, Node.oracleAll, Node.enqueueInbox, Node.arrivalsOf, Node.arrivedOnly, s', r, r',
      ChanCell.enqueue, List.nil_append]
    refine ⟨funext fun i => funext fun j => ?_, ?_, rfl⟩
    · rw [ChanCell.loop_step]
    · congr 1
      funext i
      congr 1
      funext j
      rw [ChanCell.loop_step]
  | _, _, .demux .., _, _, Δ, s => by
    intro ora s' r r'
    simp only [Node.locStep, Node.oracleAll, Node.enqueueInbox, Node.arrivalsOf, Node.arrivedOnly, s', r, r',
      ChanCell.enqueue, List.nil_append]
    refine ⟨funext fun i => funext fun j => ?_, ?_, rfl⟩
    · rw [ChanCell.loop_step]
    · congr 1
      funext i
      congr 1
      funext j
      rw [ChanCell.loop_step]
  | _, _, .call N _, sch, t, Δ, s => Dag.iterate_locStep m tk N sch t _ s
  | _, _, .fixN _ body, sch, t, Δ, s => by
    intro ora s' r r'
    obtain ⟨h1, h2, h3⟩ := Dag.iterate_locStep m tk body sch t (s.2.append Δ) s.1
    simp only [Node.locStep, Node.oracleAll, Node.enqueueInbox, Node.arrivalsOf, Node.arrivedOnly, s', r, r', ora]
    exact ⟨Prod.ext h1 (congrArg HList.take h2), congrArg _ (congrArg HList.drop h2), h3⟩
  | _, _, .map .., _, _, _, _ => ⟨rfl, rfl, rfl⟩
  | _, _, .filterMap .., _, _, _, _ => ⟨rfl, rfl, rfl⟩
  | _, _, .values .., _, _, _, _ => ⟨rfl, rfl, rfl⟩
  | _, _, .into_keyed .., _, _, _, _ => ⟨rfl, rfl, rfl⟩
  | _, _, .kassume_ordering .., _, _, _, _ => ⟨rfl, rfl, rfl⟩
  | _, _, .kfold_early_stop .., _, _, _, _ => ⟨rfl, rfl, rfl⟩
  | _, _, .kget_max_key .., _, _, _, _ => ⟨rfl, rfl, rfl⟩
  | _, _, .weaken_retries .., _, _, _, _ => ⟨rfl, rfl, rfl⟩
  | _, _, .union .., _, _, _, _ => ⟨rfl, rfl, rfl⟩
  | _, _, .assume_ordering .., _, _, _, _ => ⟨rfl, rfl, rfl⟩
  | _, _, .fold .., _, _, _, _ => ⟨rfl, rfl, rfl⟩
  | _, _, .fold_monotone .., _, _, _, _ => ⟨rfl, rfl, rfl⟩
  | _, _, .snapshot .., _, _, _, _ => ⟨rfl, rfl, rfl⟩
  | _, _, .batch .., _, _, _, _ => ⟨rfl, rfl, rfl⟩
  | _, _, .batch_ordered .., _, _, _, _ => ⟨rfl, rfl, rfl⟩
  | _, _, .sample_every .., _, _, _, _ => ⟨rfl, rfl, rfl⟩
  | _, _, .timeout_snapshot .., _, _, _, _ => ⟨rfl, rfl, rfl⟩
  | _, _, .source_interval_batch .., _, _, _, _ => ⟨rfl, rfl, rfl⟩
  | _, _, .mapTick .., _, _, _, _ => ⟨rfl, rfl, rfl⟩
  | _, _, .zipTick .., _, _, _, _ => ⟨rfl, rfl, rfl⟩
  | _, _, .defer_tick .., _, _, _, _ => ⟨rfl, rfl, rfl⟩
  | _, _, .allTicks .., _, _, _, _ => ⟨rfl, rfl, rfl⟩
  | _, _, .flattenOrdered .., _, _, _, _ => ⟨rfl, rfl, rfl⟩
  | _, _, .flattenUnordered .., _, _, _, _ => ⟨rfl, rfl, rfl⟩
  | _, _, .tinto_keyed .., _, _, _, _ => ⟨rfl, rfl, rfl⟩
  | _, _, .tassume_ordering .., _, _, _, _ => ⟨rfl, rfl, rfl⟩
  | _, _, .reduce_watermark .., _, _, _, _ => ⟨rfl, rfl, rfl⟩
  | _, _, .tick_scan .., _, _, _, _ => ⟨rfl, rfl, rfl⟩

theorem Dag.iterate_locStep (m : Member L mem) (tk : Bool) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} →
    (d : Dag Γ outs) → (sch : Dag.Sched d) → (t : Nat) → (Δ : HList (Kind.locDelta m) Γ) →
    (s : Dag.LocState m d) →
    let ora := Dag.cursorsAt m d sch t
    let s' := Dag.enqueueInbox m d (Dag.arrivalsOf m d ora s) (Dag.arrivedOnly m d s)
    let r := Dag.locStep m tk d ora Δ s
    let r' := Dag.locStep m tk d (Dag.oracleAll m d sch s') Δ s'
    r'.1 = Dag.arrivedOnly m d r.1 ∧ r'.2.1 = r.2.1 ∧ r'.2.2 = r.2.2
  | _, _, .ret _, _, _, _, _ => ⟨rfl, rfl, rfl⟩
  | _, _, .letN n rest, sch, t, Δ, s => by
    intro ora s' r r'
    obtain ⟨h1, h2, h3⟩ := Node.iterate_locStep m tk n sch.1 t Δ s.1
    have ihr := Dag.iterate_locStep m tk rest sch.2 t
      ((Node.locStep m tk n (Node.cursorsAt m n sch.1 t) Δ s.1).2.1.append Δ) s.2
    obtain ⟨g1, g2, g3⟩ := ihr
    simp only [Dag.locStep, Dag.oracleAll, Dag.enqueueInbox, Dag.arrivalsOf, Dag.arrivedOnly, Dag.cursorsAt,
      Node.memoState_eq, memoEnv_eq, s', r, r', ora]
    rw [h2]
    exact ⟨Prod.ext h1 g1, g2, Prod.ext h3 g3⟩

end

/-- The loop body on the loop cells, fed the cursor's arrivals, is the lock-step
process step: same outputs, same sends, and its cells are the step's loop cells (the
step's cells with the in-flight queue — the network's — removed). -/
theorem Dag.iterate_eq (m : Member L mem) (tk : Bool) {Γ : Ctx L mem} {outs : List (Kind L mem)} (d : Dag Γ outs)
    (sch : Dag.Sched d) (t : Nat) (Δ : HList (Kind.locDelta m) Γ) (s : Dag.LocState m d) :
    Dag.iterate m tk d sch (Dag.arrivalsOf m d (Dag.cursorsAt m d sch t) s) Δ (Dag.arrivedOnly m d s)
      = ((Dag.arrivedOnly m d (Dag.locStep m tk d (Dag.cursorsAt m d sch t) Δ s).1),
         (Dag.locStep m tk d (Dag.cursorsAt m d sch t) Δ s).2.1,
         (Dag.locStep m tk d (Dag.cursorsAt m d sch t) Δ s).2.2) := by
  obtain ⟨h1, h2, h3⟩ := Dag.iterate_locStep m tk d sch t Δ s
  unfold Dag.iterate
  rw [Dag.memoState_eq]
  exact Prod.ext h1 (Prod.ext h2 h3)

mutual

/-- Receiving another process's sends does not change the loop cells. -/
theorem Node.arrivedOnly_enqueueLoc (m src : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} →
    (n : Node Γ κs) → (snd : Node.Sends src n) → (s : Node.LocState m n) →
    Node.arrivedOnly m n (Node.enqueueLoc m src n snd s) = Node.arrivedOnly m n s
  | _, _, .broadcast_closed .., snd, s => by
    funext i j
    simp only [Node.arrivedOnly, Node.enqueueLoc]
    split <;> rfl
  | _, _, .demux .., snd, s => by
    funext i j
    simp only [Node.arrivedOnly, Node.enqueueLoc]
    split <;> rfl
  | _, _, .call N _, snd, s => Dag.arrivedOnly_enqueueLoc m src N snd s
  | _, _, .fixN _ body, snd, s =>
    congrArg (fun x => (x, s.2)) (Dag.arrivedOnly_enqueueLoc m src body snd s.1)
  | _, _, .map .., _, _ => rfl
  | _, _, .filterMap .., _, _ => rfl
  | _, _, .values .., _, _ => rfl
  | _, _, .into_keyed .., _, _ => rfl
  | _, _, .kassume_ordering .., _, _ => rfl
  | _, _, .kfold_early_stop .., _, _ => rfl
  | _, _, .kget_max_key .., _, _ => rfl
  | _, _, .weaken_retries .., _, _ => rfl
  | _, _, .union .., _, _ => rfl
  | _, _, .assume_ordering .., _, _ => rfl
  | _, _, .fold .., _, _ => rfl
  | _, _, .fold_monotone .., _, _ => rfl
  | _, _, .snapshot .., _, _ => rfl
  | _, _, .batch .., _, _ => rfl
  | _, _, .batch_ordered .., _, _ => rfl
  | _, _, .sample_every .., _, _ => rfl
  | _, _, .timeout_snapshot .., _, _ => rfl
  | _, _, .source_interval_batch .., _, _ => rfl
  | _, _, .mapTick .., _, _ => rfl
  | _, _, .zipTick .., _, _ => rfl
  | _, _, .defer_tick .., _, _ => rfl
  | _, _, .allTicks .., _, _ => rfl
  | _, _, .flattenOrdered .., _, _ => rfl
  | _, _, .flattenUnordered .., _, _ => rfl
  | _, _, .tinto_keyed .., _, _ => rfl
  | _, _, .tassume_ordering .., _, _ => rfl
  | _, _, .reduce_watermark .., _, _ => rfl
  | _, _, .tick_scan .., _, _ => rfl

theorem Dag.arrivedOnly_enqueueLoc (m src : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} →
    (d : Dag Γ outs) → (snd : Dag.Sends src d) → (s : Dag.LocState m d) →
    Dag.arrivedOnly m d (Dag.enqueueLoc m src d snd s) = Dag.arrivedOnly m d s
  | _, _, .ret _, _, _ => rfl
  | _, _, .letN n rest, snd, s => by
    show (Node.arrivedOnly m n (Node.enqueueLoc m src n snd.1 s.1), Dag.arrivedOnly m rest (Dag.enqueueLoc m src rest snd.2 s.2)) = _
    rw [Node.arrivedOnly_enqueueLoc, Dag.arrivedOnly_enqueueLoc]
    rfl

end

end Project

end Hydro
