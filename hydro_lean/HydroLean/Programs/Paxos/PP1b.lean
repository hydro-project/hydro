import HydroLean.Programs.Paxos.Types
import HydroLean.Programs.CollectQuorumWithResponse
import HydroLean.Programs.CollectQuorumStreams

/-!
# `p_p1b` (paxos.rs:528–593) — module

Proposer logic for processing P1bs. Composition of:
1. `collect_quorum_with_response(a_to_proposers_p1b, quorum_size,
   num_quorum_participants)` (paxos.rs:545–546) — REUSED
   `collectQuorumWRTick`, which runs on **its own slice clock** (the
   `sliced!` block inside `hydro_std::quorum`), scheduled independently by
   the system;
2. the keyed `fold_early_stop` over the emitted quorum stream
   (paxos.rs:548–559): accumulate up to `quorum_size` accepted logs per
   ballot — an *unbounded* keyed fold (no clock of its own), modeled as the
   pure function `foldEarlyStopBallots` of the quorum-output history;
3. `get_max_key()` (paxos.rs:560) — `p1bMaxQuorumBallot`;
4. `.snapshot(proposer_tick, nondet!(/** stale max_by_key → delayed
   leadership … does not result in any safety issues */))` (paxos.rs:561–572)
   — the proposer_tick reads (2)+(3) at an **adversarial monotone cut** into
   the quorum-output history; that doc-comment claim is a phase-2 proof
   obligation;
5. `.zip(p_ballot).filter_map(quorum_ballot == my_ballot)` + `p_is_leader`
   (paxos.rs:573–585) — `pP1bView`.

`fail_ballots` (paxos.rs:591) is the `collect_quorum_with_response` error
stream — a pure `filterMap` of the raw P1b stream (quorum.rs:82–85), feeding
the `p1b_fail` forward-ref cycle (paxos.rs:271–272).
-/

namespace HydroLean.Programs.Paxos

open HydroLean.Hydro
open HydroLean.Programs

variable {P : Type} {nP : Nat}

/-- The p1b quorum-response payload: Rust
`(Option<usize>, HashMap<usize, LogValue<P>>)` (checkpoint dropped ⇒ `none`). -/
abbrev P1bPayload (P : Type) (nP : Nat) := Option Nat × LogMap P nP

/-- One insertion of the keyed `fold_early_stop` (paxos.rs:553–559): push
into the ballot's vector until `quorum_size` logs are collected, then stop
(`logs.len() >= quorum_size`). -/
def p1bLogsInsert (quorumSize : Nat)
    (logs : List (Ballot nP × List (P1bPayload P nP)))
    (b : Ballot nP) (v : P1bPayload P nP) :
    List (Ballot nP × List (P1bPayload P nP)) :=
  match logs with
  | [] => [(b, [v])]
  | (b', vs) :: rest =>
    if b' = b then
      if vs.length < quorumSize then (b', vs ++ [v]) :: rest
      else (b', vs) :: rest
    else (b', vs) :: p1bLogsInsert quorumSize rest b v

/-- The `fold_early_stop` keyed state as a pure function of (a prefix of) the
quorum-output history: this is what the stale snapshot at paxos.rs:561
observes. -/
def foldEarlyStopBallots (quorumSize : Nat)
    (quorumOuts : List (Ballot nP × P1bPayload P nP)) :
    List (Ballot nP × List (P1bPayload P nP)) :=
  quorumOuts.foldl (fun acc (b, v) => p1bLogsInsert quorumSize acc b v) []

/-- `get_max_key` over ballots holding a full quorum of logs
(paxos.rs:560). -/
def p1bMaxQuorumBallot (quorumSize : Nat)
    (logs : List (Ballot nP × List (P1bPayload P nP))) :
    Option (Ballot nP × List (P1bPayload P nP)) :=
  logs.foldl
    (fun acc e =>
      if quorumSize ≤ e.2.length then
        match acc with
        | none => some e
        | some a => if a.1.blt e.1 then some e else some a
      else acc)
    none

/-- `p_received_quorum_of_p1bs` + `p_is_leader` as seen by one proposer_tick
fire (paxos.rs:561–585): given the stale cut prefix of the quorum-output
history, our current ballot, and `p_has_largest_ballot`, produce the
accepted-logs view and the leader flag. -/
def pP1bView (quorumSize : Nat)
    (quorumOutsPrefix : List (Ballot nP × P1bPayload P nP))
    (myBallot : Ballot nP) (hasLargest : Bool) :
    Option (List (P1bPayload P nP)) × Bool :=
  let folded := foldEarlyStopBallots quorumSize quorumOutsPrefix
  let relevant :=
    match p1bMaxQuorumBallot quorumSize folded with
    | some (qb, qlogs) => if qb = myBallot then some qlogs else none
    | none => none
  (relevant, relevant.isSome && hasLargest)

/-! ## Module spec -/

/-- `fold_early_stop` never collects more than `quorum_size` logs per ballot
(the early stop; paxos.rs:557), for `1 ≤ quorum_size`. With
sender-deduplicated inputs (the guarded variant's contract) this makes a full
vector a genuine quorum. -/
theorem p1bLogsInsert_length_le (quorumSize : Nat) (hq1 : 1 ≤ quorumSize)
    (logs : List (Ballot nP × List (P1bPayload P nP)))
    (b : Ballot nP) (v : P1bPayload P nP)
    (h : ∀ e ∈ logs, e.2.length ≤ quorumSize) :
    ∀ e ∈ p1bLogsInsert quorumSize logs b v, e.2.length ≤ quorumSize := by
  induction logs with
  | nil =>
    intro e he
    rcases List.mem_singleton.mp he with rfl
    simpa using hq1
  | cons hd rest ih =>
    intro e he
    obtain ⟨b', vs⟩ := hd
    simp only [p1bLogsInsert] at he
    by_cases hb : b' = b
    · rw [if_pos hb] at he
      by_cases hlen : vs.length < quorumSize
      · rw [if_pos hlen] at he
        rcases List.mem_cons.mp he with rfl | hmem
        · simpa using Nat.succ_le_of_lt hlen
        · exact h e (List.mem_cons_of_mem _ hmem)
      · rw [if_neg hlen] at he
        exact h e he
    · rw [if_neg hb] at he
      rcases List.mem_cons.mp he with rfl | hmem
      · exact h _ List.mem_cons_self
      · exact ih (fun e' he' => h e' (List.mem_cons_of_mem _ he')) e hmem

/-- A max-quorum result really holds a full quorum's worth of logs. -/
theorem p1bMaxQuorumBallot_holds_quorum (quorumSize : Nat)
    (logs : List (Ballot nP × List (P1bPayload P nP)))
    {qb : Ballot nP} {qlogs : List (P1bPayload P nP)}
    (h : p1bMaxQuorumBallot quorumSize logs = some (qb, qlogs)) :
    quorumSize ≤ qlogs.length := by
  unfold p1bMaxQuorumBallot at h
  -- fold invariant: any `some` accumulator satisfies the bound
  have hgen : ∀ (l : List (Ballot nP × List (P1bPayload P nP)))
      (acc : Option (Ballot nP × List (P1bPayload P nP))),
      (∀ a, acc = some a → quorumSize ≤ a.2.length) →
      ∀ r, l.foldl
        (fun acc e =>
          if quorumSize ≤ e.2.length then
            match acc with
            | none => some e
            | some a => if a.1.blt e.1 then some e else some a
          else acc) acc = some r → quorumSize ≤ r.2.length := by
    intro l
    induction l with
    | nil => exact fun acc hacc r h' => hacc r h'
    | cons e rest ih =>
      intro acc hacc r h'
      simp only [List.foldl_cons] at h'
      refine ih _ ?_ r h'
      intro a ha
      by_cases hq : quorumSize ≤ e.2.length
      · rw [if_pos hq] at ha
        cases hacc' : acc with
        | none => rw [hacc'] at ha; cases ha; exact hq
        | some a' =>
          rw [hacc'] at ha
          have ha' : (if a'.1.blt e.1 = true then some e else some a')
              = some a := ha
          by_cases hlt : a'.1.blt e.1
          · rw [if_pos hlt] at ha'; cases ha'; exact hq
          · rw [if_neg hlt] at ha'; cases ha'; exact hacc _ hacc'
      · rw [if_neg hq] at ha
        exact hacc _ ha
  exact hgen logs none (fun a ha => by cases ha) (qb, qlogs) h

/-! ## The `fold_early_stop` bucket calculus

The keyed fold's assoc-list state, characterized as **stable full buckets**:
once a ballot has collected `quorum_size` logs, its bucket never changes
again — the "elected quorum view" of a `(proposer, ballot)` pair is a single
well-defined value across all later snapshot cuts. These are the module
lemmas behind the proposer's cross-tick recommit/`next_slot` invariants. -/

/-- Insertion never removes or reorders buckets: membership after an insert
is an old entry, the updated bucket (appended, if not full), or a fresh
singleton bucket. -/
theorem p1bLogsInsert_mem_cases {quorumSize : Nat}
    {logs : List (Ballot nP × List (P1bPayload P nP))}
    {b : Ballot nP} {v : P1bPayload P nP}
    {qb : Ballot nP} {vs' : List (P1bPayload P nP)}
    (h : (qb, vs') ∈ p1bLogsInsert quorumSize logs b v) :
    (qb, vs') ∈ logs ∨
      (qb = b ∧
        ((∃ vs, (b, vs) ∈ logs ∧ vs.length < quorumSize ∧ vs' = vs ++ [v]) ∨
          vs' = [v])) := by
  induction logs with
  | nil =>
    have := List.mem_singleton.mp h
    cases this
    exact Or.inr ⟨rfl, Or.inr rfl⟩
  | cons hd rest ih =>
    obtain ⟨b', vs⟩ := hd
    simp only [p1bLogsInsert] at h
    by_cases hb : b' = b
    · rw [if_pos hb] at h
      by_cases hlen : vs.length < quorumSize
      · rw [if_pos hlen] at h
        rcases List.mem_cons.mp h with heq | hmem
        · cases heq
          exact Or.inr ⟨hb, Or.inl ⟨vs, hb ▸ List.mem_cons_self .., hlen, rfl⟩⟩
        · exact Or.inl (List.mem_cons_of_mem _ hmem)
      · rw [if_neg hlen] at h
        exact Or.inl h
    · rw [if_neg hb] at h
      rcases List.mem_cons.mp h with heq | hmem
      · exact Or.inl (heq ▸ List.mem_cons_self ..)
      · rcases ih hmem with hold | ⟨hqb, hcase⟩
        · exact Or.inl (List.mem_cons_of_mem _ hold)
        · refine Or.inr ⟨hqb, ?_⟩
          rcases hcase with ⟨vs₀, hvs₀, hlen₀, heq₀⟩ | hnew
          · exact Or.inl ⟨vs₀, List.mem_cons_of_mem _ hvs₀, hlen₀, heq₀⟩
          · exact Or.inr hnew

/-- Insertion preserves duplicate-free bucket keys. -/
theorem p1bLogsInsert_keys_nodup {quorumSize : Nat}
    {logs : List (Ballot nP × List (P1bPayload P nP))}
    (hnd : (logs.map Prod.fst).Nodup) (b : Ballot nP) (v : P1bPayload P nP) :
    ((p1bLogsInsert quorumSize logs b v).map Prod.fst).Nodup := by
  induction logs with
  | nil => exact List.pairwise_singleton _ _
  | cons hd rest ih =>
    obtain ⟨b', vs⟩ := hd
    rw [List.map_cons, List.nodup_cons] at hnd
    simp only [p1bLogsInsert]
    by_cases hb : b' = b
    · rw [if_pos hb]
      by_cases hlen : vs.length < quorumSize
      · rw [if_pos hlen, List.map_cons, List.nodup_cons]
        exact ⟨hnd.1, hnd.2⟩
      · rw [if_neg hlen, List.map_cons, List.nodup_cons]
        exact ⟨hnd.1, hnd.2⟩
    · rw [if_neg hb, List.map_cons, List.nodup_cons]
      refine ⟨fun hc => ?_, ih hnd.2⟩
      obtain ⟨⟨qb, vs'⟩, hmem, hkey⟩ := List.mem_map.mp hc
      have hkey' : qb = b' := hkey
      rcases p1bLogsInsert_mem_cases hmem with hold | ⟨hqb, -⟩
      · exact hnd.1 (List.mem_map.mpr ⟨_, hold, hkey⟩)
      · exact hb (hkey' ▸ hqb)

/-- The fold, in `foldl`-over-pairs form. -/
theorem foldEarlyStopBallots_eq_foldl (quorumSize : Nat)
    (quorumOuts : List (Ballot nP × P1bPayload P nP)) :
    foldEarlyStopBallots quorumSize quorumOuts
      = quorumOuts.foldl
          (fun acc e => p1bLogsInsert quorumSize acc e.1 e.2) [] := rfl

/-- The fold's bucket keys are duplicate-free. -/
theorem foldEarlyStop_keys_nodup (quorumSize : Nat)
    (quorumOuts : List (Ballot nP × P1bPayload P nP)) :
    ((foldEarlyStopBallots quorumSize quorumOuts).map Prod.fst).Nodup := by
  rw [foldEarlyStopBallots_eq_foldl]
  have hgen : ∀ (l : List (Ballot nP × P1bPayload P nP))
      (acc : List (Ballot nP × List (P1bPayload P nP))),
      (acc.map Prod.fst).Nodup →
      ((l.foldl (fun acc e => p1bLogsInsert quorumSize acc e.1 e.2)
        acc).map Prod.fst).Nodup := by
    intro l
    induction l with
    | nil => exact fun acc h => h
    | cons x rest ih =>
      intro acc hacc
      rw [List.foldl_cons]
      exact ih _ (p1bLogsInsert_keys_nodup hacc x.1 x.2)
  exact hgen _ [] List.Pairwise.nil

/-- The fold appends: extending the source continues the fold. -/
theorem foldEarlyStop_append (quorumSize : Nat)
    (pfx ext : List (Ballot nP × P1bPayload P nP)) :
    foldEarlyStopBallots quorumSize (pfx ++ ext)
      = ext.foldl (fun acc e => p1bLogsInsert quorumSize acc e.1 e.2)
          (foldEarlyStopBallots quorumSize pfx) := by
  rw [foldEarlyStopBallots_eq_foldl, foldEarlyStopBallots_eq_foldl,
    List.foldl_append]

/-- A full bucket survives one insertion untouched. -/
theorem p1bLogsInsert_full_stable {quorumSize : Nat}
    {logs : List (Ballot nP × List (P1bPayload P nP))}
    {qb : Ballot nP} {vs : List (P1bPayload P nP)}
    (hmem : (qb, vs) ∈ logs) (hfull : vs.length = quorumSize)
    (b : Ballot nP) (v : P1bPayload P nP) :
    (qb, vs) ∈ p1bLogsInsert quorumSize logs b v := by
  induction logs with
  | nil => cases hmem
  | cons hd rest ih =>
    obtain ⟨b', vs'⟩ := hd
    simp only [p1bLogsInsert]
    by_cases hb : b' = b
    · rw [if_pos hb]
      by_cases hlen : vs'.length < quorumSize
      · rw [if_pos hlen]
        rcases List.mem_cons.mp hmem with heq | hmemr
        · -- the head bucket would be full, contradicting `hlen`
          cases heq
          omega
        · exact List.mem_cons_of_mem _ hmemr
      · rw [if_neg hlen]
        exact hmem
    · rw [if_neg hb]
      rcases List.mem_cons.mp hmem with heq | hmemr
      · exact heq ▸ List.mem_cons_self ..
      · exact List.mem_cons_of_mem _ (ih hmemr)

/-- **Full buckets are stable**: once a ballot's bucket is full, it is the
same at every later cut of the source stream — the elected quorum view is a
single value per `(proposer, ballot)`. -/
theorem foldEarlyStop_full_stable {quorumSize : Nat}
    {pfx : List (Ballot nP × P1bPayload P nP)}
    {qb : Ballot nP} {vs : List (P1bPayload P nP)}
    (hmem : (qb, vs) ∈ foldEarlyStopBallots quorumSize pfx)
    (hfull : vs.length = quorumSize)
    (ext : List (Ballot nP × P1bPayload P nP)) :
    (qb, vs) ∈ foldEarlyStopBallots quorumSize (pfx ++ ext) := by
  rw [foldEarlyStop_append]
  have hgen : ∀ (l : List (Ballot nP × P1bPayload P nP))
      (acc : List (Ballot nP × List (P1bPayload P nP))),
      (qb, vs) ∈ acc →
      (qb, vs) ∈ l.foldl
        (fun acc e => p1bLogsInsert quorumSize acc e.1 e.2) acc := by
    intro l
    induction l with
    | nil => exact fun acc h => h
    | cons x rest ih =>
      intro acc hacc
      rw [List.foldl_cons]
      exact ih _ (p1bLogsInsert_full_stable hacc hfull x.1 x.2)
  exact hgen ext _ hmem

/-- All buckets hold at most `quorum_size` logs (`1 ≤ quorum_size`). -/
theorem foldEarlyStop_length_le {quorumSize : Nat} (hq1 : 1 ≤ quorumSize)
    (pfx : List (Ballot nP × P1bPayload P nP)) :
    ∀ e ∈ foldEarlyStopBallots quorumSize pfx, e.2.length ≤ quorumSize := by
  rw [foldEarlyStopBallots_eq_foldl]
  have hgen : ∀ (l : List (Ballot nP × P1bPayload P nP))
      (acc : List (Ballot nP × List (P1bPayload P nP))),
      (∀ e ∈ acc, e.2.length ≤ quorumSize) →
      ∀ e ∈ l.foldl (fun acc e => p1bLogsInsert quorumSize acc e.1 e.2) acc,
        e.2.length ≤ quorumSize := by
    intro l
    induction l with
    | nil => exact fun acc h => h
    | cons x rest ih =>
      intro acc hacc
      rw [List.foldl_cons]
      exact ih _ (p1bLogsInsert_length_le quorumSize hq1 acc x.1 x.2 hacc)
  exact hgen _ [] (fun e he => nomatch he)

/-- **Bucket counting**: a bucket's per-value multiplicities are covered by
the source stream's `(ballot, value)` multiplicities — collected quorum
entries are genuine emissions, *with multiplicity*. -/
theorem foldEarlyStop_count [DecidableEq P] {quorumSize : Nat}
    {pfx : List (Ballot nP × P1bPayload P nP)}
    {qb : Ballot nP} {vs : List (P1bPayload P nP)}
    (hmem : (qb, vs) ∈ foldEarlyStopBallots quorumSize pfx)
    (v : P1bPayload P nP) :
    vs.countP (fun w => decide (w = v))
      ≤ pfx.countP (fun e => decide (e.1 = qb) && decide (e.2 = v)) := by
  rw [foldEarlyStopBallots_eq_foldl] at hmem
  -- fold invariant with a per-key bound function
  have hgen : ∀ (l : List (Ballot nP × P1bPayload P nP))
      (acc : List (Ballot nP × List (P1bPayload P nP)))
      (C : Ballot nP → Nat),
      (∀ e ∈ acc, e.2.countP (fun w => decide (w = v)) ≤ C e.1) →
      ∀ e ∈ l.foldl (fun acc e => p1bLogsInsert quorumSize acc e.1 e.2) acc,
        e.2.countP (fun w => decide (w = v))
          ≤ C e.1 + l.countP (fun x => decide (x.1 = e.1) && decide (x.2 = v)) := by
    intro l
    induction l with
    | nil =>
      intro acc C hacc e he
      have := hacc e he
      rw [List.countP_nil]
      omega
    | cons x rest ih =>
      intro acc C hacc e he
      rw [List.foldl_cons] at he
      -- absorb `x` into the per-key bound
      have hstep := ih (p1bLogsInsert quorumSize acc x.1 x.2)
        (fun k => C k + (if x.1 = k ∧ x.2 = v then 1 else 0))
        (by
          intro e' he'
          rcases p1bLogsInsert_mem_cases he' with hold | ⟨hqb', hcase⟩
          · have := hacc e' hold
            have hnn : (0 : Nat) ≤ (if x.1 = e'.1 ∧ x.2 = v then 1 else 0) :=
              Nat.zero_le _
            omega
          · rcases hcase with ⟨vs₀, hvs₀, -, heq₀⟩ | hnew
            · have hbase : vs₀.countP (fun w => decide (w = v)) ≤ C x.1 :=
                hacc (x.1, vs₀) (hqb' ▸ hvs₀)
              have hkeq : e'.1 = x.1 := hqb'
              rw [show e'.2 = vs₀ ++ [x.2] from heq₀, List.countP_append,
                List.countP_cons, List.countP_nil, hkeq]
              by_cases hv : x.2 = v
              · rw [if_pos (by simpa using hv),
                  if_pos (show x.1 = x.1 ∧ x.2 = v from ⟨rfl, hv⟩)]
                omega
              · rw [if_neg (by simpa using hv), if_neg (by
                  rintro ⟨-, hc⟩
                  exact hv hc)]
                omega
            · have hkeq : e'.1 = x.1 := hqb'
              rw [show e'.2 = [x.2] from hnew, List.countP_cons,
                List.countP_nil, hkeq]
              by_cases hv : x.2 = v
              · rw [if_pos (by simpa using hv),
                  if_pos (show x.1 = x.1 ∧ x.2 = v from ⟨rfl, hv⟩)]
                omega
              · rw [if_neg (by simpa using hv), if_neg (by
                  rintro ⟨-, hc⟩
                  exact hv hc)]
                omega)
        e he
      rw [List.countP_cons]
      by_cases hx : (decide (x.1 = e.1) && decide (x.2 = v)) = true
      · rw [if_pos hx]
        rw [Bool.and_eq_true, decide_eq_true_iff, decide_eq_true_iff] at hx
        have hone : (if x.1 = e.1 ∧ x.2 = v then 1 else 0) = 1 := if_pos hx
        omega
      · rw [if_neg hx]
        have hzero : (if x.1 = e.1 ∧ x.2 = v then 1 else 0) = 0 := by
          refine if_neg ?_
          rintro ⟨h1, h2⟩
          exact hx (by
            rw [Bool.and_eq_true, decide_eq_true_iff, decide_eq_true_iff]
            exact ⟨h1, h2⟩)
        omega
  have := hgen pfx [] (fun _ => 0) (fun e he => nomatch he) (qb, vs) hmem
  simpa using this

/-- Two buckets of one ballot are the same bucket (bucket keys are
duplicate-free). -/
theorem foldEarlyStop_bucket_unique {quorumSize : Nat}
    {pfx : List (Ballot nP × P1bPayload P nP)} {qb : Ballot nP}
    {vs vs' : List (P1bPayload P nP)}
    (h : (qb, vs) ∈ foldEarlyStopBallots quorumSize pfx)
    (h' : (qb, vs') ∈ foldEarlyStopBallots quorumSize pfx) : vs = vs' :=
  congrArg Prod.snd (eq_of_nodup_keys (g := Prod.fst)
    (foldEarlyStop_keys_nodup quorumSize pfx) h h' rfl)

/-- **Frozen-bucket pinning**: a full bucket equals the same ballot's bucket
at any later cut — the "elected quorum view" is one value per ballot across
the whole run. -/
theorem foldEarlyStop_full_pin {quorumSize : Nat} (hq1 : 1 ≤ quorumSize)
    {pfx ext : List (Ballot nP × P1bPayload P nP)} {qb : Ballot nP}
    {vs vs' : List (P1bPayload P nP)}
    (hmem : (qb, vs) ∈ foldEarlyStopBallots quorumSize pfx)
    (hfull : quorumSize ≤ vs.length)
    (hmem' : (qb, vs') ∈ foldEarlyStopBallots quorumSize (pfx ++ ext)) :
    vs = vs' := by
  have hlen : vs.length = quorumSize :=
    Nat.le_antisymm (foldEarlyStop_length_le hq1 pfx _ hmem) hfull
  exact foldEarlyStop_bucket_unique (foldEarlyStop_full_stable hmem hlen ext)
    hmem'

/-- The leader flag forces a realized quorum view (`pP1bView` inversion). -/
theorem pP1bView_leader_isSome {quorumSize : Nat}
    {pfx : List (Ballot nP × P1bPayload P nP)} {myBallot : Ballot nP}
    {hasLargest : Bool}
    (h : (pP1bView quorumSize pfx myBallot hasLargest).2 = true) :
    (pP1bView quorumSize pfx myBallot hasLargest).1.isSome = true := by
  unfold pP1bView at h ⊢
  dsimp only at h ⊢
  rw [Bool.and_eq_true] at h
  exact h.1

/-- `get_max_key`'s result is an entry of its input (fold spec; module-local
restatement of the invariant behind `p1bMaxQuorumBallot`). -/
theorem p1bMaxQuorumBallot_mem_self {quorumSize : Nat}
    {logs : List (Ballot nP × List (P1bPayload P nP))}
    {r : Ballot nP × List (P1bPayload P nP)}
    (h : p1bMaxQuorumBallot quorumSize logs = some r) : r ∈ logs := by
  unfold p1bMaxQuorumBallot at h
  suffices hgen : ∀ (l : List (Ballot nP × List (P1bPayload P nP)))
      (acc : Option (Ballot nP × List (P1bPayload P nP))),
      (∀ a, acc = some a → a ∈ logs) → (∀ x ∈ l, x ∈ logs) →
      ∀ r, l.foldl (fun acc e =>
        if quorumSize ≤ e.2.length then
          match acc with
          | none => some e
          | some a => if a.1.blt e.1 then some e else some a
        else acc) acc = some r → r ∈ logs by
    exact hgen logs none (fun a ha => nomatch ha) (fun x hx => hx) r h
  intro l
  induction l with
  | nil =>
    intro acc hacc _ r hr
    rw [List.foldl_nil] at hr
    exact hacc r hr
  | cons x rest ih =>
    intro acc hacc hmem r hr
    rw [List.foldl_cons] at hr
    refine ih _ ?_ (fun y hy => hmem y (List.mem_cons_of_mem _ hy)) r hr
    intro a ha
    by_cases hq : quorumSize ≤ x.2.length
    · rw [if_pos hq] at ha
      cases hacc' : acc with
      | none =>
        rw [hacc'] at ha
        dsimp only at ha
        cases ha
        exact hmem x List.mem_cons_self
      | some a' =>
        rw [hacc'] at ha
        dsimp only at ha
        by_cases hlt : a'.1.blt x.1
        · rw [if_pos hlt] at ha
          cases ha
          exact hmem x List.mem_cons_self
        · rw [if_neg hlt] at ha
          cases ha
          exact hacc _ hacc'
    · rw [if_neg hq] at ha
      exact hacc _ ha

/-- `get_max_key` dominates every full bucket: if some ballot holds a full
quorum, the result exists and is at least that ballot. -/
theorem p1bMaxQuorumBallot_ge {quorumSize : Nat}
    {logs : List (Ballot nP × List (P1bPayload P nP))}
    {b₀ : Ballot nP} {vs : List (P1bPayload P nP)}
    (hmem : (b₀, vs) ∈ logs) (hfull : quorumSize ≤ vs.length) :
    ∃ r, p1bMaxQuorumBallot quorumSize logs = some r ∧ b₀.ble r.1 = true := by
  unfold p1bMaxQuorumBallot
  suffices hgen : ∀ (l : List (Ballot nP × List (P1bPayload P nP)))
      (acc : Option (Ballot nP × List (P1bPayload P nP))),
      ((∃ a, acc = some a ∧ b₀.ble a.1 = true) ∨
        ((b₀, vs) ∈ l ∧ quorumSize ≤ vs.length)) →
      ∃ r, l.foldl (fun acc e =>
        if quorumSize ≤ e.2.length then
          match acc with
          | none => some e
          | some a => if a.1.blt e.1 then some e else some a
        else acc) acc = some r ∧ b₀.ble r.1 = true by
    exact hgen logs none (Or.inr ⟨hmem, hfull⟩)
  intro l
  induction l with
  | nil =>
    intro acc hcase
    rcases hcase with ⟨a, rfl, ha⟩ | ⟨hmem', -⟩
    · exact ⟨a, rfl, ha⟩
    · cases hmem'
  | cons e rest ih =>
    intro acc hcase
    rw [List.foldl_cons]
    -- the step keeps or improves domination
    have hkeep : ∀ a, acc = some a → b₀.ble a.1 = true →
        ∃ a', (if quorumSize ≤ e.2.length then
            match acc with
            | none => some e
            | some a => if a.1.blt e.1 then some e else some a
          else acc) = some a' ∧ b₀.ble a'.1 = true := by
      intro a hacc ha
      subst hacc
      by_cases hq : quorumSize ≤ e.2.length
      · rw [if_pos hq]
        show ∃ a', (if a.1.blt e.1 then some e else some a) = some a' ∧
          b₀.ble a'.1 = true
        by_cases hlt : a.1.blt e.1
        · rw [if_pos hlt]
          exact ⟨e, rfl, Ballot.ble_trans ha (Ballot.ble_of_blt hlt)⟩
        · rw [if_neg hlt]
          exact ⟨a, rfl, ha⟩
      · rw [if_neg hq]
        exact ⟨a, rfl, ha⟩
    rcases hcase with ⟨a, hacc, ha⟩ | ⟨hmem', hfull'⟩
    · obtain ⟨a', ha', hd⟩ := hkeep a hacc ha
      exact ih _ (Or.inl ⟨a', ha', hd⟩)
    · rcases List.mem_cons.mp hmem' with heq | hrest
      · -- e is the full b₀ bucket: the step's result dominates b₀
        have hqe : quorumSize ≤ e.2.length := by
          rw [← heq]
          exact hfull'
        have he1 : e.1 = b₀ := by rw [← heq]
        refine ih _ (Or.inl ?_)
        rw [if_pos hqe]
        cases acc with
        | none => exact ⟨e, rfl, he1 ▸ Ballot.ble_refl _⟩
        | some a =>
          show ∃ a', (if a.1.blt e.1 then some e else some a) = some a' ∧
            b₀.ble a'.1 = true
          by_cases hlt : a.1.blt e.1
          · rw [if_pos hlt]
            exact ⟨e, rfl, he1 ▸ Ballot.ble_refl _⟩
          · rw [if_neg hlt]
            refine ⟨a, rfl, ?_⟩
            have hba := Ballot.ble_of_not_blt (fun hc => hlt hc)
            rw [he1] at hba
            exact hba
      · exact ih _ (Or.inr ⟨hrest, hfull'⟩)

/-- `pP1bView` inversion at the fold: a relevant view IS a (full) bucket of
the proposer's own ballot. -/
theorem pP1bView_some_bucket {quorumSize : Nat}
    {pfx : List (Ballot nP × P1bPayload P nP)} {myBallot : Ballot nP}
    {hasLargest : Bool} {qlogs : List (P1bPayload P nP)}
    (h : (pP1bView quorumSize pfx myBallot hasLargest).1 = some qlogs) :
    (myBallot, qlogs) ∈ foldEarlyStopBallots quorumSize pfx ∧
      quorumSize ≤ qlogs.length := by
  unfold pP1bView at h
  dsimp only at h
  revert h
  cases hmb : p1bMaxQuorumBallot quorumSize
      (foldEarlyStopBallots quorumSize pfx) with
  | none => intro h; cases h
  | some r =>
    obtain ⟨qb, vs⟩ := r
    intro h
    dsimp only at h
    by_cases hqb : qb = myBallot
    · rw [if_pos hqb] at h
      cases h
      subst hqb
      exact ⟨p1bMaxQuorumBallot_mem_self hmb,
        p1bMaxQuorumBallot_holds_quorum _ _ hmb⟩
    · rw [if_neg hqb] at h
      cases h

/-! ## `p_p1b`, transcribed across the located surface (paxos.rs:528–593)

The Rust body: `collect_quorum_with_response` on the (per-acceptor family
view of the) P1b reply stream, the keyed `fold_early_stop` + `get_max_key`
read through the **`NoOrder` snapshot** (paxos.rs:561–572 — the decision is
the per-tick arrival increment, adversary-ordered, which is also the
faithful model of the `assume_ordering::<TotalOrder>` at :550), zipped with
`p_ballot`/`p_has_largest_ballot`, and the fail-ballot stream computed from
the raw input (quorum.rs:82–85). -/

/-- The materialized `nondet!`s of `p_p1b` and its quorum slice. -/
structure PP1bNondet (P : Type) (nP : Nat) where
  /-- The `collect_quorum_with_response` slice's consumed batches (its own
  `sliced!` clock; `NoOrder` input ⇒ the decision is the batch itself). -/
  quorumBatch :
    List (List (Ballot nP × Except (Option (Ballot nP)) (P1bPayload P nP)))
  /-- The stale-snapshot arrival increments (paxos.rs:561–572), one list per
  proposer tick. -/
  quorumSnap : List (List (Ballot nP × P1bPayload P nP))

set_option synthInstance.maxSize 1024

variable [DecidableEq P]

/-! ## The verified artifact

The function face of the typed dataflow is `p_p1b` below: `.f`/`.mono` the
transcription stage (wire `let`s, monotone by construction), `.ensures` the
module contract, stated over the artifact's inputs and its **actual
output** `out = (p_is_leader flags, per-tick leader views, fail ballots)`.
The proofs are ghost `have`s inside the definition, over the applied
values. -/

section Verified

variable {nA : Nat}
variable (qs nqp : Nat) (nondet : PP1bNondet P nP)

/-- What `p_p1b` **requires** for its reign-stability guarantee, named per
guarantee (the caller's solicitation discipline; the election loop
discharges these from its own wires). `flags` is the artifact's own
leader-flag output (`out.1`) — the solicitation fact is a feedback fact
through the election cycle. -/
structure PP1bRequires
    (rs : Fin nA → Stream (Ballot nP × Except (Option (Ballot nP)) (P1bPayload P nP)))
    (pb : TSing (Ballot nP)) (gl : TSing Bool) (flags : TSing Bool)
    (me : Fin nP) : Prop where
  /-- `p_has_largest_ballot` is identically `true` on realized ticks. -/
  has_largest : ∀ x ∈ gl, x = true
  /-- Ballots are owned. -/
  ballot_own : ∀ b ∈ pb, (b : Ballot nP).proposerId = me
  /-- Ballot `num`s never decrease. -/
  ballot_mono : ∀ {t t' : Nat} (h : t ≤ t') (ht' : t' < pb.length),
    (pb[t]'(Nat.lt_of_le_of_lt h ht')).num ≤ (pb[t']'ht').num
  /-- Every `Ok`-promised ballot was solicited at a flag-`false` tick. -/
  solicited_at_follower : ∀ (j : Fin nA) (b : Ballot nP)
    (v : P1bPayload P nP), (b, Except.ok v) ∈ rs j →
    ∃ (u : Nat) (hu : u < flags.length) (hb : u < pb.length),
      pb[u]'hb = b ∧ flags[u]'hu = false

/-- The per-member **reply-cap requirement**, named: at most one `Ok`
promise per ballot per member (the B1 send-once fan-in). -/
def PP1bReplyCap
    (rs : Fin nA → Stream (Ballot nP × Except (Option (Ballot nP)) (P1bPayload P nP))) :
    Prop :=
  ∀ (j : Fin nA) (b : Ballot nP), (rs j).countP (okKey b) ≤ 1

/-- What `p_p1b` **ensures**, over its inputs and its actual output. -/
structure PP1bEnsures
    (rs : Fin nA → Stream (Ballot nP × Except (Option (Ballot nP)) (P1bPayload P nP)))
    (pb : TSing (Ballot nP)) (gl : TSing Bool) (qs : Nat)
    (out : TSing Bool × TStream (P1bPayload P nP)
      × (Fin nA → Stream (Ballot nP))) : Prop where
  /-- Trace alignment: realized flag ticks are realized ballot ticks. -/
  flags_ballot_len : out.1.length ≤ pb.length
  /-- Trace alignment: the view trace and the flag trace tick together. -/
  views_flags_len : out.2.1.length = out.1.length
  /-- Leader ticks carry a full view. -/
  leader_len : ∀ {t : Nat} (htv : t < out.2.1.length)
    (htf : t < out.1.length), out.1[t]'htf = true →
    qs ≤ (out.2.1[t]'htv).length
  /-- Leader-view provenance: every payload of a leader tick's view is an
  `Ok` reply of some member at the tick's own ballot (unconditional). -/
  view_promise : ∀ {t : Nat} (htv : t < out.2.1.length)
    (htf : t < out.1.length), out.1[t]'htf = true →
    ∀ {v : P1bPayload P nP}, v ∈ out.2.1[t]'htv →
    ∃ (hb : t < pb.length) (j : Fin nA), (pb[t]'hb, Except.ok v) ∈ rs j
  /-- Distinct providers, given the reply-cap requirement. -/
  leader_providers : 1 ≤ qs → ∀ {t : Nat} (htv : t < out.2.1.length)
    (htf : t < out.1.length), out.1[t]'htf = true → PP1bReplyCap rs →
    ∃ (hb : t < pb.length) (S : List (Fin nA)), S.Nodup ∧ qs ≤ S.length ∧
      ∀ j ∈ S, ∃ v ∈ out.2.1[t]'htv, (pb[t]'hb, Except.ok v) ∈ rs j
  /-- Frozen buckets: same-ballot leader ticks see the same view. -/
  qlogs_pinned : 1 ≤ qs → ∀ {t t' : Nat} (h : t ≤ t')
    (htv' : t' < out.2.1.length) (htf' : t' < out.1.length)
    {hbt : t < pb.length} {hbt' : t' < pb.length},
    pb[t]'hbt = pb[t']'hbt' →
    out.1[t]'(Nat.lt_of_le_of_lt h htf') = true →
    out.1[t']'htf' = true →
    out.2.1[t]'(Nat.lt_of_le_of_lt h htv') = out.2.1[t']'htv'
  /-- Ballot stability along reigns, given the solicitation requirements:
  a fabricated reign cannot bootstrap (FINDINGS D21). -/
  ballot_stable : 1 ≤ qs → ∀ {me : Fin nP},
    PP1bRequires rs pb gl out.1 me →
    ∀ {t : Nat} (ht1 : t + 1 < out.1.length),
    out.1[t + 1]'ht1 = true →
    out.1[t]'(Nat.lt_of_succ_lt ht1) = true →
    ∃ hb1 : t + 1 < pb.length,
      pb[t + 1]'hb1 = pb[t]'(Nat.lt_of_succ_lt hb1)

/-- **paxos.rs:528–593 `p_p1b`**: the single verified artifact. -/
def p_p1b :
    Verified ((Fin nA → Stream (Ballot nP × Except (Option (Ballot nP)) (P1bPayload P nP)))
        × TSing (Ballot nP) × TSing Bool)
      (TSing Bool × TStream (P1bPayload P nP) × (Fin nA → Stream (Ballot nP)))
      (fun x out => PP1bEnsures x.1 x.2.1 x.2.2 qs out) :=
  let rs := MonoMap.fst
  let pb := (MonoMap.snd (α := _)).fstOf
  let gl := (MonoMap.snd (α := _)).sndOf
  -- let (quorums, fails) = collect_quorum_with_response(…) (paxos.rs:545)
  let wr := collect_quorum_with_responseM qs nqp nondet.quorumBatch ∘ₘ rs
  -- .fold_early_stop(…).get_max_key().snapshot(tick, nondet!) (:548–572):
  -- the NoOrder snapshot views, folded per tick by the established pure
  -- fold (`pP1bView` composes fold_early_stop + get_max_key + the zip
  -- with p_ballot and the filter_map at :573–585)
  let views := (wr.fstOf.asCnt).snapshotC nondet.quorumSnap
  let pv := (views.zip (pb.zip gl)).map
    (fun vbl => pP1bView qs vbl.1 vbl.2.1 vbl.2.2)
  Verified.ofMono
    -- p_is_leader = p_received_quorum_of_p1bs.is_some().and(has_largest);
    -- p_received_quorum_of_p1bs.flatten_unordered() (:589);
    -- fails.flat_map_ordered(|(_, ballot)| ballot) (:591)
    (MonoMap.pair (pv.map (·.2))
      (MonoMap.pair (pv.map (fun x => x.1.getD []))
        (MonoMap.piMap (fun _j => filterMapM (fun ke => ke.2)) ∘ₘ wr.sndOf)))
    (fun x =>
    -- the applied values, bound once (the ghost vocabulary)
    let rsx := x.1
    let pbx := x.2.1
    let glx := x.2.2
    let qsrc := (collect_quorum_with_response rsx qs nqp nondet.quorumBatch).1
    let cutsx := snapshotC qsrc [] nondet.quorumSnap
    let pvx := (TSing.zip cutsx (TSing.zip pbx glx)).map
      (fun vbl => pP1bView qs vbl.1 vbl.2.1 vbl.2.2)
    let flagsx := pvx.map (fun e => e.2)
    let viewsx := pvx.map (fun e => e.1.getD [])
    -- ghost plumbing: the output legs are projections of the fused trace
    have hfl_len : flagsx.length = pvx.length := by
      show (pvx.map (fun e => e.2)).length = _
      rw [List.length_map]
    have hvw_len : viewsx.length = pvx.length := by
      show (pvx.map (fun e => e.1.getD [])).length = _
      rw [List.length_map]
    have hflag : ∀ {t : Nat} (ht : t < pvx.length)
        (htf : t < flagsx.length), flagsx[t]'htf = (pvx[t]'ht).2 := by
      intro t ht htf
      have htf' : t < (pvx.map (fun e => e.2)).length := htf
      show (pvx.map (fun e => e.2))[t]'htf' = _
      rw [List.getElem_map]
    have hviewel : ∀ {t : Nat} (ht : t < pvx.length)
        (htv : t < viewsx.length),
        viewsx[t]'htv = (pvx[t]'ht).1.getD [] := by
      intro t ht htv
      have htv' : t < (pvx.map (fun e => e.1.getD [])).length := htv
      show (pvx.map (fun e => e.1.getD []))[t]'htv' = _
      rw [List.getElem_map]
    -- ghost: the fused entry at a tick, opened (`pP1bView` of the tick's
    -- snapshot cut, ballot and has-largest inputs)
    have hopen : ∀ {t : Nat} (ht : t < pvx.length),
        ∃ (hv : t < cutsx.length) (hb : t < pbx.length)
          (hg : t < glx.length),
          pvx[t]'ht = pP1bView qs (cutsx[t]'hv) (pbx[t]'hb) (glx[t]'hg) := by
      intro t ht
      have ht0 : t < ((TSing.zip cutsx (TSing.zip pbx glx)).map
          (fun vbl => pP1bView qs vbl.1 vbl.2.1 vbl.2.2)).length := ht
      have hzip : t < (List.zip cutsx (List.zip pbx glx)).length := by
        rwa [List.length_map] at ht0
      have hlens := hzip
      rw [List.length_zip, List.length_zip] at hlens
      have hv : t < cutsx.length := by omega
      have hb : t < pbx.length := by omega
      have hg : t < glx.length := by omega
      refine ⟨hv, hb, hg, ?_⟩
      show ((TSing.zip cutsx (TSing.zip pbx glx)).map
        (fun vbl => pP1bView qs vbl.1 vbl.2.1 vbl.2.2))[t]'ht0 = _
      rw [List.getElem_map]
      show pP1bView qs ((List.zip cutsx (List.zip pbx glx))[t]'hzip).1 _ _
        = _
      rw [List.getElem_zip]
      dsimp only
      congr 1
      · show ((List.zip cutsx (List.zip pbx glx))[t]'hzip).2.1 = pbx[t]'hb
        rw [List.getElem_zip]
        dsimp only
        rw [List.getElem_zip]
      · show ((List.zip cutsx (List.zip pbx glx))[t]'hzip).2.2 = glx[t]'hg
        rw [List.getElem_zip]
        dsimp only
        rw [List.getElem_zip]
    -- ghost: leader-tick opening — the view is a full frozen bucket of the
    -- tick's own ballot inside the tick's snapshot cut
    have hbucket : ∀ {t : Nat} (ht : t < pvx.length),
        (pvx[t]'ht).2 = true →
        ∃ (hv : t < cutsx.length) (hb : t < pbx.length),
          (pbx[t]'hb, (pvx[t]'ht).1.getD [])
            ∈ foldEarlyStopBallots qs (cutsx[t]'hv) ∧
          qs ≤ ((pvx[t]'ht).1.getD []).length := by
      intro t ht hl
      obtain ⟨hv, hb, hg, hev⟩ := hopen ht
      rw [hev] at hl ⊢
      obtain ⟨q, hq⟩ :=
        Option.isSome_iff_exists.mp (pP1bView_leader_isSome hl)
      obtain ⟨hmem, hfull⟩ := pP1bView_some_bucket hq
      rw [hq]
      exact ⟨hv, hb, hmem, hfull⟩
    -- ghost: the masking face — at a flag-`false` tick with has-largest
    -- true, a full own bucket is only suppressed by a strictly larger full
    -- bucket in the same cut (the self-poisoning step of the regress)
    have hmask : ∀ {t : Nat} (ht : t < pvx.length),
        (pvx[t]'ht).2 = false → (∀ x ∈ glx, x = true) →
        ∀ {b₀ : Ballot nP} {vs : List (P1bPayload P nP)}
          (hv : t < cutsx.length) (hbt : t < pbx.length),
          pbx[t]'hbt = b₀ →
          (b₀, vs) ∈ foldEarlyStopBallots qs (cutsx[t]'hv) →
          qs ≤ vs.length →
          ∃ (qb : Ballot nP) (vs' : List (P1bPayload P nP)),
            (qb, vs') ∈ foldEarlyStopBallots qs (cutsx[t]'hv) ∧
            qs ≤ vs'.length ∧ b₀.blt qb = true := by
      intro t ht hfl hglt b₀ vs hv hbt hb₀ hmem hfull
      obtain ⟨hv', hb', hg', hev⟩ := hopen ht
      rw [hev] at hfl
      -- the has-largest input at this tick is true
      have hgl : glx[t]'hg' = true := hglt _ (List.getElem_mem hg')
      -- the max full bucket exists and dominates the tick's own full bucket
      obtain ⟨r, hr, hdom⟩ := p1bMaxQuorumBallot_ge hmem hfull
      -- the flag being false with has-largest true forces `relevant = none`
      have hne : r.1 ≠ b₀ := by
        intro heq
        unfold pP1bView at hfl
        dsimp only at hfl
        rw [hr] at hfl
        obtain ⟨qb, vsr⟩ := r
        dsimp only at hfl
        have hqb : qb = pbx[t]'hb' := heq.trans hb₀.symm
        rw [if_pos hqb, hgl] at hfl
        simp at hfl
      refine ⟨r.1, r.2, p1bMaxQuorumBallot_mem_self hr,
        p1bMaxQuorumBallot_holds_quorum _ _ hr, ?_⟩
      rcases Ballot.eq_or_blt_of_ble hdom with heq | hlt
      · exact absurd heq.symm hne
      · exact hlt
    -- ghost: bucket provenance — every full bucket's ballot was genuinely
    -- `Ok`-promised by some member (buckets are never fabricated)
    have hbpromise : 1 ≤ qs → ∀ {t : Nat} (hv : t < cutsx.length)
        {qb : Ballot nP} {vs : List (P1bPayload P nP)},
        (qb, vs) ∈ foldEarlyStopBallots qs (cutsx[t]'hv) →
        qs ≤ vs.length →
        ∃ (v : P1bPayload P nP) (j : Fin nA), (qb, Except.ok v) ∈ rsx j := by
      intro hq1 t hv qb vs hmem hfull
      -- the bucket is nonempty
      obtain ⟨v, hvvs⟩ : ∃ v, v ∈ vs := by
        cases vs with
        | nil => simp at hfull; omega
        | cons v vs' => exact ⟨v, List.mem_cons_self ..⟩
      have hcnt : 1 ≤ vs.countP (fun w => decide (w = v)) :=
        List.countP_pos_iff.mpr ⟨v, hvvs, by simp⟩
      have hview := Nat.le_trans hcnt (foldEarlyStop_count hmem v)
      obtain ⟨e, he, hpe⟩ := List.countP_pos_iff.mp hview
      rw [Bool.and_eq_true, decide_eq_true_iff, decide_eq_true_iff] at hpe
      have hee : e = (qb, v) := by
        obtain ⟨e1, e2⟩ := e
        rw [Prod.mk.injEq]
        exact hpe
      subst hee
      have hq := snapshotC_view_mem (List.getElem_mem hv) _ he
      obtain ⟨j, hj⟩ := collect_quorum_with_response_mem hq
      exact ⟨v, j, hj⟩
    -- ghost: full buckets persist — cuts accumulate and full buckets are
    -- frozen, so a full bucket is present verbatim at every later tick
    have hpersists : 1 ≤ qs → ∀ {t t' : Nat} (hle : t ≤ t')
        (hv' : t' < cutsx.length)
        {qb : Ballot nP} {vs : List (P1bPayload P nP)},
        (qb, vs) ∈ foldEarlyStopBallots qs
          (cutsx[t]'(Nat.lt_of_le_of_lt hle hv')) →
        qs ≤ vs.length →
        (qb, vs) ∈ foldEarlyStopBallots qs (cutsx[t']'hv') := by
      intro hq1 t t' hle hv' qb vs hmem hfull
      have hpre : cutsx[t]'(Nat.lt_of_le_of_lt hle hv') <+: cutsx[t']'hv' :=
        snapshotC_getElem_prefix hle hv'
      obtain ⟨ext, hext⟩ := hpre
      have hlen : vs.length = qs :=
        Nat.le_antisymm (foldEarlyStop_length_le hq1 _ _ hmem) hfull
      rw [← hext]
      exact foldEarlyStop_full_stable hmem hlen ext
    -- ghost: the fabricated-reign regress — no realized tick can be
    -- flag-`false` at its own ballot while that ballot holds a full bucket
    -- in the tick's cut (strong induction on the remaining ticks;
    -- FINDINGS D21)
    have hnff : 1 ≤ qs → ∀ {me : Fin nP},
        PP1bRequires rsx pbx glx flagsx me →
        ∀ (n u : Nat) (hu : u < pvx.length),
          pvx.length - u ≤ n →
          ∀ (b₀ : Ballot nP) (vs : List (P1bPayload P nP))
            (hb : u < pbx.length),
            pbx[u]'hb = b₀ →
            (pvx[u]'hu).2 = false →
            ∀ (hv : u < cutsx.length),
              (b₀, vs) ∈ foldEarlyStopBallots qs (cutsx[u]'hv) →
              qs ≤ vs.length → False := by
      intro hq1 me ho n
      induction n with
      | zero =>
        intro u hu hn b₀ vs hb hbeq hfl hv hmem hfull
        omega
      | succ n ih =>
        intro u hu hn b₀ vs hb hbeq hfl hv hmem hfull
        -- the mask: a strictly larger full own bucket in the same cut
        obtain ⟨qb, vs', hmem', hfull', hblt⟩ :=
          hmask hu hfl ho.has_largest hv hb hbeq hmem hfull
        -- its ballot was genuinely promised, hence solicited
        obtain ⟨v, j, hok⟩ := hbpromise hq1 hv hmem' hfull'
        obtain ⟨w, hwf, hwb, hwbeq, hwfl⟩ :=
          ho.solicited_at_follower j qb v hok
        have hw : w < pvx.length := by
          rw [← hfl_len]
          exact hwf
        have hwfl' : (pvx[w]'hw).2 = false := by
          rw [← hflag hw hwf]
          exact hwfl
        -- ownership: both ballots live on `me`'s wire
        have hb₀own : b₀.proposerId = me := by
          rw [← hbeq]
          exact ho.ballot_own _ (List.getElem_mem hb)
        have hqbown : qb.proposerId = me := by
          rw [← hwbeq]
          exact ho.ballot_own _ (List.getElem_mem hwb)
        -- strict `num` increase, so the solicitation tick is strictly later
        have hnum : b₀.num < qb.num := by
          rcases Ballot.blt_iff.mp hblt with hlt | ⟨-, hpid⟩
          · exact hlt
          · rw [hb₀own, hqbown] at hpid
            omega
        have huw : u < w := by
          rcases Nat.lt_or_ge u w with hlt | hge
          · exact hlt
          · exfalso
            have hmu : (pbx[w]'hwb).num ≤ (pbx[u]'hb).num :=
              ho.ballot_mono hge hb
            rw [hwbeq, hbeq] at hmu
            omega
        -- the full `qb` bucket persists to the solicitation tick
        obtain ⟨hvw, -, -, -⟩ := hopen hw
        have hpers := hpersists hq1 (Nat.le_of_lt huw) hvw hmem' hfull'
        exact ih w hw (by omega) qb vs' hwb hwbeq hwfl' hvw hpers hfull'
    { flags_ballot_len := by
        show flagsx.length ≤ pbx.length
        rw [hfl_len]
        show ((TSing.zip cutsx (TSing.zip pbx glx)).map
          (fun vbl => pP1bView qs vbl.1 vbl.2.1 vbl.2.2)).length ≤ _
        simp only [TSing.zip, List.length_map, List.length_zip]
        omega
      views_flags_len := by
        show viewsx.length = flagsx.length
        rw [hvw_len, hfl_len]
      leader_len := fun {t} htv htf hfl => by
        have htf' : t < flagsx.length := htf
        have htv' : t < viewsx.length := htv
        have ht : t < pvx.length := by
          rw [← hfl_len]
          exact htf'
        have hfl' : (pvx[t]'ht).2 = true := by
          rw [← hflag ht htf']
          exact hfl
        show qs ≤ (viewsx[t]'htv').length
        rw [hviewel ht htv']
        obtain ⟨-, -, -, hfull⟩ := hbucket ht hfl'
        exact hfull
      view_promise := fun {t} htv htf hfl {v} hv => by
        have htf' : t < flagsx.length := htf
        have htv' : t < viewsx.length := htv
        have ht : t < pvx.length := by
          rw [← hfl_len]
          exact htf'
        have hfl' : (pvx[t]'ht).2 = true := by
          rw [← hflag ht htf']
          exact hfl
        have hv0 : v ∈ viewsx[t]'htv' := hv
        rw [hviewel ht htv'] at hv0
        obtain ⟨hvw, hb, hmem, -⟩ := hbucket ht hfl'
        have hcnt : 1 ≤ ((pvx[t]'ht).1.getD []).countP
            (fun w => decide (w = v)) :=
          List.countP_pos_iff.mpr ⟨v, hv0, by simp⟩
        have hview := Nat.le_trans hcnt (foldEarlyStop_count hmem v)
        obtain ⟨e, he, hpe⟩ := List.countP_pos_iff.mp hview
        rw [Bool.and_eq_true, decide_eq_true_iff, decide_eq_true_iff] at hpe
        have hee : e = (pbx[t]'hb, v) := by
          obtain ⟨e1, e2⟩ := e
          rw [Prod.mk.injEq]
          exact hpe
        subst hee
        have hq := snapshotC_view_mem (List.getElem_mem hvw) _ he
        obtain ⟨j, hj⟩ := collect_quorum_with_response_mem hq
        exact ⟨hb, j, hj⟩
      leader_providers := fun hq1 {t} htv htf hfl hcap => by
        have htf' : t < flagsx.length := htf
        have htv' : t < viewsx.length := htv
        have ht : t < pvx.length := by
          rw [← hfl_len]
          exact htf'
        have hfl' : (pvx[t]'ht).2 = true := by
          rw [← hflag ht htf']
          exact hfl
        obtain ⟨hvw, hb, hmem, hfull⟩ := hbucket ht hfl'
        have hL : ∀ v, ((pvx[t]'ht).1.getD []).countP
            (fun w => decide (w = v)) ≤ outCnt (pbx[t]'hb) v qsrc := by
          intro v
          refine Nat.le_trans (foldEarlyStop_count hmem v) ?_
          refine Nat.le_trans (countP_le_of_count_le
            (fun x => snapshotC_view_count (fun _ => by simp)
              (List.getElem_mem hvw) x) _) ?_
          exact Nat.le_refl _
        obtain ⟨S, hnd, hlen, hprov⟩ := collect_quorum_with_response_providers
          hq1 _ hL (fun j => hcap j (pbx[t]'hb))
        show ∃ (hb : t < pbx.length) (S : List (Fin nA)),
          S.Nodup ∧ qs ≤ S.length ∧
          ∀ j ∈ S, ∃ v ∈ viewsx[t]'htv', (pbx[t]'hb, Except.ok v) ∈ rsx j
        rw [hviewel ht htv']
        exact ⟨hb, S, hnd, Nat.le_trans hfull hlen, hprov⟩
      qlogs_pinned := fun hq1 {t t'} h htv' htf' {hbt hbt'} hbeq hl hl' => by
        have htf0' : t' < flagsx.length := htf'
        have ht' : t' < pvx.length := by
          rw [← hfl_len]
          exact htf0'
        have ht : t < pvx.length := Nat.lt_of_le_of_lt h ht'
        have hl0 : (pvx[t]'ht).2 = true := by
          rw [← hflag ht (Nat.lt_of_le_of_lt h htf0')]
          exact hl
        have hl0' : (pvx[t']'ht').2 = true := by
          rw [← hflag ht' htf0']
          exact hl'
        have htv0' : t' < viewsx.length := htv'
        have htv0 : t < viewsx.length := Nat.lt_of_le_of_lt h htv0'
        show viewsx[t]'htv0 = viewsx[t']'htv0'
        rw [hviewel ht htv0, hviewel ht' htv0']
        suffices hopt : (pvx[t]'ht).1 = (pvx[t']'ht').1 by rw [hopt]
        obtain ⟨hv, hb, hg, hev⟩ := hopen ht
        obtain ⟨hv', hb', hg', hev'⟩ := hopen ht'
        rw [hev] at hl0 ⊢
        rw [hev'] at hl0' ⊢
        -- open both leader views
        obtain ⟨q, hq⟩ :=
          Option.isSome_iff_exists.mp (pP1bView_leader_isSome hl0)
        obtain ⟨q', hq'⟩ :=
          Option.isSome_iff_exists.mp (pP1bView_leader_isSome hl0')
        obtain ⟨hmem, hfull⟩ := pP1bView_some_bucket hq
        obtain ⟨hmem', -⟩ := pP1bView_some_bucket hq'
        -- the cuts chain
        have hpre : cutsx[t]'hv <+: cutsx[t']'hv' :=
          snapshotC_getElem_prefix h hv'
        obtain ⟨ext, hext⟩ := hpre
        rw [← hext] at hmem'
        -- ballots agree
        have hbb : pbx[t]'hbt = pbx[t']'hbt' := hbeq
        rw [show pbx[t']'hb' = pbx[t]'hb from by
          rw [show pbx[t]'hb = pbx[t]'hbt from rfl, hbb]] at hmem'
        have := foldEarlyStop_full_pin hq1 hmem hfull hmem'
        rw [hq, hq', this]
      ballot_stable := fun hq1 {me} ho {t} ht1 hl1 hl0 => by
        have htf1 : t + 1 < flagsx.length := ht1
        have hpt1 : t + 1 < pvx.length := by
          rw [← hfl_len]
          exact htf1
        have hpt0 : t < pvx.length := Nat.lt_of_succ_lt hpt1
        have hl1' : (pvx[t + 1]'hpt1).2 = true := by
          rw [← hflag hpt1 htf1]
          exact hl1
        have hl0' : (pvx[t]'hpt0).2 = true := by
          rw [← hflag hpt0 (Nat.lt_of_succ_lt htf1)]
          exact hl0
        obtain ⟨-, hpb1, -, -⟩ := hopen hpt1
        refine ⟨hpb1, ?_⟩
        refine Classical.byContradiction fun hne0 => ?_
        have hne : ¬ pbx[t + 1]'hpb1 = pbx[t]'(Nat.lt_of_succ_lt hpb1) :=
          hne0
        -- ownership + monotonicity force a strict `num` increase
        have hown' : (pbx[t + 1]'hpb1).proposerId = me :=
          ho.ballot_own _ (List.getElem_mem hpb1)
        have hown0 : (pbx[t]'(Nat.lt_of_succ_lt hpb1)).proposerId = me :=
          ho.ballot_own _ (List.getElem_mem (Nat.lt_of_succ_lt hpb1))
        have hmono0 : (pbx[t]'(Nat.lt_of_succ_lt hpb1)).num
            ≤ (pbx[t + 1]'hpb1).num :=
          ho.ballot_mono (Nat.le_succ t) hpb1
        have hnum : (pbx[t]'(Nat.lt_of_succ_lt hpb1)).num
            < (pbx[t + 1]'hpb1).num := by
          rcases Nat.lt_or_ge (pbx[t]'(Nat.lt_of_succ_lt hpb1)).num
              (pbx[t + 1]'hpb1).num with hlt | hge
          · exact hlt
          · exact absurd (Ballot.ext' (by omega)
              (by rw [hown0, hown'])).symm hne
        -- the leader flag at `t+1` exposes a full bucket at the new ballot
        obtain ⟨hv1, hbt1, hmem1, hfull1⟩ := hbucket hpt1 hl1'
        have hmem1' : (pbx[t + 1]'hpb1, (pvx[t + 1]'hpt1).1.getD [])
            ∈ foldEarlyStopBallots qs (cutsx[t + 1]'hv1) := hmem1
        -- the new ballot was solicited at a flag-false tick
        obtain ⟨v, j, hok⟩ := hbpromise hq1 hv1 hmem1' hfull1
        obtain ⟨u, huf, hub, hubeq, hufl⟩ :=
          ho.solicited_at_follower j _ v hok
        have hu : u < pvx.length := by
          rw [← hfl_len]
          exact huf
        have hufl' : (pvx[u]'hu).2 = false := by
          rw [← hflag hu huf]
          exact hufl
        -- the solicitation tick is strictly after `t+1`
        have hut : t + 1 < u := by
          rcases Nat.lt_trichotomy u (t + 1) with hlt | heq | hgt
          · exfalso
            have hle : u ≤ t := Nat.lt_succ_iff.mp hlt
            have hmu : (pbx[u]'hub).num
                ≤ (pbx[t]'(Nat.lt_of_succ_lt hpb1)).num :=
              ho.ballot_mono hle (Nat.lt_of_succ_lt hpb1)
            rw [hubeq] at hmu
            omega
          · exfalso
            subst heq
            have hcontr : (pvx[t + 1]'hpt1).2 = false := hufl'
            rw [hl1'] at hcontr
            cases hcontr
          · exact hgt
        -- the full bucket persists to the solicitation tick; the regress
        -- kills it
        obtain ⟨hvu, -, -, -⟩ := hopen hu
        have hpers := hpersists hq1 (Nat.le_of_lt hut) hvu hmem1' hfull1
        exact hnff hq1 ho pvx.length u hu (by omega) _ _ hub hubeq hufl'
          hvu hpers hfull1 })

end Verified

end HydroLean.Programs.Paxos
