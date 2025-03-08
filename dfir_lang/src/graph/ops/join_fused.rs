use proc_macro2::TokenStream;
use quote::{ToTokens, quote_spanned};
use syn::spanned::Spanned;
use syn::{Expr, ExprCall, parse_quote};

use super::{
    DelayType, OpInstGenerics, OperatorCategory, OperatorConstraints, OperatorInstance,
    OperatorWriteOutput, Persistence, RANGE_0, RANGE_1, WriteContextArgs,
};
use crate::diagnostic::{Diagnostic, Level};

/// > 2 input streams of type `<(K, V1)>` and `<(K, V2)>`, 1 output stream of type `<(K, (V1, V2))>`
///
/// `join_fused` takes two arguments, they are the configuration options for the left hand side and right hand side inputs respectively.
/// There are three available configuration options, they are `Reduce`: if the input type is the same as the accumulator type,
/// `Fold`: if the input type is different from the accumulator type, and the accumulator type has a sensible default value, and
/// `FoldFrom`: if the input type is different from the accumulator type, and the accumulator needs to be derived from the first input value.
/// Examples of all three configuration options are below:
/// ```dfir,ignore
/// // Left hand side input will use fold, right hand side input will use reduce,
/// join_fused(Fold(|| "default value", |x, y| *x += y), Reduce(|x, y| *x -= y))
///
/// // Left hand side input will use FoldFrom, and the right hand side input will use Reduce again
/// join_fused(FoldFrom(|x| "conversion function", |x, y| *x += y), Reduce(|x, y| *x *= y))
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
/// source_iter(vec![("key", 0), ("key", 1), ("key", 2)])
///     -> [0]my_join;
/// source_iter(vec![("key", 2), ("key", 3)])
///     -> [1]my_join;
/// my_join = join_fused(Reduce(|x, y| *x += y), Fold(|| 1, |x, y| *x *= y))
///     -> assert_eq([("key", (3, 6))]);
/// ```
///
/// Here is an example of using FoldFrom to derive the accumulator from the first value:
///
/// ```dfir
/// source_iter(vec![("key", 0), ("key", 1), ("key", 2)])
///     -> [0]my_join;
/// source_iter(vec![("key", 2), ("key", 3)])
///     -> [1]my_join;
/// my_join = join_fused(FoldFrom(|x| x + 3, |x, y| *x += y), Fold(|| 1, |x, y| *x *= y))
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
/// source_iter(vec![("key", 0), ("key", 1), ("key", 2)]) -> persist::<'static>() -> [0]my_join;
/// source_iter(vec![("key", 2)]) -> my_union;
/// source_iter(vec![("key", 3)]) -> defer_tick() -> my_union;
/// my_union = union() -> persist::<'static>() -> [1]my_join;
///
/// my_join = join_fused(Reduce(|x, y| *x += y), Fold(|| 1, |x, y| *x *= y))
///     -> assert_eq([("key", (3, 2)), ("key", (3, 6))]);
/// ```
///
/// ```dfir
/// source_iter(vec![("key", 0), ("key", 1), ("key", 2)]) -> [0]my_join;
/// source_iter(vec![("key", 2)]) -> my_union;
/// source_iter(vec![("key", 3)]) -> defer_tick() -> my_union;
/// my_union = union() -> [1]my_join;
///
/// my_join = join_fused::<'static>(Reduce(|x, y| *x += y), Fold(|| 1, |x, y| *x *= y))
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
                   context,
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

        let persistences = parse_persistences(persistence_args);

        let lhs_join_options =
            parse_argument(&arguments[0]).map_err(|err| diagnostics.push(err))?;
        let rhs_join_options =
            parse_argument(&arguments[1]).map_err(|err| diagnostics.push(err))?;

        let (lhs_prologue, lhs_pre_write_iter, lhs_borrow) =
            make_joindata(wc, persistences[0], &lhs_join_options, "lhs")
                .map_err(|err| diagnostics.push(err))?;

        let (rhs_prologue, rhs_pre_write_iter, rhs_borrow) =
            make_joindata(wc, persistences[1], &rhs_join_options, "rhs")
                .map_err(|err| diagnostics.push(err))?;

        let write_prologue = quote_spanned! {op_span=>
            #lhs_prologue
            #rhs_prologue
        };

        let lhs = &inputs[0];
        let rhs = &inputs[1];

        let arg0_span = arguments[0].span();
        let arg1_span = arguments[1].span();

        let lhs_tokens = match lhs_join_options {
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

        let rhs_tokens = match rhs_join_options {
            JoinOptions::FoldFrom(rhs_from, rhs_fold) => quote_spanned! {arg0_span=>
                #rhs_borrow.fold_into(#rhs, #rhs_fold, #rhs_from);
            },
            JoinOptions::Fold(rhs_default, rhs_fold) => quote_spanned! {arg1_span=>
                #rhs_borrow.fold_into(#rhs, #rhs_fold, #rhs_default);
            },
            JoinOptions::Reduce(rhs_reduce) => quote_spanned! {arg1_span=>
                #rhs_borrow.reduce_into(#rhs, #rhs_reduce);
            },
        };

        // Since both input arguments are stratum blocking then we don't need to keep track of ticks to avoid emitting the same thing twice in the same tick.
        let write_iterator = quote_spanned! {op_span=>
            #lhs_pre_write_iter
            #rhs_pre_write_iter

            let #ident = {
                #lhs_tokens
                #rhs_tokens

                // TODO: start the iterator with the smallest len() table rather than always picking rhs.
                #[allow(clippy::clone_on_copy)]
                #[allow(suspicious_double_ref_op)]
                #rhs_borrow
                    .table
                    .iter()
                    .filter_map(|(k, v2)| #lhs_borrow.table.get(k).map(|v1| (k.clone(), (v1.clone(), v2.clone()))))
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
            write_prologue,
            write_iterator,
            write_iterator_after,
        })
    },
};

pub(crate) enum JoinOptions<'a> {
    FoldFrom(&'a Expr, &'a Expr),
    Fold(&'a Expr, &'a Expr),
    Reduce(&'a Expr),
}

pub(crate) fn parse_argument(arg: &Expr) -> Result<JoinOptions, Diagnostic> {
    let Expr::Call(ExprCall {
        attrs: _,
        func,
        paren_token: _,
        args,
    }) = arg
    else {
        return Err(Diagnostic::spanned(
            arg.span(),
            Level::Error,
            format!("Argument must be a function call: {arg:?}"),
        ));
    };

    let mut elems = args.iter();
    let func_name = func.to_token_stream().to_string();

    match func_name.as_str() {
        "Fold" => match (elems.next(), elems.next()) {
            (Some(default), Some(fold)) => Ok(JoinOptions::Fold(default, fold)),
            _ => Err(Diagnostic::spanned(
                args.span(),
                Level::Error,
                format!(
                    "Fold requires two arguments, first is the default function, second is the folding function: {func:?}"
                ),
            )),
        },
        "FoldFrom" => match (elems.next(), elems.next()) {
            (Some(from), Some(fold)) => Ok(JoinOptions::FoldFrom(from, fold)),
            _ => Err(Diagnostic::spanned(
                args.span(),
                Level::Error,
                format!(
                    "FoldFrom requires two arguments, first is the From function, second is the folding function: {func:?}"
                ),
            )),
        },
        "Reduce" => match elems.next() {
            Some(reduce) => Ok(JoinOptions::Reduce(reduce)),
            _ => Err(Diagnostic::spanned(
                args.span(),
                Level::Error,
                format!("Reduce requires one argument, the reducing function: {func:?}"),
            )),
        },
        _ => Err(Diagnostic::spanned(
            func.span(),
            Level::Error,
            format!("Unknown summarizing function: {func:?}"),
        )),
    }
}

pub(crate) fn make_joindata(
    wc: &WriteContextArgs,
    persistence: Persistence,
    join_options: &JoinOptions<'_>,
    side: &str,
) -> Result<(TokenStream, TokenStream, TokenStream), Diagnostic> {
    let joindata_ident = wc.make_ident(format!("joindata_{}", side));
    let borrow_ident = wc.make_ident(format!("joindata_{}_borrow", side));

    let &WriteContextArgs {
        context,
        df_ident,
        root,
        op_span,
        ..
    } = wc;

    let join_type = match *join_options {
        JoinOptions::FoldFrom(_, _) => {
            quote_spanned!(op_span=> #root::compiled::pull::HalfJoinStateFoldFrom)
        }
        JoinOptions::Fold(_, _) => {
            quote_spanned!(op_span=> #root::compiled::pull::HalfJoinStateFold)
        }
        JoinOptions::Reduce(_) => {
            quote_spanned!(op_span=> #root::compiled::pull::HalfJoinStateReduce)
        }
    };

    let (prologue, pre_write_iter, borrow) = match persistence {
        Persistence::None => (
            Default::default(),
            quote_spanned! {op_span=>
                let mut #borrow_ident = #join_type::default();
            },
            quote_spanned! {op_span=>
                #borrow_ident
            },
        ),
        Persistence::Tick => (
            quote_spanned! {op_span=>
                let #joindata_ident = #df_ident.add_state(std::cell::RefCell::new(
                    #root::util::monotonic_map::MonotonicMap::new_init(
                        #join_type::default()
                    )
                ));
            },
            quote_spanned! {op_span=>
                let mut #borrow_ident = unsafe {
                    // SAFETY: handles from `#df_ident`.
                    #context.state_ref_unchecked(#joindata_ident)
                }.borrow_mut();
            },
            quote_spanned! {op_span=>
                #borrow_ident.get_mut_clear(#context.current_tick())
            },
        ),
        Persistence::Static => (
            quote_spanned! {op_span=>
                let #joindata_ident = #df_ident.add_state(std::cell::RefCell::new(
                    #join_type::default()
                ));
            },
            quote_spanned! {op_span=>
                let mut #borrow_ident = unsafe {
                    // SAFETY: handles from `#df_ident`.
                    #context.state_ref_unchecked(#joindata_ident)
                }.borrow_mut();
            },
            quote_spanned! {op_span=>
                #borrow_ident
            },
        ),
        Persistence::Mutable => {
            return Err(Diagnostic::spanned(
                op_span,
                Level::Error,
                "An implementation of 'mutable does not exist",
            ));
        }
    };
    Ok((prologue, pre_write_iter, borrow))
}

pub(crate) fn parse_persistences(persistences: &[Persistence]) -> [Persistence; 2] {
    match persistences {
        [] => [Persistence::Tick, Persistence::Tick],
        [a] => [*a, *a],
        [a, b] => [*a, *b],
        _ => panic!("Too many persistences: {persistences:?}"),
    }
}
