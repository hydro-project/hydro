use quote::quote_spanned;

use super::{
    OperatorCategory, OperatorConstraints, OperatorWriteOutput, RANGE_0,
    RANGE_1, WriteContextArgs,
};

/// > 1 input stream, 1 output stream
///
/// > Arguments: two arguments, both closures. The first closure is used to create the initial
/// > value for the accumulator, and the second is used to transform new items with the existing
/// > accumulator value. The second closure takes two arguments: an `&mut Accum` accumulated
/// > value, and an `Item`, and returns a `Future<Output = Option<O>>`. The closure runs
/// > synchronously (so it can mutate the accumulator), then returns a future that is polled
/// > to completion. If the future resolves to `Some`, the value is emitted. If it resolves to
/// > `None`, the item is filtered out.
///
/// Async version of [`scan`]. It applies an async function to each element of the stream,
/// maintaining an internal state (accumulator) and emitting the values returned by the function.
///
/// > Note: The closures have access to the [`context` object](surface_flows.mdx#the-context-object).
///
/// `scan_async_blocking` can also be provided with one generic lifetime persistence argument, either
/// `'tick` or `'static`, to specify how data persists. With `'tick`, the accumulator will only be maintained
/// within the same tick. With `'static`, the accumulated value will be remembered across ticks.
/// When not explicitly specified persistence defaults to `'tick`.
///
/// ```dfir
/// source_iter([1, 2, 3, 4])
///     -> scan_async_blocking::<'tick>(|| 0, |acc: &mut i32, x: i32| {
///         *acc += x;
///         let val = *acc;
///         async move { Some(val) }
///     })
///     -> assert_eq([1, 3, 6, 10]);
/// ```
pub const SCAN_ASYNC_BLOCKING: OperatorConstraints = OperatorConstraints {
    name: "scan_async_blocking",
    categories: &[OperatorCategory::Fold],
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 2,
    persistence_args: &(0..=1),
    type_args: RANGE_0,
    is_external_input: false,
    has_singleton_output: true,
    flo_type: None,
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| None,
    write_fn: |wc @ &WriteContextArgs {
                   root,
                   context,
                   df_ident,
                   op_span,
                   ident,
                   is_pull,
                   inputs,
                   outputs,
                   arguments,
                   singleton_output_ident,
                   ..
               },
               diagnostics| {
        let init_fn = &arguments[0];
        let func = &arguments[1];

        let initializer_func_ident = wc.make_ident("initializer_func");
        let init = quote_spanned! {op_span=>
            (#initializer_func_ident)()
        };

        let [persistence] = wc.persistence_args_disallow_mutable(diagnostics);

        let input = &inputs[0];
        let iterator_item_ident = wc.make_ident("iterator_item");
        let state_ident = wc.make_ident("scan_state");

        let write_prologue = quote_spanned! {op_span=>
            #[allow(unused_mut, reason = "for if `Fn` instead of `FnMut`.")]
            let mut #initializer_func_ident = #init_fn;

            #[allow(clippy::redundant_closure_call)]
            let #singleton_output_ident = #df_ident.add_state(::std::cell::RefCell::new(
                Some(#init)
            ));
        };

        let write_prologue_after = wc.persistence_as_state_lifespan(persistence)
            .map(|lifespan| {
                quote_spanned! {op_span=>
                    #[allow(clippy::redundant_closure_call)]
                    #df_ident.set_state_lifespan_hook(
                        #singleton_output_ident, #lifespan, move |rcell| {
                            rcell.replace(Some(#init));
                        },
                    );
                }
            })
            .unwrap_or_default();

        // The closure for filter_map_async returns a future.
        // We create the inner future synchronously (borrowing state), then return it.
        // Note: unlike sync scan, returning None from the future only filters the
        // current item; it does not terminate the stream.
        let filter_map_async_body = quote_spanned! {op_span=>
            #[inline(always)]
            fn call_scan_async_blocking_fn<Accum, Item, Output, Fut>(
                accum: &mut Accum,
                item: Item,
                func: impl Fn(&mut Accum, Item) -> Fut,
            ) -> Fut
            where
                Fut: ::std::future::Future<Output = Option<Output>>,
            {
                (func)(accum, item)
            }

            let mut #state_ident = unsafe {
                // SAFETY: handle from `#df_ident.add_state(..)`.
                #context.state_ref_unchecked(#singleton_output_ident)
            }.borrow_mut();

            #[allow(clippy::redundant_closure_call)]
            call_scan_async_blocking_fn(#state_ident.as_mut().unwrap(), #iterator_item_ident, #func)
            // borrow dropped here when the future is returned
        };

        let write_iterator = if is_pull {
            quote_spanned! {op_span=>
                let #ident = #root::dfir_pipes::pull::Pull::filter_map_async(#input, |#iterator_item_ident| {
                    #filter_map_async_body
                });
            }
        } else {
            let output = &outputs[0];
            quote_spanned! {op_span=>
                let #ident = #root::dfir_pipes::push::filter_map_async(|#iterator_item_ident| {
                    #filter_map_async_body
                }, #output);
            }
        };

        Ok(OperatorWriteOutput {
            write_prologue,
            write_prologue_after,
            write_iterator,
            write_iterator_after: Default::default(),
        })
    },
};
