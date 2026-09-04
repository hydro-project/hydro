import Hydro.Std.Quorum
import Hydro.EagerProj

/-!
# Port of `two_pc` (Rust: `hydro_test/src/cluster/two_pc.rs`)

Two-phase commit as a thin wire composition over the **shared**
`hydro_std::quorum` stage (`Std/Quorum.lean` — the same
`collect_quorum` the Paxos port consumes):

```rust
pub fn two_pc(coordinator: &Process<Coordinator>, participants: &Cluster<Participant>,
              num_participants, payloads: Stream<Payload, Process<Coordinator>, Unbounded, NoOrder>) -> … {
    let p_prepare = payloads.broadcast(participants, …, nondet!(/** TODO */));        // :25
    let c_votes   = p_prepare.send(coordinator, …).values();                          // :33
    let (c_all_vote_yes, _) = collect_quorum(c_votes.map(|kv| (kv, Ok(()))), n, n);   // :40
    let p_commit  = c_all_vote_yes.broadcast(participants, …, nondet!(/** TODO */));  // :49
    let c_commits = p_commit.send(coordinator, …).values();                           // :56
    let (c_all_commit, _) = collect_quorum(c_commits.map(|kv| (kv, Ok(()))), n, n);   // :61
    c_all_commit                                                                      // :68
}
```

**Where the nondeterminism lives**: the two `collect_quorum` stages consume
batch-cut decisions (`TwoPCDec`, two content decisions); the four
network edges carry delivery cursors (`TwoPCSched`, `Unit` at `Values`).
two_pc.rs's two `nondet!(/** TODO */)`s at the broadcasts are
`nondet_membership` — assumed away by closed membership (FINDINGS D20).

**The coordinator is a `Process`** (one member): the port takes a cluster
`coord : L`; the faithful premise `mem coord = 1` is the hypothesis of
the at-most-once face (with two coordinators every coordinator receives
every vote, so both would vote-yes the same payload and the ack pool
would hold `2n` copies — the quorum stage's `max` purge then re-emits).

**The vote hook**: Rust tags every echo `Ok(())` at the coordinator
(`c_votes.map(|kv| (kv, Ok(())))`, two_pc.rs:41 — participants never
vote; "participant 1 aborts" is a TODO comment). Here each participant
tags its own echo with `vote j p` before the send, recovering the Rust
at `vote = fun _ _ => true`; this is what makes unanimity a non-vacuous
theorem. It is the one line of this program that is not the Rust's.

**The contract** (`TPCEnsures`) is over the output `c_all_commit` and
the inputs only (DOCTRINE R3): a commit was proposed; a once-proposed
commit was voted yes by every participant; a once-proposed payload
commits at most once. The *completeness* direction ("unanimous and
complete decisions ⇒ commits") is two_pc's liveness rung
(`LIVENESS.md`): a decision's completeness is relative to the pool it
consumes, which for the ack stage is an internal wire — the chain layer
states it pool-free from fairness.
-/

namespace Hydro

variable {L : Type} {mem : L → Nat} {P : Type} [DecidableEq P]

/-! ## Prerequisites for the contract face — the program starts at
`hydro def two_pc` -/

/-- Participant `j`'s vote on payload `p`, as the `Result<(), ()>` it
sends back (`Ok(())` = commit vote). -/
def voteE {n : Nat} (vote : Fin n → P → Bool) (j : Fin n) (p : P) :
    Except Unit Unit :=
  if vote j p then .ok () else .error ()

/-- The decisions (content nondeterminism) of one 2PC execution. -/
structure TwoPCDec (H : HydroSem L mem) (nC nPt : Nat) (P : Type)
    [DecidableEq P] where
  /-- Phase-1 `collect_quorum`'s `use::batch(…, nondet!(…))`. -/
  votes : H.BatchDec nC (P × Except Unit Unit)
  /-- Phase-2 `collect_quorum`'s `use::batch(…, nondet!(…))`. -/
  acks : H.BatchDec nC (P × Except Unit Unit)

/-- **`two_pc`'s adversary-side (sched-det) bundle** (`Unit` at
`Values`; delivery cursors at the machine). -/
structure TwoPCSched (H : HydroSem L mem) (nC nPt : Nat) (P : Type)
    [DecidableEq P] where
  /-- `payloads.broadcast(&participants, …)`: prepare transport. -/
  prepCh : H.TransportDec nPt nC
  /-- The vote echo's `send(coordinator)` transport. -/
  voteCh : H.TransportDec nC nPt
  /-- `c_all_vote_yes.broadcast(&participants, …)`: commit transport. -/
  commitCh : H.TransportDec nPt nC
  /-- The ack echo's transport. -/
  ackCh : H.TransportDec nC nPt

/-- The trivial (all-`Unit`) scheduling record at `Values`. -/
def TwoPCSched.triv {nC nPt : Nat} {P : Type} [DecidableEq P] :
    TwoPCSched (Values L mem) nC nPt P :=
  ⟨(), (), (), ()⟩

/-- Everything the coordinators proposed: the payload input, pooled over
the (one) coordinator member. -/
def tpcProposed {nC : Nat} (payloads : Fin nC → Multiset P) : Multiset P :=
  ((List.finRange nC).map payloads).sum

/-- What `two_pc` **ensures**, over the `Values` denotation, stated on
the output `c_all_commit` and the inputs (`n` participants, their
`vote`s, the proposed `payloads`). -/
structure TPCEnsures {nC : Nat} (n : Nat) (vote : Fin n → P → Bool)
    (payloads : Fin nC → Multiset P) (out : Fin nC → Multiset P) : Prop where
  /-- **Soundness**: a committed payload was proposed. -/
  commit_proposed : ∀ c, ∀ p ∈ out c, 1 ≤ n → p ∈ tpcProposed payloads
  /-- **Unanimity** (the safety headline): a once-proposed payload that
  commits was voted yes by **every** participant — under every decision. -/
  commit_unanimous : ∀ c, ∀ p ∈ out c, 1 ≤ n →
    (tpcProposed payloads).count p = 1 → ∀ j, vote j p = true
  /-- **At-most-once**: with the one coordinator Rust has, a payload
  proposed at most once commits at most once — under every decision. -/
  commit_once : ∀ c (p : P), 1 ≤ n → nC = 1 →
    (tpcProposed payloads).count p ≤ 1 → (out c).count p ≤ 1

/-- **two_pc.rs:14–69 `two_pc`**: one Rust fn, one Lean def. Returns
`c_all_commit`. -/
hydro def two_pc (H : HydroSem L mem) (coord part : L)
    (num_participants : Nat) (vote : Fin (mem part) → P → Bool)
    (payloads : H.Stream coord P .noOrder .exactlyOnce)
    (dec : TwoPCDec H (mem coord) (mem part) P)
    (sched : TwoPCSched H (mem coord) (mem part) P) :
    H.Stream coord P .noOrder .exactlyOnce
  ensures out => num_participants = mem part →
    TPCEnsures (mem part) vote payloads out :=
  ghost intro hn
  ghost subst hn
  -- let p_prepare = payloads.broadcast(participants, …, nondet!(/** TODO */));  (two_pc.rs:25–29)
  let p_prepare := H.values (H.broadcast_closed sched.prepCh payloads)
  -- the vote hook (see the header): Rust's `c_votes.map(|kv| (kv, Ok(())))`
  -- (:41) at the coordinator becomes each participant tagging its echo
  -- with its vote; the Rust is `vote = fun _ _ => true`
  let p_votes := H.map p_prepare (fun j p => (p, voteE vote j p))
  -- let c_votes = p_prepare.send(coordinator, …).values();  (:33–37)
  let c_votes := H.values (H.broadcast_closed sched.voteCh p_votes)
  -- let (c_all_vote_yes, _) = collect_quorum(c_votes…, num_participants, num_participants);  (:40–44)
  let cq1 := collect_quorum H coord c_votes num_participants
    num_participants dec.votes
  ghost have hcq1 := collect_quorum.ensures coord c_votes (mem part)
    (mem part) dec.votes
  -- let p_commit = c_all_vote_yes.broadcast(participants, …, nondet!(/** TODO */));  (:49–53)
  let p_commit := H.values (H.broadcast_closed sched.commitCh cq1.1)
  -- the ack echo: Rust's `c_commits.map(|kv| (kv, Ok(())))` (:62), tagged
  -- at the participant as above (always `Ok` — acks are not votes)
  let p_acks := H.map p_commit
    (fun _j p => (p, (Except.ok () : Except Unit Unit)))
  -- let c_commits = p_commit.send(coordinator, …).values();  (:56–60)
  let c_acks := H.values (H.broadcast_closed sched.ackCh p_acks)
  -- let (c_all_commit, _) = collect_quorum(c_commits…, num_participants, num_participants);  (:61–65)
  let cq2 := collect_quorum H coord c_acks num_participants
    num_participants dec.acks
  ghost have hcq2 := collect_quorum.ensures coord c_acks (mem part)
    (mem part) dec.acks
  -- **the wires, read at the denotation** (`den`): every participant sees
  -- every proposal; every coordinator sees every participant's echo
  ghost have hprep : ∀ j, p_prepare j = tpcProposed payloads := fun _ => rfl
  ghost have hvotes : ∀ c, c_votes c
      = ((List.finRange (mem part)).map (fun j =>
          (tpcProposed payloads).map (fun p => (p, voteE vote j p)))).sum :=
    fun _ => rfl
  ghost have hacks : ∀ c, c_acks c
      = ((List.finRange (mem part)).map (fun _j =>
          (((List.finRange (mem coord)).map (fun c' => cq1.1 c')).sum).map
            (fun p => (p, (Except.ok () : Except Unit Unit))))).sum :=
    fun _ => rfl
  -- **the vote pool's counts**: a once-proposed payload's `Ok` votes are
  -- its yes-voters; its responses are one per participant per proposal
  ghost have hok_votes : ∀ c (p : P), (tpcProposed payloads).count p = 1 →
      cqOkCount (c_votes c) p
        = ((List.finRange (mem part)).filter (fun j => vote j p)).length :=
    fun c p hone => by
    rw [hvotes, cqOkCount_listSum, List.map_map]
    have : (List.finRange (mem part)).map
        ((fun m => cqOkCount m p) ∘ fun j =>
          (tpcProposed payloads).map (fun q => (q, voteE vote j q)))
        = (List.finRange (mem part)).map (fun j => if vote j p then 1 else 0) := by
      refine List.map_congr_left (fun j _ => ?_)
      show cqOkCount ((tpcProposed payloads).map (fun q => (q, voteE vote j q))) p = _
      rw [cqOkCount_map_tag, hone]
      by_cases hv : vote j p
      · rw [if_pos hv, if_pos (by unfold voteE; rw [if_pos hv]; rfl)]
      · rw [if_neg hv, if_neg (by unfold voteE; rw [if_neg hv]; decide)]
    rw [this]
    exact List.sum_map_indicator _ _
  ghost have hkey_votes : ∀ c (p : P),
      cqKeyCount (c_votes c) p = mem part * (tpcProposed payloads).count p :=
    fun c p => by
    rw [hvotes, cqKeyCount_listSum, List.map_map]
    have : (List.finRange (mem part)).map
        ((fun m => cqKeyCount m p) ∘ fun j =>
          (tpcProposed payloads).map (fun q => (q, voteE vote j q)))
        = (List.finRange (mem part)).map (fun _ => (tpcProposed payloads).count p) :=
      List.map_congr_left (fun j _ => cqKeyCount_map_tag _ _ p)
    rw [this, List.map_const', List.sum_replicate_nat, List.length_finRange]
  -- **the ack pool's counts**: `n` `Ok` acks per vote-yes emission
  ghost have hok_acks : ∀ c (p : P), cqOkCount (c_acks c) p
      = mem part * (((List.finRange (mem coord)).map (fun c' => cq1.1 c')).sum).count p :=
    fun c p => by
    rw [hacks, cqOkCount_listSum, List.map_map]
    have : (List.finRange (mem part)).map
        ((fun m => cqOkCount m p) ∘ fun _j =>
          (((List.finRange (mem coord)).map (fun c' => cq1.1 c')).sum).map
            (fun q => (q, (Except.ok () : Except Unit Unit))))
        = (List.finRange (mem part)).map
            (fun _ => (((List.finRange (mem coord)).map (fun c' => cq1.1 c')).sum).count p) :=
      List.map_congr_left (fun j _ => by
        show cqOkCount (Multiset.map (fun q => (q, (Except.ok () : Except Unit Unit))) _) p = _
        rw [cqOkCount_map_tag]
        rfl)
    rw [this, List.map_const', List.sum_replicate_nat, List.length_finRange]
  ghost have hkey_acks : ∀ c (p : P), cqKeyCount (c_acks c) p
      = mem part * (((List.finRange (mem coord)).map (fun c' => cq1.1 c')).sum).count p :=
    fun c p => by
    rw [hacks, cqKeyCount_listSum, List.map_map]
    have : (List.finRange (mem part)).map
        ((fun m => cqKeyCount m p) ∘ fun _j =>
          (((List.finRange (mem coord)).map (fun c' => cq1.1 c')).sum).map
            (fun q => (q, (Except.ok () : Except Unit Unit))))
        = (List.finRange (mem part)).map
            (fun _ => (((List.finRange (mem coord)).map (fun c' => cq1.1 c')).sum).count p) :=
      List.map_congr_left (fun j _ => cqKeyCount_map_tag _ _ p)
    rw [this, List.map_const', List.sum_replicate_nat, List.length_finRange]
  -- **a commit was vote-yes'd by some coordinator**: phase 2's `n` `Ok`
  -- acks among the consumed pool need at least one ack, i.e. one vote-yes
  ghost have hcommit_src : ∀ c, ∀ p ∈ cq2.1 c, 1 ≤ mem part →
      ∃ c', p ∈ cq1.1 c' := fun c p hp hn => by
    have h2 : mem part ≤ cqOkCount (c_acks c) p :=
      le_trans (hcq2.emit_sound c p hp) (cqOkCount_mono (cqConsumed_le _ _) p)
    rw [hok_acks] at h2
    have hpos : 0 < (((List.finRange (mem coord)).map (fun c' => cq1.1 c')).sum).count p := by
      rcases Nat.eq_zero_or_pos ((((List.finRange (mem coord)).map (fun c' => cq1.1 c')).sum).count p)
        with h0 | h1
      · rw [h0, Nat.mul_zero] at h2; omega
      · exact h1
    rw [Multiset.count_listSum, List.map_map] at hpos
    obtain ⟨x, hx, hxpos⟩ := List.exists_pos_of_sum_pos _ hpos
    obtain ⟨c', -, rfl⟩ := List.mem_map.mp hx
    exact ⟨c', Multiset.count_pos.mp hxpos⟩
  -- **a vote-yes needs `n` yes-votes among the consumed pool**, which sits
  -- below the whole vote pool
  ghost have hvy_votes : ∀ c', ∀ p ∈ cq1.1 c',
      mem part ≤ cqOkCount (c_votes c') p := fun c' p hp =>
    le_trans (hcq1.emit_sound c' p hp) (cqOkCount_mono (cqConsumed_le _ _) p)
  cq2.1
  prove
    commit_proposed := fun c p hp hn => by
      obtain ⟨c', hpc'⟩ := hcommit_src c p hp hn
      have h1 := le_trans (hvy_votes c' p hpc') (cqOkCount_le_keyCount _ p)
      rw [hkey_votes] at h1
      -- `n ≤ n · count p` with `1 ≤ n` forces `1 ≤ count p`
      rcases Nat.eq_zero_or_pos ((tpcProposed payloads).count p) with h0 | h1'
      · rw [h0, Nat.mul_zero] at h1; omega
      · exact Multiset.count_pos.mp h1',
    commit_unanimous := fun c p hp hn hone => by
      obtain ⟨c', hpc'⟩ := hcommit_src c p hp hn
      have h1 := hvy_votes c' p hpc'
      rw [hok_votes c' p hone] at h1
      have hlen : ((List.finRange (mem part)).filter (fun j => vote j p)).length
          = (List.finRange (mem part)).length := by
        have hub := List.length_filter_le (fun j => vote j p) (List.finRange (mem part))
        rw [List.length_finRange] at hub ⊢
        omega
      intro j
      simpa using List.length_filter_eq_length_iff.mp hlen j (List.mem_finRange j),
    commit_once := fun c p hn hc1 hone => by
      -- phase 1: each coordinator vote-yeses `p` at most once (the usage
      -- cap `cqKeyCount ≤ n` holds: `n` responses per proposal, ≤ 1 proposal)
      have hvy1 : ∀ c', (cq1.1 c').count p ≤ 1 := fun c' => by
        have hcap1 : cqKeyCount (cqConsumed (c_votes c') (dec.votes c')) p ≤ mem part := by
          refine le_trans (cqKeyCount_mono (cqConsumed_le _ _) p) ?_
          rw [hkey_votes]
          exact le_trans (Nat.mul_le_mul_left _ hone) (by omega)
        rw [hcq1.emit_count c' p hn le_rfl hcap1]
        split <;> omega
      -- the one coordinator's vote-yes pool holds `p` at most once …
      have hsum : (((List.finRange (mem coord)).map (fun c' => cq1.1 c')).sum).count p ≤ 1 := by
        rw [Multiset.count_listSum, List.map_map]
        refine le_trans (List.sum_le_length_of_le_one _ (fun x hx => ?_)) ?_
        · obtain ⟨c', -, rfl⟩ := List.mem_map.mp hx
          exact hvy1 c'
        · rw [List.length_map, List.length_finRange, hc1]
      -- … so the ack pool holds ≤ `n` responses of `p`: the phase-2 cap
      have hcap2 : cqKeyCount (cqConsumed (c_acks c) (dec.acks c)) p ≤ mem part := by
        refine le_trans (cqKeyCount_mono (cqConsumed_le _ _) p) ?_
        rw [hkey_acks]
        exact le_trans (Nat.mul_le_mul_left _ hsum) (by omega)
      rw [hcq2.emit_count c p hn le_rfl hcap2]
      split <;> omega

#nondet_census two_pc (nondets := 2) (scheds := 4) (fuels := 0)

/-! ## Executable smoke test (the `twopc` scenario, at `Values`)

One coordinator, two participants, payloads `{10, 20}`; participant 1
votes **no** on `20`; complete decisions. `10` commits (unanimous),
`20` does not, and `10` commits once. -/

private inductive TPCLoc | coord | part
  deriving DecidableEq

@[reducible] private def tpcMem : TPCLoc → Nat
  | .coord => 1
  | .part => 2

/-- Participant 1 rejects payload 20. -/
private def tpcVote : Fin 2 → Nat → Bool := fun j p => !(j.val = 1 && p = 20)

#guard (two_pc (Values TPCLoc tpcMem) .coord .part 2 tpcVote
    (fun _ => ({10, 20} : Multiset Nat))
    ⟨fun _ => [{(10, .ok ()), (20, .ok ()), (10, .ok ()), (20, .error ())}],
     fun _ => [{(10, .ok ()), (10, .ok ())}]⟩
    TwoPCSched.triv) 0
  = ({10} : Multiset Nat)

/-! ## The machine-run headlines over the generated artifacts

`two_pc_safe_sched'`/`two_pc_once_sched'`: the `cq_safe_sched'`/
`paxos_safe_sched'` pattern — the corner run's machine leg is the
`SchedSem` run (`two_pc_co_sr₁`), its denotational leg is the `Values`
run at the corner's own derived decisions (`two_pc_co_rr₁`,
`two_pc_vdec`), the coupling is premise-free (`two_pc_co_wf₁`), and the
contract face at the derived decisions does the rest. Any pacing, any
delivery schedule, any horizon; no satisfiability premise, no decision
witness. -/

section TwoPCSafe

variable (pacing : (ℓ : L) → Fin (mem ℓ) → Nat → Bool) (coord part : L)
  (T : Nat) (vote : Fin (mem part) → P → Bool)
  (cpS : (SchedSem L mem pacing).Stream coord P .noOrder .exactlyOnce)
  (cpV : (Values L mem).Stream coord P .noOrder .exactlyOnce)
  (hcp : ∀ t i, ListLe .noOrder .exactlyOnce ((cpS i).view t) (cpV i))
  (sdec : TwoPCDec (SchedSem L mem pacing) (mem coord) (mem part) P)
  (ssched : TwoPCSched (SchedSem L mem pacing) (mem coord) (mem part) P)

/-- The machine decisions, re-packaged for the corner (content sites are
the corner's own derived witnesses). -/
def twoPCCoDec :
    TwoPCDec (CoupleSem L mem pacing T T (Nat.le_refl T)) (mem coord)
      (mem part) P :=
  ⟨CoDec.batch _, CoDec.batch _⟩

/-- The machine sched-det bundle, re-packaged for the corner (cursor
data at every transport). -/
def twoPCCoSched :
    TwoPCSched (CoupleSem L mem pacing T T (Nat.le_refl T)) (mem coord)
      (mem part) P :=
  ⟨CoDec.cursors ssched.prepCh, CoDec.cursors ssched.voteCh,
   CoDec.cursors ssched.commitCh, CoDec.cursors ssched.ackCh⟩

/-- The corner-derived `Values` decision record: the generated
`two_pc_vdec` at the machine schedule's transports (`noncomputable` —
generated definitions carry no compiled code). -/
noncomputable def twoPCVDec :
    TwoPCDec (Values L mem) (mem coord) (mem part) P :=
  let csched := twoPCCoSched pacing coord part T ssched
  two_pc_vdec (pacing := pacing) (Td := T) coord part (mem part) vote cpS
    csched.prepCh csched.voteCh csched.commitCh csched.ackCh

/-- **The machine-projection identity**: the corner run's machine leg is
the `SchedSem` run (the generated `two_pc_co_sr₁`). -/
theorem two_pc_co_sr :
    (two_pc (CoupleSem L mem pacing T T (Nat.le_refl T)) coord part
        (mem part) vote (CoStream.inputC cpS cpV hcp)
        (twoPCCoDec pacing coord part T)
        (twoPCCoSched pacing coord part T ssched)).sr
      = two_pc (SchedSem L mem pacing) coord part (mem part) vote cpS
          sdec ssched :=
  two_pc_co_sr₁ coord part (mem part) vote (CoStream.inputC cpS cpV hcp)
    (twoPCCoDec pacing coord part T) (twoPCCoSched pacing coord part T ssched)

/-- **The naming identity**: the corner run's denotational leg is the
`Values` run at the corner's own derived decisions (the generated
`two_pc_co_rr₁`). -/
theorem two_pc_co_rr :
    (two_pc (CoupleSem L mem pacing T T (Nat.le_refl T)) coord part
        (mem part) vote (CoStream.inputC cpS cpV hcp)
        (twoPCCoDec pacing coord part T)
        (twoPCCoSched pacing coord part T ssched)).rr
      = two_pc (Values L mem) coord part (mem part) vote cpV
          (twoPCVDec pacing coord part T vote cpS ssched) TwoPCSched.triv :=
  two_pc_co_rr₁ coord part (mem part) vote (CoStream.inputC cpS cpV hcp)
    (twoPCCoDec pacing coord part T) (twoPCCoSched pacing coord part T ssched)

/-- **The coupling, premise-free**: the machine run's commits below any
horizon sit inside the denotational commit pool (the generated
`two_pc_co_wf₁` discharges the corner's obligation). -/
theorem two_pc_co_cpl (c : Fin (mem coord)) :
    ListLe .noOrder .exactlyOnce
      (((two_pc (CoupleSem L mem pacing T T (Nat.le_refl T)) coord part
          (mem part) vote (CoStream.inputC cpS cpV hcp)
          (twoPCCoDec pacing coord part T)
          (twoPCCoSched pacing coord part T ssched)).sr c).view T)
      ((two_pc (CoupleSem L mem pacing T T (Nat.le_refl T)) coord part
          (mem part) vote (CoStream.inputC cpS cpV hcp)
          (twoPCCoDec pacing coord part T)
          (twoPCCoSched pacing coord part T ssched)).rr c) :=
  (two_pc (CoupleSem L mem pacing T T (Nat.le_refl T)) coord part
      (mem part) vote (CoStream.inputC cpS cpV hcp)
      (twoPCCoDec pacing coord part T)
      (twoPCCoSched pacing coord part T ssched)).cpl
    (two_pc_co_wf₁ coord part (mem part) vote (CoStream.inputC cpS cpV hcp)
      (twoPCCoDec pacing coord part T) (twoPCCoSched pacing coord part T ssched)
      trivial) c

include cpV hcp in
/-- **Machine-run 2PC unanimity, premise-free**: a payload the step
machine has committed by any horizon, under any pacing and any
schedule, that was proposed once, was voted yes by every participant. -/
theorem two_pc_safe_sched' (hn : 1 ≤ mem part) (c : Fin (mem coord)) (p : P)
    (hp : p ∈ ((two_pc (SchedSem L mem pacing) coord part (mem part) vote
      cpS sdec ssched) c).view T)
    (hone : (tpcProposed cpV).count p = 1) :
    ∀ j, vote j p = true := by
  have hens := two_pc.ensures coord part (mem part) vote cpV
    (twoPCVDec pacing coord part T vote cpS ssched) TwoPCSched.triv rfl
  have hcpl := two_pc_co_cpl pacing coord part T vote cpS cpV hcp ssched c
  rw [two_pc_co_sr pacing coord part T vote cpS cpV hcp sdec ssched,
    two_pc_co_rr pacing coord part T vote cpS cpV hcp ssched] at hcpl
  exact hens.commit_unanimous c p
    (Multiset.mem_of_le hcpl (Multiset.mem_coe.mpr hp)) hn hone

include cpV hcp in
/-- **Machine-run 2PC at-most-once, premise-free**: with Rust's one
coordinator, a payload proposed at most once is committed at most once by
the step machine, by any horizon, under any pacing and any schedule. -/
theorem two_pc_once_sched' (hn : 1 ≤ mem part) (hc1 : mem coord = 1)
    (c : Fin (mem coord)) (p : P)
    (hone : (tpcProposed cpV).count p ≤ 1) :
    (((two_pc (SchedSem L mem pacing) coord part (mem part) vote
      cpS sdec ssched) c).view T).count p ≤ 1 := by
  have hens := two_pc.ensures coord part (mem part) vote cpV
    (twoPCVDec pacing coord part T vote cpS ssched) TwoPCSched.triv rfl
  have hcpl := two_pc_co_cpl pacing coord part T vote cpS cpV hcp ssched c
  rw [two_pc_co_sr pacing coord part T vote cpS cpV hcp sdec ssched,
    two_pc_co_rr pacing coord part T vote cpS cpV hcp ssched] at hcpl
  have hout := hens.commit_once c p hn hc1 hone
  rw [← Multiset.coe_count]
  exact le_trans (Multiset.count_le_of_le p hcpl) hout

end TwoPCSafe

end Hydro
