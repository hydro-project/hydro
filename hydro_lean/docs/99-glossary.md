# Glossary: Rust ↔ Lean ↔ Paper

Lemma/theorem names are Lean-idiomatic; *surface-API* names mirror Rust
(DESIGN.md naming policy). Paper references are to the dissertation.

## Markers and carriers

| Rust | Lean | Paper |
|---|---|---|
| `TotalOrder` / `NoOrder` | `Hydro.StreamOrder.totalOrder` / `.noOrder`; carriers `Stream α := List α` / `Multiset α`; growth orders: prefix / `Cnt` count-domination (`Growth.lean` — the ordering markers *are* the ⊑ instances) | collection types, §2.3.2; `[T]unord`, Fig 3.10 |
| `ExactlyOnce` / `AtLeastOnce` | `Hydro.Retries`; `DupList` carrier, `Mem` membership growth order for at-least-once | `[T]dup`, Fig 3.8 |
| `Bounded` / `Unbounded` | `Boundedness.bounded/.unbounded` (`Flo/Collection.lean`) | §2.3.3, Fig 2.2 |
| `Monotonic` (SingletonBound) | `MonoSing` wire type (`Hydro/MonoSing.lean`); constructor `fold_monotonic` (the `monotone =` obligation paid at the definition site) | LVars §2.6.2 (threshold reasoning) |
| `InitNone` (OptionalBound), `MonotonicValue`/`MonotonicKeys` (KeyedSingletonBound) | retired trajectory layer (`Hydro/Bounds.lean`, `Hydro/KeyedSingletonTraj.lean` — jj history); marker mapping table kept in DESIGN.md | — (implementation hierarchy; richer than the paper's B/U) |
| weakening conversions (`Into`) | `StreamOrder.weakensTo`, `Retries.weakensTo`; carrier weakening `toCnt`/`toMem` (`Growth.lean`) | subtyping, Fig 2.2 / §4.3.2 |

## Operators and APIs

| Rust | Lean | Paper |
|---|---|---|
| `map`/`filter`/`filter_map`/`flat_map`/`chain`/`enumerate` | `Stream.map` etc. (`Hydro/Stream.lean`) | §4.3.1 |
| `fold` / `reduce` | `Stream.fold` / `Stream.reduce` | Fig 2.14, §4.3.3 |
| `fold_commutative` (with `commutative = manual_proof!`) | `Multiset.foldComm` requiring `Multiset.AccComm` (`Collections/Multiset.lean`, denotational); `Gyatso.foldCommutative : Op1` (operational, Fig 3.11) | §3.5.2, §4.3.4 |
| `into_keyed().fold` | `Stream.keyedFold` (`Hydro/Stream.lean`), `Multiset` keyed views (`Collections/Keyed.lean`) | §4.3.5 |
| `join` / `anti_join` / `filter_not_in` / `cross_singleton` | `Stream.join` (`Hydro/Join.lean`), `Stream.antiJoin`, `filterNotIn`, `crossSingleton` (tick form: `TStream.crossSingleton` — the atomic same-tick read) | §4.5.2 |
| `send`/`send_bincode` (o2o TCP) | an identity edge (Gyatso: every edge is a pure prefix-monotone function; operationally `Gyatso.networkO2o`) | Fig 3.7 (`network_o2o`) |
| `broadcast_closed` | type-level `Fin n` fan-out: every member sees the full stream (`TwoPC.lean` `c_votes`, Paxos P1a wiring) | one-to-many §3.5.3 |
| `broadcast` (dynamic membership) | retired with the old layer (jj history); re-formalization design in FINDINGS D20 | — |
| `demux` | destination-keyed pairs + member-indexed family projection (e.g. p1b replies keyed by `ballot.proposerId`, Paxos wiring) | `network_o2m`, Fig 3.12 |
| cluster fan-in (`.values()` receive) | `unionF` + `batchC` decisions (`TStream.lean`; the shuffle is decision data) | `network_m2o`, Fig 3.13 |
| `sliced!` block / tick | `TickLoop` (`Hydro/Tick.lean`); lifted to a wire stage via `.loop` (`Growth.lean`) | ticks §4.5.2; nested graphs §2.5 |
| `use::batch(s, nondet!)` | a decision input: demand counts (`batch`, TotalOrder) or the consumed batches themselves (`batchC`, NoOrder) | eager execution Def 2.3.1 (what makes it safe) |
| `all_ticks()` | `allTicks` / `allTicksOutput` | §4.5.2 |
| `Singleton` / `Optional` (unbounded) | `TSing` per-tick singleton wires; value-ascending form `MonoSing` (`Hydro/MonoSing.lean`) | §4.3.3 |
| `snapshot(tick, nondet!)` | snapshot decision inputs: `snapshotC` (NoOrder fold views) / `snapshotD` (AtLeastOnce membership views) in `TStream.lean` | §4.5.1 (unsafe APIs) |
| `latest_atomic` / `use::atomic` | same-tick `crossSingleton` against the tick's published wire (e.g. the Paxos `a_log` knot: replies read the log atomically, `alog_succ`) | §4.5.2 |
| `sample_every` | `sampleEvery` (`TStream.lean`; wire form `Growth.lean`) | Fig 4.15 |
| write-state-then-ack (`yield_atomic`) | write-before-ack `acceptor_p2` contract (`ap2t_ok_spec`), transported through the `a_log` knot (`alog_succ`, `PaxosCore.lean`) | §4.5.2 "atomic processing" |
| `across_ticks` | `scanSt` / `Stream.acrossTicksMax` (tick faces of the unbounded fold); monotone form `fold_monotonic` | §4.3.3 |

## Guards, choices, proofs

| Rust | Lean | Paper |
|---|---|---|
| `NonDet` parameter in a signature | an explicit decision parameter of the stage (docs/10) | unsafe operators §4.5.1 |
| `nondet!(/** locally resolved */)` | a decision-invariance theorem (e.g. `twoPC_deterministic`) | — |
| `nondet!(/** forwarded */)`, `take_hook`, tuple payloads | decision parameters threaded through the stage's signature (product decision spaces, e.g. `PaxosNondet`) | — |
| `nondet!(/** causal justification */)` | a **derived guarantee** when the comment claims one (e.g. leader-ballot stability → `le_ballot_stable`, paxos.rs:186–189, FINDINGS D21), else a **named proof input** of the verified face | — |
| `manual_proof!(/** ... */)` | a real hypothesis (`Multiset.AccComm`, `fold_monotonic`'s closure obligation, ...) | §4.3.4 |
| sim hooks (`BatchHook`, scripted decisions) | the quantified decision values themselves; `#guard` scripts = scripted runs | §4.5 |
| `assert_has_consistency_of(manual_proof!)` | discharged by decision-invariance faces (e.g. `collect_quorum_spec`) | — |

## Simulator vocabulary

The Lean mirror of the simulator's decision space is the decision inputs
themselves (docs/10); the retired `Sim`/`SimGraph` operational mirror lives in
jj history.

| Rust (`hydro_lang/src/sim/`) | Lean |
|---|---|
| scheduler step: run one tick or resolve one observation (`compiled.rs`) | a tick's realized cut — where the batch decisions place the boundary |
| `TickInputHook`s of one tick resolve jointly (`tick_input.rs`) | one tick's joint decision entry (blocking zips: a tick realizes only when all its inputs do) |
| `ObservationHook` one-element release (`observation.rs`) | snapshot decision views (`snapshotC`/`snapshotD`) |
| quiescence | complete-consumption decisions (`Consumes`, `TStream.lean`) |
| `sim_input()` / `sim_output()` | stage inputs / run views (`.f` at the boundary) |
| exhaustive/fuzz exploration | the ∀-decisions in each face (unbounded); `#guard`/`lake exe falsify` (bounded) |

## Metatheory (paper-first)

| Paper | Lean |
|---|---|
| collection language `L_C` (§2.3.2) | `Coll` (`Flo/Collection.lean`) |
| operator language `L_O`, `→_δ`, `⊢_O` (§2.3.4) | `Operator`, `Operator.step`, `Operator.Lawful` |
| Lemma 2.3.1 (operator stuck state) | `Operator.Lawful.sn` / `exists_stuck` |
| Def 2.3.1 (eager execution) | `Operator.Eager`; graphs: `Graph.EagerG` |
| Defs 2.3.2/2.3.3 (maximality/progress) | `Operator.OutputsMaximal` / `Operator.Progress` (with the `Inv` amendment, FINDINGS §A1) |
| graph grammar + semantics (Figs 2.4–2.6) | `Graph`, `Graph.Step`, `Graph.CStep`, `Graph.WT` |
| Lemmas 2.4.2/2.4.3/2.4.4 | `Graph.sn_of_leavesLawful`, `Graph.deterministic_and_eager` (+ `Graph.unique_stuck`), `Graph.progress_of_wt` |
| cluster upgrade (Figs 3.5/3.6) | `Gyatso.multiColl`, `Operator.clusterOp` (+ lawfulness) |
| `network_o2o` / `[T]unord` (Figs 3.7/3.10) | `Gyatso.networkO2o` / `networkO2oUnord` |
| Thm 3.4.1 (eventual determinism) | `Gyatso.eventual_determinism` (via `Graph.unique_stuck`); `outputs_below_settled` |
| Thm 3.4.2 (monotone outputs) | `Gyatso.monotone_outputs`; at the proof surface, the `Growth` (⊑) orders / `→ₘ` wire types |
