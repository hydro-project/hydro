# Acceptance Bar (final state)

The binding criteria for this project being *done*. Set by the user; changes here
require explicit user sign-off. Everything below is in addition to the standing
rules in DESIGN.md (zero `sorry`, build green, Rust-parity naming, findings
recorded in FINDINGS.md).

This version ratifies the **decisions-as-inputs** architecture (docs/10) as the
project's single semantics. The earlier two-lens/choreographic and
sealed-component commitments that previously occupied §2 are retired; their
deliverables were migrated (FINDINGS D19) and the code is recoverable from jj
history.

## 1. Mission

Entire Hydro programs are written in Lean 4 and proven correct **unboundedly**:
all input sizes, all materializations of nondeterminism — never bounded model
checking as the evidence. Zero `sorry` anywhere; every headline theorem depends
only on `propext` / `Classical.choice` / `Quot.sound`.

## 2. Deliverables (status gates)

- **(a) `collect_quorum`** ✅ — unbounded correctness, both branches
  (min = max and min < max), all inputs, all batch decisions:
  `collectQuorum_correct`, `collectQuorum_deterministic`,
  `collectQuorum_minEqMax_correct` (`Programs/CollectQuorumProof.lean`,
  `CollectQuorumMinMax.lean`) with the typed-stage face `collect_quorum_spec`
  (`CollectQuorumStreams.lean`). The quorum stage is **shared**: 2PC and Paxos
  consume the same verified module.
- **(b) Medium programs** ✅ — `two_pc` written over the shared
  `collect_quorumM` stage: master theorem `twoPC_committed_eq` + corollaries
  (`twoPC_unanimity`, `twoPC_nodup`, `twoPC_all_yes_deterministic`,
  `twoPC_deterministic`) in `Programs/TwoPCProof.lean`; `index_payloads`
  (`index_payloads_no_reelection`/`_preservation`/`_slots_strictMono`) and
  `join_responses` (`join_responses_spec`, `joinResponses_correct`) as typed
  stages with verified faces. (`sequence_payload` lives in the Paxos port.)
  The open-membership negative results are **retired** to the FINDINGS record
  (§C row + D20, which pins the design for their return) — not a live
  commitment.
- **(c) Paxos** ✅ — 1:1 port of `hydro_test/src/cluster/paxos.rs`
  (`Programs/Paxos/`, one Lean file per Rust function) and the headline
  (`Programs/Paxos/Safety.lean`):

  ```
  theorem commit_agreement
      (hnA : nA ≤ 2 * f + 1)
      (h  : (slot, v)  ∈ (paxos_core PaxosVariant.guarded f cbM nd).2 i)
      (h' : (slot, v') ∈ (paxos_core PaxosVariant.guarded f cbM nd).2 i') :
      v = v'
  ```

  Cross-proposer slot-functionality of `p_to_replicas` over the **full**
  decision space (every batching/snapshot/shuffle decision, every client
  timing, every cycle-unfolding depth). Named proof inputs of the verified
  face are exactly: `nA ≤ 2f + 1`; the client callback is
  typed `→ₘ` (prefix-monotonicity carried by the type). The B1 send-once and
  B2 recommit-once guards, paxos.rs:862's key uniqueness, **and the
  paxos.rs:186–189 leader-ballot stability (`le_ballot_stable`,
  LeaderElection's module contract; FINDINGS D21)** are **derived**,
  never assumed. The `faithful` variant executably falsifies the same
  statement (§5).
- Axiom audits: every theorem named above is `#print axioms`-audited in
  `Programs/AxCheck.lean`, `Programs/Paxos/AxCheck.lean`, `Flo/AxiomCheck.lean`.

## 3. Architecture terms (binding; must never regress)

- **One semantics: decisions-as-inputs** (docs/10). Programs are literal Lean
  functions over tick-located streams (`Hydro/TStream.lean`); every Rust
  `nondet!` site is an explicit, structured **decision input** (batch cuts,
  consumed batches, snapshot views, fuel); theorems quantify over all
  decisions. Interleavings are quotiented into decisions — given the decision
  trace, everything is pure evaluation. **No event/schedule induction
  anywhere**; a protocol proof gets exactly one induction: the generic
  fixpoint induction over `forward_ref` unfolding depth (causal depth, never
  schedules).
- **1:1 function-per-file transcription**: every Rust function under proof is
  one Lean file containing a literal transcription (same name, same
  intermediate streams, paxos.rs line cites inline). Rust
  `nondet!`/`manual_proof!` justification comments become **named proof
  inputs** (hypotheses of the verified face); guarantees are output clauses.
  A side-by-side Rust ↔ Lean table lives in the program's README.
- **Theorems live at the module that owns the data**: proof content about a
  module's I/O belongs in that module's file, stated with the module's input
  requirements as hypotheses. Top-level protocol files contain wiring,
  handoffs, and the final assembly only; counting/provenance stacks
  accumulating at the top are an architecture smell.
- **Proofs through types**: prefix-monotonicity is **type-derived** — the
  `Growth` (⊑) class's carrier instances are the Rust ordering markers
  (List = prefix, `Cnt` = count-domination, `Mem` = membership; families
  pointwise); each module's dataflow exists **once**, as a bundled monotone
  stage (`α →ₘ β`, `Hydro/Growth.lean`) written with the reader-lifted wire
  combinators; cycles close with `MonoMap.fix`; `.f` appears only at
  boundaries (run views, face statements, the falsifier). Hand-written
  per-stage growth lemmas are forbidden — if a stage needs one, the framework
  gets a combinator instead.
- **Singleton honesty**: singletons over time are trajectory-forward only.
  Value-monotonicity may be claimed **only** through the `MonoSing` wire type
  (`Hydro/MonoSing.lean` — the Rust `Monotonic` singleton bound), obtained by
  paying `fold_monotonic`'s inflationary closure obligation at the definition
  site (the Lean form of Rust's `monotone =` promise).

## 4. Verification gates (binding, per change)

- Clean `lake build` (the `#guard` tests — Lean mirrors of Rust sim tests —
  run during compilation).
- Zero-`sorry` grep across the tree.
- Axiom audits green: `Programs/AxCheck.lean`, `Programs/Paxos/AxCheck.lean`,
  `Flo/AxiomCheck.lean` report standard axioms only.
- `lake exe falsify` prints **All falsification checks PASSED**.
- **Non-vacuity (the D15 standing rule)**: every headline theorem needs an
  executable inhabitation witness — a concrete run in which the guarded
  program actually produces the guarded output. A theorem whose subject
  streams are empty in every run is not done.

## 5. Falsification record (must remain executable)

- **B1** (duplicate P1a broadcasts double-counted in p1b quorums) and **B2**
  (every-tick recommit ⇒ duplicate `(slot, ballot)` keys): `lake exe falsify`
  reproduces both against the `faithful` variant — B1 commits two different
  values at slot 0; the `guarded` variant agrees *and genuinely commits* —
  and the verified outputs are recorded in FINDINGS.md §B.
- B1/B2/B3 are recommended for upstream confirmation as Rust sim tests
  against `paxos.rs` / `quorum.rs`.

## 6. Governance

- This file is binding; modifications require explicit user sign-off.
- FINDINGS.md kept current: paper errata (§A), Rust bug candidates with repro
  scripts (§B), formalized contracts (§C), methodology findings (§D).
- SORRIES.md kept accurate (currently: none; one deferred metatheory
  statement, not a code gap).
- docs/: 10 describes the current architecture (docs/09's two carried-over
  insights — configurations as consumed tick-batch nestings, clusters as
  maps — are noted there); 02–05, 08–09, 98 are marked historical. Retired
  architectures are recoverable from jj history — history
  is the archive; the tree carries only the live design.

## 7. Out of scope / deferred (explicit)

- **Operational-lens transport** (the former TGraphSys/wiring-uniformization
  phase): the code was deleted as unused; per the user's ruling the
  denotational verified face over `paxos_core` is the deliverable. Resuming
  it is a new mandate, not a debt of this one.
- **Open membership** (dynamic clusters): retired; the re-formalization
  design is pinned in FINDINGS D20 (membership as an unordered cluster-located
  input stream with per-member snapshots).
- ~~**Deriving `LeaderBallotStable`**~~ — RESOLVED: derived as
  `le_ballot_stable` (LeaderElection's module contract; no time-indexed
  availability needed — the
  election trigger routes through a `forward_ref` cycle wire; FINDINGS
  D21). The generic causal-availability design for *gateless* loops stays
  shelved in `Hydro/CausalAvail.lean` until a program needs it.
