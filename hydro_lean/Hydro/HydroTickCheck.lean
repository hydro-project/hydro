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
    (dec : H.BatchDec (mem ℓ) (Nat × Bool)) :
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
    (dec : H.BatchDec (mem ℓ) (Nat × Nat)) :
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
    (dec : H.BatchDec (mem ℓ) (Nat × (Nat × Nat))) :
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
    (dec : H.BatchDec (mem ℓ) (Nat × Nat)) :
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

-- a caller discharges the callee's clause for its input: a `map` that
-- forces the tie (the second component is a function of the first)
hydro def toy_pre_caller (H : HydroSem L mem) (ℓ : L)
    (src : H.Stream ℓ (Nat × Nat) .noOrder .exactlyOnce)
    (wm : H.Atomic ℓ (Option Nat))
    (dec : H.BatchDec (mem ℓ) (Nat × (Nat × Nat))) :
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
  batch : H.BatchDec n (Nat × (Nat × Nat))
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
  batch : H.BatchDec n (Nat × Nat)
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
    (· + ·) 0 (fun _ _ => True) (fun s x y _ => Nat.add_right_comm s x y)
    ({3, 1, 2} : Multiset Nat))
    6 := by
  show Multiset.foldlOn (fun _ _ => True) (· + ·) 0 (fun s x y _ => Nat.add_right_comm s x y)
    ({3, 1, 2} : Multiset Nat) = 6
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
example : (toy_shape (SchedSem Unit (fun _ => 1) (fun _ _ _ => true)) ()
    (fun _ _ => [1, 2, 3]) (fun _ _ => [[10], [], [1, 2]])).1 0 5 = [2, 4, 9] := rfl

end ShapedFormer

end Hydro
