/-
Declaration-level reachability census (`REACH_MODE=headlines` restricts the roots to
the headlines and the exes). Run with
  lake env lean scripts/reach.lean
after `lake build Hydro Examples falsify explore paxos twopc wired bench benchprof`.

Roots: the headlines, every exe `main`, every name the two `AxCheck.lean` files print axioms for,
and every declaration of the metaprogram modules (the generator references names
through quoted syntax that the kernel never sees). The closure is over every constant
mentioned in a root's type or value, transitively. A project declaration outside the
closure is referenced by nothing the gate runs or proves — a deletion candidate, to be
confirmed by the compiler.
-/
import Lean

open Lean

/-- Modules whose every declaration counts as a root (metaprograms, checks that run
commands, exe roots). -/
def rootModules : List String :=
  ["Hydro.HydroDef", "Hydro.HydroGen", "Hydro.HydroGenKnot", "Hydro.HydroGenProject",
   "Hydro.HydroGenLag", "Hydro.HydroTick", "Hydro.HydroStage",
   "Hydro.KnotTactics", "Hydro.WfTactics", "Hydro.SimpAttr", "Hydro.CoupleProj",
   "Hydro.AxCheck", "Examples.AxCheck",
   "Examples.Demo.Falsify", "Examples.Demo.Explore", "Examples.Demo.PaxosDemo",
   "Examples.Demo.TwoPCDemo", "Examples.Demo.WiredDemo", "Examples.Demo.Bench",
   "Examples.Demo.BenchProfile"]

def extraRoots : List Name :=
  [`Hydro.paxos_safe_sched', `Hydro.paxos_safe_sched, `Hydro.paxos_safe_wired,
   `Hydro.cq_safe_sched, `Hydro.cq_safe_sched', `Hydro.two_pc_safe_sched,
   `Hydro.two_pc_safe_sched', `Hydro.two_pc_once_sched, `Hydro.two_pc_once_sched',
   `Hydro.cq_live, `Hydro.cq_live_chain, `Hydro.collect_quorum_tight]

def isProject (env : Environment) (n : Name) : Bool :=
  match env.getModuleIdxFor? n with
  | some idx =>
    let m := env.header.moduleNames[idx.toNat]!.toString
    m.startsWith "Hydro" || m.startsWith "Examples"
  | none => false

def moduleOf (env : Environment) (n : Name) : String :=
  match env.getModuleIdxFor? n with
  | some idx => env.header.moduleNames[idx.toNat]!.toString
  | none => "<current>"

partial def collect (env : Environment) (seen : IO.Ref NameSet) (n : Name) : IO Unit := do
  if (← seen.get).contains n then return
  seen.modify (·.insert n)
  match env.find? n with
  | none => return
  | some ci =>
    let visit (e : Expr) : IO Unit := do
      for c in e.getUsedConstants do
        collect env seen c
    visit ci.type
    match ci.value? (allowOpaque := true) with
    | some v => visit v
    | none => pure ()
    -- inductives/structures: constructors and recursors pull in their fields' types
    match ci with
    | .inductInfo ii => for c in ii.ctors do collect env seen c
    | _ => pure ()

def main : IO Unit := do
  initSearchPath (← findSysroot)
  let seen ← IO.mkRef ({} : NameSet)
  -- Each exe root defines a top-level `main`, so they cannot share one environment:
  -- import each alone, root everything it declares.
  let exes := (if (← IO.getEnv "REACH_MODE") == some "headlines" then [`Falsify, `Explore, `PaxosDemo, `TwoPCDemo, `WiredDemo, `Bench] else [`Falsify, `Explore, `PaxosDemo, `TwoPCDemo, `WiredDemo, `Bench, `BenchProfile]).map (`Examples.Demo ++ ·)
  for exe in exes do
    let envE ← importModules #[{module := exe}] {} 0
    for (n, _) in envE.constants.map₁.toList do
      if moduleOf envE n == exe.toString || moduleOf envE n == "Examples.Demo.BenchDemo" || moduleOf envE n == "Hydro.Project.Sockets" then
        collect envE seen n
  -- The library environment with the check modules.
  let env ← importModules #[{module := `Hydro}, {module := `Hydro.AxCheck},
    {module := `Examples}, {module := `Examples.AxCheck},
    {module := `Hydro.HydroTickCheck}] {} 0
  let mut roots : Array Name := extraRoots.toArray
  let headlinesOnly := (← IO.getEnv "REACH_MODE") == some "headlines"
  if !headlinesOnly then
    for (n, _) in env.constants.map₁.toList do
      if rootModules.contains (moduleOf env n) then roots := roots.push n
    -- AxCheck's names are read from the files' text
    let ax := (← IO.FS.readFile "Hydro/AxCheck.lean") ++ "\n" ++ (← IO.FS.readFile "Examples/AxCheck.lean")
    for line in ax.splitOn "\n" do
      if line.startsWith "#print axioms " then
        let nm := (line.drop "#print axioms ".length).trim
        roots := roots.push nm.toName
  IO.println s!"roots: {roots.size}"
  for r in roots do collect env seen r
  let reach ← seen.get
  -- every project constant, grouped by module
  let mut byMod : Std.HashMap String (Array Name × Array Name) := {}
  for (n, ci) in env.constants.map₁.toList do
    if !isProject env n then continue
    if n.isInternal then continue
    -- skip auxiliary/implementation-detail names
    let s := n.toString
    if (s.splitOn "match_").length > 1 then continue
    if (s.splitOn "proof_").length > 1 then continue
    if (s.splitOn "_eq_").length > 1 && (s.endsWith "_1" || s.endsWith "_2") then continue
    if (s.splitOn ".eq_").length > 1 then continue
    if (s.splitOn "._").length > 1 then continue
    if s.endsWith ".inj" || s.endsWith ".injEq" || s.endsWith ".sizeOf_spec" || s.endsWith ".noConfusionType" || s.endsWith ".noConfusion" || s.endsWith ".casesOn" || s.endsWith ".recOn" || s.endsWith ".rec" || s.endsWith ".below" || s.endsWith ".brecOn" || s.endsWith ".ibelow" || s.endsWith ".binductionOn" || s.endsWith ".mk.sizeOf_spec" || s.endsWith ".ctorIdx" || s.endsWith ".toCtorIdx" || s.endsWith ".ofNat" || s.endsWith ".elim" then continue
    let _ := ci
    let m := moduleOf env n
    let (r, u) := byMod.getD m (#[], #[])
    if reach.contains n then byMod := byMod.insert m (r.push n, u)
    else byMod := byMod.insert m (r, u.push n)
  let mods := byMod.toList.toArray.qsort (fun a b => a.1 < b.1)
  let mut totR := 0; let mut totU := 0
  let hR ← IO.FS.Handle.mk (if headlinesOnly then "scripts/reach-headlines.txt" else "scripts/reach-reachable.txt") .write
  for (m, (r, _)) in mods do
    for n in r do hR.putStrLn s!"{m} {n}"
  IO.println "\n=== per module: reachable / unreachable (unreachable listed) ==="
  for (m, (r, u)) in mods do
    totR := totR + r.size; totU := totU + u.size
    IO.println s!"{m}: {r.size} reachable, {u.size} UNREACHABLE"
    for n in u.qsort (fun a b => a.toString < b.toString) do
      IO.println s!"    {n}"
  IO.println s!"\nTOTAL project decls: {totR + totU}; reachable {totR}; unreachable {totU}"
