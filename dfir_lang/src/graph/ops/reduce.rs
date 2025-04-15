use quote::quote_spanned;

use super::{
    DelayType, OperatorCategory, OperatorConstraints, OperatorWriteOutput, Persistence, RANGE_0,
    RANGE_1, WriteContextArgs,
};

/// > 1 input stream, 1 output stream
///
/// > Arguments: a closure which itself takes two arguments:
/// > an `&mut Accum` accumulator mutable reference, and an `Item`. The closure should merge the item
/// > into the accumulator.
///
/// Akin to Rust's built-in [`reduce`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.reduce)
/// operator, except that it takes the accumulator by `&mut` instead of by value. Reduces every
/// item into an accumulator by applying a closure, returning the final result.
///
/// > Note: The closure has access to the [`context` object](surface_flows.mdx#the-context-object).
///
/// `reduce` can also be provided with one generic lifetime persistence argument, either
/// `'tick` or `'static`, to specify how data persists. With `'tick`, values will only be collected
/// within the same tick. With `'static`, the accumulated value will be remembered across ticks and
/// items are aggregated with items arriving in later ticks. When not explicitly specified
/// persistence defaults to `'tick`.
///
/// ```dfir
/// source_iter([1,2,3,4,5])
///     -> reduce::<'tick>(|accum: &mut _, elem| {
///         *accum *= elem;
///     })
///     -> assert_eq([120]);
/// ```
pub const REDUCE: OperatorConstraints = OperatorConstraints {
    name: "reduce",
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
                   ident,
                   inputs,
                   is_pull,
                   singleton_output_ident,
                   work_fn,
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
        let iterator_item_ident = wc.make_ident("iterator_item");

        let iterator_foreach = quote_spanned! {op_span=>
            #[inline(always)]
            fn call_comb_type<Item>(
                accum: &mut Option<Item>,
                item: Item,
                func: impl Fn(&mut Item, Item),
            ) {
                match accum {
                    accum @ None => *accum = Some(item),
                    Some(accum) => (func)(accum, item),
                }
            }
            #[allow(clippy::redundant_closure_call)]
            call_comb_type(&mut *#accumulator_ident, #iterator_item_ident, #func);
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
                let #ident = {
                    #assign_accum_ident

                    #work_fn(|| #input.for_each(|#iterator_item_ident| {
                        #iterator_foreach
                    }));

                    #[allow(clippy::clone_on_copy)]
                    {
                        ::std::iter::IntoIterator::into_iter(#work_fn(|| ::std::clone::Clone::clone(&*#accumulator_ident)))
                    }
                };
            }
        } else {
            // Is only push when used as a singleton, so no need to push to `outputs[0]`.
            quote_spanned! {op_span=>
                let #ident = #root::pusherator::for_each::ForEach::new(|#iterator_item_ident| {
                    #assign_accum_ident

                    #iterator_foreach
                });
            }
        };

        let write_iterator_after = if Persistence::Static == persistence {
            quote_spanned! {op_span=>
                #context.schedule_subgraph(#context.current_subgraph(), false);
            }
        } else {
            Default::default()
        };

        Ok(OperatorWriteOutput {
            write_prologue,
            write_prologue_after,
            write_iterator,
            write_iterator_after,
        })
    },
};
