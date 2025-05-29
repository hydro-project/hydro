use quote::quote_spanned;
use syn::parse_quote_spanned;

use super::{
    DelayType, OperatorCategory, OperatorConstraints, OperatorWriteOutput, RANGE_0, RANGE_1,
    WriteContextArgs,
};

/// > 1 input stream, 1 output stream
///
/// A specialized operator for merging lattices together into an accumulated value. Like [`reduce()`](#reduce)
/// but specialized for lattice types. `lattice_reduce()` is equivalent to `reduce(dfir_rs::lattices::Merge::merge)`.
///
/// `lattice_reduce` can also be provided with one generic lifetime persistence argument, either
/// `'tick` or `'static`, to specify how data persists. With `'tick`, values will only be collected
/// within the same tick. With `'static`, values will be remembered across ticks and will be
/// aggregated with pairs arriving in later ticks. When not explicitly specified persistence
/// defaults to `'tick`.
///
/// `lattice_reduce` is differentiated from `lattice_fold` in that `lattice_reduce` does not require the accumulating type to have a sensible default value.
/// But it also means that the accumulating function inputs and the accumulating type must be the same.
///
/// ```dfir
/// use dfir_rs::lattices::Max;
///
/// source_iter([1, 2, 3, 4, 5])
///     -> map(Max::new)
///     -> lattice_reduce()
///     -> assert_eq([Max::new(5)]);
/// ```
pub const LATTICE_REDUCE: OperatorConstraints = OperatorConstraints {
    name: "lattice_reduce",
    categories: &[OperatorCategory::LatticeFold],
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
    input_delaytype_fn: |_| Some(DelayType::MonotoneAccum),
    write_fn: |wc @ &WriteContextArgs {
                   root,
                   inputs,
                   op_span,
                   is_pull,
                   ..
               },
               diagnostics| {
        assert!(is_pull);

        let arguments = &parse_quote_spanned! {op_span=>
            |acc, item| { #root::lattices::Merge::<_>::merge(acc, item); }
        };
        let wc = WriteContextArgs {
            arguments,
            ..wc.clone()
        };

        let OperatorWriteOutput {
            write_prologue,
            write_prologue_after,
            write_iterator,
            write_iterator_after,
        } = (super::reduce::REDUCE.write_fn)(&wc, diagnostics)?;

        assert_eq!(1, inputs.len());
        let input = &inputs[0];

        let write_iterator = quote_spanned! {op_span=>
            let #input = {
                #[inline(always)]
                fn check_inputs<Lat>(
                    input: impl ::std::iter::Iterator<Item = Lat>
                ) -> impl ::std::iter::Iterator<Item = Lat>
                where
                    Lat: #root::lattices::Merge<Lat>,
                {
                    input
                }
                check_inputs::<_>(#input)
            };
            #write_iterator
        };
        Ok(OperatorWriteOutput {
            write_prologue,
            write_prologue_after,
            write_iterator,
            write_iterator_after,
        })
    },
};
