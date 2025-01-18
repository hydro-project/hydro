use quote::quote_spanned;

use super::{OperatorConstraints, OperatorWriteOutput, WriteContextArgs};

/// TODO(mingwei): docs
pub const REPEAT_N: OperatorConstraints = OperatorConstraints {
    name: "repeat_n",
    num_args: 1,
    write_fn: |wc @ &WriteContextArgs {
                   context,
                   hydroflow,
                   op_span,
                   arguments,
                   ..
               },
               diagnostics| {
        let OperatorWriteOutput {
            write_prologue,
            write_iterator,
            write_iterator_after,
        } = (super::all_once::ALL_ONCE.write_fn)(wc, diagnostics)?;

        let count_ident = wc.make_ident("count");

        let write_prologue = quote_spanned! {op_span=>
            #write_prologue

            let #count_ident = #hydroflow.add_state(::std::cell::Cell::new(0_usize));
            #hydroflow.set_state_tick_hook(#count_ident, move |cell| { cell.take(); });
        };

        // Reschedule, to repeat.
        let count_arg = &arguments[0];
        let write_iterator_after = quote_spanned! {op_span=>
            #write_iterator_after

            {
                let count_ref = #context.state_ref(#count_ident);
                if #context.is_first_loop_iteration() {
                    count_ref.set(0);
                }
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
