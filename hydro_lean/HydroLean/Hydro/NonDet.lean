import HydroLean.Hydro.Markers

/-!
# Nondeterminism as data (Rust: `hydro_lang::nondet`, dissertation §4.5)

In Rust Hydro, every safe API is deterministic; operations that observe
nondeterminism (batch boundaries, snapshot timing, interleaving order,
sampling) take an explicit `NonDet` guard created by `nondet!(/** reason */)`,
and the deterministic-simulation tester takes control of exactly those sites
via *hooks* (`hydro_lang::sim::hooks`).

The Lean embedding makes the guard **carry the adversarial choice itself**:

- an unsafe operator like `batch` takes a `Batching α` — the concrete split of
  its input into per-tick batches;
- snapshot-style operators take a `SnapshotSchedule` — which prefix of the
  evolving value each tick observes;
- `NoOrder` fan-ins are consumed through `batchC` decisions (`TStream.lean`):
  the consumed batch IS the decision, arrival shuffle included.

**Proof discipline.** A Hydro program with `N` guard sites becomes a Lean
function with `N` extra parameters. An *unbounded correctness* theorem is then

```
theorem prog_correct (input : …) (c₁ : Batching …) … (cₙ : …)
    (h₁ : c₁.of input) … : P (prog input c₁ … cₙ)
```

— a universal quantification over `N` *structured* choices, never over global
interleavings of the whole system. The Flo/Gyatso metatheory
(`Flo/Theorems.lean`: unique stuck states; `Gyatso/Correctness.lean`) is what
licenses this: every scheduling decision *not* exposed through a guard is
provably unobservable, which is the same reduction the Rust simulator exploits
when it fuzzes only the hooks (§4.5.1).
-/

namespace HydroLean.Hydro

universe u

/-- An adversarial split of a stream prefix into per-tick batches
(Rust: the `NonDet` guard of `Stream::batch` / `use::batch` inside `sliced!`;
simulator hook: `BatchHook`). `b[i]` is the batch delivered to tick `i`; empty
batches are allowed (ticks may fire without new input). -/
def Batching (α : Type u) : Type u := List (List α)

namespace Batching

variable {α : Type u}

/-- `b.of input`: the batching materializes exactly `input` — concatenating
the batches yields the input stream. Theorems about tick programs quantify
over all `b` with `b.of input`, giving correctness *for every way the runtime
could have chopped the stream*. -/
def of (b : Batching α) (input : List α) : Prop := b.flatten = input

/-- The one-batch batching. -/
def whole (input : List α) : Batching α := [input]

@[simp] theorem whole_of (input : List α) : (whole input).of input := by
  simp [whole, of]

/-- The singleton-batches batching (one element per tick). -/
def singletons (input : List α) : Batching α := input.map ([·])

@[simp] theorem singletons_of (input : List α) : (singletons input).of input := by
  induction input with
  | nil => rfl
  | cons x xs ih => simpa [singletons, of] using ih

end Batching

/-- A snapshot schedule (Rust: the `NonDet` guard of `Singleton::snapshot` /
`tick_snapshot`, §4.5.2; simulator hook: `SnapshotHook`): at each tick, the
adversary decides *how much* of the watched input has been incorporated into
the snapshotted value. `cuts[i]` is the number of upstream elements folded
into the snapshot observed at tick `i`; monotonicity reflects that the
underlying singleton evolves by concatenation and is never rolled back
(Gyatso's monotone-outputs guarantee, Thm 3.4.2). -/
structure SnapshotSchedule where
  /-- Elements of the watched stream incorporated at each tick. -/
  cuts : List Nat
  /-- Snapshots advance monotonically. -/
  mono : cuts.Pairwise (· ≤ ·)

/-- Observe the snapshot at tick `i` of a stream folded by `f`: the fold of
the first `cuts[i]` elements. Consumers pair this with a batch of another
stream in the same tick — the essence of §4.5.2's *temporal co-incidence*. -/
def SnapshotSchedule.observe {α β : Type u} (s : SnapshotSchedule)
    (f : List α → β) (input : List α) (i : Nat) : β :=
  f (input.take (s.cuts.getD i input.length))

end HydroLean.Hydro
