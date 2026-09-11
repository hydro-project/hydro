import HydroLean.Hydro.TStream

/-!
# The growth discipline: monotonicity from the types

**User directive (docs/10 Step 0)**: prefix-monotonicity of Hydro dataflow
("realized ticks are final", Flo streaming progress) must be *automatic from
the Flo/Gyatso typing*, never hand-stated per stage. This module provides the
framework:

- **`Growth`**: the growth order `⊑` a located carrier evolves along as its
  producers run. The instances mirror the Rust ordering/retries markers
  exactly — the growth order IS the marker:
  - `List α` (streams, tick streams, tick singletons — `TotalOrder`
    carriers): prefix (`<+:`) — realized elements/ticks are final;
  - `Cnt α` (a `NoOrder + ExactlyOnce` availability multiset): value-count
    domination — elements accrue with multiplicity, order already forgotten;
  - `Mem α` (a `NoOrder + AtLeastOnce` availability set): membership
    inclusion — re-delivery is free, so only support matters;
  - products and member-indexed families (`Fin n → τ`, clusters-as-maps):
    pointwise.
- **`α →ₘ β`**: bundled growth-monotone maps, closed under composition,
  pairing, and family lifting. Every framework combinator exports its bundled
  form (`mapM`, `scanM`, `batchCM`, …) whose `.f` is *definitionally* the raw
  combinator — so a module's dataflow, written as a `→ₘ` composition, is
  `rfl`-equal to its 1:1 transcription and carries monotonicity **in its
  type**. Module authors never state growth lemmas; they read them off as
  `.mono`.

The value-level analogue (Rust's `Monotonic` singleton bound obtained from
`fold` closures with a `monotone` proof) is `Hydro/MonoSing.lean`.

## Where unorderedness lives (no positional cheating)

- **The marker is the carrier**: positional combinators (`map`, `zipWith`,
  `scan`, prefixes, `getElem`) exist only on `List`-typed (`TotalOrder`)
  wires. `Cnt`/`Mem` availabilities expose **only order-insensitive
  eliminators** (`unionF`, merge, `batchC/batchD`, `snapshotC/snapshotD`,
  weakenings) — a proof cannot mention a position of an unordered
  collection. Per-element ops on `NoOrder` *cluster* streams act on the
  per-member slices (clusters-as-maps) before fan-in, where they commute
  with every interleaving. (Count/membership-lawful `map` lifts on
  `Cnt`/`Mem` are definable; they are omitted because no transcribed
  program maps after weakening — one combinator per used Rust op.)
- **Residual order is adversary data**: `batchC`/`snapshotD` re-materialize
  lists, but their content and internal order are the *decision* (the
  `nondet!` materialization), and theorems quantify over all legal
  decisions — order-sensitivity downstream of a `NoOrder` face surfaces as
  an unprovable goal, never as a hidden assumption.
- **Unordered folds are decision-indexed trajectory families**: a fold over
  an unordered stream has one trajectory per decision, all represented. A
  `MonoSing`-typed fold output claims only per-trajectory ascent (true of
  every shuffle, from the inflationary obligation); cross-shuffle
  **confluence is deliberately not claimed** — a proof needing it must
  state commutativity explicitly. Single-run theorems (decisions fixed as
  inputs) never do.
-/

namespace HydroLean.Hydro

universe u v w u₁ u₂ u₃

/-- The growth order a located carrier evolves along (a preorder). -/
class Growth (τ : Type u) where
  /-- The growth order. -/
  le : τ → τ → Prop
  /-- Growth is reflexive. -/
  le_refl : ∀ a, le a a
  /-- Growth is transitive. -/
  le_trans : ∀ {a b c}, le a b → le b c → le a c

@[inherit_doc] infix:50 " ⊑ " => Growth.le

theorem Growth.refl {τ : Type u} [Growth τ] (a : τ) : a ⊑ a :=
  Growth.le_refl a

theorem Growth.trans {τ : Type u} [Growth τ] {a b c : τ} (h₁ : a ⊑ b)
    (h₂ : b ⊑ c) : a ⊑ c :=
  Growth.le_trans h₁ h₂

/-! ## Carrier instances -/

/-- `TotalOrder`-marker carriers (streams, tick streams, tick singletons)
grow by prefix: realized elements are final. -/
instance instGrowthList {α : Type u} : Growth (List α) where
  le := (· <+: ·)
  le_refl := List.prefix_refl
  le_trans h₁ h₂ := h₁.trans h₂

@[simp] theorem Growth.list_le_iff {α : Type u} {l l' : List α} :
    l ⊑ l' ↔ l <+: l' := Iff.rfl

instance instGrowthProd {α : Type u} {β : Type v} [Growth α] [Growth β] :
    Growth (α × β) where
  le a b := a.1 ⊑ b.1 ∧ a.2 ⊑ b.2
  le_refl a := ⟨Growth.refl a.1, Growth.refl a.2⟩
  le_trans h₁ h₂ := ⟨Growth.trans h₁.1 h₂.1, Growth.trans h₁.2 h₂.2⟩

@[simp] theorem Growth.prod_le_iff {α : Type u} {β : Type v} [Growth α]
    [Growth β] {a b : α × β} : a ⊑ b ↔ a.1 ⊑ b.1 ∧ a.2 ⊑ b.2 := Iff.rfl

/-- Member-indexed families (clusters-as-maps) grow pointwise. -/
instance instGrowthPi {ι : Type u} {τ : ι → Type v} [∀ i, Growth (τ i)] :
    Growth (∀ i, τ i) where
  le f g := ∀ i, f i ⊑ g i
  le_refl f i := Growth.refl (f i)
  le_trans h₁ h₂ i := Growth.trans (h₁ i) (h₂ i)

@[simp] theorem Growth.pi_le_iff {ι : Type u} {τ : ι → Type v}
    [∀ i, Growth (τ i)] {f g : ∀ i, τ i} : f ⊑ g ↔ ∀ i, f i ⊑ g i := Iff.rfl

/-- A `NoOrder + ExactlyOnce` availability: a multiset presented as a list
whose order is never exposed. Grows by value-count domination. -/
structure Cnt (α : Type u) : Type u where
  val : List α

/-- A `NoOrder + AtLeastOnce` availability: a set presented as a list whose
order and multiplicities are never exposed. Grows by membership. -/
structure Mem (α : Type u) : Type u where
  val : List α

instance instGrowthCnt {α : Type u} [DecidableEq α] : Growth (Cnt α) where
  le a b := ∀ v, a.val.count v ≤ b.val.count v
  le_refl _ _ := Nat.le_refl _
  le_trans h₁ h₂ v := Nat.le_trans (h₁ v) (h₂ v)

instance instGrowthMem {α : Type u} : Growth (Mem α) where
  le a b := ∀ x ∈ a.val, x ∈ b.val
  le_refl _ _ h := h
  le_trans h₁ h₂ x hx := h₂ x (h₁ x hx)

@[simp] theorem Growth.cnt_le_iff {α : Type u} [DecidableEq α] {a b : Cnt α} :
    a ⊑ b ↔ ∀ v, a.val.count v ≤ b.val.count v := Iff.rfl

@[simp] theorem Growth.mem_le_iff {α : Type u} {a b : Mem α} :
    a ⊑ b ↔ ∀ x ∈ a.val, x ∈ b.val := Iff.rfl

/-! ## Bundled growth-monotone maps -/

/-- A growth-monotone map: the typed form of a Hydro dataflow stage. Its
type *is* the streaming-progress guarantee (input grows ⇒ output grows). -/
structure MonoMap (α : Type u) (β : Type v) [Growth α] [Growth β] :
    Type (max u v) where
  f : α → β
  mono : ∀ {a b : α}, a ⊑ b → f a ⊑ f b

@[inherit_doc] infixr:25 " →ₘ " => MonoMap

instance {α : Type u} {β : Type v} [Growth α] [Growth β] :
    CoeFun (α →ₘ β) (fun _ => α → β) := ⟨MonoMap.f⟩

namespace MonoMap

variable {α : Type u} {β : Type v} {γ : Type w}
variable [Growth α] [Growth β] [Growth γ]

/-- Identity stage. -/
def id : α →ₘ α := ⟨fun a => a, fun h => h⟩

/-- Stage composition: the closure property of the discipline. -/
def comp (g : β →ₘ γ) (h : α →ₘ β) : α →ₘ γ :=
  ⟨fun a => g.f (h.f a), fun hab => g.mono (h.mono hab)⟩

@[inherit_doc] infixr:80 " ∘ₘ " => MonoMap.comp

@[simp] theorem comp_f (g : β →ₘ γ) (h : α →ₘ β) (a : α) :
    (g ∘ₘ h).f a = g.f (h.f a) := rfl

def pair (g : α →ₘ β) (h : α →ₘ γ) : α →ₘ β × γ :=
  ⟨fun a => (g.f a, h.f a), fun hab => ⟨g.mono hab, h.mono hab⟩⟩

def fst : α × β →ₘ α := ⟨Prod.fst, fun h => h.1⟩

def snd : α × β →ₘ β := ⟨Prod.snd, fun h => h.2⟩

/-- A constant is (vacuously) monotone — decisions and fixed parameters
enter dataflow this way. -/
def const (b : β) : α →ₘ β := ⟨fun _ => b, fun _ => Growth.refl b⟩

/-- Family lift: a member-indexed family of stages is a stage into the
family carrier (clusters-as-maps). -/
def pi {ι : Type u₁} {τ : ι → Type u₂} [∀ i, Growth (τ i)]
    (g : ∀ i, α →ₘ τ i) : α →ₘ ∀ i, τ i :=
  ⟨fun a i => (g i).f a, fun hab i => (g i).mono hab⟩

/-- Project a member out of a family carrier. -/
def proj {ι : Type u₁} {τ : ι → Type u₂} [∀ i, Growth (τ i)] (i : ι) :
    (∀ j, τ j) →ₘ τ i :=
  ⟨fun g => g i, fun h => h i⟩

/-- Apply a stage under a family index (map a family pointwise). -/
def piMap {ι : Type u₁} {τ : ι → Type u₂} {σ : ι → Type u₃}
    [∀ i, Growth (τ i)] [∀ i, Growth (σ i)] (g : ∀ i, τ i →ₘ σ i) :
    (∀ i, τ i) →ₘ ∀ i, σ i :=
  ⟨fun a i => (g i).f (a i), fun hab i => (g i).mono (hab i)⟩

/-! ### Application faces (`simp` set)

Applying a composed stage reduces to the raw dataflow — the equations
proofs unfold single-source stage compositions with. All are `rfl`.
Face set: these (and the `Growth.*_le_iff` characterizations above) are
the combinators' documented interface — kept intentionally whether or not
any current program consumes each one. -/

@[simp] theorem id_f (a : α) : (MonoMap.id (α := α)).f a = a := rfl

@[simp] theorem pair_f (g : α →ₘ β) (h : α →ₘ γ) (a : α) :
    (MonoMap.pair g h).f a = (g.f a, h.f a) := rfl

@[simp] theorem fst_f (ab : α × β) :
    (MonoMap.fst (α := α) (β := β)).f ab = ab.1 := rfl

@[simp] theorem snd_f (ab : α × β) :
    (MonoMap.snd (α := α) (β := β)).f ab = ab.2 := rfl

@[simp] theorem const_f (b : β) (a : α) :
    (MonoMap.const (α := α) b).f a = b := rfl

@[simp] theorem pi_f {ι : Type u₁} {τ : ι → Type u₂} [∀ i, Growth (τ i)]
    (g : ∀ i, α →ₘ τ i) (a : α) (i : ι) :
    (MonoMap.pi g).f a i = (g i).f a := rfl

@[simp] theorem proj_f {ι : Type u₁} {τ : ι → Type u₂} [∀ i, Growth (τ i)]
    (i : ι) (g : ∀ j, τ j) :
    (MonoMap.proj (τ := τ) i).f g = g i := rfl

@[simp] theorem piMap_f {ι : Type u₁} {τ : ι → Type u₂} {σ : ι → Type u₃}
    [∀ i, Growth (τ i)] [∀ i, Growth (σ i)] (g : ∀ i, τ i →ₘ σ i)
    (a : ∀ i, τ i) (i : ι) :
    (MonoMap.piMap g).f a i = (g i).f (a i) := rfl

/-- Reassociation plumbing for nested carriers. -/
def assocR {a : Type u₁} {b : Type u₂} {c : Type u₃}
    [Growth a] [Growth b] [Growth c] : (a × b) × c →ₘ a × b × c :=
  ⟨fun x => (x.1.1, x.1.2, x.2), fun h => ⟨h.1.1, h.1.2, h.2⟩⟩

end MonoMap

/-! ## Bundled framework combinators

One bundled form per framework combinator; `.f` is definitionally the raw
combinator, so dataflows written as `→ₘ` compositions are `rfl`-equal to
their 1:1 transcriptions. -/

section Combinators

variable {α : Type u} {β : Type v} {γ : Type w}

/-- Per-element `map` (also `TStream.map` at `List (List α)`). -/
def mapM (g : α → β) : List α →ₘ List β :=
  ⟨List.map g, fun h => h.map g⟩

def filterM (p : α → Bool) : List α →ₘ List α :=
  ⟨List.filter p, fun h => h.filter p⟩

def filterMapM (g : α → Option β) : List α →ₘ List β :=
  ⟨List.filterMap g, fun h => h.filterMap g⟩

/-- `all_ticks` / flatten. -/
def flattenM : List (List α) →ₘ List α :=
  ⟨List.flatten, fun h => prefix_flatten h⟩

/-- Blocking tick-aligned zip (`cross_singleton`, `zip`, `filter_if` cores). -/
def zipWithM (g : α → β → γ) : List α × List β →ₘ List γ :=
  ⟨fun ab => List.zipWith g ab.1 ab.2, fun h => prefix_zipWith h.1 h.2 g⟩

/-- `use::state` output scan (a `sliced!` body across ticks). -/
def scanM {ι : Type u} {σ : Type v} {out : Type w} (g : σ → ι → σ × out)
    (init : σ) : List ι →ₘ List out :=
  ⟨scan g init, fun h => scan_prefix g init h⟩

/-- `across_ticks` state scan (post-tick states). -/
def scanStM {ι : Type u} {σ : Type v} (g : σ → ι → σ) (init : σ) :
    List ι →ₘ List σ :=
  ⟨scanSt g init, fun h => scanSt_prefix g init h⟩

/-- `defer_tick` with initial value (cons). -/
def consM (d : α) : List α →ₘ List α :=
  ⟨(d :: ·), fun h => List.cons_prefix_cons.mpr ⟨rfl, h⟩⟩

/-- A `TickLoop` (a whole `sliced!` block) as a bundled stage: outputs. -/
def TickLoop.outputsM {In : Type u} {St : Type v} {Out : Type w}
    (t : TickLoop In St Out) : List In →ₘ List Out :=
  ⟨t.outputs, fun h => t.outputs_prefix h⟩

/-- A `TickLoop` as a bundled stage: published per-tick states. -/
def TickLoop.statesM {In : Type u} {St : Type v} {Out : Type w}
    (t : TickLoop In St Out) : List In →ₘ List St :=
  ⟨t.states, fun h => t.states_prefix h⟩

/-- `TotalOrder` batching at demand decisions `d`. -/
def batchM (d : List Nat) : List α →ₘ TStream α :=
  ⟨fun s => batch s d, fun h => batch_prefix h d⟩

/-- `sample_every` at sample decisions. -/
def sampleEveryM (samples : List Nat) : List α →ₘ List α :=
  ⟨fun v => sampleEvery v samples, fun h => sampleEvery_prefix h samples⟩

/-- Cluster fan-in read at membership (`AtLeastOnce` consumers). -/
def unionFMemM {n : Nat} : (Fin n → List α) →ₘ Mem α :=
  ⟨fun srcs => Mem.mk (unionF srcs), fun h x hx => by
    obtain ⟨j, hj⟩ := unionF_mem hx
    exact List.mem_flatMap.mpr ⟨j, List.mem_finRange j, (h j).subset hj⟩⟩

/-- `memoF` as a stage (semantically the identity; keeps `forward_ref`
iteration linear at module handoffs). -/
def memoFM {n : Nat} {τ : Type v} [Growth τ] : (Fin n → τ) →ₘ (Fin n → τ) :=
  ⟨memoF, fun h i => by
    rw [memoF_eq, memoF_eq]
    exact h i⟩

variable [DecidableEq α]

/-- Forget order and multiplicity budget: a `TotalOrder` stream read as a
`NoOrder + ExactlyOnce` availability. -/
def toCnt : List α →ₘ Cnt α :=
  ⟨Cnt.mk, fun h v => count_le_of_prefix_d h v⟩

/-- Forget down to membership (`AtLeastOnce`). -/
def toMem : List α →ₘ Mem α :=
  ⟨Mem.mk, fun h _x hx => h.subset hx⟩

/-- Multiset availability weakens to set availability. -/
def cntToMem : Cnt α →ₘ Mem α :=
  ⟨fun c => Mem.mk c.val, fun h x hx => by
    have hle := h x
    have hpos : 0 < _ := List.count_pos_iff.mpr hx
    exact List.count_pos_iff.mp (Nat.lt_of_lt_of_le hpos hle)⟩

/-- Cluster fan-in (`merge_unordered` over the member family): the `NoOrder`
union multiset. -/
def unionFM {n : Nat} : (Fin n → List α) →ₘ Cnt α :=
  ⟨fun srcs => Cnt.mk (unionF srcs), fun h v => unionF_count_mono h v⟩

/-- Merge two set availabilities (`merge_unordered` of `AtLeastOnce`
streams). -/
def appendMemM : Mem α × Mem α →ₘ Mem α :=
  ⟨fun ab => Mem.mk (ab.1.val ++ ab.2.val), fun h x hx => by
    rcases List.mem_append.mp hx with hx | hx
    · exact List.mem_append_left _ (h.1 x hx)
    · exact List.mem_append_right _ (h.2 x hx)⟩

/-- `NoOrder + ExactlyOnce` batching: the decision is the consumed batch. -/
def batchCM (d : List (List α)) : Cnt α →ₘ TStream α :=
  ⟨fun avail => batchC avail.val [] d, fun h => batchC_le_count h [] d⟩

/-- `NoOrder + AtLeastOnce` batching (membership-legal). -/
def batchDM (d : List (List α)) : Mem α →ₘ TStream α :=
  ⟨fun avail => batchD avail.val d, fun h => batchD_subset h d⟩

/-- `NoOrder` fold snapshot (`ExactlyOnce`): accumulated increments. -/
def snapshotCM (d : List (List α)) : Cnt α →ₘ TSing (List α) :=
  ⟨fun avail => snapshotC avail.val [] d, fun h => snapshotC_le_count h [] d⟩

/-- `NoOrder` fold snapshot (`AtLeastOnce`). -/
def snapshotDM (d : List (List α)) : Mem α →ₘ TSing (List α) :=
  ⟨fun avail => snapshotD avail.val [] d, fun h => snapshotD_subset h [] d⟩

end Combinators

/-! ### Combinator application faces (`simp` set, all `rfl`)

Face set — one `_f` equation per bundled primitive, kept as the documented
interface (per-combinator contract, docs/10) whether or not any current
program consumes each one. Likewise the marker-completeness primitives
above (`filterM`, `toMem`, `cntToMem`, `batchDM`, …): one bundled
primitive exists per framework combinator even where Paxos happens not to
use it. -/

section CombinatorFaces

variable {α : Type u} {β : Type v} {γ : Type w}

@[simp] theorem mapM_f (g : α → β) (l : List α) :
    (mapM g).f l = l.map g := rfl

@[simp] theorem filterM_f (p : α → Bool) (l : List α) :
    (filterM p).f l = l.filter p := rfl

@[simp] theorem filterMapM_f (g : α → Option β) (l : List α) :
    (filterMapM g).f l = l.filterMap g := rfl

@[simp] theorem flattenM_f (l : List (List α)) :
    (flattenM (α := α)).f l = l.flatten := rfl

@[simp] theorem zipWithM_f (g : α → β → γ) (ab : List α × List β) :
    (zipWithM g).f ab = List.zipWith g ab.1 ab.2 := rfl

@[simp] theorem scanM_f {ι σ out} (g : σ → ι → σ × out) (init : σ)
    (l : List ι) : (scanM g init).f l = scan g init l := rfl

@[simp] theorem scanStM_f {ι σ} (g : σ → ι → σ) (init : σ) (l : List ι) :
    (scanStM g init).f l = scanSt g init l := rfl

@[simp] theorem consM_f (d : α) (l : List α) : (consM d).f l = d :: l := rfl

@[simp] theorem TickLoop.outputsM_f {In St Out} (t : TickLoop In St Out)
    (l : List In) : (TickLoop.outputsM t).f l = t.outputs l := rfl

@[simp] theorem TickLoop.statesM_f {In St Out} (t : TickLoop In St Out)
    (l : List In) : (TickLoop.statesM t).f l = t.states l := rfl

@[simp] theorem batchM_f (d : List Nat) (l : List α) :
    (batchM d).f l = batch l d := rfl

@[simp] theorem sampleEveryM_f (samples : List Nat) (l : List α) :
    (sampleEveryM samples).f l = sampleEvery l samples := rfl

@[simp] theorem unionFMemM_f {n : Nat} (srcs : Fin n → List α) :
    (unionFMemM.f srcs).val = unionF srcs := rfl

@[simp] theorem memoFM_f {n : Nat} {τ : Type v} [Growth τ] (g : Fin n → τ) :
    (memoFM.f g) = memoF g := rfl

variable [DecidableEq α]

@[simp] theorem toCnt_f (l : List α) : ((toCnt.f l) : Cnt α).val = l := rfl

@[simp] theorem toMem_f (l : List α) : ((toMem.f l) : Mem α).val = l := rfl

@[simp] theorem unionFM_f {n : Nat} (srcs : Fin n → List α) :
    ((unionFM.f srcs) : Cnt α).val = unionF srcs := rfl

@[simp] theorem appendMemM_f (ab : Mem α × Mem α) :
    ((appendMemM.f ab) : Mem α).val = ab.1.val ++ ab.2.val := rfl

@[simp] theorem batchCM_f (d : List (List α)) (avail : Cnt α) :
    (batchCM d).f avail = batchC avail.val [] d := rfl

@[simp] theorem batchDM_f (d : List (List α)) (avail : Mem α) :
    (batchDM d).f avail = batchD avail.val d := rfl

@[simp] theorem snapshotCM_f (d : List (List α)) (avail : Cnt α) :
    (snapshotCM d).f avail = snapshotC avail.val [] d := rfl

@[simp] theorem snapshotDM_f (d : List (List α)) (avail : Mem α) :
    (snapshotDM d).f avail = snapshotD avail.val [] d := rfl

end CombinatorFaces

/-! ## Wire combinators (reader-lifted; Rust method-chaining order)

Every stage out of a module's growth carrier `Γ` is a **wire** `Γ →ₘ τ`.
The lifted combinators below take wires as arguments and return wires, so
module bodies read **forward**, in Rust's method-chaining order
(`w.map g |>.batchC d`…), with `pair`/`proj` plumbing hidden inside. Each
lift is *definitionally* the `∘ₘ` composition it abbreviates — proofs see
straight through. The set is complete over the bundled primitives (one
lift per combinator, kept whether or not each is currently consumed). -/

namespace MonoMap

section Wires

variable {Γ : Type u₁} [Growth Γ]
variable {α : Type u} {β : Type v} {γ : Type w}

/-- Member access on a family wire (cluster demux). -/
def member {ι : Type u₂} {τ : ι → Type u₃} [∀ i, Growth (τ i)]
    (w : Γ →ₘ ∀ i, τ i) (i : ι) : Γ →ₘ τ i := proj i ∘ₘ w

/-- First component of a pair wire. -/
def fstOf {α' : Type u₂} {β' : Type u₃} [Growth α'] [Growth β']
    (w : Γ →ₘ α' × β') : Γ →ₘ α' := fst ∘ₘ w

/-- Second component of a pair wire. -/
def sndOf {α' : Type u₂} {β' : Type u₃} [Growth α'] [Growth β']
    (w : Γ →ₘ α' × β') : Γ →ₘ β' := snd ∘ₘ w

def map (w : Γ →ₘ List α) (g : α → β) : Γ →ₘ List β := mapM g ∘ₘ w

def filter (w : Γ →ₘ List α) (p : α → Bool) : Γ →ₘ List α := filterM p ∘ₘ w

def filterMap (w : Γ →ₘ List α) (g : α → Option β) : Γ →ₘ List β :=
  filterMapM g ∘ₘ w

/-- `all_ticks` / flatten. -/
def flatten (w : Γ →ₘ List (List α)) : Γ →ₘ List α := flattenM ∘ₘ w

def zipWith (w : Γ →ₘ List α) (w' : Γ →ₘ List β) (g : α → β → γ) :
    Γ →ₘ List γ := zipWithM g ∘ₘ pair w w'

def zip (w : Γ →ₘ List α) (w' : Γ →ₘ List β) : Γ →ₘ List (α × β) :=
  w.zipWith w' Prod.mk

def scan {σ : Type u₂} {out : Type u₃} (w : Γ →ₘ List α)
    (g : σ → α → σ × out) (init : σ) : Γ →ₘ List out := scanM g init ∘ₘ w

def scanSt {σ : Type u₂} (w : Γ →ₘ List α) (g : σ → α → σ) (init : σ) :
    Γ →ₘ List σ := scanStM g init ∘ₘ w

/-- `defer_tick` with initial value. -/
def cons (w : Γ →ₘ List α) (d : α) : Γ →ₘ List α := consM d ∘ₘ w

def batch (w : Γ →ₘ List α) (d : List Nat) : Γ →ₘ TStream α :=
  batchM d ∘ₘ w

def sampleEvery (w : Γ →ₘ List α) (samples : List Nat) : Γ →ₘ List α :=
  sampleEveryM samples ∘ₘ w

/-- A `TickLoop` (a whole `sliced!` block) consuming a wire: outputs. -/
def loop {St : Type u₂} {Out : Type u₃} (w : Γ →ₘ List α)
    (t : TickLoop α St Out) : Γ →ₘ List Out := TickLoop.outputsM t ∘ₘ w

/-- A `TickLoop` consuming a wire: published per-tick states. -/
def loopStates {St : Type u₂} {Out : Type u₃} (w : Γ →ₘ List α)
    (t : TickLoop α St Out) : Γ →ₘ List St := TickLoop.statesM t ∘ₘ w

/-- Cluster fan-in read at membership (`AtLeastOnce` consumers). -/
def unionFMem {n : Nat} (w : Γ →ₘ (Fin n → List α)) : Γ →ₘ Mem α :=
  unionFMemM ∘ₘ w

/-- Merge two set availabilities (`merge_unordered`). -/
def mergeMem (w w' : Γ →ₘ Mem α) : Γ →ₘ Mem α := appendMemM ∘ₘ pair w w'

/-- `memoF` on a family wire (module handoff boundary). -/
def memoFam {n : Nat} {τ : Type u₂} [Growth τ] (w : Γ →ₘ (Fin n → τ)) :
    Γ →ₘ (Fin n → τ) := memoFM ∘ₘ w

variable [DecidableEq α]

def asCnt (w : Γ →ₘ List α) : Γ →ₘ Cnt α := toCnt ∘ₘ w

def asMem (w : Γ →ₘ List α) : Γ →ₘ Mem α := toMem ∘ₘ w

/-- Cluster fan-in (`merge_unordered` over the member family). -/
def unionF {n : Nat} (w : Γ →ₘ (Fin n → List α)) : Γ →ₘ Cnt α :=
  unionFM ∘ₘ w

def batchC (w : Γ →ₘ Cnt α) (d : List (List α)) : Γ →ₘ TStream α :=
  batchCM d ∘ₘ w

def batchD (w : Γ →ₘ Mem α) (d : List (List α)) : Γ →ₘ TStream α :=
  batchDM d ∘ₘ w

def snapshotC (w : Γ →ₘ Cnt α) (d : List (List α)) : Γ →ₘ TSing (List α) :=
  snapshotCM d ∘ₘ w

def snapshotD (w : Γ →ₘ Mem α) (d : List (List α)) : Γ →ₘ TSing (List α) :=
  snapshotDM d ∘ₘ w

end Wires

end MonoMap

/-! ## The typed fixpoint: `forward_ref` in the `→ₘ` world

`fix` closes a typed body over its cycle: **both** monotonicities — along
the unfolding depth (`fixHist_chain`, via `iterate_chain`) and in the
inputs (`fixHist_rel`, via `iterate_rel`) — are combinator faces, never
re-proven per program. `.f` is definitionally `forward_ref`. -/

section Fix

variable {Γ : Type u₁} {α : Type u} {β : Type v}
variable [Growth Γ] [Growth α] [Growth β]

/-- The cycle history after `k` unfoldings of a typed body at inputs `g`. -/
def MonoMap.fixHist (init : α) (body : Γ × α →ₘ α × β) (g : Γ) (k : Nat) :
    α :=
  iterate (fun a => (body.f (g, a)).1) init k

/-- Histories grow along the unfolding depth (generic; needs only that
`init` is bottom). -/
theorem MonoMap.fixHist_chain {init : α} (hbot : ∀ x, init ⊑ x)
    (body : Γ × α →ₘ α × β) (g : Γ) {k k' : Nat} (h : k ≤ k') :
    MonoMap.fixHist init body g k ⊑ MonoMap.fixHist init body g k' :=
  iterate_chain (R := (· ⊑ ·)) (fun h1 h2 => Growth.trans h1 h2) Growth.refl
    (fun _ _ ha => (body.mono ⟨Growth.refl g, ha⟩).1) (hbot _) h

/-- Histories relate across input growth (generic). -/
theorem MonoMap.fixHist_rel {init : α} (body : Γ × α →ₘ α × β) {g g' : Γ}
    (hg : g ⊑ g') (k : Nat) :
    MonoMap.fixHist init body g k ⊑ MonoMap.fixHist init body g' k :=
  iterate_rel (R := (· ⊑ ·))
    (F := fun a => (body.f (g, a)).1) (G := fun a => (body.f (g', a)).1)
    (fun _ _ ha => (body.mono ⟨hg, ha⟩).1) (Growth.refl init) k

/-- Rust `forward_ref`, typed: the closed fixpoint is a wire in the
body's inputs. -/
def MonoMap.fix (fuel : Nat) (init : α) (body : Γ × α →ₘ α × β) : Γ →ₘ β :=
  ⟨fun g => forward_ref fuel init (fun a => body.f (g, a)),
   fun {g g'} hg =>
     (body.mono ⟨hg, MonoMap.fixHist_rel body hg fuel⟩).2⟩

/-- The input-free form (top-level cycles). -/
def MonoMap.fixHist₀ (init : α) (body : α →ₘ α × β) (k : Nat) : α :=
  iterate (fun a => (body.f a).1) init k

theorem MonoMap.fixHist₀_chain {init : α} (hbot : ∀ x, init ⊑ x)
    (body : α →ₘ α × β) {k k' : Nat} (h : k ≤ k') :
    MonoMap.fixHist₀ init body k ⊑ MonoMap.fixHist₀ init body k' :=
  iterate_chain (R := (· ⊑ ·)) (fun h1 h2 => Growth.trans h1 h2) Growth.refl
    (fun _ _ ha => (body.mono ha).1) (hbot _) h

/-- Rust `forward_ref`, typed, input-free. -/
def MonoMap.fix₀ (fuel : Nat) (init : α) (body : α →ₘ α × β) : β :=
  forward_ref fuel init (fun a => body.f a)

end Fix

end HydroLean.Hydro
