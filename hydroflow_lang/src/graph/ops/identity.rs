use super::{
    OperatorCategory, OperatorConstraints, IDENTITY_WRITE_FN, RANGE_0, RANGE_1,
};

/// > 1 input stream of type T, 1 output stream of type T
///
/// For each item passed in, pass it out without any change.
///
/// ```hydroflow
/// source_iter(vec!["hello", "world"])
///     -> identity()
///     -> assert_eq(["hello", "world"]);
/// ```
///
/// You can also supply a type parameter `identity::<MyType>()` to specify what items flow through the
/// the pipeline. This can be useful for helping the compiler infer types.
///
/// ```hydroflow
/// // Use type parameter to ensure items are `i32`s.
/// source_iter(0..10)
///     -> identity::<i32>()
///     -> assert_eq([0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
/// ```
pub const IDENTITY: OperatorConstraints = OperatorConstraints {
    name: "identity",
    categories: &[OperatorCategory::Map],
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 0,
    persistence_args: RANGE_0,
    type_args: &(0..=1),
    is_external_input: false,
    has_singleton_output: false,
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| None,
    write_fn: IDENTITY_WRITE_FN,
};
