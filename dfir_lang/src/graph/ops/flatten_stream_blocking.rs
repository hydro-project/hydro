use quote::quote_spanned;

use super::{
    OperatorCategory, OperatorConstraints, OperatorWriteOutput, WriteContextArgs,
    RANGE_0, RANGE_1,
};

/// > 1 input stream, 1 output stream
///
/// For each item passed in, treat it as a [`Stream`](https://docs.rs/futures/latest/futures/stream/trait.Stream.html)
/// and emit its elements one by one. The type of the input items must implement `Stream`.
/// When the inner stream yields `Pending`, this operator yields `Pending` as well.
///
/// ```dfir
/// source_iter(vec![futures::stream::iter(vec![1, 2]), futures::stream::iter(vec![3])])
///     -> flatten_stream_blocking()
///     -> assert_eq([1, 2, 3]);
/// ```
pub const FLATTEN_STREAM_BLOCKING: OperatorConstraints = OperatorConstraints {
    name: "flatten_stream_blocking",
    categories: &[OperatorCategory::Flatten],
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 0,
    persistence_args: RANGE_0,
    type_args: RANGE_0,
    is_external_input: false,
    has_singleton_output: false,
    flo_type: None,
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| None,
    write_fn: |&WriteContextArgs {
                   root,
                   op_span,
                   ident,
                   inputs,
                   outputs,
                   is_pull,
                   ..
               },
               _| {
        let write_iterator = if is_pull {
            let input = &inputs[0];
            quote_spanned! {op_span=>
                let #ident = #root::dfir_pipes::pull::Pull::flatten_stream(#input);
            }
        } else {
            let output = &outputs[0];
            quote_spanned! {op_span=>
                let #ident = #root::dfir_pipes::push::flatten_stream(#output);
            }
        };
        Ok(OperatorWriteOutput {
            write_iterator,
            ..Default::default()
        })
    },
};
