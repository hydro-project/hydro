use quote::quote_spanned;

use super::{
    OperatorCategory, OperatorConstraints, OperatorWriteOutput, RANGE_0, RANGE_1, WriteContextArgs,
};

/// > 0 input streams, 1 output stream
///
/// > Arguments: A `#handoff_name` reference to a `handoff()` node.
///
/// Iterates over the referenced handoff buffer each tick, emitting `&T` for each element.
/// This is a zero-copy alternative to `tee()` — multiple `iter_ref` operators can read
/// the same handoff without cloning.
///
/// The referenced handoff must be filled by a producer in the same tick. Scheduling
/// constraints are automatically created via the `#` reference mechanism.
///
/// ```dfir
/// my_buf = source_iter(1..=5_i32) -> handoff();
/// my_buf -> for_each(|v| println!("consumed: {v}"));
///
/// iter_ref(#my_buf) -> for_each(|v: &i32| println!("ref: {v}"));
/// ```
pub const ITER_REF: OperatorConstraints = OperatorConstraints {
    name: "iter_ref",
    categories: &[OperatorCategory::Source],
    hard_range_inn: RANGE_0,
    soft_range_inn: RANGE_0,
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 1,
    persistence_args: RANGE_0,
    type_args: RANGE_0,
    is_external_input: false,
    has_singleton_output: false,
    flo_type: None,
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| None,
    write_fn: |&WriteContextArgs {
                   root,
                   op_span,
                   ident,
                   arguments,
                   ..
               },
               _| {
        let arg = &arguments[0];
        let write_iterator = quote_spanned! {op_span=>
            let #ident = #root::dfir_pipes::pull::iter((#arg).iter());
        };
        Ok(OperatorWriteOutput {
            write_iterator,
            ..Default::default()
        })
    },
};
