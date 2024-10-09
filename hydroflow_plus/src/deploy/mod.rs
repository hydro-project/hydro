use std::future::Future;
use std::io::Error;
use std::pin::Pin;

use hydroflow::bytes::Bytes;
use hydroflow::futures::{Sink, Stream};
use hydroflow_lang::graph::HydroflowGraph;
use serde::de::DeserializeOwned;
use serde::Serialize;
use stageleft::Quoted;

#[cfg(feature = "deploy_runtime")]
pub mod macro_runtime;

#[cfg(feature = "deploy")]
pub(crate) mod trybuild;

pub use macro_runtime::*;
#[cfg(feature = "deploy")]
pub use trybuild::init_test;

#[cfg(feature = "deploy_runtime")]
pub mod deploy_runtime;
#[cfg(feature = "deploy_runtime")]
pub use deploy_runtime::HydroflowPlusMeta;

#[cfg(feature = "deploy")]
pub mod deploy_graph;

#[cfg(feature = "deploy")]
pub use deploy_graph::*;

pub mod in_memory_graph;
pub use in_memory_graph::*;

pub trait LocalDeploy<'a> {
    type Process: Node<Meta = Self::Meta>;
    type Cluster: Node<Meta = Self::Meta>;
    type ExternalProcess: Node<Meta = Self::Meta>;
    type Meta: Default;
    type GraphId;

    fn has_trivial_node() -> bool {
        false
    }

    fn trivial_process(_id: usize) -> Self::Process {
        panic!("No trivial process")
    }

    fn trivial_cluster(_id: usize) -> Self::Cluster {
        panic!("No trivial cluster")
    }
}

pub trait Deploy<'a> {
    type InstantiateEnv;
    type CompileEnv;

    type Process: Node<Meta = Self::Meta, InstantiateEnv = Self::InstantiateEnv> + Clone;
    type Cluster: Node<Meta = Self::Meta, InstantiateEnv = Self::InstantiateEnv> + Clone;
    type ExternalProcess: Node<Meta = Self::Meta, InstantiateEnv = Self::InstantiateEnv>
        + RegisterPort<'a, Self>;
    type Port: Clone;
    type ExternalRawPort;
    type Meta: Default;

    /// Type of ID used to switch between different subgraphs at runtime.
    type GraphId;

    fn has_trivial_node() -> bool {
        false
    }

    fn trivial_process(_id: usize) -> Self::Process {
        panic!("No trivial process")
    }

    fn trivail_cluster(_id: usize) -> Self::Cluster {
        panic!("No trivial cluster")
    }

    fn allocate_process_port(process: &Self::Process) -> Self::Port;
    fn allocate_cluster_port(cluster: &Self::Cluster) -> Self::Port;
    fn allocate_external_port(external: &Self::ExternalProcess) -> Self::Port;

    fn o2o_sink_source(
        compile_env: &Self::CompileEnv,
        p1: &Self::Process,
        p1_port: &Self::Port,
        p2: &Self::Process,
        p2_port: &Self::Port,
    ) -> (syn::Expr, syn::Expr);
    fn o2o_connect(
        p1: &Self::Process,
        p1_port: &Self::Port,
        p2: &Self::Process,
        p2_port: &Self::Port,
    ) -> Box<dyn FnOnce()>;

    fn o2m_sink_source(
        compile_env: &Self::CompileEnv,
        p1: &Self::Process,
        p1_port: &Self::Port,
        c2: &Self::Cluster,
        c2_port: &Self::Port,
    ) -> (syn::Expr, syn::Expr);
    fn o2m_connect(
        p1: &Self::Process,
        p1_port: &Self::Port,
        c2: &Self::Cluster,
        c2_port: &Self::Port,
    ) -> Box<dyn FnOnce()>;

    fn m2o_sink_source(
        compile_env: &Self::CompileEnv,
        c1: &Self::Cluster,
        c1_port: &Self::Port,
        p2: &Self::Process,
        p2_port: &Self::Port,
    ) -> (syn::Expr, syn::Expr);
    fn m2o_connect(
        c1: &Self::Cluster,
        c1_port: &Self::Port,
        p2: &Self::Process,
        p2_port: &Self::Port,
    ) -> Box<dyn FnOnce()>;

    fn m2m_sink_source(
        compile_env: &Self::CompileEnv,
        c1: &Self::Cluster,
        c1_port: &Self::Port,
        c2: &Self::Cluster,
        c2_port: &Self::Port,
    ) -> (syn::Expr, syn::Expr);
    fn m2m_connect(
        c1: &Self::Cluster,
        c1_port: &Self::Port,
        c2: &Self::Cluster,
        c2_port: &Self::Port,
    ) -> Box<dyn FnOnce()>;

    fn e2o_source(
        compile_env: &Self::CompileEnv,
        p1: &Self::ExternalProcess,
        p1_port: &Self::Port,
        p2: &Self::Process,
        p2_port: &Self::Port,
    ) -> syn::Expr;
    fn e2o_connect(
        p1: &Self::ExternalProcess,
        p1_port: &Self::Port,
        p2: &Self::Process,
        p2_port: &Self::Port,
    ) -> Box<dyn FnOnce()>;

    fn o2e_sink(
        compile_env: &Self::CompileEnv,
        p1: &Self::Process,
        p1_port: &Self::Port,
        p2: &Self::ExternalProcess,
        p2_port: &Self::Port,
    ) -> syn::Expr;
    fn o2e_connect(
        p1: &Self::Process,
        p1_port: &Self::Port,
        p2: &Self::ExternalProcess,
        p2_port: &Self::Port,
    ) -> Box<dyn FnOnce()>;

    fn cluster_ids(
        env: &Self::CompileEnv,
        of_cluster: usize,
    ) -> impl Quoted<'a, &'a Vec<u32>> + Copy + 'a;
    fn cluster_self_id(env: &Self::CompileEnv) -> impl Quoted<'a, u32> + Copy + 'a;
}

impl<
        'a,
        T: Deploy<'a, Process = N, Cluster = C, ExternalProcess = E, Meta = M, GraphId = R>,
        N: Node<Meta = M>,
        C: Node<Meta = M>,
        E: Node<Meta = M>,
        M: Default,
        R,
    > LocalDeploy<'a> for T
{
    type Process = N;
    type Cluster = C;
    type ExternalProcess = E;
    type Meta = M;
    type GraphId = R;

    fn has_trivial_node() -> bool {
        <T as Deploy<'a>>::has_trivial_node()
    }

    fn trivial_process(id: usize) -> Self::Process {
        <T as Deploy<'a>>::trivial_process(id)
    }

    fn trivial_cluster(id: usize) -> Self::Cluster {
        <T as Deploy<'a>>::trivail_cluster(id)
    }
}

pub trait ProcessSpec<'a, D: LocalDeploy<'a> + ?Sized> {
    fn build(self, id: usize, name_hint: &str) -> D::Process;
}

pub trait ClusterSpec<'a, D: LocalDeploy<'a> + ?Sized> {
    fn build(self, id: usize, name_hint: &str) -> D::Cluster;
}

pub trait ExternalSpec<'a, D: LocalDeploy<'a> + ?Sized> {
    fn build(self, id: usize, name_hint: &str) -> D::ExternalProcess;
}

pub trait Node {
    type Port;
    type Meta;
    type InstantiateEnv;

    fn next_port(&self) -> Self::Port;

    fn update_meta(&mut self, meta: &Self::Meta);

    fn instantiate(
        &self,
        env: &mut Self::InstantiateEnv,
        meta: &mut Self::Meta,
        graph: HydroflowGraph,
        extra_stmts: Vec<syn::Stmt>,
    );
}

pub trait RegisterPort<'a, D: Deploy<'a> + ?Sized>: Clone {
    fn register(&self, key: usize, port: D::Port);
    fn raw_port(&self, key: usize) -> D::ExternalRawPort;

    fn as_bytes_sink(
        &self,
        key: usize,
    ) -> impl Future<Output = Pin<Box<dyn Sink<Bytes, Error = Error>>>> + 'a;

    fn as_bincode_sink<T: Serialize + 'static>(
        &self,
        key: usize,
    ) -> impl Future<Output = Pin<Box<dyn Sink<T, Error = Error>>>> + 'a;

    fn as_bytes_source(
        &self,
        key: usize,
    ) -> impl Future<Output = Pin<Box<dyn Stream<Item = Bytes>>>> + 'a;

    fn as_bincode_source<T: DeserializeOwned + 'static>(
        &self,
        key: usize,
    ) -> impl Future<Output = Pin<Box<dyn Stream<Item = T>>>> + 'a;
}
