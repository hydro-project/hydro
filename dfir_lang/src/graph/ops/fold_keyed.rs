use proc_macro2::TokenStream;
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
    write_fn: |wc @ &WriteContextArgs { root, op_span, .. }, _| {
        accum_keyed_codegen(
            wc,
            quote_spanned! {op_span=>
                #root::util::accumulator::Fold
            },
        )
    },
};

pub fn accum_keyed_codegen(
    wc: &WriteContextArgs,
    accum_type: TokenStream,
) -> Result<OperatorWriteOutput, ()> {
    let &WriteContextArgs {
        df_ident,
        context,
        op_span,
        ident,
        inputs,
        singleton_output_ident,
        is_pull,
        root,
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
    } = wc;

    assert!(is_pull);

    let persistence = match persistence_args[..] {
        [] => Persistence::Tick,
        [a] => a,
        _ => unreachable!(),
    };

    let key_type_arg = type_args
        .first()
        .map(ToTokens::to_token_stream)
        .unwrap_or(quote_spanned!(op_span=> _));
    let val_type_arg = type_args
        .get(1)
        .map(ToTokens::to_token_stream)
        .unwrap_or(quote_spanned!(op_span=> _));

    let input = &inputs[0];

    let hashtable_ident = wc.make_ident("hashtable");

    let write_prologue = quote_spanned! {op_span=>
        let #singleton_output_ident = #df_ident.add_state(::std::cell::RefCell::new(#root::rustc_hash::FxHashMap::<#key_type_arg, #val_type_arg>::default()));
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
        todo!()
        // quote_spanned! {op_span=>
        //     #assign_hashtable_ident

        //     #work_fn(|| {
        //         #[inline(always)]
        //         fn check_input<Iter, K, V>(iter: Iter) -> impl ::std::iter::Iterator<Item = #root::util::PersistenceKeyed::<K, V>>
        //         where
        //             Iter: ::std::iter::Iterator<Item = #root::util::PersistenceKeyed::<K, V>>,
        //             K: ::std::clone::Clone,
        //             V: ::std::clone::Clone,
        //         {
        //             iter
        //         }

        //         /// A: accumulator type
        //         /// T: iterator item type
        //         /// O: output type
        //         #[inline(always)]
        //         fn call_comb_type<A, T, O>(a: &mut A, t: T, f: impl Fn(&mut A, T) -> O) -> O {
        //             (f)(a, t)
        //         }

        //         for item in check_input(#input) {
        //             match item {
        //                 Persist(k, v) => {
        //                     let entry = #hashtable_ident.entry(k).or_insert_with(#initfn);
        //                     call_comb_type(entry, v, #aggfn);
        //                 },
        //                 Delete(k) => {
        //                     #hashtable_ident.remove(&k);
        //                 },
        //             }
        //         }
        //     });

        //     let #ident = #hashtable_ident
        //         .iter()
        //         .map(#[allow(suspicious_double_ref_op, clippy::clone_on_copy)] |(k, v)| (k.clone(), v.clone()));
        // }
    } else {
        let then_fn = match persistence {
            Persistence::None | Persistence::Tick => {
                quote_spanned! {op_span=> |ht| ht.drain() }
            }
            Persistence::Loop => quote_spanned! {op_span=>
                |ht| ht.iter().map(
                    #[allow(suspicious_double_ref_op, clippy::clone_on_copy)]
                    |(k, v)| (
                        ::std::clone::Clone::clone(k),
                        ::std::clone::Clone::clone(v),
                    )
                )
            },
            Persistence::Static => quote_spanned! {op_span=>
                |ht| {
                    // Play everything but only on the first run of this tick/stratum.
                    // (We know we won't have any more inputs, so it is fine to only play once.
                    // Because of the `DelayType::Stratum` or `DelayType::MonotoneAccum`).
                    #context.is_first_run_this_tick()
                        .then_some(ht.iter())
                        .into_iter()
                        .flatten()
                        .map(
                            #[allow(suspicious_double_ref_op, clippy::clone_on_copy)]
                            |(k, v)| (
                                ::std::clone::Clone::clone(k),
                                ::std::clone::Clone::clone(v),
                            )
                        )
                }
            },
            Persistence::Mutable => unreachable!(),
        };

        quote_spanned! {op_span=>
            #assign_hashtable_ident

            let #ident = {
                fn __check_inputs<'a, St, Key, ValAccum, Hasher, Accum, ThenFn, ThenIter, ValIn>(
                    stream: St,
                    state: &'a mut ::std::collections::HashMap<Key, ValAccum, Hasher>,
                    accum: Accum,
                    then_fn: ThenFn,
                ) -> impl #root::futures::stream::Stream<Item = ThenIter::Item>
                where
                    St: #root::futures::stream::Stream<Item = (Key, ValIn)>,
                    Key: ::std::cmp::Eq + ::std::hash::Hash,
                    Hasher: ::std::hash::BuildHasher,
                    Accum: #root::util::accumulator::Accumulator<ValAccum, ValIn>,
                    ThenFn: ::std::ops::FnOnce(&'a mut ::std::collections::HashMap<Key, ValAccum, Hasher>) -> ThenIter,
                    ThenIter: ::std::iter::Iterator,
                {
                    #root::compiled::pull::AccumKeyedThen::new(stream, state, accum, then_fn)
                }

                __check_inputs::<'_, _, #key_type_arg, #val_type_arg, _, _, _, _, _>(#input, &mut *#hashtable_ident, #accum_type::new(#arguments), #then_fn)
            };
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
}
