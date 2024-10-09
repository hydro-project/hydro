use std::collections::{BTreeMap, HashMap};
use std::marker::PhantomData;

use hydroflow_lang::graph::{eliminate_extra_unions_tees, HydroflowGraph};

use super::deploy::{DeployFlow, DeployResult};
use crate::deploy::{ClusterSpec, Deploy, ExternalSpec, LocalDeploy, ProcessSpec};
use crate::ir::HfPlusLeaf;
use crate::location::{Cluster, ExternalProcess, Process};
use crate::HfCompiled;

pub struct BuiltFlow<'a> {
    pub(super) ir: Vec<HfPlusLeaf>,
    pub(super) processes: Vec<usize>,
    pub(super) clusters: Vec<usize>,
    pub(super) used: bool,

    pub(super) _phantom: PhantomData<&'a mut &'a ()>,
}

impl<'a> Drop for BuiltFlow<'a> {
    fn drop(&mut self) {
        if !self.used {
            panic!("Dropped BuiltFlow without instantiating, you may have forgotten to call `compile` or `deploy`.");
        }
    }
}

impl<'a> BuiltFlow<'a> {
    pub fn ir(&self) -> &Vec<HfPlusLeaf> {
        &self.ir
    }

    pub fn optimize_with(
        mut self,
        f: impl FnOnce(Vec<HfPlusLeaf>) -> Vec<HfPlusLeaf>,
    ) -> BuiltFlow<'a> {
        self.used = true;
        BuiltFlow {
            ir: f(std::mem::take(&mut self.ir)),
            processes: std::mem::take(&mut self.processes),
            clusters: std::mem::take(&mut self.clusters),
            used: false,
            _phantom: PhantomData,
        }
    }
}

pub(crate) fn build_inner(ir: &mut Vec<HfPlusLeaf>) -> BTreeMap<usize, HydroflowGraph> {
    let mut builders = BTreeMap::new();
    let mut built_tees = HashMap::new();
    let mut next_stmt_id = 0;
    for leaf in ir {
        leaf.emit(&mut builders, &mut built_tees, &mut next_stmt_id);
    }

    builders
        .into_iter()
        .map(|(k, v)| {
            let (mut flat_graph, _, _) = v.build();
            eliminate_extra_unions_tees(&mut flat_graph);
            (k, flat_graph)
        })
        .collect()
}

impl<'a> BuiltFlow<'a> {
    pub fn compile_no_network<D: LocalDeploy<'a>>(mut self) -> HfCompiled<'a, D::GraphId> {
        self.used = true;

        HfCompiled {
            hydroflow_ir: build_inner(&mut self.ir),
            extra_stmts: BTreeMap::new(),
            _phantom: PhantomData,
        }
    }

    pub fn with_default_optimize(self) -> BuiltFlow<'a> {
        self.optimize_with(crate::persist_pullup::persist_pullup)
    }

    fn into_deploy<D: LocalDeploy<'a>>(mut self) -> DeployFlow<'a, D> {
        self.used = true;
        let processes = if D::has_trivial_node() {
            self.processes
                .iter()
                .map(|id| (*id, D::trivial_process(*id)))
                .collect()
        } else {
            HashMap::new()
        };

        let clusters = if D::has_trivial_node() {
            self.clusters
                .iter()
                .map(|id| (*id, D::trivial_cluster(*id)))
                .collect()
        } else {
            HashMap::new()
        };

        DeployFlow {
            ir: std::mem::take(&mut self.ir),
            nodes: processes,
            clusters,
            externals: HashMap::new(),
            used: false,
            _phantom: PhantomData,
        }
    }

    pub fn with_process<P, D: LocalDeploy<'a>>(
        self,
        process: &Process<P>,
        spec: impl ProcessSpec<'a, D>,
    ) -> DeployFlow<'a, D> {
        self.into_deploy().with_process(process, spec)
    }

    pub fn with_external<P, D: LocalDeploy<'a>>(
        self,
        process: &ExternalProcess<P>,
        spec: impl ExternalSpec<'a, D>,
    ) -> DeployFlow<'a, D> {
        self.into_deploy().with_external(process, spec)
    }

    pub fn with_cluster<C, D: LocalDeploy<'a>>(
        self,
        cluster: &Cluster<C>,
        spec: impl ClusterSpec<'a, D>,
    ) -> DeployFlow<'a, D> {
        self.into_deploy().with_cluster(cluster, spec)
    }

    pub fn compile<D: Deploy<'a> + 'a>(self, env: &D::CompileEnv) -> HfCompiled<'a, D::GraphId> {
        self.into_deploy::<D>().compile(env)
    }

    pub fn deploy<D: Deploy<'a, CompileEnv = ()> + 'a>(
        self,
        env: &mut D::InstantiateEnv,
    ) -> DeployResult<'a, D> {
        self.into_deploy::<D>().deploy(env)
    }
}
