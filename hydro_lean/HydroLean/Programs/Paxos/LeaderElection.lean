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
  MonoMap.fst ∘ₘ (p_ballot_calc i).toMonoMap
    ∘ₘ le_p_received_max_ballotM nd i

def le_p_has_largest_ballotM (i : Fin nP) : LEG P nP nA →ₘ TSing Bool :=
  MonoMap.snd ∘ₘ (p_ballot_calc i).toMonoMap
    ∘ₘ le_p_received_max_ballotM nd i

/-- `p_leader_heartbeat` (paxos.rs:306–309). -/
def le_heartbeatM (i : Fin nP) :
    LEG P nP nA →ₘ Stream (Ballot nP) × TSing Bool :=
  (p_leader_heartbeat (nd.heartbeat i)).toMonoMap ∘ₘ MonoMap.pair
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
  acceptor_p1_ticks.toMonoMap ∘ₘ MonoMap.pair
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
  (p_p1b qs nqp (nd.p1b i)).toMonoMap ∘ₘ MonoMap.pair
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
  exact ((p_ballot_calc i).ensures _).own b hb''

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
    fun b hb => ((p_ballot_calc i).ensures _).own b (hsub.subset hb)
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
  ((p_leader_heartbeat (nd.heartbeat i)).ensures
    (r.p_is_leader i,
     ((le_p_ballotM nd i).f (sq, al, r)).vals)).trigger_gate hg ht


/-! ## The verified face: one artifact — dataflow, monotonicity, contract

`leader_election` is a `Verified` map: the fixpoint stage (prefix-monotone,
usable in `paxos_core`'s body via `.toMonoMap`) together with its
contract (`LEEnsures`) — ballot ownership, leader-tick view promises
(existential acceptor-tick witnesses into the `a_log` input and the
`a_max_ballot` output), distinct providers (guarded), frozen views,
ballot stability, and leader-tick nonemptiness. `num`-monotonicity of the
ballot leg and max-monotonicity are carried by the `MonoSing` output
types. These fields are exactly `sequence_payload`'s requirements
(`SPRequires`): composition is projection. -/

section Verified

variable (variant : PaxosVariant) (qs nqp : Nat) (nd : LENondet P nP nA)
variable [DecidableEq P]
variable (al : Fin nA → TSing (Option Nat × LogMap P nP))

/-- What `leader_election` **ensures**: the Rust output tuple
`(p_ballot, p_is_leader, p_relevant_p1bs, a_max_ballot)` against the
`a_log` input wire. -/
structure LEEnsures
    (out : (Fin nP → MonoSing (ballotNumVO (nP := nP)))
      × (Fin nP → TSing Bool)
      × (Fin nP → TStream (P1bPayload P nP))
      × (Fin nA → MonoSing (obtVO (nP := nP)))) : Prop where
  /-- Ballot ownership. -/
  own : ∀ (i : Fin nP), ∀ b ∈ (out.1 i).vals, (b : Ballot nP).proposerId = i
  /-- Leader ticks see nonempty views. -/
  lead_ne : 1 ≤ qs → ∀ (i : Fin nP) {t : Nat}
    (hpl : t < (out.2.1 i).length) (hpr : t < (out.2.2.1 i).length),
    (out.2.1 i)[t]'hpl = true → (out.2.2.1 i)[t]'hpr ≠ []
  /-- Ballot stability along reigns (the paxos.rs:186–189 `nondet!`
  guarantee, derived — FINDINGS D21). -/
  stable : 1 ≤ qs → ∀ (i : Fin nP) {t : Nat}
    (ht1 : t + 1 < (out.2.1 i).length)
    (hb1 : t + 1 < (out.1 i).vals.length),
    (out.2.1 i)[t + 1]'ht1 = true →
    (out.2.1 i)[t]'(Nat.lt_of_succ_lt ht1) = true →
    (out.1 i).vals[t + 1]'hb1
      = (out.1 i).vals[t]'(Nat.lt_of_succ_lt hb1)
  /-- View pinning by ballot number (frozen quorum buckets). -/
  pinned : 1 ≤ qs → ∀ (i : Fin nP) {t t' : Nat}
    (hpl : t < (out.2.1 i).length) (hpl' : t' < (out.2.1 i).length)
    (hpr : t < (out.2.2.1 i).length) (hpr' : t' < (out.2.2.1 i).length)
    (hpb : t < (out.1 i).vals.length) (hpb' : t' < (out.1 i).vals.length),
    (out.2.1 i)[t]'hpl = true → (out.2.1 i)[t']'hpl' = true →
    ((out.1 i).vals[t]'hpb).num = ((out.1 i).vals[t']'hpb').num →
    (out.2.2.1 i)[t]'hpr = (out.2.2.1 i)[t']'hpr'
  /-- Leader-view promise: at a leader tick the view is full, and every
  payload is the `a_log` **input** value at an acceptor tick where the
  `a_max_ballot` **output** carries the tick's own ballot. -/
  view_promise : ∀ (i : Fin nP) {t : Nat}
    (ht : t < (out.2.1 i).length), (out.2.1 i)[t]'ht = true →
    ∃ (hpr : t < (out.2.2.1 i).length) (hpb : t < (out.1 i).vals.length),
      qs ≤ ((out.2.2.1 i)[t]'hpr).length ∧
      ∀ v ∈ (out.2.2.1 i)[t]'hpr,
        ∃ (j : Fin nA) (tj : Nat) (htj : tj < (al j).length),
          v = (al j)[tj]'htj ∧
          ∃ hm : tj < (out.2.2.2 j).vals.length,
            (out.2.2.2 j).vals[tj]'hm = some ((out.1 i).vals[t]'hpb)
  /-- Distinct providers (guarded — the B1 send-once fan-in): a leader
  tick's view was contributed by `qs` **distinct** acceptors. -/
  providers : variant = .guarded → 1 ≤ qs → ∀ (i : Fin nP) {t : Nat}
    (ht : t < (out.2.1 i).length), (out.2.1 i)[t]'ht = true →
    ∃ (hpr : t < (out.2.2.1 i).length) (hpb : t < (out.1 i).vals.length),
      ∃ S : List (Fin nA), S.Nodup ∧ qs ≤ S.length ∧
        ∀ j ∈ S, ∃ v ∈ (out.2.2.1 i)[t]'hpr,
          ∃ (tj : Nat) (htj : tj < (al j).length),
            v = (al j)[tj]'htj ∧
            ∃ hm : tj < (out.2.2.2 j).vals.length,
              (out.2.2.2 j).vals[tj]'hm = some ((out.1 i).vals[t]'hpb)

/-- **paxos.rs:253–345 `leader_election`**: the single verified artifact —
`.f`/`.mono` the fixpoint stage, `.ensures` the contract, paid here once.
The proofs are ghost `have`s over the applied wire values: the internal
`forward_ref` traffic (`run`, `g`, `b1`, `rsx`, …) is reconstructed as
local `let`s — consumers see only the contract fields, stated on the
signature (the output tuple and the `a_log` input wire). -/
def leader_election :
    Verified ((Fin nP → Fin nA → Stream (Ballot nP))
        × (Fin nA → TSing (Option Nat × LogMap P nP)))
      ((Fin nP → MonoSing (ballotNumVO (nP := nP)))
        × (Fin nP → TSing Bool)
        × (Fin nP → TStream (P1bPayload P nP))
        × (Fin nA → MonoSing (obtVO (nP := nP))))
      (fun x out => LEEnsures variant qs x.2 out) :=
  -- the three internal forward_refs (paxos.rs:259–265), closed over the
  -- body as one product cycle
  let body := MonoMap.fix nd.fuel LERef.init
    (leader_election_bodyM variant qs nqp nd ∘ₘ MonoMap.assocR)
  Verified.ofMono body
    (fun x =>
    -- the applied values, bound once (the ghost vocabulary): the internal
    -- fixpoint history and the stage wires at the run
    let sq := x.1
    let al := x.2
    let run := MonoMap.fixHist LERef.init
      (leader_election_bodyM variant qs nqp nd ∘ₘ MonoMap.assocR)
      (sq, al) nd.fuel
    let g : LEG P nP nA := (sq, al, run)
    let rsx := fun (i : Fin nP) (j : Fin nA) =>
      (le_a_to_proposers_p1bM variant nd i j).f g
    let pbx := fun (i : Fin nP) => (le_p_ballotM nd i).f g
    let glx := fun (i : Fin nP) => (le_p_has_largest_ballotM nd i).f g
    let p1b := fun (i : Fin nP) => (le_pp1bM variant qs nqp nd i).f g
    let b1 := fun (j : Fin nA) =>
      batchC (unionF (fun i => (le_p_to_acceptors_p1aM variant nd i).f g))
        [] (nd.p1aBatch j)
    let p1bs := fun (j : Fin nA) =>
      ((acceptor_p1_ticks.f (b1 j, al j)).2).flatten
    -- the sub-artifacts' contracts at the run's wires
    let ep1b := fun (i : Fin nP) =>
      (p_p1b qs nqp (nd.p1b i)).ensures (rsx i, (pbx i).vals, glx i)
    let ap1 := fun (j : Fin nA) => acceptor_p1_ticks.ensures (b1 j, al j)
    -- ghost: the memoized family boundary, opened — the output legs are
    -- the stage wires at the run
    have hout_bal : ∀ i, (body.f x).1 i = pbx i := fun i => by
      show memoF _ i = _
      rw [memoF_eq]
      rfl
    have hout_led : ∀ i, (body.f x).2.1 i = (p1b i).1 := fun i => by
      show memoF _ i = _
      rw [memoF_eq]
      rfl
    have hout_qlogs : ∀ i, (body.f x).2.2.1 i = (p1b i).2.1 := fun i => by
      show memoF _ i = _
      rw [memoF_eq]
      rfl
    have hout_max : ∀ j, (body.f x).2.2.2 j
        = (acceptor_p1_ticks.f (b1 j, al j)).1 := fun j => by
      show memoF _ j = _
      rw [memoF_eq]
      rfl
    -- ghost: has-largest is identically `true` (`p_ballot_calc`'s field)
    have hgl_true : ∀ (i : Fin nP), ∀ y ∈ glx i, y = true := fun i =>
      ((p_ballot_calc i).ensures _).hasLargest_true
    -- ghost: decode opening — an `Ok` entry of the demuxed p1b slice is an
    -- `Ok` promise of the acceptor's raw reply stream, addressed to `i`
    have hrs_elim : ∀ (i : Fin nP) (j : Fin nA) {b : Ballot nP}
        {v : P1bPayload P nP}, (b, Except.ok v) ∈ rsx i j →
        ∃ dm ∈ p1bs j, dm.1 = i ∧ dm.2.ballot = b ∧ dm.2.res = .ok v := by
      intro i j b v hin
      have hin' : (b, Except.ok v)
          ∈ (p1bs j).filterMap (fun dm => if dm.1 = i then
            some (dm.2.ballot, dm.2.res) else none) := hin
      obtain ⟨dm, hdm, hdmeq⟩ := List.mem_filterMap.mp hin'
      have hdmi : dm.1 = i := by
        by_cases hi : dm.1 = i
        · exact hi
        · rw [if_neg hi] at hdmeq
          cases hdmeq
      rw [if_pos hdmi] at hdmeq
      have hpair := Option.some.inj hdmeq
      exact ⟨dm, hdm, hdmi, congrArg Prod.fst hpair,
        congrArg Prod.snd hpair⟩
    -- ghost: the cycle's flag input is a prefix of the run's flag output —
    -- the trigger's flag reads are final across the `forward_ref` unfolding
    have hflag_prefix : ∀ (i : Fin nP),
        (run.p_is_leader i) <+: (p1b i).1 := by
      intro i
      have hchain : (run : LERef P nP nA).le
          (MonoMap.fixHist LERef.init
            (leader_election_bodyM variant qs nqp nd ∘ₘ MonoMap.assocR)
            (sq, al) (nd.fuel + 1)) :=
        MonoMap.fixHist_chain (fun y => LERef.init_bot y) _ _
          (Nat.le_succ nd.fuel)
      have hp := hchain.p_is_leader i
      rw [← hout_led i]
      exact hp
    -- ghost: solicitation — every `Ok`-promised ballot of `i`'s reply
    -- slice rode a P1a released at a trigger-true tick, whose leader flag
    -- reads `false` off the cycle (paxos.rs:449 through :274–276)
    have hsol : ∀ (i : Fin nP) (j : Fin nA) {b : Ballot nP}
        {v : P1bPayload P nP}, (b, Except.ok v) ∈ rsx i j →
        ∃ (u : Nat) (hu : u < ((p1b i).1).length)
          (hb : u < ((pbx i).vals).length),
          ((pbx i).vals)[u]'hb = b ∧ ((p1b i).1)[u]'hu = false := by
      intro i j b v hin
      -- the reply echoes the P1a fan-in (`acceptor_p1`'s echo contract)
      obtain ⟨dm, hdm, hdmi, hdmb, -⟩ := hrs_elim i j hin
      have hflat : dm.2.ballot ∈ (b1 j).flatten := (ap1 j).reply_echo hdm
      rw [hdmb] at hflat
      have hunion : b ∈ unionF (fun i' =>
          (le_p_to_acceptors_p1aM variant nd i').f g) :=
        batchC_mem hflat
      obtain ⟨i', hbi'⟩ := unionF_mem hunion
      -- ownership routes it to proposer `i`'s own broadcast
      have hii : i' = i := by
        have hdst := (ap1 j).reply_dst hdm
        rw [hdmb] at hdst
        have hown := le_p1a_own (variant := variant) i' b hbi'
        rw [← hown, ← hdst]
        exact hdmi
      subst hii
      -- solicitation tick + the trigger gate + the cycle prefix
      obtain ⟨u, hu, hg, hbu, htu⟩ := le_p1a_elim i' hbi'
      obtain ⟨hf, hff⟩ := le_trigger_gate i' hg htu
      have hpre := hflag_prefix i'
      have hupv : u < ((p1b i').1).length :=
        Nat.lt_of_lt_of_le hf hpre.length_le
      refine ⟨u, hupv, hu, hbu, ?_⟩
      have hval := List.IsPrefix.getElem hpre hf
      rw [hff] at hval
      exact hval.symm
    -- ghost: promise opening — an `Ok` reply pins an acceptor tick: the
    -- payload is the `a_log` **input** value there, and the `a_max_ballot`
    -- **output** wire carries exactly the promised ballot there
    have hok_open : ∀ (i : Fin nP) (j : Fin nA) {b : Ballot nP}
        {v : P1bPayload P nP}, (b, Except.ok v) ∈ rsx i j →
        ∃ (tj : Nat) (htj : tj < (al j).length),
          v = (al j)[tj]'htj ∧
          ∃ hm : tj < ((body.f x).2.2.2 j).vals.length,
            ((body.f x).2.2.2 j).vals[tj]'hm = some b := by
      intro i j b v hin
      obtain ⟨dm, hdm, hdmi, hdmb, hdmres⟩ := hrs_elim i j hin
      obtain ⟨tj, htl, hpay, hm, hmax⟩ := (ap1 j).ok_spec hdm hdmres
      have hmeq := congrArg MonoSing.vals (hout_max j)
      have hm' : tj < ((body.f x).2.2.2 j).vals.length := by
        rw [hmeq]
        exact hm
      refine ⟨tj, htl, hpay, hm', ?_⟩
      rw [List.getElem_of_eq hmeq hm']
      have hmax' : ((acceptor_p1_ticks.f (b1 j, al j)).1).vals[tj]'hm
          = some dm.2.ballot := hmax
      rw [show ((acceptor_p1_ticks.f (b1 j, al j)).1).vals[tj]'(hmeq ▸ hm')
        = ((acceptor_p1_ticks.f (b1 j, al j)).1).vals[tj]'hm from rfl,
        hmax', hdmb]
    { own := fun i b hb => by
        rw [hout_bal i] at hb
        exact ((p_ballot_calc i).ensures _).own b hb
      lead_ne := fun hq1 i {t} hpl hpr hl hnil => by
        have hpl' : t < ((p1b i).1).length := by
          rw [← hout_led i]
          exact hpl
        have hfl : ((p1b i).1)[t]'hpl' = true := by
          rw [← List.getElem_of_eq (hout_led i) hpl]
          exact hl
        have hpr' : t < ((p1b i).2.1).length := by
          rw [← hout_qlogs i]
          exact hpr
        have hnil' : ((p1b i).2.1)[t]'hpr' = [] := by
          rw [← List.getElem_of_eq (hout_qlogs i) hpr]
          exact hnil
        have hfull := (ep1b i).leader_len hpr' hpl' hfl
        have hfull' : qs ≤ (((p1b i).2.1)[t]'hpr').length := hfull
        rw [hnil'] at hfull'
        simp at hfull'
        omega
      stable := fun hq1 i {t} ht1 hb1 hl1 hl0 => by
        have ht1' : t + 1 < ((p1b i).1).length := by
          rw [← hout_led i]
          exact ht1
        have hl1' : ((p1b i).1)[t + 1]'ht1' = true := by
          rw [← List.getElem_of_eq (hout_led i) ht1]
          exact hl1
        have hl0' : ((p1b i).1)[t]'(Nat.lt_of_succ_lt ht1') = true := by
          rw [← List.getElem_of_eq (hout_led i) (Nat.lt_of_succ_lt ht1)]
          exact hl0
        -- `p_p1b`'s stability contract, at the module's own wires
        obtain ⟨hpb1, heq⟩ := (ep1b i).ballot_stable hq1
          ⟨hgl_true i, ((p_ballot_calc i).ensures _).own,
           fun h ht' => (pbx i).ascending h ht',
           fun j b v hin => hsol i j hin⟩
          ht1' hl1' hl0'
        have hBq := congrArg MonoSing.vals (hout_bal i)
        rw [List.getElem_of_eq hBq hb1,
          List.getElem_of_eq hBq (Nat.lt_of_succ_lt hb1)]
        exact heq
      pinned := fun hq1 i {t t'} hpl hpl' hpr hpr' hpb hpb' hl hl'
          hnum => by
        have hpl2 : t < ((p1b i).1).length := by
          rw [← hout_led i]
          exact hpl
        have hpl2' : t' < ((p1b i).1).length := by
          rw [← hout_led i]
          exact hpl'
        have hfl : ((p1b i).1)[t]'hpl2 = true := by
          rw [← List.getElem_of_eq (hout_led i) hpl]
          exact hl
        have hfl' : ((p1b i).1)[t']'hpl2' = true := by
          rw [← List.getElem_of_eq (hout_led i) hpl']
          exact hl'
        have hBq := congrArg MonoSing.vals (hout_bal i)
        have hbd : t < (pbx i).vals.length := by
          rw [← hBq]
          exact hpb
        have hbd' : t' < (pbx i).vals.length := by
          rw [← hBq]
          exact hpb'
        -- ballots agree (owned + same num)
        have hbeq : (pbx i).vals[t]'hbd = (pbx i).vals[t']'hbd' := by
          refine Ballot.ext' ?_ ?_
          · rw [← List.getElem_of_eq hBq hpb,
              ← List.getElem_of_eq hBq hpb']
            exact hnum
          · have h1 := ((p_ballot_calc i).ensures _).own _
              (List.getElem_mem hbd)
            have h2 := ((p_ballot_calc i).ensures _).own _
              (List.getElem_mem hbd')
            rw [h1, h2]
        have hqr : t < ((p1b i).2.1).length := by
          rw [← hout_qlogs i]
          exact hpr
        have hqr' : t' < ((p1b i).2.1).length := by
          rw [← hout_qlogs i]
          exact hpr'
        rw [List.getElem_of_eq (hout_qlogs i) hpr,
          List.getElem_of_eq (hout_qlogs i) hpr']
        rcases Nat.le_total t t' with hle | hle
        · have hpin : ((p1b i).2.1)[t]'hqr = ((p1b i).2.1)[t']'hqr' :=
            (ep1b i).qlogs_pinned hq1 hle hqr' hpl2'
              (hbt := hbd) (hbt' := hbd') hbeq hfl hfl'
          exact hpin
        · have hpin : ((p1b i).2.1)[t']'hqr' = ((p1b i).2.1)[t]'hqr :=
            (ep1b i).qlogs_pinned hq1 hle hqr hpl2
              (hbt := hbd') (hbt' := hbd) hbeq.symm hfl' hfl
          exact hpin.symm
      view_promise := fun i {t} ht hl => by
        have ht' : t < ((p1b i).1).length := by
          rw [← hout_led i]
          exact ht
        have hfl : ((p1b i).1)[t]'ht' = true := by
          rw [← List.getElem_of_eq (hout_led i) ht]
          exact hl
        have hvfl : ((p1b i).2.1).length = ((p1b i).1).length :=
          (ep1b i).views_flags_len
        have htv : t < ((p1b i).2.1).length := by
          rw [hvfl]
          exact ht'
        have hfbl : ((p1b i).1).length ≤ ((pbx i).vals).length :=
          (ep1b i).flags_ballot_len
        have hb : t < ((pbx i).vals).length := Nat.lt_of_lt_of_le ht' hfbl
        have hpr : t < ((body.f x).2.2.1 i).length := by
          rw [hout_qlogs i]
          exact htv
        have hBq := congrArg MonoSing.vals (hout_bal i)
        have hpb : t < ((body.f x).1 i).vals.length := by
          rw [hBq]
          exact hb
        refine ⟨hpr, hpb, ?_, ?_⟩
        · rw [List.getElem_of_eq (hout_qlogs i) hpr]
          have hlen : qs ≤ (((p1b i).2.1)[t]'htv).length :=
            (ep1b i).leader_len htv ht' hfl
          exact hlen
        · intro v hv
          rw [List.getElem_of_eq (hout_qlogs i) hpr] at hv
          have hv' : v ∈ ((p1b i).2.1)[t]'htv := hv
          obtain ⟨hb2, j, hj⟩ := (ep1b i).view_promise htv ht' hfl hv'
          obtain ⟨tj, htj, hpay, hm, hmax⟩ := hok_open i j hj
          refine ⟨j, tj, htj, hpay, hm, ?_⟩
          rw [List.getElem_of_eq hBq hpb]
          exact hmax
      providers := fun hv hq1 i {t} ht hl => by
        subst hv
        -- the fan-in is duplicate-free (guarded send-once B1 +
        -- ballot-ownership cross-proposer disjointness)
        have hb1_nodup : ∀ j, ((b1 j).flatten).Nodup := by
          intro j
          refine nodup_of_count_le_one (fun b => ?_)
          have h2 : ((b1 j).flatten).count b
              ≤ (unionF (fun i' =>
                (le_p_to_acceptors_p1aM PaxosVariant.guarded nd i').f
                  g)).count b :=
            batchC_count_le _ _ b
          have h3 : (unionF (fun i' =>
              (le_p_to_acceptors_p1aM PaxosVariant.guarded nd i').f
                g)).count b ≤ 1 := by
            rw [unionF_count]
            refine sum_map_le_single _ b.proposerId 1 (fun i' hi => ?_) ?_
            · rw [List.count_eq_zero]
              intro hmem
              exact hi (le_p1a_own i' b hmem).symm
            · exact count_le_one_of_nodup (le_p1a_nodup b.proposerId) b
          exact Nat.le_trans h2 h3
        -- p1b reply cap (`acceptor_p1`'s decode-cap contract at the
        -- duplicate-free fan-in)
        have hcap : PP1bReplyCap (rsx i) := fun j b =>
          (ap1 j).decode_cap (hb1_nodup j) i b
        have ht' : t < ((p1b i).1).length := by
          rw [← hout_led i]
          exact ht
        have hfl : ((p1b i).1)[t]'ht' = true := by
          rw [← List.getElem_of_eq (hout_led i) ht]
          exact hl
        have hvfl : ((p1b i).2.1).length = ((p1b i).1).length :=
          (ep1b i).views_flags_len
        have htv : t < ((p1b i).2.1).length := by
          rw [hvfl]
          exact ht'
        obtain ⟨hb2, S, hnd', hlen, hprov⟩ :=
          (ep1b i).leader_providers hq1 htv ht' hfl hcap
        have hpr : t < ((body.f x).2.2.1 i).length := by
          rw [hout_qlogs i]
          exact htv
        have hBq := congrArg MonoSing.vals (hout_bal i)
        have hfbl : ((p1b i).1).length ≤ ((pbx i).vals).length :=
          (ep1b i).flags_ballot_len
        have hpb : t < ((body.f x).1 i).vals.length := by
          rw [hBq]
          exact Nat.lt_of_lt_of_le ht' hfbl
        refine ⟨hpr, hpb, S, hnd', hlen, fun j hj => ?_⟩
        obtain ⟨v, hvmem, hvr⟩ := hprov j hj
        obtain ⟨tj, htj, hpay, hm, hmax⟩ := hok_open i j hvr
        refine ⟨v, ?_, tj, htj, hpay, hm, ?_⟩
        · rw [List.getElem_of_eq (hout_qlogs i) hpr]
          exact hvmem
        · rw [List.getElem_of_eq hBq hpb]
          exact hmax })

end Verified

end HydroLean.Programs.Paxos
