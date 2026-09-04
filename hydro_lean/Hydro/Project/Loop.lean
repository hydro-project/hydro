import Hydro.Project.WiredRun
import Hydro.Project.Process

/-!
# Hydro · Project — the loop-body run (D78)

An interleaving, read as a run of the processes' **loop bodies**
(`Dag.iterate`, `Project/Process.lean`). Process `m`'s loop cells at step `k`
are the lock-step cells with nothing in flight (`loopView`: the in-flight queues are
the network's); at its own step it is fed the messages the schedule's cursors deliver
next (`arrivals`) and runs one activation (`loopActive`). Then

* `loop_active`: the activation's cells, outputs and sends are the lock-step step's —
  `(I.active k)` — so every theorem about `I.active` is a theorem about the loop body;
* `loop_idle`: a process that is not running does not change (its peers' sends are in
  flight, not in its cells).

So a process is `Dag.iterate` iterated along its own activations, with inboxes
that are the next cursor-many messages of each connection — exactly what a TCP shell
that preserves per-connection order hands it. Nothing about the shell is assumed in
the statements: the inbox is *defined* from the senders' sends and the cursors.
-/

namespace Hydro

namespace Project

variable {L : Type} {mem : L → Nat} [DecidableEq L]

namespace Interleaving

variable {Γ : Ctx L mem} {outs : List (Kind L mem)} {d : Dag Γ outs} (I : Interleaving d)

/-- Process `m`'s loop cells at the start of step `k`: its lock-step view with the
in-flight queues removed. -/
def loopView (k : Nat) (m : Member L mem) : Dag.LocState m d :=
  Dag.arrivedOnly m d (I.view k m)

/-- The inbox of the process running at step `k`: per connection, the next messages
the schedule's cursor delivers (the in-flight prefix up to the cursor). -/
def arrivals (k : Nat) : Dag.Inbox (I.who k) d :=
  Dag.arrivalsOf (I.who k) d (Dag.cursorsAt (I.who k) d I.sched k) (I.view k (I.who k))

/-- The running process's activation at step `k`: the loop body on its loop cells, fed
its inbox and its inputs. -/
def loopActive (k : Nat) :=
  Dag.iterate (I.who k) (I.tick k) d I.sched (I.arrivals k) (envRestrict (I.who k) (I.inp k))
    (I.loopView k (I.who k))

/-- **The loop body is the lock-step step**: same outputs, same sends, and its cells
are the next step's loop cells. -/
theorem loop_active (k : Nat) :
    I.loopActive k = (I.loopView (k + 1) (I.who k), (I.active k).2.1, (I.active k).2.2) := by
  unfold loopActive arrivals loopView
  rw [I.view_active]
  show Dag.iterate (I.who k) (I.tick k) d I.sched _ _ (Dag.arrivedOnly (I.who k) d (I.view k (I.who k)))
    = (Dag.arrivedOnly (I.who k) d (Dag.enqueueLoc (I.who k) (I.who k) d (I.active k).2.2 (I.active k).1), _, _)
  rw [Dag.arrivedOnly_enqueueLoc]
  exact Dag.iterate_eq (I.who k) (I.tick k) d I.sched k (envRestrict (I.who k) (I.inp k)) (I.view k (I.who k))

/-- **A process that is not running does not change.** -/
theorem loop_idle (k : Nat) (m : Member L mem) (hm : m ≠ I.who k) :
    I.loopView (k + 1) m = I.loopView k m := by
  unfold loopView
  rw [I.view_idle k m hm, Dag.arrivedOnly_enqueueLoc]

/-- The loop body's outputs are the lock-step step's. -/
theorem loopActive_outputs (k : Nat) : (I.loopActive k).2.1 = (I.active k).2.1 := by
  rw [loop_active]

/-! ### What a process has output by a time -/

/-- The output increments process `m` has produced by step `T` — one per activation
of `m` before `T`, oldest first. -/
def outputsAt (T : Nat) (m : Member L mem) : List (HList (Kind.locDelta m) outs) :=
  (List.range T).filterMap fun k => if h : I.who k = m then some (h ▸ (I.loopActive k).2.1) else none

/-- The items a stream output `v` of process `m` has carried by step `T`, in order. -/
def streamOutputsAt (T : Nat) (m : Member L mem) {ℓ : L} {α : Type} {iα : DecidableEq α} {ord : StrOrd}
    {ret : Retries} (v : Var outs (.stream ℓ α iα ord ret)) (i : Hosted m ℓ) : List α :=
  (I.outputsAt T m).flatMap fun o => o.get v i

/-- An item a process has output by `T` was output by one of its activations before `T`. -/
theorem mem_streamOutputsAt {T : Nat} {m : Member L mem} {ℓ : L} {α : Type} {iα : DecidableEq α} {ord : StrOrd}
    {ret : Retries} {v : Var outs (.stream ℓ α iα ord ret)} {i : Hosted m ℓ} {x : α}
    (hx : x ∈ I.streamOutputsAt T m v i) :
    ∃ k, k < T ∧ ∃ h : I.who k = m, x ∈ ((I.loopActive k).2.1).get v (cast (by rw [h]) i) := by
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

end Interleaving

end Project

end Hydro
