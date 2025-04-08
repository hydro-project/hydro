use quote::{ToTokens, quote_spanned};
use syn::parse_quote;

use super::{
    OpInstGenerics, OperatorCategory, OperatorConstraints, OperatorInstance, OperatorWriteOutput,
    Persistence, RANGE_1, WriteContextArgs,
};

/// > 2 input streams of type `<(K, V1)>` and `<(K, V2)>`, 1 output stream of type `<(K, (V1, V2))>`
///
/// Forms the equijoin of the tuples in the input streams by their first (key) attribute. Note that the result nests the 2nd input field (values) into a tuple in the 2nd output field.
///
/// ```dfir
/// source_iter(vec![("hello", "world"), ("stay", "gold"), ("hello", "world")]) -> [0]my_join;
/// source_iter(vec![("hello", "cleveland")]) -> [1]my_join;
/// my_join = join()
///     -> assert_eq([("hello", ("world", "cleveland"))]);
/// ```
///
/// `join` can also be provided with one or two generic lifetime persistence arguments, either
/// `'tick` or `'static`, to specify how join data persists. With `'tick`, pairs will only be
/// joined with corresponding pairs within the same tick. With `'static`, pairs will be remembered
/// across ticks and will be joined with pairs arriving in later ticks. When not explicitly
/// specified persistence defaults to `tick.
///
/// When two persistence arguments are supplied the first maps to port `0` and the second maps to
/// port `1`.
/// When a single persistence argument is supplied, it is applied to both input ports.
/// When no persistence arguments are applied it defaults to `'tick` for both.
///
/// The syntax is as follows:
/// ```dfir,ignore
/// join(); // Or
/// join::<'static>();
///
/// join::<'tick>();
///
/// join::<'static, 'tick>();
///
/// join::<'tick, 'static>();
/// // etc.
/// ```
///
/// `join` is defined to treat its inputs as *sets*, meaning that it
/// eliminates duplicated values in its inputs. If you do not want
/// duplicates eliminated, use the [`join_multiset`](#join_multiset) operator.
///
/// ### Examples
///
/// ```rustbook
/// let (input_send, input_recv) = dfir_rs::util::unbounded_channel::<(&str, &str)>();
/// let mut flow = dfir_rs::dfir_syntax! {
///     source_iter([("hello", "world")]) -> [0]my_join;
///     source_stream(input_recv) -> [1]my_join;
///     my_join = join::<'tick>() -> for_each(|(k, (v1, v2))| println!("({}, ({}, {}))", k, v1, v2));
/// };
/// input_send.send(("hello", "oakland")).unwrap();
/// flow.run_tick();
/// input_send.send(("hello", "san francisco")).unwrap();
/// flow.run_tick();
/// ```
/// Prints out `"(hello, (world, oakland))"` since `source_iter([("hello", "world")])` is only
/// included in the first tick, then forgotten.
///
/// ---
///
/// ```rustbook
/// let (input_send, input_recv) = dfir_rs::util::unbounded_channel::<(&str, &str)>();
/// let mut flow = dfir_rs::dfir_syntax! {
///     source_iter([("hello", "world")]) -> [0]my_join;
///     source_stream(input_recv) -> [1]my_join;
///     my_join = join::<'static>() -> for_each(|(k, (v1, v2))| println!("({}, ({}, {}))", k, v1, v2));
/// };
/// input_send.send(("hello", "oakland")).unwrap();
/// flow.run_tick();
/// input_send.send(("hello", "san francisco")).unwrap();
/// flow.run_tick();
/// ```
/// Prints out `"(hello, (world, oakland))"` and `"(hello, (world, san francisco))"` since the
/// inputs are peristed across ticks.
pub const JOIN: OperatorConstraints = OperatorConstraints {
    name: "join",
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
                   df_ident,
                   loop_id,
                   op_span,
                   ident,
                   inputs,
                   work_fn,
                   op_inst:
                       OperatorInstance {
                           generics:
                               OpInstGenerics {
                                   persistence_args,
                                   type_args,
                                   ..
                               },
                           ..
                       },
                   ..
               },
               _diagnostics| {
        let join_type =
            type_args
                .first()
                .map(ToTokens::to_token_stream)
                .unwrap_or(quote_spanned!(op_span=>
                    #root::compiled::pull::HalfSetJoinState
                ));

        // TODO: This is really bad.
        // This will break if the user aliases HalfSetJoinState to something else. Temporary hacky solution.
        // Note that cross_join() depends on the implementation here as well.
        let additional_trait_bounds = if join_type.to_string().contains("HalfSetJoinState") {
            quote_spanned!(op_span=>
                + ::std::cmp::Eq
            )
        } else {
            quote_spanned!(op_span=>)
        };

        let make_joindata = |persistence, side| {
            let joindata_ident = wc.make_ident(format!("joindata_{}", side));
            let borrow_ident = wc.make_ident(format!("joindata_{}_borrow", side));

            let lifespan = wc.persistence_as_state_lifespan(persistence);
            let reset = lifespan.map(|lifespan| quote_spanned! {op_span=>
                #df_ident.set_state_lifespan_hook(#joindata_ident, #lifespan, |rcell| (#work_fn)(|| #root::util::clear::Clear::clear(::std::cell::RefCell::get_mut(rcell))));
            }).unwrap_or_default();

            let prologue = quote_spanned! {op_span=>
                let #joindata_ident = #df_ident.add_state(::std::cell::RefCell::new(
                    #join_type::default()
                ));
            };
            let borrow = quote_spanned! {op_span=>
                unsafe {
                    // SAFETY: handle from `#df_ident.add_state(..)`.
                    #context.state_ref_unchecked(#joindata_ident)
                }.borrow_mut()
            };

            Ok((prologue, reset, borrow, borrow_ident))
        };

        let persistences = match persistence_args[..] {
            [] => {
                let p = if loop_id.is_some() {
                    Persistence::None
                } else {
                    Persistence::Tick
                };
                [p, p]
            }
            [a] => [a, a],
            [a, b] => [a, b],
            _ => panic!(),
        };

        let (lhs_prologue, lhs_prologue_after, lhs_borrow, lhs_borrow_ident) =
            (make_joindata)(persistences[0], "lhs")?;
        let (rhs_prologue, rhs_prologue_after, rhs_borrow, rhs_borrow_ident) =
            (make_joindata)(persistences[1], "rhs")?;

        let lhs = &inputs[0];
        let rhs = &inputs[1];
        let write_iterator = quote_spanned! {op_span=>
            let mut #lhs_borrow_ident = #lhs_borrow;
            let mut #rhs_borrow_ident = #rhs_borrow;
            let #ident = {
                // Limit error propagation by bounding locally, erasing output iterator type.
                #[inline(always)]
                fn check_inputs<'a, K, I1, V1, I2, V2>(
                    lhs: I1,
                    rhs: I2,
                    lhs_state: &'a mut #join_type<K, V1, V2>,
                    rhs_state: &'a mut #join_type<K, V2, V1>,
                    is_new_tick: bool,
                ) -> impl 'a + Iterator<Item = (K, (V1, V2))>
                where
                    K: Eq + std::hash::Hash + Clone,
                    V1: Clone #additional_trait_bounds,
                    V2: Clone #additional_trait_bounds,
                    I1: 'a + Iterator<Item = (K, V1)>,
                    I2: 'a + Iterator<Item = (K, V2)>,
                {
                    #work_fn(|| #root::compiled::pull::symmetric_hash_join_into_iter(lhs, rhs, lhs_state, rhs_state, is_new_tick))
                }

                check_inputs(#lhs, #rhs, &mut *#lhs_borrow_ident, &mut *#rhs_borrow_ident, #context.is_first_run_this_tick())
            };
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
            write_prologue_after: quote_spanned! {op_span=>
                #lhs_prologue_after
                #rhs_prologue_after
            },
            write_iterator,
            write_iterator_after,
        })
    },
};
