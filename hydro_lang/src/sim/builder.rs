use std::collections::{BTreeMap, HashSet};

use dfir_lang::graph::FlatGraphBuilder;
use proc_macro2::Span;
use quote::ToTokens;
use syn::parse_quote;

use crate::compile::builder::{HandoffId, StmtId};
use crate::compile::ir::{
    CollectionKind, DebugExpr, DfirBuilder, HydroIrOpMetadata, KeyedSingletonBoundKind,
    StreamOrder, StreamRetry,
};
use crate::location::dynamic::LocationId;
use crate::staging_util::get_this_crate;

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
    pub next_hoff_id: crate::Counter<HandoffId>,
    pub test_safety_only: bool,
    pub skip_consistency_assertions: bool,
    pub channel_tables: BTreeMap<u32, syn::Ident>,
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

    fn channel_elem_ty(from: &LocationId, root: &proc_macro2::TokenStream) -> syn::Type {
        if matches!(from, LocationId::Cluster(_)) {
            syn::parse_quote!((#root::__staged::location::TaglessMemberId, __root_dfir_rs::bytes::Bytes))
        } else {
            syn::parse_quote!(__root_dfir_rs::bytes::Bytes)
        }
    }

    fn channel_table_ident(&mut self, channel_id: u32, elem_ty: &syn::Type) -> syn::Ident {
        if let Some(ident) = self.channel_tables.get(&channel_id) {
            return ident.clone();
        }
        let ident = syn::Ident::new(
            &format!("__hydro_channel_{}", channel_id),
            Span::call_site(),
        );
        self.extra_stmts_global.push(syn::parse_quote! {
            let #ident: ::std::rc::Rc<::std::cell::RefCell<::std::collections::HashMap<u32, __root_dfir_rs::tokio::sync::mpsc::UnboundedSender<#elem_ty>>>> =
                ::std::rc::Rc::new(::std::cell::RefCell::new(::std::collections::HashMap::new()));
        });
        self.channel_tables.insert(channel_id, ident.clone());
        ident
    }

    #[expect(clippy::too_many_arguments, reason = "code generation")]
    fn emit_channel_send_half(
        &mut self,
        from: &LocationId,
        to: &LocationId,
        input_ident: syn::Ident,
        serialize: Option<&DebugExpr>,
        suffix: &str,
        channel_id: u32,
        root: &proc_macro2::TokenStream,
    ) {
        let from_is_cluster = matches!(from, LocationId::Cluster(_));
        let to_is_cluster = matches!(to, LocationId::Cluster(_));
        let elem_ty = Self::channel_elem_ty(from, root);
        let table = self.channel_table_ident(channel_id, &elem_ty);
        let send_table = syn::Ident::new(&format!("__channel_send_{suffix}"), Span::call_site());

        let dest_expr: syn::Expr = if to_is_cluster {
            syn::parse_quote!(#root::__staged::location::TaglessMemberId::get_raw_id(&target_member_id))
        } else {
            syn::parse_quote!(0u32)
        };
        let payload_expr: syn::Expr = if from_is_cluster {
            syn::parse_quote!((#root::__staged::location::TaglessMemberId::from_raw_id(__current_cluster_id), v))
        } else {
            syn::parse_quote!(v)
        };
        let send_pat: syn::Pat = if to_is_cluster {
            syn::parse_quote!((target_member_id, v))
        } else {
            syn::parse_quote!(v)
        };

        if from_is_cluster {
            self.extra_stmts_cluster
                .entry(from.clone())
                .or_default()
                .push(syn::parse_quote! {
                    let #send_table = #table.clone();
                });
        } else {
            self.extra_stmts_global.push(syn::parse_quote! {
                let #send_table = #table.clone();
            });
        }

        let send_body: syn::Expr = syn::parse_quote! {
            {
                if let Some(__s) = #send_table.borrow().get(&#dest_expr) {
                    let _ = __s.send(#payload_expr);
                }
            }
        };
        if let Some(serialize_pipeline) = serialize {
            self.get_dfir_mut(from).add_dfir(
                parse_quote! {
                    #input_ident -> map(#serialize_pipeline) -> for_each(|#send_pat| #send_body);
                },
                None,
                Some(&format!("send{}", suffix)),
            );
        } else {
            self.get_dfir_mut(from).add_dfir(
                parse_quote! {
                    #input_ident -> for_each(|#send_pat| #send_body);
                },
                None,
                Some(&format!("send{}", suffix)),
            );
        }
    }

    fn emit_channel_receive_half(
        &mut self,
        to: &LocationId,
        out_ident: &syn::Ident,
        deserialize: Option<&DebugExpr>,
        suffix: &str,
        channel_id: u32,
        elem_ty: &syn::Type,
    ) {
        let to_is_cluster = matches!(to, LocationId::Cluster(_));
        let table = self.channel_table_ident(channel_id, elem_ty);
        let recv_table = syn::Ident::new(&format!("__channel_recv_{suffix}"), Span::call_site());
        let channel_source =
            syn::Ident::new(&format!("__channel_source_{suffix}"), Span::call_site());

        let member_key_expr: syn::Expr = if to_is_cluster {
            syn::parse_quote!(__current_cluster_id)
        } else {
            syn::parse_quote!(0u32)
        };
        let register_stmt: syn::Stmt = syn::parse_quote! {
            let #channel_source = {
                let (__channel_sink, __channel_source) =
                    __root_dfir_rs::util::unbounded_channel::<#elem_ty>();
                #recv_table.borrow_mut().insert(#member_key_expr, __channel_sink);
                __channel_source
            };
        };

        if to_is_cluster {
            self.extra_stmts_cluster
                .entry(to.clone())
                .or_default()
                .push(syn::parse_quote! {
                    let #recv_table = #table.clone();
                });
            self.extra_stmts_cluster
                .entry(to.clone())
                .or_default()
                .push(register_stmt);
        } else {
            self.extra_stmts_global.push(syn::parse_quote! {
                let #recv_table = #table.clone();
            });
            self.extra_stmts_global.push(register_stmt);
        }

        if let Some(deserialize_pipeline) = deserialize {
            self.get_dfir_mut(to).add_dfir(
                parse_quote! {
                    #out_ident = source_stream(#channel_source) -> map(|v| -> ::std::result::Result<_, ()> { Ok(v) }) -> map(#deserialize_pipeline);
                },
                None,
                Some(&format!("recv{}", suffix)),
            );
        } else {
            self.get_dfir_mut(to).add_dfir(
                parse_quote! {
                    #out_ident = source_stream(#channel_source);
                },
                None,
                Some(&format!("recv{}", suffix)),
            );
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
        fold_hooked_idents: &HashSet<String>,
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
            let root = get_this_crate();

            match in_kind {
                CollectionKind::Stream {
                    order,
                    retry: StreamRetry::ExactlyOnce,
                    element_type,
                    ..
                } => {
                    debug_assert!(in_location.is_top_level());

                    let order_ty: syn::Type = match order {
                        StreamOrder::TotalOrder => {
                            parse_quote! { #root::live_collections::stream::TotalOrder }
                        }
                        StreamOrder::NoOrder => {
                            parse_quote! { #root::live_collections::stream::NoOrder }
                        }
                    };

                    let hoff_id = self.next_hoff_id.get_and_increment();

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
                            Box::new(#root::sim::runtime::StreamHook::<_, #order_ty> {
                                input: #buffered_ident.clone(),
                                to_release: None,
                                output: #hoff_send_ident,
                                batch_location: (#batch_location, #line, #caret),
                                format_item_debug: #root::__maybe_debug__!(#element_type),
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
                    key_type,
                    value_type,
                    ..
                } => {
                    debug_assert!(in_location.is_top_level());

                    let order_ty: syn::Type = match value_order {
                        StreamOrder::TotalOrder => {
                            parse_quote! { #root::live_collections::stream::TotalOrder }
                        }
                        StreamOrder::NoOrder => {
                            parse_quote! { #root::live_collections::stream::NoOrder }
                        }
                    };

                    let hoff_id = self.next_hoff_id.get_and_increment();

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
                            Box::new(#root::sim::runtime::KeyedStreamHook::<_, _, #order_ty> {
                                input: #buffered_ident.clone(),
                                to_release: None,
                                output: #hoff_send_ident,
                                batch_location: (#batch_location, #line, #caret),
                                format_item_debug: #root::__maybe_debug__!((#key_type, #value_type)),
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
                CollectionKind::Singleton { element_type, .. } => {
                    debug_assert!(in_location.is_top_level());

                    let hoff_id = self.next_hoff_id.get_and_increment();

                    let buffered_ident =
                        syn::Ident::new(&format!("__buffered_{hoff_id}"), Span::call_site());
                    let hoff_send_ident =
                        syn::Ident::new(&format!("__hoff_send_{hoff_id}"), Span::call_site());
                    let hoff_recv_ident =
                        syn::Ident::new(&format!("__hoff_recv_{hoff_id}"), Span::call_site());

                    let hook_expr: syn::Expr = if fold_hooked_idents.contains(&in_ident.to_string())
                    {
                        // The fold hook already controls when new values are produced.
                        // Use a PassthroughSingletonHook that always releases the latest
                        // value without non-deterministic decisions.
                        syn::parse_quote!(
                            Box::new(#root::sim::runtime::PassthroughSingletonHook::<_>::new(
                                #buffered_ident.clone(),
                                #hoff_send_ident,
                                (#batch_location, #line, #caret),
                                #root::__maybe_debug__!(#element_type),
                            ))
                        )
                    } else {
                        syn::parse_quote!(
                            Box::new(#root::sim::runtime::SingletonHook::<_>::new(
                                #buffered_ident.clone(),
                                #hoff_send_ident,
                                (#batch_location, #line, #caret),
                                #root::__maybe_debug__!(#element_type),
                            ))
                        )
                    };

                    self.add_extra_stmt_internal(in_location, syn::parse_quote! {
                        let (#hoff_send_ident, #hoff_recv_ident) = __root_dfir_rs::util::unbounded_channel();
                    });
                    self.add_extra_stmt_internal(in_location, syn::parse_quote! {
                        let #buffered_ident = ::std::rc::Rc::new(::std::cell::RefCell::new(::std::collections::VecDeque::new()));
                    });
                    self.add_hook(in_location, out_location, hook_expr);

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
                CollectionKind::KeyedSingleton {
                    bound,
                    key_type,
                    value_type,
                } => {
                    if *bound == KeyedSingletonBoundKind::Unbounded {
                        todo!(
                            "Simulation of Unbounded keyed singletons is not yet supported. \
                             Keys may be removed in Unbounded keyed singletons, which the simulator \
                             cannot currently model. Use a fold (which gives MonotonicKeys) or \
                             another operator that guarantees keys are never removed."
                        );
                    }

                    debug_assert!(in_location.is_top_level());

                    let hoff_id = self.next_hoff_id.get_and_increment();

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
                            Box::new(#root::sim::runtime::KeyedSingletonHook::<_, _>::new(
                                #buffered_ident.clone(),
                                #hoff_send_ident,
                                (#batch_location, #line, #caret),
                                #root::__maybe_debug__!(#key_type),
                                #root::__maybe_debug__!(#value_type),
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
                    eprintln!("{:?}", op_meta.backtrace.elements().collect::<Vec<_>>());
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
                    let hoff_id = self.next_hoff_id.get_and_increment();

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
        // Atomic boundaries never involve fold-hooked idents.
        self.batch(
            in_ident,
            in_location,
            in_kind,
            out_ident,
            out_location,
            op_meta,
            &HashSet::new(),
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
        } else if !location.is_root() || in_kind.is_bounded() {
            // situations where all pending elements should be processed at once
            if location.is_root() && in_kind.is_bounded() {
                todo!(
                    "observe_nondet with top-level bounded input not yet supported for kinds {:?} -> {:?}",
                    in_kind,
                    out_kind
                )
            }

            let (assume_location, line, caret) = location_for_op(op_meta);
            let root = get_this_crate();

            let location = if let LocationId::Atomic(tick) = location {
                tick.as_ref()
            } else {
                location
            };

            match (in_kind, out_kind) {
                (
                    CollectionKind::Stream {
                        order: StreamOrder::NoOrder,
                        retry: StreamRetry::ExactlyOnce,
                        element_type,
                        ..
                    },
                    CollectionKind::Stream {
                        order: StreamOrder::TotalOrder,
                        retry: StreamRetry::ExactlyOnce,
                        ..
                    },
                ) => {
                    let hoff_id = self.next_hoff_id.get_and_increment();

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
                            Box::new(#root::sim::runtime::StreamOrderHook::<_>::new(
                                #buffered_ident.clone(),
                                #hoff_send_ident,
                                (#assume_location, #line, #caret),
                                #root::__maybe_debug__!(#element_type),
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
                        key_type,
                        value_type,
                        ..
                    },
                    CollectionKind::KeyedStream {
                        value_order: StreamOrder::TotalOrder,
                        value_retry: StreamRetry::ExactlyOnce,
                        ..
                    },
                ) => {
                    let hoff_id = self.next_hoff_id.get_and_increment();

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
                            Box::new(#root::sim::runtime::KeyedStreamOrderHook::<_, _>::new(
                                #buffered_ident.clone(),
                                #hoff_send_ident,
                                (#assume_location, #line, #caret),
                                #root::__maybe_debug__!(#key_type),
                                #root::__maybe_debug__!(#value_type),
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
                        value_order: StreamOrder::TotalOrder,
                        value_retry: StreamRetry::ExactlyOnce,
                        key_type,
                        value_type,
                        ..
                    },
                    CollectionKind::Stream {
                        order: StreamOrder::TotalOrder,
                        retry: StreamRetry::ExactlyOnce,
                        ..
                    },
                ) => {
                    let hoff_id = self.next_hoff_id.get_and_increment();

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
                            Box::new(#root::sim::runtime::PartiallyOrderedStreamHook::<_, _>::new(
                                #buffered_ident.clone(),
                                #hoff_send_ident,
                                (#assume_location, #line, #caret),
                                #root::__maybe_debug__!(#key_type),
                                #root::__maybe_debug__!(#value_type),
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
            let (assume_location, line, caret) = location_for_op(op_meta);
            let root = get_this_crate();

            match (in_kind, out_kind) {
                (
                    CollectionKind::Stream {
                        order: StreamOrder::NoOrder,
                        retry: StreamRetry::ExactlyOnce,
                        element_type,
                        ..
                    },
                    CollectionKind::Stream {
                        order: StreamOrder::TotalOrder,
                        retry: StreamRetry::ExactlyOnce,
                        ..
                    },
                ) => {
                    let hoff_id = self.next_hoff_id.get_and_increment();

                    let buffered_ident =
                        syn::Ident::new(&format!("__buffered_{hoff_id}"), Span::call_site());
                    let hoff_send_ident =
                        syn::Ident::new(&format!("__hoff_send_{hoff_id}"), Span::call_site());
                    let hoff_recv_ident =
                        syn::Ident::new(&format!("__hoff_recv_{hoff_id}"), Span::call_site());

                    self.add_extra_stmt_internal(location, syn::parse_quote! {
                        let (#hoff_send_ident, #hoff_recv_ident) = __root_dfir_rs::util::unbounded_channel();
                    });
                    self.add_extra_stmt_internal(location, syn::parse_quote! {
                        let #buffered_ident = ::std::rc::Rc::new(::std::cell::RefCell::new(::std::collections::VecDeque::new()));
                    });
                    self.add_hook(
                        location,
                        location,
                        syn::parse_quote!(
                            Box::new(#root::sim::runtime::TopLevelStreamOrderHook::<_> {
                                input: #buffered_ident.clone(),
                                to_release: None,
                                output: #hoff_send_ident,
                                location: (#assume_location, #line, #caret),
                                format_item_debug: #root::__maybe_debug__!(#element_type),
                            })
                        ),
                    );

                    self.get_dfir_mut(location).add_dfir(
                        parse_quote! {
                            #in_ident -> for_each(|v| #buffered_ident.borrow_mut().push_back(v));
                        },
                        None,
                        None,
                    );

                    self.get_dfir_mut(location).add_dfir(
                        parse_quote! {
                            #out_ident = source_stream(#hoff_recv_ident);
                        },
                        None,
                        None,
                    );
                }
                (
                    CollectionKind::KeyedStream {
                        value_order: StreamOrder::NoOrder,
                        value_retry: StreamRetry::ExactlyOnce,
                        key_type,
                        value_type,
                        ..
                    },
                    CollectionKind::KeyedStream {
                        value_order: StreamOrder::TotalOrder,
                        value_retry: StreamRetry::ExactlyOnce,
                        ..
                    },
                ) => {
                    let hoff_id = self.next_hoff_id.get_and_increment();

                    let buffered_ident =
                        syn::Ident::new(&format!("__buffered_{hoff_id}"), Span::call_site());
                    let hoff_send_ident =
                        syn::Ident::new(&format!("__hoff_send_{hoff_id}"), Span::call_site());
                    let hoff_recv_ident =
                        syn::Ident::new(&format!("__hoff_recv_{hoff_id}"), Span::call_site());

                    self.add_extra_stmt_internal(location, syn::parse_quote! {
                        let (#hoff_send_ident, #hoff_recv_ident) = __root_dfir_rs::util::unbounded_channel();
                    });
                    self.add_extra_stmt_internal(location, syn::parse_quote! {
                        let #buffered_ident = ::std::rc::Rc::new(::std::cell::RefCell::new(__root_dfir_rs::rustc_hash::FxHashMap::default()));
                    });
                    self.add_hook(
                        location,
                        location,
                        syn::parse_quote!(
                            Box::new(#root::sim::runtime::TopLevelKeyedStreamOrderHook::<_, _> {
                                input: #buffered_ident.clone(),
                                to_release: None,
                                output: #hoff_send_ident,
                                location: (#assume_location, #line, #caret),
                                format_item_debug: #root::__maybe_debug__!((#key_type, #value_type)),
                            })
                        ),
                    );

                    self.get_dfir_mut(location).add_dfir(
                        parse_quote! {
                            #in_ident -> for_each(|(k, v)| #buffered_ident.borrow_mut().entry(k).or_insert_with(::std::collections::VecDeque::new).push_back(v));
                        },
                        None,
                        None,
                    );

                    self.get_dfir_mut(location).add_dfir(
                        parse_quote! {
                            #out_ident = source_stream(#hoff_recv_ident);
                        },
                        None,
                        None,
                    );
                }
                (
                    CollectionKind::KeyedStream {
                        value_order: StreamOrder::TotalOrder,
                        value_retry: StreamRetry::ExactlyOnce,
                        key_type,
                        value_type,
                        ..
                    },
                    CollectionKind::Stream {
                        order: StreamOrder::TotalOrder,
                        retry: StreamRetry::ExactlyOnce,
                        ..
                    },
                ) => {
                    let hoff_id = self.next_hoff_id.get_and_increment();

                    let buffered_ident =
                        syn::Ident::new(&format!("__buffered_{hoff_id}"), Span::call_site());
                    let hoff_send_ident =
                        syn::Ident::new(&format!("__hoff_send_{hoff_id}"), Span::call_site());
                    let hoff_recv_ident =
                        syn::Ident::new(&format!("__hoff_recv_{hoff_id}"), Span::call_site());

                    self.add_extra_stmt_internal(location, syn::parse_quote! {
                        let (#hoff_send_ident, #hoff_recv_ident) = __root_dfir_rs::util::unbounded_channel();
                    });
                    self.add_extra_stmt_internal(location, syn::parse_quote! {
                        let #buffered_ident = ::std::rc::Rc::new(::std::cell::RefCell::new(__root_dfir_rs::rustc_hash::FxHashMap::default()));
                    });
                    self.add_hook(
                        location,
                        location,
                        syn::parse_quote!(
                            Box::new(#root::sim::runtime::TopLevelPartiallyOrderedStreamHook::<_, _> {
                                input: #buffered_ident.clone(),
                                to_release: None,
                                output: #hoff_send_ident,
                                location: (#assume_location, #line, #caret),
                                format_item_debug: #root::__maybe_debug__!((#key_type, #value_type)),
                            })
                        ),
                    );

                    self.get_dfir_mut(location).add_dfir(
                        parse_quote! {
                            #in_ident -> for_each(|(k, v)| #buffered_ident.borrow_mut().entry(k).or_insert_with(::std::collections::VecDeque::new).push_back(v));
                        },
                        None,
                        None,
                    );

                    self.get_dfir_mut(location).add_dfir(
                        parse_quote! {
                            #out_ident = source_stream(#hoff_recv_ident);
                        },
                        None,
                        None,
                    );
                }
                _ => {
                    todo!(
                        "non-trusted observe_nondet not yet supported for kinds {:?} -> {:?} at top-level locations",
                        in_kind,
                        out_kind
                    );
                }
            }
        }
    }

    fn merge_ordered(
        &mut self,
        location: &LocationId,
        first_ident: syn::Ident,
        second_ident: syn::Ident,
        out_ident: &syn::Ident,
        in_kind: &CollectionKind,
        op_meta: &HydroIrOpMetadata,
        _operator_tag: Option<&str>,
    ) {
        let location = if let LocationId::Atomic(tick) = location {
            tick.as_ref()
        } else {
            location
        };

        let (assume_location, line, caret) = location_for_op(op_meta);
        let root = get_this_crate();

        let element_type: syn::Type = match in_kind {
            CollectionKind::Stream { element_type, .. } => parse_quote!(#element_type),
            CollectionKind::KeyedStream {
                key_type,
                value_type,
                ..
            } => parse_quote!((#key_type, #value_type)),
            CollectionKind::Singleton { element_type, .. } => parse_quote!(#element_type),
            CollectionKind::Optional { element_type, .. } => parse_quote!(#element_type),
            CollectionKind::KeyedSingleton {
                key_type,
                value_type,
                ..
            } => parse_quote!((#key_type, #value_type)),
        };

        if !location.is_root() || in_kind.is_bounded() {
            // Inside a tick: both inputs are fully materialized batches.
            // Generate a valid interleaving preserving per-input order.
            let hoff_id = self.next_hoff_id.get_and_increment();

            let buffered_first_ident =
                syn::Ident::new(&format!("__buffered_first_{hoff_id}"), Span::call_site());
            let buffered_second_ident =
                syn::Ident::new(&format!("__buffered_second_{hoff_id}"), Span::call_site());
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

            self.add_extra_stmt_internal(
                location.root(),
                syn::parse_quote! {
                    let #buffered_first_ident = ::std::rc::Rc::new(::std::cell::RefCell::new(None));
                },
            );

            self.add_extra_stmt_internal(location.root(), syn::parse_quote! {
                let #buffered_second_ident = ::std::rc::Rc::new(::std::cell::RefCell::new(None));
            });

            self.add_inline_hook(
                location,
                syn::parse_quote!(
                    Box::new(#root::sim::runtime::MergeOrderedHook::<_>::new(
                        #buffered_first_ident.clone(),
                        #buffered_second_ident.clone(),
                        #hoff_send_ident,
                        (#assume_location, #line, #caret),
                        #root::__maybe_debug__!(#element_type),
                    ))
                ),
            );

            let builder = self.get_dfir_mut(location);

            // First input: buffer the batch
            let first_fold_ident =
                syn::Ident::new(&format!("__merge_first_fold_{hoff_id}"), Span::call_site());
            builder.add_dfir(
                parse_quote! {
                    #first_fold_ident = #first_ident -> fold::<'tick>(
                        || ::std::vec::Vec::new(),
                        |acc, v| {
                            acc.push(v);
                        }
                    ) -> for_each(|v| {
                        *#buffered_first_ident.borrow_mut() = Some(v);
                    });
                },
                None,
                None,
            );

            // Second input: buffer the batch
            let second_fold_ident =
                syn::Ident::new(&format!("__merge_second_fold_{hoff_id}"), Span::call_site());
            builder.add_dfir(
                parse_quote! {
                    #second_fold_ident = #second_ident -> fold::<'tick>(
                        || ::std::vec::Vec::new(),
                        |acc, v| {
                            acc.push(v);
                        }
                    ) -> for_each(|v| {
                        *#buffered_second_ident.borrow_mut() = Some(v);
                    });
                },
                None,
                None,
            );

            // Output: await the hook's interleaved result
            builder.add_dfir(
                parse_quote! {
                    #out_ident = source_iter([{
                        let #hoff_recv_ident = #hoff_recv_ident.clone();
                        async move {
                            #hoff_recv_ident.borrow_mut().recv().await.unwrap()
                        }
                    }]) -> resolve_futures_blocking() -> flatten();
                },
                None,
                None,
            );
        } else {
            let hoff_id = self.next_hoff_id.get_and_increment();

            let buffered_first_ident =
                syn::Ident::new(&format!("__buffered_first_{hoff_id}"), Span::call_site());
            let buffered_second_ident =
                syn::Ident::new(&format!("__buffered_second_{hoff_id}"), Span::call_site());
            let hoff_send_ident =
                syn::Ident::new(&format!("__hoff_send_{hoff_id}"), Span::call_site());
            let hoff_recv_ident =
                syn::Ident::new(&format!("__hoff_recv_{hoff_id}"), Span::call_site());

            self.add_extra_stmt_internal(location, syn::parse_quote! {
                let (#hoff_send_ident, #hoff_recv_ident) = __root_dfir_rs::util::unbounded_channel();
            });
            self.add_extra_stmt_internal(location, syn::parse_quote! {
                let #buffered_first_ident = ::std::rc::Rc::new(::std::cell::RefCell::new(::std::collections::VecDeque::new()));
            });
            self.add_extra_stmt_internal(location, syn::parse_quote! {
                let #buffered_second_ident = ::std::rc::Rc::new(::std::cell::RefCell::new(::std::collections::VecDeque::new()));
            });
            self.add_hook(
                location,
                location,
                syn::parse_quote!(
                    Box::new(#root::sim::runtime::TopLevelMergeOrderedHook::<_> {
                        first: #buffered_first_ident.clone(),
                        second: #buffered_second_ident.clone(),
                        to_release: None,
                        release_source: None,
                        output: #hoff_send_ident,
                        location: (#assume_location, #line, #caret),
                        format_item_debug: #root::__maybe_debug__!(#element_type),
                    })
                ),
            );

            self.get_dfir_mut(location).add_dfir(
                parse_quote! {
                    #first_ident -> for_each(|v| #buffered_first_ident.borrow_mut().push_back(v));
                },
                None,
                None,
            );

            self.get_dfir_mut(location).add_dfir(
                parse_quote! {
                    #second_ident -> for_each(|v| #buffered_second_ident.borrow_mut().push_back(v));
                },
                None,
                None,
            );

            self.get_dfir_mut(location).add_dfir(
                parse_quote! {
                    #out_ident = source_stream(#hoff_recv_ident);
                },
                None,
                None,
            );
        }
    }

    fn create_network(
        &mut self,
        from: &LocationId,
        to: &LocationId,
        input_ident: syn::Ident,
        out_ident: &syn::Ident,
        serialize: Option<&DebugExpr>,
        sink: syn::Expr,
        source: syn::Expr,
        deserialize: Option<&DebugExpr>,
        tag_id: StmtId,
        networking_info: &crate::networking::NetworkingInfo,
    ) {
        use crate::networking::{NetworkingInfo, TcpFault};
        match networking_info {
            NetworkingInfo::Tcp { fault } => match fault {
                TcpFault::FailStop => {}
                TcpFault::LossyDelayedForever => {
                    assert!(
                        self.test_safety_only,
                        "Simulating `lossy_delayed_forever` requires `.test_safety_only()` on the \
                         SimFlow because the simulator models dropped messages as indefinitely \
                         delayed, which only tests safety (not liveness). Call \
                         `.sim().test_safety_only()` to opt in."
                    );
                }
                _ => todo!(
                    "SimBuilder only supports fail-stop and lossy-delayed-forever TCP networking"
                ),
            },
        }

        let root = get_this_crate();

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
                    let (#sink, #source) = __root_dfir_rs::util::unbounded_channel::<(#root::__staged::location::TaglessMemberId, __root_dfir_rs::bytes::Bytes)>();
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
                            #input_ident -> map(#serialize_pipeline) -> for_each(|v| #sink.send((#root::__staged::location::TaglessMemberId::from_raw_id(__current_cluster_id), v)).unwrap());
                        },
                        None,
                        Some(&format!("send{}", tag_id)),
                    );
                } else {
                    self.get_dfir_mut(from).add_dfir(
                        parse_quote! {
                            #input_ident -> for_each(|v| #sink.send((#root::__staged::location::TaglessMemberId::from_raw_id(__current_cluster_id), v)).unwrap());
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
                            #input_ident -> map(#serialize_pipeline) -> for_each(|(target_member_id, v)| (#sink.borrow())[#root::__staged::location::TaglessMemberId::get_raw_id(&target_member_id) as usize].send(v).unwrap());
                        },
                        None,
                        Some(&format!("send{}", tag_id)),
                    );
                } else {
                    self.get_dfir_mut(from).add_dfir(
                        parse_quote! {
                            #input_ident -> for_each(|(target_member_id, v)| (#sink.borrow())[#root::__staged::location::TaglessMemberId::get_raw_id(&target_member_id) as usize].send(v).unwrap());
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
                    let #sink: ::std::rc::Rc<::std::cell::RefCell<Vec<__root_dfir_rs::tokio::sync::mpsc::UnboundedSender<(#root::__staged::location::TaglessMemberId, __root_dfir_rs::bytes::Bytes)>>>> = ::std::rc::Rc::new(::std::cell::RefCell::new(Vec::new()));
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
                            let (__sink, __source) = __root_dfir_rs::util::unbounded_channel::<(#root::__staged::location::TaglessMemberId, __root_dfir_rs::bytes::Bytes)>();
                            #sink_writer.borrow_mut().push(__sink);
                            __source
                        };
                    });

                if let Some(serialize_pipeline) = serialize {
                    self.get_dfir_mut(from).add_dfir(
                        parse_quote! {
                            #input_ident -> map(#serialize_pipeline) -> for_each(|(target_member_id, v)| (#sink.borrow())[#root::__staged::location::TaglessMemberId::get_raw_id(&target_member_id) as usize].send((#root::__staged::location::TaglessMemberId::from_raw_id(__current_cluster_id), v)).unwrap());
                        },
                        None,
                        Some(&format!("send{}", tag_id)),
                    );
                } else {
                    self.get_dfir_mut(from).add_dfir(
                        parse_quote! {
                            #input_ident -> for_each(|(target_member_id, v)| (#sink.borrow())[#root::__staged::location::TaglessMemberId::get_raw_id(&target_member_id) as usize].send((#root::__staged::location::TaglessMemberId::from_raw_id(__current_cluster_id), v)).unwrap());
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
        deserialize: Option<&DebugExpr>,
        tag_id: StmtId,
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
        serialize: Option<&DebugExpr>,
        tag_id: StmtId,
    ) {
        let grabbed_ident = syn::Ident::new(&format!("__sink_{tag_id}"), Span::call_site());
        self.add_extra_stmt_internal(
            on,
            syn::parse_quote! {
                let #grabbed_ident = #sink_expr;
            },
        );

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

    fn emit_fold_hook(
        &mut self,
        location: &LocationId,
        in_ident: &syn::Ident,
        in_kind: &CollectionKind,
        op_meta: &HydroIrOpMetadata,
    ) -> Option<syn::Ident> {
        if !location.is_top_level() {
            // For in-tick folds on NoOrder input,
            // emit an inline shuffle hook to permute elements before the fold.
            let element_type = match in_kind {
                CollectionKind::Stream {
                    order: StreamOrder::NoOrder,
                    retry: StreamRetry::ExactlyOnce,
                    element_type,
                    ..
                } => element_type.clone(),
                _ => return None,
            };

            let (assume_location, line, caret) = location_for_op(op_meta);
            let root = get_this_crate();

            let tick_location = location;
            let hoff_id = self.next_hoff_id.get_and_increment();

            let buffered_ident =
                syn::Ident::new(&format!("__buffered_{hoff_id}"), Span::call_site());
            let hoff_send_ident =
                syn::Ident::new(&format!("__hoff_send_{hoff_id}"), Span::call_site());
            let hoff_recv_ident =
                syn::Ident::new(&format!("__hoff_recv_{hoff_id}"), Span::call_site());
            let out_ident =
                syn::Ident::new(&format!("__fold_hook_out_{hoff_id}"), Span::call_site());

            self.add_extra_stmt_internal(tick_location.root(), syn::parse_quote! {
                let (#hoff_send_ident, #hoff_recv_ident) = __root_dfir_rs::util::unbounded_channel();
            });

            self.add_extra_stmt_internal(tick_location.root(), syn::parse_quote! {
                let #hoff_recv_ident = ::std::rc::Rc::new(::std::cell::RefCell::new(#hoff_recv_ident.into_inner()));
            });

            self.add_extra_stmt_internal(
                tick_location.root(),
                syn::parse_quote! {
                    let #buffered_ident = ::std::rc::Rc::new(::std::cell::RefCell::new(None));
                },
            );

            self.add_inline_hook(
                tick_location,
                syn::parse_quote!(
                    Box::new(#root::sim::runtime::StreamOrderHook::<_>::new(
                        #buffered_ident.clone(),
                        #hoff_send_ident,
                        (#assume_location, #line, #caret),
                        #root::__maybe_debug__!(#element_type),
                    ))
                ),
            );

            let builder = self.get_dfir_mut(tick_location);
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

            return Some(out_ident);
        }

        let (assume_location, line, caret) = location_for_op(op_meta);
        let root = get_this_crate();

        let debug_type: syn::Type = match in_kind {
            CollectionKind::Stream {
                order: StreamOrder::NoOrder,
                retry: StreamRetry::ExactlyOnce,
                element_type,
                ..
            } => (*element_type.0).clone(),
            CollectionKind::KeyedStream {
                value_order: StreamOrder::NoOrder,
                value_retry: StreamRetry::ExactlyOnce,
                key_type,
                value_type,
                ..
            } => syn::parse_quote!((#key_type, #value_type)),
            _ => return None,
        };

        let hoff_id = self.next_hoff_id.get_and_increment();

        let buffered_ident = syn::Ident::new(&format!("__buffered_{hoff_id}"), Span::call_site());
        let hoff_send_ident = syn::Ident::new(&format!("__hoff_send_{hoff_id}"), Span::call_site());
        let hoff_recv_ident = syn::Ident::new(&format!("__hoff_recv_{hoff_id}"), Span::call_site());
        let out_ident = syn::Ident::new(&format!("__fold_hook_out_{hoff_id}"), Span::call_site());

        self.add_extra_stmt_internal(location, syn::parse_quote! {
            let (#hoff_send_ident, #hoff_recv_ident) = __root_dfir_rs::util::unbounded_channel();
        });
        self.add_extra_stmt_internal(location, syn::parse_quote! {
            let #buffered_ident = ::std::rc::Rc::new(::std::cell::RefCell::new(::std::collections::VecDeque::new()));
        });
        self.add_hook(
            location,
            location,
            syn::parse_quote!(
                Box::new(#root::sim::runtime::TopLevelFoldHook::<_> {
                    input: #buffered_ident.clone(),
                    to_release: None,
                    output: #hoff_send_ident,
                    location: (#assume_location, #line, #caret),
                    format_item_debug: #root::__maybe_debug__!(#debug_type),
                })
            ),
        );

        self.get_dfir_mut(location).add_dfir(
            parse_quote! {
                #in_ident -> for_each(|v| #buffered_ident.borrow_mut().push_back(v));
            },
            None,
            None,
        );

        self.get_dfir_mut(location).add_dfir(
            parse_quote! {
                #out_ident = source_stream(#hoff_recv_ident);
            },
            None,
            None,
        );

        Some(out_ident)
    }

    fn assert_is_consistent(
        &mut self,
        trusted: bool,
        location: &LocationId,
        in_ident: syn::Ident,
        out_ident: &syn::Ident,
    ) {
        if self.skip_consistency_assertions || trusted {
            let builder = self.get_dfir_mut(location);
            builder.add_dfir(
                parse_quote! {
                    #out_ident = #in_ident;
                },
                None,
                None,
            );
        } else {
            // TODO(shadaj): inject assertions that validate consistency in simulation
            panic!(
                "validating consistency assertions is not yet supported in the simulator; call `.skip_consistency_assertions()` on the SimFlow to skip them"
            );
        }
    }

    fn observe_for_mut(
        &mut self,
        location: &LocationId,
        in_ident: syn::Ident,
        in_kind: &CollectionKind,
        out_ident: &syn::Ident,
        op_meta: &HydroIrOpMetadata,
    ) {
        let out_kind = in_kind.strict_kind();
        self.observe_nondet(
            false, location, in_ident, in_kind, out_ident, &out_kind, op_meta,
        );
    }

    fn create_versioned_network_fork(
        &mut self,
        channel_id: u32,
        dest: &LocationId,
        senders: Vec<(LocationId, syn::Ident, Option<DebugExpr>)>,
        tag_id: StmtId,
    ) {
        let root = get_this_crate();
        for (idx, (source, input_ident, serialize)) in senders.into_iter().enumerate() {
            let suffix = format!("{}_{}", tag_id, idx);
            self.emit_channel_send_half(
                &source,
                dest,
                input_ident,
                serialize.as_ref(),
                &suffix,
                channel_id,
                &root,
            );
        }
    }

    fn create_versioned_network(
        &mut self,
        channel_id: u32,
        source: &LocationId,
        dest: &LocationId,
        out_ident: &syn::Ident,
        deserialize: Option<&DebugExpr>,
        tag_id: StmtId,
    ) {
        let root = get_this_crate();
        let elem_ty = Self::channel_elem_ty(source, &root);
        self.emit_channel_receive_half(
            dest,
            out_ident,
            deserialize,
            &tag_id.to_string(),
            channel_id,
            &elem_ty,
        );
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
        .next()
        .and_then(|e| {
            let filename = e.filename.as_deref()?;
            let lineno = e.lineno?;
            let colno = e.colno?;

            let line = std::fs::read_to_string(filename)
                .ok()
                .and_then(|s| {
                    s.lines()
                        .nth(lineno.saturating_sub(1).try_into().unwrap())
                        .map(|s| s.to_owned())
                })
                .unwrap_or_default();

            let relative_path = (|| {
                std::path::Path::new(filename)
                    .strip_prefix(std::env::current_dir().ok()?)
                    .ok()
            })();

            let filename_display = relative_path
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| filename.to_owned());

            Some((
                format!("{}:{}:{}", filename_display, lineno, colno),
                line,
                format!("{:>1$}", "", (colno - 1).try_into().unwrap()),
            ))
        })
        .unwrap_or_else(|| ("unknown location".to_owned(), "".to_owned(), "".to_owned()))
}
