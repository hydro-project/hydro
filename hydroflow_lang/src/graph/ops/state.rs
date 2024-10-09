use syn::parse_quote_spanned;
use super::{
    OperatorCategory, OperatorConstraints,
    WriteContextArgs, RANGE_1,
};

// TODO(mingwei): Improve example when things are more stable.
/// A lattice-based state operator, used for accumulating lattice state
///
/// Emits both a referenceable singleton and (optionally) a pass-through stream. In the future the
/// pass-through stream may be deduplicated.
///
/// ```hydroflow
/// use std::collections::HashSet;
///
/// use lattices::set_union::{CartesianProductBimorphism, SetUnionHashSet, SetUnionSingletonSet};
///
/// my_state = source_iter(0..3)
///     -> map(SetUnionSingletonSet::new_from)
///     -> state::<SetUnionHashSet<usize>>();
/// ```
pub const STATE: OperatorConstraints = OperatorConstraints {
    name: "state",
    categories: &[OperatorCategory::Persistence],
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
    hard_range_out: &(0..=1),
    soft_range_out: &(0..=1),
    num_args: 0,
    persistence_args: &(0..=1),
    type_args: &(0..=1),
    is_external_input: false,
    has_singleton_output: true,
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| None,
    write_fn: |wc @ &WriteContextArgs { op_span, .. },
               diagnostics| {

        let wc = WriteContextArgs {
            arguments: &parse_quote_spanned!(op_span => ::std::convert::identity),
            ..wc.clone()
        };

        (super::state_by::STATE_BY.write_fn)(&wc, diagnostics)
    },
};
