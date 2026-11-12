---------------------------- MODULE VoteQuorum ----------------------------
(***************************************************************************)
(* The voting quorum of the Hydro Lean quickstart (`vote_quorum`,          *)
(* `hydro_lean/Hydro/Tutorials/Quorum.lean`) as a TLA+ state machine, for  *)
(* the comparison in `docs/docs/hydro-lean/using/comparison.mdx`. Not part *)
(* of the Lean build. One coordinator; each request is proposed at most    *)
(* once. Checked with TLC (see `README.md`): `VoteQuorum.cfg` (safety and  *)
(* `Inv` on every reachable state), `VoteQuorumInd.cfg` (`Inv` is          *)
(* inductive), `VoteQuorumLive.cfg` (liveness under weak fairness).        *)
(***************************************************************************)
EXTENDS Naturals, FiniteSets

CONSTANTS Participants, Requests, MinYes

ASSUME MinYes \in Nat

VARIABLES
  vote,      \* vote[j][r]: participant j's voting rule, fixed in Init
  proposed,  \* the requests the coordinator has broadcast
  reqMsgs,   \* requests in flight to participants: <<r, j>>
  voteMsgs,  \* votes in flight to the coordinator: <<j, r, yes?>>
  seen,      \* the coordinator's state: every vote it has consumed
  decided,   \* the coordinator's state: every request it has decided
  out        \* the output stream, as a bag: out[r] = times r was emitted

vars == <<vote, proposed, reqMsgs, voteMsgs, seen, decided, out>>

Votes == Participants \X Requests \X BOOLEAN

\* the yes votes for r in a set of votes (the sender keeps them distinct)
YesIn(S, r) == {m \in S : m[2] = r /\ m[3]}
YesVoters(r) == {j \in Participants : vote[j][r]}

Init ==
  /\ vote \in [Participants -> [Requests -> BOOLEAN]]
  /\ proposed = {}
  /\ reqMsgs = {}
  /\ voteMsgs = {}
  /\ seen = {}
  /\ decided = {}
  /\ out = [r \in Requests |-> 0]

\* the coordinator broadcasts a new request to every participant
Propose(r) ==
  /\ r \notin proposed
  /\ proposed' = proposed \cup {r}
  /\ reqMsgs' = reqMsgs \cup {<<r, j>> : j \in Participants}
  /\ UNCHANGED <<vote, voteMsgs, seen, decided, out>>

\* participant j receives request r and sends its vote back
CastVote(j, r) ==
  /\ <<r, j>> \in reqMsgs
  /\ reqMsgs' = reqMsgs \ {<<r, j>>}
  /\ voteMsgs' = voteMsgs \cup {<<j, r, vote[j][r]>>}
  /\ UNCHANGED <<vote, proposed, seen, decided, out>>

\* one coordinator tick: consume any batch of the votes that have arrived,
\* count yes votes per request, and emit the requests newly at quorum
Tick ==
  \E batch \in SUBSET voteMsgs :
    LET all     == seen \cup batch
        reached == {r \in Requests : /\ \E m \in all : m[2] = r
                                     /\ Cardinality(YesIn(all, r)) >= MinYes}
        fresh   == reached \ decided
    IN /\ voteMsgs' = voteMsgs \ batch
       /\ seen' = all
       /\ decided' = decided \cup fresh
       /\ out' = [r \in Requests |-> out[r] + (IF r \in fresh THEN 1 ELSE 0)]
       /\ UNCHANGED <<vote, proposed, reqMsgs>>

Next ==
  \/ \E r \in Requests : Propose(r)
  \/ \E j \in Participants, r \in Requests : CastVote(j, r)
  \/ Tick

Spec == Init /\ [][Next]_vars

\* the three fields of `QuorumEnsures`
DecidedProposed == MinYes >= 1 => \A r \in Requests : out[r] >= 1 => r \in proposed
Quorum          == \A r \in Requests : out[r] >= 1 => Cardinality(YesVoters(r)) >= MinYes
Once            == \A r \in Requests : out[r] <= 1

\* every vote is eventually cast, and the coordinator keeps consuming votes
Fairness ==
  /\ \A j \in Participants, r \in Requests : WF_vars(CastVote(j, r))
  /\ WF_vars(Tick)
LiveSpec == Spec /\ Fairness
\* a proposed request with enough yes voters is eventually emitted
Decides == \A r \in Requests :
  (r \in proposed /\ Cardinality(YesVoters(r)) >= MinYes) ~> (out[r] = 1)

TypeOK ==
  /\ vote \in [Participants -> [Requests -> BOOLEAN]]
  /\ proposed \subseteq Requests
  /\ reqMsgs \subseteq Requests \X Participants
  /\ voteMsgs \subseteq Votes
  /\ seen \subseteq Votes
  /\ decided \subseteq Requests
  /\ out \in [Requests -> Nat]

\* the inductive invariant; drop any conjunct and TLC finds a state that
\* satisfies the rest and steps out of it, or violates safety
Inv ==
  /\ TypeOK
  \* requests in flight were proposed
  /\ \A m \in reqMsgs : m[1] \in proposed
  \* every vote in flight or consumed is its sender's vote on a proposed request
  /\ \A m \in voteMsgs \cup seen : m[2] \in proposed /\ m[3] = vote[m[1]][m[2]]
  \* the output is exactly the decided set, each once
  /\ \A r \in Requests : out[r] = IF r \in decided THEN 1 ELSE 0
  \* a decided request was proposed and has `MinYes` yes voters
  /\ \A r \in decided : r \in proposed /\ Cardinality(YesVoters(r)) >= MinYes

-----------------------------------------------------------------------------
(* Inductiveness by TLC: start from every state of a finite universe that  *)
(* satisfies `Inv`, take one `Next` step, and check `Inv` again. Drawing   *)
(* votes from `HonestVotes` loses no state, since `Inv` requires it.       *)
HonestVotes(v) == {<<j, r, v[j][r]>> : j \in Participants, r \in Requests}

IndInit ==
  /\ vote \in [Participants -> [Requests -> BOOLEAN]]
  /\ proposed \in SUBSET Requests
  /\ reqMsgs \in SUBSET (Requests \X Participants)
  /\ voteMsgs \in SUBSET HonestVotes(vote)
  /\ seen \in SUBSET HonestVotes(vote)
  /\ decided \in SUBSET Requests
  /\ out \in [Requests -> 0..1]
  /\ Inv

IndSpec == IndInit /\ [][Next]_vars
=============================================================================
