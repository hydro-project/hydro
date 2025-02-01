use quote::quote_spanned;

use super::{
    FloType, OperatorCategory, OperatorConstraints, OperatorWriteOutput, WriteContextArgs, RANGE_0,
    RANGE_1,
};

/// TODO(mingwei): docs
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
                   hydroflow,
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
            let #singleton_output_ident = #hydroflow.add_state(
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

        Ok(OperatorWriteOutput {
            write_prologue,
            write_iterator,
            ..Default::default()
        })
    },
};
