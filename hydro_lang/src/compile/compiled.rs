use dfir_lang::graph::DfirGraph;
use slotmap::{SecondaryMap, SparseSecondaryMap};
use syn::Stmt;

use crate::location::{Location, LocationKey};
use crate::staging_util::Invariant;

pub struct CompiledFlow<'a> {
    /// The DFIR graph for each location.
    pub(super) dfir: SecondaryMap<LocationKey, DfirGraph>,

    /// Extra statements to be added above the DFIR graph code, for each location.
    pub(super) extra_stmts: SparseSecondaryMap<LocationKey, Vec<Stmt>>,

    /// `Future` expressions to be run alongside the DFIR graph execution, per-location. See [`crate::telemetry::Sidecar`].
    pub(super) sidecars: SparseSecondaryMap<LocationKey, Vec<syn::Expr>>,

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
