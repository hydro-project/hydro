use quote::quote_spanned;
use syn::Ident;

use super::{
    OperatorCategory, OperatorConstraints, OperatorWriteOutput, RANGE_0, RANGE_1, WriteContextArgs,
};

/// Given an incoming stream of `F: Future`, resolves each future, blocking the subgraph execution.
/// Until the results are resolved. The output order is based on when futures complete, and may be
/// different than the input order.
pub const RESOLVE_FUTURES_BLOCKING: OperatorConstraints = OperatorConstraints {
    name: "resolve_futures_blocking",
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
        resolve_futures_writer(
            Ident::new("FuturesUnordered", wc.op_span),
            wc,
        )
    },
};

pub fn resolve_futures_writer(
    future_type: Ident,
    wc @ &WriteContextArgs {
        root,
        context,
        op_span,
        ident,
        outputs,
        is_pull,
        ..
    }: &WriteContextArgs,
) -> Result<OperatorWriteOutput, ()> {
    let futures_ident = wc.make_ident("futures");

    let write_prologue = quote_spanned! {op_span=>
        let #futures_ident = df.add_state(
            ::std::cell::RefCell::new(
                #root::futures::stream::#future_type::new()
            )
        );
    };

    let write_iterator = if is_pull {
        panic!("ResolveFuturesBlocking cannot be used in pull mode");
    } else {
        let output = &outputs[0];
        quote_spanned! {op_span=>
            let #ident = {
                let queue = unsafe {
                    // SAFETY: handle from `#df_ident.add_state(..)`.
                    #context.state_ref_unchecked(#futures_ident).borrow_mut()
                };
                #root::compiled::push::ResolveFutures::new(queue, None, #output)
            };
        }
    };

    Ok(OperatorWriteOutput {
        write_prologue,
        write_iterator,
        ..Default::default()
    })
}
