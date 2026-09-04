import HydroV2.Paxos.Exploration

/-!
# `lake exe explore` — the E1 acausal-fabrication exploration (V2)

Runs the two E1 decision scripts (`Exploration.lean`) against the
**guarded** `paxos_core` at the `Eager` interpretation and reports:

- the acausal script STARVES (P0's flag trace truncates — the
  fabricated `bC` reign cannot bootstrap through the `p_is_leader`
  fixpoint) and trivially agrees (only P1 commits);
- the causal control heals: the reign breaks, the recommit recognizes
  slot 0 as committed and skips it, P0's fresh payload lands at slot 1
  — per-slot agreement within and across replicas, with the healing
  visible.
-/

open HydroV2 HydroV2.Exploration

abbrev HXE := Eager PaxLoc (paxMem 2 3)

def cpX : HXE.Stream .prop Nat .totalOrder .exactlyOnce :=
  EagStream.input ⟨#[[yv], [xv]], rfl⟩

def ckX : HXE.TickSingleton .acc (Option Nat) .unbounded :=
  EagTickSing.input ⟨#[ckpt 0, ckpt 1, ckpt 2], rfl⟩

/-- Guarded `paxos_core` at a script. -/
def runX (d : PaxosCoreDec HX 2 3 Nat) :=
  (paxos_core HXE .guarded .prop .acc 1 cpX ckX (pcdecE d)).val

/-- Per-slot agreement within and across two commit pools. -/
def agreeAll (a b : Multiset (Nat × Option Nat)) : Bool :=
  decide (∀ v ∈ a + b, ∀ w ∈ a + b, v.1 = w.1 → v.2 = w.2)

def main : IO UInt32 := do
  let acaus ← IO.lazyPure (fun _ => runX ndDec)
  let caus ← IO.lazyPure (fun _ => runX ndCausalDec)
  let a0 := acaus.2.data.get 0
  let a1 := acaus.2.data.get 1
  let c0 := caus.2.data.get 0
  let c1 := caus.2.data.get 1
  IO.println "== E1: acausal fabrication attempt (FINDINGS D21) =="
  let starve := decide (a0 = (0 : Multiset (Nat × Option Nat)))
  let aP1 := decide (a1 = ({(0, some xv)} : Multiset (Nat × Option Nat)))
  IO.println s!"[acausal] P0 starves (commits nothing): {starve}"
  IO.println s!"[acausal] P1 commits x@0: {aP1}"
  IO.println ""
  IO.println "== E1: causal control (the healed run) =="
  let cP0 := decide (c0 = ({(1, some yv)} : Multiset (Nat × Option Nat)))
  let cP1 := decide (c1 = ({(0, some xv)} : Multiset (Nat × Option Nat)))
  IO.println s!"[causal] P0 commits y@1 (slot 0 healed away): {cP0}"
  IO.println s!"[causal] P1 commits x@0: {cP1}"
  IO.println s!"[causal] per-slot agreement (within and across): {agreeAll c0 c1}"
  IO.println ""
  IO.println s!"acausal script agreement: {agreeAll a0 a1}"
  IO.println s!"causal control agreement: {agreeAll c0 c1}"
  unless starve && aP1 && cP0 && cP1
      && agreeAll a0 a1 && agreeAll c0 c1 do
    IO.println "explore: FAILED"
    return 1
  return 0
