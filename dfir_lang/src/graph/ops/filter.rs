use quote::quote_spanned;

use super::{
    OperatorCategory, OperatorConstraints, OperatorWriteOutput,
    WriteContextArgs, RANGE_0, RANGE_1,
};

/// Filter outputs a subsequence of the items it receives at its input, according to a
/// Rust boolean closure passed in as an argument.
///
/// The closure receives a reference `&T` rather than an owned value `T` because filtering does
/// not modify or take ownership of the values. If you need to modify the values while filtering
/// use [`filter_map`](#filter_map) instead.
///
/// > Note: The closure has access to the [`context` object](surface_flows.mdx#the-context-object).
///
/// ```dfir
/// source_iter(vec!["hello", "world"]) -> filter(|x| x.starts_with('w'))
///     -> assert_eq(["world"]);
/// ```
pub const FILTER: OperatorConstraints = OperatorConstraints {
    name: "filter",
    categories: &[OperatorCategory::Filter],
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
                let #ident = #input.filter(#func);
            }
        } else {
            let output = &outputs[0];
            quote_spanned! {op_span=>
                let #ident = #root::sinktools::filter(#func, #output);
            }
        };
        Ok(OperatorWriteOutput {
            write_iterator,
            ..Default::default()
        })
    },
};
