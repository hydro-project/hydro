import Hydro.MonoRel
import Hydro.Paxos.Types
import Mathlib.Data.Multiset.Filter
import Hydro.HydroTick

/-!
# `acceptor_p2` (paxos.rs:808–899)

Acceptor phase-2 logic: batch the P2as (`nondet!` — safe because the
entries accumulate across ticks under a commutative merge
(`across_ticks` + keyed `reduce_watermark`, paxos.rs:851–864), so
batch boundaries cannot affect the eventual log), qualify them against
`a_max_ballot` (`Some(&p2a.ballot) >= max_ballot`), merge the qualified
entries per slot across ticks (`reduce_watermark`: max by ballot,
collected below the checkpoint), publish the live map's entries zipped
with the checkpoint, and ack each P2a to its sender with `Ok(())` iff
its ballot *is* the current max.

**The `manual_proof!` hole, made honest (D71, R3)**: Rust's per-slot
merge `if entry.ballot > prev.ballot { *prev = entry }` is not
commutative when one slot sees two entries with equal ballots and
different values (paxos.rs:862's `TODO: need assume`). The merge is
transcribed literally (`logMerge`); its commutativity is paid under the
tie prerequisite `LogTie` (`logMerge_comm`), and the content half — the
accumulated entries at one slot are tie-coherent — is this module's
**`requires`** (`P2aFunctional`: the pool is `(slot, ballot) ↦ value`
functional), proven at the operator site by `ghost pre` from the
clause. Upstream this is `commutative = manual_proof!(requires: …)`.

**Watermark GC (`reduce_watermark`'s deletion)** is modeled faithfully:
keys below the running watermark are collected (`KeyedMap.gcBelow`).
The checkpoint input is typed `Monotonic` (Rust's
`SingletonBound::Monotonic`, singleton.rs:39): its snapshots ascend, so
the running watermark IS the published checkpoint and a collected slot
sits below the view's own checkpoint — the shape the proposer's
recommit discipline (slots ≤ the view checkpoint are skipped, holes and
fresh slots start above it) consumes. **Divergence from paxos.rs
noted**: Rust takes `a_checkpoint: Optional<usize, …, Unbounded>`
(`Optional` has no `Monotonic` bound yet, optional.rs); the protocol's
safety genuinely relies on the checkpoint only growing (a checkpoint
that retreats lets an acceptor GC a slot its published view does not
cover — a re-proposal hole), so Rust should be strengthened to demand it
(SCHED_AUDIT). The `.faithful` variant's two-commits run under GC
(fresh payloads indexed from the view's max slot, below the checkpoint:
B4) is a Falsify witness; `.guarded` rebases above the checkpoint.
-/

namespace Hydro

variable {L : Type} {mem : L → Nat} {P : Type} [DecidableEq P]

/-- **The P2a pool is `(slot, ballot) ↦ value` functional** (per
acceptor): two P2as at one slot with one ballot carry one value. The
`requires` of `acceptor_p2` — the content half of paxos.rs:862's
`commutative = manual_proof!(/** max by ballot (TODO: not if two entries
with same ballot, need assume) */)`; `sequence_payload` discharges it
(a leader sends a key once, `hp2a_agree`). -/
def P2aFunctional {nP nA : Nat} (pool : Fin nA → Multiset (P2a P nP)) : Prop :=
  ∀ j, ∀ a ∈ pool j, ∀ b ∈ pool j, a.slot = b.slot → a.ballot = b.ballot → a.value = b.value

/-- **What `acceptor_p2` ensures**, over the `Values` denotation — stated
on its OUTPUT WIRES in the consumer's vocabulary (per-tick reads by
`[t]?`, membership in the input pool), never in this module's own fold
vocabulary: a consumer reads these faces and sees `a_log`, `p2as`,
`a_max_ballot`, `a_checkpoint`, nothing unfolded (FINDINGS D63). The
faces that read the merge's content are conditional on the module's
`requires` (D71, R3: a refused fold has no content to speak of). -/
structure AP2Ensures (acc prop : L)
    (mx : TickV (mem acc) (Option (Ballot (mem prop))))
    (pool : MemberMap (mem acc) (Multiset (P2a P (mem prop))))
    (ck : Fin (mem acc) → Trace (Option Nat))
    (out : TickV (mem acc) (ALog P (mem prop))
      × (MemberMap (mem prop) (Multiset (P2b (mem prop))))) : Prop where
  /-- **The published clock**: the log wire ticks at most as often as the
  checkpoint input (its tick zip). -/
  log_len_le_ck : ∀ j, (out.1 j).length ≤ (ck j).length
  /-- **The published checkpoint** is the tick's checkpoint read. -/
  log_ck : ∀ (j : Fin (mem acc)) {t : Nat} {lt : ALog P (mem prop)},
    (out.1 j)[t]? = some lt → (ck j)[t]? = some lt.1
  /-- **The published checkpoint ascends** (the `Monotonic` input). -/
  log_ck_mono : ∀ (j : Fin (mem acc)) {t t' : Nat} (_ : t ≤ t') {lt lt' : ALog P (mem prop)},
    (out.1 j)[t]? = some lt → (out.1 j)[t']? = some lt' →
    ValueOrder.optNat.le lt.1 lt'.1
  /-- **Coverage ascent, or collection** (under the clause): a slot
  covered at a tick is covered at every later tick, unless collected —
  the later view's checkpoint exceeds it (`reduce_watermark`'s GC). -/
  log_covers_mono : P2aFunctional pool → ∀ (j : Fin (mem acc)) {t t' : Nat} (_ : t ≤ t')
    {lt lt' : ALog P (mem prop)},
    (out.1 j)[t]? = some lt → (out.1 j)[t']? = some lt' →
    ∀ {slot : Nat} {b : Ballot (mem prop)},
      LogCovers lt.2 slot b → LogCoversOrGC lt' slot b
  /-- **Entry provenance**: a published entry at `(slot, b)` IS a consumed
  P2a at that key (the merge keeps one of the slot's entries). -/
  log_entry_src : ∀ (j : Fin (mem acc)) {t : Nat} {lt : ALog P (mem prop)},
    (out.1 j)[t]? = some lt → ∀ {slot : Nat} {e : LogValue P (mem prop)},
    (slot, e) ∈ lt.2 →
    ∃ p2a ∈ pool j, p2a.slot = slot ∧ p2a.ballot = e.ballot ∧ p2a.value = e.value
  /-- **The acks, by acceptor**: proposer `r`'s ack pool is a sum of
  per-acceptor pools; acceptor `j` acks a key at most as often as its
  pool holds P2as at that key; every ack opens to a consumed P2a of the
  sender, and an `Ok` ack pins the tick's max at its ballot and (under
  the clause) the published log's coverage of its key at that very tick
  — or the key's collection below the tick's checkpoint
  (write-before-ack). -/
  acks_by_acceptor : ∀ (r : Fin (mem prop)),
    ∃ byAcc : Fin (mem acc) → Multiset (P2b (mem prop)),
      out.2 r = ((List.finRange (mem acc)).map byAcc).sum
      ∧ (∀ (j : Fin (mem acc)) (s : Nat) (b : Ballot (mem prop)),
          ((byAcc j).filter (fun m => m.slot = s ∧ m.ballot = b)).card
            ≤ (pool j).countP (fun a => a.slot = s ∧ a.ballot = b))
      ∧ ∀ (j : Fin (mem acc)), ∀ m ∈ byAcc j,
          ∃ p2a ∈ pool j, p2a.sender.val = r.val
            ∧ m.slot = p2a.slot ∧ m.ballot = p2a.ballot
            ∧ (m.res = .ok () →
                ∃ t : Nat, (mx j)[t]? = some (some m.ballot)
                  ∧ (P2aFunctional pool → t < (ck j).length →
                      ∃ lt, (out.1 j)[t]? = some lt ∧ LogCoversOrGC lt m.slot m.ballot))
  /-- **Ack verdicts**: every ack compares its P2a's ballot against the
  acking acceptor's max at the ack tick. -/
  ack_src : ∀ (r : Fin (mem prop)), ∀ m ∈ out.2 r,
    ∃ (j : Fin (mem acc)) (t : Nat) (mb : Option (Ballot (mem prop))),
      (mx j)[t]? = some mb
      ∧ ∃ p2a ∈ pool j, p2a.sender.val = r.val
          ∧ m.slot = p2a.slot ∧ m.ballot = p2a.ballot
          ∧ m.res = (if some p2a.ballot = mb then Except.ok () else Except.error mb)

/-- **`acceptor_p2`'s `nondet!` sites** (content nondeterminism): the
P2a batch and the checkpoint snapshot (paxos.rs:819/:829). The
checkpoint's upstream vocabulary is the record's own generics. -/
structure AP2Dec (H : HydroSem L mem) (nP nA : Nat) (P : Type)
    [DecidableEq P] (ckα : Type) [DecidableEq ckα]
    (ckord : StrOrd) where
  /-- `p_to_acceptors_p2a.batch(acceptor_tick, nondet!(…))`. -/
  p2aBatch : H.BatchDec nA (P2a P nP)
  /-- `a_checkpoint.snapshot(acceptor_tick, nondet!(/** delayed GC is
  safe */))` (paxos.rs:829, B2). -/
  ckSnap : H.SnapDec nA ckα ckord

/-- **`acceptor_p2`'s adversary-side (sched-det) bundle** (`Unit` at
`Values`; see `Sem.lean`'s classification table). -/
structure AP2Sched (H : HydroSem L mem) (nP nA : Nat) where
  /-- `a_to_proposers_p2b.demux(&proposers, TCP.fail_stop()…)`: P2b
  delivery cursors (Rust's `demux` carries no `nondet!` — delivery is
  unmarked machine freedom). -/
  p2bCh : H.TransportDec nP nA

/-- The trivial bundle at the denotation. -/
def AP2Sched.triv {nP nA : Nat} : AP2Sched (Values L mem) nP nA := ⟨()⟩

/-- **paxos.rs:808–899 `acceptor_p2`** over the acceptor cluster `acc`,
acking into the proposer cluster `prop`. Returns (`a_log` — the
`(checkpoint, log)` wire for `acceptor_p1` —, `a_to_proposers_p2b`).
The checkpoint input is `Monotonic` (see the header: a deliberate
strengthening of paxos.rs's `Unbounded`).

Rust `nondet!` tally: 2 (the P2a batch = `dec.p2aBatch` and the
checkpoint snapshot = `dec.ckSnap` — paxos.rs:829, B2) — exactly the
census; the P2b `demux` carries no `nondet!` (delivery cursors are
unmarked machine freedom). The checkpoint's upstream vocabulary is
anonymous generics (`ckα`/`ckord`/`ckret`): the module is parametric
in where the async optional came from; the snapshot decision borrows
the same vocabulary. -/
hydro def acceptor_p2 {ckα : Type} [DecidableEq ckα]
    {ckord : StrOrd} {ckret : Retries}
    (H : HydroSem L mem) (acc prop : L)
    (a_max_ballot :
      H.Atomic acc (Option (Ballot (mem prop))))
    (p_to_acceptors_p2a :
      H.Stream acc (P2a P (mem prop)) .noOrder .exactlyOnce)
    (a_checkpoint : H.Singleton acc ckα (Option Nat) ckord ckret
      (.monotonic ValueOrder.optNat))
    (dec : AP2Dec H (mem prop) (mem acc) P ckα ckord)
    (sched : AP2Sched H (mem prop) (mem acc)) :
    (H.Atomic acc (ALog P (mem prop))
      × H.Stream prop (P2b (mem prop)) .noOrder .exactlyOnce)
  -- commutative = manual_proof!(requires: same slot ∧ same ballot ⇒ same value)
  requires P2aFunctional p_to_acceptors_p2a
  ensures out => AP2Ensures acc prop a_max_ballot p_to_acceptors_p2a
    (fun i => (a_checkpoint i).val (dec.ckSnap i)) out :=
  -- .batch(acceptor_tick, nondet!(…))
  let p_to_acceptors_p2a_batch := H.batch p_to_acceptors_p2a
    dec.p2aBatch
  -- a_checkpoint.snapshot(acceptor_tick, nondet!(/** We can arbitrarily
  -- snapshot the checkpoint sequence number, since a delayed garbage
  -- collection does not affect correctness. */))  (paxos.rs:829, B2)
  let a_checkpoint_tick := H.snapshot a_checkpoint dec.ckSnap
  -- let a_p2as_to_place_in_log = p_to_acceptors_p2a_batch.clone()
  --   .cross_singleton(a_max_ballot.clone()) // Don't consider p2as if the current ballot is higher
  --   .filter_map(q!(|(p2a, max_ballot)|
  --     if Some(&p2a.ballot) >= max_ballot.as_ref() {
  --       Some((p2a.slot, LogValue { ballot: p2a.ballot, value: p2a.value }))
  --     } else { None }));
  tick (input p2as := p_to_acceptors_p2a_batch)
      (input mb := a_max_ballot) :=
    yield (a_p2as_to_place_in_log := H.bfilterMap
      (H.bcrossSingleton p2as mb)
      (fun (p2a, max_ballot) =>
        if p2aQualifies max_ballot p2a.ballot then
          some (p2a.slot, (⟨p2a.ballot, p2a.value⟩ : LogValue P (mem prop)))
        else none));
  -- a qualified entry at a tick is a consumed, qualifying P2a of that
  -- tick (the block's reader, with the in-tick `filter_map` opened)
  ghost have hqual_mem : ∀ (j : Fin (mem acc)) {n : Nat}
      {q : Multiset (Nat × LogValue P (mem prop))},
      (a_p2as_to_place_in_log j)[n]? = some q →
      ∀ {en : Nat × LogValue P (mem prop)}, en ∈ q →
      ∃ p2a ∈ p_to_acceptors_p2a j, ∃ mb_t, (a_max_ballot j)[n]? = some mb_t
        ∧ p2aQualifies mb_t p2a.ballot = true
        ∧ en = (p2a.slot, ⟨p2a.ballot, p2a.value⟩) := fun j n q hq en hen => by
    obtain ⟨b_t, mb_t, hb, hmb, rfl⟩ := (ha_p2as_to_place_in_log_at j n q).mp hq
    have hen' : en ∈ Multiset.filterMap
        (fun p2a : P2a P (mem prop) => if p2aQualifies mb_t p2a.ballot then
          some (p2a.slot, (⟨p2a.ballot, p2a.value⟩ : LogValue P (mem prop))) else none)
        b_t := by
      have : en ∈ Multiset.filterMap _ (Multiset.map
          (fun a => (a, show Option (Ballot (mem prop)) from mb_t))
          (show Multiset (P2a P (mem prop)) from b_t)) := hen
      rwa [Multiset.filterMap_map] at this
    obtain ⟨p2a, hp2a, hq⟩ := (Multiset.mem_filterMap _ _).mp hen'
    have hq' : (if p2aQualifies mb_t p2a.ballot then
        some (p2a.slot, (⟨p2a.ballot, p2a.value⟩ : LogValue P (mem prop))) else none)
        = some en := hq
    refine ⟨p2a, mem_pool_of_mem_batch hb hp2a, mb_t, hmb, ?_⟩
    by_cases hc : p2aQualifies mb_t p2a.ballot = true
    · rw [if_pos hc] at hq'
      exact ⟨hc, (Option.some.inj hq').symm⟩
    · rw [if_neg hc] at hq'
      cases hq'
  -- a consumed P2a that qualifies IS a qualified entry of its tick
  ghost have hqual_of : ∀ (j : Fin (mem acc)) {n : Nat}
      {b_t : Multiset (P2a P (mem prop))} {mb_t : Option (Ballot (mem prop))},
      (batchCuts (p_to_acceptors_p2a j) 0 (dec.p2aBatch j))[n]? = some b_t →
      (a_max_ballot j)[n]? = some mb_t →
      ∀ p2a ∈ b_t, p2aQualifies mb_t p2a.ballot = true →
      ∃ q, (a_p2as_to_place_in_log j)[n]? = some q
        ∧ ((p2a.slot, ⟨p2a.ballot, p2a.value⟩) : Nat × LogValue P (mem prop)) ∈ q :=
    fun j n b_t mb_t hb hmb p2a hp2a hqual => by
    refine ⟨_, (ha_p2as_to_place_in_log_at j n _).mpr ⟨b_t, mb_t, hb, hmb, rfl⟩, ?_⟩
    show _ ∈ Multiset.filterMap _ (Multiset.map
      (fun a => (a, show Option (Ballot (mem prop)) from mb_t))
      (show Multiset (P2a P (mem prop)) from b_t))
    rw [Multiset.filterMap_map]
    refine (Multiset.mem_filterMap _ _).mpr ⟨p2a, hp2a, ?_⟩
    show (if p2aQualifies mb_t p2a.ballot then
      some (p2a.slot, (⟨p2a.ballot, p2a.value⟩ : LogValue P (mem prop))) else none) = _
    rw [if_pos hqual]
  -- let a_log = a_p2as_to_place_in_log.across_ticks(|s| {
  --   s.into_keyed().reduce_watermark(a_checkpoint.clone(),
  --     q!(|prev_entry, entry| {
  --       // Insert p2a into the log if it has a higher ballot than what was there before
  --       if entry.ballot > prev_entry.ballot { *prev_entry = LogValue { … } }
  --     }, commutative = manual_proof!(/** max by ballot (TODO: not if two
  --        entries with same ballot, need assume) */)))});
  let a_log_keyed := H.tinto_keyed a_p2as_to_place_in_log
  -- the content half of the `manual_proof!`, at the site: the entries
  -- accumulated at one slot are tie-coherent (every entry quotes a pool
  -- P2a; the clause makes the pool `(slot, ballot) ↦ value` functional)
  ghost pre hpre : ∀ j, (KeyedPool.accum (a_log_keyed j)).Pairwise LogTie := fun j => by
    show (KeyedPool.accum ((a_p2as_to_place_in_log j).map KeyedPool.ofMultiset)).Pairwise LogTie
    rw [KeyedPool.accum_map_ofMultiset, KeyedPool.Pairwise_ofMultiset_iff]
    intro a ha b hb hk hbal
    obtain ⟨qa, hqa, ha'⟩ := mem_list_sum.mp ha
    obtain ⟨ta, hta⟩ := List.mem_iff_getElem?.mp hqa
    obtain ⟨pa, hpa, -, -, -, rfl⟩ := hqual_mem j hta ha'
    obtain ⟨qb, hqb, hb'⟩ := mem_list_sum.mp hb
    obtain ⟨tb, htb⟩ := List.mem_iff_getElem?.mp hqb
    obtain ⟨pb, hpb, -, -, -, rfl⟩ := hqual_mem j htb hb'
    exact hreq j pa hpa pb hpb hk hbal
  let a_log := H.reduce_watermark a_log_keyed a_checkpoint_tick logMerge LogTie logMerge_comm
  -- let a_log_snapshot = a_log.entries().fold(q!(|| HashMap::new()),
  --   q!(|map, (slot, entry)| { map.insert(slot, entry); },
  --     commutative = manual_proof!(/** inserting elements into map is
  --       commutative because no keys will overlap */)));
  -- (`NoKeyOverlap` is structural for a map's entries: the wf normalizer
  -- closes it)
  -- a_checkpoint.into_singleton().zip(a_log_snapshot).latest_atomic()
  tick (input lg := a_log) (input ck := a_checkpoint_tick) :=
    let a_log_snapshot := H.bfold HMap.insert HMap.empty NoKeyOverlap HMap.insert_foldOk
      (H.bksEntries lg)
    emit (a_log_wc := H.bsZip ck a_log_snapshot);
  -- the qualified entries consumed through tick `t`, and the per-slot
  -- group of a slot in them (the merge's content)
  ghost have hE : ∀ (j : Fin (mem acc)) (n : Nat), ∀ en ∈ ((a_p2as_to_place_in_log j).take n).sum,
      ∃ p2a ∈ p_to_acceptors_p2a j, ∃ (t₀ : Nat) (mb_t : Option (Ballot (mem prop))),
        (a_max_ballot j)[t₀]? = some mb_t ∧ p2aQualifies mb_t p2a.ballot = true
        ∧ en = (p2a.slot, ⟨p2a.ballot, p2a.value⟩) := fun j n en hen => by
    obtain ⟨q, hq, hen'⟩ := mem_list_sum.mp hen
    obtain ⟨t, ht⟩ := List.mem_iff_getElem?.mp hq
    rw [List.getElem?_take] at ht
    split at ht
    · obtain ⟨p2a, hp, mb_t, hmb, hq, rfl⟩ := hqual_mem j ht hen'
      exact ⟨p2a, hp, t, mb_t, hmb, hq, rfl⟩
    · cases ht
  -- the groups are tie-coherent under the clause (the `ghost pre`, read
  -- at a prefix)
  ghost have hE_pairwise : P2aFunctional p_to_acceptors_p2a → ∀ (j : Fin (mem acc)) (n : Nat) (slot : Nat),
      PairwiseM LogTie ((((a_p2as_to_place_in_log j).take n).sum.filter
        (fun e => e.1 = slot)).map Prod.snd) := fun hR j n slot => by
    have hp : (KeyedPool.accum ((a_p2as_to_place_in_log j).map KeyedPool.ofMultiset)).Pairwise LogTie :=
      hpre hR j
    rw [KeyedPool.accum_map_ofMultiset, KeyedPool.Pairwise_ofMultiset_iff] at hp
    have hle : ((a_p2as_to_place_in_log j).take n).sum ≤ (a_p2as_to_place_in_log j).sum := by
      conv_rhs => rw [← List.take_append_drop n (a_p2as_to_place_in_log j)]
      rw [List.sum_append]
      exact Multiset.le_add_right _ _
    intro x hx y hy
    obtain ⟨a, ha, rfl⟩ := Multiset.mem_map.mp hx
    obtain ⟨b, hb, rfl⟩ := Multiset.mem_map.mp hy
    obtain ⟨ha, hak⟩ := Multiset.mem_filter.mp ha
    obtain ⟨hb, hbk⟩ := Multiset.mem_filter.mp hb
    exact hp a (Multiset.mem_of_le hle ha) b (Multiset.mem_of_le hle hb) (hak.trans hbk.symm)
  -- **the live map, read at a tick**: tick `t`'s map is the per-slot
  -- merge of the entries consumed through `t`, collected below the
  -- running watermark through `t`
  ghost have hmap_at : ∀ (j : Fin (mem acc)) (t : Nat),
      (a_log j)[t]? = ((Trace.zip (a_p2as_to_place_in_log j) (a_checkpoint_tick j))[t]?).map
        (fun _ => KeyedMap.gcBelow (wmRun ((a_checkpoint_tick j).take (t + 1)))
          (KeyedMap.ofReduce logMerge LogTie logMerge_comm
            (KeyedPool.ofMultiset (((a_p2as_to_place_in_log j).take (t + 1)).sum)))) :=
    fun j t => reduceWatermark_getElem?_ofMultiset logMerge LogTie logMerge_comm _ _ t
  -- the checkpoint snapshots ascend (`Monotonic`): the running watermark
  -- is the tick's own checkpoint
  ghost have hwm : ∀ (j : Fin (mem acc)) (t : Nat) (c : Option Nat),
      (a_checkpoint_tick j)[t]? = some c → wmRun ((a_checkpoint_tick j).take (t + 1)) = c :=
    fun j t c hc => by
    obtain ⟨ht, rfl⟩ := List.getElem?_eq_some_iff.mp hc
    exact wmRun_take_of_ascending ((a_checkpoint j).property (dec.ckSnap j)) t ht
  -- **the published log, read at a tick**: the checkpoint read, with the
  -- sorted entries of the live map (membership = the map's read)
  ghost have hlog_at : ∀ (j : Fin (mem acc)) {t : Nat} {lt : ALog P (mem prop)},
      (a_log_wc j)[t]? = some lt →
        ∃ (q : Multiset (Nat × LogValue P (mem prop))) (c : Option Nat),
          (a_p2as_to_place_in_log j)[t]? = some q ∧ (a_checkpoint_tick j)[t]? = some c
          ∧ lt.1 = c
          ∧ ∀ (slot : Nat) (e : LogValue P (mem prop)), (slot, e) ∈ lt.2 ↔
              (KeyedMap.gcBelow c (KeyedMap.ofReduce logMerge LogTie logMerge_comm
                (KeyedPool.ofMultiset (((a_p2as_to_place_in_log j).take (t + 1)).sum)))).get slot
              = some e := fun j {t lt} h => by
    obtain ⟨M, c, hM, hc, rfl⟩ := (ha_log_wc_at j t lt).mp h
    rw [hmap_at, Option.map_eq_some_iff] at hM
    obtain ⟨⟨q, c'⟩, hx, rfl⟩ := hM
    obtain ⟨hq, hc'⟩ := Trace.getElem?_zip_eq_some.mp hx
    obtain rfl : c' = c := Trace.read_inj hc' hc
    rw [hwm j t _ hc']
    refine ⟨q, c', hq, hc', rfl, fun slot e => ?_⟩
    show (slot, e) ∈ Multiset.foldlOn NoKeyOverlap HMap.insert HMap.empty HMap.insert_foldOk
      (KeyedMap.gcBelow c' (KeyedMap.ofReduce logMerge LogTie logMerge_comm
        (KeyedPool.ofMultiset (((a_p2as_to_place_in_log j).take (t + 1)).sum)))).entries ↔ _
    rw [HMap.mem_foldlOn_insert_empty _ (KeyedMap.entries_pairwise_key _), KeyedMap.mem_entries_iff]
  -- a tick with a qualified batch and a checkpoint read publishes a log
  ghost have hlog_exists : ∀ (j : Fin (mem acc)) {t : Nat}
      {q : Multiset (Nat × LogValue P (mem prop))} {c : Option Nat},
      (a_p2as_to_place_in_log j)[t]? = some q → (a_checkpoint_tick j)[t]? = some c →
      ∃ lt, (a_log_wc j)[t]? = some lt := fun j {t q c} hq hc => by
    have hM : (a_log j)[t]? = some (KeyedMap.gcBelow (wmRun ((a_checkpoint_tick j).take (t + 1)))
        (KeyedMap.ofReduce logMerge LogTie logMerge_comm
          (KeyedPool.ofMultiset (((a_p2as_to_place_in_log j).take (t + 1)).sum)))) := by
      rw [hmap_at, Trace.getElem?_zip_eq_some.mpr ⟨hq, hc⟩]
      rfl
    exact ⟨_, (ha_log_wc_at j t _).mpr ⟨_, c, hM, hc, rfl⟩⟩
  -- **a published entry is a consumed entry** (unconditionally: a refused
  -- group publishes nothing)
  ghost have hentry_src : ∀ (j : Fin (mem acc)) {t : Nat} {lt : ALog P (mem prop)},
      (a_log_wc j)[t]? = some lt → ∀ {slot : Nat} {e : LogValue P (mem prop)},
      (slot, e) ∈ lt.2 → (slot, e) ∈ ((a_p2as_to_place_in_log j).take (t + 1)).sum :=
    fun j {t lt} h {slot e} he => by
    obtain ⟨q, c, -, -, -, hmem⟩ := hlog_at j h
    obtain ⟨-, -, hfold⟩ := (KeyedMap.get_gcBelow_ofReduce_ofMultiset _ _ _ _ _ _ _).mp
      ((hmem slot e).mp he)
    obtain ⟨hv, -⟩ := logReduce_none_some_mem hfold
    obtain ⟨x, hx, rfl⟩ := Multiset.mem_map.mp hv
    obtain ⟨hxE, hxk⟩ := Multiset.mem_filter.mp hx
    obtain ⟨k, v⟩ := x
    have hk : k = slot := hxk
    subst hk
    exact hxE
  -- **a consumed entry is covered, or collected** (under the clause): at
  -- a tick whose consumed entries hold `(slot, e)`, the published log
  -- covers `slot` at `e.ballot` (the slot's merge dominates its group),
  -- unless the tick's checkpoint exceeds the slot
  ghost have hcover_or_collect : P2aFunctional p_to_acceptors_p2a → ∀ (j : Fin (mem acc)) {t : Nat}
      {lt : ALog P (mem prop)}, (a_log_wc j)[t]? = some lt →
      ∀ {slot : Nat} {e : LogValue P (mem prop)},
      (slot, e) ∈ ((a_p2as_to_place_in_log j).take (t + 1)).sum →
      LogCoversOrGC lt slot e.ballot :=
    fun hR j {t lt} h {slot e} he => by
    obtain ⟨q, c, -, -, hlt1, hmem⟩ := hlog_at j h
    by_cases hcoll : ∃ w, c = some w ∧ slot < w
    · obtain ⟨w, rfl, hw⟩ := hcoll
      exact Or.inr ⟨w, hlt1, hw⟩
    · left
      have hgrp : e ∈ ((((a_p2as_to_place_in_log j).take (t + 1)).sum.filter
          (fun x => x.1 = slot)).map Prod.snd) :=
        Multiset.mem_map.mpr ⟨(slot, e), Multiset.mem_filter.mpr ⟨he, rfl⟩, rfl⟩
      obtain ⟨hnone, hsome⟩ := logReduce_spec _ (hE_pairwise hR j (t + 1) slot) none
      cases hv : Multiset.foldlOn LogTie (reduceStep logMerge) none logMerge_comm
          ((((a_p2as_to_place_in_log j).take (t + 1)).sum.filter (fun x => x.1 = slot)).map Prod.snd) with
      | none =>
        exfalso
        rw [(hnone.mp hv).1] at hgrp
        exact Multiset.notMem_zero _ hgrp
      | some v =>
        obtain ⟨-, hdom, -⟩ := hsome v hv
        refine ⟨v, (hmem slot v).mpr ?_, hdom e hgrp⟩
        refine (KeyedMap.get_gcBelow_ofReduce_ofMultiset _ _ _ _ _ _ _).mpr ⟨?_, ?_, hv⟩
        · intro w hw
          subst hw
          exact Nat.le_of_not_lt (fun hlt => hcoll ⟨w, rfl, hlt⟩)
        · rw [Multiset.mem_toFinset]
          exact Multiset.mem_map.mpr ⟨(slot, e), he, rfl⟩
  -- let a_to_proposers_p2b = p_to_acceptors_p2a_batch
  --   .cross_singleton(a_max_ballot)
  --   .map(q!(|(p2a, max_ballot)| (p2a.sender,
  --     ((p2a.slot, p2a.ballot.clone()),
  --      if Some(p2a.ballot) == max_ballot { Ok(()) } else { Err(max_ballot) }))))
  tick (input p2as' := p_to_acceptors_p2a_batch)
      (input mb' := a_max_ballot) :=
    yield (acks := H.bmap (H.bcrossSingleton p2as' mb')
      (fun (p2a, max_ballot) =>
        (p2a.sender.val,
          (⟨p2a.slot, p2a.ballot,
            if some p2a.ballot = max_ballot then .ok () else .error max_ballot⟩
            : P2b (mem prop)))));
  -- an ack at a tick is the tick's verdict on a consumed P2a
  ghost have hack_mem : ∀ (j : Fin (mem acc)) {n : Nat} {e : Multiset (Nat × P2b (mem prop))},
      (acks j)[n]? = some e → ∀ {dx : Nat × P2b (mem prop)}, dx ∈ e →
      ∃ (b_t : Multiset (P2a P (mem prop))) (mb_t : Option (Ballot (mem prop))),
        (batchCuts (p_to_acceptors_p2a j) 0 (dec.p2aBatch j))[n]? = some b_t
        ∧ (a_max_ballot j)[n]? = some mb_t
        ∧ ∃ a ∈ b_t, dx = (a.sender.val,
            (⟨a.slot, a.ballot, if some a.ballot = mb_t then .ok () else .error mb_t⟩
              : P2b (mem prop))) := fun j n e he dx hdx => by
    obtain ⟨b_t, mb_t, hb, hmb, rfl⟩ := (hacks_at j n e).mp he
    refine ⟨b_t, mb_t, hb, hmb, ?_⟩
    have : dx ∈ Multiset.map _ (Multiset.map
        (fun a => (a, show Option (Ballot (mem prop)) from mb_t))
        (show Multiset (P2a P (mem prop)) from b_t)) := hdx
    rw [Multiset.map_map] at this
    obtain ⟨a, ha, rfl⟩ := Multiset.mem_map.mp this
    exact ⟨a, ha, rfl⟩
  -- the demux face: proposer `r`'s acks are the per-acceptor address
  -- filters of the ack streams, summed
  ghost have hdecomp : ∀ (r : Fin (mem prop)),
      ((Values L mem).values ((Values L mem).demux sched.p2bCh
        ((Values L mem).allTicks acks) (fun r => r.val))) r
      = ((List.finRange (mem acc)).map
          (fun j => Multiset.filterMap
            (fun dx : Nat × P2b (mem prop) => if dx.1 = r.val then some dx.2 else none)
            ((acks j).sum))).sum := fun r => rfl
  -- .all_ticks().demux(proposers, …).values()
  (a_log_wc,
    H.values (H.demux sched.p2bCh (H.allTicks acks) (fun r => r.val)))
  prove
    log_len_le_ck := (fun j => by
      show (a_log_wc j).length ≤ _
      rw [ha_log_wc_run j]
      exact le_trans (le_of_eq (scanAcrossTicksTrace_length _ _ _))
        (le_trans (le_of_eq (List.length_zip ..)) (Nat.min_le_right _ _))),
    log_ck := (fun j {t lt} h => by
      obtain ⟨q, c, -, hc, hlt1, -⟩ := hlog_at j h
      rw [hlt1]
      exact hc),
    log_ck_mono := (fun j {t t'} htt {lt lt'} h h' => by
      obtain ⟨q, c, -, hc, hlt1, -⟩ := hlog_at j h
      obtain ⟨q', c', -, hc', hlt1', -⟩ := hlog_at j h'
      obtain ⟨ht, rfl⟩ := List.getElem?_eq_some_iff.mp hc
      obtain ⟨ht', rfl⟩ := List.getElem?_eq_some_iff.mp hc'
      rw [hlt1, hlt1']
      exact (a_checkpoint j).property (dec.ckSnap j) htt ht'),
    log_covers_mono := (fun hR j {t t'} htt {lt lt'} hlt hlt' {slot b} hcov => by
      obtain ⟨e, he, hbe⟩ := hcov
      have hE_t := hentry_src j hlt he
      have hE_t' : (slot, e) ∈ ((a_p2as_to_place_in_log j).take (t' + 1)).sum :=
        Multiset.mem_of_le (List.sum_take_le_sum_take _ (Nat.succ_le_succ htt)) hE_t
      rcases hcover_or_collect hR j hlt' hE_t' with ⟨v, hv, hev⟩ | hcoll
      · exact Or.inl ⟨v, hv, Ballot.ble_trans hbe hev⟩
      · exact Or.inr hcoll),
    log_entry_src := (fun j {t lt} hlt {slot e} hent => by
      obtain ⟨p2a, hp, -, -, -, -, heq⟩ := hE j (t + 1) _ (hentry_src j hlt hent)
      obtain ⟨rfl, rfl⟩ := Prod.mk.inj heq
      exact ⟨p2a, hp, rfl, rfl, rfl⟩),
    acks_by_acceptor := (fun r => by
      refine ⟨fun j => Multiset.filterMap
        (fun dx : Nat × P2b (mem prop) => if dx.1 = r.val then some dx.2 else none)
        ((acks j).sum), hdecomp r, ?_, ?_⟩
      · -- the cap: acks at a key ≤ consumed P2as at the key ≤ pool P2as at the key
        intro j s b
        rw [← Multiset.countP_eq_card_filter]
        refine le_trans (countP_filterMap_le _ _
          (fun dx : Nat × P2b (mem prop) => dx.2.slot = s ∧ dx.2.ballot = b)
          (fun dx m hdx hp => by
            by_cases hr : dx.1 = r.val
            · rw [if_pos hr] at hdx
              injection hdx with h
              rw [h]
              exact hp
            · rw [if_neg hr] at hdx
              cases hdx) _) ?_
        have hacks : acks j = (Trace.zip (batchCuts (p_to_acceptors_p2a j) 0 (dec.p2aBatch j))
            (a_max_ballot j)).map (fun bx => bx.1.map (fun a =>
              (a.sender.val,
                (⟨a.slot, a.ballot, if some a.ballot = bx.2 then .ok () else .error bx.2⟩
                  : P2b (mem prop))))) := by
          rw [hacks_run j]
          refine Eq.trans (scanAcrossTicksTrace_stateless _ _) ?_
          refine List.map_congr_left (fun bx _ => ?_)
          show Multiset.map _ (Multiset.map
              (fun a => (a, show Option (Ballot (mem prop)) from bx.2))
              (show Multiset (P2a P (mem prop)) from bx.1)) = _
          rw [Multiset.map_map]
          rfl
        rw [hacks, countP_list_sum, List.map_map]
        refine le_trans (sum_map_zip_le _
          (fun x : Multiset (P2a P (mem prop)) => x.countP (fun a => a.slot = s ∧ a.ballot = b))
          (fun x => countP_map_le_countP _ _ _ (fun a hp => hp) x.1) _ _) ?_
        rw [← countP_list_sum]
        refine Multiset.countP_le_of_le _ ?_
        have := batchCuts_sum_le (pool := p_to_acceptors_p2a j)
          (d := dec.p2aBatch j) (consumed := 0) (Multiset.zero_le _)
        rwa [Multiset.zero_add] at this
      · -- each ack opens to its P2a, with write-before-ack coverage (or
        -- collection) under the clause
        intro j m hm
        obtain ⟨dx, hdx, haddr⟩ := (Multiset.mem_filterMap _ _).mp hm
        by_cases hr : dx.1 = r.val
        · rw [if_pos hr] at haddr
          injection haddr with haddr'
          obtain ⟨e, he, hdxe⟩ := mem_list_sum.mp hdx
          obtain ⟨t, he'⟩ := List.mem_iff_getElem?.mp he
          obtain ⟨b_t, mb_t, hb, hmb, a, ha, rfl⟩ := hack_mem j he' hdxe
          refine ⟨a, mem_pool_of_mem_batch hb ha, by rw [← hr], by rw [← haddr'],
            by rw [← haddr'], ?_⟩
          intro hok
          rw [← haddr'] at hok
          by_cases hcond : some a.ballot = mb_t
          · refine ⟨t, by rw [hmb, ← haddr', ← hcond], ?_⟩
            intro hR hck
            obtain ⟨ck_t, hckt⟩ : ∃ ck_t, (a_checkpoint_tick j)[t]? = some ck_t :=
              ⟨_, List.getElem?_eq_getElem hck⟩
            -- the consumed P2a qualifies at its own max, so its entry is
            -- in the tick's qualified batch, hence among the consumed
            -- entries through `t`
            have hqual : p2aQualifies mb_t a.ballot = true := by
              rw [← hcond]
              show a.ballot.ble a.ballot = true
              exact Ballot.ble_iff_key.mpr (le_refl _)
            obtain ⟨q, hq, hent⟩ := hqual_of j hb hmb a ha hqual
            obtain ⟨lt, hlt⟩ := hlog_exists j hq hckt
            refine ⟨lt, hlt, ?_⟩
            rw [← haddr']
            refine hcover_or_collect hR j hlt (slot := a.slot) (e := ⟨a.ballot, a.value⟩) ?_
            refine mem_list_sum.mpr ⟨q, ?_, hent⟩
            exact List.mem_iff_getElem?.mpr ⟨t, by
              rw [List.getElem?_take, if_pos (Nat.lt_succ_self t)]; exact hq⟩
          · exfalso
            simp only [if_neg hcond] at hok
            cases hok
        · rw [if_neg hr] at haddr
          cases haddr),
    ack_src := (fun r m hm => by
      have hm : m ∈ ((Values L mem).values ((Values L mem).demux sched.p2bCh
          ((Values L mem).allTicks acks) (fun r => r.val))) r := hm
      rw [hdecomp r] at hm
      obtain ⟨mm, hmm, hm1⟩ := mem_list_sum.mp hm
      obtain ⟨j, -, rfl⟩ := List.mem_map.mp hmm
      obtain ⟨dx, hdx, haddr⟩ := (Multiset.mem_filterMap _ _).mp hm1
      by_cases hr : dx.1 = r.val
      · rw [if_pos hr] at haddr
        injection haddr with haddr'
        obtain ⟨e, he, hdxe⟩ := mem_list_sum.mp hdx
        obtain ⟨t, he'⟩ := List.mem_iff_getElem?.mp he
        obtain ⟨b_t, mb_t, hb, hmb, a, ha, rfl⟩ := hack_mem j he' hdxe
        exact ⟨j, t, mb_t, hmb, a, mem_pool_of_mem_batch hb ha, by rw [← hr],
          by rw [← haddr'], by rw [← haddr'], by rw [← haddr']⟩
      · rw [if_neg hr] at haddr
        cases haddr)

/-- **`Ok` acks pin the max** (corollary): an `Ok` P2b's ballot *is*
the `a_max_ballot` wire value at its ack tick (paxos.rs:881) — with
`acceptor_p1`'s ascent, late low-ballot votes are impossible. -/
theorem acceptor_p2_ok_max (acc prop : L)
    {ckα : Type} [DecidableEq ckα] {ckord : StrOrd} {ckret : Retries}
    (mx : TickV (mem acc) (Option (Ballot (mem prop))))
    (pool : MemberMap (mem acc) (Multiset (P2a P (mem prop))))
    (ck : (Values L mem).Singleton acc ckα (Option Nat) ckord ckret
      (.monotonic ValueOrder.optNat))
    (dec : AP2Dec (Values L mem) (mem prop) (mem acc) P ckα ckord)
    (ch : AP2Sched (Values L mem) (mem prop) (mem acc))
    (r : Fin (mem prop)) (m : P2b (mem prop))
    (hm : m ∈ (acceptor_p2 (Values L mem) acc prop mx pool ck dec
      ch).2 r)
    (hok : m.res = .ok ()) :
    ∃ (j : Fin (mem acc)) (t : Nat), (mx j)[t]? = some (some m.ballot) := by
  obtain ⟨j, t, mb, hmb, p2a, -, -, -, hballot, hres⟩ :=
    (acceptor_p2.ensures acc prop mx pool ck dec ch).ack_src r m hm
  refine ⟨j, t, ?_⟩
  by_cases hc : some p2a.ballot = mb
  · rw [hmb, ← hc, hballot]
  · rw [if_neg hc] at hres
    rw [hres] at hok
    cases hok


/-! ## Executable smoke tests -/

/-- A checkpoint that never advances (the scenarios' `Monotonic`
singleton: every snapshot reads `none`). -/
def ckNone {ckα : Type} [DecidableEq ckα] {ckord : StrOrd} {ckret : Retries} (acc : L) :
    (Values L mem).Singleton acc ckα (Option Nat) ckord ckret (.monotonic ValueOrder.optNat) :=
  .mk' fun _ => SingletonV.noneCell ckα ckord

/-- A checkpoint that advances once: `none` at the first tick, `some c`
from the second on (the GC scenarios). -/
def ckStep {ckα : Type} [DecidableEq ckα] {ckord : StrOrd} {ckret : Retries} (acc : L) (c : Nat) :
    (Values L mem).Singleton acc ckα (Option Nat) ckord ckret (.monotonic ValueOrder.optNat) :=
  .mk' fun _ => SingletonV.stepCell ckα ckord c

-- One acceptor, one proposer: the qualified P2a lands in the log and is
-- acked `Ok`; a stale-ballot P2a in the same batch is rejected and kept
-- out of the log.
#guard_eval (acceptor_p2 (Values PaxLoc (paxMem 1 1)) .acc .prop
    (ckα := Nat) (ckord := .totalOrder) (ckret := .exactlyOnce)
    (.mk' fun _ => [some (Ballot.mk 1 0)])
    (.mk' fun _ => {⟨0, Ballot.mk 1 0, 0, some 42⟩, ⟨0, Ballot.mk 0 0, 0, some 7⟩})
    (ckNone (mem := paxMem 1 1) .acc)
    ⟨fun _ => [{⟨0, Ballot.mk 1 0, 0, some 42⟩, ⟨0, Ballot.mk 0 0, 0, some 7⟩}],
     fun _ => [0]⟩
    .triv).1 0
  = [((none : Option Nat),
      HMap.ofList [(0, (⟨Ballot.mk 1 0, some 42⟩ : LogValue Nat 1))])]

#guard_eval (acceptor_p2 (Values PaxLoc (paxMem 1 1)) .acc .prop
    (ckα := Nat) (ckord := .totalOrder) (ckret := .exactlyOnce)
    (.mk' fun _ => [some (Ballot.mk 1 0)])
    (.mk' fun _ => ({⟨0, Ballot.mk 1 0, 0, some 42⟩, ⟨0, Ballot.mk 0 0, 0, some 7⟩}
      : Multiset (P2a Nat 1)))
    (ckNone (mem := paxMem 1 1) .acc)
    ⟨fun _ => [{⟨0, Ballot.mk 1 0, 0, some 42⟩, ⟨0, Ballot.mk 0 0, 0, some 7⟩}],
     fun _ => [0]⟩
    .triv).2 0
  = {⟨0, Ballot.mk 1 0, .ok ()⟩, ⟨0, Ballot.mk 0 0, .error (some (Ballot.mk 1 0))⟩}

-- Garbage collection: two ticks, slots 0 and 1 accepted at the first,
-- the checkpoint advancing to 1 at the second — slot 0 is collected
-- (the published checkpoint names the deletion).
#guard_eval (acceptor_p2 (Values PaxLoc (paxMem 1 1)) .acc .prop
    (ckα := Nat) (ckord := .totalOrder) (ckret := .exactlyOnce)
    (.mk' fun _ => [some (Ballot.mk 1 0), some (Ballot.mk 1 0)])
    (.mk' fun _ => {⟨0, Ballot.mk 1 0, 0, some 42⟩, ⟨0, Ballot.mk 1 0, 1, some 7⟩})
    (ckStep (mem := paxMem 1 1) .acc 1)
    ⟨fun _ => [{⟨0, Ballot.mk 1 0, 0, some 42⟩, ⟨0, Ballot.mk 1 0, 1, some 7⟩}, {}],
     fun _ => [0, 0]⟩
    .triv).1 0
  = [((none : Option Nat),
      HMap.ofList [(0, (⟨Ballot.mk 1 0, some 42⟩ : LogValue Nat 1)), (1, ⟨Ballot.mk 1 0, some 7⟩)]),
     (some 1, HMap.ofList [(1, ⟨Ballot.mk 1 0, some 7⟩)])]

-- The refused merge (the `assume`, visible): two same-slot same-ballot
-- entries with different values — the pool violates the `requires`, the
-- slot's group is refused and the slot vanishes from the log. Unreachable
-- from a compiled `hydro def` (the clause is mandatory at the site).
#guard_eval (acceptor_p2 (Values PaxLoc (paxMem 1 1)) .acc .prop
    (ckα := Nat) (ckord := .totalOrder) (ckret := .exactlyOnce)
    (.mk' fun _ => [some (Ballot.mk 1 0)])
    (.mk' fun _ => {⟨0, Ballot.mk 1 0, 0, some 42⟩, ⟨0, Ballot.mk 1 0, 0, some 7⟩})
    (ckNone (mem := paxMem 1 1) .acc)
    ⟨fun _ => [{⟨0, Ballot.mk 1 0, 0, some 42⟩, ⟨0, Ballot.mk 1 0, 0, some 7⟩}],
     fun _ => [0]⟩
    .triv).1 0
  = [((none : Option Nat), HMap.ofList [])]

#nondet_census acceptor_p2 (nondets := 2) (scheds := 1) (fuels := 0)

end Hydro
