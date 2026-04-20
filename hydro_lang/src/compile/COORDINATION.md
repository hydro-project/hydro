# Coordination Criterion Analysis

Static analysis for the Hydro compiler that determines whether a distributed program requires coordination, based on [The Coordination Criterion](https://arxiv.org/abs/2602.09435) (Hellerstein, 2025 preprint) and the Flo/Gyatso semantics from [Laddad's dissertation](https://www2.eecs.berkeley.edu/Pubs/TechRpts/2025/EECS-2025-85.html).

## What is to be proved

A distributed program admits a coordination-free implementation if and only if its observable outputs are **future-monotone**: as inputs grow over time, outputs only grow (never contradict earlier observations) under some meaningful partial order.

The analysis proves monotonicity at each observable sink (e.g., `for_each`, `SendExternal`) under one of three built-in partial orders:

| Order | Applies to | Meaning |
|-------|-----------|---------|
| **Prefix** | `TotalOrder` streams | Output is a growing deterministic sequence; each observation is a prefix of all future observations |
| **Set inclusion** | `NoOrder` streams | Output elements only accumulate; new elements appear but none are retracted |
| **Lattice** | Singletons from commutative+idempotent aggregation | Output value only grows under a join-semilattice order |

The default order is inferred from the sink's collection kind. Users can override it via `check_coordination_with_goals()` — for example, requesting a SetInclusion proof for a TotalOrder stream (a weaker but still useful guarantee). In principle, the Coordination Criterion applies to *any* user-defined partial order on outputs, not just these three; see [Unfinished goals](#unfinished-goals) for the path toward supporting arbitrary `PartialOrd`-based orders.

## How it works

The analysis walks **backward** from each observable sink, carrying a proof goal (the partial order to prove). At each operator in the IR graph, one of three things happens:

- **Discharged**: the operator establishes monotonicity. Proof complete — don't look upstream.
  - `Source` / `ExternalInput`: data only arrives (base case)
  - `Fold`/`FoldKeyed` proven to be a lattice join (via `commutative + idempotent` annotations): value only grows
  - Bounded input to any aggregation: complete data, deterministic result

- **Preserved**: the operator transparently maintains the order. Walk continues to inputs.
  - `Map`, `Filter`, `FilterMap`, `FlatMap`, `Inspect`: element-wise transforms preserve SetInclusion and Prefix
  - `Scan` on `TotalOrder` input: deterministic stateful transform — preserves Prefix and SetInclusion
  - `Chain` / union: preserves SetInclusion (both branches must prove it)
  - `Join`, `CrossProduct`: preserves SetInclusion (both inputs must prove it)
  - `Network`: preserves SetInclusion (elements arrive, may reorder). Preserves Prefix when the transport guarantees ordering (e.g., TCP FailStop point-to-point), as reflected in the IR metadata.
  - `DeferTick`: preserves SetInclusion and Lattice

- **Broken**: the operator violates the order. Proof fails here.
  - `Fold`/`FoldKeyed` without commutativity+idempotency proof on unbounded input
  - `Difference` / `AntiJoin` with unbounded negative input
  - `Network` under Prefix goal (reordering breaks prefix)
  - Element-wise transforms under Lattice goal (closure not proven order-preserving)

Special handling:
- **`CrossSingleton`**: the stream side preserves SetInclusion and Prefix; the singleton side needs Lattice (unless bounded, in which case it's stable)
- **`CycleSource`**: inherits the proof result from its matching `CycleSink` (fixpoint analysis for inter-cycle dependencies)

The analysis produces a trace for each sink showing every operator visited, whether it preserved/discharged/broke the proof, and the source span pointing to user code.

## IR enrichment

The analysis depends on proof annotations surviving into the IR:

- `Fold`, `FoldKeyed`: `is_commutative: bool`, `is_idempotent: bool`
- `Reduce`, `ReduceKeyed`, `ReduceKeyedWatermark`: `is_commutative: bool`, `is_idempotent: bool`

These capture whether the aggregation has been proven to be a lattice join, via the `commutative = manual_proof!(...)` and `idempotent = manual_proof!(...)` annotations on fold/reduce closures. The `IsProved` trait in `properties/mod.rs` bridges the compile-time proof to the IR boolean.

## Consistency labels and propagation

Each passing sink receives a consistency label based on its proof goal and location:

| Label | Condition | Guarantee |
|-------|-----------|-----------|
| **SEQ** (Sequentially Consistent) | Prefix passes on Cluster | All replicas produce prefixes of the same deterministic sequence |
| **CONV** (Convergent) | SetInclusion or Lattice passes on Cluster | All replicas converge via commutative merge |
| **SELF** (Self-Consistent) | Proof passes, no cross-replica guarantee | Future-monotone per-replica, but replicas may differ |
| **INCON** (Inconsistent) | Proof fails | No guarantee |

Labels propagate forward along the channel DAG using five rules (Definition 6.2 of the consistency paper):

1. **Cluster with monotone upstream** (upstream ≥ SELF): Cluster re-establishes consistency via broadcast → use local label.
2. **Cluster with non-monotone upstream** (upstream = INCON): local proof's monotone-input assumption is violated → INCON.
3. **Deterministic Process**: inherits upstream, capped by local proof strength → min(local, upstream).
4. **Nondeterministic Process**: nondeterminism erases upstream provenance → SELF.
5. **Failing proof**: → INCON regardless of upstream.

Edge labels are computed by running the backward proof at each Network boundary (even for broadcast-to-Cluster edges), so upstream INCON is correctly detected and propagated.

## Unfinished goals

**User-defined refinement orders.** The three built-in orders (Prefix, SetInclusion, Lattice) cover common cases, but the Coordination Criterion is parameterized over *any* partial order on outputs. A user should be able to specify a custom `PartialOrd` and have the analysis check monotonicity under that order. The `goal_overrides` API provides the hook; the missing piece is a `UserDefined` variant of `OrderGoal` that carries the order specification and rules for how operators interact with it.

**Closure analysis.** Element-wise transforms (`Map`, `FilterMap`, etc.) are conservatively treated as breaking Lattice order because the analysis cannot inspect closures. A `monotone = manual_proof!(...)` annotation on closures would let users assert that their closure preserves the lattice order, similar to how `commutative` works for folds.

**IDE integration.** The analysis captures `proc_macro2::Span` from IR nodes and can generate `dfir_lang::Diagnostic` entries. The `coordination_diagnostics_tokens()` method on `BuiltFlow` produces `#[deprecated]`-based warning tokens. These are not yet wired into the stageleft/trybuild code generation pipeline, so they don't appear in rust-analyzer automatically. Integration requires injecting the tokens into the generated code during compilation.

**Viz overlay.** The v1 analysis had a viz integration that colored non-monotone edges red in Mermaid/DOT graphs. This was removed during the v2 rewrite. Re-adding it with the v2 report structure (which tracks sinks and traces rather than individual edges) is straightforward but not yet done.

## Potential weaknesses

**Lattice join proofs are trusted, not verified.** The `manual_proof!` macro accepts a doc comment as justification but performs no actual verification. A user who incorrectly claims their fold is a lattice join (via `commutative + idempotent` annotations) will get a false "coordination-free" result. This is by design — the same trust model as Rust's `unsafe` — but it means the analysis is only as sound as the user's proofs.

**Gyatso's trivial monotonicity.** As noted by Laddad, Flo/Gyatso semantics guarantee that *every* program is monotone under the trivial order induced by each collection type's concatenation operator. Our analysis is meaningful only because it checks monotonicity under *non-trivial* orders (set inclusion, prefix, lattice). If the default order inference picks the wrong order, the analysis may give vacuously true results. The `goal_overrides` API mitigates this by letting users specify the order they care about.

**Network ordering is transport-aware.** The analysis checks the `Network` node's IR metadata for the output ordering, which reflects the transport's guarantee (e.g., TCP FailStop preserves `TotalOrder` for point-to-point sends). Multi-sender scenarios (e.g., cluster-to-cluster demux) correctly produce `NoOrder` due to receiver-side interleaving.

**Cycle analysis uses fixpoint iteration.** Inter-cycle dependencies are handled by seeding all cycles with optimistic placeholders and iterating until convergence. This correctly handles arbitrary nesting depth (cycle A depends on cycle B depends on cycle C, etc.).

**Bounded vs. unbounded is structural.** The analysis uses Hydro's `Bounded`/`Unbounded` distinction to determine whether aggregation results are stable. `Bounded` means "within a tick" — the fold sees complete input and produces a deterministic result. Cross-tick accumulation is handled by `YieldConcat` (which preserves set inclusion) and `DeferTick` (which preserves set inclusion and lattice order). This structural approach is sound for Hydro's tick-based execution model but would need revisiting if Hydro added non-tick-based windowing.
