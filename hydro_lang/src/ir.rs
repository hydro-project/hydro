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
#[cfg(feature = "build")]
use quote::quote;
use quote::ToTokens;
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

/// A source in a Hydro graph, where data enters the graph.
#[derive(Debug, Hash)]
pub enum HydroSource {
    Stream(DebugExpr),
    ExternalNetwork(),
    Iter(DebugExpr),
    Spin(),
}

/// An leaf in a Hydro graph, which is an pipeline that doesn't emit
/// any downstream values. Traversals over the dataflow graph and
/// generating Hydroflow IR start from leaves.
#[derive(Debug, Hash)]
pub enum HydroLeaf {
    ForEach {
        f: DebugExpr,
        input: Box<HydroNode>,
    },
    DestSink {
        sink: DebugExpr,
        input: Box<HydroNode>,
    },
    CycleSink {
        ident: syn::Ident,
        location_kind: LocationId,
        input: Box<HydroNode>,
    },
}

impl HydroLeaf {
    #[cfg(feature = "build")]
    pub fn compile_network<'a, D: Deploy<'a>>(
        &mut self,
        compile_env: &D::CompileEnv,
        seen_tees: &mut SeenTees,
        processes: &HashMap<usize, D::Process>,
        clusters: &HashMap<usize, D::Cluster>,
        externals: &HashMap<usize, D::ExternalProcess>,
    ) {
        self.transform_children(
            |n, s| {
                n.compile_network::<D>(compile_env, s, processes, clusters, externals);
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

    pub fn transform_bottom_up<C>(
        &mut self,
        mut transform_leaf: impl FnMut(&mut HydroLeaf, &mut C, &mut usize) + Copy,
        transform_node: impl FnMut(&mut HydroNode, &mut C, &mut usize) + Copy,
        seen_tees: &mut SeenTees,
        ctx: &mut C,
        next_stmt_id: &mut usize
    ) {
        self.transform_children(|n, s| n.transform_bottom_up(transform_node, s, ctx, next_stmt_id), seen_tees);

        transform_leaf(self, ctx, next_stmt_id);

        // Don't increment next_stmt_id in special cases
        match self {
            HydroLeaf::CycleSink { .. } => {}
            _ => {
                *next_stmt_id += 1;
            }
        }
    }

    pub fn transform_children(
        &mut self,
        mut transform: impl FnMut(&mut HydroNode, &mut SeenTees),
        seen_tees: &mut SeenTees,
    ) {
        match self {
            HydroLeaf::ForEach { f: _, ref mut input }
            | HydroLeaf::DestSink { sink: _, ref mut input }
            | HydroLeaf::CycleSink {
                ident: _,
                location_kind: _,
                ref mut input,
            } => {
                transform(input, seen_tees);
            }
        }
    }

    #[cfg(feature = "build")]
    pub fn emit(
        &self,
        graph_builders: &mut BTreeMap<usize, FlatGraphBuilder>,
        built_tees: &mut HashMap<*const RefCell<HydroNode>, (syn::Ident, usize)>,
        next_stmt_id: &mut usize,
    ) {
        match self {
            HydroLeaf::ForEach { f, input } => {
                let (input_ident, input_location_id) =
                    input.emit(graph_builders, built_tees, next_stmt_id);

                let stream_id = *next_stmt_id;
                *next_stmt_id += 1;

                graph_builders
                    .entry(input_location_id)
                    .or_default()
                    .add_dfir(
                        parse_quote! {
                            #input_ident -> for_each(#f);
                        },
                        None,
                        Some(&stream_id.to_string()),
                    );
            }

            HydroLeaf::DestSink { sink, input } => {
                let (input_ident, input_location_id) =
                    input.emit(graph_builders, built_tees, next_stmt_id);

                let stream_id = *next_stmt_id;
                *next_stmt_id += 1;

                graph_builders
                    .entry(input_location_id)
                    .or_default()
                    .add_dfir(
                        parse_quote! {
                            #input_ident -> dest_sink(#sink);
                        },
                        None,
                        Some(&stream_id.to_string()),
                    );
            }

            HydroLeaf::CycleSink {
                ident,
                location_kind,
                input,
            } => {
                let (input_ident, input_location_id) =
                    input.emit(graph_builders, built_tees, next_stmt_id);

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

                graph_builders.entry(*location_id).or_default().add_dfir(
                    parse_quote! {
                        #ident = #input_ident;
                    },
                    None,
                    None,
                );
            }
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

#[derive(Debug, Clone, Hash)]
pub struct HydroNodeMetadata {
    pub location_kind: LocationId,
    pub output_type: Option<DebugType>,
}

/// An intermediate node in a Hydro graph, which consumes data
/// from upstream nodes and emits data to downstream nodes.
#[derive(Debug, Hash)]
pub enum HydroNode {
    Placeholder,

    Source {
        source: HydroSource,
        location_kind: LocationId,
        metadata: HydroNodeMetadata,
    },

    CycleSource {
        ident: syn::Ident,
        location_kind: LocationId,
        metadata: HydroNodeMetadata,
    },

    Tee {
        inner: TeeNode,
        metadata: HydroNodeMetadata,
    },

    Persist {
        inner: Box<HydroNode>,
        metadata: HydroNodeMetadata,
    },

    Unpersist {
        inner: Box<HydroNode>,
        metadata: HydroNodeMetadata,
    },

    Delta {
        inner: Box<HydroNode>,
        metadata: HydroNodeMetadata,
    },

    Chain {
        first: Box<HydroNode>,
        second: Box<HydroNode>,
        metadata: HydroNodeMetadata,
    },

    CrossProduct {
        left: Box<HydroNode>,
        right: Box<HydroNode>,
        metadata: HydroNodeMetadata,
    },

    CrossSingleton {
        left: Box<HydroNode>,
        right: Box<HydroNode>,
        metadata: HydroNodeMetadata,
    },

    Join {
        left: Box<HydroNode>,
        right: Box<HydroNode>,
        metadata: HydroNodeMetadata,
    },

    Difference {
        pos: Box<HydroNode>,
        neg: Box<HydroNode>,
        metadata: HydroNodeMetadata,
    },

    AntiJoin {
        pos: Box<HydroNode>,
        neg: Box<HydroNode>,
        metadata: HydroNodeMetadata,
    },

    Map {
        f: DebugExpr,
        input: Box<HydroNode>,
        metadata: HydroNodeMetadata,
    },
    FlatMap {
        f: DebugExpr,
        input: Box<HydroNode>,
        metadata: HydroNodeMetadata,
    },
    Filter {
        f: DebugExpr,
        input: Box<HydroNode>,
        metadata: HydroNodeMetadata,
    },
    FilterMap {
        f: DebugExpr,
        input: Box<HydroNode>,
        metadata: HydroNodeMetadata,
    },

    DeferTick {
        input: Box<HydroNode>,
        metadata: HydroNodeMetadata,
    },
    Enumerate {
        is_static: bool,
        input: Box<HydroNode>,
        metadata: HydroNodeMetadata,
    },
    Inspect {
        f: DebugExpr,
        input: Box<HydroNode>,
        metadata: HydroNodeMetadata,
    },

    Unique {
        input: Box<HydroNode>,
        metadata: HydroNodeMetadata,
    },

    Sort {
        input: Box<HydroNode>,
        metadata: HydroNodeMetadata,
    },
    Fold {
        init: DebugExpr,
        acc: DebugExpr,
        input: Box<HydroNode>,
        metadata: HydroNodeMetadata,
    },
    FoldKeyed {
        init: DebugExpr,
        acc: DebugExpr,
        input: Box<HydroNode>,
        metadata: HydroNodeMetadata,
    },

    Reduce {
        f: DebugExpr,
        input: Box<HydroNode>,
        metadata: HydroNodeMetadata,
    },
    ReduceKeyed {
        f: DebugExpr,
        input: Box<HydroNode>,
        metadata: HydroNodeMetadata,
    },

    Network {
        from_location: LocationId,
        from_key: Option<usize>,
        to_location: LocationId,
        to_key: Option<usize>,
        serialize_fn: Option<DebugExpr>,
        instantiate_fn: DebugInstantiate,
        deserialize_fn: Option<DebugExpr>,
        input: Box<HydroNode>,
        metadata: HydroNodeMetadata,
    },
}

pub type SeenTees = HashMap<*const RefCell<HydroNode>, Rc<RefCell<HydroNode>>>;

impl<'a> HydroNode {
    #[cfg(feature = "build")]
    pub fn compile_network<D: Deploy<'a>>(
        &mut self,
        compile_env: &D::CompileEnv,
        seen_tees: &mut SeenTees,
        nodes: &HashMap<usize, D::Process>,
        clusters: &HashMap<usize, D::Cluster>,
        externals: &HashMap<usize, D::ExternalProcess>,
    ) {
        let mut next_stmt_id = 0;
        self.transform_bottom_up(|n, _, _| {
            if let HydroNode::Network {
                from_location,
                from_key,
                to_location,
                to_key,
                instantiate_fn,
                ..
            } = n
            {
                let (sink_expr, source_expr, connect_fn) = match instantiate_fn {
                    DebugInstantiate::Building() => instantiate_network::<D>(
                        from_location,
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

                *instantiate_fn = DebugInstantiate::Finalized(sink_expr, source_expr, Some(connect_fn));
            }
        }, seen_tees, &mut (), &mut next_stmt_id);
    }

    pub fn connect_network(&mut self, seen_tees: &mut SeenTees) {
        let mut next_stmt_id = 0;
        self.transform_bottom_up(|n, _, _| {
            if let HydroNode::Network { instantiate_fn, .. } = n {
                match instantiate_fn {
                    DebugInstantiate::Building() => panic!("network not built"),

                    DebugInstantiate::Finalized(_, _, connect_fn) => {
                        connect_fn.take().unwrap()();
                    }
                }
            }
        }, seen_tees, &mut (), &mut next_stmt_id);
    }

    pub fn transform_bottom_up<C>(
        &mut self,
        mut transform: impl FnMut(&mut HydroNode, &mut C, &mut usize) + Copy,
        seen_tees: &mut SeenTees,
        ctx: &mut C,
        next_stmt_id: &mut usize
    ) {
        self.transform_children(|n, s| n.transform_bottom_up(transform, s, ctx, next_stmt_id), seen_tees);

        transform(self, ctx, next_stmt_id);

        // Don't increment next_stmt_id in special cases
        match self {
            HydroNode::Placeholder | HydroNode::CycleSource { .. } => {}
            _ => {
                *next_stmt_id += 1;
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

            HydroNode::Persist { inner, .. } | HydroNode::Unpersist { inner, .. } | HydroNode::Delta { inner, .. } => {
                transform(inner.as_mut(), seen_tees);
            },

            HydroNode::Chain { first, second, .. } => {
                transform(first.as_mut(), seen_tees);
                transform(second.as_mut(), seen_tees);
            }
            HydroNode::CrossSingleton { left, right, .. } => {
                transform(left.as_mut(), seen_tees);
                transform(right.as_mut(), seen_tees);
            }
            HydroNode::CrossProduct { .. } | HydroNode::Join { .. } => {
                let (HydroNode::CrossProduct { left, right, .. }
                | HydroNode::Join { left, right, .. }) = self
                else {
                    unreachable!()
                };

                let left_inner =
                    if let HydroNode::Persist { inner: left, .. } = left.as_mut() {
                        left
                    } else {
                        left
                    };

                let right_inner =
                    if let HydroNode::Persist { inner: right, .. } = right.as_mut() {
                        right
                    } else {
                        right
                    };

                transform(left_inner.as_mut(), seen_tees);
                transform(right_inner.as_mut(), seen_tees);
            }
            HydroNode::Difference { .. } | HydroNode::AntiJoin { .. } => {
                let (HydroNode::Difference { pos, neg, .. } | HydroNode::AntiJoin { pos, neg, .. }) =
                    self
                else {
                    unreachable!()
                };

                let neg =
                    if let HydroNode::Persist { inner: neg, .. } = neg.as_mut() {
                        neg
                    } else {
                        neg
                    };

                transform(pos.as_mut(), seen_tees);
                transform(neg.as_mut(), seen_tees);
            }

            HydroNode::Map { input, .. } | HydroNode::FlatMap { input, .. } 
            | HydroNode::Filter { input, .. } | HydroNode::FilterMap { input, .. }
            | HydroNode::Sort { input, .. } | HydroNode::DeferTick { input, .. }
            | HydroNode::Enumerate { input, .. } | HydroNode::Inspect { input, .. }
            | HydroNode::Unique { input, .. } |  HydroNode::Network { input, .. } => {
                transform(input.as_mut(), seen_tees);
            }

            HydroNode::Fold { .. } | HydroNode::FoldKeyed { .. } | HydroNode::Reduce { .. } | HydroNode::ReduceKeyed { .. } => {
                let (HydroNode::Fold {
                    init: _, acc: _, input, ..
                }
                | HydroNode::FoldKeyed {
                    init: _, acc: _, input, ..
                } | HydroNode::Reduce { f: _, input, .. } | HydroNode::ReduceKeyed { f: _, input, .. }) = self
                else {
                    unreachable!()
                };

                let input =
                    if let HydroNode::Persist { inner: input, .. } = input.as_mut() {
                        input
                    } else {
                        input
                    };
                    
                transform(input.as_mut(), seen_tees);
            }
        }
    }

    #[cfg(feature = "build")]
    pub fn emit(
        &self,
        graph_builders: &mut BTreeMap<usize, FlatGraphBuilder>,
        built_tees: &mut HashMap<*const RefCell<HydroNode>, (syn::Ident, usize)>,
        next_stmt_id: &mut usize,
    ) -> (syn::Ident, usize) {
        match self {
            HydroNode::Placeholder => {
                panic!()
            }

            HydroNode::Persist { inner, .. } => {
                let (inner_ident, location) = inner.emit(graph_builders, built_tees, next_stmt_id);

                let persist_id = *next_stmt_id;
                *next_stmt_id += 1;

                let persist_ident =
                    syn::Ident::new(&format!("stream_{}", persist_id), Span::call_site());

                let builder = graph_builders.entry(location).or_default();
                builder.add_dfir(
                    parse_quote! {
                        #persist_ident = #inner_ident -> persist::<'static>();
                    },
                    None,
                    Some(&persist_id.to_string()),
                );

                (persist_ident, location)
            }

            HydroNode::Unpersist { .. } => {
                panic!("Unpersist is a marker node and should have been optimized away. This is likely a compiler bug.")
            }

            HydroNode::Delta { inner, .. } => {
                let (inner_ident, location) = inner.emit(graph_builders, built_tees, next_stmt_id);

                let delta_id = *next_stmt_id;
                *next_stmt_id += 1;

                let delta_ident =
                    syn::Ident::new(&format!("stream_{}", delta_id), Span::call_site());

                let builder = graph_builders.entry(location).or_default();
                builder.add_dfir(
                    parse_quote! {
                        #delta_ident = #inner_ident -> multiset_delta();
                    },
                    None,
                    Some(&delta_id.to_string()),
                );

                (delta_ident, location)
            }

            HydroNode::Source {
                source,
                location_kind,
                ..
            } => {
                let location_id = match location_kind {
                    LocationId::Process(id) => id,
                    LocationId::Cluster(id) => id,
                    LocationId::Tick(_, _) => panic!(),
                    LocationId::ExternalProcess(id) => id,
                };

                if let HydroSource::ExternalNetwork() = source {
                    (syn::Ident::new("DUMMY", Span::call_site()), *location_id)
                } else {
                    let source_id = *next_stmt_id;
                    *next_stmt_id += 1;

                    let source_ident =
                        syn::Ident::new(&format!("stream_{}", source_id), Span::call_site());

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

                    graph_builders.entry(*location_id).or_default().add_dfir(
                        source_stmt,
                        None,
                        Some(&source_id.to_string()),
                    );

                    (source_ident, *location_id)
                }
            }

            HydroNode::CycleSource {
                ident,
                location_kind,
                ..
            } => {
                let location_id = match location_kind.root() {
                    LocationId::Process(id) => id,
                    LocationId::Cluster(id) => id,
                    LocationId::Tick(_, _) => panic!(),
                    LocationId::ExternalProcess(_) => panic!(),
                };

                (ident.clone(), *location_id)
            }

            HydroNode::Tee { inner, .. } => {
                if let Some(ret) = built_tees.get(&(inner.0.as_ref() as *const RefCell<HydroNode>))
                {
                    ret.clone()
                } else {
                    let (inner_ident, inner_location_id) =
                        inner
                            .0
                            .borrow()
                            .emit(graph_builders, built_tees, next_stmt_id);

                    let tee_id = *next_stmt_id;
                    *next_stmt_id += 1;

                    let tee_ident =
                        syn::Ident::new(&format!("stream_{}", tee_id), Span::call_site());

                    let builder = graph_builders.entry(inner_location_id).or_default();
                    builder.add_dfir(
                        parse_quote! {
                            #tee_ident = #inner_ident -> tee();
                        },
                        None,
                        Some(&tee_id.to_string()),
                    );

                    built_tees.insert(
                        inner.0.as_ref() as *const RefCell<HydroNode>,
                        (tee_ident.clone(), inner_location_id),
                    );

                    (tee_ident, inner_location_id)
                }
            }

            HydroNode::Chain { first, second, .. } => {
                let (first_ident, first_location_id) =
                    first.emit(graph_builders, built_tees, next_stmt_id);
                let (second_ident, second_location_id) =
                    second.emit(graph_builders, built_tees, next_stmt_id);

                assert_eq!(
                    first_location_id, second_location_id,
                    "chain inputs must be in the same location"
                );

                let chain_id = *next_stmt_id;
                *next_stmt_id += 1;

                let chain_ident =
                    syn::Ident::new(&format!("stream_{}", chain_id), Span::call_site());

                let builder = graph_builders.entry(first_location_id).or_default();
                builder.add_dfir(
                    parse_quote! {
                        #chain_ident = chain();
                        #first_ident -> [0]#chain_ident;
                        #second_ident -> [1]#chain_ident;
                    },
                    None,
                    Some(&chain_id.to_string()),
                );

                (chain_ident, first_location_id)
            }

            HydroNode::CrossSingleton { left, right, .. } => {
                let (left_ident, left_location_id) =
                    left.emit(graph_builders, built_tees, next_stmt_id);
                let (right_ident, right_location_id) =
                    right.emit(graph_builders, built_tees, next_stmt_id);

                assert_eq!(
                    left_location_id, right_location_id,
                    "cross_singleton inputs must be in the same location"
                );

                let cross_id = *next_stmt_id;
                *next_stmt_id += 1;

                let cross_ident =
                    syn::Ident::new(&format!("stream_{}", cross_id), Span::call_site());

                let builder = graph_builders.entry(left_location_id).or_default();
                builder.add_dfir(
                    parse_quote! {
                        #cross_ident = cross_singleton();
                        #left_ident -> [input]#cross_ident;
                        #right_ident -> [single]#cross_ident;
                    },
                    None,
                    Some(&cross_id.to_string()),
                );

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
                    if let HydroNode::Persist { inner: left, .. } = left.as_ref() {
                        (left, quote!('static))
                    } else {
                        (left, quote!('tick))
                    };

                let (right_inner, right_lifetime) =
                    if let HydroNode::Persist { inner: right, .. } = right.as_ref() {
                        (right, quote!('static))
                    } else {
                        (right, quote!('tick))
                    };

                let (left_ident, left_location_id) =
                    left_inner.emit(graph_builders, built_tees, next_stmt_id);
                let (right_ident, right_location_id) =
                    right_inner.emit(graph_builders, built_tees, next_stmt_id);

                assert_eq!(
                    left_location_id, right_location_id,
                    "join / cross product inputs must be in the same location"
                );

                let stream_id = *next_stmt_id;
                *next_stmt_id += 1;

                let stream_ident =
                    syn::Ident::new(&format!("stream_{}", stream_id), Span::call_site());

                let builder = graph_builders.entry(left_location_id).or_default();

                builder.add_dfir(
                    parse_quote! {
                        #stream_ident = #operator::<#left_lifetime, #right_lifetime>();
                        #left_ident -> [0]#stream_ident;
                        #right_ident -> [1]#stream_ident;
                    },
                    None,
                    Some(&stream_id.to_string()),
                );

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
                    if let HydroNode::Persist { inner: neg, .. } = neg.as_ref() {
                        (neg, quote!('static))
                    } else {
                        (neg, quote!('tick))
                    };

                let (pos_ident, pos_location_id) =
                    pos.emit(graph_builders, built_tees, next_stmt_id);
                let (neg_ident, neg_location_id) =
                    neg.emit(graph_builders, built_tees, next_stmt_id);

                assert_eq!(
                    pos_location_id, neg_location_id,
                    "difference / anti join inputs must be in the same location"
                );

                let stream_id = *next_stmt_id;
                *next_stmt_id += 1;

                let stream_ident =
                    syn::Ident::new(&format!("stream_{}", stream_id), Span::call_site());

                let builder = graph_builders.entry(pos_location_id).or_default();

                builder.add_dfir(
                    parse_quote! {
                        #stream_ident = #operator::<'tick, #neg_lifetime>();
                        #pos_ident -> [pos]#stream_ident;
                        #neg_ident -> [neg]#stream_ident;
                    },
                    None,
                    Some(&stream_id.to_string()),
                );

                (stream_ident, pos_location_id)
            }

            HydroNode::Map { f, input, .. } => {
                let (input_ident, input_location_id) =
                    input.emit(graph_builders, built_tees, next_stmt_id);

                let map_id = *next_stmt_id;
                *next_stmt_id += 1;

                let map_ident = syn::Ident::new(&format!("stream_{}", map_id), Span::call_site());

                let builder = graph_builders.entry(input_location_id).or_default();
                builder.add_dfir(
                    parse_quote! {
                        #map_ident = #input_ident -> map(#f);
                    },
                    None,
                    Some(&map_id.to_string()),
                );

                (map_ident, input_location_id)
            }

            HydroNode::FlatMap { f, input, .. } => {
                let (input_ident, input_location_id) =
                    input.emit(graph_builders, built_tees, next_stmt_id);

                let flat_map_id = *next_stmt_id;
                *next_stmt_id += 1;

                let flat_map_ident =
                    syn::Ident::new(&format!("stream_{}", flat_map_id), Span::call_site());

                let builder = graph_builders.entry(input_location_id).or_default();
                builder.add_dfir(
                    parse_quote! {
                        #flat_map_ident = #input_ident -> flat_map(#f);
                    },
                    None,
                    Some(&flat_map_id.to_string()),
                );

                (flat_map_ident, input_location_id)
            }

            HydroNode::Filter { f, input, .. } => {
                let (input_ident, input_location_id) =
                    input.emit(graph_builders, built_tees, next_stmt_id);

                let filter_id = *next_stmt_id;
                *next_stmt_id += 1;

                let filter_ident =
                    syn::Ident::new(&format!("stream_{}", filter_id), Span::call_site());

                let builder = graph_builders.entry(input_location_id).or_default();
                builder.add_dfir(
                    parse_quote! {
                        #filter_ident = #input_ident -> filter(#f);
                    },
                    None,
                    Some(&filter_id.to_string()),
                );

                (filter_ident, input_location_id)
            }

            HydroNode::FilterMap { f, input, .. } => {
                let (input_ident, input_location_id) =
                    input.emit(graph_builders, built_tees, next_stmt_id);

                let filter_map_id = *next_stmt_id;
                *next_stmt_id += 1;

                let filter_map_ident =
                    syn::Ident::new(&format!("stream_{}", filter_map_id), Span::call_site());

                let builder = graph_builders.entry(input_location_id).or_default();
                builder.add_dfir(
                    parse_quote! {
                        #filter_map_ident = #input_ident -> filter_map(#f);
                    },
                    None,
                    Some(&filter_map_id.to_string()),
                );

                (filter_map_ident, input_location_id)
            }

            HydroNode::Sort { input, .. } => {
                let (input_ident, input_location_id) =
                    input.emit(graph_builders, built_tees, next_stmt_id);

                let sort_id = *next_stmt_id;
                *next_stmt_id += 1;

                let sort_ident = syn::Ident::new(&format!("stream_{}", sort_id), Span::call_site());

                let builder = graph_builders.entry(input_location_id).or_default();
                builder.add_dfir(
                    parse_quote! {
                        #sort_ident = #input_ident -> sort();
                    },
                    None,
                    Some(&sort_id.to_string()),
                );

                (sort_ident, input_location_id)
            }

            HydroNode::DeferTick { input, .. } => {
                let (input_ident, input_location_id) =
                    input.emit(graph_builders, built_tees, next_stmt_id);

                let defer_tick_id = *next_stmt_id;
                *next_stmt_id += 1;

                let defer_tick_ident =
                    syn::Ident::new(&format!("stream_{}", defer_tick_id), Span::call_site());

                let builder = graph_builders.entry(input_location_id).or_default();
                builder.add_dfir(
                    parse_quote! {
                        #defer_tick_ident = #input_ident -> defer_tick_lazy();
                    },
                    None,
                    Some(&defer_tick_id.to_string()),
                );

                (defer_tick_ident, input_location_id)
            }

            HydroNode::Enumerate {
                is_static, input, ..
            } => {
                let (input_ident, input_location_id) =
                    input.emit(graph_builders, built_tees, next_stmt_id);

                let enumerate_id = *next_stmt_id;
                *next_stmt_id += 1;

                let enumerate_ident =
                    syn::Ident::new(&format!("stream_{}", enumerate_id), Span::call_site());

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
                    Some(&enumerate_id.to_string()),
                );

                (enumerate_ident, input_location_id)
            }

            HydroNode::Inspect { f, input, .. } => {
                let (input_ident, input_location_id) =
                    input.emit(graph_builders, built_tees, next_stmt_id);

                let inspect_id = *next_stmt_id;
                *next_stmt_id += 1;

                let inspect_ident =
                    syn::Ident::new(&format!("stream_{}", inspect_id), Span::call_site());

                let builder = graph_builders.entry(input_location_id).or_default();
                builder.add_dfir(
                    parse_quote! {
                        #inspect_ident = #input_ident -> inspect(#f);
                    },
                    None,
                    Some(&inspect_id.to_string()),
                );

                (inspect_ident, input_location_id)
            }

            HydroNode::Unique { input, .. } => {
                let (input_ident, input_location_id) =
                    input.emit(graph_builders, built_tees, next_stmt_id);

                let unique_id = *next_stmt_id;
                *next_stmt_id += 1;

                let unique_ident =
                    syn::Ident::new(&format!("stream_{}", unique_id), Span::call_site());

                let builder = graph_builders.entry(input_location_id).or_default();
                builder.add_dfir(
                    parse_quote! {
                        #unique_ident = #input_ident -> unique::<'tick>();
                    },
                    None,
                    Some(&unique_id.to_string()),
                );

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
                    if let HydroNode::Persist { inner: input, .. } = input.as_ref() {
                        (input, quote!('static))
                    } else {
                        (input, quote!('tick))
                    };

                let (input_ident, input_location_id) =
                    input.emit(graph_builders, built_tees, next_stmt_id);

                let fold_id = *next_stmt_id;
                *next_stmt_id += 1;

                let fold_ident =
                    syn::Ident::new(&format!("stream_{}", fold_id), Span::call_site());

                let builder = graph_builders.entry(input_location_id).or_default();
                builder.add_dfir(
                    parse_quote! {
                        #fold_ident = #input_ident -> #operator::<#lifetime>(#init, #acc);
                    },
                    None,
                    Some(&fold_id.to_string()),
                );

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
                    if let HydroNode::Persist { inner: input, .. } = input.as_ref() {
                        (input, quote!('static))
                    } else {
                        (input, quote!('tick))
                    };

                let (input_ident, input_location_id) =
                    input.emit(graph_builders, built_tees, next_stmt_id);

                let reduce_id = *next_stmt_id;
                *next_stmt_id += 1;

                let reduce_ident =
                    syn::Ident::new(&format!("stream_{}", reduce_id), Span::call_site());

                let builder = graph_builders.entry(input_location_id).or_default();
                builder.add_dfir(
                    parse_quote! {
                        #reduce_ident = #input_ident -> #operator::<#lifetime>(#f);
                    },
                    None,
                    Some(&reduce_id.to_string()),
                );

                (reduce_ident, input_location_id)
            }

            HydroNode::Network {
                from_location: _,
                from_key: _,
                to_location,
                to_key: _,
                serialize_fn: serialize_pipeline,
                instantiate_fn,
                deserialize_fn: deserialize_pipeline,
                input,
                ..
            } => {
                let (sink_expr, source_expr, _connect_fn) = match instantiate_fn {
                    DebugInstantiate::Building() => {
                        panic!("Expected the network to be finalized")
                    }

                    DebugInstantiate::Finalized(sink, source, connect_fn) => {
                        (sink, source, connect_fn)
                    }
                };

                let (input_ident, input_location_id) =
                    input.emit(graph_builders, built_tees, next_stmt_id);

                let sender_builder = graph_builders.entry(input_location_id).or_default();
                let stream_id = *next_stmt_id;
                *next_stmt_id += 1;

                if let Some(serialize_pipeline) = serialize_pipeline {
                    sender_builder.add_dfir(
                        parse_quote! {
                            #input_ident -> map(#serialize_pipeline) -> dest_sink(#sink_expr);
                        },
                        None,
                        Some(&stream_id.to_string()),
                    );
                } else {
                    sender_builder.add_dfir(
                        parse_quote! {
                            #input_ident -> dest_sink(#sink_expr);
                        },
                        None,
                        Some(&stream_id.to_string()),
                    );
                }

                let to_id = match to_location {
                    LocationId::Process(id) => id,
                    LocationId::Cluster(id) => id,
                    LocationId::Tick(_, _) => panic!(),
                    LocationId::ExternalProcess(id) => id,
                };

                let receiver_builder = graph_builders.entry(*to_id).or_default();
                

                let receiver_stream_ident =
                    syn::Ident::new(&format!("stream_{}", stream_id), Span::call_site());

                if let Some(deserialize_pipeline) = deserialize_pipeline {
                    receiver_builder.add_dfir(parse_quote! {
                        #receiver_stream_ident = source_stream(#source_expr) -> map(#deserialize_pipeline);
                    }, None, Some(&stream_id.to_string()));
                } else {
                    receiver_builder.add_dfir(
                        parse_quote! {
                            #receiver_stream_ident = source_stream(#source_expr);
                        },
                        None,
                        Some(&stream_id.to_string()),
                    );
                }

                (receiver_stream_ident, *to_id)
            }
        }
    }

    pub fn metadata(&self) -> &HydroNodeMetadata {
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
            HydroNode::Persist { .. } => format!("Persist()"),
            HydroNode::Unpersist { .. } => format!("Unpersist()"),
            HydroNode::Delta { .. } => format!("Delta()"),
            HydroNode::Chain { first, second, .. } => {
                format!("Chain({}, {})", first.print_root(), second.print_root())
            }
            HydroNode::CrossProduct { left, right, .. } => {
                format!("CrossProduct({}, {})", left.print_root(), right.print_root())
            }
            HydroNode::CrossSingleton { left, right, .. } => {
                format!("CrossSingleton({}, {})", left.print_root(), right.print_root())
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
            HydroNode::DeferTick { .. } => format!("DeferTick()"),
            HydroNode::Enumerate { is_static, .. } => format!("Enumerate({:?})", is_static),
            HydroNode::Inspect { f, .. } => format!("Inspect({:?})", f),
            HydroNode::Unique { .. } => format!("Unique()"),
            HydroNode::Sort { .. } => format!("Sort()"),
            HydroNode::Fold { init, acc, .. } => format!("Fold({:?}, {:?})", init, acc),
            HydroNode::FoldKeyed { init, acc, .. } => format!("FoldKeyed({:?}, {:?})", init, acc),
            HydroNode::Reduce { f, .. } => format!("Reduce({:?})", f),
            HydroNode::ReduceKeyed { f, .. } => format!("ReduceKeyed({:?})", f),
            HydroNode::Network { .. } => format!("Network()"),

        }
    }
}

#[cfg(feature = "build")]
#[expect(clippy::too_many_arguments, reason = "networking internals")]
fn instantiate_network<'a, D: Deploy<'a>>(
    from_location: &mut LocationId,
    from_key: Option<usize>,
    to_location: &mut LocationId,
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
