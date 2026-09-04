import Hydro.Paxos.CoupleWf
import Hydro.Project.Model.WiredRun
import Hydro.Project.Model.DistributedModel
import Hydro.Project.Model.WireTrace
import Hydro.Project.Model.EnvOfInc

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

  The interleaving's inputs are arbitrary per-step increments; the machine runs on them
  as wires (`I.wires`, monotone and single-slot by construction). There is no settling or
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

variable (prop acc : L) (config : PaxosConfig)

variable [DecidableEq L]

/-- The graph of `paxos_core` (guarded variant) the processes run: the elaborator's
compilation of the `hydro def` body, per pacing. No decision or schedule is baked in
(D77): the delivery cursors and timer verdicts are the execution's (`Interleaving.sched`). -/
abbrev paxosDag :=
  paxos_core.dag (mem := mem) (P := P) (ckα := Option Nat) (ckord := .totalOrder) (ckret := .exactlyOnce) .guarded prop acc config

/-- The checkpoint increments an interleaving feeds the acceptors only grow (the wire's
`Monotonic` typing, as a fact about the inputs: `Var.zero.succ` is the `a_checkpoint`
input, `Var.zero` the client payloads). -/
def Project.Interleaving.CkMono (I : Interleaving (paxosDag (mem := mem) (P := P) prop acc config)) : Prop :=
  ∀ k (j : Fin (mem acc)), ValueOrder.optNat.le ((I.inp k).get Var.zero.succ j) ((I.inp (k + 1)).get Var.zero.succ j)

/-- The codecs of `paxos_core`'s network nodes: the elaborator's `wireCodecs`, from the
`Wire` instances of `Ballot`, `P1b`, `P2a`, `P2b` and the payload/checkpoint types. -/
abbrev paxosCodecs [Wire P] : Dag.Codecs (paxosDag (mem := mem) (P := P) prop acc config) :=
  paxos_core.wireCodecs (mem := mem) (P := P) (ckα := Option Nat) (ckord := .totalOrder) (ckret := .exactlyOnce)
    .guarded prop acc config

/-- **Wired Paxos safety.**

The statement quantifies over what a deployment produces: a **trace at the bytes**
(`WireTrace`, D90) — which process activated when, with which inputs and clock/timer
readings, and how many bytes it read off each of its connections. A connection is a byte
queue (TCP) read in prefixes, cut anywhere; nothing else is assumed — every such trace is a
run. Every cluster member is a process running the production loop body
`Dag.iterateBytes` (the bytes in, cut into frames; the frames out — what `lake exe wired`
runs) on exactly those bytes (`WireTrace.act`), with the codecs `paxosCodecs` built from
the message types' `Wire` instances. Its outputs are the body's outputs
(`WireTrace.streamOutputsAt`). Crash-stop is a trace that stops activating a process.

No interleaving, schedule or cursor appears in the statement. The proof induces them: an
activation is a tick pass and then stutter passes until the process's tick-local knots are
quiet (`Dag.iterate`, D93), so a trace expands pass by pass to a trace of passes
(`WireTrace.expand`, `WireTrace.streamOutputsAt_expand`); that is a batched interleaving
(`PassTrace.interleaving`, `PassTrace.batched`) whose run at the wire is the trace's
(`PassTrace.streamOutputsAt_eq`, through the framing law `Wire.framesFrom_prefix_frameAll`,
the codecs' round trips `Codec.dec_enc` and the one-walk inbox `Dag.inboxOfWire_wireEncode`);
at the interleaving the active process's outputs are the lock-step machine's
(`Interleaving.outputs`, `Dag.run_spec`), the machine's run is `paxos_core`'s denotation
under the induced schedule (`paxos_core.dag_denote`, `recordOf`), and `paxos_safe_sched`
closes.

Conclusion: at any time `T`, the `sequenced` commit logs any two proposers have output
along the trace agree on every slot. Client payloads, scheduling and the shell's timing are
unconstrained; nothing is assumed about settling, quiescence or the sleeping processes. -/
theorem paxos_safe_wired [Wire P]
    -- Paxos's quorum condition: at most `f` of the acceptors may be missing from a quorum.
    (hnA : mem acc ≤ 2 * config.f + 1)
    -- The trace at the wire.
    (τ : WireTrace (paxosDag (mem := mem) (P := P) prop acc config))
    -- The checkpoint input never decreases (the `a_checkpoint` wire's `Monotonic` type;
    -- the acceptors' log GC is unsound without it — B4). Satisfied by the constant `none`.
    (hck : (τ.interleaving (paxosCodecs prop acc config)).CkMono prop acc)
    -- A time, two proposers, a slot, two values.
    (T : Nat) (i j : Fin (mem prop)) {s : Nat} {w w' : Option P}
    -- The two commits: what proposer `i` (the process `⟨prop, i⟩`) has output on
    -- `Var.zero.succ` — the second output, the `sequenced` commit stream — by time `T`.
    (hw : (s, w) ∈ τ.streamOutputsAt (paxosCodecs prop acc config) T ⟨prop, i⟩ Var.zero.succ ⟨i, rfl⟩)
    (hw' : (s, w') ∈ τ.streamOutputsAt (paxosCodecs prop acc config) T ⟨prop, j⟩ Var.zero.succ ⟨j, rfl⟩) :
    w = w' := by
  -- the trace's run is the induced interleaving's
  rw [τ.streamOutputsAt_eq] at hw hw'
  set I := τ.interleaving (paxosCodecs prop acc config) with hI
  -- the activations that output them, and their outputs as the lock-step step's
  obtain ⟨k, _, hk, hw⟩ := I.mem_streamOutputsAt hw
  obtain ⟨k', _, hk', hw'⟩ := I.mem_streamOutputsAt hw'
  rw [I.loopPass_outputs] at hw hw'
  -- the schedule, as the program's decision and schedule bundles (`recordOf`: every
  -- schedule of the graph is some bundles')
  let sdec := (paxos_core.recordOf I.pacing .guarded prop acc config I.sched).1
  let ssched := (paxos_core.recordOf I.pacing .guarded prop acc config I.sched).2
  have hS : paxos_core.schedOf I.pacing .guarded prop acc config sdec ssched = I.sched :=
    paxos_core.schedOf_recordOf I.pacing .guarded prop acc config I.sched
  -- the interleaving's inputs as the machine's wires
  let env := I.wires
  let cpS := env.get Var.zero
  let ckS : CkInput I.pacing acc := env.get Var.zero.succ
  have hckS : CkMono I.pacing acc ckS := fun j t => by
    show ValueOrder.optNat.le ((I.env t).get Var.zero.succ j) ((I.env (t + 1)).get Var.zero.succ j)
    exact hck t j
  -- the commit stream of the machine run
  let commits : Fin (mem prop) → StepHist (Nat × Option P) :=
    (paxos_core (SchedSem L mem I.pacing) .guarded prop acc config cpS ckS sdec ssched).2
  have hout : ∀ k (i : Hosted (I.who k) prop),
      (I.active k).2.1.get Var.zero.succ i = incView (commits i.1) k := by
    intro k i
    rw [I.outputs k]
    show (envRestrict (I.who k) (envIncAt (Dag.denote I.pacing (paxosDag prop acc config) I.sched env) k)).get
      Var.zero.succ i = _
    rw [← hS, paxos_core.dag_denote I.pacing .guarded prop acc config sdec ssched env, envRestrict_get]
    rfl
  rw [hout k _] at hw
  rw [hout k' _] at hw'
  have hw1 := ((commits _).mono_le (Nat.le_max_left k k')).subset (mem_view_of_mem_incView _ _ hw)
  have hw2 := ((commits _).mono_le (Nat.le_max_right k k')).subset (mem_view_of_mem_incView _ _ hw')
  exact paxos_safe_sched I.pacing prop acc config (max k k') hnA cpS ckS hckS sdec ssched _ _ s w w' hw1 hw2

end PaxosWired

end Hydro
