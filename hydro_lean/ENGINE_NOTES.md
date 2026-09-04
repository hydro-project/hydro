# Engine notes — the elaboration gotchas canon

For anyone touching `HydroDef.lean`, `HydroGen.lean`, `HydroGenKnot.lean`,
`HydroTick.lean`, `HydroRel.lean`/`HydroParam.lean`, or writing proofs inside a
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

- Single-leg modules get `M_param` (no subscript); multi-leg get `M_param₁…` (D67 ix).
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
- When deleting a `HydroSem` field, the three `HRel.Laws` instances are anonymous
  constructors ordered by field — the matching `by …_law_tac, -- op` line must go too,
  or every later law shifts by one and the error lands on an unrelated op (D67 x).
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

## Workflow

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
  (D65).
