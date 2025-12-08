use proc_macro2::TokenStream;
use quote::quote_spanned;
use syn::{Ident, parse_quote};

use super::{
    DelayType, OperatorCategory, OperatorConstraints, OperatorWriteOutput, Persistence, RANGE_0,
    RANGE_1, WriteContextArgs,
};
use crate::diagnostic::Diagnostic;

/// > 2 input streams of type `<(K, V1)>` and `<(K, V2)>`, 1 output stream of type `<(K, (V1, V2))>`
///
/// `join_fused` takes two arguments, they are the aggregators for the left hand side and right hand side inputs respectively.
/// There are three main aggregators available, they are `Reduce`: if the input type is the same as the accumulator type,
/// `Fold`: if the input type is different from the accumulator type, and the accumulator type has a sensible default value, and
/// `FoldFrom`: if the input type is different from the accumulator type, and the accumulator needs to be derived from the first input value.
/// Examples of all three configuration options are below:
/// ```dfir,ignore
/// // Left hand side input will use `Fold`, right hand side input will use `Reduce`,
/// join_fused(Fold::new(|| "default value", |x, y| *x += y), Reduce::new(|x, y| *x -= y))
///
/// // Left hand side input will use `FoldFrom`, and the right hand side input will use `Reduce` again
/// join_fused(FoldFrom::new(|x| "conversion function", |x, y| *x += y), Reduce::new(|x, y| *x *= y))
/// ```
/// The three currently supported fused operator types are `Fold(Fn() -> A, Fn(A, T) -> A)`, `Reduce(Fn(A, A) -> A)`, and `FoldFrom(Fn(T) -> A, Fn(A, T) -> A)`
///
/// `join_fused` first performs a fold_keyed/reduce_keyed operation on each input stream before performing joining. See `join()`. There is currently no equivalent for `FoldFrom` in dfir operators.
///
/// For example, the following two dfir programs are equivalent, the former would optimize into the latter:
///
/// ```dfir
/// source_iter(vec![("key", 0), ("key", 1), ("key", 2)])
///     -> reduce_keyed(|x: &mut _, y| *x += y)
///     -> [0]my_join;
/// source_iter(vec![("key", 2), ("key", 3)])
///     -> fold_keyed(|| 1, |x: &mut _, y| *x *= y)
///     -> [1]my_join;
/// my_join = join_multiset()
///     -> assert_eq([("key", (3, 6))]);
/// ```
///
/// ```dfir
/// use dfir_rs::util::accumulator::{Fold, Reduce};
///
/// source_iter(vec![("key", 0), ("key", 1), ("key", 2)])
///     -> [0]my_join;
/// source_iter(vec![("key", 2), ("key", 3)])
///     -> [1]my_join;
/// my_join = join_fused(Reduce::new(|x, y| *x += y), Fold::new(|| 1, |x, y| *x *= y))
///     -> assert_eq([("key", (3, 6))]);
/// ```
///
/// Here is an example of using `FoldFrom` to derive the accumulator from the first value:
///
/// ```dfir
/// use dfir_rs::util::accumulator::{Fold, FoldFrom};
///
/// source_iter(vec![("key", 0), ("key", 1), ("key", 2)])
///     -> [0]my_join;
/// source_iter(vec![("key", 2), ("key", 3)])
///     -> [1]my_join;
/// my_join = join_fused(FoldFrom::new(|x: u32| x + 3, |x, y| *x += y), Fold::new(|| 1, |x, y| *x *= y))
///     -> assert_eq([("key", (6, 6))]);
/// ```
///
/// The benefit of this is that the state between the reducing/folding operator and the join is merged together.
///
/// `join_fused` follows the same persistence rules as `join` and all other operators. By default, both the left hand side and right hand side are `'tick` persistence. They can be set to `'static` persistence
/// by specifying `'static` in the type arguments of the operator.
///
/// for `join_fused::<'static>`, the operator will replay all _keys_ that the join has ever seen each tick, and not only the new matches from that specific tick.
/// This means that it behaves identically to if `persist::<'static>()` were placed before the inputs and the persistence of
/// for example, the two following examples have identical behavior:
///
/// ```dfir
/// use dfir_rs::util::accumulator::{Fold, Reduce};
///
/// source_iter(vec![("key", 0), ("key", 1), ("key", 2)]) -> persist::<'static>() -> [0]my_join;
/// source_iter(vec![("key", 2)]) -> my_union;
/// source_iter(vec![("key", 3)]) -> defer_tick() -> my_union;
/// my_union = union() -> persist::<'static>() -> [1]my_join;
///
/// my_join = join_fused(Reduce::new(|x, y| *x += y), Fold::new(|| 1, |x, y| *x *= y))
///     -> assert_eq([("key", (3, 2)), ("key", (3, 6))]);
/// ```
///
/// ```dfir
/// use dfir_rs::util::accumulator::{Fold, Reduce};
///
/// source_iter(vec![("key", 0), ("key", 1), ("key", 2)]) -> [0]my_join;
/// source_iter(vec![("key", 2)]) -> my_union;
/// source_iter(vec![("key", 3)]) -> defer_tick() -> my_union;
/// my_union = union() -> [1]my_join;
///
/// my_join = join_fused::<'static>(Reduce::new(|x, y| *x += y), Fold::new(|| 1, |x, y| *x *= y))
///     -> assert_eq([("key", (3, 2)), ("key", (3, 6))]);
/// ```
pub const JOIN_FUSED: OperatorConstraints = OperatorConstraints {
    name: "join_fused",
    categories: &[OperatorCategory::MultiIn],
    hard_range_inn: &(2..=2),
    soft_range_inn: &(2..=2),
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 2,
    persistence_args: &(0..=2),
    type_args: RANGE_0,
    is_external_input: false,
    has_singleton_output: false,
    flo_type: None,
    ports_inn: Some(|| super::PortListSpec::Fixed(parse_quote! { 0, 1 })),
    ports_out: None,
    input_delaytype_fn: |_| Some(DelayType::Stratum),
    write_fn: |wc @ &WriteContextArgs {
                   root,
                   context,
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

        let (rhs_prologue, rhs_prologue_after, rhs_pre_write_iter, rhs_borrow) =
            make_joindata(wc, persistences[1], "rhs").map_err(|err| diagnostics.push(err))?;

        let lhs = &inputs[0];
        let rhs = &inputs[1];

        let lhs_accum = &arguments[0];
        let rhs_accum = &arguments[1];

        // Since both input arguments are stratum blocking then we don't need to keep track of ticks to avoid emitting the same thing twice in the same tick.
        let write_iterator = quote_spanned! {op_span=>
            #lhs_pre_write_iter
            #rhs_pre_write_iter

            let #ident = {
                async fn __check_accum<Accumulator, Key, Accum, St, Hasher, Item>(accum: &mut Accumulator, borrow: &mut ::std::collections::HashMap<Key, Accum, Hasher>, st: St)
                where
                    Accumulator: #root::util::accumulator::Accumulator<Accum, Item>,
                    Key: ::std::cmp::Eq + ::std::hash::Hash + ::std::clone::Clone,
                    St: #root::futures::stream::Stream<Item = (Key, Item)>,
                    Hasher: ::std::hash::BuildHasher,
                    Item: ::std::clone::Clone,
                {
                    #root::compiled::pull::accumulate_all(accum, borrow, st).await;
                }
                #work_fn_async(__check_accum(&mut #lhs_accum, &mut *#lhs_borrow, #lhs)).await;
                #work_fn_async(__check_accum(&mut #rhs_accum, &mut *#rhs_borrow, #rhs)).await;

                // TODO: start the iterator with the smallest len() table rather than always picking rhs.
                #[allow(suspicious_double_ref_op, clippy::clone_on_copy)]
                let iter = #rhs_borrow
                    .iter()
                    .filter_map(|(k, v2)| {
                        #lhs_borrow.get(k).map(|v1| (k.clone(), (v1.clone(), v2.clone())))
                    });
                #root::futures::stream::iter(iter)
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

/// Returns `(prologue, prologue_after, pre_write_iter, borrow)`.
pub(crate) fn make_joindata(
    wc: &WriteContextArgs,
    persistence: Persistence,
    side: &str,
) -> Result<(TokenStream, TokenStream, TokenStream, Ident), Diagnostic> {
    let joindata_ident = wc.make_ident(format!("joindata_{}", side));
    let borrow_ident = wc.make_ident(format!("joindata_{}_borrow", side));

    let &WriteContextArgs {
        context,
        df_ident,
        root,
        op_span,
        ..
    } = wc;

    Ok(match persistence {
        Persistence::None => (
            Default::default(),
            Default::default(),
            quote_spanned! {op_span=>
                let #borrow_ident = &mut #root::rustc_hash::FxHashMap::default();
            },
            borrow_ident,
        ),
        Persistence::Tick | Persistence::Loop | Persistence::Static => {
            let lifespan = wc.persistence_as_state_lifespan(persistence);
            (
                quote_spanned! {op_span=>
                    let #joindata_ident = #df_ident.add_state(::std::cell::RefCell::new(#root::rustc_hash::FxHashMap::default()));
                },
                lifespan.map(|lifespan| quote_spanned! {op_span=>
                    // Reset the value to the initializer fn at the end of each tick/loop execution.
                    #df_ident.set_state_lifespan_hook(#joindata_ident, #lifespan, |rcell| { rcell.take(); });
                }).unwrap_or_default(),
                quote_spanned! {op_span=>
                    let mut #borrow_ident = unsafe {
                        // SAFETY: handles from `#df_ident`.
                        #context.state_ref_unchecked(#joindata_ident)
                    }.borrow_mut();
                },
                borrow_ident
            )
        }
        Persistence::Mutable => panic!(),
    })
}
