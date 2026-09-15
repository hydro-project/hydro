import HydroLean.Prelude
import HydroLean.Hydro.NonDet

/-!
# Ticks and the `sliced!` model (dissertation §4.5.2)

A *tick* is a local, atomic loop iteration: a batch of input enters, looped
state (`use::state_null` variables in Rust's `sliced!` blocks) is read and
rewritten, and a batch of output leaves via `all_ticks`/`yield`. Contrary to
appearances, this is where Hydro programs are *most* deterministic: inside a
tick everything is a pure function of (state, batch); the only nondeterminism
is **where the batch boundaries fall**, which is exactly what the `NonDet`
guard on `batch` exposes (`Hydro/NonDet.lean`).

`TickLoop` captures one such loop denotationally. Running it over a
`Batching` is a fold — *determinism given the batching is definitional*
(it is a function; no theorem needed). The interesting theorem is
**batching invariance**: the program's observable output depends only on
`Batching.flatten`, i.e. on the input stream, not on the adversarial split.

The reusable proof device is `TickLoop.run_characterize`: exhibit
- `stF : List α → St` — the looped state as a function of the *consumed
  prefix* of the input, and
- `outF : List α → List β` — the cumulative emitted output as a function of
  the consumed prefix,
and check a single equation about one step on one batch. The conclusion is
that for *every* batching, the final state is `stF input` and the
`all_ticks` output is `outF input`. This turns "deterministic under arbitrary
batching" (the justification written inside Rust `nondet!(...)` comments,
e.g. in `hydro_std::quorum`) into a mechanical invariant check.
-/

namespace HydroLean.Hydro

universe u v w

/-- One local tick loop (the denotation of a `sliced!` block): `init` is the
initial value of the looped state (`use::state_null`), and `step` consumes one
input batch, producing the next state and one output batch. -/
structure TickLoop (In : Type u) (St : Type v) (Out : Type w) where
  /-- Initial looped state (before the first tick). -/
  init : St
  /-- One tick: `(state, batch) ↦ (state', out-batch)`. Pure — all
  nondeterminism lives in the batching, outside the loop. -/
  step : St → In → St × Out

namespace TickLoop

variable {In : Type u} {St : Type v} {Out : Type w}

/-- Run the loop from state `s` over a list of batches, collecting the
per-tick outputs. -/
def runFrom (t : TickLoop In St Out) (s : St) : List In → St × List Out
  | [] => (s, [])
  | b :: bs =>
    let (s', o) := t.step s b
    let (s'', os) := t.runFrom s' bs
    (s'', o :: os)

/-- Run the loop from its initial state. -/
def run (t : TickLoop In St Out) (bs : List In) : St × List Out :=
  t.runFrom t.init bs

/-- Per-tick outputs of a run. -/
def outputs (t : TickLoop In St Out) (bs : List In) : List Out :=
  (t.run bs).2

/-- Final looped state of a run. -/
def finalState (t : TickLoop In St Out) (bs : List In) : St :=
  (t.run bs).1

@[simp] theorem runFrom_nil (t : TickLoop In St Out) (s : St) :
    t.runFrom s [] = (s, []) := rfl

@[simp] theorem runFrom_cons (t : TickLoop In St Out) (s : St) (b : In) (bs : List In) :
    t.runFrom s (b :: bs) =
      ((t.runFrom (t.step s b).1 bs).1, (t.step s b).2 :: (t.runFrom (t.step s b).1 bs).2) := by
  simp [runFrom]

/-- Running on `bs₁ ++ bs₂` runs `bs₁` and continues with `bs₂` from the
reached state. -/
theorem runFrom_append (t : TickLoop In St Out) (s : St) (bs₁ bs₂ : List In) :
    t.runFrom s (bs₁ ++ bs₂) =
      ((t.runFrom (t.runFrom s bs₁).1 bs₂).1,
        (t.runFrom s bs₁).2 ++ (t.runFrom (t.runFrom s bs₁).1 bs₂).2) := by
  induction bs₁ generalizing s with
  | nil => simp
  | cons b bs ih => simp [ih]

/-- One-tick extension of the final state. -/
theorem finalState_append (t : TickLoop In St Out) (bs : List In) (b : In) :
    t.finalState (bs ++ [b]) = (t.step (t.finalState bs) b).1 := by
  simp [TickLoop.finalState, TickLoop.run, TickLoop.runFrom_append,
    TickLoop.runFrom]

/-- One-tick extension of the outputs. -/
theorem outputs_append (t : TickLoop In St Out) (bs : List In) (b : In) :
    t.outputs (bs ++ [b])
      = t.outputs bs ++ [(t.step (t.finalState bs) b).2] := by
  simp [TickLoop.outputs, TickLoop.finalState, TickLoop.run,
    TickLoop.runFrom_append, TickLoop.runFrom]

end TickLoop

/-- `all_ticks` (Rust: `Stream::all_ticks`, §4.5.2): release the per-tick
output batches as one stream, in tick order. -/
def allTicks {β : Type u} (outs : List (List β)) : List β := outs.flatten

namespace TickLoop

variable {α : Type u} {β : Type w} {St : Type v}

/-- The `all_ticks` output of running a tick loop over a batching. -/
def allTicksOutput (t : TickLoop (List α) St (List β)) (bs : Batching α) : List β :=
  allTicks (t.outputs bs)

/-- **Batching invariance / prefix characterization** (the proof device for
"deterministic quorum results ... even with arbitrary batching",
`hydro_std::quorum`).

Hypotheses: `stF`/`outF` give the looped state and *cumulative* output as
functions of the consumed input prefix, with
- `h0`: `stF []` is the initial state;
- `hstep`: one tick on batch `b` from the state for prefix `p` reaches the
  state for prefix `p ++ b` and emits exactly the new output suffix
  (`dOut p b`), where `houtF`: `outF (p ++ b) = outF p ++ dOut p b`.

Conclusion: for any batch list, final state and flattened output are
`stF`/`outF` of the flattened input — so **any two batchings of the same
stream yield identical `all_ticks` output** (`allTicksOutput_of_eq_flatten`). -/
theorem run_characterize (t : TickLoop (List α) St (List β))
    (stF : List α → St) (outF : List α → List β) (dOut : List α → List α → List β)
    (h0 : stF [] = t.init)
    (hstep : ∀ p b, t.step (stF p) b = (stF (p ++ b), dOut p b))
    (houtF : ∀ p b, outF (p ++ b) = outF p ++ dOut p b)
    (hout0 : outF [] = []) (bs : Batching α) :
    t.finalState bs = stF bs.flatten ∧ t.allTicksOutput bs = outF bs.flatten := by
  suffices h : ∀ (p : List α) (bs : List (List α)),
      (t.runFrom (stF p) bs).1 = stF (p ++ bs.flatten) ∧
      outF p ++ ((t.runFrom (stF p) bs).2).flatten = outF (p ++ bs.flatten) by
    have := h [] bs
    rw [hout0, h0] at this
    simpa [TickLoop.finalState, TickLoop.allTicksOutput, TickLoop.outputs,
      TickLoop.run, allTicks, ← h0] using this
  intro p bs
  induction bs generalizing p with
  | nil => simp
  | cons b bs ih =>
    have hs := hstep p b
    have ⟨ih₁, ih₂⟩ := ih (p ++ b)
    constructor
    · simp [hs, ih₁]
    · simp only [runFrom_cons, hs, List.flatten_cons, ← List.append_assoc]
      rw [← houtF p b, ih₂]

/-- Batching invariance, packaged: two batchings of the same input stream
produce the same `all_ticks` output. This is the unbounded analogue of what
the Rust simulator checks instance-by-instance when it fuzzes batch hooks. -/
theorem allTicksOutput_of_eq_flatten (t : TickLoop (List α) St (List β))
    (stF : List α → St) (outF : List α → List β) (dOut : List α → List α → List β)
    (h0 : stF [] = t.init)
    (hstep : ∀ p b, t.step (stF p) b = (stF (p ++ b), dOut p b))
    (houtF : ∀ p b, outF (p ++ b) = outF p ++ dOut p b)
    (hout0 : outF [] = [])
    {bs₁ bs₂ : Batching α} (h : bs₁.flatten = bs₂.flatten) :
    t.allTicksOutput bs₁ = t.allTicksOutput bs₂ := by
  have h₁ := t.run_characterize stF outF dOut h0 hstep houtF hout0 bs₁
  have h₂ := t.run_characterize stF outF dOut h0 hstep houtF hout0 bs₂
  rw [h₁.2, h₂.2, h]

end TickLoop

end HydroLean.Hydro
