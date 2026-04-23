use quote::quote_spanned;
use syn::parse_quote;

use super::{
    OperatorCategory, OperatorConstraints, OperatorWriteOutput, Persistence, RANGE_0, RANGE_1,
    WriteContextArgs,
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
            let mut #lhs_ident: ::std::collections::VecDeque<_> = ::std::collections::VecDeque::new();
            let mut #rhs_ident: ::std::collections::VecDeque<_> = ::std::collections::VecDeque::new();
        };

        let write_tick_end_lhs = match lhs_persistence {
            Persistence::None | Persistence::Tick => Some(quote_spanned! {op_span=>
                #lhs_ident.clear();
            }),
            _ => None,
        };
        let write_tick_end_rhs = match rhs_persistence {
            Persistence::None | Persistence::Tick => Some(quote_spanned! {op_span=>
                #rhs_ident.clear();
            }),
            _ => None,
        };

        let lhs_input = &inputs[0];
        let rhs_input = &inputs[1];

        let write_iterator = quote_spanned! {op_span=>
            let #ident = {
                // Consume input eagerly to avoid short-circuiting, update state.
                let () = #work_fn_async(#root::dfir_pipes::pull::Pull::for_each(#lhs_input, |item| {
                    ::std::collections::VecDeque::push_back(&mut #lhs_ident, item);
                })).await;
                let () = #work_fn_async(#root::dfir_pipes::pull::Pull::for_each(#rhs_input, |item| {
                    ::std::collections::VecDeque::push_back(&mut #rhs_ident, item);
                })).await;

                let len = ::std::cmp::min(#lhs_ident.len(), #rhs_ident.len());
                let iter = #lhs_ident.drain(..len).zip(#rhs_ident.drain(..len));
                #root::dfir_pipes::pull::iter(iter)
            };
        };

        Ok(OperatorWriteOutput {
            write_prologue,
            write_iterator,
            write_tick_end: quote_spanned! {op_span=>
                #write_tick_end_lhs
                #write_tick_end_rhs
            },
            ..Default::default()
        })
    },
};
