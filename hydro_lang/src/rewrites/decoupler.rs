use std::collections::HashMap;

use proc_macro2::Span;

use crate::ir::*;
use crate::location::LocationId;
use crate::stream::{deserialize_bincode_with_type, serialize_bincode_with_type};

pub struct Decoupler {
    pub output_to_decoupled_machine_after: Vec<usize>,
    pub output_to_original_machine_after: Vec<usize>,
    pub orig_location: LocationId,
    pub decoupled_location: LocationId,
}

fn modify_network(node: &mut HydroNode, new_location: &LocationId) {
    println!("Creating network to location {:?}, node {}", new_location, node.print_root());

    let node_content = std::mem::replace(node, HydroNode::Placeholder);
    if let HydroNode::Network { 
        from_key,
        to_location: _,
        to_key,
        serialize_fn,
        instantiate_fn,
        deserialize_fn,
        input,
        metadata,
     } = node_content {
        *node = HydroNode::Network {
            from_key,
            to_location: new_location.clone(),
            to_key,
            serialize_fn,
            instantiate_fn,
            deserialize_fn,
            input,
            metadata,
        }
    }
    else {
        std::panic!("Decoupler modifying network on non-network node: {}", node.print_root());
    }
}

fn add_network(node: &mut HydroNode, new_location: &LocationId) {
    println!("Creating network to location {:?} after node {}", new_location, node.print_root());

    let metadata = node.metadata().clone();
    let output_debug_type = metadata.output_type.clone().unwrap();

    let parent_id =  metadata.location_kind.raw_id();
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
    let mapped_node = HydroNode::Map {
        f: f.into(),
        input: Box::new(node_content),
        metadata: HydroIrMetadata {
            location_kind: metadata.location_kind.root().clone(), // Remove any ticks
            output_type: Some(output_debug_type.clone()), // TODO: Fix to account for the ClusterId
            cardinality: None,
            cpu_usage: None,
            network_recv_cpu_usage: None,
            id: None,
        },
    };

    // Set up the network node
    let network_metadata = HydroIrMetadata {
        location_kind: new_location.clone(),
        output_type: Some(output_debug_type.clone()),
        cardinality: None,
        cpu_usage: None,
        network_recv_cpu_usage: None,
        id: None,
    };
    let output_type = output_debug_type.0;
    let network_node = HydroNode::Network {
        from_key: None,
        to_location: new_location.clone(),
        to_key: None,
        serialize_fn: Some(serialize_bincode_with_type(true, output_type.clone()))
            .map(|e| e.into()),
        instantiate_fn: DebugInstantiate::Building(),
        deserialize_fn: Some(deserialize_bincode_with_type(
            Some(stageleft::quote_type::<()>()),
            output_type.clone(),
        ))
        .map(|e| e.into()),
        input: Box::new(mapped_node),
        metadata: network_metadata.clone(),
    };

    // Map again to remove the cluster Id (mimicking send_anonymous)
    let f: syn::Expr = syn::parse_quote!(|(_, b)| b);
    let mapped_node = HydroNode::Map {
        f: f.into(),
        input: Box::new(network_node),
        metadata: network_metadata,
    };
    *node = mapped_node;
}

fn decouple_node(node: &mut HydroNode, decoupler: &Decoupler, next_stmt_id: &mut usize) {
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
        HydroNode::Placeholder => {
            std::panic!("Decoupler modifying placeholder node");
        }
        HydroNode::Source { location_kind, metadata, .. } => {
            // Don't need to decouple, just instantiate on a new node
            *location_kind = new_location.clone();
            metadata.location_kind = new_location.clone();
        }
        HydroNode::CycleSource { .. } => {
            // TODO: Must match CycleSink's id, if decoupling. Should remove location_kind and refactor emit_core
        }
        HydroNode::Tee { .. } => {
            // TODO: Share same source as other Tees after decoupling
        }
        HydroNode::Network { .. } => {
            modify_network(node, new_location);
        }
        _ => {
            add_network(node, new_location);
        }
    }
}

/// Limitations: Cannot decouple across a cycle. Can only decouple clusters (not processes).
pub fn decouple(ir: &mut [HydroLeaf], decoupler: &Decoupler) {
    traverse_dfir(
        ir,
        |_, _| {},
        |node, next_stmt_id| {
            decouple_node(node, decoupler, next_stmt_id);
        },
    );
}
