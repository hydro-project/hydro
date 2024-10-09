use quote::quote_spanned;

use super::{
    DelayType, OperatorCategory, OperatorConstraints, OperatorWriteOutput,
    WriteContextArgs, RANGE_0, RANGE_1,
};

/// Like sort, takes a stream as input and produces a version of the stream as output.
/// This operator sorts according to the key extracted by the closure.
///
/// > Note: The closure has access to the [`context` object](surface_flows.mdx#the-context-object).
///
/// ```hydroflow
/// source_iter(vec![(2, 'y'), (3, 'x'), (1, 'z')])
///     -> sort_by_key(|(k, _v)| k)
///     -> assert_eq([(1, 'z'), (2, 'y'), (3, 'x')]);
/// ```
pub const SORT_BY_KEY: OperatorConstraints = OperatorConstraints {
    name: "sort_by_key",
    categories: &[OperatorCategory::Persistence],
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 1,
    persistence_args: RANGE_0,
    type_args: RANGE_0,
    is_external_input: false,
    has_singleton_output: false,
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| Some(DelayType::Stratum),
    write_fn: |&WriteContextArgs {
                   root,
                   op_span,
                   ident,
                   inputs,
                   is_pull,
                   arguments,
                   ..
               },
               _| {
        assert!(is_pull);
        let input = &inputs[0];
        let write_iterator = quote_spanned! {op_span=>
            let mut tmp = #input.collect::<Vec<_>>();
            #root::util::sort_unstable_by_key_hrtb(&mut tmp, #arguments);
            let #ident = tmp.into_iter();
        };
        Ok(OperatorWriteOutput {
            write_iterator,
            ..Default::default()
        })
    },
};
