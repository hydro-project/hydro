import HydroLean.Prelude

/-!
# Gyatso abstract-rewriting aliases

Thin aliases re-exporting the general abstract-rewriting results from
`HydroLean.Prelude` (`LocallyConfluent`, `newman`, joinability helpers) under
the `HydroLean.Gyatso` namespace, where they are used to prove confluence of
network and cluster operators (dissertation §3.3.3, §3.5).

The proofs formerly lived here as a local copy (to avoid racing concurrent work
on `Prelude.lean`); they have been unified into `Prelude.lean` and only the
names remain, for source stability of the Gyatso layer.
-/

namespace HydroLean.Gyatso

/-- Local confluence (weak Church–Rosser). Alias of
`HydroLean.LocallyConfluent`. -/
abbrev LocallyConfluent {α : Sort u} (r : α → α → Prop) : Prop :=
  HydroLean.LocallyConfluent r

/-- **Newman's lemma**. Alias of `HydroLean.newman`. -/
theorem newman {α : Sort u} {r : α → α → Prop} (hsn : SN r)
    (hlc : LocallyConfluent r) : Confluent r :=
  HydroLean.newman hsn hlc

/-- A single step yields joinability of its endpoints. Alias of
`Joinable.of_step`. -/
theorem joinable_of_step {α : Sort u} {r : α → α → Prop} {a b : α} (h : r a b) :
    Joinable r a b :=
  Joinable.of_step h

/-- Equal points are joinable. Alias of `Joinable.of_eq`. -/
theorem joinable_of_eq {α : Sort u} {r : α → α → Prop} {a b : α} (h : a = b) :
    Joinable r a b :=
  Joinable.of_eq h

end HydroLean.Gyatso
