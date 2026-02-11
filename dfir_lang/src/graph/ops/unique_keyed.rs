use quote::quote_spanned;

use super::{
    OperatorCategory, OperatorConstraints, OperatorWriteOutput, RANGE_0, RANGE_1, WriteContextArgs,
};

/// Takes a `(K, V)` stream as input and filters out any duplicate values within each key group.
/// The output contains all unique `(K, V)` pairs, where uniqueness is determined per key.
///
/// ```dfir
/// source_iter([("a", 1), ("b", 2), ("a", 1), ("b", 3), ("a", 2)])
///     -> unique_keyed()
///     -> fold::<'static>(::std::collections::BTreeMap::<&str, Vec<i32>>::new, |map: &mut ::std::collections::BTreeMap<&str, Vec<i32>>, (k, v): (&str, i32)| {
///         map.entry(k).or_default().push(v);
///     })
///     -> for_each(|map: ::std::collections::BTreeMap::<&str, Vec<i32>>| {
///         assert_eq!(map.get("a").unwrap(), &vec![1, 2]);
///         assert_eq!(map.get("b").unwrap(), &vec![2, 3]);
///     });
/// ```
///
/// `unique_keyed` can also be provided with one generic lifetime persistence argument, either
/// `'tick` or `'static`, to specify how data persists. The default is `'tick`.
/// With `'tick`, uniqueness is only considered within the current tick, so across multiple ticks
/// duplicate values may be emitted for the same key.
/// With `'static`, values will be remembered across ticks and no duplicates will ever be emitted
/// for any key.
pub const UNIQUE_KEYED: OperatorConstraints = OperatorConstraints {
    name: "unique_keyed",
    categories: &[OperatorCategory::Persistence],
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
                   ..
               },
               diagnostics| {
        let [persistence] = wc.persistence_args_disallow_mutable(diagnostics);

        let input = &inputs[0];
        let output = &outputs[0];

        let uniquedata_ident = wc.make_ident("uniquedata");

        let write_prologue = quote_spanned! {op_span=>
            let #uniquedata_ident = #df_ident.add_state(::std::cell::RefCell::new(
                #root::rustc_hash::FxHashMap::<_, #root::rustc_hash::FxHashSet<_>>::default()
            ));
        };
        let write_prologue_after = wc
            .persistence_as_state_lifespan(persistence)
            .map(|lifespan| quote_spanned! {op_span=>
                #df_ident.set_state_lifespan_hook(#uniquedata_ident, #lifespan, |rcell| { rcell.take(); });
            }).unwrap_or_default();

        let filter_fn = quote_spanned! {op_span=>
            |(k, v)| {
                let mut map = unsafe {
                    // SAFETY: handle from `#df_ident.add_state(..)`.
                    #context.state_ref_unchecked(#uniquedata_ident)
                }.borrow_mut();

                if let Some(set) = map.get_mut(k) {
                    if set.contains(v) {
                        false
                    } else {
                        set.insert(::std::clone::Clone::clone(v));
                        true
                    }
                } else {
                    let mut set = #root::rustc_hash::FxHashSet::default();
                    set.insert(::std::clone::Clone::clone(v));
                    map.insert(::std::clone::Clone::clone(k), set);
                    true
                }
            }
        };
        let write_iterator = if is_pull {
            quote_spanned! {op_span=>
                let #ident = #root::tokio_stream::StreamExt::filter(#input, #filter_fn);
            }
        } else {
            quote_spanned! {op_span=>
                let #ident = #root::sinktools::filter(#filter_fn, #output);
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
