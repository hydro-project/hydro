use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use proc_macro2::Span;
use stageleft::quote_type;
use syn::visit_mut::VisitMut;

use crate::{ir::*, ClusterId};
use crate::location::LocationId;
use crate::stream::{deserialize_bincode_with_type, serialize_bincode_with_type};

use super::partitioner::ClusterSelfIdReplace;
use super::{link_cycles, populate_metadata, print_id};

pub struct Decoupler {
    pub output_to_decoupled_machine_after: Vec<usize>, // The output of the operator at this index should be sent to the decoupled machine
    pub output_to_original_machine_after: Vec<usize>, // The output of the operator at this index should be sent to the original machine
    pub place_on_decoupled_machine: Vec<usize>, // This operator should be placed on the decoupled machine. Only for sources
    pub orig_location: LocationId,
    pub decoupled_location: LocationId,
}

fn add_network(node: &mut HydroNode, new_location: &LocationId) {
    let metadata = node.metadata().clone();
    let output_debug_type = metadata.output_type.clone().unwrap();

    let parent_id =  metadata.location_kind.root().raw_id();
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
            output_type: Some(DebugType(mapped_output_type.clone())),
            cardinality: None,
            cpu_usage: None,
            network_recv_cpu_usage: None,
            id: None,
        },
    };

    // Set up the network node
    let output_type = output_debug_type.clone().0;
    let network_node = HydroNode::Network {
        from_key: None,
        to_location: new_location.clone(),
        to_key: None,
        serialize_fn: Some(serialize_bincode_with_type(true, output_type.clone()))
            .map(|e| e.into()),
        instantiate_fn: DebugInstantiate::Building,
        deserialize_fn: Some(deserialize_bincode_with_type(
            Some(quote_type::<()>()),
            output_type,
        ))
        .map(|e| e.into()),
        input: Box::new(mapped_node),
        metadata: HydroIrMetadata {
            location_kind: new_location.clone(),
            output_type: Some(DebugType(mapped_output_type)),
            cardinality: None,
            cpu_usage: None,
            network_recv_cpu_usage: None,
            id: None,
        },
    };

    // Map again to remove the cluster Id (mimicking send_anonymous)
    let f: syn::Expr = syn::parse_quote!(|(_, b)| b);
    let mapped_node = HydroNode::Map {
        f: f.into(),
        input: Box::new(network_node),
        metadata: HydroIrMetadata {
            location_kind: new_location.clone(),
            output_type: Some(output_debug_type),
            cardinality: None,
            cpu_usage: None,
            network_recv_cpu_usage: None,
            id: None,
        },
    };
    *node = mapped_node;
}

fn add_tee(node: &mut HydroNode, new_location: &LocationId, new_inners: &mut HashMap<(usize, LocationId), Rc<RefCell<HydroNode>>>) {
    let metadata = node.metadata().clone();
    let inner_id = if let HydroNode::Tee { inner, .. } = node {
        inner.0.borrow().metadata().id.unwrap()
    } else {
        std::panic!("Decoupler add_tee() called on non-Tee");
    };

    let new_inner = new_inners.entry((inner_id, new_location.clone())).or_insert_with(|| {
        println!("Adding network before Tee to location {:?} after id: {}", new_location, inner_id);
        add_network(node, new_location);
        let node_content = std::mem::replace(node, HydroNode::Placeholder);
        Rc::new(RefCell::new(node_content))
    }).clone();

    let teed_node = HydroNode::Tee {
        inner: TeeNode(new_inner),
        metadata,
    };
    *node = teed_node;
}

fn decouple_node(node: &mut HydroNode, decoupler: &Decoupler, next_stmt_id: &mut usize, new_inners: &mut HashMap<(usize, LocationId), Rc<RefCell<HydroNode>>>) {
    // Replace location of sources, if necessary
    if decoupler.place_on_decoupled_machine.contains(next_stmt_id) {
        match node {
            HydroNode::Source { location_kind, metadata, .. }
            | HydroNode::Network { to_location: location_kind, metadata, .. } => {
                println!("Changing source/network destination from {:?} to location {:?}, id: {}", location_kind, decoupler.decoupled_location.clone(), next_stmt_id);
                *location_kind = decoupler.decoupled_location.clone();
                metadata.location_kind.swap_root(decoupler.decoupled_location.clone());
            }
            _ => {
                std::panic!("Decoupler placing non-source/network node on decoupled machine: {}", node.print_root());
            }
        }
        return;
    }

    // Otherwise, replace where the outputs go
    let new_location = if decoupler.output_to_decoupled_machine_after.contains(next_stmt_id) {
        &decoupler.decoupled_location
    }
    else if decoupler.output_to_original_machine_after.contains(next_stmt_id) {
        &decoupler.orig_location
    }
    else {
        return;
    };

    match node {
        HydroNode::Placeholder | HydroNode::Network { .. } => {
            std::panic!("Decoupler modifying placeholder node or incorrectly handling network node: {}", next_stmt_id);
        }
        HydroNode::Tee { .. } => {
            println!("Creating a TEE to location {:?}, id: {}", new_location, next_stmt_id);
            add_tee(node, new_location, new_inners);
        }
        _ => {
            println!("Creating network to location {:?} after node {}, id: {}", new_location, node.print_root(), next_stmt_id);
            add_network(node, new_location);
        }
    }
}

fn fix_cluster_self_id_leaf(leaf: &mut HydroLeaf, mut locations: ClusterSelfIdReplace) {
    if let ClusterSelfIdReplace::Decouple { decoupled_cluster_id, .. } = locations {
        if leaf.metadata().location_kind.root().raw_id() == decoupled_cluster_id {
            leaf.visit_debug_expr(|expr| {
                locations.visit_expr_mut(&mut expr.0);
            });
        }
    }
}

fn fix_cluster_self_id_node(node: &mut HydroNode, mut locations: ClusterSelfIdReplace) {
    if let ClusterSelfIdReplace::Decouple { decoupled_cluster_id, .. } = locations {
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
    populate_metadata::inject_id(ir);
    // Fix locations since we changed some
    let cycle_source_to_sink_input = link_cycles::cycle_source_to_sink_input(ir);
    populate_metadata::inject_location(ir, &cycle_source_to_sink_input);
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
    );

    println!("Printing IDs after fixing");
    print_id::print_id(ir);
}



#[cfg(test)]
mod tests {
    use hydro_deploy::Deployment;
    use stageleft::*;

    #[cfg(stageleft_runtime)]
    use crate::deploy::DeployCrateWrapper;
    use crate::ir::{deep_clone, HydroLeaf};
    use crate::location::{Location, LocationId};
    #[cfg(stageleft_runtime)]
    use crate::rewrites::{persist_pullup, populate_metadata, decoupler};
    use crate::{ir, Cluster, FlowBuilder};
    
    fn simple_send_recv<'a>(builder: &FlowBuilder<'a>) -> (Cluster<'a, ()>, Cluster<'a, ()>) {
        let send_cluster = builder.cluster::<()>();
        let recv_cluster = builder.cluster::<()>();

        send_cluster.source_iter(q!(0..10))
            .map(q!(|a| a + 1))
            .broadcast_bincode_anonymous(&recv_cluster)
            .for_each(q!(|a| println!("Got it: {}", a)));

        (send_cluster, recv_cluster)
    }

    #[cfg(stageleft_runtime)]
    async fn decouple_send(with_decoupler: &decoupler::Decoupler) -> Vec<HydroLeaf> {
        use std::collections::HashSet;

        let builder = FlowBuilder::new();
        let (send_cluster, recv_cluster) = simple_send_recv(&builder);

        let decoupled_cluster = builder.cluster::<()>();
        let decoupler = decoupler::Decoupler {
            output_to_decoupled_machine_after: with_decoupler.output_to_decoupled_machine_after.clone(),
            output_to_original_machine_after: with_decoupler.output_to_original_machine_after.clone(),
            place_on_decoupled_machine: with_decoupler.place_on_decoupled_machine.clone(),
            decoupled_location: decoupled_cluster.id().clone(),
            orig_location: send_cluster.id().clone(),
        };

        let built = builder
            .optimize_with(persist_pullup::persist_pullup)
            .optimize_with(populate_metadata::inject_id)
            .optimize_with(|ir| decoupler::decouple(ir, &decoupler));
        
        let ir = deep_clone(built.ir());

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
        
        ir
    }

    #[tokio::test]
    async fn decouple_after_source() {
        let decoupler = decoupler::Decoupler {
            output_to_decoupled_machine_after: vec![0],
            output_to_original_machine_after: vec![],
            place_on_decoupled_machine: vec![],
            decoupled_location: LocationId::Cluster(0), // Doesn't matter, will be ignored
            orig_location: LocationId::Cluster(0),
        };

        let ir = decouple_send(&decoupler).await;
        ir::dbg_dedup_tee(|| {
            insta::assert_debug_snapshot!(ir);
        });
    }

    #[tokio::test]
    async fn move_source_decouple_map() {
        let decoupler = decoupler::Decoupler {
            output_to_decoupled_machine_after: vec![],
            output_to_original_machine_after: vec![1],
            place_on_decoupled_machine: vec![0],
            decoupled_location: LocationId::Cluster(0), // Doesn't matter, will be ignored
            orig_location: LocationId::Cluster(0),
        };

        let ir = decouple_send(&decoupler).await;
        ir::dbg_dedup_tee(|| {
            insta::assert_debug_snapshot!(ir);
        });
    }
}