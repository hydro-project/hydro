use quote::quote_spanned;

use super::{
    OperatorCategory, OperatorConstraints, OperatorWriteOutput,
    WriteContextArgs, RANGE_0, RANGE_1,
};

/// > 1 input stream, 1 output stream
///
/// > Arguments: A Rust closure
///
/// For each item passed in, apply the closure to generate an item to emit.
///
/// If you do not want to modify the item stream and instead only want to view
/// each item use the [`inspect`](#inspect) operator instead.
///
/// > Note: The closure has access to the [`context` object](surface_flows.mdx#the-context-object).
///
/// ```dfir
/// source_iter(vec!["hello", "world"]) -> map(|x| x.to_uppercase())
///     -> assert_eq(["HELLO", "WORLD"]);
/// ```
pub const MAP: OperatorConstraints = OperatorConstraints {
    name: "map",
    categories: &[OperatorCategory::Map],
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
                #[allow(clippy::map_clone, reason = "dfir has no explicit `cloned`/`copied` operator")]
                let #ident = #root::futures::stream::StreamExt::map(#input, #func);
            }
        } else {
            let output = &outputs[0];
            quote_spanned! {op_span=>
                let #ident = #root::sinktools::map(#func, #output);
            }
        };
        Ok(OperatorWriteOutput {
            write_iterator,
            ..Default::default()
        })
    },
};
