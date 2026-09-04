# SCHED_AUDIT — red-team fidelity audit of the step machine

**Subject**: `HydroV2/Sched.lean` (the `SchedSem` step machine) and the
statement-level premises of the machine-run theorems
(now `paxos_safe_sched'`, `cq_safe_sched'` — the premise-free corner
headlines; the audit below was written against their retired
predecessors `paxos_safe_sched`/`cq_safe_sched`, see the F1 status)
that quantify over its schedules.

**Question audited**: is the machine an accurate model of true
distributed semantics — i.e., can every behavior a deployed Hydro
program can exhibit be represented by some
`(pacing, schedule, decision)` tuple, and do the theorems' premises
quantify over all of those tuples? A gap in either half is a hole in
`paxos_safe_sched`, because `SchedSem`'s definition is part of the
trusted statement (see CORRESPONDENCE.md, "Auditing the trust
boundary").

**Why this audit exists**: the per-member ticking gap (FINDINGS D35)
was found incidentally, not systematically. Its moral — *ambient
parameters whose sharing scope is coarser than the unit of concurrency
are a smell* — is one axis below; this audit sweeps all of them, plus a
dedicated pass for **Lean-internal accidental holes**: definitions that
silently constrain the adversary more than their prose claims
(blocking branches, freeze branches, quantifier placement).

Line numbers are as of this commit and may drift; definition/theorem
names are the stable anchors.

---

## Severity scale

| Rank | Meaning | Consequence |
|---|---|---|
| **S1** | A real behavior (or schedule class) is outside what the theorem's quantifiers range over, and the exclusion is semantically visible | weakens/narrows the theorem; needs a fix or an explicit scope condition |
| **S2** | Unreachable in the model, but with a stated argument for why no proven property depends on it | document; optional hardening |
| **S3** | The model admits **more** than reality | safe direction for safety theorems; document only |
| **S4** | Checked faithful | recorded so the next auditor doesn't redo it |

---

## Ground truth: what the Rust runtime actually does

Read for this audit (citations used throughout):

- **Ticks are per-process local logical time.** `dfir_rs/src/scheduled/ticks.rs`
  (module doc, lines 1–6): "Each iteration of a process loop is called a
  tick … Each process produces totally ordered, sequentially increasing
  clock values … the 'local logical time' at the process." There is no
  cross-process tick synchronization anywhere in the runtime.
- **Ticks are event-driven; empty ticks happen; mid-tick arrivals wait.**
  `dfir_rs/src/scheduled/context.rs`: `run` (line 391) loops
  `run_available` then sleeps until a waker fires; `run_available`
  (line 353) "Always run at least one tick", then re-runs while
  `can_start_tick`; external arrivals during a tick set the flag and are
  processed in the *next* tick (`run_tick`, lines 327–336). Wall-clock
  timers wake the loop the same way as network input, so ticks with no
  data input are real.
- **A tick drains what is buffered at tick start** (handoff semantics;
  `run_tick` runs the whole tick closure once). There is no partial
  consumption of an already-arrived buffer within one tick — "partial
  batches" only ever arise from arrival timing.
- **Transports and failure policies.** `hydro_lang/src/networking/mod.rs`:
  `TCP.fail_stop()` gives `TotalOrder` per connection and "stops sending
  messages after a failed connection … models the recipient as having
  failed" (lines 99–113). `TCP.lossy()` may drop an arbitrary message
  *and continue delivering later ones* (lines 115–128, requires a
  `NonDet` guard); `lossy_delayed_forever` re-grades to `NoOrder`
  (lines 130–154); UDP is always `NoOrder` and connectionless
  (lines 156–194). Fail-stop connections never retransmit, so **TCP
  never duplicates**; duplication in Hydro is born at application-level
  re-sampling (`sample_every` and kin), exactly where the model puts it.
- **The mirrored programs use fail-stop everywhere.**
  `hydro_test/src/cluster/paxos.rs` lines 316, 442, 521, 748, 889: every
  `broadcast`/`demux` edge is `TCP.fail_stop().bincode()`. Likewise
  `hydro_std` (bench_client, compartmentalize).
- **Batch boundaries are the documented nondeterminism.**
  `hydro_lang/src/live_collections/stream/mod.rs::batch` (line 2196):
  "batches are guaranteed to be contiguous across ticks and preserve the
  order of the input … batch boundaries are non-deterministic."
- **Cluster membership is fixed per deployment** (`Cluster` locations
  are sized at deploy; `mem ℓ` constant matches).

"True distributed semantics" for a deployed Hydro program is therefore:
per-process event-driven local ticks; per-connection FIFO prefix
delivery with fail-stop; independent per-recipient fan-out; timers on
wall-clock; all inter-process influence mediated by messages.

---

## Axis-by-axis verdicts

| Axis | Model artifact | Runtime ground truth | Verdict |
|---|---|---|---|
| Tick skeleton granularity | `pacing : (ℓ) → Fin (mem ℓ) → Nat → Bool` (per member, post-D35) | per-process local clocks (ticks.rs) | **S4** |
| Delivery cursor granularity | `TransportDec p c := Fin p → Fin c → Nat → Nat`, one field per network edge (`p1aCh`, `p1bCh`, `p2aCh`, `p2bCh`, `ial` in `PaxosCoreDec`) | one TCP connection per (edge, sender, receiver) | **S4** |
| Emission linearization granularity | `EmitDec n β := Fin n → List (List β)`, one field per emit site | per-member hashmap iteration order per tick | **S4** |
| Timing decisions granularity | `SampleTimes/TimerVerdicts/TimingPulses n := Fin n → …` (per member) | per-process timers | **S4** |
| Per-pair FIFO + prefix delivery | `StepHist.deliver` + `cumMax` (Sched.lean:136–166) | TCP fail_stop TotalOrder per connection | **S4** |
| Unbounded delay / silence / crash | cursor stalls; `pacing` goes all-false; outgoing cursors freeze | fail-stop, arbitrary latency | **S4** |
| Partial / divergent fan-out | independent per-recipient cursors, item-granular | independent connections; crash mid-tick flushes some connections and not others | **S4** |
| Consume-all-at-tick | `batchesFrom` (Sched.lean:217–221) | handoff drain per tick | **S4** |
| Empty/idle ticks; tick-counting ops | arbitrary pacing; `timeout_snapshot`/`source_interval_batch` take per-tick entries | event-driven ticks incl. timer-only wakes | **S4** (and S3: model also allows *withheld* verdicts, see F5) |
| `+1` floors (network, knot) | `deliver` reads sender's `view (t-1)`; `shift` at knots | handoffs and network are next-tick at soonest; no same-instant hop | **S4** |
| Duplication sites | transport preserves grade; ALO born at sampling | TCP never retransmits; duplication at re-sampling | **S4** |
| Static membership | `mem : L → Nat` constant | fixed at deploy | **S4** |
| Fan-in decision-freedom | `merge2`/`mergeN` take no decision; interleaving from delivery timing; canonical within-step order | in-tick operator scheduling is deterministic; cross-sender order is arrival order | **S2** (F4) |
| Re-monotonization | `famFreeze` (freeze branch never fires for op-built flows — *unproven*) | n/a (Lean-internal) | **S2** (F3) |
| Transport scope | prefix cursors only | `TCP.lossy` (drop-then-continue), UDP exist in Hydro | **S1-scope** (F6) |
| `hsat` premise dischargedness | `SquareNonVacuity` toy self-loop only | n/a (Lean-internal) | **S1** (F1) |
| Input-coupling premise quantifiers | `hcp : ∀ T i, view T <+: cpV i`, `cpV` fixed first | clients send unboundedly | **S1** (F2) |

---

## Findings

### F1 (S1) — `paxos_safe_sched`'s `hsat` premise is discharged nowhere for the real program

**What the theorem says.** `Paxos/SquareSafety.lean::paxos_safe_sched`
concludes slot-agreement for machine commits **given**
`hsat : (…square paxos_core…).val.2.dlt T d` for a caller-supplied
decision environment `d`. So the theorem is of the form
"∀ schedule, ∀ T, ∀ d, δ(schedule, T, d) → safety(schedule, T)"; it has
content at a given schedule and horizon **only if some `d` satisfies δ
there**.

**What δ contains.** For knot-free ops, δ is a conjunction of extension
atoms `derived(T, sr) <+: lens d` (e.g. `batch`'s
`batchDerive (pacing ℓ) T s.sr i <+: cutd d i`, Square.lean, `batch`
case) — each independently satisfiable by taking the lens value to *be*
the derived value, and jointly satisfiable because each atom constrains
its own record field. This is how `cq_safe_sched`
(SquareTheory.lean:48ff) discharges its δ inline:
`⟨batchDerive (pacing ()) T h, …, ⟨trivial, fun _j => List.prefix_refl _⟩⟩`
— possible because `collect_quorum` is **knot-free**.

For a `fix_stream`, however, δ additionally accumulates (Square.lean,
`fix_stream` case, the `dlt` field):

1. the stage-δ telescope `∀ k ≤ T, (stages k).dlt k d`;
2. the fuel floor `T + 1 ≤ df d`;
3. the **reader bridges**
   `∀ m ≤ df d, (stages m).rl d = iterate (vbody d) ⊥ m ∧ (stages m).rr d = iterate (vbody d) ⊥ (m+1)`;
4. the machine bridge
   `∀ t ≤ T, ∀ i, (diagonal stage view at t) = ((stages t).sr i).view t`.

Components 3–4 are *program-shape facts about the specific knot body*,
not extension atoms.

**What is actually proven.** `SquareNonVacuity.lean` proves generic
telescope/bridge lemmas (`sq_dlt_iterate`, `sq_r_iterate`,
`sq_sr_iterate_view`) and combines them in `sq_fix_dlt_nonvacuous` —
**for a toy instance only**: `Unit` location, one member, the identity
body, fuel `T+1`. Nothing instantiates the bridge lemmas' hypotheses
for the real `paxos_core` body (which contains five knots:
`fuelFail`/`fuelIAL`/`fuelLead`/`fuelSeqMax`/`fuelALog`).
`Paxos/TransferCheck.lean` is a *CorrSem* coverage witness at one
benign schedule (generous cursors `c t = t`, quiet timers) — it does
not touch the square δ. A repo-wide grep confirms no other artifact
proves `∃ d, dlt T d` for the Paxos square, at any schedule, any
horizon.

**Consequence.** If any reader-bridge component were false for the real
body (e.g. a stage/vbody mismatch introduced by an op whose `rl/rr`
does not commute with staging the way the bridge demands), δ would be
**unsatisfiable for every schedule**, and `paxos_safe_sched` would be
vacuously true — while still compiling, still zero-sorry, and still
passing every current gate. Nothing machine-checked excludes this
today. This is precisely the failure mode this audit was commissioned
to find, and it is also a violation in spirit of the project's D15 rule
(every headline theorem carries an executable/proved inhabitation
witness): the inhabitation of `hsat` for the real program is currently
*analogy* (the toy) rather than *evidence*.

**Discriminating scenario.** None needed at the semantics level — the
gap is in the premise's dischargedness, not in `Sched.lean`. The test
that would settle it: exhibit, for one nontrivial schedule (e.g. the §6
race-style cursors) and one horizon, a concrete
`d : PaxD …` with a proof of
`(…square paxos_core…).val.2.dlt T d`.

**Proposed fix shape** (per-operator, per the D33 genericity mandate):
generic lemmas
"for op-built bodies, `(F C).rl/rr` commute with the stage/vbody
embedding" and "op-built bodies have shift-shaped `sr`" — each proved
once per operator, assembled per-program by `sq_transfer`-style
elaboration — culminating in
`paxos_dlt_nonvacuous : ∀ pacing sched T, ∃ d, (…).dlt T d`, and
`paxos_safe_sched'` with the `hsat` premise *gone* (or kept, plus the
∃-witness corollary). Alternatively (cheaper, weaker): a single
concrete witness instantiation as a compiled check.

**Status: Stage A of the fix is DONE** (this tree): the knot
combinators δ names (`SqStream.stages`/`vbody`/`sbody`, …) are now
top-level definitions (`Square.lean`), the `SquareProj.lean` projection
kit gained the `_rl` mirror family and raw-binder embed collapses, and
`HydroV2/SquareKnot.lean` proves the **reader bridges and the machine
bridge hold for ALL decision environments** once the body satisfies
three `sq_transfer`-provable naturality identities
(`sq_fix_stream_dlt_intro`/`sq_fix_tick_dlt_intro`: δ of a knot ≡
telescope + fuel). Mechanism validated end-to-end on an op-built knot:
`sq_fix_dlt_nonvacuous_ops` (SquareNonVacuity.lean) discharges the
entire δ of a `union`-with-input knot at every schedule and horizon.
**Status: Stage B core is DONE, and it VINDICATED this finding**
(FINDINGS.md D37): the first attempt to inhabit a *nested* knot's δ
proved the old left reader bridge **unsatisfiable** (it equated the
two Kleene stages of any captured outer wire) — i.e. `hsat` was False
for every `d` at every `T ≥ 1` for the paxos shape, and
`paxos_safe_sched` was vacuously true at every interesting horizon.
This was precisely the failure mode this finding warned about. The δ
was repaired (user-ratified): the left bridge is replaced by the
Values-only Kleene chain condition the fix's `cpl` actually consumes;
the statement of `paxos_safe_sched` is unchanged. The causality
family (`SchedCausal.lean`, all ~40 ops), δ-projections
(`SquareDlt.lean`), derive chains and telescope introduction
(`SquareKnot.lean`) are built and validated end-to-end on single and
nested knots with decision atoms (`sq_fix_dlt_exists_batch`,
`sq_fix_dlt_exists_nested`: `∃ d, dlt T d` at every schedule and
horizon). **Remaining**: `paxos_hsat` and the premise-free
`paxos_safe_sched'` (Stage C), now routed through the
instance-generic-`fix` architecture (FINDINGS.md **D38**): the five
paxos knots take named `∀ H'`-generic bodies with curried captures
(`HydroSem.fix`/`fixTick`, landed and gate-green), so the remaining
per-knot obligations (causality/chain) are discharged by colocated
clauses instantiating the bodies at gluing instances
(`CausalRel`-to-be + `MonoRel`), with per-knot `causal_fix_*` gluing
— the `MonoRel` consumption pattern — feeding the (fast, validated)
atom-pinning assembly of `SquareHsat.lean`. F2
was dropped by user ruling.

**Status: Stage C re-routed (user-ratified) through the coupling
corner** (FINDINGS.md **D39**): instead of *checking* δ-satisfiability
per program, a fourth interpretation (`Couple.lean`,
`CoupleSem L mem pacing Tc Td`) **constructs** the `Values` decisions
from the machine leg op by op, and carries machine run + `Values` run
+ their coupling in the carrier — so `∃ d` and the coupling both fall
out of running the program once; `hsat` ceases to exist as a premise.
Knots are handled by the generic `co_fix_cpl`/`co_tick_fix_cpl`
(no telescope, no stages; the graded-coupling obligation `hcplj` is
discharged by re-instantiating the D38 `∀ H'`-generic knot bodies at
a horizon-lowered corner). Validated end-to-end at knot scale
(`CoupleCheck.lean`); **both whole-program naming identities are now
proved** — `paxos_co_sr` (machine leg) and `paxos_co_rr` (reader leg
at the corner's own derived decisions, `paxosVDecR`; ∃-corollary
`paxos_co_rr_ex`) — structurally, per knot (FINDINGS.md **D40**: defeq
through nested knots is exponential in the nesting depth, so
whole-program `rfl` was abandoned for per-knot naming lemmas,
packaged as the `KnotTactics.lean` macros; both identities land in
~3 s). The remaining assembly toward the *strong* fix shape (premise
gone, not the weak single-witness one) is `paxos_co_wf` (per-knot
`hcaus`/`hchain`/`hcplj`, the `cc_wf` pattern at paxos scale) and the
premise-free `paxos_safe_sched'` consuming `.cpl` + the two namings.

**Status: CLOSED — the strong fix shape landed (FINDINGS.md D41).**
`paxos_safe_sched'` (`Paxos/CoupleWf.lean`) is the premise-free
headline: the `hsat`/`d` arguments are **gone**, not discharged — the
coupling corner constructs the decisions and the coupling, and the
five paxos knots' residual `wf` obligations (causality, Kleene ascent,
graded coupling) are proven per knot (`leFails_co_wf` … `pcSeqF_fix_co_wf`,
assembled in `paxos_co_wf`, coupling `paxos_co_cpl`). The knot-free
analogue `cq_safe_sched'` (`Std/Quorum.lean`) replaces
`cq_safe_sched`'s inline δ witness the same way. Axiom audit for all
of them: `[propext, Classical.choice, Quot.sound]`
(`AxCheck.lean`). *Update (D42): the retirement was ratified and
executed — the Square δ-stack and the original `paxos_safe_sched`
(δ form) are deleted; the file/line citations in the F1 narrative
above describe the tree as it stood when the finding was made (the
falsified δ design survives in git history and FINDINGS D32–D41).
`paxos_safe_sched'`/`cq_safe_sched'` are the only machine-safety
headlines.*

### F2 (S1) — the input-coupling premises exclude unbounded inputs

**What the theorem says.** `paxos_safe_sched` takes
`cpV : (Values L mem).Stream prop P …` — a **finite** pool per member
(`Values.lean:153`: `Stream ℓ α _ ord ret := Fin (mem ℓ) → PoolCarrier
α ord ret`, and `PoolCarrier` at `(TO, EO)` is `List`) — and
`hcp : ∀ T i, (cpS i).view T <+: cpV i` with `cpV` bound **before** the
`∀ T`. A machine input whose content grows without bound (a client that
never stops sending) admits *no* `cpV` satisfying `hcp`, so such runs
are outside the theorem's quantifiers entirely — even though each of
their finite prefixes is perfectly well-behaved.

**Why this is probably harmless — informally.** The conclusion at
horizon `T` should only depend on `cpS`'s views up to `T` (the machine
is causal: `deliver` reads `view (t-1)`, ticks read current views), so
one can truncate the input at `T`, apply the theorem to the truncated
run with `cpV :=` the content at `T`, and transfer the conclusion back.
But **machine causality is not a theorem in the tree**, so this
reduction is currently hand-waving — the same status the "WLOG" had
before D35's reproducer demanded per-member pacing.

**Proposed fix shape.** Either (a) restate the premise per-horizon —
the conclusion mentions a specific `T`, so
`hcp : ∀ i, Multiset.ofList ((cpS i).view T) ≤ cpV i` (or `<+:` at the
same `T`) suffices for the pipeline, since the coupling only ever
consumes `hcp` at horizons `≤ T`; or (b) prove the machine-causality
lemma (`paxos_core @ SchedSem` at horizon `T` is invariant under
input-history changes above `T`) once, generically per-op. Option (a)
is a statement improvement with no new proof machinery and makes the
theorem apply verbatim to unbounded clients.

*Status: OPEN (deliberately).* The user ruled F2 out of the F1 work's
scope; the successor headline `paxos_safe_sched'` carries the same
`hcp`/`hck` premise shape (bounded-input coupling), so this finding
applies to it verbatim. The corner's two-horizon design
(couple-at-`Tc`, derive-at-`Td`) is the natural hook for fix (a).

### F3 (S2) — `famFreeze` transparency at knots is assumed, not proven

`fix_stream`/`fix_tick` outputs (and every `allTicks`/`sample_every`
downstream of them) pass raw per-member diagonal views through
`famFreeze` (Sched.lean:231–272), which is a no-op **iff** the raw
family views are per-member prefix-monotone. For op-built bodies this
is true (each machine op preserves prefix-monotonicity of inputs; the
diagonal of a monotone body is monotone), but it is **proven nowhere**.
If the freeze branch ever fired for a reachable program+schedule, the
machine wire would silently stop growing — sound for the ⊆-direction of
safety (a frozen run is a stalled run), but schedule coverage would
silently shrink, and the family-atomic freeze would couple members
(one member's glitch freezes its siblings — a D35-smell in the
otherwise-per-member story, dormant only as long as the branch is
dead).

Present evidence is one positive probe: TransferChecks §6's race knot
produces full content through the machine fix
(`#guard (raceKnot 0).view 7 = [1, 101, 2, 102]`). That is one body.

**Proposed fix shape**: per-op lemmas "machine ops map prefix-monotone
step families to prefix-monotone step families" + "the Kleene diagonal
of a monotone `sbody` is per-member prefix-monotone" ⟹
`famFreeze_transparent_for_op_built` — after which `famFreeze` can stay
(as totality armor) with a theorem that it is dead code on real flows.

### F4 (S2) — canonical within-step fan-in order (sharpened statement)

`mergeN` appends senders' same-step increments in `finRange` order;
`assume_ordering` is the identity on the machine (Sched.lean:330). Thus
the machine's realized total order after `assume_ordering` is always
"arrival order with the canonical within-step tiebreak". Real
executions can realize other in-tick production interleavings (dfir's
in-tick operator scheduling is deterministic, but its order is not the
`finRange` order). These orders are: (i) invisible to every
`NoOrder`-graded consumer (multiset quotient); (ii) visible only
through `assume_ordering`, which is a `nondet!` site whose Values
semantics quantifies over **all** legal selections — so every safety
fact transported from Values holds for the unrealized orders too. The
residual gap is only that the *machine-level* theorem's schedule space
does not contain them; the elasticity argument (split arrivals across
steps) recovers all **cross-wire** interleavings but not reorderings
**within one sender's same-step increment** — those are pinned by
per-pair FIFO anyway, so the only truly unreachable orders are
same-step cross-sender permutations *finer than tick granularity that
no consumer can distinguish from a split-step schedule*. CORRESPONDENCE
already carries the WLOG note; this audit adds: the WLOG is load-bearing
**only at `assume_ordering`**, and every `assume_ordering` in the
mirrored programs is on wires whose downstream proof obligations were
discharged at Values over all selections.

### F5 (S3) — timing decisions are freer than reality (safe direction)

`timeout_snapshot` ignores its input stream entirely
(`Sched.lean:371–372`: verdicts are `(d i).take #ticks`), so verdict
traces uncorrelated with delivery — including "timer claims expiry in
the same tick a heartbeat arrived" — are schedulable. Real verdicts are
determined by wall-clock arrival times; since wall-time is free, nearly
all traces are realizable anyway, and the extras are adversary
generosity. Similarly, a decision may supply *fewer* verdicts than
ticks (the `take` truncates), modeling a timer that stops being
consulted — unreal, but strictly behavior-removing (safe). Same for
`source_interval_batch`. `sample_every`'s `SampleTimes` likewise.

### F6 (S1-scope) — the transport model is fail-stop only; say so in the theorem's advertising

The cursor model (`deliver` = monotone prefix) is exactly
`TCP.fail_stop`: per-connection FIFO, prefix delivery, silence-forever
expressible. It does **not** model:

- `TCP.lossy` (networking/mod.rs:115–128): drop an arbitrary message,
  *continue delivering later ones* — a non-prefix subsequence; no
  cursor produces it;
- `lossy_delayed_forever` / UDP: per-message arbitrary reordering
  within a pair.

This is fine — the Lean `Sem` signature only *has* fail-stop-shaped
network ops, and every edge of the mirrored programs is
`TCP.fail_stop` (paxos.rs:316/442/521/748/889) — but any prose of the
form "safety under any schedule of the distributed runtime" must carry
the scope condition **"for programs whose channels are all
`TCP.fail_stop`"**. If a future mirrored program uses `lossy` or UDP
edges, the network op and its coupling must be extended first (drop-set
decisions / per-message delay maps instead of cursors).

---

## What was checked and found faithful (S4 ledger)

Each item states the property, the model artifact, and the ground truth
citation. TransferChecks §n = the machine-checked existence witness.

1. **Per-member tick independence** — `pacing` per member; §9. Runtime:
   ticks.rs per-process clocks. (D35, re-verified.)
2. **Decision-granularity sweep** (the D35 class, all families):
   `TransportDec` per (receiver, sender) *and* per network edge
   (distinct record fields `p1aCh/p1bCh/p2aCh/p2bCh/ial`, matching one
   TCP connection per edge-pair — two edges between the same pair have
   independent cursors, matching independent connections with no
   cross-channel ordering); `EmitDec` per member per emit site
   (`cqwrEmit/rcEmit/cqEmit/jrEmit`); `SampleTimes`/`TimerVerdicts`/
   `TimingPulses` per member (Decisions.lean:51/56/60). No remaining
   ambient parameter is coarser than its unit of concurrency.
3. **Per-pair FIFO prefix delivery = fail_stop TCP** — `deliver`+`cumMax`;
   §3. Runtime: networking/mod.rs:99–113 (TotalOrder, stop-forever).
   Cursor freedoms verified expressible: unbounded delay (§2), silence
   forever, stall-at-k-forever (crash), delivery of partial per-tick
   emissions (cursors are item-granular), per-recipient divergence of a
   broadcast (member A gets all, B gets none, forever).
4. **Crash realism** — fail-stop = outgoing cursors freeze + pacing
   all-false from the crash step; a crash *mid-tick* that flushed some
   connections and not others = cursors split at different points of
   the same tick's emission. All expressible; no "atomic multicast"
   accidentally baked in.
5. **Consume-all-at-tick** — `batchesFrom` drops `consumed`, takes all
   the rest (Sched.lean:217–221). Runtime: handoff drain per
   `run_tick`; items arriving mid-tick go to the next tick
   (context.rs:327–336) = arrival-time refinement, covered by cursor
   freedom. "Partial batches" exist only via arrival timing, matching
   stream/mod.rs:2185–2195 ("contiguous across ticks, preserve input
   order").
6. **Idle/empty ticks and timer-only ticks** — arbitrary `pacing`;
   run_available's at-least-one-tick and waker-driven wakes make empty
   real ticks; §4 shows stutter ticks are observable where they should
   be (tick-counting ops).
7. **`+1` floors exclude no real behavior** — dfir handoffs and network
   sends are next-tick at soonest (no same-instant hop even for
   self-sends, which traverse the same egress/ingress machinery); the
   model floors at one *step* (finer than a tick), strictly more
   permissive.
8. **Duplication only at sampling** — fail_stop never retransmits;
   `weaken_retries`/transport are grade-preserving identities; ALO born
   at `sample_every` (§5 destuttering witness). Matches the runtime's
   duplication story exactly.
9. **Static membership** — `mem` constant; clusters fixed at deploy.
10. **`emitLin` blocking is legality, not coverage** — every real
    linearization of each tick's multiset is expressible (choose the
    matching lists); mismatched claims truncate the wire from that tick
    on, which only removes illegal decision claims, never real runs.
11. **`Trace.zip` truncation is benign post-D35** — every zip site in
    `SchedSem` pairs carriers of the *same member on the same skeleton*
    (batch legs and singleton legs both derive their tick domain from
    `tickSteps (pacing ℓ i)`), so no cross-skeleton length skew exists
    to be silently dropped.
12. **Oblivious-schedule obliviousness is by type and covers adaptive
    adversaries for safety** — cursors/pacing/verdicts are functions of
    the step only; the machine is deterministic given the tuple, so any
    adaptive adversary's realized run *is* some tuple
    (CORRESPONDENCE.md, batchDerive section, re-verified).
13. **Fan-in takes no decision anywhere** — `values`/`union` merge by
    increments; §1's cross-sender interleave witness shows the schedule
    controls the interleaving; the canonical within-step order is F4's
    S2 (the one nuance, documented).

---

## Relation to prior art, and audit checklist deltas

CORRESPONDENCE.md's "Auditing the trust boundary" section documents the
*intended* semantics of each definition with what-to-check notes; this
audit executed those checks against the Rust runtime and then swept for
the holes the checklist could not see (premise dischargedness, freeze
branches, quantifier order). Recommended follow-ups, in order of the
theorem-strength they buy:

1. **F1**: generic knot-δ discharge lemmas + `∃ d, hsat` for the real
   Paxos square (removes the one premise that could hide vacuity).
2. **F2**: per-horizon input coupling (or the causality lemma) — makes
   the headline apply verbatim to unbounded clients.
3. **F3**: famFreeze-transparency theorem for op-built bodies (turns
   "the freeze branch is dead code" from belief into theorem).
4. **F6**: one sentence of scope ("all channels fail-stop") wherever
   the theorem is advertised; extend the transport model only if/when a
   mirrored program needs lossy/UDP edges.

None of these change `Sched.lean`'s operational definitions: the
machine's *op semantics* survived the audit; the findings are about
what the theorems' premises quantify over and about unproven
no-op/coverage beliefs at the model's totality armor.
