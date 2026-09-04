# HydroLean: Formalizing Hydro (Flo + Gyatso + the Hydro surface language) in Lean 4

Goal: write entire Hydro programs in Lean and prove **unbounded correctness** (all input
sizes, all nondeterministic schedules), exploiting the dissertation's key insight:

> The type system defers materialization of nondeterminism. Collections quotient network
> nondeterminism away (multisets for reordering, dup-collapse for retries); safe operators
> are congruences on those quotients, so determinism is preserved by typing; nondeterminism
> becomes observable only at explicit `NonDet`-guarded sites (batching, snapshots,
> assume_*). Proofs therefore quantify over N structured choices at guard sites, never over
> global interleavings. The Flo/Gyatso metatheory (confluence / eager execution / eventual
> determinism), proven once, is what licenses this collapse from trace semantics to
> functional semantics.

## Environment

- Lean 4 (v4.33.1, pinned in `lean-toolchain`), **no external dependencies** (no Mathlib:
  the toolchain's bundled clang cannot run on this host's old glibc, so `lake exe cache`
  is unavailable). We build needed collection types from scratch on core Lean.
- Build: `export ELAN_HOME=$TMPDIR/elan; export PATH=$ELAN_HOME/bin:$PATH` (install elan
  first if missing: `curl -sSfL https://raw.githubusercontent.com/leanprover/elan/master/elan-init.sh -o /tmp/../elan-init.sh; sh ... -y --no-modify-path --default-toolchain none; elan toolchain install stable`),
  then `lake build` in `hydro_lean/`.
- **No `sorry` may remain in merged work** unless tracked in `SORRIES.md` with an owner
  and a plan. Prefer strengthening hypotheses over admitting gaps.
- Style: `namespace HydroLean`, doc comments on every public def/theorem, cite the
  dissertation (e.g. "Def 2.3.1 (Eager Execution)") in doc comments so the formalization
  is auditable against the paper.
- **Naming policy (Rust parity)**: surface-API defs (operators, networking, collection
  APIs — anything a Hydro user calls in Rust) mirror the Rust API names verbatim in
  snake_case (`broadcast_closed`, `send_bincode`, `fold_commutative`, `assume_ordering`,
  `tick_batch`, ...). Where a Lean def was created before its Rust counterpart was
  identified, add the Rust-named `abbrev` and migrate call sites; the Rust name is
  canonical. Lemma/theorem names remain Lean-idiomatic. Semantic differences from the
  Rust API of the same name are bugs (or FINDINGS.md entries), not naming choices.

## Layout (as built)

```
HydroLean/
  Prelude.lean       -- abstract rewriting: Star, confluence, Newman (plain + rooted),
                     -- stuckness, normalization, SN
  Collections/       -- quotient collection library (core-Lean, no Mathlib)
    Multiset.lean    -- Quotient of List by Perm; count/countP, map/filter/filterMap, add,
                     -- foldComm (commutativity as a hypothesis: Rust `fold_commutative`),
                     -- MNodup, ext, induction
    Keyed.lean       -- per-key views: keyVals, countKey(P), filterKey, antiJoin, ...
    DupList.lean     -- [T]dup: adjacent-duplicate collapse (AtLeastOnce), assoc concat
    Instances.lean   -- Coll instances: seqColl, msetColl, dupColl, singletonColl, lvarColl
  Flo/               -- Chapter 2, fully mechanized (zero sorry)
    Collection.lean  -- Coll (carrier, ++, ∅, fix, fixed); Vals port tuples; FixedWhere
    Operator.lean    -- Operator (state, →δ, per-port bounds, consistency invariant Inv);
                     -- Lawful obligations; Lemma 2.3.1 (operator stuck state)
    Graph.lean       -- e ::= op | e;e | e|e; typing WT; Consistent; small-step (Fig 2.6)
    Theorems.lean    -- Lemmas 2.4.2 (SN), 2.4.3 (determinism + eager execution),
                     -- 2.4.4 (streaming progress), unique_stuck (the denotational collapse)
    MetaAux/Confluence/Progress.lean -- proof infrastructure (Newman-based)
    AxiomCheck.lean  -- #print axioms audits
  Gyatso/            -- Chapter 3, fully mechanized
    ARS.lean         -- aliases into Prelude's Newman lemmas
    LocalColl.lean   -- flagged-carrier Coll combinator; seqC/msetC/singC (Mset = alias
                     -- of Collections' Multiset)
    Op1.lean         -- single-port operator spec + Lawful transfer; maximality machinery
    Location.lean    -- process/cluster locations; located typing judgment LWT
    Multibuffer.lean -- multiColl; clusterOp upgrade (Fig 3.5/3.6) + Lawful lift
    Network.lean     -- networkO2o (FIFO), networkO2oUnord, foldCommutative + Lawful
    Correctness.lean -- NaturalOrder; Thm 3.4.2 (monotone outputs); Thm 3.4.1
  Hydro/             -- the proof-oriented surface language
    Markers.lean     -- Boundedness / StreamOrder / Retries markers, carrier table
    NonDet.lean      -- Batching, SnapshotSchedule: nondet as data
    Tick.lean        -- TickLoop (sliced! model), run lemmas, run_characterize
    Stream.lean      -- List-carrier ops (TotalOrder+ExactlyOnce)
    StreamLemmas.lean-- shared List/keyed-count lemma library (canonical home)
    Join.lean        -- Stream.join (list equi-join)
    TStream.lean     -- decisions-as-inputs surface: ticks×collections, batch/snapshot
                     -- regimes keyed by ordering marker; Consumes legality (docs/10)
    ForwardRef.lean  -- forward_ref cycles as higher-order Kleene fixpoints; fixpoint
                     -- induction; gas-less budget bridges; memoF
    Growth.lean      -- Growth (⊑) orders = ordering markers; bundled →ₘ monotone maps,
                     -- reader-lifted wire combinators, MonoMap.fix cycles
    MonoSing.lean    -- Monotonic singleton wires; fold_monotonic (closure obligation
                     -- at the definition site for a monotonic output type)
    ClusterFamily.lean -- clusters as member-indexed stream families; fan-in exchange
  Programs/          -- verified Hydro programs (Rust ports; every program is its
                     -- typed M-form stage + a colocated verified face)
    CollectQuorum.lean / ...Proof  -- the quorum tick engine + the goal-(a)
                     -- unbounded proof (both branches; batching-free counts)
    CollectQuorumWithResponse.lean -- the with-response tick engine
    CollectQuorumMinMax.lean       -- min = max count characterization (shared
                     -- by two_pc and available to any collect_quorum consumer)
    CollectQuorumStreams.lean      -- collect_quorumM / collect_quorum_with_responseM
                     -- typed stages + verified faces (QuorumSpec, quorum extraction)
    PaxosQuorumModel.lean / PaxosQuorumCount.lean -- unconditional (off-contract)
                     -- quorum soundness + (key,value) count caps consumed by the
                     -- Paxos agreement proof
    IndexPayloads.lean             -- index_payloads: tick engine + typed stage
                     -- (index_payloadsM) + slot faces
    JoinResponses.lean             -- join_responses: tick engine + typed stage
                     -- (join_responsesM) + the join face
    TwoPC.lean / TwoPCProof.lean   -- two_pc over the shared quorum stage (two_pcM)
                     -- + master theorem + corollaries (goal b)
    Paxos/           -- paxos.rs port (goal c): one file per Rust function over the
                     -- decisions-as-inputs surface; commit_agreement (Safety.lean);
                     -- executable falsifications (FINDINGS.md B1/B2, `lake exe falsify`)
```

See `FINDINGS.md` for paper gaps, candidate Rust bugs, and formalized contracts
discovered along the way; `SORRIES.md` tracks deferred work (currently: the
output-maximality half of graph streaming progress, statement not yet formalized).

## Architecture: the proof surface

> **Era note.** Earlier phases proved the (a)/(b) deliverables in a
> three-layer architecture (Sim/SimGraph operational ground truth, sealed
> `Multiset → Multiset` components, adequacy bridges) and before that in a
> choreographic/TGraphSys layer. Both were superseded by the
> **decisions-as-inputs** surface (docs/10) and deleted as unused; the
> deliverables were migrated: programs are typed monotone stages (`→ₘ`,
> M-form, `Hydro/Growth.lean`) with colocated verified faces (input
> properties as hypotheses, output properties as clauses — Paxos style),
> nondeterminism is explicit decision inputs, and unbounded correctness is
> stated over all decisions. Recover the old layers from jj history; the
> docs/02–05 series documents them as historical record.

## Semantic conventions

### Two semantic layers, one justification

1. **Denotational (surface) layer** used for program proofs: a stream with markers
   `(O, R)` denotes a value in a **quotient carrier**:
   - `TotalOrder, ExactlyOnce` → `List T`
   - `NoOrder,   ExactlyOnce` → `Multiset T` (quotient of `List` by `Perm`)
   - `TotalOrder, AtLeastOnce` → `DupList T` (quotient by adjacent-dup collapse)
   - `NoOrder,   AtLeastOnce` → `Multiset T` quotiented further by cardinality collapse
     (elements present-with-unknown-multiplicity); v1 may use `Finset`-like `SupSet`.
   Safe operators are total functions on carriers; operators with algebraic side
   conditions (`fold_commutative`, `fold_idempotent`) take those conditions as
   **hypotheses** and are defined via `Quotient.lift` — the Rust `manual_proof!` becomes a
   real proof term.
2. **Operational (core) layer** (Flo/Gyatso): small-step machines over buffered
   collections. The metatheorems (determinism/eager execution/eventual determinism)
   justify that the denotational layer is *the* semantics of any well-typed composition.
   (A mechanized per-operator bridge between the two layers was considered and is
   explicitly out of scope — ACCEPTANCE §7; the justification is the metatheory's
   role, per docs/01.)

### Unbounded correctness

"Unbounded" = for **all** finite inputs of arbitrary size and **all** materializations of
nondeterminism (vs. the Rust simulator which explores bounded instances). Standard theorem
shape for a program `P` with guard sites `g₁ : NonDet C₁, …`:

```
theorem P_correct (input : Carrier …) (c₁ : C₁) … (cₙ : Cₙ) :
    P input c₁ … cₙ = spec input          -- determinism + functional correctness
theorem P_monotone (input prefix : …) : prefix ≤ input → P prefix … ≤ P input …
```

Liveness-style statements are phrased via settled values (eventual determinism), safety
via natural-order monotonicity (Thm 3.4.2).

### NonDet = structured adversarial choice

`NonDet` guard parameters become explicit universally-quantified values whose types
capture exactly the choice space (mirroring the simulator's hooks):

- `Batching (α)`: a partition of the input into a (finite or infinite) sequence of
  per-tick chunks: for `NoOrder` a `List (Multiset α)` with `sum = input`; for
  `TotalOrder` a `List (List α)` with `join = input`.
- `Snapshot`: a monotone sampling: for each tick, a prefix-point of the evolving value.
- `Interleave`: for m2o networking, a merge of per-sender sequences into arrival order
  (only observable if the consumer materializes order — otherwise quotiented away).
- `assume_ordered` / `assume_exactly_once`: a chosen representative of the equivalence
  class, together with a proof obligation or an explicit adversarial choice.

### Ticks / `sliced!`

A tick block = transition system `step : State → InBatch → State × OutBatch` (pure!),
with `state_null` cycle variables inside `State`. Running it over a batching yields
`allTicks : List OutBatch`; `all_ticks` output = concatenation/sum. Cross-location
programs (Paxos) = a network of such systems, where the only adversarial inputs are the
batchings/interleavings at guard sites; local steps are pure functions.

### Faithfulness notes (deliberate deviations from the paper)

- We collapse the paper's collection *expressions* `E_C` into collection *values* (the
  paper's well-formedness `⟦⌊c⌋⌋⟧ = c` makes them isomorphic where it matters); syntax
  is only kept where the small-step needs to inspect it (operator expressions/state).
- The paper's "finite, downwards-closed partial order" becomes a well-founded relation
  (`WellFounded`) that steps decrease; this is what Lemma 2.3.1 actually needs.
- Tuples of ports `[C]` are modeled as `Fin n → C` (or `List`-indexed families).

## Extended bound hierarchy (Rust parity) — *historical table; the live form is `MonoSing`/`Growth`*

> The trajectory-marker layer described below (`Hydro/Bounds.lean`,
> `Hydro/KeyedSingletonTraj.lean`) was retired with the old architectures;
> its role is subsumed by the `Growth`/`MonoSing` wire types
> (`Hydro/Growth.lean`, `Hydro/MonoSing.lean`): program values carry the
> marker types in their stage signatures, and invariants like Paxos's
> max-ballot monotonicity are type-derived — never manual run inductions.
> The table is kept as the Rust-marker mapping record.

The dissertation formalizes only `Bounded`/`Unbounded`; the Hydro *implementation* has a
richer marker hierarchy that is load-bearing for invariant-by-typing. Mapping table
(Rust marker ↔ retired Lean type ↔ generic theorem proven once per marker):

| Rust marker | Lean | generic theorems |
|---|---|---|
| `Monotonic` (singleton.rs:72) | `MonotonicSingleton r α` (bundled `Traj` + preorder + `MonoAlong r`) | `observe_mono` (cross-tick, any schedule/atomic pinning — the type-level `MaxBallotMono`), `observe_le_settled`, `threshold_stable`, `reads_chain`, `observeAtomic_mono` |
| `InitNone` (optional.rs) | `InitNoneOptional α β` = `MonotonicSingleton initNoneLe α` | `once_some_stays_some`; constructors `first`, `reduce` (Rust `AggregatedOptional`: unbounded aggregation ⇒ InitNone) |
| — (`Stream::max`) | `Stream.max le hrefl htrans` | `Monotonic` in `optionLe le` AND `InitNone` via `weaken` |
| `MonotonicValue` (keyed_singleton.rs:117) | `KTraj.MonotonicValue r t` | per-key `observe_mono`, `observe_le_settled`, `threshold_stable`; constructors `keyedFold`/`keyedReduce` with the `monotone = manual_proof!` promise as a hypothesis |
| `MonotonicKeys` (keyed_singleton.rs:134) | `KTraj.MonotonicKeys t` | `observe_stays` (once present, stays); weakening `MonotonicValue.toMonotonicKeys` (Rust `EraseMonotonic`) |
| `BoundedValue` (keyed_singleton.rs:100) | `KTraj.BoundedValue t` | `observe_eq_settled` (first observation is the settled per-key value) |

## Targets and status

1. ✅ Core formalisms + proofs (Flo ch. 2, Gyatso ch. 3) — zero sorry, axiom-audited.
2. ✅ The decisions-as-inputs proof surface (docs/10): `TStream`/`ForwardRef`
   decision semantics, `Growth`/`MonoSing` wire types, nondet-as-data.
3. ✅ (a) `collect_quorum` port + unbounded correctness (all inputs, all
   batch decisions, both branches) + the typed-stage face
   (`collect_quorum_spec`); the quorum stage is shared by 2PC and Paxos.
4. ✅ (b) medium programs: `two_pc` over the shared quorum stage (master
   theorem `twoPC_committed_eq` + unanimity/no-dup/determinism corollaries),
   `index_payloads`, `join_responses` (`sequence_payload` lives in the Paxos
   port). Open-membership negatives retired to FINDINGS (§C row + D20).
5. ✅ (c) Paxos: 1:1 port, the headline `commit_agreement`
   (`Programs/Paxos/Safety.lean`, standard axioms only), and the executable
   falsifications of two candidate Rust bugs (FINDINGS.md B1/B2,
   `lake exe falsify`).
