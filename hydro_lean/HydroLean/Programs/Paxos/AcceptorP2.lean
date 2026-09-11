import HydroLean.Programs.Paxos.Types
import HydroLean.Hydro.TStream
import HydroLean.Hydro.ClusterFamily
import HydroLean.Hydro.Growth
import HydroLean.Hydro.MonoSing

/-!
# `acceptor_p2` (paxos.rs:809–899) — module

Acceptor phase-2 logic: filter incoming P2as against the current max ballot
(`a_p2as_to_place_in_log`, paxos.rs:838–850: keep iff `Some(p2a.ballot) >=
max_ballot`), merge the kept entries into the per-slot log keeping the
higher-ballot entry (`reduce_watermark`, paxos.rs:851–864; ties keep the
existing entry), and ack every P2a with `Ok(())` iff its ballot *is* the max,
keyed `(slot, ballot)` (paxos.rs:875–890). Checkpointing dropped as
authorized (`a_checkpoint = none`; no watermark GC).

The Rust `manual_proof!(/** max by ballot (TODO: not if two entries with same
ballot, need assume) */)` (paxos.rs:862) — commutativity of the per-slot
merge — holds under the one-value-per-(slot, ballot) hypothesis that phase 2
establishes as a system invariant (`Paxos/Safety.lean`); the Rust TODO is
precisely this missing `assume`.

Module spec:
- `insertMax_find_*`: the per-slot merge keeps the max-ballot entry and
  touches no other slot;
- `aP2bReply_ok_iff`: an `Ok` ack is issued exactly to the current max ballot;
- `accepted_means_logged`: if a P2a in the batch is acked `Ok`, the post-merge
  log holds an entry at its slot with ballot ≥ the acked ballot — the
  *"log written before acknowledging"* guarantee (paxos.rs:227–236,
  `snapshot_atomic`; write-before-release, `Hydro/Atomic.lean`), in the
  compositional form the safety proof consumes.
-/

namespace HydroLean.Programs.Paxos

open HydroLean.Hydro

variable {P : Type} {nP : Nat}

/-- `Some(p2a.ballot) >= max_ballot` (paxos.rs:842): `none` max accepts
everything. -/
def p2aQualifies (max' : Option (Ballot nP)) (m : P2a P nP) : Bool :=
  match max' with
  | none => true
  | some mb => mb.ble m.ballot

/-- Place qualifying P2as into the log (paxos.rs:838–864). -/
def aLogAfter (max' : Option (Ballot nP)) (log : LogMap P nP)
    (p2as : List (P2a P nP)) : LogMap P nP :=
  p2as.foldl
    (fun l m =>
      if p2aQualifies max' m then l.insertMax m.slot ⟨m.ballot, m.value⟩
      else l)
      log

theorem aLogAfter_cons (max' : Option (Ballot nP)) (log : LogMap P nP)
    (a : P2a P nP) (rest : List (P2a P nP)) :
    aLogAfter max' log (a :: rest) =
      aLogAfter max'
        (if p2aQualifies max' a then log.insertMax a.slot ⟨a.ballot, a.value⟩
         else log) rest := by
  simp only [aLogAfter, List.foldl_cons]

/-- The p2b ack for one P2a (paxos.rs:875–890): destination `p2a.sender`,
key `(slot, ballot)`, `Ok` iff the ballot is the current max. -/
def aP2bReply (max' : Option (Ballot nP)) (m : P2a P nP) :
    Fin nP × P2b nP :=
  (m.sender,
    ⟨m.slot, m.ballot, if some m.ballot = max' then .ok () else .error max'⟩)

/-! ## Spec lemmas -/

/-- `insertMax` does not touch other slots. -/
theorem insertMax_find_other {log : LogMap P nP} {slot s : Nat}
    (v : LogValue P nP) (h : s ≠ slot) :
    (log.insertMax slot v).find? s = log.find? s := by
  induction log with
  | nil =>
    show LogMap.find? [(slot, v)] s = _
    rw [LogMap.find?_cons, if_neg (Ne.symm h)]
  | cons e rest ih =>
    obtain ⟨s', w⟩ := e
    simp only [LogMap.insertMax]
    by_cases hs' : s' = slot
    · subst hs'
      rw [if_pos rfl]
      by_cases hb : w.ballot.blt v.ballot <;>
        simp [hb, LogMap.find?_cons, Ne.symm h]
    · rw [if_neg hs', LogMap.find?_cons, LogMap.find?_cons, ih]

/-- `insertMax` into an absent slot stores the new entry. -/
theorem insertMax_find_self_none {log : LogMap P nP} {slot : Nat}
    (v : LogValue P nP) (h : log.find? slot = none) :
    (log.insertMax slot v).find? slot = some v := by
  induction log with
  | nil =>
    show LogMap.find? [(slot, v)] slot = _
    rw [LogMap.find?_cons, if_pos rfl]
  | cons e rest ih =>
    obtain ⟨s', w⟩ := e
    rw [LogMap.find?_cons] at h
    simp only [LogMap.insertMax]
    by_cases hs' : s' = slot
    · rw [if_pos hs'] at h
      cases h
    · rw [if_neg hs'] at h
      rw [if_neg hs', LogMap.find?_cons, if_neg hs']
      exact ih h

/-- `insertMax` into a present slot keeps the higher-ballot entry (ties keep
the existing one, paxos.rs:856). -/
theorem insertMax_find_self_some {log : LogMap P nP} {slot : Nat}
    {w : LogValue P nP} (v : LogValue P nP) (h : log.find? slot = some w) :
    (log.insertMax slot v).find? slot
      = some (if w.ballot.blt v.ballot then v else w) := by
  induction log with
  | nil => simp at h
  | cons e rest ih =>
    obtain ⟨s', w'⟩ := e
    rw [LogMap.find?_cons] at h
    simp only [LogMap.insertMax]
    by_cases hs' : s' = slot
    · rw [if_pos hs'] at h
      cases h
      rw [if_pos hs']
      by_cases hb : w.ballot.blt v.ballot
      · rw [if_pos hb, if_pos hb, LogMap.find?_cons, if_pos hs']
      · rw [if_neg hb, if_neg hb, LogMap.find?_cons, if_pos hs']
    · rw [if_neg hs'] at h
      rw [if_neg hs', LogMap.find?_cons, if_neg hs']
      exact ih h

/-- Spec: an `Ok` p2b ack is issued iff the P2a's ballot is exactly the
current max (paxos.rs:881). -/
theorem aP2bReply_ok_iff (max' : Option (Ballot nP)) (m : P2a P nP) :
    (aP2bReply max' m).2.res = .ok () ↔ some m.ballot = max' := by
  unfold aP2bReply
  by_cases h : some m.ballot = max' <;> simp [h]

/-- Slot-ballot lower bounds survive further merges: if the log holds an
entry with ballot ≥ b at `slot`, it still does after any batch of inserts. -/
theorem aLogAfter_ballot_lb (max' : Option (Ballot nP))
    (rest : List (P2a P nP)) (log : LogMap P nP) {slot : Nat} {b : Ballot nP}
    (h : ∃ e : LogValue P nP, log.find? slot = some e ∧
      (e.ballot = b ∨ b.blt e.ballot = true)) :
    ∃ e : LogValue P nP, (aLogAfter max' log rest).find? slot = some e ∧
      (e.ballot = b ∨ b.blt e.ballot = true) := by
  induction rest generalizing log with
  | nil => exact h
  | cons a t ih =>
    rw [aLogAfter_cons]
    by_cases hq : p2aQualifies max' a
    · rw [if_pos hq]
      refine ih _ ?_
      obtain ⟨e, he, hbe⟩ := h
      by_cases hs : slot = a.slot
      · subst hs
        rw [insertMax_find_self_some ⟨a.ballot, a.value⟩ he]
        by_cases hb : e.ballot.blt a.ballot
        · refine ⟨_, rfl, ?_⟩
          rw [if_pos hb]
          refine Or.inr ?_
          show b.blt a.ballot = true
          rcases hbe with rfl | hbb
          · exact hb
          · rw [Ballot.blt_iff] at *; omega
        · exact ⟨_, rfl, by rw [if_neg hb]; exact hbe⟩
      · rw [insertMax_find_other _ hs]
        exact ⟨e, he, hbe⟩
    · rw [if_neg hq]
      exact ih _ h

/-- Entries of a merged log are old entries or echoes of batch P2as
(the membership cases of the whole-batch merge). -/
theorem aLogAfter_mem_cases {max' : Option (Ballot nP)} {log : LogMap P nP}
    {p2as : List (P2a P nP)} {x : Nat × LogValue P nP}
    (h : x ∈ aLogAfter max' log p2as) :
    x ∈ log ∨ ∃ m ∈ p2as, x = (m.slot, ⟨m.ballot, m.value⟩) := by
  induction p2as generalizing log with
  | nil => exact Or.inl h
  | cons a rest ih =>
    rw [aLogAfter_cons] at h
    by_cases hq : p2aQualifies max' a
    · rw [if_pos hq] at h
      rcases ih h with hold | ⟨m, hm, hx⟩
      · rcases LogMap.insertMax_mem_cases hold with hlog | hnew
        · exact Or.inl hlog
        · exact Or.inr ⟨a, List.mem_cons_self .., hnew⟩
      · exact Or.inr ⟨m, List.mem_cons_of_mem _ hm, hx⟩
    · rw [if_neg hq] at h
      rcases ih h with hold | ⟨m, hm, hx⟩
      · exact Or.inl hold
      · exact Or.inr ⟨m, List.mem_cons_of_mem _ hm, hx⟩

/-- **Write-before-release, compositionally**: if a P2a in this batch is
acked `Ok`, the post-merge log has an entry at its slot with ballot ≥ the
acked ballot (possibly higher, from another P2a in the same batch). -/
theorem accepted_means_logged (max' : Option (Ballot nP))
    (log : LogMap P nP) (p2as : List (P2a P nP)) (m : P2a P nP)
    (hm : m ∈ p2as) (hok : some m.ballot = max') :
    ∃ e : LogValue P nP, (aLogAfter max' log p2as).find? m.slot = some e ∧
      (e.ballot = m.ballot ∨ m.ballot.blt e.ballot = true) := by
  induction p2as generalizing log with
  | nil => cases hm
  | cons a rest ih =>
    rw [aLogAfter_cons]
    rcases List.mem_cons.mp hm with rfl | hmem
    · have hq : p2aQualifies max' m = true := by
        unfold p2aQualifies
        rw [← hok]
        exact Ballot.ble_iff.mpr (Or.inr ⟨rfl, Nat.le_refl _⟩)
      rw [if_pos hq]
      refine aLogAfter_ballot_lb max' rest _ ?_
      cases hf : log.find? m.slot with
      | none =>
        rw [insertMax_find_self_none ⟨m.ballot, m.value⟩ hf]
        exact ⟨_, rfl, Or.inl rfl⟩
      | some w =>
        rw [insertMax_find_self_some ⟨m.ballot, m.value⟩ hf]
        by_cases hb : w.ballot.blt m.ballot
        · exact ⟨_, rfl, by rw [if_pos hb]; exact Or.inl rfl⟩
        · refine ⟨_, rfl, ?_⟩
          rw [if_neg hb]
          -- totality: ¬(w < m) ⇒ m ≤ w ⇒ m = w ∨ m < w
          rcases Ballot.eq_or_blt_of_ble (Ballot.ble_of_not_blt hb) with
            heq | hlt
          · exact Or.inl heq.symm
          · exact Or.inr hlt
    · exact (if hq : p2aQualifies max' a then by rw [if_pos hq]; exact ih _ hmem
        else by rw [if_neg hq]; exact ih _ hmem)


/-! ## `acceptor_p2`, transcribed (paxos.rs:809–899)

The Rust body, line for line — the `across_ticks(reduce_watermark)` state is
the previous tick's log (the tick face of the unbounded keyed reduce; the
checkpoint watermark's garbage collection is dropped as authorized), the
`.batch(acceptor_tick, nondet!)` site is the tick input itself, and
`all_ticks().demux(proposers).values()` is the host's edge boundary. The
`manual_proof!(max by ballot (TODO: not if two entries with same ballot,
need assume))` at paxos.rs:862 is materialized downstream as the key-nodup
input requirement of the run-level clauses (`ap2_okOnce`) and as the
uniqueness hypothesis of the merge's order-invariance. -/

open HydroLean.Hydro in
/-- paxos.rs:809–899 `acceptor_p2` (one tick). -/
def acceptor_p2 (a_max_ballot : Option (Ballot nP))
    (p_to_acceptors_p2a_batch : Stream (P2a P nP))
    (a_checkpoint : Option Nat)
    (a_log_prev : LogMap P nP) :
    (Option Nat × LogMap P nP) × Stream (Fin nP × P2b nP) :=
  let a_p2as_to_place_in_log :=
    (p_to_acceptors_p2a_batch.crossSingleton a_max_ballot).filterMap
      (fun (p2a, max_ballot) =>
        if p2aQualifies max_ballot p2a then
          some (p2a.slot, LogValue.mk p2a.ballot p2a.value)
        else none)
  let a_log := a_p2as_to_place_in_log.acrossTicksKeyedReduce a_log_prev
    (fun prev_entry entry =>
      if prev_entry.ballot.blt entry.ballot then entry else prev_entry)
  let a_to_proposers_p2b :=
    (p_to_acceptors_p2a_batch.crossSingleton a_max_ballot).map
      (fun (p2a, max_ballot) =>
        (p2a.sender,
          (⟨p2a.slot, p2a.ballot,
            if some p2a.ballot = max_ballot then .ok () else .error max_ballot⟩
            : P2b nP)))
  ((a_checkpoint, a_log), a_to_proposers_p2b)

/-- `insert_with` at the ballot-max closure is the compiled `insertMax`. -/
theorem insertWith_maxComb_eq (log : LogMap P nP) (slot : Nat)
    (v : LogValue P nP) :
    HydroLean.Hydro.Stream.insertWith
        (fun prev_entry entry =>
          if prev_entry.ballot.blt entry.ballot then entry else prev_entry)
        slot v log
      = log.insertMax slot v := by
  induction log with
  | nil => rfl
  | cons hd rest ih =>
    obtain ⟨s, w⟩ := hd
    simp only [HydroLean.Hydro.Stream.insertWith, LogMap.insertMax]
    by_cases hs : s = slot
    · rw [if_pos hs, if_pos hs]
      by_cases hb : w.ballot.blt v.ballot
      · rw [if_pos hb, if_pos hb]
      · rw [if_neg hb, if_neg hb]
    · rw [if_neg hs, if_neg hs, ih]

/-- The transcription's log is the compiled whole-batch merge. -/
theorem acceptor_p2_log_eq (max' : Option (Ballot nP))
    (p2as : Stream (P2a P nP)) (chk : Option Nat) (log : LogMap P nP) :
    (acceptor_p2 max' p2as chk log).1.2 = aLogAfter max' log p2as := by
  show (HydroLean.Hydro.Stream.acrossTicksKeyedReduce
    ((Stream.crossSingleton p2as max').filterMap _) log _) = _
  induction p2as generalizing log with
  | nil => rfl
  | cons m rest ih =>
    rw [aLogAfter_cons]
    show HydroLean.Hydro.Stream.acrossTicksKeyedReduce
      (List.filterMap _ ((m, max') :: Stream.crossSingleton rest max')) log _
        = _
    rw [List.filterMap_cons]
    dsimp only
    by_cases hq : p2aQualifies max' m
    · rw [if_pos hq, if_pos hq]
      show HydroLean.Hydro.Stream.acrossTicksKeyedReduce
        (List.filterMap _ (Stream.crossSingleton rest max'))
        (HydroLean.Hydro.Stream.insertWith _ m.slot
          (LogValue.mk m.ballot m.value) log) _ = _
      rw [insertWith_maxComb_eq]
      exact ih _
    · rw [if_neg hq, if_neg hq]
      exact ih _

/-- The transcription's acks are the compiled per-element replies. -/
theorem acceptor_p2_out_eq (max' : Option (Ballot nP))
    (p2as : Stream (P2a P nP)) (chk : Option Nat) (log : LogMap P nP) :
    (acceptor_p2 max' p2as chk log).2 = p2as.map (aP2bReply max') := by
  show (p2as.crossSingleton max').map _ = _
  unfold Stream.crossSingleton Stream.map
  rw [List.map_map]
  rfl

/-! ## The module's verified face: `acceptor_p2` over its own run

Proof inputs and outputs (module doc):
- inputs: the cumulative consumed P2as have duplicate-free
  `(slot, ballot)` keys (paxos.rs:862's missing `assume`) — the hypothesis
  of `ap2_okOnce`;
- outputs: coverage-monotonicity of the `a_log` wire is **carried by
  `acceptor_p2_ticksM`'s output type** (`MonoSing covVOc` — the manual
  witness the Unbounded marker does not give, earned once by
  `ap2_step_infl`); `ap2_ackLb` (write-before-ack, cumulative),
  `ap2_okCnt`/`ap2_okOnce`, `ap2_voteEcho`, `ap2_logEcho`; per-tick:
  `acceptor_p2_step_ok_max` (an `Ok` vote's ballot IS the tick's `a_max`
  wire value). -/

/-- The `Ok`-accept indicator for key `(slot, b)`. -/
def isOkP2b (slot : Nat) (b : Ballot nP) (m : P2b nP) : Bool :=
  decide (m.slot = slot) && decide (m.ballot = b) &&
    (match m.res with
     | .ok _ => true
     | .error _ => false)

/-- The `(slot, b)`-key indicator on P2as. -/
def isKeyP2a (slot : Nat) (b : Ballot nP) (m : P2a P nP) : Bool :=
  decide (m.slot = slot) && decide (m.ballot = b)

/-- One `acceptor_p2` tick's input: the P2a batch, the tick's `a_max` wire
value, and the checkpoint snapshot (dropped: `none` in this port). -/
structure AP2In (P : Type) (nP : Nat) where
  p2as : List (P2a P nP)
  a_max : Option (Ballot nP)
  a_checkpoint : Option Nat := none

open HydroLean.Hydro in
/-- `acceptor_p2` folded over its tick inputs: state = the accepted log
(published each tick on the atomic wire), output = the ack batch. -/
def acceptorP2Loop (P : Type) (nP : Nat) :
    TickLoop (AP2In P nP) (LogMap P nP) (List (Fin nP × P2b nP)) where
  init := []
  step s t :=
    ((acceptor_p2 t.a_max t.p2as t.a_checkpoint s).1.2,
     (acceptor_p2 t.a_max t.p2as t.a_checkpoint s).2)

open HydroLean.Hydro

/-- The accepted log after consuming `ins`. -/
def ap2Log (ins : List (AP2In P nP)) : LogMap P nP :=
  (acceptorP2Loop P nP).finalState ins

/-- Cumulative ack stream. -/
def ap2Acks (ins : List (AP2In P nP)) : List (Fin nP × P2b nP) :=
  ((acceptorP2Loop P nP).outputs ins).flatten

/-- Cumulative consumed P2as. -/
def ap2Cons (ins : List (AP2In P nP)) : List (P2a P nP) :=
  (ins.map (·.p2as)).flatten

@[simp] theorem ap2Log_append (ins : List (AP2In P nP)) (t : AP2In P nP) :
    ap2Log (ins ++ [t]) = aLogAfter t.a_max (ap2Log ins) t.p2as := by
  unfold ap2Log
  rw [TickLoop.finalState_append]
  exact acceptor_p2_log_eq t.a_max t.p2as t.a_checkpoint
    ((acceptorP2Loop P nP).finalState ins)

@[simp] theorem ap2Acks_append (ins : List (AP2In P nP)) (t : AP2In P nP) :
    ap2Acks (ins ++ [t])
      = ap2Acks ins ++ t.p2as.map (aP2bReply t.a_max) := by
  unfold ap2Acks
  rw [TickLoop.outputs_append, List.flatten_append, List.flatten_cons,
    List.flatten_nil, List.append_nil]
  congr 1
  exact acceptor_p2_out_eq t.a_max t.p2as t.a_checkpoint
    ((acceptorP2Loop P nP).finalState ins)

@[simp] theorem ap2Cons_append (ins : List (AP2In P nP)) (t : AP2In P nP) :
    ap2Cons (ins ++ [t]) = ap2Cons ins ++ t.p2as := by
  unfold ap2Cons
  simp

/-- Coverage of a slot at (or above) a ballot. -/
def LogCovers (log : LogMap P nP) (slot : Nat) (b : Ballot nP) : Prop :=
  ∃ e : LogValue P nP, log.find? slot = some e ∧
    (e.ballot = b ∨ b.blt e.ballot = true)

/-- The log **coverage order** — the value order of the `a_log` wire
(per-slot ballot-max lattice). paxos.rs types `a_log` `Unbounded`, so the
framework rightly gives nothing for free; the `monotone =` closure
obligation (`aLogAfter_ballot_lb`) is what earns the `Monotonic` face. -/
def covVO {P : Type} {nP : Nat} : ValueOrder (LogMap P nP) where
  le log log' := ∀ slot b, LogCovers log slot b → LogCovers log' slot b
  le_refl _ _ _ h := h
  le_trans h₁ h₂ slot b h := h₂ slot b (h₁ slot b h)

/-- The coverage order on the *published* `a_log` wire
(checkpoint × post-merge log): coverage of the log component. -/
def covVOc {P : Type} {nP : Nat} :
    ValueOrder (Option Nat × LogMap P nP) where
  le a b := covVO.le a.2 b.2
  le_refl a := covVO.le_refl a.2
  le_trans h₁ h₂ := covVO.le_trans h₁ h₂

/-- The tick step only extends coverage — the closure obligation of the
`reduce_watermark` keyed fold. -/
theorem ap2_step_infl (s : LogMap P nP) (t : AP2In P nP) :
    covVO.le s ((acceptorP2Loop P nP).step s t).1 := by
  intro slot b h
  show LogCovers (acceptor_p2 t.a_max t.p2as t.a_checkpoint s).1.2 slot b
  rw [acceptor_p2_log_eq]
  exact aLogAfter_ballot_lb t.a_max t.p2as s h

/-- **Write-before-ack, cumulative**: every `Ok` ack ever issued is covered
by the current log (paxos.rs:227–236 `snapshot_atomic`: "we will always
write payloads to the log before acknowledging them"). -/
theorem ap2_ackLb (ins : List (AP2In P nP)) :
    ∀ dm ∈ ap2Acks ins, (dm : Fin nP × P2b nP).2.res = .ok () →
      LogCovers (ap2Log ins) dm.2.slot dm.2.ballot := by
  induction ins using HydroLean.list_snoc_induction with
  | h0 => intro dm hdm; cases hdm
  | h1 ins t ih =>
    intro dm hdm hok
    rw [ap2Acks_append] at hdm
    rw [ap2Log_append]
    rcases List.mem_append.mp hdm with hold | hnew
    · exact aLogAfter_ballot_lb _ _ _ (ih dm hold hok)
    · obtain ⟨m, hm, rfl⟩ := List.mem_map.mp hnew
      have hbm : some m.ballot = t.a_max :=
        (aP2bReply_ok_iff t.a_max m).mp hok
      exact accepted_means_logged t.a_max (ap2Log ins) t.p2as m hm hbm

/-- **`Ok`-vote counting**: votes at a key never outnumber the consumed
copies of the key. -/
theorem ap2_okCnt (ins : List (AP2In P nP)) (slot : Nat) (b : Ballot nP) :
    (ap2Acks ins).countP (fun dm => isOkP2b slot b dm.2)
      ≤ (ap2Cons ins).countP (isKeyP2a slot b) := by
  induction ins using HydroLean.list_snoc_induction with
  | h0 => exact Nat.le_refl 0
  | h1 ins t ih =>
    rw [ap2Acks_append, ap2Cons_append, List.countP_append,
      List.countP_append]
    refine Nat.add_le_add ih ?_
    rw [List.countP_map]
    refine List.countP_mono_left ?_
    intro m _ hok
    have hok' : isOkP2b slot b (aP2bReply t.a_max m).2 = true := hok
    unfold isOkP2b at hok'
    rw [Bool.and_eq_true, Bool.and_eq_true, decide_eq_true_iff,
      decide_eq_true_iff] at hok'
    unfold isKeyP2a
    rw [Bool.and_eq_true, decide_eq_true_iff, decide_eq_true_iff]
    exact ⟨hok'.1.1, hok'.1.2⟩

/-- **Votes echo consumption**: every `Ok` ack quotes a consumed P2a and is
routed to its sender. -/
theorem ap2_voteEcho (ins : List (AP2In P nP)) :
    ∀ dm ∈ ap2Acks ins, (dm : Fin nP × P2b nP).2.res = .ok () →
      ∃ m ∈ ap2Cons ins, dm.1 = (m : P2a P nP).sender ∧
        dm.2.slot = m.slot ∧ dm.2.ballot = m.ballot := by
  induction ins using HydroLean.list_snoc_induction with
  | h0 => intro dm hdm; cases hdm
  | h1 ins t ih =>
    intro dm hdm hok
    rw [ap2Acks_append] at hdm
    rcases List.mem_append.mp hdm with hold | hnew
    · obtain ⟨m, hm, hprops⟩ := ih dm hold hok
      exact ⟨m, by rw [ap2Cons_append]; exact List.mem_append_left _ hm,
        hprops⟩
    · obtain ⟨m, hm, rfl⟩ := List.mem_map.mp hnew
      exact ⟨m, by rw [ap2Cons_append]; exact List.mem_append_right _ hm,
        rfl, rfl, rfl⟩

/-- **Log entries echo consumption**. -/
theorem ap2_logEcho (ins : List (AP2In P nP)) :
    ∀ x ∈ ap2Log ins, ∃ m ∈ ap2Cons ins,
      x = ((m : P2a P nP).slot, ⟨m.ballot, m.value⟩) := by
  induction ins using HydroLean.list_snoc_induction with
  | h0 => intro x hx; cases hx
  | h1 ins t ih =>
    intro x hx
    rw [ap2Log_append] at hx
    rcases aLogAfter_mem_cases hx with hold | ⟨m, hm, hx'⟩
    · obtain ⟨m, hm, hx'⟩ := ih x hold
      exact ⟨m, by rw [ap2Cons_append]; exact List.mem_append_left _ hm, hx'⟩
    · exact ⟨m, by rw [ap2Cons_append]; exact List.mem_append_right _ hm, hx'⟩

/-- **At most one `Ok` vote per `(slot, ballot)`**, given the module's input
requirement: the consumed P2a stream has duplicate-free keys — the missing
`assume` of paxos.rs:862's `manual_proof!`. -/
theorem ap2_okOnce (ins : List (AP2In P nP))
    (hnd : ((ap2Cons ins).map (fun m => (m.slot, m.ballot))).Nodup)
    (slot : Nat) (b : Ballot nP) :
    (ap2Acks ins).countP (fun dm => isOkP2b slot b dm.2) ≤ 1 := by
  refine Nat.le_trans (ap2_okCnt ins slot b) ?_
  refine HydroLean.Hydro.countP_key_le_one hnd (slot, b) ?_
  intro m hm
  unfold isKeyP2a at hm
  rw [Bool.and_eq_true, decide_eq_true_iff, decide_eq_true_iff] at hm
  rw [hm.1, hm.2]

/-- **Per-tick `Ok` inversion**: an `Ok` vote's ballot IS the tick's `a_max`
wire value (paxos.rs:881; combined with `acceptor_p1.ap1_max_mono` this is
what makes late low-ballot votes impossible). -/
theorem acceptor_p2_step_ok_max {s : LogMap P nP} {t : AP2In P nP}
    {dm : Fin nP × P2b nP}
    (hdm : dm ∈ ((acceptorP2Loop P nP).step s t).2)
    (hok : dm.2.res = .ok ()) : some dm.2.ballot = t.a_max := by
  have hdm' : dm ∈ t.p2as.map (aP2bReply t.a_max) := by
    have := acceptor_p2_out_eq t.a_max t.p2as t.a_checkpoint s
    rw [show ((acceptorP2Loop P nP).step s t).2
      = (acceptor_p2 t.a_max t.p2as t.a_checkpoint s).2 from rfl, this] at hdm
    exact hdm
  obtain ⟨m, hm, rfl⟩ := List.mem_map.mp hdm'
  exact (aP2bReply_ok_iff t.a_max m).mp hok

/-- **Per-proposer decoded vote cap**: with duplicate-free consumed
`(slot, ballot)` keys (paxos.rs:862's missing `assume`), the demuxed
`(key, result)` slice carries at most one `Ok` per key — the
`collect_quorum` input requirement, discharged at the module that owns the
acks. -/
theorem ap2_decode_cap (ins : List (AP2In P nP))
    (hnd : ((ap2Cons ins).map (fun m => (m.slot, m.ballot))).Nodup)
    (i : Fin nP) (slot : Nat) (b : Ballot nP) :
    ((ap2Acks ins).filterMap (fun dm => if dm.1 = i then
        some ((dm.2.slot, dm.2.ballot), dm.2.res) else none)).countP
      (fun e => decide (e.1 = (slot, b)) && e.2.isOk) ≤ 1 := by
  refine Nat.le_trans (countP_filterMap_le ?_) (ap2_okOnce ins hnd slot b)
  intro dm _ e hg hp
  by_cases hi : dm.1 = i
  · rw [if_pos hi] at hg
    cases hg
    rw [Bool.and_eq_true, decide_eq_true_iff] at hp
    unfold isOkP2b
    rw [Bool.and_eq_true, Bool.and_eq_true, decide_eq_true_iff,
      decide_eq_true_iff]
    have h1 : dm.2.slot = slot := congrArg Prod.fst hp.1
    have h2 : dm.2.ballot = b := congrArg Prod.snd hp.1
    refine ⟨⟨h1, h2⟩, ?_⟩
    cases hres : dm.2.res with
    | ok _ => rfl
    | error _ =>
      rw [hres] at hp
      cases hp.2
  · rw [if_neg hi] at hg
    cases hg

/-! ## `acceptor_p2` across ticks (the module at the located surface) -/

/-- `acceptor_p2` lifted across ticks: the P2a tick batches zipped with the
`a_max_ballot` wire (blocking), scanned by the transcription; the checkpoint
input is dropped (`none`) as authorized (garbage collection only). Returns
(the `a_log` atomic wire per tick — checkpoint × post-merge log — **at its
`Monotonic`-coverage type** (`MonoSing covVOc`, the keyed `reduce_watermark`
fold with its inflationary obligation `ap2_step_infl` paid once), and the
p2b ack batches per tick). -/
def acceptor_p2_ticksM :
    TStream (P2a P nP) × TSing (Option (Ballot nP))
      →ₘ MonoSing (covVOc (P := P) (nP := nP)) × TStream (Fin nP × P2b nP) :=
  let p2a := MonoMap.fst
  let a_max := MonoMap.snd
  -- the tick inputs: P2a batches zipped with the (blocking) a_max wire
  let ins := (p2a.zip a_max).map (fun bm => AP2In.mk bm.1 bm.2 none)
  -- a_log = reduce_watermark(…insertMax…)  [monotonic = ap2_step_infl]
  let a_log := (ins.loopFoldMonotonic (acceptorP2Loop P nP) covVO
      ap2_step_infl).mapWire
    (fun log => ((none : Option Nat), log)) (fun h => h)
  MonoMap.pair a_log (ins.loop (acceptorP2Loop P nP))


/-! ## Output elimination (module face) -/

/-- Tick-`t` p2b acks, characterized. -/
theorem acceptorP2_out_elim {ins : List (AP2In P nP)}
    {dm : Fin nP × P2b nP}
    (hdm : dm ∈ ((acceptorP2Loop P nP).outputs ins).flatten) :
    ∃ t, ∃ ht : t < (ins).length,
      ∃ m ∈ ((ins)[t]'ht).p2as,
      dm = aP2bReply (((ins)[t]'ht).a_max) m := by
  obtain ⟨t, ht, hstep⟩ := (acceptorP2Loop P nP).mem_outputs_elim hdm
  refine ⟨t, ht, ?_⟩
  have hout := acceptor_p2_out_eq ((ins)[t]'ht).a_max
    ((ins)[t]'ht).p2as
    ((ins)[t]'ht).a_checkpoint
    ((acceptorP2Loop P nP).finalState ((ins).take t))
  rw [show ((acceptorP2Loop P nP).step
      ((acceptorP2Loop P nP).finalState ((ins).take t))
      ((ins)[t]'ht)).2
    = (acceptor_p2 ((ins)[t]'ht).a_max
        ((ins)[t]'ht).p2as
        ((ins)[t]'ht).a_checkpoint
        ((acceptorP2Loop P nP).finalState
          ((ins).take t))).2 from rfl, hout] at hstep
  obtain ⟨m, hm, heq⟩ := List.mem_map.mp hstep
  exact ⟨m, hm, heq.symm⟩


/-- Membership in a tick's ack batch gives membership in the cumulative
acks of any covering prefix. -/
theorem mem_ap2Acks_of_tick {ins : List (AP2In P nP)} {t n : Nat}
    (ht : t < ins.length) (htn : t < n)
    {dm : Fin nP × P2b nP}
    (hdm : dm ∈ ((acceptorP2Loop P nP).step
      ((acceptorP2Loop P nP).finalState (ins.take t)) (ins[t]'ht)).2) :
    dm ∈ ap2Acks (ins.take n) := by
  have hsub : ins.take (t + 1) <+: ins.take n := by
    have h1 := List.take_prefix (t + 1) (ins.take n)
    rwa [List.take_take, Nat.min_eq_left (by omega)] at h1
  have hmem : dm ∈ ap2Acks (ins.take (t + 1)) := by
    rw [take_succ_eq ins t ht, ap2Acks_append]
    refine List.mem_append_right _ ?_
    have := acceptor_p2_out_eq (ins[t]'ht).a_max (ins[t]'ht).p2as
      (ins[t]'ht).a_checkpoint
      ((acceptorP2Loop P nP).finalState (ins.take t))
    rw [show ((acceptorP2Loop P nP).step
        ((acceptorP2Loop P nP).finalState (ins.take t)) (ins[t]'ht)).2
      = (acceptor_p2 (ins[t]'ht).a_max (ins[t]'ht).p2as
          (ins[t]'ht).a_checkpoint
          ((acceptorP2Loop P nP).finalState (ins.take t))).2 from rfl,
      this] at hdm
    exact hdm
  unfold ap2Acks at hmem ⊢
  exact (prefix_flatten
    ((acceptorP2Loop P nP).outputs_prefix hsub)).subset hmem

/-! ## The module contract at the ticks signature

Stated on `acceptor_p2_ticksM`'s own I/O — inputs `(p2a, am)` (the consumed
P2a tick batches and the blocking `a_max_ballot` wire), outputs (the
coverage-`Monotonic` published log and the ack batches). Callers
(`sequence_payload`) consume these; the loop/zip plumbing never escapes
this file. -/

section TicksContract

variable {p2a : TStream (P2a P nP)} {am : TSing (Option (Ballot nP))}

/-- **`Ok`-ack contract (write-before-ack)**: an `Ok` ack pins a tick — the
`a_max_ballot` **input** carries exactly the acked ballot there, and the
published log **output** already covers the acked key at that tick. -/
theorem ap2t_ok_spec {dm : Fin nP × P2b nP}
    (hdm : dm ∈ ((acceptor_p2_ticksM.f (p2a, am)).2).flatten)
    (hok : dm.2.res = .ok ()) :
    ∃ (t : Nat) (hta : t < am.length)
      (htl : t < ((acceptor_p2_ticksM.f (p2a, am)).1).vals.length),
      am[t]'hta = some dm.2.ballot ∧
      LogCovers ((((acceptor_p2_ticksM.f (p2a, am)).1).vals[t]'htl).2)
        dm.2.slot dm.2.ballot := by
  have hdm' : dm ∈ ((acceptorP2Loop P nP).outputs
      ((TSing.zip p2a am).map fun bm => AP2In.mk bm.1 bm.2 none)).flatten :=
    hdm
  obtain ⟨t, ht, m, hm, hdmr⟩ := acceptorP2_out_elim hdm'
  have hzip : t < (List.zip p2a am).length := by
    have := ht
    rwa [List.length_map] at this
  have hta : t < am.length := by
    have := hzip
    rw [List.length_zip] at this
    omega
  have hamax : (((TSing.zip p2a am).map
      fun bm => AP2In.mk bm.1 bm.2 none)[t]'ht).a_max = am[t]'hta := by
    rw [List.getElem_map]
    show ((List.zip p2a am)[t]'hzip).2 = _
    rw [List.getElem_zip]
  have hvm : some m.ballot = (((TSing.zip p2a am).map
      fun bm => AP2In.mk bm.1 bm.2 none)[t]'ht).a_max := by
    refine (aP2bReply_ok_iff _ m).mp ?_
    rw [← hdmr]
    exact hok
  have hsl : t < ((acceptorP2Loop P nP).states
      ((TSing.zip p2a am).map fun bm => AP2In.mk bm.1 bm.2 none)).length := by
    rw [TickLoop.states_length]
    exact ht
  have htl : t < ((acceptor_p2_ticksM.f (p2a, am)).1).vals.length := by
    show t < (((acceptorP2Loop P nP).states
      ((TSing.zip p2a am).map fun bm => AP2In.mk bm.1 bm.2 none)).map
        (fun log => ((none : Option Nat), log))).length
    rw [List.length_map]
    exact hsl
  have hdmb : dm.2.ballot = m.ballot := by
    rw [hdmr]
    rfl
  have hdms : dm.2.slot = m.slot := by
    rw [hdmr]
    rfl
  refine ⟨t, hta, htl, ?_, ?_⟩
  · rw [← hamax, ← hvm, hdmb]
  · -- write-before-ack at the ack's own tick
    have hstep : aP2bReply ((((TSing.zip p2a am).map
        fun bm => AP2In.mk bm.1 bm.2 none)[t]'ht).a_max) m
        ∈ ((acceptorP2Loop P nP).step
          ((acceptorP2Loop P nP).finalState
            (((TSing.zip p2a am).map
              fun bm => AP2In.mk bm.1 bm.2 none).take t))
          (((TSing.zip p2a am).map
            fun bm => AP2In.mk bm.1 bm.2 none)[t]'ht)).2 := by
      have hout := acceptor_p2_out_eq
        ((((TSing.zip p2a am).map
          fun bm => AP2In.mk bm.1 bm.2 none)[t]'ht).a_max)
        ((((TSing.zip p2a am).map
          fun bm => AP2In.mk bm.1 bm.2 none)[t]'ht).p2as)
        ((((TSing.zip p2a am).map
          fun bm => AP2In.mk bm.1 bm.2 none)[t]'ht).a_checkpoint)
        ((acceptorP2Loop P nP).finalState
          (((TSing.zip p2a am).map
            fun bm => AP2In.mk bm.1 bm.2 none).take t))
      rw [show ((acceptorP2Loop P nP).step
          ((acceptorP2Loop P nP).finalState
            (((TSing.zip p2a am).map
              fun bm => AP2In.mk bm.1 bm.2 none).take t))
          (((TSing.zip p2a am).map
            fun bm => AP2In.mk bm.1 bm.2 none)[t]'ht)).2
        = (acceptor_p2 _ _ _ _).2 from rfl, hout]
      exact List.mem_map.mpr ⟨m, hm, rfl⟩
    have hacks : aP2bReply ((((TSing.zip p2a am).map
        fun bm => AP2In.mk bm.1 bm.2 none)[t]'ht).a_max) m
        ∈ ap2Acks ((((TSing.zip p2a am).map
          fun bm => AP2In.mk bm.1 bm.2 none)).take (t + 1)) :=
      mem_ap2Acks_of_tick ht (Nat.lt_succ_self t) hstep
    have hcov := ap2_ackLb ((((TSing.zip p2a am).map
        fun bm => AP2In.mk bm.1 bm.2 none)).take (t + 1)) _ hacks
      (by rw [← hdmr]; exact hok)
    have hval : ((acceptor_p2_ticksM.f (p2a, am)).1).vals[t]'htl
        = ((none : Option Nat), ap2Log ((((TSing.zip p2a am).map
            fun bm => AP2In.mk bm.1 bm.2 none)).take (t + 1))) := by
      show (((acceptorP2Loop P nP).states
        ((TSing.zip p2a am).map fun bm => AP2In.mk bm.1 bm.2 none)).map
          (fun log => ((none : Option Nat), log)))[t]'(by
            rw [List.length_map]; exact hsl) = _
      rw [List.getElem_map,
        (acceptorP2Loop P nP).states_getElem _ t hsl]
      rfl
    rw [hval, hdms, hdmb]
    exact hcov

/-- **Log-entry echo contract**: every entry of the published log quotes a
consumed P2a — its slot, ballot, and value occur verbatim on the P2a
**input**. -/
theorem ap2t_log_entry {t : Nat}
    (htl : t < ((acceptor_p2_ticksM.f (p2a, am)).1).vals.length)
    {slot : Nat} {e : LogValue P nP}
    (hE : (slot, e)
      ∈ ((((acceptor_p2_ticksM.f (p2a, am)).1).vals[t]'htl).2
          : LogMap P nP)) :
    ∃ m ∈ p2a.flatten, slot = (m : P2a P nP).slot
      ∧ e = ⟨m.ballot, m.value⟩ := by
  have hsl : t < ((acceptorP2Loop P nP).states
      ((TSing.zip p2a am).map fun bm => AP2In.mk bm.1 bm.2 none)).length := by
    have := htl
    show t < ((acceptorP2Loop P nP).states _).length
    rw [show ((acceptor_p2_ticksM.f (p2a, am)).1).vals
      = ((acceptorP2Loop P nP).states
        ((TSing.zip p2a am).map fun bm => AP2In.mk bm.1 bm.2 none)).map
        (fun log => ((none : Option Nat), log)) from rfl,
      List.length_map] at this
    exact this
  have hval : ((acceptor_p2_ticksM.f (p2a, am)).1).vals[t]'htl
      = ((none : Option Nat), ap2Log ((((TSing.zip p2a am).map
          fun bm => AP2In.mk bm.1 bm.2 none)).take (t + 1))) := by
    show (((acceptorP2Loop P nP).states
      ((TSing.zip p2a am).map fun bm => AP2In.mk bm.1 bm.2 none)).map
        (fun log => ((none : Option Nat), log)))[t]'(by
          rw [List.length_map]; exact hsl) = _
    rw [List.getElem_map,
      (acceptorP2Loop P nP).states_getElem _ t hsl]
    rfl
  rw [hval] at hE
  obtain ⟨m, hmc, hme⟩ := ap2_logEcho _ _ hE
  refine ⟨m, ?_, congrArg Prod.fst hme, congrArg Prod.snd hme⟩
  have hsub1 : ap2Cons ((((TSing.zip p2a am).map
      fun bm => AP2In.mk bm.1 bm.2 none)).take (t + 1))
      <+: ap2Cons (((TSing.zip p2a am).map
        fun bm => AP2In.mk bm.1 bm.2 none)) :=
    prefix_flatten ((List.take_prefix _ _).map _)
  have hsub2 : ap2Cons (((TSing.zip p2a am).map
      fun bm => AP2In.mk bm.1 bm.2 none)) <+: p2a.flatten := by
    refine prefix_flatten ?_
    rw [show ((((TSing.zip p2a am).map
        fun bm => AP2In.mk bm.1 bm.2 none)).map (·.p2as))
      = (List.zip p2a am).map Prod.fst from by
      show List.map _ (List.map _ _) = _
      rw [List.map_map]
      rfl]
    exact zip_fst_prefix _ _
  exact (hsub1.trans hsub2).subset hmc

/-- **Decode-cap contract**: with duplicate-free `(slot, ballot)` keys on
the P2a **input** (the B2 once-per-ballot fan-in discipline), the demuxed
ack slice carries at most one `Ok` vote per key. -/
theorem ap2t_decode_cap
    (hnd : ((p2a.flatten).map
      (fun m => ((m : P2a P nP).slot, m.ballot))).Nodup)
    (i : Fin nP) (slot : Nat) (b : Ballot nP) :
    ((((acceptor_p2_ticksM.f (p2a, am)).2).flatten).filterMap
      (fun dm => if dm.1 = i then
        some ((dm.2.slot, dm.2.ballot), dm.2.res) else none)).countP
      (fun e => decide (e.1 = (slot, b)) && e.2.isOk) ≤ 1 := by
  refine ap2_decode_cap _ ?_ i slot b
  have hsub : ap2Cons (((TSing.zip p2a am).map
      fun bm => AP2In.mk bm.1 bm.2 none)) <+: p2a.flatten := by
    refine prefix_flatten ?_
    rw [show ((((TSing.zip p2a am).map
        fun bm => AP2In.mk bm.1 bm.2 none)).map (·.p2as))
      = (List.zip p2a am).map Prod.fst from by
      show List.map _ (List.map _ _) = _
      rw [List.map_map]
      rfl]
    exact zip_fst_prefix _ _
  exact hnd.sublist (hsub.sublist.map _)

end TicksContract

end HydroLean.Programs.Paxos
