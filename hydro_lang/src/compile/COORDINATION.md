# Coordination Criterion Analysis

Static analysis for the Hydro compiler that determines whether a distributed program requires coordination, based on [The Coordination Criterion](https://arxiv.org/abs/2602.09435) (Hellerstein, 2026) and the Flo/Gyatso semantics from [Laddad's dissertation](https://www2.eecs.berkeley.edu/Pubs/TechRpts/2025/EECS-2025-85.html).

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
  - `Fold`/`FoldKeyed` with `commutative + idempotent`: lattice join — value only grows
  - `Scan` on `TotalOrder` input: deterministic stateful transform — preserves Prefix and SetInclusion
  - Bounded input to any aggregation: complete data, deterministic result

- **Preserved**: the operator transparently maintains the order. Walk continues to inputs.
  - `Map`, `Filter`, `FilterMap`, `FlatMap`, `Inspect`: element-wise transforms preserve SetInclusion and Prefix
  - `Chain` / union: preserves SetInclusion (both branches must prove it)
  - `Join`, `CrossProduct`: preserves SetInclusion (both inputs must prove it)
  - `Network`: preserves SetInclusion (elements arrive, may reorder)
  - `DeferTick`: preserves SetInclusion and Lattice

- **Broken**: the operator violates the order. Proof fails here.
  - `Fold`/`FoldKeyed` without commutativity+idempotency proof on unbounded input
  - `Difference` / `AntiJoin` with unbounded negative input
  - `Network` under Prefix goal (reordering breaks prefix)
  - Element-wise transforms under Lattice goal (closure not proven order-preserving)

Special handling:
- **`CrossSingleton`**: the stream side needs SetInclusion; the singleton side needs Lattice (unless bounded, in which case it's stable)
- **`CycleSource`**: inherits the proof result from its matching `CycleSink` (two-pass analysis for inter-cycle dependencies)

The analysis produces a trace for each sink showing every operator visited, whether it preserved/discharged/broke the proof, and the source span pointing to user code.

## IR enrichment

The analysis depends on proof annotations surviving into the IR:

- `Fold`, `FoldKeyed`: `is_commutative: bool`, `is_idempotent: bool`
- `Reduce`, `ReduceKeyed`, `ReduceKeyedWatermark`: `is_commutative: bool`, `is_idempotent: bool`

These are captured at IR construction time from the `commutative = manual_proof!(...)` and `idempotent = manual_proof!(...)` annotations on fold/reduce closures, via the `IsProved` trait in `properties/mod.rs`.

## Unfinished goals

**User-defined refinement orders.** The three built-in orders (Prefix, SetInclusion, Lattice) cover common cases, but the Coordination Criterion is parameterized over *any* partial order on outputs. A user should be able to specify a custom `PartialOrd` and have the analysis check monotonicity under that order. The `goal_overrides` API provides the hook; the missing piece is a `UserDefined` variant of `OrderGoal` that carries the order specification and rules for how operators interact with it.

**Closure analysis.** Element-wise transforms (`Map`, `FilterMap`, etc.) are conservatively treated as breaking Lattice order because the analysis cannot inspect closures. A `monotone = manual_proof!(...)` annotation on closures would let users assert that their closure preserves the lattice order, similar to how `commutative` works for folds.

**IDE integration.** The analysis captures `proc_macro2::Span` from IR nodes and can generate `dfir_lang::Diagnostic` entries. The `coordination_diagnostics_tokens()` method on `BuiltFlow` produces `#[deprecated]`-based warning tokens. These are not yet wired into the stageleft/trybuild code generation pipeline, so they don't appear in rust-analyzer automatically. Integration requires injecting the tokens into the generated code during compilation.

**Viz overlay.** The v1 analysis had a viz integration that colored non-monotone edges red in Mermaid/DOT graphs. This was removed during the v2 rewrite. Re-adding it with the v2 report structure (which tracks sinks and traces rather than individual edges) is straightforward but not yet done.

## Potential weaknesses

**Commutativity proofs are trusted, not verified.** The `manual_proof!` macro accepts a doc comment as justification but performs no actual verification. A user who incorrectly claims commutativity (e.g., on a non-commutative fold) will get a false "coordination-free" result. This is by design — the same trust model as Rust's `unsafe` — but it means the analysis is only as sound as the user's proofs.

**Gyatso's trivial monotonicity.** As noted by Laddad, Flo/Gyatso semantics guarantee that *every* program is monotone under the trivial order induced by each collection type's concatenation operator. Our analysis is meaningful only because it checks monotonicity under *non-trivial* orders (set inclusion, prefix, lattice). If the default order inference picks the wrong order, the analysis may give vacuously true results. The `goal_overrides` API mitigates this by letting users specify the order they care about.

**Network ordering assumptions.** The analysis treats `Network` as preserving SetInclusion but breaking Prefix. In practice, some network transports (e.g., TCP with a single connection) preserve ordering, but the analysis conservatively assumes reordering. This may produce false negatives for programs that rely on transport-level ordering guarantees.

**Cycle analysis is not a full fixpoint.** Inter-cycle dependencies are handled by running the cycle analysis twice. This handles one level of nesting (cycle A depends on cycle B) but not deeper chains. A true fixpoint iteration would be more robust, though deeper cycle nesting is rare in practice.

**Bounded vs. unbounded is coarse.** The analysis uses Hydro's `Bounded`/`Unbounded` distinction to determine whether aggregation results are stable. This is a structural property (inside a tick vs. across ticks) rather than a semantic one. A fold that happens to be inside a tick is treated as bounded even if the tick processes unbounded data over time. This is correct for within-tick stability but may miss cross-tick non-monotonicity in some patterns.
