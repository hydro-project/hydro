use std::cell::RefCell;
use std::collections::HashSet;

use crate::ir::*;

fn persist_pullup_node(
    node: &mut HfPlusNode,
    persist_pulled_tees: &mut HashSet<*const RefCell<HfPlusNode>>,
) {
    *node = match std::mem::replace(node, HfPlusNode::Placeholder) {
        HfPlusNode::Unpersist(box HfPlusNode::Persist(box behind_persist)) => behind_persist,

        HfPlusNode::Delta(box HfPlusNode::Persist(box behind_persist)) => behind_persist,

        HfPlusNode::Tee { inner } => {
            if persist_pulled_tees.contains(&(inner.0.as_ref() as *const RefCell<HfPlusNode>)) {
                HfPlusNode::Persist(Box::new(HfPlusNode::Tee {
                    inner: TeeNode(inner.0.clone()),
                }))
            } else if matches!(*inner.0.borrow(), HfPlusNode::Persist(_)) {
                persist_pulled_tees.insert(inner.0.as_ref() as *const RefCell<HfPlusNode>);
                if let HfPlusNode::Persist(box behind_persist) =
                    inner.0.replace(HfPlusNode::Placeholder)
                {
                    *inner.0.borrow_mut() = behind_persist;
                } else {
                    unreachable!()
                }

                HfPlusNode::Persist(Box::new(HfPlusNode::Tee {
                    inner: TeeNode(inner.0.clone()),
                }))
            } else {
                HfPlusNode::Tee { inner }
            }
        }

        HfPlusNode::Map {
            f,
            input: box HfPlusNode::Persist(behind_persist),
        } => HfPlusNode::Persist(Box::new(HfPlusNode::Map {
            f,
            input: behind_persist,
        })),

        HfPlusNode::FlatMap {
            f,
            input: box HfPlusNode::Persist(behind_persist),
        } => HfPlusNode::Persist(Box::new(HfPlusNode::FlatMap {
            f,
            input: behind_persist,
        })),

        HfPlusNode::Filter {
            f,
            input: box HfPlusNode::Persist(behind_persist),
        } => HfPlusNode::Persist(Box::new(HfPlusNode::Filter {
            f,
            input: behind_persist,
        })),

        HfPlusNode::Network {
            from_location,
            from_key,
            to_location,
            to_key,
            serialize_pipeline,
            instantiate_fn,
            deserialize_pipeline,
            input: box HfPlusNode::Persist(behind_persist),
            ..
        } => HfPlusNode::Persist(Box::new(HfPlusNode::Network {
            from_location,
            from_key,
            to_location,
            to_key,
            serialize_pipeline,
            instantiate_fn,
            deserialize_pipeline,
            input: behind_persist,
        })),

        HfPlusNode::Union(box HfPlusNode::Persist(left), box HfPlusNode::Persist(right)) => {
            HfPlusNode::Persist(Box::new(HfPlusNode::Union(left, right)))
        }

        HfPlusNode::CrossProduct(box HfPlusNode::Persist(left), box HfPlusNode::Persist(right)) => {
            HfPlusNode::Persist(Box::new(HfPlusNode::Delta(Box::new(
                HfPlusNode::CrossProduct(
                    Box::new(HfPlusNode::Persist(left)),
                    Box::new(HfPlusNode::Persist(right)),
                ),
            ))))
        }

        HfPlusNode::Join(box HfPlusNode::Persist(left), box HfPlusNode::Persist(right)) => {
            HfPlusNode::Persist(Box::new(HfPlusNode::Delta(Box::new(HfPlusNode::Join(
                Box::new(HfPlusNode::Persist(left)),
                Box::new(HfPlusNode::Persist(right)),
            )))))
        }

        HfPlusNode::Unique(box HfPlusNode::Persist(inner)) => {
            HfPlusNode::Persist(Box::new(HfPlusNode::Delta(Box::new(HfPlusNode::Unique(
                Box::new(HfPlusNode::Persist(inner)),
            )))))
        }

        node => node,
    };
}

pub fn persist_pullup(ir: Vec<HfPlusLeaf>) -> Vec<HfPlusLeaf> {
    let mut seen_tees = Default::default();
    let mut persist_pulled_tees = Default::default();
    ir.into_iter()
        .map(|l| {
            l.transform_children(
                |n, s| n.transform_bottom_up(persist_pullup_node, s, &mut persist_pulled_tees),
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
    fn persist_pullup_through_map() {
        let flow = crate::builder::FlowBuilder::new();
        let process = flow.process::<()>();

        process
            .source_iter(q!(0..10))
            .map(q!(|v| v + 1))
            .for_each(q!(|n| println!("{}", n)));

        let built = flow.finalize();

        insta::assert_debug_snapshot!(built.ir());

        let optimized = built.optimize_with(super::persist_pullup);

        insta::assert_debug_snapshot!(optimized.ir());
        for (id, graph) in optimized.compile_no_network::<MultiGraph>().hydroflow_ir() {
            insta::with_settings!({snapshot_suffix => format!("surface_graph_{id}")}, {
                insta::assert_snapshot!(graph.surface_syntax_string());
            });
        }
    }

    #[test]
    fn persist_pullup_behind_tee() {
        let flow = crate::builder::FlowBuilder::new();
        let process = flow.process::<()>();

        let before_tee = process.source_iter(q!(0..10)).tick_batch().persist();

        before_tee
            .clone()
            .map(q!(|v| v + 1))
            .all_ticks()
            .for_each(q!(|n| println!("{}", n)));

        before_tee
            .clone()
            .map(q!(|v| v + 1))
            .all_ticks()
            .for_each(q!(|n| println!("{}", n)));

        let built = flow.finalize();

        insta::assert_debug_snapshot!(built.ir());

        let optimized = built.optimize_with(super::persist_pullup);

        insta::assert_debug_snapshot!(optimized.ir());

        for (id, graph) in optimized.compile_no_network::<MultiGraph>().hydroflow_ir() {
            insta::with_settings!({snapshot_suffix => format!("surface_graph_{id}")}, {
                insta::assert_snapshot!(graph.surface_syntax_string());
            });
        }
    }
}
