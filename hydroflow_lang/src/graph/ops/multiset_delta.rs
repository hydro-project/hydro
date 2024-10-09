use quote::quote_spanned;

use super::{
    OperatorCategory, OperatorConstraints, OperatorWriteOutput, WriteContextArgs,
    RANGE_0, RANGE_1,
};

// TODO(mingwei): more doc
/// Multiset delta from the previous tick.
///
/// ```rustbook
/// let (input_send, input_recv) = hydroflow::util::unbounded_channel::<u32>();
/// let mut flow = hydroflow::hydroflow_syntax! {
///     source_stream(input_recv)
///         -> multiset_delta()
///         -> for_each(|n| println!("{}", n));
/// };
///
/// input_send.send(3).unwrap();
/// input_send.send(4).unwrap();
/// input_send.send(3).unwrap();
/// flow.run_tick();
/// // 3, 4,
///
/// input_send.send(3).unwrap();
/// input_send.send(5).unwrap();
/// input_send.send(3).unwrap();
/// input_send.send(3).unwrap();
/// flow.run_tick();
/// // 5, 3
/// // First two "3"s are removed due to previous tick.
/// ```
pub const MULTISET_DELTA: OperatorConstraints = OperatorConstraints {
    name: "multiset_delta",
    categories: &[OperatorCategory::Persistence],
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 0,
    persistence_args: RANGE_0,
    type_args: RANGE_0,
    is_external_input: false,
    // If this is set to true, the state will need to be cleared using `#context.set_state_tick_hook`
    // to prevent reading uncleared data if this subgraph doesn't run.
    // https://github.com/hydro-project/hydroflow/issues/1298
    // If `'tick` lifetimes are added.
    has_singleton_output: false,
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| None,
    write_fn: |wc @ &WriteContextArgs {
                   root,
                   op_span,
                   context,
                   hydroflow,
                   ident,
                   inputs,
                   outputs,
                   is_pull,
                   ..
               },
               _| {
        let input = &inputs[0];
        let output = &outputs[0];

        let prev_data = wc.make_ident("prev_data");
        let curr_data = wc.make_ident("curr_data");

        let write_prologue = quote_spanned! {op_span=>
            let #prev_data = #hydroflow.add_state(::std::cell::RefCell::new(#root::rustc_hash::FxHashMap::default()));
            let #curr_data = #hydroflow.add_state(::std::cell::RefCell::new(#root::rustc_hash::FxHashMap::default()));
        };

        let tick_swap = quote_spanned! {op_span=>
            {
                if context.is_first_run_this_tick() {
                    let mut prev_map = #context.state_ref(#prev_data).borrow_mut();
                    let mut curr_map = #context.state_ref(#curr_data).borrow_mut();
                    ::std::mem::swap(::std::ops::DerefMut::deref_mut(&mut prev_map), ::std::ops::DerefMut::deref_mut(&mut curr_map));
                    curr_map.clear();
                }
            }
        };

        let filter_fn = quote_spanned! {op_span=>
            |item| {
                let mut prev_map = #context.state_ref(#prev_data).borrow_mut();
                let mut curr_map = #context.state_ref(#curr_data).borrow_mut();

                *curr_map.entry(#[allow(clippy::clone_on_copy)] item.clone()).or_insert(0_usize) += 1;
                if let Some(old_count) = prev_map.get_mut(item) {
                    #[allow(clippy::absurd_extreme_comparisons)] // Usize cannot be less than zero.
                    if *old_count <= 0 {
                        true
                    } else {
                        *old_count -= 1;
                        false
                    }
                } else {
                    true
                }
            }
        };
        let write_iterator = if is_pull {
            quote_spanned! {op_span=>
                #tick_swap
                let #ident = #input.filter(#filter_fn);
            }
        } else {
            quote_spanned! {op_span=>
                #tick_swap
                let #ident = #root::pusherator::filter::Filter::new(#filter_fn, #output);
            }
        };

        Ok(OperatorWriteOutput {
            write_prologue,
            write_iterator,
            ..Default::default()
        })
    },
};
