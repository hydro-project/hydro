use quote::quote_spanned;

use super::{
    DelayType, OperatorCategory, OperatorConstraints, OperatorWriteOutput, RANGE_0, RANGE_1,
    WriteContextArgs,
};

/// Takes a `(K, V)` stream as input, groups tuples by key K, sorts the values V within each
/// group in ascending order, and emits `(K, V)` tuples with values sorted per key group.
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
/// `sort_keyed` is blocking. Only the values collected within a single tick will be sorted and
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
            let #ident = {
                let mut map: #root::rustc_hash::FxHashMap<_, ::std::vec::Vec<_>> =
                    #root::rustc_hash::FxHashMap::default();
                {
                    let fut = #root::compiled::pull::ForEach::new(#input, |(k, v)| {
                        map.entry(k).or_default().push(v);
                    });
                    let () = #work_fn_async(fut).await;
                }
                #root::futures::stream::iter(
                    {
                        #[allow(clippy::disallowed_methods, reason = "FxHasher is deterministic")]
                        let items = map.drain()
                            .flat_map(|(k, mut vs)| {
                                vs.sort_unstable();
                                vs.into_iter().map(move |v| (k.clone(), v))
                            })
                            .collect::<::std::vec::Vec<_>>();
                        items
                    }
                )
            };
        };
        Ok(OperatorWriteOutput {
            write_iterator,
            ..Default::default()
        })
    },
};
