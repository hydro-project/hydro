import Hydro.Trace
import Hydro.Decisions

/-!
# Hydro — the location-typed combinator signature (graded carriers)

One program text per Hydro function, SPMD over located live collections,
polymorphic over an interpretation of this signature. Naming mirrors
hydro_lang's surface API; **guarantees are graded by the carrier types**,
mirroring Rust's marker parameters:

| Lean carrier | Rust type |
|---|---|
| `Stream ℓ α ord ret` | `Stream<α, ℓ, Unbounded, (ord, ret)>` |
| `KeyedStream p c α ord ret` | `KeyedStream<MemberId<c>, α, p, (ord, ret)>` |
| `Singleton ℓ α σ b` | `Singleton<σ, ℓ, b>` (b ∈ {Unbounded, Monotonic}) |
| `Ticked ℓ σ` | `Singleton<σ, Tick<ℓ>, Bounded>` (as a wire across ticks) |
| `Ticked ℓ (Option σ)` | `Optional<σ, Tick<ℓ>, Bounded>` (as a wire) |
| `Ticked ℓ (BoundedStream α ord ret)` (= `TickStream ℓ α ord ret`) | `Stream<α, Tick<ℓ>, Bounded, (ord, ret)>` (as a wire) |
| `BoundedSingleton σ` / `BoundedOptional σ` / `BoundedStream α ord ret` | the same three, **inside** a tick body (this tick's values) |

A located wire across ticks is `Ticked ℓ ⟨per-tick element⟩` — the
tick-located trace of per-tick values. Inside a tick body (the `tick`
construct, Rust `sliced!`) this tick's values are the three bounded
families, all interpretation-dependent: the denotation quotients a
bounded stream by its grade while the step machine keeps the runtime's
plain `List`; the correspondence corner carries every in-tick value as
a coupled pair of legs (D60).

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
  per-pair delivery cursors at `broadcast_closed`/`demux`, emission
  concrete timing — and takes
  `Unit` at content sites (a machine consumes its buffers in arrival
  order; batch pacing is delivery + tick-skeleton freedom);
- the **relational packaging** (`Rel.lean`) takes `Unit` everywhere
  (decisions absorbed into set-valued carriers).

Deterministic plumbing (`values`/`union` fan-in) takes no decision in
any interpretation: interleaving *emerges* from delivery timing and
rides the `NoOrder` quotient. The `Values` decision shapes carry domain
names (`Decisions.lean`): `BatchCuts`, `OrderedBatchCuts`,
`SnapshotCuts`, `OrderSelection`, `SampleTimes`,
`TimerVerdicts`, `TimingPulses`, `UnfoldFuel` — a signature reads as
its dataflow contract, not its encoding.

**The three kinds of decision** (the nondet / sched-det split; checked
per module by `#nondet_census`):

| kind | families | `Values` | `Sched` | who reasons about it |
|---|---|---|---|---|
| **nondet** | `SnapDec` `BatchDec` `OrdBatchDec` `OrderSelDec` `SampleDec` `TimerDec` `PulseDec` | real | `Unit` (content) / real (timing) | **proofs** — these mirror Rust `nondet!` sites; they live in the program's `…Dec` records and appear in `ensures` contracts |
| **sched-det** | `TransportDec` | **`Unit`** | cursors | nobody — adversary/runtime freedom, silently ∀-quantified like the delivery cursors and pacing; they live in the per-module `…Sched` bundles (one trailing `sched` binder), never in contracts |
| **fuel** | `FixDec` | Kleene depth | `Unit` | machinery — the fueled-`fix` artifact (the corner pins it to the horizon); kept in the `…Dec` records because ascent proofs consume it |

Rust-tally accounting: the `nondet!(/** TODO */)` at dynamic
`broadcast_closed` sites is `nondet_membership` — it guards the **cluster
membership snapshot** (async member joins), NOT delivery. Our model
assumes closed membership (type-level `Fin n`; Rust's
`broadcast_closed` counterpart, which takes no `nondet!` — the D20
finding), so those sites correspond to freedom we assume away, not to
`TransportDec`. `TransportDec` is per-pair delivery cursors — machine
freedom Rust never marks (`demux`/`send` take no `nondet!` at all).
So a module's Rust `nondet!` count = its nondets + its dynamic
broadcasts; unordered emissions carry no decision (a `tick` block
computes them on the machine's plain lists — D61) and cycles carry no
`nondet!` (the fuel is our fixpoint
encoding).
-/

namespace Hydro

/-- The graded fold obligation (Rust's properties API): exactly the
respect proof the content quotient demands (`Grades.FoldOkP`). -/
abbrev FoldOk {α σ : Type _} (ord : StrOrd) (ret : Retries)
    (g : σ → α → σ) : Prop := FoldOkP ord ret g

/-! ## Tick shapes (D60): what a tick body consumes and emits

A `tick` block (Rust `sliced!`) consumes this tick's slice of any
number of singleton wires and bounded-stream wires and emits any mix of
the two. Rather than one former per arity, the former is indexed by a
**shape**: the type-level zip of its inputs (resp. emissions); the
construct computes it from the wires' types and generates the unzip
inside the body. The shape's realizations are plain recursive
functions over the carrier families, so they can type a signature
field. -/
inductive TickShape : Type 1 where
  /-- A singleton wire's slice: this tick's value. -/
  | sing (τ : Type)
  /-- A bounded-stream wire's slice: this tick's batch. -/
  | stream (α : Type) (inst : DecidableEq α) (ord : StrOrd) (ret : Retries)
  /-- Zip. -/
  | pair (a b : TickShape)

/-- The wires of a shape, for an interpretation's ticked / tick-stream
carriers. -/
@[reducible] def TickedOf {L : Type} (Tk : L → Type → Type)
    (TS : L → (α : Type) → [DecidableEq α] → StrOrd → Retries → Type)
    (ℓ : L) : TickShape → Type
  | .sing τ => Tk ℓ τ
  | .stream α inst ord ret => @TS ℓ α inst ord ret
  | .pair a b => TickedOf Tk TS ℓ a × TickedOf Tk TS ℓ b

/-- This tick's values of a shape, for an interpretation's bounded
families (the body's input/emission tuple). -/
@[reducible] def BoundedOf (BSing : Type → Type)
    (BStr : (α : Type) → [DecidableEq α] → StrOrd → Retries → Type) :
    TickShape → Type
  | .sing τ => BSing τ
  | .stream α inst ord ret => @BStr α inst ord ret
  | .pair a b => BoundedOf BSing BStr a × BoundedOf BSing BStr b

/-- The seed of a shaped register: a value for a singleton register
(`use::state(|l| l.singleton(q!(v)))`), nothing for a stream register
(`use::state_null`: it starts empty), componentwise for a tuple. -/
@[reducible] def SeedOf : TickShape → Type
  | .sing τ => τ
  | .stream _ _ _ _ => Unit
  | .pair a b => SeedOf a × SeedOf b

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
  /-- **The tick-located trace** of a per-tick value: Rust's
  `…<_, Tick<ℓ>, Bounded>` collections *are* the per-tick element types
  (`BoundedSingleton σ = σ`, `BoundedOptional σ = Option σ`,
  `BoundedStream α ord ret`), and a located wire across ticks is
  `Ticked ℓ ⟨that element type⟩` — one value per tick per member. The
  denotation's `Ticked` is the realized tick sequence; the step
  machine's additionally records at which machine step each tick was
  realized (its carrier is step-indexed). No boundedness grade: a tick
  collection is always `Bounded` in Rust; cross-tick *ascent* is a
  guarantee and lives in `ensures` faces, not in the type. -/
  Ticked : L → Type → Type
  /-- **One tick's bounded stream content**, Rust
  `Stream<α, Tick<ℓ>, Bounded, ord, ret>` **inside** the tick:
  interpretation-dependent — the denotation quotients it by its grade
  (`List`/`Multiset`/`StutterSeq`/`RetryPool`, the anti-cheating
  contract), the step machine keeps the concrete `List α` the runtime
  actually holds at every grade. In-tick operators are typed over this
  family; the correspondence between the two representations is a proof
  obligation per operator, not a coercion. -/
  BoundedStream : (α : Type) → [DecidableEq α] → StrOrd → Retries → Type
  /-- Rust `Singleton<σ, Tick<ℓ>, Bounded>` **inside** a tick: this tick's
  value. A plain `σ` in the running interpretations; at the
  correspondence corner a coupled pair of legs — every value inside a
  tick body keeps two legs, as every wire does outside, so the tick
  former can project the machine's run and the denotation's run from
  ONE run of the body (there are no bare values in a slice: Rust's
  `num_payloads.zip(base_slot).map(|(n, b)| b + n)` is `bsZip` +
  `bsMap`). Wires across ticks stay `Ticked ℓ σ` over the plain type. -/
  BoundedSingleton : Type → Type
  -- (Rust `Optional<σ, Tick<ℓ>, Bounded>` inside a tick is the tick
  -- singleton of an `Option`: `BoundedOptional σ := BoundedSingleton
  -- (Option σ)`, a derived spelling below the class — the wire it
  -- enters from is `Ticked ℓ (Option σ)`, one carrier)
  /-- A tick-located bounded stream wire = the ticked trace of per-tick
  bounded streams. This is the **default** (the identity holds
  definitionally in `Values`, `Sched`, `Eager`); the coupling
  interpretation (`Couple`) overrides it, because a coupling relates
  two *wires* whose legs carry
  different history axes — a trace of per-element pairs would put the
  step axis on the wrong leg. -/
  TickStream : L → (α : Type) → [DecidableEq α] → StrOrd → Retries → Type :=
    fun ℓ α _ ord ret => Ticked ℓ (BoundedStream α ord ret)
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
  /-- `.sample_every` timing (concrete tick times in every
  interpretation — sampling is genuine timing freedom). -/
  SampleDec : Nat → Type
  /-- `.timeout` verdicts (pure timing). -/
  TimerDec : Nat → Type
  /-- `source_interval` pulses (pure timing). -/
  PulseDec : Nat → Type
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
  /-- Rust's **`broadcast_closed`** (networking.rs): one-to-all
  shipping under **closed membership** (the recipient set is the
  type-level `Fin (mem p)` — fixed at deploy time), **keyed by
  sender** at each receiver — pure plumbing, no decision (the
  interleave nondeterminism rides `values`'s `NoOrder` type); per-key
  content keeps its grade. The mirrored programs call *dynamic*
  `.broadcast(…, nondet!(…))`, whose extra `nondet_membership` guards
  the membership snapshot under async joins — freedom this model
  deliberately assumes away (the D20/C-table upstream note: those
  call sites should be `broadcast_closed`, or the model needs open
  membership). -/
  broadcast_closed : ∀ {c p : L} {α : Type} [DecidableEq α] {ord : StrOrd}
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
  retry duplication is the increment's freedom). The singleton's bound
  does not transport: the tick read is a plain ticked trace (its ascent,
  for a `Monotonic` source, is a contract fact). -/
  snapshot : ∀ {ℓ : L} {α σ : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} {b : SingBound σ},
    Singleton ℓ α σ ord ret b →
    (d : SnapDec (mem ℓ) α ord) → Ticked ℓ σ
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
  /-- `.latest().sample_every(q!(dur), nondet!(…))`: read the live latest
  value at decision-chosen tick times (an absent latest emits nothing;
  reads of unrealized ticks **block**, so under the surrounding election
  `fix` the samples stabilize by prefix). **Sampling introduces
  `AtLeastOnce`**: an unchanged latest sampled twice is a consecutive
  stutter — exactly the `TotalOrder × AtLeastOnce` quotient. -/
  sample_every : ∀ {ℓ : L} {α : Type} [DecidableEq α],
    Ticked ℓ (Option α) →
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
    (d : TimerDec (mem ℓ)) → Ticked ℓ Bool
  /-- `source_interval_delayed(…).batch(&tick, nondet!(…)).first()
  .is_some()`: a pure timing source, per tick. -/
  source_interval_batch : ∀ {ℓ : L},
    (d : PulseDec (mem ℓ)) → Ticked ℓ Bool
  /-- `.map(q!(…))` on a tick singleton. -/
  mapTick : ∀ {ℓ : L} {α β : Type},
    Ticked ℓ α → (Fin (mem ℓ) → α → β) →
    Ticked ℓ β
  /-- `.zip(…)` of two tick singletons — same-tick alignment (tick
  atomicity). -/
  zipTick : ∀ {ℓ : L} {α β : Type},
    Ticked ℓ α → Ticked ℓ β →
    Ticked ℓ (α × β)
  /-- Rust `.defer_tick()` seeded with an initial state: tick `t` reads
  tick `t-1` (tick 0 reads the seed) — the guardedness device. -/
  defer_tick : ∀ {ℓ : L} {σ : Type}, σ → Ticked ℓ σ →
    Ticked ℓ σ
  /-- Rust `.all_ticks()`: leave the tick — per-tick batches concatenate
  in tick order and the batch grade is **preserved** (`Stream<T, L,
  Unbounded, O, R>` from `Stream<T, Tick<L>, Bounded, O, R>`). -/
  allTicks : ∀ {ℓ : L} {β : Type} [DecidableEq β] {ord : StrOrd},
    TickStream ℓ β ord .exactlyOnce → Stream ℓ β ord .exactlyOnce
  /-- Per-tick emission lists entering the tick boundary as an ordered
  bounded batch stream (the identity staging of a computed batch). -/
  flattenOrdered : ∀ {ℓ : L} {β : Type} [DecidableEq β],
    Ticked ℓ (List β) →
    TickStream ℓ β .totalOrder .exactlyOnce
  /-- Per-tick emissions whose order is a selection artifact, published
  **unordered** (Rust's `flatten_unordered` output type: the list is
  forgotten into the batch multiset — always a sound coarsening). -/
  flattenUnordered : ∀ {ℓ : L} {β : Type} [DecidableEq β],
    Ticked ℓ (List β) →
    TickStream ℓ β .noOrder .exactlyOnce
  /-- Rust `forward_ref`/`complete_cycle` on a stream wire: guarded
  Kleene iteration from the empty wire; the decision is the unfolding
  depth for the denotation (`Unit` for the step machine, whose cycle
  unfolding is step progression). -/
  fix_stream : ∀ {ℓ : L} {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries}, (d : FixDec) →
    (Stream ℓ α ord ret → Stream ℓ α ord ret) → Stream ℓ α ord ret
  /-- `forward_ref` on a ticked wire (the `a_log` knot shape). -/
  fix_tick : ∀ {ℓ : L} {σ : Type},
    (d : FixDec) →
    (Ticked ℓ σ → Ticked ℓ σ) →
    Ticked ℓ σ
  -- ### In-tick operators — the `sliced!` body vocabulary (D60)
  --
  -- Operators on one tick's bounded stream content, Rust's
  -- `Stream<α, Tick<ℓ>, Bounded, ord, ret>` API inside a slice. Closures
  -- take no member id (the enclosing tick body already has `me`).
  -- Grade constraints are Rust's trait bounds: order-sensitive
  -- consumption (`enumerate`, `first`, `flat_map_ordered`) exists only
  -- at `TotalOrder`; `count` only at `ExactlyOnce`; `fold` carries the
  -- graded `FoldOk` obligation (`commutative =`/`idempotent =`). Every
  -- operator is implemented twice in substance — on the denotation's
  -- quotient and on the step machine's plain `List` — and the Couple
  -- corner proves, per operator, that the two agree under the per-tick
  -- batch coupling (`CoupledBatch`): that proof is what the
  -- quotient-vs-list split buys.
  /-- `.map(q!(…))` inside the tick. -/
  bmap : ∀ {α β : Type} [DecidableEq α] [DecidableEq β] {ord : StrOrd},
    BoundedStream α ord .exactlyOnce → (α → β) →
    BoundedStream β ord .exactlyOnce
  /-- `.filter_map(q!(…))` inside the tick. -/
  bfilterMap : ∀ {α β : Type} [DecidableEq α] [DecidableEq β] {ord : StrOrd},
    BoundedStream α ord .exactlyOnce → (α → Option β) →
    BoundedStream β ord .exactlyOnce
  /-- `.flat_map_ordered(q!(…))`: order-preserving flatten — `TotalOrder`
  only. -/
  bflatMapOrdered : ∀ {α β : Type} [DecidableEq α] [DecidableEq β],
    BoundedStream α .totalOrder .exactlyOnce → (α → List β) →
    BoundedStream β .totalOrder .exactlyOnce
  /-- `.flat_map_unordered(q!(…))`: flatten per-element bounded streams
  into an unordered stream (the closure yields bounded streams — their
  order history is already the runtime's; a Lean collection *value*
  enters as a stream through `bofList`). -/
  bflatMapUnordered : ∀ {α β : Type} [DecidableEq α] [DecidableEq β]
    {ord : StrOrd},
    BoundedStream α ord .exactlyOnce →
    (α → BoundedStream β .noOrder .exactlyOnce) →
    BoundedStream β .noOrder .exactlyOnce
  /-- Rust `Vec::into_iter()` into a `NoOrder` stream: a Lean list value
  enters the tick as unordered content (the list's order is forgotten
  by the type; the runtime keeps it, deterministically). -/
  bofList : ∀ {β : Type} [DecidableEq β],
    List β → BoundedStream β .noOrder .exactlyOnce
  /-- `.count()` — `ExactlyOnce` only (retries would be counted). -/
  bcount : ∀ {α : Type} [DecidableEq α] {ord : StrOrd},
    BoundedStream α ord .exactlyOnce → BoundedSingleton Nat
  /-- `.fold(q!(init), q!(g))` inside the tick, with the graded
  obligation. -/
  bfold : ∀ {α σ : Type} [DecidableEq α] {ord : StrOrd} {ret : Retries}
    (g : σ → α → σ) (init : σ), FoldOk ord ret g →
    BoundedStream α ord ret → BoundedSingleton σ
  /-- `.enumerate()` — `TotalOrder` only (an unordered batch has no
  positions without `assume_ordering`). -/
  benumerate : ∀ {α : Type} [DecidableEq α],
    BoundedStream α .totalOrder .exactlyOnce →
    BoundedStream (Nat × α) .totalOrder .exactlyOnce
  /-- `.first()` — `TotalOrder` only. -/
  bfirst : ∀ {α : Type} [DecidableEq α],
    BoundedStream α .totalOrder .exactlyOnce → BoundedSingleton (Option α)
  /-- `.cross_singleton(s)` with this tick's singleton. -/
  bcrossSingleton : ∀ {α σ : Type} [DecidableEq α] [DecidableEq σ]
    {ord : StrOrd},
    BoundedStream α ord .exactlyOnce → BoundedSingleton σ →
    BoundedStream (α × σ) ord .exactlyOnce
  /-- `.chain(other)`: concatenation at `TotalOrder`, union at
  `NoOrder`. -/
  bchain : ∀ {α : Type} [DecidableEq α] {ord : StrOrd},
    BoundedStream α ord .exactlyOnce → BoundedStream α ord .exactlyOnce →
    BoundedStream α ord .exactlyOnce
  /-- `.weaken_ordering()` — forget the order into the `NoOrder`
  quotient (a sound coarsening; Rust inserts it implicitly through
  `MinOrder` at `chain` and through `From` at call sites). -/
  bweakenOrder : ∀ {α : Type} [DecidableEq α] {ord : StrOrd},
    BoundedStream α ord .exactlyOnce → BoundedStream α .noOrder .exactlyOnce
  /-- `.filter(q!(…))` inside the tick. -/
  bfilter : ∀ {α : Type} [DecidableEq α] {ord : StrOrd},
    BoundedStream α ord .exactlyOnce → (α → Bool) →
    BoundedStream α ord .exactlyOnce
  /-- `.into_keyed().fold(q!(init), q!(g)).entries()` — the keyed fold,
  read back as its entries (Rust's keyed collections are collapsed onto
  `NoOrder` entry streams, each key once; a keyed carrier is deferred).
  The obligation is the input order's graded `FoldOk` (quorum.rs's
  `commutative = manual_proof!`). -/
  bkeyedFold : ∀ {K V A : Type} [DecidableEq K] [DecidableEq V] [DecidableEq A]
    {ord : StrOrd} (g : A → V → A) (init : A), FoldOk ord .exactlyOnce g →
    BoundedStream (K × V) ord .exactlyOnce →
    BoundedStream (K × A) .noOrder .exactlyOnce
  /-- `.keys()` of a keyed collection (its entries' keys). -/
  bkeys : ∀ {K A : Type} [DecidableEq K] [DecidableEq A],
    BoundedStream (K × A) .noOrder .exactlyOnce →
    BoundedStream K .noOrder .exactlyOnce
  /-- `.join(other)`: the pairs of entries at a common key (`NoOrder`). -/
  bjoin : ∀ {K V W : Type} [DecidableEq K] [DecidableEq V] [DecidableEq W]
    {ord ord' : StrOrd},
    BoundedStream (K × V) ord .exactlyOnce →
    BoundedStream (K × W) ord' .exactlyOnce →
    BoundedStream (K × (V × W)) .noOrder .exactlyOnce
  /-- `.anti_join(keys)`: the entries whose key is absent from `keys`. -/
  bantiJoin : ∀ {K V : Type} [DecidableEq K] [DecidableEq V] {ord ord' : StrOrd},
    BoundedStream (K × V) ord .exactlyOnce → BoundedStream K ord' .exactlyOnce →
    BoundedStream (K × V) ord .exactlyOnce
  /-- `.filter_not_in(other)`: the items absent from `other`. -/
  bfilterNotIn : ∀ {α : Type} [DecidableEq α] {ord ord' : StrOrd},
    BoundedStream α ord .exactlyOnce → BoundedStream α ord' .exactlyOnce →
    BoundedStream α ord .exactlyOnce
  /-- `.max()` — the running maximum of an `Ord` type, `None` when
  empty (Rust's `Optional`). -/
  bmax : ∀ {α : Type} [DecidableEq α] [LinearOrder α] {ord : StrOrd},
    BoundedStream α ord .exactlyOnce → BoundedSingleton (Option α)
  /-- `.filter_if(flag)` with this tick's boolean singleton. -/
  bfilterIf : ∀ {α : Type} [DecidableEq α] {ord : StrOrd} {ret : Retries},
    BoundedStream α ord ret → BoundedSingleton Bool → BoundedStream α ord ret
  /-- A constant tick singleton (`l.singleton(q!(v))`; register seeds). -/
  bsPure : ∀ {σ : Type}, σ → BoundedSingleton σ
  /-- `.map(q!(…))` on this tick's singleton. -/
  bsMap : ∀ {σ τ : Type}, BoundedSingleton σ → (σ → τ) → BoundedSingleton τ
  /-- `.zip(other)` of two tick singletons. -/
  bsZip : ∀ {σ τ : Type}, BoundedSingleton σ → BoundedSingleton τ →
    BoundedSingleton (σ × τ)
  /-- `.map(q!(…))` on this tick's optional. -/
  boMap : ∀ {σ τ : Type}, BoundedSingleton (Option σ) → (σ → τ) →
    BoundedSingleton (Option τ)
  /-- `.unwrap_or(singleton)`: the optional's value, else the
  singleton's. -/
  boUnwrapOr : ∀ {σ : Type}, BoundedSingleton (Option σ) → BoundedSingleton σ →
    BoundedSingleton σ
  /-- `.filter(q!(…))` on this tick's optional. -/
  boFilter : ∀ {σ : Type}, BoundedSingleton (Option σ) → (σ → Bool) →
    BoundedSingleton (Option σ)
  /-- `.into_singleton()`-style read of an optional as a singleton
  (Rust `Optional::unwrap_or`-free: a `Some`-presence flag + value). -/
  boIsSome : ∀ {σ : Type}, BoundedSingleton (Option σ) → BoundedSingleton Bool
  /-- **The tick former** (Rust `sliced!` with `use::state`): a per-tick
  body over this tick's bounded values — the register(s) (previous
  tick's value), the zipped input slices — producing the next register
  and this tick's emissions; the structural fold over the location's
  ticks (`cycle_with_initial` + `defer_tick` + body: Hydro compiles no
  instantaneous cycle, so a tick register is a fold, not a fixpoint).
  One field for every arity: registers, inputs and emissions are
  shapes (a register may be a singleton — `use::state` — or a bounded
  stream persisted across ticks — `use::state_null::<Stream<…>>`; a
  stream register keeps the machine's list order, which is what makes
  the quorum bodies writable with no emission decision).
  Programs never write it — the `tick` construct does. -/
  tick_scan : ∀ {ℓ : L} (sts ins outs : TickShape),
    TickedOf Ticked TickStream ℓ ins →
    (Fin (mem ℓ) → BoundedOf BoundedSingleton BoundedStream sts →
      BoundedOf BoundedSingleton BoundedStream ins →
      BoundedOf BoundedSingleton BoundedStream sts
        × BoundedOf BoundedSingleton BoundedStream outs) →
    SeedOf sts → TickedOf Ticked TickStream ℓ outs

/-- Rust `Optional<σ, Tick<ℓ>, Bounded>` inside a tick: the tick
singleton of an `Option` (the wire it is read from is `Ticked ℓ
(Option σ)`; the `bo*` operators are its Rust `Optional` API). -/
@[reducible] def HydroSem.BoundedOptional {L : Type} {mem : L → Nat}
    (H : HydroSem L mem) (σ : Type) : Type :=
  H.BoundedSingleton (Option σ)

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

/-- `forward_ref` on a ticked wire, curried-captures form. -/
def HydroSem.fixTick {L : Type} {mem : L → Nat} (H : HydroSem L mem)
    {Γ : HydroSem L mem → Type} {ℓ : L} {σ : Type}
    (d : H.FixDec) (caps : Γ H)
    (body : ∀ (H' : HydroSem L mem), Γ H' →
      H'.Ticked ℓ σ → H'.Ticked ℓ σ) :
    H.Ticked ℓ σ :=
  H.fix_tick d (body H caps)

end Hydro
