use crate::graph::PortIndexValue;

use super::{
    DelayType, OperatorCategory, OperatorConstraints, RANGE_0, RANGE_1
};

/// > 2 input streams of the same type, 1 output stream of the same type
///
/// Chains together a pair of streams, with all the elements of the first emitted before the second.
///
/// Since `chain` has multiple input streams, it needs to be assigned to
/// a variable to reference its multiple input ports across statements.
///
/// ```dfir
/// source_iter(vec!["hello", "world"]) -> [0]my_chain;
/// source_iter(vec!["stay", "gold"]) -> [1]my_chain;
/// my_chain = chain()
///     -> map(|x| x.to_uppercase())
///     -> assert_eq(["HELLO", "WORLD", "STAY", "GOLD"]);
/// ```
pub const CHAIN: OperatorConstraints = OperatorConstraints {
    name: "chain",
    categories: &[OperatorCategory::MultiIn],
    persistence_args: RANGE_0,
    type_args: RANGE_0,
    hard_range_inn: &(2..=2),
    soft_range_inn: &(2..=2),
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 0,
    is_external_input: false,
    has_singleton_output: false,
    flo_type: None,
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |idx| match idx {
        PortIndexValue::Int(idx) if idx.value == 0 => {
            Some(DelayType::Stratum)
        }
        _else => None,
    },
    write_fn: super::union::UNION.write_fn,
};
