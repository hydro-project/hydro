use quote::quote_spanned;
use syn::parse_quote;
use syn::spanned::Spanned;

use super::{
    DelayType, OpInstGenerics, OperatorCategory, OperatorConstraints, OperatorInstance,
    OperatorWriteOutput, RANGE_1, WriteContextArgs,
};

/// > 2 input streams of type `(K, V1)` and `(K, V2)`, 1 output stream of type `(K, (V1', V2'))` where `V1`, `V2`, `V1'`, `V2'` are lattice types
///
/// Performs a [`fold_keyed`](#fold_keyed) with lattice-merge aggregate function on each input and then forms the
/// equijoin of the resulting key/value pairs in the input streams by their first (key) attribute.
/// Unlike [`join`](#join), the result is not a stream of tuples, it's a stream of MapUnionSingletonMap
/// lattices. You can (non-monotonically) "reveal" these as tuples if desired via [`map`](#map); see the examples below.
///
/// You must specify the the accumulating lattice types, they cannot be inferred. The first type argument corresponds to the `[0]` input of the join, and the second to the `[1]` input.
/// Type arguments are specified in dfir using the rust turbofish syntax `::<>`, for example `_lattice_join_fused_join::<Min<_>, Max<_>>()`
/// The accumulating lattice type is not necessarily the same type as the input, see the below example involving SetUnion for such a case.
///
/// Like [`join`](#join), `_lattice_join_fused_join` can also be provided with one or two generic lifetime persistence arguments, either
/// `'tick` or `'static`, to specify how join data persists. With `'tick`, pairs will only be
/// joined with corresponding pairs within the same tick. With `'static`, pairs will be remembered
/// across ticks and will be joined with pairs arriving in later ticks. When not explicitly
/// specified persistence defaults to `tick.
///
/// Like [`join`](#join), when two persistence arguments are supplied the first maps to port `0` and the second maps to
/// port `1`.
/// When a single persistence argument is supplied, it is applied to both input ports.
/// When no persistence arguments are applied it defaults to `'tick` for both.
/// It is important to specify all persistence arguments before any type arguments, otherwise the persistence arguments will be ignored.
///
/// The syntax is as follows:
/// ```dfir,ignore
/// _lattice_join_fused_join::<MaxRepr<usize>, MaxRepr<usize>>(); // Or
/// _lattice_join_fused_join::<'static, MaxRepr<usize>, MaxRepr<usize>>();
///
/// _lattice_join_fused_join::<'tick, MaxRepr<usize>, MaxRepr<usize>>();
///
/// _lattice_join_fused_join::<'static, 'tick, MaxRepr<usize>, MaxRepr<usize>>();
///
/// _lattice_join_fused_join::<'tick, 'static, MaxRepr<usize>, MaxRepr<usize>>();
/// // etc.
/// ```
///
/// ### Examples
///
/// ```dfir
/// use dfir_rs::lattices::Min;
/// use dfir_rs::lattices::Max;
///
/// source_iter([("key", Min::new(1)), ("key", Min::new(2))]) -> [0]my_join;
/// source_iter([("key", Max::new(1)), ("key", Max::new(2))]) -> [1]my_join;
///
/// my_join = _lattice_join_fused_join::<'tick, Min<usize>, Max<usize>>()
///     -> map(|singleton_map| {
///         let lattices::collections::SingletonMap(k, v) = singleton_map.into_reveal();
///         (k, (v.into_reveal()))
///     })
///     -> assert_eq([("key", (Min::new(1), Max::new(2)))]);
/// ```
///
/// ```dfir
/// use dfir_rs::lattices::set_union::SetUnionSingletonSet;
/// use dfir_rs::lattices::set_union::SetUnionHashSet;
///
/// source_iter([("key", SetUnionSingletonSet::new_from(0)), ("key", SetUnionSingletonSet::new_from(1))]) -> [0]my_join;
/// source_iter([("key", SetUnionHashSet::new_from([0])), ("key", SetUnionHashSet::new_from([1]))]) -> [1]my_join;
///
/// my_join = _lattice_join_fused_join::<'tick, SetUnionHashSet<usize>, SetUnionHashSet<usize>>()
///     -> map(|singleton_map| {
///         let lattices::collections::SingletonMap(k, v) = singleton_map.into_reveal();
///         (k, (v.into_reveal()))
///     })
///     -> assert_eq([("key", (SetUnionHashSet::new_from([0, 1]), SetUnionHashSet::new_from([0, 1])))]);
/// ```
pub const _LATTICE_JOIN_FUSED_JOIN: OperatorConstraints = OperatorConstraints {
    name: "_lattice_join_fused_join",
    categories: &[OperatorCategory::CompilerFusionOperator],
    hard_range_inn: &(2..=2),
    soft_range_inn: &(2..=2),
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 0,
    persistence_args: &(0..=2),
    type_args: &(2..=2),
    is_external_input: false,
    has_singleton_output: false,
    flo_type: None,
    ports_inn: Some(|| super::PortListSpec::Fixed(parse_quote! { 0, 1 })),
    ports_out: None,
    input_delaytype_fn: |_| Some(DelayType::MonotoneAccum),
    write_fn: |wc @ &WriteContextArgs {
                   root,
                   op_span,
                   ident,
                   inputs,
                   is_pull,
                   op_inst:
                       OperatorInstance {
                           generics:
                               OpInstGenerics {
                                   type_args,
                                   persistence_args,
                                   ..
                               },
                           ..
                       },
                   ..
               },
               diagnostics| {
        let lhs_type = &type_args[0];
        let rhs_type = &type_args[1];

        let wc = WriteContextArgs {
            arguments: &parse_quote! {
                FoldFrom(<#lhs_type as #root::lattices::LatticeFrom::<_>>::lattice_from, #root::lattices::Merge::merge),
                FoldFrom(<#rhs_type as #root::lattices::LatticeFrom::<_>>::lattice_from, #root::lattices::Merge::merge)
            },
            ..wc.clone()
        };

        // initialize write_prologue and write_iterator_after via join_fused, but specialize the write_iterator
        let OperatorWriteOutput {
            write_prologue,
            write_prologue_after,
            write_iterator: _,
            write_iterator_after,
        } = (super::join_fused::JOIN_FUSED.write_fn)(&wc, diagnostics).unwrap();

        assert!(is_pull);
        let persistences = super::join_fused::parse_persistences(persistence_args);

        let lhs_join_options = super::join_fused::parse_argument(&wc.arguments[0])
            .map_err(|err| diagnostics.push(err))?;
        let rhs_join_options = super::join_fused::parse_argument(&wc.arguments[1])
            .map_err(|err| diagnostics.push(err))?;
        let (_lhs_prologue, _lhs_prologue_after, lhs_pre_write_iter, lhs_borrow) =
            super::join_fused::make_joindata(&wc, persistences[0], &lhs_join_options, "lhs")
                .map_err(|err| diagnostics.push(err))?;

        let (_rhs_prologue, _rhs_prologue_after, rhs_pre_write_iter, rhs_borrow) =
            super::join_fused::make_joindata(&wc, persistences[1], &rhs_join_options, "rhs")
                .map_err(|err| diagnostics.push(err))?;

        let lhs = &inputs[0];
        let rhs = &inputs[1];

        let arg0_span = wc.arguments[0].span();
        let arg1_span = wc.arguments[1].span();

        let lhs_tokens = quote_spanned! {arg0_span=>
            #lhs_borrow.fold_into(#lhs, #root::lattices::Merge::merge,
                <#lhs_type as #root::lattices::LatticeFrom::<_>>::lattice_from);
        };

        let rhs_tokens = quote_spanned! {arg1_span=>
            #rhs_borrow.fold_into(#rhs, #root::lattices::Merge::merge,
                <#rhs_type as #root::lattices::LatticeFrom::<_>>::lattice_from);
        };

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
                    .filter_map(|(k, v2)| #lhs_borrow.table.get(k).map(|v1| (k.clone(), lattices::Pair::<#lhs_type, #rhs_type>::new_from(v1.clone(), v2.clone()))))
                    .map(|(key, p)| #root::lattices::map_union::MapUnionSingletonMap::new_from((key, p)))
            };
        };

        Ok(OperatorWriteOutput {
            write_prologue,
            write_prologue_after,
            write_iterator,
            write_iterator_after,
        })
    },
};
