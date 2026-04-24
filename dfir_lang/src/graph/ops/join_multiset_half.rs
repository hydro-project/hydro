use quote::{ToTokens, quote_spanned};
use syn::parse_quote;

use super::{
    DelayType, OperatorCategory, OperatorConstraints, OperatorWriteOutput, PortIndexValue, RANGE_0,
    RANGE_1, WriteContextArgs,
};
use crate::graph::ops::Persistence;

/// > 2 input streams of type `<(K, V1)>` (build) and `<(K, V2)>` (probe),
/// > with output type `<(K, (V2, V1))>`
///
/// An asymmetric hash join where the `build` side is accumulated first
/// (stratum-delayed) and then the `probe` side streams through, emitting
/// matches. This preserves the probe side's arrival order.
///
/// ```dfir
/// source_iter(vec![("cat", 'x'), ("dog", 'y')]) -> [build]my_join;
/// source_iter(vec![("cat", 1), ("dog", 2), ("cat", 3)]) -> [probe]my_join;
/// my_join = join_multiset_half()
///     -> assert_eq([("cat", (1, 'x')), ("dog", (2, 'y')), ("cat", (3, 'x'))]);
/// ```
pub const JOIN_MULTISET_HALF: OperatorConstraints = OperatorConstraints {
    name: "join_multiset_half",
    categories: &[OperatorCategory::MultiIn],
    hard_range_inn: &(2..=2),
    soft_range_inn: &(2..=2),
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 0,
    persistence_args: &(0..=2),
    type_args: RANGE_0,
    is_external_input: false,
    has_singleton_output: false,
    flo_type: None,
    ports_inn: Some(|| super::PortListSpec::Fixed(parse_quote! { build, probe })),
    ports_out: None,
    input_delaytype_fn: |idx| match idx {
        PortIndexValue::Path(path) if "build" == path.to_token_stream().to_string() => {
            Some(DelayType::Stratum)
        }
        _else => None,
    },
    write_fn: |wc @ &WriteContextArgs {
                   root,
                   context,
                   df_ident,
                   op_span,
                   work_fn_async,
                   ident,
                   is_pull,
                   inputs,
                   ..
               },
               diagnostics| {
        assert!(is_pull);

        let persistences: [_; 2] = wc.persistence_args_disallow_mutable(diagnostics);

        let probe_ident = wc.make_ident("probe");
        let build_ident = wc.make_ident("build");
        let probe_borrow = wc.make_ident("probe_borrow");
        let build_borrow = wc.make_ident("build_borrow");

        // persistences[0] = build (first port), persistences[1] = probe (second port)
        let probe_persist = match persistences[1] {
            Persistence::None | Persistence::Tick => false,
            Persistence::Loop | Persistence::Static => true,
            Persistence::Mutable => unreachable!(),
        };

        let write_prologue_probe = probe_persist.then(|| {
            quote_spanned! {op_span=>
                let #probe_ident = #df_ident.add_state(std::cell::RefCell::new(
                    ::std::vec::Vec::new()
                ));
            }
        });
        let write_prologue_after_probe = probe_persist.then(|| wc
            .persistence_as_state_lifespan(persistences[1])
            .map(|lifespan| quote_spanned! {op_span=>
                #[allow(clippy::redundant_closure_call)]
                #df_ident.set_state_lifespan_hook(
                    #probe_ident, #lifespan, move |rcell| { rcell.borrow_mut().clear(); },
                );
            })).flatten();

        let write_prologue_build = quote_spanned! {op_span=>
            let #build_ident = #df_ident.add_state(std::cell::RefCell::new(
                #root::rustc_hash::FxHashMap::default()
            ));
        };
        let write_prologue_after_build = wc
            .persistence_as_state_lifespan(persistences[0])
            .map(|lifespan| quote_spanned! {op_span=>
                #[allow(clippy::redundant_closure_call)]
                #df_ident.set_state_lifespan_hook(
                    #build_ident, #lifespan, move |rcell| { rcell.borrow_mut().clear(); },
                );
            }).unwrap_or_default();

        let input_build = &inputs[0]; // build before probe (stratum-delayed comes first)
        let input_probe = &inputs[1];

        let accum_build = quote_spanned! {op_span=>
            let fut = #root::dfir_pipes::pull::Pull::for_each(#input_build, |(k, v)| {
                #build_borrow.entry(k).or_insert_with(::std::vec::Vec::new).push(v);
            });
            let () = #work_fn_async(fut).await;
        };

        let write_iterator = if !probe_persist {
            quote_spanned! {op_span=>
                let mut #build_borrow = unsafe {
                    // SAFETY: handle from `#df_ident`.
                    #context.state_ref_unchecked(#build_ident)
                }.borrow_mut();

                let #ident = {
                    #accum_build

                    // Bound K/V types explicitly to prevent inference failures across subgraph handoffs.
                    #[allow(clippy::clone_on_copy, noop_method_call)]
                    #[inline(always)]
                    fn probe_join<'a, K, V1, V2, I>(
                        probe: I,
                        build_state: &'a #root::rustc_hash::FxHashMap<K, ::std::vec::Vec<V1>>,
                    ) -> impl 'a + #root::dfir_pipes::pull::Pull<Item = (K, (V2, V1)), Meta = ()>
                    where
                        K: ::std::cmp::Eq + ::std::hash::Hash + ::std::clone::Clone + 'a,
                        V1: ::std::clone::Clone + 'a,
                        V2: ::std::clone::Clone + 'a,
                        I: 'a + #root::dfir_pipes::pull::Pull<Item = (K, V2), Meta = ()>,
                    {
                        #root::dfir_pipes::pull::Pull::flat_map(probe, move |(k, v_probe)| {
                            build_state
                                .get(&k)
                                .map(|vals| vals.iter().map(|v_build| (k.clone(), (v_probe.clone(), v_build.clone()))).collect::<::std::vec::Vec<_>>())
                                .unwrap_or_default()
                                .into_iter()
                        })
                    }
                    probe_join(#input_probe, &*#build_borrow)
                };
            }
        } else {
            quote_spanned! {op_span =>
                let (mut #build_borrow, mut #probe_borrow) = unsafe {
                    // SAFETY: handles from `#df_ident`.
                    (
                        #context.state_ref_unchecked(#build_ident).borrow_mut(),
                        #context.state_ref_unchecked(#probe_ident).borrow_mut(),
                    )
                };

                let #ident = {
                    #accum_build

                    let replay_idx = if #context.is_first_run_this_tick() {
                        0
                    } else {
                        #probe_borrow.len()
                    };

                    // Accum into probe vec
                    let fut = #root::dfir_pipes::pull::Pull::for_each(#input_probe, |kv| {
                        #probe_borrow.push(kv);
                    });
                    let () = #work_fn_async(fut).await;

                    // Replay out of probe vec
                    #[allow(clippy::clone_on_copy, noop_method_call)]
                    let iter = #probe_borrow[replay_idx..].iter().flat_map(|(k, v_probe)| {
                        #build_borrow
                            .get(k)
                            .map(|vals: &::std::vec::Vec<_>| {
                                vals.iter().map(|v_build| (k.clone(), (v_probe.clone(), v_build.clone()))).collect::<::std::vec::Vec<_>>()
                            })
                            .unwrap_or_default()
                    });
                    #root::dfir_pipes::pull::iter(iter)
                };
            }
        };

        Ok(OperatorWriteOutput {
            write_prologue: quote_spanned! {op_span=>
                #write_prologue_probe
                #write_prologue_build
            },
            write_prologue_after: quote_spanned! {op_span=>
                #write_prologue_after_probe
                #write_prologue_after_build
            },
            write_iterator,
            ..Default::default()
        })
    },
};
