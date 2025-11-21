use quote::quote_spanned;

use super::{
    OperatorCategory, OperatorConstraints, OperatorWriteOutput, WriteContextArgs,
    RANGE_0, RANGE_1,
};

/// > 1 input stream, 1 output stream
///
/// For each item `i` passed in, treat `i` as an iterator and produce its items one by one.
/// The type of the input items must be iterable.
///
/// ```dfir
/// source_iter(vec![[1, 2], [3, 4], [5, 6]])
///     -> flatten()
///     -> assert_eq([1, 2, 3, 4, 5, 6]);
/// ```
pub const FLATTEN: OperatorConstraints = OperatorConstraints {
    name: "flatten",
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
                let #ident = #root::compiled::pull::Flatten::new(#input);
            }
        } else {
            let output = &outputs[0];
            quote_spanned! {op_span=>
                let #ident = #root::sinktools::flatten::Flatten::new(#output);
            }
        };
        Ok(OperatorWriteOutput {
            write_iterator,
            ..Default::default()
        })
    },
};
