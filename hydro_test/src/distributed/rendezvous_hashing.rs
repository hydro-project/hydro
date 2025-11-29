use std::hash::{Hash, Hasher};

use hydro_lang::live_collections::stream::NoOrder;
use hydro_lang::location::MemberId;
use hydro_lang::location::cluster::CLUSTER_SELF_ID;
use hydro_lang::location::external_process::{ExternalBincodeSink, ExternalBincodeStream};
use hydro_lang::prelude::*;
use hydro_std::membership::track_membership;
use serde::{Deserialize, Serialize};

pub struct P1 {}
pub struct C2 {}
pub struct P3 {}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum Command {
    Put(String, String),
    Get(String),
}

fn hash<Tag>(key: &str, node: &MemberId<Tag>) -> u64 {
    let mut hasher = std::hash::DefaultHasher::new();
    key.hash(&mut hasher);
    node.hash(&mut hasher);
    hasher.finish()
}

#[expect(clippy::type_complexity, reason = "example code")]
pub fn distributed_rendezvous_partitioning<'a>(
    external: &External<'a, ()>,
    p1: &Process<'a, P1>,
    c2: &Cluster<'a, C2>,
    p3: &Process<'a, P3>,
) -> (
    ExternalBincodeSink<Command>,
    ExternalBincodeStream<(MemberId<C2>, (String, Option<String>)), NoOrder>,
) {
    let (tx, rx) = p1.source_external_bincode::<_, Command, _, _>(external);

    let req = rx
        .map(q!(|n| {
            match n.clone() {
                Command::Put(k, _) => (k.clone(), n),
                Command::Get(k) => (k, n),
            }
        }))
        .inspect(q!(|v| println!("req: {v:?}")));

    c2.source_iter(q!([CLUSTER_SELF_ID]))
        .for_each(q!(|cluster_self_id| eprintln!(
            "cluster_self_id: {cluster_self_id:?}"
        )));

    let tick = p1.tick();

    let members = track_membership(p1.source_cluster_members(c2))
        .snapshot(&tick, nondet!(/** */))
        .map(q!(|_| ()))
        .entries()
        .map(q!(|(k, _)| k))
        .inspect(q!(|v| println!("members: {v:?}")))
        .assume_ordering(nondet!(/** */))
        .collect_vec();

    let max_hash_member = req
        .batch(&tick, nondet!(/** */))
        .cross_singleton(members)
        .filter_map(q!(|((key, command), member_ids)| {
            member_ids
                .into_iter()
                .map(|member_id| (member_id.clone(), self::hash(&key, &member_id)))
                .max_by_key(|v| v.1)
                .map(|(node_id, max_hash)| (key.clone(), command.clone(), node_id, max_hash))
        }));

    let stream = max_hash_member.all_ticks().inspect(q!(|v| println!("stream: {v:?}"))) // ????
        .map(q!(|(_, command, node_id, _)| (node_id, (command)))).into_keyed().demux_bincode(c2);

    let gets = stream
        .clone()
        .filter_map(q!(|v| {
            if let Command::Get(v) = v {
                Some((v, ()))
            } else {
                None
            }
        }))
        .inspect(q!(|v| println!("get: {v:?}")));

    let puts = stream
        .clone()
        .filter_map(q!(|v| {
            if let Command::Put(k, v) = v {
                Some((k, v))
            } else {
                None
            }
        }))
        .inspect(q!(|v| println!("put: {v:?}")))
        .into_keyed();

    let storage = puts
        .assume_ordering(nondet!(/** */))
        .fold(q!(|| None), q!(|acc, v| *acc = Some(v)));

    let new_tick = c2.tick();

    let ret = storage
        .snapshot(&new_tick, nondet!(/** */))
        .entries()
        .join(gets.batch(&new_tick, nondet!(/** */)))
        .map(q!(|(k, (v1, _))| { (k, v1) }))
        .all_ticks();

    let rx = ret.send_bincode(p3).entries();

    let rx = rx.send_bincode_external(external);

    (tx, rx)
}
