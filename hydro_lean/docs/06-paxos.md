# Paxos: the Capstone (status: **safety proof complete**)

`Programs/Paxos/` ports `paxos_core` from `hydro_test/src/cluster/paxos.rs` —
proposer cluster + acceptor cluster, leader election, log recovery, payload
sequencing — in the decisions-as-inputs architecture (docs/10): every Rust
function is one Lean function over the located surface, every `nondet!` site
an explicit decision input, `forward_ref` a higher-order Kleene fixpoint,
prefix-monotonicity type-derived (`Hydro/Growth.lean`). The authoritative
file map is `Programs/Paxos/README.md`.

## The headline (proven; `Programs/Paxos/Safety.lean`)

```
theorem commit_agreement
    (hnA : nA ≤ 2 * f + 1)
    (h  : (slot, v)  ∈ (paxos_core PaxosVariant.guarded f cbM nd).2 i)
    (h' : (slot, v') ∈ (paxos_core PaxosVariant.guarded f cbM nd).2 i') :
    v = v'
```

Cross-proposer slot-functionality of `p_to_replicas` over the **full**
decision space — every batching/snapshot/shuffle decision, every client
payload timing, every cycle-unfolding depth. The named proof inputs of the
verified face are exactly:

- the client callback `cbM` is typed `→ₘ` — a Hydro program of the ballot
  streams is prefix-monotone *by construction*, so the requirement is
  carried by the type, not stated as a hypothesis;
- `nA ≤ 2f + 1` — `f + 1` quorums over the acceptor cluster intersect.

The B1 send-once and B2 recommit-once guards, paxos.rs:862's key
uniqueness, **and the paxos.rs:186–189 leader-ballot stability**
(`le_ballot_stable` — the election trigger reads `!p_is_leader`
through the `forward_ref` cycle, so a fabricated reign cannot bootstrap;
FINDINGS D21, executable record `lake exe explore`) are **derived** from
the guarded variant, never assumed. Axiom
audit (`Paxos/AxCheck.lean`): `propext`, `Classical.choice`, `Quot.sound`
only.

## Proof architecture (one induction)

- **K1** (`PaxosCore.lean` `run_promise_covers`): an acceptor's `Ok` promise at
  `b` carries a log covering each of its `Ok` votes below `b` — the `a_log`
  knot opened equationally (`alog_succ`), zero event induction.
- **K2/K3** (`Safety.lean` handoffs): consumed-P1a nodup (guarded send-once
  `le_p1a_nodup` + ballot-ownership disjointness) ⇒ decoded reply caps;
  P2a key uniqueness at the run (`spKeys_inv` over the `SPChainQ` chain) ⇒
  vote caps; `SPChosen` / `run_leader_providers` instantiate the
  `collect_quorum` faces to get `f+1` **distinct** members on both sides;
  frozen buckets (`pP1bPv_qlogs_pinned`) pin a single quorum view per
  ballot (`le_view_pinned`).
- **K4** (`emission_chosen_agree`): the ONE `Nat`-induction over the unfolding
  depth — quorum intersection, K1, merge dominance
  (`mergeQuorumLogs_covers`), the covered-slot scan discipline
  (`spCovered_inv`: fresh payloads never land on merge-covered slots;
  recommits carry the merged max-ballot value), and the provenance regress
  `k+1 → k` through `alog_succ`/`ap2t_log_entry`/`batchC_mem`. Ballots may not
  descend; the induction is on causal depth, never on schedules.
- **Agreement**: `commit_elim` (unconditional `join_responses` provenance)
  reduces commits to own-P2As + chosen keys; equal ballots close by key
  uniqueness, unequal by K4.

## Faithful vs guarded, and the executable falsifications

- **`faithful`** mirrors paxos.rs exactly — and is **provably unsafe**:
  `lake exe falsify` (63 ms) runs the FINDINGS B1 decision script — a
  re-triggered election re-broadcasts the *same* ballot, one acceptor `Ok`s
  both copies, `collect_quorum_with_response` counts both (sender identity
  dropped by `.values()`) — and prints **two different values committed for
  slot 0**; plus the B2 witness (every-tick recommit ⇒ duplicate
  `(slot, ballot)` keys, also `#guard`-checked). Verified outputs are
  recorded in [FINDINGS.md](../FINDINGS.md) §B.
- **`guarded`** applies the minimal fixes (P1a once per ballot; recommit
  once per ballot acquisition) and satisfies `commit_agreement`; on the same
  B1 script it commits exactly one value.

Running the falsifier is also the model's **vacuity check** (FINDINGS D15):
it caught a deadlocked `a_log` knot in an earlier model revision (max wire
blocked on the log) that had made every run empty.

## Status against ACCEPTANCE

The agreement proof and executable falsifications are complete — deliverable
(c) is ✅ (ACCEPTANCE §2). The operational-lens transport (the former
TGraphSys/wiring-uniformization idea) is explicitly **out of scope**
(ACCEPTANCE §7): per the user's ruling, the denotational-style headline over
`paxos_core`'s verified face is the deliverable; the pivot-1
TGraphSys/choreographic framework code was deleted as unused (jj history is
the archive), and resuming that phase would be a new mandate.
