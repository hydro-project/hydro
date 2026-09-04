import Hydro.Trace
import Hydro.Keyed
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
| `KeyedStream ℓ K V ord ret` | `KeyedStream<K, V, ℓ, Unbounded, (ord, ret)>` (the network's is `K = MemberId`) |
| `KeyedSingleton ℓ α K V ord ret b` | `KeyedSingleton<K, V, ℓ, b>` (b ∈ {Unbounded, BoundedValue}; `α ord ret` describe its keyed source, as `Singleton`'s `α ord ret` do) |
| `Singleton ℓ α σ b` | `Singleton<σ, ℓ, b>` (b ∈ {Unbounded, Monotonic}) |
| `Atomic ℓ σ` | `Singleton<σ, Tick<ℓ>, Bounded>` (as a wire across ticks) |
| `Atomic ℓ (Option σ)` | `Optional<σ, Tick<ℓ>, Bounded>` (as a wire) |
| `Atomic ℓ (BoundedStream α ord ret)` (= `AtomicStream ℓ α ord ret`) | `Stream<α, Tick<ℓ>, Bounded, (ord, ret)>` (as a wire) |
| `BoundedSingleton σ` / `BoundedOptional σ` / `BoundedStream α ord ret` | the same three, **inside** a tick body (this tick's values) |
| `BoundedKeyedStream K V ord ret` / `BoundedKeyedSingleton K V` | `KeyedStream<K, V, Tick<ℓ>, Bounded, …>` / `KeyedSingleton<K, V, Tick<ℓ>, Bounded>` inside a tick body |
| `AtomicKeyedSingleton ℓ K V` (= `Atomic ℓ (BoundedKeyedSingleton K V)`) | `KeyedSingleton<K, V, Tick<ℓ>, Bounded>` (as a wire across ticks) |

A tick-located trace is also the **`Atomic<ℓ>` collection**: Rust's
`Stream<α, Atomic<ℓ>, Unbounded>` is the unbounded stream as the tick
sees it — its batches, tick-synchronously — i.e. `AtomicStream ℓ α`, and
`KeyedSingleton<K, V, Atomic<ℓ>, Unbounded>` is `AtomicKeyedSingleton ℓ K V`; the
runtime's `all_ticks_atomic`/`batched_atomic` are casts
(`HydroNode::Cast`) and identities here — so Rust's
`x.across_ticks(|s| s.op(…))` is written `H.op x …` directly on the
atomic wire. `Bounded*` is the collection
*inside* `Tick<ℓ>` (one tick's value); `Atomic`/`AtomicStream`/`AtomicKeyedSingleton`
is the unbounded-over-ticks wire.

A located wire across ticks is `Atomic ℓ ⟨per-tick element⟩` — the
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

| grade | `FoldOk … Pre g` obligation |
|---|---|
| `TotalOrder, ExactlyOnce` | none |
| `NoOrder, ExactlyOnce` | commutativity on pairs satisfying `Pre` |
| `TotalOrder, AtLeastOnce` | consecutive idempotence |
| `NoOrder, AtLeastOnce` | commutativity on `Pre`-pairs ∧ idempotence |

(`Pre := fun _ _ => True` is the unconditional obligation; a non-`True`
`Pre` is Rust's `manual_proof!` made honest — see `FoldOk` below.)

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
`SnapshotCuts`, `OrderSelection`, `StutterBatchCuts`,
`ClockReadings`, `UnfoldFuel` — a signature reads as
its dataflow contract, not its encoding.

**The three kinds of decision** (the nondet / sched-det split; checked
per module by `#nondet_census`):

| kind | families | `Values` | `Sched` | who reasons about it |
|---|---|---|---|---|
| **nondet** | `SnapDec` `BatchDec` `OrderSelDec` `TicksDec` `ClockDec` | real | `Unit` (content) / real (timing) | **proofs** — these mirror Rust `nondet!` sites; they live in the program's `…Dec` records and appear in `ensures` contracts |
| **sched-det** | `TransportDec` `IntervalDec` | **`Unit`** | cursors / fire-count scripts | nobody — adversary/runtime freedom, silently ∀-quantified like the delivery cursors and pacing; they live in the per-module `…Sched` bundles (one trailing `sched` binder), never in contracts |
| **fuel** | `FixDec` | Kleene depth | `Unit` | machinery — the fueled-`fix` artifact (the corner pins it to the horizon); kept in the `…Dec` records because ascent proofs consume it |

Rust-tally accounting: the `nondet!(/** TODO */)` at dynamic
`broadcast_closed` sites is `nondet_membership` — it guards the **cluster
membership snapshot** (async member joins), NOT delivery. Our model
assumes closed membership (type-level `Fin n`; Rust's
`broadcast_closed` counterpart, which takes no `nondet!` — the D20
finding), so those sites correspond to freedom we assume away, not to
`TransportDec`. `TransportDec` is per-pair delivery cursors — machine
freedom Rust never marks (`demux`/`send` take no `nondet!` at all);
`IntervalDec` (D91) is an interval timer's fire counts — likewise unmarked
(`source_interval` takes no `nondet!`: its `AtLeastOnce` stream of `()` is
one class, and the timing is the downstream `batch`'s `nondet!`).
So a module's Rust `nondet!` count = its nondets + its dynamic
broadcasts; unordered emissions carry no decision (a `tick` block
computes them on the machine's plain lists — D61) and cycles carry no
`nondet!` (the fuel is our fixpoint
encoding).
-/

namespace Hydro

/-- The graded fold obligation (Rust's properties API): exactly the
respect proof the content quotient demands (`Grades.FoldOkP`),
**conditional on a pairwise prerequisite** `Pre` on the folded values
(D71, R3): commutativity need only hold on pairs satisfying `Pre` — the
honest typing of Rust's `commutative = manual_proof!(…)` sites, whose
closures commute only under a protocol fact (paxos.rs:862 the log merge:
same ballot ⇒ same value; :869 the map insert: distinct keys). The
content half — every two values that meet satisfy `Pre` — is not an
operator argument (a program against an abstract `H` cannot prove
content): a `hydro def` proves it at the denotation **before the site**
(`ghost pre`, mandatory for any non-`True` `Pre`), the construct packs
those proofs into the module's `wf` artifact, and the corner's `wf`
consumes it. `Pre := fun _ _ => True` is the unconditional obligation. -/
abbrev FoldOk {α σ : Type _} (ord : StrOrd) (ret : Retries)
    (Pre : α → α → Prop) (g : σ → α → σ) : Prop := FoldOkP ord ret Pre g

/-- The shape of a `ghost pre` theorem's conclusion: the module's
`requires` clause `R` implies the site fact `S` (a named wrapper so the
closer `hydro_pre_close` finds the split unambiguously). -/
def PreFact (R S : Prop) : Prop := R → S

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
  /-- A keyed-singleton wire's slice: this tick's map. -/
  | ksing (K V : Type) (instK : DecidableEq K) (instV : DecidableEq V)
  /-- A keyed-stream wire's slice: this tick's keyed batch (Rust
  `KeyedStream<K, V, Tick<L>, Bounded, O, R>` entering through
  `KeyedStream::batch` or leaving through `KeyedStream::all_ticks`, D81). -/
  | kstream (K V : Type) (instK : DecidableEq K) (instV : DecidableEq V)
      (ord : StrOrd) (ret : Retries)
  /-- Zip. -/
  | pair (a b : TickShape)

/-- The wires of a shape, for an interpretation's ticked / tick-stream
carriers. -/
@[reducible] def AtomicOf {L : Type} (Tk : L → Type → Type)
    (TS : L → (α : Type) → [DecidableEq α] → StrOrd → Retries → Type)
    (TKS : L → (K V : Type) → [DecidableEq K] → [DecidableEq V] → Type)
    (TKStr : L → (K V : Type) → [DecidableEq K] → [DecidableEq V] → StrOrd → Retries → Type)
    (ℓ : L) : TickShape → Type
  | .sing τ => Tk ℓ τ
  | .stream α inst ord ret => @TS ℓ α inst ord ret
  | .ksing K V iK iV => @TKS ℓ K V iK iV
  | .kstream K V iK iV ord ret => @TKStr ℓ K V iK iV ord ret
  | .pair a b => AtomicOf Tk TS TKS TKStr ℓ a × AtomicOf Tk TS TKS TKStr ℓ b

/-- This tick's values of a shape, for an interpretation's bounded
families (the body's input/emission tuple). -/
@[reducible] def BoundedOf (BSing : Type → Type)
    (BStr : (α : Type) → [DecidableEq α] → StrOrd → Retries → Type)
    (BKS : (K V : Type) → [DecidableEq K] → [DecidableEq V] → Type)
    (BKStr : (K V : Type) → [DecidableEq K] → [DecidableEq V] → StrOrd → Retries → Type) :
    TickShape → Type
  | .sing τ => BSing τ
  | .stream α inst ord ret => @BStr α inst ord ret
  | .ksing K V iK iV => @BKS K V iK iV
  | .kstream K V iK iV ord ret => @BKStr K V iK iV ord ret
  | .pair a b => BoundedOf BSing BStr BKS BKStr a × BoundedOf BSing BStr BKS BKStr b

/-- The seed of a shaped register: a value for a singleton register
(`use::state(|l| l.singleton(q!(v)))`), nothing for a stream register
(`use::state_null`: it starts empty), componentwise for a tuple. -/
@[reducible] def SeedOf : TickShape → Type
  | .sing τ => τ
  | .stream _ _ _ _ => Unit
  | .ksing _ _ _ _ => Unit
  | .kstream _ _ _ _ _ _ => Unit
  | .pair a b => SeedOf a × SeedOf b

/-- Rust's singleton boundedness marker (`Monotonic` carries the value
order the `monotone =` obligation is about). -/
inductive SingBound (σ : Type _) where
  | unbounded
  | monotonic (vo : ValueOrder σ)

/-- Rust's `KeyedSingletonBound` (keyed_singleton.rs): `Unbounded` (keys
and values may change, keys may be removed) or `BoundedValue` (keys are
only added and a key's value never changes once present — the
`fold_early_stop` output; what `get_max_key`/`entries` demand). -/
inductive KSBound where
  | unbounded
  | boundedValue

/-- The Hydro combinator signature over a location alphabet `L` with
cluster sizes `mem`. Carriers are location-indexed **whole collections**
(SPMD: a cluster-located value denotes every member's wire at once).
Closures take the member id first — `q!` capturing `CLUSTER_SELF_ID`. -/
class HydroSem (L : Type) (mem : L → Nat) where
  Stream : L → (α : Type) → [DecidableEq α] → StrOrd → Retries → Type
  /-- Rust `KeyedStream<K, V, ℓ, Unbounded, ord, ret>`: per-key graded
  content, keys unordered. The network's `broadcast`/`demux` outputs are
  this family keyed by the sender member (`K = Fin (mem c)`). -/
  KeyedStream : L → (K V : Type) → [DecidableEq K] → [DecidableEq V] →
    StrOrd → Retries → Type
  Singleton : L → (α σ : Type) → [DecidableEq α] →
    StrOrd → Retries → SingBound σ → Type
  /-- Rust `KeyedSingleton<K, V, ℓ, b>` at a root location: a keyed
  fold's live map. `α ord ret` describe the keyed SOURCE it is read
  from (its pair type and grade — the read-cut vocabulary), as
  `Singleton`'s do. -/
  KeyedSingleton : L → (α K V : Type) → [DecidableEq α] → [DecidableEq K] →
    [DecidableEq V] → StrOrd → Retries → KSBound → Type
  /-- **The tick-located trace** of a per-tick value: Rust's
  `…<_, Tick<ℓ>, Bounded>` collections *are* the per-tick element types
  (`BoundedSingleton σ = σ`, `BoundedOptional σ = Option σ`,
  `BoundedStream α ord ret`), and a located wire across ticks is
  `Atomic ℓ ⟨that element type⟩` — one value per tick per member. The
  denotation's `Atomic` is the realized tick sequence; the step
  machine's additionally records at which machine step each tick was
  realized (its carrier is step-indexed). No boundedness grade: a tick
  collection is always `Bounded` in Rust; cross-tick *ascent* is a
  guarantee and lives in `ensures` faces, not in the type. -/
  Atomic : L → Type → Type
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
  `bsMap`). Wires across ticks stay `Atomic ℓ σ` over the plain type. -/
  BoundedSingleton : Type → Type
  /-- Rust `KeyedStream<K, V, Tick<ℓ>, Bounded, ord, ret>` inside a tick:
  this tick's keyed content (the denotation's `KeyedPool`, the machine's
  flat `List (K × V)` — Rust's `into_keyed` is a cast at the runtime). -/
  BoundedKeyedStream : (K V : Type) → [DecidableEq K] → [DecidableEq V] →
    StrOrd → Retries → Type
  /-- Rust `KeyedSingleton<K, V, Tick<ℓ>, Bounded>` inside a tick: this
  tick's keyed map (one value per key; the machine's association list). -/
  BoundedKeyedSingleton : (K V : Type) → [DecidableEq K] → [DecidableEq V] → Type
  -- (Rust `Optional<σ, Tick<ℓ>, Bounded>` inside a tick is the tick
  -- singleton of an `Option`: `BoundedOptional σ := BoundedSingleton
  -- (Option σ)`, a derived spelling below the class — the wire it
  -- enters from is `Atomic ℓ (Option σ)`, one carrier)
  /-- A tick-located bounded stream wire = the ticked trace of per-tick
  bounded streams. This is the **default** (the identity holds
  definitionally in `Values`, `Sched`, `Eager`); the coupling
  interpretation (`Couple`) overrides it, because a coupling relates
  two *wires* whose legs carry
  different history axes — a trace of per-element pairs would put the
  step axis on the wrong leg. -/
  AtomicStream : L → (α : Type) → [DecidableEq α] → StrOrd → Retries → Type :=
    fun ℓ α _ ord ret => Atomic ℓ (BoundedStream α ord ret)
  /-- A tick-located keyed-singleton wire (`KeyedSingleton<K, V, Tick<ℓ>,
  Bounded>` across ticks) = the ticked trace of per-tick keyed maps; the
  default, overridden by the coupling corner for the same reason as
  `AtomicStream`. -/
  AtomicKeyedSingleton : L → (K V : Type) → [DecidableEq K] → [DecidableEq V] → Type :=
    fun ℓ K V _ _ => Atomic ℓ (BoundedKeyedSingleton K V)
  /-- Rust `KeyedStream<K, V, Atomic<ℓ>, Unbounded, …>` as the tick sees it
  (per tick, this tick's keyed batch) = the atomic trace of per-tick
  keyed content; the default, overridden by the corner as `AtomicStream`
  is. -/
  AtomicKeyedStream : L → (K V : Type) → [DecidableEq K] → [DecidableEq V] →
    StrOrd → Retries → Type :=
    fun ℓ K V _ _ ord ret => Atomic ℓ (BoundedKeyedStream K V ord ret)
  /-- Transport decision at a network edge, per `(receiver, sender)`
  pair (`p`×`c` members): delivery cursors for the step machine, `Unit`
  for the denotation. -/
  TransportDec : Nat → Nat → Type
  /-- `.assume_ordering` selection: an order realization for the
  denotation, `Unit` for the step machine (arrival order *is* the
  machine's order). -/
  OrderSelDec : Nat → Type → Type
  /-- Per-tick `.assume_ordering` selection on a tick's keyed batch
  (`tassume_ordering`): an order realization per tick for the
  denotation, `Unit` for the step machine (D71). -/
  TickOrderSelDec : Nat → Type → Type
  /-- `.snapshot` read decision (graded by the source order for the
  denotation; `Unit` for the step machine — reads happen at ticks). -/
  SnapDec : Nat → Type → StrOrd → Type
  /-- `.batch` consumption decision, graded like the stream it cuts
  (Rust's one `Stream::batch` over every `(O, R)`, D91): unordered
  exactly-once content takes multiset increments, unordered at-least-once
  content sets drawn from the pool, ordered exactly-once content slice
  sizes, ordered at-least-once content the realized batches. -/
  BatchDec : Nat → Type → StrOrd → Retries → Type
  /-- `source_interval(_delayed)`'s fire timing (D91): a **sched-det** —
  `Unit` for the denotation (the stream's content is the one
  `AtLeastOnce` class of `()`s, decided by nothing), the per-tick fire
  counts for the step machine (runtime timing freedom, like the
  transport cursors; the Rust has no `nondet!` here). -/
  IntervalDec : Nat → Type
  /-- `use::snapshot(bounded_singleton, nondet!(…))` (D81): a `Bounded`
  singleton read at a tick is the constant — what the `nondet!` leaves
  open is how many ticks the runtime runs the slice (DFIR schedules a
  tick while a subgraph has work, here while the deferred register
  carries a value): a per-member **tick count** for the denotation,
  `Unit` for the step machine (ticks are its pacing). -/
  TicksDec : Nat → Type
  /-- `use::time(nondet!(…))` (D81; `Tick::current_tick_instant`,
  tick.rs:309): the wall-clock reading of each tick — pure timing, a
  per-member reading trace in every interpretation (the wired runtime
  reads its monotonic clock). -/
  ClockDec : Nat → Type
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
    Stream c α ord ret → KeyedStream p (Fin (mem c)) α ord ret
  /-- `.demux(&cluster, TCP.fail_stop().bincode())`: addressed shipping —
  each receiver keeps, per sender, exactly the payloads addressed to it.
  Pure plumbing over exactly-once content (the address is data). -/
  demux : ∀ {c p : L} {α : Type} [DecidableEq α] {ord : StrOrd},
    (d : TransportDec (mem p) (mem c)) → Stream c (Nat × α) ord .exactlyOnce →
    (addr : Fin (mem p) → Nat) → KeyedStream p (Fin (mem c)) α ord .exactlyOnce
  /-- `.values()`: forget the keys — pure; the interleaving
  nondeterminism rides the resulting `NoOrder` type. -/
  values : ∀ {ℓ : L} {K V : Type} [DecidableEq K] [DecidableEq V] {ord : StrOrd}
    {ret : Retries},
    KeyedStream ℓ K V ord ret → Stream ℓ V .noOrder ret
  /-- `.into_keyed()`: a stream of pairs as a keyed stream (a cast at the
  runtime; grouping at the denotation). -/
  into_keyed : ∀ {ℓ : L} {K V : Type} [DecidableEq K] [DecidableEq V]
    {ord : StrOrd},
    Stream ℓ (K × V) ord .exactlyOnce → KeyedStream ℓ K V ord .exactlyOnce
  /-- `KeyedStream::assume_ordering::<TotalOrder>(nondet!(…))`: realize
  unordered keyed content as per-key sequences — the selection is over
  the pair stream (every per-key order family arises from one). -/
  kassume_ordering : ∀ {ℓ : L} {K V : Type} [DecidableEq K] [DecidableEq V],
    KeyedStream ℓ K V .noOrder .exactlyOnce →
    (d : OrderSelDec (mem ℓ) (K × V)) → KeyedStream ℓ K V .totalOrder .exactlyOnce
  /-- `.fold_early_stop(q!(init), q!(|acc, v| …))` (`O: IsOrdered, R:
  IsExactlyOnce`): per key, fold until the closure signals stop; only
  the keys whose fold stopped are entries, and an entry never changes
  (`BoundedValue`). -/
  kfold_early_stop : ∀ {ℓ : L} {K V A : Type} [DecidableEq K] [DecidableEq V]
    [DecidableEq A],
    KeyedStream ℓ K V .totalOrder .exactlyOnce → (A → V → A × Bool) → A →
    KeyedSingleton ℓ (K × V) K A .totalOrder .exactlyOnce .boundedValue
  /-- `.get_max_key()` (`K: Ord`, `ValueBound = Bounded`): the entry at
  the largest key, a live optional read off the keyed source (its read
  cuts are increments of the source's pairs — `.noOrder` cuts). -/
  kget_max_key : ∀ {ℓ : L} {α K V : Type} [DecidableEq α] [DecidableEq K]
    [DecidableEq V] [LinearOrder K] {ord : StrOrd} {ret : Retries},
    KeyedSingleton ℓ α K V ord ret .boundedValue →
    Singleton ℓ α (Option (K × V)) .noOrder ret .unbounded
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
    {ret : Retries} (g : σ → α → σ) (init : σ) (Pre : α → α → Prop) [DecidableRel Pre],
    FoldOk ord ret Pre g → Stream ℓ α ord ret →
    Singleton ℓ α σ ord ret .unbounded
  /-- `.fold` with `Monotone = Proved` in addition: the inflationary
  obligation upgrades the output bound to `Monotonic`. The commutativity
  obligation is **unconditional** here: ascent of the reads along ticks
  (`.monotonic`) has no room for a refused-content branch. -/
  fold_monotone : ∀ {ℓ : L} {α σ : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (vo : ValueOrder σ) (g : σ → α → σ) (init : σ),
    FoldOk ord ret (fun _ _ => True) g → (∀ s x, vo.le s (g s x)) →
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
    (d : SnapDec (mem ℓ) α ord) → Atomic ℓ σ
  /-- `.batch(&tick, nondet!(…))` (`stream/mod.rs:2200`, generic in the
  grades): per-tick consumed increments of the live stream, legal against
  the realized pool — count-legal on unordered exactly-once content; at
  `AtLeastOnce` (D89; a set since D91) a batch is a set legal iff its
  elements are entitled (in the class's support) and nothing leaves the
  pool — a delivered element may be delivered again, in any later batch,
  in any order; on ordered exactly-once content the next
  slice of the sequence; on ordered at-least-once content (D91) a batch is
  legal while the destuttered concatenation of the batches so far stays
  in the pool's class. **The grade travels through the tick boundary**:
  the batches keep the stream's order and retry grade. -/
  batch : ∀ {ℓ : L} {α : Type} [DecidableEq α] {ord : StrOrd} {ret : Retries},
    Stream ℓ α ord ret →
    (d : BatchDec (mem ℓ) α ord ret) →
    AtomicStream ℓ α ord ret
  /-- `source_interval_delayed(q!(delay), q!(period))` (location/mod.rs:1434;
  `source_interval(q!(period))` is `delay = 0`, :1411): the interval timer as a
  stream of `()` — `Stream<(), L, Unbounded, TotalOrder, AtLeastOnce>` (D91): every
  non-empty sequence of `()` is one `AtLeastOnce` class, so the stream is
  **deterministic** and all the timing freedom (when, and how many, fires fell before a
  tick) sits in the `.batch(&tick, nondet!(…))` downstream, as the Rust's own comment
  says. The delay is per member (the Rust's is a closure over `CLUSTER_SELF_ID`); delay
  and period are the shell's. The machine's leg takes the fire-count script (`IntervalDec`,
  a sched-det). -/
  source_interval : ∀ {ℓ : L},
    (delay : Fin (mem ℓ) → Nat) → (period : Nat) →
    (d : IntervalDec (mem ℓ)) → Stream ℓ Unit .totalOrder .atLeastOnce
  /-- `.first()` on a tick-located stream (`Stream<T, Tick<L>, Bounded, O, R>::first()`,
  stream/mod.rs:1632, `O: IsOrdered`): per tick, the head of that tick's batch — the
  tick-level `first`, as `mapTick`/`zipTick` are the tick-level `map`/`zip` (D91). -/
  firstTick : ∀ {ℓ : L} {α : Type} [DecidableEq α] {ret : Retries},
    AtomicStream ℓ α .totalOrder ret → Atomic ℓ (Option α)
  /-- `.map(q!(…))` on a tick singleton. -/
  mapTick : ∀ {ℓ : L} {α β : Type},
    Atomic ℓ α → (Fin (mem ℓ) → α → β) →
    Atomic ℓ β
  /-- `.zip(…)` of two tick singletons — same-tick alignment (tick
  atomicity). -/
  zipTick : ∀ {ℓ : L} {α β : Type},
    Atomic ℓ α → Atomic ℓ β →
    Atomic ℓ (α × β)
  /-- Rust `.defer_tick()` seeded with an initial state: tick `t` reads
  tick `t-1` (tick 0 reads the seed) — the guardedness device. -/
  defer_tick : ∀ {ℓ : L} {σ : Type}, σ → Atomic ℓ σ →
    Atomic ℓ σ
  /-- Rust `.all_ticks()`: leave the tick — per-tick batches concatenate
  in tick order and the batch grade is **preserved** (`Stream<T, L,
  Unbounded, O, R>` from `Stream<T, Tick<L>, Bounded, O, R>`). -/
  allTicks : ∀ {ℓ : L} {β : Type} [DecidableEq β] {ord : StrOrd},
    AtomicStream ℓ β ord .exactlyOnce → Stream ℓ β ord .exactlyOnce
  /-- Per-tick emission lists entering the tick boundary as an ordered
  bounded batch stream (the identity staging of a computed batch). -/
  flattenOrdered : ∀ {ℓ : L} {β : Type} [DecidableEq β],
    Atomic ℓ (List β) →
    AtomicStream ℓ β .totalOrder .exactlyOnce
  /-- Per-tick emissions whose order is a selection artifact, published
  **unordered** (Rust's `flatten_unordered` output type: the list is
  forgotten into the batch multiset — always a sound coarsening). -/
  flattenUnordered : ∀ {ℓ : L} {β : Type} [DecidableEq β],
    Atomic ℓ (List β) →
    AtomicStream ℓ β .noOrder .exactlyOnce
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
    (Atomic ℓ σ → Atomic ℓ σ) →
    Atomic ℓ σ
  -- ### Operators over the atomic traces (Rust's `across_ticks(thunk) =
  -- thunk(self.all_ticks_atomic()).batched_atomic()`,
  -- keyed_stream/mod.rs:3034): the thunk's collections live at
  -- `Atomic<ℓ>` — the tick-located traces (`AtomicStream`/`AtomicKeyedSingleton`),
  -- the two casts are identities here, so a program invokes the thunk's
  -- operators DIRECTLY on the atomic wire (`across_ticks` itself has no
  -- Lean counterpart: there is nothing to pop out of). The Rust quote
  -- above the line carries the `across_ticks(|s| …)` text.
  /-- `.into_keyed()` on the atomic stream: per tick, this tick's batch
  as keyed content. -/
  tinto_keyed : ∀ {ℓ : L} {K V : Type} [DecidableEq K] [DecidableEq V]
    {ord : StrOrd},
    AtomicStream ℓ (K × V) ord .exactlyOnce → AtomicKeyedStream ℓ K V ord .exactlyOnce
  /-- `.assume_ordering::<TotalOrder>(nondet!(…))` INSIDE a tick, on the
  tick's unordered batch (D71): per tick, realize the batch as a
  sequence. The machine's order is its arrival order; the denotation's
  is the decision. (Rust's `into_keyed().assume_ordering()` on a tick
  batch is this on the pair stream followed by `binto_keyed` in the
  tick body — every per-key order family arises from one pair order,
  as for `kassume_ordering`.) -/
  tassume_ordering : ∀ {ℓ : L} {α : Type} [DecidableEq α],
    AtomicStream ℓ α .noOrder .exactlyOnce →
    (d : TickOrderSelDec (mem ℓ) α) → AtomicStream ℓ α .totalOrder .exactlyOnce
  /-- `.reduce_watermark(watermark, q!(|acc, v| …), commutative = …)`
  (`K: Ord`) on the atomic keyed stream: the runtime's `fold::<'static>`
  whole-map state, **scanned across ticks** over the per-tick batches,
  with **garbage collection** below the watermark
  (hydro-project/hydro#3189's semantics: the watermark only ever rises
  — a lower one is ignored — entries at keys below it are removed and
  arrivals at such keys are dropped). The combiner carries the graded
  obligation of its reduce step; the library-internal
  `assume_ordering`/`assume_retries` the Rust implementation performs
  are discharged by it. -/
  reduce_watermark : ∀ {ℓ : L} {K V : Type} [DecidableEq K] [Hashable K] [DecidableEq V]
    [LinearOrder K] {ord : StrOrd},
    AtomicKeyedStream ℓ K V ord .exactlyOnce → Atomic ℓ (Option K) →
    (f : V → V → V) → (Pre : V → V → Prop) → [DecidableRel Pre] →
    FoldOk ord .exactlyOnce Pre (reduceStep f) →
    AtomicKeyedSingleton ℓ K V
  -- ### D81: the bench client's vocabulary (`hydro_std/src/bench_client`,
  -- `kv_replica`, `paxos_bench`)
  /-- `KeyedStream::entries()`: the keyed content as its pair stream —
  a cast at the runtime; **unordered** (keyed_stream/mod.rs: the output
  is `NoOrder` whatever the per-key order, the keys interleave). -/
  kentries : ∀ {ℓ : L} {K V : Type} [DecidableEq K] [DecidableEq V] {ord : StrOrd},
    KeyedStream ℓ K V ord .exactlyOnce → Stream ℓ (K × V) .noOrder .exactlyOnce
  /-- `KeyedStream::map(q!(…))`: per value, keys kept. -/
  kmap : ∀ {ℓ : L} {K V W : Type} [DecidableEq K] [DecidableEq V] [DecidableEq W]
    {ord : StrOrd},
    KeyedStream ℓ K V ord .exactlyOnce → (Fin (mem ℓ) → V → W) →
    KeyedStream ℓ K W ord .exactlyOnce
  /-- `KeyedStream::all_ticks()`: a `sliced!` leaving with a keyed
  stream — per-tick keyed batches concatenate per key in tick order,
  the grade preserved. -/
  kallTicks : ∀ {ℓ : L} {K V : Type} [DecidableEq K] [DecidableEq V] {ord : StrOrd},
    AtomicKeyedStream ℓ K V ord .exactlyOnce → KeyedStream ℓ K V ord .exactlyOnce
  /-- `KeyedStream::fold(q!(init), q!(g), commutative = …)` /
  `.reduce(…)` on a live keyed stream (`K: Eq + Hash`): the per-key
  graded fold, read through increment cuts of the keyed source's pairs
  (as `kfold_early_stop`); the closure carries its graded obligation. -/
  kfold : ∀ {ℓ : L} {K V A : Type} [DecidableEq K] [Hashable K] [DecidableEq V]
    [DecidableEq A] {ord : StrOrd} (g : A → V → A) (init : A)
    (Pre : V → V → Prop) [DecidableRel Pre],
    FoldOk ord .exactlyOnce Pre g → KeyedStream ℓ K V ord .exactlyOnce →
    KeyedSingleton ℓ (K × V) K A ord .exactlyOnce .unbounded
  /-- `KeyedSingleton::snapshot(&tick, nondet!(…))` (`use::snapshot` of
  a keyed singleton): read the live map at ticks — increment cuts of
  the keyed source's pairs (`SnapDec … .noOrder`). -/
  ksnapshot : ∀ {ℓ : L} {α K V : Type} [DecidableEq α] [DecidableEq K] [DecidableEq V]
    {ord : StrOrd} {ret : Retries} {b : KSBound},
    KeyedSingleton ℓ α K V ord ret b →
    (d : SnapDec (mem ℓ) α .noOrder) → AtomicKeyedSingleton ℓ K V
  /-- `.across_ticks(|s| s.fold(q!(init), q!(g)))` (stream/mod.rs:3369:
  `all_ticks_atomic` → `fold` → `batched_atomic`): the fold's state
  **retained across ticks** — at each tick, the graded fold of every
  batch so far. -/
  tfold : ∀ {ℓ : L} {α σ : Type} [DecidableEq α] {ord : StrOrd} {ret : Retries}
    (g : σ → α → σ) (init : σ) (Pre : α → α → Prop) [DecidableRel Pre],
    FoldOk ord ret Pre g → AtomicStream ℓ α ord ret → Atomic ℓ σ
  /-- `Optional::latest()` (optional.rs:1637): the tick-located optional as the
  top-level `Optional<σ, ℓ, Unbounded>` — spelled as a singleton of an `Option`, read by
  `snapshot` through prefix cuts of the tick trace (`.totalOrder` cuts): the read is the
  **last update** in the prefix (absent before any, and absent when the latest tick's
  optional was `None`). No decision of its own: the asynchrony is the consumer's
  `snapshot` cut (D89). -/
  latest : ∀ {ℓ : L} {σ : Type} [DecidableEq σ],
    Atomic ℓ (Option σ) →
    Singleton ℓ (Option σ) (Option σ) .totalOrder .exactlyOnce .unbounded
  /-- `Optional::latest()` (optional.rs:1637) of a tick-located
  `Optional<usize>` whose consumer demands a **`Monotonic`** singleton
  (`paxos_core`'s `checkpoints`, acceptor_p2's GC bound): the live
  latest present value, read through prefix cuts of the tick trace
  (`.totalOrder` cuts), **smoothed to its running maximum** so the
  read ascends by construction. The smoothing is the identity exactly
  when the tick trace's latests already ascend — the coupling's `wf`
  conjunct, what the program's `ghost pre` proves (a knot invariant
  when the wire is a `fix_tick` knot); on a descending trace the
  machine and the denotation part ways (the refused-content branch of
  an unjustified `Monotonic` claim). -/
  latest_monotone : ∀ {ℓ : L},
    Atomic ℓ (Option Nat) →
    Singleton ℓ (Option Nat) (Option Nat) .totalOrder .exactlyOnce
      (.monotonic ValueOrder.optNat)
  /-- `use::time(nondet!(…))`: this tick's wall-clock reading
  (`Tick::current_tick_instant`, tick.rs:309). -/
  time : ∀ {ℓ : L}, (d : ClockDec (mem ℓ)) → Atomic ℓ Nat
  /-- `use::snapshot(bounded_singleton, nondet!(…))`: the constant at
  every tick the runtime runs (`TicksDec`). -/
  snapshot_bounded : ∀ {ℓ : L} {σ : Type}, σ → (d : TicksDec (mem ℓ)) → Atomic ℓ σ
  -- ### In-tick operators — the `sliced!` body vocabulary (D60)
  --
  -- `bkfold`/`bjoin`/`bantiJoin`/`bfilterNotIn` carry `[Hashable K]` where
  -- the Rust operator demands `K: Hash` (the collections carry no bound):
  -- the machine computes them through a hash map (`KeyedHash.lean`, D75).
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
  /-- `.is_empty()` — at every grade (D89: emptiness is a fact of the class). -/
  bisEmpty : ∀ {α : Type} [DecidableEq α] {ord : StrOrd} {ret : Retries},
    BoundedStream α ord ret → BoundedSingleton Bool
  /-- `.fold(q!(init), q!(g))` inside the tick, with the graded
  obligation. -/
  bfold : ∀ {α σ : Type} [DecidableEq α] {ord : StrOrd} {ret : Retries}
    (g : σ → α → σ) (init : σ) (Pre : α → α → Prop) [DecidableRel Pre],
    FoldOk ord ret Pre g →
    BoundedStream α ord ret → BoundedSingleton σ
  /-- `.enumerate()` — `TotalOrder` only (an unordered batch has no
  positions without `assume_ordering`). -/
  benumerate : ∀ {α : Type} [DecidableEq α],
    BoundedStream α .totalOrder .exactlyOnce →
    BoundedStream (Nat × α) .totalOrder .exactlyOnce
  /-- `.first()` — ordered content at either retry grade (Rust `Stream::first`'s
  `O: IsOrdered`; `first is idempotent`, stream/mod.rs:1632–1640): the head of the
  sequence, which stutter never changes. -/
  bfirst : ∀ {α : Type} [DecidableEq α] {ret : Retries},
    BoundedStream α .totalOrder ret → BoundedSingleton (Option α)
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
  /-- `.into_keyed()` inside the tick (a cast at the runtime; grouping at
  the denotation). -/
  binto_keyed : ∀ {K V : Type} [DecidableEq K] [DecidableEq V] {ord : StrOrd},
    BoundedStream (K × V) ord .exactlyOnce → BoundedKeyedStream K V ord .exactlyOnce
  /-- `KeyedStream::fold(q!(init), q!(g), commutative = …)` inside the
  tick: one value per key; the obligation is the input grade's `FoldOk`
  (quorum.rs's `commutative = manual_proof!`). -/
  bkfold : ∀ {K V A : Type} [DecidableEq K] [Hashable K] [DecidableEq V] [DecidableEq A]
    {ord : StrOrd} {ret : Retries} (g : A → V → A) (init : A)
    (Pre : V → V → Prop) [DecidableRel Pre],
    FoldOk ord ret Pre g → BoundedKeyedStream K V ord ret →
    BoundedKeyedSingleton K A
  /-- `KeyedSingleton::filter(q!(|v| …))`: keep the keys whose VALUE
  passes (Rust's closure sees the value only). -/
  bksFilter : ∀ {K V : Type} [DecidableEq K] [DecidableEq V],
    BoundedKeyedSingleton K V → (V → Bool) → BoundedKeyedSingleton K V
  /-- `KeyedSingleton::map(q!(|v| …))`. -/
  bksMap : ∀ {K V W : Type} [DecidableEq K] [DecidableEq V] [DecidableEq W],
    BoundedKeyedSingleton K V → (V → W) → BoundedKeyedSingleton K W
  /-- `KeyedSingleton::keys()`: the keys, unordered, each once. -/
  bksKeys : ∀ {K V : Type} [DecidableEq K] [DecidableEq V],
    BoundedKeyedSingleton K V → BoundedStream K .noOrder .exactlyOnce
  /-- `KeyedSingleton::entries()`: the `(key, value)` pairs, unordered. -/
  bksEntries : ∀ {K V : Type} [DecidableEq K] [DecidableEq V],
    BoundedKeyedSingleton K V → BoundedStream (K × V) .noOrder .exactlyOnce
  /-- `.join(other)`: the pairs of entries at a common key (`NoOrder`). -/
  bjoin : ∀ {K V W : Type} [DecidableEq K] [Hashable K] [DecidableEq V] [DecidableEq W]
    {ord ord' : StrOrd},
    BoundedStream (K × V) ord .exactlyOnce →
    BoundedStream (K × W) ord' .exactlyOnce →
    BoundedStream (K × (V × W)) .noOrder .exactlyOnce
  /-- `.anti_join(keys)`: the entries whose key is absent from `keys`. -/
  bantiJoin : ∀ {K V : Type} [DecidableEq K] [Hashable K] [DecidableEq V] {ord ord' : StrOrd},
    BoundedStream (K × V) ord .exactlyOnce → BoundedStream K ord' .exactlyOnce →
    BoundedStream (K × V) ord .exactlyOnce
  /-- `.filter_not_in(other)`: the items absent from `other`. -/
  bfilterNotIn : ∀ {α : Type} [DecidableEq α] [Hashable α] {ord ord' : StrOrd},
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
  /-- `.or(other)`: this tick's optional, else the other. -/
  boOr : ∀ {σ : Type}, BoundedSingleton (Option σ) → BoundedSingleton (Option σ) →
    BoundedSingleton (Option σ)
  /-- `Optional::filter_if(signal)` (optional.rs:1258): this tick's optional when the
  signal holds. -/
  boFilterIf : ∀ {σ : Type}, BoundedSingleton (Option σ) → BoundedSingleton Bool →
    BoundedSingleton (Option σ)
  /-- `.filter(q!(…))` on this tick's optional. -/
  boFilter : ∀ {σ : Type}, BoundedSingleton (Option σ) → (σ → Bool) →
    BoundedSingleton (Option σ)
  /-- `.into_singleton()`-style read of an optional as a singleton
  (Rust `Optional::unwrap_or`-free: a `Some`-presence flag + value). -/
  boIsSome : ∀ {σ : Type}, BoundedSingleton (Option σ) → BoundedSingleton Bool
  -- ### In-tick operators added for the bench client (D81): the rest of
  -- Rust's `Optional`/`Singleton`/`KeyedStream`/`KeyedSingleton` tick
  -- API the mirrored programs use (`hydro_std/src/bench_client/mod.rs`,
  -- `hydro_test/src/cluster/{paxos_bench,paxos_with_client,kv_replica}.rs`).
  /-- `Optional::is_none()`. -/
  boIsNone : ∀ {σ : Type}, BoundedSingleton (Option σ) → BoundedSingleton Bool
  /-- `Optional::zip(other)` / `Singleton::zip(optional)` (optional.rs,
  singleton.rs:830 — `ZipResult` is an `Optional` when either side is):
  present iff both are. -/
  boZip : ∀ {σ τ : Type}, BoundedSingleton (Option σ) → BoundedSingleton (Option τ) →
    BoundedSingleton (Option (σ × τ))
  /-- `Optional::zip(singleton)` (D81): present iff the optional is. -/
  boZipSingleton : ∀ {σ τ : Type}, BoundedSingleton (Option σ) → BoundedSingleton τ →
    BoundedSingleton (Option (σ × τ))
  /-- `Singleton::zip(optional)` (singleton.rs:830, D81): present iff the
  optional is. -/
  bsZipOptional : ∀ {σ τ : Type}, BoundedSingleton σ → BoundedSingleton (Option τ) →
    BoundedSingleton (Option (σ × τ))
  /-- `Optional::filter_map(q!(…))`. -/
  boFilterMap : ∀ {σ τ : Type}, BoundedSingleton (Option σ) → (σ → Option τ) →
    BoundedSingleton (Option τ)
  /-- `Optional::into_stream()`: zero or one element, in order
  (optional.rs). -/
  boIntoStream : ∀ {σ : Type} [DecidableEq σ], BoundedSingleton (Option σ) →
    BoundedStream σ .totalOrder .exactlyOnce
  /-- `Singleton::filter_if(signal)` → `Optional` (singleton.rs:911). -/
  bsFilterIf : ∀ {σ : Type}, BoundedSingleton σ → BoundedSingleton Bool →
    BoundedSingleton (Option σ)
  /-- `Singleton::filter_map(q!(…))` → `Optional` (singleton.rs:779). -/
  bsFilterMap : ∀ {σ τ : Type}, BoundedSingleton σ → (σ → Option τ) →
    BoundedSingleton (Option τ)
  /-- `.min()` — the running minimum of an `Ord` type, `None` when empty
  (stream/mod.rs:1596). -/
  bmin : ∀ {α : Type} [DecidableEq α] [LinearOrder α] {ord : StrOrd},
    BoundedStream α ord .exactlyOnce → BoundedSingleton (Option α)
  /-- `.reduce(q!(|curr, new| …), commutative = …)` inside the tick
  (stream/mod.rs:1508): the first element seeds, later ones combine
  (`reduceStep`); `None` when empty. The graded obligation is the
  combiner's, conditional on `Pre` (R3). -/
  breduce : ∀ {α : Type} [DecidableEq α] {ord : StrOrd} {ret : Retries}
    (g : α → α → α) (Pre : α → α → Prop) [DecidableRel Pre],
    FoldOk ord ret Pre (reduceStep g) →
    BoundedStream α ord ret → BoundedSingleton (Option α)
  /-- `.sort()` (stream/mod.rs:2715, `T: Ord`): one tick's content as an
  ascending sequence. A `LinearOrder` makes the result a function of the
  content (no ties between distinct elements). -/
  bsort : ∀ {α : Type} [DecidableEq α] [LinearOrder α] {ord : StrOrd},
    BoundedStream α ord .exactlyOnce → BoundedStream α .totalOrder .exactlyOnce
  /-- `.cross_singleton(optional)` (stream/mod.rs:1014, `impl
  Into<Optional>`): every element paired with the optional's value —
  nothing while it is `None`. (`bcrossSingleton` is the same method at
  a `Singleton` argument.) -/
  bcrossOptional : ∀ {α σ : Type} [DecidableEq α] [DecidableEq σ] {ord : StrOrd},
    BoundedStream α ord .exactlyOnce → BoundedSingleton (Option σ) →
    BoundedStream (α × σ) ord .exactlyOnce
  /-- `KeyedStream::map(q!(|v| …))` inside the tick (keyed_stream/mod.rs:824:
  the closure sees the value only). -/
  bkmap : ∀ {K V W : Type} [DecidableEq K] [DecidableEq V] [DecidableEq W] {ord : StrOrd},
    BoundedKeyedStream K V ord .exactlyOnce → (V → W) →
    BoundedKeyedStream K W ord .exactlyOnce
  /-- `KeyedStream::cross_singleton(s)` inside the tick
  (keyed_stream/mod.rs:1214): every value paired with this tick's
  singleton. -/
  bkcrossSingleton : ∀ {K V σ : Type} [DecidableEq K] [DecidableEq V] [DecidableEq σ]
    {ord : StrOrd},
    BoundedKeyedStream K V ord .exactlyOnce → BoundedSingleton σ →
    BoundedKeyedStream K (V × σ) ord .exactlyOnce
  /-- `KeyedStream::chain(other)` inside the tick: per key, concatenation
  at `TotalOrder`, union at `NoOrder`. -/
  bkchain : ∀ {K V : Type} [DecidableEq K] [DecidableEq V] {ord : StrOrd},
    BoundedKeyedStream K V ord .exactlyOnce → BoundedKeyedStream K V ord .exactlyOnce →
    BoundedKeyedStream K V ord .exactlyOnce
  /-- `KeyedStream::weaken_ordering()` inside the tick. -/
  bkweakenOrder : ∀ {K V : Type} [DecidableEq K] [DecidableEq V] {ord : StrOrd},
    BoundedKeyedStream K V ord .exactlyOnce → BoundedKeyedStream K V .noOrder .exactlyOnce
  /-- `KeyedStream::reduce(q!(|curr, new| …), commutative = …)` inside
  the tick (keyed_stream/mod.rs:2105): one value per key with content;
  the obligation is the input grade's, conditional on `Pre`. -/
  bkreduce : ∀ {K V : Type} [DecidableEq K] [Hashable K] [DecidableEq V]
    {ord : StrOrd} {ret : Retries} (g : V → V → V)
    (Pre : V → V → Prop) [DecidableRel Pre],
    FoldOk ord ret Pre (reduceStep g) → BoundedKeyedStream K V ord ret →
    BoundedKeyedSingleton K V
  /-- `KeyedSingleton::join_keyed_singleton(other)` (keyed_singleton.rs:888):
  the pairs of values at the keys both maps hold. -/
  bksJoin : ∀ {K V W : Type} [DecidableEq K] [Hashable K] [DecidableEq V] [DecidableEq W],
    BoundedKeyedSingleton K V → BoundedKeyedSingleton K W →
    BoundedKeyedSingleton K (V × W)
  /-- `KeyedSingleton::filter_key_not_in(keys)`: drop the keys present in
  this tick's key stream. -/
  bksFilterKeyNotIn : ∀ {K V : Type} [DecidableEq K] [Hashable K] [DecidableEq V] {ord : StrOrd},
    BoundedKeyedSingleton K V → BoundedStream K ord .exactlyOnce →
    BoundedKeyedSingleton K V
  /-- `KeyedSingleton::key_count()` (keyed_singleton.rs:615). -/
  bksKeyCount : ∀ {K V : Type} [DecidableEq K] [DecidableEq V],
    BoundedKeyedSingleton K V → BoundedSingleton Nat
  /-- `KeyedSingleton::into_keyed_stream()` (keyed_singleton.rs:1643): one
  element per key, `TotalOrder` per key. -/
  bksIntoStream : ∀ {K V : Type} [DecidableEq K] [DecidableEq V],
    BoundedKeyedSingleton K V → BoundedKeyedStream K V .totalOrder .exactlyOnce
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
    AtomicOf Atomic AtomicStream AtomicKeyedSingleton AtomicKeyedStream ℓ ins →
    (Fin (mem ℓ) → BoundedOf BoundedSingleton BoundedStream BoundedKeyedSingleton BoundedKeyedStream sts →
      BoundedOf BoundedSingleton BoundedStream BoundedKeyedSingleton BoundedKeyedStream ins →
      BoundedOf BoundedSingleton BoundedStream BoundedKeyedSingleton BoundedKeyedStream sts
        × BoundedOf BoundedSingleton BoundedStream BoundedKeyedSingleton BoundedKeyedStream outs) →
    SeedOf sts → AtomicOf Atomic AtomicStream AtomicKeyedSingleton AtomicKeyedStream ℓ outs

/-- Rust `Optional<σ, Tick<ℓ>, Bounded>` inside a tick: the tick
singleton of an `Option` (the wire it is read from is `Atomic ℓ
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
      H'.Atomic ℓ σ → H'.Atomic ℓ σ) :
    H.Atomic ℓ σ :=
  H.fix_tick d (body H caps)

end Hydro
