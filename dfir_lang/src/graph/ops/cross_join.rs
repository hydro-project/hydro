use quote::quote_spanned;
use syn::parse_quote;

use super::{OperatorCategory, OperatorConstraints, RANGE_1, WriteContextArgs};

/// > 2 input streams of type S and T, 1 output stream of type (S, T)
///
/// Forms the cross-join (Cartesian product) of the items in the input streams, returning all
/// tupled pairs.
///
/// ```dfir
/// source_iter(vec!["happy", "sad"]) -> [0]my_join;
/// source_iter(vec!["dog", "cat"]) -> [1]my_join;
/// my_join = cross_join() -> assert_eq([("happy", "dog"), ("sad", "dog"), ("happy", "cat"), ("sad", "cat")]);
/// ```
///
/// `cross_join` can be provided with one or two generic lifetime persistence arguments
/// in the same way as [`join`](#join), see [`join`'s documentation](#join) for more info.
///
/// ```rustbook
/// let (input_send, input_recv) = dfir_rs::util::unbounded_channel::<&str>();
/// let mut flow = dfir_rs::dfir_syntax! {
///     my_join = cross_join::<'tick>();
///     source_iter(["hello", "bye"]) -> [0]my_join;
///     source_stream(input_recv) -> [1]my_join;
///     my_join -> for_each(|(s, t)| println!("({}, {})", s, t));
/// };
/// input_send.send("oakland").unwrap();
/// flow.run_tick();
/// input_send.send("san francisco").unwrap();
/// flow.run_tick();
/// ```
/// Prints only `"(hello, oakland)"` and `"(bye, oakland)"`. The `source_iter` is only included in
/// the first tick, then forgotten, so when `"san francisco"` arrives on input `[1]` in the second tick,
/// there is nothing for it to match with from input `[0]`, and therefore it does appear in the output.
pub const CROSS_JOIN: OperatorConstraints = OperatorConstraints {
    name: "cross_join",
    categories: &[OperatorCategory::MultiIn],
    hard_range_inn: &(2..=2),
    soft_range_inn: &(2..=2),
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 0,
    persistence_args: &(0..=2),
    type_args: &(0..=1),
    is_external_input: false,
    has_singleton_output: false,
    flo_type: None,
    ports_inn: Some(|| super::PortListSpec::Fixed(parse_quote! { 0, 1 })),
    ports_out: None,
    input_delaytype_fn: |_| None,
    write_fn: |wc @ &WriteContextArgs {
                   root,
                   op_span,
                   ident,
                   inputs,
                   ..
               },
               diagnostics| {
        let mut output = (super::join::JOIN.write_fn)(wc, diagnostics)?;

        let lhs = &inputs[0];
        let rhs = &inputs[1];
        let write_iterator = output.write_iterator;
        output.write_iterator = quote_spanned!(op_span=>
            let #lhs = #root::futures::stream::StreamExt::map(#lhs, |a| ((), a));
            let #rhs = #root::futures::stream::StreamExt::map(#rhs, |b| ((), b));
            #write_iterator
            let #ident = #root::futures::stream::StreamExt::map(#ident, |((), (a, b))| (a, b));
        );

        Ok(output)
    },
};
