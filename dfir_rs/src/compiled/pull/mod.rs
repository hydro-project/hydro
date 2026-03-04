//! Pull-based operator helpers, i.e. [`futures::Stream`] helpers.

mod cross_singleton;
pub use cross_singleton::CrossSingleton;

mod flatten;
mod for_each;
mod into_next;
pub use into_next::IntoNext;

mod lattice_bimorphism;
#[expect(missing_docs, reason = "TODO(mingwei):")]
pub mod lattice_bimorphism_pull;
mod zip_longest;
// Re-export HalfJoinState types from dfir_pipes
pub use dfir_pipes::{
    AccumulateAll, Accumulator, Fold, FoldFrom, HalfJoinState, HalfMultisetJoinState,
    HalfSetJoinState, Reduce, SymmetricHashJoin, accumulate_all,
};
pub use flatten::Flatten;
pub use for_each::ForEach;
pub use lattice_bimorphism::LatticeBimorphismStream;
pub use zip_longest::ZipLongest;
