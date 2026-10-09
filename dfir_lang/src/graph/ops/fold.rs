use quote::quote_spanned;

use super::{
    OperatorCategory, OperatorConstraints, OperatorWriteOutput, Persistence, RANGE_0,
    RANGE_1, WriteContextArgs,
};

/// > 1 input stream, 1 output stream
///
/// > Arguments: two arguments, both closures. The first closure is used to create the initial
/// > value for the accumulator, and the second is used to combine new items with the existing
/// > accumulator value. The second closure takes two two arguments: an `&mut Accum` accumulated
/// > value, and an `Item`.
///
/// Akin to Rust's built-in [`fold`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.fold)
/// operator, except that it takes the accumulator by `&mut` instead of by value. Folds every item
/// into an accumulator by applying a closure, returning the final result.
///
/// > Note: The closures have access to the [`context` object](surface_flows.mdx#the-context-object).
///
/// `fold` can also be provided with one generic lifetime persistence argument, either
/// `'tick` or `'static`, to specify how data persists. With `'tick`, Items will only be collected
/// within the same tick. With `'static`, the accumulated value will be remembered across ticks and
/// will be aggregated with items arriving in later ticks. When not explicitly specified
/// persistence defaults to `'tick`.
///
/// ```dfir
/// // should print `Reassembled vector [1,2,3,4,5]`
/// source_iter([1,2,3,4,5])
///     -> fold::<'tick>(Vec::new, |accum: &mut Vec<_>, elem| {
///         accum.push(elem);
///     })
///     -> assert_eq([vec![1, 2, 3, 4, 5]]);
/// ```
pub const FOLD: OperatorConstraints = OperatorConstraints {
    name: "fold",
    categories: &[OperatorCategory::Fold],
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
    hard_range_out: &(0..=1),
    soft_range_out: &(0..=1),
    num_args: 2,
    persistence_args: &(0..=1),
    type_args: RANGE_0,
    is_external_input: false,
    flo_type: None,
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| None,
    write_fn: |wc @ &WriteContextArgs {
                   root,
                   op_span,
                   work_fn,
                   work_fn_async,
                   ident,
                   is_pull,
                   inputs,
                   outputs,
                   arguments,
                   ..
               },
               diagnostics| {
        let init_fn = &arguments[0];
        let func = &arguments[1];
        let singleton_output_ident = wc.make_ident("singleton_output");

        let initializer_func_ident = wc.make_ident("initializer_func");
        let init = quote_spanned! {op_span=>
            (#initializer_func_ident)()
        };

        let [persistence] = wc.persistence_args(diagnostics);

        let input = &inputs[0];
        let accumulator_ident = wc.make_ident("accumulator");
        let item_ident = wc.make_ident("item");

        // The initializer closure is evaluated once, in the prologue.
        let mut write_prologue = quote_spanned! {op_span=>
            #[allow(unused_mut, reason = "for if `Fn` instead of `FnMut`.")]
            let mut #initializer_func_ident = #init_fn;
        };
        // For `'static` the accumulator state lives in the prologue, persisting across ticks.
        // For `'tick` the accumulator is created fresh each tick (within `write_iterator`), so it
        // can be moved (not cloned) into the output.
        if Persistence::Static == persistence {
            write_prologue.extend(quote_spanned! {op_span=>
                #[allow(clippy::redundant_closure_call)]
                let mut #singleton_output_ident = #init;
            });
        }
        let make_tick_state = matches!(persistence, Persistence::Tick).then(|| {
            quote_spanned! {op_span=>
                #[allow(clippy::redundant_closure_call)]
                let mut #singleton_output_ident = #init;
            }
        });

        let assign_accum_ident = quote_spanned! {op_span=>
            #[allow(unused_mut)]
            let mut #accumulator_ident = &mut #singleton_output_ident;
        };
        let foreach_body = quote_spanned! {op_span=>
            #[inline(always)]
            fn call_comb_type<Accum, Item>(
                accum: &mut Accum,
                item: Item,
                mut func: impl FnMut(&mut Accum, Item),
            ) {
                (func)(accum, item);
            }
            #[allow(clippy::redundant_closure_call)]
            call_comb_type(&mut *#accumulator_ident, #item_ident, #func);
        };

        let write_iterator = if is_pull {
            // `'tick`: the accumulator is a local created fresh this tick, so move it into the
            // output without cloning. `'static`: the accumulator persists in the prologue state,
            // so emit a clone of its current value.
            let output_expr = match persistence {
                Persistence::Tick => quote_spanned! {op_span=>
                    #singleton_output_ident
                },
                Persistence::Static => quote_spanned! {op_span=>
                    ::std::clone::Clone::clone(&*#accumulator_ident)
                },
            };
            quote_spanned! {op_span=>
                #make_tick_state
                #assign_accum_ident

                // Eagerly consume input to ensure updated state.
                {
                    let __fut = #root::dfir_pipes::pull::Pull::for_each(#input, |#item_ident| {
                        #foreach_body
                    });
                    let () = #work_fn_async(__fut).await;
                }

                let #ident = #work_fn(
                    || #root::dfir_pipes::pull::once(#output_expr)
                );
            }
        } else if outputs.is_empty() {
            // Terminal push: fold is a singleton reference target with no downstream.
            quote_spanned! {op_span=>
                #make_tick_state
                let #ident = #root::dfir_pipes::push::for_each(|#item_ident| {
                    #assign_accum_ident

                    #foreach_body
                });
            }
        } else if Persistence::Tick == persistence {
            // `'tick`: owned-mode fold. The accumulator is created fresh each tick and moved
            // (not cloned) into the output on finalize.
            let output = &outputs[0];
            quote_spanned! {op_span=>
                let #ident = {
                    #[inline(always)]
                    fn __push_fold_owned<Acc, Item, CombFn, Next>(
                        acc: Acc,
                        comb_fn: CombFn,
                        next: Next,
                    ) -> #root::dfir_pipes::push::Accumulate<
                        #root::dfir_pipes::push::FoldState<Acc, CombFn, Acc, Item>,
                        Next,
                    >
                    where
                        CombFn: ::std::ops::FnMut(&mut Acc, Item),
                        Next: #root::dfir_pipes::push::Push<Acc, ()>,
                    {
                        #root::dfir_pipes::push::fold(acc, comb_fn, next)
                    }
                    #[allow(clippy::redundant_closure_call)]
                    let #singleton_output_ident = #init;
                    __push_fold_owned(
                        #singleton_output_ident,
                        |#accumulator_ident: &mut _, #item_ident| { #foreach_body },
                        #output,
                    )
                };
            }
        } else {
            // `'static`: borrowed-mode fold. The accumulator persists in the prologue state, so
            // emit a clone of its current value on finalize.
            let output = &outputs[0];
            quote_spanned! {op_span=>
                let #ident = {
                    #[inline(always)]
                    fn __push_fold<'a, Acc, Item, CombFn, Next>(
                        acc_ref: &'a mut Acc,
                        comb_fn: CombFn,
                        next: Next,
                    ) -> #root::dfir_pipes::push::Accumulate<
                        #root::dfir_pipes::push::FoldState<&'a mut Acc, CombFn, Acc, Item>,
                        Next,
                    >
                    where
                        CombFn: ::std::ops::FnMut(&mut Acc, Item),
                        Next: #root::dfir_pipes::push::Push<&'a mut Acc, ()>,
                    {
                        #root::dfir_pipes::push::fold(acc_ref, comb_fn, next)
                    }
                    __push_fold(
                        &mut #singleton_output_ident,
                        |#accumulator_ident: &mut _, #item_ident| { #foreach_body },
                        #root::dfir_pipes::push::map(
                            |__val: &mut _| ::std::clone::Clone::clone(&*__val),
                            #output,
                        ),
                    )
                };
            }
        };

        Ok(OperatorWriteOutput {
            write_prologue,
            write_iterator,
            ..Default::default()
        })
    },
};
