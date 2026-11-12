import Hydro.Project.Model.StepOps4

/-!
# Hydro · Project — per-operator step lemmas (the network cells)

`NodeSpec` for `broadcast_closed` and `demux`. A channel pair's cell (`ChanCell`) holds the
sender's undelivered suffix and the delivered count. At a step `t` where the receiver is active
the cell hands over the cursor's prefix of the sender's view before `t` — `StepHist.deliver`
reads the sender one step back — and then enqueues this step's sends; at an inactive step only
the enqueue happens. `chan_cell_step` is this one-step law for a single pair; the two node
lemmas apply it pairwise and merge the delivered increments with `incView_mergeN`.
-/

namespace Hydro

namespace Project

variable {L : Type} {mem : L → Nat} (pacing : Pacing L mem)

/-- The delivered count before step `t`: `0` before step `0`, the delivered view's length at
`t-1` otherwise. -/
def prevDelivered {α : Type} (h : StepHist α) (p : Nat → Bool) (c : Nat → Nat) : Nat → Nat
  | 0 => 0
  | t + 1 => ((h.deliver p c).view t).length

/-- The channel cell before step `t`: the sender's view before `t` beyond the delivered count,
and the delivered count. -/
def prevChanCell {α : Type} (h : StepHist α) (p : Nat → Bool) (c : Nat → Nat) (t : Nat) : ChanCell α :=
  ⟨((prevView h t).drop (prevDelivered h p c t)).toArray, prevDelivered h p c t⟩

/-- `prevDelivered` is the length of the delivered view before `t`. -/
theorem prevDelivered_eq {α : Type} (h : StepHist α) (p : Nat → Bool) (c : Nat → Nat) (t : Nat) :
    prevDelivered h p c t = (prevView (h.deliver p c) t).length := by
  cases t <;> rfl

/-- The delivered count before `t` is within the sender's view before `t`. -/
theorem prevDelivered_le_prevView {α : Type} (h : StepHist α) (p : Nat → Bool) (c : Nat → Nat) (t : Nat) :
    prevDelivered h p c t ≤ (prevView h t).length := by
  cases t with
  | zero => exact Nat.zero_le _
  | succ t => exact ((deliver_view_prefix_prev h p c t).trans (prevView_prefix h t)).length_le

/-- At an active step (`hp`) the delivered count before `t` is within the cursor's prefix of the
sender's view before `t`: the cursor is a `cumMax`, so it never moves back. -/
theorem prevDelivered_le_take {α : Type} (h : StepHist α) (p : Nat → Bool) (c : Nat → Nat) (t : Nat)
    (hp : p t = true) : prevDelivered h p c t ≤ ((prevView h t).take (cumMax c t)).length := by
  rw [prevDelivered_eq, ← deliver_view_eq h p c t hp]
  exact (prevView_prefix (h.deliver p c) t).length_le

/-- The delivered increment at step `t`: the cursor's prefix of the sender's view before `t`,
beyond what was already delivered, when the receiver is active; nothing otherwise. -/
theorem incView_deliver {α : Type} (h : StepHist α) (p : Nat → Bool) (c : Nat → Nat) (t : Nat) :
    incView (h.deliver p c) t
      = if p t then ((prevView h t).take (cumMax c t)).drop (prevDelivered h p c t) else [] := by
  have hinc : incView (h.deliver p c) t = ((h.deliver p c).view t).drop (prevDelivered h p c t) := by
    rw [prevDelivered_eq, view_eq_prev_inc (h.deliver p c) t, List.drop_left]
  rw [hinc]
  cases hp : p t
  · rw [deliver_view_eq_prev h p c t hp, prevDelivered_eq, List.drop_length]; rfl
  · rw [deliver_view_eq h p c t hp]; rfl

/-- One step of a channel cell: `ChanCell.stepG` from the cell before `t`, releasing to the
cursor if the receiver is active and enqueueing the sender's increment, reaches `chanCellAt`
at `t` and emits `incView_deliver`. -/
theorem chan_cell_step {α : Type} (h : StepHist α) (p : Nat → Bool) (c : Nat → Nat) (t : Nat) :
    (prevChanCell h p c t).stepG (p t) (cumMax c t) (incView h t).toArray
      = (chanCellAt h p c t, (incView (h.deliver p c) t).toArray) := by
  rw [incView_deliver]
  unfold ChanCell.stepG
  cases hp : p t
  · -- inactive: enqueue only
    simp only [Bool.false_eq_true, if_false]
    rw [ChanCell.enqueue_eq]
    unfold prevChanCell chanCellAt
    simp only [List.toList_toArray]
    rw [deliver_view_eq_prev h p c t hp, ← prevDelivered_eq, view_eq_prev_inc h t,
      List.drop_append_of_le_length (prevDelivered_le_prevView h p c t)]
  · simp only [if_true]
    have hD := prevDelivered_le_take h p c t hp
    have hT : ((prevView h t).take (cumMax c t)).length ≤ (prevView h t).length := by
      rw [List.length_take]; exact Nat.min_le_right _ _
    rw [ChanCell.step_eq]
    unfold prevChanCell chanCellAt
    simp only [List.toList_toArray]
    have hout : ((prevView h t).drop (prevDelivered h p c t)).take (cumMax c t - prevDelivered h p c t)
        = ((prevView h t).take (cumMax c t)).drop (prevDelivered h p c t) := by
      rw [List.drop_take]
    rw [hout, deliver_view_eq h p c t hp]
    congr 1
    · rw [List.length_drop, List.drop_drop, Nat.add_sub_cancel' hD]
      congr 1
      rw [view_eq_prev_inc h t, List.drop_append_of_le_length hT]

/-- `broadcast_closed` meets `NodeSpec`: `chan_cell_step` per receiver/sender pair, the
delivered increments tagged and merged by `incView_mergeN`. -/
theorem spec_broadcast_closed {Γ : Ctx L mem} {c p : L} {α : Type} (iα : DecidableEq α) {ord : StrOrd}
    {ret : Retries} (d : Fin (mem p) → Fin (mem c) → Nat → Nat)
    (x : Var Γ (.stream c α iα ord ret)) (env : Env Γ) (t : Nat) :
    NodeSpec pacing (.broadcast_closed (p := p) iα x) d env t := by
  have hp : Node.prevState pacing (.broadcast_closed (p := p) iα x) d env t
      = fun i j => prevChanCell (env.get x j) (pacing.active p i) (d i j) t := by
    cases t <;> rfl
  simp only [NodeSpec, Node.step, Node.denote, Node.stateOf, envIncAt_get, Kind.incAt, ctxAt]
  rw [hp]
  simp only [chan_cell_step]
  congr 1
  congr 1
  funext i
  show _ = (incView (mergeN (fun j => ((env.get x j).deliver (pacing.active p i) (d i j)).map (fun v => (j, v)))) t).toArray
  apply Array.toList_inj.mp
  rw [List.toList_toArray, tagConcat_toList, incView_mergeN]
  congr 1
  funext j
  rw [incView_map, List.toList_toArray]

/-- `demux` meets `NodeSpec`: `spec_broadcast_closed` with each pair's sender view filtered to
the receiver's address (`incView_filterMap`). -/
theorem spec_demux {Γ : Ctx L mem} {c p : L} {α : Type} (iα : DecidableEq α) {ord : StrOrd}
    (d : Fin (mem p) → Fin (mem c) → Nat → Nat)
    (x : Var Γ (.stream c (Nat × α) inferInstance ord .exactlyOnce)) (addr : Fin (mem p) → Nat)
    (env : Env Γ) (t : Nat) : NodeSpec pacing (.demux (p := p) iα x addr) d env t := by
  have hp : Node.prevState pacing (.demux (p := p) iα x addr) d env t
      = fun i j => prevChanCell ((env.get x j).filterMap (fun dx => if dx.1 = addr i then some dx.2 else none))
          (pacing.active p i) (d i j) t := by
    cases t <;> rfl
  have hf : ∀ i j, filterMapB (fun dx => if dx.1 = addr i then some dx.2 else none) ((envIncAt env t).get x j)
      = (incView ((env.get x j).filterMap (fun dx => if dx.1 = addr i then some dx.2 else none)) t).toArray := by
    intro i j
    rw [envIncAt_get]
    show filterMapB _ (incView (env.get x j) t).toArray = _
    rw [filterMapB_toArray, incView_filterMap]
  simp only [NodeSpec, Node.step, Node.denote, Node.stateOf, ctxAt]
  rw [hp]
  simp only [hf, chan_cell_step]
  congr 1
  congr 1
  funext i
  show _ = (incView (mergeN (fun j => (((env.get x j).filterMap
    (fun dx => if dx.1 = addr i then some dx.2 else none)).deliver (pacing.active p i) (d i j)).map (fun v => (j, v)))) t).toArray
  apply Array.toList_inj.mp
  rw [List.toList_toArray, tagConcat_toList, incView_mergeN]
  congr 1
  funext j
  rw [incView_map, List.toList_toArray]

end Project

end Hydro
