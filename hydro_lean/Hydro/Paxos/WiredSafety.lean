import Hydro.Paxos.CoupleWf
import Hydro.Project.WiredRun
import Hydro.Project.Loop
import Hydro.Project.EnvOfInc

/-!
# Hydro · Paxos — safety of the wired processes

The wired headline (the machine-only headline `paxos_safe_sched` it rests on is in
`Paxos/CoupleWf.lean`):

* `paxos_safe_wired` — run `paxos_core.dag` as processes: every cluster member a process
  stepping `Dag.locStep` on its own cells and exchanging only `Sends`, under **any**
  interleaving, with checkpoints that only grow. Any two commits two processes ever
  emit at one slot agree. The chain:

    the loop body `Dag.iterate` at an activation ＝ the process's lock-step step (`Interleaving.loop_active`)
    ＝ clocked step cut to the process (`Interleaving.outputs`)
    ＝ the machine denotation's increment (`Dag.run_spec`)
    ＝ `paxos_core` at `SchedSem` (`paxos_core.dag_denote`)
    ⊢ `paxos_safe_sched`.

  The interleaving's inputs are arbitrary per-step increments; the environment they are
  the increments of is recovered inside the proof (`envOfInc`). There is no settling or
  quiescence hypothesis: a member that is not active is inert in `SchedSem` (gated
  delivery, knot feedback and `defer_tick`), so a sleeping process is exactly an idle
  member of the lock-step machine; a running member ticks or stutters (`Pace`).
-/

namespace Hydro

open Project

variable {L : Type} {mem : L → Nat} {P : Type} [DecidableEq P]

/-- An increment of a step history is part of its view at that step. -/
theorem mem_view_of_mem_incView {α : Type} (h : StepHist α) (k : Nat) {x : α} (hx : x ∈ incView h k) :
    x ∈ h.view k := by
  cases k with
  | zero => exact hx
  | succ k =>
    have hd : incView h (k + 1) = (h.view (k + 1)).drop (h.view k).length := by
      simp only [incView, StepHist.inc]
    rw [hd] at hx
    exact List.mem_of_mem_drop hx

/-! ## The wired headline -/

section PaxosWired

variable (prop acc : L) (f : Nat)

variable [DecidableEq L]

/-- The graph of `paxos_core` (guarded variant) the processes run: the elaborator's
compilation of the `hydro def` body, per pacing. No decision or schedule is baked in
(D77): the delivery cursors and timer verdicts are the execution's (`Interleaving.sched`). -/
abbrev paxosDag :=
  paxos_core.dag (mem := mem) (P := P) (ckα := Option Nat) (ckord := .totalOrder) (ckret := .exactlyOnce) .guarded prop acc f

/-- The checkpoint increments an interleaving feeds the acceptors only grow (the wire's
`Monotonic` typing, as a fact about the inputs: `Var.zero.succ` is the `a_checkpoint`
input, `Var.zero` the client payloads). -/
def Project.Interleaving.CkMono (I : Interleaving (paxosDag (mem := mem) (P := P) prop acc f)) : Prop :=
  ∀ k (j : Fin (mem acc)), ValueOrder.optNat.le ((I.inp k).get Var.zero.succ j) ((I.inp (k + 1)).get Var.zero.succ j)

/-- **Wired Paxos safety.**

Setting: every cluster member is a process (`Member`: a cluster `ℓ` and an index); each
runs the graph `paxosDag` on its own cells — `Dag.locStep` — and they exchange only the
graph's network messages. An execution is an `Interleaving`: the linearization of the
asynchronous run — at each clocked step `k` one member (`I.who k`) runs: either a
*stutter* (its asynchronous dataflow progresses — deliveries land, knot feedback
propagates — without any tick body firing) or a *tick* (every `Tick` location of it at
once, `I.tick k`), the inputs `I.inp k` arrive, and the schedule `I.sched` says what
the network delivered (per edge, per receiver, per sender: the cursor at each step) and
what the timers said (per member: the heartbeat timeout/pulse verdicts at each tick).
Nothing else: at `SchedSem` every content decision (batch cuts, order selections, knot
fuel) is `Unit`, derived from the schedule by the transfer.

What a process is here: the loop body `Dag.iterate` (`Project/Process.lean` —
read the inbox, deliver everything, one step of the projected graph, send; what
`lake exe wired` runs) iterated along its own activations, with inboxes that are the
next cursor-many messages of each connection (`Interleaving.loop_active`/`loop_idle`).
`I.streamOutputsAt T m v i` is what process `m` has output on stream `v` by time `T`.

Conclusion: at any time `T`, the `sequenced` commit logs any two proposers have output
so far agree on every slot. Client payloads, scheduling and decisions are
unconstrained; nothing is assumed about settling, quiescence or the sleeping
processes. -/
theorem paxos_safe_wired
    -- Paxos's quorum condition: at most `f` of the acceptors may be missing from a quorum.
    (hnA : mem acc ≤ 2 * f + 1)
    -- The execution: which member runs at each clocked step, whether it ticks (else it
    -- stutters), the inputs, and the schedule — the network's delivery cursors per edge /
    -- receiver / sender / step (clamped to what has arrived) and the timers' verdicts.
    (I : Interleaving (paxosDag (mem := mem) (P := P) prop acc f))
    -- The checkpoint input never decreases (the `a_checkpoint` wire's `Monotonic` type;
    -- the acceptors' log GC is unsound without it — B4). Satisfied by the constant `none`.
    (hck : I.CkMono prop acc)
    -- A time, two proposers, a slot, two values.
    (T : Nat) (i j : Fin (mem prop)) {s : Nat} {w w' : Option P}
    -- The two commits: what proposer `i` (the process `⟨prop, i⟩`) has output on
    -- `Var.zero.succ` — the second output, the `sequenced` commit stream — by time `T`.
    (hw : (s, w) ∈ I.streamOutputsAt T ⟨prop, i⟩ Var.zero.succ ⟨i, rfl⟩)
    (hw' : (s, w') ∈ I.streamOutputsAt T ⟨prop, j⟩ Var.zero.succ ⟨j, rfl⟩) :
    w = w' := by
  -- the activations that output them, and their outputs as the lock-step step's
  obtain ⟨k, _, hk, hw⟩ := I.mem_streamOutputsAt hw
  obtain ⟨k', _, hk', hw'⟩ := I.mem_streamOutputsAt hw'
  rw [I.loopActive_outputs] at hw hw'
  -- the schedule, as the program's decision and schedule bundles (`recordOf`: every
  -- schedule of the graph is some bundles')
  let sdec := (paxos_core.recordOf I.pacing .guarded prop acc f I.sched).1
  let ssched := (paxos_core.recordOf I.pacing .guarded prop acc f I.sched).2
  have hS : paxos_core.schedOf I.pacing .guarded prop acc f sdec ssched = I.sched :=
    paxos_core.schedOf_recordOf I.pacing .guarded prop acc f I.sched
  -- the environment the interleaving's consumed increments are the increments of
  let env := envOfInc I.env
  have hinp : I.env = fun k => envIncAt env k := funext fun k => (envIncAt_envOfInc I.env k).symm
  have henv : HTMono env := HTMono_envOfInc I.env
  let cpS := env.get Var.zero
  let ckS : CkInput I.pacing acc := env.get Var.zero.succ
  have hckS : CkMono I.pacing acc ckS := fun j t => by
    show ValueOrder.optNat.le ((I.env t).get Var.zero.succ j) ((I.env (t + 1)).get Var.zero.succ j)
    exact hck t j
  -- the commit stream of the machine run
  let commits : Fin (mem prop) → StepHist (Nat × Option P) :=
    (paxos_core (SchedSem L mem I.pacing) .guarded prop acc f cpS ckS sdec ssched).2
  have hout : ∀ k (i : Hosted (I.who k) prop),
      (I.active k).2.1.get Var.zero.succ i = incView (commits i.1) k := by
    intro k i
    rw [I.outputs env henv hinp k]
    show (envRestrict (I.who k) (envIncAt (Dag.denote I.pacing (paxosDag prop acc f) I.sched env) k)).get
      Var.zero.succ i = _
    rw [← hS, paxos_core.dag_denote I.pacing .guarded prop acc f sdec ssched env, envRestrict_get]
    rfl
  rw [hout k _] at hw
  rw [hout k' _] at hw'
  have hw1 := ((commits _).mono_le (Nat.le_max_left k k')).subset (mem_view_of_mem_incView _ _ hw)
  have hw2 := ((commits _).mono_le (Nat.le_max_right k k')).subset (mem_view_of_mem_incView _ _ hw')
  exact paxos_safe_sched I.pacing prop acc f (max k k') hnA cpS ckS hckS sdec ssched _ _ s w w' hw1 hw2

end PaxosWired

end Hydro
