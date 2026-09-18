import HydroLean.Programs.Paxos.PaxosCore

/-!
# Axiom audit: the Paxos headline

The headline IS `paxos_core`'s type: the safety guarantee is a proof field
of the definition, so `#print axioms paxos_core` audits the verified face
itself. Every module's contract is likewise a ghost field of its
definition (`leader_election`, `sequence_payload`, `p_p1b`,
`acceptor_p1_ticks`, `acceptor_p2_ticks`, `p_ballot_calc`,
`p_leader_heartbeat`) — auditing the def audits the face. All must depend
only on the standard axioms (`propext`, `Classical.choice`, `Quot.sound`)
— no `sorryAx`, no extra axioms.
-/

namespace HydroLean.Programs.Paxos

#print axioms paxos_core
#print axioms fix_slot_functional
#print axioms emission_chosen_agree
#print axioms run_promise_covers
#print axioms sequence_payload
#print axioms leader_election
#print axioms p_p1b
#print axioms acceptor_p1_ticks
#print axioms acceptor_p2_ticks
#print axioms p_ballot_calc
#print axioms p_leader_heartbeat
#print axioms le_p1a_nodup

end HydroLean.Programs.Paxos
