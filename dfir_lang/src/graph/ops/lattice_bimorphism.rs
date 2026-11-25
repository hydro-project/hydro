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
/// > 1 output stream of the `(LhsState, RhsState)` bimorphism output type.
///
/// The function must be a lattice bimorphism for `(LhsItem, RhsState)`, `(LhsState, RhsItem)`, and
/// `(LhsState, RhsState)`. The emitted value has the `(LhsState, RhsState)` output type. Each
/// delta is converted with `LatticeFrom` and merged, so `PairBimorphism` can pair
/// singleton-collection deltas with set or map state even though those `Pair`s do not implement
/// `Merge<Self>`.
///
/// ```dfir
/// use std::collections::HashSet;
/// use lattices::set_union::{CartesianProductBimorphism, SetUnionHashSet, SetUnionSingletonSet};
///
/// lhs_op = source_iter(0..3)
///     -> map(SetUnionSingletonSet::new_from)
///     -> state::<'static, SetUnionHashSet<u32>>();
/// rhs_op = source_iter(3..5)
///     -> map(SetUnionSingletonSet::new_from)
///     -> state::<'static, SetUnionHashSet<u32>>();
///
/// lhs_op[items] -> [0]my_join;
/// rhs_op[items] -> [1]my_join;
/// lhs = lhs_op[state] -> singleton();
/// rhs = rhs_op[state] -> singleton();
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
///
/// `PairBimorphism` over singleton collections. The deltas contain `SetUnionSingletonSet`, which
/// is not `Merge<Self>`; they merge into the `Pair` of the two accumulated sets.
///
/// ```dfir
/// use lattices::Pair;
/// use lattices::PairBimorphism;
/// use lattices::set_union::{SetUnionHashSet, SetUnionSingletonSet};
///
/// lhs_op = source_iter([0, 1])
///     -> map(SetUnionSingletonSet::new_from)
///     -> state::<'static, SetUnionHashSet<u32>>();
/// rhs_op = source_iter([10, 20])
///     -> map(SetUnionSingletonSet::new_from)
///     -> state::<'static, SetUnionHashSet<u32>>();
///
/// lhs_op[items] -> [0]my_join;
/// rhs_op[items] -> [1]my_join;
/// lhs = lhs_op[state] -> singleton();
/// rhs = rhs_op[state] -> singleton();
///
/// my_join = lattice_bimorphism(PairBimorphism, #lhs, #rhs)
///     -> assert_eq([Pair::new(
///         SetUnionHashSet::new_from([0, 1]),
///         SetUnionHashSet::new_from([10, 20]),
///     )]);
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
    flo_type: None,
    ports_inn: Some(|| super::PortListSpec::Fixed(parse_quote! { 0, 1 })),
    ports_out: None,
    input_delaytype_fn: |_| None,
    write_fn: |&WriteContextArgs {
                     root,
                     op_span,
                   is_pull,
                   ident,
                   inputs,
                   arguments,
                   ..
               },
               _| {
        assert!(is_pull);

        let func = &arguments[0];
        let lhs_state = &arguments[1];
        let rhs_state = &arguments[2];

        let lhs_items = &inputs[0];
        let rhs_items = &inputs[1];

        let write_iterator = quote_spanned! {op_span=>
            let #ident = {
                let (lhs_state, rhs_state) = (
                    #lhs_state,
                    #rhs_state,
                );
                #[inline(always)]
                fn check_inputs<'a, Func, LhsPull, RhsPull, LhsState, RhsState, Output, DeltaLhs, DeltaRhs>(
                    lhs_pull: LhsPull,
                    rhs_pull: RhsPull,
                    func: Func,
                    lhs_state: &'a LhsState,
                    rhs_state: &'a RhsState,
                ) -> impl #root::dfir_pipes::pull::Pull<
                    Item = Output,
                    Meta = (),
                    CanPend = <LhsPull::CanPend as #root::dfir_pipes::Toggle>::Or<RhsPull::CanPend>,
                    CanEnd = <LhsPull::CanEnd as #root::dfir_pipes::Toggle>::And<RhsPull::CanEnd>,
                >
                where
                    Func: 'a
                        + #root::lattices::LatticeBimorphism<LhsState, RhsState, Output = Output>
                        + #root::lattices::LatticeBimorphism<LhsPull::Item, RhsState, Output = DeltaLhs>
                        + #root::lattices::LatticeBimorphism<LhsState, RhsPull::Item, Output = DeltaRhs>,
                    LhsPull: 'a + #root::dfir_pipes::pull::Pull,
                    RhsPull: 'a + #root::dfir_pipes::pull::Pull,
                    LhsState: 'static + ::std::clone::Clone,
                    RhsState: 'static + ::std::clone::Clone,
                    Output: #root::lattices::LatticeFrom<DeltaLhs>
                        + #root::lattices::Merge<DeltaLhs>
                        + #root::lattices::LatticeFrom<DeltaRhs>
                        + #root::lattices::Merge<DeltaRhs>,
                {
                    #root::compiled::pull::LatticeBimorphismPull::new(
                        #root::dfir_pipes::pull::Pull::fuse(lhs_pull),
                        #root::dfir_pipes::pull::Pull::fuse(rhs_pull),
                        func,
                        lhs_state,
                        rhs_state,
                    )
                }
                check_inputs(
                    #lhs_items,
                    #rhs_items,
                    #func,
                    lhs_state,
                    rhs_state,
                )
            };
        };

        Ok(OperatorWriteOutput {
            write_prologue: Default::default(),
            write_iterator,
            write_iterator_after: Default::default(),
            write_tick_end: Default::default(),
        })
    },
};
