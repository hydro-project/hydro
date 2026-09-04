import HydroV2.Paxos.EagerCheck
import HydroV2.Paxos.CoupleSafety
import HydroV2.CoupleCheck
import HydroV2.Paxos.CoupleWf
import HydroV2.HydroGenCheck
import HydroV2.HydroParamCheck

/-! Axiom audit of the V2 headline (build artifact; not part of the
library). The expected foundation is the standard three:
`propext`, `Classical.choice`, `Quot.sound`. -/

open HydroV2

-- programs and their colocated contracts
#print axioms HydroV2.paxos_core
#print axioms HydroV2.paxos_core_agree
#print axioms HydroV2.collect_quorum
#print axioms HydroV2.collect_quorum_with_response
#print axioms HydroV2.join_responses

-- the machine-run safety headlines (premise-free)
#print axioms HydroV2.paxos_co_wf
#print axioms HydroV2.paxos_co_cpl
#print axioms HydroV2.paxos_safe_sched'
#print axioms HydroV2.cq_safe_sched'

-- the eager-variant headlines
#print axioms HydroV2.paxos_eager_den
#print axioms HydroV2.paxos_eager_den_ballots
#print axioms HydroV2.paxos_eager_commits

-- corner combinator theory
#print axioms HydroV2.co_fix_cpl
#print axioms HydroV2.co_tick_fix_cpl
#print axioms HydroV2.cc_sr
#print axioms HydroV2.cc_rr
#print axioms HydroV2.cc_wf
#print axioms HydroV2.cc_cpl

-- the generated naming surface (spot checks: the whole-program glue
-- stack and the deepest knot)
#print axioms HydroV2.paxos_co_sr
#print axioms HydroV2.paxos_co_rr
#print axioms HydroV2.paxos_core_co_sr₂
#print axioms HydroV2.paxos_core_co_rr₂
#print axioms HydroV2.paxos_core_co_wf₂
#print axioms HydroV2.paxos_core.sequencing_max_ballots_co_sr₁
#print axioms HydroV2.paxos_core.sequencing_max_ballots_co_rr₁
#print axioms HydroV2.paxos_core.sequencing_max_ballots_co_wf₁
#print axioms HydroV2.leader_election.p_is_leader_co_wf₁
#print axioms HydroV2.leader_election_co_rr₄
#print axioms HydroV2.toy_safe_sched

/-! The relational layer (HydroRel/HydroParam): the generated free
theorems and the eager-agreement instance. -/
#print axioms HydroV2.leader_election.p_is_leader_param
#print axioms HydroV2.leader_election.p1b_fail_param
#print axioms HydroV2.leader_election.body_param₁
#print axioms HydroV2.p_ballot_calc_param₁
#print axioms HydroV2.eagLaws
#print axioms HydroV2.monoLaws
#print axioms HydroV2.causalLaws
#print axioms HydroV2.paxos_core_param₁
#print axioms HydroV2.paxos_core_param₂
#print axioms HydroV2.leader_election_param₁
