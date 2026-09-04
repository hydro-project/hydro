import HydroLean.Programs.TwoPCProof
import HydroLean.Programs.IndexPayloads
import HydroLean.Programs.JoinResponses

/-!
# Axiom audit: goal (a)/(b) headline theorems

Every theorem below must report only Lean's standard axioms
(`propext`, `Classical.choice`, `Quot.sound`) — no `sorryAx`, no extra
axioms. (Goal (c)'s audit is `Programs/Paxos/AxCheck.lean`; the Flo
metatheory's is `Flo/AxiomCheck.lean`.)
-/

-- goal (a): collect_quorum, engine proof + typed-stage face
#print axioms HydroLean.Programs.collectQuorum_correct
#print axioms HydroLean.Programs.collectQuorum_deterministic
#print axioms HydroLean.Programs.collect_quorum_spec
#print axioms HydroLean.Programs.collectQuorum_minEqMax_correct

-- goal (b): two_pc over the shared quorum stage
#print axioms HydroLean.Programs.twoPC_committed_eq
#print axioms HydroLean.Programs.twoPC_unanimity
#print axioms HydroLean.Programs.twoPC_nodup
#print axioms HydroLean.Programs.twoPC_all_yes_deterministic
#print axioms HydroLean.Programs.twoPC_deterministic

-- goal (b): index_payloads / join_responses typed-stage faces
#print axioms HydroLean.Programs.index_payloads_no_reelection
#print axioms HydroLean.Programs.index_payloads_preservation
#print axioms HydroLean.Programs.index_payloads_slots_strictMono
#print axioms HydroLean.Programs.join_responses_spec
#print axioms HydroLean.Programs.joinResponses_correct
