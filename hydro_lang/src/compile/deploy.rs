use std::collections::HashMap;
use std::io::Error;
use std::marker::PhantomData;
use std::pin::Pin;

use bytes::{Bytes, BytesMut};
use futures::{Sink, Stream};
use proc_macro2::Span;
use serde::Serialize;
use serde::de::DeserializeOwned;
use slotmap::{SecondaryMap, SlotMap, SparseSecondaryMap};
use stageleft::QuotedWithContext;

use super::built::build_inner;
use super::compiled::CompiledFlow;
use super::deploy_provider::{
    ClusterSpec, Deploy, ExternalSpec, IntoProcessSpec, Node, ProcessSpec, RegisterPort,
};
use super::ir::HydroRoot;
use crate::live_collections::stream::{Ordering, Retries};
use crate::location::dynamic::LocationId;
use crate::location::external_process::{
    ExternalBincodeBidi, ExternalBincodeSink, ExternalBincodeStream, ExternalBytesPort,
};
use crate::location::{Cluster, External, Location, LocationKey, LocationType, Process};
use crate::staging_util::Invariant;
use crate::telemetry::Sidecar;

pub struct DeployFlow<'a, D>
where
    D: Deploy<'a>,
{
    pub(super) ir: Vec<HydroRoot>,

    pub(super) locations: SlotMap<LocationKey, LocationType>,
    pub(super) location_names: SecondaryMap<LocationKey, String>,

    /// Deployed instances of each process in the flow
    pub(super) processes: SparseSecondaryMap<LocationKey, D::Process>,
    pub(super) clusters: SparseSecondaryMap<LocationKey, D::Cluster>,
    pub(super) externals: SparseSecondaryMap<LocationKey, D::External>,

    /// Sidecars which may be added to each location (process or cluster, not externals).
    /// See [`crate::telemetry::Sidecar`].
    pub(super) sidecars: SparseSecondaryMap<LocationKey, Vec<syn::Expr>>,

    /// Application name used in telemetry.
    pub(super) flow_name: String,

    pub(super) _phantom: Invariant<'a, D>,
}

impl<'a, D: Deploy<'a>> DeployFlow<'a, D> {
    pub fn ir(&self) -> &Vec<HydroRoot> {
        &self.ir
    }

    /// Application name used in telemetry.
    pub fn flow_name(&self) -> &str {
        &self.flow_name
    }

    pub fn with_process<P>(
        mut self,
        process: &Process<P>,
        spec: impl IntoProcessSpec<'a, D>,
    ) -> Self {
        self.processes.insert(
            process.key,
            spec.into_process_spec()
                .build(process.key, &self.location_names[process.key]),
        );
        self
    }

    /// TODO(mingwei): unstable API
    #[doc(hidden)]
    pub fn with_process_erased(
        mut self,
        process_loc_key: LocationKey,
        spec: impl IntoProcessSpec<'a, D>,
    ) -> Self {
        assert_eq!(
            Some(&LocationType::Process),
            self.locations.get(process_loc_key),
            "No process with the given `LocationKey` was found."
        );
        self.processes.insert(
            process_loc_key,
            spec.into_process_spec()
                .build(process_loc_key, &self.location_names[process_loc_key]),
        );
        self
    }

    pub fn with_remaining_processes<S: IntoProcessSpec<'a, D> + 'a>(
        mut self,
        spec: impl Fn() -> S,
    ) -> Self {
        for (location_key, &location_type) in self.locations.iter() {
            if LocationType::Process == location_type {
                self.processes
                    .entry(location_key)
                    .expect("location was removed")
                    .or_insert_with(|| {
                        spec()
                            .into_process_spec()
                            .build(location_key, &self.location_names[location_key])
                    });
            }
        }
        self
    }

    pub fn with_cluster<C>(mut self, cluster: &Cluster<C>, spec: impl ClusterSpec<'a, D>) -> Self {
        self.clusters.insert(
            cluster.key,
            spec.build(cluster.key, &self.location_names[cluster.key]),
        );
        self
    }

    /// TODO(mingwei): unstable API
    #[doc(hidden)]
    pub fn with_cluster_erased(
        mut self,
        cluster_loc_key: LocationKey,
        spec: impl ClusterSpec<'a, D>,
    ) -> Self {
        assert_eq!(
            Some(&LocationType::Cluster),
            self.locations.get(cluster_loc_key),
            "No cluster with the given `LocationKey` was found."
        );
        self.clusters.insert(
            cluster_loc_key,
            spec.build(cluster_loc_key, &self.location_names[cluster_loc_key]),
        );
        self
    }

    pub fn with_remaining_clusters<S: ClusterSpec<'a, D> + 'a>(
        mut self,
        spec: impl Fn() -> S,
    ) -> Self {
        for (location_key, &location_type) in self.locations.iter() {
            if LocationType::Cluster == location_type {
                self.clusters
                    .entry(location_key)
                    .expect("location was removed")
                    .or_insert_with(|| {
                        spec().build(location_key, &self.location_names[location_key])
                    });
            }
        }
        self
    }

    pub fn with_external<P>(
        mut self,
        external: &External<P>,
        spec: impl ExternalSpec<'a, D>,
    ) -> Self {
        self.externals.insert(
            external.key,
            spec.build(external.key, &self.location_names[external.key]),
        );
        self
    }

    pub fn with_remaining_externals<S: ExternalSpec<'a, D> + 'a>(
        mut self,
        spec: impl Fn() -> S,
    ) -> Self {
        for (location_key, &location_type) in self.locations.iter() {
            if LocationType::External == location_type {
                self.externals
                    .entry(location_key)
                    .expect("location was removed")
                    .or_insert_with(|| {
                        spec().build(location_key, &self.location_names[location_key])
                    });
            }
        }
        self
    }

    /// Adds a [`Sidecar`] to all processes and clusters in the flow.
    pub fn with_sidecar_all(mut self, sidecar: &impl Sidecar) -> Self {
        for (location_key, &location_type) in self.locations.iter() {
            if !matches!(location_type, LocationType::Process | LocationType::Cluster) {
                continue;
            }

            let location_name = &self.location_names[location_key];

            let sidecar = sidecar.to_expr(
                self.flow_name(),
                location_key,
                location_type,
                location_name,
                &quote::format_ident!("{}", super::DFIR_IDENT),
            );
            self.sidecars
                .entry(location_key)
                .expect("location was removed")
                .or_default()
                .push(sidecar);
        }

        self
    }

    /// Adds a [`Sidecar`] to the given location.
    pub fn with_sidecar_internal(
        mut self,
        location_key: LocationKey,
        sidecar: &impl Sidecar,
    ) -> Self {
        let location_type = self.locations[location_key];
        let location_name = &self.location_names[location_key];
        let sidecar = sidecar.to_expr(
            self.flow_name(),
            location_key,
            location_type,
            location_name,
            &quote::format_ident!("{}", super::DFIR_IDENT),
        );
        self.sidecars
            .entry(location_key)
            .expect("location was removed")
            .or_default()
            .push(sidecar);
        self
    }

    /// Adds a [`Sidecar`] to a specific process in the flow.
    pub fn with_sidecar_process(self, process: &Process<()>, sidecar: &impl Sidecar) -> Self {
        self.with_sidecar_internal(process.key, sidecar)
    }

    /// Adds a [`Sidecar`] to a specific cluster in the flow.
    pub fn with_sidecar_cluster(self, cluster: &Cluster<()>, sidecar: &impl Sidecar) -> Self {
        self.with_sidecar_internal(cluster.key, sidecar)
    }

    /// Compiles the flow into DFIR ([`dfir_lang::graph::DfirGraph`]) without networking.
    /// Useful for generating Mermaid diagrams of the DFIR.
    ///
    /// (This returned DFIR will not compile due to the networking missing).
    pub fn preview_compile(&mut self) -> CompiledFlow<'a> {
        // NOTE: `build_inner` does not actually mutate the IR, but `&mut` is required
        // only because the shared traversal logic requires it
        CompiledFlow {
            dfir: build_inner::<D>(&mut self.ir),
            extra_stmts: SparseSecondaryMap::new(),
            sidecars: SparseSecondaryMap::new(),
            _phantom: PhantomData,
        }
    }

    /// Compiles the flow into DFIR ([`dfir_lang::graph::DfirGraph`]) including networking.
    ///
    /// (This does not compile the DFIR itself, instead use [`Self::deploy`] to compile & deploy the DFIR).
    pub fn compile(mut self) -> CompiledFlow<'a> {
        self.compile_internal()
    }

    /// Same as [`Self::compile`] but does not invalidate `self`, for internal use.
    ///
    /// Empties `self.sidecars` and modifies `self.ir`, leaving `self` in a partial state.
    fn compile_internal(&mut self) -> CompiledFlow<'a> {
        let mut seen_tees: HashMap<_, _> = HashMap::new();
        let mut extra_stmts = SparseSecondaryMap::new();
        for leaf in self.ir.iter_mut() {
            leaf.compile_network::<D>(
                &mut extra_stmts,
                &mut seen_tees,
                &self.processes,
                &self.clusters,
                &self.externals,
            );
        }

        CompiledFlow {
            dfir: build_inner::<D>(&mut self.ir),
            extra_stmts,
            sidecars: std::mem::take(&mut self.sidecars),
            _phantom: PhantomData,
        }
    }

    /// Creates the variables for cluster IDs and adds them into `extra_stmts`.
    fn cluster_id_stmts(&self, extra_stmts: &mut SparseSecondaryMap<LocationKey, Vec<syn::Stmt>>) {
        let mut all_clusters_sorted = self.clusters.keys().collect::<Vec<_>>();
        all_clusters_sorted.sort();

        for cluster_key in all_clusters_sorted {
            let self_id_ident = syn::Ident::new(
                &format!("__hydro_lang_cluster_self_id_{}", cluster_key),
                Span::call_site(),
            );
            let self_id_expr = D::cluster_self_id().splice_untyped();
            extra_stmts
                .entry(cluster_key)
                .expect("location was removed")
                .or_default()
                .push(syn::parse_quote! {
                    let #self_id_ident = &*Box::leak(Box::new(#self_id_expr));
                });

            for other_location in self.processes.keys().chain(self.clusters.keys()) {
                let other_id_ident = syn::Ident::new(
                    &format!("__hydro_lang_cluster_ids_{}", cluster_key),
                    Span::call_site(),
                );
                let other_id_expr = D::cluster_ids(cluster_key).splice_untyped();
                extra_stmts
                    .entry(other_location)
                    .expect("location was removed")
                    .or_default()
                    .push(syn::parse_quote! {
                        let #other_id_ident = #other_id_expr;
                    });
            }
        }
    }

    /// Compiles and deploys the flow.
    ///
    /// Rough outline of steps:
    /// * Compiles the Hydro into DFIR.
    /// * Instantiates nodes as configured.
    /// * Compiles the corresponding DFIR into binaries for nodes as needed.
    /// * Connects up networking as needed.
    #[must_use]
    pub fn deploy(mut self, env: &mut D::InstantiateEnv) -> DeployResult<'a, D> {
        let CompiledFlow {
            dfir,
            mut extra_stmts,
            mut sidecars,
            _phantom,
        } = self.compile_internal();

        let mut compiled = dfir;
        self.cluster_id_stmts(&mut extra_stmts);
        let mut meta = D::Meta::default();

        let (mut processes, mut clusters, mut externals) = (
            self.processes
                .into_iter()
                .filter(|&(node_key, ref node)| {
                    if let Some(ir) = compiled.remove(node_key) {
                        node.instantiate(
                            env,
                            &mut meta,
                            ir,
                            extra_stmts.remove(node_key).as_deref().unwrap_or_default(),
                            sidecars.remove(node_key).as_deref().unwrap_or_default(),
                        );
                        true
                    } else {
                        false
                    }
                })
                .collect::<SparseSecondaryMap<_, _>>(),
            self.clusters
                .into_iter()
                .filter(|&(cluster_key, ref cluster)| {
                    if let Some(ir) = compiled.remove(cluster_key) {
                        cluster.instantiate(
                            env,
                            &mut meta,
                            ir,
                            extra_stmts
                                .remove(cluster_key)
                                .as_deref()
                                .unwrap_or_default(),
                            sidecars.remove(cluster_key).as_deref().unwrap_or_default(),
                        );
                        true
                    } else {
                        false
                    }
                })
                .collect::<SparseSecondaryMap<_, _>>(),
            self.externals
                .into_iter()
                .inspect(|&(external_key, ref external)| {
                    assert!(!extra_stmts.contains_key(external_key));
                    assert!(!sidecars.contains_key(external_key));
                    external.instantiate(env, &mut meta, Default::default(), &[], &[]);
                })
                .collect::<SparseSecondaryMap<_, _>>(),
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
        for leaf in self.ir.iter_mut() {
            leaf.connect_network(&mut seen_tees_connect);
        }

        DeployResult {
            location_names: self.location_names,
            processes,
            clusters,
            externals,
        }
    }
}

pub struct DeployResult<'a, D: Deploy<'a>> {
    location_names: SecondaryMap<LocationKey, String>,
    processes: SparseSecondaryMap<LocationKey, D::Process>,
    clusters: SparseSecondaryMap<LocationKey, D::Cluster>,
    externals: SparseSecondaryMap<LocationKey, D::External>,
}

impl<'a, D: Deploy<'a>> DeployResult<'a, D> {
    pub fn get_process<P>(&self, p: &Process<P>) -> &D::Process {
        let LocationId::Process(location_key) = p.id() else {
            panic!("Process ID expected")
        };
        self.processes.get(location_key).unwrap()
    }

    pub fn get_cluster<C>(&self, c: &Cluster<'a, C>) -> &D::Cluster {
        let LocationId::Cluster(location_key) = c.id() else {
            panic!("Cluster ID expected")
        };
        self.clusters.get(location_key).unwrap()
    }

    pub fn get_external<P>(&self, e: &External<P>) -> &D::External {
        self.externals.get(e.key).unwrap()
    }

    pub fn get_all_processes(&self) -> impl Iterator<Item = (LocationId, String, &D::Process)> {
        self.processes.iter().map(|(location_key, p)| {
            (
                LocationId::Process(location_key),
                self.location_names[location_key].clone(),
                p,
            )
        })
    }

    pub fn get_all_clusters(&self) -> impl Iterator<Item = (LocationId, String, &D::Cluster)> {
        self.clusters.iter().map(|(location_key, c)| {
            (
                LocationId::Cluster(location_key),
                self.location_names[location_key].clone(),
                c,
            )
        })
    }

    #[deprecated(note = "use `connect` instead")]
    pub async fn connect_bytes<M>(
        &self,
        port: ExternalBytesPort<M>,
    ) -> (
        Pin<Box<dyn Stream<Item = Result<BytesMut, Error>>>>,
        Pin<Box<dyn Sink<Bytes, Error = Error>>>,
    ) {
        self.connect(port).await
    }

    #[deprecated(note = "use `connect` instead")]
    pub async fn connect_sink_bytes<M>(
        &self,
        port: ExternalBytesPort<M>,
    ) -> Pin<Box<dyn Sink<Bytes, Error = Error>>> {
        self.connect(port).await.1
    }

    pub async fn connect_bincode<
        InT: Serialize + 'static,
        OutT: DeserializeOwned + 'static,
        Many,
    >(
        &self,
        port: ExternalBincodeBidi<InT, OutT, Many>,
    ) -> (
        Pin<Box<dyn Stream<Item = OutT>>>,
        Pin<Box<dyn Sink<InT, Error = Error>>>,
    ) {
        self.externals
            .get(port.process_key)
            .unwrap()
            .as_bincode_bidi(port.port_id)
            .await
    }

    #[deprecated(note = "use `connect` instead")]
    pub async fn connect_sink_bincode<T: Serialize + DeserializeOwned + 'static, Many>(
        &self,
        port: ExternalBincodeSink<T, Many>,
    ) -> Pin<Box<dyn Sink<T, Error = Error>>> {
        self.connect(port).await
    }

    #[deprecated(note = "use `connect` instead")]
    pub async fn connect_source_bytes(
        &self,
        port: ExternalBytesPort,
    ) -> Pin<Box<dyn Stream<Item = Result<BytesMut, Error>>>> {
        self.connect(port).await.0
    }

    #[deprecated(note = "use `connect` instead")]
    pub async fn connect_source_bincode<
        T: Serialize + DeserializeOwned + 'static,
        O: Ordering,
        R: Retries,
    >(
        &self,
        port: ExternalBincodeStream<T, O, R>,
    ) -> Pin<Box<dyn Stream<Item = T>>> {
        self.connect(port).await
    }

    pub async fn connect<'b, P: ConnectableAsync<&'b Self>>(
        &'b self,
        port: P,
    ) -> <P as ConnectableAsync<&'b Self>>::Output {
        port.connect(self).await
    }
}

#[cfg(stageleft_runtime)]
#[cfg(feature = "deploy")]
#[cfg_attr(docsrs, doc(cfg(feature = "deploy")))]
impl DeployResult<'_, crate::deploy::HydroDeploy> {
    /// Get the raw port handle.
    pub fn raw_port<M>(
        &self,
        port: ExternalBytesPort<M>,
    ) -> hydro_deploy::custom_service::CustomClientPort {
        self.externals
            .get(port.process_key)
            .unwrap()
            .raw_port(port.port_id)
    }
}

pub trait ConnectableAsync<Ctx> {
    type Output;

    fn connect(self, ctx: Ctx) -> impl Future<Output = Self::Output>;
}

impl<'a, D: Deploy<'a>, M> ConnectableAsync<&DeployResult<'a, D>> for ExternalBytesPort<M> {
    type Output = (
        Pin<Box<dyn Stream<Item = Result<BytesMut, Error>>>>,
        Pin<Box<dyn Sink<Bytes, Error = Error>>>,
    );

    async fn connect(self, ctx: &DeployResult<'a, D>) -> Self::Output {
        ctx.externals
            .get(self.process_key)
            .unwrap()
            .as_bytes_bidi(self.port_id)
            .await
    }
}

impl<'a, D: Deploy<'a>, T: DeserializeOwned + 'static, O: Ordering, R: Retries>
    ConnectableAsync<&DeployResult<'a, D>> for ExternalBincodeStream<T, O, R>
{
    type Output = Pin<Box<dyn Stream<Item = T>>>;

    async fn connect(self, ctx: &DeployResult<'a, D>) -> Self::Output {
        ctx.externals
            .get(self.process_key)
            .unwrap()
            .as_bincode_source(self.port_id)
            .await
    }
}

impl<'a, D: Deploy<'a>, T: Serialize + 'static, Many> ConnectableAsync<&DeployResult<'a, D>>
    for ExternalBincodeSink<T, Many>
{
    type Output = Pin<Box<dyn Sink<T, Error = Error>>>;

    async fn connect(self, ctx: &DeployResult<'a, D>) -> Self::Output {
        ctx.externals
            .get(self.process_key)
            .unwrap()
            .as_bincode_sink(self.port_id)
            .await
    }
}
