use std::cell::RefCell;

use hydroflow::futures::channel::mpsc::UnboundedSender;
use stageleft::*;

use crate::ir::*;
use crate::{profiler as myself, RuntimeContext};

pub fn increment_counter(count: &mut u64) {
    *count += 1;
}

fn quoted_any_fn<'a, F: Fn(&usize) + 'a, Q: IntoQuotedMut<'a, F>>(q: Q) -> Q {
    q
}

/// Add a profiling node before each node to count the cardinality of its input
fn add_profiling_node<'a>(
    node: &mut HfPlusNode,
    _context: RuntimeContext<'a>,
    counters: RuntimeData<&'a RefCell<Vec<u64>>>,
    counter_queue: RuntimeData<&'a RefCell<UnboundedSender<(usize, u64)>>>,
    id: &mut u32,
    seen_tees: &mut SeenTees,
) {
    let my_id = *id;
    *id += 1;

    node.transform_children(
        |node, seen_tees| {
            add_profiling_node(node, _context, counters, counter_queue, id, seen_tees)
        },
        seen_tees,
    );
    let orig_node = std::mem::replace(node, HfPlusNode::Placeholder);
    *node = HfPlusNode::Inspect {
        f: quoted_any_fn(q!({
            // Put counters on queue
            counter_queue
                .borrow()
                .unbounded_send((my_id as usize, counters.borrow()[my_id as usize]))
                .unwrap();
            counters.borrow_mut()[my_id as usize] = 0;
            move |_| {
                myself::increment_counter(&mut counters.borrow_mut()[my_id as usize]);
            }
        }))
        .splice_untyped()
        .into(),
        input: Box::new(orig_node),
    }
}

/// Count the cardinality of each input and periodically output to a file
pub fn profiling<'a>(
    ir: Vec<HfPlusLeaf>,
    context: RuntimeContext<'a>,
    counters: RuntimeData<&'a RefCell<Vec<u64>>>,
    counter_queue: RuntimeData<&'a RefCell<UnboundedSender<(usize, u64)>>>,
) -> Vec<HfPlusLeaf> {
    let mut id = 0;
    let mut seen_tees = Default::default();
    ir.into_iter()
        .map(|l| {
            l.transform_children(
                |node, seen_tees| {
                    add_profiling_node(node, context, counters, counter_queue, &mut id, seen_tees)
                },
                &mut seen_tees,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use stageleft::*;

    use crate::deploy::MultiGraph;
    use crate::location::Location;

    #[test]
    fn profiler_wrapping_all_operators() {
        let flow = crate::builder::FlowBuilder::new();
        let process = flow.process::<()>();

        process
            .source_iter(q!(0..10))
            .map(q!(|v| v + 1))
            .for_each(q!(|n| println!("{}", n)));

        let runtime_context = flow.runtime_context();
        let built = flow.finalize();

        insta::assert_debug_snapshot!(&built.ir());

        // Print mermaid
        // let mut mermaid_config = WriteConfig {op_text_no_imports: true, ..Default::default()};
        // for (_, ir) in built.clone().with_default_optimize().compile().hydroflow_ir() {
        //     println!("{}", ir.to_mermaid(&mermaid_config));
        // }

        let counters = RuntimeData::new("Fake");
        let counter_queue = RuntimeData::new("Fake");

        let pushed_down = built
            .with_default_optimize()
            .optimize_with(|ir| super::profiling(ir, runtime_context, counters, counter_queue));

        insta::assert_debug_snapshot!(&pushed_down.ir());

        let _ = pushed_down.compile_no_network::<MultiGraph>();
    }
}
