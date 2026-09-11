/-!
# Prelude: abstract rewriting infrastructure

Flo's metatheory (Chapter 2 of the dissertation) is, at its core, a theory of
abstract rewriting systems: operators and graphs are relations whose small-steps
are confluent and strongly normalizing, and "eager execution" is a compatibility
condition between the rewrite relation and the monoid action of input deltas.
This file provides the general vocabulary: reflexive-transitive closure,
joinability, confluence, stuckness (normal forms), normalization, and strong
normalization, with the standard lemmas (unique normal forms under confluence,
existence of normal forms under strong normalization).
-/

namespace HydroLean

/-- Right-to-left list induction (snoc recursion): prove a property of all
lists from the empty case and the append-one case. -/
@[elab_as_elim]
theorem list_snoc_induction {α : Type _} {P : List α → Prop} (l : List α)
    (h0 : P []) (h1 : ∀ (l : List α) (a : α), P l → P (l ++ [a])) : P l := by
  have hrev : ∀ r : List α, P r.reverse := by
    intro r
    induction r with
    | nil => exact h0
    | cons a t ih =>
      rw [List.reverse_cons]
      exact h1 _ a ih
  have := hrev l.reverse
  rwa [List.reverse_reverse] at this

/-- Reflexive-transitive closure of a relation `r` (the paper's `→*`),
tail-recursive form. -/
inductive Star {α : Sort u} (r : α → α → Prop) : α → α → Prop
  | refl (a : α) : Star r a a
  | tail {a b c : α} : Star r a b → r b c → Star r a c

/-- Head-recursive form of the reflexive-transitive closure; equivalent to
`Star` (see `Star.toStar'`, `Star'.toStar`) and convenient for induction from
the head of a reduction sequence. -/
inductive Star' {α : Sort u} (r : α → α → Prop) : α → α → Prop
  | refl (a : α) : Star' r a a
  | head {a b c : α} : r a b → Star' r b c → Star' r a c

namespace Star

variable {α : Sort u} {r : α → α → Prop} {a b c : α}

theorem single (h : r a b) : Star r a b := (Star.refl a).tail h

theorem trans (h₁ : Star r a b) (h₂ : Star r b c) : Star r a c := by
  induction h₂ with
  | refl => exact h₁
  | tail _ hxc ih => exact ih.tail hxc

theorem head (h : r a b) (h' : Star r b c) : Star r a c :=
  (single h).trans h'

theorem toStar' (h : Star r a b) : Star' r a b := by
  induction h with
  | refl => exact Star'.refl _
  | tail hab hbc ih =>
    clear hab
    induction ih with
    | refl => exact Star'.head hbc (Star'.refl _)
    | head hxy _ ih' => exact Star'.head hxy (ih' hbc)

end Star

namespace Star'

variable {α : Sort u} {r : α → α → Prop} {a b c : α}

theorem toStar (h : Star' r a b) : Star r a b := by
  induction h with
  | refl => exact Star.refl _
  | head hab _ ih => exact Star.head hab ih

end Star'

/-- Two elements are joinable if they reduce to a common element. -/
def Joinable {α : Sort u} (r : α → α → Prop) (a b : α) : Prop :=
  ∃ c, Star r a c ∧ Star r b c

theorem Joinable.refl {α : Sort u} (r : α → α → Prop) (a : α) : Joinable r a a :=
  ⟨a, Star.refl a, Star.refl a⟩

theorem Joinable.symm {α : Sort u} {r : α → α → Prop} {a b : α}
    (h : Joinable r a b) : Joinable r b a :=
  let ⟨c, h₁, h₂⟩ := h; ⟨c, h₂, h₁⟩

/-- Confluence: all peaks (of `→*`) can be joined. -/
def Confluent {α : Sort u} (r : α → α → Prop) : Prop :=
  ∀ a b c, Star r a b → Star r a c → Joinable r b c

/-- A configuration is stuck (a normal form) when no step applies
(the paper's "stuck state", §2.3.4). -/
def Stuck {α : Sort u} (r : α → α → Prop) (a : α) : Prop :=
  ¬ ∃ b, r a b

/-- `a` normalizes to the stuck configuration `f`. -/
def NormalizesTo {α : Sort u} (r : α → α → Prop) (a f : α) : Prop :=
  Star r a f ∧ Stuck r f

/-- Strong normalization: the inverse of the step relation is well-founded,
i.e. there are no infinite step sequences. -/
def SN {α : Sort u} (r : α → α → Prop) : Prop :=
  WellFounded (fun b a => r a b)

theorem Star.eq_of_stuck {α : Sort u} {r : α → α → Prop} {a b : α}
    (h : Star r a b) (ha : Stuck r a) : a = b := by
  induction h with
  | refl => rfl
  | tail _ hbc ih =>
    cases ih
    exact absurd ⟨_, hbc⟩ ha

/-- Under confluence, normal forms are unique (Flo's "unique stuck state",
combining Def 2.4.1 with Lemma 2.4.2). -/
theorem NormalizesTo.unique {α : Sort u} {r : α → α → Prop} {a f₁ f₂ : α}
    (hc : Confluent r) (h₁ : NormalizesTo r a f₁) (h₂ : NormalizesTo r a f₂) :
    f₁ = f₂ := by
  obtain ⟨d, hd₁, hd₂⟩ := hc _ _ _ h₁.1 h₂.1
  exact (Star.eq_of_stuck hd₁ h₁.2).trans (Star.eq_of_stuck hd₂ h₂.2).symm

/-- Under strong normalization, every configuration reaches some stuck state
(the engine behind Lemma 2.3.1 / Lemma 2.4.2). -/
theorem exists_normalizesTo {α : Sort u} {r : α → α → Prop} (hsn : SN r) (a : α) :
    ∃ f, NormalizesTo r a f := by
  induction a using hsn.induction with
  | _ a ih =>
    cases Classical.em (∃ b, r a b) with
    | inl h =>
      obtain ⟨b, hb⟩ := h
      obtain ⟨f, hf, hstuck⟩ := ih b hb
      exact ⟨f, Star.head hb hf, hstuck⟩
    | inr h => exact ⟨a, Star.refl a, h⟩

/-- Normal forms are stable along reduction: if `a →* b` and `a` normalizes to
`f`, then under confluence `b` also normalizes to `f`. -/
theorem NormalizesTo.of_star {α : Sort u} {r : α → α → Prop} {a b f : α}
    (hc : Confluent r) (hab : Star r a b) (h : NormalizesTo r a f) :
    NormalizesTo r b f := by
  obtain ⟨d, hd₁, hd₂⟩ := hc _ _ _ hab h.1
  cases Star.eq_of_stuck hd₂ h.2
  exact ⟨hd₁, h.2⟩

/-- If two configurations are joinable and both normalize, they normalize to the
same stuck state. -/
theorem NormalizesTo.eq_of_joinable {α : Sort u} {r : α → α → Prop} {a b fa fb : α}
    (hc : Confluent r) (hj : Joinable r a b)
    (ha : NormalizesTo r a fa) (hb : NormalizesTo r b fb) : fa = fb := by
  obtain ⟨c, hac, hbc⟩ := hj
  have ha' := ha.of_star hc hac
  have hb' := hb.of_star hc hbc
  exact ha'.unique hc hb'

/-- Rooted variant of `exists_normalizesTo`: accessibility of a single point in
the inverse step relation suffices for it to reach a stuck state. -/
theorem exists_normalizesTo_of_acc {α : Sort u} {r : α → α → Prop} {a : α}
    (h : Acc (fun b a => r a b) a) : ∃ f, NormalizesTo r a f := by
  induction h with
  | intro a _ ih =>
    cases Classical.em (∃ b, r a b) with
    | inl hex =>
      obtain ⟨b, hb⟩ := hex
      obtain ⟨f, hf, hstuck⟩ := ih b hb
      exact ⟨f, Star.head hb hf, hstuck⟩
    | inr hnex => exact ⟨a, Star.refl a, hnex⟩

/-- Local confluence (weak Church–Rosser): one-step peaks are joinable. -/
def LocallyConfluent {α : Sort u} (r : α → α → Prop) : Prop :=
  ∀ a b c, r a b → r a c → Joinable r b c

/-- **Newman's lemma, rooted and relativized**: on a class of configurations `P`
closed under steps, if every `P`-point is accessible (no infinite reductions)
and locally confluent (one-step peaks join), then any two reducts of a `P`-point
are joinable. This is the engine that upgrades Flo's per-operator local
commutation arguments (§2.4.2) to global determinism (Def 2.4.1). -/
theorem newman_rooted {α : Sort u} {r : α → α → Prop} (P : α → Prop)
    (hstep : ∀ {x y}, P x → r x y → P y)
    (hacc : ∀ x, P x → Acc (fun v u => r u v) x)
    (hlc : ∀ x y z, P x → r x y → r x z → Joinable r y z) :
    ∀ a, P a → ∀ b c, Star r a b → Star r a c → Joinable r b c := by
  intro a hPa
  induction hacc a hPa with
  | intro a _ ih =>
    intro b c hab hac
    -- work with head-recursive closures to peel the first step of each leg
    cases hab.toStar' with
    | refl => exact ⟨c, hac, Star.refl c⟩
    | head hab₁ hb₁b =>
      cases hac.toStar' with
      | refl =>
        exact ⟨b, Star.refl b, Star.head hab₁ hb₁b.toStar⟩
      | head hac₁ hc₁c =>
        rename_i b₁ c₁
        -- local confluence at the peak `b₁ ← a → c₁`
        obtain ⟨d, hb₁d, hc₁d⟩ := hlc a b₁ c₁ hPa hab₁ hac₁
        -- IH at b₁: join `b` and `d`
        obtain ⟨e, hbe, hde⟩ := ih b₁ hab₁ (hstep hPa hab₁) b d hb₁b.toStar hb₁d
        -- IH at c₁: join `c` and `e` (via c₁ →* d →* e)
        obtain ⟨f, hcf, hef⟩ :=
          ih c₁ hac₁ (hstep hPa hac₁) c e hc₁c.toStar (hc₁d.trans hde)
        exact ⟨f, hbe.trans hef, hcf⟩

/-- **Newman's lemma** (global form): a strongly normalizing, locally confluent
relation is confluent. Derived from `newman_rooted` with the trivial class. -/
theorem newman {α : Sort u} {r : α → α → Prop} (hsn : SN r)
    (hlc : LocallyConfluent r) : Confluent r :=
  fun a b c hab hac =>
    newman_rooted (fun _ => True) (fun _ _ => trivial) (fun x _ => hsn.apply x)
      (fun x y z _ => hlc x y z) a trivial b c hab hac

/-- A single step yields joinability of its endpoints. -/
theorem Joinable.of_step {α : Sort u} {r : α → α → Prop} {a b : α} (h : r a b) :
    Joinable r a b :=
  ⟨b, Star.single h, Star.refl b⟩

/-- Equal points are joinable (deterministic-successor peaks). -/
theorem Joinable.of_eq {α : Sort u} {r : α → α → Prop} {a b : α} (h : a = b) :
    Joinable r a b := h ▸ Joinable.refl r a

end HydroLean

/-- Decidable equality for `Except` (hand-written; not in core). -/
instance {E : Type u} {V : Type v} [DecidableEq E] [DecidableEq V] :
    DecidableEq (Except E V) := fun a b =>
  match a, b with
  | .ok x, .ok y =>
    if h : x = y then isTrue (by rw [h])
    else isFalse (by intro hc; cases hc; exact h rfl)
  | .error x, .error y =>
    if h : x = y then isTrue (by rw [h])
    else isFalse (by intro hc; cases hc; exact h rfl)
  | .ok _, .error _ => isFalse (by intro hc; cases hc)
  | .error _, .ok _ => isFalse (by intro hc; cases hc)
