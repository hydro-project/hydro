import Hydro.Paxos.CoupleWf
import Hydro.Project.WiredRun
import Hydro.Project.EnvOfInc

/-!
# Hydro · Paxos — safety of the wired processes

The wired headline (the machine-only headline `paxos_safe_sched` it rests on is in
`Paxos/CoupleWf.lean`):

* `paxos_safe_wired` — run `paxos_core.dag` as processes: every cluster member a process
  stepping `Dag.locStep` on its own cells and exchanging only `Sends`, under **any**
  interleaving, with checkpoints that only grow. Any two commits two processes ever
  emit at one slot agree. The chain:

    wired process step ＝ clocked step cut to the process (`Interleaving.outputs`)
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

/-- The input context of `paxos_core`: the client payloads at the proposers and the
checkpoint singleton at the acceptors. -/
abbrev PaxosCtx : Ctx L mem :=
  [Kind.stream prop P inferInstance .totalOrder .exactlyOnce,
   Kind.sing acc (Option Nat) (Option Nat) inferInstance .totalOrder .exactlyOnce (.monotonic ValueOrder.optNat)]

variable [DecidableEq L]

/-- The graph of `paxos_core` (guarded variant) the processes run: the elaborator's
compilation of the `hydro def` body, with the decision records baked in, at the
interleaving's pacing (the process running is the active member; it ticks or stutters
per `I.tick`). -/
abbrev paxosDag (I : Interleaving L mem (PaxosCtx (P := P) prop acc))
    (sdec : PaxosCoreDec (SchedSem L mem I.pacing) (mem prop) (mem acc) P (Option Nat) .totalOrder)
    (ssched : PaxosCoreSched (SchedSem L mem I.pacing) (mem prop) (mem acc) P) :=
  paxos_core.dag (ckret := .exactlyOnce) I.pacing .guarded prop acc f sdec ssched

/-- The checkpoint increments an interleaving feeds the acceptors only grow (the wire's
`Monotonic` typing, as a fact about the inputs: `Var.zero.succ` is the `a_checkpoint`
input, `Var.zero` the client payloads). -/
def Project.Interleaving.CkMono (I : Interleaving L mem (PaxosCtx (P := P) prop acc)) : Prop :=
  ∀ k (j : Fin (mem acc)), ValueOrder.optNat.le ((I.inp k).get Var.zero.succ j) ((I.inp (k + 1)).get Var.zero.succ j)

/-- **Wired Paxos safety.**

Setting: every cluster member is a process (`Member`: a cluster `ℓ` and an index); each
runs the graph `paxosDag` on its own cells — `Dag.locStep` — and they exchange only the
graph's network messages. An execution is an `Interleaving`: the linearization of the
asynchronous run — at each clocked step `k` one member (`I.who k`) runs: either a
*stutter* (its asynchronous dataflow progresses — deliveries land, knot feedback
propagates — without any tick body firing) or a *tick* (every `Tick` location of it at
once, `I.tick k`), and the inputs `I.inp k` arrive. `ssched` is the run's delivery
cursors and `sdec` its heartbeat timing — at `SchedSem` the content decisions (batch
cuts, order selections, knot fuel) are all `Unit`, derived from the schedule by the
transfer; both are baked into the graph as inputs.

Conclusion: two commits of the `sequenced` output at one slot — emitted by any two
proposers, at any two steps — carry the same value.
Client payloads, scheduling and decisions are unconstrained; nothing is assumed about
settling, quiescence or the sleeping processes. -/
theorem paxos_safe_wired
    -- Paxos's quorum condition: at most `f` of the acceptors may be missing from a quorum.
    (hnA : mem acc ≤ 2 * f + 1)
    -- The execution: which member runs at each clocked step, whether it ticks (else it
    -- stutters), and the inputs.
    (I : Interleaving L mem (PaxosCtx (P := P) prop acc))
    -- The run's timing: at `SchedSem` every content decision is `Unit` (derived from the
    -- schedule), so `sdec` is only the heartbeat timer/pulse verdicts (`le.hb`).
    (sdec : PaxosCoreDec (SchedSem L mem I.pacing) (mem prop) (mem acc) P (Option Nat) .totalOrder)
    -- The run's delivery cursors: how much of each sender's wire each receiver has taken
    -- at each step (clamped to what has arrived).
    (ssched : PaxosCoreSched (SchedSem L mem I.pacing) (mem prop) (mem acc) P)
    -- The checkpoint input never decreases (the `a_checkpoint` wire's `Monotonic` type;
    -- the acceptors' log GC is unsound without it — B4). Satisfied by the constant `none`.
    (hck : I.CkMono prop acc)
    -- Two steps of the run.
    {k k' : Nat}
    -- The proposer running at each step (`Hosted (I.sel k) prop`: a proposer index `i`
    -- with `I.who k = ⟨prop, i⟩`), a slot, two values.
    {i : Hosted (I.sel k) prop} {j : Hosted (I.sel k') prop} {s : Nat} {w w' : Option P}
    -- The two commits: `(I.active _ k).2.1` is the running process's output increments at
    -- step `k`; `Var.zero.succ` is the second output, the `sequenced` commit stream.
    (hw : (s, w) ∈ (I.active (paxosDag prop acc f I sdec ssched) k).2.1.get Var.zero.succ i)
    (hw' : (s, w') ∈ (I.active (paxosDag prop acc f I sdec ssched) k').2.1.get Var.zero.succ j) :
    w = w' := by
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
  have hout : ∀ k (i : Hosted (I.sel k) prop),
      (I.active (paxosDag prop acc f I sdec ssched) k).2.1.get Var.zero.succ i = incView (commits i.1) k := by
    intro k i
    rw [I.outputs _ env henv hinp k,
      paxos_core.dag_denote I.pacing .guarded prop acc f sdec ssched env, envRestrict_get]
    rfl
  rw [hout k i] at hw
  rw [hout k' j] at hw'
  have hw1 := ((commits i.1).mono_le (Nat.le_max_left k k')).subset (mem_view_of_mem_incView _ _ hw)
  have hw2 := ((commits j.1).mono_le (Nat.le_max_right k k')).subset (mem_view_of_mem_incView _ _ hw')
  exact paxos_safe_sched I.pacing prop acc f (max k k') hnA cpS ckS hckS sdec ssched i.1 j.1 s w w' hw1 hw2

end PaxosWired

end Hydro
