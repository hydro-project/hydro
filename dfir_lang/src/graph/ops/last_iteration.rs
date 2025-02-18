use super::{FloType, OperatorCategory, OperatorConstraints, IDENTITY_WRITE_FN, RANGE_0, RANGE_1};

// TODO(mingwei)
pub const LAST_ITERATION: OperatorConstraints = OperatorConstraints {
    name: "last_iteration",
    categories: &[OperatorCategory::Unwindowing],
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 0,
    persistence_args: RANGE_0,
    type_args: &(0..=1),
    is_external_input: false,
    has_singleton_output: true,
    flo_type: Some(FloType::Unwindowing),
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| None,
    write_fn: IDENTITY_WRITE_FN,
};
