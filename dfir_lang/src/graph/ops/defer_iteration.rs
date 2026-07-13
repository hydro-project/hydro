use super::{DelayType, OperatorCategory, OperatorConstraints, IDENTITY_WRITE_FN, RANGE_0, RANGE_1};

/// Deprecated: use `defer_tick()` instead, which now works in both tick and loop contexts.
///
/// This operator is kept as an alias for backwards compatibility.
pub const DEFER_ITERATION: OperatorConstraints = OperatorConstraints {
    name: "defer_iteration",
    categories: &[OperatorCategory::Control],
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 0,
    persistence_args: RANGE_0,
    type_args: RANGE_0,
    is_external_input: false,
    flo_type: None,
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| Some(DelayType::Tick),
    write_fn: IDENTITY_WRITE_FN,
};
