import Hydro.HydroStage
import Hydro.HydroGenToy

/-!
# `HydroStageCheck`: the stager on the toy

`toy_loop` (`HydroGenToy.lean`: a `fix` knot whose body calls `toy_step`, a tick, a
network hop) staged by `hydro_run`, and checked three ways:

* the generated equations exist and rest on the standard axioms (`Hydro/AxCheck.lean` audits
  `toyRun.eq`);
* on data (bytes in the inbox, an input increment, a timer reading) the staged function
  and the interpreter `Dag.iterateBytes` return the same cells, outputs and sends. The
  equation is a theorem; the `#guard`s run the compiled code and check that it agrees too,
  so they cover the `@[csimp]` seams of `memo`/`MemoBox`, which the theorem does not
  cover;
* the staged function is total straight-line code: no `Dag` or `HList` spine is consumed
  at run time (inspect with `#print toyRun`).
-/

namespace Hydro.StageCheck

open Hydro Hydro.Project

/-- The toy's activation, staged: `toy_loop.dag` at `()`, member `m` symbolic. -/
hydro_run toyRun (m : Member Unit toyOne) where
  member := m, graph := toy_loop.dag (mem := toyOne) (), codecs := toy_loop.wireCodecs (mem := toyOne) ()

def mU : Member Unit toyOne := ⟨(), 0⟩
def d0 := toy_loop.dag (mem := toyOne) ()
def c0 := toy_loop.wireCodecs (mem := toyOne) ()
def s0 : Dag.LocState mU d0 := Dag.memoState mU d0 (Dag.restrict mU d0 (Dag.init d0))
def rd0 : TickReadings mU := ⟨fun _ _ => 7, fun _ _ _ _ => 1⟩
def Δ0 : HList (Kind.locDelta mU) [Kind.stream () Nat instDecidableEqNat .noOrder .exactlyOnce] :=
  HList.cons (fun (_ : Hosted mU ()) => (#[3, 5] : Array Nat)) HList.nil
def chans0 : List ((_ : Member Unit toyOne) × ByteArray) :=
  [⟨mU, Wire.frameAll [(0, 0, Wire.encode (9 : Nat))]⟩]

/-- The interpreter's activation. -/
def r1 := Dag.iterateBytes mU true d0 c0 (Dag.Timing.ofReadings mU rd0 d0) chans0 Δ0 s0
/-- The staged one. -/
def r2 := toyRun mU true rd0 chans0 Δ0 s0

-- the same outputs at the member, the same sends, the same tails
#guard (r1.2.1.1 ⟨0, rfl⟩ : Array Nat) == r2.2.1.1 ⟨0, rfl⟩
#guard r1.2.2.1 == r2.2.2.1
#guard r1.2.2.2.map (·.size) == r2.2.2.2.map (·.size)
-- a second activation from the first's cells agrees too; the cells are closures and
-- boxes, so they are compared through what they produce
def r1' := Dag.iterateBytes mU true d0 c0 (Dag.Timing.ofReadings mU rd0 d0) [] Δ0 r1.1
def r2' := toyRun mU true rd0 [] Δ0 r2.1
#guard (r1'.2.1.1 ⟨0, rfl⟩ : Array Nat) == r2'.2.1.1 ⟨0, rfl⟩
#guard r1'.2.2.1 == r2'.2.2.1

end Hydro.StageCheck
