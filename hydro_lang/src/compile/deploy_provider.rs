use std::io::Error;
use std::pin::Pin;

use bytes::{Bytes, BytesMut};
use dfir_lang::graph::DfirGraph;
use futures::{Sink, Stream};
use serde::Serialize;
use serde::de::DeserializeOwned;
use stageleft::QuotedWithContext;

use crate::location::dynamic::LocationId;
use crate::location::member_id::TaglessMemberId;
use crate::location::{MembershipEvent, NetworkHint};

pub trait Deploy<'a> {
    type InstantiateEnv;

    type Process: Node<Meta = Self::Meta, InstantiateEnv = Self::InstantiateEnv> + Clone;
    type Cluster: Node<Meta = Self::Meta, InstantiateEnv = Self::InstantiateEnv> + Clone;
    type External: Node<Meta = Self::Meta, InstantiateEnv = Self::InstantiateEnv>
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

    fn trivial_cluster(_id: usize) -> Self::Cluster {
        panic!("No trivial cluster")
    }

    fn trivial_external(_id: usize) -> Self::External {
        panic!("No trivial external process")
    }

    fn allocate_process_port(process: &Self::Process) -> Self::Port;
    fn allocate_cluster_port(cluster: &Self::Cluster) -> Self::Port;
    fn allocate_external_port(external: &Self::External) -> Self::Port;

    fn o2o_sink_source(
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

    fn e2o_many_source(
        extra_stmts: &mut Vec<syn::Stmt>,
        p2: &Self::Process,
        p2_port: &Self::Port,
        codec_type: &syn::Type,
        shared_handle: String,
    ) -> syn::Expr;
    fn e2o_many_sink(shared_handle: String) -> syn::Expr;

    fn e2o_source(
        extra_stmts: &mut Vec<syn::Stmt>,
        p1: &Self::External,
        p1_port: &Self::Port,
        p2: &Self::Process,
        p2_port: &Self::Port,
        codec_type: &syn::Type,
        shared_handle: String,
    ) -> syn::Expr;
    fn e2o_connect(
        p1: &Self::External,
        p1_port: &Self::Port,
        p2: &Self::Process,
        p2_port: &Self::Port,
        many: bool,
        server_hint: NetworkHint,
    ) -> Box<dyn FnOnce()>;

    fn o2e_sink(
        p1: &Self::Process,
        p1_port: &Self::Port,
        p2: &Self::External,
        p2_port: &Self::Port,
        shared_handle: String,
    ) -> syn::Expr;

    fn cluster_ids(
        of_cluster: usize,
    ) -> impl QuotedWithContext<'a, &'a [TaglessMemberId], ()> + Clone + 'a;

    fn cluster_self_id() -> impl QuotedWithContext<'a, TaglessMemberId, ()> + Clone + 'a;

    fn cluster_membership_stream(
        location_id: &LocationId,
    ) -> impl QuotedWithContext<'a, Box<dyn Stream<Item = (TaglessMemberId, MembershipEvent)> + Unpin>, ()>;
}

pub trait ProcessSpec<'a, D>
where
    D: Deploy<'a> + ?Sized,
{
    fn build(self, id: usize, name_hint: &str) -> D::Process;
}

pub trait IntoProcessSpec<'a, D>
where
    D: Deploy<'a> + ?Sized,
{
    type ProcessSpec: ProcessSpec<'a, D>;
    fn into_process_spec(self) -> Self::ProcessSpec;
}

impl<'a, D, T> IntoProcessSpec<'a, D> for T
where
    D: Deploy<'a> + ?Sized,
    T: ProcessSpec<'a, D>,
{
    type ProcessSpec = T;
    fn into_process_spec(self) -> Self::ProcessSpec {
        self
    }
}

pub trait ClusterSpec<'a, D>
where
    D: Deploy<'a> + ?Sized,
{
    fn build(self, id: usize, name_hint: &str) -> D::Cluster;
}

pub trait ExternalSpec<'a, D>
where
    D: Deploy<'a> + ?Sized,
{
    fn build(self, id: usize, name_hint: &str) -> D::External;
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
        graph: DfirGraph,
        extra_stmts: Vec<syn::Stmt>,
    );
}

pub type DynSourceSink<Out, In, InErr> = (
    Pin<Box<dyn Stream<Item = Out>>>,
    Pin<Box<dyn Sink<In, Error = InErr>>>,
);

pub trait RegisterPort<'a, D>: Clone
where
    D: Deploy<'a> + ?Sized,
{
    fn register(&self, key: usize, port: D::Port);
    fn raw_port(&self, key: usize) -> D::ExternalRawPort;

    fn as_bytes_bidi(
        &self,
        key: usize,
    ) -> impl Future<Output = DynSourceSink<Result<BytesMut, Error>, Bytes, Error>> + 'a;

    fn as_bincode_bidi<InT, OutT>(
        &self,
        key: usize,
    ) -> impl Future<Output = DynSourceSink<OutT, InT, Error>> + 'a
    where
        InT: Serialize + 'static,
        OutT: DeserializeOwned + 'static;

    fn as_bincode_sink<T>(
        &self,
        key: usize,
    ) -> impl Future<Output = Pin<Box<dyn Sink<T, Error = Error>>>> + 'a
    where
        T: Serialize + 'static;

    fn as_bincode_source<T>(
        &self,
        key: usize,
    ) -> impl Future<Output = Pin<Box<dyn Stream<Item = T>>>> + 'a
    where
        T: DeserializeOwned + 'static;
}
