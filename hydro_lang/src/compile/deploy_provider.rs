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

    /// Generates the source and sink expressions when connecting a [`Self::Process`] to another
    /// [`Self::Process`].
    ///
    /// The [`Self::InstantiateEnv`] can be used to record metadata about the created channel. The
    /// provided `name` is the user-configured channel name from the network IR node.
    fn o2o_sink_source(
        env: &mut Self::InstantiateEnv,
        p1: &Self::Process,
        p1_port: &<Self::Process as Node>::Port,
        p2: &Self::Process,
        p2_port: &<Self::Process as Node>::Port,
        name: Option<&str>,
    ) -> (syn::Expr, syn::Expr);

    /// Performs any runtime wiring needed after code generation for a
    /// [`Self::Process`]-to-[`Self::Process`] channel.
    ///
    /// The returned closure is executed once all locations have been instantiated.
    fn o2o_connect(
        p1: &Self::Process,
        p1_port: &<Self::Process as Node>::Port,
        p2: &Self::Process,
        p2_port: &<Self::Process as Node>::Port,
    ) -> Box<dyn FnOnce()>;

    /// Generates the source and sink expressions when connecting a [`Self::Process`] to a
    /// [`Self::Cluster`] (one-to-many).
    ///
    /// The sink expression is used on the sending process and the source expression on each
    /// receiving cluster member. The [`Self::InstantiateEnv`] can be used to record metadata
    /// about the created channel. The provided `name` is the user-configured channel name
    /// from the network IR node.
    fn o2m_sink_source(
        env: &mut Self::InstantiateEnv,
        p1: &Self::Process,
        p1_port: &<Self::Process as Node>::Port,
        c2: &Self::Cluster,
        c2_port: &<Self::Cluster as Node>::Port,
        name: Option<&str>,
    ) -> (syn::Expr, syn::Expr);

    /// Performs any runtime wiring needed after code generation for a
    /// [`Self::Process`]-to-[`Self::Cluster`] channel.
    ///
    /// The returned closure is executed once all locations have been instantiated.
    fn o2m_connect(
        p1: &Self::Process,
        p1_port: &<Self::Process as Node>::Port,
        c2: &Self::Cluster,
        c2_port: &<Self::Cluster as Node>::Port,
    ) -> Box<dyn FnOnce()>;

    /// Generates the source and sink expressions when connecting a [`Self::Cluster`] to a
    /// [`Self::Process`] (many-to-one).
    ///
    /// The sink expression is used on each sending cluster member and the source expression
    /// on the receiving process. The [`Self::InstantiateEnv`] can be used to record metadata
    /// about the created channel. The provided `name` is the user-configured channel name
    /// from the network IR node.
    fn m2o_sink_source(
        env: &mut Self::InstantiateEnv,
        c1: &Self::Cluster,
        c1_port: &<Self::Cluster as Node>::Port,
        p2: &Self::Process,
        p2_port: &<Self::Process as Node>::Port,
        name: Option<&str>,
    ) -> (syn::Expr, syn::Expr);

    /// Performs any runtime wiring needed after code generation for a
    /// [`Self::Cluster`]-to-[`Self::Process`] channel.
    ///
    /// The returned closure is executed once all locations have been instantiated.
    fn m2o_connect(
        c1: &Self::Cluster,
        c1_port: &<Self::Cluster as Node>::Port,
        p2: &Self::Process,
        p2_port: &<Self::Process as Node>::Port,
    ) -> Box<dyn FnOnce()>;

    /// Generates the source and sink expressions when connecting a [`Self::Cluster`] to another
    /// [`Self::Cluster`] (many-to-many).
    ///
    /// The sink expression is used on each sending cluster member and the source expression
    /// on each receiving cluster member. The [`Self::InstantiateEnv`] can be used to record
    /// metadata about the created channel. The provided `name` is the user-configured channel
    /// name from the network IR node.
    fn m2m_sink_source(
        env: &mut Self::InstantiateEnv,
        c1: &Self::Cluster,
        c1_port: &<Self::Cluster as Node>::Port,
        c2: &Self::Cluster,
        c2_port: &<Self::Cluster as Node>::Port,
        name: Option<&str>,
    ) -> (syn::Expr, syn::Expr);

    /// Performs any runtime wiring needed after code generation for a
    /// [`Self::Cluster`]-to-[`Self::Cluster`] channel.
    ///
    /// The returned closure is executed once all locations have been instantiated.
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
        env: &mut Self::InstantiateEnv,
        at_location: &LocationId,
        location_id: &LocationId,
    ) -> impl QuotedWithContext<'a, Box<dyn Stream<Item = (TaglessMemberId, MembershipEvent)> + Unpin>, ()>;

    /// Registers an embedded input for the given ident and element type.
    ///
    /// Only meaningful for the embedded deployment backend. The default
    /// implementation panics.
    fn register_embedded_input(
        _env: &mut Self::InstantiateEnv,
        _location_key: LocationKey,
        _ident: &syn::Ident,
        _element_type: &syn::Type,
    ) {
        panic!("register_embedded_input is only supported by EmbeddedDeploy");
    }

    /// Registers an embedded output for the given ident and element type.
    ///
    /// Only meaningful for the embedded deployment backend. The default
    /// implementation panics.
    fn register_embedded_output(
        _env: &mut Self::InstantiateEnv,
        _location_key: LocationKey,
        _ident: &syn::Ident,
        _element_type: &syn::Type,
    ) {
        panic!("register_embedded_output is only supported by EmbeddedDeploy");
    }
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
