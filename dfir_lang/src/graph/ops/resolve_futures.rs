use quote::quote_spanned;
use syn::Ident;

use super::{
    OperatorCategory, OperatorConstraints, OperatorWriteOutput, RANGE_0, RANGE_1, WriteContextArgs,
};

/// Given an incoming stream of `F: Future`, sends those futures to the executor being used
/// by the DFIR runtime and emits elements whenever a future is completed. The output order
/// is based on when futures complete, and may be different than the input order.
pub const RESOLVE_FUTURES: OperatorConstraints = OperatorConstraints {
    name: "resolve_futures",
    categories: &[OperatorCategory::Map],
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
    input_delaytype_fn: |_| None,
    write_fn: move |wc, _| {
        resolve_futures_writer(Ident::new("FuturesUnordered", wc.op_span), false, wc)
    },
};

pub fn resolve_futures_writer(
    future_type: Ident,
    blocking: bool,
    wc @ &WriteContextArgs {
        root,
        context,
        op_span,
        work_fn_async,
        ident,
        inputs,
        outputs,
        is_pull,
        ..
    }: &WriteContextArgs,
) -> Result<OperatorWriteOutput, ()> {
    let futures_ident = wc.make_ident("futures");
    let queue_ident = wc.make_ident("queue");

    let write_prologue = quote_spanned! {op_span=>
        let #futures_ident = df.add_state(
            ::std::cell::RefCell::new(
                #root::futures::stream::#future_type::new()
            )
        );
    };

    let opt_waker = if blocking {
        quote_spanned! {op_span=> None }
    } else {
        quote_spanned! {op_span=> Some(#context.waker()) }
    };

    let stream_or_sink = if is_pull {
        let input = &inputs[0];

        let task_cx = if blocking {
            quote_spanned! {op_span=> _cx }
        } else {
            quote_spanned! {op_span=> &mut ::std::task::Context::from_waker(&#context.waker()) }
        };

        let if_pending = if blocking {
            // Wait for all items.
            quote_spanned! {op_span=> ::std::task::Poll::Pending }
        } else {
            // EOS immediately, futures items will be in future ticks.
            quote_spanned! {op_span=> ::std::task::Poll::Ready(::std::option::Option::None) }
        };

        quote_spanned! {op_span=>
            {
                let first_item_opt = #work_fn_async(async {
                    // Accumulate all futures.
                    let () = #root::compiled::pull::ForEach::new(#input, |f| {
                        ::std::iter::Extend::extend(&mut *#queue_ident, ::std::iter::once(f));
                    }).await;

                    // Ensure the queue starts, by polling it.
                    // This unfortunately means we also need to store the result of the first poll in `first_item_opt`.
                    #root::futures::future::poll_fn(|_cx| {
                        let opt = if let ::std::task::Poll::Ready(opt) = #root::futures::stream::Stream::poll_next(::std::pin::Pin::new(&mut *#queue_ident), #task_cx) {
                            opt
                        } else {
                            ::std::option::Option::None
                        };
                        ::std::task::Poll::Ready(opt) // Always resolve immediately.
                    }).await
                }).await;

                #root::futures::stream::StreamExt::chain(
                    #root::futures::stream::iter(first_item_opt),
                    #root::futures::stream::poll_fn(|_cx| {
                        match #root::futures::Stream::poll_next(::std::pin::Pin::new(&mut *#queue_ident), #task_cx) {
                            ::std::task::Poll::Ready(opt) => ::std::task::Poll::Ready(opt),
                            ::std::task::Poll::Pending => #if_pending,
                        }
                    }),
                )
            }
        }
    } else {
        let output = &outputs[0];
        quote_spanned! {op_span=>
            #root::compiled::push::ResolveFutures::new(&mut *#queue_ident, #opt_waker, #output)
        }
    };
    let write_iterator = quote_spanned! {op_span=>
        let mut #queue_ident = unsafe {
            // SAFETY: handle from `#df_ident.add_state(..)`.
            #context.state_ref_unchecked(#futures_ident).borrow_mut()
        };
        let #ident = #stream_or_sink;
    };

    Ok(OperatorWriteOutput {
        write_prologue,
        write_iterator,
        ..Default::default()
    })
}
