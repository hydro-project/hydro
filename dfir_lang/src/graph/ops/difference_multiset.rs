use quote::{quote_spanned, ToTokens};
use syn::parse_quote;

use super::{
    DelayType, OperatorCategory, OperatorConstraints, OperatorInstance,
    OperatorWriteOutput, PortIndexValue, WriteContextArgs, RANGE_0, RANGE_1,
};

/// > 2 input streams of the same type T, 1 output stream of type T
///
/// Forms the set difference of the items in the input
/// streams, returning items in the `pos` input that are not found in the
/// `neg` input.
///
/// `difference` can be provided with one or two generic lifetime persistence arguments
/// in the same way as [`join`](#join), see [`join`'s documentation](#join) for more info.
///
/// Note multiset semantics here: each (possibly duplicated) item in the `pos` input
/// that has no match in `neg` is sent to the output.
///
/// ```dfir
/// source_iter(vec!["cat", "cat", "elephant", "elephant"]) -> [pos]diff;
/// source_iter(vec!["cat", "gorilla"]) -> [neg]diff;
/// diff = difference_multiset() -> assert_eq(["elephant", "elephant"]);
/// ```
pub const DIFFERENCE_MULTISET: OperatorConstraints = OperatorConstraints {
    name: "difference_multiset",
    categories: &[OperatorCategory::MultiIn],
    hard_range_inn: &(2..=2),
    soft_range_inn: &(2..=2),
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 0,
    persistence_args: &(0..=2),
    type_args: RANGE_0,
    is_external_input: false,
    has_singleton_output: false,
    flo_type: None,
    ports_inn: Some(|| super::PortListSpec::Fixed(parse_quote! { pos, neg })),
    ports_out: None,
    input_delaytype_fn: |idx| match idx {
        PortIndexValue::Path(path) if "neg" == path.to_token_stream().to_string() => {
            Some(DelayType::Stratum)
        }
        _else => None,
    },
    write_fn: |wc @ &WriteContextArgs {
                   op_span,
                   ident,
                   inputs,
                   op_inst: OperatorInstance { .. },
                   ..
               },
               diagnostics| {
        let OperatorWriteOutput {
            write_prologue,
            write_prologue_after,
            write_iterator,
            write_iterator_after,
        } = (super::anti_join_multiset::ANTI_JOIN_MULTISET.write_fn)(wc, diagnostics)?;

        let pos = &inputs[1];
        let write_iterator = quote_spanned! {op_span=>
            let #pos = #pos.map(|k| (k, ()));
            #write_iterator
            let #ident = #ident.map(|(k, ())| k);
        };

        Ok(OperatorWriteOutput {
            write_prologue,
            write_prologue_after,
            write_iterator,
            write_iterator_after,
        })
    },
};
