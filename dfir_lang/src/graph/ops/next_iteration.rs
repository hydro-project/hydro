use quote::quote_spanned;

use super::{
    FloType, OperatorCategory, OperatorConstraints, OperatorWriteOutput, RANGE_0, RANGE_1,
    WriteContextArgs,
};

// TODO(mingwei)
pub const NEXT_ITERATION: OperatorConstraints = OperatorConstraints {
    name: "next_iteration",
    categories: &[OperatorCategory::Control],
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 0,
    persistence_args: RANGE_0,
    type_args: RANGE_0,
    is_external_input: false,
    has_singleton_output: false,
    flo_type: Some(FloType::NextIteration),
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| None,
    write_fn: |&WriteContextArgs {
                   root,
                   context,
                   ident,
                   is_pull,
                   inputs,
                   op_span,
                   ..
               },
               _diagnostics| {
        assert!(is_pull);

        let input = &inputs[0];
        let write_iterator = quote_spanned! {op_span=>
            // Discard items from previous loop executions.
            let #ident = #root::tokio_stream::StreamExt::filter(#input, |_| 0 != #context.loop_iter_count());
        };

        Ok(OperatorWriteOutput {
            write_iterator,
            ..Default::default()
        })
    },
};
