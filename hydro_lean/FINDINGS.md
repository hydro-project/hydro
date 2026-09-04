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
   `leader_election.ensures.stable`) because the election trigger
   routes through a `forward_ref` cycle wire — no hypothesis remains on
   the safety face (now carried on `paxos_core`'s output type) beyond
   `nA ≤ 2f + 1`.
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
   (`leader_election_bodyM`, `paxos_core_bodyM`, the `Verified` bodies; the
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
   `NoOrder` — cf. `SlotFunctional`'s `∈`; positions only for
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
  leadership at a new ballot needs a full quorum bucket (the `hbucket`
  ghost in `p_p1b`), whose entries were genuinely promised (`hbpromise`),
  hence solicited at a flag-**false** tick (`le_p1a_elim` +
  `le_trigger_gate` + the `hflag_prefix` ghost — realized ticks are final
  across the unfolding); own-ballot `num`-monotonicity puts that tick
  strictly *later*, where the accumulated snapshot cut still holds the
  full bucket (`hpersists`) and masking it needs a strictly larger full
  own bucket — an ascending regress that terminates on the finite run
  (the `hnff` ghost, exported as `p_p1b.ensures.ballot_stable` over the
  named `PP1bRequires`). Result: **`leader_election.ensures.stable`
  proves leader-ballot stability for every decision
  trace**; the safety face's inputs reduce to `nA ≤ 2f + 1` alone. Executable
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

1. The assembly layer computed over run views reconstructing
   `leader_election`/`sequence_payload` internals → both modules now export
   contracts on their signatures (`LEEnsures`: `view_promise`,
   `providers`, `pinned`, `stable`, `own`; `SPEmission`/`SPChosen` +
   `SPEnsures`: `commit_spec`, `emission_spec`, `log_entry_spec`,
   `emission_functional` over the named `SPRequires`), and the final
   assembly is one induction
   over contract applications (now living with `paxos_core` itself).
2. Those module contracts in turn assembled facts about *their* callees'
   internals (loop states, input zips, fold bridges) → the acceptors now
   export ticks-signature contracts (`AP1Ensures`: `ok_spec`/`reply_echo`/
   `reply_dst`/`decode_cap`; `AP2Ensures`: `ok_spec` — write-before-ack
   surfaced on the published log *output* — /`log_entry`/`decode_cap`),
   and `p_p1b` exports the fabricated-reign regress
   (`ballot_stable`) over the named `PP1bRequires` — the caller supplies
   only its trigger-gate fact (`solicited_at_follower`).
3. The wiring layer (`PaxosCore.lean`) shrank to: the carrier definitions
   (`pcG`/`spInputs`), hypothesis discharge (`pcReq` — one
   module's contract set is exactly the other's named requirement), the
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

### D24 — the endgame of proofs-through-types: the program's type IS its spec

`paxos_core` is a `Verified` artifact whose contract slot is
`variant = .guarded → nA ≤ 2 * f + 1 → SlotFunctional out.2` —
the safety guarantee is a **field of the artifact**, so the D23
cascade terminates at the top: the outermost function carries a contract
on *its* signature exactly like the modules it calls, and there is **no
separate headline theorem left** (consumers project
`(paxos_core …).ensures cb rfl hnA`; `#print axioms paxos_core` audits the
face because the definition contains the proof). Three ingredients make
it work:

1. **The `fold_monotonic` pattern at whole-program scale**: the value
   components stay *definitionally* the 1:1 transcription (the fixpoint
   projections); the proof is attached at the boundary and never touches
   the computation — the falsifier/explorer executables compile and print
   byte-identical output through `.f`.
2. **The guarantee is an implication guarded by the bugfix flag**
   (`variant = .guarded → …`): one definition serves the faithful and the
   guarded variant; for `faithful` the hypothesis is false and the type
   claims nothing — "run with the bugfix flag and the properties hold" is
   literally the type. This mirrors how Rust would express it: a
   `paxos_core` whose output type is refined under a const-generic flag.
3. **`Safety.lean` had no Rust counterpart** — a separate safety file was
   a TLA-era organizational habit. Under the 1:1 file-per-function rule,
   the assembly (`emission_chosen_agree`, `fix_slot_functional`) lives
   with `paxos_core`, and the file map is again exactly the Rust file map.

Rule of thumb: when a "final assembly" file exists whose only content is
composing a function's own contracts, dissolve it — the composition is the
proof obligation of that function's *type*.

### D25 — the artifact IS the function: `Verified` + ghost `have`s (the final form)

D24 put the guarantee on the outermost function's type; this wave makes
every Rust function ship as **one Lean definition** carrying all three
faces, and fixes where the proofs live:

```
structure Verified (α β) [Growth α] [Growth β] (Ens : α → β → Prop)
    extends MonoMap α β where
  ensures : ∀ a, Ens a (f a)
```

1. **Shape of a definition** (`BallotCalc.lean` is the reference): the
   body is an inline `let`-chain of wire combinators — one `let` per Rust
   `let`, in Rust order; closures are spelled once (inline, or as a body
   `let` when proofs must analyze them, e.g. `jump`); per-closure
   obligations ride the combinator (`fold_monotonicM` takes the
   `monotonic =` proof inline-curried). The ensures closure binds the
   **applied values** once (`let pvx := …`, `let rsx := …`) and proves
   ghost `have`s over those short names — the Verus ghost style: `let`
   for data, `have` for proofs. Props erase; the falsifier executes the
   identical wire graph.
2. **Contracts are stated over the actual output** (`Ens : α → β → Prop`
   used honestly): `PP1bEnsures`/`LEEnsures`/`SPEnsures` fields quantify
   over `out` projections, not over parallel "spec projection" defs. This
   killed the view-def stacks (`pP1bPv`/`pP1bViews`/`leOut`/`leRun`/
   `lePv`/`sp_*` stage views and their `*_eq` lemma trains): what used to
   be a file-level lemma over a view is now a ghost `have` over a local
   `let`, and what used to be a view-to-output bridge is one `memoF_eq`
   opening (`hout_*`). Consumers get facts about the wires **they hold**
   (composition = `.ensures` applied at their own values, e.g. `pcReq` /
   `(ep1b i).ballot_stable`), with no defeq excursions through another
   module's vocabulary.
3. **Preconditions are named per guarantee** — no global `Req` slot on
   `Verified`. Guarantees needing hypotheses take them as implications of
   the individual field, packaged in named structures (`PP1bRequires`
   with its cycle-feedback `solicited_at_follower`; `PP1bReplyCap`;
   `SPRequires`). Heterogeneous requirements per field made a single
   artifact-level `Req` unusable (`p_p1b` proved the point).
4. **What stays file-level**: pure-function characterizations (the bucket
   calculus, `SendStep` layer, `dedupLast`) and input-generic wire faces
   (`le_p1a_own`/`le_p1a_nodup`/`le_p1a_elim`/`le_trigger_gate` — generic
   over the carrier, consumed at several instantiations). Rule: a lemma
   survives at file level iff its statement is generic over the inputs;
   anything stated at *the run* is a ghost `have`.
5. **Elaboration gotchas** (hard-won): `where`-bound names are opaque
   during the def's own elaboration — everything the body or record needs
   must be inline `have`s; `rw` needs syntactically aligned bound proofs
   (re-derive `have ht' : t < form.length := ht` first) and syntactically
   matching forms (state the `hout_*` equations over the raw `body.f x`
   spelling, then land in local-`let` vocabulary); `omega` atomizes
   syntactically distinct-but-defeq terms (ascribe hypotheses into one
   spelling before arithmetic); term-mode ascription `have h : T := e`
   and tactic `show` are the defeq bridges of choice.
6. **Prop-propagating combinators considered and rejected** (except for
   closed families): propagation works exactly when the prop family is
   closed under the combinator algebra — prefix-monotonicity is such a
   family and lives in the types (`MonoMap`/`MonoSing`); arbitrary safety
   props are not (a zip's derived prop `P a bc.1 ∧ Q a bc.2` loses the
   *correlation* between the legs, which is where the content is — e.g.
   "the flag leg was computed from the same view as the view leg"), and
   the heavy facts (cross-tick pinning, the fabricated-reign regress) are
   trace-global. They ride ghost `have`s.

Net effect: `Programs/Paxos/` has zero contract-statement duplication —
each guarantee is spelled exactly once, as an `Ensures` field.

### D26 — paxos.rs:386–390 `p_has_largest_ballot` is provably identically `true` (upstream note candidate)

`p_ballot_calc`'s output ensures `hasLargest_true`: on every realized
tick, `p_has_largest_ballot = (p_received_max_ballot <= Some(cur_ballot))`
is `true`, because the zip pairs each `p_received_max_ballot` view with
the ballot computed *from that same view* within the tick, and the jump
closure (paxos.rs:365–379) always overtakes the received max (either it
jumps to `received.num + 1 > received.num`, or lex-totality gives
`received ≤ (num, me)`). Consequently the `.and(p_has_largest_ballot)`
conjunct in `p_is_leader` (paxos.rs:588) is **vacuous under tick
atomicity** — it can only bite if the leader flag were computed against a
*staler* ballot than the max it compares, which the tick structure forbids.
The Lean proof consumes the field anyway (`PP1bRequires.has_largest`), so
the model would survive the conjunct's removal; as with D22, this is a
place where the Rust code embeds a cross-wire timing invariant that the
dataflow discharges by construction — an upstream comment (or removal)
would record that.

### D27 — the colocated contract: the dependent-implication clause on the return type (V2's `Verified`)

The V2 surface replaces v1's `Verified` wrapper with a **colocated
contract**: a polymorphic program `def foo (H : HydroSem L mem) …`
returns a subtype whose clause is a *dependent implication on the
instantiation* —

    {out : H.Wires … //
      ∀ hv : H = Values L mem,
        match H, hv, input₁, …, out with
        | _, rfl, x₁, …, o => FooEnsures … x₁ … o}

- The program text stays the 1:1 paxos.rs transcription (consumers
  project `.val`; the proof is ghost, erased at runtime — the demo
  executables run the same traces byte-for-byte).
- The proof is written **inside the definition**, after the body's
  `let`s (`intro hv; subst hv; …`), so it speaks about the wires **by
  let-name** — the Verus-style colocation that killed both the external
  `*_ensures` duplicates and the def-per-let vocabulary.
- Flo monotonicity stays the `MonoRel` two-liner (`.val.….property`),
  **except across variable-fuel `fix` boundaries**, where the
  iterate-projection principle does not hold definitionally; there the
  knots are re-crossed with the Kleene lemmas (`iterate_le_succ`,
  `iterate_chain`, `iterate_mono_param`) with the core's `MonoRel`
  instantiation as the step (`leader_election`'s knot fact and
  `leader_election_mono` are the patterns).
- Mirror vocabulary (`spSentTrace`, `pP1bViews`, …) is *checked*
  duplication: the colocated proof pins it to the body with a
  `have hface : wire = mirror := rfl` — drift breaks the build.
- File shape (user-ratified): contract vocabulary → `Requires` →
  `Ensures` → the program (proof colocated; single-consumer helpers as
  `where` bindings) → everything else; pure lemma stacks too big to
  review inline get a sibling `…Lemmas.lean` (PP1b, SequencePayload).
- **The correspondence hook**: `H = Values L mem` is one point of a
  gluing-indexed family; generalizing the clause over the gluing (the
  `MonoRel` diagonal is already the monotonicity instance) is the
  staged path to a single statement covering denotation, monotonicity,
  and the Sched/Corr transfer.
- **The curried completion** (`fix`-closed modules, user-ratified): a
  module whose wires close over `forward_ref` knots returns its I/O
  **function** with a single clause carrying the unary contract AND
  binary Flo monotonicity between any two runs —

      {f : In₁ → In₂ → Out //
        ∀ hv : H = Values L mem,
          (∀ x₁ x₂, Ensures … (f x₁ x₂))
          ∧ (∀ x₁ x₁' x₂ x₂', x₁ ⊑ x₁' → x₂ ⊑ x₂' →
              GradedRel (f x₁ x₂) (f x₁' x₂'))}

  The pass is a reviewable `let` (reading order: the Rust text first),
  and because the mono clause is *inside* the definition, its proof
  reaches the pass **by name** — no external theorem ever re-spells it.
  The external `leader_election_mono`/`leader_election_core` pair died
  of this; consumers project `(….property rfl).2 x₁ x₁' x₂ x₂' h₁ h₂`.
  Two elaboration rules learned: (i) the `MonoRel` step applications
  inside the clause must be **inline terms**, not `have`-bound (a
  `have` is opaque, so its projections don't defeq-reduce to the
  component runs); (ii) consumers that name run stages (`pcLE`) must
  **type-ascribe** the contract projections to the stage spelling —
  the raw curried application is defeq but not syntactic, and `omega`
  &co. work syntactically. The fixpoint HOF package shipped with this
  (`Trace.lean`): `fix_induction`, `iterate_val_proj`,
  `iterate_stab_of_fixed`, `fix_stabilizes`.

### D28 — V2 headline: `paxos_core`'s type carries `SlotFunctional`, end to end

`HydroV2.paxos_core`'s colocated clause is the safety headline —

    variant = .guarded → mem acc ≤ 2 * f + 1 → SlotFunctional out.2

— proved from module contracts only (composition never re-enters a
callee): `LEEnsures` (ownership, reign stability via the
fabricated-reign regress D21, pinned views, view promises, distinct
providers from the B1 send-once fan-in), `SPEnsures` (commits are owned
emissions at chosen keys; published log entries quote emissions;
coverage ascends), the pure sequencing calculus (the fused
`spSendStep` scan, guarded key-send-once from B2 + slot freshness), and
`AP2Ensures` (per-sender ack caps + write-before-ack coverage). The
`a_log` knot (`snapshot_atomic`) is opened **once**
(`pcAlogW_succ : pcAlogW (m+1) = (pcSP m).val.2.1 := rfl`) and K4
regresses `k+1 → k` through it, exactly v1's `emission_chosen_agree`
shape. Audit: zero sorries in `HydroV2/`; `paxos_core` and
`paxos_core_agree` depend on `[propext, Classical.choice, Quot.sound]`
only; `lake exe falsify`/`explore`/`v2paxos` unchanged and green.

### D29 — the shared verified stage library (`HydroV2/Std/`): hydro_std's quorum layer, extracted

The V2 port had silently **fused** `hydro_std`'s reusable stages into
their Paxos call sites — `p_p1b` implemented
`collect_quorum_with_response`'s success leg as a raw ok-`filter_map`
(the accumulate/emit-once registers absorbed by `fold_early_stop`'s
cap), and `sequence_payload` fused `collect_quorum` + `join_responses`
into one bespoke scan with a single batch decision where Rust has two
`nondet!` sites. That broke the one-Rust-fn-one-Lean-def discipline
*and* silently weakened fidelity (one decision where Rust exposes two).
This wave extracts the layer as V2's first shared verified stages, and
re-routes both consumers:

- **`Std/Quorum.lean`** — `collect_quorum` (quorum.rs:90–160) and
  `collect_quorum_with_response` (quorum.rs:7–88), program text once
  over the signature, `sliced!` registers as the pure `cqTick`/
  `cqwrTick`, colocated contracts. quorum.rs:25's
  `nondet!(/** …arbitrary batching…deterministic quorum results */)`
  comment is now a **theorem**: `CQEnsures.emit_count` says the
  emission count of a key is the crossing indicator — an *iff*, once,
  for every batching (under the usage caps `1 ≤ min ≤ max`, per-key
  responses `≤ max`); B3's straggler caveat is visible as exactly the
  cap hypotheses on `CQWREnsures.emit_le`/`emit_complete`.
- **`Std/RequestResponse.lean`** — `join_responses`
  (request_response.rs:15–43), `remaining_to_join` as `jrTick`. The
  metadata side is `atomic` in Rust (paxos.rs:764–770), so it enters as
  a **tick-stream parameter, not a decision** — same-tick availability
  is the meaning of `atomic`, and `JREnsures.join_complete` proves the
  no-stale-join property from it (user ruling).
- **The enabling clause** (`CQWREnsures.emit_pool_le`): the success
  leg's embedding is a **direct clause of the contract record** — the
  emissions embed **with multiplicity** in the consumed pool's `Ok`
  projection *unconditionally* — no usage caps. (An earlier draft
  routed this through an `@[irreducible]` spec function
  `cqwrEmissions` equated to the output by an `emit_eq` clause plus a
  satellite theorem about the opaque name; the user rejected that as
  the satellite-lemma anti-pattern at one remove — an equation to an
  opaque name carries no content, and consumers must read everything
  off the method's type. All surface content now lives in the Ensures
  clauses; the machine names are private to `Std/`.) The
  `min_but_not_max` register can only re-emit post-reset
  responses, so windows never double-count. This is what lets
  `p_p1b`'s fabricated-reign regress (D21) rebase onto the shared
  stage without adding a single hypothesis to `LEEnsures`/`PCEnsures`:
  the regress is soundness-only, and soundness survives even
  adversarial batching.
- **Re-routes**: `sequence_payload`'s commit proof now composes
  `JREnsures.join_src` + `CQEnsures.emit_sound` (its bespoke
  `spOkCount`/`spNewCommits`/`spCommitStep` calculus died);
  `p_p1b`/`leader_election` regained Rust's `num_quorum_participants`
  parameter (paxos.rs:259, 539) with `paxos_core` passing `2f + 1`.
  The realized success pool is an **existential witness of `p_p1b`'s
  own contract** (`∃ okPool, PP1bEnsures … okPool …` with the clause
  `ok_pool_le : okPool ≤ filterMap p1bOkPair pool` — the v1
  `SPEmission` witness precedent): the view vocabulary
  (`pP1bViews`/`pP1bFlags`) is parameterized by the pool value
  directly, `p_p1b`'s colocated proof instantiates the witness with
  its own success wire and pays the embedding from
  `CQWREnsures.emit_pool_le` (bridged by `consumed_okProj_le`), and
  `leader_election` destructures the witness — every consumption site
  reads a contract clause. Headline (`paxos_core` +
  `paxos_core_agree`) statements unchanged.
- **Layout (user rulings; final form — the generic-combinator wave)**:
  each Std module file reads Requires/Ensures → the register machine
  (`CQState`/`cqTick`/`cqwrTick`/`jrTick` — the Rust `sliced!` bodies,
  **defined once**; the program bodies fold with the named step, so
  there is no spec mirror and nothing to drift) → step obligations
  (per-tick case analyses over the module's own step) → run facts,
  each a single application of a generic scan combinator → the program
  definitions → smoke tests. The run-level induction boilerplate lives
  once, generically, in `Trace.lean`'s `ScanRun` section: `scan_sound`
  (per-emission provenance), `scan_emit_ind` (emission-accumulator
  induction with an ambient total — the capped/crossing shape),
  `scan_bound` (the potential-function/amortized bound behind the
  unconditional `emit_pool_le`), and `map_sum_additive`. An earlier
  draft kept the machine in a `…Theory.lean` sibling as an
  `rfl`-bridged specification mirror because where-clauses cannot host
  dependent theorem stacks (a where-binding's type cannot reference a
  sibling); the user rejected duplicating core logic, and naming the
  step (the ratified `ipStep`/`pbcJump` pattern) plus genericizing the
  inductions removed the need. `…Theory.lean` files now hold only
  contract vocabulary and pool algebra (`QuorumTheory.lean`;
  `RequestResponseTheory.lean` dissolved). Consumers operate off the
  surface only (grep-enforced: no machine name appears outside
  `Std/`).
- **Elaboration note**: `DecidableEq (Ballot nP × Except (Option
  (Ballot nP)) (ALog P nP))` exceeds the default instance-synthesis
  depth even though every component instance exists (assembling
  `instDecidableEqProd` explicitly checks instantly) — pinned as
  `p1bPairDecEq` in `PP1bLemmas.lean`.

### D30 — the step-machine transfer: erase the quotients, let order emerge, derive the decisions

The Sched/Corr transfer theorem shipped
(`Sched.lean`/`Rel.lean`/`Transfer.lean`/`TransferTheory.lean`/
`TransferChecks.lean`, `Paxos/TransferCheck.lean`) after a design
session that overturned three drafts; the final architecture and the
reasons each draft died are both findings.

- **Draft 1 (quotient carriers on the machine) died on honesty**: an
  operational semantics whose `NoOrder` buffers are `Multiset`s is not
  a machine — a real runtime holds lists and folds them in arrival
  order. The ruling: in sched mode all ordering/retry types are
  **completely erased** — plain lists everywhere; the correspondence
  proof, not the machine, pays the quotients. Corollary discovered on
  the way: transport never *creates* retries (`TCP.fail_stop`
  preserves the grade), so an earlier per-pair `next/again` redelivery
  machinery was deleted outright — `AtLeastOnce` duplicates are
  content, born at sampling sites, riding the wire.
- **Draft 2 (merge-order oracles) died on physics**: fan-in
  interleaving is not a decision made *at* `union`/`values` — it is
  *determined* by delivery timing. Forcing it: carriers became
  **step-indexed histories** (`StepHist`, prefix-monotone "buffer as
  of step `t`", one global clock), after which fan-in is increment
  concatenation, batch is consume-all-at-tick, snapshot reads the
  accumulator, and `fix` is the Kleene diagonal — cycle fuel
  disappears because cycle unfolding *is* step progression. The
  machine's whole vocabulary shrank to: per-pair delivery cursors,
  tick skeletons (irreducible content: stutter ticks — batch
  partitioning is redundant with delivery bursts), concrete timing,
  and emission linearizations.
- **Draft 3 (validated claims) died on the two-liner**: sharing the
  `Values` decision vocabulary and having the machine *validate*
  quotient-typed claims against its buffers would have kept a `∀ d`
  theorem, but put quotient-flavored logic inside the machine — the
  very thing whose absence makes the machine auditable. The
  alternative (deriving denotational decisions from the machine flow)
  breaks the definitional projection `P@Corr → P@Values`, which
  tagless-final cannot repair (no induction over program syntax).
  Resolution: **instance-declared decision types** (the `…Dec` fields
  on `HydroSem`) — each interpretation names its own nondeterminism,
  with sites erasing to `Unit` exactly where their freedom lives on
  the other side — plus **`RelSem`**, the set-valued ∃-packaging
  instance (ops = images of `Values` ops over their decision spaces,
  composition = the nondeterminism monad's bind), so the transfer
  statement quantifies `∀ machine run ∃ denotational run` *without
  naming a global decision assignment*. Headline transfer stays
  `d`-free (the Paxos safety statement is `∀ d`).
- **`EmitDec` is a discovery about Rust, not a modeling artifact**:
  `emitMultisetBatches` is the one site where a program puts *computed
  unordered state* (Lean `Multiset` = Rust `HashMap`) on a wire, so
  wire order is genuinely born there — it is exactly
  `flatten_unordered`'s hashmap-iteration nondeterminism, and it
  cannot ride an ambient environment because it selects typed content
  (the original reason Nat-encoded schedules died). `Vec`-backed state
  uses `emitBatchesUnordered`: deterministic, decision-free, only the
  *type* forgets the order.
- **Totality is a two-sided cheat detector**: `CorrSem` fills a
  coupling field for every op. Unfillable means either an op's
  `Values` semantics observes something the machine cannot provide,
  or the denotational decision space cannot cover a real interleaving.
  It fired once in anger: per-member `freezeView` at tick→stream
  boundaries made three couplings unfillable (members could freeze at
  different raw steps while the witness is one family-level run) —
  fixed by **family-atomic freezing** (`famFreeze`), whose
  `famFreeze_eq_raw` restores "every frozen view is one raw view".
- **Per-step-∃ coupling is what lets cycles couple**: every carrier
  relation is `∀ step, ∃ v ∈ V, …` — step `t` couples to the
  depth-`t+1` Kleene iterate; reads that assemble machine state across
  steps (snapshots) recover a single witness through the stream
  carriers' bundled monotonicity. Raw (unbundled) tick carriers made
  most tick-op couplings proof-free.
- **Adequacy is false, by design, and saying so is the finding**:
  there is no `∀ d ∃ schedule` theorem —
  `emitBatchesUnordered → assume_ordering` is the counterexample (the
  machine realizes one emission order per schedule; `selectOrder`'s
  space licenses every permutation; intra-sender FIFO likewise
  unreachable). The quotient deliberately over-approximates: safety
  needs machine ⊆ denotation only, and the surplus strengthens program
  obligations (robustness to weaker transports). What *is* true and
  shipped: **tightness** (∀ schedule ∀ horizon, decision-mediated
  observations are *equalities* at derived decisions — truncation *is*
  the decision at consumption sites) and **per-wire end-of-time
  equality** (`StabilizesAt`: stabilized wires attain their pools
  exactly; cycles under the per-program Kleene-fixing hypothesis;
  deliberately no global quiescence predicate — a partially-churning
  program keeps equality on its stabilized wires). Existence of a
  denotation does *not* imply the machine stops (`Values`' fuel is a
  truncation decision — the forever-counter has denotations at every
  fuel and no end of time); the implication runs machine-stabilizes ⇒
  derived fuel is a true fixpoint (`iterate_stab_of_fixed` makes the
  equality against *the* denotation).
- **The fixpoint race is a real behavior, executably witnessed**
  (`TransferChecks.lean`): a stalled elder message is overtaken by the
  offspring of a younger one through a knot, and the raced order is
  covered by the denotational fixpoint — plus interleaving/latency/
  FIFO/stutter-tick/sampling-birth witnesses, and the snapshot pair
  (pacing captured, micro-order quotiented).

### D31 — time in the step machine: elastic above, floored below

The transfer work forced a precise answer to "what is a step?", and
the answer became load-bearing. The machine's clock has **idle steps**
(legal, observationally invisible — observations are tick- and
content-indexed, never step-indexed) and **two structural floors**:
network delivery takes at least one step (`StepHist.deliver` exposes
the sender's wire as of `t − 1`), and a `fix` knot's feedback edge
takes at least one step (`StepHist.shift` at `fix_stream`; an implicit
defer at `fix_tick` — hydro forbids synchronous in-tick cycles, so the
machine says so structurally rather than relying on each program's
`defer`).

- **Elasticity above is adversary coverage**: async ticks and member
  asymmetry are per-location idling; unbounded lateness is idle steps
  on a path; granularity refinement (uniform idle-step insertion) is
  why the canonical within-step fan-in order and the floors lose no
  real behaviors. Every witness `#guard` passed unchanged when the
  floors landed — floors bound *below*, the adversary was already free
  *above*.
- **The knot floor is well-definedness — and it is the one that is
  needed**. Without it, Zeno executions (multiple loop passes in zero
  time) make the Kleene diagonal's value at a step depend on unfolding
  depth without bound; the concrete symptom in the transfer was that a
  consumption site inside a nested knot derived *conflicting*
  decisions across Kleene stages, making the derived-decision record
  unsatisfiable. With the floor, `iterate (t+1)` determines everything
  visible at step `t`, for arbitrary bodies — stabilization-in-depth
  became a structural fact instead of a per-program guardedness proof
  (which ruling 4 forbids). Note the precise reading: the *feedback
  edge* costs the step, not the body — one full dataflow pass per step
  is legal; content just cannot re-enter the loop within the step.
- **The network floor is NOT needed for safety soundness.** Every
  cycle in the signature passes through a `fix` op (tagless-final
  terms have no other back-edges), so the knot floor alone yields
  stabilization-in-depth; no transfer proof uses the network delay.
  It is kept for what a step *means*: (1) the **Lamport property** —
  with it, every remote hop costs a step, so step count dominates
  causal depth globally (without it, an acyclic chain `A→B→C→D` can
  cross three machines in one "step", and the step is just a scheduler
  tick with no causal reading); this makes the step clock a usable
  ruler for future latency/liveness/complexity reasoning over the same
  machine. (2) Cheaper causality bookkeeping in the transfer kit
  (agreement horizons advance through network paths — convenient, not
  load-bearing). (3) Physical honesty for the eventual
  compiled-implementation refinement: a real runtime never delivers in
  the same instant, so the machine that already says so is the closer
  refinement target. It costs nothing: a floor under an
  already-arbitrary cursor.
- **Quiescence recast**: "no more work" = all remaining steps idle, so
  the end-of-time equality reads *once time stops mattering, the
  machine is its denotation*. Breaking the two sides fails
  differently: remove idle steps and coverage dies (asymmetry/latency
  inexpressible); remove the floors and `fix` is ill-defined.

### D32 — the square: closing the transfer at the `∀ d` contracts

The transfer's last mile — "machine run ⊑ *some* denotational run, and
the `∀ d` contract applies to it" — died twice (set-valued couplings
decorrelate diamonds; flow-derived decisions break the projection to
`P@Values d` absent term induction) before landing on the **square
coupling** (`Square.lean`): carriers pair a *machine stage-pair*
(agreement below a cutoff — guardedness as per-op cutoff arithmetic,
raised by the structural floors) with a *reader pair* `D → Values`
(`PoolLe` — the Kleene chain from per-op monotonicity, never per-knot),
under a decision-condition family δ with extension-inequality atoms at
consumption sites (derived decisions are horizon-monotone, so nested
stages never pin one lens to conflicting values — the flat-record
conflict that killed the previous draft).

- **Both projections are theorems, not hypotheses**: the reader leg at
  `d` *is* `P@Values (fields of d)` and the machine leg *is*
  `P@SchedSem`. Naively these are `rfl` through both knots — but the
  kernel cost is (program size) × (instance size) × (knot-unfolding
  duplication): the monolithic `rfl` at `paxos_core` scale ran 280+
  CPU-minutes without finishing. The fix is **congruence assembly**
  (`SquareSafetyId.lean` + `SquareSafety.lean`): restate the loop body
  at top level (`pcBody`, pinned to the program by a *variable-
  instance* `rfl` — cheap, nothing evaluates), discharge six knot-free
  body identities by `rfl` (~40 min total, the honest per-program
  price of tagless-final parametricity), then close both knots with
  generic `rfl` lemmas (`sq_fix_stream_rr/sr`, `sq_fix_tick_rr/sr` —
  free at *variable* body) and `iterate_congr`/`congrArg`. The kernel
  never unfolds a knot at a concrete instance. The reader is the
  correlation mechanism: a diamond shares its reader, so both
  consumers see the same denotational run at every `d` — the
  nondeterminism-monad idea with the monad in the *carrier*, not the
  denotation.
- **`fix` needs no program facts**: the field iterates a shifted
  square whose stage-pairs advance in lockstep — the subtype's own
  `mono` *is* `F^m ⊥ ⊑ F^(m+1) ⊥` — with freeze-picks obtained
  generically, fuel pinned by inequality (lifted along the chain), and
  `rfl`-certifiable bridges in δ. Unfillable δ never blocks totality;
  it blocks satisfiability, per-site — the cheat detector again.
- **The closing theorems**: `cq_safe_sched` (SquareTheory.lean) — every
  key the step machine's `collect_quorum` emits, under any pacing/
  schedule/horizon, carries `min` Ok-votes among the derived decision's
  consumed pool; seven lines, witness = `batchDerive`, satisfiability =
  `⟨trivial, prefix_refl⟩`. `paxos_safe_sched` (Paxos/SquareSafety.lean)
  — any two commits the step machine's `paxos_core` emits at one slot
  agree, under intersecting quorums, given a δ-satisfying environment;
  the body is couple → name (the projection theorems) → headline →
  transport. δ-satisfiability for knot-free programs is reflexive; for
  knots the *structural* part (stage telescope, fuel floor, reader
  bridges, machine bridge) is discharged by the closed witness
  `sq_fix_dlt_nonvacuous` (SquareNonVacuity.lean), and the per-op part
  is extension atoms `lens d ⊒ derived flow` — satisfiable by the
  record of derived values by construction. The paxos-scale witness
  record (kernel-heavy, same cost class as the identities and amenable
  to the same staging) is the identified follow-up.

### D33 — kill the per-program kernel work: reducibility hygiene + a per-op projection simp set

D32 shipped the square with an honest debt: the projection identities
were discharged by restating `paxos_core`'s loop body at top level
(`pcBody`/`pcALogF`/`pcSeqMax`) and kernel-`rfl`-ing six body-level
identities (~42 min wall / ~102 CPU-min, all of it in the three
machine-leg identities), assembled by 25.6M-heartbeat hand-built
congruence chains — per-program proof machinery duplicating the
program text. Replaced end to end by `SquareProj.lean`: per-**op**
projection `rfl` lemmas (each proved once at variable arguments), and
`sq_transfer [<def names>]` = `simp only` through the program text +
`with_reducible rfl`. `paxos_sq_rr`/`paxos_sq_sr` are now one tactic
call each; **`SquareSafety.lean` elaborates in 16 s** and the restated
bodies and `SquareSafetyId.lean` are gone. The per-program content of
a machine-run safety theorem is the list of its definition names.

- **The blowup was never "the kernel is slow" — it was reducibility
  hygiene.** `simp`/`rw` match up to *reducible* transparency and
  refuse targets that are not type-correct at that level. The three
  instances are semireducible defs, so one type has many
  reducibly-distinct spellings (`SqStream D (mem ℓ) …` vs
  `(SquareSem …).Stream ℓ …`; `Unit` vs `(SchedSem …).BatchDec …`),
  and ANY subterm carrying the wrong spelling — an inline proof arg
  typed up to delta (`fun _me h => h` at `ValueOrder.nat.le` where
  `Ballot.numVO.le ⟨a,i⟩ ⟨b,i⟩` is expected), a record-projected
  decision, a raw-typed embed — blinds every keyed rewrite above it.
  One blind node strands its whole subtree at the square instance, and
  the closing `rfl` silently degenerates into whole-program defeq:
  the 40-minute mode. The same friction was the source of the sus
  `maxHeartbeats` bumps in the hand-built assembly. The discipline
  (now in `SquareProj.lean`'s docstring): spell every binder at the
  instance projection the program produces; declare embeds/inputs at
  those types; make obligation vocabulary consumed by op arguments
  `@[reducible]` (`FoldOkP`, `ValueOrder.nat/multiset`,
  `Ballot.numVO/obtVO` — attributes only, no text changes); route
  decision records through typed constructors (`SqDec.*`) that the
  decision-carrying op lemmas match on; hand custom `DecidableEq`
  wrappers (`p1bPairDecEq`) to the transfer call because simp
  re-synthesizes instance args when instantiating lemmas; and close
  with `with_reducible rfl` so a lemma that failed to fire FAILS FAST
  instead of paying the defeq.
- **Debugging method worth keeping**: the residual after `simp only`
  is computable in seconds even when the closing `rfl` runs for an
  hour — dump both sides with `pp.explicit`, tree-diff the token
  streams, and every stuck node names its own missing lemma or
  wrong-spelled binder. Five iterations of that loop replaced weeks of
  guessing.
- **Program text: zero changes.** The Rust-mirror modules are
  untouched (the one `show`-ascription tried during diagnosis was
  reverted once the vocabulary attributes made it unnecessary).
  `paxosSqDec` (the lens record) now spells its fields through
  `SqDec.*` — same data, type-stable faces. The
  `paxos_safe_sched`/`paxos_sq_*` statements are unchanged in meaning;
  input premises are typed at `(SchedSem …)`/`(Values …)` projections
  (definitionally the same carriers, and the honest reading: a machine
  wire coupled below a denotational pool).
- Axiom audit now covers the closing theorems themselves
  (`paxos_sq_rr/sr`, `paxos_safe_sched`, `cq_safe_sched`): standard
  three.

### D34 — the `Eager` interpretation: execution is a representation, not a semantics; V1 retired onto V2 ports

**Context.** The C2 mandate (port falsify/explore/2PC to V2, retire
V1's `Hydro/`+`Programs/`) hit the execution wall: the B1 falsification
(2 proposers × 3 acceptors) would not finish in 30 minutes at
call-by-name `Values`, even with the program loop bodies hoisted to
named top-level defs so the IO harness could run the *same* defs the
program closes its knots over (no restated wiring). That hoist was
**reverted** once `Eager` landed — with harness-free thin mains nothing
consumes the names, and the Paxos program files return to their
original (pre-hoist, `let body`-inline) text.

**The user's diagnosis, confirmed**: `Values` should be the *fastest*
semantics to execute — decisions-as-inputs means no runtime search, and
the commutative-fold seams are site-local (`FoldOk` is exactly the
license that representative order doesn't matter; snapshot observes
decision-chosen cut chains, not machine paths). The slowness was purely
representational: `Fin n → …` closure carriers + no compiler sharing
(D14) compound re-evaluation multiplicatively. A step machine would be
*slower*, not faster (per-step work + diagonal fix).

**The fix — `Eager.lean`**: pair data with denotation in the carrier.
`EPack n C = {data : Vector C n, den : Fin n → C, agree : ∀ i, …}`;
every op = the `Values` op applied twice (once over materialized
inputs, once over denotation legs — zero re-spelled formulas), so the
uniform `mk_agree` family discharges every agreement and the fix knots
are data loops with one generic induction. **Per the user's ruling, a
new instance kind requires generic correspondence theorems** —
`EagerProj.lean` (per-op `rfl` den-projection lemmas + `eager_transfer`
macro, the SquareProj pattern with the D33 hygiene: instance-typed
embeds, decision binders at `(Values …).XDec`) assembles per-program
pinning identities (`paxos_eager_den`/`_commits`, ~11 s) so the
executed data provably IS the `Values` run — the interpretations cannot
silently diverge.

**Results**: v2paxos 120 s → **0 ms**; B1 falsify (both variants,
2×3) **5 ms**; explore (acausal starvation + causal healing) and
v2twopc (2PC commit-iff witness) all thin mains at `Eager`. B1/E1
decision scripts ported to the V2 vocabulary (`Falsification.lean`,
`Exploration.lean`) reproduce the V1 records exactly (faithful
double-commit, guarded blocked; starvation; healing at slot 1).

**Gotcha for script writers**: `SnapDec` cuts are per-tick
*increments* (`snapshotCuts`/`prefixCuts` add to an accumulator and
truncate on overflow), not cumulative positions — V1's cumulative
spelling silently truncates the trace.

**2PC port** (`TwoPC.lean`): the Rust fn over the shared
`Std/Quorum.lean` stage; colocated contract = the two `CQEnsures`
faces at the canonical fan-in pools (which the `Values` wires equal
definitionally); theory = `two_pc_unanimous`, `two_pc_once`,
`two_pc_commit_iff` (the V1 master theorem, iff-shaped) by multiset
accounting.

**V1 retirement**: `HydroLean/Hydro/` + `HydroLean/Programs/` deleted
(recoverable from jj history); `Flo/` (ch. 2), `Gyatso/` (ch. 3) and
their `Collections/` substrate remain; README/ACCEPTANCE/SORRIES/docs
carry pointers from the historical paths to the V2 homes. New exe
`v2twopc` joins the gate.

### D35 — per-member tick pacing: independence across cluster members is the model, not a mitigation

**The gap** (found while writing CORRESPONDENCE.md's Sched audit
section): the tick skeleton was per *location* —
`pacing : L → Nat → Bool`, every op consuming
`tickSteps (pacing ℓ)` uniformly over `i : Fin (mem ℓ)` — so all
members of a cluster shared one skeleton. Joint behaviors like
"member A ticks at step 3 while member B has *no* tick entry at 3"
were unreachable, and tick-*counting* ops (`timeout_snapshot`,
`source_interval_batch` — both take `(tickSteps …).length`) make tick
presence semantically visible, so the gap was real, not
presentational. User ruling (verbatim): *"oh yeah presence of ticks
should definitely be independent across cluster members, of course!"*

**The fix**: `pacing : (ℓ : L) → Fin (mem ℓ) → Nat → Bool` across
`SchedSem`/`CorrSem`/`SquareSem`, with the transfer chain re-proved.
The change is almost entirely mechanical *because the architecture
already paid for it*: the member index `i` is in scope at every one of
the five consumption sites (`snapshot`, `batch`, `batch_ordered`,
`timeout_snapshot`, `source_interval_batch`), so each becomes
`tickSteps (pacing ℓ i)`; the derivation functions
(`batchDerive`/`batchOrdDerive`/`snapDerive`) and the per-member
correspondence lemmas (`corr_snapshot`/`corr_batch`/
`corr_batch_ordered`) generalize `p : Nat → Bool` to
`p : Fin n → Nat → Bool` and use `p i` — call sites keep their
spelling (`batchDerive (pacing ℓ) …`). The single non-cosmetic
adjustment: the square tick ops' `cut` bound now quantifies over
members (`∀ i t, ((tickSteps (pacing ℓ i) t).take n).all (· ≤ c)`),
and `agree` consumes it at its own member. `HydroSem` itself is
untouched (pacing was never in the signature); `Values`/`Eager` and
every program module are pacing-free — zero program-text changes;
`paxos_safe_sched` got strictly stronger with an unchanged body
(the `sq_transfer` projection lemmas are pacing-generic).

**Reproducer** (`TransferChecks.lean` §9, #guard-enforced): two
members of one cluster on different skeletons — at global step 1,
member 0 ticks *with content* (`batchesSkew 0 1 = [[], [10]]`) while
member 1 has no tick entry at that step at all
(`batchesSkew 1 1 = [[]]`), and one pulse list consumed per member
skeleton yields different tick counts (4 vs 2 by step 4).

**Moral**: "ambient parameter all sites share" is a smell worth
auditing whenever the sharing scope is coarser than the unit of
concurrency; and per-member quantifiers cost nothing when every proof
is already per-operator and per-member — the expensive version of this
change would only have existed if there had been per-program proof
machinery to re-prove.

### D36 — the Sched red-team audit, and the knot-δ bridges made generic (F1 Stage A)

**Context.** A systematic red-team audit of the step machine
(`HydroV2/SCHED_AUDIT.md`) was commissioned after D35 ("ambient
parameters coarser than the unit of concurrency are a smell" — swept
here in full). Verdicts: the D35 granularity class is clean across
every decision family; delivery/tick/floor/duplication semantics are
faithful to the Rust runtime (fail-stop TCP only — a scope condition
to state, not a bug, since every mirrored-program edge is
`TCP.fail_stop`); and the two real findings are **statement-level
Lean holes, not modeling holes**: (F1) `paxos_safe_sched`'s `hsat`
premise was discharged nowhere for knotted programs — if any knot
bridge were false for the real body the theorem would be *vacuously*
true at every schedule; (F2) the input-coupling premise's quantifier
order excludes unbounded client streams. The audit's moral extends
D35's: **auditing the trusted statement means auditing its premises'
dischargedness, not just its definitions' fidelity** — zero-sorry,
gate-green trees can still carry a vacuous headline.

**F1 Stage A (this tree).** The knot combinators the δ names
(`shiftC`, `stages`, `vbody`, `sbody`, seeds/probes) are hoisted to
top-level definitions; `SquareKnot.lean` proves the **reader bridges
and machine bridge hold for all decision environments** given three
per-knot naturality identities, each `sq_transfer`-provable at a
*variable* stage carrier `C` — so δ of a knot ≡ stage telescope +
fuel, and two of its four components became theorems. Validated
end-to-end on an op-built knot (`sq_fix_dlt_nonvacuous_ops`: full δ
discharged at every schedule/horizon). New D33-class lesson: simp's
metavariable **assignment** also type-checks at reducible
transparency, so lemma binders must exist at both the
instance-projected *and* the raw carrier spellings when goals mix
them (the `_raw` embed-collapse twins; and the `_rl` mirror family —
every square op treats `rl` exactly as `rr`, 55 generated `rfl`s).

**Remaining (F1 Stage B/C, queued).** The telescope's extension atoms
chain into one derived-value witness only through machine-op
**causality** (views at `t` depend on views `≤ t`) — best built as a
relational instance in the CorrSem pattern (totality = cheat
detector), then `∃ d, hsat` for the Paxos square and a premise-free
`paxos_safe_sched'`. F2's fix (per-horizon coupling premise or a
causality corollary) falls out of the same lemma family.

### D37 — the knot δ's left reader bridge was unsatisfiable for nested knots (F1 Stage B: the vacuity was real, and is repaired)

**The find.** Building the F1 Stage-B validation ladder (single knot →
nested knots) forced the first-ever attempt to *inhabit* a nested
knot's δ — and it cannot be done: the fix ops' δ carried, per stage
`m`, the bridge `(stages m).rl d = iterate (vbody d) ⊥ m`, where
`vbody` probes the body with **both** readers pinned (`readEmbed`), so
the right-hand chain reads captured wires' `.rr` while the stage's
actual `rl` leg reads captures' `.rl`. An outer knot's stage `rl`/`rr`
are *consecutive Kleene iterates* — different until convergence — so
for any inner knot capturing an outer knot's wire (the paxos shape:
`alogF` captures `sm`; `leader_election`'s knots capture `p2b`) the
inner left bridge at `m = 1` demands `(outer stage k).rl d = (outer
stage k).rr d`, false at `k = 0` for any nonempty input. The fuel
floor forces `m = 1` into range. Hence `dlt T d` was **False for
every `d`, every schedule, at every `T ≥ 1`** — `paxos_safe_sched`
was vacuously true at every interesting horizon, in a zero-sorry,
gate-green tree. Exactly the failure mode `SCHED_AUDIT.md` F1 warned
about, surfaced only because something finally tried to *prove* the
premise. Moral (sharpening D36's): **an undischarged premise is not
just missing evidence — it can be false**; inhabitation witnesses are
load-bearing, not ceremony.

**The repair** (user-ratified). The left bridge's only consumer was
the fix `cpl`'s chain-gluing step (stage-mono → Kleene-chain-mono).
The δ component is replaced by what that step actually needs, a
Values-only **Kleene chain condition**
`∀ m, m + 1 ≤ df d → iterate (vbody d) ⊥ m ⊑ iterate (vbody d) ⊥ (m+1)`
(the right bridge stays; the `rr` leg — the Values run that
`paxos_sq_rr` names — is untouched, so `paxos_safe_sched`'s statement
is unchanged; only its δ became inhabitable). The chain condition is
discharged by body monotonicity — `iterate_le_succ` /
`iterate_mono_param` over the per-op reader-mono lemmas
(`batchCuts_le`, `sum_le_sum_of_prefix`, `Multiset.add_le_add_*`), the
MonoRel-style machinery the tree already had.

**Stage B machinery (this tree).** `HydroV2/SchedCausal.lean`:
view-agreement relations per machine carrier shape + **causality
congruence lemmas for all ~40 `SchedSem` ops** (heterogeneous — two
composites, different leaf wires — so one family serves stage-vs-stage
and stage-vs-diagonal uses; totality = cheat detector) + stabilization
roots (shift-iterates of causal bodies are view-stable below their
index). `HydroV2/SquareDlt.lean`: 52 `rfl` δ-projections (the op-by-op
dlt unfolding kit; only four op families carry site-wire atoms; timing
ops pin by equality to machine data). `SquareKnot.lean`: derive
congruence + **chain lemmas** (a stage-stabilized wire family's atoms
all sit below the top derive) + stage stabilization + telescope
introduction. Validated: `sq_fix_dlt_exists_batch` (single knot with a
batch atom) and `sq_fix_dlt_exists_nested` (fix-inside-fix with
capture-crossing stabilization, inner telescope threading the outer
stage's δ via `dmono`, one witness pinning both fuels and the lens **by
unification**) — `∃ d, dlt T d` at every schedule and horizon, no
per-knot lemmas.

**Elaboration lessons.** (a) Naturality `have`s must be established
*before* the witness metavariable enters the goal — simp will not
rewrite under unassigned holes; (b) `apply Exists.intro ((_, T+1))`
yields assignable metavariables — named `refine` holes are
synthetic-opaque and unification cannot pin them; (c) `SqDec.*`
constructors are `@[reducible]` so pin unification sees through the
lens application; (d) inline `(by omega)` inside applied terms can
elaborate against unreduced expected types — hoist to a tactic-mode
`have`; (e) decision-record component types must be spelled at
instance projections or every projection lemma goes blind.

**Remaining (Stage B completion + Stage C).** The per-program assembly
at paxos scale (5 nested knots, ~20 sites) needs the `sq_hsat`
elaborator macro — the validation theorems are its hand-written
blueprint; then `paxos_hsat : ∀ sched T, ∃ d, dlt T d` and
`paxos_safe_sched'` with the `hsat` premise gone. F2 dropped per user
ruling.

### D38 — instance-generic `fix` bodies (curried captures): programs answer the knot-body-opacity problem; tactics stop carrying semantic weight

**Why the syntactic assembly was retired mid-flight.** The `sq_hsat`
route (D37's plan) was built end to end — orchestrator macro
(`SquareHsat.lean`: witness-skeleton entry, top-level atom pinning by
unification, telescope recursion, goal dedup by proof-sharing) plus
two per-op walker tactics (`sched_causal` over the `SchedCausal.lean`
congruences with raw-spelling knot-leaf lemmas
`stages_sr_fix_hetero`/`causal_sq_fix_*_raw`; `values_mono` over a new
per-op ⊑-preservation family, `ValuesMono.lean`) — and its cheap parts
measured fine (δ-normalization + splitting + pinning: **2 s** on the
single-knot validation). But the walkers kept hitting D33-class
elaboration cliffs (whole-instance `whnf` inside failing
`first`-alternatives; 3-minute runs on a TOY knot even after
`with_reducible` discipline, stray-goal-pinning and `R`-inference
fixes). The user challenged the premise — "shouldn't the requirements
be dispatched over the body / can't the body be generic over the
instance, since the enclosing function already is?" — and that
diagnosis is exactly right: **`fix` is the one op whose semantics
quantifies over a program fragment**, and every hard δ component is a
theorem about that opaque body (machine-leg causality for stage
stabilization, `Values`-leg monotonicity for the Kleene chain,
leg-naturality for the bridges). Walking the body's syntax re-derives
per body what one instantiation could project.

**The pivot (user-designed, landed, gate-green).** `HydroSem.fix` /
`HydroSem.fixTick` (`Sem.lean`): *derived* combinators — a signature
field cannot quantify over `HydroSem` itself (non-positivity), so the
generic shape lives program-side —

    H.fix df (caps : Γ H)
      (body : ∀ H', Γ H' → H'.Stream … → H'.Stream …)
      = H.fix_stream df (body H caps)

Closures keep capturing local wires and decisions; the capture tuple
is just *explicit* (`Γ` is any program-chosen family, so `LEDec H'`
etc. curry through). All five paxos knots rewritten
(`leader_election`'s fails/iam/ffx — whose pass was *already*
`H'`-generic for `MonoRel`, so this formalizes the house style —
and `paxos_core`'s alogF/seqMax). Every existing proof survived:
K4/`paxos_core_agree` untouched (2 s), LE's colocated contract +
binary mono needed only mechanical prefix edits, `paxos_sq_rr/sr` and
the eager identities re-closed by adding the two def names to the
transfer lists. Full gate re-verified (796 jobs, zero sorries,
standard three, all four exes). Gotchas: the caps-tuple binder in each
knot body needs an explicit type ascription (`Γ` does not infer
through the tuple before the body elaborates); `SquareSafety`
elaboration regressed 16 s → 156 s because `sq_transfer` now churns
through the `HydroSem.fix` delta + tuple projections — restore by
adding `HydroSem.fix`-keyed lens projection lemmas to
`SquareProj.lean` (match the wrapper spelling directly instead of
unfolding it).

**Findings for the successor (the proof half).**
- A `CausalRel` gluing instance (pairs of `SchedSem` carriers +
  view-agreement-below-`h`; op fields = the `SchedCausal.lean` lemmas)
  is buildable including `fix` (relational-diagonal construction,
  agreement internal via `famFreeze` congruence) — **but its `fix`
  projections are not definitionally the `SchedSem` fixes** (the
  identification needs induction over a symbolic iterate index, which
  kernel defeq cannot do). Consequence: op-level causality comes free
  from the instance; **knot-level gluing stays per-knot** (the
  `causal_fix_*` congruence lemmas — they exist).
- That is exactly how `MonoRel` is already consumed (`leader_election`
  instantiates the knot-free pass at `MonoRel` and glues its three
  knots manually with `iterate_mono_param`). The architecturally
  consistent completion: **colocate a causality clause — and export
  the knot-body monotonicity facts — on each program module's
  contract**, proven with `body @ CausalRel` + per-knot gluing,
  mirroring the existing colocated mono clauses (the knot bodies are
  `let`s; colocation is where they are in scope). With those clauses,
  `hsat`'s per-knot obligations become projections (chain ⟵ mono
  clause, stabilization ⟵ causality clause, naturality = one
  `sq_transfer` each) feeding the fast pinning macro; the syntactic
  walkers become unnecessary at program scale (they remain valid for
  small/knot-free uses, as does all the lemma content:
  `SquareHsat.lean`'s stage-vs-output stabilization,
  `famFreeze_eq_of_ascending` — which discharges audit finding F3 for
  causal bodies — atom chains, `cutle_refl`, `dedup_goals`, and
  `ValuesMono.lean` wholesale).

**Moral** (extends D33/D37): tactics may *assemble* once-proven
per-op facts, but the moment a tactic starts re-deriving a semantic
property of a program fragment, the fragment should have been a named
instance-generic object instead. Tagless-final gives reflection for
free — but only for the fragments the program names.


### D39 — the coupling corner: derived decisions replace δ-satisfiability; the transfer premise dissolves into the carrier

**Finding.** The Square/δ architecture (D30–D37) made the
Sched→Values transfer *conditional*: the headline needed an `hsat`
premise ("the δ-stack is satisfiable at this schedule and horizon"),
discharged per-program by walking the program text. That walk is the
same trap D33/D38 warn about — a tactic re-deriving a semantic
property of a program fragment. The repair is to stop *checking* that
suitable `Values`-decisions exist and have the interpretation
*construct* them: a fourth interpretation, the **coupling corner**

    CoupleSem L mem pacing Tc Td (hjT : Tc ≤ Td) : HydroSem

whose carriers are corners `{sr // machine leg (SchedSem verbatim)}
× {rr // plain Values leg} × (wf : Prop) × (cpl : wf → coupling at
horizon Tc)`. Decision-mediated ops **derive their own `Values`
decisions from the machine leg** (`batchDerive`, `ordSelDerive`,
`snapDerive`, … at horizon `Td`); timing/cursor/emission sites take
machine data; content sites take `Unit`. Each op's `cpl` field is its
realization theorem *at its own derived decision* — no decision
environment, no lens records, no legality atoms, no δ. Running a
program once at the corner yields, structurally: the machine run
(`paxos_co_sr`, a projection identity), a `Values` run at
program-shaped derived decisions (`paxos_co_rr`, packaged as `∃ d,
rr = Values-run at d`), and a coupling between them (`cpl`), so the
headline becomes **premise-free**: `PCEnsures` at `d` (∀-quantified,
so any `d` works) + `cpl` + `Multiset.mem_of_le`. Satisfiability is
not a proof obligation anywhere; it is a *definition* with a
`corr_*` theorem per op — exactly the D38 moral applied to the
transfer itself.

**Knots.** `fix` is where δ died (D37) and where the corner is
novel. The machine leg is a *named* diagonal (`CoStream.fixSr body` =
history of shift-iterates); the `Values` leg is the plain Kleene
iterate of the body's `rr`-projection along a probe carrier
(`probe2`) that carries **the knot's own machine diagonal** — so a
decision derived at a site *inside* the knot body is spelled
identically to the same site's decision at top level (pin
consistency; the "diamond after nondet" concern). `wf` for a knot is
three program-facing facts: machine-body causality (`hcaus`), a
Kleene mono ascent (`hchain`), and a **graded coupling** `hcplj : ∀
j ≤ Tc, probe legs coupled below j → body legs coupled at j`. The
generic knot theorems `co_fix_cpl`/`co_tick_fix_cpl` (choice-free:
`[propext, Quot.sound]`) close the loop by induction on the horizon
via the machine fixpoint equation — no stages, no telescope. And
`hcplj` is where D38's `H'`-generic knot bodies pay off completely:
instantiate the *same* body at the lowered corner `CoupleSem j Td`,
and its carrier-borne `cpl` **is** the obligation, glued by two
`co_transfer` identifications (machine legs and `rr` texts are
horizon-independent below `Tc`). Validated end-to-end at knot scale
(`CoupleCheck.lean`: `cc_sr`/`cc_rr`/`cc_wf`/`cc_cpl` over a
batch-inside-fix blueprint).

**Perf half (the D38 regression class, finally root-caused).**
`paxos_sq_sr`/`paxos_co_sr` cost ~140 s *each*, and profiling shows
it is **pure kernel defeq** (simp 6 s, kernel 135 s; plain `rfl`
costs the same): the D38 capture tuples substitute proof-carrying
carrier literals through knot bodies and the kernel re-normalizes
the program text quadratically. Not a corner flaw — the corner
merely inherits it at whole-program scale. Two corner-specific
quadratic sources were designed away first (derived decisions must
be spelled off the *named* machine wire, `VDec.*D x.sr`, never off
its normal form; the knot diagonal must stay folded as `fixSrC` on
the `Values`-naming side). The committed fix for the rest is
**module-wise naming lemmas**: prove `<mod>_co_sr`/`<mod>_co_rr`
per program module over its own body (~2 s each; `pbc_co_sr`
measured), then assemble the paxos-level identities with modules
*folded* — kernel cost linear in module count. This also retires
the walker cliffs: the re-enabled δ-hsat validations in
`SquareHsat.lean` hit `whnf` heartbeat cliffs from the same kernel
class and are disabled in favor of the corner (walker infra remains
in-tree).

**Gotchas** (extending the D33–D38 ledger): simp lemmas with an
*unused* section-variable binder are silently skipped
(unassignable metavariable — `SquareProj`'s `sched_hfix` bug);
metavariable pins typecheck at reducible transparency, so raw `()`
literals at instance-typed positions blind the match (pre-fill
`Unit` fields — the `values_*_unit` collapses erase their
occurrences before pinning — and route non-unit pins through
`@[reducible]` wrappers); under-applied top-level defs do not
simp-unfold (η-expand knot bodies at check sites); `cases` on a
horizon that parametrizes carrier *types* fails to generalize
(hoist to standalone lemmas over the horizon).

**The naming architecture, revised in-flight (the pinned-∃ wall).**
The first plan read the derived decisions off by *unification*: per
module, `∃ dV, corner.rr = Values-run at dV` with the record fields
pinned by the final `rfl`. This works at knot-blueprint scale
(`CoupleCheck.lean`) and for the knot-free modules (`CoupleStd.lean`,
`CoupleModules.lean`: `collect_quorum`, `collect_quorum_with_response`,
`join_responses`, `p_ballot_calc`, `p_leader_heartbeat`,
`acceptor_p1`, `acceptor_p2`, `p_p1b`, `recommit`, `index_payloads`,
and all of `sequence_payload`) — with two new tools: quantifying the
∃ over the *machine legs only* (hypotheses `wire.sr = x` rewritten
before the pin, so the choice-named record is horizon-generic), and a
`rw`-fixpoint loop, because **simp will not rewrite a module
projection nested inside another module application's argument** —
the colocated-contract subtypes make every wire argument a
dependently-typed position, and simp's congruence machinery silently
treats the whole application as atomic there, even though the very
same lemma instance `isDefEq`s at *reducible* transparency
(diagnosed with DiscrTree `getMatch` + manual `isDefEq`; `rw`'s
default-transparency `kabstract` reaches these spots fine, but cannot
reach under binders). At `leader_election`'s three knots both tools
die together: the canonicalized machine diagonals blow the pipeline
past 25M heartbeats, and the in-knot occurrences sit under binders
where only simp — which stalls — could rewrite.

**The resolution: name the decisions, don't pin them**
(`CoupleDec.lean`). The corner's derived decisions are an *explicit*
program-shaped record: `paxosVDec` (with slices `leVDec`, `spVDec`)
replays the program's wire graph verbatim at the corner — module
bodies' `let` chains copied with submodules folded, knots as the
corner `HydroSem.fix`/`fixTick` wrappers — and applies
`batchDerive`/`batchOrdDerive`/`snapDerive`/`ordSelDerive`/fuel
`Td + 1` at each content site's machine leg (`.sr`). No ∃, no
metavariables, no choice, no tactic normalization: the naming
identity is a **plain kernel `rfl`** (the `paxos_co_sr` cost class),
and the ∃-shaped corollary is `⟨paxosVDec …, rfl⟩`. Machine-leg
namings for the knot modules are the same story (`le_co_sr₁`–`₄`:
plain `rfl`, ~4–6 min kernel each).

**Status.** In-tree and green: `Couple.lean` (the interpretation +
generic knot kit), `CoupleProj.lean`, `CoupleCheck.lean`,
`CoupleStd.lean` + `CoupleModules.lean` (module namings as above),
`CoupleDec.lean` (the explicit decision record). The whole-program
`paxos_co_rr` as a single kernel `rfl` turned out to be **impossible,
for a quantifiable reason** — see D40, which replaces the whole-program
defeq strategy (including the interim ~140 s `paxos_co_sr` `rfl` and
the ~4–6 min/knot `le_co_sr` `rfl`s described above) with per-knot
structural namings; both headline identities now land in ~3 s total.
Still open: `paxos_co_wf` (per-knot `hcaus`/`hchain`/`hcplj`, the
`cc_wf` pattern at paxos scale) and the premise-free
`paxos_safe_sched'`. A `CausalRel` gluing instance (D38's suggestion)
was deliberately *not* built: its `fix` projections cannot be
definitionally the `SchedSem` fixes, so it would still leave per-knot
gluing — the corner subsumes it by making the gluing a carrier field.
Once `paxos_safe_sched'` lands, the Square δ-stack becomes a candidate
for retirement (proposal pending).


### D40 — defeq through `k` nested knots is exponential in `k`; the per-knot structural naming recipe, packaged as tactics

**Finding (the law).** Kernel/elaborator definitional equality through
nested `HydroSem.fix` knots is **exponential in the nesting depth**.
Measured on the real module code (one `rfl` at default transparency,
same machine): the full `leader_election` body with no knots closes in
**9 ms**; the same body inside one knot, **7 ms**; two nested knots,
**21 ms**; three nested knots (the real `leader_election` shape),
**~4 min** for the machine-leg naming and **>70 min** for the reader
leg; five knots (`leader_election`'s `fails ⊂ iam ⊂ ffx` under
`paxos_core`'s `alog ⊂ seqMax`), **>80 min and budget-blown at 51 M
heartbeats** — unprovable in practice. The mechanism: each outer
knot's body re-closes the inner knots (a Bekić-style sequential
closure of what is semantically a mutual fixpoint), and kernel
substitution loses sharing, so the program text multiplies once per
nesting level. This retroactively explains every defeq cliff in
D30–D39: whole-program `rfl`s and `simp …; with_reducible rfl`
transfer closers were all walking text that grows as
`O(program · c^knots)`. Corollary worth pinning: **module-scale and
body-scale `rfl`s are and stay cheap** (milliseconds — the modules are
compiled constants, folded), so the cliff is *only* the knots.

**The recipe (validated, then packaged).** Per knot, one lemma per
projection leg, and kernel defeq never crosses a knot boundary:

1. rewrite the knot's corner/square/eager `fix` to the target-instance
   `fix_stream`/`fix_tick` via the generic projection lemma — **by
   `Eq.trans`, never by `rw`** (with the knot bodies hoisted to named
   constants, `rw`'s higher-order pattern matcher fails to match the
   constant-body `fix`; unifying the fully-applied lemma against the
   goal side is first-order and always works);
2. split the boundary with `congrArg₂ _ rfl ?_` + `funext` — the only
   defeq steps are record-scale (fuel legs) and leaf-scale;
3. fold the body with the *body-level* naming lemma (a body-scale
   `rfl`) plus the **inner knots' own lemmas, consumed folded**, in a
   `rw`-fixpoint loop (`repeat first | rw […] | …` — one `rw` per
   instantiation; simp cannot be used here, D39's dependent-position
   limitation);
4. close with `with_reducible rfl` / `exact rfl` (leaf residues,
   `Unit`-eta fuel legs, record-eta decision collapses).

Prerequisite, and the one program-side change: the knot bodies must be
**top-level named constants** (`leBody`, `leCore`, `leFails`, `leIam`,
`leFfx` in `LeaderElection.lean`; `pcBody`, `pcAlogF`, `pcSeqF` in
`PaxosCore.lean`) — a pure code-motion hoist; inside the programs the
original `let`s now merely alias the hoisted names, so the program
values are definitionally unchanged and every colocated contract proof
survived verbatim. (User ruling recorded: no re-expression of program
logic, aliasing only. A `fix` that takes a colocated naming
certificate was considered and set aside — the hoist achieves the
same leverage without touching the interface.)

**The tactic layer (`KnotTactics.lean`).** The recipe is packaged as
macros, so a knot's naming lemma is one line naming its body rule and
inner-knot rules: `co_knot_sr`/`co_knot_rr` (coupling corner; `rr`
takes the knot's own `sr` lemma to bridge the machine diagonal inside
the reader iterate), `sq_knot_sr`/`sq_knot_rr` (square; simpler — no
diagonal, and decision records are *generic lens projections*
`leSqSDec`/`leSqRDec`/…, compositional across knots, no hand-derived
records), `eag_knot` (eager; single leg, decision vocabularies shared
with `Values`), over a common `co_body_rw` rule-loop builder. The
proof files are mirrors: `Paxos/CoupleKnots.lean`,
`Paxos/SquareKnots.lean`, `Paxos/EagerKnots.lean` — body-scale `rfl`s,
five knot one-liners, module-level namings, `pcBody` pipelines, and
the two program-level fix knots each.

**Measured effect** (whole tree, per-file wall clock):

| identity | before | after |
|---|---|---|
| `paxos_co_sr` (machine naming) | ~140 s (kernel `rfl`) | ~3 s total with `paxos_co_rr` |
| `paxos_co_rr` (reader naming) | **unprovable** (>80 min, budget-blown) | (same ~3 s) |
| `paxos_sq_sr` + `paxos_sq_rr` | ~154 s each (pre-hoist); broken post-hoist | 3.0 s file |
| `paxos_eager_den` (+`_ballots`) | ~675 s, broken post-hoist | 2.7 s file |
| `CoupleKnots.lean` (whole file) | — | 7.9 s |
| `SquareKnots.lean` (whole file, first compile) | — | 7.5 s |
| `EagerKnots.lean` (whole file, first compile) | — | 4.5 s |

The square and eager mirrors compiled **clean on the first attempt**
— the recipe transfers across proof instances without re-derivation,
which is the point: the per-knot cost is linear in knot count and the
authoring cost is one line per knot.

**Methodology note (the proper-architecture question).** This is
Isabelle's Transfer/Lifting discipline (per-constant rules + a
compositional closer) hand-rolled for a tagless-final embedding; the
couple/square instances are the standard relational (parametricity)
model built manually, since Lean has no parametricity plugin. The
programs' polymorphism over `H : HydroSem` is what makes every proof
instance attachable without touching program text. Deliberately *not*
pursued: a deep syntax instance (initiality — rejected: re-expresses
programs), order-enriched interface signatures (bundled monotone maps
— would touch every program). Next steps on this axis, agreed with
the user: (a) a `fun_prop`-style compositional monotonicity tactic so
the colocated mono clauses (embedding artifacts, not domain content —
e.g. `leader_election`'s binary Flo-monotonicity clause) become
one-liners or move out entirely; (b) possibly certificate-carrying
`fix` in the *proof-side instances only* (`CoupleSem`/`SquareSem`),
which would delete the per-knot lemmas rather than automate them —
to be validated on one knot before committing. Known residual
gotchas, so nobody re-learns them: `rw` with under-applied
higher-order lemmas fails on constant-body fixes (use `Eq.trans` with
the fully-applied lemma, metas solved by first-order unification);
trailing `rfl` after `rw`-loops must be `try`-guarded (`rw`
auto-closes); `simp` still cannot rewrite in contract-subtype
argument positions (use the `rw`-loop); `Unit`-eta fuel legs and
record-eta decision collapses (`pcSDec ∘ paxosCoDec = id`,
`pcSqRDec (paxosSqDec sdec) d = d`, `pcdecVE ∘ pcdecE = id`) close by
default-transparency `rfl`, not `with_reducible`.

### D41 — the premise-free machine-safety headline lands: per-knot `wf` discharge at paxos scale; δ-satisfiability fully dissolved

**Landed (the strong fix shape of SCHED_AUDIT F1).**
`paxos_safe_sched'` (`Paxos/CoupleWf.lean`): for **any** pacing, **any**
machine schedule, **any** horizon `T`, any coupled inputs and
intersecting quorums, any two commits the *step machine's* `paxos_core`
has emitted at one slot agree — with **no** satisfiability premise and
**no** decision argument. The δ-`hsat` hypothesis of
`paxos_safe_sched` is not discharged; it is *gone*. The pipeline is
four names and nothing else: `paxos_co_wf` (the corner run's residual
well-formedness, proven by construction) → `paxos_co_cpl` (the
machine/denotational coupling, `= .cpl paxos_co_wf`) →
`paxos_co_sr`/`paxos_co_rr` (the D40 structural namings) → the
colocated `PCEnsures.slot_functional`. The knot-free analogue
`cq_safe_sched'` (`CoupleStd.lean`) mirrors `cq_safe_sched` with the
inline δ-witness construction replaced by the corner's `cpl` — the
shared-stage instance of the same shape.

**What had to be proven: the five knots' `wf` triples (`cc_wf` at
paxos scale).** The corner's `fix_stream`/`fix_tick` carry
`hcaus ∧ hchain ∧ hcplj` (D39); `Paxos/CoupleWf.lean` discharges them
per knot, bottom-up (`leFails → leIam → leFfx → pcAlogF →
pcSeqF`-fix), with the D40 law intact — kernel defeq never crosses a
knot or corner boundary:

1. **Exposure** — `co_knot_wf` (`WfTactics.lean`): unfold the knot,
   `co_wf_simp [HydroSem.fix, HydroSem.fixTick, …]` projects the
   instance record-literal's `wf`, split into the three cases (~2 s at
   full leFails scale).
2. **`hcaus`** (machine-body causality) — `causal_of_eq` consumes the
   D40 body-naming lemma *as a lambda at the `schedEmbed`-embedded
   knot leg* (body-scale defeq, the cheap class; never `show` across
   the corner). The Sched-side walk is **compositional**: per-module
   congruence lemmas (`causal_pbc/plh/ap1/cqwr/pp1b/rc/ip/ap2/cq/jr`,
   one `simp only [module]` + walker each, ~0.2 s each), then
   `causal_leBody_*`/`causal_le₁₋₄`/`causal_sp₁₋₃`/`causal_pcBody₃₋₄`
   consume modules **folded**, and Sched knot congruences
   (`causal_leFails/leIam/leFfx/pcAlogF` via `causal_fix_stream/tick`)
   close outer knots' recursion hypotheses — every elaboration stays
   one-body-sized.
3. **`hchain`** (reader Kleene ascent) — `iterate_le_succ` with the
   body step's Flo monotonicity as `MonoRel` projections
   (`leBody_mono_fail/iam/pisl`; `pcBody_mono₃/₄` compose the
   `leader_election` colocated binary clause with
   `sequence_payload_mono`), inner knots threaded by
   `iterate_mono_param` — exactly the colocated `leader_election`
   monotonicity proof's shape, reused per knot.
4. **`hcplj`** (the graded coupling) — the H′-genericity payoff: the
   body is re-instantiated at the `j`-lowered corner
   (`CoupleSem … j Td`); captured wires re-built by `lowerC`
   (`wf`-preserving) and the knot leg by `mkC` at the probe data;
   decision records lowered by field-wise repack
   (`leDecLower`/`spDecLower`/`pcDecLower` — corner decision types are
   horizon-independent data, so the repack is `@[reducible]` and all
   dec-comparisons close by `rfl`); the two legs bridge to the
   obligation through the *horizon-generic* D40 body lemmas at both
   horizons (`Eq.trans` pairs), inner knots through their own
   `co_sr`/`co_rr` lemmas as **fully-applied `have`s**.
5. **wf threading** — per-module corner wf lemmas (inputs' `wf` →
   outputs' `wf`: `sp_co_wf₁₋₃`, `le_co_wf₁₋₄`, `pcBody_co_wf₂₋₄`),
   knots closed by their own wf lemmas.

**Measured** (`lake env lean`, warm oleans): `CoupleWf.lean` — 2.7 k
lines, ~60 lemmas including all five knot triples, the assembly, and
the headline — **26 s whole file**; the LE-knot stack alone ~6 s; the
leaf causal catalog ~3 s. `WfTactics.lean` 2 s. Whole-tree `lake
build` unchanged otherwise.

**New gotchas pinned** (so nobody re-learns them):
- `SquareHsat.causal_step` is unusable on these wires for **two**
  reasons: its `with_reducible apply` cannot type the op lemmas' raw
  decision binders against instance-typed goal args (and fails
  `KAgree (?mem ?p)` flex-flex), while *plain* `apply` on a
  head-**mismatched** candidate delta-unfolds `SchedSem` op bodies
  (whnf-exponential). The only fast quadrant is **head dispatch +
  plain apply** (`co_causal_step` in `WfTactics.lean`), with
  `synthAssignedInstances := false` for the custom `DecidableEq`
  wires.
- Cross-file `rw` with CoupleKnots lemma *patterns* misses occurrences
  (proj-form/instance-type mismatches at reducible transparency);
  instantiating the equation as a fully-applied `have` and rewriting
  with that always fires — both sides then come from the same
  elaboration.
- `simp only [sequence_payload, …]` inside `co_wf_simp`'s list throws
  `unsupportedSyntax`; unfold the module *first*, then run the wf set.
- Assembling `paxos_co_wf` by *spelling* the knot terms hits a whnf
  cliff (higher-order `Γ` unification on `HydroSem.fix`'s capture
  type); `refine pcBody_co_wf₂ … _ _ _ _ _ … ?_ ?_` with the wires as
  underscores lets the goal assign them first-order — instant.
- Section hypotheses not mentioned in a statement need `include … in`
  (the headline's `hcp`/`hck`).

**Consequence: δ-satisfiability is fully dissolved.** The D33→D37→D39
arc closes: the transfer premise that began as "∀ d, δ → safety" is
now a construction. The Square δ-stack
(`SquareDlt`/`SquareKnot`/`SquareHsat`/`SquareNonVacuity`, and
`paxos_safe_sched`'s `hsat` form) is hereby a **retirement
CANDIDATE** — *proposal only, nothing deleted*: the coupling corner
subsumes its role in every machine-safety headline; the square remains
valuable as the δ vocabulary record, the D37 falsification's home, and
the `sq_hsat` assembly blueprint. Decision deferred to the user.

### D42 — the Square δ-stack retired: the corner is the only transfer headline

D41's retirement proposal was ratified by the user ("let's do 1").
Deleted (net −9 files, ~300KB of Lean): `Square.lean`,
`SquareTheory.lean` (old `cq_safe_sched`), `SquareProj.lean`,
`SquareDlt.lean`, `SquareKnot.lean`, `SquareHsat.lean`,
`SquareNonVacuity.lean`, `Paxos/SquareSafety.lean` (old
`paxos_safe_sched`, `paxos_sq_rr`/`paxos_sq_sr`, `paxosSqDec`),
`Paxos/SquareKnots.lean` — plus `KnotTactics.lean`'s square-variant
macros (`sq_knot_sr`/`sq_knot_rr`). The machine-safety headlines are
now exactly `paxos_safe_sched'` (`Paxos/CoupleWf.lean`) and
`cq_safe_sched'` (`CoupleStd.lean`) — premise-free, so nothing of the
δ vocabulary is needed to *state* them. The full build drops
809 → 800 jobs and loses its slowest file (the old
`Paxos/SquareSafety.lean`, ~156 s of kernel defeq — the D38/D40
regression class had already been solved corner-side by per-module
lemmas; deleting the square deleted the last whole-program identity).

**What was generic got moved, not deleted** (the square files had
accreted instance-generic content):

- `Square.lean` → `Transfer.lean`: the decision-extension order
  `CutLe` + `cutLe_trans`, the `*_mono_dec` read-extension family
  (`prefixCuts`/`snapshotCuts`/`snapshotMemCuts`/`batchCuts`/
  `sliceCuts`/`snapTrace`), the **derive functions**
  (`batchDerive`/`batchOrdDerive`/`snapDerive`/`ordSelDerive`) with
  their horizon-monotonicity lemmas and list plumbing
  (`batchesEnd`/`sqBatchesFrom_append`/`tickSteps_le_steps`/
  `cutsLen_append`/`cutsMS_append`), and the shared machine/values
  utilities the corner consumes (`poolBot_le`, `tickVals_snapshot`,
  `selectOrder_prefix_ext`, `pool_chain_glue`, `trace_chain_glue`).
- `SquareHsat.lean` → `SchedCausal.lean`: the `causalDispatch` table +
  `causal_step` head-dispatch elab, **stripped of the square-carrier
  branches** (`SqStream.sr`/`SqTickSing.sr` dispatch and the
  `sq_sr_simp` fallbacks — replaced by a plain `simp only [ids]`;
  verified empirically: `paxos_co_wf` re-elaborates unchanged).
- `SquareProj.lean` → `CoupleProj.lean`: the ten Unit-decision
  normalizers (`values_*_unit`/`sched_*_unit`) that `co_transfer`'s
  simp list consumes.

Kept with their consumers stated: `SchedCausal.lean` (corner causality
+ audit artifact), `ValuesMono.lean` (knot chain conditions via
`WfTactics`), the whole `Transfer`/`TransferTheory`/`TransferChecks`
layer (the correspondence evidence and #guard adversary suite — never
part of the δ-stack). The D37 vacuity falsification and the δ design
vocabulary survive **as ledger** (D32–D41, `SCHED_AUDIT.md` F1) and in
git history — the lesson stands without keeping a
vacuous-premise-shaped theorem in the build.

Method note for future retirements: the reverse-dependency sweep must
be *name-granular*, not import-granular — three of the four corner
files imported square files, but for exactly 15 generic definitions +
10 lemmas + 1 elab, all relocatable in an afternoon. An import edge to
a big file is not evidence of a big dependency.

### D43 — `HydroGen`: the program machinery generated from signatures (phase 1, toy-validated)

**The mandate** ("all/most files in Paxos that are not proving
paxos-specific safety properties should disappear") is implemented as
a command-elaborator family (`HydroV2/HydroGen.lean`) that generates
the corner machinery from a module's **signature** plus fixed proof
scripts — the design bet being that every statement in the
hand-written stacks (`CoupleStd`/`CoupleModules`/`CoupleKnots`/most of
`CoupleWf`/`CoupleDec`) is type-directed and every proof is one of the
house script shapes:

- `hydro_couple M` — per-leg machine namings `M_co_srᵢ`
  (`co_transfer`), and for content decisions the **choice triple**
  `M_co_rr_ex` / `M_vdec := Classical.choose …` / `M_co_rrᵢ`: the
  derived decision is *named by choice* from the ∃-lemma (quantified
  over machine legs + machine-data decisions with pin hypotheses, at
  every couple horizon `Tc' ≤ Td`) — **no wire replay**, resolving
  `CoupleDec.lean`'s TEMP note by deletion. Knot modules get machine
  namings only (`hasKnot` detection); their reader legs go through the
  knot route.
- `hydro_causal M` / `hydro_wf M` / `hydro_mono M` — per-leg step
  causality (head-dispatch walker), corner wf-threading, and `Values`
  monotonicity via the `MonoRel` instantiation (grade-directed
  relations, statements at the `Values` legs, proofs = `.property`
  projections).
- `hydro_knot K` — the knot `wf` triple `K_co_wf` for a
  `HydroSem.fix` wrapper over a registered body module, by the
  validated blueprint (`toy_loop_co_wf_blueprint`,
  `HydroGenCheck.lean`): `hcaus` from the body's generated causality
  bridged by its machine naming, `hchain` from its generated
  monotonicity at the probe (assembled at the `Expr` level — the
  `R`-relation ascription is knot-specific), `hcplj` from the
  H′-generic body re-instantiated at the `j`-lowered corner, bridged
  by the generated namings at two horizons — **the choice-`vdec`'s
  horizon-genericity doing exactly the work D39's explicit records
  did**. A generation registry (`genRegistry`, persistent env
  extension) lets callers consume callees' artifacts.

**Validated end-to-end on a toy suite** (`HydroGenToy.lean`:
`toy_relay` with batch/pulse/emit decisions and a colocated contract;
`toy_step`; `toy_loop` = `HydroSem.fix` over `toy_step`):
26 artifacts generated, and `toy_safe_sched` — every value the step
machine's relay emits at any pacing/schedule/horizon is positive —
proven purely from generated artifacts + the colocated contract, on
the standard three axioms.

**Meta-engineering lessons** (the ledger's tax):
- proofs execute via `Lean.Elab.runTactic` on synthetic-opaque goals
  with (a) an **empty local context** for closed statements (the
  generator's open telescopes otherwise shadow script names), (b)
  **assigned-goal filtering** (the ∃-witness hole stays in the goal
  list after unification pins it), (c) every `try` parenthesized (in
  one-line scripts `try` gobbles the rest of the sequence — a failing
  pin rw silently swallowed a closing `rfl`);
- `case tag =>` selection misbehaves under `runTactic` (post-`case`
  goal lists come back empty) — generated scripts sequence
  parenthesized blocks positionally instead;
- gadget constructors (`schedC`/`probeC`/`mkC`/`lowerC`) leave
  `Tc`/`Td`/`hjT` metavariables (machine-typed arguments cannot pin
  them) — `applyNamed` supplies implicits **by binder name** and
  synthesizes instances at the end; wire binders must be created at
  the **unreduced** carrier types (whnf erases `ord`/`ret`, and
  `PoolCarrier` whnfs away entirely — use `whnfUntil`);
- variable-spelling proof steps are installed as **proved local
  facts** (`FX`/`FC`/`HVA`/`FH`/`FOUT`/`FSR`/`FRR`, `mkAppM`-built,
  gadget projections normalized by the `rfl` projection simp set)
  and discharged by beta after the fixed script runs — no
  pretty-print round-trips anywhere.

**Phase 2** (successor thread): migrate Paxos — generate the module
stacks for the 12 program modules and the five knots, delete
`CoupleStd`/`CoupleModules`/`CoupleKnots`/`CoupleDec` and CoupleWf's
mechanical majority (the `paxos_safe_sched'` statement and assembly
stay), same for the Eager naming files; extend `hydro_knot` to tick
knots and subtype-faced bodies; optional `hydro def` authoring sugar
(inline knot bodies, auto-hoist). `Std/Quorum` is the rehearsal
target (knot-free; its hand stack in `CoupleStd.lean` is the
reference).

### D44 — `HydroGen` phase 2a: every knot-free module generates (records, wire bundles, program scale)

The `HydroGen` commands now cover **all knot-free program modules** —
`Std` (`collect_quorum`, `collect_quorum_with_response`,
`join_responses`) and Paxos (`p_ballot_calc`, `p_leader_heartbeat`,
`acceptor_p1`, `recommit_after_leader_election`, `index_payloads`,
`acceptor_p2`, `p_p1b`, `sequence_payload`, `leBody`) — validated
build-enforced in `HydroV2/HydroGenStage.lean` alongside the
hand-written stacks they will replace, including `cq_safe_sched''`
(the `collect_quorum` machine-safety pipeline re-proven end-to-end on
generated artifacts only). Engine extensions:

- **Decision records** (`ArgKind.decRec`): structures over the
  interpretation whose fields are decision families (nested allowed —
  `LEDec` contains `PLHDec`/`PP1bDec`). Field trees are read off the
  structure; repacks (corner→sched/values/MonoRel/lowered corner) are
  built field-wise at the `Expr` level — the generated counterparts
  of the hand-written `leSDec`/`spSDec`/`pcSDec`/`ledecMR'`/
  `leDecLower` family. The ∃-unit of the choice naming is the whole
  `Values`-typed record: machine-data fields become flattened,
  pinned outer binders (`sp_co_rr_ex`'s shape); the witness skeleton
  is a nested anonymous constructor with `_` for `Values`-data fields
  and literal `()` for machine-only fields (a hole for a field the
  reduced equations never mention stays unassigned — the `()` is
  load-bearing); **`fixd` fields get explicit trailing pin conjuncts
  `dV.fuel = Td + 1`** (carried-but-unused by knot-free bodies, so
  unification alone cannot assign them).
- **Wire bundles**: output structures whose fields are all carriers
  (`LEWires`) peel into `.field`-step legs — `leBody` generates 7
  per-leg stacks.
- **Bound-directed mono relations**: `.monotonic vo` tick singletons
  compare on `.vals` prefixes (`MonoTrace`), `.unbounded` on plain
  prefixes.
- **Command extras**: `hydro_couple M [p1bPairDecEq]` threads custom
  unfold hints (match-scrutinized `DecidableEq` instances) into every
  generated script — the hand-written files' inline hint lists,
  surfaced as command arguments.
- **Interleaved callee rewriting**: the ex-script's callee namings
  must alternate with the full projection simp set (each rewrite
  exposes fresh `rr`/`sr` projections); running them in sequence
  leaves the closing `rfl` a whole-module defeq — the phase-1 script
  shape silently depended on toy-sized bodies.
- **Heartbeat policy** (user ruling): the engine sets NO budget;
  a generated proof that times out is a diagnostic failure of the
  script structure. The two big module bodies (`sequence_payload`,
  `leBody`) carry explicit `set_option maxHeartbeats 1000000 in` at
  their command call sites — same class as the 1.6M the hand-written
  `sp_co_rr_ex` paid, now visible where it is paid. Everything else
  runs in the default budget; the full 12-module ladder elaborates in
  ~38 s.
- Two D-class lessons: outer binder types for record fields must be
  reinterped corner→sched or the corner's `Tc`/`hjT` fvars escape to
  the kernel; generated inner-∀ binder names must be positional
  (`w{i}`) — hygienic user names from anonymous Pi binders
  (`a✝'`) are unparsable in `intro` scripts.

**Remaining phase-2 work** (successor): `hydro_knot` generalization —
tick knots (`fixTick`/`CoTickSing` gadgets), leg-selected bodies
(`(pcBody …).2.2.1`), composed bodies (`leFfx`'s
`leCore(leFails(leIam f) f, leIam f, f)` nesting), then the knot
namings (`CoupleKnots`), the `CoupleWf` assembly rebuild on generated
names, the command-block migration into the program files, and the
deletions: `CoupleStd`, `CoupleModules`, `CoupleKnots`, `CoupleDec`,
`EagerKnots`/`EagerCheck` namings, CoupleWf's mechanical majority,
and `HydroGenStage` itself. User ruling recorded for the optional
authoring sugar: custom keywords (e.g. a `fix` block) are acceptable,
but the `let`-chain house style must be preserved — the sugar should
change program text as little as possible.

## D45 — `hydro def`: recorded composition specs replace reverse-engineering (and where the generator route actually breaks)

**Finding.** Phase 2b's mandate — make every Paxos file that is not a
paxos-specific safety proof disappear — failed twice under the
"reverse-engineer the module body" architecture before landing under a
"record the composition at definition time" one. The failures were
structural, not effort-shaped, and each one identifies a boundary of
what Lean gives for free:

1. **Choice-opaque decisions kill definitional folding.** The glue
   route's final step re-spells a discovered term as the folded
   `M @ Values (M_vdec …)`. With `M_vdec := Classical.choose …`, two
   spellings of the same record at different outer arguments are
   *never* definitionally equal, so the kernel rejects (or times out
   on) any `mkExpectedTypeHint` that must bridge them — even when the
   elaborator's caching made the hint look cheap. Symptom: kernel
   `deterministic timeout` / `application type mismatch` on
   `leader_election_co_rr₄`-shaped lemmas. Cure: the two-sided fold
   (`glueLegProof`): normalize BOTH the corner side and the module
   side to the same spelling and require them to meet
   **syntactically**; the kernel then only ever replays one-delta
   `rfl`s and congruence steps.

2. **`simp` cannot rewrite dependent argument positions.** `leBody`'s
   output subtype mentions its `ff` wire, so the congruence lemma for
   that argument has no motive and `(leFfx@corner …).sr` sits
   unrewritten precisely there. The redex IS rewritable at the
   equation level — `kabstract` over the equation's RHS, whose type no
   longer depends on the argument once the `.val`/leg projections are
   outside. The residual pass (`RwSt.rwResiduals`) runs the callee
   naming lemmas this way: one keyed scan per round buckets closed
   subterms by LHS head-shape, `matchLemma` unifies at *reducible*
   transparency (default transparency on a near-miss deltas a whole
   module body — the classic whnf bomb), and results are memoized per
   `(lemma, candidate)` — the memo took `hydro_glue leader_election`
   from 70 s to 3.9 s, back inside the default heartbeat budget.

3. **Module-scale `MonoRel` projection is definitional only below the
   first `fix`.** `hydro_mono leBody` is a projection; `hydro_mono
   leader_election` kernel-fails, because `.1` of the MonoRel pair
   iterate and the `Values` iterate are different stuck functions on
   opaque fuel. The compositional generator (`genGlueMono` /
   `resolveRel`) rebuilds the hand-world shape mechanically: unify the
   normalized leg subjects against callee mono lemma conclusions,
   resolve hypothesis metavariables recursively (which also pins wire
   metavariables the conclusion left loose), default the binders the
   statement provably does not pin. Routing: the glue route is taken
   iff a callee transitively reaches a knot (`reachesKnotN`).

4. **`wf` threading: fold everything.** Unfolding knot-free callees
   into `co_wf_simp` was the cost center (`sequence_payload` alone put
   the normalizer at ~17 s) and forced transitive closer lists. Every
   callee now stays folded and closes by its own `_co_wf` lemmas; the
   loop's closers stay at reducible transparency (`assumption` and
   anonymous-constructor `refine ⟨…⟩` at default transparency whnf
   through folded callee deltas — two more heartbeat bombs), with
   `And.intro` splitting only syntactic conjunctions.

**The surface.** `hydro def M …` (or post-hoc `hydro_register M`)
walks the definition against a strict module grammar — core operators,
registered module invocations, `HydroSem.fix`/`fixTick`,
`hydro_inline`-marked wrappers (`leCore`, `pcSeqF`), data
(interp-free), ghost contracts (proofs, skipped) — and records the
callee graph in an environment extension. Unrecognized invocation
heads error at the definition, not three files downstream. Naming the
seq knot as a module (`pcSeqX`, ten lines in `PaxosCore.lean`)
dissolved the last special case: `hydro_knot pcSeqX` and even
`hydro_glue paxos_core` — the whole program as one glue module — then
came out of the ordinary machinery, and `paxos_co_sr/rr/wf` became
one-line applications of `paxos_core_co_*`.

**The deletion.** `CoupleStd.lean` (−350), `Paxos/CoupleModules.lean`
(−2100), `Paxos/CoupleKnots.lean` (−640), `Paxos/CoupleDec.lean`
(−250) deleted; `Paxos/CoupleWf.lean` 2722 → 92 lines (only
`paxos_co_wf`/`paxos_co_cpl`/`paxos_safe_sched'`, statements
meaning-identical); `Paxos/CoupleSafety.lean` rebuilt at 115 lines
(`paxosCoDec` + one-line `paxos_co_sr`/`paxos_co_rr` over the
generated `paxos_core_vdec`); the staging files dissolved into
program-file tails. Remaining hand machinery: `EagerKnots.lean` (331
lines — the eager/`den` projection is a different correspondence with
no generator yet) and two measured `set_option maxHeartbeats 1000000`
bumps on `hydro_couple sequence_payload`/`leBody` (the leaf
choice-route script; porting it to the keyed rewriter would remove
them).

**Deferred (design lesson).** The principled endgame is a relational
interpretation (`MonoRel` for the couple/`Values` correspondence
itself): carriers pair the corner couple with the `Values` value under
an rr-agreement invariant, and `HydroSem.fix`'s already-polymorphic
body (`∀ H', Γ H' → …`) means each combinator pays its obligation
once (`co_hfix` for `fix`, a computable trace-realization function per
content decision — de-classicalizing the demon witness). Namings then
become projections like monotonicity, with no metaprogramming at all.
That refactor touches every combinator's semantics and is deferred;
`hydro def` keeps the door open because module invocation needed no
marking in the first place.

### D46 — the relational layer: generated parametricity replaces per-pair generators (design + load-bearing prototype)

**The user's directive** (interjected mid-design): do not hand-declare
"some mega relation that hardcodes all the correspondences" — make the
macro capture the information Lean lacks (parametricity for the
`HydroSem` signature) so new language-generic guarantees are pure data.
Landed as three pieces:

1. **`HRelC I H₁ H₂`** (`HydroRel.lean`, hand-written, 16 fields
   mirroring the class's *type* fields): one relation family per
   carrier former and per decision family, indexed by a preordered `I`
   (`Unit` for unindexed guarantees, `Nat` horizons for step-indexed
   ones).
2. **`hydro_rel_laws`** (generated — `HydroRelLaws.lean` is one
   command): for every *operation* field of `HydroSem`, the H-relative
   binary parametricity lift of its projection type — non-`H`
   data/closure/proof binders shared (detected by syntactic equality of
   the lockstep-instantiated binder domains), carrier/decision binders
   doubled with relatedness premises at the ambient index. The only
   negative carrier positions (`fix_stream`/`fix_tick` bodies) lift
   with **index lowering and strong-form inputs**
   (`∀ i' ≤ i, ∀ y₁ y₂, (∀ i'' ≤ i', rel i'' y₁ y₂) → rel i' (b₁ y₁)
   (b₂ y₂)`) — the step-indexed logical-relation shape, with the
   loop-wire fact re-lowerable at capture-crossing inner knots and
   **no downward-closure field on the signature** (instances spend
   their own closure privately, e.g. `SAgree.mono`). Laws bundle into
   `HRel.Laws C` (an `And`-chain def) with generated per-op accessors
   and an op-head → accessor dispatch registry.
3. **`hydro_param M`** (`HydroParam.lean`): the per-module free
   theorem, one per output leg, stated directly about `M H₁ …` and
   `M H₂ …` — so the D41 wall (gluing-instance fix projections not
   kernel-defeq to the base fixes) never arises, and kernel cost never
   crosses a module or knot boundary (D40 discipline: callees fold
   into their own `_param`s, knots close by the `fix_stream` law).
   All wire facts ride the uniform strong form `∀ i' ≤ i, rel i' x y`.

**Validated** (`HydroParamCheck.lean`, whole file ~5 s): free theorems
for `p_ballot_calc`, `p_leader_heartbeat`,
`collect_quorum_with_response`, `acceptor_p1`, `p_p1b`, `leBody`
(7 legs, `LEDec` record premises field-wise), and the three real
knots `leFails`/`leIam`/`leFfx` (composed `leCore` bodies, nested
knots, the tick knot). The **eager-agreement instance**
(`EagerRel.lean`: `eagC` = den-projection equality, `eagLaws` = 38
uniform one-liners + 2 `congrArg` fix fields, 3 s) then reproduces the
`EagerKnots.lean` content as two-line corollaries — `leFails_eag` (a
1.6M-heartbeat `eag_knot` call) and `leFfx_eag` become direct
applications of `leFails_param`/`leFfx_param`. Causality and
monotonicity fit the same signature (compiled law-field demos:
`causal_map`, `causal_fix_stream` through the index-lowered premise
with `SAgree.mono` spending the strong form, `pool_map_le`).

**Walker lessons** (extending D43's ledger): goal types out of
`MVarId.apply` need `instantiateMVars` before head dispatch;
`subjectHead` must NOT strip `HydroSem`'s own projection functions
(ops ARE projections — stop at `info.ctorName == HydroSem.mk`);
callee `_param` rules must have their ambient index **pinned to the
goal's index** at apply time (defeq-checked `le_refl` assignment —
leaving it to unification lets an eager `assumption` pick the outer
bound and strand capture-crossing loop wires; raw `MVarId.assign`
skips typechecking and produces kernel-level mismatches); `≤`-chains
need a deterministic backtracking closer (`leChain`) — `solve_by_elim`
misses depth-4 chains and `mkAppM` rejects under-applied `le_trans`.

**What this does NOT subsume** (honest boundary): the couple *rr*
choice-∃ naming existentially *produces* `Values` decisions from
machine wires — a pointwise relation between decision types cannot
express that; it stays on the `hydro_couple` engine. The corner's
`cpl` needs body side facts beyond pointwise preservation (`hcaus`,
`hchain`); if the corner is ever wanted as a relation, the standard
cure is conjoining the invariants into the relation.

**Migration (successor)**: full `mono`/`causal` `HRel.Laws` instances
(fields = `ValuesMono`/`SchedCausal` content, mechanically);
`hydro_param` for the remaining modules (function-output legs —
`leader_election`'s applied `.val` — need the `legPre` extension) and
the PaxosCore side; then `hydro_causal`/`hydro_mono` (and the eager
naming files) collapse into instances + `_param` corollaries.

### D47 — post-churn GC sweep: the residue census after D33–D45 (D46 reserved for the relational-layer design thread)

**Method.** Name-granular reverse-dependency census over every
declaration in `HydroV2/` (614 decls across 34 files): unicode-aware
token matching (subscripted generated names, primes), code-references
vs. doc-references (CORRESPONDENCE/SCHED_AUDIT/READMEs/ACCEPTANCE
count as consumers; FINDINGS is a ledger and does not), in-file-only
liveness resolved manually through the macro layer (per-op families
like `co_*_wf` are consumed via the `co_wf_simp` macro list; the
generators reference tactic macros in generated script strings — both
greppable), and `@[simp]`-tagged lemmas excluded from deletion
outright (a bare `simp` consumes them invisibly). Import-granular
sweeps lie (D42's method note); so do plain word boundaries at
generated names.

**Deleted (no code or doc consumer anywhere, no attributes, trivially
re-addable from git history).**
- `TransferTheory.lean`: the square-era per-wire ∃-form/projection kit —
  `stream_sound`, `tick_stream_sound`, `tick_sing_sound`,
  `corr_fix_stream_fst/snd`, `corr_fix_tick_fst/snd`, plus their
  private helpers `iterate_subtype_fst/snd` (only consumers were the
  deleted four), `TickStabilizesAt`, `StabilizesAt.fold_read`. The
  doc-cited members of the file (`snapshot_tight`, `batch_tight`,
  `deliver_id_attains`, `batch_flat_attains`, `fix_diag_attains`, the
  StabilizesAt kit core) stay — CORRESPONDENCE walks them.
- `Transfer.lean`: `cutLe_trans` (D42 relocation whose consumer died;
  the rest of the `CutLe` cluster is live via `snapTrace_mono_dec` ←
  `Couple.lean`).
- `HydroGen.lean`: `isInline`, `GenExtras.suffix`;
  `HydroGenKnot.lean`: `cornerBodyAt`; `HydroGenCheck.lean`:
  `toy_lowerC_wf` — superseded helpers of the engine's own evolution.

**Kept deliberately (so the next auditor doesn't redo the sweep).**
- `ValuesMono.lean` is **entirely dead** (the `values_mono` walker and
  all 42 `vmono_*` lemmas have zero consumers; the generators
  discharge chain-mono via `MonoRel` projections +
  `iterate_mono_param`; `WfTactics`' import of it is vestigial). It is
  kept ONLY because the relational-layer design thread (D46 slot)
  lists it as instance-field source material for the mono relation.
  **If that design does not absorb it, delete the file.**
- `Trace.lean`'s ten unreferenced lemmas: four are `@[simp]`; the
  scan/cuts families are the queued ScanRun-adoption and
  quiescent-completeness consumers (standing ruling).
- `Grades.norm_mk`/`support_mk` (`@[simp]` carrier API),
  `ballots_covered` (coverage witness, standing ruling),
  `CoupleCheck.lean` (the corner's hand-validated blueprint,
  AxCheck-audited), all doc-cited Transfer/TransferChecks artifacts,
  the Eager stack (relational-thread validation target; census found
  zero dead there anyway), `HydroGen`'s BFS callee-scan fallback
  (fallback for unregistered modules; stale "staged migration"
  comment fixed).

**Doc fixes.** SCHED_AUDIT header now names the primed headlines;
CoupleProj's two comments referencing the retired `SquareProj.lean`
reworded as historical; TransferTheory's header no longer advertises
the deleted readers. ACCEPTANCE/READMEs verified clean (remaining
Square mentions are intentional retirement notes).

**Net.** ~180 lines of true dead code removed, zero behavior change,
one explicit deferred-deletion flag (ValuesMono). The tree after
D42+D45+this sweep carries no known dead code outside that flag.

## D48 — liveness under fairness: the schedule space is the behavior space (rung 1 proven)

*(Numbered per parent coordination: D46/D47 are reserved for the
concurrent HydroRel-design and dead-code-GC threads.)*

**The design result** (`LIVENESS.md`, prototype `Liveness.lean`):
TLA+-style liveness needs **no new semantics**. The machine is
deterministic given its schedule tuple, so a TLA+ behavior *is* a
schedule point and WF conditions are ∃-predicates on parameters we
already quantify over (`FairTicks`, `FairCursor`); *eventually* is
∃-horizon attainment over the same prefix-monotone views (upward-
closed observations get `◇P ↔ ◇□P` free); the WF1 rule becomes the
per-operator fair-attainment family — `TransferTheory`'s end-of-time
kit with generous schedules generalized to fair ones and explicit
horizons existentialized (`deliver_fair_attains`,
`ticks_fair_attains`); the leads-to induction rule already existed
(`scan_emit_ind`). Safety theorems are untouched: fairness only
shrinks the quantifier.

**Rung 1 proven**: `cq_live` — on the real step machine, a stabilized
response wire holding a quorum of `Ok` votes for `k` (usage contract),
a fair tick past stabilization within the emission-claim supply, and
honest claims imply `∃ T, k ∈ output.view T`. The crossing argument is
`cq_run_count`'s iff face; `famFreeze` is discharged by the
prefix-monotonicity chain (`tickSteps_prefix → batchesFrom_prefix →
scanAcrossTicksTrace_prefix → emitLin_prefix → flatten`); the
machine-output naming was a plain `rfl` at default heartbeats
(module-sized defeq — no D44-class cost at this scale). D15 witnesses:
`#guard`s on a non-trivially fair (odd-tick) schedule computing the
promised `T`, plus an `example` discharging every premise jointly —
the D37 lesson applied to liveness, whose premise sets (fairness ∧
supply ∧ honesty ∧ stability) are exactly the kind that can silently
conflict.

**The finding liveness surfaced (safety-invisible)**: `SchedSem`'s
`EmitDec` is a *finite* claim list while fair skeletons tick forever —
every claim list eventually exhausts and `emitLin` stops emitting.
Prefix-closed ∀-horizon safety cannot see this; for liveness it is
the WF(emit) obligation. Handled today by an honest supply premise;
the alternative claim-*stream* carrier (`Fin n → Nat → List β`) is a
small safety-preserving upgrade catalogued as a fork in `LIVENESS.md`.

**The ladder** (user-facing, in `LIVENESS.md`): (a)
quiescent-completeness at paxos scale — next rung, zero model changes,
composes this kit through the knots via `fix_diag_attains`-style
chain-fixes hypotheses; (b) commit liveness under leader stability —
statement spelled in two candidate premise styles (wire-level
`StabilizesAt` vs input-level with derived stability), user ruling
pending; (c) liveness under churn — the only rung forcing the
fuel-less/lfp `Values` upgrade, deliberately last. Convergence with
the HydroRel design noted: fair-attainment is a step-indexed relation
whose index is progress rather than agreement horizon — one more
instance of the same free-theorem shape, making `hydro_live`
generable.

### D49 — the relational layer at program scale: `hydro_param` everywhere, the per-pair machinery collapsed

**The migration** (successor to D46's prototype). Three instances now
cover four guarantees; one free theorem per module covers every
program; the eager naming machinery is gone.

1. **Instances**: `MonoHRel.lean` (the ⊑-diagonal at `Values`;
   `ValuesMono.lean`'s 41 `vmono_*` lemmas absorbed as its field
   library — the D47 deferred-deletion flag resolved by absorption,
   the file deleted) and `CausalHRel.lean` (agreement-below-horizon at
   the machine diagonal; fields = the `SchedCausal` congruences, the
   knot fields spending strong forms through `SAgree.mono`/
   `TAgree.mono`). Each: all 40 op laws by one uniform tactic,
   ~3–5 s, first try — D46's "mechanical repackaging" prediction held
   exactly.
2. **`hydro_param` at full scale**: the generator moved to the
   program-file tails (all 12 knot-free modules, the 3
   `leader_election` knots, `leader_election` itself, `pcBody`,
   `pcAlogF`, `pcSeqX`, and **`paxos_core` — 2 legs, whole program**).
   Two engine extensions: (a) function-output legs (`leader_election`'s
   applied `.val`): the terminal case walks the Pi with doubled wire
   binders + strong-form premises and peels the applied result into
   sub-legs (4 lemmas); (b) `subjectHead` now beta-reduces and unfolds
   reducible non-registered heads. The latter fixed a real walker bug:
   the fix-law body premise presents subjects as beta-blocked lambdas
   or reducible named bodies (`pcSeqF`), the blocked head deferred to
   the un-pinned user rules, and an eager `≤`-closure pinned their
   free ambient index to the OUTER bound — stranding capture-crossing
   loop wires (the D46 pin lesson, replayed one level up). Kernel cost
   stayed module-sized: PaxosCore 47 s vs 42 s baseline for all four
   generators.
3. **The eager collapse**: `paxos_eager_den`/`_ballots` (statements
   unchanged) are now two-line corollaries of `paxos_core_param₁/₂` at
   `eagC`/`eagLaws` — `EagerCheck.lean` builds in 1.5 s (was a
   3.2M-heartbeat, 65536-recDepth rw-chain file). `EagerKnots.lean`
   (331 lines of hand-written per-knot namings) DELETED; its
   `*decVE` repacks moved beside their `*decE` inverses in
   `EagerCheck.lean`; `KnotTactics.lean`'s eager section
   (`eag_knot`, `eag_hfix_*_den`) died with its only consumer.
4. **Program-scale showcase** (`HydroParamCheck.lean`): whole-program
   Flo monotonicity and whole-program step causality of `paxos_core`,
   each a direct `paxos_core_param₂` application at the mono/causal
   instance — compiled in seconds, zero new machinery.

**Honest boundary — the `hydro_causal`/`hydro_mono` walkers stay.**
Their generated per-module lemmas are consumed inside the knot/wf
generated proof stacks (`kc.comp.gi.causalLemmas[0]`/`monoLemmas[0]`
applied as Exprs with exact statement scaffolding: plain
at-horizon premises, machine-typed shared binders). Deriving
statement-identical lemmas from `_param` corollaries needs a
premise-form conversion (plain ↔ strong) threaded through the knot
composition machinery — mechanical but engine surgery on the
paxos_safe_sched' chain, deferred with this note as its spec. The
user's actual goal is nonetheless fully met: a NEW guarantee is one
`HRelC` + one `HRel.Laws` instance (`EagerRel`/`MonoHRel`/`CausalHRel`
are the templates), and every `hydro def` program already has its
free theorem.

### D50 — liveness over decision chains: behaviors are chains in decision space; proofs at `Values`, transfer restored

**The correction arc.** Rung 1 (D48) proved `cq_live` *at the
machine*: fairness as schedule-parameter predicates, protocol content
re-derived against machine batches. The user challenged the frame
twice — *leader change* and *lossy links with retries* — and both
challenges are fatal to it: TLA+ fairness is predicated on
**enabledness** (state-dependent; WF for persistent, SF for
recurring), so obligations transfer automatically when the leader
changes, and lossy-retry is SF territory (rung 1's "SF has no client"
was an artifact of the fail-stop model, not a fact about fairness).
Parameter predicates + end-of-time saturation are the
persistent-enabledness *quiescent fragment* only. The user then asked
for the real target: proofs **over the denotational semantics** that
transfer, not proofs over the step-indexed machine.

**The design of record** (`LivenessChain.lean`, `LIVENESS.md`
rewritten): a machine behavior *denotes an ascending chain in
decision space* — derivation is horizon-monotone, so `T ↦ d(T)` is a
chain, and the decision lattice (safety's `∀d` shadow of the schedule
space) has its chains as the shadow of infinite behaviors.

- Temporal operators = quantifier shapes over chain positions
  (`ChEventually`/`ChEvAlways`/`ChAlwEventually`/`ChLeadsTo`, with
  the small calculus).
- **Fairness = a class of chains**, in decision vocabulary:
  WF(tick) = `Exhausts` (the cut chain eventually consumes the
  pool); ◇□-stability = `ChStabilizes` of a chain component (the
  leader-change premise); SF = `ChSF`, a coupling between an
  enabledness trajectory and an action trajectory (lossy-retry;
  needs the skipping-cursor model fork before its first real
  client). Enabledness is expressible because the state at a chain
  point is computed by `Values` from the decisions so far.
- Every liveness theorem factors **V ∘ K1 ∘ K2**: (V) `Values` chain
  theorems carry all protocol content (`cq_complete_mem` — the point
  lemma at complete decisions, pure `emit_count` contract reasoning;
  `cq_chain_live` ◇ and `cq_chain_live_stable` ◇□ along chains);
  (K1) chain-fairness transfer — a fair schedule's derived chain is
  a fair chain (`cqDerivedChain_isCutChain` needs no fairness;
  `cqDerivedChain_complete_at`/`_exhausts` spend `FairTicks`);
  (K2) tightness — the machine observation *is* the `Values` run at
  the derived point (`cq_values_at_derived`, near-`rfl` after
  `batchCuts`-legality) + emission attainment (`cq_emit_attains`).
- **`cq_live_chain`**: rung 1's conclusion re-derived — identical
  premises, few-line glue, the quorum crossing consumed *only*
  through the `Values` contract. The transfer property is restored
  for liveness. Rung 1's `cq_live` kept for comparison; its WF1
  lemmas live on inside K1.

**Why the classical obstruction doesn't apply**: "fairness is not
denotational" (fair merge is not Scott-continuous) forbids folding
fairness into the semantic domain; here the semantics is untouched
and fairness is a predicate on chains in the logic above it;
conclusions are ∃-chain-point shaped, so finite points witness them.
Only rung (c) (progress under unbounded load) touches ω-limits — the
queued lfp upgrade, deliberately last.

**Non-vacuity**: the odd-tick witness re-certified through the chain
frame; `#guard`s compute the derived chain's exhaustion point (empty
at horizon 0, complete at the first odd tick); an `example`
discharges every `cq_live_chain` premise jointly.

**Convergence + forks**: K1+K2 are ∃-progress-indexed relations —
the `hydro_live` = `HRelC`-instance + `M_param` endgame is recorded
in `LIVENESS.md` (its `emit` law is blocked on the `EmitDec`
claim-stream fork, whose recommendation this rung strengthens: the
supply premise lives in K2 and would multiply at Paxos scale). New
model fork recorded: skipping cursors for lossy links (rung b′).
Elaboration notes: root-level `add_assoc`/`List.sum_append` are not
usable on `Multiset` sums here — use `Multiset.add_assoc` and a
manual append-sum induction (house pattern, cf. `ofList_flatten`).

### D51 — the surface syntax: `hydro def` annotation, inline `fix`, and the ghost layer; Flo monotonicity is a free theorem

**The two halves** (user-directed): (1) `hydro def` as a Rust-attribute
style annotation that runs the *whole* pipeline — no command tails —
with `forward_ref` cycles written as inline `fix` blocks; (2) a ghost
layer (`ensures`/`ghost`/`prove`) so contracts read like Verus specs
and proof code shrinks to what is actually interesting.

**Half 1 — `hydro def` + `fix` (`HydroDef.lean`).**

- `hydro [hints]? (inline)? def M … := …` grafts the doc comment,
  elaborates the wrapped `def`, registers the composition spec, drains
  the emitted-knot queue, and routes the generation pipeline from the
  spec itself (hasFix → knot+param; reaches-knot → glue+…; else
  couple+…) by *synthesizing the existing phase commands*. The old
  per-module command tails (~170 lines across 11 program files) are
  gone; `hydro inline def` marks wrapper defs.
- The `fix` block (`fix (w₁ : τ₁) … (wₙ : τₙ) via (f₁,…,fₙ) := body;
  rest`) is Rust's `forward_ref` reading: a term elaborator performs
  the **Bekić decomposition** to exactly the hand-written cascade
  (last component outermost), hoists each component to a top-level
  knot `M.wᵢ` with an instance-generic body and curried caps, and
  queues it for the enclosing `hydro def`'s pipeline pass. Author
  wire names bind the *closed* knots over the rest of the body.
- Hard-won elaboration laws: (a) lambda-binder types in the lctx can
  be unassigned metavariables — `instantiateMVars` before any fvar
  classification, or captures silently escape; (b) knot component
  bodies must be emitted as **named `@[reducible]` constants**
  (`M.wᵢ.body`, inline-registered) — inlining the composed body
  re-creates the D40 pathology (wf whnf blowup at `pcBody` scale);
  (c) inline wrappers must be `@[reducible]` because the param walk
  crosses them under *partial* applications where `simp only` cannot
  reach (the `pcSeqF` precedent, now automatic); (d) manual
  `addDecl`s need `enableRealizationsForConst`, and tactic blocks in
  the `rest` need `synthesizeSyntheticMVarsNoPostponing` before wire
  fvars are abstracted (delayed-assignment leaks).
- The auto-generated knots are *definitionally equal* to the
  hand-written ones (toy: `@toy_loop2 = @toy_loop := rfl`), and
  faster to check: PaxosCore 47s → 23s (folded bodies beat the hand
  layout); LeaderElection unchanged (≈6min, wf-dominated).

**Half 2 — the ghost layer (`HydroDef.lean`, term-level only).**

    hydro def M (H : HydroSem L mem) (args…) :
        τ ensures out => P out := 
      let a := …
      ghost let g := ‹spec-only value›
      ghost have h : S := proof          -- at the program point
      ghost obtain ⟨w, hw⟩ := sub.property rfl
      (value)
      prove f₁ := e₁, f₂ := e₂, …

- `ensures` (type position) generates the `{out // ∀ hv : H = Values
  L mem, match H, hv, deps…, out with | _, rfl, … => P}` face,
  transporting every `H`-dependent binder — the `∀ hv`/`match`/`subst`
  ritual is never written again. `ghost` clauses interleave with the
  computational `let` chain but are **stripped from the value leg**
  and replayed (source order) in the proof leg after `intro hv; subst
  hv` — facts stated where they hold, under the `Values`
  substitution, with all wires in scope (Verus `assert … by` at the
  program point). `prove` assembles the contract field-by-field.
  Implementation: three term elaborators + an `IO.Ref` queue (the
  `pendingKnots` pattern); the elaborated term is *identical in
  shape* to the hand-written defs, so generators/AxCheck/consumers
  are untouched. Since `subst` eliminates `H`, ghost statements spell
  `(Values L mem)` explicitly — as the hand proofs always did.
- **Wires are inputs** (user ruling): `leader_election`'s cycle wires
  `(p2b, a_log)` became plain binders — the returned-lambda trick
  existed only to host a *binary* mono conjunct in a unary face.
- **Flo monotonicity is a free theorem, not a face** (user insight):
  the mono half of `leader_election`'s old contract — ~100 lines of
  `iterate_mono_param` coupling with 250-char knot spellings — is
  `leader_election_param₁..₄` instantiated at `monoC/monoLaws`
  (diagonal decisions = `fun _ _ => rfl`), consumed in
  `PaxosCoreLemmas.leMono` as one corollary. The per-knot coupling
  legs (`hfl`/`hia`) inside the unary Kleene-ascent proof remain as
  ghost `hstep`, spelled over 5-char ghost-let aliases of the knots.
- `leBody`'s 500-line contract proof decomposed: shared ghost facts
  (`hflag`, `hface`, the `hflag_lt`/`hflag_true`/`hacc_len` getElem
  transports, `hzip_at`) replace per-leg re-derivations (the flag
  face was derived 3×, the zip projections 2×, six index casts
  inlined per leg); the six `prove` legs now carry only the
  solicitation chain, send-once counting, regress, and pinning — the
  actually-interesting content. `leCore` is gone (the fix body
  computes through `leBody` directly; the knot `.body` hoisting keeps
  it folded).
- Numbers: `LeaderElection.lean` 1235 → 1107 lines, max width 524 →
  84 chars; the `leader_election` def+proof 244 → 137 lines; quorum
  defs at zero time cost. Gate: 808 jobs, zero sorries, AxCheck
  standard-three, 4/4 exes.

**Verus translatability** (the denotational layer only; transfer
machinery stays Lean-optimal):

| HydroLean                              | Verus                        |
|----------------------------------------|------------------------------|
| `ensures out => P` face                | `ensures` clause             |
| `ghost let`                            | `let ghost` / spec values    |
| `ghost have h : S := prf`              | `assert(S) by { … }` / lemma call at the program point |
| `ghost obtain ⟨…⟩ := e`                | `let ghost (…) = e` destructuring |
| `ghost witness e` (D52)                | `choose`/witness for `exists` ensures |
| `ghost intro h` / `ghost subst h` (D52)| `requires` clause naming (hypotheses of an implication face) |
| `prove f := e` fields                  | postcondition obligations at fn exit |
| fold/scan step lemmas (`cqTick` etc.)  | loop invariants on the register loop |
| `fix … via fuel` + Kleene ascent       | `decreases` fuel + invariant on the unrolling |
| decision records (`LEDec`)             | ghost nondeterminism parameters |
| `_param` free theorems / `HRelC`       | no analogue — stays Lean-side (Verus would state mono per-module) |

The contract *statements* (`CQEnsures`, `LECoreEnsures`, `LEEnsures`)
and the ghost-chained proofs are the part that ports: each ghost fact
becomes an assert-by or a proof-fn call at the same program point;
the structural induction content (`cq_run_*`, `pP1b_*`) becomes
proof fns over the pure layer, which Verus handles natively. What
does *not* port is the instance-generic quantification (`H :
HydroSem L mem`) — in Verus each module is monomorphic over the exec
semantics and the spec face is stated directly; our `Values`
substitution point is exactly where a Verus `ensures` would live.
The reserved `invariant` clause on `fix` binders is the designed
landing spot for Verus-style loop invariants on cycle knots.

**Reserved/next**: `invariant` clauses on `fix` binders (parsed,
rejected with "not yet supported"); ghost clauses inside `fix`
bodies (the user wants body-inline contracts eventually — no `leBody`
split); `ensures` for named-binder-free signatures (deps are read
from the local context, so pi binders must be named).

### D52 — every module on the ghost layer; boilerplate goes to shared eliminators, not named lemmas

All remaining V2 modules now state their contracts as `ensures out =>`
faces and carry their proofs as ghost clauses along the program (D51
pilots were `leader_election` + `collect_quorum*`): `p_ballot_calc`,
`p_leader_heartbeat`, `acceptor_p1`, `acceptor_p2`, `p_p1b`,
`recommit_after_leader_election`, `index_payloads`, `join_responses`,
`sequence_payload`, `paxos_core`, `two_pc`. Zero subtype-face `match
H, hv, …` spellings remain in module signatures.

**New ghost vocabulary** (all in `HydroDef.lean`, term elaborators
over the `pendingGhosts` queue):
- `ghost witness e` — refine an `∃`-shaped ensures (`p_p1b`'s
  `okPool`, `two_pc`'s `voteYes`): the Verus witness choice, placed
  at the wire that *is* the witness.
- `ghost intro h` / `ghost subst h` — name and substitute the
  hypotheses of an implication-shaped face: the `requires` analogue
  (`two_pc`'s `num_participants = mem part`).
- `prove` with an empty ghost queue (fix for a splice bug: pilots
  always had ghosts, so plain-`exact` assembly was never exercised).

**Where the boilerplate actually went** (the user's ruling: moving
lines to named theorems is *not* reduction — only elimination
counts). Two mechanisms did the real work:
1. **Shared eliminators in `Trace.lean`** (+40 lines, once):
   `zip_map_getElem`, `mem_zip_map`, `zip_getElem`, with `Trace.zip`
   made `@[reducible]` so both spellings match. These absorb the
   ubiquitous "open a zip-of-map wire at an index / at a member"
   transport that every module re-derived inline (5–15 lines per
   occurrence, 2–4 occurrences per module).
2. **Ghost-fact deduplication along the program**: facts derived
   once at the wire that owns them, replacing per-`prove`-leg
   re-derivations. The big wins: `acceptor_p2`'s per-sender ack
   characterization was derived twice (~50 duplicated lines →
   one `ghost have hfrom`); `sequence_payload`'s `hp2a_own` was
   derived in two legs and its log face in four (→ one parameterized
   ghost have + `hlogface`).

**Census** (whole-file lines, before → after; bucket-ii moves noted):

| module              | before | after | notes |
|---------------------|--------|-------|-------|
| PBallotCalc         | 213    | 178   | zip transport eliminated; `pbcJump_le/overtakes` hoisted = **moved, not counted** |
| PLeaderHeartbeat    | 186    | 169   | nested zip transport → 2 eliminator calls |
| AcceptorP1          | 281    | 261   | reply-wire ghost dedup |
| AcceptorP2          | 476    | 426   | the duplicated ack/from transport → one ghost have |
| PP1b                | 406    | 385   | `ghost witness`; flag/accept faces deduped |
| SequencePayload     | 790    | 759   | `hp2a_own`×2, log face×4 → ghost haves |
| Recommit            | 190    | 188   | face conversion |
| IndexPayloads       | 90     | 89    | face conversion |
| RequestResponse     | 281    | 278   | face + ghost let legs |
| PaxosCore           | 136    | 133   | face conversion |
| TwoPC               | 479    | 477   | face + `ghost intro/subst`; proof was already minimal |
| **modules total**   | 3528   | 3343  | **−185 eliminated** (moves excluded) |
| Trace.lean (infra)  | 1403   | 1443  | +40, shared eliminators |
| HydroDef.lean       | 639    | 687   | +48, new ghost forms |

What remains in the `prove` legs is protocol content: quorum
crossings, ballot comparisons, write-before-ack coverage, the
solicitation chains. The pure-lemma files (`PP1bLemmas`,
`SequencePayloadLemmas`, `PaxosCoreLemmas` with the quorum-
intersection argument) are the proof-fn layer of the D51 Verus
taxonomy and stay as-is.

**Technique bank** (recurring, now canon):
- *Re-anchor before rewriting*: a bound proof whose type is spelled
  at the wire (`accepted_logs[i]` etc.) blinds keyed matching even
  with `@[reducible]`; `have h' : <denotational spelling> := h`
  first, then the eliminator applies.
- `prove` field lambdas must mirror the *implicitness* of the
  `Ensures` field binders (`fun i {t} ht …`); a mismatch fails
  silently as "unsolved goals" at the `hydro def` line.
- `prove` fields containing tactic blocks must be parenthesized —
  `(fun … => by …),` — or the `by` block swallows the comma.
- Never discard (`-`) the equation component when `obtain`-ing an
  eliminator's output: dependent bound proofs lose their anchor.

**Deferred by design** (unchanged from D51's reserved list):
`invariant` clauses on `fix` binders still parse-and-reject. The
honest assessment after this migration: every proof that would live
there currently lives in the pure-lemma layer as fuel-induction
lemmas (`cq_run_*`, `leAsc`), which is *also* where Verus would put
the loop-invariant proof fns; wiring `invariant` into the knot
hoister needs an obligation generator over the Kleene ascent and
buys no line reduction until a module needs a *new* cycle contract.
Same for ghost clauses inside `fix` bodies (the leBody fold-in).

Gate: 808 jobs green, zero sorries, AxCheck standard-three (subsets
only), falsify/explore/v2paxos/v2twopc 4/4, all headline statements
meaning-identical (faces are the same `Prop`s, now written once).

### D53 — structural fidelity to the Rust source: single-source `fix` bodies, `complete`, and the wire chains restored

The value metric for this pass (user ruling): NOT proof lines but
**structural alignment with the Rust source** — the Lean modules
should read like paxos.rs/quorum.rs/two_pc.rs, deviation by deviation.
A census (Lean module vs its cited Rust fn) found five classes; the
fixes:

**Single-source `fix` bodies (`complete`)** — the big one. Rust's
`forward_ref`/`complete_cycle.complete(…)` lets ONE call chain both
feed and close a cycle; Lean's `fix` returned only the wire values, so
the body had to be a separate named def applied twice (`pcBody`,
`leBody` — "hoisted so the knot bodies are named"). Now a fix body may
contain a `complete (e₁, …, eₙ)` marker: the chain appears ONCE; the
continuation below sees the chain's `let`s with the wires closed
(`fix` kept as the surface — user: more natural in Lean than aping
`forward_ref`). Ghost clauses inside fix bodies work (the reserved
feature): they queue once and replay at the closed wires.

Elaboration (all behind the scenes, `HydroDef.lean`):
- pass 1 elaborates the chain up to `complete` and Bekić-slices it
  per wire exactly as before — same knot constants, so
  PaxosCoreLemmas' defeq connection and the AxCheck/co lemma names
  survive;
- the whole chain is hoisted and REGISTERED as `<def>.body` — the
  generated replacement for the hand-written pcBody/leBody (the D40
  law and the generation walker's module boundary, kept as
  elaboration artifacts);
- pass 2 re-elaborates the source with wires closed, then swaps each
  chain-`let` value for projections of the registered constant —
  flattened to flat-wire leaves and pruned to continuation-referenced
  lets (the leg granularity the mono/co walkers support; subtype
  contract-fetch lets stay inline).

**Applied**:
- `paxos_core` (136→118 lines): leader_election →
  just_became_leader → sequence_payload → `complete` → outputs, in
  Rust order; pcBody deleted.
- `leader_election` (1107→982): the election chain (paxos.rs:271–345)
  and its ~15 per-pass ghost facts live inside the fix body; leBody,
  `LEWires` and `LECoreEnsures` deleted (the prove legs prove
  `LEEnsures` directly at the closed knots); `hstep`/`hknot`
  re-expressed over the generated `leader_election.body` at `MonoRel`;
  Rust names restored (wire `p1b_fail`, binders
  `p_received_p2b_ballots`/`a_log`). Elaboration 6m33s (old budget
  ≈6min held). AxCheck/HydroParamCheck renames:
  `leBody_param₁ → leader_election.body_param₁`,
  `fail_ballots_param → p1b_fail_param`.

**Merged wire chains restored** — `recommit_after_leader_election`
(190→349): the Rust 6-wire tick-local dataflow
(`p_p1b_max_checkpoint` / `p_p1b_highest_entries_and_count` /
`p_log_to_try_commit` / `p_max_slot` / `p_log_holes` / `chain`) had
been collapsed into one pure `recommitList` application; it is now a
real wire chain (mapTick/zipTick over the existing combinators, no
HydroSem changes), each wire citing its paxos.rs lines. `RCEnsures`
statements UNCHANGED (still the canonical `recommitList`/`rcMaxSlot`
faces); the bridge is `recommit_chain_eq`/`recommit_wires_eq` — the
`filterMap`/`map` fusions of the dataflow ops, proved once
(+~120 lines: the honest price of the dataflow mirror; the consumer
fix in `sequence_payload.hsent` trades `rfl` for one `show` + two
face rewrites). `rcChampCounts` names Rust's keyed-fold value.

**Small fixes**: `p_p1b`'s `fail_ballots` let moved to output position
(Rust computes `fails` at the return); `sequence_payload`'s
batch+filter_if inlined into the `index_payloads` call (Rust shape).

**Census remainder** (unfixed, with reasons):
- B1 `paxos_core.c_to_proposers`: Rust takes a CALLBACK fed by the
  new-leader announcements; Lean takes the stream. Aligning changes
  the headline statement shape — held for user decision.
- B2 `acceptor_p2.a_checkpoint`: Rust snapshots the async checkpoint
  inside the module; Lean's chain takes it pre-snapshotted. Same —
  held for user decision.
- Class D (sliced!/use::state loops as named step defs: pbcJump,
  ipStep, cqTick, jrTick, aMaxStep, p1aSendStep, spGateStep): KEPT —
  Rust's `sliced!` is itself a named delimited sub-program; the step
  fns anchor the loop-invariant lemma layer (D51 Verus row) and the
  contract statements.
- `paxos_core`'s fix binder order (a_log, seq_max) is reversed vs the
  Rust forward_ref declaration order: reordering changes the Bekić
  knot nesting and every downstream defeq — cosmetic, skipped.
- The rcGated variant gate in `sequence_payload` is an intentional
  divergence (the B2 bugfix), documented at the site.

Gotchas bank (this pass): `simp` cannot see through local `let` fvars
(state the wire-composition equality as a pure theorem over traces
and let `exact`'s defeq do the bridging — unifier unfolds let fvars
and `Values` structure projections, `simp only` does not; or use
`simp (config := { zetaDelta := true })`); lambdas in standalone
theorem statements need binder type annotations that the same lambdas
in combinator argument position infer; the generation walker needs
module boundaries as REGISTERED constants — `.val` of an
ensures-faced module in a leg diverges the glue walk (hence the
pass-2 projection swap).

Gate: 808 jobs green, zero sorries, axiom profile standard-three
(subsets only), falsify/explore/v2paxos/v2twopc 4/4; headline
statements meaning-identical (`LEEnsures`/`PCEnsures`/`RCEnsures`/
`SPEnsures` untouched).
