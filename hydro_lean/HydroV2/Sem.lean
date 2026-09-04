import HydroV2.Trace
import HydroV2.Decisions

/-!
# HydroV2 — the location-typed combinator signature (graded carriers)

One program text per Hydro function, SPMD over located live collections,
polymorphic over an interpretation of this signature. Naming mirrors
hydro_lang's surface API; **guarantees are graded by the carrier types**,
mirroring Rust's marker parameters:

| Lean carrier | Rust type |
|---|---|
| `Stream ℓ α ord ret` | `Stream<α, ℓ, Unbounded, (ord, ret)>` |
| `KeyedStream p c α ord ret` | `KeyedStream<MemberId<c>, α, p, (ord, ret)>` |
| `Singleton ℓ α σ b` | `Singleton<σ, ℓ, b>` (b ∈ {Unbounded, Monotonic}) |
| `TickSingleton ℓ σ b` | `Singleton<σ, Tick<ℓ>, Bounded>` |
| `TickStream ℓ α ord ret` | `Stream<α, Tick<ℓ>, Bounded, (ord, ret)>` |

**The anti-cheating contract** (see `Grades.lean`): whatever a marker
pair refuses to promise is unobservable by type — `NoOrder` content is a
`Multiset` (order gone), `AtLeastOnce` content is further quotiented by
retry multiplicity (`RetryPool`) or consecutive stutter (`StutterSeq`).
Consumers of quotient content exist only through lifts whose respect
proofs are the Rust properties API's obligations, collected in one
graded predicate `FoldOk`:

| grade | `FoldOk` obligation |
|---|---|
| `TotalOrder, ExactlyOnce` | none |
| `NoOrder, ExactlyOnce` | commutativity |
| `TotalOrder, AtLeastOnce` | consecutive idempotence |
| `NoOrder, AtLeastOnce` | commutativity ∧ idempotence |

**Where nondeterminism lives — instance-declared**: every `nondet!`
site takes a decision argument whose *type is a field of the instance*
(the `…Dec` families below), so each interpretation declares its own
nondeterminism vocabulary and sites erase to `Unit` exactly where their
freedom lives elsewhere:

- the **denotation** (`Values.lean`) pays content decisions — per-tick
  consumption increments at `batch`/`snapshot` (legality graded:
  count-legal at `ExactlyOnce`, membership at `AtLeastOnce`; illegal
  increments block, legality is realizability), order selections at
  `assume_ordering`, cycle fuels at `fix*` — and takes `Unit` at
  transport and emission order (it delivers whole pools; partial
  delivery is the transfer coupling's relaxation);
- the **step machine** (`Sched.lean`) pays operational decisions —
  per-pair delivery cursors at `broadcast`/`demux`, emission
  linearizations at `emitMultisetBatches`, concrete timing — and takes
  `Unit` at content sites (a machine consumes its buffers in arrival
  order; batch pacing is delivery + tick-skeleton freedom);
- the **relational packaging** (`Rel.lean`) takes `Unit` everywhere
  (decisions absorbed into set-valued carriers).

Deterministic plumbing (`values`/`union` fan-in) takes no decision in
any interpretation: interleaving *emerges* from delivery timing and
rides the `NoOrder` quotient. The `Values` decision shapes carry domain
names (`Decisions.lean`): `BatchCuts`, `OrderedBatchCuts`,
`SnapshotCuts`, `OrderSelection`, `BatchOrderSelection`, `SampleTimes`,
`TimerVerdicts`, `TimingPulses`, `UnfoldFuel` — a signature reads as
its dataflow contract, not its encoding.
-/

namespace HydroV2

/-- The graded fold obligation (Rust's properties API): exactly the
respect proof the content quotient demands (`Grades.FoldOkP`). -/
abbrev FoldOk {α σ : Type _} (ord : StrOrd) (ret : Retries)
    (g : σ → α → σ) : Prop := FoldOkP ord ret g

/-- Rust's singleton boundedness marker (`Monotonic` carries the value
order the `monotone =` obligation is about). -/
inductive SingBound (σ : Type _) where
  | unbounded
  | monotonic (vo : ValueOrder σ)

/-- The Hydro combinator signature over a location alphabet `L` with
cluster sizes `mem`. Carriers are location-indexed **whole collections**
(SPMD: a cluster-located value denotes every member's wire at once).
Closures take the member id first — `q!` capturing `CLUSTER_SELF_ID`. -/
class HydroSem (L : Type) (mem : L → Nat) where
  Stream : L → (α : Type) → [DecidableEq α] → StrOrd → Retries → Type
  KeyedStream : L → L → (α : Type) → [DecidableEq α] →
    StrOrd → Retries → Type
  Singleton : L → (α σ : Type) → [DecidableEq α] →
    StrOrd → Retries → SingBound σ → Type
  TickSingleton : L → (σ : Type) → SingBound σ → Type
  TickStream : L → (α : Type) → [DecidableEq α] → StrOrd → Retries → Type
  /-- Transport decision at a network edge, per `(receiver, sender)`
  pair (`p`×`c` members): delivery cursors for the step machine, `Unit`
  for the denotation. -/
  TransportDec : Nat → Nat → Type
  /-- `.assume_ordering` selection: an order realization for the
  denotation, `Unit` for the step machine (arrival order *is* the
  machine's order). -/
  OrderSelDec : Nat → Type → Type
  /-- `.snapshot` read decision (graded by the source order for the
  denotation; `Unit` for the step machine — reads happen at ticks). -/
  SnapDec : Nat → Type → StrOrd → Type
  /-- `.batch` consumption decision on unordered content. -/
  BatchDec : Nat → Type → Type
  /-- `.batch` consumption decision on ordered content. -/
  OrdBatchDec : Nat → Type
  /-- Per-batch `.assume_ordering` selection inside the tick. -/
  BatchOrdSelDec : Nat → Type → Type
  /-- `.sample_every` timing (concrete tick times in every
  interpretation — sampling is genuine timing freedom). -/
  SampleDec : Nat → Type
  /-- `.timeout` verdicts (pure timing). -/
  TimerDec : Nat → Type
  /-- `source_interval` pulses (pure timing). -/
  PulseDec : Nat → Type
  /-- Emission linearization at `emitMultisetBatches`: the one site
  where a program puts *computed unordered data* on a wire, so wire
  order is born — `Unit` for the denotation, per-tick linearizations
  for the step machine. -/
  EmitDec : Nat → Type → Type
  /-- Cycle-knot decision: unfold fuel for the denotation, `Unit` for
  the step machine (cycle unfolding *is* step progression). -/
  FixDec : Type
  /-- `.map(q!(…))` (grade-preserving; at unordered grades the closure
  acts inside the quotient). -/
  map : ∀ {ℓ : L} {α β : Type} [DecidableEq α] [DecidableEq β]
    {ord : StrOrd},
    Stream ℓ α ord .exactlyOnce → (Fin (mem ℓ) → α → β) →
    Stream ℓ β ord .exactlyOnce
  /-- `.filter_map(q!(…))`. -/
  filterMap : ∀ {ℓ : L} {α β : Type} [DecidableEq α] [DecidableEq β]
    {ord : StrOrd},
    Stream ℓ α ord .exactlyOnce → (Fin (mem ℓ) → α → Option β) →
    Stream ℓ β ord .exactlyOnce
  /-- `.broadcast(&cluster, TCP.fail_stop().bincode(), nondet!(…))`:
  one-to-all shipping, **keyed by sender** at each receiver — pure
  plumbing, no decision (the interleave nondeterminism rides `values`'s
  `NoOrder` type); per-key content keeps its grade. -/
  broadcast : ∀ {c p : L} {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries}, (d : TransportDec (mem p) (mem c)) →
    Stream c α ord ret → KeyedStream p c α ord ret
  /-- `.demux(&cluster, TCP.fail_stop().bincode())`: addressed shipping —
  each receiver keeps, per sender, exactly the payloads addressed to it.
  Pure plumbing over exactly-once content (the address is data). -/
  demux : ∀ {c p : L} {α : Type} [DecidableEq α] {ord : StrOrd},
    (d : TransportDec (mem p) (mem c)) → Stream c (Nat × α) ord .exactlyOnce →
    (addr : Fin (mem p) → Nat) → KeyedStream p c α ord .exactlyOnce
  /-- `.values()`: forget the keys — pure; the interleaving
  nondeterminism rides the resulting `NoOrder` type. -/
  values : ∀ {p c : L} {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries},
    KeyedStream p c α ord ret → Stream p α .noOrder ret
  /-- Forget the exactly-once guarantee (a sound coarsening into the
  retry quotient — merging with genuinely at-least-once traffic forces
  it). -/
  weaken_retries : ∀ {ℓ : L} {α : Type} [DecidableEq α] {ord : StrOrd},
    Stream ℓ α ord .exactlyOnce → Stream ℓ α ord .atLeastOnce
  /-- Merge two unordered streams — deterministic plumbing (the merge
  interleaving emerges from delivery timing and is unobservable through
  the `NoOrder` quotient). -/
  union : ∀ {ℓ : L} {α : Type} [DecidableEq α] {ret : Retries},
    Stream ℓ α .noOrder ret → Stream ℓ α .noOrder ret →
    Stream ℓ α .noOrder ret
  /-- `.assume_ordering::<TotalOrder>(nondet!(…))`: realize unordered
  exactly-once content as a sequence — the explicit selection decision
  (an illegal selection blocks). -/
  assume_ordering : ∀ {ℓ : L} {α : Type} [DecidableEq α],
    Stream ℓ α .noOrder .exactlyOnce →
    (d : OrderSelDec (mem ℓ) α) → Stream ℓ α .totalOrder .exactlyOnce
  /-- `.fold(q!(init), q!(g))`: the closure carries the graded
  obligation (`FoldOk`) — **consumed by the content quotient**, so a
  fold that mishandles reordering or retries does not typecheck. -/
  fold : ∀ {ℓ : L} {α σ : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (g : σ → α → σ) (init : σ),
    FoldOk ord ret g → Stream ℓ α ord ret →
    Singleton ℓ α σ ord ret .unbounded
  /-- `.fold` with `Monotone = Proved` in addition: the inflationary
  obligation upgrades the output bound to `Monotonic`. -/
  fold_monotone : ∀ {ℓ : L} {α σ : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (vo : ValueOrder σ) (g : σ → α → σ) (init : σ),
    FoldOk ord ret g → (∀ s x, vo.le s (g s x)) →
    Stream ℓ α ord ret → Singleton ℓ α σ ord ret (.monotonic vo)
  /-- `.snapshot(&tick, nondet!(…))`: read a folded singleton at ticks;
  the decision vocabulary is graded (`CutDec`): prefix cut counts for
  ordered sources, arrival-increment multisets for unordered ones
  (count-legal at `ExactlyOnce`, membership-legal at `AtLeastOnce` —
  retry duplication is the increment's freedom). The bound transports. -/
  snapshot : ∀ {ℓ : L} {α σ : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} {b : SingBound σ},
    Singleton ℓ α σ ord ret b →
    (d : SnapDec (mem ℓ) α ord) → TickSingleton ℓ σ b
  /-- `.batch(&tick, nondet!(…))` on unordered exactly-once content:
  per-tick consumed increments, count-legal. **The grade travels through
  the tick boundary** — the batches stay unordered. -/
  batch : ∀ {ℓ : L} {α : Type} [DecidableEq α],
    Stream ℓ α .noOrder .exactlyOnce →
    (d : BatchDec (mem ℓ) α) →
    TickStream ℓ α .noOrder .exactlyOnce
  /-- `.batch(&tick, nondet!(…))` on **ordered** content: per-tick
  consumed slice sizes, count-legal (blocking); slices keep the order. -/
  batch_ordered : ∀ {ℓ : L} {α : Type} [DecidableEq α],
    Stream ℓ α .totalOrder .exactlyOnce →
    (d : OrdBatchDec (mem ℓ)) →
    TickStream ℓ α .totalOrder .exactlyOnce
  /-- `.assume_ordering` inside the tick: realize each unordered batch
  as a sequence — the only way to consume a `NoOrder` batch
  order-sensitively. -/
  assume_ordering_batch : ∀ {ℓ : L} {α : Type} [DecidableEq α],
    TickStream ℓ α .noOrder .exactlyOnce →
    (d : BatchOrdSelDec (mem ℓ) α) →
    TickStream ℓ α .totalOrder .exactlyOnce
  /-- Per-tick whole-batch closure with a same-tick singleton read
  (ordered batches only — unordered ones fold commutatively or pay
  `assume_ordering_batch`). -/
  mapBatchWith : ∀ {ℓ : L} {α σ β : Type} [DecidableEq α],
    TickStream ℓ α .totalOrder .exactlyOnce →
    TickSingleton ℓ σ .unbounded →
    (Fin (mem ℓ) → List α → σ → β) → TickSingleton ℓ β .unbounded
  /-- Per-tick whole-batch closure (ordered batches only). -/
  mapBatch : ∀ {ℓ : L} {α β : Type} [DecidableEq α],
    TickStream ℓ α .totalOrder .exactlyOnce →
    (Fin (mem ℓ) → List α → β) → TickSingleton ℓ β .unbounded
  /-- `cross_singleton` + element-wise `map` inside the tick on
  **unordered** batches: the closure acts on each element inside the
  multiset (no order observed), pairing it with same-tick singleton
  reads. -/
  mapBatchesWith : ∀ {ℓ : L} {α σ β : Type} [DecidableEq α]
    [DecidableEq β],
    TickStream ℓ α .noOrder .exactlyOnce →
    TickSingleton ℓ σ .unbounded →
    (Fin (mem ℓ) → α → σ → β) →
    TickStream ℓ β .noOrder .exactlyOnce
  /-- Stateful per-tick batch body with a same-tick singleton read — the
  `use::state` tick loop (ordered batches only). -/
  scan_batches_across_ticks : ∀ {ℓ : L} {α τ σ β : Type} [DecidableEq α],
    TickStream ℓ α .totalOrder .exactlyOnce →
    TickSingleton ℓ τ .unbounded →
    (Fin (mem ℓ) → σ → List α → τ → σ × β) → σ →
    TickSingleton ℓ β .unbounded
  /-- `use::state` fold across **unordered** tick batches: the step
  carries commutativity (a batch is a multiset) and the `monotonic =`
  proof — the acceptor max wire's type. -/
  fold_batches_across_ticks_monotone : ∀ {ℓ : L} {α σ : Type} [DecidableEq α]
    (vo : ValueOrder σ) (g : Fin (mem ℓ) → σ → α → σ) (init : σ),
    (∀ i s x y, g i (g i s x) y = g i (g i s y) x) →
    (∀ i s x, vo.le s (g i s x)) →
    TickStream ℓ α .noOrder .exactlyOnce →
    TickSingleton ℓ σ (.monotonic vo)
  /-- `.latest().sample_every(q!(dur), nondet!(…))`: read the live latest
  value at decision-chosen tick times (an absent latest emits nothing;
  reads of unrealized ticks **block**, so under the surrounding election
  `fix` the samples stabilize by prefix). **Sampling introduces
  `AtLeastOnce`**: an unchanged latest sampled twice is a consecutive
  stutter — exactly the `TotalOrder × AtLeastOnce` quotient. -/
  sample_every : ∀ {ℓ : L} {α : Type} [DecidableEq α],
    TickSingleton ℓ (Option α) .unbounded →
    (d : SampleDec (mem ℓ)) →
    Stream ℓ α .totalOrder .atLeastOnce
  /-- `.timeout(q!(dur), nondet!(…)).snapshot(&tick, …)`: per-tick expiry
  verdicts. Timeout firing is **pure timing** (arbitrarily delayed
  messages can expire any timer), so every verdict trace is realizable —
  the input only documents the dataflow, and the Rust doc comment's
  claim (timing only affects *which* leader wins) is this op exporting
  no contract. -/
  timeout_snapshot : ∀ {ℓ : L} {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries}, Stream ℓ α ord ret →
    (d : TimerDec (mem ℓ)) → TickSingleton ℓ Bool .unbounded
  /-- `source_interval_delayed(…).batch(&tick, nondet!(…)).first()
  .is_some()`: a pure timing source, per tick. -/
  source_interval_batch : ∀ {ℓ : L},
    (d : PulseDec (mem ℓ)) → TickSingleton ℓ Bool .unbounded
  /-- Stateful `use::state` loop over **unordered** tick batches with a
  same-tick singleton read: the step consumes each batch as its multiset
  — order-safe by construction. -/
  scan_batches_unordered_across_ticks : ∀ {ℓ : L} {α τ σ β : Type}
    [DecidableEq α],
    TickStream ℓ α .noOrder .exactlyOnce →
    TickSingleton ℓ τ .unbounded →
    (Fin (mem ℓ) → σ → Multiset α → τ → σ × β) → σ →
    TickSingleton ℓ β .unbounded
  /-- Companion-free stateful `use::state` loop over **unordered** tick
  batches (the `sliced!` shape of `hydro_std`'s quorum collector): the
  step consumes each batch as its multiset — order-safe by
  construction. -/
  scan_batches_unordered : ∀ {ℓ : L} {α σ β : Type} [DecidableEq α],
    TickStream ℓ α .noOrder .exactlyOnce →
    (Fin (mem ℓ) → σ → Multiset α → σ × β) → σ →
    TickSingleton ℓ β .unbounded
  /-- Stateful `use::state` loop over **two** tick-aligned unordered
  batch streams (the `join_responses` shape: an async response batch
  joined against same-tick metadata — the metadata leg is `atomic`, so
  its batches are the caller's tick domain, not a decision). -/
  scan_batches_unordered₂ : ∀ {ℓ : L} {α γ σ β : Type} [DecidableEq α]
    [DecidableEq γ],
    TickStream ℓ α .noOrder .exactlyOnce →
    TickStream ℓ γ .noOrder .exactlyOnce →
    (Fin (mem ℓ) → σ → Multiset α → Multiset γ → σ × β) → σ →
    TickSingleton ℓ β .unbounded
  /-- Stateful per-tick body over a singleton wire (`use::state`). -/
  scan_across_ticks : ∀ {ℓ : L} {α σ β : Type},
    TickSingleton ℓ α .unbounded → (Fin (mem ℓ) → σ → α → σ × β) → σ →
    TickSingleton ℓ β .unbounded
  /-- `.map(q!(…))` on a tick singleton. -/
  mapTick : ∀ {ℓ : L} {α β : Type},
    TickSingleton ℓ α .unbounded → (Fin (mem ℓ) → α → β) →
    TickSingleton ℓ β .unbounded
  /-- `.zip(…)` of two tick singletons — same-tick alignment (tick
  atomicity). -/
  zipTick : ∀ {ℓ : L} {α β : Type},
    TickSingleton ℓ α .unbounded → TickSingleton ℓ β .unbounded →
    TickSingleton ℓ (α × β) .unbounded
  /-- `use::state` fold across ticks with the `monotonic =` proof. -/
  fold_across_ticks_monotone : ∀ {ℓ : L} {α σ : Type}
    (vo : ValueOrder σ) (g : Fin (mem ℓ) → σ → α → σ) (init : σ),
    (∀ i s x, vo.le s (g i s x)) →
    TickSingleton ℓ α .unbounded → TickSingleton ℓ σ (.monotonic vo)
  /-- Order-preserving map on a `Monotonic` tick singleton. -/
  mapMonotone : ∀ {ℓ : L} {σ τ : Type} {vo : ValueOrder σ}
    (vo' : ValueOrder τ), TickSingleton ℓ σ (.monotonic vo) →
    (h : Fin (mem ℓ) → σ → τ) →
    (∀ i {a b}, vo.le a b → vo'.le (h i a) (h i b)) →
    TickSingleton ℓ τ (.monotonic vo')
  /-- Forget the `Monotonic` bound. -/
  forgetBound : ∀ {ℓ : L} {σ : Type} {vo : ValueOrder σ},
    TickSingleton ℓ σ (.monotonic vo) → TickSingleton ℓ σ .unbounded
  /-- Rust `.defer_tick()` seeded with an initial state: tick `t` reads
  tick `t-1` (tick 0 reads the seed) — the guardedness device. -/
  defer : ∀ {ℓ : L} {σ : Type}, σ → TickSingleton ℓ σ .unbounded →
    TickSingleton ℓ σ .unbounded
  /-- `cross_singleton` + element-wise `filter_map` inside the tick on
  **unordered** batches (paxos.rs:838–849's qualification filter). -/
  filterMapBatchesWith : ∀ {ℓ : L} {α σ β : Type} [DecidableEq α]
    [DecidableEq β],
    TickStream ℓ α .noOrder .exactlyOnce →
    TickSingleton ℓ σ .unbounded →
    (Fin (mem ℓ) → α → σ → Option β) →
    TickStream ℓ β .noOrder .exactlyOnce
  /-- Consume a whole **unordered** tick batch through a function of its
  multiset (with a same-tick singleton read): functions out of the
  quotient are order-safe by construction — the `NoOrder` analogue of
  `mapBatchWith`. -/
  mapBatchesUnordered : ∀ {ℓ : L} {α σ β : Type} [DecidableEq α],
    TickStream ℓ α .noOrder .exactlyOnce →
    TickSingleton ℓ σ .unbounded →
    (Fin (mem ℓ) → Multiset α → σ → β) →
    TickSingleton ℓ β .unbounded
  /-- Per-tick multiset emissions entering the tick boundary as an
  unordered bounded batch stream (the identity staging of a computed
  unordered batch). The one site where computed unordered data hits a
  wire, so wire order is *born* here — the decision is its emission
  linearization (`Unit` denotationally). -/
  emitMultisetBatches : ∀ {ℓ : L} {β : Type} [DecidableEq β],
    TickSingleton ℓ (Multiset β) .unbounded →
    (d : EmitDec (mem ℓ) β) →
    TickStream ℓ β .noOrder .exactlyOnce
  /-- Rust `.all_ticks()`: leave the tick — per-tick batches concatenate
  in tick order and the batch grade is **preserved** (`Stream<T, L,
  Unbounded, O, R>` from `Stream<T, Tick<L>, Bounded, O, R>`). -/
  allTicks : ∀ {ℓ : L} {β : Type} [DecidableEq β] {ord : StrOrd},
    TickStream ℓ β ord .exactlyOnce → Stream ℓ β ord .exactlyOnce
  /-- Per-tick emission lists entering the tick boundary as an ordered
  bounded batch stream (the identity staging of a computed batch). -/
  emitBatches : ∀ {ℓ : L} {β : Type} [DecidableEq β],
    TickSingleton ℓ (List β) .unbounded →
    TickStream ℓ β .totalOrder .exactlyOnce
  /-- Per-tick emissions whose order is a selection artifact, published
  **unordered** (Rust's `flatten_unordered` output type: the list is
  forgotten into the batch multiset — always a sound coarsening). -/
  emitBatchesUnordered : ∀ {ℓ : L} {β : Type} [DecidableEq β],
    TickSingleton ℓ (List β) .unbounded →
    TickStream ℓ β .noOrder .exactlyOnce
  /-- Rust `forward_ref`/`complete_cycle` on a stream wire: guarded
  Kleene iteration from the empty wire; the decision is the unfolding
  depth for the denotation (`Unit` for the step machine, whose cycle
  unfolding is step progression). -/
  fix_stream : ∀ {ℓ : L} {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries}, (d : FixDec) →
    (Stream ℓ α ord ret → Stream ℓ α ord ret) → Stream ℓ α ord ret
  /-- `forward_ref` on a tick-singleton wire (the `a_log` knot shape). -/
  fix_tick : ∀ {ℓ : L} {σ : Type},
    (d : FixDec) →
    (TickSingleton ℓ σ .unbounded → TickSingleton ℓ σ .unbounded) →
    TickSingleton ℓ σ .unbounded

/-! ## Cycles with instance-generic bodies

`forward_ref` closures capture local wires and decisions. Writing the
capture tuple *explicitly* and the loop body *generically over the
interpretation* costs the program nothing (the enclosing function is
already `H`-generic; `Γ` is any program-chosen family, so decision
records and wires curry through) — and it makes the body a named
object every proof can re-instantiate: monotonicity at `MonoRel`,
machine causality at the causal gluing instance, coupling at the
square. The raw signature fields cannot have this shape (a structure
field cannot quantify over the structure's own type), so these are the
program-facing spellings of `fix_stream`/`fix_tick`. -/

/-- Rust `forward_ref`/`complete_cycle` on a stream wire, with the
captured context `caps` curried through an instance-generic body. -/
def HydroSem.fix {L : Type} {mem : L → Nat} (H : HydroSem L mem)
    {Γ : HydroSem L mem → Type} {ℓ : L} {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries}
    (d : H.FixDec) (caps : Γ H)
    (body : ∀ (H' : HydroSem L mem), Γ H' →
      H'.Stream ℓ α ord ret → H'.Stream ℓ α ord ret) :
    H.Stream ℓ α ord ret :=
  H.fix_stream d (body H caps)

/-- `forward_ref` on a tick-singleton wire, curried-captures form. -/
def HydroSem.fixTick {L : Type} {mem : L → Nat} (H : HydroSem L mem)
    {Γ : HydroSem L mem → Type} {ℓ : L} {σ : Type}
    (d : H.FixDec) (caps : Γ H)
    (body : ∀ (H' : HydroSem L mem), Γ H' →
      H'.TickSingleton ℓ σ .unbounded → H'.TickSingleton ℓ σ .unbounded) :
    H.TickSingleton ℓ σ .unbounded :=
  H.fix_tick d (body H caps)

end HydroV2
