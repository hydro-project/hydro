import HydroV2.CoupleProj
import HydroV2.MonoRel

/-!
# HydroV2 · `HydroGen` toy modules (`HydroGenToy`)

A minimal NON-Paxos program suite exercising every artifact class the
`HydroGen` commands generate (validation lives in
`HydroGenCheck.lean`):

- `toy_relay` — knot-free module with one **content** decision (a
  batch site) and two **machine-data** decisions (a timing pulse and
  an emission linearization), a colocated `Values` contract, and two
  output legs. The shape of `Std/Quorum.lean`'s `collect_quorum`,
  minimized.
- `toyBody`/`toy_loop` — a `HydroSem.fix` knot whose body calls
  `toy_relay` (inner-module folding through a knot boundary), in the
  hoisted-constant style of `LeaderElection.lean` (D40).

The modules are hand-written exactly in the house style — the point
is that everything AROUND them (naming lemmas, derived decisions,
causality, wf threading, monotonicity) is generated.
-/

namespace HydroV2

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

/-- Membership in the sum of a list of multisets. -/
theorem mem_of_mem_list_sum {α : Type} {l : List (Multiset α)}
    {x : α} (hx : x ∈ l.sum) : ∃ ms ∈ l, x ∈ ms := by
  induction l with
  | nil => simp at hx
  | cons b bs ih =>
    rw [List.sum_cons, Multiset.mem_add] at hx
    rcases hx with hx | hx
    · exact ⟨b, List.mem_cons_self, hx⟩
    · obtain ⟨ms, hms, hxm⟩ := ih hx
      exact ⟨ms, List.mem_cons_of_mem _ hms, hxm⟩

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
gate the batches on a timing pulse (identity payload — the gate is a
timing artifact), emit them, and leave the tick. One content decision
(the batch cuts) and two machine-data decisions (the pulse and the
emission linearization). -/
def toy_relay (H : HydroSem L mem) (ℓ : L)
    (inp : H.Stream ℓ Nat .noOrder .exactlyOnce)
    (dec : H.BatchDec (mem ℓ) Nat)
    (pt : H.PulseDec (mem ℓ))
    (de : H.EmitDec (mem ℓ) Nat) :
    {out : H.Stream ℓ Nat .noOrder .exactlyOnce
        × H.TickStream ℓ Nat .noOrder .exactlyOnce //
      ∀ hv : H = Values L mem,
        match H, hv, out with
        | _, rfl, o => ToyEnsures o} :=
  let bumped := H.map inp (fun _i n => n + 1)
  let b := H.batch bumped dec
  let gate := H.source_interval_batch pt
  let staged := H.mapBatchesUnordered b gate (fun _i ms _g => ms)
  let emitted := H.emitMultisetBatches staged de
  ⟨(H.allTicks emitted, emitted), by
    intro hv
    subst hv
    constructor
    intro i x hx
    -- Values: the output pool is the sum of the gated batch cuts of
    -- the bumped pool
    obtain ⟨ms, hms, hxm⟩ := mem_of_mem_list_sum hx
    obtain ⟨bx, hbx, rfl⟩ := List.mem_map.mp hms
    have hb := (List.of_mem_zip hbx).1
    have hle := mem_of_mem_batchCuts hb
    have : x ∈ mapPool (ord := .noOrder) (fun n => n + 1) (inp i) :=
      Multiset.mem_of_le hle hxm
    obtain ⟨y, -, rfl⟩ := Multiset.mem_map.mp this
    exact Nat.le_add_left 1 y⟩

/-- The loop body, as a house module (hoisted, D40): relay the knot
wire and re-enter it merged with the external input. The knot wrapper
below passes it `H'`-generically — its generated stack (naming,
causality, wf, monotonicity) is what the knot machinery consumes. -/
def toy_step (H : HydroSem L mem) (ℓ : L)
    (s : H.Stream ℓ Nat .noOrder .exactlyOnce)
    (inp : H.Stream ℓ Nat .noOrder .exactlyOnce)
    (dec : H.BatchDec (mem ℓ) Nat)
    (pt : H.PulseDec (mem ℓ))
    (de : H.EmitDec (mem ℓ) Nat) :
    H.Stream ℓ Nat .noOrder .exactlyOnce :=
  H.union (toy_relay H ℓ s dec pt de).val.1 inp

/-- The toy knot: close `toy_step` over the loop wire
(`HydroSem.fix`, captures curried). -/
def toy_loop (H : HydroSem L mem) (ℓ : L)
    (inp : H.Stream ℓ Nat .noOrder .exactlyOnce)
    (dec : H.BatchDec (mem ℓ) Nat)
    (pt : H.PulseDec (mem ℓ))
    (de : H.EmitDec (mem ℓ) Nat)
    (df : H.FixDec) :
    H.Stream ℓ Nat .noOrder .exactlyOnce :=
  H.fix df (dec, pt, de, inp)
    (fun H'' (caps : H''.BatchDec (mem ℓ) Nat × H''.PulseDec (mem ℓ)
        × H''.EmitDec (mem ℓ) Nat
        × H''.Stream ℓ Nat .noOrder .exactlyOnce) s =>
      toy_step H'' ℓ s caps.2.2.2 caps.1 caps.2.1 caps.2.2.1)

/-! Executable smoke test (`@Values` evaluates). -/

abbrev toyOne : Unit → Nat := fun _ => 1

#guard ((toy_relay (Values Unit toyOne) () (fun _ => {3, 5})
    (fun _ => [{4}, {6}]) (fun _ => [true, true]) ()).val.2 0)
  = [{4}, {6}]

end HydroV2
