use quote::{ToTokens, quote_spanned};
use syn::parse_quote;
use syn::spanned::Spanned;

use super::join_fused::{JoinOptions, make_joindata, parse_argument, parse_persistences};
use super::{
    DelayType, OpInstGenerics, OperatorCategory, OperatorConstraints, OperatorInstance,
    OperatorWriteOutput, Persistence, PortIndexValue, RANGE_0, RANGE_1, WriteContextArgs,
};
use crate::diagnostic::{Diagnostic, Level};

/// See `join_fused`
///
/// This operator is identical to `join_fused` except that the right hand side input `1` is a regular `join_multiset` input.
///
/// This means that `join_fused_lhs` only takes one argument input, which is the reducing/folding operation for the left hand side only.
///
/// For example:
/// ```dfir
/// source_iter(vec![("key", 0), ("key", 1), ("key", 2)]) -> [0]my_join;
/// source_iter(vec![("key", 2), ("key", 3)]) -> [1]my_join;
/// my_join = join_fused_lhs(Reduce(|x, y| *x += y))
///     -> assert_eq([("key", (3, 2)), ("key", (3, 3))]);
/// ```
pub const JOIN_FUSED_LHS: OperatorConstraints = OperatorConstraints {
    name: "join_fused_lhs",
    categories: &[OperatorCategory::MultiIn],
    hard_range_inn: &(2..=2),
    soft_range_inn: &(2..=2),
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 1,
    persistence_args: &(0..=2),
    type_args: RANGE_0,
    is_external_input: false,
    has_singleton_output: false,
    flo_type: None,
    ports_inn: Some(|| super::PortListSpec::Fixed(parse_quote! { 0, 1 })),
    ports_out: None,
    input_delaytype_fn: |idx| match idx {
        PortIndexValue::Int(path) if "0" == path.to_token_stream().to_string() => {
            Some(DelayType::Stratum)
        }
        _ => None,
    },
    write_fn: |wc @ &WriteContextArgs {
                   context,
                   df_ident,
                   op_span,
                   ident,
                   inputs,
                   is_pull,
                   op_inst:
                       OperatorInstance {
                           generics:
                               OpInstGenerics {
                                   persistence_args, ..
                               },
                           ..
                       },
                   arguments,
                   ..
               },
               diagnostics| {
        assert!(is_pull);

        let arg0_span = arguments[0].span();

        let persistences = parse_persistences(persistence_args);

        let lhs_join_options =
            parse_argument(&arguments[0]).map_err(|err| diagnostics.push(err))?;

        let (lhs_prologue, lhs_pre_write_iter, lhs_borrow) =
            make_joindata(wc, persistences[0], &lhs_join_options, "lhs")
                .map_err(|err| diagnostics.push(err))?;

        let rhs_joindata_ident = wc.make_ident("rhs_joindata");
        let rhs_borrow_ident = wc.make_ident("rhs_joindata_borrow_ident");

        let write_prologue_rhs = match persistences[1] {
            Persistence::None | Persistence::Loop | Persistence::Tick => quote_spanned! {op_span=>},
            Persistence::Static => quote_spanned! {op_span=>
                let #rhs_joindata_ident = #df_ident.add_state(::std::cell::RefCell::new(
                    ::std::vec::Vec::new()
                ));
            },
            Persistence::Mutable => {
                diagnostics.push(Diagnostic::spanned(
                    op_span,
                    Level::Error,
                    "An implementation of 'mutable does not exist",
                ));
                return Err(());
            }
        };

        let write_prologue = quote_spanned! {op_span=>
            #lhs_prologue
            #write_prologue_rhs
        };

        let lhs = &inputs[0];
        let rhs = &inputs[1];

        let lhs_fold_or_reduce_into_from = match lhs_join_options {
            JoinOptions::FoldFrom(lhs_from, lhs_fold) => quote_spanned! {arg0_span=>
                #lhs_borrow.fold_into(#lhs, #lhs_fold, #lhs_from);
            },
            JoinOptions::Fold(lhs_default, lhs_fold) => quote_spanned! {arg0_span=>
                #lhs_borrow.fold_into(#lhs, #lhs_fold, #lhs_default);
            },
            JoinOptions::Reduce(lhs_reduce) => quote_spanned! {arg0_span=>
                #lhs_borrow.reduce_into(#lhs, #lhs_reduce);
            },
        };

        let write_iterator = match persistences[1] {
            Persistence::None | Persistence::Loop | Persistence::Tick => quote_spanned! {op_span=>
                #lhs_pre_write_iter

                let #ident = {
                    #lhs_fold_or_reduce_into_from

                    #[allow(clippy::clone_on_copy)]
                    #rhs.filter_map(|(k, v2)| #lhs_borrow.table.get(&k).map(|v1| (k, (v1.clone(), v2.clone()))))
                };
            },
            Persistence::Static => quote_spanned! {op_span=>
                #lhs_pre_write_iter
                let mut #rhs_borrow_ident = unsafe {
                    // SAFETY: handle from `#df_ident.add_state(..)`.
                    #context.state_ref_unchecked(#rhs_joindata_ident)
                }.borrow_mut();

                let #ident = {
                    #lhs_fold_or_reduce_into_from

                    #[allow(clippy::clone_on_copy)]
                    #[allow(suspicious_double_ref_op)]
                    if #context.is_first_run_this_tick() {
                        #rhs_borrow_ident.extend(#rhs);
                        #rhs_borrow_ident.iter()
                    } else {
                        let len = #rhs_borrow_ident.len();
                        #rhs_borrow_ident.extend(#rhs);
                        #rhs_borrow_ident[len..].iter()
                    }
                    .filter_map(|(k, v2)| #lhs_borrow.table.get(k).map(|v1| (k.clone(), (v1.clone(), v2.clone()))))
                };
            },
            Persistence::Mutable => {
                diagnostics.push(Diagnostic::spanned(
                    op_span,
                    Level::Error,
                    "An implementation of 'mutable does not exist",
                ));
                return Err(());
            }
        };

        let write_iterator_after =
            if persistences[0] == Persistence::Static || persistences[1] == Persistence::Static {
                quote_spanned! {op_span=>
                    // TODO: Probably only need to schedule if #*_borrow.len() > 0?
                    #context.schedule_subgraph(#context.current_subgraph(), false);
                }
            } else {
                quote_spanned! {op_span=>}
            };

        Ok(OperatorWriteOutput {
            write_prologue,
            write_iterator,
            write_iterator_after,
        })
    },
};
