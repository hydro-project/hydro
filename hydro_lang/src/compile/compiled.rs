use dfir_lang::graph::DfirGraph;
use slotmap::{SecondaryMap, SparseSecondaryMap};
use syn::Stmt;

use crate::location::{Location, LocationKey};
use crate::staging_util::Invariant;

pub struct CompiledFlow<'a> {
    pub(super) dfir: SecondaryMap<LocationKey, DfirGraph>,
    pub(super) extra_stmts: SparseSecondaryMap<LocationKey, Vec<Stmt>>,
    pub(super) _phantom: Invariant<'a>,
}

impl<'a> CompiledFlow<'a> {
    pub fn dfir_for(&self, location: &impl Location<'a>) -> &DfirGraph {
        self.dfir.get(Location::id(location).key()).unwrap()
    }

    pub fn all_dfir(&self) -> &SecondaryMap<LocationKey, DfirGraph> {
        &self.dfir
    }
}
