use quote::quote_spanned;

use super::{
    DelayType, OperatorCategory, OperatorConstraints, OperatorWriteOutput, Persistence, RANGE_0,
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
    input_delaytype_fn: |_| Some(DelayType::Stratum),
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

        // For static persistence, we need to track termination state across ticks
        let is_static = matches!(persistence, Persistence::Static);

        let input = &inputs[0];
        let accumulator_ident = wc.make_ident("accumulator");
        let iterator_item_ident = wc.make_ident("iterator_item");
        let result_ident = wc.make_ident("result");
        let state_ident = wc.make_ident("scan_state");

        // Define the state as a tuple that will hold both the accumulator and termination flag
        let write_prologue = if is_static {
            quote_spanned! {op_span=>
                #[allow(unused_mut, reason = "for if `Fn` instead of `FnMut`.")]
                let mut #initializer_func_ident = #init_fn;

                // Use a tuple to hold both the accumulator and termination state
                #[allow(clippy::redundant_closure_call)]
                let #singleton_output_ident = #df_ident.add_state(::std::cell::RefCell::new(
                    (#init, false) // (accumulator, terminated)
                ));
            }
        } else {
            quote_spanned! {op_span=>
                #[allow(unused_mut, reason = "for if `Fn` instead of `FnMut`.")]
                let mut #initializer_func_ident = #init_fn;

                #[allow(clippy::redundant_closure_call)]
                let #singleton_output_ident = #df_ident.add_state(::std::cell::RefCell::new(#init));
            }
        };

        let write_prologue_after = if is_static {
            wc.persistence_as_state_lifespan(persistence)
                .map(|lifespan| {
                    quote_spanned! {op_span=>
                        #[allow(clippy::redundant_closure_call)]
                        #df_ident.set_state_lifespan_hook(
                            #singleton_output_ident, #lifespan, move |rcell| {
                                rcell.replace((#init, false)); // Reset to (accumulator, terminated=false)
                            },
                        );
                    }
                })
                .unwrap_or_default()
        } else {
            wc.persistence_as_state_lifespan(persistence)
                .map(|lifespan| quote_spanned! {op_span=>
                    #[allow(clippy::redundant_closure_call)]
                    #df_ident.set_state_lifespan_hook(
                        #singleton_output_ident, #lifespan, move |rcell| { rcell.replace(#init); },
                    );
                }).unwrap_or_default()
        };

        // Access the state differently based on persistence type
        let assign_accum_ident = if is_static {
            quote_spanned! {op_span=>
                #[allow(unused_mut)]
                let mut #state_ident = unsafe {
                    // SAFETY: handle from `#df_ident.add_state(..)`.
                    #context.state_ref_unchecked(#singleton_output_ident)
                }.borrow_mut();

                // Check if the scan was previously terminated
                if #state_ident.1 { // Check terminated flag (second element of tuple)
                    return None;
                }
            }
        } else {
            quote_spanned! {op_span=>
                #[allow(unused_mut)]
                let mut #accumulator_ident = unsafe {
                    // SAFETY: handle from `#df_ident.add_state(..)`.
                    #context.state_ref_unchecked(#singleton_output_ident)
                }.borrow_mut();
            }
        };

        // Call the scan function differently based on persistence type
        let iterator_foreach = if is_static {
            quote_spanned! {op_span=>
                #[inline(always)]
                fn call_scan_fn<Accum, Item, Output>(
                    accum: &mut (Accum, bool),
                    item: Item,
                    func: impl Fn(&mut Accum, Item) -> Option<Output>,
                ) -> Option<Output> {
                    let result = (func)(&mut accum.0, item);
                    // Update termination state if None was returned
                    if result.is_none() {
                        accum.1 = true;
                    }
                    result
                }
                #[allow(clippy::redundant_closure_call)]
                let #result_ident = call_scan_fn(&mut #state_ident, #iterator_item_ident, #func);
            }
        } else {
            quote_spanned! {op_span=>
                #[inline(always)]
                fn call_scan_fn<Accum, Item, Output>(
                    accum: &mut Accum,
                    item: Item,
                    func: impl Fn(&mut Accum, Item) -> Option<Output>,
                ) -> Option<Output> {
                    (func)(accum, item)
                }
                #[allow(clippy::redundant_closure_call)]
                let #result_ident = call_scan_fn(&mut *#accumulator_ident, #iterator_item_ident, #func);
            }
        };

        // Generate the iterator code based on pull/push mode and persistence
        let filter_map_body = if is_static {
            quote_spanned! {op_span=>
                #assign_accum_ident
                #iterator_foreach
                #result_ident
            }
        } else {
            quote_spanned! {op_span=>
                if done {
                    return None;
                }

                #assign_accum_ident
                #iterator_foreach

                if #result_ident.is_none() {
                    done = true;
                }

                #result_ident
            }
        };

        let write_iterator = if is_pull {
            if is_static {
                quote_spanned! {op_span=>
                    let #ident = #input.filter_map(|#iterator_item_ident| {
                        #filter_map_body
                    });
                }
            } else {
                quote_spanned! {op_span=>
                    let #ident = {
                        let mut done = false;
                        let #context = &#context;
                        #input.filter_map(move |#iterator_item_ident| {
                            #filter_map_body
                        })
                    };
                }
            }
        } else if is_static {
            quote_spanned! {op_span=>
                let #ident = #root::pusherator::filter_map::FilterMap::new(|#iterator_item_ident| {
                    #filter_map_body
                });
            }
        } else {
            quote_spanned! {op_span=>
                let #ident = {
                    let mut done = false;
                    let #context = &#context;
                    #root::pusherator::filter_map::FilterMap::new(move |#iterator_item_ident| {
                        #filter_map_body
                    })
                };
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
