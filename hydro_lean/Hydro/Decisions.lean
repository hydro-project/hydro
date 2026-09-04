import Hydro.Grades

/-!
# Hydro · the named decision vocabulary

Every Rust `nondet!` site consumes decision data of a specific shape.
These aliases name the shapes after **what the decision means**, so a
signature reads as its dataflow contract rather than its encoding; each
op's doc names its Rust site. All are reducible `abbrev`s — pure
documentation, definitionally transparent.
-/

namespace Hydro

/-- The unfolding-depth decision of a `forward_ref` cycle (the Kleene
iteration fuel — Rust's cycle `nondet!` freedom is *when the loop's
knot stabilizes*). -/
abbrev UnfoldFuel : Type := Nat

/-- `.assume_ordering::<TotalOrder>(nondet!(…))` on live unordered
content: the per-member **selection** realizing a `NoOrder ×
ExactlyOnce` pool as a sequence (an illegal selection blocks). -/
abbrev OrderSelection (n : Nat) (α : Type) : Type := Fin n → List α

/-- `.assume_ordering::<TotalOrder>(nondet!(…))` on ONE TICK's unordered
content (D71): the per-member, per-tick **selection** realizing each
tick's `NoOrder × ExactlyOnce` batch as a sequence — the order the fold
inside the tick will see. A tick is realized only by a permutation of
its batch; the first tick whose selection is not blocks the trace
(`realizeTicks`). The coupling derives the selection from the machine's
arrival order at every tick (`CoupleSem.tassume_ordering`), so the
denotation's realizations are exactly the real concurrent executions'
orders. -/
abbrev TickOrderSelection (n : Nat) (α : Type) : Type := Fin n → List (List α)

/-- `.batch(&tick, nondet!(…))` on unordered exactly-once content:
per-member, per-tick **consumed multiset increments** (count-legal
against the pool; an illegal increment blocks — legality is
realizability). -/
abbrev BatchCuts (n : Nat) (α : Type) : Type := Fin n → List (Multiset α)

/-- `.batch(&tick, nondet!(…))` on ordered content: per-member,
per-tick **consumed slice sizes** (the batch is the next slice of the
sequence). -/
abbrev OrderedBatchCuts (n : Nat) : Type := Fin n → List Nat

/-- `.batch(&tick, nondet!(…))` on ordered at-least-once content (D91): per-member,
per-tick **realized batches** (lists — duplicates are the grade; a batch whose
destuttered concatenation leaves the pool's class blocks). -/
abbrev StutterBatchCuts (n : Nat) (α : Type) : Type := Fin n → List (List α)

/-- `.batch(&tick, nondet!(…))` on unordered at-least-once content (D91, user ruling: "it
should pick a set, and the elements remain in the pool"): per-member, per-tick **sets**
drawn from the pool's support — nothing is consumed, an element may be in any number of
batches, and a batch's multiplicities are not a thing (the class forgets them), so the
decision does not carry them either. A batch with an element the pool lacks blocks. -/
abbrev RetryBatchCuts (n : Nat) (α : Type) : Type := Fin n → List (Finset α)

/-- The denotation's `.batch` decision by the stream's grade: unordered exactly-once
batches are multiset increments, unordered at-least-once batches are sets drawn from the
pool, ordered exactly-once batches are slice sizes, ordered at-least-once batches are
the realized lists. -/
@[reducible] def BatchCutsOf (n : Nat) (α : Type) : StrOrd → Retries → Type
  | .noOrder, .exactlyOnce => BatchCuts n α
  | .noOrder, .atLeastOnce => RetryBatchCuts n α
  | .totalOrder, .exactlyOnce => OrderedBatchCuts n
  | .totalOrder, .atLeastOnce => StutterBatchCuts n α

/-- `.snapshot(&tick, nondet!(…))` of a folded singleton: per-member
**arrival cuts**, graded by the source order (`CutDec` — prefix counts
at `TotalOrder`, arrival-increment multisets at `NoOrder`). -/
abbrev SnapshotCuts (n : Nat) (α : Type) (ord : StrOrd) : Type :=
  Fin n → CutDec α ord

/-- `use::snapshot(bounded_singleton, nondet!(…))` (D81): per-member,
the number of ticks the runtime runs the slice. -/
abbrev TickCounts (n : Nat) : Type := Fin n → Nat

/-- `use::time(nondet!(…))` (D81): per-member, per-tick **wall-clock
readings** (`Tick::current_tick_instant`) — pure timing. The denotation's
form: the readings *are* the Atomic trace, one per tick (the list's
length is the slice's tick count). -/
abbrev ClockReadings (n : Nat) : Type := Fin n → List Nat

/-! ### Timing at the machine: total per-tick scripts (D84)

At `SchedSem` the pacing already decides how many ticks a member runs; a
timing decision there supplies only **the value read at each tick**. A
script is therefore a total function of the tick index — every tick has
its reading (Rust's `current_tick_instant` always returns; a `timeout`
always has a verdict) — and a running process is handed one entry per
activation (`Project`). The denotation's lists are *derived* from a
script and the pacing at the coupling horizon (`scriptDerive`,
`sampleDerive` in `Transfer.lean`), so `Values` keeps its trace-shaped
decisions and the proofs over them. -/

/-- A per-member, per-tick script: the value at tick index `k`. -/
abbrev TickScript (n : Nat) (β : Type) : Type := Fin n → Nat → β

/-- The first `k` entries of a script as a trace. -/
def scriptTrace {β : Type} (sc : Nat → β) (k : Nat) : List β := (List.range k).map sc

@[simp] theorem scriptTrace_zero {β : Type} (sc : Nat → β) : scriptTrace sc 0 = [] := rfl

theorem scriptTrace_succ {β : Type} (sc : Nat → β) (k : Nat) :
    scriptTrace sc (k + 1) = scriptTrace sc k ++ [sc k] := by
  simp only [scriptTrace, List.range_succ, List.map_append, List.map_cons, List.map_nil]

@[simp] theorem scriptTrace_length {β : Type} (sc : Nat → β) (k : Nat) :
    (scriptTrace sc k).length = k := by
  simp [scriptTrace]

theorem scriptTrace_prefix {β : Type} (sc : Nat → β) {k k' : Nat} (h : k ≤ k') :
    scriptTrace sc k <+: scriptTrace sc k' := by
  induction k' with
  | zero => cases Nat.le_zero.mp h; exact List.prefix_refl _
  | succ k' ih =>
    rcases Nat.lt_or_ge k (k' + 1) with hlt | hge
    · exact (ih (Nat.lt_succ_iff.mp hlt)).trans (by rw [scriptTrace_succ]; exact List.prefix_append _ _)
    · cases Nat.le_antisymm h hge; exact List.prefix_refl _

theorem scriptTrace_getElem? {β : Type} (sc : Nat → β) (k j : Nat) :
    (scriptTrace sc k)[j]? = if j < k then some (sc j) else none := by
  simp only [scriptTrace, List.getElem?_map]
  split <;> simp_all

end Hydro
