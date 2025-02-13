use quote::quote_spanned;

use super::{
    FloType, OperatorCategory, OperatorConstraints, OperatorWriteOutput, WriteContextArgs, RANGE_0,
    RANGE_1,
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

        let count_ident = wc.make_ident("count");

        let write_prologue = quote_spanned! {op_span=>
            #[allow(clippy::redundant_closure_call)]
            let #singleton_output_ident = #df_ident.add_state(
                ::std::cell::RefCell::new(::std::vec::Vec::new())
            );

            // TODO(mingwei): Is this needed?
            // Reset the value to the initializer fn if it is a new tick.
            #df_ident.set_state_tick_hook(#singleton_output_ident, move |rcell| { rcell.take(); });

            let #count_ident = #df_ident.add_state(::std::cell::Cell::new(0_usize));
            #df_ident.set_state_tick_hook(#count_ident, move |cell| { cell.take(); });
        };

        let vec_ident = wc.make_ident("vec");

        let input = &inputs[0];
        let write_iterator = quote_spanned! {op_span=>
            let mut #vec_ident = #context.state_ref(#singleton_output_ident).borrow_mut();
            if #context.is_first_run_this_tick() {
                *#vec_ident = #input.collect::<::std::vec::Vec<_>>();
            }
            let #ident = std::iter::IntoIterator::into_iter(::std::clone::Clone::clone(&*#vec_ident));
        };

        // Reschedule, to repeat.
        let count_arg = &arguments[0];
        let write_iterator_after = quote_spanned! {op_span=>
            {
                let count_ref = #context.state_ref(#count_ident);
                println!("{}", context.is_first_loop_iteration());
                if #context.is_first_loop_iteration() {
                    count_ref.set(0);
                }
                let count = count_ref.get() + 1;
                if count < #count_arg {
                    count_ref.set(count);
                    #context.reschedule_loop_block();
                }
            }
        };

        Ok(OperatorWriteOutput {
            write_prologue,
            write_iterator,
            write_iterator_after,
        })
    },
};
