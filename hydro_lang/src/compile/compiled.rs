use dfir_lang::graph::{DfirGraph, PartitionError};
use slotmap::{SecondaryMap, SparseSecondaryMap};
use syn::Stmt;

use crate::location::{Location, LocationKey};
use crate::staging_util::Invariant;

pub struct CompiledFlow<'a> {
    /// The DFIR graph for each location.
    ///
    /// Each entry is `Ok(partitioned_graph)` on success, or `Err(PartitionError)` if
    /// partitioning failed (e.g. an intra-tick cycle). The error still carries the
    /// renderable flat graph and the diagnostic, so a failed location can be visualized
    /// and diagnosed rather than aborting the whole compile.
    pub(super) dfir: SecondaryMap<LocationKey, Result<DfirGraph, PartitionError>>,

    /// Extra statements to be added above the DFIR graph code, for each location.
    pub(super) extra_stmts: SparseSecondaryMap<LocationKey, Vec<Stmt>>,

    /// `Future` expressions to be run alongside the DFIR graph execution, per-location. See [`crate::telemetry::Sidecar`].
    pub(super) sidecars: SparseSecondaryMap<LocationKey, Vec<syn::Expr>>,

    pub(super) _phantom: Invariant<'a>,
}

impl<'a> CompiledFlow<'a> {
    /// Returns the DFIR graph for the given location.
    ///
    /// - `Ok(&partitioned_graph)` if partitioning succeeded.
    /// - `Err(&PartitionError)` if partitioning failed. The error still exposes the
    ///   (renderable) flat graph via [`PartitionError::flat_graph`] and the reason via
    ///   [`PartitionError::diagnostic`], so the graph can be visualized to diagnose the
    ///   failure — e.g.
    ///   ```ignore
    ///   let mermaid = match compiled.dfir_for(&process) {
    ///       Ok(graph) => graph.to_mermaid(&Default::default()),
    ///       Err(err) => err.flat_graph.mermaid_string_flat(),
    ///   };
    ///   ```
    pub fn dfir_for(&self, location: &impl Location<'a>) -> Result<&DfirGraph, &PartitionError> {
        self.dfir
            .get(Location::id(location).key())
            .unwrap()
            .as_ref()
    }

    /// Returns the DFIR graph (or [`PartitionError`]) for every location.
    pub fn all_dfir(&self) -> &SecondaryMap<LocationKey, Result<DfirGraph, PartitionError>> {
        &self.dfir
    }
}
