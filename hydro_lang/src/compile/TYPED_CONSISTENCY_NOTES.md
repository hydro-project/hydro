# Typed Consistency: Design Notes and Rust Limitations

## What we built

Added a `Con` type parameter to `Stream<Type, Loc, Bound, Order, Retry, Con>` with
five marker types: `Seq`, `Conv`, `SelfCon`, `Incon`, `UnknownCon`.

- `source_iter` returns `Seq` (bounded deterministic source)
- `map`, `filter`, `scan`, etc. preserve `C` (generic passthrough)
- `chain` and `join` compute `MinConsistency<C1, C2>`
- `From` impls for Bounded→Unbounded, TotalOrder→NoOrder preserve `C`
- Full `hydro_lang` crate compiles with `source_iter` returning `Seq`

## The Rust limitation

Rust's method resolution with default type parameters breaks when the
parameter is explicitly provided as a generic. Specifically:

```rust
pub struct Stream<..., Con = UnknownCon> { ... }

impl<..., C> Stream<..., C> {
    pub fn some_method(self) -> ... { ... }
}
```

When calling `some_method()` on a `Stream<..., Seq>` (concrete type), it works.
When calling it on a `Stream<..., C>` (generic type parameter), the compiler
says "method not found" even though the impl block accepts any `C`.

This affects ALL methods on Stream — not just consistency-related ones.
The compiler reports "method was found for `Stream<T, L, B, O, R>`" (5 params,
defaulting C) but can't find it for `Stream<T, L, B, O, R, C>` (6 params,
explicit C).

This is a known Rust limitation with default type parameters on structs.
It affects method resolution when:
1. The struct has a default type parameter
2. An impl block is generic over that parameter
3. The method is called from a context where the parameter is a generic

## What works despite the limitation

- `source_iter` returning `Seq` compiles
- `map(f)` on a `Seq` stream returns `Seq` — type inference works
- `chain(Seq, Seq)` returns `Seq` via `MinConsistency`
- Explicit type annotations verify the correct type:
  `let _s: Stream<i32, _, _, _, _, Seq> = p.source_iter(q!([1,2,3]));`

## What doesn't work

- Calling methods defined in OTHER impl blocks (networking, keyed, etc.)
  on a stream with explicit `C` fails method resolution
- `into_keyed()`, `demux()`, `round_robin()`, `broadcast()` all fail
- This makes the parameter unusable in practice for multi-location programs

## Possible paths forward

1. **Wrapper types**: `SeqStream<T, L, B, O, R>` as a newtype around
   `Stream<T, L, B, O, R, Seq>`. Methods on `SeqStream` delegate to `Stream`.
   Avoids the default parameter issue but doubles the API surface.

2. **Associated type on Location**: Instead of a parameter on Stream, make
   consistency a property of the Location. `Cluster<'a, L, Seq>` would mean
   "this cluster provides sequential consistency." Streams on that location
   inherit the consistency. Fewer type parameters, but less flexible.

3. **Trait-based approach**: `trait HasConsistency { type Con; }` implemented
   for Stream, with methods that return `Self::Con`. IDE shows the associated
   type on hover. No extra type parameter needed.

4. **Wait for Rust improvements**: The default type parameter limitation may
   be fixed in future Rust versions. Track rust-lang/rust#27336.

5. **Keep the walk-based analysis**: The existing `coordination.rs` analysis
   works correctly and produces the right labels. Surface results via
   `#[deprecated]` warnings or a custom rust-analyzer extension.
