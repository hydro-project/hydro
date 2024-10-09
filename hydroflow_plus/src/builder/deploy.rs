use std::collections::{BTreeMap, HashMap};
use std::io::Error;
use std::marker::PhantomData;
use std::pin::Pin;

use hydroflow::bytes::Bytes;
use hydroflow::futures::{Sink, Stream};
use proc_macro2::Span;
use serde::de::DeserializeOwned;
use serde::Serialize;
use stageleft::Quoted;

use super::built::build_inner;
use crate::deploy::{ExternalSpec, LocalDeploy, Node, RegisterPort};
use crate::ir::HfPlusLeaf;
use crate::location::{
    ExternalBincodeSink, ExternalBincodeStream, ExternalBytesPort, ExternalProcess, Location,
    LocationId,
};
use crate::{Cluster, ClusterSpec, Deploy, HfCompiled, Process, ProcessSpec};

pub struct DeployFlow<'a, D: LocalDeploy<'a>> {
    pub(super) ir: Vec<HfPlusLeaf>,
    pub(super) nodes: HashMap<usize, D::Process>,
    pub(super) externals: HashMap<usize, D::ExternalProcess>,
    pub(super) clusters: HashMap<usize, D::Cluster>,
    pub(super) used: bool,

    pub(super) _phantom: PhantomData<&'a mut &'a D>,
}

impl<'a, D: LocalDeploy<'a>> Drop for DeployFlow<'a, D> {
    fn drop(&mut self) {
        if !self.used {
            panic!("Dropped DeployFlow without instantiating, you may have forgotten to call `compile` or `deploy`.");
        }
    }
}

impl<'a, D: LocalDeploy<'a>> DeployFlow<'a, D> {
    pub fn with_process<P>(mut self, process: &Process<P>, spec: impl ProcessSpec<'a, D>) -> Self {
        let tag_name = std::any::type_name::<P>().to_string();
        self.nodes
            .insert(process.id, spec.build(process.id, &tag_name));
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

    pub fn with_cluster<C>(mut self, cluster: &Cluster<C>, spec: impl ClusterSpec<'a, D>) -> Self {
        let tag_name = std::any::type_name::<C>().to_string();
        self.clusters
            .insert(cluster.id, spec.build(cluster.id, &tag_name));
        self
    }
}

impl<'a, D: Deploy<'a>> DeployFlow<'a, D> {
    pub fn compile(mut self, env: &D::CompileEnv) -> HfCompiled<'a, D::GraphId> {
        self.used = true;

        let mut seen_tees: HashMap<_, _> = HashMap::new();
        let mut flow_state_networked: Vec<HfPlusLeaf> = std::mem::take(&mut self.ir)
            .into_iter()
            .map(|leaf| {
                leaf.compile_network::<D>(
                    env,
                    &mut seen_tees,
                    &self.nodes,
                    &self.clusters,
                    &self.externals,
                )
            })
            .collect();

        let extra_stmts = self.extra_stmts(env);

        HfCompiled {
            hydroflow_ir: build_inner(&mut flow_state_networked),
            extra_stmts,
            _phantom: PhantomData,
        }
    }

    fn extra_stmts(&self, env: &<D as Deploy<'a>>::CompileEnv) -> BTreeMap<usize, Vec<syn::Stmt>> {
        let all_locations_count = self.nodes.len() + self.clusters.len();

        let mut extra_stmts: BTreeMap<usize, Vec<syn::Stmt>> = BTreeMap::new();
        for &c_id in self.clusters.keys() {
            let self_id_ident = syn::Ident::new(
                &format!("__hydroflow_plus_cluster_self_id_{}", c_id),
                Span::call_site(),
            );
            let self_id_expr = D::cluster_self_id(env).splice_untyped();
            extra_stmts
                .entry(c_id)
                .or_default()
                .push(syn::parse_quote! {
                    let #self_id_ident = #self_id_expr;
                });

            for other_location in 0..all_locations_count {
                let other_id_ident = syn::Ident::new(
                    &format!("__hydroflow_plus_cluster_ids_{}", c_id),
                    Span::call_site(),
                );
                let other_id_expr = D::cluster_ids(env, c_id).splice_untyped();
                extra_stmts
                    .entry(other_location)
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
        let mut flow_state_networked: Vec<HfPlusLeaf> = std::mem::take(&mut self.ir)
            .into_iter()
            .map(|leaf| {
                leaf.compile_network::<D>(
                    &(),
                    &mut seen_tees_instantiate,
                    &self.nodes,
                    &self.clusters,
                    &self.externals,
                )
            })
            .collect();

        let mut compiled = build_inner(&mut flow_state_networked);
        let mut extra_stmts = self.extra_stmts(&());
        let mut meta = D::Meta::default();

        let (mut processes, mut clusters, mut externals) = (
            std::mem::take(&mut self.nodes)
                .into_iter()
                .map(|(node_id, node)| {
                    node.instantiate(
                        env,
                        &mut meta,
                        compiled.remove(&node_id).unwrap(),
                        extra_stmts.remove(&node_id).unwrap_or_default(),
                    );
                    (node_id, node)
                })
                .collect::<HashMap<_, _>>(),
            std::mem::take(&mut self.clusters)
                .into_iter()
                .map(|(cluster_id, cluster)| {
                    cluster.instantiate(
                        env,
                        &mut meta,
                        compiled.remove(&cluster_id).unwrap(),
                        extra_stmts.remove(&cluster_id).unwrap_or_default(),
                    );
                    (cluster_id, cluster)
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
        for leaf in flow_state_networked {
            leaf.connect_network(&mut seen_tees_connect);
        }

        DeployResult {
            processes,
            clusters,
            externals,
        }
    }
}

pub struct DeployResult<'a, D: Deploy<'a>> {
    processes: HashMap<usize, D::Process>,
    clusters: HashMap<usize, D::Cluster>,
    externals: HashMap<usize, D::ExternalProcess>,
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
