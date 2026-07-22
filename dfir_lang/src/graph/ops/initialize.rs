use syn::parse_quote_spanned;

use super::{
    OperatorCategory, OperatorConstraints, WriteContextArgs, RANGE_0, RANGE_1,
};

/// > 0 input streams, 1 output stream
///
/// > Arguments: None.
///
/// Emits a single unit `()` at the start of the first tick.
///
/// ```dfir
/// initialize()
///     -> assert_eq([()]);
/// ```
pub const INITIALIZE: OperatorConstraints = OperatorConstraints {
    name: "initialize",
    categories: &[OperatorCategory::Source],
    hard_range_inn: RANGE_0,
    soft_range_inn: RANGE_0,
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 0,
    persistence_args: RANGE_0,
    type_args: RANGE_0,
    is_external_input: false,
    // NOTE: `initialize` is deliberately NOT `FloType::Source`, even though it is categorized as a
    // source. Unlike external/timing sources (`source_stream`, `source_interval`, `spin`, ...) it
    // is a pure, deterministic, one-shot generator, so it is permitted *inside* `loop { ... }`
    // contexts where it serves as a loop-local seed (e.g. `initialize() -> persist::<'loop>()` to
    // materialize a held "absent" marker once per loop execution). Do not "fix" this to
    // `FloType::Source`: that would ban it from loops via `check_loop_errors` and break held-state
    // constructions.
    flo_type: None,
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| None,
    write_fn: |wc @ &WriteContextArgs { op_span, .. }, diagnostics| {
        let wc = WriteContextArgs {
            arguments: &parse_quote_spanned!(op_span=> [()]),
            ..wc.clone()
        };
        (super::source_iter::SOURCE_ITER.write_fn)(&wc, diagnostics)
    },
};
