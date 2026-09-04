# 02 — What a program denotes

A `hydro def` is a function of `H : HydroSem L mem`. Instantiating `H := Values L mem`
(`Hydro/Values.lean:236`) gives the program's **denotation**: a mathematical value,
computed from its inputs and its decisions, with no schedule anywhere. Every contract
is stated at `Values` and every proof lives there.

## Carriers at `Values`

```lean
-- Hydro/Values.lean:300–
@[reducible] def Values (L : Type) (mem : L → Nat) : HydroSem L mem where
  Stream ℓ α _ ord ret := MemberMap (mem ℓ) (PoolCarrier α ord ret)
  TransportDec _ _ := Unit
  OrderSelDec n α := OrderSelection n α
  SnapDec n α ord := SnapshotCuts n α ord
  BatchDec n α := BatchCuts n α
  …
  FixDec := UnfoldFuel
  …
  Atomic ℓ σ := TickV (mem ℓ) σ
  -- one tick's bounded stream: the graded content quotient
  BoundedStream α _ ord ret := PoolCarrier α ord ret
  BoundedSingleton σ := σ
```

Read the first line as the whole idea: a cluster stream at location `ℓ` is **one pool
per member** — a `MemberMap (mem ℓ) C` is the function `Fin (mem ℓ) → C` in a structure
(read `m i`, built `.mk' fun i => …`; the structure is there for the compiled run, see
"Running it" below) — and the pool's *type* is chosen by
the grade (`Hydro/Grades.lean`): a `List` at `TotalOrder`/`ExactlyOnce`, a `Multiset` at
`NoOrder`/`ExactlyOnce`, a destuttered sequence or a support quotient at `AtLeastOnce`.
There is no order to observe in a `.noOrder` stream because there is no order in a
`Multiset` — "what a stream's marker types refuse to promise is unobservable by
construction" (`../ARCHITECTURE.md` §1).

A tick-located wire (`Atomic ℓ σ`) is a per-member **trace**, `MemberMap (mem ℓ) (Trace σ)`
with `Trace σ := List σ` (`Hydro/Trace.lean:43`) — the value at each tick the member
took, in order. One tick's content (`BoundedStream`) is the same graded pool; a
`BoundedSingleton σ` is just `σ`.

## Decisions are inputs

The `…Dec` field types name what Rust's `nondet!` sites decide. A `BatchDec` is
`BatchCuts n α := Fin n → List (Multiset α)` (`Hydro/Decisions.lean:29`): per member,
the sequence of per-tick batches. The `batch` operator at `Values` is a `rfl` reader:

```lean
-- Hydro/Values.lean:416–419
@[den] theorem values_batch {α : Type} [DecidableEq α]
    (s : (Values L mem).Stream ℓ α .noOrder .exactlyOnce)
    (d : (Values L mem).BatchDec (mem ℓ) α) (i : Fin (mem ℓ)) :
    (Values L mem).batch s d i = batchCuts (s i) 0 (d i) := rfl
```

and `batchCuts` (`Hydro/Trace.lean:892–898`) walks the decision, keeping each claimed
batch while it still fits inside the member's pool and **truncating the trace at the
first illegal cut** — a decision that claims messages the pool never contained simply
ends the run there. So *legality is realizability*: quantifying over all decisions
quantifies over all batchings that could happen, and nothing else. (`cqConsumed pool d
:= (batchCuts pool 0 d).sum` — `Trace.lean:1527` — is "what this member consumed under
decision `d`", the vocabulary `CQEnsures` is stated in.)

Transport cursors are `Unit` at `Values` (`TransportDec _ _ := Unit`): the denotation
does not know *when* a message arrived, only that the receiver's pool contains it.
Chapter 06 is where cursors become real.

## Reading a wire: `den`

Every `HydroSem` operator at `Values` is one `rfl` lemma tagged `@[den]`
(`Hydro/SimpAttr.lean:27`, `Hydro/Values.lean:403–…`). To read what a wire *is*, name
the `let`s it is built from and let the readers fire:

```lean
simp only [just_reached_quorum_step, den]      -- Quorum.lean:727, inside the tick obligation
simp only [views, folded, quorum_outs, den]    -- a chain of stream-level ops (D66)
```

The `_step` name is the `tick` block's body as a function, emitted by the construct
(chapter 04). After `simp only [den]` a goal about a wire is a goal about plain `List`/
`Multiset` terms — `List.map`, `Multiset.filter`, `batchCuts`, `mapPool` — and ordinary
Lean/Mathlib takes over. The house rule for what a proof cites, by what it meets: an
operator → `den`; a `tick` block → its readers; a module call → the callee's `.ensures`
face, never unfolded; a `fix` knot → its generated `stages` (`../ARCHITECTURE.md` §5).

## Running it: the denotation *is* the executable

A cluster-located value at `Values` is one cell per member: a `MemberMap (mem ℓ) C`
(`Hydro/MemberMap.lean`), which is the function `Fin (mem ℓ) → C` in a structure. Every
operator builds its output with `MemberMap.mk' fun i => …`; every read `m i` of a
constructor is `f i` by `rfl`, so proofs never see anything but the function. The
compiler sees something else: the `@[csimp]` equation `mk'_eq_mkImpl : @mk' = @mkImpl`
lets it replace `mk'` by `MemberMap.mkImpl`, which forces the cells into a `Vector`
once and serves reads from it — so a run is
call-by-value and shares every wire, and `lake exe paxos` runs its scenario in
milliseconds instead of the hours a bare-function carrier took (`FINDINGS.md` D14, D73).
What an executable prints is **provably** the `Values` run through exactly one lemma,
and the compiler is only *allowed* the substitution because of it:

```lean
-- Hydro/MemberMap.lean
@[csimp] theorem mk'_eq_mkImpl : @mk' = @mkImpl
```

(`csimp` is how core itself replaces `List.map` by `List.mapTR`: the attribute demands a
kernel-checked equation; there is no `implemented_by` in `Hydro/`.) Nothing per program
and nothing Paxos-specific: the executables are the `Values` programs, the one compiled
seam is a proven-equal constructor — and the proof is what authorizes the seam. This is why
`lake exe paxos` is a witness for the headline theorem rather than a separate program:
`README.md` §headlines. (Why the structure carries a second field `boxed : Unit`, and
why it must: `../ENGINE_NOTES.md` "Runtime".)

## What this buys

- A theorem quantified over `dec` is a theorem about every batching, every snapshot
  cut, every order selection the Rust `nondet!` could realize.
- A proof reads program wires as `List`/`Multiset` values after one `simp only [den]`.
- Nothing about scheduling, delivery or time exists at this layer — that is the
  machine's job (chapter 06), and the coupling corner is what lets the `Values` proofs
  speak about machine runs.
