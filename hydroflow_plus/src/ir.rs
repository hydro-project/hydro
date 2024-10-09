use core::panic;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::fmt::Debug;
use std::ops::Deref;
use std::rc::Rc;

use hydroflow_lang::graph::FlatGraphBuilder;
use hydroflow_lang::parse::Pipeline;
use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use syn::parse_quote;

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

#[derive(Clone)]
pub struct DebugPipelineFn(pub Rc<dyn Fn() -> Pipeline + 'static>);

impl Debug for DebugPipelineFn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<function>")
    }
}

/// A source in a Hydroflow+ graph, where data enters the graph.
#[derive(Debug)]
pub enum HfPlusSource {
    Stream(DebugExpr),
    ExternalNetwork(),
    Iter(DebugExpr),
    Interval(DebugExpr),
    Spin(),
}

/// An leaf in a Hydroflow+ graph, which is an pipeline that doesn't emit
/// any downstream values. Traversals over the dataflow graph and
/// generating Hydroflow IR start from leaves.
#[derive(Debug)]
pub enum HfPlusLeaf {
    ForEach {
        f: DebugExpr,
        input: Box<HfPlusNode>,
    },
    DestSink {
        sink: DebugExpr,
        input: Box<HfPlusNode>,
    },
    CycleSink {
        ident: syn::Ident,
        location_kind: LocationId,
        input: Box<HfPlusNode>,
    },
}

impl HfPlusLeaf {
    pub fn compile_network<'a, D: Deploy<'a> + 'a>(
        self,
        compile_env: &D::CompileEnv,
        seen_tees: &mut SeenTees,
        nodes: &HashMap<usize, D::Process>,
        clusters: &HashMap<usize, D::Cluster>,
        externals: &HashMap<usize, D::ExternalProcess>,
    ) -> HfPlusLeaf {
        self.transform_children(
            |n, s| {
                n.compile_network::<D>(compile_env, s, nodes, clusters, externals);
            },
            seen_tees,
        )
    }

    pub fn connect_network(self, seen_tees: &mut SeenTees) -> HfPlusLeaf {
        self.transform_children(
            |n, s| {
                n.connect_network(s);
            },
            seen_tees,
        )
    }

    pub fn transform_children(
        self,
        mut transform: impl FnMut(&mut HfPlusNode, &mut SeenTees),
        seen_tees: &mut SeenTees,
    ) -> HfPlusLeaf {
        match self {
            HfPlusLeaf::ForEach { f, mut input } => {
                transform(&mut input, seen_tees);
                HfPlusLeaf::ForEach { f, input }
            }
            HfPlusLeaf::DestSink { sink, mut input } => {
                transform(&mut input, seen_tees);
                HfPlusLeaf::DestSink { sink, input }
            }
            HfPlusLeaf::CycleSink {
                ident,
                location_kind,
                mut input,
            } => {
                transform(&mut input, seen_tees);
                HfPlusLeaf::CycleSink {
                    ident,
                    location_kind,
                    input,
                }
            }
        }
    }

    pub fn emit(
        &self,
        graph_builders: &mut BTreeMap<usize, FlatGraphBuilder>,
        built_tees: &mut HashMap<*const RefCell<HfPlusNode>, (syn::Ident, usize)>,
        next_stmt_id: &mut usize,
    ) {
        match self {
            HfPlusLeaf::ForEach { f, input } => {
                let (input_ident, input_location_id) =
                    input.emit(graph_builders, built_tees, next_stmt_id);

                graph_builders
                    .entry(input_location_id)
                    .or_default()
                    .add_statement(parse_quote! {
                        #input_ident -> for_each(#f);
                    });
            }

            HfPlusLeaf::DestSink { sink, input } => {
                let (input_ident, input_location_id) =
                    input.emit(graph_builders, built_tees, next_stmt_id);

                graph_builders
                    .entry(input_location_id)
                    .or_default()
                    .add_statement(parse_quote! {
                        #input_ident -> dest_sink(#sink);
                    });
            }

            HfPlusLeaf::CycleSink {
                ident,
                location_kind,
                input,
            } => {
                let (input_ident, input_location_id) =
                    input.emit(graph_builders, built_tees, next_stmt_id);

                let location_id = match location_kind {
                    LocationId::Process(id) => id,
                    LocationId::Cluster(id) => id,
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

type PrintedTees = RefCell<Option<(usize, HashMap<*const RefCell<HfPlusNode>, usize>)>>;
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

pub struct TeeNode(pub Rc<RefCell<HfPlusNode>>);

impl Debug for TeeNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        PRINTED_TEES.with(|printed_tees| {
            let mut printed_tees_mut_borrow = printed_tees.borrow_mut();
            let printed_tees_mut = printed_tees_mut_borrow.as_mut();

            if let Some(printed_tees_mut) = printed_tees_mut {
                if let Some(existing) = printed_tees_mut
                    .1
                    .get(&(self.0.as_ref() as *const RefCell<HfPlusNode>))
                {
                    write!(f, "<tee {}>", existing)
                } else {
                    let next_id = printed_tees_mut.0;
                    printed_tees_mut.0 += 1;
                    printed_tees_mut
                        .1
                        .insert(self.0.as_ref() as *const RefCell<HfPlusNode>, next_id);
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

/// An intermediate node in a Hydroflow+ graph, which consumes data
/// from upstream nodes and emits data to downstream nodes.
#[derive(Debug)]
pub enum HfPlusNode {
    Placeholder,

    Source {
        source: HfPlusSource,
        location_kind: LocationId,
    },

    CycleSource {
        ident: syn::Ident,
        location_kind: LocationId,
    },

    Tee {
        inner: TeeNode,
    },

    Persist(Box<HfPlusNode>),
    Unpersist(Box<HfPlusNode>),
    Delta(Box<HfPlusNode>),

    Union(Box<HfPlusNode>, Box<HfPlusNode>),
    CrossProduct(Box<HfPlusNode>, Box<HfPlusNode>),
    CrossSingleton(Box<HfPlusNode>, Box<HfPlusNode>),
    Join(Box<HfPlusNode>, Box<HfPlusNode>),
    Difference(Box<HfPlusNode>, Box<HfPlusNode>),
    AntiJoin(Box<HfPlusNode>, Box<HfPlusNode>),

    Map {
        f: DebugExpr,
        input: Box<HfPlusNode>,
    },
    FlatMap {
        f: DebugExpr,
        input: Box<HfPlusNode>,
    },
    Filter {
        f: DebugExpr,
        input: Box<HfPlusNode>,
    },
    FilterMap {
        f: DebugExpr,
        input: Box<HfPlusNode>,
    },

    DeferTick(Box<HfPlusNode>),
    Enumerate(Box<HfPlusNode>),
    Inspect {
        f: DebugExpr,
        input: Box<HfPlusNode>,
    },

    Unique(Box<HfPlusNode>),

    Sort(Box<HfPlusNode>),
    Fold {
        init: DebugExpr,
        acc: DebugExpr,
        input: Box<HfPlusNode>,
    },
    FoldKeyed {
        init: DebugExpr,
        acc: DebugExpr,
        input: Box<HfPlusNode>,
    },

    Reduce {
        f: DebugExpr,
        input: Box<HfPlusNode>,
    },
    ReduceKeyed {
        f: DebugExpr,
        input: Box<HfPlusNode>,
    },

    Network {
        from_location: LocationId,
        from_key: Option<usize>,
        to_location: LocationId,
        to_key: Option<usize>,
        serialize_pipeline: Option<Pipeline>,
        instantiate_fn: DebugInstantiate,
        deserialize_pipeline: Option<Pipeline>,
        input: Box<HfPlusNode>,
    },
}

pub type SeenTees = HashMap<*const RefCell<HfPlusNode>, Rc<RefCell<HfPlusNode>>>;

impl<'a> HfPlusNode {
    pub fn compile_network<D: Deploy<'a> + 'a>(
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

        if let HfPlusNode::Network {
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
        if let HfPlusNode::Network { instantiate_fn, .. } = self {
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
        mut transform: impl FnMut(&mut HfPlusNode, &mut C) + Copy,
        seen_tees: &mut SeenTees,
        ctx: &mut C,
    ) {
        self.transform_children(|n, s| n.transform_bottom_up(transform, s, ctx), seen_tees);

        transform(self, ctx)
    }

    #[inline(always)]
    pub fn transform_children(
        &mut self,
        mut transform: impl FnMut(&mut HfPlusNode, &mut SeenTees),
        seen_tees: &mut SeenTees,
    ) {
        match self {
            HfPlusNode::Placeholder => {
                panic!();
            }

            HfPlusNode::Source { .. } => {}

            HfPlusNode::CycleSource { .. } => {}

            HfPlusNode::Tee { inner } => {
                if let Some(transformed) =
                    seen_tees.get(&(inner.0.as_ref() as *const RefCell<HfPlusNode>))
                {
                    *inner = TeeNode(transformed.clone());
                } else {
                    let transformed_cell = Rc::new(RefCell::new(HfPlusNode::Placeholder));
                    seen_tees.insert(
                        inner.0.as_ref() as *const RefCell<HfPlusNode>,
                        transformed_cell.clone(),
                    );
                    let mut orig = inner.0.replace(HfPlusNode::Placeholder);
                    transform(&mut orig, seen_tees);
                    *transformed_cell.borrow_mut() = orig;
                    *inner = TeeNode(transformed_cell);
                }
            }

            HfPlusNode::Persist(inner) => transform(inner.as_mut(), seen_tees),
            HfPlusNode::Unpersist(inner) => transform(inner.as_mut(), seen_tees),
            HfPlusNode::Delta(inner) => transform(inner.as_mut(), seen_tees),

            HfPlusNode::Union(left, right) => {
                transform(left.as_mut(), seen_tees);
                transform(right.as_mut(), seen_tees);
            }
            HfPlusNode::CrossProduct(left, right) => {
                transform(left.as_mut(), seen_tees);
                transform(right.as_mut(), seen_tees);
            }
            HfPlusNode::CrossSingleton(left, right) => {
                transform(left.as_mut(), seen_tees);
                transform(right.as_mut(), seen_tees);
            }
            HfPlusNode::Join(left, right) => {
                transform(left.as_mut(), seen_tees);
                transform(right.as_mut(), seen_tees);
            }
            HfPlusNode::Difference(left, right) => {
                transform(left.as_mut(), seen_tees);
                transform(right.as_mut(), seen_tees);
            }
            HfPlusNode::AntiJoin(left, right) => {
                transform(left.as_mut(), seen_tees);
                transform(right.as_mut(), seen_tees);
            }

            HfPlusNode::Map { input, .. } => {
                transform(input.as_mut(), seen_tees);
            }
            HfPlusNode::FlatMap { input, .. } => {
                transform(input.as_mut(), seen_tees);
            }
            HfPlusNode::Filter { input, .. } => {
                transform(input.as_mut(), seen_tees);
            }
            HfPlusNode::FilterMap { input, .. } => {
                transform(input.as_mut(), seen_tees);
            }
            HfPlusNode::Sort(input) => {
                transform(input.as_mut(), seen_tees);
            }
            HfPlusNode::DeferTick(input) => {
                transform(input.as_mut(), seen_tees);
            }
            HfPlusNode::Enumerate(input) => {
                transform(input.as_mut(), seen_tees);
            }
            HfPlusNode::Inspect { input, .. } => {
                transform(input.as_mut(), seen_tees);
            }

            HfPlusNode::Unique(input) => {
                transform(input.as_mut(), seen_tees);
            }

            HfPlusNode::Fold { input, .. } => {
                transform(input.as_mut(), seen_tees);
            }
            HfPlusNode::FoldKeyed { input, .. } => {
                transform(input.as_mut(), seen_tees);
            }

            HfPlusNode::Reduce { input, .. } => {
                transform(input.as_mut(), seen_tees);
            }
            HfPlusNode::ReduceKeyed { input, .. } => {
                transform(input.as_mut(), seen_tees);
            }

            HfPlusNode::Network { input, .. } => {
                transform(input.as_mut(), seen_tees);
            }
        }
    }

    pub fn emit(
        &self,
        graph_builders: &mut BTreeMap<usize, FlatGraphBuilder>,
        built_tees: &mut HashMap<*const RefCell<HfPlusNode>, (syn::Ident, usize)>,
        next_stmt_id: &mut usize,
    ) -> (syn::Ident, usize) {
        match self {
            HfPlusNode::Placeholder => {
                panic!()
            }

            HfPlusNode::Persist(inner) => {
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

            HfPlusNode::Unpersist(_) => {
                panic!("Unpersist is a marker node and should have been optimized away. This is likely a compiler bug.")
            }

            HfPlusNode::Delta(inner) => {
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

            HfPlusNode::Source {
                source,
                location_kind,
            } => {
                let location_id = match location_kind {
                    LocationId::Process(id) => id,
                    LocationId::Cluster(id) => id,
                    LocationId::ExternalProcess(id) => id,
                };

                if let HfPlusSource::ExternalNetwork() = source {
                    (syn::Ident::new("DUMMY", Span::call_site()), *location_id)
                } else {
                    let source_id = *next_stmt_id;
                    *next_stmt_id += 1;

                    let source_ident =
                        syn::Ident::new(&format!("stream_{}", source_id), Span::call_site());

                    let source_stmt = match source {
                        HfPlusSource::Stream(expr) => {
                            parse_quote! {
                                #source_ident = source_stream(#expr);
                            }
                        }

                        HfPlusSource::ExternalNetwork() => {
                            unreachable!()
                        }

                        HfPlusSource::Iter(expr) => {
                            parse_quote! {
                                #source_ident = source_iter(#expr);
                            }
                        }

                        HfPlusSource::Interval(expr) => {
                            parse_quote! {
                                #source_ident = source_interval(#expr);
                            }
                        }

                        HfPlusSource::Spin() => {
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

            HfPlusNode::CycleSource {
                ident,
                location_kind,
            } => {
                let location_id = match location_kind {
                    LocationId::Process(id) => id,
                    LocationId::Cluster(id) => id,
                    LocationId::ExternalProcess(_) => panic!(),
                };

                (ident.clone(), *location_id)
            }

            HfPlusNode::Tee { inner } => {
                if let Some(ret) = built_tees.get(&(inner.0.as_ref() as *const RefCell<HfPlusNode>))
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
                        inner.0.as_ref() as *const RefCell<HfPlusNode>,
                        (tee_ident.clone(), inner_location_id),
                    );

                    (tee_ident, inner_location_id)
                }
            }

            HfPlusNode::Union(left, right) => {
                let (left_ident, left_location_id) =
                    left.emit(graph_builders, built_tees, next_stmt_id);
                let (right_ident, right_location_id) =
                    right.emit(graph_builders, built_tees, next_stmt_id);

                assert_eq!(
                    left_location_id, right_location_id,
                    "union inputs must be in the same location"
                );

                let union_id = *next_stmt_id;
                *next_stmt_id += 1;

                let union_ident =
                    syn::Ident::new(&format!("stream_{}", union_id), Span::call_site());

                let builder = graph_builders.entry(left_location_id).or_default();
                builder.add_statement(parse_quote! {
                    #union_ident = union();
                });

                builder.add_statement(parse_quote! {
                    #left_ident -> [0]#union_ident;
                });

                builder.add_statement(parse_quote! {
                    #right_ident -> [1]#union_ident;
                });

                (union_ident, left_location_id)
            }

            HfPlusNode::CrossSingleton(left, right) => {
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

            HfPlusNode::CrossProduct(..) | HfPlusNode::Join(..) => {
                let operator: syn::Ident = if matches!(self, HfPlusNode::CrossProduct(..)) {
                    parse_quote!(cross_join_multiset)
                } else {
                    parse_quote!(join_multiset)
                };

                let (HfPlusNode::CrossProduct(left, right) | HfPlusNode::Join(left, right)) = self
                else {
                    unreachable!()
                };

                let (left_inner, left_was_persist) =
                    if let HfPlusNode::Persist(left) = left.as_ref() {
                        (left, true)
                    } else {
                        (left, false)
                    };

                let (right_inner, right_was_persist) =
                    if let HfPlusNode::Persist(right) = right.as_ref() {
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

            HfPlusNode::Difference(..) | HfPlusNode::AntiJoin(..) => {
                let operator: syn::Ident = if matches!(self, HfPlusNode::Difference(..)) {
                    parse_quote!(difference_multiset)
                } else {
                    parse_quote!(anti_join_multiset)
                };

                let (HfPlusNode::Difference(left, right) | HfPlusNode::AntiJoin(left, right)) =
                    self
                else {
                    unreachable!()
                };

                let (right, right_was_persist) = if let HfPlusNode::Persist(right) = right.as_ref()
                {
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

            HfPlusNode::Map { f, input } => {
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

            HfPlusNode::FlatMap { f, input } => {
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

            HfPlusNode::Filter { f, input } => {
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

            HfPlusNode::FilterMap { f, input } => {
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

            HfPlusNode::Sort(input) => {
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

            HfPlusNode::DeferTick(input) => {
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

            HfPlusNode::Enumerate(input) => {
                let (input_ident, input_location_id) =
                    input.emit(graph_builders, built_tees, next_stmt_id);

                let enumerate_id = *next_stmt_id;
                *next_stmt_id += 1;

                let enumerate_ident =
                    syn::Ident::new(&format!("stream_{}", enumerate_id), Span::call_site());

                let builder = graph_builders.entry(input_location_id).or_default();
                builder.add_statement(parse_quote! {
                    #enumerate_ident = #input_ident -> enumerate();
                });

                (enumerate_ident, input_location_id)
            }

            HfPlusNode::Inspect { f, input } => {
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

            HfPlusNode::Unique(input) => {
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

            HfPlusNode::Fold { .. } | HfPlusNode::FoldKeyed { .. } => {
                let operator: syn::Ident = if matches!(self, HfPlusNode::Fold { .. }) {
                    parse_quote!(fold)
                } else {
                    parse_quote!(fold_keyed)
                };

                let (HfPlusNode::Fold { init, acc, input }
                | HfPlusNode::FoldKeyed { init, acc, input }) = self
                else {
                    unreachable!()
                };

                let (input, input_was_persist) = if let HfPlusNode::Persist(input) = input.as_ref()
                {
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

            HfPlusNode::Reduce { .. } | HfPlusNode::ReduceKeyed { .. } => {
                let operator: syn::Ident = if matches!(self, HfPlusNode::Reduce { .. }) {
                    parse_quote!(reduce)
                } else {
                    parse_quote!(reduce_keyed)
                };

                let (HfPlusNode::Reduce { f, input } | HfPlusNode::ReduceKeyed { f, input }) = self
                else {
                    unreachable!()
                };

                let (input, input_was_persist) = if let HfPlusNode::Persist(input) = input.as_ref()
                {
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

            HfPlusNode::Network {
                from_location: _,
                from_key: _,
                to_location,
                to_key: _,
                serialize_pipeline,
                instantiate_fn,
                deserialize_pipeline,
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
                        #input_ident -> #serialize_pipeline -> dest_sink(#sink_expr);
                    });
                } else {
                    sender_builder.add_statement(parse_quote! {
                        #input_ident -> dest_sink(#sink_expr);
                    });
                }

                let to_id = match to_location {
                    LocationId::Process(id) => id,
                    LocationId::Cluster(id) => id,
                    LocationId::ExternalProcess(id) => id,
                };

                let receiver_builder = graph_builders.entry(*to_id).or_default();
                let receiver_stream_id = *next_stmt_id;
                *next_stmt_id += 1;

                let receiver_stream_ident =
                    syn::Ident::new(&format!("stream_{}", receiver_stream_id), Span::call_site());

                if let Some(deserialize_pipeline) = deserialize_pipeline {
                    receiver_builder.add_statement(parse_quote! {
                        #receiver_stream_ident = source_stream(#source_expr) -> #deserialize_pipeline;
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

#[expect(clippy::too_many_arguments, reason = "networking internals")]
fn instantiate_network<'a, D: Deploy<'a> + 'a>(
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
    };
    (sink, source, connect_fn)
}
