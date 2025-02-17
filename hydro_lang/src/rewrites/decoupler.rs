use proc_macro2::Span;

use crate::ir::*;
use crate::location::LocationId;
use crate::stream::{deserialize_bincode_with_type, serialize_bincode_with_type};

pub struct Decoupler {
    pub nodes_to_decouple: Vec<usize>,
    pub new_location: LocationId,
}

fn decouple_node(
    node: &mut HydroNode,
    decoupler: &Decoupler,
    next_stmt_id: usize,
) {
    
    let metadata = node.metadata().clone();
    if decoupler.nodes_to_decouple.contains(&next_stmt_id) {
        let output_debug_type = metadata.output_type.clone().unwrap();

        // If parent is a cluster, find the ID and send the message to the decoupled node with the same ID
        if let LocationId::Cluster(parent_id) = metadata.location_kind {
            let map_metadata = HydroNodeMetadata {
                location_kind: metadata.location_kind.clone(),
                output_type: Some(output_debug_type.clone()),
            };

            let ident = syn::Ident::new(
                &format!("__hydro_lang_cluster_self_id_{}", parent_id),
                Span::call_site(),
            );
            let f: syn::Expr = syn::parse_quote!(|b| (
                ClusterId::from_raw(#ident),
                b.clone()
            ));

            let node_content = std::mem::replace(node, HydroNode::Placeholder);
            let mapped_node = HydroNode::Map { 
                f: f.into(),
                input: Box::new(node_content), 
                metadata: map_metadata,
            };
            *node = mapped_node;
        }
        else {
            panic!("Expected parent location to be a cluster, got {:?}", metadata.location_kind);
        }

        // Set up the network node
        let network_metadata = HydroNodeMetadata {
            location_kind: decoupler.new_location.clone(),
            output_type: Some(output_debug_type.clone()),
        };
        let node_content = std::mem::replace(node, HydroNode::Placeholder);
        let output_type = output_debug_type.0;
        let network_node = HydroNode::Network {
            from_location: metadata.location_kind.clone(),
            from_key: None,
            to_location: decoupler.new_location.clone(),
            to_key: None,
            serialize_fn: Some(serialize_bincode_with_type(true, output_type.clone())).map(|e| e.into()),
            instantiate_fn: DebugInstantiate::Building(),
            deserialize_fn: Some(deserialize_bincode_with_type(None, output_type.clone())).map(|e| e.into()),
            input: Box::new(node_content),
            metadata: network_metadata,
        };

        *node = network_node;
    }
}

pub fn decouple(ir: &mut [HydroLeaf], decoupler: &Decoupler) {
    traverse_dfir(
        ir,
        |_, _| {},
        |node, next_stmt_id| {
            decouple_node(node, decoupler, next_stmt_id);
        },
    );
}