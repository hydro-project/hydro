use crate::ir::*;
use crate::location::LocationId;

#[derive(Clone, PartialEq, Eq)]
pub enum NetworkType {
    Recv,
    Send,
    SendRecv,
}

pub fn get_network_type(node: &HydroNode, location: &LocationId) -> Option<NetworkType> {
    let mut is_to_us = false;
    let mut is_from_us = false;

    if let HydroNode::Network {
        input, to_location, ..
    } = node
    {
        if input.metadata().location_kind.root() == location {
            is_from_us = true;
        }
        if to_location.root() == location {
            is_to_us = true;
        }

        return if is_from_us && is_to_us {
            Some(NetworkType::SendRecv)
        } else if is_from_us {
            Some(NetworkType::Send)
        } else if is_to_us {
            Some(NetworkType::Recv)
        } else {
            None
        };
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
    let metadata = node.metadata();
    let network_type = get_network_type(node, location);
    match network_type {
        Some(NetworkType::Send) | Some(NetworkType::SendRecv) => {
            if let Some(cpu_usage) = metadata.cpu_usage {
                // Use cardinality from the network's input, not the network itself.
                // Reason: Cardinality is measured at ONE recipient, but the sender may be sending to MANY machines.
                if let Some(cardinality) = node.input_metadata().first().unwrap().cardinality {
                    let overhead = cpu_usage / cardinality as f64;

                    println!("New send overhead: {}", overhead);
                    if overhead > *max_send_overhead {
                        *max_send_overhead = overhead;
                    }
                }
            }
        }
        _ => {}
    }
    match network_type {
        Some(NetworkType::Recv) | Some(NetworkType::SendRecv) => {
            if let Some(cardinality) = metadata.cardinality {
                if let Some(cpu_usage) = metadata.network_recv_cpu_usage {
                    let overhead = cpu_usage / cardinality as f64;

                    println!("New receive overhead: {}", overhead);
                    if overhead > *max_recv_overhead {
                        *max_recv_overhead = overhead;
                    }
                }
            }
        }
        _ => {}
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
