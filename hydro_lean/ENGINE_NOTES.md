# Engine notes — the elaboration gotchas canon

For anyone touching `HydroDef.lean`, `HydroGen.lean`, `HydroGenKnot.lean`,
`HydroTick.lean`, `HydroGenProject.lean`, or writing proofs inside a
`hydro def`. Each item was learned the expensive way; the D-number is the `FINDINGS.md`
entry with the full story. House method for new ones: **toy-repro for wrong-answer
bugs, `trace.profiler` for slow-answer bugs** — never a budget bump (D56, D63).

## Reading wires and faces

- **`simp only [<wire lets>, den]`** reads a chain of stream ops at the denotation:
  naming the let-bound wires zeta-delta-unfolds exactly those, then the `den` `rfl`
  readers fire. `simp only [den]` alone makes no progress on an alias (D66 v, D67 viii).
  A `tick` block's `_at` reader states its input reads against the block's **input
  aliases** (`(bb i)[n]?` for `(input bb := b)`): read it as
  `simp only [bb, b, <upstream lets>, den]`.
- **`simp` cannot see through local `let` fvars**; the unifier can (it unfolds let
  fvars and `Values` projections). When `simp only` stalls on a let-bound wire, state the
  fact as a pure theorem over traces and let `exact`'s defeq bridge, or use
  `simp (config := { zetaDelta := true })` (D53).
- **`rw` is blind across instance-projected or construct-shaped types** vs their plain
  spelling (`(Values).BoundedSingleton τ` vs `τ`, `BoundedStream` vs `Multiset`, a
  generated step tuple vs `(a, b)`): use `exact`/`Eq.trans`/`Iff.trans` (defeq), and
  `@id τ e` rather than `show τ from e` (the latter makes a `letFun` that blocks `rw`)
  (D63 ii). Construct-step projections have type `(Values).BoundedSingleton τ`, which
  typeclass resolution does not see through (DiscrTree keys don't delta projections):
  state step facts over `@id τ (step i s x).2.k` (D64 ii).
- **The lazy-delta hotspot** (D63 vii): `rfl`/`show` across a boundary operator
  (`values (broadcast_closed … (map (allTicks w) f)) j = …`) can cost seconds per check
  because the unifier unfolds the side with the lower head (`Multiset.map → Quot.lift`,
  four deep) before whnf-ing the folded `Values` projection on the other. Fix: one-step
  `rfl` readers at the semantics (`ValuesTick.values_*`, the `den` set), never a
  definitional `show` of the folded form. Same family: never `show` the zipped
  `TickShape` form of a `tick_scan` (the `TickShape.rec` cliff, D60).
- **Raw `Multiset.sum` membership** in a hypothesis that gets whnf'd repeatedly
  (`((slot,b),v) ∈ (w i).sum`) is a 0.4 s `Non-easy whnf` per use; give the predicate a
  `def` head (D64).
- **`obtain ⟨-, …⟩` with `-` on an ∃-witness** clears every hypothesis depending on it,
  silently — the later "unknown identifier" is the only symptom; name the witness
  (`⟨_ls, -, h, …⟩`) (D64 i, D63 v). `obtain ⟨…⟩ := h` clears `h`; use `:= id h` if the
  packed fact is needed again (D58).
- `obtain rfl := Trace.read_inj h₁ h₂` on two reads of the same tick identifies a face's
  witness with a program read — the house replacement for `List.getElem_of_eq` casts
  (D66 vii).
- Option-indexed reads (`(w i)[n]? = some x`) over dependent-index reads (`(w i)[n]'h`):
  the bound plumbing (3–8 lines per read) is the single largest source of proof mass
  (D63 taxonomy). Generic readers: `Trace.getElem?_zip_eq_some`, `getElem?_map_eq_some`,
  `prefixCuts_getElem?_prefix/_mono`, `scanAcrossTicksTrace_getElem?`.
- An untyped binder whose only use is a projection (`x.2.1.1.num`), or an untyped
  `∀ u gu, (w i)[u]? = some gu`, leaves a metavariable / stalls `GetElem?` resolution and
  surfaces as an unrelated error downstream — type the binders (D62 v, D63 vi).

## Inside `fix` / `tick` obligations

- `subst` cannot eliminate an fvar that let-binder **values** depend on: inside
  invariant obligations keep `variant` generic, `rw [hvar]` only at emission-typed
  hypotheses, pass `hvar` to the guarded face fields (D58).
- Tactic `intro` **consumes the obligation's let binders** (term-mode `fun` skips them):
  intro the run names first (D58).
- An un-ascribed `have := <generated mono> … (fun _ => PoolLe.refl _ _ _)` leaves the
  diagonal-input implicits unsolved — pin them by name
  (`(p_received_p2b_ballots := …)`) (D58).
- `rw` with generated equations cannot see through `set`-introduced fvars (the D53
  zeta gotcha replayed) — term-level `congrFun`/`exact` bridges (D57).
- `⟨…, fun h => by cases h, …⟩`: a `by` block inside an anonymous constructor swallows
  the following `, …` as tactic arguments — parenthesize `(fun h => by cases h)` (D62 i).
- `rw [List.getElem_map]` after `unfold f` fails ("motive is not type correct") because
  the index proof no longer matches the unfolded list — re-derive the bound first
  (D62 ii). `rw … at hv hq` where `hq` depends on `hv`: if the two spellings are defeq
  don't rewrite, pass the hypothesis as is (D62 iii).
- `subst h` with `h : a = b` eliminates `b`; orient so the name you keep is on the left
  (D62 iv).
- Two `match` sites on the same scrutinee generate different matcher constants — `omega`
  sees different atoms; destructure once or `change` (D63 iv). `omega` also atomizes
  syntactically distinct-but-defeq terms — ascribe into one spelling first (D25).
- `simp only [step, den]` leaves `match some b with …` as projections; `simp only []`
  after `cases` reduces the match; a composed closure is defeq to the intended one by
  structure eta — `exact` works, `rw` does not (D64 iii).
- `rfl` after a `rw` that already closed the goal is "no goals" — construct step proofs
  are often closed by the rewrite itself (D64 vii).
- `Bool.and_eq_true` is an `Eq` of Props (simp lemma), not an `Iff` (D66 ii). The `<+`
  Sublist notation is not in scope — spell `List.Sublist` (D66 vi). `cases hres : m.res`
  rewrites the goal too; re-anchor with `show … ↔ _` (D66 iii).
- The capped keyed fold's closed form needs `max cap 1` (a fresh key always opens a
  singleton bucket) (D66 i). `List.argmax` needs `import Mathlib.Data.List.MinMax`;
  `argmax_mem`/`le_of_mem_argmax` take `Option` membership (D66 iv).

## Generated artifacts and their shapes

- Single-leg modules get unsubscripted artifact names (`M_mono`, `M_co_wf`); multi-leg get
  `M_mono₁…` (D67 ix).
- A `tick invariant` block in a `hydro def` **without `ensures` has dead prove legs**:
  ghosts replay only in the proof leg, which exists only under `ensures`/`prove`; Lean's
  "this tactic is never executed" is the only symptom. Every invariant toy carries an
  `ensures` (D65 i).
- `M_co_rr_ex`/`M_vdec` name the derived decision **by choice** (`Classical.choose`), so
  a `change` of the `Values` run at the derived chain into the corner's `rr` leg is not
  definitional. The route: `have hrr : C.1.rr = (M Values … (fun i => derivedChain …)).1
  := by co_transfer [M]` at an **explicit, η-expanded** decision (`fun i => … (h i) …`,
  never `fun _ => … (h 0) …`) (D65 ii).
- `rw [famFreeze_of_mono hmono]` fails on a family whose members depend on `i` (mvars not
  HO patterns): `rw [famFreeze_of_mono]; swap; exact fun t i => …` (D65 iii).
- `batchCuts` **truncates the chain at the first illegal cut** (not the element): a
  record naming a message the program never sent ends the trace there; a `snap` is a
  per-tick consumption *count*, not a cursor position (D65 iv). (A diagnostic mode
  flagging the first illegal cut per member is queued.)
- `Trace.pairwise_reads honce huv (List.getElem?_zip_eq_some.mpr ⟨…⟩)`: the zip read's
  pair is not inferable from the components — state the two reads as `have`s with the
  pair spelled (D64 vi).
- `decide (x ∈ poolWeakenOrder m)` after `den` carries a `Decidable` instance over the
  unfolded form while the proposition reads `m`; generalize the instance and `exact`
  (D64 v).

## Writing elaborators (`HydroDef`/`HydroGen*`/`HydroTick`)

- **Whole-program defeq is the recurring failure class** (D40, D44): kernel defeq
  through `k` nested knots is exponential in `k`. Everything is per module; knots are
  hoisted (`<M>.<wire>`, folded `@[reducible]` bodies); namings assemble module-folded.
- **Key before unifying** (D49, D56): a *failing* `isDefEq` between two projections of
  the same folded module application (`(leader_election …).2.1` vs `.2.2.1`) is a
  whole-program whnf after all visible arguments unify. Any resolver that tries
  candidates by conclusion unification keys on head constant + projection path first
  (`matchLemma` at reducible transparency; `resolveRel`'s keyed pre-filter).
- `instantiateMVars` before dispatching on an expression's head (D46). Defeq-checked
  index pinning; backtracking `≤`-closers; `subjectHead` = beta + reducible unfold (D49).
- Positional context telescopes for phantom generics (D55): a module's anonymous
  generics (`{ckα ckord ckret}`) are threaded by position; re-telescoping a hoisted
  statement walks into the invariant's own binders — take args from the outer telescope
  only (D57).
- `mkObligation`'s value→binder rewrite must replace **largest terms first** — otherwise
  it silently drops obligation binders when two valuations share let values (D58).
- A mis-named tactic hypothesis in a generated obligation used to error-recover into a
  kernel-only failure with cascading whnf timeouts; the packaged inductions now carry a
  loud dead-fvar guard (D58).
- `co_simp`/`co_wf_simp` rule lists are `Array Name`s spliced by the macros — a literal
  quotation of the list hit `maxRecDepth` (D61). `HydroGenKnot.coSimpBase :=
  coSimpRules` (one list).
- When deleting a `HydroSem` field, remove its `vmono_<op>` leaf (`ValuesMono.lean`), its
  `causal_<op>`/`tmono_<op>` laws, the `Node` constructor and its cases in
  `Project/Model/*` (D67 x, D100: the `HRel.Laws` instances that used to shift by one
  are gone).
- Local `fun … => do` closures default their monad unpredictably — ascribe
  (`: … → TermElabM Expr`) (D58).
- The engine's own functions can outgrow the compiler's default budget — extract
  helpers to top level rather than add a budget (D59).
- Simp lemmas with an unused section-variable binder are silently skipped; metavariable
  pins typecheck at reducible transparency, so raw `()` at instance-typed positions blind
  the match (pre-fill `Unit` fields; route non-unit pins through `@[reducible]`
  wrappers); under-applied top-level defs do not simp-unfold (η-expand); `cases` on a
  horizon that parametrizes carrier types fails to generalize (hoist to standalone
  lemmas) (D37/D38).
- `simp` will not rewrite a module projection nested inside another module
  application's *argument* (dependently-typed position through the contract subtype) —
  the `rw`-fixpoint loop or the generated namings (D38).
- Naturality `have`s before witness mvars; `apply Exists.intro` for assignable holes;
  reducible decision wrappers; hoist inline `omega`; instance-projected types vs raw
  binders blind simp's assignment transparency (D37's five).
- Parenthesize every `try`; caps ascriptions on curried knot bodies; closed statements
  elaborate in an empty local context (open installed facts don't); `applyNamed` for
  name-supplied implicits; interleaved (not sequential) ex-scripts (D43/D44).
- Section hypotheses not mentioned in a statement need `include … in` (D41).
- **Structural, not per-arity** (D70): the projection of a shaped tick former's output
  along a `.1`/`.2` path is one simproc per interpretation (`eagTickScanProj`,
  `coTickScanProj`): strip the `Prod.fst/snd` chain, check the root is `tick_scan` at
  the interpretation, re-apply the chain to the other interpretation's former — a
  proof-free (definitional) `Simp.Step`, since `pack`/`CoTick.mk` are shape-recursive.
  The simp sets are `Array Name`s: `simpTerm` (HydroGenKnot) routes `Simp.isSimproc`
  names into a `Simprocs` set, theorems into `SimpTheorems`, other constants to unfold.
  Build the helpers' implicit prefix (`{L mem pacing Tc Td hjT ℓ}`) explicitly from the
  corner's arguments — `mkAppM` inference fails when a body's binder types are the raw
  corner carriers. Guard `e.isApp` before `appArg!` (a bare projection constant appears
  η-reduced). `shapeOfCarrierType` reads `AtomicOf`'s shape at `args[5]` (6 args:
  `{L} Tk TS TKS ℓ sh`) and `BoundedOf`'s at `args[3]`.
- **Inner-first rewriting** (D70): a composite `den` rule (`values (broadcast_closed d
  s) p = sum`) never fires once per-op rules rewrite its interior first — state the
  decode on the interior's normal form (`KeyedPool.values_transport`). Likewise a `wf`
  closer keyed on `(embed2 sh m (coerce sh m)).wf` is dead once `coerce_<leaf>` is in
  the set: state it on the rewritten form (`embed2_ksing_wf` + `eq_self_iff_true`).
- Phantom indices on `Values` carriers (`KeyedSingleton … ord ret`) must be passed
  explicitly at the corner (`(ord := ord) (ret := ret)`): the "don't know how to
  synthesize placeholder" cascade inside structure-instance fields is that (D70).
- **Sched follows DFIR** (user ruling, D70): a machine operator is DFIR's algorithm on
  DFIR's representation (assoc list = `HashMap` entries with the unique-keys invariant;
  `reduce_watermark` = the `fold::<'static>` of `compile/ir/mod.rs:5221`), not an
  output-equal spec. When the spec form is easier to couple, keep it as a proof device
  and prove the simulation (`EagerSim`/`scan_eager_eq_assoc`).
- The two causal dispatch tables (`SchedCausal.causalDispatch`, `WfTactics.coCausalDispatch`)
  must both carry a new op; a toy through every walk is the only oracle (D70). Since
  D71 there is a third walk, `_smono` (`coSMonoDispatch`/`hydro_smono_walk`), with the
  same rule: tick-family ops prove `TMono`, everything else `TMonoTriv.intro`.
- **`try` swallows the rest of a tactic sequence** (D71): `rw [a]; try rw [a]; exact h`
  parses as `try (rw [a]; exact h)` — a failing `rw` silently discards `exact h` and
  the goal "mysteriously" survives. Parenthesize: `(try rw [a]); exact h`. Found when a
  knot with NO wire inputs (its two `fixRw` sides identical) made the second rewrite a
  no-op in `genKnotCausal`.
- **A glue module whose body contains a `tick_scan` needs the tick former's simp set**
  (`coSimpBase`) in `genGlue` (D71): the former's simproc and its under-binder rewrites
  are unreachable by the keyed residual pass alone. Found by `toy_tk_order`.
- **A `fix` chain `let` used only AFTER `complete` is not captured** (D71, open): the
  closure analysis sees the post-`complete` tail as the enclosing module's, so the
  let's free variable escapes. House pattern: keep content inside the knot body. A glue
  module whose knot captures a record that ALSO carries a callee's content decisions
  mis-extracts the `vdec` when that callee is called after `complete` — same root.
- **Default-transparency `whnf` zeta-deltas a wire and delta-unfolds every callee
  module down to its tick formers** (D71, the SP ghost blowup): `elabAppArgs` on a
  hypothesis `p2a ∈ Multiset.map f (List.sum (payloads_to_send r))` spent 2.7 s per
  `obtain` (161M raw heartbeats on one fact). Fix in the engine: every registered module
  not prefixed by the current one is set `[irreducible]` while a ghost fact elaborates
  (restored in `finally`); a ghost that legitimately reads a callee body
  (`leader_election`'s `MonoRel` instantiation) says `with_unfolding_all`. The
  "opaque-first lets" alternative was tried and reverted: with zeta blocked the
  unifier delta-unfolds the callees instead — slower. Diagnose with `trace.profiler`
  and `HYDRO_GHOST_DIAG=1` (per-fact raw heartbeats).
- **Heartbeats accumulate per command** (D71, D72): every declaration a `hydro def`
  generates — knot-stack lemmas, per-leg couplings, tick monotonicity, wf
  threading, the graph and its lemmas, hoisted ghost facts, the `invariant` induction — counts
  against the ONE command, as Lean counts them; `leader_election`'s def-elaboration sums
  to ≈500M raw with no single theorem above 90M, and that sum is its 600k override.
  Heartbeats are a proxy, proof TIME is the goal (user ruling, D72): an override is a
  measured fact to report with its table, and the question to ask of a large one is
  "which declaration is slow, and why" — `HYDROGEN_HB_DIAG=1` prints each one's cost
  (`[hb] <name>: <k> <ms>`, `cmd-used=` the running command total),
  `scripts/heartbeat_table.sh <file>` sorts them. (D72 tried per-declaration
  accounting — `withCurrHeartbeats` plus resetting the counter on exit — which would have
  deleted both overrides; dropped on the user's ruling: it moves the number, not the
  time.)
- **A failing `apply` against a deep shared projection spine is exponential** (D72):
  `with_reducible apply body_smono₁₄` on a goal whose subject is `(body …).2^14.1` takes
  7 s to FAIL (`₁₂` 2.4 s, `₁₀` 0.8 s — ~1.7× per shared `Prod.snd` level): the unifier
  learns the mismatch at the bottom of the spine and retries every level in its `.proj`
  form (`Prod.snd` is reducible) on the way back up; failures are not cached while the
  rule side has metavariables. Every `first | with_reducible apply r₁ | … | apply rₙ`
  over callee rules pays this for every rule before the right one (the deepest leg pays
  most: `leader_election_smono₃` 240M, the since-deleted `_causal₃` 204M). Rule: **select
  callee rules by key, never by trial unification** — `hydro_rule_apply [rules]` (the smono
  walk and the wf closers) skips a rule whose conclusion key (head constant + literal
  projection path, modulo `@[reducible]` heads such as `<K>.body`, projection-function
  vs `.proj` spelling, and the goal's relation at reducible transparency — `TAgreeOf h
  (.stream …) x y` IS `TAgree h x y`) differs from the goal's. `smono₃` 240M → 0.2M. Two traps met building
  it: `forallMetaTelescopeReducing` unfolds the conclusion too (`TMono x` became its
  `List.IsPrefix` body — use the non-reducing `forallMetaTelescope`), and a rule's
  conclusion head may be a reducible alias of the goal's (`toy_loop.w.body` vs
  `toy_loop.body`'s leg — key modulo reducible unfolding, `subjKeyOfM`). The toys
  caught both.
- **Deep `Prod.snd` spines and the unifier's same-head heuristic** (D73): `MemberMap.get`
  is a `def`, not the structure projection `cells`. For projection functions `isDefEq`
  skips `isDefEqArgs` and goes to `.proj` nodes; a stuck projection then `whnf`s its
  structure argument WITH delta — the whole program upstream of the wire. One `exact`
  in a knot's `wf` cost 34M heartbeats (+50 s on `LeaderElection`) with `get` as the
  projection; through the `def`, two reads of the same wire compare carriers
  structurally first.
- **`conclKeyMismatch` and operator-headed subjects** (D73): a callee-leg lemma
  (`leader_election_mono₂`) tried against a subject headed by an OPERATOR
  (`mapTick (zipTick (le.1) …)` in `paxos_core.body`) unified for 403M heartbeats before
  failing. The walks now skip a callee rule whose subject is `Hydro.HydroSem.*`-headed;
  `PaxosCore`'s def elaboration 50 s → 17 s and its `maxHeartbeats` override is gone.
- **Class fields are not reducible** (D73): `DiscrTree.getUnify` runs `withReducible`,
  so an instance keyed on `MemberMap n C` is never found for a term of type
  `(Values L mem).Stream …`. Hence the per-family `CoeFun` instances in `Values.lean`
  (`instCoeFunValuesStream`, …) rather than one on `MemberMap`. Marking the class
  fields `@[reducible]` was tried and reverted: it broke the `tick` elaborator's wire
  classification for abstract `H`.

- **Per-call simp-set construction was the class-size tax** (D81): `co_simp`/`co_wf_simp`
  as macros splicing ~300 names rebuilt the simp set on every call, and `simpTerm` did the
  same with `addSimpTheorem` loops — so every field added to `HydroSem` taxed every module
  (LE +12 s, PaxosCore +50 %). Registered simp attributes (`Hydro/SimpAttr.lean`
  `co_simp_rules`/`co_wf_simp_rules`, loaded once by `CoupleProj.registerCoRules`) and
  `simpTerm` starting from `ext.getTheorems`: full build 4m50s → 2m51s. A new op is one
  entry in `coSimpRules`/`coWfSimpRules` (the `registerCoRules` command picks it up).
- **Large meta elaborators hit the LCNF compile budget** (D81): the tick elaborator's one
  function stopped compiling when the in-tick vocabulary grew; split into helper defs
  (`parseTickState`, `tickInputShape`, `mkStepLam`). No `set_option` — split.
- **Glue mono over operator-headed legs** (D81): `resolveRel` resolves a derived wire by the
  operator's `vmono_<op>` law (`opHeadOf` finds the op through `MemberMap.get`,
  `Subtype.val` and `.proj`; HydroGenKnot imports `ValuesMono`), hypotheses recursively, depth
  16. Legs that are not callee legs used to be silent placeholders — a consumer found out
  only when its knot asked (`paxos_core_mono₁` did not exist until `with_client`'s knot
  needed it). Diag: `HYDROGEN_MONO_DIAG=1` prints each goal with its op candidates.
- **`hydro_wf_close` tries a callee lemma only when the callee HEADS the subject** (D81).
  `find?` (the callee anywhere) made it `apply` `kv_replica_co_wf₁` to `(collect_quorum (…
  kv_replica …)).1.wf`: two module applications unified through their knots — 3.6 G
  heartbeats / 318 s for one leg; the head rule: 8.6 M. `HYDROGEN_WF_DIAG=1` (the
  fallback-free script) would have shown the leg closing fast — the bomb was in a
  fallback alternative, which is the general lesson: when the diag mode is fast and the
  real mode is slow, bisect the fallback list.
- **`kabstract` at default transparency inside the knot generator** (D81): `rwRhs`
  (`kabstract rhs c`) tries `isDefEq` on every same-headed subterm; with `.rr`
  projections of many module applications in `rhs`, one call cost 419 M heartbeats (36 s)
  in the outer knot's `wf`. The occurrence is syntactic: `withReducible`. Instrumenting:
  `withDeclDiag` around the pieces — and lift `(← e)` out of `withDeclDiag "…" <| f (← e)`,
  or the measured piece is not the one you think.
- **Dead chain lets** (D81): pass 2 of a single-source `fix` re-spelled a body `let` only
  the wire closing consumed as a `have` re-applying its callee (postponed elaboration
  hides the deadness until mvars are instantiated — check AFTER `instantiateMVars`:
  `dropDeadLets`). The symptom was the production graph carrying a second callee node
  and `schedOf_recordOf` refused by the kernel.
- **Knot binders come from the RAW body** (D81): the fvar analysis read the zeta-reduced
  wire-closing body, so a binder only a continuation-kept chain `let` mentions
  (`paxos_bench`'s `aggregator`) escaped the hoist as a free variable.
- **Nested `fix`** (D81): chain bodies are named `<M>.body` / `<M>.<wire>.body` (no
  collision), but the inner hoist still takes the def's binders as its context, so an
  inner fix inside a knot body does not elaborate; write one flat knot per module
  (`paxos_protocol` holds the checkpoint knot, `paxos_bench` the client cycles).
- **Glue modules' own decisions** (D81): a knot body's content decisions are consumed
  through callees — a bare `OrderSelDec` field a callee takes is matched by type
  (`collectValuesDecArgs`/`extractGlueVdec`); an in-body `assume_ordering` on a knot body
  is not (factor it into a callee: `tag_with_sender`), and a knot's continuation carrying
  decisions is not (the module-level vdec would have to be field-wise where the knot
  consumes the record whole: queued — today the measurement lines of `paxos_bench` sit
  inside its knot body).
- **`addThmByKernelRfl` judges once** (D81): the `Kernel.isDefEqGuarded` pre-check plus
  `addDecl` = two kernel checks of the same `Eq.refl`; a failed `addDecl` throws and
  leaves the environment unchanged, so `try addThm … catch _ => false` is the check.

### The graded coupling's lag (D95)

- A `Nat` that sits inside horizons (`CoupleSem … (j + L) …`) is evaluated by the
  elaborator's `Nat` defeq on every horizon comparison (`Tc =?= j + L` tries `whnf` to a
  literal). `CoStream.fixLag`/`CoTickSing.fixLag` are `@[irreducible]` for that reason, and
  the generator names each knot's lag (`K_lag binders`) so the lowered corner carries a
  constant application, not the body (an open spelling made the lowered types grow
  multiplicatively through nested knots and `rw` never finished).
- `rw … at h` over a lowered body application is a default-transparency `kabstract` — on
  `leader_election`'s nested knots it whnfs the module through its knots (the D81 lesson
  again). The knot's coupling fact is transported by `Eq.mp` over a motive abstracted at
  REDUCIBLE transparency (`fFOUT/transport`); the script is `intro …; exact FOUT …`.
- `omega` is exponential in the number of `max`es (a case split each): the knot's side
  condition `j + Λ ≤ j + L` cost 105M heartbeats on `p1b_fail` and never finished on
  `i_am_leader`. `co_lag_le` decides it leaf-wise (`Nat.max_le`/`le_max_iff` after pushing
  the subtractions to the leaves) — 0.1–1M.
- AC normalization of `max` trees with `simp [Nat.max_comm, Nat.max_left_comm, …]` is
  super-linear (24M on a 20-leaf tree, a timeout at 50). `lagCanon` builds the sorted
  one-leaf-per-atom normal form directly and proves `E = C` by `Nat.le_antisymm` over
  leaf-wise `max_le` splits and `le_max` paths (a term of size `|E|·|C|`).
- Never run `simp` over a module APPLICATION to normalize one projection of it: the walk
  over its arguments (records, proofs, the Kleene iterate) is what costs (24M for
  `p_is_leader`'s lag fact, a timeout at the lowered corner). `lagFactAt` rewrites the body's
  lag naming with per-argument equations (`lagEqOf`: a lowering's is its wire's, a `mkC`'s
  its lag, an inner knot's its own naming) and normalizes the small arithmetic that remains.
- A simp lemma with a repeated metavariable (`max ?a ?a`, `?a ≤ ?a`) makes `simp` run
  `isDefEq` on every pair of candidates — keep such lemmas out of sets that walk terms
  containing module projections. The leaf lags (inputs, probes, `mkC`, lowerings) are a
  definitional pre-simproc (`coLagLeaf`): raw-typed legs (the knot's diagonal under a probe)
  never meet a lemma's binder types, which `simp` unifies at `instances` transparency.
- A pretty-printer inside a diagnostic tick's `s!` string runs whether or not the
  diagnostic is on (the string is built before the guard) — 13 s of `leader_election`'s
  wall time. Diagnostic labels are thunks.
- `lake build` replays a target's stored output when its dependencies' oleans are
  unchanged — identical heartbeat numbers across "runs" mean a replay, not a measurement;
  `lake env lean <file>` always elaborates.

### The kernel and knots (D95, stages 2 and 3)

The ungated knot made every kernel judgement that touches a knot a potential evaluation of
the whole program. The rules that hold the production files at ≈ 2.7 GB (the lakefile caps
every `lean` at `-M 4500` — 4000 until D98, see "Array batches" below and GATE §1; measure
with `/usr/bin/time -v lake env lean -M 4500 -DElab.async=false <file>` — synchronous kernel, so a failure names its declaration; a
baseline file needs ≈ 2.5 GB, anything above 3.5 is a regression, and a cap of 12 GB only
hides it):

- **The kernel never evaluates a module application, a lag analysis or a knot wire.** Three
  shapes did, each at 4–19 GB: (a) `Dag.valid`/`Dag.lagOf` of a production graph by `rfl`
  (call-by-name — every wire-lag read re-runs its producer; `brecOn` towers) → the
  generator computes the lags and proves them one node at a time against the `_lit`
  lemmas (`HydroGenLag.lean`: `nodeLag`/`dagLag`, `nodeValid`/`dagValid`, the cascade
  `evalPhi`); (b) a knot's `K_co_lag₁` by comparing `.lag` of two differently-headed module
  applications — the kernel falls into `whnf_core(cheap_proj := false)`, a full `whnf` of
  the chain → generic instance lemmas (`co_hfix_stream_fixLag`/`co_hfix_tick_fixLag`,
  `knotLagOpen`) whose instantiation is the fact; (c) tactic scripts for `K_strict₁`/
  `K_smono₁` → direct terms (`causalL_fix_*`/`tmono_fix_tick` applied to the body function
  `machineBodyFn`, `mkExpectedTypeHint` to the conclusion the kernel judges by unfolding
  `K`).
- **`stepGate`.** A tick knot applied at a symbolic step must be inert: `fix_tick … i t :=
  stepGate (fun t => (iterate body ⊥ (t + 1)) i t) t`, `stepGate` by cases on `t` (stuck for
  the kernel on a variable, `stepGate_eq` the identity; `fix_tick_apply`/`fix_tick_eq_diag`
  for proofs; `CoTickSing.fixSr` identically so `co_tick_fixSrC_eq` stays `rfl`). Without it
  `t + 1` is `Nat.succ t`, `iterate` unfolds once and the kernel evaluates the knot's whole
  body at every symbolic application; D94's pacing gate (`gateView (pacing.active …)`) had
  been stuck the same way by accident. `fix_stream` needs none — `famFreeze` recurses on
  the step. Found on `paxos_core.body.dag_denote_graphs` (hop 1 of the denotation lemma):
  the comparison is the same as before, but a kernel defeq that *fails* an optimistic
  sub-comparison (e.g. `Dag.denote PC.dag … =?= Dag.denote LE.dag …`, by design) unfolds
  both sides to normal forms before concluding, and those normal forms apply the knots.
  Reproduced minimally as "a knot-bearing callee's output feeding another callee with ≥ 3
  unrelated nodes between the calls" — the node count changes the kernel's path, not the
  terms. Under `HYDRO_PHASE_DIAG=1` a refused graphs lemma is re-tried per output
  component (`[diag] M graphs component c: OK/REFUSED`) to name the segment.
- **`Meta.isDefEq` on a module application explodes too** (`whnf` timeouts in `glue`/`wf`
  at 200k): never let a generated statement put two spellings of a module output on the
  two sides of a unification; name the output (`let`/`have`) or state it over the chain's
  record projections (`co_*_rr`/`co_*_sr`), and give `rw` the instantiated lemma
  (`fix_tick_eq_diag (pacing := pacing) () f`: with `f : (KnotKind.atomic ℓ σ).toKind.sched →
  …` the uninstantiated pattern does not unify with the `Atomic ℓ σ` binder).
- **HList and `whnfR`.** `HList` is a reducible type function (`f κ × HList f κs`), so an
  HList-typed binder may arrive as a `Prod` — guard with `whnfR` before matching on
  `HList.cons`; a `Lags` component is typed `(fun _ => Nat) κ`, so `omega`/`decide` fail on
  `.1`-typed numerals — state `Nat` facts with `(x : Nat)` or apply lemmas; `HList.cons` in a
  `show` needs `(f := fun _ => Nat)`.
- **The lag algebra is tropical**: monotone, sub-additive (`Φ₀ (addC m c) ≤ addC (Φ₀ m) c`),
  max-distributive, `0 ↦ 0`, and `Φ m = max (Φ₀ m) (Φ 0)` (externals at their real lags) are
  exactly what the Bekić cascade needs (`Graph/Lags.lean`, `LagLaws`); a `min`-like
  rule would break it (`max (a ∸ 1) (min a b)`).
- **Validity is cascade validity, not all-strict** (`leader_election`'s knot output lags at
  the probe are `[0, 0, 1]`: `p_is_leader` reads `i_am_leader` immediately). `cascadeValid κs
  Φ` closes the knots left to right (`LagFun.next`), each knot strict in its own wire with
  the earlier ones closed; it is acyclicity of the immediate-dependence digraph in Bekić
  order. The generator mirrors it with ≤ 2ⁱ body-lag evaluations for knot `i` (memoized).
- A knot's output lag `λ_K` (`GenInfo.knotOutLag`) is a literal computed at the probe
  (params at lag 1, externals at 0); inner knots enter the chain's `dag_agreeL` at lag 1 via
  `mono_lag` from their own `K_strict₁` (which may conclude at lag 0 — `agreeAtLag1`
  accepts any μ ≤ 1).
- `let x := f` followed by less-indented argument lines parses the arguments as the `let`
  body (`colGt`); a generic lemma whose `sw` lambda `rw` cannot match is applied with
  `exact` after `change … at` on the differently-typed hypotheses (`Node.Timing (fixN)` vs
  `Dag.Timing body`).
- A `do`-block line whose expression's type is a type-level function of the production
  graph (`Dag.Part m g.dag`, `Mask Γ`) costs ≈ 300k heartbeats of unification per line
  (`BenchProfile.idleBench`'s fused-sweep line, budget 600k); the other lines' types are
  the stored shapes.

## Runtime (what the executables actually run — D73)

The `Values` program IS the executable. Its member carriers are `MemberMap`s, and the
sharing of a run lives or dies on four compiler facts, each learned the hard way:

- **Lean eta-expands every declaration to its TYPE's arity**
  (`Lean/Compiler/LCNF/ToDecl.lean`, `Decl.etaExpand`). A definition of function type
  can never be pre-forced: `def m := let v := Vector.ofFn f; fun i => v.get i` becomes
  `fun i => (Vector.ofFn f).get i`. This is why the carrier is a structure and the
  forcing constructor `MemberMap.mkImpl` returns it.
- **A structure with exactly one relevant field is erased to that field**
  (`hasTrivialStructure?`). With `cells` alone, a member map was a FUNCTION to the
  mono-phase passes, and two of them undid the forcing: `Simp` eta-expands local
  lambdas whose (erased) type is an arrow — `fix_stream` became `fun fuel body i =>
  iterate body seed fuel i`, the whole Kleene iteration per read — and `simpAppApp?`
  (`let g := f a; g b ⟹ f a b`, no check that `f a` was a cheap partial application)
  merges a let-bound map into its read, moving `mkImpl n g` INSIDE the downstream op's
  per-cell lambda. Both compound through the chain: `n^depth`. Measured with a
  `dbgTrace` on the forcing (the `MK n` count = map constructions): `p_leader_heartbeat`
  alone 1→4 members 11 → 43 → 139 → 367; `leader_election` alone 1→4 proposers
  852 → 14k → 344k → 3.9M (acceptor count irrelevant — the acceptor chains are
  shallow); `paxos_core` 2×1 did not finish in 60 s. The `boxed : Unit` field makes the
  map a heap object; everything is flat (686 constructions at any proposer count,
  `paxos_core` 4×4 in 36 ms). The old call-by-name `Values` (bare `Fin n → C`) suffered
  the same, which is the other half of why D14 took hours.
- **The specializer copies ground `let`s into specialized code**
  (`Specialize.lean`, `Collector.collect`): a value whose free variables are all ground
  is not abstracted into a parameter but re-created in the specialization. Harmless
  for a heap object (it is built once per specialization call), another sharing leak
  for a function-typed one.
- **`dbgTrace` is itself subject to all of the above.** `dbgTrace msg fun _ => <map>`
  wraps a thunk returning a (then function-typed) map and re-introduced the laziness —
  the instrumented `LeaderElection` went 74 s → 9+ min. Trace the forced VECTOR
  (`let v := dbgTrace … fun _ => Vector.ofFn f`), never the map. Never trace or
  `dbgTrace` inside a proof-relevant reducible definition (a `Values` op): it changes
  the definitional unfolding the `den` set relies on (an LE elaboration ran 11 min /
  9 GB). Never run `trace.Meta.isDefEq`-class traces on whole-program terms (8.9 GB log).
- **Finding a sharing leak**: count constructions per member count with the vector
  trace at 1, 2, 3, 4 members — flat is right, anything else is a leak; then dump IR
  (`Lean.IR.declMapExt.getModuleEntries env modIdx`, in a scratch `#eval`; the
  `*._redArg` decls are the real bodies) and look for a `mkImpl`/`iterate` RESULT
  being `app`lied inside a `_lam_`, or an op lambda with a trailing extra parameter.
  A 2-op toy with `set_option trace.Compiler.result true` shows the moved call as
  `_x.overap := mkImpl n _f` inside the cell lambda.

### The wired interpreter's memo tables (D74)

`Dag.locStep`'s cells and increments are per-member functions
(`Hosted sel ℓ → X`); without materialization a run is call-by-name over the graph
(the D73 leak again — a proposer activation took 3.3 s and the sixth never finished).
`memo`/`memo2` (`Project/Wired.lean`) are the identity in the logic and build a table in
compiled code (`@[csimp]`). Getting the table to be built ONCE took three rounds, each
diagnosed from the IR (`.lake/build/ir/Hydro/Project/Wired.c`) and a `gdb -p` backtrace
of the hung binary (`thread apply all bt`; the work is on a helper thread, `perf`
without call graphs only names the hot leaf functions):

- the D73 fact #1 applies to **local** functions too (`Simp/Main.lean`
  `decl.isEtaExpandCandidate → etaExpand` for `FunDecl`s, and ToLCNF's "cases
  produces a function → local fun decl, eta-expanded"): `fun i => memoFin (s i)`
  became `fun i j => (build table) j`;
- the D73 fact #2 again: a `MemoBox` with the one field `fn` was the function itself
  — hence `tag : Unit := ()`;
- **a `match` arm whose type reduces to a function type is eta-expanded as well**:
  `Node.memoState`'s `| .fold .., s => memo s` arm has type `(Hosted → σ) → Hosted → σ`
  after the constructor is known, so the arm became `fun s i => (memoBoxImpl s).fn i`
  and the table was rebuilt per read. The arms return a `StateBox` (two relevant
  fields) and `Node.memoState` projects OUTSIDE the match. `memoEnv` never had the
  problem because its arms return a product.

After: a `paxos_core` process step is 0–1 ms, the whole 60-activation TCP run ≈ 0.5 s.
The csimp replacement happens AFTER `macro_inline` (`ToDecl.lean`: eta-expand the body
→ `macroInline` → `toLCNF` with `CSimp.replaceConstant`), which is why `memo` is a
`@[macro_inline]` projection of the (never eta-expanded) box: the use site compiles to
"build the box, project the closure". `Kernel.isDefEqGuarded` is pure — time it with
`IO.lazyPure`; `IO.asTask` for a blocking socket loop needs `(prio := .dedicated)`
(thread-pool exhaustion deadlocks otherwise); `tick`, `state`, `complete` are Hydro
keywords and cannot name binders in files that import the surface.

### A memo table must not capture what it tabulates (D84)

The stored lookup closure of `memo` captured the tabulated function `f` for a `none`
fallback that can never fire (the table is total at the hosted members). `f` is `fun i =>
s i ++ Δ.get x i`-shaped — a closure over the previous cells and increments, themselves
such closures — so every cell retained every past state of everything it reads: ≈ 0.5 MB
per activation, 30 GB in a 6 s `bench`, OOM at 20 s, and step times growing with the
run. The table now carries the proof that it is total at the hosted members
(`hostedTable_get`) and the lookup is `Option.get` — the closure captures the table and
nothing else. Two traps on the way: writing the inner channel tables as `hostedTable fun
i => (memoFinBoxImpl (f i)).fn` is a lambda returning a function, which the compiler
eta-expands (fact #1 above) and so rebuilds the inner table on every lookup — the demo
spun at 600 % CPU forever; tabulate the *boxes* and project inside a `@[noinline]`
lookup (`lookupHosted2`). And `Option.get` on a `none` compiles to a field projection of
a scalar — undefined behaviour, a spin, not a panic — so the totality proof is
load-bearing for the compiled code, not decoration. Diagnose this class with `/usr/bin/time
-v` (max RSS) and the harness's first-quarter/last-quarter step times before reading IR.

### A process reads its clock once per activation (D84)

Timing at the machine is a total per-tick script (`TickScript`); the lock-step cells of
the timer nodes are tick counters indexing it; a process is handed `Dag.Timing` — this
tick's reading/verdict/pulse/flags — and `Dag.timingAt` is the only place the script
meets the process. Do not reintroduce a per-process timing history (the D83 `Timing.readings
: Array Nat` with `toList` per activation was O(ticks) per step on every process). The
timer oracles are one reading per tick (D85's `TPaced` is the bound that makes one slot
enough). Since D89 there are two timer nodes — `time` and `source_interval (delay)
(period)` (D91: a stream-kind source node; its oracle is this activation's fire COUNT,
its delta `replicate fires ()`) — and a shell computes every fire count from its clock
(`TickReadings.ofClock now last`, `fireCount` = the fire instants in `(last, now]`,
tokio's `Burst`); `Dag.timers` lists a process's timers so it can sleep until
`nextDue`. Two interval nodes with the same `(delay, period)` at one location read one
timer.

### Adding an inductive constructor with an instance argument (D91)

`Node Γ [.stream ℓ Unit instDecidableEqUnit .totalOrder .atLeastOnce]` in the `Node`
inductive silently made `instDecidableEqUnit` an **auto-bound implicit** of the
constructor (the core instance is `instDecidableEqPUnit`): every arm matched, every
`Node.Sched`/`Node.State` projection of the new node then failed to reduce ("expected
`(Node.source_interval delay period).Sched`"). Name the instance constant that exists,
or write `(inferInstance : DecidableEq Unit)`. Also: when a new node's arms are produced
by duplicating a sibling's (`.mapTick ..` → `.firstTick ..`), the match tables reject a
doubled arm as "redundant alternative" — dedupe before building.

### Single slots and `TPaced` (D85)

- An atomic kind's production increment is `Option`; a lock-step increment is
  `(incTrace …).head?`. Every `spec_<op>` on an atomic input needs the input paced
  (`hp : TPaced …`) to turn `head?` back into the list: `simp only
  [incTrace_eq_toList_head? hp …]`. The hypothesis is threaded as `HTPaced pacing env`
  through `Node.step_spec`/`Dag.step_spec`/`Dag.run_spec`; knot bodies get it from
  `bekic_tpaced`/`Dag.denote_tpaced` (an explicit `pacing` argument, unlike
  `denote_causal`).
- `rw [Option.toList_none]` fails when the goal has the term at two types — use
  `simp only [Option.toList_none]`. `show` must spell the `Option` shape (`some v` /
  `none`), not the list it replaces.
- Beta-redexes left by `Kind.delta` unfolding stop `omega`: `simp only []` first.
- `deferReleased` is recursive in the step; its facts are `deferReleased_succ`,
  `_le_succ`, `_succ_le`, `_of_not`, `_mono`, `_le_len_succ`, `_congr` (`Sched.lean`),
  and the pure-list fact under `spec_defer_tick` is `defer_fifo_step` (`StepOps.lean`).
  Do not reach for `(init :: t).take ticks` — it is a theorem about Rust-expressible
  inputs (`deferReleased_eq_ticks`), not the definition.

### Hash maps (D75)

- `Std.HashMap` is **not** extensional (bucket layout is part of `=`); `Std.ExtHashMap` is
  the quotient by contents, has `DecidableEq`/`ext_getElem?`, and is a real hash map at
  runtime. A Rust `HashMap` that travels as a *value* is an `ExtHashMap` (`Hydro.HMap`),
  never a sorted association list; an order is forced only at the wire (`toList`, slots
  ascending — unique keys make it a function of the quotient, `List.Perm.eq_of_pairwise`).
- The machine's keyed operators keep their list definitions as the semantics; the
  `@[csimp]` route needs a constant to hook, so each operator has an `…H` wrapper carrying
  `[Hashable K]` (the Rust's bound, at the operator) whose `…Impl` is the hash-backed
  computation. Equations are proved on lists; for `reduce_watermark`'s batch the tool is
  `assoc_ext` (same key list + same lookups ⇒ equal lists) with `keys_foldl_payload` and
  `lookup_foldl_payload`.
- `List.dedup` keeps the LAST occurrence (`dedup_cons_of_mem`) — the machine's key order
  is last-arrival; first-arrival order is `fdGo`, and `List.nodup_append`'s disjointness
  conjunct is `∀ a ∈ l₁, ∀ b ∈ l₂, a ≠ b` here.
- The leader's cost was proportional to the inbox, not the log: profile per *node*
  (`stepDagT`-style timing inside the `locStep` recursion) before touching a
  representation; `perf` names leaves only.

### The production path's costs (D86–D87)

Everything below was found by measuring (`benchprof stages`/`nodes`/`idle` — the
instrumented entry point, `BenchProfile.lean`, since D94; `perf`); FINDINGS D86–D87 have the numbers.

- A `def D cfg := … dag …` rebuilds the graph on every call: build it once per process
  (`BenchDemo.Graph`) and pass it. Four rebuilds per activation hid in `activate` alone.
- A value stored into an `IO.Ref` created on another thread is marked multi-threaded
  (`lean_mark_mt` walks it) and every reference-count operation on it is atomic from then
  on. The cells are a loop argument, never a shared reference.
- A `let` binding a *function* and using it twice is two evaluations when the function
  is applied twice: the `locStep` arms bind the per-member value under `memo`/`memo2`
  (`rfl` identities in the logic — `memo_apply`/`memo2_apply` belong in every `simp only`
  set that unfolds `Node.locStep`; `rw` cannot see through them). `tick_scan` ran its body
  `1 + outputs` times before.
- Materialization after stepping compounds with nesting: `Node.memoState` after a `call`
  rebuilt cells the callee had already materialized, once per level. `Node.memoStateImpl`
  is the identity on `call`/`fixN` (`Node.memoStateImpl_eq`), and the whole compiled step
  is `Dag.locStepImpl` under `@[csimp] Dag.locStep_eq_impl`, which also skips the nodes a
  process does not host (`Node.touches`, `Node.locStep_idle`).
- `(c.send bytes).block` through the libuv loop costs ~1 ms per call (a cross-thread
  hand-off): queue frames to a writer thread per connection (`Outbox`, vectored `sendAll`).
  Ten machines as threads of one process share one event loop, allocator and reference
  counts: `bench` runs each as its own OS process. `noDelay` was not it.
- One walk of the typed graph (`Dag` recursion, tuple per node) is 9–17 µs for 187 nodes;
  an activation makes eight. Per-batch walks multiply: `Dag.inboxOfWire` builds the inbox
  in one walk for all batches (`inboxOfWire_eq` to the per-batch `Dag.inboxOf`).
- JSON on the wire was 0.9 µs a message to decode and 0.65 to encode; the binary `Wire`
  class (LEB128, one-byte tags, length-prefixed lists; frames = varint size then bytes — D90)
  is 0.53 / 0.28. No theorem names the representation (`cod.enc`/`cod.dec` only), but the
  framing has a law (`Wire.framesFrom_prefix_frameAll`) the trace proof rests on.
- `#guard`/`#eval` in a library file interpret the IR (the elaborator does not run compiled
  code for the project's own modules); a Paxos activation that is 100–400 µs in `lake exe
  wired` is ~2 ms interpreted and a run of a few hundred steps is minutes. Heavy witnesses go
  in an executable (`wired --traces`); `#guard` is for toys (D90).
- A `Std.HashMap` operation on a boxed `(Nat × Ballot)` key through the polymorphic
  `Hashable`/`BEq` instances is ~100 ns; the tick bodies make ~50 per request. That is the
  constant between 100k requests/s and the Rust's 400k; the closed-loop benchmark's
  throughput is `in-flight / latency`, so spacing ticks to amortize fixed cost lowers it.

### Profiling the benchmark and what it found (D92)

The instrument is `scripts/bench_perf_role.sh <proc> [seconds]`: it starts `bench 500 10 …`,
finds the OS process `bench --proc P` by reading `/proc/*/cmdline`, attaches `perf record
--call-graph dwarf,8192` to it, and prints the flat self profile, the inclusive profile
(Lean's mangling stripped: `lp_hydro__lean_F___at___00G_spec__N` is `F` specialized inside
`G`), and the **machine thread alone**. FINDINGS D92 has the rounds; the methods and traps:

- `perf record -p PID` samples every thread. The reader and writer threads of a `bench`
  process (one `recv?`/`sendAll` per socket through libuv's loop, each a promise and a
  condition-variable hand-off) are ~60 % of its user cycles and none of its throughput; the
  machine is the busiest thread. Read it by `tid` (`perf script -F tid,ip,sym`, one blank-line
  separated record per sample) — the script's last section does.
- When DWARF unwinding is partial the inclusive report over-attributes (an 8 % entry was
  1.2 % by counting samples whose stack contains the symbol). Count stacks: awk over `perf
  script` with `RS=""`, `index($0, "Name")`; callers of a hot leaf are the record's second
  line (`lean_dec_ref_cold`'s callers tell you which structure is being freed).
- Never `pgrep -f`/`pkill -f` with a pattern that matches the shell running it (it does).
- An `IO.Ref` created on another thread marks everything stored into it multi-threaded:
  an `Array` pushed through such a ref is copied at every push and `lean_mark_mt` re-walks
  it — O(n) per activation that looked like a slow drift (D87's "leader step drift"). Loop
  arguments, never shared refs, for anything that grows (D84's rule; one site had escaped).
- `Nat.shiftLeft` (`lean_nat_shiftl`) and `Nat.pow` (`lean_nat_pow`) allocate GMP integers
  even on small operands; `*` and `+` stay scalar. The LEB128 reader carries a multiplier.
- A `Std.HashMap` grown from `∅` rehashes at every doubling (`expand_go`); size it
  (`emptyWithCapacity n`, `insertMany`) when the input's length is in hand.
- `Ctx = List Kind` is runtime data: every walk of a `Dag Γ outs` recomputes `κs ++ Γ` at
  each `letN` (the implicit argument of the recursive call) — `List.appendTR` →
  `List.reverseAux` in every walk's profile. Fewer walks (one, `Fused.lean`) is the fix
  available; erasing `Ctx` would need it to be a type index the compiler drops.
- `encode x.1 ++ encode x.2` is an array per field and a copy per `++`; framing one message
  was ~10 allocations and four copies of its payload. The `Wire` class carries `put : α →
  ByteArray → ByteArray` with `put_eq : put x b = b ++ encode x` (default: the law), and
  `Codec.ofWire` encodes by `put` into one buffer; `pushNat` writes LEB128 in place
  (`pushNat_eq`). `ByteArray.push`/`++` on an exclusive buffer with capacity are in place.
- In a mutual `Node`/`Dag` theorem, rewrite the memo identities (`Node.memoState_eq`,
  `memoEnv_eq`) while the terms are still `Node`-typed; after the `*_call` equations the two
  sides are cross-typed and `rw`/`simp` fail with "not type-correct under implicit
  transparency". `cod : (Node.call N _).Codecs` is not
  syntactically a `Codec α`: `rw [decodeAt_filter]` does not find it — instantiate
  (`have h := decodeAt_filter cod …; rw [h]`).
- Hydro compiles every `defer_tick` to DFIR `defer_tick_lazy` (buffered items do not
  schedule a tick: `dfir_lang/src/graph/ops/defer_tick_lazy.rs`), so production is idle
  between arrivals. The shells idle too since D93: an activation is `Dag.iterate` — since
  D95 the inbox, cells and oracle of a pass followed by ONE sweep of the graph
  (`Dag.sweepTop`, `Project/Sweep.lean`: a node is computed when its inputs are known, a
  strict node released then pushed, a `fix … complete` block swept until its knots are
  known — `knotLoop`, ≤ |ks|+1 sweeps, done nodes skipped) — and the loop waits unless
  bytes arrived or a timer is due. The D93 settling layer (`Dag.settle`, `knotPending`,
  `batchPending`, `settleFuel`, `settleFlag`, `Timing.quiet`) is gone with the knot's
  one-step floor: the knot wire is its body's same-step output, so nothing is pending after
  an activation (`Dag.iterate_eq_pass` on a valid graph).
- **The sweep's visit arithmetic (D95).** A leader idle activation visits 1235 nodes for a
  233-node graph: the Bekić cascade across NESTED blocks (LE.k1 → PC.k1 → LE.k2,k3 → PP.k1 →
  PC.k2 → LE) re-sweeps every inner body at each outer round — 33 body sweeps, 1059 visits of
  already-done nodes, stepping ≈ 1 % of the time. So a done visit must cost its context only:
  `Dag.sweepF` returns ONE `SweepRes` per visit, updated in place (the compiler reuses a
  unique object — pass loop arguments, never a shared ref), a done plain node is read off its
  part at the `letN` (no `memoState`/`memoEnv` rebuild), only network nodes encode
  (`Node.encodeMsgs`). `benchprof idle 0 N full` times one walk over all-done parts: ≈ 47 µs
  = ≈ 200 ns per visit, the floor of this design; the way down is fewer visits (a compiled
  per-process schedule — D96). `κs ++ Γ` is runtime data in every walk (`List.appendTR` per
  `letN`; `HList.get` matches on `Γ`), so contexts cannot be erased without an IR change.
- `benchprof trace` decodes the leading LEB128 naturals of every message
  (`node:sender:[nats]`): slots, ballots, addresses, checkpoints are readable off the trace —
  look there before suspecting the engine (D95's "replica emits nothing" was a leader
  proposing from slot 1).
- Proof techniques of the expansion (`WireTrace.expand`, D93): `rw` into a structure goal
  fails — `constructor` first, then `all_goals rw [...]`; `Nat.find` with an explicit
  existence proof `hex` and `show Nat.find hex = k`; a dependent cast `h ▸ x` over
  `(τ.expand cod).who t`: `generalize (τ.expand cod).who t = w at h ⊢; subst h` — and a
  term whose *type* depends on it (`readings`) must be `generalize`d and `revert`ed first,
  then recovered by `HEq` (`cases hR`); `simp only [Nat.add_zero, eq_self_iff_true, if_true]`
  for `if bs + 0 = bs` (not `rfl`: `Nat.decEq` on a variable does not reduce); a `foldl`
  pattern with `?f (j + 1)` does not unify higher-order — pass `f` explicitly and `.trans`.
- The closed-loop benchmark's throughput is `in-flight / latency`; at the CPU wall more
  in-flight only adds latency (500 → 1000 virtual clients: 120k → 133k, p50 8 → 14.5 ms).
  The leader's per-tick re-fold of all in-flight state (the program's `use::state` taken
  literally — and the Rust takes it literally too: these folds are over tick-located
  operands and compile to `fold_keyed::<'tick>`/`anti_join::<'tick, _>`, recomputed every
  tick, `compile/ir/mod.rs:4981–4984`/`4461–4463`; the `'static`-incremental remark that
  stood here from D92 to D96 was wrong) is 41 %
  of its machine thread after D92 and the figure 400k would need ≤ 2.5 µs per request.

### The second campaign (D94): threads, allocation, the reader's shape

- **Lean's task manager wakes every blocked waiter on every task completion** (one
  condition variable for all blocked `Task.get`/`.block`s). A process with eighteen OS
  threads each blocked in `recv?`/`sendAll` `.block` spent ~2 cores on wake-ups — D92's
  "60 % of user cycles in the socket threads". Never block many threads: a reader is an
  `Async` continuation chain (receive → push → re-arm, `Sockets.reader`), a send is
  fire-and-forget, and one `Sender.writer` thread per process issues the `sendAll`s so the
  machine never takes the event loop's lock. 428 % → 334 % CPU at higher throughput.
- `perf`'s **self** symbols on the machine thread are the allocation story:
  `lean_dec_ref_cold` + `mi_malloc` + `mi_free` were 35 % of the leader after D94's rounds;
  the callers of `lean_dec_ref_cold` (second stack line) name what is freed. The script's
  last section prints them (`scripts/bench_perf_role.sh`), with the thread's busy share
  (samples / 499 Hz × seconds) — the leader's is 83 % at 100k requests/s.
- A `Hashable (α × β)` through `instHashableProd` is a `lean_apply` per component with the
  `UInt64` boxed on return; a monomorphic instance for the hot key (`Nat × Ballot nP`) mixes
  the fields in one function.
- A `LinearOrder` lifted through `Prod.Lex`/`WithBot` decides `≤` by the lifted instances —
  ~6 allocations per comparison. Decide by a `Bool` function with an iff
  (`toDecidableLE := decidable_of_iff (leB a b = true) (leB_iff a b)`); the structure update
  must re-prove `min_def`/`max_def`/`compare_eq_compareOfLessAndEq` (`congr`).
- A decoder returning `Except String (α × Nat)` per field allocates two objects per field;
  the `Wire` class's one reader is `getK` (CPS: `(α → Nat → Except String β) → Except
  String β`) with the law `getK_bind` (the continuation is applied once, at the end); `get`
  and `decode` are definitions over it — no second reader, no seam (a first version kept
  `get` as a member with `getK_eq` and a `@[csimp] decode_eq_decodeK`; user ruling: the
  headline wants the minimal gap to what runs). Instances go through `Wire.ofSpec` with a
  pair-shaped `g` as the proof scaffold and `get_eq_of_bind` reads `get` back as `g`.
  Trap: a lemma quantified over the instance's *proof* arguments (`get_ofSpec … {hK}`) is
  not usable — proof-irrelevant unification leaves the implicit unassigned ("don't know how
  to synthesize implicit argument `hK`"); quantify over the law as a hypothesis instead.
  Reading a frame's message **in place** at its offset
  (`parseFramesImpl`) needs the reader to read inside an extract as it reads the buffer:
  `Wire.Restr` (bounds + the extract law), closed under `Prod` (`restr_prod`), proved for
  `getNat`/`getBytes`. Microbench (`benchprof decode N`): P2b decode 136 → 101 ns, frame parse
  141 → 115 ns.
- Pending flags (`knotPending`/`batchPending`) recomputed by a walk after each pass cost two
  walks per settling pass; the fused walk returns them in its `Trail` (`Trail.flag_eq`).
- Proof gotchas: `simp only [hg] at h` with `h : Except.error _ = Except.ok _` leaves the
  goal — `cases h`; `subst` the extract's variable *before* `rw [ByteArray.extract_extract]`;
  `rw [← hsz] at this` rewrites every `len` — give the lemma its explicit argument instead.

### Staging the activation (D96): what a symbolic executor in `MetaM` must avoid

`hydro_run` (`HydroStage.lean`) runs the fused sweep on the closed graph at elaboration
time and emits the data steps as `let`s. The design space and the probes are in FINDINGS
D96; the engineering facts:

- **Naive partial evaluation by `simp` does not scale.** Driving `Dag.sweepF` with `simp`
  on a four-node toy exceeded 4 GB: a structural conditional the simplifier cannot decide
  duplicates its continuation, and the sweep is nothing but such conditionals. The control
  flow has to be executed at the meta level over literal data; only the data steps are
  terms.
- **The bridge is the kernel's.** `Meta.isDefEq` on the toy's structural mask took 46 s
  where the kernel judged it in 78 ms; the production equation (`benchRun.eq_impl`, 668
  lets against `Dag.iterateBytesImpl` on the whole graph) is 20 s of kernel time. `addThmByKernelRfl`.
- **Never `whnf` with δ to reach a constructor through a projection.** `whnfCore`'s default
  projection mode (`proj := .yesWithDelta`) runs a full `whnf` on the structure: on
  `(Node.memoStateBox m n st).1` with `st` a 110-deep `Prod.snd` chain that was 0.7 s per
  node and grew with the node's position (6.2 s of a 17 s stage). Reduce the structure by
  the stager's own procedure and project (`reduceHead`), and build projection chains as
  primitive `Expr.proj`s (a `Prod.snd` *application* goes through the projection-function
  machinery at every level).
- **Smart unfolding reduces its discriminants.** `unfoldDefinition?` on `enqueueInbox m n
  (inboxOfWire …).1 cell` reduced the inbox inside — then `memoState` on top reduced the
  whole thing again. Bind every intermediate to a `let` before passing it on.
- **The compiler's front end `whnf`s the type of every subterm, uncached.** `toLCNF`
  calls `inferType`+`whnf` per visited term; a type with a free variable (`Dag.LocState m
  (D cfg)`, `Node.LocState m n`) is never in the whnf cache, so each projection of the
  cells re-walked the graph's type: 41 of 46 s of `compileDecl`. Unfold the type-level
  functions (`typeLevel`) wherever they occur — the lets' types, the implicit arguments,
  the binder types — and bind the cells and codecs as lets at their reduced (nested
  product) type. 6 s after.
- **`let`-bound variables are the program**: `zetaDelta := false` everywhere (`whnfNZ`),
  or a condition over symbolic inputs is re-evaluated to no end.
- `Decidable.rec` cannot be compiled: decide `ite`/`dite` through their instance
  (`decideIte`) and never unfold them.
- `ShareCommon.shareCommon'` on the final term (71k → 24k objects) is free and the
  compiler's and kernel's traversals cache by pointer.
- The command's heartbeats: stage 149 M raw for `paxos_bench`, compile 110 M, kernel bridge
  163 M — three declarations' work, run as three budgets (`withCurrHeartbeats`), none
  overridden; the figures are in the info line, the gate reads them (GATE §6).
- Measure with `-DElab.async=false`: asynchronously the compile and kernel phases return at
  once and their printed times are meaningless (a 46 s compile printed as 0.5 s).

### Specializing the staged activation per role (D97): why it is not worth it

The staged body is generic in the member `m`; every role runs every node's `let`, the
unhosted ones through `memoBoxImpl`'s `hostedSelf` test. Two measurements closed the idea:

- **The idle floor is not the unhosted nodes.** `benchprof idle P 300` by role: aggregator
  36.5 µs (53 hosted nodes), replica 35 (63), acceptor 46 (62), client 48 (91), leader 56
  (131) — a fit of ≈ 22 µs fixed (parse, buckets, the result tuple, encode) + 0.26 µs per
  *hosted* node. An unhosted node costs one `decEq` call, one `pap lookupHosted` closure and
  one `MemoBox` allocation (≈ 50 ns): ≈ 5 µs of a 700 µs leader step.
- **A location literal does not fold at the compiler.** `memoBoxImpl f` at `m := ⟨.prop, i⟩`,
  `ℓ := .acc` compiles to a call of the specialized `memoBoxImpl` with the `Sigma.mk` and the
  literal as arguments; with `hostedSelf`/`memoBoxImpl` marked `@[inline]` the body is
  inlined but `instDecidableEqPBLoc 0 1` stays a *call* — LCNF does not evaluate a derived
  `DecidableEq` on constructor literals (`trace.Compiler.result`, FINDINGS D97). So the
  definitional variant (stage with the location literal, bridge still kernel-`rfl`) buys
  nothing, and the propositional variant (delete the unhosted lets, bridge by `funext` over
  empty `Hosted` domains through a 668-let body) would buy the same ≈ 1 %.

Where the time is instead (the leader's machine thread, `scripts/bench_perf_role.sh 0`):
the tick bodies' per-tick re-fold ≈ 45 % (the program's own semantics, D96 correction),
decode + parse + frame ≈ 20 %, allocation and refcounting ≈ 40 % self across everything
(`List` carriers: every `++`/`map`/`filter` conses per element, core's `mapTR`/`filterMapTR`
twice). The two per-activation costs that *were* constants — `bucketArray`'s hash map per
message and `routeSends`' peer loop per message, together ≈ 10 % — are the D97 round.

### Array batches (D98): what changed for the proofs, the stager and the cap

One tick's content and every stream increment of the lock-step machine is an `Array`
(`SchedSem.BoundedStream`, `Kind.delta` of `.stream`/`.keyed`, `Node.Sends`/`Node.Inbox`,
the batch cell, `decodeAt`'s result, since D99 the channel queue `ChanCell.pending`); the remaining list cells (the atomic
pendings) and the frames stay lists. The `List` lemmas stay the proofs': the corner couples
`(sr i k).map Array.toList` and every machine spec goes through `Array.toList_inj` /
`List.toList_toArray`. What bit, and the rule each time:

- **`Array.filterMap`/`Array.foldl` carry `stop := as.size` — the batch a second time.** Two
  consequences. The staged activation (`hydro_run`) is compared with the interpreter by the
  kernel term for term, so a repeated argument is a larger term at every operator; and a
  *dependent* rewrite (`envRestrict_get`, whose type mentions the kind) under such an
  application rewrites the `stop` occurrence only — `simp` reports progress and the goal is
  unchanged where it matters. The operator arms use the wrappers **`filterMapB f as`** and
  **`foldlB f init as`** (`Project/Step.lean`), the batch named once; and `WiredSpec.lean`
  has the plain-typed `envRestrict_get_stream`/`envRestrict_get_keyed` for the simp sets
  (or state the fact over `(envIncAt env t).get x i` before `simp`, as `StepOps4` does).
- **Concatenation through one fold.** `appendFold h l := l.foldl (fun acc x => acc ++ h x)
  #[]` with `appendFold_nil/cons/singleton/append/toList/map/congr`; `tagConcat` (the
  network nodes' tagged merge) and `Node.inboxOfWire` are instances. A `show appendFold _
  [x] = _` with a metavariable function does not unify through a mutual-structural
  definition (`Node.inboxOfWire`): spell the function out in the `show`.
- **`Array.map`/`filterMap` on literals are not `rfl`- or `decide`-reducible** (well-founded
  loops over the index): state toy examples (`HydroTickCheck`, `HydroStageCheck`) through
  `toList` — `show (…).toList = …; rw [Array.toList_map]; rfl`.
- **`exact (override_pointwise …).symm` with a tactic-block argument** fails to unify its
  function metavariable at array types: `refine (override_pointwise m _ s _ (fun i hi =>
  ?_)).symm` and then the tactic.
- **`rw [decodeAt_toList]` does not match `(decodeAt cod [] base j).toList`** through the
  codec coercion: `show (filterMapToArray _ []).toList = _` then `toList_filterMapToArray`.
- **The `demux` arm of `Node.step_wired`** timed out at `whnf` inside the 32-constructor
  mutual block after the change (the array arm's `filterMapB` under the `if` per receiver):
  it is the standalone `wired_demux` the mutual arm calls.
- **The kernel bridge's footprint.** Same heartbeats and kernel time as on lists
  (`paxos_core` 102.6 → 103.8 M, `benchRun` 163.4 → 165.0 M; 20.7 → 20.9 s), terms ≈ 7 %
  larger (`#[]` is `List.toArray []`; every array op carries its instances): the bridge's
  peak RSS 3.78 → 4.06 GB alone, `WiredSafety` 3.91 → 4.20, `BenchDemo` 4.88 → 5.17
  (synchronous; imports are 2.33 GB of each). `-M` is not RSS: it is Lean's accounting at
  the kernel's checks, and under asynchronous elaboration it depends on what else is live
  (`BenchDemo` needs > 5000 synchronously, passes at 4500 under `lake build`; `WiredSafety`
  passes at 4000 synchronously, failed at 4000 under `lake build`). The lakefile is at
  `-M 4500` (D98, provisional); to compare two trees, compare bridge-only runs at `-M 30000
  -DElab.async=false` by RSS and heartbeats, as FINDINGS D98 does.
- **Measuring a carrier change.** The tick-level change alone (stage A) moved nothing
  end-to-end: the gain reappeared as `lean_array_to_list_impl`/`List.toArrayAux` at the
  tick's boundary. A representation change pays off only when the boundary is gone (stage
  B: +2–3 %). `perf` on the leader (`scripts/bench_perf_role.sh 0 5`) shows the
  conversions by name; look for them before extrapolating a microbenchmark.

### The leader's constants, third campaign (D99): what the profile taught

- **A dictionary-passing instance is a `lean_apply` per use.** Core's `instDecidableEqProd`
  takes the components' instances as arguments; at a hash map keyed by a pair, every bucket
  probe applied each component's instance through `lean_apply_2` with boxed arguments —
  8–10 % of the leader's thread (`l_instDecidableEqProd` inclusive; `_boxed` wrappers and a
  `lean_free_object` under the apply). `Order.lean`'s `instDecidableEqProdInline` is core's
  instance term for term (so definitionally equal — explicit `instDecidableEqProd`
  annotations still typecheck) at `priority := high` and `@[inline]`: inlined at the
  specialized hash-map code, the component instances are known and the comparison is a
  direct call. The same pattern as D94's `Hashable (Nat × Ballot)`, generically. Check
  with `#synth DecidableEq (Nat × Nat)` that it is the one picked.
- **A continuation that survives a loop is a closure.** `getNatKLoop` (the multi-byte
  LEB128 loop) took `k` as a value: every slot number ≥ 128 allocated a closure per field
  per message, and the field continuations showed as `Wire.decode@…___lam__N` called via
  `lean_apply`. `@[specialize]` on the loop inlines the continuation at each call site:
  `Wire.decode (P2b)` 105 → 38 ns. A `@[inline]` first step (D94) only covers the one-byte
  case.
- **`List.flatMap`/`List.map` compile to `flatMapTR`/`mapTR`**: an array accumulation, a
  `toList`, a reversed map — three allocations per element. On a hot path, write the fold
  that conses directly (`encodeHosted`, `foldr_cons_map`/`foldr_append_flatMap` for the
  equations). `List.foldr` compiles to `foldrTR` (through an array) — fine on short lists.
- **One hash operation per element**: `getThenInsertIfNew?` returns the old value and
  inserts if absent in one probe; with the key's slot index as the value and the per-slot
  data in arrays updated by `Array.modify` (in place when unique), a grouping pass needs no
  second probe and no per-key lookup at the end (`groupPosStep`/`collectRev`). The old form
  (`groupKeysStep`: lookup, insert, then a lookup per key and a `map`) is kept for the list
  operator. Linearity: the fold's accumulator tuple is consumed by its projections before
  the `modify`, so the arrays are unique — `lean_copy_expand_array` stays at ≈ 0.2 %.
- **…but only where the component instances are constants.** The same trick on
  `Hashable (α × β)` regressed the leader 221k → 200k: the keyed operators are generic in
  `K`, the `Hashable K` dictionary arrives through the operator's closure, and inlining the
  pair instance there just moves the `lean_apply` (and the boxed `UInt64`) one level down
  — it showed as `instHashableProdInline___lam`, `lean_apply_1` doubled. `DecidableEq`
  inlines because the elaborated `Kind` names the program's instance as a lambda over the
  concrete components. A hash instance that is a direct call has to be written at the
  concrete key type (D94's `Hashable (Nat × Ballot nP)`), which is the one place a
  type-specific instance is justified; check the profile for `___lam` closures after any
  instance change.
- **A representation change behind a list characterization.** `ChanCell.pending` became an
  `Array` (R6) without touching a chan proof's argument: `ChanCell.step_eq`/`enqueue_eq`
  state the cell's operations as `take`/`drop`/`++` on `pending.toList`, `mk_toList` builds
  a cell from a list and a count, and every spec that named the queue as a list now names
  `(…).toArray`. The proofs are the old ones after `rw [ChanCell.step_eq]` plus
  `simp only [List.toList_toArray]`. The pattern generalizes: when a cell changes carrier,
  give the specs the list view as lemmas and keep the carrier out of them.
- **`-M`'s meter**: the inlined instance and the specialization changed no kernel work;
  `paxosRun`/`benchRun` pass under 4500 as before.

## Workflow

- Where does a slow file's time go? `set_option profiler true` at the TOP of the file
  (not `in` on the def) and read the cumulative table: `interpretation` is `#eval`/
  `#guard_eval`, `tactic execution` the proofs, `elaboration` the def; the generator's
  own Meta work shows in none of them — `HYDRO_PHASE_DIAG=1` prints the pipeline's
  per-phase wall clock and `scripts/heartbeat_table.sh` the per-declaration costs (D72:
  a 320 s file was 70 s of `hydro def` and 207 s of `#guard_eval`).
- `lake env lean file` uses **stale oleans** of edited dependencies — `lake build
  <Module>` after editing an import ("all toys passed" against the old construct once)
  (D63 i).
- `trace.profiler` (not `profiler`) attributes time to source positions — the fast way to
  find a hotspot inside a `hydro def` (D63 viii).
- A Python slice-and-splice edit keyed on a docstring that also appears as a structure
  field docstring duplicated 700 lines of a file; key edits on unique anchors
  (declaration heads) (D62 vi).
- **The compiler, not the census, is the dead-code oracle** (D67): anonymous instances,
  dot-notation chains through `example`-only lemmas, names mentioned only inside live
  statements, and syntax quoted by other elaborators all look dead to grep. Delete, then
  build.
- Concurrent `lake exe` invocations race on the build directory — run them sequentially
  (D65). Never edit a source file while a build or `lake exe` runs (D71: an edit
  mid-run broke a `twopc` run).
- **`#guard` on a whole-program run is a kernel evaluation** (D71): one nested-knot
  witness took 36 min at `#guard`; `#guard_eval` (compiled evaluation, build-failing on
  `false`) takes milliseconds. Use `#guard` for small pure facts only.
- **A decision that must be an exact permutation of a tick's content** (`RCDec.order`,
  `TickOrderSelDec`) is the easiest scenario datum to get wrong: a wrong order
  truncates the trace and a witness "commits nothing". Under `.faithful` the recommit
  gate re-presents the view on every leader tick, under `.guarded` once — the order
  list is variant-dependent in every B1/B2/B4 scenario (D71).
