use quote::quote_spanned;

use super::{
    FloType, OperatorCategory, OperatorConstraints, OperatorWriteOutput, WriteContextArgs, RANGE_0,
    RANGE_1,
};
use crate::graph::{OpInstGenerics, OperatorInstance};

// TODO(mingwei): docs
pub const NEXT_LOOP: OperatorConstraints = OperatorConstraints {
    name: "next_loop",
    categories: &[OperatorCategory::Control],
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 0,
    persistence_args: RANGE_0,
    type_args: &(0..=1),
    is_external_input: false,
    has_singleton_output: false,
    flo_type: Some(FloType::NextLoop),
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| None,
    write_fn: |&WriteContextArgs {
                   root,
                   context,
                   op_span,
                   ident,
                   is_pull,
                   inputs,
                   outputs,
                   op_inst:
                       OperatorInstance {
                           generics: OpInstGenerics { type_args, .. },
                           ..
                       },
                   ..
               },
               _| {
        let generic_type = type_args
            .first()
            .map(quote::ToTokens::to_token_stream)
            .unwrap_or(quote_spanned!(op_span=> _));

        let write_iterator = if is_pull {
            let input = &inputs[0];
            quote_spanned! {op_span=>
                let #ident = {
                    fn check_input<Iter: ::std::iter::Iterator<Item = Item>, Item>(iter: Iter, filter: bool) -> impl ::std::iter::Iterator<Item = Item> {
                        iter.filter(move |_item| filter)
                    }
                    check_input::<_, #generic_type>(#input, #context.is_first_run_this_tick() || #context.was_rescheduled())
                };
            }
        } else {
            let output = &outputs[0];
            quote_spanned! {op_span=>
                let #ident = {
                    fn check_output<Push: #root::pusherator::Pusherator<Item = Item>, Item>(push: Push, filter: bool) -> impl #root::pusherator::Pusherator<Item = Item> {
                        // Don't continue to the next loop iteration if we weren't rescheduled, to prevent spinning.
                        #root::pusherator::filter::Filter::new(push, move |_item| filter)
                    }
                    check_output::<_, #generic_type>(#output, #context.is_first_run_this_tick() || #context.was_rescheduled())
                };
            }
        };

        Ok(OperatorWriteOutput {
            write_iterator,
            ..Default::default()
        })
    },
};
