use quote::quote_spanned;
use syn::{parse_quote, parse_quote_spanned};

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
                   op_inst:
                       OperatorInstance {
                           generics: OpInstGenerics { type_args, .. },
                           ..
                       },
                   ..
               },
               diagnostics| {
        let lhs_type = &type_args[0];
        let rhs_type = &type_args[1];

        let wc = WriteContextArgs {
            arguments: &parse_quote_spanned! {op_span=>
                #root::compiled::pull::join_fused::FoldFrom::new(
                    <#lhs_type as #root::lattices::LatticeFrom::<_>>::lattice_from,
                    |state, delta| { #root::lattices::Merge::merge(state, delta); },
                ),
                #root::compiled::pull::join_fused::FoldFrom::new(
                    <#rhs_type as #root::lattices::LatticeFrom::<_>>::lattice_from,
                    |state, delta| { #root::lattices::Merge::merge(state, delta); },
                ),
            },
            ..wc.clone()
        };

        // Use `join_fused`'s codegen.
        let OperatorWriteOutput {
            write_prologue,
            write_prologue_after,
            write_iterator,
            write_iterator_after,
        } = (super::join_fused::JOIN_FUSED.write_fn)(&wc, diagnostics).unwrap();

        let write_iterator = quote_spanned! {op_span=>
            #write_iterator

            #[allow(suspicious_double_ref_op, clippy::clone_on_copy)]
            let #ident = #root::futures::stream::StreamExt::map(#ident, |(k, (v1, v2))| {
                #root::lattices::map_union::MapUnionSingletonMap::new_from(
                    (
                        k,
                        #root::lattices::Pair::<#lhs_type, #rhs_type>::new_from(v1, v2),
                    ),
                )
            });
        };

        Ok(OperatorWriteOutput {
            write_prologue,
            write_prologue_after,
            write_iterator,
            write_iterator_after,
        })
    },
};
