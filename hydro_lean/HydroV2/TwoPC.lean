import HydroV2.Std.Quorum
import HydroV2.EagerProj

/-!
# Port of `two_pc` (Rust: `hydro_test/src/cluster/two_pc.rs`) — V2

Two-phase commit as a thin wire composition over the **shared**
`hydro_std::quorum` stage (`Std/Quorum.lean` — the same
`collect_quorum` the Paxos port consumes):

```rust
pub fn two_pc(coordinator, participants, num_participants, payloads) -> … {
    let p_prepare = payloads.broadcast(participants, …, nondet!(…));
    let c_votes   = p_prepare.send(coordinator, …).values();
    let (c_all_vote_yes, _) = collect_quorum(c_votes.map(|kv| (kv, Ok(()))), n, n);
    let p_commit  = c_all_vote_yes.broadcast(participants, …, nondet!(…));
    let c_commits = p_commit.send(coordinator, …).values();
    let (c_all_commit, _) = collect_quorum(c_commits.map(|kv| (kv, Ok(()))), n, n);
    c_all_commit
}
```

**Where the nondeterminism lives**: the cluster→coordinator fan-ins are
`NoOrder` pools; the two `collect_quorum` stages consume batch-cut
decisions — one execution is pinned by exactly two content decisions
(+ transports/emits, `Unit` at `Values`). The Rust `nondet!(/** TODO */)`
justifications are *theorems* (`TwoPCTheory` section below): the
committed output is invariant under **all** complete decisions.

**The vote hook** (V1 precedent): Rust maps every echoed payload to
`Ok(())`; we generalize with `vote : Fin n → P → Bool` — participant
`j` votes `Ok ()` on `p` iff `vote j p`, recovering the Rust code at
`vote = fun _ _ => true`. This makes unanimity a non-vacuous theorem.
-/

namespace HydroV2

variable {L : Type} {mem : L → Nat} {P : Type} [DecidableEq P]

/-- Participant `j`'s vote on payload `p`, as the `Result<(), ()>` it
sends back (`Ok(())` = commit vote). -/
def voteE {n : Nat} (vote : Fin n → P → Bool) (j : Fin n) (p : P) :
    Except Unit Unit :=
  if vote j p then .ok () else .error ()

/-- The decisions of one 2PC execution. -/
structure TwoPCDec (H : HydroSem L mem) (nC nPt : Nat) (P : Type)
    [DecidableEq P] where
  /-- `payloads.broadcast(&participants, …)`: prepare transport. -/
  prepCh : H.TransportDec nPt nC
  /-- The vote echo's `send_bincode(&coordinator)` transport. -/
  voteCh : H.TransportDec nC nPt
  /-- Phase-1 `collect_quorum`'s `use::batch(…, nondet!(…))`. -/
  votes : H.BatchDec nC (P × Except Unit Unit)
  /-- Phase-1 quorum emission. -/
  votesEmit : H.EmitDec nC P
  /-- `c_all_vote_yes.broadcast(&participants, …)`: commit transport. -/
  commitCh : H.TransportDec nPt nC
  /-- The ack echo's transport. -/
  ackCh : H.TransportDec nC nPt
  /-- Phase-2 `collect_quorum`'s `use::batch(…, nondet!(…))`. -/
  acks : H.BatchDec nC (P × Except Unit Unit)
  /-- Phase-2 quorum emission. -/
  acksEmit : H.EmitDec nC P

/-! ## The canonical pools (the `Values` fan-ins, in closed form) -/

/-- The prepare pool a participant sees: every coordinator member's
payloads, unordered. -/
def tpcPrepPool {nC : Nat} (payloads : Fin nC → List P) : Multiset P :=
  ((List.finRange nC).map (fun i => (↑(payloads i) : Multiset P))).sum

/-- The vote pool at the coordinator: every participant's echo of the
prepare pool through its vote. -/
def tpcVotesPool {nC : Nat} (nPt : Nat) (vote : Fin nPt → P → Bool)
    (payloads : Fin nC → List P) : Multiset (P × Except Unit Unit) :=
  ((List.finRange nPt).map (fun j =>
    (tpcPrepPool payloads).map (fun p => (p, voteE vote j p)))).sum

/-- The ack pool at the coordinator, given the phase-1 outputs: every
participant acks every committed payload it sees. -/
def tpcAckPool {nC : Nat} (nPt : Nat) (voteYes : Fin nC → Multiset P) :
    Multiset (P × Except Unit Unit) :=
  ((List.finRange nPt).map (fun _j =>
    (((List.finRange nC).map (fun c => voteYes c)).sum).map
      (fun p => (p, (Except.ok () : Except Unit Unit))))).sum

/-- What `two_pc` **ensures**, over the `Values` denotation — stated
against the phase-1 wire `voteYes` (the existential witness, PP1b's
`okPool` pattern) and the canonical pools; the four clauses are the
shared quorum stage's faces (`CQEnsures`) at the two fan-ins. -/
structure TPCEnsures {nC : Nat} (nPt : Nat) (vote : Fin nPt → P → Bool)
    (payloads : Fin nC → List P)
    (dvotes dacks : BatchCuts nC (P × Except Unit Unit))
    (voteYes : Fin nC → Multiset P)
    (out : Fin nC → Multiset P) : Prop where
  /-- Phase-1 soundness: a vote-yes payload holds `n` `Ok` votes among
  the consumed vote pool. -/
  vy_sound : ∀ c, ∀ p ∈ voteYes c,
    nPt ≤ cqOkCount
      (cqConsumed (tpcVotesPool nPt vote payloads) (dvotes c)) p
  /-- Phase-1 crossing count (under the usage contract): emitted iff
  the consumed pool holds `n` `Ok` votes — exactly once. -/
  vy_count : ∀ c (p : P), 1 ≤ nPt →
    cqKeyCount
      (cqConsumed (tpcVotesPool nPt vote payloads) (dvotes c)) p ≤ nPt →
    (voteYes c).count p
      = if nPt ≤ cqOkCount
          (cqConsumed (tpcVotesPool nPt vote payloads) (dvotes c)) p
        then 1 else 0
  /-- Phase-2 soundness, at the ack pool of phase 1. -/
  commit_sound : ∀ c, ∀ p ∈ out c,
    nPt ≤ cqOkCount
      (cqConsumed (tpcAckPool nPt voteYes) (dacks c)) p
  /-- Phase-2 crossing count. -/
  commit_count : ∀ c (p : P), 1 ≤ nPt →
    cqKeyCount
      (cqConsumed (tpcAckPool nPt voteYes) (dacks c)) p ≤ nPt →
    (out c).count p
      = if nPt ≤ cqOkCount
          (cqConsumed (tpcAckPool nPt voteYes) (dacks c)) p
        then 1 else 0

/-- **two_pc.rs `two_pc`**: one Rust fn, one Lean def. Returns
`c_all_commit`. -/
def two_pc (H : HydroSem L mem) (coord part : L)
    (num_participants : Nat) (vote : Fin (mem part) → P → Bool)
    (payloads : H.Stream coord P .totalOrder .exactlyOnce)
    (dec : TwoPCDec H (mem coord) (mem part) P) :
    H.Stream coord P .noOrder .exactlyOnce
  ensures out => num_participants = mem part →
    ∃ voteYes, TPCEnsures (mem part) vote payloads dec.votes dec.acks
      voteYes out :=
  ghost intro hn
  ghost subst hn
  -- let p_prepare = payloads.broadcast(&participants, …).values();
  let p_prepare := H.values (H.broadcast dec.prepCh payloads)
  -- the vote echo: .map(|kv| (kv, Ok(()))) with the vote hook
  let p_votes := H.map p_prepare (fun j p => (p, voteE vote j p))
  -- .send_bincode(&coordinator).values()
  let c_votes := H.values (H.broadcast dec.voteCh p_votes)
  -- let (c_all_vote_yes, _) = collect_quorum(c_votes, n, n);
  let cq1 := collect_quorum H coord c_votes num_participants
    num_participants dec.votes dec.votesEmit
  -- the phase-1 wire IS the canonical pool crossing (definitional
  -- at `Values`): its quorum faces, at the vote pool
  ghost have hcq1 := cq1.property rfl
  ghost witness cq1.val.1
  -- let p_commit = c_all_vote_yes.broadcast(&participants, …).values();
  let p_commit := H.values (H.broadcast dec.commitCh cq1.val.1)
  -- the ack echo (always `Ok`)
  let p_acks := H.map p_commit
    (fun _j p => (p, (Except.ok () : Except Unit Unit)))
  -- .send_bincode(&coordinator).values()
  let c_acks := H.values (H.broadcast dec.ackCh p_acks)
  -- let (c_all_commit, _) = collect_quorum(c_commits, n, n);
  let cq2 := collect_quorum H coord c_acks num_participants
    num_participants dec.acks dec.acksEmit
  -- the phase-2 quorum faces, at the ack pool of phase 1
  ghost have hcq2 := cq2.property rfl
  cq2.val.1
  prove
    vy_sound := hcq1.emit_sound,
    vy_count := (fun c p h1 hcap =>
      hcq1.emit_count c p h1 le_rfl hcap),
    commit_sound := hcq2.emit_sound,
    commit_count := (fun c p h1 hcap =>
      hcq2.emit_count c p h1 le_rfl hcap)

/-! ## TwoPCTheory — the Rust `nondet!` justifications, as theorems

Derived from the contract's quorum faces by multiset accounting over
the canonical pools (single coordinator, the Rust shape). -/

section TwoPCTheory

variable {nPt : Nat} {vote : Fin nPt → P → Bool}
  {payloads : Fin 1 → List P}
  {dvotes dacks : BatchCuts 1 (P × Except Unit Unit)}
  {voteYes out : Fin 1 → Multiset P}

/-- Counts distribute over pool fan-ins. -/
theorem cqOkCount_listSum (l : List (Multiset (P × Except Unit Unit)))
    (k : P) :
    cqOkCount l.sum k = (l.map (fun m => cqOkCount m k)).sum := by
  induction l with
  | nil => simp [cqOkCount]
  | cons a t ih =>
    rw [List.sum_cons, cqOkCount_add, List.map_cons, List.sum_cons, ih]

theorem cqKeyCount_listSum (l : List (Multiset (P × Except Unit Unit)))
    (k : P) :
    cqKeyCount l.sum k = (l.map (fun m => cqKeyCount m k)).sum := by
  induction l with
  | nil => simp [cqKeyCount]
  | cons a t ih =>
    rw [List.sum_cons, cqKeyCount_add, List.map_cons, List.sum_cons, ih]

/-- `Ok` votes of `p` in one participant's echo: its vote times the
payload's multiplicity. -/
theorem cqOkCount_echo (pool : Multiset P) (j : Fin nPt) (p : P) :
    cqOkCount (pool.map (fun q => (q, voteE vote j q))) p
      = if vote j p then pool.count p else 0 := by
  unfold cqOkCount
  rw [Multiset.filter_map, Multiset.card_map]
  by_cases hv : vote j p
  · rw [if_pos hv, Multiset.count_eq_card_filter_eq]
    refine congrArg Multiset.card (Multiset.filter_congr ?_)
    intro q _
    simp only [Function.comp]
    constructor
    · rintro ⟨h1, _⟩
      exact h1.symm
    · rintro rfl
      refine ⟨rfl, ?_⟩
      show (voteE vote j p).isOk = true
      unfold voteE
      rw [if_pos hv]
      rfl
  · rw [if_neg hv, Multiset.card_eq_zero, Multiset.filter_eq_nil]
    intro q _
    simp only [Function.comp]
    rintro ⟨h1, h2⟩
    subst h1
    unfold voteE at h2
    rw [if_neg hv] at h2
    exact absurd h2 (by decide)

/-- Responses of `p` in one participant's echo: the payload's
multiplicity. -/
theorem cqKeyCount_echo (pool : Multiset P) (j : Fin nPt) (p : P) :
    cqKeyCount (pool.map (fun q => (q, voteE vote j q))) p
      = pool.count p := by
  unfold cqKeyCount
  rw [Multiset.filter_map, Multiset.card_map,
    Multiset.count_eq_card_filter_eq]
  refine congrArg Multiset.card (Multiset.filter_congr ?_)
  intro q _
  simp only [Function.comp]
  exact ⟨fun h => h.symm, fun h => h.symm⟩

/-- `Ok` acks of `p`: its multiplicity in the committed wire. -/
theorem cqOkCount_ack (m : Multiset P) (p : P) :
    cqOkCount (m.map (fun q => (q, (Except.ok () : Except Unit Unit)))) p
      = m.count p := by
  unfold cqOkCount
  rw [Multiset.filter_map, Multiset.card_map,
    Multiset.count_eq_card_filter_eq]
  refine congrArg Multiset.card (Multiset.filter_congr ?_)
  intro q _
  simp only [Function.comp]
  constructor
  · rintro ⟨h1, _⟩
    exact h1.symm
  · rintro rfl
    exact ⟨rfl, rfl⟩

theorem cqKeyCount_ack (m : Multiset P) (p : P) :
    cqKeyCount (m.map (fun q => (q, (Except.ok () : Except Unit Unit)))) p
      = m.count p := by
  unfold cqKeyCount
  rw [Multiset.filter_map, Multiset.card_map,
    Multiset.count_eq_card_filter_eq]
  refine congrArg Multiset.card (Multiset.filter_congr ?_)
  intro q _
  simp only [Function.comp]
  exact ⟨fun h => h.symm, fun h => h.symm⟩

/-- Indicator sums count filters. -/
theorem tpc_sum_indicator {n : Nat} (l : List (Fin n))
    (q : Fin n → Bool) :
    (l.map (fun j => if q j then 1 else 0)).sum = (l.filter q).length := by
  induction l with
  | nil => rfl
  | cons a t ih =>
    by_cases hq : q a
    · simp [hq, ih, Nat.add_comm]
    · simp [hq, ih]

/-- The vote pool's `Ok` count of a once-proposed payload: the number
of yes-voters. -/
theorem tpc_okCount_votes (hone : (tpcPrepPool payloads).count p = 1) :
    cqOkCount (tpcVotesPool nPt vote payloads) p
      = ((List.finRange nPt).filter (fun j => vote j p)).length := by
  unfold tpcVotesPool
  rw [cqOkCount_listSum, List.map_map]
  have : (List.finRange nPt).map
      ((fun m => cqOkCount m p)
        ∘ fun j => (tpcPrepPool payloads).map
          (fun q => (q, voteE vote j q)))
      = (List.finRange nPt).map (fun j => if vote j p then 1 else 0) := by
    refine List.map_congr_left (fun j _ => ?_)
    show cqOkCount ((tpcPrepPool payloads).map
      (fun q => (q, voteE vote j q))) p = _
    rw [cqOkCount_echo, hone]
  rw [this]
  exact tpc_sum_indicator (List.finRange nPt) (fun j => vote j p)

theorem tpc_sum_replicate (n a : Nat) :
    (List.replicate n a).sum = n * a := by
  induction n with
  | zero => simp
  | succ k ih =>
    rw [List.replicate_succ, List.sum_cons, ih, Nat.succ_mul,
      Nat.add_comm]

/-- The vote pool's response count: `n` times the payload's
multiplicity. -/
theorem tpc_keyCount_votes (p : P) :
    cqKeyCount (tpcVotesPool nPt vote payloads) p
      = nPt * (tpcPrepPool payloads).count p := by
  unfold tpcVotesPool
  rw [cqKeyCount_listSum, List.map_map]
  have : (List.finRange nPt).map
      ((fun m => cqKeyCount m p)
        ∘ fun j => (tpcPrepPool payloads).map
          (fun q => (q, voteE vote j q)))
      = (List.finRange nPt).map
          (fun _j => (tpcPrepPool payloads).count p) := by
    refine List.map_congr_left (fun j _ => ?_)
    exact cqKeyCount_echo _ j p
  rw [this, List.map_const', tpc_sum_replicate, List.length_finRange]

/-- The ack pool's counts: `n` times the committed multiplicity. -/
theorem tpc_okCount_acks (voteYes : Fin 1 → Multiset P) (p : P) :
    cqOkCount (tpcAckPool nPt voteYes) p
      = nPt * (voteYes 0).count p := by
  unfold tpcAckPool
  rw [cqOkCount_listSum, List.map_map]
  have h0 : ((List.finRange 1).map (fun c => voteYes c)).sum
      = voteYes 0 := by
    rw [show (List.finRange 1) = [(0 : Fin 1)] from rfl]
    simp
  have : (List.finRange nPt).map
      ((fun m => cqOkCount m p) ∘ fun _j =>
        (((List.finRange 1).map (fun c => voteYes c)).sum).map
          (fun q => (q, (Except.ok () : Except Unit Unit))))
      = (List.finRange nPt).map (fun _j => (voteYes 0).count p) := by
    refine List.map_congr_left (fun j _ => ?_)
    show cqOkCount _ p = _
    rw [h0, cqOkCount_ack]
  rw [this, List.map_const', tpc_sum_replicate, List.length_finRange]

theorem tpc_keyCount_acks (voteYes : Fin 1 → Multiset P) (p : P) :
    cqKeyCount (tpcAckPool nPt voteYes) p
      = nPt * (voteYes 0).count p := by
  unfold tpcAckPool
  rw [cqKeyCount_listSum, List.map_map]
  have h0 : ((List.finRange 1).map (fun c => voteYes c)).sum
      = voteYes 0 := by
    rw [show (List.finRange 1) = [(0 : Fin 1)] from rfl]
    simp
  have : (List.finRange nPt).map
      ((fun m => cqKeyCount m p) ∘ fun _j =>
        (((List.finRange 1).map (fun c => voteYes c)).sum).map
          (fun q => (q, (Except.ok () : Except Unit Unit))))
      = (List.finRange nPt).map (fun _j => (voteYes 0).count p) := by
    refine List.map_congr_left (fun j _ => ?_)
    show cqKeyCount _ p = _
    rw [h0, cqKeyCount_ack]
  rw [this, List.map_const', tpc_sum_replicate, List.length_finRange]

/-- **Unanimity** (`two_pc`'s safety headline; V1 `twoPC_unanimity`):
a committed payload proposed once was voted yes by **every**
participant — under every decision. -/
theorem two_pc_unanimous
    (h : TPCEnsures nPt vote payloads dvotes dacks voteYes out)
    (hn : 1 ≤ nPt) {p : P} (hp : p ∈ out 0)
    (hone : (tpcPrepPool payloads).count p = 1) :
    ∀ j, vote j p = true := by
  -- phase 2 → p entered the ack pool → p ∈ voteYes
  have hs2 := h.commit_sound 0 p hp
  have hmem : 1 ≤ cqOkCount
      (cqConsumed (tpcAckPool nPt voteYes) (dacks 0)) p :=
    le_trans hn hs2
  have hle : cqOkCount (cqConsumed (tpcAckPool nPt voteYes) (dacks 0)) p
      ≤ cqOkCount (tpcAckPool nPt voteYes) p :=
    cqOkCount_mono (cqConsumed_le _ _) p
  have hvy : 1 ≤ (voteYes 0).count p := by
    have hgt := le_trans hmem hle
    rw [tpc_okCount_acks] at hgt
    rcases Nat.eq_zero_or_pos ((voteYes 0).count p) with h0 | h1
    · rw [h0, Nat.mul_zero] at hgt
      omega
    · exact h1
  have hpvy : p ∈ voteYes 0 := Multiset.one_le_count_iff_mem.mp hvy
  -- phase 1 → n yes-votes among consumed ≤ pool → all voted yes
  have hs1 := h.vy_sound 0 p hpvy
  have hle1 : cqOkCount
      (cqConsumed (tpcVotesPool nPt vote payloads) (dvotes 0)) p
      ≤ cqOkCount (tpcVotesPool nPt vote payloads) p :=
    cqOkCount_mono (cqConsumed_le _ _) p
  have hcount := le_trans hs1 hle1
  rw [tpc_okCount_votes hone] at hcount
  have hlen : ((List.finRange nPt).filter (fun j => vote j p)).length
      = (List.finRange nPt).length := by
    have hub := List.length_filter_le (fun j => vote j p)
      (List.finRange nPt)
    rw [List.length_finRange] at hub ⊢
    omega
  intro j
  have hall := List.length_filter_eq_length_iff.mp hlen j
    (List.mem_finRange j)
  simpa using hall

/-- **At-most-once** (V1 `twoPC_nodup`): a once-proposed payload
commits at most once — under every decision. -/
theorem two_pc_once
    (h : TPCEnsures nPt vote payloads dvotes dacks voteYes out)
    (hn : 1 ≤ nPt) (p : P)
    (hone : (tpcPrepPool payloads).count p ≤ 1) :
    (out 0).count p ≤ 1 := by
  -- the phase-1 cap holds, so voteYes commits at most once…
  have hcap1 : cqKeyCount
      (cqConsumed (tpcVotesPool nPt vote payloads) (dvotes 0)) p ≤ nPt := by
    refine le_trans (cqKeyCount_mono (cqConsumed_le _ _) p) ?_
    rw [tpc_keyCount_votes]
    exact le_trans (Nat.mul_le_mul_left _ hone) (by omega)
  have hvy := h.vy_count 0 p hn hcap1
  have hvy1 : (voteYes 0).count p ≤ 1 := by
    rw [hvy]; split <;> omega
  -- …so the phase-2 cap holds, and the commit is an indicator
  have hcap2 : cqKeyCount
      (cqConsumed (tpcAckPool nPt voteYes) (dacks 0)) p ≤ nPt := by
    refine le_trans (cqKeyCount_mono (cqConsumed_le _ _) p) ?_
    rw [tpc_keyCount_acks]
    exact le_trans (Nat.mul_le_mul_left _ hvy1) (by omega)
  have hout := h.commit_count 0 p hn hcap2
  rw [hout]; split <;> omega

/-- **Commit-iff (the master theorem; V1 `twoPC_committed_eq`)**: under
*complete* decisions (both stages consume their whole pools), a
once-proposed payload commits **iff** every participant voted yes —
the batching `nondet!`s are invisible in the output. -/
theorem two_pc_commit_iff
    (h : TPCEnsures nPt vote payloads dvotes dacks voteYes out)
    (hn : 1 ≤ nPt) (p : P)
    (hone : (tpcPrepPool payloads).count p = 1)
    (hcv : cqConsumed (tpcVotesPool nPt vote payloads) (dvotes 0)
      = tpcVotesPool nPt vote payloads)
    (hca : cqConsumed (tpcAckPool nPt voteYes) (dacks 0)
      = tpcAckPool nPt voteYes) :
    p ∈ out 0 ↔ ∀ j, vote j p = true := by
  constructor
  · exact fun hp => two_pc_unanimous h hn hp hone
  · intro hall
    -- all yes → the vote pool crosses at p → voteYes count 1
    have hok1 : cqOkCount (tpcVotesPool nPt vote payloads) p = nPt := by
      rw [tpc_okCount_votes hone]
      have : (List.finRange nPt).filter (fun j => vote j p)
          = List.finRange nPt := by
        apply List.filter_eq_self.mpr
        intro j _
        simp [hall j]
      rw [this, List.length_finRange]
    have hcap1 : cqKeyCount
        (cqConsumed (tpcVotesPool nPt vote payloads) (dvotes 0)) p
        ≤ nPt := by
      rw [hcv, tpc_keyCount_votes, hone]
      omega
    have hvy := h.vy_count 0 p hn hcap1
    rw [hcv, hok1, if_pos le_rfl] at hvy
    -- → the ack pool crosses at p → committed
    have hok2 : cqOkCount (tpcAckPool nPt voteYes) p = nPt := by
      rw [tpc_okCount_acks, hvy]
      omega
    have hcap2 : cqKeyCount
        (cqConsumed (tpcAckPool nPt voteYes) (dacks 0)) p ≤ nPt := by
      rw [hca, tpc_keyCount_acks, hvy]
      omega
    have hout := h.commit_count 0 p hn hcap2
    rw [hca, hok2, if_pos le_rfl] at hout
    exact Multiset.one_le_count_iff_mem.mp (by omega)

end TwoPCTheory

end HydroV2
