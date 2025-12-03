use quote::{ToTokens, quote_spanned};

use super::{
    DelayType, OpInstGenerics, OperatorCategory, OperatorConstraints, OperatorInstance,
    OperatorWriteOutput, Persistence, RANGE_1, WriteContextArgs,
};

/// > 1 input stream of type `(K, V1)`, 1 output stream of type `(K, V2)`.
/// > The output will have one tuple for each distinct `K`, with an accumulated value of type `V2`.
///
/// If the input and output value types are the same and do not require initialization then use
/// [`reduce_keyed`](#reduce_keyed).
///
/// > Arguments: two Rust closures. The first generates an initial value per group. The second
/// > itself takes two arguments: an 'accumulator', and an element. The second closure returns the
/// > value that the accumulator should have for the next iteration.
///
/// A special case of `fold`, in the spirit of SQL's GROUP BY and aggregation constructs. The input
/// is partitioned into groups by the first field ("keys"), and for each group the values in the second
/// field are accumulated via the closures in the arguments.
///
/// > Note: The closures have access to the [`context` object](surface_flows.mdx#the-context-object).
///
/// ```dfir
/// source_iter([("toy", 1), ("toy", 2), ("shoe", 11), ("shoe", 35), ("haberdashery", 7)])
///     -> fold_keyed(|| 0, |old: &mut u32, val: u32| *old += val)
///     -> assert_eq([("toy", 3), ("shoe", 46), ("haberdashery", 7)]);
/// ```
///
/// `fold_keyed` can be provided with one generic lifetime persistence argument, either
/// `'tick` or `'static`, to specify how data persists. With `'tick`, values will only be collected
/// within the same tick. With `'static`, values will be remembered across ticks and will be
/// aggregated with pairs arriving in later ticks. When not explicitly specified persistence
/// defaults to `'tick`.
///
/// `fold_keyed` can also be provided with two type arguments, the key type `K` and aggregated
/// output value type `V2`. This is required when using `'static` persistence if the compiler
/// cannot infer the types.
///
/// ```dfir
/// source_iter([("toy", 1), ("toy", 2), ("shoe", 11), ("shoe", 35), ("haberdashery", 7)])
///     -> fold_keyed(|| 0, |old: &mut u32, val: u32| *old += val)
///     -> assert_eq([("toy", 3), ("shoe", 46), ("haberdashery", 7)]);
/// ```
///
/// Example using `'tick` persistence:
/// ```rustbook
/// let (input_send, input_recv) = dfir_rs::util::unbounded_channel::<(&str, &str)>();
/// let mut flow = dfir_rs::dfir_syntax! {
///     source_stream(input_recv)
///         -> fold_keyed::<'tick, &str, String>(String::new, |old: &mut _, val| {
///             *old += val;
///             *old += ", ";
///         })
///         -> for_each(|(k, v)| println!("({:?}, {:?})", k, v));
/// };
///
/// input_send.send(("hello", "oakland")).unwrap();
/// input_send.send(("hello", "berkeley")).unwrap();
/// input_send.send(("hello", "san francisco")).unwrap();
/// flow.run_available();
/// // ("hello", "oakland, berkeley, san francisco, ")
///
/// input_send.send(("hello", "palo alto")).unwrap();
/// flow.run_available();
/// // ("hello", "palo alto, ")
/// ```
pub const FOLD_KEYED: OperatorConstraints = OperatorConstraints {
    name: "fold_keyed",
    categories: &[OperatorCategory::KeyedFold],
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 2,
    persistence_args: &(0..=1),
    type_args: &(0..=2),
    is_external_input: false,
    has_singleton_output: true,
    flo_type: None,
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| Some(DelayType::Stratum),
    write_fn: |wc @ &WriteContextArgs {
                   df_ident,
                   context,
                   op_span,
                   work_fn_async,
                   ident,
                   inputs,
                   singleton_output_ident,
                   is_pull,
                   root,
                   op_name,
                   op_inst:
                       OperatorInstance {
                           generics:
                               OpInstGenerics {
                                   persistence_args,
                                   type_args,
                                   ..
                               },
                           ..
                       },
                   arguments,
                   ..
               },
               _| {
        assert!(is_pull, "TODO(mingwei): `{}` only supports pull.", op_name);

        let persistence = match persistence_args[..] {
            [] => Persistence::Tick,
            [a] => a,
            _ => unreachable!(),
        };

        let generic_type_args = [
            type_args
                .first()
                .map(ToTokens::to_token_stream)
                .unwrap_or(quote_spanned!(op_span=> _)),
            type_args
                .get(1)
                .map(ToTokens::to_token_stream)
                .unwrap_or(quote_spanned!(op_span=> _)),
        ];

        let input = &inputs[0];
        let initfn = &arguments[0];
        let aggfn = &arguments[1];

        let hashtable_ident = wc.make_ident("hashtable");

        let write_prologue = quote_spanned! {op_span=>
            let #singleton_output_ident = #df_ident.add_state(::std::cell::RefCell::new(#root::rustc_hash::FxHashMap::<#( #generic_type_args ),*>::default()));
        };
        let write_prologue_after =wc
            .persistence_as_state_lifespan(persistence)
            .map(|lifespan| quote_spanned! {op_span=>
                #[allow(clippy::redundant_closure_call)]
                #df_ident.set_state_lifespan_hook(#singleton_output_ident, #lifespan, move |rcell| { rcell.take(); });
            }).unwrap_or_default();

        let assign_hashtable_ident = quote_spanned! {op_span=>
            let mut #hashtable_ident = unsafe {
                // SAFETY: handle from `#df_ident.add_state(..)`.
                #context.state_ref_unchecked(#singleton_output_ident)
            }.borrow_mut();
        };

        let write_iterator = if Persistence::Mutable == persistence {
            quote_spanned! {op_span=>
                #assign_hashtable_ident

                {
                    #[inline(always)]
                    fn check_input<St, K, V>(st: St) -> impl #root::futures::stream::Stream<Item = #root::util::PersistenceKeyed::<K, V>>
                    where
                        St: #root::futures::stream::Stream<Item = #root::util::PersistenceKeyed::<K, V>>,
                        K: ::std::clone::Clone,
                        V: ::std::clone::Clone,
                    {
                        st
                    }

                    /// A: accumulator type
                    /// T: iterator item type
                    #[inline(always)]
                    fn call_comb_type<A, T>(a: &mut A, t: T, f: impl Fn(&mut A, T)) {
                        let () = (f)(a, t);
                    }

                    let fut = #root::compiled::pull::ForEach::new(check_input(#input), |item| {
                        match item {
                            Persist(k, v) => {
                                let entry = #hashtable_ident.entry(k).or_insert_with(#initfn);
                                call_comb_type(entry, v, #aggfn);
                            },
                            Delete(k) => {
                                #hashtable_ident.remove(&k);
                            },
                        }
                    });
                    let () = #work_fn_async(fut).await;
                }

                let #ident = #hashtable_ident
                    .iter()
                    .map(#[allow(suspicious_double_ref_op, clippy::clone_on_copy)] |(k, v)| (k.clone(), v.clone()));
                let #ident = #root::futures::stream::iter(#ident);
            }
        } else {
            let iter_expr = match persistence {
                Persistence::None | Persistence::Tick => quote_spanned! {op_span=>
                    #hashtable_ident.drain()
                },
                Persistence::Loop => quote_spanned! {op_span=>
                    #hashtable_ident.iter().map(
                        #[allow(suspicious_double_ref_op, clippy::clone_on_copy)]
                        |(k, v)| (
                            ::std::clone::Clone::clone(k),
                            ::std::clone::Clone::clone(v),
                        )
                    )
                },
                Persistence::Static => quote_spanned! {op_span=>
                    // Play everything but only on the first run of this tick/stratum.
                    // (We know we won't have any more inputs, so it is fine to only play once.
                    // Because of the `DelayType::Stratum` or `DelayType::MonotoneAccum`).
                    #context.is_first_run_this_tick()
                        .then_some(#hashtable_ident.iter())
                        .into_iter()
                        .flatten()
                        .map(
                            #[allow(suspicious_double_ref_op, clippy::clone_on_copy)]
                            |(k, v)| (
                                ::std::clone::Clone::clone(k),
                                ::std::clone::Clone::clone(v),
                            )
                        )
                },
                Persistence::Mutable => unreachable!(),
            };

            quote_spanned! {op_span=>
                #assign_hashtable_ident

                {
                    #[inline(always)]
                    fn check_input<St, K, V>(st: St) -> impl #root::futures::stream::Stream<Item = (K, V)>
                    where
                        St: #root::futures::stream::Stream<Item = (K, V)>,
                        K: ::std::clone::Clone,
                        V: ::std::clone::Clone
                    {
                        st
                    }

                    /// A: accumulator type
                    /// T: iterator item type
                    #[inline(always)]
                    fn call_comb_type<A, T>(a: &mut A, t: T, f: impl Fn(&mut A, T)) {
                        let () = (f)(a, t);
                    }


                    let fut = #root::compiled::pull::ForEach::new(check_input(#input), |kv| {
                        // TODO(mingwei): remove `unknown_lints` when `clippy::unwrap_or_default` is stabilized.
                        #[allow(unknown_lints, clippy::unwrap_or_default)]
                        let entry = #hashtable_ident.entry(kv.0).or_insert_with(#initfn);
                        call_comb_type(entry, kv.1, #aggfn);
                    });
                    let () = #work_fn_async(fut).await;
                }

                let #ident = #iter_expr;
                let #ident = #root::futures::stream::iter(#ident);
            }
        };

        let write_iterator_after = match persistence {
            Persistence::None | Persistence::Tick | Persistence::Loop => Default::default(),
            Persistence::Static | Persistence::Mutable => quote_spanned! {op_span=>
                // Reschedule the subgraph lazily to ensure replay on later ticks.
                #context.schedule_subgraph(#context.current_subgraph(), false);
            },
        };

        Ok(OperatorWriteOutput {
            write_prologue,
            write_prologue_after,
            write_iterator,
            write_iterator_after,
        })
    },
};
