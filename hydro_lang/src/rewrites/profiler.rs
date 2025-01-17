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
    id: &mut u32,
    seen_tees: &mut SeenTees,
) {
    let my_id = *id;
    *id += 1;

    node.transform_children(
        |node, seen_tees| add_profiling_node(node, counters, counter_queue, id, seen_tees),
        seen_tees,
    );
    let orig_node = std::mem::replace(node, HydroNode::Placeholder);
    let new_metadata = HydroNodeMetadata {
        id: None,
        location_kind: orig_node.metadata().location_kind.clone(),
        output_type: orig_node.metadata().output_type.clone(),
    };
    *node = HydroNode::Inspect {
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
        metadata: new_metadata,
    }
}

/// Count the cardinality of each input and periodically output to a file
pub fn profiling<'a>(
    ir: Vec<HydroLeaf>,
    counters: RuntimeData<&'a RefCell<Vec<u64>>>,
    counter_queue: RuntimeData<&'a RefCell<UnboundedSender<(usize, u64)>>>,
) -> Vec<HydroLeaf> {
    let mut id = 0;
    let mut seen_tees = Default::default();
    ir.into_iter()
        .map(|l| {
            l.transform_children(
                |node, seen_tees| {
                    add_profiling_node(node, counters, counter_queue, &mut id, seen_tees)
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
            .optimize_with(crate::rewrites::persist_pullup::persist_pullup)
            .optimize_with(|ir| super::profiling(ir, counters, counter_queue));

        insta::assert_debug_snapshot!(&pushed_down.ir());

        let _ = pushed_down.compile_no_network::<MultiGraph>();
    }
}
