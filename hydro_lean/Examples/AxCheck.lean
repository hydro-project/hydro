import Examples.Paxos.CoupleSafety
import Examples.Paxos.CoupleWf
import Examples.TwoPC
import Examples.Paxos.WiredSafety
import Examples.Paxos.PaxosBench
import Examples.Tutorials.HelloWorld
import Examples.Tutorials.Quorum

/-! The axiom audit of the example protocols' headline theorems (Paxos, 2PC, the quickstart
tutorials): a build artifact, not part of the library. The framework's headlines are audited
in `Hydro/AxCheck.lean`. Every declaration below must report a subset of the standard
foundation `propext`, `Classical.choice`, `Quot.sound`; the gate greps the build log for these
lines, and `scripts/reach.lean` reads the audited names from this file's text. The section
comments are the authoritative list of what is audited. Add each new headline here in the same
change that introduces it. -/

open Hydro

-- the programs and their colocated contracts
#print axioms Hydro.paxos_core
#print axioms Hydro.paxos_core.a_log.inv
#print axioms Hydro.two_pc
-- the benchmark stack
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

-- the machine-run safety headlines, premise-free
#print axioms Hydro.paxos_co_wf
#print axioms Hydro.paxos_co_cpl
#print axioms Hydro.paxos_safe_sched'
#print axioms Hydro.two_pc_co_cpl
#print axioms Hydro.two_pc_safe_sched'
#print axioms Hydro.two_pc_once_sched'
#print axioms Hydro.two_pc_safe_sched
#print axioms Hydro.two_pc_once_sched

-- the staged activations' equations to the interpreter: the headline's body (`paxosRun`,
-- which `paxos_safe_wired` reaches through `paxosBody`); `BenchDemo.benchRun.eq` lives in an
-- executable module, outside this library
#print axioms Hydro.paxosRun.eq
#print axioms Hydro.paxosBody_eq

-- the generated `sr`/`rr` equations and `wf` theorems, spot-checked at the whole-program glue
-- stack and the deepest knot
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

-- the production interpreter: the graphs, their denotation lemmas, the lock-step run
#print axioms Hydro.paxos_core.dag_denote
#print axioms Hydro.two_pc.dag_denote
-- the schedule is execution data: every schedule of the graph is some bundles'
#print axioms Hydro.paxos_core.schedOf_recordOf
#print axioms Hydro.two_pc.schedOf_recordOf

-- the wired machine (its theorems are in `Hydro/AxCheck.lean`): Paxos safety holds over TCP
-- for every interleaving
#print axioms Hydro.paxos_safe_sched
#print axioms Hydro.paxos_safe_wired
-- the generated validity and knot strictness of the headline program
#print axioms Hydro.paxos_core.dag_valid
#print axioms Hydro.leader_election.dag_valid
#print axioms Hydro.leader_election.p_is_leader_strict₁
-- the quickstart tutorials: contracts at `Values`, the step-machine lemmas, the wired headlines
#print axioms Hydro.Tutorials.hello_broadcast.ensures
#print axioms Hydro.Tutorials.hello_safe_sched
#print axioms Hydro.Tutorials.helloRun.eq
#print axioms Hydro.Tutorials.hello_safe_wired
#print axioms Hydro.Tutorials.vote_quorum.ensures
#print axioms Hydro.Tutorials.vote_quorum_safe_sched'
#print axioms Hydro.Tutorials.vote_quorum_safe_sched
#print axioms Hydro.Tutorials.quorumRun.eq
#print axioms Hydro.Tutorials.vote_quorum_safe_wired
