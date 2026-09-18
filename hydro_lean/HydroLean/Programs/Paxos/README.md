# Paxos (`hydro_test/src/cluster/paxos.rs`) — decisions-as-inputs port

Every file is one Rust function, transcribed 1:1 over the located surface
(`Hydro/TStream.lean`, `Hydro/ForwardRef.lean`), with its verified face
(proof inputs = materialized `nondet!`/`manual_proof!` sites; proof outputs
= run-level guarantee clauses). Each Rust function is **one `Verified`
artifact** (`Hydro/Growth.lean`): `.f` the dataflow (an inline `let`-chain
of wire combinators, one `let` per Rust `let`, in Rust order), `.mono`
prefix-monotonicity (type-derived — no hand-written growth lemmas exist),
`.ensures` the module contract, stated over the artifact's inputs and its
**actual output** and proved by ghost `have`s inside the definition
(`let` for data, `have` for proofs — the Verus ghost style; proofs erase,
the falsifier executes the identical wire graph). Named per-guarantee
requirement structures (`PP1bRequires`, `SPRequires`, `PP1bReplyCap`)
carry preconditions; composition is projection (`.ensures` applied at the
consumer's own wires). Monotone singleton wires (`MonoSing`,
`Hydro/MonoSing.lean`) appear in the stage *signatures*, and cycles close
with `MonoMap.fix`. See `docs/10-decisions-as-inputs.md` for the
architecture.

| Lean file | Rust | contents |
|---|---|---|
| `Types.lean` | paxos.rs:44–70 | `Ballot`/`LogValue`/`P2a`/`P1b`/`P2b`, `LogMap`, `PaxosVariant` (faithful/guarded = FINDINGS B1/B2 fixes) |
| `BallotCalc.lean` | `p_ballot_calc` (:348–412) | `Verified` artifact (`MonoSing` ballot-number wire in the signature; the jump closure's `monotonic =` obligation inline at the fold); ensures ownership + `p_has_largest_ballot ≡ true` |
| `PLeaderHeartbeat.lean` | `p_leader_heartbeat` (:414–482) | `Verified` artifact; ensures the trigger gate (paxos.rs:449 reads a `false` flag off the cycle) + `i_am_leader` ownership |
| `AcceptorP1.lean` | `acceptor_p1` (:485–524) | `Verified` tick artifact (max wire = productive batch fold; replies block on the `a_log` knot); `AP1Ensures`: `ok_spec`/`reply_echo`/`reply_dst`/`decode_cap` |
| `PP1b.lean` | `p_p1b` (:528–593) | `Verified` artifact + bucket calculus; `PP1bEnsures` (over the output legs): `leader_len`/`view_promise`/`leader_providers` (given `PP1bReplyCap`)/`qlogs_pinned` (frozen buckets, `foldEarlyStop_full_pin`)/`ballot_stable` (given `PP1bRequires` — the fabricated-reign regress as ghost `have`s) |
| `Recommit.lean` | `recommit_after_leader_election` (:596–672) | pure function + merge lemmas + toCommit/holes inversions |
| `AcceptorP2.lean` | `acceptor_p2` (:809–899) | `Verified` tick artifact (coverage-`MonoSing` wire in the signature); `AP2Ensures`: `ok_spec` (write-before-ack)/`log_entry`/`decode_cap` |
| `SequencePayload.lean` | `sequence_payload` (:679–774) | `Verified` artifact + the guarded scan invariants **`spKeys_inv`** (B2 key discipline) and **`spCovered_inv`** (covered-slot discipline) + `SPChainQ`; `SPEnsures`: **`commit_spec`**, **`emission_spec`**, **`log_entry_spec`**, **`emission_functional`**, each over the named **`SPRequires`**; public witnesses **`SPEmission`/`SPChosen`** |
| `LeaderElection.lean` | `leader_election` (:253–345) | `Verified` artifact (`le_*` stages + `leader_election_bodyM`; internal 3-cycle `forward_ref`); guarded send-once (`le_p1a_nodup`, B1); `LEEnsures`: `own`/`lead_ne`/`stable`/`pinned`/`view_promise`/`providers` — exactly `SPRequires` |
| `PaxosCore.lean` | `paxos_core` (:136–246) | the `Verified` artifact **whose contract IS the headline** (`SlotFunctional`); outer 2-cycle `forward_ref`; `paxos_core_bodyM`; the wiring layer: the `sequence_payload` carrier at each cycle (`pcG`/`spInputs`), its requirements discharged from `leader_election.ensures` (`pcReq`), `run_emission_zero`, the `a_log` knot (`alog_succ`), **K1 `run_promise_covers`**, and the final assembly (`emission_chosen_agree` — the ONE protocol induction — and `fix_slot_functional`, the proof the artifact carries) |
| `AxCheck.lean` | — | axiom audit: `#print axioms paxos_core` (the def carries the proof) + spine on `propext`/`Classical.choice`/`Quot.sound` only |
| `Falsification.lean` | — | the executable B1 disagreement script + B2 module-face witness (`lake exe falsify`, output recorded in FINDINGS.md) |
| `AcausalExploration.lean` | — | the D21 causality record: the acausal fabricated-reign script starves, the causal control heals (`lake exe explore`) |

Every proof lives with the function that owns the data, as ghost `have`s
inside that function's `Verified` definition: `AcceptorP1/P2.lean` carry
the reply/ack openings and decoded reply caps; `LeaderElection.lean` the
guarded send-once (`le_p1a_nodup`, B1) and the run-level solicitation
ghosts; `PP1b.lean` the frozen-bucket pinning and the fabricated-reign
regress; `SequencePayload.lean` the guarded key and covered-slot
disciplines (with the ballot-stability clause — derived at the run,
FINDINGS D21); `Recommit.lean` the merge/output-structure lemmas.
`PaxosCore.lean` holds wiring, the requirement discharge (`pcReq`), K1,
and the final assembly — ending in the artifact whose contract slot
carries the guarantee.

**The headline IS `paxos_core`'s type** (`PaxosCore.lean`):

```
def paxos_core (variant : PaxosVariant) (f : Nat)
    (nondet : PaxosNondet P nP nA) :
    Verified (PaxosCallback P nP)                        -- c_to_proposers
      ((Fin nP → Stream (Ballot nP))                     -- p_to_clients
        × (Fin nP → Stream (Nat × Option P)))            -- p_to_replicas
      (fun _cb out =>
        variant = .guarded → nA ≤ 2 * f + 1 → SlotFunctional out.2)
```

— one artifact: the map, its monotonicity in the client callback (the
callback is a first-class monotone input), and the guarantee (bugfix flag
⇒ quorum intersection ⇒ `SlotFunctional`: any two commits at one slot,
across any two proposers, carry the same value), over the full decision
space (all batching/snapshot/shuffle decisions, all client timings, all
unfolding depths). There is no separate headline theorem: consumers
project `(paxos_core …).ensures cb rfl hnA`, the map stays definitionally
the 1:1 transcription, and `#print axioms paxos_core` audits the face.
The B1/B2 guards, paxos.rs:862's key uniqueness, **and the
paxos.rs:186–189 leader-ballot stability** (`leader_election.ensures.stable`
— the election trigger reads `!p_is_leader` through the `forward_ref`
cycle; FINDINGS D21, executable record `lake exe explore` +
`AcausalExploration.lean`) are *derived*, not assumed; the faithful
variant falsifies the statement executably.

`hydro_std` dependencies: `Programs/CollectQuorumStreams.lean`
(`collect_quorum`, `collect_quorum_with_response` + quorum-extraction faces),
`Programs/IndexPayloads.lean`, `Programs/JoinResponses.lean`.
