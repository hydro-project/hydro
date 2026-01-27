use std::marker::PhantomData;

use dfir_lang::graph::{DfirGraph, eliminate_extra_unions_tees, partition_graph};
use slotmap::{SecondaryMap, SlotMap, SparseSecondaryMap};

use super::compiled::CompiledFlow;
use super::deploy::{DeployFlow, DeployResult};
use super::deploy_provider::{ClusterSpec, Deploy, ExternalSpec, IntoProcessSpec};
use super::ir::{HydroRoot, emit};
use crate::location::{Cluster, External, LocationKey, LocationType, Process};
#[cfg(stageleft_runtime)]
#[cfg(feature = "sim")]
use crate::sim::{flow::SimFlow, graph::SimNode};
use crate::staging_util::Invariant;
#[cfg(stageleft_runtime)]
#[cfg(feature = "viz")]
use crate::viz::api::GraphApi;

pub struct BuiltFlow<'a> {
    pub(super) ir: Vec<HydroRoot>,
    pub(super) locations: SlotMap<LocationKey, LocationType>,
    pub(super) location_names: SecondaryMap<LocationKey, String>,

    /// Application name used in telemetry.
    pub(super) flow_name: String,

    pub(super) _phantom: Invariant<'a>,
}

pub(crate) fn build_inner<'a, D: Deploy<'a>>(
    ir: &mut Vec<HydroRoot>,
) -> SecondaryMap<LocationKey, DfirGraph> {
    emit::<D>(ir)
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
    /// Returns all [`HydroRoot`]s in the IR.
    pub fn ir(&self) -> &[HydroRoot] {
        &self.ir
    }

    /// Returns all raw location ID -> location name mappings.
    pub fn location_names(&self) -> &SecondaryMap<LocationKey, String> {
        &self.location_names
    }

    /// Get a GraphApi instance for this built flow
    #[cfg(stageleft_runtime)]
    #[cfg(feature = "viz")]
    pub fn graph_api(&self) -> GraphApi<'_> {
        GraphApi::new(&self.ir, self.location_names())
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
    pub fn hydroscope_string(
        &self,
        show_metadata: bool,
        show_location_groups: bool,
        use_short_labels: bool,
    ) -> String {
        self.graph_api()
            .hydroscope_to_string(show_metadata, show_location_groups, use_short_labels)
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
    pub fn hydroscope_to_file(
        &self,
        filename: &str,
        show_metadata: bool,
        show_location_groups: bool,
        use_short_labels: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.graph_api().hydroscope_to_file(
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
    pub fn hydroscope_to_browser(
        &self,
        show_metadata: bool,
        show_location_groups: bool,
        use_short_labels: bool,
        message_handler: Option<&dyn Fn(&str)>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.graph_api().hydroscope_to_browser(
            show_metadata,
            show_location_groups,
            use_short_labels,
            message_handler,
        )
    }

    pub fn optimize_with(mut self, f: impl FnOnce(&mut [HydroRoot])) -> Self {
        f(&mut self.ir);
        self
    }

    pub fn with_default_optimize<D: Deploy<'a>>(self) -> DeployFlow<'a, D> {
        self.into_deploy()
    }

    #[cfg(feature = "sim")]
    /// Creates a simulation for this builder, which can be used to run deterministic simulations
    /// of the Hydro program.
    pub fn sim(mut self) -> SimFlow<'a> {
        use std::cell::RefCell;
        use std::rc::Rc;

        use slotmap::SparseSecondaryMap;

        use crate::sim::graph::SimNodePort;

        let shared_port_counter = Rc::new(RefCell::new(SimNodePort::default()));

        let mut processes = SparseSecondaryMap::new();
        let mut clusters = SparseSecondaryMap::new();
        let externals = SparseSecondaryMap::new();

        for (key, loc) in self.locations.iter() {
            match loc {
                LocationType::Process => {
                    processes.insert(
                        key,
                        SimNode {
                            shared_port_counter: shared_port_counter.clone(),
                        },
                    );
                }
                LocationType::Cluster => {
                    clusters.insert(
                        key,
                        SimNode {
                            shared_port_counter: shared_port_counter.clone(),
                        },
                    );
                }
                LocationType::External => {
                    panic!("Sim cannot have externals");
                }
            }
        }

        SimFlow {
            ir: std::mem::take(&mut self.ir),
            processes,
            clusters,
            externals,
            cluster_max_sizes: SparseSecondaryMap::new(),
            externals_port_registry: Default::default(),
            _phantom: PhantomData,
        }
    }

    pub fn into_deploy<D: Deploy<'a>>(self) -> DeployFlow<'a, D> {
        let (processes, clusters, externals) = Default::default();
        DeployFlow {
            ir: self.ir,
            locations: self.locations,
            location_names: self.location_names,
            processes,
            clusters,
            externals,
            sidecars: SparseSecondaryMap::new(),
            flow_name: self.flow_name,
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

    pub fn compile<D: Deploy<'a>>(self) -> CompiledFlow<'a> {
        self.into_deploy::<D>().compile()
    }

    pub fn deploy<D: Deploy<'a>>(self, env: &mut D::InstantiateEnv) -> DeployResult<'a, D> {
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
        config: &crate::viz::config::GraphConfig,
        message_handler: Option<&dyn Fn(&str)>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.graph_api()
            .generate_graph_with_config(config, message_handler)
    }

    #[cfg(feature = "viz")]
    pub fn generate_all_files_with_config(
        &self,
        config: &crate::viz::config::GraphConfig,
        prefix: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.graph_api()
            .generate_all_files_with_config(config, prefix)
    }
}
