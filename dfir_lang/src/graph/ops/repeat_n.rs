use quote::quote_spanned;

use super::{OperatorConstraints, OperatorWriteOutput, WriteContextArgs};

/// TODO(mingwei): docs
pub const REPEAT_N: OperatorConstraints = OperatorConstraints {
    name: "repeat_n",
    num_args: 1,
    write_fn: |wc @ &WriteContextArgs {
                   root,
                   context,
                   hydroflow,
                   op_span,
                   ident,
                   is_pull,
                   inputs,
                   outputs,
                   singleton_output_ident,
                   arguments,
                   ..
               },
               _diagnostics| {
        let write_prologue = quote_spanned! {op_span=>
            #[allow(clippy::redundant_closure_call)]
            let #singleton_output_ident = #hydroflow.add_state(
                ::std::cell::RefCell::new(::std::vec::Vec::new())
            );

            // Reset the value to the initializer fn if it is a new tick.
            #hydroflow.set_state_tick_hook(#singleton_output_ident, move |rcell| { rcell.take(); });
        };

        let write_iterator = if is_pull {
            // Pull.
            let input = &inputs[0];
            quote_spanned! {op_span=>
                let mut vec = #context.state_ref(#singleton_output_ident).borrow_mut();
                if #context.is_first_run_this_tick() {
                    *vec = #input.collect::<::std::vec::Vec<_>>();
                }
                let #ident = ::std::iter::once(::std::clone::Clone::clone(&*vec));
            }
        } else if let Some(_output) = outputs.first() {
            // Push with output.
            // TODO(mingwei): Not supported - cannot tell EOS for pusherators.
            panic!("Should not happen - batch must be at ingress to a loop, therefore ingress to a subgraph, so would be pull-based.");
        } else {
            // Push with no output.
            quote_spanned! {op_span=>
                let mut vec = #context.state_ref(#singleton_output_ident).borrow_mut();
                let #ident = #root::pusherator::for_each::ForEach::new(|item| {
                    ::std::vec::Vec::push(vec, item);
                });
            }
        };

        let count_ident = wc.make_ident("count");

        let write_prologue = quote_spanned! {op_span=>
            #write_prologue

            let #count_ident = #hydroflow.add_state(::std::cell::Cell::new(0_usize));
            #hydroflow.set_state_tick_hook(#count_ident, move |cell| { cell.take(); });
        };

        // Reschedule, to repeat.
        let count_arg = &arguments[0];
        let write_iterator_after = quote_spanned! {op_span=>
            {
                let count_ref = #context.state_ref(#count_ident);
                let count = count_ref.get() + 1;
                if count < #count_arg {
                    count_ref.set(count);
                    #context.reschedule_current_subgraph();
                }
            }
        };

        Ok(OperatorWriteOutput {
            write_prologue,
            write_iterator,
            write_iterator_after,
        })
    },
    ..super::all_once::ALL_ONCE
};
