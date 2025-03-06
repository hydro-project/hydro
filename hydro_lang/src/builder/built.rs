use std::collections::{BTreeMap, HashMap};
use std::marker::PhantomData;

use dfir_lang::graph::{DfirGraph, eliminate_extra_unions_tees};

use super::compiled::CompiledFlow;
use super::deploy::{DeployFlow, DeployResult};
use crate::deploy::{ClusterSpec, Deploy, ExternalSpec, IntoProcessSpec, LocalDeploy};
use crate::ir::{HydroLeaf, emit};
use crate::location::{Cluster, ExternalProcess, Process};
use crate::staging_util::Invariant;

pub struct BuiltFlow<'a> {
    pub(super) ir: Vec<HydroLeaf>,
    pub(super) process_id_name: Vec<(usize, String)>,
    pub(super) cluster_id_name: Vec<(usize, String)>,
    pub(super) external_id_name: Vec<(usize, String)>,
    pub(super) used: bool,

    pub(super) _phantom: Invariant<'a>,
}

impl Drop for BuiltFlow<'_> {
    fn drop(&mut self) {
        if !self.used {
            panic!(
                "Dropped BuiltFlow without instantiating, you may have forgotten to call `compile` or `deploy`."
            );
        }
    }
}

pub(crate) fn build_inner(ir: &mut Vec<HydroLeaf>) -> BTreeMap<usize, DfirGraph> {
    emit(ir)
        .into_iter()
        .map(|(k, v)| {
            let (mut flat_graph, _, _) = v.build();
            eliminate_extra_unions_tees(&mut flat_graph);
            (k, flat_graph)
        })
        .collect()
}

impl<'a> BuiltFlow<'a> {
    pub fn into_ir(mut self) -> Vec<HydroLeaf> {
        self.used = true;
        std::mem::take(&mut self.ir)
    }

    pub fn ir(&self) -> &Vec<HydroLeaf> {
        &self.ir
    }

    pub fn optimize_with(mut self, f: impl FnOnce(&mut [HydroLeaf])) -> Self {
        self.used = true;
        f(&mut self.ir);
        BuiltFlow {
            ir: std::mem::take(&mut self.ir),
            process_id_name: std::mem::take(&mut self.process_id_name),
            cluster_id_name: std::mem::take(&mut self.cluster_id_name),
            external_id_name: std::mem::take(&mut self.external_id_name),
            used: false,
            _phantom: PhantomData,
        }
    }

    pub fn with_default_optimize<D: LocalDeploy<'a>>(self) -> DeployFlow<'a, D> {
        self.optimize_with(crate::rewrites::persist_pullup::persist_pullup)
            .into_deploy()
    }

    pub fn into_deploy<D: LocalDeploy<'a>>(mut self) -> DeployFlow<'a, D> {
        self.used = true;
        let processes = if D::has_trivial_node() {
            self.process_id_name
                .iter()
                .map(|id| (id.0, D::trivial_process(id.0)))
                .collect()
        } else {
            HashMap::new()
        };

        let clusters = if D::has_trivial_node() {
            self.cluster_id_name
                .iter()
                .map(|id| (id.0, D::trivial_cluster(id.0)))
                .collect()
        } else {
            HashMap::new()
        };

        let externals = if D::has_trivial_node() {
            self.external_id_name
                .iter()
                .map(|id| (id.0, D::trivial_external(id.0)))
                .collect()
        } else {
            HashMap::new()
        };

        DeployFlow {
            ir: std::mem::take(&mut self.ir),
            processes,
            process_id_name: std::mem::take(&mut self.process_id_name),
            clusters,
            cluster_id_name: std::mem::take(&mut self.cluster_id_name),
            externals,
            external_id_name: std::mem::take(&mut self.external_id_name),
            used: false,
            _phantom: PhantomData,
        }
    }

    pub fn with_process<P, D: LocalDeploy<'a>>(
        self,
        process: &Process<P>,
        spec: impl IntoProcessSpec<'a, D>,
    ) -> DeployFlow<'a, D> {
        self.into_deploy().with_process(process, spec)
    }

    pub fn with_remaining_processes<D: LocalDeploy<'a>, S: IntoProcessSpec<'a, D> + 'a>(
        self,
        spec: impl Fn() -> S,
    ) -> DeployFlow<'a, D> {
        self.into_deploy().with_remaining_processes(spec)
    }

    pub fn with_external<P, D: LocalDeploy<'a>>(
        self,
        process: &ExternalProcess<P>,
        spec: impl ExternalSpec<'a, D>,
    ) -> DeployFlow<'a, D> {
        self.into_deploy().with_external(process, spec)
    }

    pub fn with_remaining_externals<D: LocalDeploy<'a>, S: ExternalSpec<'a, D> + 'a>(
        self,
        spec: impl Fn() -> S,
    ) -> DeployFlow<'a, D> {
        self.into_deploy().with_remaining_externals(spec)
    }

    pub fn with_cluster<C, D: LocalDeploy<'a>>(
        self,
        cluster: &Cluster<C>,
        spec: impl ClusterSpec<'a, D>,
    ) -> DeployFlow<'a, D> {
        self.into_deploy().with_cluster(cluster, spec)
    }

    pub fn with_remaining_clusters<D: LocalDeploy<'a>, S: ClusterSpec<'a, D> + 'a>(
        self,
        spec: impl Fn() -> S,
    ) -> DeployFlow<'a, D> {
        self.into_deploy().with_remaining_clusters(spec)
    }

    pub fn compile<D: Deploy<'a>>(self, env: &D::CompileEnv) -> CompiledFlow<'a, D::GraphId> {
        self.into_deploy::<D>().compile(env)
    }

    pub fn compile_no_network<D: LocalDeploy<'a>>(self) -> CompiledFlow<'a, D::GraphId> {
        self.into_deploy::<D>().compile_no_network()
    }

    pub fn deploy<D: Deploy<'a, CompileEnv = ()>>(
        self,
        env: &mut D::InstantiateEnv,
    ) -> DeployResult<'a, D> {
        self.into_deploy::<D>().deploy(env)
    }
}
