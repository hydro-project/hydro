import Lean

/-!
# Simp sets shared across the engine

`register_simp_attr` must live in a module imported by every module that
TAGS a lemma with the attribute, so the sets are declared here, before
`Grades.lean`.

- `den`: **the denotation, one operator at a time** — every `HydroSem`
  operator at `Values` as a `rfl` rewrite (stream-level: `H.map` ↦
  `mapPool`, `H.allTicks` ↦ `.sum`/`.flatten`, `H.broadcast_closed`,
  `H.values`, …; in-tick: `H.bmap` ↦ `mapPool`, `H.bcount` ↦ `poolCount`,
  …) and the pool operators at concrete grades (`mapPool (ord :=
  .totalOrder) f l = l.map f`, …). `simp only [den]` reads a wire of
  THIS module at the denotation as the plain `List`/`Multiset` term of
  its Rust lines — a stream pipeline between calls, a boundary fan-in,
  or (with the block's `<out>_step`) one tick of a `tick` block — so a
  ghost states facts about the PROGRAM directly: no hand-written pure
  twin of a body, no body-shape bridge lemma (FINDINGS D64, E6). What
  a ghost meets, and what it cites: an operator → `simp only [den]`; a
  `tick` block → its `_run`/`_at`/`_reg` readers; a module call → the
  callee's face (never unfolded); a `fix` knot → the generated
  `stages`.
-/

register_simp_attr den
