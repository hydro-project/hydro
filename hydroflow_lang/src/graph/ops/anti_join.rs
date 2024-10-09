use quote::{quote_spanned, ToTokens};
use syn::parse_quote;

use super::{
    DelayType, OpInstGenerics, OperatorCategory, OperatorConstraints,
    OperatorInstance, OperatorWriteOutput, Persistence, PortIndexValue, WriteContextArgs, RANGE_0,
    RANGE_1,
};
use crate::diagnostic::{Diagnostic, Level};

/// > 2 input streams the first of type (K, T), the second of type K,
/// > with output type (K, T)
///
/// For a given tick, computes the anti-join of the items in the input
/// streams, returning unique items in the `pos` input that do not have matching keys
/// in the `neg` input. Note this is set semantics only for the `neg element`. Order
/// is preserved for new elements in a given tick, but not for elements processed
/// in a previous tick with `'static`.
///
/// ```hydroflow
/// source_iter(vec![("dog", 1), ("cat", 2), ("elephant", 3)]) -> [pos]diff;
/// source_iter(vec!["dog", "cat", "gorilla"]) -> [neg]diff;
/// diff = anti_join() -> assert_eq([("elephant", 3)]);
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
    // If this is set to true, the state will need to be cleared using `#context.set_state_tick_hook`
    // to prevent reading uncleared data if this subgraph doesn't run.
    // https://github.com/hydro-project/hydroflow/issues/1298
    has_singleton_output: false,
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
                   hydroflow,
                   op_span,
                   ident,
                   inputs,
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
        let persistences = match persistence_args[..] {
            [] => [Persistence::Tick, Persistence::Tick],
            [a] => [a, a],
            [a, b] => [a, b],
            _ => unreachable!(),
        };

        let mut make_antijoindata = |persistence, side| {
            let antijoindata_ident = wc.make_ident(format!("antijoindata_{}", side));
            let borrow_ident = wc.make_ident(format!("antijoindata_{}_borrow", side));
            let (init, borrow) = match persistence {
                Persistence::Tick => (
                    quote_spanned! {op_span=>
                        #root::util::monotonic_map::MonotonicMap::<_, #root::rustc_hash::FxHashSet<_>>::default()
                    },
                    quote_spanned! {op_span=>
                        &mut *#borrow_ident.get_mut_clear(#context.current_tick())
                    },
                ),
                Persistence::Static => (
                    quote_spanned! {op_span=>
                        #root::rustc_hash::FxHashSet::default()
                    },
                    quote_spanned! {op_span=>
                        &mut *#borrow_ident
                    },
                ),
                Persistence::Mutable => {
                    diagnostics.push(Diagnostic::spanned(
                        op_span,
                        Level::Error,
                        "An implementation of 'mutable does not exist",
                    ));
                    return Err(());
                }
            };
            Ok((antijoindata_ident, borrow_ident, init, borrow))
        };

        let (pos_antijoindata_ident, pos_borrow_ident, pos_init, pos_borrow) =
            make_antijoindata(persistences[0], "pos")?;
        let (neg_antijoindata_ident, neg_borrow_ident, neg_init, neg_borrow) =
            make_antijoindata(persistences[1], "neg")?;

        let write_prologue = quote_spanned! {op_span=>
            let #neg_antijoindata_ident = #hydroflow.add_state(std::cell::RefCell::new(
                #neg_init
            ));
            let #pos_antijoindata_ident = #hydroflow.add_state(std::cell::RefCell::new(
                #pos_init
            ));
        };

        let input_neg = &inputs[0]; // N before P
        let input_pos = &inputs[1];
        let write_iterator = {
            quote_spanned! {op_span=>
                let mut #neg_borrow_ident = #context.state_ref(#neg_antijoindata_ident).borrow_mut();
                let mut #pos_borrow_ident = #context.state_ref(#pos_antijoindata_ident).borrow_mut();
                let #ident = {
                    /// Limit error propagation by bounding locally, erasing output iterator type.
                    #[inline(always)]
                    fn check_inputs<'a, K, I1, V, I2>(
                        input_neg: I1,
                        input_pos: I2,
                        neg_state: &'a mut #root::rustc_hash::FxHashSet<K>,
                        pos_state: &'a mut #root::rustc_hash::FxHashSet<(K, V)>,
                        is_new_tick: bool,
                    ) -> impl 'a + Iterator<Item = (K, V)>
                    where
                        K: Eq + ::std::hash::Hash + Clone,
                        V: Eq + ::std::hash::Hash + Clone,
                        I1: 'a + Iterator<Item = K>,
                        I2: 'a + Iterator<Item = (K, V)>,
                    {
                        neg_state.extend(input_neg);

                        #root::compiled::pull::anti_join_into_iter(input_pos, neg_state, pos_state, is_new_tick)
                    }

                    check_inputs(
                        #input_neg,
                        #input_pos,
                        #neg_borrow,
                        #pos_borrow,
                        context.is_first_run_this_tick(),
                    )
                };
            }
        };

        Ok(OperatorWriteOutput {
            write_prologue,
            write_iterator,
            ..Default::default()
        })
    },
};
