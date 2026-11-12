import Hydro
import Hydro.Couple
import Hydro.Graph.Causal
import Hydro.HMap
import Hydro.HydroGenCheck
import Hydro.HydroStageCheck
import Hydro.KeyedHash
import Hydro.MemberMap
import Hydro.Project.Fused
import Hydro.Project.Model.DistributedModel
import Hydro.Project.Model.EnvOfInc
import Hydro.Project.Model.StepAgree
import Hydro.Project.Model.StepMain
import Hydro.Project.Model.SweepSpec
import Hydro.Project.Model.WireTrace
import Hydro.Project.Model.WiredMain
import Hydro.Project.Model.WiredRun
import Hydro.Project.Wired
import Hydro.Std.BenchClient
import Hydro.Std.Quorum
import Hydro.Std.RequestResponse

/-! The axiom audit of the framework's headline theorems (the standard library, the graph
layer, the wired machine, the `@[csimp]` seams, the toys): a build artifact, not part of the
library. The protocols' headlines are audited in `Examples/AxCheck.lean`. Every declaration
below must report a subset of the standard foundation `propext`, `Classical.choice`,
`Quot.sound`; the gate greps the build log for these lines, and `scripts/reach.lean` reads the
audited names from this file's text. The section comments are the authoritative list of what
is audited. Add each new headline here in the same change that introduces it. -/

open Hydro

-- the programs and their colocated contracts
#print axioms Hydro.collect_quorum
#print axioms Hydro.collect_quorum_with_response
#print axioms Hydro.join_responses
-- the benchmark stack
#print axioms Hydro.bench_client
#print axioms Hydro.compute_throughput_latency
#print axioms Hydro.aggregate_bench_results
#print axioms Hydro.Histogram.record_comm
#print axioms Hydro.Histogram.add_comm_ok

-- the machine-run safety headlines, premise-free
#print axioms Hydro.cq_safe_sched'
#print axioms Hydro.cq_safe_sched

-- the staged activation's equation to the interpreter, at the toy (the protocols' are in
-- `Examples/AxCheck.lean`)
#print axioms Hydro.StageCheck.toyRun.eq

-- the executable-witness pin: the compiled `MemberMap` constructor equals the denotational one
#print axioms Hydro.MemberMap.mkImpl_eq
#print axioms Hydro.MemberMap.mk'_eq_mkImpl

-- the corner combinator theory: the knot couplings
#print axioms Hydro.co_fix_cpl
#print axioms Hydro.co_tick_fix_cpl

-- the generator's toy, end to end
#print axioms Hydro.toy_safe_sched

-- the production interpreter: the graphs, their denotation lemmas, the lock-step run
#print axioms Hydro.collect_quorum.dag_denote
-- the schedule is execution data: every schedule of the graph is some bundles'
#print axioms Hydro.Project.Dag.run_spec

-- the wired machine: every process steps its own cells, and the interleaving's outputs are
-- the denotation's, so a program's safety holds over TCP for every interleaving (non-ticking
-- members are inert; drained cells carry)
#print axioms Hydro.Project.Dag.step_wired
#print axioms Hydro.Project.Dag.step_drained
#print axioms Hydro.Project.Interleaving.step_wired
#print axioms Hydro.Project.Interleaving.view_active
#print axioms Hydro.Project.Interleaving.view_idle
#print axioms Hydro.Project.Interleaving.outputs
-- the loop body: the library's per-activation function `Dag.pass` matches the lock-step step
-- `Dag.locStep` in outputs, sends and loop cells
#print axioms Hydro.Project.Dag.pass_eq
#print axioms Hydro.Project.Interleaving.loop_pass
#print axioms Hydro.Project.Interleaving.loop_idle
-- the activation: one tick, in which the knot wires are their bodies' outputs of the same
-- tick. It is computed as a sweep (`Dag.iterate`) and equals the pass on a valid graph: every
-- knot body is strict in its knot wires (the lag analysis, `Dag.denote_agreeL`), and the
-- lock-step machine's knot outputs do not read the knot increments (`Dag.step_agree0`); a
-- trace of activations is a trace of ticking passes with the same cells, sends and outputs
#print axioms Hydro.Project.Dag.denote_agreeL
#print axioms Hydro.Project.Dag.step_agree0
#print axioms Hydro.Project.Dag.iterate_eq_pass
-- the byte-level run (`Dag.iterateBytes`, bytes per connection) equal to its implementation
#print axioms Hydro.Project.WireTrace.streamOutputsAt_eq
#print axioms Hydro.Project.Dag.iterateBytes_eq_impl

-- the input environment of the lock-step model and the wired machine's memo `@[csimp]` seams
#print axioms Hydro.Project.envIncAt_envOfInc
#print axioms Hydro.Project.memoBox_eq_memoBoxImpl
#print axioms Hydro.Project.memoBox2_eq_memoBox2Impl

-- the hash-backed keyed operators: the Rust's `K: Hash` at the operator, the machine's list
-- semantics unchanged, the implementations `@[csimp]` seams
#print axioms Hydro.keyedFoldAssocH_eq_impl
#print axioms Hydro.arrJoinH_eq_impl
#print axioms Hydro.keyedFoldArrH_eq_impl
#print axioms Hydro.arrAntiJoin_eq_impl
#print axioms Hydro.arrFilterNotIn_eq_impl
#print axioms Hydro.dedupH_eq_dedupImpl
#print axioms Hydro.reduceWatermarkEagerH_eq_impl
#print axioms Hydro.HMap.mem_foldlOn_insert_empty
#print axioms Hydro.HMap.coe_toList
