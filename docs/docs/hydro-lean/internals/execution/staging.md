---
sidebar_position: 6
title: Staging the Activation
---

# Staging the Activation

The byte-level trace theorem ([Wire Codecs and Byte-Level Traces](./wire-and-traces.md)) is stated for the loop body `Dag.iterateBytes`, which runs as its fused implementation `Dag.iterateBytesImpl` ([The Activation](./activation.md#the-fused-sweep)). That body is an **interpreter**: a function that receives the program's graph as a value and walks it at every activation. It dispatches on every node, rebuilds the typed environment at every `letN`, and recomputes the knownness masks and the knot rounds of the sweep, although every one of those depends on the graph alone and the graph never changes. The compiler cannot remove any of it, because to the compiler the graph is data. On the Paxos benchmark an empty activation cost about 450 µs at every role, almost all of it walking.

`hydro_run` (`Hydro/HydroStage.lean`, the **stager**) removes the walk by running the interpreter's control flow at elaboration time, on the closed graph, and emitting what is left: one straight-line function per program, with the operators applied to local variables and the program's closures as literal lambdas the compiler can inline. The kernel then checks that the result equals the interpreter (`Dag.iterateBytesImpl`, and through its `@[csimp]` equation `Dag.iterateBytes`), so the theorems keep meaning what they meant. The partial evaluation must not blow up, which rules out the obvious tools, and the output must be shaped so that the kernel's check and the compiler's front end both finish, which dictates what the evaluator may and may not reduce.

A **symbolic executor** here is a `MetaM` program that evaluates the interpreter over a mix of literal data (the graph, the masks, the wire numbers) and free variables (the cells, the inputs, the bytes), deciding at elaboration time everything that depends only on the literal part.

## Approaches That Fail

| approach | what happens |
|---|---|
| a `@[csimp]` seam from `Dag.iterateBytes` to the staged code | impossible: a seam replaces one constant by another without inspecting arguments, so it cannot specialize at one graph |
| partial evaluation of `Dag.sweepF` by `simp`/`dsimp` | a conditional on structure the simplifier cannot decide duplicates its continuation; the 4-node toy exceeded 4 GB |
| unification of the interpreter against a schematic term (`Meta.isDefEq`) | 46 s on the toy's mask, an equation the kernel judges in 78 ms |
| a flattened `Dag` with the knots as plain lets, still interpreted | one walk, but still an interpreter, and `M.dag_denote` (the generated theorem that the graph's `Dag.denote` reading equals the program's `SchedSem` run) stops being a definitional equation because the graph changed |

The third row is an instance of a general rule of the framework, developed in [Elaboration and Kernel Cost](../cost-model.md): judge large terms with the kernel, not with the elaborator's unifier.

## The `hydro_run` Command

`paxos_safe_wired`'s body is staged at any cluster sizes, payload type and configuration (`Examples/Paxos/WiredSafety.lean`):

```lean
hydro_run paxosRun {L : Type} {mem : L → Nat} [DecidableEq L] (P : Type) [DecidableEq P] [Wire P]
    (prop acc : L) (config : PaxosConfig) (m : Member L mem) where
  member := m,
  graph := paxosDag (mem := mem) (P := P) prop acc config,
  codecs := paxosCodecs (mem := mem) (P := P) prop acc config
```

It adds `paxosRun binders tk r chans Δ s` (with `r : TickReadings m`, the clock and timer readings of the shell, the executable that owns the sockets), compiles it, and adds two theorems:

- `paxosRun.eq_impl : paxosRun … = Dag.iterateBytesImpl m tk d c (Dag.Timing.ofReadings m r d) chans Δ s`, by the kernel;
- `paxosRun.eq : paxosRun … = Dag.iterateBytes …`, by `congrFun` over `Dag.iterateBytes_eq_impl`.

The member `m` stays a variable, and so do `L`, `mem` and `P`: the graph literal's shape does not depend on them, so the stager can decide the control flow, and the data conditionals stay `ite`s in the output.

## The Symbolic Executor

### Bookkeeping for the Sweep's Parts

`buildDag`/`buildNode` walk the graph literal once and produce one `NodeSt` per `letN`: the node, its cell and codec as projections of the cells and codecs variables, its timing as `Node.Timing.ofReadings m r n` (so no per-activation timing walk remains), its wire number, and the sweep's state for it (phase `todo`/`released`/`done`, the stored outputs and sends as expressions, whether it has been visited, and for a call or a knot block its sub-nodes and knot masks). Each wire of the environment is a `Wire`:

```lean
structure Wire where
  kind : Expr
  val : Option Expr
  known : Bool
  deriving Inhabited
```

`val` is the let-bound variable holding the wire's increment once it is known. A skipped node's outputs are wires with `val := none`; there is no filler expression to build.

### Running the Sweep

`sweepDag`/`sweepNode`/`knotLoop` run the fused sweep's control flow at the meta level, decision for decision as `Dag.sweepF` makes it: `enq` is "not yet visited", a `done` node yields its stored wires, a `released` node is pushed once its inputs are known, and the knot loop re-sweeps a body until no knot becomes newly bound or the fuel `ks.size` runs out. The `todo` arm is where code is emitted:

```lean
    | .todo =>
      withCell fun cell => do
        let st := st.withCell cell
        if known || isStrict ctor then
          let ora ← mkAppM ``Node.oracleOf #[c.m, st.node, st.tm, cell]
          bindRed c (Name.mkSimple s!"{nm}_ora") ora fun oraV => do
            let fn := if known then ``Node.locStep else ``Node.release
            let r ← withEnvVar c st.ctxKinds env fun Δv => mkAppM fn #[c.m, c.tk, st.node, oraV, Δv, cell]
            bindResult c nm st.kinds r fun stV outVs sndV => do
              let ms ← mkAppM ``Node.memoState #[c.m, st.node, stV]
              bindRed c (Name.mkSimple s!"{nm}_mst") ms fun mstV => do
                let st := (((st.withPhase (if known then .done else .released)).withCell mstV).withOuts outVs).withSnd (some sndV)
                k st (knownWires st ++ env)
        else
          -- skipped this sweep: fillers nobody reads
          k st (st.kinds.map (fun κ => ({ kind := κ, val := none, known := false } : Wire)) ++ env)
```

`known` is computed from the `Wire`s at elaboration time, so the branch is decided before any code exists. The executor is written in continuation-passing style (`k`) because each emitted step is a `let` whose body is the rest of the program: `bindRed`/`bindLet` open a `withLetDecl` and the continuation runs inside it.

### Applying a Node's Step to Symbolic Wires

A node's step (`Node.locStep`, `Node.release`, `Node.pushStep`) reads its inputs out of an environment `HList`. `withEnvVar` applies the step to a fresh variable `Δenv` of the node's context type, reduces, and replaces every read of `Δenv` by the variable of the wire it refers to:

```lean
def substEnv (Δv : Expr) (env : Array Wire) (e : Expr) : MetaM Expr := do
  let read (i : Nat) : MetaM Expr := do
    match env[i]!.val with
    | some w => return w
    | none => throwError "hydro_run: a stepped node reads an unknown wire (index {i})"
  Meta.transform e (pre := fun e => do
    if e.isAppOfArity ``HList.get 7 && e.getArg! 5 == Δv then
      return .done (← read (← varIdx (e.getArg! 6)))
    else match projIdx Δv e with
      | some i => return .done (← read i)
      | none => return .continue)
```

A read appears either as `HList.get Δv v` with `v` a literal or, after reduction, as a projection chain `Δv.2.2….1`; `projIdx` recovers the index from the chain's depth. The error enforces, at the meta level, the sweep's invariant that no stepped node reads a filler. If `Δenv` survives the substitution, the stager stops with `the environment did not reduce away`.

`bindResult` destructures the reduced result `(cells, outputs, sends)` into lets: the arm's own `let`s are kept as `let`s (they carry the arm's sharing), each output goes through `memo` (an identity function in the logic that compiled code materializes as a table; [Processes](./processes.md)) as `memoEnv` does, and `bindRed` emits no `let` for a variable, a constant or a value of type `Unit` (a stateless node's cells, the sends of a node that sends nothing).

## Controlled Reduction

Reduction is by the stager's own `reduceHead`, which unfolds only a whitelist:

```lean
partial def reduceHead (e : Expr) : MetaM Expr := do
  -- a projection: reduce the structure by this procedure (never `whnf` with delta, whose
  -- default would unfold anything to reach a constructor), then project
  if let .proj _ i c := e then
    let c' ← reduceHead c
    match ← projectCore? c' i with
    | some v => return ← reduceHead v
    | none => return if c' == c then e else e.updateProj! c'
  -- no ζ (the lets form the program), no projection-with-delta (handled above)
  let e ← withConfig (fun c => { c with zeta := false, zetaDelta := false, proj := .yes }) (whnfCore e)
  if let .proj .. := e then return ← reduceHead e
  match e.getAppFn with
  | .const n _ =>
    if unfoldable.contains n then
      match ← unfoldDefinition? e with
      | some e' => reduceHead e'
      | none => return e
    else if n == ``ite || n == ``dite then
      match ← decideIte e with
      | some e' => reduceHead e'
      | none => return e
    else return e
  | _ => return e
```

`unfoldable` lists the interpreter's node-level functions (`Node.locStep`, `Node.release`, `Node.pushStep`, `Node.memoState`, `Node.enqueueInbox`, `Node.inboxOfWire`, `Node.oracleOf`, `Node.encodeMsgs`, …), the `HList` and `Prod` plumbing, and `typeLevel`. Each exclusion below prevents a measured blowup:

| never | because | measured |
|---|---|---|
| `whnf` with δ to reach a constructor through a projection | `whnfCore`'s default projection mode unfolds anything; a cell is `(Node.memoStateBox m n st).1` with `st` a 100-deep `Prod.snd` chain | 0.1–0.7 s per node, growing with the node's position; the cells' and codecs' projections are therefore primitive `Expr.proj`s and reduced by `reduceHead` |
| unfold over a nested unreduced argument | smart unfolding reduces its discriminants, so an inbox inside an enqueue inside a memo is reduced three times | 2.8 s for twelve nodes; the inbox, the enqueue and the memo are each a `let` first |
| leave a type-level function folded | the compiler's front end (`toLCNF`) infers and `whnf`s the type of every subterm and never caches a type over a free variable, so a folded `Dag.LocState m (D cfg)` is re-walked per projection | compile 46 → 6 s once `typeLevel` is unfolded in the lets' types, the arms' implicit arguments and binder types, and the cells and codecs are lets at their reduced types |
| unfold a `let`-bound variable | the lets bind the program's earlier steps; re-evaluating one symbolically under a condition on symbolic inputs goes nowhere (`whnfNZ`: `zetaDelta := false`) | |
| unfold `ite`/`dite` | `Decidable.rec` does not compile; `decideIte` reduces the instance and takes the branch when it is a constructor, otherwise leaves the `ite` | |
| unfold `memo`, `memo2`, `memoBox` | they are the sharing seams: the kernel sees through them, and the compiler materializes them ([Processes](./processes.md)) | |

`typeLevel` lists the type-level functions:

```lean
def typeLevel : Array Name := #[
  ``Node.LocState, ``Dag.LocState, ``Node.Codecs, ``Dag.Codecs, ``Node.Oracle, ``Dag.Oracle,
  ``Node.Sends, ``Dag.Sends, ``Node.Inbox, ``Dag.Inbox, ``Node.Timing, ``Dag.Timing,
  ``StateBox, ``Kind.locDelta]
```

On `paxos_bench` the stage takes 17 s without the first two rules and 7.7 s with them; unfolding the type-level functions everywhere costs it 1.7 s more (9.4 s) and saves 40 s of compilation.

## Assembly and the Kernel Bridge

The result is `(cells, outs, msgs, tails)`. `cellsOf` rebuilds the nested cell tuple from the final `NodeSt`s. `msgsOf` concatenates the network nodes' encoded sends with the fused sweep's exact nesting, a right-nested `nodeMsgs n ++ (… ++ [])` per graph with a sub-graph's chain spliced in as one element, because `List.append` is not definitionally associative on symbolic lists and the kernel compares the two sides term for term. `ShareCommon.shareCommon'` then restores the sharing the unfoldings lost (71k → 24k objects on the benchmark), since both the compiler's and the kernel's traversals cache by pointer.

The bridge is one kernel judgement. `addThmByKernelRfl` (`Hydro/HydroGen.lean`, shown in [Generator Overview](../generator/overview.md#addthmbykernelrfl)) builds `Eq.refl lhs` for the statement `paxosRun.eq_impl` and hands it to the kernel alone. It has no fallback: on a refusal it returns `false`, and `hydro_run` fails with ``hydro_run: the kernel refused `paxosRun.eq_impl` ``.

The kernel normalizes `Dag.iterateBytesImpl` on the closed graph and zeta-reduces the staged lets. The right-hand side of `eq_impl` is the implementation, not `Dag.iterateBytes`, because the fused sweep is what the stager mirrored: the separate walks equal it only propositionally (`Dag.sweepF_eq` is proved by induction over the graph). `name.eq` is then a `congrFun` chain over `Dag.iterateBytes_eq_impl`, which needs no kernel evaluation:

```lean
          let mut h2 : Expr := Lean.mkConst ``Dag.iterateBytes_eq_impl
          for a in implArgs do
            h2 ← mkCongrFun h2 a
          mkEqTrans h1 (← mkEqSymm h2))
```

## Phases and Budgets

The command does the work of three declarations: the symbolic execution, the compilation, the kernel bridge. Each runs under its own heartbeat budget (`withCurrHeartbeats`, what Lean gives each declaration of a block), none overridden, and the command reports all three:

```lean
        logInfo m!"hydro_run {name}: {nNodes} top-level nodes, {wireN} wire nodes, {(← lets.get).size} lets; stage {t1 - t0} ms ({(hb1 - hb0) / 1000}k heartbeats), compile {t2 - t1} ms ({(hb2 - hb1) / 1000}k), kernel {t3 - t2} ms ({(hb3 - hb2) / 1000}k)"
```

Measured (synchronously, `-DElab.async=false`; asynchronously the kernel phase prints as about 1 ms because it has not run yet):

| staged function | lets | stage | compile | kernel |
|---|---|---|---|---|
| `paxosRun` | 364 | 4.9 s, 74 M | 3.3 s, 50 M | 12.4 s, 103 M |
| `BenchDemo.benchRun` | 668 | 9.4 s, 149 M | 6 s, 110 M | 20 s, 165 M |

A phase above 200 M heartbeats is a blowup to diagnose, not a budget to raise ([The Gate](https://github.com/hydro-project/hydro/blob/main/hydro_lean/internal/GATE.md)). The kernel phase is also where the memory peaks (about 4 GB for the bridge alone).

## Results and Limits

The empty activation at the Paxos benchmark's leader is 54 µs compiled against 462 µs for the interpreter on the same cells; the other roles are 35–48 µs. What remains is about 22 µs of fixed cost plus a quarter of a microsecond per node at the process's location.

The headline is stated over the staged function itself. `paxosBody` is `paxosRun` as a `WireTrace.Body` ([Wire Codecs and Byte-Level Traces](./wire-and-traces.md)), `lake exe wired` calls `paxosBody`, and `paxos_safe_wired` quantifies over `τ.streamOutputsAt (paxosBody …)`. The proof uses the equation, so this step does not rest on the compiler:

```lean
theorem paxosBody_eq [Wire P] :
    paxosBody (mem := mem) (P := P) prop acc config
      = WireTrace.Body.ofCodecs (paxosCodecs (mem := mem) (P := P) prop acc config) := by
  funext m r chans Δ s
  exact paxosRun.eq (mem := mem) P prop acc config m true r chans Δ s
```

`paxosRun.eq` is a statement about the logic. That the compiled `paxosRun` computes what the compiled interpreter computes also depends on the compiler and the memo seams inside both, so `Hydro/HydroStageCheck.lean` stages the toy knot program `toy_loop` and `#guard`s, over two activations on data, that the staged function and `Dag.iterateBytes` return the same outputs, sends and tails; `lake exe wired --traces` does the same for `paxosRun` on the scripted traces.

The member stays symbolic, so one staged function serves every role, and a node at another location still costs a `decEq` and a closure allocation (about 5 µs of a loaded leader step of about 700 µs). Specializing per role would need a propositional bridge (rewriting functions over empty `Hosted` domains through every let), at five times the kernel time, for about 1 %.
