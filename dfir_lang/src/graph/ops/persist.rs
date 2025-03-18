use quote::quote_spanned;

use super::{
    OpInstGenerics, OperatorCategory, OperatorConstraints, OperatorInstance, OperatorWriteOutput,
    Persistence, RANGE_0, RANGE_1, WriteContextArgs,
};
use crate::diagnostic::{Diagnostic, Level};

/// Stores each item as it passes through, and replays all item every tick.
///
/// ```dfir
/// // Normally `source_iter(...)` only emits once, but `persist::<'static>()` will replay the `"hello"`
/// // on every tick.
/// source_iter(["hello"])
///     -> persist::<'static>()
///     -> assert_eq(["hello"]);
/// ```
///
/// `persist()` can be used to introduce statefulness into stateless pipelines. In the example below, the
/// join only stores data for single tick. The `persist::<'static>()` operator introduces statefulness
/// across ticks. This can be useful for optimization transformations within the dfir
/// compiler. Equivalently, we could specify that the join has `static` persistence (`my_join = join::<'static>()`).
/// ```rustbook
/// let (input_send, input_recv) = dfir_rs::util::unbounded_channel::<(&str, &str)>();
/// let mut flow = dfir_rs::dfir_syntax! {
///     source_iter([("hello", "world")]) -> persist::<'static>() -> [0]my_join;
///     source_stream(input_recv) -> persist::<'static>() -> [1]my_join;
///     my_join = join::<'tick>() -> for_each(|(k, (v1, v2))| println!("({}, ({}, {}))", k, v1, v2));
/// };
/// input_send.send(("hello", "oakland")).unwrap();
/// flow.run_tick();
/// input_send.send(("hello", "san francisco")).unwrap();
/// flow.run_tick();
/// // (hello, (world, oakland))
/// // (hello, (world, oakland))
/// // (hello, (world, san francisco))
/// ```
pub const PERSIST: OperatorConstraints = OperatorConstraints {
    name: "persist",
    categories: &[OperatorCategory::Persistence],
    hard_range_inn: RANGE_1,
    soft_range_inn: RANGE_1,
    hard_range_out: RANGE_1,
    soft_range_out: RANGE_1,
    num_args: 0,
    persistence_args: RANGE_1,
    type_args: RANGE_0,
    is_external_input: false,
    has_singleton_output: true,
    flo_type: None,
    ports_inn: None,
    ports_out: None,
    input_delaytype_fn: |_| None,
    write_fn: |wc @ &WriteContextArgs {
                   root,
                   context,
                   df_ident,
                   op_span,
                   ident,
                   is_pull,
                   inputs,
                   outputs,
                   singleton_output_ident,
                   op_name,
                   work_fn,
                   op_inst:
                       OperatorInstance {
                           generics:
                               OpInstGenerics {
                                   persistence_args, ..
                               },
                           ..
                       },
                   ..
               },
               diagnostics| {
        if [Persistence::Static] != persistence_args[..] {
            diagnostics.push(Diagnostic::spanned(
                op_span,
                Level::Error,
                format!("{} only supports `'static`.", op_name),
            ));
        }

        let persistdata_ident = singleton_output_ident;
        let vec_ident = wc.make_ident("persistvec");
        let write_prologue = quote_spanned! {op_span=>
            let #persistdata_ident = #df_ident.add_state(::std::cell::RefCell::new(
                ::std::vec::Vec::new(),
            ));
        };

        let write_iterator = if is_pull {
            let input = &inputs[0];
            quote_spanned! {op_span=>
                let mut #vec_ident = unsafe {
                    // SAFETY: handle from `#df_ident.add_state(..)`.
                    #context.state_ref_unchecked(#persistdata_ident)
                }.borrow_mut();

                let #ident = {
                    if #context.is_first_run_this_tick() {
                        #work_fn(|| #vec_ident.extend(#input));
                        #vec_ident.iter().cloned()
                    } else {
                        let len = #vec_ident.len();
                        #work_fn(|| #vec_ident.extend(#input));
                        #vec_ident[len..].iter().cloned()
                    }
                };
            }
        } else {
            let output = &outputs[0];
            quote_spanned! {op_span=>
                let mut #vec_ident = unsafe {
                    // SAFETY: handle from `#df_ident.add_state(..)`.
                    #context.state_ref_unchecked(#persistdata_ident)
                }.borrow_mut();

                let #ident = {
                    fn constrain_types<'ctx, Push, Item>(vec: &'ctx mut Vec<Item>, mut output: Push, is_new_tick: bool) -> impl 'ctx + #root::pusherator::Pusherator<Item = Item>
                    where
                        Push: 'ctx + #root::pusherator::Pusherator<Item = Item>,
                        Item: ::std::clone::Clone,
                    {
                        if is_new_tick {
                            #work_fn(|| vec.iter().cloned().for_each(|item| {
                                #root::pusherator::Pusherator::give(&mut output, item);
                            }));
                        }
                        #root::pusherator::map::Map::new(|item| {
                            vec.push(item);
                            vec.last().unwrap().clone()
                        }, output)
                    }
                    constrain_types(&mut *#vec_ident, #output, #context.is_first_run_this_tick())
                };
            }
        };

        let write_iterator_after = quote_spanned! {op_span=>
            #context.schedule_subgraph(#context.current_subgraph(), false);
        };

        Ok(OperatorWriteOutput {
            write_prologue,
            write_iterator,
            write_iterator_after,
        })
    },
};
