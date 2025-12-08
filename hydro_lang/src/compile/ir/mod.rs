use core::panic;
use std::cell::RefCell;
#[cfg(feature = "build")]
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::rc::Rc;

#[cfg(feature = "build")]
use dfir_lang::graph::FlatGraphBuilder;
#[cfg(feature = "build")]
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::ToTokens;
#[cfg(feature = "build")]
use quote::quote;
#[cfg(feature = "build")]
use syn::parse_quote;
use syn::visit::{self, Visit};
use syn::visit_mut::VisitMut;

#[cfg(feature = "build")]
use crate::compile::deploy_provider::{Deploy, RegisterPort};
use crate::location::NetworkHint;
use crate::location::dynamic::LocationId;

pub mod backtrace;
use backtrace::Backtrace;

/// Wrapper that displays only the tokens of a parsed expr.
///
/// Boxes `syn::Type` which is ~240 bytes.
#[derive(Clone, Hash)]
pub struct DebugExpr(pub Box<syn::Expr>);

impl From<syn::Expr> for DebugExpr {
    fn from(expr: syn::Expr) -> Self {
        Self(Box::new(expr))
    }
}

impl Deref for DebugExpr {
    type Target = syn::Expr;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ToTokens for DebugExpr {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.0.to_tokens(tokens);
    }
}

impl Debug for DebugExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.to_token_stream())
    }
}

impl Display for DebugExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let original = self.0.as_ref().clone();
        let simplified = simplify_q_macro(original);

        // For now, just use quote formatting without trying to parse as a statement
        // This avoids the syn::parse_quote! issues entirely
        write!(f, "q!({})", quote::quote!(#simplified))
    }
}

/// Simplify expanded q! macro calls back to q!(...) syntax for better readability
fn simplify_q_macro(mut expr: syn::Expr) -> syn::Expr {
    // Try to parse the token string as a syn::Expr
    // Use a visitor to simplify q! macro expansions
    let mut simplifier = QMacroSimplifier::new();
    simplifier.visit_expr_mut(&mut expr);

    // If we found and simplified a q! macro, return the simplified version
    if let Some(simplified) = simplifier.simplified_result {
        simplified
    } else {
        expr
    }
}

/// AST visitor that simplifies q! macro expansions
#[derive(Default)]
pub struct QMacroSimplifier {
    pub simplified_result: Option<syn::Expr>,
}

impl QMacroSimplifier {
    pub fn new() -> Self {
        Self::default()
    }
}

impl VisitMut for QMacroSimplifier {
    fn visit_expr_mut(&mut self, expr: &mut syn::Expr) {
        // Check if we already found a result to avoid further processing
        if self.simplified_result.is_some() {
            return;
        }

        if let syn::Expr::Call(call) = expr && let syn::Expr::Path(path_expr) = call.func.as_ref()
            // Look for calls to stageleft::runtime_support::fn*
            && self.is_stageleft_runtime_support_call(&path_expr.path)
            // Try to extract the closure from the arguments
            && let Some(closure) = self.extract_closure_from_args(&call.args)
        {
            self.simplified_result = Some(closure);
            return;
        }

        // Continue visiting child expressions using the default implementation
        // Use the default visitor to avoid infinite recursion
        syn::visit_mut::visit_expr_mut(self, expr);
    }
}

impl QMacroSimplifier {
    fn is_stageleft_runtime_support_call(&self, path: &syn::Path) -> bool {
        // Check if this is a call to stageleft::runtime_support::fn*
        if let Some(last_segment) = path.segments.last() {
            let fn_name = last_segment.ident.to_string();
            // if fn_name.starts_with("fn") && fn_name.contains("_expr") {
            fn_name.contains("_type_hint")
                && path.segments.len() > 2
                && path.segments[0].ident == "stageleft"
                && path.segments[1].ident == "runtime_support"
        } else {
            false
        }
    }

    fn extract_closure_from_args(
        &self,
        args: &syn::punctuated::Punctuated<syn::Expr, syn::Token![,]>,
    ) -> Option<syn::Expr> {
        // Look through the arguments for a closure expression
        for arg in args {
            if let syn::Expr::Closure(_) = arg {
                return Some(arg.clone());
            }
            // Also check for closures nested in other expressions (like blocks)
            if let Some(closure_expr) = self.find_closure_in_expr(arg) {
                return Some(closure_expr);
            }
        }
        None
    }

    fn find_closure_in_expr(&self, expr: &syn::Expr) -> Option<syn::Expr> {
        let mut visitor = ClosureFinder {
            found_closure: None,
            prefer_inner_blocks: true,
        };
        visitor.visit_expr(expr);
        visitor.found_closure
    }
}

/// Visitor that finds closures in expressions with special block handling
struct ClosureFinder {
    found_closure: Option<syn::Expr>,
    prefer_inner_blocks: bool,
}

impl<'ast> Visit<'ast> for ClosureFinder {
    fn visit_expr(&mut self, expr: &'ast syn::Expr) {
        // If we already found a closure, don't continue searching
        if self.found_closure.is_some() {
            return;
        }

        match expr {
            syn::Expr::Closure(_) => {
                self.found_closure = Some(expr.clone());
            }
            syn::Expr::Block(block) if self.prefer_inner_blocks => {
                // Special handling for blocks - look for inner blocks that contain closures
                for stmt in &block.block.stmts {
                    if let syn::Stmt::Expr(stmt_expr, _) = stmt
                        && let syn::Expr::Block(_) = stmt_expr
                    {
                        // Check if this nested block contains a closure
                        let mut inner_visitor = ClosureFinder {
                            found_closure: None,
                            prefer_inner_blocks: false, // Avoid infinite recursion
                        };
                        inner_visitor.visit_expr(stmt_expr);
                        if inner_visitor.found_closure.is_some() {
                            // Found a closure in an inner block, return that block
                            self.found_closure = Some(stmt_expr.clone());
                            return;
                        }
                    }
                }

                // If no inner block with closure found, continue with normal visitation
                visit::visit_expr(self, expr);

                // If we found a closure, just return the closure itself, not the whole block
                // unless we're in the special case where we want the containing block
                if self.found_closure.is_some() {
                    // The closure was found during visitation, no need to wrap in block
                }
            }
            _ => {
                // Use default visitor behavior for all other expressions
                visit::visit_expr(self, expr);
            }
        }
    }
}

/// Debug displays the type's tokens.
///
/// Boxes `syn::Type` which is ~320 bytes.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct DebugType(pub Box<syn::Type>);

impl From<syn::Type> for DebugType {
    fn from(t: syn::Type) -> Self {
        Self(Box::new(t))
    }
}

impl Deref for DebugType {
    type Target = syn::Type;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ToTokens for DebugType {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.0.to_tokens(tokens);
    }
}

impl Debug for DebugType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.to_token_stream())
    }
}

pub enum DebugInstantiate {
    Building,
    Finalized(Box<DebugInstantiateFinalized>),
}

#[cfg_attr(
    not(feature = "build"),
    expect(
        dead_code,
        reason = "sink, source unused without `feature = \"build\"`."
    )
)]
pub struct DebugInstantiateFinalized {
    sink: syn::Expr,
    source: syn::Expr,
    connect_fn: Option<Box<dyn FnOnce()>>,
}

impl From<DebugInstantiateFinalized> for DebugInstantiate {
    fn from(f: DebugInstantiateFinalized) -> Self {
        Self::Finalized(Box::new(f))
    }
}

impl Debug for DebugInstantiate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<network instantiate>")
    }
}

impl Hash for DebugInstantiate {
    fn hash<H: Hasher>(&self, _state: &mut H) {
        // Do nothing
    }
}

impl Clone for DebugInstantiate {
    fn clone(&self) -> Self {
        match self {
            DebugInstantiate::Building => DebugInstantiate::Building,
            DebugInstantiate::Finalized(_) => {
                panic!("DebugInstantiate::Finalized should not be cloned")
            }
        }
    }
}

/// A source in a Hydro graph, where data enters the graph.
#[derive(Debug, Hash, Clone)]
pub enum HydroSource {
    Stream(DebugExpr),
    ExternalNetwork(),
    Iter(DebugExpr),
    Spin(),
    ClusterMembers(LocationId),
}

#[cfg(feature = "build")]
/// A trait that abstracts over elements of DFIR code-gen that differ between production deployment
/// and simulations.
///
/// In particular, this lets the simulator fuse together all locations into one DFIR graph, spit
/// out separate graphs for each tick, and emit hooks for controlling non-deterministic operators.
pub trait DfirBuilder {
    /// Whether the representation of singletons should include intermediate states.
    fn singleton_intermediates(&self) -> bool;

    /// Gets the DFIR builder for the given location, creating it if necessary.
    fn get_dfir_mut(&mut self, location: &LocationId) -> &mut FlatGraphBuilder;

    fn batch(
        &mut self,
        in_ident: syn::Ident,
        in_location: &LocationId,
        in_kind: &CollectionKind,
        out_ident: &syn::Ident,
        out_location: &LocationId,
        op_meta: &HydroIrOpMetadata,
    );
    fn yield_from_tick(
        &mut self,
        in_ident: syn::Ident,
        in_location: &LocationId,
        in_kind: &CollectionKind,
        out_ident: &syn::Ident,
        out_location: &LocationId,
    );

    fn begin_atomic(
        &mut self,
        in_ident: syn::Ident,
        in_location: &LocationId,
        in_kind: &CollectionKind,
        out_ident: &syn::Ident,
        out_location: &LocationId,
        op_meta: &HydroIrOpMetadata,
    );
    fn end_atomic(
        &mut self,
        in_ident: syn::Ident,
        in_location: &LocationId,
        in_kind: &CollectionKind,
        out_ident: &syn::Ident,
    );

    #[expect(clippy::too_many_arguments, reason = "TODO // internal")]
    fn observe_nondet(
        &mut self,
        trusted: bool,
        location: &LocationId,
        in_ident: syn::Ident,
        in_kind: &CollectionKind,
        out_ident: &syn::Ident,
        out_kind: &CollectionKind,
        op_meta: &HydroIrOpMetadata,
    );

    #[expect(clippy::too_many_arguments, reason = "TODO")]
    fn create_network(
        &mut self,
        from: &LocationId,
        to: &LocationId,
        input_ident: syn::Ident,
        out_ident: &syn::Ident,
        serialize: &Option<DebugExpr>,
        sink: syn::Expr,
        source: syn::Expr,
        deserialize: &Option<DebugExpr>,
        tag_id: usize,
    );

    fn create_external_source(
        &mut self,
        on: &LocationId,
        source_expr: syn::Expr,
        out_ident: &syn::Ident,
        deserialize: &Option<DebugExpr>,
        tag_id: usize,
    );

    fn create_external_output(
        &mut self,
        on: &LocationId,
        sink_expr: syn::Expr,
        input_ident: &syn::Ident,
        serialize: &Option<DebugExpr>,
        tag_id: usize,
    );
}

#[cfg(feature = "build")]
impl DfirBuilder for BTreeMap<usize, FlatGraphBuilder> {
    fn singleton_intermediates(&self) -> bool {
        false
    }

    fn get_dfir_mut(&mut self, location: &LocationId) -> &mut FlatGraphBuilder {
        self.entry(location.root().raw_id()).or_default()
    }

    fn batch(
        &mut self,
        in_ident: syn::Ident,
        in_location: &LocationId,
        _in_kind: &CollectionKind,
        out_ident: &syn::Ident,
        _out_location: &LocationId,
        _op_meta: &HydroIrOpMetadata,
    ) {
        let builder = self.get_dfir_mut(in_location.root());
        builder.add_dfir(
            parse_quote! {
                #out_ident = #in_ident;
            },
            None,
            None,
        );
    }

    fn yield_from_tick(
        &mut self,
        in_ident: syn::Ident,
        in_location: &LocationId,
        _in_kind: &CollectionKind,
        out_ident: &syn::Ident,
        _out_location: &LocationId,
    ) {
        let builder = self.get_dfir_mut(in_location.root());
        builder.add_dfir(
            parse_quote! {
                #out_ident = #in_ident;
            },
            None,
            None,
        );
    }

    fn begin_atomic(
        &mut self,
        in_ident: syn::Ident,
        in_location: &LocationId,
        _in_kind: &CollectionKind,
        out_ident: &syn::Ident,
        _out_location: &LocationId,
        _op_meta: &HydroIrOpMetadata,
    ) {
        let builder = self.get_dfir_mut(in_location.root());
        builder.add_dfir(
            parse_quote! {
                #out_ident = #in_ident;
            },
            None,
            None,
        );
    }

    fn end_atomic(
        &mut self,
        in_ident: syn::Ident,
        in_location: &LocationId,
        _in_kind: &CollectionKind,
        out_ident: &syn::Ident,
    ) {
        let builder = self.get_dfir_mut(in_location.root());
        builder.add_dfir(
            parse_quote! {
                #out_ident = #in_ident;
            },
            None,
            None,
        );
    }

    fn observe_nondet(
        &mut self,
        _trusted: bool,
        location: &LocationId,
        in_ident: syn::Ident,
        _in_kind: &CollectionKind,
        out_ident: &syn::Ident,
        _out_kind: &CollectionKind,
        _op_meta: &HydroIrOpMetadata,
    ) {
        let builder = self.get_dfir_mut(location);
        builder.add_dfir(
            parse_quote! {
                #out_ident = #in_ident;
            },
            None,
            None,
        );
    }

    fn create_network(
        &mut self,
        from: &LocationId,
        to: &LocationId,
        input_ident: syn::Ident,
        out_ident: &syn::Ident,
        serialize: &Option<DebugExpr>,
        sink: syn::Expr,
        source: syn::Expr,
        deserialize: &Option<DebugExpr>,
        tag_id: usize,
    ) {
        let sender_builder = self.get_dfir_mut(from);
        if let Some(serialize_pipeline) = serialize {
            sender_builder.add_dfir(
                parse_quote! {
                    #input_ident -> map(#serialize_pipeline) -> dest_sink(#sink);
                },
                None,
                // operator tag separates send and receive, which otherwise have the same next_stmt_id
                Some(&format!("send{}", tag_id)),
            );
        } else {
            sender_builder.add_dfir(
                parse_quote! {
                    #input_ident -> dest_sink(#sink);
                },
                None,
                Some(&format!("send{}", tag_id)),
            );
        }

        let receiver_builder = self.get_dfir_mut(to);
        if let Some(deserialize_pipeline) = deserialize {
            receiver_builder.add_dfir(
                parse_quote! {
                    #out_ident = source_stream(#source) -> map(#deserialize_pipeline);
                },
                None,
                Some(&format!("recv{}", tag_id)),
            );
        } else {
            receiver_builder.add_dfir(
                parse_quote! {
                    #out_ident = source_stream(#source);
                },
                None,
                Some(&format!("recv{}", tag_id)),
            );
        }
    }

    fn create_external_source(
        &mut self,
        on: &LocationId,
        source_expr: syn::Expr,
        out_ident: &syn::Ident,
        deserialize: &Option<DebugExpr>,
        tag_id: usize,
    ) {
        let receiver_builder = self.get_dfir_mut(on);
        if let Some(deserialize_pipeline) = deserialize {
            receiver_builder.add_dfir(
                parse_quote! {
                    #out_ident = source_stream(#source_expr) -> map(#deserialize_pipeline);
                },
                None,
                Some(&format!("recv{}", tag_id)),
            );
        } else {
            receiver_builder.add_dfir(
                parse_quote! {
                    #out_ident = source_stream(#source_expr);
                },
                None,
                Some(&format!("recv{}", tag_id)),
            );
        }
    }

    fn create_external_output(
        &mut self,
        on: &LocationId,
        sink_expr: syn::Expr,
        input_ident: &syn::Ident,
        serialize: &Option<DebugExpr>,
        tag_id: usize,
    ) {
        let sender_builder = self.get_dfir_mut(on);
        if let Some(serialize_fn) = serialize {
            sender_builder.add_dfir(
                parse_quote! {
                    #input_ident -> map(#serialize_fn) -> dest_sink(#sink_expr);
                },
                None,
                // operator tag separates send and receive, which otherwise have the same next_stmt_id
                Some(&format!("send{}", tag_id)),
            );
        } else {
            sender_builder.add_dfir(
                parse_quote! {
                    #input_ident -> dest_sink(#sink_expr);
                },
                None,
                Some(&format!("send{}", tag_id)),
            );
        }
    }
}

#[cfg(feature = "build")]
pub enum BuildersOrCallback<'a, L, N>
where
    L: FnMut(&mut HydroRoot, &mut usize),
    N: FnMut(&mut HydroNode, &mut usize),
{
    Builders(&'a mut dyn DfirBuilder),
    Callback(L, N),
}

/// An root in a Hydro graph, which is an pipeline that doesn't emit
/// any downstream values. Traversals over the dataflow graph and
/// generating DFIR IR start from roots.
#[derive(Debug, Hash)]
pub enum HydroRoot {
    ForEach {
        f: DebugExpr,
        input: Box<HydroNode>,
        op_metadata: HydroIrOpMetadata,
    },
    SendExternal {
        to_external_id: usize,
        to_key: usize,
        to_many: bool,
        unpaired: bool,
        serialize_fn: Option<DebugExpr>,
        instantiate_fn: DebugInstantiate,
        input: Box<HydroNode>,
        op_metadata: HydroIrOpMetadata,
    },
    DestSink {
        sink: DebugExpr,
        input: Box<HydroNode>,
        op_metadata: HydroIrOpMetadata,
    },
    CycleSink {
        ident: syn::Ident,
        input: Box<HydroNode>,
        op_metadata: HydroIrOpMetadata,
    },
}

impl HydroRoot {
    #[cfg(feature = "build")]
    pub fn compile_network<'a, D>(
        &mut self,
        extra_stmts: &mut BTreeMap<usize, Vec<syn::Stmt>>,
        seen_tees: &mut SeenTees,
        processes: &HashMap<usize, D::Process>,
        clusters: &HashMap<usize, D::Cluster>,
        externals: &HashMap<usize, D::External>,
    ) where
        D: Deploy<'a>,
    {
        let refcell_extra_stmts = RefCell::new(extra_stmts);
        self.transform_bottom_up(
            &mut |l| {
                if let HydroRoot::SendExternal {
                    input,
                    to_external_id,
                    to_key,
                    to_many,
                    unpaired,
                    instantiate_fn,
                    ..
                } = l
                {
                    let ((sink_expr, source_expr), connect_fn) = match instantiate_fn {
                        DebugInstantiate::Building => {
                            let to_node = externals
                                .get(to_external_id)
                                .unwrap_or_else(|| {
                                    panic!("A external used in the graph was not instantiated: {}", to_external_id)
                                })
                                .clone();

                            match input.metadata().location_kind.root() {
                                LocationId::Process(process_id) => {
                                    if *to_many {
                                        (
                                            (
                                                D::e2o_many_sink(format!("{}_{}", *to_external_id, *to_key)),
                                                parse_quote!(DUMMY),
                                            ),
                                            Box::new(|| {}) as Box<dyn FnOnce()>,
                                        )
                                    } else {
                                        let from_node = processes
                                            .get(process_id)
                                            .unwrap_or_else(|| {
                                                panic!("A process used in the graph was not instantiated: {}", process_id)
                                            })
                                            .clone();

                                        let sink_port = D::allocate_process_port(&from_node);
                                        let source_port: <D as Deploy<'a>>::Port = D::allocate_external_port(&to_node);

                                        if *unpaired {
                                            use stageleft::quote_type;
                                            use tokio_util::codec::LengthDelimitedCodec;

                                            to_node.register(*to_key, source_port.clone());

                                            let _ = D::e2o_source(
                                                refcell_extra_stmts.borrow_mut().entry(*process_id).or_default(),
                                                &to_node, &source_port,
                                                &from_node, &sink_port,
                                                &quote_type::<LengthDelimitedCodec>(),
                                                format!("{}_{}", *to_external_id, *to_key)
                                            );
                                        }

                                        (
                                            (
                                                D::o2e_sink(
                                                    &from_node,
                                                    &sink_port,
                                                    &to_node,
                                                    &source_port,
                                                    format!("{}_{}", *to_external_id, *to_key)
                                                ),
                                                parse_quote!(DUMMY),
                                            ),
                                            if *unpaired {
                                                D::e2o_connect(
                                                    &to_node,
                                                    &source_port,
                                                    &from_node,
                                                    &sink_port,
                                                    *to_many,
                                                    NetworkHint::Auto,
                                                )
                                            } else {
                                                Box::new(|| {}) as Box<dyn FnOnce()>
                                            },
                                        )
                                    }
                                }
                                LocationId::Cluster(_) => todo!(),
                                _ => panic!()
                            }
                        },

                        DebugInstantiate::Finalized(_) => panic!("network already finalized"),
                    };

                    *instantiate_fn = DebugInstantiateFinalized {
                        sink: sink_expr,
                        source: source_expr,
                        connect_fn: Some(connect_fn),
                    }
                    .into();
                }
            },
            &mut |n| {
                if let HydroNode::Network {
                    input,
                    instantiate_fn,
                    metadata,
                    ..
                } = n
                {
                    let (sink_expr, source_expr, connect_fn) = match instantiate_fn {
                        DebugInstantiate::Building => instantiate_network::<D>(
                            input.metadata().location_kind.root(),
                            metadata.location_kind.root(),
                            processes,
                            clusters,
                        ),

                        DebugInstantiate::Finalized(_) => panic!("network already finalized"),
                    };

                    *instantiate_fn = DebugInstantiateFinalized {
                        sink: sink_expr,
                        source: source_expr,
                        connect_fn: Some(connect_fn),
                    }
                    .into();
                } else if let HydroNode::ExternalInput {
                    from_external_id,
                    from_key,
                    from_many,
                    codec_type,
                    port_hint,
                    instantiate_fn,
                    metadata,
                    ..
                } = n
                {
                    let ((sink_expr, source_expr), connect_fn) = match instantiate_fn {
                        DebugInstantiate::Building => {
                            let from_node = externals
                                .get(from_external_id)
                                .unwrap_or_else(|| {
                                    panic!(
                                        "A external used in the graph was not instantiated: {}",
                                        from_external_id
                                    )
                                })
                                .clone();

                            match metadata.location_kind.root() {
                                LocationId::Process(process_id) => {
                                    let to_node = processes
                                        .get(process_id)
                                        .unwrap_or_else(|| {
                                            panic!("A process used in the graph was not instantiated: {}", process_id)
                                        })
                                        .clone();

                                    let sink_port = D::allocate_external_port(&from_node);
                                    let source_port = D::allocate_process_port(&to_node);

                                    from_node.register(*from_key, sink_port.clone());

                                    (
                                        (
                                            parse_quote!(DUMMY),
                                            if *from_many {
                                                D::e2o_many_source(
                                                    refcell_extra_stmts.borrow_mut().entry(*process_id).or_default(),
                                                    &to_node, &source_port,
                                                    codec_type.0.as_ref(),
                                                    format!("{}_{}", *from_external_id, *from_key)
                                                )
                                            } else {
                                                D::e2o_source(
                                                    refcell_extra_stmts.borrow_mut().entry(*process_id).or_default(),
                                                    &from_node, &sink_port,
                                                    &to_node, &source_port,
                                                    codec_type.0.as_ref(),
                                                    format!("{}_{}", *from_external_id, *from_key)
                                                )
                                            },
                                        ),
                                        D::e2o_connect(&from_node, &sink_port, &to_node, &source_port, *from_many, *port_hint),
                                    )
                                }
                                LocationId::Cluster(_) => todo!(),
                                _ => panic!()
                            }
                        },

                        DebugInstantiate::Finalized(_) => panic!("network already finalized"),
                    };

                    *instantiate_fn = DebugInstantiateFinalized {
                        sink: sink_expr,
                        source: source_expr,
                        connect_fn: Some(connect_fn),
                    }
                    .into();
                }
            },
            seen_tees,
            false,
        );
    }

    pub fn connect_network(&mut self, seen_tees: &mut SeenTees) {
        self.transform_bottom_up(
            &mut |l| {
                if let HydroRoot::SendExternal { instantiate_fn, .. } = l {
                    match instantiate_fn {
                        DebugInstantiate::Building => panic!("network not built"),

                        DebugInstantiate::Finalized(finalized) => {
                            (finalized.connect_fn.take().unwrap())();
                        }
                    }
                }
            },
            &mut |n| {
                if let HydroNode::Network { instantiate_fn, .. }
                | HydroNode::ExternalInput { instantiate_fn, .. } = n
                {
                    match instantiate_fn {
                        DebugInstantiate::Building => panic!("network not built"),

                        DebugInstantiate::Finalized(finalized) => {
                            (finalized.connect_fn.take().unwrap())();
                        }
                    }
                }
            },
            seen_tees,
            false,
        );
    }

    pub fn transform_bottom_up(
        &mut self,
        transform_root: &mut impl FnMut(&mut HydroRoot),
        transform_node: &mut impl FnMut(&mut HydroNode),
        seen_tees: &mut SeenTees,
        check_well_formed: bool,
    ) {
        self.transform_children(
            |n, s| n.transform_bottom_up(transform_node, s, check_well_formed),
            seen_tees,
        );

        transform_root(self);
    }

    pub fn transform_children(
        &mut self,
        mut transform: impl FnMut(&mut HydroNode, &mut SeenTees),
        seen_tees: &mut SeenTees,
    ) {
        match self {
            HydroRoot::ForEach { input, .. }
            | HydroRoot::SendExternal { input, .. }
            | HydroRoot::DestSink { input, .. }
            | HydroRoot::CycleSink { input, .. } => {
                transform(input, seen_tees);
            }
        }
    }

    pub fn deep_clone(&self, seen_tees: &mut SeenTees) -> HydroRoot {
        match self {
            HydroRoot::ForEach {
                f,
                input,
                op_metadata,
            } => HydroRoot::ForEach {
                f: f.clone(),
                input: Box::new(input.deep_clone(seen_tees)),
                op_metadata: op_metadata.clone(),
            },
            HydroRoot::SendExternal {
                to_external_id,
                to_key,
                to_many,
                unpaired,
                serialize_fn,
                instantiate_fn,
                input,
                op_metadata,
            } => HydroRoot::SendExternal {
                to_external_id: *to_external_id,
                to_key: *to_key,
                to_many: *to_many,
                unpaired: *unpaired,
                serialize_fn: serialize_fn.clone(),
                instantiate_fn: instantiate_fn.clone(),
                input: Box::new(input.deep_clone(seen_tees)),
                op_metadata: op_metadata.clone(),
            },
            HydroRoot::DestSink {
                sink,
                input,
                op_metadata,
            } => HydroRoot::DestSink {
                sink: sink.clone(),
                input: Box::new(input.deep_clone(seen_tees)),
                op_metadata: op_metadata.clone(),
            },
            HydroRoot::CycleSink {
                ident,
                input,
                op_metadata,
            } => HydroRoot::CycleSink {
                ident: ident.clone(),
                input: Box::new(input.deep_clone(seen_tees)),
                op_metadata: op_metadata.clone(),
            },
        }
    }

    #[cfg(feature = "build")]
    pub fn emit<'a, D: Deploy<'a>>(
        &mut self,
        graph_builders: &mut dyn DfirBuilder,
        built_tees: &mut HashMap<*const RefCell<HydroNode>, syn::Ident>,
        next_stmt_id: &mut usize,
    ) {
        self.emit_core::<D>(
            &mut BuildersOrCallback::Builders::<
                fn(&mut HydroRoot, &mut usize),
                fn(&mut HydroNode, &mut usize),
            >(graph_builders),
            built_tees,
            next_stmt_id,
        );
    }

    #[cfg(feature = "build")]
    pub fn emit_core<'a, D: Deploy<'a>>(
        &mut self,
        builders_or_callback: &mut BuildersOrCallback<
            impl FnMut(&mut HydroRoot, &mut usize),
            impl FnMut(&mut HydroNode, &mut usize),
        >,
        built_tees: &mut HashMap<*const RefCell<HydroNode>, syn::Ident>,
        next_stmt_id: &mut usize,
    ) {
        match self {
            HydroRoot::ForEach { f, input, .. } => {
                let input_ident =
                    input.emit_core::<D>(builders_or_callback, built_tees, next_stmt_id);

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        graph_builders
                            .get_dfir_mut(&input.metadata().location_kind)
                            .add_dfir(
                                parse_quote! {
                                    #input_ident -> for_each(#f);
                                },
                                None,
                                Some(&next_stmt_id.to_string()),
                            );
                    }
                    BuildersOrCallback::Callback(leaf_callback, _) => {
                        leaf_callback(self, next_stmt_id);
                    }
                }

                *next_stmt_id += 1;
            }

            HydroRoot::SendExternal {
                serialize_fn,
                instantiate_fn,
                input,
                ..
            } => {
                let input_ident =
                    input.emit_core::<D>(builders_or_callback, built_tees, next_stmt_id);

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        let (sink_expr, _) = match instantiate_fn {
                            DebugInstantiate::Building => (
                                syn::parse_quote!(DUMMY_SINK),
                                syn::parse_quote!(DUMMY_SOURCE),
                            ),

                            DebugInstantiate::Finalized(finalized) => {
                                (finalized.sink.clone(), finalized.source.clone())
                            }
                        };

                        graph_builders.create_external_output(
                            &input.metadata().location_kind,
                            sink_expr,
                            &input_ident,
                            serialize_fn,
                            *next_stmt_id,
                        );
                    }
                    BuildersOrCallback::Callback(leaf_callback, _) => {
                        leaf_callback(self, next_stmt_id);
                    }
                }

                *next_stmt_id += 1;
            }

            HydroRoot::DestSink { sink, input, .. } => {
                let input_ident =
                    input.emit_core::<D>(builders_or_callback, built_tees, next_stmt_id);

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        graph_builders
                            .get_dfir_mut(&input.metadata().location_kind)
                            .add_dfir(
                                parse_quote! {
                                    #input_ident -> dest_sink(#sink);
                                },
                                None,
                                Some(&next_stmt_id.to_string()),
                            );
                    }
                    BuildersOrCallback::Callback(leaf_callback, _) => {
                        leaf_callback(self, next_stmt_id);
                    }
                }

                *next_stmt_id += 1;
            }

            HydroRoot::CycleSink { ident, input, .. } => {
                let input_ident =
                    input.emit_core::<D>(builders_or_callback, built_tees, next_stmt_id);

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        graph_builders
                            .get_dfir_mut(&input.metadata().location_kind)
                            .add_dfir(
                                parse_quote! {
                                    #ident = #input_ident;
                                },
                                None,
                                None,
                            );
                    }
                    // No ID, no callback
                    BuildersOrCallback::Callback(_, _) => {}
                }
            }
        }
    }

    pub fn op_metadata(&self) -> &HydroIrOpMetadata {
        match self {
            HydroRoot::ForEach { op_metadata, .. }
            | HydroRoot::SendExternal { op_metadata, .. }
            | HydroRoot::DestSink { op_metadata, .. }
            | HydroRoot::CycleSink { op_metadata, .. } => op_metadata,
        }
    }

    pub fn op_metadata_mut(&mut self) -> &mut HydroIrOpMetadata {
        match self {
            HydroRoot::ForEach { op_metadata, .. }
            | HydroRoot::SendExternal { op_metadata, .. }
            | HydroRoot::DestSink { op_metadata, .. }
            | HydroRoot::CycleSink { op_metadata, .. } => op_metadata,
        }
    }

    pub fn input(&self) -> &HydroNode {
        match self {
            HydroRoot::ForEach { input, .. }
            | HydroRoot::SendExternal { input, .. }
            | HydroRoot::DestSink { input, .. }
            | HydroRoot::CycleSink { input, .. } => input,
        }
    }

    pub fn input_metadata(&self) -> &HydroIrMetadata {
        self.input().metadata()
    }

    pub fn print_root(&self) -> String {
        match self {
            HydroRoot::ForEach { f, .. } => format!("ForEach({:?})", f),
            HydroRoot::SendExternal { .. } => "SendExternal".to_string(),
            HydroRoot::DestSink { sink, .. } => format!("DestSink({:?})", sink),
            HydroRoot::CycleSink { ident, .. } => format!("CycleSink({:?})", ident),
        }
    }

    pub fn visit_debug_expr(&mut self, mut transform: impl FnMut(&mut DebugExpr)) {
        match self {
            HydroRoot::ForEach { f, .. } | HydroRoot::DestSink { sink: f, .. } => {
                transform(f);
            }
            HydroRoot::SendExternal { .. } | HydroRoot::CycleSink { .. } => {}
        }
    }
}

#[cfg(feature = "build")]
pub fn emit<'a, D: Deploy<'a>>(ir: &mut Vec<HydroRoot>) -> BTreeMap<usize, FlatGraphBuilder> {
    let mut builders = BTreeMap::new();
    let mut built_tees = HashMap::new();
    let mut next_stmt_id = 0;
    for leaf in ir {
        leaf.emit::<D>(&mut builders, &mut built_tees, &mut next_stmt_id);
    }
    builders
}

#[cfg(feature = "build")]
pub fn traverse_dfir<'a, D: Deploy<'a>>(
    ir: &mut [HydroRoot],
    transform_root: impl FnMut(&mut HydroRoot, &mut usize),
    transform_node: impl FnMut(&mut HydroNode, &mut usize),
) {
    let mut seen_tees = HashMap::new();
    let mut next_stmt_id = 0;
    let mut callback = BuildersOrCallback::Callback(transform_root, transform_node);
    ir.iter_mut().for_each(|leaf| {
        leaf.emit_core::<D>(&mut callback, &mut seen_tees, &mut next_stmt_id);
    });
}

pub fn transform_bottom_up(
    ir: &mut [HydroRoot],
    transform_root: &mut impl FnMut(&mut HydroRoot),
    transform_node: &mut impl FnMut(&mut HydroNode),
    check_well_formed: bool,
) {
    let mut seen_tees = HashMap::new();
    ir.iter_mut().for_each(|leaf| {
        leaf.transform_bottom_up(
            transform_root,
            transform_node,
            &mut seen_tees,
            check_well_formed,
        );
    });
}

pub fn deep_clone(ir: &[HydroRoot]) -> Vec<HydroRoot> {
    let mut seen_tees = HashMap::new();
    ir.iter()
        .map(|leaf| leaf.deep_clone(&mut seen_tees))
        .collect()
}

type PrintedTees = RefCell<Option<(usize, HashMap<*const RefCell<HydroNode>, usize>)>>;
thread_local! {
    static PRINTED_TEES: PrintedTees = const { RefCell::new(None) };
}

pub fn dbg_dedup_tee<T>(f: impl FnOnce() -> T) -> T {
    PRINTED_TEES.with(|printed_tees| {
        let mut printed_tees_mut = printed_tees.borrow_mut();
        *printed_tees_mut = Some((0, HashMap::new()));
        drop(printed_tees_mut);

        let ret = f();

        let mut printed_tees_mut = printed_tees.borrow_mut();
        *printed_tees_mut = None;

        ret
    })
}

pub struct TeeNode(pub Rc<RefCell<HydroNode>>);

impl TeeNode {
    pub fn as_ptr(&self) -> *const RefCell<HydroNode> {
        Rc::as_ptr(&self.0)
    }
}

impl Debug for TeeNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        PRINTED_TEES.with(|printed_tees| {
            let mut printed_tees_mut_borrow = printed_tees.borrow_mut();
            let printed_tees_mut = printed_tees_mut_borrow.as_mut();

            if let Some(printed_tees_mut) = printed_tees_mut {
                if let Some(existing) = printed_tees_mut
                    .1
                    .get(&(self.0.as_ref() as *const RefCell<HydroNode>))
                {
                    write!(f, "<tee {}>", existing)
                } else {
                    let next_id = printed_tees_mut.0;
                    printed_tees_mut.0 += 1;
                    printed_tees_mut
                        .1
                        .insert(self.0.as_ref() as *const RefCell<HydroNode>, next_id);
                    drop(printed_tees_mut_borrow);
                    write!(f, "<tee {}>: ", next_id)?;
                    Debug::fmt(&self.0.borrow(), f)
                }
            } else {
                drop(printed_tees_mut_borrow);
                write!(f, "<tee>: ")?;
                Debug::fmt(&self.0.borrow(), f)
            }
        })
    }
}

impl Hash for TeeNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.borrow_mut().hash(state);
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum BoundKind {
    Unbounded,
    Bounded,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum StreamOrder {
    NoOrder,
    TotalOrder,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum StreamRetry {
    AtLeastOnce,
    ExactlyOnce,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum KeyedSingletonBoundKind {
    Unbounded,
    BoundedValue,
    Bounded,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum CollectionKind {
    Stream {
        bound: BoundKind,
        order: StreamOrder,
        retry: StreamRetry,
        element_type: DebugType,
    },
    Singleton {
        bound: BoundKind,
        element_type: DebugType,
    },
    Optional {
        bound: BoundKind,
        element_type: DebugType,
    },
    KeyedStream {
        bound: BoundKind,
        value_order: StreamOrder,
        value_retry: StreamRetry,
        key_type: DebugType,
        value_type: DebugType,
    },
    KeyedSingleton {
        bound: KeyedSingletonBoundKind,
        key_type: DebugType,
        value_type: DebugType,
    },
}

#[derive(Clone)]
pub struct HydroIrMetadata {
    pub location_kind: LocationId,
    pub collection_kind: CollectionKind,
    pub cardinality: Option<usize>,
    pub tag: Option<String>,
    pub op: HydroIrOpMetadata,
}

// HydroIrMetadata shouldn't be used to hash or compare
impl Hash for HydroIrMetadata {
    fn hash<H: Hasher>(&self, _: &mut H) {}
}

impl PartialEq for HydroIrMetadata {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl Eq for HydroIrMetadata {}

impl Debug for HydroIrMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HydroIrMetadata")
            .field("location_kind", &self.location_kind)
            .field("collection_kind", &self.collection_kind)
            .finish()
    }
}

/// Metadata that is specific to the operator itself, rather than its outputs.
/// This is available on _both_ inner nodes and roots.
#[derive(Clone)]
pub struct HydroIrOpMetadata {
    pub backtrace: Backtrace,
    pub cpu_usage: Option<f64>,
    pub network_recv_cpu_usage: Option<f64>,
    pub id: Option<usize>,
}

impl HydroIrOpMetadata {
    #[expect(
        clippy::new_without_default,
        reason = "explicit calls to new ensure correct backtrace bounds"
    )]
    pub fn new() -> HydroIrOpMetadata {
        Self::new_with_skip(1)
    }

    fn new_with_skip(skip_count: usize) -> HydroIrOpMetadata {
        HydroIrOpMetadata {
            backtrace: Backtrace::get_backtrace(2 + skip_count),
            cpu_usage: None,
            network_recv_cpu_usage: None,
            id: None,
        }
    }
}

impl Debug for HydroIrOpMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HydroIrOpMetadata").finish()
    }
}

impl Hash for HydroIrOpMetadata {
    fn hash<H: Hasher>(&self, _: &mut H) {}
}

/// An intermediate node in a Hydro graph, which consumes data
/// from upstream nodes and emits data to downstream nodes.
#[derive(Debug, Hash)]
pub enum HydroNode {
    Placeholder,

    /// Manually "casts" between two different collection kinds.
    ///
    /// Using this IR node requires special care, since it bypasses many of Hydro's core
    /// correctness checks. In particular, the user must ensure that every possible
    /// "interpretation" of the input corresponds to a distinct "interpretation" of the output,
    /// where an "interpretation" is a possible output of `ObserveNonDet` applied to the
    /// collection. This ensures that the simulator does not miss any possible outputs.
    Cast {
        inner: Box<HydroNode>,
        metadata: HydroIrMetadata,
    },

    /// Strengthens the guarantees of a stream by non-deterministically selecting a possible
    /// interpretation of the input stream.
    ///
    /// In production, this simply passes through the input, but in simulation, this operator
    /// explicitly selects a randomized interpretation.
    ObserveNonDet {
        inner: Box<HydroNode>,
        trusted: bool, // if true, we do not need to simulate non-determinism
        metadata: HydroIrMetadata,
    },

    Source {
        source: HydroSource,
        metadata: HydroIrMetadata,
    },

    SingletonSource {
        value: DebugExpr,
        metadata: HydroIrMetadata,
    },

    CycleSource {
        ident: syn::Ident,
        metadata: HydroIrMetadata,
    },

    Tee {
        inner: TeeNode,
        metadata: HydroIrMetadata,
    },

    BeginAtomic {
        inner: Box<HydroNode>,
        metadata: HydroIrMetadata,
    },

    EndAtomic {
        inner: Box<HydroNode>,
        metadata: HydroIrMetadata,
    },

    Batch {
        inner: Box<HydroNode>,
        metadata: HydroIrMetadata,
    },

    YieldConcat {
        inner: Box<HydroNode>,
        metadata: HydroIrMetadata,
    },

    Chain {
        first: Box<HydroNode>,
        second: Box<HydroNode>,
        metadata: HydroIrMetadata,
    },

    ChainFirst {
        first: Box<HydroNode>,
        second: Box<HydroNode>,
        metadata: HydroIrMetadata,
    },

    CrossProduct {
        left: Box<HydroNode>,
        right: Box<HydroNode>,
        metadata: HydroIrMetadata,
    },

    CrossSingleton {
        left: Box<HydroNode>,
        right: Box<HydroNode>,
        metadata: HydroIrMetadata,
    },

    Join {
        left: Box<HydroNode>,
        right: Box<HydroNode>,
        metadata: HydroIrMetadata,
    },

    Difference {
        pos: Box<HydroNode>,
        neg: Box<HydroNode>,
        metadata: HydroIrMetadata,
    },

    AntiJoin {
        pos: Box<HydroNode>,
        neg: Box<HydroNode>,
        metadata: HydroIrMetadata,
    },

    ResolveFutures {
        input: Box<HydroNode>,
        metadata: HydroIrMetadata,
    },
    ResolveFuturesOrdered {
        input: Box<HydroNode>,
        metadata: HydroIrMetadata,
    },

    Map {
        f: DebugExpr,
        input: Box<HydroNode>,
        metadata: HydroIrMetadata,
    },
    FlatMap {
        f: DebugExpr,
        input: Box<HydroNode>,
        metadata: HydroIrMetadata,
    },
    Filter {
        f: DebugExpr,
        input: Box<HydroNode>,
        metadata: HydroIrMetadata,
    },
    FilterMap {
        f: DebugExpr,
        input: Box<HydroNode>,
        metadata: HydroIrMetadata,
    },

    DeferTick {
        input: Box<HydroNode>,
        metadata: HydroIrMetadata,
    },
    Enumerate {
        input: Box<HydroNode>,
        metadata: HydroIrMetadata,
    },
    Inspect {
        f: DebugExpr,
        input: Box<HydroNode>,
        metadata: HydroIrMetadata,
    },

    Unique {
        input: Box<HydroNode>,
        metadata: HydroIrMetadata,
    },

    Sort {
        input: Box<HydroNode>,
        metadata: HydroIrMetadata,
    },
    Fold {
        init: DebugExpr,
        acc: DebugExpr,
        input: Box<HydroNode>,
        metadata: HydroIrMetadata,
    },

    Scan {
        init: DebugExpr,
        acc: DebugExpr,
        input: Box<HydroNode>,
        metadata: HydroIrMetadata,
    },
    FoldKeyed {
        init: DebugExpr,
        acc: DebugExpr,
        input: Box<HydroNode>,
        metadata: HydroIrMetadata,
    },

    Reduce {
        f: DebugExpr,
        input: Box<HydroNode>,
        metadata: HydroIrMetadata,
    },
    ReduceKeyed {
        f: DebugExpr,
        input: Box<HydroNode>,
        metadata: HydroIrMetadata,
    },
    ReduceKeyedWatermark {
        f: DebugExpr,
        input: Box<HydroNode>,
        watermark: Box<HydroNode>,
        metadata: HydroIrMetadata,
    },

    Network {
        serialize_fn: Option<DebugExpr>,
        instantiate_fn: DebugInstantiate,
        deserialize_fn: Option<DebugExpr>,
        input: Box<HydroNode>,
        metadata: HydroIrMetadata,
    },

    ExternalInput {
        from_external_id: usize,
        from_key: usize,
        from_many: bool,
        codec_type: DebugType,
        port_hint: NetworkHint,
        instantiate_fn: DebugInstantiate,
        deserialize_fn: Option<DebugExpr>,
        metadata: HydroIrMetadata,
    },

    Counter {
        tag: String,
        duration: DebugExpr,
        prefix: String,
        input: Box<HydroNode>,
        metadata: HydroIrMetadata,
    },
}

pub type SeenTees = HashMap<*const RefCell<HydroNode>, Rc<RefCell<HydroNode>>>;
pub type SeenTeeLocations = HashMap<*const RefCell<HydroNode>, LocationId>;

impl HydroNode {
    pub fn transform_bottom_up(
        &mut self,
        transform: &mut impl FnMut(&mut HydroNode),
        seen_tees: &mut SeenTees,
        check_well_formed: bool,
    ) {
        self.transform_children(
            |n, s| n.transform_bottom_up(transform, s, check_well_formed),
            seen_tees,
        );

        transform(self);

        let self_location = self.metadata().location_kind.root();

        if check_well_formed {
            match &*self {
                HydroNode::Network { .. } => {}
                _ => {
                    self.input_metadata().iter().for_each(|i| {
                        if i.location_kind.root() != self_location {
                            panic!(
                                "Mismatching IR locations, child: {:?} ({:?}) of: {:?} ({:?})",
                                i,
                                i.location_kind.root(),
                                self,
                                self_location
                            )
                        }
                    });
                }
            }
        }
    }

    #[inline(always)]
    pub fn transform_children(
        &mut self,
        mut transform: impl FnMut(&mut HydroNode, &mut SeenTees),
        seen_tees: &mut SeenTees,
    ) {
        match self {
            HydroNode::Placeholder => {
                panic!();
            }

            HydroNode::Source { .. }
            | HydroNode::SingletonSource { .. }
            | HydroNode::CycleSource { .. }
            | HydroNode::ExternalInput { .. } => {}

            HydroNode::Tee { inner, .. } => {
                if let Some(transformed) = seen_tees.get(&inner.as_ptr()) {
                    *inner = TeeNode(transformed.clone());
                } else {
                    let transformed_cell = Rc::new(RefCell::new(HydroNode::Placeholder));
                    seen_tees.insert(inner.as_ptr(), transformed_cell.clone());
                    let mut orig = inner.0.replace(HydroNode::Placeholder);
                    transform(&mut orig, seen_tees);
                    *transformed_cell.borrow_mut() = orig;
                    *inner = TeeNode(transformed_cell);
                }
            }

            HydroNode::Cast { inner, .. }
            | HydroNode::ObserveNonDet { inner, .. }
            | HydroNode::BeginAtomic { inner, .. }
            | HydroNode::EndAtomic { inner, .. }
            | HydroNode::Batch { inner, .. }
            | HydroNode::YieldConcat { inner, .. } => {
                transform(inner.as_mut(), seen_tees);
            }

            HydroNode::Chain { first, second, .. } => {
                transform(first.as_mut(), seen_tees);
                transform(second.as_mut(), seen_tees);
            }

            HydroNode::ChainFirst { first, second, .. } => {
                transform(first.as_mut(), seen_tees);
                transform(second.as_mut(), seen_tees);
            }

            HydroNode::CrossSingleton { left, right, .. }
            | HydroNode::CrossProduct { left, right, .. }
            | HydroNode::Join { left, right, .. } => {
                transform(left.as_mut(), seen_tees);
                transform(right.as_mut(), seen_tees);
            }

            HydroNode::Difference { pos, neg, .. } | HydroNode::AntiJoin { pos, neg, .. } => {
                transform(pos.as_mut(), seen_tees);
                transform(neg.as_mut(), seen_tees);
            }

            HydroNode::ReduceKeyedWatermark {
                input, watermark, ..
            } => {
                transform(input.as_mut(), seen_tees);
                transform(watermark.as_mut(), seen_tees);
            }

            HydroNode::Map { input, .. }
            | HydroNode::ResolveFutures { input, .. }
            | HydroNode::ResolveFuturesOrdered { input, .. }
            | HydroNode::FlatMap { input, .. }
            | HydroNode::Filter { input, .. }
            | HydroNode::FilterMap { input, .. }
            | HydroNode::Sort { input, .. }
            | HydroNode::DeferTick { input, .. }
            | HydroNode::Enumerate { input, .. }
            | HydroNode::Inspect { input, .. }
            | HydroNode::Unique { input, .. }
            | HydroNode::Network { input, .. }
            | HydroNode::Fold { input, .. }
            | HydroNode::Scan { input, .. }
            | HydroNode::FoldKeyed { input, .. }
            | HydroNode::Reduce { input, .. }
            | HydroNode::ReduceKeyed { input, .. }
            | HydroNode::Counter { input, .. } => {
                transform(input.as_mut(), seen_tees);
            }
        }
    }

    pub fn deep_clone(&self, seen_tees: &mut SeenTees) -> HydroNode {
        match self {
            HydroNode::Placeholder => HydroNode::Placeholder,
            HydroNode::Cast { inner, metadata } => HydroNode::Cast {
                inner: Box::new(inner.deep_clone(seen_tees)),
                metadata: metadata.clone(),
            },
            HydroNode::ObserveNonDet {
                inner,
                trusted,
                metadata,
            } => HydroNode::ObserveNonDet {
                inner: Box::new(inner.deep_clone(seen_tees)),
                trusted: *trusted,
                metadata: metadata.clone(),
            },
            HydroNode::Source { source, metadata } => HydroNode::Source {
                source: source.clone(),
                metadata: metadata.clone(),
            },
            HydroNode::SingletonSource { value, metadata } => HydroNode::SingletonSource {
                value: value.clone(),
                metadata: metadata.clone(),
            },
            HydroNode::CycleSource { ident, metadata } => HydroNode::CycleSource {
                ident: ident.clone(),
                metadata: metadata.clone(),
            },
            HydroNode::Tee { inner, metadata } => {
                if let Some(transformed) = seen_tees.get(&inner.as_ptr()) {
                    HydroNode::Tee {
                        inner: TeeNode(transformed.clone()),
                        metadata: metadata.clone(),
                    }
                } else {
                    let new_rc = Rc::new(RefCell::new(HydroNode::Placeholder));
                    seen_tees.insert(inner.as_ptr(), new_rc.clone());
                    let cloned = inner.0.borrow().deep_clone(seen_tees);
                    *new_rc.borrow_mut() = cloned;
                    HydroNode::Tee {
                        inner: TeeNode(new_rc),
                        metadata: metadata.clone(),
                    }
                }
            }
            HydroNode::YieldConcat { inner, metadata } => HydroNode::YieldConcat {
                inner: Box::new(inner.deep_clone(seen_tees)),
                metadata: metadata.clone(),
            },
            HydroNode::BeginAtomic { inner, metadata } => HydroNode::BeginAtomic {
                inner: Box::new(inner.deep_clone(seen_tees)),
                metadata: metadata.clone(),
            },
            HydroNode::EndAtomic { inner, metadata } => HydroNode::EndAtomic {
                inner: Box::new(inner.deep_clone(seen_tees)),
                metadata: metadata.clone(),
            },
            HydroNode::Batch { inner, metadata } => HydroNode::Batch {
                inner: Box::new(inner.deep_clone(seen_tees)),
                metadata: metadata.clone(),
            },
            HydroNode::Chain {
                first,
                second,
                metadata,
            } => HydroNode::Chain {
                first: Box::new(first.deep_clone(seen_tees)),
                second: Box::new(second.deep_clone(seen_tees)),
                metadata: metadata.clone(),
            },
            HydroNode::ChainFirst {
                first,
                second,
                metadata,
            } => HydroNode::ChainFirst {
                first: Box::new(first.deep_clone(seen_tees)),
                second: Box::new(second.deep_clone(seen_tees)),
                metadata: metadata.clone(),
            },
            HydroNode::CrossProduct {
                left,
                right,
                metadata,
            } => HydroNode::CrossProduct {
                left: Box::new(left.deep_clone(seen_tees)),
                right: Box::new(right.deep_clone(seen_tees)),
                metadata: metadata.clone(),
            },
            HydroNode::CrossSingleton {
                left,
                right,
                metadata,
            } => HydroNode::CrossSingleton {
                left: Box::new(left.deep_clone(seen_tees)),
                right: Box::new(right.deep_clone(seen_tees)),
                metadata: metadata.clone(),
            },
            HydroNode::Join {
                left,
                right,
                metadata,
            } => HydroNode::Join {
                left: Box::new(left.deep_clone(seen_tees)),
                right: Box::new(right.deep_clone(seen_tees)),
                metadata: metadata.clone(),
            },
            HydroNode::Difference { pos, neg, metadata } => HydroNode::Difference {
                pos: Box::new(pos.deep_clone(seen_tees)),
                neg: Box::new(neg.deep_clone(seen_tees)),
                metadata: metadata.clone(),
            },
            HydroNode::AntiJoin { pos, neg, metadata } => HydroNode::AntiJoin {
                pos: Box::new(pos.deep_clone(seen_tees)),
                neg: Box::new(neg.deep_clone(seen_tees)),
                metadata: metadata.clone(),
            },
            HydroNode::ResolveFutures { input, metadata } => HydroNode::ResolveFutures {
                input: Box::new(input.deep_clone(seen_tees)),
                metadata: metadata.clone(),
            },
            HydroNode::ResolveFuturesOrdered { input, metadata } => {
                HydroNode::ResolveFuturesOrdered {
                    input: Box::new(input.deep_clone(seen_tees)),
                    metadata: metadata.clone(),
                }
            }
            HydroNode::Map { f, input, metadata } => HydroNode::Map {
                f: f.clone(),
                input: Box::new(input.deep_clone(seen_tees)),
                metadata: metadata.clone(),
            },
            HydroNode::FlatMap { f, input, metadata } => HydroNode::FlatMap {
                f: f.clone(),
                input: Box::new(input.deep_clone(seen_tees)),
                metadata: metadata.clone(),
            },
            HydroNode::Filter { f, input, metadata } => HydroNode::Filter {
                f: f.clone(),
                input: Box::new(input.deep_clone(seen_tees)),
                metadata: metadata.clone(),
            },
            HydroNode::FilterMap { f, input, metadata } => HydroNode::FilterMap {
                f: f.clone(),
                input: Box::new(input.deep_clone(seen_tees)),
                metadata: metadata.clone(),
            },
            HydroNode::DeferTick { input, metadata } => HydroNode::DeferTick {
                input: Box::new(input.deep_clone(seen_tees)),
                metadata: metadata.clone(),
            },
            HydroNode::Enumerate { input, metadata } => HydroNode::Enumerate {
                input: Box::new(input.deep_clone(seen_tees)),
                metadata: metadata.clone(),
            },
            HydroNode::Inspect { f, input, metadata } => HydroNode::Inspect {
                f: f.clone(),
                input: Box::new(input.deep_clone(seen_tees)),
                metadata: metadata.clone(),
            },
            HydroNode::Unique { input, metadata } => HydroNode::Unique {
                input: Box::new(input.deep_clone(seen_tees)),
                metadata: metadata.clone(),
            },
            HydroNode::Sort { input, metadata } => HydroNode::Sort {
                input: Box::new(input.deep_clone(seen_tees)),
                metadata: metadata.clone(),
            },
            HydroNode::Fold {
                init,
                acc,
                input,
                metadata,
            } => HydroNode::Fold {
                init: init.clone(),
                acc: acc.clone(),
                input: Box::new(input.deep_clone(seen_tees)),
                metadata: metadata.clone(),
            },
            HydroNode::Scan {
                init,
                acc,
                input,
                metadata,
            } => HydroNode::Scan {
                init: init.clone(),
                acc: acc.clone(),
                input: Box::new(input.deep_clone(seen_tees)),
                metadata: metadata.clone(),
            },
            HydroNode::FoldKeyed {
                init,
                acc,
                input,
                metadata,
            } => HydroNode::FoldKeyed {
                init: init.clone(),
                acc: acc.clone(),
                input: Box::new(input.deep_clone(seen_tees)),
                metadata: metadata.clone(),
            },
            HydroNode::ReduceKeyedWatermark {
                f,
                input,
                watermark,
                metadata,
            } => HydroNode::ReduceKeyedWatermark {
                f: f.clone(),
                input: Box::new(input.deep_clone(seen_tees)),
                watermark: Box::new(watermark.deep_clone(seen_tees)),
                metadata: metadata.clone(),
            },
            HydroNode::Reduce { f, input, metadata } => HydroNode::Reduce {
                f: f.clone(),
                input: Box::new(input.deep_clone(seen_tees)),
                metadata: metadata.clone(),
            },
            HydroNode::ReduceKeyed { f, input, metadata } => HydroNode::ReduceKeyed {
                f: f.clone(),
                input: Box::new(input.deep_clone(seen_tees)),
                metadata: metadata.clone(),
            },
            HydroNode::Network {
                serialize_fn,
                instantiate_fn,
                deserialize_fn,
                input,
                metadata,
            } => HydroNode::Network {
                serialize_fn: serialize_fn.clone(),
                instantiate_fn: instantiate_fn.clone(),
                deserialize_fn: deserialize_fn.clone(),
                input: Box::new(input.deep_clone(seen_tees)),
                metadata: metadata.clone(),
            },
            HydroNode::ExternalInput {
                from_external_id,
                from_key,
                from_many,
                codec_type,
                port_hint,
                instantiate_fn,
                deserialize_fn,
                metadata,
            } => HydroNode::ExternalInput {
                from_external_id: *from_external_id,
                from_key: *from_key,
                from_many: *from_many,
                codec_type: codec_type.clone(),
                port_hint: *port_hint,
                instantiate_fn: instantiate_fn.clone(),
                deserialize_fn: deserialize_fn.clone(),
                metadata: metadata.clone(),
            },
            HydroNode::Counter {
                tag,
                duration,
                prefix,
                input,
                metadata,
            } => HydroNode::Counter {
                tag: tag.clone(),
                duration: duration.clone(),
                prefix: prefix.clone(),
                input: Box::new(input.deep_clone(seen_tees)),
                metadata: metadata.clone(),
            },
        }
    }

    #[cfg(feature = "build")]
    pub fn emit_core<'a, D: Deploy<'a>>(
        &mut self,
        builders_or_callback: &mut BuildersOrCallback<
            impl FnMut(&mut HydroRoot, &mut usize),
            impl FnMut(&mut HydroNode, &mut usize),
        >,
        built_tees: &mut HashMap<*const RefCell<HydroNode>, syn::Ident>,
        next_stmt_id: &mut usize,
    ) -> syn::Ident {
        let out_location = self.metadata().location_kind.clone();
        match self {
            HydroNode::Placeholder => {
                panic!()
            }

            HydroNode::Cast { inner, .. } => {
                let inner_ident =
                    inner.emit_core::<D>(builders_or_callback, built_tees, next_stmt_id);

                match builders_or_callback {
                    BuildersOrCallback::Builders(_) => {}
                    BuildersOrCallback::Callback(_, node_callback) => {
                        node_callback(self, next_stmt_id);
                    }
                }

                *next_stmt_id += 1;

                inner_ident
            }

            HydroNode::ObserveNonDet {
                inner,
                trusted,
                metadata,
                ..
            } => {
                let inner_ident =
                    inner.emit_core::<D>(builders_or_callback, built_tees, next_stmt_id);

                let observe_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        graph_builders.observe_nondet(
                            *trusted,
                            &inner.metadata().location_kind,
                            inner_ident,
                            &inner.metadata().collection_kind,
                            &observe_ident,
                            &metadata.collection_kind,
                            &metadata.op,
                        );
                    }
                    BuildersOrCallback::Callback(_, node_callback) => {
                        node_callback(self, next_stmt_id);
                    }
                }

                *next_stmt_id += 1;

                observe_ident
            }

            HydroNode::Batch {
                inner, metadata, ..
            } => {
                let inner_ident =
                    inner.emit_core::<D>(builders_or_callback, built_tees, next_stmt_id);

                let batch_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        graph_builders.batch(
                            inner_ident,
                            &inner.metadata().location_kind,
                            &inner.metadata().collection_kind,
                            &batch_ident,
                            &out_location,
                            &metadata.op,
                        );
                    }
                    BuildersOrCallback::Callback(_, node_callback) => {
                        node_callback(self, next_stmt_id);
                    }
                }

                *next_stmt_id += 1;

                batch_ident
            }

            HydroNode::YieldConcat { inner, .. } => {
                let inner_ident =
                    inner.emit_core::<D>(builders_or_callback, built_tees, next_stmt_id);

                let yield_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        graph_builders.yield_from_tick(
                            inner_ident,
                            &inner.metadata().location_kind,
                            &inner.metadata().collection_kind,
                            &yield_ident,
                            &out_location,
                        );
                    }
                    BuildersOrCallback::Callback(_, node_callback) => {
                        node_callback(self, next_stmt_id);
                    }
                }

                *next_stmt_id += 1;

                yield_ident
            }

            HydroNode::BeginAtomic { inner, metadata } => {
                let inner_ident =
                    inner.emit_core::<D>(builders_or_callback, built_tees, next_stmt_id);

                let begin_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        graph_builders.begin_atomic(
                            inner_ident,
                            &inner.metadata().location_kind,
                            &inner.metadata().collection_kind,
                            &begin_ident,
                            &out_location,
                            &metadata.op,
                        );
                    }
                    BuildersOrCallback::Callback(_, node_callback) => {
                        node_callback(self, next_stmt_id);
                    }
                }

                *next_stmt_id += 1;

                begin_ident
            }

            HydroNode::EndAtomic { inner, .. } => {
                let inner_ident =
                    inner.emit_core::<D>(builders_or_callback, built_tees, next_stmt_id);

                let end_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        graph_builders.end_atomic(
                            inner_ident,
                            &inner.metadata().location_kind,
                            &inner.metadata().collection_kind,
                            &end_ident,
                        );
                    }
                    BuildersOrCallback::Callback(_, node_callback) => {
                        node_callback(self, next_stmt_id);
                    }
                }

                *next_stmt_id += 1;

                end_ident
            }

            HydroNode::Source {
                source, metadata, ..
            } => {
                if let HydroSource::ExternalNetwork() = source {
                    syn::Ident::new("DUMMY", Span::call_site())
                } else {
                    let source_ident =
                        syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                    let source_stmt = match source {
                        HydroSource::Stream(expr) => {
                            debug_assert!(metadata.location_kind.is_top_level());
                            parse_quote! {
                                #source_ident = source_stream(#expr);
                            }
                        }

                        HydroSource::ExternalNetwork() => {
                            unreachable!()
                        }

                        HydroSource::Iter(expr) => {
                            if metadata.location_kind.is_top_level() {
                                parse_quote! {
                                    #source_ident = source_iter(#expr);
                                }
                            } else {
                                // TODO(shadaj): a more natural semantics would be to to re-evaluate the expression on each tick
                                parse_quote! {
                                    #source_ident = source_iter(#expr) -> persist::<'static>();
                                }
                            }
                        }

                        HydroSource::Spin() => {
                            debug_assert!(metadata.location_kind.is_top_level());
                            parse_quote! {
                                #source_ident = spin();
                            }
                        }

                        HydroSource::ClusterMembers(location_id) => {
                            debug_assert!(metadata.location_kind.is_top_level());

                            let expr = stageleft::QuotedWithContext::splice_untyped_ctx(
                                D::cluster_membership_stream(location_id),
                                &(),
                            );

                            parse_quote! {
                                #source_ident = source_stream(#expr);
                            }
                        }
                    };

                    match builders_or_callback {
                        BuildersOrCallback::Builders(graph_builders) => {
                            let builder = graph_builders.get_dfir_mut(&out_location);
                            builder.add_dfir(source_stmt, None, Some(&next_stmt_id.to_string()));
                        }
                        BuildersOrCallback::Callback(_, node_callback) => {
                            node_callback(self, next_stmt_id);
                        }
                    }

                    *next_stmt_id += 1;

                    source_ident
                }
            }

            HydroNode::SingletonSource { value, metadata } => {
                let source_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        let should_replay = !graph_builders.singleton_intermediates();
                        let builder = graph_builders.get_dfir_mut(&out_location);

                        if should_replay || !metadata.location_kind.is_top_level() {
                            builder.add_dfir(
                                parse_quote! {
                                    #source_ident = source_iter([#value]) -> persist::<'static>();
                                },
                                None,
                                Some(&next_stmt_id.to_string()),
                            );
                        } else {
                            builder.add_dfir(
                                parse_quote! {
                                    #source_ident = source_iter([#value]);
                                },
                                None,
                                Some(&next_stmt_id.to_string()),
                            );
                        }
                    }
                    BuildersOrCallback::Callback(_, node_callback) => {
                        node_callback(self, next_stmt_id);
                    }
                }

                *next_stmt_id += 1;

                source_ident
            }

            HydroNode::CycleSource { ident, .. } => {
                let ident = ident.clone();

                match builders_or_callback {
                    BuildersOrCallback::Builders(_) => {}
                    BuildersOrCallback::Callback(_, node_callback) => {
                        node_callback(self, next_stmt_id);
                    }
                }

                // consume a stmt id even though we did not emit anything so that we can instrument this
                *next_stmt_id += 1;

                ident
            }

            HydroNode::Tee { inner, .. } => {
                let ret_ident = if let Some(teed_from) =
                    built_tees.get(&(inner.0.as_ref() as *const RefCell<HydroNode>))
                {
                    match builders_or_callback {
                        BuildersOrCallback::Builders(_) => {}
                        BuildersOrCallback::Callback(_, node_callback) => {
                            node_callback(self, next_stmt_id);
                        }
                    }

                    teed_from.clone()
                } else {
                    let inner_ident = inner.0.borrow_mut().emit_core::<D>(
                        builders_or_callback,
                        built_tees,
                        next_stmt_id,
                    );

                    let tee_ident =
                        syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                    built_tees.insert(
                        inner.0.as_ref() as *const RefCell<HydroNode>,
                        tee_ident.clone(),
                    );

                    match builders_or_callback {
                        BuildersOrCallback::Builders(graph_builders) => {
                            let builder = graph_builders.get_dfir_mut(&out_location);
                            builder.add_dfir(
                                parse_quote! {
                                    #tee_ident = #inner_ident -> tee();
                                },
                                None,
                                Some(&next_stmt_id.to_string()),
                            );
                        }
                        BuildersOrCallback::Callback(_, node_callback) => {
                            node_callback(self, next_stmt_id);
                        }
                    }

                    tee_ident
                };

                // we consume a stmt id regardless of if we emit the tee() operator,
                // so that during rewrites we touch all recipients of the tee()

                *next_stmt_id += 1;
                ret_ident
            }

            HydroNode::Chain { first, second, .. } => {
                let first_ident =
                    first.emit_core::<D>(builders_or_callback, built_tees, next_stmt_id);
                let second_ident =
                    second.emit_core::<D>(builders_or_callback, built_tees, next_stmt_id);

                let chain_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        let builder = graph_builders.get_dfir_mut(&out_location);
                        builder.add_dfir(
                            parse_quote! {
                                #chain_ident = chain();
                                #first_ident -> [0]#chain_ident;
                                #second_ident -> [1]#chain_ident;
                            },
                            None,
                            Some(&next_stmt_id.to_string()),
                        );
                    }
                    BuildersOrCallback::Callback(_, node_callback) => {
                        node_callback(self, next_stmt_id);
                    }
                }

                *next_stmt_id += 1;

                chain_ident
            }

            HydroNode::ChainFirst { first, second, .. } => {
                let first_ident =
                    first.emit_core::<D>(builders_or_callback, built_tees, next_stmt_id);
                let second_ident =
                    second.emit_core::<D>(builders_or_callback, built_tees, next_stmt_id);

                let chain_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        let builder = graph_builders.get_dfir_mut(&out_location);
                        builder.add_dfir(
                            parse_quote! {
                                #chain_ident = chain_first_n(1);
                                #first_ident -> [0]#chain_ident;
                                #second_ident -> [1]#chain_ident;
                            },
                            None,
                            Some(&next_stmt_id.to_string()),
                        );
                    }
                    BuildersOrCallback::Callback(_, node_callback) => {
                        node_callback(self, next_stmt_id);
                    }
                }

                *next_stmt_id += 1;

                chain_ident
            }

            HydroNode::CrossSingleton { left, right, .. } => {
                let left_ident =
                    left.emit_core::<D>(builders_or_callback, built_tees, next_stmt_id);
                let right_ident =
                    right.emit_core::<D>(builders_or_callback, built_tees, next_stmt_id);

                let cross_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        let builder = graph_builders.get_dfir_mut(&out_location);
                        builder.add_dfir(
                            parse_quote! {
                                #cross_ident = cross_singleton();
                                #left_ident -> [input]#cross_ident;
                                #right_ident -> [single]#cross_ident;
                            },
                            None,
                            Some(&next_stmt_id.to_string()),
                        );
                    }
                    BuildersOrCallback::Callback(_, node_callback) => {
                        node_callback(self, next_stmt_id);
                    }
                }

                *next_stmt_id += 1;

                cross_ident
            }

            HydroNode::CrossProduct { .. } | HydroNode::Join { .. } => {
                let operator: syn::Ident = if matches!(self, HydroNode::CrossProduct { .. }) {
                    parse_quote!(cross_join_multiset)
                } else {
                    parse_quote!(join_multiset)
                };

                let (HydroNode::CrossProduct { left, right, .. }
                | HydroNode::Join { left, right, .. }) = self
                else {
                    unreachable!()
                };

                let is_top_level = left.metadata().location_kind.is_top_level()
                    && right.metadata().location_kind.is_top_level();
                let (left_inner, left_lifetime) = if left.metadata().location_kind.is_top_level() {
                    (left, quote!('static))
                } else {
                    (left, quote!('tick))
                };

                let (right_inner, right_lifetime) = if right.metadata().location_kind.is_top_level()
                {
                    (right, quote!('static))
                } else {
                    (right, quote!('tick))
                };

                let left_ident =
                    left_inner.emit_core::<D>(builders_or_callback, built_tees, next_stmt_id);
                let right_ident =
                    right_inner.emit_core::<D>(builders_or_callback, built_tees, next_stmt_id);

                let stream_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        let builder = graph_builders.get_dfir_mut(&out_location);
                        builder.add_dfir(
                            if is_top_level {
                                // if both inputs are root, the output is expected to have streamy semantics, so we need
                                // a multiset_delta() to negate the replay behavior
                                parse_quote! {
                                    #stream_ident = #operator::<#left_lifetime, #right_lifetime>() -> multiset_delta();
                                    #left_ident -> [0]#stream_ident;
                                    #right_ident -> [1]#stream_ident;
                                }
                            } else {
                                parse_quote! {
                                    #stream_ident = #operator::<#left_lifetime, #right_lifetime>();
                                    #left_ident -> [0]#stream_ident;
                                    #right_ident -> [1]#stream_ident;
                                }
                            }
                            ,
                            None,
                            Some(&next_stmt_id.to_string()),
                        );
                    }
                    BuildersOrCallback::Callback(_, node_callback) => {
                        node_callback(self, next_stmt_id);
                    }
                }

                *next_stmt_id += 1;

                stream_ident
            }

            HydroNode::Difference { .. } | HydroNode::AntiJoin { .. } => {
                let operator: syn::Ident = if matches!(self, HydroNode::Difference { .. }) {
                    parse_quote!(difference)
                } else {
                    parse_quote!(anti_join)
                };

                let (HydroNode::Difference { pos, neg, .. } | HydroNode::AntiJoin { pos, neg, .. }) =
                    self
                else {
                    unreachable!()
                };

                let (neg, neg_lifetime) = if neg.metadata().location_kind.is_top_level() {
                    (neg, quote!('static))
                } else {
                    (neg, quote!('tick))
                };

                let pos_ident = pos.emit_core::<D>(builders_or_callback, built_tees, next_stmt_id);
                let neg_ident = neg.emit_core::<D>(builders_or_callback, built_tees, next_stmt_id);

                let stream_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        let builder = graph_builders.get_dfir_mut(&out_location);
                        builder.add_dfir(
                            parse_quote! {
                                #stream_ident = #operator::<'tick, #neg_lifetime>();
                                #pos_ident -> [pos]#stream_ident;
                                #neg_ident -> [neg]#stream_ident;
                            },
                            None,
                            Some(&next_stmt_id.to_string()),
                        );
                    }
                    BuildersOrCallback::Callback(_, node_callback) => {
                        node_callback(self, next_stmt_id);
                    }
                }

                *next_stmt_id += 1;

                stream_ident
            }

            HydroNode::ResolveFutures { input, .. } => {
                let input_ident =
                    input.emit_core::<D>(builders_or_callback, built_tees, next_stmt_id);

                let futures_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        let builder = graph_builders.get_dfir_mut(&out_location);
                        builder.add_dfir(
                            parse_quote! {
                                #futures_ident = #input_ident -> resolve_futures();
                            },
                            None,
                            Some(&next_stmt_id.to_string()),
                        );
                    }
                    BuildersOrCallback::Callback(_, node_callback) => {
                        node_callback(self, next_stmt_id);
                    }
                }

                *next_stmt_id += 1;

                futures_ident
            }

            HydroNode::ResolveFuturesOrdered { input, .. } => {
                let input_ident =
                    input.emit_core::<D>(builders_or_callback, built_tees, next_stmt_id);

                let futures_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        let builder = graph_builders.get_dfir_mut(&out_location);
                        builder.add_dfir(
                            parse_quote! {
                                #futures_ident = #input_ident -> resolve_futures_ordered();
                            },
                            None,
                            Some(&next_stmt_id.to_string()),
                        );
                    }
                    BuildersOrCallback::Callback(_, node_callback) => {
                        node_callback(self, next_stmt_id);
                    }
                }

                *next_stmt_id += 1;

                futures_ident
            }

            HydroNode::Map { f, input, .. } => {
                let input_ident =
                    input.emit_core::<D>(builders_or_callback, built_tees, next_stmt_id);

                let map_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        let builder = graph_builders.get_dfir_mut(&out_location);
                        builder.add_dfir(
                            parse_quote! {
                                #map_ident = #input_ident -> map(#f);
                            },
                            None,
                            Some(&next_stmt_id.to_string()),
                        );
                    }
                    BuildersOrCallback::Callback(_, node_callback) => {
                        node_callback(self, next_stmt_id);
                    }
                }

                *next_stmt_id += 1;

                map_ident
            }

            HydroNode::FlatMap { f, input, .. } => {
                let input_ident =
                    input.emit_core::<D>(builders_or_callback, built_tees, next_stmt_id);

                let flat_map_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        let builder = graph_builders.get_dfir_mut(&out_location);
                        builder.add_dfir(
                            parse_quote! {
                                #flat_map_ident = #input_ident -> flat_map(#f);
                            },
                            None,
                            Some(&next_stmt_id.to_string()),
                        );
                    }
                    BuildersOrCallback::Callback(_, node_callback) => {
                        node_callback(self, next_stmt_id);
                    }
                }

                *next_stmt_id += 1;

                flat_map_ident
            }

            HydroNode::Filter { f, input, .. } => {
                let input_ident =
                    input.emit_core::<D>(builders_or_callback, built_tees, next_stmt_id);

                let filter_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        let builder = graph_builders.get_dfir_mut(&out_location);
                        builder.add_dfir(
                            parse_quote! {
                                #filter_ident = #input_ident -> filter(#f);
                            },
                            None,
                            Some(&next_stmt_id.to_string()),
                        );
                    }
                    BuildersOrCallback::Callback(_, node_callback) => {
                        node_callback(self, next_stmt_id);
                    }
                }

                *next_stmt_id += 1;

                filter_ident
            }

            HydroNode::FilterMap { f, input, .. } => {
                let input_ident =
                    input.emit_core::<D>(builders_or_callback, built_tees, next_stmt_id);

                let filter_map_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        let builder = graph_builders.get_dfir_mut(&out_location);
                        builder.add_dfir(
                            parse_quote! {
                                #filter_map_ident = #input_ident -> filter_map(#f);
                            },
                            None,
                            Some(&next_stmt_id.to_string()),
                        );
                    }
                    BuildersOrCallback::Callback(_, node_callback) => {
                        node_callback(self, next_stmt_id);
                    }
                }

                *next_stmt_id += 1;

                filter_map_ident
            }

            HydroNode::Sort { input, .. } => {
                let input_ident =
                    input.emit_core::<D>(builders_or_callback, built_tees, next_stmt_id);

                let sort_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        let builder = graph_builders.get_dfir_mut(&out_location);
                        builder.add_dfir(
                            parse_quote! {
                                #sort_ident = #input_ident -> sort();
                            },
                            None,
                            Some(&next_stmt_id.to_string()),
                        );
                    }
                    BuildersOrCallback::Callback(_, node_callback) => {
                        node_callback(self, next_stmt_id);
                    }
                }

                *next_stmt_id += 1;

                sort_ident
            }

            HydroNode::DeferTick { input, .. } => {
                let input_ident =
                    input.emit_core::<D>(builders_or_callback, built_tees, next_stmt_id);

                let defer_tick_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        let builder = graph_builders.get_dfir_mut(&out_location);
                        builder.add_dfir(
                            parse_quote! {
                                #defer_tick_ident = #input_ident -> defer_tick_lazy();
                            },
                            None,
                            Some(&next_stmt_id.to_string()),
                        );
                    }
                    BuildersOrCallback::Callback(_, node_callback) => {
                        node_callback(self, next_stmt_id);
                    }
                }

                *next_stmt_id += 1;

                defer_tick_ident
            }

            HydroNode::Enumerate { input, .. } => {
                let input_ident =
                    input.emit_core::<D>(builders_or_callback, built_tees, next_stmt_id);

                let enumerate_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        let builder = graph_builders.get_dfir_mut(&out_location);
                        let lifetime = if input.metadata().location_kind.is_top_level() {
                            quote!('static)
                        } else {
                            quote!('tick)
                        };
                        builder.add_dfir(
                            parse_quote! {
                                #enumerate_ident = #input_ident -> enumerate::<#lifetime>();
                            },
                            None,
                            Some(&next_stmt_id.to_string()),
                        );
                    }
                    BuildersOrCallback::Callback(_, node_callback) => {
                        node_callback(self, next_stmt_id);
                    }
                }

                *next_stmt_id += 1;

                enumerate_ident
            }

            HydroNode::Inspect { f, input, .. } => {
                let input_ident =
                    input.emit_core::<D>(builders_or_callback, built_tees, next_stmt_id);

                let inspect_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        let builder = graph_builders.get_dfir_mut(&out_location);
                        builder.add_dfir(
                            parse_quote! {
                                #inspect_ident = #input_ident -> inspect(#f);
                            },
                            None,
                            Some(&next_stmt_id.to_string()),
                        );
                    }
                    BuildersOrCallback::Callback(_, node_callback) => {
                        node_callback(self, next_stmt_id);
                    }
                }

                *next_stmt_id += 1;

                inspect_ident
            }

            HydroNode::Unique { input, .. } => {
                let input_ident =
                    input.emit_core::<D>(builders_or_callback, built_tees, next_stmt_id);

                let unique_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        let builder = graph_builders.get_dfir_mut(&out_location);
                        let lifetime = if input.metadata().location_kind.is_top_level() {
                            quote!('static)
                        } else {
                            quote!('tick)
                        };

                        builder.add_dfir(
                            parse_quote! {
                                #unique_ident = #input_ident -> unique::<#lifetime>();
                            },
                            None,
                            Some(&next_stmt_id.to_string()),
                        );
                    }
                    BuildersOrCallback::Callback(_, node_callback) => {
                        node_callback(self, next_stmt_id);
                    }
                }

                *next_stmt_id += 1;

                unique_ident
            }

            HydroNode::Fold { .. } | HydroNode::FoldKeyed { .. } | HydroNode::Scan { .. } => {
                let operator: syn::Ident = if matches!(self, HydroNode::Fold { .. }) {
                    parse_quote!(fold)
                } else if matches!(self, HydroNode::Scan { .. }) {
                    parse_quote!(scan)
                } else {
                    parse_quote!(fold_keyed)
                };

                let (HydroNode::Fold { input, .. }
                | HydroNode::FoldKeyed { input, .. }
                | HydroNode::Scan { input, .. }) = self
                else {
                    unreachable!()
                };

                let (input, lifetime) = if input.metadata().location_kind.is_top_level() {
                    (input, quote!('static))
                } else {
                    (input, quote!('tick))
                };

                let input_ident =
                    input.emit_core::<D>(builders_or_callback, built_tees, next_stmt_id);

                let (HydroNode::Fold { init, acc, .. }
                | HydroNode::FoldKeyed { init, acc, .. }
                | HydroNode::Scan { init, acc, .. }) = &*self
                else {
                    unreachable!()
                };

                let fold_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        if matches!(self, HydroNode::Fold { .. })
                            && self.metadata().location_kind.is_top_level()
                            && !(matches!(self.metadata().location_kind, LocationId::Atomic(_)))
                            && graph_builders.singleton_intermediates()
                        {
                            let builder = graph_builders.get_dfir_mut(&out_location);

                            let acc: syn::Expr = parse_quote!({
                                let mut __inner = #acc;
                                move |__state, __value| {
                                    __inner(__state, __value);
                                    Some(__state.clone())
                                }
                            });

                            builder.add_dfir(
                                parse_quote! {
                                    source_iter([(#init)()]) -> [0]#fold_ident;
                                    #input_ident -> scan::<#lifetime>(#init, #acc) -> [1]#fold_ident;
                                    #fold_ident = chain();
                                },
                                None,
                                Some(&next_stmt_id.to_string()),
                            );
                        } else if matches!(self, HydroNode::FoldKeyed { .. })
                            && self.metadata().location_kind.is_top_level()
                            && !(matches!(self.metadata().location_kind, LocationId::Atomic(_)))
                            && graph_builders.singleton_intermediates()
                        {
                            let builder = graph_builders.get_dfir_mut(&out_location);

                            let acc: syn::Expr = parse_quote!({
                                let mut __init = #init;
                                let mut __inner = #acc;
                                move |__state, __kv: (_, _)| {
                                    // TODO(shadaj): we can avoid the clone when the entry exists
                                    let __state = __state
                                        .entry(::std::clone::Clone::clone(&__kv.0))
                                        .or_insert_with(|| (__init)());
                                    __inner(__state, __kv.1);
                                    Some((__kv.0, ::std::clone::Clone::clone(&*__state)))
                                }
                            });

                            builder.add_dfir(
                                parse_quote! {
                                    source_iter([(#init)()]) -> [0]#fold_ident;
                                    #fold_ident = #input_ident -> scan::<#lifetime>(|| ::std::collections::HashMap::new(), #acc);
                                },
                                None,
                                Some(&next_stmt_id.to_string()),
                            );
                        } else {
                            let builder = graph_builders.get_dfir_mut(&out_location);
                            builder.add_dfir(
                                parse_quote! {
                                    #fold_ident = #input_ident -> #operator::<#lifetime>(#init, #acc);
                                },
                                None,
                                Some(&next_stmt_id.to_string()),
                            );
                        }
                    }
                    BuildersOrCallback::Callback(_, node_callback) => {
                        node_callback(self, next_stmt_id);
                    }
                }

                *next_stmt_id += 1;

                fold_ident
            }

            HydroNode::Reduce { .. } | HydroNode::ReduceKeyed { .. } => {
                let operator: syn::Ident = if matches!(self, HydroNode::Reduce { .. }) {
                    parse_quote!(reduce)
                } else {
                    parse_quote!(reduce_keyed)
                };

                let (HydroNode::Reduce { input, .. } | HydroNode::ReduceKeyed { input, .. }) = self
                else {
                    unreachable!()
                };

                let (input, lifetime) = if input.metadata().location_kind.is_top_level() {
                    (input, quote!('static))
                } else {
                    (input, quote!('tick))
                };

                let input_ident =
                    input.emit_core::<D>(builders_or_callback, built_tees, next_stmt_id);

                let (HydroNode::Reduce { f, .. } | HydroNode::ReduceKeyed { f, .. }) = &*self
                else {
                    unreachable!()
                };

                let reduce_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        if matches!(self, HydroNode::Reduce { .. })
                            && self.metadata().location_kind.is_top_level()
                            && !(matches!(self.metadata().location_kind, LocationId::Atomic(_)))
                            && graph_builders.singleton_intermediates()
                        {
                            todo!(
                                "Reduce with optional intermediates is not yet supported in simulator"
                            );
                        } else if matches!(self, HydroNode::ReduceKeyed { .. })
                            && self.metadata().location_kind.is_top_level()
                            && !(matches!(self.metadata().location_kind, LocationId::Atomic(_)))
                            && graph_builders.singleton_intermediates()
                        {
                            todo!();
                        } else {
                            let builder = graph_builders.get_dfir_mut(&out_location);
                            builder.add_dfir(
                                parse_quote! {
                                    #reduce_ident = #input_ident -> #operator::<#lifetime>(#f);
                                },
                                None,
                                Some(&next_stmt_id.to_string()),
                            );
                        }
                    }
                    BuildersOrCallback::Callback(_, node_callback) => {
                        node_callback(self, next_stmt_id);
                    }
                }

                *next_stmt_id += 1;

                reduce_ident
            }

            HydroNode::ReduceKeyedWatermark {
                f,
                input,
                watermark,
                ..
            } => {
                let (input, lifetime) = if input.metadata().location_kind.is_top_level() {
                    (input, quote!('static))
                } else {
                    (input, quote!('tick))
                };

                let input_ident =
                    input.emit_core::<D>(builders_or_callback, built_tees, next_stmt_id);

                let watermark_ident =
                    watermark.emit_core::<D>(builders_or_callback, built_tees, next_stmt_id);

                let chain_ident = syn::Ident::new(
                    &format!("reduce_keyed_watermark_chain_{}", *next_stmt_id),
                    Span::call_site(),
                );

                let fold_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        let builder = graph_builders.get_dfir_mut(&out_location);
                        // 1. Don't allow any values to be added to the map if the key <=the watermark
                        // 2. If the entry didn't exist in the BTreeMap, add it. Otherwise, call f.
                        //    If the watermark changed, delete all BTreeMap entries with a key < the watermark.
                        // 3. Convert the BTreeMap back into a stream of (k, v)
                        builder.add_dfir(
                            parse_quote! {
                                #chain_ident = chain();
                                #input_ident
                                    -> map(|x| (Some(x), None))
                                    -> [0]#chain_ident;
                                #watermark_ident
                                    -> map(|watermark| (None, Some(watermark)))
                                    -> [1]#chain_ident;

                                #fold_ident = #chain_ident
                                    -> fold::<#lifetime>(|| (::std::collections::HashMap::new(), None), {
                                        let __reduce_keyed_fn = #f;
                                        move |(map, opt_curr_watermark), (opt_payload, opt_watermark)| {
                                            if let Some((k, v)) = opt_payload {
                                                if let Some(curr_watermark) = *opt_curr_watermark {
                                                    if k <= curr_watermark {
                                                        return;
                                                    }
                                                }
                                                match map.entry(k) {
                                                    ::std::collections::hash_map::Entry::Vacant(e) => {
                                                        e.insert(v);
                                                    }
                                                    ::std::collections::hash_map::Entry::Occupied(mut e) => {
                                                        __reduce_keyed_fn(e.get_mut(), v);
                                                    }
                                                }
                                            } else {
                                                let watermark = opt_watermark.unwrap();
                                                if let Some(curr_watermark) = *opt_curr_watermark {
                                                    if watermark <= curr_watermark {
                                                        return;
                                                    }
                                                }
                                                *opt_curr_watermark = opt_watermark;
                                                map.retain(|k, _| *k > watermark);
                                            }
                                        }
                                    })
                                    -> flat_map(|(map, _curr_watermark)| map);
                            },
                            None,
                            Some(&next_stmt_id.to_string()),
                        );
                    }
                    BuildersOrCallback::Callback(_, node_callback) => {
                        node_callback(self, next_stmt_id);
                    }
                }

                *next_stmt_id += 1;

                fold_ident
            }

            HydroNode::Network {
                serialize_fn: serialize_pipeline,
                instantiate_fn,
                deserialize_fn: deserialize_pipeline,
                input,
                ..
            } => {
                let input_ident =
                    input.emit_core::<D>(builders_or_callback, built_tees, next_stmt_id);

                let receiver_stream_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        let (sink_expr, source_expr) = match instantiate_fn {
                            DebugInstantiate::Building => (
                                syn::parse_quote!(DUMMY_SINK),
                                syn::parse_quote!(DUMMY_SOURCE),
                            ),

                            DebugInstantiate::Finalized(finalized) => {
                                (finalized.sink.clone(), finalized.source.clone())
                            }
                        };

                        graph_builders.create_network(
                            &input.metadata().location_kind,
                            &out_location,
                            input_ident,
                            &receiver_stream_ident,
                            serialize_pipeline,
                            sink_expr,
                            source_expr,
                            deserialize_pipeline,
                            *next_stmt_id,
                        );
                    }
                    BuildersOrCallback::Callback(_, node_callback) => {
                        node_callback(self, next_stmt_id);
                    }
                }

                *next_stmt_id += 1;

                receiver_stream_ident
            }

            HydroNode::ExternalInput {
                instantiate_fn,
                deserialize_fn: deserialize_pipeline,
                ..
            } => {
                let receiver_stream_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        let (_, source_expr) = match instantiate_fn {
                            DebugInstantiate::Building => (
                                syn::parse_quote!(DUMMY_SINK),
                                syn::parse_quote!(DUMMY_SOURCE),
                            ),

                            DebugInstantiate::Finalized(finalized) => {
                                (finalized.sink.clone(), finalized.source.clone())
                            }
                        };

                        graph_builders.create_external_source(
                            &out_location,
                            source_expr,
                            &receiver_stream_ident,
                            deserialize_pipeline,
                            *next_stmt_id,
                        );
                    }
                    BuildersOrCallback::Callback(_, node_callback) => {
                        node_callback(self, next_stmt_id);
                    }
                }

                *next_stmt_id += 1;

                receiver_stream_ident
            }

            HydroNode::Counter {
                tag,
                duration,
                prefix,
                input,
                ..
            } => {
                let input_ident =
                    input.emit_core::<D>(builders_or_callback, built_tees, next_stmt_id);

                let counter_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        let builder = graph_builders.get_dfir_mut(&out_location);
                        builder.add_dfir(
                            parse_quote! {
                                #counter_ident = #input_ident -> _counter(#tag, #duration, #prefix);
                            },
                            None,
                            Some(&next_stmt_id.to_string()),
                        );
                    }
                    BuildersOrCallback::Callback(_, node_callback) => {
                        node_callback(self, next_stmt_id);
                    }
                }

                *next_stmt_id += 1;

                counter_ident
            }
        }
    }

    pub fn visit_debug_expr(&mut self, mut transform: impl FnMut(&mut DebugExpr)) {
        match self {
            HydroNode::Placeholder => {
                panic!()
            }
            HydroNode::Cast { .. } | HydroNode::ObserveNonDet { .. } => {}
            HydroNode::Source { source, .. } => match source {
                HydroSource::Stream(expr) | HydroSource::Iter(expr) => transform(expr),
                HydroSource::ExternalNetwork()
                | HydroSource::Spin()
                | HydroSource::ClusterMembers(_) => {} // TODO: what goes here?
            },
            HydroNode::SingletonSource { value, .. } => {
                transform(value);
            }
            HydroNode::CycleSource { .. }
            | HydroNode::Tee { .. }
            | HydroNode::YieldConcat { .. }
            | HydroNode::BeginAtomic { .. }
            | HydroNode::EndAtomic { .. }
            | HydroNode::Batch { .. }
            | HydroNode::Chain { .. }
            | HydroNode::ChainFirst { .. }
            | HydroNode::CrossProduct { .. }
            | HydroNode::CrossSingleton { .. }
            | HydroNode::ResolveFutures { .. }
            | HydroNode::ResolveFuturesOrdered { .. }
            | HydroNode::Join { .. }
            | HydroNode::Difference { .. }
            | HydroNode::AntiJoin { .. }
            | HydroNode::DeferTick { .. }
            | HydroNode::Enumerate { .. }
            | HydroNode::Unique { .. }
            | HydroNode::Sort { .. } => {}
            HydroNode::Map { f, .. }
            | HydroNode::FlatMap { f, .. }
            | HydroNode::Filter { f, .. }
            | HydroNode::FilterMap { f, .. }
            | HydroNode::Inspect { f, .. }
            | HydroNode::Reduce { f, .. }
            | HydroNode::ReduceKeyed { f, .. }
            | HydroNode::ReduceKeyedWatermark { f, .. } => {
                transform(f);
            }
            HydroNode::Fold { init, acc, .. }
            | HydroNode::Scan { init, acc, .. }
            | HydroNode::FoldKeyed { init, acc, .. } => {
                transform(init);
                transform(acc);
            }
            HydroNode::Network {
                serialize_fn,
                deserialize_fn,
                ..
            } => {
                if let Some(serialize_fn) = serialize_fn {
                    transform(serialize_fn);
                }
                if let Some(deserialize_fn) = deserialize_fn {
                    transform(deserialize_fn);
                }
            }
            HydroNode::ExternalInput { deserialize_fn, .. } => {
                if let Some(deserialize_fn) = deserialize_fn {
                    transform(deserialize_fn);
                }
            }
            HydroNode::Counter { duration, .. } => {
                transform(duration);
            }
        }
    }

    pub fn op_metadata(&self) -> &HydroIrOpMetadata {
        &self.metadata().op
    }

    pub fn metadata(&self) -> &HydroIrMetadata {
        match self {
            HydroNode::Placeholder => {
                panic!()
            }
            HydroNode::Cast { metadata, .. } => metadata,
            HydroNode::ObserveNonDet { metadata, .. } => metadata,
            HydroNode::Source { metadata, .. } => metadata,
            HydroNode::SingletonSource { metadata, .. } => metadata,
            HydroNode::CycleSource { metadata, .. } => metadata,
            HydroNode::Tee { metadata, .. } => metadata,
            HydroNode::YieldConcat { metadata, .. } => metadata,
            HydroNode::BeginAtomic { metadata, .. } => metadata,
            HydroNode::EndAtomic { metadata, .. } => metadata,
            HydroNode::Batch { metadata, .. } => metadata,
            HydroNode::Chain { metadata, .. } => metadata,
            HydroNode::ChainFirst { metadata, .. } => metadata,
            HydroNode::CrossProduct { metadata, .. } => metadata,
            HydroNode::CrossSingleton { metadata, .. } => metadata,
            HydroNode::Join { metadata, .. } => metadata,
            HydroNode::Difference { metadata, .. } => metadata,
            HydroNode::AntiJoin { metadata, .. } => metadata,
            HydroNode::ResolveFutures { metadata, .. } => metadata,
            HydroNode::ResolveFuturesOrdered { metadata, .. } => metadata,
            HydroNode::Map { metadata, .. } => metadata,
            HydroNode::FlatMap { metadata, .. } => metadata,
            HydroNode::Filter { metadata, .. } => metadata,
            HydroNode::FilterMap { metadata, .. } => metadata,
            HydroNode::DeferTick { metadata, .. } => metadata,
            HydroNode::Enumerate { metadata, .. } => metadata,
            HydroNode::Inspect { metadata, .. } => metadata,
            HydroNode::Unique { metadata, .. } => metadata,
            HydroNode::Sort { metadata, .. } => metadata,
            HydroNode::Scan { metadata, .. } => metadata,
            HydroNode::Fold { metadata, .. } => metadata,
            HydroNode::FoldKeyed { metadata, .. } => metadata,
            HydroNode::Reduce { metadata, .. } => metadata,
            HydroNode::ReduceKeyed { metadata, .. } => metadata,
            HydroNode::ReduceKeyedWatermark { metadata, .. } => metadata,
            HydroNode::ExternalInput { metadata, .. } => metadata,
            HydroNode::Network { metadata, .. } => metadata,
            HydroNode::Counter { metadata, .. } => metadata,
        }
    }

    pub fn op_metadata_mut(&mut self) -> &mut HydroIrOpMetadata {
        &mut self.metadata_mut().op
    }

    pub fn metadata_mut(&mut self) -> &mut HydroIrMetadata {
        match self {
            HydroNode::Placeholder => {
                panic!()
            }
            HydroNode::Cast { metadata, .. } => metadata,
            HydroNode::ObserveNonDet { metadata, .. } => metadata,
            HydroNode::Source { metadata, .. } => metadata,
            HydroNode::SingletonSource { metadata, .. } => metadata,
            HydroNode::CycleSource { metadata, .. } => metadata,
            HydroNode::Tee { metadata, .. } => metadata,
            HydroNode::YieldConcat { metadata, .. } => metadata,
            HydroNode::BeginAtomic { metadata, .. } => metadata,
            HydroNode::EndAtomic { metadata, .. } => metadata,
            HydroNode::Batch { metadata, .. } => metadata,
            HydroNode::Chain { metadata, .. } => metadata,
            HydroNode::ChainFirst { metadata, .. } => metadata,
            HydroNode::CrossProduct { metadata, .. } => metadata,
            HydroNode::CrossSingleton { metadata, .. } => metadata,
            HydroNode::Join { metadata, .. } => metadata,
            HydroNode::Difference { metadata, .. } => metadata,
            HydroNode::AntiJoin { metadata, .. } => metadata,
            HydroNode::ResolveFutures { metadata, .. } => metadata,
            HydroNode::ResolveFuturesOrdered { metadata, .. } => metadata,
            HydroNode::Map { metadata, .. } => metadata,
            HydroNode::FlatMap { metadata, .. } => metadata,
            HydroNode::Filter { metadata, .. } => metadata,
            HydroNode::FilterMap { metadata, .. } => metadata,
            HydroNode::DeferTick { metadata, .. } => metadata,
            HydroNode::Enumerate { metadata, .. } => metadata,
            HydroNode::Inspect { metadata, .. } => metadata,
            HydroNode::Unique { metadata, .. } => metadata,
            HydroNode::Sort { metadata, .. } => metadata,
            HydroNode::Scan { metadata, .. } => metadata,
            HydroNode::Fold { metadata, .. } => metadata,
            HydroNode::FoldKeyed { metadata, .. } => metadata,
            HydroNode::Reduce { metadata, .. } => metadata,
            HydroNode::ReduceKeyed { metadata, .. } => metadata,
            HydroNode::ReduceKeyedWatermark { metadata, .. } => metadata,
            HydroNode::ExternalInput { metadata, .. } => metadata,
            HydroNode::Network { metadata, .. } => metadata,
            HydroNode::Counter { metadata, .. } => metadata,
        }
    }

    pub fn input(&self) -> Vec<&HydroNode> {
        match self {
            HydroNode::Placeholder => {
                panic!()
            }
            HydroNode::Source { .. }
            | HydroNode::SingletonSource { .. }
            | HydroNode::ExternalInput { .. }
            | HydroNode::CycleSource { .. }
            | HydroNode::Tee { .. } => {
                // Tee should find its input in separate special ways
                vec![]
            }
            HydroNode::Cast { inner, .. }
            | HydroNode::ObserveNonDet { inner, .. }
            | HydroNode::YieldConcat { inner, .. }
            | HydroNode::BeginAtomic { inner, .. }
            | HydroNode::EndAtomic { inner, .. }
            | HydroNode::Batch { inner, .. } => {
                vec![inner]
            }
            HydroNode::Chain { first, second, .. } => {
                vec![first, second]
            }
            HydroNode::ChainFirst { first, second, .. } => {
                vec![first, second]
            }
            HydroNode::CrossProduct { left, right, .. }
            | HydroNode::CrossSingleton { left, right, .. }
            | HydroNode::Join { left, right, .. } => {
                vec![left, right]
            }
            HydroNode::Difference { pos, neg, .. } | HydroNode::AntiJoin { pos, neg, .. } => {
                vec![pos, neg]
            }
            HydroNode::Map { input, .. }
            | HydroNode::FlatMap { input, .. }
            | HydroNode::Filter { input, .. }
            | HydroNode::FilterMap { input, .. }
            | HydroNode::Sort { input, .. }
            | HydroNode::DeferTick { input, .. }
            | HydroNode::Enumerate { input, .. }
            | HydroNode::Inspect { input, .. }
            | HydroNode::Unique { input, .. }
            | HydroNode::Network { input, .. }
            | HydroNode::Counter { input, .. }
            | HydroNode::ResolveFutures { input, .. }
            | HydroNode::ResolveFuturesOrdered { input, .. }
            | HydroNode::Fold { input, .. }
            | HydroNode::FoldKeyed { input, .. }
            | HydroNode::Reduce { input, .. }
            | HydroNode::ReduceKeyed { input, .. }
            | HydroNode::Scan { input, .. } => {
                vec![input]
            }
            HydroNode::ReduceKeyedWatermark {
                input, watermark, ..
            } => {
                vec![input, watermark]
            }
        }
    }

    pub fn input_metadata(&self) -> Vec<&HydroIrMetadata> {
        self.input()
            .iter()
            .map(|input_node| input_node.metadata())
            .collect()
    }

    pub fn print_root(&self) -> String {
        match self {
            HydroNode::Placeholder => {
                panic!()
            }
            HydroNode::Cast { .. } => "Cast()".to_string(),
            HydroNode::ObserveNonDet { .. } => "ObserveNonDet()".to_string(),
            HydroNode::Source { source, .. } => format!("Source({:?})", source),
            HydroNode::SingletonSource { value, .. } => format!("SingletonSource({:?})", value),
            HydroNode::CycleSource { ident, .. } => format!("CycleSource({})", ident),
            HydroNode::Tee { inner, .. } => format!("Tee({})", inner.0.borrow().print_root()),
            HydroNode::YieldConcat { .. } => "YieldConcat()".to_string(),
            HydroNode::BeginAtomic { .. } => "BeginAtomic()".to_string(),
            HydroNode::EndAtomic { .. } => "EndAtomic()".to_string(),
            HydroNode::Batch { .. } => "Batch()".to_string(),
            HydroNode::Chain { first, second, .. } => {
                format!("Chain({}, {})", first.print_root(), second.print_root())
            }
            HydroNode::ChainFirst { first, second, .. } => {
                format!(
                    "ChainFirst({}, {})",
                    first.print_root(),
                    second.print_root()
                )
            }
            HydroNode::CrossProduct { left, right, .. } => {
                format!(
                    "CrossProduct({}, {})",
                    left.print_root(),
                    right.print_root()
                )
            }
            HydroNode::CrossSingleton { left, right, .. } => {
                format!(
                    "CrossSingleton({}, {})",
                    left.print_root(),
                    right.print_root()
                )
            }
            HydroNode::Join { left, right, .. } => {
                format!("Join({}, {})", left.print_root(), right.print_root())
            }
            HydroNode::Difference { pos, neg, .. } => {
                format!("Difference({}, {})", pos.print_root(), neg.print_root())
            }
            HydroNode::AntiJoin { pos, neg, .. } => {
                format!("AntiJoin({}, {})", pos.print_root(), neg.print_root())
            }
            HydroNode::ResolveFutures { .. } => "ResolveFutures()".to_string(),
            HydroNode::ResolveFuturesOrdered { .. } => "ResolveFuturesOrdered()".to_string(),
            HydroNode::Map { f, .. } => format!("Map({:?})", f),
            HydroNode::FlatMap { f, .. } => format!("FlatMap({:?})", f),
            HydroNode::Filter { f, .. } => format!("Filter({:?})", f),
            HydroNode::FilterMap { f, .. } => format!("FilterMap({:?})", f),
            HydroNode::DeferTick { .. } => "DeferTick()".to_string(),
            HydroNode::Enumerate { .. } => "Enumerate()".to_string(),
            HydroNode::Inspect { f, .. } => format!("Inspect({:?})", f),
            HydroNode::Unique { .. } => "Unique()".to_string(),
            HydroNode::Sort { .. } => "Sort()".to_string(),
            HydroNode::Fold { init, acc, .. } => format!("Fold({:?}, {:?})", init, acc),
            HydroNode::Scan { init, acc, .. } => format!("Scan({:?}, {:?})", init, acc),
            HydroNode::FoldKeyed { init, acc, .. } => format!("FoldKeyed({:?}, {:?})", init, acc),
            HydroNode::Reduce { f, .. } => format!("Reduce({:?})", f),
            HydroNode::ReduceKeyed { f, .. } => format!("ReduceKeyed({:?})", f),
            HydroNode::ReduceKeyedWatermark { f, .. } => format!("ReduceKeyedWatermark({:?})", f),
            HydroNode::Network { .. } => "Network()".to_string(),
            HydroNode::ExternalInput { .. } => "ExternalInput()".to_string(),
            HydroNode::Counter { tag, duration, .. } => {
                format!("Counter({:?}, {:?})", tag, duration)
            }
        }
    }
}

#[cfg(feature = "build")]
fn instantiate_network<'a, D>(
    from_location: &LocationId,
    to_location: &LocationId,
    processes: &HashMap<usize, D::Process>,
    clusters: &HashMap<usize, D::Cluster>,
) -> (syn::Expr, syn::Expr, Box<dyn FnOnce()>)
where
    D: Deploy<'a>,
{
    let ((sink, source), connect_fn) = match (from_location, to_location) {
        (LocationId::Process(from), LocationId::Process(to)) => {
            let from_node = processes
                .get(from)
                .unwrap_or_else(|| {
                    panic!("A process used in the graph was not instantiated: {}", from)
                })
                .clone();
            let to_node = processes
                .get(to)
                .unwrap_or_else(|| {
                    panic!("A process used in the graph was not instantiated: {}", to)
                })
                .clone();

            let sink_port = D::allocate_process_port(&from_node);
            let source_port = D::allocate_process_port(&to_node);

            (
                D::o2o_sink_source(&from_node, &sink_port, &to_node, &source_port),
                D::o2o_connect(&from_node, &sink_port, &to_node, &source_port),
            )
        }
        (LocationId::Process(from), LocationId::Cluster(to)) => {
            let from_node = processes
                .get(from)
                .unwrap_or_else(|| {
                    panic!("A process used in the graph was not instantiated: {}", from)
                })
                .clone();
            let to_node = clusters
                .get(to)
                .unwrap_or_else(|| {
                    panic!("A cluster used in the graph was not instantiated: {}", to)
                })
                .clone();

            let sink_port = D::allocate_process_port(&from_node);
            let source_port = D::allocate_cluster_port(&to_node);

            (
                D::o2m_sink_source(&from_node, &sink_port, &to_node, &source_port),
                D::o2m_connect(&from_node, &sink_port, &to_node, &source_port),
            )
        }
        (LocationId::Cluster(from), LocationId::Process(to)) => {
            let from_node = clusters
                .get(from)
                .unwrap_or_else(|| {
                    panic!("A cluster used in the graph was not instantiated: {}", from)
                })
                .clone();
            let to_node = processes
                .get(to)
                .unwrap_or_else(|| {
                    panic!("A process used in the graph was not instantiated: {}", to)
                })
                .clone();

            let sink_port = D::allocate_cluster_port(&from_node);
            let source_port = D::allocate_process_port(&to_node);

            (
                D::m2o_sink_source(&from_node, &sink_port, &to_node, &source_port),
                D::m2o_connect(&from_node, &sink_port, &to_node, &source_port),
            )
        }
        (LocationId::Cluster(from), LocationId::Cluster(to)) => {
            let from_node = clusters
                .get(from)
                .unwrap_or_else(|| {
                    panic!("A cluster used in the graph was not instantiated: {}", from)
                })
                .clone();
            let to_node = clusters
                .get(to)
                .unwrap_or_else(|| {
                    panic!("A cluster used in the graph was not instantiated: {}", to)
                })
                .clone();

            let sink_port = D::allocate_cluster_port(&from_node);
            let source_port = D::allocate_cluster_port(&to_node);

            (
                D::m2m_sink_source(&from_node, &sink_port, &to_node, &source_port),
                D::m2m_connect(&from_node, &sink_port, &to_node, &source_port),
            )
        }
        (LocationId::Tick(_, _), _) => panic!(),
        (_, LocationId::Tick(_, _)) => panic!(),
        (LocationId::Atomic(_), _) => panic!(),
        (_, LocationId::Atomic(_)) => panic!(),
    };
    (sink, source, connect_fn)
}

#[cfg(test)]
mod test {
    use std::mem::size_of;

    use stageleft::{QuotedWithContext, q};

    use super::*;

    #[test]
    #[cfg_attr(
        not(feature = "build"),
        ignore = "expects inclusion of feature-gated fields"
    )]
    fn hydro_node_size() {
        assert_eq!(size_of::<HydroNode>(), 272);
    }

    #[test]
    #[cfg_attr(
        not(feature = "build"),
        ignore = "expects inclusion of feature-gated fields"
    )]
    fn hydro_root_size() {
        assert_eq!(size_of::<HydroRoot>(), 168);
    }

    #[test]
    fn test_simplify_q_macro_basic() {
        // Test basic non-q! expression
        let simple_expr: syn::Expr = syn::parse_str("x + y").unwrap();
        let result = simplify_q_macro(simple_expr.clone());
        assert_eq!(result, simple_expr);
    }

    #[test]
    fn test_simplify_q_macro_actual_stageleft_call() {
        // Test a simplified version of what a real stageleft call might look like
        let stageleft_call = q!(|x: usize| x + 1).splice_fn1_ctx(&());
        let result = simplify_q_macro(stageleft_call);
        // This should be processed by our visitor and simplified to q!(...)
        // since we detect the stageleft::runtime_support::fn_* pattern
        hydro_build_utils::assert_snapshot!(result.to_token_stream().to_string());
    }

    #[test]
    fn test_closure_no_pipe_at_start() {
        // Test a closure that does not start with a pipe
        let stageleft_call = q!({
            let foo = 123;
            move |b: usize| b + foo
        })
        .splice_fn1_ctx(&());
        let result = simplify_q_macro(stageleft_call);
        hydro_build_utils::assert_snapshot!(result.to_token_stream().to_string());
    }
}
