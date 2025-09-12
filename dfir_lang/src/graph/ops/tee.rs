use quote::{ToTokens, quote_spanned};

use super::{
    OperatorCategory, OperatorConstraints, OperatorWriteOutput, RANGE_0, RANGE_1, RANGE_ANY,
    WriteContextArgs,
};

/// > 1 input stream, *n* output streams
///
/// Takes the input stream and delivers a copy of each item to each output.
/// > Note: Downstream operators may need explicit type annotations.
///
/// ```dfir
/// my_tee = source_iter(vec!["Hello", "World"]) -> tee();
/// my_tee -> map(|x: &str| x.to_uppercase()) -> assert_eq(["HELLO", "WORLD"]);
/// my_tee -> map(|x: &str| x.to_lowercase()) -> assert_eq(["hello", "world"]);
/// my_tee -> assert_eq(["Hello", "World"]);
/// ```
pub const TEE: OperatorConstraints = OperatorConstraints {
    name: "tee",
    categories: &[OperatorCategory::MultiOut],
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
    hard_range_out: RANGE_ANY,
    soft_range_out: &(2..),
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
                   op_name,
                   ident,
                   inputs,
                   outputs,
                   is_pull,
                   ..
               },
               _| {
        let write_iterator = if !is_pull {
            let tees = outputs
                .iter()
                .rev()
                .map(|i| i.to_token_stream())
                .reduce(|b, a|
                    quote_spanned! {op_span=> #root::futures::sink::SinkExt::fanout(#a, #b) },
                )
                .unwrap_or_else(
                    || quote_spanned! {op_span=> #root::sinktools::for_each::ForEach::new(::std::mem::drop) },
                );
            quote_spanned! {op_span=>
                let #ident = #tees;
            }
        } else {
            // Pull.
            assert_eq!(1, inputs.len());
            assert_eq!(
                1,
                outputs.len(),
                "`{}()` marked as pull should have exactly one output, actually has {}. This is a bug.",
                op_name,
                outputs.len()
            );
            let input = &inputs[0];
            quote_spanned! {op_span=>
                let #ident = #input;
            }
        };
        Ok(OperatorWriteOutput {
            write_iterator,
            ..Default::default()
        })
    },
};
