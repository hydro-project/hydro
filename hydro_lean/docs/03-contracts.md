# 03 — Contracts: `ensures` faces, and composing by contract

A module's contract is a structure of propositions about its **output** at `Values`,
attached to the definition by `ensures`. Consumers cite it by name. Nothing else about a
callee is ever used — not its body, not its internal wires, not a lemma file.

## The face

```lean
-- Hydro/Std/Quorum.lean:378–398
structure CQEnsures (ℓ : L) (min max : Nat)
    (resp : Fin (mem ℓ) → Multiset (K × Except E Unit))
    (dec : BatchCuts (mem ℓ) (K × Except E Unit))
    (out : (Fin (mem ℓ) → Multiset K)
      × (Fin (mem ℓ) → Multiset (K × E))) : Prop where
  /-- **Soundness** (unconditional): an emitted key holds `min` `Ok`
  votes among the consumed responses. -/
  emit_sound : ∀ (i : Fin (mem ℓ)), ∀ k ∈ out.1 i,
    min ≤ cqOkCount (cqConsumed (resp i) (dec i)) k
  /-- **The crossing count** (under the usage contract): a key is
  emitted **iff** it reached `min` `Ok` votes among the consumed pool —
  exactly once (the emission multiset's count is the indicator). -/
  emit_count : ∀ (i : Fin (mem ℓ)) (k : K), 1 ≤ min → min ≤ max →
    cqKeyCount (cqConsumed (resp i) (dec i)) k ≤ max →
    (out.1 i).count k
      = if min ≤ cqOkCount (cqConsumed (resp i) (dec i)) k
        then 1 else 0
  /-- **The error leg is pure**: the fails quote the raw stream's `Err`
  responses (no clock, no decision). -/
  fails_eq : ∀ (i : Fin (mem ℓ)),
    out.2 i = (resp i).filterMap cqErrProj
```

Everything in the statement is an **input** of the module (`resp`, `dec`, `min`, `max`)
or its **output** (`out`). That is the rule (`../DOCTRINE.md` R3): *ensures are over
outputs, never internal wires.* `cqConsumed (resp i) (dec i)` is "the responses member
`i` consumed under the decision" — input vocabulary, defined in `Trace.lean`, not a
mirror of the slice's registers. The guard `cqKeyCount … ≤ max` on `emit_count` is the
usage contract Rust's `nondet!` comment promises informally ("we always persist values
that have not reached quorum, so … deterministic quorum results"), now a theorem under
an explicit cap — and the cap is where `FINDINGS.md` B3 (stragglers past `max`) is made
visible rather than hidden.

The three fields are discharged in the module's trailing `prove` block
(`Quorum.lean:867–897`; chapter 04). `hydro def` then provides:

- `collect_quorum H …` — the program, at any `H`;
- `collect_quorum.ensures ℓ resp min max dec : CQEnsures ℓ min max resp dec
  (collect_quorum (Values L mem) ℓ resp min max dec)` — the face, at `Values`
  (`Hydro/HydroDef.lean:1358`).

## A consumer: `two_pc`

Two-phase commit (`Hydro/TwoPC.lean:134–179`, mirroring `hydro_test/src/cluster/two_pc.rs`)
is two `collect_quorum` calls and some network edges. Its proof is **only** contract
composition:

```lean
-- Hydro/TwoPC.lean:148–157
  -- let (c_all_vote_yes, _) = collect_quorum(c_votes, n, n);
  let cq1 := collect_quorum H coord c_votes num_participants
    num_participants dec.votes
  -- the phase-1 wire IS the canonical pool crossing (definitional
  -- at `Values`): its quorum faces, at the vote pool
  ghost have hcq1 := collect_quorum.ensures coord c_votes (mem part)
    (mem part) dec.votes
  ghost witness cq1.1
```

`ghost have hcq1 := collect_quorum.ensures …` brings the callee's face into scope at the
program point where the callee is called — a spec-only binding, erased from the
computational leg (chapter 04). `ghost witness cq1.1` supplies the `∃ voteYes` of
`two_pc`'s own face. Then:

```lean
-- Hydro/TwoPC.lean:171–179
  cq2.1
  prove
    vy_sound := hcq1.emit_sound,
    vy_count := (fun c p h1 hcap =>
      hcq1.emit_count c p h1 le_rfl hcap),
    commit_sound := hcq2.emit_sound,
    commit_count := (fun c p h1 hcap =>
      hcq2.emit_count c p h1 le_rfl hcap)
```

Each field of `TPCEnsures` (`TwoPC.lean:100–`) is one field of a `CQEnsures`, instantiated
at the vote pool or the ack pool. No fact about how `collect_quorum` works internally
appears — if its body changed tomorrow but its face held, `two_pc` would not notice.
The 2PC theorems (`two_pc_unanimous`, `two_pc_once`, `two_pc_commit_iff`,
`TwoPC.lean:379–`) are then pure reasoning over `TPCEnsures` plus the echo pools
(`tpcVotesPool`, `tpcAckPool`) — `Multiset` arithmetic, no program.

## Consumer-shaped faces

Because consumers can only use faces, a face is designed for its consumers. `D63`/`D64`
reshaped several: `AP2Ensures` states `log_entry_src`/`log_covers_mono`/`log_len_le_ck` over the
output log (proven inside `acceptor_p2` from an internal `hlog_pool` ghost — so no
consumer ever sees the fold that produced the log); `PP1bEnsures` dropped its
`∃ okPool` and decision arguments (D66) so `LeaderElection` cites `hpp.leader_batch`
instead of transporting indices; `SPEnsures` is stated over **log entries** (outputs)
rather than the sequencer's sent traffic (an internal wire), which is what let K4 be
restated and the entire `spSentTrace` mirror be deleted (chapter 05, D64).

Two consequences you will see in every module:

- **"Re-execution with larger inputs"** is never a face. If a consumer needs "the fact I
  learned at stage `w` still holds at the closed knot", it composes the module's
  generated monotonicity `M_mono…` (`../ARCHITECTURE.md` §4) with `M.ensures` at the
  larger inputs — `hblift₄ := leader_election_mono₄ …` in `PaxosCore.lean:182` is one.
- **A face states what its consumers need, in their vocabulary.** `RCEnsures` is stated
  via `RCTick` (one tick of recommit computation as the sequencer wants to read it,
  `Recommit.lean`), `LEEnsures.discipline` packages exactly `LeaderDiscipline`, the
  requirement `sequence_payload` takes.

## Requires

A module whose contract needs facts about its *inputs* takes them as premises of the
face, in the same vocabulary — `LEEnsures.discipline : 1 ≤ qs → LeaderDiscipline …`,
`PP1bEnsures.ballot_stable : … → PP1bRequires … →` (`PP1b.lean`). There is no separate
`requires` keyword: a requirement is a hypothesis of the field that needs it, so a
consumer pays it exactly where it consumes.

Next: chapter 04 — how the fields get proven, inside the program.
