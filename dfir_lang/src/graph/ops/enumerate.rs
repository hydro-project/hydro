use quote::quote_spanned;

use super::{
    OpInstGenerics, OperatorCategory, OperatorConstraints, OperatorInstance, OperatorWriteOutput,
    Persistence, RANGE_0, RANGE_1, WriteContextArgs,
};
use crate::diagnostic::{Diagnostic, Level};

/// > 1 input stream of type `T`, 1 output stream of type `(usize, T)`
///
/// For each item passed in, enumerate it with its index: `(0, x_0)`, `(1, x_1)`, etc.
///
/// `enumerate` can also be provided with one generic lifetime persistence argument, either
/// `'tick` or `'static`, to specify if indexing resets. If `'tick` (the default) is specified, indexing will
/// restart at zero at the start of each tick. Otherwise `'static` will never reset
/// and count monotonically upwards.
///
/// ```dfir
/// source_iter(vec!["hello", "world"])
///     -> enumerate()
///     -> assert_eq([(0, "hello"), (1, "world")]);
/// ```
pub const ENUMERATE: OperatorConstraints = OperatorConstraints {
    name: "enumerate",
    categories: &[OperatorCategory::Map],
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 0,
    persistence_args: &(0..=1),
    type_args: RANGE_0,
    is_external_input: false,
    has_singleton_output: false,
    flo_type: None,
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| None,
    write_fn: |wc @ &WriteContextArgs {
                   root,
                   op_span,
                   context,
                   df_ident,
                   ident,
                   inputs,
                   outputs,
                   is_pull,
                   op_inst:
                       OperatorInstance {
                           generics:
                               OpInstGenerics {
                                   persistence_args, ..
                               },
                           ..
                       },
                   ..
               },
               diagnostics| {
        let persistence = match persistence_args[..] {
            [] => Persistence::Tick,
            [p @ Persistence::Mutable] => {
                diagnostics.push(Diagnostic::spanned(
                    op_span,
                    Level::Error,
                    format!(
                        "An implementation of `'{}` does not exist",
                        p.to_str_lowercase()
                    ),
                ));
                Persistence::Tick
            }
            [p] => p,
            _ => unreachable!(),
        };

        let input = &inputs[0];
        let output = &outputs[0];

        let counter_ident = wc.make_ident("counterdata");

        let write_prologue = quote_spanned! {op_span=>
            let #counter_ident = #df_ident.add_state(::std::cell::RefCell::new(0..));
        };
        let write_prologue_after = wc
            .persistence_as_state_lifespan(persistence)
            .map(|lifespan| quote_spanned! {op_span=>
                #df_ident.set_state_lifespan_hook(#counter_ident, #lifespan, |rcell| { rcell.replace(0..); });
            }).unwrap_or_default();

        let map_fn = quote_spanned! {op_span=>
            |item| {
                let mut counter = unsafe {
                    // SAFETY: handle from `#df_ident.add_state(..)`.
                    #context.state_ref_unchecked(#counter_ident)
                }.borrow_mut();
                (counter.next().unwrap(), item)
            }
        };
        let write_iterator = if is_pull {
            quote_spanned! {op_span=>
                let #ident = ::std::iter::Iterator::map(#input, #map_fn);
            }
        } else {
            quote_spanned! {op_span=>
                let #ident = #root::pusherator::map::Map::new(#map_fn, #output);
            }
        };

        Ok(OperatorWriteOutput {
            write_prologue,
            write_prologue_after,
            write_iterator,
            ..Default::default()
        })
    },
};
