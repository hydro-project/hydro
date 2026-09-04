/-!
# HydroV2 · value orders

The partial order a `Monotonic` bound is *about* (Rust
`Singleton<_, _, Monotonic>` — the `monotone = <proof>` obligation of a
fold names an order on the accumulator and promises inflation in it).
Self-contained: V2 carries its own foundation.
-/

namespace HydroV2

/-- A (pre)order on values — the vocabulary of the `Monotonic` bound. -/
structure ValueOrder (σ : Type _) where
  le : σ → σ → Prop
  le_refl : ∀ a, le a a
  le_trans : ∀ {a b c}, le a b → le b c → le a c

/-- `Nat` under `≤` (ballot numbers, slot counters). -/
def ValueOrder.nat : ValueOrder Nat :=
  ⟨(· ≤ ·), fun _ => Nat.le_refl _, Nat.le_trans⟩

/-- Inflationary steps fold upward. -/
theorem ValueOrder.le_foldl {σ ι : Type _} (vo : ValueOrder σ)
    {g : σ → ι → σ} (hinfl : ∀ s x, vo.le s (g s x)) :
    ∀ (l : List ι) (s : σ), vo.le s (l.foldl g s)
  | [], s => vo.le_refl s
  | x :: xs, s =>
    vo.le_trans (hinfl s x) (ValueOrder.le_foldl vo hinfl xs (g s x))

/-- Appending only moves an inflationary fold up. -/
theorem ValueOrder.foldl_append_le {σ ι : Type _} (vo : ValueOrder σ)
    {g : σ → ι → σ} (hinfl : ∀ s x, vo.le s (g s x))
    (l e : List ι) (s : σ) :
    vo.le (l.foldl g s) ((l ++ e).foldl g s) := by
  rw [List.foldl_append]
  exact vo.le_foldl hinfl e _

/-- Longer inflationary folds dominate shorter ones. -/
theorem ValueOrder.foldl_take_le {σ ι : Type _} (vo : ValueOrder σ)
    {g : σ → ι → σ} (hinfl : ∀ s x, vo.le s (g s x))
    {l : List ι} {t t' : Nat} (h : t ≤ t') (s : σ) :
    vo.le ((l.take t).foldl g s) ((l.take t').foldl g s) := by
  have hsplit : l.take t ++ (l.take t').drop t = l.take t' := by
    have h1 : (l.take t').take t = l.take t := by
      rw [List.take_take, Nat.min_eq_left h]
    rw [← h1, List.take_append_drop]
  rw [← hsplit]
  exact vo.foldl_append_le hinfl _ _ s

end HydroV2
