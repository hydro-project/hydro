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
/// > value, and an `Item`, and returns an `Option<o>` that will be emitted to the output stream
/// > if it's `Some`, or terminate the stream if it's `None`.
///
/// Similar to Rust's standard library `scan` method. It applies a function to each element of the stream,
/// maintaining an internal state (accumulator) and emitting the values returned by the function.
/// The function can return `None` to terminate the stream early.
///
/// > Note: The closures have access to the [`context` object](surface_flows.mdx#the-context-object).
///
/// `scan` can also be provided with one generic lifetime persistence argument, either
/// `'tick` or `'static`, to specify how data persists. With `'tick`, the accumulator will only be maintained
/// within the same tick. With `'static`, the accumulated value will be remembered across ticks.
/// When not explicitly specified persistence defaults to `'tick`.
///
/// ```dfir
/// // Running sum example
/// source_iter([1, 2, 3, 4])
///     -> scan::<'tick>(|| 0, |acc: &mut i32, x: i32| {
///         *acc += x;
///         Some(*acc)
///     })
///     -> assert_eq([1, 3, 6, 10]);
///
/// // Early termination example
/// source_iter([1, 2, 3, 4])
///     -> scan::<'tick>(|| 1, |state: &mut i32, x: i32| {
///         *state = *state * x;
///         if *state > 6 {
///             None
///         } else {
///             Some(-*state)
///         }
///     })
///     -> assert_eq([-1, -2, -6]);
/// ```
pub const SCAN: OperatorConstraints = OperatorConstraints {
    name: "scan",
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
        let result_ident = wc.make_ident("result");
        let state_ident = wc.make_ident("scan_state");

        let write_prologue = quote_spanned! {op_span=>
            #[allow(unused_mut, reason = "for if `Fn` instead of `FnMut`.")]
            let mut #initializer_func_ident = #init_fn;

            #[allow(clippy::redundant_closure_call)]
            let #singleton_output_ident = ::std::cell::RefCell::new(
                Some(#init)
            );
        };

        let write_tick_end = match persistence {
            super::Persistence::Tick => quote_spanned! {op_span=>
                #[allow(clippy::redundant_closure_call)]
                #singleton_output_ident.replace(Some(#init));
            },
            _ => Default::default(),
        };

        let assign_accum_ident = quote_spanned! {op_span=>
            let mut #state_ident = #singleton_output_ident.borrow_mut();

            if #state_ident.is_none() {
                return None;
            }
        };

        let iterator_foreach = quote_spanned! {op_span=>
            #[inline(always)]
            fn call_scan_fn<Accum, Item, Output>(
                accum: &mut Option<Accum>,
                item: Item,
                func: impl Fn(&mut Accum, Item) -> Option<Output>,
            ) -> Option<Output> {
                let result = (func)(accum.as_mut().unwrap(), item);

                if result.is_none() {
                    *accum = None;
                }

                result
            }

            #[allow(clippy::redundant_closure_call)]
            let #result_ident = call_scan_fn(&mut *#state_ident, #iterator_item_ident, #func);
        };

        let filter_map_body = quote_spanned! {op_span=>
            #assign_accum_ident
            #iterator_foreach
            #result_ident
        };

        let write_iterator = if is_pull {
            quote_spanned! {op_span=>
                let #ident = #root::dfir_pipes::pull::Pull::filter_map(#input, |#iterator_item_ident| {
                    #filter_map_body
                });
            }
        } else {
            let output = &outputs[0];
            quote_spanned! {op_span=>
                let #ident = #root::dfir_pipes::push::filter_map(|#iterator_item_ident| {
                    #filter_map_body
                }, #output);
            }
        };

        Ok(OperatorWriteOutput {
            write_prologue,
            write_iterator,
            write_tick_end,
            ..Default::default()
        })
    },
};
