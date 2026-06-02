use quote::quote_spanned;

use super::{
    OperatorCategory, OperatorConstraints, OperatorWriteOutput, RANGE_0, RANGE_1,
    WriteContextArgs,
};

/// Like sort, takes a stream as input and produces a version of the stream as output.
/// This operator sorts according to the key extracted by the closure.
///
/// > Note: The closure has access to the [`context` object](surface_flows.mdx#the-context-object).
///
/// ```dfir
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
    flo_type: None,
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| None,
    write_fn: |&WriteContextArgs {
                   root,
                   op_span,
                   work_fn_async,
                   ident,
                   inputs,
                   outputs,
                   is_pull,
                   arguments,
                   ..
               },
               _| {
        let write_iterator = if is_pull {
            let input = &inputs[0];
            quote_spanned! {op_span=>
                let #ident = {
                    let mut tmp = #work_fn_async(#root::dfir_pipes::pull::Pull::collect::<::std::vec::Vec<_>>(#input)).await;
                    #root::util::sort_unstable_by_key_hrtb(&mut tmp, #arguments);
                    #root::dfir_pipes::pull::iter(tmp)
                };
            }
        } else {
            let output = &outputs[0];
            quote_spanned! {op_span=>
                let #ident = #root::dfir_pipes::push::Fold::new(
                    ::std::vec::Vec::new(),
                    |__buf: &mut ::std::vec::Vec<_>, __item| {
                        __buf.push(__item);
                    },
                    #root::dfir_pipes::push::flat_map(
                        |__buf: ::std::vec::Vec<_>| {
                            let mut __buf = __buf;
                            #root::util::sort_unstable_by_key_hrtb(&mut __buf, #arguments);
                            __buf
                        },
                        #output,
                    ),
                );
            }
        };
        Ok(OperatorWriteOutput {
            write_iterator,
            ..Default::default()
        })
    },
};
