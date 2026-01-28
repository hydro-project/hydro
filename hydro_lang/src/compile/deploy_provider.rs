use std::io::Error;
use std::pin::Pin;

use bytes::{Bytes, BytesMut};
use dfir_lang::graph::DfirGraph;
use futures::{Sink, Stream};
use serde::Serialize;
use serde::de::DeserializeOwned;
use stageleft::QuotedWithContext;

use crate::compile::builder::ExternalPortId;
use crate::location::dynamic::LocationId;
use crate::location::member_id::TaglessMemberId;
use crate::location::{LocationKey, MembershipEvent, NetworkHint};

pub trait Deploy<'a> {
    type Meta: Default;
    type InstantiateEnv;

    type Process: Node<Meta = Self::Meta, InstantiateEnv = Self::InstantiateEnv> + Clone;
    type Cluster: Node<Meta = Self::Meta, InstantiateEnv = Self::InstantiateEnv> + Clone;
    type External: Node<Meta = Self::Meta, InstantiateEnv = Self::InstantiateEnv>
        + RegisterPort<'a, Self>;

    fn o2o_sink_source(
        p1: &Self::Process,
        p1_port: &<Self::Process as Node>::Port,
        p2: &Self::Process,
        p2_port: &<Self::Process as Node>::Port,
    ) -> (syn::Expr, syn::Expr);
    fn o2o_connect(
        p1: &Self::Process,
        p1_port: &<Self::Process as Node>::Port,
        p2: &Self::Process,
        p2_port: &<Self::Process as Node>::Port,
    ) -> Box<dyn FnOnce()>;

    fn o2m_sink_source(
        p1: &Self::Process,
        p1_port: &<Self::Process as Node>::Port,
        c2: &Self::Cluster,
        c2_port: &<Self::Cluster as Node>::Port,
    ) -> (syn::Expr, syn::Expr);
    fn o2m_connect(
        p1: &Self::Process,
        p1_port: &<Self::Process as Node>::Port,
        c2: &Self::Cluster,
        c2_port: &<Self::Cluster as Node>::Port,
    ) -> Box<dyn FnOnce()>;

    fn m2o_sink_source(
        c1: &Self::Cluster,
        c1_port: &<Self::Cluster as Node>::Port,
        p2: &Self::Process,
        p2_port: &<Self::Process as Node>::Port,
    ) -> (syn::Expr, syn::Expr);
    fn m2o_connect(
        c1: &Self::Cluster,
        c1_port: &<Self::Cluster as Node>::Port,
        p2: &Self::Process,
        p2_port: &<Self::Process as Node>::Port,
    ) -> Box<dyn FnOnce()>;

    fn m2m_sink_source(
        c1: &Self::Cluster,
        c1_port: &<Self::Cluster as Node>::Port,
        c2: &Self::Cluster,
        c2_port: &<Self::Cluster as Node>::Port,
    ) -> (syn::Expr, syn::Expr);
    fn m2m_connect(
        c1: &Self::Cluster,
        c1_port: &<Self::Cluster as Node>::Port,
        c2: &Self::Cluster,
        c2_port: &<Self::Cluster as Node>::Port,
    ) -> Box<dyn FnOnce()>;

    fn e2o_many_source(
        extra_stmts: &mut Vec<syn::Stmt>,
        p2: &Self::Process,
        p2_port: &<Self::Process as Node>::Port,
        codec_type: &syn::Type,
        shared_handle: String,
    ) -> syn::Expr;
    fn e2o_many_sink(shared_handle: String) -> syn::Expr;

    fn e2o_source(
        extra_stmts: &mut Vec<syn::Stmt>,
        p1: &Self::External,
        p1_port: &<Self::External as Node>::Port,
        p2: &Self::Process,
        p2_port: &<Self::Process as Node>::Port,
        codec_type: &syn::Type,
        shared_handle: String,
    ) -> syn::Expr;
    fn e2o_connect(
        p1: &Self::External,
        p1_port: &<Self::External as Node>::Port,
        p2: &Self::Process,
        p2_port: &<Self::Process as Node>::Port,
        many: bool,
        server_hint: NetworkHint,
    ) -> Box<dyn FnOnce()>;

    fn o2e_sink(
        p1: &Self::Process,
        p1_port: &<Self::Process as Node>::Port,
        p2: &Self::External,
        p2_port: &<Self::External as Node>::Port,
        shared_handle: String,
    ) -> syn::Expr;

    fn cluster_ids(
        of_cluster: LocationKey,
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
    fn build(self, location_key: LocationKey, name_hint: &str) -> D::Process;
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
    fn build(self, location_key: LocationKey, name_hint: &str) -> D::Cluster;
}

pub trait ExternalSpec<'a, D>
where
    D: Deploy<'a> + ?Sized,
{
    fn build(self, location_key: LocationKey, name_hint: &str) -> D::External;
}

pub trait Node {
    /// A logical communication endpoint for this node.
    ///
    /// Implementors are free to choose the concrete representation (for example,
    /// a handle or identifier), but it must be `Clone` so that a single logical
    /// port can be duplicated and passed to multiple consumers. New ports are
    /// allocated via [`Self::next_port`].
    type Port: Clone;
    type Meta: Default;
    type InstantiateEnv;

    /// Allocates and returns a new port.
    fn next_port(&self) -> Self::Port;

    fn update_meta(&self, meta: &Self::Meta);

    fn instantiate(
        &self,
        env: &mut Self::InstantiateEnv,
        meta: &mut Self::Meta,
        graph: DfirGraph,
        extra_stmts: &[syn::Stmt],
        sidecars: &[syn::Expr],
    );
}

pub type DynSourceSink<Out, In, InErr> = (
    Pin<Box<dyn Stream<Item = Out>>>,
    Pin<Box<dyn Sink<In, Error = InErr>>>,
);

pub trait RegisterPort<'a, D>: Node + Clone
where
    D: Deploy<'a> + ?Sized,
{
    fn register(&self, external_port_id: ExternalPortId, port: Self::Port);

    fn as_bytes_bidi(
        &self,
        external_port_id: ExternalPortId,
    ) -> impl Future<Output = DynSourceSink<Result<BytesMut, Error>, Bytes, Error>> + 'a;

    fn as_bincode_bidi<InT, OutT>(
        &self,
        external_port_id: ExternalPortId,
    ) -> impl Future<Output = DynSourceSink<OutT, InT, Error>> + 'a
    where
        InT: Serialize + 'static,
        OutT: DeserializeOwned + 'static;

    fn as_bincode_sink<T>(
        &self,
        external_port_id: ExternalPortId,
    ) -> impl Future<Output = Pin<Box<dyn Sink<T, Error = Error>>>> + 'a
    where
        T: Serialize + 'static;

    fn as_bincode_source<T>(
        &self,
        external_port_id: ExternalPortId,
    ) -> impl Future<Output = Pin<Box<dyn Stream<Item = T>>>> + 'a
    where
        T: DeserializeOwned + 'static;
}
