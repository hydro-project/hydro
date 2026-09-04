import HydroV2.Std.QuorumTheory
import HydroV2.Values
import Mathlib.Data.Multiset.Bind
import HydroV2.HydroDef

/-!
# `hydro_std/src/request_response.rs` — `join_responses`

One Rust function = one Lean definition: join an incoming
request-response stream with metadata generated at request time.

**The atomic causality**: the metadata leg enters via
`use::atomic(metadata.all_ticks_atomic())` — it is synchronized with
the caller's tick, so metadata is available **immediately** (never a
stale batch that misses metadata sent before an async response came
back). In this signature that is a `TickStream` parameter in the
caller's tick domain — *not* a decision; the only `nondet!` is the
response `use::batch` (`BatchCuts`). The usage contract
(request_response.rs's doc): only one response element is produced
with a given key, same for the metadata stream.

Layout: contract → the register machine (core logic) → step
obligations → run facts (`Trace.lean` scan combinators) → the program
definition → smoke tests.
-/

namespace HydroV2

variable {L : Type} {mem : L → Nat}
variable {K M V : Type} [DecidableEq K] [DecidableEq M] [DecidableEq V]

/-- What `join_responses` **ensures**, over the `Values` denotation:
`md` is the per-tick metadata (the caller's atomic tick wire), `dec`
the response batch cuts. -/
structure JREnsures (ℓ : L)
    (resp : Fin (mem ℓ) → Multiset (K × V))
    (md : Fin (mem ℓ) → Trace (Multiset (K × M)))
    (dec : BatchCuts (mem ℓ) (K × V))
    (out : Fin (mem ℓ) → Multiset (K × (M × V))) : Prop where
  /-- **Soundness** (unconditional): every joined output quotes its
  key's metadata and its key's consumed response. -/
  join_src : ∀ (i : Fin (mem ℓ)) {k : K} {m : M} {v : V},
    (k, (m, v)) ∈ out i →
    (k, m) ∈ (md i).sum ∧ (k, v) ∈ cqConsumed (resp i) (dec i)
  /-- **Completeness** (under the once-responder usage contract): a
  consumed response whose metadata was generated at or before its tick
  (the `atomic` availability) joins. -/
  join_complete : ∀ (i : Fin (mem ℓ)) {k : K} {m : M} {v : V},
    (((cqConsumed (resp i) (dec i)).map Prod.fst).count k ≤ 1) →
    ∀ (u : Nat)
      (hu : u < (Trace.zip (batchCuts (resp i) 0 (dec i))
        (md i)).length),
      (k, v) ∈ ((Trace.zip (batchCuts (resp i) 0 (dec i))
        (md i))[u]'hu).1 →
      (k, m) ∈ ((((Trace.zip (batchCuts (resp i) 0 (dec i))
        (md i)).take (u + 1)).map Prod.snd).sum) →
      (k, (m, v)) ∈ out i


/-! ## The `remaining_to_join` register machine (request_response.rs
core logic) -/

/-- One tick of `join_responses` (request_response.rs:19–43),
Rust-literal. State: `remaining_to_join`. Input: the tick's response
batch and metadata batch. -/
def jrTick (s : Multiset (K × M)) (respB : Multiset (K × V))
    (mdB : Multiset (K × M)) :
    Multiset (K × M) × Multiset (K × (M × V)) :=
  -- let remaining_and_new = remaining_to_join.chain(metadata_batch);
  -- let joined_this_tick = remaining_and_new.join(response_batch);
  -- remaining_to_join
  --   = remaining_and_new.anti_join(response_batch.map(key))
  ((s + mdB).filter (fun km => (respB.map Prod.fst).count km.1 = 0),
   respB.bind (fun kv => ((s + mdB).filter
     (fun km => km.1 = kv.1)).map (fun km => (kv.1, (km.2, kv.2)))))

/-! ## Step obligations (per-tick facts about `jrTick`) -/

/-- The register never grows beyond the tick's available metadata. -/
theorem jrTick_state_le (s : Multiset (K × M)) (rb : Multiset (K × V))
    (mb : Multiset (K × M)) : (jrTick s rb mb).1 ≤ s + mb :=
  Multiset.filter_le _ _

/-- A joined output quotes its key's metadata from the tick's
window. -/
theorem jrTick_emit_md (s : Multiset (K × M)) (rb : Multiset (K × V))
    (mb : Multiset (K × M)) {y : K × (M × V)}
    (hy : y ∈ (jrTick s rb mb).2) : (y.1, y.2.1) ∈ s + mb := by
  obtain ⟨kv, hkv, hy2⟩ := Multiset.mem_bind.mp hy
  obtain ⟨km, hkm, rfl⟩ := Multiset.mem_map.mp hy2
  obtain ⟨hmem, hkey⟩ := Multiset.mem_filter.mp hkm
  show ((kv.1, km.2) : K × M) ∈ s + mb
  rw [← hkey]
  exact hmem

/-- A joined output quotes its key's consumed response. -/
theorem jrTick_emit_resp (s : Multiset (K × M)) (rb : Multiset (K × V))
    (mb : Multiset (K × M)) {y : K × (M × V)}
    (hy : y ∈ (jrTick s rb mb).2) : (y.1, y.2.2) ∈ rb := by
  obtain ⟨kv, hkv, hy2⟩ := Multiset.mem_bind.mp hy
  obtain ⟨km, hkm, rfl⟩ := Multiset.mem_map.mp hy2
  exact hkv

/-! ## Run facts (`scan_sound`, instantiated twice; the positional
completeness induction) -/

/-- **Join soundness**: every joined output quotes its key's metadata
(among the metadata seen so far) and its key's consumed response. -/
theorem jr_run_src (k : K) (m : M) (v : V)
    (zbs : List (Multiset (K × V) × Multiset (K × M)))
    (s pfxMd : Multiset (K × M)) (hw : s ≤ pfxMd)
    (h : (k, (m, v)) ∈ (scanAcrossTicksTrace
      (fun s bt => jrTick s bt.1 bt.2) s zbs).sum) :
    (k, m) ∈ pfxMd + ((zbs.map Prod.snd).sum)
      ∧ (k, v) ∈ (zbs.map Prod.fst).sum := by
  constructor
  · exact scan_sound (fun s bt => jrTick s bt.1 bt.2) Prod.snd id
      (fun p (y : K × (M × V)) => (y.1, y.2.1) ∈ p)
      (fun {p p' y} hle hq => Multiset.mem_of_le hle hq)
      (fun s' bt => by exact jrTick_state_le s' bt.1 bt.2)
      (fun s' bt {y} hy => by exact jrTick_emit_md s' bt.1 bt.2 hy)
      zbs s pfxMd hw h
  · have h2 := scan_sound (fun s bt => jrTick s bt.1 bt.2) Prod.fst
      (fun _ => (0 : Multiset (K × V)))
      (fun p (y : K × (M × V)) => (y.1, y.2.2) ∈ p)
      (fun {p p' y} hle hq => Multiset.mem_of_le hle hq)
      (fun s' bt => Multiset.zero_le _)
      (fun s' bt {y} hy => by
        rw [Multiset.zero_add]
        exact jrTick_emit_resp s' bt.1 bt.2 hy)
      zbs s 0 (le_refl 0) h
    rwa [Multiset.zero_add] at h2

omit [DecidableEq M] [DecidableEq V] in
/-- **Join completeness** (under the once-responder usage contract):
if `k` responds at tick `u` and its metadata was generated at or
before `u` (the `atomic` availability), the join emits it. -/
theorem jr_run_complete (k : K) (m : M) (v : V) :
    ∀ (zbs : List (Multiset (K × V) × Multiset (K × M)))
      (s pfxMd : Multiset (K × M)),
      s.filter (fun km => km.1 = k)
        = pfxMd.filter (fun km => km.1 = k) →
      (((zbs.map Prod.fst).sum.map Prod.fst).count k ≤ 1) →
      ∀ (u : Nat) (hu : u < zbs.length),
        (k, v) ∈ (zbs[u]'hu).1 →
        (k, m) ∈ pfxMd + (((zbs.take (u + 1)).map Prod.snd).sum) →
        (k, (m, v)) ∈ (scanAcrossTicksTrace
          (fun s bt => jrTick s bt.1 bt.2) s zbs).sum
  | [], _, _, _, _, u, hu, _, _ => by cases hu
  | zb :: zbs, s, pfxMd, hreg, honce, u, hu, hresp, hmd => by
    rw [show scanAcrossTicksTrace (fun s bt => jrTick s bt.1 bt.2) s
        (zb :: zbs)
        = (jrTick s zb.1 zb.2).2
          :: scanAcrossTicksTrace (fun s bt => jrTick s bt.1 bt.2)
            (jrTick s zb.1 zb.2).1 zbs from rfl,
      List.sum_cons]
    cases u with
    | zero =>
      -- the response tick: the metadata is in remaining_and_new
      refine Multiset.mem_add.mpr (Or.inl ?_)
      have hkm : ((k, m) : K × M) ∈ s + zb.2 := by
        rw [List.take_succ_cons, List.take_zero, List.map_cons,
          List.map_nil, List.sum_cons, List.sum_nil,
          Multiset.add_zero] at hmd
        have hkpart : ((k, m) : K × M)
            ∈ (pfxMd + zb.2).filter (fun km => km.1 = k) :=
          Multiset.mem_filter.mpr ⟨hmd, rfl⟩
        rw [Multiset.filter_add, ← hreg, ← Multiset.filter_add]
          at hkpart
        exact Multiset.mem_of_le (Multiset.filter_le _ _) hkpart
      refine Multiset.mem_bind.mpr ⟨(k, v), hresp, ?_⟩
      refine Multiset.mem_map.mpr
        ⟨(k, m), Multiset.mem_filter.mpr ⟨hkm, rfl⟩, rfl⟩
    | succ u' =>
      -- an earlier tick: the head batch holds no `k` response
      refine Multiset.mem_add.mpr (Or.inr ?_)
      have hresp' : (k, v) ∈ (zbs[u']'(Nat.lt_of_succ_lt_succ hu)).1 :=
        hresp
      have hktail : 1 ≤ ((zbs.map Prod.fst).sum.map Prod.fst).count k :=
        Multiset.count_pos.mpr (Multiset.mem_map.mpr ⟨(k, v),
          mem_list_sum.mpr
            ⟨(zbs[u']'(Nat.lt_of_succ_lt_succ hu)).1,
             List.mem_map.mpr ⟨_, List.getElem_mem _, rfl⟩, hresp'⟩, rfl⟩)
      have hhd0 : ((zb.1.map Prod.fst).count k) = 0 := by
        have hsplit : ((zb :: zbs).map Prod.fst).sum
            = zb.1 + (zbs.map Prod.fst).sum := by
          rw [List.map_cons, List.sum_cons]
        rw [hsplit, Multiset.map_add, Multiset.count_add] at honce
        omega
      have hreg' : ((jrTick s zb.1 zb.2).1).filter
          (fun km => km.1 = k)
          = (pfxMd + zb.2).filter (fun km => km.1 = k) := by
        show (((s + zb.2).filter
            (fun km => (zb.1.map Prod.fst).count km.1 = 0)).filter
            (fun km => km.1 = k)) = _
        rw [filter_key_of_pred _
            (fun k' => (zb.1.map Prod.fst).count k' = 0) k,
          if_pos hhd0, Multiset.filter_add, hreg,
          ← Multiset.filter_add]
      have hmd' : ((k, m) : K × M)
          ∈ (pfxMd + zb.2) + (((zbs.take (u' + 1)).map Prod.snd).sum) := by
        rw [List.take_succ_cons, List.map_cons, List.sum_cons,
          ← Multiset.add_assoc] at hmd
        exact hmd
      exact jr_run_complete k m v zbs (jrTick s zb.1 zb.2).1
        (pfxMd + zb.2) hreg' (by
          have hsplit : ((zb :: zbs).map Prod.fst).sum
              = zb.1 + (zbs.map Prod.fst).sum := by
            rw [List.map_cons, List.sum_cons]
          rw [hsplit, Multiset.map_add, Multiset.count_add] at honce
          omega) u' (Nat.lt_of_succ_lt_succ hu) hresp' hmd'

set_option maxHeartbeats 1000000 in
/-- **request_response.rs:15–43 `join_responses`** over location `ℓ`:
join each consumed response with its request-time metadata; unmatched
metadata persists (`remaining_to_join`). -/
hydro def join_responses (H : HydroSem L mem) (ℓ : L)
    (responses : H.Stream ℓ (K × V) .noOrder .exactlyOnce)
    (metadata : H.TickStream ℓ (K × M) .noOrder .exactlyOnce)
    (dec : H.BatchDec (mem ℓ) (K × V))
    (demit : H.EmitDec (mem ℓ) (K × (M × V))) :
    H.Stream ℓ (K × (M × V)) .noOrder .exactlyOnce
  ensures out => JREnsures ℓ responses metadata dec out :=
  -- let response_batch = use::batch(responses, nondet!(…));
  -- let metadata_batch = use::atomic(metadata.all_ticks_atomic(), …);
  let response_batch := H.batch responses dec
  -- the realized per-member (response cut, metadata) tick pairing
  -- (spec-only)
  ghost let legs := fun i =>
    Trace.zip (batchCuts (responses i) 0 (dec i)) (metadata i)
  -- the `sliced!` register loop (remaining_to_join),
  -- request_response.rs:19–43 Rust-literal — the step is `jrTick` above
  let joined_this_tick := H.scan_batches_unordered₂ response_batch
    metadata (fun _me => jrTick) 0
  H.allTicks (H.emitMultisetBatches joined_this_tick demit)
  prove
    join_src := fun i k m v hk => by
      have h := jr_run_src k m v (legs i) 0 0 (le_refl _) hk
      rw [Multiset.zero_add] at h
      refine ⟨?_, ?_⟩
      · -- metadata legs of the zip sum below the full metadata sum
        refine Multiset.mem_of_le
          (sublist_sum_le (map_snd_zip_prefix ..).sublist) h.1
      · -- response legs of the zip sum below the consumed pool
        exact Multiset.mem_of_le
          (sublist_sum_le (map_fst_zip_prefix ..).sublist) h.2,
    join_complete := fun i k m v honce u hu hresp hmd => by
      refine jr_run_complete k m v (legs i)
        0 0 rfl ?_ u hu hresp (by rw [Multiset.zero_add]; exact hmd)
      -- once-responder transfers from the consumed pool to the zip legs
      refine le_trans (Multiset.count_le_of_le k
        (Multiset.map_le_map
          (sublist_sum_le (map_fst_zip_prefix ..).sublist))) honce

/-! ## Executable smoke tests (mirroring request_response.rs's) -/

private abbrev jrOneLoc : Unit → Nat := fun _ => 1

-- basic join: metadata at tick 0, response at tick 1
#guard (join_responses (Values Unit jrOneLoc) ()
    (fun _ => ({(1, "resp")} : Multiset (Nat × String)))
    (fun _ => [({(1, 42)} : Multiset (Nat × Int)), 0])
    (fun _ => [0, {(1, "resp")}]) ()).val 0
  = ({(1, (42, "resp"))} : Multiset (Nat × (Int × String)))
-- metadata persists: generated two ticks before the response
#guard (join_responses (Values Unit jrOneLoc) ()
    (fun _ => ({(1, "resp")} : Multiset (Nat × String)))
    (fun _ => [({(1, 42)} : Multiset (Nat × Int)), 0, 0])
    (fun _ => [0, 0, {(1, "resp")}]) ()).val 0
  = ({(1, (42, "resp"))} : Multiset (Nat × (Int × String)))
-- no metadata, no join
#guard (join_responses (Values Unit jrOneLoc) ()
    (fun _ => ({(1, "resp")} : Multiset (Nat × String)))
    (fun _ => [(0 : Multiset (Nat × Int)), 0])
    (fun _ => [0, {(1, "resp")}]) ()).val 0
  = (0 : Multiset (Nat × (Int × String)))

end HydroV2
