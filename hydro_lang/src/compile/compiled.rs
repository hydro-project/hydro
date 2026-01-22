use std::collections::BTreeMap;

use dfir_lang::graph::DfirGraph;
use syn::Stmt;

use crate::location::Location;
use crate::staging_util::Invariant;

pub struct CompiledFlow<'a> {
    pub(super) dfir: BTreeMap<usize, DfirGraph>,
    pub(super) extra_stmts: BTreeMap<usize, Vec<Stmt>>,
    pub(super) _phantom: Invariant<'a>,
}

impl<'a> CompiledFlow<'a> {
    pub fn dfir_for(&self, location: &impl Location<'a>) -> &DfirGraph {
        self.dfir.get(&Location::id(location).raw_id()).unwrap()
    }

    pub fn all_dfir(&self) -> &BTreeMap<usize, DfirGraph> {
        &self.dfir
    }
}
