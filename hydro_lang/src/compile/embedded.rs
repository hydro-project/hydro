//! "Embedded" deployment backend for Hydro.
//!
//! Instead of compiling each location into a standalone binary, this backend generates
//! a Rust source file containing one function per location. Each function returns a
//! `dfir_rs::scheduled::graph::Dfir` that can be manually driven by the caller.
//!
//! This is useful when you want full control over where and how the projected DFIR
//! code runs (e.g. embedding it into an existing application).
//!
//! # Networking
//!
//! Process-to-process (o2o) networking is supported. When a location has network
//! sends or receives, the generated function takes additional `network_out` and
//! `network_in` parameters whose types are generated structs with one field per
//! network port (named after the channel). Network channels must be named via
//! `.name()` on the networking config.
//!
//! - Sinks (`EmbeddedNetworkOut`): one `FnMut(Bytes)` field per outgoing channel.
//! - Sources (`EmbeddedNetworkIn`): one `Stream<Item = Result<BytesMut, io::Error>>`
//!   field per incoming channel.
//!
//! The caller is responsible for wiring these together (e.g. via in-memory channels,
//! sockets, etc.). Cluster networking and external ports are not supported.

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
use slotmap::SparseSecondaryMap;
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
    /// The location key for this node, used to register network ports.
    pub location_key: LocationKey,
}

impl Node for EmbeddedNode {
    type Port = ();
    type Meta = ();
    type InstantiateEnv = EmbeddedInstantiateEnv;

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

    #[expect(clippy::manual_async_fn, reason = "false positive, involves lifetimes")]
    fn as_bytes_bidi(
        &self,
        _external_port_id: ExternalPortId,
    ) -> impl Future<
        Output = super::deploy_provider::DynSourceSink<Result<BytesMut, Error>, Bytes, Error>,
    > + 'a {
        async { panic!("EmbeddedDeploy does not support external ports") }
    }

    #[expect(clippy::manual_async_fn, reason = "false positive, involves lifetimes")]
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

    #[expect(clippy::manual_async_fn, reason = "false positive, involves lifetimes")]
    fn as_bincode_sink<T>(
        &self,
        _external_port_id: ExternalPortId,
    ) -> impl Future<Output = Pin<Box<dyn Sink<T, Error = Error>>>> + 'a
    where
        T: Serialize + 'static,
    {
        async { panic!("EmbeddedDeploy does not support external ports") }
    }

    #[expect(clippy::manual_async_fn, reason = "false positive, involves lifetimes")]
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
    fn build(self, location_key: LocationKey, _name_hint: &str) -> EmbeddedNode {
        EmbeddedNode {
            fn_name: self.into(),
            location_key,
        }
    }
}

impl<S: Into<String>> ClusterSpec<'_, EmbeddedDeploy> for S {
    fn build(self, location_key: LocationKey, _name_hint: &str) -> EmbeddedNode {
        EmbeddedNode {
            fn_name: self.into(),
            location_key,
        }
    }
}

impl<S: Into<String>> ExternalSpec<'_, EmbeddedDeploy> for S {
    fn build(self, location_key: LocationKey, _name_hint: &str) -> EmbeddedNode {
        EmbeddedNode {
            fn_name: self.into(),
            location_key,
        }
    }
}

/// Collected embedded input/output registrations, keyed by location.
///
/// During `compile_network`, each `HydroSource::Embedded` and `HydroRoot::EmbeddedOutput`
/// IR node registers its ident, element type, and location key here.
/// `generate_embedded` then uses this to add the appropriate parameters
/// to each generated function.
#[derive(Default)]
pub struct EmbeddedInstantiateEnv {
    /// (ident name, element type) pairs per location key, for inputs.
    pub inputs: SparseSecondaryMap<LocationKey, Vec<(syn::Ident, syn::Type)>>,
    /// (ident name, element type) pairs per location key, for outputs.
    pub outputs: SparseSecondaryMap<LocationKey, Vec<(syn::Ident, syn::Type)>>,
    /// Network output port names per location key (sender side of channels).
    /// Each entry is (port_name, is_tagged) where is_tagged means the type is (TaglessMemberId, Bytes).
    pub network_outputs: SparseSecondaryMap<LocationKey, Vec<(String, bool)>>,
    /// Network input port names per location key (receiver side of channels).
    /// Each entry is (port_name, is_tagged) where is_tagged means the type is Result<(TaglessMemberId, BytesMut), Error>.
    pub network_inputs: SparseSecondaryMap<LocationKey, Vec<(String, bool)>>,
    /// Cluster membership streams needed per location key.
    /// Maps location_key -> vec of cluster LocationKeys whose membership is needed.
    pub membership_streams: SparseSecondaryMap<LocationKey, Vec<LocationKey>>,
}

impl<'a> Deploy<'a> for EmbeddedDeploy {
    type Meta = ();
    type InstantiateEnv = EmbeddedInstantiateEnv;

    type Process = EmbeddedNode;
    type Cluster = EmbeddedNode;
    type External = EmbeddedNode;

    fn o2o_sink_source(
        env: &mut Self::InstantiateEnv,
        p1: &Self::Process,
        _p1_port: &(),
        p2: &Self::Process,
        _p2_port: &(),
        name: Option<&str>,
    ) -> (syn::Expr, syn::Expr) {
        let name = name.expect(
            "EmbeddedDeploy o2o networking requires a channel name. Use `TCP.name(\"my_channel\")` to provide one.",
        );

        let sink_ident = syn::Ident::new(&format!("__network_out_{name}"), Span::call_site());
        let source_ident = syn::Ident::new(&format!("__network_in_{name}"), Span::call_site());

        env.network_outputs
            .entry(p1.location_key)
            .unwrap()
            .or_default()
            .push((name.to_owned(), false));
        env.network_inputs
            .entry(p2.location_key)
            .unwrap()
            .or_default()
            .push((name.to_owned(), false));

        (
            syn::parse_quote!(__root_dfir_rs::sinktools::for_each(#sink_ident)),
            syn::parse_quote!(#source_ident),
        )
    }

    fn o2o_connect(
        _p1: &Self::Process,
        _p1_port: &(),
        _p2: &Self::Process,
        _p2_port: &(),
    ) -> Box<dyn FnOnce()> {
        Box::new(|| {})
    }

    fn o2m_sink_source(
        env: &mut Self::InstantiateEnv,
        p1: &Self::Process,
        _p1_port: &(),
        c2: &Self::Cluster,
        _c2_port: &(),
        name: Option<&str>,
    ) -> (syn::Expr, syn::Expr) {
        let name = name.expect("EmbeddedDeploy o2m networking requires a channel name.");
        let sink_ident = syn::Ident::new(&format!("__network_out_{name}"), Span::call_site());
        let source_ident = syn::Ident::new(&format!("__network_in_{name}"), Span::call_site());
        env.network_outputs
            .entry(p1.location_key)
            .unwrap()
            .or_default()
            .push((name.to_owned(), true));
        env.network_inputs
            .entry(c2.location_key)
            .unwrap()
            .or_default()
            .push((name.to_owned(), false));
        (
            syn::parse_quote!(__root_dfir_rs::sinktools::for_each(#sink_ident)),
            syn::parse_quote!(#source_ident),
        )
    }

    fn o2m_connect(
        _p1: &Self::Process,
        _p1_port: &(),
        _c2: &Self::Cluster,
        _c2_port: &(),
    ) -> Box<dyn FnOnce()> {
        Box::new(|| {})
    }

    fn m2o_sink_source(
        env: &mut Self::InstantiateEnv,
        c1: &Self::Cluster,
        _c1_port: &(),
        p2: &Self::Process,
        _p2_port: &(),
        name: Option<&str>,
    ) -> (syn::Expr, syn::Expr) {
        let name = name.expect("EmbeddedDeploy m2o networking requires a channel name.");
        let sink_ident = syn::Ident::new(&format!("__network_out_{name}"), Span::call_site());
        let source_ident = syn::Ident::new(&format!("__network_in_{name}"), Span::call_site());
        env.network_outputs
            .entry(c1.location_key)
            .unwrap()
            .or_default()
            .push((name.to_owned(), false));
        env.network_inputs
            .entry(p2.location_key)
            .unwrap()
            .or_default()
            .push((name.to_owned(), true));
        (
            syn::parse_quote!(__root_dfir_rs::sinktools::for_each(#sink_ident)),
            syn::parse_quote!(#source_ident),
        )
    }

    fn m2o_connect(
        _c1: &Self::Cluster,
        _c1_port: &(),
        _p2: &Self::Process,
        _p2_port: &(),
    ) -> Box<dyn FnOnce()> {
        Box::new(|| {})
    }

    fn m2m_sink_source(
        env: &mut Self::InstantiateEnv,
        c1: &Self::Cluster,
        _c1_port: &(),
        c2: &Self::Cluster,
        _c2_port: &(),
        name: Option<&str>,
    ) -> (syn::Expr, syn::Expr) {
        let name = name.expect("EmbeddedDeploy m2m networking requires a channel name.");
        let sink_ident = syn::Ident::new(&format!("__network_out_{name}"), Span::call_site());
        let source_ident = syn::Ident::new(&format!("__network_in_{name}"), Span::call_site());
        env.network_outputs
            .entry(c1.location_key)
            .unwrap()
            .or_default()
            .push((name.to_owned(), true));
        env.network_inputs
            .entry(c2.location_key)
            .unwrap()
            .or_default()
            .push((name.to_owned(), true));
        (
            syn::parse_quote!(__root_dfir_rs::sinktools::for_each(#sink_ident)),
            syn::parse_quote!(#source_ident),
        )
    }

    fn m2m_connect(
        _c1: &Self::Cluster,
        _c1_port: &(),
        _c2: &Self::Cluster,
        _c2_port: &(),
    ) -> Box<dyn FnOnce()> {
        Box::new(|| {})
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

    fn cluster_self_id() -> impl QuotedWithContext<'a, TaglessMemberId, ()> + Clone + 'a {
        super::embedded_runtime::embedded_cluster_self_id()
    }

    fn cluster_membership_stream(
        env: &mut Self::InstantiateEnv,
        at_location: &LocationId,
        location_id: &LocationId,
    ) -> impl QuotedWithContext<'a, Box<dyn Stream<Item = (TaglessMemberId, MembershipEvent)> + Unpin>, ()>
    {
        let at_key = match at_location {
            LocationId::Process(key) | LocationId::Cluster(key) => *key,
            _ => panic!("cluster_membership_stream must be called from a process or cluster"),
        };
        let cluster_key = match location_id {
            LocationId::Cluster(key) => *key,
            _ => panic!("cluster_membership_stream target must be a cluster"),
        };
        let vec = env.membership_streams.entry(at_key).unwrap().or_default();
        let idx = if let Some(pos) = vec.iter().position(|k| *k == cluster_key) {
            pos
        } else {
            vec.push(cluster_key);
            vec.len() - 1
        };

        super::embedded_runtime::embedded_cluster_membership_stream(idx)
    }

    fn register_embedded_input(
        env: &mut Self::InstantiateEnv,
        location_key: LocationKey,
        ident: &syn::Ident,
        element_type: &syn::Type,
    ) {
        env.inputs
            .entry(location_key)
            .unwrap()
            .or_default()
            .push((ident.clone(), element_type.clone()));
    }

    fn register_embedded_output(
        env: &mut Self::InstantiateEnv,
        location_key: LocationKey,
        ident: &syn::Ident,
        element_type: &syn::Type,
    ) {
        env.outputs
            .entry(location_key)
            .unwrap()
            .or_default()
            .push((ident.clone(), element_type.clone()));
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
        let mut env = EmbeddedInstantiateEnv::default();
        let compiled = self.compile_internal(&mut env);

        let root = crate::staging_util::get_this_crate();
        let orig_crate_name = quote::format_ident!("{}", crate_name.replace('-', "_"));

        let mut items: Vec<syn::Item> = Vec::new();

        // Sort location keys for deterministic output.
        let mut location_keys: Vec<_> = compiled.all_dfir().keys().collect();
        location_keys.sort();

        // Build a map from location key to fn_name for lookups.
        let fn_names: SparseSecondaryMap<LocationKey, &str> = location_keys
            .iter()
            .map(|&k| {
                let name = self
                    .processes
                    .get(k)
                    .map(|n| n.fn_name.as_str())
                    .or_else(|| self.clusters.get(k).map(|n| n.fn_name.as_str()))
                    .or_else(|| self.externals.get(k).map(|n| n.fn_name.as_str()))
                    .expect("location key not found in any node map");
                (k, name)
            })
            .collect();

        for location_key in location_keys {
            let graph = &compiled.all_dfir()[location_key];

            // Get the user-provided function name from the node.
            let fn_name = fn_names[location_key];
            let fn_ident = syn::Ident::new(fn_name, Span::call_site());

            // Get inputs for this location, sorted by name.
            let mut loc_inputs = env.inputs.get(location_key).cloned().unwrap_or_default();
            loc_inputs.sort_by(|a, b| a.0.to_string().cmp(&b.0.to_string()));

            // Get outputs for this location, sorted by name.
            let mut loc_outputs = env.outputs.get(location_key).cloned().unwrap_or_default();
            loc_outputs.sort_by(|a, b| a.0.to_string().cmp(&b.0.to_string()));

            let mut diagnostics = Diagnostics::new();
            let dfir_tokens = graph
                .as_code(&quote! { __root_dfir_rs }, true, quote!(), &mut diagnostics)
                .expect("DFIR code generation failed with diagnostics.");

            // --- Build module items (cluster info, output struct, network structs) ---
            let mut mod_items: Vec<proc_macro2::TokenStream> = Vec::new();
            let mut extra_fn_generics: Vec<proc_macro2::TokenStream> = Vec::new();
            let mut cluster_params: Vec<proc_macro2::TokenStream> = Vec::new();
            let mut output_params: Vec<proc_macro2::TokenStream> = Vec::new();
            let mut net_out_params: Vec<proc_macro2::TokenStream> = Vec::new();
            let mut net_in_params: Vec<proc_macro2::TokenStream> = Vec::new();
            let mut extra_destructure: Vec<proc_macro2::TokenStream> = Vec::new();

            // For cluster locations, add self_id parameter.
            if self.clusters.contains_key(location_key) {
                cluster_params.push(quote! {
                    __cluster_self_id: &'a #root::location::member_id::TaglessMemberId
                });
                // Alias to the name the generated DFIR code expects.
                let self_id_ident = syn::Ident::new(
                    &format!("__hydro_lang_cluster_self_id_{}", location_key),
                    Span::call_site(),
                );
                extra_destructure.push(quote! {
                    let #self_id_ident = __cluster_self_id;
                });
            }

            // For any location that needs cluster membership streams, add parameters.
            if let Some(loc_memberships) = env.membership_streams.get(location_key) {
                let membership_struct_ident =
                    syn::Ident::new("EmbeddedMembershipStreams", Span::call_site());

                let mem_generic_idents: Vec<syn::Ident> = loc_memberships
                    .iter()
                    .enumerate()
                    .map(|(i, _)| quote::format_ident!("__Mem{}", i))
                    .collect();

                let mem_field_names: Vec<syn::Ident> = loc_memberships
                    .iter()
                    .map(|k| {
                        let cluster_fn_name = fn_names[*k];
                        syn::Ident::new(cluster_fn_name, Span::call_site())
                    })
                    .collect();

                let struct_fields: Vec<proc_macro2::TokenStream> = mem_field_names
                    .iter()
                    .zip(mem_generic_idents.iter())
                    .map(|(field, generic)| {
                        quote! { pub #field: #generic }
                    })
                    .collect();

                let struct_generics: Vec<proc_macro2::TokenStream> = mem_generic_idents
                    .iter()
                    .map(|generic| {
                        quote! { #generic: __root_dfir_rs::futures::Stream<Item = (#root::location::member_id::TaglessMemberId, #root::location::MembershipEvent)> + Unpin }
                    })
                    .collect();

                for generic in &mem_generic_idents {
                    extra_fn_generics.push(
                        quote! { #generic: __root_dfir_rs::futures::Stream<Item = (#root::location::member_id::TaglessMemberId, #root::location::MembershipEvent)> + Unpin + 'a },
                    );
                }

                cluster_params.push(quote! {
                    __membership: #fn_ident::#membership_struct_ident<#(#mem_generic_idents),*>
                });

                for (i, field) in mem_field_names.iter().enumerate() {
                    let var_ident =
                        syn::Ident::new(&format!("__membership_{}", i), Span::call_site());
                    extra_destructure.push(quote! {
                        let #var_ident = __membership.#field;
                    });
                }

                mod_items.push(quote! {
                    pub struct #membership_struct_ident<#(#struct_generics),*> {
                        #(#struct_fields),*
                    }
                });
            }

            // Embedded inputs (Stream sources).
            let input_params: Vec<proc_macro2::TokenStream> = loc_inputs
                .iter()
                .map(|(ident, element_type)| {
                    quote! { #ident: impl __root_dfir_rs::futures::Stream<Item = #element_type> + Unpin + 'a }
                })
                .collect();

            // Embedded outputs (FnMut callbacks).
            if !loc_outputs.is_empty() {
                let output_struct_ident = syn::Ident::new("EmbeddedOutputs", Span::call_site());

                let output_generic_idents: Vec<syn::Ident> = loc_outputs
                    .iter()
                    .enumerate()
                    .map(|(i, _)| quote::format_ident!("__Out{}", i))
                    .collect();

                let struct_fields: Vec<proc_macro2::TokenStream> = loc_outputs
                    .iter()
                    .zip(output_generic_idents.iter())
                    .map(|((ident, _), generic)| {
                        quote! { pub #ident: #generic }
                    })
                    .collect();

                let struct_generics: Vec<proc_macro2::TokenStream> = loc_outputs
                    .iter()
                    .zip(output_generic_idents.iter())
                    .map(|((_, element_type), generic)| {
                        quote! { #generic: FnMut(#element_type) }
                    })
                    .collect();

                for ((_, element_type), generic) in
                    loc_outputs.iter().zip(output_generic_idents.iter())
                {
                    extra_fn_generics.push(quote! { #generic: FnMut(#element_type) + 'a });
                }

                output_params.push(quote! {
                    __outputs: &'a mut #fn_ident::#output_struct_ident<#(#output_generic_idents),*>
                });

                for (ident, _) in &loc_outputs {
                    extra_destructure.push(quote! { let mut #ident = &mut __outputs.#ident; });
                }

                mod_items.push(quote! {
                    pub struct #output_struct_ident<#(#struct_generics),*> {
                        #(#struct_fields),*
                    }
                });
            }

            // Network outputs (FnMut sinks).
            if let Some(mut loc_net_outputs) = env.network_outputs.remove(location_key) {
                loc_net_outputs.sort();

                let net_out_struct_ident = syn::Ident::new("EmbeddedNetworkOut", Span::call_site());

                let net_out_generic_idents: Vec<syn::Ident> = loc_net_outputs
                    .iter()
                    .enumerate()
                    .map(|(i, _)| quote::format_ident!("__NetOut{}", i))
                    .collect();

                let struct_fields: Vec<proc_macro2::TokenStream> = loc_net_outputs
                    .iter()
                    .zip(net_out_generic_idents.iter())
                    .map(|((name, _), generic)| {
                        let field_ident = syn::Ident::new(name, Span::call_site());
                        quote! { pub #field_ident: #generic }
                    })
                    .collect();

                let struct_generics: Vec<proc_macro2::TokenStream> = loc_net_outputs
                    .iter()
                    .zip(net_out_generic_idents.iter())
                    .map(|((_, is_tagged), generic)| {
                        if *is_tagged {
                            quote! { #generic: FnMut((#root::location::member_id::TaglessMemberId, #root::runtime_support::dfir_rs::bytes::Bytes)) }
                        } else {
                            quote! { #generic: FnMut(#root::runtime_support::dfir_rs::bytes::Bytes) }
                        }
                    })
                    .collect();

                for ((_, is_tagged), generic) in
                    loc_net_outputs.iter().zip(net_out_generic_idents.iter())
                {
                    if *is_tagged {
                        extra_fn_generics.push(
                            quote! { #generic: FnMut((#root::location::member_id::TaglessMemberId, #root::runtime_support::dfir_rs::bytes::Bytes)) + 'a },
                        );
                    } else {
                        extra_fn_generics.push(
                            quote! { #generic: FnMut(#root::runtime_support::dfir_rs::bytes::Bytes) + 'a },
                        );
                    }
                }

                net_out_params.push(quote! {
                    __network_out: &'a mut #fn_ident::#net_out_struct_ident<#(#net_out_generic_idents),*>
                });

                for (name, _) in &loc_net_outputs {
                    let field_ident = syn::Ident::new(name, Span::call_site());
                    let var_ident =
                        syn::Ident::new(&format!("__network_out_{name}"), Span::call_site());
                    extra_destructure
                        .push(quote! { let mut #var_ident = &mut __network_out.#field_ident; });
                }

                mod_items.push(quote! {
                    pub struct #net_out_struct_ident<#(#struct_generics),*> {
                        #(#struct_fields),*
                    }
                });
            }

            // Network inputs (Stream sources).
            if let Some(mut loc_net_inputs) = env.network_inputs.remove(location_key) {
                loc_net_inputs.sort();

                let net_in_struct_ident = syn::Ident::new("EmbeddedNetworkIn", Span::call_site());

                let net_in_generic_idents: Vec<syn::Ident> = loc_net_inputs
                    .iter()
                    .enumerate()
                    .map(|(i, _)| quote::format_ident!("__NetIn{}", i))
                    .collect();

                let struct_fields: Vec<proc_macro2::TokenStream> = loc_net_inputs
                    .iter()
                    .zip(net_in_generic_idents.iter())
                    .map(|((name, _), generic)| {
                        let field_ident = syn::Ident::new(name, Span::call_site());
                        quote! { pub #field_ident: #generic }
                    })
                    .collect();

                let struct_generics: Vec<proc_macro2::TokenStream> = loc_net_inputs
                    .iter()
                    .zip(net_in_generic_idents.iter())
                    .map(|((_, is_tagged), generic)| {
                        if *is_tagged {
                            quote! { #generic: __root_dfir_rs::futures::Stream<Item = Result<(#root::location::member_id::TaglessMemberId, __root_dfir_rs::bytes::BytesMut), std::io::Error>> + Unpin }
                        } else {
                            quote! { #generic: __root_dfir_rs::futures::Stream<Item = Result<__root_dfir_rs::bytes::BytesMut, std::io::Error>> + Unpin }
                        }
                    })
                    .collect();

                for ((_, is_tagged), generic) in
                    loc_net_inputs.iter().zip(net_in_generic_idents.iter())
                {
                    if *is_tagged {
                        extra_fn_generics.push(
                            quote! { #generic: __root_dfir_rs::futures::Stream<Item = Result<(#root::location::member_id::TaglessMemberId, __root_dfir_rs::bytes::BytesMut), std::io::Error>> + Unpin + 'a },
                        );
                    } else {
                        extra_fn_generics.push(
                            quote! { #generic: __root_dfir_rs::futures::Stream<Item = Result<__root_dfir_rs::bytes::BytesMut, std::io::Error>> + Unpin + 'a },
                        );
                    }
                }

                net_in_params.push(quote! {
                    __network_in: #fn_ident::#net_in_struct_ident<#(#net_in_generic_idents),*>
                });

                for (name, _) in &loc_net_inputs {
                    let field_ident = syn::Ident::new(name, Span::call_site());
                    let var_ident =
                        syn::Ident::new(&format!("__network_in_{name}"), Span::call_site());
                    extra_destructure.push(quote! { let #var_ident = __network_in.#field_ident; });
                }

                mod_items.push(quote! {
                    pub struct #net_in_struct_ident<#(#struct_generics),*> {
                        #(#struct_fields),*
                    }
                });
            }

            // Emit the module if there are any structs.
            if !mod_items.is_empty() {
                let output_mod: syn::Item = syn::parse_quote! {
                    pub mod #fn_ident {
                        use super::*;
                        #(#mod_items)*
                    }
                };
                items.push(output_mod);
            }

            // Build the function.
            let all_params: Vec<proc_macro2::TokenStream> = cluster_params
                .into_iter()
                .chain(input_params)
                .chain(output_params)
                .chain(net_in_params)
                .chain(net_out_params)
                .collect();

            let func = if !extra_fn_generics.is_empty() {
                syn::parse_quote! {
                    #[allow(unused, non_snake_case, clippy::suspicious_else_formatting)]
                    pub fn #fn_ident<'a, #(#extra_fn_generics),*>(#(#all_params),*) -> #root::runtime_support::dfir_rs::scheduled::graph::Dfir<'a> {
                        #(#extra_destructure)*
                        #dfir_tokens
                    }
                }
            } else {
                syn::parse_quote! {
                    #[allow(unused, non_snake_case, clippy::suspicious_else_formatting)]
                    pub fn #fn_ident<'a>(#(#all_params),*) -> #root::runtime_support::dfir_rs::scheduled::graph::Dfir<'a> {
                        #dfir_tokens
                    }
                }
            };

            items.push(func);
        }

        syn::parse_quote! {
            use #orig_crate_name::__staged::__deps::*;
            use #root::prelude::*;
            use #root::runtime_support::dfir_rs as __root_dfir_rs;
            pub use #orig_crate_name::__staged;

            #( #items )*
        }
    }
}
