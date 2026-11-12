import Hydro.Project.Process

/-!
# Hydro · Project · the shell's share of the wire

What a socket shell does between the sockets and `Dag.iterateBytes`, in one place so the two
shells (`lake exe wired`, `lake exe bench`) do it the same way: identify a connection when it
is accepted (`hello`), gather the bytes its readers pulled off each connection since the last
activation (`groupChunks`), hand the body each connection's kept tail followed by those bytes
(`chansOf`), keep the tails it returns (`storeTails`), and route the sends to the peers
(`routeSendsIdx`). The bytes are never looked at here: cutting them into frames is
`Dag.iterateBytes`'s (`parseFrames`), and the theorem's (`WireTrace`).
-/

namespace Hydro

namespace Project

namespace Shell

/-- The one byte a dialer writes first on a new connection: its process index. The accepting
side reads it before anything else; it is connection setup, not program traffic. -/
def hello (p : Nat) : ByteArray := ByteArray.mk #[p.toUInt8]

def helloOf (b : ByteArray) : Option Nat := if h : 0 < b.size then some (b[0]'h).toNat else none

/-- The chunks the readers pushed since the last activation, `(connection, bytes)` in arrival
order, grouped per connection: each connection once, in order of first arrival, its chunks
concatenated in order. -/
def groupChunks {n : Nat} (items : Array (Fin n × ByteArray)) : Array (Fin n × ByteArray) :=
  items.foldl (fun acc (q, b) =>
    match acc.findIdx? (·.1 = q) with
    | some i => acc.modify i fun (q', b') => (q', b' ++ b)
    | none => acc.push (q, b)) #[]

/-- The connections to hand the body: for each connection read, its kept tail then the new
bytes. -/
def chansOf {L : Type} {mem : L → Nat} {n : Nat} (memberOf : Fin n → Member L mem) (tails : Array ByteArray)
    (grouped : Array (Fin n × ByteArray)) : List ((src : Member L mem) × ByteArray) :=
  grouped.toList.map fun (q, b) => ⟨memberOf q, tails.getD q.val .empty ++ b⟩

/-- Keep the tails the body returned, one per connection handed, in order. -/
def storeTails {n : Nat} (tails : Array ByteArray) (grouped : Array (Fin n × ByteArray)) (ret : List ByteArray) :
    Array ByteArray :=
  (grouped.toList.zip ret).foldl (fun acc ((q, _), t) => acc.setIfInBounds q.val t) tails

/-! ### Routing an activation's sends to the peers

A process's sends carry their wire node; the routing table (`Dag.wireRoutes`) says which
cluster receives each node and, for a `demux`, which address names which member. One pass
over the sends buckets them per peer process: a message goes to every peer hosting the
receiver cluster, and for a `demux` to the one whose member the payload's address names. -/

/-- The routing table as an array indexed by wire node: the receiver cluster and, for a
`demux`, the addresses. -/
def routeTable {L : Type} (routes : List (Nat × L × Option (Nat → Option Nat))) : Array (Option (L × Option (Nat → Option Nat))) :=
  let n := routes.foldl (fun acc r => max acc (r.1 + 1)) 0
  routes.foldl (fun arr (k, ℓ, a) => arr.setIfInBounds k (some (ℓ, a))) (Array.replicate n none)

/-- The routing table resolved against the peers once: per wire node, the peers hosting the
receiver cluster with their member's address (the `demux` address function applied to the
member, `none` at a `broadcast`), so that routing a message tests the node's peers only. -/
def routeIndex {L : Type} {mem : L → Nat} [DecidableEq L] {n : Nat} (table : Array (Option (L × Option (Nat → Option Nat))))
    (peerOf : Fin n → Member L mem) : Array (Option (Bool × Array (Nat × Option Nat))) :=
  table.map fun e => e.map fun (ℓ, addr) =>
    (addr.isSome, ((List.finRange n).filterMap fun q =>
      let m := peerOf q
      if m.1 = ℓ then some (q.val, addr.bind fun f => f m.2.val) else none).toArray)

/-- Bucket the sends per peer through the index: per message, the node's peers only. Peer `q`
receives message `msg` iff its cluster is the node's receiver and, at a `demux`, its member's
address is the payload's. Peers in order; each bucket in send order. -/
def routeSendsIdx {n : Nat} (idx : Array (Option (Bool × Array (Nat × Option Nat)))) (sends : List WireMsg) :
    Array (List WireMsg) :=
  let buckets : Array (Array WireMsg) := sends.foldl (fun bs msg =>
    match idx.getD msg.1 none with
    | none => bs
    | some (isDemux, peers) =>
      if isDemux then
        let t := demuxTarget msg.2.2
        peers.foldl (fun bs (q, a) => if a = t then bs.modify q (·.push msg) else bs) bs
      else peers.foldl (fun bs (q, _) => bs.modify q (·.push msg)) bs) (Array.replicate n #[])
  buckets.map (·.toList)

end Shell

end Project

end Hydro
