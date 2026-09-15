import HydroLean.Programs.Paxos.PaxosCore

/-!
# Axiom audit: the Paxos headline

The headline IS `paxos_core`'s type: the safety guarantee is a proof field
of the definition, so `#print axioms paxos_core` audits the verified face
itself. It, the assembly spine (`fix_slot_functional`,
`emission_chosen_agree`), and the module contracts they compose must
depend only on the standard axioms (`propext`, `Classical.choice`,
`Quot.sound`) — no `sorryAx`, no extra axioms.
-/

namespace HydroLean.Programs.Paxos

#print axioms paxos_core
#print axioms fix_slot_functional
#print axioms emission_chosen_agree
#print axioms sp_commit_spec
#print axioms sp_log_entry_spec
#print axioms sp_emission_spec
#print axioms sp_emission_functional
#print axioms le_ballot_stable
#print axioms le_leader_providers
#print axioms le_view_pinned
#print axioms run_promise_covers
#print axioms le_p1a_nodup

end HydroLean.Programs.Paxos
