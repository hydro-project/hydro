import Hydro.MonoRel
import Hydro.Paxos.Types
import Mathlib.Data.Multiset.Filter
import Hydro.HydroTick

/-!
# `acceptor_p1` (paxos.rs:484–524)

Acceptor phase-1 logic: maintain `a_max_ballot` — `across_ticks(|s|
s.max())` over the **unordered** P1a tick batches, safe because max is
commutative (the `NoOrder` fold obligation, consumed by the batch
quotient) and `Monotonic` by the inflation obligation — and reply to
each P1a with `Ok(log)` iff its ballot *is* the current max, else
`Err(max_ballot)`, routed to `ballot.proposer_id` by `demux`.

The `a_log` singleton is the same-tick wire from `acceptor_p2` (the
`a_log` `forward_ref` knot, closed in `paxos_core`): a tick's replies
exist only once its log value is realized — the *write-before-ack*
staging of Rust's `snapshot_atomic`.
-/

namespace Hydro

variable {L : Type} {mem : L → Nat} {P : Type} [DecidableEq P]

/-! ## The contract: vocabulary and guarantees (proofs below) -/

/-- The `a_max_ballot` batch step (`s.max()` absorbed batchwise — the
element closure's lift to the batch multiset). -/
def aMaxStep {nP : Nat} (s : Option (Ballot nP))
    (b : Multiset (Ballot nP)) : Option (Ballot nP) :=
  @Multiset.foldl _ _ Ballot.maxFold
    ⟨fun s x y => Ballot.maxFold_comm s x y⟩ s b


/-- The canonical reply multiset acceptor `j` addresses to proposer `r`
(contract vocabulary: the per-sender summand of the merged reply pool —
sender identity is channel structure, not message content). -/
def ap1From {nA nP : Nat} {P : Type} [DecidableEq P]
    (bs : Fin nA → Trace (Multiset (Ballot nP)))
    (alog : TickV nA (ALog P nP)) (j : Fin nA) (r : Fin nP) :
    Multiset (P1b P nP) :=
  Multiset.filterMap
    (fun dx : Nat × P1b P nP => if dx.1 = r.val then some dx.2 else none)
    ((Trace.zip (bs j)
        (Trace.zip (foldAcrossTicksTrace aMaxStep none (bs j)) (alog j))).map
      (fun bx => bx.1.map (fun a =>
        (a.proposerId.val,
          (⟨a, if some a = bx.2.1 then .ok bx.2.2
            else .error bx.2.1⟩ : P1b P nP))))).sum


/-- What `acceptor_p1` **ensures**, over the `Values` denotation. -/
structure AP1Ensures (acc prop : L)
    (bs : Fin (mem acc) → Trace (Multiset (Ballot (mem prop))))
    (alog : TickV (mem acc) (ALog P (mem prop)))
    (out : TickV (mem acc) (Option (Ballot (mem prop)))
      × (Fin (mem prop) → Multiset (P1b P (mem prop)))) : Prop where
  /-- The max face: per tick, the fold of everything consumed so far
  (including this tick's batch) — the two faces cannot drift. -/
  max_face : ∀ i, out.1 i = foldAcrossTicksTrace aMaxStep none (bs i)
  /-- **Ascent**: the promised max only grows along an acceptor's ticks
  (`reduce` by max is inflationary) — a contract fact, since the Rust
  wire is a plain `Bounded` tick singleton. -/
  max_mono : ∀ i, Ascending Ballot.obtVO (out.1 i)
  /-- **The pool decomposes by sender**: proposer `r`'s replies are the
  sum of the per-acceptor summands (`values ∘ demux` opened once). -/
  reply_decomp : ∀ (r : Fin (mem prop)),
    out.2 r = ((List.finRange (mem acc)).map
      (fun j => ap1From bs alog j r)).sum
  /-- **Per-sender reply cap**: acceptor `j` replies at ballot `b` at
  most once per consumed copy of `b` (replies are an elementwise image
  of the consumed batches). -/
  from_ballot_cap : ∀ (j : Fin (mem acc)) (r : Fin (mem prop))
    (b : Ballot (mem prop)),
    ((ap1From bs alog j r).filter (fun m => m.ballot = b)).card
      ≤ ((bs j).sum).count b
  /-- **Per-sender reply characterization** (echo + routing + promise
  shape at the SENDING acceptor): every reply in `j`'s summand quotes a
  P1a consumed at some realized tick `t` of `j`, is routed to the
  ballot's owner, and its verdict compares against tick `t`'s max and
  quotes tick `t`'s `a_log` wire value (write-before-ack). -/
  from_src : ∀ (j : Fin (mem acc)) (r : Fin (mem prop)),
    ∀ m ∈ ap1From bs alog j r,
    ∃ (t : Nat) (hb : t < (bs j).length)
      (hm : t < (out.1 j).length) (hl : t < (alog j).length),
      m.ballot ∈ (bs j)[t]'hb
      ∧ m.ballot.proposerId.val = r.val
      ∧ m.res = (if some m.ballot = (out.1 j)[t]'hm
          then Except.ok ((alog j)[t]'hl)
          else Except.error ((out.1 j)[t]'hm))
  /-- **Reply characterization** (merged-pool corollary shape): every
  reply in a proposer's pool quotes a P1a consumed at some realized
  acceptor tick `t`, is routed to the ballot's owner, and its verdict
  compares against tick `t`'s max and quotes tick `t`'s `a_log` wire
  value (write-before-ack). -/
  reply_src : ∀ (r : Fin (mem prop)), ∀ m ∈ out.2 r,
    ∃ (j : Fin (mem acc)) (t : Nat) (hb : t < (bs j).length)
      (hm : t < (out.1 j).length) (hl : t < (alog j).length),
      m.ballot ∈ (bs j)[t]'hb
      ∧ m.ballot.proposerId.val = r.val
      ∧ m.res = (if some m.ballot = (out.1 j)[t]'hm
          then Except.ok ((alog j)[t]'hl)
          else Except.error ((out.1 j)[t]'hm))


/-- **`acceptor_p1`'s adversary-side (sched-det) bundle** (`Unit` at
`Values`; see `Sem.lean`'s classification table). -/
structure AP1Sched (H : HydroSem L mem) (nP nA : Nat) where
  /-- `a_to_proposers_p1b.demux(&proposers, TCP.fail_stop()…)`: P1b
  delivery cursors (Rust's `demux` carries no `nondet!` — delivery is
  unmarked machine freedom). -/
  p1bCh : H.TransportDec nP nA

/-- The trivial bundle at the denotation. -/
def AP1Sched.triv {nP nA : Nat} : AP1Sched (Values L mem) nP nA := ⟨()⟩

/-- **paxos.rs:484–524 `acceptor_p1`** over the acceptor cluster `acc`,
replying into the proposer cluster `prop`. Returns (`a_max_ballot`,
`a_to_proposers_p1b`).

Rust `nondet!` tally: 1 (the P1b demux transport; the P1a batch is the
caller's — `leader_election` consumes it before the call). -/
hydro def acceptor_p1 (H : HydroSem L mem) (acc prop : L)
    (p_to_acceptors_p1a :
      H.TickStream acc (Ballot (mem prop)) .noOrder .exactlyOnce)
    (a_log : H.Ticked acc (ALog P (mem prop)))
    (sched : AP1Sched H (mem prop) (mem acc)) :
    (H.Ticked acc (Option (Ballot (mem prop)))
      × H.Stream prop (P1b P (mem prop)) .noOrder .exactlyOnce)
  ensures out => AP1Ensures acc prop p_to_acceptors_p1a a_log out :=
  -- let a_max_ballot = p_to_acceptors_p1a.clone()
  --   .across_ticks(|s| s.max())
  --   .into_singleton();
  tick (state persisted : H.BoundedStream (Ballot (mem prop)) .noOrder .exactlyOnce)
      (input p1as := p_to_acceptors_p1a) :=
    let s := H.bchain persisted p1as
    rebind (persisted := s)
    emit (a_max_ballot := H.bmax s);
  -- the max face: the block IS the running-max fold — a persisted stream
  -- chained and `.max()`ed per tick is `aMaxStep`'s trace (the generic
  -- `maxStep` at Ballot's lifted order is `Ballot.maxFold`;
  -- `Multiset.foldl_add` telescopes the accumulated pool)
  ghost have hmax : ∀ j, a_max_ballot j
      = foldAcrossTicksTrace aMaxStep none (p_to_acceptors_p1a j) :=
    fun j => by
    show scanAcrossTicksTrace (a_max_ballot_step j)
      (0 : Multiset (Ballot (mem prop))) (p_to_acceptors_p1a j) = _
    suffices hscan : ∀ (l : Trace (Multiset (Ballot (mem prop)))) (m : Multiset (Ballot (mem prop))),
        scanAcrossTicksTrace (a_max_ballot_step j) m l
          = foldAcrossTicksTrace aMaxStep (poolMax (ord := .noOrder) m) l from
      hscan (p_to_acceptors_p1a j) 0
    intro l
    induction l with
    | nil => intro m; rfl
    | cons b l ih =>
      intro m
      have h : poolMax (ord := .noOrder) (m + b)
          = aMaxStep (poolMax (ord := .noOrder) m) b := by
        show Multiset.foldl maxStep none (m + b)
          = @Multiset.foldl _ _ Ballot.maxFold
            ⟨fun s x y => Ballot.maxFold_comm s x y⟩ _ b
        simp only [Ballot.maxFold_eq_maxStep]
        exact Multiset.foldl_add _ _ _ _
      show poolMax (ord := .noOrder) (m + b) :: scanAcrossTicksTrace _ (m + b) l
        = aMaxStep (poolMax (ord := .noOrder) m) b
          :: foldAcrossTicksTrace aMaxStep (aMaxStep (poolMax (ord := .noOrder) m) b) l
      rw [← h, ih (m + b), h]
  -- p_to_acceptors_p1a
  --   .cross_singleton(a_max_ballot)
  --   .cross_singleton(a_log)
  --   .map(q!(|((ballot, max_ballot), log)| (ballot.proposer_id.clone(),
  --     (ballot.clone(), if Some(ballot) == max_ballot { Ok(log) } else { Err(max_ballot) }))))
  tick (input p1as' := p_to_acceptors_p1a) (input mb := a_max_ballot)
      (input lg := a_log) :=
    yield (replies := H.bmap
      (H.bcrossSingleton (H.bcrossSingleton p1as' mb) lg)
      (fun ((ballot, max_ballot), log) =>
        (ballot.proposerId.val,
          (⟨ballot,
            if some ballot = max_ballot then .ok log else .error max_ballot⟩
            : P1b P (mem prop)))));
  -- the reply face: the block IS the mapped zip over the fold's trace
  ghost have hreplies : ∀ j, replies j
      = (Trace.zip (p_to_acceptors_p1a j)
          (Trace.zip (foldAcrossTicksTrace aMaxStep none
            (p_to_acceptors_p1a j)) (a_log j))).map
        (fun bx => bx.1.map (fun a =>
          (a.proposerId.val,
            (⟨a, if some a = bx.2.1 then .ok bx.2.2
              else .error bx.2.1⟩ : P1b P (mem prop))))) := fun j => by
    show scanAcrossTicksTrace (replies_step j) ()
      (Trace.zip (p_to_acceptors_p1a j)
        (Trace.zip (a_max_ballot j) (a_log j))) = _
    rw [hmax j]
    refine Eq.trans (scanAcrossTicksTrace_stateless _ _) ?_
    refine List.map_congr_left (fun bx _ => ?_)
    show Multiset.map _ (Multiset.map
        (fun a => (a, show ALog P (mem prop) from bx.2.2))
        (Multiset.map
          (fun a => (a, show Option (Ballot (mem prop)) from bx.2.1))
          (show Multiset (Ballot (mem prop)) from bx.1))) = _
    rw [Multiset.map_map, Multiset.map_map]
    rfl
  -- per-sender reply characterization, at the reply wire: every routed
  -- reply quotes a P1a consumed at a realized tick, is routed to the
  -- ballot's owner, and its verdict compares against that tick's max
  -- and quotes that tick's `a_log` (write-before-ack)
  ghost have hsrc : ∀ (j : Fin (mem acc)) (r : Fin (mem prop)),
      ∀ m ∈ ap1From p_to_acceptors_p1a a_log j r,
      ∃ (t : Nat) (hb : t < (p_to_acceptors_p1a j).length)
        (hm : t < (a_max_ballot j).length)
        (hl : t < (a_log j).length),
        m.ballot ∈ (p_to_acceptors_p1a j)[t]'hb
        ∧ m.ballot.proposerId.val = r.val
        ∧ m.res = (if some m.ballot = (a_max_ballot j)[t]'hm
            then Except.ok ((a_log j)[t]'hl)
            else Except.error ((a_max_ballot j)[t]'hm)) := fun j r m hm1 => by
    simp only [hmax]
    -- demux: the address filter pins the routing
    obtain ⟨dx, hdx, haddr⟩ := (Multiset.mem_filterMap _ _).mp hm1
    by_cases hr : dx.1 = r.val
    · rw [if_pos hr] at haddr
      injection haddr with haddr'
      -- all_ticks: the reply's tick, and the tick's three reads
      obtain ⟨mt, hmt, hdx2⟩ := mem_list_sum.mp hdx
      obtain ⟨t, hb, hz, rfl⟩ := Trace.mem_zip_map hmt
      obtain ⟨hm2, hl, hzread⟩ := Trace.zip_getElem hz
      rw [hzread] at hdx2
      -- the payload shape pins everything
      obtain ⟨a, ha, hpay⟩ := Multiset.mem_map.mp hdx2
      have hfst : a.proposerId.val = dx.1 := congrArg Prod.fst hpay
      have hsnd : (⟨a, if some a = (foldAcrossTicksTrace aMaxStep none
            (p_to_acceptors_p1a j))[t]'hm2
          then .ok ((a_log j)[t]'hl)
          else .error ((foldAcrossTicksTrace aMaxStep none
            (p_to_acceptors_p1a j))[t]'hm2)⟩
            : P1b P (mem prop)) = dx.2 := congrArg Prod.snd hpay
      rw [haddr'] at hsnd
      refine ⟨t, hb, hm2, hl, ?_, ?_, ?_⟩
      · rw [show m.ballot = a from by rw [← hsnd]]
        exact ha
      · rw [show m.ballot = a from by rw [← hsnd], hfst, hr]
      · rw [show m.ballot = a from by rw [← hsnd], ← hsnd]
    · rw [if_neg hr] at haddr
      cases haddr
  -- per-sender reply cap, at the reply wire: replies are an
  -- elementwise image of the consumed batches
  ghost have hcap : ∀ (j : Fin (mem acc)) (r : Fin (mem prop))
      (b : Ballot (mem prop)),
      ((ap1From p_to_acceptors_p1a a_log j r).filter
        (fun m => m.ballot = b)).card
        ≤ ((p_to_acceptors_p1a j).sum).count b := fun j r b => by
    unfold ap1From
    rw [← Multiset.countP_eq_card_filter]
    refine le_trans (countP_filterMap_le _ _
      (fun dx : Nat × P1b P (mem prop) => dx.2.ballot = b)
      (fun dx m hdx hp => by
        by_cases hr : dx.1 = r.val
        · rw [if_pos hr] at hdx; injection hdx with h; rw [h]; exact hp
        · rw [if_neg hr] at hdx; cases hdx) _) ?_
    rw [countP_list_sum, count_list_sum]
    simp only [List.map_map, Trace.zip]
    refine sum_map_zip_le _ _ (fun x => ?_) (p_to_acceptors_p1a j) _
    show ((x.1.map (fun a =>
        (a.proposerId.val,
          (⟨a, if some a = x.2.1 then .ok x.2.2
            else .error x.2.1⟩ : P1b P (mem prop))))).countP
        (fun dx => dx.2.ballot = b)) ≤ (x.1).count b
    exact countP_map_le_count _ _ b (fun a hp => hp) x.1
  -- the demux face: the merged pool decomposes by sender into
  -- `ap1From` (through the reply face)
  ghost have hdecomp : ∀ (r : Fin (mem prop)),
      ((Values L mem).values ((Values L mem).demux sched.p1bCh
        ((Values L mem).allTicks replies) (fun r => r.val))) r
      = ((List.finRange (mem acc)).map
          (fun j => ap1From p_to_acceptors_p1a a_log j r)).sum := fun r => by
    refine congrArg List.sum (List.map_congr_left (fun j _ => ?_))
    show Multiset.filterMap _ ((replies j).sum) = ap1From _ _ j r
    rw [hreplies j]
    rfl
  -- .all_ticks().demux(proposers, …).values()
  (a_max_ballot,
    H.values (H.demux sched.p1bCh (H.allTicks replies) (fun r => r.val)))
  prove
    max_face := fun i => hmax i,
    max_mono := fun i => by
      show Ascending Ballot.obtVO (a_max_ballot i)
      rw [hmax i]
      -- `max` is inflationary on every consumed copy
      exact foldAcrossTicksTrace_ascending Ballot.obtVO aMaxStep none
        (fun s b => multiset_le_foldl Ballot.obtVO Ballot.maxFold
          Ballot.maxFold_comm Ballot.obtLE_maxFold b s)
        (p_to_acceptors_p1a i),
    reply_decomp := fun r => hdecomp r,
    from_ballot_cap := hcap,
    from_src := hsrc,
    reply_src := fun r m hm => by
      have hm0 : m ∈ ((List.finRange (mem acc)).map
          (fun j => ap1From p_to_acceptors_p1a a_log j r)).sum := by
        rw [← hdecomp r]; exact hm
      obtain ⟨mm, hmm, hm1⟩ := mem_list_sum.mp hm0
      obtain ⟨j, -, rfl⟩ := List.mem_map.mp hmm
      obtain ⟨t, hb, hm2, hl, hmem, hrt, hres⟩ := hsrc j r m hm1
      exact ⟨j, t, hb, hm2, hl, hmem, hrt, hres⟩


/-- **`Ok` promises pin the max** (corollary): an `Ok` reply's ballot
*is* `a_max_ballot` at its promise tick — with the contract's
`max_mono`, it bounds the max from below forever after. -/
theorem acceptor_p1_ok_pins (acc prop : L)
    (bs : Fin (mem acc) → Trace (Multiset (Ballot (mem prop))))
    (alog : TickV (mem acc) (ALog P (mem prop)))
    (ch : AP1Sched (Values L mem) (mem prop) (mem acc))
    (r : Fin (mem prop)) (m : P1b P (mem prop))
    (hm : m ∈ (acceptor_p1 (Values L mem) acc prop bs alog ch).2 r)
    (pl : ALog P (mem prop)) (hok : m.res = .ok pl) :
    ∃ (j : Fin (mem acc)) (t : Nat)
      (hm2 : t < ((acceptor_p1 (Values L mem) acc prop bs alog ch).1
        j).length),
      ((acceptor_p1 (Values L mem) acc prop bs alog ch).1 j)[t]'hm2
        = some m.ballot := by
  obtain ⟨j, t, hb, hm2, hl, -, -, hres⟩ :=
    (acceptor_p1.ensures acc prop bs alog ch).reply_src r m hm
  refine ⟨j, t, hm2, ?_⟩
  by_cases hc : some m.ballot
      = ((acceptor_p1 (Values L mem) acc prop bs alog ch).1 j)[t]'hm2
  · exact hc.symm
  · rw [if_neg hc] at hres
    rw [hres] at hok
    cases hok

/-! ## Executable smoke tests -/

-- One acceptor, one proposer: tick 0 consumes `{(1,0)}` (Ok, quoting the
-- tick's log wire), tick 1 consumes the stale `{(0,0)}` (Err, quoting
-- the pinned max).
#guard ((acceptor_p1 (Values PaxLoc (paxMem 1 1)) .acc .prop
    (fun _ => [{Ballot.mk 1 0}, {Ballot.mk 0 0}])
    (fun _ => [(none, []), (none, [(0, ⟨Ballot.mk 1 0, some 42⟩)])]
      : TickV 1 (ALog Nat 1)) .triv).1 0)
  = [some (Ballot.mk 1 0), some (Ballot.mk 1 0)]

#guard (acceptor_p1 (Values PaxLoc (paxMem 1 1)) .acc .prop
    (fun _ => [{Ballot.mk 1 0}, {Ballot.mk 0 0}])
    (fun _ => [(none, []), (none, [(0, ⟨Ballot.mk 1 0, some 42⟩)])]
      : TickV 1 (ALog Nat 1)) .triv).2 0
  = {⟨Ballot.mk 1 0, .ok (none, [])⟩,
     ⟨Ballot.mk 0 0, .error (some (Ballot.mk 1 0))⟩}

#nondet_census acceptor_p1 (nondets := 0) (scheds := 1) (fuels := 0)

end Hydro
