import HydroLean.Programs.Paxos.Types
import HydroLean.Hydro.TStream
import HydroLean.Hydro.ClusterFamily
import HydroLean.Hydro.Growth
import HydroLean.Hydro.MonoSing

/-!
# `acceptor_p1` (paxos.rs:485–524) — module

Acceptor phase-1 logic: maintain `a_max_ballot` (the running max over all
received P1as, `across_ticks(max)`, paxos.rs:498–502) and reply to each P1a
with `Ok(checkpoint, log)` iff its ballot *is* the current max, else
`Err(max_ballot)` (paxos.rs:506–521), routed to `ballot.proposer_id`.

The log snapshot placed in `Ok` is supplied by the `a_log` forward-ref cycle
(paxos.rs:165–166, 227–237), whose `snapshot_atomic` guarantee — *"we will
always write payloads to the log before acknowledging them"* — is realized in
the composed acceptor tick (`PaxosCore.lean`, `alog_spec`) by passing the
**post-P2a-merge** log of the same tick.

Module spec (the run faces below):
- the max ballot only grows — **carried by the output type** of
  `acceptor_p1_ticksM` (`MonoSing obtVO`, the `across_ticks(max)` fold with
  its `monotone =` obligation; the NoOrder batch is safe because max is
  commutative).
- `ap1_echo` / `ap1_pinsMax` / `ap1_okCnt` / `ap1_okOnce`: every reply quotes
  a consumed P1a; an `Ok` reply is issued exactly to the current max ballot,
  at most once per ballot.
-/

namespace HydroLean.Programs.Paxos

open HydroLean.Hydro

variable {P : Type} {nP : Nat}

/-- `a_max_ballot` after absorbing a P1a batch (paxos.rs:498–502). -/
def aMaxBallot (cur : Option (Ballot nP)) (p1as : List (Ballot nP)) :
    Option (Ballot nP) :=
  Ballot.maxList cur p1as

/-- The `a_max_ballot` value order, packaged (`none`-bottomed ballot
order). -/
def obtVO {nP : Nat} : HydroLean.Hydro.ValueOrder (Option (Ballot nP)) :=
  ⟨obtLE, obtLE_refl, fun h₁ h₂ => obtLE_trans h₁ h₂⟩

/-- One-element inflation: absorbing a ballot never lowers the max — the
`monotone =` closure obligation of `across_ticks(|s| s.max())`. -/
theorem obtLE_maxOpt (s : Option (Ballot nP)) (b : Ballot nP) :
    obtLE s (some (Ballot.maxOpt s b)) := by
  cases s with
  | none => trivial
  | some a =>
    show a.ble (a.max b) = true
    unfold Ballot.max
    by_cases h : a.blt b
    · rw [if_pos h]
      exact Ballot.ble_of_blt h
    · rw [if_neg h]
      exact Ballot.ble_refl a

/-! ## `acceptor_p1`, transcribed (paxos.rs:485–524)

The Rust body, line for line — the `across_ticks(|s| s.max()).into_singleton()`
state appears as the previous tick's value (`a_max_prev`, the tick face of the
unbounded fold), and the trailing `all_ticks().demux(proposers).values()` is
the host's edge boundary. The `a_log` singleton is the atomic wire from
`acceptor_p2` (the `a_log` forward-ref knot): which value flows on it is the
host's staging, so it is a tick input here. -/

/-- paxos.rs:485–524 `acceptor_p1` (one tick; `L` fixed to the accepted-log
payload as in `paxos_core`'s instantiation). -/
def acceptor_p1 (p_to_acceptors_p1a : Stream (Ballot nP))
    (a_log : Option Nat × LogMap P nP)
    (a_max_prev : Option (Ballot nP)) :
    Option (Ballot nP) × Stream (Fin nP × P1b P nP) :=
  let a_max_ballot := p_to_acceptors_p1a.acrossTicksMax a_max_prev Ballot.max
  (a_max_ballot,
    ((p_to_acceptors_p1a.crossSingleton a_max_ballot).crossSingleton a_log).map
      (fun ((ballot, max_ballot), log) =>
        (ballot.proposerId,
          ⟨ballot,
           if some ballot = max_ballot then .ok log
           else .error max_ballot⟩)))

/-- The tick face of `across_ticks(max)` is the compiled max-fold. -/
theorem acrossTicksMax_eq_maxList (p1as : Stream (Ballot nP))
    (cur : Option (Ballot nP)) :
    p1as.acrossTicksMax cur Ballot.max = Ballot.maxList cur p1as := by
  unfold Stream.acrossTicksMax Stream.fold Ballot.maxList
  congr 1
  funext acc b
  cases acc <;> rfl

/-- The transcription's max is the compiled tick-step's max (the two faces
cannot drift). -/
theorem acceptor_p1_max_eq (p1as : Stream (Ballot nP))
    (a_log : Option Nat × LogMap P nP) (cur : Option (Ballot nP)) :
    (acceptor_p1 p1as a_log cur).1 = aMaxBallot cur p1as :=
  acrossTicksMax_eq_maxList p1as cur

/-- The transcription's replies, as one `map` (fusing the two
`cross_singleton`s). -/
theorem acceptor_p1_out_eq (p1as : Stream (Ballot nP))
    (a_log : Option Nat × LogMap P nP) (cur : Option (Ballot nP)) :
    (acceptor_p1 p1as a_log cur).2
      = p1as.map (fun ballot =>
          (ballot.proposerId,
            (⟨ballot,
              if some ballot = aMaxBallot cur p1as then .ok a_log
              else .error (aMaxBallot cur p1as)⟩ : P1b P nP))) := by
  show ((p1as.crossSingleton (p1as.acrossTicksMax cur Ballot.max)).crossSingleton
      a_log).map _ = _
  unfold Stream.crossSingleton Stream.map
  rw [List.map_map, List.map_map, acrossTicksMax_eq_maxList]
  rfl

/-! ## The module's verified face: `acceptor_p1` over its own run

Proof outputs (unconditional, except `ap1_okOnce` whose hypothesis
materializes the send-once requirement on `p_to_acceptors_p1a` — the input
contract whose violation is FINDINGS.md B1):
- `ap1_max_mono` — `a_max_ballot` only grows across ticks (max-lattice fold;
  batch-order safety is the commutativity of `max`, a sealing fact of the
  collection, not a per-call obligation);
- `ap1_pinsMax` — an `Ok` promise pins the max at its ballot, forever;
- `ap1_echo` — replies echo consumed P1as, routed to the ballot's owner;
- `ap1_okCnt`/`ap1_okOnce` — `Ok`-promise counting against consumption;
- `promiseCarriesLog` is the *shape of the transcription*: tick `n`'s `Ok`
  promises carry exactly tick `n`'s `a_log` wire value
  (`acceptor_p1_out_eq`). -/

/-- The `Ok`-promise indicator for ballot `b`. -/
def isOkP1b (b : Ballot nP) (m : P1b P nP) : Bool :=
  decide (m.ballot = b) &&
    (match m.res with
     | .ok _ => true
     | .error _ => false)

/-- One `acceptor_p1` tick's input: the P1a batch and the tick's `a_log`
wire value. -/
structure AP1In (P : Type) (nP : Nat) where
  p1as : List (Ballot nP)
  a_log : Option Nat × LogMap P nP

/-- `acceptor_p1` folded over its tick inputs: state = `a_max_ballot`
(published each tick), output = the reply batch. -/
def acceptorP1Loop (P : Type) (nP : Nat) :
    TickLoop (AP1In P nP) (Option (Ballot nP)) (List (Fin nP × P1b P nP)) where
  init := none
  step s t := acceptor_p1 t.p1as t.a_log s

/-- The `a_max_ballot` value published after consuming `ins`. -/
def ap1Max (ins : List (AP1In P nP)) : Option (Ballot nP) :=
  (acceptorP1Loop P nP).finalState ins

/-- Cumulative reply stream over the consumption `ins`. -/
def ap1Replies (ins : List (AP1In P nP)) : List (Fin nP × P1b P nP) :=
  ((acceptorP1Loop P nP).outputs ins).flatten

/-- Cumulative consumed P1as. -/
def ap1Cons (ins : List (AP1In P nP)) : List (Ballot nP) :=
  (ins.map (·.p1as)).flatten

@[simp] theorem ap1Max_append (ins : List (AP1In P nP)) (t : AP1In P nP) :
    ap1Max (ins ++ [t]) = aMaxBallot (ap1Max ins) t.p1as := by
  unfold ap1Max
  rw [TickLoop.finalState_append]
  exact acceptor_p1_max_eq t.p1as t.a_log _

@[simp] theorem ap1Replies_append (ins : List (AP1In P nP)) (t : AP1In P nP) :
    ap1Replies (ins ++ [t])
      = ap1Replies ins
        ++ t.p1as.map (fun ballot =>
            (ballot.proposerId,
              (⟨ballot,
                if some ballot = aMaxBallot (ap1Max ins) t.p1as then
                  .ok t.a_log
                else .error (aMaxBallot (ap1Max ins) t.p1as)⟩ : P1b P nP))) := by
  unfold ap1Replies
  rw [TickLoop.outputs_append, List.flatten_append, List.flatten_cons,
    List.flatten_nil, List.append_nil]
  congr 1
  exact acceptor_p1_out_eq t.p1as t.a_log _

@[simp] theorem ap1Cons_append (ins : List (AP1In P nP)) (t : AP1In P nP) :
    ap1Cons (ins ++ [t]) = ap1Cons ins ++ t.p1as := by
  unfold ap1Cons
  simp

/-- The tick step never lowers the max (batch form of `obtLE_maxOpt`,
via `ValueOrder.foldl_le`). -/
theorem ap1_step_infl (s : Option (Ballot nP)) (t : AP1In P nP) :
    obtLE s ((acceptorP1Loop P nP).step s t).1 := by
  show obtLE s (acceptor_p1 t.p1as t.a_log s).1
  rw [acceptor_p1_max_eq]
  exact obtVO.foldl_le (g := fun acc b => some (Ballot.maxOpt acc b))
    obtLE_maxOpt t.p1as s

/-- `obtLE` composed with `aMaxBallot` growth (one tick). -/
theorem obtLE_aMaxBallot (cur : Option (Ballot nP)) (p1as : List (Ballot nP)) :
    obtLE cur (aMaxBallot cur p1as) :=
  obtVO.foldl_le (g := fun acc b => some (Ballot.maxOpt acc b))
    obtLE_maxOpt p1as cur

/-- **`a_max_ballot` is tick-monotone** — a projection of the wire's
`Monotonic` type (no hand induction). -/
theorem ap1_max_mono (ins ext : List (AP1In P nP)) :
    obtLE (ap1Max ins) (ap1Max (ins ++ ext)) :=
  (acceptorP1Loop P nP).finalState_le obtVO ap1_step_infl ins ext

/-- **Replies echo consumption**: every reply quotes a consumed P1a and is
routed to its owner. -/
theorem ap1_echo (ins : List (AP1In P nP)) :
    ∀ dm ∈ ap1Replies ins,
      (dm : Fin nP × P1b P nP).1 = dm.2.ballot.proposerId ∧
        dm.2.ballot ∈ ap1Cons ins := by
  induction ins using HydroLean.list_snoc_induction with
  | h0 => intro dm hdm; cases hdm
  | h1 ins t ih =>
    intro dm hdm
    rw [ap1Replies_append] at hdm
    rcases List.mem_append.mp hdm with hold | hnew
    · obtain ⟨h1, h2⟩ := ih dm hold
      exact ⟨h1, by rw [ap1Cons_append]; exact List.mem_append_left _ h2⟩
    · obtain ⟨b', hb', rfl⟩ := List.mem_map.mp hnew
      refine ⟨rfl, ?_⟩
      rw [ap1Cons_append]
      exact List.mem_append_right _ hb'

/-- **`Ok`-promise counting**: promises at `b` never outnumber the consumed
copies of `b`. -/
theorem ap1_okCnt (ins : List (AP1In P nP)) (b : Ballot nP) :
    (ap1Replies ins).countP (fun dm => isOkP1b b dm.2)
      ≤ (ap1Cons ins).count b := by
  induction ins using HydroLean.list_snoc_induction with
  | h0 => exact Nat.le_refl 0
  | h1 ins t ih =>
    rw [ap1Replies_append, ap1Cons_append, List.countP_append,
      List.count_append]
    refine Nat.add_le_add ih ?_
    rw [List.countP_map]
    refine List.countP_mono_left ?_
    intro b' _ hok
    have hok' : isOkP1b b
        (⟨b', if some b' = aMaxBallot (ap1Max ins) t.p1as then .ok t.a_log
          else .error (aMaxBallot (ap1Max ins) t.p1as)⟩ : P1b P nP)
        = true := hok
    unfold isOkP1b at hok'
    rw [Bool.and_eq_true, decide_eq_true_iff] at hok'
    have : b' = b := hok'.1
    simpa using this

/-- **`Ok` promises pin the max**: an `Ok` promise's ballot bounds
`a_max_ballot` from below, forever (with `ap1_max_mono`). -/
theorem ap1_pinsMax (ins : List (AP1In P nP)) :
    ∀ dm ∈ ap1Replies ins, (∃ pl, (dm : Fin nP × P1b P nP).2.res = .ok pl) →
      obtLE (some dm.2.ballot) (ap1Max ins) := by
  induction ins using HydroLean.list_snoc_induction with
  | h0 => intro dm hdm; cases hdm
  | h1 ins t ih =>
    intro dm hdm hok
    rw [ap1Replies_append] at hdm
    rw [ap1Max_append]
    rcases List.mem_append.mp hdm with hold | hnew
    · exact obtLE_trans (ih dm hold hok) (obtLE_aMaxBallot _ _)
    · obtain ⟨b', hb', rfl⟩ := List.mem_map.mp hnew
      obtain ⟨pl, hres⟩ := hok
      have hbm : some b' = aMaxBallot (ap1Max ins) t.p1as := by
        by_cases h : some b' = aMaxBallot (ap1Max ins) t.p1as
        · exact h
        · rw [show (⟨b', if some b' = aMaxBallot (ap1Max ins) t.p1as then
              .ok t.a_log else .error (aMaxBallot (ap1Max ins) t.p1as)⟩
                : P1b P nP).res
              = .error (aMaxBallot (ap1Max ins) t.p1as) from by
              rw [if_neg h]] at hres
          cases hres
      show obtLE (some b') (aMaxBallot (ap1Max ins) t.p1as)
      rw [← hbm]
      exact Ballot.ble_refl _

/-- **At most one `Ok` promise per ballot**, given the module's input
requirement: the consumed P1a stream is duplicate-free (the send-once
contract on `p_to_acceptors_p1a`, whose violation is FINDINGS.md B1). -/
theorem ap1_okOnce (ins : List (AP1In P nP))
    (hnd : (ap1Cons ins).Nodup) (b : Ballot nP) :
    (ap1Replies ins).countP (fun dm => isOkP1b b dm.2) ≤ 1 :=
  Nat.le_trans (ap1_okCnt ins b)
    (HydroLean.Hydro.count_le_one_of_nodup hnd b)

/-- **Per-proposer decoded reply cap**: with duplicate-free consumed P1as
(the send-once contract, B1), the demuxed `(ballot, result)` slice carries
at most one `Ok` per ballot — the `collect_quorum_with_response` input
requirement, discharged at the module that owns the replies. -/
theorem ap1_decode_cap (ins : List (AP1In P nP))
    (hnd : (ap1Cons ins).Nodup) (i : Fin nP) (b : Ballot nP) :
    ((ap1Replies ins).filterMap (fun dm => if dm.1 = i then
        some (dm.2.ballot, dm.2.res) else none)).countP
      (fun e => decide (e.1 = b) && e.2.isOk) ≤ 1 := by
  refine Nat.le_trans (countP_filterMap_le ?_) (ap1_okOnce ins hnd b)
  intro dm _ e hg hp
  by_cases hi : dm.1 = i
  · rw [if_pos hi] at hg
    cases hg
    rw [Bool.and_eq_true, decide_eq_true_iff] at hp
    unfold isOkP1b
    rw [Bool.and_eq_true, decide_eq_true_iff]
    refine ⟨hp.1, ?_⟩
    cases hres : dm.2.res with
    | ok _ => rfl
    | error _ =>
      rw [hres] at hp
      cases hp.2
  · rw [if_neg hi] at hg
    cases hg

/-! ## `acceptor_p1` across ticks (the module at the located surface)

The multi-tick face (`Hydro/TStream.lean`): the P1a tick batches zipped with
the `a_log` wire (blocking — a tick's replies exist only once its `a_log`
value is realized, which is exactly the `snapshot_atomic` write-before-ack
staging), scanned by the transcription. -/

/-- `acceptor_p1` lifted across ticks. Returns
(the `a_max_ballot` wire **at its `Monotonic` type** — the
`across_ticks(max)` fold with its `monotone =` obligation
(`obtLE_aMaxBallot`) paid here — and the p1b reply batches per tick).

**Productivity (the `a_log` knot)**: `a_max_ballot` is the
`across_ticks(max)` fold of the P1a batches **alone** (paxos.rs:498–502 —
it does not read `a_log`), so the max wire ticks as soon as batches are
consumed. Only the p1b *replies* block on the same tick's `a_log` wire
(the `snapshot_atomic` write-before-ack staging): a tick's replies exist
only once its post-merge log value is realized by the `paxos_core`
`forward_ref`. Blocking the max on the log too would deadlock the knot
(the log needs the max through `acceptor_p2`) — the executable
falsification caught exactly that vacuity (FINDINGS D15). -/
def acceptor_p1_ticksM :
    TStream (Ballot nP) × TSing (Option Nat × LogMap P nP)
      →ₘ MonoSing (obtVO (nP := nP)) × TStream (Fin nP × P1b P nP) :=
  let p1a := MonoMap.fst
  let a_log := MonoMap.snd
  -- the tick inputs: P1a batches zipped with the (blocking) a_log wire
  let ins := (p1a.zip a_log).map (fun bl => AP1In.mk bl.1 bl.2)
  -- a_max_ballot = p1a.across_ticks(|s| s.max())  [monotonic = obtLE_aMaxBallot]
  let a_max_ballot := p1a.foldMonotonic obtVO (fun s b => aMaxBallot s b)
    none (fun s b => obtLE_aMaxBallot s b)
  MonoMap.pair a_max_ballot (ins.loop (acceptorP1Loop P nP))

/-- The reply-side running max agrees with the batch fold on realized reply
ticks (the two faces of the same `across_ticks(max)`). -/
theorem ap1Max_take_eq_batchFold (p1a : TStream (Ballot nP))
    (a_log : TSing (Option Nat × LogMap P nP)) {n : Nat}
    (hn : n ≤ ((TSing.zip p1a a_log).map (fun bl =>
      AP1In.mk bl.1 bl.2)).length) :
    ap1Max (((TSing.zip p1a a_log).map (fun bl =>
        AP1In.mk bl.1 bl.2)).take n)
      = (p1a.take n).foldl (fun s b => aMaxBallot s b) none := by
  unfold ap1Max
  rw [← foldl_eq_finalState]
  have hfn : (fun (s : Option (Ballot nP)) (b : AP1In P nP) =>
      ((acceptorP1Loop P nP).step s b).1)
      = fun s b => aMaxBallot s b.p1as := by
    funext s b
    exact acceptor_p1_max_eq b.p1as b.a_log s
  rw [hfn, show (acceptorP1Loop P nP).init = (none : Option (Ballot nP))
    from rfl]
  rw [show (((TSing.zip p1a a_log).map (fun bl =>
      AP1In.mk bl.1 bl.2)).take n).foldl
      (fun s (b : AP1In P nP) => aMaxBallot s b.p1as) none
    = ((((TSing.zip p1a a_log).map (fun bl =>
        AP1In.mk bl.1 bl.2)).take n).map (·.p1as)).foldl
      (fun s b => aMaxBallot s b) none from by
    rw [List.foldl_map]]
  congr 1
  rw [List.map_take]
  have hfst : ((TSing.zip p1a a_log).map (fun bl =>
      AP1In.mk bl.1 bl.2)).map (·.p1as)
      = (List.zip p1a a_log).map Prod.fst := by
    show List.map _ (List.map _ _) = _
    rw [List.map_map]
    rfl
  rw [hfst]
  exact prefix_take_eq (zip_fst_prefix _ _) n (by
    rw [List.length_map]
    have := hn
    rwa [List.length_map] at this)

/-! ## Output elimination (module face) -/

/-- Tick-`t` p1b replies, characterized (`acceptor_p1_out_eq` at the loop):
each reply quotes a consumed P1a of its tick, is judged against the tick's
running max, and — when `Ok` — carries the tick's `a_log` wire value. -/
theorem acceptorP1_out_elim {ins : List (AP1In P nP)}
    {dm : Fin nP × P1b P nP}
    (hdm : dm ∈ ((acceptorP1Loop P nP).outputs ins).flatten) :
    ∃ t, ∃ ht : t < (ins).length,
      ∃ b' ∈ ((ins)[t]'ht).p1as,
      dm = (b'.proposerId,
        ⟨b', if some b' = ap1Max ((ins).take (t + 1))
             then .ok ((ins)[t]'ht).a_log
             else .error (ap1Max ((ins).take (t + 1)))⟩) := by
  obtain ⟨t, ht, hstep⟩ := (acceptorP1Loop P nP).mem_outputs_elim hdm
  refine ⟨t, ht, ?_⟩
  have hout := acceptor_p1_out_eq ((ins)[t]'ht).p1as
    ((ins)[t]'ht).a_log
    ((acceptorP1Loop P nP).finalState ((ins).take t))
  rw [show ((acceptorP1Loop P nP).step
      ((acceptorP1Loop P nP).finalState ((ins).take t))
      ((ins)[t]'ht)).2
    = (acceptor_p1 ((ins)[t]'ht).p1as
        ((ins)[t]'ht).a_log
        ((acceptorP1Loop P nP).finalState
          ((ins).take t))).2 from rfl, hout] at hstep
  obtain ⟨b', hb', rfl⟩ := List.mem_map.mp hstep
  have hmax : aMaxBallot
      ((acceptorP1Loop P nP).finalState ((ins).take t))
      ((ins)[t]'ht).p1as
      = ap1Max ((ins).take (t + 1)) := by
    rw [take_succ_eq (ins) t ht]
    exact (ap1Max_append ((ins).take t)
      ((ins)[t]'ht)).symm
  rw [hmax]
  exact ⟨b', hb', rfl⟩

/-- **`Ok`-promise elimination (module face)**: an `Ok` reply pins its
tick — the promised ballot is in that tick's consumed batch *and* is the
running max at the tick, and the carried payload is exactly the tick's
`a_log` wire value (the write-before-ack staging; paxos.rs:506–521). -/
theorem acceptorP1_ok_elim {ins : List (AP1In P nP)}
    {dm : Fin nP × P1b P nP}
    (hdm : dm ∈ ((acceptorP1Loop P nP).outputs ins).flatten)
    {pay : Option Nat × LogMap P nP} (hok : dm.2.res = .ok pay) :
    ∃ t, ∃ ht : t < ins.length,
      dm.2.ballot ∈ (ins[t]'ht).p1as ∧
      some dm.2.ballot = ap1Max (ins.take (t + 1)) ∧
      pay = (ins[t]'ht).a_log := by
  obtain ⟨t, ht, b', hb', rfl⟩ := acceptorP1_out_elim hdm
  by_cases hbm : some b' = ap1Max (ins.take (t + 1))
  case pos =>
    refine ⟨t, ht, hb', hbm, ?_⟩
    rw [show ((b'.proposerId,
        ⟨b', if some b' = ap1Max (ins.take (t + 1))
          then .ok (ins[t]'ht).a_log
          else .error (ap1Max (ins.take (t + 1)))⟩) :
        Fin nP × P1b P nP).2.res
      = .ok (ins[t]'ht).a_log from by
        show (if some b' = _ then _ else _ : Except _ _) = _
        rw [if_pos hbm]] at hok
    exact (Except.ok.inj hok).symm
  case neg =>
    exfalso
    rw [show ((b'.proposerId,
        ⟨b', if some b' = ap1Max (ins.take (t + 1))
          then .ok (ins[t]'ht).a_log
          else .error (ap1Max (ins.take (t + 1)))⟩) :
        Fin nP × P1b P nP).2.res
      = .error (ap1Max (ins.take (t + 1))) from by
        show (if some b' = _ then _ else _ : Except _ _) = _
        rw [if_neg hbm]] at hok
    cases hok

/-! ## The module contract at the ticks signature

Stated on `acceptor_p1_ticksM`'s own I/O — inputs `(p1a, al)` (the consumed
P1a tick batches and the blocking `a_log` wire), outputs (the `Monotonic`
max wire and the reply batches). Callers (`leader_election`) consume these;
the loop/zip plumbing never escapes this file. -/

section TicksContract

variable {p1a : TStream (Ballot nP)} {al : TSing (Option Nat × LogMap P nP)}

/-- **`Ok`-promise contract**: an `Ok` reply pins a tick — the carried
payload is the `a_log` **input** value at that tick, and the `Monotonic`
max **output** wire carries exactly the promised ballot there. -/
theorem ap1t_ok_spec {dm : Fin nP × P1b P nP}
    (hdm : dm ∈ ((acceptor_p1_ticksM.f (p1a, al)).2).flatten)
    {pay : Option Nat × LogMap P nP} (hok : dm.2.res = .ok pay) :
    ∃ (t : Nat) (htl : t < al.length),
      pay = al[t]'htl ∧
      ∃ hm : t < ((acceptor_p1_ticksM.f (p1a, al)).1).vals.length,
        ((acceptor_p1_ticksM.f (p1a, al)).1).vals[t]'hm
          = some dm.2.ballot := by
  have hdm' : dm ∈ ((acceptorP1Loop P nP).outputs
      ((TSing.zip p1a al).map fun bl => AP1In.mk bl.1 bl.2)).flatten := hdm
  obtain ⟨t, ht, hbmem, hmax, hpay⟩ := acceptorP1_ok_elim hdm' hok
  have hzip : t < (List.zip p1a al).length := by
    have := ht
    rwa [List.length_map] at this
  have htl : t < al.length := by
    have := hzip
    rw [List.length_zip] at this
    omega
  have htb : t < p1a.length := by
    have := hzip
    rw [List.length_zip] at this
    omega
  have hlog : (((TSing.zip p1a al).map
      fun bl => AP1In.mk bl.1 bl.2)[t]'ht).a_log = al[t]'htl := by
    rw [List.getElem_map]
    show ((List.zip p1a al)[t]'hzip).2 = _
    rw [List.getElem_zip]
  have hsc : t < (scanSt (fun s b => aMaxBallot s b) none p1a).length := by
    rw [scanSt_length]
    exact htb
  have hm : t < ((acceptor_p1_ticksM.f (p1a, al)).1).vals.length := hsc
  refine ⟨t, htl, hpay.trans hlog, hm, ?_⟩
  have hval : ((acceptor_p1_ticksM.f (p1a, al)).1).vals[t]'hm
      = (p1a.take (t + 1)).foldl (fun s b => aMaxBallot s b) none :=
    scanSt_getElem _ _ _ t hsc
  rw [hval, ← ap1Max_take_eq_batchFold p1a al (Nat.succ_le_of_lt ht),
    ← hmax]

/-- **Reply echo contract**: every reply quotes a consumed P1a — its ballot
occurs on the P1a **input**. -/
theorem ap1t_reply_echo {dm : Fin nP × P1b P nP}
    (hdm : dm ∈ ((acceptor_p1_ticksM.f (p1a, al)).2).flatten) :
    dm.2.ballot ∈ p1a.flatten := by
  have hdm' : dm ∈ ((acceptorP1Loop P nP).outputs
      ((TSing.zip p1a al).map fun bl => AP1In.mk bl.1 bl.2)).flatten := hdm
  obtain ⟨t, ht, b', hb', rfl⟩ := acceptorP1_out_elim hdm'
  have hzip : t < (List.zip p1a al).length := by
    have := ht
    rwa [List.length_map] at this
  have htb : t < p1a.length := by
    have := hzip
    rw [List.length_zip] at this
    omega
  have hproj : ((((TSing.zip p1a al).map
      fun bl => AP1In.mk bl.1 bl.2))[t]'ht).p1as = p1a[t]'htb := by
    rw [List.getElem_map]
    show ((List.zip p1a al)[t]'hzip).1 = _
    rw [List.getElem_zip]
  rw [hproj] at hb'
  exact List.mem_flatten.mpr ⟨_, List.getElem_mem htb, hb'⟩

/-- **Reply routing contract**: every reply is addressed to its ballot's
owner (`.demux(proposers)` by `proposer_id`, paxos.rs:329). -/
theorem ap1t_reply_dst {dm : Fin nP × P1b P nP}
    (hdm : dm ∈ ((acceptor_p1_ticksM.f (p1a, al)).2).flatten) :
    dm.1 = dm.2.ballot.proposerId := by
  have hdm' : dm ∈ ((acceptorP1Loop P nP).outputs
      ((TSing.zip p1a al).map fun bl => AP1In.mk bl.1 bl.2)).flatten := hdm
  obtain ⟨t, ht, b', hb', rfl⟩ := acceptorP1_out_elim hdm'
  rfl

/-- **Decode-cap contract**: with duplicate-free P1a **input** (the B1
send-once fan-in discipline), the demuxed reply slice carries at most one
`Ok` promise per ballot. -/
theorem ap1t_decode_cap (hnd : (p1a.flatten).Nodup) (i : Fin nP)
    (b : Ballot nP) :
    ((((acceptor_p1_ticksM.f (p1a, al)).2).flatten).filterMap
      (fun dm => if dm.1 = i then
        some (dm.2.ballot, dm.2.res) else none)).countP
      (fun e => decide (e.1 = b) && e.2.isOk) ≤ 1 := by
  refine ap1_decode_cap _ ?_ i b
  have hpre : ((((TSing.zip p1a al).map
      fun bl => AP1In.mk bl.1 bl.2)).map (·.p1as)) <+: p1a := by
    rw [show ((((TSing.zip p1a al).map
        fun bl => AP1In.mk bl.1 bl.2)).map (·.p1as))
      = (List.zip p1a al).map Prod.fst from by
      show List.map _ (List.map _ _) = _
      rw [List.map_map]
      rfl]
    exact zip_fst_prefix _ _
  exact hnd.sublist (prefix_flatten hpre).sublist

end TicksContract

end HydroLean.Programs.Paxos
