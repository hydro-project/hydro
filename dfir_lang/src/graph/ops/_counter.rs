use quote::quote_spanned;
use syn::parse_quote_spanned;

use super::{
    OperatorCategory, OperatorConstraints, OperatorWriteOutput, RANGE_0, RANGE_1, WriteContextArgs,
};

/// > Arguments: A `tag` string and a `Duration` for how long to wait between printing. A third optional
/// > parameter controls the prefix used for logging (defaults to "_counter").
///
/// Counts the number of items passing through and prints to stdout whenever the stream trigger activates.
///
/// ```dfir
/// source_stream(dfir_rs::util::iter_batches_stream(0..=100_000, 1))
///     -> _counter("nums", std::time::Duration::from_millis(100));
/// ```
/// stdout:
/// ```text
/// _counter(nums): 1
/// _counter(nums): 6202
/// _counter(nums): 12540
/// _counter(nums): 18876
/// _counter(nums): 25218
/// _counter(nums): 31557
/// _counter(nums): 37893
/// _counter(nums): 44220
/// _counter(nums): 50576
/// _counter(nums): 56909
/// _counter(nums): 63181
/// _counter(nums): 69549
/// _counter(nums): 75914
/// _counter(nums): 82263
/// _counter(nums): 88638
/// _counter(nums): 94980
/// ```
pub const _COUNTER: OperatorConstraints = OperatorConstraints {
    name: "_counter",
    categories: &[OperatorCategory::Map],
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
    hard_range_out: &(0..=1),
    soft_range_out: &(0..=1),
    num_args: 2,
    persistence_args: RANGE_0,
    type_args: RANGE_0,
    is_external_input: false,
    has_singleton_output: false,
    flo_type: None,
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| None,
    write_fn: |wc @ &WriteContextArgs {
                   root,
                   df_ident,
                   op_span,
                   ident,
                   inputs,
                   outputs,
                   is_pull,
                   arguments,
                   ..
               },
               _| {
        let read_ident = wc.make_ident("read");
        let write_ident = wc.make_ident("write");

        let tag_expr = &arguments[0];
        let tag_ident = wc.make_ident("tag");
        let duration_expr = &arguments[1];
        let duration_ident = wc.make_ident("duration");
        let prefix = arguments.get(2).cloned().unwrap_or(parse_quote_spanned!(op_span=> "_counter"));

        let write_prologue = quote_spanned! {op_span=>
            let #write_ident = ::std::rc::Rc::new(::std::cell::Cell::new(0_u64));

            let #read_ident = ::std::rc::Rc::clone(&#write_ident);
            let #duration_ident = #duration_expr;
            let #tag_ident = #tag_expr;
            #df_ident.request_task(async move {
                loop {
                    println!("{}({}): {}", #prefix, #tag_ident, #read_ident.get());
                    #root::tokio::time::sleep(#duration_ident).await;
                }
            });
        };

        let count_ident = wc.make_ident("count");
        let write_iterator = if is_pull {
            let input = &inputs[0];
            quote_spanned! {op_span=>
                let #ident = #input.inspect(|_| { #count_ident += 1; });
            }
        } else if outputs.is_empty() {
            quote_spanned! {op_span=>
                let #ident = #root::pusherator::inspect::Inspect::new(|_| { #count_ident += 1; }, #root::pusherator::null::Null::new());
            }
        } else {
            let output = &outputs[0];
            quote_spanned! {op_span=>
                let #ident = #root::pusherator::inspect::Inspect::new(|_| { #count_ident += 1; }, #output);
            }
        };
        let write_iterator = quote_spanned! {op_span=>
            let mut #count_ident = 0;
            #write_iterator
        };

        let write_iterator_after = quote_spanned! {op_span=>
            #write_ident.set(#write_ident.get() + #count_ident);
        };

        Ok(OperatorWriteOutput {
            write_prologue,
            write_iterator,
            write_iterator_after,
            ..Default::default()
        })
    },
};
