import Hydro.Project.Wired

/-!
# Hydro · Project — wired machine: predicates and environment lemmas

The hypotheses under which a lock-step step *is* one process's step
(`Project/WiredMain.lean`), and the plumbing lemmas on cut/overridden
increment environments.

- `Node.OracleAt`: the oracle reads the schedule at `t` — the cursors, and
  the timing scripts at the cells' tick counts; `Node.cursorsAt` is the
  oracle that does;
- `Node.Drained`: the multi-input tick cells hold no complete item (a
  `zipTick`/`reduce_watermark` pair has an empty side, a tick block has
  no complete tuple pending, a sampler is at its greedy position) — true
  of every state the machine produces, and what makes these cells inert
  at a non-ticking member with empty inputs.
-/

namespace Hydro

namespace Project

variable {L : Type} {mem : L → Nat}

/-! ## Increment environments -/

theorem envRestrict_get (m : Member L mem) : {Γ : Ctx L mem} → {κ : Kind L mem} →
    (Δ : HList (Kind.delta (mem := mem)) Γ) → (v : Var Γ κ) →
    (envRestrict m Δ).get v = Kind.restrictDelta m κ (Δ.get v)
  | _, _, _, .zero => rfl
  | _, _, Δ, .succ v => envRestrict_get m Δ.2 v

theorem envRestrict_append (m : Member L mem) : {κs Γ : List (Kind L mem)} →
    (xs : HList (Kind.delta (mem := mem)) κs) → (ys : HList (Kind.delta (mem := mem)) Γ) →
    envRestrict m (xs.append ys) = (envRestrict m xs).append (envRestrict m ys)
  | [], _, _, _ => rfl
  | _ :: _, _, xs, ys => by
    show (Kind.restrictDelta m _ xs.1, envRestrict m (xs.2.append ys))
      = (Kind.restrictDelta m _ xs.1, (envRestrict m xs.2).append (envRestrict m ys))
    rw [envRestrict_append m xs.2 ys]

theorem envRestrict_take (m : Member L mem) : {κs Γ : List (Kind L mem)} →
    (xs : HList (Kind.delta (mem := mem)) (κs ++ Γ)) →
    envRestrict m (HList.take (κs := κs) xs) = HList.take (κs := κs) (envRestrict m xs)
  | [], _, _ => rfl
  | _ :: κs, Γ, xs => by
    show (Kind.restrictDelta m _ xs.1, envRestrict m (HList.take (κs := κs) xs.2))
      = (Kind.restrictDelta m _ xs.1, HList.take (κs := κs) (envRestrict m xs.2))
    rw [envRestrict_take m (κs := κs) (Γ := Γ) xs.2]

theorem envRestrict_drop (m : Member L mem) : {κs Γ : List (Kind L mem)} →
    (xs : HList (Kind.delta (mem := mem)) (κs ++ Γ)) →
    envRestrict m (HList.drop (κs := κs) xs) = HList.drop (κs := κs) (envRestrict m xs)
  | [], _, _ => rfl
  | _ :: κs, Γ, xs => envRestrict_drop m (κs := κs) (Γ := Γ) xs.2

theorem envRestrict_mapSubst (m : Member L mem) {Γ Γ' : Ctx L mem} (Δ : HList (Kind.delta (mem := mem)) Γ)
    (args : Subst Γ' Γ) :
    envRestrict m (args.map (fun v => Δ.get v)) = args.map (fun v => (envRestrict m Δ).get v) := by
  induction Γ' with
  | nil => rfl
  | cons κ κs ih =>
    show (Kind.restrictDelta m _ (Δ.get args.1), envRestrict m (args.2.map (fun v => Δ.get v)))
      = ((envRestrict m Δ).get args.1, args.2.map (fun v => (envRestrict m Δ).get v))
    rw [ih args.2, envRestrict_get]

theorem IdleEnv.get (m : Member L mem) : {Γ : Ctx L mem} → {κ : Kind L mem} →
    (Δ : HList (Kind.delta (mem := mem)) Γ) → IdleEnv m Δ → (v : Var Γ κ) →
    Kind.idleDelta m κ (Δ.get v)
  | _, _, _, h, .zero => h.1
  | _, _, Δ, h, .succ v => IdleEnv.get m Δ.2 h.2 v

section IdleGet
variable (m : Member L mem) {Γ : Ctx L mem} {ℓ : L} {Δ : HList (Kind.delta (mem := mem)) Γ} (hΔ : IdleEnv m Δ)
include hΔ

theorem IdleEnv.get_stream {α : Type} {iα : DecidableEq α} {ord : StrOrd} {ret : Retries}
    (x : Var Γ (.stream ℓ α iα ord ret)) (i : Fin (mem ℓ)) (hi : (⟨ℓ, i⟩ : Member L mem) ≠ m) : Δ.get x i = [] :=
  IdleEnv.get m Δ hΔ x i hi

theorem IdleEnv.get_keyed {K V : Type} {iK : DecidableEq K} {iV : DecidableEq V} {ord : StrOrd} {ret : Retries}
    (x : Var Γ (.keyed ℓ K V iK iV ord ret)) (i : Fin (mem ℓ)) (hi : (⟨ℓ, i⟩ : Member L mem) ≠ m) : Δ.get x i = [] :=
  IdleEnv.get m Δ hΔ x i hi

theorem IdleEnv.get_atomic {σ : Type} (x : Var Γ (.atomic ℓ σ)) (i : Fin (mem ℓ)) (hi : (⟨ℓ, i⟩ : Member L mem) ≠ m) :
    Δ.get x i = none :=
  IdleEnv.get m Δ hΔ x i hi

theorem IdleEnv.get_atomicStream {α : Type} {iα : DecidableEq α} {ord : StrOrd} {ret : Retries}
    (x : Var Γ (.atomicStream ℓ α iα ord ret)) (i : Fin (mem ℓ)) (hi : (⟨ℓ, i⟩ : Member L mem) ≠ m) : Δ.get x i = none :=
  IdleEnv.get m Δ hΔ x i hi

theorem IdleEnv.get_atomicKeyed {K V : Type} {iK : DecidableEq K} {iV : DecidableEq V} {ord : StrOrd} {ret : Retries}
    (x : Var Γ (.atomicKeyed ℓ K V iK iV ord ret)) (i : Fin (mem ℓ)) (hi : (⟨ℓ, i⟩ : Member L mem) ≠ m) : Δ.get x i = none :=
  IdleEnv.get m Δ hΔ x i hi

theorem IdleEnv.get_atomicKSing {K V : Type} {iK : DecidableEq K} {iV : DecidableEq V}
    (x : Var Γ (.atomicKSing ℓ K V iK iV)) (i : Fin (mem ℓ)) (hi : (⟨ℓ, i⟩ : Member L mem) ≠ m) : Δ.get x i = none :=
  IdleEnv.get m Δ hΔ x i hi

end IdleGet

theorem IdleEnv.append (m : Member L mem) : {κs Γ : List (Kind L mem)} →
    (xs : HList (Kind.delta (mem := mem)) κs) → (ys : HList (Kind.delta (mem := mem)) Γ) →
    IdleEnv m xs → IdleEnv m ys → IdleEnv m (xs.append ys)
  | [], _, _, _, _, hy => hy
  | _ :: _, _, xs, ys, hx, hy => ⟨hx.1, IdleEnv.append m xs.2 ys hx.2 hy⟩

theorem IdleEnv.take (m : Member L mem) : {κs Γ : List (Kind L mem)} →
    (xs : HList (Kind.delta (mem := mem)) (κs ++ Γ)) → IdleEnv m xs → IdleEnv m (HList.take (κs := κs) xs)
  | [], _, _, _ => trivial
  | _ :: κs, Γ, xs, h => ⟨h.1, IdleEnv.take m (κs := κs) (Γ := Γ) xs.2 h.2⟩

theorem IdleEnv.drop (m : Member L mem) : {κs Γ : List (Kind L mem)} →
    (xs : HList (Kind.delta (mem := mem)) (κs ++ Γ)) → IdleEnv m xs → IdleEnv m (HList.drop (κs := κs) xs)
  | [], _, _, h => h
  | _ :: κs, Γ, xs, h => IdleEnv.drop m (κs := κs) (Γ := Γ) xs.2 h.2

theorem IdleEnv.mapSubst (m : Member L mem) {Γ Γ' : Ctx L mem} (Δ : HList (Kind.delta (mem := mem)) Γ)
    (h : IdleEnv m Δ) (args : Subst Γ' Γ) : IdleEnv m (args.map (fun v => Δ.get v)) := by
  induction Γ' with
  | nil => trivial
  | cons κ κs ih => exact ⟨IdleEnv.get m Δ h args.1, ih args.2⟩

theorem Kind.idleDelta_of_allEmpty (m : Member L mem) : (κ : Kind L mem) → (Δ : κ.delta) →
    Kind.allEmpty κ Δ → Kind.idleDelta m κ Δ
  | .stream .., _, h => fun i _ => h i
  | .keyed .., _, h => fun i _ => h i
  | .sing .., _, _ => trivial
  | .ksing .., _, _ => trivial
  | .atomic .., _, h => fun i _ => h i
  | .atomicStream .., _, h => fun i _ => h i
  | .atomicKeyed .., _, h => fun i _ => h i
  | .atomicKSing .., _, h => fun i _ => h i

theorem IdleEnv.of_empty (m : Member L mem) : {Γ : Ctx L mem} → (Δ : HList (Kind.delta (mem := mem)) Γ) →
    EmptyEnv Δ → IdleEnv m Δ
  | [], _, _ => trivial
  | _ :: _, Δ, h => ⟨Kind.idleDelta_of_allEmpty m _ Δ.1 h.1, IdleEnv.of_empty m Δ.2 h.2⟩

/-- The list-like kinds (a singleton's increment is a value, not a list). -/
def Kind.listLike : Kind L mem → Prop
  | .sing .. => False
  | .ksing .. => False
  | _ => True

theorem KnotKind.toKind_listLike : (k : KnotKind L mem) → k.toKind.listLike
  | .stream .. => trivial
  | .atomic .. => trivial

section Override

variable [DecidableEq L]

/-- Writing a process's increment back over an idle one gives the increment
back when the increment is itself idle (list-like kinds). -/
theorem Kind.overrideDelta_restrict (m : Member L mem) : (κ : Kind L mem) → κ.listLike →
    (Δ0 Δ : κ.delta) → Kind.idleDelta m κ Δ0 → Kind.idleDelta m κ Δ →
    Kind.overrideDelta m κ Δ0 (Kind.restrictDelta m κ Δ) = Δ
  | .stream ℓ .., _, Δ0, Δ, h0, h => by
    funext i
    simp only [Kind.overrideDelta, Kind.restrictDelta]
    split
    · rfl
    · rename_i hn
      rw [h0 i hn, h i hn]
  | .keyed ℓ .., _, Δ0, Δ, h0, h => by
    funext i
    simp only [Kind.overrideDelta, Kind.restrictDelta]
    split
    · rfl
    · rename_i hn
      rw [h0 i hn, h i hn]
  | .sing .., hl, _, _, _, _ => hl.elim
  | .ksing .., hl, _, _, _, _ => hl.elim
  | .atomic ℓ .., _, Δ0, Δ, h0, h => by
    funext i
    simp only [Kind.overrideDelta, Kind.restrictDelta]
    split
    · rfl
    · rename_i hn
      rw [h0 i hn, h i hn]
  | .atomicStream ℓ .., _, Δ0, Δ, h0, h => by
    funext i
    simp only [Kind.overrideDelta, Kind.restrictDelta]
    split
    · rfl
    · rename_i hn
      rw [h0 i hn, h i hn]
  | .atomicKeyed ℓ .., _, Δ0, Δ, h0, h => by
    funext i
    simp only [Kind.overrideDelta, Kind.restrictDelta]
    split
    · rfl
    · rename_i hn
      rw [h0 i hn, h i hn]
  | .atomicKSing ℓ .., _, Δ0, Δ, h0, h => by
    funext i
    simp only [Kind.overrideDelta, Kind.restrictDelta]
    split
    · rfl
    · rename_i hn
      rw [h0 i hn, h i hn]

/-- Every kind of the context is list-like. -/
def ListLikeCtx : Ctx L mem → Prop
  | [] => True
  | κ :: Γ => κ.listLike ∧ ListLikeCtx Γ

omit [DecidableEq L] in
theorem ListLikeCtx.knots : (ks : List (KnotKind L mem)) → ListLikeCtx (ks.map KnotKind.toKind)
  | [] => trivial
  | k :: ks => ⟨KnotKind.toKind_listLike k, ListLikeCtx.knots ks⟩

theorem envOverride_restrict (m : Member L mem) : {Γ : Ctx L mem} → ListLikeCtx Γ →
    (Δ0 Δ : HList (Kind.delta (mem := mem)) Γ) → IdleEnv m Δ0 → IdleEnv m Δ →
    envOverride m Δ0 (envRestrict m Δ) = Δ
  | [], _, _, _, _, _ => rfl
  | _ :: _, hl, Δ0, Δ, h0, h => by
    show (Kind.overrideDelta m _ Δ0.1 (Kind.restrictDelta m _ Δ.1), envOverride m Δ0.2 (envRestrict m Δ.2)) = Δ
    rw [Kind.overrideDelta_restrict m _ hl.1 Δ0.1 Δ.1 h0.1 h.1, envOverride_restrict m hl.2 Δ0.2 Δ.2 h0.2 h.2]

/-- The generic write-back: a per-member cell map whose idle members are unchanged. -/
theorem override_pointwise (m : Member L mem) (ℓ : L) {X : Type} (s F : Fin (mem ℓ) → X)
    (hidle : ∀ i, (⟨ℓ, i⟩ : Member L mem) ≠ m → F i = s i) :
    (fun i => if _ : (⟨ℓ, i⟩ : Member L mem) = m then F i else s i) = F := by
  funext i
  split
  · rfl
  · rename_i hn
    exact (hidle i hn).symm

end Override

/-! ## The oracle and the drained cells -/

mutual

/-- The oracle reads the schedule at step `t`: the cursors (network nodes), and each
timer's script at the node's tick count in the lock-step state `s` (D84 — a timer is
indexed by the member's own ticks, not by the global step). -/
def Node.OracleAt (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    Node.Sched n → Node.Oracle m n → Nat → Node.State n → Prop
  | _, _, .broadcast_closed _ _, d, ora, t, _ => ∀ i j, ora i j = cumMax (d i.1 j) t
  | _, _, .demux _ _ _, d, ora, t, _ => ∀ i j, ora i j = cumMax (d i.1 j) t
  | _, _, .source_interval .., d, ora, _, s => ∀ i, ora i = d i.1 (s i.1)
  | _, _, .time .., d, ora, _, s => ∀ i, ora i = d i.1 (s i.1)
  | _, _, .call N _, sch, ora, t, s => Dag.OracleAt m N sch ora t s
  | _, _, .fixN _ body, sch, ora, t, s => Dag.OracleAt m body sch ora t s.1
  | _, _, _, _, _, _, _ => True

def Dag.OracleAt (m : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    Dag.Sched d → Dag.Oracle m d → Nat → Dag.State d → Prop
  | _, _, .ret _, _, _, _, _ => True
  | _, _, .letN n rest, sch, ora, t, s => Node.OracleAt m n sch.1 ora.1 t s.1 ∧ Dag.OracleAt m rest sch.2 ora.2 t s.2

end

mutual

/-- The oracle that reads the schedule at step `t` from the process's cells `s`: the
cursors cut to the member's receiving ends (monotone: `cumMax`), the timing scripts
at the cells' tick counts (`timingAt`). -/
def Node.cursorsAt (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    Node.Sched n → Nat → Node.LocState m n → Node.Oracle m n
  | _, _, .broadcast_closed _ _, d, t, _ => fun i j => cumMax (d i.1 j) t
  | _, _, .demux _ _ _, d, t, _ => fun i j => cumMax (d i.1 j) t
  | _, _, .source_interval .., d, _, s => fun i => d i.1 (s i)
  | _, _, .time .., d, _, s => fun i => d i.1 (s i)
  | _, _, .call N _, sch, t, s => Dag.cursorsAt m N sch t s
  | _, _, .fixN _ body, sch, t, s => Dag.cursorsAt m body sch t s.1
  | _, _, .map .., _, _, _ => ()
  | _, _, .filterMap .., _, _, _ => ()
  | _, _, .values .., _, _, _ => ()
  | _, _, .into_keyed .., _, _, _ => ()
  | _, _, .kassume_ordering .., _, _, _ => ()
  | _, _, .kfold_early_stop .., _, _, _ => ()
  | _, _, .kget_max_key .., _, _, _ => ()
  | _, _, .weaken_retries .., _, _, _ => ()
  | _, _, .union .., _, _, _ => ()
  | _, _, .assume_ordering .., _, _, _ => ()
  | _, _, .fold .., _, _, _ => ()
  | _, _, .fold_monotone .., _, _, _ => ()
  | _, _, .snapshot .., _, _, _ => ()
  | _, _, .batch .., _, _, _ => ()
  | _, _, .mapTick .., _, _, _ => ()
  | _, _, .firstTick .., _, _, _ => ()
  | _, _, .zipTick .., _, _, _ => ()
  | _, _, .defer_tick .., _, _, _ => ()
  | _, _, .allTicks .., _, _, _ => ()
  | _, _, .flattenOrdered .., _, _, _ => ()
  | _, _, .flattenUnordered .., _, _, _ => ()
  | _, _, .tinto_keyed .., _, _, _ => ()
  | _, _, .tassume_ordering .., _, _, _ => ()
  | _, _, .reduce_watermark .., _, _, _ => ()
  | _, _, .kentries .., _, _, _ => ()
  | _, _, .kmap .., _, _, _ => ()
  | _, _, .kallTicks .., _, _, _ => ()
  | _, _, .kfold .., _, _, _ => ()
  | _, _, .ksnapshot .., _, _, _ => ()
  | _, _, .tfold .., _, _, _ => ()
  | _, _, .latest .., _, _, _ => ()
  | _, _, .latest_monotone .., _, _, _ => ()
  | _, _, .snapshot_bounded .., _, _, _ => ()
  | _, _, .tick_scan .., _, _, _ => ()

def Dag.cursorsAt (m : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    Dag.Sched d → Nat → Dag.LocState m d → Dag.Oracle m d
  | _, _, .ret _, _, _, _ => ()
  | _, _, .letN n rest, sch, t, s => (Node.cursorsAt m n sch.1 t s.1, Dag.cursorsAt m rest sch.2 t s.2)

end

mutual

theorem Node.cursorsAt_oracleAt (m : Member L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} →
    (n : Node Γ κs) → (sch : Node.Sched n) → (t : Nat) → (s : Node.State n) →
    Node.OracleAt m n sch (Node.cursorsAt m n sch t (Node.restrict m n s)) t s
  | _, _, .broadcast_closed .., _, _, _ => fun _ _ => rfl
  | _, _, .demux .., _, _, _ => fun _ _ => rfl
  | _, _, .source_interval .., _, _, _ => fun _ => rfl
  | _, _, .time .., _, _, _ => fun _ => rfl
  | _, _, .call N _, sch, t, s => Dag.cursorsAt_oracleAt m N sch t s
  | _, _, .fixN _ body, sch, t, s => Dag.cursorsAt_oracleAt m body sch t s.1
  | _, _, .map .., _, _, _ => trivial
  | _, _, .filterMap .., _, _, _ => trivial
  | _, _, .values .., _, _, _ => trivial
  | _, _, .into_keyed .., _, _, _ => trivial
  | _, _, .kassume_ordering .., _, _, _ => trivial
  | _, _, .kfold_early_stop .., _, _, _ => trivial
  | _, _, .kget_max_key .., _, _, _ => trivial
  | _, _, .weaken_retries .., _, _, _ => trivial
  | _, _, .union .., _, _, _ => trivial
  | _, _, .assume_ordering .., _, _, _ => trivial
  | _, _, .fold .., _, _, _ => trivial
  | _, _, .fold_monotone .., _, _, _ => trivial
  | _, _, .snapshot .., _, _, _ => trivial
  | _, _, .batch .., _, _, _ => trivial
  | _, _, .mapTick .., _, _, _ => trivial
  | _, _, .firstTick .., _, _, _ => trivial
  | _, _, .zipTick .., _, _, _ => trivial
  | _, _, .defer_tick .., _, _, _ => trivial
  | _, _, .allTicks .., _, _, _ => trivial
  | _, _, .flattenOrdered .., _, _, _ => trivial
  | _, _, .flattenUnordered .., _, _, _ => trivial
  | _, _, .tinto_keyed .., _, _, _ => trivial
  | _, _, .tassume_ordering .., _, _, _ => trivial
  | _, _, .reduce_watermark .., _, _, _ => trivial
  | _, _, .kentries .., _, _, _ => trivial
  | _, _, .kmap .., _, _, _ => trivial
  | _, _, .kallTicks .., _, _, _ => trivial
  | _, _, .kfold .., _, _, _ => trivial
  | _, _, .ksnapshot .., _, _, _ => trivial
  | _, _, .tfold .., _, _, _ => trivial
  | _, _, .latest .., _, _, _ => trivial
  | _, _, .latest_monotone .., _, _, _ => trivial
  | _, _, .snapshot_bounded .., _, _, _ => trivial
  | _, _, .tick_scan .., _, _, _ => trivial

theorem Dag.cursorsAt_oracleAt (m : Member L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} →
    (d : Dag Γ outs) → (sch : Dag.Sched d) → (t : Nat) → (s : Dag.State d) →
    Dag.OracleAt m d sch (Dag.cursorsAt m d sch t (Dag.restrict m d s)) t s
  | _, _, .ret _, _, _, _ => trivial
  | _, _, .letN n rest, sch, t, s =>
    ⟨Node.cursorsAt_oracleAt m n sch.1 t s.1, Dag.cursorsAt_oracleAt m rest sch.2 t s.2⟩

end

mutual

/-- **Drained**: the multi-input tick cells hold no complete item — a
`zipTick`/`reduce_watermark` pair has an empty side, a tick block has no
complete tuple pending. Every state
the machine produces is drained (`Dag.step_drained`), and so is the
initial state; it is what makes a step with empty inputs at a non-ticking
member leave these cells alone. -/
def Node.Drained : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) → Node.Sched n → Node.State n → Prop
  | _, _, .zipTick .., _, s => ∀ i, (s i).1 = [] ∨ (s i).2 = []
  | _, _, .reduce_watermark .., _, s => ∀ i, (s i).1.1 = [] ∨ (s i).1.2 = []
  | _, _, .tick_scan _ ins _ _ _ _, _, s => ∀ i, PendingOf.avail ins (s i).1 = 0
  | _, _, .call N _, sch, s => Dag.Drained N sch s
  | _, _, .fixN _ body, sch, s => Dag.Drained body sch s.1
  | _, _, _, _, _ => True

def Dag.Drained : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) → Dag.Sched d → Dag.State d → Prop
  | _, _, .ret _, _, _ => True
  | _, _, .letN n rest, sch, s => Node.Drained n sch.1 s.1 ∧ Dag.Drained rest sch.2 s.2

end

/-! ## Facts about the cells' drained shapes -/

theorem PendingOf.avail_dropN : (sh : TickShape) → (p : PendingOf sh) → (n : Nat) →
    PendingOf.avail sh (PendingOf.dropN sh p n) = PendingOf.avail sh p - n
  | .sing _, p, n => by simp [PendingOf.avail, PendingOf.dropN]
  | .stream _ _ _ _, p, n => by simp [PendingOf.avail, PendingOf.dropN]
  | .ksing _ _ _ _, p, n => by simp [PendingOf.avail, PendingOf.dropN]
  | .kstream _ _ _ _ _ _, p, n => by simp [PendingOf.avail, PendingOf.dropN]
  | .pair a b, (pa, pb), n => by
    simp only [PendingOf.avail, PendingOf.dropN, PendingOf.avail_dropN a pa n, PendingOf.avail_dropN b pb n]
    omega

theorem PendingOf.avail_empty : (sh : TickShape) → PendingOf.avail sh (PendingOf.empty sh) = 0
  | .sing _ => rfl
  | .stream _ _ _ _ => rfl
  | .ksing _ _ _ _ => rfl
  | .kstream _ _ _ _ _ _ => rfl
  | .pair a b => by
    simp only [PendingOf.avail, PendingOf.empty, PendingOf.avail_empty a, PendingOf.avail_empty b, Nat.min_self]

theorem PendingOf.avail_dropN_self (sh : TickShape) (p : PendingOf sh) :
    PendingOf.avail sh (PendingOf.dropN sh p (PendingOf.avail sh p)) = 0 := by
  rw [PendingOf.avail_dropN]; omega

/-! ## Shaped tick inputs, cut to a process -/

theorem readShapeLoc_restrict (m : Member L mem) {Γ : Ctx L mem} (Δ : HList (Kind.delta (mem := mem)) Γ) (ℓ : L) :
    (sh : TickShape) → (vs : VarsOf Γ ℓ sh) →
    readShapeLoc m (envRestrict m Δ) ℓ sh vs = envRestrict m (readShapeDelta Δ ℓ sh vs)
  | .sing _, v => by
    show (_, ()) = (Kind.restrictDelta m _ (Δ.get v), ())
    rw [envRestrict_get]
  | .stream _ _ _ _, v => by
    show (_, ()) = (Kind.restrictDelta m _ (Δ.get v), ())
    rw [envRestrict_get]
  | .ksing _ _ _ _, v => by
    show (_, ()) = (Kind.restrictDelta m _ (Δ.get v), ())
    rw [envRestrict_get]
  | .kstream _ _ _ _ _ _, v => by
    show (_, ()) = (Kind.restrictDelta m _ (Δ.get v), ())
    rw [envRestrict_get]
  | .pair a b, (va, vb) => by
    simp only [readShapeLoc, readShapeDelta, readShapeLoc_restrict m Δ ℓ a va, readShapeLoc_restrict m Δ ℓ b vb]
    exact (envRestrict_append m _ _).symm

theorem readShapeDelta_idle (m : Member L mem) {Γ : Ctx L mem} (Δ : HList (Kind.delta (mem := mem)) Γ)
    (hΔ : IdleEnv m Δ) (ℓ : L) : (sh : TickShape) → (vs : VarsOf Γ ℓ sh) →
    IdleEnv m (readShapeDelta Δ ℓ sh vs)
  | .sing _, v => ⟨IdleEnv.get m Δ hΔ v, trivial⟩
  | .stream _ _ _ _, v => ⟨IdleEnv.get m Δ hΔ v, trivial⟩
  | .ksing _ _ _ _, v => ⟨IdleEnv.get m Δ hΔ v, trivial⟩
  | .kstream _ _ _ _ _ _, v => ⟨IdleEnv.get m Δ hΔ v, trivial⟩
  | .pair a b, (va, vb) =>
    IdleEnv.append m _ _ (readShapeDelta_idle m Δ hΔ ℓ a va) (readShapeDelta_idle m Δ hΔ ℓ b vb)

/-- Pushing the member's leaves through `Hosted` is pushing that member's leaves. -/
theorem PendingOf.pushLoc_restrict (m : Member L mem) {ℓ : L} : (sh : TickShape) → (p : PendingOf sh) →
    (D : Fin (mem ℓ) → HList (Kind.delta (mem := mem)) (shapeKinds ℓ sh)) → (i : Hosted m ℓ) →
    PendingOf.pushLoc m sh p (fun i => envRestrict m (D i.1)) i = PendingOf.push sh p D i.1
  | .sing _, p, D, i => rfl
  | .stream _ _ _ _, p, D, i => rfl
  | .ksing _ _ _ _, p, D, i => rfl
  | .kstream _ _ _ _ _ _, p, D, i => rfl
  | .pair a b, (pa, pb), D, i => by
    have ha : (fun (i : Hosted m ℓ) => (envRestrict m (D i.1)).take)
        = fun i => envRestrict m (HList.take (κs := shapeKinds ℓ a) (Γ := shapeKinds ℓ b) (D i.1)) :=
      funext fun i => (envRestrict_take m (κs := shapeKinds ℓ a) (Γ := shapeKinds ℓ b) (D i.1)).symm
    have hb : (fun (i : Hosted m ℓ) => (envRestrict m (D i.1)).drop)
        = fun i => envRestrict m (HList.drop (κs := shapeKinds ℓ a) (Γ := shapeKinds ℓ b) (D i.1)) :=
      funext fun i => (envRestrict_drop m (κs := shapeKinds ℓ a) (Γ := shapeKinds ℓ b) (D i.1)).symm
    show (PendingOf.pushLoc m a pa (fun i => (envRestrict m (D i.1)).take) i,
        PendingOf.pushLoc m b pb (fun i => (envRestrict m (D i.1)).drop) i)
      = (PendingOf.push a pa (fun i => (D i).take) i.1, PendingOf.push b pb (fun i => (D i).drop) i.1)
    rw [ha, hb, PendingOf.pushLoc_restrict m a pa (fun i => (D i).take) i,
      PendingOf.pushLoc_restrict m b pb (fun i => (D i).drop) i]

/-- Pushing empty leaves changes nothing. -/
theorem PendingOf.push_idle (m : Member L mem) {ℓ : L} : (sh : TickShape) → (p : PendingOf sh) →
    (D : Fin (mem ℓ) → HList (Kind.delta (mem := mem)) (shapeKinds ℓ sh)) → (i : Fin (mem ℓ)) →
    IdleEnv m (D i) → (⟨ℓ, i⟩ : Member L mem) ≠ m → PendingOf.push sh p D i = p
  | .sing _, p, D, i, h, hi => by
    show p ++ ((D i).1 i).toList = p
    rw [h.1 i hi, Option.toList_none, List.append_nil]
  | .stream _ _ _ _, p, D, i, h, hi => by
    show p ++ ((D i).1 i).toList = p
    rw [h.1 i hi, Option.toList_none, List.append_nil]
  | .ksing _ _ _ _, p, D, i, h, hi => by
    show p ++ ((D i).1 i).toList = p
    rw [h.1 i hi, Option.toList_none, List.append_nil]
  | .kstream _ _ _ _ _ _, p, D, i, h, hi => by
    show p ++ ((D i).1 i).toList = p
    rw [h.1 i hi, Option.toList_none, List.append_nil]
  | .pair a b, (pa, pb), D, i, h, hi => by
    simp only [PendingOf.push]
    rw [PendingOf.push_idle m a pa (fun i => (D i).take) i (IdleEnv.take m (D i) h) hi,
      PendingOf.push_idle m b pb (fun i => (D i).drop) i (IdleEnv.drop m (D i) h) hi]

theorem PendingOf.takeZip_zero : (sh : TickShape) → (p : PendingOf sh) →
    PendingOf.takeZip sh p 0 = []
  | .sing _, _ => rfl
  | .stream _ _ _ _, _ => rfl
  | .ksing _ _ _ _, _ => rfl
  | .kstream _ _ _ _ _ _, _ => rfl
  | .pair a b, (pa, pb) => by
    simp only [PendingOf.takeZip, PendingOf.takeZip_zero a pa, PendingOf.takeZip_zero b pb, List.zip_nil_left]

theorem PendingOf.dropN_zero : (sh : TickShape) → (p : PendingOf sh) →
    PendingOf.dropN sh p 0 = p
  | .sing _, _ => rfl
  | .stream _ _ _ _, _ => rfl
  | .ksing _ _ _ _, _ => rfl
  | .kstream _ _ _ _ _ _, _ => rfl
  | .pair a b, (pa, pb) => by
    simp only [PendingOf.dropN, PendingOf.dropN_zero a pa, PendingOf.dropN_zero b pb]

theorem unzipShape_restrict (m : Member L mem) (ℓ : L) : (sh : TickShape) →
    (l : Fin (mem ℓ) → Option (BoundedOf BS BStr BKS BKStr sh)) →
    envRestrict m (unzipShape ℓ sh l) = unzipShapeLoc m ℓ sh (fun i => l i.1)
  | .sing _, _ => rfl
  | .stream _ _ _ _, _ => rfl
  | .ksing _ _ _ _, _ => rfl
  | .kstream _ _ _ _ _ _, _ => rfl
  | .pair a b, l => by
    show envRestrict m ((unzipShape ℓ a fun i => (l i).map Prod.fst).append (unzipShape ℓ b fun i => (l i).map Prod.snd))
      = (unzipShapeLoc m ℓ a fun i => (l i.1).map Prod.fst).append (unzipShapeLoc m ℓ b fun i => (l i.1).map Prod.snd)
    rw [envRestrict_append, unzipShape_restrict m ℓ a, unzipShape_restrict m ℓ b]

theorem unzipShape_idle (m : Member L mem) (ℓ : L) : (sh : TickShape) →
    (l : Fin (mem ℓ) → Option (BoundedOf BS BStr BKS BKStr sh)) → (∀ i, (⟨ℓ, i⟩ : Member L mem) ≠ m → l i = none) →
    IdleEnv m (unzipShape ℓ sh l)
  | .sing _, _, h => ⟨h, trivial⟩
  | .stream _ _ _ _, _, h => ⟨h, trivial⟩
  | .ksing _ _ _ _, _, h => ⟨h, trivial⟩
  | .kstream _ _ _ _ _ _, _, h => ⟨h, trivial⟩
  | .pair a b, l, h =>
    IdleEnv.append m _ _ (unzipShape_idle m ℓ a (fun i => (l i).map Prod.fst) (fun i hi => by rw [h i hi]; rfl))
      (unzipShape_idle m ℓ b (fun i => (l i).map Prod.snd) (fun i hi => by rw [h i hi]; rfl))

/-- The cells drained after any step (what `Node.Inv` demands of the stateful tick cells). -/
theorem zip_drop_min_empty {α β : Type} (a : List α) (b : List β) :
    a.drop (min a.length b.length) = [] ∨ b.drop (min a.length b.length) = [] := by
  rcases Nat.le_total a.length b.length with h | h
  · left; rw [Nat.min_eq_left h]; simp
  · right; rw [Nat.min_eq_right h]; simp

end Project

end Hydro
