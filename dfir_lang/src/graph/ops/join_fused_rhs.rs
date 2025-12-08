use quote::{ToTokens, quote_spanned};
use syn::parse_quote;

use super::{
    DelayType, OperatorCategory, OperatorConstraints, OperatorWriteOutput, PortIndexValue, RANGE_0,
    RANGE_1, WriteContextArgs,
};

/// See `join_fused_lhs`
///
/// This operator is identical to `join_fused_lhs` except that it is the right hand side that is fused instead of the left hand side.
pub const JOIN_FUSED_RHS: OperatorConstraints = OperatorConstraints {
    name: "join_fused_rhs",
    categories: &[OperatorCategory::MultiIn],
    hard_range_inn: &(2..=2),
    soft_range_inn: &(2..=2),
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 1,
    persistence_args: &(0..=2),
    type_args: RANGE_0,
    is_external_input: false,
    has_singleton_output: false,
    flo_type: None,
    ports_inn: Some(|| super::PortListSpec::Fixed(parse_quote! { 0, 1 })),
    ports_out: None,
    input_delaytype_fn: |idx| match idx {
        PortIndexValue::Int(path) if "1" == path.to_token_stream().to_string() => {
            Some(DelayType::Stratum)
        }
        _ => None,
    },
    write_fn: |wc @ &WriteContextArgs {
                   root,
                   op_span,
                   ident,
                   inputs,
                   ..
               },
               diagnostics| {
        let inputs = inputs.iter().cloned().rev().collect::<Vec<_>>();

        let wc = WriteContextArgs {
            inputs: &inputs[..],
            ..wc.clone()
        };

        let OperatorWriteOutput {
            write_prologue,
            write_prologue_after,
            write_iterator,
            write_iterator_after,
        } = (super::join_fused_lhs::JOIN_FUSED_LHS.write_fn)(&wc, diagnostics)?;

        let write_iterator = quote_spanned! {op_span=>
            #write_iterator
            let #ident = #root::futures::stream::StreamExt::map(#ident, |(k, (v1, v2))| (k, (v2, v1)));
        };

        Ok(OperatorWriteOutput {
            write_prologue,
            write_prologue_after,
            write_iterator,
            write_iterator_after,
        })
    },
};
