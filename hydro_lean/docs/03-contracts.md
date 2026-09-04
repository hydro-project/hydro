# 03 — Contracts: `ensures` faces, and composing by contract

A module's contract is a structure of propositions about its **output** at `Values`,
attached to the definition by `ensures`. Consumers cite it by name. Nothing else about a
callee is ever used — not its body, not its internal wires, not a lemma file.

## The face

```lean
-- Hydro/Std/Quorum.lean:469–
structure CQEnsures (ℓ : L) (min max : Nat)
    (resp : MemberMap (mem ℓ) (Multiset (K × Except E Unit)))
    (dec : BatchCuts (mem ℓ) (K × Except E Unit))
    (out : (MemberMap (mem ℓ) (Multiset K))
      × (MemberMap (mem ℓ) (Multiset (K × E)))) : Prop where
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

Two-phase commit (`Hydro/TwoPC.lean:115–286`, mirroring `hydro_test/src/cluster/two_pc.rs:14–69`)
is two `collect_quorum` calls and four network edges. Its face is stated over its
**output** `c_all_commit` and its **inputs** only — the participant count, their votes,
the proposed payloads (`TwoPC.lean:100–111`):

```lean
structure TPCEnsures {nC : Nat} (n : Nat) (vote : Fin n → P → Bool)
    (payloads : Fin nC → Multiset P) (out : Fin nC → Multiset P) : Prop where
  /-- **Soundness**: a committed payload was proposed. -/
  commit_proposed : ∀ c, ∀ p ∈ out c, 1 ≤ n → p ∈ tpcProposed payloads
  /-- **Unanimity** (the safety headline): a once-proposed payload that
  commits was voted yes by **every** participant — under every decision. -/
  commit_unanimous : ∀ c, ∀ p ∈ out c, 1 ≤ n →
    (tpcProposed payloads).count p = 1 → ∀ j, vote j p = true
  /-- **At-most-once**: with the one coordinator Rust has, a payload
  proposed at most once commits at most once — under every decision. -/
  commit_once : ∀ c (p : P), 1 ≤ n → nC = 1 →
    (tpcProposed payloads).count p ≤ 1 → (out c).count p ≤ 1
```

No internal wire, no witness, no decision in the signature (DOCTRINE R3). Inside the
program, the callee's face arrives at the call site:

```lean
-- Hydro/TwoPC.lean:134–137
  let cq1 := collect_quorum H coord c_votes num_participants
    num_participants dec.votes
  ghost have hcq1 := collect_quorum.ensures coord c_votes (mem part)
    (mem part) dec.votes
```

`ghost have hcq1 := collect_quorum.ensures …` is a spec-only binding (chapter 04) —
`two_pc` never unfolds `collect_quorum`; if its body changed tomorrow but its face held,
`two_pc` would not notice. What `two_pc` *does* read is its own wires, by name, at the
denotation (`den`); these are `rfl`:

```lean
-- Hydro/TwoPC.lean:153–157
  ghost have hprep : ∀ j, p_prepare j = tpcProposed payloads := fun _ => rfl
  ghost have hvotes : ∀ c, c_votes c
      = ((List.finRange (mem part)).map (fun j =>
          (tpcProposed payloads).map (fun p => (p, voteE vote j p)))).sum :=
    fun _ => rfl
```

Then the protocol argument is a handful of ghosts about those wires (the vote pool's
`Ok` count of a once-proposed payload is its number of yes-voters; the ack pool holds
`n` `Ok` acks per vote-yes; a commit needs an ack, so some coordinator vote-yes'd it —
`TwoPC.lean:165–240`), and each face is a few lines composing them with the callee's
`emit_sound`/`emit_count`:

```lean
-- Hydro/TwoPC.lean:251–254
    commit_unanimous := fun c p hp hn hone => by
      obtain ⟨c', hpc'⟩ := hcommit_src c p hp hn
      have h1 := hvy_votes c' p hpc'
      rw [hok_votes c' p hone] at h1
```

Because `two_pc` is a `hydro def`, the pipeline also generates its `two_pc_mono₁`,
`two_pc_param` and the corner artifacts, and the same four-name derivation that gives
`cq_safe_sched'` (chapter 06) gives `two_pc_safe_sched'` and `two_pc_once_sched'`
(`TwoPC.lean:407–434`): the step machine's commits, under any pacing, schedule and
horizon, satisfy the faces — with no satisfiability premise and no decision witness.

What is *not* a face: "unanimous and once-proposed ⟹ commits". A decision's
completeness is relative to the pool it consumes, and the ack stage's pool is an
internal wire, so that direction has no output-shaped statement; it is `two_pc`'s
liveness rung (`LIVENESS.md`, the ladder's (a)), where completeness is derived from
fairness, pool-free. The executable `lake exe twopc` and the `#guard` at
`TwoPC.lean:305` witness it on the demo scenario (10 commits once; the vetoed 20 never).

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
