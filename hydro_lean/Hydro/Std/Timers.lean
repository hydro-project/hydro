import Hydro.Values
import Hydro.Duration
import Hydro.HydroTick
import Hydro.HydroDef

/-!
# `hydro_lang/src/live_collections/stream/mod.rs` `timeout`, `optional.rs` `sample_every`

The two timer-driven library functions, one Lean definition each (D89). They were
primitives of the signature (`timeout_snapshot`, `sample_every`) while the Rust
`timeout` read the wall clock inside a closure — unmirrorable; the Rust now reads the
clock once per tick (`use::time`) and both are ordinary programs over the two timing
primitives that remain: `time` (the tick's clock reading) and `source_interval` (the
interval timer as a stream of `()`, batched at the tick — D91).

The clock values are naturals in microseconds (`Hydro/Duration.lean`); `duration` and
`interval` likewise.
-/

namespace Hydro

variable {L : Type} {mem : L → Nat}

/-! ## `Stream::timeout` -/

/-- **`timeout`'s nondeterminism bundle**: the Rust's one `nondet` guard covers the
batching of the arrivals and the clock reading (the two `nondet!(nondet)` sites). -/
structure TimeoutDec (H : HydroSem L mem) (n : Nat) (α : Type) where
  /-- `use::batch(self, nondet!(/** sampling timing … */ nondet))`. -/
  arrivals : H.BatchDec n α .noOrder .atLeastOnce
  /-- `use::time(nondet!(/** the verdict depends on when the tick runs */ nondet))`. -/
  clock : H.ClockDec n

/-- What `timeout` **ensures**, over the `Values` denotation: the output is the
`latest()` of a per-tick verdict trace on which **a fresh arrival is never expired** —
at a tick whose batch is non-empty, whatever verdict the tick produces is `none` (the
latest arrival is stamped with this tick's reading, and `now − now = 0 ≤ duration`). -/
structure TimeoutEnsures {α : Type} [DecidableEq α] (ℓ : L) (duration : Nat)
    (arrivals : MemberMap (mem ℓ) (RetryPool α))
    (dec : TimeoutDec (Values L mem) (mem ℓ) α)
    (out : MemberMap (mem ℓ) (CutDec (Option Unit) .totalOrder → Trace (Option Unit))) : Prop where
  fresh_not_expired : ∃ verdict : MemberMap (mem ℓ) (Trace (Option Unit)),
    (∀ i, out i = latestCell (verdict i)) ∧
    ∀ (i : Fin (mem ℓ)) (u : Nat) (b : RetryPool α),
      (retryBatchCuts (arrivals i) (dec.arrivals i))[u]? = some b →
      poolIsEmpty (ord := .noOrder) (ret := .atLeastOnce) b = false →
      ∀ e, (verdict i)[u]? = some e → e = none

/-- **`stream/mod.rs` `timeout`** over location `ℓ`: expired at a tick iff nothing has ever
arrived, or the clock reading is more than `duration` past the reading at the latest
arrival; returned as Rust's `Optional<(), L, Unbounded>` (the per-tick verdict's
`latest()` — the consumer's `snapshot` reads it asynchronously). The arrivals stream is
at-least-once (the heartbeat stream that feeds it is); batching it is legal at that grade
(D89). -/
hydro def timeout {α : Type} [DecidableEq α] (H : HydroSem L mem) (ℓ : L)
    (arrivals : H.Stream ℓ α .noOrder .atLeastOnce) (duration : Nat)
    (dec : TimeoutDec H (mem ℓ) α) :
    H.Singleton ℓ (Option Unit) (Option Unit) .totalOrder .exactlyOnce .unbounded
  ensures out => TimeoutEnsures ℓ duration arrivals dec out :=
  -- sliced! {
  --   let new_arrivals = use::batch(self, nondet!(…));
  --   let now = use::time(nondet!(…));
  --   let mut latest_received = use::state_null::<Optional<Instant, Tick<_>, Bounded>>();
  tick (state latest_received : Option Nat := none)
      (input new_arrivals := H.batch arrivals dec.arrivals)
      (input now := H.time dec.clock) :=
    -- // the latest arrival is stamped with this tick's clock reading
    -- latest_received = now.clone().filter_if(new_arrivals.is_empty().map(q!(|empty| !empty)))
    --   .or(latest_received);
    let stamped := H.boOr
      (H.bsFilterIf now (H.bsMap (H.bisEmpty new_arrivals) (fun empty => !empty)))
      latest_received
    -- latest_received.clone().into_singleton().zip(now).filter_map(q!(move |(latest_received, now)| {
    --   if let Some(latest_received) = latest_received {
    --     if now.duration_since(latest_received) > duration { Some(()) } else { None }
    --   } else { Some(()) } }))
    let expired := H.bsFilterMap (H.bsZip stamped now)
      (fun (latest_received, now) => match latest_received with
        | some latest_received => if duration < now - latest_received then some () else none
        | none => some ())
    rebind (latest_received := stamped)
    emit (expired := expired);
  -- }   — the block's `Optional<(), Tick, Bounded>` leaves it as `Optional<(), L, Unbounded>`: `.latest()`
  H.latest expired
  prove
    fresh_not_expired := ⟨expired, fun _ => rfl, fun i u b hb hne e he => by
      obtain ⟨na, nw, hna, _hnw, he⟩ := (hexpired_at i u e).mp he
      have hna' : (retryBatchCuts (arrivals i) (dec.arrivals i))[u]? = some na := hna
      rw [hb] at hna'
      cases hna'
      subst he
      simp [expired_step, den, hne]⟩

#nondet_census timeout (nondets := 2) (scheds := 0) (fuels := 0)

/-! ## `Optional::sample_every` -/

/-- **`sample_every`'s nondeterminism bundle**: the one `nondet!` site, the batch of the
interval timer's `()`s (the snapshot is the caller's). -/
structure SampleEveryDec (H : HydroSem L mem) (n : Nat) where
  /-- `samples.batch(&tick, nondet!(/** sample timing … */))`. -/
  samples : H.BatchDec n Unit .totalOrder .atLeastOnce

/-- **`sample_every`'s sched-det bundle**: the interval timer's fire timing (D91). -/
structure SampleEverySched (H : HydroSem L mem) (n : Nat) where
  /-- `self.location.source_interval(interval)`. -/
  interval : H.IntervalDec n

def SampleEverySched.triv {n : Nat} : SampleEverySched (Values L mem) n := ⟨()⟩

/-- What `sample_every` **ensures**, over the `Values` denotation. -/
structure SampleEveryEnsures {α : Type} [DecidableEq α] (ℓ : L)
    (snapshot : MemberMap (mem ℓ) (Trace (Option α)))
    (out : MemberMap (mem ℓ) (StutterSeq α)) : Prop where
  /-- **Samples are snapshots**: every sampled value was the optional's value at some
  tick. -/
  sampled_src : ∀ (i : Fin (mem ℓ)) (x : α), x ∈ (↑(out i).norm : Multiset α) →
    ∃ u : Nat, (snapshot i)[u]? = some (some x)

/-- **`optional.rs` `sample_every`** over location `ℓ`: at every tick the interval fired,
the optional's value, if any; `AtLeastOnce` because an unchanged value sampled twice is
a re-delivery. The input is the per-tick snapshot the Rust takes (`self.snapshot(&tick,
nondet)` — the caller's `.latest()`, an `Atomic` here). -/
hydro def sample_every {α : Type} [DecidableEq α] (H : HydroSem L mem) (ℓ : L)
    (snapshot : H.Atomic ℓ (Option α)) (interval : Nat)
    (dec : SampleEveryDec H (mem ℓ)) (sched : SampleEverySched H (mem ℓ)) :
    H.Stream ℓ α .totalOrder .atLeastOnce
  ensures out => SampleEveryEnsures ℓ snapshot out :=
  -- let samples = self.location.source_interval(interval);   (optional.rs:1524)
  let samples := H.source_interval (fun _ => 0) interval sched.interval
  -- self.snapshot(&tick, nondet)
  --   .filter_if(samples.batch(&tick, nondet!(/** sample timing is captured by the caller's guard */))
  --     .first().is_some())   (optional.rs:1528–1539)
  tick (input snap := snapshot) (input fires := H.batch samples dec.samples) :=
    yield (sampled := H.boIntoStream (H.boFilterIf snap (H.boIsSome (H.bfirst fires))));
  -- .all_ticks().weaken_retries()
  H.weaken_retries (H.allTicks sampled)
  prove
    sampled_src := fun i x hx => by
      have hx' : x ∈ (↑(destutter ((sampled i).flatten)) : Multiset α) := hx
      have h2 := (destutter_sublist _).subset (Multiset.mem_coe.mp hx')
      obtain ⟨l, hl, hxl⟩ := List.mem_flatten.mp h2
      obtain ⟨n, hn⟩ := List.mem_iff_getElem?.mp hl
      obtain ⟨st, pt, hst, _hpt, he⟩ := (hsampled_at i n l).mp hn
      subst he
      refine ⟨n, ?_⟩
      simp only [sampled_step, den] at hxl
      split at hxl
      · cases st <;> simp at hxl
        subst hxl; exact hst
      · simp at hxl

#nondet_census sample_every (nondets := 1) (scheds := 1) (fuels := 0)

end Hydro
