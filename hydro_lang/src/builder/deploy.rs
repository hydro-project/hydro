use std::collections::{BTreeMap, HashMap};
use std::io::Error;
use std::marker::PhantomData;
use std::pin::Pin;

use dfir_rs::bytes::Bytes;
use dfir_rs::futures::{Sink, Stream};
use proc_macro2::Span;
use serde::Serialize;
use serde::de::DeserializeOwned;
use stageleft::QuotedWithContext;

use super::built::build_inner;
use super::compiled::CompiledFlow;
use crate::deploy::{
    ClusterSpec, Deploy, ExternalSpec, IntoProcessSpec, LocalDeploy, Node, ProcessSpec,
    RegisterPort,
};
use crate::ir::HydroLeaf;
use crate::location::external_process::{
    ExternalBincodeSink, ExternalBincodeStream, ExternalBytesPort,
};
use crate::location::{Cluster, ExternalProcess, Location, LocationId, Process};
use crate::staging_util::Invariant;

pub struct DeployFlow<'a, D: LocalDeploy<'a>> {
    pub(super) ir: Vec<HydroLeaf>,

    /// Deployed instances of each process in the flow
    pub(super) processes: HashMap<usize, D::Process>,

    /// Lists all the processes that were created in the flow, same ID as `processes`
    /// but with the type name of the tag.
    pub(super) process_id_name: Vec<(usize, String)>,

    pub(super) externals: HashMap<usize, D::ExternalProcess>,
    pub(super) external_id_name: Vec<(usize, String)>,

    pub(super) clusters: HashMap<usize, D::Cluster>,
    pub(super) cluster_id_name: Vec<(usize, String)>,
    pub(super) used: bool,

    pub(super) _phantom: Invariant<'a, D>,
}

impl<'a, D: LocalDeploy<'a>> Drop for DeployFlow<'a, D> {
    fn drop(&mut self) {
        if !self.used {
            panic!(
                "Dropped DeployFlow without instantiating, you may have forgotten to call `compile` or `deploy`."
            );
        }
    }
}

impl<'a, D: LocalDeploy<'a>> DeployFlow<'a, D> {
    pub fn ir(&self) -> &Vec<HydroLeaf> {
        &self.ir
    }

    pub fn with_process<P>(
        mut self,
        process: &Process<P>,
        spec: impl IntoProcessSpec<'a, D>,
    ) -> Self {
        let tag_name = std::any::type_name::<P>().to_string();
        self.processes.insert(
            process.id,
            spec.into_process_spec().build(process.id, &tag_name),
        );
        self
    }

    pub fn with_remaining_processes<S: IntoProcessSpec<'a, D> + 'a>(
        mut self,
        spec: impl Fn() -> S,
    ) -> Self {
        for (id, name) in &self.process_id_name {
            self.processes
                .insert(*id, spec().into_process_spec().build(*id, name));
        }

        self
    }

    pub fn with_external<P>(
        mut self,
        process: &ExternalProcess<P>,
        spec: impl ExternalSpec<'a, D>,
    ) -> Self {
        let tag_name = std::any::type_name::<P>().to_string();
        self.externals
            .insert(process.id, spec.build(process.id, &tag_name));
        self
    }

    pub fn with_remaining_externals<S: ExternalSpec<'a, D> + 'a>(
        mut self,
        spec: impl Fn() -> S,
    ) -> Self {
        for (id, name) in &self.external_id_name {
            self.externals.insert(*id, spec().build(*id, name));
        }

        self
    }

    pub fn with_cluster<C>(mut self, cluster: &Cluster<C>, spec: impl ClusterSpec<'a, D>) -> Self {
        let tag_name = std::any::type_name::<C>().to_string();
        self.clusters
            .insert(cluster.id, spec.build(cluster.id, &tag_name));
        self
    }

    pub fn with_remaining_clusters<S: ClusterSpec<'a, D> + 'a>(
        mut self,
        spec: impl Fn() -> S,
    ) -> Self {
        for (id, name) in &self.cluster_id_name {
            self.clusters.insert(*id, spec().build(*id, name));
        }

        self
    }

    pub fn compile_no_network(mut self) -> CompiledFlow<'a, D::GraphId> {
        self.used = true;

        CompiledFlow {
            dfir: build_inner(&mut self.ir),
            extra_stmts: BTreeMap::new(),
            _phantom: PhantomData,
        }
    }
}

impl<'a, D: Deploy<'a>> DeployFlow<'a, D> {
    pub fn compile(mut self, env: &D::CompileEnv) -> CompiledFlow<'a, D::GraphId> {
        self.used = true;

        let mut seen_tees: HashMap<_, _> = HashMap::new();
        let mut seen_tee_locations: HashMap<_, _> = HashMap::new();
        self.ir.iter_mut().for_each(|leaf| {
            leaf.compile_network::<D>(
                env,
                &mut seen_tees,
                &mut seen_tee_locations,
                &self.processes,
                &self.clusters,
                &self.externals,
            );
        });

        let extra_stmts = self.extra_stmts(env);

        CompiledFlow {
            dfir: build_inner(&mut self.ir),
            extra_stmts,
            _phantom: PhantomData,
        }
    }

    fn extra_stmts(&self, env: &<D as Deploy<'a>>::CompileEnv) -> BTreeMap<usize, Vec<syn::Stmt>> {
        let mut extra_stmts: BTreeMap<usize, Vec<syn::Stmt>> = BTreeMap::new();
        for &c_id in self.clusters.keys() {
            let self_id_ident = syn::Ident::new(
                &format!("__hydro_lang_cluster_self_id_{}", c_id),
                Span::call_site(),
            );
            let self_id_expr = D::cluster_self_id(env).splice_untyped();
            extra_stmts
                .entry(c_id)
                .or_default()
                .push(syn::parse_quote! {
                    let #self_id_ident = #self_id_expr;
                });

            for other_location in self.processes.keys().chain(self.clusters.keys()) {
                let other_id_ident = syn::Ident::new(
                    &format!("__hydro_lang_cluster_ids_{}", c_id),
                    Span::call_site(),
                );
                let other_id_expr = D::cluster_ids(env, c_id).splice_untyped();
                extra_stmts
                    .entry(*other_location)
                    .or_default()
                    .push(syn::parse_quote! {
                        let #other_id_ident = #other_id_expr;
                    });
            }
        }
        extra_stmts
    }
}

impl<'a, D: Deploy<'a, CompileEnv = ()>> DeployFlow<'a, D> {
    #[must_use]
    pub fn deploy(mut self, env: &mut D::InstantiateEnv) -> DeployResult<'a, D> {
        self.used = true;

        let mut seen_tees_instantiate: HashMap<_, _> = HashMap::new();
        let mut seen_tee_locations: HashMap<_, _> = HashMap::new();
        self.ir.iter_mut().for_each(|leaf| {
            leaf.compile_network::<D>(
                &(),
                &mut seen_tees_instantiate,
                &mut seen_tee_locations,
                &self.processes,
                &self.clusters,
                &self.externals,
            );
        });

        let mut compiled = build_inner(&mut self.ir);
        let mut extra_stmts = self.extra_stmts(&());
        let mut meta = D::Meta::default();

        let (mut processes, mut clusters, mut externals) = (
            std::mem::take(&mut self.processes)
                .into_iter()
                .filter_map(|(node_id, node)| {
                    if let Some(ir) = compiled.remove(&node_id) {
                        node.instantiate(
                            env,
                            &mut meta,
                            ir,
                            extra_stmts.remove(&node_id).unwrap_or_default(),
                        );
                        Some((node_id, node))
                    } else {
                        None
                    }
                })
                .collect::<HashMap<_, _>>(),
            std::mem::take(&mut self.clusters)
                .into_iter()
                .filter_map(|(cluster_id, cluster)| {
                    if let Some(ir) = compiled.remove(&cluster_id) {
                        cluster.instantiate(
                            env,
                            &mut meta,
                            ir,
                            extra_stmts.remove(&cluster_id).unwrap_or_default(),
                        );
                        Some((cluster_id, cluster))
                    } else {
                        None
                    }
                })
                .collect::<HashMap<_, _>>(),
            std::mem::take(&mut self.externals)
                .into_iter()
                .map(|(external_id, external)| {
                    external.instantiate(
                        env,
                        &mut meta,
                        compiled.remove(&external_id).unwrap(),
                        extra_stmts.remove(&external_id).unwrap_or_default(),
                    );
                    (external_id, external)
                })
                .collect::<HashMap<_, _>>(),
        );

        for node in processes.values_mut() {
            node.update_meta(&meta);
        }

        for cluster in clusters.values_mut() {
            cluster.update_meta(&meta);
        }

        for external in externals.values_mut() {
            external.update_meta(&meta);
        }

        let mut seen_tees_connect = HashMap::new();
        self.ir.iter_mut().for_each(|leaf| {
            leaf.connect_network(&mut seen_tees_connect);
        });

        DeployResult {
            processes,
            clusters,
            externals,
            cluster_id_name: std::mem::take(&mut self.cluster_id_name)
                .into_iter()
                .collect(),
        }
    }
}

pub struct DeployResult<'a, D: Deploy<'a>> {
    processes: HashMap<usize, D::Process>,
    clusters: HashMap<usize, D::Cluster>,
    externals: HashMap<usize, D::ExternalProcess>,
    cluster_id_name: HashMap<usize, String>,
}

impl<'a, D: Deploy<'a>> DeployResult<'a, D> {
    pub fn get_process<P>(&self, p: &Process<P>) -> &D::Process {
        let id = match p.id() {
            LocationId::Process(id) => id,
            _ => panic!("Process ID expected"),
        };

        self.processes.get(&id).unwrap()
    }

    pub fn get_cluster<C>(&self, c: &Cluster<'a, C>) -> &D::Cluster {
        let id = match c.id() {
            LocationId::Cluster(id) => id,
            _ => panic!("Cluster ID expected"),
        };

        self.clusters.get(&id).unwrap()
    }

    pub fn get_all_clusters(&self) -> impl Iterator<Item = (LocationId, String, &D::Cluster)> {
        self.clusters.iter().map(|(&id, c)| {
            (
                LocationId::Cluster(id),
                self.cluster_id_name.get(&id).unwrap().clone(),
                c,
            )
        })
    }

    pub fn get_external<P>(&self, p: &ExternalProcess<P>) -> &D::ExternalProcess {
        self.externals.get(&p.id).unwrap()
    }

    pub fn raw_port(&self, port: ExternalBytesPort) -> D::ExternalRawPort {
        self.externals
            .get(&port.process_id)
            .unwrap()
            .raw_port(port.port_id)
    }

    pub async fn connect_sink_bytes(
        &self,
        port: ExternalBytesPort,
    ) -> Pin<Box<dyn Sink<Bytes, Error = Error>>> {
        self.externals
            .get(&port.process_id)
            .unwrap()
            .as_bytes_sink(port.port_id)
            .await
    }

    pub async fn connect_sink_bincode<T: Serialize + DeserializeOwned + 'static>(
        &self,
        port: ExternalBincodeSink<T>,
    ) -> Pin<Box<dyn Sink<T, Error = Error>>> {
        self.externals
            .get(&port.process_id)
            .unwrap()
            .as_bincode_sink(port.port_id)
            .await
    }

    pub async fn connect_source_bytes(
        &self,
        port: ExternalBytesPort,
    ) -> Pin<Box<dyn Stream<Item = Bytes>>> {
        self.externals
            .get(&port.process_id)
            .unwrap()
            .as_bytes_source(port.port_id)
            .await
    }

    pub async fn connect_source_bincode<T: Serialize + DeserializeOwned + 'static>(
        &self,
        port: ExternalBincodeStream<T>,
    ) -> Pin<Box<dyn Stream<Item = T>>> {
        self.externals
            .get(&port.process_id)
            .unwrap()
            .as_bincode_source(port.port_id)
            .await
    }
}
