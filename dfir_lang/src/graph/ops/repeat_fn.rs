use super::{
    OperatorConstraints, OperatorWriteOutput, WriteContextArgs,
    RANGE_0, RANGE_1,
};
use crate::{
    diagnostic::{Diagnostic, Level},
    graph::OperatorInstance,
};
use quote::quote_spanned;
use syn::Expr;
/// > 0 input streams, 1 output stream
///
/// > Arguments: A batch size per tick, and a zero argument closure to produce each item in the stream.
/// Similar to `repeat_iter`, but generates the items by calling the supplied closure instead of cloning them from an input iter
///
/// ```hydroflow
///     repeat_fn(10, || 7) -> for_each(|x| println!("{}", x));
/// ```

pub const REPEAT_FN: OperatorConstraints = OperatorConstraints {
    name: "repeat_fn",
    categories: &[],
    hard_range_inn: RANGE_0,
    soft_range_inn: RANGE_0,
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 2,
    persistence_args: RANGE_0,
    type_args: RANGE_0,
    is_external_input: false,
    has_singleton_output: false,
    flo_type: None,
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| None,
    write_fn: |wc @ &WriteContextArgs {
        context,
        op_span,
        ident,
        arguments,
        ..
    },
               diagnostics| {
        let func = &arguments[1];
        let Expr::Closure(func) = func else {
            diagnostics.push(Diagnostic::spanned(
                op_span,
                Level::Error,
                "Second argument must be a 0 argument closure"),
            );
            return Err(());
        };
        if !func.inputs.is_empty() {
            diagnostics.push(Diagnostic::spanned(
                op_span,
                Level::Error,
                "The function supplied must take zero arguments",
            ));
            return Err(());
        }
        let gen_ident = wc.make_ident("gen_fun");
        let write_prologue = quote_spanned! {op_span=>
            #[allow(unused_mut)] // Sometimes the closure provided is an FnMut in which case it does need mut.
            let mut #gen_ident = #func;
        };
        let batch_size = &arguments[0];
        let write_iterator = quote_spanned! {op_span=>
            let #ident = {
                (0..#batch_size).map(|_| #gen_ident())
            };
        };
        let write_iterator_after = quote_spanned! {op_span=>
            #context.schedule_subgraph(#context.current_subgraph(), true);
        };
        Ok(OperatorWriteOutput {
            write_prologue,
            write_iterator,
            write_iterator_after,
            ..Default::default()
        })
    },
};
