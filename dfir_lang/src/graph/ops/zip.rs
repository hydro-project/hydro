use quote::quote_spanned;
use syn::parse_quote;

use super::{
    OperatorCategory, OperatorConstraints, OperatorWriteOutput, RANGE_0, RANGE_1, WriteContextArgs,
};

/// > 2 input streams of type `V1` and `V2`, 1 output stream of type `(V1, V2)`
///
/// Zips the streams together, forming paired tuples of the inputs. Note that zipping is done per-tick. If you do not
/// want to discard the excess, use [`zip_longest`](#zip_longest) instead.
///
/// Takes in up to two generic lifetime persistence argument, one for each input. Within the lifetime, excess items
/// from one input or the other will be discarded. Using a `'static` persistence lifetime may result in unbounded
/// buffering if the rates are mismatched.
///
/// ```dfir
/// source_iter(0..3) -> [0]my_zip;
/// source_iter(0..5) -> [1]my_zip;
/// my_zip = zip() -> assert_eq([(0, 0), (1, 1), (2, 2)]);
/// ```
pub const ZIP: OperatorConstraints = OperatorConstraints {
    name: "zip",
    categories: &[OperatorCategory::MultiIn],
    hard_range_inn: &(2..=2),
    soft_range_inn: &(2..=2),
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 0,
    persistence_args: &(0..=2),
    type_args: RANGE_0,
    is_external_input: false,
    has_singleton_output: false,
    flo_type: None,
    ports_inn: Some(|| super::PortListSpec::Fixed(parse_quote! { 0, 1 })),
    ports_out: None,
    input_delaytype_fn: |_| None,
    write_fn: |wc @ &WriteContextArgs {
                   root,
                   context,
                   df_ident,
                   op_span,
                   work_fn_async,
                   ident,
                   is_pull,
                   inputs,
                   ..
               },
               diagnostics| {
        assert!(is_pull);

        let [lhs_persistence, rhs_persistence] = wc.persistence_args_disallow_mutable(diagnostics);

        let lhs_ident = wc.make_ident("lhs");
        let rhs_ident = wc.make_ident("rhs");

        let write_prologue = quote_spanned! {op_span=>
            let #lhs_ident = #df_ident.add_state(::std::cell::RefCell::new(::std::collections::VecDeque::new()));
            let #rhs_ident = #df_ident.add_state(::std::cell::RefCell::new(::std::collections::VecDeque::new()));
        };

        let write_prologue_after_lhs = wc
            .persistence_as_state_lifespan(lhs_persistence)
            .map(|lifespan| {
                quote_spanned! {op_span=>
                    #df_ident.set_state_lifespan_hook(#lhs_ident, #lifespan, |rcell| { rcell.borrow_mut().clear(); });
                }
            });
        let write_prologue_after_rhs = wc
            .persistence_as_state_lifespan(rhs_persistence)
            .map(|lifespan| {
                quote_spanned! {op_span=>
                    #df_ident.set_state_lifespan_hook(#rhs_ident, #lifespan, |rcell| { rcell.borrow_mut().clear(); });
                }
            });

        let lhs_borrow = wc.make_ident("lhs_borrow");
        let rhs_borrow = wc.make_ident("rhs_borrow");
        let lhs_input = &inputs[0];
        let rhs_input = &inputs[1];

        let write_iterator = quote_spanned! {op_span=>
            let (mut #lhs_borrow, mut #rhs_borrow) = unsafe {
                // SAFETY: handle from `#df_ident.add_state(..)`.
                (
                    #context.state_ref_unchecked(#lhs_ident).borrow_mut(),
                    #context.state_ref_unchecked(#rhs_ident).borrow_mut(),
                )
            };

            let #ident = {
                // Consume input eagerly to avoid short-circuiting, update state.
                let () = #work_fn_async(#root::compiled::pull::ForEach::new(#lhs_input, |item| {
                    ::std::collections::VecDeque::push_back(&mut *#lhs_borrow, item);
                })).await;
                let () = #work_fn_async(#root::compiled::pull::ForEach::new(#rhs_input, |item| {
                    ::std::collections::VecDeque::push_back(&mut *#rhs_borrow, item);
                })).await;

                let len = ::std::cmp::min(#lhs_borrow.len(), #rhs_borrow.len());
                let iter = #lhs_borrow.drain(..len).zip(#rhs_borrow.drain(..len));
                #root::futures::stream::iter(iter)
            };
        };

        Ok(OperatorWriteOutput {
            write_prologue,
            write_prologue_after: quote_spanned! {op_span=>
                #write_prologue_after_lhs
                #write_prologue_after_rhs
            },
            write_iterator,
            ..Default::default()
        })
    },
};
