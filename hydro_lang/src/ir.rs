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

use crate::NetworkHint;
#[cfg(stageleft_runtime)]
use crate::backtrace::Backtrace;
#[cfg(feature = "build")]
use crate::deploy::{Deploy, RegisterPort};
use crate::location::LocationId;

/// Represents the kind of stream/collection type for metadata
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StreamKind {
    /// Regular stream
    Stream,
    /// Keyed stream
    KeyedStream,
    /// Singleton collection
    Singleton,
    /// Optional collection
    Optional,
}

/// Debug displays the type's tokens.
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
#[derive(Clone, Hash)]
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
}

#[cfg(feature = "build")]
pub enum BuildersOrCallback<'a, L, N>
where
    L: FnMut(&mut HydroLeaf, &mut usize),
    N: FnMut(&mut HydroNode, &mut usize),
{
    Builders(&'a mut BTreeMap<usize, FlatGraphBuilder>),
    Callback(L, N),
}

/// An leaf in a Hydro graph, which is an pipeline that doesn't emit
/// any downstream values. Traversals over the dataflow graph and
/// generating DFIR IR start from leaves.
#[derive(Debug, Hash)]
pub enum HydroLeaf {
    ForEach {
        f: DebugExpr,
        input: Box<HydroNode>,
        metadata: HydroIrMetadata,
    },
    SendExternal {
        to_external_id: usize,
        to_key: usize,
        to_many: bool,
        serialize_fn: Option<DebugExpr>,
        instantiate_fn: DebugInstantiate,
        input: Box<HydroNode>,
    },
    DestSink {
        sink: DebugExpr,
        input: Box<HydroNode>,
        metadata: HydroIrMetadata,
    },
    CycleSink {
        ident: syn::Ident,
        input: Box<HydroNode>,
        metadata: HydroIrMetadata,
    },
}

impl HydroLeaf {
    #[cfg(feature = "build")]
    pub fn compile_network<'a, D>(
        &mut self,
        compile_env: &D::CompileEnv,
        extra_stmts: &mut BTreeMap<usize, Vec<syn::Stmt>>,
        seen_tees: &mut SeenTees,
        processes: &HashMap<usize, D::Process>,
        clusters: &HashMap<usize, D::Cluster>,
        externals: &HashMap<usize, D::External>,
    ) where
        D: Deploy<'a>,
    {
        self.transform_bottom_up(
            &mut |l| {
                if let HydroLeaf::SendExternal {
                    input,
                    to_external_id,
                    to_key,
                    to_many,
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
                                        let source_port = D::allocate_external_port(&to_node);

                                        to_node.register(*to_key, source_port.clone());

                                        (
                                            (
                                                D::o2e_sink(compile_env, &from_node, &sink_port, &to_node, &source_port),
                                                parse_quote!(DUMMY),
                                            ),
                                            D::o2e_connect(&from_node, &sink_port, &to_node, &source_port),
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
                            compile_env,
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
                                                    compile_env,
                                                    extra_stmts.entry(*process_id).or_default(),
                                                    &to_node, &source_port,
                                                    codec_type.0.as_ref(),
                                                    format!("{}_{}", *from_external_id, *from_key)
                                                )
                                            } else {
                                                D::e2o_source(compile_env, &from_node, &sink_port, &to_node, &source_port)
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
                if let HydroLeaf::SendExternal { instantiate_fn, .. } = l {
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
        transform_leaf: &mut impl FnMut(&mut HydroLeaf),
        transform_node: &mut impl FnMut(&mut HydroNode),
        seen_tees: &mut SeenTees,
        check_well_formed: bool,
    ) {
        self.transform_children(
            |n, s| n.transform_bottom_up(transform_node, s, check_well_formed),
            seen_tees,
        );

        transform_leaf(self);
    }

    pub fn transform_children(
        &mut self,
        mut transform: impl FnMut(&mut HydroNode, &mut SeenTees),
        seen_tees: &mut SeenTees,
    ) {
        match self {
            HydroLeaf::ForEach { input, .. }
            | HydroLeaf::SendExternal { input, .. }
            | HydroLeaf::DestSink { input, .. }
            | HydroLeaf::CycleSink { input, .. } => {
                transform(input, seen_tees);
            }
        }
    }

    pub fn deep_clone(&self, seen_tees: &mut SeenTees) -> HydroLeaf {
        match self {
            HydroLeaf::ForEach { f, input, metadata } => HydroLeaf::ForEach {
                f: f.clone(),
                input: Box::new(input.deep_clone(seen_tees)),
                metadata: metadata.clone(),
            },
            HydroLeaf::SendExternal {
                to_external_id,
                to_key,
                to_many,
                serialize_fn,
                instantiate_fn,
                input,
            } => HydroLeaf::SendExternal {
                to_external_id: *to_external_id,
                to_key: *to_key,
                to_many: *to_many,
                serialize_fn: serialize_fn.clone(),
                instantiate_fn: instantiate_fn.clone(),
                input: Box::new(input.deep_clone(seen_tees)),
            },
            HydroLeaf::DestSink {
                sink,
                input,
                metadata,
            } => HydroLeaf::DestSink {
                sink: sink.clone(),
                input: Box::new(input.deep_clone(seen_tees)),
                metadata: metadata.clone(),
            },
            HydroLeaf::CycleSink {
                ident,
                input,
                metadata,
            } => HydroLeaf::CycleSink {
                ident: ident.clone(),
                input: Box::new(input.deep_clone(seen_tees)),
                metadata: metadata.clone(),
            },
        }
    }

    #[cfg(feature = "build")]
    pub fn emit(
        &mut self,
        graph_builders: &mut BTreeMap<usize, FlatGraphBuilder>,
        built_tees: &mut HashMap<*const RefCell<HydroNode>, (syn::Ident, usize)>,
        next_stmt_id: &mut usize,
    ) {
        self.emit_core(
            &mut BuildersOrCallback::Builders::<
                fn(&mut HydroLeaf, &mut usize),
                fn(&mut HydroNode, &mut usize),
            >(graph_builders),
            built_tees,
            next_stmt_id,
        );
    }

    #[cfg(feature = "build")]
    pub fn emit_core(
        &mut self,
        builders_or_callback: &mut BuildersOrCallback<
            impl FnMut(&mut HydroLeaf, &mut usize),
            impl FnMut(&mut HydroNode, &mut usize),
        >,
        built_tees: &mut HashMap<*const RefCell<HydroNode>, (syn::Ident, usize)>,
        next_stmt_id: &mut usize,
    ) {
        match self {
            HydroLeaf::ForEach { f, input, .. } => {
                let (input_ident, input_location_id) =
                    input.emit_core(builders_or_callback, built_tees, next_stmt_id);

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        graph_builders
                            .entry(input_location_id)
                            .or_default()
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

            HydroLeaf::SendExternal {
                serialize_fn,
                instantiate_fn,
                input,
                ..
            } => {
                let (input_ident, input_location_id) =
                    input.emit_core(builders_or_callback, built_tees, next_stmt_id);

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

                        let sender_builder = graph_builders.entry(input_location_id).or_default();
                        if let Some(serialize_fn) = serialize_fn {
                            sender_builder.add_dfir(
                                parse_quote! {
                                    #input_ident -> map(#serialize_fn) -> dest_sink(#sink_expr);
                                },
                                None,
                                // operator tag separates send and receive, which otherwise have the same next_stmt_id
                                Some(&format!("send{}", next_stmt_id)),
                            );
                        } else {
                            sender_builder.add_dfir(
                                parse_quote! {
                                    #input_ident -> dest_sink(#sink_expr);
                                },
                                None,
                                Some(&format!("send{}", next_stmt_id)),
                            );
                        }
                    }
                    BuildersOrCallback::Callback(leaf_callback, _) => {
                        leaf_callback(self, next_stmt_id);
                    }
                }

                *next_stmt_id += 1;
            }

            HydroLeaf::DestSink { sink, input, .. } => {
                let (input_ident, input_location_id) =
                    input.emit_core(builders_or_callback, built_tees, next_stmt_id);

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        graph_builders
                            .entry(input_location_id)
                            .or_default()
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

            HydroLeaf::CycleSink {
                ident,
                input,
                metadata,
                ..
            } => {
                let (input_ident, input_location_id) =
                    input.emit_core(builders_or_callback, built_tees, next_stmt_id);

                let location_id = metadata.location_kind.root().raw_id();

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        assert_eq!(
                            input_location_id, location_id,
                            "cycle_sink location mismatch"
                        );

                        graph_builders.entry(location_id).or_default().add_dfir(
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

    pub fn metadata(&self) -> &HydroIrMetadata {
        match self {
            HydroLeaf::ForEach { metadata, .. }
            | HydroLeaf::DestSink { metadata, .. }
            | HydroLeaf::CycleSink { metadata, .. } => metadata,
            HydroLeaf::SendExternal { .. } => panic!(),
        }
    }

    pub fn metadata_mut(&mut self) -> &mut HydroIrMetadata {
        match self {
            HydroLeaf::ForEach { metadata, .. }
            | HydroLeaf::DestSink { metadata, .. }
            | HydroLeaf::CycleSink { metadata, .. } => metadata,
            HydroLeaf::SendExternal { .. } => panic!(),
        }
    }

    pub fn input_metadata(&self) -> Vec<&HydroIrMetadata> {
        match self {
            HydroLeaf::ForEach { input, .. }
            | HydroLeaf::SendExternal { input, .. }
            | HydroLeaf::DestSink { input, .. }
            | HydroLeaf::CycleSink { input, .. } => {
                vec![input.metadata()]
            }
        }
    }

    pub fn print_root(&self) -> String {
        match self {
            HydroLeaf::ForEach { f, .. } => format!("ForEach({:?})", f),
            HydroLeaf::SendExternal { .. } => "SendExternal".to_string(),
            HydroLeaf::DestSink { sink, .. } => format!("DestSink({:?})", sink),
            HydroLeaf::CycleSink { ident, .. } => format!("CycleSink({:?})", ident),
        }
    }

    pub fn visit_debug_expr(&mut self, mut transform: impl FnMut(&mut DebugExpr)) {
        match self {
            HydroLeaf::ForEach { f, .. } | HydroLeaf::DestSink { sink: f, .. } => {
                transform(f);
            }
            HydroLeaf::SendExternal { .. } | HydroLeaf::CycleSink { .. } => {}
        }
    }
}

#[cfg(feature = "build")]
pub fn emit(ir: &mut Vec<HydroLeaf>) -> BTreeMap<usize, FlatGraphBuilder> {
    let mut builders = BTreeMap::new();
    let mut built_tees = HashMap::new();
    let mut next_stmt_id = 0;
    for leaf in ir {
        leaf.emit(&mut builders, &mut built_tees, &mut next_stmt_id);
    }
    builders
}

#[cfg(feature = "build")]
pub fn traverse_dfir(
    ir: &mut [HydroLeaf],
    transform_leaf: impl FnMut(&mut HydroLeaf, &mut usize),
    transform_node: impl FnMut(&mut HydroNode, &mut usize),
) {
    let mut seen_tees = HashMap::new();
    let mut next_stmt_id = 0;
    let mut callback = BuildersOrCallback::Callback(transform_leaf, transform_node);
    ir.iter_mut().for_each(|leaf| {
        leaf.emit_core(&mut callback, &mut seen_tees, &mut next_stmt_id);
    });
}

pub fn transform_bottom_up(
    ir: &mut [HydroLeaf],
    transform_leaf: &mut impl FnMut(&mut HydroLeaf),
    transform_node: &mut impl FnMut(&mut HydroNode),
    check_well_formed: bool,
) {
    let mut seen_tees = HashMap::new();
    ir.iter_mut().for_each(|leaf| {
        leaf.transform_bottom_up(
            transform_leaf,
            transform_node,
            &mut seen_tees,
            check_well_formed,
        );
    });
}

pub fn deep_clone(ir: &[HydroLeaf]) -> Vec<HydroLeaf> {
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

#[derive(Clone)]
pub struct HydroIrMetadata {
    pub location_kind: LocationId,
    pub backtrace: Backtrace,
    pub output_type: Option<DebugType>,
    /// The kind of stream/collection
    pub stream_kind: Option<StreamKind>,
    /// Whether this collection is bounded (finite) or unbounded (potentially infinite)
    pub is_bounded: bool,
    pub cardinality: Option<usize>,
    pub cpu_usage: Option<f64>,
    pub network_recv_cpu_usage: Option<f64>,
    pub id: Option<usize>,
    pub tag: Option<String>,
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
            .field("output_type", &self.output_type)
            .field("stream_kind", &self.stream_kind)
            .field("is_bounded", &self.is_bounded)
            .finish()
    }
}

/// An intermediate node in a Hydro graph, which consumes data
/// from upstream nodes and emits data to downstream nodes.
#[derive(Debug, Hash)]
pub enum HydroNode {
    Placeholder,

    Source {
        source: HydroSource,
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

    Persist {
        inner: Box<HydroNode>,
        metadata: HydroIrMetadata,
    },

    Unpersist {
        inner: Box<HydroNode>,
        metadata: HydroIrMetadata,
    },

    Delta {
        inner: Box<HydroNode>,
        metadata: HydroIrMetadata,
    },

    Chain {
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
        is_static: bool,
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

            HydroNode::Persist { inner, .. }
            | HydroNode::Unpersist { inner, .. }
            | HydroNode::Delta { inner, .. } => {
                transform(inner.as_mut(), seen_tees);
            }

            HydroNode::Chain { first, second, .. } => {
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
            HydroNode::Source { source, metadata } => HydroNode::Source {
                source: source.clone(),
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
            HydroNode::Persist { inner, metadata } => HydroNode::Persist {
                inner: Box::new(inner.deep_clone(seen_tees)),
                metadata: metadata.clone(),
            },
            HydroNode::Unpersist { inner, metadata } => HydroNode::Unpersist {
                inner: Box::new(inner.deep_clone(seen_tees)),
                metadata: metadata.clone(),
            },
            HydroNode::Delta { inner, metadata } => HydroNode::Delta {
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
            HydroNode::Enumerate {
                is_static,
                input,
                metadata,
            } => HydroNode::Enumerate {
                is_static: *is_static,
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
                input,
                metadata,
            } => HydroNode::Counter {
                tag: tag.clone(),
                duration: duration.clone(),
                input: Box::new(input.deep_clone(seen_tees)),
                metadata: metadata.clone(),
            },
        }
    }

    #[cfg(feature = "build")]
    pub fn emit_core(
        &mut self,
        builders_or_callback: &mut BuildersOrCallback<
            impl FnMut(&mut HydroLeaf, &mut usize),
            impl FnMut(&mut HydroNode, &mut usize),
        >,
        built_tees: &mut HashMap<*const RefCell<HydroNode>, (syn::Ident, usize)>,
        next_stmt_id: &mut usize,
    ) -> (syn::Ident, usize) {
        match self {
            HydroNode::Placeholder => {
                panic!()
            }

            HydroNode::Persist { inner, .. } => {
                let (inner_ident, location) =
                    inner.emit_core(builders_or_callback, built_tees, next_stmt_id);

                let persist_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        let builder = graph_builders.entry(location).or_default();
                        builder.add_dfir(
                            parse_quote! {
                                #persist_ident = #inner_ident -> persist::<'static>();
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

                (persist_ident, location)
            }

            HydroNode::Unpersist { .. } => {
                panic!(
                    "Unpersist is a marker node and should have been optimized away. This is likely a compiler bug."
                )
            }

            HydroNode::Delta { inner, .. } => {
                let (inner_ident, location) =
                    inner.emit_core(builders_or_callback, built_tees, next_stmt_id);

                let delta_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        let builder = graph_builders.entry(location).or_default();
                        builder.add_dfir(
                            parse_quote! {
                                #delta_ident = #inner_ident -> multiset_delta();
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

                (delta_ident, location)
            }

            HydroNode::Source {
                source, metadata, ..
            } => {
                let location_id = metadata.location_kind.root().raw_id();

                if let HydroSource::ExternalNetwork() = source {
                    (syn::Ident::new("DUMMY", Span::call_site()), location_id)
                } else {
                    let source_ident =
                        syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                    let source_stmt = match source {
                        HydroSource::Stream(expr) => {
                            parse_quote! {
                                #source_ident = source_stream(#expr);
                            }
                        }

                        HydroSource::ExternalNetwork() => {
                            unreachable!()
                        }

                        HydroSource::Iter(expr) => {
                            parse_quote! {
                                #source_ident = source_iter(#expr);
                            }
                        }

                        HydroSource::Spin() => {
                            parse_quote! {
                                #source_ident = spin();
                            }
                        }
                    };

                    match builders_or_callback {
                        BuildersOrCallback::Builders(graph_builders) => {
                            let builder = graph_builders.entry(location_id).or_default();
                            builder.add_dfir(source_stmt, None, Some(&next_stmt_id.to_string()));
                        }
                        BuildersOrCallback::Callback(_, node_callback) => {
                            node_callback(self, next_stmt_id);
                        }
                    }

                    *next_stmt_id += 1;

                    (source_ident, location_id)
                }
            }

            HydroNode::CycleSource {
                ident, metadata, ..
            } => {
                let location_id = metadata.location_kind.root().raw_id();

                let ident = ident.clone();

                match builders_or_callback {
                    BuildersOrCallback::Builders(_) => {}
                    BuildersOrCallback::Callback(_, node_callback) => {
                        node_callback(self, next_stmt_id);
                    }
                }

                // consume a stmt id even though we did not emit anything so that we can instrument this
                *next_stmt_id += 1;

                (ident, location_id)
            }

            HydroNode::Tee { inner, .. } => {
                let (ret_ident, inner_location_id) = if let Some((teed_from, inner_location_id)) =
                    built_tees.get(&(inner.0.as_ref() as *const RefCell<HydroNode>))
                {
                    match builders_or_callback {
                        BuildersOrCallback::Builders(_) => {}
                        BuildersOrCallback::Callback(_, node_callback) => {
                            node_callback(self, next_stmt_id);
                        }
                    }

                    (teed_from.clone(), *inner_location_id)
                } else {
                    let (inner_ident, inner_location_id) = inner.0.borrow_mut().emit_core(
                        builders_or_callback,
                        built_tees,
                        next_stmt_id,
                    );

                    let tee_ident =
                        syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                    built_tees.insert(
                        inner.0.as_ref() as *const RefCell<HydroNode>,
                        (tee_ident.clone(), inner_location_id),
                    );

                    match builders_or_callback {
                        BuildersOrCallback::Builders(graph_builders) => {
                            let builder = graph_builders.entry(inner_location_id).or_default();
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

                    (tee_ident, inner_location_id)
                };

                // we consume a stmt id regardless of if we emit the tee() operator,
                // so that during rewrites we touch all recipients of the tee()

                *next_stmt_id += 1;
                (ret_ident, inner_location_id)
            }

            HydroNode::Chain { first, second, .. } => {
                let (first_ident, first_location_id) =
                    first.emit_core(builders_or_callback, built_tees, next_stmt_id);
                let (second_ident, second_location_id) =
                    second.emit_core(builders_or_callback, built_tees, next_stmt_id);

                let chain_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        assert_eq!(
                            first_location_id, second_location_id,
                            "chain inputs must be in the same location"
                        );
                        let builder = graph_builders.entry(first_location_id).or_default();
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

                (chain_ident, first_location_id)
            }

            HydroNode::CrossSingleton { left, right, .. } => {
                let (left_ident, left_location_id) =
                    left.emit_core(builders_or_callback, built_tees, next_stmt_id);
                let (right_ident, right_location_id) =
                    right.emit_core(builders_or_callback, built_tees, next_stmt_id);

                let cross_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        assert_eq!(
                            left_location_id, right_location_id,
                            "cross_singleton inputs must be in the same location"
                        );

                        let builder = graph_builders.entry(left_location_id).or_default();
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

                (cross_ident, left_location_id)
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

                let (left_inner, left_lifetime) =
                    if let HydroNode::Persist { inner: left, .. } = left.as_mut() {
                        (left, quote!('static))
                    } else {
                        (left, quote!('tick))
                    };

                let (right_inner, right_lifetime) =
                    if let HydroNode::Persist { inner: right, .. } = right.as_mut() {
                        (right, quote!('static))
                    } else {
                        (right, quote!('tick))
                    };

                let (left_ident, left_location_id) =
                    left_inner.emit_core(builders_or_callback, built_tees, next_stmt_id);
                let (right_ident, right_location_id) =
                    right_inner.emit_core(builders_or_callback, built_tees, next_stmt_id);

                let stream_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        assert_eq!(
                            left_location_id, right_location_id,
                            "join / cross product inputs must be in the same location"
                        );

                        let builder = graph_builders.entry(left_location_id).or_default();
                        builder.add_dfir(
                            parse_quote! {
                                #stream_ident = #operator::<#left_lifetime, #right_lifetime>();
                                #left_ident -> [0]#stream_ident;
                                #right_ident -> [1]#stream_ident;
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

                (stream_ident, left_location_id)
            }

            HydroNode::Difference { .. } | HydroNode::AntiJoin { .. } => {
                let operator: syn::Ident = if matches!(self, HydroNode::Difference { .. }) {
                    parse_quote!(difference_multiset)
                } else {
                    parse_quote!(anti_join_multiset)
                };

                let (HydroNode::Difference { pos, neg, .. } | HydroNode::AntiJoin { pos, neg, .. }) =
                    self
                else {
                    unreachable!()
                };

                let (neg, neg_lifetime) =
                    if let HydroNode::Persist { inner: neg, .. } = neg.as_mut() {
                        (neg, quote!('static))
                    } else {
                        (neg, quote!('tick))
                    };

                let (pos_ident, pos_location_id) =
                    pos.emit_core(builders_or_callback, built_tees, next_stmt_id);
                let (neg_ident, neg_location_id) =
                    neg.emit_core(builders_or_callback, built_tees, next_stmt_id);

                let stream_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        assert_eq!(
                            pos_location_id, neg_location_id,
                            "difference / anti join inputs must be in the same location"
                        );

                        let builder = graph_builders.entry(pos_location_id).or_default();
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

                (stream_ident, pos_location_id)
            }

            HydroNode::ResolveFutures { input, .. } => {
                let (input_ident, input_location_id) =
                    input.emit_core(builders_or_callback, built_tees, next_stmt_id);

                let futures_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        let builder = graph_builders.entry(input_location_id).or_default();
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

                (futures_ident, input_location_id)
            }

            HydroNode::ResolveFuturesOrdered { input, .. } => {
                let (input_ident, input_location_id) =
                    input.emit_core(builders_or_callback, built_tees, next_stmt_id);

                let futures_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        let builder = graph_builders.entry(input_location_id).or_default();
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

                (futures_ident, input_location_id)
            }

            HydroNode::Map { f, input, .. } => {
                let (input_ident, input_location_id) =
                    input.emit_core(builders_or_callback, built_tees, next_stmt_id);

                let map_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        let builder = graph_builders.entry(input_location_id).or_default();
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

                (map_ident, input_location_id)
            }

            HydroNode::FlatMap { f, input, .. } => {
                let (input_ident, input_location_id) =
                    input.emit_core(builders_or_callback, built_tees, next_stmt_id);

                let flat_map_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        let builder = graph_builders.entry(input_location_id).or_default();
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

                (flat_map_ident, input_location_id)
            }

            HydroNode::Filter { f, input, .. } => {
                let (input_ident, input_location_id) =
                    input.emit_core(builders_or_callback, built_tees, next_stmt_id);

                let filter_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        let builder = graph_builders.entry(input_location_id).or_default();
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

                (filter_ident, input_location_id)
            }

            HydroNode::FilterMap { f, input, .. } => {
                let (input_ident, input_location_id) =
                    input.emit_core(builders_or_callback, built_tees, next_stmt_id);

                let filter_map_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        let builder = graph_builders.entry(input_location_id).or_default();
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

                (filter_map_ident, input_location_id)
            }

            HydroNode::Sort { input, .. } => {
                let (input_ident, input_location_id) =
                    input.emit_core(builders_or_callback, built_tees, next_stmt_id);

                let sort_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        let builder = graph_builders.entry(input_location_id).or_default();
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

                (sort_ident, input_location_id)
            }

            HydroNode::DeferTick { input, .. } => {
                let (input_ident, input_location_id) =
                    input.emit_core(builders_or_callback, built_tees, next_stmt_id);

                let defer_tick_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        let builder = graph_builders.entry(input_location_id).or_default();
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

                (defer_tick_ident, input_location_id)
            }

            HydroNode::Enumerate {
                is_static, input, ..
            } => {
                let (input_ident, input_location_id) =
                    input.emit_core(builders_or_callback, built_tees, next_stmt_id);

                let enumerate_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        let builder = graph_builders.entry(input_location_id).or_default();
                        let lifetime = if *is_static {
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

                (enumerate_ident, input_location_id)
            }

            HydroNode::Inspect { f, input, .. } => {
                let (input_ident, input_location_id) =
                    input.emit_core(builders_or_callback, built_tees, next_stmt_id);

                let inspect_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        let builder = graph_builders.entry(input_location_id).or_default();
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

                (inspect_ident, input_location_id)
            }

            HydroNode::Unique { input, .. } => {
                let (input_ident, input_location_id) =
                    input.emit_core(builders_or_callback, built_tees, next_stmt_id);

                let unique_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        let builder = graph_builders.entry(input_location_id).or_default();
                        builder.add_dfir(
                            parse_quote! {
                                #unique_ident = #input_ident -> unique::<'tick>();
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

                (unique_ident, input_location_id)
            }

            HydroNode::Fold { .. } | HydroNode::FoldKeyed { .. } | HydroNode::Scan { .. } => {
                let operator: syn::Ident = if matches!(self, HydroNode::Fold { .. }) {
                    parse_quote!(fold)
                } else if matches!(self, HydroNode::Scan { .. }) {
                    parse_quote!(scan)
                } else {
                    parse_quote!(fold_keyed)
                };

                let (HydroNode::Fold {
                    init, acc, input, ..
                }
                | HydroNode::FoldKeyed {
                    init, acc, input, ..
                }
                | HydroNode::Scan {
                    init, acc, input, ..
                }) = self
                else {
                    unreachable!()
                };

                let (input, lifetime) =
                    if let HydroNode::Persist { inner: input, .. } = input.as_mut() {
                        (input, quote!('static))
                    } else {
                        (input, quote!('tick))
                    };

                let (input_ident, input_location_id) =
                    input.emit_core(builders_or_callback, built_tees, next_stmt_id);

                let fold_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        let builder = graph_builders.entry(input_location_id).or_default();
                        builder.add_dfir(
                            parse_quote! {
                                #fold_ident = #input_ident -> #operator::<#lifetime>(#init, #acc);
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

                (fold_ident, input_location_id)
            }

            HydroNode::ReduceKeyedWatermark {
                f,
                input,
                watermark,
                ..
            } => {
                let (input, lifetime) =
                    if let HydroNode::Persist { inner: input, .. } = input.as_mut() {
                        (input, quote!('static))
                    } else {
                        (input, quote!('tick))
                    };

                let (input_ident, input_location_id) =
                    input.emit_core(builders_or_callback, built_tees, next_stmt_id);

                let (watermark_ident, watermark_location_id) =
                    watermark.emit_core(builders_or_callback, built_tees, next_stmt_id);

                let chain_ident = syn::Ident::new(
                    &format!("reduce_keyed_watermark_chain_{}", *next_stmt_id),
                    Span::call_site(),
                );

                let fold_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        assert_eq!(
                            input_location_id, watermark_location_id,
                            "ReduceKeyedWatermark inputs must be in the same location"
                        );

                        let builder = graph_builders.entry(input_location_id).or_default();
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

                (fold_ident, input_location_id)
            }

            HydroNode::Reduce { .. } | HydroNode::ReduceKeyed { .. } => {
                let operator: syn::Ident = if matches!(self, HydroNode::Reduce { .. }) {
                    parse_quote!(reduce)
                } else {
                    parse_quote!(reduce_keyed)
                };

                let (HydroNode::Reduce { f, input, .. } | HydroNode::ReduceKeyed { f, input, .. }) =
                    self
                else {
                    unreachable!()
                };

                let (input, lifetime) =
                    if let HydroNode::Persist { inner: input, .. } = input.as_mut() {
                        (input, quote!('static))
                    } else {
                        (input, quote!('tick))
                    };

                let (input_ident, input_location_id) =
                    input.emit_core(builders_or_callback, built_tees, next_stmt_id);

                let reduce_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        let builder = graph_builders.entry(input_location_id).or_default();
                        builder.add_dfir(
                            parse_quote! {
                                #reduce_ident = #input_ident -> #operator::<#lifetime>(#f);
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

                (reduce_ident, input_location_id)
            }

            HydroNode::Network {
                serialize_fn: serialize_pipeline,
                instantiate_fn,
                deserialize_fn: deserialize_pipeline,
                input,
                metadata,
                ..
            } => {
                let (input_ident, input_location_id) =
                    input.emit_core(builders_or_callback, built_tees, next_stmt_id);

                let to_id = metadata.location_kind.root().raw_id();

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

                        let sender_builder = graph_builders.entry(input_location_id).or_default();
                        if let Some(serialize_pipeline) = serialize_pipeline {
                            sender_builder.add_dfir(
                                parse_quote! {
                                    #input_ident -> map(#serialize_pipeline) -> dest_sink(#sink_expr);
                                },
                                None,
                                // operator tag separates send and receive, which otherwise have the same next_stmt_id
                                Some(&format!("send{}", next_stmt_id)),
                            );
                        } else {
                            sender_builder.add_dfir(
                                parse_quote! {
                                    #input_ident -> dest_sink(#sink_expr);
                                },
                                None,
                                Some(&format!("send{}", next_stmt_id)),
                            );
                        }

                        let receiver_builder = graph_builders.entry(to_id).or_default();
                        if let Some(deserialize_pipeline) = deserialize_pipeline {
                            receiver_builder.add_dfir(parse_quote! {
                                #receiver_stream_ident = source_stream(#source_expr) -> map(#deserialize_pipeline);
                            }, None, Some(&format!("recv{}", next_stmt_id)));
                        } else {
                            receiver_builder.add_dfir(
                                parse_quote! {
                                    #receiver_stream_ident = source_stream(#source_expr);
                                },
                                None,
                                Some(&format!("recv{}", next_stmt_id)),
                            );
                        }
                    }
                    BuildersOrCallback::Callback(_, node_callback) => {
                        node_callback(self, next_stmt_id);
                    }
                }

                *next_stmt_id += 1;

                (receiver_stream_ident, to_id)
            }

            HydroNode::ExternalInput {
                instantiate_fn,
                deserialize_fn: deserialize_pipeline,
                metadata,
                ..
            } => {
                let to_id = metadata.location_kind.root().raw_id();

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

                        let receiver_builder = graph_builders.entry(to_id).or_default();
                        if let Some(deserialize_pipeline) = deserialize_pipeline {
                            receiver_builder.add_dfir(parse_quote! {
                                #receiver_stream_ident = source_stream(#source_expr) -> map(#deserialize_pipeline);
                            }, None, Some(&format!("recv{}", next_stmt_id)));
                        } else {
                            receiver_builder.add_dfir(
                                parse_quote! {
                                    #receiver_stream_ident = source_stream(#source_expr);
                                },
                                None,
                                Some(&format!("recv{}", next_stmt_id)),
                            );
                        }
                    }
                    BuildersOrCallback::Callback(_, node_callback) => {
                        node_callback(self, next_stmt_id);
                    }
                }

                *next_stmt_id += 1;

                (receiver_stream_ident, to_id)
            }

            HydroNode::Counter {
                tag,
                duration,
                input,
                ..
            } => {
                let (input_ident, input_location_id) =
                    input.emit_core(builders_or_callback, built_tees, next_stmt_id);

                let counter_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        let builder = graph_builders.entry(input_location_id).or_default();
                        builder.add_dfir(
                            parse_quote! {
                                #counter_ident = #input_ident -> _counter(#tag, #duration);
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

                (counter_ident, input_location_id)
            }
        }
    }

    pub fn visit_debug_expr(&mut self, mut transform: impl FnMut(&mut DebugExpr)) {
        match self {
            HydroNode::Placeholder => {
                panic!()
            }
            HydroNode::Source { source, .. } => match source {
                HydroSource::Stream(expr) | HydroSource::Iter(expr) => transform(expr),
                HydroSource::ExternalNetwork() | HydroSource::Spin() => {}
            },
            HydroNode::CycleSource { .. }
            | HydroNode::Tee { .. }
            | HydroNode::Persist { .. }
            | HydroNode::Unpersist { .. }
            | HydroNode::Delta { .. }
            | HydroNode::Chain { .. }
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

    pub fn metadata(&self) -> &HydroIrMetadata {
        match self {
            HydroNode::Placeholder => {
                panic!()
            }
            HydroNode::Source { metadata, .. } => metadata,
            HydroNode::CycleSource { metadata, .. } => metadata,
            HydroNode::Tee { metadata, .. } => metadata,
            HydroNode::Persist { metadata, .. } => metadata,
            HydroNode::Unpersist { metadata, .. } => metadata,
            HydroNode::Delta { metadata, .. } => metadata,
            HydroNode::Chain { metadata, .. } => metadata,
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

    pub fn metadata_mut(&mut self) -> &mut HydroIrMetadata {
        match self {
            HydroNode::Placeholder => {
                panic!()
            }
            HydroNode::Source { metadata, .. } => metadata,
            HydroNode::CycleSource { metadata, .. } => metadata,
            HydroNode::Tee { metadata, .. } => metadata,
            HydroNode::Persist { metadata, .. } => metadata,
            HydroNode::Unpersist { metadata, .. } => metadata,
            HydroNode::Delta { metadata, .. } => metadata,
            HydroNode::Chain { metadata, .. } => metadata,
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

    pub fn input_metadata(&self) -> Vec<&HydroIrMetadata> {
        match self {
            HydroNode::Placeholder => {
                panic!()
            }
            HydroNode::Source { .. }
            | HydroNode::ExternalInput { .. }
            | HydroNode::CycleSource { .. } // CycleSource and Tee should calculate input metadata in separate special ways
            | HydroNode::Tee { .. } => {
                vec![]
            }
            HydroNode::Persist { inner, .. }
            | HydroNode::Unpersist { inner, .. }
            | HydroNode::Delta { inner, .. } => {
                vec![inner.metadata()]
            }
            HydroNode::Chain { first, second, .. } => {
                vec![first.metadata(), second.metadata()]
            }
            HydroNode::CrossProduct { left, right, .. }
            | HydroNode::CrossSingleton { left, right, .. }
            | HydroNode::Join { left, right, .. } => {
                vec![left.metadata(), right.metadata()]
            }
            HydroNode::Difference { pos, neg, .. } | HydroNode::AntiJoin { pos, neg, .. } => {
                vec![pos.metadata(), neg.metadata()]
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
            | HydroNode::ResolveFuturesOrdered { input, .. } => {
                vec![input.metadata()]
            }
            HydroNode::Fold { input, .. }
            | HydroNode::FoldKeyed { input, .. }
            | HydroNode::Reduce { input, .. }
            | HydroNode::ReduceKeyed { input, .. }
            | HydroNode::Scan { input, .. } => {
                // Skip persist before fold/reduce
                if let HydroNode::Persist { inner, .. } = input.as_ref() {
                    vec![inner.metadata()]
                } else {
                    vec![input.metadata()]
                }
            }
            HydroNode::ReduceKeyedWatermark { input, watermark, .. } => {
                // Skip persist before fold/reduce
                if let HydroNode::Persist { inner, .. } = input.as_ref() {
                    vec![inner.metadata(), watermark.metadata()]
                } else {
                    vec![input.metadata(), watermark.metadata()]
                }
            }
        }
    }

    pub fn print_root(&self) -> String {
        match self {
            HydroNode::Placeholder => {
                panic!()
            }
            HydroNode::Source { source, .. } => format!("Source({:?})", source),
            HydroNode::CycleSource { ident, .. } => format!("CycleSource({})", ident),
            HydroNode::Tee { inner, .. } => format!("Tee({})", inner.0.borrow().print_root()),
            HydroNode::Persist { .. } => "Persist()".to_string(),
            HydroNode::Unpersist { .. } => "Unpersist()".to_string(),
            HydroNode::Delta { .. } => "Delta()".to_string(),
            HydroNode::Chain { first, second, .. } => {
                format!("Chain({}, {})", first.print_root(), second.print_root())
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
            HydroNode::Enumerate { is_static, .. } => format!("Enumerate({:?})", is_static),
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
    compile_env: &D::CompileEnv,
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
                D::o2o_sink_source(compile_env, &from_node, &sink_port, &to_node, &source_port),
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
                D::o2m_sink_source(compile_env, &from_node, &sink_port, &to_node, &source_port),
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
                D::m2o_sink_source(compile_env, &from_node, &sink_port, &to_node, &source_port),
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
                D::m2m_sink_source(compile_env, &from_node, &sink_port, &to_node, &source_port),
                D::m2m_connect(&from_node, &sink_port, &to_node, &source_port),
            )
        }
        (LocationId::Tick(_, _), _) => panic!(),
        (_, LocationId::Tick(_, _)) => panic!(),
    };
    (sink, source, connect_fn)
}

#[cfg(test)]
mod test {
    use std::mem::size_of;

    use stageleft::{QuotedWithContext, q};

    use super::*;

    #[test]
    fn hydro_node_size() {
        assert_eq!(size_of::<HydroNode>(), 240);
    }

    #[test]
    fn hydro_leaf_size() {
        assert_eq!(size_of::<HydroLeaf>(), 232);
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
