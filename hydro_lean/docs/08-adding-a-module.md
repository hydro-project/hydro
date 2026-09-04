# 08 — Adding a module

The checklist for mirroring one more Rust function. `index_payloads`
(`Hydro/Paxos/IndexPayloads.lean`, 203 lines, `paxos.rs:776–806`) is a good-sized
template; `Hydro/Std/Quorum.lean` is the one with every feature.

## 0. Before writing

- Pick the Rust function. Note its `nondet!` sites (→ `…Dec` fields), its network edges
  (→ `…Sched` cursors), its `use::state` registers and `forward_ref` cycles. The census
  you will end with is fixed now (`../GATE.md` §5).
- Decide the **contract face** from the consumer's side: what will the caller need to
  know about the *outputs* in terms of the *inputs*? Write it as if you were the
  consumer (`../DOCTRINE.md` R3; chapter 03). If the face wants to say something about
  an internal wire, it is wrong — find the output fact the consumer actually uses.
- Check `HydroSem` has every operator the Rust uses, with the Rust name. If not, stop:
  chapter 09.

## 1. The file, top to bottom

```
import Hydro.<deps>

/-! # <Rust path>:<lines> `<fn>` … -/           -- file header: what it mirrors, in one paragraph

/-! ## Prerequisites for the contract face — the program starts at `hydro def <fn>` -/
structure <Fn>Dec (H : HydroSem L mem) … where   -- one field per nondet! site, Rust-named
structure <Fn>Sched (H : HydroSem L mem) … where -- one TransportDec per network edge (Unit at Values)
def <Fn>Sched.triv : <Fn>Sched (Values L mem) … -- the trivial record
-- vocabulary the face is stated in, with its decode lemmas (only what the face's TYPE needs)
-- loop-invariant vocabulary for the registers, if any (CQRegInv / OnceInv precedent)
structure <Fn>Ensures … (out : <output type at Values>) : Prop where
  <field> : … -- outputs and inputs only

/-! ## The program -/
/-- **<Rust path>:<lines> `<fn>`**: … Rust `nondet!` tally: n. -/
hydro def <fn> (H : HydroSem L mem) … (dec : <Fn>Dec H …) (sched : <Fn>Sched H …) :
    <output type>
  ensures out => <Fn>Ensures … out :=
  -- <Rust line>
  let … := H.<op> …
  …
  (<outputs>)
  prove <field> := …, …

/-! ## Executable non-vacuity -/
#guard (<fn> (Values …) … concrete decisions …) = …   -- one or two concrete runs

#nondet_census <fn> (nondets := n) (scheds := s) (fuels := f)
```

Nothing else precedes the `hydro def` (`../DOCTRINE.md` §layout). If you find yourself
wanting a theorem before the program, ask whether it is (a) vocabulary the face's type
needs — keep it; (b) a generic list/multiset fact — `Trace.lean`/`Grades.lean`; (c) a
fact about the program — it is a ghost inside the def.

## 2. The body

- Quote each Rust line as a comment above its Lean line; keep the Rust binder names;
  keep the Rust order. Stream-level ops take `H.`; in-tick ops take `H.b…`.
- `sliced!` → `tick (state …) (input …) := … rebind … emit/yield …`. Registers:
  `(state r : τ := seed)` for `use::state(|l| l.singleton(seed))`, `(state s :
  H.BoundedStream …)` for `use::state_null::<Stream<…>>`. `use::atomic`/`use::batch`
  entries are `(input x := <wire>)`; a `batch` with a `nondet!` is `H.batch wire
  dec.<site>`.
- `forward_ref` cycles → `fix (w : τ) … via (dec.fuel…) := body complete (…)`.
- Network edges → `H.broadcast_closed sched.<edge> wire`, `H.values …`, `H.demux …`.
- A Rust `manual_proof!(commutative)` on a fold is a real proof argument to `H.bfold`/
  `H.bkeyedFold`/`H.fold` (`FoldOk`); write the lemma (`cqCount_comm` is the pattern).
- A Lean-only line (a guard the Rust lacks, a fix under test) is marked as such in its
  comment and written as the Rust it would be (`SequencePayload.lean:226–229`).

## 3. The proofs

- Cross-tick facts → the `tick` block's `(invariant (out r … spectators) => P)` in
  loop-invariant normal form (emissions so far, registers, inputs; no indices) with
  `prove init := …, tick := …` (chapter 04 §2). Reuse `OnceInv` if the register is
  "last ballot fired at"; write a `<Fn>RegInv` with `init`/`step` lemmas before the def
  otherwise (the one allowed pre-def proof budget).
- Decode one tick as a `ghost have` after the block using the construct's readers
  (`h<out>_at`, `h<out>_inv_take`, `h<out>_reg`) and `simp only [<out>_step, den]`.
- Callee facts → `ghost have h := callee.ensures …` at the call; lifted facts →
  `callee_mono…`.
- Discharge each face field in `prove`, citing the ghosts. If a field needs more than
  ~60 lines, something upstream is wrong (a missing face, an index-form invariant, an
  unfolded wire) — fix that, not the proof (chapter 04 §6).
- Never: a mirror of the body as a pure function; a `ghost let` alias of a wire; an
  unfolded wire denotation in a proof; a lemma file.

## 4. Non-vacuity

A `#guard` or two running the program at `Values` on concrete decisions (and at `Eager`
if the scenario is big — `Paxos/EagerCheck.lean` pattern), chosen so the face's premises
are *inhabited* and the interesting branch fires. If the module fixes a Rust bug, add the
faithful/guarded pair to `Paxos/Falsification.lean` and `Falsify.lean`.

## 5. The census and the artifacts

`#nondet_census <fn> (nondets := …) (scheds := …) (fuels := …)` — the build fails on a
mismatch. After `lake build`, the module has `<fn>.ensures`, `<fn>_mono…`,
`<fn>_param…`, `<fn>_co_sr/_rr/_wf…`, and per `tick` block the `_step`/`_run`/`_at`/
`_reg` readers (`../ARCHITECTURE.md` §6). Add the headline to `AxCheck.lean`.

## 6. The gate and the ledger

`../GATE.md`, all eight steps; then a `FINDINGS.md` entry: the Rust anchors, the face
and why it is shaped that way, the census, the build time of the new file, anything the
mirror revealed about the Rust (candidate bugs go to `SCHED_AUDIT.md`'s upstream list).

## Perf

A module should elaborate in seconds; `Quorum.lean` (two defs, two invariants) is ~7 s.
If a new file is over 15 s, `set_option trace.profiler true in` on the def and find the
hotspot (`../ENGINE_NOTES.md` §workflow) — the usual suspects are a `show` across a
boundary operator or a raw `Multiset.sum` membership — before even thinking about a
budget.
