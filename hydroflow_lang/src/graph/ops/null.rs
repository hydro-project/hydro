use super::{OperatorCategory, OperatorConstraints, NULL_WRITE_FN, RANGE_0};

/// > unbounded number of input streams of any type, unbounded number of output streams of any type.
///
/// As a source, generates nothing. As a sink, absorbs anything with no effect.
///
/// ```hydroflow
/// // should print `1, 2, 3, 4, 5, 6, a, b, c` across 9 lines
/// null() -> for_each(|_: ()| panic!());
/// source_iter([1,2,3]) -> map(|i| println!("{}", i)) -> null();
/// null_src = null();
/// null_sink = null();
/// null_src[0] -> for_each(|_: ()| panic!());
/// // note: use `for_each()` (or `inspect()`) instead of this:
/// source_iter([4,5,6]) -> map(|i| println!("{}", i)) -> [0]null_sink;
/// ```
pub const NULL: OperatorConstraints = OperatorConstraints {
    name: "null",
    categories: &[OperatorCategory::Source, OperatorCategory::Sink],
    hard_range_inn: &(0..=1),
    soft_range_inn: &(0..=1),
    hard_range_out: &(0..=1),
    soft_range_out: &(0..=1),
    num_args: 0,
    persistence_args: RANGE_0,
    type_args: &(0..=1),
    is_external_input: false,
    has_singleton_output: false,
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| None,
    write_fn: NULL_WRITE_FN,
};
