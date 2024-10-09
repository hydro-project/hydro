use quote::quote_spanned;

use super::{
    DelayType, OperatorCategory, OperatorConstraints, OperatorWriteOutput,
    WriteContextArgs, RANGE_0, RANGE_1,
};

/// Takes a stream as input and produces a sorted version of the stream as output.
///
/// ```hydroflow
/// source_iter(vec![2, 3, 1])
///     -> sort()
///     -> assert_eq([1, 2, 3]);
/// ```
///
/// `sort` is blocking. Only the values collected within a single tick will be sorted and
/// emitted.
pub const SORT: OperatorConstraints = OperatorConstraints {
    name: "sort",
    categories: &[OperatorCategory::Persistence],
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 0,
    persistence_args: RANGE_0,
    type_args: RANGE_0,
    is_external_input: false,
    has_singleton_output: false,
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| Some(DelayType::Stratum),
    write_fn: |&WriteContextArgs {
                   op_span,
                   ident,
                   inputs,
                   is_pull,
                   ..
               },
               _| {
        assert!(is_pull);

        let input = &inputs[0];
        let write_iterator = quote_spanned! {op_span=>
            // TODO(mingwei): unnecessary extra handoff into_iter() then collect().
            // Fix requires handoff specialization.
            let #ident = {
                let mut v = #input.collect::<::std::vec::Vec<_>>();
                v.sort_unstable();
                v.into_iter()
            };
        };
        Ok(OperatorWriteOutput {
            write_iterator,
            ..Default::default()
        })
    },
};
