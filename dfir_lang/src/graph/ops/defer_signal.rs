use quote::quote_spanned;
use syn::parse_quote;

use super::{
    DelayType, OperatorCategory, OperatorConstraints, OperatorWriteOutput, RANGE_0, RANGE_1,
    WriteContextArgs,
};

/// > 2 input streams, 1 output stream, no arguments.
///
/// Defers streaming input and releases it downstream when a signal is delivered. The order of input is preserved. This allows for buffering data and delivering it at a later, chosen, tick.
///
/// There are two inputs to `defer_signal`, they are `input` and `signal`.
/// `input` is the input data flow. Data that is delivered on this input is collected in order inside of the `defer_signal` operator.
/// When anything is sent to `signal` the collected data is released downstream. The entire `signal` input is consumed each tick, so sending 5 things on `signal` will not release inputs on the next 5 consecutive ticks.
///
/// ```dfir
/// gate = defer_signal();
///
/// source_iter([1, 2, 3]) -> [input]gate;
/// source_iter([()]) -> [signal]gate;
///
/// gate -> assert_eq([1, 2, 3]);
/// ```
pub const DEFER_SIGNAL: OperatorConstraints = OperatorConstraints {
    name: "defer_signal",
    categories: &[OperatorCategory::Persistence],
    persistence_args: RANGE_0,
    type_args: RANGE_0,
    hard_range_inn: &(2..=2),
    soft_range_inn: &(2..=2),
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 0,
    is_external_input: false,
    has_singleton_output: false,
    flo_type: None,
    ports_inn: Some(|| super::PortListSpec::Fixed(parse_quote! { input, signal })),
    ports_out: None,
    input_delaytype_fn: |_| Some(DelayType::Stratum),
    write_fn: |wc @ &WriteContextArgs {
                   root,
                   context,
                   df_ident,
                   ident,
                   op_span,
                   inputs,
                   is_pull,
                   ..
               },
               _| {
        assert!(is_pull);

        let internal_buffer = wc.make_ident("internal_buffer");
        let borrow_ident = wc.make_ident("borrow_ident");

        // TODO(mingwei): different lifetimes? `'tick`?
        let write_prologue = quote_spanned! {op_span=>
            let #internal_buffer = #df_ident.add_state(::std::cell::RefCell::new(::std::collections::VecDeque::new()));
        };

        let input = &inputs[0];
        let signal = &inputs[1];

        let write_iterator = {
            quote_spanned! {op_span=>
                let mut #borrow_ident = unsafe {
                    // SAFETY: handle from `#df_ident.add_state(..)`.
                    #context.state_ref_unchecked(#internal_buffer)
                }.borrow_mut();

                let #ident = #root::compiled::pull::DeferSignal::new(
                    #input,
                    #root::futures::stream::StreamExt::fuse(#signal),
                    &mut *#borrow_ident,
                );
            }
        };

        Ok(OperatorWriteOutput {
            write_prologue,
            write_iterator,
            write_iterator_after: Default::default(),
            ..Default::default()
        })
    },
};
