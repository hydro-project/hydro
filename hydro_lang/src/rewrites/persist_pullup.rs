use std::cell::RefCell;
use std::collections::HashSet;

use crate::ir::*;

fn persist_pullup_node(
    node: &mut HydroNode,
    persist_pulled_tees: &mut HashSet<*const RefCell<HydroNode>>,
) {
    *node = match_box::match_box! {
        match std::mem::replace(node, HydroNode::Placeholder) {
            HydroNode::Unpersist { inner: mb!(* HydroNode::Persist { inner: mb!(* behind_persist), .. }), .. } => behind_persist,

            HydroNode::Delta { inner: mb!(* HydroNode::Persist { inner: mb!(* behind_persist), .. }), .. } => behind_persist,

            // TODO: Figure out if persist needs to copy its metadata or can just use original metadata here. If it can just use original, figure out where that is
            HydroNode::Tee { inner, metadata } => {
                if persist_pulled_tees.contains(&(inner.0.as_ref() as *const RefCell<HydroNode>)) {
                    HydroNode::Persist {
                        inner: Box::new(HydroNode::Tee {
                            inner: TeeNode(inner.0.clone()),
                            metadata: metadata.clone(),
                        }),
                        metadata: metadata.clone(),
                    }
                } else if matches!(*inner.0.borrow(), HydroNode::Persist { .. }) {
                    persist_pulled_tees.insert(inner.0.as_ref() as *const RefCell<HydroNode>);
                    if let HydroNode::Persist { inner: behind_persist, .. } =
                        inner.0.replace(HydroNode::Placeholder)
                    {
                        *inner.0.borrow_mut() = *behind_persist;
                    } else {
                        unreachable!()
                    }

                    HydroNode::Persist {
                        inner: Box::new(HydroNode::Tee {
                            inner: TeeNode(inner.0.clone()),
                            metadata: metadata.clone(),
                        }),
                        metadata: metadata.clone(),
                    }
                } else {
                    HydroNode::Tee { inner, metadata }
                }
            }

            HydroNode::ResolveFutures {
                input: mb!(* HydroNode::Persist { inner: behind_persist, .. }),
                metadata,
            } => HydroNode::Persist {
                inner: Box::new(HydroNode::ResolveFutures {
                    input: behind_persist,
                    metadata: metadata.clone(),
                }),
                metadata: metadata.clone(),
            },

            HydroNode::ResolveFuturesOrdered {
                input: mb!(* HydroNode::Persist { inner: behind_persist, .. }),
                metadata,
            } => HydroNode::Persist {
                inner: Box::new(HydroNode::ResolveFuturesOrdered {
                    input: behind_persist,
                    metadata: metadata.clone(),
                }),
                metadata: metadata.clone(),
            },

            HydroNode::Map {
                f,
                input: mb!(* HydroNode::Persist { inner: behind_persist, .. }),
                metadata,
            } => HydroNode::Persist {
                inner: Box::new(HydroNode::Map {
                    f,
                    input: behind_persist,
                    metadata: metadata.clone(),
                }),
                metadata: metadata.clone(),
            },

            HydroNode::FilterMap {
                f,
                input: mb!(* HydroNode::Persist { inner: behind_persist, .. }),
                metadata,
            } => HydroNode::Persist {
                inner: Box::new(HydroNode::FilterMap {
                    f,
                    input: behind_persist,
                    metadata: metadata.clone(),
                }),
                metadata: metadata.clone()
            },

            HydroNode::FlatMap {
                f,
                input: mb!(* HydroNode::Persist { inner: behind_persist, .. }),
                metadata,
            } => HydroNode::Persist {
                inner: Box::new(HydroNode::FlatMap {
                    f,
                    input: behind_persist,
                    metadata: metadata.clone(),
                }),
                metadata: metadata.clone()
            },

            HydroNode::Filter {
                f,
                input: mb!(* HydroNode::Persist { inner: behind_persist, .. }),
                metadata,
            } => HydroNode::Persist {
                inner: Box::new(HydroNode::Filter {
                    f,
                    input: behind_persist,
                    metadata: metadata.clone(),
                }),
                metadata: metadata.clone()
            },

            HydroNode::Network {
                from_key,
                to_location,
                to_key,
                serialize_fn,
                instantiate_fn,
                deserialize_fn,
                input: mb!(* HydroNode::Persist { inner: behind_persist, .. }),
                metadata,
            } => HydroNode::Persist {
                inner: Box::new(HydroNode::Network {
                    from_key,
                    to_location,
                    to_key,
                    serialize_fn,
                    instantiate_fn,
                    deserialize_fn,
                    input: behind_persist,
                    metadata: metadata.clone()
                }),
                metadata: metadata.clone(),
            },

            HydroNode::Chain {
                first: mb!(* HydroNode::Persist { inner: first, metadata: persist_metadata }),
                second: mb!(* HydroNode::Persist { inner: second, .. }),
                metadata
            } => HydroNode::Persist {
                inner: Box::new(HydroNode::Chain { first, second, metadata }),
                metadata: persist_metadata
            },

            HydroNode::CrossProduct {
                left: mb!(* HydroNode::Persist { inner: left, metadata: left_metadata }),
                right: mb!(* HydroNode::Persist { inner: right, metadata: right_metadata }),
                metadata
            } => HydroNode::Persist {
                inner: Box::new(HydroNode::Delta {
                    inner: Box::new(HydroNode::CrossProduct {
                        left: Box::new(HydroNode::Persist { inner: left, metadata: left_metadata }),
                        right: Box::new(HydroNode::Persist { inner: right, metadata: right_metadata }),
                        metadata: metadata.clone()
                    }),
                    metadata: metadata.clone(),
                }),
                metadata: metadata.clone(),
            },
            HydroNode::Join {
                left: mb!(* HydroNode::Persist { inner: left, metadata: left_metadata }),
                right: mb!(* HydroNode::Persist { inner: right, metadata: right_metadata }),
                metadata
             } => HydroNode::Persist {
                inner: Box::new(HydroNode::Delta {
                    inner: Box::new(HydroNode::Join {
                        left: Box::new(HydroNode::Persist { inner: left, metadata: left_metadata }),
                        right: Box::new(HydroNode::Persist { inner: right, metadata: right_metadata }),
                        metadata: metadata.clone()
                    }),
                    metadata: metadata.clone(),
                }),
                metadata: metadata.clone(),
            },

            HydroNode::Unique { input: mb!(* HydroNode::Persist {inner, metadata: persist_metadata } ), metadata } => HydroNode::Persist {
                inner: Box::new(HydroNode::Delta {
                    inner: Box::new(HydroNode::Unique {
                        input: Box::new(HydroNode::Persist { inner, metadata: persist_metadata }),
                        metadata: metadata.clone()
                    }),
                    metadata: metadata.clone(),
                }),
                metadata: metadata.clone()
            },

            node => node,
        }
    };
}

pub fn persist_pullup(ir: &mut [HydroLeaf]) {
    let mut persist_pulled_tees = Default::default();
    transform_bottom_up(ir, &mut |_| (), &mut |node| {
        persist_pullup_node(node, &mut persist_pulled_tees)
    });
}

#[cfg(stageleft_runtime)]
#[cfg(test)]
mod tests {
    use stageleft::*;

    use crate::deploy::HydroDeploy;
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
        for (id, graph) in optimized
            .into_deploy::<HydroDeploy>()
            .preview_compile()
            .all_dfir()
        {
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
        let before_tee = unsafe { process.source_iter(q!(0..10)).tick_batch(&tick).persist() };

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

        for (id, graph) in optimized
            .into_deploy::<HydroDeploy>()
            .preview_compile()
            .all_dfir()
        {
            insta::with_settings!({snapshot_suffix => format!("surface_graph_{id}")}, {
                insta::assert_snapshot!(graph.surface_syntax_string());
            });
        }
    }
}
