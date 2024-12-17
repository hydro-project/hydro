use quote::quote_spanned;

use super::{
    OperatorCategory, OperatorConstraints, OperatorWriteOutput, WriteContextArgs, RANGE_0, RANGE_1,
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
    flo_type: None,
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| None,
    write_fn: |wc @ &WriteContextArgs {
                   root,
                   hydroflow,
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

        let state_ident = wc.make_ident("prev_loop");

        let write_prologue = quote_spanned! {op_span=>
            let #state_ident = #hydroflow.add_state(
                ::std::cell::RefCell::new(::std::vec::Vec::<#generic_type>::new())
            );

            // TODO(mingwei): Is this needed?
            // Reset if it is a new tick.
            #hydroflow.set_state_tick_hook(#state_ident, move |rcell| { rcell.take(); });
        };

        let write_iterator = if is_pull {
            let input = &inputs[0];
            quote_spanned! {op_span=>
                let #ident = {
                    let out = #context.state_ref(#state_ident).take();
                    #input.for_each(|item| {
                        let mut vec = #context.state_ref(#state_ident).borrow_mut();
                        ::std::vec::Vec::push(&mut *vec, item);
                    });
                    println!("OUT {:?}", out);
                    out.into_iter()
                };
            }
        } else {
            let output = &outputs[0];
            quote_spanned! {op_span=>
                let #ident = {
                    let out = #context.state_ref(#state_ident).take();
                    println!("OUT {:?}", out);
                    for item in out {
                        #output.push(item);
                    }
                    #root::pusherator::for_each::ForEach::new(|item| {
                        let mut vec = #context.state_ref(#state_ident).borrow_mut();
                        println!("VEC {:?}", vec);
                        ::std::vec::Vec::push(&mut *vec, item);
                    });
                };
            }
        };

        Ok(OperatorWriteOutput {
            write_prologue,
            write_iterator,
            ..Default::default()
        })
    },
};
