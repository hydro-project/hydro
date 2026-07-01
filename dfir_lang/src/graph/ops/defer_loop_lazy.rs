use super::{
    DelayType, FloType, OperatorCategory, OperatorConstraints, IDENTITY_WRITE_FN, RANGE_0, RANGE_1,
};

/// Buffers all input items and releases them on the next iteration of the enclosing loop.
/// Does NOT cause the loop to re-fire (lazy).
///
/// Must be used inside a `loop { }` block. Data written in one loop firing
/// is available on the next firing, but will not trigger the loop to fire.
/// The data persists until the loop fires for another reason (e.g., new
/// data arriving at an entry handoff, or a non-lazy `defer_loop` triggering).
pub const DEFER_LOOP_LAZY: OperatorConstraints = OperatorConstraints {
    name: "defer_loop_lazy",
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
    input_delaytype_fn: |_| Some(DelayType::LoopLazy),
    write_fn: IDENTITY_WRITE_FN,
};
