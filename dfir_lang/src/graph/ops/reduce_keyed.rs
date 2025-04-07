use quote::{ToTokens, quote_spanned};

use super::{
    DelayType, OpInstGenerics, OperatorCategory, OperatorConstraints, OperatorInstance,
    OperatorWriteOutput, Persistence, RANGE_1, WriteContextArgs,
};
use crate::diagnostic::{Diagnostic, Level};

/// > 1 input stream of type `(K, V)`, 1 output stream of type `(K, V)`.
/// > The output will have one tuple for each distinct `K`, with an accumulated (reduced) value of
/// > type `V`.
///
/// If you need the accumulated value to have a different type than the input, use [`fold_keyed`](#fold_keyed).
///
/// > Arguments: one Rust closures. The closure takes two arguments: an `&mut` 'accumulator', and
/// > an element. Accumulator should be updated based on the element.
///
/// A special case of `reduce`, in the spirit of SQL's GROUP BY and aggregation constructs. The input
/// is partitioned into groups by the first field, and for each group the values in the second
/// field are accumulated via the closures in the arguments.
///
/// > Note: The closures have access to the [`context` object](surface_flows.mdx#the-context-object).
///
/// `reduce_keyed` can also be provided with one generic lifetime persistence argument, either
/// `'tick` or `'static`, to specify how data persists. With `'tick`, values will only be collected
/// within the same tick. With `'static`, values will be remembered across ticks and will be
/// aggregated with pairs arriving in later ticks. When not explicitly specified persistence
/// defaults to `'tick`.
///
/// `reduce_keyed` can also be provided with two type arguments, the key and value type. This is
/// required when using `'static` persistence if the compiler cannot infer the types.
///
/// ```dfir
/// source_iter([("toy", 1), ("toy", 2), ("shoe", 11), ("shoe", 35), ("haberdashery", 7)])
///     -> reduce_keyed(|old: &mut u32, val: u32| *old += val)
///     -> assert_eq([("toy", 3), ("shoe", 46), ("haberdashery", 7)]);
/// ```
///
/// Example using `'tick` persistence and type arguments:
/// ```rustbook
/// let (input_send, input_recv) = dfir_rs::util::unbounded_channel::<(&str, &str)>();
/// let mut flow = dfir_rs::dfir_syntax! {
///     source_stream(input_recv)
///         -> reduce_keyed::<'tick, &str>(|old: &mut _, val| *old = std::cmp::max(*old, val))
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
pub const REDUCE_KEYED: OperatorConstraints = OperatorConstraints {
    name: "reduce_keyed",
    categories: &[OperatorCategory::KeyedFold],
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 1,
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
                   ident,
                   inputs,
                   singleton_output_ident,
                   is_pull,
                   work_fn,
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
               diagnostics| {
        assert!(is_pull, "TODO(mingwei): `{}` only supports pull.", op_name);

        let persistence = match persistence_args[..] {
            [] => Persistence::Tick,
            [Persistence::Mutable] => {
                diagnostics.push(Diagnostic::spanned(
                    op_span,
                    Level::Error,
                    "An implementation of `'mutable` does not exist",
                ));
                Persistence::Tick
            }
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
        let aggfn = &arguments[0];

        let hashtable_ident = wc.make_ident("hashtable");

        let write_prologue = quote_spanned! {op_span=>
            let #singleton_output_ident = #df_ident.add_state(::std::cell::RefCell::new(#root::rustc_hash::FxHashMap::<#( #generic_type_args ),*>::default()));
        };
        let write_prologue_after = wc
            .persistence_as_state_lifespan(persistence)
            .map(|lifespan| quote_spanned! {op_span=>
                #df_ident.set_state_lifespan_hook(#singleton_output_ident, #lifespan, |rcell| { rcell.take(); });
            }).unwrap_or_default();

        let write_iterator = {
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
                let mut #hashtable_ident = unsafe {
                    // SAFETY: handle from `#df_ident.add_state(..)`.
                    #context.state_ref_unchecked(#singleton_output_ident)
                }.borrow_mut();

                #work_fn(|| {
                    #[inline(always)]
                    fn check_input<Iter, K, V>(iter: Iter) -> impl ::std::iter::Iterator<Item = (K, V)>
                    where
                        Iter: std::iter::Iterator<Item = (K, V)>,
                        K: ::std::clone::Clone,
                        V: ::std::clone::Clone
                    {
                        iter
                    }

                    /// A: accumulator/item type
                    /// O: output type
                    #[inline(always)]
                    fn call_comb_type<A, O>(acc: &mut A, item: A, f: impl Fn(&mut A, A) -> O) -> O {
                        (f)(acc, item)
                    }

                    for kv in check_input(#input) {
                        match #hashtable_ident.entry(kv.0) {
                            ::std::collections::hash_map::Entry::Vacant(vacant) => {
                                vacant.insert(kv.1);
                            }
                            ::std::collections::hash_map::Entry::Occupied(mut occupied) => {
                                call_comb_type(occupied.get_mut(), kv.1, #aggfn);
                            }
                        }
                    }
                });

                let #ident = #iter_expr;
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
