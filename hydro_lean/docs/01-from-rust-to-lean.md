# 01 — From Rust to Lean: `collect_quorum`

The smallest complete module: `hydro_std`'s `collect_quorum`
(`hydro_std/src/quorum.rs:89–160`) and its Lean mirror
(`Hydro/Std/Quorum.lean:656–899`). One Rust function, one `hydro def`; the Rust body
is a `sliced!` block, the Lean body is a `tick` block.

## The Rust

```rust
// hydro_std/src/quorum.rs:89–99
pub fn collect_quorum<'a, L: Location<'a>, Order: Ordering, K: Clone + Eq + Hash, E: Clone>(
    responses: Stream<(K, Result<(), E>), L, Unbounded, Order>,
    min: usize,
    max: usize,
) -> (
    Stream<K, L, Unbounded, NoOrder>,
    Stream<(K, E), L, Unbounded, Order>,
) {
    let just_reached_quorum = sliced! {
        let new_inputs = use::batch(responses.clone(), nondet!(
```

Two things to notice before reading the Lean: the **types carry grades**
(`Unbounded, Order`; `NoOrder`), and there is exactly **one `nondet!`** — the batch
boundary. Inside the slice, `use::state_null` declares two cross-tick registers:

```rust
// quorum.rs:104–105
        let mut not_all = use::state_null::<Stream<_, _, Bounded, Order>>();
        let mut min_but_not_max = use::state_null::<Stream<K, _, Bounded, NoOrder>>();
```

## The Lean signature

```lean
-- Hydro/Std/Quorum.lean:662–668
hydro def collect_quorum (H : HydroSem L mem) (ℓ : L)
    (responses : H.Stream ℓ (K × Except E Unit) .noOrder .exactlyOnce)
    (min max : Nat)
    (dec : H.BatchDec (mem ℓ) (K × Except E Unit)) :
    (H.Stream ℓ K .noOrder .exactlyOnce
      × H.Stream ℓ (K × E) .noOrder .exactlyOnce)
  ensures out => CQEnsures ℓ min max responses dec out :=
```

- `H : HydroSem L mem` — the program is written against the **signature**
  (`../ARCHITECTURE.md` §2), not against one semantics. The same text is later
  instantiated at `Values` (to state and prove things), at `SchedSem` (the concurrent
  machine), at `Eager` (to run).
- `L` is the **type of locations** of the deployment — the set of clusters, with `mem ℓ`
  the size of cluster `ℓ`. It is a parameter of the whole signature because one
  `HydroSem` instance is one deployment (Rust fixes its `Cluster<…>` types per program
  the same way). It is *not* a location: each stream carries its own `ℓ : L` in its type
  — `Stream : L → (α : Type) → … → Type` (`Hydro/Sem.lean:160`). `collect_quorum` is
  location-generic (`(ℓ : L)`, like Rust's `L: Location<'a>`); `paxos_core` takes
  `(prop acc : L)` and types its wires `H.Stream prop …` / `H.Ticked acc …`. Data
  crosses clusters only through `H.broadcast_closed`/`H.demux` (each with a delivery
  cursor) and `H.values` — exactly the Rust network edges (chapter 06).
- `H.Stream ℓ α ord ret` — Rust's `Stream<α, L, Unbounded, Order>` at `ℓ`; the grades
  `.noOrder .exactlyOnce` are the Rust marker types. At `Values` this grade **chooses
  the carrier**: a `.noOrder` stream is a `Multiset`, so no proof can ever observe an
  order the Rust type does not promise (chapter 02).
- `dec : H.BatchDec …` — the one `nondet!`, as an **input**. Nondeterminism is data the
  program takes; "for all decisions" is how the theorems quantify over all batchings
  (`../DOCTRINE.md` P2).
- `ensures out => CQEnsures … out` — the contract face (chapter 03).

## The body: `sliced!` is `tick`

```lean
-- Hydro/Std/Quorum.lean:669–684
  -- let just_reached_quorum = sliced! {
  --   let new_inputs = use::batch(responses.clone(), nondet!(…));
  --   let mut not_all = use::state_null::<Stream<_, _, Bounded, Order>>();
  --   let mut min_but_not_max = use::state_null::<Stream<K, _, Bounded, NoOrder>>();
  tick (state not_all : H.BoundedStream (K × Except E Unit) .noOrder .exactlyOnce)
      (state min_but_not_max : H.BoundedStream K .noOrder .exactlyOnce)
      (input new_inputs := H.batch responses dec)
      -- the loop invariant: the register discipline against the
      -- responses consumed so far
      (invariant ((just_reached_quorum : List (Multiset K))
          (not_all : Multiset (K × Except E Unit)) (min_but_not_max : Multiset K)
          (new_inputs : Trace (Multiset (K × Except E Unit)))) =>
        CQRegInv min max not_all min_but_not_max
          ((new_inputs.take just_reached_quorum.length).sum)) :=
      -- let current_responses = not_all.chain(new_inputs);
      let current_responses := H.bchain not_all new_inputs
```

The Rust lines are quoted as comments directly above their Lean lines — the house
rule for every program (`../DOCTRINE.md` P1). Mapping:

| Rust | Lean |
|---|---|
| `sliced! { … }` | `tick … := …` |
| `use::batch(responses, nondet!(…))` | `(input new_inputs := H.batch responses dec)` — the `nondet!` becomes the `dec` argument |
| `let mut not_all = use::state_null::<Stream<…, Bounded, Order>>()` | `(state not_all : H.BoundedStream … .noOrder .exactlyOnce)` — a persisted-stream register, seeded empty |
| `not_all.chain(new_inputs)` | `H.bchain not_all new_inputs` — the **in-tick** operators carry a `b` prefix and work on `BoundedStream` values (one tick's content) |
| `.into_keyed().fold(…, commutative = manual_proof!(…))` | `H.bkeyedFold cqCount (0, 0) (fun s x y => cqCount_comm s x y) …` — the `manual_proof!` is a real proof argument |
| `if max == min { … } else { … }` | `let branch := if max = min then … else …` — static `if` in tick bodies |
| `not_all = …; min_but_not_max = …` (the `mut` rebinds at the end of the slice) | `rebind (not_all := branch.1, min_but_not_max := branch.2.1)` |
| the slice's value `just_reached_quorum` | `yield (just_reached_quorum := branch.2.2)` — a stream emission (`yield_atomic`); singleton emissions use `emit` |
| — | `(invariant (…) => CQRegInv …)` + `prove init := …, tick := …` — the loop invariant and its two obligations (chapter 04); this is the only text with no Rust counterpart, and it is spec, not program |

Reads of a `state` register inside the body are the **previous tick's** value — the
construct owns the one-tick lag, exactly as Rust's `use::state` does
(`FINDINGS.md` D58/D59). You cannot forget a `defer_tick` you never write.

The rest of the body (`Quorum.lean:685–717`) is the Rust lines 106–147 one for one;
compare them yourself — that is the point.

## After the slice

```lean
-- Hydro/Std/Quorum.lean:861–866
  -- (
  --   just_reached_quorum.assert_has_consistency_of(manual_proof!(/** TODO */)),
  --   responses.filter_map(q!(move |(key, res)| match res { Ok(_) => None, Err(e) => Some((key, e)) })),
  -- )
  (H.allTicks just_reached_quorum,
    H.filterMap responses (fun _me => cqErrProj))
```

The output tuple. `H.allTicks` lifts the per-tick emissions (`just_reached_quorum : H.TickStream …`)
to the unbounded output stream — the lift `sliced!` performs implicitly when its value
leaves the block in Rust; the second component is the stream-level `filter_map`. Rust's `assert_has_consistency_of(manual_proof!(/** TODO */))` has no Lean
line because the Lean *proves* the consistency it asserts (`CQEnsures.emit_count`,
chapter 03).

## The census

```lean
-- Hydro/Std/Quorum.lean:899
#nondet_census collect_quorum (nondets := 1) (scheds := 0) (fuels := 0)
```

Build-enforced: the module's decision record carries exactly one content
nondeterminism (the batch), no adversary cursors (no network edge inside this function),
no knot fuel. It must equal the Rust `nondet!` tally. Every module ends with one
(`../GATE.md` §5).

## What you have not seen yet

Everything between the `yield` and the output tuple (`Quorum.lean:720–860`) is proof
text — `prove init/tick` for the invariant, `ghost have`s decoding one tick, and the
`prove` block discharging the contract's fields. Chapter 04 reads it. First, chapter 02:
what does this program *mean*?
