# Simulation Ground Truth and the `Holds` Discipline

> **Status: historical.** This document describes the pre-decisions-as-inputs
> layers (multiset carriers, sealed components, `Holds` simulation runs),
> which were deleted as unused after the (a)/(b) deliverables were migrated
> to the decisions-as-inputs surface (docs/10; typed M-form stages with
> colocated verified faces). Names below refer to deleted code — recover
> from jj history. Kept as the record of the design and of why it was
> superseded.

The denotational layer (docs 03–04) is where proofs are cheap. This layer is where
properties are **stated** — against a Lean mirror of the Rust deterministic
simulator, so that (i) no theorem can silently under-quantify nondeterminism, and
(ii) the same specs are checkable on concrete schedules (`#guard` = a Rust sim test)
and provable universally (the unbounded generalization).

## The simulator correspondence

`Hydro/Sim.lean` (linear chains, the pedagogical layer) and `Hydro/SimGraph.lean`
(general graphs: fan-in, fan-out, feedback cycles) mirror
`hydro_lang/src/sim/` decision-for-decision:

| Rust simulator | Lean |
|---|---|
| scheduler picks one unit per step (`sim/compiled.rs`) | `Event`: `inject` / `fire site want` / `observe hook choice`; `Schedule := List Event`; `simRun` is a total pure fold (deterministic given the schedule, like the sim given a seed) |
| a tick's co-located `TickInputHook`s resolve **jointly** (`sim/runtime/tick_input.rs`) | `fire` resolves one cut per input port of the site, together |
| **ports** | `GraphSys.ports i` = the number of input streams of slice `i` — one per `use::batch`/`use::atomic` site; *not* network endpoints |
| `sim_input()` senders / `sim_output()` receivers | `inject` events / reading the designated output stream off the `Trace` |
| top-level `ObservationHook`s release **one element at a time** "in order to handle possible feedback cycles" (`sim/runtime/observation.rs`) | `observe` events release exactly one element; the `[1, map 1, 2, map 2, …]` interleavings are expressible (`SimGraphDemos.CycleDemo`) |
| **no scheduler decisions on network delivery** — everything eagerly flows through the top-level graph | there are **no delivery events and no channel buffers**: every edge (network edges included) is a pure prefix-monotone function of production-so-far (`GraphSys.input`); latency ≡ the consumer's cut |
| ticks = slice clocks; a location may host several, never forced into rounds | sites are slice clocks; atomic-linked slices share a site; the same site may fire repeatedly while others stay silent (crash-stop = never scheduled) |

Feedback cycles need no machinery: a fire at step k reads histories through step
k−1, and histories only grow, so causality is by construction.

## Cheat-proofing: the `Holds` family

```lean
def HoldsAtQuiescence (input : List M) (spec : List M → List M → Prop) : Prop :=
  ∀ sched : Schedule M,
    (sys.simRun sched).injected = input →
    sys.Quiescent (sys.simRun sched) = true →
    spec input (sys.out (sys.simRun sched))
-- plus HoldsAlways, the every-prefix variant for monotone/prefix specs
```

(both on `Sys` in `Sim.lean` and `GraphSys` in `SimGraph.lean`). Program-correctness
statements **must** go through this family: the ∀ over the complete schedule space
lives structurally inside the definition, so narrowing the explored nondeterminism
requires visibly changing this one definition — a reviewable trust boundary, and a
guard against future proofs quietly skipping schedules. `simRun` is total
(illegal choices truncate), so *every* `Schedule` value denotes a legal execution —
there are no side conditions to forget.

## The bridges: from `Holds` to denotations

**Chain layer** (`Hydro/Adequacy.lean`): `quiescent_decomp` — every quiescing run
factors site-by-site (`Chain.RunsTo`) into per-site consumed batchings + the pure
stream equations; `Sys.holdsAtQuiescence_of_batching` turns any ∀-batching component
theorem into a `Holds` fact in a few lines; `quiescent_out_eq` /
`holdsAtQuiescence_of_denote` — **eventual determinism for fully sealed
multi-location systems** (network edges introduce no variance: that is Gyatso's
point, mechanized); `Sys.holdsAtQuiescence_iff_batching` — the two-way transport
(via `Sys.single_realize`: a canonical schedule realizes any batching).

**Graph layer** (`Hydro/SimGraph.lean` + `SimGraphProjection.lean`):

- `quiescent_settles` / `Settles.site_spec` — the evaluation-order-free
  decomposition for graphs *with cycles*: per-site "consumed everything" +
  history/state equations, into which any component theorem plugs.
- **Mid-run site projection** — `siteState_eq_run`, `siteHist_eq_allTicks`,
  `midrun_flatten_eq`, `midrun_consumed_prefix`: at any prefix of any run, a site's
  state and history literally equal the standalone `TickLoop` run on its consumed
  batches. Consequences: `site_spec_at` / `site_spec_consumed_at` (any ∀-batching
  theorem or `HComponent.abs` holds of a site mid-run) and
  `siteHist_eq_of_witness_at` (sealed sites collapse to pure functions of their
  availability).
- Run-level monotonicity — `RunLE`, `simRun_le`, `simRun_take_le`: consumed inputs
  and histories only grow (the operational lift of Thm 3.4.2 / the `Bounds`
  markers).
- **`cycle_invariant`** — the feedback-cycle rule: per-site contracts on
  (state, history), each preserved by one *local* step obligation under
  `ProducedUnder` (inputs read only contract-satisfying histories-so-far), hold at
  every prefix of every run. `Reaches`/`reaches_step`/`reaches_elim` let a
  model-verified site discharge its slot by its own ∀-batching theorem. The
  `SimGraphProjectionDemos.LogAck` demo is literally the Paxos "acked ⇒ logged and
  persists" shape.

Honest generality boundary: sealed-system output *uniqueness* is proven for acyclic
systems (`Ranked`, `quiescent_out_unique` in `SimGraph.lean`); for cyclic systems,
runs settle and witnessed sites collapse (which is what program proofs use via
`Settles`), but global uniqueness of fixed-points is not claimed.

## How a composed proof flows

The template (executed in `Programs/CollectQuorumSim.lean`, whose module doc is a
tutorial): **state** at the sim level, **prove** at the denotational level,
**bridge** once.

```lean
theorem collectQuorumSim_holds ... :
    (quorumSys κ E min max).HoldsAtQuiescence input (fun inp out => QuorumSpec ...)
```

is proven in a few lines from the *existing* `collectQuorum_correct` via
`holdsAtQuiescence_of_batching` — zero schedule reasoning at the program level; six
`#guard`s replay the Rust unit tests (including an order-varying schedule) against
the same spec.

For a multi-site system like Paxos, a fact such as "commit ⇒ f+1 acceptors logged"
flows as a chain of alternating **component-theorem instantiations** and **pure
stream equations**:

```
commit emitted at the p2b-quorum site
  —site_spec_at + cqRun_emit_sound→   ≥ f+1 Ok p2bs among that site's consumed input
  —consumed ⊑ avail = demux(acceptor histories)→   those p2bs are acceptor emissions
  —site_spec_at(acceptor) + acceptor model→   each emission is a pure function of that
                                              acceptor's consumed p2as, ack post-log-write
  —marker (`MonotonicValue`) / cycle_invariant→   the logged entry persists at ballot ≥ b
```

Cross-clock reads use retiming (`seqAsync`, doc 04) instead of hand inductions;
snapshot reads use `SnapshotEdge`/`SharedObs` contracts. The only manual content
left is the protocol-level argument (e.g. quorum intersection) and the contract
handoffs — everything schedule-shaped is generic.
