import HydroV2.Paxos.PaxosCore

/-! Axiom audit of the V2 headline (build artifact; not part of the
library). The expected foundation is the standard three:
`propext`, `Classical.choice`, `Quot.sound`. -/

open HydroV2

#print axioms HydroV2.paxos_core
#print axioms HydroV2.paxos_core_agree
