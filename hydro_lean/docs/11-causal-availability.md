# Causal availability: the boundary rule and the shelved design

> Status: **current** (FINDINGS D21/D22). Outcome: leader-ballot stability
> is **derived** in the unmodified model (`le_ballot_stable`,
> `Programs/Paxos/PaxosCore.lean`) — `commit_agreement`'s only remaining
> input is `nA ≤ 2f + 1`. Artifacts: `Hydro/CausalAvail.lean` (the shelved
> compiled design for gateless loops), `Programs/CausalToy.lean` (the
> gateless boundary witness, `#guard`-checked),
> `Programs/Paxos/AcausalExploration.lean` + `lake exe explore` (the
> executable record: the acausal attack starves; the causal control heals).
> The main `lake build` covers everything; `lake exe falsify` unaffected.

## The question

Leader-ballot stability was the one protocol-internal hypothesis left on
`commit_agreement`. It materializes the paxos.rs:185–188 `nondet_commit`
comment. The design discussion asked: shouldn't the causality it encodes
be derivable *from the fixpoint*, since later inductive steps are causally
after earlier ones — some kind of reasoning about the length of the prefix
of decisions consumed at an iterative step? And if a framework mechanism
is needed, what does "strengthening batch" look like without polluting
everything?

## E1 — the headline reversal: Paxos does not need any new mechanism

**Claim tested**: the decision space's acausal freedom (batch/snapshot
legality is checked against *completed* streams) admits a run that
violates leader-ballot stability and double-commits, even guarded.

**Result: the violation is NOT representable. The fabrication
self-destructs through legality chains the model already has.**

The attack: keep P0's reign continuous across a ballot switch
`bA → bC` by delivering a fabricated `bC` quorum view at the switch tick
(causally impossible: those replies answer a solicitation that has not
happened). A continuous reign suppresses `just_became_leader`, hence the
recommit that would heal slot 0; stale `next_slot` then fresh-emits over
P1's committed value.

Why it cannot complete:

1. **Solicitation is fixpoint-staged.** `p_trigger_election` is gated on
   `!p_is_leader` (paxos.rs:449), and `p_is_leader` is a `forward_ref`
   cycle wire — the p1a send at any iterate reads the *previous*
   iterate's flag. So the p1a→p1b loop is not purely within-iteration
   after all: its trigger crosses the fixpoint, and Kleene iteration from
   `⊥` stages it. Leadership at `b` needs a full `b`-bucket ⇒ realized
   `Ok @ b` replies ⇒ (acceptor echo face + `batchC` legality) `b`
   solicited ⇒ a flag-**false** `b`-tick. At the fabricated reign every
   `b`-tick's flag is true.
2. `p_has_largest_ballot` is identically true at realized ticks (the
   ballot jump overtakes the received max in the same tick), so the only
   way a `b`-tick's flag is false is **bucket masking**: the view's max
   *full* bucket differs from `b` (`p1bMaxQuorumBallot` ignores non-full
   buckets).
3. **Masking self-poisons.** A mask needs a strictly larger full *own*
   bucket `bD` (quorum entries answer the owner's own solicitations)
   delivered before the masked tick; own ballots are num-monotone, so
   `bD`'s solicitation tick comes later, and `snapshotC` views only
   accumulate — by that tick the full `bD` bucket is already in view,
   the flag is true, and `bD` is unsolicitable. Fabrication levels
   strictly ascend; a finite run has a topmost level with no mask.
4. Everything else the safety proof uses already holds for all decisions.

**Executable**: `lake exe explore`. The best acausal script *starves* —
P0's flag stream blocks at `[false, true]`, `bC` is never solicited, no
second commit (P1 commits `x@0` only). The causal control delivers the
same `bC` entries at the earliest causal tick: the reign breaks
(`[false, true, false, true, true]`), `just_became_leader` fires, the
recommit reads the carried log — slot 0 (vote count 2 > f) is recognized
as already committed and *skipped* (paxos.rs:640–654), fresh `y` is based
at slot 1. Agreement, healing visible.

**Consequences (now realized).**

- Leader-ballot stability is a **theorem of the current model** —
  `le_ballot_stable`, exported as `leader_election`'s module contract
  (`LeaderElection.lean`); `hstab` is gone from `commit_agreement` (final
  face: `nA ≤ 2f+1` only). The "causality from the fixpoint" instinct is
  exactly right for Paxos, and no framework change was needed.
- The derivation is protocol-specific: it leans on the trigger gate, view
  accumulation, ownership, and full-bucket max selection. The shipped
  shape (all at the owning modules, no new framework):
  1. `le_p1a_elim` (`LeaderElection.lean`): every broadcast P1a was
     released at a trigger-true tick of the sender's own ballot wire —
     from the transcription of `filter_if(p_trigger_election)`.
  2. `le_trigger_gate` + `p_leader_heartbeat_trigger_gate`
     (`PLeaderHeartbeat.lean`): a trigger-true tick reads a `false` leader
     flag off the `forward_ref` cycle input (paxos.rs:449).
  3. `pP1bPv_bucket_promise` / `pP1bPv_bucket_persists` /
     `pP1bPv_false_mask` / `p1bMaxQuorumBallot_ge` (`PP1b.lean`): full
     buckets are genuinely promised, persist along the accumulated cuts,
     and can only be masked by a strictly larger full own bucket
     (`p_has_largest_ballot ≡ true` at realized ticks —
     `p_ballot_calc_hasLargest`, now consumed).
  4. `leRun_flag_prefix` / `leRs_ok_solicited` (`LeaderElection.lean`):
     the cycle's flag input is a prefix of the run's flag wire (realized
     ticks are final across the unfolding); every `Ok`-promised ballot was
     solicited at a flag-false tick (the module's solicitation oracle).
  5. `pP1bPv_no_false_full` / `pP1bPv_ballot_stable` (`PP1b.lean`): the
     mask regress — strong induction on the remaining ticks; fabrication
     levels strictly ascend on the finite run — exported as a `p_p1b`
     contract over any solicitation oracle.
- The Rust comment's guarantee is enforced by the *election protocol
  itself* (a leader stops soliciting; a usurper ballot must be solicited
  from a non-leader tick), not by network timing — FINDINGS D22 (upstream
  doc suggestion).

## E2 — what the framework mechanism would look like (for the record)

Even though Paxos does not need it, the gateless program class does.
`Programs/CausalToy.lean` is the minimal citizen: P emits request token
`t` at tick `t`; the environment echoes; P consumes replies. Under plain
`batchC` (completed availability), the decision `[[3]], [], [], []` —
*receive reply 3 at tick 0, three ticks before request 3 is sent* — is
legal and **realized** (`#guard`). No protocol structure prevents it:
outputs don't depend on inputs, so there is no gate to starve the
fabrication. "No reply to an unsent request" is simply false in the
current semantics for this program.

### The two viable formalizations (one file, `Hydro/CausalAvail.lean`)

**Option C (definitional) — `causalTickLoop`**, the atomic-tick-region
combinator: tick `t` draws its batch from
`availAt env outs t = env(own outputs through tick t−1)` — the
**diagonal** of the environment pipeline over own-output prefixes.

- The guarded tick-indexed fixpoint the discussion identified:
  consumed/produced are mutually recursive over ticks, structurally
  guarded by the strict output prefix (this tick's output is appended
  only *after* its batch is drawn — tick atomicity, inputs before
  outputs).
- `env : List β →ₘ List α` — availability-family monotonicity
  (`availAt_mono`, `availAt_mono_outs`) comes from the `→ₘ` type, never
  a per-program proof. The `→ₘ` machinery is exactly what makes the
  diagonal well-defined (the user's monotonicity point).
- **`batchC` is the constant-environment special case**
  (`causalTickLoop_const`): when the source does not depend on my
  outputs, the family is constant and consumption coincides. So Option C
  is a conservative *generalization*, not a rival combinator: "which
  edges get it" dissolves — every tick input has an environment; for
  independent sources it degenerates to today's semantics.
- Unification with `forward_ref`: same guarded-recursion shape at two
  granularities — `forward_ref` stages *iterations* (cross-cycle wires),
  `causalTickLoop` stages *ticks* (within-cycle loops). E1 showed they
  overlap when a loop's trigger happens to route through a cycle wire
  (Paxos); they are orthogonal otherwise (the toy has no cycle wire at
  all).

**Option A (knowledge face) — `CausalCuts`**: consumed-prefix-through-`t`
⊆ availability-at-`t`, multiset form matching `BatchLegal`; eliminator
`CausalCuts.mem_availAt`.

- **A is C's elimination principle**: `causalTickLoop_causal` derives
  `CausalCuts` for every realized run, zero hypotheses. They are not
  rivals: C defines, A is what proofs consume.
- Migration path: a program still written with plain `batchC` can take
  `CausalCuts` as decision-bundle wellformedness (a visible hypothesis) —
  useful as a stopgap, dominated by C for new code.

**Option B (retype decisions as selections-from-availability, dependent
types)**: dominated. C achieves unrepresentability of illegal cuts with
plain data decisions + blocking (the established discipline: illegal
decisions block and are never reasoned about); B would make decision
scripts dependently typed and break the "decision bundle = plain data"
falsifier workflow.

### The toy results (all compiled/`#guard`-checked)

| | plain `batchC` | `causalTickLoop` |
|---|---|---|
| acausal cut `[[3]],…` | **legal, realized** | **blocks at tick 0** |
| causal cut `[],[0],[1],[2]` | realized | realized (identical) |
| "reply at tick t is a request from tick < t" | **false** | theorem, 0 hypotheses (`toy_no_early_reply`, `toy_reply_lt_tick`) |

### Blast radius if C were adopted (measured against the current tree)

- New file only (`Hydro/CausalAvail.lean`, ~180 lines incl. proofs);
  `TStream.lean`/`Growth.lean`/`ForwardRef.lean` untouched; `batchC`
  stays (it *is* the constant case). No existing statement changes; all
  existing ∀-decision theorems survive (the causal space is a subset).
- Adoption cost is per *loop consumer*, not per edge: a module whose
  input source depends on its own outputs would write its tick loop with
  `causalTickLoop env step` instead of `batchC` + separate scan, where
  `env` is the composite other side of the loop — for Paxos's proposer
  loop that means threading the acceptor pipeline stage as a parameter
  into PP1b (the quorum batch + snapshot) and AcceptorP1's fan-in.
  Estimated diff if retrofitted to Paxos: PP1b/LeaderElection decision
  bundles change shape (scripts re-keyed by availability cuts), the E1
  falsifier scripts need re-expression, and the K1/K3 faces over
  `snapshotC` re-prove over the guarded family — a *large* refactor
  (~all of PP1b's pinning machinery touched) **for zero additional
  theorem strength**, since E1 shows the current model already excludes
  the violations.
- The honest limit of C: with several mutually-looped locations, "the
  environment of one location" is itself a causal loop; the clean form
  is per-loop (one location vs a monotone environment). The multi-party
  generalization would need either a system-level tick interleaving (a
  scheduler flavor we deliberately quotient away) or mixing with
  `forward_ref` staging (environment taken at the previous outer
  iterate). Paxos sidesteps this precisely because its cross-location
  loops all route through cycle wires — which is WHY the fixpoint
  argument suffices there.

## Outcome (the recommendation, executed)

1. **C not adopted.** The protocol-level derivation of `LeaderBallotStable`
   shipped instead (`le_ballot_stable`; `commit_agreement` face is
   `nA ≤ 2f + 1` alone) — no framework changes, no statement churn. The E1
   artifacts are the non-vacuity witnesses (`lake exe explore`).
2. **The boundary is recorded** (FINDINGS D21): programs whose loop
   triggers route through `forward_ref` wires inherit tick causality from
   the fixpoint; gateless loops do not, and `Hydro/CausalAvail.lean` is the
   shelved, compiled design for when such a program arrives (with
   `batchC ≡` constant-env `causalTickLoop` as the compatibility theorem).
3. If the falsifier ever needs to script a gateless program, Option A's
   `CausalCuts` is the wellformedness to impose on its decision bundles.
