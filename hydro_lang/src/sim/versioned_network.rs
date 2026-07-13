use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use slotmap::SecondaryMap;

use crate::compile::ir::{HydroNode, HydroRoot, SharedNode};
use crate::location::LocationKey;

/// Identity of a logical cross-version channel: its user-provided name plus the correspondence
/// group roots of its source and destination locations. Two `Network` nodes are the same channel
/// iff they share a `ChannelKey` (so a channel name reused between unrelated location pairs is not
/// spuriously merged).
#[derive(PartialEq, Eq, Hash)]
struct ChannelKey {
    name: String,
    src_group_root: LocationKey,
    dst_group_root: LocationKey,
}

fn channel_key_of(
    node: &HydroNode,
    location_version_group_root: &SecondaryMap<LocationKey, LocationKey>,
) -> Option<ChannelKey> {
    let HydroNode::Network {
        name: Some(name),
        input,
        metadata,
        ..
    } = node
    else {
        return None;
    };

    let src_key = input.metadata().location_id.root().key();
    let dst_key = metadata.location_id.root().key();
    let src_group_root = location_version_group_root[src_key];
    let dst_group_root = location_version_group_root[dst_key];

    Some(ChannelKey {
        name: name.clone(),
        src_group_root,
        dst_group_root,
    })
}

pub(crate) fn splice_versioned_networks(
    ir: &mut [HydroRoot],
    location_version_group_root: &SecondaryMap<LocationKey, LocationKey>,
    location_version: &SecondaryMap<LocationKey, u32>,
) {
    let mut forks: HashMap<ChannelKey, Rc<RefCell<HydroNode>>> = HashMap::new();
    let mut next_channel_id: u32 = 0;

    crate::compile::ir::transform_bottom_up(
        ir,
        &mut |_root| {},
        &mut |node| {
            let Some(key) = channel_key_of(node, location_version_group_root) else {
                return;
            };

            let dst_key = node.metadata().location_id.root().key();
            let version = location_version[dst_key];

            let taken = std::mem::replace(node, HydroNode::Placeholder);
            let HydroNode::Network {
                input,
                serialize,
                deserialize,
                metadata,
                ..
            } = taken
            else {
                unreachable!("channel_key_of only returns Some for Network nodes");
            };

            let channel_name = key.name.clone();
            let fork_rc = forks.entry(key).or_insert_with(|| {
                // Assign each channel a unique ID so that unrelated channels between different pairs of nodes don't conflict.
                let channel_id = next_channel_id;
                next_channel_id += 1;
                Rc::new(RefCell::new(HydroNode::VersionedNetworkFork {
                    channel_id,
                    channel_name,
                    senders: Vec::new(),
                    metadata: metadata.clone(),
                }))
            });

            if let HydroNode::VersionedNetworkFork { senders, .. } = &mut *fork_rc.borrow_mut() {
                senders.push((version, input, serialize));
            } else {
                unreachable!("fork map only ever holds VersionedNetworkFork nodes");
            }

            *node = HydroNode::VersionedNetwork {
                fork: SharedNode(fork_rc.clone()),
                version,
                deserialize,
                metadata,
            };
        },
        false,
    );
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use stageleft::q;

    use super::*;
    use crate::live_collections::stream::{ExactlyOnce, TotalOrder};
    use crate::nondet::nondet;
    use crate::prelude::{FlowBuilder, TCP};

    fn build_two_version_shared_channel() -> (
        Vec<HydroRoot>,
        SecondaryMap<LocationKey, LocationKey>,
        SecondaryMap<LocationKey, u32>,
    ) {
        let mut flow = FlowBuilder::new();

        let servers0 = flow.cluster::<u8>();
        let (_in0, input0) = servers0.sim_input::<i32, TotalOrder, ExactlyOnce>();
        input0
            .broadcast(
                &servers0,
                TCP.fail_stop().bincode().name("ch"),
                nondet!(/** test */),
            )
            .values()
            .assume_ordering::<TotalOrder>(nondet!(/** test */))
            .for_each(q!(|x| println!("{x}")));

        let servers1 = flow.next_version(&servers0);
        let (_in1, input1) = servers1.sim_input::<i32, TotalOrder, ExactlyOnce>();
        input1
            .broadcast(
                &servers1,
                TCP.fail_stop().bincode().name("ch"),
                nondet!(/** test */),
            )
            .values()
            .assume_ordering::<TotalOrder>(nondet!(/** test */))
            .for_each(q!(|x| println!("{x}")));

        let sim = flow.sim();
        (
            sim.ir,
            sim.location_version_group_root,
            sim.location_version,
        )
    }

    #[derive(Default, Debug)]
    struct VersionedNetworkCounts {
        networks: usize,
        forks: usize,
        receivers: usize,
        fork_sender_versions: Vec<Vec<u32>>,
        receiver_versions: Vec<u32>,
    }

    fn count_versioned_networks(ir: &mut [HydroRoot]) -> VersionedNetworkCounts {
        let counts = RefCell::new(VersionedNetworkCounts::default());
        let seen_forks = RefCell::new(HashSet::new());

        crate::compile::ir::transform_bottom_up(
            ir,
            &mut |_root| {},
            &mut |node| {
                let mut c = counts.borrow_mut();
                match node {
                    HydroNode::Network { .. } => c.networks += 1,
                    HydroNode::VersionedNetworkFork { senders, .. } => {
                        c.forks += 1;
                        c.fork_sender_versions
                            .push(senders.iter().map(|(v, _, _)| *v).collect());
                    }
                    HydroNode::VersionedNetwork { fork, version, .. } => {
                        c.receivers += 1;
                        c.receiver_versions.push(*version);
                        if seen_forks.borrow_mut().insert(fork.as_ptr())
                            && let HydroNode::VersionedNetworkFork { senders, .. } =
                                &*fork.0.borrow()
                        {
                            c.fork_sender_versions
                                .push(senders.iter().map(|(v, _, _)| *v).collect());
                        }
                    }
                    _ => {}
                }
            },
            false,
        );
        counts.into_inner()
    }

    #[test]
    fn splice_builds_crossbar_for_shared_channel() {
        let (mut ir, names, versions) = build_two_version_shared_channel();
        splice_versioned_networks(&mut ir, &names, &versions);

        let c = count_versioned_networks(&mut ir);

        assert_eq!(
            c.networks, 0,
            "all named Network nodes should be replaced by VersionedNetwork"
        );
        assert_eq!(
            c.receivers, 2,
            "one VersionedNetwork per version that receives the channel"
        );
        let mut receiver_versions = c.receiver_versions.clone();
        receiver_versions.sort_unstable();
        assert_eq!(receiver_versions, vec![0, 1]);

        let mut sender_versions: Vec<u32> = c
            .fork_sender_versions
            .into_iter()
            .flatten()
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        sender_versions.sort_unstable();
        assert_eq!(
            sender_versions,
            vec![0, 1],
            "the shared crossbar must carry both versions' senders"
        );
    }

    #[test]
    fn splice_shares_one_fork_across_versions() {
        let (mut ir, names, versions) = build_two_version_shared_channel();
        splice_versioned_networks(&mut ir, &names, &versions);

        let fork_ptrs = RefCell::new(HashSet::new());
        crate::compile::ir::transform_bottom_up(
            &mut ir,
            &mut |_root| {},
            &mut |node| {
                if let HydroNode::VersionedNetwork { fork, .. } = node {
                    fork_ptrs.borrow_mut().insert(fork.as_ptr());
                }
            },
            false,
        );

        assert_eq!(
            fork_ptrs.into_inner().len(),
            1,
            "both versions' VersionedNetworks must point at one shared VersionedNetworkFork"
        );
    }
}
