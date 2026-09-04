import Hydro.Paxos.CoupleSafety
import Hydro.CoupleCheck
import Hydro.Paxos.CoupleWf
import Hydro.HydroGenCheck
import Hydro.HydroParamCheck
import Hydro.TwoPC
import Hydro.Project.Model.StepMain
import Hydro.Paxos.WiredSafety
import Hydro.Project.Fused
import Hydro.Paxos.PaxosBench

/-! Axiom audit of the headline theorems (build artifact; not part of the
library). The expected foundation is the standard three:
`propext`, `Classical.choice`, `Quot.sound`. -/

open Hydro

-- programs and their colocated contracts
#print axioms Hydro.paxos_core
#print axioms Hydro.paxos_core.a_log.inv
#print axioms Hydro.collect_quorum
#print axioms Hydro.collect_quorum_with_response
#print axioms Hydro.join_responses
#print axioms Hydro.two_pc
-- the benchmark stack (D82)
#print axioms Hydro.bench_client
#print axioms Hydro.compute_throughput_latency
#print axioms Hydro.aggregate_bench_results
#print axioms Hydro.Histogram.record_comm
#print axioms Hydro.Histogram.add_comm_ok
#print axioms Hydro.sequence_payloads
#print axioms Hydro.kv_replica
#print axioms Hydro.payloads_to_leader
#print axioms Hydro.paxos_with_client
#print axioms Hydro.paxos_with_client.ensures
#print axioms Hydro.acceptor_checkpoint_from_replicas
#print axioms Hydro.acceptor_checkpoint_from_replicas.ensures
#print axioms Hydro.checkpointQuorumMin_ascends
#print axioms Hydro.paxos_protocol
#print axioms Hydro.paxos_protocol.acceptor_checkpoint.inv
#print axioms Hydro.paxos_bench
#print axioms Hydro.paxos_bench.dag_denote
#print axioms Hydro.paxos_bench.schedOf_recordOf
#print axioms Hydro.paxos_with_client_mono₁
#print axioms Hydro.kv_replica_mono₁

-- the machine-run safety headlines (premise-free)
#print axioms Hydro.paxos_co_wf
#print axioms Hydro.paxos_co_cpl
#print axioms Hydro.paxos_safe_sched'
#print axioms Hydro.cq_safe_sched'
#print axioms Hydro.cq_safe_sched
#print axioms Hydro.two_pc_co_cpl
#print axioms Hydro.two_pc_safe_sched'
#print axioms Hydro.two_pc_once_sched'
#print axioms Hydro.two_pc_safe_sched
#print axioms Hydro.two_pc_once_sched

-- the executable-witness pin (D73): the compiled `MemberMap`
-- constructor is the denotational one
#print axioms Hydro.MemberMap.mkImpl_eq
#print axioms Hydro.MemberMap.mk'_eq_mkImpl

-- corner combinator theory
#print axioms Hydro.co_fix_cpl
#print axioms Hydro.co_tick_fix_cpl
#print axioms Hydro.cc_sr
#print axioms Hydro.cc_rr
#print axioms Hydro.cc_wf
#print axioms Hydro.cc_cpl

-- the generated naming surface (spot checks: the whole-program glue
-- stack and the deepest knot)
#print axioms Hydro.paxos_co_sr
#print axioms Hydro.paxos_co_rr
#print axioms Hydro.paxos_core_co_sr₂
#print axioms Hydro.paxos_core_co_rr₂
#print axioms Hydro.paxos_core_co_wf₂
#print axioms Hydro.paxos_core.sequencing_max_ballots_co_sr₁
#print axioms Hydro.paxos_core.sequencing_max_ballots_co_rr₁
#print axioms Hydro.paxos_core.sequencing_max_ballots_co_wf₁
#print axioms Hydro.leader_election.p_is_leader_co_wf₁
#print axioms Hydro.leader_election_co_rr₄
#print axioms Hydro.toy_safe_sched

/-! The relational layer (HydroRel/HydroParam): the generated free
theorems and the mono/causal instances. -/
#print axioms Hydro.leader_election.p_is_leader_param
#print axioms Hydro.leader_election.p1b_fail_param
#print axioms Hydro.leader_election.body_param₁
#print axioms Hydro.p_ballot_calc_param₁
#print axioms Hydro.monoLaws
#print axioms Hydro.causalLaws
#print axioms Hydro.paxos_core_param₁
#print axioms Hydro.paxos_core_param₂
#print axioms Hydro.leader_election_param₁

-- the production interpreter: graphs, their denotation lemmas, the lock-step run
#print axioms Hydro.paxos_core.dag_denote
#print axioms Hydro.two_pc.dag_denote
#print axioms Hydro.collect_quorum.dag_denote
-- the schedule is execution data (D77): every schedule of the graph is some bundles'
#print axioms Hydro.paxos_core.schedOf_recordOf
#print axioms Hydro.two_pc.schedOf_recordOf
#print axioms Hydro.Project.Dag.run_spec

-- the wired machine (D74/D76): every process steps its own cells, and the
-- interleaving's outputs are the denotation's — Paxos safety over TCP, for
-- every interleaving (non-ticking members are inert; drained cells carry)
#print axioms Hydro.Project.Dag.step_wired
#print axioms Hydro.Project.Dag.step_drained
#print axioms Hydro.Project.Interleaving.step_wired
#print axioms Hydro.Project.Interleaving.view_active
#print axioms Hydro.Project.Interleaving.view_idle
#print axioms Hydro.Project.Interleaving.outputs
#print axioms Hydro.paxos_safe_sched
#print axioms Hydro.paxos_safe_wired
-- the loop body (D78): the library's per-activation function is the lock-step step
#print axioms Hydro.Project.Dag.pass_eq
#print axioms Hydro.Project.Interleaving.loop_pass
#print axioms Hydro.Project.Interleaving.loop_idle
-- the activation (D93): a tick pass and then the settling passes (stutters while a knot
-- slot is pending, a further tick while a tick input holds items); a trace of activations
-- expands to a trace of passes, same cells, sends, outputs
#print axioms Hydro.Project.WireTrace.expands
#print axioms Hydro.Project.WireTrace.streamOutputsAt_expand
#print axioms Hydro.Project.WireTrace.streamOutputsAt_eq
#print axioms Hydro.Project.Dag.settleImpl_eq
#print axioms Hydro.Project.Dag.iterateBytes_eq_impl
#print axioms Hydro.Project.Dag.Timing.ofReadings_quiet

#print axioms Hydro.Project.envIncAt_envOfInc
#print axioms Hydro.Project.memoBox_eq_memoBoxImpl
#print axioms Hydro.Project.memoBox2_eq_memoBox2Impl

-- the hash-backed keyed operators (D75): the Rust's `K: Hash` at the operator,
-- the machine's list semantics unchanged, the implementations csimp seams
#print axioms Hydro.keyedFoldAssocH_eq_impl
#print axioms Hydro.listJoinH_eq_impl
#print axioms Hydro.listAntiJoin_eq_impl
#print axioms Hydro.listFilterNotIn_eq_impl
#print axioms Hydro.dedupH_eq_dedupImpl
#print axioms Hydro.reduceWatermarkEagerH_eq_impl
#print axioms Hydro.HMap.mem_foldlOn_insert_empty
#print axioms Hydro.HMap.coe_toList
