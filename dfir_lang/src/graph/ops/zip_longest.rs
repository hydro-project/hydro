use quote::quote_spanned;
use syn::parse_quote;

use super::{
    DelayType, OpInstGenerics, OperatorCategory, OperatorConstraints, OperatorInstance,
    OperatorWriteOutput, Persistence, RANGE_0, RANGE_1, WriteContextArgs,
};
use crate::diagnostic::{Diagnostic, Level};

/// > 2 input streams of type `V1` and `V2`, 1 output stream of type `itertools::EitherOrBoth<V1, V2>`
///
/// Zips the streams together, forming paired tuples of the inputs. Note that zipping is done
/// per-tick. Excess items are returned as `EitherOrBoth::Left(V1)` or `EitherOrBoth::Right(V2)`.
/// If you intead want to discard the excess, use [`zip`](#zip) instead.
///
/// ```dfir
/// source_iter(0..2) -> [0]my_zip_longest;
/// source_iter(0..3) -> [1]my_zip_longest;
/// my_zip_longest = zip_longest()
///     -> assert_eq([
///         itertools::EitherOrBoth::Both(0, 0),
///         itertools::EitherOrBoth::Both(1, 1),
///         itertools::EitherOrBoth::Right(2)]);
/// ```
pub const ZIP_LONGEST: OperatorConstraints = OperatorConstraints {
    name: "zip_longest",
    categories: &[OperatorCategory::MultiIn],
    hard_range_inn: &(2..=2),
    soft_range_inn: &(2..=2),
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 0,
    persistence_args: &(0..=1),
    type_args: RANGE_0,
    is_external_input: false,
    has_singleton_output: false,
    flo_type: None,
    ports_inn: Some(|| super::PortListSpec::Fixed(parse_quote! { 0, 1 })),
    ports_out: None,
    input_delaytype_fn: |_| Some(DelayType::Stratum),
    write_fn: |&WriteContextArgs {
                   root,
                   op_span,
                   ident,
                   is_pull,
                   inputs,
                   op_name,
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
        assert!(is_pull);

        let persistence = match persistence_args[..] {
            [] => Persistence::Tick,
            [a] => a,
            _ => unreachable!(),
        };
        if Persistence::Tick != persistence {
            diagnostics.push(Diagnostic::spanned(
                op_span,
                Level::Error,
                format!("`{}()` can only have `'tick` persistence.", op_name),
            ));
            // Fall-thru to still generate code.
        }

        let lhs = &inputs[0];
        let rhs = &inputs[1];
        let write_iterator = quote_spanned! {op_span=>
            let #ident = #root::compiled::pull::ZipLongest::new(
                #root::futures::stream::StreamExt::fuse(#lhs),
                #root::futures::stream::StreamExt::fuse(#rhs),
            );
        };

        Ok(OperatorWriteOutput {
            write_iterator,
            ..Default::default()
        })
    },
};
