use quote::quote_spanned;
use syn::parse_quote_spanned;

use super::{
    OperatorCategory, OperatorConstraints, WriteContextArgs, RANGE_0, RANGE_1,
};

/// > 1 input stream, 1 optional output stream
/// > Arguments: A Vector, Slice, or Array containing objects that will be compared to the input stream.
///
/// The input stream will be compared with the provided argument, element by element. If any elements do not match, `assert_eq` will panic.
/// If the input stream produces more elements than are in the provided argument, `assert_eq` will panic.
///
/// The input stream is passed through `assert_eq` unchanged to the output stream.
///
/// `assert_eq` is mainly useful for testing and documenting the behavior of dfir code inline.
///
/// `assert_eq` will remember the stream position across ticks, see example.
///
/// ```dfir
/// unioned = union();
///
/// source_iter([1]) -> assert_eq([1]) -> unioned;
/// source_iter([2]) -> defer_tick() -> assert_eq([2]) -> unioned;
///
/// unioned -> assert_eq([1, 2]);
/// ```
pub const ASSERT_EQ: OperatorConstraints = OperatorConstraints {
    name: "assert_eq",
    categories: &[OperatorCategory::Control],
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
    hard_range_out: &(0..=1),
    soft_range_out: &(0..=1),
    num_args: 1,
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
                   df_ident,
                   op_span,
                   arguments,
                   ..
               },
               diagnostics| {
        let assert_index_ident = wc.make_ident("assert_index");

        let arg = &arguments[0];

        let inspect_fn = parse_quote_spanned! {op_span=>
            |item| {
                // This is to help constrain the types so that type inference works nicely.
                fn __constrain_types<T>(array: &impl ::std::ops::Index<usize, Output = T>, index: usize) -> &T {
                    &array[index]
                }

                unsafe {
                    // SAFETY: handle from `#df_ident.add_state(..)`.
                    let index = #context.state_ref_unchecked(#assert_index_ident).get();
                    ::std::assert_eq!(__constrain_types(&#arg, index), item, "Item (right) at index {} does not equal expected (left).", index);
                    #context.state_ref_unchecked(#assert_index_ident).set(index + 1);
                };
            }
        };

        let wc = WriteContextArgs {
            arguments: &inspect_fn,
            ..wc.clone()
        };

        let mut owo = (super::inspect::INSPECT.write_fn)(&wc, diagnostics)?;

        let write_prologue = owo.write_prologue;
        owo.write_prologue = quote_spanned! {op_span=>
            let #assert_index_ident = #df_ident.add_state(
                ::std::cell::Cell::new(0usize)
            );

            #write_prologue
        };

        Ok(owo)
    },
};
