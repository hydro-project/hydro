# HydroLean Findings Report

Issues, gaps, and formally-surfaced contracts discovered while mechanizing the Hydro
dissertation (Chs. 2–4) and porting Hydro programs to Lean. Each finding states where it
was discovered, its status, and the artifact that witnesses it.

---

## A. Findings about the dissertation formalisms

### A1. Def 2.3.3 (Streaming Progress) / Def 2.3.2 (Output Maximality) are unsatisfiable as stated
- **Where**: `Flo/Operator.lean`; discovered by the Gyatso agent while proving `Lawful`
  for terminator-forwarding operators; independently confirmed by analysis of the paper's
  own `fold`/`map` (Fig 2.14).
- **Issue**: both definitions quantify over *all* small-step configurations, including
  inconsistent ones unreachable from any execution — e.g. a `fold : B ↩→ B` whose input
  records a consumed terminator paired with an output buffer that never received the
  final aggregate. From such configurations the required conclusions (bounded outputs
  fixed; outputs maximal) are false for every terminator-forwarding operator, including
  the paper's own examples. The paper implicitly assumes configurations arise from real
  runs (its configuration typing `⊢→` gestures at this, but types-as-value-sets cannot
  express the needed state/history consistency).
- **Resolution in HydroLean**: operators carry an explicit consistency invariant
  `Operator.Inv : Vals ins → State → Vals outs → Prop` (default `True`) with closure
  obligations `inv_step`/`inv_delta` in `Operator.Lawful`; `Progress` is restricted to
  `Inv`-consistent configurations; the graph lifting `Graph.Consistent` threads through
  Lemma 2.4.4 (`progress_of_wt`). Fresh programs satisfy `Inv` by construction.
  Determinism / eager execution (Lemma 2.4.3) are unaffected.
- **Suggested paper erratum**: Def 2.3.3 should be scoped to configurations reachable
  from well-formed initial states, or operators should be equipped with an explicit
  configuration invariant (the route taken here).

### A2. `fix` must be re-arming for input-side terminator consumption
- **Where**: Gyatso agent, same investigation.
- **Issue**: Output Maximality compares a run against the `fix`-ed inputs. If terminator
  consumption is recorded in the input value (as in Fig 2.14's `[⊗] → ⊗` transition) and
  `fix` is the identity on fixed values, maximality fails from post-consumption states.
- **Resolution**: three-state carrier flags (live / pending-terminator / consumed) with
  `fix` mapping `consumed ↦ pending` ("re-arming"); `pending` absorbs concatenation, so
  `fixed (fix c)` still holds. Used by all Gyatso operator instances
  (`Gyatso/LocalColl.lean` `flagged` combinator).

---

## B. Candidate bugs in the Rust Hydro codebase

> Both B1 and B2 are **executably falsified**: `lake exe falsify` runs the
> adversarial decision scripts in `HydroLean/Programs/Paxos/Falsification.lean`
> against the faithful model (line-by-line mirror of `hydro_test/src/cluster/paxos.rs`)
> and the guarded model. Verified output (63 ms):
>
> ```
> B1 faithful: proposer 0 commits [(0, some 20)]; proposer 1 commits [(0, some 10)]  -- DISAGREEMENT at slot 0
> B1 guarded:  proposer 0 commits [(0, some 20)]; proposer 1 commits []              -- agrees (the proven theorem)
> B2 faithful send side: [((0, ⟨1,1⟩), some 99), ((0, ⟨1,1⟩), some 99)]              -- duplicate (slot, ballot) keys
> B2 guarded  send side: [((0, ⟨1,1⟩), some 99)]                                     -- once per ballot
> ```
>
> The B2 key-duplication is additionally `#guard`-checked in `Falsification.lean`.
> The scripts are schedule-level repro recipes that should round-trip to Rust `sim`
> tests for upstream confirmation.

### B1. Paxos agreement violation: duplicate P1a broadcasts are double-counted in p1b quorums
- **Where**: `hydro_test/src/cluster/paxos.rs` (`leader_election`, p1a send at :311–317;
  quorum counting via `hydro_std::quorum::collect_quorum_with_response`).
- **Mechanism**: `p_ballot.filter_if(p_trigger_election).all_ticks()` re-broadcasts the
  *same* ballot when the election trigger fires twice without an intervening ballot
  change — and nothing in `p_ballot_calc` forces an increase unless a strictly higher
  ballot was observed (verified; common case under slow networks / repeated timeouts).
  Each acceptor replies `Ok` to *each* copy (its `a_max_ballot` is unchanged by the
  duplicate). `collect_quorum_with_response` counts responses per ballot key; sender
  identity was dropped by `.values()` and nothing dedups, so one acceptor's duplicate
  `Ok`s can complete a "quorum" with fewer than f+1 distinct acceptors. A leader elected
  with such a fake quorum can miss committed entries; the model exhibits **two distinct
  values committed for the same slot**.
- **Contract view**: this violates the at-most-`max`-responses-per-key contract that
  `collect_quorum`'s own `nondet!` justification comment assumes — the formal port made
  the assumption explicit (`AtMostMaxResponses`), and the composition failed it.
- **Minimal fix** (modeled as the `guarded` variant, the target of the safety proof):
  send P1a at most once per ballot (or dedup p1b responses by sender). Under the fix the
  same adversarial script agrees.

### B2. Paxos duplicate commits / slot re-basing from every-tick recommit
- **Where**: `hydro_test/src/cluster/paxos.rs` (`recommit_after_leader_election` +
  `sequence_payload`; behavior of `latest`/`latest_atomic`/`get_max_key` re-presenting
  values every tick confirmed against `hydro_lang` sources).
- **Mechanism**: while a proposer remains leader, the recommit pipeline and `p_max_slot`
  re-fire on every tick: (a) duplicate `(slot, ballot)` metadata flows into
  `join_responses`, violating its documented one-metadata-per-key contract, so commits
  are emitted multiple times; (b) payload slot assignment re-bases to `max_slot + 1`
  every tick while recovered logs are non-empty, so concurrent client payloads can be
  assigned colliding slots after a re-election.
- **Minimal fix**: perform recommit / slot re-basing once per ballot acquisition.

### B3. `collect_quorum_with_response` (min < max): late stragglers of emitted keys are batching-dependent
- **Where**: `hydro_std/src/quorum.rs` general branch; found while porting
  (`Programs/CollectQuorumWithResponse.lean`).
- **Issue**: whether responses of an already-emitted key that arrive later are included
  in the output depends on batch boundaries; only membership / ≥min-threshold facts are
  batching-invariant. Not a safety bug for Paxos (which uses membership), but the
  operator's output is only deterministic up to the weaker spec — worth documenting
  upstream; the `assert_has_consistency_of(manual_proof!(/** TODO */))` in quorum.rs is
  dischargeable only for the membership form.

---

## C. Formalized contracts (implicit in Rust, now explicit hypotheses)

| Contract | Where surfaced | Notes |
|---|---|---|
| `collect_quorum` requires `1 ≤ min` | quorum proof | `min = 0` double-emits every key each tick — batching-dependent output. |
| `collect_quorum` requires ≤ `max` responses/key (`AtMostMaxResponses`) | quorum proof | Overshoot re-emits under some batchings (empirically verified). The English `nondet!` justification in quorum.rs is exactly this assumption. Violated by B1. |
| Quorum emission **order** is batching-dependent | quorum port | Order-sensitive spec falsified by explicit batching witness ⇒ the Rust `NoOrder` output marker is tight; specs must be multiset/membership-based. |
| `join_responses` requires unique keys per stream + metadata-before-response causality | join_responses proof | Violated by B2. |
| TwoPC requires `payloads.Nodup` | TwoPC proof | Keys *are* payloads in two_pc.rs (`Hash + Eq`); duplicate payloads break the n-responses-per-key cap. Real constraint of the Rust code. |
| TwoPC `n = 0` edge | TwoPC proof | Rust commits nothing; "vacuous unanimity" reading would commit everything. Theorems assume `1 ≤ n`. |
| `index_payloads` needs `FreshUpdates` (max-slot updates never jump backwards past `next_slot`) | index_payloads proof | Otherwise slots are reassigned; global safety must key on `(slot, ballot)` pairs, not slots. Relevant to B2(b). |
| `fold_commutative` / `fold_idempotent` closures | throughout | Rust `manual_proof!(/** ... */)` promises become real Lean hypotheses (`Multiset.AccComm`, `DupList.AccIdem`); e.g. quorum's "increment counters is commutative" is now a checked proof term. |
| `reduce_watermark` commutativity caveat in acceptor_p2 | paxos.rs:862 comment | The Rust code itself notes "not if two entries with same ballot, need assume" — the formal model keys log merge by max ballot with ties broken deterministically; ballot uniqueness (by construction, `num × proposer_id`) discharges it. |
| TwoPC is correct only under **closed** broadcast | TwoPC model + user analysis; was formalized in `Programs/TwoPCOpenMembership.lean` (*historical — retired with the old sealing layer; recover from jj history; re-formalization design below*) | The proof uses `broadcast_closed` semantics (type-level `Fin n` membership: synchronous, total, constant; same `n` = cluster size = vote domain = quorum min=max). Under *open* broadcast (membership as an input stream): `open_twoPC_not_deterministic` (identical inputs & batching, two snapshot cuts, different committed outputs ⇒ the pipeline cannot seal; the erase obligation fails), `open_twoPC_overshoot_batching_dependent` (membership growth + static `num_participants` violates `AtMostMaxResponses` ⇒ batching-dependent duplicates), `open_twoPC_relativized_unanimity` (the correct weaker guarantee under epoch-pinned recipient sets). **Upstream note**: `two_pc.rs` calls the dynamic `broadcast` with `nondet!(/** TODO */)` — that TODO is formally not dischargeable as locally-resolved; the code should call `broadcast_closed` (its deployment context is static) or adopt epoch pinning. |

---

## D. Design findings about the verification methodology (novel-research notes)

1. **Determinism must be type-derived, not hand-stated** (user directive; first
   implemented in `Hydro/Sealed.lean` — *historical, retired; the discipline lives
   on as decision-invariance faces and the `Growth`/`MonoSing` wire types*): a
   component whose Rust signature has NoOrder output and no
   `NonDet` parameter seals at `Multiset → Multiset` via `Quotient.lift`; the sealing
   witnesses are forced obligations. Hand-stated Perm-corollaries are the smell that a
   component was left unsealed.
2. **Network channels are not special** (user directive): identity/weakening operators;
   no delivery schedules exist in the simulator — modeling in-flight buffers
   double-counts nondeterminism already captured by guard-site cuts.
3. **Scheduling units are slice clocks, not locations** (verified against paxos.rs and
   `hydro_lang/src/sim`): each `sliced!` loop has its own clock; atomics are the only
   synchronization; top-level observation hooks release one element at a time so
   feedback cycles interleave; never force round-robin across sites.
4. **Proofs quantify over exactly the simulator's hook space**: a failing proof
   obligation's choice witness is a sim-script repro (B1/B2 demonstrate the round-trip).
5. **The implementation's bound hierarchy is richer than the paper's — and that richness
   is what makes invariant-by-typing work** (`Hydro/Bounds.lean`,
   `Hydro/KeyedSingletonTraj.lean` — *historical, retired; subsumed by the `MonoSing`
   wire types, see the DESIGN.md marker table*): the dissertation tracks only `Bounded`/`Unbounded`,
   but hydro_lang's `Monotonic` (singleton.rs:72), `InitNone` (optional.rs),
   `MonotonicValue`/`MonotonicKeys`/`BoundedValue` (keyed_singleton.rs:100–145) markers
   each admit a once-and-for-all generic observation theorem (cross-tick monotonicity
   under every snapshot schedule and atomic pinning, once-some-stays-some, per-key
   threshold stability). Mechanized, these turn Paxos-style run invariants
   (`MaxBallotMono`, per-slot log monotonicity under `reduce_watermark`) into one-line
   instantiations (`Programs/BoundsDemo.lean`; *historical — retired, subsumed by the `MonoSing` wire types*), with the Rust
   `monotone = manual_proof!(/** ... */)` promises becoming real constructor hypotheses.
   A paper treatment of these refined stream types would be a natural formal extension of
   §2.3.3.
6. **The configuration calculus: provenance induction replaces event induction**
   (`Hydro/ChoreoConfig.lean`, demonstrations `Programs/Paxos/ConfigProofs.lean`,
   narrative docs/09; *historical — this code was deleted with the
   choreographic layer, see the docs/09 status note; the insight carries
   over into docs/10*). The methodological headline of the project. A protocol
   configuration is not a global state: it is per-clock **consumed tick-batch
   sequences** (Flo §2.5 streams-of-streams — the nesting shape IS the materialization
   record of the `nondet!` guards) + external input; states/histories/availability are
   *derived views* through the module models and wiring readers (the runner's own
   `Faithful` proves this lossless). Reachability is characterized inductively by
   **model applications** (`Grounded`), whose eliminator is a provenance-induction
   principle ("which model application produced this data") — with a per-clock rule
   (`consInv`) that generically discharges the "other site untouched" plumbing that
   dominates world-invariant proofs. One generic lowering theorem (`grounded_ofTrace`,
   the framework's only event induction) transports config-level safety to cheat-proof
   ∀-schedule statements. Measured on Paxos's hardest invariant (promise order): the
   trace-invariant proof was ~380 lines (≈40% plumbing: event dispatch, cut clamping,
   other-site stability); the config proof is the identical ~180 lines of mathematics +
   a 3-line lift — and cross-clock provenance chains (P1b ballot provenance) become
   purely equational, with no induction at all beyond the packaged rules.
7. **Cluster collections as maps-to-streams** (user insight, this wave;
   `Hydro/ClusterFamily.lean`, `Programs/CollectQuorumFamily.lean`): just as
   Flo §2.5's streams-of-streams make cross-tick invariants ordinary
   statements about a nesting, cluster collections read as member-indexed
   families (`Fin n → List α`; in-tick-on-cluster = map-to-stream-of-stream),
   so cross-member invariants — "a quorum of *distinct* members" — are
   counting statements over the map. The generic rules
   (`countP_fanin_exchange` = the flat/nested two-lens correspondence for
   clusters, `family_extract_distinct`, `providers_extract`, pigeonhole
   intersection) belong to the calculus; `collect_quorum`'s module face then
   states its Rust-doc contract ("one response per participant") as a
   per-member ≤1 hypothesis — an argument slot that the faithful paxos.rs
   *cannot fill* (B1 as a type error).
8. **Verified faces: Rust justification sites as signature-level proof
   inputs/outputs** (user directive, this wave; `AcceptorP1.lean`/
   `AcceptorP2.lean`, plan docs/10). Each Rust-mirroring Lean function is a
   line-for-line transcription (combinator vocabulary:
   `Stream.acrossTicksMax`, `acrossTicksKeyedReduce`, `crossSingleton`, …)
   whose proof layer carries hypotheses materializing the Rust
   `manual_proof!`/`nondet!` sites and returns guarantees about its own run.
   Examples: paxos.rs:862's `manual_proof!(max by ballot (TODO: not if two
   entries with same ballot, need assume))` is now the key-nodup hypothesis
   of `ap2_okOnce` *and* the condition of the merge's order-invariance
   sealing lemma; the paxos.rs:561 "stale snapshot is safe" comment is the
   frozen-bucket theorem (`foldEarlyStop_full_stable`); comments claiming
   "no safety impact" are discharged by that stream's absence from every
   hypothesis. Commutativity obligations do not vanish into the quotient:
   they become *definition-site* sealing lemmas (checked once), possibly
   conditional, with their conditions as call-site inputs — versus Rust's
   unchecked call-site text.
9. **Ghost causality across unbounded multiset streams without tick
   arithmetic**: cross-module temporal facts ride as (a) monotone witnesses
   in contract clauses (counts against a consumed prefix, frozen
   `fold_early_stop` buckets, membership) that persist under stream
   extension, and (b) the Grounded eliminator, which puts any element
   consumed by *another* clock strictly before the current fire — so the
   P2a→carried-log→earlier-P2a provenance regress (where ballots may NOT
   decrease, because paxos.rs logs `≥ max` entries) closes by induction on
   the derivation, not on ballots. Promise-order itself needs *zero*
   induction at the composition site: it is three clause applications
   through the `a_log`/`a_max` knots, with the temporal content carried by
   one marker-given monotonicity (`a_max`, a max-lattice) and one owed
   manual witness (`a_log` coverage — its Rust type is an Unbounded
   singleton, so the framework rightly does not give monotonicity for free).
10. **Causality faces for loose decision spaces (`LeaderBallotStable`) —
   RESOLVED: derived, see D21**: the decisions-as-inputs semantics (docs/10)
   deliberately over-approximates
   the adversary — `NoOrder` batch/snapshot decisions are legal against the
   *whole* eventual availability multiset, admitting acausal cuts (a snapshot
   at tick t showing responses to a P1a broadcast at tick t' > t). Safety
   over the larger space is stronger wherever it holds; where a fact
   genuinely needs causality, the missing constraint is materialized as a
   **named proof input** on the run, exactly like the Rust `nondet!`
   justification comments. Concretely: the guarded P2a key discipline
   (`spKeys_inv`) needs *a proposer that stays leader across consecutive
   ticks keeps its ballot* — the Lean form of paxos.rs:186–189's comment
   ("we are guaranteed that the max ballot will match the current ballot of
   a proposer who believes they are the leader"). This entry originally
   recorded the hypothesis as future work needing time-indexed
   availability; the exploration behind D21 showed it is **derivable in the
   unmodified model** (now `leader_election`'s module contract
   `le_ballot_stable`, `LeaderElection.lean`) because the election trigger
   routes through a `forward_ref` cycle wire — no hypothesis remains on
   `commit_agreement` beyond `nA ≤ 2f + 1`.
11. **NoOrder batching = the shuffle as data**: once a stream is `NoOrder`,
   *all* ordering information is adversarial (any `assume_ordering` is a
   full shuffle); the faithful decision for a batch site is therefore the
   consumed batch *itself* (multiset-legal against the member-union), never
   per-source prefix cuts — which under-approximate the adversary. No
   shuffling is ever executed: downstream operators are order-tolerant by
   the Flo/Gyatso typing, and the adversary-chosen order of the decision
   list IS the shuffle. `AtLeastOnce` weakens legality to membership
   (re-delivery free), absorbed by idempotent folds (`max`).
12. **Gas and gas-less `forward_ref` agree**: the cycle combinator takes an
   unfolding depth (an adversarial decision), but with a measure bounded by
   the finite decision budget and strictly growing on non-fixed unfoldings,
   the budget-depth run is a *true fixpoint* (`iterate_fixed_of_bounded`);
   beyond the budget every fuel gives that same run, below it a ⊑-prefix —
   so prefix-closed safety proven once at the fixpoint holds for every gas,
   and `∀ fuel` statements collapse to facts about *the* run. Unbounded
   retries collapse too: quotiented (count-/membership-legal) decisions can
   spend only the finite budget, never mint new ticks.
13. **The growth orders ARE the ordering markers** (`Hydro/Growth.lean`,
   `Hydro/MonoSing.lean`; Step 0 of docs/10, user directive): Hydro
   dataflow monotonicity ("realized ticks are final") is carried **by the
   types**, never hand-stated per stage. The `Growth` class's carrier
   instances mirror the Rust marker lattice exactly — `TotalOrder` carriers
   grow by prefix, `NoOrder + ExactlyOnce` availabilities (`Cnt`) by
   multiset-count domination, `AtLeastOnce` (`Mem`) by membership; clusters
   are pointwise families. Every framework combinator exports a bundled
   `α →ₘ β` form whose `.f` is definitionally the raw combinator, so a
   module's dataflow written as a `→ₘ` composition is `rfl`-equal to its
   1:1 transcription and carries streaming progress in its type
   (`leader_election_bodyM`, `sequence_payloadM`, `paxos_core_bodyM`; the
   whole hand-written `*_prefix` lemma layer was deleted). Value-level
   monotonicity is separate and opt-in, mirroring hydro_lang's `Monotonic`
   singleton bound (singleton.rs:72): `fold_monotonic` charges the
   inflationary closure obligation (`monotone = proof`,
   `AggFuncAlgebra::monotone` + `ApplyMonotoneStream`) at the definition
   site and returns a `MonoSing` whose trajectory-ascent is a type
   projection — the honest form of "singletons are only trajectory-forward
   unless typed monotonic".
14. **Compiled-code memoization is fragile under eta-expansion — verify it
   empirically** (`memoF`, `Falsify.lean` harness): the Lean compiler
   eta-expands function-returning definitions to their full arity, so a
   `def memo f := let table := …; fun i => table[i]` *recomputes the table
   on every application* — partial applications are PAPs holding no cache.
   With nested `forward_ref`s this made every read of a cycle value re-run
   all previous unfoldings (exponential wall-clock at constant memory — the
   diagnostic signature). Countermeasures: (a) route the table through a
   `@[noinline]` struct-returning constructor that also *stores the table
   in a second field* (defeating let-float-in) and `@[inline]` the
   function-typed wrapper so evaluation lands inside a pair-returning body;
   (b) for executables, an `IO`-materialization harness that forces cycle
   families into arrays between unfoldings (the operational realization of
   `memoF`, which is semantically the identity — `memoF_def`). None of this
   affects proofs; it is purely operational.
15. **Executable falsification doubles as a vacuity check on the model**:
   compiling and *running* the B1 script exposed that the `a_log` knot as
   first modeled was **deadlocked** — `acceptor_p1`'s published max wire
   was blocked on the `a_log` cycle value, whose ticks needed `acceptor_p2`,
   whose `a_max` input needed `acceptor_p1`: the Kleene chain was stuck at
   the empty run, every stream denoted `[]` at every fuel, and the headline
   theorem was **vacuously true**. (paxos.rs is productive here:
   `a_max_ballot` folds the P1a batches alone, paxos.rs:498–502; only the
   p1b *replies* wait for the same tick's post-merge log — the
   `snapshot_atomic` write-before-ack.) Fix: publish the max wire as the
   batch fold, block only the replies on the wire. The falsifier now
   witnesses genuine commits in the guarded run — a standing wrap-up rule:
   every headline theorem needs an executable witness that its premises are
   inhabited by a run that *does something*.
16. **Monotonicity belongs in module signatures, not sidecar lemmas** (the
   wire refactor): decorative `MonoSing` values defined *next to* the
   transcription drift into unconsumed ornaments while proofs rederive
   their content from raw fold lemmas (`foldl_take_le`) — the docstrings
   claimed "type-derived" facts the proofs never routed through the types.
   The fix that sticks: module functions **return** the typed wires
   (`p_ballot_calc : … → MonoSing ballotNumVO × …`), the client callback is
   taken at its staged type `→ₘ` (absorbing `ClientsMono` into the
   signature), and the single source of every dataflow is the typed stage
   (`fooM`); the plain function is its `.f` face, kept only where a
   consumer speaks the Rust-shaped name. Consumers then *project*
   guarantees (`.ascending`, `.tick_lt_of_not_le`, `.mono`) instead of
   restating them — deleting the per-module growth/`_f`-bridge stacks
   wholesale.
17. **Export typed faces in the representation the eliminators speak**:
   routing K1's tick-ordering through the wire's *indexed* face
   (`vals[t]'ht`) ballooned the proof with realization-bounds plumbing,
   because the ambient facts live in **cut form** (`(l.take t).foldl`,
   total — `take` clamps). The inversion face exists in cut form too
   (`ValueOrder.cut_lt_of_not_le`, the contrapositive of `foldl_take_le`,
   no bounds), and with it the typed proof is *shorter* than the raw one.
   Rule: when a typed face forces a representation conversion on its
   consumers, the face is at the wrong interface — the conversion tax
   shows up as bounds plumbing at every use site.
18. **The unordered-representation trust boundary (recorded for later;
   sealing design, not yet implemented)**: availabilities (`Cnt`/`Mem`)
   expose only order-insensitive eliminators and residual order is
   adversary-chosen decision data, so the *shipped* guarantees are honest —
   but the raw layer underneath admits Rust-false positional theorems
   (e.g. `unionF srcs = srcs 0 ++ srcs 1 ++ …` is provable, and a
   guarantee stated through it would over-claim arrival order; likewise
   per-member slices of `NoOrder` cluster streams over-promise channel
   FIFO). Current seal is **statement discipline**: a guarantee is honest
   iff it factors through the marker's eliminators (membership/counts for
   `NoOrder` — cf. `commit_agreement`'s `∈`; positions only for
   `TotalOrder`; tick indices only for consumed batches). Mechanical seal
   for future users: (a) make `Cnt`/`Mem` abstract (private `val`; lawful
   intro/elim only); (b) typed edge constructors return the marker's
   carrier, so a `NoOrder` edge never *exists* as a `List` wire;
   (c) availability-type the member slices when Rust does not promise
   channel FIFO; (d) lint raw `unionF`/`.val` out of `Programs/` theorem
   statements. Related honesty note: unordered folds are decision-indexed
   trajectory families — `MonoSing` claims per-trajectory ascent only;
   cross-shuffle confluence is deliberately unclaimed and would require
   explicit commutativity if a statement ever related runs across
   shuffles.

### D19 — old-design garbage collection + migration to M-form (this wave)

The pre-decisions-as-inputs layers were deleted as unused once their
deliverables were migrated: the choreographic/TGraphSys stack (pivot 1), the
sealed-component + `Holds`-simulation stack (`Component`/`ComponentAsync`/
`Sealed`/`Sim`/`SimGraph*`/`Adequacy`/`Atomic`/`Bounds`/`*Traj`/`MTrajCore`/
`Net`/`Mset*`/`OpenBroadcast`/`RustParity` and the old-lens program wrappers
and demos). Deliverable (a) kept its engine proof (`CollectQuorumProof.lean`)
and gained the typed-stage face `collect_quorum_spec`; deliverable (b)'s
`two_pc` was **rewritten over the shared `collect_quorumM` stage** (quorum
unification: 2PC and Paxos now consume the same verified quorum module), with
the old arrival-interleaving + `Batching.of` adversary quotiented into
`batchC` decisions (`Consumes` = complete-consumption legality, now a
framework face in `TStream.lean`); `index_payloads` and `join_responses`
gained typed stages (`index_payloadsM`, `join_responsesM`) with colocated
verified faces. Standing discipline: **every Hydro function is one typed
M-form stage + a colocated face** (input properties as hypotheses, output
properties as clauses); `.f` only at protocol-level run views.

### D20 — open membership: deferred, with the design pinned (user ruling)

`TwoPCOpenMembership.lean` (open-broadcast negatives + epoch-relativized
unanimity) was retired with the old layer rather than migrated — no current
consumer needs open membership. When it returns, the setup must NOT be
overconstrained (user ruling): membership events are an **unordered stream
input like any other** (`NoOrder`; no total order on join events), and the
stream is **cluster-located when the sender is a cluster** — each member
consumes it through its own decisions, so different members may take
*different membership snapshots*. Proofs must hold for every arrival order
and every per-member snapshot choice. (The retired formalization's
`MembershipLog`/`MemberCut` pinned a single global view sequence — too
strong.) The retired negative results (dynamic broadcast cannot seal;
membership growth violates `AtMostMaxResponses`; epoch-pinned relativized
unanimity) remain valid findings about `two_pc.rs`'s `nondet!(/** TODO */)`
guards; recover the proofs from jj history when re-formalizing.


### D21 — the causality boundary rule: trigger-gated loops inherit tick-causality from the fixpoint; gateless loops need `causalTickLoop`

The design question "shouldn't within-run causality be derivable from the
fixpoint?" (raised against `LeaderBallotStable`, D10) splits cleanly:

- **A loop whose trigger routes through a `forward_ref` cycle wire inherits
  tick-causality from the fixpoint.** Paxos's p1a→p1b loop is the witness:
  `p_trigger_election` is gated on `!p_is_leader` (paxos.rs:449), and
  `p_is_leader` is a cycle wire — solicitation at any iterate reads the
  *previous* iterate's flag. A fabricated reign therefore cannot bootstrap:
  leadership at a new ballot needs a full quorum bucket
  (`pP1bPv_leader_bucket`), whose entries were genuinely promised
  (`pP1bPv_bucket_promise`), hence solicited at a flag-**false** tick
  (`le_p1a_elim` + `le_trigger_gate` + `leRun_flag_prefix` — realized
  ticks are final across the unfolding); own-ballot `num`-monotonicity puts
  that tick strictly *later*, where the accumulated snapshot cut still
  holds the full bucket (`pP1bPv_bucket_persists`) and masking it needs a
  strictly larger full own bucket — an ascending regress that terminates on
  the finite run (`pP1bPv_no_false_full`, a `p_p1b` contract over any
  solicitation oracle). Result: **`le_ballot_stable`
  (`LeaderElection.lean`) proves leader-ballot stability for every decision
  trace**; `commit_agreement`'s inputs reduce to `nA ≤ 2f + 1` alone. Executable
  record: `lake exe explore` — the best acausal script *starves* (the fake
  leader never solicits; no second commit), and the causal control shows
  the healthy healing run.
- **A gateless loop does not.** `Programs/CausalToy.lean`: a request-echo
  loop whose outputs don't influence its inputs realizes "receive the reply
  to a not-yet-sent request" under plain `batchC` (`#guard` witness). For
  that program class, `Hydro/CausalAvail.lean` is the shelved, compiled
  design: `causalTickLoop` (the atomic-tick-region combinator — a guarded
  tick-indexed fixpoint whose strict guard is tick atomicity; `batchC` is
  its constant-environment special case, `causalTickLoop_const`) with
  `CausalCuts` as its derived elimination principle. Adopt it when such a
  program arrives; retrofitting Paxos would buy zero theorem strength.

Methodology refinement (extends D8's rule): Rust `nondet!` comments that
**claim guarantees** ("we are guaranteed that…") should be *derived*, not
assumed — deriving them is verifying the comment. Comments that encode
genuine environmental assumptions remain named proof inputs.

### D22 — paxos.rs:185–188's `nondet_commit` comment under-explains its own justification (upstream doc suggestion)

The comment claims *"we are guaranteed that the max ballot will match the
current ballot of a proposer who believes they are the leader"* without
saying why. The derivation (D21) shows the guarantee is enforced by the
**election protocol itself** — a standing leader stops soliciting
(`p_trigger_election` is `!p_is_leader`-gated), own ballots strictly
increase, and the quorum check is exact equality — **not** by network
timing or delivery order: it survives every batching/snapshot decision,
including physically-impossible acausal ones. Suggested upstream edit: cite
the trigger gate at paxos.rs:449 as the justification anchor.

### D23 — contracts at the signature: the push-down cascade rule

The compositional discipline behind the final proof organization (this
entry records the rule; the Paxos tree is its witness). **If a proof at
layer `X` computes over the internal representation of a function `Y` that
`X` calls, the fact being proven belongs on `Y`'s signature** — restated as
a contract over `Y`'s own inputs/outputs (with `Y`'s input requirements as
named hypotheses or abstract witnesses) and proven inside `Y`. Applying it
transitively to Paxos cascaded three levels deep:

1. `Safety.lean` computed over run views reconstructing
   `leader_election`/`sequence_payload` internals → both modules now export
   contracts on their signatures (`le_leader_view_promise`,
   `le_leader_providers`, `le_view_pinned`, `le_ballot_stable`,
   `le_out_ballot_own`; `SPEmission`/`SPChosen` + `sp_commit_spec`,
   `sp_emission_spec`, `sp_log_entry_spec`, `sp_emission_functional` over
   `SPWireDiscipline` carriers), and `Safety.lean` is one induction over
   contract applications.
2. Those module contracts in turn assembled facts about *their* callees'
   internals (loop states, input zips, fold bridges) → the acceptors now
   export ticks-signature contracts (`ap1t_ok_spec`/`ap1t_reply_echo`/
   `ap1t_reply_dst`/`ap1t_decode_cap`; `ap2t_ok_spec` — write-before-ack
   surfaced on the published log *output* — /`ap2t_log_entry`/
   `ap2t_decode_cap`), and `p_p1b` exports the fabricated-reign regress
   (`pP1bPv_ballot_stable`/`pP1bPv_no_false_full`) over a *solicitation
   oracle* — the caller supplies only its trigger-gate fact.
3. The wiring layer (`PaxosCore.lean`) shrank to: the carrier definitions
   (`pcG`/`spInputs`), hypothesis discharge (`run_discipline` — one
   module's contract set is exactly the other's carrier requirements), the
   knot equality (`alog_succ`), wire growth (`pcG_le`), and one genuinely
   cross-module composition (`run_promise_covers`, K1 — the shared
   `Monotonic` max wire orders vote before promise; the coverage-`Monotonic`
   log type carries write-before-ack forward; the knot closes it).

Two mechanical signals find the violations: (i) grep a layer for its
callees' internal stage/loop/fold names; (ii) a whole-tree
never-referenced-declaration audit after each pass (orphaned plumbing
means its consumer moved down). The abstract-witness trick (`SPEmission` is
a `def` whose body mentions module internals, but consumers only ever use
exported lemmas about it) keeps signatures clean without leaking
representation.
