import HydroV2.HydroGenToy
import HydroV2.HydroTick

/-!
# HydroV2 · the `tick` construct, validated (`HydroTickCheck`)

The `tick` construct (`HydroTick.lean`) desugared on toy programs:
a cross-tick register loop is an `H.scan_across_ticks` leg (a plain
structural fold — no fuel, no knot), and its `invariant` clause is an
auto-named ghost `have` discharged by `scanAcrossTicks_invariant`
(plain loop induction) from the user's `init`/`tick` obligations.

Covered: the plain form (no invariant), an invariant with a spectator
consumed through a prove leg (the ghost replays there — invariant
obligations only CHECK under an `ensures`/`prove`), and the two-state
form (the construct productifies the register). The invariant ghosts
are definitional at `Values`, so the toys stay axiom-clean (checked in
`AxCheck.lean` fashion by `#print axioms` here) and executable
(`#guard`).
-/

namespace HydroV2
variable {L : Type} {mem : L → Nat}

structure ToyTickEnsures {k : Nat} (o : TickV k Nat .unbounded) :
    Prop where
  ok : True

-- plain: running sum, no invariant
set_option maxHeartbeats 1600000 in
hydro def toy_tick (H : HydroSem L mem) (ℓ : L)
    (inp : H.TickSingleton ℓ Nat .unbounded) :
    H.TickSingleton ℓ Nat .unbounded :=
  tick (state acc : Nat := 0)
      (input x := inp) :=
    let next := acc + x
    rebind (acc := next)
    emit (outv := next);
  outv

#print axioms toy_tick
#guard (toy_tick (Values Unit (fun _ => 1)) () (fun _ => [1, 2, 3])) 0
  = [1, 3, 6]

-- invariant + spectator (the input), consumed by a prove leg
set_option maxHeartbeats 1600000 in
hydro def toy_tick2 (H : HydroSem L mem) (ℓ : L)
    (inp : H.TickSingleton ℓ Nat .unbounded) :
    (H.TickSingleton ℓ Nat .unbounded)
  ensures out => ToyTickEnsures out :=
  tick (state acc : Nat := 0)
      (input x := inp)
      (invariant (outv acc x) =>
        acc = outv.getLastD 0 ∧ x = x) :=
    let next := acc + x
    rebind (acc := next)
    emit (outv := next)
    prove init := fun _i => ⟨rfl, rfl⟩,
      tick := fun _i _n _hn _out _st _hlen _ih =>
        ⟨by
          simp only [List.getLastD_eq_getLast?, List.getLast?_concat,
            Option.getD_some]
          rfl,
         rfl⟩;
  outv
  prove
    ok := trivial

#print axioms toy_tick2
#guard (toy_tick2 (Values Unit (fun _ => 1)) ()
  (fun _ => [1, 2, 3])).val 0 = [1, 3, 6]

-- two states: productified by the construct
set_option maxHeartbeats 1600000 in
hydro def toy_tick3 (H : HydroSem L mem) (ℓ : L)
    (inp : H.TickSingleton ℓ Nat .unbounded) :
    (H.TickSingleton ℓ Nat .unbounded)
  ensures out => ToyTickEnsures out :=
  tick (state acc : Nat := 0) (state cnt : Nat := 0)
      (input x := inp)
      (invariant (outv acc cnt) => acc = outv.getLastD 0) :=
    rebind (acc := acc + x, cnt := cnt + 1)
    emit (outv := acc + x)
    prove init := fun _i => rfl,
      tick := fun _i _n _hn _out _st _hlen _ih => by
        simp only [List.getLastD_eq_getLast?, List.getLast?_concat,
          Option.getD_some]
        rfl;
  outv
  prove
    ok := trivial

#print axioms toy_tick3
#guard (toy_tick3 (Values Unit (fun _ => 1)) ()
  (fun _ => [1, 2, 3])).val 0 = [1, 3, 6]

end HydroV2
