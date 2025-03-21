use proc_macro2::Span;

use crate::ir::*;
use crate::location::LocationId;
use crate::stream::{deserialize_bincode_with_type, serialize_bincode_with_type};

pub struct Decoupler {
    pub nodes_to_decouple: Vec<usize>,
    pub new_location: LocationId,
}

fn decouple_node(node: &mut HydroNode, decoupler: &Decoupler, next_stmt_id: &mut usize) {
    let metadata = node.metadata().clone();
    if decoupler.nodes_to_decouple.contains(next_stmt_id) {
        println!("Decoupling node {} {}", next_stmt_id, node.print_root());

        let output_debug_type = metadata.output_type.clone().unwrap();

        let parent_id = match metadata.location_kind {
            LocationId::Cluster(id) => id,
            _ => std::panic!(
                "Expected parent location to be a cluster, got {:?}",
                metadata.location_kind
            ),
        };
        let node_content = std::mem::replace(node, HydroNode::Placeholder);

        // Map from b to (ClusterId, b), where ClusterId is the id of the decoupled node we're sending to
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
            metadata: metadata.clone(),
        };

        // Set up the network node
        let network_metadata = HydroIrMetadata {
            location_kind: decoupler.new_location.clone(),
            output_type: Some(output_debug_type.clone()),
            cardinality: None,
            cpu_usage: None,
        };
        let output_type = &*output_debug_type;
        let network_node = HydroNode::Network {
            from_key: None,
            to_location: decoupler.new_location.clone(),
            to_key: None,
            serialize_fn: Some(serialize_bincode_with_type(true, output_type)).map(|e| e.into()),
            instantiate_fn: DebugInstantiate::Building,
            deserialize_fn: Some(deserialize_bincode_with_type(
                Some(&stageleft::quote_type::<()>()),
                output_type,
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
