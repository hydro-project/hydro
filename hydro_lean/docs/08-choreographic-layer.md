# The Choreographic Layer

> Status: **historical** — the choreographic layer (`Hydro/Choreo*.lean`,
> `Programs/Paxos/Choreo*.lean`) was deleted in the pivot to verified faces
> and the decisions-as-inputs semantics (docs/10); recover the code from jj
> history if needed. Names mentioned below (e.g. `acceptorP2`,
> `collectQuorumTick`, `paxosCore.toSys`) refer to that deleted code, not to
> the current tree. This document is kept as the record of the design and of
> why it was superseded.

## 1. What a choreography is

The Rust Hydro architecture (dissertation Ch. 4) is a *staged builder*: surface
code runs once to build a global dataflow IR, which is then projected to
per-location runtimes. The Lean choreographic layer mirrors this exactly, with
the `TGraphSys` runner playing the role of the projected runtime:

- A **choreography** (`Choreo`) is a structured, *nominal* description of a
  dataflow: tick **clocks** (each = one future runner site: state, ports,
  output edges, a step body written in the surface vocabulary), pure
  **readers** wiring ports to edge histories (every network edge — broadcast,
  demux, weakening — is a reader; nothing about the network is special), and
  external I/O.
- A **module** is a choreography fragment *parameterized by its input streams
  and returning its output streams* — the Lean form of a Rust function like
  `leader_election(...) -> (Singleton<Ballot,...>, ...)`. Concretely a module
  is an environment-polymorphic function producing clocks and/or **clock
  fragments** (`Frag`, below); modules compose by instantiation, and
  composition may create cycles.

### Readers and the forward_ref trick

A **stream handle** is a `Reader`: a pure, prefix-monotone function from the
*final* history environment (all clocks' emission histories + external input)
to a list. Because readers are functions of the final environment:

- `forward_ref` needs no fixpoint machinery at the description level. A cycle
  (`sequencing_max_ballot`: sequence_payload's output feeding back into
  leader_election's input) is expressed by ordinary argument passing at
  composition time — the runtime fixpoint is broken by tick boundaries, as
  always (a fire at step k reads histories through step k−1).
- Modules are **final-environment-polymorphic**: a module does not know the
  composed system's clock type; its readers are stated over an abstract
  environment `E` together with accessors for its own edges. Composition
  instantiates `E` with the composed environment. This is what lets module
  boundaries cut through sites and edges without the module seeing the rest
  of the system.

### Clocks and the surface vocabulary

A clock's step body is ordinary Lean code in the existing surface vocabulary
(`Stream.*`/`MStream.*` ops, existing verified tick components like
`collectQuorumTick`), so a `sliced!` block ports 1:1:

- **Views as state**: a step body that reads *snapshots of other clocks'
  outputs* (the `.snapshot`/`latest` pattern) accumulates the consumed
  prefixes in its own state — the fragment's `St` carries the views
  (`LeaderElection.St.errView1/…` in Paxos). Port cuts remain the only
  nondeterminism (the `nondet!` guards).
- **Batch/trigger ports**: each Rust `nondet!` site = one declared port.

### Design findings (learned from the toys + Paxos)

- **Merge is not a wire.** A reader must be prefix-monotone in the
  environment, and the *append* of two growing streams is not — so a
  fan-in cannot be expressed as a single reader. A merge is instead
  *multi-port fan-in at the consuming clock*: one port per source, per-port
  cuts, the step body seeing per-source batches. This is exactly the Rust
  sim's merge hooks and the per-`(sender)` FIFO port structure the Paxos
  hand-wiring already used; the choreographic layer makes it a rule.
- **Observation is not a reader.** `Choreo.output` (the system boundary
  observation) is a plain function of the environment, not a `Reader`:
  cross-member `flatMap` over member-indexed histories is not
  prefix-monotone, and observations need no monotonicity — the runner
  snapshots them.
- **Module boundaries may cut through clocks.** In Rust, `leader_election`
  and `sequence_payload` both contribute logic to the *same* proposer tick
  (and both halves of the acceptor tick share `a_max_ballot`/`a_log`
  atomically). The layer supports this with **clock fragments**.

### Clock fragments (`Frag`)

A `Frag Ev PM EM Vin Vout` is a slice of a clock: state, ports with readers,
output edges, and a step `St → batches → Vin → St × Vout × emissions` that
additionally *consumes and produces tick-local values* (`Vin`/`Vout`) — the
within-tick singletons Rust modules exchange (`p_ballot`, `a_max_ballot`,
post-write `a_log`). Fragments over the same clock compose with `Frag.comp`
(state pairs, port/edge blocks concatenate, a `wire` function routes the left
fragment's `Vout` into the right's `Vin`); a closed fragment (`Vin = PUnit`)
becomes a `ClockBody` with `Frag.toClock`. Discipline: one uniform port
carrier and edge carrier per clock (variants by construction — `mapCarrier`
embeds a fragment into the clock's carrier before composing), so composition
is cast-free and the projection lemmas (`comp_st_fst/snd`, `comp_emit_left/
right`, `comp_feeds_left/right`) are `rfl` or one `simp`.

## 2. The two lenses and the triangle

```
        Choreo (modules, typed located streams)
        /                                  \
   denotes                              elaborates
      /                                      \
 component-model algebra   ⟵ correspondence ⟶  TGraphSys execution
 (HComponent models,                         (schedules, Holds,
  combinators, markers)                       #guard oracles)
```

- **Denotation is NOT a third semantics**: a module's denotation is its
  shallow model (`HComponent`-style: pure function of inputs × choices);
  choreographic composition denotes combinator composition (`seq`/`seqAsync`/
  `seqSnapshot`/cycle contracts). All of that machinery already exists.
- **Elaboration** `Choreo.elab : Choreo → TGraphSys` is generic and mostly
  structural: nominal clocks/edges become the runner's sites/edges through a
  once-written Fin-indexing bridge; readers become the runner's `input`
  functions; step bodies become site steps. There is deliberately *no
  semantic gap* to cross at elaboration — the gap is crossed by the
  correspondence theorems, which are transport statements:
  1. **Clock-standalone** (from `siteState_eq_run`/`site_spec_at`): in every
     run of the elaborated system, each clock's state/history equals a
     standalone run of its step body on its consumed batches — so every
     ∀-batching component theorem holds in-system.
  2. **Boundary-readers** (definitional by construction): a module's boundary
     streams in a run are its readers applied to the run's histories.
  3. **Module-contract transport** (the packaged once-and-for-all theorem):
     if a module's clocks carry component contracts and its inputs satisfy
     their contracts in a run, its boundary outputs satisfy the module's
     denotational contract — ∀ schedules. Composition of modules chains this
     without touching the runner.

## 3. Modules 1:1 with Rust (the grounding example)

`Programs/Paxos/ChoreoModules.lean` defines the two Rust modules over an
abstract environment, reusing the verified components and the previous
hand-wiring's carriers/decoders verbatim:

- `LeaderElection.proposerFrag ins : Frag Ev (PPortMsg …) (PEdgeMsg …) PUnit
  (LEView …)` — p_ballot_calc, the trigger gate, the P1a send, the p_p1b
  tick view; produces the tick-local `LEView` (= Rust `p_ballot`,
  `p_is_leader`, `just_became_leader`, `p_relevant_p1bs`). Its
  `ProposerIns.p2bErrs` input is the `sequencing_max_ballot` forward_ref.
- `LeaderElection.p1bQuorumClock` — a whole clock owned by the module, its
  body *is* `collectQuorumWRTick`.
- `LeaderElection.accMaxFrag` / `accP1bFrag` — the acceptor-tick halves the
  module owns (`a_max_ballot`, the staged P1b reply reading the post-write
  log).
- `SequencePayload.proposerFrag : Frag … (LEView …) PUnit` — recommit,
  `indexPayloadsTick`, P2a send, `joinResponsesTick` commits; *consumes* the
  `LEView` produced by leader_election in the same tick.
- `SequencePayload.p2bQuorumClock` (= `collectQuorumTick`) and `accP2Frag`
  (= `acceptorP2`, publishing the post-write log as its `Vout`).

`Programs/Paxos/ChoreoCore.lean` assembles `paxosCore`: the shared proposer
clock is `(LE.proposerFrag …).comp (SP.proposerFrag …)`, the shared acceptor
clock is `accMaxFrag.comp (accP2Frag.comp accP1bFrag …)` (the `a_log`
within-tick atomic cycle, by staging), and both forward_refs are ordinary
argument passing through `leProposerIns`/`spProposerIns`. Nondet sites stay
explicit: batching/trigger/snapshot cuts are ports.

**Parity gate** (`ChoreoTests.lean`): the elaborated `paxosCore.toSys`
replays every `TestsG.lean` scenario and the full `FalsificationG.lean` B1
falsification script (faithful variant: agreement violation reproduced;
guarded variant: violation gone) with identical `#guard` values — the
choreography is behaviorally identical to the retired hand-wiring
(`GraphSystem.lean`, kept as a legacy oracle).

## 4. Proof architecture over the layer

Per-module contract lemmas at the fragment/component interface, composed
along the choreography through the generic transport. Demonstrated in
`Programs/Paxos/ChoreoTransport.lean`:

- **Component reuse is transport-free**: `p2bQuorum_state_eq` — in every run
  under every schedule, the p2b quorum slice's state *is* a standalone
  `collectQuorumTick` run on its consumed batches, so every
  `CollectQuorumProof` theorem applies to the in-system slice verbatim.
- **Fragment contracts transport through assembly**: `leFrag_p1a_ownership`
  is proven against ONE fragment step (the only place any implementation is
  unfolded); `p1a_ownership` lifts it to all runs of `paxosCore.toSys` via
  the definitional `Frag.comp_emit_left` projection + the runner's
  `cycle_invariant`. No schedule induction, no other module touched.

Protocol proofs (Paxos agreement) consume only module contracts + the generic
transport (`clock_spec_at`, `cycle_invariant`, fragment projections); the
runner appears only through the once-proven correspondence.

## 5. Files

- `Hydro/Choreo.lean` — readers, clock bodies, shapes, choreographies.
- `Hydro/ChoreoFrag.lean` — clock fragments: `Frag`, `comp`, `mapCarrier`,
  `toClock`, projection lemmas.
- `Hydro/ChoreoElab.lean` — `Choreo.toSys : TGraphSys` (purely structural;
  `avail_eq_feed`/`out_eq_output` are `rfl`) and the transport theorems
  (`clockState_eq_run`, `clockHist_eq`, `clock_spec_at`, re-exported
  `cycle_invariant`/`Holds*`).
- `Hydro/ChoreoDemos.lean` — toys: a two-clock pipeline (abstract-env module
  contract transported to runs); a knot (genuine cycle + `cycle_invariant`).
- `Programs/Paxos/ChoreoModules.lean` / `ChoreoCore.lean` — the modules and
  `paxos_core`, 1:1 with paxos.rs.
- `Programs/Paxos/ChoreoTests.lean` — the parity gate (TestsG +
  FalsificationG oracles against the elaborated system).
- `Programs/Paxos/ChoreoTransport.lean` — the modular-proof payoff (above).


## Proof objects for this layer

See [docs/09-configuration-calculus.md](09-configuration-calculus.md): wiring-consistent configurations (consumed tick-batch nestings; states/histories as model-derived views), provenance induction (`Grounded`), the per-clock rule (`consInv`), and the generic lowering theorem that transports configuration-level safety proofs to ∀-schedule `Holds` statements — the calculus in which protocol proofs over choreographies are conducted.
