use core::panic;
use std::cell::RefCell;
#[cfg(feature = "build")]
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::fmt::Debug;
use std::ops::Deref;
use std::rc::Rc;

#[cfg(feature = "build")]
use dfir_lang::graph::FlatGraphBuilder;
#[cfg(feature = "build")]
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::ToTokens;
#[cfg(feature = "build")]
use syn::parse_quote;

#[cfg(feature = "build")]
use crate::deploy::{Deploy, RegisterPort};
use crate::location::LocationId;

#[derive(Clone)]
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

pub enum DebugInstantiate {
    Building(),
    Finalized(syn::Expr, syn::Expr, Option<Box<dyn FnOnce()>>),
}

impl Debug for DebugInstantiate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<network instantiate>")
    }
}

/// A source in a Hydro graph, where data enters the graph.
#[derive(Debug)]
pub enum HydroSource {
    Stream(DebugExpr),
    ExternalNetwork(),
    Iter(DebugExpr),
    Spin(),
}

/// An leaf in a Hydro graph, which is an pipeline that doesn't emit
/// any downstream values. Traversals over the dataflow graph and
/// generating Hydroflow IR start from leaves.
#[derive(Debug)]
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
        self,
        compile_env: &D::CompileEnv,
        seen_tees: &mut SeenTees,
        processes: &HashMap<usize, D::Process>,
        clusters: &HashMap<usize, D::Cluster>,
        externals: &HashMap<usize, D::ExternalProcess>,
    ) -> HydroLeaf {
        self.transform_children(
            |n, s| {
                n.compile_network::<D>(compile_env, s, processes, clusters, externals);
            },
            seen_tees,
        )
    }

    pub fn connect_network(self, seen_tees: &mut SeenTees) -> HydroLeaf {
        self.transform_children(
            |n, s| {
                n.connect_network(s);
            },
            seen_tees,
        )
    }

    pub fn transform_children(
        self,
        mut transform: impl FnMut(&mut HydroNode, &mut SeenTees),
        seen_tees: &mut SeenTees,
    ) -> HydroLeaf {
        match self {
            HydroLeaf::ForEach { f, mut input } => {
                transform(&mut input, seen_tees);
                HydroLeaf::ForEach { f, input }
            }
            HydroLeaf::DestSink { sink, mut input } => {
                transform(&mut input, seen_tees);
                HydroLeaf::DestSink { sink, input }
            }
            HydroLeaf::CycleSink {
                ident,
                location_kind,
                mut input,
            } => {
                transform(&mut input, seen_tees);
                HydroLeaf::CycleSink {
                    ident,
                    location_kind,
                    input,
                }
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

                graph_builders
                    .entry(input_location_id)
                    .or_default()
                    .add_statement(parse_quote! {
                        #input_ident -> for_each(#f);
                    });
            }

            HydroLeaf::DestSink { sink, input } => {
                let (input_ident, input_location_id) =
                    input.emit(graph_builders, built_tees, next_stmt_id);

                graph_builders
                    .entry(input_location_id)
                    .or_default()
                    .add_statement(parse_quote! {
                        #input_ident -> dest_sink(#sink);
                    });
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

                graph_builders
                    .entry(*location_id)
                    .or_default()
                    .add_statement(parse_quote! {
                        #ident = #input_ident;
                    });
            }
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

/// An intermediate node in a Hydro graph, which consumes data
/// from upstream nodes and emits data to downstream nodes.
#[derive(Debug)]
pub enum HydroNode {
    Placeholder,

    Source {
        source: HydroSource,
        location_kind: LocationId,
    },

    CycleSource {
        ident: syn::Ident,
        location_kind: LocationId,
    },

    Tee {
        inner: TeeNode,
    },

    Persist(Box<HydroNode>),
    Unpersist(Box<HydroNode>),
    Delta(Box<HydroNode>),

    Chain(Box<HydroNode>, Box<HydroNode>),
    CrossProduct(Box<HydroNode>, Box<HydroNode>),
    CrossSingleton(Box<HydroNode>, Box<HydroNode>),
    Join(Box<HydroNode>, Box<HydroNode>),
    Difference(Box<HydroNode>, Box<HydroNode>),
    AntiJoin(Box<HydroNode>, Box<HydroNode>),

    Map {
        f: DebugExpr,
        input: Box<HydroNode>,
    },
    FlatMap {
        f: DebugExpr,
        input: Box<HydroNode>,
    },
    Filter {
        f: DebugExpr,
        input: Box<HydroNode>,
    },
    FilterMap {
        f: DebugExpr,
        input: Box<HydroNode>,
    },

    DeferTick(Box<HydroNode>),
    Enumerate {
        is_static: bool,
        input: Box<HydroNode>,
    },
    Inspect {
        f: DebugExpr,
        input: Box<HydroNode>,
    },

    Unique(Box<HydroNode>),

    Sort(Box<HydroNode>),
    Fold {
        init: DebugExpr,
        acc: DebugExpr,
        input: Box<HydroNode>,
    },
    FoldKeyed {
        init: DebugExpr,
        acc: DebugExpr,
        input: Box<HydroNode>,
    },

    Reduce {
        f: DebugExpr,
        input: Box<HydroNode>,
    },
    ReduceKeyed {
        f: DebugExpr,
        input: Box<HydroNode>,
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
        self.transform_children(
            |n, s| n.compile_network::<D>(compile_env, s, nodes, clusters, externals),
            seen_tees,
        );

        if let HydroNode::Network {
            from_location,
            from_key,
            to_location,
            to_key,
            instantiate_fn,
            ..
        } = self
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
    }

    pub fn connect_network(&mut self, seen_tees: &mut SeenTees) {
        self.transform_children(|n, s| n.connect_network(s), seen_tees);
        if let HydroNode::Network { instantiate_fn, .. } = self {
            match instantiate_fn {
                DebugInstantiate::Building() => panic!("network not built"),

                DebugInstantiate::Finalized(_, _, connect_fn) => {
                    connect_fn.take().unwrap()();
                }
            }
        }
    }

    pub fn transform_bottom_up<C>(
        &mut self,
        mut transform: impl FnMut(&mut HydroNode, &mut C) + Copy,
        seen_tees: &mut SeenTees,
        ctx: &mut C,
    ) {
        self.transform_children(|n, s| n.transform_bottom_up(transform, s, ctx), seen_tees);

        transform(self, ctx)
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

            HydroNode::Source { .. } => {}

            HydroNode::CycleSource { .. } => {}

            HydroNode::Tee { inner } => {
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

            HydroNode::Persist(inner) => transform(inner.as_mut(), seen_tees),
            HydroNode::Unpersist(inner) => transform(inner.as_mut(), seen_tees),
            HydroNode::Delta(inner) => transform(inner.as_mut(), seen_tees),

            HydroNode::Chain(left, right) => {
                transform(left.as_mut(), seen_tees);
                transform(right.as_mut(), seen_tees);
            }
            HydroNode::CrossProduct(left, right) => {
                transform(left.as_mut(), seen_tees);
                transform(right.as_mut(), seen_tees);
            }
            HydroNode::CrossSingleton(left, right) => {
                transform(left.as_mut(), seen_tees);
                transform(right.as_mut(), seen_tees);
            }
            HydroNode::Join(left, right) => {
                transform(left.as_mut(), seen_tees);
                transform(right.as_mut(), seen_tees);
            }
            HydroNode::Difference(left, right) => {
                transform(left.as_mut(), seen_tees);
                transform(right.as_mut(), seen_tees);
            }
            HydroNode::AntiJoin(left, right) => {
                transform(left.as_mut(), seen_tees);
                transform(right.as_mut(), seen_tees);
            }

            HydroNode::Map { input, .. } => {
                transform(input.as_mut(), seen_tees);
            }
            HydroNode::FlatMap { input, .. } => {
                transform(input.as_mut(), seen_tees);
            }
            HydroNode::Filter { input, .. } => {
                transform(input.as_mut(), seen_tees);
            }
            HydroNode::FilterMap { input, .. } => {
                transform(input.as_mut(), seen_tees);
            }
            HydroNode::Sort(input) => {
                transform(input.as_mut(), seen_tees);
            }
            HydroNode::DeferTick(input) => {
                transform(input.as_mut(), seen_tees);
            }
            HydroNode::Enumerate { input, .. } => {
                transform(input.as_mut(), seen_tees);
            }
            HydroNode::Inspect { input, .. } => {
                transform(input.as_mut(), seen_tees);
            }

            HydroNode::Unique(input) => {
                transform(input.as_mut(), seen_tees);
            }

            HydroNode::Fold { input, .. } => {
                transform(input.as_mut(), seen_tees);
            }
            HydroNode::FoldKeyed { input, .. } => {
                transform(input.as_mut(), seen_tees);
            }

            HydroNode::Reduce { input, .. } => {
                transform(input.as_mut(), seen_tees);
            }
            HydroNode::ReduceKeyed { input, .. } => {
                transform(input.as_mut(), seen_tees);
            }

            HydroNode::Network { input, .. } => {
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

            HydroNode::Persist(inner) => {
                let (inner_ident, location) = inner.emit(graph_builders, built_tees, next_stmt_id);

                let persist_id = *next_stmt_id;
                *next_stmt_id += 1;

                let persist_ident =
                    syn::Ident::new(&format!("stream_{}", persist_id), Span::call_site());

                let builder = graph_builders.entry(location).or_default();
                builder.add_statement(parse_quote! {
                    #persist_ident = #inner_ident -> persist::<'static>();
                });

                (persist_ident, location)
            }

            HydroNode::Unpersist(_) => {
                panic!("Unpersist is a marker node and should have been optimized away. This is likely a compiler bug.")
            }

            HydroNode::Delta(inner) => {
                let (inner_ident, location) = inner.emit(graph_builders, built_tees, next_stmt_id);

                let delta_id = *next_stmt_id;
                *next_stmt_id += 1;

                let delta_ident =
                    syn::Ident::new(&format!("stream_{}", delta_id), Span::call_site());

                let builder = graph_builders.entry(location).or_default();
                builder.add_statement(parse_quote! {
                    #delta_ident = #inner_ident -> multiset_delta();
                });

                (delta_ident, location)
            }

            HydroNode::Source {
                source,
                location_kind,
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

                    graph_builders
                        .entry(*location_id)
                        .or_default()
                        .add_statement(source_stmt);

                    (source_ident, *location_id)
                }
            }

            HydroNode::CycleSource {
                ident,
                location_kind,
            } => {
                let location_id = match location_kind.root() {
                    LocationId::Process(id) => id,
                    LocationId::Cluster(id) => id,
                    LocationId::Tick(_, _) => panic!(),
                    LocationId::ExternalProcess(_) => panic!(),
                };

                (ident.clone(), *location_id)
            }

            HydroNode::Tee { inner } => {
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
                    builder.add_statement(parse_quote! {
                        #tee_ident = #inner_ident -> tee();
                    });

                    built_tees.insert(
                        inner.0.as_ref() as *const RefCell<HydroNode>,
                        (tee_ident.clone(), inner_location_id),
                    );

                    (tee_ident, inner_location_id)
                }
            }

            HydroNode::Chain(left, right) => {
                let (left_ident, left_location_id) =
                    left.emit(graph_builders, built_tees, next_stmt_id);
                let (right_ident, right_location_id) =
                    right.emit(graph_builders, built_tees, next_stmt_id);

                assert_eq!(
                    left_location_id, right_location_id,
                    "chain inputs must be in the same location"
                );

                let union_id = *next_stmt_id;
                *next_stmt_id += 1;

                let chain_ident =
                    syn::Ident::new(&format!("stream_{}", union_id), Span::call_site());

                let builder = graph_builders.entry(left_location_id).or_default();
                builder.add_statement(parse_quote! {
                    #chain_ident = chain();
                });

                builder.add_statement(parse_quote! {
                    #left_ident -> [0]#chain_ident;
                });

                builder.add_statement(parse_quote! {
                    #right_ident -> [1]#chain_ident;
                });

                (chain_ident, left_location_id)
            }

            HydroNode::CrossSingleton(left, right) => {
                let (left_ident, left_location_id) =
                    left.emit(graph_builders, built_tees, next_stmt_id);
                let (right_ident, right_location_id) =
                    right.emit(graph_builders, built_tees, next_stmt_id);

                assert_eq!(
                    left_location_id, right_location_id,
                    "cross_singleton inputs must be in the same location"
                );

                let union_id = *next_stmt_id;
                *next_stmt_id += 1;

                let cross_ident =
                    syn::Ident::new(&format!("stream_{}", union_id), Span::call_site());

                let builder = graph_builders.entry(left_location_id).or_default();
                builder.add_statement(parse_quote! {
                    #cross_ident = cross_singleton();
                });

                builder.add_statement(parse_quote! {
                    #left_ident -> [input]#cross_ident;
                });

                builder.add_statement(parse_quote! {
                    #right_ident -> [single]#cross_ident;
                });

                (cross_ident, left_location_id)
            }

            HydroNode::CrossProduct(..) | HydroNode::Join(..) => {
                let operator: syn::Ident = if matches!(self, HydroNode::CrossProduct(..)) {
                    parse_quote!(cross_join_multiset)
                } else {
                    parse_quote!(join_multiset)
                };

                let (HydroNode::CrossProduct(left, right) | HydroNode::Join(left, right)) = self
                else {
                    unreachable!()
                };

                let (left_inner, left_was_persist) = if let HydroNode::Persist(left) = left.as_ref()
                {
                    (left, true)
                } else {
                    (left, false)
                };

                let (right_inner, right_was_persist) =
                    if let HydroNode::Persist(right) = right.as_ref() {
                        (right, true)
                    } else {
                        (right, false)
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

                match (left_was_persist, right_was_persist) {
                    (true, true) => {
                        builder.add_statement(parse_quote! {
                            #stream_ident = #operator::<'static, 'static>();
                        });
                    }
                    (true, false) => {
                        builder.add_statement(parse_quote! {
                            #stream_ident = #operator::<'static, 'tick>();
                        });
                    }
                    (false, true) => {
                        builder.add_statement(parse_quote! {
                            #stream_ident = #operator::<'tick, 'static>();
                        });
                    }
                    (false, false) => {
                        builder.add_statement(parse_quote! {
                            #stream_ident = #operator::<'tick, 'tick>();
                        });
                    }
                };

                builder.add_statement(parse_quote! {
                    #left_ident -> [0]#stream_ident;
                });

                builder.add_statement(parse_quote! {
                    #right_ident -> [1]#stream_ident;
                });

                (stream_ident, left_location_id)
            }

            HydroNode::Difference(..) | HydroNode::AntiJoin(..) => {
                let operator: syn::Ident = if matches!(self, HydroNode::Difference(..)) {
                    parse_quote!(difference_multiset)
                } else {
                    parse_quote!(anti_join_multiset)
                };

                let (HydroNode::Difference(left, right) | HydroNode::AntiJoin(left, right)) = self
                else {
                    unreachable!()
                };

                let (right, right_was_persist) = if let HydroNode::Persist(right) = right.as_ref() {
                    (right, true)
                } else {
                    (right, false)
                };

                let (left_ident, left_location_id) =
                    left.emit(graph_builders, built_tees, next_stmt_id);
                let (right_ident, right_location_id) =
                    right.emit(graph_builders, built_tees, next_stmt_id);

                assert_eq!(
                    left_location_id, right_location_id,
                    "difference / anti join inputs must be in the same location"
                );

                let stream_id = *next_stmt_id;
                *next_stmt_id += 1;

                let stream_ident =
                    syn::Ident::new(&format!("stream_{}", stream_id), Span::call_site());

                let builder = graph_builders.entry(left_location_id).or_default();

                if right_was_persist {
                    builder.add_statement(parse_quote! {
                        #stream_ident = #operator::<'tick, 'static>();
                    });
                } else {
                    builder.add_statement(parse_quote! {
                        #stream_ident = #operator::<'tick, 'tick>();
                    });
                }

                builder.add_statement(parse_quote! {
                    #left_ident -> [pos]#stream_ident;
                });

                builder.add_statement(parse_quote! {
                    #right_ident -> [neg]#stream_ident;
                });

                (stream_ident, left_location_id)
            }

            HydroNode::Map { f, input } => {
                let (input_ident, input_location_id) =
                    input.emit(graph_builders, built_tees, next_stmt_id);

                let map_id = *next_stmt_id;
                *next_stmt_id += 1;

                let map_ident = syn::Ident::new(&format!("stream_{}", map_id), Span::call_site());

                let builder = graph_builders.entry(input_location_id).or_default();
                builder.add_statement(parse_quote! {
                    #map_ident = #input_ident -> map(#f);
                });

                (map_ident, input_location_id)
            }

            HydroNode::FlatMap { f, input } => {
                let (input_ident, input_location_id) =
                    input.emit(graph_builders, built_tees, next_stmt_id);

                let flat_map_id = *next_stmt_id;
                *next_stmt_id += 1;

                let flat_map_ident =
                    syn::Ident::new(&format!("stream_{}", flat_map_id), Span::call_site());

                let builder = graph_builders.entry(input_location_id).or_default();
                builder.add_statement(parse_quote! {
                    #flat_map_ident = #input_ident -> flat_map(#f);
                });

                (flat_map_ident, input_location_id)
            }

            HydroNode::Filter { f, input } => {
                let (input_ident, input_location_id) =
                    input.emit(graph_builders, built_tees, next_stmt_id);

                let filter_id = *next_stmt_id;
                *next_stmt_id += 1;

                let filter_ident =
                    syn::Ident::new(&format!("stream_{}", filter_id), Span::call_site());

                let builder = graph_builders.entry(input_location_id).or_default();
                builder.add_statement(parse_quote! {
                    #filter_ident = #input_ident -> filter(#f);
                });

                (filter_ident, input_location_id)
            }

            HydroNode::FilterMap { f, input } => {
                let (input_ident, input_location_id) =
                    input.emit(graph_builders, built_tees, next_stmt_id);

                let filter_map_id = *next_stmt_id;
                *next_stmt_id += 1;

                let filter_map_ident =
                    syn::Ident::new(&format!("stream_{}", filter_map_id), Span::call_site());

                let builder = graph_builders.entry(input_location_id).or_default();
                builder.add_statement(parse_quote! {
                    #filter_map_ident = #input_ident -> filter_map(#f);
                });

                (filter_map_ident, input_location_id)
            }

            HydroNode::Sort(input) => {
                let (input_ident, input_location_id) =
                    input.emit(graph_builders, built_tees, next_stmt_id);

                let sort_id = *next_stmt_id;
                *next_stmt_id += 1;

                let sort_ident = syn::Ident::new(&format!("stream_{}", sort_id), Span::call_site());

                let builder = graph_builders.entry(input_location_id).or_default();
                builder.add_statement(parse_quote! {
                    #sort_ident = #input_ident -> sort();
                });

                (sort_ident, input_location_id)
            }

            HydroNode::DeferTick(input) => {
                let (input_ident, input_location_id) =
                    input.emit(graph_builders, built_tees, next_stmt_id);

                let defer_tick_id = *next_stmt_id;
                *next_stmt_id += 1;

                let defer_tick_ident =
                    syn::Ident::new(&format!("stream_{}", defer_tick_id), Span::call_site());

                let builder = graph_builders.entry(input_location_id).or_default();
                builder.add_statement(parse_quote! {
                    #defer_tick_ident = #input_ident -> defer_tick_lazy();
                });

                (defer_tick_ident, input_location_id)
            }

            HydroNode::Enumerate { is_static, input } => {
                let (input_ident, input_location_id) =
                    input.emit(graph_builders, built_tees, next_stmt_id);

                let enumerate_id = *next_stmt_id;
                *next_stmt_id += 1;

                let enumerate_ident =
                    syn::Ident::new(&format!("stream_{}", enumerate_id), Span::call_site());

                let builder = graph_builders.entry(input_location_id).or_default();

                if *is_static {
                    builder.add_statement(parse_quote! {
                        #enumerate_ident = #input_ident -> enumerate::<'static>();
                    });
                } else {
                    builder.add_statement(parse_quote! {
                        #enumerate_ident = #input_ident -> enumerate::<'tick>();
                    });
                }

                (enumerate_ident, input_location_id)
            }

            HydroNode::Inspect { f, input } => {
                let (input_ident, input_location_id) =
                    input.emit(graph_builders, built_tees, next_stmt_id);

                let inspect_id = *next_stmt_id;
                *next_stmt_id += 1;

                let inspect_ident =
                    syn::Ident::new(&format!("stream_{}", inspect_id), Span::call_site());

                let builder = graph_builders.entry(input_location_id).or_default();
                builder.add_statement(parse_quote! {
                    #inspect_ident = #input_ident -> inspect(#f);
                });

                (inspect_ident, input_location_id)
            }

            HydroNode::Unique(input) => {
                let (input_ident, input_location_id) =
                    input.emit(graph_builders, built_tees, next_stmt_id);

                let unique_id = *next_stmt_id;
                *next_stmt_id += 1;

                let unique_ident =
                    syn::Ident::new(&format!("stream_{}", unique_id), Span::call_site());

                let builder = graph_builders.entry(input_location_id).or_default();
                builder.add_statement(parse_quote! {
                    #unique_ident = #input_ident -> unique::<'tick>();
                });

                (unique_ident, input_location_id)
            }

            HydroNode::Fold { .. } | HydroNode::FoldKeyed { .. } => {
                let operator: syn::Ident = if matches!(self, HydroNode::Fold { .. }) {
                    parse_quote!(fold)
                } else {
                    parse_quote!(fold_keyed)
                };

                let (HydroNode::Fold { init, acc, input }
                | HydroNode::FoldKeyed { init, acc, input }) = self
                else {
                    unreachable!()
                };

                let (input, input_was_persist) = if let HydroNode::Persist(input) = input.as_ref() {
                    (input, true)
                } else {
                    (input, false)
                };

                let (input_ident, input_location_id) =
                    input.emit(graph_builders, built_tees, next_stmt_id);

                let reduce_id = *next_stmt_id;
                *next_stmt_id += 1;

                let fold_ident =
                    syn::Ident::new(&format!("stream_{}", reduce_id), Span::call_site());

                let builder = graph_builders.entry(input_location_id).or_default();
                if input_was_persist {
                    builder.add_statement(parse_quote! {
                        #fold_ident = #input_ident -> #operator::<'static>(#init, #acc);
                    });
                } else {
                    builder.add_statement(parse_quote! {
                        #fold_ident = #input_ident -> #operator::<'tick>(#init, #acc);
                    });
                }

                (fold_ident, input_location_id)
            }

            HydroNode::Reduce { .. } | HydroNode::ReduceKeyed { .. } => {
                let operator: syn::Ident = if matches!(self, HydroNode::Reduce { .. }) {
                    parse_quote!(reduce)
                } else {
                    parse_quote!(reduce_keyed)
                };

                let (HydroNode::Reduce { f, input } | HydroNode::ReduceKeyed { f, input }) = self
                else {
                    unreachable!()
                };

                let (input, input_was_persist) = if let HydroNode::Persist(input) = input.as_ref() {
                    (input, true)
                } else {
                    (input, false)
                };

                let (input_ident, input_location_id) =
                    input.emit(graph_builders, built_tees, next_stmt_id);

                let reduce_id = *next_stmt_id;
                *next_stmt_id += 1;

                let reduce_ident =
                    syn::Ident::new(&format!("stream_{}", reduce_id), Span::call_site());

                let builder = graph_builders.entry(input_location_id).or_default();
                if input_was_persist {
                    builder.add_statement(parse_quote! {
                        #reduce_ident = #input_ident -> #operator::<'static>(#f);
                    });
                } else {
                    builder.add_statement(parse_quote! {
                        #reduce_ident = #input_ident -> #operator::<'tick>(#f);
                    });
                }

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

                if let Some(serialize_pipeline) = serialize_pipeline {
                    sender_builder.add_statement(parse_quote! {
                        #input_ident -> map(#serialize_pipeline) -> dest_sink(#sink_expr);
                    });
                } else {
                    sender_builder.add_statement(parse_quote! {
                        #input_ident -> dest_sink(#sink_expr);
                    });
                }

                let to_id = match to_location {
                    LocationId::Process(id) => id,
                    LocationId::Cluster(id) => id,
                    LocationId::Tick(_, _) => panic!(),
                    LocationId::ExternalProcess(id) => id,
                };

                let receiver_builder = graph_builders.entry(*to_id).or_default();
                let receiver_stream_id = *next_stmt_id;
                *next_stmt_id += 1;

                let receiver_stream_ident =
                    syn::Ident::new(&format!("stream_{}", receiver_stream_id), Span::call_site());

                if let Some(deserialize_pipeline) = deserialize_pipeline {
                    receiver_builder.add_statement(parse_quote! {
                        #receiver_stream_ident = source_stream(#source_expr) -> map(#deserialize_pipeline);
                    });
                } else {
                    receiver_builder.add_statement(parse_quote! {
                        #receiver_stream_ident = source_stream(#source_expr);
                    });
                }

                (receiver_stream_ident, *to_id)
            }
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
