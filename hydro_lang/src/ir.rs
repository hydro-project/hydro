use core::panic;
use std::cell::RefCell;
#[cfg(feature = "build")]
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::fmt::Debug;
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

#[cfg(feature = "build")]
use crate::deploy::{Deploy, RegisterPort};
use crate::location::LocationId;

#[derive(Clone, Hash)]
pub struct DebugExpr(pub syn::Expr);

impl From<syn::Expr> for DebugExpr {
    fn from(expr: syn::Expr) -> DebugExpr {
        DebugExpr(expr)
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

#[derive(Clone, Hash)]
pub struct DebugType(pub syn::Type);

impl From<syn::Type> for DebugType {
    fn from(t: syn::Type) -> DebugType {
        DebugType(t)
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
    Building(),
    Finalized(syn::Expr, syn::Expr, Option<Box<dyn FnOnce()>>),
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
            DebugInstantiate::Building() => DebugInstantiate::Building(),
            DebugInstantiate::Finalized(_, _, _) => {
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
pub enum BuildersOrCallback<
    'a,
    L: FnMut(&mut HydroLeaf, &mut usize),
    N: FnMut(&mut HydroNode, &mut usize),
> {
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
    DestSink {
        sink: DebugExpr,
        input: Box<HydroNode>,
        metadata: HydroIrMetadata,
    },
    CycleSink {
        ident: syn::Ident,
        location_kind: LocationId,
        input: Box<HydroNode>,
        metadata: HydroIrMetadata,
    },
}

impl HydroLeaf {
    #[cfg(feature = "build")]
    pub fn compile_network<'a, D: Deploy<'a>>(
        &mut self,
        compile_env: &D::CompileEnv,
        seen_tees: &mut SeenTees,
        seen_tee_locations: &mut SeenTeeLocations,
        processes: &HashMap<usize, D::Process>,
        clusters: &HashMap<usize, D::Cluster>,
        externals: &HashMap<usize, D::ExternalProcess>,
    ) {
        self.transform_children(
            |n, s| {
                n.compile_network::<D>(
                    compile_env,
                    s,
                    seen_tee_locations,
                    processes,
                    clusters,
                    externals,
                );
            },
            seen_tees,
        )
    }

    pub fn connect_network(&mut self, seen_tees: &mut SeenTees) {
        self.transform_children(
            |n, s| {
                n.connect_network(s);
            },
            seen_tees,
        )
    }

    pub fn transform_bottom_up(
        &mut self,
        transform_leaf: &mut impl FnMut(&mut HydroLeaf),
        transform_node: &mut impl FnMut(&mut HydroNode),
        seen_tees: &mut SeenTees,
    ) {
        self.transform_children(|n, s| n.transform_bottom_up(transform_node, s), seen_tees);

        transform_leaf(self);
    }

    pub fn transform_children(
        &mut self,
        mut transform: impl FnMut(&mut HydroNode, &mut SeenTees),
        seen_tees: &mut SeenTees,
    ) {
        match self {
            HydroLeaf::ForEach { f: _, input, .. }
            | HydroLeaf::DestSink { sink: _, input, .. }
            | HydroLeaf::CycleSink {
                ident: _,
                location_kind: _,
                input,
                ..
            } => {
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
                location_kind,
                input,
                metadata,
            } => HydroLeaf::CycleSink {
                ident: ident.clone(),
                location_kind: location_kind.clone(),
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
                location_kind,
                input,
                ..
            } => {
                let (input_ident, input_location_id) =
                    input.emit_core(builders_or_callback, built_tees, next_stmt_id);

                let location_id = match location_kind.root() {
                    LocationId::Process(id) => id,
                    LocationId::Cluster(id) => id,
                    LocationId::Tick(_, _) => panic!(),
                    LocationId::ExternalProcess(_) => panic!(),
                };

                assert_eq!(
                    input_location_id, *location_id,
                    "cycle_sink location mismatch"
                );

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        graph_builders.entry(*location_id).or_default().add_dfir(
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
        }
    }

    pub fn metadata_mut(&mut self) -> &mut HydroIrMetadata {
        match self {
            HydroLeaf::ForEach { metadata, .. }
            | HydroLeaf::DestSink { metadata, .. }
            | HydroLeaf::CycleSink { metadata, .. } => metadata,
        }
    }

    pub fn print_root(&self) -> String {
        match self {
            HydroLeaf::ForEach { f, .. } => format!("ForEach({:?})", f),
            HydroLeaf::DestSink { sink, .. } => format!("DestSink({:?})", sink),
            HydroLeaf::CycleSink { ident, .. } => format!("CycleSink({:?})", ident),
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
) {
    let mut seen_tees = HashMap::new();
    ir.iter_mut().for_each(|leaf| {
        leaf.transform_bottom_up(transform_leaf, transform_node, &mut seen_tees);
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

#[derive(Debug, Clone)]
pub struct HydroIrMetadata {
    pub location_kind: LocationId,
    pub output_type: Option<DebugType>,
    pub cardinality: Option<usize>,
    pub cpu_usage: Option<f64>,
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

/// An intermediate node in a Hydro graph, which consumes data
/// from upstream nodes and emits data to downstream nodes.
#[derive(Debug, Hash)]
pub enum HydroNode {
    Placeholder,

    Source {
        source: HydroSource,
        location_kind: LocationId,
        metadata: HydroIrMetadata,
    },

    CycleSource {
        ident: syn::Ident,
        location_kind: LocationId,
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

    Network {
        from_key: Option<usize>,
        to_location: LocationId,
        to_key: Option<usize>,
        serialize_fn: Option<DebugExpr>,
        instantiate_fn: DebugInstantiate,
        deserialize_fn: Option<DebugExpr>,
        input: Box<HydroNode>,
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

impl<'a> HydroNode {
    #[cfg(feature = "build")]
    pub fn compile_network<D: Deploy<'a>>(
        &mut self,
        compile_env: &D::CompileEnv,
        seen_tees: &mut SeenTees,
        seen_tee_locations: &mut SeenTeeLocations,
        nodes: &HashMap<usize, D::Process>,
        clusters: &HashMap<usize, D::Cluster>,
        externals: &HashMap<usize, D::ExternalProcess>,
    ) {
        let mut curr_location = None;

        self.transform_bottom_up(
            &mut |n| {
                if let HydroNode::Network {
                    from_key,
                    to_location,
                    to_key,
                    instantiate_fn,
                    ..
                } = n
                {
                    let (sink_expr, source_expr, connect_fn) = match instantiate_fn {
                        DebugInstantiate::Building() => instantiate_network::<D>(
                            curr_location.as_ref().unwrap(),
                            *from_key,
                            to_location,
                            *to_key,
                            nodes,
                            clusters,
                            externals,
                            compile_env,
                        ),

                        DebugInstantiate::Finalized(_, _, _) => panic!("network already finalized"),
                    };

                    *instantiate_fn =
                        DebugInstantiate::Finalized(sink_expr, source_expr, Some(connect_fn));
                }

                // Calculate location of current node to use as from_location
                match n {
                    HydroNode::Network {
                        to_location: location_kind,
                        ..
                    }
                    | HydroNode::CycleSource { location_kind, .. }
                    | HydroNode::Source { location_kind, .. } => {
                        // Unwrap location out of Tick
                        if let LocationId::Tick(_, tick_loc) = location_kind {
                            curr_location = Some(*tick_loc.clone());
                        } else {
                            curr_location = Some(location_kind.clone());
                        }
                    }
                    HydroNode::Tee { inner, .. } => {
                        let inner_ref = inner.0.as_ref() as *const RefCell<HydroNode>;
                        if let Some(tee_location) = seen_tee_locations.get(&inner_ref) {
                            curr_location = Some(tee_location.clone());
                        } else {
                            seen_tee_locations
                                .insert(inner_ref, curr_location.as_ref().unwrap().clone());
                        }
                    }
                    _ => {}
                }
            },
            seen_tees,
        );
    }

    pub fn connect_network(&mut self, seen_tees: &mut SeenTees) {
        self.transform_bottom_up(
            &mut |n| {
                if let HydroNode::Network { instantiate_fn, .. } = n {
                    match instantiate_fn {
                        DebugInstantiate::Building() => panic!("network not built"),

                        DebugInstantiate::Finalized(_, _, connect_fn) => {
                            connect_fn.take().unwrap()();
                        }
                    }
                }
            },
            seen_tees,
        );
    }

    pub fn transform_bottom_up(
        &mut self,
        transform: &mut impl FnMut(&mut HydroNode),
        seen_tees: &mut SeenTees,
    ) {
        self.transform_children(|n, s| n.transform_bottom_up(transform, s), seen_tees);

        transform(self);
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

            HydroNode::Source { .. } | HydroNode::CycleSource { .. } => {}

            HydroNode::Tee { inner, .. } => {
                if let Some(transformed) =
                    seen_tees.get(&(inner.0.as_ref() as *const RefCell<HydroNode>))
                {
                    *inner = TeeNode(transformed.clone());
                } else {
                    let transformed_cell = Rc::new(RefCell::new(HydroNode::Placeholder));
                    seen_tees.insert(
                        inner.0.as_ref() as *const RefCell<HydroNode>,
                        transformed_cell.clone(),
                    );
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
            | HydroNode::Fold { input, .. }
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
            HydroNode::Source {
                source,
                location_kind,
                metadata,
            } => HydroNode::Source {
                source: source.clone(),
                location_kind: location_kind.clone(),
                metadata: metadata.clone(),
            },
            HydroNode::CycleSource {
                ident,
                location_kind,
                metadata,
            } => HydroNode::CycleSource {
                ident: ident.clone(),
                location_kind: location_kind.clone(),
                metadata: metadata.clone(),
            },
            HydroNode::Tee { inner, metadata } => {
                if let Some(transformed) =
                    seen_tees.get(&(inner.0.as_ref() as *const RefCell<HydroNode>))
                {
                    HydroNode::Tee {
                        inner: TeeNode(transformed.clone()),
                        metadata: metadata.clone(),
                    }
                } else {
                    let new_rc = Rc::new(RefCell::new(HydroNode::Placeholder));
                    seen_tees.insert(
                        inner.0.as_ref() as *const RefCell<HydroNode>,
                        new_rc.clone(),
                    );
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
                from_key,
                to_location,
                to_key,
                serialize_fn,
                instantiate_fn,
                deserialize_fn,
                input,
                metadata,
            } => HydroNode::Network {
                from_key: *from_key,
                to_location: to_location.clone(),
                to_key: *to_key,
                serialize_fn: serialize_fn.clone(),
                instantiate_fn: instantiate_fn.clone(),
                deserialize_fn: deserialize_fn.clone(),
                input: Box::new(input.deep_clone(seen_tees)),
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
                source,
                location_kind,
                ..
            } => {
                let location_id = match location_kind.clone() {
                    LocationId::Process(id) => id,
                    LocationId::Cluster(id) => id,
                    LocationId::Tick(_, _) => panic!(),
                    LocationId::ExternalProcess(id) => id,
                };

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
                ident,
                location_kind,
                ..
            } => {
                let location_id = *match location_kind.root() {
                    LocationId::Process(id) => id,
                    LocationId::Cluster(id) => id,
                    LocationId::Tick(_, _) => panic!(),
                    LocationId::ExternalProcess(_) => panic!(),
                };

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

                assert_eq!(
                    first_location_id, second_location_id,
                    "chain inputs must be in the same location"
                );

                let chain_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
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

                assert_eq!(
                    left_location_id, right_location_id,
                    "cross_singleton inputs must be in the same location"
                );

                let cross_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
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

                assert_eq!(
                    left_location_id, right_location_id,
                    "join / cross product inputs must be in the same location"
                );

                let stream_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
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

                assert_eq!(
                    pos_location_id, neg_location_id,
                    "difference / anti join inputs must be in the same location"
                );

                let stream_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
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

            HydroNode::Fold { .. } | HydroNode::FoldKeyed { .. } => {
                let operator: syn::Ident = if matches!(self, HydroNode::Fold { .. }) {
                    parse_quote!(fold)
                } else {
                    parse_quote!(fold_keyed)
                };

                let (HydroNode::Fold {
                    init, acc, input, ..
                }
                | HydroNode::FoldKeyed {
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
                from_key: _,
                to_location,
                to_key: _,
                serialize_fn: serialize_pipeline,
                instantiate_fn,
                deserialize_fn: deserialize_pipeline,
                input,
                ..
            } => {
                let (input_ident, input_location_id) =
                    input.emit_core(builders_or_callback, built_tees, next_stmt_id);

                let to_id = match *to_location {
                    LocationId::Process(id) => id,
                    LocationId::Cluster(id) => id,
                    LocationId::Tick(_, _) => panic!(),
                    LocationId::ExternalProcess(id) => id,
                };

                let receiver_stream_ident =
                    syn::Ident::new(&format!("stream_{}", *next_stmt_id), Span::call_site());

                match builders_or_callback {
                    BuildersOrCallback::Builders(graph_builders) => {
                        let (sink_expr, source_expr) = match instantiate_fn {
                            DebugInstantiate::Building() => (
                                syn::parse_quote!(DUMMY_SINK),
                                syn::parse_quote!(DUMMY_SOURCE),
                            ),

                            DebugInstantiate::Finalized(sink, source, _connect_fn) => {
                                (sink.clone(), source.clone())
                            }
                        };

                        let sender_builder = graph_builders.entry(input_location_id).or_default();
                        if let Some(serialize_pipeline) = serialize_pipeline {
                            sender_builder.add_dfir(
                                parse_quote! {
                                    #input_ident -> map(#serialize_pipeline) -> dest_sink(#sink_expr);
                                },
                                None,
                                Some(&next_stmt_id.to_string()),
                            );
                        } else {
                            sender_builder.add_dfir(
                                parse_quote! {
                                    #input_ident -> dest_sink(#sink_expr);
                                },
                                None,
                                Some(&next_stmt_id.to_string()),
                            );
                        }

                        let receiver_builder = graph_builders.entry(to_id).or_default();
                        if let Some(deserialize_pipeline) = deserialize_pipeline {
                            receiver_builder.add_dfir(parse_quote! {
                                #receiver_stream_ident = source_stream(#source_expr) -> map(#deserialize_pipeline);
                            }, None, Some(&next_stmt_id.to_string()));
                        } else {
                            receiver_builder.add_dfir(
                                parse_quote! {
                                    #receiver_stream_ident = source_stream(#source_expr);
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
            | HydroNode::ReduceKeyed { f, .. } => {
                transform(f);
            }
            HydroNode::Fold { init, acc, .. } | HydroNode::FoldKeyed { init, acc, .. } => {
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
            HydroNode::Map { metadata, .. } => metadata,
            HydroNode::FlatMap { metadata, .. } => metadata,
            HydroNode::Filter { metadata, .. } => metadata,
            HydroNode::FilterMap { metadata, .. } => metadata,
            HydroNode::DeferTick { metadata, .. } => metadata,
            HydroNode::Enumerate { metadata, .. } => metadata,
            HydroNode::Inspect { metadata, .. } => metadata,
            HydroNode::Unique { metadata, .. } => metadata,
            HydroNode::Sort { metadata, .. } => metadata,
            HydroNode::Fold { metadata, .. } => metadata,
            HydroNode::FoldKeyed { metadata, .. } => metadata,
            HydroNode::Reduce { metadata, .. } => metadata,
            HydroNode::ReduceKeyed { metadata, .. } => metadata,
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
            HydroNode::Map { metadata, .. } => metadata,
            HydroNode::FlatMap { metadata, .. } => metadata,
            HydroNode::Filter { metadata, .. } => metadata,
            HydroNode::FilterMap { metadata, .. } => metadata,
            HydroNode::DeferTick { metadata, .. } => metadata,
            HydroNode::Enumerate { metadata, .. } => metadata,
            HydroNode::Inspect { metadata, .. } => metadata,
            HydroNode::Unique { metadata, .. } => metadata,
            HydroNode::Sort { metadata, .. } => metadata,
            HydroNode::Fold { metadata, .. } => metadata,
            HydroNode::FoldKeyed { metadata, .. } => metadata,
            HydroNode::Reduce { metadata, .. } => metadata,
            HydroNode::ReduceKeyed { metadata, .. } => metadata,
            HydroNode::Network { metadata, .. } => metadata,
            HydroNode::Counter { metadata, .. } => metadata,
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
            HydroNode::FoldKeyed { init, acc, .. } => format!("FoldKeyed({:?}, {:?})", init, acc),
            HydroNode::Reduce { f, .. } => format!("Reduce({:?})", f),
            HydroNode::ReduceKeyed { f, .. } => format!("ReduceKeyed({:?})", f),
            HydroNode::Network { to_location, .. } => format!("Network(to {:?})", to_location),
            HydroNode::Counter { tag, duration, .. } => {
                format!("Counter({:?}, {:?})", tag, duration)
            }
        }
    }
}

#[cfg(feature = "build")]
#[expect(clippy::too_many_arguments, reason = "networking internals")]
fn instantiate_network<'a, D: Deploy<'a>>(
    from_location: &LocationId,
    from_key: Option<usize>,
    to_location: &LocationId,
    to_key: Option<usize>,
    nodes: &HashMap<usize, D::Process>,
    clusters: &HashMap<usize, D::Cluster>,
    externals: &HashMap<usize, D::ExternalProcess>,
    compile_env: &D::CompileEnv,
) -> (syn::Expr, syn::Expr, Box<dyn FnOnce()>) {
    let ((sink, source), connect_fn) = match (from_location, to_location) {
        (LocationId::Process(from), LocationId::Process(to)) => {
            let from_node = nodes
                .get(from)
                .unwrap_or_else(|| {
                    panic!("A process used in the graph was not instantiated: {}", from)
                })
                .clone();
            let to_node = nodes
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
            let from_node = nodes
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
            let to_node = nodes
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
        (LocationId::ExternalProcess(from), LocationId::Process(to)) => {
            let from_node = externals
                .get(from)
                .unwrap_or_else(|| {
                    panic!(
                        "A external used in the graph was not instantiated: {}",
                        from
                    )
                })
                .clone();

            let to_node = nodes
                .get(to)
                .unwrap_or_else(|| {
                    panic!("A process used in the graph was not instantiated: {}", to)
                })
                .clone();

            let sink_port = D::allocate_external_port(&from_node);
            let source_port = D::allocate_process_port(&to_node);

            from_node.register(from_key.unwrap(), sink_port.clone());

            (
                (
                    parse_quote!(DUMMY),
                    D::e2o_source(compile_env, &from_node, &sink_port, &to_node, &source_port),
                ),
                D::e2o_connect(&from_node, &sink_port, &to_node, &source_port),
            )
        }
        (LocationId::ExternalProcess(_from), LocationId::Cluster(_to)) => {
            todo!("NYI")
        }
        (LocationId::ExternalProcess(_), LocationId::ExternalProcess(_)) => {
            panic!("Cannot send from external to external")
        }
        (LocationId::Process(from), LocationId::ExternalProcess(to)) => {
            let from_node = nodes
                .get(from)
                .unwrap_or_else(|| {
                    panic!("A process used in the graph was not instantiated: {}", from)
                })
                .clone();

            let to_node = externals
                .get(to)
                .unwrap_or_else(|| {
                    panic!("A external used in the graph was not instantiated: {}", to)
                })
                .clone();

            let sink_port = D::allocate_process_port(&from_node);
            let source_port = D::allocate_external_port(&to_node);

            to_node.register(to_key.unwrap(), source_port.clone());

            (
                (
                    D::o2e_sink(compile_env, &from_node, &sink_port, &to_node, &source_port),
                    parse_quote!(DUMMY),
                ),
                D::o2e_connect(&from_node, &sink_port, &to_node, &source_port),
            )
        }
        (LocationId::Cluster(_from), LocationId::ExternalProcess(_to)) => {
            todo!("NYI")
        }
        (LocationId::Tick(_, _), _) => panic!(),
        (_, LocationId::Tick(_, _)) => panic!(),
    };
    (sink, source, connect_fn)
}
