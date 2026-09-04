# Liveness under fairness — TLA+-style, natively denotational

A design document (with two compiled prototypes: `Liveness.lean`,
`LivenessChain.lean`) for the question *"how do we reason about
liveness under a fairness model similar to TLA+, in a way that is
native to our denotational semantics — with the proofs at `Values`
and a transfer property, not over the step-indexed machine?"* Written
for a reader who knows TLA+'s liveness story (WF/SF, `⟿`, the WF1
rule, machine closure) and wants to see where each piece lands here.

Status: **the chain architecture below is the design of record**, and
its load-bearing prototype is proven end-to-end
(`LivenessChain.lean`: the temporal/fairness vocabulary over decision
chains, the `Values` chain theorems, the two generic transfer kits,
and `cq_live_chain` — `collect_quorum` liveness re-derived with all
protocol content at `Values`). `Liveness.lean` (rung 1) is retained
as the machine-level special case it turned out to be, and for its
per-op WF1 lemmas, which the kits repackage. File/line pointers may
drift; names are the anchors.

## The design's history, honestly (three corrections)

1. **Rung 1** (`Liveness.lean`) stated fairness as predicates on the
   *schedule parameters* (`FairTicks` on pacing, `FairCursor` on
   cursors) and proved `cq_live` directly against the machine. It
   works — but it is the **persistent-enabledness, quiescent
   fragment**, and its proof re-derives protocol content
   machine-side.
2. **The user's challenges**: *what if the leader changes?* and
   *lossy networks with retries?* — both break the rung-1 frame. In
   TLA+, fairness is predicated on **enabledness**
   (`WF: ◇□Enabled ⟹ □◇Taken`, `SF: □◇Enabled ⟹ □◇Taken`), which is
   *state-dependent*: when the leader changes, enabledness of the old
   leader's actions lapses and the fairness obligation transfers;
   when a message is lost, delivery is re-enabled only by a retry —
   recurring, not persistent, enabledness, which is exactly SF's
   territory (rung 1 had claimed "SF has no client": true only
   because the model is fail-stop — no loss exists to retry over).
   Static parameter-predicates and "saturation to a complete final
   pool" cannot express either scenario, nor any non-quiescent
   recurring progress.
3. **The correction — proofs at the denotation** (this document's
   architecture): the temporal structure moves from machine steps to
   **chains in decision space**, where `Values` can carry all the
   protocol reasoning and the machine-side residue is generic.

## The claim, in one paragraph (revised)

The machine run at horizon `T` corresponds — via the corner — to a
**derived decision** `d(T)`, and derivation is horizon-monotone. So
an infinite machine behavior *denotes an ascending chain in the
decision lattice*: cuts extend, delivered pools grow, timer verdicts
accumulate, election-relevant decisions evolve. The decision space
was built as the denotational shadow of the schedule space for safety
(`∀d` covers all runs); its **chains** are the shadow of *infinite
behaviors*. Temporal operators are quantifier shapes over chain
positions (`ChEventually`/`ChEvAlways`/`ChAlwEventually`/`ChLeadsTo`
in `LivenessChain.lean`); **fairness is a class of chains**, stated
in decision vocabulary; **enabledness is expressible** because the
state at a chain point is computed by `Values` from the decisions so
far. All liveness proofs manipulate denotations along chains; the
machine appears only in two generic per-operator kits (below).
**Zero changes to `Sched.lean`, `Values.lean`, or any safety
artifact.**

## Why this is not the classical "fairness isn't denotational" trap

The 1980s verdict (fair merge is not Scott-continuous) rules out
folding fairness into the semantic *domain*: fairness of a behavior
is not determined by its finite prefixes, so no continuous semantics
can compute it. We do not attempt that. The semantics stays exactly
as it is — finite-horizon, fueled — and fairness lives in the
**logic over chains** (an ordinary predicate on `ℕ`-indexed decision
families, unproblematic in type theory). Liveness conclusions are
∃-chain-point shaped, so finite points witness them; only ω-limit
statements (rung (c)) would ever touch completed/infinitary
decisions, and that is the already-queued lfp upgrade.

## The TLA+ Rosetta (re-anchored to chains)

| TLA+ | here | status |
| --- | --- | --- |
| behavior (infinite state sequence) | an ascending chain in decision space (`IsCutChain` for the batch component); the machine's derived chain `cqDerivedChain` is the canonical inhabitant | `LivenessChain.lean` |
| state at a point of a behavior | the `Values` run at the chain point (computed from decisions — no machine state) | by construction |
| `◇P` / `□P` / `◇□P` / `□◇P` / `P ⟿ Q` | `ChEventually` / `ChAlways` / `ChEvAlways` / `ChAlwEventually` / `ChLeadsTo` over chain positions | `LivenessChain.lean` |
| `Enabled A` | a predicate on the `Values` run at the chain point | expressible; first client at rung (b) |
| `WF(tick)` | the **exhaustion** chain class (`Exhausts`: the cut chain eventually consumes the pool) — fairness's denotational shadow | `LivenessChain.lean` (proven end-to-end) |
| `WF(deliver)` | delivery-component cofinality in the sent pool (same shape; kit lemma `deliver_fair_attains` is its transfer) | kit proven (rung 1); chain packaging at rung (a) |
| `SF(·)` | `ChSF`: a coupling between an enabledness trajectory and an action trajectory (`□◇Enabled ⟹ □◇Taken`) | vocabulary in tree; first client = lossy links (needs the model fork below) |
| `◇□(stable leader)` (partial synchrony) | `ChStabilizes` of the election-relevant chain component | vocabulary in tree; rung (b) |
| the WF1 rule | per-**operator**: (i) chain-fairness transfer lemmas (fair schedule ⟹ derived chain in the fairness class), (ii) `Values` point lemmas at good decisions | `cqDerivedChain_*` + `cq_complete_mem` |
| `⟿`-transitivity / lattice rule | `ChLeadsTo.trans` + the register-scan induction (`scan_emit_ind`) — now stated along chains | `LivenessChain.lean` / in tree |
| machine closure | fairness restricts the same chain space safety's `∀d` covers — a fair chain is in particular a safe decision assignment | by construction |
| real-time bounds (`Δ`-synchrony) | **not expressible** (fairness gives unbounded ∃) — same as TLA+ liveness, and deliberate | — |
| stuttering insensitivity | legal ascending chains make good observations persistent: `◇ = ◇□` (`IsCutChain.consumed_persists`, `cq_chain_live_stable`) | proven |

## The architecture: three artifact kinds

Every liveness theorem factors as **V ∘ K1 ∘ K2**:

- **(V) `Values` chain theorems** — schedule-free, decision-
  quantified; *all protocol reasoning here*. Point lemmas at good
  decisions (`cq_complete_mem`: at any complete cut decision, the
  quorum key is in the output — pure `emit_count` contract
  reasoning), lifted along chains (`cq_chain_live`: `◇committed`
  along any exhausting chain; `cq_chain_live_stable`: `◇□committed`
  along legal ascending ones). At Paxos scale these are the
  quiescent-completeness clauses (rung a) and per-good-configuration
  lemmas (rung b) on the colocated contracts.
- **(K1) chain-fairness transfer** — the only place schedule
  vocabulary exists; generic, per-operator: *a fair schedule's
  derived chain is a fair chain*. (`cqDerivedChain_isCutChain`:
  every behavior's derived chain is legal and ascending — no fairness
  needed; `cqDerivedChain_complete_at`/`_exhausts`: `FairTicks`
  makes it exhaust.) Rung 1's WF1 lemmas live on inside these.
- **(K2) tightness** — generic, per-operator, protocol-free: the
  machine observation at `T` *is* the `Values` run at `d(T)`
  (`cq_values_at_derived` — essentially the corner's naming), plus
  emission attainment (`cq_emit_attains`).

`cq_live_chain` is the compiled proof of concept: identical premises
and conclusion to rung 1's `cq_live`, but the glue is a few lines and
the quorum crossing is consumed **only** through the `Values`
contract. This is the transfer property the house architecture
demands, restored for liveness.

**Convergence note (generic-relational sibling)**: K1+K2 have the
free-theorem shape — a relation between machine wires and denotational
pools with an ∃-progress index; `hydro_live M` (the per-program
composition of the kits along the spec walk) is expected to become an
`HRelC` instance + `M_param` once the relational layer hosts
∃-indexed relations. The `emit` law of that instance is blocked on
the `EmitDec` fork below. No shared code yet.

## How the user's two scenarios land

- **Leader change**: the ◇□-stability premise is `ChStabilizes` of
  the election-relevant component of the derived chain ("eventually
  the elected-leader decisions are constant"). The `Values`-side
  content is a per-good-configuration point lemma ("a decision
  containing an uncontested full round for ballot `b` puts the
  commit in the output" — contract reasoning, no schedules). The
  chain argument — fairness drives the chain into a good
  configuration within any stability window — is K1 material,
  composed with `ChLeadsTo`. Enabledness transfer across leader
  changes is automatic because enabledness is a predicate of the
  induced `Values` trajectory, which follows the chain.
- **Lossy networks with retries**: the fairness is `ChSF` — "if the
  retry component recurs unboundedly, the delivery component is
  cofinal on the retried item" (`□◇Enabled(Deliver) ⟹ □◇Deliver`).
  The ALO grade (`RetryPool`, duplicates born at sample sites) is the
  denotational carrier. **Model fork**: today's cursors deliver
  prefixes and can only delay or stop (fail-stop, SCHED_AUDIT F6) —
  loss needs either *skipping cursors* (deliver an unbounded
  subsequence; a small Sched extension with its own safety re-check)
  or application-level loss encoding. The chain vocabulary is ready
  either way; the fork needs a ruling before rung (b′) work starts.

## The `EmitDec` finding (fork 4 — open; recommendation strengthened)

`SchedSem`'s `EmitDec n β = Fin n → List (List β)` is a **finite**
list of per-tick linearization claims; fair skeletons tick forever,
so claim lists exhaust — invisible to prefix-closed safety, a
WF(emit) obligation for liveness. Today's honest spelling is the
*supply premise* (`hsupply` in both `cq_live` and `cq_live_chain`);
it survives the chain factoring (it lives in K2) and at Paxos scale
every emit site would carry one. The claim-*stream* upgrade
(`EmitDec n β := Fin n → Nat → List β`, small and safety-preserving)
would delete the premise class **and** unblock the relational `emit`
law. Recommended as the default ruling when rung (a) at Paxos scale
is commissioned; not implemented here.

## What does Paxos liveness say? (the ladder, revised)

FLP rules out unconditional liveness; the honest family, in proof
order:

**(a) Quiescent-completeness, chain form** *(next implementation
rung; no model changes)*: inputs stabilize + fairness ⟹ the derived
chain exhausts every consumption site (K1, composed through the 5
knots) ⟹ the `Values` run at the exhausted chain point is the total
denotation (V — completeness clauses on the contracts) ⟹ the machine
output attains it (K2). `cq_live_chain` is this rung for
`collect_quorum`.

**(b) Commit liveness under leader stability** *(designed)*: fairness
+ `ChStabilizes`(election component) + a proposed value ⟹
`ChEventually`(committed). Premise-style fork (2) remains open —
(α) wire-level `StabilizesAt` premises vs (β) input-level only with
internal stabilization *derived* (the liveness twin of
`SchedCausal`; β is the recommended headline) — but both spellings
are now *chain* predicates rather than raw machine facts.

**(b′) Lossy-link liveness** *(new, blocked on the skipping-cursor
fork)*: `ChSF` premises over retry/delivery components.

**(c) Liveness under churn** *(parked)*: recurring progress under
unbounded load — the only rung that **forces** the fuel-less/lfp
`Values` upgrade (ω-limit chain points). Deliberately last.

## Model-upgrade assessment

| layer | change needed | forced by |
| --- | --- | --- |
| `Sched.lean` | **none** (schedules already infinite; chains are derived) | — |
| `Sched.lean` `EmitDec` | claim-stream upgrade (small, safety-preserving) | recommended at rung (a)-paxos; required for the relational `emit` law |
| `Sched.lean` cursors | skipping cursors (lossy links) | rung (b′) only |
| `Values.lean` | **none** for rungs (a)/(b) | — |
| `Values.lean` fix | lfp/ω-limit upgrade | rung (c) only |
| safety artifacts | **none** (fairness only shrinks a quantifier that safety leaves universal) | — |

## Non-vacuity discipline

Every liveness theorem ships with (1) a fair witness that is not the
generous schedule (`fairTicks_odd`; the chain prototype `#guard`s the
derived chain's exhaustion point concretely — empty at horizon 0,
complete at the first odd tick), (2) a concrete run whose `T` the
machine computes, and (3) an `example` discharging every premise
jointly — liveness premises (fairness ∧ supply ∧ honesty ∧ stability)
are exactly the kind that can silently conflict (the D37 lesson).
The v2paxos scenario extends to a fair schedule by a generous tail
and should instantiate rung (a) when it lands.

## Desired CORRESPONDENCE.md additions (deferred to that file's next
edit)

- A Rosetta row: "liveness/fairness (WF/SF, ⟿, behaviors)" →
  decision chains, `LivenessChain.lean`, `LIVENESS.md`;
- an update to the "What is deliberately NOT proven" liveness bullet:
  from "future work" to "chain architecture + `cq_live_chain` proven;
  ladder in `LIVENESS.md`";
- `#print axioms` for `cq_live`/`cq_live_chain` migrated into
  `AxCheck.lean`.
