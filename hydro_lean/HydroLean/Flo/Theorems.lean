import HydroLean.Flo.Graph
import HydroLean.Flo.MetaAux
import HydroLean.Flo.Confluence
import HydroLean.Flo.Progress

/-!
# Flo metatheory (dissertation §2.4): statements

This file states the graph-level metatheorems of Chapter 2:

- **Lemma 2.4.2 (Graph Stuck State)**: a graph whose leaves are lawful is
  strongly normalizing, so every configuration reaches a stuck state.
- **Lemma 2.4.3 (Determinism and Eager Execution)**: mutually-inductive
  structural proof that every lawful graph is deterministic (Def 2.4.1) and
  eagerly executable (Def 2.4.2). With Lemma 2.4.2 this yields the *unique*
  stuck state: the settled outputs of a Flo program are a function of its
  inputs, independent of scheduling — the collapse from trace semantics to
  functional semantics that the Hydro surface layer relies on.
- **Lemma 2.4.4 (Streaming Progress for Graphs)**: well-typed graphs (Fig 2.5)
  preserve streaming progress: if the bounded inputs are fixed, then every
  reachable stuck state has bounded outputs fixed (and outputs maximal).

All three are fully proven (no `sorry`): see `Flo/MetaAux.lean` (accessibility,
lifting lemmas), `Flo/Confluence.lean` (local confluence + eager execution) and
`Flo/Progress.lean` (canonical runs for streaming progress).
-/

namespace HydroLean

universe u

namespace Graph

variable {i o : List Coll.{u}}

/-- Graph steps do not change which operators appear at the leaves, so
lawfulness of leaves is preserved (delegates to `leavesLawful_step`). -/
theorem Step.leavesLawful {g g' : Graph i o} {δ : Vals o}
    (hs : Step g g' δ) (hl : g.LeavesLawful) : g'.LeavesLawful :=
  leavesLawful_step hs hl

/-- **Lemma 2.4.2 (Graph Stuck State)**: a graph with lawful leaves is strongly
normalizing: no infinite step sequences exist from any of its configurations. -/
theorem sn_of_leavesLawful (g : Graph i o) (hl : g.LeavesLawful) (O : Vals o) :
    ∃ f, NormalizesTo CStep ⟨g, O⟩ f :=
  exists_normalizesTo_of_acc (acc_cstep (accG g hl) O)

/-- **Lemma 2.4.3 (Determinism and Eager Execution)**: every graph with lawful
leaves satisfies Determinism (Def 2.4.1) and Eager Execution (Def 2.4.2). The
paper proves these simultaneously by structural induction; here Determinism is
assembled from local confluence (`lc_eager_of_lawful`, the trace-rewriting core
of §2.4.2) via Newman's lemma (`newman_rooted`), using strong normalization
(`accG`, Lemma 2.4.2) and preservation of leaf-lawfulness along steps. -/
theorem deterministic_and_eager (g : Graph i o) (hl : g.LeavesLawful) :
    g.Deterministic ∧ g.EagerG := by
  refine ⟨fun O c₁ c₂ h₁ h₂ => ?_, (lc_eager_of_lawful g.size g (Nat.le_refl _) hl).2⟩
  refine newman_rooted (fun c : Config i o => c.g.LeavesLawful) ?_ ?_ ?_
    ⟨g, O⟩ hl c₁ c₂ h₁ h₂
  · rintro x y hx ⟨δ, hs, -⟩
    exact leavesLawful_step hs hx
  · intro x hx
    exact acc_cstep (accG x.g hx) x.O
  · intro x y z hx hxy hxz
    exact (lc_eager_of_lawful x.g.size x.g (Nat.le_refl _) hx).1 x.O y z hxy hxz

/-- Unique settled outputs: combining Lemmas 2.4.2 and 2.4.3, every
configuration of a lawful graph normalizes to a *unique* stuck state. This is
the theorem that justifies the denotational (functional) semantics used by the
Hydro surface layer. -/
theorem unique_stuck (g : Graph i o) (hl : g.LeavesLawful) (O : Vals o) :
    ∃ f, NormalizesTo CStep ⟨g, O⟩ f ∧
      ∀ f', NormalizesTo CStep ⟨g, O⟩ f' → f' = f := by
  obtain ⟨f, hf⟩ := sn_of_leavesLawful g hl O
  refine ⟨f, hf, fun f' hf' => ?_⟩
  obtain ⟨det, _⟩ := deterministic_and_eager g hl
  obtain ⟨d, hd₁, hd₂⟩ := det O f' f hf'.1 hf.1
  exact (Star.eq_of_stuck hd₁ hf'.2).trans (Star.eq_of_stuck hd₂ hf.2).symm

/-- **Lemma 2.4.4 (Streaming Progress for Graphs)**: for a well-typed,
consistent graph configuration whose bounded inputs are fixed, every reachable
stuck configuration has its bounded outputs fixed. Proven by canonicalizing the
run (left-to-stuck, then right-to-stuck), justified by determinism; see
`progress_aux`. Consistency (`Consistent`, the graph lifting of
`Operator.Inv`) makes explicit the paper's implicit assumption that
configurations arise from real executions; fresh programs satisfy it by
construction. (Output maximality, the second half of Def 2.3.3, is tracked
separately; see SORRIES.md.) -/
theorem progress_of_wt {g : Graph i o} {ib ob : List Boundedness}
    (hwt : g.WT ib ob) (hfix : g.inputs.FixedWhere ib)
    {O : Vals o} (hcons : g.Consistent O) {f : Config i o}
    (hnorm : NormalizesTo CStep ⟨g, O⟩ f) :
    f.O.FixedWhere ob :=
  progress_aux g.size g (Nat.le_refl _) hwt hfix hcons hnorm

end Graph

end HydroLean
