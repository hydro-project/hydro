## `#[derive(Lattice)]` Macro

A struct of multiple lattices can form a _product_ lattice. The simplest case of this is the `Pair`
lattice, which is a product of two sub-lattices. For more than two lattices, and/or to improve
readability, users can create their own product lattice structs, where each field is a sub-lattice,
and use the `#[derive(Lattice)]` macro to automatically derive the lattice traits.

Derives `Merge`, `PartialEq`, `PartialOrd`, `LatticeOrd`, `IsBot`, `IsTop`, and `LatticeFrom`,
and therefore `Lattice` too. Alternatively, individual traits can be derived:
`#[derive(Merge, LatticeOrd, IsBot, IsTop, LatticeFrom)]`. Note that `LatticeOrd` also derives
`PartialEq` and `PartialOrd`.

Note that all fields must be lattice types. If any field cannot be a lattice type then the
`where` clauses prevent the trait impl from compiling.

These derive macros will create a second set of generics to allow conversion and merging
between varying types. For example, given this struct:
```rust,ignore
#[derive(Lattice)]
struct MyLattice<KeySet, Epoch>
where
    KeySet: Collection,
    Epoch: Ord,
{
    keys: SetUnion<KeySet>,
    epoch: Max<Epoch>,
}
```
Will create derive macros `impl`s in the form:
```rust,ignore
impl<KeySet, Epoch, KeySetOther, EpochOther>
    Merge<MyLattice<KeySetOther, EpochOther>> for MyLattice<KeySet, Epoch>
where
    KeySet: Collection,
    Epoch: Ord,
    KeySetOther: Collection,
    EpochOther: Ord,
{
    // ...
}
```