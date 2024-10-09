use syn::parse_quote_spanned;

use super::{
    DelayType, OperatorCategory, OperatorConstraints, WriteContextArgs,
    RANGE_0, RANGE_1,
};

/// > 1 input stream, 1 output stream
///
/// A specialized operator for merging lattices together into a accumulated value. Like [`fold()`](#fold)
/// but specialized for lattice types. `lattice_fold(MyLattice::default)` is equivalent to `fold(MyLattice::default, hydroflow::lattices::Merge::merge)`.
///
/// `lattice_fold` can also be provided with one generic lifetime persistence argument, either
/// `'tick` or `'static`, to specify how data persists. With `'tick`, values will only be collected
/// within the same tick. With `'static`, values will be remembered across ticks and will be
/// aggregated with pairs arriving in later ticks. When not explicitly specified persistence
/// defaults to `'tick`.
///
/// `lattice_fold` is differentiated from `lattice_reduce` in that `lattice_fold` can accumulate into a different type from its input.
/// But it also means that the accumulating type must have a sensible default value
///
/// ```hydroflow
/// use hydroflow::lattices::set_union::SetUnionSingletonSet;
/// use hydroflow::lattices::set_union::SetUnionHashSet;
///
/// source_iter([SetUnionSingletonSet::new_from(7)])
///     -> lattice_fold(SetUnionHashSet::<usize>::default)
///     -> assert_eq([SetUnionHashSet::new_from([7])]);
/// ```
pub const LATTICE_FOLD: OperatorConstraints = OperatorConstraints {
    name: "lattice_fold",
    categories: &[OperatorCategory::LatticeFold],
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 1,
    persistence_args: &(0..=1),
    type_args: RANGE_0,
    is_external_input: false,
    has_singleton_output: false,
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| Some(DelayType::MonotoneAccum),
    write_fn: |wc @ &WriteContextArgs {
                   root,
                   is_pull,
                   op_span,
                   arguments,
                   ..
               },
               diagnostics| {
        assert!(is_pull);

        let first_arg = &arguments[0];

        let arguments = &parse_quote_spanned! {op_span=>
            #first_arg,
            |acc, item| { #root::lattices::Merge::<_>::merge(acc, item); }
        };

        let wc = WriteContextArgs {
            arguments,
            ..wc.clone()
        };

        // Can't do better type checking here because we need heavy type inference
        // to support different accumulator and input types.
        (super::fold::FOLD.write_fn)(&wc, diagnostics)
    },
};
