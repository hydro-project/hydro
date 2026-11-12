import Hydro.HydroTick

/-!
# `HydroGen` toy modules (`HydroGenToy`)

A minimal non-Paxos program suite exercising every artifact class the `hydro
def` pipeline generates; the checks live in `HydroGenCheck.lean`:

- `toy_relay`: a knot-free module with two **content** decisions (two batch
  sites) and one **sched-det** (an interval timer's fires), a `tick` block, a
  colocated `Values` contract (`ensures`), and two outputs. The shape of
  `Std/Quorum.lean`'s `collect_quorum`, minimized.
- `toy_step`: a knot-free module calling `toy_relay` (a folded callee). No
  callee reaches a knot, so the pipeline routes it to the couple phase; it is
  the body of `toy_loop`'s knot.
- `toy_loop`: a `fix` knot whose body is `toy_step`, with an `invariant`
  clause.

The modules are written in the house style; everything around them (the
`sr`/`rr` equations, derived decisions, causality, wf threading, monotonicity, free theorems,
knot stages) is generated.
-/

namespace Hydro

/-- Elements of a legal batch cut live in the pool: count-legality implies
support membership. -/
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

/-- What `toy_relay` **ensures** over the `Values` denotation: the relayed
stream carries only successors (every emitted value is some consumed input
`+ 1`, hence positive). -/
structure ToyEnsures {n : Nat}
    (out : (MemberMap n (Multiset Nat)) × (MemberMap n (Trace (Multiset Nat)))) :
    Prop where
  /-- Every relayed value is positive. -/
  pos : ∀ i, ∀ x ∈ out.1 i, 1 ≤ x

/-- The toy relay's decisions, one record per module: the batch cuts of the
content and the batch cuts of the interval timer's `()`s, both `nondet!`
sites. -/
structure ToyRelayDec (H : HydroSem L mem) (n : Nat) where
  batch : H.BatchDec n Nat .noOrder .exactlyOnce
  gate : H.BatchDec n Unit .totalOrder .atLeastOnce

/-- The toy relay's sched-det bundle: the interval timer's fire timing, runtime
freedom that is `Unit` at `Values`. -/
structure ToyRelaySched (H : HydroSem L mem) (n : Nat) where
  interval : H.IntervalDec n

/-- The loop body's sched-det bundle: the relay's, nested, and the loop's hop. A
knot body is strict in its wire (every cycle passes a network hop or a
`defer_tick`), and the toy closes its knot through a self-broadcast. -/
structure ToyStepSched (H : HydroSem L mem) (n : Nat) where
  relay : ToyRelaySched H n
  net : H.TransportDec n n

/-- The toy relay: bump each input, batch the bumped stream per tick, gate the
batches on an interval timer (`source_interval(..).batch(&tick, ..).first()
.is_some()` as `filter_if` in the tick), and leave the tick. Two content
decisions (the two batch cuts) and one sched-det (the timer's fires). -/
hydro def toy_relay (H : HydroSem L mem) (ℓ : L)
    (inp : H.Stream ℓ Nat .noOrder .exactlyOnce)
    (dec : ToyRelayDec H (mem ℓ)) (sched : ToyRelaySched H (mem ℓ)) :
    (H.Stream ℓ Nat .noOrder .exactlyOnce
      × H.AtomicStream ℓ Nat .noOrder .exactlyOnce)
  ensures out => ToyEnsures out :=
  let bumped := H.map inp (fun _i n => n + 1)
  let b := H.batch bumped dec.batch
  let fires := H.source_interval (fun _ => 0) 1 sched.interval
  let gate := H.batch fires dec.gate
  tick (input bb := b) (input g := gate) :=
    yield (emitted := H.bfilterIf bb (H.boIsSome (H.bfirst g)));
  (H.allTicks emitted, emitted)
  prove
    pos := fun i x hx => by
      -- `Values`: the output pool equals the sum of the per-tick emissions, and a
      -- tick's emission equals the body on that tick's reads (`_at`)
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
      -- the batch is a legal cut of the bumped pool (`b`/`bumped` read at
      -- `Values`, one operator at a time)
      simp only [bb, b, bumped, den] at hb
      have hle := mem_of_mem_batchCuts (Trace.mem_of_read hb)
      obtain ⟨y, -, rfl⟩ := Multiset.mem_map.mp (Multiset.mem_of_le hle hxm)
      exact Nat.le_add_left 1 y

/-- The loop body as a house module: relay the knot wire, ship it through a hop
(so the body is strict in `s`) and re-enter it merged with the external
input. -/
hydro def toy_step (H : HydroSem L mem) (ℓ : L)
    (s : H.Stream ℓ Nat .noOrder .exactlyOnce)
    (inp : H.Stream ℓ Nat .noOrder .exactlyOnce)
    (dec : ToyRelayDec H (mem ℓ)) (sched : ToyStepSched H (mem ℓ)) :
    H.Stream ℓ Nat .noOrder .exactlyOnce :=
  H.union (H.values (H.broadcast_closed sched.net (toy_relay H ℓ s dec sched.relay).1)) inp

/-- The toy loop's decisions, record-classified: the fuel travels in a decision
record, because a bare `H.FixDec` binder is a hazard for the composition
walker. -/
structure ToyLoopDec (H : HydroSem L mem) (n : Nat) where
  relay : ToyRelayDec H n
  fuel : H.FixDec

/-- The toy knot: close `toy_step` over the loop wire, with a minimal `invariant`
clause (chain-`let` binding at both wire valuations, obligation generation with
the `⊑`-premises, the bounded-chain composition over the generated `stages`,
the `<def>.<wire>.inv` hoist). -/
hydro def toy_loop (H : HydroSem L mem) (ℓ : L)
    (inp : H.Stream ℓ Nat .noOrder .exactlyOnce)
    (tdec : ToyLoopDec H (mem ℓ)) (sched : ToyStepSched H (mem ℓ)) :
    H.Stream ℓ Nat .noOrder .exactlyOnce :=
  fix (w : H.Stream ℓ Nat .noOrder .exactlyOnce) via tdec.fuel
    invariant w (out out') => ∀ i, out i ≤ out' i,
      base := fun _htop => toy_step_mono₁ (inp := inp) (inp' := inp)
        ℓ tdec.relay sched
        (fun _ => PoolLe.bot_le _ _ _) (fun _ => PoolLe.refl _ _ _),
      step := fun _htop _hw hbw _hbw0 _ih => toy_step_mono₁ (inp := inp)
        (inp' := inp) ℓ tdec.relay sched hbw
        (fun _ => PoolLe.refl _ _ _)
  :=
    let out := toy_step H ℓ w inp tdec.relay sched
    complete (out)
    out

/-! Executable smoke test: `@Values` evaluates. -/

abbrev toyOne : Unit → Nat := fun _ => 1

#guard ((toy_relay (Values Unit toyOne) () (.mk' fun _ => {3, 5})
    ⟨fun _ => [{4}, {6}], fun _ => [[()], [()]]⟩ ⟨()⟩).2 0)
  = [{4}, {6}]
-- the timer gates: a tick with no fire drops its batch
#guard ((toy_relay (Values Unit toyOne) () (.mk' fun _ => {3, 5})
    ⟨fun _ => [{4}, {6}], fun _ => [[()], []]⟩ ⟨()⟩).2 0)
  = [{4}, 0]

end Hydro
