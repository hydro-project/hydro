use quote::quote_spanned;
use syn::parse_quote;

use super::{
    OperatorCategory, OperatorConstraints, OperatorWriteOutput, RANGE_0, RANGE_1, WriteContextArgs,
};

/// An operator representing a [lattice bimorphism](https://hydro.run/docs/dfir/lattices_crate/lattice_math#lattice-bimorphism).
///
/// > 2 input streams, of type `LhsItem` and `RhsItem`.
///
/// > Three argument, one `LatticeBimorphism` function `Func`, an `LhsState` singleton reference, and an `RhsState` singleton reference.
///
/// > 1 output stream of the output type of the `LatticeBimorphism` function.
///
/// The function must be a lattice bimorphism for both `(LhsState, RhsItem)` and `(RhsState, LhsItem)`.
///
/// ```dfir
/// use std::collections::HashSet;
/// use lattices::set_union::{CartesianProductBimorphism, SetUnionHashSet, SetUnionSingletonSet};
///
/// lhs = source_iter(0..3)
///     -> map(SetUnionSingletonSet::new_from)
///     -> state::<'static, SetUnionHashSet<u32>>();
/// rhs = source_iter(3..5)
///     -> map(SetUnionSingletonSet::new_from)
///     -> state::<'static, SetUnionHashSet<u32>>();
///
/// lhs -> [0]my_join;
/// rhs -> [1]my_join;
///
/// my_join = lattice_bimorphism(CartesianProductBimorphism::<HashSet<_>>::default(), #lhs, #rhs)
///     -> assert_eq([SetUnionHashSet::new(HashSet::from_iter([
///        (0, 3),
///        (0, 4),
///        (1, 3),
///        (1, 4),
///        (2, 3),
///        (2, 4),
///    ]))]);
/// ```
pub const LATTICE_BIMORPHISM: OperatorConstraints = OperatorConstraints {
    name: "lattice_bimorphism",
    categories: &[OperatorCategory::MultiIn],
    hard_range_inn: &(2..=2),
    soft_range_inn: &(2..=2),
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 3,
    persistence_args: RANGE_0,
    type_args: RANGE_0,
    is_external_input: false,
    has_singleton_output: false,
    flo_type: None,
    ports_inn: Some(|| super::PortListSpec::Fixed(parse_quote! { 0, 1 })),
    ports_out: None,
    input_delaytype_fn: |_| None,
    write_fn: |&WriteContextArgs {
                   root,
                   context,
                   op_span,
                   is_pull,
                   ident,
                   inputs,
                   arguments,
                   arguments_handles,
                   ..
               },
               _| {
        assert!(is_pull);

        let func = &arguments[0];
        let lhs_state_handle = &arguments_handles[1];
        let rhs_state_handle = &arguments_handles[2];

        let lhs_items = &inputs[0];
        let rhs_items = &inputs[1];

        let write_iterator = quote_spanned! {op_span=>
            let #ident = {
                #[inline(always)]
                fn check_inputs<'a, Func, LhsStream, RhsStream, LhsState, RhsState, Output>(
                    mut lhs_stream: LhsStream,
                    mut rhs_stream: RhsStream,
                    mut func: Func,
                    lhs_state_handle: #root::scheduled::state::StateHandle<::std::cell::RefCell<LhsState>>,
                    rhs_state_handle: #root::scheduled::state::StateHandle<::std::cell::RefCell<RhsState>>,
                    context: &'a #root::scheduled::context::Context,
                ) -> impl #root::futures::stream::Stream<Item = Output>
                where
                    Func: 'a
                        + #root::lattices::LatticeBimorphism<LhsState, RhsStream::Item, Output = Output>
                        + #root::lattices::LatticeBimorphism<LhsStream::Item, RhsState, Output = Output>,
                    LhsStream: 'a + #root::futures::stream::Stream,
                    RhsStream: 'a + #root::futures::stream::Stream,
                    LhsState: 'static + ::std::clone::Clone,
                    RhsState: 'static + ::std::clone::Clone,
                    Output: #root::lattices::Merge<Output>,
                {
                    let (lhs_state, rhs_state) = unsafe {
                        // SAFETY: handle from `#df_ident.add_state(..)`.
                        (
                            context.state_ref_unchecked(lhs_state_handle),
                            context.state_ref_unchecked(rhs_state_handle),
                        )
                    };

                    #root::compiled::pull::LatticeBimorphismStream::new(
                        lhs_stream, rhs_stream, func, lhs_state, rhs_state,
                    )
                }
                check_inputs(
                    #func,
                    #lhs_items,
                    #rhs_items,
                    #lhs_state_handle,
                    #rhs_state_handle,
                    &#context,
                )
            };
        };

        Ok(OperatorWriteOutput {
            write_iterator,
            ..Default::default()
        })
    },
};
