//! "Embedded" deployment backend for Hydro.
//!
//! Instead of compiling each location into a standalone binary, this backend generates
//! a Rust source file containing one function per location. Each function returns a
//! `dfir_rs::scheduled::graph::Dfir` that can be manually driven by the caller.
//!
//! This is useful when you want full control over where and how the projected DFIR
//! code runs (e.g. embedding it into an existing application).
//!
//! # Limitations
//!
//! Networking is **not** supported. All `Deploy` networking trait methods will panic
//! if called. Only pure local computations (with data embedded in the Hydro program)
//! are supported.

use std::future::Future;
use std::io::Error;
use std::pin::Pin;

use bytes::{Bytes, BytesMut};
use dfir_lang::diagnostic::Diagnostics;
use dfir_lang::graph::DfirGraph;
use futures::{Sink, Stream};
use proc_macro2::Span;
use quote::quote;
use serde::Serialize;
use serde::de::DeserializeOwned;
use stageleft::{QuotedWithContext, q};

use super::deploy_provider::{ClusterSpec, Deploy, ExternalSpec, Node, ProcessSpec, RegisterPort};
use crate::compile::builder::ExternalPortId;
use crate::location::dynamic::LocationId;
use crate::location::member_id::TaglessMemberId;
use crate::location::{LocationKey, MembershipEvent, NetworkHint};

/// Marker type for the embedded deployment backend.
///
/// All networking methods panic — this backend only supports pure local computation.
pub enum EmbeddedDeploy {}

/// A trivial node type for embedded deployment. Stores a user-provided function name.
#[derive(Clone)]
pub struct EmbeddedNode {
    /// The function name to use in the generated code for this location.
    pub fn_name: String,
}

impl Node for EmbeddedNode {
    type Port = ();
    type Meta = ();
    type InstantiateEnv = ();

    fn next_port(&self) -> Self::Port {}

    fn update_meta(&self, _meta: &Self::Meta) {}

    fn instantiate(
        &self,
        _env: &mut Self::InstantiateEnv,
        _meta: &mut Self::Meta,
        _graph: DfirGraph,
        _extra_stmts: &[syn::Stmt],
        _sidecars: &[syn::Expr],
    ) {
        // No-op: embedded mode doesn't instantiate nodes at deploy time.
    }
}

impl<'a> RegisterPort<'a, EmbeddedDeploy> for EmbeddedNode {
    fn register(&self, _external_port_id: ExternalPortId, _port: Self::Port) {
        panic!("EmbeddedDeploy does not support external ports");
    }

    fn as_bytes_bidi(
        &self,
        _external_port_id: ExternalPortId,
    ) -> impl Future<
        Output = super::deploy_provider::DynSourceSink<Result<BytesMut, Error>, Bytes, Error>,
    > + 'a {
        async { panic!("EmbeddedDeploy does not support external ports") }
    }

    fn as_bincode_bidi<InT, OutT>(
        &self,
        _external_port_id: ExternalPortId,
    ) -> impl Future<Output = super::deploy_provider::DynSourceSink<OutT, InT, Error>> + 'a
    where
        InT: Serialize + 'static,
        OutT: DeserializeOwned + 'static,
    {
        async { panic!("EmbeddedDeploy does not support external ports") }
    }

    fn as_bincode_sink<T>(
        &self,
        _external_port_id: ExternalPortId,
    ) -> impl Future<Output = Pin<Box<dyn Sink<T, Error = Error>>>> + 'a
    where
        T: Serialize + 'static,
    {
        async { panic!("EmbeddedDeploy does not support external ports") }
    }

    fn as_bincode_source<T>(
        &self,
        _external_port_id: ExternalPortId,
    ) -> impl Future<Output = Pin<Box<dyn Stream<Item = T>>>> + 'a
    where
        T: DeserializeOwned + 'static,
    {
        async { panic!("EmbeddedDeploy does not support external ports") }
    }
}

impl<S: Into<String>> ProcessSpec<'_, EmbeddedDeploy> for S {
    fn build(self, _location_key: LocationKey, _name_hint: &str) -> EmbeddedNode {
        EmbeddedNode {
            fn_name: self.into(),
        }
    }
}

impl<S: Into<String>> ClusterSpec<'_, EmbeddedDeploy> for S {
    fn build(self, _location_key: LocationKey, _name_hint: &str) -> EmbeddedNode {
        EmbeddedNode {
            fn_name: self.into(),
        }
    }
}

impl<S: Into<String>> ExternalSpec<'_, EmbeddedDeploy> for S {
    fn build(self, _location_key: LocationKey, _name_hint: &str) -> EmbeddedNode {
        EmbeddedNode {
            fn_name: self.into(),
        }
    }
}

impl<'a> Deploy<'a> for EmbeddedDeploy {
    type Meta = ();
    type InstantiateEnv = ();

    type Process = EmbeddedNode;
    type Cluster = EmbeddedNode;
    type External = EmbeddedNode;

    fn o2o_sink_source(
        _p1: &Self::Process,
        _p1_port: &(),
        _p2: &Self::Process,
        _p2_port: &(),
    ) -> (syn::Expr, syn::Expr) {
        panic!("EmbeddedDeploy does not support networking (o2o)")
    }

    fn o2o_connect(
        _p1: &Self::Process,
        _p1_port: &(),
        _p2: &Self::Process,
        _p2_port: &(),
    ) -> Box<dyn FnOnce()> {
        panic!("EmbeddedDeploy does not support networking (o2o)")
    }

    fn o2m_sink_source(
        _p1: &Self::Process,
        _p1_port: &(),
        _c2: &Self::Cluster,
        _c2_port: &(),
    ) -> (syn::Expr, syn::Expr) {
        panic!("EmbeddedDeploy does not support networking (o2m)")
    }

    fn o2m_connect(
        _p1: &Self::Process,
        _p1_port: &(),
        _c2: &Self::Cluster,
        _c2_port: &(),
    ) -> Box<dyn FnOnce()> {
        panic!("EmbeddedDeploy does not support networking (o2m)")
    }

    fn m2o_sink_source(
        _c1: &Self::Cluster,
        _c1_port: &(),
        _p2: &Self::Process,
        _p2_port: &(),
    ) -> (syn::Expr, syn::Expr) {
        panic!("EmbeddedDeploy does not support networking (m2o)")
    }

    fn m2o_connect(
        _c1: &Self::Cluster,
        _c1_port: &(),
        _p2: &Self::Process,
        _p2_port: &(),
    ) -> Box<dyn FnOnce()> {
        panic!("EmbeddedDeploy does not support networking (m2o)")
    }

    fn m2m_sink_source(
        _c1: &Self::Cluster,
        _c1_port: &(),
        _c2: &Self::Cluster,
        _c2_port: &(),
    ) -> (syn::Expr, syn::Expr) {
        panic!("EmbeddedDeploy does not support networking (m2m)")
    }

    fn m2m_connect(
        _c1: &Self::Cluster,
        _c1_port: &(),
        _c2: &Self::Cluster,
        _c2_port: &(),
    ) -> Box<dyn FnOnce()> {
        panic!("EmbeddedDeploy does not support networking (m2m)")
    }

    fn e2o_many_source(
        _extra_stmts: &mut Vec<syn::Stmt>,
        _p2: &Self::Process,
        _p2_port: &(),
        _codec_type: &syn::Type,
        _shared_handle: String,
    ) -> syn::Expr {
        panic!("EmbeddedDeploy does not support networking (e2o)")
    }

    fn e2o_many_sink(_shared_handle: String) -> syn::Expr {
        panic!("EmbeddedDeploy does not support networking (e2o)")
    }

    fn e2o_source(
        _extra_stmts: &mut Vec<syn::Stmt>,
        _p1: &Self::External,
        _p1_port: &(),
        _p2: &Self::Process,
        _p2_port: &(),
        _codec_type: &syn::Type,
        _shared_handle: String,
    ) -> syn::Expr {
        panic!("EmbeddedDeploy does not support networking (e2o)")
    }

    fn e2o_connect(
        _p1: &Self::External,
        _p1_port: &(),
        _p2: &Self::Process,
        _p2_port: &(),
        _many: bool,
        _server_hint: NetworkHint,
    ) -> Box<dyn FnOnce()> {
        panic!("EmbeddedDeploy does not support networking (e2o)")
    }

    fn o2e_sink(
        _p1: &Self::Process,
        _p1_port: &(),
        _p2: &Self::External,
        _p2_port: &(),
        _shared_handle: String,
    ) -> syn::Expr {
        panic!("EmbeddedDeploy does not support networking (o2e)")
    }

    #[expect(
        unreachable_code,
        reason = "panic before q! which is only for return type"
    )]
    fn cluster_ids(
        _of_cluster: LocationKey,
    ) -> impl QuotedWithContext<'a, &'a [TaglessMemberId], ()> + Clone + 'a {
        panic!("EmbeddedDeploy does not support cluster IDs");
        q!(unreachable!("EmbeddedDeploy does not support cluster IDs"))
    }

    #[expect(
        unreachable_code,
        reason = "panic before q! which is only for return type"
    )]
    fn cluster_self_id() -> impl QuotedWithContext<'a, TaglessMemberId, ()> + Clone + 'a {
        panic!("EmbeddedDeploy does not support cluster self ID");
        q!(unreachable!(
            "EmbeddedDeploy does not support cluster self ID"
        ))
    }

    #[expect(
        unreachable_code,
        reason = "panic before q! which is only for return type"
    )]
    fn cluster_membership_stream(
        _location_id: &LocationId,
    ) -> impl QuotedWithContext<'a, Box<dyn Stream<Item = (TaglessMemberId, MembershipEvent)> + Unpin>, ()>
    {
        panic!("EmbeddedDeploy does not support cluster membership streams");
        q!(unreachable!(
            "EmbeddedDeploy does not support cluster membership streams"
        ))
    }
}

impl super::deploy::DeployFlow<'_, EmbeddedDeploy> {
    /// Generates a `syn::File` containing one function per location in the flow.
    ///
    /// Each generated function has the signature:
    /// ```ignore
    /// pub fn <fn_name>() -> dfir_rs::scheduled::graph::Dfir<'static>
    /// ```
    /// where `fn_name` is the `String` passed to `with_process` / `with_cluster`.
    ///
    /// The returned `Dfir` can be manually executed by the caller.
    ///
    /// # Arguments
    ///
    /// * `crate_name` — the name of the crate containing the Hydro program (used for stageleft
    ///   re-exports). Hyphens will be replaced with underscores.
    ///
    /// # Usage
    ///
    /// Typically called from a `build.rs` in a wrapper crate:
    /// ```ignore
    /// // build.rs
    /// let deploy = flow.with_process(&process, "my_fn".to_string());
    /// let code = deploy.generate_embedded("my_hydro_crate");
    /// let out_dir = std::env::var("OUT_DIR").unwrap();
    /// std::fs::write(format!("{out_dir}/embedded.rs"), prettyplease::unparse(&code)).unwrap();
    /// ```
    ///
    /// Then in `lib.rs`:
    /// ```ignore
    /// include!(concat!(env!("OUT_DIR"), "/embedded.rs"));
    /// ```
    pub fn generate_embedded(mut self, crate_name: &str) -> syn::File {
        // Use preview_compile which skips networking (calls build_inner directly).
        // This is exactly what we want: pure DFIR graphs without any networking plumbing.
        let compiled = self.preview_compile();

        let root = crate::staging_util::get_this_crate();
        let orig_crate_name = quote::format_ident!("{}", crate_name.replace('-', "_"));

        let mut functions: Vec<syn::Item> = Vec::new();

        // Sort location keys for deterministic output.
        let mut location_keys: Vec<_> = compiled.all_dfir().keys().collect();
        location_keys.sort();

        for location_key in location_keys {
            let graph = &compiled.all_dfir()[location_key];

            // Get the user-provided function name from the node.
            let fn_name = self
                .processes
                .get(location_key)
                .map(|n| &n.fn_name)
                .or_else(|| self.clusters.get(location_key).map(|n| &n.fn_name))
                .or_else(|| self.externals.get(location_key).map(|n| &n.fn_name))
                .expect("location key not found in any node map");

            let fn_ident = syn::Ident::new(fn_name, Span::call_site());

            let mut diagnostics = Diagnostics::new();
            let dfir_tokens = graph
                .as_code(&quote! { __root_dfir_rs }, true, quote!(), &mut diagnostics)
                .expect("DFIR code generation failed with diagnostics.");
            let dfir_expr: syn::Expr = syn::parse2(dfir_tokens).unwrap();

            let func: syn::Item = syn::parse_quote! {
                #[allow(unused, non_snake_case, clippy::suspicious_else_formatting)]
                pub fn #fn_ident<'a>() -> #root::runtime_support::dfir_rs::scheduled::graph::Dfir<'a> {
                    #dfir_expr
                }
            };
            functions.push(func);
        }

        syn::parse_quote! {
            use #orig_crate_name::__staged::__deps::*;
            use #root::prelude::*;
            use #root::runtime_support::dfir_rs as __root_dfir_rs;
            pub use #orig_crate_name::__staged;

            #( #functions )*
        }
    }
}
