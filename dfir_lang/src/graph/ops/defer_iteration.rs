use super::{
    DelayType, FloType, OperatorCategory, OperatorConstraints, IDENTITY_WRITE_FN, RANGE_0, RANGE_1,
};

/// Buffers all input items and releases them on the next iteration of the enclosing loop.
/// Causes the loop to re-fire (non-lazy).
///
/// Must be used inside a `loop { }` block. Data written in one loop iteration
/// is available on the next iteration. The presence of buffered data will cause
/// the loop to fire again (similar to how `defer_tick` causes a new tick).
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
    flo_type: Some(FloType::NextIteration),
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| Some(DelayType::Loop),
    write_fn: IDENTITY_WRITE_FN,
};
