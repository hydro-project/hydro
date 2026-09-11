import HydroLean.Programs.Paxos.PLeaderHeartbeat
import HydroLean.Programs.Paxos.PP1b
import HydroLean.Programs.Paxos.AcceptorP1

/-!
# `leader_election` (paxos.rs:253–345) — module

The Rust function over the located surface, 1:1 with its signature:

```rust
fn leader_election(proposers, acceptors, proposer_tick, acceptor_tick,
    quorum_size, num_quorum_participants, paxos_config,
    p_received_p2b_ballots : Stream<Ballot, Cluster<Proposer>, NoOrder>,
    a_log : Singleton<(Option<usize>, L), Tick<Acceptor>>,
    nondet_leader, nondet_acceptor_ballot)
  -> (p_ballot : Singleton<Ballot, Tick<Proposer>>,
      p_is_leader : Singleton<bool, Tick<Proposer>>,
      p_relevant_p1bs : Stream<(Option<usize>, L), Tick<Proposer>, NoOrder>,
      a_max_ballot : Singleton<Option<Ballot>, Tick<Acceptor>>)
```

Located carriers: cluster streams are member-indexed families, tick values
are `TSing`/`TStream` across the whole execution (`Hydro/TStream.lean`);
`NoOrder` inputs appear in keyed (per-sender family) form and their batch
and snapshot `nondet!`s carry the consumed increments themselves. The two
`NonDet` parameters of the Rust signature are the decision bundle
`LENondet`; the three internal `forward_ref`s (paxos.rs:271–276:
`p1b_fail`, `p_to_proposers_i_am_leader`, `p_is_leader`) are one
`forward_ref` over the product cycle `LERef`.

The **within-tick `a_log` knot** (paxos.rs:165–166): `a_log` is a function
input here (per acceptor, per tick), completed by `sequence_payload`'s
output through `paxos_core`'s `forward_ref` — `acceptor_p1`'s replies at
tick `t` block until `a_log` has tick `t`, which at every unfolding depth is
the *final* post-merge value of that tick (realized ticks are stable), i.e.
the `snapshot_atomic` write-before-ack guarantee.
-/

namespace HydroLean.Programs.Paxos

open HydroLean.Hydro

set_option synthInstance.maxSize 1024

variable {P : Type} {nP nA : Nat}

/-! ## The guarded P1a send-once filter (FINDINGS.md B1)

`dedupLast` is the stream form of the `lastP1aSent` guard: drop a send iff
it equals the last ballot actually sent. The faithful variant is the
identity (paxos.rs as written re-broadcasts on every trigger). -/

/-- Drop elements equal to the last emitted one. -/
def dedupLast (last : Option (Ballot nP)) : List (Ballot nP) → List (Ballot nP)
  | [] => []
  | b :: bs => if some b = last then dedupLast last bs
      else b :: dedupLast (some b) bs

/-- The B1 guard: send a P1a at most once per ballot. -/
def sendGuard (variant : PaxosVariant) (s : Stream (Ballot nP)) :
    Stream (Ballot nP) :=
  if variant.p1aSendOnce then dedupLast none s else s

theorem dedupLast_sublist (last : Option (Ballot nP))
    (s : List (Ballot nP)) : (dedupLast last s).Sublist s := by
  induction s generalizing last with
  | nil => exact List.Sublist.refl _
  | cons b bs ih =>
    unfold dedupLast
    by_cases h : some b = last
    · rw [if_pos h]
      exact (ih last).cons _
    · rw [if_neg h]
      exact (ih (some b)).cons₂ _

theorem sendGuard_sublist (variant : PaxosVariant) (s : Stream (Ballot nP)) :
    (sendGuard variant s).Sublist s := by
  unfold sendGuard
  by_cases h : variant.p1aSendOnce
  · rw [if_pos h]
    exact dedupLast_sublist none s
  · rw [if_neg h]
    exact List.Sublist.refl _

theorem dedupLast_prefix {s s' : List (Ballot nP)} (h : s <+: s')
    (last : Option (Ballot nP)) :
    dedupLast last s <+: dedupLast last s' := by
  induction s generalizing s' last with
  | nil => exact List.nil_prefix
  | cons b bs ih =>
    obtain ⟨u, rfl⟩ := h
    show dedupLast last (b :: bs) <+: dedupLast last (b :: (bs ++ u))
    unfold dedupLast
    by_cases hb : some b = last
    · rw [if_pos hb, if_pos hb]
      exact ih (List.prefix_append _ _) last
    · rw [if_neg hb, if_neg hb]
      exact List.cons_prefix_cons.mpr ⟨rfl, ih (List.prefix_append _ _) _⟩

theorem sendGuard_prefix (variant : PaxosVariant)
    {s s' : Stream (Ballot nP)} (h : s <+: s') :
    sendGuard variant s <+: sendGuard variant s' := by
  unfold sendGuard
  by_cases hv : variant.p1aSendOnce
  · rw [if_pos hv, if_pos hv]
    exact dedupLast_prefix h none
  · rw [if_neg hv, if_neg hv]
    exact h

/-- The B1 guard as a stage (its leaf monotonicity is `dedupLast_prefix`). -/
def sendGuardM (variant : PaxosVariant) :
    Stream (Ballot nP) →ₘ Stream (Ballot nP) :=
  ⟨sendGuard variant, fun h => sendGuard_prefix variant h⟩

/-! ## Decisions and the cycle carrier -/

/-- The materialized `nondet!` decisions of `leader_election`
(`nondet_leader` + `nondet_acceptor_ballot`), per member. -/
structure LENondet (P : Type) (nP nA : Nat) where
  /-- Per-proposer arrival increments of the `p_received_max_ballot`
  snapshot (paxos.rs:287–294; the merged `NoOrder + AtLeastOnce` fan-in of
  fail ballots and heartbeats — membership-legal, `max` absorbs
  duplication). -/
  maxSnap : Fin nP → List (List (Ballot nP))
  /-- Per-proposer heartbeat timing (paxos.rs:414–482). -/
  heartbeat : Fin nP → HeartbeatNondet
  /-- Per-proposer `p_p1b` decisions (quorum-slice batches + view snapshot;
  paxos.rs:545–572). -/
  p1b : Fin nP → PP1bNondet P nP
  /-- Per-acceptor consumed P1a batches (`nondet_acceptor_ballot`,
  paxos.rs:315–323; `NoOrder` ⇒ the decision is the batch). -/
  p1aBatch : Fin nA → List (List (Ballot nP))
  /-- `forward_ref` unfolding depth (an adversarial decision like any
  other). -/
  fuel : Nat

/-- The internal `forward_ref` cycle of `leader_election`
(paxos.rs:271–276), as one product carrier. -/
structure LERef (P : Type) (nP nA : Nat) where
  /-- `p1b_fail` (:271): per proposer, per replying acceptor. -/
  p1b_fail : Fin nP → Fin nA → Stream (Ballot nP)
  /-- `p_to_proposers_i_am_leader` (:272–273): per *sender*. -/
  i_am_leader : Fin nP → Stream (Ballot nP)
  /-- `p_is_leader` (:274–276): the tick-level forward ref. -/
  p_is_leader : Fin nP → TSing Bool

/-- The empty cycle history. -/
def LERef.init : LERef P nP nA :=
  ⟨fun _ _ => [], fun _ => [], fun _ => []⟩

/-- Pointwise prefix order on the cycle (the growth relation). -/
structure LERef.le (r r' : LERef P nP nA) : Prop where
  p1b_fail : ∀ i j, r.p1b_fail i j <+: r'.p1b_fail i j
  i_am_leader : ∀ i, r.i_am_leader i <+: r'.i_am_leader i
  p_is_leader : ∀ i, r.p_is_leader i <+: r'.p_is_leader i

theorem LERef.le_refl (r : LERef P nP nA) : r.le r :=
  ⟨fun _ _ => List.prefix_refl _, fun _ => List.prefix_refl _,
   fun _ => List.prefix_refl _⟩

theorem LERef.le_trans {a b c : LERef P nP nA} (h1 : a.le b) (h2 : b.le c) :
    a.le c :=
  ⟨fun i j => (h1.p1b_fail i j).trans (h2.p1b_fail i j),
   fun i => (h1.i_am_leader i).trans (h2.i_am_leader i),
   fun i => (h1.p_is_leader i).trans (h2.p_is_leader i)⟩

/-- The empty cycle history is bottom (the Kleene chain's base). -/
theorem LERef.init_bot (r : LERef P nP nA) : LERef.init.le r :=
  ⟨fun _ _ => List.nil_prefix, fun _ => List.nil_prefix,
   fun _ => List.nil_prefix⟩

/-- The cycle carrier grows by `LERef.le`. -/
instance : Growth (LERef P nP nA) where
  le := LERef.le
  le_refl := LERef.le_refl
  le_trans := LERef.le_trans

/-- The growth carrier of `leader_election`'s dataflow: its function inputs
(`p_received_p2b_ballots`, `a_log`) and its cycle. -/
abbrev LEG (P : Type) (nP nA : Nat) : Type :=
  (Fin nP → Fin nA → Stream (Ballot nP))
    × (Fin nA → TSing (Option Nat × LogMap P nP)) × LERef P nP nA

/-- Cycle-field projections, as stages. -/
def LERef.p1b_failM :
    LERef P nP nA →ₘ (Fin nP → Fin nA → Stream (Ballot nP)) :=
  ⟨LERef.p1b_fail, fun h => h.p1b_fail⟩

def LERef.i_am_leaderM : LERef P nP nA →ₘ (Fin nP → Stream (Ballot nP)) :=
  ⟨LERef.i_am_leader, fun h => h.i_am_leader⟩

def LERef.p_is_leaderM : LERef P nP nA →ₘ (Fin nP → TSing Bool) :=
  ⟨LERef.p_is_leader, fun h => h.p_is_leader⟩

/-- Cycle construction, as a stage. -/
def LERef.mkM :
    (Fin nP → Fin nA → Stream (Ballot nP)) × (Fin nP → Stream (Ballot nP))
        × (Fin nP → TSing Bool) →ₘ LERef P nP nA :=
  ⟨fun x => ⟨x.1, x.2.1, x.2.2⟩, fun h => ⟨h.1, h.2.1, h.2.2⟩⟩

/-! ## The stages (the Rust `let`s, one def per binding) -/

section Stages

variable (variant : PaxosVariant) (qs nqp : Nat)
variable (nd : LENondet P nP nA)

/-! The module's input wires (the growth-carrier fields, named once). -/

/-- `p_received_p2b_ballots` (function input). -/
def leSq : LEG P nP nA →ₘ (Fin nP → Fin nA → Stream (Ballot nP)) :=
  MonoMap.fst

/-- `a_log` (function input). -/
def leAl : LEG P nP nA →ₘ (Fin nA → TSing (Option Nat × LogMap P nP)) :=
  MonoMap.fst ∘ₘ MonoMap.snd

/-- The internal cycle. -/
def leCyc : LEG P nP nA →ₘ LERef P nP nA := MonoMap.snd ∘ₘ MonoMap.snd

/-- `p1b_fail` (cycle wire). -/
def leP1bFail : LEG P nP nA →ₘ (Fin nP → Fin nA → Stream (Ballot nP)) :=
  LERef.p1b_failM ∘ₘ leCyc

/-- `p_to_proposers_i_am_leader` (cycle wire). -/
def leIAmLeader : LEG P nP nA →ₘ (Fin nP → Stream (Ballot nP)) :=
  LERef.i_am_leaderM ∘ₘ leCyc

/-- `p_is_leader` (cycle wire). -/
def leIsLeader : LEG P nP nA →ₘ (Fin nP → TSing Bool) :=
  LERef.p_is_leaderM ∘ₘ leCyc

/-- What the `p_received_max_ballot` merge can see at proposer `i`
(paxos.rs:281–285): `p1b_fail.merge_unordered(p_received_p2b_ballots)
.merge_unordered(i_am_leader)` — the `NoOrder + AtLeastOnce` union
availability. -/
def leReceivedAvailM (i : Fin nP) : LEG P nP nA →ₘ Mem (Ballot nP) :=
  (((leP1bFail.member i).unionFMem).mergeMem
    ((leSq.member i).unionFMem)).mergeMem
    (leIAmLeader.unionFMem)

/-- `.max().into_singleton().snapshot(proposer_tick, nondet_leader)`
(paxos.rs:285–294): the running max read through the `NoOrder` snapshot
(views = accumulated arrival increments; `AtLeastOnce` legality — `max` is
commutative and idempotent, so arrival order and re-delivery are absorbed). -/
def le_p_received_max_ballotM (i : Fin nP) :
    LEG P nP nA →ₘ TSing (Option (Ballot nP)) :=
  ((leReceivedAvailM i).snapshotD (nd.maxSnap i)).map
    (fun view => Ballot.maxList none view)

/-- `p_ballot_calc` (paxos.rs:296–304); the ballot leg is the `Monotonic`
wire. -/
def le_p_ballotM (i : Fin nP) :
    LEG P nP nA →ₘ MonoSing (ballotNumVO (nP := nP)) :=
  MonoMap.fst ∘ₘ p_ballot_calcM i ∘ₘ le_p_received_max_ballotM nd i

def le_p_has_largest_ballotM (i : Fin nP) : LEG P nP nA →ₘ TSing Bool :=
  MonoMap.snd ∘ₘ p_ballot_calcM i ∘ₘ le_p_received_max_ballotM nd i

/-- `p_leader_heartbeat` (paxos.rs:306–309). -/
def le_heartbeatM (i : Fin nP) :
    LEG P nP nA →ₘ Stream (Ballot nP) × TSing Bool :=
  p_leader_heartbeatM (nd.heartbeat i) ∘ₘ MonoMap.pair
    (leIsLeader.member i) ((le_p_ballotM nd i).vals)

/-- `p_to_acceptors_p1a` (paxos.rs:311–317):
`p_ballot.filter_if(p_trigger_election).all_ticks().broadcast(…)` — plus the
guarded variant's send-once filter (B1). Broadcast is the identity: each
acceptor reads this per-sender stream. -/
def le_p_to_acceptors_p1aM (i : Fin nP) : LEG P nP nA →ₘ Stream (Ballot nP) :=
  sendGuardM variant ∘ₘ
    (((le_p_ballotM nd i).vals.zipWith (MonoMap.snd ∘ₘ le_heartbeatM nd i)
      (fun x b => if b then [x] else [])).flatten)

variable [DecidableEq P]

/-- `acceptor_p1` (paxos.rs:319–331) at acceptor `j`, over the
`nondet_acceptor_ballot` batches of the P1a fan-in. -/
def le_acceptor_p1M (j : Fin nA) :
    LEG P nP nA
      →ₘ MonoSing (obtVO (nP := nP)) × TStream (Fin nP × P1b P nP) :=
  acceptor_p1_ticksM ∘ₘ MonoMap.pair
    (((MonoMap.pi (fun i => le_p_to_acceptors_p1aM variant nd i)).unionF).batchC
      (nd.p1aBatch j))
    (leAl.member j)

/-- `a_to_proposers_p1b` (paxos.rs:329: `.demux(proposers).values()`), in
keyed (per-acceptor) form at proposer `i`. -/
def le_a_to_proposers_p1bM (i : Fin nP) (j : Fin nA) :
    LEG P nP nA →ₘ
      Stream (Ballot nP × Except (Option (Ballot nP)) (P1bPayload P nP)) :=
  ((MonoMap.snd ∘ₘ le_acceptor_p1M variant nd j).flatten).filterMap
    (fun dm => if dm.1 = i then some (dm.2.ballot, dm.2.res) else none)

/-- `p_p1b` (paxos.rs:333–341). -/
def le_pp1bM (i : Fin nP) :
    LEG P nP nA →ₘ TSing Bool × TStream (P1bPayload P nP)
      × (Fin nA → Stream (Ballot nP)) :=
  p_p1bM qs nqp (nd.p1b i) ∘ₘ MonoMap.pair
    (MonoMap.pi (fun j => le_a_to_proposers_p1bM variant nd i j))
    (MonoMap.pair ((le_p_ballotM nd i).vals)
      (le_p_has_largest_ballotM nd i))

/-- One unfolding of the `leader_election` body: consume the cycle, return
(the completed cycle, the Rust return tuple). Families are memoized at the
boundary (`memoF`, semantically the identity) so fixpoint iteration stays
linear. Cycle preservation and output growth are its `.mono`, by
construction. -/
def leader_election_bodyM :
    LEG P nP nA →ₘ LERef P nP nA ×
      ((Fin nP → MonoSing (ballotNumVO (nP := nP)))
        × (Fin nP → TSing Bool)
        × (Fin nP → TStream (P1bPayload P nP))
        × (Fin nA → MonoSing (obtVO (nP := nP)))) :=
  MonoMap.pair
    (-- p1b_fail_complete.complete(fail_ballots) (:342);
     -- p_to_proposers_i_am_leader_complete_cycle.complete(…) (:309);
     -- p_is_leader_complete_cycle.complete(p_is_leader) (:341)
     LERef.mkM ∘ₘ MonoMap.pair
      (memoFM ∘ₘ MonoMap.pi (fun i => memoFM ∘ₘ MonoMap.pi (fun j =>
        MonoMap.proj j ∘ₘ MonoMap.snd ∘ₘ MonoMap.snd
          ∘ₘ le_pp1bM variant qs nqp nd i)))
      (MonoMap.pair
        (memoFM ∘ₘ MonoMap.pi (fun i => MonoMap.fst ∘ₘ le_heartbeatM nd i))
        (memoFM ∘ₘ MonoMap.pi (fun i =>
          MonoMap.fst ∘ₘ le_pp1bM variant qs nqp nd i))))
    (MonoMap.pair
      (memoFM ∘ₘ MonoMap.pi (fun i => le_p_ballotM nd i))
      (MonoMap.pair
        (memoFM ∘ₘ MonoMap.pi (fun i =>
          MonoMap.fst ∘ₘ le_pp1bM variant qs nqp nd i))
        (MonoMap.pair
          (memoFM ∘ₘ MonoMap.pi (fun i =>
            MonoMap.fst ∘ₘ MonoMap.snd ∘ₘ le_pp1bM variant qs nqp nd i))
          (memoFM ∘ₘ MonoMap.pi (fun j =>
            MonoMap.fst ∘ₘ le_acceptor_p1M variant nd j)))))

end Stages

/-- The closed `leader_election` fixpoint as a typed stage in its function
inputs — `MonoMap.fix`: growth along the unfolding and monotonicity in the
inputs are **combinator faces** (`fixHist_chain`/`fixHist_rel`), never
re-proven per program. -/
def leader_electionM [DecidableEq P] (variant : PaxosVariant)
    (qs nqp : Nat) (nd : LENondet P nP nA) :
    (Fin nP → Fin nA → Stream (Ballot nP))
        × (Fin nA → TSing (Option Nat × LogMap P nP))
      →ₘ (Fin nP → MonoSing (ballotNumVO (nP := nP)))
        × (Fin nP → TSing Bool)
        × (Fin nP → TStream (P1bPayload P nP))
        × (Fin nA → MonoSing (obtVO (nP := nP))) :=
  MonoMap.fix nd.fuel LERef.init
    (leader_election_bodyM variant qs nqp nd ∘ₘ MonoMap.assocR)


/-! ## The P1a stream: ownership and (guarded) send-once -/

/-- `dedupLast` of a num-sorted, single-owner ballot stream is strictly
increasing in `num` — hence duplicate-free. -/
theorem dedupLast_num_lt {me : Fin nP} :
    ∀ (l : List (Ballot nP)) (last : Option (Ballot nP)),
    (∀ x ∈ l, x.proposerId = me) →
    l.Pairwise (fun a b => a.num ≤ b.num) →
    (∀ a, last = some a → a.proposerId = me ∧ ∀ x ∈ l, a.num ≤ x.num) →
    (dedupLast last l).Pairwise (fun a b => a.num < b.num) ∧
    (∀ a, last = some a → ∀ y ∈ dedupLast last l, a.num < y.num)
  | [], _, _, _, _ => ⟨List.Pairwise.nil, fun _ _ _ h => nomatch h⟩
  | b :: bs, last, hown, hsort, hlast => by
    unfold dedupLast
    have hbs_own : ∀ x ∈ bs, x.proposerId = me :=
      fun x hx => hown x (List.mem_cons_of_mem _ hx)
    have hbs_sort : bs.Pairwise (fun a b => a.num ≤ b.num) :=
      hsort.of_cons
    by_cases hb : some b = last
    · rw [if_pos hb]
      refine dedupLast_num_lt bs last hbs_own hbs_sort ?_
      intro a ha
      have hba : b = a := by
        rw [ha] at hb
        exact Option.some.inj hb
      subst hba
      exact ⟨hown b (List.mem_cons_self ..),
        fun x hx => (List.pairwise_cons.mp hsort).1 x hx⟩
    · rw [if_neg hb]
      have hrec := dedupLast_num_lt bs (some b) hbs_own hbs_sort
        (fun a ha => by
          cases Option.some.inj ha
          exact ⟨hown b (List.mem_cons_self ..),
            fun x hx => (List.pairwise_cons.mp hsort).1 x hx⟩)
      constructor
      · refine List.pairwise_cons.mpr ⟨?_, hrec.1⟩
        intro y hy
        exact hrec.2 b rfl y hy
      · intro a ha y hy
        rcases List.mem_cons.mp hy with rfl | hy'
        · -- y = b: last = some a with a.num ≤ b.num and a ≠ b (same owner)
          obtain ⟨haown, hale⟩ := hlast a ha
          have hne : a ≠ y := by
            intro heq
            exact hb (by rw [ha, heq])
          have hnum : a.num ≠ y.num := fun heq =>
            hne (Ballot.ext' heq (by
              rw [haown, hown y (List.mem_cons_self ..)]))
          have := hale y (List.mem_cons_self ..)
          omega
        · obtain ⟨-, hale⟩ := hlast a ha
          have hab := hale b (List.mem_cons_self ..)
          exact Nat.lt_of_le_of_lt hab (hrec.2 b rfl y hy')

/-- Ballot ownership of the P1a broadcast (any variant) — a face of the
`le_p_to_acceptors_p1aM` wire at any inputs. -/
theorem le_p1a_own {variant : PaxosVariant}
    {sq : Fin nP → Fin nA → Stream (Ballot nP)}
    {al : Fin nA → TSing (Option Nat × LogMap P nP)}
    {nd : LENondet P nP nA} {r : LERef P nP nA} (i : Fin nP) :
    ∀ b ∈ (le_p_to_acceptors_p1aM variant nd i).f (sq, al, r),
      (b : Ballot nP).proposerId = i := by
  intro b hb
  have hb' := (sendGuard_sublist variant _).subset hb
  have hb'' := (TSing.filterIf_flatten_sublist _ _).subset hb'
  exact p_ballot_calc_own i _ b hb''

/-- **Send-once (the B1 fix)**: the guarded P1a broadcast is
duplicate-free. The cross-tick sortedness input is a projection of the
`p_ballot` wire's `Monotonic` type (`.ascending`) — no fold reasoning. -/
theorem le_p1a_nodup {sq : Fin nP → Fin nA → Stream (Ballot nP)}
    {al : Fin nA → TSing (Option Nat × LogMap P nP)}
    {nd : LENondet P nP nA} {r : LERef P nP nA} (i : Fin nP) :
    ((le_p_to_acceptors_p1aM PaxosVariant.guarded nd i).f (sq, al, r)).Nodup := by
  show (sendGuard PaxosVariant.guarded
    (TStream.allTicks (TSing.filterIf
      (((le_p_ballotM nd i).f (sq, al, r)).vals)
      (((le_heartbeatM nd i).f (sq, al, r)).2)))).Nodup
  unfold sendGuard
  rw [if_pos (show PaxosVariant.guarded.p1aSendOnce = true from rfl)]
  have hsub := TSing.filterIf_flatten_sublist
    (((le_p_ballotM nd i).f (sq, al, r)).vals)
    (((le_heartbeatM nd i).f (sq, al, r)).2)
  have hown : ∀ b ∈ TStream.allTicks
      (TSing.filterIf (((le_p_ballotM nd i).f (sq, al, r)).vals)
        (((le_heartbeatM nd i).f (sq, al, r)).2)),
      (b : Ballot nP).proposerId = i :=
    fun b hb => p_ballot_calc_own i _ b (hsub.subset hb)
  have hsort : (TStream.allTicks
      (TSing.filterIf (((le_p_ballotM nd i).f (sq, al, r)).vals)
        (((le_heartbeatM nd i).f (sq, al, r)).2))).Pairwise
      (fun a b => a.num ≤ b.num) := by
    refine List.Pairwise.sublist hsub ?_
    rw [List.pairwise_iff_getElem]
    intro t t' ht ht' hlt
    exact ((le_p_ballotM nd i).f (sq, al, r)).ascending
      (Nat.le_of_lt hlt) ht'
  have h := dedupLast_num_lt (me := i) _ none hown hsort
    (fun a ha => nomatch ha)
  exact (h.1.imp (fun {a b} hlt => fun heq => by
    rw [heq] at hlt
    omega))

/-! ## Solicitation faces (the fabricated-reign regress's module inputs)

The two facts that stage solicitation across the `forward_ref` cycle:
every broadcast P1a was released at a trigger-true tick of the sender's own
ballot wire (`le_p1a_elim`), and a trigger can only fire at a tick whose
*cycle-input* leader flag is false (`le_trigger_gate` — the heartbeat
module's gate, instantiated at the cycle wire). Together with the acceptor
echo face they derive `leader_ballot_stable` at the run: a standing leader
cannot have solicited its usurper ballot. -/

/-- **Solicitation elimination**: every P1a on the broadcast wire was
released at a tick where the sender's ballot was that P1a's ballot and the
election trigger fired (any variant, any inputs). -/
theorem le_p1a_elim {variant : PaxosVariant}
    {sq : Fin nP → Fin nA → Stream (Ballot nP)}
    {al : Fin nA → TSing (Option Nat × LogMap P nP)}
    {nd : LENondet P nP nA} {r : LERef P nP nA} (i : Fin nP)
    {b : Ballot nP}
    (hb : b ∈ (le_p_to_acceptors_p1aM variant nd i).f (sq, al, r)) :
    ∃ (u : Nat)
      (hu : u < (((le_p_ballotM nd i).f (sq, al, r)).vals).length)
      (hg : u < (((le_heartbeatM nd i).f (sq, al, r)).2).length),
      (((le_p_ballotM nd i).f (sq, al, r)).vals)[u]'hu = b ∧
      (((le_heartbeatM nd i).f (sq, al, r)).2)[u]'hg = true := by
  have hb' : b ∈ TStream.allTicks
      (TSing.filterIf (((le_p_ballotM nd i).f (sq, al, r)).vals)
        (((le_heartbeatM nd i).f (sq, al, r)).2)) :=
    (sendGuard_sublist variant _).subset hb
  exact TSing.filterIf_flatten_elim hb'

/-- **The trigger gate at the cycle wire**: a trigger-true tick reads a
`false` leader flag off the `forward_ref` cycle input (`p_is_leader`
arrives from the *previous* iterate — paxos.rs:449 through :274–276). -/
theorem le_trigger_gate {sq : Fin nP → Fin nA → Stream (Ballot nP)}
    {al : Fin nA → TSing (Option Nat × LogMap P nP)}
    {nd : LENondet P nP nA} {r : LERef P nP nA} (i : Fin nP) {u : Nat}
    (hg : u < (((le_heartbeatM nd i).f (sq, al, r)).2).length)
    (ht : (((le_heartbeatM nd i).f (sq, al, r)).2)[u]'hg = true) :
    ∃ hf : u < (r.p_is_leader i).length,
      (r.p_is_leader i)[u]'hf = false :=
  p_leader_heartbeat_trigger_gate (r.p_is_leader i)
    (((le_p_ballotM nd i).f (sq, al, r)).vals) (nd.heartbeat i) hg ht

/-! ## The module contract at the run

The devices below (`leRun`, `leB1`, …, `lePv`) reconstruct the election's
internal traffic at given function inputs `(sq, al)`. They are **proof
devices private to this module** — consumers see only the contracts
(`le_out_ballot_own`, `le_ballot_stable`, `le_leader_view_promise`,
`le_leader_providers`, `le_view_pinned`), stated on the module's
**signature**: the `leader_electionM` output tuple (`leOut`) and the
`a_log` input wire. Promise witnesses are existential acceptor-tick
indices into the signature wires — internal traffic never escapes. -/

section RunContract

variable (variant : PaxosVariant) (qs nqp : Nat) (nd : LENondet P nP nA)
variable [DecidableEq P]
variable (sq : Fin nP → Fin nA → Stream (Ballot nP))
variable (al : Fin nA → TSing (Option Nat × LogMap P nP))

/-- The internal `forward_ref` fixpoint history at inputs `(sq, al)`
(device). -/
def leRun : LERef P nP nA :=
  MonoMap.fixHist LERef.init
    (leader_election_bodyM variant qs nqp nd ∘ₘ MonoMap.assocR)
    (sq, al) nd.fuel

/-- The carrier at the run (device). -/
def leG : LEG P nP nA := (sq, al, leRun variant qs nqp nd sq al)

/-- The module's output tuple at inputs `(sq, al)` — the **signature
object** the run contracts are stated on (definitionally the
`leader_electionM` application; `paxos_core` consumes it as `pcLE`). -/
def leOut :=
  (leader_electionM variant qs nqp nd).f (sq, al)

/-- The output is the body at the internal fixpoint history. -/
theorem leOut_eq :
    leOut variant qs nqp nd sq al
      = ((leader_election_bodyM variant qs nqp nd).f
          (sq, al, leRun variant qs nqp nd sq al)).2 := rfl

/-- Acceptor `j`'s consumed P1a tick batches (device). -/
def leB1 (j : Fin nA) : TStream (Ballot nP) :=
  batchC (unionF (fun i => (le_p_to_acceptors_p1aM variant nd i).f
    (leG variant qs nqp nd sq al))) [] (nd.p1aBatch j)

/-- Acceptor `j`'s cumulative p1b reply stream (device — the
`acceptor_p1_ticksM` stage output at `j`'s fan-in). -/
def leP1Bs (j : Fin nA) : Stream (Fin nP × P1b P nP) :=
  ((acceptor_p1_ticksM.f (leB1 variant qs nqp nd sq al j, al j)).2).flatten

/-- The p1b reply slice handed to proposer `i` (device). -/
def leRs (i : Fin nP) (j : Fin nA) :
    Stream (Ballot nP × Except (Option (Ballot nP)) (P1bPayload P nP)) :=
  (le_a_to_proposers_p1bM variant nd i j).f (leG variant qs nqp nd sq al)

/-- Proposer `i`'s ballot wire (device). -/
def lePb (i : Fin nP) : MonoSing (ballotNumVO (nP := nP)) :=
  (le_p_ballotM nd i).f (leG variant qs nqp nd sq al)

/-- Proposer `i`'s has-largest wire (device). -/
def leGl (i : Fin nP) : TSing Bool :=
  (le_p_has_largest_ballotM nd i).f (leG variant qs nqp nd sq al)

/-- Proposer `i`'s per-tick (quorum view, leader flag) trace (device). -/
def lePv (i : Fin nP) : TSing (Option (List (P1bPayload P nP)) × Bool) :=
  pP1bPv (leRs variant qs nqp nd sq al i)
    (lePb variant qs nqp nd sq al i).vals
    (leGl variant qs nqp nd sq al i) qs nqp (nd.p1b i)

/-- The `a_max_ballot` output is `acceptor_p1_ticksM`'s max wire at `j`'s
fan-in (the `memoF` family boundary, opened). -/
theorem leOut_max_eq (j : Fin nA) :
    (leOut variant qs nqp nd sq al).2.2.2 j
      = (acceptor_p1_ticksM.f
          (leB1 variant qs nqp nd sq al j, al j)).1 := by
  rw [leOut_eq]
  show memoF _ j = _
  rw [memoF_eq]
  rfl

/-- The ballot output is the ballot-wire device (handoff). -/
theorem leOut_ballot_eq (i : Fin nP) :
    (leOut variant qs nqp nd sq al).1 i = lePb variant qs nqp nd sq al i := by
  rw [leOut_eq]
  show memoF _ i = _
  rw [memoF_eq]
  rfl

/-- The leader flag output is the `pv` projection (handoff). -/
theorem leOut_leader_eq (i : Fin nP) :
    (leOut variant qs nqp nd sq al).2.1 i
      = (lePv variant qs nqp nd sq al i).map (·.2) := by
  rw [leOut_eq]
  show memoF _ i = _
  rw [memoF_eq]
  rfl

/-- The quorum-view output is the `pv` projection (handoff). -/
theorem leOut_qlogs_eq (i : Fin nP) :
    (leOut variant qs nqp nd sq al).2.2.1 i
      = (lePv variant qs nqp nd sq al i).map (fun x => x.1.getD []) := by
  rw [leOut_eq]
  show memoF _ i = _
  rw [memoF_eq]
  rfl

/-! ### The phase-1 fan-in discipline (devices) -/

/-- **The fan-in is duplicate-free** (guarded send-once B1 +
ballot-ownership cross-proposer disjointness, through the batch
legality). -/
theorem leB1_flatten_nodup (j : Fin nA) :
    ((leB1 PaxosVariant.guarded qs nqp nd sq al j).flatten).Nodup := by
  refine nodup_of_count_le_one (fun b => ?_)
  have h2 : ((leB1 PaxosVariant.guarded qs nqp nd sq al j).flatten).count b
      ≤ (unionF (fun i =>
          (le_p_to_acceptors_p1aM PaxosVariant.guarded nd i).f
            (leG PaxosVariant.guarded qs nqp nd sq al))).count b :=
    batchC_count_le _ _ b
  have h3 : (unionF (fun i =>
        (le_p_to_acceptors_p1aM PaxosVariant.guarded nd i).f
          (leG PaxosVariant.guarded qs nqp nd sq al))).count b ≤ 1 := by
    rw [unionF_count]
    refine sum_map_le_single _ b.proposerId 1 (fun i hi => ?_) ?_
    · rw [List.count_eq_zero]
      intro hmem
      exact hi (le_p1a_own i b hmem).symm
    · exact count_le_one_of_nodup (le_p1a_nodup b.proposerId) b
  exact Nat.le_trans h2 h3

/-- **p1b reply cap** — at most one `Ok` promise per ballot per acceptor,
decoded per proposer (guarded; `acceptor_p1`'s decode-cap contract at the
duplicate-free fan-in). -/
theorem leRs_cap (i : Fin nP) (j : Fin nA) (b : Ballot nP) :
    (leRs PaxosVariant.guarded qs nqp nd sq al i j).countP
      (fun e => decide (e.1 = b) && e.2.isOk) ≤ 1 :=
  ap1t_decode_cap (leB1_flatten_nodup qs nqp nd sq al j) i b

/-- **Decode opening**: an `Ok` entry of the demuxed p1b slice is an `Ok`
promise of the acceptor's raw reply stream, addressed to this proposer. -/
theorem leRs_ok_elim (i : Fin nP) (j : Fin nA)
    {b : Ballot nP} {v : P1bPayload P nP}
    (hin : (b, Except.ok v) ∈ leRs variant qs nqp nd sq al i j) :
    ∃ dm ∈ leP1Bs variant qs nqp nd sq al j,
      dm.1 = i ∧ dm.2.ballot = b ∧ dm.2.res = .ok v := by
  have hin' : (b, Except.ok v)
      ∈ (leP1Bs variant qs nqp nd sq al j).filterMap
        (fun dm => if dm.1 = i then
          some (dm.2.ballot, dm.2.res) else none) := hin
  obtain ⟨dm, hdm, hdmeq⟩ := List.mem_filterMap.mp hin'
  have hdmi : dm.1 = i := by
    by_cases hi : dm.1 = i
    · exact hi
    · rw [if_neg hi] at hdmeq
      cases hdmeq
  rw [if_pos hdmi] at hdmeq
  have hpair := Option.some.inj hdmeq
  exact ⟨dm, hdm, hdmi, congrArg Prod.fst hpair, congrArg Prod.snd hpair⟩

/-- The has-largest wire is identically `true` at realized ticks
(`p_ballot_calc_hasLargest` at the run). -/
theorem leGl_true (i : Fin nP) :
    ∀ x ∈ leGl variant qs nqp nd sq al i, x = true :=
  p_ballot_calc_hasLargest i _

/-- The cycle's flag input is a prefix of the run's flag wire: the
trigger's flag reads are final across the `forward_ref` unfolding. -/
theorem leRun_flag_prefix (i : Fin nP) :
    (leRun variant qs nqp nd sq al).p_is_leader i
      <+: (lePv variant qs nqp nd sq al i).map (·.2) := by
  have hchain : ((leRun variant qs nqp nd sq al) : LERef P nP nA).le
      (MonoMap.fixHist LERef.init
        (leader_election_bodyM variant qs nqp nd ∘ₘ MonoMap.assocR)
        (sq, al) (nd.fuel + 1)) :=
    MonoMap.fixHist_chain (fun x => LERef.init_bot x) _ _
      (Nat.le_succ nd.fuel)
  have hp := hchain.p_is_leader i
  rw [← leOut_leader_eq variant qs nqp nd sq al i]
  exact hp

/-- **Solicitation**: every `Ok`-promised ballot of proposer `i`'s reply
slice was solicited — some realized tick of `i`'s wires carries that ballot
with the leader flag **false** (the trigger gate read through the cycle
prefix). -/
theorem leRs_ok_solicited (i : Fin nP) (j : Fin nA)
    {b : Ballot nP} {v : P1bPayload P nP}
    (hin : (b, Except.ok v) ∈ leRs variant qs nqp nd sq al i j) :
    ∃ (u : Nat) (hu : u < (lePv variant qs nqp nd sq al i).length)
      (hb : u < (lePb variant qs nqp nd sq al i).vals.length),
      (lePb variant qs nqp nd sq al i).vals[u]'hb = b ∧
      ((lePv variant qs nqp nd sq al i)[u]'hu).2 = false := by
  -- the reply echoes the P1a fan-in (`acceptor_p1`'s echo contract)
  obtain ⟨dm, hdm, hdmi, hdmb, -⟩ :=
    leRs_ok_elim variant qs nqp nd sq al i j hin
  have hflat : dm.2.ballot ∈ (leB1 variant qs nqp nd sq al j).flatten :=
    ap1t_reply_echo hdm
  rw [hdmb] at hflat
  have hunion : b ∈ unionF (fun i' =>
      (le_p_to_acceptors_p1aM variant nd i').f
        (leG variant qs nqp nd sq al)) :=
    batchC_mem hflat
  obtain ⟨i', hbi'⟩ := unionF_mem hunion
  -- ownership routes it to proposer `i`'s own broadcast
  have hii : i' = i := by
    have hdst := ap1t_reply_dst hdm
    rw [hdmb] at hdst
    have hown := le_p1a_own (variant := variant) i' b hbi'
    rw [← hown, ← hdst]
    exact hdmi
  subst hii
  -- solicitation tick + the trigger gate + the cycle prefix
  obtain ⟨u, hu, hg, hbu, htu⟩ := le_p1a_elim i' hbi'
  obtain ⟨hf, hff⟩ := le_trigger_gate i' hg htu
  have hpre := leRun_flag_prefix variant qs nqp nd sq al i'
  have hupv : u < (lePv variant qs nqp nd sq al i').length := by
    have hlen := Nat.lt_of_lt_of_le hf hpre.length_le
    rwa [List.length_map] at hlen
  refine ⟨u, hupv, hu, hbu, ?_⟩
  have hval := List.IsPrefix.getElem hpre hf
  rw [hff] at hval
  have hmap : ((lePv variant qs nqp nd sq al i').map (·.2))[u]'(by
      rw [List.length_map]; exact hupv)
      = ((lePv variant qs nqp nd sq al i')[u]'hupv).2 :=
    List.getElem_map _
  rw [hmap] at hval
  exact hval.symm

/-! ### The contracts (the module's exported I/O guarantees) -/

/-- **Ballot ownership (contract)**: every realized ballot on proposer
`i`'s output wire is owned by `i`. -/
theorem le_out_ballot_own (i : Fin nP) :
    ∀ b ∈ ((leOut variant qs nqp nd sq al).1 i).vals,
      (b : Ballot nP).proposerId = i := by
  rw [leOut_ballot_eq]
  exact p_ballot_calc_own i _

/-- **Promise opening (device face)**: an `Ok` reply on the demuxed slice
pins an acceptor tick: the payload is the `a_log` *input* value at that
tick, and the `a_max_ballot` *output* wire carries exactly the promised
ballot there. -/
theorem leRs_ok_open (i : Fin nP) (j : Fin nA) {b : Ballot nP}
    {v : P1bPayload P nP}
    (hin : (b, Except.ok v) ∈ leRs variant qs nqp nd sq al i j) :
    ∃ (tj : Nat) (htj : tj < (al j).length),
      v = (al j)[tj]'htj ∧
      ∃ hm : tj < ((leOut variant qs nqp nd sq al).2.2.2 j).vals.length,
        ((leOut variant qs nqp nd sq al).2.2.2 j).vals[tj]'hm = some b := by
  obtain ⟨dm, hdm, hdmi, hdmb, hdmres⟩ :=
    leRs_ok_elim variant qs nqp nd sq al i j hin
  obtain ⟨tj, htl, hpay, hm, hmax⟩ := ap1t_ok_spec hdm hdmres
  have hmeq := congrArg MonoSing.vals
    (leOut_max_eq variant qs nqp nd sq al j)
  have hm' : tj < ((leOut variant qs nqp nd sq al).2.2.2 j).vals.length := by
    rw [hmeq]
    exact hm
  refine ⟨tj, htl, hpay, hm', ?_⟩
  rw [List.getElem_of_eq hmeq hm']
  rw [show ((acceptor_p1_ticksM.f
      (leB1 variant qs nqp nd sq al j, al j)).1).vals[tj]'(hmeq ▸ hm')
    = ((acceptor_p1_ticksM.f
      (leB1 variant qs nqp nd sq al j, al j)).1).vals[tj]'hm from rfl,
    hmax, hdmb]

/-- **The leader-view promise contract**: at a leader tick, the view is a
full quorum of payloads, and every payload is the `a_log` **input** value
at an acceptor tick where the `a_max_ballot` **output** wire carries the
tick's own ballot (the promise, in signature vocabulary). -/
theorem le_leader_view_promise (i : Fin nP) {t : Nat}
    (ht : t < ((leOut variant qs nqp nd sq al).2.1 i).length)
    (hl : ((leOut variant qs nqp nd sq al).2.1 i)[t]'ht = true) :
    ∃ (hpr : t < ((leOut variant qs nqp nd sq al).2.2.1 i).length)
      (hpb : t < ((leOut variant qs nqp nd sq al).1 i).vals.length),
      qs ≤ (((leOut variant qs nqp nd sq al).2.2.1 i)[t]'hpr).length ∧
      ∀ v ∈ ((leOut variant qs nqp nd sq al).2.2.1 i)[t]'hpr,
        ∃ (j : Fin nA) (tj : Nat) (htj : tj < (al j).length),
          v = (al j)[tj]'htj ∧
          ∃ hm : tj < ((leOut variant qs nqp nd sq al).2.2.2 j).vals.length,
            ((leOut variant qs nqp nd sq al).2.2.2 j).vals[tj]'hm
              = some (((leOut variant qs nqp nd sq al).1 i).vals[t]'hpb) := by
  have hLq := leOut_leader_eq variant qs nqp nd sq al i
  have hQq := leOut_qlogs_eq variant qs nqp nd sq al i
  have hBq := congrArg MonoSing.vals (leOut_ballot_eq variant qs nqp nd sq al i)
  have hpv : t < (lePv variant qs nqp nd sq al i).length := by
    have := ht
    rw [hLq, List.length_map] at this
    exact this
  have hfl : ((lePv variant qs nqp nd sq al i)[t]'hpv).2 = true := by
    have h1 := List.getElem_of_eq hLq ht
    rw [h1] at hl
    rw [← hl]
    exact (List.getElem_map _).symm
  obtain ⟨-, hb, -, -⟩ := pP1bPv_getElem (leRs variant qs nqp nd sq al i)
    (lePb variant qs nqp nd sq al i).vals
    (leGl variant qs nqp nd sq al i) qs nqp (nd.p1b i) hpv
  have hpr : t < ((leOut variant qs nqp nd sq al).2.2.1 i).length := by
    rw [hQq, List.length_map]
    exact hpv
  have hpb : t < ((leOut variant qs nqp nd sq al).1 i).vals.length := by
    rw [hBq]
    exact hb
  have hview : ((leOut variant qs nqp nd sq al).2.2.1 i)[t]'hpr
      = ((lePv variant qs nqp nd sq al i)[t]'hpv).1.getD [] := by
    rw [List.getElem_of_eq hQq hpr]
    exact List.getElem_map _
  have hbal : ((leOut variant qs nqp nd sq al).1 i).vals[t]'hpb
      = (lePb variant qs nqp nd sq al i).vals[t]'hb := by
    rw [List.getElem_of_eq hBq hpb]
  refine ⟨hpr, hpb, ?_, ?_⟩
  · rw [hview]
    exact pP1bPv_leader_len (leRs variant qs nqp nd sq al i)
      (lePb variant qs nqp nd sq al i).vals
      (leGl variant qs nqp nd sq al i) qs nqp (nd.p1b i) hpv hfl
  · intro v hv
    rw [hview] at hv
    obtain ⟨hb', j, hj⟩ := pP1bPv_view_promise
      (leRs variant qs nqp nd sq al i)
      (lePb variant qs nqp nd sq al i).vals
      (leGl variant qs nqp nd sq al i) qs nqp (nd.p1b i) hpv hfl hv
    obtain ⟨tj, htj, hpay, hm, hmax⟩ :=
      leRs_ok_open variant qs nqp nd sq al i j hj
    refine ⟨j, tj, htj, hpay, hm, ?_⟩
    rw [hbal]
    exact hmax

/-- **The leader-providers contract (guarded)**: a leader view is backed by
`quorum_size` **distinct** acceptors, each contributing one of the view's
payloads — with the same signature-level promise witness per acceptor. -/
theorem le_leader_providers (hq1 : 1 ≤ qs) (i : Fin nP) {t : Nat}
    (ht : t < ((leOut PaxosVariant.guarded qs nqp nd sq al).2.1 i).length)
    (hl : ((leOut PaxosVariant.guarded qs nqp nd sq al).2.1 i)[t]'ht
      = true) :
    ∃ (hpr : t < ((leOut PaxosVariant.guarded qs nqp nd sq al).2.2.1
        i).length)
      (hpb : t < ((leOut PaxosVariant.guarded qs nqp nd
        sq al).1 i).vals.length),
      ∃ S : List (Fin nA), S.Nodup ∧ qs ≤ S.length ∧
        ∀ j ∈ S,
          ∃ v ∈ ((leOut PaxosVariant.guarded qs nqp nd sq al).2.2.1
            i)[t]'hpr,
          ∃ (tj : Nat) (htj : tj < (al j).length),
            v = (al j)[tj]'htj ∧
            ∃ hm : tj < ((leOut PaxosVariant.guarded qs nqp nd
                sq al).2.2.2 j).vals.length,
              ((leOut PaxosVariant.guarded qs nqp nd sq al).2.2.2
                  j).vals[tj]'hm
                = some (((leOut PaxosVariant.guarded qs nqp nd
                    sq al).1 i).vals[t]'hpb) := by
  have hLq := leOut_leader_eq PaxosVariant.guarded qs nqp nd sq al i
  have hQq := leOut_qlogs_eq PaxosVariant.guarded qs nqp nd sq al i
  have hBq := congrArg MonoSing.vals
    (leOut_ballot_eq PaxosVariant.guarded qs nqp nd sq al i)
  have hpv : t < (lePv PaxosVariant.guarded qs nqp nd sq al i).length := by
    have := ht
    rw [hLq, List.length_map] at this
    exact this
  have hfl : ((lePv PaxosVariant.guarded qs nqp nd sq al i)[t]'hpv).2
      = true := by
    have h1 := List.getElem_of_eq hLq ht
    rw [h1] at hl
    rw [← hl]
    exact (List.getElem_map _).symm
  obtain ⟨hb, S, hnd', hlen, hprov⟩ := pP1bPv_leader_providers
    (leRs PaxosVariant.guarded qs nqp nd sq al i)
    (lePb PaxosVariant.guarded qs nqp nd sq al i).vals
    (leGl PaxosVariant.guarded qs nqp nd sq al i) qs nqp (nd.p1b i)
    hq1 hpv hfl
    (fun j b => leRs_cap qs nqp nd sq al i j b)
  have hpr : t < ((leOut PaxosVariant.guarded qs nqp nd
      sq al).2.2.1 i).length := by
    rw [hQq, List.length_map]
    exact hpv
  have hpb : t < ((leOut PaxosVariant.guarded qs nqp nd
      sq al).1 i).vals.length := by
    rw [hBq]
    exact hb
  have hview : ((leOut PaxosVariant.guarded qs nqp nd sq al).2.2.1
      i)[t]'hpr
      = ((lePv PaxosVariant.guarded qs nqp nd sq al i)[t]'hpv).1.getD
          [] := by
    rw [List.getElem_of_eq hQq hpr]
    exact List.getElem_map _
  have hbal : ((leOut PaxosVariant.guarded qs nqp nd sq al).1
      i).vals[t]'hpb
      = (lePb PaxosVariant.guarded qs nqp nd sq al i).vals[t]'hb := by
    rw [List.getElem_of_eq hBq hpb]
  refine ⟨hpr, hpb, S, hnd', hlen, fun j hj => ?_⟩
  obtain ⟨v, hv, hvr⟩ := hprov j hj
  obtain ⟨tj, htj, hpay, hm, hmax⟩ :=
    leRs_ok_open PaxosVariant.guarded qs nqp nd sq al i j hvr
  refine ⟨v, ?_, tj, htj, hpay, hm, ?_⟩
  · rw [hview]
    exact hv
  · rw [hbal]
    exact hmax

/-- **View pinning (contract)**: two leader ticks at the same ballot number
see the same quorum view (frozen buckets). -/
theorem le_view_pinned (hq1 : 1 ≤ qs) (i : Fin nP) {t t' : Nat}
    (ht : t < ((leOut variant qs nqp nd sq al).2.1 i).length)
    (ht' : t' < ((leOut variant qs nqp nd sq al).2.1 i).length)
    (hl : ((leOut variant qs nqp nd sq al).2.1 i)[t]'ht = true)
    (hl' : ((leOut variant qs nqp nd sq al).2.1 i)[t']'ht' = true)
    (hpb : t < ((leOut variant qs nqp nd sq al).1 i).vals.length)
    (hpb' : t' < ((leOut variant qs nqp nd sq al).1 i).vals.length)
    (hnum : (((leOut variant qs nqp nd sq al).1 i).vals[t]'hpb).num
      = (((leOut variant qs nqp nd sq al).1 i).vals[t']'hpb').num) :
    ∃ (hpr : t < ((leOut variant qs nqp nd sq al).2.2.1 i).length)
      (hpr' : t' < ((leOut variant qs nqp nd sq al).2.2.1 i).length),
      ((leOut variant qs nqp nd sq al).2.2.1 i)[t]'hpr
        = ((leOut variant qs nqp nd sq al).2.2.1 i)[t']'hpr' := by
  have hLq := leOut_leader_eq variant qs nqp nd sq al i
  have hQq := leOut_qlogs_eq variant qs nqp nd sq al i
  have hBq := congrArg MonoSing.vals
    (leOut_ballot_eq variant qs nqp nd sq al i)
  have hpv : t < (lePv variant qs nqp nd sq al i).length := by
    have := ht; rw [hLq, List.length_map] at this; exact this
  have hpv' : t' < (lePv variant qs nqp nd sq al i).length := by
    have := ht'; rw [hLq, List.length_map] at this; exact this
  have hfl : ((lePv variant qs nqp nd sq al i)[t]'hpv).2 = true := by
    have h1 := List.getElem_of_eq hLq ht
    rw [h1] at hl
    rw [← hl]; exact (List.getElem_map _).symm
  have hfl' : ((lePv variant qs nqp nd sq al i)[t']'hpv').2 = true := by
    have h1 := List.getElem_of_eq hLq ht'
    rw [h1] at hl'
    rw [← hl']; exact (List.getElem_map _).symm
  have hbd : t < (lePb variant qs nqp nd sq al i).vals.length := by
    have := hpb; rw [hBq] at this; exact this
  have hbd' : t' < (lePb variant qs nqp nd sq al i).vals.length := by
    have := hpb'; rw [hBq] at this; exact this
  have hbeqt : ((leOut variant qs nqp nd sq al).1 i).vals[t]'hpb
      = (lePb variant qs nqp nd sq al i).vals[t]'hbd := by
    rw [List.getElem_of_eq hBq hpb]
  have hbeqt' : ((leOut variant qs nqp nd sq al).1 i).vals[t']'hpb'
      = (lePb variant qs nqp nd sq al i).vals[t']'hbd' := by
    rw [List.getElem_of_eq hBq hpb']
  -- ballots agree (owned + same num)
  have hbeq : (lePb variant qs nqp nd sq al i).vals[t]'hbd
      = (lePb variant qs nqp nd sq al i).vals[t']'hbd' := by
    refine Ballot.ext' ?_ ?_
    · rw [← hbeqt, ← hbeqt']
      exact hnum
    · have h1 := p_ballot_calc_own i _ _ (List.getElem_mem hbd)
      have h2 := p_ballot_calc_own i _ _ (List.getElem_mem hbd')
      rw [h1, h2]
  have hpr : t < ((leOut variant qs nqp nd sq al).2.2.1 i).length := by
    rw [hQq, List.length_map]; exact hpv
  have hpr' : t' < ((leOut variant qs nqp nd sq al).2.2.1 i).length := by
    rw [hQq, List.length_map]; exact hpv'
  have hview : ((leOut variant qs nqp nd sq al).2.2.1 i)[t]'hpr
      = ((lePv variant qs nqp nd sq al i)[t]'hpv).1.getD [] := by
    rw [List.getElem_of_eq hQq hpr]
    exact List.getElem_map _
  have hview' : ((leOut variant qs nqp nd sq al).2.2.1 i)[t']'hpr'
      = ((lePv variant qs nqp nd sq al i)[t']'hpv').1.getD [] := by
    rw [List.getElem_of_eq hQq hpr']
    exact List.getElem_map _
  refine ⟨hpr, hpr', ?_⟩
  rw [hview, hview']
  rcases Nat.le_total t t' with hle | hle
  · have hpin := pP1bPv_qlogs_pinned (leRs variant qs nqp nd sq al i)
      (lePb variant qs nqp nd sq al i).vals
      (leGl variant qs nqp nd sq al i) qs nqp (nd.p1b i) hq1 hle hpv'
      (hbt := hbd) (hbt' := hbd') hbeq hfl hfl'
    rw [show ((lePv variant qs nqp nd sq al i)[t]'hpv).1
      = ((lePv variant qs nqp nd sq al i)[t']'hpv').1 from hpin]
  · have hpin := pP1bPv_qlogs_pinned (leRs variant qs nqp nd sq al i)
      (lePb variant qs nqp nd sq al i).vals
      (leGl variant qs nqp nd sq al i) qs nqp (nd.p1b i) hq1 hle hpv
      (hbt := hbd') (hbt' := hbd) hbeq.symm hfl' hfl
    rw [show ((lePv variant qs nqp nd sq al i)[t']'hpv').1
      = ((lePv variant qs nqp nd sq al i)[t]'hpv).1 from hpin]

/-- **Ballot stability (contract)** — the paxos.rs:186–189 `nondet!`
guarantee, at the module face: a proposer that stays leader across
consecutive output ticks keeps its ballot, for every input and every
decision trace. A fabricated switch cannot bootstrap: leadership at the new
ballot needs a full bucket, whose entries were genuinely promised and hence
solicited at a flag-**false** tick (the election trigger reads
`!p_is_leader` through the `forward_ref` cycle — a standing leader stops
soliciting); by `num`-monotonicity that tick is strictly later, where the
accumulated cut still holds the full bucket, and the mask regress
(`lePv_no_false_full`) terminates on the finite run. -/
theorem le_ballot_stable (hq1 : 1 ≤ qs) (i : Fin nP) {t : Nat}
    (ht1 : t + 1 < ((leOut variant qs nqp nd sq al).2.1 i).length)
    (hl1 : ((leOut variant qs nqp nd sq al).2.1 i)[t + 1]'ht1 = true)
    (hl0 : ((leOut variant qs nqp nd sq al).2.1
      i)[t]'(Nat.lt_of_succ_lt ht1) = true) :
    ∃ hb1 : t + 1 < ((leOut variant qs nqp nd sq al).1 i).vals.length,
      ((leOut variant qs nqp nd sq al).1 i).vals[t + 1]'hb1
        = ((leOut variant qs nqp nd sq al).1
            i).vals[t]'(Nat.lt_of_succ_lt hb1) := by
  have hLq := leOut_leader_eq variant qs nqp nd sq al i
  have hBq := congrArg MonoSing.vals
    (leOut_ballot_eq variant qs nqp nd sq al i)
  have hpv1 : t + 1 < (lePv variant qs nqp nd sq al i).length := by
    have := ht1; rw [hLq, List.length_map] at this; exact this
  have hfl1 : ((lePv variant qs nqp nd sq al i)[t + 1]'hpv1).2 = true := by
    have h1 := List.getElem_of_eq hLq ht1
    rw [h1] at hl1
    rw [← hl1]; exact (List.getElem_map _).symm
  have hfl0 : ((lePv variant qs nqp nd sq al
      i)[t]'(Nat.lt_of_succ_lt hpv1)).2 = true := by
    have h1 := List.getElem_of_eq hLq (Nat.lt_of_succ_lt ht1)
    rw [h1] at hl0
    rw [← hl0]; exact (List.getElem_map _).symm
  -- `p_p1b`'s stability contract, at the module's solicitation oracle
  obtain ⟨hpb1, heq⟩ := pP1bPv_ballot_stable
    (leRs variant qs nqp nd sq al i)
    (lePb variant qs nqp nd sq al i).vals
    (leGl variant qs nqp nd sq al i) qs nqp (nd.p1b i) hq1 i
    (leGl_true variant qs nqp nd sq al i)
    (p_ballot_calc_own i _)
    (fun h ht' => (lePb variant qs nqp nd sq al i).ascending h ht')
    (fun j b v hin => leRs_ok_solicited variant qs nqp nd sq al i j hin)
    hpv1 hfl1 hfl0
  have hb1 : t + 1 < ((leOut variant qs nqp nd sq al).1 i).vals.length := by
    rw [hBq]; exact hpb1
  refine ⟨hb1, ?_⟩
  have e1 : ((leOut variant qs nqp nd sq al).1 i).vals[t + 1]'hb1
      = (lePb variant qs nqp nd sq al i).vals[t + 1]'hpb1 := by
    rw [List.getElem_of_eq hBq hb1]
  have e0 : ((leOut variant qs nqp nd sq al).1
      i).vals[t]'(Nat.lt_of_succ_lt hb1)
      = (lePb variant qs nqp nd sq al
          i).vals[t]'(Nat.lt_of_succ_lt hpb1) := by
    rw [List.getElem_of_eq hBq (Nat.lt_of_succ_lt hb1)]
  rw [e1, e0]
  exact heq

end RunContract

end HydroLean.Programs.Paxos
