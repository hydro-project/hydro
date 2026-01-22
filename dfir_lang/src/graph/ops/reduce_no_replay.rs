use quote::quote_spanned;

use super::{
    DelayType, OperatorCategory, OperatorConstraints, OperatorWriteOutput, RANGE_0,
    RANGE_1, WriteContextArgs,
};

/// > 1 input stream, 1 output stream
///
/// > Arguments: a closure which itself takes two arguments:
/// > an `&mut Accum` accumulator mutable reference, and an `Item`. The closure should merge the item
/// > into the accumulator.
///
/// Like the `reduce` operator, but does not replay the accumulated value on ticks where there is
/// no new input.
pub const REDUCE_NO_REPLAY: OperatorConstraints = OperatorConstraints {
    name: "reduce_no_replay",
    categories: &[OperatorCategory::Fold],
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
    hard_range_out: &(0..=1),
    soft_range_out: &(0..=1),
    num_args: 1,
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
                   work_fn,
                   work_fn_async,
                   ident,
                   inputs,
                   is_pull,
                   singleton_output_ident,
                   arguments,
                   ..
               },
               diagnostics| {
        let [persistence] = wc.persistence_args_disallow_mutable(diagnostics);

        let write_prologue = quote_spanned! {op_span=>
            let #singleton_output_ident = #df_ident.add_state(::std::cell::RefCell::new(::std::option::Option::None));
        };
        let write_prologue_after = wc
            .persistence_as_state_lifespan(persistence)
            .map(|lifespan| quote_spanned! {op_span=>
                #df_ident.set_state_lifespan_hook(#singleton_output_ident, #lifespan, move |rcell| { rcell.replace(::std::option::Option::None); });
            }).unwrap_or_default();

        let func = &arguments[0];
        let accumulator_ident = wc.make_ident("accumulator");
        let item_ident = wc.make_ident("item");

        let foreach_body = quote_spanned! {op_span=>
            #[inline(always)]
            fn call_comb_type<Item>(
                accum: &mut ::std::option::Option<Item>,
                item: Item,
                mut func: impl ::std::ops::FnMut(&mut Item, Item),
            ) {
                match accum {
                    accum @ ::std::option::Option::None => *accum = ::std::option::Option::Some(item),
                    ::std::option::Option::Some(accum) => (func)(accum, item),
                }
            }
            #[allow(clippy::redundant_closure_call)]
            call_comb_type(&mut *#accumulator_ident, #item_ident, #func);
        };

        let assign_accum_ident = quote_spanned! {op_span=>
            #[allow(unused_mut)]
            let mut #accumulator_ident = unsafe {
                // SAFETY: handle from `#df_ident.add_state(..)`.
                #context.state_ref_unchecked(#singleton_output_ident)
            }.borrow_mut();
        };

        let write_iterator = if is_pull {
            let input = &inputs[0];
            quote_spanned! {op_span=>
                #assign_accum_ident

                let mut __was_updated = false;
                // Eagerly consume input to ensure updated state.
                {
                    let __fut = #root::compiled::pull::ForEach::new(#input, |#item_ident| {
                        #foreach_body
                        __was_updated = true;
                    });
                    let () = #work_fn_async(__fut).await;
                }

                let #ident = if __was_updated || (#context.current_tick().0 == 0 && #context.is_first_run_this_tick()) {
                    #work_fn(
                        || #root::tokio_stream::iter(
                            ::std::clone::Clone::clone(&*#accumulator_ident)
                        )
                    )
                } else {
                    #work_fn(
                        || #root::tokio_stream::iter(::std::option::Option::None)
                    )
                };
            }
        } else {
            // Is only push when used as a singleton, so no need to push to `outputs[0]`.
            quote_spanned! {op_span=>
                let #ident = #root::sinktools::for_each::ForEach::new(|#item_ident| {
                    #assign_accum_ident

                    #foreach_body
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
