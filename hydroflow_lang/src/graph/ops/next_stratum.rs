use super::{
    DelayType, OperatorCategory, OperatorConstraints, IDENTITY_WRITE_FN, RANGE_0,
    RANGE_1,
};

/// Delays all elements which pass through to the next stratum (in the same
/// tick).
///
/// You can also supply a type parameter `next_stratum::<MyType>()` to specify what items flow
/// through the the pipeline. This can be useful for helping the compiler infer types.
pub const NEXT_STRATUM: OperatorConstraints = OperatorConstraints {
    name: "next_stratum",
    categories: &[OperatorCategory::Control],
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 0,
    persistence_args: &(0..=1),
    type_args: RANGE_0,
    is_external_input: false,
    has_singleton_output: false,
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| Some(DelayType::Stratum),
    write_fn: IDENTITY_WRITE_FN,
};
