use std::cell::RefCell;
use std::collections::HashSet;

use crate::ir::*;

fn persist_pullup_node(
    node: &mut HydroNode,
    persist_pulled_tees: &mut HashSet<*const RefCell<HydroNode>>,
) {
    *node = match_box::match_box! {
        match std::mem::replace(node, HydroNode::Placeholder) {
            HydroNode::Unpersist(mb!(* HydroNode::Persist(mb!(* behind_persist)))) => behind_persist,

            HydroNode::Delta(mb!(* HydroNode::Persist(mb!(* behind_persist)))) => behind_persist,

            HydroNode::Tee { inner } => {
                if persist_pulled_tees.contains(&(inner.0.as_ref() as *const RefCell<HydroNode>)) {
                    HydroNode::Persist(Box::new(HydroNode::Tee {
                        inner: TeeNode(inner.0.clone()),
                    }))
                } else if matches!(*inner.0.borrow(), HydroNode::Persist(_)) {
                    persist_pulled_tees.insert(inner.0.as_ref() as *const RefCell<HydroNode>);
                    if let HydroNode::Persist(behind_persist) =
                        inner.0.replace(HydroNode::Placeholder)
                    {
                        *inner.0.borrow_mut() = *behind_persist;
                    } else {
                        unreachable!()
                    }

                    HydroNode::Persist(Box::new(HydroNode::Tee {
                        inner: TeeNode(inner.0.clone()),
                    }))
                } else {
                    HydroNode::Tee { inner }
                }
            }

            HydroNode::Map {
                f,
                input: mb!(* HydroNode::Persist(behind_persist)),
            } => HydroNode::Persist(Box::new(HydroNode::Map {
                f,
                input: behind_persist,
            })),

            HydroNode::FilterMap {
                f,
                input: mb!(* HydroNode::Persist(behind_persist)),
            } => HydroNode::Persist(Box::new(HydroNode::FilterMap {
                f,
                input: behind_persist,
            })),

            HydroNode::FlatMap {
                f,
                input: mb!(* HydroNode::Persist(behind_persist)),
            } => HydroNode::Persist(Box::new(HydroNode::FlatMap {
                f,
                input: behind_persist,
            })),

            HydroNode::Filter {
                f,
                input: mb!(* HydroNode::Persist(behind_persist)),
            } => HydroNode::Persist(Box::new(HydroNode::Filter {
                f,
                input: behind_persist,
            })),

            HydroNode::Network {
                from_location,
                from_key,
                to_location,
                to_key,
                serialize_fn,
                instantiate_fn,
                deserialize_fn,
                input: mb!(* HydroNode::Persist(behind_persist)),
                ..
            } => HydroNode::Persist(Box::new(HydroNode::Network {
                from_location,
                from_key,
                to_location,
                to_key,
                serialize_fn,
                instantiate_fn,
                deserialize_fn,
                input: behind_persist,
            })),

            HydroNode::Chain(mb!(* HydroNode::Persist(left)), mb!(* HydroNode::Persist(right))) => {
                HydroNode::Persist(Box::new(HydroNode::Chain(left, right)))
            }

            HydroNode::CrossProduct(mb!(* HydroNode::Persist(left)), mb!(* HydroNode::Persist(right))) => {
                HydroNode::Persist(Box::new(HydroNode::Delta(Box::new(
                    HydroNode::CrossProduct(
                        Box::new(HydroNode::Persist(left)),
                        Box::new(HydroNode::Persist(right)),
                    ),
                ))))
            }

            HydroNode::Join(mb!(* HydroNode::Persist(left)), mb!(* HydroNode::Persist(right))) => {
                HydroNode::Persist(Box::new(HydroNode::Delta(Box::new(HydroNode::Join(
                    Box::new(HydroNode::Persist(left)),
                    Box::new(HydroNode::Persist(right)),
                )))))
            }

            HydroNode::Unique(mb!(* HydroNode::Persist(inner))) => {
                HydroNode::Persist(Box::new(HydroNode::Delta(Box::new(HydroNode::Unique(
                    Box::new(HydroNode::Persist(inner)),
                )))))
            }

            node => node,
        }
    };
}

pub fn persist_pullup(ir: Vec<HydroLeaf>) -> Vec<HydroLeaf> {
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

        let tick = process.tick();
        let before_tee = unsafe {
            process
                .source_iter(q!(0..10))
                .timestamped(&tick)
                .tick_batch()
                .persist()
        };

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
