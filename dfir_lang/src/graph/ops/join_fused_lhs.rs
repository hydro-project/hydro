use quote::{ToTokens, quote_spanned};
use syn::parse_quote;

use super::join_fused::make_joindata;
use super::{
    DelayType, OperatorCategory, OperatorConstraints, OperatorWriteOutput, Persistence,
    PortIndexValue, RANGE_0, RANGE_1, WriteContextArgs,
};

/// See `join_fused`
///
/// This operator is identical to `join_fused` except that the right hand side input `1` is a regular `join_multiset` input.
///
/// This means that `join_fused_lhs` only takes one argument input, which is the reducing/folding operation for the left hand side only.
///
/// For example:
/// ```dfir
/// use dfir_rs::util::accumulator::Reduce;
///
/// source_iter(vec![("key", 0), ("key", 1), ("key", 2)]) -> [0]my_join;
/// source_iter(vec![("key", 2), ("key", 3)]) -> [1]my_join;
/// my_join = join_fused_lhs(Reduce::new(|x, y| *x += y))
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
                   root,
                   context,
                   df_ident,
                   op_span,
                   ident,
                   inputs,
                   is_pull,
                   arguments,
                   ..
               },
               diagnostics| {
        assert!(is_pull);

        let [persistence_lhs, persistence_rhs] = wc.persistence_args_disallow_mutable(diagnostics);

        let (lhs_prologue, lhs_prologue_after, lhs_pre_write_iter, lhs_borrow) =
            make_joindata(wc, persistence_lhs, "lhs").map_err(|err| diagnostics.push(err))?;

        let rhs_joindata_ident = wc.make_ident("rhs_joindata");
        let rhs_borrow_ident = wc.make_ident("rhs_joindata_borrow_ident");

        let rhs_prologue = match persistence_rhs {
            Persistence::None | Persistence::Loop | Persistence::Tick => quote_spanned! {op_span=>},
            Persistence::Static => quote_spanned! {op_span=>
                let #rhs_joindata_ident = #df_ident.add_state(::std::cell::RefCell::new(
                    ::std::vec::Vec::new()
                ));
            },
            Persistence::Mutable => unreachable!(),
        };

        let lhs = &inputs[0];
        let rhs = &inputs[1];

        let lhs_accum = &arguments[0];

        let write_iterator = match persistence_rhs {
            Persistence::None | Persistence::Loop | Persistence::Tick => quote_spanned! {op_span=>
                #lhs_pre_write_iter

                let #rhs_borrow_ident = &mut ::std::vec::Vec::new();

                let #ident = #root::compiled::pull::JoinFusedLhs::new(
                    #root::futures::stream::StreamExt::fuse(#lhs),
                    #rhs,
                    #lhs_accum,
                    &mut *#lhs_borrow,
                    &mut *#rhs_borrow_ident,
                    0,
                );
            },
            Persistence::Static => quote_spanned! {op_span=>
                #lhs_pre_write_iter
                let mut #rhs_borrow_ident = unsafe {
                    // SAFETY: handle from `#df_ident.add_state(..)`.
                    #context.state_ref_unchecked(#rhs_joindata_ident)
                }.borrow_mut();

                let #ident = {
                    let rhs_replay_idx = if #context.is_first_run_this_tick() {
                        0
                    } else {
                        #rhs_borrow_ident.len()
                    };
                    #root::compiled::pull::JoinFusedLhs::new(
                        #root::futures::stream::StreamExt::fuse(#lhs),
                        #rhs,
                        #lhs_accum,
                        &mut *#lhs_borrow,
                        &mut *#rhs_borrow_ident,
                        rhs_replay_idx,
                    )
                };
            },
            Persistence::Mutable => unreachable!(),
        };

        let write_iterator_after =
            if persistence_lhs == Persistence::Static || persistence_rhs == Persistence::Static {
                quote_spanned! {op_span=>
                    // TODO: Probably only need to schedule if #*_borrow.len() > 0?
                    #context.schedule_subgraph(#context.current_subgraph(), false);
                }
            } else {
                quote_spanned! {op_span=>}
            };

        Ok(OperatorWriteOutput {
            write_prologue: quote_spanned! {op_span=>
                #lhs_prologue
                #rhs_prologue
            },
            write_prologue_after: lhs_prologue_after,
            write_iterator,
            write_iterator_after,
        })
    },
};
