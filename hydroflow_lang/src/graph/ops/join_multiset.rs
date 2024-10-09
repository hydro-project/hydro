use syn::{parse_quote, parse_quote_spanned};

use super::{
    OpInstGenerics, OperatorCategory, OperatorConstraints, OperatorInstance,
    WriteContextArgs, RANGE_0, RANGE_1,
};

/// > 2 input streams of type <(K, V1)> and <(K, V2)>, 1 output stream of type <(K, (V1, V2))>
///
/// This operator is equivalent to `join` except that the LHS and RHS are collected into multisets rather than sets before joining.
///
/// If you want
/// duplicates eliminated from the inputs, use the [`join`](#join) operator.
///
/// For example:
/// ```hydroflow
/// lhs = source_iter([("a", 0), ("a", 0)]) -> tee();
/// rhs = source_iter([("a", "hydro")]) -> tee();
///
/// lhs -> [0]multiset_join;
/// rhs -> [1]multiset_join;
/// multiset_join = join_multiset() -> assert_eq([("a", (0, "hydro")), ("a", (0, "hydro"))]);
///
/// lhs -> [0]set_join;
/// rhs -> [1]set_join;
/// set_join = join() -> assert_eq([("a", (0, "hydro"))]);
/// ```
pub const JOIN_MULTISET: OperatorConstraints = OperatorConstraints {
    name: "join_multiset",
    categories: &[OperatorCategory::MultiIn],
    hard_range_inn: &(2..=2),
    soft_range_inn: &(2..=2),
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 0,
    persistence_args: &(0..=2),
    type_args: RANGE_0,
    is_external_input: false,
    has_singleton_output: false,
    ports_inn: Some(|| super::PortListSpec::Fixed(parse_quote! { 0, 1 })),
    ports_out: None,
    input_delaytype_fn: |_| None,
    write_fn: |wc @ &WriteContextArgs {
                   root,
                   op_span,
                   op_inst: op_inst @ OperatorInstance { .. },
                   ..
               },
               diagnostics| {
        let join_type = parse_quote_spanned! {op_span=> // Uses `lat_type.span()`!
            #root::compiled::pull::HalfMultisetJoinState
        };

        let wc = WriteContextArgs {
            op_inst: &OperatorInstance {
                generics: OpInstGenerics {
                    type_args: vec![join_type],
                    ..wc.op_inst.generics.clone()
                },
                ..op_inst.clone()
            },
            ..wc.clone()
        };

        (super::join::JOIN.write_fn)(&wc, diagnostics)
    },
};
