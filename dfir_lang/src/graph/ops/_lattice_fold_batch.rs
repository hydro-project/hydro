use quote::{quote_spanned, ToTokens};
use syn::parse_quote;

use super::{
    DelayType, OpInstGenerics, OperatorCategory, OperatorConstraints, OperatorInstance,
    OperatorWriteOutput, PortListSpec, WriteContextArgs, RANGE_0, RANGE_1,
};

/// > 2 input streams, 1 output stream, no arguments.
///
/// Batches streaming input and releases it downstream when a signal is delivered. This allows for buffering data and delivering it later while also folding it into a single lattice data structure.
/// This operator is similar to `defer_signal` in that it batches input and releases it when a signal is given. It is also similar to `lattice_fold` in that it folds the input into a single lattice.
/// So, `_lattice_fold_batch` is a combination of both `defer_signal` and `lattice_fold`. This operator is useful when trying to combine a sequence of `defer_signal` and `lattice_fold` operators without unnecessary memory consumption.
///
/// There are two inputs to `_lattice_fold_batch`, they are `input` and `signal`.
/// `input` is the input data flow. Data that is delivered on this input is collected in order inside of the `_lattice_fold_batch` operator.
/// When anything is sent to `signal` the collected data is released downstream. The entire `signal` input is consumed each tick, so sending 5 things on `signal` will not release inputs on the next 5 consecutive ticks.
///
/// ```dfir
/// use lattices::set_union::SetUnionHashSet;
/// use lattices::set_union::SetUnionSingletonSet;
///
/// source_iter([1, 2, 3])
///     -> map(SetUnionSingletonSet::new_from)
///     -> [input]batcher;
///
/// source_iter([()])
///     -> [signal]batcher;
///
/// batcher = _lattice_fold_batch::<SetUnionHashSet<usize>>()
///     -> assert_eq([SetUnionHashSet::new_from([1, 2, 3])]);
/// ```
pub const _LATTICE_FOLD_BATCH: OperatorConstraints = OperatorConstraints {
    name: "_lattice_fold_batch",
    categories: &[OperatorCategory::CompilerFusionOperator],
    persistence_args: RANGE_0,
    type_args: &(0..=1),
    hard_range_inn: &(2..=2),
    soft_range_inn: &(2..=2),
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 0,
    is_external_input: false,
    has_singleton_output: false,
    flo_type: None,
    ports_inn: Some(|| PortListSpec::Fixed(parse_quote! { input, signal })),
    ports_out: None,
    input_delaytype_fn: |_| Some(DelayType::MonotoneAccum),
    write_fn: |wc @ &WriteContextArgs {
                   context,
                   df_ident,
                   ident,
                   op_span,
                   root,
                   inputs,
                   is_pull,
                   op_inst:
                       OperatorInstance {
                           generics: OpInstGenerics { type_args, .. },
                           ..
                       },
                   ..
               },
               _| {
        assert!(is_pull);

        let lattice_type = type_args
            .first()
            .map(ToTokens::to_token_stream)
            .unwrap_or(quote_spanned!(op_span=> _));

        let lattice_ident = wc.make_ident("lattice");

        let write_prologue = quote_spanned! {op_span=>
            let #lattice_ident = #df_ident.add_state(::std::cell::RefCell::new(<#lattice_type as ::std::default::Default>::default()));
        };

        let input = &inputs[0];
        let signal = &inputs[1];

        let write_iterator = {
            quote_spanned! {op_span=>

                {
                    let mut __lattice = unsafe {
                        // SAFETY: handle from `#df_ident.add_state(..)`.
                        #context.state_ref_unchecked(#lattice_ident)
                    }.borrow_mut();

                    for __item in #input {
                        #root::lattices::Merge::merge(&mut *__lattice, __item);
                    }
                }

                let #ident = if #signal.count() > 0 {
                    ::std::option::Option::Some(unsafe {
                        // SAFETY: handle from `#df_ident.add_state(..)`.
                        #context.state_ref_unchecked(#lattice_ident)
                    }.take())
                } else {
                    ::std::option::Option::None
                }.into_iter();
            }
        };

        Ok(OperatorWriteOutput {
            write_prologue,
            write_iterator,
            write_iterator_after: Default::default(),
            ..Default::default()
        })
    },
};
