use quote::quote_spanned;

use crate::graph::{
    PortIndexValue,
    ops::{OperatorWriteOutput, WriteContextArgs},
};

use super::{DelayType, OperatorCategory, OperatorConstraints, RANGE_0, RANGE_1};

/// > 2 input streams of the same type, 1 output stream of the same type
///
/// Chains together a pair of streams, with all the elements of the first emitted before the second,
/// emitting up to `N` elements where `N` is passed as an argument.
///
/// Since `chain_first_n` has multiple input streams, it needs to be assigned to
/// a variable to reference its multiple input ports across statements.
///
/// ```dfir
/// source_iter(vec!["hello", "world"]) -> [0]my_chain;
/// source_iter(vec!["stay", "gold"]) -> [1]my_chain;
/// my_chain = chain_first_n(3)
///     -> map(|x| x.to_uppercase())
///     -> assert_eq(["HELLO", "WORLD", "STAY"]);
/// ```
pub const CHAIN_FIRST_N: OperatorConstraints = OperatorConstraints {
    name: "chain_first_n",
    categories: &[OperatorCategory::MultiIn],
    persistence_args: RANGE_0,
    type_args: RANGE_0,
    hard_range_inn: &(2..),
    soft_range_inn: &(2..),
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 1,
    is_external_input: false,
    has_singleton_output: false,
    flo_type: None,
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |idx| match idx {
        PortIndexValue::Int(idx) if idx.value == 0 => {
            // will no longer be needed once subgraphs are always DAGs (only run once per tick)
            Some(DelayType::Stratum)
        }
        _else => None,
    },
    write_fn: |wc @ &WriteContextArgs {
                   root,
                   op_span,
                   ident,
                   is_pull,
                   arguments,
                   ..
               },
               diagnostics| {
        assert!(is_pull);

        let OperatorWriteOutput {
            write_prologue,
            write_prologue_after,
            write_iterator,
            write_iterator_after,
        } = (super::union::UNION.write_fn)(wc, diagnostics)?;

        let arg_n = &arguments[0];

        let write_iterator = quote_spanned! {op_span=>
            #write_iterator
            let #ident = #root::futures::stream::StreamExt::take(#ident, #arg_n);
        };

        Ok(OperatorWriteOutput {
            write_prologue,
            write_prologue_after,
            write_iterator,
            write_iterator_after,
        })
    },
};
