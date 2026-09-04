import Hydro.Project.Model.WiredRun
import Hydro.Project.Process

/-!
# Hydro · Project — the distributed model (D78, D88)

**Model, not production.** `Project/Process.lean` is what a deployed process runs: the
loop body `Dag.pass` (and `Dag.passWire`, its byte-level form), a function from an
inbox and the process's own cells to its outputs and sends. This file is the *model of
many such processes running together*: an `Interleaving` (`Project/WiredRun.lean` — the
lock-step machine with one member active per clocked step, the delivery cursors and timer
verdicts as the schedule) read as a run of the processes' loop bodies. Nothing here is
executed; everything here is a theorem about what the executed function does inside the
model.

Process `m`'s loop cells at step `k` are the lock-step cells with nothing in flight
(`loopView`: the in-flight queues are the network's); at its own step it is fed the
messages the schedule's cursors deliver next (`arrivals`), the schedule's timing scripts
read at its cells' tick counts (`timing` — one entry per timer, this tick's; D84) and runs
one activation (`loopPass`). Then

* `loop_pass`: the activation's cells, outputs and sends are the lock-step step's —
  `(I.active k)` — so every theorem about `I.active` is a theorem about the loop body;
* `loop_idle`: a process that is not running does not change (its peers' sends are in
  flight, not in its cells).

So a process is `Dag.pass` iterated along its own activations, with inboxes
that are the next cursor-many messages of each connection — exactly what a TCP shell
that preserves per-connection order hands it. Nothing about the shell is assumed in
the statements: the inbox is *defined* from the senders' sends and the cursors.

**At the wire** (D88/D90, the second half of this file): the model's in-flight messages
are the senders' bytes (`view_idle_wire`, `view_active_wire`), and on a *batched*
interleaving — one whose arrivals decode from some per-sender batches of messages — the
loop body at the wire, `Dag.passWire` on those batches, is the lock-step step
(`Batched.loop_pass_wire`). That theorem goes through the codecs' round trip: the
production body and the model meet at the bytes. A `WireTrace` (`WireTrace.lean`) is a
history of the connections at the bytes and induces such an interleaving.
-/

namespace Hydro

namespace Project

variable {L : Type} {mem : L → Nat} [DecidableEq L]

/-! ## The bridge, per node: the loop body is the lock-step step on the loop cells

A lock-step cell at a process is its loop cell plus the network's in-flight queue
toward it (`Dag.arrivedOnly` drops that queue). The loop body fed the cursor's next
messages (`Dag.arrivalsOf`) from the loop cells computes exactly the lock-step step's
outputs and sends, and the loop cells of its result (`Dag.pass_locStep`,
`Dag.pass_eq`). -/

theorem ChanCell.loop_step {α : Type} (cell : ChanCell α) (c : Nat) :
    ChanCell.step (cell.delivered + (cell.pending.take (c - cell.delivered)).length) []
        ⟨cell.pending.take (c - cell.delivered), cell.delivered⟩
      = (⟨[], (cell.step c []).1.delivered⟩, (cell.step c []).2) := by
  simp only [ChanCell.step, Nat.add_sub_cancel_left, List.take_length, List.drop_length, List.nil_append]

mutual

theorem Node.pass_locStep (m : Member L mem) (tk : Bool) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} →
    (n : Node Γ κs) → (sch : Node.Sched n) → (t : Nat) → (Δ : HList (Kind.locDelta m) Γ) →
    (s : Node.LocState m n) →
    let ora := Node.cursorsAt m n sch t s
    let s' := Node.enqueueInbox m n (Node.arrivalsOf m n ora s) (Node.arrivedOnly m n s)
    let r := Node.locStep m tk n ora Δ s
    let r' := Node.locStep m tk n (Node.oracleAll m n sch s') Δ s'
    r'.1 = Node.arrivedOnly m n r.1 ∧ r'.2.1 = r.2.1 ∧ r'.2.2 = r.2.2
  | _, _, .broadcast_closed .., _, _, Δ, s => by
    intro ora s' r r'
    simp only [Node.locStep, memo_apply, memo2_apply, Node.oracleAll, Node.oracleOf, Node.timingAt, Node.enqueueInbox, Node.arrivalsOf,
      Node.arrivedOnly, s', r, r', ChanCell.enqueue, List.nil_append]
    refine ⟨funext fun i => funext fun j => ?_, ?_, rfl⟩
    · rw [ChanCell.loop_step]
    · congr 1
      funext i
      congr 1
      funext j
      rw [ChanCell.loop_step]
  | _, _, .demux .., _, _, Δ, s => by
    intro ora s' r r'
    simp only [Node.locStep, memo_apply, memo2_apply, Node.oracleAll, Node.oracleOf, Node.timingAt, Node.enqueueInbox, Node.arrivalsOf,
      Node.arrivedOnly, s', r, r', ChanCell.enqueue, List.nil_append]
    refine ⟨funext fun i => funext fun j => ?_, ?_, rfl⟩
    · rw [ChanCell.loop_step]
    · congr 1
      funext i
      congr 1
      funext j
      rw [ChanCell.loop_step]
  | _, _, .call N _, sch, t, Δ, s => Dag.pass_locStep m tk N sch t _ s
  | _, _, .fixN _ body, sch, t, Δ, s => by
    intro ora s' r r'
    obtain ⟨h1, h2, h3⟩ := Dag.pass_locStep m tk body sch t (s.2.append Δ) s.1
    simp only [Node.locStep, memo_apply, memo2_apply, Node.oracleAll, Node.oracleOf, Node.timingAt, Node.enqueueInbox, Node.arrivalsOf,
      Node.arrivedOnly, s', r, r', ora]
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
  | _, _, .source_interval .., _, _, _, _ => ⟨rfl, rfl, rfl⟩
  | _, _, .mapTick .., _, _, _, _ => ⟨rfl, rfl, rfl⟩
  | _, _, .firstTick .., _, _, _, _ => ⟨rfl, rfl, rfl⟩
  | _, _, .zipTick .., _, _, _, _ => ⟨rfl, rfl, rfl⟩
  | _, _, .defer_tick .., _, _, _, _ => ⟨rfl, rfl, rfl⟩
  | _, _, .allTicks .., _, _, _, _ => ⟨rfl, rfl, rfl⟩
  | _, _, .flattenOrdered .., _, _, _, _ => ⟨rfl, rfl, rfl⟩
  | _, _, .flattenUnordered .., _, _, _, _ => ⟨rfl, rfl, rfl⟩
  | _, _, .tinto_keyed .., _, _, _, _ => ⟨rfl, rfl, rfl⟩
  | _, _, .tassume_ordering .., _, _, _, _ => ⟨rfl, rfl, rfl⟩
  | _, _, .kentries .., _, _, _, _ => ⟨rfl, rfl, rfl⟩
  | _, _, .kmap .., _, _, _, _ => ⟨rfl, rfl, rfl⟩
  | _, _, .kallTicks .., _, _, _, _ => ⟨rfl, rfl, rfl⟩
  | _, _, .kfold .., _, _, _, _ => ⟨rfl, rfl, rfl⟩
  | _, _, .ksnapshot .., _, _, _, _ => ⟨rfl, rfl, rfl⟩
  | _, _, .tfold .., _, _, _, _ => ⟨rfl, rfl, rfl⟩
  | _, _, .latest .., _, _, _, _ => ⟨rfl, rfl, rfl⟩
  | _, _, .latest_monotone .., _, _, _, _ => ⟨rfl, rfl, rfl⟩
  | _, _, .time .., _, _, _, _ => ⟨rfl, rfl, rfl⟩
  | _, _, .snapshot_bounded .., _, _, _, _ => ⟨rfl, rfl, rfl⟩
  | _, _, .reduce_watermark .., _, _, _, _ => ⟨rfl, rfl, rfl⟩
  | _, _, .tick_scan .., _, _, _, _ => ⟨rfl, rfl, rfl⟩

theorem Dag.pass_locStep (m : Member L mem) (tk : Bool) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} →
    (d : Dag Γ outs) → (sch : Dag.Sched d) → (t : Nat) → (Δ : HList (Kind.locDelta m) Γ) →
    (s : Dag.LocState m d) →
    let ora := Dag.cursorsAt m d sch t s
    let s' := Dag.enqueueInbox m d (Dag.arrivalsOf m d ora s) (Dag.arrivedOnly m d s)
    let r := Dag.locStep m tk d ora Δ s
    let r' := Dag.locStep m tk d (Dag.oracleAll m d sch s') Δ s'
    r'.1 = Dag.arrivedOnly m d r.1 ∧ r'.2.1 = r.2.1 ∧ r'.2.2 = r.2.2
  | _, _, .ret _, _, _, _, _ => ⟨rfl, rfl, rfl⟩
  | _, _, .letN n rest, sch, t, Δ, s => by
    intro ora s' r r'
    obtain ⟨h1, h2, h3⟩ := Node.pass_locStep m tk n sch.1 t Δ s.1
    have ihr := Dag.pass_locStep m tk rest sch.2 t
      ((Node.locStep m tk n (Node.cursorsAt m n sch.1 t s.1) Δ s.1).2.1.append Δ) s.2
    obtain ⟨g1, g2, g3⟩ := ihr
    simp only [Dag.locStep, Dag.oracleAll, Dag.oracleOf, Dag.timingAt, Dag.enqueueInbox, Dag.arrivalsOf,
      Dag.arrivedOnly, Dag.cursorsAt, Node.memoState_eq, memoEnv_eq, s', r, r', ora]
    rw [h2]
    exact ⟨Prod.ext h1 g1, g2, Prod.ext h3 g3⟩

end

/-- The loop body on the loop cells, fed the cursor's arrivals and the schedule's
timing at the cells' tick counts (`timingAt` — the one seam between a process and the
lock-step machine), is the lock-step process step: same outputs, same sends, and its
cells are the step's loop cells (the step's cells with the in-flight queue — the
network's — removed). -/
theorem Dag.pass_eq (m : Member L mem) (tk : Bool) {Γ : Ctx L mem} {outs : List (Kind L mem)} (d : Dag Γ outs)
    (sch : Dag.Sched d) (t : Nat) (Δ : HList (Kind.locDelta m) Γ) (s : Dag.LocState m d) :
    Dag.pass m tk d (Dag.timingAt m d sch (Dag.arrivedOnly m d s))
        (Dag.arrivalsOf m d (Dag.cursorsAt m d sch t s) s) Δ (Dag.arrivedOnly m d s)
      = ((Dag.arrivedOnly m d (Dag.locStep m tk d (Dag.cursorsAt m d sch t s) Δ s).1),
         (Dag.locStep m tk d (Dag.cursorsAt m d sch t s) Δ s).2.1,
         (Dag.locStep m tk d (Dag.cursorsAt m d sch t s) Δ s).2.2) := by
  obtain ⟨h1, h2, h3⟩ := Dag.pass_locStep m tk d sch t Δ s
  unfold Dag.pass
  rw [Dag.memoState_eq,
    ← Dag.timingAt_enqueueInbox m d sch (Dag.arrivalsOf m d (Dag.cursorsAt m d sch t s) s) (Dag.arrivedOnly m d s)]
  exact Prod.ext h1 (Prod.ext h2 h3)

/-! ### A stutter does not read the timing (D93)

The timer nodes read their oracle entry only at a tick (`source_interval`, `time`:
`if tk then … else …`); a channel's oracle entry is its cursor, which `oracleOf` sets
from the cells. So a pass with no tick computes the same for any timing — the stutters
of an activation are handed the quieted timing (`Dag.Timing.quiet`, which the settling
ticks need), and the model may hand them anything. -/

mutual

theorem Node.locStep_false_timing (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} →
    (n : Node Γ κs) → (tm tm' : Node.Timing m n) → (Δ : HList (Kind.locDelta m) Γ) → (s : Node.LocState m n) →
    Node.locStep m false n (Node.oracleOf m n tm s) Δ s = Node.locStep m false n (Node.oracleOf m n tm' s) Δ s
  | _, _, .source_interval .., _, _, _, _ => rfl
  | _, _, .time .., _, _, _, _ => rfl
  | _, _, .call N _, tm, tm', Δ, s => Dag.locStep_false_timing m N tm tm' _ s
  | _, _, .fixN _ body, tm, tm', Δ, s => by
    show (let r := Dag.locStep m false body (Dag.oracleOf m body tm s.1) (s.2.append Δ) s.1
          ((r.1, r.2.1.take), s.2.append r.2.1.drop, r.2.2))
        = (let r := Dag.locStep m false body (Dag.oracleOf m body tm' s.1) (s.2.append Δ) s.1
          ((r.1, r.2.1.take), s.2.append r.2.1.drop, r.2.2))
    rw [Dag.locStep_false_timing m body tm tm' _ s.1]
  | _, _, .broadcast_closed .., _, _, _, _ => rfl
  | _, _, .demux .., _, _, _, _ => rfl
  | _, _, .map .., _, _, _, _ => rfl
  | _, _, .filterMap .., _, _, _, _ => rfl
  | _, _, .values .., _, _, _, _ => rfl
  | _, _, .into_keyed .., _, _, _, _ => rfl
  | _, _, .kassume_ordering .., _, _, _, _ => rfl
  | _, _, .kfold_early_stop .., _, _, _, _ => rfl
  | _, _, .kget_max_key .., _, _, _, _ => rfl
  | _, _, .weaken_retries .., _, _, _, _ => rfl
  | _, _, .union .., _, _, _, _ => rfl
  | _, _, .assume_ordering .., _, _, _, _ => rfl
  | _, _, .fold .., _, _, _, _ => rfl
  | _, _, .fold_monotone .., _, _, _, _ => rfl
  | _, _, .snapshot .., _, _, _, _ => rfl
  | _, _, .batch .., _, _, _, _ => rfl
  | _, _, .mapTick .., _, _, _, _ => rfl
  | _, _, .firstTick .., _, _, _, _ => rfl
  | _, _, .zipTick .., _, _, _, _ => rfl
  | _, _, .defer_tick .., _, _, _, _ => rfl
  | _, _, .allTicks .., _, _, _, _ => rfl
  | _, _, .flattenOrdered .., _, _, _, _ => rfl
  | _, _, .flattenUnordered .., _, _, _, _ => rfl
  | _, _, .tinto_keyed .., _, _, _, _ => rfl
  | _, _, .tassume_ordering .., _, _, _, _ => rfl
  | _, _, .kentries .., _, _, _, _ => rfl
  | _, _, .kmap .., _, _, _, _ => rfl
  | _, _, .kallTicks .., _, _, _, _ => rfl
  | _, _, .kfold .., _, _, _, _ => rfl
  | _, _, .ksnapshot .., _, _, _, _ => rfl
  | _, _, .tfold .., _, _, _, _ => rfl
  | _, _, .latest .., _, _, _, _ => rfl
  | _, _, .latest_monotone .., _, _, _, _ => rfl
  | _, _, .snapshot_bounded .., _, _, _, _ => rfl
  | _, _, .reduce_watermark .., _, _, _, _ => rfl
  | _, _, .tick_scan .., _, _, _, _ => rfl

theorem Dag.locStep_false_timing (m : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} →
    (d : Dag Γ outs) → (tm tm' : Dag.Timing m d) → (Δ : HList (Kind.locDelta m) Γ) → (s : Dag.LocState m d) →
    Dag.locStep m false d (Dag.oracleOf m d tm s) Δ s = Dag.locStep m false d (Dag.oracleOf m d tm' s) Δ s
  | _, _, .ret _, _, _, _, _ => rfl
  | _, _, .letN n rest, tm, tm', Δ, s => by
    have h1 := Node.locStep_false_timing m n tm.1 tm'.1 Δ s.1
    have h2 := Dag.locStep_false_timing m rest tm.2 tm'.2
    simp only [Dag.locStep, Dag.oracleOf, h1, h2]

end

/-- A pass without a tick is the same for any timing. -/
theorem Dag.pass_false_timing (m : Member L mem) {Γ : Ctx L mem} {outs : List (Kind L mem)} (d : Dag Γ outs)
    (tm tm' : Dag.Timing m d) (ib : Dag.Inbox m d) (Δ : HList (Kind.locDelta m) Γ) (s : Dag.LocState m d) :
    Dag.pass m false d tm ib Δ s = Dag.pass m false d tm' ib Δ s := by
  unfold Dag.pass
  exact Dag.locStep_false_timing m d tm tm' Δ _

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
  | _, _, .source_interval .., _, _ => rfl
  | _, _, .mapTick .., _, _ => rfl
  | _, _, .firstTick .., _, _ => rfl
  | _, _, .zipTick .., _, _ => rfl
  | _, _, .defer_tick .., _, _ => rfl
  | _, _, .allTicks .., _, _ => rfl
  | _, _, .flattenOrdered .., _, _ => rfl
  | _, _, .flattenUnordered .., _, _ => rfl
  | _, _, .tinto_keyed .., _, _ => rfl
  | _, _, .tassume_ordering .., _, _ => rfl
  | _, _, .kentries .., _, _ => rfl
  | _, _, .kmap .., _, _ => rfl
  | _, _, .kallTicks .., _, _ => rfl
  | _, _, .kfold .., _, _ => rfl
  | _, _, .ksnapshot .., _, _ => rfl
  | _, _, .tfold .., _, _ => rfl
  | _, _, .latest .., _, _ => rfl
  | _, _, .latest_monotone .., _, _ => rfl
  | _, _, .time .., _, _ => rfl
  | _, _, .snapshot_bounded .., _, _ => rfl
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

namespace Interleaving

variable {Γ : Ctx L mem} {outs : List (Kind L mem)} {d : Dag Γ outs} (I : Interleaving d)

/-- Process `m`'s loop cells at the start of step `k`: its lock-step view with the
in-flight queues removed. -/
def loopView (k : Nat) (m : Member L mem) : Dag.LocState m d :=
  Dag.arrivedOnly m d (I.view k m)

/-- The inbox of the process running at step `k`: per connection, the next messages
the schedule's cursor delivers (the in-flight prefix up to the cursor). -/
def arrivals (k : Nat) : Dag.Inbox (I.who k) d :=
  Dag.arrivalsOf (I.who k) d (I.oracle k) (I.view k (I.who k))

/-- The timing the process running at step `k` is handed: the schedule's scripts at its
loop cells' tick counts — what a shell whose per-tick readings form `I.sched`'s scripts
reads at this activation (D84). -/
def timing (k : Nat) : Dag.Timing (I.who k) d :=
  Dag.timingAt (I.who k) d I.sched (I.loopView k (I.who k))

/-- The running process's activation at step `k`: the loop body on its loop cells, fed
its timing, its inbox and its inputs. -/
def loopPass (k : Nat) :=
  Dag.pass (I.who k) (I.tick k) d (I.timing k) (I.arrivals k) (envRestrict (I.who k) (I.inp k))
    (I.loopView k (I.who k))

/-- **The loop body is the lock-step step**: same outputs, same sends, and its cells
are the next step's loop cells. -/
theorem loop_pass (k : Nat) :
    I.loopPass k = (I.loopView (k + 1) (I.who k), (I.active k).2.1, (I.active k).2.2) := by
  unfold loopPass arrivals timing loopView oracle
  rw [I.view_active]
  show Dag.pass (I.who k) (I.tick k) d _ _ _ (Dag.arrivedOnly (I.who k) d (I.view k (I.who k)))
    = (Dag.arrivedOnly (I.who k) d (Dag.enqueueLoc (I.who k) (I.who k) d (I.active k).2.2 (I.active k).1), _, _)
  rw [Dag.arrivedOnly_enqueueLoc]
  exact Dag.pass_eq (I.who k) (I.tick k) d I.sched k (envRestrict (I.who k) (I.inp k)) (I.view k (I.who k))

/-- **A process that is not running does not change.** -/
theorem loop_idle (k : Nat) (m : Member L mem) (hm : m ≠ I.who k) :
    I.loopView (k + 1) m = I.loopView k m := by
  unfold loopView
  rw [I.view_idle k m hm, Dag.arrivedOnly_enqueueLoc]

/-- The loop body's outputs are the lock-step step's. -/
theorem loopPass_outputs (k : Nat) : (I.loopPass k).2.1 = (I.active k).2.1 := by
  rw [loop_pass]

/-! ### What a process has output by a time -/

/-- The output increments process `m` has produced by step `T` — one per activation
of `m` before `T`, oldest first. -/
def outputsAt (T : Nat) (m : Member L mem) : List (HList (Kind.locDelta m) outs) :=
  (List.range T).filterMap fun k => if h : I.who k = m then some (h ▸ (I.loopPass k).2.1) else none

/-- The items a stream output `v` of process `m` has carried by step `T`, in order. -/
def streamOutputsAt (T : Nat) (m : Member L mem) {ℓ : L} {α : Type} {iα : DecidableEq α} {ord : StrOrd}
    {ret : Retries} (v : Var outs (.stream ℓ α iα ord ret)) (i : Hosted m ℓ) : List α :=
  (I.outputsAt T m).flatMap fun o => o.get v i

/-- An item a process has output by `T` was output by one of its activations before `T`. -/
theorem mem_streamOutputsAt {T : Nat} {m : Member L mem} {ℓ : L} {α : Type} {iα : DecidableEq α} {ord : StrOrd}
    {ret : Retries} {v : Var outs (.stream ℓ α iα ord ret)} {i : Hosted m ℓ} {x : α}
    (hx : x ∈ I.streamOutputsAt T m v i) :
    ∃ k, k < T ∧ ∃ h : I.who k = m, x ∈ ((I.loopPass k).2.1).get v (cast (by rw [h]) i) := by
  unfold streamOutputsAt outputsAt at hx
  obtain ⟨o, ho, hxo⟩ := List.mem_flatMap.mp hx
  obtain ⟨k, hk, hko⟩ := List.mem_filterMap.mp ho
  have hkT := List.mem_range.mp hk
  split at hko
  · rename_i h
    subst h
    cases hko
    exact ⟨k, hkT, rfl, hxo⟩
  · cases hko

/-! ### The run at the wire (D88)

The lock-step model moves a sender's sends into the receivers' queues as typed values
(`view_idle`: `enqueueLoc`). On the wire they are bytes: the sender's `wireEncode`,
filtered to what the receiver's nodes read (`wireFor`). `view_idle_wire` is the same
queue update from the bytes — through the codecs' round trip.

A **batched** interleaving is one whose arrivals at each step are the decode of some
batches of messages — per sender, the messages it has been handed from that connection
this activation. Its loop body at the wire is `Dag.passWire` on those batches;
`loop_pass_wire` is the theorem that it is the lock-step step. A `WireTrace`
(`WireTrace.lean`) induces one: its batches are the complete frames each activation reads
off its connections. -/

/-- The bytes step `k`'s activation puts on the wire. -/
def wireOut (cod : Dag.Codecs d) (k : Nat) : List WireMsg :=
  (Dag.wireEncode (I.who k) d cod 0 (I.active k).2.2).1

/-- **Every other process's queues grow by the decode of the active process's bytes**
(the frame on its connection: the sender's messages filtered to its nodes). -/
theorem view_idle_wire (cod : Dag.Codecs d) (k : Nat) (m : Member L mem) (hm : m ≠ I.who k) :
    I.view (k + 1) m
      = Dag.enqueueInbox m d (Dag.inboxOfWire m d cod 0 [⟨I.who k, Dag.wireFor m d (I.wireOut cod k)⟩]).1
          (I.view k m) := by
  rw [I.view_idle k m hm, Dag.enqueueLoc_eq, Dag.inboxOfWire_wireFor, wireOut, Dag.inboxOfWire_wireEncode_self]

/-- **The active process's own queues grow by the decode of its own bytes** (what it
sent to itself), after its step. -/
theorem view_active_wire (cod : Dag.Codecs d) (k : Nat) :
    I.view (k + 1) (I.who k)
      = Dag.enqueueInbox (I.who k) d
          (Dag.inboxOfWire (I.who k) d cod 0 [⟨I.who k, Dag.wireFor (I.who k) d (I.wireOut cod k)⟩]).1
          (I.active k).1 := by
  rw [I.view_active k, Dag.enqueueLoc_eq, Dag.inboxOfWire_wireFor, wireOut, Dag.inboxOfWire_wireEncode_self]

/-- A batched interleaving: at each step the running process's arrivals are the decode of
some batches `(sender, its messages)`. -/
structure Batched (cod : Dag.Codecs d) where
  batches : Nat → List ((src : Member L mem) × List WireMsg)
  arrivals_eq : ∀ k, I.arrivals k = (Dag.inboxOfWire (I.who k) d cod 0 (batches k)).1

namespace Batched

variable {I} {cod : Dag.Codecs d} (B : I.Batched cod)

/-- The running process's activation at step `k`, at the wire: `Dag.passWire` on its
loop cells, its timing, the batches. -/
def loopPassWire (k : Nat) :=
  Dag.passWire (I.who k) (I.tick k) d cod (I.timing k) (B.batches k) (envRestrict (I.who k) (I.inp k))
    (I.loopView k (I.who k))

/-- **The loop body at the wire is the lock-step step**: same cells next, same outputs,
and its bytes are the active step's. -/
theorem loop_pass_wire (k : Nat) :
    B.loopPassWire k = (I.loopView (k + 1) (I.who k), (I.active k).2.1, I.wireOut cod k) := by
  unfold loopPassWire Dag.passWire
  rw [← B.arrivals_eq k]
  have h := I.loop_pass k
  unfold loopPass at h
  rw [h]
  rfl

/-- The output increments process `m` has produced by step `T`, running at the wire. -/
def outputsAt (T : Nat) (m : Member L mem) : List (HList (Kind.locDelta m) outs) :=
  (List.range T).filterMap fun k => if h : I.who k = m then some (h ▸ (B.loopPassWire k).2.1) else none

/-- The items a stream output `v` of process `m` has carried by step `T`, running at the
wire. -/
def streamOutputsAt (T : Nat) (m : Member L mem) {ℓ : L} {α : Type} {iα : DecidableEq α} {ord : StrOrd}
    {ret : Retries} (v : Var outs (.stream ℓ α iα ord ret)) (i : Hosted m ℓ) : List α :=
  (B.outputsAt T m).flatMap fun o => o.get v i

/-- **What a process outputs at the wire is what it outputs in the lock-step model.** -/
theorem streamOutputsAt_eq (T : Nat) (m : Member L mem) {ℓ : L} {α : Type} {iα : DecidableEq α} {ord : StrOrd}
    {ret : Retries} (v : Var outs (.stream ℓ α iα ord ret)) (i : Hosted m ℓ) :
    B.streamOutputsAt T m v i = I.streamOutputsAt T m v i := by
  unfold streamOutputsAt Interleaving.streamOutputsAt outputsAt Interleaving.outputsAt
  congr 1
  apply List.filterMap_congr
  intro k _
  split
  · rw [B.loop_pass_wire, I.loopPass_outputs]
  · rfl

end Batched

end Interleaving

end Project

end Hydro
