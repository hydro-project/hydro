import Hydro.HydroGenToy
import Hydro.HydroTick
import Hydro.EagerRel

/-!
# Hydro · the `tick` construct, validated (`HydroTickCheck`)

The `tick` construct (`HydroTick.lean`) desugared on toy programs:
a cross-tick register loop is an `H.tick_scan` leg (a plain
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

namespace Hydro
variable {L : Type} {mem : L → Nat}

structure ToyTickEnsures {k : Nat} (o : TickV k Nat) :
    Prop where
  ok : True

-- plain: running sum, no invariant (the body is in-tick operators over
-- this tick's singletons: `bsZip`/`bsMap` are Rust's `.zip(…).map(…)`)
hydro def toy_tick (H : HydroSem L mem) (ℓ : L)
    (inp : H.Ticked ℓ Nat) :
    H.Ticked ℓ Nat :=
  tick (state acc : Nat := 0)
      (input x := inp) :=
    let next := H.bsMap (H.bsZip acc x) (fun p => p.1 + p.2)
    rebind (acc := next)
    emit (outv := next);
  outv

#print axioms toy_tick
#guard (toy_tick (Values Unit (fun _ => 1)) () (fun _ => [1, 2, 3])) 0
  = [1, 3, 6]

-- invariant + spectator (the input), consumed by a prove leg
hydro def toy_tick2 (H : HydroSem L mem) (ℓ : L)
    (inp : H.Ticked ℓ Nat) :
    (H.Ticked ℓ Nat)
  ensures out => ToyTickEnsures out :=
  tick (state acc : Nat := 0)
      (input x := inp)
      (invariant ((outv : List Nat) (acc : Nat) x) =>
        acc = outv.getLastD 0 ∧ x = x) :=
    let next := H.bsMap (H.bsZip acc x) (fun p => p.1 + p.2)
    rebind (acc := next)
    emit (outv := next)
    prove init := fun _i => ⟨rfl, rfl⟩,
      tick := fun _i _n _out _st _x_t _hx _hlen _ih =>
        ⟨by
          simp only [List.append_eq, List.getLastD_eq_getLast?,
            List.getLast?_concat, Option.getD_some]
          rfl,
         rfl⟩;
  outv
  prove
    ok := trivial

#print axioms toy_tick2
#guard (toy_tick2 (Values Unit (fun _ => 1)) ()
  (fun _ => [1, 2, 3])) 0 = [1, 3, 6]

-- two states: productified by the construct (one `BoundedSingleton`
-- register of the pair, opened into the names by `bsMap`)
hydro def toy_tick3 (H : HydroSem L mem) (ℓ : L)
    (inp : H.Ticked ℓ Nat) :
    (H.Ticked ℓ Nat)
  ensures out => ToyTickEnsures out :=
  tick (state acc : Nat := 0) (state cnt : Nat := 0)
      (input x := inp)
      (invariant ((outv : List Nat) (acc : Nat) (cnt : Nat)) =>
        acc = outv.getLastD 0) :=
    let next := H.bsMap (H.bsZip acc x) (fun p => p.1 + p.2)
    rebind (acc := next, cnt := H.bsMap cnt (· + 1))
    emit (outv := next)
    prove init := fun _i => rfl,
      tick := fun _i _n _out _st _x_t _hx _hlen _ih => by
        simp only [List.append_eq, List.getLastD_eq_getLast?,
          List.getLast?_concat, Option.getD_some]
        rfl;
  outv
  prove
    ok := trivial

#print axioms toy_tick3
#guard (toy_tick3 (Values Unit (fun _ => 1)) ()
  (fun _ => [1, 2, 3])) 0 = [1, 3, 6]

-- two inputs of both kinds, a singleton emission and a stream
-- emission (`yield`): the construct zips the inputs at the TYPE level
hydro def toy_tick4 (H : HydroSem L mem) (ℓ : L)
    (inp : H.Ticked ℓ Nat)
    (bs : H.TickStream ℓ Nat .totalOrder .exactlyOnce) :
    H.Ticked ℓ Nat × H.TickStream ℓ Nat .totalOrder .exactlyOnce :=
  tick (state acc : Nat := 0)
      (input x := inp) (input b := bs) :=
    let n := H.bcount b
    let next := H.bsMap (H.bsZip acc (H.bsMap (H.bsZip x n) (fun p => p.1 + p.2)))
      (fun p => p.1 + p.2)
    rebind (acc := next)
    emit (total := next)
    yield (bumped := H.bmap b (· + 1));
  (total, bumped)

#print axioms toy_tick4
#check @toy_tick4_co_wf₁
#check @toy_tick4_param₂
#guard (toy_tick4 (Values Unit (fun _ => 1)) () (fun _ => [1, 2, 3])
  (fun _ => [[10], [], [1, 2]])).1 0 = [2, 4, 9]
#guard (toy_tick4 (Values Unit (fun _ => 1)) () (fun _ => [1, 2, 3])
  (fun _ => [[10], [], [1, 2]])).2 0 = [[11], [], [2, 3]]

structure ToyTick6Ensures {k : Nat} (o : Fin k → Trace (List Nat)) :
    Prop where
  /-- The emitted batches, concatenated, are exactly as many elements as
  all the input batches so far. -/
  total : ∀ (i : Fin k), True

-- the invariant clause, v3 (D63): TWO inputs (a singleton and a stream),
-- a `yield` (stream) output, a spectator; the `tick` obligation is the
-- Verus loop body — this tick's reads `x_t`/`b_t` (one per input, by
-- `(x i)[n]? = some x_t`), the emissions so far, the register, and the
-- step's components on the named reads; no zipped tuple, no bound
hydro def toy_tick6 (H : HydroSem L mem) (ℓ : L)
    (inp : H.Ticked ℓ Nat)
    (bs : H.TickStream ℓ Nat .totalOrder .exactlyOnce) :
    H.TickStream ℓ Nat .totalOrder .exactlyOnce
  ensures out => ToyTick6Ensures out :=
  tick (state acc : Nat := 0)
      (input x := inp) (input b := bs)
      (invariant ((outv : List (List Nat)) (acc : Nat) (x : List Nat)) =>
        acc = (outv.map List.length).sum ∧ outv.length ≤ x.length) :=
    let n := H.bcount b
    let next := H.bsMap (H.bsZip acc n) (fun p => p.1 + p.2)
    rebind (acc := next)
    yield (bumped := H.bmap b (· + 1))
      prove init := fun _i => ⟨rfl, by simp⟩,
        tick := fun _i _n out st _x_t b_t hx _hb _hlen ih => by
          obtain ⟨hsum, hle⟩ := ih
          refine ⟨?_, ?_⟩
          · -- the in-tick `bcount`/`bmap` ARE the list length/map at the
            -- denotation: the `den` simp set reads the generated step
            -- (D64 E6 — no `show`, no hand-written twin of the body)
            simp only [bumped_step, den, List.append_eq, List.map_append,
              List.sum_append, ← hsum]
            simp
          · simp only [List.append_eq, List.length_append, List.length_singleton]
            have := Trace.read_lt hx
            omega;
    -- the construct's readers: the wire IS the fold (`_run`), tick `n`'s
    -- emission is the body on tick `n`'s reads (`_at`), the invariant at
    -- every prefix (`_inv_take`) — consumed here
    ghost have hread : ∀ i n e, (bumped i)[n]? = some e →
        ∃ b_t : List Nat, (bs i)[n]? = some b_t ∧ e = b_t.map (· + 1) := fun i n e he => by
      obtain ⟨_, b_t, _, hb, rfl⟩ := (hbumped_at i n e).mp he
      exact ⟨b_t, hb, rfl⟩
    ghost have hpre : ∀ i n, ((bumped i).take n).length ≤ (inp i).length :=
      fun i n => (hbumped_inv_take i n).2
    -- the register ∃-abstracted (`_reg`, D64 E7): seed, read, step, one
    -- stall per input, the invariant before every tick — the fold is
    -- never seen
    ghost obtain ⟨acc_reg, hreg0, hreg_at, hreg_succ, hreg_stall_x, hreg_stall_b,
      hreg_inv⟩ := hbumped_reg
    ghost have hreg_climb : ∀ i n x_t b_t, (inp i)[n]? = some x_t → (bs i)[n]? = some b_t →
        acc_reg i (n + 1) = acc_reg i n + b_t.length := fun i n x_t b_t hx hb => by
      rw [hreg_succ i n x_t b_t hx hb]
      simp only [bumped_step, den]
    ghost have hreg_sum : ∀ i n, acc_reg i n = (((bumped i).take n).map List.length).sum :=
      fun i n => (hreg_inv i n).1
    ghost have hreg_zero : ∀ i, acc_reg i 0 = 0 := hreg0
  bumped
  prove
    total := fun _ => trivial

#print axioms toy_tick6
#guard (toy_tick6 (Values Unit (fun _ => 1)) () (fun _ => [1, 2, 3])
  (fun _ => [[10], [], [1, 2]])) 0 = [[11], [], [2, 3]]

-- a persisted stream state (Rust `use::state` on a `Stream<…, Tick,
-- Bounded>` — `quorum.rs` `state_null`): the register is a shape, seeded
-- empty; reads see everything chained in at EARLIER ticks
hydro def toy_tick5 (H : HydroSem L mem) (ℓ : L)
    (bs : H.TickStream ℓ Nat .totalOrder .exactlyOnce) :
    H.Ticked ℓ Nat :=
  tick (state seen : H.BoundedStream Nat .totalOrder .exactlyOnce)
      (input b := bs) :=
    rebind (seen := H.bchain seen b)
    emit (sofar := H.bcount seen);
  sofar

#print axioms toy_tick5
#guard (toy_tick5 (Values Unit (fun _ => 1)) ()
  (fun _ => [[10], [], [1, 2], [5]])) 0 = [0, 1, 1, 3]

-- a program-level `if` over a static parameter inside the body (Rust's
-- construction-time `if max == min { … } else { … }`, quorum.rs): one
-- term, both branches walked by every generator
hydro def toy_ite (H : HydroSem L mem) (ℓ : L) (n : Nat)
    (inp : H.Ticked ℓ Nat) :
    H.Ticked ℓ Nat :=
  tick (state acc : Nat := 0)
      (input x := inp) :=
    let next := if n = 0 then H.bsMap acc (· + 1)
      else H.bsMap (H.bsZip acc x) (fun p => p.1 + p.2)
    rebind (acc := next)
    emit (outv := next);
  outv

#print axioms toy_ite
#guard (toy_ite (Values Unit (fun _ => 1)) () 0 (fun _ => [1, 2, 3])) 0 = [1, 2, 3]
#guard (toy_ite (Values Unit (fun _ => 1)) () 1 (fun _ => [1, 2, 3])) 0 = [1, 3, 6]

-- the keyed vocabulary (quorum.rs's shape in miniature): a persisted
-- stream state, `chain`, `into_keyed().fold()`, `filter`/`keys`,
-- `anti_join`, a stream emission (`yield`) leaving through `all_ticks`
hydro def toy_keyed (H : HydroSem L mem) (ℓ : L) (min : Nat)
    (resp : H.Stream ℓ (Nat × Bool) .noOrder .exactlyOnce)
    (dec : H.BatchDec (mem ℓ) (Nat × Bool)) :
    H.Stream ℓ Nat .noOrder .exactlyOnce :=
  let new_inputs := H.batch resp dec
  tick (state not_all : H.BoundedStream (Nat × Bool) .noOrder .exactlyOnce)
      (input b := new_inputs) :=
    let current := H.bchain not_all b
    let count_per_key := H.bkeyedFold
      (fun (a : Nat) (v : Bool) => if v then a + 1 else a) 0
      (fun _ x y => by cases x <;> cases y <;> rfl) current
    let reached := H.bkeys (H.bfilter count_per_key (fun e => decide (min ≤ e.2)))
    rebind (not_all := H.bantiJoin current reached)
    yield (out := reached);
  H.allTicks out

#print axioms toy_keyed
#check @toy_keyed_co_wf₁
#check @toy_keyed_param
-- key 1 reaches 2 votes across two ticks (once); key 2's single vote
-- stays persisted
#guard (toy_keyed (Values Unit (fun _ => 1)) () 2
    (fun _ => ({(1, true), (2, true), (1, true)} : Multiset (Nat × Bool)))
    (fun _ => [{(1, true), (2, true)}, {(1, true)}])) 0 = {1}

structure ToyKeyedInvEnsures {k : Nat} (o : Multiset Nat → Prop) : Prop where
  /-- Every emitted key is a key of some consumed response. -/
  keys_src : ∀ (_i : Fin k), True

-- the invariant clause on STREAM states (D65): quorum.rs's two
-- `state_null` registers, with the loop invariant "the window sits
-- below everything consumed so far, and the locked keys are keys of the
-- window" — the binders are typed at the `Values` carrier
-- (`Multiset …`), the seeds are the grade's bottom, the `tick`
-- obligation reads the step under `den` (the `ensures` is what makes the
-- obligations CHECK — ghosts replay in the proof leg; the content is
-- consumed by the ghost chain after the block)
hydro def toy_keyed_inv (H : HydroSem L mem) (ℓ : L) (min : Nat)
    (resp : H.Stream ℓ (Nat × Bool) .noOrder .exactlyOnce)
    (dec : H.BatchDec (mem ℓ) (Nat × Bool)) :
    H.Stream ℓ Nat .noOrder .exactlyOnce
  ensures out => ToyKeyedInvEnsures (k := mem ℓ) (fun m => ∀ i, out i = m) :=
  let new_inputs := H.batch resp dec
  tick (state not_all : H.BoundedStream (Nat × Bool) .noOrder .exactlyOnce)
      (state locked : H.BoundedStream Nat .noOrder .exactlyOnce)
      (input b := new_inputs)
      (invariant ((outv : List (Multiset Nat)) (not_all : Multiset (Nat × Bool))
          (locked : Multiset Nat) (b : Trace (Multiset (Nat × Bool)))) =>
        not_all ≤ (b.take outv.length).sum
          ∧ ∀ k ∈ locked, k ∈ ((b.take outv.length).sum).map Prod.fst) :=
    let current := H.bchain not_all b
    let count_per_key := H.bkeyedFold
      (fun (a : Nat) (v : Bool) => if v then a + 1 else a) 0
      (fun _ x y => by cases x <;> cases y <;> rfl) current
    let reached := H.bkeys (H.bfilter count_per_key (fun e => decide (min ≤ e.2)))
    rebind (not_all := H.bantiJoin current reached, locked := H.bchain locked reached)
    yield (out := reached)
      prove init := fun _i => by
          simp [ValuesTick.seed_pair, ValuesTick.seed_stream, den],
        tick := fun i n out st b_t hb hlen ih => by
          simp only [List.append_eq, List.length_append, List.length_singleton, hlen,
            List.take_add_one, hb, Option.toList_some, List.sum_append, List.sum_singleton] at ih ⊢
          simp only [out_step, den]
          obtain ⟨hwin, hlock⟩ := ih
          refine ⟨?_, fun k hk => ?_⟩
          · exact le_trans (Multiset.filter_le _ _) (add_le_add hwin le_rfl)
          · rcases Multiset.mem_add.mp hk with h | h
            · exact Multiset.mem_of_le (Multiset.map_le_map (Multiset.le_add_right _ _)) (hlock k h)
            · obtain ⟨e, he, rfl⟩ := Multiset.mem_map.mp h
              obtain ⟨he, -⟩ := Multiset.mem_filter.mp he
              obtain ⟨k', hk', rfl⟩ := Multiset.mem_map.mp he
              exact Multiset.mem_of_le (Multiset.map_le_map (add_le_add hwin le_rfl))
                (Multiset.mem_dedup.mp hk');
  -- the invariant read at every prefix through `_reg` (the register
  -- before tick `n`), on a stream state
  ghost obtain ⟨reg, hreg0, hout_at, hreg_succ, hreg_stall, hreg_inv⟩ := hout_reg
  ghost have hwin : ∀ i n, (reg i n).1 ≤ ((new_inputs i).take ((out i).take n).length).sum :=
    fun i n => (hreg_inv i n).1
  -- the locked keys before tick `n + 1` are keys of the consumed prefix
  ghost have hlocked : ∀ i n k, k ∈ (reg i n).2 →
      k ∈ (((new_inputs i).take ((out i).take n).length).sum).map Prod.fst :=
    fun i n k hk => (hreg_inv i n).2 k hk
  H.allTicks out
  prove
    keys_src := fun _ => trivial

#print axioms toy_keyed_inv
#guard (toy_keyed_inv (Values Unit (fun _ => 1)) () 2
    (fun _ => ({(1, true), (2, true), (1, true)} : Multiset (Nat × Bool)))
    (fun _ => [{(1, true), (2, true)}, {(1, true)}])) 0 = {1}

/-! ## In-tick operators (D60): the two representations, and their
coupling

One tick's bounded stream is the quotient at `Values` and the plain
`List` at `SchedSem`; the same in-tick expression computes on both,
and at the corner the per-operator coupling lemmas glue the legs. The
`#guard`s observe the two representations directly; the `example`s are
the corner's conditional agreements, instantiated. -/

section InTickCheck

-- a NoOrder batch: the denotation sees a multiset, the machine a list
example : @Eq Nat ((Values Unit (fun _ => 1)).bcount
    ((Values Unit (fun _ => 1)).bmap (ord := .noOrder)
      (({3, 1, 2} : Multiset Nat)) (· * 2))) 3 := by
  show Multiset.card (Multiset.map (· * 2) ({3, 1, 2} : Multiset Nat)) = 3
  simp
example : @Eq Nat ((SchedSem Unit (fun _ => 1) (fun _ _ _ => true)).bcount
    ((SchedSem Unit (fun _ => 1) (fun _ _ _ => true)).bmap (ord := .noOrder)
      [3, 1, 2] (· * 2))) 3 := rfl
-- the machine keeps the runtime's order; the type forbids observing it
example : ((SchedSem Unit (fun _ => 1) (fun _ _ _ => true)).bmap (ord := .noOrder)
    [3, 1, 2] (· * 2) : List Nat) = [6, 2, 4] := rfl
-- `enumerate` exists only at TotalOrder, where both legs are lists
example : ((Values Unit (fun _ => 1)).benumerate [10, 11] : List (Nat × Nat))
  = [(0, 10), (1, 11)] := rfl
example : ((SchedSem Unit (fun _ => 1) (fun _ _ _ => true)).benumerate [10, 11]
  : List (Nat × Nat)) = [(0, 10), (1, 11)] := rfl
-- `fold` at NoOrder demands commutativity (the quotient's own obligation)
example : @Eq Nat ((Values Unit (fun _ => 1)).bfold (ord := .noOrder) (ret := .exactlyOnce)
    (· + ·) 0 (fun _ _ _ => Nat.add_right_comm _ _ _) ({3, 1, 2} : Multiset Nat))
    6 := by
  show @Multiset.foldl Nat Nat (· + ·) _ 0 ({3, 1, 2} : Multiset Nat) = 6
  rfl

-- the corner: a coupled pair, and the conditional agreements
abbrev CS := CoupleSem Unit (fun _ => 1) (fun _ _ _ => true) 3 3 le_rfl

/-- A coupled NoOrder batch: list `[3,1,2]` against multiset `{3,1,2}`. -/
def cb : CS.BoundedStream Nat .noOrder .exactlyOnce :=
  { sr := [3, 1, 2], rr := ({3, 1, 2} : Multiset Nat), wf := True,
    cpl := fun _ => rfl }

-- `count` keeps both legs: the machine's length and the denotation's
-- card, coupled BECAUSE the pair is (the `wf` premise discharges `cpl`)
example : (CS.bcount cb).sr = (CS.bcount cb).rr := (CS.bcount cb).cpl trivial
example : (CS.bcount cb).rr = (Values Unit (fun _ => 1)).bcount cb.rr := rfl
-- the mapped pair stays coupled; its legs are the two `bmap`s
example : (CS.bmap cb (· * 2)).rr = (Values Unit (fun _ => 1)).bmap cb.rr (· * 2) := rfl
example : (CS.bmap cb (· * 2)).sr
    = (SchedSem Unit (fun _ => 1) (fun _ _ _ => true)).bmap (ord := .noOrder) cb.sr (· * 2) := rfl
example : (CS.bmap cb (· * 2)).wf → CoupledBatch .noOrder .exactlyOnce
    (CS.bmap cb (· * 2)).sr (CS.bmap cb (· * 2)).rr :=
  (CS.bmap cb (· * 2)).cpl

end InTickCheck

/-! ## The shaped tick former (D60, 0c): one former for every arity

`H.tick_scan ins outs` takes a tuple of wires (a shape) and a body over
this tick's bounded tuple, and returns a tuple of wires. The toy below
exercises it, hand-written (the `tick` construct will emit exactly
this), through the whole generated stack: the corner namings (`co_sr`/
`co_rr`), the corner's `wf` (the body's leg-independence conditions
W1–W3 discharged by the projection walk), machine causality,
denotational monotonicity, and the free theorem — whose `eagC`
instance IS the eager agreement (the body's in-tick ops are the
denotation's by definition). -/

section ShapedFormer

-- a singleton wire and an ordered tick stream in; the register trace
-- and the batch, bumped, out
hydro def toy_shape (H : HydroSem L mem) (ℓ : L)
    (inp : H.Ticked ℓ Nat)
    (bs : H.TickStream ℓ Nat .totalOrder .exactlyOnce) :
    H.Ticked ℓ Nat × H.TickStream ℓ Nat .totalOrder .exactlyOnce :=
  let r := H.tick_scan
    (.sing Nat)
    (.pair (.sing Nat) (.stream Nat inferInstance .totalOrder .exactlyOnce))
    (.pair (.sing Nat) (.stream Nat inferInstance .totalOrder .exactlyOnce))
    (inp, bs)
    (fun _i acc inpt =>
      let x := inpt.1
      let b := inpt.2
      let n := H.bcount b
      let next := H.bsMap (H.bsZip acc (H.bsMap (H.bsZip x n) (fun p => p.1 + p.2)))
        (fun p => p.1 + p.2)
      let out := H.bmap b (· + 1)
      (next, (next, out)))
    0
  (r.1, r.2)

#print axioms toy_shape
#check @toy_shape_co_sr₁
#check @toy_shape_co_rr₂
#check @toy_shape_co_wf₁
#check @toy_shape_causal₂
#check @toy_shape_mono₁
#check @toy_shape_param₁

-- the denotation: per tick, register += input + batch count; batch bumped
#guard (toy_shape (Values Unit (fun _ => 1)) () (fun _ => [1, 2, 3])
  (fun _ => [[10], [], [1, 2]])).1 0 = [2, 4, 9]
#guard (toy_shape (Values Unit (fun _ => 1)) () (fun _ => [1, 2, 3])
  (fun _ => [[10], [], [1, 2]])).2 0 = [[11], [], [2, 3]]
-- the machine, as of any step
example : (toy_shape (SchedSem Unit (fun _ => 1) (fun _ _ _ => true)) ()
    (fun _ _ => [1, 2, 3]) (fun _ _ => [[10], [], [1, 2]])).1 0 5 = [2, 4, 9] := rfl

-- eager agreement: the free theorem at `eagC`
example (ℓ : L) (inp : (Eager L mem).Ticked ℓ Nat)
    (bs : (Eager L mem).TickStream ℓ Nat .totalOrder .exactlyOnce) :
    (toy_shape (Eager L mem) ℓ inp bs).1.den
      = (toy_shape (Values L mem) ℓ inp.den bs.den).1 :=
  toy_shape_param₁ (eagC L mem) eagLaws () ℓ inp inp.den (fun _ _ => rfl)
    bs bs.den (fun _ _ => rfl) () le_rfl
example (ℓ : L) (inp : (Eager L mem).Ticked ℓ Nat)
    (bs : (Eager L mem).TickStream ℓ Nat .totalOrder .exactlyOnce) :
    (toy_shape (Eager L mem) ℓ inp bs).2.den
      = (toy_shape (Values L mem) ℓ inp.den bs.den).2 :=
  toy_shape_param₂ (eagC L mem) eagLaws () ℓ inp inp.den (fun _ _ => rfl)
    bs bs.den (fun _ _ => rfl) () le_rfl

end ShapedFormer

end Hydro
