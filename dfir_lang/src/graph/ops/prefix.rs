use quote::quote_spanned;

use super::{
    FloType, OperatorCategory, OperatorConstraints, OperatorWriteOutput, WriteContextArgs, RANGE_0,
    RANGE_1,
};

/// Given an _unbounded_ input stream, emits full prefixes of the input, of arbitrarily increasing length, in the same order.
///
/// Will cause additional loop iterations as long as new values arrive.
pub const PREFIX: OperatorConstraints = OperatorConstraints {
    name: "prefix",
    categories: &[OperatorCategory::Fold, OperatorCategory::Windowing],
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
    hard_range_out: &(0..=1),
    soft_range_out: &(0..=1),
    num_args: 0,
    persistence_args: RANGE_0,
    type_args: RANGE_0,
    is_external_input: false,
    has_singleton_output: true,
    flo_type: Some(FloType::Windowing),
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| None,
    write_fn: |wc @ &WriteContextArgs {
                   context,
                   df_ident,
                   op_span,
                   ident,
                   is_pull,
                   inputs,
                   singleton_output_ident,
                   ..
               },
               _diagnostics| {
        assert!(is_pull);

        let write_prologue = quote_spanned! {op_span=>
            #[allow(clippy::redundant_closure_call)]
            let #singleton_output_ident = #df_ident.add_state(
                ::std::cell::RefCell::new(::std::vec::Vec::new())
            );
        };

        let vec_ident = wc.make_ident("vec");

        let input = &inputs[0];
        let write_iterator = quote_spanned! {op_span=>
            let mut #vec_ident = #context.state_ref(#singleton_output_ident).borrow_mut();
            ::std::iter::Extend::extend(&mut *#vec_ident, #input);
            let #ident = ::std::iter::IntoIterator::into_iter(::std::clone::Clone::clone(&*#vec_ident));
        };
        let write_iterator_after = quote_spanned! {op_span=>
            #context.allow_another_iteration();
        };

        Ok(OperatorWriteOutput {
            write_prologue,
            write_iterator,
            write_iterator_after,
        })
    },
};
