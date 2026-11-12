---
sidebar_position: 1
title: The Approach
---

# The Approach

Most tools for verifying distributed protocols, such as TLA+ and I/O automata, model the network as part of the state: a bag of in-flight messages, with every safety argument an inductive invariant over all interleavings of send, deliver and step. Hydro Lean proves a protocol in a denotational semantics where the network is an identity operator and nondeterminism stays unresolved inside the values, and it comes with built-in proofs that carry each such result down to the step machine and to the bytes the processes exchange. As a result, a protocol is proved once, in the semantics where the proof is easiest, and the theorem holds of the compiled processes.

The framework follows from six design decisions, each developed in one section of this page:

1. [Quotient denotations](#quotient-denotations): in the denotational semantics a stream captures all its possible histories. Nondeterminism is therefore deferred until something observes it, and the network is an identity operator. Decisions 2 to 6 make this semantics usable on real programs and connect it to execution.
2. [Tagless-final programs](#tagless-final-programs): a program is a function of its semantics, so one program text is read by every semantics.
3. [Decisions as arguments](#decisions-as-arguments): a nondeterministic choice that a program observes is an argument of the program, so a contract quantifies over every choice with no side condition.
4. [A coupling proved once](#coupling-the-denotation-to-the-step-machine): the denotation and an operational step machine are related by an instance proved once for all programs, so no program needs its own simulation proof.
5. [A dataflow graph](#the-dataflow-graph): execution goes through a compiled graph, which can be stepped where the program text cannot.
6. [Per-program proof generation](#per-program-proof-generation): every fact that links a program's levels is generated for that program by `hydro def`, so the user writes only the contract and its proof.

Decisions 1, 4 and 5 give a program three levels of meaning, with one safety theorem per level. Each level's theorem is derived from the one above by machinery proved once for all programs, so a protocol's correctness is proved only at the top level:

| level | semantics | Paxos safety | carried down by |
|---|---|---|---|
| denotational | `Values`: classes of histories, the network an identity operator | `paxos_core.ensures` | — |
| operational, through the class | `SchedSem`: a step machine on a global clock, defined operator by operator as an instance | `paxos_safe_sched` | the coupling `CoupleSem` and the generated `M_co_*` lemmas |
| operational, over the compiled graph | `Dag.step`, then the per-process step, the activation and the bytes on each connection | `paxos_safe_wired` | `M.dag_denote`, `Dag.run_spec`, `Dag.step_wired`, `Dag.iterate_eq_pass`, `WireTrace.streamOutputsAt_eq` |

The second level exists because a process cannot run a class of histories. `SchedSem` is the first place time and delivery appear, and its streams are flat sequences with one actual order, so coupling it to the denotation is what holds the denotation to what a run actually does ([below](#coupling-the-denotation-to-the-step-machine)). The third exists because an instance cannot be executed step by step ([The Dataflow Graph](#the-dataflow-graph)). [Soundness](./soundness.md) walks the chain link by link.

Each section below describes the design a decision replaces, why that design fails, the technique with its code, and its cost. The [Architecture Overview](./index.md#vocabulary) defines the vocabulary (instance, carrier, grade, knot, corner, machine run, denotational run).

## Quotient Denotations

### Operational Semantics with a Scheduler

The standard way to verify a distributed protocol is to give it an operational semantics with a scheduler, as in TLA+ or I/O automata. The state contains the network: a bag of in-flight messages per channel. A transition sends, delivers or steps one process, and the scheduler resolves every nondeterministic choice at the moment it arises: which message is delivered next, in which order a process sees its inputs, where its batch ends. A safety proof is an inductive invariant over the reachable states, so it must hold after every interleaving of send, deliver and step, including the many interleavings the protocol's result does not depend on, and much of the invariant describes the network rather than the protocol.

### Deferred Nondeterminism

Hydro Lean resolves nothing when a stream is produced. At `Values`, the denotational semantics, a stream's value is a **class** that identifies all the histories the stream could have had: every arrival order of an unordered stream, every retry count of an at-least-once one. `PoolCarrier` (`hydro_lean/Hydro/Grades.lean`) picks it per **grade** (the stream's ordering and retry markers): at exactly-once a `List` when ordered and a `Multiset` when unordered; at at-least-once a `StutterSeq` (a list modulo consecutive repetition) and a `RetryPool` (a multiset modulo multiplicity) ([Grades](./semantics/grades.md#quotient-carriers) shows the definition).

The nondeterminism then travels unresolved inside the value, and the signature decides, per operator, how it may be resolved. Three neighbouring fields of `HydroSem` (`hydro_lean/Hydro/Sem.lean`), the class whose fields are the operators every program is written against ([below](#tagless-final-programs)), show the three cases:

```lean
  union : ∀ {ℓ : L} {α : Type} [DecidableEq α] {ret : Retries},
    Stream ℓ α .noOrder ret → Stream ℓ α .noOrder ret →
    Stream ℓ α .noOrder ret
  /-- `.assume_ordering::<TotalOrder>(nondet!(…))`: realize unordered exactly-once content as
  a sequence, through the explicit selection decision. An illegal selection blocks. -/
  assume_ordering : ∀ {ℓ : L} {α : Type} [DecidableEq α],
    Stream ℓ α .noOrder .exactlyOnce →
    (d : OrderSelDec (mem ℓ) α) → Stream ℓ α .totalOrder .exactlyOnce
  /-- `.fold(q!(init), q!(g))`: the closure carries the graded obligation (`FoldOk`),
  **consumed by the content quotient**, so a fold that mishandles reordering or retries does
  not typecheck. -/
  fold : ∀ {ℓ : L} {α σ : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (g : σ → α → σ) (init : σ) (Pre : α → α → Prop) [DecidableRel Pre],
    FoldOk ord ret Pre g → Stream ℓ α ord ret →
    Singleton ℓ α σ ord ret .unbounded
```

- `union` takes neither a proof nor a decision. How the two inputs interleave depends only on delivery timing, and at `.noOrder` it is not in the value at all: at exactly-once the result is the sum of two multisets.
- `fold` resolves the nondeterminism with no choice. It takes `FoldOk ord ret Pre g`, the proof that the closure gives one result on the whole class (it commutes on unordered content, it absorbs a repeat on at-least-once content), so its result is deterministic. The Rust's `commutative = manual_proof!(…)` becomes that argument. In `collect_quorum` (`hydro_std/src/quorum.rs`, mirrored in `Hydro/Std/Quorum.lean`):

  ```rust
          let count_per_key = current_responses.clone().into_keyed().fold(
              q!(move || (0, 0)),
              q!(move |accum, value| {
                  …
              }, commutative = manual_proof!(/** increment counters is commutative */)),
          );
  ```

  ```lean
      let count_per_key := H.bkfold cqCount (0, 0) (fun _ _ => True)
        (fun s x y _ => cqCount_comm s x y) (H.binto_keyed current_responses)
  ```

- `assume_ordering` observes one particular history, which no function of the class can compute. It takes a **decision** `d`, the Lean form of the Rust `nondet!`, and the result is the history `d` selects ([below](#decisions-as-arguments)).

A program can therefore be captured as an ordinary function that _encapsulates_ the network: it maps classes to classes, given its explicit decisions. Its contract is a statement about values, proved by equations and induction over the program text. Paxos's agreement is membership in a multiset per replica (a `MemberMap n` holds one value per cluster member), with no index, timestamp or schedule (`Examples/Paxos/PaxosCore.lean`):

```lean
def SlotFunctional {n : Nat} (out : MemberMap n (Multiset (Nat × Option P))) :
    Prop :=
  ∀ (i j : Fin n) (s : Nat) (v w : Option P),
    (s, v) ∈ out i → (s, w) ∈ out j → v = w
```

### The Network in the Denotation

In network-as-state reasoning a channel is a component of the state. At `Values` the network's behaviour is encoded in types instead. A receiver gets each sender's pool, whose grade's quotient already identifies every arrival order, and merging senders gives a `.noOrder` pool that identifies every interleaving. How much has arrived by a tick is not in the pool; it is observed only by `batch` or `snapshot`, which take a decision. The network's nondeterminism is therefore deferred to the operators that observe it, and the transport itself changes no content. The broadcast at `Values` ignores its transport decision `_d` (its type is `Unit` there) and hands every receiver `_p` the same keyed pool, one group per sender `j` holding that sender's whole pool (`Hydro/Values.lean`):

```lean
  broadcast_closed _d s := .mk' fun _p => KeyedPool.transport (fun j => s j)
```

Read as one unordered stream, what a receiver gets is the sum of the senders' pools, and the lemma is `rfl`:

```lean
@[den] theorem values_broadcast_sum {c : L} {α : Type} [DecidableEq α]
    (d : (Values L mem).TransportDec (mem ℓ) (mem c))
    (s : (Values L mem).Stream c α .noOrder .exactlyOnce) (p : Fin (mem ℓ)) :
    (Values L mem).values ((Values L mem).broadcast_closed d s) p
      = ((List.finRange (mem c)).map (fun j => s j)).sum := rfl
```

The same operator in the step machine `SchedSem` (`Hydro/Sched.lean`) is where the network state lives. Each receiver `i` sees, from each sender `j`, the prefix that the delivery cursor `d i j` has handed it, gated by its own activity:

```lean
  broadcast_closed {_c p _α _ _ord _ret} d s := fun i =>
    mergeN (fun j => ((s j).deliver (pacing.active p i) (d i j)).map (fun v => (j, v)))
```

| | network as state (TLA+, I/O automata) | network as identity (`Values`) |
|---|---|---|
| what a channel is | a bag of in-flight messages in the state | nothing: the receiver's stream keys the senders' pools by sender (`KeyedPool.transport`) |
| a send, a delivery | transitions | `KeyedPool.transport`: the senders' pools, grouped by sender |
| reordering, delay, batching | interleavings the invariant must survive | encoded in the quotient types; observed only through decisions |
| a safety proof | an inductive invariant over every reachable state | an equation or implication between classes |
| where delivery is reasoned about | in every protocol proof | once, for all programs, in the coupling |

The Paxos proof therefore never mentions a message in flight. Time and delivery exist only in the operational semantics `SchedSem`, where no protocol is proved correct, and the coupling ([below](#coupling-the-denotation-to-the-step-machine)) proves once, for every program, that the step machine's runs are bounded by the denotation.

### Costs of Quotient Denotations

**Every operator must be well-defined on classes.** A consumer of class content exists only as a lift whose respect proof is an argument. A fold whose closure commutes only under a pairwise prerequisite `Pre` takes its commutativity proof conditional on `Pre`, and the denotation's fold (`Multiset.foldlOn`, a `Quot.liftOn` whose respect proof is that conditional commutativity) returns its seed on content in which some pair violates `Pre`, rather than pick an order ([Grades](./semantics/grades.md)). A fold that does not commute has no meaning on a `Multiset` at all. Paxos's recommit fold is such a fold: its `(count, champion)` state skips a slot under one arrival order of three log entries and recommits it under another, from the same multiset. The program must therefore observe a history. The Rust calls `.assume_ordering::<TotalOrder>(nondet!(…))` before the fold (`hydro_test/src/cluster/paxos.rs`), and the Lean mirror is

```lean
  let p_entries_ordered := H.tassume_ordering p_entries dec.order
```

The decision `dec.order` selects the order, and the recommit's facts are proved for every value of it.

**Arrival order is not observable at `Values`.** For a cycle `S = {1} ∪ ship(ship(S) + 1)`, with `ship` a network hop, an `assume_ordering` of `S` taken outside the cycle may select `[2, 1, 3, …]`: the class of `S` holds all three elements, and nothing in it records that `1` arrived first. "The ordered output is `1, 2, 3, …`" is then unprovable at `Values`, although every machine run satisfies it. Selecting the order inside the cycle recovers the fact, because there a selection that takes an element before the iteration has produced it is illegal and blocks.

**Time moves to the step machine and the coupling.** The denotation says what is delivered, never when, so anything about steps (that the machine's output so far is covered by the contract, that an activation is a fixpoint, that a fair schedule eventually delivers) is proved about the step machine, the coupling or the compiled graph, never in a contract. The coupling is an inclusion, which suffices for safety; liveness needs an equality and gets it separately ([Liveness](./liveness.md)).

## Tagless-Final Programs

One program is read four ways: in the denotational semantics, in the operational step machine, in a relational semantics whose values are pairs of related `Values` carriers (for monotonicity), and in the coupling of the step machine to the denotation (for the transfer to the machine). A copy of the program per reading drifts into another program, and each copy would need its own equivalence proof. The framework therefore writes the program once, as a function of an instance of the class `HydroSem`, whose fields are the carriers, the decision families and the operators. This is the **tagless-final** style: the operators are projections of the instance argument, and the caller chooses the instance.

```lean
class HydroSem (L : Type) (mem : L → Nat) where
  Stream : L → (α : Type) → [DecidableEq α] → StrOrd → Retries → Type
  …
  BatchDec : Nat → Type → StrOrd → Retries → Type
  …
  batch : ∀ {ℓ : L} {α : Type} [DecidableEq α] {ord : StrOrd} {ret : Retries},
    Stream ℓ α ord ret →
    (d : BatchDec (mem ℓ) α ord ret) →
    AtomicStream ℓ α ord ret
```

`L` is the type of locations and `mem ℓ` the size of cluster `ℓ`. The instances are the denotational `Values`, the operational `SchedSem pacing` (the **pacing** says, per member and step, whether the member idles, receives, or ticks), and the two proof instances `MonoRel` and `CoupleSem pacing Tc Td hjT`. The first two give a stream as:

```lean
@[reducible] def Values (L : Type) (mem : L → Nat) : HydroSem L mem where
  Stream ℓ α _ ord ret := MemberMap (mem ℓ) (PoolCarrier α ord ret)
```

```lean
def SchedSem (L : Type) (mem : L → Nat) (pacing : Pacing L mem) :
    HydroSem L mem where
  Stream ℓ α _ _ _ := Fin (mem ℓ) → StepHist α
```

At `Values` a stream is a `MemberMap` (one cell per cluster member) of classes. At `SchedSem` it is, per member, a step history: a list that grows step by step. At the corner (`CoStream`, [below](#coupling-the-denotation-to-the-step-machine)) it is both, coupled. A proof instance turns a per-program theorem into a projection. `MonoRel` pairs two `Values` carriers under the grade's order, and every operator proves the order for its output, so the `.property` of `M (MonoRel L mem) …` proves the monotonicity of a module `M`, with no per-program content ([Monotonicity by instantiation](./semantics/monorel.md)).

**What the style costs.** A function of an instance has no syntax to induct over. The first consequence is the shape of `fix`, the Lean form of a `forward_ref` cycle (a **knot**). A knot's coupling and monotonicity are facts about its body, and a body passed to the instance as `Stream → Stream` is opaque: it can only be applied, not re-read at another instance. A class field cannot quantify over the class itself, so the program-facing knot is a derived definition whose body is generic over the instance:

```lean
def HydroSem.fix {L : Type} {mem : L → Nat} (H : HydroSem L mem)
    {Γ : HydroSem L mem → Type} {ℓ : L} {α : Type} [DecidableEq α]
    {ord : StrOrd} {ret : Retries}
    (d : H.FixDec) (caps : Γ H)
    (body : ∀ (H' : HydroSem L mem), Γ H' →
      H'.Stream ℓ α ord ret → H'.Stream ℓ α ord ret) :
    H.Stream ℓ α ord ret :=
  H.fix_stream d (body H caps)
```

The wires and decisions the body captures travel as the tuple `caps`, typed by a family `Γ` over instances, so a proof can instantiate the same body at another instance: at `MonoRel`, or at a corner with a lower horizon. The alternative, a set of tactics that walked the body's syntax and re-derived causality and monotonicity per body, took three minutes on a toy knot. The elaborator hoists every `fix` body to its own top-level `@[reducible]` `def` of this form (`toy_loop.w.body`, [Elaborating `hydro def`](./generator/elaborating-hydro-def.md)). The second consequence is the [last decision](#per-program-proof-generation) on this page: what cannot be proved for all programs is generated for each.

## Decisions as Arguments

A Hydro program depends on two kinds of choice, and the Rust already tells them apart. **Content** choices are the ones it marks `nondet!`: where a batch ends, which order `assume_ordering` sees, what a clock reads. **Schedule** choices are made by the runtime without a marker: when a message is delivered, when a timer fires. A contract must hold for every content choice, and the step machine must admit every schedule.

Each operator with a choice takes a decision argument whose type is a field of the instance, and each instance gives the type it can use:

```lean
  TransportDec _ _ := Unit
  …
  BatchDec n α ord ret := BatchCutsOf n α ord ret
  IntervalDec _ := Unit
```

```lean
  TransportDec p c := Fin p → Fin c → Nat → Nat
  …
  BatchDec _ _ _ _ := Unit
  …
  IntervalDec n := TickScript n Nat
```

The first block is `Values`, the second `SchedSem`. The denotation represents content (a batch decision is a list of per-tick **cuts**, the multisets each tick consumes) and takes `Unit` for transport, because it delivers whole pools. The machine represents schedule (a delivery cursor per receiver, sender and step, a fire script per timer) and takes `Unit` for batches, because its tick consumes whatever has arrived. A module bundles its content decisions in a `…Dec` record, which its contract quantifies over, and its schedule in a `…Sched` record, which no contract mentions (`ToyRelayDec`, `ToyRelaySched`). `#nondet_census` counts the content decisions, schedule sites and cycles of each module and fails the build when they differ from the declared numbers ([Nondeterminism](../using/programs/nondeterminism.md)).

The placements that look simpler each put a choice in the semantics that cannot represent it. Class-valued buffers in the machine would describe no runtime, since the runtime always holds a concrete order. A merge-order decision at `union` would name a choice that follows from delivery timing and is invisible at `.noOrder`. One shared decision vocabulary, validated by the machine against its run, would put class logic inside the machine, which must stay auditable as a model of DFIR.

**Total decision spaces.** A contract quantifies over every decision record, so a record that is illegal for the run (a cut that takes an element the pool does not hold, an order that is not a permutation of the tick's batch) must still mean something. Such a record yields the run truncated at the first illegal entry. `batchCuts` keeps a tick's cut only while the consumed total stays within the pool ([Nondeterminism](../using/programs/nondeterminism.md#total-decision-spaces) shows it), and `realizeTicks` keeps a tick's order selection only while it is a permutation of that tick's batch (`Hydro/Trace.lean`):

```lean
def realizeTicks {α : Type _} [DecidableEq α] :
    Trace (Multiset α) → List (List α) → Trace (List α)
  | p :: ps, s :: ss => if Multiset.ofList s = p then s :: realizeTicks ps ss else []
  | _, _ => []
```

A theorem over all decisions therefore carries no legality premise, and an illegal record yields a shorter run that is itself the run of a legal record. The alternative, a legality predicate on every theorem, puts a premise on every headline that every consumer must discharge, and a fallback order on illegal input would invent an order the Rust does not have. Totality is also what lets the corner below derive a decision from a machine run without proving it legal first ([Values](./semantics/values.md) has the cut vocabulary per grade).

## Coupling the Denotation to the Step Machine

The framework has one **denotational** semantics, `Values`, where contracts are proved, and one **operational** semantics, `SchedSem`: a step machine that runs the program on a global clock with an explicit pacing, delivery cursors and queues of plain lists in arrival order. Every stream there is a flat, concrete sequence, so the machine always has one actual order and one actual batching, as a deployed process does, and its nondeterministic points do nothing: every content decision is `Unit`, and `assume_ordering u _d := u` is an identity operator. A safety theorem must hold of every `SchedSem` run, so the denotation must cover whatever order and batching the machine happens to produce, and a `Values` proof that relied on a convenient order would not transfer. `SchedSem` is not as close to production as the processes exchanging bytes, but it is much closer than `Values`, and it connects the two: it is an instance of the same class, so a program runs at it unchanged, and the compiled graph and the processes are proved equal to it.

A per-program simulation argument could also bridge the two semantics: given a machine run, exhibit `Values` decisions whose run covers it. Its direct form carries a premise that matching `Values` decisions exist, and a per-program proof would discharge that premise by walking the program's text. At `Values` a knot is defined as the Kleene iteration of its body from the empty wire (`Values.fix_stream`), and its iterates are its **stages**. For an inner knot that reads an outer knot's wire, the premise requires two consecutive stages of the outer knot to be equal. Consecutive stages differ until the iteration converges, so at the outer knot's first stage the premise fails for every non-empty input, and hence for every decision, every schedule and every horizon `T ≥ 1`. A headline with that premise is vacuously true, with no `sorry` anywhere to flag it.

The framework relates the two semantics instead by a third instance, the corner `CoupleSem`, once for all programs. Its stream carrier holds an operational run, a denotational run and their coupling (`Hydro/Couple.lean`):

```lean
structure CoStream (T : Nat) (n : Nat) (α : Type) [DecidableEq α]
    (ord : StrOrd) (ret : Retries) : Type where
  sr : Fin n → StepHist α
  rr : MemberMap n (PoolCarrier α ord ret)
  …
  lag : Nat
  wf : Prop
  cpl : wf → ∀ k, k + lag ≤ T → ∀ i, ListLe ord ret ((sr i).view k) (rr i)
```

`sr` is the **machine run**, `rr` the **denotational run**. `T` is the **horizon**, the last step the coupling speaks about, and `lag` how many steps short of it the coupling is guaranteed. `wf` is the **residual**, a proposition the construction leaves to be proved. `cpl` says that under `wf` the machine's list at each step `k` with `k + lag ≤ T` sits below the denotation's class in the grade's order `ListLe` (prefix at total order, sub-multiset at no order).

The corner's operators compute the `Values` decision from the machine run instead of taking one:

```lean
  batch {ℓ _α _ ord ret} s _cutd :=
    match ord, ret, s with
    | .noOrder, .exactlyOnce, s =>
      { sr := fun i t => (batchesFrom ((s.sr i).view)
          (tickSteps (pacing.ticks ℓ i) t) 0).map List.toArray
        rr := (Values L mem).batch s.rr (batchDerive (pacing.ticks ℓ) Td s.sr)
        lag := s.lag
        wf := s.wf
        cpl := fun hwf k hk i => by
          …
```

The decision argument `_cutd` is ignored. The cut is `batchDerive (pacing.ticks ℓ) Td s.sr`, the machine's tick segments up to the derive horizon `Td` read back as multisets (the coupling horizon satisfies `Tc ≤ Td`), and `corr_batch` (`Hydro/Transfer.lean`) proves that this cut realizes the machine's batches. Running a program at the corner therefore yields its operational run (`.sr`), its denotation at decisions derived from that run (`.rr`), and their coupling. The contract holds at every decision record, hence at the derived one, and the operational safety theorem has no premise about decisions. The generator emits an equation for each of the two runs per module: `toy_relay_co_sr₁` says the machine run `sr` equals the module's `SchedSem` run, `toy_relay_co_rr₁` that the denotational run `rr` equals the module's `Values` run at the derived record `toy_relay_vdec`, and `toy_safe_sched` (`Hydro/HydroGenCheck.lean`) combines them with the contract.

A knot's corner wire must be built from a body the corner can only apply. Its machine run `fixSr` is defined as the **Kleene diagonal** of the body (the wire's view at step `t` is the body iterated `t + 1` times from the empty wire, read at `t`), its denotational run the Kleene iterate, and its `wf` asks three things of the opaque body:

1. the machine body is **strict** in the knot wire (its view at a step depends only on the wire's earlier views), which comes from the graph ([below](#the-dataflow-graph));
2. the denotation body's Kleene chain ascends, from the body instantiated at `MonoRel`;
3. the coupling holds at every lower horizon `j`, from the body instantiated at the corner with horizon `j`.

`co_fix_cpl` then couples the knot by strong induction on the horizon. A network hop or a `defer_tick` reads its input one step back, so its output's lag is one less than its input's, and the induction consumes that decrement ([The Coupling Corner](./corner/coupling.md)). The coupling is an inclusion: the denotation admits orders no single operational run realizes, and safety needs no more.

## The Dataflow Graph

`SchedSem` says what a run is, but a process cannot execute it. A process must consume what arrived, emit what is new, and compute each shared wire once, and no instance can do that. An instance sees operator applications, not `let`-sharing: in `let a := …; H.fold a …; H.filterMap a …` it receives `a` twice, as a value. Nor can it step a `fix` body once per global step: it receives the body as a function, and the knot's one-step feedback is a property of the dataflow graph, not of any operator. Rust's `hydro_lang` reifies sharing and feedback through `Rc` identity, and pointer identity is `unsafe` in Lean.

The `hydro def` elaborator is the one component that sees the `let`s, so it compiles each module body into a typed graph `M.dag` (`Hydro/Graph/IR.lean`), one node per wire `let`, and proves that the graph's reading `Dag.denote` equals the module's `SchedSem` run (`M.dag_denote`). For `toy_relay`:

```lean
@toy_relay.dag_denote : ∀ {L : Type} {mem : L → ℕ} (pacing : Pacing L mem) (ℓ : L)
  (dec : ToyRelayDec (SchedSem L mem pacing) (mem ℓ)) (sched : ToyRelaySched (SchedSem L mem pacing) (mem ℓ))
  (env :
    Project.HList Project.Kind.sched [Project.Kind.stream ℓ ℕ instDecidableEqNat StrOrd.noOrder Retries.exactlyOnce]),
  Project.Dag.denote pacing (toy_relay.dag ℓ) (toy_relay.schedOf pacing ℓ dec sched) env =
    Project.HList.cons (toy_relay (SchedSem L mem pacing) ℓ (env.get Project.Var.zero) dec sched).1
      (Project.HList.cons (toy_relay (SchedSem L mem pacing) ℓ (env.get Project.Var.zero) dec sched).2
        Project.HList.nil)
```

`Dag.denote` is not a new semantics: it reads each node with `SchedSem`'s definition of the operator, so `dag_denote` says that the graph's reading at the second level equals the program's `SchedSem` run. The third level is the graph's own step-by-step execution. `Dag.step` advances every node one global step against a cell of state, consuming and emitting only what is new; the per-process step runs one member's share of it; an activation runs one tick of a process as one sweep of its nodes; and the processes exchange framed bytes. Each of these is proved once, over all graphs, to reproduce the level above it, and `paxos_safe_wired` is `paxos_safe_sched` carried down that chain.

The graph speaks the step machine's vocabulary, so a batch node carries no decision, and the timer's fire script lives in the schedule `toy_relay.schedOf …`, not in the node. The graph is therefore free of decisions and of pacing, and one graph serves every interleaving of processes. `dag_denote` is the one per-module link in the chain ([Graph and Execution](./execution/graph-ir-causality.md)).

The graph is also used above the step machine. Its lag analysis assigns each wire a **lag**, and one theorem, proved by structural induction over every graph, says that two runs whose inputs agree at their lags below a horizon have outputs that agree at the computed lags (`Dag.denote_agreeL`). A knot's strictness, which the knot coupling needs, is that theorem at the knot body's graph, carried to the program by `dag_denote`.

## Per-Program Proof Generation

The `sr`/`rr` equations, the derived decision record, the residual, monotonicity, strictness and `dag_denote` are facts about one program, and a program has no syntax to induct over, so they cannot be library theorems over all programs. Hand-written per-module lemmas would duplicate the program text.

The obvious proof is definitional unfolding. The `sr` equation is true by definition, but definitional equality through `k` nested knots costs exponentially in `k`: on `leader_election`'s body one `rfl` takes 9 ms with no knots, 21 ms with two, about 4 minutes with three (its real shape), and more than 80 minutes, exhausting a 51 M heartbeat budget, with five, under `paxos_core`. Each outer knot's body re-closes the inner ones and kernel substitution does not preserve sharing, so the term grows by a constant factor per nesting level.

`hydro def` generates them instead. It walks the module's definition against a fixed grammar, records its callees, and runs phases that synthesize each statement from the module's type and prove it by a fixed tactic script or a kernel judgement ([How Proofs Are Generated](./proof-generation.md)). The proof of a knot's `sr`/`rr` equations is a rewrite loop over the body's own equations and the inner knots' lemmas, never an unfolding through the knot, so the cost is linear in the number of knots. Kernel work is module-sized, with callees folded; a module outside the grammar is an error at its definition; and a module's whole proof cost is paid inside its one `hydro def` command ([Elaboration and Kernel Cost](./cost-model.md)).
