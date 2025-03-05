use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use proc_macro2::Span;
use stageleft::quote_type;

use crate::{ir::*, ClusterId};
use crate::location::LocationId;
use crate::stream::{deserialize_bincode_with_type, serialize_bincode_with_type};

pub struct Decoupler {
    pub output_to_decoupled_machine_after: Vec<usize>, // The output of the operator at this index should be sent to the decoupled machine
    pub output_to_original_machine_after: Vec<usize>, // The output of the operator at this index should be sent to the original machine
    pub place_on_decoupled_machine: Vec<usize>, // This operator should be placed on the decoupled machine. Only for sources
    pub orig_location: LocationId,
    pub decoupled_location: LocationId,
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
        instantiate_fn: DebugInstantiate::Building(),
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
    let node_content = std::mem::replace(node, HydroNode::Placeholder);
    let metadata = node_content.metadata().clone();

    let new_inner = new_inners.entry((metadata.id.unwrap(), new_location.clone())).or_insert_with(|| {
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
            HydroNode::Source { location_kind, metadata, .. } => {
                *location_kind = decoupler.decoupled_location.clone();
                metadata.location_kind = decoupler.decoupled_location.clone();
            }
            HydroNode::Network { to_location, .. } => {
                *to_location = decoupler.decoupled_location.clone();
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
        HydroNode::Placeholder => {
            std::panic!("Decoupler modifying placeholder node");
        }
        HydroNode::Tee { .. } => {
            add_network(node, new_location);
            add_tee(node, new_location, new_inners);
        }
        HydroNode::Network { to_location, .. } => {
            // Instead of inserting a network after an existing Network, just modify the location
            *to_location = new_location.clone();
        }
        _ => {
            add_network(node, new_location);
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
}
