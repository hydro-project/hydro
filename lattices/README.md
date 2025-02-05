<h1 class="crate-title">The <code>lattices</code> Crate</h1>

The `lattices` crate provides ergonomic and composable lattice types. You can also implement custom
lattices via a few simple traits.

Lattices are an incredibly powerful mathematical concept which can greatly simplify the trickiness
of distributed computing. They align very well with the reality of what happens physically in a
distributed system: messages can always arrive out-of-order or duplicated. But if that data is
represented as lattices then all machines will always reach the same end result simply by merging the data together.
One popular way that lattices are currently used in distributed systems is as the data underlying
[Conflict-free Replicated Data Types](https://en.wikipedia.org/wiki/Conflict-free_replicated_data_type)
(CRDTs).

Lattices also allow us to harness the power of the CALM Theorem: "a program has a consistent,
coordination-free distributed implementation if and only if it is monotonic." Lattice state is
always monotonic, meaning any part of a distributed system built on lattice state can be
freely distributed with no coordination overhead. The goal of the [Hydro Project](https://hydro.run/)
is to allow users to write programs that automatically scale and distribute effortlessly.

For more information on the underlying mathematics of lattices and monotonicity, take a look at
[Lattice Math section of the Hydroflow Book](https://hydro.run/docs/dfir/lattices_crate/lattice_math)
and Section 2 of the [Hydroflow Thesis (2021)](https://hydro.run/papers/hydroflow-thesis.pdf).

Take a look at the [`lattice` rustdocs](https://hydro.run/rustdoc/lattices/index.html).

## Lattices

`lattices` provides implementations of common lattice types:
* [`Min<T>`] and [`Max<T>`] - totally-ordered lattices.
* [`set_union::SetUnion<T>`] - set-union lattice of scalar values.
* [`map_union::MapUnion<K, Lat>`] - scalar keys with nested lattice values.
* [`union_find::UnionFind<K>`] - union partitions of a set of scalar values.
* [`VecUnion<Lat>`] - growing `Vec` of nested lattices, like `MapUnion<<usize, Lat>>` but without missing entries.
* [`WithBot<Lat>`] - wraps a lattice in `Option` with `None` as the new bottom value.
* [`WithTop<Lat>`] - wraps a lattice in `Option` with `None` as the new _top_ value.
* [`Pair<LatA, LatB>`] - product of two nested lattices.
* [`DomPair<LatKey, LatVal>`]* - a versioned pair where the `LatKey` dominates the `LatVal`.
* [`Conflict<T>`]* - adds a "conflict" top to domain `T`. Merging inequal `T`s results in top.
* [`Point<T, *>`]* - a single "point lattice" value which cannot be merged with any inequal value.
* [`()`](https://doc.rust-lang.org/std/primitive.unit.html) - the "unit" lattice, a "point lattice" with unit `()` as the only value in the domain.

*Special implementations which do not obey all lattice properties but are still useful under
certain circumstances.

Additionally, custom lattices can be made by implementing the traits below.

## Traits

A type becomes a lattice by implementing one or more traits starting with `Merge`. These traits
are already implemented for all the provided lattice types.

### `Merge`

The main trait is [`Merge`], which defines a lattice merge function (AKA "join" or "least upper
bound"). Implementors must define the [`Merge::merge`] method which does a merge in-place into
`&mut self`. The method must return `true` if `self` was modified (i.e. the value got larger in the
lattice partial order) and `false` otherwise (i.e. `other` was smaller than `self`). The [`Merge::merge_owned`]
function, which merges two owned values, is provided.

The `merge` method must be associative, commutative, and idempotent. This is not checked by the
compiler, but the implementor can use the [`test::check_lattice_properties`] method to spot-check
these properties on a collection of values.

### `PartialOrd`, `LatticeOrd`, and `NaiveLatticeOrd`

Rust already has a trait for partial orders, [`PartialOrd`], which should be implemented on lattice
types. However that trait is not specific to lattice partial orders, therefore we provide the[`LatticeOrd<Rhs>`]`: PartialOrd<Rhs>`
marker trait to denote when a `PartialOrd` implementation is a lattice partial order. `LatticeOrd`
must always agree with the `Merge` function.

Additionally, the sealed [`NaiveLatticeOrd`] trait is provided on all lattice types that implement
`Merge` and `Clone`. This trait provides a `naive_cmp` method which derives a lattice order from
the `Merge` function directly. However the implementation is generally slow and inefficient.

Implementors should use the [`test::check_partial_ord_properties`] method to check their
`PartialOrd` implementation, and should use the [`test::check_lattice_ord`] to ensure the partial
order agrees with the `Merge`-derived `NaiveLatticeOrd` order.

### `LatticeFrom`

[`LatticeFrom`] is equivalent to the [`std::convert::From`] trait but specific to lattices.
`LatticeFrom` should be implemented only between different representations of the same lattice
type, e.g. between [`set_union::SetUnionBTreeSet`] and [`set_union::SetUnionHashSet`]. For compound
lattice (lattices with nested lattice types), the `LatticeFrom` implementation should be recursive
for those nested lattices.

### `IsBot`, `IsTop`, and `Default`

A bottom (⊥) is strictly less than all other values. A top (⊤) is strictly greater than all other
values. `IsBot::is_bot` and `IsTop::is_top` determine if a lattice instance is top or bottom
respectively.

For lattice types, `Default::default()` must create a bottom value. `IsBot::is_bot(&Default::default())`
should always return true for all lattice types.

### `Atomize`

[`Atomize::atomize`] converts a lattice point into a bunch of smaller lattice points. When these
"atoms" are merged together they will form the original lattice point. See the docs for more
precise semantics.

### `DeepReveal`

[`DeepReveal`] allows recursive "revealing" of the underlying data within latties. Particularly
useful for revealing nested lattices.
