use std::collections::{BTreeMap, HashMap, HashSet};

use dfir_lang::graph::FlatGraphBuilder;
use proc_macro2::Span;
use quote::ToTokens;
use syn::parse_quote;

use crate::compile::builder::{HandoffId, StmtId};
use crate::compile::ir::{
    CollectionKind, DebugExpr, DfirBuilder, HydroIrOpMetadata, KeyedSingletonBoundKind,
    OptionalBoundKind, StreamOrder, StreamRetry,
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
/// observes them (such as batch or `assume_ordering`).
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
    /// Descriptions (what is observed, and where) of the at-least-once observation points
    /// that are not bound to a sim hook and so are explored autonomously under the run's
    /// retry budget; reported by modes that require a bound when none is configured.
    pub unhooked_at_least_once: Vec<String>,
    pub channel_tables: BTreeMap<u32, syn::Ident>,
    /// Tracks which operators sim hook handles have been bound to, to report double-binds
    /// at flow build time with both operator locations.
    pub bound_sim_hooks: HashMap<usize, String>,
}

impl SimBuilder {
    /// Gets the DFIR builder for the given location, creating it if necessary.
    ///
    /// Unlike production codegen, the simulator emits a separate DFIR graph for each tick
    /// location, in addition to the fused async graph for each root location.
    fn get_dfir_mut(&mut self, location: &LocationId) -> &mut FlatGraphBuilder {
        match location {
            LocationId::Process(_) => self.process_graphs.entry(location.clone()).or_default(),
            LocationId::Cluster(_) => self.cluster_graphs.entry(location.clone()).or_default(),
            LocationId::Atomic(tick) => self.get_dfir_mut(tick.as_ref()),
            LocationId::Tick {
                tick: _,
                parent_location,
            } => match parent_location.root() {
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
        let root = get_this_crate();
        let out_location_ser = serde_json::to_string(out_location).unwrap();
        // Tick-input hooks are keyed by their tick's location; observation hooks by the
        // top-level process/cluster location. Route each to the map with the matching
        // trait-object type (`TickInputHook` vs `ObservationHook`).
        let map: syn::Ident = match out_location {
            LocationId::Tick { .. } => syn::parse_quote!(__hydro_hooks),
            LocationId::Process(_) | LocationId::Cluster(_) => {
                syn::parse_quote!(__hydro_observation_hooks)
            }
            _ => unreachable!("hooks are keyed by a tick or top-level location"),
        };
        match in_location {
            LocationId::Process(_) => {
                self.add_extra_stmt_internal(
                    in_location,
                    syn::parse_quote! {
                        #map.entry(#root::sim::runtime::SimLocation { location: #root::sim::runtime::parse_location(#out_location_ser), cluster_id: None }).or_default().push(#expr);
                    },
                );
            }
            LocationId::Cluster(_) => {
                self.add_extra_stmt_internal(in_location, syn::parse_quote! {
                    #map.entry(#root::sim::runtime::SimLocation { location: #root::sim::runtime::parse_location(#out_location_ser), cluster_id: Some(__current_cluster_id) }).or_default().push(#expr);
                });
            }
            _ => unreachable!(),
        }
    }

    /// Registers a scripted hook (one bound to a sim hook handle): emits the shared
    /// `Rc<RefCell<...>>`, registers it by handle ID in the per-instance registry
    /// (`__hydro_scripted_registry`) for the test-side handle, and pushes it into the
    /// location-keyed scripted-hook map matching its kind (`__hydro_scripted_hooks` for
    /// tick inputs, `__hydro_scripted_observation_hooks` for top-level observations).
    ///
    /// On a cluster, these statements are emitted inside the per-member instantiation
    /// loop (where `__current_cluster_id` is bound), so every member gets its own
    /// independent hook instance, registered under `(handle_id, Some(member))` and
    /// targeting that member's tick/observation.
    fn add_scripted_hook(
        &mut self,
        hook_id: usize,
        in_location: &LocationId,
        out_location: &LocationId,
        hook_rc_ident: &syn::Ident,
        op_location: &str,
        core_expr: syn::Expr,
    ) {
        if let Some(prev) = self.bound_sim_hooks.insert(hook_id, op_location.to_owned()) {
            panic!(
                "the same sim hook handle was bound to two different operators:\n  first:  {}\n  second: {}",
                prev, op_location
            );
        }

        let root = get_this_crate();
        let out_location_ser = serde_json::to_string(out_location).unwrap();
        // The member the hook instance belongs to: `None` on a process, the loop
        // variable of the per-member instantiation loop on a cluster.
        let cluster_id_expr: syn::Expr = match in_location {
            LocationId::Process(_) => syn::parse_quote!(None),
            LocationId::Cluster(_) => syn::parse_quote!(Some(__current_cluster_id)),
            _ => unreachable!("scripted hooks are emitted at a top-level location"),
        };
        // Like `add_hook`, tick-input and observation hooks go to separately typed maps
        // (`ScriptedTickHooks` vs `ScriptedObservationHooks`), matching the target kind.
        let (target, map): (syn::Expr, syn::Ident) = match out_location {
            LocationId::Tick { .. } => (
                syn::parse_quote! {
                    #root::sim::runtime::ScriptTarget::Tick {
                        location: #root::sim::runtime::SimLocation {
                            location: #root::sim::runtime::parse_location(#out_location_ser),
                            cluster_id: #cluster_id_expr,
                        },
                    }
                },
                syn::parse_quote!(__hydro_scripted_hooks),
            ),
            LocationId::Process(_) | LocationId::Cluster(_) => (
                syn::parse_quote! {
                    #root::sim::runtime::ScriptTarget::Observation {
                        location: #root::sim::runtime::SimLocation {
                            location: #root::sim::runtime::parse_location(#out_location_ser),
                            cluster_id: #cluster_id_expr,
                        },
                        hook_id: #hook_id,
                    }
                },
                syn::parse_quote!(__hydro_scripted_observation_hooks),
            ),
            _ => unreachable!("scripted hooks are keyed by a tick or top-level location"),
        };

        self.add_extra_stmt_internal(
            in_location,
            syn::parse_quote! {
                let #hook_rc_ident = ::std::rc::Rc::new(::std::cell::RefCell::new(
                    #root::sim::runtime::Scripted::new(#core_expr, #target)
                ));
            },
        );

        self.add_extra_stmt_internal(
            in_location,
            syn::parse_quote! {
                assert!(
                    __hydro_scripted_registry.insert((#hook_id, #cluster_id_expr), #hook_rc_ident.clone()).is_none(),
                    "a sim hook handle was bound to multiple operators"
                );
            },
        );

        self.add_extra_stmt_internal(in_location, syn::parse_quote! {
            #map.entry(#root::sim::runtime::SimLocation { location: #root::sim::runtime::parse_location(#out_location_ser), cluster_id: #cluster_id_expr }).or_default().push(#hook_rc_ident);
        });
    }

    /// Registers a scripted inline hook. The concrete wrapper is shared between the
    /// handle-facing registry and the tick-keyed inline-hook map, but exposed through the
    /// separate trait surfaces each side needs.
    ///
    /// Like [`Self::add_scripted_hook`], on a cluster these statements run once per
    /// member (inside the loop binding `__current_cluster_id`), yielding one independent
    /// hook instance per member.
    fn add_scripted_inline_hook(
        &mut self,
        hook_id: usize,
        tick_location: &LocationId,
        hook_rc_ident: &syn::Ident,
        op_location: &str,
        core_expr: syn::Expr,
    ) {
        if let Some(prev) = self.bound_sim_hooks.insert(hook_id, op_location.to_owned()) {
            panic!(
                "the same sim hook handle was bound to two different operators:\n  first:  {}\n  second: {}",
                prev, op_location
            );
        }

        let root = get_this_crate();
        let tick_location_ser = serde_json::to_string(tick_location).unwrap();
        let cluster_id_expr: syn::Expr = match tick_location.root() {
            LocationId::Process(_) => syn::parse_quote!(None),
            LocationId::Cluster(_) => syn::parse_quote!(Some(__current_cluster_id)),
            _ => unreachable!("ticks are rooted at a top-level location"),
        };

        self.add_extra_stmt_internal(
            tick_location.root(),
            syn::parse_quote! {
                let #hook_rc_ident = ::std::rc::Rc::new(::std::cell::RefCell::new(
                    #root::sim::runtime::ScriptedInline::new(
                        #core_expr,
                        #root::sim::runtime::ScriptTarget::Tick {
                            location: #root::sim::runtime::SimLocation {
                                location: #root::sim::runtime::parse_location(#tick_location_ser),
                                cluster_id: #cluster_id_expr,
                            },
                        },
                    )
                ));
            },
        );
        self.add_extra_stmt_internal(
            tick_location.root(),
            syn::parse_quote! {
                assert!(
                    __hydro_scripted_registry.insert((#hook_id, #cluster_id_expr), #hook_rc_ident.clone()).is_none(),
                    "a sim hook handle was bound to multiple operators"
                );
            },
        );
        self.add_extra_stmt_internal(tick_location.root(), syn::parse_quote! {
            __hydro_scripted_inline_hooks.entry(#root::sim::runtime::SimLocation { location: #root::sim::runtime::parse_location(#tick_location_ser), cluster_id: #cluster_id_expr }).or_default().push(#hook_rc_ident);
        });
    }

    fn add_inline_hook(&mut self, tick_location: &LocationId, expr: syn::Expr) {
        let root = get_this_crate();
        let tick_location_ser = serde_json::to_string(tick_location).unwrap();
        match tick_location {
            LocationId::Tick {
                tick: _,
                parent_location,
            } => match parent_location.root() {
                LocationId::Process(_) => {
                    self.add_extra_stmt_internal(
                        parent_location.root(),
                        syn::parse_quote! {
                            __hydro_inline_hooks.entry(#root::sim::runtime::SimLocation { location: #root::sim::runtime::parse_location(#tick_location_ser), cluster_id: None }).or_default().push(#expr);
                        },
                    );
                }
                LocationId::Cluster(_) => {
                    self.add_extra_stmt_internal(parent_location.root(), syn::parse_quote! {
                        __hydro_inline_hooks.entry(#root::sim::runtime::SimLocation { location: #root::sim::runtime::parse_location(#tick_location_ser), cluster_id: Some(__current_cluster_id) }).or_default().push(#expr);
                    });
                }
                _ => unreachable!(),
            },
            _ => unreachable!(),
        }
    }

    /// Resolves how an at-least-once observation point (whose decision space is infinite
    /// unless bounded) is driven, returning the hook's `budget` expression:
    /// `RetryBudget::Scripted` when the operator is bound to a sim hook (the script
    /// decides), otherwise the run's autonomous budget (`__hydro_retry_budget`, a parameter
    /// of the generated runtime entry point chosen by the simulation mode). Unhooked points
    /// are recorded so the host can refuse modes that need a bound without one.
    fn at_least_once_budget(
        &mut self,
        op_meta: &HydroIrOpMetadata,
        op_location: &str,
        what: &str,
    ) -> syn::Expr {
        let root = get_this_crate();
        if op_meta.sim_hook_id.is_some() {
            syn::parse_quote!(#root::sim::runtime::RetryBudget::Scripted)
        } else {
            self.unhooked_at_least_once
                .push(format!("{what}\n--> {op_location}"));
            syn::parse_quote!(__hydro_retry_budget)
        }
    }

    fn channel_elem_ty(
        from: &LocationId,
        root: &proc_macro2::TokenStream,
        external_ty: Option<&syn::Type>,
    ) -> syn::Type {
        // For embedded (external) serialization, the raw payload type flows across the in-memory
        // simulation channel instead of serialized `Bytes`.
        let payload: syn::Type = match external_ty {
            Some(ty) => ty.clone(),
            None => syn::parse_quote!(__root_dfir_rs::bytes::Bytes),
        };
        if matches!(from, LocationId::Cluster(_)) {
            syn::parse_quote!((#root::__staged::location::TaglessMemberId, #payload))
        } else {
            payload
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
            let #ident: ::std::rc::Rc<::std::cell::RefCell<::std::collections::HashMap<u32, __root_dfir_rs::util::unsync::mpsc::Sender<#elem_ty>>>> =
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
        external_ty: Option<&syn::Type>,
        suffix: &str,
        channel_id: u32,
        root: &proc_macro2::TokenStream,
    ) {
        let from_is_cluster = matches!(from, LocationId::Cluster(_));
        let to_is_cluster = matches!(to, LocationId::Cluster(_));
        let elem_ty = Self::channel_elem_ty(from, root, external_ty);
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
                    let _ = __s.try_send(#payload_expr);
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
                    __root_dfir_rs::util::unsync::mpsc::unbounded::<#elem_ty>();
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

    /// Lowers a `batch_atomic` / `snapshot_atomic`: the input already lives in the tick's
    /// atomic region, so the non-deterministic decision happened where the collection *entered*
    /// the atomic context and the tick boundary itself is the identity.
    fn enter_tick_from_atomic(
        &mut self,
        in_ident: syn::Ident,
        in_location: &LocationId,
        out_ident: &syn::Ident,
        op_meta: &HydroIrOpMetadata,
        op_name: &str,
    ) {
        assert!(
            op_meta.sim_hook_id.is_none(),
            "sim hooks are not supported on `{op_name}`: the non-deterministic decision happens \
             where the collection *enters* the atomic context, not at the atomic tick boundary \
             (at {})",
            location_for_op(op_meta).0
        );
        self.get_dfir_mut(in_location).add_dfir(
            parse_quote! {
                #out_ident = #in_ident;
            },
            None,
            None,
        );
    }

    /// Allocates the plumbing shared by every tick-input hook: a buffer that the top-level graph
    /// pushes arriving items into, and a handoff channel through which the hook releases the
    /// items chosen for each tick execution into the tick's graph.
    fn tick_input_plumbing(
        &mut self,
        in_location: &LocationId,
        buffer: TickInputBuffer,
    ) -> TickInputPlumbing {
        let hoff_id = self.next_hoff_id.get_and_increment();

        let buffered_ident = syn::Ident::new(&format!("__buffered_{hoff_id}"), Span::call_site());
        let hoff_send_ident = syn::Ident::new(&format!("__hoff_send_{hoff_id}"), Span::call_site());
        let hoff_recv_ident = syn::Ident::new(&format!("__hoff_recv_{hoff_id}"), Span::call_site());

        self.add_extra_stmt_internal(in_location, syn::parse_quote! {
            let (#hoff_send_ident, #hoff_recv_ident) = __root_dfir_rs::util::unsync::mpsc::unbounded();
        });
        let buffer_init: syn::Expr = match buffer {
            TickInputBuffer::Queue => syn::parse_quote!(::std::collections::VecDeque::new()),
            TickInputBuffer::KeyedQueues => {
                syn::parse_quote!(__root_dfir_rs::rustc_hash::FxHashMap::<
                    _,
                    ::std::collections::VecDeque<_>,
                >::default())
            }
        };
        self.add_extra_stmt_internal(
            in_location,
            syn::parse_quote! {
                let #buffered_ident = ::std::rc::Rc::new(::std::cell::RefCell::new(#buffer_init));
            },
        );

        TickInputPlumbing {
            hoff_id,
            buffer,
            buffered_ident,
            hoff_send_ident,
            hoff_recv_ident,
        }
    }

    /// Registers `inner_hook` as the tick-input hook for this boundary, wrapping it as a
    /// scripted hook when the operator's guard carries a sim hook handle.
    fn install_tick_input_hook(
        &mut self,
        in_location: &LocationId,
        out_location: &LocationId,
        plumbing: &TickInputPlumbing,
        sim_hook_id: Option<usize>,
        op_location: &str,
        inner_hook: syn::Expr,
    ) {
        if let Some(hook_id) = sim_hook_id {
            let hook_rc_ident = syn::Ident::new(
                &format!("__scripted_hook_{}", plumbing.hoff_id),
                Span::call_site(),
            );
            self.add_scripted_hook(
                hook_id,
                in_location,
                out_location,
                &hook_rc_ident,
                op_location,
                inner_hook,
            );
        } else {
            self.add_hook(
                in_location,
                out_location,
                syn::parse_quote!(Box::new(#inner_hook)),
            );
        }
    }

    /// Wires the top-level input into the hook's buffer and the hook's handoff into the tick's
    /// graph as the windowed output.
    fn connect_tick_input(
        &mut self,
        in_ident: syn::Ident,
        in_location: &LocationId,
        out_ident: &syn::Ident,
        out_location: &LocationId,
        plumbing: &TickInputPlumbing,
    ) {
        let TickInputPlumbing {
            buffered_ident,
            hoff_recv_ident,
            ..
        } = plumbing;
        let push: dfir_lang::parse::DfirCode = match plumbing.buffer {
            TickInputBuffer::Queue => parse_quote! {
                #in_ident -> for_each(|v| #buffered_ident.borrow_mut().push_back(v));
            },
            TickInputBuffer::KeyedQueues => parse_quote! {
                #in_ident -> for_each(|(k, v)| #buffered_ident.borrow_mut().entry(k).or_default().push_back(v));
            },
        };
        self.get_dfir_mut(in_location).add_dfir(push, None, None);

        self.get_dfir_mut(out_location).add_dfir(
            parse_quote! {
                #out_ident = source_stream(#hoff_recv_ident);
            },
            None,
            None,
        );
    }
}

/// The shape of the buffer a tick-input hook accumulates arriving items into.
#[derive(Clone, Copy)]
enum TickInputBuffer {
    /// A single `VecDeque` of items (streams, singletons, optionals).
    Queue,
    /// A `VecDeque` of values per key (keyed streams, keyed singletons).
    KeyedQueues,
}

/// Identifiers allocated by [`SimBuilder::tick_input_plumbing`].
struct TickInputPlumbing {
    hoff_id: HandoffId,
    buffer: TickInputBuffer,
    buffered_ident: syn::Ident,
    hoff_send_ident: syn::Ident,
    hoff_recv_ident: syn::Ident,
}

/// The tick a windowing operator targets: an atomic output location is unwrapped to its tick.
fn tick_of_out_location(out_location: &LocationId) -> &LocationId {
    if let LocationId::Atomic(tick) = out_location {
        tick.as_ref()
    } else {
        out_location
    }
}

fn stream_order_type(order: &StreamOrder) -> syn::Type {
    let root = get_this_crate();
    match order {
        StreamOrder::TotalOrder => {
            parse_quote! { #root::live_collections::stream::TotalOrder }
        }
        StreamOrder::NoOrder => {
            parse_quote! { #root::live_collections::stream::NoOrder }
        }
    }
}

impl DfirBuilder for SimBuilder {
    fn singleton_intermediates(&self) -> bool {
        true
    }

    fn add_dfir_at(
        &mut self,
        location: &LocationId,
        dfir: dfir_lang::parse::DfirCode,
        operator_tag: Option<&str>,
    ) {
        self.get_dfir_mut(location)
            .add_dfir(dfir, None, operator_tag);
    }

    /// Places the source directly in the tick's own graph.
    fn add_tick_source(
        &mut self,
        location: &LocationId,
        source_rhs: proc_macro2::TokenStream,
        out_ident: &syn::Ident,
        replay_each_tick: bool,
        operator_tag: Option<&str>,
    ) {
        let dfir = if replay_each_tick {
            parse_quote! {
                #out_ident = #source_rhs -> persist::<'static>();
            }
        } else {
            parse_quote! {
                #out_ident = #source_rhs;
            }
        };
        self.add_dfir_at(location, dfir, operator_tag);
    }

    fn batch(
        &mut self,
        in_ident: syn::Ident,
        in_location: &LocationId,
        in_kind: &CollectionKind,
        out_ident: &syn::Ident,
        out_location: &LocationId,
        op_meta: &HydroIrOpMetadata,
        _fold_hooked_idents: &HashSet<String>,
    ) {
        if let LocationId::Atomic(_) = in_location {
            self.enter_tick_from_atomic(in_ident, in_location, out_ident, op_meta, "batch_atomic");
            return;
        }

        let out_location = tick_of_out_location(out_location);
        let (batch_location, line, caret) = location_for_op(op_meta);
        let root = get_this_crate();

        match in_kind {
            CollectionKind::Stream {
                order,
                retry,
                element_type,
                ..
            } => {
                debug_assert!(in_location.is_top_level());
                let order_ty = stream_order_type(order);
                let plumbing = self.tick_input_plumbing(in_location, TickInputBuffer::Queue);
                let TickInputPlumbing {
                    buffered_ident,
                    hoff_send_ident,
                    ..
                } = &plumbing;

                let args: proc_macro2::TokenStream = quote::quote!(
                    #buffered_ident.clone(),
                    #hoff_send_ident,
                    #root::sim::runtime::HookLocationMeta { location: #batch_location, line: #line, caret_indent: #caret },
                    #root::__maybe_debug__!(#element_type),
                );
                let inner_hook: syn::Expr = match retry {
                    StreamRetry::ExactlyOnce => syn::parse_quote!(
                        #root::sim::runtime::StreamHook::<_, #order_ty, #root::live_collections::stream::ExactlyOnce>::new(#args)
                    ),
                    StreamRetry::AtLeastOnce => {
                        // Batching an at-least-once stream: an element may be delivered in
                        // several batches (a cross-tick redelivery), so the decision space
                        // is infinite.
                        let budget = self.at_least_once_budget(
                            op_meta,
                            &batch_location,
                            "the batching of an at-least-once stream (an element may be \
                             delivered again in a later batch)",
                        );
                        syn::parse_quote!(
                            #root::sim::runtime::StreamHook::<_, #order_ty, #root::live_collections::stream::AtLeastOnce>::new(#args #budget)
                        )
                    }
                };
                self.install_tick_input_hook(
                    in_location,
                    out_location,
                    &plumbing,
                    op_meta.sim_hook_id,
                    &batch_location,
                    inner_hook,
                );
                self.connect_tick_input(in_ident, in_location, out_ident, out_location, &plumbing);
            }
            CollectionKind::KeyedStream {
                value_order,
                value_retry,
                key_type,
                value_type,
                ..
            } => {
                debug_assert!(in_location.is_top_level());
                let order_ty = stream_order_type(value_order);
                let plumbing = self.tick_input_plumbing(in_location, TickInputBuffer::KeyedQueues);
                let TickInputPlumbing {
                    buffered_ident,
                    hoff_send_ident,
                    ..
                } = &plumbing;

                let args: proc_macro2::TokenStream = quote::quote!(
                    #buffered_ident.clone(),
                    #hoff_send_ident,
                    #root::sim::runtime::HookLocationMeta { location: #batch_location, line: #line, caret_indent: #caret },
                    #root::__maybe_debug__!((#key_type, #value_type)),
                );
                let inner_hook: syn::Expr = match value_retry {
                    StreamRetry::ExactlyOnce => syn::parse_quote!(
                        #root::sim::runtime::KeyedStreamHook::<_, _, #order_ty, #root::live_collections::stream::ExactlyOnce>::new(#args)
                    ),
                    StreamRetry::AtLeastOnce => {
                        // Batching a keyed stream with at-least-once values: an entry may be
                        // delivered in several batches (a cross-tick redelivery), so the
                        // decision space is infinite.
                        let budget = self.at_least_once_budget(
                            op_meta,
                            &batch_location,
                            "the batching of an at-least-once keyed stream (an entry may be \
                             delivered again in a later batch)",
                        );
                        syn::parse_quote!(
                            #root::sim::runtime::KeyedStreamHook::<_, _, #order_ty, #root::live_collections::stream::AtLeastOnce>::new(#args #budget)
                        )
                    }
                };
                self.install_tick_input_hook(
                    in_location,
                    out_location,
                    &plumbing,
                    op_meta.sim_hook_id,
                    &batch_location,
                    inner_hook,
                );
                self.connect_tick_input(in_ident, in_location, out_ident, out_location, &plumbing);
            }
            CollectionKind::KeyedSingleton {
                bound: KeyedSingletonBoundKind::BoundedValue | KeyedSingletonBoundKind::Bounded,
                key_type,
                value_type,
            } => {
                // Each key's value is final once it appears, so the keyed singleton is a feed
                // of finalized `(K, V)` entries that each arrive exactly once. A batch is the
                // set of entries that arrived since the previous batch, which is exactly how a
                // keyed stream with (at most) one value per key is batched. Per-key ordering
                // is vacuous with a single value per key, so the ordered hook (whose scripted
                // decisions name entries or per-key counts) is used.
                debug_assert!(in_location.is_top_level());
                let plumbing = self.tick_input_plumbing(in_location, TickInputBuffer::KeyedQueues);
                let TickInputPlumbing {
                    buffered_ident,
                    hoff_send_ident,
                    ..
                } = &plumbing;

                let inner_hook: syn::Expr = syn::parse_quote!(
                    #root::sim::runtime::KeyedStreamHook::<_, _, #root::live_collections::stream::TotalOrder, #root::live_collections::stream::ExactlyOnce>::new(
                        #buffered_ident.clone(),
                        #hoff_send_ident,
                        #root::sim::runtime::HookLocationMeta { location: #batch_location, line: #line, caret_indent: #caret },
                        #root::__maybe_debug__!((#key_type, #value_type)),
                    )
                );
                self.install_tick_input_hook(
                    in_location,
                    out_location,
                    &plumbing,
                    op_meta.sim_hook_id,
                    &batch_location,
                    inner_hook,
                );
                self.connect_tick_input(in_ident, in_location, out_ident, out_location, &plumbing);
            }
            CollectionKind::Singleton { .. }
            | CollectionKind::Optional { .. }
            | CollectionKind::KeyedSingleton {
                bound:
                    KeyedSingletonBoundKind::Unbounded
                    | KeyedSingletonBoundKind::MonotonicKeys
                    | KeyedSingletonBoundKind::MonotonicValue,
                ..
            } => {
                panic!(
                    "`Batch` is only defined for stream-like collections (streams, keyed streams, \
                     and keyed singletons with bounded per-key values), but was applied to a \
                     singleton-like collection {in_kind:?} (at {batch_location}); use `Snapshot` \
                     instead"
                );
            }
        }
    }

    fn snapshot(
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
            self.enter_tick_from_atomic(
                in_ident,
                in_location,
                out_ident,
                op_meta,
                "snapshot_atomic",
            );
            return;
        }

        let out_location = tick_of_out_location(out_location);
        let (batch_location, line, caret) = location_for_op(op_meta);
        let root = get_this_crate();

        match in_kind {
            CollectionKind::Singleton { element_type, .. } => {
                debug_assert!(in_location.is_top_level());
                let plumbing = self.tick_input_plumbing(in_location, TickInputBuffer::Queue);
                let TickInputPlumbing {
                    buffered_ident,
                    hoff_send_ident,
                    ..
                } = &plumbing;

                if fold_hooked_idents.contains(&in_ident.to_string()) {
                    assert!(
                        op_meta.sim_hook_id.is_none(),
                        "sim hooks are not yet supported on snapshots of top-level folds \
                         over unordered streams: the fold's input order is fuzzed \
                         separately (at {})",
                        batch_location
                    );
                    // The fold hook already controls when new values are produced.
                    // Use a PassthroughSingletonHook that always releases the latest
                    // value without non-deterministic decisions.
                    self.add_hook(
                        in_location,
                        out_location,
                        syn::parse_quote!(
                            Box::new(#root::sim::runtime::PassthroughSingletonHook::<_>::new(
                                #buffered_ident.clone(),
                                #hoff_send_ident,
                                #root::sim::runtime::HookLocationMeta { location: #batch_location, line: #line, caret_indent: #caret },
                                #root::__maybe_debug__!(#element_type),
                            ))
                        ),
                    );
                } else {
                    let inner_hook: syn::Expr = syn::parse_quote!(
                        #root::sim::runtime::SingletonHook::<_>::new(
                            #buffered_ident.clone(),
                            #hoff_send_ident,
                            #root::sim::runtime::HookLocationMeta { location: #batch_location, line: #line, caret_indent: #caret },
                            #root::__maybe_debug__!(#element_type),
                        )
                    );
                    self.install_tick_input_hook(
                        in_location,
                        out_location,
                        &plumbing,
                        op_meta.sim_hook_id,
                        &batch_location,
                        inner_hook,
                    );
                }

                self.connect_tick_input(in_ident, in_location, out_ident, out_location, &plumbing);
            }
            CollectionKind::KeyedSingleton {
                bound:
                    KeyedSingletonBoundKind::MonotonicKeys
                    | KeyedSingletonBoundKind::MonotonicValue
                    | KeyedSingletonBoundKind::Bounded,
                key_type,
                value_type,
            } => {
                // Keys are never removed, so each firing observes, for every key revealed so
                // far, some version of its value (a fully `Bounded` keyed singleton has a single
                // version per key, so the only choice is when each key is first revealed).
                debug_assert!(in_location.is_top_level());
                let plumbing = self.tick_input_plumbing(in_location, TickInputBuffer::KeyedQueues);
                let TickInputPlumbing {
                    buffered_ident,
                    hoff_send_ident,
                    ..
                } = &plumbing;

                let inner_hook: syn::Expr = syn::parse_quote!(
                    #root::sim::runtime::KeyedSingletonHook::<_, _>::new(
                        #buffered_ident.clone(),
                        #hoff_send_ident,
                        #root::sim::runtime::HookLocationMeta { location: #batch_location, line: #line, caret_indent: #caret },
                        #root::__maybe_debug__!(#key_type),
                        #root::__maybe_debug__!(#value_type),
                    )
                );
                self.install_tick_input_hook(
                    in_location,
                    out_location,
                    &plumbing,
                    op_meta.sim_hook_id,
                    &batch_location,
                    inner_hook,
                );
                self.connect_tick_input(in_ident, in_location, out_ident, out_location, &plumbing);
            }
            CollectionKind::KeyedSingleton {
                bound: KeyedSingletonBoundKind::Unbounded,
                ..
            } => {
                todo!(
                    "Simulation of Unbounded keyed singletons is not yet supported. \
                     Keys may be removed in Unbounded keyed singletons, which the simulator \
                     cannot currently model. Use a fold (which gives MonotonicKeys) or \
                     another operator that guarantees keys are never removed."
                );
            }
            CollectionKind::Optional {
                bound: OptionalBoundKind::InitNone,
                element_type,
                ..
            } => {
                // Only `InitNone` optionals (null prefix, then monotone presence) are
                // supported: their monotone presence is what `OptionalInitNoneHook` models. A general
                // `Unbounded` optional can return to null, which this hook does not represent,
                // so it stays rejected below.
                debug_assert!(in_location.is_top_level());
                let plumbing = self.tick_input_plumbing(in_location, TickInputBuffer::Queue);
                let TickInputPlumbing {
                    buffered_ident,
                    hoff_send_ident,
                    ..
                } = &plumbing;

                self.add_hook(
                    in_location,
                    out_location,
                    syn::parse_quote!(
                        Box::new(#root::sim::runtime::OptionalInitNoneHook::<_>::new(
                            #buffered_ident.clone(),
                            #hoff_send_ident,
                            #root::sim::runtime::HookLocationMeta { location: #batch_location, line: #line, caret_indent: #caret },
                            #root::__maybe_debug__!(#element_type),
                        ))
                    ),
                );
                self.connect_tick_input(in_ident, in_location, out_ident, out_location, &plumbing);
            }
            CollectionKind::Stream { .. }
            | CollectionKind::KeyedStream { .. }
            | CollectionKind::KeyedSingleton {
                bound: KeyedSingletonBoundKind::BoundedValue,
                ..
            } => {
                panic!(
                    "`Snapshot` is only defined for singleton-like collections (singletons, \
                     optionals, and keyed singletons whose per-key values may change), but was \
                     applied to a stream-like collection {in_kind:?} (at {batch_location}); use \
                     `Batch` instead"
                );
            }
            CollectionKind::Optional {
                bound: OptionalBoundKind::Unbounded | OptionalBoundKind::Bounded,
                ..
            } => {
                eprintln!("{:?}", op_meta.backtrace.elements().collect::<Vec<_>>());
                todo!("snapshot not implemented for kind {:?}", in_kind)
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
        _out_kind: &CollectionKind,
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
                        let (#hoff_send_ident, #hoff_recv_ident) = __root_dfir_rs::util::unsync::mpsc::unbounded();
                    });

                    self.get_dfir_mut(in_location).add_dfir(
                        parse_quote! {
                            #in_ident -> for_each(|v| #hoff_send_ident.try_send(v).unwrap());
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
                    // NOTE: `Optional::latest()` is non-monotone (it reflects the latest tick's
                    // value, "including whether the optional is null or not"), so it cannot be
                    // modeled by the monotone `OptionalInitNoneHook`. Simulating it soundly needs a
                    // representation that conveys per-tick nullness, which is not yet implemented.
                    // (`Singleton::latest()` lowering yields via the `Singleton` arm above, so it
                    // does not depend on this path.)
                    todo!("Non-atomic yield of an Optional is not yet supported");
                }
            }
            o => todo!("Not yet supported, yield collection type {:?}", o),
        }
    }

    /// This implementation is the identity: simulation does not emit production
    /// `loop { ... }` contexts, so no un-windowing operator is required.
    fn unwindow_for_consume(
        &mut self,
        in_ident: syn::Ident,
        _in_location: &LocationId,
        _out_location: &LocationId,
    ) -> syn::Ident {
        in_ident
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
        out_kind: &CollectionKind,
    ) {
        if let LocationId::Atomic(tick) = in_location
            && let LocationId::Tick {
                tick: _,
                parent_location,
            } = tick.as_ref()
        {
            self.yield_from_tick(
                in_ident,
                in_location,
                in_kind,
                out_ident,
                parent_location.as_ref(),
                out_kind,
            );
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
                        let (#hoff_send_ident, #hoff_recv_ident) = __root_dfir_rs::util::unsync::mpsc::unbounded();
                    });

                    self.add_extra_stmt_internal(location.root(), syn::parse_quote! {
                        let #hoff_recv_ident = ::std::rc::Rc::new(::std::cell::RefCell::new(#hoff_recv_ident));
                    });

                    self.add_extra_stmt_internal(location.root(), syn::parse_quote! {
                        let #buffered_ident = ::std::rc::Rc::new(::std::cell::RefCell::new(None));
                    });

                    let inner_hook: syn::Expr = syn::parse_quote!(
                        #root::sim::runtime::StreamOrderHook::<_>::new(
                            #buffered_ident.clone(),
                            #hoff_send_ident,
                            #root::sim::runtime::HookLocationMeta { location: #assume_location, line: #line, caret_indent: #caret },
                            #root::__maybe_debug__!(#element_type),
                        )
                    );
                    if let Some(hook_id) = op_meta.sim_hook_id {
                        let hook_rc_ident = syn::Ident::new(
                            &format!("__scripted_inline_hook_{hoff_id}"),
                            Span::call_site(),
                        );
                        self.add_scripted_inline_hook(
                            hook_id,
                            location,
                            &hook_rc_ident,
                            &assume_location,
                            inner_hook,
                        );
                    } else {
                        self.add_inline_hook(location, syn::parse_quote!(Box::new(#inner_hook)));
                    }

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
                        let (#hoff_send_ident, #hoff_recv_ident) = __root_dfir_rs::util::unsync::mpsc::unbounded();
                    });

                    self.add_extra_stmt_internal(location.root(), syn::parse_quote! {
                        let #hoff_recv_ident = ::std::rc::Rc::new(::std::cell::RefCell::new(#hoff_recv_ident));
                    });

                    self.add_extra_stmt_internal(location.root(), syn::parse_quote! {
                        let #buffered_ident = ::std::rc::Rc::new(::std::cell::RefCell::new(None));
                    });

                    let inner_hook: syn::Expr = syn::parse_quote!(
                        #root::sim::runtime::KeyedStreamOrderHook::<_, _>::new(
                            #buffered_ident.clone(),
                            #hoff_send_ident,
                            #root::sim::runtime::HookLocationMeta { location: #assume_location, line: #line, caret_indent: #caret },
                            #root::__maybe_debug__!(#key_type),
                            #root::__maybe_debug__!(#value_type),
                        )
                    );
                    if let Some(hook_id) = op_meta.sim_hook_id {
                        let hook_rc_ident = syn::Ident::new(
                            &format!("__scripted_inline_hook_{hoff_id}"),
                            Span::call_site(),
                        );
                        self.add_scripted_inline_hook(
                            hook_id,
                            location,
                            &hook_rc_ident,
                            &assume_location,
                            inner_hook,
                        );
                    } else {
                        self.add_inline_hook(location, syn::parse_quote!(Box::new(#inner_hook)));
                    }

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
                        let (#hoff_send_ident, #hoff_recv_ident) = __root_dfir_rs::util::unsync::mpsc::unbounded();
                    });

                    self.add_extra_stmt_internal(location.root(), syn::parse_quote! {
                        let #hoff_recv_ident = ::std::rc::Rc::new(::std::cell::RefCell::new(#hoff_recv_ident));
                    });

                    self.add_extra_stmt_internal(location.root(), syn::parse_quote! {
                        let #buffered_ident = ::std::rc::Rc::new(::std::cell::RefCell::new(None));
                    });

                    let inner_hook: syn::Expr = syn::parse_quote!(
                        #root::sim::runtime::PartiallyOrderedStreamHook::<_, _>::new(
                            #buffered_ident.clone(),
                            #hoff_send_ident,
                            #root::sim::runtime::HookLocationMeta { location: #assume_location, line: #line, caret_indent: #caret },
                            #root::__maybe_debug__!(#key_type),
                            #root::__maybe_debug__!(#value_type),
                        )
                    );
                    if let Some(hook_id) = op_meta.sim_hook_id {
                        let hook_rc_ident = syn::Ident::new(
                            &format!("__scripted_inline_hook_{hoff_id}"),
                            Span::call_site(),
                        );
                        self.add_scripted_inline_hook(
                            hook_id,
                            location,
                            &hook_rc_ident,
                            &assume_location,
                            inner_hook,
                        );
                    } else {
                        self.add_inline_hook(location, syn::parse_quote!(Box::new(#inner_hook)));
                    }

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
                    CollectionKind::Stream {
                        order: in_order @ (StreamOrder::NoOrder | StreamOrder::TotalOrder),
                        retry: StreamRetry::AtLeastOnce,
                        element_type,
                        ..
                    },
                    CollectionKind::Stream {
                        order: out_order,
                        retry: StreamRetry::ExactlyOnce,
                        ..
                    },
                ) if in_order == out_order => {
                    // `assume_retries`: the point where the simulator injects the
                    // duplicates the `AtLeastOnce` type says downstream must tolerate.
                    let budget = self.at_least_once_budget(
                        op_meta,
                        &assume_location,
                        "retries (`assume_retries`)",
                    );

                    let hoff_id = self.next_hoff_id.get_and_increment();

                    let buffered_ident =
                        syn::Ident::new(&format!("__buffered_{hoff_id}"), Span::call_site());
                    let hoff_send_ident =
                        syn::Ident::new(&format!("__hoff_send_{hoff_id}"), Span::call_site());
                    let hoff_recv_ident =
                        syn::Ident::new(&format!("__hoff_recv_{hoff_id}"), Span::call_site());

                    self.add_extra_stmt_internal(location.root(), syn::parse_quote! {
                        let (#hoff_send_ident, #hoff_recv_ident) = __root_dfir_rs::util::unsync::mpsc::unbounded();
                    });

                    self.add_extra_stmt_internal(location.root(), syn::parse_quote! {
                        let #hoff_recv_ident = ::std::rc::Rc::new(::std::cell::RefCell::new(#hoff_recv_ident));
                    });

                    self.add_extra_stmt_internal(location.root(), syn::parse_quote! {
                        let #buffered_ident = ::std::rc::Rc::new(::std::cell::RefCell::new(None));
                    });

                    let hook_ty: syn::Path = if *in_order == StreamOrder::TotalOrder {
                        syn::parse_quote!(#root::sim::runtime::OrderedStreamRetriesHook)
                    } else {
                        syn::parse_quote!(#root::sim::runtime::NoOrderStreamRetriesHook)
                    };
                    let inner_hook: syn::Expr = syn::parse_quote!(
                        #hook_ty::<_>::new(
                            #buffered_ident.clone(),
                            #hoff_send_ident,
                            #root::sim::runtime::HookLocationMeta { location: #assume_location, line: #line, caret_indent: #caret },
                            #root::__maybe_debug__!(#element_type),
                            #budget,
                        )
                    );
                    if let Some(hook_id) = op_meta.sim_hook_id {
                        let hook_rc_ident = syn::Ident::new(
                            &format!("__scripted_inline_hook_{hoff_id}"),
                            Span::call_site(),
                        );
                        self.add_scripted_inline_hook(
                            hook_id,
                            location,
                            &hook_rc_ident,
                            &assume_location,
                            inner_hook,
                        );
                    } else {
                        self.add_inline_hook(location, syn::parse_quote!(Box::new(#inner_hook)));
                    }

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
                    CollectionKind::Stream {
                        order: StreamOrder::NoOrder,
                        retry: StreamRetry::AtLeastOnce,
                        element_type,
                        ..
                    },
                    CollectionKind::Stream {
                        order: StreamOrder::TotalOrder,
                        retry: StreamRetry::AtLeastOnce,
                        ..
                    },
                ) => {
                    // `assume_ordering` on an at-least-once stream: ordering also decides
                    // which slots each element's retries occupy (an element may be emitted
                    // into several slots), so the decision space is infinite.
                    let budget = self.at_least_once_budget(
                        op_meta,
                        &assume_location,
                        "the order of an at-least-once stream (which also decides which slots \
                         each element's retries occupy)",
                    );

                    let hoff_id = self.next_hoff_id.get_and_increment();

                    let buffered_ident =
                        syn::Ident::new(&format!("__buffered_{hoff_id}"), Span::call_site());
                    let hoff_send_ident =
                        syn::Ident::new(&format!("__hoff_send_{hoff_id}"), Span::call_site());
                    let hoff_recv_ident =
                        syn::Ident::new(&format!("__hoff_recv_{hoff_id}"), Span::call_site());

                    self.add_extra_stmt_internal(location.root(), syn::parse_quote! {
                        let (#hoff_send_ident, #hoff_recv_ident) = __root_dfir_rs::util::unsync::mpsc::unbounded();
                    });

                    self.add_extra_stmt_internal(location.root(), syn::parse_quote! {
                        let #hoff_recv_ident = ::std::rc::Rc::new(::std::cell::RefCell::new(#hoff_recv_ident));
                    });

                    self.add_extra_stmt_internal(location.root(), syn::parse_quote! {
                        let #buffered_ident = ::std::rc::Rc::new(::std::cell::RefCell::new(None));
                    });

                    let inner_hook: syn::Expr = syn::parse_quote!(
                        #root::sim::runtime::AtLeastOnceStreamOrderHook::<_>::new(
                            #buffered_ident.clone(),
                            #hoff_send_ident,
                            #root::sim::runtime::HookLocationMeta { location: #assume_location, line: #line, caret_indent: #caret },
                            #root::__maybe_debug__!(#element_type),
                            #budget,
                        )
                    );
                    if let Some(hook_id) = op_meta.sim_hook_id {
                        let hook_rc_ident = syn::Ident::new(
                            &format!("__scripted_inline_hook_{hoff_id}"),
                            Span::call_site(),
                        );
                        self.add_scripted_inline_hook(
                            hook_id,
                            location,
                            &hook_rc_ident,
                            &assume_location,
                            inner_hook,
                        );
                    } else {
                        self.add_inline_hook(location, syn::parse_quote!(Box::new(#inner_hook)));
                    }

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
                        let (#hoff_send_ident, #hoff_recv_ident) = __root_dfir_rs::util::unsync::mpsc::unbounded();
                    });
                    self.add_extra_stmt_internal(location, syn::parse_quote! {
                        let #buffered_ident = ::std::rc::Rc::new(::std::cell::RefCell::new(::std::collections::VecDeque::new()));
                    });
                    let inner_hook: syn::Expr = syn::parse_quote!(
                        #root::sim::runtime::TopLevelStreamOrderHook::<_> {
                            input: #buffered_ident.clone(),
                            to_release: None,
                            output: #hoff_send_ident,
                            location: #root::sim::runtime::HookLocationMeta { location: #assume_location, line: #line, caret_indent: #caret },
                            format_item_debug: #root::__maybe_debug__!(#element_type),
                        }
                    );
                    if let Some(hook_id) = op_meta.sim_hook_id {
                        let hook_rc_ident = syn::Ident::new(
                            &format!("__scripted_observation_hook_{hoff_id}"),
                            Span::call_site(),
                        );
                        self.add_scripted_hook(
                            hook_id,
                            location,
                            location,
                            &hook_rc_ident,
                            &assume_location,
                            inner_hook,
                        );
                    } else {
                        self.add_hook(location, location, syn::parse_quote!(Box::new(#inner_hook)));
                    }

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
                        let (#hoff_send_ident, #hoff_recv_ident) = __root_dfir_rs::util::unsync::mpsc::unbounded();
                    });
                    self.add_extra_stmt_internal(location, syn::parse_quote! {
                        let #buffered_ident = ::std::rc::Rc::new(::std::cell::RefCell::new(__root_dfir_rs::rustc_hash::FxHashMap::default()));
                    });
                    let inner_hook: syn::Expr = syn::parse_quote!(
                        #root::sim::runtime::TopLevelKeyedStreamOrderHook::<_, _> {
                            input: #buffered_ident.clone(),
                            to_release: None,
                            output: #hoff_send_ident,
                            location: #root::sim::runtime::HookLocationMeta { location: #assume_location, line: #line, caret_indent: #caret },
                            format_item_debug: #root::__maybe_debug__!((#key_type, #value_type)),
                        }
                    );
                    if let Some(hook_id) = op_meta.sim_hook_id {
                        let hook_rc_ident = syn::Ident::new(
                            &format!("__scripted_observation_hook_{hoff_id}"),
                            Span::call_site(),
                        );
                        self.add_scripted_hook(
                            hook_id,
                            location,
                            location,
                            &hook_rc_ident,
                            &assume_location,
                            inner_hook,
                        );
                    } else {
                        self.add_hook(location, location, syn::parse_quote!(Box::new(#inner_hook)));
                    }

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
                        let (#hoff_send_ident, #hoff_recv_ident) = __root_dfir_rs::util::unsync::mpsc::unbounded();
                    });
                    self.add_extra_stmt_internal(location, syn::parse_quote! {
                        let #buffered_ident = ::std::rc::Rc::new(::std::cell::RefCell::new(__root_dfir_rs::rustc_hash::FxHashMap::default()));
                    });
                    let inner_hook: syn::Expr = syn::parse_quote!(
                        #root::sim::runtime::TopLevelPartiallyOrderedStreamHook::<_, _> {
                            input: #buffered_ident.clone(),
                            to_release: None,
                            output: #hoff_send_ident,
                            location: #root::sim::runtime::HookLocationMeta { location: #assume_location, line: #line, caret_indent: #caret },
                            format_item_debug: #root::__maybe_debug__!((#key_type, #value_type)),
                        }
                    );
                    if let Some(hook_id) = op_meta.sim_hook_id {
                        let hook_rc_ident = syn::Ident::new(
                            &format!("__scripted_observation_hook_{hoff_id}"),
                            Span::call_site(),
                        );
                        self.add_scripted_hook(
                            hook_id,
                            location,
                            location,
                            &hook_rc_ident,
                            &assume_location,
                            inner_hook,
                        );
                    } else {
                        self.add_hook(location, location, syn::parse_quote!(Box::new(#inner_hook)));
                    }

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
                    CollectionKind::Stream {
                        order: in_order @ (StreamOrder::NoOrder | StreamOrder::TotalOrder),
                        retry: StreamRetry::AtLeastOnce,
                        element_type,
                        ..
                    },
                    CollectionKind::Stream {
                        order: out_order,
                        retry: StreamRetry::ExactlyOnce,
                        ..
                    },
                ) if in_order == out_order => {
                    // `assume_retries`: the point where the simulator injects the
                    // duplicates the `AtLeastOnce` type says downstream must tolerate.
                    let budget = self.at_least_once_budget(
                        op_meta,
                        &assume_location,
                        "retries (`assume_retries`)",
                    );

                    let hoff_id = self.next_hoff_id.get_and_increment();

                    let buffered_ident =
                        syn::Ident::new(&format!("__buffered_{hoff_id}"), Span::call_site());
                    let hoff_send_ident =
                        syn::Ident::new(&format!("__hoff_send_{hoff_id}"), Span::call_site());
                    let hoff_recv_ident =
                        syn::Ident::new(&format!("__hoff_recv_{hoff_id}"), Span::call_site());

                    self.add_extra_stmt_internal(location, syn::parse_quote! {
                        let (#hoff_send_ident, #hoff_recv_ident) = __root_dfir_rs::util::unsync::mpsc::unbounded();
                    });
                    self.add_extra_stmt_internal(location, syn::parse_quote! {
                        let #buffered_ident = ::std::rc::Rc::new(::std::cell::RefCell::new(::std::collections::VecDeque::new()));
                    });
                    let hook_ty: syn::Path = if *in_order == StreamOrder::TotalOrder {
                        syn::parse_quote!(#root::sim::runtime::TopLevelOrderedStreamRetriesHook)
                    } else {
                        syn::parse_quote!(#root::sim::runtime::TopLevelNoOrderStreamRetriesHook)
                    };
                    let extra_fields: proc_macro2::TokenStream =
                        if *in_order == StreamOrder::TotalOrder {
                            quote::quote!(release_provisional: false, front_started: false,)
                        } else {
                            Default::default()
                        };
                    let inner_hook: syn::Expr = syn::parse_quote!(
                        #hook_ty::<_> {
                            input: #buffered_ident.clone(),
                            to_release: None,
                            #extra_fields
                            budget: #budget,
                            output: #hoff_send_ident,
                            location: #root::sim::runtime::HookLocationMeta { location: #assume_location, line: #line, caret_indent: #caret },
                            format_item_debug: #root::__maybe_debug__!(#element_type),
                        }
                    );
                    if let Some(hook_id) = op_meta.sim_hook_id {
                        let hook_rc_ident = syn::Ident::new(
                            &format!("__scripted_observation_hook_{hoff_id}"),
                            Span::call_site(),
                        );
                        self.add_scripted_hook(
                            hook_id,
                            location,
                            location,
                            &hook_rc_ident,
                            &assume_location,
                            inner_hook,
                        );
                    } else {
                        self.add_hook(location, location, syn::parse_quote!(Box::new(#inner_hook)));
                    }

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
                    CollectionKind::Stream {
                        order: StreamOrder::NoOrder,
                        retry: StreamRetry::AtLeastOnce,
                        element_type,
                        ..
                    },
                    CollectionKind::Stream {
                        order: StreamOrder::TotalOrder,
                        retry: StreamRetry::AtLeastOnce,
                        ..
                    },
                ) => {
                    // `assume_ordering` on an at-least-once stream: ordering also decides
                    // which slots each element's retries occupy (an element may be emitted
                    // into several slots), so the decision space is infinite.
                    let budget = self.at_least_once_budget(
                        op_meta,
                        &assume_location,
                        "the order of an at-least-once stream (which also decides which slots \
                         each element's retries occupy)",
                    );

                    let hoff_id = self.next_hoff_id.get_and_increment();

                    let buffered_ident =
                        syn::Ident::new(&format!("__buffered_{hoff_id}"), Span::call_site());
                    let hoff_send_ident =
                        syn::Ident::new(&format!("__hoff_send_{hoff_id}"), Span::call_site());
                    let hoff_recv_ident =
                        syn::Ident::new(&format!("__hoff_recv_{hoff_id}"), Span::call_site());

                    self.add_extra_stmt_internal(location, syn::parse_quote! {
                        let (#hoff_send_ident, #hoff_recv_ident) = __root_dfir_rs::util::unsync::mpsc::unbounded();
                    });
                    self.add_extra_stmt_internal(location, syn::parse_quote! {
                        let #buffered_ident = ::std::rc::Rc::new(::std::cell::RefCell::new(::std::collections::VecDeque::new()));
                    });
                    let inner_hook: syn::Expr = syn::parse_quote!(
                        #root::sim::runtime::TopLevelAtLeastOnceOrderHook::<_> {
                            input: #buffered_ident.clone(),
                            to_release: None,
                            release_provisional: false,
                            last_emitted: None,
                            emitted_counts: ::std::vec::Vec::new(),
                            budget: #budget,
                            last_emitted_index: None,
                            output: #hoff_send_ident,
                            location: #root::sim::runtime::HookLocationMeta { location: #assume_location, line: #line, caret_indent: #caret },
                            format_item_debug: #root::__maybe_debug__!(#element_type),
                        }
                    );
                    if let Some(hook_id) = op_meta.sim_hook_id {
                        let hook_rc_ident = syn::Ident::new(
                            &format!("__scripted_observation_hook_{hoff_id}"),
                            Span::call_site(),
                        );
                        self.add_scripted_hook(
                            hook_id,
                            location,
                            location,
                            &hook_rc_ident,
                            &assume_location,
                            inner_hook,
                        );
                    } else {
                        self.add_hook(location, location, syn::parse_quote!(Box::new(#inner_hook)));
                    }

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

        // A `KeyedStream` only guarantees ordering *within* each key. A plain
        // stream merge over the `(K, V)` pairs already preserves each input's
        // order (and hence each key's order), so it would be correct here too.
        // Using dedicated keyed hooks is an optimization tailored to keyed
        // streams: since cross-key order is unobservable, they interleave the two
        // inputs independently per key and explore those per-key orderings
        // directly, rather than global interleavings that differ only in
        // unobservable cross-key order.
        let is_keyed = matches!(in_kind, CollectionKind::KeyedStream { .. });

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
                let (#hoff_send_ident, #hoff_recv_ident) = __root_dfir_rs::util::unsync::mpsc::unbounded();
            });

            self.add_extra_stmt_internal(location.root(), syn::parse_quote! {
                let #hoff_recv_ident = ::std::rc::Rc::new(::std::cell::RefCell::new(#hoff_recv_ident));
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

            if is_keyed {
                let inner_hook: syn::Expr = syn::parse_quote!(
                    #root::sim::runtime::KeyedMergeOrderedHook::<_, _>::new(
                        #buffered_first_ident.clone(),
                        #buffered_second_ident.clone(),
                        #hoff_send_ident,
                        #root::sim::runtime::HookLocationMeta { location: #assume_location, line: #line, caret_indent: #caret },
                        #root::__maybe_debug__!(#element_type),
                    )
                );
                if let Some(hook_id) = op_meta.sim_hook_id {
                    let hook_rc_ident = syn::Ident::new(
                        &format!("__scripted_inline_hook_{hoff_id}"),
                        Span::call_site(),
                    );
                    self.add_scripted_inline_hook(
                        hook_id,
                        location,
                        &hook_rc_ident,
                        &assume_location,
                        inner_hook,
                    );
                } else {
                    self.add_inline_hook(location, syn::parse_quote!(Box::new(#inner_hook)));
                }
            } else {
                let inner_hook: syn::Expr = syn::parse_quote!(
                    #root::sim::runtime::MergeOrderedHook::<_>::new(
                        #buffered_first_ident.clone(),
                        #buffered_second_ident.clone(),
                        #hoff_send_ident,
                        #root::sim::runtime::HookLocationMeta { location: #assume_location, line: #line, caret_indent: #caret },
                        #root::__maybe_debug__!(#element_type),
                    )
                );
                if let Some(hook_id) = op_meta.sim_hook_id {
                    let hook_rc_ident = syn::Ident::new(
                        &format!("__scripted_inline_hook_{hoff_id}"),
                        Span::call_site(),
                    );
                    self.add_scripted_inline_hook(
                        hook_id,
                        location,
                        &hook_rc_ident,
                        &assume_location,
                        inner_hook,
                    );
                } else {
                    self.add_inline_hook(location, syn::parse_quote!(Box::new(#inner_hook)));
                }
            }

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
                let (#hoff_send_ident, #hoff_recv_ident) = __root_dfir_rs::util::unsync::mpsc::unbounded();
            });

            if is_keyed {
                self.add_extra_stmt_internal(location, syn::parse_quote! {
                    let #buffered_first_ident = ::std::rc::Rc::new(::std::cell::RefCell::new(__root_dfir_rs::rustc_hash::FxHashMap::default()));
                });
                self.add_extra_stmt_internal(location, syn::parse_quote! {
                    let #buffered_second_ident = ::std::rc::Rc::new(::std::cell::RefCell::new(__root_dfir_rs::rustc_hash::FxHashMap::default()));
                });
                let inner_hook: syn::Expr = syn::parse_quote!(
                    #root::sim::runtime::TopLevelKeyedMergeOrderedHook::<_, _> {
                        first: #buffered_first_ident.clone(),
                        second: #buffered_second_ident.clone(),
                        to_release: None,
                        release_source: None,
                        output: #hoff_send_ident,
                        location: #root::sim::runtime::HookLocationMeta { location: #assume_location, line: #line, caret_indent: #caret },
                        format_item_debug: #root::__maybe_debug__!(#element_type),
                    }
                );
                if let Some(hook_id) = op_meta.sim_hook_id {
                    let hook_rc_ident = syn::Ident::new(
                        &format!("__scripted_observation_hook_{hoff_id}"),
                        Span::call_site(),
                    );
                    self.add_scripted_hook(
                        hook_id,
                        location,
                        location,
                        &hook_rc_ident,
                        &assume_location,
                        inner_hook,
                    );
                } else {
                    self.add_hook(location, location, syn::parse_quote!(Box::new(#inner_hook)));
                }

                self.get_dfir_mut(location).add_dfir(
                    parse_quote! {
                        #first_ident -> for_each(|(k, v)| #buffered_first_ident.borrow_mut().entry(k).or_insert_with(::std::collections::VecDeque::new).push_back(v));
                    },
                    None,
                    None,
                );

                self.get_dfir_mut(location).add_dfir(
                    parse_quote! {
                        #second_ident -> for_each(|(k, v)| #buffered_second_ident.borrow_mut().entry(k).or_insert_with(::std::collections::VecDeque::new).push_back(v));
                    },
                    None,
                    None,
                );
            } else {
                self.add_extra_stmt_internal(location, syn::parse_quote! {
                    let #buffered_first_ident = ::std::rc::Rc::new(::std::cell::RefCell::new(::std::collections::VecDeque::new()));
                });
                self.add_extra_stmt_internal(location, syn::parse_quote! {
                    let #buffered_second_ident = ::std::rc::Rc::new(::std::cell::RefCell::new(::std::collections::VecDeque::new()));
                });
                let inner_hook: syn::Expr = syn::parse_quote!(
                    #root::sim::runtime::TopLevelMergeOrderedHook::<_> {
                        first: #buffered_first_ident.clone(),
                        second: #buffered_second_ident.clone(),
                        to_release: None,
                        release_source: None,
                        output: #hoff_send_ident,
                        location: #root::sim::runtime::HookLocationMeta { location: #assume_location, line: #line, caret_indent: #caret },
                        format_item_debug: #root::__maybe_debug__!(#element_type),
                    }
                );
                if let Some(hook_id) = op_meta.sim_hook_id {
                    let hook_rc_ident = syn::Ident::new(
                        &format!("__scripted_observation_hook_{hoff_id}"),
                        Span::call_site(),
                    );
                    self.add_scripted_hook(
                        hook_id,
                        location,
                        location,
                        &hook_rc_ident,
                        &assume_location,
                        inner_hook,
                    );
                } else {
                    self.add_hook(location, location, syn::parse_quote!(Box::new(#inner_hook)));
                }

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
            }

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
        external_element_type: Option<&syn::Type>,
        tag_id: StmtId,
        networking_info: &crate::networking::NetworkingInfo,
    ) {
        use crate::compile::ir::StreamOrder;
        use crate::location::dynamic::ClusterConsistency;
        use crate::networking::{NetworkingInfo, TcpFault, UdpFault};
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
            NetworkingInfo::Udp { fault } => match fault {
                UdpFault::LossyDelayedForever => {
                    assert!(
                        self.test_safety_only,
                        "Simulating `lossy_delayed_forever` requires `.test_safety_only()` on the \
                         SimFlow because the simulator models dropped messages as indefinitely \
                         delayed, which only tests safety (not liveness). Call \
                         `.sim().test_safety_only()` to opt in."
                    );
                }
                _ => todo!("SimBuilder only supports lossy-delayed-forever UDP networking"),
            },
            // An `EMBEDDED` channel is simulated by the guarantees it declares: ordered and
            // eventually consistent like a fail-stop TCP channel, or unordered and eventually
            // consistent like a lossy-delayed-forever one.
            NetworkingInfo::Embedded {
                ordering,
                consistency,
            } => match (ordering, consistency) {
                (StreamOrder::TotalOrder, ClusterConsistency::EventualConsistency) => {}
                (StreamOrder::NoOrder, ClusterConsistency::EventualConsistency) => {
                    assert!(
                        self.test_safety_only,
                        "Simulating `lossy_delayed_forever` requires `.test_safety_only()` on the \
                         SimFlow because the simulator models dropped messages as indefinitely \
                         delayed, which only tests safety (not liveness). Call \
                         `.sim().test_safety_only()` to opt in."
                    );
                }
                (_, ClusterConsistency::NoConsistency) => todo!(
                    "SimBuilder only supports eventually consistent EMBEDDED networking \
                     (`fail_stop()` and `lossy_delayed_forever()`)"
                ),
            },
        }

        let root = get_this_crate();

        // For embedded (external) serialization, the raw payload type flows across the in-memory
        // channel instead of serialized `Bytes`.
        let payload: syn::Type = match external_element_type {
            Some(ty) => ty.clone(),
            None => parse_quote!(__root_dfir_rs::bytes::Bytes),
        };

        // Bincode channels wrap the received value in a transport `Result` (matching real
        // deployments) which the bincode deserialize expression then unwraps. Embedded channels
        // deliver the raw payload directly, so no such wrapper is inserted.
        let ok_wrap: proc_macro2::TokenStream = if external_element_type.is_some() {
            proc_macro2::TokenStream::new()
        } else {
            quote::quote!(-> map(|v| -> ::std::result::Result<_, ()> { Ok(v) }))
        };

        match (from, to) {
            (LocationId::Process(_), LocationId::Process(_)) => {
                self.extra_stmts_global.push(syn::parse_quote! {
                    let (#sink, #source) = __root_dfir_rs::util::unsync::mpsc::unbounded::<#payload>();
                });

                if let Some(serialize_pipeline) = serialize {
                    self.get_dfir_mut(from).add_dfir(
                        parse_quote! {
                            #input_ident -> map(#serialize_pipeline) -> for_each(|v| #sink.try_send(v).unwrap());
                        },
                        None,
                        Some(&format!("send{}", tag_id)),
                    );
                } else {
                    self.get_dfir_mut(from).add_dfir(
                        parse_quote! {
                            #input_ident -> for_each(|v| #sink.try_send(v).unwrap());
                        },
                        None,
                        Some(&format!("send{}", tag_id)),
                    );
                }

                if let Some(deserialize_pipeline) = deserialize {
                    self.get_dfir_mut(to).add_dfir(
                        parse_quote! {
                            #out_ident = source_stream(#source) #ok_wrap -> map(#deserialize_pipeline);
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
                    let (#sink, #source) = __root_dfir_rs::util::unsync::mpsc::unbounded::<(#root::__staged::location::TaglessMemberId, #payload)>();
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
                            #input_ident -> map(#serialize_pipeline) -> for_each(|v| #sink.try_send((#root::__staged::location::TaglessMemberId::from_raw_id(__current_cluster_id), v)).unwrap());
                        },
                        None,
                        Some(&format!("send{}", tag_id)),
                    );
                } else {
                    self.get_dfir_mut(from).add_dfir(
                        parse_quote! {
                            #input_ident -> for_each(|v| #sink.try_send((#root::__staged::location::TaglessMemberId::from_raw_id(__current_cluster_id), v)).unwrap());
                        },
                        None,
                        Some(&format!("send{}", tag_id)),
                    );
                }

                if let Some(deserialize_pipeline) = deserialize {
                    self.get_dfir_mut(to).add_dfir(
                        parse_quote! {
                            #out_ident = source_stream(#source) #ok_wrap -> map(#deserialize_pipeline);
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
                    let #sink: ::std::rc::Rc<::std::cell::RefCell<Vec<__root_dfir_rs::util::unsync::mpsc::Sender<#payload>>>> = ::std::rc::Rc::new(::std::cell::RefCell::new(Vec::new()));
                });

                self.extra_stmts_global.push(syn::parse_quote! {
                    let #sink_writer = #sink.clone();
                });

                self.extra_stmts_cluster
                    .entry(to.clone())
                    .or_default()
                    .push(syn::parse_quote! {
                        let #source = {
                            let (__sink, __source) = __root_dfir_rs::util::unsync::mpsc::unbounded::<#payload>();
                            #sink_writer.borrow_mut().push(__sink);
                            __source
                        };
                    });

                if let Some(serialize_pipeline) = serialize {
                    self.get_dfir_mut(from).add_dfir(
                        parse_quote! {
                            #input_ident -> map(#serialize_pipeline) -> for_each(|(target_member_id, v)| (#sink.borrow())[#root::__staged::location::TaglessMemberId::get_raw_id(&target_member_id) as usize].try_send(v).unwrap());
                        },
                        None,
                        Some(&format!("send{}", tag_id)),
                    );
                } else {
                    self.get_dfir_mut(from).add_dfir(
                        parse_quote! {
                            #input_ident -> for_each(|(target_member_id, v)| (#sink.borrow())[#root::__staged::location::TaglessMemberId::get_raw_id(&target_member_id) as usize].try_send(v).unwrap());
                        },
                        None,
                        Some(&format!("send{}", tag_id)),
                    );
                }

                if let Some(deserialize_pipeline) = deserialize {
                    self.get_dfir_mut(to).add_dfir(
                        parse_quote! {
                            #out_ident = source_stream(#source) #ok_wrap -> map(#deserialize_pipeline);
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
                    let #sink: ::std::rc::Rc<::std::cell::RefCell<Vec<__root_dfir_rs::util::unsync::mpsc::Sender<(#root::__staged::location::TaglessMemberId, #payload)>>>> = ::std::rc::Rc::new(::std::cell::RefCell::new(Vec::new()));
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
                            let (__sink, __source) = __root_dfir_rs::util::unsync::mpsc::unbounded::<(#root::__staged::location::TaglessMemberId, #payload)>();
                            #sink_writer.borrow_mut().push(__sink);
                            __source
                        };
                    });

                if let Some(serialize_pipeline) = serialize {
                    self.get_dfir_mut(from).add_dfir(
                        parse_quote! {
                            #input_ident -> map(#serialize_pipeline) -> for_each(|(target_member_id, v)| (#sink.borrow())[#root::__staged::location::TaglessMemberId::get_raw_id(&target_member_id) as usize].try_send((#root::__staged::location::TaglessMemberId::from_raw_id(__current_cluster_id), v)).unwrap());
                        },
                        None,
                        Some(&format!("send{}", tag_id)),
                    );
                } else {
                    self.get_dfir_mut(from).add_dfir(
                        parse_quote! {
                            #input_ident -> for_each(|(target_member_id, v)| (#sink.borrow())[#root::__staged::location::TaglessMemberId::get_raw_id(&target_member_id) as usize].try_send((#root::__staged::location::TaglessMemberId::from_raw_id(__current_cluster_id), v)).unwrap());
                        },
                        None,
                        Some(&format!("send{}", tag_id)),
                    );
                }

                if let Some(deserialize_pipeline) = deserialize {
                    self.get_dfir_mut(to).add_dfir(
                        parse_quote! {
                            #out_ident = source_stream(#source) #ok_wrap -> map(#deserialize_pipeline);
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
                    #input_ident -> map(#serialize_pipeline) -> for_each(|v| #grabbed_ident.try_send(v).unwrap());
                },
                None,
                Some(&format!("send{}", tag_id)),
            );
        } else {
            self.get_dfir_mut(on).add_dfir(
                parse_quote! {
                    #input_ident -> for_each(|v| #grabbed_ident.try_send(v).unwrap());
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
                let (#hoff_send_ident, #hoff_recv_ident) = __root_dfir_rs::util::unsync::mpsc::unbounded();
            });

            self.add_extra_stmt_internal(tick_location.root(), syn::parse_quote! {
                let #hoff_recv_ident = ::std::rc::Rc::new(::std::cell::RefCell::new(#hoff_recv_ident));
            });

            self.add_extra_stmt_internal(
                tick_location.root(),
                syn::parse_quote! {
                    let #buffered_ident = ::std::rc::Rc::new(::std::cell::RefCell::new(None));
                },
            );

            let inner_hook: syn::Expr = syn::parse_quote!(
                #root::sim::runtime::StreamOrderHook::<_>::new(
                    #buffered_ident.clone(),
                    #hoff_send_ident,
                    #root::sim::runtime::HookLocationMeta { location: #assume_location, line: #line, caret_indent: #caret },
                    #root::__maybe_debug__!(#element_type),
                )
            );
            if let Some(hook_id) = op_meta.sim_hook_id {
                let hook_rc_ident = syn::Ident::new(
                    &format!("__scripted_inline_fold_hook_{hoff_id}"),
                    Span::call_site(),
                );
                self.add_scripted_inline_hook(
                    hook_id,
                    tick_location,
                    &hook_rc_ident,
                    &assume_location,
                    inner_hook,
                );
            } else {
                self.add_inline_hook(tick_location, syn::parse_quote!(Box::new(#inner_hook)));
            }

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
            let (#hoff_send_ident, #hoff_recv_ident) = __root_dfir_rs::util::unsync::mpsc::unbounded();
        });
        self.add_extra_stmt_internal(location, syn::parse_quote! {
            let #buffered_ident = ::std::rc::Rc::new(::std::cell::RefCell::new(::std::collections::VecDeque::new()));
        });
        let inner_hook: syn::Expr = syn::parse_quote!(
            #root::sim::runtime::TopLevelFoldHook::<_> {
                input: #buffered_ident.clone(),
                to_release: None,
                output: #hoff_send_ident,
                location: #root::sim::runtime::HookLocationMeta { location: #assume_location, line: #line, caret_indent: #caret },
                format_item_debug: #root::__maybe_debug__!(#debug_type),
            }
        );
        if let Some(hook_id) = op_meta.sim_hook_id {
            let hook_rc_ident = syn::Ident::new(
                &format!("__scripted_fold_hook_{hoff_id}"),
                Span::call_site(),
            );
            self.add_scripted_hook(
                hook_id,
                location,
                location,
                &hook_rc_ident,
                &assume_location,
                inner_hook,
            );
        } else {
            self.add_hook(location, location, syn::parse_quote!(Box::new(#inner_hook)));
        }

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
        external_element_type: Option<&syn::Type>,
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
                external_element_type,
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
        external_element_type: Option<&syn::Type>,
        tag_id: StmtId,
    ) {
        let root = get_this_crate();
        let elem_ty = Self::channel_elem_ty(source, &root, external_element_type);
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
        .unwrap_or_else(|| ("unknown location".to_owned(), String::new(), String::new()))
}
