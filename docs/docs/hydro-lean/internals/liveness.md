---
sidebar_position: 9
---

# Liveness under Fairness

The safety theorems move a fact from the **denotation**, the denotational semantics `Values` in which a stream captures all the histories it could have had ([Deferred Nondeterminism](./approach.md#deferred-nondeterminism)), to the **step machine** `SchedSem`, the operational semantics that runs a program on a global step clock ([The Step Machine](./semantics/sched.md)). The **coupling corner** `CoupleSem` relates the two: it runs a program as a machine run `sr` and a denotational run `rr` at once ([The Coupling Corner](./corner/coupling.md)). The transfer is an inclusion: the machine's view at every step lies below the denotation's class ([Soundness](./soundness.md)). An inclusion cannot carry "eventually something is emitted", because the empty view lies below every class.

A liveness theorem therefore adds three things to the safety chain: a restriction to **fair** schedules, a way to say "eventually" without leaving the finite-horizon semantics, and an **equality** between what the machine emits and the denotation at the decision the run realized. With these, `cq_live` proves liveness of the quorum stage `collect_quorum`; the results above it are open ([Proven and Open Results](#proven-and-open-results)).

The page uses these terms:

- The machine's **schedule** consists of the **pacing**, which says per member and step whether the member is idle, receives without ticking, or ticks, and whose tick component per member is a **tick skeleton** `p : Nat → Bool` (`pacing.ticks ℓ i`); the per-receiver **delivery cursors**, which say how many messages have been delivered at each step; and the timer scripts.
- A **wire** is a collection-valued `let` or binder of the program, and a machine wire is its step history `StepHist`.
- A **cut** is a batch decision, the list of batches a stream is consumed in ([Nondeterminism](../using/programs/nondeterminism.md)), and a **derived decision** is the decision the corner computes from a machine run instead of taking it as an argument ([Derived Decisions](./corner/coupling.md#derived-decisions)).
- A **face** is a field of a module's contract ([Contracts](../using/programs/contracts.md)).

## Fairness Predicates

The machine is deterministic given its schedule. A TLA+ behavior is therefore a point of the schedule space, and weak fairness is a predicate on the arguments the safety theorems already quantify over (`hydro_lean/Hydro/Liveness.lean`):

```lean
/-- WF(tick): the skeleton ticks infinitely often. -/
def FairTicks (p : Nat → Bool) : Prop :=
  ∀ n, ∃ t, n ≤ t ∧ p t = true

/-- WF(deliver): the delivery cursor grows without bound, so every sent
message is eventually delivered. -/
def FairCursor (c : Nat → Nat) : Prop :=
  ∀ n, ∃ t, n ≤ cumMax c t
```

`FairCursor` is stated over `cumMax c`, the running maximum, because the machine reads a cursor that way: a cursor below an earlier one reads as the earlier one, so a delivered count never shrinks. Enabledness of delivery is therefore persistent, and weak fairness is enough. No theorem here needs strong fairness. Its client would be lossy links with retransmission, and the cursor model is fail-stop TCP, which loses nothing.

Safety stays quantified over every schedule. Nothing in the machine, the denotation or the safety artifacts changes.

The two per-operator facts at this level are analogues of TLA+'s WF1 rule. Both start from a wire that has settled: `StabilizesAt h T` (`Hydro/TransferTheory.lean`) says the wire's history is constant from step `T` on (`∀ t, h.view (T + t) = h.view T`). It is per wire, not a global quiescence predicate, so a program in which some wires keep churning still gets facts about the wires that settle. The statements (proofs elided):

```lean
theorem deliver_fair_attains {α : Type} {h : StepHist α} {T₀ : Nat}
    {p : Nat → Bool} {c : Nat → Nat} (hs : StabilizesAt h T₀) (hp : FairTicks p)
    (hc : FairCursor c) :
    ∃ T, ∀ t, T ≤ t → (h.deliver p c).view t = h.view T₀

theorem ticks_fair_attains {α : Type} (h : StepHist α) {T₀ : Nat}
    (hstab : StabilizesAt h T₀) {p : Nat → Bool} (hp : FairTicks p) :
    ∃ T, T₀ ≤ T ∧
      (batchesFrom h.view (tickSteps p T) 0).flatten = h.view T₀
```

`deliver_fair_attains` says a fair cursor at a fairly ticking receiver eventually delivers the whole settled view and keeps it. `ticks_fair_attains` rests on the fact that a tick consumes everything that has arrived: over an ascending run of tick steps the batches telescope to the view at the last tick (`batchesFrom_flatten`).

## Fairness outside the Semantics

Fair merge is not Scott-continuous: whether a behavior is fair is not determined by its finite prefixes. A fixpoint semantics with fairness in its domain is therefore not available. The semantics stays finite-horizon and fueled. Fairness is a predicate in the logic above it, and the conclusions have the shape "some finite point satisfies P", so a finite point is the witness. Only an ω-limit statement (progress under unbounded load) would need decisions over the completed infinite run.

## Behaviors as Decision Chains

A machine run up to the **horizon** `T` (its first `T` steps) determines a derived decision `d(T)`, and derivation is monotone in the horizon. An infinite behavior therefore determines an ascending chain of decisions, and the temporal operators are quantifier shapes over chain positions (`Hydro/LivenessChain.lean`):

```lean
def ChEventually (P : Nat → Prop) : Prop := ∃ n, P n
def ChAlways (P : Nat → Prop) : Prop := ∀ n, P n
def ChEvAlways (P : Nat → Prop) : Prop := ∃ n, ∀ m, n ≤ m → P m
def ChAlwEventually (P : Nat → Prop) : Prop := ∀ n, ∃ m, n ≤ m ∧ P m
def ChLeadsTo (P Q : Nat → Prop) : Prop := ∀ n, P n → ∃ m, n ≤ m ∧ Q m
```

At a batch site the chain is a chain of cuts of a **pool** `v` (the multiset the batches are taken from), and fairness of the tick action becomes a property of that chain, with no schedule vocabulary:

```lean
structure IsCutChain (v : Multiset α) (c : Nat → List (Multiset α)) :
    Prop where
  mono : ∀ n, c n <+: c (n + 1)
  legal : ∀ n, batchCuts v 0 (c n) = c n

def Exhausts (v : Multiset α) (c : Nat → List (Multiset α)) : Prop :=
  ChEventually fun n => cqConsumed v (c n) = v
```

`legal` uses the fact that `batchCuts` (the `Values` meaning of a batch cut, [Total Decision Spaces](../using/programs/nondeterminism.md#total-decision-spaces)) truncates a cut at its first illegal batch: a cut is legal exactly when truncation leaves it unchanged. `Exhausts` is WF(tick) at `Values`: the chain eventually consumes the whole pool. Along a legal ascending chain the consumed pool only grows (`IsCutChain.consumed_persists`), so exhaustion, once reached, persists. For upward-closed observations `◇` and `◇□` therefore coincide.

Two more predicates are defined for later rungs and have no client yet. `ChStabilizes f` (a chain component is eventually constant) states the premise shape for "eventually one leader". `ChSF Enabled Taken` is strong fairness between two trajectories.

## The Protocol Argument at `Values`

The point lemma instantiates the contract's `emit_count` face at a complete decision. In it, `cqConsumed v d` is the part of pool `v` the cuts `d` consume, `cqKeyCount v k` the number of responses for key `k` in `v`, and `cqOkCount v k` the number of `Ok` responses among them (`Hydro/Std/Quorum.lean`):

```lean
theorem cq_complete_mem (v : MemberMap 1 (Multiset (K × Except E Unit)))
    (d : (Values Unit (fun _ => 1)).BatchDec 1 (K × Except E Unit) .noOrder .exactlyOnce)
    (mn mx : Nat) (k : K) (h1 : 1 ≤ mn) (hmm : mn ≤ mx)
    (hcomplete : cqConsumed (v 0) (d 0) = v 0)
    (hcap : cqKeyCount (v 0) k ≤ mx)
    (hq : mn ≤ cqOkCount (v 0) k) :
    k ∈ (collect_quorum (Values Unit (fun _ => 1)) () v mn mx
      d).1 0 := by
  have hens := collect_quorum.ensures (L := Unit) (mem := fun _ => 1) () v mn mx
    d
  have hcount := hens.emit_count 0 k h1 hmm
    (by rw [hcomplete]; exact hcap)
  rw [hcomplete, if_pos hq] at hcount
  rw [← Multiset.count_pos, hcount]
  exact Nat.zero_lt_one
```

`h1`, `hmm` and `hcap` are quorum.rs's **usage contract**: `1 ≤ min ≤ max`, and no key receives more than `max` responses. Without `hcap` the statement is false. Once a key reaches its drop point (`max` responses, or `min` `Ok`s when `min = max`) its responses leave the `not_all` register (a `use::state` value carried across ticks), so further responses start a fresh count. `collect_quorum` with `(min, max) = (2, 3)` on the cuts `{7×3}, {7×2}` emits `{7, 7}`. `cq_chain_live` lifts the point lemma to any exhausting chain (`ChEventually`), and `cq_chain_live_stable` to `ChEvAlways` along a legal ascending one. Both touch only `CQEnsures`. The contract carries the quorum-crossing argument, which is not re-proved for the machine.

## Derived Chains of Fair Schedules

The derived cut chain of a machine wire under a tick skeleton `p` is defined as the realized batches up to each horizon:

```lean
def cqDerivedChain (hi : StepHist α) (p : Nat → Bool)
    (T : Nat) : List (Multiset α) :=
  (batchesFrom hi.view (tickSteps p T) 0).map
    (fun b => Multiset.ofList b)
```

Three facts, each generic in the wire:

- `cqDerivedChain_isCutChain`: given only stabilization, the chain is ascending and legal. No fairness is needed, because every machine behavior determines a chain. Legality is `corr_batch`, the exactness of the derived batch decision at its own horizon.
- `cqDerivedChain_complete_at`: at any tick at or after stabilization, the chain point consumes the whole stabilized pool.
- `cqDerivedChain_exhausts`: under `FairTicks p` the chain exhausts. `FairTicks` enters the development only here; `cq_live` below needs only the one tick that `cqDerivedChain_complete_at` consumes.

## Tightness

```lean
theorem collect_quorum_tight {pacing : Pacing Unit (fun _ => 1)}
    (h : Fin 1 → StepHist (K × Except E Unit)) {T₀ : Nat}
    (hstab : StabilizesAt (h 0) T₀) (mn mx : Nat) (T : Nat) :
    Multiset.ofList (((collect_quorum (SchedSem Unit (fun _ => 1) pacing) ()
        h mn mx ()).1 0).view T)
      = (collect_quorum (Values Unit (fun _ => 1)) ()
          (.mk' fun _ => Multiset.ofList ((h 0).view T₀)) mn mx
          (fun i => cqDerivedChain (h i) (pacing.ticks () i) T)).1 0
```

The machine output equals the `Values` run at the derived chain. The safety chain has no such equality, and no hand-written mirror of the module proves it. It comes entirely from artifacts that the generator (the code behind `hydro def`) emits for `collect_quorum` ([The Couple Phase](./generator/couple-phase.md)):

1. `collect_quorum_co_wf₁`, the corner's **residual** for the module (the obligation the corner leaves to it), unfolded by `change CoTick.scanWf _ _ _ _ _ at hwf`. For a `tick` block the residual conjoins the body's well-formedness and the independence of `sr` and `rr`.
2. The machine run `sr` equals the `SchedSem` run (`collect_quorum_co_sr₁`).
3. The denotational run `rr` equals the `Values` run at `cqDerivedChain` (`co_transfer [collect_quorum]`).
4. Under the body's well-formedness, the machine's tick former (`tick_scan`, what a `tick` block compiles to) coerced to multisets equals the `Values` tick former on the coerced batches (`CoTick.scan_comm`).

Step 3 does not use `collect_quorum_co_rr₁`, because the generator defines `_vdec` by `Classical.choose`, and that definition is not definitionally `cqDerivedChain`. The proof re-runs the projection walk (`co_transfer`) at the explicit decision. The decision must be spelled eta-expanded, `fun i => cqDerivedChain (h i) …`. The form `fun _ => … (h 0) …` is not definitionally equal under the binder.

## The Liveness Theorem

```lean
theorem cq_live {pacing : Pacing Unit (fun _ => 1)}
    (h : Fin 1 → StepHist (K × Except E Unit)) {T₀ : Nat}
    (hstab : StabilizesAt (h 0) T₀)
    (mn mx : Nat) (k : K)
    (h1 : 1 ≤ mn) (hmm : mn ≤ mx)
    (hcap : cqKeyCount (Multiset.ofList ((h 0).view T₀)) k ≤ mx)
    (hq : mn ≤ cqOkCount (Multiset.ofList ((h 0).view T₀)) k)
    (hsupply : ∃ Tw, T₀ ≤ Tw ∧ pacing.ticks () 0 Tw = true) :
    ∃ T, k ∈ ((collect_quorum (SchedSem Unit (fun _ => 1) pacing) ()
      h mn mx ()).1 0).view T := by
  obtain ⟨Tw, hT₀Tw, hpTw⟩ := hsupply
  refine ⟨Tw, ?_⟩
  have hval := cq_complete_mem
    (.mk' fun _ => Multiset.ofList ((h 0).view T₀))
    (fun i => cqDerivedChain (h i) (pacing.ticks () i) Tw) mn mx k h1 hmm
    (cqDerivedChain_complete_at hstab hT₀Tw hpTw) hcap hq
  rw [← collect_quorum_tight h hstab mn mx Tw] at hval
  exact Multiset.mem_coe.mp hval
```

The hypotheses, in order:

- `h` is any machine history of the one member's response wire, and `pacing` any pacing of the one location `Unit` with one member.
- `hstab`: the response wire settles at step `T₀`.
- `h1`, `hmm`: quorum.rs's usage contract on the parameters, `1 ≤ min ≤ max`.
- `hcap`: the other half of the usage contract, no key has more than `max` responses in the settled view.
- `hq`: the settled view holds at least `min` `Ok` responses for `k`.
- `hsupply`: the member ticks at least once at or after `T₀`.

`hsupply` is the only scheduling premise, and it is not `FairTicks`: one tick after stabilization suffices, and `FairTicks (pacing.ticks () 0)` gives it as `hp T₀`. The witness `T` is that tick. `collect_quorum` runs on one member at one location with no network edge, so `FairCursor` has no role here; it becomes relevant for a stage whose input crosses a hop, through `deliver_fair_attains`. `cq_live_chain` states the same fact under the chain-factored name.

The premises are jointly inhabited on a skeleton that is fair but not the always-tick one:

```lean
private def wOdd : Unit → Fin 1 → Nat → Bool := fun _ _ t => t % 2 == 1
…
example : ∃ T, 7 ∈ ((collect_quorum
    (SchedSem Unit (fun _ => 1) (Pacing.ofTicks wOdd)) () wResp 2 2 ()).1 0).view T :=
  cq_live (T₀ := 0) wResp (fun _ => rfl) 2 2 7
    (by decide) (by decide) (by decide) (by decide)
    ⟨1, Nat.zero_le _, by decide⟩
```

`#guard`s beside it check that the wire is empty at step 0 and holds `[7]` from step 1, and that the derived chain consumes nothing at horizon 0 and everything at horizon 1.

## Limits

There is no theorem of the form "for every decision there is a schedule that realizes it", and there cannot be one. The machine realizes strictly fewer behaviors than the denotational decision space admits:

- `flattenUnordered` publishes one program-determined order per schedule, where `selectOrder` admits every permutation.
- The machine is FIFO per sender and receiver pair.
- The machine never performs the re-reads an at-least-once pool admits.

Safety needs only machine ⊆ denotation. The surplus makes a program's obligations stronger than necessary, which costs conservativity and never soundness. For liveness the surplus is irrelevant, because the conclusion is about the machine run, and tightness is an equality at the run's own derived point.

## Proven and Open Results

| rung | statement | status |
|---|---|---|
| quiescent completeness | the input stabilizes and the member ticks after it, so every consumption site exhausts and the machine attains the total denotation | proven for `collect_quorum` (`cq_live`) |
| multi-stage completeness | `two_pc`: a unanimously voted, once-proposed payload is eventually committed. The ack stage's pool is an internal wire, so the statement is pool-free and from fairness alone | open: `TPCEnsures` has no completeness face (the `Examples/TwoPC.lean` header assigns the direction to the chain layer), and no theorem states it |
| commit under leader stability | fairness plus `ChStabilizes` of the election component plus a proposed value gives `ChEventually committed` | open; the vocabulary exists |
| lossy links | `ChSF` premises | open; needs a cursor model that skips, which no mirrored program uses |
| liveness under churn | progress with unbounded load | open; the only rung that needs an ω-limit `Values` |
