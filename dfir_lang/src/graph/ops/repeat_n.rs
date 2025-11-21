use quote::quote_spanned;

use super::{
    FloType, OperatorCategory, OperatorConstraints, OperatorWriteOutput, RANGE_0, RANGE_1,
    WriteContextArgs,
};

/// Given a _bounded_ input stream, emits all values repeatedly over `N` iterations, in the same order.
///
/// Will cause `N` loop iterations.
pub const REPEAT_N: OperatorConstraints = OperatorConstraints {
    name: "repeat_n",
    categories: &[OperatorCategory::Windowing],
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 1,
    persistence_args: RANGE_0,
    type_args: RANGE_0,
    is_external_input: false,
    has_singleton_output: true,
    flo_type: Some(FloType::Windowing),
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| None,
    write_fn: |wc @ &WriteContextArgs {
                   root,
                   context,
                   df_ident,
                   op_span,
                   arguments,
                   ident,
                   is_pull,
                   inputs,
                   singleton_output_ident,
                   ..
               },
               _diagnostics| {
        assert!(is_pull);

        let write_prologue = quote_spanned! {op_span=>
            #[allow(clippy::redundant_closure_call)]
            let #singleton_output_ident = #df_ident.add_state(
                ::std::cell::RefCell::new(::std::vec::Vec::new())
            );
        };

        let vec_ident = wc.make_ident("vec");

        let input = &inputs[0];
        let write_iterator = quote_spanned! {op_span=>
            let mut #vec_ident = unsafe {
                // SAFETY: handle from `#df_ident.add_state(..)`.
                #context.state_ref_unchecked(#singleton_output_ident)
            }.borrow_mut();

            let #ident = {
                fn constrain_types<'ctx, Pull, Item>(input: Pull, vec: &'ctx mut Vec<Item>) -> impl 'ctx + #root::futures::stream::Stream<Item = Item>
                where
                    Pull: 'ctx + #root::futures::stream::Stream<Item = Item>,
                    Item: ::std::clone::Clone,
                {
                    #root::compiled::pull::Persist::new(input, vec, 0)
                }
                constrain_types(#input, &mut *#vec_ident)
            };

            // TODO(mingwei): remove old code. This code is copied (is it correct?) from `prefix()`, we should DRY this.
            // if 0 == #context.loop_iter_count() {
            //     *#vec_ident = #input.collect::<::std::vec::Vec<_>>();
            // }
            // let #ident = ::std::iter::IntoIterator::into_iter(::std::clone::Clone::clone(&*#vec_ident));
        };

        // Reschedule, to repeat.
        // TODO(mingwei): ensure `count_arg` is a usize literal (or eval it to such).
        let count_arg = &arguments[0];
        let write_iterator_after = quote_spanned! {op_span=>
            {
                if #context.loop_iter_count() + 1 < #count_arg {
                    #context.reschedule_loop_block();
                }
            }
        };

        Ok(OperatorWriteOutput {
            write_prologue,
            write_iterator,
            write_iterator_after,
            ..Default::default()
        })
    },
};
