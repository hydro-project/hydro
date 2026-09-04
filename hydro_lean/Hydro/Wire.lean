import Lean.Data.Json

/-!
# Hydro · the wire codec

JSON codecs for what crosses between processes. Lean's `Json` instances
cover the containers; `Fin`/`Except` are supplied here, and the Rust
message structs (`Ballot`, `P1b`, …) derive theirs in `Paxos/Types.lean` —
the mirror of their `#[derive(Serialize, Deserialize)]`.
-/

namespace Hydro

open Lean

instance {n : Nat} : ToJson (Fin n) := ⟨fun i => toJson i.val⟩

instance {n : Nat} : FromJson (Fin n) := ⟨fun j => do
  let v : Nat ← fromJson? j
  if h : v < n then pure ⟨v, h⟩ else throw s!"Fin {n}: {v} out of range"⟩

instance {ε α : Type} [ToJson ε] [ToJson α] : ToJson (Except ε α) := ⟨fun
  | .ok a => Json.mkObj [("ok", toJson a)]
  | .error e => Json.mkObj [("error", toJson e)]⟩

instance {ε α : Type} [FromJson ε] [FromJson α] : FromJson (Except ε α) := ⟨fun j => do
  match j.getObjVal? "ok" with
  | .ok a => return .ok (← fromJson? a)
  | .error _ => return .error (← fromJson? (← j.getObjVal? "error"))⟩

/-- A payload codec: what a network node's items look like on the wire. -/
structure Codec (α : Type) where
  enc : α → Json
  dec : Json → Except String α

def Codec.ofJson {α : Type} [ToJson α] [FromJson α] : Codec α := ⟨toJson, fromJson?⟩

end Hydro
