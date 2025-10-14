use quote::{quote_spanned, ToTokens};

use super::{
    OperatorCategory, OperatorConstraints, OperatorWriteOutput,
    WriteContextArgs, RANGE_0, RANGE_1, RANGE_ANY,
};

/// > *n* input streams of the same type, 1 output stream of the same type
///
/// Unions an arbitrary number of input streams into a single stream. Each input sequence is a subsequence of the output, but no guarantee is given on how the inputs are interleaved.
///
/// Since `union` has multiple input streams, it needs to be assigned to
/// a variable to reference its multiple input ports across statements.
///
/// ```dfir
/// source_iter(vec!["hello", "world"]) -> my_union;
/// source_iter(vec!["stay", "gold"]) -> my_union;
/// source_iter(vec!["don't", "give", "up"]) -> my_union;
/// my_union = union()
///     -> map(|x| x.to_uppercase())
///     -> assert_eq(["HELLO", "WORLD", "STAY", "GOLD", "DON'T", "GIVE", "UP"]);
/// ```
pub const UNION: OperatorConstraints = OperatorConstraints {
    name: "union",
    categories: &[OperatorCategory::MultiIn],
    hard_range_inn: RANGE_ANY,
    soft_range_inn: &(2..),
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
                   arguments,
                   ..
               },
               _| {
        let max_output = arguments.get(0); // used in chain_first_n

        let write_iterator = if is_pull {
            let mut chain_expr = quote_spanned! {op_span=>
                // NOTE(mingwei): tokio `StreamExt::merge` may make more sense, but also might inline worse.
                #root::futures::stream::StreamExt::chain(a, b)
            };
            if let Some(max) = max_output {
                chain_expr = quote_spanned! {op_span=>
                    #chain_expr.take(#max)
                };
            }

            let chains = inputs
                .iter()
                .map(|i| i.to_token_stream())
                .reduce(|a, b| quote_spanned! {op_span=> check_inputs(#a, #b) })
                .unwrap_or_else(|| quote_spanned! {op_span=> ::std::iter::empty() });
            quote_spanned! {op_span=>
                let #ident = {
                    #[allow(unused)]
                    #[inline(always)]
                    fn check_inputs<A: #root::futures::stream::Stream<Item = Item>, B: #root::futures::stream::Stream<Item = Item>, Item>(a: A, b: B) -> impl #root::futures::stream::Stream<Item = Item> {
                        #chain_expr
                    }
                    #chains
                };
            }
        } else {
            assert_eq!(1, outputs.len());
            assert_eq!(None, max_output);
            let output = &outputs[0];
            quote_spanned! {op_span=>
                let #ident = #output;
            }
        };
        Ok(OperatorWriteOutput {
            write_iterator,
            ..Default::default()
        })
    },
};
