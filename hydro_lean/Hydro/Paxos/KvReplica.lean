import Hydro.Values
import Hydro.HydroTick
import Hydro.HydroDef
import Hydro.HMap
import Mathlib.Data.Prod.Lex
import Mathlib.Order.WithBot

/-!
# `hydro_test/src/cluster/kv_replica/` — the replicated KV store

One Rust function = one Lean definition: `sequence_payloads`
(sequence_payloads.rs:9–57) and `kv_replica` (mod.rs:46–111).

**Closures/handles, flattened (ruling F).** Rust's `sequence_payloads`
returns the `TickCycleHandle` of its `r_next_slot` cycle for
`kv_replica` to complete (`cycle_with_initial(singleton(0))` +
`complete_next_tick(r_next_slot)`). Here `sequence_payloads` takes the
`r_next_slot` tick wire and `kv_replica` closes it — its two tick cycles
(`r_next_slot`, `r_checkpointed_seqs`) are one `fix … complete` whose
body `defer_tick`s the next values (the `complete_next_tick`). The
let-chains are otherwise the Rust's.

**The sort.** `SequencedKv`'s Rust `Ord` compares `seq` only
(mod.rs:34–44) — not antisymmetric against the derived `Eq`
(SCHED_AUDIT). A sound `sort` needs a linear order, so the Lean order is
lexicographic (`seq`, then the payload); on the slot-functional traffic
Paxos delivers (one value per slot) the two orders agree.
-/

namespace Hydro

variable {L : Type} {mem : L → Nat}

/-- `pub struct KvPayload<K, V> { pub key: K, pub value: V }` (mod.rs:21–25). -/
structure KvPayload (K V : Type) where
  key : K
  value : V
  deriving DecidableEq, Repr

instance {K V : Type} [Wire K] [Wire V] : Wire (KvPayload K V) :=
  Wire.ofEquiv (fun x => (x.key, x.value)) (fun (key, value) => { key, value }) (fun _ => rfl)

/-- `pub struct SequencedKv<K, V> { pub seq: usize, pub kv: Option<KvPayload<K, V>> }`
(mod.rs:27–32). -/
structure SequencedKv (K V : Type) where
  seq : Nat
  kv : Option (KvPayload K V)
  deriving DecidableEq, Repr

instance {K V : Type} [Wire K] [Wire V] : Wire (SequencedKv K V) :=
  Wire.ofEquiv (fun x => (x.seq, x.kv)) (fun (seq, kv) => { seq, kv }) (fun _ => rfl)

namespace SequencedKv

variable {K V : Type} [LinearOrder K] [LinearOrder V]

/-- The sort key: `seq` first (the Rust `Ord`), then the payload. -/
def key (s : SequencedKv K V) : Lex (Nat × WithBot (Lex (K × V))) :=
  toLex (s.seq, (s.kv.map (fun p => toLex (p.key, p.value)) : WithBot (Lex (K × V))))

theorem key_injective : Function.Injective (key (K := K) (V := V)) := by
  intro a b h
  have hinj : Function.Injective (fun p : KvPayload K V => (toLex (p.key, p.value) : Lex (K × V))) := by
    intro p q hpq
    have := toLex_inj.mp hpq
    simp only [Prod.mk.injEq] at this
    cases p; cases q
    simp only [KvPayload.mk.injEq]
    exact this
  have h' := toLex_inj.mp h
  have h1 : a.seq = b.seq := congrArg Prod.fst h'
  have h2 : a.kv.map (fun p => (toLex (p.key, p.value) : Lex (K × V)))
      = b.kv.map (fun p => (toLex (p.key, p.value) : Lex (K × V))) := congrArg Prod.snd h'
  have h3 : a.kv = b.kv := Option.map_injective hinj h2
  cases a; cases b
  simp only at h1 h3
  rw [h1, h3]

/-- `impl Ord for SequencedKv` (mod.rs:34–39), made linear (see the header). -/
instance : LinearOrder (SequencedKv K V) := LinearOrder.lift' key key_injective

end SequencedKv

/-- `sequence_payloads`'s decisions: its one `nondet!`. -/
structure SPaysDec (H : HydroSem L mem) (n : Nat) (K V : Type) [DecidableEq K] [DecidableEq V] where
  /-- `p_to_replicas.batch(replica_tick, nondet!(/** because we fill slots one-by-one … */))` (sequence_payloads.rs:19–22). -/
  batch : H.BatchDec n (SequencedKv K V) .noOrder .exactlyOnce

/-- **sequence_payloads.rs:9–57 `sequence_payloads`**: buffer out-of-order
payloads and release the contiguous prefix from `r_next_slot`. Returns
`r_processable_payloads` (this tick's releasable payloads, sorted); the
`r_next_slot_complete_cycle` handle is the caller's knot (F). -/
hydro def sequence_payloads {K V : Type} [DecidableEq K] [DecidableEq V] [LinearOrder K] [LinearOrder V]
    (H : HydroSem L mem) (replica : L)
    (p_to_replicas : H.Stream replica (SequencedKv K V) .noOrder .exactlyOnce)
    (r_next_slot : H.Atomic replica Nat)
    (dec : SPaysDec H (mem replica) K V) :
    H.AtomicStream replica (SequencedKv K V) .totalOrder .exactlyOnce :=
  -- let (r_buffered_payloads_complete_cycle, r_buffered_payloads) =
  --   replica_tick.cycle::<Stream<SequencedKv<K, V>, Tick<L::DropConsistency>, Bounded>, _>();
  tick (state r_buffered_payloads : H.BoundedStream (SequencedKv K V) .totalOrder .exactlyOnce)
      -- let r_sorted_payloads = p_to_replicas.batch(replica_tick, nondet!(…))
      (input batch := H.batch p_to_replicas dec.batch)
      -- let (r_next_slot_complete_cycle, r_next_slot) = replica_tick.cycle_with_initial(replica_tick.singleton(q!(0)));   — the caller's knot (F)
      (input r_next_slot := r_next_slot) :=
    -- .chain(r_buffered_payloads) // Combine with all payloads that we've received and not processed yet
    -- .sort();                    — `chain`'s `MinOrder` forgets the buffer's order; `sort` restores one
    let r_sorted_payloads := H.bsort (H.bchain batch (H.bweakenOrder r_buffered_payloads))
    -- // Find highest the sequence number of any payload that can be processed in this tick. This is the payload right before a hole.
    -- let r_next_slot_after_processing_payloads = r_sorted_payloads.clone().cross_singleton(r_next_slot).fold(
    --   q!(|| 0),
    --   q!(|new_next_slot, (sorted_payload, next_slot)| {
    --     if sorted_payload.seq == std::cmp::max(*new_next_slot, next_slot) { *new_next_slot = sorted_payload.seq + 1; } }));
    let r_next_slot_after_processing_payloads := H.bfold
      (fun new_next_slot (sorted_payload, next_slot) =>
        if sorted_payload.seq = max new_next_slot next_slot then sorted_payload.seq + 1 else new_next_slot)
      0 (fun _ _ => True) trivial (H.bcrossSingleton r_sorted_payloads r_next_slot)
    -- // Find all payloads that can and cannot be processed in this tick.
    -- let r_processable_payloads = r_sorted_payloads.clone()
    --   .cross_singleton(r_next_slot_after_processing_payloads.clone())
    --   .filter(q!(|(sorted_payload, highest_seq)| sorted_payload.seq < *highest_seq))
    --   .map(q!(|(sorted_payload, _)| { sorted_payload }));
    let r_processable_payloads := H.bmap
      (H.bfilter (H.bcrossSingleton r_sorted_payloads r_next_slot_after_processing_payloads)
        (fun (sorted_payload, highest_seq) => decide (sorted_payload.seq < highest_seq)))
      (fun (sorted_payload, _) => sorted_payload)
    -- let r_new_non_processable_payloads = r_sorted_payloads
    --   .cross_singleton(r_next_slot_after_processing_payloads)
    --   .filter(q!(|(sorted_payload, highest_seq)| sorted_payload.seq > *highest_seq))
    --   .map(q!(|(sorted_payload, _)| { sorted_payload }));
    let r_new_non_processable_payloads := H.bmap
      (H.bfilter (H.bcrossSingleton r_sorted_payloads r_next_slot_after_processing_payloads)
        (fun (sorted_payload, highest_seq) => decide (sorted_payload.seq > highest_seq)))
      (fun (sorted_payload, _) => sorted_payload)
    -- // Save these, we can process them once the hole has been filled
    -- r_buffered_payloads_complete_cycle.complete_next_tick(r_new_non_processable_payloads);
    rebind (r_buffered_payloads := r_new_non_processable_payloads)
    -- (r_processable_payloads, r_next_slot_complete_cycle)
    yield (r_processable_payloads := r_processable_payloads);
  r_processable_payloads

#nondet_census sequence_payloads (nondets := 1) (scheds := 0) (fuels := 0)

/-- `kv_replica`'s decisions. -/
structure KvRDec (H : HydroSem L mem) (n : Nat) (K V : Type) [DecidableEq K] [DecidableEq V] where
  /-- `sequence_payloads`'s. -/
  seq : SPaysDec H n K V
  /-- `r_next_slot`'s tick cycle (`cycle_with_initial` + `complete_next_tick`). -/
  fuelNextSlot : H.FixDec
  /-- `r_checkpointed_seqs`' tick cycle (`cycle` + `complete_next_tick`). -/
  fuelCheckpointed : H.FixDec

/-- **mod.rs:46–111 `kv_replica`**: sequence, apply to the KV store,
checkpoint every `checkpoint_frequency` slots, acknowledge to the
clients. Returns (`r_checkpoint_seq_new.all_ticks()`, `r_to_clients`). -/
hydro def kv_replica {K V : Type} [DecidableEq K] [Hashable K] [DecidableEq V] [LinearOrder K] [LinearOrder V]
    (H : HydroSem L mem) (replicas : L)
    (p_to_replicas : H.Stream replicas (Nat × Option (KvPayload K V)) .noOrder .exactlyOnce)
    (checkpoint_frequency : Nat)
    (dec : KvRDec H (mem replicas) K V) :
    (H.Stream replicas Nat .totalOrder .exactlyOnce
      × H.Stream replicas (KvPayload K V) .totalOrder .exactlyOnce) :=
  -- let p_to_replicas: Stream<SequencedKv<K, V>, …, NoOrder> = p_to_replicas.into()
  --   .map(q!(|(slot, kv)| SequencedKv { seq: slot, kv }));
  let p_to_replicas := H.map p_to_replicas (fun _me (slot, kv) => ⟨slot, kv⟩)
  -- let replica_tick = replicas.tick();
  -- let (r_processable_payloads, r_next_slot_complete_cycle) = sequence_payloads(&replica_tick, p_to_replicas);
  -- … r_next_slot_complete_cycle.complete_next_tick(r_next_slot.clone());
  -- let (r_checkpointed_seqs_complete_cycle, r_checkpointed_seqs) = replica_tick.cycle::<Optional<usize, _, _>, _>();
  -- … r_checkpointed_seqs_complete_cycle.complete_next_tick(r_checkpoint_seq_new.clone());
  --   — the two tick cycles, one knot (F)
  fix (r_next_slot : H.Atomic replicas Nat) (r_checkpointed_seqs : H.Atomic replicas (Option Nat))
      via (dec.fuelNextSlot, dec.fuelCheckpointed) :=
    let r_processable_payloads := sequence_payloads H replicas p_to_replicas r_next_slot dec.seq
    -- let r_kv_store = r_processable_payloads.clone().across_ticks(|s| {
    --   s.fold(q!(|| (HashMap::new(), 0)), q!(|(kv_store, next_slot), payload| {
    --     if let Some(kv) = payload.kv { kv_store.insert(kv.key, kv.value); }
    --     *next_slot = payload.seq + 1; })) });
    let r_kv_store := H.tfold
      (fun ((kv_store, _next_slot) : HMap K V × Nat) (payload : SequencedKv K V) =>
        (match payload.kv with
         | some kv => kv_store.insert (kv.key, kv.value)
         | none => kv_store,
         payload.seq + 1))
      (HMap.empty, 0) (fun _ _ => True) trivial r_processable_payloads
    -- // Update the highest seq for the next tick
    -- let r_next_slot = r_kv_store.map(q!(|(_kv_store, next_slot)| next_slot));
    let r_next_slot' := H.mapTick r_kv_store (fun _me (_kv_store, next_slot) => next_slot)
    -- // Send checkpoints to the acceptors when we've processed enough payloads
    -- let r_max_checkpointed_seq = r_checkpointed_seqs.into_stream().across_ticks(|s| s.max()).into_singleton();
    --   — `.into_stream()` of a tick optional: its (zero or one) value as this tick's batch
    let r_max_checkpointed_seq := H.tfold (reduceStep max) none (fun _ _ => True) trivial
      (H.flattenOrdered (H.mapTick r_checkpointed_seqs (fun _me o => o.toList)))
    -- let r_checkpoint_seq_new = r_max_checkpointed_seq
    --   .zip(Optional::from(r_next_slot).defer_tick().unwrap_or(replica_tick.singleton(q!(0))))   — `defer_tick` seeded `0`
    --   .filter_map(q!(move |(max_checkpointed_seq, next_slot)|
    --     if max_checkpointed_seq.map(|m| next_slot - m >= checkpoint_frequency).unwrap_or(true)
    --     { Some(next_slot) } else { None }));
    let r_checkpoint_seq_new := H.mapTick
      (H.zipTick r_max_checkpointed_seq (H.defer_tick 0 r_next_slot'))
      (fun _me (max_checkpointed_seq, next_slot) =>
        if (max_checkpointed_seq.map (fun m => decide (checkpoint_frequency ≤ next_slot - m))).getD true
        then some next_slot else none)
  -- r_next_slot_complete_cycle.complete_next_tick(r_next_slot.clone());
  -- r_checkpointed_seqs_complete_cycle.complete_next_tick(r_checkpoint_seq_new.clone());
  complete (H.defer_tick 0 r_next_slot', H.defer_tick none r_checkpoint_seq_new)
  -- // Tell clients that the payload has been committed. All ReplicaPayloads contain the client's machine ID (to string) as value.
  -- let r_to_clients = r_processable_payloads.filter_map(q!(|payload| payload.kv)).all_ticks();
  --   — `filter_map` after `all_ticks` (the two commute; the stream-level `filterMap`)
  -- (r_checkpoint_seq_new.all_ticks(), r_to_clients)
  (H.allTicks (H.flattenOrdered (H.mapTick r_checkpoint_seq_new (fun _me o => o.toList))),
   H.filterMap (H.allTicks r_processable_payloads) (fun _me payload => payload.kv))

#nondet_census kv_replica (nondets := 1) (scheds := 0) (fuels := 2)

end Hydro
