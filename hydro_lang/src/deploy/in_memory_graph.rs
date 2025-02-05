use dfir_lang::graph::DfirGraph;

use super::{LocalDeploy, Node, ProcessSpec};

pub struct SingleProcessGraph {}

impl LocalDeploy<'_> for SingleProcessGraph {
    type Process = SingleNode;
    type Cluster = SingleNode;
    type ExternalProcess = SingleNode;
    type Meta = ();
    type GraphId = ();

    fn has_trivial_node() -> bool {
        true
    }

    fn trivial_process(_id: usize) -> Self::Process {
        SingleNode {}
    }

    fn trivial_cluster(_id: usize) -> Self::Cluster {
        SingleNode {}
    }

    fn trivial_external(_id: usize) -> Self::ExternalProcess {
        SingleNode {}
    }
}

impl ProcessSpec<'_, SingleProcessGraph> for () {
    fn build(self, _id: usize, _name_hint: &str) -> SingleNode {
        SingleNode {}
    }
}

#[derive(Clone)]
pub struct SingleNode {}

impl Node for SingleNode {
    type Port = ();
    type Meta = ();
    type InstantiateEnv = ();

    fn next_port(&self) {
        panic!();
    }

    fn update_meta(&mut self, _meta: &Self::Meta) {}

    fn instantiate(
        &self,
        _env: &mut Self::InstantiateEnv,
        _meta: &mut Self::Meta,
        _graph: DfirGraph,
        _extra_stmts: Vec<syn::Stmt>,
    ) {
    }
}

pub struct MultiGraph {}

impl LocalDeploy<'_> for MultiGraph {
    type Process = MultiNode;
    type Cluster = MultiNode;
    type ExternalProcess = MultiNode;
    type Meta = ();
    type GraphId = usize;

    fn has_trivial_node() -> bool {
        true
    }

    fn trivial_process(_id: usize) -> Self::Process {
        MultiNode {}
    }

    fn trivial_cluster(_id: usize) -> Self::Cluster {
        MultiNode {}
    }

    fn trivial_external(_id: usize) -> Self::ExternalProcess {
        MultiNode {}
    }
}

impl ProcessSpec<'_, MultiGraph> for () {
    fn build(self, _id: usize, _name_hint: &str) -> MultiNode {
        MultiNode {}
    }
}

#[derive(Clone)]
pub struct MultiNode {}

impl Node for MultiNode {
    type Port = ();
    type Meta = ();
    type InstantiateEnv = ();

    fn next_port(&self) {
        panic!();
    }

    fn update_meta(&mut self, _meta: &Self::Meta) {}

    fn instantiate(
        &self,
        _env: &mut Self::InstantiateEnv,
        _meta: &mut Self::Meta,
        _graph: DfirGraph,
        _extra_stmts: Vec<syn::Stmt>,
    ) {
    }
}
