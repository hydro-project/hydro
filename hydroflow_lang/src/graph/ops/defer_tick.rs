use super::{
    DelayType, OperatorCategory, OperatorConstraints, IDENTITY_WRITE_FN, RANGE_0,
    RANGE_1,
};

/// Buffers all input items and releases them in the next tick.
/// the state of the current tick. For example,
/// See the [book discussion of Hydroflow time](../concepts/life_and_times) for details on ticks.
/// A tick may be divided into multiple [strata](../concepts/stratification); see the [`next_stratum()`](#next_stratum)
/// operator.
///
/// `defer_tick` is sometimes needed to separate conflicting data across time,
/// in order to preserve invariants. Consider the following example, which implements
/// a flip-flop -- the invariant is that it emit one of true or false in a given tick
/// (but never both!)
///
/// ```rustbook
/// pub fn main() {
///     let mut df = hydroflow::hydroflow_syntax! {
///         source_iter(vec!(true))
///                 -> state;
///         state = union()
///                 -> assert(|x| if context.current_tick().0 % 2 == 0 { *x == true } else { *x == false })
///                 -> map(|x| !x)
///                 -> defer_tick()
///                 -> state;
///     };
///     for i in 1..100 {
///         println!("tick {}", i);
///         df.run_tick();
///     }
/// }
/// ```
///
/// `defer_tick` can also be handy for comparing stream content across ticks.
/// In the example below `defer_tick()` is used alongside `difference()` to
/// filter out any items that arrive from `inp` in the current tick which match
/// an item from `inp` in the previous
/// tick.
/// ```rustbook
/// // Outputs 1 2 3 4 5 6 (on separate lines).
/// let (input_send, input_recv) = hydroflow::util::unbounded_channel::<usize>();
/// let mut flow = hydroflow::hydroflow_syntax! {
///     inp = source_stream(input_recv) -> tee();
///     inp -> [pos]diff;
///     inp -> defer_tick() -> [neg]diff;
///     diff = difference() -> for_each(|x| println!("{}", x));
/// };
///
/// for x in [1, 2, 3, 4] {
///     input_send.send(x).unwrap();
/// }
/// flow.run_tick();
///
/// for x in [3, 4, 5, 6] {
///     input_send.send(x).unwrap();
/// }
/// flow.run_tick();
/// ```
///
/// You can also supply a type parameter `defer_tick::<MyType>()` to specify what items flow
/// through the the pipeline. This can be useful for helping the compiler infer types.
pub const DEFER_TICK: OperatorConstraints = OperatorConstraints {
    name: "defer_tick",
    categories: &[OperatorCategory::Control],
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 0,
    persistence_args: RANGE_0,
    type_args: &(0..=1),
    is_external_input: false,
    has_singleton_output: false,
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| Some(DelayType::Tick),
    flow_prop_fn: None,
    write_fn: IDENTITY_WRITE_FN,
};
