use std::cell::UnsafeCell;
use std::collections::{BTreeMap, HashMap};
use std::marker::PhantomData;

use dfir_lang::graph::{DfirGraph, eliminate_extra_unions_tees, partition_graph};

use super::compiled::CompiledFlow;
use super::deploy::{DeployFlow, DeployResult};
use super::deploy_provider::{ClusterSpec, Deploy, ExternalSpec, IntoProcessSpec};
use super::ir::{HydroRoot, emit};
#[cfg(feature = "viz")]
use crate::graph::api::GraphApi;
use crate::location::{Cluster, External, Process};
use crate::staging_util::Invariant;

pub struct BuiltFlow<'a> {
    pub(super) ir: Vec<HydroRoot>,
    pub(super) process_id_name: Vec<(usize, String)>,
    pub(super) cluster_id_name: Vec<(usize, String)>,
    pub(super) external_id_name: Vec<(usize, String)>,

    pub(super) _phantom: Invariant<'a>,
}

pub(crate) fn build_inner(ir: &mut Vec<HydroRoot>) -> BTreeMap<usize, DfirGraph> {
    emit(ir)
        .into_iter()
        .map(|(k, v)| {
            let (mut flat_graph, _, _) = v.build();
            eliminate_extra_unions_tees(&mut flat_graph);
            let partitioned_graph =
                partition_graph(flat_graph).expect("Failed to partition (cycle detected).");
            (k, partitioned_graph)
        })
        .collect()
}

impl<'a> BuiltFlow<'a> {
    pub fn ir(&self) -> &Vec<HydroRoot> {
        &self.ir
    }

    pub fn process_id_name(&self) -> &Vec<(usize, String)> {
        &self.process_id_name
    }

    pub fn cluster_id_name(&self) -> &Vec<(usize, String)> {
        &self.cluster_id_name
    }

    pub fn external_id_name(&self) -> &Vec<(usize, String)> {
        &self.external_id_name
    }

    /// Get a GraphApi instance for this built flow
    #[cfg(feature = "viz")]
    pub fn graph_api(&self) -> GraphApi<'_> {
        GraphApi::new(
            &self.ir,
            &self.process_id_name,
            &self.cluster_id_name,
            &self.external_id_name,
        )
    }

    // String generation methods
    #[cfg(feature = "viz")]
    pub fn mermaid_string(
        &self,
        show_metadata: bool,
        show_location_groups: bool,
        use_short_labels: bool,
    ) -> String {
        self.graph_api()
            .mermaid_to_string(show_metadata, show_location_groups, use_short_labels)
    }

    #[cfg(feature = "viz")]
    pub fn dot_string(
        &self,
        show_metadata: bool,
        show_location_groups: bool,
        use_short_labels: bool,
    ) -> String {
        self.graph_api()
            .dot_to_string(show_metadata, show_location_groups, use_short_labels)
    }

    #[cfg(feature = "viz")]
    pub fn reactflow_string(
        &self,
        show_metadata: bool,
        show_location_groups: bool,
        use_short_labels: bool,
    ) -> String {
        self.graph_api()
            .reactflow_to_string(show_metadata, show_location_groups, use_short_labels)
    }

    // File generation methods
    #[cfg(feature = "viz")]
    pub fn mermaid_to_file(
        &self,
        filename: &str,
        show_metadata: bool,
        show_location_groups: bool,
        use_short_labels: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.graph_api().mermaid_to_file(
            filename,
            show_metadata,
            show_location_groups,
            use_short_labels,
        )
    }

    #[cfg(feature = "viz")]
    pub fn dot_to_file(
        &self,
        filename: &str,
        show_metadata: bool,
        show_location_groups: bool,
        use_short_labels: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.graph_api().dot_to_file(
            filename,
            show_metadata,
            show_location_groups,
            use_short_labels,
        )
    }

    #[cfg(feature = "viz")]
    pub fn reactflow_to_file(
        &self,
        filename: &str,
        show_metadata: bool,
        show_location_groups: bool,
        use_short_labels: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.graph_api().reactflow_to_file(
            filename,
            show_metadata,
            show_location_groups,
            use_short_labels,
        )
    }

    // Browser generation methods
    #[cfg(feature = "viz")]
    pub fn mermaid_to_browser(
        &self,
        show_metadata: bool,
        show_location_groups: bool,
        use_short_labels: bool,
        message_handler: Option<&dyn Fn(&str)>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.graph_api().mermaid_to_browser(
            show_metadata,
            show_location_groups,
            use_short_labels,
            message_handler,
        )
    }

    #[cfg(feature = "viz")]
    pub fn dot_to_browser(
        &self,
        show_metadata: bool,
        show_location_groups: bool,
        use_short_labels: bool,
        message_handler: Option<&dyn Fn(&str)>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.graph_api().dot_to_browser(
            show_metadata,
            show_location_groups,
            use_short_labels,
            message_handler,
        )
    }

    #[cfg(feature = "viz")]
    pub fn reactflow_to_browser(
        &self,
        show_metadata: bool,
        show_location_groups: bool,
        use_short_labels: bool,
        message_handler: Option<&dyn Fn(&str)>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.graph_api().reactflow_to_browser(
            show_metadata,
            show_location_groups,
            use_short_labels,
            message_handler,
        )
    }

    pub fn optimize_with(mut self, f: impl FnOnce(&mut [HydroRoot])) -> Self {
        f(&mut self.ir);
        BuiltFlow {
            ir: std::mem::take(&mut self.ir),
            process_id_name: std::mem::take(&mut self.process_id_name),
            cluster_id_name: std::mem::take(&mut self.cluster_id_name),
            external_id_name: std::mem::take(&mut self.external_id_name),
            _phantom: PhantomData,
        }
    }

    pub fn with_default_optimize<D: Deploy<'a>>(self) -> DeployFlow<'a, D> {
        self.optimize_with(crate::rewrites::persist_pullup::persist_pullup)
            .into_deploy()
    }

    pub fn into_deploy<D: Deploy<'a>>(mut self) -> DeployFlow<'a, D> {
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
            ir: UnsafeCell::new(std::mem::take(&mut self.ir)),
            processes,
            process_id_name: std::mem::take(&mut self.process_id_name),
            clusters,
            cluster_id_name: std::mem::take(&mut self.cluster_id_name),
            externals,
            external_id_name: std::mem::take(&mut self.external_id_name),
            _phantom: PhantomData,
        }
    }

    pub fn with_process<P, D: Deploy<'a>>(
        self,
        process: &Process<P>,
        spec: impl IntoProcessSpec<'a, D>,
    ) -> DeployFlow<'a, D> {
        self.into_deploy().with_process(process, spec)
    }

    pub fn with_remaining_processes<D: Deploy<'a>, S: IntoProcessSpec<'a, D> + 'a>(
        self,
        spec: impl Fn() -> S,
    ) -> DeployFlow<'a, D> {
        self.into_deploy().with_remaining_processes(spec)
    }

    pub fn with_external<P, D: Deploy<'a>>(
        self,
        process: &External<P>,
        spec: impl ExternalSpec<'a, D>,
    ) -> DeployFlow<'a, D> {
        self.into_deploy().with_external(process, spec)
    }

    pub fn with_remaining_externals<D: Deploy<'a>, S: ExternalSpec<'a, D> + 'a>(
        self,
        spec: impl Fn() -> S,
    ) -> DeployFlow<'a, D> {
        self.into_deploy().with_remaining_externals(spec)
    }

    pub fn with_cluster<C, D: Deploy<'a>>(
        self,
        cluster: &Cluster<C>,
        spec: impl ClusterSpec<'a, D>,
    ) -> DeployFlow<'a, D> {
        self.into_deploy().with_cluster(cluster, spec)
    }

    pub fn with_remaining_clusters<D: Deploy<'a>, S: ClusterSpec<'a, D> + 'a>(
        self,
        spec: impl Fn() -> S,
    ) -> DeployFlow<'a, D> {
        self.into_deploy().with_remaining_clusters(spec)
    }

    pub fn compile<D: Deploy<'a>>(self, env: &D::CompileEnv) -> CompiledFlow<'a, D::GraphId> {
        self.into_deploy::<D>().compile(env)
    }

    pub fn compile_no_network<D: Deploy<'a>>(self) -> CompiledFlow<'a, D::GraphId> {
        self.into_deploy::<D>().compile_no_network()
    }

    pub fn deploy<D: Deploy<'a, CompileEnv = ()>>(
        self,
        env: &mut D::InstantiateEnv,
    ) -> DeployResult<'a, D> {
        self.into_deploy::<D>().deploy(env)
    }

    #[cfg(feature = "viz")]
    pub fn generate_all_files(
        &self,
        prefix: &str,
        show_metadata: bool,
        show_location_groups: bool,
        use_short_labels: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.graph_api().generate_all_files(
            prefix,
            show_metadata,
            show_location_groups,
            use_short_labels,
        )
    }

    #[cfg(feature = "viz")]
    pub fn generate_graph_with_config(
        &self,
        config: &crate::graph::config::GraphConfig,
        message_handler: Option<&dyn Fn(&str)>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.graph_api()
            .generate_graph_with_config(config, message_handler)
    }

    #[cfg(feature = "viz")]
    pub fn generate_all_files_with_config(
        &self,
        config: &crate::graph::config::GraphConfig,
        prefix: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.graph_api()
            .generate_all_files_with_config(config, prefix)
    }
}
