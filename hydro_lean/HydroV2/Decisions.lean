import HydroV2.Grades

/-!
# HydroV2 · the named decision vocabulary

Every Rust `nondet!` site consumes decision data of a specific shape.
These aliases name the shapes after **what the decision means**, so a
signature reads as its dataflow contract rather than its encoding; each
op's doc names its Rust site. All are reducible `abbrev`s — pure
documentation, definitionally transparent.
-/

namespace HydroV2

/-- The unfolding-depth decision of a `forward_ref` cycle (the Kleene
iteration fuel — Rust's cycle `nondet!` freedom is *when the loop's
knot stabilizes*). -/
abbrev UnfoldFuel : Type := Nat

/-- `.assume_ordering::<TotalOrder>(nondet!(…))` on live unordered
content: the per-member **selection** realizing a `NoOrder ×
ExactlyOnce` pool as a sequence (an illegal selection blocks). -/
abbrev OrderSelection (n : Nat) (α : Type) : Type := Fin n → List α

/-- `.assume_ordering` inside the tick: per-member, per-tick sequence
realizations of unordered batches. -/
abbrev BatchOrderSelection (n : Nat) (α : Type) : Type :=
  Fin n → List (List α)

/-- `.batch(&tick, nondet!(…))` on unordered exactly-once content:
per-member, per-tick **consumed multiset increments** (count-legal
against the pool; an illegal increment blocks — legality is
realizability). -/
abbrev BatchCuts (n : Nat) (α : Type) : Type := Fin n → List (Multiset α)

/-- `.batch(&tick, nondet!(…))` on ordered content: per-member,
per-tick **consumed slice sizes** (the batch is the next slice of the
sequence). -/
abbrev OrderedBatchCuts (n : Nat) : Type := Fin n → List Nat

/-- `.snapshot(&tick, nondet!(…))` of a folded singleton: per-member
**arrival cuts**, graded by the source order (`CutDec` — prefix counts
at `TotalOrder`, arrival-increment multisets at `NoOrder`). -/
abbrev SnapshotCuts (n : Nat) (α : Type) (ord : StrOrd) : Type :=
  Fin n → CutDec α ord

/-- `.latest().sample_every(q!(dur), nondet!(…))`: per-member
decision-chosen **sample tick times** (sampling is where `AtLeastOnce`
is born — an unchanged latest sampled twice is a consecutive
stutter). -/
abbrev SampleTimes (n : Nat) : Type := Fin n → List Nat

/-- `.timeout(q!(dur), nondet!(…))` read at ticks: per-member, per-tick
**expiry verdicts** — pure timing (arbitrarily delayed messages can
expire any timer), so every verdict trace is realizable. -/
abbrev TimerVerdicts (n : Nat) : Type := Fin n → List Bool

/-- `source_interval_delayed(…).batch(&tick, …).first().is_some()`:
per-member, per-tick **pulse arrivals** — pure timing. -/
abbrev TimingPulses (n : Nat) : Type := Fin n → List Bool

end HydroV2
