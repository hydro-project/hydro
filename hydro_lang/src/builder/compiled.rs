use std::collections::BTreeMap;

use dfir_lang::graph::DfirGraph;

use crate::location::Location;
use crate::staging_util::Invariant;

pub struct CompiledFlow<'a, ID> {
    pub(super) dfir: BTreeMap<usize, DfirGraph>,
    pub(super) _phantom: Invariant<'a, ID>,
}

impl<'a, ID> CompiledFlow<'a, ID> {
    pub fn dfir_for(&self, location: &impl Location<'a>) -> &DfirGraph {
        self.dfir.get(&Location::id(location).raw_id()).unwrap()
    }

    pub fn all_dfir(&self) -> &BTreeMap<usize, DfirGraph> {
        &self.dfir
    }
}
