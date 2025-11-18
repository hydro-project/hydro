use quote::{ToTokens, quote_spanned};
use syn::parse_quote;

use super::{
    DelayType, OperatorCategory, OperatorConstraints, OperatorWriteOutput, PortIndexValue, RANGE_0,
    RANGE_1, WriteContextArgs,
};
use crate::graph::ops::Persistence;

// This implementation is largely redundant to ANTI_JOIN and should be DRY'ed
/// > 2 input streams the first of type (K, T), the second of type K,
/// > with output type (K, T)
///
/// For a given tick, computes the anti-join of the items in the input
/// streams, returning items in the `pos` input that do not have matching keys
/// in the `neg` input. NOTE this uses multiset semantics only on the positive side,
/// so duplicated positive inputs will appear in the output either 0 times (if matched in `neg`)
/// or as many times as they appear in the input (if not matched in `neg`)
///
/// ```dfir
/// source_iter(vec![("cat", 2), ("cat", 2), ("elephant", 3), ("elephant", 3)]) -> [pos]diff;
/// source_iter(vec!["dog", "cat", "gorilla"]) -> [neg]diff;
/// diff = anti_join() -> assert_eq([("elephant", 3), ("elephant", 3)]);
/// ```
pub const ANTI_JOIN: OperatorConstraints = OperatorConstraints {
    name: "anti_join",
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
    ports_inn: Some(|| super::PortListSpec::Fixed(parse_quote! { pos, neg })),
    ports_out: None,
    input_delaytype_fn: |idx| match idx {
        PortIndexValue::Path(path) if "neg" == path.to_token_stream().to_string() => {
            Some(DelayType::Stratum)
        }
        _else => None,
    },
    write_fn: |wc @ &WriteContextArgs {
                   root,
                   context,
                   df_ident,
                   op_span,
                   ident,
                   is_pull,
                   inputs,
                   ..
               },
               diagnostics| {
        assert!(is_pull);

        let persistences: [_; 2] = wc.persistence_args_disallow_mutable(diagnostics);

        let pos_antijoindata_ident = wc.make_ident("antijoindata_pos");
        let neg_antijoindata_ident = wc.make_ident("antijoindata_neg");

        let pos_persist = match persistences[0] {
            Persistence::None | Persistence::Tick => false,
            Persistence::Loop | Persistence::Static => true,
            Persistence::Mutable => unreachable!(),
        };

        let write_prologue_pos = pos_persist.then(|| {
            quote_spanned! {op_span=>
                let #pos_antijoindata_ident = #df_ident.add_state(std::cell::RefCell::new(
                    ::std::vec::Vec::new()
                ));
            }
        });
        let write_prologue_after_pos = pos_persist.then(|| wc
            .persistence_as_state_lifespan(persistences[0])
            .map(|lifespan| quote_spanned! {op_span=>
                #[allow(clippy::redundant_closure_call)]
                #df_ident.set_state_lifespan_hook(
                    #pos_antijoindata_ident, #lifespan, move |rcell| { rcell.borrow_mut().clear(); },
                );
            })).flatten();

        let write_prologue_neg = quote_spanned! {op_span=>
            let #neg_antijoindata_ident = #df_ident.add_state(std::cell::RefCell::new(
                #root::rustc_hash::FxHashSet::default()
            ));
        };
        let write_prologue_after_neg = wc
            .persistence_as_state_lifespan(persistences[1])
            .map(|lifespan| quote_spanned! {op_span=>
                #[allow(clippy::redundant_closure_call)]
                #df_ident.set_state_lifespan_hook(
                    #neg_antijoindata_ident, #lifespan, move |rcell| { rcell.borrow_mut().clear(); },
                );
            }).unwrap_or_default();

        let input_neg = &inputs[0]; // N before P
        let input_pos = &inputs[1];
        let write_iterator = if !pos_persist {
            quote_spanned! {op_span=>
                let mut neg_borrow = unsafe {
                    // SAFETY: handle from `#df_ident`.
                    #context.state_ref_unchecked(#neg_antijoindata_ident)
                }.borrow_mut();

                let #ident = #root::compiled::pull::AntiJoin::new(
                    #input_pos,
                    #root::futures::stream::StreamExt::fuse(#input_neg),
                    &mut *neg_borrow);
            }
        } else {
            quote_spanned! {op_span =>
                let (mut neg_borrow, mut pos_borrow) = unsafe {
                    // SAFETY: handles from `#df_ident`.
                    (
                        #context.state_ref_unchecked(#neg_antijoindata_ident).borrow_mut(),
                        #context.state_ref_unchecked(#pos_antijoindata_ident).borrow_mut(),
                    )
                };


                let #ident = {
                    let replay_idx = if #context.is_first_run_this_tick() {
                        0
                    } else {
                        pos_borrow.len()
                    };
                    #root::compiled::pull::AntiJoinPersist::new(
                        #input_pos,
                        #root::futures::stream::StreamExt::fuse(#input_neg),
                        &mut *pos_borrow,
                        &mut *neg_borrow,
                        replay_idx
                    )
                };
            }
        };

        Ok(OperatorWriteOutput {
            write_prologue: quote_spanned! {op_span=>
                #write_prologue_pos
                #write_prologue_neg
            },
            write_prologue_after: quote_spanned! {op_span=>
                #write_prologue_after_pos
                #write_prologue_after_neg
            },
            write_iterator,
            ..Default::default()
        })
    },
};
