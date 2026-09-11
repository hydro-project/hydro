# Paxos (`hydro_test/src/cluster/paxos.rs`) — decisions-as-inputs port

Every file is one Rust function, transcribed 1:1 over the located surface
(`Hydro/TStream.lean`, `Hydro/ForwardRef.lean`), with its verified face
(proof inputs = materialized `nondet!`/`manual_proof!` sites; proof outputs
= run-level guarantee clauses). Prefix-monotonicity is **type-derived**: each
module's dataflow exists exactly once, as a bundled monotone map (`→ₘ`,
`Hydro/Growth.lean`) written with the reader-lifted wire combinators (so the
body reads in Rust method-chaining order); monotone singleton wires
(`MonoSing`, `Hydro/MonoSing.lean`) appear in the stage *signatures*, and
cycles close with `MonoMap.fix` — no hand-written growth lemmas exist and
`.f` appears only at boundaries (run views, face statements, falsifier). See
`docs/10-decisions-as-inputs.md` for the architecture.

| Lean file | Rust | contents |
|---|---|---|
| `Types.lean` | paxos.rs:44–70 | `Ballot`/`LogValue`/`P2a`/`P1b`/`P2b`, `LogMap`, `PaxosVariant` (faithful/guarded = FINDINGS B1/B2 fixes) |
| `BallotCalc.lean` | `p_ballot_calc` (:348–412) | `p_ballot_calcM` stage (`MonoSing` ballot-number wire in the signature) + ownership/has-largest faces |
| `PLeaderHeartbeat.lean` | `p_leader_heartbeat` (:414–482) | transcription + `p_leader_heartbeatM`; no safety clauses (discharge by absence) |
| `AcceptorP1.lean` | `acceptor_p1` (:485–524) | transcription + tick lift (max wire = productive batch fold; replies block on the `a_log` knot) + `ap1_*` faces + `ap1_decode_cap` |
| `PP1b.lean` | `p_p1b` (:528–593) | transcription + bucket calculus + frozen-bucket pinning (`foldEarlyStop_full_pin`, `pP1bPv_qlogs_pinned`) |
| `Recommit.lean` | `recommit_after_leader_election` (:596–672) | pure function + merge lemmas + toCommit/holes inversions |
| `AcceptorP2.lean` | `acceptor_p2` (:809–899) | transcription + tick lift (`acceptor_p2_ticksM`, coverage-`MonoSing` wire in the signature) + `ap2_*` faces + `ap2_decode_cap` |
| `SequencePayload.lean` | `sequence_payload` (:679–774) | the function + `sp_*` stages + `sequence_payloadM` + the guarded scan invariants **`spKeys_inv`** (B2 key discipline) and **`spCovered_inv`** (covered-slot discipline) + `SPChainQ` + the module contracts: **`SPEmission`/`SPChosen`** (abstract witnesses), **`sp_commit_spec`**, **`sp_emission_spec`**, **`sp_log_entry_spec`**, **`sp_emission_functional`**, over a carrier satisfying **`SPWireDiscipline`** |
| `LeaderElection.lean` | `leader_election` (:253–345) | the function + `le_*` stages + `leader_election_bodyM`/`leader_electionM`; internal 3-cycle `forward_ref`; guarded send-once (`le_p1a_nodup`, B1) |
| `PaxosCore.lean` | `paxos_core` (:136–246) | the function; outer 2-cycle `forward_ref`; `paxos_core_bodyM`; the wiring layer: the `sequence_payload` carrier at each cycle (`pcG`/`spInputs`), its wire discipline discharged from `leader_election`'s contracts (`run_discipline`), the contract instantiations (`run_leader_providers`, `run_leader_view_promise`, `run_emission_zero`), the `a_log` knot (`alog_succ`), and **K1 `run_promise_covers`** (promise coverage of chosen keys, through the knot) |
| `Safety.lean` | — | the final assembly, pure contract composition: `emission_chosen_agree` (the ONE protocol induction), **`commit_agreement`** (the headline) |
| `AxCheck.lean` | — | axiom audit: headline + spine on `propext`/`Classical.choice`/`Quot.sound` only |
| `Falsification.lean` | — | the executable B1 disagreement script + B2 module-face witness (`lake exe falsify`, output recorded in FINDINGS.md) |
| `AcausalExploration.lean` | — | the D21 causality record: the acausal fabricated-reign script starves, the causal control heals (`lake exe explore`) |

Every proof lives with the function that owns the data: `AcceptorP1/P2.lean`
carry their output eliminators, run faces, and decoded reply caps;
`LeaderElection.lean` the guarded send-once (`le_p1a_nodup`, B1); `PP1b.lean`
the frozen-bucket pinning; `SequencePayload.lean` the guarded key and
covered-slot disciplines (with the ballot-stability clause
— derived at the run, FINDINGS D21); `Recommit.lean` the
merge/output-structure lemmas.
`PaxosCore.lean` holds wiring, handoffs, and K1; `Safety.lean` holds only
handoffs (module contracts instantiated at the run) and the final assembly.

**The headline** (`Safety.lean`):

```
theorem commit_agreement
    (hnA : nA ≤ 2 * f + 1)
    (h  : (slot, v)  ∈ (paxos_core PaxosVariant.guarded f cbM nd).2 i)
    (h' : (slot, v') ∈ (paxos_core PaxosVariant.guarded f cbM nd).2 i') :
    v = v'
```

— cross-proposer slot-functionality of `p_to_replicas`, over the full
decision space (all batching/snapshot/shuffle decisions, all client
timings, all unfolding depths). The client callback `cbM` is typed `→ₘ`, so
its prefix-monotonicity is carried by the type rather than a hypothesis.
The B1/B2 guards, paxos.rs:862's key uniqueness, **and the
paxos.rs:186–189 leader-ballot stability** (`le_ballot_stable` — the
election trigger reads `!p_is_leader` through the `forward_ref` cycle;
FINDINGS D21, executable record `lake exe explore` +
`AcausalExploration.lean`) are *derived*, not assumed; the faithful
variant falsifies the statement executably.

`hydro_std` dependencies: `Programs/CollectQuorumStreams.lean`
(`collect_quorum`, `collect_quorum_with_response` + quorum-extraction faces),
`Programs/IndexPayloads.lean`, `Programs/JoinResponses.lean`.
