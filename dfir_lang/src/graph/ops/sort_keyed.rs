use quote::quote_spanned;

use super::{
    DelayType, OperatorCategory, OperatorConstraints, OperatorWriteOutput, RANGE_0, RANGE_1,
    WriteContextArgs,
};

/// Takes a `(K, V)` stream as input and sorts tuples lexicographically by `(K, V)`, so that
/// values within each key group are in ascending order and keys are grouped together.
///
/// ```dfir
/// source_iter([("a", 3), ("b", 1), ("a", 1), ("b", 2)])
///     -> sort_keyed()
///     -> fold::<'static>(::std::collections::BTreeMap::<&str, Vec<i32>>::new, |map: &mut ::std::collections::BTreeMap<&str, Vec<i32>>, (k, v): (&str, i32)| {
///         map.entry(k).or_default().push(v);
///     })
///     -> for_each(|map| {
///         assert_eq!(map.get("a").unwrap(), &vec![1, 3]);
///         assert_eq!(map.get("b").unwrap(), &vec![1, 2]);
///     });
/// ```
///
/// `sort_keyed` is blocking. Only the tuples collected within a single tick will be sorted and
/// emitted.
pub const SORT_KEYED: OperatorConstraints = OperatorConstraints {
    name: "sort_keyed",
    categories: &[OperatorCategory::KeyedFold],
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
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
    input_delaytype_fn: |_| Some(DelayType::Stratum),
    write_fn: |&WriteContextArgs {
                   root,
                   op_span,
                   work_fn_async,
                   ident,
                   inputs,
                   is_pull,
                   ..
               },
               _| {
        assert!(is_pull);

        let input = &inputs[0];
        let write_iterator = quote_spanned! {op_span=>
            // TODO(mingwei): unnecessary extra handoff into_iter() then collect().
            let #ident = {
                let mut tmp = #work_fn_async(#root::futures::stream::StreamExt::collect::<::std::vec::Vec<_>>(#input)).await;
                <[_]>::sort_unstable_by(&mut tmp, |a, b| a.1.cmp(&b.1));
                #root::futures::stream::iter(tmp)
            };
        };
        Ok(OperatorWriteOutput {
            write_iterator,
            ..Default::default()
        })
    },
};
