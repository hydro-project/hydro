use quote::quote_spanned;
use syn::parse_quote;

use super::{
    OperatorCategory, OperatorConstraints, OperatorWriteOutput, WriteContextArgs,
    RANGE_0, RANGE_1,
};

/// > 1 input stream of pair tuples `(A, B)`, 2 output streams
///
/// Takes the input stream of pairs and unzips each one, delivers each item to
/// its corresponding side.
///
/// ```dfir
/// my_unzip = source_iter(vec![("Hello", "Foo"), ("World", "Bar")]) -> unzip();
/// my_unzip[0] -> assert_eq(["Hello", "World"]);
/// my_unzip[1] -> assert_eq(["Foo", "Bar"]);
/// ```
pub const UNZIP: OperatorConstraints = OperatorConstraints {
    name: "unzip",
    categories: &[OperatorCategory::MultiOut],
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
    hard_range_out: &(2..=2),
    soft_range_out: &(2..=2),
    num_args: 0,
    persistence_args: RANGE_0,
    type_args: RANGE_0,
    is_external_input: false,
    has_singleton_output: false,
    flo_type: None,
    ports_inn: None,
    ports_out: Some(|| super::PortListSpec::Fixed(parse_quote!(0, 1))),
    input_delaytype_fn: |_| None,
    write_fn: |&WriteContextArgs {
                   root,
                   op_span,
                   ident,
                   outputs,
                   is_pull,
                   ..
               },
               _| {
        assert!(!is_pull);
        let output0 = &outputs[0];
        let output1 = &outputs[1];
        let write_iterator = quote_spanned! {op_span=>
            let #ident = #root::sinktools::unzip(#output0, #output1);
        };
        Ok(OperatorWriteOutput {
            write_iterator,
            ..Default::default()
        })
    },
};
