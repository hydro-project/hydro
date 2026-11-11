import Lean

/-!
# Simp sets shared across the engine

Every module that tags a lemma with an attribute must import the attribute's
`register_simp_attr`, so this module, below every other module, declares the engine's simp sets.

- `den` is **the denotation, one operator at a time**: every `HydroSem` operator at
  `Values` as a `rfl` rewrite (`H.map` ↦ `mapPool`, `H.allTicks` ↦ `.sum`/`.flatten`,
  `H.bmap` ↦ `mapPool`, `H.bcount` ↦ `poolCount`, …) together with the pool operators at
  concrete grades (`mapPool (ord := .totalOrder) f l = l.map f`, …). `simp only [den]`
  therefore reads a wire of the current module as the plain `List`/`Multiset` term of its
  Rust lines — a stream pipeline between calls, a boundary fan-in, or (with the block's
  `<out>_step`) one tick of a `tick` block — and a ghost states facts about the program
  itself. A ghost cites an operator through `simp only [den]`, a `tick` block through its
  `_run`/`_at`/`_reg` readers, a module call through the callee's face, and a `fix` knot
  through the generated `stages`.
-/

register_simp_attr den

/-- The corner's **projection set** (`CoupleProj.lean`): every `co_<op>_rr`/`co_<op>_sr`
rfl rewrite that pushes the `sr` and `rr` fields of a `CoupleSem` run through an operator, plus the tick former's
facade rules. `co_simp [extra…]` is `simp only [co_simp_rules, …]`; as a simp attribute the
set is built once per module and not once per call. -/
register_simp_attr co_simp_rules

/-- The corner's **`wf` normalizer set** (`CoupleProj.lean`'s `coWfSimpRules`), a simp
attribute for the same reason. -/
register_simp_attr co_wf_simp_rules

/-- The corner's **lag projection set** (`CoupleProj.lean`'s `coLagRules`): a gadget's `lag`
through every operator. -/
register_simp_attr co_lag_rules
