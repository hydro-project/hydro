# The Configuration Calculus: Hydro-Native Protocol Reasoning

*(**Status note**: historical — for protocol proofs this organization is
superseded by the decisions-as-inputs semantics of docs/10; the Paxos safety
proof lives entirely at that surface (`Programs/Paxos/Safety.lean`). Its
host module `Hydro/ChoreoConfig.lean` was deleted with the rest of the
choreographic layer (recover from jj history if needed). The two
methodological insights recorded here (configurations = consumed tick-batch
nestings; clusters as maps) carry over verbatim into docs/10.)*

## Why not event induction

The standard way to verify a protocol — TLA+, IronFleet, Verdi, and our own
first two Paxos attempts — is: define a global state (every node's internal
state + the network), define a step relation over events, guess an inductive
invariant, prove it preserved by every event. It works, and it teaches nothing
new: the proof lives at a level of abstraction *below* the program, and every
protocol pays the full cost again (in our old `AcceptorInv.lean`, roughly 40%
of every clause proof was "the event touched a different site, so everything
this clause reads is unchanged" plumbing).

The premise of this project is that Hydro programs carry more structure than
state machines: modules are pure functions with verified models, wiring is
pure prefix-monotone stream functions (network edges included — Gyatso), and
**all** nondeterminism is materialized at typed guard sites. The configuration
calculus is the proof theory that exploits this.

## Configurations: the consumption structure is the state

A **configuration** (`Conf`) of a choreography carries exactly two things:

- `cons i : List (In i)` — each clock's **consumed tick-batch sequence**: the
  outer list is tick iterations, each entry is one per-port batch tuple. This
  is the Flo §2.5 stream-of-streams presentation: a `Tick`-located collection
  is literally "tick index ↦ collection in that iteration". Crucially the
  *shape* of this nesting (where batch boundaries fall) **is** the
  materialization record of the program's `nondet!` guards — the adversary's
  entire power, as plain data.
- `ext` — the injected external input.

Nothing else. Clock states are *derived views* through the models
(`Conf.st i = finalState (cons i)`); per-edge histories likewise
(`Conf.hist i = emitCat (outputs (cons i))` — the **model equations**,
definitional); availability at any port is the choreography's reader applied
to the derived environment (`Conf.avail` — the **wiring equations**). The
runner's own `Faithful` invariant proves this is lossless: every trace field
is a function of `(batches, injected)`.

### The representation decision (nested primary, flat derived)

Two candidate representations for consumed prefixes were considered: flat
per-port streams + cut counters (what the runtime does), and tick-indexed
nested lists (Flo §2.5). The calculus uses **nested as primary** and derives
flat (`consFlat` = flatten; cuts = lengths):

1. models consume `List In` directly — `finalState`/`outputs` apply with no
   re-chunking;
2. Rust `Tick`-located types read as tick-index ↦ collection, so module
   contracts stated over the nested view align with the signatures they
   describe;
3. the flat view is recovered definitionally where wiring needs it, and
   `Conf.asTrace`/`asTrace_faithful` present any wiring-consistent
   configuration as a `Faithful` runner trace, so the entire existing
   trace-level vocabulary (accessors, avail lemmas, component-run
   identifications) transports with zero duplication. This is the
   collection-level two-lens correspondence: flat = runtime lens, nested =
   proof lens, mediated by `ofTrace`/`asTrace`.

Evidence: in the Paxos demonstrations, per-clock invariants consumed the
nested view (batch-append induction), while the cross-clock wiring steps
consumed the flat view — both definitional from one structure, neither
converted by hand.

## Grounded configurations: provenance induction

`Conf.Grounded` characterizes causally-reachable configurations inductively:

- `init` — the empty configuration;
- `inject` — external input arrives;
- `fire i b` — **one model application**: clock `i` consumes one more
  tick-batch `b`, legal iff its cumulative flat consumption stays a prefix of
  availability *derived from the configuration so far* (a clock reading its
  own edges sees its history before this fire). Causality with no clocks,
  no events, no scheduler.

`Grounded.rec` **is** the provenance-induction principle: proofs proceed by
"which model application produced this data", never "what happened at step k".
Three derived forms cover practice:

- **`Grounded.consInv`** — the per-clock invariant rule: a predicate over one
  clock's consumption holds everywhere given (init) and (own-fire
  preservation). Fires of other clocks can't touch `cons i`, so the
  "other site untouched" plumbing of world-invariant proofs is discharged
  here once, generically. The fire obligation receives the ambient
  configuration (grounded, with the wiring prefix condition), so cross-clock
  facts are available *at the interface level* inside a per-clock proof.
- **`Conf.mem_hist_elim`** — data provenance for produced elements: every
  element of an emitted history was produced by some tick's model application
  on the consumption strictly before it.
- **`Grounded.wiringOk`** — every grounded configuration is wiring-consistent,
  so cross-clock chains (consumed ⊑ avail = reader of neighbor histories =
  neighbor's model of *its* consumption …) are purely equational.

## Lowering: one event induction, ever

`Conf.grounded_ofTrace : ∀ sched, Grounded (ofTrace (simRun sched))` — every
run of the elaborated system induces a grounded configuration (the runner's
fire *is* a `Grounded.fire`; `Faithful` supplies the wiring side condition).
This is the only event induction in the development, paid once inside the
framework for all choreographies and all protocols. Its corollaries
(`holdsConf_of_grounded`, `holdsTraceAlways_of_grounded`) turn any
config-level safety proof into a cheat-proof `Holds`-family statement — the ∀
over the full schedule space, with the proof never mentioning a schedule.

## The demonstrations (Programs/Paxos/ConfigProofs.lean)

| Fact | Old proof (`AcceptorInv.lean` / `Projections.lean`) | Config proof |
|---|---|---|
| P1b ballot provenance (cross-clock) | CInv clause inside a 13-clause trace invariant, event induction | equational chain: `consInv` + `wiringOk` + definitional avail + `consInv` (ownership); no induction beyond the packaged rules |
| max-ballot lower bounds, write-before-ack | world-invariant clauses, per-event dispatch + cut plumbing | `AccFacts` via `consInv`; component lemmas (`acceptorStep_log_lb`, `acceptorStep_ack_logged`) verbatim |
| **promise order** (the Ok-p1b-covers-lower-Ok-accepts internalization) | ~380 lines: 4-way math + ~150 lines of neg-case/cut/world plumbing | the same 4-way math (~180 lines), `consInv` lift (3 lines), **zero plumbing** |
| run-level form | `acceptorInv_run`: fold over scripts | `promise_order_holds : HoldsTraceAlways …` via the lowering corollary |

The step relation of every one of these proofs is a *module model*
(`acceptorStep`, the proposer fragment step), and the only implementation
unfolded is the clock's own (interface-first pragmatism, ACCEPTANCE.md).

## Proof-engineering notes

- With symbolic port indices (`apP1a i₀` at fragment-composed port types),
  `rw` of membership lemmas can fail motive typechecking at reducible
  transparency. Reliable idiom: term-level `.mp`/`.mpr`/`▸` applications and
  **early-decode views** (filterMap at the port, as `ChoreoStep`'s accessors
  do), keeping list reasoning at clean types.
- State per-clock invariants over the clock's consumption list (`bs`), not
  over configurations: `consInv` wants `Q : List (In i) → Prop`, and the
  derived views (`accStOf`, `accP1bsOf`, …) make the clauses read like the
  old world clauses with the world gone.

## Relation to the two-lens architecture (docs/08)

The choreographic layer gives programs-as-modules and definitional
elaboration; the configuration calculus gives the **proof objects** for that
layer: configurations are to choreographies what traces are to the runner,
and the lowering theorem closes the triangle (choreography → configurations →
runner) so that module contracts, marker theorems, and sealed denotations all
compose at the configuration level and transport to executions for free.
