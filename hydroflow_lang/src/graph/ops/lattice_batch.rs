use super::{
    FlowProperties, FlowPropertyVal, OperatorConstraints, OperatorWriteOutput, WriteContextArgs,
    RANGE_0, RANGE_1,
};

use crate::graph::{OpInstGenerics, OperatorInstance};
use quote::{quote_spanned, ToTokens};

/// > 1 input stream, 1 output stream
///
/// > Arguments: The one and only argument is the receive end of a tokio channel that signals when to release the batch downstream.
///
/// Given a [`Stream`](https://docs.rs/futures/latest/futures/stream/trait.Stream.html)
/// created in Rust code, `lattice_batch`
/// is passed the receive end of the channel and when receiving any element
/// will pass through all received inputs to the output unchanged.
///
/// ```rustbook
///     let (tx, rx) = hydroflow::util::unbounded_channel::<()>();
///
///     // Will print 0, 1, 2, 3, 4 each on a new line just once.
///     let mut df = hydroflow::hydroflow_syntax! {
///         repeat_iter(0..5)
///             -> map(|x| hydroflow::lattices::Max::new(x))
///             -> lattice_batch::<hydroflow::lattices::Max<usize>>(rx)
///             -> for_each(|x| { println!("{x:?}"); });
///     };
///
///     tx.send(()).unwrap();
///
///     df.run_available();
/// ```
#[hydroflow_internalmacro::operator_docgen]
pub const LATTICE_BATCH: OperatorConstraints = OperatorConstraints {
    name: "lattice_batch",
    persistence_args: RANGE_0,
    type_args: &(0..=1),
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 1,
    is_external_input: false,
    ports_inn: None,
    ports_out: None,
    properties: FlowProperties {
        deterministic: FlowPropertyVal::Preserve,
        monotonic: FlowPropertyVal::No,
        inconsistency_tainted: false,
    },
    input_delaytype_fn: |_| None,
    write_fn: |wc @ &WriteContextArgs {
                   context,
                   hydroflow,
                   ident,
                   op_span,
                   root,
                   inputs,
                   outputs,
                   is_pull,
                   op_inst:
                       OperatorInstance {
                           arguments,
                           generics: OpInstGenerics { type_args, .. },
                           ..
                       },
                   ..
               },
               _| {
        let internal_buffer = wc.make_ident("internal_buffer");

        let lattice_type = type_args
            .get(0)
            .map(ToTokens::to_token_stream)
            .unwrap_or(quote_spanned!(op_span=> _));

        let receiver = &arguments[0];
        let stream_ident = wc.make_ident("stream");

        let write_prologue = quote_spanned! {op_span=>
            let mut #stream_ident = ::std::boxed::Box::pin(#receiver);
            let #internal_buffer = #hydroflow.add_state(::std::cell::RefCell::<Option<#lattice_type>>::new(None));
        };

        let (write_iterator, write_iterator_after) = if is_pull {
            let input = &inputs[0];

            (
                quote_spanned! {op_span=>

                    {
                        let mut lattice = #context.state_ref(#internal_buffer).borrow_mut();
                        for i in #input {
                            if let Some(lattice) = &mut *lattice {
                                #root::lattices::Merge::merge(lattice, i);
                            } else {
                                *lattice = Some(#root::lattices::ConvertFrom::from(i));
                            };
                        }
                    }

                    let mut lattice = #context.state_ref(#internal_buffer).borrow_mut();
                    let #ident = match #root::futures::stream::Stream::poll_next(#stream_ident.as_mut(), &mut ::std::task::Context::from_waker(&context.waker())) {
                        ::std::task::Poll::Ready(Some(_)) => {
                            if let Some(lattice) = lattice.take() {
                                Some(lattice).into_iter()
                            } else {
                                None.into_iter()
                            }
                        }
                        ::std::task::Poll::Ready(None) | ::std::task::Poll::Pending => {
                            None.into_iter()
                        }
                    };
                },
                quote_spanned! {op_span=>},
            )
        } else {
            let output = &outputs[0];

            (
                quote_spanned! {op_span=>

                    let mut out = #output;

                    let #ident = #root::pusherator::for_each::ForEach::new(|x| {
                        let mut lattice = #context.state_ref(#internal_buffer).borrow_mut();

                        if let Some(lattice) = &mut *lattice {
                            #root::lattices::Merge::merge(&mut *lattice, x);
                        } else {
                            *lattice = Some(#root::lattices::ConvertFrom::from(x));
                        };
                    });
                },
                quote_spanned! {op_span=>
                    {
                        let mut lattice = #context.state_ref(#internal_buffer).borrow_mut();

                        match #root::futures::stream::Stream::poll_next(#stream_ident.as_mut(), &mut ::std::task::Context::from_waker(&context.waker())) {
                            ::std::task::Poll::Ready(Some(_)) => {
                                if let Some(lattice) = lattice.take() {
                                    #root::pusherator::Pusherator::give(&mut out, lattice);
                                }
                            },
                            ::std::task::Poll::Ready(None) | ::std::task::Poll::Pending => {
                            },
                        }
                    }
                },
            )
        };

        Ok(OperatorWriteOutput {
            write_prologue,
            write_iterator,
            write_iterator_after,
            ..Default::default()
        })
    },
};
