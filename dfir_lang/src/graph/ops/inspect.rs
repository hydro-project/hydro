use quote::quote_spanned;

use super::{
    OperatorCategory, OperatorConstraints, OperatorWriteOutput, RANGE_0, RANGE_1, WriteContextArgs,
};

/// > Arguments: A single closure `FnMut(&Item)`.
///
/// An operator which allows you to "inspect" each element of a stream without
/// modifying it. The closure is called on a reference to each item. This is
/// mainly useful for debugging as in the example below, and it is generally an
/// anti-pattern to provide a closure with side effects.
///
/// > Note: The closure has access to the [`context` object](surface_flows.mdx#the-context-object).
///
/// ```dfir
/// source_iter([1, 2, 3, 4])
///     -> inspect(|x| println!("{}", x))
///     -> assert_eq([1, 2, 3, 4]);
/// ```
pub const INSPECT: OperatorConstraints = OperatorConstraints {
    name: "inspect",
    categories: &[OperatorCategory::Map],
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
    hard_range_out: &(0..=1),
    soft_range_out: &(0..=1),
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
                let #ident = #root::futures::stream::StreamExt::inspect(#input, #func);
            }
        } else if outputs.is_empty() {
            quote_spanned! {op_span=>
                let #ident = #root::sinktools::inspect(#func, #root::sinktools::for_each::ForEach::new(::std::mem::drop));
            }
        } else {
            let output = &outputs[0];
            quote_spanned! {op_span=>
                let #ident = #root::sinktools::inspect(#func, #output);
            }
        };
        Ok(OperatorWriteOutput {
            write_iterator,
            ..Default::default()
        })
    },
};
