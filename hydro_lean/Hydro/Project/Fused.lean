import Hydro.Project.Process
import Hydro.KeyedHash

/-!
# Hydro · Project — the activation in one walk (D92)

`Dag.iterateBytes` is the loop body the theorems are about: the inbox decoded
(`inboxOfWire`), appended to the queues (`enqueueInbox`), the cells materialized
(`memoState`), this activation's timing spread over the graph (`Timing.ofReadings`), the
delivery cursors read (`oracleOf`), the graph stepped (`locStep`), the sends encoded
(`wireEncode`) — seven structural walks of the ~230-node graph, each allocating a fresh
spine of pairs and recomputing every node's context `κs ++ Γ` (the context is data: a
`Kind` carries its location and grades). `perf` on the benchmark put ~45 % of a process's
cycles into these walks and their deallocation, independent of the load.

`Dag.activateFused` does the seven things to each node as it meets it, in ONE walk;
`Dag.activateFused_eq` says the result is the composition of the seven, node for node,
and `@[csimp] Dag.iterateBytes_eq_impl` is the equation the compiler substitutes: the
harnesses call `Dag.iterateBytes` — the very function `paxos_safe_wired` and `WireTrace`
are stated over — and run `Dag.iterateBytesImpl`. Nothing else is named between the
theorem and the binary. The proof is the same mutual induction as `Dag.locStep_eq_impl`,
every node-level walk decomposing at `letN` by definition and the identities
`memoState_eq`/`memoEnv_eq`. (A module that imports `Process` but not this file compiles
`iterateBytes` to its definition — slower, never different.)
-/

namespace Hydro

namespace Project

variable {L : Type} {mem : L → Nat} [DecidableEq L]

/-- A network node the process does not touch has no sender here: nothing to encode. -/
theorem hostedList_eq_nil_of_ne (m : Member L mem) (ℓ : L) (h : ¬ m.1 = ℓ) : hostedList m ℓ = [] := by
  unfold hostedList
  obtain ⟨ℓ', j⟩ := m
  have hne : ℓ' ≠ ℓ := h
  simp [hostedSelf, hne]

/-- A node the process does not touch keeps its cells through `enqueueInbox`: a network
node's queues live at the receiver, which the process does not host. -/
theorem Node.enqueueInbox_idle (m : Member L mem) {Γ : Ctx L mem} {κs : List (Kind L mem)}
    (n : Node Γ κs) (ib : Node.Inbox m n) (s : Node.LocState m n) (h : Node.touches m n = false) :
    Node.enqueueInbox m n ib s = s := by
  obtain ⟨hx, hκ⟩ := Node.touches_false m n h
  cases n <;> first
    | rfl
    | (exfalso; simp [Node.extraLoc] at hx; done)
    | exact funext fun i => (Hosted.elim (hκ _ List.mem_cons_self) i).elim

/-- A node the process does not touch encodes nothing. -/
theorem Node.wireEncode_idle (m : Member L mem) {Γ : Ctx L mem} {κs : List (Kind L mem)}
    (n : Node Γ κs) (cod : Node.Codecs n) (base : Nat) (snd : Node.Sends m n) (h : Node.touches m n = false) :
    (Node.wireEncode m n cod base snd).1 = [] := by
  obtain ⟨hx, _⟩ := Node.touches_false m n h
  cases n <;> first
    | rfl
    | (exfalso; simp [Node.extraLoc] at hx; done)
    | (show ((hostedList m _).flatMap _) = []
       rw [hostedList_eq_nil_of_ne m _ (by simpa [Node.extraLoc] using hx)]
       rfl)

/-! ### The inbox bucketed by wire node (D92)

`Node.inboxOfWire` at a network node scans every batch's messages for the node's number —
for every network node and every sender: on the leader ~10 scans of ~230 messages per
activation. `bucketOf batches k` is the batches with only node `k`'s messages — what
`decodeAt` reads anyway (`decodeAt_filter`) — and `bucketArray` computes every bucket in
one pass (the messages of a batch grouped by node through a hash map). The fused walk takes
the buckets as a function of the node number. -/

/-- The batches restricted to the messages of wire node `k`. -/
def bucketOf (batches : List ((_src : Member L mem) × List WireMsg)) (k : Nat) : List ((_src : Member L mem) × List WireMsg) :=
  batches.map fun b => ⟨b.1, b.2.filter fun m => decide (m.1 = k)⟩

/-- The messages of a batch at node `k`, through the batch's grouping by node. -/
def bucketMsgs (g : Std.HashMap Nat (List (Nat × ByteArray))) (k : Nat) : List WireMsg :=
  ((g[k]?).getD []).reverse.map fun v => (k, v)

theorem bucketMsgs_groupRev (msgs : List WireMsg) (k : Nat) :
    bucketMsgs (groupRev msgs) k = msgs.filter fun m => decide (m.1 = k) := by
  unfold bucketMsgs
  rw [groupRev_get, List.reverse_reverse]
  unfold keyVals
  rw [List.map_map]
  conv_rhs => rw [← List.map_id (msgs.filter fun m => decide (m.1 = k))]
  apply List.map_congr_left
  intro m hm
  have hk : m.1 = k := of_decide_eq_true (List.mem_filter.mp hm).2
  simp only [Function.comp_apply, id]
  rw [← hk]

/-- Every bucket in one pass over the batches (`n` buckets, the wire nodes' numbers). -/
def bucketArray (n : Nat) (batches : List ((_src : Member L mem) × List WireMsg)) :
    Array (List ((_src : Member L mem) × List WireMsg)) :=
  batches.foldr (fun b acc =>
    let g := groupRev b.2
    (List.range n).foldl (fun acc k => acc.modify k fun l => ⟨b.1, bucketMsgs g k⟩ :: l) acc)
    (Array.replicate n [])

/-- Modifying every index of a duplicate-free list: index `k` is modified once iff `k` is
in the list. -/
theorem foldl_modify_get {α : Type} (f : Nat → α → α) : ∀ (l : List Nat) (acc : Array α) (k : Nat), l.Nodup →
    (l.foldl (fun acc k' => acc.modify k' (f k')) acc)[k]? = if k ∈ l then (acc[k]?).map (f k) else acc[k]?
  | [], acc, k, _ => by simp
  | k' :: l, acc, k, hnd => by
    rw [List.foldl_cons, foldl_modify_get f l _ k (List.nodup_cons.mp hnd).2, Array.getElem?_modify]
    by_cases hkk : k' = k
    · subst hkk
      have hnot : k' ∉ l := (List.nodup_cons.mp hnd).1
      simp [hnot]
    · have hkk' : ¬ k = k' := fun h => hkk h.symm
      simp [hkk, hkk', List.mem_cons]

theorem size_foldl_modify {α : Type} (f : Nat → α → α) (l : List Nat) (acc : Array α) :
    (l.foldl (fun acc k' => acc.modify k' (f k')) acc).size = acc.size := by
  induction l generalizing acc with
  | nil => rfl
  | cons k' l ih => rw [List.foldl_cons, ih, Array.size_modify]

omit [DecidableEq L] in
theorem size_bucketArray (n : Nat) : ∀ (batches : List ((_src : Member L mem) × List WireMsg)),
    (bucketArray n batches).size = n
  | [] => by simp [bucketArray]
  | b :: batches => by
    unfold bucketArray
    rw [List.foldr_cons, size_foldl_modify]
    exact size_bucketArray n batches

omit [DecidableEq L] in
theorem bucketArray_get (n : Nat) (k : Nat) (hk : k < n) : ∀ (batches : List ((_src : Member L mem) × List WireMsg)),
    (bucketArray n batches)[k]? = some (bucketOf batches k)
  | [] => by
    simp [bucketArray, bucketOf, hk]
  | b :: batches => by
    have ih := bucketArray_get n k hk batches
    unfold bucketArray at ih ⊢
    rw [List.foldr_cons]
    rw [foldl_modify_get _ (List.range n) _ k (List.nodup_range), if_pos (List.mem_range.mpr hk), ih]
    simp only [Option.map_some, bucketOf, List.map_cons, bucketMsgs_groupRev]

/-- One node of the activation, everything at once: its inbox decoded and enqueued, its
cells materialized, its timing and cursors read, its step, its sends encoded. A node the
process does not touch is skipped whole (cells kept, empty increments, no messages —
`Node.locStep_idle`, `enqueueInbox_idle`, `wireEncode_idle`). For a callee or a knot the
fused graph walk does this node by node (`Node.activateFused`). -/
def Node.activatePlain (m : Member L mem) (tk : Bool) {Γ : Ctx L mem} {κs : List (Kind L mem)}
    (n : Node Γ κs) (cod : Node.Codecs n) (base : Nat) (buckets : Nat → List ((_src : Member L mem) × List WireMsg))
    (tm : Node.Timing m n) (Δ : HList (Kind.locDelta m) Γ) (s : Node.LocState m n) :
    (Node.LocState m n × HList (Kind.locDelta m) κs × List WireMsg) × Nat :=
  if h : Node.touches m n = false then
    ((s, HList.idle m _ (Node.touches_false m n h).2, []), (Node.wireReceivers n base).2)
  else
    let ib := Node.inboxOfWire m n cod base (buckets base)
    let s1 := Node.memoState m n (Node.enqueueInbox m n ib.1 s)
    let ora := Node.oracleOf m n tm s1
    let st := Node.locStep m tk n ora Δ s1
    let msgs := Node.wireEncode m n cod base st.2.2
    ((Node.memoState m n st.1, memoEnv m st.2.1, msgs.1), msgs.2)

mutual

/-- One node of the activation: callees and knots walked by the fused graph walk, every
other node `Node.activatePlain`. Returns the cells, the increments and the messages, and
the next wire number. -/
def Node.activateFused (m : Member L mem) (tk : Bool) :
    {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) → Node.Codecs n → Nat →
    (Nat → List ((_src : Member L mem) × List WireMsg)) → Node.Timing m n → HList (Kind.locDelta m) Γ →
    Node.LocState m n → (Node.LocState m n × HList (Kind.locDelta m) κs × List WireMsg) × Nat
  | _, _, .call N args, cod, base, batches, tm, Δ, s =>
    Dag.activateFused m tk N cod base batches tm (args.map (fun v => Δ.get v)) s
  | _, _, .fixN _ body, cod, base, batches, tm, Δ, s =>
    let q := Dag.activateFused m tk body cod base batches tm (s.2.append Δ) s.1
    (((q.1.1, q.1.2.1.take), s.2.append q.1.2.1.drop, q.1.2.2), q.2)
  | _, _, n, cod, base, batches, tm, Δ, s => Node.activatePlain m tk n cod base batches tm Δ s

/-- The activation of a graph in one walk. -/
def Dag.activateFused (m : Member L mem) (tk : Bool) :
    {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) → Dag.Codecs d → Nat →
    (Nat → List ((_src : Member L mem) × List WireMsg)) → Dag.Timing m d → HList (Kind.locDelta m) Γ →
    Dag.LocState m d → (Dag.LocState m d × HList (Kind.locDelta m) outs × List WireMsg) × Nat
  | _, _, .ret xs, _, base, _, _, Δ, s => ((s, xs.map (fun v => Δ.get v), []), base)
  | _, _, .letN n rest, cod, base, batches, tm, Δ, s =>
    let q := Node.activateFused m tk n cod.1 base batches tm.1 Δ s.1
    let q' := Dag.activateFused m tk rest cod.2 q.2 batches tm.2 (q.1.2.1.append Δ) s.2
    (((q.1.1, q'.1.1), q'.1.2.1, q.1.2.2 ++ q'.1.2.2), q'.2)

end

/-- What the seven walks compute for one node. -/
def Node.activateSpec (m : Member L mem) (tk : Bool) {Γ : Ctx L mem} {κs : List (Kind L mem)}
    (n : Node Γ κs) (cod : Node.Codecs n) (base : Nat) (batches : List ((_src : Member L mem) × List WireMsg))
    (tm : Node.Timing m n) (Δ : HList (Kind.locDelta m) Γ) (s : Node.LocState m n) :
    Node.LocState m n × HList (Kind.locDelta m) κs × List WireMsg :=
  let ib := Node.inboxOfWire m n cod base batches
  let s1 := Node.memoState m n (Node.enqueueInbox m n ib.1 s)
  let st := Node.locStep m tk n (Node.oracleOf m n tm s1) Δ s1
  (Node.memoState m n st.1, memoEnv m st.2.1, (Node.wireEncode m n cod base st.2.2).1)

/-- What the seven walks compute for a graph. -/
def Dag.activateSpec (m : Member L mem) (tk : Bool) {Γ : Ctx L mem} {outs : List (Kind L mem)}
    (d : Dag Γ outs) (cod : Dag.Codecs d) (base : Nat) (batches : List ((_src : Member L mem) × List WireMsg))
    (tm : Dag.Timing m d) (Δ : HList (Kind.locDelta m) Γ) (s : Dag.LocState m d) :
    Dag.LocState m d × HList (Kind.locDelta m) outs × List WireMsg :=
  let s' := Dag.memoState m d (Dag.enqueueInbox m d (Dag.inboxOfWire m d cod base batches).1 s)
  let st := Dag.locStep m tk d (Dag.oracleOf m d tm s') Δ s'
  (st.1, st.2.1, (Dag.wireEncode m d cod base st.2.2).1)

/-- A callee or a knot (walked by the fused graph walk, not `Node.activatePlain`). -/
def Node.isCallOrKnot {Γ : Ctx L mem} {κs : List (Kind L mem)} : Node Γ κs → Bool
  | .call .. => true
  | .fixN .. => true
  | _ => false

/-- A node reads the same inbox from its own bucket (`decodeAt_filter`). -/
theorem Node.inboxOfWire_bucketOf (m : Member L mem) {Γ : Ctx L mem} {κs : List (Kind L mem)} (n : Node Γ κs)
    (cod : Node.Codecs n) (base : Nat) (batches : List ((_src : Member L mem) × List WireMsg))
    (hp : Node.isCallOrKnot n = false) :
    Node.inboxOfWire m n cod base (bucketOf batches base) = Node.inboxOfWire m n cod base batches := by
  cases n with
  | call N args => cases hp
  | fixN ks body => cases hp
  | broadcast_closed =>
    unfold Node.inboxOfWire bucketOf
    simp only [List.flatMap_map]
    congr 1
    funext i j
    apply List.flatMap_congr
    intro b _
    have h := decodeAt_filter cod b.2 base j.val
    rw [h]
  | demux =>
    unfold Node.inboxOfWire bucketOf
    simp only [List.flatMap_map]
    congr 1
    funext i j
    apply List.flatMap_congr
    intro b _
    have h := decodeAt_filter cod b.2 base j.val
    rw [h]
  | _ => rfl

/-- The plain node's fused activation is what the seven walks compute for it. -/
theorem Node.activatePlain_eq (m : Member L mem) (tk : Bool) {Γ : Ctx L mem} {κs : List (Kind L mem)}
    (n : Node Γ κs) (cod : Node.Codecs n) (base : Nat) (batches : List ((_src : Member L mem) × List WireMsg))
    (buckets : Nat → List ((_src : Member L mem) × List WireMsg)) (hb : ∀ k, buckets k = bucketOf batches k)
    (tm : Node.Timing m n) (Δ : HList (Kind.locDelta m) Γ) (s : Node.LocState m n) (hp : Node.isCallOrKnot n = false) :
    Node.activatePlain m tk n cod base buckets tm Δ s
      = (Node.activateSpec m tk n cod base batches tm Δ s, (Node.wireReceivers n base).2) := by
  unfold Node.activatePlain
  rw [hb, Node.inboxOfWire_bucketOf m n cod base batches hp]
  split
  · rename_i h
    unfold Node.activateSpec
    simp only [Node.memoState_eq, Node.enqueueInbox_idle m n _ s h, Node.locStep_idle m tk n _ Δ s h,
      memoEnv_eq, Node.wireEncode_idle m n cod base _ h]
  · unfold Node.activateSpec
    simp only [Node.wireEncode_snd_eq]

mutual

theorem Node.activateFused_eq (m : Member L mem) (tk : Bool)
    (batches : List ((_src : Member L mem) × List WireMsg))
    (buckets : Nat → List ((_src : Member L mem) × List WireMsg)) (hb : ∀ k, buckets k = bucketOf batches k) :
    {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) → (cod : Node.Codecs n) → (base : Nat) →
    (tm : Node.Timing m n) → (Δ : HList (Kind.locDelta m) Γ) → (s : Node.LocState m n) →
    Node.activateFused m tk n cod base buckets tm Δ s
      = (Node.activateSpec m tk n cod base batches tm Δ s, (Node.wireReceivers n base).2)
  | _, _, .call N args, cod, base, tm, Δ, s => by
    have h := Dag.activateFused_eq m tk batches buckets hb N cod base tm (args.map (fun v => Δ.get v)) s
    show Dag.activateFused m tk N cod base buckets tm (args.map (fun v => Δ.get v)) s = _
    rw [h]
    unfold Node.activateSpec Dag.activateSpec
    -- the materializations are identities (rewritten while the terms are still typed at the
    -- node); what remains is the node-level walks at a callee, definitionally the callee's
    simp only [Node.memoState_eq, memoEnv_eq, Dag.memoState_eq]
    all_goals rfl
  | _, _, .fixN ks body, cod, base, tm, Δ, s => by
    have h := Dag.activateFused_eq m tk batches buckets hb body cod base tm (s.2.append Δ) s.1
    show (let q := Dag.activateFused m tk body cod base buckets tm (s.2.append Δ) s.1
      (((q.1.1, q.1.2.1.take), s.2.append q.1.2.1.drop, q.1.2.2), q.2)) = _
    rw [h]
    unfold Node.activateSpec Dag.activateSpec
    simp only [Node.memoState_eq, memoEnv_eq, Dag.memoState_eq]
    all_goals rfl
  | _, _, .map .., cod, base, tm, Δ, s => Node.activatePlain_eq m tk _ cod base batches buckets hb tm Δ s rfl
  | _, _, .filterMap .., cod, base, tm, Δ, s => Node.activatePlain_eq m tk _ cod base batches buckets hb tm Δ s rfl
  | _, _, .broadcast_closed .., cod, base, tm, Δ, s => Node.activatePlain_eq m tk _ cod base batches buckets hb tm Δ s rfl
  | _, _, .demux .., cod, base, tm, Δ, s => Node.activatePlain_eq m tk _ cod base batches buckets hb tm Δ s rfl
  | _, _, .values .., cod, base, tm, Δ, s => Node.activatePlain_eq m tk _ cod base batches buckets hb tm Δ s rfl
  | _, _, .into_keyed .., cod, base, tm, Δ, s => Node.activatePlain_eq m tk _ cod base batches buckets hb tm Δ s rfl
  | _, _, .kassume_ordering .., cod, base, tm, Δ, s => Node.activatePlain_eq m tk _ cod base batches buckets hb tm Δ s rfl
  | _, _, .kfold_early_stop .., cod, base, tm, Δ, s => Node.activatePlain_eq m tk _ cod base batches buckets hb tm Δ s rfl
  | _, _, .kget_max_key .., cod, base, tm, Δ, s => Node.activatePlain_eq m tk _ cod base batches buckets hb tm Δ s rfl
  | _, _, .weaken_retries .., cod, base, tm, Δ, s => Node.activatePlain_eq m tk _ cod base batches buckets hb tm Δ s rfl
  | _, _, .union .., cod, base, tm, Δ, s => Node.activatePlain_eq m tk _ cod base batches buckets hb tm Δ s rfl
  | _, _, .assume_ordering .., cod, base, tm, Δ, s => Node.activatePlain_eq m tk _ cod base batches buckets hb tm Δ s rfl
  | _, _, .fold .., cod, base, tm, Δ, s => Node.activatePlain_eq m tk _ cod base batches buckets hb tm Δ s rfl
  | _, _, .fold_monotone .., cod, base, tm, Δ, s => Node.activatePlain_eq m tk _ cod base batches buckets hb tm Δ s rfl
  | _, _, .snapshot .., cod, base, tm, Δ, s => Node.activatePlain_eq m tk _ cod base batches buckets hb tm Δ s rfl
  | _, _, .batch .., cod, base, tm, Δ, s => Node.activatePlain_eq m tk _ cod base batches buckets hb tm Δ s rfl
  | _, _, .source_interval .., cod, base, tm, Δ, s => Node.activatePlain_eq m tk _ cod base batches buckets hb tm Δ s rfl
  | _, _, .mapTick .., cod, base, tm, Δ, s => Node.activatePlain_eq m tk _ cod base batches buckets hb tm Δ s rfl
  | _, _, .firstTick .., cod, base, tm, Δ, s => Node.activatePlain_eq m tk _ cod base batches buckets hb tm Δ s rfl
  | _, _, .zipTick .., cod, base, tm, Δ, s => Node.activatePlain_eq m tk _ cod base batches buckets hb tm Δ s rfl
  | _, _, .defer_tick .., cod, base, tm, Δ, s => Node.activatePlain_eq m tk _ cod base batches buckets hb tm Δ s rfl
  | _, _, .allTicks .., cod, base, tm, Δ, s => Node.activatePlain_eq m tk _ cod base batches buckets hb tm Δ s rfl
  | _, _, .flattenOrdered .., cod, base, tm, Δ, s => Node.activatePlain_eq m tk _ cod base batches buckets hb tm Δ s rfl
  | _, _, .flattenUnordered .., cod, base, tm, Δ, s => Node.activatePlain_eq m tk _ cod base batches buckets hb tm Δ s rfl
  | _, _, .tinto_keyed .., cod, base, tm, Δ, s => Node.activatePlain_eq m tk _ cod base batches buckets hb tm Δ s rfl
  | _, _, .tassume_ordering .., cod, base, tm, Δ, s => Node.activatePlain_eq m tk _ cod base batches buckets hb tm Δ s rfl
  | _, _, .kentries .., cod, base, tm, Δ, s => Node.activatePlain_eq m tk _ cod base batches buckets hb tm Δ s rfl
  | _, _, .kmap .., cod, base, tm, Δ, s => Node.activatePlain_eq m tk _ cod base batches buckets hb tm Δ s rfl
  | _, _, .kallTicks .., cod, base, tm, Δ, s => Node.activatePlain_eq m tk _ cod base batches buckets hb tm Δ s rfl
  | _, _, .kfold .., cod, base, tm, Δ, s => Node.activatePlain_eq m tk _ cod base batches buckets hb tm Δ s rfl
  | _, _, .ksnapshot .., cod, base, tm, Δ, s => Node.activatePlain_eq m tk _ cod base batches buckets hb tm Δ s rfl
  | _, _, .tfold .., cod, base, tm, Δ, s => Node.activatePlain_eq m tk _ cod base batches buckets hb tm Δ s rfl
  | _, _, .latest .., cod, base, tm, Δ, s => Node.activatePlain_eq m tk _ cod base batches buckets hb tm Δ s rfl
  | _, _, .latest_monotone .., cod, base, tm, Δ, s => Node.activatePlain_eq m tk _ cod base batches buckets hb tm Δ s rfl
  | _, _, .time .., cod, base, tm, Δ, s => Node.activatePlain_eq m tk _ cod base batches buckets hb tm Δ s rfl
  | _, _, .snapshot_bounded .., cod, base, tm, Δ, s => Node.activatePlain_eq m tk _ cod base batches buckets hb tm Δ s rfl
  | _, _, .reduce_watermark .., cod, base, tm, Δ, s => Node.activatePlain_eq m tk _ cod base batches buckets hb tm Δ s rfl
  | _, _, .tick_scan .., cod, base, tm, Δ, s => Node.activatePlain_eq m tk _ cod base batches buckets hb tm Δ s rfl

theorem Dag.activateFused_eq (m : Member L mem) (tk : Bool)
    (batches : List ((_src : Member L mem) × List WireMsg))
    (buckets : Nat → List ((_src : Member L mem) × List WireMsg)) (hb : ∀ k, buckets k = bucketOf batches k) :
    {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) → (cod : Dag.Codecs d) → (base : Nat) →
    (tm : Dag.Timing m d) → (Δ : HList (Kind.locDelta m) Γ) → (s : Dag.LocState m d) →
    Dag.activateFused m tk d cod base buckets tm Δ s
      = (Dag.activateSpec m tk d cod base batches tm Δ s, (Dag.wireReceivers d base).2)
  | _, _, .ret xs, _, base, _, Δ, s => by
    have hs : s = () := rfl
    subst hs
    rfl
  | _, _, .letN n rest, cod, base, tm, Δ, s => by
    have h1 := Node.activateFused_eq m tk batches buckets hb n cod.1 base tm.1 Δ s.1
    have h2 := Dag.activateFused_eq m tk batches buckets hb rest cod.2 (Node.wireReceivers n base).2 tm.2
      ((Node.activateSpec m tk n cod.1 base batches tm.1 Δ s.1).2.1.append Δ) s.2
    show (let q := Node.activateFused m tk n cod.1 base buckets tm.1 Δ s.1
      let q' := Dag.activateFused m tk rest cod.2 q.2 buckets tm.2 (q.1.2.1.append Δ) s.2
      (((q.1.1, q'.1.1), q'.1.2.1, q.1.2.2 ++ q'.1.2.2), q'.2)) = _
    simp only [h1, h2]
    unfold Dag.activateSpec Node.activateSpec
    -- the graph-level walks at `letN` are the node's and the rest's; the wire numbering is
    -- `wireReceivers`' either way
    simp only [Dag.inboxOfWire, Dag.enqueueInbox, Dag.memoState, Dag.oracleOf, Dag.locStep,
      Dag.wireEncode, Dag.wireReceivers, Node.inboxOfWire_snd_eq, Node.wireEncode_snd_eq, Node.memoState_eq,
      memoEnv_eq, Dag.memoState_eq]
    all_goals rfl

end

/-- The settling passes of an activation, compiled (D93): while `Dag.settleFlag` names one
and the counter remains, one fused walk with that tick flag and nothing arriving, its outputs
merged into the activation's and its messages appended to the activation's. -/
def Dag.settleImpl (m : Member L mem) {Γ : Ctx L mem} {outs : List (Kind L mem)} (d : Dag Γ outs)
    (cod : Dag.Codecs d) (tm : Dag.Timing m d) (Δ : HList (Kind.locDelta m) Γ) :
    Nat → Dag.LocState m d → HList (Kind.locDelta m) outs → List WireMsg →
    Dag.LocState m d × HList (Kind.locDelta m) outs × List WireMsg
  | 0, s, o, w => (s, o, w)
  | n + 1, s, o, w =>
    if (Dag.settleFlag m d s).isSome then
      let q := Dag.activateFused m (Dag.flagOf m d s) d cod 0 (fun _ => []) tm Δ s
      Dag.settleImpl m d cod tm Δ n q.1.1 (envMerge m o q.1.2.1) (w ++ q.1.2.2)
    else (s, o, w)

/-- **The compiled `Dag.iterateBytes`**: the frames parsed once per connection, the messages
bucketed by wire node, the graph walked once for the first pass and once per settling pass.
`Dag.iterateBytes` (Process.lean) is the function the theorems are about and the one the
harnesses write; `@[csimp] Dag.iterateBytes_eq_impl` is the kernel-checked equation the
compiler substitutes this for. -/
def Dag.iterateBytesImpl (m : Member L mem) (tk : Bool) {Γ : Ctx L mem} {outs : List (Kind L mem)} (d : Dag Γ outs)
    (cod : Dag.Codecs d) (tm : Dag.Timing m d) (chans : List ((_src : Member L mem) × ByteArray))
    (Δ : HList (Kind.locDelta m) Γ) (s : Dag.LocState m d) :
    Dag.LocState m d × HList (Kind.locDelta m) outs × List WireMsg × List ByteArray :=
  let parsed := chans.map fun c => (⟨c.1, parseFrames c.2⟩ : (_src : Member L mem) × List WireMsg × ByteArray)
  let batches := parsed.map fun c => (⟨c.1, c.2.1⟩ : (_src : Member L mem) × List WireMsg)
  let arr := bucketArray (Dag.wireCount d 0) batches
  let q := Dag.activateFused m tk d cod 0 (fun k => (arr[k]?).getD (bucketOf batches k)) tm Δ s
  let r := Dag.settleImpl m d cod (Dag.Timing.quiet m d tm) (envQuiet m Δ) (Dag.settleFuel d) q.1.1 q.1.2.1 q.1.2.2
  (r.1, r.2.1, r.2.2, parsed.map fun c => c.2.2)

/-- A settling pass's fused walk is the pass (`Dag.pass` on the empty inbox), its messages
encoded. -/
theorem Dag.activateFused_settle (m : Member L mem) (f : Bool) {Γ : Ctx L mem} {outs : List (Kind L mem)} (d : Dag Γ outs)
    (cod : Dag.Codecs d) (tm : Dag.Timing m d) (Δ : HList (Kind.locDelta m) Γ) (s : Dag.LocState m d) :
    (Dag.activateFused m f d cod 0 (fun _ => []) tm Δ s).1
      = (let r := Dag.pass m f d tm (Dag.Inbox.empty m d) Δ s
         (r.1, r.2.1, (Dag.wireEncode m d cod 0 r.2.2).1)) := by
  rw [Dag.activateFused_eq m f [] (fun _ => []) (fun _ => rfl)]
  simp only [Dag.activateSpec, Dag.pass, Dag.inboxOfWire_nil]

/-- The compiled settling is `Dag.settle`, folded. -/
theorem Dag.settleImpl_eq (m : Member L mem) {Γ : Ctx L mem} {outs : List (Kind L mem)} (d : Dag Γ outs)
    (cod : Dag.Codecs d) (tm : Dag.Timing m d) (Δ : HList (Kind.locDelta m) Γ) :
    ∀ (n : Nat) (s : Dag.LocState m d) (o : HList (Kind.locDelta m) outs) (w : List WireMsg),
      Dag.settleImpl m d cod tm Δ n s o w
        = (let st := Dag.settle m d tm Δ n s
           (st.foldl (fun _ q => q.1) s, st.foldl (fun o q => envMerge m o q.2.1) o,
            w ++ (st.map (·.2.2)).flatMap fun snd => (Dag.wireEncode m d cod 0 snd).1))
  | 0, s, o, w => by simp [Dag.settleImpl, Dag.settle]
  | n + 1, s, o, w => by
    unfold Dag.settleImpl Dag.settle
    split
    · simp only [Dag.activateFused_settle, Dag.settleImpl_eq m d cod tm Δ n, List.foldl_cons, List.map_cons,
        List.flatMap_cons, List.append_assoc]
    · simp

omit [DecidableEq L] in
theorem bucketArray_getD (n : Nat) (batches : List ((_src : Member L mem) × List WireMsg)) (k : Nat) :
    ((bucketArray n batches)[k]?).getD (bucketOf batches k) = bucketOf batches k := by
  by_cases hk : k < n
  · rw [bucketArray_get n k hk batches]; rfl
  · have hsz : (bucketArray n batches).size ≤ k := by
      rw [size_bucketArray]; omega
    rw [Array.getElem?_eq_none hsz]; rfl

@[csimp] theorem Dag.iterateBytes_eq_impl : @Dag.iterateBytes = @Dag.iterateBytesImpl := by
  funext L mem _ m tk Γ outs d cod tm chans Δ s
  simp only [Dag.iterateBytes, Dag.iterateBytesImpl, Dag.iterateWire, Dag.iterate,
    List.map_map, Function.comp_def]
  rw [Dag.activateFused_eq m tk _ _ (fun k => bucketArray_getD _ _ k), Dag.settleImpl_eq]
  simp only [Dag.activateSpec, Dag.pass, List.flatMap_cons]

end Project

end Hydro
