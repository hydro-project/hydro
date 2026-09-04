import Hydro.Values
import Hydro.HydroTick
import Hydro.HydroDef
import Hydro.HMap
import Hydro.Duration

/-!
# `hydro_std/src/bench_client/mod.rs` — the benchmark client

One Rust function = one Lean definition: `bench_client` (mod.rs:52–128),
`compute_throughput_latency` (mod.rs:130–202) and
`aggregate_bench_results` (mod.rs:204–297). `pretty_print_bench_results`
(mod.rs:299–325) is printing — the shell's (`WiredDemo`), not the
program's.

**The Rust this mirrors is the restructured `bench_client` of D81**
(mod.rs, the latency slice): the original persisted start times in a
root `fold` whose closure called `Instant::now()` (impure: the fold's
value depended on when the runtime ran it) and read the previous tick's
value through `defer_tick()`. The restructure reads the wall clock
through the new `use::time(nondet!(…))` slice style
(`Tick::current_tick_instant`, tick.rs:309 — the slice half hydro #2393
asked for), keeps the start times in a `use::state_null::<KeyedSingleton>`
register, and makes the one-in-flight assumption an
`assume_ordering(nondet!(…))` on the batch instead of a `commutative =
manual_proof!`. Every line below quotes its Rust.

**Closures, flattened (ruling F).** Rust's `bench_client` takes the
workload generator and the protocol as closures and closes the
`protocol_outputs` `forward_ref` itself. Here the module takes the two
wires those closures produce (`protocol_inputs`, `protocol_outputs`) and
returns the wire the generator consumes (`ids_and_prev_payloads`) beside
the latencies; the caller (`paxos_bench`) ties the knot. The let-chain is
otherwise the Rust's.

**The histogram.** `hdrhistogram::Histogram<u64>::new(3)` is **ported**
(hdrhistogram 7.5.4, `lib.rs`): `new_with_bounds(1, 2, 3)` with
`auto_resize` — unit magnitude 0, `sub_bucket_count = 2048`, so a value's
bucket is `bitlen(v) - 11` (saturating), its equivalent range `2^bucket`
and its class `lowest_equivalent(v) = (v >> bucket) << bucket`
(`bucket_for`/`sub_bucket_for`/`value_from_loc`, lib.rs:1571–1606). The
auto-resized counts array is carried **sparsely**: a `HMap` from a
class's lowest equivalent value to its count (an occupied bucket is one
entry — the memory bound hdrhistogram exists for), with `record`,
`add` (per-class count addition, lib.rs:462–501, the bucket geometry
being shared), `len` (`total_count`) and `value_at_quantile`
(lib.rs:1340–1368: the ascending walk to the first class reaching
`ceil(q · total)`, reported at its `highest_equivalent`). Its
serialization (`SerializableHistogramWrapper`, mod.rs:13–36) is the
`HMap` `Wire` instance.
-/

namespace Hydro

variable {L : Type} {mem : L → Nat}

/-! ## `hdrhistogram::Histogram<u64>::new(3)`, ported -/

namespace HdrHistogram

/-- `sub_bucket_count_magnitude` for `sigfig = 3`: `ceil(log2(2 · 10³)) = 11`. -/
def subBucketCountMagnitude : Nat := 11

/-- `bucket_for(v)` (lib.rs:1571): the powers of two by which `v` exceeds
what fits in bucket 0 — `bitlen(v) - sub_bucket_count_magnitude`,
saturating (`leading_zero_count_base - lzcnt(v | sub_bucket_mask)`). -/
def bucketFor (v : Nat) : Nat := Nat.log2 v + 1 - subBucketCountMagnitude

/-- `equivalent_range(v)` (lib.rs:1507): `1 << (unit_magnitude + bucket)`. -/
def equivalentRange (v : Nat) : Nat := 2 ^ bucketFor v

/-- `lowest_equivalent(v)` (lib.rs:1464): `value_from_loc(bucket,
sub_bucket_for(v, bucket))` = `(v >> bucket) << bucket`. -/
def lowestEquivalent (v : Nat) : Nat := (v / equivalentRange v) * equivalentRange v

/-- `highest_equivalent(v)` (lib.rs:1475): `next_non_equivalent(v) - 1`. -/
def highestEquivalent (v : Nat) : Nat := lowestEquivalent v + equivalentRange v - 1

theorem lowestEquivalent_le (v : Nat) : lowestEquivalent v ≤ v := Nat.div_mul_le_self _ _

/-- The class representative keeps the bit length: `2^log2 v ≤ lowest_equivalent v`. -/
theorem pow_log2_le_lowestEquivalent (v : Nat) (hv : v ≠ 0) :
    2 ^ Nat.log2 v ≤ lowestEquivalent v := by
  unfold lowestEquivalent equivalentRange bucketFor subBucketCountMagnitude
  have hb : Nat.log2 v + 1 - 11 ≤ Nat.log2 v := by omega
  have hdvd : 2 ^ (Nat.log2 v + 1 - 11) ∣ 2 ^ Nat.log2 v := Nat.pow_dvd_pow 2 hb
  have hle : 2 ^ Nat.log2 v ≤ v := Nat.log2_self_le hv
  obtain ⟨k, hk⟩ := hdvd
  have hpos : 0 < 2 ^ (Nat.log2 v + 1 - 11) := Nat.two_pow_pos _
  rw [hk] at hle ⊢
  rw [Nat.mul_comm] at hle ⊢
  have : k ≤ v / 2 ^ (Nat.log2 v + 1 - 11) := (Nat.le_div_iff_mul_le hpos).mpr hle
  exact Nat.mul_le_mul_right _ this

/-- Classes are fixed points of `lowest_equivalent` (a stored key is its own class). -/
theorem lowestEquivalent_idem (v : Nat) : lowestEquivalent (lowestEquivalent v) = lowestEquivalent v := by
  by_cases hv : v = 0
  · subst hv; rfl
  have hle := lowestEquivalent_le v
  have hge := pow_log2_le_lowestEquivalent v hv
  have hne : lowestEquivalent v ≠ 0 := by
    intro h0; rw [h0] at hge; exact absurd hge (Nat.not_le.mpr (Nat.two_pow_pos _))
  have hlog : Nat.log2 (lowestEquivalent v) = Nat.log2 v := by
    apply Nat.le_antisymm
    · have : lowestEquivalent v < 2 ^ (Nat.log2 v + 1) :=
        Nat.lt_of_le_of_lt hle (Nat.lt_log2_self (n := v))
      have := (Nat.log2_lt hne).mpr this
      omega
    · exact (Nat.le_log2 hne).mpr hge
  -- same bucket, and the representative is already a multiple of the range
  have hrange : equivalentRange (lowestEquivalent v) = equivalentRange v := by
    unfold equivalentRange bucketFor; rw [hlog]
  generalize hL : lowestEquivalent v = L at hrange ⊢
  have hL' : L = (v / equivalentRange v) * equivalentRange v := hL.symm
  show (L / equivalentRange L) * equivalentRange L = L
  rw [hrange, hL', Nat.mul_div_cancel _ (show 0 < equivalentRange v from Nat.two_pow_pos _)]

/-- The sparse counts array's invariants: no class is stored with count zero
(a zero count IS absence), and every stored key is a class — what makes two
histograms with the same counts the same value. -/
structure Wf (c : HMap Nat Nat) : Prop where
  nozero : ∀ cls, c.get? cls ≠ some 0
  classes : ∀ e ∈ c.toList, lowestEquivalent e.1 = e.1

/-- The count of a class (`count_at`, lib.rs:1452; absent = zero). -/
def countAt (c : HMap Nat Nat) (cls : Nat) : Nat := (c.get? cls).getD 0

/-- `record_n(v, n)` on the counts (lib.rs:885): add `n` to `v`'s class (the
resize is the sparse map's growth); recording nothing changes nothing. -/
def recordN (c : HMap Nat Nat) (v n : Nat) : HMap Nat Nat :=
  if n = 0 then c else
    let cls := lowestEquivalent v
    c.insert (cls, countAt c cls + n)

theorem countAt_recordN (c : HMap Nat Nat) (v n cls : Nat) :
    countAt (recordN c v n) cls
      = if lowestEquivalent v = cls then countAt c cls + n else countAt c cls := by
  unfold recordN
  split
  · subst n; split <;> rfl
  · simp only [countAt, HMap.get?, HMap.insert, Std.ExtHashMap.getElem?_insert, beq_iff_eq]
    split <;> simp_all

theorem Wf.recordN {c : HMap Nat Nat} (hc : Wf c) (v n : Nat) : Wf (recordN c v n) := by
  unfold HdrHistogram.recordN
  split
  · exact hc
  · refine ⟨fun cls => ?_, fun e he => ?_⟩
    · simp only [HMap.get?, HMap.insert, Std.ExtHashMap.getElem?_insert, beq_iff_eq]
      split
      · intro he; simp only [Option.some.injEq] at he; omega
      · exact hc.nozero cls
    · rw [HMap.mem_toList, HMap.mem_def] at he
      simp only [HMap.insert, Std.ExtHashMap.getElem?_insert, beq_iff_eq] at he
      split at he
      · rename_i hk; rw [← hk]; exact lowestEquivalent_idem v
      · exact hc.classes e (HMap.mem_toList.mpr he)

theorem countAt_ext {a b : HMap Nat Nat} (ha : Wf a) (hb : Wf b)
    (h : ∀ cls, countAt a cls = countAt b cls) : a = b := by
  apply HMap.ext'
  intro cls
  have h1 := h cls
  have h2 := ha.nozero cls
  have h3 := hb.nozero cls
  simp only [countAt, HMap.get?] at *
  cases ha' : a.map[cls]? <;> cases hb' : b.map[cls]? <;> simp_all <;> omega

/-- The count a list of (class, count) entries contributes to a class. -/
def contrib (l : List (Nat × Nat)) (cls : Nat) : Nat :=
  (l.filter (fun e => lowestEquivalent e.1 = cls)).foldl (fun acc e => acc + e.2) 0

theorem foldl_add_shift (l : List (Nat × Nat)) (a b : Nat) :
    l.foldl (fun acc e => acc + e.2) (a + b) = a + l.foldl (fun acc e => acc + e.2) b := by
  induction l generalizing a b with
  | nil => rfl
  | cons x l ih => simp only [List.foldl_cons]; rw [Nat.add_assoc, ih]

theorem contrib_cons (e : Nat × Nat) (l : List (Nat × Nat)) (cls : Nat) :
    contrib (e :: l) cls = (if lowestEquivalent e.1 = cls then e.2 else 0) + contrib l cls := by
  unfold contrib
  simp only [List.filter_cons]
  by_cases h : lowestEquivalent e.1 = cls
  · simp only [h, decide_true, if_true, List.foldl_cons, Nat.zero_add]
    have := foldl_add_shift (l.filter (fun e => decide (lowestEquivalent e.1 = cls))) e.2 0
    rw [Nat.add_zero] at this
    exact this
  · simp [h]

theorem contrib_append (l l' : List (Nat × Nat)) (cls : Nat) :
    contrib (l ++ l') cls = contrib l cls + contrib l' cls := by
  induction l with
  | nil => simp [contrib]
  | cons e l ih => rw [List.cons_append, contrib_cons, contrib_cons, ih, Nat.add_assoc]

/-- Folding entries in by `recordN` adds their contribution at every class. -/
theorem countAt_foldl_recordN (l : List (Nat × Nat)) (c : HMap Nat Nat) (cls : Nat) :
    countAt (l.foldl (fun h (e : Nat × Nat) => recordN h e.1 e.2) c) cls = countAt c cls + contrib l cls := by
  induction l generalizing c with
  | nil => simp [contrib]
  | cons e l ih =>
    rw [List.foldl_cons, ih, countAt_recordN, contrib_cons]
    split <;> omega

theorem Wf.foldl_recordN (l : List (Nat × Nat)) {c : HMap Nat Nat} (hc : Wf c) :
    Wf (l.foldl (fun h (e : Nat × Nat) => HdrHistogram.recordN h e.1 e.2) c) := by
  induction l generalizing c with
  | nil => exact hc
  | cons e l ih => exact ih (hc.recordN e.1 e.2)

theorem Wf.empty : Wf (HMap.empty : HMap Nat Nat) :=
  ⟨fun cls => by simp [HMap.get?, HMap.empty],
   fun e he => by
     rw [HMap.mem_toList] at he
     exact absurd he (HMap.not_mem_empty e)⟩

/-- The entries of a well-formed map at a class are exactly its count there. -/
theorem contrib_toList {c : HMap Nat Nat} (hc : Wf c) (cls : Nat) : contrib c.toList cls = countAt c cls := by
  -- every listed entry's key is its own class, so the filter is on the key
  have hfilt : c.toList.filter (fun e => lowestEquivalent e.1 = cls) = c.toList.filter (fun e => e.1 = cls) := by
    apply List.filter_congr
    intro e he
    rw [hc.classes e he]
  unfold contrib
  rw [hfilt]
  have hmem : ∀ e, e ∈ c.toList ↔ c.get? e.1 = some e.2 := fun e => HMap.mem_toList
  cases hget : c.get? cls with
  | none =>
    have : c.toList.filter (fun e => e.1 = cls) = [] := by
      rw [List.filter_eq_nil_iff]
      intro e he hk
      simp only [decide_eq_true_eq] at hk
      rw [hmem, hk, hget] at he
      cases he
    rw [this]
    simp [countAt, hget]
  | some n =>
    -- the one entry at `cls`
    have hin : (cls, n) ∈ c.toList := (hmem (cls, n)).mpr hget
    have huniq : ∀ e ∈ c.toList, e.1 = cls → e = (cls, n) := by
      intro e he hk
      have := (hmem e).mp he
      rw [hk, hget] at this
      exact Prod.ext hk (Option.some.inj this).symm
    have hnd : (c.toList.map Prod.fst).Nodup := by
      obtain ⟨⟨q⟩⟩ := c
      induction q using Std.ExtDHashMap.inductionOn with
      | mk h =>
        rw [HMap.toList_mk]
        exact ((List.mergeSort_perm _ _).map Prod.fst).nodup_iff.mpr (HMap.toList_keys_nodup h)
    have hsingle : c.toList.filter (fun e => e.1 = cls) = [(cls, n)] := by
      have hsub : ∀ e ∈ c.toList.filter (fun e => e.1 = cls), e = (cls, n) := by
        intro e he
        obtain ⟨he, hk⟩ := List.mem_filter.mp he
        exact huniq e he (by simpa using hk)
      have hmemf : (cls, n) ∈ c.toList.filter (fun e => e.1 = cls) :=
        List.mem_filter.mpr ⟨hin, by simp⟩
      have hndf : (c.toList.filter (fun e => e.1 = cls)).Nodup :=
        (hnd.of_map Prod.fst).filter _
      exact List.eq_replicate_of_mem hsub ▸ by
        have hlen : (c.toList.filter (fun e => e.1 = cls)).length = 1 := by
          have h1 := List.nodup_replicate.mp (List.eq_replicate_of_mem hsub ▸ hndf)
          have h0 : 0 < (c.toList.filter (fun e => e.1 = cls)).length := List.length_pos_of_mem hmemf
          omega
        rw [List.eq_replicate_of_mem hsub, hlen]
        rfl
    rw [hsingle]
    simp [countAt, hget]

end HdrHistogram

/-- `Histogram<u64>` (3 significant figures, auto-resizing): the sparse
counts array — each occupied equivalence class, keyed by its lowest
equivalent value, with its (nonzero) count. -/
structure Histogram where
  counts : HMap Nat Nat
  wf : HdrHistogram.Wf counts

namespace Histogram
open HdrHistogram

instance : DecidableEq Histogram := fun a b =>
  decidable_of_iff (a.counts = b.counts)
    ⟨fun h => by cases a; cases b; cases h; rfl, fun h => h ▸ rfl⟩

theorem ext {a b : Histogram} (h : ∀ cls, countAt a.counts cls = countAt b.counts cls) : a = b := by
  cases a; cases b
  simp only [Histogram.mk.injEq]
  exact countAt_ext ‹_› ‹_› h

/-- `Histogram::<u64>::new(3).unwrap()`: no counts. -/
def empty : Histogram := ⟨HMap.empty, Wf.empty⟩

/-- `record_n(v, n)` (lib.rs:885). -/
def recordN (h : Histogram) (v n : Nat) : Histogram :=
  ⟨HdrHistogram.recordN h.counts v n, h.wf.recordN v n⟩

/-- `.record(latency.as_nanos() as u64).unwrap()`. -/
def record (h : Histogram) (v : Nat) : Histogram := h.recordN v 1

/-- `old.add(&new)` (lib.rs:462–501): per class, add the source's count
(the bucket geometry is shared, so classes correspond). -/
def add (old new : Histogram) : Histogram :=
  ⟨new.counts.toList.foldl (fun h (e : Nat × Nat) => HdrHistogram.recordN h e.1 e.2) old.counts,
   Wf.foldl_recordN _ old.wf⟩

/-- `.len()`: `total_count`. -/
def len (h : Histogram) : Nat := (h.counts.toList.map Prod.snd).sum

/-- `.value_at_quantile(num/den)` (lib.rs:1340): the first class, ascending,
whose cumulative count reaches `ceil(q · total)` (at least 1), at its
`highest_equivalent`; `0` on an empty histogram (the Rust fall-through). -/
def valueAtQuantile (h : Histogram) (num den : Nat) : Nat :=
  let total := len h
  let countAtQuantile := max 1 ((num * total + den - 1) / den)
  let rec walk : List (Nat × Nat) → Nat → Nat
    | [], _ => 0
    | (cls, n) :: rest, acc =>
      if countAtQuantile ≤ acc + n then highestEquivalent cls else walk rest (acc + n)
  walk h.counts.toList 0

theorem countAt_record (h : Histogram) (v cls : Nat) :
    countAt (h.record v).counts cls
      = if lowestEquivalent v = cls then countAt h.counts cls + 1 else countAt h.counts cls :=
  countAt_recordN _ _ _ _

theorem countAt_add (a b : Histogram) (cls : Nat) :
    countAt (add a b).counts cls = countAt a.counts cls + countAt b.counts cls := by
  show countAt (b.counts.toList.foldl (fun h (e : Nat × Nat) => HdrHistogram.recordN h e.1 e.2) a.counts) cls = _
  rw [countAt_foldl_recordN, contrib_toList b.wf]

/-- Recording commutes: the fold obligation of the per-tick histogram fold
(`commutative = manual_proof!(/** adding elements to histogram is commutative */)`). -/
theorem record_comm : FoldOkP .noOrder .exactlyOnce (fun (_ _ : Nat) => True) record :=
  fun s x y _ => Histogram.ext fun cls => by
    simp only [countAt_record]
    split <;> split <;> omega

/-- Merging commutes: the reduce obligation of the aggregator
(`commutative = manual_proof!(/** Merge is commutative */)`). -/
theorem add_comm_ok : FoldOkP .noOrder .exactlyOnce (fun (_ _ : Histogram) => True) (reduceStep add) :=
  fun s x y _ => by
    cases s with
    | none =>
      show some (add x y) = some (add y x)
      congr 1
      exact Histogram.ext fun cls => by rw [countAt_add, countAt_add, Nat.add_comm]
    | some h =>
      show some (add (add h x) y) = some (add (add h y) x)
      congr 1
      exact Histogram.ext fun cls => by simp only [countAt_add]; omega

/-- The wire form (`SerializableHistogramWrapper`, mod.rs:13–36) is the counts listing;
decoding normalizes (the Rust's `histogram.auto(true)` side — here, re-recording every
entry, which drops zeros and re-classes keys). -/
def ofCounts (c : HMap Nat Nat) : Histogram :=
  ⟨c.toList.foldl (fun h (e : Nat × Nat) => HdrHistogram.recordN h e.1 e.2) HMap.empty,
    Wf.foldl_recordN _ Wf.empty⟩

/-- Re-recording a histogram's own counts gives it back. -/
theorem ofCounts_counts (h : Histogram) : ofCounts h.counts = h := by
  apply Histogram.ext
  intro cls
  show countAt (h.counts.toList.foldl (fun c (e : Nat × Nat) => HdrHistogram.recordN c e.1 e.2) HMap.empty) cls = _
  rw [countAt_foldl_recordN, contrib_toList h.wf]
  have : countAt (HMap.empty : HMap Nat Nat) cls = 0 := by simp [countAt, HMap.get?, HMap.empty]
  rw [this, Nat.zero_add]

instance : Wire Histogram := Wire.ofEquiv Histogram.counts ofCounts ofCounts_counts

end Histogram

/-! ## `bench_client` (mod.rs:52–128) -/

/-- `bench_client`'s decisions (one per `nondet!` site). -/
structure BCDec (H : HydroSem L mem) (n : Nat) (Input Output : Type) where
  /-- `use::snapshot(num_clients_per_node, nondet!(/** This is a constant */))`
  — the ticks the virtual-client spawner runs. -/
  spawnTicks : H.TicksDec n
  /-- `use::time(nondet!(/** Latencies are measured against the wall clock. */))`. -/
  clock : H.ClockDec n
  /-- `use::batch(protocol_inputs.entries(), nondet!(…))`. -/
  inputs : H.BatchDec n (Nat × Input) .noOrder .exactlyOnce
  /-- `.assume_ordering(nondet!(/** one in flight per virtual client */))` on the new inputs. -/
  inputsOrder : H.TickOrderSelDec n (Nat × Input)
  /-- `use::batch(protocol_outputs.entries(), nondet!(…))`. -/
  outputs : H.BatchDec n (Nat × Output) .noOrder .exactlyOnce
  /-- `.assume_ordering(nondet!(/** one in flight per virtual client */))` on the outputs. -/
  outputsOrder : H.TickOrderSelDec n (Nat × Output)

/-- **mod.rs:52–128 `bench_client`**: virtual clients (`u32` ids), one
request in flight each; returns (`ids_and_prev_payloads` — the workload
generator's input: new virtual-client ids with `None`, completed outputs
with `Some`; `latencies` — per completed request, `(output, duration)`).
Latencies are nanoseconds (`Duration`). -/
hydro def bench_client {Input Output : Type} [DecidableEq Input] [DecidableEq Output]
    (H : HydroSem L mem) (clients : L)
    (num_clients_per_node : Nat)
    (protocol_inputs : H.KeyedStream clients Nat Input .noOrder .exactlyOnce)
    (protocol_outputs : H.KeyedStream clients Nat Output .noOrder .exactlyOnce)
    (dec : BCDec H (mem clients) Input Output) :
    (H.KeyedStream clients Nat (Option Output) .noOrder .exactlyOnce
      × H.KeyedStream clients Nat (Output × Nat) .noOrder .exactlyOnce) :=
  -- let new_payload_ids = sliced! {
  --   let num_clients_per_node = use::snapshot(num_clients_per_node, nondet!(/** This is a constant */));
  --   let mut next_virtual_client = use::state(|l| Optional::from(l.singleton(q!((0u32, None)))));
  tick (state next_virtual_client : Option (Nat × Option Output) := some (0, none))
      (input num_clients_per_node := H.snapshot_bounded num_clients_per_node dec.spawnTicks) :=
    -- // Set up virtual clients - spawn new ones each tick until we reach the limit
    -- let new_virtual_client = next_virtual_client.clone();
    let new_virtual_client := next_virtual_client
    -- next_virtual_client = new_virtual_client.clone().zip(num_clients_per_node)
    --   .filter_map(q!(move |((virtual_id, _), num_clients_per_node)| {
    --     if virtual_id + 1 < num_clients_per_node as u32 { Some((virtual_id + 1, None)) } else { None } }));
    rebind (next_virtual_client := H.boFilterMap
      (H.boZipSingleton new_virtual_client num_clients_per_node)
      (fun ((virtual_id, _), num_clients_per_node) =>
        if virtual_id + 1 < num_clients_per_node then some (virtual_id + 1, none) else none))
    -- new_virtual_client.into_stream().into_keyed()
    kyield (new_payload_ids := H.binto_keyed (H.boIntoStream new_virtual_client));
  -- };
  -- let (protocol_outputs_complete, protocol_outputs) = clients.forward_ref::<…>();   — the caller's knot (F)
  -- // Use new payload IDS and previous outputs to generate new payloads
  -- let protocol_inputs = workload_generator(
  --   new_payload_ids.merge_unordered(protocol_outputs.map(q!(|payload| Some(payload)))));
  --   — `merge_unordered` is the union of the pair streams, regrouped
  let ids_and_prev_payloads := H.into_keyed (H.union
    (H.kentries (H.kallTicks new_payload_ids))
    (H.kentries (H.kmap protocol_outputs (fun _me payload => some payload))))
  -- // Feed new payloads to the protocol
  -- let protocol_outputs = protocol(protocol_inputs.clone());
  -- protocol_outputs_complete.complete(protocol_outputs.clone());        — the caller's (F)
  -- sliced! {
  --   let new_inputs = use::batch(protocol_inputs.entries(), nondet!(/** Batching is required to stamp each input with its start time, but does not affect the result. */));
  --   let current_outputs = use::batch(protocol_outputs.entries(), nondet!(/** Batching is required to compare output to input time, but does not actually affect the result. */));
  --   let now = use::time(nondet!(/** Latencies are measured against the wall clock. */));
  --   // Persisted start times, one per in-flight request. Memory footprint = O(num_clients_per_node)
  --   let mut start_times = use::state_null::<KeyedSingleton<u32, _, _, _>>();
  tick (state start_times : H.BoundedKeyedSingleton Nat Nat)
      (input new_inputs := H.tassume_ordering (H.batch (H.kentries protocol_inputs) dec.inputs) dec.inputsOrder)
      (input current_outputs := H.tassume_ordering (H.batch (H.kentries protocol_outputs) dec.outputs) dec.outputsOrder)
      (input now := H.time dec.clock) :=
    -- let new_inputs = new_inputs
    --   .assume_ordering::<TotalOrder>(nondet!(/** Only one in-flight message per virtual client at any time, so each batch holds at most one input per key. */))
    --   .into_keyed();   — the order selection is the input's (`tassume_ordering`), the cast here
    let new_inputs := H.binto_keyed new_inputs
    -- let current_outputs = current_outputs
    --   .assume_ordering::<TotalOrder>(nondet!(/** Only one in-flight message per virtual client at any time, and they are causally dependent, so this just casts to KeyedSingleton */))
    --   .into_keyed();
    let current_outputs := H.binto_keyed current_outputs
    -- let end_times_and_output = current_outputs.cross_singleton(now.clone())
    --   .reduce(q!(|curr, new| { *curr = new; }));
    let end_times_and_output := H.bkreduce (fun _curr new => new) (fun _ _ => True) trivial
      (H.bkcrossSingleton current_outputs now)
    -- let latencies = start_times.clone().join_keyed_singleton(end_times_and_output.clone())
    --   .map(q!(|(start_time, (output, end_time))| (output, end_time.duration_since(start_time))));
    let latencies := H.bksMap (H.bksJoin start_times end_times_and_output)
      (fun (start_time, (output, end_time)) => (output, end_time - start_time))
    -- let new_start_times = new_inputs.cross_singleton(now)
    --   .map(q!(|(_input, start_time)| start_time))
    --   .reduce(q!(|curr, new| { *curr = new; }));
    let new_start_times := H.bkreduce (fun _curr new => new) (fun _ _ => True) trivial
      (H.bkmap (H.bkcrossSingleton new_inputs now) (fun (_input, start_time) => start_time))
    -- // Outputs complete their requests; new inputs start the next ones
    -- start_times = start_times.filter_key_not_in(end_times_and_output.keys())
    --   .into_keyed_stream().chain(new_start_times.into_keyed_stream())
    --   .reduce(q!(|curr, new| { *curr = new; }));
    rebind (start_times := H.bkreduce (fun _curr new => new) (fun _ _ => True) trivial
      (H.bkchain
        (H.bksIntoStream (H.bksFilterKeyNotIn start_times (H.bksKeys end_times_and_output)))
        (H.bksIntoStream new_start_times)))
    -- latencies.into_keyed_stream().weaken_ordering()
    kyield (latencies := H.bkweakenOrder (H.bksIntoStream latencies));
  -- }
  (ids_and_prev_payloads, H.kallTicks latencies)

#nondet_census bench_client (nondets := 6) (scheds := 0) (fuels := 0)

/-! ## `compute_throughput_latency` (mod.rs:130–202) -/

/-- `compute_throughput_latency`'s decisions: its two `use::batch` sites. -/
structure CTLDec (H : HydroSem L mem) (n : Nat) where
  /-- `use::batch(punctuation, nondet!(nondet_measurement_window))` (mod.rs:163). -/
  punctuation : H.BatchDec n Unit .totalOrder .atLeastOnce
  /-- `use::batch(latencies, nondet!(nondet_measurement_window))` (mod.rs:167). -/
  latencies : H.BatchDec n Nat .noOrder .exactlyOnce

/-- `compute_throughput_latency`'s sched-det bundle: the measurement-window timer. -/
structure CTLSched (H : HydroSem L mem) (n : Nat) where
  /-- `clients.source_interval(q!(Duration::from_millis(interval_millis)))` (mod.rs:160). -/
  punctuation : H.IntervalDec n

/-- The trivial bundle at the denotation. -/
def CTLSched.triv {n : Nat} : CTLSched (Values L mem) n := ⟨()⟩

/-- **mod.rs:130–202 `compute_throughput_latency`**: per client, every
measurement window, the window's request count and latency histogram.
Returns `BenchResult { throughput, latency_histogram }` as the pair
(`interval_throughput`, `interval_latency`). -/
hydro def compute_throughput_latency (H : HydroSem L mem) (clients : L)
    (latencies : H.Stream clients Nat .noOrder .exactlyOnce)
    (interval_millis : Nat)
    (dec : CTLDec H (mem clients)) (sched : CTLSched H (mem clients)) :
    (H.Stream clients Nat .totalOrder .exactlyOnce
      × H.Stream clients Histogram .totalOrder .exactlyOnce) :=
  -- let punctuation = clients.source_interval(q!(Duration::from_millis(interval_millis)));
  let punctuation := H.source_interval (fun _ => 0) (Duration.from_millis interval_millis) sched.punctuation
  -- let (interval_throughput, interval_latency) = sliced! {
  --   let punctuation = use::batch(punctuation, nondet!(nondet_measurement_window));
  --   let latencies = use::batch(latencies, nondet!(nondet_measurement_window));
  --   let mut latency_histogram = use::state(|l| l.singleton(q!(Rc::new(RefCell::new(Histogram::<u64>::new(3).unwrap())))));
  --   let mut throughput = use::state(|l| l.singleton(q!(0usize)));
  tick (state latency_histogram : Histogram := Histogram.empty) (state throughput : Nat := 0)
      (input punctuation := H.batch punctuation dec.punctuation)
      (input latencies := H.batch latencies dec.latencies) :=
    -- let punctuation_option = punctuation.first();
    let punctuation_option := H.bfirst punctuation
    -- let batched_latency_histogram = latencies.clone().fold(
    --   q!(move || Histogram::<u64>::new(3).unwrap()),
    --   q!(move |latencies, latency| { latencies.record(latency.as_nanos() as u64).unwrap(); },
    --      commutative = manual_proof!(/** adding elements to histogram is commutative */)));
    let batched_latency_histogram := H.bfold Histogram.record Histogram.empty (fun _ _ => True)
      Histogram.record_comm latencies
    -- // Output every punctuation
    -- let interval_throughput = throughput.clone().filter_if(punctuation_option.clone().is_some());
    let interval_throughput := H.bsFilterIf throughput (H.boIsSome punctuation_option)
    -- let interval_latency = latency_histogram.clone().filter_if(punctuation_option.clone().is_some());
    let interval_latency := H.bsFilterIf latency_histogram (H.boIsSome punctuation_option)
    -- let batched_throughput = latencies.count();
    let batched_throughput := H.bcount latencies
    -- // Clear every punctuation
    -- let prev_throughput = throughput.filter_if(punctuation_option.clone().is_none());
    let prev_throughput := H.bsFilterIf throughput (H.boIsNone punctuation_option)
    -- // Clear every punctuation
    -- let prev_histogram = latency_histogram.filter_if(punctuation_option.is_none());
    let prev_histogram := H.bsFilterIf latency_histogram (H.boIsNone punctuation_option)
    -- // Merge new values
    -- throughput = batched_throughput.clone().zip(prev_throughput.clone())
    --   .map(q!(|(new, old)| new + old)).unwrap_or(batched_throughput.clone());
    -- // Merge new values
    -- latency_histogram = batched_latency_histogram.clone().zip(prev_histogram.clone())
    --   .map(q!(|(new, old)| { old.borrow_mut().add(new).expect("…"); old }))
    --   .unwrap_or(batched_latency_histogram.map(q!(|histogram| Rc::new(RefCell::new(histogram)))));
    rebind (latency_histogram := H.boUnwrapOr
        (H.boMap (H.bsZipOptional batched_latency_histogram prev_histogram)
          (fun (new, old) => Histogram.add old new))
        batched_latency_histogram,
      throughput := H.boUnwrapOr
        (H.boMap (H.bsZipOptional batched_throughput prev_throughput) (fun (new, old) => new + old))
        batched_throughput)
    -- (interval_throughput.into_stream(), interval_latency.into_stream())
    yield (interval_throughput := H.boIntoStream interval_throughput,
      interval_latency := H.boIntoStream interval_latency);
  -- };
  -- BenchResult { latency_histogram: interval_latency, throughput: interval_throughput }
  (H.allTicks interval_throughput, H.allTicks interval_latency)

#nondet_census compute_throughput_latency (nondets := 2) (scheds := 1) (fuels := 0)

/-! ## `aggregate_bench_results` (mod.rs:204–297) -/

/-- `aggregate_bench_results`'s decisions: its three `use::batch` sites. -/
structure ABRDec (H : HydroSem L mem) (nA : Nat) where
  /-- `use::batch(punctuation, nondet!(nondet_sampling))` (mod.rs:251). -/
  punctuation : H.BatchDec nA Unit .totalOrder .atLeastOnce
  /-- `use::batch(a_throughputs, nondet!(nondet_sampling))`. -/
  throughputs : H.BatchDec nA Nat .noOrder .exactlyOnce
  /-- `use::batch(a_latencies, nondet!(nondet_sampling))`. -/
  latencies : H.BatchDec nA Histogram .noOrder .exactlyOnce

/-- `aggregate_bench_results`'s adversary-side (sched-det) bundle: the two
`send(aggregator, TCP.fail_stop().bincode())` transports. -/
structure ABRSched (H : HydroSem L mem) (nC nA : Nat) where
  throughputs : H.TransportDec nA nC
  latencies : H.TransportDec nA nC
  /-- `aggregator.source_interval(q!(Duration::from_millis(output_interval_millis)))` (mod.rs:232). -/
  punctuation : H.IntervalDec nA

/-- The trivial bundle at the denotation. -/
def ABRSched.triv {nC nA : Nat} : ABRSched (Values L mem) nC nA := ⟨(), (), ()⟩

/-- **mod.rs:204–297 `aggregate_bench_results`**: the clients' window
results, summed at the aggregator process every output interval.
Returns `BenchResult { throughput, latency_histogram }` as the pair
(`combined_throughputs`, `combined_latencies`). -/
hydro def aggregate_bench_results (H : HydroSem L mem) (clients aggregator : L)
    (results_throughput : H.Stream clients Nat .totalOrder .exactlyOnce)
    (results_latency_histogram : H.Stream clients Histogram .totalOrder .exactlyOnce)
    (output_interval_millis : Nat)
    (dec : ABRDec H (mem aggregator))
    (sched : ABRSched H (mem clients) (mem aggregator)) :
    (H.Stream aggregator Nat .totalOrder .exactlyOnce
      × H.Stream aggregator Histogram .totalOrder .exactlyOnce) :=
  -- let punctuation = aggregator.source_interval(q!(Duration::from_millis(output_interval_millis)));
  let punctuation := H.source_interval (fun _ => 0) (Duration.from_millis output_interval_millis) sched.punctuation
  -- let a_throughputs = results.throughput.send(aggregator, TCP.fail_stop().bincode()).values();
  --   — `send` to a `Process` is the broadcast to its one member
  let a_throughputs := H.values (H.broadcast_closed sched.throughputs results_throughput)
  -- let a_latencies = results.latency_histogram
  --   .map(q!(|latencies| SerializableHistogramWrapper { histogram: latencies }))   — the codec
  --   .send(aggregator, TCP.fail_stop().bincode()).values()
  --   .map(q!(|wrapper| wrapper.histogram));
  let a_latencies := H.values (H.broadcast_closed sched.latencies results_latency_histogram)
  -- let (combined_throughputs, combined_latencies) = sliced! {
  --   let punctuation = use::batch(punctuation, nondet!(nondet_sampling));
  let punctuation_batch := H.batch punctuation dec.punctuation
  --   let punctuation_option = punctuation.first();   (:266 — the tick-level `first`, read
  --   inside the slice below and deferred for the reset)
  let punctuation_option := H.firstTick punctuation_batch
  --   let a_throughputs = use::batch(a_throughputs, nondet!(nondet_sampling));
  --   let a_latencies = use::batch(a_latencies, nondet!(nondet_sampling));
  --   let mut latency_histogram = use::state(|l| l.singleton(q!(Rc::new(RefCell::new(Histogram::<u64>::new(3).unwrap())))));
  --   let mut throughput = use::state(|l| l.singleton(q!(0usize)));
  tick (state latency_histogram : Histogram := Histogram.empty) (state throughput : Nat := 0)
      (input punctuation_option := punctuation_option)
      (input a_throughputs := H.batch a_throughputs dec.throughputs)
      (input a_latencies := H.batch a_latencies dec.latencies)
      -- punctuation_option.defer_tick().into_singleton()  (:290): the previous tick's punctuation
      (input reset := H.defer_tick none punctuation_option) :=
    -- // Output every punctuation
    -- let interval_throughput = throughput.clone().filter_if(punctuation_option.clone().is_some());
    let interval_throughput := H.bsFilterIf throughput (H.boIsSome punctuation_option)
    -- let interval_latency = latency_histogram.clone().filter_if(punctuation_option.clone().is_some());
    let interval_latency := H.bsFilterIf latency_histogram (H.boIsSome punctuation_option)
    -- // Clear every punctuation
    -- let prev_throughput = throughput.filter_if(punctuation_option.clone().is_none()).into_stream();
    let prev_throughput := H.boIntoStream (H.bsFilterIf throughput (H.boIsNone punctuation_option))
    -- // Merge new values
    -- throughput = a_throughputs.chain(prev_throughput)   — `chain`'s `MinOrder`: `NoOrder`
    --   .fold(q!(|| 0usize), q!(|curr, new| { *curr += new; }, commutative = manual_proof!(/** Addition is commutative */)));
    let throughput' := H.bfold (fun curr new => curr + new) 0 (fun _ _ => True)
      (fun s x y _ => Nat.add_right_comm s x y)
      (H.bchain a_throughputs (H.bweakenOrder prev_throughput))
    -- // Merge new values
    -- let merged_new_histograms = a_latencies.reduce(
    --   q!(|curr, new| { curr.borrow_mut().add(&*new.borrow_mut()).expect("…"); },
    --      commutative = manual_proof!(/** Merge is commutative */)));
    let merged_new_histograms := H.breduce Histogram.add (fun _ _ => True) Histogram.add_comm_ok a_latencies
    -- // Clear every punctuation
    -- latency_histogram = latency_histogram
    --   .zip(merged_new_histograms.into_singleton())        — an `Optional` IS a singleton of an `Option`
    --   .zip(punctuation_option.defer_tick().into_singleton())
    --   .map(q!(|((old, new), reset)| {
    --     if reset.is_some() { old.replace(Histogram::<u64>::new(3).unwrap()); }
    --     if let Some(new) = new { old.borrow_mut().add(&*new.borrow_mut()).expect("…"); }
    --     old }));
    rebind (latency_histogram := H.bsMap (H.bsZip (H.bsZip latency_histogram merged_new_histograms) reset)
        (fun ((old, new), reset) =>
          let old := if reset.isSome then Histogram.empty else old
          match new with
          | some new => Histogram.add old new
          | none => old),
      throughput := throughput')
    -- (interval_throughput.into_stream(), interval_latency.into_stream())
    yield (combined_throughputs := H.boIntoStream interval_throughput,
      combined_latencies := H.boIntoStream interval_latency);
  -- };
  -- BenchResult { throughput: combined_throughputs, latency_histogram: combined_latencies }
  (H.allTicks combined_throughputs, H.allTicks combined_latencies)

#nondet_census aggregate_bench_results (nondets := 3) (scheds := 3) (fuels := 0)

end Hydro
