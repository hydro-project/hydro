import Hydro.Project.Model.StepOps4

/-!
# Hydro · Project — per-operator step lemmas (the network cells)

A channel pair's cell holds the sender's undelivered suffix and the
delivered count; delivery at a step `t` at which the receiver ticks hands
over the cursor's prefix of the sender's *previous* view
(`StepHist.deliver`'s `+1` floor), then this step's sends are enqueued;
at a non-ticking step only the enqueue happens.
-/

namespace Hydro

namespace Project

variable {L : Type} {mem : L → Nat} (pacing : Pacing L mem)

/-- The delivered count before step `t`. -/
def prevDelivered {α : Type} (h : StepHist α) (p : Nat → Bool) (c : Nat → Nat) : Nat → Nat
  | 0 => 0
  | t + 1 => ((h.deliver p c).view t).length

/-- The cell before step `t`. -/
def prevChanCell {α : Type} (h : StepHist α) (p : Nat → Bool) (c : Nat → Nat) (t : Nat) : ChanCell α :=
  ⟨(prevView h t).drop (prevDelivered h p c t), prevDelivered h p c t⟩

theorem prevDelivered_eq {α : Type} (h : StepHist α) (p : Nat → Bool) (c : Nat → Nat) (t : Nat) :
    prevDelivered h p c t = (prevView (h.deliver p c) t).length := by
  cases t <;> rfl

/-- The delivered count never exceeds the previous view's length. -/
theorem prevDelivered_le_prevView {α : Type} (h : StepHist α) (p : Nat → Bool) (c : Nat → Nat) (t : Nat) :
    prevDelivered h p c t ≤ (prevView h t).length := by
  cases t with
  | zero => exact Nat.zero_le _
  | succ t => exact ((deliver_view_prefix_prev h p c t).trans (prevView_prefix h t)).length_le

/-- At a ticking step the previous delivered count is below the cursor's prefix. -/
theorem prevDelivered_le_take {α : Type} (h : StepHist α) (p : Nat → Bool) (c : Nat → Nat) (t : Nat)
    (hp : p t = true) : prevDelivered h p c t ≤ ((prevView h t).take (cumMax c t)).length := by
  rw [prevDelivered_eq, ← deliver_view_eq h p c t hp]
  exact (prevView_prefix (h.deliver p c) t).length_le

/-- The delivered increment at step `t`. -/
theorem incView_deliver {α : Type} (h : StepHist α) (p : Nat → Bool) (c : Nat → Nat) (t : Nat) :
    incView (h.deliver p c) t
      = if p t then ((prevView h t).take (cumMax c t)).drop (prevDelivered h p c t) else [] := by
  have hinc : incView (h.deliver p c) t = ((h.deliver p c).view t).drop (prevDelivered h p c t) := by
    rw [prevDelivered_eq, view_eq_prev_inc (h.deliver p c) t, List.drop_left]
  rw [hinc]
  cases hp : p t
  · rw [deliver_view_eq_prev h p c t hp, prevDelivered_eq, List.drop_length]; rfl
  · rw [deliver_view_eq h p c t hp]; rfl

/-- One step of a channel cell. -/
theorem chan_cell_step {α : Type} (h : StepHist α) (p : Nat → Bool) (c : Nat → Nat) (t : Nat) :
    (prevChanCell h p c t).stepG (p t) (cumMax c t) (incView h t) = (chanCellAt h p c t, incView (h.deliver p c) t) := by
  rw [incView_deliver]
  unfold ChanCell.stepG
  cases hp : p t
  · -- no tick: enqueue only
    simp only [Bool.false_eq_true, if_false]
    unfold ChanCell.enqueue prevChanCell chanCellAt
    rw [deliver_view_eq_prev h p c t hp, ← prevDelivered_eq, view_eq_prev_inc h t,
      List.drop_append_of_le_length (prevDelivered_le_prevView h p c t)]
  · simp only [if_true]
    have hD := prevDelivered_le_take h p c t hp
    have hT : ((prevView h t).take (cumMax c t)).length ≤ (prevView h t).length := by
      rw [List.length_take]; exact Nat.min_le_right _ _
    unfold ChanCell.step prevChanCell chanCellAt
    simp only
    have hout : ((prevView h t).drop (prevDelivered h p c t)).take (cumMax c t - prevDelivered h p c t)
        = ((prevView h t).take (cumMax c t)).drop (prevDelivered h p c t) := by
      rw [List.drop_take]
    rw [hout, deliver_view_eq h p c t hp]
    congr 1
    · rw [List.length_drop, List.drop_drop, Nat.add_sub_cancel' hD]
      congr 1
      rw [view_eq_prev_inc h t, List.drop_append_of_le_length hT]

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
  show _ = incView (mergeN (fun j => ((env.get x j).deliver (pacing.active p i) (d i j)).map (fun v => (j, v)))) t
  rw [incView_mergeN]
  congr 1
  funext j
  rw [incView_map]

theorem spec_demux {Γ : Ctx L mem} {c p : L} {α : Type} (iα : DecidableEq α) {ord : StrOrd}
    (d : Fin (mem p) → Fin (mem c) → Nat → Nat)
    (x : Var Γ (.stream c (Nat × α) inferInstance ord .exactlyOnce)) (addr : Fin (mem p) → Nat)
    (env : Env Γ) (t : Nat) : NodeSpec pacing (.demux (p := p) iα x addr) d env t := by
  have hp : Node.prevState pacing (.demux (p := p) iα x addr) d env t
      = fun i j => prevChanCell ((env.get x j).filterMap (fun dx => if dx.1 = addr i then some dx.2 else none))
          (pacing.active p i) (d i j) t := by
    cases t <;> rfl
  simp only [NodeSpec, Node.step, Node.denote, Node.stateOf, envIncAt_get, Kind.incAt, ctxAt]
  rw [hp]
  simp only [← incView_filterMap, chan_cell_step]
  congr 1
  congr 1
  funext i
  show _ = incView (mergeN (fun j => (((env.get x j).filterMap
    (fun dx => if dx.1 = addr i then some dx.2 else none)).deliver (pacing.active p i) (d i j)).map (fun v => (j, v)))) t
  rw [incView_mergeN]
  congr 1
  funext j
  rw [incView_map]

end Project

end Hydro
