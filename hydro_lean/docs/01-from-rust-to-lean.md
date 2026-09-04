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
  instantiated at `Values` (to state and prove things — and to run: the denotation is
  the executable, `MemberMap`), at `SchedSem` (the concurrent machine).
- `L` is the **type of locations** of the deployment — the set of clusters, with `mem ℓ`
  the size of cluster `ℓ`. It is a parameter of the whole signature because one
  `HydroSem` instance is one deployment (Rust fixes its `Cluster<…>` types per program
  the same way). It is *not* a location: each stream carries its own `ℓ : L` in its type
  — `Stream : L → (α : Type) → … → Type` (`Hydro/Sem.lean:160`). `collect_quorum` is
  location-generic (`(ℓ : L)`, like Rust's `L: Location<'a>`); `paxos_core` takes
  `(prop acc : L)` and types its wires `H.Stream prop …` / `H.Atomic acc …`. Data
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
| `use::batch(responses, nondet!(…))` | `(input new_inputs := H.batch responses dec)` — the `nondet!` becomes the `dec` argument; `BatchDec n α ord ret` is one family across the grades (multiset increments at `NoOrder × ExactlyOnce`, sets from the pool at `NoOrder × AtLeastOnce`, slice sizes at `TotalOrder × ExactlyOnce`, realized batches at `TotalOrder × AtLeastOnce` — D91) |
| `source_interval_delayed(delay, period)` (`Stream<(), _, _, TotalOrder, AtLeastOnce>`, no `nondet!`) | `H.source_interval delay period sched.interval` — a deterministic stream (its value is the one class `[()]`), its fire timing an `IntervalDec` in the module's `…Sched` bundle, like a transport cursor (D91) |
| `interval.batch(&tick, nondet!(…)).first()` on an interval stream | `H.firstTick (H.batch interval dec.interval)` outside a `sliced!` (the tick-level `Stream::first`), or `bfirst` inside one (D91) |
| `let mut not_all = use::state_null::<Stream<…, Bounded, Order>>()` | `(state not_all : H.BoundedStream … .noOrder .exactlyOnce)` — a persisted-stream register, seeded empty |
| `not_all.chain(new_inputs)` | `H.bchain not_all new_inputs` — the **in-tick** operators carry a `b` prefix and work on `BoundedStream` values (one tick's content) |
| `.into_keyed().fold(…, commutative = manual_proof!(…))` | `H.bkfold cqCount (0, 0) (fun s x y => cqCount_comm s x y) (H.binto_keyed …)` — the `manual_proof!` is a real proof argument; the closure is value-only, as in Rust |
| `count_per_key.entries().filter_map(…)` / `.filter(…).keys()` | `H.bfilterMap (H.bksEntries count_per_key) …` / `H.bksKeys (H.bksFilter count_per_key …)` — the keyed singleton's API |
| `x.across_ticks(\|s\| s.into_keyed().reduce_watermark(wm, …))` | `H.reduce_watermark (H.tinto_keyed x) wm f ok` — `all_ticks_atomic`/`batched_atomic` are identities here, so the thunk's operators are invoked directly on the atomic wire; the Rust quote keeps the `across_ticks` text |
| `.into_keyed().assume_ordering::<TotalOrder>(nondet!(…)).fold(…)` on a tick's batch (paxos.rs:617, the order-dependent recommit fold) | `H.tassume_ordering p_entries dec.order` on the tick-located pair stream, then `H.bkfold rcStep … (H.binto_keyed ents)` in the tick body with NO commutativity obligation — the `nondet!` is a `TickOrderSelDec` (one order per tick), the proof is over every order (D71) |
| `commutative = manual_proof!(/** … TODO: not if two entries with same ballot, need assume */)` (paxos.rs:862) | `H.reduce_watermark … LogTie logMerge logMerge_comm` + `ghost pre … : <the pool is tie-coherent> := …` before the site — the combiner is commutative *under* the tie `Pre`, the content half is proven at the denotation from the module's `requires` (D71, R3) |
| `if max == min { … } else { … }` | `let branch := if max = min then … else …` — static `if` in tick bodies |
| `not_all = …; min_but_not_max = …` (the `mut` rebinds at the end of the slice) | `rebind (not_all := branch.1, min_but_not_max := branch.2.1)` |
| the slice's value `just_reached_quorum` | `yield (just_reached_quorum := branch.2.2)` — a stream emission (`yield_atomic`); singleton emissions use `emit`, keyed-stream emissions `kyield` (`new_virtual_client.into_stream().into_keyed()` in `bench_client`, D81) |
| `let mut start_times = use::state_null::<KeyedSingleton<u32, _, _, _>>();` (bench_client/mod.rs:108) | `tick (state start_times : H.BoundedKeyedSingleton Nat Nat)` — a keyed-singleton register, seeded empty (D81) |
| `let now = use::time(nondet!(/** … wall clock */));` / `.cross_singleton(now)` | `(input now := H.time dec.clock)` — a `ClockDec` (the readings are a timing decision) / `H.bkcrossSingleton … now` (D81) |
| `.batch(…).assume_ordering::<TotalOrder>(nondet!(/** one in flight per virtual client */)).into_keyed().reduce(\|curr, new\| *curr = new)` | `(input new_inputs := H.tassume_ordering (H.batch …) dec.inputsOrder)` then `H.bkreduce (fun _ new => new) … (H.binto_keyed new_inputs)` — the overwrite is not commutative, so the per-tick order is a decision, not a `manual_proof!` (ruling R-b, D81) |
| `x.across_ticks(\|s\| s.fold(…))` (kv_replica/mod.rs:60) | `H.tfold g init … x` — the fold's state retained across ticks (D81) |
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

The output tuple. `H.allTicks` lifts the per-tick emissions (`just_reached_quorum : H.AtomicStream …`)
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
