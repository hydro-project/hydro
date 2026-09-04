# HydroLean: Overview

HydroLean is a Lean 4 mechanization of the semantic foundations of
[Hydro](https://hydro.run) — the **Flo** streaming semantics (dissertation Ch. 2) and
the **Gyatso** stream-choreographic semantics (Ch. 3) — together with a
proof-oriented surface layer for **writing Hydro programs in Lean and proving
unbounded correctness**: all input sizes, all materializations of nondeterminism.

It is built on core Lean 4 only (no Mathlib), with **zero `sorry`** anywhere; headline
theorems are axiom-audited to `propext`/`Classical.choice`/`Quot.sound` (see
`HydroLean/Flo/AxiomCheck.lean`, `Programs/Paxos/AxCheck.lean`,
`Programs/AxCheck.lean`).

## The central thesis

Hydro's type system **defers the materialization of nondeterminism**. Traditional
distributed-systems semantics branch at every network delivery, so proofs quantify
over all interleavings. Hydro instead:

1. **Quotients nondeterminism into collection types.** A `NoOrder` stream denotes a
   multiset (`HydroLean.Multiset`, a quotient of `List` by permutation): arrival order
   is not *unknown* — it is *unrepresentable*. Network fan-in is a deterministic
   function into the quotient (now: the `NoOrder` shuffle is `batchC` decision data, `Hydro/TStream.lean`).
2. **Proves scheduler-irrelevance once, in the metatheory.** Confluence + eager
   execution (`Flo/Theorems.lean`, `Graph.unique_stuck`) collapse trace semantics to
   functional semantics: settled outputs are a *function* of inputs.
3. **Materializes nondeterminism only at explicit guard sites.** Every Rust
   `nondet!(...)` guard becomes a structured, universally-quantified decision value
   in Lean (batch cuts / consumed batches, snapshot views, cycle fuel — docs/10;
   arrival interleavings are quotiented into `batchC` decision data). A theorem
   about a program with N guard sites quantifies over exactly N structured choices —
   the same decision space the Rust deterministic simulator's hooks script, so
   bounded sim tests and unbounded theorems share their specs, and a failing proof
   obligation's witness is a sim-script repro.

Consequences that show up everywhere in this codebase:

- **Correctness properties are derived from types, not hand-stated**: prefix
  monotonicity comes from the `→ₘ` wire types (`Hydro/Growth.lean`, whose
  growth orders *are* the Rust ordering markers), value ascent from the
  `MonoSing` singleton wire type (`Hydro/MonoSing.lean`, the Rust `Monotonic`
  bound), and determinism from decision-invariance faces (a theorem whose
  right-hand side mentions no decision, e.g. `twoPC_committed_eq`).
  (Historically this discipline was "sealing" at quotient carriers —
  `Hydro/Sealed.lean`, deleted; docs/03.)
- **Rust `manual_proof!` promises become real hypotheses**: `fold_commutative`'s
  commutativity is `Multiset.AccComm`, demanded by `Multiset.foldComm`'s
  `Quotient.lift`; the `monotone =` promise is `fold_monotonic`'s closure
  obligation, paid at the definition site for a `MonoSing` output type.
- **English `nondet!` justification comments become formal artifacts**: a
  decision-invariance theorem ("locally resolved"), a derived guarantee
  (comments that *claim* guarantees get proven — e.g. Paxos's
  `LeaderBallotStable`, FINDINGS D21), or a named proof input of the
  verified face (genuine environmental assumptions only).

## The proof surface (current form: decisions-as-inputs)

Programs are literal Lean functions over tick-located streams
(`Hydro/TStream.lean`), written **once** as typed monotone stages (`→ₘ`
M-form, `Hydro/Growth.lean`; `Monotonic` singleton wires `MonoSing.lean`)
with **colocated verified faces**: input properties are hypotheses, output
properties are clauses, every `nondet!` guard is an explicit decision input,
and theorems quantify over all decisions. `forward_ref` cycles are Kleene
fixpoints (`ForwardRef.lean`). See [10-decisions-as-inputs.md](10-decisions-as-inputs.md).

> *Historical*: two earlier architectures — the sealed-component /
> `Holds`-simulation three-layer stack (docs/02–05) and the choreographic /
> TGraphSys layer (docs/08–09) — were superseded and deleted after their
> deliverables were migrated; the docs remain as the design record.

Underneath sits the mechanized metatheory (`Flo/`, `Gyatso/`) that
*justifies* the collapse from schedules to functions — see
[01-metatheory.md](01-metatheory.md).

## Project goals and status

- **(a) `collect_quorum` unbounded correctness — done.**
  `Programs/CollectQuorumProof.lean` (`collectQuorum_correct`): both branches
  (min = max and min < max), all inputs, all batchings — with the typed-stage
  face `collect_quorum_spec` at the module boundary
  (`CollectQuorumStreams.lean`).
- **(b) Medium programs — done.** Two-phase commit as a wire composition
  over the **shared** quorum stage (`TwoPC.lean`; master theorem
  `twoPC_committed_eq` in `TwoPCProof.lean`, with a decision-free right-hand
  side); `index_payloads` and `join_responses` (Paxos building blocks) as
  typed stages with verified faces and unbounded theorems. (`sequence_payload`
  lives in the Paxos port, `Programs/Paxos/SequencePayload.lean`. The
  open-membership negative results were retired with the old sealing layer;
  membership-as-unordered-input is recorded future work — FINDINGS.)
- **(c) Paxos — safety proven.** The full `paxos_core` port
  (`Programs/Paxos/`, 1:1 file-per-Rust-function over the
  decisions-as-inputs surface, docs/10), the headline **`commit_agreement`**
  (`Programs/Paxos/Safety.lean`: cross-proposer slot-functionality of
  `p_to_replicas` for the minimally-fixed `guarded` variant, over the full
  decision space; standard axioms only, `Paxos/AxCheck.lean`), and two
  **executable falsifications of candidate bugs in the real paxos.rs**
  (`lake exe falsify`; `Falsification.lean`). Per ACCEPTANCE §7 the
  operational-lens transport is explicitly out of scope (the old operational
  stack was deleted; jj history is the archive). See
  [06-paxos.md](06-paxos.md) and [ACCEPTANCE.md](../ACCEPTANCE.md) for the binding
  final-state criteria.

Formalization findings (dissertation errata, Rust bug candidates, implicit
contracts) are cataloged in [FINDINGS.md](../FINDINGS.md) and narrated in
[07-findings-and-methodology.md](07-findings-and-methodology.md).

## Building

```bash
curl -sSfL https://raw.githubusercontent.com/leanprover/elan/master/elan-init.sh -o /tmp/elan-init.sh
ELAN_HOME="$HOME/.elan" sh /tmp/elan-init.sh -y --no-modify-path --default-toolchain none
export PATH="$HOME/.elan/bin:$PATH"
elan toolchain install "$(cat lean-toolchain)"
cd hydro_lean && lake build
```

Every module under `HydroLean/` compiles automatically (lakefile glob). `#guard`
tests — the Lean mirrors of Rust sim tests, running the *same specs* on concrete
schedules — execute during compilation, so a green build means they all passed.

## Repository layout

```
HydroLean/
  Prelude.lean        abstract rewriting: Star, confluence, Newman's lemma, SN
  Flo/                Ch. 2 metatheory (collections, operators, graphs, theorems)
  Gyatso/             Ch. 3 metatheory (locations, clusters, network ops, Thm 3.4.*)
  Collections/        Multiset (quotient), keyed views, DupList, Coll instances
  Hydro/              the surface + proof layers (docs 09–10; the decision
                      surface TStream/ForwardRef, wire types Growth/MonoSing)
  Programs/           verified programs: quorum, 2PC, index_payloads,
                      join_responses, Paxos, axiom audits
docs/                 this documentation
DESIGN.md             architecture decisions and naming policy
ACCEPTANCE.md         the binding final-state bar
FINDINGS.md           errata, bugs, contracts surfaced by formalization
SORRIES.md            deferred items (no sorries exist in code)
```

## Reading order

1. This overview, then [10-decisions-as-inputs.md](10-decisions-as-inputs.md) —
   the architecture: programs as typed monotone stages, nondeterminism as
   decision inputs, `forward_ref` fixpoints.
2. [06-paxos.md](06-paxos.md) — the capstone: the Paxos port, the
   `commit_agreement` proof, and the executable falsifications.
3. [11-causal-availability.md](11-causal-availability.md) — the causality
   boundary rule: trigger-gated loops inherit tick causality from the
   fixpoint (`le_ballot_stable` derived); the shelved `causalTickLoop`
   design for gateless loops.
4. [01-metatheory.md](01-metatheory.md) — the Flo/Gyatso metatheory that
   licenses the collapse from schedules to functions.
5. [07-findings-and-methodology.md](07-findings-and-methodology.md) — what
   formalization surfaced, and how.
6. *Historical* (superseded designs, kept as the record):
   [02-writing-programs.md](02-writing-programs.md),
   [03-proving-determinism.md](03-proving-determinism.md),
   [04-components-and-composition.md](04-components-and-composition.md),
   [05-simulation-and-holds.md](05-simulation-and-holds.md) — the
   sealed-component / `Holds` stack;
   [08-choreographic-layer.md](08-choreographic-layer.md),
   [09-configuration-calculus.md](09-configuration-calculus.md) — the
   choreographic layer and the configuration calculus (whose two key insights
   carry over into docs/10).
