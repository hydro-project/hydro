use cc_traits::Collection;
use lattices::map_union::MapUnionHashMap;
use lattices::set_union::SetUnion;
use lattices::{DomPair, IsBot, IsTop, Lattice, LatticeFrom, LatticeOrd, Max, Merge};

type Table<C> = MapUnionHashMap<RowKey, RowValEntry<C>>;
type RowKey = String;
type RowValEntry<C> = DomPair<C, RowVal>;
type RowVal = Max<String>;

/// Macro to test all the different derives on the same set of structs.
macro_rules! test_derive_all {
    ( $( $m:ident => $i:ident, )* ) => {
        $(
            #[allow(clippy::allow_attributes, dead_code, reason = "compilation test code")]
            mod $m {
                use super::*;

                #[derive($i)]
                struct Pair<LatA, LatB> {
                    pub a: LatA,
                    pub b: LatB,
                }

                #[derive($i)]
                struct MyTables<C> {
                    system: Table<C>,
                    user: Table<C>,
                }

                #[derive($i)]
                struct Empty;

                #[derive($i)]
                struct Tuple(Max<usize>);

                #[derive($i)]
                struct TupleGeneric<O>(Max<O>, Max<O>)
                where
                    O: Ord;

                #[derive($i)]
                struct MyLattice<KeySet, Epoch>
                where
                    KeySet: Collection,
                    Epoch: Ord,
                {
                    keys: SetUnion<KeySet>,
                    epoch: Max<Epoch>,
                }

                #[derive($i)]
                pub struct SimilarFields {
                    a: Max<usize>,
                    b: Max<usize>,
                    c: Max<usize>,
                }
            }
        )*
    };
}
test_derive_all! {
    // module_name => DeriveName,
    lattice => Lattice,
    merge => Merge,
    lattice_ord => LatticeOrd,
    is_top => IsTop,
    is_bot => IsBot,
    lattice_from => LatticeFrom,
}
