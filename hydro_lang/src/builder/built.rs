use std::cell::UnsafeCell;
use std::collections::{BTreeMap, HashMap};
use std::marker::PhantomData;

use dfir_lang::graph::{DfirGraph, eliminate_extra_unions_tees, partition_graph};

use super::compiled::CompiledFlow;
use super::deploy::{DeployFlow, DeployResult};
use crate::deploy::{ClusterSpec, Deploy, ExternalSpec, IntoProcessSpec};
use crate::graph::render::HydroWriteConfig;
use crate::ir::{HydroLeaf, emit};
use crate::location::{Cluster, ExternalProcess, Process};
use crate::staging_util::Invariant;

pub struct BuiltFlow<'a> {
    pub(super) ir: Vec<HydroLeaf>,
    pub(super) process_id_name: Vec<(usize, String)>,
    pub(super) cluster_id_name: Vec<(usize, String)>,
    pub(super) external_id_name: Vec<(usize, String)>,

    pub(super) _phantom: Invariant<'a>,
}

pub(crate) fn build_inner(ir: &mut Vec<HydroLeaf>) -> BTreeMap<usize, DfirGraph> {
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
    pub fn ir(&self) -> &Vec<HydroLeaf> {
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

    pub fn optimize_with(mut self, f: impl FnOnce(&mut [HydroLeaf])) -> Self {
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
        process: &ExternalProcess<P>,
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

    /// Convert configuration options to HydroWriteConfig
    pub fn to_hydro_config(
        &self,
        show_metadata: bool,
        show_location_groups: bool,
        include_tee_ids: bool,
        use_short_labels: bool,
    ) -> HydroWriteConfig {
        HydroWriteConfig {
            show_metadata,
            show_location_groups,
            include_tee_ids,
            use_short_labels,
            process_id_name: self.process_id_name.clone(),
            cluster_id_name: self.cluster_id_name.clone(),
            external_id_name: self.external_id_name.clone(),
        }
    }

    /// Generate mermaid graph and open in browser
    pub fn generate_mermaid(
        &self,
        show_metadata: bool,
        show_location_groups: bool,
        include_tee_ids: bool,
        use_short_labels: bool,
        message_handler: Option<&dyn Fn(&str)>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let default_handler = |msg: &str| println!("{}", msg);
        let handler = message_handler.unwrap_or(&default_handler);

        let config = self.to_hydro_config(
            show_metadata,
            show_location_groups,
            include_tee_ids,
            use_short_labels,
        );

        handler("Opening Mermaid graph in browser...");
        crate::graph::debug::open_mermaid(&self.ir, Some(config))?;
        Ok(())
    }

    /// Generate DOT graph and open in browser
    pub fn generate_dot(
        &self,
        show_metadata: bool,
        show_location_groups: bool,
        include_tee_ids: bool,
        use_short_labels: bool,
        message_handler: Option<&dyn Fn(&str)>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let default_handler = |msg: &str| println!("{}", msg);
        let handler = message_handler.unwrap_or(&default_handler);

        let config = self.to_hydro_config(
            show_metadata,
            show_location_groups,
            include_tee_ids,
            use_short_labels,
        );

        handler("Opening Graphviz/DOT graph in browser...");
        crate::graph::debug::open_dot(&self.ir, Some(config))?;
        Ok(())
    }

    /// Generate ReactFlow graph and open in browser
    pub fn generate_reactflow(
        &self,
        show_metadata: bool,
        show_location_groups: bool,
        include_tee_ids: bool,
        use_short_labels: bool,
        message_handler: Option<&dyn Fn(&str)>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let default_handler = |msg: &str| println!("{}", msg);
        let handler = message_handler.unwrap_or(&default_handler);

        let config = self.to_hydro_config(
            show_metadata,
            show_location_groups,
            include_tee_ids,
            use_short_labels,
        );

        handler("Opening ReactFlow graph in browser...");
        crate::graph::debug::open_reactflow_browser(&self.ir, None, Some(config))?;
        Ok(())
    }

    /// Generate all graph types and save to files with a given prefix
    pub fn generate_all_files(
        &self,
        prefix: &str,
        show_metadata: bool,
        show_location_groups: bool,
        include_tee_ids: bool,
        use_short_labels: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let config = self.to_hydro_config(
            show_metadata,
            show_location_groups,
            include_tee_ids,
            use_short_labels,
        );

        let label_suffix = if use_short_labels { "_short" } else { "_long" };

        // Generate Mermaid
        let mermaid_content = crate::graph::render::render_hydro_ir_mermaid(&self.ir, &config);
        let mermaid_file = format!("{}{}_labels.mmd", prefix, label_suffix);
        std::fs::write(&mermaid_file, mermaid_content)?;
        println!("Generated: {}", mermaid_file);

        // Generate Graphviz
        let dot_content = crate::graph::render::render_hydro_ir_dot(&self.ir, &config);
        let dot_file = format!("{}{}_labels.dot", prefix, label_suffix);
        std::fs::write(&dot_file, dot_content)?;
        println!("Generated: {}", dot_file);

        // Generate ReactFlow
        let reactflow_content = crate::graph::render::render_hydro_ir_reactflow(&self.ir, &config);
        let reactflow_file = format!("{}{}_labels.json", prefix, label_suffix);
        std::fs::write(&reactflow_file, reactflow_content)?;
        println!("Generated: {}", reactflow_file);

        Ok(())
    }

    /// Generate graph based on GraphConfig, delegating to the appropriate method
    #[cfg(feature = "build")]
    pub fn generate_graph_with_config(
        &self,
        config: &crate::graph_util::GraphConfig,
        message_handler: Option<&dyn Fn(&str)>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(graph_type) = config.graph {
            match graph_type {
                crate::graph_util::GraphType::Mermaid => self.generate_mermaid(
                    !config.no_metadata,
                    !config.no_location_groups,
                    !config.no_tee_ids,
                    !config.long_labels, // use_short_labels is the inverse of long_labels
                    message_handler,
                ),
                crate::graph_util::GraphType::Dot => self.generate_dot(
                    !config.no_metadata,
                    !config.no_location_groups,
                    !config.no_tee_ids,
                    !config.long_labels, // use_short_labels is the inverse of long_labels
                    message_handler,
                ),
                crate::graph_util::GraphType::Reactflow => self.generate_reactflow(
                    !config.no_metadata,
                    !config.no_location_groups,
                    !config.no_tee_ids,
                    !config.long_labels, // use_short_labels is the inverse of long_labels
                    message_handler,
                ),
            }
        } else {
            Ok(())
        }
    }

    /// Generate all graph files based on GraphConfig
    #[cfg(feature = "build")]
    pub fn generate_all_files_with_config(
        &self,
        config: &crate::graph_util::GraphConfig,
        prefix: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.generate_all_files(
            prefix,
            !config.no_metadata,
            !config.no_location_groups,
            !config.no_tee_ids,
            !config.long_labels, // Inverted because flag is for long labels
        )
    }
}
