import HydroLean.Programs.Paxos.Safety

/-!
# Axiom audit: the Paxos headline

`commit_agreement` and its spine — the module contracts it composes — must
depend only on the standard axioms (`propext`, `Classical.choice`,
`Quot.sound`) — no `sorryAx`, no extra axioms.
-/

namespace HydroLean.Programs.Paxos

#print axioms commit_agreement
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
