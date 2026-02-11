use quote::quote_spanned;

use super::{
    OperatorCategory, OperatorConstraints, OperatorWriteOutput, RANGE_0, RANGE_1, WriteContextArgs,
};

/// Takes a `(K, V)` stream as input and assigns a zero-based index to each value within each
/// key group, emitting `(K, (usize, V))` tuples. Indices are tracked independently per key,
/// so each key's values are enumerated starting from 0.
///
/// ```dfir
/// source_iter([("a", "x"), ("b", "y"), ("a", "z")])
///     -> enumerate_keyed()
///     -> fold::<'static>(::std::collections::BTreeMap::<&str, Vec<(usize, &str)>>::new, |map: &mut ::std::collections::BTreeMap<&str, Vec<(usize, &str)>>, (k, iv): (&str, (usize, &str))| {
///         map.entry(k).or_default().push(iv);
///     })
///     -> for_each(|map| {
///         assert_eq!(map.get("a").unwrap(), &vec![(0, "x"), (1, "z")]);
///         assert_eq!(map.get("b").unwrap(), &vec![(0, "y")]);
///     });
/// ```
///
/// `enumerate_keyed` can also be provided with one generic lifetime persistence argument, either
/// `'tick` or `'static`, to specify how counters persist. The default is `'tick`.
/// With `'tick`, per-key counters reset to zero at the start of each tick.
/// With `'static`, counters persist across ticks and continue incrementing monotonically.
pub const ENUMERATE_KEYED: OperatorConstraints = OperatorConstraints {
    name: "enumerate_keyed",
    categories: &[OperatorCategory::Map],
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

        let counterdata_ident = wc.make_ident("counterdata");

        let write_prologue = quote_spanned! {op_span=>
            let #counterdata_ident = #df_ident.add_state(::std::cell::RefCell::new(
                #root::rustc_hash::FxHashMap::<_, usize>::default()
            ));
        };
        let write_prologue_after = wc
            .persistence_as_state_lifespan(persistence)
            .map(|lifespan| quote_spanned! {op_span=>
                #df_ident.set_state_lifespan_hook(#counterdata_ident, #lifespan, |rcell| { rcell.take(); });
            }).unwrap_or_default();

        let map_fn = quote_spanned! {op_span=>
            |(k, v)| {
                let mut map = unsafe {
                    // SAFETY: handle from `#df_ident.add_state(..)`.
                    #context.state_ref_unchecked(#counterdata_ident)
                }.borrow_mut();

                let counter = map.entry(::std::clone::Clone::clone(&k))
                    .or_insert(0);
                let index = *counter;
                *counter += 1;
                (k, (index, v))
            }
        };
        let write_iterator = if is_pull {
            quote_spanned! {op_span=>
                let #ident = #root::futures::stream::StreamExt::map(#input, #map_fn);
            }
        } else {
            quote_spanned! {op_span=>
                let #ident = #root::sinktools::map(#map_fn, #output);
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
