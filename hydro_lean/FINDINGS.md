# hydro_lean Findings Report (the D-ledger)

Issues, gaps, and formally-surfaced contracts discovered while mechanizing the Hydro
dissertation (Chs. 2–4) and porting Hydro programs to Lean. Each finding states where it
was discovered, its status, and the artifact that witnesses it.

> **Paths.** Entries before D68 cite the library as `HydroV2/…` and the V1 metatheory
> tree as `HydroLean/…`. Since D68 the library is `Hydro/…` (same files, renamed) and the
> V1 tree is deleted; `HydroV2.X` declaration names read `Hydro.X`. Entries are historical
> records and are not rewritten.

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

### D54 — the nondet/sched-det vocabulary split: decision records carry only content nondeterminism; the adversary gets its own bundle, census-enforced

**The mandate** (user-directed): the `…Dec` records had accreted three
distinct kinds of freedom under one name, muddying what "∀ d, safety"
quantifies over. The split, ratified from instance ground truth (what
is `Unit` where):

- **nondets** (content — the Rust `nondet!` sites): `SnapDec`,
  `BatchDec`, `OrdBatchDec`, `OrderSelDec`, `BatchOrdSelDec` (real at
  `Values`, `Unit` at `SchedSem` — the machine *derives* them), and
  the timing family `SampleDec`/`TimerDec`/`PulseDec` (real in BOTH
  legs: at `Values` they are the decision, at the machine they are
  observed timing data — a correction to the earlier framing that
  timing was machine-only).
- **sched-dets** (adversary/scheduling — `Unit` at `Values`, real at
  the machine): `TransportDec` (delivery cursors) and `EmitDec`
  (emission linearizations). Rust tally: transports DO carry
  `nondet!(/** TODO */)` in paxos.rs (`broadcast`/`send_bincode`
  sites); emits mirror the `NoOrder` output typing and carry no
  `nondet!`; cycles carry none.
- **fuels**: `FixDec` — kept in the Dec records (truncation decisions,
  the `Values` fuel; `Td + 1` at the corner).

**Mechanism** (zero `HydroSem` restructuring — no parametricity risk):
per-module hand-authored `<M>Sched` structures (the D43 authored-Dec
precedent) with one trailing `sched` binder after `dec`; `.triv`
constructors at `Values` (all fields `Unit`); ensures faces never
mention them. New records: `CQSched`/`CQWRSched`/`JRSched` (Std),
`PLHSched`/`AP1Sched`/`AP2Sched`/`PP1bSched`/`LESched`/`SPSched`/
`PaxosCoreSched`/`TwoPCSched` (nested compositionally, mirroring the
Dec nesting). Enforcement: `#nondet_census M (nondets := a)
(scheds := b) (fuels := c)` (implemented in `HydroDef.lean`) walks the
def's pi binders + recursive record fields, classifies leaves by head
constant, and build-fails on drift. Census table (build-checked):
collect_quorum 1/1/0, cqwr 1/1/0, join_responses 1/1/0,
p_leader_heartbeat 3/1/0, acceptor_p1 **0**/1/0, acceptor_p2 1/1/0,
p_p1b 3/1/0, sequence_payload 4/5/0, leader_election 8/4/3,
paxos_core 12/9/5, two_pc 2/6/0. `acceptor_p1`'s 0/1/0 is the
sharpest reading: phase-1 acceptors are *deterministic* given
delivery — every prior "decision" was scheduling.

**What the generators needed (three real bugs, all the same shape).**
The engine assumed exactly one decision record per module; with two
(`dec` + `sched`) three sites pushed the composed `vdec` (an
`LEDec`-typed record) into the *sched* slot, producing **ill-typed
generated statements** that surfaced only as naming-tactic failures
("rewrite: did not find an occurrence", "motive is not type correct")
— because knot/glue statements are built by raw `mkAppN` with no
typecheck until the script runs. Fixed by the field-tree test the
direct route already had (`treeHasContent`): `knotRrArgs` and
`fmDecVal` (HydroGenKnot) push the vdec only for content-carrying
records and repack sched-only records field-wise to `Values`
(all-`Unit`); the glue rr-assembly does the same; `extractGlueVdec`
now selects the *content* record instead of the last `.decRec` binder
(which had become `sched`). Debugging lesson (extends the D37/D41
ledger): when a `first | rw [A] | rw [B]` reports B's failure, A may
have matched and failed *later* in its branch — and an ill-typed
raw-built statement reports as A-failed-to-match; dump the goal and
look at what sits in each record slot before theorizing about
unification.

**Downstream re-shapes** (mechanical, meaning-identical): param free
theorems gained `sched sched'` binders + per-sched-leaf premises
after the dec premises (LE: 11 dec rfls + pair + 4 sched rfls;
paxos_core: 17 + pair + 9); `paxos_eager_den`/`_ballots`/`_commits`
statements carry `pcschedE`/`PaxosCoreSched.triv` (Unit records — a
unique value, so meaning-identical); `paxosVDecG`'s outer order
decoded from the generated `paxos_core_vdec` signature
(sample/timeout/interval, p1aCh, ial, p1bCh, cqwrEmit, rcEmit, p2aCh,
p2bCh, cqEmit, jrEmit); drivers/checks (V2Paxos, Falsify, Explore,
TransferCheck, Liveness×2, V2TwoPC) updated; `two_pc` split into
TwoPCDec {votes, acks} + TwoPCSched (transports + CQ scheds) and
gained its census.

Gate: 808 jobs green, zero sorries, axiom profile standard-three
(subsets only), falsify/explore/v2paxos/v2twopc 4/4, all 11 censuses
enforced in-build; headline statements meaning-identical
(`paxos_safe_sched'`/`cq_safe_sched'`/`PCEnsures` faces untouched —
the sched records appear only as quantified machine data, exactly
where cursors/linearizations already lived).

### D55 — B2 landed (the checkpoint snapshot moves inside `acceptor_p2`); B1 dropped by ruling; the boundary-vocabulary design; five engine fixes for phantom generics

**The rulings** (closing D53's census remainder). B1
(`c_to_proposers` as a callback) is **dropped**: safety quantifies
over all payload streams, and every callback-generated stream is one
of them — the callback shape adds content only for liveness-style
statements (client sends after learning a leader) or structural
fidelity; nothing current pays for the engine surgery (function-typed
program args). B2 is guarantee-neutral by the same argument (every
snapshot outcome of every async checkpoint IS some input trace) but
was **built for structural fidelity**: the module signature and its
`nondet!` census now match paxos.rs exactly.

**The boundary-vocabulary design** (the interesting part — a user
design session). Rust's `Optional<usize, Cluster<Acceptor>,
Unbounded>` erases its provenance; Lean's async `Singleton ℓ α σ ord
ret b` is graded by its *source* vocabulary, because the carrier is
the pre-snapshot object — a read function `CutDec α ord → Trace σ`
whose "advance or not advance" freedom IS the cut decision, with
legality (count/membership honesty against the source pool) keyed by
the source grade; proofs consume that legality (the received-max
regress), so erasing it inside a program is not an option. At module
*boundaries* the grading over-specifies. Options weighed: an erased
async-singleton former (new decision vocabulary, severs pool
legality); a dependent type member `SnapDecOf : Singleton … → Type`
(expressible, but moves decision types from signature-level indexed
families into value-dependent territory — breaks the mechanical
parametricity lift, the corner derives, and the type-level work the
indices do). **The answer: anonymous generics.** The module is
parametric in the upstream vocabulary —

    hydro def acceptor_p2 {ckα} [DecidableEq ckα] {ckord} {ckret}
        (H : HydroSem L mem) …
        (a_checkpoint : H.Singleton acc ckα (Option Nat) ckord ckret
          .unbounded)
        (dec : AP2Dec H … ckα ckord) …    -- ckSnap : SnapDec … ckα ckord

— the snapshot decision borrows the same vocabulary and the module
hands it to `H.snapshot` opaquely; meaning stays owned by the
upstream grade. This is the dependent-types intuition with the
dependency at the binder level instead of packed in a sigma. The
generics thread `acceptor_p2 → sequence_payload → paxos_core`
(records `AP2Dec` — new, bundling the P2a batch with the checkpoint
snapshot — `SPDec`, `PaxosCoreDec` gain `(ckα, ckord)` params);
ensures faces state checkpoint clauses at the applied read
`fun j => a_checkpoint j (dec.ap2.ckSnap j)`; censuses: acceptor_p2
1/1/0 → **2/1/0** (= Rust's in-module `nondet!` tally),
sequence_payload 5/5/0, paxos_core 13/9/5.

**The engine law: generics go in the context telescope (before `H`),
and context telescopes apply positionally.** Everything before the
`HydroSem` binder is opaque `ctxArgs`; phantom implicits (`ckord`/
`ckret` appear in NO machine-typed argument — machine carriers erase
them) can never be pinned by unification, so every generated-artifact
application must thread the context POSITIONALLY. Five engine fixes,
each found by the house toy-repro method (a knot capturing a
singleton across the boundary — `ScratchB2`, deleted after use), not
by profiling (which has historically found nothing):
1. `applyNamed` gains a positional `pre` telescope (replacing
   `explicitOnly`-then-unify at `applyModLemma`/`vdecCompOf`);
2. the two wf-stage `mkAppM lem` sites (causal/mono `go`) likewise;
3. `CoSing.lower`/`CoSing.lowerC` + rfl lemmas (the `hcplj` lowering
   dispatched only CoTickSing/CoStream — first singleton captured
   across a knot ever);
4. `valuesRelOf`/`valuesRelRefl` gain `.sing` cases (`SingRel`'s
   read-prefix-at-every-cut shape);
5. the earlier D54 lesson replayed: a bare `H.FixDec` binder consumed
   inside a `fix` body is ill-typed across the `∀ H'` boundary — fuels
   must live in decision records (the toy's initial `via df` hang; the
   real modules were already correct).

**Corner/headline restatement**: the corner's `snapshot` derives the
`Values` cut decision from the machine leg (`snapDerive` at `Td`) and
spends the `CoSing` fold provenance, so `paxos_safe_sched'`'s
checkpoint input premise became the honest boundary form
(`CoSing.inputC`): machine and denotational legs quote a common fold
over a source dominated by a common pool — the input-side mirror of
what `fold` sites establish internally. Statement otherwise
unchanged; still premise-free in the decision space.

**Driver vocabulary**: executables pin `ckα := Nat, ckord :=
.totalOrder, ckret := .exactlyOnce`, read functions
`fun (d : List Nat) => d.map (fun _ => none)` (annotate the binder —
`CutDec` is a match, stuck at a metavariable ord; and name
`α/ord/ret` at `EagSing.input`, ret is phantom). New helpers:
`EagSing.input` (+ den lemma), `CoSing.inputC`. `sequence_payload`'s
mono lemma states the checkpoint premise in `SingRel` form and pins
`(ckret := ckret)` at the `MonoRel` instantiation (phantom again);
same named-pin on `paxos_eager_*`'s `Values`-side applications.

**Open perf item**: `PaxosCore.lean` elaboration 25 s → 298 s with
the change (the generated pipeline through the singleton-carrying
records; LeaderElection unchanged at ≈6.5 min). Worth a look next
time the engine is open — likely the same class as the D44 heartbeat
notes (choice-route ex-script at bigger statements), not a defeq
cliff (it completes).

Gate: 808 jobs green, zero sorries, axiom profile standard-three
(subsets only), falsify/explore/v2paxos/v2twopc 4/4, all 11 censuses
build-enforced; `paxos_safe_sched'`/`PCEnsures` meaning-identical
modulo the checkpoint input's honest boundary form.

**Erratum to D54's Rust-tally reading (user catch).** D54 said "Rust
marks transports with `nondet!(/** TODO */)`". Wrong attribution: the
parameter at dynamic `broadcast` is literally `nondet_membership` —
it guards the **cluster-membership snapshot** (async member joins;
networking.rs's `broadcast` snapshots the tracked membership under
that guard), and `broadcast_closed` — the closed-membership
counterpart our model realizes by type-level `Fin n` (the D20/C-table
finding) — takes no `nondet!`; `demux`/`send` take none either. So
transport-site `nondet!`s correspond to membership freedom we
*assume away*, NOT to `TransportDec` (delivery cursors — machine
freedom Rust never marks). Consequences: `acceptor_p2`'s Rust tally
is **2**, exactly its census (the demux was never a `nondet!` site);
a module's Rust `nondet!` count = its census nondets + its dynamic
broadcasts (membership, unmodeled). `Sem.lean`'s accounting
paragraph, the five transport-field docs, and the module tally notes
are corrected; the census numbers themselves were always right.
Follow-up rename (user ruling): the op the model implements IS
Rust's closed-membership `broadcast_closed` (total deterministic
fan-out to `Fin n`; `Values.broadcast_closed _d s := fun _p j => s
j`), so the `HydroSem` field — and its whole derived lemma family
(`_sr`/`_rr`/`_den`/`_wf`/`_unit`, instances, walkers) — is renamed
`broadcast` → **`broadcast_closed`**; the API no longer implies the
dynamic-membership op is modeled. The programs' Rust-quote comments
keep the source text (`.broadcast(…, nondet!(…))`) — that mismatch
is exactly the D20 upstream note, now visible at every call site as
`-- .broadcast(…)` above `H.broadcast_closed …`.

### D56 — the PaxosCore 296 s: six failed `isDefEq`s in the glue-mono resolver, killed by a keyed pre-filter

**The regression (D55's open perf item), profiled.** `PaxosCore.lean`
25 s (pre-B2) → 296 s. `trace.profiler` (threshold 3 s) decomposes
the command pipeline: def elaboration 7.3 s, `hydro_glue
paxos_core.body` 9.7 s, `hydro_wf` 3.3 s, **`hydro_mono
paxos_core.body` 258.4 s**, `hydro_param` 5.0 s (everything else,
including both wire-knot pipelines, under 3 s). Inside `hydro_mono`:
**six failed `Meta.isDefEq` calls at 31–45 s each = 251 s** of the
258. All six have one shape — `resolveRel` (`genGlueMono`'s
candidate resolver) tries a callee mono lemma by
`forallMetaTelescope` + `isDefEq concl goal` at default
transparency, and the near-miss candidate is the **same callee at a
different output leg**: lemma `(↑(leader_election ?args…)).2.1 ?i
<+: …` vs goal `(↑(leader_election args…)).2.2.1 i <+: …`. The
unifier happily assigns every argument mvar, then must refute `.2.1
=?= .2.2.1` of the *same folded application* — which it can only do
by delta-unfolding `leader_election.body`, i.e. whnf'ing the
whole-program knot chain (~44 s per candidate; one of the six blows
the heartbeat inside `observing?` and burns 31 s before the catch).
This is the D44 whole-program-defeq class arriving through the
unifier's delta path **on failed candidates** — exactly the case
`resolveRel`'s comment assumed away ("the module delta this crosses
is zeta/beta/proj only, every callee stays FOLDED": true for
matches, false for near-misses). B2 didn't create the hazard, it
surfaced it: the checkpoint-carrying records added enough candidate
trials (the new `SingRel` premise resolves recursively through the
same loop) to hit six near-misses.

**The fix (engine-only, D49's keyed-matching precedent ported to the
mono resolver).** `subjKeyOf`: the cheap terminal key of a relation
argument — descend `Prod.fst`/`Prod.snd` applications and native
`.proj` steps recording the projection path, cross `Subtype.val`
transparently (the ensures-face coercion), ignore trailing index
applications; stop at the first non-projection head constant.
`conclKeyMismatch`: skip a candidate **only** when the relation
heads and arities agree positionally AND some argument pair has
callee-headed keys on *both* sides that disagree (same callee,
different leg — or different callees). Under the folded-callee
invariant a genuine match always agrees on head + path, so nothing
is lost; anything uncertain (combinator-wrapped subjects like
`Values.forgetBound (…)`, mvar heads, differing relation heads)
falls through to `isDefEq` unchanged. Four lines in `resolveRel`
before the unification, two ~25-line helpers.

**Measurements** (full fresh builds, exit 0): PaxosCore **296 s →
49 s**; LeaderElection 406 s → 394 s (its cost is the knot pipeline,
not glue mono — unchanged in-family); SequencePayload 28 s → 28 s.
Zero over-filtering: per-module `hydro_mono` generated-lemma counts
identical to baseline across all 15 mono-bearing modules
(`paxos_core.body` 9, `leader_election.body` 13, …), every consumer
green. The remaining 49 s re-profiled **flat** (threshold 2 s: def
7.4, glue 9.9, mono 7.0, param 4.9, wf 3.3, wire knots 3.0 + 3.7, +
under-threshold tail — no node ≥ 2 s inside any phase): that is
legitimate B2-scale work (bigger records/premises through the
singleton-carrying pipeline), not another cliff; 49 s vs the 25 s
pre-B2 floor is the honest cost of B2's structural fidelity at
current engine shape.

**Gotcha for the ledger.** A *failing* `isDefEq` between two
projections of the same folded module application is a whole-program
whnf, even when every visible argument unifies instantly — the
refutation cost lives after mvar assignment. Any generator that
tries candidates by conclusion unification must key on
head-constant + projection path first (`matchLemma` learned this at
reducible transparency in D49; `resolveRel` needed the keyed skip
because its default-transparency match is load-bearing for real
matches). Profiling found this in one pass (135 s of trace on a
296 s file) — the house rule stays "toy-repro for wrong-answer bugs,
profiler for slow-answer bugs".

### D57 — the proof-cleanup pass: K4 becomes the `fix` `invariant` clause; any-body truths move to the engine; the outside proof is contract composition only

**The user's mandate, sharpened over the pass**: a high ratio of
Paxos-specific content in the proofs; anything true for ANY fix body
belongs to the engine; proofs must sit with the program ("we wouldn't
be invoking paxos core from outside"); and — the decisive ruling —
**the invariant machinery is part of the `fix` construct, and no
proof outside the fix mentions stages**.

**GC (riders)**: nine dead lemmas deleted (~110 ln): PP1bLemmas'
`foldEarlyStop_full_pin`; Types' `LogMap.insertMax`/`find?`; Trace's
`scanAcrossTicksTrace_getElem`/`_states`, `snapshotCuts_length_le`/
`_all_of_le`, `batchCuts_all_of_le`, `MonoTrace.vals_ext`
(`MonoTrace.map` survived — a dot-notation consumer the bare-name
grep missed: verify deletions by BARE last-component grep). Discovery:
`leader_election_mono₁..₄`/`sequence_payload_mono₁..₃` were ALREADY
generated in exactly the diagonal form the hand `leMono` (65 ln of
`_param` diagonal filler) and hand `sequence_payload_mono`+repackers
(~60 ln) re-derived — both deleted, consumers rewired.

**Engine, part 1 — `genKnotStages`** (HydroGenKnot): every knot now
also generates the Kleene-chain vocabulary — `K.stages` (the `Values`
iterates at the module boundary), `stages_zero`/`_succ`/`_fix` (rfl)
and `stages_mono` (`iterate_chain` over the body's composed mono).
Supported by `PoolLe.bot_le` (Grades) and `prefix_getElem_lift`
(Trace). These are INTERNAL vocabulary: consumed by the invariant
clause below (and available to any future chain reasoning).

**Engine, part 2 — the eager knot-stack drain** (HydroDef):
`hydro_knot`/`hydro_glue`/`hydro_couple`/`hydro_causal`/`hydro_wf`/
`hydro_mono` cores extracted into idempotent `TermElabM` functions
(`run*T`); `elabFixTerm` drains `pendingKnots` right after hoisting
(chain body first, knots innermost-first), so everything generated is
available to ghost clauses, prove legs, and the invariant clause OF
THE ENCLOSING DEF. The post-def pipeline pass no-ops on generated
stacks; only `hydro_param` stays post-def. `pendingHints` carries the
def's unfold hints to the eager pass.

**Engine, part 3 — the `invariant` clause** (D51's reserved slot,
now live; Verus-ordered: clause before the body):

    fix (w : τ) … via fuels
      invariant w (le sp le' sp') => P, base := prf, step := prf
    := body

- The predicate relates the fix body's own chain-`let` runs at the
  CURRENT wire (unprimed) and at the CLOSED knot (primed) — the
  binder names are the program's `let` names; nothing is respelled.
- The obligations are stated over transparent LET binders (the runs +
  `<wire>'` for the closed knot; `intro` them in tactic proofs), with
  auto-premises: the ensures faces of every subtype-valued chain
  `let` at all three valuations (`hle`, `hle_step`, `hle'`, …), the
  chain orders `hw : w ⊑ closed`, `hbw : step w ⊑ closed`,
  `htop : closed ⊑ step closed`, and the induction hypothesis.
- The construct composes `iterate_invariant` (Trace) with the
  generated `stages_mono`/`stages_fix` and hoists
  `<def>.<wire>.inv : ∀ args, I(closed, closed)` — the ONLY thing
  consumers see. No stage index appears in any user-written text.

**K4 rewritten** (PaxosCore.lean, one self-contained file; the
560-line PaxosCoreLemmas.lean is DELETED): the invariant is the
protocol statement — "chosen-at-b₁ at the closed knot dominates: any
current-stage emission at a higher ballot carries the chosen value" —
conditional on `variant = .guarded` and quorum intersection
(`mem acc ≤ 2f+1`). `base` = no emissions over the empty seed;
`step` = THE Paxos regress (promise quorum ∩ vote quorum, the
vote-precedes-promise asymmetry on the ascending max wire, coverage
ascent to the promise tick, the champion regress, send-once
dichotomy) — spelled entirely at `le`/`sp`/`le_step`/`le'`/`sp'` and
the faces. The top-pinned binary form ELIMINATES the old
`max(k+1,K)` stage juggling: all lifts go up to the closed knot via
`hw`/`hbw`/`htop` (one `hpublift := (hbw ⬝).trans (htop ⬝)` replaces
the knot-equation rewrites — the old `hknot`/`pcAlogW_succ` bridge is
now definitional inside the construct). The `slot_functional` prove
leg is contract composition only: `commit_spec` ×2, ballot
trichotomy, `paxos_core.a_log.inv`, send-once — final-stage
statements exclusively.

**Numbers**: PaxosCoreLemmas 560 → 0; PaxosCore 141 → 554 (program +
contract + THE WHOLE agreement layer); net −147 across the pair, with
the surviving lines ~all Paxos-specific (the step is ~200 lines of
pure protocol argument; the scaffolding count — stage vocabulary,
Kleene plumbing, mono filler, face fetches at stages — went from
~200 ln to 0 user-written). PaxosCore elaborates in 63s (baseline
49+9.8 split; the +4s is the invariant machinery), LeaderElection
~390s (in-family), full build ~8m30s. Gate: exit 0, zero sorries,
AxCheck standard-three incl. `paxos_core.a_log.inv`,
falsify/explore/v2paxos/v2twopc 4/4, censuses build-enforced.

**Verus translatability** (the pass's real payoff): the clause IS a
loop invariant in Verus ordering; `base`/`step` are the entry check
and the inductiveness obligation Verus discharges around the loop
body; the auto-premise faces are the callee postconditions in scope
at the loop; `<wire>.inv` is the post-loop assert. The top-pinned
binary invariant shape (relate the running state to the FINAL state,
premises `⊑`) is the mechanical translation of Paxos's
history-variable argument into a loop invariant.

**Gotchas bank (this pass)**:
- `subst` cannot eliminate an fvar that let-binder VALUES depend on —
  inside invariant obligations keep `variant` generic, `rw [hvar]`
  only at emission-typed hypotheses, and pass `hvar` to the guarded
  face fields (`providers`, `log_entry`, `commit_spec`).
- tactic `intro` CONSUMES the obligation's let binders (term-mode
  `fun` skips them) — intro the run names first.
- un-ascribed `have := <generated mono> … (fun _ => PoolLe.refl _ _ _)`
  leaves the diagonal-input implicits unsolved — pin them by name
  (`(p_received_p2b_ballots := …)`); the old ascribed-statement style
  pinned them implicitly.
- meta: local `fun … => do` closures default their monad
  unpredictably — ascribe (`: … → TermElabM Expr`); a failing
  `isDefEq` between whole-knot spellings stays the D56 hazard — the
  invariant machinery avoids candidate trying entirely (everything is
  pinned).
- `obtain ⟨…⟩ := h` CLEARS `h`; use `:= id h` when the packed fact is
  needed again (the invariant's `hch` feeds both the destructure and
  the IH).

**Remaining (successor)**: P4 — SequencePayloadLemmas' three
monolithic scan inductions (~1030 ln) restructure (named scan-input
abbrev + invariant bundling); P5 — Std/Quorum cq/cqwr twin dedup
(~600 ln twins); optional ghost-face sugar so prove legs get closed-
run faces without the two one-time respells in `slot_functional`.

### D58 — ghost proofs by composition (zero respells); the acceptor-GC audit; the `use::state` ground truth and the ratified `tick` construct (A1's hand attempt retired by design)

**Task 1 — the closed-wire respells are gone.** The user's question
("why is the prove leg re-executing `leader_election`/`sequence_payload`?")
had a two-part answer. (a) Nothing re-executes: at `Values` a module
application is a pure denotation and `.property rfl` merely projects
the PROOF component of the ensures-subtype — definitionally the same
value the body named `le`. (b) The syntactic respelling was still the
D52 disease, and the cure was composition, exactly as the user
guessed: `ghost have hle := leS.property rfl` written INSIDE a fix
body replays in the prove leg AT THE CLOSED WIRES (pass 2 re-elaborates
the body with wires closed; LeaderElection already used the pattern —
PaxosCore's body just predated it, binding `.val` directly with no
name for the subtype). One small engine addition completes the story:
`elabFixInvariantClause` now auto-queues `have h<wire>_inv :=
<def>.<wire>.inv <args by name>` (args = the knot telescope's explicit
binders — all in scope in the leg), so the packaged induction also
arrives as a named ghost. PaxosCore.lean now contains ZERO hand-spelled
module applications; the `slot_functional` leg is: `subst; hI :=
ha_log_inv rfl hnA; hreq' := hle.spRequires …; two commit_specs from
hsp; dichotomy; hreq'.send_once`. Engine notes: the auto-ghost args
must come from the OUTER telescope only (re-telescoping the hoisted
statement walks into the invariant's own binders); and a 2-space
indent slip nested the generation under a dead `if` with NO error —
the invariant vanished tree-wide, caught only by the `#check` guard
(defense-in-depth for generated artifacts, again).

**The ratified rider.** `LEEnsures.spRequires` (PaxosCore.lean — the
composition point; LE's face projected to SP's input requirement) and
`SPRequires.send_once` (SequencePayloadLemmas.lean — key-nodup +
injectivity, packaged) deleted ~100 duplicated lines across the K4
step obligation and the prove leg; PaxosCore elaboration 64 s → 50 s
(the duplication was real elaboration cost).

**The acceptor-site audit (user ruling 2, plus the GC exchange).**
Stale doc comments claimed `persist()`; the Rust truth
(paxos.rs:851–873) is `across_ticks` + keyed `reduce_watermark` with a
`commutative = manual_proof!` obligation. The model's
accumulate-all-multiset + view-time `logView` is extensionally the
keyed reduce, with the equal-ballot tie (Rust's `TODO: need assume`)
made CHECKABLE (the `logView` conflict branch + slot-functional
contract). `reduce_watermark`'s library-internal
`assume_ordering/assume_retries` nondet!s are discharged by the
commutativity proof — principled census convention: commutative-
discharged assumes take no decision; order-sensitive ones (PP1b's
`fold_early_stop`) are `OrderSelDec` nondets. **Watermark GC is
unmodeled and provably safety-neutral within `paxos_core`** — the user
drove the resolution live: replicas are OUTSIDE `paxos_core` (both
sides — `p_to_replicas` is an output, `a_checkpoint` an arbitrary
input); the proposer mirrors the watermark discipline (`recommitList`
skips slots ≤ view-checkpoint, holes from `ckpt + 1`, Recommit.lean =
paxos.rs:606–670), so a GC'd slot's acceptor carries `ckpt > s` in the
same view and no new emission at `s` can occur; theorem-level
confirmation: `paxos_safe_sched'` has NO checkpoint premise. What
early GC threatens is LIVENESS at whole-system scope (committed-but-
GC'd-before-execution slots = permanent holes = replicas block); the
"checkpoint never outruns replica execution" invariant belongs to the
unmodeled replica feedback loop — a premise for a future
replica/commit-liveness rung. AcceptorP2's header now says all this.

**The scan ruling's ground truth.** `use::state` IS a tick cycle:
hydro_lang/src/live_collections/sliced/style.rs:114–127 ("Creates a
stateful cycle with an initial value… `cycle_with_initial`"); the
`mut` rebind at the slice's end is the complete. Every cross-tick
state in paxos.rs/quorum.rs/request_response.rs is `use::state`/
`use::state_null` inside `sliced!`; no scan operator exists in Hydro.
Program-site inventory for the migration (7 sites):
LeaderElection:382 (B1 gate), PBallotCalc:120 (paxos.rs:363),
IndexPayloads:45 (paxos.rs:783), SequencePayload:158 (B2 gate),
Quorum:1372/1414 (quorum.rs:28/103 ×2 registers each),
RequestResponse:247 (request_response.rs:20). Acceptor folds stay
op-shaped (they mirror `reduce_watermark`, not `use::state`).

**A1's hand attempt — landed engine content, retired program surgery.**
The register-as-4th-fix-wire version was built end to end and
abandoned by design (the user's reaction to the 200-line take-algebra
tick lemma was correct: "I thought with the invariant syntax it would
be easier"). What LANDED and stays:
- `relWB` — step obligations now also receive the ONE-STEP chain order
  `w ⊑ closing w` (from the already-generated `stages_mono`; intro
  lists gained `hbw0`). The user's monotonicity insight: ticked
  carriers grow only by appending ticks, so this plus letwise mono is
  what reduces a stage step to a per-tick argument.
- `prefix_ext_invariant` (Trace.lean): the tick-axis leg of the
  two-axis induction — a property preserved per appended tick
  transfers along any prefix extension (~30 ln, pure; the ONLY new
  generic lemma the whole tick story needs).
- `zip_map_prefix_left` (Trace.lean): kept as a pure List lemma
  (NOT engine glue — see the shape ruling below).
- **Engine bug (fixed): `mkObligation`'s value→binder rewrite.** It
  replaced binder values in reverse binder order; when two valuations
  share a value (a wire-independent run at `w` and at its image — the
  LE tie case), the later binder consumed occurrences INSIDE the
  yet-unprocessed containing values, the earlier binders went unused,
  and `mkForallFVars` silently DROPPED them — the obligation telescope
  shifted and intro lists mis-bound. Fix: size-DECREASING replacement
  order (containing values before their subterms) with earlier-binder
  tie-break. K4 never tripped it (its values nest within one
  valuation).
- **Engine guard (new): dead-fvar check on the packaged induction.**
  A mis-named hypothesis inside an obligation tactic (`_hbc` intro'd,
  `hbc.own` used) error-recovers into a proof term with DEAD free
  variables; `addThm` accepts it, and only the ASYNC kernel rejects —
  far from the cause, with cascading whnf timeouts downstream. The
  clause now collects fvars of stmt/val and fails loudly at generation.
- Honest residue: even with the fixes, the hand version's prove legs
  hit multi-second `isDefEq` cliffs that scale with the (now deeper)
  knot tower (profiled terminal node: a `Multiset.Le` unification
  inside the solicitation leg at 13.7 s, total 122 s → heartbeat
  death). The per-site hand bookkeeping is structurally the wrong
  interface — which is the design argument for the construct below.
  LE was restored to the validated scan version; LEDec/censuses
  unchanged; the net tree state keeps Task 1 + rider + engine fixes.

**The ratified `tick` construct (successor spec — user co-designed,
every point below is a ruling).**
1. `tick` (user-named: "it's declaring a tick") corresponds to Rust
   `sliced!`, NOT to a single `use::state`: it is a tick block that
   may declare MULTIPLE `state` registers; they compile to ONE
   product-state knot (all reads deferred ⇒ jointly guarded; one
   fuel). Shape: `tick via <fuel> (state r : σ := seed)+ invariant
   (letIds) => P := <ticked let-chain> rebind (r := …,…)
   emit (out := …,…) prove tick := <proof>`.
2. `state` reads are implicitly the PREVIOUS tick's value — the
   construct owns the defer ("own the shape, don't check it"; a
   forgotten defer is unrepresentable). Batch/atomic entry devices are
   NOT mirrored — our ticked collections already model tick entry;
   input nondets are paid outside the block.
3. Proof placement: predicate before the body (true Verus order),
   obligations TRAILING (`prove tick := …`) — Verus has no
   proof-before-body (preservation is SMT-implicit/inline asserts).
   `fix`'s `base := …, step := …` should move to a trailing `prove`
   in the same pass for uniformity.
4. **No operator-shape assumptions** (ruling): the construct must not
   recognize `mapTick`/`zipTick` compositions — future tick operators
   must participate by the type-system route, "just like mono": the
   per-leg lifts come from the body's MonoRel property (per-op laws
   composed by the existing walker), per-tick values are definitional
   (each op's `Values` denotation computes at an index; `defer`'s
   read-link is `rfl`), and the only generic lemma is
   `prefix_ext_invariant` (already proven). Per-LET obligations (no
   zip packaging — ruling); lockstep length facts close definitionally
   per leg and surface as visible side goals if an exotic op ever
   breaks them.
5. The user obligation is the loop-body proof: `P (emissions-so-far)
   (reg) → P (emissions ++ tick's emit) (tick's rebind)` — invariants
   in loop-invariant NORMAL FORM (emissions so far + current register,
   NOT ∀-t history-indexed statements: that transliteration is what
   ballooned the hand attempt). B1's whole layer becomes `p1aSendStep`
   + a ~40-line fire/hold lemma + the clause.
6. `fix` + `base`/`step` stays for genuine forward_refs — `a_log`'s
   same-tick write-before-ack must NOT auto-defer; the two constructs
   mirror Rust's two-level hierarchy (`tick.cycle()`/`forward_ref` vs
   `use::state`-sugar-over-cycle).

**Verus rows.** `tick`/`state` = `while` + `mut` loop variables; the
invariant clause = the loop invariant; `prove tick :=` = the loop-body
proof; `fix` = recursion with fixpoint induction. The GC note:
boundary premises ("checkpoint ≤ replica execution") become
`requires` clauses of the enclosing component, not loop invariants.

### D59 — the `tick` construct lands as a structural fold (`scan_across_ticks` + one generic loop-induction lemma); the knot route for ticks is retired; LE's B1 invariant rides the new surface

**The pivot, and the ruling that forced it.** D58's ratified spec was
built: tick desugared to a fueled single-wire knot, the invariant to
the two-axis induction (`iterate_invariant` × `prefix_ext_invariant`,
per-leg chain monos lifting `stages_mono` at `m ≤ m+1`). Toys were
green — and `leader_election` died at glue sr-leg 11, the first chain
leg CONTAINING an inline closed knot: the callees-stay-folded
discipline does not survive a knot that is not a top-level callee (its
co-rules exist and verify, but the normalizer descends anyway; body
folding did not save it). The user then re-cut the design space:
*"I have no attachment to the current macro implementation — all I
care about is the syntax, and using the type system to auto-derive
facts"*; the macro must do **no op/AST analysis** (the same rules must
port to Verus, which has no macros); hoisting tricks are unnecessary
now that `ghost have` replays into prove legs. And the semantic
observation that unlocks everything (user-confirmed): **Hydro compiles
no instantaneous cycles** — every cycle passes `defer_tick` or a
network boundary — so a tick register is not a fixpoint at all. It is
a **structural fold over the local tick index**. `fix` remains a fold
over async exchange ROUNDS (= the Kleene stages); prefix-ascent
(`stages_mono`), decided-at-index immutability, and
no-instantaneous-cycles are one triangle: each is the other two.

**What `tick` is now.** Surface (HydroTick.lean, syntax-transform
only — no elaborator-side typing):

    tick (state r : σ := seed)+ (input x := t)?
        (invariant (out r… spect…) => P) :=
      <per-tick body — r, x are THIS TICK's VALUES>
      rebind (r := e,…) emit (out := e')
      (prove init := …, tick := …)?;
    <continuation>

desugars to `let x := t; let out_step := fun me r… x => ((rebinds),
emit); let out := H.scan_across_ticks x out_step (seeds); ghost have
hout_inv : ∀ i, P (out i) (state-projections of scanAcrossTicksState
(out_step i) seeds (x i)) (spect i)… := by have hinit : … := $init;
have htick : … := $tick; exact fun i => scanAcrossTicks_invariant …`.
No fuel, no knot, no `FixDec`, no stage induction: the register loop
is an ordinary op (`HydroSem.scan_across_ticks`, the construct's
semantic former — an engine field, deliberately NOT a user-facing
operator, since Rust has none), and the whole invariant story is ONE
generic lemma, `scanAcrossTicks_invariant` (Trace.lean): plain loop
induction `P [] seed → (per-tick append step) → P (trace) (final
state)`. The user obligations are in Verus loop normal form —
`init : P [] seeds` and `tick : out.length = n → P out st →
P (out ++ [(step st in[n]).2]) (step st in[n]).1` — handed the
register DIRECTLY (no read-link, no take-algebra). Multiple `state`s
productify (match-pattern step binder + projection splices); the
invariant ghost replays into prove legs like any ghost; everything is
definitional at `Values`, so tick programs stay axiom-clean.
`HydroTickCheck.lean` pins the three toys (plain running sum;
invariant + spectator consumed through a prove leg; two-state).

**LE under the new surface.** The site is 9 lines (state/input/
invariant/rebind/emit/prove); the preamble is only `p1aSendStep` +
`p1aInvariant` + `p1aInvariant_tick` (append form). Everything else
moved INTO the body as ghosts, per the user's follow-up ruling
(preamble theorems that exist for one consumer should be ghosts that
refer to the wires locally instead of being instantiated): `init` is
inlined at its prove leg; the source fact (`hsrc`), the owned/
ascending input premises (`hown_bt`/`hsort_bt`), and send-once nodup
(`hnodup`, premise-free modulo the variant) are ghost `have`s composed
directly from `hp1a_out_inv` — the providers leg's 40-line zip
plumbing collapsed to `hnodup rfl i`. Second pass (user follow-up):
the invariant's dedup clause DROPPED its same-owner/num-ascending
premises — those are facts about the input WIRE, not the loop state;
`p1aInvariant_tick` takes them as ordinary hypotheses, proven at the
leg. Ghost scoping ruling confirmed: obligations and invariants may
cite any ghost bound EARLIER in the chain (replay preserves order),
whole-wire ghosts cover every tick index (so "the next iteration" is
reachable), and forward references are impossible by construction —
nothing upstream of the construct can cite `hp1a_out_inv`. Third pass
(user follow-up): ghost is for SHARING, not for scoping — a fact used
only by one obligation should be a plain term-level `have` inside that
leg, which (being spliced into the chain) sees the local streams AND
every earlier ghost. The owned/ascending input facts ended up as two
`have`s inside the `tick :=` leg (fed by the pre-construct ghosts
`hbc`/`hzip_at`, which stay ghost because the stable/providers legs
also consume them); only multi-consumer facts remain in the ghost
layer. The `tick` loop-body lemma itself
does NOT shrink by ghosting: it quantifies over the abstract
mid-induction loop state, which no wire names — its ~75-line FIRE-case
freshness argument is the irreducible B1 mathematics (in Verus, SMT
would eat exactly this block), so it stays a named preamble theorem.
Two tactic notes: `split at h`
cases a `match` on an OPAQUE scrutinee (the construct's register
expression never needs respelling), and `rw [hvar]`'s closing rfl is
reducible-only — projection-valued goals (`.guarded.sendOnce = true`)
need an explicit `rfl` after. `LEDec.fuelP1a` is REVERTED everywhere
(census back to nondets 8, scheds 4, fuels 3) — a register is not a
decision. The
bridge lemma `p1aInvariant_tick'` (55 lines of take-algebra) and
`tick_read_link` died with the knot route; `prefix_ext_invariant` /
`zip_map_prefix_left` stay as pure Trace lemmas (they are the
stage-axis bridge the OPEN problem below will want).

**OPEN fix-TODO (the user's "you are sidestepping a bigger problem").**
The leg-11 cliff is dodged, not solved: a chain leg whose value
contains an INLINE closed knot still grinds `runGlueT` (co-rules
present but the normalizer descends into the folded body). Today no
program hits it (ticks are scans; knots are top-level callees), but
**nested `fix`** and **`fix` over tick-typed wires mid-chain** will.
The step obligations for that route already receive the one-step chain
order `relWB` (kept in `mkFixRoute`), and `prefix_ext_invariant` is
proven — what is missing is glue that keeps non-callee knots folded.

**Engine keepers from the detour** (all landed, all general):
`fixPassMode` is a STACK (nested single-source fixes no longer confuse
pass-2 `complete` markers; ghost guards use `.contains true`);
`mkLambdaFVarsOpaque` at the chain/knot/body hoists (outer `let` fvars
become opaque binders — frees no longer leak for fix-after-lets);
`pendingInvs` replay (packaged inductions proven in speculative
elaboration branches reach the persistent env); eager-generation
idempotence guard (`_co_sr₁`-keyed) across nested-fix re-elaboration;
`Inhabited (CutDec α ord)`; and the rr-ex **unused-dec-field pinning**
with its correction: only the TOP-LEVEL projection's absence from the
module's constant census is sound evidence of non-use — a used
sub-record flows whole into callees whose own leaf projections the
census cannot see (LE's `hb.sample` was falsely pinned to `default`;
the fix pins by `path.take 1`).

**Elaboration lessons (macro-writing).** (1) Obligation splices MUST
be `have h : <fully-spelled type> := $userTerm` — applied-lambda
partial application loses expected-type propagation and anonymous
constructors die. (2) A standalone `have`'s `∀ i` binder needs a type
anchor when the body doesn't pin it (no spectators): anchor through
the input with an applied lambda `(fun (_ : List _) => P [] seeds)
(inp i)` — `Fin _` holes do NOT survive to the use site, and a `take
0` anchor type-clashes when input ≠ emission type. (3) `optSemicolon`
slots are flat `[semi, term]`; paren-group syntax alternatives need
`atomic(…)`. (4) Invariant ghosts only CHECK under a prove leg — a
def without `ensures` never elaborates them (the toys carry a trivial
`ensures` precisely to force replay). (5) The recurring 2-space
indentation disaster: Lean silently re-scopes `do` blocks (dead code
under a `throwError` guard, a queue push inside the wrong loop) —
verify region indentation mechanically before/after bulk edits.

**Verus rows (updated).** `tick` = `while` + `mut` loop variables with
the invariant clause as THE loop invariant and `prove tick :=` as the
loop-body proof — now literally, since the Lean obligation is the
loop-body step over (emissions-so-far, register). The construct's
no-op-analysis discipline is the Verus port's precondition: a Verus
`tick` macro only needs the same productification + obligation
splicing, with SMT in place of the two `have`s.

### D60 — `Atomic`/`BoundedStream`: tick-located traces named for what they are, one tick's content made interpretation-dependent, the monotonic tick grade retired into faces (Stage 0a of the in-tick redesign)

**Why (the user's thread, in order).** A2 (SP + IndexPayloads onto the
`tick` construct) was paused by ruling after three questions cut
deeper than the migration: (1) *why hand-compressed step functions
(`ipStep`, `spGateStep`, `p1aSendStep`) when Rust's `sliced!` body is
dataflow over tick-bounded collections?* — tick bodies must mirror
Rust line-for-line over "some equivalent of a bounded stream with
generics for ordering/retries, with operators like `map`"; (2) *how
does a `Multiset`-typed body even run in SchedSem?* — answered from the
code (Sched coerces its concrete `List` batch through `Multiset.ofList`
at the body; sound by typing, mirrors Rust's `NoOrder` API; now
SCHED_AUDIT S4 item 14), but judged "technically sound but a bit
scary" since the runtime never erases order; (3) therefore: *bounded
collections should be interpretation-dependent — quotiented at Values,
plain `List` at SchedSem — and the soundness proof forced to reason
about the two.* In-tick `assume_ordering` skipped for now (no Rust
Paxos site needs it; the corner refuses it with `wf := False`).

**The vocabulary (ratified point by point).**
- **`H.Atomic ℓ σ`** — the tick-located trace, one `σ` per tick per
  member; today's `TickSingleton`, renamed (the user: `TickSingleton`
  *is* the trace; "Atomic" because it corresponds to Rust's
  `Tick<ℓ>`-located collections). Values: `Fin → Trace σ`; Sched:
  `Fin → Nat → Trace σ` (the `Nat` is the machine step — "which step
  each tick ran on", the prefix-monotone history; a `Trace`-level
  wrapper was considered and dropped: three interpretations' carriers
  are not member-pointwise — Eager's `EPack` vector+agreement, Couple's
  shared-`wf` structures, Rel's sets of whole wires — so a signature-
  level `Fin → H.Trace σ` cannot be definitional; `Atomic` already IS
  the located trace family).
- **`H.BoundedStream α ord ret`** — one tick's bounded stream content,
  Rust `Stream<α, Tick<ℓ>, Bounded, ord, ret>` *inside* the tick: the
  only interpretation-dependent per-tick element type. Values:
  `PoolCarrier` (the grade quotient); Sched: `List α` at every grade;
  Eager: `PoolCarrier` for now (its data leg still runs the quotient —
  a `List` data leg with `ofList data = den` agreement is the
  executable analogue of the Couple coupling, a later stage); Couple/
  CorrSem/Rel/MonoRel: `PoolCarrier`.
- **`BoundedSingleton σ := σ`, `BoundedOptional σ := Option σ`** —
  reducible abbrevs (not fields: identical in every interpretation).
  Wire types now read as Rust tick types under `Atomic`:
  `H.Atomic prop (BoundedSingleton (Ballot …))`,
  `H.Atomic acc (BoundedOptional (Ballot …))`.
- **`AtomicStream ℓ α ord ret := Atomic ℓ (BoundedStream α ord ret)`** —
  a class **default**, not a new carrier. Values, Sched, Eager, Reader,
  MonoRel inherit it definitionally; Couple, CorrSem, RelSem override
  (a coupling relates two *wires* whose legs have different history
  axes — `CoAtomicStream` pairs `sr : Fin → Nat → Trace (List α)` with
  `rr : Fin → Trace (PoolCarrier …)`; a trace of per-element pairs would
  put the step axis on the wrong leg). The signature states the
  identity; the overrides are the correspondence interpretations and
  say why.
- **No boundedness grade on `Atomic`.** `TickSingleton`'s `SingBound b`
  was a Lean-side extension (Rust tick singletons are always
  `Bounded`; the only `Monotonic` in hydro_lang is the keyed-singleton
  value bound). User ruling (a): drop it; cross-tick ascent is a
  *guarantee*, so it belongs in `ensures` faces — the same move as
  D54's "nondets are what proofs reason about". `fold_across_ticks_
  monotone`/`fold_batches_across_ticks_monotone` lose their `vo`/`hinfl`
  arguments and the `_monotone` suffix (the commutativity argument
  stays — Rust's `commutative = manual_proof!`); `mapMonotone`/
  `forgetBound` are deleted; `snapshot` of a `Monotonic` singleton
  yields a plain `Atomic`. The ascent facts moved to faces:
  `PBCEnsures.mono`, `AP1Ensures.max_mono`, composed into
  `LEEnsures.mono`/`max_mono`, consumed by `LEEnsures.spRequires`
  (`mono := h.mono i …`) and PaxosCore's vote-precedes-promise step
  (`hle'.max_mono j`). Pure content: `foldAcrossTicksTrace_ascending`
  + `Ascending.map` (Trace.lean); `MonoTrace`, `MonoTrace.map`,
  `foldAcrossTicksMonotoneTrace`, `scanMonotone_vals`, `tickVals` are
  GC'd. Program text got *closer* to Rust: four `H.forgetBound` calls
  and one `mapMonotone` (PBallotCalc's `p_ballot` is now a plain
  `mapTick`) disappeared.

**Engine.** `HRelC` gains `atomicRel` (for `tickSingRel`) and
`boundedRel`; `CarrierFam` gains `.ticked`/`.bounded`; the mono/causal/
eager relation instances define `boundedRel` (`PoolLe`/`Eq`/`Eq`);
`agreeRelOf .bounded := Eq`. Law bundles lose the two deleted ops' rows;
`co_*`/`eag_*`/`causal_*`/`vmono_*` for the renamed folds are renamed;
`TickRel`/`TickSingRel`/`CoTickSing` lose their `b` index. The
`hydro_rel_laws` lift and every generator took the new type fields and
the defaulted field without change.

**Invariant held.** Every `Values` denotation is byte-identical (the
carriers unfold to exactly what they were); zero program-text changes
beyond the wire-type spellings, the four `forgetBound` removals and the
`mapMonotone → mapTick`; censuses unchanged; headlines byte-identical.
Full build 809 jobs green, zero sorries, falsify/explore/v2paxos/
v2twopc 4/4.

**Stage 0b — the in-tick operators (landed, gate green).** The user's
question "don't we do something similar outside tick bodies for
unordered unbounded streams?" caught a design error mid-stage: outside
ticks every value is a two-leg carrier, so Couple's `fold` applies the
closure on each leg; my first cut made `BoundedSingleton σ := σ` a
plain abbrev, so `bcount` returned a *bare* `Nat` — one leg — and the
denotation's leg was unrecoverable (I had wrongly concluded the former
needed a vocabulary-generic body). Correction (the user had anticipated
it — "similarly there will be bounded singleton"): **`BoundedSingleton`
and `BoundedOptional` are interpretation-dependent families too**
(`σ`/`Option σ` in the running interpretations; `CoBSing σ = {sr rr : σ,
wf, cpl : wf → sr = rr}` at the corner), with Rust's singleton API as
fields (`bsPure`, `bsMap`, `bsZip`, `boMap`, `boUnwrapOr`, `boFilter`,
`boIsSome` — paxos.rs:799 `num_payloads.zip(base_slot).map(|(n,b)|
b+n)` IS `bsZip` + `bsMap`; Rust does no bare arithmetic on tick
singletons either), and `bcount`/`bfold`/`bfirst`/`bcrossSingleton`/
`bfilterIf` typed over them. Every value inside a tick body now keeps
two legs at the corner, so the former will be an ordinary closure
field and Couple projects `sr`/`rr` from one run of the body (0c).
Wires across ticks stay `Atomic ℓ σ` over the plain type (a wire of
corner-pairs would put the step axis on the wrong leg, as for
`AtomicStream`) — the `BoundedSingleton` decoration on wire types was
reverted. Twenty signature fields over the bounded families, Rust
names with a `b` prefix,
grade constraints = Rust's trait bounds: `bmap`, `bfilterMap`,
`bflatMapOrdered` (TotalOrder), `bflatMapUnordered` (closure yields
bounded `NoOrder` streams — the order history is the runtime's; no
decision), `bofList` (`Vec::into_iter` into `NoOrder`), `bcount`
(ExactlyOnce), `bfold` (the graded `FoldOk` obligation), `benumerate`
(TotalOrder), `bfirst` (TotalOrder), `bcrossSingleton`, `bchain`,
`bfilterIf`, `emitBounded` (`yield_atomic`: identity staging; a real
conversion only at the correspondence interpretations). User
correction folded in: `emitMultisetBatches` keeps its `EmitDec` — a
computed unordered *value* (Rust: iterating a hash collection) still
needs the machine to choose a linearization; what has no new freedom
is `flat_map_unordered` over bounded streams whose elements already
carry their order history. In-tick `assume_ordering` skipped (ruling).
- Implementations: Values = the quotient's own ops (`mapPool`,
  `filterMapPool`, new `poolCount`/`poolChain`/`poolFlatMapUnordered`
  (`Multiset.bind`)/`poolFilterIf`/`listEnumerate` in Grades.lean);
  Sched = plain `List` ops at every grade; Eager/Reader/Rel/MonoRel
  delegate to Values (one tick's content is a single value, not a
  pair of runs); CorrSem = `List α` (the simulation's set leg holds
  list-valued witnesses; `emitBounded` coerces them by `poolOfList`).
- **The coupling as proof** (Couple.lean): `CoBounded α ord ret =
  {sr : List α, rr : PoolCarrier α ord ret, wf : Prop, cpl : wf →
  CoupledBatch ord ret sr rr}` and `CoBSing σ` — the same `wf`-guarded
  shape as every wire carrier; `CoupledBatch` is equality (up to `↑`)
  at ExactlyOnce, `True` at the retry grades (as `BatchTraceLe`).
  Every in-tick op is legwise and TRANSPORTS the coupling into its
  result's `cpl`: streams via `CoupledBatch.map/filterMap/chain/
  filterIf/flatMapUnordered` (`Multiset.map_coe`, `filterMap_coe`,
  `coe_add`, `coe_bind`+`bind_congr`); singleton results via
  `CoupledBatch.count` (`coe_card`), `CoupledBatch.fold` (at
  ExactlyOnce: `Multiset.foldl` on `↑`; `bfold`'s result `wf` adds
  `ret = .exactlyOnce` — the retry grades have no per-tick coupling and
  no tick stream is born at them), `bfirst` by the TotalOrder equality
  — exactly the proofs the grade constraints make possible (`count` at
  AtLeastOnce would be false). All co-rules are unconditional `rfl`
  projections; the conditionality lives inside each pair's `cpl`.
  `emitBounded` at Couple: `wf` adds "every realized tick's pair is
  wf" and `cpl` assembles `BatchTraceLe` from the pairs' couplings.
- Engine: `HRelC.bsingRel`/`boptRel` (Eq everywhere) and
  `HRelC.atomicBoundedRel` (the one nested-carrier shape:
  `Atomic ℓ (BoundedStream …)` has an interpretation-dependent element
  type, so `atomicRel`'s shared-`σ` signature cannot relate it; the
  lift keys `Atomic`-of-`BoundedStream` binders to it); the lift
  relates **plain result types by equality** (`count`/`fold`/`first`)
  and gives **plain-domain carrier-valued closures** a pointwise
  premise (`bflatMapUnordered`'s); mono/causal/eager `boundedRel :=
  Eq` (within a tick there is nothing to ascend — the mono relation
  lives on the tick axis), law tactics gain `rfl`/`assumption`/`funext`
  closers; co-rules/eager/causal projections for all thirteen ops in
  the registries. `HydroTickCheck.lean` pins both representations
  (`#guard`/`rfl` at Values and Sched) and the corner's conditional
  agreements on a coupled pair. Full build 809 jobs green, zero
  sorries, 4/4 exes.

**Open (Stage 0c, this thread).** One grade-polymorphic scan former
replacing both `scan_batches_*_across_ticks` — an ORDINARY closure
field (body typed over the bounded families; at Couple the former
wraps each tick's two legs into pairs, runs the body once, and
projects `sr`/`rr`, with `cpl` assembled from the pairs' own
`wf`-guarded couplings — no rank-2 body needed, since no value in a
slice is bare); the `tick` construct re-elaborated: multi-input blocks
dispatched by input kind, the body a let-chain of in-tick ops over
`BoundedSingleton` registers, hoisted as a named step for the
generator walks. Then IP and the B2 gate (and LE's `p1aSendStep`)
migrate to Rust-line-for-line bodies, and A2's monolith dissolution
resumes on the new bodies. Follow-ups noted: the Eager `List` data
leg; the member-pointwise carrier refactor that would make `AtomicStream
:= Fin → H.Trace _` literal; in-tick `assume_ordering` with its
`batchOrdSelDerive` + `cpl`; a data-lift `bofMultiset` with a per-tick
linearization claim validated at the former, if a Rust site ever
iterates a hash collection inside a slice.

**Stage 0c landed (this thread): the shaped tick former `tick_scan`
and its engine support.** The former is ONE signature field indexed by
a *shape* — `TickShape := sing τ | stream α inst ord ret | pair a b`,
with `AtomicOf Tk TS ℓ sh` (the wires of a shape) and `BoundedOf BSing
BStr sh` (this tick's values of a shape), both `@[reducible]` so the
simp unifier sees `AtomicOf … (pair a b)` as the product it is:

    tick_scan (ins outs : TickShape) : AtomicOf Atomic AtomicStream ℓ ins →
      (Fin (mem ℓ) → BoundedSingleton σ → BoundedOf BoundedSingleton BoundedStream ins →
        BoundedSingleton σ × BoundedOf BoundedSingleton BoundedStream outs) →
      σ → AtomicOf Atomic AtomicStream ℓ outs

(zip at the TYPE level — a wire-level `zipTick` of pair-valued wires
would break the corner's literal-prefix `cpl`). Implementations, all by
shape recursion: Values/Sched `slice` (zip of the leaves' traces)/
`unslice` (map `.1`/`.2` back out) around `scanAcrossTicksTrace`
(`ValuesTick.scan`, `SchedTick.scan`, the machine adding the step
index); Reader through `D → Values`; Eager as `pack` of the data-leg
fold and the denotation-leg fold with `fn_eq` agreement (`EagerTick`);
Rel the image of the leaf-sets' product; MonoRel two Values legs with
`ValuesTick.scan_le` (prefix in, prefix out); CorrSem the machine
former with the degenerate witness family {coerced machine run as of
step k} (`CorrTick`, `poolOfListR` = `poolOfList` at every retry
grade); **Couple** (`CoTick`): the body runs ONCE as a Couple-typed
function; each leg of the output is the body on a leg-*embedded* input
(`embedSR`/`embedRR`: the other leg dummy, `wf := False`) so the leg
projections are definitional, and the coupling (`CoTick.scan_cpl`) is
proven from three `wf` conjuncts the walk discharges — **W1** the `sr`
legs of the body depend only on the inputs' `sr` legs (the body on
`embedSR` agrees componentwise with the body on the coupled embedding
`embed2 inp (coerce inp)`), **W2** likewise for `rr`, **W3** the
coupled run's outputs are `wf` — plus the shape-level `allEO` fact
(every stream leaf ExactlyOnce: the retry grades carry no per-tick
coupling, as for `BatchTraceLe`). Pointwise `body_comm` (W1+W2+W3 ⟹
the machine body coerced IS the denotation body on the coerced input)
lifts to `scan_comm` by induction on the tick trace, and
`slice_le`/`unslice_le` carry the leafwise wire couplings through the
tuple traces.

Engine: `HRelC.atomicOfRel`/`boundedOfRel` (reducible, leafwise) and a
general structural type-lift `relOfTypes` in `hydro_rel_laws`
(carrier heads → their field; `AtomicOf`/`BoundedOf` → the shaped
lifts; products componentwise; non-dependent functions pointwise /
by relatedness-preservation; `H`-free types by equality) — the
generated `HRel.law_tick_scan` relates the two bodies on related
registers and related input tuples. Law rows: mono via `scan_le`,
causal via `TAgreeOf` + `causal_tick_scan` (the step-`k` output
depends on the step-`k` inputs only), eager via `den_pack`; all three
bundles close by `g = g'` from the Eq-valued in-tick relations. Walkers:
`projShapeOfSubject`/`shapeOfCarrierType` (WfTactics) read a wire's
shape off a `Prod` projection's type; `hydro_param_proj` and the
causal step apply `atomicOfRel_fst/snd` / `TAgreeOf_fst/snd` with the
shapes pinned (`applyWithShapes`), guarded so projections of literal
tuples reduce by `dsimp` instead (the guard is what stopped an
And-split/re-project loop); `hydro_param_leaf` accepts any `HRelC`
family (a shaped leaf goal unfolds to its leaf family under
reducible `apply`); `genWf` gains a `co_simp` alternative (W1/W2 are
`sr`/`rr` facts). Validated end to end on `toy_shape`
(`HydroTickCheck.lean`: a singleton wire and an ordered tick stream in,
register trace and bumped batch out): `co_sr/co_rr/co_wf/causal/mono/
param` all generate (3.5 s), eager agreement rides `_param` at `eagC`
(the body's in-tick ops are the denotation's by definition — the
`eager_transfer` simp route is NOT the path for shaped formers: the
Eager-instantiated body mentions `(Eager).bmap …`, only `rfl`-equal to
the Values body).

Canon learned (three rounds of a single lesson): **the simp unifier
works at `instances` transparency and cannot see through a
semireducible interpretation instance** — `(CoupleSem …).BoundedSingleton
σ` and `CoBSing σ` are defeq but NOT unifiable by simp, so a rule whose
binder types mention one presentation never fires on a term carrying
the other. Consequences, each hit and fixed here: (i) the Couple
structural rules are stated through an *instance-typed facade*
`CoTickI` (CoupleProj.lean: `mk`, `srLegs`, `embedSR`, `gS`, `W1`, …,
all definitionally the generic `CoTick` machinery but DECLARED at the
interpretations' own carrier types) — never the generic lemmas; (ii)
a rewrite must not change a term's type *presentation*: `coerce (.sing
τ) v = idV v` with `idV` a reducible identity Sched→Values, and every
`.rr` of a Sched-typed register/leaf produces `idV _`, so both sides of
W2 meet syntactically; (iii) the W-conditions are stated
COMPONENTWISE (`(gS …).1 = (gC …).1.sr ∧ (gS …).2 = projSR …`) — a
`Prod.mk` packaging gets its implicit type args from one presentation
and the `Eq` from the other, and `eq_self` then fails on identical-
looking sides; (iv) `CoupledBatch` is reducible and simp keys the
goal on its reduct, so the W3 stream leaf closes by
`embed2_stream_coerce_wf` (the specific `(embed2 … m (coerce … m)).wf
= True`), not by a lemma on `CoupledBatch`; (v) shape literals must be
constructors in program text — an `abbrev ShapeNS := .pair …` blocks
every shape-keyed rule (the `tick` construct will always emit
literals); (vi) `Eager`'s `pack` has a DEPENDENT proof argument, so
its projections rewrite PRE-order (`↓eag_pack_pair_fst` …) before the
walk rewrites inside the legs, and `EagerTick.pack/fn/den` are typed
over `(Values L mem).Atomic` so whnf-unfolding (simp's own `proj`
reduction uses default-transparency whnf!) and the lemma route produce
the same presentation. The general rule for any future shaped/dependent
engine piece: state every rewrite at the instance's carrier types,
keep presentations invariant across a rewrite, and never rely on
`eq_self` across a presentation boundary.

**0c-ii landed: the `tick` construct emits `tick_scan`.** Surface:
`tick (state r : σ := seed)+ (input x := wire)+ (invariant …)? :=
<in-tick let-chain> rebind (…) emit (a := …)? yield (b := …)? (prove
…)?; cont`. Registers and inputs are `H.BoundedSingleton`/
`H.BoundedStream` values inside the body (Rust's sliced body over this
tick's values; arithmetic goes through `bsZip`/`bsMap` as Rust's
`.zip().map()`); each input's kind is read off its wire TYPE (`Atomic`
↦ `.sing`, `AtomicStream` ↦ `.stream`) — the only type-directed step;
the output shape's structure comes from the `emit`/`yield` keywords
with leaf parameters as holes unification fills from the body; several
states productify into one `BoundedSingleton (σ₁ × σ₂)` register opened
by `bsMap … Prod.fst/snd` and rebound by `bsZip`; several inputs/
outputs are right-nested pairs destructured by PROJECTIONS (not
`match`, so both sides of every naming normalize identically); the
output wires are `let`-bound with their LEAF carrier types ascribed
(`H.Atomic _ _`/`H.AtomicStream _ _ _ _` — a `AtomicOf … (.sing _)`-typed
leg is not a carrier to the generators). The `invariant` clause (v1:
single input, single output) keeps its loop-invariant normal form via
`scanAcrossTicks_invariant`; its output binder must be typed `(out :
List τ)` — the obligations spell the step's components `@Prod.fst/snd σ
τ`, because at `Values` they are `BoundedSingleton _`-typed, only
definitionally the plain types the user's simp lemmas key on.
HydroTickCheck's three toys and LeaderElection's B1 gate are on the new
construct (LE: `p1aSendStep` as the step closure of a `bsMap` over the
(register, input) pair — B1 has no Rust counterpart; `p1aInvariant_tick`
unchanged); a fourth toy exercises two inputs of both kinds with an
`emit` and a `yield`. LeaderElection builds in 6m41s (unchanged); full
gate green.

Engine fixes this stage, each a canon entry: (vii) **a rewrite rule
keyed on a bare op application is a hazard** — `co_tick_scan :
Couple.tick_scan … = CoTickI.mk …` fired (as a rfl-dsimp) inside
callee arguments, and the `mk`'s dependent proof argument then kept
the ORIGINAL legs in its type while the walk rewrote the explicit legs,
so no `mk` projection rule could unify afterwards (the symptom: the
`co_sr` fallback `exact rfl` evaluating the whole Sched program —
`tickSteps`/`snapDerive`/`Nat.rec` by the hundred-thousand in
`set_option diagnostics`). Replaced by **output-PATH rules**
`co_tick_scan_<path>_<leaf>_{sr,rr,wf}` (paths `[]`,`1`,`2`,`21`,`22`,
`221`,`222` × sing/stream: outputs of up to four legs), keyed on the
leg projection of a projected former and with proof-free right-hand
sides; same for Eager (`eag_tick_scan_<path>_<leaf>_den`). (viii) the
causal/param walkers restate a LEAF-shaped former output (`outs =
.sing _`) in shaped form (`TAgreeOf h outs …`/`atomicOfRel i outs …`)
before applying the law, with `g.withContext` — generated proofs run
from an EMPTY ambient context, and `mkAppM` outside the goal's context
fails on the goal's fvars (silently, inside `first`). (ix) two
diagnostics stay, env-gated: `HYDROGEN_SR_DIAG=1` makes `genSr` close
with `co_simp; with_reducible rfl` only (a normal-form mismatch then
surfaces as the residual goal instead of a whole-program whnf — this
is how (vii) was found), `HYDRO_DEF_DBG=1` prints the eager knot
pipeline's progress to stderr (messages logged inside `first` are
rolled back with the failed alternative; `IO.eprintln` is not).

**0c-iii landed: IndexPayloads and SequencePayload on `tick` blocks;
the contract face leaves the definition.**

Programs. `index_payloads` is paxos.rs:782–804 line for line (`boMap`
`boUnwrapOr` `benumerate` `bcrossSingleton` `bmap` `bcount`, rebind,
`yield`); its output is now the tick stream Rust has
(`H.AtomicStream prop (Nat × P) .totalOrder .exactlyOnce` — at `Values`
the same trace as before, so `IPEnsures` is unchanged; `ipStep` stays
as the pure spec, `ipBody_eq` is the pointwise bridge).
`sequence_payload`: the B2 gate is a stateful `tick` (register
`recommittedAt`, `bcount`/`bsZip`/`bsMap` for the fire condition,
`bfilterIf view fire` as the gated view — a STREAM filter, so the
`emitMultisetBatches` site and its `rcEmit` linearization are gone:
SPSched 5 → 4 scheds, PaxosCore 9 → 8, census-enforced; `spGateStep`
stays as the pure step, `spGateBody_eq` bridges); `payload_batch` and
`payloads_to_send` are stateless ticks (`bfilterIf`, and Rust's
`cross_singleton · map · chain · filter_if` with ONE new in-tick op,
`bweakenOrder` — Rust's `chain` types at the MINIMUM order
(`TotalOrder ⊓ NoOrder = NoOrder`) and our `bchain` is same-grade, so
the coercion Rust's trait system inserts is written; consequence:
`payloads_to_send` is a `NoOrder` tick stream exactly as in Rust and
`join_responses` consumes it directly, `hsent` states the sent face as
`spSentTrace … |>.map Multiset.ofList`, the `SequencePayloadLemmas`
monolith is untouched). `BoundedOptional` is a reducible alias for
`BoundedSingleton (Option σ)` (a `Atomic (Option Nat)` input must be
usable with the `bo*` ops inside a tick; still interpretation-
dependent through `BoundedSingleton`). Reading a tick at the denotation
in ghost proofs goes through `ValuesTick.{tick_scan_sing,
tick_scan_stream, slice_*, values_*}` rewrite rules (one constructor
step each) — a `show` of the zipped form made the elaborator reduce
the whole shaped former definitionally (`TickShape.rec` ×167k).

The engine finding (the real content of this stage). A callee's type
was `{out // ∀ hv : H = Values, match … args … => Ens}`, so when a
callee naming rewrote `(↑(join_responses C … r m)).sr` to
`join_responses S … r.sr m.sr`, the arguments landed in positions
simp's auto-congruence marks FIXED (the result type depends on them):
only `rfl`-lemmas (the op projections) fire there, the propositional
callee namings never do. Every such stuck leg — `recommit.2.sr` under
`index_payloads`, `collect_quorum.1.sr` under `join_responses` — had
been closed silently by `co_transfer`'s `exact rfl`, a whole-program
whnf at both interpretations; the new SP (three ticks, deeper nesting)
no longer fit in the budget, which is how the mechanism surfaced.
Tried and rejected on the way: (a) a global `@[congr]` lemma per
module (`(M xs).val` congruence) — never fires, because the hypothesis
binder is instance-typed (`S.Atomic …`) and a machine leg `x.sr` is
raw-typed (`Fin _ → ℕ → Trace _`); Lean's metavariable-assignment
type check runs at `implicit` transparency and cannot see through
`SchedSem`; (b) `backward.isDefEq.respectTransparency.types false` in
`co_simp` — makes those checks pass but each one then whnf-unfolds the
WHOLE interpretation literal (`Values` ×144k): the instance records
inline every implementation, so a default-transparency type check is
a program-sized term copy; (c) a `rw`/`rewrite` loop with the callee
rules — `rw`'s trailing reducible `rfl` explores the mismatched pair
after every step, and with the ∃-witness holes in the goal `kabstract`
unifies every candidate against them (both: hundreds of k heartbeats).
Landed instead: (1) the knot glue's engine is now THE naming engine
(`HydroGenKnot.genNaming`: one module delta → simp → keyed residual
pass → both sides MEET syntactically or by reducible defeq; a
default-transparency `rfl` only on a residual mismatch, loud under
`HYDROGEN_SR_DIAG=1` with the first divergent node); fixed in the glue
chain on the way: a purely definitional simp pass (all op rules are
`rfl`) returned no proof and its RESULT was dropped, the residual pass
re-tried the whole op set (every candidate under a key is one
unification), the meet diff reported bare heads; (2) `genRrEx` runs
the engine on the corner side only (`co_residuals`) and never unifies
against the witness holes; (3) **the contract face leaves the
definition**: `hydro def M … : τ ensures out => P := …` now elaborates
the pair under `M._spec` and derives `M : ∀ xs, τ` (the pair's value,
`Subtype.mk` stripped — a plain, non-dependent function) and
`M.ensures : ∀ xs[H := Values], P xs (M (Values L mem) xs)` (stated AT
the denotation, no `hv`/`match`; proof `(M._spec …).property rfl`).
Call sites lose a line (`let x := M H …; ghost have hx := M.ensures
…` — no `.val`, no `.property rfl`); a callee application is an
ordinary function application, so plain simp reaches every argument
and the residual pass only ever has leaves to do; the `fix`
construct's face auto-premise reads `M.ensures` off the chain `let`.
Hoisted knots keep their `M.<wire>` names (`moduleOfSpec`). `two_pc`
(a plain `def … ensures`, not a `hydro def`) keeps its Subtype.

Numbers. SequencePayload 35 s (was 28 s, with the three ticks) at
`maxHeartbeats 250000` (was 1000000); PaxosCore 72 s (was ~50 s) at
3200000 (was 12800000); LeaderElection 376 s (was 401 s); full gate
green (zero sorries, axioms standard, `falsify`/`explore`/`v2paxos`/
`v2twopc` OK, censuses build-enforced). The `BoundedOf`/`AtomicOf`
unfolds joined `co_simp`/`coSimpBase` (a shaped former's generic type
presentation vs the body's leaf one — the `Prod.mk` type arguments of
a `gS` reduct), and `gS`/`gR` are typed at instance carriers. Canon
(x): a module's result type must not depend on its arguments —
dependent results make callee arguments invisible to simp, and every
closer that "handles" that is a whole-program whnf in disguise.

Next: stage C (retire `scan_across_ticks`/`scan_batches_*_across_
ticks`, final op-table diff pinged first), PaxosCore's 72 s, and A2's
monolith dissolution on the new bodies.

**Stage C, steps E(1)–E(2) landed: shaped registers, the keyed in-tick
vocabulary, and `collect_quorum`/`collect_quorum_with_response`/
`join_responses` as quorum.rs/request_response.rs line for line.**
(1) **Registers are shapes.** `tick_scan` takes `(sts ins outs :
TickShape)`; the register is `BoundedOf … sts`, the seed `SeedOf sts`
(sing ↦ the value, stream ↦ `Unit`, pair ↦ product); every
interpretation's `seed` (Values `PoolBot`, Sched `[]`) and the corner's
`gS/gR/gC/W1/W2/W3/scanWf` thread the shape (the `regS/regR/regC`
facade and the `idV`-W2 form go away — a register is embedded exactly
like an input). Surface: `(state s : H.BoundedStream α ord ret)` is a
persisted stream (Rust `use::state_null`), seeded empty, no `:=`; the
macro reads the kind off the type and builds a right-nested pair of
states with `.1/.2` projections (the `bsMap`/`bsZip` productification
is gone). The IP/SP/LE ghost proofs needed no change.
(2) **In-tick ops**: `bfilter`, `bkeyedFold` (`into_keyed().fold().
entries()` collapsed onto a `NoOrder` entries stream — a keyed carrier
is deferred by ruling), `bkeys`, `bjoin`, `bantiJoin`, `bfilterNotIn`;
pure forms in `Grades.lean` (`keyedFoldList`/`keyedFoldMultiset` with
`coe_keyedFoldList`, `listJoin`/`multisetJoin`, `poolFilter`,
`poolAntiJoin`, `poolFilterNotIn`), the corner's per-op couplings
(`CoupledBatch.filter/keyedFold/join/antiJoin/filterNotIn`,
`coe_eq_weaken`), rules in every list, laws by the law tactics.
(3) **A program-level `if` over static parameters** (quorum.rs's
construction-time `if max == min {…} else {…}`) is one body term:
`co_ite_*` push the legs through `ite` — spelled AT the leg's carrier
type (the raw `apply_ite` codomain is not the `Eq`'s type at reducible
transparency, so `eq_self_iff_true` never fired on syntactically equal
sides), `fst_ite/snd_ite` for projected tuples, `ite_self` in the wf
normalizer, and `split` as a late alternative in the causal/param
walkers (both sides branch on the same condition). Toys `toy_ite`,
`toy_keyed`, `toy_tick5` (stream state) run every generator axiom-free.
(4) **The bridges**: `cqBody_eq`/`cqwrBody_eq`/`jrBody_eq` — one tick
of the block at `Values` IS `cqTick`/`cqwrTick`/`jrTick` (keyed-counter
characterization `keyedFold_cqCount`, `cq_reached_eq`,
`cq_keys_filter_eq`, the anti-join/filter-not-in facts);
`scanAcrossTicksTrace_map_state`/`_map_both` (a step machine simulated
through a state map; and through a state+emission map) carry the
block's tuple register to the proofs' record; every existing run fact
is reused unchanged. `collect_quorum_values_eq` exports the identity.
(5) **Emission is the program's own.** `emitMultisetBatches`'s
`EmitDec`/`emitLin` claim lists leave the three modules (census:
`collect_quorum` 1→0 scheds, `collect_quorum_with_response` 1→0,
`join_responses` 1→0, `p_p1b` 1→0, `leader_election` 4→3,
`sequence_payload` 4→2, `paxos_core` 8→5, `two_pc` 6→4 — the last an
implied consequence of the ruled retirement, flagged; **ratified by the
user post-squash**: the two dropped fields were the `EmitDec`s inherited
from `two_pc`'s two `collect_quorum` calls, not two_pc's own; the four
that remain are exactly its four `broadcast_closed` transports
prepare/vote/commit/ack). The liveness
headlines (`cq_live`, `cq_live_chain`) lose their WF(emit) supply and
`OverlapLegal` premises — D48's "emission-supply finding" is resolved
by construction: the machine's per-tick emission LISTS are the register
machine's multisets (`cqTickS`, the block on plain lists;
`cqTickS_coe`, the corner's per-operator couplings chained;
`cqMachineTraceS_coe`). Conclusions unchanged; premises strictly
weaker (`FairTicks` alone).
(6) `co_simp`/`co_wf_simp` rule lists are `Array Name`s
(`coSimpRules`/`coWfSimpRules`) spliced by the macros — the literal
quotation had hit `maxRecDepth`. Timings unchanged (SP 37 s, PaxosCore
74 s, LE 385 s; Quorum 7.6 s). Full gate green.

**E(3) AcceptorP2 landed (child thread).** `acceptor_p2` is now three
tick blocks mirroring paxos.rs:819–899 line for line: the
qualification (`cross_singleton(a_max_ballot).filter_map(…)`,
stateless), the log (`across_ticks(reduce_watermark)` as a persisted
entry-stream state chained per tick, `entries().fold(insert)` as
`bfold ((s + {e}))`, the checkpoint zip and the canonical view in the
emission), and the acks (`cross_singleton.map`, stateless). NO new op:
Rust's `reduce_watermark` combiner is non-commutative exactly on
equal-ballot ties (paxos.rs:862's own `TODO: need assume`), so a
faithful `bkeyedReduceWatermark` would carry a program-specific
premise in its signature; the module keeps its documented honest
refinement — the register is the RAW accumulated entry multiset
(commutativity paid unconditionally) and the per-slot keyed reduce +
watermark GC collapse onto `logView` at the emission (header
unchanged). Bridges: `hquals` (the block IS `ap2QualBatches`),
`hface` via the generic `scan_chain_view` (an accumulate-and-view scan
IS the checkpoint-zip of the running fold, mapped), `hacks`/`hdecomp`
(the AcceptorP1 pattern). `fold_batches_across_ticks`,
`filterMapBatchesWith`, `mapBatchesWith`, `mapTick`/`zipTick` uses all
leave the module; contract (`AP2Ensures`), census (2 nondets, 1
sched), and every downstream proof unchanged. Full gate green.

Remaining stage C: E(3) Recommit/AcceptorP1/AcceptorP2/PBallotCalc
(open: `across_ticks` as the Rust-faithful persisted-stream state +
`bmax`, vs the ruled fold-register wrapper; AcceptorP2's keyed
`reduce_watermark`), E(4) delete the A-list ops from `HydroSem`, E(5)
the renames.

### D61 — stage C complete: every `sliced!` body is its Rust dataflow; the collapsed tick-level operator table is gone

**E(3) (rulings: persisted-stream `across_ticks` + `bmax`; the
`reduce_watermark` deviation approved).** `Ballot` carries Rust's
`derive(Ord)` as a `LinearOrder` lifted along the existing `key`
embedding with `Ballot.max`/`Ballot.min` as ITS `max`/`min` — so the
generic in-tick `.max()` (`bmax`, `maxStep`) IS `Ballot.maxFold` by
`rfl`. `acceptor_p1` = a persisted stream state chained and `bmax`ed
(paxos.rs:498–502, `across_ticks(|s| s.max()).into_singleton()`) plus
a stateless `cross_singleton∘cross_singleton∘map` block; bridge
`scan_chain_max` (`Multiset.foldl_add` telescopes the pool) reconnects
to `aMaxStep`'s fold trace. `acceptor_p2` = qualification block,
persisted entry-stream accumulation (`across_ticks`), `entries().fold`
as `bfold (s + {e})`, checkpoint zip + the canonical `logView` at the
emission, acks block; bridge `scan_chain_view`. NO
`bkeyedReduceWatermark`: Rust's combiner is non-commutative on
equal-ballot ties (paxos.rs:862's own `TODO: need assume`), so a
faithful collapsed op would need a program-specific premise or a silent
tie-break; the module's long-documented `logView` refinement stands
(the Rust lines quoted at the collapse point). `recommit_after_leader_
election` = ONE stateless block over (batch, ballot): `filter_map∘max`
for the checkpoint, `map∘flatten_unordered` + pooled `bfold` for the
entries (the keyed champ fold's `manual_proof!(TODO)` collapses onto
`logView` + `rcCount` of the pooled entries — same honesty), then the
`cross_singleton`/`filter_map`/`keys().max()`/holes/`chain` lines as
singleton maps; bridges `rc_body_ckpt`/`rc_body_pool`/`rc_body_fst_eq`
/`rc_body_snd_eq` land on the existing `recommit_chain_eq`/
`rcMaxSlot_chain_eq` fusions. `p_ballot_calc` = the `sliced!` block
verbatim (register `p_ballot_num`, the jump as `pbcJump me` — `me` is
the body's `CLUSTER_SELF_ID`), bridge `scanAcrossTicksTrace_fold_emit`
(a register block emitting a function of the NEW register and the input
is the fold's trace zipped with the inputs) + `map_fst/snd_zip_map_pair`.
Every contract (`AP1Ensures`/`AP2Ensures`/`RCEnsures`/`PBCEnsures`),
every downstream proof and every census unchanged. Side effect: the
knots got cheaper — LeaderElection 385 s → 273 s, PaxosCore 74 s →
30 s.

**E(4).** Deleted from `HydroSem` and every layer (interpretations,
law rows, `SchedCausal` lemmas, `WfTactics`/`SchedCausal` dispatch,
`CoupleProj`/`EagerProj` rules + lists, `HydroGenKnot`'s list, the
`DecFam.emit`/`emitRel`/`.sched` classification of `EmitDec`):
`mapBatchWith`, `mapBatch`, `mapBatchesWith`, `filterMapBatchesWith`,
`mapBatchesUnordered`, `scan_batches_across_ticks`,
`fold_batches_across_ticks`, `scan_batches_unordered_across_ticks`,
`scan_batches_unordered`, `scan_batches_unordered₂`,
`scan_across_ticks`, `fold_across_ticks`, `emitMultisetBatches`,
`emitBounded`, `EmitDec`, `emitLin` (+ its prefix lemmas). The
HydroGen toys (`toy_relay`/`toy_step`/`toy_loop`, `toy_loop_inv`) lost
their emission decision; `toy_relay`'s gate is now a real `filter_if`
in a tick block (its `ToyEnsures` proof walks the stateless scan).
Nothing else moved: no program had a use left.

**E(5).** `flattenOrdered`/`flattenUnordered` (were `emitBatches`/
`emitBatchesUnordered`) and `defer_tick` (was `defer`) — Rust's names;
`mapTick`/`zipTick` keep their names (Rust's `Singleton::map/zip`
collide with the stream fields). The ruled `across_ticks` wrapper is
superseded by ruling 1 (a persisted stream state IS Rust's
`across_ticks`). LIVENESS.md's `EmitDec` fork is closed as resolved by
construction; README/CORRESPONDENCE/Trace tables updated.

The tick-level op table of `HydroSem` is now: `batch`/`batch_ordered`/
`assume_ordering_batch` (entering the tick), `sample_every`/
`timeout_snapshot`/`source_interval_batch` (timing), `mapTick`/
`zipTick`/`defer_tick` (Rust's `Singleton` methods), `allTicks`/
`flattenOrdered`/`flattenUnordered` (leaving the tick), `fix_stream`/
`fix_tick` (knots) and `tick_scan` (the `tick` block) — every stateful
`sliced!` is a `tick` block in the in-tick vocabulary. Full gate green
(809 jobs, zero sorries, axioms standard, `falsify`/`explore`/
`v2paxos`/`v2twopc` OK; censuses as E(2) recorded).


### D62 — A2 on the in-tick foundation: the sequencing monoliths dissolve into two loop invariants

**Where the program already stood.** D60's 0c-iii had put `index_payloads`
and `sequence_payload` on `tick` blocks line for line (fidelity table,
re-audited against the source this round):

| Lean site | Rust | class |
|---|---|---|
| `IndexPayloads.lean` `tick (state next_slot : Nat := 0) (input updated_max_slot := p_max_slot) (input payload_batch := c_to_proposers)` → `boMap` · `boUnwrapOr` · `benumerate`∘`bcrossSingleton`∘`bmap` · `bcount` · rebind `bsZip`/`bsMap` · `yield` | paxos.rs:782–804 `sliced! { let mut next_slot = use::state(…singleton(0)); …map(s+1); unwrap_or(next_slot); enumerate().cross_singleton(base).map(…); count(); next_slot = num.zip(base).map(+); yield_atomic }` | `use::state` — 1:1 (the two `use::atomic` entries are tick-entry devices, D58 ruling) |
| `SequencePayload.lean` B2 gate `tick (state recommittedAt := none) (input view/bl/ledLast) … yield (rcGated := bfilterIf view fire)` | no Rust line (FINDINGS B2; the comment says so) | the natural `use::state` spelling |
| SP `payload_batch` stateless tick `bfilterIf c_batch p_is_leader` | paxos.rs:711–723 `.batch(tick, nondet_commit).filter_if(p_is_leader)` | tick-level operators |
| SP `payloads_to_send` stateless tick `bfilterIf (bchain (bweakenOrder (bmap (bcrossSingleton …) …)) p_log_to_recommit) p_is_leader` | paxos.rs:726–734 `.cross_singleton(p_ballot).map(…).chain(p_log_to_recommit).filter_if(p_is_leader).all_ticks_atomic()` | tick-level operators (`bweakenOrder` = the MinOrder coercion Rust's traits insert) |

So A2's substance was the proof layer: `SequencePayloadLemmas` carried
three private suffix-form inductions over a FUSED two-register scan
(`spSent_nodup_go` 495 ln, `spSent_open_input_go` 437, `spSent_open_go`
111) plus the fusion scaffolding that existed only to feed them
(`spSendStep`, `spSent_fuse_go`, `spSentTrace_eq_scan`, ~80 ln). The two
registers live in different modules (`next_slot` in `index_payloads`,
`recommittedAt` in the gate), so Rust fidelity forbids the fused block the
monoliths inducted over; the dissolution puts each register's induction
on its own program block and makes the faces read ONE tick.

**Engine: `tick invariant` v1 → v2** (`HydroTick.lean`, ~40 ln, pure
syntax, no `HydroSem` change). The clause now accepts any number of
inputs — the invariant and its `tick` obligation see the ZIPPED input
trace `(Trace.zip (x₁ i) (Trace.zip (x₂ i) …))[n]` (`ValuesTick.slice` on
the pair shape, spelled directly), the tuple the body sees — and a
`yield` (stream) single output, whose `(out : List τ)` binder names the
`Values` carrier (`Multiset α` at NoOrder). The ghost's proof reads the
former through the one-step `ValuesTick.*` rewrite rules
(`tick_scan_stream`/`slice_pair`/`seed_sing`…, `first` with the plain
`exact` as fallback) — never a definitional `show` of the zipped form
(the 0c-iii `TickShape.rec` cliff). Toy `toy_tick6` (`HydroTickCheck`:
two inputs of both kinds, a `yield`, a spectator) axiom-clean + `#guard`.
Still single-output, register-only.

**Generic (`Trace.lean`, +60 ln, all with consumers now):**
`scanAcrossTicksTrace_take`, `scanAcrossTicksTrace_getElem` (tick `n`'s
emission = the step on the register after `l.take n`),
`scanAcrossTicksState_take_succ`, `scanAcrossTicks_invariant_take` (the
loop invariant at every prefix — how a non-inductive face cites the
register that produced one tick's emission).

**`index_payloads`' loop invariant** (`IndexPayloads.lean`):
`IPInv out ns ms` — `len` (one emission per tick) · `rebase` (a rebase
`ms[u] = some m` bounds every slot indexed at `v ≥ u` until the next
rebase) · `dom` (no rebase in `(u, v]` ⇒ tick `v`'s slots above tick
`u`'s) · `reg` (slots since the last rebase sit below the register) ·
`reg_rebase` (the last rebase sits below the register); the max-slot
wire is a spectator indexed by `[·]?` so the clauses carry no bound
proofs. `ipInvariant_tick` (one tick; `range'` arithmetic by case on the
rebase) is the block's `prove tick :=` obligation (`ipBody_eq ▸`) and,
through `ipRun_inv` (`scanAcrossTicks_invariant` on the pure `ipStep`
scan), the pure layer's. `IPEnsures` gains `rebase_dominates` /
`slots_dominate`, one-liners off the clause's ghost
`hindexed_payloads_inv`.

**The B2 gate's loop invariant** (`SequencePayloadLemmas.lean`):
`SPGateInv ro out ra inp` — `len` · `view` (an emission is the tick's
view or `0`) and, guarded (`ro = true`): `fresh` (a fire is at a fresh
leader tick with a nonempty view) · `reg_none` · `reg_some` (the register
names a fire's ballot) · `reg_dom` (…and num-dominates every fire) ·
`once` (two fires carry distinct ballots). `spGateInvariant_tick` (the
fire/hold split; wire facts `hown`/`hmono` on the zipped input as
leg-local premises) is the block's obligation (`spGateBody_eq ▸`) and,
through `spGatedTrace_inv`/`_inv_take`, the pure layer's. In SP the
ballot wire is an INPUT, so the in-program clause quantifies the wire
facts (`∀ me, own → mono → SPGateInv …`).

**The faces are now non-inductive** (statements unchanged — `SPRequires.
send_once` and `PaxosCore` consume them untouched): the pipeline's legs
are named (`spGateInput`, `spMaxSlots`, `spCommits`, `spGatedPayloads`,
`spIndexed`, `spStamp`; `spSentTrace_def` is `rfl`) with one `getElem`
reader each (`spSentTrace_getElem`, `spSentTrace_mem_at`: a sent key at
tick `t` is a stamped payload of `indexed[t]` or a recommit of
`gated[t]`, at a leader tick, at the tick's ballot); the reign calculus:
`reign_contig` (num-ascent + ownership ⇒ a reign is contiguous),
`reign_first` (`Nat.find`: the reign's first leader tick is fresh and
minimal), `spGatedTrace_fires_at` (fresh leader tick, nonempty view, no
earlier fire at the ballot ⇒ the gate passes the view — read off the
step with the register from `_inv_take`), `spGatedTrace_once`,
`spGatedTrace_reign` (the reign's first leader tick fires on the input
view; no tick of the reign after it fires, so none rebases).
- `spSentTrace_open` (variant-generic, 40 ln): read the tick; a payload
  sits above the gated view's max (`IPInv.rebase` at `u = v`), a
  recommit is the champion (`recommitList_value_best`).
- `spSentTrace_key_nodup` (~145 ln): `nodup_flatten`; within a tick fresh
  slots (`range'`) above the recommits; across ticks the same key means
  the same reign (`reign_first`'s minimality), whose single fire rebased
  once — `IPInv.dom` separates payload slots, `IPInv.rebase` puts the
  fire tick's recommits below every later payload.
- `spSentTrace_open_input` (~70 ln): `pinned` moves the tick's input view
  to the fire tick's; `IPInv.rebase` from the fire tick's recorded max.

**Numbers.** `SequencePayloadLemmas` 1992 → 1760 (monoliths + fusion
−1120; gate invariant + step +330, leg readers/length bookkeeping ~250,
reign calculus ~120, faces ~255 — the structure goal is met; the raw
count is modest: Lean's dependent-index `getElem` proofs cost 10–15 ln
per reader). `SequencePayload` 781 → 809 (the gate's `invariant`
clause + obligations). `IndexPayloads` 144 → 436 (the invariant layer).
`PP1b` 386 → 330 (polish: `pP1bQuorum_eq_some_iff` — the `none`/`some`/
`if` pyramid once, in `PP1bLemmas` — and a `hviews` face; `pP1bViews`
respells 15 → 4, all in contract statements). `HydroGenKnot.coSimpBase :=
coSimpRules` (the two 209-name arrays were identical). Timings: SP 33 s
(was 33–37), PaxosCore 30 s, IP ~1 s, PP1b ~5 s, LeaderElection 267–272 s
(unchanged); full build 5 m 08 s, 809 jobs; censuses unchanged
(`sequence_payload` 5/2/0). Full gate green (zero sorries, axioms
standard-three, `falsify`/`explore`/`v2paxos`/`v2twopc` 4/4).

**Budgets (the pass, with `scripts/budget_probe.sh`: elaborate one file
with its `set_option maxHeartbeats N in` replaced, `lake env lean`).**
The option prefixes the whole `hydro def` COMMAND — the budget is the
def's entire elaboration (body, ghost layer, obligations, the generation
phases that share its `CoreM` context), not one tactic; a "timeout at
`simp at htj`" is where the counter crossed, not what was expensive.
Measured at the default 200 000: PASS and option DELETED —
`IndexPayloads` (1 M → default), `PP1b` (1 M), `PBallotCalc` (1 M),
`PLeaderHeartbeat` (1 M), `AcceptorP1` (1 M), `AcceptorP2` (1 M),
`Recommit` (1 M), `SequencePayload` (250 k), `RequestResponse` (1 M),
`Quorum` ×2 (1 M), `TransferCheck` (3.2 M), `HydroTickCheck` ×9 (1.6 M),
`HydroGenCheck` ×10 (1.6 M). FAIL at 200 k, set to the measured
power-of-two: `PaxosCore` 3.2 M → **400 k** (fails at 200 k, passes at
400 k: the K4 `base`+`step` obligations plus five knots in one command),
`LeaderElection` 3.2 M → **1.6 M** (fails at 800 k, passes at 1.6 M: the
three-knot tower; its 272 s is its own knot pipeline, pre-existing).
`maxRecDepth 65536` kept where present (not probed this round).

**Design note (open, for the user).** The in-program `invariant` clauses
on `index_payloads` and the gate instantiate `ipInvariant_tick`/
`spGateInvariant_tick` through the construct; the pure faces instantiate
the SAME step lemmas again (`ipRun_inv`, `spGatedTrace_inv`) because
`SPEmission` — K4's interface, stated over two `leader_election`
valuations — is a pure predicate on input traces, so the faces must be
pure theorems. Consequence: `IPEnsures.rebase_dominates`/`slots_dominate`
and the gate's ghost `hrcGated_inv` have no consumer today; they are the
modules' honest exported contracts (the Verus loop invariant at the
loop), the load-bearing path is the pure wrapper. Kept (option A) per the
milestone exchange; a module-level restatement of `SPEmission` (so SP's
faces are consumed) would touch K4's statement shape and is out of scope.

**QUEUED (user, verbatim intent).** Dig into why the SP/IP proofs are
still so big (the ~250 ln of `getElem`-through-zip/map readers, the
~330 ln gate invariant+step, the ~255 ln faces) and whether the
infrastructure (the `tick` construct, generated readers/leg lemmas,
invariant-clause ergonomics, zipped-trace indexing) could make them
small enough to live INLINE in the program as ghosts/prove legs rather
than in a separate lemma file.

**Verus rows.** `IPInv`/`SPGateInv` = `invariant` clauses on the
`sliced!` loops (loop-variable normal form: emissions-so-far + register,
spectator inputs indexed by position); `ipInvariant_tick`/
`spGateInvariant_tick` = the loop-body proof obligations; the faces =
`proof fn`s that read ONE iteration (`scanAcrossTicksTrace_getElem`) and
cite the invariant at that iteration (`scanAcrossTicks_invariant_take`) —
no induction at the call site; `reign_first` = a `choose`-style minimal
witness (`Nat.find`).

**Gotchas (canon).** (i) `⟨…, fun h => by cases h, …⟩` — a `by` block
inside an anonymous constructor swallows the following `, …` as tactic
arguments ("multiple induction targets"); parenthesize `(fun h => by
cases h)`. (ii) `rw [List.getElem_map]` after `unfold f` fails with
"motive is not type correct": the index proof `ht : t < (f …).length`
no longer matches the unfolded list — `have ht' : t < (map …).length :=
ht; show (map …)[t]'ht' = _` first. (iii) `rw … at hv hq` where `hq`'s
type depends on `hv` ("motive is not type correct") — when the two
spellings are definitionally equal (a `let`-bound wire vs its pure
trace), don't rewrite: pass the hypothesis as is. (iv) `subst h` with
`h : a = b` both variables eliminates `b`; orient the equation so the
name you keep is on the left. (v) an untyped spectator/lambda binder in
an `invariant` predicate whose only use is a projection (`x.2.1.1.num`)
leaves a metavariable that surfaces as "argument has type … but is
expected" at the generic lemma — type the binder. (vi) a Python
slice-and-splice edit keyed on a docstring that also appears as a
structure FIELD docstring duplicated 700 lines of a file; key edits on
unique anchors (declaration heads).

### D63 — the SP/IP proofs move inline: option-indexed readers, construct-emitted run/at faces, consumer-shaped callee contracts; the pure mirror and the reign calculus are gone

**The question (D62's queue, verbatim intent).** Why are the sequencing
proofs still big, and could infrastructure make them small enough to
live INLINE as ghosts/`prove` legs? **Headline metric (user ruling):**
the ratio of important proof (key protocol behaviour, what a human would
think about) to plumbing — make the interesting proofs easier and more
straightforward to write.

**Checkpoint #1 taxonomy (measured, D62 state).** `SequencePayloadLemmas`
1760 ln: pure calculus 332 (23 %), faces 293 (20 %), gate invariant +
step + wrappers 277 (19 %; step alone 196), readers 219 (15 %), reign
calculus 214 (15 %), vocab 102. IP invariant material ~200
(`ipInvariant_tick` 155). `SequencePayload` prove legs 430 (≈90 ln
`getElem` plumbing on `hlogface`, ≈80 respells). Root causes: **RC1**
dependent-index `]'` reads (143 in SPLemmas, 88 LE, 68 PP1bLemmas; 179
`.length` lines) — every reader proves a bound before it reads; **RC2**
history-indexed invariant clauses spelled with indices → a snoc case
split per clause (14×/10×); **RC3** a pure mirror of the pipeline (six
named pure legs + readers; the invariants instantiated twice; 76 ln of
five-premise signatures); **RC4** a hand `Nat.find` reign calculus;
**RC5** no construct-emitted run face / tick reader. Toy repros before
committing: a reader 32 → 13 ln option-indexed; the gate step 196 → 68
ln in history form (membership/`Pairwise` + a snoc simp set).

**Rulings.** E1 (construct-emitted readers) · E2 (generic `Trace` layer)
· E3 option C-lite: the emission faces become `SPEnsures` FIELDS over the
UNCHANGED pure `SPEmission` (`spSentTrace`/`SPEmission`/
`spSentTrace_prefix` stay pure — K4's interface) · E4: `index_payloads`
carries no invariant at all (per-step facts + register chaining) · LE in
scope. **E5 (`ghost let` aliases of program wires) REJECTED**: "ghosts
should be able to refer to the values instantiation of non-ghosts" —
every unfolded program-wire term in a proof is a bug in how the face or
binder reached that proof. The diagnosis under that ruling: the 25+8
respells in SP were INHERITED from `AP2Ensures.log_face/ack_decomp/
from_key_cap/from_src`, which were stated in `acceptor_p2`'s own pure
vocabulary (`foldAcrossTicksTrace … ap2QualBatches …`, `ap2From (fun j' =>
batchCuts …)`). A callee's contract stated in its fold vocabulary forces
the caller to respell its wires into that vocabulary. Fix: contracts are
consumer-shaped and wire-level.

**Infrastructure (E1/E2).**
- `HydroTick.lean` **v3**: for every single-output `tick` block the
  construct emits `h<out>_run : ∀ i, out i = scanAcrossTicksTrace (<out>_step
  i) seed inpTrace` and `h<out>_at : ∀ i n e, (out i)[n]? = some e ↔ ∃ x₁_t …
  xₘ_t, (x₁ i)[n]? = some x₁_t ∧ … ∧ e = (step i ⟨state before⟩
  (x₁_t,…)).2` — option-indexed, destructured per input (proof: `Iff.trans`
  on the zip reader + defeq, never a keyed `rw`); with an `invariant`
  clause also `h<out>_inv`/`h<out>_inv_take`. The `tick` obligation is now
  `∀ i n out stv x₁_t … xₘ_t, (x₁ i)[n]? = some x₁_t → … → out.length = n → P
  out stv → P (out ++ [(step …).2]) (step …).1` — a loop body with NAMED
  per-input reads (the bridge to `scanAcrossTicks_invariant?` is
  `Trace.getElem?_zip_eq_some'` projections). Toys `toy_tick2/3/6` ported;
  `toy_tick6` consumes `hbumped_at`/`hbumped_inv_take`.
- `Trace.lean` **`TickReads`** (+~390 ln, all consumed): `[n]?` through
  `zip`/`map`/`cons`, `scanAcrossTicksTrace_getElem?`(`_eq_some`),
  `foldAcrossTicksTrace_getElem?`; `Trace.hist` (= `zip inp out`, the
  history a clause talks about) with `hist_snoc`/`mem_hist_iff`/
  `hist_mem_pairwise`; the snoc simp set (`forall_mem_snoc`,
  `exists_mem_snoc`, `pairwise_snoc`, `length_snoc_le`);
  `scanAcrossTicks_invariant?`/`_take?` (option-indexed step);
  `scanAcrossTicksState_induct` (a register fact preserved by steps holds at
  every prefix), `scanAcrossTicksState_chain` (a transitive relation chained
  across ticks satisfying a condition — replaces the "register climbs"
  history clauses), `Trace.first_tick` (`findIdx?`: the first tick
  satisfying a decidable predicate — `Nat.find` without the classical
  detour), `pairwise_reads`/`pairwise_of_reads`, `mem_of_read`, `read_inj`,
  `read_lt`; `Trace.sum_countP_le_one` (at most one `p`-element in a trace
  of batches when each batch has ≤ 1 and no two ticks both do).
- `Values.lean`: wire-level `rfl` readers for the stream boundary
  (`values_map`, `values_allTicks_noOrder/_totalOrder`,
  `values_broadcast_closed`, `values_values_noOrder/_totalOrder`) — see
  gotcha (vii). `Types.lean`: `logView_entry_value`.

**Modules.**
- `AcceptorP2.lean` (603 → 582): `AP2Ensures` consumer-shaped — `log_len_le_ck`,
  `log_covers_mono` (option-indexed), `log_entry_src` (∃ pool P2a at the key
  ∧ pool agreement at the key ⇒ the value), `acks_by_acceptor` (∃ `byAcc`
  decomposition ∧ per-acceptor cap ≤ `pool.countP key` ∧ each `Ok` ack opens
  to a max read and a covering log read), `ack_src`. The fold's register is
  read in ONE place: `hlog_pool` (∃ `pool : Nat → Multiset`, the published
  log at `t` is `(ck_t, logView (pool t))`, `pool` monotone, entries from
  qualifying P2as) — the honest module-internal object, ∃-abstracted so no
  consumer sees `foldAcrossTicksTrace`. Zero `ap2QualBatches/ap2From/
  foldAcrossTicksTrace` in the interface.
- `IndexPayloads.lean` (436 → 284): no `invariant` clause. `hstep` (body =
  `ipStep`), `ghost obtain ⟨reg, hreg_at, hreg_succ, hreg_stall⟩` (the register
  named: read / step / stall), `hreg_climb` (12 ln: the register climbs
  while no rebase), faces `rebase_dominates`/`slots_dominate`/`slots_nodup`
  ~15 ln each by `cases` on the rebase.
- `SequencePayloadLemmas.lean` (1760 → 579 → **merged into
  `SequencePayload.lean` and DELETED**, user ruling): what remained was
  exactly the pure vocabulary (`spGatedTrace`/`spSentTrace`/`SPEmission`/
  `SPChosen`),
  the recommit/`logView` calculus (`recommitList_value_best`,
  `spSentTrace_prefix` for K4's `hemW`), the body-shape lemmas
  (`spGateBody_eq`, `spStamp_eq`), and `SPRequires` restated option-indexed
  (`mono : Pairwise (num ≤)`, `lead_ne`, `stable`, `pinned` on `[t]?` reads).
  DELETED: the six named legs and their readers, the old `SPGateInv` and
  wrappers, the reign calculus, the three faces, `SPRequires.send_once`.
- `SequencePayload.lean` (809 → 1153 with the faces moved IN; 1721 once the
  pure layer is merged in as the file's opening section — the sequencing
  module is ONE file: pure layer, gate invariant, contract, program):
  `SPGateInv ro ra
  hist` in history form (four guarded clauses `reg_none`/`reg_some`/
  `reg_dom`/`once` over `Trace.hist`), `spGateInvariant_tick` ~70 ln (was
  196 + 80 wrappers). `SPEnsures` gains `emission_open` (variant-generic),
  `emission_once`, `emission_open_input` (both guarded, under `SPRequires`).
  The ghost layer, in program order: register naming (`hgate_ex` → `ra`),
  one reader per wire (`hgate_at`, `hrc_at`, `hmax_at`, `hsend_at` — each a
  per-tick statement in protocol terms, proved by `Iff.trans` on the
  generated `_at`), then the protocol: `hmem_tick`/`hemit` (a sent key is a
  leader tick's, at the tick's ballot), `hkeys_nodup` (within a tick),
  `hreign` (the reign of a ballot via `Trace.first_tick`: fresh, contiguous,
  minimal — 61 ln, no `Nat.find`), `hfire` (the gate at a tick, decoded),
  `hreign_fire` (the reign's single fire: the first leader tick fires on
  the input view; no later tick of the reign fires — `once` on the history
  at prefix `t+1`), `hemission_open`, `hsend_disjoint` (distinct ticks never
  share a key: payload/payload by `slots_dominate`, recommit at the fire
  tick by `rebase_dominates`), `hkey_once` (`Trace.sum_countP_le_one` over
  the send wire — the wire-level send-once, consumed by `commit_spec`'s
  owner cap), `hemission_once`, `hemission_open_input`; the P2a pool
  (`hp2apool` by the `values_*` readers, `hp2a_src`, `hp2a_own`); the
  callee faces `hap2`/`hcq`/`hjr`. Prove legs: `emission_*` are the ghosts;
  `commit_spec` through `acks_by_acceptor` (per-acceptor unit caps:
  `sum_map_le_single` + `hp2a_own` + `hkey_once`; `exists_distinct_reps`);
  `log_len_le_ck := hap2.log_len_le_ck`; `log_covers_mono` two lines;
  `log_entry` via `log_entry_src` + `hemission_once` for the pool agreement.
- `PaxosCore.lean` (478 → 493): `LEEnsures.spRequires` builds the
  option-indexed `SPRequires` from LE's dependent faces
  (`List.getElem?_eq_some_iff`, `pairwise_iff_getElem`); the K4 step reads
  `hsp_step.emission_open_input hreq_step hvar i₂ hm₂` (one line, was a
  nine-premise pure call), `hsp.emission_once` ×3 for the former
  `send_once`; `hemW` keeps `spSentTrace_prefix`. **K4's statement, `paxos_
  safe_sched'`, `cq_safe_sched'`, `paxos_eager_den/_ballots`: byte-identical.**
- `LeaderElection.lean`: `p1aInvariant_tick` takes the read `bts[n]? = some x`;
  the `tick` obligation on the v3 binder order (`fun i _n _out _st _bt_t hx
  hlen ih`). Its p1a invariant stays in index form (in scope, not needed).

**Numbers — the metric.** Lines classified by hand (program text, Rust
quotes, contract statements and docs are neutral; "protocol" = reign /
single fire / send-once / emission faces / commit quorum / log entry /
recommit-champion calculus / the gate invariant and its step;
"plumbing" = reading one tick of a wire, register naming, length and
index bookkeeping, respells and `show`s, instance-shape casts,
dependent-index conversions, pipeline mirror legs and wrappers).
`SequencePayload` (+ the former `SequencePayloadLemmas`): **2569 → 1721
ln (−848, −33 %)**; protocol ≈ 1300 → ≈ 930 (the same facts, shorter: the reign
calculus 214 → 61, gate inv+step 277 → 91, the three faces 293 → 110 +
their 110 of shared reign/fire ghosts); plumbing ≈ 650 → ≈ 270
(readers 219 → ~115 one-liners-per-wire, pure mirror legs/wrappers
~330 → 0, prove-leg `getElem`/respell plumbing 170 → ~30). **Ratio
protocol : plumbing ≈ 2 : 1 → ≈ 3.4 : 1.** `IndexPayloads` 436 → 284
(invariant material 200 → 0; the faces are ~15 ln each). `AcceptorP2`
603 → 582 with a consumer-shaped interface (the 33 downstream respells
→ 0). Dependent-index `]'` reads in the SP pair: 166 → 6 (`.length`
lines 205 → 8); IP/AP2: 0. Not one `ghost let` alias. Infrastructure
added: `Trace` +393 (generic), `HydroTick` +123, `Values` +29, `Types`
+32 — one-time, every tick block pays for it. Inline census: every
sequencing face is a ghost or a `prove` leg in the program; the pure
vocabulary, pure calculus, and body-shape lemmas open the same file
(`SequencePayloadLemmas.lean` is gone; `Falsification` imports
`PaxosCore` alone). 808 jobs.
Timings: SP ~39 s as a file (was 33–37; the `hemission_iff` opens cost
~1.3 s each ×4 — instance-path unification on the `.sum` of a wire,
left), PaxosCore ~30 s, LE ~270 s (unchanged), full build ~6 m, 809
jobs. **No budget on SP** (the one timeout met this round was a hotspot,
gotcha (vii), not a budget). Gate green: zero sorries, axioms the
standard three, `falsify` PASSED, `explore` 2/2, `v2paxos`/`v2twopc` OK,
censuses unchanged (`sequence_payload` 5/2/0).

**Verus rows.** The `tick` obligation IS the loop body with named
per-input reads (`x₁_t` is "this iteration's input"); `h<out>_at` is
"reading one iteration of the loop"; `hlog_pool`-style ∃-abstraction is
the honest module-internal object (a `ghost` existential over the
register, exported without its definition); consumer-shaped contracts =
`ensures` over the function's RETURN values, never its locals;
`Trace.first_tick` = a `choose`-free minimal witness; history-form
invariants (`∀ x ∈ hist …`, `Pairwise`) = loop invariants quantified over
the sequence so far, not over indices.

**Gotchas (canon).** (i) `lake env lean file` uses STALE oleans of edited
dependencies — `lake build <Module>` after editing an import (burned once:
"all toys passed" against the old construct). (ii) `rw` is blind across
instance-projected or construct-shaped types vs their plain spelling
(`BoundedStream` vs `Multiset`; a generated step tuple vs `(a, b)`) — use
`exact`/`Iff.trans`/`Eq.trans` (defeq), and `@id T e` rather than `show T
from e` (the latter makes a `letFun` that blocks `rw`). (iii) `rw [h]`
with an under-applied generated hypothesis leaves stray `?j` goals — pass
the arguments (`rw [hlog_step j _ _]`). (iv) two `match` sites on the
same scrutinee generate DIFFERENT matcher constants — `omega` sees
different atoms; destructure once or `change`. (v) `obtain ⟨-, -, …⟩`
clearing the witnesses `v b l d` of a generated `_at` reader also clears
the hypothesis that mentions them ("unknown identifier `hvt'`"); use `_`
for witnesses you still read through. (vi) an untyped `∀ u gu, (w i)[u]?
= some gu` binder stalls `GetElem?` resolution ("typeclass instance
problem is stuck") and surfaces downstream as an unrelated "unsolved
goals" at the enclosing `by` — type the binders. (vii) **`fun j => rfl` /
`show` across a boundary operator (`values (broadcast_closed … (map
(allTicks w) f)) j = ((finRange).map (fun r => ((w r).sum).map …)).sum`)
cost 4.5 s per check (and the ghost type/value/leg replay ran it three
times = the 200 k timeout): the lazy-delta unifier unfolds the SIDE WITH
THE LOWER HEAD — `Multiset.map` → `Quot.liftOn` → `Quot.lift` four deep —
before whnf-ing the folded `Values` instance projection on the other.
Fix: one-step `rfl` readers at the semantics (`ValuesTick.values_*`) and
`simp only [p2as, values_values_noOrder, values_broadcast_closed,
values_map, values_allTicks_noOrder, mapPool]`; 13 s → milliseconds.**
(viii) `trace.profiler` (not `profiler`) attributes time to source
positions — the fast way to find a hotspot inside a `hydro def`.

### D64 — no theorems before the program: mirrors die, the register is the construct's `_reg`, bodies are read under `den`; ensures are over outputs only (K4 restated over entries); one once-per-ballot register for B1 and B2

**The question (D63's queue, verbatim intent).** Make the proofs "more
direct to the interesting bits", with no theorems before the program.
**Census at D63 (measured):** ≈4040 lines precede the `hydro def`s
tree-wide, ≈930 statement-necessary, ≈3100 proof-only (SP 818/1720,
Quorum 1420/1736 — 1320 proof-only, LE 359, Recommit 302, RR 245).
Root causes: a hand-written pure STEP MIRROR per module (`spGateStep`,
`ipStep`, `cqTick`/`cqwrTick`, `jrTick`, `p1aSendStep`, `pbcJump`,
`recommitList`), the body-shape bridge proving the construct's step
equals it (`*Body_eq`), facts about the mirror, and definitional run
faces (`run_eq`, `hrun`) that re-export the fold; plus the 5 s whnf
cliff on `∈ (payloads_to_send i).sum`.

**Rulings (all binding).** **E6** kill the mirrors: a per-tick fact is a
ghost about the construct's generated `<out>_step`, proved by `simp only
[<out>_step, den]` then the protocol argument — no pure twin, no
body-shape bridge, no `run_eq`. **E7** the construct emits `h<out>_reg`:
the register ∃-ABSTRACTED (`∃ reg, reg i 0 = seed ∧ read-per-leg ∧ step ∧
stall-per-input [∧ invariant before every tick]`) — consumers `ghost
obtain ⟨reg, …⟩` and never see the fold. **E8 (hoisting / exported ghost
theorems) REJECTED**: "ensures are over outputs, never internal wires;
consumers consume ensures, period" — `SPEnsures` is restated on the
module's OUTPUT wires (the published `a_log`s and `p_to_replicas`):
`log_entry_open`, `log_entry_agree`, `commit_spec`, `commit_distinct`
(with `CommitWitness` = owned/chosen/`LeaderOpen`/entries_agree, and
`LeaderOpen pb pl p1bs b slot v` = ∃ leader tick of `b` whose input view
characterizes `v` via the `logView` champion), `log_len_le_ck`,
`log_covers_mono`. `SPEmission`/`spSentTrace`/`spGatedTrace`/`spGateStep`/
`spStamp_eq`/`spGateBody_eq`/`spSentTrace_prefix`/the decode lemmas:
DELETED. User's principle (verbatim intent): persistence across
re-execution with larger inputs = the generated mono ∘ the top-stage
faces — decisions are fixed across knot stages; a decision-independent
future guarantee would be a new free-theorem family, not needed. **E9**
consumer-shaped callee faces (RCEnsures per-tick `RCTick`; LE's
`reign`; AP2's unit caps → distinct acceptors). **E10** layout: only
statement prerequisites precede the def, under a "Prerequisites for the
contract face — the program starts at `hydro def M`" header. **No
separate theory files** ("short and sweet and inline"; `RecommitTheory.
lean` was created then deleted). `@[reducible] def Values` RATIFIED (the
enabler for `den`: every interpretation file had only suppressed the
`classDefReducibility` linter; full tree 5m15s vs 6m03s).
**B2 falsifier** (user: "I don't see what's the actual bug … it's not a
safety violation if we double emit the same slot + payload twice"):
RIGHT — the duplicate recommit is a duplicate delivery, not a safety
violation (and the old mirror witness checked only duplicate keys). The
B2 hazard is the REBASE: a reign re-presenting its nonempty view
re-pins `index_payloads`' base to `max_slot + 1` on every tick, so two
fresh payloads on two ticks get the same slot at the same ballot and ONE
proposer commits two values at one slot (`SlotFunctional` violated).
The witness now runs the real `sequence_payload` (f = 1, two acceptors)
and checks the violation on `p_to_replicas` (faithful) vs slot-
functional (guarded); `lake exe falsify` rewired. **Queued (user):** the
same scenario driven through `paxos_core` like B1 (exe-only; kernel
`#guard` on the nested knots is prohibitive).

**Infrastructure.**
- `SimpAttr.lean`: `register_simp_attr den` — the denotation simp set:
  the pool-grade reducts in `Grades` (`mapPool_*`, `poolCount_*`,
  `poolFilter_*`, `poolFilterIf_true/false`, `PoolBot_*`, `PoolFold_*`,
  `poolChain_*`, `poolWeakenOrder_*`, the `poolJoin/poolAntiJoin/
  poolFilterNotIn/listEnumerate` unfolds) and the `values_b*` operator
  readers + stream-level `values_*` in `Values`. `simp only [<out>_step,
  den]` turns a construct step into the body's quotient formula.
- `HydroTick.lean`: `mkRegGhost`/`RegCtx` (top-level builder — the
  elaborator function had outgrown the compiler budget) emits `h<out>_reg`
  for every stateful block, one read clause per output leg (multi-output
  blocks get per-leg `_run`/`_at` too; `ValuesTick.tick_scan_pair`/
  `unslice_*`); a stream state's type is filled from the step's binder
  (`_`).
- `Types.lean`: **`OnceInv`** — the once-per-ballot register's loop
  invariant (`reg_none`/`reg_some`/`reg_dom`/`once` over `Trace.hist`,
  guarded by the variant flag) with `init`/`fire`/`hold`: B1's P1a
  dedup register and B2's recommit gate are the SAME register; the two
  obligations are now `OnceInv.fire hx hlen hown hmono ih hguard hfired` /
  `OnceInv.hold …` (the LE one was 140 lines, the SP one 60).
  **`LeaderDiscipline`** (own / mono-`Pairwise` / lead_ne / pinned /
  `reign`, all `[t]?`) — `LEEnsures.discipline : 1 ≤ qs →
  LeaderDiscipline …` IS `sequence_payload`'s requirement; `SPRequires`,
  `SPGateInv`, `LEEnsures.spRequires` (PaxosCore's bridge), `p1aSendStep/
  p1aInvariant/p1aInvariant_tick` DELETED. `reign` is derived once, in
  LE (ghosts `hstable`/`hreign`), consumed by SP as `hreq.reign`;
  `stable` left the face (subsumed). `logView_keys_nodup`, `rcEntries`/
  `rcCount`.
- `Grades.lean`: `listEnumerate_indices_add` (enumeration indices from a
  base are `List.range'`). `Trace.lean`: `Trace.two_reads_le_sum` (two
  ticks' batches sit together below the pool), `List.nat_max?_ge`,
  `List.map_filterMap_sublist`, `List.eq_of_keys_nodup`,
  `mem_pool_of_mem_batch` (from AP2).

**Modules (before → after; "def at" = first line of the program).**
- `Recommit.lean` 416 ln, def at 92: `RCTick`/`RCEnsures` prerequisites;
  ~250 ln of champion calculus as ghosts along the program (`hopen/htry/
  howned/hslots_nodup/hslot_le_max/hmax_ge/hview_empty/hvalue_best/htick`);
  faces via `hp_log_chained_at`/`hp_max_slot_at`; `recommitList` gone.
- `SequencePayload.lean` **1720 → 1057, def at 818 → 203** (everything
  before the def is statement-necessary: `SPDec`/`SPSched`/`SPChosen`/
  `LeaderOpen`/`CommitWitness`/`SPGateIn`/`SPEnsures`). The gate block's
  invariant is `OnceInv` (bal = the input's ballot, fired = nonempty
  gated view); its obligation is FIRE/HOLD in 15 lines. Ghost layer:
  `hrcGated_reg` → `ra`; `hfire` (the gate decoded); `hgate_reads`; the
  callee faces `hrc`/`hip`; `hsend_at` (the send wire read, under `den`);
  `hmem_tick`, `hkeys_nodup`, `hreign_fire` (consumes `hreq.reign`),
  `hsend_open` (a sent value is characterized against the gated view),
  `hsend_disjoint`, `hkey_once`, `hsend_once`, `hsend_opened` (→
  `LeaderOpen` on the INPUT view via `pinned`); the P2a pool (`hp2apool`
  by the `values_*` readers, `hp2a_src/own/agree`); `hap2`;
  `hlog_entry_src`; `hcq`/`hjr`; `hcommit` (join → chosen through the
  unit caps); `hwitness`. Internal emission facts are in read form
  (`(payloads_to_send i)[t]? = some e ∧ k ∈ e`) — no `.sum` membership
  (the whnf cliff is gone). 45 s → 34 s.
- `PaxosCore.lean` 493 → 345: K4 restated — chosen-at-`b₁` (top) ∧ every
  top entry at `(slot, b₁)` carries `val₁` ∧ `b₁ < b₂` ∧ this stage's
  leader OPENS `v₂` at `b₂` (`LeaderOpen` on `le`) ⇒ `v₂ = val₁`. The
  step: quorum intersection, the covering entry in the opening view,
  `logView_entry_value` + `log_entry_agree` for the champion's value,
  dichotomy (`= b₁`: the entry lifted to the top by `hpublift`; `>`: `ih`
  on `log_entry_open`). `hemW`/`hlift₁₋₃`/`spSentTrace_prefix` gone.
  Headline: `commit_distinct` + `hI` once per order. `PCEnsures`,
  `paxos_safe_sched'`, `cq_safe_sched'`, `paxos_eager_*` byte-identical.
  31 s → 27 s.
- `IndexPayloads.lean` 284 → 203, def at 59 (only `IPEnsures` before):
  `hindexed_payloads_reg` → `reg`; `hslots` (a tick's slots are `range'
  base batch.length`, by `den` + `listEnumerate_indices_add`), `hadvance`,
  `hreg_climb`; faces unchanged. `ipStep/ipBody_eq/run_eq` gone.
- `LeaderElection.lean` 1044 → 940, def at 152: the P1a block is the
  Rust lines (fire ⇔ trigger ∧ (faithful ∨ ballot ≠ lastSent)) with the
  `OnceInv` invariant; `hp1a_out_reg`; `hsrc` (a release is its tick's
  ballot at a trigger-true tick, by `den`), `hnodup` (guarded, via
  `List.nodup_flatten` + `once`); `hmono`/`hstable`/`hreign` ghosts;
  `discipline` leg assembled from them. `hzip_at` gone. Still ~265 s.
- `RequestResponse.lean` 332 → 271, def at 66: `hjoined_reg` → `rem`
  (a STREAM state through `_reg`); `hjoin_tick`, `hrem_window`
  (anti-join = key filter), `hrem_sub`; `join_src` by the window below
  the metadata consumed so far; `join_complete` restated consumer-shaped
  (`(batchCuts …)[u]? = some rb → (md i)[u]? = some mb → …`, the
  `atomic` availability made explicit — the metadata wire must tick at
  `u`; no consumers), proved by `hkeep` (the register tracks `k`'s
  metadata while `k` is unanswered; `Trace.two_reads_le_sum` for the
  once-responder). `jrTick/jrBody_eq/jr_run_*` gone.
- `PBallotCalc.lean` 219 → 199, def at 48: the jump closure is INLINE
  (paxos.rs:365–379 quoted); `hp_ballot_reg`; `hjump_le`/`hballot`/
  `hovertake` over `@id τ (p_ballot_step i n rm).k` (gotcha ii); `pbcJump*`
  gone.
- `AcceptorP1.lean`: `scan_chain_max` → the `hmax` ghost (E10). `AcceptorP2`:
  the generic batch lemma → `Trace`.
- `Quorum.lean` — **NOT converted (mini-checkpoint, pending user ruling).**
  The mirror (`cqTick`/`CQState`/`cqBody_eq`/`cq_run_count`/
  `collect_quorum_values_eq`) is also the ANCHOR of the liveness layer:
  `Liveness.lean` defines `cqTickS` (the list-level machine step), proves
  `cqTickS_coe` from `cqBody_eq` + hand-chained per-operator couplings,
  and `cq_live`/`cq_values_at_derived`/`cq_emit_attains` scan `cqTick`. No
  E8-compliant way exports the block's step from inside the def. Options
  put to the user: (1) engine-emitted global step constant + liveness
  re-anchored on it and the generated couplings (engine extension); (2)
  Quorum inline, the mirror relocated to Liveness as its anchor (counting
  proof duplicated); (3) defer Quorum to the queued quiescent-completeness
  (rung 2) work, which is what makes (1)'s tightness generic.
  Recommendation recorded: 3 now, 1 later.

**Numbers.** Tree-wide lines before the `hydro def`s (converted
modules): SP 818 → 202, IP 128 → 58, LE 358 → 151, RR 244 → 65, PBC 109
→ 47, PaxosCore 161 → 120, Recommit 302 → 91 — ≈2120 → ≈735, and every
remaining pre-def line is a `Dec`/`Sched`/`Ensures`/vocabulary
definition (statement-necessary) except Quorum's 1320 (pending). Mirrors
deleted: 7 (`spGateStep`, `ipStep`, `jrTick`, `p1aSendStep`, `pbcJump`,
`recommitList`, + `SPEmission`'s pure sent-trace); body-shape bridges
deleted: 6; `run_eq`-style definitional run faces: 0 remain in the
converted modules. Full tree **809 jobs, 5m22s**, zero sorries, axioms
the standard three (48 reports), `falsify`/`v2paxos`/`v2twopc`/`explore`
green, censuses unchanged.

**Verus rows.** `h<out>_reg` = a ghost existential over the loop's
mutable local, exported without its body (the loop is read as "there is
a register with these step/read laws"); `simp only [step, den]` = the
loop body's own semantics at the concrete types (no spec twin to keep in
sync); `OnceInv` = one reusable loop invariant for a "do once per key"
register, instantiated by what the key is and what firing means;
`LeaderDiscipline` = the requires/ensures handshake stated ONCE at the
composition point; `CommitWitness`/`LeaderOpen` = the caller's
postcondition phrased in the caller's return values and its arguments'
history (`[t]?` reads), never in callee locals; the K4 invariant quantifies
over the knot's top-level outputs (`sp'.2.1`) and this stage's inputs
(`le`) — a loop invariant over the function's own variables.

**Gotchas (canon).** (i) **`ghost obtain ⟨-, …⟩` with `-` on the ∃-WITNESS
clears every hypothesis depending on it, silently** — the later "unknown
identifier `hp1a_at`" is the only symptom; name the witness (`⟨_ls, -,
hp1a_at, …⟩`). (ii) construct-step projections have type `(Values).
BoundedSingleton τ`, which TC does not see through (DiscrTree keys don't
delta projections): state step facts over `@id τ (step i s x).2.k` and
`rw [show (step …).2.1 = @id τ (step …).2.1 from rfl]` at the use site.
(iii) `simp only [step, den]` leaves `match some b with …` and pattern-
lambdas as projections; `simp only []` after `cases` reduces the match,
and the composed `fun x => (x.2 + x.1.1, x.1.2)` is defeq to the intended
closure by structure eta (`exact` a lemma stated with the plain closure
works; `rw` does not). (iv) `den` unfolds `listEnumerate`/`poolAntiJoin`
etc. (they carry the attribute): state generic lemmas over the unfolded
form or `rw [List.map_map]` the right number of times before `exact`.
(v) `decide (x ∈ poolWeakenOrder m)` after `den` has a `Decidable`
instance over `poolWeakenOrder m` while the proposition reads `m` — `rw
[decide_eq_false_iff_not]` fails on the instance mismatch; generalize the
instance (`∀ hd : Decidable _, (!@decide _ hd) = true ↔ …`) and `exact`.
(vi) `Trace.pairwise_reads honce huv (List.getElem?_zip_eq_some.mpr ⟨…⟩)
…`: the zip read's pair is not inferable from the components — state the
two reads as `have`s with the pair spelled. (vii) `rfl` after a `rw` that
already closed the goal is "no goals" — the construct's step proofs are
often closed by the rewrite itself.

### D65 — Quorum's turn: the two `use::state` registers get ONE loop invariant (`CQRegInv`), the 615-line twin dies, `QuorumTheory.lean` dies, and `cq_live` is re-proven through the corner (tightness = the generated couplings, no hand mirror)

**The question.** D64 deferred `Std/Quorum.lean` (1420 proof-only lines
before its two defs; the anchor of the liveness layer's hand mirror
`cqTickS`). User ruling: "do the quorum cleanup part of 1 but defer
actually doing liveness all the way to paxos core". Plan ratified
R1–R5: **R1** tightness as a corner-derived OUTPUT-level theorem
(`collect_quorum_tight`, no hoisted step); **R2** engine lift — the
`tick invariant` clause on STREAM states (quorum.rs's `state_null`
registers), toy first; **R3** `CQRegInv` + `init`/`step` as the one
pre-def proof budget (`OnceInv` precedent); **R4** `cq_live` statement
byte-identical, the D48 machine-walk proof retired; **R5**
`QuorumTheory.lean` deleted (`cqConsumed` → `Trace.lean`, generic keyed
pool algebra → `Grades.lean` `section KeyedAlgebra`, quorum vocabulary
+ decode → `Quorum.lean`'s prerequisites). Rider: the D64-queued B2
through `paxos_core` in `lake exe falsify`.

**Quorum.lean (rewritten, 1736 + 482 → 1421 lines).** Layout before
the defs, by kind: header 35 · count vocabulary (`cqOkCount`/
`cqKeyCount`/`cqOkProj`/`cqErrProj` + arithmetic) 60 · the keyed
counter (`cqCount`, `cq_reached_eq`, `cq_keys_filter_eq`) and the post-
`den` decode (`filter_notmem_keys_filter`,
`keys_filter_notmem_keys_filter`) ≈240 · faces (`CQEnsures`/
`CQWREnsures`, statements UNCHANGED; `cqDropped`) ≈100 · **`CQRegInv
min max notAll mbnm pfx`** ≈200 — `window_le : notAll ≤ pfx`; `window :
∀ k, cap → notAll.filter k = if dropped then 0 else pfx.filter k`;
`locked : min < max → ∀ k, cap → (k ∈ mbnm ↔ min ≤ ok ∧ key < max)` —
with `init`, `step_eq` (`min = max`: the `reached_min_count` branch)
and `step_ne` (`min < max`: the `filter_not_in(min_but_not_max)`
branch), `dropped_kills_batch`, `window_kpart`. Pre-def total 1420 →
661. **Both programs** carry the clause `(invariant ((just_reached_quorum
: List (Multiset K)) (not_all : Multiset (K × Except E Unit))
(min_but_not_max : Multiset K) (new_inputs : Trace _)) => CQRegInv min
max not_all min_but_not_max ((new_inputs.take just_reached_quorum.
length).sum))` and discharge it with `prove init := … exact CQRegInv.
init, tick := simp only [just_reached_quorum_step, den]; by_cases hmm :
max = min; …; exact CQRegInv.step_eq/step_ne …`. The Ensures are then
proven from the construct's ghosts over `den`: `collect_quorum` —
`htick` (a read decoded: `∃ b_t S, read ∧ CQRegInv S pfx ∧ e = …`),
`hcount_tick` (the per-tick crossing indicator), `hcount` (prefix
induction) ⟹ `emit_sound`/`emit_count`; `collect_quorum_with_response` —
`hopen` (an emission decoded), `hcross_tick` (the emission at the
crossing tick IS the key's consumed `Ok`s), `hpool_step` (the per-key
potential: emitted + (if locked then 0 else window `Ok`s) ≤ consumed
`Ok`s, plus the register disjunction), `hpool` (induction through `_reg`'s
`succ`/`stall`) ⟹ `emit_mem_sound`/`emit_le`/`emit_complete` (first
crossing via `Nat.find`)/`emit_pool_le`. DELETED: `CQState`/`cqTick`/
`cqwrTick` (the mirror), `cqBody_eq`/`cqwrBody_eq` (bridges), every step
obligation, `CQKeyGood` + the `key_step` twins (615 lines — the P5 debt:
the two blocks' shared register step was proven twice), the run facts,
`collect_quorum_values_eq`, the `hrun` ghosts. Censuses unchanged
(1/0/0 ×2); `cq_safe_sched'` byte-identical.

**R2 (engine).** `HydroTick.lean`: the register-only guard on
`invariant` is gone; a stream state's binder is typed at the `Values`
carrier (`Multiset …`), its seed is the grade's bottom (`ValuesTick.
seed`), and the obligations/`_reg`/`_inv_take` use `seedVal` for the
state tuple throughout. Toy `toy_keyed_inv` (two stream states + the
window/locked invariant) in `HydroTickCheck.lean`, axiom-free, `#guard`.

**Liveness (R1/R4).** `Liveness.lean` 455 → 238: the mirror section
(`cqTickS`, `cqTickS_coe`, `cqMachineTrace(S)`, `cqMachineTraceS_coe`,
`cqMachineFam`, the D48 `cq_live` proof) DELETED; what remains is the
schedule-side fairness vocabulary and the per-op fair kit.
`LivenessChain.lean`: kit 2 is now **`collect_quorum_tight`** — `↑(((cq
Sched h).1 0).view T) = (cq Values (fun _ => ↑(view T₀)) (fun i =>
cqDerivedChain (h i) (pacing () i) T)).1 0` — proven ENTIRELY from
generated artifacts: `collect_quorum_co_wf₁` (whose statement `change`s
to `CoTick.scanWf …` — the block's body coupling, no body respelled),
`collect_quorum_co_sr₁` (the machine leg), **`co_transfer [collect_
quorum]`** (the generator's own projection walk, run by hand at the
EXPLICIT derived decision: the rr leg IS the `Values` run at `fun i =>
cqDerivedChain …`), then `change` both legs to their formers (`CoTick.gS`
on `batchesFrom`, flattened/frozen; `CoTick.gR` on `batchCuts` of the
derived chain, summed), `famFreeze_of_mono` + `ofList_flatten` +
`IsCutChain.legal` (`corr_batch`: the derived decision is exact at its
horizon), and `CoTick.scan_comm` + `coerce_seed` (machine former coerced
IS the denotation former). **`cq_live`** moved here, statement byte-
identical to D48, proof = `cq_complete_mem` ∘ `cqDerivedChain_complete_at`
∘ `collect_quorum_tight` (+ `Multiset.mem_coe`); `cq_live_chain` is now
a one-line alias; the D48 non-vacuity witness (`wOdd`/`wResp`/`wOut`
`#guard`s, `example`) and the chain-fairness `#guard`s sit together.

**Re-anchoring map (old mirror fact → what carries it now).** `cqTick`/
`cqwrTick` → the construct's `just_reached_quorum_step` under `den`;
`cqBody_eq`/`cqwrBody_eq` → nothing (there is no twin to bridge);
`cq_run_count` → `hcount`; `cqwr_run_le_init` → `hpool`;
`collect_quorum_values_eq` + `cq_values_at_derived` + `cq_emit_attains`
→ `collect_quorum_tight`; `cqTickS_coe`/`cqMachineTraceS_coe` →
`CoTick.body_comm`/`scan_comm` — the engine already had these
generically (D61); the hand couplings were duplicates.

**B2 through `paxos_core` (rider).** `Falsify.lean` `paxosB2`: two
proposers, three acceptors, `f = 1`; proposer 0 (ballot `(0,0)`) is
elected by acceptors {0,1} and sends `99` at slot 0, accepted by
acceptor 0 only; proposer 1 (`(0,1)`) is elected by {0,2}, its view
carries the lone slot-0 entry (recommit + REBASE to 1), leads two ticks
(`x`, then `y`) with the view re-read on both (`snap = [0, 2, 0]`).
Faithful: proposer 1 commits `(1, x)` AND `(1, y)` (plus `(0, 99)`
twice — duplicate delivery, not a safety violation); guarded: `(1, x)`,
`(0, 99)`, `(2, y)`. 7 ms at `Eager`; `lake exe falsify` ALL PASSED.
`ySlot` is the record's only variant-dependent datum (the acceptor P2a
cuts and ack/join cuts name `y`'s placement).

**Numbers.** Full tree **808 jobs, 5m12s** (LE 271s, SP 36s, PaxosCore
28s, Quorum 10.0s vs 7.4 + 1.9s, LivenessChain 2.4s); zero sorries;
axioms the standard three; `falsify`/`v2paxos`/`v2twopc`/`explore`
green; censuses unchanged; `maxHeartbeats` call sites still two.

**Verus rows.** `CQRegInv` = the loop invariant on the two `use::state`
registers of `collect_quorum`'s `sliced!` block, stated once and
instantiated by both `collect_quorum` and `collect_quorum_with_response`
(the shared body); `step_eq`/`step_ne` = the loop body's proof per
STATIC branch (`max == min` is a compile-time flag in Rust too);
`h<out>_reg` = the ghost existential over the mutable locals, consumed
by `obtain`; `collect_quorum_tight` = the exec/spec agreement the
corner certifies — the Verus analogue is "the executable's output at
this cut equals the spec function at the spec-level decision", which
Verus gets from `ensures` on the exec fn; here it falls out of the
generated couplings with no per-program proof.

**Gotchas (canon).** (i) **A `tick invariant` block in a `hydro def`
WITHOUT `ensures` has DEAD prove legs**: ghosts replay only in the proof
leg, which only exists under `ensures`/`prove` (the file header said
so; the first `toy_keyed_inv` draft "compiled" with `by done` in a leg
— Lean's "this tactic is never executed" warning is the only symptom).
Every invariant toy must carry an `ensures` (`toy_tick6` precedent).
(ii) The generated `M_co_rr_ex`/`M_vdec` name the derived decision BY
CHOICE — `collect_quorum_co_rr₁` is stated at `Classical.choose ⋯`, not
at `batchDerive`, and a `change` of the `Values` run at the derived
chain into the corner's rr leg is NOT definitional (the Values-
elaborated step vs `CoTick.gR g` differ by the `rr` walk through the
body's static `if`). The route that works: `have hrr : C.1.rr = (cq
Values … (fun i => cqDerivedChain …)).1 := by co_transfer [collect_
quorum]` — the house closer at an EXPLICIT decision (`exact rfl` sees
`batchDerive (pacing ()) T h = fun i => cqDerivedChain (h i) (pacing () i)
T` by unfolding); state tightness at the ETA-EXPANDED decision `fun i =>
… (h i) …`, never `fun _ => … (h 0) …` (not defeq under the binder).
(iii) `rw [famFreeze_of_mono hmono]` fails on a family whose members
depend on `i` (`(h i).view`, `pacing () i`) — the mono proof's mvars are
not HO patterns; `rw [famFreeze_of_mono]; swap; exact fun t i => …`.
(iv) `batchCuts` TRUNCATES THE CHAIN at the first illegal cut (not the
element): a record naming a P2a the program never sent (guarded `y` at
slot 2 under faithful) ends the acceptor's trace there, and a `snap`
over-consuming the P1b stream ends the proposer's — `snap` is a per-tick
CONSUMPTION count on the ordered stream (`[0, 2, 0]` re-reads the two
P1bs), not a cursor position. (v) **(corrected — the first draft of
this entry misdiagnosed it)** a key reaching `min` and `max` in ONE cut
IS emitted (`just_reached_quorum` is computed from `current_responses`,
not from `not_all`; checked by `#eval`: `{7×4}` in one cut at `(2, 3)`
emits `{7}`, both programs). The empty-run failures that looked like a
quorum drop were (iv) + the empty-view non-rebase below. `nA = 2, f = 1`
was still wrong (an invalid configuration: `max = 2f+1 > nA`, so no key
is ever purged from `not_all`) and was replaced by `nA = 3`. (vi) The
in-tick `b` of an invariant
spectator is the INPUT WIRE inside the `tick` leg (`(b i).take …`), the
leg's read is `b_t` with `hb : (b i)[n]? = some b_t`.

**Quorum observation (for the upstream report, not a Lean bug).**
quorum.rs's `max` purge assumes AT MOST `max` responses per key: when a
key reaches `max`, its responses leave `not_all` and the key leaves
`min_but_not_max`, so if MORE responses for that key arrive later it
re-crosses `min` and is EMITTED AGAIN (checked: `collect_quorum (2, 3)`
on cuts `{7×3}, {7×2}` emits `{7, 7}`). The `responses` stream is typed
exactly-once with no retries parameter, so under the type the only
source of over-delivery is an upstream logic duplicate — exactly B1's
duplicate P1a/P1b (and, for `sequence_payload`, duplicate acks would
mean a duplicate commit delivery, benign per the D64 ruling). Worth a
doc comment upstream ("`max` must bound the per-key response count;
over-delivery re-emits"); `CQEnsures.emit_count` states the per-key
emission count faithfully, so a consumer that needs once-per-key must
supply the `cqKeyCount ≤ max` cap (as `cq_live`'s `hcap` does).

**Queued (user).** A decision-record VALIDITY mode: `batchCuts`/
`prefixCuts`/`snapshotCuts` silently return `[]` at the first illegal
cut, so a hand-written `PaxosCoreDec` that is wrong looks like "nothing
committed". Wanted: an execution mode (or an `Eager`-level checker over
a record) that flags the first illegal cut per member with its tick and
the offending increment — diagnostic only, no semantic change (an
illegal cut is still not a behaviour).

**Honest notes.** The ≈240 lines of count-vocabulary algebra and
post-`den` decode before the defs exceed the ≈150 estimate — they are
quorum-specific lemmas about the Ensures' own vocabulary (`cqOkCount`
etc.) consumed by both programs' obligations, not mirror-anchored, but
they are the largest remaining pre-def proof mass tree-wide. Liveness
rung 2 (Paxos-scale quiescent completeness; the generic tightness kit
generalizing `collect_quorum_tight`) remains queued per the user's
ruling.

### D66 — PP1b's turn: `PP1bLemmas.lean` dies; the keyed `fold_early_stop` gets a closed form; the D21 regress becomes a face of `p_p1b`; LeaderElection consumes outputs only

**The question.** The last Paxos module untouched by D63–D65: `PP1b.lean`
(330 ln) + `PP1bLemmas.lean` (1279 ln — the pure fold calculus, 68
dependent-index `]'` reads, and the fabricated-reign regress as pure
theorems over a hand mirror of the pipeline). User: "a similar clean up
for PP1b, cleaning it up a bunch and eliminating PP1bLemmas." Plan
ratified R1–R5: **R1** output-shaped `PP1bEnsures` (no `∃ okPool`, no
decision arguments in the face); **R2** `get_max_key` restated as
`List.argmax Prod.fst` over the full buckets (a program-closure
restatement, not the let-chain; the `#guard` smoke tests pin behaviour);
**R3** a GENERIC capped keyed fold with a closed form in `Grades.lean`'s
`KeyedAlgebra`; **R4** the D21 regress and its consumer theorems become
`PP1bEnsures` faces proven inline; **R5** `PP1bLemmas.lean` deleted,
generic duplicates deleted. The keyed-op fidelity observation (below) is
QUEUED, not in scope.

**Diagnosis (why 1279 lines).** The same mechanism as D64, in three
layers: (1) a HAND MIRROR of the pipeline — `foldEarlyStopBallots` (the
fold), `pP1bViews` (fold ∘ snapshot cuts), `pP1bFlags` (the whole flag
wire) — all `rfl`-unfoldings of program wires, exported to LeaderElection
through the definitional faces `flags_eq`/`accepted_eq`, which then paid
~55 ln of index transport (`hflag/hface/hflag_lt/hflag_true/hacc_len/
hbatch`) to read them; (2) ~330 ln of per-lemma inductions on
`p1bLogsInsert` (source, cap, freeze, count, nodup keys, bucket
uniqueness, `mem_cases`), each re-walking the association list with the
same `if k' = k / if length < cap` split — plus 133 ln on the
`get_max_key` fold (`holds_quorum`/`mem_self`/`ge_full`); (3) ~500 ln of
view/flag lemmas and the regress (`pP1b_no_false_full` 87,
`pP1b_ballot_stable` 100, `pP1b_leader_bucket` 46, `pP1b_quorum_pinned`
48) stated over the mirror in `]'` form, each read paying 3–8 ln of bound
plumbing. Generic duplicates: `eq_of_nodup_keys` (= Trace's
`List.eq_of_keys_nodup`), `count_ofList` (= `Multiset.coe_count`).

**The closed form (R3; `Grades.lean` `KeyedAlgebra`, +~230 ln generic).**
`insertCapped cap bs k v` is one `fold_early_stop` insertion into keyed
buckets; `keyVals k xs` the values that arrived at `k` in order. The
whole fold has a closed form — **`foldl_insertCapped_spec`**: the
buckets of `xs.foldl (insertCapped cap) []` have duplicate-free keys, and
`(k, vs)` is a bucket iff `k` arrived and `vs = (keyVals k xs).take (max
cap 1)`. ONE induction (`insertCapped_keys`, `mem_insertCapped`,
`take_maxcap_snoc`, `insertCapped_spec` → the fold) replaces every
bucket lemma: source = `mem_keyVals` (`take ⊆ filterMap ⊆ xs`), cap =
`length_take`, **freeze** = `keyVals_take_of_prefix` (once `cap` values
arrived at `k`, every later prefix's bucket is the same `take`),
multiplicity = `keyVals_map_pair_sublist` (the bucket re-keyed is a
`List.Sublist` of the source — so `Multiset.coe_le` gives the embedding
with multiplicity for free), uniqueness = `foldl_insertCapped_unique`.
`p1bLogsInsert quorumSize` is now an `abbrev` of `insertCapped
quorumSize`. `get_max_key` (R2): `p1bMaxQuorumBallot q logs :=
(logs.filter (q ≤ ·.2.length)).argmax Prod.fst` — Mathlib's `argmax` IS
`foldl (argAux …) none`, the same first-maximal fold the hand `blt`
version was, over the D60 `LinearOrder (Ballot)`; its three lemmas
(133 ln) are `List.argmax_mem` + `List.mem_filter` +
`List.le_of_mem_argmax` (`p1bMaxQuorumBallot_spec` 12 ln, `_ge_full`
15 ln). `p1bOkPair m := cqOkProj (m.ballot, m.res)` — so the
consumed-pool bridge (`okProj_pairs` + `consumed_okProj_le`, 37 ln) is
`Multiset.filterMap_map` + `rfl` inside one ghost, and
`p1bOkPair_eq_some` replaces every `unfold p1bOkPair; cases m.res` block
(two 10-ln copies in LE, two in PP1b).

**PP1b.lean (rewritten, 330 → 619; no `]'` read, was 68 + 23).** Layout
before `hydro def p_p1b` (line 246): header 34 · program closures 45
(`p1bLogsInsert`, `p1bMaxQuorumBallot`, `pP1bQuorum`, `p1bOkPair`,
`p1bPairDecEq`) · closure readers ~65 (`pP1bQuorum_eq_some_iff`,
`p1bMaxQuorumBallot_spec`/`_ge_full`, `p1bOkPair_eq_some` — the one
pre-def proof budget, the D65 "decode lemmas of the vocabulary"
category) · `PP1bRequires` 25 (restated in `[t]?` form over the
module's OWN flag output) · `PP1bEnsures` 55 · `PP1bDec` 13. The let-chain
is byte-identical; the three mirror ghosts (`hviews`/`hflags`/`haccept`
= `rfl` faces exporting the mirror) are replaced by READER ghosts at the
wires, each read through `den` (three new `rfl` readers in `Values.lean`:
`values_assume_ordering`, `values_snapshot_fold_totalOrder` — the
stream-level `assume_ordering → fold → snapshot` chain, read at one
member as `(prefixCuts (selectOrder …) 0 d).map (·.foldl g init)`) and
the option-indexed `Trace.getElem?_map/zip_eq_some` + two new
`prefixCuts_getElem?_prefix/_mono`: `hviews_at` (a view is the fold of a
prefix cut), `hbucket` (the closed form at the view), `hsel_src` (a
selected quorum output is an `Ok` reply), `hflag_at`/`haccept_at` (the
flag / the accepted batch read the view, the ballot, has-largest),
`hleader` (a leader tick opened: the own bucket is `get_max_key`'s full
answer, the first `quorum_size` of the ballot's logs in the tick's
selection prefix, and the accepted batch is that bucket), `hcarry` (a
full own bucket carries to every later tick: prefix cuts ascend +
freeze). **The D21 regress is a ghost**: `hno_false_full` (strong
induction on `length − u`, ~55 ln): a flag-`false` tick with a full own
bucket forces a strictly higher full bucket (`argmax` dominates), whose
ballot was promised, hence solicited flag-`false` STRICTLY LATER
(ownership + ascent), where the mask carries forward — recurse. The
faces: `accepted_src` (12 ln), `leader_batch` (18: card facts + the
multiplicity embedding `bt.map (b, ·) ≤ filterMap p1bOkPair pool` via
`Sublist.subperm` ∘ `selectOrder_subpool` ∘ `emit_pool_le`), `pinned`
(12: two leader ticks at one ballot — freeze along the prefix order),
`ballot_stable` (~30: the reign's full bucket at `t+1` was solicited
strictly after `t+1`, where it is a full own bucket at a flag-false tick
— `hno_false_full`), `fails_src` (unchanged). PP1b 5.3 s (was ~5).

**`PP1bEnsures` (R1, internal statement change).** OLD: `∃ okPool,
PP1bEnsures prop qs pool pb phl okPool dOrd dSnap out` with `ok_pool_le`
(the internal success pool), `leader_gate` (over `pP1bViews`),
`flags_eq`/`accepted_eq` (the mirror, definitionally). NEW: `PP1bEnsures
prop qs pool pb phl out` — `accepted_src` · `leader_batch` (flag true ⇒
batch read, ballot read, has-largest true, `qs ≤ card`, `1 ≤ qs → card =
qs`, embedding) · `pinned` · `ballot_stable : 1 ≤ qs → PP1bRequires … (out.1
i) i → …` · `fails_src`. The faces are over OUTPUTS (D64 doctrine); the
decisions `dec.order`/`dec.snap` and the internal `okPool` no longer
appear in the face at all.

**LeaderElection (940 → 739; LEEnsures UNCHANGED; 251 s, was ~270).**
`ghost have hpp := p_p1b.ensures …` (no `obtain ⟨okPool, _⟩`); the 55-ln
transport block deleted; `hstable` = `hpp.ballot_stable hq1 i req` where
`req` packs `hbc.hasLargest_true`/`own`/`hmono` and `hsol` (the
solicitation chain through `hap1.reply_src` → the merged pool → `hsrc` →
`hhb.trigger_gate` → the knot `hknot`, now ending in `(pp.1 i)[u]? = some
false` — one `rw` through `List.IsPrefix.getElem`); `discipline.lead_ne`
and `.pinned` are one-liners off `leader_batch`/`pinned`; `view_promise`
and `providers` consume `leader_batch` + `accepted_src` (the bucket is
the multiset `bt` directly — no `Multiset.ofList (qlogs.map …)`). The
hand `pP1b_leader_bucket`/`pP1b_quorum_pinned`/`pP1bViews_bucket_sub_oks`/
`pP1b_ballot_stable` citations are gone.

**Numbers.** `PP1bLemmas.lean` 1279 → DELETED; PP1b pair 1609 → 619;
LeaderElection −201; generic additions `Grades` +230, `Trace` +15,
`Values` +12, `Types` +12 (`Ballot.lt_iff_blt`,
`num_lt_of_lt_of_owner`). Net ≈ −920 lines. Full tree **808 jobs**
(cache warm 1m17s; cold ~6m), LE 251 s, SP 35 s, PaxosCore 28 s, PP1b
5.3 s; zero sorries; 47 axiom reports all standard-three; `falsify`/
`explore`/`v2paxos`/`v2twopc` green; censuses unchanged (`p_p1b`
3/0/0); `maxHeartbeats` call sites still two.

**Verus rows.** `foldl_insertCapped_spec` = a library `proof fn` about the
`fold_early_stop` combinator (proved once, cited by every keyed fold);
`PP1bRequires` = `p_p1b`'s `requires` clause (the caller's discipline,
stated over the callee's own output flag — a cyclic `requires`, as the
`forward_ref` dictates); `PP1bEnsures.ballot_stable` = the `ensures`
that consumes it; `hno_false_full` = a `proof fn` with `decreases
(length − u)` inside the module; `hflag_at`/`haccept_at` = reading one
tick of an output as the composition of the Rust lines at the spec
level (`den`); LeaderElection's `hstable` = `requires` discharged from
the caller's own wires, `ensures` cited — no spec twin anywhere.

**Gotchas (canon).** (i) **The capped fold's closed form needs `max cap
1`**: with `cap = 0` a fresh key still opens the singleton bucket `[v]`,
so "the first `cap` values" is false; every consumer that has `1 ≤ cap`
rewrites `Nat.max_eq_left`. (ii) `Bool.and_eq_true` is an `Eq` of Props
in core (a simp lemma), not an `Iff` — `(Bool.and_eq_true _ _).mp` (or
`simp only [Bool.and_eq_true] at h`), not `.mp` on an `Iff`. (iii)
`cases hres : m.res` rewrites `m.res` in the GOAL too; re-anchor with
`show … ↔ _` before `simp only` on a `match`-defined projection
(`cqOkProj`). (iv) Mathlib's `List.argmax` needs `import
Mathlib.Data.List.MinMax`; `argmax_mem`/`le_of_mem_argmax` take OPTION
membership (`Option.mem_def.mpr h`), and `argmax_eq_none : argmax f l =
none ↔ l = []`. (v) **Reading a chain of stream ops at the denotation**:
`simp only [views, folded, quorum_outs, den]` — naming the let-bound
wires in `simp only` zeta-delta-unfolds exactly those, then the `den`
`rfl` readers fire; with `(by trivial)` as the `FoldOk` argument the
pattern variable still matches (no `show` of the folded instance needed
— the D63 lazy-delta hotspot does not recur). (vi) The `<+` (Sublist)
notation is not in scope in these files — spell `List.Sublist a b`.
(vii) `obtain rfl := Trace.read_inj h₁ h₂` on two reads of the same tick
is the house way to identify a face's witness with a program read
(`sel`/`b` here), replacing every `List.getElem_of_eq` cast.

**QUEUED (user ruling): the keyed-op fidelity item.** paxos.rs:548–560
is `quorums.into_keyed().assume_ordering(…).fold_early_stop(…).
get_max_key().snapshot(…)`. The Lean let-chain models the KEYED
`fold_early_stop` as `H.fold` over an association list (`p1bLogsInsert`)
and applies `get_max_key` (`pP1bQuorum`'s `p1bMaxQuorumBallot`) AFTER
the snapshot inside the `mapTick` — semantically equal (a pure function
of the snapshotted keyed state) but not op-for-op: no `into_keyed`/keyed
`fold_early_stop`/`get_max_key` ops exist at stream level. A literal
mirror needs two new `HydroSem` stream ops (keyed `fold_early_stop`,
`get_max_key`) across the 8 interpretations + `HRel` laws + walkers
(est. 150–250 ln engine), then the let-chain reads
`H.keyed_fold_early_stop … |> H.get_max_key |> H.snapshot`. Separate
ratification if ever; this pass kept the let-chain byte-identical.

**Honest notes.** ~65 ln of closure readers precede the def (the
`argmax`/`cqOkProj` decode lemmas the inline proofs cite) — the same
category D65 kept pre-def. The regress ghost `hno_false_full` is ~55 ln
of genuine protocol content (the D21 argument) and sits inline; the
alternative — a colocated named theorem after the def — would need the
module's wires as parameters, i.e. a mirror. PP1b is now the smallest
Paxos module by proof mass per Rust line.

### D67 — dead-mechanism cleanup: two interpretations, the ∃-cover transfer, the standalone phase commands, one dead op, 94 unreachable lemmas, and the docs that still described the square

**The question.** After D56–D66 (tick construct, in-tick semantics,
`den`, construct readers, output-shaped faces, every mirror and lemma
file gone), what superseded machinery was HydroV2 still carrying? User
ruling: "do a pass of cleanup, like identify leftover old mechanisms in
the hydrov2 that we no longer need."

**Method.** (i) A textual census of all 1,870 top-level declarations
in `HydroV2/` with out-of-file consumer counts (dot-notation-aware —
the D57 `MonoTrace.map` lesson); (ii) transitive reachability from the
roots (programs, `…Ensures`, checks, exes, `#guard`/`example` blocks,
attribute-registered `simp`/`den`/`co_*` rules, macros/elabs) — 108
declarations unreachable, 94 after rescuing `#guard`-only consumers;
(iii) a semantic-supersession review per layer (two routes to one
result, interpretations with no consumer, ops with no program and no
Rust counterpart, surface syntax with no users); (iv) a retired-name
grep over code and docs (`EmitDec`, `Square*`, `pcBody`/`leBody`/
`leCore`, `PaxosCoreLemmas`, `hsat`, "emission linearization", …).
Baseline build: 808 jobs, 5m59s.

**Census and dispositions (all user-ratified at Checkpoint #1; B4
rejected — see below).**

| item | disposition | lines |
|---|---|---|
| `Reader.lean` (`ReaderSem`, the pre-D39 correlated-nondeterminism reader) — zero consumers | DELETED | 130 |
| `Rel.lean` (`RelSem`, the ∃-set packaging) — zero consumers (TransferTheory imported it and used nothing) | DELETED | 148 |
| 94 unreachable lemmas/defs: Trace ×22 (`scanAcrossTicksTrace_congr/_map_state/_map_both/_fold_emit`, `fix_induction`, `iterate_val_proj`, `filter_pos_length_of_sum_ge`, zip/prefix helpers…), Couple ×16 (`shiftC`/`botC`/`readEmbed` DUPLICATED in two namespaces, eight `mk_*` projection lemmas, `shift_view_prefix`, `embed2_stream_wf`), CoupleProj ×11 (the same `mk_*` set again, `coerce_*_eq`), Eager/EagerProj ×11 (`pack_*`/`den_*`), Grades ×4, Transfer ×4, `GenExtras`/`registerInfo`/`explicitOnly`, `Types.ble_of_blt`, WfTactics `co_knot_wf`/`co_causal`/`causal_of_eq` | DELETED | ≈ 580 |
| Dead surface: `hydro inline def` flag, `hydro_inline`/`hydro_register`/`hydro_glue` commands (the `leCore` wrapper marking — `leCore` died in D53; the `inlineRegistry` mechanism stays, the `fix` elaborator feeds it) | DELETED | ≈ 60 |
| Root scratch files `helpers_block.txt`, `v1_bucket_block.txt`, `v2_bucket_block.txt` (tracked paste buffers about `foldEarlyStopBallots`, deleted D66) | DELETED | 691 |
| **B1** `CorrSem` — the ∃-cover interpretation (`Transfer.lean` L1036–1416 + `CorrTick` except `Le`, the `StreamRel`/`KeyedRel`/`AtomicStreamRel`/`TickSingRel`/`FoldSingRel` carriers, `corr_snapshot`, `mem_merge*View`, `poolOfList`, `CutLe`…) + `Paxos/TransferCheck.lean` (ruling-4 `replica_covered`/`ballots_covered`) + TransferTheory's hand attainment kit (`snapshot_tight`, `batch_tight`, `deliver_id_attains`, `batch_flat_attains`, `fix_diag_attains`, `flatten_batches_range`, `tickSteps_true`) | SUPERSEDED → DELETED. `paxos_co_cpl` (Couple) is the EXACT coupling with no premise — strictly stronger than the ∃-cover; D65's `collect_quorum_tight` showed tightness comes from the generated `co_*` artifacts, not hand per-op lemmas. `CorrSem` also cost a degenerate row per new op (D60 "machine-side only for tick bodies"). KEPT in `Transfer.lean`: `ListLe`, every per-op coupling lemma, `batchDerive`/`snapDerive`, read agreement (Couple consumes 47 of them). `TransferTheory.lean` is now the `StabilizesAt` kit (316 → 190). | ≈ 650 |
| **B2** the legacy generation surface: `HydroGenToy.lean` (plain `def` toys + hand `H.fix` + hand ensures subtype) and `HydroGenCheck.lean` driving the standalone phase COMMANDS `hydro_couple/hydro_causal/hydro_wf/hydro_mono/hydro_knot/hydro_param` — zero program users (every program is `hydro def`) | SUPERSEDED → the three toys are `hydro def`s (`toy_relay` with `ensures`/`prove`, `toy_step`, `toy_loop` with the `invariant` clause — `toy_loop_inv` folded in); `runPipeline` calls the `TermElabM` cores directly (`runKnotStackT`/`runGlueT`/`runCoupleT`/`runModCausalT`/`runModWfT`/`runModMonoT`/`genParam`) instead of quoting command syntax; the six command elabs DELETED; HydroGenCheck pins every artifact class by `#check` (34 names) and keeps `toy_safe_sched` + the axiom audit | net ≈ −250 |
| **B3** `assume_ordering_batch` + `BatchOrdSelDec` + `Decisions.BatchOrderSelection` + `sched_assume_ordering_batch_unit`/`vmono_…`/`causal_…`/`eag_…_den`, the `batchOrdSelRel` HRel field, the `DecFam.batchOrdSel` dispatch (22 sites across 20 files) | HydroSem FIELD DELETED (ratified per op). Zero program consumers; the pre-D60 TICK-LEVEL in-tick `assume_ordering`, which the corner refused (`wf := False`) and D60 skipped "for now" naming its real successor (an in-tick b-op with `batchOrdSelDerive` + `cpl`). | ≈ 60 |
| **B4** `fold_monotone` + `SingBound.monotonic` (one consumer: `leader_election`'s `receivedFold` for `.max().into_singleton()`) | **KEEP — REJECTED by the user**: "rust does have monotonic singletons." Verified: `hydro_lang/src/live_collections/singleton.rs:39–45` — `SingletonBound` "includes an additional variant `Monotonic`, which means that the value will only grow", reached via `ApplyMonotoneStream<Proved, B::StreamToMonotone>` (`properties/mod.rs:532–541`) from a fold whose combiner carries a `monotone(…)` proof. `.monotonic` is a faithful Rust-grade mirror; D60 retired the TICK-level monotonic grade only. `Sem.lean`'s `SingBound` doc already says so. | 0 |
| KEEP-DOCUMENTED: corollary theorems with no consumer (`acceptor_p1_ok_pins`, `acceptor_p2_ok_max`, `two_pc_once` — V1-parity faces of the contracts); `Liveness.lean`'s WF(deliver) kit (`FairCursor`, `deliver_fair_attains`, `ticks_fair_attains`, `cumMax_mono_le`, three sanity lemmas — unreachable today, reserved by LIVENESS.md for rung (a)); LivenessChain's temporal vocabulary (`ChAlways`, `.leadsTo`, `.alwEventually`); `Trace.fix_stabilizes`/`iterate_stab_of_fixed` (the staged 6b stabilization-search semantics, README "Staged work"); b-ops with a Rust counterpart but no program consumer (`bflatMapOrdered`, `bfirst`, `bsPure`, `boFilter`, `boIsSome`); `TransferChecks.lean` (the SCHED_AUDIT ruling-1 `#guard` suite), `CoupleCheck` (D39 blueprint), `HydroParamCheck`, `HydroTickCheck`, `EagerCheck`, `Exploration`, `Falsification`, `AxCheck` | — | — |

**Docs (D).** `CORRESPONDENCE.md`: the architecture picture is five
instances (`Values`, `SchedSem`, `Eager`, `MonoRel`, `CoupleSem`) with
one coupling instance; "How the proof is layered" no longer describes
`CorrSem`/`StreamRel`; the Rosetta table's simulation rows point at the
corner; the K4 rows point at `paxos_core`'s `slot_functional` leg /
the `a_log` invariant clause (not `PaxosCoreLemmas.lean`); Walkthrough
C's comments name the `hydro def` phases, not commands; the audit
checklist's `emitLin` row became "unordered emissions take no
decision". `README.md`: `Rel.lean`/`RelSem`/`CorrSem`/`TransferCheck`
rows and the dead attainment-kit names removed; `Transfer`/
`TransferTheory` described as the corner's per-op content and the
stabilization kit; layout table updated. `SCHED_AUDIT.md`: F1 headed
as a CLOSED historical record (every file it names is deleted);
`assume_ordering_batch` row updated. Lean module docs: every
`Square*.lean`/`SquareHsat`/`SquareProj` mention, every `pcBody`/
`leBody`/`leCore`/`LEWires` mention, every "emission linearization"
(Sem/HydroDef/Liveness/TwoPC/HydroGenToy), and every description of
`hydro_couple`/… as user commands (HydroGen/HydroGenKnot/HydroParam/
HydroDef/CoupleSafety/HydroParamCheck headers) rewritten to the phase
vocabulary. `Sem.lean`'s interpretation list and the decision-kind
table no longer mention `Reader`/`RelSem`/`CorrSem`/`BatchOrdSelDec`.

**Outcome.** 41 files; −3,038/+391 lines (HydroV2 Lean: 32,453 →
30,517; three files deleted: `Reader.lean`, `Rel.lean`,
`Paxos/TransferCheck.lean`; `Transfer.lean` 1,810 → 1,188;
`TransferTheory.lean` 316 → 190; `Trace.lean` 2,125 → 1,906;
`HydroGenCheck.lean` 234 → 98). `HydroSem` lost one op and one
decision family; every `HRel.Laws` instance lost the matching law
(`hydro_rel_laws` regenerated the bundle). Build: 808 → 805 jobs,
5m59s → 5m37s (LeaderElection 254 s, SequencePayload 35 s, PaxosCore
27 s, HydroTickCheck 16 s; the interpretation rows and the
phase-command elaborations were cheap — the win is maintenance, not
time). Gate: zero sorries, 48 axiom reports all standard-three,
falsify/explore/v2paxos/v2twopc 4/4, all 11 `#nondet_census` checks
unchanged, two `maxHeartbeats` sites; headlines byte-identical.

**Surprises — things that looked dead and weren't (why the compiler,
not the census, is the oracle).** (a) `Grades.maxStep_comm` is
consumed only by an ANONYMOUS `instance : RightCommutative maxStep` —
the textual census does not key anonymous instances, so its reach
walk never entered the instance body. (b) `StabilizesAt.inc_nil` is
consumed via dot notation inside `StabilizesAt.deliver_id`, whose
only consumer is an `example` in `TransferChecks.lean` — the
`#guard`/`example` rescue pass was non-transitive. (c) `lastLen` is
mentioned only in the STATEMENT of the live `batchesFrom_append`.
(d) The `hydro_glue`/`hydro_couple`/… command SYNTAX was load-bearing
in a way grep could not see: `runPipeline` built the phase invocations
by quotation (`` `(hydro_glue $M $ids) ``) — deleting one command broke
parsing of the pipeline itself. The fix (cores called directly) is
what made the surface removable at all. Each was caught by the next
build; none reached the gate.

**Gotchas for the canon.** (viii) A `tick` block's `_at` reader states
its input reads against the block's INPUT ALIASES (`(bb i)[n]?` for
`(input bb := b)`), so reading one at the denotation is
`simp only [bb, b, bumped, den]` — the alias, the wire, its upstream
lets, then `den` — not `simp only [den]` alone (which makes no
progress on an alias). (ix) Single-leg modules get `M_param` (no
subscript); multi-leg get `M_param₁…`. (x) When deleting a
`HydroSem` field, the three `HRel.Laws` instances are anonymous
constructors ordered by field — the matching `by …_law_tac, -- op`
line must go too, or every later law shifts by one and the error
lands on an unrelated op.

**Pending ruling (NOT edited this pass, per the user).** Root-level
docs outside `HydroV2/`: `DESIGN.md` still says "no external
dependencies (no Mathlib)"; `SORRIES.md`'s status note is from the V1
decisions-as-inputs wave; `docs/00–11` are the V1 set (already headed
"Historical note"). Options: refresh `DESIGN.md`/`SORRIES.md` to the
V2 state, or fold both into `README.md` + `HydroV2/README.md` and
delete.

**Queued.** The in-tick `assume_ordering` b-op (`bassumeOrdering` with
`batchOrdSelDerive` + `cpl`) if a Rust site ever needs it (D60 note);
the keyed-op fidelity item (D66); liveness rung 2 (Paxos-scale
quiescent completeness; the generic tightness kit that generalizes
`collect_quorum_tight`).

### D68 — the V1 tree is deleted, `HydroV2` is `Hydro`, and the docs are rewritten from the ground up (reference set + guided tour)

**The ruling.** "Delete flo + gyatso, since their semantic ideas are
subsumed into hydrov2, and fully rewrite the docs" — plus, at
Checkpoint #1: delete the whole `HydroLean/` tree (Collections and
Prelude were orphaned by the Flo/Gyatso deletion — HydroV2 imported
nothing from them), rename the library `HydroV2` → `Hydro` now so the
docs are written against final names, make `HydroV2/AxCheck.lean` the
sole axiom oracle, keep "Flo monotonicity" as the term, and add a NEW
`docs/` that "walks through properly" — a narrated tour for someone who
knows Hydro and some Lean but has never seen this tree.

**Import graph (Checkpoint #1a).** `HydroV2/`, `HydroV2.lean` and the
four exes imported nothing from `HydroLean/` (zero hits). Inside the
tree: `Prelude` (ARS vocabulary) → `Collections` (798 ln: DupList /
Multiset / Keyed / Instances) → `Flo` (1,698) → `Gyatso` (2,433);
Collections' only out-of-directory consumer was `Gyatso/LocalColl`.
So the whole tree (5,165 ln, 20 files) goes together.

**Subsumption audit (#1c), honest.**

| V1 idea | home now | verdict |
|---|---|---|
| graded collections seq/mset/dup (Figs 2.13/3.10/3.8) | `Grades.StrOrd × Retries` carriers; `BoundedStream` families | subsumed |
| overwrite singletons (§4.3.3) | `Atomic`/`BoundedSingleton`/`BoundedOptional` | subsumed |
| Lemma 2.4.3 determinism + eager execution (unique stuck state) | `Values` is a function of inputs+decisions; machine = denotation is the corner (`paxos_co_cpl`, exact, premise-free), generated per `hydro def` (`co_wf`) | subsumed per program (generated), not as one "for all lawful graphs" metatheorem |
| Lemma 2.4.4 streaming progress | `Atomic`/`BoundedStream` types — per-tick completeness is a typing invariant | subsumed by construction |
| Thm 3.4.2 monotone outputs under crash-stop | `MonoRel`/`MonoHRel`, generated `<M>_mono`/`stages_mono`; crash-stop = pacing `false` forever (D35) | subsumed |
| Thm 3.4.1 eventual determinism | corner exactness + chain liveness | subsumed |
| cluster upgrade lawfulness (Fig 3.5/3.6) | clusters are `Fin (mem ℓ)` per-member traces by definition | dissolved into the model |
| network ops + `foldCommutative` (Figs 3.7/3.10/3.11) | `TransportDec` cursors + `flattenUnordered` quotient + `FoldOk` | subsumed |
| **LVars / lattice collections** (`lvarColl`, Fig 2.16) | — | not represented; Hydro Rust has no LVar collection either — no fidelity gap |
| **the ARS small-step framework** (Prelude: confluence/SN) | — | not represented; it was Flo's proof *technique*, not a semantic idea Hydro programs need |

**What landed.**
- Deleted: `HydroLean/` (20 files), `HydroLean.lean`, `DESIGN.md`
  (V1 design, "no Mathlib"), `SORRIES.md` (V1 status), `docs/00–11,
  98, 99` (the V1 set, 14 files), `ACCEPTANCE.md` (→ `GATE.md`),
  `HydroV2/README.md` (→ `ARCHITECTURE.md`).
- Renamed: `HydroV2/` → `Hydro/`, `HydroV2.lean` → `Hydro.lean`,
  namespace/imports `HydroV2` → `Hydro` (sed over the Lean/toml/sh
  tree; FINDINGS untouched — path note at the top), exes
  `v2paxos`/`v2twopc` → `paxos`/`twopc` (roots `PaxosDemo.lean`/
  `TwoPCDemo.lean`), lakefile `defaultTargets = ["Hydro"]`, the
  `HydroLean` lib block removed. Lean doc comments: "V1"/"V2"
  mentions retired (the names mean nothing without the other tree);
  `HydroDef.lean`'s header no longer claims the phase commands exist
  (D67 deleted them).
- Gate: `Hydro/AxCheck.lean` is the sole axiom oracle; sorry-grep over
  `Hydro/ *.lean`; `GATE.md` is the executable definition.
- **Reference docs (8, at `hydro_lean/`)**: `README.md` (what, build,
  how to read a module, headline table), `ARCHITECTURE.md` (grades →
  signature → interpretations → relational layer → surface →
  generation pipeline → corner → module map with Rust anchors and
  censuses → perf envelope), `DOCTRINE.md` (P1–P3, R1–R9, the E10
  layout rule, process, the Verus translation table, "Flo
  monotonicity" defined once), `CORRESPONDENCE.md` (rewritten
  1,059 → 261 ln: claim, Rosetta, layering, two walkthroughs,
  `batchDerive`, commutative folds, not-proven, trust base, the Sched
  audit + 13-row checklist, the D37 cautionary section), `SCHED_AUDIT.md`
  (trimmed 526 → 236: ground truth incl. the D58/D67 facts, axis table,
  F1 closed/F2–F6 live, S4 ledger 1–16, the upstream report list),
  `LIVENESS.md` (rewritten 241 → 127: claim, Rosetta, V∘K1∘K2, the two
  scenarios, the ladder, non-vacuity; `EmitDec` fork text gone),
  `GATE.md` (eight steps + governance), `ENGINE_NOTES.md` (the D37–D67
  gotchas canon consolidated, grouped by mechanism, each with its
  D-number). Every claim names a declaration or a `file:line`; history
  is cited by D-number, not re-narrated.
- **The guided tour (`docs/`, 10 files, ~1,180 ln)**: `00-index`,
  `01-from-rust-to-lean` (`collect_quorum` beside `quorum.rs:89–160`,
  the Rust→Lean table), `02-what-a-program-denotes` (`Values`
  carriers, decisions as inputs, `batchCuts` legality, `den`,
  `paxos_eager_den`), `03-contracts` (`CQEnsures`, `two_pc` composing
  by contract, consumer-shaped faces, requires-as-hypotheses),
  `04-ghosts-and-proofs` (the construct's readers, the loop invariant
  and its obligations, ghosts, the `prove` block, `OnceInv` end to end,
  what makes proofs small), `05-knots` (`fix … complete`, Bekić, the
  generated chain, K4 read line by line, the prove leg), `06-the-
  machine-and-the-corner` (`SchedSem`, the headline's quantifiers, the
  four moves, what the corner is not, in-tick coupling),
  `07-liveness-chains` (`cq_live` factored), `08-adding-a-module` (the
  checklist), `09-extending-the-engine` (when an op is justified, the
  `bmax` cost table, extending a construct, generators, measuring).
  Every excerpt is copied from the tree with a `file:line` anchor.

**Numbers.** Build 805 → 783 jobs (the `HydroLean` lib was ~20 jobs);
wall time unchanged within noise (the V1 files were cheap); tree
−5,165 ln of Lean, −2,100 ln of V1 docs, −1,000 ln of accreted docs;
+~2,575 ln of new docs. Gate at close: see the thread's closing report.

**Not done / pending.** The `docs/` tour cites line numbers as of D68 —
they drift; names are the anchors. The keyed-op fidelity item (D66), the
decision-record validity diagnostic (D65), liveness rung 2 and the
upstream reports remain queued (`SCHED_AUDIT.md` carries the report
list; `LIVENESS.md` the ladder).

### D69 — `two_pc` to the doctrine, and `two_pc_safe_sched'`/`two_pc_once_sched'`

**What was wrong.** `two_pc` was the one module left in the pre-D64
shape: a *plain* `def … ensures` (no generation pipeline, hence no
`_mono`/`_param`/`co_*` and no machine-run headline); a face
`TPCEnsures` stated against the phase-1 wire `voteYes` as an ∃-witness
("PP1b's `okPool` pattern", the shape D66 removed from `PP1bEnsures`),
against hand "canonical pool" mirrors `tpcPrepPool`/`tpcVotesPool`/
`tpcAckPool` of the echo wires, and with the decision cuts `dvotes dacks`
in its signature — four fields that were `CQEnsures` re-exported in
internal vocabulary; and the actual guarantees (`two_pc_unanimous`,
`two_pc_once`, `two_pc_commit_iff`) as consumer theorems *after* the
def, re-derived through ~180 ln of `tpc_*` pool algebra, all at `Fin 1`.
Census of the 490-line file: header 36 · Dec/Sched 30 · mirrors 21 ·
face 35 · program 47 · pool algebra 180 · consumer theorems 103 · no
`#guard` (the exe was the only witness).

**Fidelity findings against `two_pc.rs` (user-ruled).**
- F1 `coordinator: &Process<Coordinator>` — one member. The port's
  `coord : L` has arbitrary `mem coord`; the faithful premise
  `mem coord = 1` is now the hypothesis of the at-most-once face. It is
  needed: with two coordinators every coordinator receives every vote,
  so both vote-yes the same payload, the ack pool holds `2n` copies, and
  the quorum stage's `max` purge re-emits (the D65 quorum.rs note) —
  at-most-once is *false* for `nC ≥ 2`; soundness and unanimity hold for
  any `nC`.
- F2 `payloads: Stream<…, NoOrder>` — the binder was `.totalOrder`;
  now `.noOrder .exactlyOnce` (the `Values` input is `Fin nC → Multiset P`).
- F3 the vote hook: Rust tags every echo `Ok(())` at the coordinator
  (`c_votes.map(|kv| (kv, Ok(())))`, two_pc.rs:41 — participants never
  vote in the Rust); the port tags at the participant with `vote j p`
  (recovered at `vote = fun _ _ => true`). Kept — it is what makes
  unanimity non-vacuous — with the comment now naming the Rust line it
  replaces. The one line of the program that is not the Rust's.

**The face (R3).** `TPCEnsures n vote payloads out`, over the output
`c_all_commit` and the inputs only; vocabulary `tpcProposed payloads`
(the pooled input): `commit_proposed` (a commit was proposed — new,
free), `commit_unanimous` (a once-proposed commit was voted yes by every
participant), `commit_once` (`nC = 1 →` a payload proposed at most once
commits at most once). No witness, no mirror, no decision in the
signature. **`two_pc_commit_iff`'s ⇐ direction is not a face**: the
premise "complete decisions" is relative to the pool each decision
consumes (legality truncates), and the ack stage's pool is the phase-1
wire — internal — so no output-shaped statement exists. It is `two_pc`'s
liveness rung: fairness ⟹ both stages exhaust ⟹ the `Values` crossing at
the exhausted point, pool-free (`LIVENESS.md`, ladder (a): the first
multi-stage client of the chain layer). The executable witness of the
iff (10 commits once, the vetoed 20 never) stays in `lake exe twopc` and
a new `#guard` at `Values`.

**The proofs, inline (R1/R2/R4).** Three `rfl` wire reads under `den`
(`hprep : p_prepare j = tpcProposed payloads`; `hvotes`/`hacks`: each
coordinator's pool is the participants' tagged echoes of what they
received — `values_values_noOrder` ∘ `values_broadcast_closed` ∘
`values_map`), four counting ghosts about those wires (`hok_votes`: a
once-proposed payload's `Ok` votes = its yes-voters; `hkey_votes`:
`n · count`; `hok_acks`/`hkey_acks`: `n ·` the vote-yes multiplicity),
`hcommit_src` (a commit needs an ack, so some coordinator vote-yes'd it)
and `hvy_votes` (a vote-yes needs `n` yes-votes among the consumed pool,
which sits below the whole pool); each face is ≤ 20 lines composing
them with `hcq1`/`hcq2`'s `emit_sound`/`emit_count`. Generic count
algebra moved to its vocabulary: `cqOkCount_listSum`/`cqKeyCount_listSum`
(a fan-in sum) and `cqOkCount_map_tag`/`cqKeyCount_map_tag` (a pool
tagged by a response function) in `Std/Quorum.lean`'s prerequisites;
`Multiset.count_listSum`, `List.sum_map_indicator`,
`List.sum_replicate_nat`, `List.sum_le_length_of_le_one`,
`List.exists_pos_of_sum_pos` in `Trace.lean`'s counting toolkit.

**`hydro def` (R4).** Two module calls, four `values ∘ broadcast_closed`
edges, two `map`s, no tick, no fix: the pipeline ran first try —
`hydro_couple` 1 sr / 1 rr (+ `two_pc_vdec`), `hydro_causal` 1,
`hydro_wf` 1, `hydro_mono` 1, `hydro_param` 1; `two_pc` itself is
axiom-free. No engine gap. Census 2/4/0 unchanged. The file builds in
3.2 s. `two_pc` is now the plain program (no `.val`); `TwoPCDemo.lean`
adjusted (and its input is a multiset per F2).

**`two_pc_safe_sched'` / `two_pc_once_sched'`** (TwoPC.lean, the
`cq_safe_sched'` pattern, nothing new in CoupleWf/CoupleSafety):
`twoPCCoDec sdec := ⟨CoDec.batch _, CoDec.batch _⟩`, `twoPCCoSched` =
`CoDec.cursors` at the four transports, `twoPCVDec` = the generated
`two_pc_vdec` at those cursors; `two_pc_co_sr`/`two_pc_co_rr` = the
generated `two_pc_co_sr₁`/`_co_rr₁` (the corner's legs *are* the
`SchedSem` run and the `Values` run at the derived decisions);
`two_pc_co_cpl` = `.cpl (two_pc_co_wf₁ … trivial)` (premise-free); then
the face at the derived decisions. Statements: for the `SchedSem` run of
`two_pc` under any pacing, any schedule, any horizon `T`, with the
machine input `cpS` coupled below a `Values` pool `cpV`
(`ListLe .noOrder .exactlyOnce ((cpS i).view t) (cpV i)`): a payload in
`(… c).view T` with `(tpcProposed cpV).count p = 1` was voted yes by
every participant; and with `mem coord = 1` and `count p ≤ 1`, the view's
count of `p` is `≤ 1`. No satisfiability premise, no decision witness.
Both standard-three in `AxCheck.lean` (+ `two_pc_co_cpl`, `two_pc`).

**Numbers.** TwoPC.lean 490 → 440 (header 50 · prerequisites 60 · def
@113 with ~110 ln of ghosts/prove · census · `#guard` 20 · headline
section 130); the pure-layer-after-the-program is gone (0 ln). Full
build: see the gate below. Docs: README headline row, ARCHITECTURE
module row + headline paragraph, CORRESPONDENCE Walkthrough A (+1
paragraph), LIVENESS ladder (a) (+ the two_pc client), docs/03 § "A
consumer: `two_pc`" rewritten to the new faces.

**Observed, not touched.** The generator's `simp only [M, HydroSem.fix,
HydroSem.fixTick]` warns "unused simp argument" on every `hydro def` (92
hits tree-wide) — a trivial engine nit for the next engine pass.

**Queue after D69.** `two_pc` liveness (completeness ⟸, with rung (a));
the keyed-op fidelity item (D66); the decision-record validity
diagnostic (D65); liveness rung 2; the upstream reports
(`SCHED_AUDIT.md`).

### D70 — the keyed vocabulary: `KeyedStream`/`KeyedSingleton` as first-class graded families, `Ticked` is `Atomic`, the shaped-former path tables become one simproc

**What was wrong.** Three fidelity gaps and one engine smell. (i) The
only keyed carrier was the network's `KeyedStream p c α` (a pool per
(receiver, sender) pair), with `values` special-cased on it; Rust's
`KeyedStream<K, V, …>` — the output of `.into_keyed()`, `.broadcast()`,
`.demux()` alike — had no general counterpart, so `collect_quorum`
mirrored `.into_keyed().fold(…)` as an ad-hoc in-tick `bkeyedFold`
whose closure took the whole pair (Rust's takes the value), and
paxos.rs:548–560's `into_keyed().assume_ordering().fold_early_stop()
.get_max_key().snapshot()` was modeled as `H.fold` over an assoc list
with `get_max_key` after the snapshot (D66's queued item). (ii)
`reduce_watermark` (the acceptor log, paxos.rs:851) was mirrored as a
persisted-pool fold with view-time max — equal in output, not the
operator. (iii) `Ticked`/`TickStream`/`TickKSing` named Rust's
`Atomic<ℓ>` by a Lean-only word. (iv) `EagTickScanPaths` and its corner
twin hardcoded one rewrite rule per (output path, leaf kind, leg) for
right-nested outputs of up to four legs (`_0,_1,_2,_21,_22,_221,_222`
× sing/stream × den|sr|rr|wf — 14 + 42 theorems), and knew no `.ksing`
leaf; the user's question "do you generate one for every arity? that
seems bad" was the right one.

**Rulings (user, in-thread).** R5: the transport returns the GENERAL
`KeyedStream ℓ K V`, `values` is the general keyed op, the old carrier
is deleted. R6 (refined twice): a machine singleton is a materialized
VALUE per step (`Fin (mem ℓ) → Nat → σ`, recomputed to the latest
cursor), not a recompute record and not a version list; its reads are
read-agreement packages (`CoSing.cpl`). (a) `Ticked` IS `Atomic<ℓ>` —
renamed throughout (`Atomic`, `AtomicStream`, `AtomicKeyedSingleton`,
`AtomicOf`, the `HRelC` fields, `CarrierFam.atomic*`); no separate
atomic families, since `all_ticks_atomic`/`batched_atomic` are casts
and identities on the tick-located carrier. (b) Rust's `across_ticks`
has NO Lean counterpart: "if we already have an `AtomicStream` in Lean
we don't need `across_ticks`, we should be able to directly invoke an
operator on it" — a derived `across_ticks x body := body x` was written
and deleted the same session; the Rust quote above the line carries the
`across_ticks(|s| …)` text, the Lean line is `H.reduce_watermark
(H.tinto_keyed x) …`. R3 (migrating Recommit/AP2's combiners — honest
refinement vs literal closure + conditional `FoldOk`) is PENDING; the
PP1b and AP2 migrations onto the new operators wait for it.

**The vocabulary (`Sem.lean`).** Families: `KeyedStream ℓ K V ord
ret`, `KeyedSingleton ℓ α K V ord ret kb` (`KSBound | unbounded |
boundedValue`; `α` is the keyed SOURCE pair type the singleton is read
off, `ord` its cut vocabulary), `BoundedKeyedStream K V ord ret`,
`BoundedKeyedSingleton K V`, defaults `AtomicKeyedStream`/
`AtomicKeyedSingleton := Atomic ℓ (Bounded…)`; `TickShape.ksing K V`
with `AtomicOf`/`BoundedOf`/`SeedOf` cases. Ops: `broadcast_closed`/
`demux : … → KeyedStream p (Fin (mem c)) α`, `values : KeyedStream ℓ K
V → Stream ℓ V .noOrder ret`, `into_keyed`, `kassume_ordering
(OrderSelDec (mem ℓ) (K × V))`, `kfold_early_stop : KeyedStream ℓ K V
.totalOrder .eo → (A → V → A × Bool) → A → KeyedSingleton … .boundedValue`,
`kget_max_key : [LinearOrder K] → KeyedSingleton … .boundedValue →
Singleton ℓ α (Option (K × V)) .noOrder ret .unbounded`, `tinto_keyed`,
`reduce_watermark : AtomicKeyedStream ℓ K V ord .eo → Atomic ℓ (Option
K) → (f : V → V → V) → FoldOk ord .eo (reduceStep f) → AtomicKeyedSingleton`
(restricted to exactly-once — the library-internal `assume_retries` is
the combiner's obligation), in-tick `binto_keyed`, `bkfold (FoldOk)`,
`bksFilter`, `bksMap`, `bksKeys`, `bksEntries`. Deleted: `bkeyedFold`,
`bkeys`, the transport `KeyedStream p c`, `EKeyed`, `KAgree`.

**The denotation (`Keyed.lean`, new).** `KeyedPool K V ord ret` = a
finite key set with per-key graded content (`PoolCarrier V ord ret`),
`dom : k ∉ keys → grp k = ⊥` — grouped at EVERY grade (anti-cheating:
no cross-key order exists at `.totalOrder` either); `ofList`/
`ofMultiset`/`ofPool` (into_keyed by grade), `transport` (`Finset.univ`
keyed by sender — `values (broadcast_closed d s) i` is rfl-equal to the
D69 sum), `values` (`.eo`: `sumMS`; `.alo`: `RetryPool.mk sumMS`),
`Le` per key with the growth lemmas. `KeyedMap K V` = `keys : Finset K`,
`get : K → Option V`, `dom : (get k).isSome ↔ k ∈ keys`, with
`entries`/`keysMS`/`filter`/`map`/`maxEntry` (`Finset.max`)/`ofFold`/
`ofReduce`/`gcBelow`/`ofFoldEarlyStop`; `reduceWatermarkStep` =
accumulate the pool, running-max watermark, `gcBelow w (ofReduce f ok
acc)` — hydro-project/hydro#3189's whole-map form read every tick
(decreases ignored, entries below removed, arrivals below dropped).
Machine side: `SchedKMap K V := {l : List (K × V) // (l.map fst).Nodup}`
— a flat assoc list WITH the `HashMap` unique-keys invariant (ratified:
it is how DFIR's keyed state works, and sched follows DFIR), with
`keyedFoldAssoc`/`keyedFoldEarlyStopAssoc`/`keyedReduceAssoc` and the
`ofAssoc_*` agreement lemmas. **The machine's `reduce_watermark` is
DFIR's `ReduceKeyedWatermark` literally** (`compile/ir/mod.rs:5221–5310`,
read at the user's "in sched mode it will be eager right?" — the first
cut was accumulate-and-GC-at-read on lists, output-equal but not the
algorithm): `reduceWatermarkEager` = a `fold::<'static>` over
`(SchedKMap, Option K)`, this tick's payloads first (`chain` input
`[0]`; a payload at a key below the current watermark is dropped, else
`insertReduce` = `map.entry(k)` Vacant-insert / Occupied-reduce), then
this tick's watermark (`[1]`; `≤ current` ignored, else retain `k ≥ wm`
and set). `scan_eager_eq_assoc` (via `EagerSim`: watermarks agree and
the map, by key, is the GC'd reduce of all arrivals so far) proves the
eager fold's output trace IS the accumulate form's, so the coupling to
the denotation's step is the composition — the output-equality claim is
now a theorem, not an assertion. Reads of a `.boundedValue` keyed
singleton are increment cuts of the SOURCE pairs (`CutDec α .noOrder`)
with per-key-prefix legality (`legalView`) — sound both for keyed
streams from `kassume_ordering` (a free flat selection) and from
transport (grouped per sender: no canonical interleave is ever
exposed); hence `kget_max_key` yields a `.noOrder`-cut `Singleton`.

**Through the stack.** `Values`/`Sched`/`Eager`/`MonoRel`/`SchedCausal`
(`FAgree h x y := ∀ i t, t ≤ h → x i t = y i t` over the materialized
singleton), `Transfer` (`KeyedListLe`, `corr_transport`, `corr_values`,
`kviews_cutsMS`), `Couple` (`CoSing` as a read-agreement package with
`src`; `CoKStream`/`CoKSing`/`CoBKStream`/`CoBKSing`/`CoAtomicKStream`/
`CoAtomicKSing`; `CoKSing.maxEntry_cpl`; every CoupleSem field, zero
sorry), `CoupleProj`/`MonoHRel`/`CausalHRel`/`EagerRel`/`HydroRel`
(`keyedRel`, `ksingRel`, `bkeyedRel`, `bksingRel`, `atomicKSingRel`,
`atomicKeyedRel`), `HydroGen` (`CarrierFam` with `keyed|ksing|atomic*|
bkeyed|bksing`; outer binder `x_src` for `.sing` wire args pins
`.src`), `HydroTick` (`AtomicKeyedSingleton` input ↦ `.ksing`),
`WfTactics` (`shapeOfCarrierType` for the keyed families; the
`AtomicOf`/`BoundedOf` shape index moved to `args[5]`/`args[3]` — the
first full build found it). Headline hypotheses: `paxos_safe_sched'`/
`paxosVDecG` gained `(ckSrc : Sched.Stream acc …)` and `hck` restated as
`∀ j t, ckS j t = ((ckSrc j).view t).foldl g init` (the materialized
singleton's provenance) — meaning-identical.

**The path tables → one simproc each.** `eagTickScanProj` (EagerProj)
and `coTickScanProj` (CoupleProj): on a leaf projection (`EPack.den`;
`CoTickSing/CoAtomicStream/CoAtomicKSing.sr|rr|wf`, as projection app
or raw `.proj`) whose subject is a `Prod.fst/snd` chain rooted at
`tick_scan` at the interpretation, emit the same chain over the other
interpretation's former (`Values.tick_scan … (EagerTick.den ins x)`;
`SchedSem.tick_scan … (srLegs ins x) (gS …)`/`Values.tick_scan …
(rrLegs ins x) (gR …)`/`scanWf …`) as a proof-free definitional step —
`pack`/`CoTick.mk` are shape-recursive, so it IS `rfl` for every path
and leaf kind. 56 theorems deleted; arbitrary depth and `.ksing` leaves
for free. `simpTerm` (HydroGenKnot) routes simproc names into a
`Simprocs` set. Facade-typed `.ksing` leaf rules (`srLegs_ksing` …
`embed2_ksing_wf`, `coerce_ksing`, `allEO_ksing`) join the arrays.

**Programs.** `collect_quorum`/`cqwr`: `bkfold cqCount (0,0) comm
(binto_keyed current_responses)`, `bfilterMap (bksEntries
count_per_key) …`, `bksKeys (bksFilter count_per_key (fun (success,
error) => …))` — Rust's value-only closures, literally; the decode
lemmas restated on `KeyedMap.ofFold … (KeyedPool.ofMultiset pool)`
(`cqCounter_get`, `keyedFold_cqCount`, `cq_reached_eq`,
`cq_keys_filter_eq`); headlines unchanged. `p_leader_heartbeat` reads
`values ∘ broadcast_closed` at the stutter grade through
`values_broadcast_mem_stutter`. Every other `values ∘ broadcast_closed`
decode goes through `KeyedPool.values_transport` (composite rules
never fire under inner-first rewriting — ENGINE_NOTES).

**Toys as the oracle.** `toy_kmax` (the p1b chain, `ToyKDec` record)
and `toy_wm` (`reduce_watermark (tinto_keyed (batch …)) wm` as a
`.ksing` tick input, `bksEntries` in the body; the `#guard` checks
#3189's semantics across two ticks) run every walk. They found two
engine gaps the programs could not: `coCausalDispatch` (WfTactics)
lacked the new ops (the SchedCausal table had them), and a `W3` closer
keyed on `coerce` is dead once `coerce_ksing` is in the set. Keyed
toys in HydroTickCheck migrated to the new API.

**Gate.** Full build (825 jobs); zero sorries; axioms standard;
`falsify` ALL PASSED; `explore` true/true; `paxos` OK; `twopc` OK; 11
`#nondet_census` + the two toys'; two `maxHeartbeats` sites unchanged.

**Deviations to ratify.** (1) `SchedKMap` is a nodup assoc list, not a
hash map — RATIFIED (user: "it's how things work in DFIR; generally,
sched should follow what DFIR does"). (2) `kget_max_key` yields a
`.noOrder`-cut `Singleton` (the `ord` index on `Singleton` is the read
vocabulary, a Lean device, not a Rust grade). (3) The
headline-hypothesis shape change (`ckSrc`). (4) The denotation's
`reduce_watermark` accumulates and GCs at read; the machine's is DFIR's
eager fold, proven output-equal. (5) `Atomic` rename.

**Queue after D70.** R3 ruling → PP1b (`into_keyed → kassume_ordering →
kfold_early_stop → kget_max_key → snapshot`; `dec.snap` becomes
`.noOrder` cuts) and AP2 (`reduce_watermark (tinto_keyed …)
a_checkpoint`; K4/`log_covers_mono` restated — mini-checkpoint first)
migrations; the two items below (the `.boundedValue` keyed-singleton
batch op; `SchedSem` against production DFIR); then the D69 queue.

**Queued (user, during D70): the simulator-soundness question.** A
Rust simulator bug was found while D70 was in flight: batching a
`BoundedValue` `KeyedSingleton` into a tick dispatched to the
*unbounded* hook, which has snapshot logic (every key's current value,
every tick), where production batches (each finalized entry delivered
once). Would Lean have caught it? Three layers, three answers:

- *Types*: `KSBound.boundedValue`/`.unbounded` is an index on
  `KeyedSingleton` since D70, so the two hooks are different ops with
  different signatures (`.boundedValue → AtomicKeyedStream` vs
  `.unbounded → AtomicKeyedSingleton`); the conflation is untypable in
  Lean. Cheap — but Lean models OUR machine (`SchedSem`), not the Rust
  simulator, so this alone catches nothing in Rust.
- *The coupling obligation* is the real net: the denotation of the
  `.boundedValue` batch is an exactly-once stream of entries across
  ticks; a snapshot-shaped machine re-delivers every key every tick,
  so the corner's `cpl` (prefix / exactly-once trace coupling,
  `KBatchTraceLe`) is unprovable. The gate would have refused it at
  the proof, not at a test — IF the op were modeled.
- *The op is not modeled yet*: D70 has `kfold_early_stop →
  kget_max_key → snapshot` (paxos.rs's path) and `reduce_watermark`
  as a tick input, no "batch a `.boundedValue` keyed singleton" op.
  And nothing checks the Rust simulator against `SchedSem`.

Two items:

- **The `.boundedValue` keyed-singleton batch op.** Add the `.boundedValue` keyed-singleton `batch` as
  its own op (machine: an entry is delivered in the tick its fold
  finalizes, once; denotation: exactly-once stream of entries), with
  its Couple proof. Re-derive the simulator bug as a REFUSED corner
  instance (the snapshot-shaped machine fails `cpl`) — a negative
  witness, in the Falsify spirit.
- **`SchedSem` against production DFIR** (ruled by the user after D71:
  the comparison target is the *production* DFIR runtime, NOT the
  simulator — the simulator is architecturally closer to `Values`, a
  denotation-level replay of nondeterministic choices, so checking
  `SchedSem` against it would compare two models of the wrong kind).
  Think through a conformance harness: run a compiled DFIR program with
  recorded network delivery/tick pacing, replay those choices as a
  `…Sched` decision record through `SchedSem`, and compare per-tick
  outputs — `SchedSem` as the specification of what DFIR's step machine
  computes. The simulator bug above is then caught indirectly: the
  simulator should agree with the denotation (`Values`), DFIR with
  `SchedSem`, and the two are tied by the corner's coupling proof.

**R3 RULED (user, closing D70): conditional commutativity, the Rust
closure verbatim.** Today's `FoldOk` (a global commutativity proof) is
replaced on the fold family (`fold`, `bfold`, `bkfold`,
`reduce_watermark`) by three things: `Pre : V → V → Prop` (the
prerequisite on pairs — for the two `manual_proof!(TODO)` combiners,
`x.ballot = y.ballot → x.value = y.value`), `hcomm : ∀ s x y, Pre x y →
g (g s x) y = g (g s y) x` (interpretation-independent, passed to the
operator; the literal closure is provably commutative under `Pre`), and
the content fact "any two values that meet at a key satisfy `Pre`",
stated over the input's DENOTATION and carried by the corner's `wf`
(it cannot be an operator argument: a program against an abstract `H`
cannot prove content). Denotation: a total order-free per-key reduce
(`Multiset.foldlOn` via `List.Perm.foldl_eq'` — permutation-invariance
needs commutativity only on the list's own pairs — with `⊥` when the
pairwise fact fails: the semantics of an unjustified `manual_proof!`,
`logView`'s degrade rule derived). Machine: the literal closure in
arrival order (DFIR). Transfer: `cpl : wf → …` — the machine's realized
content is a sub-multiset of the denotation's, pairwise facts are
hereditary, so the arrival-order fold IS the order-free one; proven
once in the `CoupleSem` field. The content `wf` residual threads out as
a module obligation (new `prove wf` leg in HydroGen) and is discharged
where the justification lives — for AP2 at `paxos_core` from SP's
slot-functional `ensures`. `Pre := True` recovers today's `FoldOk`.
Upstream suggestion: `manual_proof!` with an `assuming:` clause.
**Sequencing ruled: D71 (this redesign + the PP1b/AP2/Recommit
migrations) opens after the parent squashes D70.**

### D71 — R3 lands: conditional commutativity (`Pre`/`requires`/`ghost pre`), the PP1b/AP2/Recommit migrations, the recommit fold is NOT commutative (Rust changed), per-tick order selection coupled to the machine, tick-trace monotonicity in the corner, ghost hoisting

**Scope.** The D70 queue: R3 (ruled at D70's close) → migrate the three
keyed sites onto the D70 operators. It became five stages because the
third site refused to migrate under R3 — the Rust claim it carries is
false — and because the engine's ghost layer did not scale to the
proofs that followed.

**Stage A — R3 built: `FoldOk … Pre g`, `requires`, `ghost pre`, one
`invariant`.** The fold family (`fold`, `bfold`, `bkfold`,
`reduce_watermark`) takes `Pre : α → α → Prop` and `FoldOk ord ret Pre
g` (commutativity on `Pre`-pairs; `Pre := fun _ _ => True` is the old
obligation — every existing site unchanged modulo the extra argument).
Denotation: a permutation-invariant per-key reduce (`Multiset.foldlOn`
via `List.Perm.foldl_eq'`), `⊥` where the pairwise fact fails. Machine:
the literal closure in arrival order. Transfer: the realized content is
a sub-multiset of the denotation's and pairwise facts are hereditary, so
the arrival-order fold IS the order-free one — proven once per op in the
`CoupleSem` field. The content half ("every two values that meet
satisfy `Pre`") is not an operator argument (a program against an
abstract `H` cannot prove content): the surface grew **`requires R`**
(the module's premise — a hypothesis `hreq` of its proofs, NEVER a
conditional on the `ensures` face; ruling) and **`ghost pre h : S :=
prf`** (the site fact, proven at the denotation BEFORE the site, packed
as `M.pre_h : PreFact R S`, consumed by the generated `_co_wf` through
`hydro_pre_close`; a conditional fold without its `ghost pre` does not
elaborate). The `fix` construct's `assume`/`export` invariant dialects
collapsed into one **`invariant`** keyword (ruling): a wire-only
predicate is assumable of the loop input (conjoined into the hoisted
chain's `requires`, fed to the body at exactly the Kleene stage the
`wf` probes — sound because the `wf` only probes Kleene stages) AND
exported; a predicate over chain lets is export-only. Toys `toy_pre`,
`toy_wm`, `toy_wm_fold`, `toy_kmax` pin the shapes.

**Stage B — PP1b.** paxos.rs:548–560 is now operator-for-operator:
`into_keyed → kassume_ordering → kfold_early_stop → kget_max_key →
snapshot`; `dec.snap` became `.noOrder` cuts of the keyed source. The
keyed chain is read through closed forms in `Keyed.lean` (a view is a
legal cumulative cut of the selected pairs, ascending along ticks; a
bucket exists iff `quorum_size` logs arrived at the ballot and is their
first `quorum_size` — `ofFoldEarlyStop_push_get_eq_some_iff`;
`get_max_key` answers the entry at the largest ballot —
`maxEntry_eq_some_iff`). `PP1bEnsures` byte-identical (R9).

**Stage C — AP2.** paxos.rs:851–864 is `H.reduce_watermark
(H.tinto_keyed …) a_checkpoint LogTie logMerge logMerge_comm`: the merge
`if entry.ballot > prev.ballot { *prev = entry }` transcribed literally
(`logMerge`), commutative under the tie `LogTie` (`x.ballot = y.ballot →
x.value = y.value`), the content half `P2aFunctional` (the P2a pool is
`(slot, ballot) ↦ value` functional) as `acceptor_p2`'s `requires`,
discharged by `ghost pre` in `sequence_payload` (`hp2a_fun`, a leader
sends a key once) and threaded to `paxos_core`'s `requires variant =
.guarded`. **Watermark GC is modeled** (`KeyedMap.gcBelow`) — the D58
"safety-neutral" note was a belief about an unmodeled operator. Two
consequences: (i) the checkpoint input is typed **`.monotonic`**
(ruling): the running watermark IS the published checkpoint, so a
collected slot sits below the view's own checkpoint — what the
proposer's recommit discipline consumes; Rust's `Optional` has no
`Monotonic` bound (divergence noted, upstream: strengthen). (ii) **B4**:
with GC, `p_max_slot` (the champions' max) can sit below the view
checkpoint, and `index_payloads` rebases fresh payloads from it — a
fresh payload at a collected, executed slot (paxos.rs:655 × :788).
Fixed under `.guarded` only (`rcMaxSlot`/`rebaseAboveCheckpoint`);
`.faithful` runs it in `lake exe falsify` (B4 section). The Rust is
not changed for B4 (report list). K4 restated over `rcEntries` and the
checkpoint (`log_covers_mono`/`log_len_le_ck` faces of `AP2Ensures`).

**The mono gap (user: "very important to make sure values actually
covers real concurrent executions").** Preparing Stage D, the per-tick
order decision had to be DERIVED from the machine's arrival order (not
chosen freely at `Values`), and deriving it needs the machine's tick
trace at step `k` to be a prefix of its trace at `k' ≥ k`. `Sched` had
no such fact and the corner could not state it. Ruling **(A)**: record
`mono` in the corner, not in `Sched` — `Sched` stays the minimal,
auditable operational semantics; `Couple` is where proofs live. (A
proof-expressiveness gap, not a soundness one: the machine IS
monotone.) Built: `SchedMono.lean` (`TMono x := ∀ i k k', k ≤ k' → x i
k <+: x i k'`, per-op lemmas for every tick-producing op incl.
`tick_scan` and `fix_tick` — stabilization `stab_iterate_tick` + body
monotonicity, generic in the knot arity); every tick carrier of the
corner (`CoTickSing/CoAtomicStream/CoAtomicKStream/CoAtomicKSing`) got
`mono : wf → TMono sr`; `fix_tick`'s `wf` is now FOUR conjuncts (W4:
the body preserves `TMono`); the engine got the `_smono` walk
(`GenInfo.smonoLemmas`, `genSMono` per leg, `K_smono₁` per knot,
`coSMonoDispatch`), run after causal in every route.

**Stage D — the recommit fold is not commutative.** Migrating
paxos.rs:617–640's `(count, champion)` closure under R3 meant proving
`hcomm` under some `Pre`. There is none: one slot, f = 1, entries
X = (ballot 1, v), Z = (ballot 2, w), Y = (ballot 3, v). Order X·Y·Z:
(1,X)→(2,Y)→(2,Y) — count 2 > f, the slot is SKIPPED. Order X·Z·Y:
(1,X)→(1,Z)→(1,Y) — count 1, the slot is RECOMMITTED. Same multiset,
same champion, opposite decisions; reachable with three acceptors (A
accepts (1,v); the ballot-2 leader with quorum {B,C} proposes w, B
accepts; the ballot-3 leader with quorum {A,C} recommits v, C accepts;
the next leader's quorum sees X, Z, Y in arrival order). The pre-D71
model (`pooled` multiset + `logView` + `rcCount`) was a commutative
"honest refinement" that HID this — exactly R3's point. Safety holds
under every order (the count never overshoots the entries carrying the
champion's value, so a skip means ≥ f+1 hold it — chosen by quorum
intersection, no other value chosen; a recommit proposes the champion's
value, the slot's max-ballot value); only efficiency (a redundant
recommit) is order-dependent. **Ruled (i)**: the Rust gets
`.into_keyed().assume_ordering::<TotalOrder>(nondet!(/** X·Y·Z + safety
argument */))` before the `.fold(…)` and loses the `commutative =
manual_proof!(/** TODO */)` (landed in `hydro_test/src/cluster/paxos.rs`;
`cargo check`, `cargo fmt`, all 67 `hydro_test` lib tests green, the
three `paxos_ir` snapshots regenerated — the diff is the one
`ObserveNonDet` node); the Lean mirror is a per-tick order-selection op
plus a content-nondet family, with the LITERAL closure (`rcStep`), no
tie prerequisite, champion = first-arrived among equal ballots.
Rejected (ii): commutative per-value counts (a different algorithm).

*The operator.* `tassume_ordering : AtomicStream ℓ α .noOrder → TickOrderSelDec
(mem ℓ) α → AtomicStream ℓ α .totalOrder` (stream-typed: `TickShape` has
no keyed-stream leaf, so Rust's `into_keyed().assume_ordering()` on a
tick batch is this on the pair stream + `binto_keyed` in the body).
Decision `TickOrderSelection n α := Fin n → List (List α)`. Denotation
`realizeTicks`: a tick is realized only by a selection that is a
permutation of its batch (`Multiset.ofList s = p`), else the trace is
CUT there — **"an illegal selection blocks"**. Three semantics were
weighed: a legality hypothesis on the headline (rejected: the theorem
would carry a decision premise, D37's lesson), a `LinearOrder` fallback
(rejected: invents an order the Rust does not have), blocking (chosen:
the decision space is total by construction, the headline quantifies
over every order, and a scenario with a wrong order visibly commits
nothing). `Sched`: identity (`Unit` decision — the machine's order is
the arrival order). Corner: `rr := Values.tassume_ordering u.rr
(tkOrdSelDerive Td u.sr)`, `cpl := prefix_realizeTicks (u.cpl …) (u.mono
…)` — the derivation reads the machine's realized tick through `mono`.
`HRel` field `tickOrderSelRel`, laws in `CausalHRel`/`EagerRel`/
`MonoHRel`, `DecFam.tickOrderSel` in the census.

*Recommit rewritten.* `RCDec.order : TickOrderSelDec`; block 1 flattens
the view's logs to entry pairs, `p_entries_ordered := H.tassume_ordering
p_entries dec.order`, block 2 `bkfold rcStep (0, none) (fun _ _ => True)
trivial (binto_keyed ents)` — the literal closure — then the Rust
`filter_map`/holes. `RCTick` restated: `value_best` (∃ best, (slot,
best) ∈ rcEntries gv ∧ e.2 = best.value ∧ every entry's ballot ≤
best.ballot — the champion is itself a view entry, D64's `logView_entry_value`
detour is gone) and new `skipped_holders` (a slot with entries and no
recommit is executed — ≤ the reported checkpoint — or its champion is
held by > f entries); `RCEnsures` lost `tick_at` (SP reads via
`maxslot_at`). `Types.lean`: `rcStep`, `RCInv`, `rcFold_champ`
(champion ∈ l ∧ dominates ∧ count ≤ holders), `rcFold_none`;
`logView`/`ValWitness`/`valOf`/`rcCount` and their lemmas DELETED. SP:
`SPDec.rc`, `hrc_at` (RCTick at any `w ≤ t`), `LeaderOpen` restated
over `rcEntries`. PaxosCore: K4 step 7 simplified (the champion is a
view entry: `hview_entry best hbestmem`). The X·Y·Z pair is a
`#guard_eval` pair in `Recommit.lean` (ballots 1/2/3, values 5/6/5,
leader ballot 4: `[[]]` vs `[[((0, ⟨4,0⟩), some 5)]]`), plus an
illegal-order witness (`[[]]` on a nonempty view → `[]`).

*Scenarios.* Every B1/B2/B4 record and the Exploration/Demo records
gained `rc` orders — exact permutations of the GATED view per tick.
Three were wrong on the first try and the exes said so (B1 faithful
"P1 commits x: false"; B2-core faithful commits nothing): under
`.faithful` the gate re-presents the view on every leader tick, under
`.guarded` once — so the order list is variant-dependent (`b2Run`,
`b2cDec ySlot`), and B1's P1 sees two EMPTY logs (its fake quorum is
acceptor 0's duplicates), not acceptor 1's entry.

**Engine findings (fixed).** (1) `try rw [a]; exact h` parses as `try
(rw [a]; exact h)` — a knot with no wire inputs (identical `fixRw`
sides) silently lost the rest of `genKnotCausal`'s script;
parenthesized. (2) A glue module whose body contains a `tick_scan`
needs `coSimpBase` in `genGlue` (the former's simproc is unreachable by
the keyed residual pass). (3) **The SP ghost blowup**: `hcommit` cost
161M raw heartbeats. `trace.profiler`: `elabAppArgs` on `hmem : p2a ∈
Multiset.map f (List.sum (payloads_to_send r))` did a default-
transparency `whnf` that zeta-delta'd the wire and delta-unfolded every
callee module down to its tick formers — 2.7 s per `obtain`. Fix: every
registered module not prefixed by the current one is `[irreducible]`
while a ghost fact elaborates (restored in `finally`); 161M → 4.85M.
`leader_election`'s `hstep` legitimately instantiates a callee at
`MonoRel` and says `with_unfolding_all`. An "opaque-first lets" variant
was tried and reverted (the unifier delta-unfolds callees when zeta is
blocked — slower). (4) **Ghost hoisting + per-fact budgets**: every
top-level `ghost have` is its own theorem `M.<h>` (the proof leg cites
it, the module's ghost text elaborates once), each under
`withCurrHeartbeats` — SP had crossed the default budget by GROWTH
alone (~60 facts sharing one command budget). `HYDRO_GHOST_DIAG=1`
prints per-fact cost. (5) `#guard` on a nested-knot whole-program run
is a kernel evaluation — 36 minutes once; `#guard_eval` (compiled,
build-failing) replaces it everywhere a program is run. **Open
(noted)**: a `fix` chain `let` used only after `complete` is not
captured (free-variable error); a glue module whose knot captures a
record that also carries a callee's content decisions mis-extracts the
`vdec` when that callee is called after `complete`. House pattern keeps
content inside the knot body.

**Heartbeats.** LE's per-fact table summed to ≈1091M raw before hoisting
(the 1.6M budget stays for now — "eliminate the Paxos overrides" is
queued with a per-lemma baseline); PaxosCore's budget dropped 400k →
250k (sum of its facts); SP builds under the default (47 s). Doctrine
unchanged: a module exceeding the default budget is an engine blowup to
fix, never a budget to raise.

**Gate.** `lake build` 826 jobs green (≈ 7 min: LE 5m22s, SP 47s);
zero sorries; axioms std-three / `[propext, Quot.sound]` / `[propext]` /
axiom-free as before, `sorryAx` 0; `falsify` ALL PASSED (B1, B2, B2-core,
B4), `explore` true+true, `paxos` OK, `twopc` OK; censuses LE 8/3/3,
acceptor_p1 0/1/0, p_p1b 3/0/0, **recommit 1/0/0 (new)**,
**sequence_payload 6/2/0**, p_leader_heartbeat 3/1/0, acceptor_p2 2/1/0,
**paxos_core 14/5/5**, two_pc 2/4/0, quorum ×3 1/0/0; toys toy_kmax
2/0/0, toy_wm 1/0/0, toy_pre 1/0/0, toy_wm_fold 1/0/0, **toy_tk_consume
2/0/0, toy_tk_order 2/0/1 (new)**. Rust: `cargo check -p hydro_test`,
`cargo fmt --check`, `cargo test -p hydro_test --lib` 67/67.

**Upstream (added to `SCHED_AUDIT.md`'s list).** paxos.rs:617–640
order-dependence (landed here as the `assume_ordering` + `nondet!`
text); B4 (`rcMaxSlot`); `Optional` lacks a `Monotonic` bound;
`manual_proof!(requires: …)` for paxos.rs:862.

**Queue after D71.** Paxos under default heartbeats (PaxosCore 250k, LE
1.6M; per-lemma baseline first); memoized `Values` evaluation for
witnesses; liveness rung 2; the already-queued decision-record validity
diagnostic — D71 is its second motivating instance: `realizeTicks` cuts
a trace on a non-permutation order exactly as `batchCuts` does on an
illegal cut, and the hand-written `rc` orders cost three exe rounds to
locate by output alone (the diagnostic should report, per member, the
first tick at which ANY cutting decision diverged from the realized
content); engine nits (unused `HydroSem.fix/fixTick` simp-arg
warnings, residual `]'` reads, D49 walker → `_param`); the two open
capture findings above; the `.boundedValue` keyed-singleton batch op;
`SchedSem` against production DFIR; rung-b commit
liveness; the upstream report list.

### D72 — Paxos proof time: a failing `apply` is exponential in the projection spine, the witness that cost two thirds of a file, LeaderElection 323 s → 74 s; the overrides are measured sums (ruling: heartbeats are a proxy, time is the goal)

**Scope.** Queue item 1 after D71: find out why the proofs behind the
`set_option maxHeartbeats` overrides on `leader_election` (1.6M) and
`paxos_core` (250k) take so long, fix the engine causes — never move a
budget — and recover `LeaderElection.lean`'s D71 regression (273 s →
324 s). Baseline first. (The task as posed asked to delete the overrides;
the user re-ruled mid-way: heartbeats are a proxy, proof time is the
goal — see root cause 2.)

**Baseline (per-declaration table, `HYDROGEN_HB_DIAG`/`HYDRO_GHOST_DIAG`
+ the new `HYDRO_PHASE_DIAG`, `scripts/heartbeat_table.sh`).** Units are
`maxHeartbeats` units (raw / 1000; the default is 200,000). LE's
`hydro def` command: 70.6 s wall, of which the def's own elaboration
49 s (the eager knot stacks and the body's module pipeline run INSIDE
it, by design — ghosts and `prove` legs cite the generated vocabulary).
Largest declarations: `leader_election_smono₃` **254,131** and
`_causal₃` **203,854** (each above the default alone), `_param_all`
109,748, `body_param_all` 87,363, `_smono₂` 85,252, `body_co_rr_ex`
82,596, knot `p_is_leader` rr 74,537, `_causal₂` 68,707, `_co_wf₂`
36,785, knot `p_is_leader` wf 35,641, `body_co_wf₁…₁₆` 10–17k each
(≈190k together), ghost `hstep` 13,313. Everything a `hydro def` emits
counted against the ONE command: the def-elaboration sum ran to ≈500M
raw — that sum, not any theorem, was the 1.6M. PaxosCore: def-elab
20.5 s (knot `a_log` wf 49.5k, `sequencing_max_ballots` wf 41k,
`body_param_all` 68k, the K4 `invariant` induction ≈41k) — the sum was
the 250k. But the file-level profiler (`set_option profiler true` at the
TOP of the file, not `in` on the def) told the real story for LE:
`interpretation 208 s` — the three `#guard_eval`s of `leScenario`, a
call-by-name `Values` run of the whole election, took 138 s + 33 s +
36 s. The `hydro def` was 70 s of a 320 s file.

**Root cause 1 — a failing `apply` against a deep shared projection
spine is exponential.** Reproduced in isolation: `with_reducible apply
leader_election.body_smono₁₄` on the goal `TMono (body …).2^14.1` takes
**7.08 s to fail**; `₁₂` 2.44 s, `₁₀` 0.83 s — ~1.7× per shared
`Prod.snd` level. The unifier matches the spine structurally, learns
the mismatch at the bottom (`body ?args =?= Prod.snd (body A)`), and on
the way back up retries each level in its unfolded `.proj` form
(`Prod.snd` is reducible); with metavariables on the rule side the
failures are not cached, so the retries multiply. The walks
(`hydro_causal_walk`, `hydro_smono_walk`, `hydro_param_walk`, the wf
phase's knot closers, `hydro_param_callee`) were `first |
with_reducible apply r₁ | … | apply rₙ` over every callee rule in
order — the deepest leg paid for every shallower rule first. D56 had
met the same cliff in the mono resolver and keyed it
(`conclKeyMismatch`); the other walks had not been keyed.
**Fix: `hydro_rule_apply [rules]`** — a rule is applied only when its
conclusion CAN match: same relation head and arity (the goal's relation
read at reducible transparency, so a shaped leaf's `TAgreeOf h (.stream
…) x y` IS `TAgree h x y`; a structure projection read the same whether
spelled `S.f s` or `.proj S i s`), and every argument pair with two known
subject keys (`subjKeyOfM`: head constant + literal projection path,
descending through `Prod.fst/snd`, `Subtype.val`, `.proj`, and
`@[reducible]` heads — a knot's hoisted `<K>.body` keys as the chain
leg it unfolds to) agrees. Anything uncertain falls through to `apply`.
`smono₃` **240,810 → 206** (11.4 s → 16 ms); `causal₃` 203,854 → under
1k; LE's wrapper `co_wf₂` 36,785 → 2,730; `param_all` 109,748 → 64,576.
Two traps on the way, both caught by the toys: `forallMetaTelescopeReducing`
unfolds the rule's conclusion too (`TMono x` became its `List.IsPrefix`
body — the key test silently never fired until the non-reducing
`forallMetaTelescope` replaced it), and `toy_loop`'s knot cited rules
whose conclusion head is the chain module while the goal's head is the
knot's reducible `.body` alias (hence `subjKeyOfM`).

**Root cause 2 — one command, sixty declarations (the overrides).**
Everything a `hydro def` generates is elaborated inside the ONE command
(the eager knot stacks and the body's module pipeline run inside the
`fix` elaborator, by design — ghosts and `prove` legs cite the
generated vocabulary), and Lean counts it all against that command:
LE's def-elaboration sums to ≈500M raw after the fixes below, PaxosCore's
to ≈225M, with no single generated theorem above 90M. The overrides ARE
those sums. D72 built and then dropped per-declaration accounting
(`withCurrHeartbeats` for each generated theorem plus
`IO.setNumHeartbeats` back on exit — Lean's snapshot-resumption
primitive; it deleted both overrides with the command residue at 35M).
**Ruling (user): heartbeats are a proxy; proof TIME is the goal.** The
accounting trick moves the number without moving the time, so it is
out; the overrides stay as measured facts — `LeaderElection` 1.6M →
**600k** (500k fails, 600k passes), `PaxosCore` 250k → **250k** (measured minima after the time
fixes) — and the per-declaration table is the instrument that says
which declaration is slow and why. The `hydro def` command's wall time
is what the table is for: LE 70 s → 47 s after the walks were keyed.

**Root cause 3 — the witness.** `LeaderElection.lean`'s non-vacuity
scenario ran at `Values` (call-by-name) under `#guard_eval` (D71 moved
it off kernel `#guard`, which took 36 minutes). 207 s of interpretation.
The scenario now runs at **`Eager`** — the same decision record `leDec`
(at `Values`; `ledecE` repacks it, moved here from `EagerCheck`), inputs
`EagStream.input`/`EagTickSing.input`, outputs read off the `data` leg —
exactly the house pattern of `PaxosDemo`/`EagerCheck`. **The guard is
covered by theorem** (user: "LE has a Values → Eager correspondence
proof, right? to make sure the guard is correct" — `paxos_eager_den`
covers `paxos_core`'s run, not an election run on its own inputs):
`leader_election_eager_den₁/₂/₄` state that the `den` leg of the eager
election at `ledecE d`/`LESched.trivE` IS the `Values` election at `d`/
`LESched.triv` on the inputs' denotations, for the three legs the guards
read — two-line corollaries of the generated free theorem
`leader_election_param₁…₄` at `eagC`/`eagLaws`, no scenario-specific
proof; `EPack.agree` ties the executed `data` to `den`. Milliseconds.

**Deleted.** `scripts/budget_probe.sh` (replaced by
`scripts/heartbeat_table.sh`, the per-declaration cost table: which
generated theorem / ghost / knot phase costs what, in heartbeats AND
milliseconds). The overrides: `LeaderElection` 1.6M → 600k, `PaxosCore`
250k (unchanged), both measured.

**Per-file, before → after** (same machine, 16 cores): `LeaderElection`
**323 s → 74 s** (the `hydro def` 70 s → 47 s, the witness 207 s → ms);
`PaxosCore` 35 s → 35 s; `SequencePayload` 47 s → 46 s; `HydroTickCheck` 43 s → 43 s
(twenty toys, nothing pathological; `TMono`/`TAgreeOf` toys gained
`hydro_rule_apply` coverage); `Grades` 19 s (unchanged). Full build
≈ 8 min → **≈ 2.5–3 min** wall. Remaining per-declaration hot spots
(none above half the default): `body_param_all` 87M / 5.3 s (sixteen
conjuncts each re-unfolding the body), `body_co_rr_ex` 83M / 7 s,
`sequence_payload_co_wf₁` 76M / 7.6 s, knot `p_is_leader` wf 17.8M but
7 s (kernel time on the proof term, not Meta heartbeats), `a_log` wf
likewise 8.3 s. Queued below.

**Instrumentation added (env-gated, zero cost when off).**
`HYDRO_PHASE_DIAG=1` — wall clock per pipeline phase and for the def's
own elaboration; `HYDROGEN_HB_DIAG=1` now prints `[hb] <decl>: <k> <ms>`
for EVERY generated declaration (including the naming theorems and the
knot phases, with `cmd-used=` the running command total); `HYDROGEN_TAC_DIAG=<suffix>`
prints a generated lemma's statement and script (how `smono₃` was
reproduced byte-for-byte outside the pipeline); `HYDROGEN_RULE_DIAG=1`
prints `hydro_rule_apply`'s per-rule key decision and apply time.
`scripts/heartbeat_table.sh <file> [N]` wraps them.

**Gate.** `lake build` 826 jobs green; zero sorries; axioms std-three /
`[propext, Quot.sound]` / `[propext]` / axiom-free as before, `sorryAx`
0; `falsify` ALL PASSED (B1, B2, B2-core, B4), `explore` true+true,
`paxos` OK, `twopc` OK; all twelve program censuses unchanged (LE 8/3/3,
acceptor_p1 0/1/0, p_p1b 3/0/0, recommit 1/0/0, sequence_payload 6/2/0,
p_leader_heartbeat 3/1/0, acceptor_p2 2/1/0, paxos_core 14/5/5, two_pc
2/4/0, quorum ×3 1/0/0) and six toys; `grep -rn "^set_option
maxHeartbeats" Hydro/ *.lean` → exactly the two (800k, 250k). Program
text, faces, headlines, `Values` denotations untouched (the only
program-file edits are the two option values and `leScenario` at
`Eager`).

**Queue after D72.** The remaining slow generated declarations (the
overrides' remaining mass): `body_co_rr_ex` 7 s and `_param_all` 5 s in
LE, `sequence_payload_co_wf₁` 7.6 s, the knot wf theorems' kernel time
(7–8 s each in LE/PaxosCore) — `_param_all` re-unfolds the module per
conjunct (unfold once, then split); `_co_rr_ex` and `sequence_payload_co_wf₁`
(profile the `co_wf_simp` loop — `repeat'` re-running a module-sized
`simp` as a fallback alternative); the knot wf theorems' kernel time
(proof-term size). Then the D71 queue: memoized `Values` evaluation for
witnesses (less pressing — witnesses run at `Eager`); liveness rung 2;
the decision-record validity diagnostic; the engine nits; the two open
capture findings; the `.boundedValue` keyed-singleton batch op;
`SchedSem` against production DFIR; rung-b commit liveness; the
upstream report list.

### D73 — the denotation is the executable: `MemberMap` carriers at `Values`, the `Eager` interpretation retired, and the two compiler facts that made a one-field structure a function again

**Scope.** Queue item 1 after D72 (user: "fast Values evaluation /
interpretation unification"): make the `Values` programs themselves the
executables, so the executable-witness rule (R7) needs no second
interpretation and no per-program pin, and the three `#guard_eval`s D72
moved to `Eager` come back to the denotation. Ratified design (checkpoint
#1; user: "I like MemberMap, otherwise sounds good"): member-indexed
carriers become `MemberMap n C`, a structure around `Fin n → C` whose
denotational constructor `mk'` is `@[reducible]` (reads `rfl`) and whose
compiled constructor `mkImpl` forces the cells into a `Vector` once;
`mkImpl_eq` is the one pin — and after a user question about who checks
the seam, the pin became the authorization: `@[csimp] mk'_eq_mkImpl :
@mk' = @mkImpl` (how core replaces `List.map` by `List.mapTR`), so the
compiler may substitute ONLY through a kernel-checked equation and there
is no `implemented_by` in `Hydro/`. (`implemented_by` is unverified by
design — the kernel never reads it; it can never make a false theorem
provable, only make an executable diverge from the program; the one
bridge, `native_decide`/`Lean.ofReduceBool`, is an axiom we don't use.) Options weighed and
rejected: a `Vector` carrier (loses the `rfl` reads every `den` lemma and
generated script relies on), status quo + more pins, `Eager` as THE
denotation, a `Values`/`Sched` carrier unification (out of scope;
`MemberMap` could later wrap `Sched`'s member indexing too).

**Why a structure at all.** Lean's compiler eta-expands every
declaration to its TYPE's arity (`Lean/Compiler/LCNF/ToDecl.lean`,
`Decl.etaExpand`), so a definition of function type can never be
pre-forced — `let v := Vector.ofFn f; fun i => v.get i` becomes `fun i =>
(Vector.ofFn f).get i`. The measurement that motivated it: at the
call-by-name `Values` one output leg of the LE witness read its `p2b`
input 4,775,952 times (48.6 s); `Eager` read it once.

**What changed (engine).** `Hydro/MemberMap.lean` (new). `Values.lean` on
`MemberMap` — every op `.mk' fun i => …`; `SingletonV`/`TickV` are
`MemberMap`s; `ValuesTick.unslice` takes one; per-family `CoeFun`
instances keyed on the instance-projected types (class fields are not
reducible, so `DiscrTree.getUnify` cannot see `(Values L mem).Stream …`
as a `MemberMap`; marking the fields `@[reducible]` broke the `tick`
elaborator's wire classification for abstract `H` — reverted).
`MonoRel`/`Couple`/`CoupleProj`/`WfTactics`/`Transfer`/`CoupleCheck`/
`TransferChecks`/`MonoHRel`: binder spellings; `coSimpRules`/`coWfSimpRules`
gained `MemberMap.get_mk'`/`get_mk`. Generator: `memberMapParts`/
`mmRead`/`mmMk` helpers, seeds via `mmMk`, reads via `mmRead`, `congrFun`
→ `MemberMap.get_congr`; `subjKeyOf`/`subjKeyOfM` descend through
`MemberMap.get`/`cells` like `Subtype.val`. **Deleted:** `Eager.lean`,
`EagerProj.lean`, `EagerRel.lean`, `Paxos/EagerCheck.lean`, the LE pins
`leader_election_eager_den₁/₂/₄` + `ledecE`/`trivE`, the `HydroParamCheck`
/`HydroTickCheck` eager examples; `AxCheck` lists `MemberMap.mkImpl_eq`
instead. Programs: carrier spellings `Fin (mem ℓ) → T` → `MemberMap (mem
ℓ) T` for carriers only (derived plain functions such as `ck : Fin (mem
acc) → Trace (Option Nat)` in `AP2Ensures`/`SPEnsures`, `vote` in TwoPC,
the existentials `byAcc`/`pool` stay functions); scenario inputs `.mk'
fun _ => …`; `leScenario` back at `Values` with `#guard_eval`; the four
exes at `Values` (`PaxosDemo` needed `(ckret := .exactlyOnce)`).
Statements: `paxos_safe_sched'`, `cq_safe_sched'`, `cq_live`,
`cq_live_chain`, `two_pc_*` byte-identical; `cq_safe_sched'` wraps `.mk' v`
inside its proof; `collect_quorum_tight` gained `.mk'` around its
input (`(.mk' fun _ => Multiset.ofList ((h 0).view T₀))`) — the one
headline-shape change, meaning-identical; `cq_complete_mem`/
`cq_chain_live`/`cq_chain_live_stable` take `MemberMap 1 …`.

**Perf gotchas on the proof side (fixed, in `ENGINE_NOTES.md`).** (a)
`MemberMap.get` as the structure PROJECTION made `get A i =?= get A' i`
skip the unifier's same-head `isDefEqArgs` heuristic and `whnf` the whole
program upstream (one `exact` in a knot wf = 34M heartbeats, LE +50 s) —
`get` is a regular `@[reducible] def` over `cells`. (b) `conclKeyMismatch`
let `leader_election_mono₂` through against `paxos_core.body`'s leg 7
(`mapTick (zipTick (le.1) …)`): 403M heartbeats failing — the walks now
skip a callee-leg rule against an OPERATOR-headed (`Hydro.HydroSem.*`)
subject; PaxosCore def-elab 50 s → 17 s, 605M → 189M raw, and
**`PaxosCore`'s `maxHeartbeats` override is deleted** (one override left:
LE 600k, measured). (c) the `CoeFun` fact above.

**The runtime blocker and its root cause (this entry's finding).** With
all of the above green, `paxos_core` at `Values` ran 1×1 in 107 ms but
2×1 did not finish in 60 s — linear in acceptors, catastrophic in
proposers; `falsify`/`explore` (2×3) spun for minutes. Hypotheses
examined and NOT confirmed first: compiler eta-expansion of `mkImpl` (a
toy iterate read once per member), trivial-structure erasure of the
return type (eta runs on base-phase types), Bekić duplication of the
knot bodies after `zetaLets` (the IR of `paxos_core._redArg` is
straight-line: `sequencing_max_ballots`, then `a_log`, then `body`, each
once; the nested fuels predict 4–8 chain evaluations). The method that
found it: a `dbgTrace` on the forced VECTOR inside `mkImpl` (compiler-
only — the compiled constructor is not proof-relevant; a first attempt traced the
map itself, `dbgTrace msg fun _ => ⟨…⟩`, and re-introduced the laziness:
`LeaderElection` 74 s → 9+ min, which is itself the finding below), so
`sort | uniq -c` over stderr counts member-map constructions by member
count — i.e. op applications per location. Three harnesses (a
`paxos_core` scaler, per-sub-program runners, the toy knot) at 1, 2, 3, 4
members. Flat is right. Measured: `p_ballot_calc`/`acceptor_p1` flat;
`toy_loop` flat (knots per se are fine); `p_leader_heartbeat` 11 → 43 →
139 → 367 and `p_p1b` ~11 → 60 → 175 → 440 (knot-free, straight-line
programs whose op applications grow ×3 per member); `leader_election`
852 → 14k → 344k → 3.9M (≈16^(n−1) — 16 is LE's chain-evaluation count,
so the whole knot solution was being re-run per read, recursively);
`paxos_core` 2×1 at fuel 0 — two chain evaluations — 2.95M proposer-side
constructions in 14 s. Then the IR (`Lean.IR.declMapExt.getModuleEntries`
dumped from a scratch `#eval`): in the specialized `p_leader_heartbeat`
the cell lambda of `mapTick` was `_lam_7 n g f i := let s := mkImpl n g;
(s i).map f` — the UPSTREAM map rebuilt inside the downstream cell — and
the `fix_stream` op lambda had a trailing extra parameter, `_lam_9 … fuel
body i := iterate body seed fuel |> (· i)` — the whole Kleene iteration
per read. A 2-op toy with `trace.Compiler.result` shows the first as
`_x.overap := mkImpl n _f` moving across the lambda between the
`specialize` output (correct) and the final result. Two LCNF rules, one
cause: **a structure with exactly one relevant field is erased to that
field** (`hasTrivialStructure?`, `MonoTypes.lean`), so to the mono-phase
passes a `MemberMap` WAS a function, and (i) `Simp` eta-expands local
lambdas whose erased type is an arrow (`isEtaExpandCandidate` → the extra
`i`), (ii) `simpAppApp?` (`Simp/SimpValue.lean:34`: `let g := f a; g b ⟹
f a b`, with no check that `f a` was a cheap partial application) merges
a let-bound map into its read. Each read re-runs its producer; through a
chain, `n^depth`; through a knot, the iteration per read. `Eager` was
immune because `EPack` had two relevant fields; the old bare-function
`Values` suffered both, which is the other half of D14's hours. A
`@[noinline] get` (keeping the map out of head position) fixed the
straight-line programs (`p_leader_heartbeat` 367 → 13 at four members)
but not the knots (eta-expansion is about the op lambda's TYPE, not the
read) — reverted in favour of the real fix: **a second relevant field,
`boxed : Unit := ()`**. The map is a heap object; no arrow to expand, a
read is a projection then an application, `let`s stay where the program
put them. After: `leader_election` 694 → 694 → 686 constructions at 2, 3,
4 proposers (the n=1 figures include ~850 constructions from module
initializers — the library's closed top-level maps); `paxos_core` 2×1
17 ms, 2×3 (fuel 3) 35 ms, 4×4 36 ms; the four binaries run their
scenarios in ≤ 40 ms wall (`paxos`/`twopc`/`explore`/`falsify`).

**Per-file, before → after** (same machine; D72 figures in parentheses):
`LeaderElection` (74 s) 75 s — def-elab 47 s, `p_is_leader` wf 34.6M, the
three guards now at `Values` in ms; `SequencePayload` (46) 47 s;
`HydroTickCheck` (43) 44 s; `PaxosCore` (35) **32 s, no override**;
`Quorum` 11 s; everything else < 12 s; full build (2m50s) 3m12s / 823
jobs (the Eager files are gone; the per-family `CoeFun` instances and the
`MemberMap` binder spellings cost nothing measurable). Wall figures vary
±5% run to run.

**Gate.** `lake build` 823 jobs green; zero sorries; `AxCheck` 36 reports,
33 `[propext, Classical.choice, Quot.sound]` + 3 `[propext, Quot.sound]`
(`mk'_eq_mkImpl` included; no `Lean.ofReduceBool`),
`sorryAx` 0; `falsify` ALL PASSED (B1, B2, B2-core, B4), `explore`
true+true, `paxos` OK, `twopc` OK — every binary ≤ 40 ms; all twelve
program censuses unchanged (LE 8/3/3, acceptor_p1 0/1/0, p_p1b 3/0/0,
recommit 1/0/0, sequence_payload 6/2/0, p_leader_heartbeat 3/1/0,
acceptor_p2 2/1/0, paxos_core 14/5/5, two_pc 2/4/0, quorum ×3 1/0/0) and
six toys; `grep -rn "maxHeartbeats" Hydro/Paxos` → exactly LE's 600k.
Docs: `ARCHITECTURE` (§3 interpretations table: four rows + `MemberMap`;
§9 perf), `DOCTRINE` R7/R9, `GATE` (headline list, exe timing envelope),
`CORRESPONDENCE`, `README`, `ENGINE_NOTES` (gotchas (a)–(c) and a new
"Runtime" section with the four compiler facts and the leak-finding
method), `docs/01/02/03/04/08/09` — every `Eager` mention retired or
rewritten; `HydroDef.lean`'s budget note.

**Operational lessons (user rulings this entry).** Never print unbounded
output — redirect and `tail`; for a timed probe `sleep` exactly its cap;
never run `trace.Meta.isDefEq`-class traces on whole-program terms (8.9 GB
log); never put `dbgTrace` into a proof-relevant reducible definition (an
LE elaboration ran 11 min / 9 GB); kill stray `lake env lean`/exe
processes; and the user wants the costs REASONED about ("figure out
where the costs are that weren't there with Eager") — the IR, not the
stopwatch, located this one.

**Queue after D73.** (Low priority, user) one kernel-trust witness: a
`decide`/`rfl` theorem that a minimal `paxos_core` scenario (1×1, one tick)
at `Values` produces its expected output — the compiled path's agreement
with the kernel on one instance as a checked fact rather than an argument
(`#guard_eval` and the exes sit at compiler trust). Cosmetic: move the four exe `main` roots (`Falsify.lean`,
`Explore.lean`, `PaxosDemo.lean`, `TwoPCDemo.lean` — loose at the top level
because the lib is `globs = ["Hydro.**"]`) into an `Exe/` directory so
`Falsify`/`Falsification` stop echoing each other (user). `lake check` (core's Comparator, toolchain ≥ 4.35)
for the headlines — kernel replay, statement identity, permitted axioms —
replacing `AxCheck.lean` and the R9 diff rule. The remaining slow generated declarations
(`body_co_rr_ex` 7 s, `_param_all` 5 s re-unfolding per conjunct,
`sequence_payload_co_wf₁` 7.6 s, knot wf kernel time 7–8 s); liveness
rung 2 (Paxos-scale quiescent completeness in chain form; two_pc commit ⇔
unanimous); the illegal-decision-record diagnostic (`batchCuts`/
`realizeTicks` silently return `[]` — report the first illegal entry per
member); the `.boundedValue` keyed-singleton batch operator (the Rust
simulator bug as a refused `cpl`); `SchedSem` against PRODUCTION DFIR
(record DFIR delivery/pacing, replay as a `…Sched` record, compare per-tick
outputs — user ruling: the production runtime, not the simulator); the
two engine limitations (post-`complete` `let` capture in a `fix` chain;
glue decision mis-extraction with overlapping records); the generator's
unused-simp-arg warnings; residual `]'` reads in `SequencePayload`; the
upstream report list (`SCHED_AUDIT.md`).

### D74 — `Project`: the program compiled to a typed dataflow graph, run as processes over TCP, and proved all the way to `paxos_safe_wired`; three compiler eta-expansions found in the IR

**Directive (user).** "Project" = a production interpreter: per-location functions
executable by wiring network in/out, stateful, with a correspondence proof (automatic
per module) that `k` concurrent projections wired with channels correspond to
`SchedSem`, `Values` by implication through the corner. Ruling: *grind until the whole
thing is pulled off against Paxos — correspondence proved all the way through, and 2
proposers + 3 acceptors actually executing over local TCP — then wait for review.* The
user was "not 100% confident in this approach" and accepted the observable-sharing
argument below as the reason it is a compilation.

**Why a compilation and not a fifth interpretation (ratified).** A tagless-final
instance of `HydroSem` sees operator applications; it cannot see the body's
`let`-sharing (it would duplicate a shared wire per consumer) and cannot step the body
of a `fix` once per global step — the knot's one-step delay is a property of the graph,
not of any operator. So `Project` is a phase of the `hydro def` pipeline
(`HydroGenProject.lean`, after `_param`), `HydroSem` is unchanged, and the relation to
the program is a theorem per module.

**What was built (all proven, standard axioms, zero sorries).**

- *IR* (`Project/IR.lean`): `Kind` (eight wire kinds), `Var Γ κ`, `HList` (reducible,
  nested products), `Node Γ κs` (32 constructors: each `HydroSem` method once, `call`,
  `fixN`), `Dag Γ outs` (`letN`/`ret`), `Dag.denote` in `SchedSem` with knots as
  `bekic`/`knotFix`.
- *The `_project` phase*: per module `M.dag`, **`M.dag_denote : ∀ binders env,
  Dag.denote pacing (M.dag …) env = (M (SchedSem …) … (env.get v₀) …).1 …`** and
  `M.wireCodecs : Dag.Codecs (M.dag …)` (a JSON codec per network node, synthesized
  from `ToJson`/`FromJson` of the message types — `Wire.lean`; `Ballot`/`LogValue`/
  `P1b`/`P2a`/`P2b` derive them, Rust's `#[derive(Serialize, Deserialize)]`). 44
  modules. Proof routes: leaf modules (27) by **kernel-judged `Eq.refl`**
  (`HydroGen.addThmByKernelRfl`: `Kernel.isDefEqGuarded` pre-check then `addThm` —
  `Meta.isDefEq` was 50× slower, a Meta `rfl` on the LE wrapper took 108 s); modules
  with callees/knots (17) in two hops — `M.dag_denote_graphs` (kernel; callee
  applications opened to component tuples of their own `Dag.denote`, knot groups as
  `bekic` cascades: `graphsTerm`/`openCallees`/`hlistComponent`/`schedTuple`) then
  `Meta.simp` with the callees' `dag_denote` (bodies opaque) and a closing kernel
  `Eq.refl`, assembled as `Eq.trans (Eq.trans hG hS) (Eq.refl T')`. The statement is
  over an ARBITRARY `env`: a literal `HList.cons` env put `Kind.sched`-reduced carrier
  types into the DiscrTree keys and the simp lemma never matched. Timings: hop 1 LE
  wrapper 4.5 s, `paxos_core` 3.5 s (the kernel re-evaluates the body a few times —
  queued), hop 2 ≤ 6.6 s. `proveByTac` runs under `linter.unusedSimpArgs false` and
  `withEnableInfoTree false` (zero generator warnings); `HYDRO_PHASE_DIAG=1` prints the
  per-module phase breakdown.
- *The lock-step machine* (`Project/Step.lean`; spec `ProjectCausal/Knot/StepSpec/
  StepOps1–5/StepMain`): `Node.step`/`Dag.step`/`Dag.run` with cells (`ChanCell` =
  pending queue + delivered count per network edge, deliver then enqueue; `PendingOf`
  for `tick_scan`; `sampleAdvance` for `sample_every`), **`Dag.run_spec`**: the run's
  per-step output increments are `envIncAt (Dag.denote …) k` — the graph's baked
  decisions are the delivery oracle.
- *The wired machine* (`Project/Wired.lean`, spec `Project/WiredSpec`, theorems
  `ProjectWiredMain/Inv/Run`, now `Project/Wired*`): `Sel L mem` (hosted members), `Dag.LocState sel d`
  (hosted cells; network cells at the receiver, one queue per sender), `Dag.Oracle`
  (cursor per hosted receiver per sender), `Dag.Sends`, `restrict`/`override`/
  `enqueue`/`enqueueLoc`, **`Dag.locStep`** (one process's step), predicates
  `OracleAt`/`Inv`/`ChanMono`/`Feasible`/`CursorsStill`/`Quiet`/`KnotsIdle`.
  **`Dag.step_wired`** (all 32 constructors; under `IdleEnv`, `OracleAt`, `Inv`,
  `KnotsIdle` the global step restricted to `sel` is `locStep`, the rest = `enqueue`
  of its sends, non-hosted increments idle), **`Dag.inv_step`** (`Inv`/`ChanMono`/
  `KnotsIdle` carry), **`Topology`/`Interleaving`** with `ValidAt` (idle others,
  feasible cursors, `CursorsStill`, *switch only when quiet*), `SettledAt`,
  **`view_active`/`view_idle`/`outputs`**. **`paxos_safe_wired`**
  (`Paxos/WiredSafety.lean`): processes running `paxos_core.dag (pacingOf ticks)
  .guarded …` under a valid interleaving from a settled `t₀` never commit two values
  at one slot — `outputs` + `paxos_core.dag_denote` + `StepHist.mono_le` +
  `paxos_safe_sched'`.
- *The wire* (`Project/Wire.lean`): `Dag.Codecs`, `WireMsg := Nat × Nat × Json`,
  `wireEncode`/`wireDecode` (network nodes numbered in graph order, callee graphs
  inline), `oracleAll` (deliver everything arrived), `quietLoc`.
- *The witness* (`WiredDemo.lean`, `lake exe wired`): five processes on
  `127.0.0.1:9400+p`, newline-framed JSON, dedicated tasks, one lock serializing
  activations (the run IS an interleaving), each activation = drain inbox →
  `enqueueLoc` → one tick step → settle until `quietLoc` (fuel 64); proposers inject
  their payload once they see themselves elected (the `ballots` output; a payload
  injected before election is dropped — the Rust semantics). Proposer 0 wins ballot
  0 and commits `(0, some 42)`; the log is slot-functional across processes, every
  switch was at a quiet process, and a single-threaded replay through the same pure
  `activate` reproduces every output. **60 activations in ≈ 0.5 s wall.**

**Design facts the model forced.** (1) Knot feedback is one global step late, so a
process must settle its own feedback before another is activated — `ValidAt`'s
`sel (k+1) = sel k ∨ Quiet`; the harness loops tick-less `locStep`s until `quietLoc`.
(2) The decisions baked into the graph are the oracle; a process's cursors must equal
what the run realized (`OracleAt`) and be feasible — the demo's oracle "deliver all
arrived" is one of the oracles the theorem quantifies over. (3) The initial state is
not in `Inv` (`defer_tick` fires its seed at step 0 for every member): processes start
from a settled lock-step prefix each computes for itself (`S0`). (4) Singleton wires
have no empty increment (an idle member re-emits its value) — `IdleEnv` constrains
list-like kinds only.

**The runtime leak, found three times in the IR.** The first run: a proposer
activation 3.3 s, acceptors 174 ms, the sixth activation never finished — call-by-name
over the graph's per-member closures (`Hosted sel ℓ → X`), D73's `n^depth` again.
`memo`/`memo2` (identity in the logic, a table in compiled code via `@[csimp]`) took
three rounds, each diagnosed from `.lake/build/ir/Hydro/Project/Wired.c` and a `gdb -p`
`thread apply all bt` of the hung binary (the work runs on a helper thread; `perf`
without call graphs only named the hot leaves — `Node.step` lambdas, `readShapeDelta`,
`PendingOf.push` — which first pointed at the wrong suspect, the global `S0`):
(i) **LCNF eta-expands local functions too** (`Simp/Main.lean` `FunDecl.etaExpand`;
ToLCNF "cases produces a function → local fun decl, eta-expanded"): `fun i => memoFin
(s i)` became `fun i j => (build table) j`, the inner table rebuilt per read — the
two-level `memo2` builds inner tables as array *elements*; (ii) the D73 trivial-
structure erasure: a one-field `MemoBox` was the function — `tag : Unit := ()`;
(iii) **a `match` arm whose type reduces to a function type is eta-expanded**:
`Node.memoState`'s `| .fold .., s => memo s` arm became `fun s i => (memoBoxImpl
s).fn i` — the arms return a two-field `StateBox` and `Node.memoState` projects outside
the match (`memoEnv` never had the problem: its arms return a product). The csimp
replacement runs after `macro_inline` (`ToDecl.lean`), which is why `memo` is a
`@[macro_inline]` projection of the box. After: a process step 0–1 ms; the TCP run
535 ms for 60 activations (the demo was tuned to a `prof` exe measuring per-node cell
forcing with `IO.monoNanosNow`, removed).

**Gotchas.** `Kernel.isDefEqGuarded` is pure — time with `IO.lazyPure`; "heartbeat
timeout at whnf" can come from the kernel; `simp` refuses goals not type-correct under
reducible transparency (`Node.State n` applied as a function) → `rw`/`congr`; `subst`
on `sel' = sel` eliminated the wrong variable → `rw`; `omit … in` not allowed inside
`mutual`; `Array.get!`/`[i]!` need `Inhabited` → an `idx` helper in IO; `IO.asTask`
for blocking socket loops needs `(prio := .dedicated)` (thread-pool exhaustion
deadlocks); `tick`/`state`/`complete` are Hydro keywords and cannot name binders;
`$!` after `cmd &` in the sandbox shell captured the wrong PID — `pgrep -x`; a
structure must be declared outside a `mutual` block of defs; stdout of a killed
binary is lost — `eprintln` for diagnostics.

**Per-file.** `Project/WiredMain` 66 s (the 32-constructor `step_wired` mutual block),
`Project/WiredRun` 12 s, `Project/WiredSpec` 3 s, `Project/Wired` 4 s, `Project/Wire` 4 s,
`HydroGenProject` 4 s, `Paxos/WiredSafety` 2 s; `LeaderElection` 83 s, `SequencePayload`
44 s, `HydroTickCheck` 42 s, `PaxosCore` 41 s; full build 3m31s / 843 jobs (D73: 3m12s /
823). New library code ≈ 7.4k lines (`Project*.lean` 5.9k, `HydroGenProject.lean` 1.0k,
`Wire.lean`, `Paxos/WiredSafety.lean`), harness 353 lines.

**Gate.** `lake build` 843 jobs green; zero sorries; no `implemented_by`/`extern`/
`unsafe`/`native_decide` in `Hydro/`; exactly three `@[csimp]` seams (`mk'_eq_mkImpl`,
`memoBox_eq_memoBoxImpl`, `memoBox2_eq_memoBox2Impl`), each kernel-checked; `AxCheck`
standard-three subsets, `sorryAx` 0, `ofReduceBool` 0 (new entries: `Dag.step_wired`,
`Dag.inv_step`, `Interleaving.view_active/view_idle/outputs`, `paxos_safe_wired`, the two
memo equations); `falsify` ALL PASSED, `explore` true+true, `paxos` OK, `twopc` OK,
**`wired` OK**; all twelve program censuses unchanged and six toys. Docs: `ARCHITECTURE`
§7b + module map + §9, `DOCTRINE` R7 (three seams, five exes) + new R10, `CORRESPONDENCE`
(the wired chain table; not-proven list; trust base), `SCHED_AUDIT` follow-up 4 (Project
is the vehicle for the production comparison), `ENGINE_NOTES` "Runtime" (the memo
tables), `GATE` (exe, envelope, AxCheck list, seam grep), `README`, `docs/06` + index.

**Queue after D74.** The hop-1 kernel re-evaluation of knot-wrapper bodies (make the
graphs term's `fixN` closure match the dag's inner shape, or a generic
`Dag.denote_call_ret` lemma); the remaining slow generated declarations (`body_co_rr_ex`
7 s, `_param_all` 5 s, `sequence_payload_co_wf₁` 7.6 s, knot wf kernel time) and LE's
600k override; a lock-step replay of the wired log (reconstruct the baked cursors from
the recorded deliveries — the `OracleAt` side of the demo as a check rather than an
argument); `SchedSem` against production DFIR with the graph as the replay vehicle
(`SCHED_AUDIT.md` follow-up 4); liveness rung 2; the decision-record validity
diagnostic; the `.boundedValue` keyed-singleton batch operator; `lake check` once the
toolchain is ≥ 4.35, the kernel-trust `decide` witness for 1×1 `paxos_core`, exe roots
into `Exe/`; the two engine limitations (post-`complete` `let` capture in a `fix`
chain; glue decision mis-extraction); residual `]'` reads in `SequencePayload`; the
upstream report list. Lint debt in the new files: unused-variable warnings in
`Project/Step`/`Project/StepOps2`/`Project/Causal` (three never-executed tactics at
`Project/Causal.lean:53,121`).

### D75 — Paxos throughput: independent machines, the acceptor log as Rust's `HashMap` (`Std.ExtHashMap`), and `K: Hash` at the operators — 2.3k → 31k commits/s

**The question (user).** "What is paxos throughput?" — and two corrections on the way
that reshaped the demo and the mirror: the D74 harness serialized activations under a
global lock ("why would we round robin — the whole point of paxos is there's no
coordinator, they are separate machines"), and the acceptor's log snapshot was a sorted
association list where the Rust has a `HashMap` ("I'm confused why you are using a
sorted list where the rust code uses hashmap").

**Independent machines (`WiredDemo.lean`).** Each process now loops on its own: drain
whatever arrived on its sockets, tick, settle, then flush this activation's sends (the
DFIR discipline — a tick runs to completion before outputs leave; it is also what makes
every concurrent run linearizable into an `Interleaving`: a step reads only its own cells
and inbox, so steps of different machines commute). An idle machine ticks on a local 1 ms
timer (the proposers' timeouts are timer-driven in the Rust too). The only shared state
is the harness's stop flag and a linearization counter read AFTER the drain, used by the
replay check — nothing blocks a machine. The leader (the proposer that saw itself elected
on the `ballots` output) injects client payloads; `--bench total batch` injects `batch`
fresh payloads per activation until `total` are in flight and stops when the leader has
seen them all committed. Every run is checked slot-functional across processes, settled
before every flush, and replayed single-threaded through the same pure `activate`.

**The log snapshot is a hash map (`Hydro/HMap.lean`).** `a_log.entries()
.fold(HashMap::new(), insert)` (paxos.rs:890) had been mirrored as
`bfold insertSorted []` — a canonical sorted assoc list standing in for `HashMap`
because a singleton's *value* needs decidable equality and extensionality, which
`Std.HashMap` lacks (bucket layout leaks into `=`). The toolchain has the right type:
**`Std.ExtHashMap`**, the quotient by contents (`ext_getElem?`, `DecidableEq`, the
`insert/erase/filter/map` API with lemmas; `toList`/`fold` deliberately absent). `HMap K V`
wraps it with the proofs' vocabulary — `(slot, e) ∈ m` membership, `entries : Multiset`
(a `Quotient.lift` of `Const.toList` via `Equiv.constToList_perm`), `mem_insert`,
`eq_of_mem_of_key`, `insert_foldOk` (the `NoKeyOverlap` fold obligation),
`mem_foldlOn_insert_empty`, and `toList` (slots ascending — unique keys make the sort
canonical: `List.Perm.eq_of_pairwise` on `a.1 < b.1`, so it is a function of the quotient)
for the wire (`ToJson`), `Repr`, and the one place the Rust orders it anyway
(`recommit`'s `flatten_unordered` → `bofList log.toList`, under the existing
`tassume_ordering` decision). `LogMap P nP := HMap Nat (LogValue P nP)`; `rcEntries` reads
`lg.2.entries`; the four `foldlOn_insertSorted` proof sites re-pointed; `#guard_eval`
literals use `HMap.ofList`. Per-tick acceptor cost O(n²) → O(n), like the Rust.

**`K: Hash` at the operators (`Hydro/KeyedHash.lean`).** Profiling the free-running
leader showed `collect_quorum`'s per-tick `fold_keyed` over the pending pool at
O(keys × pool): the machine's `keyedFoldAssoc` is `dedup` + a per-key `filter`. Rust's
`fold_keyed`/`join`/`anti_join`/`filter_not_in`/`reduce_watermark` demand `K: Eq + Hash`
*at the operator* (the keyed types carry no bound) — ratified, mirrored exactly:
`[Hashable K]` on `HydroSem.bkfold/bjoin/bantiJoin/bfilterNotIn/reduce_watermark` (nothing
on the carriers or the IR kinds), on `collect_quorum`/`collect_quorum_with_response`/
`join_responses` (Rust: `K: Clone + Eq + Hash`) and `two_pc` (Rust: `Payload: Hash`),
`Ballot` derives `Hashable`; `cq_safe_sched'`, `two_pc_safe_sched'`, `two_pc_once_sched'`
and `cq_live*` gain the hypothesis (statement-shape change, ratified in this exchange).
The machine's list definitions stay the semantics — every `Keyed.lean` proof untouched;
`…H` wrappers carry the instance and `@[csimp]` equations (kernel-checked) swap in
`Std.HashMap`/`HashSet` implementations: `dedupImpl` (last occurrence, one pass from
the right), `listFilterNotInImpl`/`listAntiJoinImpl` (a `HashSet` of the other side),
`listJoinImpl` (`groupRev`: the right side grouped by key, values in arrival order),
`keyedFoldAssocImpl` (`foldAcc`: accumulators per key + the dedup'd key list), and
`reduceWatermarkEagerImpl` — the per-tick payload batch of DFIR's `ReduceKeyedWatermark`
as one pass: group the kept arrivals by key, update every existing key, append the new
keys first-arrival-first (`fdGo`). Its equation `payloadsImpl_eq` goes through
**`assoc_ext`** (an association list with distinct keys is its key list and its
lookups) with `keys_foldl_payload` (the appended keys are `fdGo (keys m) (kept keys)`)
and the existing `lookup_foldl_payload`. The IR constructor `Node.reduce_watermark` gains
`iH` right after `iK` (the generator maps the field's arguments positionally, instances
explicit); ~15 `Project` pattern sites gain one `_`; `Node.step`/`locStep` call the `H`
wrapper. `kfold_early_stop`'s cell (`KFESCell`) now carries the unique-keys invariant and
emits with one `filterMap` (`kfesOutMap`) instead of re-`dedup`ing every key ever seen
(`kfesOut_spec` restated; `step_wired`/`run_spec` unchanged).

**Measured (2 proposers + 3 acceptors, independent machines, loopback TCP; every run
slot-functional and replay-reproducible).**

| payloads × batch | lock-step harness (start) | machines + `HMap` | + in-tick `Hash` | + `reduce_watermark` `Hash` |
|---|---|---|---|---|
| 1000 × 50 | 7.3k/s | 8.7k/s | 10.1k/s | **10.1k/s** |
| 4000 × 50 | 2.3k/s | 5.3k/s | 9.0k/s | **10.8k/s** |
| 10000 × 100 | 0.9k/s | — | 4.7k/s | **22.3k/s** |
| 20000 × 200 | — | — | — | **30.9k/s** |

Single-payload commit latency ≈ 3 ms (two round trips + the idle timer); election
≈ 15 ms; batch 1 is latency-bound at ≈ 350 commits/s. Leader step 1.3 → 3.6 ms over
20k slots (proportional to inbox size); acceptor step 0.8 → 12 ms at 20k slots × batch
200 — the remaining list costs are `gcBelow`/`filterEntries` and the snapshot's per-tick
O(log) re-fold (the Rust pays the latter too); bounding the log is the checkpoint path,
queued next. Not measured: real network latency; all five processes share one Lean
runtime.

**Method.** `perf record` named the hot leaves (`insertSorted@recommit…` 33% +
refcounting; later `List.find?`/`insertReduceList`; the Lean binary has symbols, no call
graphs); a per-node timer inside a throwaway `prof` exe (`stepDagT`, the `locStep`
recursion with `IO.monoNanosNow` per node) showed the Dag step itself at 0.3 ms with 97
nodes — the cost was proportional to the *inbox* (≈100 µs per inbound message), which
pointed at the pool fold. Three dead ends worth recording: `Std.HashMap` is not
extensional but `ExtHashMap` is (D75's first draft argued the sorted list was forced);
`List.dedup` keeps the LAST occurrence (`dedup_cons_of_mem`), so the machine's key order is
last-arrival — `fdGo` is first-arrival and the two meet only through `fdGo_congr`; and
`List.nodup_append`'s third conjunct is `∀ a ∈ l₁, ∀ b ∈ l₂, a ≠ b` in this toolchain, not
`Disjoint`.

**Gate.** `lake build` 845 jobs green (3m33s); zero sorries; no `implemented_by`/`extern`/
`unsafe`/`native_decide`; nine `@[csimp]` seams (`mk'_eq_mkImpl`, `memoBox*`,
`dedupH`, `listFilterNotIn`, `listAntiJoin`, `listJoinH`, `keyedFoldAssocH`,
`reduceWatermarkEagerH`), each kernel-checked; `AxCheck` 61 reports all standard-three
subsets, `sorryAx` 0, `ofReduceBool` 0; `falsify` ALL PASSED, `explore` true+true,
`paxos` OK, `twopc` OK, `wired` OK; all twelve program censuses unchanged.

**Checkpoints (`--bench total batch ckFreq`).** Wired as `paxos_bench.rs` wires them:
the harness's replica (colocated with the leader — `kv_replica.rs:81–104`) applies the
committed prefix and every `ckFreq` slots sends `next_slot` to the acceptors on the
existing sockets (`Line.ckpt`); an acceptor reduces the replicas' checkpoints by max
(`paxos_bench.rs:72–78`) and reads the result as `paxos_core`'s `a_checkpoint` input
each activation; `reduce_watermark` collects the log below it — the GC path the proofs
already cover (`LogCoversOrGC`, B4). The replay records the checkpoint read per
activation. With `ckFreq = 1000` the acceptor step is flat over the run: 10000 × 100 →
22.4k/s (acceptor 0.76 → 1.09 ms, was 0.6 → 6.5 without), 20000 × 200 → 34.5k/s
(1.16 → 1.24 ms, was 0.8 → 11.4), **40000 × 200 → 36.5k/s** (acceptor 1.19 → 1.31 ms,
leader 2.3 → 2.6 ms). Steady state.

**Where the bottleneck is now (`perf`, 40000 × 200 with checkpoints).** After making the
harness's post-run slot-functionality check linear (it was 42% of the profile — an
O(n²) `List.all` over 40k commits) and sending each batch encoded once and only to the
processes hosting a receiver (`Dag.wireReceivers`, the routing table; was 4 encodings
and 4 sends per batch): **46k commits/s** at batch 200, 32k at batch 100, 17k at batch
50. The profile is flat: ≈45% Lean runtime allocation/refcount churn (`lean_dec_ref_cold`
15%, `lean_del_core_other` 10%, `mi_malloc`/`mi_free` ≈17%, `lean_mark_mt` — the small
lists/pairs/closures a step builds and drops, objects crossing threads); ≈10% JSON on the
wire (`Json.compress`, UTF-8, the parser, `DTreeMap.insert`); ≈6% `lean_apply_*`; ≈3%
`List.reverseAux`/`Array.ofFn`; **the Dag interpreter proper ≈2%** (`locStep`,
`memoState`, `HList.get`); nothing grows with the log or the run. The leader is the
critical path (step 2.8 ms vs acceptors' 1.3 ms per 200-slot batch; cycle ≈4.2 ms).
The next levers are a binary codec (`Codec` is abstract; the Rust uses bincode for the
same reason) and `Array`-backed increments inside the machine (a representation change
in `SchedSem`/`Project`, proof-touching).

**The headline, slimmed (user: "very hard to read and make sure it isn't accidentally
vacuous").** `paxos_safe_wired` carried five Values-side objects (`cpV hcp ckSrc ckV
hck` — the corner's pools, inherited from `paxos_safe_sched'`), the machine inputs
twice (`cpS`/`ckS` and `hinp : I.inp = envIncAt …`), and the ticks twice. Now: the
environment is recovered from the interleaving's increments (`envOfInc`,
`Project/EnvOfInc.lean` — every per-step increment sequence is the increments of the
environment whose history is their concatenation; `envIncAt_envOfInc`,
`HTMono_envOfInc`), and a machine-only **`paxos_safe_sched`** builds the pools inside
the proof: inputs truncated at the horizon `T` are their own finite pools (`hcp` by
`mono_le`), the checkpoint register is the running `optMax` fold of its own value list
(`foldl_optMax_ckList`, needs the register monotone; `snapTrace_ascending` gives the
Values singleton's ascent for free since `optMax` is inflationary), and the commits of
the untruncated run are those of the truncated one at `T` by causality through the graph
(`paxos_core.dag_denote` + `Dag.denote_causal`). This closes SCHED_AUDIT F2 for Paxos.
The wired statement's hypotheses were then: `hnA`, the interleaving, `hI : I.pacing =
pacing`, `CkMono` (the checkpoint wire's own `Monotonic` typing — satisfiable by the
constant `none`; without it the acceptor's GC is unsound, B4), `SettledAt`/`ValidAt`
from `t₀`, and the two commits. Standard axioms. (D76 removes `hI`, `t₀`, `SettledAt`
and `ValidAt`.)

**Queue after D75.** Apply the `paxos_safe_sched` recipe to `cq_safe_sched'`/`two_pc_*`
(F2 fully closed); a binary `Codec` for the wire; `Array` increments; `gcBelow`/
`filterEntries` as one pass (they already are O(n); the constant); a `--bench` latency
histogram; then the D74 queue (lock-step replay of the wired log, the hop-1
kernel re-evaluation, the slow generated declarations, `SchedSem` against production
DFIR with the graph as the replay vehicle).

### D76 — A member that is not active is inert: `SchedSem`'s delivery, knot feedback and `defer_tick` gated by the member's pace (idle / stutter / tick); `paxos_safe_wired` for every interleaving, no settling

**The question (user).** Reading the D75 headline binder by binder: "why is `top`
arbitrary — shouldn't `hnA` restrict it?", "why do we need `SettledAt`?", "why does
being settled even matter for safety? even if there's pending stuff it should satisfy
safety", "I don't want more hypotheses of this form", "whenever a machine takes a step
all its ticks step too, does that help?", "defer_tick is lazy, it does not trigger
another tick on its own", "I'm confused why idle-ness is even relevant to a safety
argument". The first has a clean answer (`hnA` constrains `mem acc`, the cluster size,
a parameter of the program; a `Topology` was only a *placement* of members onto
processes and cannot change how many acceptors exist). The others exposed an artifact.
`Topology` itself was then dropped (user: "there should be one cluster member per process
during runtime … can we drop the extra generality?"): a process **is** a member,
`Member L mem := (ℓ : L) × Fin (mem ℓ)` (a cluster and an index; a Hydro `Process` is a
cluster of size 1), `Sel.single m` the process hosting exactly `m`, and `Interleaving L
mem Γ` says which member runs at each step. `Hosted (I.sel k) prop` in the headline is
then "a proposer index `i` with `I.who k = ⟨prop, i⟩`".

**Diagnosis.** The wired proof is a transfer: safety is proven once at `SchedSem`
(`paxos_safe_sched`), and `Dag.step_wired` identified a clocked step with the active
process's `locStep`, the other processes' cells untouched. That identification needs the
lock-step machine to have *nothing to do* at a sleeping process's members. It did not:
three `SchedSem` primitives moved content at a member regardless of whether it ticked —
`StepHist.deliver` (the cursor delivered whether or not the receiver ran),
`StepHist.shift` at `fix_stream` and the implicit step shift at `fix_tick` (knot
feedback advanced one *clocked* step, the "event-loop pass", though a sleeping process
makes no passes), and `defer_tick`'s seed (emitted at step 0 at every member, not at the
member's first tick). Every hypothesis of the D74/D75 wired theorem beyond `hnA` and
`CkMono` — `IdleEnv`, `OracleAt`, `Feasible`, `CursorsStill`, `Quiet` before a switch,
`Inv`/`KnotsIdle`/`ChanMono` (`SettledAt`), the settled prefix `S0` — was scaffolding
for that gap, and `SettledAt 0` was in fact false at `Dag.init` (`defer_tick`'s cell
starts unfired), so the base case `(t₀, hs)` was an assumption nothing in the tree
discharged. Morally the asleep run was always safe (a longer knot delay is latency), but
`SchedSem` had no run describing it, because knot propagation was governed by neither
pacing nor decisions.

**The change — inertness as the semantic principle.** `Sched.lean` gains `gateView p f`
(`0 ↦ []`, `t+1 ↦ if p (t+1) then f t else gateView p f t`): a ticking step sees the
source one step late (the `+1` floor stays), a non-ticking step repeats. The ungated
primitives are the special case `p = fun _ => true` (`gateView_true`). Then
`StepHist.deliver h p c` gates on the receiver's skeleton (`broadcast_closed`/`demux`
pass `pacing p i`), `StepHist.shift h p` and `fix_tick`'s iterate gate on the knot
member's (`fix_stream`/`fix_tick` pass `pacing ℓ j`), and `defer_tick init t i step =
if (tickSteps (pacing ℓ i) step).isEmpty then [] else init :: t i step` (the seed at the
first tick — positional tick consumers cannot tell; the `allTicks`/`sample_every`
boundary emits at tick time, which is Hydro's behaviour). Lemmas: `gateView_prefix`,
`gateView_mono(_le)`, `gateView_congr` (agreement below a horizon from agreement
strictly below it), `gateView_stable`. The two uncertain pieces — the operator
couplings in the transfer — went through on exactly the abstract properties they
already used: `deliver_view_cpl` ("a delivered view is a `take` of an earlier source
view": the gated view is either that or the previous delivered view, inductively such a
take), and the knot coupling, where the old *fixpoint equation* `fixSr.view (t+1) =
(sbody fixSr).view t` became the **fixpoint inequation** `co_fix_view_succ : fixSr.view
(t+1) <+: (sbody fixSr).view t` (equal at a ticking step; at a non-ticking step the knot
holds its previous view, itself such a prefix — `co_fix_stage_eq`), which is all
`co_fix_cpl` needs (`ListLe.of_prefix`); same for the tick knot
(`co_tick_fix_succ`, needs `TMono` of the body, already a `wf` conjunct). Causality
(`iterate_shift_congr`, `stab_iterate_shift/tick` — now by strong induction on the step
since the non-ticking branch recurses on the same iterate), monotonicity
(`TMono.tickshift`, `tmono_defer_tick` via `tickSteps_isEmpty_of_le`) and the liveness
kit (`deliver_fair_attains` now also needs `FairTicks` of the receiver — the right
fairness for liveness; `StabilizesAt.deliver_id` is the generous-skeleton special case)
were mechanical. The `hydro def` elaborator passes the knot's skeleton `fun j => pacing
ℓ j` to `CoStream.fixSr`/`vbody` and the tick twins (`KnotCtx.tickOf`, read off the loop
type's location). **The Paxos-specific argument was untouched** — `ensures.slot_functional`
lives at `Values`, and the sixty-odd lemma sites touched are all per-operator.

**The machine.** `Node.step` mirrors the gating: channels `ChanCell.stepG tick c sends`
(deliver per cursor if the receiver ticks, else `enqueue` only); `fixN` keeps *pending*
feedback (`knotFeed` releases it at ticking members and nothing elsewhere, `knotKeep`
holds a non-ticking member's and appends the body's new output); `defer_tick`'s cell is
`Bool × List σ` (fired?, buffered input while unfired — fires at the member's first
tick with the buffer). `Project/Knot`'s feedback equations are the gated ones
(`KnotKind.delayed pacing`), `Project/StepMain` carries `knotPendingAt`/`knotPrevPending`
with `delayed_feed`/`delayed_keep` (needs the knot's view to be a prefix of the body's,
`delayed_prefix`), `chanCellAt h p c t` reads the delivered count off the gated view.
`Dag.step_spec`/`Dag.run_spec` unchanged in statement.

**The wired layer, shrunk.** `Dag.locStep sel d ora Δ s` — no `tick` argument: a
process's step ticks the member it hosts — every `Tick` location of that member at once
(user: "whenever a machine takes a step all its
ticks step too"). `Dag.step_wired` (`Project/WiredMain.lean`, 5 → 4 hypotheses, 4 → 3
conclusions): `ctx.tick = sel`, `IdleEnv`, `OracleAt`, and **`Dag.Drained`** — the
multi-input tick cells (`zipTick`/`reduce_watermark` pair with an empty side,
`tick_scan` with no complete tuple, `sample_every` at its greedy position) hold no
complete item, the one condition on the state, and a *theorem* about every state the
machine produces (`Dag.step_drained` for any step from any state, `Dag.init_drained`).
`Inv`, `ChanMono`, `Feasible`, `CursorsStill`, `Quiet`, `KnotsIdle`, `inv_step` and
`ProjectWiredInv.lean` are gone. `Dag.cursorsAt sel d t` is the oracle reading the
graph's own cursors (`cursorsAt_oracleAt`), so the interleaving carries no oracle.
**`Interleaving top Γ`** is `who : Nat → Fin n` and `inp`; `I.pacing ℓ i k := top.sel
(I.who k) ℓ i`, `I.env k := envMask (I.sel k) (I.inp k)` (inputs at non-hosted members
masked, not assumed empty — a sleeping process cannot consume them; singletons
unchanged, they are read only at ticks), and `view_active`/`view_idle`/`outputs` hold
with **no hypotheses**. The headline:

    theorem paxos_safe_wired (hnA : mem acc ≤ 2 * f + 1)
        (I : Interleaving top (PaxosCtx prop acc))
        (sdec : PaxosCoreDec (SchedSem L mem I.pacing) …) (ssched : PaxosCoreSched (SchedSem L mem I.pacing) …)
        (hck : I.CkMono prop acc) {k k'} {i : Hosted (I.sel k) prop} {j : Hosted (I.sel k') prop} {s w w'}
        (hw : (s, w) ∈ (I.active (paxosDag prop acc f I sdec ssched) k).2.1.get Var.zero.succ i)
        (hw' : (s, w') ∈ (I.active (paxosDag …) k').2.1.get Var.zero.succ j) : w = w'

every binder commented in the source. `sdec` at `SchedSem` is all-`Unit` except the
heartbeat timing (`le.hb`: timer/pulse verdicts) — the user's question "why do we need
`sdec`? dec on sched is just unit" is right in spirit; reclassifying the timing family
(`SampleDec`/`TimerDec`/`PulseDec`) from the content bundle to the schedule bundle
would make `sdec` vanish from the statement and is queued.

**The harness.** An activation is **one tick**; no settle loop, no `quiet`, no `S0`
prefix (processes start from `Dag.init`; each fires its own `defer_tick`s at its first
tick). A knot's pending feedback is state the next tick consumes, and the next tick
comes with the next arrival — user: "won't all knots be over the network and thus you
don't need to immediately re-run until you get something from the network?" — or the
idle timer. `quietLoc` remains as a throughput hint only (stream knots; a tick knot's
pending is this tick's entry, present after every tick). The D74/D75 linearization
counter, activation log and single-threaded replay check are gone (user: "why do you
have a counter? each should be its own thread. I don't care about reconstructing a
linearization") — each process is its own thread sharing nothing but its sockets, keeps
its own commit record, and the harness reads the records after the stop flag; that the
run is an interleaving is the read-before-send argument of the DFIR audit, not a
check. `wired: OK`; `--bench 40000 200 1000`: **50k commits/s** (46k before).

**Gate.** Full build green (845 jobs); exes falsify/explore/paxos/twopc/wired all pass;
standard axioms only (`AxCheck` now lists `Dag.step_drained`, `Interleaving.step_wired`
in place of `inv_step`); the nine `@[csimp]` theorems unchanged; censuses unchanged.
Doctrine R12 ("a member that does not tick is inert"), ARCHITECTURE §7b,
CORRESPONDENCE, SCHED_AUDIT, GATE, README, docs/06 updated.

**DFIR audit of the inertness model (user: "does this overconstrain in a way that will
miss real executions? read dfir — it does not allow fixpoint within a tick anymore").**
Read against `dfir_rs/src/scheduled/context.rs` (the runner: one generated async tick
closure per process, `run_tick`/`run_available`/`run`), `dfir_lang/src/graph/meta_graph.rs`
(the closure: every subgraph once in stratum order, then the `defer_tick` double-buffer
swap, then `__end_tick`), `flat_to_partitioned.rs:187` ("Cyclical dataflow within a tick
is not supported" — intra-tick cycles are rejected at partition time; no fixpoint
iteration) and `hydro_lang/src/compile/ir/mod.rs` (`DeferTick` → `defer_tick_lazy()`,
never schedules a tick by itself; `CycleSink` → `identity()`, so a `forward_ref` cycle
closes through the network or not at all; all `Tick` locations of a process land in the
root location's one graph, `graph_mut` keys by `location.root()`, so they tick together).
Verdicts: *exact* — a runtime process **is** one cluster member, and its step ticks
every `Tick` location of that member at once (`Member`/`Sel.single` — co-location is no
longer expressible); knot feedback
consumed at the member's next tick and held while it sleeps (`defer_tick_lazy`'s back
buffer); `defer_tick` seed at the member's first tick (`__dfir_work_done` pre-set so the
first tick runs); inputs at a non-ticking member ignored (`source_stream` is polled inside
the tick); the `+1` floor — the deployed binary is `tokio::main(flavor = "current_thread")`
+ a `LocalSet` (`trybuild/generate.rs:234–242`), reader tasks run only between ticks, the
tick closure never awaits (`run_tick_sync` panics if it does), sinks push to the sink
task's channel, so a tick sees exactly what arrived before it began and nothing it sent;
one process per clocked step (each tick reads before it sends, so concurrent ticks
linearize losslessly); FIFO per pair (TCP `fail_stop`). *Model ⊇ runtime* — delivery
cursors (the runtime delivers everything arrived; the model any prefix), pacing (the
runtime ticks on arrivals/timers; the model whenever). Nothing the deployed runtime does
is outside the model. The one load-bearing assumption is the floor: a multi-threaded
executor or a tick that awaits mid-way could produce a same-tick round trip the model
excludes — not possible as compiled today; noted in `SCHED_AUDIT.md`. Semantics vs
runtime: Hydro's semantic model lets independent `Tick` locations batch independently;
`SchedSem`'s per-member pacing (unchanged by D76) is DFIR's special case — restricts
nothing for real executions.

**Stutter steps (`Pace`) — the over-constraint the DFIR audit found, and the fix (user:
"I like stutter steps").** The first D76 cut fused *clocked step* with *tick*: a member
either ticked (deliveries landed, knot feedback moved one step, tick bodies fired) or was
inert. Apply the user's stall test — "one model tick must have exactly the consequences
of one DFIR tick; the model may never need a tick nothing in production triggers" — to a
`forward_ref` knot closed through the network (every Paxos knot: `sequencing_max_ballot`,
`a_log`, `p1b_fail`, `i_am_leader`, `p_is_leader`). In DFIR the fed-back value is
consumed inside the *same* tick (the `CycleSink` is `identity()`, the subgraphs run in
stratum order); in the fused model the knot's one-step shift needed a *second tick* of
the member, which production never triggers (`defer_tick_lazy` schedules nothing, and
a member with nothing arriving does not tick). A real over-constraint: executions in
which the fed-back value is used before the next tick were outside the model.

The fix separates the two: a member's pace at a clocked step is `Pace.idle`,
`Pace.stutter` or `Pace.tick` (`Sched.lean`; `Pacing L mem := (ℓ : L) → Fin (mem ℓ) →
Nat → Pace`, with `pacing.active` / `pacing.ticks` the two Bool projections and
`Pacing.ofTicks` for the tick-only tests). `SchedSem`'s delivery (`StepHist.deliver`),
knot shift (`StepHist.shift`, `fix_tick`'s `gateView`) are gated by `active`; `tickSteps`
and `defer_tick`'s seed by `ticks`. `StepCtx` is `⟨t, tick, act⟩`; `Node.step` gates
channel delivery and knot feed/keep on `act`, `defer_tick` on `tick`. `Dag.locStep sel
tk`: the hosted members are active, and tick iff `tk` (`tk = false`: `snapshot`,
`batch`/`batch_ordered`, timers and `defer_tick` keep their state and emit nothing;
channels, knots and every stream operator fire). `Dag.step_wired ctx sel tk (hact : ctx.act
= sel) (htick : ctx.tick = fun ℓ i => sel ℓ i && tk)`. `Interleaving` gains `tick : Nat →
Bool`; `I.pacing ℓ i k` is `tick`/`stutter` at the running member per `I.tick k`, `idle`
elsewhere (`ctx_act`, `ctx_tick`); `I.active d k := locStep (I.sel k) (I.tick k) …`. The
wired headline's binders are unchanged except that `I` now says whether each step ticks.

What a DFIR tick is in this model: **stutters until the member's asynchronous dataflow
is quiet, then a tick** — exactly the stratum-ordered sweep followed by the `defer_tick`
swap and `__end_tick`. `WiredDemo` takes one `locStep … true` per activation (every
Paxos knot closes over the network, so within a process the feedback is consumed at the
next activation either way; the stutters matter for the *theorem's* coverage, not the
harness's throughput). Stutters also make the model what Hydro's semantics says and DFIR
specializes: asynchronous streams progress independently of any tick. The `+1` floor is
untouched.

Queued (user agreed): a *linearized* per-process `Project` — forward refs reordered after
their producers, network nodes split into receive/send halves, no knot cell, one pass per
tick — and a bridge theorem "one linear tick = stutters-until-quiet, then a tick" in this
model, so the executable process is the DFIR shape and the lock-step model stays the
proof's.

**`paxos_safe_sched` moved home.** The machine-only headline (the Values pools built
inside the proof from the inputs truncated at the horizon; `CkInput`, `CkMono`,
`StepHist.truncate`, `optMax`, `ckList`, `foldl_optMax_ckList`) had been parked in
`Paxos/WiredSafety.lean` since D75. It is the statement to read — `paxos_safe_sched'`'s
`cpV`/`hcp`/`ckV`/`hck` are the coupling's shape leaking into a theorem, not facts about
a run — so it now sits directly after `paxos_safe_sched'` in `Paxos/CoupleWf.lean`, the
primed one is documented as the transfer lemma, and CORRESPONDENCE / docs/06 / README /
ARCHITECTURE quote the unprimed statement. `WiredSafety.lean` holds the wired headline
only.

**Leftovers removed.** The D74/D75 scaffolding D76 retired: `ChanCell.step_idle` /
`step_saturates` / `step_delivered_ge` / `step_delivered_le` (the `ChanMono`/`Feasible`
facts), `prevChanCell_zero/succ`, `scanRun_length`, `prevView_length_le`,
`incView_eq_drop`, `Kind.emptyDelta`, `Kind.loc`, `Sel.single_self`, `WiredDemo.emptyInp`,
the `rfl` restatements `gateView_zero` / `deliver_view_zero/succ` / `shift_view_zero/succ`,
and `Dag.quietLoc` / `Node.quietLoc` / `KnotKind.locEmpty` / `knotsLocEmpty` (the settle
loop's exit test; user ruling: delete, reintroduce with the linearized `Project` bridge,
whose "stutter until quiet, then tick" needs exactly that predicate). The Project modules
moved to `Hydro/Project/`.

**Queue after D76.** The linearized `Project` and its bridge theorem (above); the
decision bundles out of the wired statement (done in D77); the D75 queue
(`cq_safe_sched'`/`two_pc_*` by the `paxos_safe_sched` recipe, binary codec, `Array`
increments); the D74 queue (lock-step replay of the wired log constructing an
`Interleaving` value, the hop-1 kernel re-evaluation, `Dag.prune` per location).

### D77 — The schedule is execution data: the compiled graph carries no decisions, `Dag.Sched` is a field of the interleaving, and `paxos_safe_wired` takes one execution object

**The question (user).** "why does paxos_safe_wired take ssched? isn't the idea that
we're writing a theorem over concurrent stepping of projected?" — and, on coverage, "I
want to make sure the proof isn't too weak, that it actually gives correctness
guarantees for true distributed executions (under TCP + fail-stop) — if there are
multiple senders we need to capture different orders of which one arrives first because
each sender to receiver is a separate TCP".

**Coverage, answered first.** Per-pair FIFO is `TransportDec p c := Fin p → Fin c → Nat →
Nat` with `StepHist.deliver` taking a *prefix* of each sender's stream; latency and loss
are a cursor that stays put; cross-sender arrival order is independent cursors per
sender, appended per step in sender order (`mergeN`) — and with D76's stutters every
interleaving of per-pair FIFO streams is reachable at tick granularity (deliver B at a
stutter, A at the next step: the tick's batch is `[B, A]`); concurrent ticks linearize
losslessly because a tick reads before it sends (the `+1` floor); separate channels
(P1a, P2a from one proposer) are separate nodes with separate cursors. Model ⊇ runtime;
"runtime ⊆ model" is the D76 DFIR audit, argued not proved. The one *structural* defect
was the statement's shape.

**The defect.** The compiled graph baked the program's `…Dec`/`…Sched` bundles into its
nodes: `Node.broadcast_closed`/`demux` carried the `TransportDec`, the timer nodes
their verdict lists, every other decision its (`Unit`) value — because `dag_denote`
equates the graph to the *term* `paxos_core (SchedSem …) … sdec ssched`, where the
schedule is an argument. So `paxosDag I sdec ssched` was "the program at this future
delivery schedule", the wired theorem pinned the processes' oracle to the baked cursors
(`Dag.cursorsAt`), and the processes themselves never read them (`Dag.locStep` takes an
oracle; `WiredDemo` built `D` with dummy cursors and ran `oracleAll`). Coverage held
only via "for every run there exists a reproducing `ssched`" — the quantifier on the
wrong side of the dag.

**The fix.** Decisions are execution data, not program data.

- IR: `Node.broadcast_closed`/`demux` lose `d`; `sample_every`/`timeout_snapshot`/
  `source_interval_batch` lose their verdict lists. `Node.Sched n` / `Dag.Sched d`
  (`Project/IR.lean`): per network node `Fin (mem p) → Fin (mem c) → Nat → Nat`, per
  timer node its `SampleTimes`/`TimerVerdicts`/`TimingPulses`, per callee or knot the
  body's, `Unit` otherwise. `Dag.denote pacing d sch env`, `Node.step ctx n sch Δ s`,
  `Dag.run d sch pacing inp`, `Node.stateOf`/`prevState`, `NodeSpec pacing n sch env t`,
  `denote_causal`/`denote_tmono`, `step_spec`/`run_spec` all thread `sch` (mechanical;
  `rw` needs the knot/call lemmas stated with `sch : Node.Sched (.call N args)`, not
  `Dag.Sched N` — `kabstract` matches types at instance transparency).
- Wired: `Node.Oracle` gains the timers (`Hosted sel ℓ → List Bool`/`List Nat`);
  `locStep` reads `ora i` where it read the baked list; `OracleAt sel n sch ora t` /
  `cursorsAt sel d sch t` (the schedule cut to the hosted members, `cumMax` for cursors);
  `Drained n sch s`; `oracleAll sel d sch s` (cursors online, verdicts from the schedule).
- `Interleaving (d : Pacing L mem → Dag Γ outs)` with fields `who tick inp` and
  `sched : Dag.Sched (d (pacingOf who tick))` — one execution object. The graph is
  given *per pacing* because `M.dag` still takes `pacing`: not through any decision
  (those are gone) but because the program's data terms (`bchain`/`bmax` over
  `BoundedStream` carriers) are typed at `SchedSem L mem pacing`; `pacingOf who tick`
  closes the loop at the structure's own fields. `I.active k` reads
  `Dag.cursorsAt (I.sel k) I.dag I.sched k`.
- Generator (`HydroGenProject.lean`): a node argument whose type is
  `TransportDec`/`SampleDec`/`TimerDec`/`PulseDec` leaves the node and becomes its
  `Node.Sched` component; a `Unit`-typed argument becomes `()` (kernel: unit eta). The
  graph binders are the ones the body still mentions, closed under their types
  (`keepMask`; `GenInfo.dagKeep`); every dropped decision binder — a record, a bare
  decision, a `Unit` — is a *bundle* (`GenInfo.schedIdxs`). Emitted per module:
  `M.schedOf : ∀ binders, Dag.Sched (M.dag kept)` (the components as a right-nested
  tuple along the chain, callees' as `C.schedOf`), `M.recordOf : ∀ binders', Dag.Sched
  (M.dag kept) → bundle₁ × … × bundleₙ` (built from the structures' fields: a field
  whose projection path matches a component is that `Prod` access, a sub-bundle passed
  whole to a callee is `C.recordOf`'s component, `Unit` fields are `()`), and
  `M.schedOf_recordOf : M.schedOf b[bundles := M.recordOf S] = S` **by kernel `rfl`**
  (`Prod` and `Unit` eta). `dag_denote` becomes `Dag.denote pacing (M.dag kept) (M.schedOf
  all) env = M all gets`, kernel `rfl` as before; the hop-1 chain writes
  `Dag.denote (C.dag kept) (C.schedOf all) ⟨wires⟩`. All 12 programs and the toys
  regenerate (`paxos_core.recordOf` rebuilds `PaxosCoreDec × PaxosCoreSched` through
  `leader_election`/`sequence_payload`/the knot bodies).
- `paxos_safe_wired (hnA) (I : Interleaving (paxosDag prop acc f)) (hck : I.CkMono …)
  (hw hw') : w = w'` — no `sdec`, no `ssched`. The proof sets `sdec/ssched :=
  paxos_core.recordOf … I.sched`, rewrites `I.sched` by `schedOf_recordOf`, and applies
  `dag_denote` and `paxos_safe_sched` as before. `paxos_safe_sched` (CoupleWf) uses
  `Dag.denote_causal` at `paxos_core.schedOf … sdec ssched`.
- `WiredDemo`: `D := paxos_core.dag pacing0 .guarded .prop .acc 1` (no bundles),
  `sched := paxos_core.schedOf … sdec ssched` for the timers; `oracleAll … sched`.
  Throughput unchanged (44–48k commits/s).

**What this closes.** SCHED_AUDIT F2b (new, closed): the wired headline quantifies over
the execution's deliveries and timer verdicts directly; the "`sdec` out of the headline"
queue item (D75/D76) falls out — `PaxosCoreDec` is no longer anywhere in the wired
statement. Gate: build green, standard axioms (`paxos_safe_wired`,
`paxos_core.schedOf_recordOf` in AxCheck), censuses unchanged, exes OK.

**Queue after D77.** The loop body as the library's (done in D78); the linearized
`Project` and its bridge theorem; make the compiled graph pacing-free (done, D79); the D75 queue (`cq_safe_sched'`/`two_pc_*` by the
`paxos_safe_sched` recipe, binary codec, `Array` increments); the D74 queue (lock-step
replay of the wired log constructing an `Interleaving` value, the hop-1 kernel
re-evaluation, `Dag.prune` per location).

### D78 — The loop body is the library's: `Dag.iterate` is what a process runs per activation, and `paxos_safe_loop` is the headline over interleavings of it

**The ask (user).** "I understand we can't prove an actual distributed execution on
machines, but I'd like to minimize the gap between what runs in production and what is
proved — the loop that pulls in events and executes the graph; make that loop body
provided by Hydro and strengthen the formal guarantee to an interleaving of those
iterating loop bodies. This would require the proof to be over the actual projections."
And on the shape: the TCP/FIFO assumption "wouldn't show up as a hypothesis, right?" —
right: the inbox is *defined* from the senders' sends and the cursors, so order,
no-loss and monotone arrival are by construction, not premises.

**The gap before.** The theorem was over `I.active k = Dag.locStep …` — the projected
graph, the actual process state — but everything *around* `locStep` in the harness was
hand-written and unspecified: decode → `enqueueLoc` per sender → `memoState` →
`locStep` with `oracleAll` → encode. And the shape differed: the lock-step model
enqueues a sender's messages at the sender's step; a real process reads its sockets at
its own activation, from several senders and several of their steps at once.

**The loop body (`Project/Process.lean`).** `Dag.Inbox m d`: per network node, per
hosted receiver, per sender, the items that arrived since the last activation (a
`demux` already filtered by address). `Dag.enqueueInbox`; `Dag.inboxOfSends m src d
snd` (a sender process's `Sends` as this process's inbox) with `enqueueLoc_eq :
enqueueLoc = enqueueInbox ∘ inboxOfSends`. `Dag.arrivedOnly`: the loop cells — every
queue's pending part dropped, delivered counts kept (a process keeps nothing in flight;
the network does). **`Dag.iterate m tk d sch ib Δ s`**: append the inbox, deliver
everything (`oracleAll`), `locStep`; returns cells, outputs, sends — to
every peer *and to itself* (self-messages travel through the network like any other,
which is what DFIR does over loopback TCP; the harness pushes them into its own
accumulator). There is **one** such function: the processes run `Dag.iterate` and the
theorems are about `Dag.iterate` (an earlier draft had a proof-side core and a
harness-side wrapper; the user struck that — "I want the proofs to say as much as
possible about actual production code"). The shell's only contribution is the inbox,
`Dag.inboxOf m d (decoded : List ((src : Member) × Dag.Sends src d))` — the sends it
decoded, per sender, in arrival order, folded with `Dag.Inbox.append` from
`Dag.Inbox.empty` through `inboxOfSends`; `memoState` sits inside `iterate` as an
identity (`Dag.memoState_eq`). `WiredDemo.activate` is `Dag.inboxOf` + `Dag.iterate`
plus codecs; the harness around it is sockets, line framing, the accumulator, threads.

**The bridge.** `Dag.arrivalsOf sel d ora s`: the inbox the cursor delivers next, read
off the lock-step cells (`pending.take (c − delivered)` per queue). Per node and graph,
by mutual recursion (`iterate_locStep`, three conjuncts: cells after `arrivedOnly`,
outputs, sends): `iterate` on `arrivedOnly s`, fed `arrivalsOf (cursorsAt …) s`,
*is* `locStep` at the cursors on `s` — the one computation is `ChanCell.loop_step`:
the loop cell `⟨arrivals, delivered⟩` stepped at `oracleAll`'s count delivers exactly
the arrivals, reaches the same `delivered`, and leaves only this step's sends pending —
which `arrivedOnly` drops. `arrivedOnly_enqueueLoc`: receiving does not change the loop
cells. At the interleaving (`Project/Loop.lean`): `I.loopView k m := arrivedOnly (I.view
k m)`, `I.arrivals k`, `I.loopActive k := iterate … (I.arrivals k) … (I.loopView k
(I.who k))`; **`loop_active : I.loopActive k = (I.loopView (k+1) (I.who k), (I.active
k).2.1, (I.active k).2.2)`**, **`loop_idle : m ≠ I.who k → I.loopView (k+1) m =
I.loopView k m`**. So a process *is* `iterate` folded along its own activations
with the computed inboxes, and a lock-step cell at a process is its loop cell plus the
network's in-flight queue.

**The headline.** `paxos_safe_loop (hnA) (I : Interleaving (paxosDag prop acc f)) (hck)
(hw : (s, w) ∈ (I.loopActive k).2.1.get Var.zero.succ i) (hw' : …) : w = w'` — by
`loopActive_outputs` and `paxos_safe_wired`. Same three binders; the commits are the
loop body's outputs. And at a *time* rather than at two steps (user: "instead of who is
active at k / k', have a single point in time and query the outputs at loc1 and loc2"):
`Interleaving.outputsAt T m` — the output increments process `m` has produced by step
`T`, one per activation — and `streamOutputsAt T m v i` (the items a stream output has
carried), with `mem_streamOutputsAt` locating the activation; then
`I.commitsAt T i` and **`paxos_safe_at (hnA) (I) (hck) (T) (i j : Fin (mem prop)) (hw :
(s, w) ∈ I.commitsAt T i) (hw' : (s, w') ∈ I.commitsAt T j) : w = w'`** — at any time,
any two proposers' commit logs so far agree on every slot. No `who`, no step, no
`Hosted` in the statement.

**What remains outside the theorem**, now one sentence about the shell: *the inbox the
sockets hand to `iterate` is the computed one* — the connection delivered the sender's
bytes in order and the accumulator (one FIFO per connection, drained at activation)
dropped and reordered nothing within a connection; plus JSON framing (`wireEncode`/
`wireDecode` round-trip is not proved — derived `ToJson`/`FromJson` carry no lemma).
That is the TCP + fail-stop assumption, on the I/O code, not in the statement.

Not changed: the loop body is one tick per activation (D76 ruling); the DFIR-shaped
"stutter until quiet, then tick" is the queued linearized-`Project` bridge, which will
be a different body under the same `iterate` signature.

Gate: build green (847 jobs), standard axioms (`iterate_eq`, `loop_active`,
`loop_idle`, `paxos_safe_loop` in AxCheck), censuses unchanged, exes OK; `wired: OK` at
47k commits/s with self-sends through the inbox.

**Doctrine (user, general): the model follows production, never the proof** — R13.
The deviation it caught, and the fix: `locStep` used to append a *hosted* sender's sends
straight into the receiver's own queue at the same step (so that `restrict ∘ step =
locStep`; the lock-step machine enqueues every send at the sender's step) — loopback
handled differently from every other connection — and `Dag.iterate` then dropped them
(`arrivedOnly`) so they would travel through the network like DFIR's over loopback TCP:
"enqueue, then drop, then loop back", a proof device on the executable path, and a
hand-written shell obligation (push own sends into own inbox) the proof could not see.
Now `locStep` only delivers (`ChanCell.step … []`) and returns its sends; the hosted /
remote split lives solely in the correspondence: `step_wired`'s first conjunct is
`restrict ∘ step = override … (enqueueLoc sel sel (locStep …).sends (locStep …).cells)` —
the lock-step cut is the process step with its own sends put into the model's network
queue (`ChanCell.step_enqueue`). `view_active` reads the same way. `Dag.iterate` is
enqueue inbox → deliver everything → `locStep` → send, nothing else; `arrivedOnly` is a
proof-layer projection only (`loopView`), and `iterate_eq`'s left side is the raw
step. The lock-step model (`Step.lean`) is untouched.

**One headline.** `paxos_safe_loop`/`paxos_safe_at`/`commitsAt` folded into
`paxos_safe_wired`, stated directly with the framework's `Interleaving.streamOutputsAt`:
`(hnA) (I) (hck) (T) (i j : Fin (mem prop)) (hw : (s, w) ∈ I.streamOutputsAt T ⟨prop, i⟩
Var.zero.succ ⟨i, _⟩) (hw' : …) : w = w'`.

Aside (user): `Sel`/`Hosted` — a process as a *set* of members — is D74 generality that
D76 made redundant (every process is `Sel.single m`); collapsing it to `Member` is a
large mechanical simplification. **Done (user: "let's collapse").** `Sel L mem`
(`(ℓ : L) → Fin (mem ℓ) → Bool`) is gone; the process parameter of every wired
definition and theorem is `m : Member L mem` (`Project/Wired.lean`), `Hosted m ℓ :=
{ i : Fin (mem ℓ) // ⟨ℓ, i⟩ = m }` is the member's index at `ℓ` (the Prop, so the
headline's hosted witness is `⟨i, rfl⟩`), and the lock-step machine's Bool activity
mask is `Member.hosts m ℓ i := decide (⟨ℓ, i⟩ = m)` (`hact : ctx.act = m.hosts`,
`pacingOf`). `Interleaving.sel`, `Sel.single`, `Sel.single_self/_disjoint` are deleted
(`I.who k` is the process; `view_idle`'s hypothesis is `m ≠ I.who k`;
`Hosted.ne_of_ne`). `DecidableEq L` is now required wherever a member is decided
against (the activity mask, `overrideDelta`, `enqueueLoc`'s "is this my sender", the
memo tables) — every `L` is a finite enum, so this is free; `[DecidableEq L]` is scoped
so the Kind-level lemmas stay instance-free. Binder names: `sel`→`m`, `selS`→`src`.
Build, exes, censuses, AxCheck unchanged.

**Ruling (user, after D79): the linearized formulation stays.** The per-process
alternative was designed — `History m d` (tick/inp/inbox per activation, cells the fold of
`Dag.iterate`), one existential clock `time : Member → Nat → Nat` (per-process strictly
increasing; the Lamport/physical reading), and the network as one line per connection,
`received m src x n <+: sentBefore src m x (time m n)` (FIFO prefix: order, no
duplicates, loss and delay allowed, loopback by `time m n' < time m n ⇒ n' < n`); the
bridge would sort events by `(time, member)` into an `Interleaving` and prove
`I.arrivals (rank m n) = inbox m n` through a queue invariant (pending of `(i ← j, x)` =
sent-before minus received-so-far), a second `step_wired`-sized structural proof. It would
restate the hypotheses in the shell's vocabulary (no `who`, no `T`: a commit is witnessed
by the activation that emitted it) without changing the conclusion. Parked: the global
sequence *is* the linearization argument, stated once in prose
(CORRESPONDENCE, docs/06), and D79 made it an interleaving of *the* graph.

**Queue after D78.** (Parked, see ruling above:) the linearized `Project` and
its bridge theorem (the DFIR tick as the loop body); a proper client in `WiredDemo` — the main program as the client/replica:
it learns the leader from the proposers' `p_is_leader` output over the wire, sends
payloads to it, applies commits and checkpoints the acceptors (today the proposer's
shell injects payloads itself behind a once-leader flag and the replica is colocated
with it — client behaviour baked into the proposer's thread; user: leave as is for
now); the D75 queue (`cq_safe_sched'`/`two_pc_*` by
the `paxos_safe_sched` recipe, binary codec, `Array` increments); the D74 queue
(lock-step replay of the wired log constructing an `Interleaving` value, the hop-1
kernel re-evaluation, `Dag.prune` per location).

### D79 — The compiled graph is pacing-free

**Ruling first (user), on the DFIR tick.** The queued "DFIR tick as the loop body"
(stutter to quiescence, then tick) is dropped: DFIR runs one pass per tick and does not
iterate to a fixpoint outside ticks either, so `Dag.iterate` — one `locStep`, ticking or
stuttering — already *is* the loop body. Nothing to add.

**The residue.** `M.dag` took a `pacing : Pacing L mem` binder in `H`'s place, and
`Interleaving (d : Pacing L mem → Dag Γ outs)` instantiated it at the interleaving's own
pacing (`I.dag := d I.pacing`). Inspection of the generated terms: `pacing` occurs in
`M.dag` only as the instance `SchedSem L mem pacing` inside tick-body payloads
(`HydroSem.BoundedStream H …`, `bmap H …`, `bsZip H …` — 60 occurrences in
`acceptor_p1.dag`), never in a way that affects the value: `paxosDag p = paxosDag q` is
`rfl` (171 ms). Elaboration residue, carried by every statement about the wired machine.
It would have blocked the linearized `Project` (a per-process history has to type its
cells `Dag.LocState m d` before any global pacing exists).

**The fix (generator).** `Pacing.idle : Pacing L mem := fun _ _ _ => .idle`
(`Sched.lean`). `genProject` substitutes it for the `pacing` fvar in the assembled graph
body before `keepMask`, so the binder is dropped (`dagKeep`), callee `C.dag` applications
drop it too (`keptArgs`), and `M.dag_denote` keeps quantifying `pacing` — the kernel
judges `Dag.denote pacing (M.dag kept) (M.schedOf pacing …) env = M (SchedSem L mem
pacing) …` with the graph at `Pacing.idle` and the denotation at `pacing`, as before
(all 44 modules, no route change). `M.schedOf`/`M.recordOf` keep `pacing`: the decision
records are typed at the instance. Downstream: `Interleaving (d : Dag Γ outs)` with
`sched : Dag.Sched d` and no `I.dag`; `paxosDag prop acc f` without a binder;
`paxos_safe_sched` uses `paxos_core.dag … .guarded prop acc f`; `WiredDemo.D`/`codecs`
without `pacing0` (`pacing0 := Pacing.idle` remains for `schedOf`). All 12 censuses,
exes, AxCheck unchanged.

### D80 — `cq_safe_sched` and `two_pc_safe_sched`/`two_pc_once_sched`: inputs unconstrained

**Ruling (user).** The wired/process layer is Paxos-only ("I don't think we need to
worry about wired for cq or two_pc"); the machine-level headlines of the client-quorum
stage and 2PC are upgraded to the `paxos_safe_sched` shape instead.

**Before.** `cq_safe_sched'` took a Values-side pool `v` with `hc : ∀ T i,
↑((h i).view T) ≤ v i`; `two_pc_safe_sched'`/`two_pc_once_sched'` took `cpV`/`hcp` and
stated "proposed once" on `tpcProposed cpV`. Those are the shapes `CoStream.inputC`
needs, not facts about a run.

**Now** (`Std/Quorum.lean`, `TwoPC.lean`): the unprimed theorems take only the machine
inputs. Recipe as in `paxos_safe_sched`: truncate the input at the horizon
(`StepHist.truncate`, moved to `Sched.lean`), carry the output back to the untruncated
run by causality through the graph (`collect_quorum.dag_denote`/`two_pc.dag_denote` +
`Dag.denote_causal`, one output component for `two_pc`, the first of two for
`collect_quorum`), and feed the primed theorem the truncated input with its own view at
`T` as the pool (`Multiset.coe_le` from the prefix). Statements:
`cq_safe_sched … (hk : k ∈ ((collect_quorum (SchedSem …) () h mn mx ()).1 i).view T) :
∃ d, mn ≤ cqOkCount (cqConsumed ↑((h i).view T) (d i)) k`;
`two_pc_safe_sched … (hp : p ∈ (two_pc (SchedSem …) … cpS sdec ssched c).view T) (hone :
(tpcProposed fun c => ↑((cpS c).view T)).count p = 1) : ∀ j, vote j p = true`;
`two_pc_once_sched` likewise with `≤ 1` both sides. `tpcPoolAt`/`tpcPoolAt_above`/
`two_pc_truncate` are the pieces. AxCheck: the three new names (standard axioms). The
primed theorems stay as the transfer lemmas.

**Cleanup after D80 (review of `Interleaving`, user).** Reviewed `Interleaving`
(`Project/WiredRun.lean`) and the loop layer (`Project/Loop.lean`) for honesty about a
projected execution; verified: no same-step self-delivery (`locStep` steps channel cells
before the new sends exist), idle processes inert (`view_idle`/`loop_idle`), timers index
the node's own tick count or trace position (no node reads `ctx.t`), `arrivals` is a FIFO
prefix of pending, `loopActive` is literally `Dag.iterate`. Generous, not dishonest:
stutters never happen in `wired`; register inputs (`Kind.sing`) are supplied for every
member at every step but read only at a member's ticks — this is why `inp` stays a global
delta masked by `envMask` rather than "the running member's increments": a register has
no bottom to put at the idle members (`SingBound`/`ValueOrder` have none). Docstring now
says so. Cosmetic: the `Sel`-era "hosted members" plural removed from the `Project`
docstrings; the last four section-variable lint warnings in `WiredMain` closed (`omit`).
Timers' place in `Sched`/the wired layer is slated to change (user: no decision in the
schedule for timers), so the `Sched` → cursors × timers split is not done.

### D81 — The benchmark vocabulary: in-tick ops for keyed state, the keyed-stream tick leaf, wall-clock/tick-count decisions, `tfold`/`kfold`/`ksnapshot`/`latest_monotone`, and the class-size tax cured

**Scope.** Mirror `hydro_std/src/bench_client/mod.rs`, `hydro_test/src/cluster/{paxos_bench,
paxos_with_client}.rs`, `kv_replica/{mod,sequence_payloads}.rs` 1:1. The programs needed
vocabulary the signature did not have; every piece landed as an any-body truth, toys first (R6).

**Rulings (user, checkpoint #1 of this entry), verbatim where they were given.**
- C1 the wall clock: `use::time(nondet!(…))` — a new `sliced!` style delegating to
  `Tick::current_tick_instant` (tick.rs:309; the slice half of GH #2393) — and in Lean
  `H.time dec.clock` (`ClockDec`: `ClockReadings n := Fin n → List Nat` in both worlds, a
  timing decision). Rust `bench_client` mod.rs:100–128 restructured into ONE `sliced!`
  block ("(a)": the `Instant::now()` inside a `fold` closure and inside a `map` were impure
  reads the type system could not see).
- R-b the non-commutative reduce: `tassume_ordering` (per-tick order selection) then a keyed
  reduce, not a `commutative = manual_proof!`.
- K the acceptor checkpoint knot on the TICK wire: `fix` on an `Atomic` + `latest_monotone`,
  ascent as the knot invariant, `requires mem replicas = f + 1` (the `== f + 1` gate).
- T `tfold` (`across_ticks(|s| s.fold(…))`); S the `TickShape.kstream` leaf (`kyield`);
  F closure flattening: `bench_client` takes the `protocol_inputs`/`protocol_outputs`
  wires and returns `(ids_and_prev_payloads, latencies)`; the closures' cycles are `fix …
  complete` knots at the callers; `sequence_payloads` takes the `r_next_slot` wire.
- "do remaining missing Rust APIs faithfully".
- The histogram (user interjection): "won't the histogram blow up in memory… port the
  Rust hdrhistogram" → `Hydro/Std/BenchClient.lean` `namespace HdrHistogram`: a faithful
  port of hdrhistogram 7.5.4 `Histogram::<u64>::new(3)` (sub-bucket magnitude 11, bucket
  = `log2 v + 1 − 11`, equivalent range `2^bucket`, `lowestEquivalent`/`highestEquivalent`,
  sparse counts `HMap Nat Nat` with the no-zero/stored-keys-are-classes invariant,
  `recordN`/`record`/`add`/`len`/`valueAtQuantile`), `record_comm : FoldOkP … record`,
  `add_comm_ok`, ToJson/FromJson (normalizing).
- The budget: "I accept (a) for now" — `PaxosCore`'s `set_option maxHeartbeats 250000`
  stays as a measured fact (215M raw) and the residual `HRel.Laws`-scaled tax is queued.

**Engine A (in-tick ops, 21).** `boIsNone boZip boFilterMap boIntoStream bsFilterIf
bsFilterMap bmin breduce bsort bcrossOptional bkmap bkcrossSingleton bkchain bkweakenOrder
bkreduce bksJoin bksFilterKeyNotIn bksKeyCount bksIntoStream boZipSingleton bsZipOptional`,
each with its Values/Sched/corner/HRel laws. The tick parser takes `(state s :
H.BoundedKeyedSingleton K V)` seeded empty (`use::state_null::<KeyedSingleton>`) and the
`.kstream` leaf (`AtomicKeyedStream`, 5 holes) with `kyield`. The elaborator was split
(`parseTickState`, `tickInputShape`, `mkStepLam`) to stay under the LCNF compile budget.
`CoupleProj.coerce_kstream` is kept OUT of the simp lists (as `coerce_stream`);
`embed2_kstream_coerce_wf` matches it. Toys `toy_bench_ops`, `toy_kleaf` (HydroTickCheck).

**Engine B (stream-level ops, 9).** `kentries kmap kallTicks kfold ksnapshot tfold
latest_monotone time snapshot_bounded` + `TicksDec`/`ClockDec`; helpers `latestOf optMax
runMax AscendingLatest latestMonoCell tfoldTrace` (Trace.lean); corner `wf`s carry the
content halves (`latest_monotone.wf := t.wf ∧ ∀ i, AscendingLatest (t.rr i)`, discharged
by a `ghost pre`; `kfold.wf` per-key `Pairwise Pre`; `tfold.wf` per-batch); `coLegOfProj`/
`coTickScanProj` learned the `CoAtomicKStream` leg (an rr-route `whnf` hang when a
`kyield` output was consumed). Project layer: one `Node` ctor per op (`StepOps6.lean`;
`.time`'s `Node.Sched` is the readings), `kfold`'s machine state is the raw arrival list
(hashed fold recomputed per step — incremental cell queued), `latest_monotone`'s the
current latest, `time`/`snapshot_bounded`'s the tick count. Toy `toy_bench_wires`
(5/0/0) through every phase incl. project; `#check @toy_bench_wires_param₁`.

**The class-size tax, root cause and cure.** Adding fields to `HydroSem` made every module
slower; the measured reason was not the class but the `co_simp`/`co_wf_simp` macros and
`HydroGenKnot.simpTerm` rebuilding ~300-lemma simp sets per call. Cure: `Hydro/SimpAttr.lean`
registers `co_simp_rules`/`co_wf_simp_rules`; `CoupleProj.registerCoRules` loads
`coSimpRules`/`coWfSimpRules` into them once; the macros are `simp only [co_simp_rules,
coTickScanProj, …]`; `simpTerm` starts from `ext.getTheorems` when the list carries
`coSimpBase[0]` (`coSimpBaseSet`). Full fresh build 4m50s → 2m51s; LE 89 s → 77 s (600k
fits); SP needs no override (46 s); PaxosCore 48 s, still 215M → 250k (ruling above).

**Engine C (what the whole-program modules forced; each an any-body truth).**
- *Glue mono resolves operator-headed legs* (`HydroGenKnot.resolveRel`): a derived wire
  (`H.allTicks (H.flattenOrdered (H.mapTick …))`) is resolved by the operator's own
  `vmono_<op>` law (MonoHRel, now imported by HydroGenKnot; `opHeadOf` sees the op through
  `MemberMap.get`/`Subtype.val`/`.proj`), hypotheses recursively (depth 6 → 16). Before:
  such legs were skipped with a placeholder — `paxos_core_mono₁` (the new-leader output)
  did not exist, and `kv_replica` had no mono lemma at all; `with_client`'s knot needed both.
- *Glue vdec with bare content decisions*: `collectValuesDecArgs` also collects a callee's
  bare `OrderSelDec`-typed arg; `extractGlueVdec` matches a content prim field by type
  (ambiguity = two fields of one decision type → the meet refuses; documented).
- *`hydro_wf_close` applies a callee's lemma only when the callee HEADS the subject*: it
  used `find?` (the callee anywhere in the subject), so `(collect_quorum (… kv_replica (…
  paxos_with_client …) …)).1.wf` tried `kv_replica_co_wf₁` — `apply` unifying two module
  applications through their knots: **3.6 G heartbeats, 318 s**, for one leg. Head rule:
  8.6 M, 0.8 s.
- *`rwRhs`'s `kabstract` at reducible transparency*: the outer knot's `wf` generator spent
  419 M heartbeats (36 s) in one `kabstract` — every same-headed `.rr` subterm an `isDefEq`
  whnf-ing two modules. The occurrence is syntactic: `withReducible`. 3.5 s.
- *Dead chain lets dropped* (`dropDeadLets`, after instantiation): a body `let` only the
  wire closing consumed (`with_client`'s `payloads_at_proposer`) was re-spelled as a dead
  `have` re-applying its callee — the production graph carried a second copy and
  `schedOf_recordOf` failed.
- *Nested `fix` names*: an inner fix's chain body is `<M>.<wire>.body` (the elaborator's
  `fixPassMode` tells nesting). The inner-fix hoist still assumes the def's binders are the
  context, so `paxos_bench` keeps one flat knot per module instead (below).
- *Knot binders from the RAW body*: a binder only a continuation-kept chain `let` mentions
  (`paxos_bench`'s `aggregator`) was not a knot binder (the fvar analysis read the
  zeta-reduced wire-closing body).
- *`τ requires R` without `ensures`* (`requiresTerm`): a module using a fact about its
  inputs with no face of its own.
- *Keyed-stream arguments of knot modules*: `CoKStream.lower`/`lowerC` (+ `co_kstream_lowerC_sr/rr`),
  `valuesRelRefl` for `.keyed`/`.ksing`.
- *`addThmByKernelRfl` judges once*: the `Kernel.isDefEqGuarded` pre-check doubled the
  kernel's work on the production-graph lemmas (`paxos_bench`: 25 s of 50).

**Gotchas canon.** Positional `Laws` constructors in declaration order; `/-- -/` before
`namespace`/`open … in` is a parse error; `yield`/`emit`/`kyield` take ONE parenthesized
comma list; `set_option … in` goes BEFORE the doc comment; Mathlib has no `LinearOrder
(Option α)` (`WithBot` + `LinearOrder.lift'`) nor `LinearOrder (α × β)` (`Lex`); a `do`
`(← e)` inside `withDeclDiag "…" <| f (← e)` runs OUTSIDE the diag (lift it first);
`intros` then `rename_i` in generated-binder proofs (the `invariant` step's binder list
carries the listed lets, their images, and the callees' faces).

### D82 — The benchmark programs: `bench_client`, `compute_throughput_latency`, `aggregate_bench_results`, `sequence_payloads`/`kv_replica`, `payloads_to_leader`/`paxos_with_client`, `acceptor_checkpoint_from_replicas`/`tag_with_sender`/`paxos_protocol`/`paxos_bench`

**Modules (census nondets/scheds/fuels, all build-enforced).** `Std/BenchClient.lean`:
`bench_client` 6/0/0, `compute_throughput_latency` 2/0/0, `aggregate_bench_results` 3/2/0.
`Paxos/KvReplica.lean`: `sequence_payloads` 1/0/0, `kv_replica` 1/0/2 (`KvPayload`,
`SequencedKv` with `LinearOrder` via `LinearOrder.lift'` into `Lex (Nat × WithBot (Lex (K ×
V)))` — Rust's `Ord` compares `seq` only, SCHED_AUDIT). `Paxos/PaxosWithClient.lean`:
`payloads_to_leader` 3/2/0 (the `with_client` closure as the function Rust would write),
`paxos_with_client` 17/7/6 with `ensures PWCEnsures` (= `paxos_core`'s slot-functionality
on its one output, by the replayed `ghost have hcore`). `Paxos/PaxosBench.lean`:
`inc_i32_workload_generator` 0/0/0, `tag_with_sender` 1/0/0 (paxos_bench.rs:38–43 as a
function), `acceptor_checkpoint_from_replicas` 1/1/0 (the `a_checkpoint` block, :69–96)
with `ensures ACQEnsures : nR = f + 1 → ∀ i, AscendingLatest (out i)`, `paxos_protocol`
21/10/9 (the protocol closure `|input| { … }`, :32–120; the acceptor-checkpoint knot with
`invariant acceptor_checkpoint (kv a_checkpoint) => (mem replicas = f + 1 → ∀ i,
AscendingLatest …)`, base the empty trace, step the block's `ensures` at this stage's `kv`;
`requires variant = .guarded ∧ mem replicas = f + 1`), `paxos_bench` 32/12/11 (the two
`bench_client` cycles as one knot over the keyed streams' pair spellings; `requires` the
protocol's clause).

**The proof that was owed: the checkpoint minimum ascends.** `paxos_core` reads the
acceptor checkpoint as `Monotonic`; in Rust it is an `Optional` (no such bound) computed as
`snapshot.entries().filter_if(key_count == f + 1).map(seq).min()` over `reduce(max)` per
replica. `checkpointQuorumMin_ascends` (PaxosBench.lean): along cumulative views `M ≤ M'`
(`KeyedPool.kviews_le`) of the acceptor's per-replica pool, once the gate opened every key
is present (`Finset.card_eq_iff_eq_univ` with `nR = f + 1`), later views keep every key
(`Multiset.map_le_map`), raise each key's running maximum (`readAt_grp_prefix_of_le` +
`foldl_keepLarger_prefix_le`), so the minimum rises (`foldl_minStep_spec`:
what `.min()` returns is a member below every member). Lifted to the trace by
`AscendingLatest.of_pairwise` (`List.pairwise_iff_getElem` + `Pairwise.filterMap`) through
the tick readers `ha_checkpoint_run`/`ha_checkpoint_at`. The knot invariant then needs only
the callee's `ensures`; `latest_monotone`'s `ghost pre` is `hacceptor_checkpoint_inv hreq.2`.
With MORE than `f + 1` replicas the gate closes forever once all report and a late replica
could lower the minimum — the `requires` is the honest premise (SCHED_AUDIT row).

**Where the mirror bends, and why (each said in the program text).** The closures are
functions (`payloads_to_leader`, `tag_with_sender`, `paxos_protocol`, ruling F); the
measurement lines after `bench_client` returns sit inside `paxos_bench`'s knot body (the
engine's knot continuation carries no decisions — an engine queue item, ENGINE_NOTES);
`kv_replica`'s two tick cycles are one `fix`; `BenchValue nC := Lex (Fin nC × Int)` (the
Rust tuple's `Ord`, needed by the KV sort); `demux` addresses by `Fin.val`.

**Rust landed in this tree.** `hydro_lang/src/live_collections/sliced/style.rs`: the `Time`
style, `use::time(nondet)` → `Tick::current_tick_instant` (tokio-gated; not the first `use`
of a block). `hydro_std/src/bench_client/mod.rs:100–128`: one `sliced!` block — `use::batch`
of the inputs' and outputs' `entries()`, `use::time`, `use::state_null::<KeyedSingleton<u32,
_, _, _>>()`; `assume_ordering::<TotalOrder>` (one in flight per virtual client) replaces
the `commutative = manual_proof!` of a reduce that overwrote; `cross_singleton(now)` stamps
inputs and outputs; `join_keyed_singleton`, `filter_key_not_in(keys())`, `into_keyed_stream
().chain(…).reduce` keep one start time per in-flight request. `bench_client` is tokio-gated.
`cargo check`/`clippy`/`fmt` clean; `hydro_std` tests 11/11; `hydro_test` `paxos_ir`,
`two_pc_ir` (snapshots regenerated: the client's dataflow), `paxos_some_throughput`,
`two_pc_some_throughput`, `paxos_log_some_throughput` pass.

**Witnesses (R7).** The whole stack runs: `lake exe bench` (D83). (A `Paxos/BenchCheck.lean` of
per-module `#guard_eval`s — the checkpoint minimum `[none, some 5, some 7]`, `kv_replica`'s
`[⟨0,10⟩, ⟨1,11⟩]`/checkpoints `[0, 2]`, `payloads_to_leader`'s `[7, 8]` — was the interim witness
and was removed once the benchmark ran end to end; user: "you can get rid of benchcheck now that
it's actually runnable".) Exes unchanged: `falsify` all PASSED, `explore` true/true, `paxos` OK,
`twopc` OK, `wired` OK.

**Cost (measured, `scripts/heartbeat_table.sh Hydro/Paxos/PaxosBench.lean`).** Full fresh
build 852 jobs ≈ 5 min (the three new program files ≈ 2 min of it): `PaxosWithClient` ≈ 30 s,
`KvReplica` 10 s, `PaxosBench` ≈ 140 s — `paxos_protocol` 44 s (def-elab 11, project 17),
`paxos_bench` 96 s (project 56 s: `dag_denote`'s graph judgement 25 s + close 25 s —
production-graph lemmas at whole-benchmark nesting; queued). Command totals 122 M
(`paxos_protocol`) and 85 M (`paxos_bench`) raw heartbeats — under the 200k default, so
**no new `maxHeartbeats` site** (still LE 600k, PaxosCore 250k).

### D83 — The benchmark end to end: `lake exe bench` runs `paxos_bench` as ten processes over TCP

**Ruling (user).** "i want the benchmark to run end-to-end."

**What runs (`BenchDemo.lean`, `lake exe bench [nClients ckFreq durationMs]`).** `paxos_bench.dag`
(`.guarded`, `f = 1`, `num_replicas = 2`) as 10 processes — 2 proposers, 3 acceptors, 2 clients,
1 aggregator, 2 replicas (`PBLoc`, `pbMem`) — each stepping `Dag.locStep` on its own cells
(`Dag.iterate`, the `wired` runtime of D74–D78 unchanged), JSON lines over
`127.0.0.1:9500+p`, delivery read online (`Dag.oracleAll`). The graph has no inputs: the
clients generate the load. The aggregator prints Rust's `pretty_print_bench_results` lines
(`Throughput: N requests/s`, `Latency p50 | p99 | p999 ms (samples)`, the percentiles by
`Histogram.valueAtQuantile` of the merged `HdrHistogram` port). Exit 0 iff a request completed.

**The clock and the timers are the execution's, read online.** The schedule a process steps
with is rebuilt at every activation from the process's own `Timing` (`paxos_bench.schedOf …
(decOf tm) ssched`): the wall-clock reading (µs) appended at each tick (`ClockDec` —
`SchedSem.time d = (d i).take ticks`, so one reading per tick keeps the trace exact), the
interval pulse (`PulseDec`: elapsed ≥ 200 ms at the clients, ≥ 1000 ms at the aggregator),
the proposers' heartbeat verdicts (constant lists of length 200 000, built once). Content
decisions are `Unit` at the machine. No change to the Project runtime was needed: `Dag.iterate`
takes the schedule per call, and `Node.Sched` for `.time` IS the readings.

**Open loop vs closed loop (user: "i thought we were at 50k/s").** D75's ≈ 35k commits/s is
`wired --bench`: `paxos_core` alone, the leader injecting 200 payloads per ≈ 1 ms activation.
`bench` is the Rust benchmark's closed loop — one request in flight per virtual client, so
throughput = in-flight ÷ round trip, and a round trip is ≈ 6 process hops (client → proposer →
acceptors → proposer → replicas → client), each one activation of the whole `paxos_bench` graph
(≈ 3× `paxos_core`'s node count: the base step is ≈ 0.6–0.7 ms on every process, the restricted
nodes still visited). Checkpoints work end to end (without them the acceptor step climbs to
12 ms; every 10 slots it is flat).

**Measured (16 cores, 6 s runs, checkpoints every 10 slots).** Before the hashed client ops:
8 in flight ≈ 350 requests/s (p50 19 ms), 200 ≈ 5.5k, 1000 ≈ 8k, 4000 ≈ 9k — the client's step
grew from 1.3 to 19 ms with the in-flight count: `SchedKMap.join` looked every key up linearly
and `keyedReduceAssoc` grouped by a per-key filter, O(n²) a tick in `start_times`' size. After
(`KeyedHash.lean`, the D75 pattern): `keyedReduceAssocH`/`SchedKMap.joinH`/`filterKeyNotInH`,
the same lists with `@[csimp]` hash-backed implementations (`keyedReduceAssocH_eq_impl`,
`joinH_eq_impl`; `filterKeyNotInH` over the existing `listAntiJoin`); `bksJoin`/
`bksFilterKeyNotIn` gained `[Hashable K]` (Rust's `K: Hash`, R11) — definitionally the old
fields, no downstream proof changed. **1000 in flight: ≈ 22k requests/s, p50 41 ms, p99 49 ms**,
the client step flat at ≈ 1.1 ms; 4000 in flight: 18.8k, p50 129 ms — the leader saturates
(step 1.7 → 4.1 ms), the next ceiling (queued 3: skip non-hosted nodes in `locStep`).

**One harness lesson (the first runs degraded 2× in 5 s).** `clock := fun _ => tm.readings.toList`
rebuilt the readings list at EVERY read — and the schedule is read on every process at every
step (the restricted node still applies `d i`), so every process paid O(ticks) several times per
activation. `Thunk`s in `decOf` (built at most once per activation, only on a process that reads
them) and constant heartbeat lists: step times flat. The clock reading itself is O(1) per tick.

**Still queued.**
1. **Higher-order `hydro def`** (ruling, user: "let's leave this clean up queued — to support
   higher order hydro def"): `bench_client` should take the generator and the protocol as
   module arguments and own its `protocol_outputs` cycle as the Rust does; today it takes the
   two wires and `paxos_bench` ties the knot (ruling F). A module-typed argument's naming/wf/
   mono/causal facts become hypotheses of the generated lemmas, and `bench_client.dag` a
   parametric call node. The closure BODIES stay `hydro def`s (ruling, user: "the closure bodies
   should be hydro def").
2. **`paxos_with_client_safe_wired`** — its one output is the commit stream (`PWCEnsures`); the
   `paxos_safe_wired` shape needs the `paxosCoDec`/`paxosVDecG`-style scaffolding of
   `CoupleWf.lean` written for `paxos_with_client`. `bench` observes no commits (they are internal
   to `paxos_bench`); its witness is the aggregated throughput/latency.
3. **The per-activation base cost**: every process steps every node of the graph (a non-hosted
   node's cells are `Unit` but the walk visits it) — ≈ 0.6 ms a step for `paxos_bench`; skipping
   non-hosted nodes in `Dag.locStep` needs `step_wired`'s per-node cases to say the skipped
   node's cell is unchanged. The leader's own step (≈ 2 ms at 1000 in flight) is the other half.
4. Engine: knot continuations carrying their own decisions (field-wise vdec at the module when
   the knot consumes the record whole and the continuation's callees consume fields); glue
   modules with their own primitive content ops (today: factor into a callee, as
   `tag_with_sender`); nested `fix` hoisting (the inner body's context); `kfold`'s incremental
   machine cell; the production-graph lemma cost at depth; a `ClockReadings` carrier a process can
   extend in O(1) (today a `List`, hence the per-activation `toList`).
5. SCHED_AUDIT upstream rows added: the `== f + 1` liveness gate; `Optional` without
   `Monotonic` (now proven where the Rust assumes it); `Ord for SequencedKv` ignores `kv`;
   `Instant::now()` inside closures (closed by `use::time`).

### D84 — A process reads its clock once: timing at the machine is a total per-tick script, the production interpreter holds no history, and the 30 GB `bench` leak

**Scope (user).** "It's super weird that it's a list in prod, we should fix that first" —
after reviewing how `Sched` models `use::time`: `ClockDec n := ClockReadings n := Fin n →
List Nat`, the `.time` node's production oracle `Hosted m ℓ → List Nat`, the bench shell
pushing every reading onto an `Array` and rebuilding the whole decision record
(`decOf tm`, `tm.readings.toList`) at each activation. Then: "you should also fix the
`sample_every` memory blowup", and "we should take the long way if it eventually results in
a cleaner result".

**The audit (checkpoint #1).** The list is forced by the Project layer's types, not by
`Values`/`Sched`: `Node.Sched (.time) = ClockReadings`, `Node.Oracle m (.time) = Hosted m ℓ
→ List Nat`, `Dag.iterate` taking a whole `Dag.Sched`, `oracleAll` copying the lists through;
`locStep` indexing `(d i)[s i]?` with the node's tick counter. Same for
`timeout_snapshot`/`source_interval_batch` (`List Bool`; the demos' `hbDec` were constant
lists of length 200 000 and 64 — the `wired` demo silently stopped getting heartbeat verdicts
after 64 ticks). `sample_every` was worse: its cell held the ENTIRE input trace `Trace
(Option α)` because a `SampleTimes` entry may index any past tick. `Values` and `Sched` were
right as they stood: at `Values` the Atomic carrier *is* the per-tick trace and the list's
length is the slice's tick count; at `Sched` `time d i step = (d i).take (ticks so far)`.

**Measuring the baseline found the dominant defect.** `bench 500 10 6000`: **30.5 GB max
RSS**; 12 s: 55 GB; 20 s: OOM-killed (61 GB box); step times 0.75 → 2.0 ms across every
process; not content-proportional (1 virtual client/node: 39 GB) — ≈ 0.5 MB retained per
activation. Root cause: `lookupHosted f tbl h` (`Project/Wired.lean`), the memo table's stored
closure, captured **`f`** for a `none` fallback that can never fire (the table is total at
the hosted members). `f` is `fun i => s i ++ Δ.get x i`-shaped — a closure over the previous
cells and increments, themselves such closures — so every cell retained every past state of
everything it reads. Fix (within the sanctioned `@[csimp]` seam; the equations' meaning
unchanged): `hostedTable f` with `hostedTable_get : tbl[i] = some (f i)` at hosted `i`, the
lookup `Option.get` under that proof — the closure captures the table and nothing else.
Two traps on the way, both now in `ENGINE_NOTES`: `hostedTable fun i => (memoFinBoxImpl (f
i)).fn` is a lambda returning a function, eta-expanded by the compiler, so the inner channel
tables were rebuilt per lookup and the demo spun at 600 % CPU forever (tabulate the boxes,
project inside a `@[noinline]` `lookupHosted2`); and `Option.get` on `none` is a field
projection of a scalar — undefined behaviour, a spin, not a panic — so the totality proof is
load-bearing for the compiled code. After: **325 MB** at 6 s, 56k activations (was 35k), 94k
requests (was 66k); the 20 s run completes at a steady 20–23k req/s, 361 MB.

**Design rulings (user), with the reasoning that produced them.**
- *Option (iii) over (i)/(ii).* The first proposal typed the process oracle `Option Nat` to
  keep `iterate (timingAt sch s) = step sch` an equation for every schedule — because
  `Sched`'s list could be shorter than the pacing's ticks, a tick without a reading. The user
  asked why a `Nat` oracle needed a side condition; the answer located the artifact: `Sched`
  reused the denotation's list type, admitting a configuration that has no counterpart in
  Rust (`current_tick_instant` always returns) nor at `Values` (where a shorter list is fewer
  ticks). Ruled: **remove the artifact at the source** — at `Sched` the timing decisions are
  total scripts of the tick index, `TickScript n β := Fin n → Nat → β` (`SampleDec`,
  `TimerDec`, `PulseDec` at `Bool`, `ClockDec` at `Nat`); `time d i step = scriptTrace (d i)
  (ticks so far)`; `sample_every t fl = famHist (fun step i => sampleFlags (t i step) (fl i))`
  — the present latests at the flagged ticks of the realized trace. `Values` keeps its lists
  (its traces); the corner derives them from the scripts and the pacing at the horizon
  (`scriptDerive (pacing.ticks ℓ) Td d`, `sampleDerive fl Td t.sr := flaggedBelow (fl i) (t.sr i
  Td).length` — the `snapshot_bounded` pattern), with `cpl` by `scriptTrace_prefix` and
  `sampleFlags_le_derived` (a prefix trace's flag samples are a prefix of the derived-index
  samples of any longer trace). The generator reclassifies the four timing families as
  machine data **and** content (`DecFam.isContent`, `schedPass`; `valuesPass` is now empty):
  the script is an outer binder, the `Values` list the corner's `∃`-witness — the same route
  `snap`/`batch` take. Consequence recorded honestly: a `…Dec (SchedSem …)` record changes
  TYPE (lists → scripts); the headlines' hypotheses (`sdec`) change shape with it; the
  realizable behaviours are the same and the spurious ones (fewer verdicts than ticks) are
  gone — `TransferChecks` §6 now reads `pulsesSkew 0 4 = [true, true, true, true, true]` (5
  ticks by step 4; it used to be 4 of a 4-element list).
- *The process receives one entry per tick.* `Node.Oracle m` for `.time` is `Hosted m ℓ →
  Nat`, for `timeout_snapshot`/`source_interval_batch` `→ Bool`; `Dag.Timing m d` is what
  the shell hands over per activation (`TickReadings m` — clock/timeout/pulse/sample per
  location — builds it: `Dag.Timing.ofReadings`); `Dag.oracleOf tm s` is the online oracle
  (cursors from the queues, timers from `tm`); **`Dag.iterate m tk d tm ib Δ s` takes no
  schedule**. `Dag.timingAt m d sch s` reads each script at the node's tick count — the one
  seam — and `oracleAll sch s := oracleOf (timingAt sch s) s` by definition.
- *Why the proof needs no side condition and no `Option`.* A run's per-tick readings `r₀,
  r₁, …` define the total script `fun k => r_k`; at the process's k-th activation its counter
  is `k`, so `timingAt sch s = r_k` — exactly what the shell handed it. Every per-tick sequence
  is the restriction of some total script, so the universal statement over `I.sched` covers
  every run. The user also asked whether the IO code must guarantee a monotone clock: no
  theorem in the tree rests on a clock fact (`CORRESPONDENCE.md`'s `Instant::now()` row); if
  one ever does, it enters as a `requires` on the program and a listed trust-base item about
  the shell, exactly like the FIFO channels — recorded in `CORRESPONDENCE.md`.
- *`sample_every`'s oracle is `Hosted m ℓ → Nat → Bool`, not `Bool`* ("take the long way if
  it is cleaner"): the flags of the ticks the sampled trace gains this activation, numbered
  from 0 (`sampleFlagsFrom (d i) Δ 0`; the shell passes a constant). A bare `Bool` would make
  the bridge need "an Atomic trace gains ≤ 1 entry per step" as an invariant of every
  Atomic-producing node — and that is FALSE in the model: `defer_tick`'s trace is
  `init :: t` at the first tick (two entries in one step). The lock-step cell is the tick
  count (`Nat`, was `Trace (Option α) × Nat`); the bridge is `sampleFlagsFrom_shift`.
  `Drained` loses its sampler clause; `sampleAdvance`/`samplePos`/`sampleAtOpt_split` are gone.

**The proof chain, link by link (unchanged in structure).** `M.dag_denote` (kernel `rfl`,
untouched) → `spec_time`/`spec_timeout_snapshot`/`spec_source_interval_batch` are one shared
lemma `scriptTrace_ticks_inc` (the increment of `scriptTrace sc (ticks k)` at a tick is `[sc
(prev ticks)]`) plus `tickSteps_length_ite` — the `take`/`[k]?`/`toList` juggling is gone;
`spec_sample_every` is `sampleFlags_append` under `famFreeze_eq_of_mono` → `OracleAt` gains the
lock-step state (`ora i = d i.1 (s i.1)`; a timer is indexed by the member's ticks, not the
global step) and `cursorsAt` the process cells; `wired_time` and the three timer cases of
`step_wired` are today's with `[d i.1 (s i.1)]` on both sides → `iterate_locStep`'s timer
cases stay `⟨rfl, rfl, rfl⟩` (`timingAt_enqueueInbox`: the inbox touches only queues) →
`Interleaving.oracle k := cursorsAt … (I.view k …)`, `Interleaving.timing k := timingAt I.sched
(I.loopView k …)`, `loop_active` as before → `paxos_safe_wired` unchanged in shape.

**Generator.** Timing families content+machine-data: outer binders for every `schedPass`
field, the ∃-skeleton's `()` only at transport (`skeletonOf`). New: `recordFlowsWhole` — the
"unused dec field ⇒ pin to `default`" heuristic looked for projection constants in the body
and so pinned the fields of a record a glue module passes WHOLE to a callee (`toy_step`'s
`dec` → `toy_relay`); a bare occurrence of the record's fvar now counts as using every field.
The toys `toy_relay`/`toy_step` took two primitive decisions (a `BatchDec` and a `PulseDec`)
— two content units — and are now house style (`ToyRelayDec { batch, pulse }`, nested in
`ToyLoopDec.relay`); the one-content-unit rule stands as the house convention.

**Harnesses.** `WiredDemo`/`BenchDemo` lose `pacing0`/`HS`/`hbDec`/`sdec`/`ssched`/`sched`/
`schedOf`/`decOf`/`maxTicks` and the 200 000-element lists; `Timing` is `{nowUs, pulse}` read
this activation; `readings m tm : TickReadings m` (proposer 0 times out and pulses at every
tick, proposer 1 never; sample every tick) → `Dag.Timing.ofReadings`.

**Measurements (same box, `bench 500 10 …`).**

| | RSS 6 s | RSS 20 s | req/s steady | activations/6 s | step first→last quarter |
|---|---|---|---|---|---|
| D83 | 30.5 GB | OOM-killed (55 GB at 12 s) | collapsing (13k, 3k at the tail of 12 s) | 34.8k | all processes ≈ 2.7× (acc 0.75→2.0 ms, agg 0.59→1.6 ms, client 1.26→1.89 ms) |
| memo fix only | 325 MB | 361 MB | 20–23k | 56.1k | clients/reps flat; agg 0.52→0.65 ms; acc 0.70→1.14 ms; prop1 0.66→1.05 ms |
| D84 | 310 MB | 321 MB | 22–23k (431k requests / 20 s) | 63.5k | prop1 0.60→0.61 ms, agg 0.495→0.498 ms, clients 0.98→0.94 ms, reps flat; **acc 0.67→1.14 ms remains** (the queued `kfold` refold cell — `acceptor_checkpoint_from_replicas` re-folds every arrival each step) |

`wired` OK (41 activations, 9 ms).

**Gate.** `lake build` + the six exes green; zero sorries; `AxCheck` 92 reports, all
standard subsets (`[propext, Classical.choice, Quot.sound]` ×70, `[propext, Quot.sound]`
×21, `[Quot.sound]` ×1; `sorryAx`/`ofReduceBool` 0); no unverified seams (15 `@[csimp]`
equations, the two memo tables' re-proven); `maxHeartbeats` sites unchanged (LE 600k,
PaxosCore 250k); all 33 censuses identical to D83; `falsify` PASSED, `explore` true/true,
`paxos` OK, `twopc` OK, `wired` OK, `bench 500 10 6000` OK (97k requests, 23k req/s at the
end). Per-file (standalone `lake env lean`, vs the D83 gate): `Project/Process` 175 s (158;
the new 39-arm `timingAt_enqueueInbox`), `PaxosBench` 103 s (103), `Project/WiredMain` 76 s
(82), `LeaderElection` 67 s (67), `Project/StepOps` 59 s (62), `SequencePayload` 45 s (45),
`PaxosCore` 44 s (44), `HydroTickCheck` 27 s (27), `Project/Wired` 16 s (15),
`PaxosWithClient` 16 s (16).

**Rulings for the record.** Sched representation change ratified ("ok iii"); `sample_every`
included ("you should also fix the sample every memory blowup"); the `Nat → Bool` sample oracle
over `Bool` by the long-way rule. Statement shapes: `…Dec (SchedSem …)` records and the
headlines' `sdec` binders change type (scripts); meaning identical on realizable runs.

**Ruling at close (user): "in general unbounded singletons in prod should be single slot."**
The `sample_every` oracle as landed here (`Nat → Bool`, the flags of the ticks appended this
activation) is per-tick but still list-shaped at the production boundary; it accommodates
the model's `defer_tick`, whose Sched trace `init :: t` exposes next tick's entry at this
tick (its Dag node emits two entries at its first tick) — not what DFIR does (the deferred
value stays in the node's own state until the next tick). The clean end state, next item:
**tick-located wires are single slots in production** — `Kind.delta` for `.atomic`/
`.atomicStream`/`.atomicKeyed`/`.atomicKSing` becomes `Option` (this tick's value), Sched's
`defer_tick init t` at step `k` is the prefix `(init :: t).dropLast` (the Values trace is the
limit) with the Dag node holding the pending value and releasing one entry per tick, a
per-op `TPaced` fact (an Atomic trace has ≤ one entry per tick of its member — the `TMono`
sibling), and `sample_every`'s cell a register `Option α` with a `Bool` oracle. The
unbounded singletons proper (`.sing`, `.ksing`) are already single-slot; `kfold` is the other
exception.

**Queue after D84.** The single-slot redesign above (own checkpoint: it changes
`defer_tick`'s machine semantics); the acceptors' `kfold` cell (keep the map, not the
arrivals — DFIR's `fold_keyed` keeps the map) — the last growing step time; a clock-monotonicity `requires`
when a latency `ensures` lands (trust-base item: the shell's clock); the bench harness's
back-to-back runs collide on ports (`bind` without reuse; a 5 s gap between runs avoids it);
then the D83 queue (higher-order `hydro def`, `paxos_with_client_safe_wired`, skipping
non-hosted nodes in `locStep`, …). Removed from the queue: "a `ClockReadings` carrier a process
can extend in O(1)" — a process no longer holds one.

### D85 — Tick-located wires are single slots in production: `Kind.delta` of the atomic kinds is `Option`, `TPaced` is a proved fact of every Atomic-producing operator, and `defer_tick` is DFIR's FIFO at the machine too

**Scope (user, the D84 close ruling).** "In general unbounded singletons in prod should be
single slot." The D84 production interpreter still carried an Atomic increment as a list
(`Fin (mem ℓ) → List X`), the `sample_every` oracle was `Nat → Bool` (per-tick flags of the
entries appended this activation) and the `defer_tick` node held a list — all to accommodate
the machine's `defer_tick init t = init :: t`, which released the current tick's write in the
same step (two entries at its first tick). Not what DFIR does: the deferred value stays in the
node's own state until the next tick.

**Checkpoint #1 (approved).** (a) The machine's `defer_tick` is release-at-next-tick, not the
`dropLast` sketched at D84's close: the knot wire runs one step behind and `take ticks` works
inside knots. (b) The per-op fact is per STEP with an inactive clause — `TPaced act x := ∀ i k,
(incTrace (x i) k).length ≤ 1 ∧ (act i k = false → incTrace (x i) k = [])` — because "one entry
per tick" is false of the knot wire (`fix_tick`'s `gateView` releases at active steps, tick or
stutter). (c) An `Option` cell is fine where a cell is a slot. (d) `sample_every`'s machine
semantics unchanged; its production oracle is a `Bool` per tick, its cell a tick counter — no
value register (the sampled trace's latest is this step's slot). (e) "forward_ref fidelity"
is queued: DFIR resolves an acyclic forward reference inside the same tick, the model's knot
wire is a tick late; the fix is a `let` for acyclic tick knots, not a change to `fix`.

**Ruling by proof: the machine's `defer_tick` is the FIFO's own step.** Writing
`spec_defer_tick` against the first candidate, `defer_tick init t = (init :: t).take ticks`,
showed the two sides disagree on inputs more than one tick behind the tick count: a
Sched-admissible Atomic (a per-step view `Fin n → Nat → Trace σ`) may gain nothing for several
ticks and then catch up, and a one-slot FIFO releases what it has at each tick while
`take ticks` jumps. Such inputs are not Rust-expressible (a tick singleton has a value at
every tick of its member), but the rule is that every production execution is a Sched run
with no side condition (R13, "the model follows production, never the proof"), so the
machine took the FIFO's semantics: `deferReleased p len : Nat → Nat` (`Sched.lean`) — `0 ↦ if
p 0 then 1 else 0`; `k+1 ↦ if p (k+1) ∧ released k ≤ len (k+1) then released k + 1 else
released k` (a tick releases the buffered entry if there is one; the buffer is `init :: t`
beyond the released count) — and `defer_tick init t i step := (init :: t i step).take
(deferReleased (pacing.ticks ℓ i) (fun k => (t i k).length) step)`. On inputs at most one
tick behind the tick count — every Rust-expressible wire — the released count IS the tick
count (`deferReleased_eq_ticks`, `SchedPaced.lean`), i.e. `(init :: t).take ticks`: tick `j`
reads entry `j` of `init :: t`, the previous tick's write. `Values` is untouched
(`defer_tick init t = init :: t`, the limit). DFIR reference recorded at the definition:
`dfir_lang/src/graph/ops/defer_tick.rs:6` ("buffers all input items and releases them at the
next time boundary"), `hydro_lang/src/compile/ir/mod.rs:4807–4828`. `causal_defer_tick`,
`tmono_defer_tick` (`take_prefix_take_of_prefix`) and the Couple corner (`cpl` by `take`
prefix) follow the new definition; `TransferChecks` unchanged.

**The production side.** `Kind.delta` of `.atomic`/`.atomicStream`/`.atomicKeyed`/
`.atomicKSing` is `Fin (mem ℓ) → Option X`; the lock-step increment `Kind.incAt` is
`(incTrace …).head?`; `PendingOf.push` appends `.toList`; the knot cells (`KnotKind.deltaInit`
`none`, `feed` `if act then d i else none`, `keep` `if act then inc i else (d i).or (inc i)`).
Per node: `defer_tick` cell `List σ` (init `[init]`; `buf := s ++ Δ.toList`; a tick emits
`buf.head?` and keeps `buf.tail`) — the single slot is the DELTA, the cell holds ≤ 1 entry
between steps on Rust-expressible inputs and whatever a Sched-admissible input makes it hold
otherwise; `sample_every` cell a tick counter (`if Δ.isSome then s+1 else s`), emission `match
Δ with | some o => if d i (s i) then o.toList else [] | none => []`; `zipTick`/
`reduce_watermark` pendings `++ Δ.toList`, outputs `.head?`; `allTicks`/`kallTicks` `(Δ).getD
[]`; `tfold` `scanRun … Δ.toList`, `.2.head?`; `latest_monotone` `match Δ with | some o => o.or
(s i) | none => s i`; `tick_scan` `(run i).2.head?`; every timer/snapshot node emits `some v`/
`none`. `Wired.lean` mirrors it (`locDelta` atomic `Option`, `idleDelta`/`allEmpty` `= none`,
`Node.Oracle` for `sample_every` `Hosted m ℓ → Bool`, `TickReadings.sample : Bool`).

**`TPaced` — the fact that makes `head?` lossless** (`SchedPaced.lean`, 484 lines; the `TMono`
sibling). `incTrace_eq_toList_head?`: under `TPaced`, `incTrace x k = (incTrace x k).head?.toList`.
Per operator, on paced inputs: `tpaced_snapshot`, `_batch`, `_batch_ordered`,
`_timeout_snapshot`, `_source_interval_batch`, `_time`, `_snapshot_bounded`, `_ksnapshot`,
`_mapTick`, `_zipTick`, `_defer_tick`, `_flattenOrdered`, `_flattenUnordered`, `_tinto_keyed`,
`_tassume_ordering`, `_reduce_watermark`, `_tfold`, `_tick_scan` (via `SchedTick.slice_lenPaced`/
`unslice_paced`), and the knot: `gateView_paced` (+ `gateView_len_behind`), `TPaced.tickshift`,
`tpaced_tick_diag` (by `stab_iterate_tick`), `tpaced_fix_tick`. Carried through the graph as
`Kind.tpaced pacing` / `HTPaced pacing env` (`Project/Causal.lean`: `readShape_tpaced`,
`shapeToList_tpaced`, `knotFix_tpaced`, `bekic_tpaced`, `Node.denote_tpaced`/`Dag.denote_tpaced`
— explicit `pacing` argument). Consumers: `spec_defer_tick`, `spec_allTicks`, `spec_kallTicks`,
`spec_zipTick`, `spec_reduce_watermark`, `spec_sample_every`, `spec_tick_scan`, `spec_tfold`,
`spec_latest_monotone` take a `TPaced` hypothesis; `Node.step_spec`/`Dag.step_spec`/
`Dag.run_spec` take `HTPaced pacing env`. Discharged once, inside `WiredRun`: an
interleaving's inputs as wires, `Interleaving.wires := envOfInc I.env`, are monotone
(`wires_mono`) and single-slot (`wires_paced`, from `env_idleSlots` — `HIdleSlots I.pacing (fun
k => envMask (I.who k) (inp k))` — and `HTPaced_envOfInc`; `Project/EnvOfInc.lean`: `Kind.ofInc`
atomic is `accum (fun t => (inc t i).toList)`, `tpaced_accum_slots`), so **`Interleaving.outputs`
takes no hypotheses** (`outputs_env` is the general form). The user caught the first draft
threading `hpenv` — and the pre-existing `env := envOfInc I.env`/`hinp`/`henv` — through
`paxos_safe_wired`: generic facts about every interleaving do not belong in a program's proof,
and the Paxos proof now only names `I.wires`. None of the program proofs change (all at
`Values`).

**Looseness audit (Project ⊆ Sched).** Every production step is a Sched step with no side
condition: `Dag.run_spec` is an equality on prescribed cells for *every* `TMono`+`TPaced`
environment, and every interleaving's inputs are paced by construction. What `TPaced` adds
over D84 is a proved bound, not an assumption about the shell. Remaining gaps are the
recorded ones: DFIR fidelity of the node steps is R13 by inspection (not a Lean theorem);
the knot wire's release is one step late relative to DFIR's same-tick forward_ref (queued,
item below).

**The toy** (`toy_defer_next_tick`, `HydroTickCheck.lean`; census 0/0/0 — 34 censuses now).
Input `7` written at every tick (visible in its step). Machine: `[[0], [0, 7], [0, 7, 7]]`
at steps 0–2; production node (`Dag.run` on the input's increments): `[some 0, some 7, some
7]`. Pre-D85 the same input gave `[[0, 7], [0, 7, 7], [0, 7, 7, 7]]` at the machine and the
node's list cell `[0, 7]`, `[7]`, `[7]`.

**A harness hang, not a model one.** `lake exe wired` intermittently never exited after
printing the commit (roughly one run in four). Traced with a per-activation log: every activation
completed, every machine thread returned, the main thread printed `wired: OK` to a buffered
stdout — and the process did not exit: the Lean runtime waits for dedicated tasks at exit,
and the reader threads block in `recv?` on sockets nobody closes (a `shutdown` of the dialed
sockets made it hang in every run tried). Both harnesses now flush and `IO.Process.exit` with the
computed code; 10/10 runs since. Pre-existing and timing-dependent; not a D85 regression in
the model.

**Measurements.** `bench 500 10 6000`: 62.6k activations, 94.2k requests, 314 MB max RSS at
20 s with 427.9k requests (D84: 310/321 MB, 431k) — unchanged within noise; the acceptors'
0.9→1.5 ms growth is the queued `kfold` cell. `wired`: 41 activations, 9 ms. Full rebuild
7 m 30 s wall; per-file: `Project/Process` 186 s (D84 175), `PaxosBench` 104 (103),
`Project/WiredMain` 85 (76), `LeaderElection` 69 (67), `Project/StepOps` 61 (59),
`SequencePayload` 45 (45), `PaxosCore` 45 (44), `HydroTickCheck` 27 (27), `PaxosWithClient` 16
(16), `Project/Wired` 15 (16); `SchedPaced` 2.3 s, `Project/Causal` 7.8 s.

**Gate.** `lake build Hydro falsify explore paxos twopc wired bench` green; zero sorries;
`AxCheck` 92 reports, all standard subsets (`[propext, Classical.choice, Quot.sound]` ×70,
`[propext, Quot.sound]` ×21, `[Quot.sound]` ×1; `sorryAx`/`ofReduceBool` 0); no unverified
seams (15 `@[csimp]` equations, unchanged); `maxHeartbeats` sites unchanged (LE 600k,
PaxosCore 250k); 33 censuses identical to D84 plus `toy_defer_next_tick` 0/0/0; `falsify`
PASSED, `explore` true/true, `paxos` OK, `twopc` OK, `wired` OK, `bench 500 10 6000` OK.

**Docs.** `ARCHITECTURE` (single slots, `TPaced`, `defer_tick`), `CORRESPONDENCE` (two rows:
single slots, `defer_tick`; `IdleEnv` now constrains the atomic slots), `SCHED_AUDIT` S4 item
18, `ENGINE_NOTES` (D84 note corrected; single-slot gotchas: `incTrace_eq_toList_head?` under
`simp only`, `Option.toList_none` at two types, `show` on `Option` shapes, beta-redexes before
`omega`, `deferReleased` facts), `GATE` (census row), `docs/06`, `docs/09`.

**Queue after D85.** forward_ref fidelity — acyclic tick knots as same-tick `let`s (DFIR
resolves an acyclic forward reference inside one tick; the model's knot wire is a tick late;
e5 above); the acceptors' `kfold` cell (the last growing step time); per-activation base cost
(skip non-hosted nodes in `Dag.locStep`); higher-order `hydro def`;
`paxos_with_client_safe_wired`; a clock-monotonicity `requires` when a latency `ensures`
lands; bench port reuse; liveness rung 2; the illegal-decision-record diagnostic;
`.boundedValue` keyed-singleton batch op; `SchedSem` against production DFIR (the cross
check); the two engine limitations; `Exe/` roots; lint debt; upstream reports.

### D86 — The production interpreter's throughput, round by round: 22.5k → 52k requests/s (goal 400k; the campaign continues in D87)

**Scope (user).** An autonomous performance campaign on the production path — the
`Project` interpreter, the wire, the harness — toward 400k requests/s on `bench 500 10
20000`, starting with the acceptors' `kfold` cell; the program text, the Values
denotation, the Sched semantics and every theorem statement untouched; the only compiled
seam `@[csimp]` with a kernel-checked equation. Each round: measure, change, gate, record.
The baseline reproduced: **22.5k requests/s**, p50 43 ms, 431k requests in 20 s, max RSS
318 MB; leader step 2.0 ms at ~105 requests per activation, acceptors growing 0.9 → 1.5 ms
over the run, idle processes spinning at ~0.5 ms per empty activation.

**Instrumentation first.** `HYDRO_BENCH_PROFILE=1` splits an activation into decode+enqueue
/ step / encode with message counts; `HYDRO_BENCH_NODES=1` attributes the step to the
operators — `Hydro/Project/Profile.lean`: `Dag.locStepProfiled` is `Dag.locStep`'s walk in
`IO` with the clock read around each node, nodes tagged by constructor and location
(`Node.tag`), callees and knots walked into; `NodeProfile.summarize`. `bench idle P N`
times an empty activation of process `P` stage by stage without sockets.
`HYDRO_BENCH_TRACE=1` prints every activation with traffic (time, process, messages in and
out, wire nodes sent on) for the first 1.5 s. `perf` on the Lean binary attributes 60 % of
cycles to the runtime (`lean_dec_ref_cold`, `mi_malloc`/`mi_free`, `lean_apply`): the cost
is allocation and reference counting spread over everything, not one hot function.

**Round 1 — the acceptors' `kfold` cell.** `Node.step`/`Node.locStep` kept the arrivals
so far and recomputed `keyedFoldAssocH` over them every step (the growth: 80k checkpoint
entries after 20 s). The cell is now the fold's map, this step's arrivals folded in
(`kfoldStepH g init m Δ := Δ.foldl (SchedKMap.insertFold g init) m`; `insertFold` moves the
key's entry to the end with its accumulator advanced — the machine's maps are keyed by
last arrival, `List.dedup`). `keyedFoldAssoc_append_foldl : keyedFoldAssoc g init (l ++ Δ)
= Δ.foldl (insertFold g init) (keyedFoldAssoc g init l)` (via `dedup_append_singleton` and
`keyedFoldAssoc_append_singleton`, `Hydro/KeyedHash.lean`); `Node.prevState` of `kfold` is
`keyedFoldAssocH g init (view t)`; `spec_kfold` and `wired_kfold` re-proved. Acceptor step
flat at 0.97 ms (was 0.9 → 1.5).

**Round 2 — one `Option` per node, not a table per member.** `memoBoxImpl`/`memoBox2Impl`
tabulated every node's cells and increments with `Array.ofFn` over `mem ℓ` and a `Sigma`
equality test per entry, on every activation. A process is one member: `hostedSelf m ℓ :
Option (Hosted m ℓ)` (one `DecidableEq L` test) and the materialized value is `(hostedSelf
m ℓ).map f`; `lookupHosted`/`lookupHosted2` read it (`@[noinline]`, capturing the value and
nothing else — D84). `hostedList` likewise (`hostedList_eq` against the old `finRange`
scan). The `@[csimp]` equations re-proved. Idle step 424 → 272 µs.

**Round 3 — the graph built once, the cells never shared across threads.** `D cfg` was a
function of the configuration: every call (`activate` made four, `wireDecode` one per inbox
batch) rebuilt the whole 187-node graph. And the cells lived in an `IO.Ref` created on the
main thread and captured by the task closure: an object stored into a shared reference is
marked multi-threaded (`lean_mark_mt`, 2.8 % of cycles walking the new state every
activation) and every reference-count operation on it is atomic thereafter
(`lean_dec_ref_cold`). `BenchDemo.Graph` holds `dag` and `cod`, built inside each process's
task; the cells are a loop argument of `machine`. Idle activation 485 → 297 µs.

**Round 4 — every tick body ran `1 + outputs` times.** `Node.locStep`'s arms shared the
per-member computation between the cells and the increments as a *closure*, re-evaluated at
each use: `tick_scan` ran its `sliced!` body once for the cells and once per output leaf
(`unzipShapeLoc` reads the run per leaf); `reduce_watermark`, `zipTick`, `defer_tick`,
`batch`, every `fold` and `kfold` computed twice; a network node stepped each channel cell
twice. The arms now bind the shared value under `memo`/`memo2` — the identity in the logic
(`memo_apply`/`memo2_apply` are `rfl`, added to the `simp only` sets that unfold
`Node.locStep`), one evaluation when compiled. Leader `tick_scan@prop` 999 → 599 µs per
activation, acceptor `reduce_watermark` 258 → 139, client `tick_scan@client` 381 → 194.

**Round 5 — skipping the nodes of other locations.** `Node.touches m n` (the process hosts
one of the node's output locations or its sender cluster; callees and knots always walked);
`Node.locStep_idle`: an untouched node's step is `(s, HList.idle …, Node.idleSends …)` —
every component a function out of an empty hosted domain (`Hosted.elim`,
`Kind.locDelta_eq_idle`, `HList.eq_idle`, `shapeKinds_not_hosted`). `Dag.locStepImpl` skips
such nodes (cells untouched, empty increments to the rest); `Node.locStepImpl` routes
callees and knots to it; `Dag.locStep_eq_impl : @Dag.locStep = @Dag.locStepImpl` is the
`@[csimp]` equation (mutual induction, `Node.locStepImpl_eq`/`Dag.locStepImpl_eq`). Idle
`locStep` 176 → 160 µs only — the floor was elsewhere: `Node.locStep` materialized a
callee's or a knot body's cells with `Node.memoState` *after* stepping it, and the callee had
already materialized its own nodes' cells as it stepped them, so a cell at nesting depth
`d` was rebuilt `d` times (`paxos_bench` calls `paxos_with_client` calls `paxos_protocol`
calls `paxos_core` calls `leader_election`/`sequence_payload` — and the knots inside). `Node.memoStateImpl` leaves
`call`/`fixN` cells as they are (`Node.memoStateImpl_eq : memoStateImpl m n s = s` — the
materialization is the identity in the logic anyway, `Node.memoState_eq`); `Dag.locStepImpl`
uses it. Idle `locStep` 160 → 60 µs on the leader (41 on an acceptor; what is left is the
hosted `sliced!` blocks stepping on their empty batches); the remaining per-activation floor is the eight walks
of 187 nodes (`inboxOf`, `enqueueInbox`, `memoState`, `Timing.ofReadings`, `oracleOf`,
`locStep`, `wireEncode`, one `wireDecode` per inbox batch at 8.4 µs each — 50 batches on
the leader), each 9–17 µs of tuple allocation.

**Round 6 — the wire, where the time actually was.** `HYDRO_BENCH_TRACE` at two requests in
flight: round trip 15 ms for ~6 hops of 0.25 ms activations. `(c.send bytes).block` through
the libuv event loop cost **~1 ms per send** (cross-thread hand-off), 2–7 ms per leader
activation (`sends took` in the trace); and the ten machines were threads of one OS process
sharing one event loop, one allocator and one set of reference counts — the receive side
paid the same hand-off. (`noDelay` on every socket changed nothing: Nagle was not it.)
Two changes: an `Outbox` per connection — the machine queues frames (3 µs) and a writer
thread per connection drains them into one vectored `sendAll`; and **`bench` runs each
process as its own OS process** (`bench --proc P …` spawned by the parent, which relays
their output and sums their `stats` lines) — as deployed Hydro processes are. Idle round
trip 15 → 4.5 ms; **`bench 500 10 10000`: 52k requests/s, p50 18 ms** (from 22.5k / 43 ms).
The inbox is a mutex + condition variable the readers and a per-process pulse timer
signal; a process with no input, no due pulse and no pending work (`Dag.hasPending`,
`Profile.lean`: `defer_tick` buffers, knot feedback, undelivered channel items, and every
hosted `sliced!` block — Rust `use::state` is `cycle_with_initial` + `complete_next_tick`, a
`defer_tick` DFIR keeps ticking on) waits instead of spinning; with ticks everywhere this
waits only the processes without a tick, which is none of the benchmark's, but it is the
honest rule and the mechanism is in place.

**What did not move.** Idle activation ~130 µs (the eight walks); the leader's per-request
work ~5 µs in the tick bodies plus JSON encode/decode per message (~1 µs a message, 192 µs
of a 0.85 ms leader activation); the acceptors' `reduce_watermark` at 139 µs per activation
(`payloadsImpl` rewrites the whole map every tick). D87 continues.

### D87 — The wire in one walk and in binary: 52k → 82k requests/s at 1000 clients, 100k at saturation; the cost model that says what 400k would take

**Round 7 — the inbox in one walk.** `Dag.inboxOf` of the decoded batches walked the graph
three times per inbox batch (`wireDecode` — also the elaborator's numbering of the network
nodes —, `inboxOfSends`, `Inbox.append`), ~25 µs a batch, six batches a leader activation:
the leader's whole decode budget. `Dag.inboxOfWire m d cod 0 batches` (`Process.lean`) walks
once: at each network node, in wire order, the inbox is the batches' messages for that node
and sender, decoded, in arrival order. **`Dag.inboxOfWire_eq`**: it is `Dag.inboxOf` of the
batches each decoded by `Dag.wireDecode` — through `inboxOfSends_wireDecode` (one batch),
`inboxOfWire_append` (two batch lists), `inboxOfWire_nil`, and `inboxOfWire_snd` (the
numbering does not depend on the batches), each a mutual structural induction over the
graph; `Dag.inboxOf` stays the proof layer's statement. The harnesses call `inboxOfWire`.

**Round 8 — the wire in binary.** JSON was the first wire (D74): `Json` values parsed and
printed per message, lines split on newlines. The leader spent 0.9 µs a message decoding
and 0.65 encoding — at 170 messages an activation, a third of the activation. `Hydro/
Wire.lean` is now `class Wire (α) : put : α → ByteArray → ByteArray, get : ByteArray → Nat →
Except String (α × Nat)` — the mirror of the Rust structs' `#[derive(Serialize,
Deserialize)]` under `bincode`: fields in order, naturals LEB128 varints, tags one byte,
lists length-prefixed; `Nat`/`Int` (zigzag)/`Bool`/`Unit`/`Fin`/`Option`/`Except`/`Prod`/
`List`/`ByteArray`/`String` here, `Lex`/`HMap` (its canonical listing)/`Histogram`
(its counts) and the message structs `Ballot`/`LogValue`/`P1b`/`P2a`/`P2b`/`KvPayload`/
`SequencedKv` written by hand at their definitions (the `deriving Lean.ToJson, FromJson`
clauses are gone — nothing else read them). `Codec α := {enc : α → ByteArray, dec :
ByteArray → Except String α}`, `Codec.ofWire`; the elaborator's `M.wireCodecs` takes `[Wire
X]` per payload type parameter. `WireMsg := Nat × Nat × ByteArray`; a `Line` on the socket is
a 4-byte length then its bytes (`Wire.frame`/`Wire.frames`, the reader's buffer cut into
complete frames). No theorem mentions the representation (`decodeAt`/`wireEncode` use
`cod.dec`/`cod.enc` only; the wire's round trip was never a proof obligation — the headlines
take a process's inbox as given, D80). Leader decode 0.9 → 0.53 µs a message, encode 0.65 →
0.28; **`bench 500 10 10000`: 82k requests/s, p50 11.5 ms** (`wired` and the four Values
witnesses unchanged). 20 s and 40 s runs: throughput steady at 77–84k, max RSS 222 MB (one
process; the parent's `time -v` reports the largest descendant), the leader's step drifts
+5 % per 20 s (470 → 538 µs over 40 s) and the replicas' +20 % over the first 20 s then
flat — queued below.

**The cost model, measured.** At 500 virtual clients per client process (1000 in flight),
the leader runs 95 % busy: 16k activations in 8 s, 464 µs each, 40 requests and 170
messages an activation. The activation is **fixed** work (the eight walks, 130 µs idle —
`bench idle 0 3000`: `inboxOfWire` 9, `enqueueInbox` 10, `memoState` 16,
`Timing.ofReadings` 8, `oracleOf` 9, `locStep` 60 — the hosted `sliced!` blocks on their
empty batches —, `wireEncode` 10), **per-in-flight** work (the two `sliced!`
blocks that re-read their registers every tick as the Rust does — `collect_quorum`'s
`not_all`/`min_but_not_max` and `join_responses`'s `remaining_to_join`, `#103` 120 µs and
`#108` 73 µs at this load) and **per-request** work (decode 4.3 messages, the hash-map
operations of the tick bodies — `collect_quorum` ~10 per p2b through `dedupImpl`,
`foldAcc`, the lookups, `listAntiJoinImpl`, two `listFilterNotInImpl`; `join_responses`
~3 per entry —, encode 2 messages; ~4 µs). Three consequences, each checked:

- *Closed-loop clients make throughput `in-flight / latency`* (Little's law): 1000 / 11.5
  ms = 87k ✓. Spacing activations to coalesce arrivals (a harness experiment, 2 ms minimum
  between activations, since removed) *lowered* throughput to 62k: fewer, larger ticks pay
  the fixed cost less often but stretch the pipeline, and latency is the denominator.
- *More clients saturate at the per-request cost*: 1000/2000/4000 virtual clients per
  client process give 96k / 100k / 104k requests/s at p50 20 / 37 / 72 ms — the leader at
  100 % with 130 requests an activation (`#103` 440 µs, `#108` 277 µs, decode 275, encode
  72, step 1.18 ms): ~10 µs of leader time per request, of which 3.4 in `collect_quorum`,
  2.1 in `join_responses`, 2.3 decoding, 0.6 encoding.
- *The remaining factor is the runtime's constant*: a `Std.HashMap` operation on a boxed
  `(Nat × Ballot)` key through the polymorphic `Hashable`/`BEq` instances costs ~100 ns in
  Lean against ~10 in Rust; the tick bodies are already hash-backed and linear (D75) and
  do what the Rust does, operation for operation. 400k requests/s is 2.5 µs of leader time
  a request — below the ~50 hash-map operations the three `sliced!` blocks make per request
  at Lean's constant. What remains cheap: fusing the eight walks (~80 µs fixed → latency);
  a single-pass `keyedFoldAssocImpl` (4 hash operations an element → 3); the decoder's
  `Except` allocations (0.53 µs a message → ~0.2). What is not: the per-element constant,
  short of specializing the tick bodies' instances (a compiler, not an interpreter).

**Also in this round.** `bench`'s 0-request run prints `bench: FAILED (no request completed)`
and exits 1 (a run started while a previous one's ports were still bound did that once —
the port-reuse item in the queue). `WiredDemo` uses the same frames.

**Queue after D87.** The leader's slow step drift (+5 % per 20 s at flat RSS — which
register grows, and whether the Rust's grows the same); the eight walks
fused into `Dag.iterate`'s one pass; `keyedFoldAssocImpl` single-pass; the decoder without
`Except` per field; `reduce_watermark`'s `payloadsImpl` is O(log) per tick (147 µs at the
acceptors, the largest single node in the system — the Rust's `ReduceKeyedWatermark` is
O(batch), its map persistent: the Sched carrier of an unbounded keyed singleton as a hash
map with the per-tick output as the map, not a rebuilt list); then the D85 queue.

### D88 — Wire laws, the loop body at the wire, the model split, and the headline over traces

**What landed.** `Hydro/Wire.lean`: a `Wire α` class (`encode`, `get`, and the law
`get_encode : get (pre ++ encode x ++ suf) pre.size = .ok (x, …)`) with every instance's law
proved (LEB128 naturals, zigzag integers, products, options, exceptions, lists, strings,
byte arrays; message structs through `Wire.ofEquiv` to their field tuples), and `Codec α`
(`dec_enc`) as what `HydroGenProject.wireCodecs` builds from them. Serde's `deriving
ToJson/FromJson` is gone from every message type. `Dag.iterateWire` (`Project/Process.lean`)
is the production body the shells call verbatim — decode the frames handed in, iterate,
encode the sends — with `inboxOfWire_wireEncode_self`/`inboxOfWire_wireFor` closing the
round trip through the receiver's routing table.

**The folder split.** `Hydro/Project/` is production (`IR`, `Step`, `Wired`, `Wire`,
`Process`, `Profile`); `Hydro/Project/Model/` is the lock-step model and its
correspondence (`Causal`, `Knot`, `EnvOfInc`, `StepSpec`, `StepOps*`, `StepMain`,
`WiredSpec`, `WiredMain`, `WiredRun`, `DistributedModel` — the former `Loop.lean` —, and
`WireTrace`). Model imports production, never the reverse; a shell links no model.

**The headline over traces.** `paxos_safe_wired` quantifies over a `WireTrace`: who ran at
each step, its inputs, its clock/timer readings, and which earlier activations' frames it
consumed, with the TCP promise as the one field `fifo` (per connection, consumed frames
are a prefix of the sender's, in order). No interleaving appears in the statement;
`WireTrace.interleaving`/`batched`/`act_eq` (D89) produce one and show the trace's run is
its run at the wire. `Interleaving.Batched` (`DistributedModel.lean`) is the intermediate
notion: cursors that deliver whole activations, `loop_active_wire` its loop-body theorem.

### D89 — Timers as programs, the real at-least-once coupling, `latest`, and the trace proofs

**Timers are programs.** Rust `Stream::timeout` (`stream/mod.rs`) read the wall clock inside
a `fold` closure — unmirrorable; it now reads it once per tick (`use::time`) in a
`sliced!` block, with no `assume_retries`. Its Lean mirror `Std/Timers.lean` `timeout` is
that body line by line, returns Rust's `Optional<(), L, Unbounded>` (`H.latest expired`),
and `ensures fresh_not_expired`; `sample_every` (`optional.rs`) likewise, `ensures
sampled_src`. The two primitives `timeout_snapshot`/`sample_every` and their decision
families (`TimerDec`/`SampleDec`, `sampleAtOpt`/`flaggedBelow`/`sampleDerive`) are
deleted; `time` and `source_interval_batch` are the only timing primitives, the latter now
`source_interval_batch (delay : Fin (mem ℓ) → Nat) (period : Nat)` — Rust's
`source_interval_delayed(q!(CLUSTER_SELF_ID * mult), q!(period))`. Durations are
microseconds (`Hydro/Duration.lean`: `from_secs`/`from_millis` where the Rust calls them).
`PaxosConfig` is a structure threaded where Rust passes `paxos_config`; `paxos_bench` takes
the two interval parameters.

**What that exposed (the D83 harness bug).** The shells had been scripting the timers —
`timeout := i = 0`, `sample := true` — i.e. proposer 0 always expired and gossiped every
tick (~75 heartbeats per activation into proposer 1). Legal in the model (any timing
verdict is), but not Rust's timing. Now `TickReadings = {clock, pulse ℓ i delay period}`,
`TickReadings.ofClock now last` computes every interval pulse from the clock
(`intervalFired`: a fire instant in `(last, now]`), `Dag.timers` lists a process's timers,
and `bench`/`wired` wake on `nextDue`. Heartbeats are 5 s apart, as in `paxos_bench.rs`.

**Vocabulary.** `batch` at any `Retries` (`retryBatchCuts`: a batch is legal iff its
elements are entitled — re-delivery is the at-least-once freedom), `boOr`, `bisEmpty`,
`boFilterIf`, and **the real per-tick at-least-once coupling**: `BatchTraceLe` at
`.atLeastOnce` is `s.map StutterSeq.mk <+: v` / `s.map (RetryPool.mk ∘ ofList) <+: v`
instead of `True`, `allEO` only fences `.kstream` leaves. A `True` coupling was never a
soundness hole (nothing machine-side follows from `True`), but it was a missing guardrail
— an op with a vacuous coupling can be wrong silently.

**`latest`.** `Optional::latest()` as a primitive: `Atomic ℓ (Option σ) → Singleton ℓ (Option
σ) (Option σ) .totalOrder .exactlyOnce .unbounded` (the singleton-of-`Option` spelling of
`Optional<σ, L, Unbounded>`, for now). Semantics are the runtime's: the unbounded optional
streams the latest `Some`/`None` **update**, and a `snapshot` reads the last update at its
cut — absent before any, absent when the latest tick's optional was `None` (`lastUpdate`,
not the old sticky `latestOf`). Values' cut is asynchronous (a `SnapDec`, the Rust
`nondet!(/** absorbed into timeout */)` site, now in the census), Sched's is the latest
value. `latest_monotone` lost its hidden `runMax`: the Values cell is the last updates while
the realized prefix ascends in `none ⊑ some x ⊑ some y`, blocked after — a branch that is
dead on every coupled run (`wf` carries the ascent, `latestMonoCell_val_of_ascending`), as
D71's refused-content branches are. Queued: ghost parameters (interpretation-indexed
`Prop` fields + naturality through the generated `_co_rr`/`_param` lemmas) so the proof is
an argument and the branch disappears; a proper `Optional` carrier (today's
`Singleton`-of-`Option` conflates `Singleton<Option<T>>` with `Optional<T>` and drags a
source grade along).

**Generator.** A callee may now return a singleton: `hydro_couple` emits `M_co_srcᵢ` (the
corner's remembered machine source in simp-normal form), `rr_ex` scripts `simp only` the
pin hypotheses under binders, `coTickScanProj` rewrites over-applied leg projections.
Toys `toy_lat`/`toy_snap` (1/0/0) are the build-enforced witness, with two `#guard`s on a
synchronous and an asynchronous cut.

**The trace proofs.** `WireTrace.lean` is sorry-free. The induced schedule (`traceSched`):
per connection the cursor at `t` is the message count of the frames the receiver consumed
through `t`; per timer the readings at the member's `n`-th activation (`nthActivation`).
One invariant (`Dag.TraceQueues`, mutual over the graph): each receiving end holds the
sender's unconsumed messages in activation order with the consumed count, each timer cell
the activation count. `fifo` makes the consumed activations a prefix of the sender's
(`consumed_succ_prefix`), so the taken frames are the head of the queue and the induced
cursor delivers exactly them (`traceQueues_arrivals`); the init/idle/active steps and the
timing lemma are the other four mutual theorems; `Inv` carries queues, cells and typed
sends through `view_active`/`view_idle`/`loop_active`, and `act_eq` follows.

**Censuses.** `timeout` 2/0/0, `sample_every` 1/0/0, `p_leader_heartbeat` 5/1/0,
`leader_election` 10/3/3, `paxos_core` 16/5/5, `paxos_with_client` 19/7/6,
`paxos_protocol` 23/10/9, `paxos_bench` 34/12/11. Bench at the Rust intervals (100 ms
client / 1000 ms aggregate): 66–70k requests/s at `500 10`, RSS 248 MB; the D87 figure
of 82k was at a 200 ms client window with the scripted timers.

**Also.** `Trace.lean` lost `sampleAtOpt`, `latestOf`, `runMax`, `AscendingLatest` (now plain
`Ascending optNat` on the update trace; `Ascending.of_pairwise_some` in `PaxosBench`).
`takeWhile_prefix_mono`, `Ascending.of_prefix`, `ascending_iff_pairwise` (decidable
ascent), `lastUpdate_prefix_le`. `by_ref` handles only work in `Stream` closures (a
`Singleton`/`Optional` closure panics at `handoff_ref.rs:80`).

**Queue after D89.** Ghost parameters (above); the `Optional` carrier and B2 (`.monotonic`
as a marker with `IsMonotone` theorems) if the proof-as-argument route is not taken;
`TO × ALO batch_ordered` (unused); `hasPending` makes every tick-hosting process spin
(43k activations of an idle proposer in 10 s — a timer-driven idle like the Rust's);
then the D87 queue.

### D90 — The trace at the bytes: `fifo` becomes a definition, the shells stop bundling

**The question that started it.** "`taken_lt` and `fifo` could in theory be accidentally
too strong and exclude real runs — how do we reason about that, and guard against it?"
Reviewing the D89 `WireTrace` against the shells found exactly that bug class, twice:

1. **Empty frames.** The shells sent a frame to a peer only `if !mine.isEmpty`; `fifo`
   demanded the consumed activations be a prefix of *all* the sender's activations,
   including those that sent the receiver nothing. The shell's own trace violated the
   theorem's hypothesis; it was saved by an unproved argument (consuming an empty batch is
   a no-op) that lived in nobody's head but the author's.
2. **Granularity.** `taken` consumed whole activations. That matched the shells only
   because they bundled one `Line` per (activation, destination). DFIR does no such
   bundling: every message is its own frame, and a receiver's tick sees whatever prefix of
   the byte stream TCP has delivered — half a frame included. The model excluded real runs
   of the real system.

**The fix is to stop assuming and start defining.** A `WireTrace` is now four functions —
`who`, `inp`, `readings`, `reads : Nat → List (Member × Nat)` (how many bytes to read off
each connection this activation) — with **no proof fields**. The run (`WireTrace.run`)
keeps, per connection, the receiver's byte cursor and its kept tail; a step reads the next
`min n available` bytes off each listed connection, hands the body each connection's tail
plus those bytes, keeps the tails it gets back, and puts its sends on the wire as frames.
Every quadruple is a trace. "A connection is a byte queue read in prefixes" is the whole
trust statement about TCP; the fifo predicate, the `taken_lt` linearization argument and
the empty-frame repair are all gone because there is nothing left to assume. Crash-stop is
a trace that stops activating a process. (TCP-vs-UDP as a declared requirement on a
network edge — the Rust's choice of transport — stays queued.)

**The framing law.** `Wire.frame x := encode (encode x : ByteArray)` (varint size, then
the bytes — the 4-byte header is gone, so no `n < 2³²` side condition), `frameAll`,
`framesFrom`/`frames` (read every complete frame off a buffer, return the tail).
`framesFrom_frame_append` (a complete frame at the front is read), `framesFrom_partial` (a
strict prefix of a frame reads nothing — via `getNat_natBytes_trunc`: a truncated varint
does not read), and **`framesFrom_prefix_frameAll`**: cutting a stream of frames anywhere
yields the frames that fit (`splitFrames`) and keeps the bytes of the one that does not.
`splitFrames_spec`/`_add`/`_append_of_le`/`_fst_mono` are the arithmetic of the cut;
`tail_append_of_le` says a kept tail does not change when frames arrive behind it.

**The proof.** Two invariants. `RunInv` on the run itself: each cursor is within what the
sender has written, each kept tail is the bytes past the last complete frame
(`tailOf`). From it, `parse_eq`: what an activation parses off a connection is the next
`pc (k+1) − pc k` messages of the stream, where `pc k m src := (splitFrames stream cur).1`
is the parsed count — the four facts the lock-step side needs about `pc` are `PcOk`
(idle-stable, within the stream, monotone, and after a step still within the stream
*before* the step's own sends), all proved from the run (`WireTrace.pcOk`). Then
`Dag.TraceQueues` (each receiving end holds the connection's unparsed messages seen through
the node's decoder, with the parsed count; each timer cell the activation count), the
mutual init/idle/active/arrivals/timing theorems, and `Inv` → `act_eq` →
`streamOutputsAt_eq`, as in D88 but with the typed sends gone from the invariant: the
queue facts are stated at the wire (`bcSel`/`dxSel` are `decodeAt`'s filter, `view_active_wire`
joins `view_idle_wire`), and the per-node proofs never see a typed message. `Interleaving.Batched`
generalized: any `batches` whose `inboxOfWire` is the arrivals. `paxos_safe_wired` is
restated over the new `WireTrace` with the same conclusion and the same two hypotheses
(`hnA`, `hck`).

**Production.** `Dag.iterateBytes` (`Process.lean`): per connection the unread bytes (kept
tail ++ what the socket delivered), `parseFrames` → `iterateWire`, the new tails returned
one per connection. `Project/Shell.lean` is the shells' share: `hello` (the dialer's first
byte names it — connection setup, not traffic), `groupChunks` (the readers' `(connection,
bytes)` chunks per connection in order of first arrival), `chansOf`, `storeTails`. Both
shells now push raw chunks from their reader threads and send `frameMsgs mine` (one frame
per message); `Line` is gone. `wired`'s replica checkpoints — harness traffic — moved to
their own sockets (`ckPortOf`), so the program's connections carry the program's frames and
nothing else (the theorem's stream is exactly the senders' `wireFor`).

**Scenarios (the E of the review).** `Hydro/WireTraceCheck.lean` writes five traces on the
Paxos graph and `lake exe wired --traces` evaluates them natively (4 s): `sync` (round robin,
read everything), `trickle` (five bytes per connection per activation — every frame cut
mid-way), `silent` (nothing read: nothing commits), `stalled` (acceptor 2 never runs: a
majority commits), `lopsided` (proposers ten times between acceptor activations). Proposer 0
commits its payload in all but `silent`, and `trickle` and `sync` commit the same log.
`runOutputs` is the one-pass evaluator (`streamOutputsAt_eq_runOutputs`; `streamOutputsAt`
itself recomputes the run per step — fine as a spec, quadratic to evaluate). Two lessons:
`#guard` on these runs in the library takes minutes (the elaborator interprets IR; the
exe compiles it), and a payload handed to `paxos_core` before its proposer leads is
dropped (`paxos_with_client` is what routes to the leader) — the scenarios hand it on an
activation after the election.

**Measured.** `wired` demo and `--bench 2000 20 10` OK (8.3k commits/s at batch 20;
`--bench 10000 100 1000` 39k commits/s, the checkpoints over the side channel); `bench 500 10 20000` OK at 1.23M requests in the aggregated
intervals (≈68k/s), per-process RSS flat over the run (leader ≈220 MB, others 160–185 MB;
`time -v` on the launcher reports ≈910 MB — a property of how `ru_maxrss` sums over the
ten OS processes, not of any one of them).

**What stays outside the theorem.** The sockets (that `recv` returns a prefix of what the
peer wrote, in order — TCP), the Lean compiler and the twelve `@[csimp]` seams, and the
1:1 mirror argument (CORRESPONDENCE). Byzantine senders: `frames` is total on any bytes
(undecodable frames are dropped) but the theorem speaks only of streams the program wrote.

**Queue after D90.** A pure `source_interval` in place of `source_interval_batch`: the
Rust's `source_interval(q!(period))`/`source_interval_delayed(q!(delay), q!(period))` is an
unbounded `Stream<Instant>` at the location — the `.batch(&tick, nondet!(…)).first().is_some()`
fused into today's primitive is the *program's* doing (`PLeaderHeartbeat`, `sample_every`,
`compute_throughput_latency`, `aggregate_bench_results`), so the primitive should be the
stream of fire instants and the batch-and-test a program over it, as `timeout`/`sample_every`
became programs over `time` in D89 (the `PulseDec` family becomes a `BatchDec` on that
stream; `TickReadings.pulse` becomes the instants delivered this activation; the pulse-fence
arms in `Project` go). Transport as a declared requirement on the network edge (TCP/UDP, as
the Rust chooses); `frameAll` is right-nested (`frame x ++ frameAll xs`, quadratic to
build — the shells frame per activation so it does not show; a left fold with the same
lemmas when it does); the D89 queue (ghost parameters, the `Optional` carrier, the idle
`hasPending` spin, D87 perf).

### D91 — `source_interval` as the Rust has it; one graded `batch`; the interval timer is a sched-det

**The question.** D89 left an interval timer that was not the Rust's: `source_interval_batch
(delay) (period) (d : PulseDec)` fused `source_interval_delayed(…)` with the *program's*
`.batch(&tick, nondet!(…)).first().is_some()` into one primitive whose decision was a per-tick
`Bool` pulse list. Four call sites (`p_leader_heartbeat`, `sample_every`,
`compute_throughput_latency`, `aggregate_bench_results`) spelled the Rust's three operators
as one Lean one, and the primitive's output was a tick singleton the Rust never has. The
queue item was: make the primitive the stream and the batch-and-test a program over it.

**The ruling that decided the shape (user).** Rust's `source_interval` yields
`Stream<(), L, Unbounded, TotalOrder, …>`. Nothing about the stream's *content* is
nondeterministic once the grade is right: every non-empty sequence of `()` is one
`AtLeastOnce` equivalence class, so the stream's denotation is one value — the class of
`[()]` — with NO content decision at all. The timer's only freedom is *when* its fires reach
a tick, and that is runtime freedom the denotation never sees: a **sched-det**, like a
transport cursor. Hence:

- `source_interval (delay : Fin (mem ℓ) → Nat) (period : Nat) (d : IntervalDec (mem ℓ)) :
  Stream ℓ Unit .totalOrder .atLeastOnce`. `IntervalDec n` is `Unit` at `Values`, a
  `TickScript n Nat` (fire count per tick) at `Sched`, and "fires this activation : Nat" at a
  process (`Node.Sched`, the `Wired` oracle). At `Values` the value is `StutterSeq.mk [()]`,
  constant. The corner's `cpl` is `stutter_le_unit`: any `()`-history the machine produces is
  ≤ `[()]` in the stutter order, for every script and horizon — `wf := True`.
- The program's `.batch(&tick, nondet!(…))` is then a `batch` at `TotalOrder × AtLeastOnce` —
  a grade `batch` did not have. Rust has ONE `Stream::batch`; the Lean had `batch` (NoOrder,
  either Retries) and `batch_ordered` (TotalOrder × ExactlyOnce) with two decision families.
  Ruling: one `BatchDec : Nat → Type → StrOrd → Retries → Type` and one `batch`, dispatched by
  grade (`BatchCutsOf`): multiset increments at `NoOrder × ExactlyOnce`; at `NoOrder ×
  AtLeastOnce` **sets** drawn from the pool (`RetryBatchCuts n α := Fin n → List (Finset
  α)`; user, on review: "it should pick a set, and the elements remain in the pool" — the
  D89 version decided a multiset whose multiplicities the class forgot on the spot, a
  redundant decision space; nothing is consumed, so an element may appear in any number of
  batches); slice sizes at `TotalOrder × ExactlyOnce`; and at `TotalOrder × AtLeastOnce`
  the **realized batches themselves**
  (`StutterBatchCuts n α := Fin n → List (List α)`), legal when `destutter (acc ++ b)` is a
  prefix of the pool's normal form (`stutterBatchCuts`; an illegal batch truncates, as
  `batchCuts` does — P2's totality). `batch_ordered`/`OrdBatchDec`/`PulseDec`/
  `source_interval_batch`/`TimingPulses` are gone.
- `.first()` on a tick stream outside a `sliced!` block (Rust `Stream::first`, `O: IsOrdered`)
  is the new tick-level `firstTick : AtomicStream ℓ α .totalOrder ret → Atomic ℓ (Option α)`;
  inside a block it is `bfirst`, now over any `ret`.

**The Rust side (same change, same reason).** `location/mod.rs:1413`/`:1439`:
`source_interval`/`source_interval_delayed` return `Stream<(), Self, Unbounded, TotalOrder,
AtLeastOnce>` via `weaken_retries` with the `manual_proof!` text above; `singleton.rs` and
`stream/mod.rs` (`sample_every`) take `BatchHook<(), TotalOrder, AtLeastOnce>`; raft's
interval parameters become `AtLeastOnce` with `count().map(n > 0)` → `is_empty().map(!e)`
(a count of an `AtLeastOnce` stream is not a function of the class; emptiness is); the raft
example/test adds `.assume_retries::<ExactlyOnce>(nondet!(…))` on the client `requests`;
`SimClusterSender::send` (sim/compiled.rs) is generalized over the retries grade at
`TotalOrder`; the simulator's `batch` (sim/builder.rs) accepts any retries grade — it was
`todo!` at `AtLeastOnce`, and the sound subset (no re-deliveries) is a legal batching.
`cargo check/clippy/fmt` clean; `hydro_std` 11/11, `hydro_test` lib 67/67 incl. the raft
simulation tests; snapshots regenerated (`paxos_ir`, `two_pc_ir`, `compute_pi_ir`,
`raft_ir` + mermaids — the expected new `Cast` and `ObserveNonDet { trusted: true }` nodes).

**Per site (Rust ↔ Lean).**

| Rust | Lean (D91) |
|---|---|
| `paxos.rs` `p_leader_heartbeat`: `source_interval_delayed(q!(…CLUSTER_SELF_ID…), q!(i_am_leader_check_timeout)).batch(&tick, nondet!(…)).first()`, `.and(…)` | `election_interval := H.source_interval delay period sched.interval`; `election_fire := H.firstTick (H.batch election_interval dec.interval)`; the `.and` as before; `trigger_gate` re-anchored on `election_fire i` |
| `optional.rs` `sample_every`: `source_interval(interval).batch(&tick, nondet).first()` + `filter_if_some` | `samples := H.source_interval 0 interval sched.interval`; in the tick `boFilterIf snap (boIsSome (bfirst (batch samples dec.samples)))`; `allTicks`, `weaken_retries` |
| `bench_client/mod.rs` `compute_throughput_latency`: `source_interval(…).batch(&tick, nondet!(…)).first()` → `is_some()`/`is_none()` | `CTLDec.punctuation : BatchDec … Unit .totalOrder .atLeastOnce` + `CTLSched.punctuation : IntervalDec`; `bfirst`/`boIsSome`/`boIsNone` in the tick |
| `bench_client/mod.rs` `aggregate_bench_results`: `source_interval(…).batch(&tick, nondet!(…)).first().defer_tick()` | `punctuation_option := H.firstTick (H.batch punctuation dec.punctuation)` before the tick; `reset := H.defer_tick none punctuation_option` |
| every other `.batch(&tick, nondet!(…))` (≈29 sites) | `H.BatchDec n α .noOrder .exactlyOnce` spelled out (the `Timers` arrivals batch `.noOrder .atLeastOnce`; `sequence_payload`/`paxos_with_client` payload batches `.totalOrder .exactlyOnce`) — the grade was implicit before, it is in the record type now |

**Faces changed.** `Sem.lean` (`BatchDec` graded, `batch` over all grades, `IntervalDec`,
`source_interval`, `firstTick`; classification table: sched-det row = `TransportDec`,
`IntervalDec`); `Decisions.lean` (`StutterBatchCuts`, `BatchCutsOf`); `Grades.lean`
(`StutterSeq.destutter_head?`/`head?`, `poolHead?`, `destutter_replicate_unit`,
`stutter_le_unit`); `Trace.lean` (`stutterBatchCuts`, `_le`, `_mono_dec`; `retryBatchCuts` over `Finset`,
`RetryPool.mk_toFinset_val`); `Transfer.lean` (`stutterBatchCuts_real`,
`batchStutterDerive(_mono)`, `batchRetryDerive(_mono)`, `corr_batch_stutter`); `Sched.lean`
(`scriptSum`, `intervalHist`, `replicate_prefix_of_le`); `Values`/`MonoRel`/`Couple`/
`CoupleProj` (`CoDec.batch` with implicit grades so `CoDec.batch _` call sites stay stable,
`CoDec.interval`, `co_batch_stutter_*`, `co_source_interval_*`, `co_firstTick_*`);
`SchedMono`/`SchedPaced`/`SchedCausal` (`tmono_batch`/`tpaced_batch`/`causal_batch` generic;
`*_firstTick`, `causal_source_interval`); `HydroRel`/`CausalHRel`/`MonoHRel` (graded
`batchRel`, `intervalRel`; the positional law lists); `HydroGen` (`DecFam.interval` is a
sched family; `.ordBatch`/`.pulse` gone); `HydroDef` (census counts `IntervalDec` as a
sched); `HydroGenProject` (`isSchedDecTy`, node map). `Project`: `Node.batch iα ord ret x d`
(one constructor), `Node.source_interval delay period` (a stream source), `Node.firstTick`;
`Node.Sched` carries `TickScript (mem ℓ) Nat`; `Step` emits `List.replicate (fires) ()` at the
tick; `Wire.lean` `TickReadings.fires`/`firesUpTo`/`fireCount` replace `pulse` (a process
counts the fire instants in `(last, now]` from one clock reading — tokio `Burst`);
`StepOps` `intervalHist_incView`, `spec_source_interval`, `spec_firstTick`; the `firstTick`
arms of `WireTrace`/`WiredSpec`/`WiredMain`/`DistributedModel`/`Process`/`Profile` are the
`mapTick` arms over `Option`. Toys: `toy_batch_stutter` (`HydroTickCheck`, two `#guard`s:
a legal realized batch sequence and an illegal one truncating), `toy_relay` rewritten over
`ToyRelayDec {batch, gate : BatchDec n Unit .totalOrder .atLeastOnce}` + `ToyRelaySched
{interval}` (`HydroGenToy`, `HydroGenCheck.toy_safe_sched`), `TransferChecks.firesSkew`.

**Censuses.** Nondet counts are unchanged everywhere except the toy; sched counts grow by one
per interval timer, as the ruling says they must: `sample_every` 1/0/0 → 1/1/0;
`compute_throughput_latency` 2/0/0 → 2/1/0; `aggregate_bench_results` 3/2/0 → 3/3/0;
`p_leader_heartbeat` 5/1/0 → 5/3/0 (its own timer + `sample_every`'s); `leader_election`
10/3/3 → 10/5/3; `paxos_core` 16/5/5 → 16/7/5; `paxos_with_client` 19/7/6 → 19/9/6;
`paxos_protocol` 23/10/9 → 23/12/9; `paxos_bench` 34/12/11 → 34/16/11 (`PBSched` gains
`ctl : CTLSched`); new `toy_batch_stutter` 1/0/0. 39 build-enforced censuses (GATE §5).
`paxos_core`'s sched record is now (clock, p1aCh, hb.ial, hb.sample.interval, hb.interval,
ap1.p1bCh, sp.p2aCh, sp.ap2.p2bCh) — `paxos_core_param₂` has seven sched relations; every
scenario record (`Exploration`, `Falsification`, `PaxosDemo`, `Falsify`, `LeaderElection`'s
`leDec`, `HydroParamCheck`, `CoupleSafety`) exchanges its `Bool` pulse lists for `List (List
Unit)` realized batches. `cq_safe_sched`'s witness type reads `BatchDec 1 (K × Except E Unit)
.noOrder .exactlyOnce` — the same set, the grade now written (recorded as a representation
change to the statement, GATE §7).

**Budget.** `PaxosCore`'s override is 300k (was 250k): the re-measured sum is ≈ 251M
(`_param_all` 88M, the two knot `wf`s 64M/55M, `a_log.inv` 54M — the two extra sched
relations and the extra `source_interval`/`firstTick` nodes in the chain) at an unchanged
≈45 s wall; no single declaration grew out of proportion, so this is the measured-sum rule
of D72 and GATE §6 applied, not a blowup. `LeaderElection` 600k unchanged. Flagged for
ratification with the entry.

**Gate.** Library + six exes green (1950 jobs); zero sorries; axioms standard-three subsets;
seams unchanged (12 `@[csimp]`, no `implemented_by`/`unsafe`/`native_decide`); `wired`
(demo, `--traces`, `--bench`) and `bench 500 10 6000` OK; `bench 500 10 20000` ≈60k req/s,
RSS as D90.

**Gotchas (ENGINE_NOTES).** `instDecidableEqUnit` is not a constant — `autoImplicit` made it an
implicit binder of the new `Node` constructor and every projection stopped reducing
(`instDecidableEqPUnit` is the instance). HRel `Laws` lists are positional per `HydroSem`
field: a new field means a new entry at the same index in every interpretation's list.
Duplicating a sibling's arms to build a new node's (`.mapTick` → `.firstTick`) leaves
doubled arms the match compiler rejects as redundant.

**Queue after D91.** Transport (TCP/UDP) as a declared requirement on the network edge;
left-fold `frameAll`; the D89 queue (ghost parameters so `latest_monotone` takes its ascent
proof, an `Optional` carrier, the idle `hasPending` spin); D87 perf (leader step drift, fused
per-activation walks, single-pass `keyedFoldAssocImpl`, decoder without `Except` per field,
`reduce_watermark` payload path); and the carried-over list (forward_ref fidelity, bare-value
tick-singleton increments, `paxos_with_client_safe_wired`, higher-order `hydro def`, liveness
rung 2, `SchedSem` against production DFIR, upstream Rust reports in SCHED_AUDIT).

### D92 — Profile-driven throughput: 62k → 120k requests/s; the wall is the leader's per-tick state re-walk

**The directive.** A new round of performance work, each change justified by a profile
before and measured after, towards 400k requests/s (the Rust's figure for the same
benchmark). The method is in ENGINE_NOTES "Profiling the benchmark"; the instrument is
`scripts/bench_perf_role.sh <proc>`: it starts `bench 500 10 …`, finds the OS process
`bench --proc P` by reading `/proc/*/cmdline` (never `pgrep -f`, which matches the caller's
own shell), attaches `perf record --call-graph dwarf` to it, and prints the flat self
profile, the inclusive profile, and the **machine thread alone** — `perf record -p` samples
every thread of the process, and the reader and writer threads (condition-variable
hand-offs through libuv's loop) are ~60 % of a process's user cycles and none of its
throughput (the leader process is 163 % of a core; the machine thread is 100 % of one).
Where the inclusive report mis-attributed (partial DWARF unwinding), `perf script -F ip,sym`
with an awk over the stacks gave callers and "share of samples whose stack contains X".
Baseline reproduced at the start: `bench 500 10 20000` 61.5–64.5k requests/s, p50 16 ms.

**Rounds.** Each is profile → hypothesis → change → measurement; everything kept is in the
tree, the one reverted experiment is recorded.

1. *Harness.* `machine` pushed every activation's duration onto an `IO.Ref (Array Nat)`
   created on another thread: the array was multi-threaded-marked, so every `push` copied it
   and `lean_mark_mt` re-walked it — O(n) per activation, the "leader step drift" D87 queued.
   `nanos` is a loop argument now (D84's rule, missed at one site). `Dag.iterateBytes` called
   `parseFrames` twice per connection (messages, then tails): `Dag.iterateBytesImpl` parses
   once, `@[csimp] Dag.iterateBytes_eq_impl` (the equation round 3 extends to the one walk). `Wire.getNat` shifted with `<<<`, and the
   runtime's `lean_nat_shiftl` builds a GMP integer even for small operands (7 % of cycles):
   `getNatImpl` carries the multiplier `2 ^ shift` (`pow2`, since `Nat.pow` is a GMP call
   too), `@[csimp] getNat_eq_impl`. **62k → 85k**, p50 11 ms, drift gone.
2. *Keyed fold.* `keyedFoldAssocImpl` walked its input twice (keys, then groups) through a
   `Std.HashMap` grown from empty (`expand_go` rehashing was 12.5 % of a client's cycles):
   `groupKeys` is one right fold into `HashMap.emptyWithCapacity l.length`, keys in
   `List.dedup` order (`groupKeys_spec`); `hashSetOfList` pre-sizes the sets of
   `listFilterNotInImpl`/`listAntiJoinImpl`/`payloadsImpl`. **85k → 92k.**
3. *One walk.* An activation walked the typed graph six times (inbox, enqueue,
   materialize, timing, oracle, step, encode). `Project/Fused.lean`: `Node.activatePlain`
   does everything for one node, `Node.activateFused`/`Dag.activateFused` for the graph,
   skipping untouched nodes whole (`Node.enqueueInbox_idle`, `Node.locStep_idle`,
   `Node.wireEncode_idle`); `Node.activateSpec`/`Dag.activateSpec` are the unfused
   compositions, `Node.activatePlain_eq` and the mutual `Node.activateFused_eq`/
   `Dag.activateFused_eq` the equalities; `Dag.iterateBytesImpl` the fused form and
   `@[csimp] Dag.iterateBytes_eq_impl` the equation — on `Dag.iterateBytes` itself, the
   function `paxos_safe_wired`/`WireTrace.act` are stated over, which the harnesses call
   verbatim (user ruling on review: no named abbreviation between theorem and binary — a
   first version had `Dag.iterateBytesR`, `iterateBytes` at a `TickReadings`, so that the
   fused walk could take readings; it now takes the `Dag.Timing` structure apart node by
   node, as `Dag.oracleOf` does). Root cause of the walk's cost: `Ctx = List Kind` is
   runtime data, so every walk recomputes `κs ++ Γ` per `letN` node (`List.appendTR` →
   `List.reverseAux`, 10 % self). Idle activation 145 → 79 µs; **92k → ~100k**, p50 9 ms.
4. *Routing at the sender.* A `demux` message was sent to every process of the receiver
   cluster and each decoded it to drop it. `Dag.wireRoutes` is the routing table with the
   demux address function; `Shell.routeSends` buckets one pass per peer by address
   (`demuxTarget`: the payload's first varint — every demux payload codec is
   `Codec.ofWire` of the `Prod` instance, so the address is the first field);
   `HYDRO_BENCH_NODEMUX=1` restores cluster routing for A/B. Measured: clients decode half
   the acks (54 → 22 messages per activation), the standby proposer receives nothing,
   **throughput unchanged** (92–97k both) — kept as the faithful routing. The peek is a
   theorem (`demuxTarget_ofWire`: on a `Codec.ofWire` payload of `Nat × α` it reads the
   address — added on review); which member a process hosts stays a harness fact. The
   headlines quantify over all byte histories, so a wrong table is a dropped message
   (liveness), never a safety case.
5. *The lazy-tick experiment (reverted).* Hydro compiles every `defer_tick` to DFIR
   `defer_tick_lazy` (`compile/ir/mod.rs:4819`; `dfir_lang/src/graph/ops/defer_tick_lazy.rs`:
   buffered items do **not** schedule a tick), so a production process is idle between
   arrivals, while `Dag.hasPending` (D86) ticks whenever a `tick_scan` is hosted — every
   `bench` process spins at 100 % (clients had input on 3 % of activations, the leader on
   27 %). Making the clients and aggregator idle worked; making the `.atomic` knot
   feedback lazy **deadlocked leader election**: same-tick `forward_ref`s (acyclic tick
   knots) are emulated by `fixN` feedback delivered at the next activation (D76), so the
   knot needs the tick DFIR does not schedule. Reverted; the fact is recorded at
   `Profile.lean`'s "Deferred work" section and SCHED_AUDIT. Idle clients did not change
   throughput. Prerequisite: forward_ref fidelity (acyclic tick knots as same-tick lets).
6. *`frameAll`.* The right-nested `frame x ++ frameAll xs` copied the tail at every frame
   (quadratic per bucket): `frameAllImpl` is a left fold, `@[csimp] frameAll_eq_impl`.
7. *Decode buckets.* `Node.inboxOfWire` scanned every batch for every (network node,
   sender) pair — ~10 scans of ~230 messages per leader activation (7 % self).
   `bucketArray n batches` groups each batch's messages by node in one pass
   (`groupRev`; `bucketMsgs_groupRev`, `bucketArray_get`, `bucketArray_getD`), the fused
   walk takes `buckets : Nat → List (src × List WireMsg)` with `∀ k, buckets k = bucketOf
   batches k` (`Node.inboxOfWire_bucketOf` via `decodeAt_filter`). `Dag.wireCount` is the
   node count without the table (`Dag.wireCount_eq`). Scan 7 % → 0.5 % at **unchanged
   throughput** (96k): the saved cycles moved to the buckets' allocation and the program
   bodies; kept for the cleaner profile and the acceptors/clients.
8. *Framing in place.* `frame (k, j, p)` for `WireMsg` built three `natBytes` lists, their
   arrays, two inner appends, the outer append, then `frameAll`'s — ~10 allocations and
   four copies of the payload per message (8 % of the machine thread), and
   `Shell.routeSends` allocated `List.finRange n` and called `peerOf` per message.
   `Wire.pushNat`/`natLen` (`pushNat_eq`, `natLen_eq`), `frameOneInto` writes the frame
   onto the buffer (`frameOneInto_eq`), `frameMsgsImpl` folds it, `@[csimp]
   frameMsgs_eq_impl`; the peers are computed once per `routeSends`. **96k → 110k**
   (119k at 1000 virtual clients), p50 8.8 ms.
9. *Encoding in place.* The `Wire` class gets `put : α → ByteArray → ByteArray` with the law
   `put_eq : put x b = b ++ encode x` (default: the law itself); `encode` stays the
   specification every proof is about, `put` is what runs: `Codec.ofWire.enc` is
   `putEmpty = put x (emptyWithCapacity 64)` (`putEmpty_eq`, `Codec.ofWire_enc`). `Nat`/`Fin`
   push LEB128 bytes (`pushNat`), `Bool`/`Option`/`Except` push their tag, `Prod` chains,
   `List` folds (`foldl_put_listBytes`), `ByteArray`/`String` push size then bytes,
   `ofEquiv` forwards — every program type inherits it. `Node.wireEncode` 9.4 % → 2.2 % of
   the machine thread. **110k → 116–121k** at 500 virtual clients (p50 8.0 ms, p99 9.3),
   **133k at 1000**.

**Where the time goes now (leader machine thread, share of samples whose stack contains
the function).** `Dag.activateFused` 83 %: tick bodies `sequence_payload` 41 % —
`collect_quorum` 24 % (`keyedFoldAssocImpl` 19 %), `join_responses` 17 %
(`listAntiJoinImpl` 17 %, `listJoinImpl` 8 %); inbox decode (`Node.enqueueInbox` 7.6 %,
`decodeAt` 5.2 %, `parseFrames` 5.5 %); `HList.map` for `call` arguments 6 %; `bucketArray`
2.4 %; `Dag.wireCount` 2.6 % (a walk is a walk: `κs ++ Γ` per node); encode 2.2 %;
`frameMsgs` 1.6 %, `routeSends` 2.1 %. Allocation is 31 % (`lean_dec_ref_cold`) + 11 %
(`mi_free`) + 8 % (`mi_malloc`), spread across all of the above. At 500 virtual clients the
leader takes ~380 µs per activation for ~45 requests, 8.5 µs per request; raising the
in-flight count only adds latency (1000 → 133k at p50 14.5 ms, 2000 → 106k before round 8
at p50 36 ms): the leader's CPU is the wall, not the pipeline.

**Why 400k is not reachable in this architecture.** 400k at one leader is ≤ 2.5 µs of
leader time per request. The two tick bodies that are 41 % of it are the program's
`use::state` semantics taken literally: every tick re-folds the whole in-flight set
(~2000 entries at 500 virtual clients: `collect_quorum`'s keyed fold over all pending
quorums, `join_responses`'s anti-join and join over all pending requests) at ~60 ns per
entry per operator, Lean's constant for a boxed `(Nat × Ballot)` key through the
polymorphic `Hashable`/`BEq` instances (the hash itself is 1.2 % — the cost is the walk and
the allocation of the rebuilt lists). DFIR's `fold_keyed::<'static>` and
`anti_join::<'static>` are *incremental*: a tick folds the new elements into kept state
(re-emitting the state, not recomputing it). Matching
that needs incremental `use::state` — a persisted index per keyed operator with the
denotation unchanged (the same `@[csimp]` discipline, a far larger equation) — or
compiled tick bodies. Everything outside the program bodies (decode, encode, framing,
routing, the walk, 59 % at the start of the campaign) is now ~35 % and shrinking returns:
the next candidates are each 2–3 % (the decoder's `Except`/pair allocations per field,
`HList.map` de Bruijn lookups, the `κs ++ Γ` recomputation, a bucket count from the
harness instead of `Dag.wireCount`). The honest summary is 62k → 120k (×1.9) with every
step kernel-checked, and the remaining ×3.3 is one architectural change away, not a
hundred small ones.

| | baseline | after D92 (500 vc) | after D92 (1000 vc) |
|---|---|---|---|
| requests/s (steady windows) | 61.5–64.5k | 116–122k | 133k |
| p50 / p99 latency | 16 ms / — | 8.0 / 9.3 ms | 14.5 / 17.2 ms |
| leader activation | ~390 µs for ~45 requests, drifting +5 %/20 s | ~380 µs for ~45 requests, flat | ~620 µs for ~75 |
| leader RSS | 232 MB | 221 MB | — |

`wired`'s open-loop `--bench` mode (the leader spoon-fed payloads in one OS process, a
sub-second window dominated by startup, with a harness checkpoint channel standing in for
replicas) was removed on review: it measured nothing the benchmark does not measure better,
and the witness should be only the witness. `wired` takes `--traces` or nothing.

**Faces.** `Wire.lean` (`put`/`put_eq` in the class, `putEmpty`, `pushNat`/`natLen`,
`getNatImpl`/`pow2`, `frameAllImpl`, `Codec.ofWire` by `put`, `Codec.ofWire_enc`);
`Project/Wire.lean` (`Node.wireRoutes`/`Dag.wireRoutes`, `demuxTarget` + `demuxTarget_ofWire`,
`Node.wireCount`/`Dag.wireCount`); `Project/Process.lean` (`Dag.iterateBytesImpl`, `frameOneInto`,
`frameMsgsImpl`); `Project/Fused.lean` (new); `Project/Shell.lean` (`routeTable`,
`routeSends`); `KeyedHash.lean` (`groupKeys`, `hashSetOfList`); `Profile.lean` (the
`defer_tick_lazy` comparison); `BenchDemo.lean` (`nanos` as a loop argument, `peers`
indexed by process, `recv` with the demux address, `HYDRO_BENCH_NODEMUX`); `WiredDemo.lean`
(`--bench` and the checkpoint channel removed; the harness calls `Dag.iterateBytes` verbatim);
`scripts/bench_perf_role.sh`. Five new `@[csimp]` equations (`Dag.iterateBytes_eq_impl`,
`getNat_eq_impl`, `frameAll_eq_impl`, `frameMsgs_eq_impl` — `Dag.iterateBytes_eq_impl` is
one equation: parse once, bucket, one walk): sixteen in all, each kernel-checked (GATE §4).

**Gotchas (ENGINE_NOTES).** An `IO.Ref` created on another thread marks what is stored in
it; `Nat.shiftLeft`/`Nat.pow` are GMP calls in the runtime even on small operands; a
`Std.HashMap` grown from `∅` rehashes log n times — size it; in a mutual `Node`/`Dag`
theorem, rewrite the memo identities while the terms are still `Node`-typed (after the
`_call` equations the sides are cross-typed and `rw`/`simp` fail "not type-correct");
`Codecs` of a constructor are not syntactically `Codec α` — instantiate the lemma
(`decodeAt_filter cod …`) rather than `rw` with it; `perf record -p` is the whole process
— separate the machine thread; `pgrep -f` matches the shell that runs it.

**Gate.** Library + six exes green (1952 jobs); zero sorries; axioms standard-three
subsets (132 reports); seams: sixteen `@[csimp]`, no `implemented_by`/`unsafe`/
`native_decide`; `wired` and `wired --traces`, `bench 500 10 6000` and
`/usr/bin/time -v bench 500 10 20000` OK (117–122k, p50 8.0 ms, leader RSS 221 MB, step
first/last quarter 383/381 µs); 39 censuses unchanged. Compile times: `PaxosBench` 114 s,
`DistributedModel` 85 s, `LeaderElection` 83 s, `Process` 78 s, `PaxosCore` 50 s,
`SequencePayload` 46 s, `WireTrace` 43 s, `Fused` 16 s (new); `WiredMain`/`StepOps` not
touched. Heartbeat sites unchanged (LE 600k, PaxosCore 300k).

**Queue after D92.** Incremental `use::state` for the keyed operators (the wall);
forward_ref fidelity (acyclic tick knots as same-tick lets — also the prerequisite for
DFIR-faithful lazy ticks, round 5); the decoder's per-field `Except` allocations; a bucket
count from the harness (`Dag.wireCount` is 2.6 %); the `κs ++ Γ` recomputation (a `Ctx`
the compiler could erase); `HList.map` for `call` arguments; the upstream note that
`defer_tick_lazy` is the only `defer_tick` Hydro emits (SCHED_AUDIT); and the carried-over
list (transport as a declared edge requirement, ghost parameters for `latest_monotone`,
`Optional` carrier, `paxos_with_client_safe_wired`, higher-order `hydro def`, heartbeat
tax, liveness rung 2, `SchedSem` against production DFIR, upstream Rust reports).

### D93 — The DFIR activation: a tick, then settling passes; the wire headline over DFIR's own schedules

**Discovered:** D92's round 5 — making the shells idle between arrivals as DFIR does
(`defer_tick_lazy`) deadlocked leader election; the lazy pacing was deferred behind
"forward_ref fidelity". This finding is the diagnosis, the fix inside the loop body, its
proof, and what the fix is not.

**What DFIR does.** A tick runs at start, on a socket or timer wake, or for a non-lazy
`defer_tick` (`dfir_rs/src/scheduled/context.rs:391–408, 50–53, 119`;
`meta_graph.rs:2131–2219`) — and Hydro never emits a non-lazy one
(`hydro_lang/src/compile/ir/mod.rs:4819`). Every Rust `forward_ref` is a same-tick alias
(`CycleSink → identity()`, `compile/ir/mod.rs:1790`; `forward_handle.rs:103`); within a
tick only `defer_tick*` edges leave the topological order
(`dfir_lang/src/graph/flat_to_partitioned.rs:17–40, 187`). So in one DFIR tick a knot's
production reaches its consumer, and a tick input downstream of it, within the tick.

**What the model does.** A knot's feedback crosses one *active* step (`KnotKind.feed`,
Step.lean), and a tick input (`batch`) accumulates on non-tick steps and emits on a tick
(`Step.lean:427`). With one pass per activation, a knot-dependent send — the proposer's P1a
(`p_is_leader`), the acceptor's P1b (`a_log`) — left at the *next* activation; under
DFIR's wake rule that activation never came. Reproduced on the five-process graph with a
DFIR-rule scheduler (activate only on first run / unread bytes / due timer): one pass per
activation stalls at step 5 ("a stutter would send 1 msg: node 1 → acceptors"); the
spinning harness commits at 25; a tick followed by stutters commits at 30; stutters
*before* the tick stalls at 10 — D76's wording "stutters, then a tick" was the wrong way
round (the tick's own knot productions are what the stutters carry). Then, with the knots
settled, `bench` still collapsed (54k → 10k → 500 → 0 requests/s, latencies quantized at
the 100 ms pulse): the client's trace read `in 27 bytes out 497` / `in 9456 bytes out 3`
— a response crosses `paxos_bench`'s knot slot 2 (one stutter), the generator's request
knot slot 1 (a second stutter), and lands in `payloads_to_leader`'s `batch`, where only a
tick consumes it; the same at the leader through `paxos_with_client`'s knot into
`sequence_payload`'s batch. Stutters do not tick.

**The fix (user-ruled, option "proved instead of harness-level"):** the activation
`Dag.iterate` is one pass, then **`Dag.settle`**: `Dag.pass m false` while a knot slot
of the member holds feedback (`Dag.knotPending`), `Dag.pass m true` while a tick input
holds items (`Dag.batchPending`) — `Dag.settleFlag` reads the cells and names the next
pass or none — on the empty inbox, the quieted inputs (`envQuiet`) and the **quieted
timing** (`Dag.Timing.quiet`: the clock reading kept, every interval timer's fires taken,
so a settling tick re-fires nothing; `TickReadings.quiet`, `Dag.Timing.ofReadings_quiet`),
bounded by the counter `Dag.settleFuel = (batchCount + 1) · (knotCount + 1)`. The
outputs merge (`envMerge`), the sends come pass by pass. `Dag.iterateBytes` is unchanged
in text; the compiled form adds `Dag.settleImpl` (one fused walk per settling pass) and
`@[csimp] Dag.iterateBytes_eq_impl` is reproved (`settleImpl_eq`). **What it is not:** a
settling tick is a second tick of the process's blocks inside the activation — DFIR has one.
The denotation is the same program's (ticks are batch boundaries, every theorem ranges
over them) but tick counts are not DFIR's: the benchmark client spawns one virtual client
per tick, so during its ramp-up every added tick puts a new request into the batch and
the counter runs out (`bench` reports four such activations per client, none elsewhere);
what is pending then is the next activation's, which a shell starts at once. Acyclic knots
as same-tick lets (the activation one pass) remains the faithful change and heads the queue.

**The proof.** The old `WireTrace` bridge — one pass per step, tick flags per step — is now
**`PassTrace`** (five fields: `who tick inp readings reads`; `Dag.passBytes`;
`pass_eq_of_inv`, `PassTrace.streamOutputsAt_eq` to the interleaving). The trusted
**`WireTrace`** keeps its four fields and `Dag.iterateBytes m true` per activation, and
**expands** to a `PassTrace` (`WireTrace.expand`): activation `k` is the block of passes
`blockStart k … blockStart (k+1) − 1` (`passes k = (settleOf k).length + 1`, `blockOf` by
`Nat.find`), the first with the activation's bytes, inputs, readings and `tick = true`,
the rest with nothing read, `envQuietG` inputs, `.quiet` readings and the flag
`passFlag k j = Dag.flagOf` of the cells before the pass. `Expands k` (every process's
cells, cursors and tails agree before activation `k` and before its block; earlier sends are
the passes' concatenated) and `Inner k j` (after `j` passes of the block: the process's
cells are `passCells k j`, the others untouched, cursors and tails the activation's, the
block's sends so far the passes') carry the induction (`inner_one` the tick pass,
`inner_succ` a settling pass, `expands_succ`); **`expands`**, **`streamOutputsAt_expand`**
(the block's merged outputs are the passes' outputs concatenated: `body_outs`,
`foldl_envMerge_get`, `pass_outs` through the dependent cast) and
**`WireTrace.streamOutputsAt_eq`** (`τ.streamOutputsAt T = (τ.interleaving).streamOutputsAt
(blockStart T)`, `WireTrace.interleaving := (expand).interleaving`). `paxos_safe_wired`'s
statement is unchanged; its proof goes through the expansion. Before D93 the headline's
`WireTrace` had `tick := fun _ => true` per step and the shells spun, so DFIR's own
executions were outside its scope; now `WireTraceCheck.lazy` — DFIR's wake rule as a
greedy schedule (a member runs only on its first activation, unread bytes, a due timer, or
work its last activation left pending; the payload arrives when all is quiet) — is one of
its traces: it commits in 15 activations
(`p0 p1 a0 a1 a2 p0 p1 p0 a1 a2 p0 p1 a0 p0 p1`, payload at step 7) and then no member
has a reason to run.

**The shells.** `Dag.hasPending` (Profile.lean) is deleted; `bench` and `wired` wait
unless bytes arrived, a timer is due, it is the first activation, or `Dag.settleFlag` is
still set after the last one. `wired`: 17 activations in 6 ms (it ticked every millisecond
before). `bench 500 10 20000`: **87k requests/s steady, p50 11.5 ms, 476 % CPU for the ten
OS processes, launcher RSS 190 MB** (D92: 116–122k, p50 8 ms, every tick-hosting process at
100 %); the drop is the settling tick per burst at the leader — its per-tick re-fold of the
in-flight state (the D92 wall) now runs twice per burst. The profiler (`HYDRO_BENCH_PROFILE`)
has a `settle` stage; `bench idle P N` times the settling after the tick walk and prints
the knot count, the tick-input count and the counter.

| schedule, body | leader election (5 processes, DFIR-rule scheduler) | `bench 500 10 20000` |
|---|---|---|
| spin (D92 shells), one pass | commits at step 25 | 116–122k, ≈ 1000 % CPU |
| lazy, one pass | **stalls at step 5** | — |
| lazy, tick then stutters | commits at step 30 | **collapses** (54k → 0; the batch cells) |
| lazy, stutters then tick | **stalls at step 10** | — |
| lazy, tick then settling (D93) | 15 activations (`lazy` trace) | 87k, 476 % CPU |

**Faces.** `Project/Process.lean` (`Dag.pass`, `Kind.quietDelta`/`mergeDelta`, `envQuiet`,
`envMerge`, `envMerge_get_stream`, `knotSlotsPending`, `Node/Dag.knotPending`,
`Node/Dag.batchPending`, `Node/Dag.knotCount`, `Node/Dag.batchCount`, `Dag.settleFlag`,
`Dag.flagOf`, `Dag.settleFuel`, `Dag.settle`, `Dag.iterate`, `Dag.passState`,
`Dag.settlePass`, `passState_shift`, `settle_getElem?`, `settle_last_quiet`,
`settle_pending`, `settle_foldl_fst`, `iterate_fst`, `Dag.passWire`/`passBytes`);
`Project/Wire.lean` (`TickReadings.quiet`, `Node/Dag.Timing.quiet`, `ofReadings_quiet`);
`Project/Model/DistributedModel.lean` (`pass_locStep`, `pass_eq`, `loopPass`, `loop_pass`,
`loopPass_outputs`, `loop_pass_wire`, `Dag.pass_false_timing`); `Project/Model/WireTrace.lean`
(`PassTrace`, `ticksOf`/`nthTick`, the induced scripts over `tick`, `WireTrace` with
`expand` and the theorems above); `Project/Fused.lean` (`Dag.settleImpl`,
`activateFused_settle`, `settleImpl_eq`); `Paxos/WiredSafety.lean`; `WireTraceCheck.lean`
(`hasReason`, `lazySchedule`, `lazy`); `Profile.lean`; `BenchDemo.lean`; `WiredDemo.lean`;
`AxCheck.lean` (the new theorems and `Dag.iterateBytes_eq_impl`).

**Gotchas (ENGINE_NOTES).** `rw` into a structure-valued goal — `constructor` first; a
dependent cast over `(τ.expand cod).who t` — `generalize … = w at h ⊢; subst h`, and a
term whose type depends on it must be generalized and reverted first, recovered by `HEq`;
`if bs + 0 = bs` is not `rfl` (`Nat.decEq` on a variable) — `simp only [Nat.add_zero,
eq_self_iff_true, if_true]`; a `foldl` lemma with `?f (j + 1)` does not unify — pass `f`
and use `.trans`.

**Gate.** Library + six exes green (1952 jobs); zero sorries; axioms standard-three subsets
(102 reports; `AxCheck` now imports `Project/Fused` for `Dag.iterateBytes_eq_impl`); seams: sixteen
`@[csimp]`, no `implemented_by`/`unsafe`/`native_decide`; `explore`, `paxos`, `twopc`,
`falsify`, `wired`, `wired --traces` (eight checks), `bench 500 10 6000`,
`/usr/bin/time -v bench 500 10 20000` OK; 39 censuses unchanged. Compile times: `Process`
76 s, `WireTrace` 44 s, `Fused` 16 s, `Project/Wire` 10 s, `WiredSafety` 3 s,
`WireTraceCheck` 3 s. Heartbeat sites unchanged (LE 600k, PaxosCore 300k).

**Queue after D93.** Acyclic knots as same-tick lets at `SchedSem`/`Project` (the
faithful change: the activation one pass, DFIR's tick counts, the 116k back without
spinning — the knot semantics `KnotKind.feed`, the Couple corner's knot wf conjuncts and
`dag_denote` are what it touches; a knot that is acyclic through the body is a shift-0
alias, and its one-step-delayed form is the same denotation under a shifted schedule); the
theorem that nothing is pending after `Dag.iterate` on a locally acyclic graph whose knots
feed no tick input (`Dag.iterate_quiet` under a `LocallyAcyclic` predicate; with it the
counter is provably never reached and the shells' "pending work" wake is provably idle);
the settling as a pass *list with flags* rather than flags recomputed from cells
(`Dag.settle` returning `(Bool × result)` would make `passFlag` a projection and shorten
`inner_succ`/`pass_outs`); incremental `use::state` for the keyed operators (the wall, now
paid twice per burst at the leader); and the carried-over list (transport as a declared edge requirement, ghost parameters for
`latest_monotone`, `Optional` carrier, `paxos_with_client_safe_wired`, higher-order
`hydro def`, heartbeat tax, liveness rung 2, `SchedSem` against production DFIR, upstream
Rust reports incl. D76's reversed wording wherever it was quoted).
