---
sidebar_position: 1
title: The Coupling Instance
---

# The Coupling Instance (`CoupleSem`)

A Hydro Lean program is one Lean function generic over an **instance** `H : HydroSem L mem` of the type class that supplies a carrier type for every kind of live collection and a definition for every operator ([Tagless-Final Programs](../approach.md#tagless-final-programs)). Two instances matter here.

- The denotational semantics `Values` reads a stream as a **pool** per **member** (each process of a location, or each member of a cluster): the pool captures all the histories the stream could have had, up to the differences its **grade** (its ordering and retry guarantees, [Grades](../semantics/grades.md)) allows. Every `nondet!` site takes an explicit **decision** argument ([Values](../semantics/values.md)). You prove a program's contract here, for every decision.
- The operational semantics `SchedSem` is a step machine ([The Step Machine](../semantics/sched.md)). It reads a stream as a **step history** per member, the list that member has received by each step, and it is driven by a **pacing** (which members are active and which tick at each step) and by **delivery cursors** (how far each channel has delivered). Every stream there is a flat, concrete sequence and every ordering assumption is an identity operator, so the machine has one actual order and one actual batching, as a deployed process does. Execution is proved equal to it.

A theorem about the machine speaks about its histories up to some step, the **horizon**. You prove contracts at `Values`, and the safety theorems are about the machine: whatever the machine emits up to the horizon, under any pacing and any delivery, satisfies the contract. Transferring a contract needs, for each machine run, a decision at which the `Values` run sits above the machine run. `CoupleSem` computes that decision from the machine run itself, so no premise about it is left to prove, inside cycles included. It also accounts for the one place where the two semantics differ in time: the network hop. The hop is an identity operator at `Values` and costs the machine one step.

The page first shows why the obvious transfer theorem fails, then builds the corner: its carriers, the decisions it derives from the machine run, the per-operator couplings, the lag a network hop introduces, knots and their residual, ticks, and finally how a machine theorem consumes the result.

## The Satisfiability Premise

The obvious transfer theorem quantifies over decisions and asks for a hypothesis tying the decision to the run:

```text
∀ pacing, ∀ d, Sat pacing d → machine output ⊆ Values output at d
```

`Sat` says that `d` makes the same choices the machine made: the same batch boundaries, the same arrival orders, the same timer fires. Each program discharges `Sat` before the theorem says anything, and for a program without cycles the discharge is direct.

Cycles break it. A **knot** is a `fix`, the Lean form of Rust's `forward_ref`/`complete_cycle`: a wire defined in terms of itself. At `Values` a knot is a Kleene chain, `stage 0 = ⊥` and `stage (m+1) = body (stage m)`; at the machine it is a diagonal over steps ([The Ungated Knot](../semantics/sched.md#the-ungated-knot)). `Sat` for a knot is stated per Kleene stage, and it relates the decision's reading of every wire the knot body **captures** from the enclosing program to two values at once: that wire as the stage itself presents it and as the next stage does. For a wire of the enclosing program the two coincide. When an inner knot captures an outer knot's wire (the Paxos shape: `a_log` nested inside the sequencer's knot, and leader election's knots reading `p2b`), they are two consecutive Kleene stages of the outer knot, so the inner `Sat` demands that those be equal. They are not: outer stage `0` is empty and stage `1` is not, for every nonempty input. `Sat` is then false for every decision, every pacing and every horizon `≥ 1`, and the safety theorem holds only because its premise can never be met. A build without `sorry` does not detect this, because a false premise is invisible until something tries to inhabit it.

A premise that says "a suitable decision exists" has to be proved by constructing the decision, and once the construction is the proof, the premise is redundant. `CoupleSem` is that construction, made into an instance.

## Corner Carriers

`CoupleSem L mem pacing Tc Td hjT` runs a program on the machine and at `Values` at once. Each of its carriers is a **corner**: a record holding the machine's value of the wire, the `Values` value of the same wire, and the proof that the two agree. The other corners follow the shape of the stream corner:

```lean
structure CoStream (T : Nat) (n : Nat) (α : Type) [DecidableEq α]
    (ord : StrOrd) (ret : Retries) : Type where
  sr : Fin n → StepHist α
  rr : MemberMap n (PoolCarrier α ord ret)
  /-- The wire's **lag**: … -/
  lag : Nat
  wf : Prop
  cpl : wf → ∀ k, k + lag ≤ T → ∀ i, ListLe ord ret ((sr i).view k) (rr i)
```

(`Hydro/Couple.lean`, with the `lag` docstring shortened.) The fields:

- `sr` is the **machine run**: one `StepHist` per member `i : Fin n`, a prefix-monotone history whose `view k` is what member `i` holds at step `k`. It is the `SchedSem` carrier verbatim.
- `rr` is the **denotational run**: one pool per member, the `Values` carrier verbatim.
- `lag` is the wire's **lag**, a natural number saying how far below the horizon the coupling is established. Most wires have lag `0`; [Lag](#lag) explains the rest.
- `wf` is the wire's **residual**, a proposition the coupling is conditional on. It is `True` at program inputs, and every operator conjoins its inputs' residuals; only two kinds of operator add to it (conditional folds and knots, below).
- `cpl` is the **coupling**: under `wf`, at every step `k` with `k + lag ≤ T`, member `i`'s machine view at `k` sits below its denotation pool in the grade's order.

`T` is instantiated with `Tc`, the **couple horizon**: the last step the coupling speaks about. The grade's order is `ListLe`, one relation with four faces, matching the four quotients a pool can be:

```lean
@[reducible] def ListLe {α : Type} [DecidableEq α] :
    (ord : StrOrd) → (ret : Retries) → List α →
      PoolCarrier α ord ret → Prop
  | .totalOrder, .exactlyOnce => fun s v => s <+: v
  | .totalOrder, .atLeastOnce => fun s v => StutterSeq.le (.mk s) v
  | .noOrder, .exactlyOnce => fun s v => (↑s : Multiset α) ≤ v
  | .noOrder, .atLeastOnce => fun s v => RetryPool.le (.mk ↑s) v
```

(`Hydro/Transfer.lean`.) `Values` holds a class of histories ([Grades](../semantics/grades.md)); the machine holds one concrete list per member at every step. `ListLe` reads the machine's list as a representative of the grade's quotient and says it is below the pool: the machine's history so far is the beginning of one history in the class. A totally ordered exactly-once view is a prefix of its pool, an unordered one's multiset is a sub-multiset, and the retry grades compare after destuttering or by support.

The other corners follow the same pattern with the carrier's own order: `CoKStream` couples a keyed stream per key (`KeyedListLe`); `CoTickSing` and `CoAtomicStream` couple tick traces by prefix; `CoSing` and `CoKSing` couple a fold's live value by **read agreement** (there is one read function `rd` with `sr i t = rd i ((src i).view t)`, and the `Values` reads, at the decisions the corner computes, are `rd` mapped over the machine's views). Tick-trace corners carry one extra field, `mono : wf → TMono sr`, explained under [Ticks](#ticks).

### Decision Types

The instance's decision types say what the corner takes from outside and what it computes itself:

```lean
def CoupleSem (L : Type) (mem : L → Nat)
    (pacing : Pacing L mem)
    (Tc Td : Nat) (hjT : Tc ≤ Td) : HydroSem L mem where
  Stream ℓ α _ ord ret := CoStream Tc (mem ℓ) α ord ret
  …
  TransportDec p c := Fin p → Fin c → Nat → Nat
  OrderSelDec _ _ := Unit
  TickOrderSelDec _ _ := Unit
  SnapDec _ _ _ := Unit
  BatchDec _ _ _ _ := Unit
  -- timing: the machine's per-tick scripts; the denotation's lists are derived from them at
  -- the horizon (`scriptDerive`)
  IntervalDec n := TickScript n Nat
  TicksDec _ := Unit
  ClockDec n := TickScript n Nat
  FixDec := Unit
```

A **content decision** chooses what data an operator sees: which elements a `batch` puts in each tick (its **cut**), which order `assume_ordering` presents, which prefix a `snapshot` reads. At the corner every content decision is `Unit`: the corner accepts no choice and computes one. Network cursors (`TransportDec`, how far each channel has delivered) and timing scripts (`IntervalDec`, `ClockDec`) are machine data, passed through to the machine run. A program run at the corner is therefore given exactly the machine's inputs, and produces the denotation's decisions as a by-product.

## Derived Decisions

The machine does not choose its batches or its orders: every stream there is a flat sequence, a tick's batch is whatever has arrived, and `assume_ordering` is an identity operator ([The Step Machine](../semantics/sched.md)). The corner therefore cannot ask the machine which decision it took. It reads the decision off the machine's run instead, and this reading holds the denotational run to what a run close to production does. Each content operator's denotational run is defined as the `Values` operator at a decision **derived** from the operator's own machine run. For an unordered exactly-once `batch` the derived cut consists of the machine's tick segments, read off the machine history at the tick steps the pacing fixes:

```lean
def batchDerive {n : Nat} {α : Type} [DecidableEq α]
    (p : Fin n → Nat → Bool)
    (T : Nat) (s : Fin n → StepHist α) : BatchCuts n α :=
  fun i => (batchesFrom ((s i).view) (tickSteps (p i) T) 0).map
    (fun b => Multiset.ofList b)
```

(`Hydro/Transfer.lean`.) A derived decision is **exact** at its own horizon: the `Values` batch run at the derived cut returns the machine's segments unchanged. It is also **monotone** in the horizon: the cut derived at `T` is a prefix of the cut derived at any `T' ≥ T`.

```lean
theorem corr_batch {α : Type} [DecidableEq α] {n : Nat}
    (p : Fin n → Nat → Bool) (T : Nat) (h : Fin n → StepHist α)
    (v : Fin n → Multiset α)
    (hc : ∀ i, Multiset.ofList ((h i).view T) ≤ v i) :
    ∀ i, batchCuts (v i) 0
        ((batchesFrom ((h i).view) (tickSteps (p i) T) 0).map
          (fun b => Multiset.ofList b))
      = (batchesFrom ((h i).view) (tickSteps (p i) T) 0).map
          (fun b => Multiset.ofList b)

theorem batchDerive_mono {n : Nat} {α : Type} [DecidableEq α]
    (p : Fin n → Nat → Bool) {T T' : Nat} (h : T ≤ T')
    (s : Fin n → StepHist α)
    (i : Fin n) : batchDerive p T s i <+: batchDerive p T' s i
```

Exactness relies on the total decision space. `Values` makes every cut legal by truncation: `batchCuts` admits a batch only if it fits in what remains of the pool and returns the run cut short at the first batch that does not ([Decisions as Cuts](../semantics/values.md#decisions-as-cuts)). `corr_batch` says that a machine run whose view is below the pool is never truncated: each segment fits, because the machine only batches what it received. The hypothesis `hc` is the input's own coupling.

The corner's `batch` puts the two facts together:

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
            show (((batchesFrom ((s.sr i).view) (tickSteps (pacing.ticks ℓ i) k)
                0).map List.toArray).map Array.toList).map (fun b => Multiset.ofList b)
              <+: batchCuts (s.rr i) 0 (batchDerive (pacing.ticks ℓ) Td s.sr i)
            rw [map_toList_map_toArray]
            have hreal : batchCuts (s.rr i) 0
                (batchDerive (pacing.ticks ℓ) k s.sr i)
                = batchDerive (pacing.ticks ℓ) k s.sr i :=
              corr_batch (pacing.ticks ℓ) k s.sr (fun j => s.rr j)
                (fun j => s.cpl hwf k hk j) i
            have hmono := batchCuts_mono_dec (pool := s.rr i)
              (c := 0) (batchDerive_mono (pacing.ticks ℓ) (Nat.le_trans (by omega) hjT : k ≤ Td) s.sr i)
            rw [hreal] at hmono
            exact hmono
          mono := fun _ => tmono_batch (pacing := pacing) (ord := .noOrder) (ret := .exactlyOnce) s.sr () }
      …
```

(`Hydro/Couple.lean`; the other three grades follow with `batchRetryDerive`, `batchOrdDerive` and `batchStutterDerive`.) The machine run is defined as the `SchedSem` batch: at step `t`, the segments cut at every tick up to `t`. The denotational run is defined as the `Values` batch at the cut derived at `Td`. The coupling at step `k` reads:

- `show`: the machine's segments at `k`, as multisets, are a prefix of the `Values` tick trace.
- `hreal`: at the cut derived at `k`, `Values` returns the machine's segments at `k` (`corr_batch`, with the input's coupling at `k` as the legality hypothesis).
- `hmono`: the cut derived at `k` is a prefix of the cut derived at `Td`, and `batchCuts` is monotone in its cut, so the run at `k`'s cut is a prefix of the run at `Td`'s.

The decision is derived at `Td` and not at `k` because `rr` is one value. It cannot depend on the step being asked about, so it is one decision for every step `k ≤ Tc`, and monotonicity in the horizon makes it serve all of them.

`Td`, the **derive horizon**, is separate from `Tc` for knots. A knot's proof instantiates its loop body at couple horizons below `Tc` (see [Knots](#knots)). If the decisions were derived at `Tc`, every lowered instance would compute a different denotational run, and the per-horizon couplings would be about different `Values` runs. With `Td` fixed, the denotational run does not depend on `Tc` at all, and lowering `Tc` weakens only what `cpl` promises. A program's own run takes `Tc = Td = T`.

The other derived decisions follow the same pattern, each exact at its own horizon and monotone in it:

| operator | derived decision | exactness |
|---|---|---|
| `batch` (4 grades) | `batchDerive`, `batchRetryDerive`, `batchOrdDerive`, `batchStutterDerive` | `corr_batch`, `retryBatchCuts_real`, `corr_batch_ordered`, `corr_batch_stutter` |
| `snapshot`, `ksnapshot` | `snapDerive` (the machine's fold reads at its tick steps) | `sched_snap_eq'` |
| `assume_ordering`, `kassume_ordering` | `ordSelDerive` (the machine's arrival order at `Td`) | `selectOrder_prefix_ext` |
| `tassume_ordering` | `tkOrdSelDerive` (each tick's arrival order at `Td`) | `tassume_cpl` |
| `time` | `scriptDerive` (the clock script's first `ticks-by-T` entries) | `scriptTrace_prefix` |

A program's derived decisions are collected into one `Values` decision record per module, `M_vdec`, a function of only the machine runs of the module's inputs and the module's machine data. The generator states that such a record exists and defines `M_vdec` by `Classical.choose` ([The Couple Phase](../generator/couple-phase.md)).

## Per-Operator Couplings

Every operator's `cpl` field is a lemma about that operator alone.

**Program inputs** are where the coupling enters. An input is a machine history paired with a pool it stays below at every step; its residual is `True`:

```lean
def input (h : Fin n → StepHist α) (v : MemberMap n (PoolCarrier α ord ret))
    (hc : ∀ t i, ListLe ord ret ((h i).view t) (v i)) :
    CoStream T n α ord ret where
  sr := h
  rr := v
  lag := 0
  wf := True
  cpl := fun _ k _hk i => hc k i
```

**Sources** carry no input. The corner defines the interval timer's machine run as the fire script's history and its denotational run as the single stutter class `[()]`, and every finite run of `()`s is below it at every step and every script:

```lean
    source_interval {ℓ} delay period fires :=
      { sr := (SchedSem L mem pacing).source_interval (ℓ := ℓ) delay period fires
        rr := (Values L mem).source_interval (ℓ := ℓ) delay period ()
        lag := 0
        wf := True
        cpl := fun _ _k _hk i => stutter_le_unit _ }
```

**Pointwise operators** transport the coupling with one preservation lemma and carry the lag and the residual through unchanged:

```lean
    map s f :=
      { sr := fun i => (s.sr i).map (f i)
        rr := (Values L mem).map s.rr f
        lag := s.lag
        wf := s.wf
        cpl := fun hwf k hk i => ListLe.map_eo (f i) (s.cpl hwf k hk i) }
```

The signature states `map` at `ExactlyOnce` only, so one preservation lemma (`ListLe.map_eo`, at both orderings) covers it. Binary operators (`union`, `zipTick`) take the larger lag and the conjunction of residuals.

**Conditional folds** are the first operators that add to the residual. A fold over an unordered stream is deterministic only if its closure commutes on the pairs the stream can present together; the program states that condition as a relation `Pre`. At the corner, the fold's `wf` records that the denotational run's content is pairwise `Pre`:

```lean
    fold {_ℓ _α _σ _ ord ret} g init Pre _ ok s :=
      { sr := fun i => schedFold g init (s.sr i)
        rr := (Values L mem).fold g init Pre ok s.rr
        src := s.sr
        lag := s.lag
        wf := s.wf ∧ ∀ i, PoolPairwise ord ret Pre (s.rr i)
        …
```

The program discharges that conjunct with a `ghost pre` fact about its own wires ([Contracts](../../using/programs/contracts.md)); the generator routes the fact to the conjunct when it proves the module's `wf`.

## Lag

### The Network Hop

At `Values` the hop is an identity operator: `Values.broadcast_closed` re-keys the senders' pools by sender (`KeyedPool.transport`), its transport decision is `Unit`, and the rewrite rules that say so are `rfl` ([Values](../semantics/values.md#the-network-as-identity)). On the machine it is a delivery: at step `k` the receiver sees a cursor-chosen prefix of what the sender held at step `k - 1`, and nothing at step `0` ([The Step Machine](../semantics/sched.md#delivery-and-the-1-floor)). The corner holds both, so its hop carries the pools through unchanged and the machine histories through delivery:

```lean
    broadcast_closed {_c p _α _ _ord _ret} dt s :=
      { sr := fun i => mergeN (fun j => ((s.sr j).deliver (pacing.active p i) (dt i j)).map (fun v => (j, v)))
        rr := (Values L mem).broadcast_closed () s.rr
        lag := s.lag - 1
        wf := s.wf
        cpl := fun hwf k hk i =>
          corr_transport (fun j => deliver_view_cpl_pred (s.sr j) (pacing.active p i) (dt i j) k
            (fun _h1 => s.cpl hwf (k - 1) (by omega) j)) }
```

The delivery cursors `dt` are machine data, and the corner passes them to the machine run only; the denotational run receives `()`. The coupling follows from the delivery lemma, applied per sender and assembled by `corr_transport`:

```lean
theorem deliver_view_cpl_pred {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (x : StepHist α) (p : Nat → Bool) (cur : Nat → Nat)
    {v : PoolCarrier α ord ret} :
    ∀ k, (1 ≤ k → ListLe ord ret (x.view (k - 1)) v) →
      ListLe ord ret ((x.deliver p cur).view k) v
```

(`Hydro/Couple.lean`.) Whatever the cursor does and whether the receiver is active, the delivered view at `k` is a prefix of the sender's view at `k - 1`, so it is below every pool the sender's view at `k - 1` is below. Every behaviour a cursor can produce (latency, stalls, silence) is covered by this one lemma.

The output's coupling at `k` therefore needs the input's only at `k - 1`. If the input is coupled at the steps `k' + lag ≤ T`, the output is coupled at the steps `k + (lag - 1) ≤ T`, because for `k ≥ 1`, `k - 1` is among the input's coupled steps. The field `lag := s.lag - 1` records this shift: the hop costs the machine one step, and the corner records that step as the output being coupled one step further than its input. An operator whose output at a step reads its input only at earlier steps is **strict** in that input. `defer_tick` is strict in the same way, since it releases at a tick what its input held one step back, and its lag is `t.lag - 1`. `Nat` subtraction truncates, so a hop on a lag-`0` wire has lag `0`. This truncation is sound, because the input is then coupled at every step `≤ T`, including `k - 1`.

A per-program safety proof reads only the denotational run, so it never mentions delivery: the machine's network behaviour is bounded by the pool once, in `deliver_view_cpl_pred`, and the step it costs is accounted for by the lag.

### Lag in Knots

Lag exists for knots. Inside a knot's proof, the induction hypothesis (below) couples the loop input only strictly below the step under proof, so the corner runs the body with the loop input at lag `1`. The rules then compute the lag of the body's output:

| wire | lag |
|---|---|
| a program input, a source | `0` |
| a knot's loop input | `1` when the knot's lag is computed; `L + 1` in the graded coupling at corner `j + L` (below) |
| a pointwise operator | its input's |
| a binary operator | the `max` of its inputs' |
| a network hop, `defer_tick` | its input's, minus one |
| a knot's output | its body's output lag when the loop input carries lag `1` (`fixLag`) |

If every path from the loop input to the body's output passes a hop or a `defer_tick`, the output's lag is determined by the knot's other inputs alone, and the knot's output is coupled at the horizon. This is Rust's rule for `forward_ref`: a cycle has no synchronous path from the wire back to itself. For the toy knot of `Hydro/HydroGenToy.lean`, whose body ships its wire through `broadcast_closed`, the generated lag equation is

```lean
@toy_loop.w_co_lag₁ : ∀ … (inp : HydroSem.Stream mem ℓ ℕ StrOrd.noOrder Retries.exactlyOnce) …,
  (toy_loop.w (CoupleSem L mem pacing Tc Td hjT) ℓ inp tdec sched).lag = inp.lag
```

(shortened from `#check`). A cycle with no strict operator on it is rejected by the generator's lag analysis of the program graph while the `fix` elaborates; for `fix w := union w inp` the message is `HydroGen: a lag-2 agreement cannot be read at lag 1` ([The Knot Stack](../generator/knot-stack.md#strictness)).

The knot's lag is computed by running the body on a **lag probe**, a carrier whose only meaningful field is its lag (`lagProbe x 1`; an operator's lag reads only its inputs' lags):

```lean
@[irreducible] def fixLag : Nat :=
  (body (lagProbe (fixSr body) 1)).lag
```

`fixSr body` is the knot's machine run, defined under [Knots](#knots). `fixLag` is `@[irreducible]` because it sits inside horizon bounds `j + lag ≤ T`, and `Nat` definitional unfolding at any of those comparisons would evaluate the whole knot body. The generator states every lag in a lag equation (`M_co_lagᵢ`) and normalizes the arithmetic; see [Projections and Tactics](./projections-and-tactics.md#lag-expressions).

## Knots

A knot `fix d body` closes `body` over its own output. At the corner it has to produce a machine run equal to the machine's knot, a denotational run equal to the `Values` knot at a derived fuel, and the coupling between them. The two semantics close the loop differently:

- `SchedSem.fix_stream` is defined as the **Kleene diagonal**: the knot wire at step `t` is the `(t + 1)`-th iterate of the body at step `t`. It is a fixpoint when the body is strict in the wire, because then the iterate at `t + 1` agrees with every later iterate up to step `t` ([The Ungated Knot](../semantics/sched.md#the-ungated-knot)).
- `Values.fix_stream fuel body` is defined as the Kleene iterate `iterate body ⊥ fuel`. The **fuel**, the number of iterations, is the knot's decision (`FixDec`).

The corner runs the body on **probes**, carriers built for one purpose whose residual is `False`, so their `cpl` is trivially constructible and never used. `probe2 x v` pairs a machine history with a pool; `schedEmbed x` is the same with the empty pool, for running the body on a machine value alone; the lag probe was above:

```lean
def probe2 (x : Fin n → StepHist α) (v : MemberMap n (PoolCarrier α ord ret)) :
    CoStream T n α ord ret where
  sr := x
  rr := v
  lag := 0
  wf := False
  cpl := fun hwf => absurd hwf (by simp)

def sbody (x : Fin n → StepHist α) : Fin n → StepHist α :=
  (body (schedEmbed x)).sr

def fixSr : Fin n → StepHist α :=
  famHist (fun t i =>
    ((iterate (sbody body) (fun _ => StepHist.bot) (t + 1)) i).view t)

def vbody (v : MemberMap n (PoolCarrier α ord ret)) :
    MemberMap n (PoolCarrier α ord ret) :=
  (body (probe2 (fixSr body) v)).rr
```

(`Hydro/Couple.lean`, inside `namespace CoStream`.) `sbody` is the machine body: the corner body on a machine-only wire, machine run read back. `fixSr` is defined as the diagonal of `sbody`, by the same term that defines `SchedSem.fix_stream`. `vbody` is the denotation body, run on a probe whose machine run is the knot's own diagonal `fixSr body`. A `batch` inside the loop body derives its cut from its machine run, and in `vbody` that machine run is computed from the same diagonal the closed knot presents to the rest of the program. The site's decision is therefore spelled identically whether it is reached from inside the knot or from outside, and the whole program has one decision record. A probe with any other machine run would give each loop site a second, different decision.

The instance's knot:

```lean
    fix_stream {ℓ α _ ord ret} _df body :=
      { sr := CoStream.fixSr body
        rr := iterate (CoStream.vbody body)
          (.mk' fun _i => PoolBot ord ret) (Td + 1)
        lag := CoStream.fixLag body
        wf := SStrict (CoStream.sbody body)
            ∧ (∀ m i, PoolLe ord ret
                (iterate (CoStream.vbody body)
                  (.mk' fun _ => PoolBot ord ret) m i)
                (iterate (CoStream.vbody body)
                  (.mk' fun _ => PoolBot ord ret) (m + 1) i))
            -- the graded coupling, at the Kleene stage `j` (the only denotation `co_fix_cpl`
            -- probes, so a body may use what the stage chain proves of its loop input: the
            -- knot's `requires`), at the horizons `j + lag ≤ Tc`, with the loop input coupled
            -- strictly below `j`
            ∧ (∀ j, j + CoStream.fixLag body ≤ Tc →
                ∀ (x : Fin (mem ℓ) → StepHist α),
                (∀ t, t < j → ∀ i, ListLe ord ret ((x i).view t)
                  (iterate (CoStream.vbody body) (.mk' fun _ => PoolBot ord ret) j i)) →
                ∀ i, ListLe ord ret ((CoStream.sbody body x i).view j)
                  ((body (CoStream.probe2 x
                    (iterate (CoStream.vbody body) (.mk' fun _ => PoolBot ord ret) j))).rr i))
        cpl := fun hwf k hk i => by
          obtain ⟨hstrict, hchain, hcplj⟩ := hwf
          refine ListLe.trans_pool
            (co_fix_cpl body hstrict _ hchain hcplj k hk i) ?_
          refine pool_chain_glue (f := fun m =>
            iterate (CoStream.vbody body)
              (.mk' fun _ => PoolBot ord ret) m i)
            (a := k + 1) (b := Td + 1) (by omega) ?_
          intro m _hm _hmb
          exact hchain m i }
```

The denotational run is Kleene stage `Td + 1`, and the `Values` program's fuel is set to the same `Td + 1` by the derived decision record. The knot does not need to converge: the transfer needs the machine's view at `k ≤ Tc` below the `Values` run, and the contract holds at every fuel because the fuel is a decision.

### The Knot's Residual

The knot's `wf` is a triple. Apart from the conditional folds' pairwise facts, it is the only residual obligation that does not come from the inputs:

1. `SStrict (sbody body)`, **strictness**: two loop wires that agree strictly below step `h` give machine body outputs that agree up to `h`. This makes the diagonal a fixpoint of the body.
2. **Ascent**: the denotation body's Kleene chain ascends, stage `m` below stage `m + 1`.
3. **The graded coupling**: at every horizon `j` with `j + lag ≤ Tc`, if a loop input `x` is coupled with Kleene stage `j` at every step `t < j`, then the body's machine output at `j` is coupled with the body's denotation output at that stage.

`co_fix_cpl` turns the triple into the knot's coupling by strong induction on the horizon:

```lean
theorem co_fix_cpl (L : Nat)
    (hchain : ∀ m i, PoolLe ord ret
    …
    (hcplj : ∀ j, j + L ≤ T → ∀ (x : Fin n → StepHist α),
    …
    ∀ j, j + L ≤ T → ∀ i, ListLe ord ret ((CoStream.fixSr body i).view j)
      (iterate (CoStream.vbody body)
        (.mk' fun _ => PoolBot ord ret) (j + 1) i) := by
  intro j
  induction j using Nat.strongRecOn with
  | _ j ih =>
  intro hj i
  rw [co_fix_eq body hstrict j i]
  refine hcplj j hj _ (fun t ht i' => ?_) i
  refine ListLe.trans_pool (ih t ht (by omega) i') ?_
  refine pool_chain_glue (f := fun m =>
    iterate (CoStream.vbody body) (.mk' fun _ => PoolBot ord ret) m i')
    (a := t + 1) (b := j) (by omega) ?_
  intro m _hm _hmb
  exact hchain m i'
```

(`Hydro/Couple.lean`, under `include hstrict`; the elided hypotheses are the second and third conjuncts of the knot's `wf` above, with the lag as a parameter `L`.) The claim at horizon `j` is that the diagonal's view at `j` is below stage `j + 1`.

- `rw [co_fix_eq …]` replaces the diagonal's view at `j` with the body's view at `j` applied to the closed diagonal. `co_fix_eq` is the machine fixpoint equation, and it takes strictness `hstrict` as an argument.
- `hcplj j hj _` reduces the goal to the graded coupling's hypothesis, with `x` the diagonal itself: the diagonal must be coupled with stage `j` at every `t < j`.
- `ih t ht` gives the diagonal at `t` below stage `t + 1`, and `pool_chain_glue` with `hchain` lifts stage `t + 1` to stage `j` (ascent, used `j - t - 1` times).

Back in the instance's `cpl`, the result at `k` (below stage `k + 1`) is lifted to stage `Td + 1` by the same gluing.

Strictness and ascent are facts about the body's machine causality and the body's `Values` monotonicity. The graded coupling needs no induction over the body's operators, because knot bodies are written generic over the instance (`HydroSem.fix` takes `body : ∀ H', Γ H' → H'.Stream … → H'.Stream …`). The generator instantiates the same body at the lowered corner `CoupleSem L mem pacing (j + L) Td _`, with the loop input built by `mkC` at lag `L + 1`:

```lean
def CoStream.mkC {α : Type} [DecidableEq α] {ord : StrOrd}
    {ret : Retries} (x : (SchedSem L mem pacing).Stream ℓ α ord ret)
    (v : (Values L mem).Stream ℓ α ord ret) (lag : Nat)
    (hc : ∀ k, k + lag ≤ Tc → ∀ i, ListLe ord ret ((x i).view k) (v i)) :
    (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α ord ret :=
  { sr := x, rr := v, lag := lag, wf := True, cpl := fun _ => hc }
```

(`Hydro/CoupleProj.lean`.) At corner `j + L`, a wire of lag `L + 1` is coupled at the steps `k + L + 1 ≤ j + L`, that is `k < j`. This condition is the hypothesis of the graded coupling. The body's own output at that corner has its own `cpl` field, read at step `j`, and that field yields the conclusion once the body's generated `sr` and `rr` equations identify its machine run with `sbody x` and its denotational run with the body at the probe. The induction over machine prefixes is carried by the instance; no tactic walks the body. The assembly of the triple (`K_co_wf₁`, from the facts `FX`, `FC`, `HVA`, `FM`, `FH`, `FOUT`) is on [The Knot Stack](../generator/knot-stack.md).

The tick knot (`fix_tick`, the `a_log` shape) follows the same construction over tick traces, with a fourth conjunct: the machine body preserves step-monotonicity, so the diagonal is step-monotone (`co_tick_fix_mono`). Its strong induction `co_tick_fix_cpl` glues with `trace_chain_glue`, since tick entries are not views of one growing history. The tick diagonal sits behind `stepGate`, an identity function written as a case split on the step so that the kernel does not evaluate the knot's body at a symbolic step ([The Step Machine](../semantics/sched.md#stepgate)).

## Ticks

A `tick` block compiles to one former, `tick_scan sts ins outs x g init`: a **register** (the state carried from tick to tick) of shape `sts`, inputs of shape `ins` and emissions of shape `outs`, where a **shape** (`TickShape`) is a tree of `sing`, `stream`, `ksing`, `kstream` leaves and `pair`s, and `g` is the per-tick body ([Values](../semantics/values.md#tick-shapes)). At the corner `g` is one function over coupled bounded tuples: inside a tick, a stream is `CoBounded`, a concrete batch and its denotation content with a coupling,

```lean
@[reducible] def CoupledBatch {α : Type} [DecidableEq α] :
    (ord : StrOrd) → (ret : Retries) → List α →
      PoolCarrier α ord ret → Prop
  | .totalOrder, .exactlyOnce => fun s v => s = v
  | .noOrder, .exactlyOnce => fun s v => (↑s : Multiset α) = v
  -- the retry grades: the machine batch is the denotation's class
  | .totalOrder, .atLeastOnce => fun s v => StutterSeq.mk s = v
  | .noOrder, .atLeastOnce => fun s v => RetryPool.mk (Multiset.ofList s) = v
```

and a plain value is `CoBSing`, two values with `cpl : wf → sr = rr`. Within one tick the coupling is equality, not `≤`: a tick's batch is the consumed increment in both `sr` and `rr`.

The corner needs the machine former applied to a machine-only body, and the `Values` former applied to a `Values`-only body. It gets both from the one corner body `g` by feeding it inputs that embed one run (`embedSR` or `embedRR`), with a dummy as the other run and `wf := False`:

```lean
def gS (i : Fin (mem ℓ)) (s : BoundedOf SchedTick.BS SchedTick.BStr SchedTick.BKS SchedTick.BKStr sts)
    (inp : BoundedOf SchedTick.BS SchedTick.BStr SchedTick.BKS SchedTick.BKStr ins) :
    … :=
  let r := g i (embedSR sts s) (embedSR ins inp)
  (projSR sts r.1, projSR outs r.2)
```

`gR` is the same with `embedRR`/`projRR`, and `gC` runs `g` on both runs at once, the machine value paired with its own coercion (`coerce` reads a machine value as a denotation value: an `Array` as its pool, an association list as its map), via `embed2 sh m (coerce sh m)`, whose `wf` is the leafwise coupling. Because each run of the output is the body on an input that embeds that run, the `.sr` and `.rr` projections of the corner's `tick_scan` reduce definitionally: for every projection path `π` into the output tuple, `(π (tick_scan … x g init)).sr` reduces to `π (SchedSem.tick_scan … (srLegs ins x) (gS … g) init)`, and `.rr` reduces to the `Values` former over `rrLegs` and `gR`. No single lemma states this for every path, so the generator reaches it through the `coTickScanProj` simproc, which computes the path and emits the right-hand side as a definitional step ([Projections and Tactics](./projections-and-tactics.md#simprocs)). Three conditions on the body tie the three runs together:

```lean
/-- W1: the machine legs are leg-independent, for the register and the emissions. -/
def W1 : Prop := ∀ i s inp,
  (gS sts ins outs g i s inp).1 = projSR sts (gC sts ins outs g i s inp).1
    ∧ (gS sts ins outs g i s inp).2 = projSR outs (gC sts ins outs g i s inp).2
```

W1 says the body's machine output does not read the denotational run. `W2` states the converse (the denotation body on coerced inputs equals the `rr` of the coupled run), and `W3` says that the coupled run's outputs are coupled (`WfB`) when its inputs are. Every in-tick operator at the corner acts on each run separately (`bmap` maps both runs, `bcount` counts both), so W1 and W2 hold by rewriting with the per-operator projection lemmas (`co_simp`), and W3 by the residual normalizer (`co_wf_simp`); both are on [Projections and Tactics](./projections-and-tactics.md). From the three, `body_comm` proves that the machine body, coerced, equals the denotation body on coerced inputs, `scan_comm` lifts that to the whole scan across ticks, and `scan_cpl` gives the leafwise coupling of the two formers. The residual of the former, `scanWf`, collects these conditions:

```lean
def scanWf {ℓ : L} (sts ins outs : TickShape)
    (x : AtomicOf (Tk Tc mem) (TS Tc mem) (TKS Tc mem) (TKStr Tc mem) ℓ ins)
    (g : …) : Prop :=
  Wf ins x ∧ allEO sts = true ∧ allEO ins = true ∧ allEO outs = true
    ∧ W1 sts ins outs g ∧ W2 sts ins outs g ∧ W3 sts ins outs g
```

`allEO` requires every keyed-stream leaf to be `ExactlyOnce`. A plain stream leaf at a retry grade is coupled by its class (`CoupledBatch`'s last two faces), but a keyed batch at a retry grade has no per-tick content equality, and no operator produces one.

For plain-typed in-tick results, the grade constraints of Hydro's API are conditions of provability. The signature defines `bcount` at `ExactlyOnce` only ("retries would be counted"): at `AtLeastOnce` the machine's count depends on how many duplicates the run delivered, which the class does not determine, so the coupling `sr = rr` would be false. `bfold` and `breduce` add `ret = .exactlyOnce` and the pairwise `Pre` fact to their residual for the same reason the wire-level fold does.

Tick-trace wire carriers have the extra field `mono : wf → TMono sr`, where `TMono x` says member `i`'s trace at step `k` is a prefix of its trace at any `k' ≥ k`. A stream history is one growing list, so its coupling at `Tc` restricts to every earlier step for free. A tick trace at step `k` is a separate value per `k`, and `tassume_ordering` derives each tick's order from the trace at `Td`. Its coupling at `k` needs the machine's tick at `k` to be a prefix of the one at `Td`, and `mono` supplies that fact. The generator proves it for every module (`M_smonoᵢ`, [The `wf`, `smono` and `mono` Phases](../generator/wf-smono-mono.md)).

## Machine Theorems

A consumer never unfolds a corner run. For every module, `hydro def` generates, per output, the `sr` equation, the `rr` equation and the residual lemma ([The Couple Phase](../generator/couple-phase.md)); for `toy_relay` (`Hydro/HydroGenToy.lean`, statements shortened from `#check`):

```lean
toy_relay_co_sr₁ : (toy_relay (CoupleSem L mem pacing Tc Td hjT) ℓ inp dec sched).1.sr =
    (toy_relay (SchedSem L mem pacing) ℓ inp.sr { batch := (), gate := () } { interval := sched.interval }).1
toy_relay_co_rr₁ : (toy_relay (CoupleSem L mem pacing Tc Td hjT) ℓ inp dec sched).1.rr =
    (toy_relay (Values L mem) ℓ inp.rr (toy_relay_vdec ℓ inp.sr sched.interval) { interval := () }).1
toy_relay_co_wf₁ : inp.4 → (toy_relay (CoupleSem L mem pacing Tc Td hjT) ℓ inp dec sched).1.wf
```

(`inp.4` is the input's `wf` field, printed positionally.) A fourth, the lag equation `toy_relay_co_lag₁`, states the output's lag.

The machine-run theorem is then four moves: discharge the residual, read `cpl` at the horizon, rewrite `sr` and `rr` with their equations, apply the `Values` contract at the derived decision. From `Hydro/HydroGenCheck.lean`:

```lean
  have hwf := toy_relay_co_wf₁ (Tc := T) (Td := T)
    (hjT := Nat.le_refl T) (pacing := pacing) ℓ
    (CoStream.inputC h v hc) ⟨CoDec.batch Nat, CoDec.batch Unit⟩ ⟨CoDec.interval fires⟩ trivial
  have hcpl := (toy_relay (CoupleSem L mem pacing T T (Nat.le_refl T))
    ℓ (CoStream.inputC h v hc) ⟨CoDec.batch Nat, CoDec.batch Unit⟩ ⟨CoDec.interval fires⟩).1.cpl hwf
    T (by co_lag [toy_relay_co_lag₁]) i
  have hsr := toy_relay_co_sr₁ …
  have hrr := toy_relay_co_rr₁ …
  rw [hsr, hrr] at hcpl
  have hens := toy_relay.ensures ℓ v
    (toy_relay_vdec (pacing := pacing) (Td := T) ℓ h fires) ⟨()⟩
  exact hens.pos i x (Multiset.mem_of_le hcpl (Multiset.mem_coe.mpr hx))
```

There is no decision argument and no satisfiability premise: the contract `ensures` is universally quantified over decisions, and the corner supplies the one it derived. The same four moves give `paxos_safe_sched`, whose only input-side premise is the checkpoint wire's own `Monotonic` typing (`CkMono`); the Paxos residual is closed by the generated `paxos_core_co_wf₂` with all five knots' triples discharged. The `Values` pool above each input that `CoStream.inputC` asks for is built in the proof from the input truncated at the horizon, so unbounded inputs are inside the quantifiers ([Transferring to the Machine and the Network](../../using/proving/transferring-to-the-machine.md)).

The corner over-approximates the machine. The machine realizes strictly fewer behaviours than the decision space licenses: it is per-pair FIFO, it publishes one order per schedule where `selectOrder` licenses every permutation, and it never performs a `RetryPool` re-read. There is no `∀ decision, ∃ schedule` theorem and there cannot be one. Safety transfers because it needs machine ⊆ denotation only; the surplus makes contracts harder to prove, never unsound. The generator defines each decision record `M_vdec` by `Classical.choose`, so choice enters the generated artifacts through `M_vdec`; they rest on Lean's three standard axioms ([Soundness](../soundness.md)).
