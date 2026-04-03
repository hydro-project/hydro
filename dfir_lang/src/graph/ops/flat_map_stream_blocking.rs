use quote::quote_spanned;

use super::{
    OperatorCategory, OperatorConstraints, OperatorWriteOutput, WriteContextArgs,
    RANGE_0, RANGE_1,
};

/// > 1 input stream, 1 output stream
///
/// > Arguments: A Rust closure that maps each item to a [`Stream`](https://docs.rs/futures/latest/futures/stream/trait.Stream.html)
///
/// For each item passed in, the closure is applied to produce a `Stream`, and the items
/// of that stream are emitted one by one. When the inner stream yields `Pending`, this
/// operator yields `Pending` as well.
///
/// ```dfir
/// source_iter(vec![1, 2, 3])
///     -> flat_map_stream_blocking(|x| futures::stream::iter(vec![x, x * 10]))
///     -> assert_eq([1, 10, 2, 20, 3, 30]);
/// ```
pub const FLAT_MAP_STREAM_BLOCKING: OperatorConstraints = OperatorConstraints {
    name: "flat_map_stream_blocking",
    categories: &[OperatorCategory::Flatten],
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 1,
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
                   arguments,
                   ..
               },
               _| {
        let func = &arguments[0];
        let write_iterator = if is_pull {
            let input = &inputs[0];
            quote_spanned! {op_span=>
                let #ident = #root::dfir_pipes::pull::Pull::flat_map_stream(#input, #func);
            }
        } else {
            let output = &outputs[0];
            quote_spanned! {op_span=>
                let #ident = #root::dfir_pipes::push::flat_map_stream(#func, #output);
            }
        };
        Ok(OperatorWriteOutput {
            write_iterator,
            ..Default::default()
        })
    },
};
