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
                   context,
                   df_ident,
                   op_span,
                   ident,
                   is_pull,
                   inputs,
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

        // Use Option<State> to represent both the accumulator and termination state
        // None means terminated, Some(state) means active with accumulator value
        let write_prologue = quote_spanned! {op_span=>
            #[allow(unused_mut, reason = "for if `Fn` instead of `FnMut`.")]
            let mut #initializer_func_ident = #init_fn;

            #[allow(clippy::redundant_closure_call)]
            let #singleton_output_ident = #df_ident.add_state(::std::cell::RefCell::new(
                Some(#init) // Some(accumulator) means active state
            ));
        };

        let write_prologue_after = wc.persistence_as_state_lifespan(persistence)
            .map(|lifespan| {
                quote_spanned! {op_span=>
                    #[allow(clippy::redundant_closure_call)]
                    #df_ident.set_state_lifespan_hook(
                        #singleton_output_ident, #lifespan, move |rcell| {
                            rcell.replace(Some(#init)); // Reset to Some(accumulator) for active state
                        },
                    );
                }
            })
            .unwrap_or_default();

        // Access the state using Option<State> pattern
        let assign_accum_ident = quote_spanned! {op_span=>
            let mut #state_ident = unsafe {
                // SAFETY: handle from `#df_ident.add_state(..)`.
                #context.state_ref_unchecked(#singleton_output_ident)
            }.borrow_mut();

            // Check if the scan was previously terminated
            if #state_ident.is_none() {
                return None;
            }
        };

        // Call the scan function using Option<State> pattern
        let iterator_foreach = quote_spanned! {op_span=>
            #[inline(always)]
            fn call_scan_fn<Accum, Item, Output>(
                accum: &mut Option<Accum>,
                item: Item,
                func: impl Fn(&mut Accum, Item) -> Option<Output>,
            ) -> Option<Output> {
                // We already checked that accum is Some in assign_accum_ident,
                // if an earlier element terminated then filter_map will not reach this
                let result = (func)(accum.as_mut().unwrap(), item);

                // Update termination state if None was returned
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
                let #ident = #input.filter_map(|#iterator_item_ident| {
                    #filter_map_body
                });
            }
        } else {
            quote_spanned! {op_span=>
                let #ident = #root::pusherator::filter_map::FilterMap::new(|#iterator_item_ident| {
                    #filter_map_body
                });
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
