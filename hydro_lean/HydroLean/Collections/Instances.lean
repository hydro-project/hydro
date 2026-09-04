import HydroLean.Flo.Collection
import HydroLean.Collections.Multiset
import HydroLean.Collections.DupList

/-!
# Concrete collection languages (dissertation §2.6, §3.5)

(Metatheory substrate for Flo/Gyatso — the algebra/simp API here is kept
complete whether or not each lemma is currently consumed.)

`Coll` instances for the collection types used throughout the dissertation's
case studies and network semantics:

- `seqColl` — ordered sequences with a terminator (Flink case study, Fig 2.13):
  the carrier of `TotalOrder + ExactlyOnce` streams.
- `msetColl` — multisets with a terminator (`[T]unord`, Fig 3.10): the carrier
  of `NoOrder` streams, where network reordering is quotiented away.
- `dupColl` — adjacent-duplicate-collapsed sequences (`[T]dup`, Fig 3.8): the
  carrier of `AtLeastOnce` (retry) streams.
- `singletonColl` — overwrite singletons (Fig 3.11): the carrier of Hydro's
  asynchronously-updated `Singleton`/`Optional` live collections, where
  concatenation replaces the value ("the singleton may skip intermediate
  states", §4.3.3).
- `lvarColl` — lattice values (LVars, Fig 2.16): concatenation is lattice join.

Each carrier is paired with a `Bool` "fixed" flag; a delta whose flag is set
acts as the terminator `⊗`. Fixed values absorb all further deltas, which is
exactly the paper's `fixed` predicate.
-/

namespace HydroLean

universe u

/-! ## Ordered sequences with terminator (Fig 2.13) -/

/-- Ordered sequences with a termination flag: values `(payload, fixed?)`;
concatenating `(δ, true)` appends `δ` and terminates the stream (`⊗`). -/
def seqColl (α : Type u) : Coll.{u} where
  C := List α × Bool
  concat c δ := if c.2 then c else (c.1 ++ δ.1, δ.2)
  empty := ([], false)
  fix c := (c.1, true)
  concat_empty c := by cases c with | mk l b => cases b <;> simp
  fix_fixed c δ := by simp

@[simp] theorem seqColl_concat_unfixed {α : Type u} (l : List α) (δ : List α × Bool) :
    (seqColl α).concat (l, false) δ = (l ++ δ.1, δ.2) := rfl

/-- A sequence value is fixed iff its flag is set. -/
theorem seqColl_fixed_iff {α : Type u} (c : List α × Bool) :
    (seqColl α).Fixed c ↔ c.2 = true := by
  cases c with | mk l b =>
  cases b
  · constructor
    · intro h
      have := congrArg Prod.snd (h ([], true))
      simp [seqColl] at this
    · intro h; cases h
  · exact ⟨fun _ => rfl, fun _ δ => by simp [seqColl]⟩

/-! ## Multisets with terminator (`[T]unord`, Fig 3.10) -/

/-- Multisets with a termination flag: the collection language of unordered
exactly-once delivery. Concatenation is multiset union, which is commutative —
the order in which the network delivers chunks is unobservable. -/
def msetColl (α : Type u) : Coll.{u} where
  C := Multiset α × Bool
  concat c δ := if c.2 then c else (c.1 + δ.1, δ.2)
  empty := (Multiset.nil, false)
  fix c := (c.1, true)
  concat_empty c := by cases c with | mk m b => cases b <;> simp
  fix_fixed c δ := by simp

theorem msetColl_fixed_iff {α : Type u} (c : Multiset α × Bool) :
    (msetColl α).Fixed c ↔ c.2 = true := by
  cases c with | mk m b =>
  cases b
  · constructor
    · intro h
      have := congrArg Prod.snd (h (Multiset.nil, true))
      simp [msetColl] at this
    · intro h; cases h
  · exact ⟨fun _ => rfl, fun _ δ => by simp [msetColl]⟩

/-! ## Dup-collapsed sequences (`[T]dup`, Fig 3.8) -/

/-- Adjacent-duplicate-collapsed sequences with a termination flag: the
collection language of ordered at-least-once delivery (retries). -/
def dupColl (α : Type u) [DecidableEq α] : Coll.{u} where
  C := DupList α × Bool
  concat c δ := if c.2 then c else (c.1.concat δ.1, δ.2)
  empty := (DupList.nil, false)
  fix c := (c.1, true)
  concat_empty c := by cases c with | mk d b => cases b <;> simp
  fix_fixed c δ := by simp

/-! ## Overwrite singletons (Fig 3.11, §4.3.3) -/

/-- Asynchronously-updated single values: a delta of `some x` overwrites the
current value (`s ++ x = x`, the Singleton collection of Fig 3.11); a delta of
`none` leaves it unchanged. The carrier of Hydro's `Singleton`/`Optional`. -/
def singletonColl (α : Type u) : Coll.{u} where
  C := Option α × Bool
  concat c δ :=
    if c.2 then c
    else (match δ.1 with | none => c.1 | some x => some x, δ.2)
  empty := (none, false)
  fix c := (c.1, true)
  concat_empty c := by cases c with | mk v b => cases b <;> simp
  fix_fixed c δ := by simp

/-! ## LVars (Fig 2.16) -/

/-- A join-semilattice with bottom, defined locally (no Mathlib): the algebraic
structure of an LVar (§2.6.2). -/
structure JoinSemilattice (L : Type u) : Type u where
  join : L → L → L
  bot : L
  join_assoc : ∀ a b c, join (join a b) c = join a (join b c)
  join_comm : ∀ a b, join a b = join b a
  join_idem : ∀ a, join a a = a
  join_bot : ∀ a, join a bot = a

/-- LVars: lattice values where concatenation is join (Fig 2.16). Grows
monotonically; associativity/commutativity/idempotence of join make arrival
order and duplication unobservable — the CRDT-style collection. -/
def lvarColl {L : Type u} (S : JoinSemilattice L) : Coll.{u} where
  C := L × Bool
  concat c δ := if c.2 then c else (S.join c.1 δ.1, δ.2)
  empty := (S.bot, false)
  fix c := (c.1, true)
  concat_empty c := by cases c with | mk v b => cases b <;> simp [S.join_bot]
  fix_fixed c δ := by simp

end HydroLean
