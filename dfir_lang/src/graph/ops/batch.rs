use quote::quote_spanned;

use super::{
    FloType, OperatorCategory, OperatorConstraints, OperatorWriteOutput, WriteContextArgs, RANGE_0,
    RANGE_1,
};

/// TODO(mingwei): docs
pub const BATCH: OperatorConstraints = OperatorConstraints {
    name: "batch",
    categories: &[OperatorCategory::Fold, OperatorCategory::Windowing],
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
    hard_range_out: &(0..=1),
    soft_range_out: &(0..=1),
    num_args: 0,
    persistence_args: RANGE_0,
    type_args: RANGE_0,
    is_external_input: false,
    has_singleton_output: true,
    flo_type: Some(FloType::Windowing),
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| None,
    write_fn: |_wc @ &WriteContextArgs {
                   root,
                   context,
                   hydroflow,
                   op_span,
                   ident,
                   is_pull,
                   inputs,
                   outputs,
                   singleton_output_ident,
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
                *vec = #input.collect::<::std::vec::Vec<_>>();
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

        Ok(OperatorWriteOutput {
            write_prologue,
            write_iterator,
            ..Default::default()
        })
    },
};
