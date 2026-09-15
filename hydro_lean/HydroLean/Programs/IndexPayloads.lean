import HydroLean.Hydro.Stream
import HydroLean.Hydro.TStream
import HydroLean.Hydro.Growth

/-!
# `index_payloads`: slot assignment for Paxos proposers

Port of `index_payloads` from `hydro_test/src/cluster/paxos.rs` (lines
777–806). The Rust code is a `sliced!` block on the proposer tick:

```rust
let mut next_slot = use::state(|l| l.singleton(q!(0)));
let updated_max_slot = use::atomic(p_max_slot.latest_atomic(), nondet!(...));
let payload_batch = use::atomic(c_to_proposers.all_ticks_atomic(), nondet!(...));

let next_slot_after_reconciling_p1bs = updated_max_slot.map(q!(|s| s + 1));
let base_slot = next_slot_after_reconciling_p1bs.unwrap_or(next_slot);

let indexed_payloads = payload_batch
    .enumerate()
    .cross_singleton(base_slot.clone())
    .map(q!(|((index, payload), base_slot)| (base_slot + index, payload)));

let num_payloads = indexed_payloads.clone().count();
next_slot = num_payloads
    .zip(base_slot)
    .map(q!(|(num_payloads, base_slot)| base_slot + num_payloads));

yield_atomic(indexed_payloads)
```

The `nondet!` guards in the Rust callers (e.g. `sequence_payload`'s
`c_to_proposers.batch(proposer_tick, nondet!(/** We batch payloads so that we
can compute the correct slot based on base slot ... */))`) become the
universally quantified tick-input sequence here: the theorems below hold for
**all** batchings and **all** update schedules (goal: unbounded correctness,
vs. the Rust simulator's bounded exploration in the sim tests
`proposer_indexes_payloads` and `proposer_indexes_payloads_jumps_on_new_max`).
-/

namespace HydroLean.Programs

open HydroLean.Hydro

universe u

variable {P : Type u}

/-- Per-tick input of `index_payloads`:
- `maxSlotUpdate` — the per-tick snapshot of `p_max_slot` (Rust:
  `updated_max_slot = use::atomic(p_max_slot.latest_atomic(), ...)`); `some m`
  means leader election reported a maximum reconciled slot `m` this tick.
- `batch` — this tick's batch of client payloads (Rust: `payload_batch`; the
  batch boundaries are the `nondet!`-guarded adversarial choice). -/
structure IndexTickIn (P : Type u) : Type u where
  maxSlotUpdate : Option Nat
  batch : List P

/-- The base slot for a tick (Rust: `base_slot =
next_slot_after_reconciling_p1bs.unwrap_or(next_slot)`). -/
def baseSlot (next_slot : Nat) (t : IndexTickIn P) : Nat :=
  (t.maxSlotUpdate.map (· + 1)).getD next_slot

/-- The `index_payloads` tick loop: looped state is `next_slot` (initially
`0`); each tick enumerates its payload batch starting at `base_slot` and
advances `next_slot` past the assigned range. The body mirrors the Rust
statement-for-statement (see module docs). -/
def indexPayloadsTick (P : Type u) : TickLoop (IndexTickIn P) Nat (List (Nat × P)) where
  init := 0
  step := fun next_slot input =>
    let next_slot_after_reconciling_p1bs := input.maxSlotUpdate.map (· + 1)
    let base_slot := next_slot_after_reconciling_p1bs.getD next_slot
    let indexed_payloads :=
      ((Stream.enumerate input.batch).crossSingleton base_slot).map
        (fun ((index, payload), base) => (base + index, payload))
    let num_payloads := indexed_payloads.length
    (base_slot + num_payloads, indexed_payloads)

/-! ## Specification helpers -/

/-- `enumFrom n l`: pair the elements of `l` with consecutive slots starting
at `n` — the specification of one tick's output. -/
def enumFrom (n : Nat) : List P → List (Nat × P)
  | [] => []
  | p :: ps => (n, p) :: enumFrom (n + 1) ps

@[simp] theorem enumFrom_nil (n : Nat) : enumFrom n ([] : List P) = [] := rfl

@[simp] theorem enumFrom_cons (n : Nat) (p : P) (ps : List P) :
    enumFrom n (p :: ps) = (n, p) :: enumFrom (n + 1) ps := rfl

theorem enumFrom_append (n : Nat) (l₁ l₂ : List P) :
    enumFrom n (l₁ ++ l₂) = enumFrom n l₁ ++ enumFrom (n + l₁.length) l₂ := by
  induction l₁ generalizing n with
  | nil => simp
  | cons p ps ih => simp [ih, Nat.add_comm, Nat.add_left_comm]

@[simp] theorem enumFrom_map_snd (n : Nat) (l : List P) :
    (enumFrom n l).map Prod.snd = l := by
  induction l generalizing n with
  | nil => rfl
  | cons p ps ih => simp [ih]

@[simp] theorem enumFrom_length (n : Nat) (l : List P) :
    (enumFrom n l).length = l.length := by
  induction l generalizing n with
  | nil => rfl
  | cons p ps ih => simp [ih]

/-- Slots produced by `enumFrom n l` lie in `[n, n + l.length)`. -/
theorem fst_mem_enumFrom {n : Nat} {l : List P} {q : Nat × P}
    (h : q ∈ enumFrom n l) : n ≤ q.1 ∧ q.1 < n + l.length := by
  induction l generalizing n with
  | nil => cases h
  | cons p ps ih =>
    cases List.mem_cons.mp h with
    | inl heq => subst heq; simp
    | inr h' =>
      have := ih h'
      simp only [List.length_cons]
      omega

/-- Slots within one tick are strictly increasing (contiguous by
construction). -/
theorem enumFrom_pairwise_lt (n : Nat) (l : List P) :
    ((enumFrom n l).map Prod.fst).Pairwise (· < ·) := by
  induction l generalizing n with
  | nil => exact .nil
  | cons p ps ih =>
    refine .cons (fun s hs => ?_) (ih (n + 1))
    obtain ⟨q, hq, rfl⟩ := List.mem_map.mp hs
    exact Nat.lt_of_lt_of_le (Nat.lt_succ_self n) (fst_mem_enumFrom hq).1

/-- Bridge from the Rust-shaped body (`enumerate` + `cross_singleton` + `map`)
to the specification `enumFrom`. -/
theorem enumerate_cross_map_eq_enumFrom (base : Nat) (l : List P) :
    ((Stream.enumerate l).crossSingleton base).map
        (fun ((index, payload), b) => (b + index, payload))
      = enumFrom base l := by
  have key : ∀ n, (l.zipIdx n).map (fun (q : P × Nat) => (base + q.2, q.1))
      = enumFrom (base + n) l := by
    intro n
    induction l generalizing n with
    | nil => rfl
    | cons p ps ih => simp [List.zipIdx, ih (n + 1), Nat.add_assoc]
  calc ((Stream.enumerate l).crossSingleton base).map
        (fun ((index, payload), b) => (b + index, payload))
      = (l.zipIdx 0).map (fun (q : P × Nat) => (base + q.2, q.1)) := by
        simp only [Stream.enumerate, Stream.crossSingleton, Stream.map, List.map_map]
        exact List.map_congr_left (fun q _ => by cases q; rfl)
    _ = enumFrom (base + 0) l := key 0
    _ = enumFrom base l := by rw [Nat.add_zero]

/-- **Per-tick characterization (contiguity)**: one tick emits exactly
`enumFrom base batch` — a contiguous, strictly increasing slot range — and
advances `next_slot` to `base + |batch|`. -/
theorem indexPayloadsTick_step (s : Nat) (t : IndexTickIn P) :
    (indexPayloadsTick P).step s t
      = (baseSlot s t + t.batch.length, enumFrom (baseSlot s t) t.batch) := by
  simp [indexPayloadsTick, baseSlot, enumerate_cross_map_eq_enumFrom]

/-! ## Theorem 1: determinism without re-election

Rust sim test `proposer_indexes_payloads`: with `p_max_slot` never firing,
payloads receive slots `0, 1, 2, …` in input order — for **every** batching
(the sim test explores this exhaustively on a 4-element instance; here it is
unbounded). -/

/-- Lift a payload batching to tick inputs with no re-election updates. -/
def noReelection (bs : Batching P) : List (IndexTickIn P) :=
  bs.map (fun b => ⟨none, b⟩)

/-- `Stream.enumerate` is `enumFrom 0`. -/
theorem enumerate_eq_enumFrom (l : List P) : Stream.enumerate l = enumFrom 0 l := by
  suffices h : ∀ n, (l.zipIdx n).map (fun (a, i) => (i, a)) = enumFrom n l by
    simpa [Stream.enumerate] using h 0
  intro n
  induction l generalizing n with
  | nil => rfl
  | cons p ps ih => simp [List.zipIdx, ih (n + 1)]

/-- Generalized run lemma for the no-re-election case: starting from
`next_slot = s`, consuming batches `bs` yields final state `s + |flatten bs|`
and per-tick outputs flattening to `enumFrom s bs.flatten`. -/
theorem runFrom_noReelection (s : Nat) (bs : Batching P) :
    ((indexPayloadsTick P).runFrom s (noReelection bs)).1 = s + bs.flatten.length ∧
    allTicks ((indexPayloadsTick P).runFrom s (noReelection bs)).2
      = enumFrom s bs.flatten := by
  induction bs generalizing s with
  | nil => simp [noReelection, allTicks]
  | cons b bs ih =>
    have hstep : (indexPayloadsTick P).step s ⟨none, b⟩
        = (s + b.length, enumFrom s b) := by
      simpa [baseSlot] using indexPayloadsTick_step s (⟨none, b⟩ : IndexTickIn P)
    have hcons : noReelection (b :: bs) = ⟨none, b⟩ :: noReelection bs := rfl
    have ⟨ih₁, ih₂⟩ := ih (s + b.length)
    rw [hcons, TickLoop.runFrom_cons, hstep]
    refine ⟨?_, ?_⟩
    · show ((indexPayloadsTick P).runFrom (s + b.length) (noReelection bs)).1 = _
      rw [ih₁]
      simp [Nat.add_assoc]
    · show allTicks (enumFrom s b
          :: ((indexPayloadsTick P).runFrom (s + b.length) (noReelection bs)).2) = _
      simp only [allTicks, List.flatten_cons]
      have ih₂' : (((indexPayloadsTick P).runFrom (s + b.length) (noReelection bs)).2).flatten
          = enumFrom (s + b.length) bs.flatten := ih₂
      rw [ih₂', enumFrom_append]

/-- **Theorem (no-reelection determinism, unbounded)**: with no `p_max_slot`
updates, for **every** batching of the payload stream, the `all_ticks` output
is exactly `input.enumerate` — slots `0, 1, 2, …` in input order.

Rust counterpart: sim test `proposer_indexes_payloads` (bounded, 4 payloads,
exhaustive batchings); the `nondet!(/** We batch payloads so that we can
compute the correct slot based on base slot ... */)` guard in
`sequence_payload` is the `∀ bs, bs.of input` quantifier here. -/
theorem indexPayloads_no_reelection_deterministic
    (input : List P) (bs : Batching P) (h : bs.of input) :
    allTicks ((indexPayloadsTick P).run (noReelection bs)).2
      = Stream.enumerate input := by
  rw [enumerate_eq_enumFrom, ← h]
  exact (runFrom_noReelection 0 bs).2

/-! ## Theorem 2: general update schedules

When leader election delivers `maxSlotUpdate`s, slot assignment jumps (Rust
sim test `proposer_indexes_payloads_jumps_on_new_max`). Two properties hold
for **arbitrary** update schedules and batchings:

1. **payload preservation** — the emitted payloads are exactly the input
   payloads, in order;
2. **strict slot monotonicity** — provided each update is *fresh* (jumps
   forward: `next_slot ≤ m + 1`, which is how `p_max_slot` behaves after a
   re-election reconciles a log at least as long as what this proposer
   assigned), all emitted slots across the entire run are strictly increasing.
   Freshness is necessary: a backward jump re-assigns already-used slots (in
   full Paxos this is harmless for safety because slots are paired with
   ballots, but local monotonicity genuinely requires the premise). -/

/-- Payload preservation: for any tick-input sequence (any batching, any
update schedule), emitted payloads = concatenated input batches, in order. -/
theorem indexPayloads_payload_preservation (s : Nat) (ts : List (IndexTickIn P)) :
    (allTicks ((indexPayloadsTick P).runFrom s ts).2).map Prod.snd
      = (ts.map (·.batch)).flatten := by
  induction ts generalizing s with
  | nil => simp [allTicks]
  | cons t ts ih =>
    simp only [TickLoop.runFrom_cons, indexPayloadsTick_step, allTicks,
      List.flatten_cons, List.map_append, List.map_cons]
    rw [enumFrom_map_snd]
    have := ih (baseSlot s t + t.batch.length)
    simp only [allTicks] at this
    rw [this]

/-- Freshness of an update schedule along a run: every `some m` update
satisfies `next_slot ≤ m + 1` at the tick where it fires. -/
def FreshUpdates : Nat → List (IndexTickIn P) → Prop
  | _, [] => True
  | s, t :: ts =>
    (∀ m, t.maxSlotUpdate = some m → s ≤ m + 1) ∧
      FreshUpdates (baseSlot s t + t.batch.length) ts

/-- Under fresh updates, every slot emitted from state `s` is `≥ s`, and the
whole emitted slot sequence is strictly increasing. -/
theorem indexPayloads_slots_strictMono (s : Nat) (ts : List (IndexTickIn P))
    (h : FreshUpdates s ts) :
    ((allTicks ((indexPayloadsTick P).runFrom s ts).2).map Prod.fst).Pairwise (· < ·) ∧
    (∀ q ∈ allTicks ((indexPayloadsTick P).runFrom s ts).2, s ≤ q.1) := by
  induction ts generalizing s with
  | nil => exact ⟨.nil, by simp [allTicks]⟩
  | cons t ts ih =>
    obtain ⟨hfresh, hrest⟩ := h
    have hbase : s ≤ baseSlot s t := by
      cases hm : t.maxSlotUpdate with
      | none => simp [baseSlot, hm]
      | some m => simpa [baseSlot, hm] using hfresh m hm
    have ⟨ihMono, ihLb⟩ := ih (baseSlot s t + t.batch.length) hrest
    simp only [TickLoop.runFrom_cons, indexPayloadsTick_step, allTicks,
      List.flatten_cons, List.map_append] at *
    constructor
    · refine List.pairwise_append.mpr ⟨enumFrom_pairwise_lt _ _, ihMono, ?_⟩
      intro a ha b hb
      obtain ⟨qa, hqa, rfl⟩ := List.mem_map.mp ha
      obtain ⟨qb, hqb, rfl⟩ := List.mem_map.mp hb
      have h₁ := (fst_mem_enumFrom hqa).2
      have h₂ := ihLb qb hqb
      omega
    · intro q hq
      cases List.mem_append.mp hq with
      | inl h => exact Nat.le_trans hbase (fst_mem_enumFrom h).1
      | inr h => have := ihLb q h; omega

/-! ## Executable mirrors of the Rust sim tests -/

-- Sim test `proposer_indexes_payloads`: 4 payloads, no updates —
-- slots `0..3` in order, under both extreme batchings.
#guard allTicks ((indexPayloadsTick Nat).run (noReelection (Batching.whole [1,2,3,4]))).2
    = [(0,1), (1,2), (2,3), (3,4)]
#guard allTicks ((indexPayloadsTick Nat).run (noReelection (Batching.singletons [1,2,3,4]))).2
    = [(0,1), (1,2), (2,3), (3,4)]

-- Sim test `proposer_indexes_payloads_jumps_on_new_max`: an update `123`
-- firing between batches makes subsequent slots continue from `124`.
#guard allTicks ((indexPayloadsTick Nat).run
    [⟨none, [1,2]⟩, ⟨some 123, [3]⟩, ⟨none, [4]⟩]).2
    = [(0,1), (1,2), (124,3), (125,4)]
-- Update arriving in the very first tick.
#guard allTicks ((indexPayloadsTick Nat).run
    [⟨some 123, [1,2,3,4]⟩]).2
    = [(124,1), (125,2), (126,3), (127,4)]

/-! ## The typed stage and its verified face

`index_payloads` over the located surface: inputs are the `p_max_slot`
per-tick snapshot wire (`TSing (Option Nat)`) and the client payload stream;
the `nondet!` batching guard is the demand-count decision `dbatch`
(TotalOrder input). The stage is the single source of the dataflow; the
faces below state its input/output contract over the run. -/

/-- `index_payloads` as the typed stage (Rust: the `sliced!` block of
paxos.rs:777–806). -/
def index_payloadsM (dbatch : List Nat) :
    TSing (Option Nat) × Stream P →ₘ TStream (Nat × P) :=
  let updated_max_slot := MonoMap.fst (α := TSing (Option Nat)) (β := Stream P)
  let payload_batch :=
    (MonoMap.snd (α := TSing (Option Nat)) (β := Stream P)).batch dbatch
  ((payload_batch.zipWith updated_max_slot
    (fun b m => IndexTickIn.mk m b)).loop (indexPayloadsTick P))

/-- The run reduces to the tick loop over the zipped tick inputs
(definitional bridge). -/
theorem index_payloadsM_run (dbatch : List Nat)
    (ms : TSing (Option Nat)) (pay : Stream P) :
    (index_payloadsM dbatch).f (ms, pay)
      = (indexPayloadsTick P).outputs
          (List.zipWith (fun b m => IndexTickIn.mk m b)
            (Hydro.batch pay dbatch) ms) := rfl

/-- Zipping an all-`none` update wire produces `noReelection` inputs. -/
private theorem zip_none_eq_noReelection {bs : List (List P)}
    {ms : List (Option Nat)} (hlen : bs.length ≤ ms.length)
    (hnone : ∀ m ∈ ms, m = none) :
    List.zipWith (fun b m => IndexTickIn.mk m b) bs ms = noReelection bs := by
  induction bs generalizing ms with
  | nil => rfl
  | cons b bs ih =>
    cases ms with
    | nil => exact absurd hlen (by simp)
    | cons m ms =>
      rw [List.zipWith_cons_cons,
        show noReelection (b :: bs) = ⟨none, b⟩ :: noReelection bs from rfl,
        hnone m List.mem_cons_self,
        ih (Nat.le_of_succ_le_succ (by simpa using hlen))
          (fun x hx => hnone x (List.mem_cons_of_mem m hx))]

/-- **Face (no-reelection determinism)**: if the update wire never fires
(input property) and the batching decision consumes the whole payload stream
without truncation (decision property), the output is exactly
`payloads.enumerate` — slots `0, 1, 2, …` in input order, for **every**
batching. Rust sim test `proposer_indexes_payloads`, unbounded. -/
theorem index_payloads_no_reelection (dbatch : List Nat)
    {ms : TSing (Option Nat)} {pay : Stream P}
    (hnone : ∀ m ∈ ms, m = none)
    (hcons : (Hydro.batch pay dbatch).flatten = pay)
    (hlen : (Hydro.batch pay dbatch).length ≤ ms.length) :
    allTicks ((index_payloadsM dbatch).f (ms, pay))
      = Stream.enumerate pay := by
  rw [index_payloadsM_run, zip_none_eq_noReelection hlen hnone]
  exact indexPayloads_no_reelection_deterministic pay _ hcons

/-- **Face (payload preservation)**: for any update wire and any batching
decision, the emitted payloads are exactly the consumed payload batches, in
order — `index_payloads` never drops, duplicates, or reorders. -/
theorem index_payloads_preservation (dbatch : List Nat)
    (ms : TSing (Option Nat)) (pay : Stream P) :
    (allTicks ((index_payloadsM dbatch).f (ms, pay))).map Prod.snd
      = ((List.zipWith (fun b m => IndexTickIn.mk m b)
          (Hydro.batch pay dbatch) ms).map (·.batch)).flatten := by
  rw [index_payloadsM_run]
  exact indexPayloads_payload_preservation 0 _

/-- **Face (strict slot monotonicity)**: if every reconciliation update is
*fresh* (jumps forward — how `p_max_slot` behaves after a re-election
reconciles a log at least as long as what this proposer assigned; the named
input property, cf. paxos.rs), all emitted slots across the run are strictly
increasing — no slot is ever assigned twice. -/
theorem index_payloads_slots_strictMono (dbatch : List Nat)
    {ms : TSing (Option Nat)} {pay : Stream P}
    (hfresh : FreshUpdates 0
      (List.zipWith (fun b m => IndexTickIn.mk m b)
        (Hydro.batch pay dbatch) ms)) :
    ((allTicks ((index_payloadsM dbatch).f (ms, pay))).map
      Prod.fst).Pairwise (· < ·) := by
  rw [index_payloadsM_run]
  exact (indexPayloads_slots_strictMono 0 _ hfresh).1


end HydroLean.Programs
