use std::cell::RefCell;

use dfir_rs::futures::channel::mpsc::UnboundedSender;
use stageleft::*;

use super::profiler as myself; // TODO(shadaj): stageleft does not support `self::...`
use crate::ir::*;

pub fn increment_counter(count: &mut u64) {
    *count += 1;
}

fn quoted_any_fn<'a, F: Fn(&usize) + 'a, Q: IntoQuotedMut<'a, F, ()>>(q: Q) -> Q {
    q
}

/// Add a profiling node before each node to count the cardinality of its input
fn add_profiling_node<'a>(
    node: &mut HydroNode,
    counters: RuntimeData<&'a RefCell<Vec<u64>>>,
    counter_queue: RuntimeData<&'a RefCell<UnboundedSender<(usize, u64)>>>,
    my_id: usize,
) {
    let orig_node = std::mem::replace(node, HydroNode::Placeholder);
    let new_metadata = orig_node.metadata().clone();
    *node = HydroNode::Inspect {
        f: quoted_any_fn(q!({
            // Put counters on queue
            counter_queue
                .borrow()
                .unbounded_send((my_id, counters.borrow()[my_id]))
                .unwrap();
            counters.borrow_mut()[my_id] = 0;
            move |_| {
                myself::increment_counter(&mut counters.borrow_mut()[my_id]);
            }
        }))
        .splice_untyped()
        .into(),
        input: Box::new(orig_node),
        metadata: new_metadata,
    }
}

/// Count the cardinality of each input and periodically output to a file
#[cfg(feature = "build")]
#[stageleft::runtime]
pub fn profiling<'a>(
    ir: &mut Vec<HydroLeaf>,
    counters: RuntimeData<&'a RefCell<Vec<u64>>>,
    counter_queue: RuntimeData<&'a RefCell<UnboundedSender<(usize, u64)>>>,
) {
    traverse_dfir(ir, |_, _| (),
    |node, next_stmt_id| add_profiling_node(node, counters, counter_queue,next_stmt_id));
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

        let built = flow.finalize();

        insta::assert_debug_snapshot!(&built.ir());

        // Print mermaid
        // let mut mermaid_config = WriteConfig {op_text_no_imports: true, ..Default::default()};
        // for (_, ir) in built.clone().with_default_optimize().compile().dfir() {
        //     println!("{}", ir.to_mermaid(&mermaid_config));
        // }

        let counters = RuntimeData::new("Fake");
        let counter_queue = RuntimeData::new("Fake");

        let pushed_down = built
            .optimize_with(crate::rewrites::persist_pullup::persist_pullup)
            .optimize_with(|ir| super::profiling(ir, counters, counter_queue));

        insta::assert_debug_snapshot!(&pushed_down.ir());

        let _ = pushed_down.compile_no_network::<MultiGraph>();
    }
}
