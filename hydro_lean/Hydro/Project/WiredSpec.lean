import Hydro.Project.Wired

/-!
# Hydro · Project — wired machine: predicates and environment lemmas

The hypotheses under which a lock-step step *is* one process's step
(`Project/WiredMain.lean`), and the plumbing lemmas on cut/overridden
increment environments.

- `Node.OracleAt`: the oracle reads the lock-step cursors at `t`;
  `Node.cursorsAt` is the oracle that does;
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

theorem envRestrict_get (sel : Sel L mem) : {Γ : Ctx L mem} → {κ : Kind L mem} →
    (Δ : HList (Kind.delta (mem := mem)) Γ) → (v : Var Γ κ) →
    (envRestrict sel Δ).get v = Kind.restrictDelta sel κ (Δ.get v)
  | _, _, _, .zero => rfl
  | _, _, Δ, .succ v => envRestrict_get sel Δ.2 v

theorem envRestrict_append (sel : Sel L mem) : {κs Γ : List (Kind L mem)} →
    (xs : HList (Kind.delta (mem := mem)) κs) → (ys : HList (Kind.delta (mem := mem)) Γ) →
    envRestrict sel (xs.append ys) = (envRestrict sel xs).append (envRestrict sel ys)
  | [], _, _, _ => rfl
  | _ :: _, _, xs, ys => by
    show (Kind.restrictDelta sel _ xs.1, envRestrict sel (xs.2.append ys))
      = (Kind.restrictDelta sel _ xs.1, (envRestrict sel xs.2).append (envRestrict sel ys))
    rw [envRestrict_append sel xs.2 ys]

theorem envRestrict_take (sel : Sel L mem) : {κs Γ : List (Kind L mem)} →
    (xs : HList (Kind.delta (mem := mem)) (κs ++ Γ)) →
    envRestrict sel (HList.take (κs := κs) xs) = HList.take (κs := κs) (envRestrict sel xs)
  | [], _, _ => rfl
  | _ :: κs, Γ, xs => by
    show (Kind.restrictDelta sel _ xs.1, envRestrict sel (HList.take (κs := κs) xs.2))
      = (Kind.restrictDelta sel _ xs.1, HList.take (κs := κs) (envRestrict sel xs.2))
    rw [envRestrict_take sel (κs := κs) (Γ := Γ) xs.2]

theorem envRestrict_drop (sel : Sel L mem) : {κs Γ : List (Kind L mem)} →
    (xs : HList (Kind.delta (mem := mem)) (κs ++ Γ)) →
    envRestrict sel (HList.drop (κs := κs) xs) = HList.drop (κs := κs) (envRestrict sel xs)
  | [], _, _ => rfl
  | _ :: κs, Γ, xs => envRestrict_drop sel (κs := κs) (Γ := Γ) xs.2

theorem envRestrict_mapSubst (sel : Sel L mem) {Γ Γ' : Ctx L mem} (Δ : HList (Kind.delta (mem := mem)) Γ)
    (args : Subst Γ' Γ) :
    envRestrict sel (args.map (fun v => Δ.get v)) = args.map (fun v => (envRestrict sel Δ).get v) := by
  induction Γ' with
  | nil => rfl
  | cons κ κs ih =>
    show (Kind.restrictDelta sel _ (Δ.get args.1), envRestrict sel (args.2.map (fun v => Δ.get v)))
      = ((envRestrict sel Δ).get args.1, args.2.map (fun v => (envRestrict sel Δ).get v))
    rw [ih args.2, envRestrict_get]

theorem IdleEnv.get (sel : Sel L mem) : {Γ : Ctx L mem} → {κ : Kind L mem} →
    (Δ : HList (Kind.delta (mem := mem)) Γ) → IdleEnv sel Δ → (v : Var Γ κ) →
    Kind.idleDelta sel κ (Δ.get v)
  | _, _, _, h, .zero => h.1
  | _, _, Δ, h, .succ v => IdleEnv.get sel Δ.2 h.2 v

section IdleGet
variable (sel : Sel L mem) {Γ : Ctx L mem} {ℓ : L} {Δ : HList (Kind.delta (mem := mem)) Γ} (hΔ : IdleEnv sel Δ)
include hΔ

theorem IdleEnv.get_stream {α : Type} {iα : DecidableEq α} {ord : StrOrd} {ret : Retries}
    (x : Var Γ (.stream ℓ α iα ord ret)) (i : Fin (mem ℓ)) (hi : sel ℓ i = false) : Δ.get x i = [] :=
  IdleEnv.get sel Δ hΔ x i hi

theorem IdleEnv.get_keyed {K V : Type} {iK : DecidableEq K} {iV : DecidableEq V} {ord : StrOrd} {ret : Retries}
    (x : Var Γ (.keyed ℓ K V iK iV ord ret)) (i : Fin (mem ℓ)) (hi : sel ℓ i = false) : Δ.get x i = [] :=
  IdleEnv.get sel Δ hΔ x i hi

theorem IdleEnv.get_atomic {σ : Type} (x : Var Γ (.atomic ℓ σ)) (i : Fin (mem ℓ)) (hi : sel ℓ i = false) :
    Δ.get x i = [] :=
  IdleEnv.get sel Δ hΔ x i hi

theorem IdleEnv.get_atomicStream {α : Type} {iα : DecidableEq α} {ord : StrOrd} {ret : Retries}
    (x : Var Γ (.atomicStream ℓ α iα ord ret)) (i : Fin (mem ℓ)) (hi : sel ℓ i = false) : Δ.get x i = [] :=
  IdleEnv.get sel Δ hΔ x i hi

theorem IdleEnv.get_atomicKeyed {K V : Type} {iK : DecidableEq K} {iV : DecidableEq V} {ord : StrOrd} {ret : Retries}
    (x : Var Γ (.atomicKeyed ℓ K V iK iV ord ret)) (i : Fin (mem ℓ)) (hi : sel ℓ i = false) : Δ.get x i = [] :=
  IdleEnv.get sel Δ hΔ x i hi

end IdleGet

theorem IdleEnv.append (sel : Sel L mem) : {κs Γ : List (Kind L mem)} →
    (xs : HList (Kind.delta (mem := mem)) κs) → (ys : HList (Kind.delta (mem := mem)) Γ) →
    IdleEnv sel xs → IdleEnv sel ys → IdleEnv sel (xs.append ys)
  | [], _, _, _, _, hy => hy
  | _ :: _, _, xs, ys, hx, hy => ⟨hx.1, IdleEnv.append sel xs.2 ys hx.2 hy⟩

theorem IdleEnv.take (sel : Sel L mem) : {κs Γ : List (Kind L mem)} →
    (xs : HList (Kind.delta (mem := mem)) (κs ++ Γ)) → IdleEnv sel xs → IdleEnv sel (HList.take (κs := κs) xs)
  | [], _, _, _ => trivial
  | _ :: κs, Γ, xs, h => ⟨h.1, IdleEnv.take sel (κs := κs) (Γ := Γ) xs.2 h.2⟩

theorem IdleEnv.drop (sel : Sel L mem) : {κs Γ : List (Kind L mem)} →
    (xs : HList (Kind.delta (mem := mem)) (κs ++ Γ)) → IdleEnv sel xs → IdleEnv sel (HList.drop (κs := κs) xs)
  | [], _, _, h => h
  | _ :: κs, Γ, xs, h => IdleEnv.drop sel (κs := κs) (Γ := Γ) xs.2 h.2

theorem IdleEnv.mapSubst (sel : Sel L mem) {Γ Γ' : Ctx L mem} (Δ : HList (Kind.delta (mem := mem)) Γ)
    (h : IdleEnv sel Δ) (args : Subst Γ' Γ) : IdleEnv sel (args.map (fun v => Δ.get v)) := by
  induction Γ' with
  | nil => trivial
  | cons κ κs ih => exact ⟨IdleEnv.get sel Δ h args.1, ih args.2⟩

theorem Kind.idleDelta_of_allEmpty (sel : Sel L mem) : (κ : Kind L mem) → (Δ : κ.delta) →
    Kind.allEmpty κ Δ → Kind.idleDelta sel κ Δ
  | .stream .., _, h => fun i _ => h i
  | .keyed .., _, h => fun i _ => h i
  | .sing .., _, _ => trivial
  | .ksing .., _, _ => trivial
  | .atomic .., _, h => fun i _ => h i
  | .atomicStream .., _, h => fun i _ => h i
  | .atomicKeyed .., _, h => fun i _ => h i
  | .atomicKSing .., _, h => fun i _ => h i

theorem IdleEnv.of_empty (sel : Sel L mem) : {Γ : Ctx L mem} → (Δ : HList (Kind.delta (mem := mem)) Γ) →
    EmptyEnv Δ → IdleEnv sel Δ
  | [], _, _ => trivial
  | _ :: _, Δ, h => ⟨Kind.idleDelta_of_allEmpty sel _ Δ.1 h.1, IdleEnv.of_empty sel Δ.2 h.2⟩

/-- The list-like kinds (a singleton's increment is a value, not a list). -/
def Kind.listLike : Kind L mem → Prop
  | .sing .. => False
  | .ksing .. => False
  | _ => True

theorem KnotKind.toKind_listLike : (k : KnotKind L mem) → k.toKind.listLike
  | .stream .. => trivial
  | .atomic .. => trivial

/-- Writing a process's increment back over an idle one gives the increment
back when the increment is itself idle (list-like kinds). -/
theorem Kind.overrideDelta_restrict (sel : Sel L mem) : (κ : Kind L mem) → κ.listLike →
    (Δ0 Δ : κ.delta) → Kind.idleDelta sel κ Δ0 → Kind.idleDelta sel κ Δ →
    Kind.overrideDelta sel κ Δ0 (Kind.restrictDelta sel κ Δ) = Δ
  | .stream ℓ .., _, Δ0, Δ, h0, h => by
    funext i
    simp only [Kind.overrideDelta, Kind.restrictDelta]
    split
    · rfl
    · rename_i hn
      rw [h0 i (Bool.eq_false_iff.mpr hn), h i (Bool.eq_false_iff.mpr hn)]
  | .keyed ℓ .., _, Δ0, Δ, h0, h => by
    funext i
    simp only [Kind.overrideDelta, Kind.restrictDelta]
    split
    · rfl
    · rename_i hn
      rw [h0 i (Bool.eq_false_iff.mpr hn), h i (Bool.eq_false_iff.mpr hn)]
  | .sing .., hl, _, _, _, _ => hl.elim
  | .ksing .., hl, _, _, _, _ => hl.elim
  | .atomic ℓ .., _, Δ0, Δ, h0, h => by
    funext i
    simp only [Kind.overrideDelta, Kind.restrictDelta]
    split
    · rfl
    · rename_i hn
      rw [h0 i (Bool.eq_false_iff.mpr hn), h i (Bool.eq_false_iff.mpr hn)]
  | .atomicStream ℓ .., _, Δ0, Δ, h0, h => by
    funext i
    simp only [Kind.overrideDelta, Kind.restrictDelta]
    split
    · rfl
    · rename_i hn
      rw [h0 i (Bool.eq_false_iff.mpr hn), h i (Bool.eq_false_iff.mpr hn)]
  | .atomicKeyed ℓ .., _, Δ0, Δ, h0, h => by
    funext i
    simp only [Kind.overrideDelta, Kind.restrictDelta]
    split
    · rfl
    · rename_i hn
      rw [h0 i (Bool.eq_false_iff.mpr hn), h i (Bool.eq_false_iff.mpr hn)]
  | .atomicKSing ℓ .., _, Δ0, Δ, h0, h => by
    funext i
    simp only [Kind.overrideDelta, Kind.restrictDelta]
    split
    · rfl
    · rename_i hn
      rw [h0 i (Bool.eq_false_iff.mpr hn), h i (Bool.eq_false_iff.mpr hn)]

/-- Every kind of the context is list-like. -/
def ListLikeCtx : Ctx L mem → Prop
  | [] => True
  | κ :: Γ => κ.listLike ∧ ListLikeCtx Γ

theorem ListLikeCtx.knots : (ks : List (KnotKind L mem)) → ListLikeCtx (ks.map KnotKind.toKind)
  | [] => trivial
  | k :: ks => ⟨KnotKind.toKind_listLike k, ListLikeCtx.knots ks⟩

theorem envOverride_restrict (sel : Sel L mem) : {Γ : Ctx L mem} → ListLikeCtx Γ →
    (Δ0 Δ : HList (Kind.delta (mem := mem)) Γ) → IdleEnv sel Δ0 → IdleEnv sel Δ →
    envOverride sel Δ0 (envRestrict sel Δ) = Δ
  | [], _, _, _, _, _ => rfl
  | _ :: _, hl, Δ0, Δ, h0, h => by
    show (Kind.overrideDelta sel _ Δ0.1 (Kind.restrictDelta sel _ Δ.1), envOverride sel Δ0.2 (envRestrict sel Δ.2)) = Δ
    rw [Kind.overrideDelta_restrict sel _ hl.1 Δ0.1 Δ.1 h0.1 h.1, envOverride_restrict sel hl.2 Δ0.2 Δ.2 h0.2 h.2]

/-- The generic write-back: a per-member cell map whose idle members are unchanged. -/
theorem override_pointwise (sel : Sel L mem) (ℓ : L) {X : Type} (s F : Fin (mem ℓ) → X)
    (hidle : ∀ i, sel ℓ i = false → F i = s i) :
    (fun i => if _ : sel ℓ i = true then F i else s i) = F := by
  funext i
  split
  · rfl
  · rename_i hn
    exact (hidle i (Bool.eq_false_iff.mpr hn)).symm

/-! ## The oracle and the drained cells -/

mutual

/-- The oracle reads the lock-step cursors at step `t`. -/
def Node.OracleAt (sel : Sel L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) →
    Node.Oracle sel n → Nat → Prop
  | _, _, .broadcast_closed _ d _, ora, t => ∀ i j, ora i j = cumMax (d i.1 j) t
  | _, _, .demux _ d _ _, ora, t => ∀ i j, ora i j = cumMax (d i.1 j) t
  | _, _, .call N _, ora, t => Dag.OracleAt sel N ora t
  | _, _, .fixN _ body, ora, t => Dag.OracleAt sel body ora t
  | _, _, _, _, _ => True

def Dag.OracleAt (sel : Sel L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) →
    Dag.Oracle sel d → Nat → Prop
  | _, _, .ret _, _, _ => True
  | _, _, .letN n rest, ora, t => Node.OracleAt sel n ora.1 t ∧ Dag.OracleAt sel rest ora.2 t

end

mutual

/-- The oracle that reads the graph's own (baked) cursors at step `t`. -/
def Node.cursorsAt (sel : Sel L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) → Nat →
    Node.Oracle sel n
  | _, _, .broadcast_closed _ d _, t => fun i j => cumMax (d i.1 j) t
  | _, _, .demux _ d _ _, t => fun i j => cumMax (d i.1 j) t
  | _, _, .call N _, t => Dag.cursorsAt sel N t
  | _, _, .fixN _ body, t => Dag.cursorsAt sel body t
  | _, _, .map .., _ => ()
  | _, _, .filterMap .., _ => ()
  | _, _, .values .., _ => ()
  | _, _, .into_keyed .., _ => ()
  | _, _, .kassume_ordering .., _ => ()
  | _, _, .kfold_early_stop .., _ => ()
  | _, _, .kget_max_key .., _ => ()
  | _, _, .weaken_retries .., _ => ()
  | _, _, .union .., _ => ()
  | _, _, .assume_ordering .., _ => ()
  | _, _, .fold .., _ => ()
  | _, _, .fold_monotone .., _ => ()
  | _, _, .snapshot .., _ => ()
  | _, _, .batch .., _ => ()
  | _, _, .batch_ordered .., _ => ()
  | _, _, .sample_every .., _ => ()
  | _, _, .timeout_snapshot .., _ => ()
  | _, _, .source_interval_batch .., _ => ()
  | _, _, .mapTick .., _ => ()
  | _, _, .zipTick .., _ => ()
  | _, _, .defer_tick .., _ => ()
  | _, _, .allTicks .., _ => ()
  | _, _, .flattenOrdered .., _ => ()
  | _, _, .flattenUnordered .., _ => ()
  | _, _, .tinto_keyed .., _ => ()
  | _, _, .tassume_ordering .., _ => ()
  | _, _, .reduce_watermark .., _ => ()
  | _, _, .tick_scan .., _ => ()

def Dag.cursorsAt (sel : Sel L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) → Nat →
    Dag.Oracle sel d
  | _, _, .ret _, _ => ()
  | _, _, .letN n rest, t => (Node.cursorsAt sel n t, Dag.cursorsAt sel rest t)

end

mutual

theorem Node.cursorsAt_oracleAt (sel : Sel L mem) : {Γ : Ctx L mem} → {κs : List (Kind L mem)} →
    (n : Node Γ κs) → (t : Nat) → Node.OracleAt sel n (Node.cursorsAt sel n t) t
  | _, _, .broadcast_closed .., _ => fun _ _ => rfl
  | _, _, .demux .., _ => fun _ _ => rfl
  | _, _, .call N _, t => Dag.cursorsAt_oracleAt sel N t
  | _, _, .fixN _ body, t => Dag.cursorsAt_oracleAt sel body t
  | _, _, .map .., _ => trivial
  | _, _, .filterMap .., _ => trivial
  | _, _, .values .., _ => trivial
  | _, _, .into_keyed .., _ => trivial
  | _, _, .kassume_ordering .., _ => trivial
  | _, _, .kfold_early_stop .., _ => trivial
  | _, _, .kget_max_key .., _ => trivial
  | _, _, .weaken_retries .., _ => trivial
  | _, _, .union .., _ => trivial
  | _, _, .assume_ordering .., _ => trivial
  | _, _, .fold .., _ => trivial
  | _, _, .fold_monotone .., _ => trivial
  | _, _, .snapshot .., _ => trivial
  | _, _, .batch .., _ => trivial
  | _, _, .batch_ordered .., _ => trivial
  | _, _, .sample_every .., _ => trivial
  | _, _, .timeout_snapshot .., _ => trivial
  | _, _, .source_interval_batch .., _ => trivial
  | _, _, .mapTick .., _ => trivial
  | _, _, .zipTick .., _ => trivial
  | _, _, .defer_tick .., _ => trivial
  | _, _, .allTicks .., _ => trivial
  | _, _, .flattenOrdered .., _ => trivial
  | _, _, .flattenUnordered .., _ => trivial
  | _, _, .tinto_keyed .., _ => trivial
  | _, _, .tassume_ordering .., _ => trivial
  | _, _, .reduce_watermark .., _ => trivial
  | _, _, .tick_scan .., _ => trivial

theorem Dag.cursorsAt_oracleAt (sel : Sel L mem) : {Γ : Ctx L mem} → {outs : List (Kind L mem)} →
    (d : Dag Γ outs) → (t : Nat) → Dag.OracleAt sel d (Dag.cursorsAt sel d t) t
  | _, _, .ret _, _ => trivial
  | _, _, .letN n rest, t => ⟨Node.cursorsAt_oracleAt sel n t, Dag.cursorsAt_oracleAt sel rest t⟩

end

mutual

/-- **Drained**: the multi-input tick cells hold no complete item — a
`zipTick`/`reduce_watermark` pair has an empty side, a tick block has no
complete tuple pending, a sampler is at its greedy position. Every state
the machine produces is drained (`Dag.step_drained`), and so is the
initial state; it is what makes a step with empty inputs at a non-ticking
member leave these cells alone. -/
def Node.Drained : {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) → Node.State n → Prop
  | _, _, .zipTick .., s => ∀ i, (s i).1 = [] ∨ (s i).2 = []
  | _, _, .reduce_watermark .., s => ∀ i, (s i).1.1 = [] ∨ (s i).1.2 = []
  | _, _, .tick_scan _ ins _ _ _ _, s => ∀ i, PendingOf.avail ins (s i).1 = 0
  | _, _, .sample_every _ _ d, s => ∀ i, sampleAdvance (s i).1 (d i) (s i).2 = ((s i).2, [])
  | _, _, .call N _, s => Dag.Drained N s
  | _, _, .fixN _ body, s => Dag.Drained body s.1
  | _, _, _, _ => True

def Dag.Drained : {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) → Dag.State d → Prop
  | _, _, .ret _, _ => True
  | _, _, .letN n rest, s => Node.Drained n s.1 ∧ Dag.Drained rest s.2

end

/-! ## Facts about the cells' drained shapes -/

theorem ChanCell.step_idle {α : Type} (c : Nat) (cell : ChanCell α) (h : c ≤ cell.delivered) (sends : List α) :
    cell.step c sends = (cell.enqueue sends, []) := by
  simp only [ChanCell.step, ChanCell.enqueue, Nat.sub_eq_zero_of_le h, List.take_zero, List.length_nil,
    List.drop_zero, Nat.add_zero]

theorem ChanCell.step_saturates {α : Type} (c : Nat) (cell : ChanCell α) (sends : List α)
    (hmono : cell.delivered ≤ c) (hfeas : c ≤ cell.delivered + cell.pending.length) :
    (cell.step c sends).1.delivered = c := by
  simp only [ChanCell.step, List.length_take]
  omega

theorem ChanCell.step_delivered_ge {α : Type} (c : Nat) (cell : ChanCell α) (sends : List α) :
    cell.delivered ≤ (cell.step c sends).1.delivered := by
  simp only [ChanCell.step]
  omega

theorem ChanCell.step_delivered_le {α : Type} (c : Nat) (cell : ChanCell α) (sends : List α)
    (hmono : cell.delivered ≤ c) : (cell.step c sends).1.delivered ≤ c := by
  simp only [ChanCell.step, List.length_take]
  omega

theorem sampleAdvance_fix {α : Type} (tr : Trace (Option α)) (us : List Nat) (pos : Nat) :
    sampleAdvance tr us (sampleAdvance tr us pos).1 = ((sampleAdvance tr us pos).1, []) := by
  induction pos using sampleAdvance.induct tr us with
  | case1 pos h =>
    have e : sampleAdvance tr us pos = (pos, []) := by rw [sampleAdvance]; split <;> simp_all
    rw [e]; exact e
  | case2 pos u h hu =>
    have e : sampleAdvance tr us pos = (pos, []) := by rw [sampleAdvance]; split <;> (try split) <;> simp_all
    rw [e]; exact e
  | case3 pos u h hu ih =>
    have e : sampleAdvance tr us pos = sampleAdvance tr us (pos + 1) := by rw [sampleAdvance]; split <;> (try split) <;> simp_all
    rw [e]; exact ih
  | case4 pos u h x hu ih =>
    have e : sampleAdvance tr us pos
        = ((sampleAdvance tr us (pos + 1)).1, x :: (sampleAdvance tr us (pos + 1)).2) := by
      rw [sampleAdvance]; split <;> (try split) <;> simp_all
    rw [e]; exact ih

theorem PendingOf.avail_dropN : (sh : TickShape) → (p : PendingOf sh) → (n : Nat) →
    PendingOf.avail sh (PendingOf.dropN sh p n) = PendingOf.avail sh p - n
  | .sing _, p, n => by simp [PendingOf.avail, PendingOf.dropN]
  | .stream _ _ _ _, p, n => by simp [PendingOf.avail, PendingOf.dropN]
  | .ksing _ _ _ _, p, n => by simp [PendingOf.avail, PendingOf.dropN]
  | .pair a b, (pa, pb), n => by
    simp only [PendingOf.avail, PendingOf.dropN, PendingOf.avail_dropN a pa n, PendingOf.avail_dropN b pb n]
    omega

theorem PendingOf.avail_empty : (sh : TickShape) → PendingOf.avail sh (PendingOf.empty sh) = 0
  | .sing _ => rfl
  | .stream _ _ _ _ => rfl
  | .ksing _ _ _ _ => rfl
  | .pair a b => by
    simp only [PendingOf.avail, PendingOf.empty, PendingOf.avail_empty a, PendingOf.avail_empty b, Nat.min_self]

theorem PendingOf.avail_dropN_self (sh : TickShape) (p : PendingOf sh) :
    PendingOf.avail sh (PendingOf.dropN sh p (PendingOf.avail sh p)) = 0 := by
  rw [PendingOf.avail_dropN]; omega

/-! ## Shaped tick inputs, cut to a process -/

theorem readShapeLoc_restrict (sel : Sel L mem) {Γ : Ctx L mem} (Δ : HList (Kind.delta (mem := mem)) Γ) (ℓ : L) :
    (sh : TickShape) → (vs : VarsOf Γ ℓ sh) →
    readShapeLoc sel (envRestrict sel Δ) ℓ sh vs = envRestrict sel (readShapeDelta Δ ℓ sh vs)
  | .sing _, v => by
    show (_, ()) = (Kind.restrictDelta sel _ (Δ.get v), ())
    rw [envRestrict_get]
  | .stream _ _ _ _, v => by
    show (_, ()) = (Kind.restrictDelta sel _ (Δ.get v), ())
    rw [envRestrict_get]
  | .ksing _ _ _ _, v => by
    show (_, ()) = (Kind.restrictDelta sel _ (Δ.get v), ())
    rw [envRestrict_get]
  | .pair a b, (va, vb) => by
    simp only [readShapeLoc, readShapeDelta, readShapeLoc_restrict sel Δ ℓ a va, readShapeLoc_restrict sel Δ ℓ b vb]
    exact (envRestrict_append sel _ _).symm

theorem readShapeDelta_idle (sel : Sel L mem) {Γ : Ctx L mem} (Δ : HList (Kind.delta (mem := mem)) Γ)
    (hΔ : IdleEnv sel Δ) (ℓ : L) : (sh : TickShape) → (vs : VarsOf Γ ℓ sh) →
    IdleEnv sel (readShapeDelta Δ ℓ sh vs)
  | .sing _, v => ⟨IdleEnv.get sel Δ hΔ v, trivial⟩
  | .stream _ _ _ _, v => ⟨IdleEnv.get sel Δ hΔ v, trivial⟩
  | .ksing _ _ _ _, v => ⟨IdleEnv.get sel Δ hΔ v, trivial⟩
  | .pair a b, (va, vb) =>
    IdleEnv.append sel _ _ (readShapeDelta_idle sel Δ hΔ ℓ a va) (readShapeDelta_idle sel Δ hΔ ℓ b vb)

/-- Pushing a hosted member's leaves is pushing that member's leaves. -/
theorem PendingOf.pushLoc_restrict (sel : Sel L mem) {ℓ : L} : (sh : TickShape) → (p : PendingOf sh) →
    (D : Fin (mem ℓ) → HList (Kind.delta (mem := mem)) (shapeKinds ℓ sh)) → (i : Hosted sel ℓ) →
    PendingOf.pushLoc sel sh p (fun i => envRestrict sel (D i.1)) i = PendingOf.push sh p D i.1
  | .sing _, p, D, i => rfl
  | .stream _ _ _ _, p, D, i => rfl
  | .ksing _ _ _ _, p, D, i => rfl
  | .pair a b, (pa, pb), D, i => by
    have ha : (fun (i : Hosted sel ℓ) => (envRestrict sel (D i.1)).take)
        = fun i => envRestrict sel (HList.take (κs := shapeKinds ℓ a) (Γ := shapeKinds ℓ b) (D i.1)) :=
      funext fun i => (envRestrict_take sel (κs := shapeKinds ℓ a) (Γ := shapeKinds ℓ b) (D i.1)).symm
    have hb : (fun (i : Hosted sel ℓ) => (envRestrict sel (D i.1)).drop)
        = fun i => envRestrict sel (HList.drop (κs := shapeKinds ℓ a) (Γ := shapeKinds ℓ b) (D i.1)) :=
      funext fun i => (envRestrict_drop sel (κs := shapeKinds ℓ a) (Γ := shapeKinds ℓ b) (D i.1)).symm
    show (PendingOf.pushLoc sel a pa (fun i => (envRestrict sel (D i.1)).take) i,
        PendingOf.pushLoc sel b pb (fun i => (envRestrict sel (D i.1)).drop) i)
      = (PendingOf.push a pa (fun i => (D i).take) i.1, PendingOf.push b pb (fun i => (D i).drop) i.1)
    rw [ha, hb, PendingOf.pushLoc_restrict sel a pa (fun i => (D i).take) i,
      PendingOf.pushLoc_restrict sel b pb (fun i => (D i).drop) i]

/-- Pushing empty leaves changes nothing. -/
theorem PendingOf.push_idle (sel : Sel L mem) {ℓ : L} : (sh : TickShape) → (p : PendingOf sh) →
    (D : Fin (mem ℓ) → HList (Kind.delta (mem := mem)) (shapeKinds ℓ sh)) → (i : Fin (mem ℓ)) →
    IdleEnv sel (D i) → sel ℓ i = false → PendingOf.push sh p D i = p
  | .sing _, p, D, i, h, hi => by
    show p ++ (D i).1 i = p
    rw [h.1 i hi, List.append_nil]
  | .stream _ _ _ _, p, D, i, h, hi => by
    show p ++ (D i).1 i = p
    rw [h.1 i hi, List.append_nil]
  | .ksing _ _ _ _, p, D, i, h, hi => by
    show p ++ (D i).1 i = p
    rw [h.1 i hi, List.append_nil]
  | .pair a b, (pa, pb), D, i, h, hi => by
    simp only [PendingOf.push]
    rw [PendingOf.push_idle sel a pa (fun i => (D i).take) i (IdleEnv.take sel (D i) h) hi,
      PendingOf.push_idle sel b pb (fun i => (D i).drop) i (IdleEnv.drop sel (D i) h) hi]

theorem PendingOf.takeZip_zero : (sh : TickShape) → (p : PendingOf sh) →
    PendingOf.takeZip sh p 0 = []
  | .sing _, _ => rfl
  | .stream _ _ _ _, _ => rfl
  | .ksing _ _ _ _, _ => rfl
  | .pair a b, (pa, pb) => by
    simp only [PendingOf.takeZip, PendingOf.takeZip_zero a pa, PendingOf.takeZip_zero b pb, List.zip_nil_left]

theorem PendingOf.dropN_zero : (sh : TickShape) → (p : PendingOf sh) →
    PendingOf.dropN sh p 0 = p
  | .sing _, _ => rfl
  | .stream _ _ _ _, _ => rfl
  | .ksing _ _ _ _, _ => rfl
  | .pair a b, (pa, pb) => by
    simp only [PendingOf.dropN, PendingOf.dropN_zero a pa, PendingOf.dropN_zero b pb]

theorem unzipShape_restrict (sel : Sel L mem) (ℓ : L) : (sh : TickShape) →
    (l : Fin (mem ℓ) → List (BoundedOf BS BStr BKS sh)) →
    envRestrict sel (unzipShape ℓ sh l) = unzipShapeLoc sel ℓ sh (fun i => l i.1)
  | .sing _, _ => rfl
  | .stream _ _ _ _, _ => rfl
  | .ksing _ _ _ _, _ => rfl
  | .pair a b, l => by
    show envRestrict sel ((unzipShape ℓ a fun i => (l i).map Prod.fst).append (unzipShape ℓ b fun i => (l i).map Prod.snd))
      = (unzipShapeLoc sel ℓ a fun i => (l i.1).map Prod.fst).append (unzipShapeLoc sel ℓ b fun i => (l i.1).map Prod.snd)
    rw [envRestrict_append, unzipShape_restrict sel ℓ a, unzipShape_restrict sel ℓ b]

theorem unzipShape_idle (sel : Sel L mem) (ℓ : L) : (sh : TickShape) →
    (l : Fin (mem ℓ) → List (BoundedOf BS BStr BKS sh)) → (∀ i, sel ℓ i = false → l i = []) →
    IdleEnv sel (unzipShape ℓ sh l)
  | .sing _, _, h => ⟨h, trivial⟩
  | .stream _ _ _ _, _, h => ⟨h, trivial⟩
  | .ksing _ _ _ _, _, h => ⟨h, trivial⟩
  | .pair a b, l, h =>
    IdleEnv.append sel _ _ (unzipShape_idle sel ℓ a _ (fun i hi => by rw [h i hi]; rfl))
      (unzipShape_idle sel ℓ b _ (fun i hi => by rw [h i hi]; rfl))

/-- The cells drained after any step (what `Node.Inv` demands of the stateful tick cells). -/
theorem zip_drop_min_empty {α β : Type} (a : List α) (b : List β) :
    a.drop (min a.length b.length) = [] ∨ b.drop (min a.length b.length) = [] := by
  rcases Nat.le_total a.length b.length with h | h
  · left; rw [Nat.min_eq_left h]; simp
  · right; rw [Nat.min_eq_right h]; simp

end Project

end Hydro
