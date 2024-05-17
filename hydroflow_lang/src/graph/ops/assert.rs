use syn::parse_quote_spanned;

use super::{
    OperatorCategory, OperatorConstraints, WriteContextArgs, RANGE_0, RANGE_1,
};

/// > 1 input stream, 1 optional output stream
/// > Arguments: a predicate function that will be applied to each item in the stream
///
/// If the predicate returns false for any input item then the operator will panic at runtime.
///
/// ```hydroflow
/// source_iter([1, 2, 3])
///     -> assert(|x| *x > 0)
///     -> assert_eq([1, 2, 3]);
/// ```
pub const ASSERT: OperatorConstraints = OperatorConstraints {
    name: "assert",
    categories: &[OperatorCategory::Control],
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
    hard_range_out: &(0..=1),
    soft_range_out: &(0..=1),
    num_args: 1,
    persistence_args: RANGE_0,
    type_args: RANGE_0,
    is_external_input: false,
    has_singleton_output: false,
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| None,
    flow_prop_fn: None,
    write_fn: |wc @ &WriteContextArgs {
                   op_span, arguments, ..
               },
               diagnostics| {
        let arg = &arguments[0];

        let arguments = &parse_quote_spanned! {op_span=>
            |x| {
                // This is to help constrain the types so that type inference works nicely.
                fn __constrain_types<T>(f: impl Fn(&T) -> bool, x: &T) -> bool {
                    (f)(x)
                }
                assert!(__constrain_types(#arg, x));
            }
        };

        let wc = WriteContextArgs {
            arguments,
            ..wc.clone()
        };

        (super::inspect::INSPECT.write_fn)(&wc, diagnostics)
    },
};
