use quote::quote_spanned;
use syn::parse_quote;

use super::{
    OpInstGenerics, OperatorCategory, OperatorConstraints, OperatorInstance, OperatorWriteOutput,
    Persistence, WriteContextArgs, RANGE_0, RANGE_1,
};
use crate::diagnostic::{Diagnostic, Level};

/// > 2 input streams of type `V1` and `V2`, 1 output stream of type `(V1, V2)`
///
/// Zips the streams together, forming paired tuples of the inputs. Note that zipping is done
/// per-tick. Excess items from one input or the other will be discarded. If you do not want to
/// discard the excess, use [`zip_longest`](#zip_longest) instead.
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
    persistence_args: &(0..=1),
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
                   ident,
                   is_pull,
                   inputs,
                   op_name,
                   op_inst:
                       OperatorInstance {
                           generics:
                               OpInstGenerics {
                                   persistence_args, ..
                               },
                           ..
                       },
                   ..
               },
               diagnostics| {
        assert!(is_pull);

        let persistence = match persistence_args[..] {
            [] => Persistence::Tick,
            [a] => a,
            _ => unreachable!(),
        };
        if Persistence::Tick != persistence {
            diagnostics.push(Diagnostic::spanned(
                op_span,
                Level::Error,
                format!("`{}()` can only have `'tick` persistence.", op_name),
            ));
            // Fall-thru to still generate code.
        }

        let zipbuf_ident = wc.make_ident("zipbuf");

        let write_prologue = quote_spanned! {op_span=>
            let #zipbuf_ident = #df_ident.add_state(::std::cell::RefCell::new(
                #root::util::monotonic_map::MonotonicMap::<
                    #root::scheduled::ticks::TickInstant,
                    (::std::vec::Vec<_>, ::std::vec::Vec<_>),
                >::default()
            ));
        };

        let lhs = &inputs[0];
        let rhs = &inputs[1];
        let write_iterator = quote_spanned! {op_span=>
            let #ident = {
                // TODO(mingwei): performance issue - get_mut_default and std::mem::take reset the vecs, reallocs heap.
                let mut zipbuf_borrow = unsafe {
                    // SAFETY: handle from `#df_ident.add_state(..)`.
                    #context.state_ref_unchecked(#zipbuf_ident)
                }.borrow_mut();
                let (lhs_buf, rhs_buf) = zipbuf_borrow.get_mut_default(#context.current_tick());
                #root::itertools::Itertools::zip_longest(
                    ::std::mem::take(lhs_buf).into_iter().chain(#lhs),
                    ::std::mem::take(rhs_buf).into_iter().chain(#rhs),
                )
                    .filter_map(|either| {
                        if let #root::itertools::EitherOrBoth::Both(lhs, rhs) = either {
                            Some((lhs, rhs))
                        } else {
                            let mut zipbuf_burrow = unsafe {
                                // SAFETY: handle from `#df_ident.add_state(..)`.
                                #context.state_ref_unchecked(#zipbuf_ident)
                            }.borrow_mut();
                            let (lhs_buf, rhs_buf) = zipbuf_burrow.get_mut_default(#context.current_tick());
                            match either {
                                #root::itertools::EitherOrBoth::Left(lhs) => lhs_buf.push(lhs),
                                #root::itertools::EitherOrBoth::Right(rhs) => rhs_buf.push(rhs),
                                _ => ::std::unreachable!(),
                            }
                            None
                        }
                    })
            };
        };

        Ok(OperatorWriteOutput {
            write_prologue,
            write_iterator,
            ..Default::default()
        })
    },
};
