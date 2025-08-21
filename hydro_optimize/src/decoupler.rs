use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use hydro_lang::ClusterId;
use hydro_lang::ir::{
    DebugInstantiate, DebugType, HydroIrMetadata, HydroLeaf, HydroNode, TeeNode,
    transform_bottom_up, traverse_dfir,
};
use hydro_lang::location::LocationId;
use hydro_lang::stream::networking::{deserialize_bincode_with_type, serialize_bincode_with_type};
use proc_macro2::Span;
use serde::{Deserialize, Serialize};
use stageleft::quote_type;
use syn::visit_mut::VisitMut;

use crate::repair::{cycle_source_to_sink_input, inject_id, inject_location};
use crate::rewrites::ClusterSelfIdReplace;

#[derive(Clone, Serialize, Deserialize)]
pub struct Decoupler {
    pub output_to_decoupled_machine_after: Vec<usize>, /* The output of the operator at this index should be sent to the decoupled machine */
    pub output_to_original_machine_after: Vec<usize>, /* The output of the operator at this index should be sent to the original machine */
    pub place_on_decoupled_machine: Vec<usize>, /* This operator should be placed on the decoupled machine. Only for sources */
    pub orig_location: LocationId,
    pub decoupled_location: LocationId,
}

fn add_network(node: &mut HydroNode, new_location: &LocationId) {
    let metadata = node.metadata().clone();
    let output_debug_type = metadata.output_type.clone().unwrap();

    let parent_id = metadata.location_kind.root().raw_id();
    let node_content = std::mem::replace(node, HydroNode::Placeholder);

    // Map from b to (ClusterId, b), where ClusterId is the id of the decoupled (or original) node we're sending to
    let ident = syn::Ident::new(
        &format!("__hydro_lang_cluster_self_id_{}", parent_id),
        Span::call_site(),
    );
    let f: syn::Expr = syn::parse_quote!(|b| (
        ClusterId::<()>::from_raw(#ident),
        b
    ));
    let cluster_id_type = quote_type::<ClusterId<()>>();
    let mapped_output_type: syn::Type = syn::parse_quote!((#cluster_id_type, #output_debug_type));
    let mapped_node = HydroNode::Map {
        f: f.into(),
        input: Box::new(node_content),
        metadata: HydroIrMetadata {
            location_kind: metadata.location_kind.root().clone(), // Remove any ticks
            backtrace: metadata.backtrace.clone(),
            output_type: Some(DebugType(Box::new(mapped_output_type.clone()))),
            cardinality: None,
            cpu_usage: None,
            network_recv_cpu_usage: None,
            id: None,
            tag: None,
        },
    };

    // Set up the network node
    let output_type = output_debug_type.clone().0;
    let network_node = HydroNode::Network {
        serialize_fn: Some(serialize_bincode_with_type(true, &output_type)).map(|e| e.into()),
        instantiate_fn: DebugInstantiate::Building,
        deserialize_fn: Some(deserialize_bincode_with_type(
            Some(&quote_type::<()>()),
            &output_type,
        ))
        .map(|e| e.into()),
        input: Box::new(mapped_node),
        metadata: HydroIrMetadata {
            location_kind: new_location.clone(),
            backtrace: metadata.backtrace.clone(),
            output_type: Some(DebugType(Box::new(mapped_output_type))),
            cardinality: None,
            cpu_usage: None,
            network_recv_cpu_usage: None,
            id: None,
            tag: None,
        },
    };

    // Map again to remove the cluster Id (mimicking send_anonymous)
    let f: syn::Expr = syn::parse_quote!(|(_, b)| b);
    let mapped_node = HydroNode::Map {
        f: f.into(),
        input: Box::new(network_node),
        metadata: HydroIrMetadata {
            location_kind: new_location.clone(),
            backtrace: metadata.backtrace.clone(),
            output_type: Some(output_debug_type),
            cardinality: None,
            cpu_usage: None,
            network_recv_cpu_usage: None,
            id: None,
            tag: None,
        },
    };
    *node = mapped_node;
}

fn add_tee(
    node: &mut HydroNode,
    new_location: &LocationId,
    new_inners: &mut HashMap<(usize, LocationId), Rc<RefCell<HydroNode>>>,
) {
    let metadata = node.metadata().clone();
    let inner_id = if let HydroNode::Tee { inner, .. } = node {
        inner.0.borrow().metadata().id.unwrap()
    } else {
        std::panic!("Decoupler add_tee() called on non-Tee");
    };

    let new_inner = new_inners
        .entry((inner_id, new_location.clone()))
        .or_insert_with(|| {
            println!(
                "Adding network before Tee to location {:?} after id: {}",
                new_location, inner_id
            );
            add_network(node, new_location);
            let node_content = std::mem::replace(node, HydroNode::Placeholder);
            Rc::new(RefCell::new(node_content))
        })
        .clone();

    let teed_node = HydroNode::Tee {
        inner: TeeNode(new_inner),
        metadata,
    };
    *node = teed_node;
}

fn decouple_node(
    node: &mut HydroNode,
    decoupler: &Decoupler,
    next_stmt_id: &mut usize,
    new_inners: &mut HashMap<(usize, LocationId), Rc<RefCell<HydroNode>>>,
) {
    // Replace location of sources, if necessary
    if decoupler.place_on_decoupled_machine.contains(next_stmt_id) {
        match node {
            HydroNode::Source { metadata, .. } | HydroNode::Network { metadata, .. } => {
                println!(
                    "Changing source/network destination from {:?} to location {:?}, id: {}",
                    metadata.location_kind,
                    decoupler.decoupled_location.clone(),
                    next_stmt_id
                );
                metadata
                    .location_kind
                    .swap_root(decoupler.decoupled_location.clone());
            }
            _ => {
                std::panic!(
                    "Decoupler placing non-source/network node on decoupled machine: {}",
                    node.print_root()
                );
            }
        }
        return;
    }

    // Otherwise, replace where the outputs go
    let new_location = if decoupler
        .output_to_decoupled_machine_after
        .contains(next_stmt_id)
    {
        &decoupler.decoupled_location
    } else if decoupler
        .output_to_original_machine_after
        .contains(next_stmt_id)
    {
        &decoupler.orig_location
    } else {
        return;
    };

    match node {
        HydroNode::Placeholder | HydroNode::Network { .. } => {
            std::panic!(
                "Decoupler modifying placeholder node or incorrectly handling network node: {}",
                next_stmt_id
            );
        }
        HydroNode::Tee { .. } => {
            println!(
                "Creating a TEE to location {:?}, id: {}",
                new_location, next_stmt_id
            );
            add_tee(node, new_location, new_inners);
        }
        _ => {
            println!(
                "Creating network to location {:?} after node {}, id: {}",
                new_location,
                node.print_root(),
                next_stmt_id
            );
            add_network(node, new_location);
        }
    }
}

fn fix_cluster_self_id_leaf(leaf: &mut HydroLeaf, mut locations: ClusterSelfIdReplace) {
    if let ClusterSelfIdReplace::Decouple {
        decoupled_cluster_id,
        ..
    } = locations
    {
        if leaf.metadata().location_kind.root().raw_id() == decoupled_cluster_id {
            leaf.visit_debug_expr(|expr| {
                locations.visit_expr_mut(&mut expr.0);
            });
        }
    }
}

fn fix_cluster_self_id_node(node: &mut HydroNode, mut locations: ClusterSelfIdReplace) {
    if let ClusterSelfIdReplace::Decouple {
        decoupled_cluster_id,
        ..
    } = locations
    {
        if node.metadata().location_kind.root().raw_id() == decoupled_cluster_id {
            node.visit_debug_expr(|expr| {
                locations.visit_expr_mut(&mut expr.0);
            });
        }
    }
}

pub fn decouple(ir: &mut [HydroLeaf], decoupler: &Decoupler) {
    let mut new_inners = HashMap::new();
    traverse_dfir(
        ir,
        |_, _| {},
        |node, next_stmt_id| {
            decouple_node(node, decoupler, next_stmt_id, &mut new_inners);
        },
    );

    // Fix IDs since we injected nodes
    inject_id(ir);
    // Fix locations since we changed some
    let cycle_source_to_sink_input = cycle_source_to_sink_input(ir);
    inject_location(ir, &cycle_source_to_sink_input);
    // Fix CLUSTER_SELF_ID for the decoupled node
    let locations = ClusterSelfIdReplace::Decouple {
        orig_cluster_id: decoupler.orig_location.raw_id(),
        decoupled_cluster_id: decoupler.decoupled_location.raw_id(),
    };
    transform_bottom_up(
        ir,
        &mut |leaf| {
            fix_cluster_self_id_leaf(leaf, locations);
        },
        &mut |node| {
            fix_cluster_self_id_node(node, locations);
        },
        true,
    );
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use hydro_deploy::Deployment;
    use hydro_lang::rewrites::persist_pullup::persist_pullup;
    use hydro_lang::{FlowBuilder, Location, ir};
    use stageleft::q;

    use crate::debug::name_to_id_map;
    use crate::decoupler::{Decoupler, decouple};
    use crate::repair::inject_id;

    fn decouple_mini_program<'a>(
        output_to_decoupled_machine_after: Vec<(&str, i32)>, // name, offset
        output_to_original_machine_after: Vec<(&str, i32)>,
        place_on_decoupled_machine: Vec<(&str, i32)>,
    ) -> (
        hydro_lang::Cluster<'a, ()>,
        hydro_lang::Cluster<'a, ()>,
        hydro_lang::Cluster<'a, ()>,
        hydro_lang::builder::built::BuiltFlow<'a>,
    ) {
        let builder = FlowBuilder::new();
        let send_cluster = builder.cluster::<()>();
        let recv_cluster = builder.cluster::<()>();
        let decoupled_cluster = builder.cluster::<()>();

        send_cluster
            .source_iter(q!(0..10))
            .map(q!(|a| a + 1))
            .ir_node_named("map")
            .broadcast_bincode(&recv_cluster)
            .values()
            .for_each(q!(|a| println!("Got it: {}", a)));

        let built = builder
            .optimize_with(persist_pullup)
            .optimize_with(inject_id)
            .optimize_with(|ir| {
                // Convert named nodes to IDs, accounting for the offset
                let name_to_id = name_to_id_map(ir);
                let decoupler = Decoupler {
                    output_to_decoupled_machine_after: output_to_decoupled_machine_after
                        .into_iter()
                        .map(|(name, offset)| {
                            (name_to_id.get(name).cloned().unwrap() as i32 + offset) as usize
                        })
                        .collect(),
                    output_to_original_machine_after: output_to_original_machine_after
                        .into_iter()
                        .map(|(name, offset)| {
                            (name_to_id.get(name).cloned().unwrap() as i32 + offset) as usize
                        })
                        .collect(),
                    place_on_decoupled_machine: place_on_decoupled_machine
                        .into_iter()
                        .map(|(name, offset)| {
                            (name_to_id.get(name).cloned().unwrap() as i32 + offset) as usize
                        })
                        .collect(),
                    decoupled_location: decoupled_cluster.id().clone(),
                    orig_location: send_cluster.id().clone(),
                };
                decouple(ir, &decoupler)
            });
        (send_cluster, recv_cluster, decoupled_cluster, built)
    }

    async fn check_decouple_mini_program(
        output_to_decoupled_machine_after: Vec<(&str, i32)>, // name, offset
        output_to_original_machine_after: Vec<(&str, i32)>,
        place_on_decoupled_machine: Vec<(&str, i32)>,
    ) {
        let (send_cluster, recv_cluster, decoupled_cluster, built) = decouple_mini_program(
            output_to_decoupled_machine_after,
            output_to_original_machine_after,
            place_on_decoupled_machine,
        );

        // Check outputs
        let mut deployment = Deployment::new();
        let nodes = built
            .with_cluster(&send_cluster, vec![deployment.Localhost(); 1])
            .with_cluster(&recv_cluster, vec![deployment.Localhost(); 3])
            .with_cluster(&decoupled_cluster, vec![deployment.Localhost(); 1])
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let recv_members = nodes.get_cluster(&recv_cluster).members();
        let mut stdouts = vec![];
        for member in recv_members {
            use hydro_lang::deploy::DeployCrateWrapper;

            stdouts.push(member.stdout().await);
        }

        deployment.start().await.unwrap();

        for mut stdout in stdouts {
            let mut expected = HashSet::new();
            let mut received = HashSet::new();
            for i in 1..11 {
                received.insert(stdout.recv().await.unwrap());
                expected.insert(format!("Got it: {}", i));
            }
            assert_eq!(expected, received);
        }
    }

    #[test]
    fn decouple_after_source_ir() {
        let output_to_decoupled_machine_after = vec![("map", -1)];
        let output_to_original_machine_after = vec![];
        let place_on_decoupled_machine = vec![];

        let built = decouple_mini_program(
            output_to_decoupled_machine_after,
            output_to_original_machine_after,
            place_on_decoupled_machine,
        )
        .3;

        ir::dbg_dedup_tee(|| {
            insta::assert_debug_snapshot!(built.ir());
        });
    }

    #[tokio::test]
    async fn decouple_after_source() {
        let output_to_decoupled_machine_after = vec![("map", -1)];
        let output_to_original_machine_after = vec![];
        let place_on_decoupled_machine = vec![];

        check_decouple_mini_program(
            output_to_decoupled_machine_after,
            output_to_original_machine_after,
            place_on_decoupled_machine,
        )
        .await
    }

    #[test]
    fn move_source_decouple_map_ir() {
        let output_to_decoupled_machine_after = vec![];
        let output_to_original_machine_after = vec![("map", 0)];
        let place_on_decoupled_machine = vec![("map", -1)];

        let built = decouple_mini_program(
            output_to_decoupled_machine_after,
            output_to_original_machine_after,
            place_on_decoupled_machine,
        )
        .3;

        ir::dbg_dedup_tee(|| {
            insta::assert_debug_snapshot!(built.ir());
        });
    }

    #[tokio::test]
    async fn move_source_decouple_map() {
        let output_to_decoupled_machine_after = vec![];
        let output_to_original_machine_after = vec![("map", 0)];
        let place_on_decoupled_machine = vec![("map", -1)];

        check_decouple_mini_program(
            output_to_decoupled_machine_after,
            output_to_original_machine_after,
            place_on_decoupled_machine,
        )
        .await
    }
}
