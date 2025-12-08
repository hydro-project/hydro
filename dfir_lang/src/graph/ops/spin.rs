use quote::quote_spanned;

use super::{
    OperatorCategory, OperatorConstraints, OperatorWriteOutput, RANGE_0, RANGE_1, WriteContextArgs,
};

/// This operator emits Unit, and triggers the start of a new tick at the end of each tick,
/// which will cause spinning-like behavior. Note that `run_available` will run forever,
/// so in the example below we illustrate running manually for 100 ticks.
///
/// ```rustbook
/// let mut flow = dfir_rs::dfir_syntax! {
///     spin() -> for_each(|x| println!("tick {}: {:?}", context.current_tick(), x));
/// };
/// for _ in 1..100 {
///     flow.run_tick();
/// }
/// ```
pub const SPIN: OperatorConstraints = OperatorConstraints {
    name: "spin",
    categories: &[OperatorCategory::Source],
    hard_range_inn: RANGE_0,
    soft_range_inn: RANGE_0,
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 0,
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
                   context,
                   op_span,
                   ident,
                   is_pull,
                   ..
               },
               _| {
        assert!(is_pull);
        let write_iterator = quote_spanned! {op_span=>
            let #ident = #root::tokio_stream::once(());
        };
        let write_iterator_after = quote_spanned! {op_span=>
            #context.schedule_subgraph(#context.current_subgraph(), true);
        };
        Ok(OperatorWriteOutput {
            write_iterator,
            write_iterator_after,
            ..Default::default()
        })
    },
};
