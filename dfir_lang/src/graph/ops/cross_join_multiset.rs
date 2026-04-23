use quote::quote_spanned;
use syn::parse_quote;

use super::{OperatorCategory, OperatorConstraints, RANGE_1, WriteContextArgs};
use crate::graph::ops::{OperatorWriteOutput, Persistence};

/// > 2 input streams of type S and T, 1 output stream of type (S, T)
///
/// Forms the multiset cross-join (Cartesian product) of the (possibly duplicated) items in the input streams, returning all
/// tupled pairs regardless of duplicates.
///
/// ```dfir
/// source_iter(vec!["happy", "happy", "sad"]) -> [0]my_join;
/// source_iter(vec!["dog", "cat", "cat"]) -> [1]my_join;
/// my_join = cross_join_multiset() -> sort() -> assert_eq([
///     ("happy", "cat"),
///     ("happy", "cat"),
///     ("happy", "cat"),
///     ("happy", "cat"),
///     ("happy", "dog"),
///     ("happy", "dog"),
///     ("sad", "cat"),
///     ("sad", "cat"),
///     ("sad", "dog"), ]);
/// ```
pub const CROSS_JOIN_MULTISET: OperatorConstraints = OperatorConstraints {
    name: "cross_join_multiset",
    categories: &[OperatorCategory::MultiIn],
    hard_range_inn: &(2..=2),
    soft_range_inn: &(2..=2),
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 0,
    persistence_args: &(0..=2),
    type_args: &(0..=1),
    is_external_input: false,
    has_singleton_output: false,
    flo_type: None,
    ports_inn: Some(|| super::PortListSpec::Fixed(parse_quote! { 0, 1 })),
    ports_out: None,
    input_delaytype_fn: |_| None,
    write_fn: |wc @ &WriteContextArgs {
                   root,
                   context,
                   op_span,
                   work_fn_async,
                   ident,
                   inputs,
                   ..
               },
               diagnostics| {
        let lhs = &inputs[0];
        let rhs = &inputs[1];

        let [lhs_persistence, rhs_persistence] = wc.persistence_args_disallow_mutable(diagnostics);

        let lhs_state = wc.make_ident("lhs_state");
        let rhs_state = wc.make_ident("rhs_state");
        let write_prologue = quote_spanned! {op_span=>
            let mut #lhs_state: ::std::vec::Vec<_> = ::std::vec::Vec::new();
            let mut #rhs_state: ::std::vec::Vec<_> = ::std::vec::Vec::new();
        };

        let lhs_write_tick_end = match lhs_persistence {
            Persistence::None | Persistence::Tick => quote_spanned! {op_span=>
                #lhs_state.clear();
            },
            _ => Default::default(),
        };
        let rhs_write_tick_end = match rhs_persistence {
            Persistence::None | Persistence::Tick => quote_spanned! {op_span=>
                #rhs_state.clear();
            },
            _ => Default::default(),
        };

        let lhs_i = wc.make_ident("lhs_i");
        let rhs_i = wc.make_ident("rhs_i");
        let write_iterator = quote_spanned! {op_span=>
            let (#lhs_i, #rhs_i) = if #context.is_first_run_this_tick() {
                (0, 0)
            } else {
                (#lhs_state.len(), #rhs_state.len())
            };

            #work_fn_async(#root::dfir_pipes::pull::Pull::for_each(#lhs, |x| #lhs_state.push(x))).await;
            #work_fn_async(#root::dfir_pipes::pull::Pull::for_each(#rhs, |x| #rhs_state.push(x))).await;

            //       RHS
            //   +-----+-----+
            // L | Old | New |
            // H +-----+-----+
            // S | New | New |
            //   +-----+-----+
            let #ident = #root::dfir_pipes::pull::iter(
                #lhs_state
                    .iter()
                    .enumerate()
                    .flat_map(|(i, lhs)| {
                        let j = if i < #lhs_i { #rhs_i } else { 0 };
                        #rhs_state[j..]
                            .iter()
                            .map(move |rhs| (::std::clone::Clone::clone(lhs), ::std::clone::Clone::clone(rhs)))
                    })
            );
        };

        let replay_code = matches!(
            (lhs_persistence, rhs_persistence),
            (Persistence::Static, Persistence::Static)
        )
        .then(|| {
            quote_spanned! {op_span=>
                // Reschedule the subgraph lazily to ensure replay on later ticks.
                #context.schedule_subgraph(#context.current_subgraph(), false);
            }
        });
        let write_iterator_after = quote_spanned! {op_span=>
            #replay_code
        };

        // Ok(output)
        Ok(OperatorWriteOutput {
            write_prologue,
            write_iterator,
            write_iterator_after,
            write_tick_end: quote_spanned! {op_span=>
                #lhs_write_tick_end
                #rhs_write_tick_end
            },
            ..Default::default()
        })
    },
};
