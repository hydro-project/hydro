# Findings and Methodology

The authoritative catalog is [FINDINGS.md](../FINDINGS.md); this is the narrative.

## What formalization surfaced

**Dissertation errata (§A).** Mechanization found that Defs 2.3.2/2.3.3 (Output
Maximality / Streaming Progress) are unsatisfiable as literally stated — they
quantify over configurations no execution can reach, and fail even for the paper's
own `map`/`fold` (Fig 2.14). The repair (an operator-supplied configuration
invariant `Operator.Inv`, threaded through Lemma 2.4.4 as `Graph.Consistent`, plus
the re-arming-`fix` carrier design) preserves the paper's intent and is documented
as a suggested erratum. This is the classic value of mechanization: the informal
proof's implicit "configurations arise from runs" assumption became visible only
under a proof assistant's quantifiers.

**Candidate bugs in the Rust code (§B) — found by proving, confirmed by executing.**
The Paxos port produced executable falsification scripts against the
faithful model (`lake exe falsify` + `#guard`s; see [06-paxos.md](06-paxos.md)):
duplicate-P1a false quorums violating agreement (B1), and every-tick recommit
duplicating commits (B2); plus the
batching-dependence of `collect_quorum_with_response`'s straggler handling (B3).
Each was discovered as a **failed contract handoff** during composition — the formal
ports made implicit preconditions explicit (`AtMostMaxResponses`, unique-metadata,
causality), and the composition failed them. The counterexample witnesses are
schedule scripts designed to round-trip to Rust sim repros.

**Implicit contracts made explicit (§C).** `1 ≤ min` for quorum; `Nodup` payloads
for 2PC; `FreshUpdates` for slot assignment; closed membership for 2PC (with the
machine-checked analysis — in the retired layer, FINDINGS §C row — that the
dynamic-`broadcast` `nondet!(/** TODO */)` in two_pc.rs is not dischargeable);
every `manual_proof!` promise now a hypothesis. None of these is a bug — each is a
usage contract that existed only in comments or in nobody's head, and now exists
as a named hypothesis of a verified face.

## Methodology lessons (§D, distilled)

1. **Correctness properties must be type-derived.** If you find yourself *stating*
   "outputs are Perm-equal across schedules" or hand-proving that a stage's output
   grows with its input, the property should have been carried by a type — a
   quotient carrier, a `→ₘ` wire, a `MonoSing` singleton. Hand-stated determinism
   or growth is the smell of a missing type.
2. **Stop and build the combinator.** The single most effective process rule: when a
   proof starts unfolding a tick step function or hand-inducting over schedules,
   stop — the pattern is an instance of a missing generic principle (this project's
   instances, across its architectural eras: retiming, mid-run site projection,
   cycle rules, snapshot edges; in the final form, the `→ₘ` wire combinators,
   `MonoMap.fix`, and `fold_monotonic`). Each was built once, with a demo as its
   acceptance test, and eliminated a whole class of bespoke proofs. The Paxos effort
   was deliberately paused twice for exactly this; both pauses paid for themselves.
3. **Statement-preserving refactors.** Run-facts were proven against an early
   bespoke runner, then re-derived as corollaries of generic lemmas with their
   *statements kept* — proofs shrink, trust is preserved, and the proven theorem
   serves as the regression test for the refactor (the (a)/(b) deliverables were
   migrated across two architecture retirements this way — FINDINGS D19).
4. **Bounded/unbounded parity.** Every unbounded theorem's spec is exercised by
   `#guard`s on concrete schedules — the same property, model-checked (the Rust sim
   test analogue) and proven. The two artifacts cannot drift because they share the
   spec definition; and stating faces over *all* decisions makes
   under-quantification structurally impossible rather than a review concern
   (historically the `Holds` family's job).
5. **Negative results are deliverables.** The B1/B2 executable falsifications
   (and, in the retired layer, the open-membership negatives — FINDINGS §C row +
   D20) are proven or executable artifacts that *delimit* the guarantees — they
   are what make the positive theorems trustworthy, and they map failure modes to
   the exact discipline stage that catches them (illegal decision vs. missing
   guard vs. contract violation).
6. **English justifications are proof obligations.** The project's arc in one
   sentence: every `nondet!(/** reason */)` and `manual_proof!(/** reason */)`
   comment in the Rust either became a theorem, a hypothesis, or a counterexample.
