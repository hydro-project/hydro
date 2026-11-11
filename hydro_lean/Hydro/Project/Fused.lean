import Hydro.Project.Process
import Hydro.KeyedHash

/-!
# Hydro · Project — the activation in one walk

`Dag.iterateBytes` is the loop body the theorems are about: the inbox decoded
(`inboxOfWire`), appended to the queues (`enqueueInbox`), the cells materialized
(`memoState`), the graph swept (`Dag.sweepTop`: the cursors read off the cells at each node's
step, `oracleOf`), the sends encoded (`wireEncode`). Written that way these are separate
structural walks of the graph, each allocating a fresh spine of pairs and recomputing every
node's context `κs ++ Γ` (the context is data: a `Kind` carries its location and grades).

`Dag.sweepF` does these things to each node as the sweep meets it, in the sweep's walk (one
per body sweep of a knot loop); `Dag.sweepF_eq` says the result equals the sweep with the
separate walks, node for node. The compiler substitutes `Dag.iterateBytesImpl` for
`Dag.iterateBytes` by the `@[csimp]` equation `Dag.iterateBytes_eq_impl`: the shells call
`Dag.iterateBytes`, the function in the statements of `paxos_safe_wired` and `WireTrace`, and
run `Dag.iterateBytesImpl`. The `@[csimp]` equations transform the code reasoned about in
proofs to the executable code. The proof of `Dag.iterateBytes_eq_impl` follows the same
mutual induction as `Dag.locStep_eq_impl`: every node-level walk decomposes at `letN` by definition
and by the identities `memoState_eq`/`memoEnv_eq`. A module that imports `Process` but not
this file compiles `iterateBytes` to its definition, which is slower and never different.

**The visits.** The knot loops re-sweep a block's body once per knot round and once per
enclosing sweep, so a node is visited several times per activation, and most visits meet a
node already `done`. Such a visit costs only its context: the result of a visit
is one `SweepRes`, updated in place when unique; a `done` plain node is read off its part at
the `letN`, with no node-level call and no `memoState`/`memoEnv` rebuild; and only a network
node encodes (`Node.encodeMsgs`).
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

/-! ### The inbox bucketed by wire node

`Node.inboxOfWire` at a network node scans every batch's messages for the node's number, for
every network node and every sender. `bucketOf batches k` is the batches with only node
`k`'s messages; `decodeAt` reads the same messages from the bucket as from the whole batch
(`decodeAt_filter`). `bucketArray` computes every bucket in one pass, the messages of a batch
grouped by node through a hash map. The fused walk takes the buckets as a function of the node
number. -/

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

/-- Every bucket in one pass over the batches; `n` buckets, the wire nodes' numbers. -/
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

/-! `bucketArray` compiled. The grouping by node through a hash map (`groupRev`: an `alter`
with a closure per message, then per node a lookup, a `reverse` and a `map` rebuilding every
`(k, v)` pair) is replaced by one pass per batch that conses each message, as it is, onto its
node's bucket of an array of `n`; the node numbers are below `n`. -/

/-- A batch's messages bucketed by node: `n` buckets, each in message order. -/
def bucketBatch (n : Nat) (msgs : List WireMsg) : Array (List WireMsg) :=
  msgs.foldr (fun m acc => if m.1 < n then acc.modify m.1 (m :: ·) else acc) (Array.replicate n [])

theorem size_bucketBatch (n : Nat) (msgs : List WireMsg) : (bucketBatch n msgs).size = n := by
  unfold bucketBatch
  induction msgs with
  | nil => simp
  | cons m msgs ih =>
    rw [List.foldr_cons]
    split
    · rw [Array.size_modify, ih]
    · exact ih

theorem bucketBatch_get (n : Nat) (k : Nat) (hk : k < n) : ∀ (msgs : List WireMsg),
    (bucketBatch n msgs)[k]? = some (msgs.filter fun m => decide (m.1 = k))
  | [] => by simp [bucketBatch, hk]
  | m :: msgs => by
    have ih := bucketBatch_get n k hk msgs
    unfold bucketBatch at ih ⊢
    rw [List.foldr_cons, List.filter_cons]
    by_cases hm : m.1 = k
    · rw [if_pos (hm ▸ hk), Array.getElem?_modify, ih]
      simp [hm]
    · split
      · rw [Array.getElem?_modify, ih]
        have hmk : (m.1 == k) = false := by simp [hm]
        simp [hmk, hm]
      · rw [ih]; simp [hm]

def bucketArrayImpl (n : Nat) (batches : List ((_src : Member L mem) × List WireMsg)) :
    Array (List ((_src : Member L mem) × List WireMsg)) :=
  batches.foldr (fun b acc =>
    let bb := bucketBatch n b.2
    (List.range n).foldl (fun acc k => acc.modify k fun l => ⟨b.1, bb.getD k []⟩ :: l) acc)
    (Array.replicate n [])

omit [DecidableEq L] in
theorem size_bucketArrayImpl (n : Nat) : ∀ (batches : List ((_src : Member L mem) × List WireMsg)),
    (bucketArrayImpl n batches).size = n
  | [] => by simp [bucketArrayImpl]
  | b :: batches => by
    unfold bucketArrayImpl
    rw [List.foldr_cons, size_foldl_modify]
    exact size_bucketArrayImpl n batches

omit [DecidableEq L] in
theorem bucketArrayImpl_get (n : Nat) (k : Nat) (hk : k < n) : ∀ (batches : List ((_src : Member L mem) × List WireMsg)),
    (bucketArrayImpl n batches)[k]? = some (bucketOf batches k)
  | [] => by simp [bucketArrayImpl, bucketOf, hk]
  | b :: batches => by
    have ih := bucketArrayImpl_get n k hk batches
    unfold bucketArrayImpl at ih ⊢
    rw [List.foldr_cons]
    rw [foldl_modify_get _ (List.range n) _ k (List.nodup_range), if_pos (List.mem_range.mpr hk), ih]
    simp only [Option.map_some, bucketOf, List.map_cons]
    have hb : (bucketBatch n b.2).getD k [] = b.2.filter fun m => decide (m.1 = k) := by
      rw [Array.getD_eq_getD_getElem?, bucketBatch_get n k hk]; rfl
    rw [hb]

@[csimp] theorem bucketArray_eq_impl : @bucketArray = @bucketArrayImpl := by
  funext L mem n batches
  apply Array.ext
  · rw [size_bucketArray, size_bucketArrayImpl]
  · intro k hk _
    have hk' : k < n := by rw [size_bucketArray] at hk; exact hk
    have h1 := bucketArray_get n k hk' batches
    have h2 := bucketArrayImpl_get n k hk' batches
    rw [Array.getElem?_eq_getElem hk] at h1
    rw [Array.getElem?_eq_getElem (by rw [size_bucketArrayImpl]; exact hk')] at h2
    exact Option.some.inj (h1.trans h2.symm)

/-! ### The fused walk's result

One object per node visit: its cells and parts, its outputs with their knownness, its
messages and the next wire number, in place of the five nested pairs the separate walks
compose. The compiler updates a unique result in place on the way back up the chain, so a
visit of a node already `done` (the knot loops revisit every node of a body once per sweep)
allocates only its context. -/

/-- The result of a fused sweep over a node or a graph. -/
structure SweepRes (m : Member L mem) (S P : Type) (κs : List (Kind L mem)) where
  cells : S
  part : P
  outs : HList (Kind.locDelta m) κs
  mask : Mask κs
  msgs : List WireMsg
  base : Nat

/-- The separate walks' tuple, as a result. -/
@[reducible] def SweepRes.ofTuple {S P : Type} {κs : List (Kind L mem)}
    (r : (S × P) × (HList (Kind.locDelta m) κs × Mask κs × List WireMsg × Nat)) : SweepRes m S P κs :=
  ⟨r.1.1, r.1.2, r.2.1, r.2.2.1, r.2.2.2.1, r.2.2.2.2⟩

/-- A network node's sends as messages, consed straight off the per-member batches: one pass,
one cons per message. `Node.wireEncode`'s `flatMap`/`map` form compiles to an array
accumulation, its `toList` and a reversed `map`. -/
def encodeHosted {α : Type} (m : Member L mem) (c : L) (base : Nat) (enc : α → ByteArray)
    (snd : Hosted m c → Array α) : List WireMsg :=
  (hostedList m c).foldr (fun j acc => (snd j).foldr (fun v acc => (base, j.1.val, enc v) :: acc) acc) []

theorem foldr_cons_map {α β : Type} (f : α → β) : ∀ (l : List α) (acc : List β),
    l.foldr (fun v acc => f v :: acc) acc = l.map f ++ acc
  | [], _ => rfl
  | x :: l, acc => by rw [List.foldr_cons, foldr_cons_map f l acc, List.map_cons, List.cons_append]

theorem foldr_append_flatMap {α β : Type} (f : α → List β) : ∀ (l : List α),
    l.foldr (fun j acc => f j ++ acc) [] = l.flatMap f
  | [] => rfl
  | x :: l => by rw [List.foldr_cons, foldr_append_flatMap f l, List.flatMap_cons]

theorem encodeHosted_eq {α : Type} (m : Member L mem) (c : L) (base : Nat) (enc : α → ByteArray)
    (snd : Hosted m c → Array α) :
    encodeHosted m c base enc snd
      = (hostedList m c).flatMap (fun j => (snd j).toList.map (fun v => (base, j.1.val, enc v))) := by
  unfold encodeHosted
  rw [← foldr_append_flatMap]
  congr 1
  funext j acc
  rw [← Array.foldr_toList, foldr_cons_map]

/-- A plain node's sends encoded, if this is the encoding sweep. Only a network node has any;
the other nodes' `wireEncode` is `([], base)` and is not computed. -/
def Node.encodeMsgs (m : Member L mem) {Γ : Ctx L mem} {κs : List (Kind L mem)} (n : Node Γ κs)
    (cod : Node.Codecs n) (base : Nat) (enc : Bool) (snd : Node.Sends m n) : List WireMsg :=
  if enc then
    match n, cod, snd with
    | .broadcast_closed (c := c) .., cod, snd => encodeHosted m c base cod.enc snd
    | .demux (c := c) .., cod, snd => encodeHosted m c base cod.enc snd
    | .call N _, cod, snd => (Dag.wireEncode m N cod base snd).1
    | .fixN _ body, cod, snd => (Dag.wireEncode m body cod base snd).1
    | _, _, _ => []
  else []

theorem Node.encodeMsgs_eq (m : Member L mem) {Γ : Ctx L mem} {κs : List (Kind L mem)} (n : Node Γ κs)
    (cod : Node.Codecs n) (base : Nat) (enc : Bool) (snd : Node.Sends m n) :
    Node.encodeMsgs m n cod base enc snd = if enc then (Node.wireEncode m n cod base snd).1 else [] := by
  cases enc <;> cases n <;> first | rfl | (simp only [Node.encodeMsgs, Node.wireEncode, if_true]; exact encodeHosted_eq ..)

/-- One plain node of the fused sweep. At the first visit (`enq`) its inbox is decoded from
its own bucket and enqueued and its cells are materialized; then the sweep's step on it
(`Node.sweepPlain`, the cursors read off the cells); then its sends, the ones its part now
holds, encoded if this is the encoding sweep (`enc`); and the next wire number. A node
already `done` is read off its part. -/
def Node.sweepPlainF (m : Member L mem) (tk : Bool) {Γ : Ctx L mem} {κs : List (Kind L mem)}
    (n : Node Γ κs) (cod : Node.Codecs n) (base : Nat) (buckets : Nat → List ((_src : Member L mem) × List WireMsg))
    (tm : Node.Timing m n) (enq enc : Bool) (Δ : HList (Kind.locDelta m) Γ) (mk : Mask Γ) (s : Node.LocState m n)
    (part : Node.Part m n) : SweepRes m (Node.LocState m n) (Node.Part m n) κs :=
  let s1 := if enq then Node.memoState m n (Node.enqueueInbox m n (Node.inboxOfWire m n cod base (buckets base)).1 s) else s
  match part.1 with
  | .done o snd =>
    ⟨s1, part, o, Mask.const κs true, Node.encodeMsgs m n cod base enc snd, Node.wireCount n base⟩
  | _ =>
    let r := Node.sweepPlain m tk n tm Δ mk s1 part
    ⟨Node.memoState m n r.1.1, r.1.2, memoEnv m r.2.1, r.2.2.1,
      Node.encodeMsgs m n cod base enc (r.1.2.1.sends (Node.sendsNil m n)), Node.wireCount n base⟩

/-- The fused knot loop: `knotLoop` with the inbox and the encoding done at the nodes. The
first body sweep enqueues (`enq`), the later ones do not. The sweep with every knot known is
the final one (`knotLoop` binds nothing new after it) and encodes if `enc`; an exit before
that, the block stuck on an unknown external or out of fuel, encodes by one walk over the
parts' sends (`encode`). -/
def knotLoopF {K outs : List (Kind L mem)} {S P : Type}
    (swF : Bool → Bool → HList (Kind.locDelta m) K → Mask K → S → P → SweepRes m S P (K ++ outs))
    (encode : P → List WireMsg) :
    Nat → Bool → Bool → Mask K → HList (Kind.locDelta m) K → S → P →
    SweepRes m S ((Mask K × HList (Kind.locDelta m) K) × P) (K ++ outs)
  | 0, enq, enc, b, Kv, s, bp =>
    let fin := Mask.all b
    let r := swF enq (enc && fin) Kv b s bp
    ⟨r.cells, ((Mask.or b (HList.take (κs := K) r.mask), Mask.select b Kv (HList.take (κs := K) r.outs)), r.part),
     r.outs, r.mask, (if fin then r.msgs else if enc then encode r.part else []), r.base⟩
  | fuel + 1, enq, enc, b, Kv, s, bp =>
    let fin := Mask.all b
    let r := swF enq (enc && fin) Kv b s bp
    let b' := Mask.or b (HList.take (κs := K) r.mask)
    if Mask.beq b' b then
      ⟨r.cells, ((b', Mask.select b Kv (HList.take (κs := K) r.outs)), r.part),
       r.outs, r.mask, (if fin then r.msgs else if enc then encode r.part else []), r.base⟩
    else knotLoopF swF encode fuel false enc b' (Mask.select b Kv (HList.take (κs := K) r.outs)) r.cells r.part

mutual

/-- The fused sweep over a node: `Node.sweep` with the inbox, the materialization and the
encoding done at the node. `enq` says this is the first visit of these nodes this activation,
so their inboxes are decoded and enqueued; `enc` says this is the sweep that encodes their
sends. A knot block's body sweeps are its knot loop (`knotLoopF`); the final one encodes. -/
def Node.sweepF (m : Member L mem) (tk : Bool) :
    {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) → Node.Codecs n → Nat →
    (Nat → List ((_src : Member L mem) × List WireMsg)) → Node.Timing m n → Bool → Bool →
    HList (Kind.locDelta m) Γ → Mask Γ → Node.LocState m n → Node.Part m n →
    SweepRes m (Node.LocState m n) (Node.Part m n) κs
  | _, _, .call N args, cod, base, buckets, tm, enq, enc, Δ, mk, s, part =>
    let r := Dag.sweepF m tk N cod base buckets tm enq enc (args.map (fun v => Δ.get v)) (args.map (fun v => mk.get v)) s part.2
    ⟨r.cells, (part.1, r.part), r.outs, r.mask, r.msgs, r.base⟩
  | _, _, .fixN ks body, cod, base, buckets, tm, enq, enc, Δ, mk, s, part =>
    let r := knotLoopF
      (fun enq enc Kv b s bp => Dag.sweepF m tk body cod base buckets tm enq enc (Kv.append Δ) (b.append mk) s bp)
      (fun bp => (Dag.wireEncode m body cod base (Dag.partSends m body bp)).1)
      ks.length enq enc part.2.1.1 part.2.1.2 s part.2.2
    ⟨r.cells, (part.1, r.part), r.outs, r.mask, r.msgs, r.base⟩
  | _, _, n, cod, base, buckets, tm, enq, enc, Δ, mk, s, part => Node.sweepPlainF m tk n cod base buckets tm enq enc Δ mk s part

/-- The fused sweep over a graph: the wire numbers threaded in chain order, the messages in
chain order. -/
def Dag.sweepF (m : Member L mem) (tk : Bool) :
    {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) → Dag.Codecs d → Nat →
    (Nat → List ((_src : Member L mem) × List WireMsg)) → Dag.Timing m d → Bool → Bool →
    HList (Kind.locDelta m) Γ → Mask Γ → Dag.LocState m d → Dag.Part m d →
    SweepRes m (Dag.LocState m d) (Dag.Part m d) outs
  | _, _, .ret xs, _, base, _, _, _, _, Δ, mk, s, part => ⟨s, part, xs.map (fun v => Δ.get v), xs.map (fun v => mk.get v), [], base⟩
  | _, _, .letN n rest, cod, base, buckets, tm, enq, enc, Δ, mk, s, part =>
    match hp : Node.isPlain n, part.1.1 with
    | true, .done o snd =>
      -- a plain node already `done`, the common case of a re-sweep: read off its part, no
      -- node-level call; its inbox, the first visit's, was enqueued when it was stepped
      let s1 := if enq then Node.memoState m n (Node.enqueueInbox m n (Node.inboxOfWire m n cod.1 base (buckets base)).1 s.1) else s.1
      let r' := Dag.sweepF m tk rest cod.2 (Node.wireCount n base) buckets tm.2 enq enc (o.append Δ) ((Mask.const _ true).append mk) s.2 part.2
      ⟨(s1, r'.cells), (part.1, r'.part), r'.outs, r'.mask, Node.encodeMsgs m n cod.1 base enc snd ++ r'.msgs, r'.base⟩
    | _, _ =>
      let r := Node.sweepF m tk n cod.1 base buckets tm.1 enq enc Δ mk s.1 part.1
      let r' := Dag.sweepF m tk rest cod.2 r.base buckets tm.2 enq enc (r.outs.append Δ) (r.mask.append mk) s.2 part.2
      ⟨(r.cells, r'.cells), (r.part, r'.part), r'.outs, r'.mask, r.msgs ++ r'.msgs, r'.base⟩

end

/-- A plain node's sweep is `Node.sweepPlain`. -/
theorem Node.sweep_plain (m : Member L mem) (tk : Bool) {Γ : Ctx L mem} {κs : List (Kind L mem)} (n : Node Γ κs)
    (hp : Node.isPlain n = true) (tm : Node.Timing m n) (Δ : HList (Kind.locDelta m) Γ) (mk : Mask Γ)
    (s : Node.LocState m n) (part : Node.Part m n) :
    Node.sweep m tk n tm Δ mk s part = Node.sweepPlain m tk n tm Δ mk s part := by
  cases n <;> first | rfl | nomatch hp

/-- A plain node's part sends are its phase's. -/
theorem Node.partSends_plain (m : Member L mem) {Γ : Ctx L mem} {κs : List (Kind L mem)} (n : Node Γ κs)
    (hp : Node.isPlain n = true) (part : Node.Part m n) :
    Node.partSends m n part = part.1.sends (Node.sendsNil m n) := by
  cases n <;> first | rfl | nomatch hp

/-- A plain node's fused sweep when it is already `done`: what the node-level walk returns. -/
theorem Node.sweepF_done (m : Member L mem) (tk : Bool) {Γ : Ctx L mem} {κs : List (Kind L mem)} (n : Node Γ κs)
    (hp : Node.isPlain n = true) (cod : Node.Codecs n) (base : Nat) (buckets : Nat → List ((_src : Member L mem) × List WireMsg))
    (tm : Node.Timing m n) (enq enc : Bool) (Δ : HList (Kind.locDelta m) Γ) (mk : Mask Γ) (s : Node.LocState m n)
    (part : Node.Part m n) (o : HList (Kind.locDelta m) κs) (snd : Node.Sends m n) (hd : part.1 = .done o snd) :
    Node.sweepF m tk n cod base buckets tm enq enc Δ mk s part =
      ⟨(if enq then Node.memoState m n (Node.enqueueInbox m n (Node.inboxOfWire m n cod base (buckets base)).1 s) else s),
       part, o, Mask.const κs true, Node.encodeMsgs m n cod base enc snd, Node.wireCount n base⟩ := by
  obtain ⟨ph, sub⟩ := part
  simp only at hd
  subst hd
  cases n <;> first | rfl | nomatch hp

/-- What the separate walks compute for a node: the inbox decoded and enqueued (if `enq`),
the sweep, the parts' sends encoded (if `enc`). -/
def Node.sweepSpec (m : Member L mem) (tk : Bool) {Γ : Ctx L mem} {κs : List (Kind L mem)}
    (n : Node Γ κs) (cod : Node.Codecs n) (base : Nat) (batches : List ((_src : Member L mem) × List WireMsg))
    (tm : Node.Timing m n) (enq enc : Bool) (Δ : HList (Kind.locDelta m) Γ) (mk : Mask Γ) (s : Node.LocState m n)
    (part : Node.Part m n) :
    (Node.LocState m n × Node.Part m n) × (HList (Kind.locDelta m) κs × Mask κs × List WireMsg × Nat) :=
  let s1 := if enq then Node.memoState m n (Node.enqueueInbox m n (Node.inboxOfWire m n cod base batches).1 s) else s
  let r := Node.sweep m tk n tm Δ mk s1 part
  (r.1, (r.2.1, r.2.2.1, (if enc then (Node.wireEncode m n cod base (Node.partSends m n r.1.2)).1 else []),
    (Node.wireReceivers n base).2))

/-- What the separate walks compute for a graph. -/
def Dag.sweepSpec (m : Member L mem) (tk : Bool) {Γ : Ctx L mem} {outs : List (Kind L mem)}
    (d : Dag Γ outs) (cod : Dag.Codecs d) (base : Nat) (batches : List ((_src : Member L mem) × List WireMsg))
    (tm : Dag.Timing m d) (enq enc : Bool) (Δ : HList (Kind.locDelta m) Γ) (mk : Mask Γ) (s : Dag.LocState m d)
    (part : Dag.Part m d) :
    (Dag.LocState m d × Dag.Part m d) × (HList (Kind.locDelta m) outs × Mask outs × List WireMsg × Nat) :=
  let s1 := if enq then Dag.memoState m d (Dag.enqueueInbox m d (Dag.inboxOfWire m d cod base batches).1 s) else s
  let r := Dag.sweep m tk d tm Δ mk s1 part
  (r.1, (r.2.1, r.2.2.1, (if enc then (Dag.wireEncode m d cod base (Dag.partSends m d r.1.2)).1 else []),
    (Dag.wireReceivers d base).2))

/-- A node reads the same inbox from its own bucket (`decodeAt_filter`). -/
theorem Node.inboxOfWire_bucketOf (m : Member L mem) {Γ : Ctx L mem} {κs : List (Kind L mem)} (n : Node Γ κs)
    (cod : Node.Codecs n) (base : Nat) (batches : List ((_src : Member L mem) × List WireMsg))
    (hp : Node.isPlain n = true) :
    Node.inboxOfWire m n cod base (bucketOf batches base) = Node.inboxOfWire m n cod base batches := by
  cases n with
  | call N args => cases hp
  | fixN ks body => cases hp
  | broadcast_closed =>
    unfold Node.inboxOfWire bucketOf
    congr 1
    funext i j
    rw [appendFold_map]
    apply appendFold_congr
    intro b _
    have h := decodeAt_filter cod b.2 base j.val
    simp only [h]
  | demux =>
    unfold Node.inboxOfWire bucketOf
    congr 1
    funext i j
    rw [appendFold_map]
    apply appendFold_congr
    intro b _
    have h := decodeAt_filter cod b.2 base j.val
    simp only [h]
  | _ => rfl

/-- The plain node's fused sweep is what the separate walks compute for it. -/
theorem Node.sweepPlainF_eq (m : Member L mem) (tk : Bool)
    (batches : List ((_src : Member L mem) × List WireMsg))
    (buckets : Nat → List ((_src : Member L mem) × List WireMsg)) (hb : ∀ k, buckets k = bucketOf batches k)
    {Γ : Ctx L mem} {κs : List (Kind L mem)} (n : Node Γ κs) (cod : Node.Codecs n) (base : Nat)
    (tm : Node.Timing m n) (enq enc : Bool) (Δ : HList (Kind.locDelta m) Γ) (mk : Mask Γ) (s : Node.LocState m n)
    (part : Node.Part m n) (hp : Node.isPlain n = true) :
    Node.sweepPlainF m tk n cod base buckets tm enq enc Δ mk s part
      = SweepRes.ofTuple (Node.sweepSpec m tk n cod base batches tm enq enc Δ mk s part) := by
  obtain ⟨ph, sub⟩ := part
  simp only [Node.sweepPlainF, Node.sweepSpec, SweepRes.ofTuple, hb, Node.inboxOfWire_bucketOf m n cod base batches hp,
    Node.memoState_eq, memoEnv_eq, Node.wireCount_eq, Node.encodeMsgs_eq]
  cases ph <;> cases n <;> first
    | rfl
    | nomatch hp

/-- **The fused knot loop equals the knot loop with the separate walks**, given that each
fused body sweep equals the separate one (`hsw`): the inbox enqueued by the first sweep, the messages
those of the parts the loop ends with. -/
theorem knotLoopF_eq {K outs : List (Kind L mem)} {S P Snd : Type}
    (swF : Bool → Bool → HList (Kind.locDelta m) K → Mask K → S → P → SweepRes m S P (K ++ outs))
    (sw : HList (Kind.locDelta m) K → Mask K → S → P → (S × P) × (HList (Kind.locDelta m) (K ++ outs) × Mask (K ++ outs) × Snd))
    (encode : P → List WireMsg) (s1 : Bool → S → S) (hs1 : ∀ s, s1 false s = s) (wr : Nat)
    (hsw : ∀ enq enc Kv b s bp, swF enq enc Kv b s bp =
      ⟨(sw Kv b (s1 enq s) bp).1.1, (sw Kv b (s1 enq s) bp).1.2, (sw Kv b (s1 enq s) bp).2.1, (sw Kv b (s1 enq s) bp).2.2.1,
        (if enc then encode (sw Kv b (s1 enq s) bp).1.2 else []), wr⟩) :
    ∀ (fuel : Nat) (enq enc : Bool) (b : Mask K) (Kv : HList (Kind.locDelta m) K) (s : S) (bp : P),
      knotLoopF swF encode fuel enq enc b Kv s bp =
        ⟨(knotLoop sw fuel b Kv (s1 enq s) bp).1.1, (knotLoop sw fuel b Kv (s1 enq s) bp).1.2,
         (knotLoop sw fuel b Kv (s1 enq s) bp).2.1, (knotLoop sw fuel b Kv (s1 enq s) bp).2.2.1,
         (if enc then encode (knotLoop sw fuel b Kv (s1 enq s) bp).1.2.2 else []), wr⟩
  | 0, enq, enc, b, Kv, s, bp => by
    simp only [knotLoopF, knotLoop, knotFinalize, hsw]
    cases enc <;> cases Mask.all b <;> simp
  | fuel + 1, enq, enc, b, Kv, s, bp => by
    simp only [knotLoopF, knotLoop, knotFinalize, hsw]
    split
    · cases enc <;> cases Mask.all b <;> simp
    · rw [knotLoopF_eq swF sw encode s1 hs1 wr hsw fuel false enc, hs1]

mutual

/-- **The fused sweep equals the sweep with the separate walks**, node for node. -/
theorem Node.sweepF_eq (m : Member L mem) (tk : Bool)
    (batches : List ((_src : Member L mem) × List WireMsg))
    (buckets : Nat → List ((_src : Member L mem) × List WireMsg)) (hb : ∀ k, buckets k = bucketOf batches k) :
    {Γ : Ctx L mem} → {κs : List (Kind L mem)} → (n : Node Γ κs) → (cod : Node.Codecs n) → (base : Nat) →
    (tm : Node.Timing m n) → (enq enc : Bool) → (Δ : HList (Kind.locDelta m) Γ) → (mk : Mask Γ) →
    (s : Node.LocState m n) → (part : Node.Part m n) →
    Node.sweepF m tk n cod base buckets tm enq enc Δ mk s part
      = SweepRes.ofTuple (Node.sweepSpec m tk n cod base batches tm enq enc Δ mk s part)
  | _, _, .call N args, cod, base, tm, enq, enc, Δ, mk, s, part => by
    have h := Dag.sweepF_eq m tk batches buckets hb N cod base tm enq enc (args.map (fun v => Δ.get v))
      (args.map (fun v => mk.get v)) s part.2
    show (let r := Dag.sweepF m tk N cod base buckets tm enq enc (args.map (fun v => Δ.get v)) (args.map (fun v => mk.get v)) s part.2
      (⟨r.cells, (part.1, r.part), r.outs, r.mask, r.msgs, r.base⟩ : SweepRes m _ _ _)) = _
    rw [h]
    unfold Node.sweepSpec Dag.sweepSpec
    cases enq <;> simp only [Node.memoState_eq, Dag.memoState_eq, Bool.false_eq_true, ↓reduceIte] <;> rfl
  | _, _, .fixN ks body, cod, base, tm, enq, enc, Δ, mk, s, part => by
    change Dag.Codecs body at cod
    change Dag.Timing m body at tm
    have hsw : ∀ (enq enc : Bool) (Kv : HList (Kind.locDelta m) (ks.map KnotKind.toKind)) (b : Mask (ks.map KnotKind.toKind))
        (s : Dag.LocState m body) (bp : Dag.Part m body),
        Dag.sweepF m tk body cod base buckets tm enq enc (Kv.append Δ) (b.append mk) s bp =
          ⟨(Dag.sweep m tk body tm (Kv.append Δ) (b.append mk)
              (if enq then Dag.memoState m body (Dag.enqueueInbox m body (Dag.inboxOfWire m body cod base batches).1 s) else s) bp).1.1,
           (Dag.sweep m tk body tm (Kv.append Δ) (b.append mk)
              (if enq then Dag.memoState m body (Dag.enqueueInbox m body (Dag.inboxOfWire m body cod base batches).1 s) else s) bp).1.2,
           (Dag.sweep m tk body tm (Kv.append Δ) (b.append mk)
              (if enq then Dag.memoState m body (Dag.enqueueInbox m body (Dag.inboxOfWire m body cod base batches).1 s) else s) bp).2.1,
           (Dag.sweep m tk body tm (Kv.append Δ) (b.append mk)
              (if enq then Dag.memoState m body (Dag.enqueueInbox m body (Dag.inboxOfWire m body cod base batches).1 s) else s) bp).2.2.1,
           (if enc then (Dag.wireEncode m body cod base (Dag.partSends m body
              (Dag.sweep m tk body tm (Kv.append Δ) (b.append mk)
                (if enq then Dag.memoState m body (Dag.enqueueInbox m body (Dag.inboxOfWire m body cod base batches).1 s) else s) bp).1.2)).1
             else []),
           (Dag.wireReceivers body base).2⟩ := by
      intro enq enc Kv b s bp
      rw [Dag.sweepF_eq m tk batches buckets hb body cod base tm enq enc]
      rfl
    show (let r := knotLoopF
            (fun (enq enc : Bool) (Kv : HList (Kind.locDelta m) (ks.map KnotKind.toKind)) (b : Mask (ks.map KnotKind.toKind))
              (s : Dag.LocState m body) (bp : Dag.Part m body) =>
              Dag.sweepF m tk body cod base buckets tm enq enc (Kv.append Δ) (b.append mk) s bp)
            (fun bp => (Dag.wireEncode m body cod base (Dag.partSends m body bp)).1)
            ks.length enq enc part.2.1.1 part.2.1.2 s part.2.2
          (⟨r.cells, (part.1, r.part), r.outs, r.mask, r.msgs, r.base⟩ : SweepRes m _ _ _)) = _
    have e := knotLoopF_eq
      (fun (enq enc : Bool) (Kv : HList (Kind.locDelta m) (ks.map KnotKind.toKind)) (b : Mask (ks.map KnotKind.toKind))
        (s : Dag.LocState m body) (bp : Dag.Part m body) =>
        Dag.sweepF m tk body cod base buckets tm enq enc (Kv.append Δ) (b.append mk) s bp)
      (fun (Kv : HList (Kind.locDelta m) (ks.map KnotKind.toKind)) (b : Mask (ks.map KnotKind.toKind))
        (s : Dag.LocState m body) (bp : Dag.Part m body) => Dag.sweep m tk body tm (Kv.append Δ) (b.append mk) s bp)
      (fun bp => (Dag.wireEncode m body cod base (Dag.partSends m body bp)).1)
      (fun enq s => if enq then Dag.memoState m body (Dag.enqueueInbox m body (Dag.inboxOfWire m body cod base batches).1 s) else s)
      (fun _ => rfl) (Dag.wireReceivers body base).2 hsw ks.length enq enc part.2.1.1 part.2.1.2 s part.2.2
    rw [e]
    unfold Node.sweepSpec
    cases enq <;> simp only [Node.memoState_eq, Dag.memoState_eq, Bool.false_eq_true, ↓reduceIte] <;> rfl
  | _, _, .map .., cod, base, tm, enq, enc, Δ, mk, s, part =>
    Node.sweepPlainF_eq m tk batches buckets hb _ cod base tm enq enc Δ mk s part rfl
  | _, _, .filterMap .., cod, base, tm, enq, enc, Δ, mk, s, part =>
    Node.sweepPlainF_eq m tk batches buckets hb _ cod base tm enq enc Δ mk s part rfl
  | _, _, .broadcast_closed .., cod, base, tm, enq, enc, Δ, mk, s, part =>
    Node.sweepPlainF_eq m tk batches buckets hb _ cod base tm enq enc Δ mk s part rfl
  | _, _, .demux .., cod, base, tm, enq, enc, Δ, mk, s, part =>
    Node.sweepPlainF_eq m tk batches buckets hb _ cod base tm enq enc Δ mk s part rfl
  | _, _, .values .., cod, base, tm, enq, enc, Δ, mk, s, part =>
    Node.sweepPlainF_eq m tk batches buckets hb _ cod base tm enq enc Δ mk s part rfl
  | _, _, .into_keyed .., cod, base, tm, enq, enc, Δ, mk, s, part =>
    Node.sweepPlainF_eq m tk batches buckets hb _ cod base tm enq enc Δ mk s part rfl
  | _, _, .kassume_ordering .., cod, base, tm, enq, enc, Δ, mk, s, part =>
    Node.sweepPlainF_eq m tk batches buckets hb _ cod base tm enq enc Δ mk s part rfl
  | _, _, .kfold_early_stop .., cod, base, tm, enq, enc, Δ, mk, s, part =>
    Node.sweepPlainF_eq m tk batches buckets hb _ cod base tm enq enc Δ mk s part rfl
  | _, _, .kget_max_key .., cod, base, tm, enq, enc, Δ, mk, s, part =>
    Node.sweepPlainF_eq m tk batches buckets hb _ cod base tm enq enc Δ mk s part rfl
  | _, _, .weaken_retries .., cod, base, tm, enq, enc, Δ, mk, s, part =>
    Node.sweepPlainF_eq m tk batches buckets hb _ cod base tm enq enc Δ mk s part rfl
  | _, _, .union .., cod, base, tm, enq, enc, Δ, mk, s, part =>
    Node.sweepPlainF_eq m tk batches buckets hb _ cod base tm enq enc Δ mk s part rfl
  | _, _, .assume_ordering .., cod, base, tm, enq, enc, Δ, mk, s, part =>
    Node.sweepPlainF_eq m tk batches buckets hb _ cod base tm enq enc Δ mk s part rfl
  | _, _, .fold .., cod, base, tm, enq, enc, Δ, mk, s, part =>
    Node.sweepPlainF_eq m tk batches buckets hb _ cod base tm enq enc Δ mk s part rfl
  | _, _, .fold_monotone .., cod, base, tm, enq, enc, Δ, mk, s, part =>
    Node.sweepPlainF_eq m tk batches buckets hb _ cod base tm enq enc Δ mk s part rfl
  | _, _, .snapshot .., cod, base, tm, enq, enc, Δ, mk, s, part =>
    Node.sweepPlainF_eq m tk batches buckets hb _ cod base tm enq enc Δ mk s part rfl
  | _, _, .batch .., cod, base, tm, enq, enc, Δ, mk, s, part =>
    Node.sweepPlainF_eq m tk batches buckets hb _ cod base tm enq enc Δ mk s part rfl
  | _, _, .source_interval .., cod, base, tm, enq, enc, Δ, mk, s, part =>
    Node.sweepPlainF_eq m tk batches buckets hb _ cod base tm enq enc Δ mk s part rfl
  | _, _, .firstTick .., cod, base, tm, enq, enc, Δ, mk, s, part =>
    Node.sweepPlainF_eq m tk batches buckets hb _ cod base tm enq enc Δ mk s part rfl
  | _, _, .mapTick .., cod, base, tm, enq, enc, Δ, mk, s, part =>
    Node.sweepPlainF_eq m tk batches buckets hb _ cod base tm enq enc Δ mk s part rfl
  | _, _, .zipTick .., cod, base, tm, enq, enc, Δ, mk, s, part =>
    Node.sweepPlainF_eq m tk batches buckets hb _ cod base tm enq enc Δ mk s part rfl
  | _, _, .defer_tick .., cod, base, tm, enq, enc, Δ, mk, s, part =>
    Node.sweepPlainF_eq m tk batches buckets hb _ cod base tm enq enc Δ mk s part rfl
  | _, _, .allTicks .., cod, base, tm, enq, enc, Δ, mk, s, part =>
    Node.sweepPlainF_eq m tk batches buckets hb _ cod base tm enq enc Δ mk s part rfl
  | _, _, .flattenOrdered .., cod, base, tm, enq, enc, Δ, mk, s, part =>
    Node.sweepPlainF_eq m tk batches buckets hb _ cod base tm enq enc Δ mk s part rfl
  | _, _, .flattenUnordered .., cod, base, tm, enq, enc, Δ, mk, s, part =>
    Node.sweepPlainF_eq m tk batches buckets hb _ cod base tm enq enc Δ mk s part rfl
  | _, _, .tinto_keyed .., cod, base, tm, enq, enc, Δ, mk, s, part =>
    Node.sweepPlainF_eq m tk batches buckets hb _ cod base tm enq enc Δ mk s part rfl
  | _, _, .tassume_ordering .., cod, base, tm, enq, enc, Δ, mk, s, part =>
    Node.sweepPlainF_eq m tk batches buckets hb _ cod base tm enq enc Δ mk s part rfl
  | _, _, .reduce_watermark .., cod, base, tm, enq, enc, Δ, mk, s, part =>
    Node.sweepPlainF_eq m tk batches buckets hb _ cod base tm enq enc Δ mk s part rfl
  | _, _, .kentries .., cod, base, tm, enq, enc, Δ, mk, s, part =>
    Node.sweepPlainF_eq m tk batches buckets hb _ cod base tm enq enc Δ mk s part rfl
  | _, _, .kmap .., cod, base, tm, enq, enc, Δ, mk, s, part =>
    Node.sweepPlainF_eq m tk batches buckets hb _ cod base tm enq enc Δ mk s part rfl
  | _, _, .kallTicks .., cod, base, tm, enq, enc, Δ, mk, s, part =>
    Node.sweepPlainF_eq m tk batches buckets hb _ cod base tm enq enc Δ mk s part rfl
  | _, _, .kfold .., cod, base, tm, enq, enc, Δ, mk, s, part =>
    Node.sweepPlainF_eq m tk batches buckets hb _ cod base tm enq enc Δ mk s part rfl
  | _, _, .ksnapshot .., cod, base, tm, enq, enc, Δ, mk, s, part =>
    Node.sweepPlainF_eq m tk batches buckets hb _ cod base tm enq enc Δ mk s part rfl
  | _, _, .tfold .., cod, base, tm, enq, enc, Δ, mk, s, part =>
    Node.sweepPlainF_eq m tk batches buckets hb _ cod base tm enq enc Δ mk s part rfl
  | _, _, .latest .., cod, base, tm, enq, enc, Δ, mk, s, part =>
    Node.sweepPlainF_eq m tk batches buckets hb _ cod base tm enq enc Δ mk s part rfl
  | _, _, .latest_monotone .., cod, base, tm, enq, enc, Δ, mk, s, part =>
    Node.sweepPlainF_eq m tk batches buckets hb _ cod base tm enq enc Δ mk s part rfl
  | _, _, .time .., cod, base, tm, enq, enc, Δ, mk, s, part =>
    Node.sweepPlainF_eq m tk batches buckets hb _ cod base tm enq enc Δ mk s part rfl
  | _, _, .snapshot_bounded .., cod, base, tm, enq, enc, Δ, mk, s, part =>
    Node.sweepPlainF_eq m tk batches buckets hb _ cod base tm enq enc Δ mk s part rfl
  | _, _, .tick_scan .., cod, base, tm, enq, enc, Δ, mk, s, part =>
    Node.sweepPlainF_eq m tk batches buckets hb _ cod base tm enq enc Δ mk s part rfl

theorem Dag.sweepF_eq (m : Member L mem) (tk : Bool)
    (batches : List ((_src : Member L mem) × List WireMsg))
    (buckets : Nat → List ((_src : Member L mem) × List WireMsg)) (hb : ∀ k, buckets k = bucketOf batches k) :
    {Γ : Ctx L mem} → {outs : List (Kind L mem)} → (d : Dag Γ outs) → (cod : Dag.Codecs d) → (base : Nat) →
    (tm : Dag.Timing m d) → (enq enc : Bool) → (Δ : HList (Kind.locDelta m) Γ) → (mk : Mask Γ) →
    (s : Dag.LocState m d) → (part : Dag.Part m d) →
    Dag.sweepF m tk d cod base buckets tm enq enc Δ mk s part
      = SweepRes.ofTuple (Dag.sweepSpec m tk d cod base batches tm enq enc Δ mk s part)
  | _, _, .ret xs, _, base, _, enq, enc, Δ, mk, s, part => by
    have hs : s = () := rfl
    subst hs
    cases enq <;> cases enc <;> rfl
  | _, _, .letN n rest, cod, base, tm, enq, enc, Δ, mk, s, part => by
    have h1 := Node.sweepF_eq m tk batches buckets hb n cod.1 base tm.1 enq enc Δ mk s.1 part.1
    have h2 := Dag.sweepF_eq m tk batches buckets hb rest cod.2
      (Node.sweepSpec m tk n cod.1 base batches tm.1 enq enc Δ mk s.1 part.1).2.2.2.2 tm.2 enq enc
      ((Node.sweepSpec m tk n cod.1 base batches tm.1 enq enc Δ mk s.1 part.1).2.1.append Δ)
      ((Node.sweepSpec m tk n cod.1 base batches tm.1 enq enc Δ mk s.1 part.1).2.2.1.append mk) s.2 part.2
    -- the fast path for a plain `done` node returns the node-level walk's answer (`Node.sweepF_done`)
    have hfast : Dag.sweepF m tk (.letN n rest) cod base buckets tm enq enc Δ mk s part =
        (let r := Node.sweepF m tk n cod.1 base buckets tm.1 enq enc Δ mk s.1 part.1
         let r' := Dag.sweepF m tk rest cod.2 r.base buckets tm.2 enq enc (r.outs.append Δ) (r.mask.append mk) s.2 part.2
         (⟨(r.cells, r'.cells), (r.part, r'.part), r'.outs, r'.mask, r.msgs ++ r'.msgs, r'.base⟩ : SweepRes m _ _ _)) := by
      simp only [Dag.sweepF]
      split
      · next o snd hp hd =>
        rw [Node.sweepF_done m tk n hp cod.1 base buckets tm.1 enq enc Δ mk s.1 part.1 o snd hd]
        exact rfl
      · rfl
    rw [hfast]
    simp only [h1, h2]
    unfold Dag.sweepSpec Node.sweepSpec
    -- the graph-level walks at `letN` split into the node's and the rest's; the wire numbering is
    -- `wireReceivers`' either way
    cases enq <;> cases enc <;>
      simp only [Dag.inboxOfWire, Dag.enqueueInbox, Dag.memoState, Dag.sweep, Dag.wireEncode, Dag.wireReceivers,
        Dag.partSends, Node.inboxOfWire_snd_eq, Node.wireEncode_snd_eq, Node.memoState_eq, memoEnv_eq,
        Dag.memoState_eq, Bool.false_eq_true, ↓reduceIte, List.nil_append, SweepRes.ofTuple] <;>
      rfl

end

/-- **The compiled `Dag.iterateBytes`**: the frames parsed once per connection, the messages
bucketed by wire node, the graph swept (`Dag.sweepF`: the inboxes enqueued, the cursors read,
the sends encoded at the nodes). `Dag.iterateBytes` (`Project/Process.lean`) is the function
the theorems are about and the one the shells call; `@[csimp] Dag.iterateBytes_eq_impl` is
the kernel-checked equation the compiler substitutes this for. -/
def Dag.iterateBytesImpl (m : Member L mem) (tk : Bool) {Γ : Ctx L mem} {outs : List (Kind L mem)} (d : Dag Γ outs)
    (cod : Dag.Codecs d) (tm : Dag.Timing m d) (chans : List ((_src : Member L mem) × ByteArray))
    (Δ : HList (Kind.locDelta m) Γ) (s : Dag.LocState m d) :
    Dag.LocState m d × HList (Kind.locDelta m) outs × List WireMsg × List ByteArray :=
  let parsed := chans.map fun c => (⟨c.1, parseFrames c.2⟩ : (_src : Member L mem) × List WireMsg × ByteArray)
  let batches := parsed.map fun c => (⟨c.1, c.2.1⟩ : (_src : Member L mem) × List WireMsg)
  let arr := bucketArray (Dag.wireCount d 0) batches
  let r := Dag.sweepF m tk d cod 0 (fun k => (arr[k]?).getD (bucketOf batches k)) tm true true Δ (Mask.const Γ true) s
    (Dag.Part.init m d)
  (r.cells, r.outs, r.msgs, parsed.map fun c => c.2.2)

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
  simp only [Dag.iterateBytes, Dag.iterateBytesImpl, Dag.iterateWire, Dag.iterate, Dag.sweepTop,
    List.map_map, Function.comp_def]
  rw [Dag.sweepF_eq m tk _ _ (fun k => bucketArray_getD _ _ k)]
  simp only [Dag.sweepSpec, SweepRes.ofTuple, ↓reduceIte, Dag.sweep_sends]

end Project

end Hydro
