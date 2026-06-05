use proc_macro2::Span;
use syn::parse_quote_spanned;
use super::{
    OperatorCategory, OperatorConstraints, PortListSpec,
    WriteContextArgs, RANGE_1,
};

// TODO(mingwei): Improve example when things are more stable.
/// A lattice-based state operator, used for accumulating lattice state
///
/// Has two output ports:
/// - `[items]`: emits the input items that actually changed the lattice state (deltas).
/// - `[state]`: emits a clone of the accumulated lattice value after all items are processed.
///
/// ```dfir
/// use std::collections::HashSet;
///
/// use lattices::set_union::{CartesianProductBimorphism, SetUnionHashSet, SetUnionSingletonSet};
///
/// my_state = source_iter(0..3)
///     -> map(SetUnionSingletonSet::new_from)
///     -> state::<SetUnionHashSet<usize>>();
/// my_state[items] -> null();
/// my_state[state] -> null();
/// ```
/// The `state` operator is equivalent to `state_by` used with an identity mapping operator with
/// `Default::default` providing the factory function.
pub const STATE: OperatorConstraints = OperatorConstraints {
    name: "state",
    categories: &[OperatorCategory::Persistence],
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
    hard_range_out: &(2..=2),
    soft_range_out: &(2..=2),
    num_args: 0,
    persistence_args: &(0..=1),
    type_args: &(0..=1),
    is_external_input: false,
    flo_type: None,
    ports_inn: None,
    ports_out: Some(|| PortListSpec::Fixed(parse_quote_spanned!(Span::call_site()=> items, state))),
    input_delaytype_fn: |_| None,
    write_fn: |wc @ &WriteContextArgs { op_span, .. },
               diagnostics| {

        let wc = WriteContextArgs {
            arguments: &parse_quote_spanned!(op_span => ::std::convert::identity, ::std::default::Default::default),
            ..wc.clone()
        };

        (super::state_by::STATE_BY.write_fn)(&wc, diagnostics)
    },
};
