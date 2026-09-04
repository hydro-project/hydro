# The Metatheory: Flo (Ch. 2) and Gyatso (Ch. 3) in Lean

This layer mechanizes the dissertation's formal semantics. Its role in the larger
project is *foundational, not operational*: the program-level proofs in `Programs/`
do not import `Flo/` directly — instead, the metatheorems here justify the proof
architecture (functional denotations, schedule-irrelevance, quotient channel types)
that the surface layers implement concretely. Where a surface-layer theorem is a
concrete instance of a metatheorem, its docstring says so.

## Abstract rewriting (`Prelude.lean`)

Flo's metatheory is at heart a theory of abstract rewriting systems. `Prelude.lean`
provides: `Star` (reflexive-transitive closure, plus the head-recursive `Star'`),
`Joinable`, `Confluent`, `LocallyConfluent`, `Stuck` (normal forms), `NormalizesTo`,
`SN` (strong normalization), and the standard results — unique normal forms under
confluence (`NormalizesTo.unique`), existence under SN (`exists_normalizesTo`), and
**Newman's lemma** (`newman`, and the rooted variant `newman_rooted` used for
graph-level determinism).

## Collections (`Flo/Collection.lean`, §2.3.2–§2.3.3)

A collection language `L_C` is the algebraic core of a stream's value domain:

```lean
structure Coll where
  C : Type u
  concat : C → C → C        -- ++ : how deltas are absorbed
  empty : C                 -- right identity
  fix : C → C               -- equivalent-but-fixed value (e.g. append terminator ⊗)
  concat_empty : ∀ c, concat c empty = c
  fix_fixed : ∀ c δ, concat (fix c) δ = fix c
```

with `Coll.Fixed c ↔ ∀ δ, c ++ δ = c` (§2.3.2). Deliberate deviations from the paper
(documented in `DESIGN.md`): collection *expressions* are collapsed into values (the
paper's well-formedness laws make them isomorphic), and the set-valued "collection
types" `T_C` are replaced by carrier types themselves (refinements become subtypes or
quotients). Stream types add a `Boundedness` flag (`bounded`/`unbounded`) with the
Fig 2.2 subtyping (`Boundedness.le`). Multi-port tuples are heterogeneous lists
(`Vals : List Coll → Type` with pointwise `concat`, `empty`, `fixAll`, `FixedWhere`).

Concrete `Coll` instances (ordered sequences with terminator, multisets, overwrite
singletons, LVars, dup-lists) live in `Collections/Instances.lean` and
`Gyatso/LocalColl.lean` (the `flagged` carrier combinator: payload × three-state
terminator flag, with the *re-arming* `fix` — see finding A2 below).

## Operators (`Flo/Operator.lean`, §2.3.4–§2.3.6)

An `Operator ins outs` carries a `State` (the paper's operator expression), the
small-step `step : Vals ins → State → Vals ins → State → Vals outs → Prop`
(consume inputs, update state, emit a delta), boundedness annotations, and a
configuration invariant `Inv` (below). `Operator.OpStep` is the paper's `→_O`
(delta concatenated to output buffers). The proof obligations are bundled as
`Operator.Lawful`:

- `wf_decreasing` — a well-founded order each step decreases (the paper's "finite,
  downwards-closed ≺"); yields `Lawful.sn` = **Lemma 2.3.1 (operator stuck state)**.
- `confluent` — confluence of `OpStep`.
- `eager` — **Def 2.3.1 (eager execution)** in joinability form: introducing an input
  delta before or after a step yields joinable configurations.
- `progress` — **Def 2.3.3 (streaming progress)**: from any *consistent*
  configuration whose bounded inputs are fixed, every reachable stuck state has
  maximal outputs (`OutputsMaximal`, Def 2.3.2) and fixed bounded outputs.
- `inv_step`, `inv_delta` — closure of `Inv` under steps and input deltas.

### Finding A1: the `Inv` amendment (an erratum for the paper)

As literally stated, Defs 2.3.2/2.3.3 quantify over **all** configurations —
including inconsistent ones unreachable from any run (e.g. a `fold : B ↩→ B` whose
input records a consumed terminator paired with an output buffer that never received
the aggregate). From such states the required conclusions are *false for every
terminator-forwarding operator, including the paper's own `map`/`fold` (Fig 2.14)*.
The paper implicitly assumes configurations arise from executions. HydroLean makes
that explicit: operators supply `Inv : Vals ins → State → Vals outs → Prop`
(default `True`; sufficient for `U ↩→ U` operators), `Progress` is restricted to
`Inv`-consistent configurations, and the graph layer threads a structural lifting
`Graph.Consistent` through Lemma 2.4.4. Fresh programs satisfy `Inv` by
construction. Determinism and eager execution are unaffected. A related repair
(finding A2): when terminator consumption is recorded in the input value, `fix`
must *re-arm* (`consumed ↦ pending`) for Output Maximality to be satisfiable —
`pending` still absorbs concatenation, so `fix_fixed` holds.

Details: [FINDINGS.md](../FINDINGS.md) §A.

## Graphs (`Flo/Graph.lean`, §2.4) and the headline theorems (`Flo/Theorems.lean`)

Graphs are `e ::= {S}[op] | e;e | e|e` (Fig 2.4), intrinsically typed by port
collections, with buffered inputs at the leaves. The small-step `Graph.Step`
(Fig 2.6) forwards a left subgraph's emitted delta into the right subgraph's buffers;
`Graph.CStep` acts on configurations `(g, O)`. Boundedness typing is the separate
judgment `Graph.WT` (Fig 2.5).

The metatheorems (all fully proven; machinery in `Flo/MetaAux.lean`,
`Flo/Confluence.lean`, `Flo/Progress.lean`):

| Theorem | Statement (informal) | Paper |
|---|---|---|
| `Graph.sn_of_leavesLawful` | graphs with lawful leaves always reach a stuck state | Lemma 2.4.2 |
| `Graph.deterministic_and_eager` | Determinism (Def 2.4.1) + Eager Execution (Def 2.4.2), by mutual structural induction; the sequential case is the §2.4.2 trace-rewriting argument, assembled via `newman_rooted` | Lemma 2.4.3 |
| `Graph.unique_stuck` | **every configuration normalizes to a unique stuck state** — the theorem that licenses functional denotations for whole dataflows | — |
| `Graph.progress_of_wt` | well-typed + consistent + bounded-inputs-fixed ⇒ bounded outputs fixed at every reachable stuck state | Lemma 2.4.4 |

Deferred (statement only, tracked in [SORRIES.md](../SORRIES.md)): the output-
*maximality* half of graph-level streaming progress.

## Gyatso (`Gyatso/`, Ch. 3)

- **Locations** (`Location.lean`): process/cluster tags, located stream types, and a
  located typing judgment `Graph.LWT` (computational operators keep all ports at one
  location; designated network operators may bridge two) — locations are erased from
  the operational semantics, as in the paper (§3.3).
- **Clusters** (`Multibuffer.lean`, §3.3.3): `multiColl n L` (carrier `Fin n → L.C`,
  pointwise laws) and the SPMD upgrade `Operator.clusterOp` — a cluster of `n`
  copies of an operator, stepping a nondeterministically chosen member — with the
  full lawfulness transfer (`clusterOp.lawful`): WF via the finite-product
  one-member-decreases order (`OneDec`, `oneDec_wf`), confluence via commutation of
  distinct members + Newman, eager/progress componentwise (Figs 3.5/3.6).
- **Network operators** (`Network.lean`, §3.5): `networkO2o` (TCP FIFO, Fig 3.7),
  `networkO2oUnord` (unordered delivery over the multiset carrier, Fig 3.10), and
  the `foldCommutative` operator (Fig 3.11) whose *commutativity hypothesis is the
  operational justification for Hydro's `fold_commutative` API* — each with complete
  `Lawful` proofs, built through the single-port `Op1` framework (`Op1.lean`) that
  does the `Vals` plumbing once.
- **Distributed correctness** (`Correctness.lean`, §3.4): `NaturalOrder` (Def 3.4.1),
  **`monotone_outputs` (Thm 3.4.2)** — along *any* trace (including ones stuck
  because crash-stop failures froze part of the schedule, §3.4.2), outputs only grow
  in their natural order — and **`eventual_determinism` (Thm 3.4.1)** as a corollary
  of `Graph.unique_stuck`, plus `outputs_below_settled`.

## What this layer licenses downstream

- `Graph.unique_stuck` + `eventual_determinism` are the abstract ancestors of the
  proof surface's functional denotations (docs/10): settled outputs of a
  multi-location dataflow are a function of its inputs and its decision trace —
  which is why programs can be literal Lean functions and safety can be stated
  as ∀-decisions, with no schedule quantifier. (Historically this licensed the
  sealing discipline, doc 03, and GraphSys adequacy, doc 05 — both retired.)
- `monotone_outputs` (Thm 3.4.2) is the abstract ancestor of the type-derived
  growth discipline: the `Growth` (⊑) orders and `→ₘ` wire types
  (`Hydro/Growth.lean`) make every stage's prefix-monotonicity a typing fact,
  and `MonoSing` (`Hydro/MonoSing.lean`) carries value ascent in signatures.
- Eager execution (Def 2.3.1/2.4.2) is the abstract ancestor of the batch
  decision semantics (`TStream.lean`): consuming an upstream's history in
  arbitrary cuts is just another decision, so retiming never changes settled
  outputs.
- The `foldCommutative` operator's lawfulness is the operational counterpart of
  `Multiset.foldComm`'s `Quotient.lift`: the same commutativity witness, at
  the two ends of the stack.
