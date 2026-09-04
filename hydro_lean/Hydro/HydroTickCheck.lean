import Hydro.HydroGenToy
import Hydro.HydroTick

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
    (inp : H.Atomic ℓ Nat) :
    H.Atomic ℓ Nat :=
  tick (state acc : Nat := 0)
      (input x := inp) :=
    let next := H.bsMap (H.bsZip acc x) (fun p => p.1 + p.2)
    rebind (acc := next)
    emit (outv := next);
  outv

#print axioms toy_tick
#guard (toy_tick (Values Unit (fun _ => 1)) () (.mk' fun _ => [1, 2, 3])) 0
  = [1, 3, 6]

-- `boOr` (Rust `Optional::or`, D89): the latest `some` seen so far — this tick's
-- optional, else the register
hydro def toy_bo_or (H : HydroSem L mem) (ℓ : L)
    (inp : H.Atomic ℓ (Option Nat)) :
    H.Atomic ℓ (Option Nat) :=
  tick (state last : Option Nat := none)
      (input x := inp) :=
    let cur := H.boOr x last
    rebind (last := cur)
    emit (outv := cur);
  outv

#print axioms toy_bo_or
#check @toy_bo_or_co_wf₁
#nondet_census toy_bo_or (nondets := 0) (scheds := 0) (fuels := 0)
#guard (toy_bo_or (Values Unit (fun _ => 1)) () (.mk' fun _ => [none, some 2, none, some 5, none])) 0
  = [none, some 2, some 2, some 5, some 5]

-- invariant + spectator (the input), consumed by a prove leg
hydro def toy_tick2 (H : HydroSem L mem) (ℓ : L)
    (inp : H.Atomic ℓ Nat) :
    (H.Atomic ℓ Nat)
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
  (.mk' fun _ => [1, 2, 3])) 0 = [1, 3, 6]

-- two states: productified by the construct (one `BoundedSingleton`
-- register of the pair, opened into the names by `bsMap`)
hydro def toy_tick3 (H : HydroSem L mem) (ℓ : L)
    (inp : H.Atomic ℓ Nat) :
    (H.Atomic ℓ Nat)
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
  (.mk' fun _ => [1, 2, 3])) 0 = [1, 3, 6]

-- two inputs of both kinds, a singleton emission and a stream
-- emission (`yield`): the construct zips the inputs at the TYPE level
hydro def toy_tick4 (H : HydroSem L mem) (ℓ : L)
    (inp : H.Atomic ℓ Nat)
    (bs : H.AtomicStream ℓ Nat .totalOrder .exactlyOnce) :
    H.Atomic ℓ Nat × H.AtomicStream ℓ Nat .totalOrder .exactlyOnce :=
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
#guard (toy_tick4 (Values Unit (fun _ => 1)) () (.mk' fun _ => [1, 2, 3])
  (.mk' fun _ => [[10], [], [1, 2]])).1 0 = [2, 4, 9]
#guard (toy_tick4 (Values Unit (fun _ => 1)) () (.mk' fun _ => [1, 2, 3])
  (.mk' fun _ => [[10], [], [1, 2]])).2 0 = [[11], [], [2, 3]]

structure ToyTick6Ensures {k : Nat} (o : MemberMap k (Trace (List Nat))) :
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
    (inp : H.Atomic ℓ Nat)
    (bs : H.AtomicStream ℓ Nat .totalOrder .exactlyOnce) :
    H.AtomicStream ℓ Nat .totalOrder .exactlyOnce
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
#guard (toy_tick6 (Values Unit (fun _ => 1)) () (.mk' fun _ => [1, 2, 3])
  (.mk' fun _ => [[10], [], [1, 2]])) 0 = [[11], [], [2, 3]]

-- a persisted stream state (Rust `use::state` on a `Stream<…, Tick,
-- Bounded>` — `quorum.rs` `state_null`): the register is a shape, seeded
-- empty; reads see everything chained in at EARLIER ticks
hydro def toy_tick5 (H : HydroSem L mem) (ℓ : L)
    (bs : H.AtomicStream ℓ Nat .totalOrder .exactlyOnce) :
    H.Atomic ℓ Nat :=
  tick (state seen : H.BoundedStream Nat .totalOrder .exactlyOnce)
      (input b := bs) :=
    rebind (seen := H.bchain seen b)
    emit (sofar := H.bcount seen);
  sofar

#print axioms toy_tick5
#guard (toy_tick5 (Values Unit (fun _ => 1)) ()
  (.mk' fun _ => [[10], [], [1, 2], [5]])) 0 = [0, 1, 1, 3]

-- a program-level `if` over a static parameter inside the body (Rust's
-- construction-time `if max == min { … } else { … }`, quorum.rs): one
-- term, both branches walked by every generator
hydro def toy_ite (H : HydroSem L mem) (ℓ : L) (n : Nat)
    (inp : H.Atomic ℓ Nat) :
    H.Atomic ℓ Nat :=
  tick (state acc : Nat := 0)
      (input x := inp) :=
    let next := if n = 0 then H.bsMap acc (· + 1)
      else H.bsMap (H.bsZip acc x) (fun p => p.1 + p.2)
    rebind (acc := next)
    emit (outv := next);
  outv

#print axioms toy_ite
#guard (toy_ite (Values Unit (fun _ => 1)) () 0 (.mk' fun _ => [1, 2, 3])) 0 = [1, 2, 3]
#guard (toy_ite (Values Unit (fun _ => 1)) () 1 (.mk' fun _ => [1, 2, 3])) 0 = [1, 3, 6]

-- the keyed vocabulary (quorum.rs's shape in miniature): a persisted
-- stream state, `chain`, `into_keyed().fold()`, `filter`/`keys`,
-- `anti_join`, a stream emission (`yield`) leaving through `all_ticks`
hydro def toy_keyed (H : HydroSem L mem) (ℓ : L) (min : Nat)
    (resp : H.Stream ℓ (Nat × Bool) .noOrder .exactlyOnce)
    (dec : H.BatchDec (mem ℓ) (Nat × Bool) .noOrder .exactlyOnce) :
    H.Stream ℓ Nat .noOrder .exactlyOnce :=
  let new_inputs := H.batch resp dec
  tick (state not_all : H.BoundedStream (Nat × Bool) .noOrder .exactlyOnce)
      (input b := new_inputs) :=
    let current := H.bchain not_all b
    let count_per_key := H.bkfold
      (fun (a : Nat) (v : Bool) => if v then a + 1 else a) 0 (fun _ _ => True)
      (fun _ x y _ => by cases x <;> cases y <;> rfl) (H.binto_keyed current)
    let reached := H.bksKeys (H.bksFilter count_per_key (fun c => decide (min ≤ c)))
    rebind (not_all := H.bantiJoin current reached)
    yield (out := reached);
  H.allTicks out

#print axioms toy_keyed
#check @toy_keyed_co_wf₁
#check @toy_keyed_param
-- key 1 reaches 2 votes across two ticks (once); key 2's single vote
-- stays persisted
#guard (toy_keyed (Values Unit (fun _ => 1)) () 2
    (.mk' fun _ => ({(1, true), (2, true), (1, true)} : Multiset (Nat × Bool)))
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
    (dec : H.BatchDec (mem ℓ) (Nat × Bool) .noOrder .exactlyOnce) :
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
    let count_per_key := H.bkfold
      (fun (a : Nat) (v : Bool) => if v then a + 1 else a) 0 (fun _ _ => True)
      (fun _ x y _ => by cases x <;> cases y <;> rfl) (H.binto_keyed current)
    let reached := H.bksKeys (H.bksFilter count_per_key (fun c => decide (min ≤ c)))
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
            · obtain ⟨hk', -⟩ := KeyedMap.mem_keysMS_filter.mp h
              rw [KeyedMap.keysMS_ofFold, KeyedPool.keys_ofMultiset] at hk'
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
    (.mk' fun _ => ({(1, true), (2, true), (1, true)} : Multiset (Nat × Bool)))
    (fun _ => [{(1, true), (2, true)}, {(1, true)}])) 0 = {1}

/-! ## The keyed operators (D70): the located chain and the atomic-trace
chain, through every walk

`toy_kmax` is paxos.rs's p1b shape (`into_keyed → assume_ordering →
fold_early_stop → get_max_key → snapshot`); `toy_wm` is the acceptor
log's (`across_ticks(|s| s.into_keyed().reduce_watermark(…))` — in Lean
the operators are invoked directly on the atomic wire, there being
nothing to pop out of — then this tick's entries read in the body as a
`.ksing` input). Neither is used by a program yet (the migrations await
the R3 ruling); these pin the engine. -/

-- let keyed = votes.into_keyed().assume_ordering::<TotalOrder>(nondet!(…));
-- let counts = keyed.fold_early_stop(q!(|| 0), q!(|acc, _v| { *acc += 1; *acc >= 2 }));
-- counts.get_max_key().snapshot(&tick, nondet!(…))
structure ToyKDec (H : HydroSem L mem) (n : Nat) where
  /-- `.assume_ordering::<TotalOrder>(nondet!(…))` -/
  order : H.OrderSelDec n (Nat × Nat)
  /-- `.get_max_key().snapshot(&tick, nondet!(…))` -/
  snap : H.SnapDec n (Nat × Nat) .noOrder

hydro def toy_kmax (H : HydroSem L mem) (ℓ : L)
    (votes : H.Stream ℓ (Nat × Nat) .noOrder .exactlyOnce)
    (dec : ToyKDec H (mem ℓ)) :
    H.Atomic ℓ (Option (Nat × Nat)) :=
  let keyed := H.kassume_ordering (H.into_keyed votes) dec.order
  let counts := H.kfold_early_stop keyed
    (fun (acc : Nat) (_v : Nat) => (acc + 1, decide (2 ≤ acc + 1))) 0
  H.snapshot (H.kget_max_key counts) dec.snap

#print axioms toy_kmax
#check @toy_kmax_co_sr₁
#check @toy_kmax_co_rr₁
#check @toy_kmax_co_wf₁
#check @toy_kmax_causal₁
#check @toy_kmax_mono₁
#check @toy_kmax_param
#nondet_census toy_kmax (nondets := 2) (scheds := 0) (fuels := 0)

-- key 1 reaches 2 votes (entry (1, 2)), key 2 never does; the snapshot
-- after the whole source is the max-key entry
#guard (toy_kmax (Values Unit (fun _ => 1)) ()
    (.mk' fun _ => ({(1, 5), (2, 7), (1, 6)} : Multiset (Nat × Nat)))
    ⟨fun _ => [(2, 7), (1, 5), (1, 6)],
     fun _ => [{(2, 7), (1, 5), (1, 6)}]⟩) 0 = [some (1, 2)]

-- let m = batched.across_ticks(|s| s.into_keyed().reduce_watermark(wm, q!(|a, c| *a += c), commutative = …));
-- tick: m.entries()
hydro def toy_wm (H : HydroSem L mem) (ℓ : L)
    (inp : H.Stream ℓ (Nat × Nat) .noOrder .exactlyOnce)
    (wm : H.Atomic ℓ (Option Nat))
    (dec : H.BatchDec (mem ℓ) (Nat × Nat) .noOrder .exactlyOnce) :
    H.Stream ℓ (Nat × Nat) .noOrder .exactlyOnce :=
  let batched := H.batch inp dec
  let m := H.reduce_watermark (H.tinto_keyed batched) wm (fun a c => a + c) (fun _ _ => True)
    (fun s x y _ => by cases s <;> simp [reduceStep, Nat.add_comm, Nat.add_left_comm])
  tick (input mm := m) :=
    yield (out := H.bksEntries mm);
  H.allTicks out

#print axioms toy_wm
#check @toy_wm_co_sr₁
#check @toy_wm_co_rr₁
#check @toy_wm_co_wf₁
#check @toy_wm_causal₁
#check @toy_wm_mono₁
#check @toy_wm_param
#nondet_census toy_wm (nondets := 1) (scheds := 0) (fuels := 0)

-- two ticks: {1 ↦ 5, 2 ↦ 7} then +{1 ↦ 6} with the watermark rising to 2
-- (key 1 collected): the entries read per tick, summed
#guard (toy_wm (Values Unit (fun _ => 1)) ()
    (.mk' fun _ => ({(1, 5), (2, 7), (1, 6)} : Multiset (Nat × Nat)))
    (.mk' fun _ => [none, some 2])
    (fun _ => [{(1, 5), (2, 7)}, {(1, 6)}])) 0 = {(1, 5), (2, 7), (2, 7)}

/-! ## Conditional commutativity (D71, R3): `requires` + `ghost pre`

`toy_pre` is the acceptor log's merge in miniature: a `reduce_watermark`
whose combiner (`max` by the first component, Rust's `>`) commutes only
on pairs that do not share a first component with different second
components — the `commutative = manual_proof!(requires: …)` site. The
closure's commutativity under `Pre` is a lemma passed to the operator;
the content half — every two values meeting at a key satisfy `Pre` — is
the module's `requires` clause on its input, proven at the site by
`ghost pre` BEFORE the operator, and consumed by the generated
`toy_pre_co_wf₁ … (hreq : …)`. `toy_pre_caller` discharges that clause
for a concrete input through its own `ghost pre` (a callee's `requires`
is closed the same way a fold's pairwise conjunct is). -/

/-- The log-merge tie: same first component ⇒ same second (the `assume`
paxos.rs:862's `TODO` asks for). -/
abbrev ToyTie (x y : Nat × Nat) : Prop := x.1 = y.1 → x.2 = y.2

/-- Rust's `if entry.ballot > prev.ballot { *prev = entry }`. -/
def toyMerge (prev entry : Nat × Nat) : Nat × Nat :=
  if prev.1 < entry.1 then entry else prev

/-- The literal closure commutes under the tie prerequisite. -/
theorem toyMerge_comm (s : Option (Nat × Nat)) (x y : Nat × Nat) (h : ToyTie x y) :
    reduceStep toyMerge (reduceStep toyMerge s x) y
      = reduceStep toyMerge (reduceStep toyMerge s y) x := by
  obtain ⟨xb, xv⟩ := x
  obtain ⟨yb, yv⟩ := y
  simp only [ToyTie] at h
  cases s with
  | none =>
    show some (toyMerge (xb, xv) (yb, yv)) = some (toyMerge (yb, yv) (xb, xv))
    unfold toyMerge
    rcases Nat.lt_trichotomy xb yb with hlt | heq | hgt
    · simp [hlt, Nat.lt_asymm hlt]
    · subst heq; simp [h rfl]
    · simp [hgt, Nat.lt_asymm hgt]
  | some p =>
    obtain ⟨pb, pv⟩ := p
    show some (toyMerge (toyMerge (pb, pv) (xb, xv)) (yb, yv))
      = some (toyMerge (toyMerge (pb, pv) (yb, yv)) (xb, xv))
    unfold toyMerge
    rcases Nat.lt_trichotomy xb yb with hlt | heq | hgt
    · by_cases h1 : pb < xb <;> by_cases h2 : pb < yb <;> simp [h1, h2, hlt, Nat.lt_asymm hlt] <;> omega
    · subst heq
      have := h rfl; subst this
      by_cases h1 : pb < xb <;> simp [h1]
    · by_cases h1 : pb < xb <;> by_cases h2 : pb < yb <;> simp [h1, h2, hgt, Nat.lt_asymm hgt] <;> omega

/-- The toy's face: under the clause, the pairwise fact on the batched
content (the `ghost pre` fact, re-exported — a face about the INPUTS). -/
structure ToyPreEnsures {n : Nat} (inp : MemberMap n (Multiset (Nat × (Nat × Nat))))
    (dec : Fin n → List (Multiset (Nat × (Nat × Nat)))) : Prop where
  pre_holds : (∀ i, ∀ a ∈ inp i, ∀ b ∈ inp i, a.1 = b.1 → ToyTie a.2 b.2) →
    ∀ i, (KeyedPool.accum ((batchCuts (inp i) 0 (dec i)).map
      (KeyedPool.ofPool (ord := .noOrder)))).Pairwise ToyTie

-- let m = batched.across_ticks(|s| s.into_keyed().reduce_watermark(wm,
--   q!(|prev, e| if e.0 > prev.0 { *prev = e }),
--   commutative = manual_proof!(requires: same key ∧ same ballot ⇒ same value)));
-- tick: m.entries()
hydro def toy_pre (H : HydroSem L mem) (ℓ : L)
    (inp : H.Stream ℓ (Nat × (Nat × Nat)) .noOrder .exactlyOnce)
    (wm : H.Atomic ℓ (Option Nat))
    (dec : H.BatchDec (mem ℓ) (Nat × (Nat × Nat)) .noOrder .exactlyOnce) :
    H.Stream ℓ (Nat × (Nat × Nat)) .noOrder .exactlyOnce
  requires (∀ i, ∀ a ∈ inp i, ∀ b ∈ inp i, a.1 = b.1 → ToyTie a.2 b.2)
  ensures _out => ToyPreEnsures inp dec :=
  let batched := H.batch inp dec
  let keyed := H.tinto_keyed batched
  -- the site fact, before the operator: the accumulated content at every
  -- key satisfies the tie (from the module's `requires`)
  ghost pre hpre : ∀ i, (KeyedPool.accum (keyed i)).Pairwise ToyTie :=
    fun i => KeyedPool.accum_tinto_keyed_batch_pairwise inp dec i (hreq i)
  let m := H.reduce_watermark keyed wm toyMerge ToyTie toyMerge_comm
  tick (input mm := m) :=
    yield (out := H.bksEntries mm);
  H.allTicks out
  prove pre_holds := fun hR => hpre hR

#print axioms toy_pre
#check @toy_pre_co_wf₁
#check @toy_pre.pre.hpre
#print axioms toy_pre_co_wf₁
#nondet_census toy_pre (nondets := 1) (scheds := 0) (fuels := 0)

-- the merge is `max` by the first component on coherent content; on a
-- violating input (same key, same first component, different seconds)
-- the refused-content branch yields the seed — the key vanishes (the
-- executable meaning of an unjustified `manual_proof!`; unreachable in
-- a compiled `hydro def`, whose `ghost pre` is mandatory)
#guard (toy_pre (Values Unit (fun _ => 1)) ()
    (.mk' fun _ => ({(1, (5, 100)), (1, (7, 200)), (2, (3, 300))} : Multiset (Nat × (Nat × Nat))))
    (.mk' fun _ => [none])
    (fun _ => [{(1, (5, 100)), (1, (7, 200)), (2, (3, 300))}])) 0
  = {(1, (7, 200)), (2, (3, 300))}
#guard (toy_pre (Values Unit (fun _ => 1)) ()
    (.mk' fun _ => ({(1, (5, 100)), (1, (5, 200))} : Multiset (Nat × (Nat × Nat))))
    (.mk' fun _ => [none])
    (fun _ => [{(1, (5, 100)), (1, (5, 200))}])) 0
  = {}

/-- A trivial face (the caller toy exercises the `requires` discharge,
not a contract). -/
structure ToyTrivialEnsures : Prop where
  ok : True

-- the acceptor log's tail: the live map's entries, folded into the
-- canonical association list inside the tick —
--   m.entries().fold(q!(HashMap::new), q!(insert,
--     commutative = manual_proof!(/** no overlapping keys */)))
-- the fold's `Pre` (`NoKeyOverlap`) is structural for a map's entries:
-- the wf normalizer closes it (`co_bksEntries_rr_pairwise_key`), no
-- `ghost pre`
hydro def toy_wm_fold (H : HydroSem L mem) (ℓ : L)
    (inp : H.Stream ℓ (Nat × Nat) .noOrder .exactlyOnce)
    (wm : H.Atomic ℓ (Option Nat))
    (dec : H.BatchDec (mem ℓ) (Nat × Nat) .noOrder .exactlyOnce) :
    H.Atomic ℓ (List (Nat × Nat))
  ensures _out => ToyTrivialEnsures :=
  let batched := H.batch inp dec
  let m := H.reduce_watermark (H.tinto_keyed batched) wm (fun a c => a + c) (fun _ _ => True)
    (fun s x y _ => by cases s <;> simp [reduceStep, Nat.add_comm, Nat.add_left_comm])
  tick (input mm := m) :=
    emit (out := H.bfold insertSorted [] NoKeyOverlap insertSorted_foldOk (H.bksEntries mm));
  out
  prove ok := trivial

#print axioms toy_wm_fold
#check @toy_wm_fold_co_wf₁
#nondet_census toy_wm_fold (nondets := 1) (scheds := 0) (fuels := 0)

-- two ticks: {1 ↦ 5, 2 ↦ 7} then +{1 ↦ 6} with the watermark rising to 2
-- (key 1 collected): the sorted association list per tick
#guard (toy_wm_fold (Values Unit (fun _ => 1)) ()
    (.mk' fun _ => ({(1, 5), (2, 7), (1, 6)} : Multiset (Nat × Nat)))
    (.mk' fun _ => [none, some 2])
    (fun _ => [{(1, 5), (2, 7)}, {(1, 6)}])) 0 = [[(1, 5), (2, 7)], [(2, 7)]]

/-! ## The bench client's tick API (D81): every new in-tick operator,
through every walk

`toy_bench_ops` strings the operators `hydro_std/src/bench_client/mod.rs`
and `hydro_test/src/cluster/{paxos_bench,kv_replica}.rs` need and no
program used before — the `Optional`/`Singleton` API (`zip`,
`filter_if`, `filter_map`, `into_stream`, `is_none`), the stream
`reduce`/`min`/`sort`/`cross_singleton(optional)`, the in-tick keyed
stream (`map`, `cross_singleton`, `chain`, `weaken_ordering`, `reduce`)
and keyed singleton (`join_keyed_singleton`, `filter_key_not_in`,
`key_count`, `into_keyed_stream`) — in one tick body, so the corner's
per-op couplings and the two law instances are exercised together. -/

/-- `reduce` by `+` commutes (the unconditional obligation). -/
theorem toyAdd_reduceOk : FoldOkP .noOrder .exactlyOnce (fun (_ _ : Nat) => True)
    (reduceStep (fun a c : Nat => a + c)) :=
  fun s x y _ => by cases s <;> simp [reduceStep, Nat.add_comm, Nat.add_left_comm]

/-- `reduce` by `max` commutes. -/
theorem toyMax_reduceOk : FoldOkP .noOrder .exactlyOnce (fun (_ _ : Nat) => True)
    (reduceStep (fun a c : Nat => max a c)) :=
  fun s x y _ => by cases s <;> simp [reduceStep, max_comm, max_left_comm]

/-- `reduce` by componentwise `max` commutes. -/
theorem toyMaxPair_reduceOk : FoldOkP .noOrder .exactlyOnce (fun (_ _ : Nat × Nat) => True)
    (reduceStep (fun a c : Nat × Nat => (max a.1 c.1, max a.2 c.2))) :=
  fun s x y _ => by cases s <;> simp [reduceStep, max_comm, max_left_comm]

hydro def toy_bench_ops (H : HydroSem L mem) (ℓ : L)
    (inp : H.Stream ℓ (Nat × Nat) .noOrder .exactlyOnce)
    (dec : H.BatchDec (mem ℓ) (Nat × Nat) .noOrder .exactlyOnce) :
    H.Stream ℓ ((Nat × (Nat × Nat)) × Nat) .noOrder .exactlyOnce :=
  let b := H.batch inp dec
  tick (state acc : Option Nat := none) (input xs := b) :=
    -- let ks = xs.into_keyed();
    let ks := H.binto_keyed xs
    -- let sums = ks.clone().reduce(q!(|a, c| *a += c), commutative = …);
    let sums := H.bkreduce (fun a c => a + c) (fun _ _ => True) toyAdd_reduceOk ks
    -- let cnt = sums.clone().key_count();
    let cnt := H.bksKeyCount sums
    -- let gate = cnt.clone().filter_if(cnt.clone().map(q!(|n| n > 0)));
    let gate := H.bsFilterIf cnt (H.bsMap cnt (fun n => decide (0 < n)))
    -- let mn = xs.clone().map(q!(|(_, v)| v)).min();
    let mn := H.bmin (H.bmap xs Prod.snd)
    -- let both = gate.zip(mn);
    let both := H.boZip gate mn
    -- let stamped = ks.clone().cross_singleton(cnt);
    let stamped := H.bkcrossSingleton ks cnt
    -- let twice = stamped.map(q!(|(v, c)| v + c)).chain(ks.map(q!(|v| v))).weaken_ordering();
    let twice := H.bkweakenOrder (H.bkchain (H.bkmap stamped (fun p => p.1 + p.2)) (H.bkmap ks id))
    -- let mx = twice.reduce(q!(|a, c| *a = max(*a, c)), commutative = …);
    let mx := H.bkreduce (fun a c => max a c) (fun _ _ => True) toyMax_reduceOk twice
    -- let j = sums.join_keyed_singleton(mx);
    let j := H.bksJoin sums mx
    -- let dropped = j.filter_key_not_in(both.map(q!(|(c, _)| c)).into_stream());
    let dropped := H.bksFilterKeyNotIn j (H.boIntoStream (H.boMap both Prod.fst))
    -- let sorted = xs.clone().map(q!(|(k, _)| k)).sort();
    let sorted := H.bsort (H.bmap xs Prod.fst)
    -- let crossed = dropped.clone().into_keyed_stream().weaken_ordering()
    --   .reduce(…).entries().cross_singleton(sorted.first());
    let dropped' := H.bkreduce (fun a c => (max a.1 c.1, max a.2 c.2)) (fun _ _ => True)
      toyMaxPair_reduceOk (H.bkweakenOrder (H.bksIntoStream dropped))
    let crossed := H.bcrossOptional (H.bksEntries dropped') (H.bfirst sorted)
    -- let red = xs.map(q!(|(_, v)| v)).reduce(q!(|a, c| *a += c), commutative = …);
    let red := H.breduce (fun a c => a + c) (fun _ _ => True) toyAdd_reduceOk (H.bmap xs Prod.snd)
    -- let nxt = red.zip(acc.filter_map(q!(|o| o))).filter_map(q!(|(r, a)| Some(r + a)));
    let nxt := H.boFilterMap (H.boZip red (H.bsFilterMap acc id)) (fun p => some (p.1 + p.2))
    -- let out = crossed.filter_if(nxt.is_none());
    let out := H.bfilterIf crossed (H.boIsNone nxt)
    rebind (acc := H.boUnwrapOr (H.boMap nxt some) red)
    yield (outs := out);
  H.allTicks outs

#print axioms toy_bench_ops
#check @toy_bench_ops_co_sr₁
#check @toy_bench_ops_co_rr₁
#check @toy_bench_ops_co_wf₁
#check @toy_bench_ops_causal₁
#check @toy_bench_ops_mono₁
#check @toy_bench_ops_param
#nondet_census toy_bench_ops (nondets := 1) (scheds := 0) (fuels := 0)

-- one tick with {(1,5),(2,7),(1,6)}: sums {1↦11, 2↦7}, cnt 2, gate some 2,
-- mn some 5, both some (2,5); stamped {1↦(5,2),(6,2); 2↦(7,2)}, twice
-- {1↦{7,8,5,6}, 2↦{9,7}}, mx {1↦8, 2↦9}, j {1↦(11,8), 2↦(7,9)}; dropped
-- removes key 2 (= both.1) → {1↦(11,8)}; crossed with first sorted key 1;
-- acc none → nxt none → out kept
#guard (toy_bench_ops (Values Unit (fun _ => 1)) ()
    (.mk' fun _ => ({(1, 5), (2, 7), (1, 6)} : Multiset (Nat × Nat)))
    (fun _ => [{(1, 5), (2, 7), (1, 6)}])) 0 = {((1, (11, 8)), 1)}
-- the machine agrees, as of any step
#guard ((toy_bench_ops (SchedSem Unit (fun _ => 1) (Pacing.ofTicks fun _ _ _ => true)) ()
    (fun _ => StepHist.const [(1, 5), (2, 7), (1, 6)]) ()) 0).view 3
    = [((1, (11, 8)), 1)]

/-- The `.kstream` leaf (D81): a keyed batch as a tick INPUT, a keyed
stream as a `use::state_null` REGISTER, and a keyed emission (`kyield`)
— Rust's `KeyedStream<…, Tick<L>, Bounded>` at every position of a
`sliced!`. The register carries each tick's batch into the next
(`carried`), so tick `n` sums ticks `n-1` and `n`. -/
hydro def toy_kleaf (H : HydroSem L mem) (ℓ : L)
    (inp : H.Stream ℓ (Nat × Nat) .noOrder .exactlyOnce)
    (dec : H.BatchDec (mem ℓ) (Nat × Nat) .noOrder .exactlyOnce) :
    H.Stream ℓ (Nat × Nat) .noOrder .exactlyOnce :=
  let kb := H.tinto_keyed (H.batch inp dec)
  tick (state carried : H.BoundedKeyedStream Nat Nat .noOrder .exactlyOnce)
      (input ks := kb) :=
    -- let all = carried.chain(ks.clone());
    let all := H.bkchain carried ks
    -- let sums = all.clone().reduce(q!(|a, c| *a += c), commutative = …);
    let sums := H.bkreduce (fun a c => a + c) (fun _ _ => True) toyAdd_reduceOk all
    rebind (carried := H.bkmap ks id)
    yield (out := H.bksEntries sums)
    kyield (kout := all);
  H.allTicks out

#print axioms toy_kleaf
#check @toy_kleaf_co_sr₁
#check @toy_kleaf_co_rr₁
#check @toy_kleaf_co_wf₁
#check @toy_kleaf_causal₁
#check @toy_kleaf_mono₁
#check @toy_kleaf_param
#nondet_census toy_kleaf (nondets := 1) (scheds := 0) (fuels := 0)

-- two ticks: {(1,5),(2,7)} then {(1,6)}: tick 0 sums {1↦5, 2↦7}; tick 1
-- sums the carried batch and the new one, {1↦11, 2↦7}
#guard (toy_kleaf (Values Unit (fun _ => 1)) ()
    (.mk' fun _ => ({(1, 5), (2, 7), (1, 6)} : Multiset (Nat × Nat)))
    (fun _ => [{(1, 5), (2, 7)}, {(1, 6)}])) 0 = {(1, 5), (2, 7), (1, 11), (2, 7)}

-- a caller discharges the callee's clause for its input: a `map` that
-- forces the tie (the second component is a function of the first)
hydro def toy_pre_caller (H : HydroSem L mem) (ℓ : L)
    (src : H.Stream ℓ (Nat × Nat) .noOrder .exactlyOnce)
    (wm : H.Atomic ℓ (Option Nat))
    (dec : H.BatchDec (mem ℓ) (Nat × (Nat × Nat)) .noOrder .exactlyOnce) :
    H.Stream ℓ (Nat × (Nat × Nat)) .noOrder .exactlyOnce
  ensures _out => ToyTrivialEnsures :=
  let inp := H.map src (fun _me kb => (kb.1, (kb.2, kb.2 * 2)))
  -- the callee's `requires`, proven for THIS input (before the call)
  ghost pre hinp : ∀ i, ∀ a ∈ inp i, ∀ b ∈ inp i, a.1 = b.1 → ToyTie a.2 b.2 := by
    intro i a ha b hb _ hab
    simp only [inp, den] at ha hb
    obtain ⟨a', -, rfl⟩ := Multiset.mem_map.mp ha
    obtain ⟨b', -, rfl⟩ := Multiset.mem_map.mp hb
    simp only at hab ⊢
    omega
  toy_pre H ℓ inp wm dec
  prove ok := trivial

#print axioms toy_pre_caller
#check @toy_pre_caller_co_wf₁
#print axioms toy_pre_caller_co_wf₁

/-- The knot toy's decisions (house style: the fuel travels in a record). -/
structure ToyPreLoopDec (H : HydroSem L mem) (n : Nat) where
  batch : H.BatchDec n (Nat × (Nat × Nat)) .noOrder .exactlyOnce
  fuel : H.FixDec

-- a `requires` on a plain binder of a module with a knot: the body's
-- callee clause is discharged under it (paxos_core's shape: `variant =
-- .guarded`); the knot's `K_co_wf₁` carries `hreq` and passes it to the
-- body module's wf lemma
hydro def toy_pre_loop (H : HydroSem L mem) (ℓ : L)
    (src : H.Stream ℓ (Nat × Nat) .noOrder .exactlyOnce)
    (wm : H.Atomic ℓ (Option Nat)) (flag : Bool)
    (tdec : ToyPreLoopDec H (mem ℓ)) :
    H.Stream ℓ (Nat × (Nat × Nat)) .noOrder .exactlyOnce
  requires (flag = true)
  ensures _out => ToyTrivialEnsures :=
  fix (w : H.Stream ℓ (Nat × (Nat × Nat)) .noOrder .exactlyOnce) via tdec.fuel :=
    let inp := H.map src (fun _me kb =>
      if flag then (kb.1, (kb.2, kb.2 * 2)) else (kb.1, (kb.2, kb.1)))
    -- the callee's clause, under the module's own (`flag = true`)
    ghost pre hinp : ∀ i, ∀ a ∈ inp i, ∀ b ∈ inp i, a.1 = b.1 → ToyTie a.2 b.2 := by
      intro i a ha b hb _ hab
      simp only [inp, den, hreq, if_true] at ha hb
      obtain ⟨a', -, rfl⟩ := Multiset.mem_map.mp ha
      obtain ⟨b', -, rfl⟩ := Multiset.mem_map.mp hb
      simp only at hab ⊢
      omega
    let out := H.union (toy_pre H ℓ inp wm tdec.batch) w
    complete (out)
    out
  prove ok := trivial

#print axioms toy_pre_loop
#check @toy_pre_loop_co_wf₁
#print axioms toy_pre_loop_co_wf₁

/-! ## The bench client's stream-level vocabulary (D81, Stage B)

`kmap`/`kentries`/`kfold`/`ksnapshot`/`kallTicks` (the keyed wires),
`tfold` (`across_ticks(|s| s.fold(…))`), `time`/`snapshot_bounded` (the
two new `sliced!` sources with their decisions), `latest_monotone` (the
`Monotonic` read of a tick optional, under a `requires` the `ghost pre`
re-exports), and the two in-tick zips — one toy, every phase, one
`#guard`. -/

theorem toyMaxFold_ok : FoldOkP .noOrder .exactlyOnce (fun (_ _ : Nat) => True)
    (fun a v : Nat => max a v) :=
  fun s x y _ => by simp [max_comm, max_left_comm]

theorem toyAddSnd_ok : FoldOkP .noOrder .exactlyOnce (fun (_ _ : Nat × Nat) => True)
    (fun (s : Nat) (p : Nat × Nat) => s + p.2) :=
  fun s x y _ => by simp [Nat.add_right_comm]

structure ToyWiresDec (H : HydroSem L mem) (n : Nat) where
  /-- `use::snapshot(bounded, nondet!(…))` -/
  ticks : H.TicksDec n
  /-- `use::time(nondet!(…))` -/
  clock : H.ClockDec n
  /-- `use::snapshot(maxes, nondet!(…))` -/
  snap : H.SnapDec n (Nat × Nat) .noOrder
  /-- `use::batch(pairs, nondet!(…))` -/
  batch : H.BatchDec n (Nat × Nat) .noOrder .exactlyOnce
  /-- `use::snapshot(lat, nondet!(…))` -/
  lsnap : H.SnapDec n (Option Nat) .totalOrder

hydro def toy_bench_wires (H : HydroSem L mem) (ℓ : L)
    (inp : H.KeyedStream ℓ Nat Nat .noOrder .exactlyOnce)
    (src : H.Atomic ℓ (Option Nat))
    (dec : ToyWiresDec H (mem ℓ)) :
    (H.KeyedStream ℓ Nat Nat .noOrder .exactlyOnce
      × H.Stream ℓ (Nat × Option Nat) .totalOrder .exactlyOnce)
  requires (∀ i, Ascending ValueOrder.optNat (src i))
  ensures _out => ToyTrivialEnsures :=
  -- let doubled = inp.map(q!(|v| v * 2));
  let doubled := H.kmap inp (fun _me v => v * 2)
  -- let pairs = doubled.entries();
  let pairs := H.kentries doubled
  -- let maxes = inp.reduce(q!(|a, v| *a = max(*a, v)), commutative = …);
  let maxes := H.kfold (fun a v => max a v) 0 (fun _ _ => True) toyMaxFold_ok inp
  -- let lat = src.latest();  — read as `Monotonic`: its latests ascend (the clause)
  ghost pre hsrc : ∀ i, Ascending ValueOrder.optNat (src i) := hreq
  let lat := H.latest_monotone src
  let b := H.batch pairs dec.batch
  -- let run = b.across_ticks(|s| s.fold(q!(|| 0), q!(|s, (_, v)| *s += v)));
  let run := H.tfold (fun s p => s + p.2) 0 (fun _ _ => True) toyAddSnd_ok b
  tick (state seen : Nat := 0)
      (input now := H.time dec.clock)
      (input lim := H.snapshot_bounded 3 dec.ticks)
      (input ms := H.ksnapshot maxes dec.snap)
      (input ps := b)
      (input lt := H.snapshot lat dec.lsnap) :=
    let cnt := H.bksKeyCount ms
    let o := H.bsFilterIf now (H.bsMap lim (fun l => decide (0 < l)))
    let z1 := H.boZipSingleton o cnt
    let z2 := H.bsZipOptional cnt lt
    let total := H.bsMap (H.bsZip (H.boMap z1 (fun p => p.1 + p.2)) (H.boMap z2 (fun p => p.1 + p.2)))
      (fun p => p.1.getD 0 + p.2.getD 0)
    rebind (seen := H.bsMap (H.bsZip seen total) (fun p => p.1 + p.2))
    emit (acc := H.bsZip seen lt)
    kyield (kd := H.binto_keyed ps);
  (H.kallTicks kd,
   H.allTicks (H.flattenOrdered (H.mapTick (H.zipTick acc run)
     (fun _me p => [(p.1.1 + p.2, p.1.2)]))))
  prove ok := trivial

#print axioms toy_bench_wires
#check @toy_bench_wires_co_sr₁
#check @toy_bench_wires_co_rr₁
#check @toy_bench_wires_co_wf₁
#check @toy_bench_wires_causal₁
#check @toy_bench_wires_mono₁
#check @toy_bench_wires_param₁
#nondet_census toy_bench_wires (nondets := 5) (scheds := 0) (fuels := 0)

-- two ticks: views {1↦5,2↦7} then {1↦6,2↦7} (cnt 2 both), clock 10/20,
-- three bounded ticks, latest none then some 3: tick 0 total 12, acc (0,
-- none), run 24; tick 1 total 27, acc (12, some 3), run 36
#guard ((toy_bench_wires (Values Unit (fun _ => 1)) ()
    (.mk' fun _ => KeyedPool.ofMultiset ({(1, 5), (2, 7), (1, 6)} : Multiset (Nat × Nat)))
    (.mk' fun _ => [none, some 3, some 3])
    ⟨fun _ => 3, fun _ => [10, 20], fun _ => [{(1, 5), (2, 7)}, {(1, 6)}],
     fun _ => [{(1, 10), (2, 14)}, {(1, 12)}], fun _ => [1, 1]⟩).2 0)
  = [(24, none), (48, some 3)]
#guard ((toy_bench_wires (Values Unit (fun _ => 1)) ()
    (.mk' fun _ => KeyedPool.ofMultiset ({(1, 5), (2, 7), (1, 6)} : Multiset (Nat × Nat)))
    (.mk' fun _ => [none, some 3, some 3])
    ⟨fun _ => 3, fun _ => [10, 20], fun _ => [{(1, 5), (2, 7)}, {(1, 6)}],
     fun _ => [{(1, 10), (2, 14)}, {(1, 12)}], fun _ => [1, 1]⟩).1 0).toMultiset
  = {(1, 10), (2, 14), (1, 12)}

/-! ### The assumed `invariant` (D71): a loop invariant on a `fix` wire

A conditional fold inside a knot body whose prerequisite depends on the
LOOP wire's content: no `ghost pre` can prove it from the body alone
(the wire's content is the knot's own output), so the `fix` carries a
Verus-style loop invariant — `invariant w => I w, base, step` — that
the body assumes (`hw_inv` in its `ghost pre` proofs), the knot
induction discharges at every Kleene stage, and the knot `wf` feeds to
the body at exactly the stage it probes. The `step` is stated at the
program's own chain `let`s (`out` listed for the proof). -/

-- the recirculated entries re-enter the merge: the tie on the loop wire
-- is a loop invariant (here a shape: the second component is a function
-- of the first), not a site fact
hydro def toy_pre_knot (H : HydroSem L mem) (ℓ : L)
    (src : H.Stream ℓ (Nat × Nat) .noOrder .exactlyOnce)
    (wm : H.Atomic ℓ (Option Nat))
    (tdec : ToyPreLoopDec H (mem ℓ)) :
    H.Stream ℓ (Nat × (Nat × Nat)) .noOrder .exactlyOnce
  ensures _out => ToyTrivialEnsures :=
  fix (w : H.Stream ℓ (Nat × (Nat × Nat)) .noOrder .exactlyOnce) via tdec.fuel
    invariant w (out) => (∀ i, ∀ a ∈ w i, a.2.2 = a.2.1 * 2),
      base := by
        intro _w' _out i a ha
        exact absurd ha (Multiset.notMem_zero a),
      step := by
        intro _w' out _out_step _ih i a ha
        simp only [out, den] at ha
        obtain ⟨a', -, rfl⟩ := Multiset.mem_map.mp ha
        rfl :=
    let inp := H.map src (fun _me kb => (kb.1, (kb.2, kb.2 * 2)))
    let fed := H.union inp w
    let keyed := H.tinto_keyed (H.batch fed tdec.batch)
    -- the merge's pairwise prerequisite on the fed content: the fresh
    -- half by the map's shape, the recirculated half by the invariant
    ghost pre hpre : ∀ i, (KeyedPool.accum (keyed i)).Pairwise ToyTie := by
      intro i
      refine KeyedPool.accum_tinto_keyed_batch_pairwise fed tdec.batch i ?_
      intro a ha b hb _ hab
      simp only [fed, den, Multiset.mem_add] at ha hb
      have hshape : ∀ c ∈ inp i, c.2.2 = c.2.1 * 2 := by
        intro c hc
        simp only [inp, den] at hc
        obtain ⟨c', -, rfl⟩ := Multiset.mem_map.mp hc
        rfl
      have ha' : a.2.2 = a.2.1 * 2 := ha.elim (hshape a) (hw_inv i a)
      have hb' : b.2.2 = b.2.1 * 2 := hb.elim (hshape b) (hw_inv i b)
      omega
    let m := H.reduce_watermark keyed wm toyMerge ToyTie toyMerge_comm
    tick (input mm := m) :=
      yield (ents := H.bksEntries mm);
    let out := H.map (H.allTicks ents) (fun _me e => (e.1, (e.2.1, e.2.1 * 2)))
    complete (out)
    out
  prove ok := trivial

#print axioms toy_pre_knot
#check @toy_pre_knot_co_wf₁
#check @toy_pre_knot.w.inv_stages
#check @toy_pre_knot.w.inv
#print axioms toy_pre_knot_co_wf₁

/-- The tick-knot toy's decisions. -/
structure ToyTickKnotDec (H : HydroSem L mem) where
  fuel : H.FixDec

-- the same clause on a tick (`Atomic`) wire: the body assumes the
-- invariant of its loop input (`hw_inv`), the knot induction runs on the
-- prefix chain (seed `[]`)
hydro def toy_inv_tick (H : HydroSem L mem) (ℓ : L)
    (src : H.Atomic ℓ Nat) (tdec : ToyTickKnotDec H) :
    H.Atomic ℓ Nat
  ensures _out => ToyTrivialEnsures :=
  fix (w : H.Atomic ℓ Nat) via tdec.fuel
    invariant w (out) => (∀ i, ∀ x ∈ w i, x % 2 = 0),
      base := by
        intro _w' _out i x hx
        exact absurd hx (List.not_mem_nil),
      step := by
        intro _w' out _out_step _ih i x hx
        simp only [out, den] at hx
        obtain ⟨p, -, rfl⟩ := List.mem_map.mp hx
        omega :=
    let prev := H.defer_tick 0 w
    -- a site fact under the invariant (every recirculated value is even)
    ghost pre hprev : ∀ i, ∀ x ∈ w i, x % 2 = 0 := hw_inv
    let out := H.mapTick (H.zipTick src prev) (fun _me p => 2 * (p.1 + p.2))
    complete (out)
    out
  prove ok := trivial

#print axioms toy_inv_tick
#check @toy_inv_tick_co_wf₁
#check @toy_inv_tick.w.inv
#print axioms toy_inv_tick_co_wf₁

/-! ## The per-tick order selection (D71): `tassume_ordering`

A tick knot (the tick counter) feeds a tick body whose output batch is
realized as a per-tick SEQUENCE by `tassume_ordering` and then folded
per key by a NON-commutative closure (keep the last value seen). The
denotation's per-tick order is a content decision (`TickOrderSelDec`);
the corner derives it from the machine's own per-tick arrival order
(`tkOrdSelDerive`), which is why the knot's output must be known
step-monotone (`CoTickSing.mono`, W4) all the way through the tick
former. -/

structure ToyTkDec (H : HydroSem L mem) (n : Nat) where
  /-- `.batch(&tick, nondet!(…))` -/
  batch : H.BatchDec n (Nat × Nat) .noOrder .exactlyOnce
  /-- `.into_keyed().assume_ordering::<TotalOrder>(nondet!(…))` inside the tick -/
  order : H.TickOrderSelDec n (Nat × Nat)

-- the consumer (a content module, house architecture): stamp this
-- tick's pairs with a tick-located counter, realize the tick's order,
-- fold per key by a NON-commutative closure (keep the last value seen)
hydro def toy_tk_consume (H : HydroSem L mem) (ℓ : L)
    (cnt : H.Atomic ℓ Nat)
    (src : H.Stream ℓ (Nat × Nat) .noOrder .exactlyOnce)
    (dec : ToyTkDec H (mem ℓ)) :
    H.AtomicStream ℓ (Nat × Option Nat) .noOrder .exactlyOnce :=
  let batched := H.batch src dec.batch
  tick (input c := cnt) (input b := batched) :=
    let stamped := H.bmap (H.bcrossSingleton b c) (fun p => (p.1.1, p.1.2 + p.2))
    yield (stamped_out := stamped);
  let ordered := H.tassume_ordering stamped_out dec.order
  tick (input kb := ordered) :=
    let last := H.bkfold (fun (_a : Option Nat) (v : Nat) => some v) none
      (fun _ _ => True) trivial (H.binto_keyed kb)
    yield (out := H.bksEntries last);
  out

#print axioms toy_tk_consume
#check @toy_tk_consume_co_wf₁
#check @toy_tk_consume_smono₁
#nondet_census toy_tk_consume (nondets := 2) (scheds := 0) (fuels := 0)

structure ToyTkKnotDec (H : HydroSem L mem) (n : Nat) where
  fuel : H.FixDec
  consume : ToyTkDec H n

-- the knot closes over the consumer: the counter advances by one plus
-- the number of keys the consumer emitted last tick (the consumer reads
-- the counter BEFORE the tick — `defer_tick` — so the cycle is causal)
hydro def toy_tk_order (H : HydroSem L mem) (ℓ : L)
    (src : H.Stream ℓ (Nat × Nat) .noOrder .exactlyOnce)
    (dec : ToyTkKnotDec H (mem ℓ)) :
    H.AtomicStream ℓ (Nat × Option Nat) .noOrder .exactlyOnce :=
  fix (w : H.Atomic ℓ Nat) via dec.fuel :=
    let prev := H.defer_tick 0 w
    let out := toy_tk_consume H ℓ prev src dec.consume
    tick (input o := out) :=
      emit (cntv := H.bcount o);
    let next := H.mapTick (H.zipTick prev cntv) (fun _me p => p.1 + 1 + p.2)
    complete (next)
    out

#print axioms toy_tk_order
#check @toy_tk_order_co_wf₁
#check @toy_tk_order_smono₁
#check @toy_tk_order.w_smono₁
#nondet_census toy_tk_order (nondets := 2) (scheds := 0) (fuels := 1)

-- two ticks; the realized order decides which value a key keeps
#guard (toy_tk_order (Values Unit (fun _ => 1)) ()
    (.mk' fun _ => ({(1, 10), (2, 20), (1, 30)} : Multiset (Nat × Nat)))
    ⟨(3 : UnfoldFuel), ⟨fun _ => [{(1, 10), (2, 20), (1, 30)}, {}],
     fun _ => [[(2, 20), (1, 10), (1, 30)], []]⟩⟩) 0
  = [{(1, some 30), (2, some 20)}, {}]
#guard (toy_tk_order (Values Unit (fun _ => 1)) ()
    (.mk' fun _ => ({(1, 10), (2, 20), (1, 30)} : Multiset (Nat × Nat)))
    ⟨(3 : UnfoldFuel), ⟨fun _ => [{(1, 10), (2, 20), (1, 30)}, {}],
     fun _ => [[(1, 30), (2, 20), (1, 10)], []]⟩⟩) 0
  = [{(1, some 10), (2, some 20)}, {}]

/-! ## `defer_tick` is release-at-next-tick (D85)

The machine (`SchedSem`) and the production node (`Dag.run`) on the same
input: a value written at every tick (visible in the step it is written,
as a tick body's own singleton is), read one tick later. The machine's
view at step `k` is `init :: (the writes of ticks < k)` — the slot wire is
one tick behind — and the node emits one `some` per tick: the initial
value at the first tick, then the previous tick's write. (Before D85 the
machine released the current tick's write in the same step — `[0, 7]`,
`[0, 7, 7]`, `[0, 7, 7, 7]` here — and the node's cell was a list,
`[0, 7]`, `[7]`, `[7]`.) -/
hydro def toy_defer_next_tick (H : HydroSem L mem) (ℓ : L) (a : H.Atomic ℓ Nat) : H.Atomic ℓ Nat :=
  H.defer_tick 0 a

#print axioms toy_defer_next_tick
#check @toy_defer_next_tick_co_wf₁
#check @toy_defer_next_tick_smono₁
#check @toy_defer_next_tick.dag_denote
#nondet_census toy_defer_next_tick (nondets := 0) (scheds := 0) (fuels := 0)

/-- The input writes `7` at every tick; its view at step `k` has the writes
of ticks `0..k`. -/
def toyDeferInput : (SchedSem Unit (fun _ => 1) (Pacing.ofTicks fun _ _ _ => true)).Atomic () Nat :=
  fun _ step => List.replicate (step + 1) 7

-- the machine's view, as of steps 0, 1, 2
#guard ((List.range 3).map fun k =>
    toy_defer_next_tick (SchedSem Unit (fun _ => 1) (Pacing.ofTicks fun _ _ _ => true)) ()
      toyDeferInput ⟨0, by decide⟩ k)
  = [[0], [0, 7], [0, 7, 7]]
-- the production node on the same input's increments (one `some` per tick)
#guard ((List.range 3).map fun k =>
    (Project.Dag.run (toy_defer_next_tick.dag (mem := fun _ => 1) ())
      (toy_defer_next_tick.schedOf (Pacing.ofTicks fun _ _ _ => true) ())
      (Pacing.ofTicks fun _ _ _ => true)
      (fun step => Project.HList.cons (fun _ => (incTrace (toyDeferInput ⟨0, by decide⟩) step).head?) Project.HList.nil)
      k).2.get Project.Var.zero ⟨0, by decide⟩)
  = [some 0, some 7, some 7]


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
example : @Eq Nat ((SchedSem Unit (fun _ => 1) (Pacing.ofTicks fun _ _ _ => true)).bcount
    ((SchedSem Unit (fun _ => 1) (Pacing.ofTicks fun _ _ _ => true)).bmap (ord := .noOrder)
      [3, 1, 2] (· * 2))) 3 := rfl
-- the machine keeps the runtime's order; the type forbids observing it
example : ((SchedSem Unit (fun _ => 1) (Pacing.ofTicks fun _ _ _ => true)).bmap (ord := .noOrder)
    [3, 1, 2] (· * 2) : List Nat) = [6, 2, 4] := rfl
-- `enumerate` exists only at TotalOrder, where both legs are lists
example : ((Values Unit (fun _ => 1)).benumerate [10, 11] : List (Nat × Nat))
  = [(0, 10), (1, 11)] := rfl
example : ((SchedSem Unit (fun _ => 1) (Pacing.ofTicks fun _ _ _ => true)).benumerate [10, 11]
  : List (Nat × Nat)) = [(0, 10), (1, 11)] := rfl
-- `fold` at NoOrder demands commutativity (the quotient's own obligation)
example : @Eq Nat ((Values Unit (fun _ => 1)).bfold (ord := .noOrder) (ret := .exactlyOnce)
    (· + ·) 0 (fun _ _ => True) (fun s x y _ => Nat.add_right_comm s x y)
    ({3, 1, 2} : Multiset Nat))
    6 := by
  show Multiset.foldlOn (fun _ _ => True) (· + ·) 0 (fun s x y _ => Nat.add_right_comm s x y)
    ({3, 1, 2} : Multiset Nat) = 6
  rfl

-- the corner: a coupled pair, and the conditional agreements
abbrev CS := CoupleSem Unit (fun _ => 1) (Pacing.ofTicks fun _ _ _ => true) 3 3 le_rfl

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
    = (SchedSem Unit (fun _ => 1) (Pacing.ofTicks fun _ _ _ => true)).bmap (ord := .noOrder) cb.sr (· * 2) := rfl
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
denotational monotonicity, and the free theorem. The `Values` run is
the executable (D73: `#guard` below evaluates the denotation itself). -/

section ShapedFormer

-- a singleton wire and an ordered tick stream in; the register trace
-- and the batch, bumped, out
hydro def toy_shape (H : HydroSem L mem) (ℓ : L)
    (inp : H.Atomic ℓ Nat)
    (bs : H.AtomicStream ℓ Nat .totalOrder .exactlyOnce) :
    H.Atomic ℓ Nat × H.AtomicStream ℓ Nat .totalOrder .exactlyOnce :=
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
#guard (toy_shape (Values Unit (fun _ => 1)) () (.mk' fun _ => [1, 2, 3])
  (.mk' fun _ => [[10], [], [1, 2]])).1 0 = [2, 4, 9]
#guard (toy_shape (Values Unit (fun _ => 1)) () (.mk' fun _ => [1, 2, 3])
  (.mk' fun _ => [[10], [], [1, 2]])).2 0 = [[11], [], [2, 3]]
-- the machine, as of any step
example : (toy_shape (SchedSem Unit (fun _ => 1) (Pacing.ofTicks fun _ _ _ => true)) ()
    (fun _ _ => [1, 2, 3]) (fun _ _ => [[10], [], [1, 2]])).1 0 5 = [2, 4, 9] := rfl

end ShapedFormer


/-! ## D89: a callee's singleton output, snapshotted by its caller

`timeout` returns `latest`'s unbounded singleton and `p_leader_heartbeat` snapshots
it: the consumer's derived cut is spelled through the callee's generated `_co_src₁`
naming (the corner's remembered machine source). -/

hydro def toy_lat (H : HydroSem L mem) (ℓ : L) (inp : H.Atomic ℓ (Option Nat)) :
    H.Singleton ℓ (Option Nat) (Option Nat) .totalOrder .exactlyOnce .unbounded :=
  H.latest inp

structure ToySnapDec (H : HydroSem L mem) (n : Nat) where
  snap : H.SnapDec n (Option Nat) .totalOrder

hydro def toy_snap (H : HydroSem L mem) (ℓ : L) (inp : H.Atomic ℓ (Option Nat))
    (dec : ToySnapDec H (mem ℓ)) : H.Atomic ℓ (Option Nat) :=
  let lat := toy_lat H ℓ inp
  H.snapshot lat dec.snap

#nondet_census toy_snap (nondets := 1) (scheds := 0) (fuels := 0)

-- the synchronous read (one cut per tick) is the tick trace itself; an asynchronous
-- read repeats the stale update
#guard (toy_snap (Values Unit (fun _ => 1)) () (.mk' fun _ => [none, some 3, none]) ⟨fun _ => [1, 1, 1]⟩ 0)
  = [none, some 3, none]
#guard (toy_snap (Values Unit (fun _ => 1)) () (.mk' fun _ => [none, some 3, none]) ⟨fun _ => [2, 0, 1]⟩ 0)
  = [some 3, some 3, none]

/-! ## `batch` on ordered at-least-once content (D91)

The one `batch` is graded like the stream it cuts. On a `TotalOrder × AtLeastOnce`
stream the decision is the realized batches themselves (lists, duplicates allowed); a
batch is admitted while the destuttered concatenation stays in the pool's class, and
`.first()` of a batch is the head of its class. -/

structure ToyStutterDec (H : HydroSem L mem) (n : Nat) where
  batch : H.BatchDec n Nat .totalOrder .atLeastOnce

hydro def toy_batch_stutter (H : HydroSem L mem) (ℓ : L)
    (inp : H.Stream ℓ Nat .totalOrder .atLeastOnce) (dec : ToyStutterDec H (mem ℓ)) :
    H.Atomic ℓ (Option Nat) :=
  tick (input b := H.batch inp dec.batch) :=
    emit (first := H.bfirst b);
  first

#print axioms toy_batch_stutter
#check @toy_batch_stutter_co_wf₁
#check @toy_batch_stutter_smono₁
#check @toy_batch_stutter.dag_denote
#nondet_census toy_batch_stutter (nondets := 1) (scheds := 0) (fuels := 0)

-- the pool's class is `[1, 2, 3]`; the first three batches re-deliver within and across
-- the boundary (`[1,1]`, `[1,2]`, `[2,3,3]`) and are admitted; `[3, 1]` leaves the
-- class and blocks
#guard (toy_batch_stutter (Values Unit (fun _ => 1)) ()
    (.mk' fun _ => StutterSeq.mk [1, 1, 2, 3])
    ⟨fun _ => [[1, 1], [1, 2], [], [2, 3, 3], [3, 1]]⟩ 0)
  = [some 1, some 1, none, some 2]
-- the machine: a tick per step, everything arrived is consumed
#guard ((List.range 3).map fun k =>
    toy_batch_stutter (SchedSem Unit (fun _ => 1) (Pacing.ofTicks fun _ _ _ => true)) ()
      (fun _ => StepHist.const [1, 1, 2]) ⟨()⟩ ⟨0, by decide⟩ k)
  = [[some 1], [some 1, none], [some 1, none, none]]

end Hydro
