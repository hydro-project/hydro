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
/// use dfir_rs::compiled::pull::Reduce;
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
                   work_fn_async,
                   ident,
                   inputs,
                   is_pull,
                   arguments,
                   ..
               },
               diagnostics| {
        assert!(is_pull);

        let persistences: [_; 2] = wc.persistence_args_disallow_mutable(diagnostics);

        let (lhs_prologue, lhs_prologue_after, lhs_pre_write_iter, lhs_borrow) =
            make_joindata(wc, persistences[0], "lhs").map_err(|err| diagnostics.push(err))?;

        let rhs_joindata_ident = wc.make_ident("rhs_joindata");
        let rhs_borrow_ident = wc.make_ident("rhs_joindata_borrow_ident");

        let rhs_prologue = match persistences[1] {
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

        let write_iterator = match persistences[1] {
            Persistence::None | Persistence::Loop | Persistence::Tick => quote_spanned! {op_span=>
                #lhs_pre_write_iter

                let #ident = {
                    let () = #work_fn_async(
                        #root::compiled::pull::accumulate_all(&mut #lhs_accum, &mut *#lhs_borrow, #lhs),
                    ).await;

                    #[allow(clippy::clone_on_copy)]
                    #root::tokio_stream::StreamExt::filter_map(#rhs, |(k, v2)| #lhs_borrow.get(&k).map(|v1| (k, (v1.clone(), v2.clone()))))
                };
            },
            Persistence::Static => quote_spanned! {op_span=>
                #lhs_pre_write_iter
                let mut #rhs_borrow_ident = unsafe {
                    // SAFETY: handle from `#df_ident.add_state(..)`.
                    #context.state_ref_unchecked(#rhs_joindata_ident)
                }.borrow_mut();

                let #ident = {
                    // Accumulate LHS.
                    let () = #work_fn_async(
                        #root::compiled::pull::accumulate_all(&mut #lhs_accum, &mut *#lhs_borrow, #lhs),
                    ).await;

                    // RHS replay index.
                    let replay_idx = if #context.is_first_run_this_tick() {
                        0
                    } else {
                        #rhs_borrow_ident.len()
                    };

                    // Accumulate RHS.
                    let () = #work_fn_async(
                        #root::compiled::pull::ForEach::new(#rhs, |kv| {
                            #rhs_borrow_ident.push(kv);
                        }),
                    ).await;


                    #[allow(clippy::clone_on_copy)]
                    #[allow(suspicious_double_ref_op)]
                    let iter = #rhs_borrow_ident[replay_idx..]
                        .iter()
                        .filter_map(|(k, v2)| #lhs_borrow.get(k).map(|v1| (k.clone(), (v1.clone(), v2.clone()))));
                    #root::futures::stream::iter(iter)
                };
            },
            Persistence::Mutable => unreachable!(),
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
