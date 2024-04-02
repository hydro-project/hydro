use quote::quote_spanned;

use super::{
    GraphEdgeType, OperatorCategory, OperatorConstraints, OperatorWriteOutput, WriteContextArgs,
    RANGE_0, RANGE_1,
};

/// > 1 input stream, 0 output streams
///
/// > Arguments: a Rust closure
///
/// Iterates through a stream passing each element to the closure in the
/// argument.
///
/// > Note: The closure has access to the [`context` object](surface_flows.md#the-context-object).
///
/// ```hydroflow
///     source_iter(vec!["Hello", "World"])
///         -> for_each(|x| println!("{}", x));
/// ```
pub const FOR_EACH: OperatorConstraints = OperatorConstraints {
    name: "for_each",
    categories: &[OperatorCategory::Sink],
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
    hard_range_out: RANGE_0,
    soft_range_out: RANGE_0,
    num_args: 1,
    persistence_args: RANGE_0,
    type_args: RANGE_0,
    is_external_input: false,
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| None,
    input_edgetype_fn: |_| Some(GraphEdgeType::Value),
    output_edgetype_fn: |_| GraphEdgeType::Value,
    flow_prop_fn: None,
    write_fn: |wc @ &WriteContextArgs {
                   root,
                   op_span,
                   ident,
                   arguments,
                   ..
               },
               _| {
        let func = wc.wrap_check_func_arg(&arguments[0]);
        let write_iterator = quote_spanned! {op_span=>
            let #ident = #root::pusherator::for_each::ForEach::new(#func);
        };
        Ok(OperatorWriteOutput {
            write_iterator,
            ..Default::default()
        })
    },
};
