---
sidebar_position: 9
title: "The Project Phase"
---

# The Project Phase

The `sr` and `rr` equations carry a contract down to the step machine `SchedSem`, but a process cannot execute `M (SchedSem …)`. A `hydro def` is a **tagless-final** program, a function of an abstract instance `H : HydroSem L mem`, and an instance sees operator applications, not the program's `let`-sharing, and cannot step a `fix` body once per global step ([Why a Graph](../execution/graph-ir-causality.md#why-a-graph)). Processes therefore run a typed graph, `Project.Dag Γ outs`, whose nodes are operators, callee graphs and knot blocks, and the graph is read with `SchedSem`'s definitions by `Dag.denote` ([The Graph IR](../execution/graph-ir-causality.md) defines both). The generator is the one agent that sees a module's `let`s, so it builds the graph, and it has to prove, per module, that the graph agrees with the module.

The project phase (`genProject`) compiles every registered module except a knot constant, and proves that the graph agrees with the module at the machine. It is the last phase of every route through `runPipeline` (`hydro_lean/Hydro/HydroDef.lean`), and it emits, per module `M`:

| Artifact | What it is |
|---|---|
| `M.dag` | the module's `let`-chain as an A-normal-form graph |
| `M.schedOf`, `M.recordOf`, `M.schedOf_recordOf` | the module's network cursors and timer scripts in graph shape, the inverse, and the round trip |
| `M.dag_denote` | the graph read by `Dag.denote` equals `M` at `SchedSem` |
| `M.dag_valid` | every cycle among a knot block's wires passes a network hop or a `defer_tick` |
| `M.dag_agreeL` | the graph's lag law: how much of its inputs' agreement each output keeps |
| `M.wireCodecs` | one payload codec per network node |

The obvious proofs, one `rfl` for `dag_denote` and one `decide` for `dag_valid`, are true and do not finish on real programs: both make the kernel evaluate whole graphs, whole callee modules or knot wires. The phase instead proves `dag_denote` in two hops that each keep every callee folded, and `dag_valid` one node at a time on literal lag vectors. The walker that builds the graph comes first, then the schedule records, then the two proofs.

## The Walker

`genProject` (`hydro_lean/Hydro/HydroGenProject.lean`) opens the module's binders with `H := SchedSem L mem pacing`, peels the body's `let` telescope (`withLets`), and walks the items with `inferType` deciding wire versus data. A **wire** is a value of a live-collection type (`HydroSem.Stream …`, `HydroSem.Atomic …`, …, recognised by `kindOfType`). The walk is total over the module grammar. Anything wire-typed that is not an operator application, a callee invocation, a knot constant or a bound wire is an error.

`toy_step` (`Hydro/HydroGenToy.lean`) is `H.union (H.values (H.broadcast_closed sched.net (toy_relay H ℓ s dec sched.relay).1)) inp`, compiled:

```lean
fun {L} {mem} ℓ =>
  Project.Dag.letN (Project.Node.call (toy_relay.dag ℓ) (Project.HList.cons Project.Var.zero Project.HList.nil))
    (Project.Dag.letN (Project.Node.broadcast_closed instDecidableEqNat Project.Var.zero)
      (Project.Dag.letN (Project.Node.values (instDecidableEqFin (mem ℓ)) instDecidableEqNat Project.Var.zero)
        (Project.Dag.letN
          (Project.Node.union instDecidableEqNat Project.Var.zero Project.Var.zero.succ.succ.succ.succ.succ)
          (Project.Dag.ret (Project.HList.cons Project.Var.zero Project.HList.nil)))))
```

Every wire `let`, and every nested operator application, is one `letN`. Variables are de Bruijn indices into a context that grows by each node's outputs (`Var.zero` is the latest), so the `call` node, which binds `toy_relay`'s two outputs, is read by `broadcast_closed` as `Var.zero` (the first output), and `union` reaches the input `inp` five positions back. The transport cursor `sched.net` is not in the graph (below). A `tick` block becomes one `tick_scan` node whose step function is data and stays a Lean `let` between the nodes; `toy_relay`'s graph, with one, is listed on [How Proofs Are Generated](../proof-generation.md#the-project-phase).

The walker's state binds each wire fvar to its context position (`Bind.wire pos`) or, for a tuple of wires, to one position per component (`Bind.tuple poss`). `toPos` turns a wire-typed term into a context position. A bound fvar is looked up; a `.1`/`.2` chain on a tuple-bound fvar indexes its positions; `toNode` first emits anything else as an anonymous node. This emission is the A-normal-form step, so `H.values (H.broadcast_closed …)` becomes two nodes. `toNode` maps an operator field to its `Node` constructor (`nodeCtorOf`), a callee `C` to `Node.call (C.dag kept) subst` with the substitution built from the argument positions, and rejects a raw `HydroSem.fix`:

```text
project: a raw knot in the body ({c}); knots must be the elaborator's hoisted constants
```

The elaborator hoists every knot into its own top-level constant and its body into a registered module, the hoisted chain body ([Elaborating hydro def](./elaborating-hydro-def.md)). A run of the module's knot constants is one knot group, compiled to one `Node.fixN` whose body is `call B.dag` for the hoisted chain body `B`, walked in a context extended by the knot wires. The enclosing chain may re-apply `B` at the closed knots (the elaborator's Bekić continuation); the walker resolves those reads to the `fixN`'s exposed outputs, and refuses a read of the knot's own next-value component after `complete`. `toy_loop` compiles to:

```lean
fun {L} {mem} ℓ =>
  Project.Dag.letN
    (Project.Node.fixN [Project.KnotKind.stream ℓ ℕ instDecidableEqNat StrOrd.noOrder Retries.exactlyOnce]
      (Project.Dag.letN
        (Project.Node.call (toy_loop.body.dag ℓ)
          (Project.HList.cons Project.Var.zero.succ (Project.HList.cons Project.Var.zero Project.HList.nil)))
        (Project.Dag.ret
          (Project.HList.cons Project.Var.zero (Project.HList.cons Project.Var.zero.succ Project.HList.nil)))))
    (Project.Dag.ret (Project.HList.cons Project.Var.zero.succ Project.HList.nil))
```

### Schedule Decisions

A **decision** is an explicit input that resolves a `nondet!` site or a runtime freedom: batch cuts, delivery cursors, timer fires ([Nondeterminism and Decisions](../../using/programs/nondeterminism.md)). At `SchedSem`, a content decision such as a batch cut is `Unit`, because the machine's own run determines it, while delivery cursors (`TransportDec`) and timing scripts (`IntervalDec`, `ClockDec`) are real data. Those are a property of the run, not of the program, so when the walker meets an argument whose type is a schedule decision (`isSchedDecTy`) it leaves it out of the node and records it as the node's schedule component. In `toy_step.dag` above, `broadcast_closed` carries no cursor; `sched.net` became its schedule component.

The graph is compiled once, and the schedule is supplied separately to its reading `Dag.denote` and to its lock-step run. The graph is also **pacing-free**. The data terms mention `SchedSem L mem pacing` only through the types it gives wires, which do not depend on the pacing, so `genProject` replaces `pacing` by `Pacing.idle` in the graph body (the kernel judges the two equal). Therefore one graph serves every pacing, and an interleaving of processes (the order in which they step, [Processes](../execution/processes.md)) is an interleaving of one graph. The graph keeps only the binders it mentions, closed under their types (`keepMask`); the module's schedule record is dropped.

## Schedule Records

A graph's schedule has one component per node (`Hydro/Graph/IR.lean`):

```lean
def Node.Sched : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → Node Γ κs → Type
  | _, _, .broadcast_closed (c := c) (p := p) _ _ => Fin (mem p) → Fin (mem c) → Nat → Nat
  …
  | _, _, .source_interval (ℓ := ℓ) _ _ => TickScript (mem ℓ) Nat
  …
  | _, _, .call N _ => Dag.Sched N
  | _, _, .fixN _ body => Dag.Sched body
  | _, _, _ => Unit
```

A program's schedule is instead a **schedule record** per module, a structure whose fields are its network nodes' cursors, its timers' scripts and its callees' records (`ToyStepSched` with fields `relay` and `net`). The walker records each node's component as an `SComp` (`unit`, `leaf d` for a node's `TransportDec` argument, `call C args`, `nested cs` for a knot body), and the phase emits both directions. For `toy_step` (its graph is above):

```lean
fun {L} {mem} pacing ℓ dec sched => (toy_relay.schedOf pacing ℓ dec sched.relay, sched.net, (), (), ())
```

```lean
fun {L} {mem} pacing ℓ S =>
  ((toy_relay.recordOf pacing ℓ S.1).1, { relay := (toy_relay.recordOf pacing ℓ S.1).2, net := S.2.1 })
```

`recordOf` is built by following each component's projection path from the record binder (`bundlePath`: `sched.relay.interval` decomposes into the binder `sched` and the path `[relay, interval]`) and rebuilding the record type field by field (`buildBundle`), with a callee's sub-record read back through the callee's own `recordOf`. `M.schedOf_recordOf`, `schedOf (recordOf S) = S`, is proved by `addThmByKernelRfl`: after unfolding, the two sides differ only by `Prod` and `Unit` eta, which the kernel decides.

## The `dag_denote` Statement

`M.dag_denote` says that reading the graph at the module's schedule gives the module's outputs at the machine. For `toy_step`, which relays its first input through the callee `toy_relay`, ships it through a network hop and merges in its second input:

```lean
@toy_step.dag_denote : ∀ {L : Type} {mem : L → ℕ} (pacing : Pacing L mem) (ℓ : L)
  (dec : ToyRelayDec (SchedSem L mem pacing) (mem ℓ)) (sched : ToyStepSched (SchedSem L mem pacing) (mem ℓ))
  (env :
    Project.HList Project.Kind.sched
      [Project.Kind.stream ℓ ℕ instDecidableEqNat StrOrd.noOrder Retries.exactlyOnce,
        Project.Kind.stream ℓ ℕ instDecidableEqNat StrOrd.noOrder Retries.exactlyOnce]),
  Project.Dag.denote pacing (toy_step.dag ℓ) (toy_step.schedOf pacing ℓ dec sched) env =
    Project.HList.cons
      (toy_step (SchedSem L mem pacing) ℓ (env.get Project.Var.zero) (env.get Project.Var.zero.succ) dec sched)
      Project.HList.nil
```

The binders are the module's own, with `H` replaced by `pacing` (the instance is `SchedSem L mem pacing`), plus the input environment `env`, a heterogeneous list with one machine history per input wire (`Kind.sched κ` is the `SchedSem` carrier of wire kind `κ`). The left side reads the pacing-free graph `toy_step.dag ℓ` at the pacing, under the schedule `toy_step.schedOf` built from the module's records. The right side is the module itself at `SchedSem`, with input `i` read as `env.get vᵢ`, one list entry per output. A module with `n` outputs has `n` entries, each a projection of the one application (`toy_relay`'s are `(…).1` and `(…).2`).

The environment is a variable so that the lemma works as a `simp` rewrite in its callers; hop 2 below uses it that way. A literal `HList.cons …` environment puts wire types into the discrimination-tree keys, where reducible unfolding of `Kind.sched` makes the lemma's keys and the target's disagree, and the rewrite never fires.

## Proof of `dag_denote`

Both sides are definitionally equal: `Dag.denote` is a structural recursion over the graph that applies, node by node, the same `SchedSem` operators the module applies. The cost lies in which checker judges the equation and how much it unfolds. `genProject` takes one of two routes, by whether the module has callees, and its info line reports the route taken: `hydro_project Hydro.toy_relay: Hydro.toy_relay.dag, Hydro.toy_relay.dag_denote (kernel rfl)` for a module without callees, `(kernel graphs + simp + kernel)` for one with them.

**Without callees**, the proof is one kernel judgement. `addThmByKernelRfl` (`Hydro/HydroGen.lean`) builds `fun xs => Eq.refl lhs` at the statement's type and hands it straight to `addDecl`, so the kernel is the only checker; a refusal returns `false` and the phase throws. It is not `rfl` the tactic, because `Meta.isDefEq`, which `rfl` and `simp` run, re-traverses unshared subterms and is up to 50× slower than the kernel on these terms: every `Node Γ κs` carries its typed context. A `Meta` `rfl` on the leader election's wrapper took 108 s ([Judge by the kernel](../cost-model.md#kernel-judgements)).

**With callees**, one judgement is too expensive. On the left, each `call` node is `Dag.denote (C.dag …)`; on the right, each callee is `C (SchedSem …)`. One `Eq.refl` would make the kernel unfold both to normal forms and compare them, for every callee and, through `C.dag`'s own `call` nodes, at every level below; under a callee that reaches a knot those normal forms apply knot wires at symbolic steps ([The kernel never evaluates a graph, a lag analysis or a knot wire](../cost-model.md#kernel-evaluation-of-graphs-lags-and-knot-wires)). The proof is therefore split into two hops, each of which keeps every callee closed on both sides of its judgement.

### Hop 1

`graphsTerm` builds a term `T` from the module's own `let`-chain, with each full application of a callee `C S args` replaced by the components of `Dag.denote pacing (C.dag …) (C.schedOf …) ⟨args⟩` (`openCallees`), and each knot group replaced by a `let closed := bekic …` over its body graph, whose components bind the knot names. `M.dag_denote_graphs : lhs = T` is then one kernel judgement:

```lean
@toy_step.dag_denote_graphs : ∀ {L : Type} {mem : L → ℕ} (pacing : Pacing L mem) (ℓ : L)
  (dec : ToyRelayDec (SchedSem L mem pacing) (mem ℓ)) (sched : ToyStepSched (SchedSem L mem pacing) (mem ℓ))
  (env : …),
  Project.Dag.denote pacing (toy_step.dag ℓ) (toy_step.schedOf pacing ℓ dec sched) env =
    Project.HList.cons
      (HydroSem.union
        (HydroSem.values
          (HydroSem.broadcast_closed sched.net
            ((Project.Dag.denote pacing (toy_relay.dag ℓ) (toy_relay.schedOf pacing ℓ dec sched.relay)
                    (Project.HList.cons (env.get Project.Var.zero) Project.HList.nil)).1,
                (Project.Dag.denote pacing (toy_relay.dag ℓ) (toy_relay.schedOf pacing ℓ dec sched.relay)
                      (Project.HList.cons (env.get Project.Var.zero) Project.HList.nil)).2.1).1))
        (env.get Project.Var.zero.succ))
      Project.HList.nil
```

(`env`'s type elided.) The kernel unfolds `Dag.denote` over `toy_step.dag`'s own four nodes; at the `call` node it reaches `Dag.denote pacing (toy_relay.dag ℓ) …`, and the right side has the same application, with the same head, so the kernel's lazy unfolding compares the two argument by argument and never opens `toy_relay.dag`. `toy_relay` returns two outputs, so `T` rebuilds the callee's result as the tuple of its output components (`.1` and `.2.1`, the `HList` spelling), where `toy_step`'s chain projects `(toy_relay …).1`. The judgement unfolds only the module's own graph.

### Hop 2

`addDagDenoteClosed` rewrites `T` with the callees' own `C.dag_denote` lemmas by `Meta.simp`. Each rewrite replaces a `Dag.denote (C.dag …) ⟨…⟩` by `HList.cons (C (SchedSem …) …).1 …`, the callee still folded, giving `T'` and a proof `T = T'`. `T'` is now the module's chain with every callee application in place, up to `HList.get` on literal lists, `Prod` eta and the hoisted knot constants, all of which the kernel decides in milliseconds; `Meta.isDefEq` takes tens of seconds on the same comparison. A guarded kernel check runs first, so that a mismatch is a readable error and not a kernel exception:

```lean
    let (r, _) ← Meta.simp T ctx
    let T' ← instantiateMVars r.expr
    let hS ← match r.proof? with
      | some p => instantiateMVars p
      | none => pure (mkApp2 (mkConst ``Eq.refl [u]) ty T)
    unless Kernel.isDefEqGuarded (← getEnv) (← getLCtx) T' rhs do
      throwError "project: the kernel refused the closed chain of `{name}` against the module\n{T'}\n=?=\n{rhs}"
    let h1 := mkApp6 (mkConst ``Eq.trans [u]) ty lhs T T' hG hS
    let hR := mkApp2 (mkConst ``Eq.refl [u]) ty T'
    mkLambdaFVars xs (mkApp6 (mkConst ``Eq.trans [u]) ty lhs T' rhs h1 hR)
```

The proof term is `Eq.trans (Eq.trans hop1 simpProof) (Eq.refl T')`, and `Eq.refl T'` is used at type `T' = rhs`: the last step is the kernel's definitional judgement, not a lemma. This is the generated proof of `toy_step.dag_denote`, as `#print` shows it:

```lean
fun {L} {mem} pacing ℓ dec sched env =>
  Eq.trans
    (Eq.trans (toy_step.dag_denote_graphs pacing ℓ dec sched env)
      (congrFun'
        (congrArg Project.HList.cons
          (congrFun'
            (congrArg HydroSem.union
              (congrArg HydroSem.values
                (congrArg (HydroSem.broadcast_closed sched.net)
                  (congrArg Prod.fst
                    (toy_relay.dag_denote pacing ℓ dec sched.relay
                      (Project.HList.cons (env.get Project.Var.zero) Project.HList.nil))))))
            (env.get Project.Var.zero.succ)))
        Project.HList.nil))
    (Eq.refl
      (Project.HList.cons
        (HydroSem.union
          (HydroSem.values
            (HydroSem.broadcast_closed sched.net
              (toy_relay (SchedSem L mem pacing) ℓ
                  ((Project.HList.cons (env.get Project.Var.zero) Project.HList.nil).get Project.Var.zero) dec
                  sched.relay).1))
          (env.get Project.Var.zero.succ))
        Project.HList.nil))
```

Inside the first `Eq.trans`, after hop 1, the `congrFun'`/`congrArg` chain is `simp`'s proof of `T = T'`: one use of `toy_relay.dag_denote` at the callee's literal environment, carried under `Prod.fst`, `broadcast_closed sched.net`, `values`, `union` and `HList.cons` (`simp` first reduced hop 1's tuple `(D.1, D.2.1).1` to `D.1`, a definitional step that leaves nothing in the proof). `T'`, the argument of `Eq.refl`, still reads the callee's input as `(HList.cons (env.get Var.zero) HList.nil).get Var.zero`, while the right side is `toy_step (SchedSem …) …` folded; the kernel reduces the `get`, unfolds `toy_step` once, and finds the same term on both sides, with `toy_relay (SchedSem …)` folded on each.

At no step does any checker open `toy_relay`, its graph or anything below them. A module deeper in the call tree pays the same: its hop 1 sees its callees' `Dag.denote` applications on both sides, and its hop 2 cites its callees' `dag_denote`, which were proved the same way when those callees were defined. [How Proofs Are Generated](../proof-generation.md#the-chain-module-and-the-two-hop-dag_denote) follows the same two hops on `toy_loop.body`, whose single node is a call of `toy_step.dag`.

When hop 1 is refused, `HYDRO_PHASE_DIAG=1` re-judges each output component separately and prints `[diag] M graphs component c: OK` or `REFUSED`, which points at the chain segment to inspect.

### Uses of `dag_denote`

`dag_denote` is the one per-module link between the graph and the program. `cq_safe_sched` carries `Dag.denote_causal` from `collect_quorum.dag` to `collect_quorum` through `collect_quorum.dag_denote`. The wired Paxos theorem gets the schedule `I.sched` from a byte-level trace, turns it into the program's decision and schedule records `sdec` and `ssched` by `recordOf`, and rewrites the graph's reading into the module (`Examples/Paxos/WiredSafety.lean`):

```lean
  have hS : paxos_core.schedOf I.pacing .guarded prop acc config sdec ssched = I.sched :=
    paxos_core.schedOf_recordOf I.pacing .guarded prop acc config I.sched
  …
    rw [← hS, paxos_core.dag_denote I.pacing .guarded prop acc config sdec ssched env, envRestrict_get]
```

## Proving `dag_valid` per Node

`Dag.valid` says that every knot block of the graph is **cascade-valid**: taking the knots in order, each one, with the earlier ones closed, is **strict** in its own wire (its value at a step depends on its own value only at earlier steps), so every cycle passes a network hop or a `defer_tick`. Validity is computed from a **lag** per wire, how many steps of input agreement a wire loses, which a hop or a `defer_tick` reduces by one. The definitions, the cascade and the theorems validity buys are on [Graph IR and Causality](../execution/graph-ir-causality.md#cascade-validity).

`Dag.valid` and `Dag.lagOf` are the specification only; nothing evaluates them on a production graph, because a kernel `rfl` of `leader_election`'s graph validity ran to gigabytes ([Validity is proved one node at a time](../execution/graph-ir-causality.md#proving-validity-per-node)). The phase calls `proveDagValid` (`Hydro/HydroGenLag.lean`) on the graph literal it just built. It computes every lag as a `Nat` in `MetaM` and proves each node by a per-node lemma whose inputs and results are literal `HList`s of numerals, so each kernel check is one node's rule on literals. A plain node's lag is the maximum over its variable arguments, one less at a strict head (`strictHeads`: `broadcast_closed`, `demux`, `defer_tick`); a `call` reuses the callee's own `N.dag_valid`; a `fixN` runs the cascade with knot `i` probed at lag `1` on its own wire and `0` elsewhere. A block that fails is a hard error, because its knot would not be its body's fixpoint:

```text
HydroGen: knot {i} of a knot block is not strict in its own wire once the earlier knots are closed (knot output lags {r} at its own wire at lag 1, the rest at 0) — a cycle without a network hop or `defer_tick`:{indentExpr body}
```

A single knot whose body lacks a hop usually never reaches this phase: the knot stack's strictness lemma rejects it while its `fix` elaborates ([Strictness](./knot-stack.md#strictness)).

## `dag_agreeL`

`Dag.denote_agreeL` is the lag law of any valid graph: environments that agree up to the lags `lags` (`HAgreeL h lags env env'`) give outputs that agree up to `Dag.lagOf d lags`. `M.dag_agreeL` says the same of the module's outputs at `env` and `env'`. Its proof is a term, `Dag.denote_agreeL` at `M.dag_valid`, transported twice along `M.dag_denote` by `Eq.ndrec`, and it costs milliseconds. Its consumer is the knot stack: `K_strict₁` instantiates the hoisted chain body's `dag_agreeL` at lag `1` on the loop wire and `0` elsewhere, with the output lags computed by the same per-node walk (`proveDagLag`), and reads strictness off the result ([The Knot Stack](./knot-stack.md#strictness)). That is why a chain body is projected before its knots' stacks run.

## Wire Codecs

A running process sends bytes, so each network node needs a payload codec: `Node.Codecs` (`Hydro/Project/Wire.lean`) is `Codec α` at a `broadcast_closed`, `Codec (Nat × α)` at a `demux`, the callee's or body's codecs at a `call` or `fixN`, and `Unit` elsewhere. `genWireCodecs` builds the value by walking the graph term: `Codec.ofWire` at each network node, from the payload type's `Wire` instance (`Hydro/Wire.lean`, which proves the round trip `dec (enc x) = .ok x`); the callee's own `C.wireCodecs` at a `call`; `()` elsewhere. A `demux` payload is encoded with its address first, so `demuxTarget_ofWire` reads the destination off the first varint. The module's type parameters (Rust's `P: Serialize + Deserialize`) become `[Wire X]` instance binders. `toy_step`'s:

```lean
fun {L} {mem} ℓ => (toy_relay.wireCodecs ℓ, Codec.ofWire, (), (), ())
```

When a payload type has no `Wire` instance, the phase skips the codecs and logs `hydro_project M: no wire codecs (…)`; only the byte-level harnesses need them ([Wire Codecs and Byte-Level Traces](../execution/wire-and-traces.md)).
