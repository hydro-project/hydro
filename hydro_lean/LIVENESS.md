# Liveness under fairness — decision chains, natively denotational

Design of record for liveness in this tree (`Liveness.lean`, `LivenessChain.lean`),
for a reader who knows TLA+'s story (WF/SF, `⟿`, WF1, machine closure) and wants to see
where each piece lands. Status: the chain architecture is the design; its load-bearing
instance is proven end to end — `cq_live_chain` (`collect_quorum` liveness with all
protocol content at `Values`) and its tightness lemma `collect_quorum_tight`, built from
the generator's own coupling artifacts with no hand mirror (D65). Paxos-scale liveness
is the next rung, queued. History: D48 (rung 1, machine-side), D50 (the chain
correction), D60–D61 (`EmitDec` dissolved), D65 (re-anchored on the corner).

## The claim, in one paragraph

The machine run at horizon `T` corresponds — via the corner (`CORRESPONDENCE.md`) — to a
**derived decision** `d(T)`, and derivation is horizon-monotone. So an infinite machine
behavior *denotes an ascending chain in the decision lattice*: cuts extend, delivered
pools grow, timer verdicts accumulate. The decision space was built as the denotational
shadow of the schedule space for safety (`∀ d` covers all runs); its **chains** are the
shadow of *infinite behaviors*. Temporal operators are quantifier shapes over chain
positions; **fairness is a class of chains**, stated in decision vocabulary;
**enabledness is expressible** because the state at a chain point is computed by
`Values` from the decisions so far. All liveness proofs manipulate denotations along
chains; the machine appears only in two generic per-operator kits. Zero changes to
`Sched.lean`, `Values.lean`, or any safety artifact.

This is not the classical "fairness isn't denotational" trap: the semantics stays
finite-horizon and fueled; fairness lives in the **logic over chains** (a predicate on
`ℕ`-indexed decision families). Liveness conclusions are ∃-chain-point shaped, so finite
points witness them; only ω-limit statements (rung c) would touch completed decisions.

## The TLA+ Rosetta

| TLA+ | here | status |
|---|---|---|
| behavior (infinite state sequence) | an ascending chain in decision space (`IsCutChain` for the batch component); the machine's derived chain `cqDerivedChain` is the canonical inhabitant | `LivenessChain.lean` |
| state at a point of a behavior | the `Values` run at the chain point | by construction |
| `◇P` / `□P` / `◇□P` / `□◇P` / `P ⟿ Q` | `ChEventually` / `ChAlways` / `ChEvAlways` / `ChAlwEventually` / `ChLeadsTo` | `LivenessChain.lean` |
| `Enabled A` | a predicate on the `Values` run at the chain point | expressible; first client at rung (b) |
| `WF(tick)` | the **exhaustion** chain class (`Exhausts`: the cut chain eventually consumes the pool) | proven end to end |
| `WF(deliver)` | delivery-component cofinality in the sent pool (`FairCursor`, `deliver_fair_attains` — `Liveness.lean`) | kit proven; chain packaging at rung (a) |
| `SF(·)` | `ChSF`: `□◇Enabled ⟹ □◇Taken` as a coupling of two trajectories | vocabulary; first client = lossy links (model fork) |
| `◇□(stable leader)` | `ChStabilizes` of the election-relevant chain component | vocabulary; rung (b) |
| the WF1 rule | per-**operator**: (i) chain-fairness transfer lemmas, (ii) `Values` point lemmas at good decisions | `cqDerivedChain_*` + `cq_complete_mem` |
| machine closure | fairness restricts the same chain space safety's `∀ d` covers | by construction |
| real-time bounds | not expressible (fairness gives unbounded ∃) — deliberate | — |
| stuttering insensitivity | legal ascending chains make good observations persistent: `◇ = ◇□` (`IsCutChain.consumed_persists`, `cq_chain_live_stable`) | proven |

## The architecture: V ∘ K1 ∘ K2

Every liveness theorem factors as:

- **(V) `Values` chain theorems** — schedule-free, decision-quantified; *all protocol
  reasoning here*. Point lemmas at good decisions (`cq_complete_mem`: at any complete cut
  decision the quorum key is in the output — pure `emit_count` contract reasoning),
  lifted along chains (`cq_chain_live`: `◇committed` along any exhausting chain;
  `cq_chain_live_stable`: `◇□committed` along legal ascending ones).
- **(K1) chain-fairness transfer** — the only place schedule vocabulary exists; generic,
  per operator: *a fair schedule's derived chain is a fair chain*
  (`cqDerivedChain_isCutChain`: every behavior's derived chain is legal and ascending;
  `cqDerivedChain_complete_at`/`_exhausts`: `FairTicks` makes it exhaust).
- **(K2) tightness** — generic, protocol-free: the machine observation at `T` *is* the
  `Values` run at `d(T)`. `collect_quorum_tight`: the corner's `wf` for the module is the
  `tick` block's body coupling (`CoTick.scanWf`, from the generated
  `collect_quorum_co_wf₁`), under which the machine former coerced is the denotation
  former on the coerced inputs (`CoTick.scan_comm`); the derived batch decision is exact
  at its own horizon (`corr_batch`); the corner's denotation leg is named at the explicit
  derived decision by `co_transfer`. No hand mirror of the block exists.

`cq_live` (statement unchanged since D48) is proven only this way; `cq_live_chain` names
the same theorem. Its premises: a stabilized input (`StabilizesAt`), the usage caps, a
key at quorum, and `FairTicks`-shaped supply — nothing about emission (the former
`EmitDec` supply premise is gone with the decision, D60–D61).

## How the two hard scenarios land

- **Leader change**: the premise is `ChStabilizes` of the election-relevant component of
  the derived chain ("eventually the elected-leader decisions are constant"). The
  `Values`-side content is a per-good-configuration point lemma (a decision containing an
  uncontested full round for ballot `b` puts the commit in the output — contract
  reasoning). The chain argument (fairness drives the chain into a good configuration
  within any stability window) is K1 material composed with `ChLeadsTo`. Enabledness
  transfer across leader changes is automatic because enabledness is a predicate of the
  induced `Values` trajectory.
- **Lossy networks with retries**: the fairness is `ChSF` on retry/delivery components;
  the ALO grade (`RetryPool`) is the carrier. **Model fork**: today's cursors deliver
  prefixes (fail-stop, `SCHED_AUDIT.md` F6) — loss needs skipping cursors (a `Sched`
  extension with its own safety re-check) before rung (b′).

## The ladder

FLP rules out unconditional liveness; the honest family, in proof order:

- **(a) Quiescent-completeness, chain form** *(next rung; no model changes)*: inputs
  stabilize + fairness ⟹ the derived chain exhausts every consumption site (K1 composed
  through `paxos_core`'s five knots) ⟹ the `Values` run at the exhausted point is the
  total denotation (V — completeness clauses on the contracts) ⟹ the machine output
  attains it (K2 — the generic tightness kit generalizing `collect_quorum_tight`).
  `cq_live_chain` is this rung for `collect_quorum`.
- **(b) Commit liveness under leader stability** *(designed)*: fairness +
  `ChStabilizes`(election component) + a proposed value ⟹ `ChEventually`(committed).
  Premise fork: (α) wire-level `StabilizesAt` premises vs (β) input-level premises with
  internal stabilization *derived* — β is the recommended headline; both are chain
  predicates.
- **(b′) Lossy-link liveness** *(blocked on skipping cursors)*: `ChSF` premises.
- **(c) Liveness under churn** *(parked)*: the only rung that forces the lfp/ω-limit
  `Values` upgrade. Deliberately last.
- **`hydro_live`**: K1+K2 have the free-theorem shape (a relation between machine wires
  and denotational pools with an ∃-progress index); the per-program composition along
  the spec walk is expected to become an `HRelC` instance + `M_param`. The `emit` law
  that blocked it is moot since `EmitDec` is gone.

| layer | change needed | forced by |
|---|---|---|
| `Sched.lean` | none (schedules are already infinite; chains are derived) | — |
| `Sched.lean` cursors | skipping cursors | rung (b′) only |
| `Values.lean` | none for (a)/(b) | — |
| `Values.lean` fix | lfp/ω-limit upgrade | rung (c) only |
| safety artifacts | none (fairness only shrinks a quantifier safety leaves universal) | — |

## Non-vacuity discipline

Every liveness theorem ships with (1) a fair witness that is not the generous schedule
(`fairTicks_odd`; the chain prototype `#guard`s the derived chain's exhaustion point —
empty at horizon 0, complete at the first odd tick), (2) a concrete run whose `T` the
machine computes, and (3) an `example` discharging every premise jointly — liveness
premises (fairness ∧ supply ∧ honesty ∧ stability) are exactly the kind that can
silently conflict (`DOCTRINE.md` R7).
