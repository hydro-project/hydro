# The `lattices` Crate

{{#include ../lattices/README.md:book_include}}

[`Min<T>`]: https://hydro-project.github.io/hydroflow/doc/lattices/struct.Min.html
[`Max<T>`]: https://hydro-project.github.io/hydroflow/doc/lattices/struct.Min.html
[`set_union::SetUnion`]: https://hydro-project.github.io/hydroflow/doc/lattices/set_union/struct.SetUnion.html
[`map_union::MapUnion`]: https://hydro-project.github.io/hydroflow/doc/lattices/map_union/struct.MapUnion.html
[`Bottom<Lat>`]: https://hydro-project.github.io/hydroflow/doc/lattices/struct.Bottom.html
[`Pair<LatA, LatB>`]: https://hydro-project.github.io/hydroflow/doc/lattices/struct.Pair.html
[`DomPair<LatKey, LatVal>`]: https://hydro-project.github.io/hydroflow/doc/lattices/struct.DomPair.html
[`Fake<T>`]: https://hydro-project.github.io/hydroflow/doc/lattices/struct.Fake.html
[`Merge`]: https://hydro-project.github.io/hydroflow/doc/lattices/struct.Merge.html
[`Merge::merge`]: https://hydro-project.github.io/hydroflow/doc/lattices/trait.Merge.html#tymethod.merge
[`Merge::merge_owned`]: https://hydro-project.github.io/hydroflow/doc/lattices/trait.Merge.html#method.merge_owned
[`test::check_lattice_properties`]: https://hydro-project.github.io/hydroflow/doc/lattices/test/fn.check_lattice_properties.html
[`test::check_partial_ord_properties`]: https://hydro-project.github.io/hydroflow/doc/lattices/test/fn.check_ord_properties.html
[`test::check_lattice_ord`]: https://hydro-project.github.io/hydroflow/doc/lattices/test/fn.check_lattice_ord.html
[`PartialOrd`]: https://doc.rust-lang.org/std/cmp/trait.PartialOrd.html
[`LatticeOrd<Rhs>`]: https://hydro-project.github.io/hydroflow/doc/lattices/trait.LatticeOrd.html
[`NaiveLatticeOrd`]: https://hydro-project.github.io/hydroflow/doc/lattices/trait.NaiveLatticeOrd.html
[`ConvertFrom`]: https://hydro-project.github.io/hydroflow/doc/lattices/trait.ConvertFrom.html
[`std::convert::From`]: https://doc.rust-lang.org/std/convert/trait.From.html
[`set_union::SetUnionBTreeSet`]: https://hydro-project.github.io/hydroflow/doc/lattices/set_union/type.SetUnionBTreeSet.html
[`set_union::SetUnionHashSet`]: https://hydro-project.github.io/hydroflow/doc/lattices/set_union/type.SetUnionHashSet.html

## Learn More

Take a look at [Lattice Math](./lattice_math.md) for a simple explanation of what lattices are
exactly. See the [`lattice` rustdoc](https://hydro-project.github.io/hydroflow/doc/lattices/index.html)
for more information.
