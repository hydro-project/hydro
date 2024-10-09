use quote::{quote_spanned, ToTokens};
use syn::parse_quote;

use super::{
    DelayType, OperatorCategory, OperatorConstraints,
    OperatorWriteOutput, WriteContextArgs, RANGE_0, RANGE_1,
};
use crate::graph::PortIndexValue;

/// > 2 input streams, 1 output stream, no arguments.
///
/// Operates like cross-join, but treats one of the inputs as a "singleton"-like stream, emitting
/// ignoring everything after the first element. This operator blocks on the singleton input, and
/// then joins it with all the elements in the other stream if an element is present. This operator
/// is useful when a singleton input must be used to transform elements of a stream, since unlike
/// cross-product it avoids cloning the stream of inputs. It is also useful for creating conditional
/// branches, since the operator short circuits if the singleton input produces no values.
///
/// There are two inputs to `cross_singleton`, they are `input` and `single`.
/// `input` is the input data flow, and `single` is the singleton input.
///
/// ```hydroflow
/// join = cross_singleton();
///
/// source_iter([1, 2, 3]) -> [input]join;
/// source_iter([0]) -> [single]join;
///
/// join -> assert_eq([(1, 0), (2, 0), (3, 0)]);
/// ```
pub const CROSS_SINGLETON: OperatorConstraints = OperatorConstraints {
    name: "cross_singleton",
    categories: &[OperatorCategory::MultiIn],
    persistence_args: RANGE_0,
    type_args: RANGE_0,
    hard_range_inn: &(2..=2),
    soft_range_inn: &(2..=2),
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 0,
    is_external_input: false,
    has_singleton_output: false,
    ports_inn: Some(|| super::PortListSpec::Fixed(parse_quote! { input, single })),
    ports_out: None,
    input_delaytype_fn: |idx| match idx {
        PortIndexValue::Path(path) if "single" == path.to_token_stream().to_string() => {
            Some(DelayType::Stratum)
        }
        _else => None,
    },
    write_fn: |wc @ &WriteContextArgs {
                   ident,
                   op_span,
                   inputs,
                   is_pull,
                   ..
               },
               _diagnostics| {
        assert!(is_pull);

        let input = &inputs[0];
        let singleton = &inputs[1];
        let s_taken_ident = wc.make_ident("singleton_taken");
        let singleton_value_ident = wc.make_ident("singleton_value");

        let write_iterator = quote_spanned! {op_span=>
            let mut #s_taken_ident = #singleton;
            let #ident = #s_taken_ident.next().map(|#singleton_value_ident| {
                #input.map(move |x| (x, ::std::clone::Clone::clone(&#singleton_value_ident)))
            }).into_iter().flatten();
        };

        Ok(OperatorWriteOutput {
            write_prologue: Default::default(),
            write_iterator,
            ..Default::default()
        })
    },
};
