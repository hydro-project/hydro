import Hydro.HydroTick
import Hydro.CoupleProj
import Hydro.Project.Model.WiredRun
import Hydro.Project.Model.DistributedModel
import Hydro.Project.Model.WireTrace
import Hydro.Project.Model.EnvOfInc
import Hydro.HydroStage

/-!
# Quickstart: Hello World

The smallest distributed program: a leader broadcasts a stream of payloads to a cluster of
workers. Its contract says every worker receives exactly the leader's payloads, and the proof is
`rfl`, because at `Values` the network's reordering and delay are in the stream's type, so the
transport changes no content. `hello_safe_wired` carries the contract to the processes that run
the program and exchange bytes, through the `SchedSem` lemma `hello_safe_sched`; the `#guard`
runs the program at `Values`.
-/

namespace Hydro

namespace Tutorials

variable {L : Type} {mem : L → Nat}

--[sched]--
/-- The program's one network edge: how far each worker has been handed the leader's
stream. `Unit` at `Values`; delivery cursors at `SchedSem`. -/
structure HelloSched (H : HydroSem L mem) (nWorkers nLeader : Nat) where
  toWorkers : H.TransportDec nWorkers nLeader
--[/sched]--

--[ensures]--
/-- What `hello_broadcast` ensures, over the `Values` denotation. -/
structure HelloEnsures {nLeader nWorkers : Nat} (payloads : Fin nLeader → Multiset Nat)
    (out : Fin nWorkers → Multiset Nat) : Prop where
  /-- Every worker receives exactly the payloads of every leader member. -/
  recv : ∀ j, out j = ((List.finRange nLeader).map payloads).sum
--[/ensures]--

--[program]--
hydro def hello_broadcast (H : HydroSem L mem) (leader workers : L)
    (payloads : H.Stream leader Nat .noOrder .exactlyOnce)
    (sched : HelloSched H (mem workers) (mem leader)) :
    H.Stream workers Nat .noOrder .exactlyOnce
  ensures out => HelloEnsures payloads out :=
  H.values (H.broadcast_closed sched.toWorkers payloads)
  prove
    recv := fun _ => rfl
--[/program]--

--[census]--
#nondet_census hello_broadcast (nondets := 0) (scheds := 1) (fuels := 0)
--[/census]--

/-! ## From the contract to the processes -/

--[sched-lemma]--
/-- The step-machine form of the contract, for any input history: a payload worker `j` holds by
step `T` was given to some leader member by `T`. The proof makes the four moves of every machine
theorem, then the input frozen at `T` (causality through the graph). -/
theorem hello_safe_sched {pacing : Pacing L mem} (leader workers : L)
    (h : (SchedSem L mem pacing).Stream leader Nat .noOrder .exactlyOnce)
    (sched : HelloSched (SchedSem L mem pacing) (mem workers) (mem leader))
    (T : Nat) (j : Fin (mem workers)) (x : Nat)
    (hx : x ∈ ((hello_broadcast (SchedSem L mem pacing) leader workers h sched) j).view T) :
    x ∈ ((List.finRange (mem leader)).map fun i => (↑((h i).view T) : Multiset Nat)).sum := by
  -- the input frozen at `T`, and causality through the graph
  let h' : (SchedSem L mem pacing).Stream leader Nat .noOrder .exactlyOnce :=
    fun i => (h i).truncate T
  let env : Project.HList (Project.Kind.sched (mem := mem))
      [Project.Kind.stream leader Nat inferInstance .noOrder .exactlyOnce] :=
    Project.HList.cons (f := Project.Kind.sched (mem := mem)) h Project.HList.nil
  let env' : Project.HList (Project.Kind.sched (mem := mem))
      [Project.Kind.stream leader Nat inferInstance .noOrder .exactlyOnce] :=
    Project.HList.cons (f := Project.Kind.sched (mem := mem)) h' Project.HList.nil
  have hagree : Project.HAgree T env env' := by
    refine ⟨fun i t ht => ?_, trivial⟩
    show (h i).view t = (h i).view (min t T)
    rw [Nat.min_eq_left ht]
  have hcaus := Project.Dag.denote_causal (pacing := pacing) (hello_broadcast.dag leader workers)
    (hello_broadcast.schedOf pacing leader workers sched) T env env' hagree
  rw [hello_broadcast.dag_denote, hello_broadcast.dag_denote] at hcaus
  have hout : ((hello_broadcast (SchedSem L mem pacing) leader workers h sched) j).view T
      = ((hello_broadcast (SchedSem L mem pacing) leader workers h' sched) j).view T :=
    hcaus.1 j T (Nat.le_refl T)
  rw [hout] at hx
  -- the pool above the frozen input
  let v : (Values L mem).Stream leader Nat .noOrder .exactlyOnce := .mk' fun i => ↑((h i).view T)
  have hc : ∀ t i, ListLe .noOrder .exactlyOnce ((h' i).view t) (v i) := fun t i =>
    Multiset.coe_le.mpr ((h i).mono_le (Nat.min_le_right t T)).sublist.subperm
  -- 1. run at the coupling and open the output corner's `cpl`
  have hcpl := (hello_broadcast (CoupleSem L mem pacing T T (Nat.le_refl T)) leader workers
      (CoStream.inputC h' v hc) ⟨CoDec.cursors sched.toWorkers⟩).cpl
    (hello_broadcast_co_wf₁ leader workers (CoStream.inputC h' v hc)
      ⟨CoDec.cursors sched.toWorkers⟩ trivial)
    T (by co_lag [hello_broadcast_co_lag₁]) j
  -- 2. and 3. rewrite `sr` and `rr` to the `SchedSem` and `Values` runs
  rw [hello_broadcast_co_sr₁, hello_broadcast_co_rr₁] at hcpl
  -- 4. apply the contract
  rw [← (hello_broadcast.ensures leader workers v ⟨()⟩).recv j]
  exact Multiset.mem_of_le hcpl (Multiset.mem_coe.mpr hx)
--[/sched-lemma]--

variable [DecidableEq L]

open Project

--[run]--
/-- The activation each process runs: `hello_broadcast.dag` staged into straight-line code. -/
hydro_run helloRun {L : Type} {mem : L → Nat} [DecidableEq L] (leader workers : L)
    (m : Member L mem) where
  member := m,
  graph := hello_broadcast.dag (mem := mem) leader workers,
  codecs := hello_broadcast.wireCodecs (mem := mem) leader workers

/-- The body a process runs at an activation. -/
def helloBody (leader workers : L) :
    WireTrace.Body (hello_broadcast.dag (mem := mem) leader workers) :=
  fun m r chans Δ s => helloRun leader workers m true r chans Δ s
--[/run]--

/-- An increment of a step history is part of its view at that step. -/
theorem mem_view_of_mem_incView {α : Type} (h : StepHist α) (k : Nat) {x : α}
    (hx : x ∈ incView h k) : x ∈ h.view k := by
  cases k with
  | zero => exact hx
  | succ k =>
    have hd : incView h (k + 1) = (h.view (k + 1)).drop (h.view k).length := by
      simp only [incView, StepHist.inc]
    rw [hd] at hx
    exact List.mem_of_mem_drop hx

--[injected]--
/-- The payloads a trace has handed leader member `i` at its own activations among the first
`T + 1` steps. -/
def helloInjected {leader workers : L}
    (τ : WireTrace (hello_broadcast.dag (mem := mem) leader workers))
    (i : Fin (mem leader)) (T : Nat) : Multiset Nat :=
  ↑(accum (fun t => ((envMask (τ.who t) (τ.inp t)).get Var.zero i).toList) T)
--[/injected]--

--[safe-wired]--
/-- **Hello World, as processes exchanging bytes.** For any trace at the wire, every payload a
worker process has output by step `T` was injected at a leader process by then. -/
theorem hello_safe_wired (leader workers : L)
    (τ : WireTrace (hello_broadcast.dag (mem := mem) leader workers))
    (T : Nat) (j : Fin (mem workers)) (x : Nat)
    (hx : x ∈ τ.streamOutputsAt (helloBody leader workers) T ⟨workers, j⟩ Var.zero ⟨j, rfl⟩) :
    x ∈ ((List.finRange (mem leader)).map fun i => helloInjected τ i T).sum := by
  -- the staged body equals the interpreter's; the trace's run equals its induced interleaving's
  have hbody : helloBody (mem := mem) leader workers
      = WireTrace.Body.ofCodecs (hello_broadcast.wireCodecs (mem := mem) leader workers) := by
    funext m r chans Δ s
    exact helloRun.eq leader workers m true r chans Δ s
  rw [hbody, τ.streamOutputsAt_eq (hello_broadcast.dag_valid leader workers)] at hx
  set I := τ.interleaving (hello_broadcast.wireCodecs (mem := mem) leader workers) with hI
  -- the activation that output it, and its outputs as the lock-step step's
  obtain ⟨k, hkT, _hk, hx⟩ := I.mem_streamOutputsAt hx
  rw [I.loopPass_outputs] at hx
  let ssched := hello_broadcast.recordOf I.pacing leader workers I.sched
  have hS : hello_broadcast.schedOf I.pacing leader workers ssched = I.sched :=
    hello_broadcast.schedOf_recordOf I.pacing leader workers I.sched
  have hout : ∀ k (i : Hosted (I.who k) workers),
      (I.active k).2.1.get Var.zero i
        = (incView ((hello_broadcast (SchedSem L mem I.pacing) leader workers
            (I.wires.get Var.zero) ssched) i.1) k).toArray := by
    intro k i
    rw [I.outputs (hello_broadcast.dag_valid leader workers) k]
    show (envRestrict (I.who k) (envIncAt (Dag.denote I.pacing
      (hello_broadcast.dag leader workers) I.sched I.wires) k)).get Var.zero i = _
    rw [← hS, hello_broadcast.dag_denote I.pacing leader workers ssched I.wires, envRestrict_get]
    rfl
  rw [hout k _] at hx
  -- the step-machine theorem at step `k`, and the injected payloads only grow
  have h := hello_safe_sched leader workers _ ssched k _ x
    (mem_view_of_mem_incView _ _ (List.mem_toArray.mp hx))
  refine Multiset.mem_of_le (List.sum_le_sum (fun i _ => ?_)) h
  have hv : ((I.wires.get Var.zero) i).view k
      = accum (fun t => ((envMask (τ.who t) (τ.inp t)).get Var.zero i).toList) k := rfl
  rw [hv]
  exact Multiset.coe_le.mpr (accum_prefix_le _ (Nat.le_of_lt hkT)).sublist.subperm
--[/safe-wired]--

/-! ## Running it -/

--[locs]--
/-- One leader, three workers. -/
inductive HelloLoc | leader | workers
  deriving DecidableEq

@[reducible] def helloMem : HelloLoc → Nat
  | .leader => 1
  | .workers => 3
--[/locs]--

--[run-values]--
-- at `Values`: worker 2's pool is the leader's
#guard hello_broadcast (Values HelloLoc helloMem) .leader .workers
    (.mk' fun _ => ({1, 2} : Multiset Nat)) ⟨()⟩ 2
  = ({1, 2} : Multiset Nat)
--[/run-values]--

end Tutorials

end Hydro
