import Hydro.Paxos.EagerCheck
import Hydro.Paxos.CoupleSafety
import Hydro.CoupleCheck
import Hydro.Paxos.CoupleWf
import Hydro.HydroGenCheck
import Hydro.HydroParamCheck

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

-- the machine-run safety headlines (premise-free)
#print axioms Hydro.paxos_co_wf
#print axioms Hydro.paxos_co_cpl
#print axioms Hydro.paxos_safe_sched'
#print axioms Hydro.cq_safe_sched'

-- the eager-variant headlines
#print axioms Hydro.paxos_eager_den
#print axioms Hydro.paxos_eager_den_ballots
#print axioms Hydro.paxos_eager_commits

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
theorems and the eager-agreement instance. -/
#print axioms Hydro.leader_election.p_is_leader_param
#print axioms Hydro.leader_election.p1b_fail_param
#print axioms Hydro.leader_election.body_param₁
#print axioms Hydro.p_ballot_calc_param₁
#print axioms Hydro.eagLaws
#print axioms Hydro.monoLaws
#print axioms Hydro.causalLaws
#print axioms Hydro.paxos_core_param₁
#print axioms Hydro.paxos_core_param₂
#print axioms Hydro.leader_election_param₁
