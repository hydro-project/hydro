---
sidebar_position: 2
title: How Proofs Are Generated
---

# How Proofs Are Generated

You prove a contract about a module's denotation. Carrying it to the step machine and on to the compiled processes needs about a dozen further facts about that module: that its coupled run's machine side is its `SchedSem` run, that its denotation side is its `Values` run at a decision computed from the machine run, the lag and the residual of each output, step monotonicity, monotonicity, and that its compiled graph means what the module means. None of them can be a library theorem over all programs, because a program is a Lean function generic over its instance and has no syntax to induct over ([The Approach](./approach.md#per-program-proof-generation)). `hydro def` therefore states them over each module's own signature and proves them on the spot, so a user never writes or repairs one.

The page uses the vocabulary of the [Architecture Overview](./index.md#vocabulary). The terms it leans on most:

- A **module** is one `hydro def`: a Lean function of its live collections (its **wires**) and its decisions, generic over an **instance** `H : HydroSem L mem`, the semantics it is run at.
- `Values` is the **denotation**, the denotational level, where a stream captures all its possible histories and you prove contracts. `SchedSem pacing` is the **step machine**, the operational semantics defined operator by operator on whole runs, with explicit delivery, queues and a **pacing** (which members run at each step). The third level is the compiled graph, run step by step and then as processes ([The Approach](./approach.md)).
- `CoupleSem`, the **corner**, runs the program at `Values` and `SchedSem` at once. Each of its wires has a **machine run** `sr` (the `SchedSem` value), a **denotational run** `rr` (the `Values` value), a **lag**, a **residual** proposition `wf`, and a proof that under `wf` the machine run stays below the denotational run up to a step bound, the **horizon** ([The Coupling Corner](./corner/coupling.md)). `MonoRel` is the instance at which running a program proves it monotone. Both are proof devices, not levels.
- A **decision** is an argument that resolves one nondeterministic choice, Rust's `nondet!` ([Nondeterminism and Decisions](../using/programs/nondeterminism.md)). A **knot** is a cycle closed by `fix`, Rust's `forward_ref`.

The obvious proof of each fact is definitional unfolding: unfold the module, unfold the operators, compare. It works for one small module and fails in four measured ways as programs grow. Definitional equality through nested knots is exponential in the nesting depth. `Meta.isDefEq` is up to 50 times slower than the kernel on graph terms. `simp` cannot rewrite a callee application that sits in a dependent argument position. A failing `apply` against a deep projection spine costs about 1.7 times more per level. The sections below follow one module, `toy_relay`, and then the knot module `toy_loop` through every phase, with the statement each phase emits and the script or proof term that proves it; the [last section](#cross-cutting-techniques) collects the techniques that avoid the four failures.

The statements below are `#check` output. The scripts are what `HYDROGEN_TAC_DIAG=1` prints while `Hydro/HydroGenToy.lean` elaborates; they are the generator's string templates with the module's names filled in. [The Generator Overview](./generator/overview.md) documents the order of the phases and the routing rules.

## The Example Module

`toy_relay` (`hydro_lean/Hydro/HydroGenToy.lean`) bumps each input, batches the result per tick, gates the batches on an interval timer, and leaves the tick:

```lean
hydro def toy_relay (H : HydroSem L mem) (ℓ : L)
    (inp : H.Stream ℓ Nat .noOrder .exactlyOnce)
    (dec : ToyRelayDec H (mem ℓ)) (sched : ToyRelaySched H (mem ℓ)) :
    (H.Stream ℓ Nat .noOrder .exactlyOnce
      × H.AtomicStream ℓ Nat .noOrder .exactlyOnce)
  ensures out => ToyEnsures out :=
  let bumped := H.map inp (fun _i n => n + 1)
  let b := H.batch bumped dec.batch
  let fires := H.source_interval (fun _ => 0) 1 sched.interval
  let gate := H.batch fires dec.gate
  tick (input bb := b) (input g := gate) :=
    yield (emitted := H.bfilterIf bb (H.boIsSome (H.bfirst g)));
  (H.allTicks emitted, emitted)
  prove
    pos := fun i x hx => by
      …
```

The record `dec` holds two **content** decisions, the two batches' **cuts** (where each tick's batch ends), which the denotation takes as data and the machine replaces by `Unit`. The record `sched` holds one **schedule** decision, the timer's fire script, which the machine takes as data and the denotation replaces by `Unit`. The command emits, for this module alone:

| artifact | says | proved by |
|---|---|---|
| `toy_relay`, `toy_relay.ensures` | the plain program; the contract at `Values` | derived from `toy_relay._spec` |
| `toy_relay_co_sr₁`, `_co_sr₂` | the corner's machine runs `sr` equal the `SchedSem` run | a direct term from the glue engine |
| `toy_relay_co_rr_ex`, `toy_relay_vdec`, `_co_rr₁`, `_co_rr₂` | the denotational runs `rr` equal the `Values` run at a decision derived from the machine | a script, then `Classical.choose` |
| `toy_relay_co_lag₁`, `_co_lag₂` | each output's lag | a direct term from the glue engine |
| `toy_relay_smono₁`, `_smono₂` | the machine outputs grow along steps | scripts |
| `toy_relay_co_wf₁`, `_co_wf₂` | the residual holds when the inputs' does | scripts |
| `toy_relay_mono₁`, `_mono₂` | the denotation is monotone in the inputs | a term: the module at `MonoRel` |
| `toy_relay.dag`, `.schedOf`, `.recordOf`, `.wireCodecs` | the compiled graph, its schedule in both directions, its byte codecs | definitions |
| `toy_relay.dag_denote`, `.schedOf_recordOf`, `.dag_valid`, `.dag_agreeL` | the graph means the module at `SchedSem`; the schedule round-trips; every cycle passes a network hop or a `defer_tick`; the graph's lag law | kernel judgements and direct terms |

## Elaboration and Routing

The declaration elaborates as `toy_relay._spec`, a function returning the result paired with its contract, and `deriveModule` splits it into the plain program `toy_relay` and the **face** `toy_relay.ensures`, the contract as a theorem about the `Values` run ([Elaborating `hydro def`](./generator/elaborating-hydro-def.md#the-contract-face)). The `tick` block becomes one application of `tick_scan`, a single signature operator whose arity is given by type-level shapes, to a step function ([The `tick` Elaborator](./generator/tick-elaborator.md)). From `#print toy_relay`, the lines that matter to the phases below:

```lean
  let emitted_r :=
    HydroSem.tick_scan (TickShape.sing Unit)
      ((TickShape.stream ℕ instDecidableEqNat StrOrd.noOrder Retries.exactlyOnce).pair
        (TickShape.stream Unit instDecidableEqPUnit StrOrd.totalOrder Retries.atLeastOnce))
      (TickShape.stream ℕ instDecidableEqNat StrOrd.noOrder Retries.exactlyOnce) (bb, g) emitted_step ();
  let emitted := emitted_r;
  (HydroSem.allTicks emitted, emitted)
```

Every phase sees `tick_scan` as one more operator with its own per-operator lemmas. The block's facts that the contract proof reads (what tick `n` emitted) are **ghost** facts, proof-only theorems hoisted out of the program, and play no part in the generated stack.

The **spec walk** then records which registered modules the body calls (its **callees**) and whether it ties a knot. The command logs:

```text
hydro def Hydro.toy_relay: callees [], inlines [], fix := false
```

No callee reaches a knot, so `toy_relay` takes the **couple route**: the generator states its corner run's `rr` equation as an existential and closes it by one definitional check of module size ([Routing](./generator/overview.md#routing)).

## The Machine Run

```lean
@toy_relay_co_sr₁ : ∀ {L : Type} {mem : L → ℕ} {pacing : Pacing L mem} {Tc Td : ℕ} {hjT : Tc.le Td} (ℓ : L)
  (inp : HydroSem.Stream mem ℓ ℕ StrOrd.noOrder Retries.exactlyOnce)
  (dec : ToyRelayDec (CoupleSem L mem pacing Tc Td hjT) (mem ℓ))
  (sched : ToyRelaySched (CoupleSem L mem pacing Tc Td hjT) (mem ℓ)),
  (toy_relay (CoupleSem L mem pacing Tc Td hjT) ℓ inp dec sched).1.sr =
    (toy_relay (SchedSem L mem pacing) ℓ inp.sr { batch := (), gate := () } { interval := sched.interval }).1
```

`Tc ≤ Td` are the corner's two horizons: `Tc` is the step up to which the coupling is claimed, `Td` the step up to which the machine history is read when a decision is derived. The right side passes the input's machine run `inp.sr`, `()` for each content decision, and the schedule unchanged.

Every operator has a projection lemma that pushes `sr` (and `rr`, `lag`, `wf`) through it, stated at the instance-projected carrier types and proved by `rfl` (`Hydro/CoupleProj.lean`):

```lean
theorem co_batch_sr {α : Type} [DecidableEq α]
    (s : (CoupleSem L mem pacing Tc Td hjT).Stream ℓ α .noOrder .exactlyOnce)
    (cutd : (CoupleSem L mem pacing Tc Td hjT).BatchDec (mem ℓ) α .noOrder .exactlyOnce) :
    ((CoupleSem L mem pacing Tc Td hjT).batch s cutd).sr
      = (SchedSem L mem pacing).batch (ℓ := ℓ) (ord := .noOrder) (ret := .exactlyOnce) s.sr () := rfl
```

An equation of this kind identifies `sr` or `rr` of the corner run with a run at another instance: `toy_relay_co_sr₁` is the module's `sr` equation, and `toy_relay_co_rr₁` below is its `rr` equation. The proof of the `sr` equation is not a script. The glue engine normalizes each side of the equation separately (one delta of the module, `simp` with the projection set and the callees' `sr`/`rr` equations, a keyed rewrite pass for the callee redexes `simp` cannot reach, repeated to a fixpoint) and requires the two normal forms to meet. The proof is the chain on one side composed with the reverse of the other, so the kernel checks only one-delta `rfl`s and congruence steps, none of module size ([The Glue Phase](./generator/glue-phase.md#the-corner-chain)). The glue engine proves the lag equation `toy_relay_co_lag₁ : (toy_relay (CoupleSem …) …).1.lag = inp.lag` the same way, on the lag projections.

## The Denotational Run

The denotational run `rr` equals the `Values` run at a decision record that the corner computes from the machine run `sr`. For a batch, `co_batch_rr` says `rr = (Values L mem).batch s.rr (batchDerive (pacing.ticks ℓ) Td s.sr)`: the machine's own tick segments, read back as multiset cuts. The generator does not construct the module's record from these pieces. It states that one exists, as a function of the machine data alone:

```lean
@toy_relay_co_rr_ex : ∀ {L : Type} {mem : L → ℕ} {pacing : Pacing L mem} {Td : ℕ} (ℓ : L)
  (x1 : HydroSem.Stream mem ℓ ℕ StrOrd.noOrder Retries.exactlyOnce) (d3_interval : HydroSem.IntervalDec mem (mem ℓ)),
  ∃ dV,
    ∀ (Tc' : ℕ) (h' : Tc'.le Td) (w1 : HydroSem.Stream mem ℓ ℕ StrOrd.noOrder Retries.exactlyOnce)
      (w2 : ToyRelayDec (CoupleSem L mem pacing Tc' Td h') (mem ℓ))
      (w3 : ToyRelaySched (CoupleSem L mem pacing Tc' Td h') (mem ℓ)),
      w1.sr = x1 →
        w3.interval = d3_interval →
          (toy_relay (CoupleSem L mem pacing Tc' Td h') ℓ w1 w2 w3).1.rr =
              (toy_relay (Values L mem) ℓ w1.rr dV { interval := () }).1 ∧
            (toy_relay (CoupleSem L mem pacing Tc' Td h') ℓ w1 w2 w3).2.rr =
              (toy_relay (Values L mem) ℓ w1.rr dV { interval := () }).2
```

The outer binders are the machine data of the arguments (`x1` the input's machine run, `d3_interval` the timer's script). The witness `dV` comes next, and only then the corner arguments, re-bound at an arbitrary couple horizon `Tc'` and pinned to the outer binders by the two equations. `dV` can therefore depend only on the machine data: not on the corner's own decision record, not on the couple horizon. This binder order states formally that the machine run determines the `Values` decision. Because of this order, a machine theorem over all runs carries no premise that some decision matches the run ([The Satisfiability Premise](./corner/coupling.md#the-satisfiability-premise)).

The script `genRrEx` assembles for it:

```text
(intros; apply Exists.intro ((⟨_, _⟩)); intro Tc' h' w1 w2 w3 hp_x1 hp_d3_interval; refine ⟨?_, ?_⟩ <;> ((try co_simp [Hydro.toy_relay]); (try rw [hp_x1]); (try simp only [hp_x1]); (try rw [hp_d3_interval]); (try simp only [hp_d3_interval]); (try exact rfl)))
```

1. `apply Exists.intro ((⟨_, _⟩))` commits to a witness made of holes, one per field of `ToyRelayDec`. A bare hole would not do: the goal constrains the record through projections (`?dV.batch`), and a projection of a bare metavariable is not unifiable, while a projection of a constructor of holes reduces to the field's hole.
2. `refine ⟨?_, ?_⟩ <;> …` splits the two outputs and runs the rest on each.
3. `co_simp [Hydro.toy_relay]` unfolds the module once and pushes `rr` through every operator, so the left side becomes the `Values` program at `batchDerive …`-shaped cuts over `w1.sr`, and the right side the same program at the holes.
4. The pin rewrites turn `w1.sr` into `x1` and `w3.interval` into `d3_interval`. They make the closing unification legal: the holes were created before `w1` was introduced, so an assignment mentioning `w1.sr` is rejected, while one mentioning `x1` is in scope.
5. `exact rfl` closes the goal of each output, and its unification assigns the holes.

The closing `rfl` discovers the witness, so the generator needs no knowledge of what any operator derives. [The Couple Phase](./generator/couple-phase.md#the-script) covers the remaining cases (machine-only fields, fuels, unused fields). Two definitions follow from the existential:

```lean
def Hydro.toy_relay_vdec : {L : Type} →
  {mem : L → ℕ} →
    {pacing : Pacing L mem} →
      {Td : ℕ} →
        (ℓ : L) →
          HydroSem.Stream mem ℓ ℕ StrOrd.noOrder Retries.exactlyOnce →
            HydroSem.IntervalDec mem (mem ℓ) → ToyRelayDec (Values L mem) (mem ℓ) :=
fun {L} {mem} {pacing} {Td} ℓ x1 d3_interval => Classical.choose ⋯
```

and, per output, the existential's specification instantiated at the corner's own arguments and spelled through `toy_relay_vdec` (binders as in `toy_relay_co_sr₁`, elided):

```lean
@toy_relay_co_rr₁ : ∀ … ,
  (toy_relay (CoupleSem L mem pacing Tc Td hjT) ℓ inp dec sched).1.rr =
    (toy_relay (Values L mem) ℓ inp.rr (toy_relay_vdec ℓ inp.sr sched.interval) { interval := () }).1
```

`Classical.choice` enters the generated artifacts here, through the definition of `toy_relay_vdec` by `Classical.choose`. Every theorem downstream is over `toy_relay_vdec ℓ inp.sr sched.interval`, a decision computed from the machine run.

## Step Monotonicity, the Residual and Monotonicity

**Step monotonicity.** The machine run of a tick-located output grows by prefixes along steps (`TMono`). A stream output gets the placeholder `TMonoTriv`, so that every wire binder is pinned by a hypothesis. `toy_relay_smono₁` is proved by `(intros; exact TMonoTriv.intro _)`. The tick output's lemma, `toy_relay_smono₂ : TMonoTriv inp → TMono (toy_relay (SchedSem L mem pacing) ℓ inp dec sched).2`, is proved by `(intros; simp only [Hydro.toy_relay]; hydro_smono_walk [])`, a loop that dispatches on each operator head to its `tmono_*` law (`Hydro/WfTactics.lean`).

**The residual.** `wf` is `True` at a program input and conjoins through every operator. Only knots and conditional folds add a conjunct. A **conditional fold** is a fold over unordered content whose closure commutes only under a premise about the content; it is well defined on a stream's histories only where the premise holds, so the premise stays in the residual as a pairwise fact ([Grades](./semantics/grades.md)). The statement takes each input wire's `wf` (`inp.4` is the stream corner's fourth field; binders elided):

```lean
@toy_relay_co_wf₁ : ∀ … ,
  inp.4 → (toy_relay (CoupleSem L mem pacing Tc Td hjT) ℓ inp dec sched).1.wf
```

```text
(intros; co_wf_simp [Hydro.toy_relay]; repeat' first | exact trivial | (with_reducible assumption) | (with_reducible refine And.intro ?_ ?_) | (co_wf_simp [Hydro.toy_relay]) | (co_simp [Hydro.toy_relay]) | assumption | refine ⟨?_, ?_⟩)
```

`co_wf_simp` reduces the output's `wf` to a conjunction of the inputs' `wf`s and whatever knots and conditional folds contribute. The loop closes `True` by `trivial`, the inputs by reducible `assumption`, and splits conjunctions. For a module with callees the template adds alternatives between the reducible closers and the re-normalizers that apply each callee's own `wf` lemma by keyed matching; `toy_loop` below has them. The two default-transparency closers come last, because default-transparency unification on a goal that mentions a folded callee unfolds the callee ([`wf`, `smono` and `mono`](./generator/wf-smono-mono.md#the-wf-script)).

**Monotonicity.** The denotation is monotone in its inputs, in the order of each stream's **grade** (its pair of Rust ordering and retry markers, which fixes both the carrier and its order; here `PoolLe`, sub-multiset per member):

```lean
@toy_relay_mono₁ : ∀ {L : Type} {mem : L → ℕ} (ℓ : L)
  {inp inp' : HydroSem.Stream mem ℓ ℕ StrOrd.noOrder Retries.exactlyOnce} (dec : ToyRelayDec (Values L mem) (mem ℓ))
  (sched : ToyRelaySched (Values L mem) (mem ℓ)),
  (∀ (i : Fin (mem ℓ)), PoolLe StrOrd.noOrder Retries.exactlyOnce (MemberMap.get inp i) (MemberMap.get inp' i)) →
    ∀ (i : Fin (mem ℓ)),
      PoolLe StrOrd.noOrder Retries.exactlyOnce (MemberMap.get (toy_relay (Values L mem) ℓ inp dec sched).1 i)
        (MemberMap.get (toy_relay (Values L mem) ℓ inp' dec sched).1 i)
```

No tactic runs. `MonoRel` is an instance whose carriers are pairs of `Values` carriers with a proof that the first is below the second, so the module at `MonoRel`, applied to the inputs packed as `⟨(inp, inp'), hm⟩`, has a `.property` whose type is definitionally this statement. `genMono` hands that projection to the kernel as the proof ([Monotonicity by Instantiation](./semantics/monorel.md)).

## The Project Phase

The corner artifacts above bridge the denotation to the step machine. The project phase builds the bridge from the step machine to the third level. An instance sees operator applications, not the body's `let`-sharing, so `SchedSem` cannot be executed as processes ([The Approach](./approach.md#the-dataflow-graph)). The phase compiles the body to a typed dataflow graph, one `letN` node per wire `let` (`#print toy_relay.dag`, shortened):

```lean
fun {L} {mem} ℓ =>
  Project.Dag.letN (Project.Node.map instDecidableEqNat instDecidableEqNat Project.Var.zero fun _i n => n + 1)
    (Project.Dag.letN (Project.Node.batch instDecidableEqNat StrOrd.noOrder Retries.exactlyOnce Project.Var.zero ())
      (Project.Dag.letN (Project.Node.source_interval (fun x => 0) 1)
        …
            (Project.Dag.letN (Project.Node.allTicks instDecidableEqNat Project.Var.zero)
              (Project.Dag.ret
                (Project.HList.cons Project.Var.zero (Project.HList.cons Project.Var.zero.succ Project.HList.nil))))))))
```

The batch nodes carry `()`, the step machine's batch decision, and the timer node carries no fire script. Schedule data lives outside the graph, in a schedule with one component per node, which `toy_relay.schedOf` builds from the module's records and `toy_relay.recordOf` inverts:

```lean
fun {L} {mem} pacing ℓ dec sched => ((), (), sched.interval, (), (), (), ())
fun {L} {mem} pacing ℓ S => ({ batch := (), gate := () }, { interval := S.2.2.1 })
```

The graph is therefore one term for every schedule and every pacing. `toy_relay.dag_denote` says that reading the graph gives the module's run at the step machine:

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

`env` is the graph's input environment, a heterogeneous list with one machine history per input wire (here one stream). The left side is `Dag.denote`, the graph read at the step machine: each node is evaluated by the `SchedSem` operator it names, under the node's component of the schedule `toy_relay.schedOf pacing ℓ dec sched` (`Hydro/Graph/IR.lean`):

```lean
def Dag.denote : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) → Dag.Sched d → Env Γ →
    HList Kind.sched outs
  | _, _, .ret xs, _, env => xs.map (fun v => env.get v)
  | _, _, .letN n rest, sch, env => Dag.denote rest sch.2 ((Node.denote n sch.1 env).append env)
```

`Dag.denote` is therefore not a new semantics: it applies the step machine's definitions to the graph. The right side is the module at `SchedSem`, its input read from the environment (`env.get Var.zero`), its two outputs listed in order. The lemma is the one per-module link from the step machine to the compiled level; everything below it (`Dag.run_spec`, `Dag.step_wired`, `Dag.iterate_eq_pass`, `WireTrace.streamOutputsAt_eq`) is proved once over all graphs ([Graph and Execution](./execution/graph-ir-causality.md)). The environment is a variable, not a literal list, so that the lemma rewrites in its callers ([`dag_denote`: the statement](./generator/project-phase.md#the-dag_denote-statement)).

Both sides are definitionally equal: unfolding `Dag.denote` over the `letN` chain applies the same `SchedSem` operators to the same arguments as unfolding the module. For a module without callees, the proof is that one fact. `addThmByKernelRfl` builds `fun … => Eq.refl lhs` and gives it to the kernel, and the command logs `Hydro.toy_relay.dag_denote (kernel rfl)`. The judgement goes to the kernel and not to `Meta.isDefEq` because every `Node Γ κs` in the term carries its typing context and the elaborator re-traverses that unshared structure; a `Meta`-side `rfl` of `dag_denote` on the leader-election wrapper took 108 s ([below](#kernel-judged-terms)). A module with callees needs two hops, shown with the knot module below.

`toy_relay.schedOf_recordOf`, `schedOf (recordOf S).1 (recordOf S).2 = S`, is `Prod` and `Unit` eta, one kernel judgement. `toy_relay.dag_valid : (toy_relay.dag ℓ).valid = true` is proved one node at a time with lags the generator computes, and `toy_relay.dag_agreeL`, the graph's lag law (inputs that agree up to given lags give outputs that agree up to computed lags), is `Dag.denote_agreeL` transported along `dag_denote`. [The Project Phase](./generator/project-phase.md) has the walker, the schedule records and the validity proof.

## Using the Artifacts in a Machine Theorem

`toy_safe_sched` (`Hydro/HydroGenCheck.lean`) says that every value the step machine's `toy_relay` emits is positive, for every pacing, schedule and horizon. Its proof runs the module at the corner on `CoStream.inputC h v hc`, an input corner pairing the machine history `h` with a pool `v` above it, and uses only the artifacts above and the contract (shortened):

```lean
  have hwf := toy_relay_co_wf₁ (Tc := T) (Td := T)
    (hjT := Nat.le_refl T) (pacing := pacing) ℓ
    (CoStream.inputC h v hc) ⟨CoDec.batch Nat, CoDec.batch Unit⟩ ⟨CoDec.interval fires⟩ trivial
  have hcpl := (toy_relay (CoupleSem L mem pacing T T (Nat.le_refl T))
    ℓ (CoStream.inputC h v hc) ⟨CoDec.batch Nat, CoDec.batch Unit⟩ ⟨CoDec.interval fires⟩).1.cpl hwf
    T (by co_lag [toy_relay_co_lag₁]) i
  …
  rw [hsr, hrr] at hcpl
  have hens := toy_relay.ensures ℓ v
    (toy_relay_vdec (pacing := pacing) (Td := T) ℓ h fires) ⟨()⟩
  exact hens.pos i x (Multiset.mem_of_le hcpl (Multiset.mem_coe.mpr hx))
```

The residual lemma supplies the `wf` that the corner's `cpl` field consumes; the lag equation discharges its horizon side condition; the `sr` and `rr` equations (`hsr`, `hrr`) rewrite the coupling's two sides to the machine run and to the `Values` run at `toy_relay_vdec`; the contract holds at that decision because it holds at every decision. Nothing in the proof unfolds the module ([Transferring to the Machine and the Network](../using/proving/transferring-to-the-machine.md)).

## The Knot Module `toy_loop`

`toy_loop` closes `toy_step` over its own output. `toy_step` relays its first argument through `toy_relay`, sends the result over a network hop to the same location (`broadcast_closed`), and merges it with the external input:

```lean
hydro def toy_loop (H : HydroSem L mem) (ℓ : L)
    (inp : H.Stream ℓ Nat .noOrder .exactlyOnce)
    (tdec : ToyLoopDec H (mem ℓ)) (sched : ToyStepSched H (mem ℓ)) :
    H.Stream ℓ Nat .noOrder .exactlyOnce :=
  fix (w : H.Stream ℓ Nat .noOrder .exactlyOnce) via tdec.fuel
    invariant w (out out') => ∀ i, out i ≤ out' i,
      …
  :=
    let out := toy_step H ℓ w inp tdec.relay sched
    complete (out)
    out
```

At `Values` the knot is defined as the body iterated `tdec.fuel` times from the empty stream; the **fuel** is a decision of type `H.FixDec`, a `Nat` at `Values` and `Unit` at `SchedSem`. At the machine the knot wire at a step equals the body's output at that step (`fix_stream_eq`), and this fixpoint equation holds because the hop delays by one step ([The Step Machine](./semantics/sched.md)).

Elaboration hoists three constants: the **chain module** `toy_loop.body` (the `fix` body as a registered module, with the loop wire as its last argument), the knot `toy_loop.w`, whose value is `H.fix tdec.fuel (inp, tdec, sched) (toy_loop.w.body ℓ)`, and the knot's instance-generic body `toy_loop.w.body`, a `@[reducible]` projection of the chain ([Elaborating `hydro def`](./generator/elaborating-hydro-def.md#hoisting-the-knots)). Their pipelines run while the `fix` elaborates, innermost first. The command's log, with the knot stack's per-step lines:

```text
hydro_couple Hydro.toy_loop.body: 2 sr, 2 rr, 2 lag (+ex/vdec)
…
hydro_project Hydro.toy_loop.body: Hydro.toy_loop.body.dag, Hydro.toy_loop.body.dag_denote (kernel graphs + simp + kernel)
hydro_knot Hydro.toy_loop.w: sr ok
hydro_knot Hydro.toy_loop.w: lag ok
hydro_knot Hydro.toy_loop.w: rr ok
hydro_knot Hydro.toy_loop.w: strict ok
hydro_knot Hydro.toy_loop.w: smono ok
hydro_knot Hydro.toy_loop.w: mono ok
hydro_knot Hydro.toy_loop.w: stages ok ([Hydro.toy_loop.w.stages,
…
fix invariant: Hydro.toy_loop.w.inv, Hydro.toy_loop.w.inv_stages
…
hydro def Hydro.toy_loop: callees [Hydro.toy_loop.w, Hydro.toy_loop.body], inlines [], fix := false
hydro_glue Hydro.toy_loop: 1 sr, 1 rr, 1 lag (+vdec)
```

### The Chain Module and the Two-Hop `dag_denote`

`toy_loop.body` calls `toy_step`, which reaches no knot, so the chain takes the couple route. Its existential script differs from `toy_relay`'s in two places:

```text
(intros; apply Exists.intro ((⟨⟨_, _⟩, _⟩)); intro Tc' h' w1 w2 w3 w4 hp_x1 hp_d3_relay_interval hp_d3_net hp_x4; refine ⟨?_, ?_, ?_⟩ <;> ((try co_simp [Hydro.toy_loop.body, Hydro.toy_step_co_sr₁, Hydro.toy_step_co_rr₁, Hydro.toy_step_co_lag₁]); co_residuals [Hydro.toy_step_co_sr₁, Hydro.toy_step_co_rr₁, Hydro.toy_step_co_lag₁]; (try rw [hp_x1]); (try simp only [hp_x1]); (try rw [hp_d3_relay_interval]); (try simp only [hp_d3_relay_interval]); (try rw [hp_d3_net]); (try simp only [hp_d3_net]); (try rw [hp_x4]); (try simp only [hp_x4]); (try exact rfl)))
```

The callee's `sr`/`rr` equations join the simp set, and `co_residuals`, the glue engine's rewrite pass as a tactic, rewrites the callee redexes `simp`'s congruence cannot reach. It works on the left side of each goal only, so that no candidate is unified against the witness holes on the right. And there are three goals for two outputs: the record `ToyLoopDec` carries the knot's fuel, which the chain never uses, so the existential pins it with a trailing conjunct that `rfl`-unification closes. The generator states the pin as its own lemma (shortened), and the proof of the knot's `rr` equation uses it:

```lean
@toy_loop.body_vdec_fuel₁ : ∀ … (w : HydroSem.Stream mem ℓ ℕ StrOrd.noOrder Retries.exactlyOnce),
  (toy_loop.body_vdec ℓ inp.sr sched.relay.interval sched.net w.sr).fuel = Td + 1
```

The chain's graph is a single `call` of the callee's graph:

```lean
fun {L} {mem} ℓ =>
  Project.Dag.letN
    (Project.Node.call (toy_step.dag ℓ)
      (Project.HList.cons Project.Var.zero.succ (Project.HList.cons Project.Var.zero Project.HList.nil)))
    (Project.Dag.ret (Project.HList.cons Project.Var.zero (Project.HList.cons Project.Var.zero Project.HList.nil)))
```

Its `dag_denote` has the same shape as `toy_relay`'s, but one kernel judgement no longer suffices: it would unfold `toy_step.dag` through `Dag.denote`, unfold `toy_step` at `SchedSem`, and compare the normal forms, and a nested call stack repeats that per level. The proof is two hops. **Hop 1**, `toy_loop.body.dag_denote_graphs`, is a kernel judgement against an opened chain, the module's own `let`-chain with each callee application replaced by the components of the callee graph's reading:

```lean
@toy_loop.body.dag_denote_graphs : ∀ {L : Type} {mem : L → ℕ} (pacing : Pacing L mem) (ℓ : L)
  (tdec : ToyLoopDec (SchedSem L mem pacing) (mem ℓ)) (sched : ToyStepSched (SchedSem L mem pacing) (mem ℓ))
  (env : …),
  Project.Dag.denote pacing (toy_loop.body.dag ℓ) (toy_loop.body.schedOf pacing ℓ tdec sched) env =
    Project.HList.cons
      (let out :=
          (Project.Dag.denote pacing (toy_step.dag ℓ) (toy_step.schedOf pacing ℓ tdec.relay sched)
              (Project.HList.cons (env.get Project.Var.zero.succ)
                (Project.HList.cons (env.get Project.Var.zero) Project.HList.nil))).1;
        (out, out)).1
      …
```

(`env`'s type and the second output elided.) A `call` node denotes as `Dag.denote N sch (args.map (fun v => env.get v))`, so both sides contain `Dag.denote pacing (toy_step.dag ℓ) …` with the same head and the kernel compares them argument by argument without unfolding the callee's graph. **Hop 2** rewrites each such application by the callee's own `toy_step.dag_denote` with `Meta.simp`, which leaves `toy_step` folded, and closes the result against the module by `Eq.refl`. The generated proof term (`#print toy_loop.body.dag_denote`, shortened) is that chain:

```lean
fun {L} {mem} pacing ℓ tdec sched env =>
  Eq.trans
    (Eq.trans (toy_loop.body.dag_denote_graphs pacing ℓ tdec sched env)
      (congr
        (congrArg Project.HList.cons
          (congrArg Prod.fst
            (toy_step.dag_denote pacing ℓ tdec.relay sched (Project.HList.cons (env.get ⋯) (Project.HList.cons ⋯ ⋯)))))
        …))
    (Eq.refl
      (Project.HList.cons
        (toy_step (SchedSem L mem pacing) ℓ
          ((Project.HList.cons (env.get ⋯.succ) (Project.HList.cons (env.get ⋯) Project.HList.nil)).get
            Project.Var.zero)
          …
          tdec.relay sched)
        …))
```

The outer `Eq.trans` composes hop 1, `simp`'s congruence proof (the rewrite by `toy_step.dag_denote` under `HList.cons` and `Prod.fst`), and an `Eq.refl` of the rewritten chain. The kernel accepts that `Eq.refl` at the type `… = HList.cons (toy_loop.body (SchedSem L mem pacing) …).1 …` by unfolding `toy_loop.body` once, `HList.get` on a literal list and `Prod` eta; `toy_step` stays folded on both sides. A knot group in a graph is one `fixN` node, written in hop 1 as a cascade of single-knot fixpoints (`bekic`) over the body graph. The same two hops on `toy_step`, with every step of the proof term read off, the guarded check before hop 2's `Eq.refl` and the per-component diagnosis of a refused hop 1 are on [`dag_denote`: the proof](./generator/project-phase.md#proof-of-dag_denote).

### The Knot's `sr` Equation

```lean
@toy_loop.w_co_sr₁ : ∀ {L : Type} {mem : L → ℕ} {pacing : Pacing L mem} {Tc Td : ℕ} {hjT : Tc.le Td} (ℓ : L)
  (inp : HydroSem.Stream mem ℓ ℕ StrOrd.noOrder Retries.exactlyOnce)
  (tdec : ToyLoopDec (CoupleSem L mem pacing Tc Td hjT) (mem ℓ))
  (sched : ToyStepSched (CoupleSem L mem pacing Tc Td hjT) (mem ℓ)),
  (toy_loop.w (CoupleSem L mem pacing Tc Td hjT) ℓ inp tdec sched).sr =
    toy_loop.w (SchedSem L mem pacing) ℓ inp.sr { relay := { batch := (), gate := () }, fuel := () }
      { relay := { interval := sched.relay.interval }, net := sched.net }
```

The knot route does not normalize both sides, because both sides contain the knot and normalizing either means unfolding through it. The script is one tactic call:

```text
(intros; co_knot_sr [Hydro.toy_loop.w, Hydro.toy_loop.w.body] [Hydro.toy_loop.body_co_sr₁])
```

The macro (`Hydro/KnotTactics.lean`, shortened) unfolds the knot and its body wrapper, so both sides are `H.fix` over the same body at two instances, then:

```lean
      first
        | refine Eq.trans ?_ (sched_hfix_stream' _ _ _).symm
        | refine Eq.trans ?_ (sched_hfix_tick' _ _ _).symm
      …
      refine congrArg₂ _ rfl ?_
      funext x
      co_body_rw [$rules,*]
```

The `Eq.trans` turns the `SchedSem` side into `fix_stream` of a body function; the corner side is rewritten to the same form, the **machine diagonal** (the machine knot built from the corner body's machine run, evaluated on corner values that carry a machine value alone). `congrArg₂ _ rfl` splits the two `fix_stream` applications: the fuels, both `Unit`, close by a record-sized `rfl`. `funext x` leaves the body-level goal, and `co_body_rw` closes it by rewriting with the chain's `sr` equation `toy_loop.body_co_sr₁` (and, in a nested knot, each inner knot's `sr` equation). No step compares the two knots' unfoldings ([The Knot Stack](./generator/knot-stack.md#the-sr-equation)).

### The Knot's Lag and `rr` Equations

`toy_loop.w_co_lag₁ : (toy_loop.w (CoupleSem …) …).lag = inp.lag`. A knot's lag is its body's output lag when the loop input carries lag 1. The body here is `union (values (broadcast_closed net (toy_relay s …).1)) inp`, whose lag is `max (s.lag - 1) inp.lag`, and the hop's `- 1` cancels the loop's `1`. The `rr` equation, with the binders of `toy_loop.w_co_sr₁` elided:

```lean
@toy_loop.w_co_rr₁ : ∀ … ,
  (toy_loop.w (CoupleSem L mem pacing Tc Td hjT) ℓ inp tdec sched).rr =
    toy_loop.w (Values L mem) ℓ inp.rr
      (toy_loop.body_vdec ℓ inp.sr sched.relay.interval sched.net
        (toy_loop.w (SchedSem L mem pacing) ℓ inp.sr { relay := { batch := (), gate := () }, fuel := () }
          { relay := { interval := sched.relay.interval }, net := sched.net }))
      { relay := { interval := () }, net := () }
```

The denotation's loop runs at the chain's derived decision `toy_loop.body_vdec` with the loop position taken by the machine knot itself (the last argument). The loop's decision depends only on the machine run of the loop, never on a value that `Values` computes. The proof is `co_knot_rr₂ HSR [defs] [PIN1] [rules]`, the same skeleton as `co_knot_sr` with two installed facts: `HSR`, the knot's `sr` equation just proved, which bridges the machine diagonal inside the denotation iterate, and `PIN1`, the chain's fuel pin `toy_loop.body_vdec_fuel₁`, which closes the fuel field. The fuel field needs the pin because the chosen decision's fuel equals `Td + 1` only propositionally, so the `rfl` that closed the machine side's `Unit` fuels does not close it.

### Strictness, Stages and the Invariant

```lean
@toy_loop.w_strict₁ : ∀ {L : Type} {mem : L → ℕ} {pacing : Pacing L mem} {h : ℕ} (ℓ : L)
  {inp inp' : HydroSem.Stream mem ℓ ℕ StrOrd.noOrder Retries.exactlyOnce}
  (tdec : ToyLoopDec (SchedSem L mem pacing) (mem ℓ)) (sched : ToyStepSched (SchedSem L mem pacing) (mem ℓ)),
  SAgree h inp inp' →
    SAgreeL h 0 (toy_loop.w (SchedSem L mem pacing) ℓ inp tdec sched)
      (toy_loop.w (SchedSem L mem pacing) ℓ inp' tdec sched)
```

Two machine runs whose inputs agree up to step `h` give knot wires that agree up to `h`. The proof is a direct term, `causalL_fix_stream` applied to the chain's graph lag law `toy_loop.body.dag_agreeL` at the lag vector `1` on the loop wire and `0` on the inputs, with the output lags computed one graph node at a time, so the chain's project phase runs before its knots. A body with no network hop or `defer_tick` on the cycle keeps the loop input's lag, so the computed lag exceeds the one the statement reads and the knot stack stops while the `fix` elaborates. For the minimal offending knot, `fix w := union w inp`, the first error is the graded read (`agreeAtLag`):

```text
HydroGen: a lag-2 agreement cannot be read at lag 1
```

A body whose graded read passes but whose base read at lag `0` does not gets ``HydroGen: knot {K}: the body's output lag is {μ}, not `0` — the body is not strict in the knot wire (a cycle without a network hop or `defer_tick`)`` (`agreeBaseOf`). The enclosing module's graph, where the knot block is one `fixN` node, is checked once more by the project phase's validity proof, which states the rule per knot of a block (`HydroGen: knot {i} of a knot block is not strict in its own wire once the earlier knots are closed …`, [`dag_valid`](./generator/project-phase.md#proving-dag_valid-per-node)).

`toy_loop.w.stages m` is the knot's **Kleene chain** at `Values`, `iterate (fun w => (toy_loop.body (Values L mem) ℓ inp tdec sched w).1)` from the empty stream, with `stages_zero`, `stages_succ` and `stages_fix` (the knot equals the stage at its fuel) by `rfl`, and `stages_mono` by `iterate_chain`. The `invariant` clause is then elaborated over the stages into `toy_loop.w.inv` and `toy_loop.w.inv_stages` ([Knot Invariants](../using/proving/knot-invariants.md#the-invariant-clause)).

### The Knot's Residual

A knot's `wf` asks three things of its body (a fourth, step monotonicity, for a tick knot): the machine body is strict in the loop wire, the denotation body's Kleene chain ascends, and at every stage `j` the body's coupling holds at the lowered horizon `j + L`, with `L` the knot's lag and the loop input coupled to the stage. The generator builds each fact as a term, installs the facts as local hypotheses `FX`, `FC`, `HVA`, `FM`, `FH`, `FOUT` (and `FSM` for a tick knot), and runs a fixed script on the goal `(toy_loop.w (CoupleSem …) …).wf` (`genKnotWf`, the stream case):

```lean
      s!"(hb_block \"wf-simp\" (co_wf_simp [{mc.mName}]; refine ⟨?_, ?_, ?_⟩); " ++
      "(hb_block \"wf-strict\" (intro h x y hxy; have h1 := FC h x y hxy; " ++
      "rw [← FX x, ← FX y] at h1; exact h1)); " ++
      "(hb_block \"wf-chain\" (exact FH)); " ++
      "(hb_block \"wf-cpl\" (intro j hj x hxv i; exact FOUT j hj x hxv i)))"
```

`co_wf_simp` on the knot exposes the triple. `FC` is the body's strictness stated over the machine body in the generator's own spelling, and `FX`, the chain's `sr` equation at the corner values the machine diagonal runs on, rewrites it into the knot's spelling. `FH` is the ascending chain, `iterate_le_succ` from the body's monotonicity `FM`. `FOUT` is the coupling at stage `j`: the chain's own `wf` lemma at the corner lowered to horizon `j + L`, with the loop input coupled at lag `L + 1`, its `cpl` read at `j`, transported along the chain's `sr` and `rr` equations. The generator defines the lag `L` as a separate `def`, `toy_loop.w_lag`, because it occurs in every lowered horizon and an open spelling would carry the whole body into the type of every corner argument. Each `hb_block` reports its own heartbeats under `HYDROGEN_HB_DIAG` ([The Knot Stack](./generator/knot-stack.md#the-wf-triple)).

### The Module Around the Knot

After both passes `toy_loop`'s value no longer contains a `fix`: it is `toy_loop.w` applied, then a projection of `toy_loop.body` at the closed knot. Its spec walk therefore reports `fix := false` with the knot as a callee, and the module takes the **glue route**: the glue engine finds its `sr`/`rr` equations over the folded knot (`toy_loop_co_rr₁` is at `toy_loop_vdec ℓ inp.sr sched.relay.interval sched.net`), and its residual closes by the knot's and the chain's `wf` lemmas, applied by keyed matching:

```text
(intros; co_wf_simp [Hydro.toy_loop]; repeat' first | exact trivial | (with_reducible assumption) | (with_reducible refine And.intro ?_ ?_) | (hydro_rule_apply [Hydro.toy_loop.w_co_wf₁, Hydro.toy_loop.body_co_wf₁, Hydro.toy_loop.body_co_wf₂]) | (hydro_wf_close [Hydro.toy_loop.w_co_wf₁, Hydro.toy_loop.body_co_wf₁, Hydro.toy_loop.body_co_wf₂]) | (co_wf_simp [Hydro.toy_loop]) | (co_simp [Hydro.toy_loop]) | assumption | refine ⟨?_, ?_⟩)
```

The whole of `toy_loop`, every artifact above included, elaborates in about two seconds. `#print axioms toy_loop.w_co_wf₁` reports `[propext, Classical.choice, Quot.sound]`.

## Cross-Cutting Techniques

### Proof by Script String

Each phase's scripts are string templates, filled per module with the module's name, its callees' lemma names, the witness skeleton and the pin names, then parsed in the current environment and run on the closed statement in an empty local context with error recovery off (`proveByTac`, [The proof entry points](./generator/overview.md#the-proof-entry-points)). A string is the right representation because everything a script names (`toy_step_co_rr₁`, `hp_d3_net`) exists only after the module and its callees have been generated, and because a script that mentions only global names cannot collide with the generator's own open binders. Every script is a fixed composition of per-operator `rfl` lemmas and closers. A failure is therefore a program outside the grammar or a generator bug, never a proof the user repairs.

### Kernel-Judged Terms

Some statements are true by unfolding but too large for the elaborator to check: `dag_denote` of a module without callees, hop 1 of the others, `schedOf_recordOf`, and the staged activation's `eq_impl`. `addThmByKernelRfl` hands `Eq.refl lhs` to the kernel only. It runs no `Kernel.isDefEqGuarded` pre-check, because that would double the kernel's work on the production graphs. A refusal is not recovered: the function returns `false`, and every caller throws an error that names the declaration and what to compare. The same reasoning makes the knot facts direct terms rather than scripts. A script of the form `simp only [K]; rw [sched_hfix_stream']; exact …` makes `Meta` evaluate the chain as soon as the facts name the chain module, while a term whose type the kernel checks against the knot's unfolding costs one judgement.

The kernel has limits of its own. It is never asked to compare the `.lag` of two differently headed module applications, or to evaluate `Dag.valid` of a whole production graph: the first is a full `whnf` of the chain, the second re-runs a wire's producer at every read of its lag, and kernel evaluations of this kind took gigabytes (up to 19 GB). A knot's lag is therefore opened by generic instance lemmas at a shared probe term, and validity is computed by the generator and proved one node at a time on literal inputs ([Elaboration and Kernel Cost](./cost-model.md#kernel-evaluation-of-graphs-lags-and-knot-wires)).

### Rewrite Loops Instead of Definitional Unfolding

Definitional equality through `k` nested knots is exponential in `k`, because each outer knot's body re-closes the inner knots and kernel substitution loses sharing. One default-transparency `rfl` on the `leader_election` body takes 9 ms with no knot, 21 ms inside two, about 4 minutes inside three, and does not finish in 51 M heartbeats inside five ([the measurements](./cost-model.md#definitional-equality-through-knots)). The knot tactics therefore take definitional steps only at record scale (the fuel fields) and leaf scale (the closing `rfl`s), and fold the body with `co_body_rw`, a `repeat (first | rw [r₁] | … | rw [rₙ] | rw [closers])` loop over the body's `sr`/`rr` equations and the inner knots' equations ([Projections and Tactics](./corner/projections-and-tactics.md)). The loop uses `rw` and not `simp`, because `simp` does not rewrite a module projection nested in a dependent-subtype argument position; each `rw` consumes one instantiation. The `H.fix` wrapper is aligned by `Eq.trans` and not by `rw`, because a higher-order `rw` pattern does not match a `fix` whose body is a constant. Outside tactics, the generator rewrites by `kabstract` at reducible transparency: the occurrences are syntactic, and at default transparency every same-headed subterm becomes an `isDefEq` that unfolds both modules through their knots.

### Hoisting to Constants

`hydro def` gives every object a proof could otherwise unfold its own top-level `def`: each knot (`<M>.<wire>`), each knot body (`<M>.<wire>.body`, `@[reducible]` so that the `fix` laws see through it while unification keys on the constant), each chain (`<M>.body`), each knot's lag (`<K>_lag`), the Kleene chain (`<K>.stages`), each graph (`<M>.dag`); each ghost fact becomes its own `theorem` (`<M>.ghost.<h>`). Proofs about a knot then work with the constants `<M>.<wire>` and `<M>.<wire>.body`, never with their unfolded values, and a term that mentions a knot carries the constant `<M>.<wire>`, not the knot's body.

### Keyed Matching Before Unification

A failing unification costs more than a succeeding one: the unifier learns the mismatch at the bottom of a shared projection spine and retries each level in its unfolded form on the way up, and `with_reducible apply leader_election.body_smono₁₄` took 7.08 s to fail. Every generated walk that tries rules in turn (`hydro_rule_apply`, the monotonicity resolver's `conclKeyMismatch`) therefore first compares the rule's key with the goal's (the head constant and the literal projection path, read through `Prod.fst`/`Prod.snd`, `Subtype.val`, member reads and `@[reducible]` heads), skips a known mismatch, and lets anything uncertain fall through to `apply` ([The Glue Phase](./generator/glue-phase.md#keyed-rule-application)).

When a generated proof fails, the error names the script and the remaining goal, the first divergent node pair of two normal forms that did not meet, or the declaration the kernel refused; the messages and the environment variables that expand them are listed on [the Generator Overview](./generator/overview.md#diagnostics), and symptoms with their causes on [Traps](./traps.md).
