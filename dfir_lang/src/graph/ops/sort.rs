use quote::quote_spanned;

use super::{
    DelayType, OperatorCategory, OperatorConstraints, OperatorWriteOutput, RANGE_0, RANGE_1,
    WriteContextArgs,
};

/// Takes a stream as input and produces a sorted version of the stream as output.
///
/// ```dfir
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
    flo_type: None,
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| Some(DelayType::Stratum),
    write_fn: |&WriteContextArgs {
                   root,
                   op_span,
                   work_fn_async,
                   ident,
                   inputs,
                   outputs,
                   is_pull,
                   ..
               },
               _| {
        let write_iterator = if is_pull {
            let input = &inputs[0];
            quote_spanned! {op_span=>
                let #ident = {
                    let mut tmp = #work_fn_async(#root::dfir_pipes::pull::Pull::collect::<::std::vec::Vec<_>>(#input)).await;
                    <[_]>::sort_unstable(&mut tmp);
                    #root::dfir_pipes::pull::iter(tmp)
                };
            }
        } else {
            let output = &outputs[0];
            quote_spanned! {op_span=>
                let #ident = #root::dfir_pipes::push::Sort::new(#output);
            }
        };
        Ok(OperatorWriteOutput {
            write_iterator,
            ..Default::default()
        })
    },
};
