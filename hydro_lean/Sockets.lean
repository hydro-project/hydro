import Hydro.Project.Shell
import Std.Async.TCP
import Std.Sync.Mutex

/-!
# The harnesses' sockets (D94)

What `wired` and `bench` share to run processes over local TCP: a process's inbox (chunks
of bytes per connection, under a mutex with a condition variable the readers and timers
signal — an idle machine waits here), the sender (one writer thread per process issuing
`sendAll` without blocking), readers as `Async` continuation chains, the hello handshake
(`Shell.hello`: the dialer's first byte names it) and the dial-with-retry. None of it is in
any theorem: the library's loop body (`Dag.iterateBytes`) takes the bytes these deliver.

Threads: a thread blocked in `Task.get`/`.block` is woken by **every** task completion in
the process (one condition variable in Lean's task manager) — eighteen blocked
reader/writer threads were ~2 cores per process (D94). So no thread blocks on a socket: a
receive's continuation (push, re-arm) runs on a task worker, and the sends are queued for
the writer, which never waits on their completion.
-/

open Hydro Hydro.Project Std Std.Async

namespace Sockets

/-- What the readers push: the connection and a chunk of bytes, as the socket delivered it. -/
abbrev Chunk (n : Nat) := Fin n × ByteArray

/-- A process's inbox: the chunks its readers pulled off the sockets, under a mutex, with a
condition variable the readers (and the pulse timer) signal — an idle process waits here
instead of ticking (D86). -/
structure Inbox (n : Nat) where
  mu : BaseMutex
  cv : Condvar
  items : IO.Ref (Array (Chunk n))

def Inbox.new {n : Nat} : IO (Inbox n) := do
  pure ⟨← BaseMutex.new, ← Condvar.new, ← IO.mkRef #[]⟩

def Inbox.push {n : Nat} (ib : Inbox n) (x : Chunk n) : IO Unit := do
  ib.mu.lock
  ib.items.modify (·.push x)
  ib.mu.unlock
  ib.cv.notifyOne

def Inbox.drain {n : Nat} (ib : Inbox n) : IO (Array (Chunk n)) := do
  ib.mu.lock
  let a ← ib.items.get
  ib.items.set #[]
  ib.mu.unlock
  pure a

/-- Block until a reader pushes or the timer signals (one wake-up; the caller re-checks). -/
def Inbox.waitOnce {n : Nat} (ib : Inbox n) : IO Unit := do
  ib.mu.lock
  if (← ib.items.get).isEmpty then ib.cv.wait ib.mu
  ib.mu.unlock

/-- The process's sender: the machine queues `(connection, frame buffer)` pairs under a mutex
and moves on; one writer thread per process takes them and issues `sendAll` on each connection
*without* `.block` — `uv_write` is queued on the socket in call order under the loop's lock, the
completion is awaited by a continuation. D86's writer threads each blocked in `.block` on their
write: every blocked `Task.get` is woken by *every* task completion in the process (one condition
variable in the task manager), so eighteen blocked reader/writer threads cost ~2 cores of
wake-ups per process at the leader (D94). Issuing `sendAll` from the machine thread itself was
6 % of its time (the event loop's lock); the writer thread takes that off the critical path. -/
structure Sender where
  mu : BaseMutex
  cv : Condvar
  items : IO.Ref (Array (TCP.Socket.Client × ByteArray))
  closed : IO.Ref Bool
  /-- Send errors seen by the completion continuations (reported at the end). -/
  errors : IO.Ref Nat

def Sender.new : IO Sender := do
  pure ⟨← BaseMutex.new, ← Condvar.new, ← IO.mkRef #[], ← IO.mkRef false, ← IO.mkRef 0⟩

/-- Queue this activation's frame buffers (one per peer) in one lock. -/
def Sender.push (sd : Sender) (xs : Array (TCP.Socket.Client × ByteArray)) : IO Unit := do
  if xs.isEmpty then return
  sd.mu.lock
  sd.items.modify (· ++ xs)
  sd.mu.unlock
  sd.cv.notifyOne

def Sender.close (sd : Sender) : IO Unit := do
  sd.mu.lock
  sd.closed.set true
  sd.mu.unlock
  sd.cv.notifyAll

/-- The writer: wait for buffers, take them all, queue one `sendAll` per buffer on its
connection (no blocking on completion). -/
partial def Sender.writer (sd : Sender) : IO Unit := do
  sd.mu.lock
  while (← sd.items.get).isEmpty && !(← sd.closed.get) do
    sd.cv.wait sd.mu
  let batch ← sd.items.get
  sd.items.set #[]
  let closed ← sd.closed.get
  sd.mu.unlock
  for (c, b) in batch do
    let t ← (c.sendAll #[b]).toEIO
    discard <| IO.bindTask t fun r => do
      if let .error _ := r then sd.errors.modify (· + 1)
      pure (Task.pure (.ok ()))
  if closed && batch.isEmpty then return
  sd.writer

/-- `127.0.0.1:basePort + p`. -/
def addrOf (basePort p : Nat) : Net.SocketAddress :=
  .v4 ⟨Net.IPv4Addr.ofParts 127 0 0 1, (basePort + p).toUInt16⟩

/-- Read a connection's bytes into the inbox, chunk by chunk, as they come — an `Async`
chain: each chunk's arrival resolves the receive's promise on the event loop and the
continuation (push, re-arm) runs on a task worker; no thread blocks in `recv?` (D94: a thread
blocked in `Task.get` is woken by every task completion in the process). -/
partial def reader {n : Nat} (client : TCP.Socket.Client) (inbox : Inbox n) (fm : Fin n) : Async Unit := do
  match ← client.recv? 65536 with
  | none => return
  | some bytes =>
    inbox.push (fm, bytes)
    reader client inbox fm

/-- The dialer's first byte names it (`Shell.hello`). -/
partial def recvHello (client : TCP.Socket.Client) : IO Nat := do
  match ← (client.recv? 1).block with
  | none => throw (IO.userError "sockets: connection closed before hello")
  | some b =>
    match Shell.helloOf b with
    | some p => pure p
    | none => recvHello client

/-- Accept `k` peers; each names itself and gets a reader. -/
partial def acceptLoop {n : Nat} (server : TCP.Socket.Server) (inbox : Inbox n) (k : Nat) : IO Unit := do
  if k = 0 then return
  let client ← (server.accept).block
  -- no Nagle: a hop's small message must not wait for the delayed ACK (D86)
  client.noDelay
  let fm ← recvHello client
  if h : fm < n then
    let _ ← (reader client inbox ⟨fm, h⟩).asTask
  else IO.eprintln s!"sockets: bad hello {fm}"
  acceptLoop server inbox (k - 1)

/-- Dial a peer, retrying while it is not yet listening; say who is calling. -/
partial def dial (addr : Net.SocketAddress) (hello : Nat) (tries : Nat) : IO TCP.Socket.Client := do
  let c ← TCP.Socket.Client.mk
  try
    (c.connect addr).block
    c.noDelay
    (c.send (Shell.hello hello)).block
    return c
  catch e =>
    if tries = 0 then throw e
    IO.sleep 20
    dial addr hello (tries - 1)

end Sockets
