import HydroLean.Flo.Collection

/-!
# Hydro stream markers (dissertation §4.3.2)

Rust Hydro tracks sources of distributed nondeterminism in *stream markers*:
type parameters on `Stream<T, L, B, O, R>` that record whether the element
order is deterministic (`TotalOrder` / `NoOrder`) and whether the cardinality
is deterministic (`ExactlyOnce` / `AtLeastOnce`). Together with the
boundedness marker `B` (from Flo, §2.3.3 — we reuse `HydroLean.Boundedness`),
they determine which operations are safe: e.g. `fold` demands
`TotalOrder + ExactlyOnce`, while `fold_commutative` tolerates `NoOrder`.

In the Lean embedding the markers select the **carrier** a stream denotes,
realizing DESIGN.md's quotient table:

| ordering     | retries       | carrier                                        |
|--------------|---------------|------------------------------------------------|
| `totalOrder` | `exactlyOnce` | `List α`                                       |
| `noOrder`    | `exactlyOnce` | `Multiset α` (quotient of `List` by `Perm`)    |
| `totalOrder` | `atLeastOnce` | `DupList α` (adjacent-duplicate collapse)      |
| `noOrder`    | `atLeastOnce` | support quotient (presence with unknown count) |

This is the precise sense in which "the type system defers materializing
nondeterminism": a value of the `noOrder` carrier *has no order to observe*,
so network reordering is not resolved but unrepresentable, and only explicit
`assume_*`/`NonDet`-guarded operations (see `Hydro/NonDet.lean`) pick concrete
representatives.

In wave 1 only the `totalOrder + exactlyOnce` carrier (`List`) is
instantiated; the marker types are defined now so signatures are stable when
the `Multiset`/`DupList` carriers (built concurrently in
`HydroLean/Collections/`) are wired in.
-/

namespace HydroLean.Hydro

/-- Ordering marker (Rust: `hydro_lang::live_collections::stream::Ordering`,
inhabited by `TotalOrder` and `NoOrder`, §4.3.2). -/
inductive StreamOrder where
  /-- Rust `TotalOrder`: elements have a deterministic total order. -/
  | totalOrder
  /-- Rust `NoOrder`: elements may be arbitrarily shuffled (e.g. after
  many-to-one cluster networking, §4.4.2); the denotation is order-free. -/
  | noOrder
deriving DecidableEq, Repr

/-- Retries marker (Rust: `hydro_lang::live_collections::stream::Retries`,
inhabited by `ExactlyOnce` and `AtLeastOnce`, §4.3.2). -/
inductive Retries where
  /-- Rust `ExactlyOnce`: deterministic cardinality. -/
  | exactlyOnce
  /-- Rust `AtLeastOnce`: nondeterministic duplication (e.g. retry protocols,
  §3.5.2); the denotation collapses adjacent duplicates. -/
  | atLeastOnce
deriving DecidableEq, Repr

/-- Marker subtyping (§4.3.2): a stream with stronger guarantees can be used
where weaker guarantees are expected (`TotalOrder ≤ NoOrder` in the sense of
"can be weakened to"). Rust exposes this via `Into` conversions; in Lean the
weakenings are the quotient maps between carriers (e.g. `List α → Multiset α`),
which are *safe* — the reverse direction is exactly what `assume_ordered` /
`assume_exactly_once` guard with `NonDet`. -/
def StreamOrder.weakensTo : StreamOrder → StreamOrder → Prop
  | _, .noOrder => True
  | .totalOrder, .totalOrder => True
  | .noOrder, .totalOrder => False

/-- See `StreamOrder.weakensTo`. -/
def Retries.weakensTo : Retries → Retries → Prop
  | _, .atLeastOnce => True
  | .exactlyOnce, .exactlyOnce => True
  | .atLeastOnce, .exactlyOnce => False

end HydroLean.Hydro
