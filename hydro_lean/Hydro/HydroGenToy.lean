import Hydro.HydroTick

/-!
# Hydro · `HydroGen` toy modules (`HydroGenToy`)

A minimal NON-Paxos program suite exercising every artifact class the
`hydro def` pipeline generates (validation lives in
`HydroGenCheck.lean`):

- `toy_relay` — knot-free module with one **content** decision (a
  batch site) and one **timing** decision (a pulse), a `tick` block, a
  colocated `Values` contract (`ensures`), and two output legs. The
  shape of `Std/Quorum.lean`'s `collect_quorum`, minimized.
- `toy_step` — a plain glue module calling `toy_relay` (inner-module
  folding through a knot boundary).
- `toy_loop` — a `fix` knot whose body is `toy_step`, with an
  `invariant` clause (the D57 K4 shape, minimized).

The modules are written exactly in the house style — the point is that
everything AROUND them (naming lemmas, derived decisions, causality,
wf threading, monotonicity, free theorems, knot stages) is generated.
-/

namespace Hydro

/-- Elements of a legal batch cut live in the pool (count-legality
implies support membership). -/
theorem mem_of_mem_batchCuts {α : Type} [DecidableEq α]
    {pool consumed : Multiset α} {d : List (Multiset α)}
    {t : Multiset α} (ht : t ∈ batchCuts pool consumed d) :
    t ≤ pool := by
  induction d generalizing consumed with
  | nil => simp [batchCuts] at ht
  | cons b ds ih =>
    unfold batchCuts at ht
    by_cases h : consumed + b ≤ pool
    · rw [if_pos h] at ht
      rcases List.mem_cons.mp ht with rfl | ht
      · exact le_trans (Multiset.le_add_left _ _) h
      · exact ih ht
    · rw [if_neg h] at ht
      simp at ht

variable {L : Type} {mem : L → Nat}

/-- What `toy_relay` **ensures** over the `Values` denotation: the
relayed stream only carries successors (every emitted value is some
consumed input `+ 1`, hence positive). -/
structure ToyEnsures {n : Nat}
    (out : (Fin n → Multiset Nat) × (Fin n → Trace (Multiset Nat))) :
    Prop where
  /-- Every relayed value is positive. -/
  pos : ∀ i, ∀ x ∈ out.1 i, 1 ≤ x

/-- The toy relay: bump each input, batch the bumped stream per tick,
gate the batches on a timing pulse (`filter_if` in the tick), and leave
the tick. One content decision (the batch cuts) and one timing
decision (the pulse). -/
hydro def toy_relay (H : HydroSem L mem) (ℓ : L)
    (inp : H.Stream ℓ Nat .noOrder .exactlyOnce)
    (dec : H.BatchDec (mem ℓ) Nat)
    (pt : H.PulseDec (mem ℓ)) :
    (H.Stream ℓ Nat .noOrder .exactlyOnce
      × H.TickStream ℓ Nat .noOrder .exactlyOnce)
  ensures out => ToyEnsures out :=
  let bumped := H.map inp (fun _i n => n + 1)
  let b := H.batch bumped dec
  let gate := H.source_interval_batch pt
  tick (input bb := b) (input g := gate) :=
    yield (emitted := H.bfilterIf bb g);
  (H.allTicks emitted, emitted)
  prove
    pos := fun i x hx => by
      -- `Values`: the output pool is the sum of the per-tick emissions;
      -- a tick's emission is the body on that tick's reads (`_at`)
      simp only [den] at hx
      obtain ⟨e, he, hxe⟩ := mem_list_sum.mp hx
      obtain ⟨n, hn⟩ := List.mem_iff_getElem?.mp he
      obtain ⟨bb_t, g_t, hb, -, rfl⟩ := (hemitted_at i n e).mp hn
      -- the gate only drops: the element sits in the batch
      have hxm : x ∈ bb_t := by
        simp only [emitted_step, den] at hxe
        unfold poolFilterIf at hxe
        split at hxe
        · exact hxe
        · exact absurd hxe (Multiset.notMem_zero x)
      -- the batch is a legal cut of the bumped pool (`b`/`bumped` read
      -- at the denotation, one operator at a time)
      simp only [bb, b, bumped, den] at hb
      have hle := mem_of_mem_batchCuts (Trace.mem_of_read hb)
      obtain ⟨y, -, rfl⟩ := Multiset.mem_map.mp (Multiset.mem_of_le hle hxm)
      exact Nat.le_add_left 1 y

/-- The loop body, as a house module (hoisted, D40): relay the knot
wire and re-enter it merged with the external input. -/
hydro def toy_step (H : HydroSem L mem) (ℓ : L)
    (s : H.Stream ℓ Nat .noOrder .exactlyOnce)
    (inp : H.Stream ℓ Nat .noOrder .exactlyOnce)
    (dec : H.BatchDec (mem ℓ) Nat)
    (pt : H.PulseDec (mem ℓ)) :
    H.Stream ℓ Nat .noOrder .exactlyOnce :=
  H.union (toy_relay H ℓ s dec pt).1 inp

/-- The toy loop's decisions (record-classified, house style: the fuel
travels in a decision record — a bare `H.FixDec` binder is the D55
walker hazard). -/
structure ToyLoopDec (H : HydroSem L mem) (n : Nat) where
  batch : H.BatchDec n Nat
  pulse : H.PulseDec n
  fuel : H.FixDec

/-- The toy knot: close `toy_step` over the loop wire, with a minimal
`invariant` clause (chain-`let` binding at both wire valuations,
obligation generation with the `⊑`-premises, the bounded-chain
composition over the generated `stages`, the `<def>.<wire>.inv`
hoist). -/
hydro def toy_loop (H : HydroSem L mem) (ℓ : L)
    (inp : H.Stream ℓ Nat .noOrder .exactlyOnce)
    (tdec : ToyLoopDec H (mem ℓ)) :
    H.Stream ℓ Nat .noOrder .exactlyOnce :=
  fix (w : H.Stream ℓ Nat .noOrder .exactlyOnce) via tdec.fuel
    invariant w (out out') => ∀ i, out i ≤ out' i,
      base := fun _htop => toy_step_mono₁ (inp := inp) (inp' := inp)
        ℓ tdec.batch tdec.pulse
        (fun _ => PoolLe.bot_le _ _ _) (fun _ => PoolLe.refl _ _ _),
      step := fun _htop _hw hbw _hbw0 _ih => toy_step_mono₁ (inp := inp)
        (inp' := inp) ℓ tdec.batch tdec.pulse hbw
        (fun _ => PoolLe.refl _ _ _)
  :=
    let out := toy_step H ℓ w inp tdec.batch tdec.pulse
    complete (out)
    out

/-! Executable smoke test (`@Values` evaluates). -/

abbrev toyOne : Unit → Nat := fun _ => 1

#guard ((toy_relay (Values Unit toyOne) () (fun _ => {3, 5})
    (fun _ => [{4}, {6}]) (fun _ => [true, true])).2 0)
  = [{4}, {6}]
-- the pulse gates: a false tick drops its batch
#guard ((toy_relay (Values Unit toyOne) () (fun _ => {3, 5})
    (fun _ => [{4}, {6}]) (fun _ => [true, false])).2 0)
  = [{4}, 0]

end Hydro
