use crate::ir::*;
use crate::location::LocationId;

fn calculate_overhead(metadata: &HydroIrMetadata) -> Option<f64> {
    if let Some(cardinality) = metadata.cardinality {
        if let Some(cpu_usage) = metadata.cpu_usage {
            return Some(cpu_usage / cardinality as f64);
        }
    }
    None
}

fn analyze_overheads_node(
    node: &mut HydroNode,
    _next_stmt_id: &mut usize,
    max_send_overhead: &mut f64,
    max_recv_overhead: &mut f64,
    location: &LocationId,
) {
    if let HydroNode::Network {
            metadata,
            to_location,
            ..
        } = node {
        if metadata.location_kind.root() == location {
            // Sending from this location to somewhere else
            if let Some(overhead) = calculate_overhead(metadata) {
                println!("New send overhead: {}", overhead);
                if overhead > *max_send_overhead {
                    *max_send_overhead = overhead;
                }
            }
        } else if to_location.root() == location {
            // Receiving from somewhere else to this location
            if let Some(overhead) = calculate_overhead(metadata) {
                println!("New receive overhead: {}", overhead);
                if overhead > *max_recv_overhead {
                    *max_recv_overhead = overhead;
                }
            }
        }
    }
}

// Track the max of each so we decouple conservatively
pub fn analyze_send_recv_overheads(ir: &mut [HydroLeaf], location: &LocationId) -> (f64, f64) {
    let mut max_send_overhead = 0.0;
    let mut max_recv_overhead = 0.0;

    traverse_dfir(
        ir,
        |_, _| {},
        |node, next_stmt_id| {
            analyze_overheads_node(
                node,
                next_stmt_id,
                &mut max_send_overhead,
                &mut max_recv_overhead,
                location,
            );
        },
    );

    if max_send_overhead == 0.0 {
        println!("Warning: No send overhead found.");
    }
    if max_recv_overhead == 0.0 {
        println!("Warning: No receive overhead found.");
    }

    (max_send_overhead, max_recv_overhead)
}
