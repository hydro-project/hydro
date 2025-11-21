use quote::quote_spanned;

use super::{DelayType, OperatorCategory, OperatorConstraints, RANGE_1, WriteContextArgs};

/// > 1 input stream of type `(K, V)`, 1 output stream of type `(K, V)`.
/// > The output will have one tuple for each distinct `K`, with an accumulated (reduced) value of
/// > type `V`.
///
/// If you need the accumulated value to have a different type than the input, use [`fold_keyed`](#fold_keyed).
///
/// > Arguments: one Rust closures. The closure takes two arguments: an `&mut` 'accumulator', and
/// > an element. Accumulator should be updated based on the element.
///
/// A special case of `reduce`, in the spirit of SQL's GROUP BY and aggregation constructs. The input
/// is partitioned into groups by the first field, and for each group the values in the second
/// field are accumulated via the closures in the arguments.
///
/// > Note: The closures have access to the [`context` object](surface_flows.mdx#the-context-object).
///
/// `reduce_keyed` can also be provided with one generic lifetime persistence argument, either
/// `'tick` or `'static`, to specify how data persists. With `'tick`, values will only be collected
/// within the same tick. With `'static`, values will be remembered across ticks and will be
/// aggregated with pairs arriving in later ticks. When not explicitly specified persistence
/// defaults to `'tick`.
///
/// `reduce_keyed` can also be provided with two type arguments, the key and value type. This is
/// required when using `'static` persistence if the compiler cannot infer the types.
///
/// ```dfir
/// source_iter([("toy", 1), ("toy", 2), ("shoe", 11), ("shoe", 35), ("haberdashery", 7)])
///     -> reduce_keyed(|old: &mut u32, val: u32| *old += val)
///     -> assert_eq([("toy", 3), ("shoe", 46), ("haberdashery", 7)]);
/// ```
///
/// Example using `'tick` persistence and type arguments:
/// ```rustbook
/// let (input_send, input_recv) = dfir_rs::util::unbounded_channel::<(&str, &str)>();
/// let mut flow = dfir_rs::dfir_syntax! {
///     source_stream(input_recv)
///         -> reduce_keyed::<'tick, &str>(|old: &mut _, val| *old = std::cmp::max(*old, val))
///         -> for_each(|(k, v)| println!("({:?}, {:?})", k, v));
/// };
///
/// input_send.send(("hello", "oakland")).unwrap();
/// input_send.send(("hello", "berkeley")).unwrap();
/// input_send.send(("hello", "san francisco")).unwrap();
/// flow.run_available();
/// // ("hello", "oakland, berkeley, san francisco, ")
///
/// input_send.send(("hello", "palo alto")).unwrap();
/// flow.run_available();
/// // ("hello", "palo alto, ")
/// ```
pub const REDUCE_KEYED: OperatorConstraints = OperatorConstraints {
    name: "reduce_keyed",
    categories: &[OperatorCategory::KeyedFold],
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 1,
    persistence_args: &(0..=1),
    type_args: &(0..=2),
    is_external_input: false,
    has_singleton_output: true,
    flo_type: None,
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| Some(DelayType::Stratum),
    write_fn: |wc @ &WriteContextArgs { root, op_span, .. }, _| {
        super::fold_keyed::accum_keyed_codegen(
            wc,
            quote_spanned! {op_span=>
                #root::compiled::pull::ReduceKeyedThen
            },
        )
    },
};
