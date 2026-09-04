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
`a_max_ballot`
(`Some(&p2a.ballot) >= max_ballot`), accumulate the qualified entries
across ticks, and ack each P2a to its sender with `Ok(())` iff its
ballot *is* the current max.

**The `manual_proof!` hole, made honest**: Rust's per-slot
`reduce_watermark` merge is not commutative when one slot sees two
entries with equal ballots and different values (paxos.rs:862's
`TODO: need assume`). The model accumulates the raw entry
*multiset* — trivially commutative and inflationary, so the `NoOrder`
fold obligation is paid unconditionally — and computes the log as the
pure canonical view `logView` (per-slot max ballot; equal-ballot value
conflicts degrade to `none` via the commutative `ValWitness` consensus
fold). Under the slot-functional input contract the conflict branch is
unreachable, which is exactly the Rust `assume`, now checkable.

**Watermark GC (`reduce_watermark`'s deletion), and why not modeling
it is safe**: Rust deletes keys below the `a_checkpoint` watermark;
the model keeps every entry and carries the checkpoint alongside
(`a_log = (ckpt, logView pool)`). This is safety-neutral *within
`paxos_core`* — provably, since the headline takes the checkpoint as
an arbitrary input trace with NO premise: the proposer side mirrors
the watermark discipline (`recommitList` skips slots ≤ the view
checkpoint and starts holes at `ckpt + 1`, Recommit.lean =
paxos.rs:606–670), so wherever an acceptor *would* have GC'd slot `s`
its checkpoint in the same view already exceeds `s` and no new
emission at `s` can occur; retained sub-watermark entries are never
re-proposed. What early GC (a checkpoint outrunning replica
execution) actually threatens is **liveness at whole-system scope** —
permanently skipped slots that block replicas — and both replicas and
the checkpoint feedback invariant live OUTSIDE `paxos_core`'s
boundary (paxos.rs's `replica.rs` loop); that premise belongs to a
future replica/commit-liveness rung. `reduce_watermark`'s
library-internal `assume_ordering`/`assume_retries` `nondet!`s are
discharged by the commutativity obligation here (paid as a proof, not
a census decision) — the census counts protocol-level `nondet!`s only.
-/

namespace Hydro

variable {L : Type} {mem : L → Nat} {P : Type} [DecidableEq P]

/-- **What `acceptor_p2` ensures**, over the `Values` denotation — stated
on its OUTPUT WIRES in the consumer's vocabulary (per-tick reads by
`[t]?`, membership in the input pool), never in this module's own fold
vocabulary: a consumer reads these faces and sees `a_log`, `p2as`,
`a_max_ballot`, `a_checkpoint`, nothing unfolded (FINDINGS D63). -/
structure AP2Ensures (acc prop : L)
    (mx : TickV (mem acc) (Option (Ballot (mem prop))))
    (pool : Fin (mem acc) → Multiset (P2a P (mem prop)))
    (ck : TickV (mem acc) (Option Nat))
    (out : TickV (mem acc) (ALog P (mem prop))
      × (Fin (mem prop) → Multiset (P2b (mem prop)))) : Prop where
  /-- **The published clock**: the log wire ticks at most as often as the
  checkpoint input (its tick zip). -/
  log_len_le_ck : ∀ j, (out.1 j).length ≤ (ck j).length
  /-- **Coverage ascent**: a slot covered at a tick stays covered at every
  later tick (the accumulated entry pool only grows). -/
  log_covers_mono : ∀ (j : Fin (mem acc)) {t t' : Nat} (_ : t ≤ t')
    {lt lt' : ALog P (mem prop)},
    (out.1 j)[t]? = some lt → (out.1 j)[t']? = some lt' →
    ∀ {slot : Nat} {b : Ballot (mem prop)},
      LogCovers lt.2 slot b → LogCovers lt'.2 slot b
  /-- **Entry provenance**: a published entry at `(slot, b)` quotes a
  consumed P2a at that key, and carries the common value of the pool's
  P2as at that key when they agree. -/
  log_entry_src : ∀ (j : Fin (mem acc)) {t : Nat} {lt : ALog P (mem prop)},
    (out.1 j)[t]? = some lt → ∀ {slot : Nat} {e : LogValue P (mem prop)},
    (slot, e) ∈ lt.2 →
    (∃ p2a ∈ pool j, p2a.slot = slot ∧ p2a.ballot = e.ballot)
    ∧ ∀ V : Option P,
      (∀ p2a ∈ pool j, p2a.slot = slot → p2a.ballot = e.ballot → p2a.value = V) →
      e.value = V
  /-- **The acks, by acceptor**: proposer `r`'s ack pool is a sum of
  per-acceptor pools; acceptor `j` acks a key at most as often as its
  pool holds P2as at that key; every ack opens to a consumed P2a of the
  sender, and an `Ok` ack pins the tick's max at its ballot and the
  published log's coverage of its key at that very tick
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
                  ∧ (t < (ck j).length →
                      ∃ lt, (out.1 j)[t]? = some lt ∧ LogCovers lt.2 m.slot m.ballot))
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
      H.Ticked acc (Option (Ballot (mem prop))))
    (p_to_acceptors_p2a :
      H.Stream acc (P2a P (mem prop)) .noOrder .exactlyOnce)
    (a_checkpoint : H.Singleton acc ckα (Option Nat) ckord ckret
      .unbounded)
    (dec : AP2Dec H (mem prop) (mem acc) P ckα ckord)
    (sched : AP2Sched H (mem prop) (mem acc)) :
    (H.Ticked acc (ALog P (mem prop))
      × H.Stream prop (P2b (mem prop)) .noOrder .exactlyOnce)
  ensures out => AP2Ensures acc prop a_max_ballot p_to_acceptors_p2a
    (fun i => a_checkpoint i (dec.ckSnap i)) out :=
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
  --     q!(max-by-ballot, commutative = manual_proof!(/** … TODO: not if
  --        two entries with same ballot, need assume */)))});
  -- let a_log_snapshot = a_log.entries().fold(q!(HashMap::new),
  --   q!(insert, commutative = manual_proof!(/** no overlapping keys */)));
  -- (the per-slot keyed reduce + the insert fold + the checkpoint zip,
  -- collapsed onto the canonical view `logView` of the accumulated pool
  -- — GC unmodeled, the tie-break honest; see the header)
  tick (state log_entries : H.BoundedStream (Nat × LogValue P (mem prop))
        .noOrder .exactlyOnce)
      (input ck := a_checkpoint_tick)
      (input quals := a_p2as_to_place_in_log) :=
    -- across_ticks: the accumulated entry pool — the commutativity
    -- obligation (Rust's `commutative = manual_proof!`) is paid
    -- unconditionally on the accumulation
    let pool := H.bchain log_entries quals
    let a_log_snapshot := H.bfold (fun s e => s + {e}) 0
      (fun s x y => add_singleton_comm s x y) pool
    rebind (log_entries := pool)
    -- a_checkpoint.into_singleton().zip(a_log_snapshot)
    emit (a_log := H.bsMap (H.bsZip ck a_log_snapshot)
      (fun ce => ((ce.1, logView ce.2) : ALog P (mem prop))));
  -- one tick of the log block: the pool grows by the tick's qualified
  -- entries; the published pair is the checkpoint and the pool's view
  ghost have hlog_step : ∀ (j : Fin (mem acc)) st x,
      a_log_step j st x = (@id (Multiset (Nat × LogValue P (mem prop))) st
          + @id (Multiset (Nat × LogValue P (mem prop))) x.2,
        (x.1, logView (@id (Multiset (Nat × LogValue P (mem prop))) st
          + @id (Multiset (Nat × LogValue P (mem prop))) x.2))) := fun j st x => by
    show ((@id (Multiset (Nat × LogValue P (mem prop))) st
        + @id (Multiset (Nat × LogValue P (mem prop))) x.2),
      (x.1, logView (@Multiset.foldl _ _ (fun s e => s + {e})
        ⟨fun s x y => add_singleton_comm s x y⟩ 0
        (@id (Multiset (Nat × LogValue P (mem prop))) st
          + @id (Multiset (Nat × LogValue P (mem prop))) x.2)))) = _
    rw [foldl_add_singleton, Multiset.zero_add]
  -- **the accumulated pool**: the published log at every tick is the
  -- checkpoint paired with the canonical view of a pool that only grows
  -- along ticks and whose entries all quote consumed, qualifying P2as —
  -- the one place the block's register is read (everything after cites
  -- `pool`, never the fold)
  ghost have hlog_pool : ∀ (j : Fin (mem acc)),
      ∃ pool : Nat → Multiset (Nat × LogValue P (mem prop)),
        (∀ {t : Nat} {lt : ALog P (mem prop)}, (a_log j)[t]? = some lt →
          ∃ ck_t, (a_checkpoint j (dec.ckSnap j))[t]? = some ck_t ∧ lt = (ck_t, logView (pool t)))
        ∧ (∀ {t t' : Nat}, t ≤ t' → pool t ≤ pool t')
        ∧ (∀ (t : Nat), ∀ en ∈ pool t,
            ∃ p2a ∈ p_to_acceptors_p2a j, ∃ (t₀ : Nat) (mb_t : Option (Ballot (mem prop))),
              (a_max_ballot j)[t₀]? = some mb_t ∧ p2aQualifies mb_t p2a.ballot = true
              ∧ en = (p2a.slot, ⟨p2a.ballot, p2a.value⟩))
        ∧ (∀ {t : Nat} {ck_t : Option Nat} {q_t : Multiset (Nat × LogValue P (mem prop))},
            (a_checkpoint j (dec.ckSnap j))[t]? = some ck_t →
            (a_p2as_to_place_in_log j)[t]? = some q_t →
            (a_log j)[t]? = some (ck_t, logView (pool t)) ∧ q_t ≤ pool t) := fun j => by
    -- the register before tick `n`
    let R : Nat → Multiset (Nat × LogValue P (mem prop)) := fun n =>
      scanAcrossTicksState (a_log_step j)
        (ValuesTick.seed (TickShape.stream (Nat × LogValue P (mem prop)) inferInstance
          .noOrder .exactlyOnce) ())
        ((Trace.zip (a_checkpoint j (dec.ckSnap j)) (a_p2as_to_place_in_log j)).take n)
    have hR_succ : ∀ {t : Nat} {ck_t : Option Nat} {q_t : Multiset (Nat × LogValue P (mem prop))},
        (a_checkpoint j (dec.ckSnap j))[t]? = some ck_t →
        (a_p2as_to_place_in_log j)[t]? = some q_t → R (t + 1) = R t + q_t := by
      intro t ck_t q_t hck hq
      -- (`exact`, not `rw`: the fold's input type is the construct's shaped
      -- tuple, the zip read's is the plain pair — equal only by unfolding)
      exact (scanAcrossTicksState_take_succ? (a_log_step j) _
        (Trace.zip (a_checkpoint j (dec.ckSnap j)) (a_p2as_to_place_in_log j))
        (Trace.getElem?_zip_eq_some.mpr ⟨hck, hq⟩)).trans
        (congrArg Prod.fst (hlog_step j _ _))
    have hR_mono : ∀ (u k : Nat), R u ≤ R (u + k) := fun u k =>
      scanAcrossTicksState_chain (a_log_step j) _ _
        (fun a b => (@id (Multiset (Nat × LogValue P (mem prop))) a) ≤ b)
        (fun _ => le_refl _) (fun _ _ _ => le_trans) (fun _ => True)
        (fun st x _ => by rw [hlog_step j _ _]; exact Multiset.le_add_right _ _) u k
        (fun _ _ _ _ _ => trivial)
    have hR_src : ∀ (n : Nat), ∀ en ∈ R n,
        ∃ p2a ∈ p_to_acceptors_p2a j, ∃ (t₀ : Nat) (mb_t : Option (Ballot (mem prop))),
          (a_max_ballot j)[t₀]? = some mb_t ∧ p2aQualifies mb_t p2a.ballot = true
          ∧ en = (p2a.slot, ⟨p2a.ballot, p2a.value⟩) := fun n => by
      refine scanAcrossTicksState_induct (a_log_step j) _
        (Trace.zip (a_checkpoint j (dec.ckSnap j)) (a_p2as_to_place_in_log j))
        (fun st => ∀ en : Nat × LogValue P (mem prop),
          en ∈ (@id (Multiset (Nat × LogValue P (mem prop))) st) →
          ∃ p2a ∈ p_to_acceptors_p2a j, ∃ (t₀ : Nat) (mb_t : Option (Ballot (mem prop))),
            (a_max_ballot j)[t₀]? = some mb_t ∧ p2aQualifies mb_t p2a.ballot = true
            ∧ en = (p2a.slot, ⟨p2a.ballot, p2a.value⟩))
        (fun en hen => absurd hen (Multiset.notMem_zero _)) ?_ n
      intro st x hx hst en hen
      rw [hlog_step j st x] at hen
      rcases Multiset.mem_add.mp hen with h | h
      · exact hst en h
      · obtain ⟨t, hx1, hx2⟩ := Trace.mem_hist_iff.mp hx
        obtain ⟨p2a, hp, mb_t, hmb, hq, rfl⟩ := hqual_mem j hx2 h
        exact ⟨p2a, hp, t, mb_t, hmb, hq, rfl⟩
    refine ⟨fun t => R (t + 1), ?_, ?_, fun t => hR_src (t + 1), ?_⟩
    · intro t lt hlt
      obtain ⟨ck_t, q_t, hck, hq, rfl⟩ := (ha_log_at j t lt).mp hlt
      refine ⟨ck_t, hck, ?_⟩
      rw [hlog_step j _ _]
      show (ck_t, logView (R t + q_t)) = (ck_t, logView (R (t + 1)))
      rw [hR_succ hck hq]
    · intro t t' htt
      obtain ⟨k, rfl⟩ := Nat.exists_eq_add_of_le htt
      have := hR_mono (t + 1) k
      rwa [show t + 1 + k = t + k + 1 by omega] at this
    · intro t ck_t q_t hck hq
      refine ⟨(ha_log_at j t _).mpr ⟨ck_t, q_t, hck, hq, ?_⟩, ?_⟩
      · rw [hlog_step j _ _]
        show (ck_t, logView (R (t + 1))) = (ck_t, logView (R t + q_t))
        rw [hR_succ hck hq]
      · show q_t ≤ R (t + 1)
        rw [hR_succ hck hq]
        exact Multiset.le_add_left _ _
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
  (a_log,
    H.values (H.demux sched.p2bCh (H.allTicks acks) (fun r => r.val)))
  prove
    log_len_le_ck := (fun j => by
      show (a_log j).length ≤ _
      rw [ha_log_run j]
      exact le_trans (le_of_eq (scanAcrossTicksTrace_length _ _ _))
        (le_trans (le_of_eq (List.length_zip ..)) (Nat.min_le_left _ _))),
    log_covers_mono := (fun j {t t'} htt {lt lt'} hlt hlt' {slot b} hcov => by
      obtain ⟨pool, hread, hmono, -, -⟩ := hlog_pool j
      obtain ⟨ck_t, -, hlt_eq⟩ := hread hlt
      obtain ⟨ck_t', -, hlt_eq'⟩ := hread hlt'
      rw [hlt_eq] at hcov
      rw [hlt_eq']
      exact logView_covers_mono (hmono htt) hcov),
    log_entry_src := (fun j {t lt} hlt {slot e} hent => by
      obtain ⟨pool, hread, -, hsrc, -⟩ := hlog_pool j
      obtain ⟨ck_t, -, hlt_eq⟩ := hread hlt
      rw [hlt_eq] at hent
      obtain ⟨⟨lv, hlv, hlvb⟩, hval⟩ := logView_entry_value _ hent
      -- an accumulated entry at the key is a pool P2a at the key
      have hof : ∀ lv' : LogValue P (mem prop), (slot, lv') ∈ pool t →
          ∃ p2a ∈ p_to_acceptors_p2a j, p2a.slot = slot ∧ p2a.ballot = lv'.ballot
            ∧ p2a.value = lv'.value := by
        intro lv' h
        obtain ⟨p2a, hp, -, -, -, -, heq⟩ := hsrc t _ h
        injection heq with h1 h2
        subst h1 h2
        exact ⟨p2a, hp, rfl, rfl, rfl⟩
      obtain ⟨p2a, hp, hs, hb, -⟩ := hof lv hlv
      refine ⟨⟨p2a, hp, hs, hb.trans hlvb⟩, fun V hV => hval V ?_⟩
      intro lv' hlv' hlvb'
      obtain ⟨q, hq', hs', hb', hv'⟩ := hof lv' hlv'
      rw [← hv']
      exact hV q hq' hs' (hb'.trans hlvb')),
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
      · -- each ack opens to its P2a, with write-before-ack coverage
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
            intro hck
            obtain ⟨ck_t, hckt⟩ : ∃ ck_t, (a_checkpoint j (dec.ckSnap j))[t]? = some ck_t :=
              ⟨_, List.getElem?_eq_getElem hck⟩
            -- the consumed P2a qualifies at its own max, so its entry is
            -- in the tick's qualified batch, hence in the published view
            have hqual : p2aQualifies mb_t a.ballot = true := by
              rw [← hcond]
              show a.ballot.ble a.ballot = true
              exact Ballot.ble_iff_key.mpr (le_refl _)
            obtain ⟨q, hq, hent⟩ := hqual_of j hb hmb a ha hqual
            obtain ⟨pool, -, -, -, hpub⟩ := hlog_pool j
            obtain ⟨hlog, hle⟩ := hpub hckt hq
            refine ⟨_, hlog, ?_⟩
            show LogCovers (logView (pool t)) m.slot m.ballot
            rw [← haddr']
            exact logView_covers _ (Multiset.mem_of_le hle hent)
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
    (pool : Fin (mem acc) → Multiset (P2a P (mem prop)))
    (ck : (Values L mem).Singleton acc ckα (Option Nat) ckord ckret
      .unbounded)
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

-- One acceptor, one proposer: the qualified P2a lands in the log view
-- and is acked `Ok`; a stale-ballot P2a in the same batch is rejected
-- and kept out of the log.
#guard (acceptor_p2 (Values PaxLoc (paxMem 1 1)) .acc .prop
    (ckα := Nat) (ckord := .totalOrder) (ckret := .exactlyOnce)
    (fun _ => [some (Ballot.mk 1 0)])
    (fun _ => {⟨0, Ballot.mk 1 0, 0, some 42⟩, ⟨0, Ballot.mk 0 0, 0, some 7⟩})
    (fun _ d => d.map (fun _ => none))
    ⟨fun _ => [{⟨0, Ballot.mk 1 0, 0, some 42⟩, ⟨0, Ballot.mk 0 0, 0, some 7⟩}],
     fun _ => [0]⟩
    .triv).1 0
  = [((none : Option Nat),
      [(0, (⟨Ballot.mk 1 0, some 42⟩ : LogValue Nat 1))])]

#guard (acceptor_p2 (Values PaxLoc (paxMem 1 1)) .acc .prop
    (ckα := Nat) (ckord := .totalOrder) (ckret := .exactlyOnce)
    (fun _ => [some (Ballot.mk 1 0)])
    (fun _ => ({⟨0, Ballot.mk 1 0, 0, some 42⟩, ⟨0, Ballot.mk 0 0, 0, some 7⟩}
      : Multiset (P2a Nat 1)))
    (fun _ d => d.map (fun _ => none))
    ⟨fun _ => [{⟨0, Ballot.mk 1 0, 0, some 42⟩, ⟨0, Ballot.mk 0 0, 0, some 7⟩}],
     fun _ => [0]⟩
    .triv).2 0
  = {⟨0, Ballot.mk 1 0, .ok ()⟩, ⟨0, Ballot.mk 0 0, .error (some (Ballot.mk 1 0))⟩}

-- The conflict tie-break degrades to `none` (the `assume`, visible):
-- two same-slot same-ballot entries with different values.
#guard logView ({(0, ⟨Ballot.mk 1 0, some 42⟩), (0, ⟨Ballot.mk 1 0, some 7⟩)}
    : Multiset (Nat × LogValue Nat 1))
  = [(0, ⟨Ballot.mk 1 0, none⟩)]

#nondet_census acceptor_p2 (nondets := 2) (scheds := 1) (fuels := 0)

end Hydro
