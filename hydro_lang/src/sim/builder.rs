use std::collections::BTreeMap;

use dfir_lang::graph::FlatGraphBuilder;
use proc_macro2::Span;
use quote::ToTokens;
use syn::parse_quote;

use crate::compile::ir::{
    CollectionKind, DebugExpr, DfirBuilder, HydroIrOpMetadata, StreamOrder, StreamRetry,
};
use crate::location::dynamic::LocationId;

/// A builder for DFIR graphs used in simulations.
///
/// Instead of emitting one DFIR graph per location, we emit one big DFIR graph in `async_level`,
/// which contains all asynchronously executed top-level operators in the Hydro program. Because
/// "top-level" operators guarantee "eventual determinism" (per Flo), we do not need to simulate
/// every possible interleaving of message arrivals and processing. Instead, we only need to
/// simulate sources of non-determinism at the points in the program where a user intentionally
/// observes them (such as batch or assume_ordering).
///
/// Because each tick relies on a set of decisions being made to select their inputs (batch,
/// snapshot), we emit each tick's code into a separate DFIR graph. Each non-deterministic input
/// to a tick has a corresponding "hook" that the simulation runtime can use to control the
/// non-deterministic decision made at that boundary. This hook interacts with the DFIR program
/// by accumulating inputs from the async level into a buffer, and then the hook can send selected
/// elements from that buffer into the tick's DFIR graph with a separate handoff channel.
pub struct SimBuilder {
    pub extra_stmts_global: Vec<syn::Stmt>,
    pub extra_stmts_cluster: BTreeMap<LocationId, Vec<syn::Stmt>>,
    pub process_graphs: BTreeMap<LocationId, FlatGraphBuilder>,
    pub cluster_graphs: BTreeMap<LocationId, FlatGraphBuilder>,
    pub process_tick_dfirs: BTreeMap<LocationId, FlatGraphBuilder>,
    pub cluster_tick_dfirs: BTreeMap<LocationId, FlatGraphBuilder>,
    pub next_hoff_id: usize,
}

impl SimBuilder {
    fn add_extra_stmt_internal(&mut self, location: &LocationId, stmt: syn::Stmt) {
        match location {
            LocationId::Process(_) => {
                self.extra_stmts_global.push(stmt);
            }
            LocationId::Cluster(_) => {
                self.extra_stmts_cluster
                    .entry(location.clone())
                    .or_default()
                    .push(stmt);
            }
            _ => unreachable!(),
        }
    }

    fn add_hook(&mut self, in_location: &LocationId, out_location: &LocationId, expr: syn::Expr) {
        let out_location_ser = serde_json::to_string(out_location).unwrap();
        match in_location {
            LocationId::Process(_) => {
                self.add_extra_stmt_internal(
                    in_location,
                    syn::parse_quote! {
                        __hydro_hooks.entry((#out_location_ser, None)).or_default().push(#expr);
                    },
                );
            }
            LocationId::Cluster(_) => {
                self.add_extra_stmt_internal(in_location, syn::parse_quote! {
                    __hydro_hooks.entry((#out_location_ser, Some(__current_cluster_id))).or_default().push(#expr);
                });
            }
            _ => unreachable!(),
        }
    }

    fn add_inline_hook(&mut self, tick_location: &LocationId, expr: syn::Expr) {
        let tick_location_ser = serde_json::to_string(tick_location).unwrap();
        match tick_location {
            LocationId::Tick(_, l) => match l.root() {
                LocationId::Process(_) => {
                    self.add_extra_stmt_internal(
                        l.root(),
                        syn::parse_quote! {
                            __hydro_inline_hooks.entry((#tick_location_ser, None)).or_default().push(#expr);
                        },
                    );
                }
                LocationId::Cluster(_) => {
                    self.add_extra_stmt_internal(l.root(), syn::parse_quote! {
                        __hydro_inline_hooks.entry((#tick_location_ser, Some(__current_cluster_id))).or_default().push(#expr);
                    });
                }
                _ => unreachable!(),
            },
            _ => unreachable!(),
        }
    }
}

impl DfirBuilder for SimBuilder {
    fn singleton_intermediates(&self) -> bool {
        true
    }

    fn get_dfir_mut(&mut self, location: &LocationId) -> &mut FlatGraphBuilder {
        match location {
            LocationId::Process(_) => self.process_graphs.entry(location.clone()).or_default(),
            LocationId::Cluster(_) => self.cluster_graphs.entry(location.clone()).or_default(),
            LocationId::Atomic(tick) => self.get_dfir_mut(tick.as_ref()),
            LocationId::Tick(_, l) => match l.root() {
                LocationId::Process(_) => {
                    self.process_tick_dfirs.entry(location.clone()).or_default()
                }
                LocationId::Cluster(_) => {
                    self.cluster_tick_dfirs.entry(location.clone()).or_default()
                }
                _ => unreachable!(),
            },
        }
    }

    fn batch(
        &mut self,
        in_ident: syn::Ident,
        in_location: &LocationId,
        in_kind: &CollectionKind,
        out_ident: &syn::Ident,
        out_location: &LocationId,
        op_meta: &HydroIrOpMetadata,
    ) {
        if let LocationId::Atomic(_) = in_location {
            let builder = self.get_dfir_mut(in_location);
            builder.add_dfir(
                parse_quote! {
                    #out_ident = #in_ident;
                },
                None,
                None,
            );
        } else {
            let out_location = if let LocationId::Atomic(tick) = out_location {
                tick.as_ref()
            } else {
                out_location
            };

            let (batch_location, line, caret) = location_for_op(op_meta);

            match in_kind {
                CollectionKind::Stream {
                    order,
                    retry: StreamRetry::ExactlyOnce,
                    ..
                } => {
                    debug_assert!(in_location.is_top_level());

                    let order_ty: syn::Type = match order {
                        StreamOrder::TotalOrder => {
                            parse_quote! { hydro_lang::live_collections::stream::TotalOrder }
                        }
                        StreamOrder::NoOrder => {
                            parse_quote! { hydro_lang::live_collections::stream::NoOrder }
                        }
                    };

                    let hoff_id = self.next_hoff_id;
                    self.next_hoff_id += 1;

                    let buffered_ident =
                        syn::Ident::new(&format!("__buffered_{hoff_id}"), Span::call_site());
                    let hoff_send_ident =
                        syn::Ident::new(&format!("__hoff_send_{hoff_id}"), Span::call_site());
                    let hoff_recv_ident =
                        syn::Ident::new(&format!("__hoff_recv_{hoff_id}"), Span::call_site());

                    self.add_extra_stmt_internal(in_location, syn::parse_quote! {
                        let (#hoff_send_ident, #hoff_recv_ident) = __root_dfir_rs::util::unbounded_channel();
                    });
                    self.add_extra_stmt_internal(in_location, syn::parse_quote! {
                        let #buffered_ident = ::std::rc::Rc::new(::std::cell::RefCell::new(::std::collections::VecDeque::new()));
                    });
                    self.add_hook(
                        in_location,
                        out_location,
                        syn::parse_quote!(
                            Box::new(hydro_lang::sim::runtime::StreamHook::<_, #order_ty> {
                                input: #buffered_ident.clone(),
                                to_release: None,
                                output: #hoff_send_ident,
                                batch_location: (#batch_location, #line, #caret),
                                format_item_debug: ::hydro_lang::__maybe_debug__!(),
                                _order: std::marker::PhantomData,
                            })
                        ),
                    );

                    self.get_dfir_mut(in_location).add_dfir(
                        parse_quote! {
                            #in_ident -> for_each(|v| #buffered_ident.borrow_mut().push_back(v));
                        },
                        None,
                        None,
                    );

                    self.get_dfir_mut(out_location).add_dfir(
                        parse_quote! {
                            #out_ident = source_stream(#hoff_recv_ident);
                        },
                        None,
                        None,
                    );
                }
                CollectionKind::KeyedStream {
                    value_order,
                    value_retry: StreamRetry::ExactlyOnce,
                    ..
                } => {
                    debug_assert!(in_location.is_top_level());

                    let order_ty: syn::Type = match value_order {
                        StreamOrder::TotalOrder => {
                            parse_quote! { hydro_lang::live_collections::stream::TotalOrder }
                        }
                        StreamOrder::NoOrder => {
                            parse_quote! { hydro_lang::live_collections::stream::NoOrder }
                        }
                    };

                    let hoff_id = self.next_hoff_id;
                    self.next_hoff_id += 1;

                    let buffered_ident =
                        syn::Ident::new(&format!("__buffered_{hoff_id}"), Span::call_site());
                    let hoff_send_ident =
                        syn::Ident::new(&format!("__hoff_send_{hoff_id}"), Span::call_site());
                    let hoff_recv_ident =
                        syn::Ident::new(&format!("__hoff_recv_{hoff_id}"), Span::call_site());

                    self.add_extra_stmt_internal(in_location, syn::parse_quote! {
                        let (#hoff_send_ident, #hoff_recv_ident) = __root_dfir_rs::util::unbounded_channel();
                    });
                    self.add_extra_stmt_internal(in_location, syn::parse_quote! {
                        let #buffered_ident = ::std::rc::Rc::new(::std::cell::RefCell::new(__root_dfir_rs::rustc_hash::FxHashMap::<_, ::std::collections::VecDeque<_>>::default()));
                    });
                    self.add_hook(
                        in_location,
                        out_location,
                        syn::parse_quote!(
                            Box::new(hydro_lang::sim::runtime::KeyedStreamHook::<_, _, #order_ty> {
                                input: #buffered_ident.clone(),
                                to_release: None,
                                output: #hoff_send_ident,
                                batch_location: (#batch_location, #line, #caret),
                                format_item_debug: ::hydro_lang::__maybe_debug__!(),
                                _order: std::marker::PhantomData,
                            })
                        ),
                    );

                    self.get_dfir_mut(in_location).add_dfir(
                        parse_quote! {
                            #in_ident -> for_each(|(k, v)| #buffered_ident.borrow_mut().entry(k).or_default().push_back(v));
                        },
                        None,
                        None,
                    );

                    self.get_dfir_mut(out_location).add_dfir(
                        parse_quote! {
                            #out_ident = source_stream(#hoff_recv_ident);
                        },
                        None,
                        None,
                    );
                }
                CollectionKind::Singleton { .. } => {
                    debug_assert!(in_location.is_top_level());

                    let hoff_id = self.next_hoff_id;
                    self.next_hoff_id += 1;

                    let buffered_ident =
                        syn::Ident::new(&format!("__buffered_{hoff_id}"), Span::call_site());
                    let hoff_send_ident =
                        syn::Ident::new(&format!("__hoff_send_{hoff_id}"), Span::call_site());
                    let hoff_recv_ident =
                        syn::Ident::new(&format!("__hoff_recv_{hoff_id}"), Span::call_site());

                    self.add_extra_stmt_internal(in_location, syn::parse_quote! {
                        let (#hoff_send_ident, #hoff_recv_ident) = __root_dfir_rs::util::unbounded_channel();
                    });
                    self.add_extra_stmt_internal(in_location, syn::parse_quote! {
                        let #buffered_ident = ::std::rc::Rc::new(::std::cell::RefCell::new(::std::collections::VecDeque::new()));
                    });
                    self.add_hook(
                        in_location,
                        out_location,
                        syn::parse_quote! (
                            Box::new(hydro_lang::sim::runtime::SingletonHook::<_>::new(
                                #buffered_ident.clone(),
                                #hoff_send_ident,
                                (#batch_location, #line, #caret),
                                ::hydro_lang::__maybe_debug__!(),
                            ))
                        ),
                    );

                    self.get_dfir_mut(in_location).add_dfir(
                        parse_quote! {
                            #in_ident -> for_each(|v| #buffered_ident.borrow_mut().push_back(v));
                        },
                        None,
                        None,
                    );

                    self.get_dfir_mut(out_location).add_dfir(
                        parse_quote! {
                            #out_ident = source_stream(#hoff_recv_ident);
                        },
                        None,
                        None,
                    );
                }
                CollectionKind::KeyedSingleton { .. } => {
                    debug_assert!(in_location.is_top_level());

                    let hoff_id = self.next_hoff_id;
                    self.next_hoff_id += 1;

                    let buffered_ident =
                        syn::Ident::new(&format!("__buffered_{hoff_id}"), Span::call_site());
                    let hoff_send_ident =
                        syn::Ident::new(&format!("__hoff_send_{hoff_id}"), Span::call_site());
                    let hoff_recv_ident =
                        syn::Ident::new(&format!("__hoff_recv_{hoff_id}"), Span::call_site());

                    self.add_extra_stmt_internal(in_location, syn::parse_quote! {
                        let (#hoff_send_ident, #hoff_recv_ident) = __root_dfir_rs::util::unbounded_channel();
                    });
                    self.add_extra_stmt_internal(in_location, syn::parse_quote! {
                        let #buffered_ident = ::std::rc::Rc::new(::std::cell::RefCell::new(__root_dfir_rs::rustc_hash::FxHashMap::<_, ::std::collections::VecDeque<_>>::default()));
                    });
                    self.add_hook(
                        in_location,
                        out_location,
                        syn::parse_quote! (
                            Box::new(hydro_lang::sim::runtime::KeyedSingletonHook::<_, _>::new(
                                #buffered_ident.clone(),
                                #hoff_send_ident,
                                (#batch_location, #line, #caret),
                                ::hydro_lang::__maybe_debug__!(),
                                ::hydro_lang::__maybe_debug__!(),
                            ))
                        ),
                    );

                    self.get_dfir_mut(in_location).add_dfir(
                        parse_quote! {
                            #in_ident -> for_each(|(k, v)| #buffered_ident.borrow_mut().entry(k).or_default().push_back(v));
                        },
                        None,
                        None,
                    );

                    self.get_dfir_mut(out_location).add_dfir(
                        parse_quote! {
                            #out_ident = source_stream(#hoff_recv_ident);
                        },
                        None,
                        None,
                    );
                }
                _ => {
                    eprintln!("{:?}", op_meta.backtrace.elements());
                    todo!("batch not implemented for kind {:?}", in_kind)
                }
            }
        }
    }

    fn yield_from_tick(
        &mut self,
        in_ident: syn::Ident,
        in_location: &LocationId,
        in_kind: &CollectionKind,
        out_ident: &syn::Ident,
        out_location: &LocationId,
    ) {
        match in_kind {
            CollectionKind::Stream { .. }
            | CollectionKind::KeyedStream { .. }
            | CollectionKind::Singleton { .. } => {
                debug_assert!(out_location.is_top_level());
                if let LocationId::Atomic(t) = out_location {
                    if t.as_ref() == in_location {
                        self.get_dfir_mut(out_location).add_dfir(
                            parse_quote! {
                                #out_ident = #in_ident;
                            },
                            None,
                            None,
                        );
                    } else {
                        todo!("atomic yield to a different tick is not yet supported");
                    }
                } else {
                    let hoff_id = self.next_hoff_id;
                    self.next_hoff_id += 1;

                    let hoff_send_ident =
                        syn::Ident::new(&format!("__hoff_send_{hoff_id}"), Span::call_site());
                    let hoff_recv_ident =
                        syn::Ident::new(&format!("__hoff_recv_{hoff_id}"), Span::call_site());

                    self.add_extra_stmt_internal(out_location, syn::parse_quote! {
                        let (#hoff_send_ident, #hoff_recv_ident) = __root_dfir_rs::util::unbounded_channel();
                    });

                    self.get_dfir_mut(in_location).add_dfir(
                        parse_quote! {
                            #in_ident -> for_each(|v| #hoff_send_ident.send(v).unwrap());
                        },
                        None,
                        None,
                    );

                    self.get_dfir_mut(out_location).add_dfir(
                        parse_quote! {
                            #out_ident = source_stream(#hoff_recv_ident);
                        },
                        None,
                        None,
                    );
                }
            }
            CollectionKind::Optional { .. } => {
                debug_assert!(out_location.is_top_level());
                if let LocationId::Atomic(t) = out_location {
                    if t.as_ref() == in_location {
                        self.get_dfir_mut(out_location).add_dfir(
                            parse_quote! {
                                #out_ident = #in_ident;
                            },
                            None,
                            None,
                        );
                    } else {
                        todo!("atomic yield to a different tick is not yet supported");
                    }
                } else {
                    todo!("Non-atomic yield of an Optional is not yet supported");
                }
            }
            o => todo!("Not yet supported, yield collection type {:?}", o),
        }
    }

    fn begin_atomic(
        &mut self,
        in_ident: syn::Ident,
        in_location: &LocationId,
        in_kind: &CollectionKind,
        out_ident: &syn::Ident,
        out_location: &LocationId,
        op_meta: &HydroIrOpMetadata,
    ) {
        self.batch(
            in_ident,
            in_location,
            in_kind,
            out_ident,
            out_location,
            op_meta,
        );
    }

    fn end_atomic(
        &mut self,
        in_ident: syn::Ident,
        in_location: &LocationId,
        in_kind: &CollectionKind,
        out_ident: &syn::Ident,
    ) {
        if let LocationId::Atomic(tick) = in_location
            && let LocationId::Tick(_, outer) = tick.as_ref()
        {
            self.yield_from_tick(in_ident, in_location, in_kind, out_ident, outer.as_ref());
        } else {
            unreachable!()
        }
    }

    fn observe_nondet(
        &mut self,
        trusted: bool,
        location: &LocationId,
        in_ident: syn::Ident,
        in_kind: &CollectionKind,
        out_ident: &syn::Ident,
        out_kind: &CollectionKind,
        op_meta: &HydroIrOpMetadata,
    ) {
        if trusted {
            let builder = self.get_dfir_mut(location);
            builder.add_dfir(
                parse_quote! {
                    #out_ident = #in_ident;
                },
                None,
                None,
            );
        } else if !location.is_top_level() {
            let (assume_location, line, caret) = location_for_op(op_meta);
            match (in_kind, out_kind) {
                (
                    CollectionKind::Stream {
                        order: StreamOrder::NoOrder,
                        retry: StreamRetry::ExactlyOnce,
                        ..
                    },
                    CollectionKind::Stream {
                        order: StreamOrder::TotalOrder,
                        retry: StreamRetry::ExactlyOnce,
                        ..
                    },
                ) => {
                    let hoff_id = self.next_hoff_id;
                    self.next_hoff_id += 1;

                    let buffered_ident =
                        syn::Ident::new(&format!("__buffered_{hoff_id}"), Span::call_site());
                    let hoff_send_ident =
                        syn::Ident::new(&format!("__hoff_send_{hoff_id}"), Span::call_site());
                    let hoff_recv_ident =
                        syn::Ident::new(&format!("__hoff_recv_{hoff_id}"), Span::call_site());

                    self.add_extra_stmt_internal(location.root(), syn::parse_quote! {
                        let (#hoff_send_ident, #hoff_recv_ident) = __root_dfir_rs::util::unbounded_channel();
                    });

                    self.add_extra_stmt_internal(location.root(), syn::parse_quote! {
                        let #hoff_recv_ident = ::std::rc::Rc::new(::std::cell::RefCell::new(#hoff_recv_ident.into_inner()));
                    });

                    self.add_extra_stmt_internal(location.root(), syn::parse_quote! {
                        let #buffered_ident = ::std::rc::Rc::new(::std::cell::RefCell::new(None));
                    });

                    self.add_inline_hook(
                        location,
                        syn::parse_quote!(
                            Box::new(hydro_lang::sim::runtime::StreamOrderHook::<_>::new(
                                #buffered_ident.clone(),
                                #hoff_send_ident,
                                (#assume_location, #line, #caret),
                                ::hydro_lang::__maybe_debug__!(),
                            ))
                        ),
                    );

                    let builder = self.get_dfir_mut(location);
                    builder.add_dfir(
                        parse_quote! {
                            #out_ident = #in_ident -> fold::<'tick>(
                                || ::std::vec::Vec::new(),
                                |acc, v| {
                                    acc.push(v);
                                }
                            ) -> map(|v| {
                                let #buffered_ident = #buffered_ident.clone();
                                let #hoff_recv_ident = #hoff_recv_ident.clone();
                                async move {
                                    fn force_matching_type<T>(a: &mut Option<::std::vec::Vec<T>>, b: ::std::vec::Vec<T>) -> ::std::vec::Vec<T> {
                                        b
                                    }

                                    let mut out_holder = Some(v);
                                    *#buffered_ident.borrow_mut() = out_holder.take();
                                    force_matching_type(&mut out_holder, #hoff_recv_ident.borrow_mut().recv().await.unwrap())
                                }
                            }) -> resolve_futures_blocking() -> flatten();
                        },
                        None,
                        None,
                    );
                }
                (
                    CollectionKind::KeyedStream {
                        value_order: StreamOrder::NoOrder,
                        value_retry: StreamRetry::ExactlyOnce,
                        ..
                    },
                    CollectionKind::KeyedStream {
                        value_order: StreamOrder::TotalOrder,
                        value_retry: StreamRetry::ExactlyOnce,
                        ..
                    },
                ) => {
                    let hoff_id = self.next_hoff_id;
                    self.next_hoff_id += 1;

                    let buffered_ident =
                        syn::Ident::new(&format!("__buffered_{hoff_id}"), Span::call_site());
                    let hoff_send_ident =
                        syn::Ident::new(&format!("__hoff_send_{hoff_id}"), Span::call_site());
                    let hoff_recv_ident =
                        syn::Ident::new(&format!("__hoff_recv_{hoff_id}"), Span::call_site());

                    self.add_extra_stmt_internal(location.root(), syn::parse_quote! {
                        let (#hoff_send_ident, #hoff_recv_ident) = __root_dfir_rs::util::unbounded_channel();
                    });

                    self.add_extra_stmt_internal(location.root(), syn::parse_quote! {
                        let #hoff_recv_ident = ::std::rc::Rc::new(::std::cell::RefCell::new(#hoff_recv_ident.into_inner()));
                    });

                    self.add_extra_stmt_internal(location.root(), syn::parse_quote! {
                        let #buffered_ident = ::std::rc::Rc::new(::std::cell::RefCell::new(None));
                    });

                    self.add_inline_hook(
                        location,
                        syn::parse_quote!(
                            Box::new(hydro_lang::sim::runtime::KeyedStreamOrderHook::<_, _>::new(
                                #buffered_ident.clone(),
                                #hoff_send_ident,
                                (#assume_location, #line, #caret),
                                ::hydro_lang::__maybe_debug__!(),
                                ::hydro_lang::__maybe_debug__!(),
                            ))
                        ),
                    );

                    let builder = self.get_dfir_mut(location);
                    builder.add_dfir(
                        parse_quote! {
                            #out_ident = #in_ident -> fold::<'tick>(
                                || ::std::vec::Vec::new(),
                                |acc, v| {
                                    acc.push(v);
                                }
                            ) -> map(|v| {
                                let #buffered_ident = #buffered_ident.clone();
                                let #hoff_recv_ident = #hoff_recv_ident.clone();
                                async move {
                                    fn force_matching_type<T>(a: &mut Option<::std::vec::Vec<T>>, b: ::std::vec::Vec<T>) -> ::std::vec::Vec<T> {
                                        b
                                    }

                                    let mut out_holder = Some(v);
                                    *#buffered_ident.borrow_mut() = out_holder.take();
                                    force_matching_type(&mut out_holder, #hoff_recv_ident.borrow_mut().recv().await.unwrap())
                                }
                            }) -> resolve_futures_blocking() -> flatten();
                        },
                        None,
                        None,
                    );
                }
                _ => {
                    todo!(
                        "non-trusted observe_nondet not yet supported for kinds {:?} -> {:?}",
                        in_kind,
                        out_kind
                    );
                }
            }
        } else {
            todo!("non-trusted observe_nondet not yet supported at top-level locations");
        }
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
        match (from, to) {
            (LocationId::Process(_), LocationId::Process(_)) => {
                self.extra_stmts_global.push(syn::parse_quote! {
                    let (#sink, #source) = __root_dfir_rs::util::unbounded_channel::<__root_dfir_rs::bytes::Bytes>();
                });

                if let Some(serialize_pipeline) = serialize {
                    self.get_dfir_mut(from).add_dfir(
                        parse_quote! {
                            #input_ident -> map(#serialize_pipeline) -> for_each(|v| #sink.send(v).unwrap());
                        },
                        None,
                        Some(&format!("send{}", tag_id)),
                    );
                } else {
                    self.get_dfir_mut(from).add_dfir(
                        parse_quote! {
                            #input_ident -> for_each(|v| #sink.send(v).unwrap());
                        },
                        None,
                        Some(&format!("send{}", tag_id)),
                    );
                }

                if let Some(deserialize_pipeline) = deserialize {
                    self.get_dfir_mut(to).add_dfir(
                        parse_quote! {
                            #out_ident = source_stream(#source) -> map(|v| -> ::std::result::Result<_, ()> { Ok(v) }) -> map(#deserialize_pipeline);
                        },
                        None,
                        Some(&format!("recv{}", tag_id)),
                    );
                } else {
                    self.get_dfir_mut(to).add_dfir(
                        parse_quote! {
                            #out_ident = source_stream(#source);
                        },
                        None,
                        Some(&format!("recv{}", tag_id)),
                    );
                }
            }
            (LocationId::Cluster(_), LocationId::Process(_)) => {
                self.extra_stmts_global.push(syn::parse_quote! {
                    let (#sink, #source) = __root_dfir_rs::util::unbounded_channel::<(::hydro_lang::location::TaglessMemberId, __root_dfir_rs::bytes::Bytes)>();
                });

                self.extra_stmts_cluster
                    .entry(from.clone())
                    .or_default()
                    .push(syn::parse_quote! {
                        let #sink = #sink.clone();
                    });

                if let Some(serialize_pipeline) = serialize {
                    self.get_dfir_mut(from).add_dfir(
                        parse_quote! {
                            #input_ident -> map(#serialize_pipeline) -> for_each(|v| #sink.send((::hydro_lang::location::TaglessMemberId::from_raw_id(__current_cluster_id), v)).unwrap());
                        },
                        None,
                        Some(&format!("send{}", tag_id)),
                    );
                } else {
                    self.get_dfir_mut(from).add_dfir(
                        parse_quote! {
                            #input_ident -> for_each(|v| #sink.send((::hydro_lang::location::TaglessMemberId::from_raw_id(__current_cluster_id), v)).unwrap());
                        },
                        None,
                        Some(&format!("send{}", tag_id)),
                    );
                }

                if let Some(deserialize_pipeline) = deserialize {
                    self.get_dfir_mut(to).add_dfir(
                        parse_quote! {
                            #out_ident = source_stream(#source) -> map(|v| -> ::std::result::Result<_, ()> { Ok(v) }) -> map(#deserialize_pipeline);
                        },
                        None,
                        Some(&format!("recv{}", tag_id)),
                    );
                } else {
                    self.get_dfir_mut(to).add_dfir(
                        parse_quote! {
                            #out_ident = source_stream(#source);
                        },
                        None,
                        Some(&format!("recv{}", tag_id)),
                    );
                }
            }
            (LocationId::Process(_), LocationId::Cluster(_)) => {
                let sink_writer = syn::Ident::new(
                    &format!("__cloned_{}", sink.to_token_stream()),
                    Span::call_site(),
                );
                self.extra_stmts_global.push(syn::parse_quote! {
                    let #sink: ::std::rc::Rc<::std::cell::RefCell<Vec<__root_dfir_rs::tokio::sync::mpsc::UnboundedSender<__root_dfir_rs::bytes::Bytes>>>> = ::std::rc::Rc::new(::std::cell::RefCell::new(Vec::new()));
                });

                self.extra_stmts_global.push(syn::parse_quote! {
                    let #sink_writer = #sink.clone();
                });

                self.extra_stmts_cluster
                    .entry(to.clone())
                    .or_default()
                    .push(syn::parse_quote! {
                        let #source = {
                            let (__sink, __source) = __root_dfir_rs::util::unbounded_channel::<__root_dfir_rs::bytes::Bytes>();
                            #sink_writer.borrow_mut().push(__sink);
                            __source
                        };
                    });

                if let Some(serialize_pipeline) = serialize {
                    self.get_dfir_mut(from).add_dfir(
                        parse_quote! {
                            #input_ident -> map(#serialize_pipeline) -> for_each(|(target_member_id, v)| (#sink.borrow())[::hydro_lang::location::TaglessMemberId::get_raw_id(&target_member_id) as usize].send(v).unwrap());
                        },
                        None,
                        Some(&format!("send{}", tag_id)),
                    );
                } else {
                    self.get_dfir_mut(from).add_dfir(
                        parse_quote! {
                            #input_ident -> for_each(|(target_member_id, v)| (#sink.borrow())[::hydro_lang::location::TaglessMemberId::get_raw_id(&target_member_id) as usize].send(v).unwrap());
                        },
                        None,
                        Some(&format!("send{}", tag_id)),
                    );
                }

                if let Some(deserialize_pipeline) = deserialize {
                    self.get_dfir_mut(to).add_dfir(
                        parse_quote! {
                            #out_ident = source_stream(#source) -> map(|v| -> ::std::result::Result<_, ()> { Ok(v) }) -> map(#deserialize_pipeline);
                        },
                        None,
                        Some(&format!("recv{}", tag_id)),
                    );
                } else {
                    self.get_dfir_mut(to).add_dfir(
                        parse_quote! {
                            #out_ident = source_stream(#source);
                        },
                        None,
                        Some(&format!("recv{}", tag_id)),
                    );
                }
            }
            (LocationId::Cluster(_), LocationId::Cluster(_)) => {
                let sink_writer = syn::Ident::new(
                    &format!("__cloned_{}", sink.to_token_stream()),
                    Span::call_site(),
                );
                self.extra_stmts_global.push(syn::parse_quote! {
                    let #sink: ::std::rc::Rc<::std::cell::RefCell<Vec<__root_dfir_rs::tokio::sync::mpsc::UnboundedSender<(::hydro_lang::location::TaglessMemberId, __root_dfir_rs::bytes::Bytes)>>>> = ::std::rc::Rc::new(::std::cell::RefCell::new(Vec::new()));
                });

                self.extra_stmts_global.push(syn::parse_quote! {
                    let #sink_writer = #sink.clone();
                });

                self.extra_stmts_cluster
                    .entry(from.clone())
                    .or_default()
                    .push(syn::parse_quote! {
                        let #sink = #sink.clone();
                    });

                self.extra_stmts_cluster
                    .entry(to.clone())
                    .or_default()
                    .push(syn::parse_quote! {
                        let #source = {
                            let (__sink, __source) = __root_dfir_rs::util::unbounded_channel::<(::hydro_lang::location::TaglessMemberId, __root_dfir_rs::bytes::Bytes)>();
                            #sink_writer.borrow_mut().push(__sink);
                            __source
                        };
                    });

                if let Some(serialize_pipeline) = serialize {
                    self.get_dfir_mut(from).add_dfir(
                        parse_quote! {
                            #input_ident -> map(#serialize_pipeline) -> for_each(|(target_member_id, v)| (#sink.borrow())[::hydro_lang::location::TaglessMemberId::get_raw_id(&target_member_id) as usize].send((::hydro_lang::location::TaglessMemberId::from_raw_id(__current_cluster_id), v)).unwrap());
                        },
                        None,
                        Some(&format!("send{}", tag_id)),
                    );
                } else {
                    self.get_dfir_mut(from).add_dfir(
                        parse_quote! {
                            #input_ident -> for_each(|(target_member_id, v)| (#sink.borrow())[::hydro_lang::location::TaglessMemberId::get_raw_id(&target_member_id) as usize].send((::hydro_lang::location::TaglessMemberId::from_raw_id(__current_cluster_id), v)).unwrap());
                        },
                        None,
                        Some(&format!("send{}", tag_id)),
                    );
                }

                if let Some(deserialize_pipeline) = deserialize {
                    self.get_dfir_mut(to).add_dfir(
                        parse_quote! {
                            #out_ident = source_stream(#source) -> map(|v| -> ::std::result::Result<_, ()> { Ok(v) }) -> map(#deserialize_pipeline);
                        },
                        None,
                        Some(&format!("recv{}", tag_id)),
                    );
                } else {
                    self.get_dfir_mut(to).add_dfir(
                        parse_quote! {
                            #out_ident = source_stream(#source);
                        },
                        None,
                        Some(&format!("recv{}", tag_id)),
                    );
                }
            }
            _ => {
                panic!(
                    "Simulations do not yet support network between {:?} and {:?}",
                    from, to
                );
            }
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
        if let Some(deserialize_pipeline) = deserialize {
            self.get_dfir_mut(on).add_dfir(
                parse_quote! {
                    #out_ident = source_stream(#source_expr) -> map(|v| -> ::std::result::Result<_, ()> { Ok(v) }) -> map(#deserialize_pipeline);
                },
                None,
                Some(&format!("recv{}", tag_id)),
            );
        } else {
            self.get_dfir_mut(on).add_dfir(
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
        let grabbed_ident = syn::Ident::new(&format!("__sink_{tag_id}"), Span::call_site());
        self.extra_stmts_global.push(syn::parse_quote! {
            let #grabbed_ident = #sink_expr;
        });

        if let Some(serialize_pipeline) = serialize {
            self.get_dfir_mut(on).add_dfir(
                parse_quote! {
                    #input_ident -> map(#serialize_pipeline) -> for_each(|v| #grabbed_ident.send(v).unwrap());
                },
                None,
                Some(&format!("send{}", tag_id)),
            );
        } else {
            self.get_dfir_mut(on).add_dfir(
                parse_quote! {
                    #input_ident -> for_each(|v| #grabbed_ident.send(v).unwrap());
                },
                None,
                Some(&format!("send{}", tag_id)),
            );
        }
    }
}

/// Extract a location string, line, and caret indent from an op's metadata backtrace.
///
/// The return type mirrors `HookLocationMeta`, but with owned `String` that will be inlined
/// into the generated sources.
fn location_for_op(op_meta: &HydroIrOpMetadata) -> (String, String, String) {
    op_meta
        .backtrace
        .elements()
        .first()
        .and_then(|e| {
            let filename = e.filename.as_deref()?;
            let lineno = e.lineno?;
            let colno = e.colno?;

            let line = std::fs::read_to_string(filename)
                .ok()
                .and_then(|s| {
                    s.lines()
                        .nth(lineno.saturating_sub(1).try_into().unwrap())
                        .map(|s| s.to_string())
                })
                .unwrap_or_default();

            let relative_path = (|| {
                std::path::Path::new(filename)
                    .strip_prefix(std::env::current_dir().ok()?)
                    .ok()
            })();

            let filename_display = relative_path
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| filename.to_string());

            Some((
                format!("{}:{}:{}", filename_display, lineno, colno),
                line,
                format!("{:>1$}", "", (colno - 1).try_into().unwrap()),
            ))
        })
        .unwrap_or_else(|| {
            (
                "unknown location".to_string(),
                "".to_string(),
                "".to_string(),
            )
        })
}
