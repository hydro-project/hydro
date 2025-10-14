use quote::quote_spanned;

use super::{
    FloType, OperatorCategory, OperatorConstraints, OperatorWriteOutput, RANGE_0, RANGE_1,
    WriteContextArgs,
};

/// > 0 input streams, 1 output stream
///
/// > Arguments: An iterable Rust object.
///
/// Takes the iterable object and delivers its elements downstream
/// one by one.
///
/// Note that all elements are emitted during the first tick.
///
/// ```dfir
///     source_iter(vec!["Hello", "World"])
///         -> for_each(|x| println!("{}", x));
/// ```
pub const SOURCE_ITER: OperatorConstraints = OperatorConstraints {
    name: "source_iter",
    categories: &[OperatorCategory::Source],
    hard_range_inn: RANGE_0,
    soft_range_inn: RANGE_0,
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 1,
    persistence_args: RANGE_0,
    type_args: RANGE_0,
    is_external_input: false,
    has_singleton_output: false,
    flo_type: Some(FloType::Source),
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| None,
    write_fn: |wc @ &WriteContextArgs {
                   root,
                   op_span,
                   ident,
                   arguments,
                   ..
               },
               _| {
        let iter = &arguments[0];
        let iter_ident = wc.make_ident("iter");
        let write_prologue = quote_spanned! {op_span=>
            let mut #iter_ident = {
                #[inline(always)]
                fn check_iter<IntoIter: ::std::iter::IntoIterator<Item = Item>, Item>(into_iter: IntoIter) -> impl ::std::iter::Iterator<Item = Item> {
                    ::std::iter::IntoIterator::into_iter(into_iter)
                }
                check_iter(#iter)
            };
        };
        let write_iterator = quote_spanned! {op_span=>
            let #ident = #root::futures::stream::iter(#iter_ident.by_ref());
        };
        Ok(OperatorWriteOutput {
            write_prologue,
            write_iterator,
            ..Default::default()
        })
    },
};
